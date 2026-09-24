//! `plan submit` — validate a plan, spawn children, write the
//! plans.jsonl row (sd 0.5.15 `runSubmit` + `applyOverwrite`).

#![allow(
    clippy::format_push_string,
    reason = "incremental push_str(&format!(…)) line building reads clearer here, matching render.rs"
)]
use serde_json::{Value, json};

use super::{
    PlanError, append_unique_field, backref, child_fields, fresh_child_id, fresh_plan_id,
    load_plan_templates, merge_adopted_labels, mulch, normalize_labels, normalize_plan_name,
    plan_error, plan_fields, schema, success_line,
};
use crate::commands::CommandOutcome;
use crate::model::{Fields, PlanRecord, SeedRecord};
use crate::{Status, Store, timeutil};

/// One step of the submitted plan.
struct Step<'a> {
    title:         Option<&'a str>,
    kind:          Option<&'a str>,
    priority:      Option<u64>,
    blocks:        Vec<i64>,
    plan_template: Option<&'a str>,
    existing_seed: Option<&'a str>,
    labels:        Option<Vec<String>>,
}

fn step_of(value: &Value) -> Step<'_> {
    let str_field = |key: &str| value.get(key).and_then(Value::as_str);
    Step {
        title:         str_field("title").filter(|t| !t.is_empty()),
        kind:          str_field("type"),
        priority:      value.get("priority").and_then(Value::as_u64),
        blocks:        value
            .get("blocks")
            .and_then(Value::as_array)
            .map_or_default(|items| items.iter().filter_map(Value::as_i64).collect::<Vec<_>>()),
        plan_template: str_field("plan_template"),
        existing_seed: str_field("existing_seed"),
        labels:        value.get("labels").and_then(Value::as_array).map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        }),
    }
}

/// One validated submit-time adoption: the issue index and id of the
/// adopted seed.
#[derive(Clone)]
struct Adoption {
    seed_idx: usize,
    seed_id:  String,
}

struct Adoptions {
    by_step: Vec<Option<Adoption>>,
}

fn validate_adoptions(
    steps: &[Value],
    seed_id: &str,
    store: &Store,
    allowed_current_plan: Option<&str>,
    warnings: &mut String,
) -> Result<Adoptions, PlanError> {
    let mut by_step = vec![None; steps.len()];
    let mut seen: Vec<String> = Vec::new();
    for (index, value) in steps.iter().enumerate() {
        let step = step_of(value);
        let Some(adopt_id) = step.existing_seed else {
            continue;
        };
        let label = match step.title {
            Some(title) => format!("step {} ({title})", index + 1),
            None => format!("step {} (adopt {adopt_id})", index + 1),
        };
        if step.plan_template.is_some() {
            return Err(format!(
                "{label}: existing_seed and plan_template are mutually exclusive — adoption \
                 replaces spawning, so a sub-plan template cannot apply."
            ));
        }
        if adopt_id == seed_id {
            return Err(format!(
                "{label}: cannot adopt the parent seed {seed_id} into its own plan."
            ));
        }
        if seen.iter().any(|id| id == adopt_id) {
            return Err(format!(
                "{label}: existing_seed {adopt_id} is already adopted by an earlier step in \
                 this plan."
            ));
        }
        let Some(seed_idx) = store
            .issues
            .iter()
            .position(|record| record.id().as_str() == adopt_id)
        else {
            return Err(format!("{label}: existing_seed {adopt_id} not found."));
        };
        if store.issues[seed_idx].status() == Some(Status::Closed) {
            return Err(format!(
                "{label}: existing_seed {adopt_id} is closed; only open or in-progress seeds \
                 can be adopted."
            ));
        }
        if let Some(plan_id) = store.issues[seed_idx]
            .field("plan_id")
            .and_then(Value::as_str)
            && Some(plan_id) != allowed_current_plan
        {
            return Err(format!(
                "{label}: existing_seed {adopt_id} is already attached to plan {plan_id}."
            ));
        }
        let seed_title = store.issues[seed_idx].title();
        if let Some(title) = step.title
            && seed_title != title
        {
            warnings.push_str(&format!(
                "⚠ step {}: existing_seed {adopt_id} title \"{seed_title}\" differs from \
                     step.title \"{title}\"; seed title is preserved.\n",
                index + 1
            ));
        }
        seen.push(adopt_id.to_owned());
        by_step[index] = Some(Adoption {
            seed_idx,
            seed_id: adopt_id.to_owned(),
        });
    }
    Ok(Adoptions { by_step })
}

