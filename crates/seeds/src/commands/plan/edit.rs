//! `plan outcome`, `plan review`, and `plan edit` (sd 0.5.15).

#![allow(
    clippy::format_push_string,
    reason = "incremental push_str(&format!(…)) line building reads clearer here, matching render.rs"
)]
use serde_json::{Value, json};

use super::{
    PlanError, backref, load_plan_templates, normalize_plan_name, parse_step_flag, plan_by_id,
    plan_error, resolve_plan_id, success_line,
};
use crate::commands::{CommandOutcome, parse_priority, parse_seed_type};
use crate::{Store, timeutil};

const VALID_OUTCOMES: &[&str] = &["success", "partial", "failure"];

pub(super) fn run_outcome(
    store: &mut Store,
    id_arg: &str,
    result: &str,
    note: Option<&str>,
    json: bool,
) -> CommandOutcome {
    if !VALID_OUTCOMES.contains(&result) {
        return plan_error(
            format!(
                "Invalid --result value: {result}. Valid: {}",
                VALID_OUTCOMES.join("|")
            ),
            json,
        );
    }
    let plan_id = match resolve_plan_id(store, id_arg) {
        Ok(plan_id) => plan_id,
        Err(message) => return plan_error(message, json),
    };
    let Some(idx) = plan_by_id(store, &plan_id).ok() else {
        return plan_error(
            format!("Plan not found: {plan_id}. Run 'sd plan list' to see available plans."),
            json,
        );
    };
    let now = timeutil::now_iso();
    let outcome_note: Option<String>;
    let open_children = {
        let plan = &store.plans[idx];
        let open = plan
            .children()
            .into_iter()
            .filter(|cid| {
                store
                    .issue(cid)
                    .is_some_and(|issue| issue.status() != Some(crate::Status::Closed))
            })
            .count();
        let record = &mut store.plans[idx];
        record.set_field("outcome", json!(result));
        if let Some(note) = note {
            record.set_field("outcomeNote", json!(note));
        }
        outcome_note = note.map(String::from).or_else(|| {
            record
                .field("outcomeNote")
                .and_then(Value::as_str)
                .map(String::from)
        });
        record.set_field("updatedAt", json!(now));
        open
    };
    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }
    let mut stderr = String::new();
    if open_children > 0 {
        stderr.push_str(&format!(
            "⚠ plan {plan_id} has {open_children} open child{}\n",
            if open_children == 1 { "" } else { "ren" }
        ));
    }
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan outcome",
            "plan_id": plan_id,
            "outcome": result,
            "outcomeNote": outcome_note,
            "open_children": open_children,
        });
        return CommandOutcome {
            success: true,
            stdout: format!(
                "{}\n",
                serde_json::to_string_pretty(&envelope).expect("serializes")
            ),
            stderr,
        };
    }
    let note_suffix = outcome_note
        .as_deref()
        .filter(|note| !note.is_empty())
        .map_or_default(|note| format!(" — {note}"));
    let mut out = String::new();
    success_line(
        &mut out,
        &format!("plan {plan_id} outcome recorded: {result}{note_suffix}"),
    );
    CommandOutcome {
        success: true,
        stdout: out,
        stderr,
    }
}

pub(super) fn run_review(store: &mut Store, id_arg: &str, by: &str, json: bool) -> CommandOutcome {
    let plan_id = match resolve_plan_id(store, id_arg) {
        Ok(plan_id) => plan_id,
        Err(message) => return plan_error(message, json),
    };
    let Some(idx) = plan_by_id(store, &plan_id).ok() else {
        return plan_error(
            format!("Plan not found: {plan_id}. Run 'sd plan list' to see available plans."),
            json,
        );
    };
    let now = timeutil::now_iso();
    {
        let record = &mut store.plans[idx];
        record.set_field("reviewedBy", json!(by));
        record.set_field("updatedAt", json!(now));
    }
    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan review",
            "plan_id": plan_id,
            "reviewedBy": by,
        });
        return CommandOutcome {
            success: true,
            stdout:  format!(
                "{}\n",
                serde_json::to_string_pretty(&envelope).expect("serializes")
            ),
            stderr:  String::new(),
        };
    }
    let mut out = String::new();
    success_line(&mut out, &format!("plan {plan_id} reviewed by {by}"));
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}

/// One parsed step-metadata patch for `plan edit --step`.
struct StepPatch {
    index:    usize,
    title:    Option<String>,
    priority: Option<u8>,
    kind:     Option<String>,
}

