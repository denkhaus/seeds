//! `plan show` and `plan validate` (sd 0.5.15 `plan-show.ts`).

#![allow(
    clippy::format_push_string,
    reason = "incremental push_str(&format!(…)) line building reads clearer here, matching render.rs"
)]

macro_rules! push {
    ($out:expr, $line:expr) => {{
        $out.push_str($line);
        $out.push('\n');
    }};
}

use serde_json::{Map, Value, json};

use super::templates::{ItemSpec, PlanTemplate, SectionKind};
use super::{
    PlanError, load_plan_templates, plan_by_id, plan_error, resolve_plan_id, schema, success_line,
};
use crate::Store;
use crate::commands::CommandOutcome;
use crate::model::PlanRecord;

/// One child summary (`ChildSummary`).
fn child_summary(plan: &PlanRecord, store: &Store) -> Vec<Value> {
    let adopted: Vec<&str> = plan
        .field("adoptedChildren")
        .and_then(Value::as_array)
        .map_or_default(|items| items.iter().filter_map(Value::as_str).collect());
    plan.children()
        .into_iter()
        .map(|id| {
            let issue = store.issue(id);
            json!({
                "id": id,
                "title": issue.map_or("(missing)", crate::model::SeedRecord::title),
                "status": issue
                    .and_then(crate::model::SeedRecord::status)
                    .map_or("missing", crate::model::Status::as_str),
                "adopted": adopted.contains(&id),
            })
        })
        .collect()
}

/// The nested plan tree `--json` emits: a child seed that is itself a
/// plan parent recurses (display-truncated at `max_plan_depth`).
fn child_plans(plan: &PlanRecord, store: &Store, depth: u32, max_depth: u32) -> Vec<Value> {
    let mut out = Vec::new();
    for child_id in plan.children() {
        let Some(sub) = best_plan_for_seed(store, child_id) else {
            continue;
        };
        if depth + 1 > max_depth {
            out.push(json!({
                "plan_id": sub.id().as_str(),
                "truncated": true,
                "hint": format!(
                    "depth limit reached — use `sd plan show {}` to drill in",
                    sub.id()
                ),
            }));
        } else {
            out.push(plan_node(sub, store, depth + 1, max_depth));
        }
    }
    out
}

fn plan_node(plan: &PlanRecord, store: &Store, depth: u32, max_depth: u32) -> Value {
    json!({
        "plan": Value::Object(plan.fields().clone()),
        "children": child_summary(plan, store),
        "children_plans": child_plans(plan, store, depth, max_depth),
    })
}

/// The most-recently-updated plan for a seed, sd's tie-break.
fn best_plan_for_seed<'a>(store: &'a Store, seed_id: &str) -> Option<&'a PlanRecord> {
    let mut best: Option<&PlanRecord> = None;
    for plan in &store.plans {
        if plan.seed() != Some(seed_id) {
            continue;
        }
        let better = best.is_none_or(|current| {
            plan.updated_at().unwrap_or("") > current.updated_at().unwrap_or("")
        });
        if better {
            best = Some(plan);
        }
    }
    best
}