/// The per-run mutation result feeding submit's output paths.
#[derive(Default)]
struct SubmitOutcome {
    aborted:     bool,
    plan_id:     String,
    children:    Vec<String>,
    revision:    u64,
    obsolete:    Vec<String>,
    status:      String,
    reviewed_by: Option<String>,
    stderr:      String,
}

pub(super) fn run(
    store: &mut Store,
    seed_id: &str,
    plan_file: &str,
    plan_stdin: Option<&str>,
    overwrite: bool,
    record_decision: bool,
    domain_override: Option<&str>,
    name_override: Option<&str>,
    json: bool,
) -> CommandOutcome {
    let raw = match plan_stdin {
        Some(content) => content.to_owned(),
        None => match std::fs::read_to_string(plan_file) {
            Ok(text) => text,
            Err(_) => return plan_error(format!("Plan file not found: {plan_file}"), json),
        },
    };
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(parsed) => parsed,
        Err(error) => return plan_error(format!("Invalid JSON in plan file: {error}"), json),
    };
    let template_name = parsed
        .get("template")
        .and_then(Value::as_str)
        .unwrap_or("feature")
        .to_owned();
    let templates = match load_plan_templates(store) {
        Ok(templates) => templates,
        Err(message) => return plan_error(message, json),
    };
    let available = templates
        .iter()
        .map(|t| t.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let Some(template) = templates.iter().find(|t| t.name == template_name) else {
        return plan_error(
            format!("Unknown template in plan: {template_name}. Available: {available}"),
            json,
        );
    };
    if let Err(errors) = schema::validate_plan(template, &parsed) {
        let diff = schema::PartialStateDiff {
            errors:  &errors,
            current: &parsed,
        };
        return CommandOutcome {
            success: false,
            stdout:  String::new(),
            stderr:  format!(
                "{}\n",
                serde_json::to_string_pretty(&diff).expect("serializes")
            ),
        };
    }
    let steps_raw = parsed["sections"]["steps"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for (index, step) in steps_raw.iter().enumerate() {
        if let Some(reference) = step
            .get("plan_template")
            .and_then(Value::as_str)
            .filter(|reference| !templates.iter().any(|t| t.name == *reference))
        {
            return CommandOutcome {
                success: false,
                stdout:  String::new(),
                stderr:  format!(
                    "step {} ({}): plan_template '{reference}' is not defined. Available: \
                     {available}. Add it under plan_templates: in .seeds/config.yaml.\n",
                    index + 1,
                    step.get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("untitled")
                ),
            };
        }
    }

    let explicit_name: Option<String> = normalize_plan_name(name_override)
        .or_else(|| normalize_plan_name(parsed.get("name").and_then(Value::as_str)));

    let mut warnings = String::new();
    let result = (|| -> Result<SubmitOutcome, PlanError> {
        let Some(seed_idx) = store
            .issues
            .iter()
            .position(|record| record.id().as_str() == seed_id)
        else {
            return Err(format!("Seed not found: {seed_id}"));
        };
        let existing_idx = store.plans.iter().position(|record| {
            record.seed() == Some(seed_id)
                && record.field("status").and_then(Value::as_str) != Some("draft")
        });
        if let Some(idx) = existing_idx {
            if !overwrite {
                let existing = &store.plans[idx];
                return Ok(SubmitOutcome {
                    aborted: true,
                    stderr: format!(
                        "✗ plan {} already exists for {seed_id} (status: {}, revision: {})\n  \
                         Use --overwrite to replace it.\n",
                        existing.id(),
                        existing
                            .field("status")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        existing.revision().unwrap_or(1)
                    ),
                    ..SubmitOutcome::default()
                });
            }
            return apply_overwrite(
                store,
                seed_idx,
                idx,
                &steps_raw,
                &template_name,
                parsed["sections"].clone(),
                explicit_name.as_deref(),
                &mut warnings,
            );
        }
        fresh_submit(
            store,
            seed_idx,
            &steps_raw,
            &template_name,
            parsed["sections"].clone(),
            explicit_name.as_deref(),
            &mut warnings,
        )
    })();
    let mut outcome = match result {
        Ok(outcome) => outcome,
        Err(message) => return plan_error(message, json),
    };
    outcome.stderr.insert_str(0, &warnings);
    if outcome.aborted {
        return CommandOutcome {
            success: false,
            stdout:  String::new(),
            stderr:  outcome.stderr,
        };
    }
    if outcome.aborted {
        return CommandOutcome {
            success: false,
            stdout:  String::new(),
            stderr:  outcome.stderr,
        };
    }

    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }

    let mut recorded_mulch = None;
    if record_decision && store.issue(seed_id).is_some() {
        let cwd = store.root().parent().unwrap_or(store.root()).to_path_buf();
        let approach = parsed["sections"]
            .get("approach")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let domain: Option<String> = domain_override
            .map(String::from)
            .or_else(|| mulch::infer_domain(None, store.issue(seed_id)?, &cwd));
        match domain {
            Some(domain) => {
                let title = store
                    .issue(seed_id)
                    .map_or_default(|record| record.title().to_owned());
                match mulch::record_decision(&domain, &outcome.plan_id, &title, &approach, &cwd) {
                    Ok(Some(id)) => recorded_mulch = Some(id),
                    Ok(None) => {}
                    Err(reason) => {
                        outcome
                            .stderr
                            .push_str(&format!("⚠ --record-decision: {reason}\n"));
                    }
                }
            }
            None => outcome
                .stderr
                .push_str("⚠ --record-decision: no mulch domain inferred (skipping)\n"),
        }
    }

    if !outcome.obsolete.is_empty() {
        for id in &outcome.obsolete {
            outcome.stderr.push_str(&format!(
                "sd close {id} --reason \"obsoleted by plan {} revision {}\"\n",
                outcome.plan_id, outcome.revision
            ));
        }
    }

    if json {
        let envelope = json!({
            "success": true,
            "command": "plan submit",
            "plan_id": outcome.plan_id,
            "children": outcome.children,
            "parent_seed": seed_id,
            "revision": outcome.revision,
            "obsolete": outcome.obsolete,
            "overwritten": outcome.revision > 1,
        });
        return CommandOutcome {
            success: true,
            stdout:  format!(
                "{}\n",
                serde_json::to_string_pretty(&envelope).expect("serializes")
            ),
            stderr:  outcome.stderr,
        };
    }
    let mut out = String::new();
    if outcome.revision > 1 {
        success_line(
            &mut out,
            &format!(
                "plan {} overwritten (revision {}, status: approved)",
                outcome.plan_id, outcome.revision
            ),
        );
    } else {
        success_line(
            &mut out,
            &format!("plan {} created (status: approved)", outcome.plan_id),
        );
    }
    let child_plural = if outcome.children.len() == 1 { "" } else { "s" };
    success_line(
        &mut out,
        &format!(
            "{} child seed{child_plural}: {}",
            outcome.children.len(),
            outcome.children.join(", ")
        ),
    );
    if !outcome.obsolete.is_empty() {
        success_line(
            &mut out,
            &format!(
                "{} obsolete child seed{} flagged (see stderr for close suggestions)",
                outcome.obsolete.len(),
                if outcome.obsolete.len() == 1 { "" } else { "s" }
            ),
        );
    }
    success_line(
        &mut out,
        &format!(
            "{seed_id} now blocked by {} children",
            outcome.children.len()
        ),
    );
    if let Some(id) = recorded_mulch {
        success_line(&mut out, &format!("recorded mulch decision {id}"));
    }
    let reviewable =
        outcome.reviewed_by.is_none() && matches!(outcome.status.as_str(), "approved" | "active");
    outcome.stderr.push_str(&format!(
        "\nNext:\n  sd plan show {}          # review the plan as a unit\n  sd ready                      # pick up the first child step\n",
        outcome.plan_id
    ));
    if reviewable {
        outcome.stderr.push_str(&format!(
            "  sd plan review {} --by <name>   # record approval (optional)\n",
            outcome.plan_id
        ));
    }
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  outcome.stderr,
    }
}