fn parse_step_patch(
    step: Option<&str>,
    title: Option<&str>,
    priority: Option<&str>,
    kind: Option<&str>,
) -> Result<Option<StepPatch>, PlanError> {
    let any_meta = title.is_some() || priority.is_some() || kind.is_some();
    let Some(step) = step else {
        if any_meta {
            return Err(
                "--title/--priority/--type require --step <i> (the step index to edit).".to_owned(),
            );
        }
        return Ok(None);
    };
    if !any_meta {
        return Err("--step requires at least one of --title, --priority, --type.".to_owned());
    }
    let index = parse_step_flag(step)?;
    let title = match title {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Err("--title must be a non-empty string.".to_owned());
            }
            Some(trimmed.to_owned())
        }
        None => None,
    };
    let priority = match priority {
        Some(raw) => Some(
            parse_priority(raw)
                .map_err(|_| format!("--priority must be 0-4 or P0-P4 (got: {raw})."))?,
        ),
        None => None,
    };
    if let Some(kind) = kind {
        parse_seed_type(kind)
            .map_err(|_| format!("Invalid --type value: {kind}. Valid: task|bug|feature|epic"))?;
    }
    Ok(Some(StepPatch {
        index,
        title,
        priority,
        kind: kind.map(String::from),
    }))
}

