//! Plan templates: the built-in feature/bug/refactor definitions and
//! the `plan_templates:` block of `.seeds/config.yaml` (sd 0.5.15's
//! `config.ts` / `plan-schema.ts` surface).

use serde_json::Value;

use crate::Store;

/// One section's specification inside a plan template.
#[derive(Clone, Debug)]
pub(crate) struct SectionSpec {
    /// Whether `sd plan submit` requires the section.
    pub required:     bool,
    /// The section kind: text, list, steps, or a nested object.
    pub kind:         SectionKind,
    /// The author-facing prompt text (`sd plan prompt`).
    pub prompt:       String,
    /// Minimum string length for text sections.
    pub min_length:   Option<u64>,
    /// Minimum entry count for list/steps sections.
    pub min:          Option<u64>,
    /// The per-entry shape for list sections.
    pub item:         Option<ItemSpec>,
    /// The mulch record type used for prior-art enrichment.
    pub mulch_source: Option<String>,
}

/// The kind of value a section accepts.
#[derive(Clone, Debug)]
pub(crate) enum SectionKind {
    /// A plain string.
    Text,
    /// An array of entries.
    List,
    /// An array of step objects.
    Steps,
    /// A nested object with per-field specs.
    Object(Vec<(String, SectionSpec)>),
}

/// The per-entry shape of a list section.
#[derive(Clone, Debug)]
pub(crate) enum ItemSpec {
    /// Entries are non-empty strings.
    Text,
    /// Entries are objects with per-field specs.
    Object(Vec<(String, SectionSpec)>),
}

/// One plan template.
#[derive(Clone, Debug)]
pub(crate) struct PlanTemplate {
    /// The template name.
    pub name:        String,
    /// The one-line description (`sd plan templates`).
    pub description: Option<String>,
    /// The section specs in declaration order.
    pub sections:    Vec<(String, SectionSpec)>,
}

impl PlanTemplate {
    /// Looks up one section spec by name.
    pub(crate) fn section(&self, name: &str) -> Option<&SectionSpec> {
        self.sections
            .iter()
            .find(|(section, _)| section == name)
            .map(|(_, spec)| spec)
    }

    /// The steps section's key, if the template declares one.
    pub(crate) fn steps_key(&self) -> Option<&str> {
        self.sections
            .iter()
            .find(|(_, spec)| matches!(spec.kind, SectionKind::Steps))
            .map(|(name, _)| name.as_str())
    }
}

fn text(required: bool, min_length: Option<u64>, prompt: &str) -> SectionSpec {
    SectionSpec {
        required,
        kind: SectionKind::Text,
        prompt: prompt.to_owned(),
        min_length,
        min: None,
        item: None,
        mulch_source: None,
    }
}

fn list(required: bool, min: Option<u64>, item: Option<ItemSpec>, prompt: &str) -> SectionSpec {
    SectionSpec {
        required,
        kind: SectionKind::List,
        prompt: prompt.to_owned(),
        min_length: None,
        min,
        item,
        mulch_source: None,
    }
}

/// The built-in `feature` template (sd 0.5.15 `BUILTIN_FEATURE_TEMPLATE`).
fn feature_template() -> PlanTemplate {
    let alternatives_item = ItemSpec::Object(vec![
        ("name".to_owned(), text(true, None, "")),
        ("rejected_because".to_owned(), text(true, None, "")),
    ]);
    PlanTemplate {
        name:        "feature".to_owned(),
        description: Some(
            "New capability or significant change. Default for type: feature.".to_owned(),
        ),
        sections:    vec![
            (
                "context".to_owned(),
                text(
                    true,
                    Some(50),
                    "Why does this work need to happen? What problem or opportunity drives it?",
                ),
            ),
            (
                "approach".to_owned(),
                text(
                    true,
                    None,
                    "What's the chosen approach, and why this over alternatives?",
                ),
            ),
            (
                "alternatives".to_owned(),
                list(
                    false,
                    None,
                    Some(alternatives_item),
                    "What other approaches were considered and rejected?",
                ),
            ),
            ("steps".to_owned(), SectionSpec {
                required:     true,
                kind:         SectionKind::Steps,
                prompt:       "Decompose into ordered, independent implementation steps. Each \
                             becomes a child seed."
                    .to_owned(),
                min_length:   None,
                min:          Some(2),
                item:         None,
                mulch_source: None,
            }),
            ("risks".to_owned(), SectionSpec {
                required:     false,
                kind:         SectionKind::List,
                prompt:       "What could go wrong? Known failure modes from prior work are \
                             pre-filled when mulch is available."
                    .to_owned(),
                min_length:   None,
                min:          None,
                item:         Some(ItemSpec::Text),
                mulch_source: Some("failure".to_owned()),
            }),
            (
                "acceptance".to_owned(),
                list(
                    true,
                    Some(1),
                    Some(ItemSpec::Text),
                    "Concrete, verifiable conditions for plan completion.",
                ),
            ),
        ],
    }
}