pub(super) fn run_show(store: &Store, id_arg: &str, json: bool) -> CommandOutcome {
    let result = (|| -> Result<(Value, Option<PlanTemplate>), PlanError> {
        let plan_id = resolve_plan_id(store, id_arg)?;
        let idx = plan_by_id(store, &plan_id)?;
        let plan = &store.plans[idx];
        let max_depth = store.config.max_plan_depth;
        let templates = load_plan_templates(store)?;
        let template = plan
            .field("template")
            .and_then(Value::as_str)
            .and_then(|name| templates.iter().find(|t| t.name == name));
        Ok((plan_node(plan, store, 1, max_depth), template.cloned()))
    })();
    let (node, template) = match result {
        Ok(found) => found,
        Err(message) => return plan_error(message, json),
    };
    if json {
        let mut envelope = Map::new();
        envelope.insert("success".to_owned(), json!(true));
        envelope.insert("command".to_owned(), json!("plan show"));
        envelope.insert("plan".to_owned(), node["plan"].clone());
        envelope.insert("children".to_owned(), node["children"].clone());
        envelope.insert("children_plans".to_owned(), node["children_plans"].clone());
        return CommandOutcome {
            success: true,
            stdout:  format!(
                "{}\n",
                serde_json::to_string_pretty(&Value::Object(envelope)).expect("serializes")
            ),
            stderr:  String::new(),
        };
    }
    let mut out = String::new();
    let plan = &node["plan"];
    push!(
        out,
        &format!(
            "{}  {}  rev {}",
            plan["id"].as_str().unwrap_or_default(),
            plan["status"].as_str().unwrap_or_default(),
            plan["revision"].as_u64().unwrap_or(1)
        )
    );
    if let Some(name) = plan.get("name").and_then(Value::as_str) {
        push!(out, &format!("Name:     {name}"));
    }
    push!(
        out,
        &format!("Seed:     {}", plan["seed"].as_str().unwrap_or_default())
    );
    push!(
        out,
        &format!(
            "Template: {}",
            plan["template"].as_str().unwrap_or_default()
        )
    );
    push!(
        out,
        &format!(
            "Created:  {}",
            plan["createdAt"].as_str().unwrap_or_default()
        )
    );
    push!(
        out,
        &format!(
            "Updated:  {}",
            plan["updatedAt"].as_str().unwrap_or_default()
        )
    );
    if let Some(outcome) = plan.get("outcome").and_then(Value::as_str) {
        let note = plan
            .get("outcomeNote")
            .and_then(Value::as_str)
            .filter(|note| !note.is_empty())
            .map_or_default(|note| format!(" — {note}"));
        push!(out, &format!("Outcome:  {outcome}{note}"));
    }
    let status = plan["status"].as_str().unwrap_or_default();
    let reviewed = plan.get("reviewedBy").and_then(Value::as_str);
    if reviewed.is_none() && matches!(status, "approved" | "active") {
        push!(out, "Review suggested (no reviewer recorded yet)");
    }
    if let Some(reviewed_by) = reviewed {
        push!(out, &format!("Reviewed: {reviewed_by}"));
    }
    push!(out, "");
    push!(out, "Sections:");
    render_sections(&mut out, &node, template.as_ref());
    out.push('\n');
    let children = node["children"].as_array().cloned().unwrap_or_default();
    out.push_str(&format!("Children ({}):\n", children.len()));
    if children.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for child in &children {
            let tag = if child["adopted"].as_bool().unwrap_or(false) {
                " (adopted)"
            } else {
                ""
            };
            out.push_str(&format!(
                "  {}  [{}]  {}{tag}\n",
                child["id"].as_str().unwrap_or_default(),
                child["status"].as_str().unwrap_or_default(),
                child["title"].as_str().unwrap_or_default()
            ));
        }
    }
    for sub in node["children_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        out.push('\n');
        render_nested(&mut out, &sub, "  ");
    }
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}

const SECTION_ORDER: &[&str] = &[
    "context",
    "approach",
    "alternatives",
    "steps",
    "risks",
    "acceptance",
];

fn render_sections(out: &mut String, node: &Value, template: Option<&PlanTemplate>) {
    let plan = &node["plan"];
    let Some(Value::Object(sections)) = plan.get("sections") else {
        return;
    };
    let mut keys: Vec<&String> = SECTION_ORDER
        .iter()
        .filter_map(|known| sections.get_key_value(*known).map(|(k, _)| k))
        .collect();
    for key in sections.keys() {
        if !SECTION_ORDER.contains(&key.as_str()) {
            keys.push(key);
        }
    }
    for key in keys {
        let value = &sections[key];
        let spec = template.and_then(|template| template.section(key));
        push!(out, &format!("  {key}"));
        match value {
            Value::Null => push!(out, "    (empty)"),
            Value::String(text) => {
                for line in text.split('\n') {
                    push!(out, &format!("    {line}"));
                }
            }
            Value::Array(items) if items.is_empty() => push!(out, "    (none)"),
            Value::Array(items) => render_list(out, items, spec),
            other => push!(out, &format!("    {other}")),
        }
    }
}