/// Backref args for the plan being submitted.
fn backref_args<'a>(
    step_index: Option<usize>,
    plan_id: &'a str,
    seed: &'a SeedRecord,
    template_name: &'a str,
    sections: &'a Value,
) -> backref::BackrefArgs<'a> {
    backref::BackrefArgs {
        step_index,
        plan_id,
        parent_seed_id: seed.id().as_str(),
        parent_seed_title: seed.title(),
        template_name,
        approach: sections.get("approach"),
    }
}

fn normalized_step_labels(step: &Step<'_>) -> Option<Vec<String>> {
    let labels = step.labels.as_ref().filter(|labels| !labels.is_empty())?;
    let normalized = normalize_labels(labels);
    let mut deduped: Vec<String> = Vec::new();
    for label in normalized {
        if !deduped.contains(&label) {
            deduped.push(label);
        }
    }
    (!deduped.is_empty()).then_some(deduped)
}

/// Builds the fields of a freshly spawned child (shared by both
/// submit paths).
#[allow(clippy::too_many_arguments, reason = "mirrors sd's child-spawn inputs")]
fn spawn_fields(
    child_id: &str,
    step: &Step<'_>,
    index: usize,
    seed_id: &str,
    seed_title: &str,
    plan_id: &str,
    template_name: &str,
    sections: &Value,
    now: &str,
    initial_blocks_parent: bool,
) -> Fields {
    let args = backref::BackrefArgs {
        step_index: Some(index),
        plan_id,
        parent_seed_id: seed_id,
        parent_seed_title: seed_title,
        template_name,
        approach: sections.get("approach"),
    };
    let description = backref::build_plan_backref(&args);
    let Some(title) = step.title else {
        unreachable!("validate_plan guarantees title on spawn steps");
    };
    let mut fields = child_fields(
        child_id,
        title,
        step.kind.unwrap_or("task"),
        step.priority.unwrap_or(2),
        index,
        &description,
        now,
    );
    if let Some(labels) = normalized_step_labels(step) {
        fields.insert("labels".to_owned(), json!(labels));
    }
    if step.plan_template.is_some() {
        fields.insert("requires_plan".to_owned(), json!(true));
    } else {
        fields.insert("plan_id".to_owned(), json!(plan_id));
    }
    if initial_blocks_parent {
        fields.insert("blocks".to_owned(), json!([seed_id]));
    }
    fields
}

