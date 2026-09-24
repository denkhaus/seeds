//! `plan prompt` — the structured planning prompt (sd 0.5.15
//! `runPrompt`).

use serde_json::{Map, Value, json};

use super::templates::default_template_for_type;
use super::{PlanError, load_plan_templates, mulch, plan_error};
use crate::Store;
use crate::commands::plan::templates::PlanTemplate;

const INSTRUCTIONS: &str = "Fill every section. Required fields are marked. Use prior_art entries \
 to ground decisions. Reply with JSON shaped { \"template\": \"<name>\", \"name\": \"<short \
 label>\", \"sections\": { \"<section-name>\": <value>, ... } } — drop the plan_request wrapper, \
 and sections in your reply is an object keyed by name (not the array of section metadata \
 above). The top-level `name` field is an optional short human-readable label (e.g. \
 \"Schema-driven config editor\"); if you omit it, sd plan submit derives one from the parent \
 seed title. Each step is shaped { title?, type?, priority?, blocks?: number[], labels?: \
 string[], plan_template?, existing_seed? }. In each step, `blocks` lists 1-based step indices \
 that this step blocks (step 1 is the first step, step N is the last); e.g. step 1 with `blocks: \
 [2]` means step 1 must finish before step 2 starts. Leave empty if nothing depends on it. \
 Optional `labels` is an array of non-empty strings applied to the spawned (or adopted) child \
 seed; values are normalized (lowercased, trimmed, deduped) and merged additively on adoption — \
 they never clobber existing labels.";

/// Resolves the template a plan for `seed` would use: explicit
/// override, the inherited step `plan_template`, or the type default.
pub(crate) fn resolve_template_name(
    store: &Store,
    seed_id: &str,
    seed_type: &str,
    override_name: Option<&str>,
) -> String {
    if let Some(name) = override_name {
        return name.to_owned();
    }
    if let Some(inherited) = resolve_step_plan_template(store, seed_id) {
        return inherited;
    }
    default_template_for_type(seed_type).to_owned()
}

/// sd's `resolveStepPlanTemplate`: the plan_template declared on the
/// parent plan step that spawned this seed.
fn resolve_step_plan_template(store: &Store, seed_id: &str) -> Option<String> {
    let record = store.issue(seed_id)?;
    let step_index = record
        .field("plan_step_index")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())?;
    let parent = record
        .field("plan_id")
        .and_then(Value::as_str)
        .and_then(|plan_id| {
            store
                .plans
                .iter()
                .find(|plan| plan.id().as_str() == plan_id)
        })
        .or_else(|| {
            store
                .plans
                .iter()
                .find(|plan| plan.children().contains(&seed_id))
        })?;
    let step = parent
        .sections()
        .and_then(|sections| sections.get("steps"))
        .and_then(Value::as_array)
        .and_then(|steps| steps.get(step_index))?;
    step.get("plan_template")
        .and_then(Value::as_str)
        .map(String::from)
}

/// Builds the plan_request JSON for a template.
fn build_plan_request(
    seed_id: &str,
    template_name: &str,
    template: &PlanTemplate,
    prior_art: &[(String, Vec<mulch::PriorArtEntry>)],
) -> Value {
    let sections: Vec<Value> = template
        .sections
        .iter()
        .map(|(name, spec)| {
            let mut section = Map::new();
            section.insert("name".to_owned(), json!(name));
            section.insert("required".to_owned(), json!(spec.required));
            section.insert("kind".to_owned(), kind_value(&spec.kind));
            section.insert("prompt".to_owned(), json!(spec.prompt));
            let entries = prior_art
                .iter()
                .find(|(slot, _)| slot == name)
                .map(|(_, entries)| entries)
                .map_or_default(|entries| {
                    entries
                        .iter()
                        .map(|entry| {
                            json!({
                                "id": entry.id,
                                "type": entry.kind,
                                "summary": entry.summary,
                                "relevance": entry.relevance,
                            })
                        })
                        .collect::<Vec<_>>()
                });
            section.insert("prior_art".to_owned(), Value::Array(entries));
            if let Some(min_length) = spec.min_length {
                section.insert("min_length".to_owned(), json!(min_length));
            }
            if let Some(min) = spec.min {
                section.insert("min".to_owned(), json!(min));
            }
            if let Some(item) = &spec.item {
                section.insert("item".to_owned(), item_value(item));
            }
            Value::Object(section)
        })
        .collect();
    let steps_min = template
        .section("steps")
        .and_then(|spec| spec.min)
        .unwrap_or(0);
    let acceptance_min = template
        .section("acceptance")
        .and_then(|spec| spec.min)
        .unwrap_or(0);
    json!({
        "seed": seed_id,
        "template": template_name,
        "instructions": INSTRUCTIONS,
        "sections": sections,
        "validation": {
            "all_required_present": true,
            "min_steps": steps_min,
            "min_acceptance": acceptance_min,
        },
    })
}