fn render_list(out: &mut String, items: &[Value], spec: Option<&super::templates::SectionSpec>) {
    let is_steps = matches!(spec.map(|spec| &spec.kind), Some(SectionKind::Steps));
    let object_item = matches!(spec.map(|spec| &spec.kind), Some(SectionKind::List))
        && matches!(
            spec.and_then(|spec| spec.item.as_ref()),
            Some(ItemSpec::Object(_))
        );
    for (index, entry) in items.iter().enumerate() {
        let marker = format!("    {}.", index + 1);
        match entry {
            Value::String(text) => push!(out, &format!("{marker} {text}")),
            Value::Object(map) if is_steps => render_step(out, &marker, map),
            Value::Object(map) if object_item => render_object_entry(out, &marker, map),
            other => push!(out, &format!("{marker} {other}")),
        }
    }
}

fn render_object_entry(out: &mut String, marker: &str, entry: &Map<String, Value>) {
    let sub_indent = " ".repeat(marker.len() + 1);
    let rendered: Vec<String> = entry
        .iter()
        .filter(|(_, value)| !value.is_null() && value.as_str() != Some(""))
        .map(|(field, value)| {
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            format!("{field}: {text}")
        })
        .collect();
    if rendered.is_empty() {
        push!(out, &format!("{marker} {}", Value::Object(entry.clone())));
        return;
    }
    let mut first = true;
    for line in &rendered {
        if first {
            push!(out, &format!("{marker} {line}"));
            first = false;
        } else {
            push!(out, &format!("{sub_indent}{line}"));
        }
    }
}