#[allow(clippy::too_many_arguments, reason = "mirrors sd's submit pipeline")]
fn fresh_submit(
    store: &mut Store,
    seed_idx: usize,
    steps_raw: &[Value],
    template_name: &str,
    sections: Value,
    explicit_name: Option<&str>,
    warnings: &mut String,
) -> Result<SubmitOutcome, PlanError> {
    let seed_id = store.issues[seed_idx].id().as_str().to_owned();
    let adoptions = validate_adoptions(steps_raw, &seed_id, store, None, warnings)?;
    let now = timeutil::now_iso();
    let plan_id = fresh_plan_id(store);

    let mut taken: Vec<String> = Vec::new();
    let final_child_ids: Vec<String> = steps_raw
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if let Some(adoption) = adoptions.by_step.get(index).and_then(Option::as_ref) {
                return adoption.seed_id.clone();
            }
            fresh_child_id(store, &mut taken)
        })
        .collect();

    // Build fresh children; mutate adopted seeds in place.
    let mut new_issues: Vec<Fields> = Vec::new();
    for (index, value) in steps_raw.iter().enumerate() {
        let step = step_of(value);
        let Some(child_id) = final_child_ids.get(index) else {
            continue;
        };
        if let Some(adoption) = adoptions.by_step.get(index).and_then(Option::as_ref) {
            let parent_snapshot = store.issues[seed_idx].clone();
            let adopted_snapshot = store.issues[adoption.seed_idx].clone();
            let args = backref_args(
                Some(index),
                &plan_id,
                &parent_snapshot,
                template_name,
                &sections,
            );
            let updated = backref::apply_plan_backref(adopted_snapshot.description(), &args);
            let record = &mut store.issues[adoption.seed_idx];
            record.set_field("plan_id", json!(plan_id));
            record.set_field("plan_step_index", json!(index));
            record.set_field("description", json!(updated));
            if let Some(merged) = merge_adopted_labels(record.field("labels"), step.labels.as_ref())
            {
                record.set_field("labels", json!(merged));
            }
            record.set_field("updatedAt", json!(now));
            continue;
        }
        let seed_title = store.issues[seed_idx].title().to_owned();
        new_issues.push(spawn_fields(
            child_id,
            &step,
            index,
            &seed_id,
            &seed_title,
            &plan_id,
            template_name,
            &sections,
            &now,
            false,
        ));
    }

    // Unified edge wiring: forward blocks first, then the parent edge.
    let update_child =
        |store: &mut Store, new_issues: &mut [Fields], step_idx: usize, key: &str, id: &str| {
            if let Some(adoption) = adoptions.by_step.get(step_idx).and_then(Option::as_ref) {
                append_unique_field(store.issues[adoption.seed_idx].fields_mut(), key, id);
                return;
            }
            let Some(child_id) = final_child_ids.get(step_idx) else {
                return;
            };
            if let Some(fields) = new_issues
                .iter_mut()
                .find(|fields| fields.get("id").and_then(Value::as_str) == Some(child_id.as_str()))
            {
                append_unique_field(fields, key, id);
            }
        };
    for (index, value) in steps_raw.iter().enumerate() {
        let step = step_of(value);
        let Some(source_id) = final_child_ids.get(index) else {
            continue;
        };
        for block in &step.blocks {
            let Some(target_idx) = usize::try_from(block - 1).ok() else {
                continue;
            };
            let Some(target_id) = final_child_ids.get(target_idx) else {
                continue;
            };
            update_child(store, &mut new_issues, index, "blocks", target_id);
            update_child(store, &mut new_issues, target_idx, "blockedBy", source_id);
        }
    }
    for index in 0..steps_raw.len() {
        update_child(store, &mut new_issues, index, "blocks", &seed_id);
    }

    // Parent seed: plan_id + deduped blockedBy.
    {
        let parent = &mut store.issues[seed_idx];
        let mut blocked_by: Vec<String> = parent
            .field("blockedBy")
            .and_then(Value::as_array)
            .map_or_default(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });
        for id in &final_child_ids {
            if !blocked_by.contains(id) {
                blocked_by.push(id.clone());
            }
        }
        let fields = parent.fields_mut();
        fields.insert("plan_id".to_owned(), json!(plan_id));
        fields.insert("blockedBy".to_owned(), json!(blocked_by));
        fields.insert("updatedAt".to_owned(), json!(now));
    }

    let resolved_name = explicit_name
        .map(String::from)
        .or_else(|| normalize_plan_name(Some(store.issues[seed_idx].title())));
    let mut fields = plan_fields(
        &plan_id,
        &seed_id,
        template_name,
        "approved",
        1,
        sections,
        &final_child_ids,
        &now,
    );
    if let Some(name) = resolved_name {
        fields.insert("name".to_owned(), json!(name));
    }
    let submit_adopted: Vec<String> = adoptions
        .by_step
        .iter()
        .flatten()
        .map(|adoption| adoption.seed_id.clone())
        .collect();
    if !submit_adopted.is_empty() {
        fields.insert("adoptedChildren".to_owned(), json!(submit_adopted));
    }
    let record = PlanRecord::try_from_fields(fields).map_err(|error| error.to_string())?;
    for fields in new_issues {
        let record = SeedRecord::try_from_fields(fields).map_err(|error| error.to_string())?;
        store.issues.push(record);
    }
    if let Some(draft_idx) = store.plans.iter().position(|record| {
        record.seed() == Some(seed_id.as_str())
            && record.field("status").and_then(Value::as_str) == Some("draft")
    }) {
        store.plans[draft_idx] = record;
    } else {
        store.plans.push(record);
    }
    Ok(SubmitOutcome {
        aborted: false,
        plan_id,
        children: final_child_ids,
        revision: 1,
        obsolete: Vec::new(),
        status: "approved".to_owned(),
        reviewed_by: None,
        stderr: String::new(),
    })
}