/// The built-in `bug` template (sd 0.5.15 `BUILTIN_BUG_TEMPLATE`).
fn bug_template() -> PlanTemplate {
    PlanTemplate {
        name:        "bug".to_owned(),
        description: Some(
            "Defect fix. Adds reproduction and root_cause sections. Default for type: bug."
                .to_owned(),
        ),
        sections:    vec![
            (
                "context".to_owned(),
                text(
                    true,
                    None,
                    "Why does fixing this matter? Who is affected and how?",
                ),
            ),
            (
                "reproduction".to_owned(),
                text(
                    true,
                    Some(50),
                    "Concrete steps to reproduce. Inputs, environment, observed vs. expected.",
                ),
            ),
            (
                "root_cause".to_owned(),
                text(
                    true,
                    Some(50),
                    "What's actually broken? Trace the defect to its source, not just the \
                     symptom.",
                ),
            ),
            (
                "approach".to_owned(),
                text(
                    true,
                    None,
                    "Chosen fix and the rationale for it over alternatives.",
                ),
            ),
            ("steps".to_owned(), SectionSpec {
                required:     true,
                kind:         SectionKind::Steps,
                prompt:       "Ordered fix steps. Each becomes a child seed.".to_owned(),
                min_length:   None,
                min:          Some(1),
                item:         None,
                mulch_source: None,
            }),
            (
                "acceptance".to_owned(),
                list(
                    true,
                    Some(1),
                    Some(ItemSpec::Text),
                    "Verifiable conditions: regression test, behavior, etc.",
                ),
            ),
        ],
    }
}

/// The built-in `refactor` template (sd 0.5.15
/// `BUILTIN_REFACTOR_TEMPLATE`).
fn refactor_template() -> PlanTemplate {
    PlanTemplate {
        name:        "refactor".to_owned(),
        description: Some(
            "Internal restructuring. Adds behavior_invariant (must stay equal). Opt-in via \
             --template refactor."
                .to_owned(),
        ),
        sections:    vec![
            (
                "context".to_owned(),
                text(true, None, "Why this refactor? What pain does it relieve?"),
            ),
            (
                "behavior_invariant".to_owned(),
                text(
                    true,
                    Some(50),
                    "The contract that MUST remain equal across the refactor. Be specific — \
                     this is what acceptance tests verify.",
                ),
            ),
            (
                "approach".to_owned(),
                text(true, None, "Chosen restructuring strategy."),
            ),
            ("steps".to_owned(), SectionSpec {
                required:     true,
                kind:         SectionKind::Steps,
                prompt:       "Ordered restructuring steps. Each becomes a child seed.".to_owned(),
                min_length:   None,
                min:          Some(1),
                item:         None,
                mulch_source: None,
            }),
            (
                "acceptance".to_owned(),
                list(
                    true,
                    Some(1),
                    Some(ItemSpec::Text),
                    "How we'll verify the invariant is preserved.",
                ),
            ),
        ],
    }
}

/// sd's type → default-template mapping (`defaultTemplateForType`).
pub(crate) fn default_template_for_type(seed_type: &str) -> &'static str {
    match seed_type {
        "bug" => "bug",
        _ => "feature",
    }
}

/// The loaded template set: built-ins plus the `plan_templates:`
/// config block, in sd's insertion order (builtins first, custom
/// entries overriding in place or appending).
pub(crate) fn load_plan_templates(store: &Store) -> Result<Vec<PlanTemplate>, String> {
    let mut templates = vec![feature_template(), bug_template(), refactor_template()];
    let Some(block) = store.config.extra.get("plan_templates") else {
        return Ok(templates);
    };
    let Value::Object(entries) = block else {
        return Err("plan_templates must be a mapping".to_owned());
    };
    for (name, raw) in entries {
        let template = parse_custom_template(name, raw)?;
        if let Some(slot) = templates.iter_mut().find(|t| t.name == name.as_str()) {
            *slot = template;
        } else {
            templates.push(template);
        }
    }
    Ok(templates)
}

