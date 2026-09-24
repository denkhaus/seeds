//! `plan create`, `plan adopt`, `plan release`, `plan reorder`
//! (sd 0.5.15 `runCreate` / `runAdopt` / `runRelease` / `runReorder`).

#![allow(
    clippy::format_push_string,
    reason = "incremental push_str(&format!(…)) line building reads clearer here, matching render.rs"
)]
use serde_json::{Value, json};

use super::templates::default_template_for_type;
use super::{
    PlanError, append_unique_field, backref, find_duplicates, fresh_plan_id, load_plan_templates,
    merge_adopted_labels, normalize_plan_name, parse_step_flag, plan_by_id, plan_error,
    plan_fields, remove_list_value, resolve_plan_id, success_line,
};
use crate::commands::CommandOutcome;
use crate::{Status, Store, timeutil};

pub(super) fn run_create(
    store: &mut Store,
    seed_id: &str,
    name: Option<&str>,
    template_override: Option<&str>,
    json: bool,
) -> CommandOutcome {
    let explicit_name = normalize_plan_name(name);
    let result = (|| -> Result<(String, String), PlanError> {
        let templates = load_plan_templates(store)?;
        let Some(seed_idx) = store
            .issues
            .iter()
            .position(|record| record.id().as_str() == seed_id)
        else {
            return Err(format!("Seed not found: {seed_id}"));
        };
        let seed_type = store.issues[seed_idx]
            .seed_type()
            .map_or_else(|| "task".to_owned(), |kind| kind.as_str().to_owned());
        let template_name = template_override
            .unwrap_or(default_template_for_type(&seed_type))
            .to_owned();
        if !templates.iter().any(|t| t.name == template_name) {
            let available = templates
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "Unknown template: {template_name}. Available: {available}"
            ));
        }
        if let Some(existing) = store.plans.iter().find(|record| {
            record.seed() == Some(seed_id)
                && record.field("status").and_then(Value::as_str) != Some("draft")
        }) {
            return Ok((
                "aborted".to_owned(),
                format!(
                    "✗ plan {} already exists for {seed_id} (status: {}, revision: {})\n  Adopt \
                     seeds into it with 'sd plan adopt {} <seed-ids...>'.\n",
                    existing.id(),
                    existing
                        .field("status")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    existing.revision().unwrap_or(1),
                    existing.id(),
                ),
            ));
        }
        let now = timeutil::now_iso();
        let plan_id = fresh_plan_id(store);
        let resolved_name =
            explicit_name.or_else(|| normalize_plan_name(Some(store.issues[seed_idx].title())));
        let mut fields = plan_fields(
            &plan_id,
            seed_id,
            &template_name,
            "approved",
            1,
            json!({"steps": []}),
            &[],
            &now,
        );
        if let Some(name) = resolved_name {
            fields.insert("name".to_owned(), json!(name));
        }
        let record =
            crate::model::PlanRecord::try_from_fields(fields).map_err(|e| e.to_string())?;
        {
            let seed = &mut store.issues[seed_idx];
            seed.set_field("plan_id", json!(plan_id));
            seed.set_field("updatedAt", json!(now));
        }
        if let Some(draft_idx) = store.plans.iter().position(|p| {
            p.seed() == Some(seed_id) && p.field("status").and_then(Value::as_str) == Some("draft")
        }) {
            store.plans[draft_idx] = record;
        } else {
            store.plans.push(record);
        }
        Ok((plan_id, template_name))
    })();
    let (plan_id, tail) = match result {
        Ok(found) => found,
        Err(message) => return plan_error(message, json),
    };
    if plan_id == "aborted" {
        return CommandOutcome {
            success: false,
            stdout:  String::new(),
            stderr:  tail,
        };
    }
    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan create",
            "plan_id": plan_id,
            "parent_seed": seed_id,
            "template": tail,
            "children": [],
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
        &format!("plan {plan_id} created (status: approved, adopt-only)"),
    );
    let stderr = format!(
        "\nNext:\n  sd plan adopt {plan_id} <seed-ids...>   # populate children in order\n  sd \
         plan reorder {plan_id} <seed-ids...> # set the exact children order\n"
    );
    CommandOutcome {
        success: true,
        stdout: out,
        stderr,
    }
}