#[allow(clippy::too_many_arguments, reason = "mirrors sd's overwrite pipeline")]
fn apply_overwrite(
    store: &mut Store,
    seed_idx: usize,
    plan_idx: usize,
    steps_raw: &[Value],
    template_name: &str,
    sections: Value,
    explicit_name: Option<&str>,
    warnings: &mut String,
) -> Result<SubmitOutcome, PlanError> {
    let seed_id = store.issues[seed_idx].id().as_str().to_owned();
    let plan_id = store.plans[plan_idx].id().as_str().to_owned();
    let old_children: Vec<String> = store.plans[plan_idx]
        .children()
        .into_iter()
        .map(String::from)
        .collect();
    let adoptions =
        validate_adoptions(steps_raw, &seed_id, store, Some(plan_id.as_str()), warnings)?;
    let now = timeutil::now_iso();

    let old_child_issues: Vec<usize> = old_children
        .iter()
        .filter_map(|id| {
            store
                .issues
                .iter()
                .position(|record| record.id().as_str() == id)
        })
        .collect();
    let mut used_old_ids: Vec<String> = Vec::new();
    let mut adopted_external: Vec<String> = Vec::new();
    let mut final_child_ids: Vec<String> = Vec::new();
    let mut new_spawned: Vec<Fields> = Vec::new();
    let mut taken: Vec<String> = Vec::new();

    for (index, value) in steps_raw.iter().enumerate() {
        let step = step_of(value);
        if let Some(adoption) = adoptions.by_step.get(index).and_then(Option::as_ref) {
            final_child_ids.push(adoption.seed_id.clone());
            if old_children.contains(&adoption.seed_id) {
                used_old_ids.push(adoption.seed_id.clone());
            } else {
                adopted_external.push(adoption.seed_id.clone());
            }
            continue;
        }
        let title = step.title.unwrap_or_default();
        let matched = old_child_issues.iter().find_map(|&idx| {
            let id = store.issues[idx].id().as_str().to_owned();
            (!used_old_ids.contains(&id) && store.issues[idx].title() == title).then_some(id)
        });
        if let Some(id) = matched {
            used_old_ids.push(id.clone());
            final_child_ids.push(id);
        } else {
            let id = fresh_child_id(store, &mut taken);
            final_child_ids.push(id);
        }
    }

    for (index, value) in steps_raw.iter().enumerate() {
        let step = step_of(value);
        let Some(child_id) = final_child_ids.get(index) else {
            continue;
        };
        let is_matched = used_old_ids.contains(child_id);
        let is_external = adopted_external.contains(child_id);
        if is_matched || is_external {
            let Some(idx) = store
                .issues
                .iter()
                .position(|record| record.id().as_str() == child_id.as_str())
            else {
                continue;
            };
            let parent_snapshot = store.issues[seed_idx].clone();
            let child_snapshot = store.issues[idx].clone();
            let args = backref_args(
                Some(index),
                &plan_id,
                &parent_snapshot,
                template_name,
                &sections,
            );
            let updated = backref::apply_plan_backref(child_snapshot.description(), &args);
            let record = &mut store.issues[idx];
            if is_external {
                record.set_field("plan_id", json!(plan_id));
                record.set_field("plan_step_index", json!(index));
            }
            record.set_field("description", json!(updated));
            if let Some(merged) = merge_adopted_labels(record.field("labels"), step.labels.as_ref())
            {
                record.set_field("labels", json!(merged));
            }
            record.set_field("updatedAt", json!(now));
            continue;
        }
        let seed_title = store.issues[seed_idx].title().to_owned();
        new_spawned.push(spawn_fields(
            child_id,
            &step,
            index,
            &seed_id,
            &seed_title,
            &plan_id,
            template_name,
            &sections,
            &now,
            true,
        ));
    }

    // Wire step.blocks edges in both directions (deduped).
    let update_matched = |store: &mut Store, id: &str, key: &str, value: &str| {
        let Some(idx) = store
            .issues
            .iter()
            .position(|record| record.id().as_str() == id)
        else {
            return;
        };
        append_unique_field(store.issues[idx].fields_mut(), key, value);
    };
    for (index, value) in steps_raw.iter().enumerate() {
        let step = step_of(value);
        let Some(source_id) = final_child_ids.get(index) else {
            continue;
        };
        for block in &step.blocks {
            let Some(target_idx) = usize::try_from(block - 1).ok() else {
                continue;
            };
            let Some(target_id) = final_child_ids.get(target_idx) else {
                continue;
            };
            if let Some(fields) = new_spawned
                .iter_mut()
                .find(|fields| fields.get("id").and_then(Value::as_str) == Some(source_id.as_str()))
            {
                append_unique_field(fields, "blocks", target_id);
            } else {
                update_matched(store, source_id, "blocks", target_id);
            }
            if let Some(fields) = new_spawned
                .iter_mut()
                .find(|fields| fields.get("id").and_then(Value::as_str) == Some(target_id.as_str()))
            {
                append_unique_field(fields, "blockedBy", source_id);
            } else {
                update_matched(store, target_id, "blockedBy", source_id);
            }
        }
    }
    for child_id in &adopted_external {
        update_matched(store, child_id, "blocks", &seed_id);
    }

    let obsolete: Vec<String> = old_children
        .iter()
        .filter(|id| !used_old_ids.contains(id))
        .cloned()
        .collect();

    // Parent seed: external blockers first, then the final children.
    {
        let parent = &mut store.issues[seed_idx];
        let external: Vec<String> = parent
            .field("blockedBy")
            .and_then(Value::as_array)
            .map_or_default(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|id| {
                        !old_children.contains(&id.to_string())
                            && !final_child_ids.iter().any(|c| c == id)
                    })
                    .map(String::from)
                    .collect()
            });
        let mut blocked_by = external;
        blocked_by.extend(final_child_ids.iter().cloned());
        let fields = parent.fields_mut();
        fields.insert("plan_id".to_owned(), json!(plan_id));
        fields.insert("blockedBy".to_owned(), json!(blocked_by));
        fields.insert("updatedAt".to_owned(), json!(now));
    }

    // Plan row update: preserve surviving adoptions, merge new ones.
    let surviving: Vec<String> = store.plans[plan_idx]
        .field("adoptedChildren")
        .and_then(Value::as_array)
        .map_or_default(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|id| final_child_ids.iter().any(|c| c == id))
                .map(String::from)
                .collect()
        });
    let mut merged_adopted = surviving;
    for id in &adopted_external {
        if !merged_adopted.contains(id) {
            merged_adopted.push(id.clone());
        }
    }
    let old_revision = store.plans[plan_idx].revision().unwrap_or(1);
    {
        let record = &mut store.plans[plan_idx];
        let fields = record.fields_mut();
        fields.insert("template".to_owned(), json!(template_name));
        fields.insert("sections".to_owned(), sections);
        fields.insert("children".to_owned(), json!(final_child_ids));
        fields.insert("revision".to_owned(), json!(old_revision + 1));
        fields.insert("updatedAt".to_owned(), json!(now));
        if merged_adopted.is_empty() {
            fields.remove("adoptedChildren");
        } else {
            fields.insert("adoptedChildren".to_owned(), json!(merged_adopted));
        }
        if let Some(name) = &explicit_name {
            fields.insert("name".to_owned(), json!(name));
        }
    }
    for fields in new_spawned {
        let record = SeedRecord::try_from_fields(fields).map_err(|error| error.to_string())?;
        store.issues.push(record);
    }
    let plan = &store.plans[plan_idx];
    Ok(SubmitOutcome {
        aborted: false,
        plan_id,
        children: final_child_ids,
        revision: old_revision + 1,
        obsolete,
        status: plan
            .field("status")
            .and_then(Value::as_str)
            .unwrap_or("approved")
            .to_owned(),
        reviewed_by: plan
            .field("reviewedBy")
            .and_then(Value::as_str)
            .map(String::from),
        stderr: String::new(),
    })
}