fn parse_custom_template(name: &str, raw: &Value) -> Result<PlanTemplate, String> {
    let Value::Object(fields) = raw else {
        return Err(format!("plan_templates.{name} must be a mapping"));
    };
    let Some(Value::Object(sections)) = fields.get("sections") else {
        return Err(format!("plan_templates.{name}.sections must be a mapping"));
    };
    let mut parsed = Vec::new();
    for (section, spec) in sections {
        let path = format!("plan_templates.{name}.sections.{section}");
        parsed.push((section.clone(), parse_section_spec(spec, &path)?));
    }
    let description = match fields.get("description") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(other) => {
            let _ = other;
            return Err(format!(
                "plan_templates.{name}.description: must be a string (got: {other})"
            ));
        }
        None => None,
    };
    Ok(PlanTemplate {
        name: name.to_owned(),
        description,
        sections: parsed,
    })
}

fn parse_section_spec(raw: &Value, path: &str) -> Result<SectionSpec, String> {
    let Value::Object(fields) = raw else {
        return Err(format!("{path}: must be a mapping"));
    };
    let required = match fields.get("required") {
        Some(Value::Bool(value)) => *value,
        Some(other) => {
            return Err(format!("{path}.required: must be a boolean (got: {other})"));
        }
        None => {
            return Err(format!(
                "{path}.required: must be a boolean (got: undefined)"
            ));
        }
    };
    let prompt = match fields.get("prompt") {
        Some(Value::String(text)) => text.clone(),
        Some(other) => {
            return Err(format!("{path}.prompt: must be a string (got: {other})"));
        }
        None => return Err(format!("{path}.prompt: must be a string (got: undefined)")),
    };
    let kind = parse_kind(fields.get("kind"), &format!("{path}.kind"))?;
    let min_length = match fields.get("min_length") {
        Some(Value::Number(number)) => number.as_u64(),
        Some(other) => {
            return Err(format!(
                "{path}.min_length: must be a number (got: {other})"
            ));
        }
        None => None,
    };
    let min = match fields.get("min") {
        Some(Value::Number(number)) => number.as_u64(),
        Some(other) => {
            return Err(format!("{path}.min: must be a number (got: {other})"));
        }
        None => None,
    };
    let item = match fields.get("item") {
        Some(value) => Some(parse_item(value, &format!("{path}.item"))?),
        None => None,
    };
    let mulch_source = match fields.get("mulch_source") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(other) => {
            return Err(format!(
                "{path}.mulch_source: must be a string (got: {other})"
            ));
        }
        None => None,
    };
    Ok(SectionSpec {
        required,
        kind,
        prompt,
        min_length,
        min,
        item,
        mulch_source,
    })
}

fn parse_kind(raw: Option<&Value>, path: &str) -> Result<SectionKind, String> {
    match raw {
        Some(Value::String(kind)) => match kind.as_str() {
            "text" => Ok(SectionKind::Text),
            "list" => Ok(SectionKind::List),
            "steps" => Ok(SectionKind::Steps),
            other => Err(format!(
                "{path}: unknown kind '{other}' (expected text|list|steps|object)"
            )),
        },
        Some(Value::Object(fields)) => {
            let mut specs = Vec::new();
            for (name, spec) in fields {
                specs.push((
                    name.clone(),
                    parse_section_spec(spec, &format!("{path}.{name}"))?,
                ));
            }
            Ok(SectionKind::Object(specs))
        }
        Some(other) => Err(format!(
            "{path}: unknown kind '{other}' (expected text|list|steps|object)"
        )),
        None => Err(format!(
            "{path}: unknown kind 'undefined' (expected text|list|steps|object)"
        )),
    }
}

fn parse_item(raw: &Value, path: &str) -> Result<ItemSpec, String> {
    match raw {
        Value::String(kind) if kind == "text" => Ok(ItemSpec::Text),
        Value::Object(fields) => {
            let mut specs = Vec::new();
            for (name, spec) in fields {
                specs.push((
                    name.clone(),
                    parse_section_spec(spec, &format!("{path}.{name}"))?,
                ));
            }
            Ok(ItemSpec::Object(specs))
        }
        other => Err(format!(
            "{path}: must be 'text' or a mapping (got: {other})"
        )),
    }
}
