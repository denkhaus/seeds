//! Plan validation: the hand-rolled equivalent of sd 0.5.15's
//! AJV-compiled plan schema (`plan-schema.ts` + `validation.ts`),
//! producing the same `{errors: [{path, code, fix}], current}` diff
//! on stderr when a submitted plan does not validate.

use serde::Serialize;
use serde_json::Value;

use super::templates::{ItemSpec, PlanTemplate, SectionKind, SectionSpec};

/// One validation error, shaped exactly as sd's `ErrorEntry`.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct ErrorEntry {
    /// The dotted instance path (`sections.steps.0.title`).
    pub path: String,
    /// The AJV keyword (sd's `code`), with `minItems` normalized to
    /// `min`.
    pub code: String,
    /// The human-facing fix suggestion.
    pub fix:  String,
}

/// The partial-state diff submit/validate print on stderr.
#[derive(Serialize)]
pub(crate) struct PartialStateDiff<'a> {
    /// Every collected error, in AJV's `allErrors` order.
    pub errors:  &'a [ErrorEntry],
    /// The submitted document, verbatim.
    pub current: &'a Value,
}

/// Validates `data` against `template`, mirroring
/// `compilePlanTemplate`: schema checks first, then the step
/// title-or-adopt and blocks structural passes.
pub(crate) fn validate_plan(template: &PlanTemplate, data: &Value) -> Result<(), Vec<ErrorEntry>> {
    let mut errors = Vec::new();
    let Value::Object(map) = data else {
        errors.push(ErrorEntry {
            path: String::new(),
            code: "type".to_owned(),
            fix:  "'value' must be an object".to_owned(),
        });
        return Err(errors);
    };
    // Root required: template, sections (AJV evaluates `required`
    // before `properties`).
    for key in ["template", "sections"] {
        if !map.contains_key(key) {
            errors.push(ErrorEntry {
                path: key.to_owned(),
                code: "required".to_owned(),
                fix:  format!("add a '{key}' field"),
            });
        }
    }
    if let Some(template_name) = map.get("template")
        && template_name != &Value::String(template.name.clone())
    {
        let allowed = format!("allowedValue={}", template.name);
        errors.push(ErrorEntry {
            path: "template".to_owned(),
            code: "const".to_owned(),
            fix:  format!("'template': const ({allowed})"),
        });
    }
    if let Some(sections) = map.get("sections") {
        validate_sections(template, sections, &mut errors);
    }
    if let Some(steps_key) = template.steps_key()
        && let Some(Value::Array(steps)) = map.get("sections").and_then(|s| s.get(steps_key))
    {
        validate_title_or_adopt(steps, steps_key, &mut errors);
        validate_step_blocks(steps, steps_key, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_sections(template: &PlanTemplate, sections: &Value, errors: &mut Vec<ErrorEntry>) {
    let Value::Object(map) = sections else {
        errors.push(ErrorEntry {
            path: "sections".to_owned(),
            code: "type".to_owned(),
            fix:  "'sections' must be an object".to_owned(),
        });
        return;
    };
    for (name, spec) in &template.sections {
        if spec.required && !map.contains_key(name) {
            errors.push(ErrorEntry {
                path: format!("sections.{name}"),
                code: "required".to_owned(),
                fix:  format!("add a '{name}' field"),
            });
        }
    }
    for (name, spec) in &template.sections {
        if let Some(value) = map.get(name) {
            validate_section(spec, &format!("sections.{name}"), value, errors);
        }
    }
}

fn validate_section(spec: &SectionSpec, path: &str, value: &Value, errors: &mut Vec<ErrorEntry>) {
    match &spec.kind {
        SectionKind::Text => {
            let Value::String(text) = value else {
                errors.push(type_error(path, "string", value));
                return;
            };
            let limit = spec.min_length.unwrap_or(1);
            if text.chars().count() < usize::try_from(limit).expect("schema limits fit usize") {
                errors.push(ErrorEntry {
                    path: path.to_owned(),
                    code: "minLength".to_owned(),
                    fix:  format!("expand '{path}' to at least {limit} characters"),
                });
            }
        }
        SectionKind::List | SectionKind::Steps => {
            let Value::Array(items) = value else {
                errors.push(type_error(path, "array", value));
                return;
            };
            if let Some(limit) = spec.min
                && items.len() < usize::try_from(limit).expect("schema limits fit usize")
            {
                let need = usize::max(
                    usize::try_from(limit).expect("schema limits fit usize") - items.len(),
                    1,
                );
                let noun = if need == 1 { "entry" } else { "entries" };
                errors.push(ErrorEntry {
                    path: path.to_owned(),
                    code: "min".to_owned(),
                    fix:  format!("add at least {need} more {noun}"),
                });
            }
            for (index, item) in items.iter().enumerate() {
                let item_path = format!("{path}.{index}");
                match &spec.kind {
                    SectionKind::Steps => validate_step(item, &item_path, errors),
                    SectionKind::List => match spec.item.as_ref() {
                        Some(ItemSpec::Text) => {
                            if !matches!(item, Value::String(text) if !text.is_empty()) {
                                errors.push(type_error(&item_path, "string", item));
                            }
                        }
                        Some(ItemSpec::Object(fields)) => {
                            validate_object_entry(fields, &item_path, item, errors);
                        }
                        None => {}
                    },
                    SectionKind::Text | SectionKind::Object(_) => unreachable!("matched above"),
                }
            }
        }
        SectionKind::Object(fields) => {
            for (name, field_spec) in fields {
                if field_spec.required
                    && !matches!(value, Value::Object(map) if map.contains_key(name))
                {
                    errors.push(ErrorEntry {
                        path: format!("{path}.{name}"),
                        code: "required".to_owned(),
                        fix:  format!("add a '{name}' field"),
                    });
                }
            }
            if let Value::Object(map) = value {
                for (name, field_spec) in fields {
                    if let Some(field_value) = map.get(name) {
                        validate_section(
                            field_spec,
                            &format!("{path}.{name}"),
                            field_value,
                            errors,
                        );
                    }
                }
            }
        }
    }
}

fn validate_object_entry(
    fields: &[(String, SectionSpec)],
    path: &str,
    value: &Value,
    errors: &mut Vec<ErrorEntry>,
) {
    let Value::Object(map) = value else {
        errors.push(type_error(path, "object", value));
        return;
    };
    for (name, spec) in fields {
        if spec.required && !map.contains_key(name) {
            errors.push(ErrorEntry {
                path: format!("{path}.{name}"),
                code: "required".to_owned(),
                fix:  format!("add a '{name}' field"),
            });
        }
    }
    for (name, spec) in fields {
        if let Some(field_value) = map.get(name) {
            validate_section(spec, &format!("{path}.{name}"), field_value, errors);
        }
    }
}

/// The STEP_SCHEMA property checks, in sd's declaration order:
/// title, type, priority, blocks, plan_template, existing_seed, labels.
fn validate_step(step: &Value, path: &str, errors: &mut Vec<ErrorEntry>) {
    let Value::Object(map) = step else {
        errors.push(type_error(path, "object", step));
        return;
    };
    if let Some(title) = map.get("title")
        && !matches!(title, Value::String(text) if !text.is_empty())
    {
        errors.push(ErrorEntry {
            path: format!("{path}.title"),
            code: "minLength".to_owned(),
            fix:  format!("expand '{path}.title' to at least 1 characters"),
        });
    }
    if let Some(kind) = map.get("type")
        && !matches!(kind, Value::String(text) if ["task", "bug", "feature", "epic"].contains(&text.as_str()))
    {
        errors.push(ErrorEntry {
            path: format!("{path}.type"),
            code: "enum".to_owned(),
            fix:  format!("'{path}.type': enum (allowedValues=task,bug,feature,epic)"),
        });
    }
    if let Some(priority) = map.get("priority") {
        let ok =
            matches!(priority, Value::Number(number) if number.as_u64().is_some_and(|p| p <= 4));
        if !ok {
            errors.push(ErrorEntry {
                path: format!("{path}.priority"),
                code: "type".to_owned(),
                fix:  format!("'{path}.priority' must be an integer"),
            });
        }
    }
    if let Some(blocks) = map.get("blocks")
        && !matches!(blocks, Value::Array(items) if items
            .iter()
            .all(|item| matches!(item, Value::Number(n) if n.as_i64().is_some())))
    {
        errors.push(ErrorEntry {
            path: format!("{path}.blocks"),
            code: "type".to_owned(),
            fix:  format!("'{path}.blocks' must be an array of integer"),
        });
    }
    if let Some(existing) = map.get("existing_seed")
        && !matches!(existing, Value::String(text) if !text.is_empty())
    {
        errors.push(ErrorEntry {
            path: format!("{path}.existing_seed"),
            code: "minLength".to_owned(),
            fix:  format!("expand '{path}.existing_seed' to at least 1 characters"),
        });
    }
    if let Some(labels) = map.get("labels") {
        let ok = matches!(labels, Value::Array(items) if items.iter().all(|item| matches!(
            item,
            Value::String(text) if !text.is_empty() && text.trim() == *text
                && text.chars().any(|c| !c.is_whitespace())
        )));
        if !ok {
            errors.push(ErrorEntry {
                path: format!("{path}.labels"),
                code: "pattern".to_owned(),
                fix:  format!("'{path}.labels': pattern (pattern=\\\\S)"),
            });
        }
    }
}

/// sd's post-AJV pass: every step declares `title` or `existing_seed`.
fn validate_title_or_adopt(steps: &[Value], section_key: &str, errors: &mut Vec<ErrorEntry>) {
    for (index, step) in steps.iter().enumerate() {
        let Value::Object(map) = step else {
            continue;
        };
        let has_title = matches!(map.get("title"), Some(Value::String(t)) if !t.is_empty());
        let has_adopt = matches!(map.get("existing_seed"), Some(Value::String(t)) if !t.is_empty());
        if !has_title && !has_adopt {
            errors.push(ErrorEntry {
                path: format!("sections.{section_key}.{index}"),
                code: "missing-title".to_owned(),
                fix:  format!(
                    "step {} must declare either 'title' (fresh spawn) or 'existing_seed' \
                     (adoption)",
                    index + 1
                ),
            });
        }
    }
}

/// sd's post-AJV pass: `blocks` entries are 1-based step indices in
/// range, never self-referencing.
fn validate_step_blocks(steps: &[Value], section_key: &str, errors: &mut Vec<ErrorEntry>) {
    for (index, step) in steps.iter().enumerate() {
        let Value::Object(map) = step else {
            continue;
        };
        let Some(Value::Array(blocks)) = map.get("blocks") else {
            continue;
        };
        let step_label = index + 1;
        for block in blocks {
            let Value::Number(number) = block else {
                continue;
            };
            let Some(value) = number.as_i64() else {
                continue;
            };
            if value == i64::try_from(step_label).expect("step labels fit i64") {
                errors.push(ErrorEntry {
                    path: format!("sections.{section_key}.{index}.blocks"),
                    code: "self-reference".to_owned(),
                    fix:  format!(
                        "step {step_label} cannot block itself; remove {value} from blocks"
                    ),
                });
            } else if value < 1 || value > i64::try_from(steps.len()).expect("step count fits i64")
            {
                errors.push(ErrorEntry {
                    path: format!("sections.{section_key}.{index}.blocks"),
                    code: "out-of-range".to_owned(),
                    fix:  format!(
                        "step index {value} is out of range (step indices are 1-based; valid \
                         range 1..{})",
                        steps.len()
                    ),
                });
            }
        }
    }
}

fn type_error(path: &str, expected: &str, _value: &Value) -> ErrorEntry {
    ErrorEntry {
        path: path.to_owned(),
        code: "type".to_owned(),
        fix:  format!("'{path}' must be a {expected}"),
    }
}