/// sd's `--at/--before/--after` positioning spec.
enum Position {
    Append,
    At(usize),
    Before(String),
    After(String),
}

fn parse_position(
    at: Option<&str>,
    before: Option<&str>,
    after: Option<&str>,
) -> Result<Position, PlanError> {
    let provided: Vec<&str> = [at.is_some(), before.is_some(), after.is_some()]
        .iter()
        .zip(["--at", "--before", "--after"])
        .filter_map(|(given, flag)| given.then_some(flag))
        .collect();
    if provided.len() > 1 {
        return Err(format!(
            "--at, --before, and --after are mutually exclusive (got: {}).",
            provided.join(", ")
        ));
    }
    if let Some(at) = at {
        let trimmed = at.trim();
        let n: usize = trimmed
            .parse()
            .map_err(|_| format!("--at must be a positive integer (got: {at})."))?;
        if n < 1 || trimmed != n.to_string() {
            return Err(format!("--at must be a positive integer (got: {at})."));
        }
        return Ok(Position::At(n - 1));
    }
    if let Some(before) = before {
        if before.trim().is_empty() {
            return Err("--before requires a seed id.".to_owned());
        }
        return Ok(Position::Before(before.to_owned()));
    }
    if let Some(after) = after {
        if after.trim().is_empty() {
            return Err("--after requires a seed id.".to_owned());
        }
        return Ok(Position::After(after.to_owned()));
    }
    Ok(Position::Append)
}

fn resolve_insert_index(
    position: &Position,
    children: &[String],
    plan_id: &str,
) -> Result<usize, PlanError> {
    match position {
        Position::Append => Ok(children.len()),
        Position::At(index) => {
            if *index > children.len() {
                return Err(format!(
                    "--at {} is out of range (plan {plan_id} has {} child{}; valid range \
                     1..{}).",
                    index + 1,
                    children.len(),
                    if children.len() == 1 { "" } else { "ren" },
                    children.len() + 1
                ));
            }
            Ok(*index)
        }
        Position::Before(anchor) => children
            .iter()
            .position(|id| id == anchor)
            .ok_or_else(|| format!("--before {anchor} is not a child of plan {plan_id}.")),
        Position::After(anchor) => children
            .iter()
            .position(|id| id == anchor)
            .map(|index| index + 1)
            .ok_or_else(|| format!("--after {anchor} is not a child of plan {plan_id}.")),
    }
}