fn kind_value(kind: &super::templates::SectionKind) -> Value {
    use super::templates::SectionKind::{List, Object, Steps, Text};
    match kind {
        Text => json!("text"),
        List => json!("list"),
        Steps => json!("steps"),
        Object(fields) => object_spec_value(fields),
    }
}

fn item_value(item: &super::templates::ItemSpec) -> Value {
    use super::templates::ItemSpec::{Object, Text};
    match item {
        Text => json!("text"),
        Object(fields) => object_spec_value(fields),
    }
}

fn object_spec_value(fields: &[(String, super::templates::SectionSpec)]) -> Value {
    let mut map = Map::new();
    for (name, spec) in fields {
        let mut entry = Map::new();
        entry.insert("required".to_owned(), json!(spec.required));
        entry.insert("kind".to_owned(), kind_value(&spec.kind));
        entry.insert("prompt".to_owned(), json!(spec.prompt));
        map.insert(name.clone(), Value::Object(entry));
    }
    Value::Object(map)
}

pub(super) fn run(
    store: &Store,
    seed_id: &str,
    template_override: Option<&str>,
    domain_override: Option<&str>,
    json: bool,
) -> crate::commands::CommandOutcome {
    let result = (|| -> Result<Value, PlanError> {
        let seed = store
            .issue(seed_id)
            .ok_or_else(|| format!("Seed not found: {seed_id}"))?;
        let templates = load_plan_templates(store)?;
        let seed_type = seed
            .seed_type()
            .map_or_else(|| "task".to_owned(), |kind| kind.as_str().to_owned());
        let template_name = resolve_template_name(store, seed_id, &seed_type, template_override);
        let template = templates
            .iter()
            .find(|t| t.name == template_name)
            .ok_or_else(|| {
                format!(
                    "Unknown template: {template_name}. Available: {}",
                    templates
                        .iter()
                        .map(|t| t.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        let cwd = store.root().parent().unwrap_or(store.root()).to_path_buf();
        let domain = mulch::infer_domain(domain_override, seed, &cwd);
        let section_requests: Vec<(String, Option<String>)> = template
            .sections
            .iter()
            .map(|(name, spec)| (name.clone(), spec.mulch_source.clone()))
            .collect();
        let prior_art = mulch::enrich_prior_art(domain.as_deref(), &section_requests, &cwd);
        Ok(build_plan_request(
            seed_id,
            &template_name,
            template,
            &prior_art,
        ))
    })();
    let plan_request = match result {
        Ok(plan_request) => plan_request,
        Err(message) => return plan_error(message, json),
    };
    if json {
        let mut envelope = Map::new();
        envelope.insert("success".to_owned(), json!(true));
        envelope.insert("command".to_owned(), json!("plan prompt"));
        envelope.insert("plan_request".to_owned(), plan_request);
        return crate::commands::CommandOutcome {
            success: true,
            stdout:  format!(
                "{}\n",
                serde_json::to_string_pretty(&Value::Object(envelope)).expect("serializes")
            ),
            stderr:  String::new(),
        };
    }
    let seed_title = plan_request["seed"]
        .as_str()
        .and_then(|id| store.issue(id))
        .map_or_default(|record| record.title().to_owned());
    let mut out = String::new();
    let mut push = |line: &str| {
        out.push_str(line);
        out.push('\n');
    };
    push(&format!(
        "Plan prompt for {}",
        plan_request["seed"].as_str().unwrap_or_default()
    ));
    push(&format!(
        "Template: {}",
        plan_request["template"].as_str().unwrap_or_default()
    ));
    push(&format!("Seed title: {seed_title}"));
    push("");
    push(plan_request["instructions"].as_str().unwrap_or_default());
    push("");
    for section in plan_request["sections"].as_array().unwrap_or(&Vec::new()) {
        let required = if section["required"].as_bool().unwrap_or(false) {
            "required"
        } else {
            "optional"
        };
        push(&format!(
            "  {} ({}) {}",
            section["name"].as_str().unwrap_or_default(),
            kind_label(section),
            required
        ));
        push(&format!(
            "    {}",
            section["prompt"].as_str().unwrap_or_default()
        ));
        if let Some(min_length) = section.get("min_length") {
            push(&format!("    min_length: {min_length}"));
        }
        if let Some(min) = section.get("min") {
            push(&format!("    min entries: {min}"));
        }
    }
    push("");
    push("Pipe --json into a file the LLM fills, then run:");
    push(&format!(
        "  sd plan submit {} --plan <file>",
        plan_request["seed"].as_str().unwrap_or_default()
    ));
    crate::commands::CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}

fn kind_label(section: &Value) -> &str {
    section
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("object")
}