fn render_step(out: &mut String, marker: &str, entry: &Map<String, Value>) {
    let has_title = entry.get("title").and_then(Value::as_str).is_some();
    let headline = match entry.get("title").and_then(Value::as_str) {
        Some(title) => title.to_owned(),
        None => match entry.get("existing_seed").and_then(Value::as_str) {
            Some(existing) => format!("(adopt {existing})"),
            None => Value::Object(entry.clone()).to_string(),
        },
    };
    push!(out, &format!("{marker} {headline}"));
    let sub_indent = " ".repeat(marker.len() + 1);
    if let Some(Value::Array(blocks)) = entry.get("blocks")
        && !blocks.is_empty()
    {
        let labels = blocks
            .iter()
            .map(|b| match b {
                Value::Number(number) => number.to_string(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        push!(out, &format!("{sub_indent}blocks: {labels}"));
    }
    if entry.get("requires_plan") == Some(&Value::Bool(true)) {
        push!(out, &format!("{sub_indent}requires_plan: true"));
    }
    if let Some(template) = entry
        .get("plan_template")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
    {
        push!(out, &format!("{sub_indent}plan_template: {template}"));
    }
    if let Some(kind) = entry
        .get("type")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
    {
        push!(out, &format!("{sub_indent}type: {kind}"));
    }
    if let Some(priority) = entry.get("priority").and_then(Value::as_u64) {
        push!(out, &format!("{sub_indent}priority: {priority}"));
    }
    if let Some(Value::Array(labels)) = entry.get("labels")
        && !labels.is_empty()
    {
        let rendered = labels
            .iter()
            .map(|l| match l {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        push!(out, &format!("{sub_indent}labels: {rendered}"));
    }
    if has_title
        && let Some(existing) = entry
            .get("existing_seed")
            .and_then(Value::as_str)
            .filter(|e| !e.is_empty())
    {
        push!(out, &format!("{sub_indent}existing_seed: {existing}"));
    }
}

fn render_nested(out: &mut String, entry: &Value, indent: &str) {
    if entry.get("truncated").and_then(Value::as_bool) == Some(true) {
        push!(
            out,
            &format!("{indent}{}", entry["hint"].as_str().unwrap_or_default())
        );
        return;
    }
    let plan = &entry["plan"];
    let name = plan
        .get("name")
        .and_then(Value::as_str)
        .map_or_default(|name| format!("  {name}"));
    push!(
        out,
        &format!(
            "{indent}Sub-plan: {}{name}  [{}]  rev {}  seed={}",
            plan["id"].as_str().unwrap_or_default(),
            plan["status"].as_str().unwrap_or_default(),
            plan["revision"].as_u64().unwrap_or(1),
            plan["seed"].as_str().unwrap_or_default()
        )
    );
    let children = entry["children"].as_array().cloned().unwrap_or_default();
    push!(out, &format!("{indent}Children ({}):", children.len()));
    let child_indent = format!("{indent}  ");
    if children.is_empty() {
        push!(out, &format!("{child_indent}(none)"));
    } else {
        for child in &children {
            let tag = if child["adopted"].as_bool().unwrap_or(false) {
                " (adopted)"
            } else {
                ""
            };
            push!(
                out,
                &format!(
                    "{child_indent}{}  [{}]  {}{tag}",
                    child["id"].as_str().unwrap_or_default(),
                    child["status"].as_str().unwrap_or_default(),
                    child["title"].as_str().unwrap_or_default()
                )
            );
        }
    }
    for sub in entry["children_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        push!(out, "");
        render_nested(out, &sub, &child_indent);
    }
}

/// One validate run: success outcome or the stderr diff failure.
enum ValidateResult {
    Valid(String),
    Diff(String),
}

pub(super) fn run_validate(store: &Store, id_arg: &str, json: bool) -> CommandOutcome {
    let plan_id = match resolve_plan_id(store, id_arg) {
        Ok(plan_id) => plan_id,
        Err(message) => return plan_error(message, json),
    };
    let idx = match plan_by_id(store, &plan_id) {
        Ok(idx) => idx,
        Err(message) => return plan_error(message, json),
    };
    let plan = &store.plans[idx];
    let templates = match load_plan_templates(store) {
        Ok(templates) => templates,
        Err(message) => return plan_error(message, json),
    };
    let template_name = plan
        .field("template")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let available = templates
        .iter()
        .map(|t| t.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let Some(template) = templates.iter().find(|t| t.name == template_name) else {
        return plan_error(
            format!(
                "Plan {plan_id} references unknown template '{template_name}'. Available: \
                 {available}."
            ),
            json,
        );
    };
    let subject = json!({
        "template": template_name,
        "sections": plan.sections().cloned().unwrap_or_else(|| json!({})),
    });
    let result = match schema::validate_plan(template, &subject) {
        Ok(()) => ValidateResult::Valid(plan_id),
        Err(errors) => {
            let diff = schema::PartialStateDiff {
                errors:  &errors,
                current: &subject,
            };
            ValidateResult::Diff(serde_json::to_string_pretty(&diff).expect("serializes"))
        }
    };
    match result {
        ValidateResult::Valid(plan_id) => {
            if json {
                let envelope = json!({
                    "success": true,
                    "command": "plan validate",
                    "valid": true,
                    "plan_id": plan_id,
                });
                CommandOutcome {
                    success: true,
                    stdout:  format!(
                        "{}\n",
                        serde_json::to_string_pretty(&envelope).expect("serializes")
                    ),
                    stderr:  String::new(),
                }
            } else {
                let mut out = String::new();
                success_line(&mut out, &format!("plan {plan_id} valid"));
                CommandOutcome {
                    success: true,
                    stdout:  out,
                    stderr:  String::new(),
                }
            }
        }
        ValidateResult::Diff(diff) => CommandOutcome {
            success: false,
            stdout:  String::new(),
            stderr:  format!("{diff}\n"),
        },
    }
}