fn check_id_list(seed_ids: &[String]) -> Result<(), PlanError> {
    if seed_ids.is_empty() {
        return Err("At least one seed id is required.".to_owned());
    }
    let dupes = find_duplicates(seed_ids);
    if !dupes.is_empty() {
        return Err(format!(
            "Duplicate seed id{} in args: {}.",
            if dupes.len() == 1 { "" } else { "s" },
            dupes.join(", ")
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, reason = "mirrors sd's flag surface")]
pub(super) fn run_adopt(
    store: &mut Store,
    plan_id_arg: &str,
    seed_ids: &[String],
    step: Option<&str>,
    at: Option<&str>,
    before: Option<&str>,
    after: Option<&str>,
    json: bool,
) -> CommandOutcome {
    let plan_id = match resolve_plan_id(store, plan_id_arg) {
        Ok(plan_id) => plan_id,
        Err(message) => return plan_error(message, json),
    };
    if let Err(message) = check_id_list(seed_ids) {
        return plan_error(message, json);
    }
    let step_index = match step.map(parse_step_flag) {
        Some(Ok(index)) => Some(index),
        Some(Err(message)) => return plan_error(message, json),
        None => None,
    };
    let position = match parse_position(at, before, after) {
        Ok(position) => position,
        Err(message) => return plan_error(message, json),
    };
    let result = (|| -> Result<(String, Vec<String>, u64), PlanError> {
        let Some(plan_idx) = plan_by_id(store, &plan_id).ok() else {
            return Err(format!(
                "Plan not found: {plan_id}. Run 'sd plan list' to see available plans."
            ));
        };
        if let Some(step_index) = step_index {
            let blueprint = store.plans[plan_idx]
                .sections()
                .and_then(|sections| sections.get("steps"))
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            if step_index >= blueprint {
                return Err(format!(
                    "--step {} is out of range (plan {plan_id} has {blueprint} step{}).",
                    step_index + 1,
                    if blueprint == 1 { "" } else { "s" }
                ));
            }
        }
        let parent_id = store.plans[plan_idx].seed().unwrap_or_default().to_owned();
        let Some(parent_idx) = store
            .issues
            .iter()
            .position(|record| record.id().as_str() == parent_id)
        else {
            return Err(format!(
                "Plan {plan_id} references parent seed {parent_id} which no longer exists."
            ));
        };
        let mut resolved: Vec<(String, usize)> = Vec::new();
        for seed_id in seed_ids {
            if seed_id == &parent_id {
                return Err(format!(
                    "cannot adopt the parent seed {seed_id} into its own plan {plan_id}."
                ));
            }
            let Some(idx) = store
                .issues
                .iter()
                .position(|record| record.id().as_str() == seed_id)
            else {
                return Err(format!("seed {seed_id} not found."));
            };
            if store.issues[idx].status() == Some(Status::Closed) {
                return Err(format!(
                    "seed {seed_id} is closed; only open or in-progress seeds can be adopted."
                ));
            }
            if let Some(attached) = store.issues[idx].field("plan_id").and_then(Value::as_str) {
                return Err(format!(
                    "seed {seed_id} is already attached to plan {attached}; release it first."
                ));
            }
            resolved.push((seed_id.clone(), idx));
        }
        let now = timeutil::now_iso();
        let template_name = store.plans[plan_idx]
            .field("template")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let approach = store.plans[plan_idx]
            .sections()
            .and_then(|sections| sections.get("approach"))
            .cloned();
        let step_labels = step_index.and_then(|index| {
            store.plans[plan_idx]
                .sections()
                .and_then(|sections| sections.get("steps"))
                .and_then(Value::as_array)
                .and_then(|steps| steps.get(index))
                .and_then(|step| step.get("labels"))
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect::<Vec<_>>()
                })
        });
        for (_, idx) in &resolved {
            let snapshot = store.issues[*idx].clone();
            let args = backref::BackrefArgs {
                step_index,
                plan_id: &plan_id,
                parent_seed_id: &parent_id,
                parent_seed_title: store.issues[parent_idx].title(),
                template_name: &template_name,
                approach: approach.as_ref(),
            };
            let description = backref::apply_plan_backref(snapshot.description(), &args);
            let record = &mut store.issues[*idx];
            record.set_field("plan_id", json!(plan_id));
            record.set_field("description", json!(description));
            if let Some(step_index) = step_index {
                record.set_field("plan_step_index", json!(step_index));
            }
            if let Some(merged) = merge_adopted_labels(record.field("labels"), step_labels.as_ref())
            {
                record.set_field("labels", json!(merged));
            }
            append_unique_field(record.fields_mut(), "blocks", &parent_id);
            record.set_field("updatedAt", json!(now));
        }
        let mut parent_blocked_by: Vec<String> = store.issues[parent_idx]
            .field("blockedBy")
            .and_then(Value::as_array)
            .map_or_default(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });
        for (seed_id, _) in &resolved {
            if !parent_blocked_by.contains(seed_id) {
                parent_blocked_by.push(seed_id.clone());
            }
        }
        {
            let parent = &mut store.issues[parent_idx];
            parent.set_field("blockedBy", json!(parent_blocked_by));
            parent.set_field("updatedAt", json!(now));
        }
        let children_now: Vec<String> = store.plans[plan_idx]
            .children()
            .into_iter()
            .map(String::from)
            .collect();
        let insert_at = resolve_insert_index(&position, &children_now, &plan_id)?;
        let adopted_ids: Vec<String> = resolved
            .iter()
            .map(|(seed_id, _)| seed_id.clone())
            .collect();
        let old_revision = store.plans[plan_idx].revision().unwrap_or(1);
        let prior_adopted: Vec<String> = store.plans[plan_idx]
            .field("adoptedChildren")
            .and_then(Value::as_array)
            .map_or_default(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });
        {
            let record = &mut store.plans[plan_idx];
            let fields = record.fields_mut();
            let mut children: Vec<String> = children_now.clone();
            let tail = children.split_off(insert_at.min(children.len()));
            children.extend(adopted_ids.iter().cloned());
            children.extend(tail);
            fields.insert("children".to_owned(), json!(children));
            let mut next_adopted: Vec<String> = prior_adopted;
            for id in &adopted_ids {
                if !next_adopted.contains(id) {
                    next_adopted.push(id.clone());
                }
            }
            fields.insert("adoptedChildren".to_owned(), json!(next_adopted));
            fields.insert("revision".to_owned(), json!(old_revision + 1));
            fields.insert("updatedAt".to_owned(), json!(now));
        }
        Ok((plan_id.clone(), adopted_ids, old_revision + 1))
    })();
    let (plan_id, adopted, revision) = match result {
        Ok(found) => found,
        Err(message) => return plan_error(message, json),
    };
    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan adopt",
            "plan_id": plan_id,
            "adopted": adopted,
            "revision": revision,
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
    for id in &adopted {
        success_line(&mut out, &format!("{id} adopted into plan {plan_id}"));
    }
    success_line(
        &mut out,
        &format!("plan {plan_id} revision bumped to {revision}"),
    );
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}