#[allow(clippy::too_many_arguments, reason = "mirrors sd's flag surface")]
pub(super) fn run_edit(
    store: &mut Store,
    id_arg: &str,
    name: Option<&str>,
    section: Option<(&str, &str)>,
    step: Option<&str>,
    title: Option<&str>,
    priority: Option<&str>,
    kind: Option<&str>,
    json: bool,
) -> CommandOutcome {
    let plan_id = match resolve_plan_id(store, id_arg) {
        Ok(plan_id) => plan_id,
        Err(message) => return plan_error(message, json),
    };
    let step_patch = match parse_step_patch(step, title, priority, kind) {
        Ok(patch) => patch,
        Err(message) => return plan_error(message, json),
    };
    let mut edited_fields: Vec<String> = Vec::new();
    if name.is_some() {
        edited_fields.push("name".to_owned());
    }
    if let Some((section_name, _)) = &section {
        edited_fields.push(format!("section:{section_name}"));
    }
    if let Some(patch) = &step_patch {
        let one_based = patch.index + 1;
        if patch.title.is_some() {
            edited_fields.push(format!("step:{one_based}:title"));
        }
        if patch.priority.is_some() {
            edited_fields.push(format!("step:{one_based}:priority"));
        }
        if patch.kind.is_some() {
            edited_fields.push(format!("step:{one_based}:type"));
        }
    }
    if edited_fields.is_empty() {
        return plan_error(
            "No fields to edit. Pass at least one of: --name <text>, --section <name> <text>, \
             --step <i> --title/--priority/--type."
                .to_owned(),
            json,
        );
    }
    let next_name = match name {
        Some(raw) => match normalize_plan_name(Some(raw)) {
            Some(normalized) => Some(normalized),
            None => return plan_error("--name must be a non-empty string.".to_owned(), json),
        },
        None => None,
    };

    let Some(idx) = plan_by_id(store, &plan_id).ok() else {
        return plan_error(
            format!("Plan not found: {plan_id}. Run 'sd plan list' to see available plans."),
            json,
        );
    };
    let result = (|| -> Result<(bool, Vec<String>), PlanError> {
        let templates = load_plan_templates(store)?;
        let mut approach_changed = false;
        let old_revision = store.plans[idx].revision().unwrap_or(1);
        let now = timeutil::now_iso();
        let mut next_sections = store.plans[idx]
            .sections()
            .cloned()
            .unwrap_or_else(|| json!({}));
        let plan_template_name = store.plans[idx]
            .field("template")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        if let Some((section_name, text)) = section {
            let template = templates
                .iter()
                .find(|t| t.name == plan_template_name)
                .ok_or_else(|| {
                    format!(
                        "Plan {plan_id} references unknown template '{plan_template_name}'. \
                         Available: {}.",
                        templates
                            .iter()
                            .map(|t| t.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            let spec = template.section(section_name).ok_or_else(|| {
                format!(
                    "Unknown section '{section_name}' for template '{plan_template_name}'. \
                     Known: {}.",
                    template
                        .sections
                        .iter()
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            if !matches!(spec.kind, super::templates::SectionKind::Text) {
                let kind_label = match &spec.kind {
                    super::templates::SectionKind::Text => "text",
                    super::templates::SectionKind::List => "list",
                    super::templates::SectionKind::Steps => "steps",
                    super::templates::SectionKind::Object(_) => "object",
                };
                return Err(format!(
                    "--section editing supports kind=text only (V1). Section '{section_name}' \
                     is kind={kind_label}. Use 'sd plan submit --overwrite' for structural \
                     edits."
                ));
            }
            let min_length = spec.min_length.unwrap_or(0);
            if spec.required && text.trim().is_empty() {
                return Err(format!(
                    "Section '{section_name}' is required and cannot be empty."
                ));
            }
            if min_length > 0 && (text.chars().count() as u64) < min_length {
                return Err(format!(
                    "Section '{section_name}' must be at least {min_length} characters (got \
                     {}).",
                    text.chars().count()
                ));
            }
            let Value::Object(sections) = &mut next_sections else {
                return Err(format!("Plan {plan_id} has malformed sections"));
            };
            let prior = sections.get(section_name);
            if section_name == "approach" && prior != Some(&Value::String((*text).to_owned())) {
                approach_changed = true;
            }
            sections.insert(
                (*section_name).to_owned(),
                Value::String((*text).to_owned()),
            );
        }

        if let Some(patch) = &step_patch {
            let Some(Value::Array(steps)) = next_sections.get_mut("steps") else {
                return Err(format!(
                    "Plan {plan_id} has no steps section to edit. Use 'sd plan submit \
                     --overwrite' to add steps."
                ));
            };
            let total = steps.len();
            if patch.index >= total {
                return Err(format!(
                    "--step {} is out of range (plan {plan_id} has {total} step{}).",
                    patch.index + 1,
                    if total == 1 { "" } else { "s" }
                ));
            }
            let existing = steps[patch.index].clone();
            let Value::Object(mut map) = existing else {
                return Err(format!(
                    "Plan {plan_id} step {} is malformed",
                    patch.index + 1
                ));
            };
            if let Some(title) = &patch.title {
                map.insert("title".to_owned(), json!(title));
            }
            if let Some(priority) = patch.priority {
                map.insert("priority".to_owned(), json!(priority));
            }
            if let Some(kind) = &patch.kind {
                map.insert("type".to_owned(), json!(kind));
            }
            steps[patch.index] = Value::Object(map);
        }

        {
            let record = &mut store.plans[idx];
            let fields = record.fields_mut();
            fields.insert("sections".to_owned(), next_sections.clone());
            fields.insert("revision".to_owned(), json!(old_revision + 1));
            fields.insert("updatedAt".to_owned(), json!(now));
            if let Some(name) = &next_name {
                fields.insert("name".to_owned(), json!(name));
            }
        }

        // Child propagation: backref refresh on approach change, step
        // metadata on matching plan_step_index.
        let mut propagated: Vec<String> = Vec::new();
        if approach_changed || step_patch.is_some() {
            let plan_children = store.plans[idx].children();
            let parent_id = store.plans[idx].seed().unwrap_or_default().to_owned();
            let parent_title = store
                .issue(&parent_id)
                .map_or_default(|record| record.title().to_owned());
            let approach = next_sections.get("approach").cloned();
            for position in 0..store.issues.len() {
                let child_id = store.issues[position].id().as_str().to_owned();
                let in_plan = plan_children.contains(&child_id.as_str());
                if !in_plan {
                    continue;
                }
                if approach_changed {
                    let snapshot = store.issues[position].clone();
                    let args = backref::BackrefArgs {
                        step_index:        snapshot
                            .field("plan_step_index")
                            .and_then(Value::as_u64)
                            .and_then(|index| usize::try_from(index).ok()),
                        plan_id:           &plan_id,
                        parent_seed_id:    &parent_id,
                        parent_seed_title: &parent_title,
                        template_name:     &plan_template_name,
                        approach:          approach.as_ref(),
                    };
                    let updated = backref::apply_plan_backref(snapshot.description(), &args);
                    let record = &mut store.issues[position];
                    record.set_field("description", json!(updated));
                    record.set_field("updatedAt", json!(now));
                }
                if let Some(patch) = &step_patch {
                    let matches = store.issues[position]
                        .field("plan_step_index")
                        .and_then(Value::as_u64)
                        .is_some_and(|index| {
                            usize::try_from(index).is_ok_and(|index| index == patch.index)
                        });
                    if matches {
                        let record = &mut store.issues[position];
                        if let Some(title) = &patch.title {
                            record.set_field("title", json!(title));
                        }
                        if let Some(priority) = patch.priority {
                            record.set_field("priority", json!(priority));
                        }
                        if let Some(kind) = &patch.kind {
                            record.set_field("type", json!(kind));
                        }
                        record.set_field("updatedAt", json!(now));
                        propagated.push(child_id);
                    }
                }
            }
        }
        Ok((approach_changed, propagated))
    })();
    let (approach_changed, propagated) = match result {
        Ok(found) => found,
        Err(message) => return plan_error(message, json),
    };
    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }
    let new_revision = store.plans[idx].revision().unwrap_or(1);
    let final_name = store.plans[idx].name().map(String::from);
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan edit",
            "plan_id": plan_id,
            "revision": new_revision,
            "edited": edited_fields,
            "name": final_name,
            "backrefs_refreshed": if approach_changed {
                store.plans[idx].children().len()
            } else {
                0
            },
            "propagated_children": propagated,
        });
        return CommandOutcome {
            success: true,
            stdout:  format!(
                "{}\n",
                serde_json::to_string_pretty(&envelope).expect("serializes")
            ),
            stderr:  String::new(),
        };
    }
    let mut out = String::new();
    success_line(
        &mut out,
        &format!(
            "plan {plan_id} edited ({}); revision {new_revision}",
            edited_fields.join(", ")
        ),
    );
    if approach_changed {
        success_line(
            &mut out,
            &format!(
                "refreshed backrefs on {} child seed(s)",
                store.plans[idx].children().len()
            ),
        );
    }
    if !propagated.is_empty() {
        success_line(
            &mut out,
            &format!(
                "propagated step metadata to {} child seed(s): {}",
                propagated.len(),
                propagated.join(", ")
            ),
        );
    }
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}
