//! dep / blocked / block / unblock command semantics
//! (sd 0.5.15-compatible).

use serde_json::{Value, json};

use super::{
    CommandContext, CommandError, CommandOutcome, envelope_pretty, issue_value, push_line,
    store_has_unresolved_blockers,
};
use crate::{SeedRecord, Status, render, timeutil};

/// Removes `dep` from a record's string-array field, dropping the
/// field when the list empties (sd omits empty arrays).
fn remove_dep_field(record: &mut SeedRecord, name: &str, dep: &str) {
    let remaining: Vec<String> = record
        .fields()
        .get(name)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|id| *id != dep)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    if remaining.is_empty() {
        record.remove_field(name);
    } else {
        let value: Vec<Value> = remaining.into_iter().map(|id| json!(id)).collect();
        record.set_field(name, Value::Array(value));
    }
}

/// Typed input for `dep add`.
#[derive(Clone, Debug, Default)]
pub struct DepAddInput {
    /// `<issue> <depends-on>` positionals (wrong count reproduces the
    /// usage error).
    pub ids:  Vec<String>,
    /// JSON envelope output.
    pub json: bool,
}

/// `dep add` — records `issue blockedBy depends-on` and the reverse
/// `blocks` entry.
pub fn dep_add(ctx: &CommandContext, input: &DepAddInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<(String, String), CommandError> {
        if input.ids.len() != 2 {
            return Err(CommandError::new(
                "dep",
                "usage: sd dep add <issue> <depends-on>",
                json,
            ));
        }
        let issue_id = input.ids[0].clone();
        let depends_on_id = input.ids[1].clone();
        let mut store = ctx
            .open_store()
            .map_err(|message| CommandError::new("dep", message, json))?;
        for id in [&issue_id, &depends_on_id] {
            if store.issue(id).is_none() {
                return Err(CommandError::new(
                    "dep",
                    format!("Issue not found: {id}"),
                    json,
                ));
            }
        }
        let now = timeutil::now_iso();
        {
            let record = store.issue_mut(&issue_id).expect("existence checked above");
            record.add_blocked_by(&depends_on_id);
            record.set_field("updatedAt", json!(now));
        }
        {
            let record = store
                .issue_mut(&depends_on_id)
                .expect("existence checked above");
            let mut blocks = record.blocks();
            if !blocks.contains(&issue_id.as_str()) {
                blocks.push(issue_id.as_str());
            }
            let blocks: Vec<Value> = blocks.into_iter().map(|id| json!(id)).collect();
            record.set_field("blocks", Value::Array(blocks));
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| CommandError::new("dep", error.to_string(), json))?;
        Ok((issue_id, depends_on_id))
    })();
    match result {
        Ok((issue_id, depends_on_id)) => {
            let mut out = String::new();
            if json {
                push_line(
                    &mut out,
                    &envelope_pretty("dep add", &[
                        ("issueId", json!(issue_id)),
                        ("dependsOnId", json!(depends_on_id)),
                    ]),
                );
            } else {
                push_line(
                    &mut out,
                    &format!("Added dependency: {issue_id} → {depends_on_id}"),
                );
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `dep remove`.
#[derive(Clone, Debug, Default)]
pub struct DepRemoveInput {
    /// `<issue> <depends-on>` positionals (wrong count reproduces the
    /// usage error).
    pub ids:  Vec<String>,
    /// JSON envelope output.
    pub json: bool,
}

/// `dep remove` — drops both directions of a dependency.
pub fn dep_remove(ctx: &CommandContext, input: &DepRemoveInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<(String, String), CommandError> {
        if input.ids.len() != 2 {
            return Err(CommandError::new(
                "dep",
                "usage: sd dep remove <issue> <depends-on>",
                json,
            ));
        }
        let issue_id = input.ids[0].clone();
        let depends_on_id = input.ids[1].clone();
        let mut store = ctx
            .open_store()
            .map_err(|message| CommandError::new("dep", message, json))?;
        for id in [&issue_id, &depends_on_id] {
            if store.issue(id).is_none() {
                return Err(CommandError::new(
                    "dep",
                    format!("Issue not found: {id}"),
                    json,
                ));
            }
        }
        let now = timeutil::now_iso();
        {
            let record = store.issue_mut(&issue_id).expect("existence checked above");
            remove_dep_field(record, "blockedBy", &depends_on_id);
            record.set_field("updatedAt", json!(now));
        }
        {
            let record = store
                .issue_mut(&depends_on_id)
                .expect("existence checked above");
            remove_dep_field(record, "blocks", &issue_id);
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| CommandError::new("dep", error.to_string(), json))?;
        Ok((issue_id, depends_on_id))
    })();
    match result {
        Ok((issue_id, depends_on_id)) => {
            let mut out = String::new();
            if json {
                push_line(
                    &mut out,
                    &envelope_pretty("dep remove", &[
                        ("issueId", json!(issue_id)),
                        ("dependsOnId", json!(depends_on_id)),
                    ]),
                );
            } else {
                push_line(
                    &mut out,
                    &format!("Removed dependency: {issue_id} → {depends_on_id}"),
                );
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `dep list`.
#[derive(Clone, Debug, Default)]
pub struct DepListInput {
    /// The issue id (`None` reproduces the usage error).
    pub id:   Option<String>,
    /// JSON envelope output.
    pub json: bool,
}

/// `dep list` — one record's dependency graph.
pub fn dep_list(ctx: &CommandContext, input: &DepListInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<(crate::Store, SeedRecord), CommandError> {
        let Some(id) = input.id.clone() else {
            return Err(CommandError::new("dep", "usage: sd dep list <issue>", json));
        };
        let store = ctx
            .open_store()
            .map_err(|message| CommandError::new("dep", message, json))?;
        let record = store
            .issue(&id)
            .cloned()
            .ok_or_else(|| CommandError::new("dep", format!("Issue not found: {id}"), json))?;
        Ok((store, record))
    })();
    match result {
        Ok((store, record)) => {
            let mut out = String::new();
            if json {
                let blocked_by: Vec<Value> = record
                    .blocked_by()
                    .into_iter()
                    .map(|id| json!(id))
                    .collect();
                let blocks: Vec<Value> = record.blocks().into_iter().map(|id| json!(id)).collect();
                push_line(
                    &mut out,
                    &envelope_pretty("dep list", &[
                        ("issueId", json!(record.id().as_str())),
                        ("blockedBy", Value::Array(blocked_by)),
                        ("blocks", Value::Array(blocks)),
                    ]),
                );
            } else {
                let unresolved = |dep: &str| -> bool {
                    store
                        .issue(dep)
                        .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
                };
                let text = render::dep_list_text(&record, &store, &unresolved);
                out.push_str(&text);
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `blocked`.
#[derive(Clone, Debug, Default)]
pub struct BlockedInput {
    /// `--format` raw text.
    pub format: Option<String>,
    /// JSON envelope output.
    pub json:   bool,
}

/// `blocked` — non-closed records with at least one unresolved blocker.
pub fn blocked(ctx: &CommandContext, input: &BlockedInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<Vec<SeedRecord>, CommandError> {
        let store = ctx
            .open_store()
            .map_err(|message| CommandError::new("blocked", message, json))?;
        // Store order, non-closed records with at least one unresolved
        // blocker (a closed blocker resolves the block).
        Ok(store
            .issues
            .iter()
            .filter(|record| {
                record.status() != Some(Status::Closed)
                    && store_has_unresolved_blockers(&store, record)
            })
            .cloned()
            .collect())
    })();
    match result {
        Ok(records) => {
            let mut out = String::new();
            if json {
                let issues: Vec<Value> = records.iter().map(issue_value).collect();
                push_line(
                    &mut out,
                    &envelope_pretty("blocked", &[
                        ("issues", json!(issues)),
                        ("count", json!(issues.len())),
                    ]),
                );
            } else {
                let mode = render::render_mode(input.format.as_deref());
                let store = ctx.open_store();
                let unresolved = |dep: &str| -> bool {
                    store.as_ref().map_or(true, |store| {
                        store
                            .issue(dep)
                            .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
                    })
                };
                let text = render::list_text(&records, mode, &unresolved, "blocked");
                if records.is_empty()
                    && matches!(
                        mode,
                        render::RenderMode::Markdown
                            | render::RenderMode::Plain
                            | render::RenderMode::Json
                    )
                {
                    push_line(&mut out, "No blocked issues.");
                } else {
                    out.push_str(&text);
                }
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `block`.
#[derive(Clone, Debug, Default)]
pub struct BlockInput {
    /// The target id (`None` reproduces the usage error).
    pub id:   Option<String>,
    /// `--by <blocker-id>` (`None` reproduces the usage error).
    pub by:   Option<String>,
    /// JSON envelope output.
    pub json: bool,
}

/// `block` — dep add through the block spelling.
pub fn block(ctx: &CommandContext, input: &BlockInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<(String, String), CommandError> {
        let Some(id) = input.id.clone() else {
            return Err(CommandError::stderr_only(
                "error: missing required argument 'id'",
            ));
        };
        let Some(blocker_id) = input.by.clone() else {
            return Err(CommandError::stderr_only(
                "Error: Usage: sd block <id> --by <blocker-id>",
            ));
        };
        let mut store = ctx
            .open_store()
            .map_err(|message| CommandError::new("block", message, json))?;
        for target in [&id, &blocker_id] {
            if store.issue(target).is_none() {
                return Err(CommandError::new(
                    "block",
                    format!("Issue not found: {target}"),
                    json,
                ));
            }
        }
        let now = timeutil::now_iso();
        {
            let record = store.issue_mut(&id).expect("existence checked above");
            record.add_blocked_by(&blocker_id);
            record.set_field("updatedAt", json!(now));
        }
        {
            let record = store
                .issue_mut(&blocker_id)
                .expect("existence checked above");
            let mut blocks = record.blocks();
            if !blocks.contains(&id.as_str()) {
                blocks.push(id.as_str());
            }
            let blocks: Vec<Value> = blocks.into_iter().map(|blocked| json!(blocked)).collect();
            record.set_field("blocks", Value::Array(blocks));
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| CommandError::new("block", error.to_string(), json))?;
        Ok((id, blocker_id))
    })();
    match result {
        Ok((id, blocker_id)) => {
            let mut out = String::new();
            if json {
                push_line(
                    &mut out,
                    &envelope_pretty("block", &[
                        ("issueId", json!(id)),
                        ("blockerId", json!(blocker_id)),
                    ]),
                );
            } else {
                push_line(&mut out, &format!("{id} is now blocked by {blocker_id}"));
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `unblock`.
#[derive(Clone, Debug, Default)]
pub struct UnblockInput {
    /// The target id (`None` reproduces the usage error).
    pub id:   Option<String>,
    /// `--from <blocker-id>`.
    pub from: Option<String>,
    /// `--all` (only closed blockers).
    pub all:  bool,
    /// JSON envelope output.
    pub json: bool,
}

/// `unblock` — removes one (`--from`) or every closed (`--all`)
/// blocker.
pub fn unblock(ctx: &CommandContext, input: &UnblockInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<(String, Vec<String>), CommandError> {
        let Some(id) = input.id.clone() else {
            return Err(CommandError::stderr_only(
                "error: missing required argument 'id'",
            ));
        };
        let from = input.from.clone();
        let all = input.all;
        if from.is_none() && !all {
            return Err(CommandError::stderr_only(
                "Error: Usage: sd unblock <id> [--from <blocker-id> | --all]",
            ));
        }
        let mut store = ctx
            .open_store()
            .map_err(|message| CommandError::new("unblock", message, json))?;
        if store.issue(&id).is_none() {
            return Err(CommandError::new(
                "unblock",
                format!("Issue not found: {id}"),
                json,
            ));
        }
        let removed: Vec<String> = if let Some(blocker_id) = from {
            // Membership only — a blocker id that does not exist is
            // simply "not blocked by" it (the reference's error).
            let blocked_by = store
                .issue(&id)
                .expect("existence checked above")
                .blocked_by();
            if !blocked_by.contains(&blocker_id.as_str()) {
                return Err(CommandError::new(
                    "unblock",
                    format!("{id} is not blocked by {blocker_id}"),
                    json,
                ));
            }
            vec![blocker_id]
        } else {
            // `--all`: only blockers whose own issue is closed.
            store
                .issue(&id)
                .expect("existence checked above")
                .blocked_by()
                .into_iter()
                .filter(|dep| {
                    store
                        .issue(dep)
                        .is_some_and(|blocker| blocker.status() == Some(Status::Closed))
                })
                .map(str::to_owned)
                .collect()
        };
        let now = timeutil::now_iso();
        for blocker_id in &removed {
            {
                let record = store.issue_mut(&id).expect("existence checked above");
                remove_dep_field(record, "blockedBy", blocker_id);
                record.set_field("updatedAt", json!(now));
            }
            {
                // A dangling blocker id still unblocks the issue; the
                // reverse side only exists when the record does.
                if let Some(record) = store.issue_mut(blocker_id) {
                    remove_dep_field(record, "blocks", &id);
                    record.set_field("updatedAt", json!(now));
                }
            }
        }
        store
            .save()
            .map_err(|error| CommandError::new("unblock", error.to_string(), json))?;
        Ok((id, removed))
    })();
    match result {
        Ok((id, removed)) => {
            let mut out = String::new();
            if json {
                let removed_value: Vec<Value> = removed.iter().map(|r| json!(r)).collect();
                push_line(
                    &mut out,
                    &envelope_pretty("unblock", &[
                        ("issueId", json!(id)),
                        ("removed", Value::Array(removed_value)),
                    ]),
                );
            } else if removed.is_empty() {
                push_line(
                    &mut out,
                    &format!("No closed blockers to remove from {id}."),
                );
            } else {
                push_line(
                    &mut out,
                    &format!("{id} unblocked from {}", removed.join(", ")),
                );
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}