pub(super) fn run_release(
    store: &mut Store,
    plan_id_arg: &str,
    seed_ids: &[String],
    json: bool,
) -> CommandOutcome {
    let plan_id = match resolve_plan_id(store, plan_id_arg) {
        Ok(plan_id) => plan_id,
        Err(message) => return plan_error(message, json),
    };
    if let Err(message) = check_id_list(seed_ids) {
        return plan_error(message, json);
    }
    let result = (|| -> Result<(String, Vec<String>, u64), PlanError> {
        let Some(plan_idx) = plan_by_id(store, &plan_id).ok() else {
            return Err(format!(
                "Plan not found: {plan_id}. Run 'sd plan list' to see available plans."
            ));
        };
        let parent_id = store.plans[plan_idx].seed().unwrap_or_default().to_owned();
        let Some(parent_idx) = store
            .issues
            .iter()
            .position(|record| record.id().as_str() == parent_id)
        else {
            return Err(format!(
                "Plan {plan_id} references parent seed {parent_id} which no longer exists."
            ));
        };
        let mut resolved: Vec<(String, usize)> = Vec::new();
        for seed_id in seed_ids {
            if seed_id == &parent_id {
                return Err(format!(
                    "cannot release the parent seed {seed_id} from its own plan {plan_id}."
                ));
            }
            let Some(idx) = store
                .issues
                .iter()
                .position(|record| record.id().as_str() == seed_id)
            else {
                return Err(format!("seed {seed_id} not found."));
            };
            match store.issues[idx].field("plan_id").and_then(Value::as_str) {
                Some(attached) if attached != plan_id => {
                    return Err(format!(
                        "seed {seed_id} is attached to plan {attached}, not {plan_id}."
                    ));
                }
                Some(_) => {}
                None => {
                    return Err(format!("seed {seed_id} is not attached to plan {plan_id}."));
                }
            }
            resolved.push((seed_id.clone(), idx));
        }
        let now = timeutil::now_iso();
        for (_, idx) in &resolved {
            let snapshot = store.issues[*idx].clone();
            let description = snapshot.description().and_then(backref::strip_plan_backref);
            let record = &mut store.issues[*idx];
            match description {
                Some(text) => record.set_field("description", json!(text)),
                None => record.remove_field("description"),
            }
            record.remove_field("plan_id");
            record.remove_field("plan_step_index");
            remove_list_value(record.fields_mut(), "blocks", &parent_id);
            record.set_field("updatedAt", json!(now));
        }
        let released: Vec<String> = resolved.iter().map(|(id, _)| id.clone()).collect();
        {
            let parent = &mut store.issues[parent_idx];
            let kept: Vec<String> = parent
                .field("blockedBy")
                .and_then(Value::as_array)
                .map_or_default(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .filter(|id| !released.contains(&id.to_string()))
                        .map(String::from)
                        .collect()
                });
            parent.set_field("blockedBy", json!(kept));
            parent.set_field("updatedAt", json!(now));
        }
        let old_revision = store.plans[plan_idx].revision().unwrap_or(1);
        let kept_children: Vec<String> = store.plans[plan_idx]
            .children()
            .into_iter()
            .filter(|id| !released.iter().any(|r| r == id))
            .map(String::from)
            .collect();
        let kept_adopted: Vec<String> = store.plans[plan_idx]
            .field("adoptedChildren")
            .and_then(Value::as_array)
            .map_or_default(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|id| !released.iter().any(|r| r == id))
                    .map(String::from)
                    .collect()
            });
        {
            let record = &mut store.plans[plan_idx];
            let fields = record.fields_mut();
            fields.insert("children".to_owned(), json!(kept_children));
            if kept_adopted.is_empty() {
                fields.remove("adoptedChildren");
            } else {
                fields.insert("adoptedChildren".to_owned(), json!(kept_adopted));
            }
            fields.insert("revision".to_owned(), json!(old_revision + 1));
            fields.insert("updatedAt".to_owned(), json!(now));
        }
        Ok((plan_id.clone(), released, old_revision + 1))
    })();
    let (plan_id, released, revision) = match result {
        Ok(found) => found,
        Err(message) => return plan_error(message, json),
    };
    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan release",
            "plan_id": plan_id,
            "released": released,
            "revision": revision,
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
    for id in &released {
        success_line(&mut out, &format!("{id} released from plan {plan_id}"));
    }
    success_line(
        &mut out,
        &format!("plan {plan_id} revision bumped to {revision}"),
    );
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}

pub(super) fn run_reorder(
    store: &mut Store,
    plan_id_arg: &str,
    seed_ids: &[String],
    json: bool,
) -> CommandOutcome {
    let plan_id = match resolve_plan_id(store, plan_id_arg) {
        Ok(plan_id) => plan_id,
        Err(message) => return plan_error(message, json),
    };
    if let Err(message) = check_id_list(seed_ids) {
        return plan_error(message, json);
    }
    let result = (|| -> Result<(String, Vec<String>, u64), PlanError> {
        let Some(plan_idx) = plan_by_id(store, &plan_id).ok() else {
            return Err(format!(
                "Plan not found: {plan_id}. Run 'sd plan list' to see available plans."
            ));
        };
        let current = store.plans[plan_idx].children();
        let extra: Vec<String> = seed_ids
            .iter()
            .filter(|id| !current.contains(&id.as_str()))
            .cloned()
            .collect();
        if !extra.is_empty() {
            let names = extra.join(", ");
            return Err(format!(
                "{names} {} not {} of plan {plan_id}. Adopt first with 'sd plan adopt'.",
                if extra.len() == 1 { "is" } else { "are" },
                if extra.len() == 1 {
                    "a child"
                } else {
                    "children"
                }
            ));
        }
        let missing: Vec<String> = current
            .into_iter()
            .filter(|id| !seed_ids.iter().any(|s| s == id))
            .map(String::from)
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "reorder must list every child exactly once; missing: {}. Use 'sd plan \
                 release' to drop a child.",
                missing.join(", ")
            ));
        }
        let now = timeutil::now_iso();
        let old_revision = store.plans[plan_idx].revision().unwrap_or(1);
        {
            let record = &mut store.plans[plan_idx];
            let fields = record.fields_mut();
            fields.insert("children".to_owned(), json!(seed_ids));
            fields.insert("revision".to_owned(), json!(old_revision + 1));
            fields.insert("updatedAt".to_owned(), json!(now));
        }
        Ok((plan_id.clone(), seed_ids.to_vec(), old_revision + 1))
    })();
    let (plan_id, children, revision) = match result {
        Ok(found) => found,
        Err(message) => return plan_error(message, json),
    };
    if let Err(save_error) = store.save() {
        return plan_error(save_error.to_string(), json);
    }
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan reorder",
            "plan_id": plan_id,
            "children": children,
            "revision": revision,
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
        &format!("plan {plan_id} children reordered (revision {revision})"),
    );
    success_line(&mut out, &format!("order: {}", children.join(", ")));
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}
