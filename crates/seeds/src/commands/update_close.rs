//! update and close command semantics (sd 0.5.15-compatible).

use serde_json::{Map, Value, json};

use super::{
    CommandContext, CommandError, CommandOutcome, comma_list, envelope_pretty, issue_value,
    parse_priority, parse_seed_type, push_line,
};
use crate::{SeedRecord, Status, timeutil};

/// Typed input for `update`. Option values are raw strings validated
/// library-side; `None` leaves the field untouched.
#[derive(Clone, Debug, Default)]
pub struct UpdateInput {
    /// The target id (`None` reproduces the usage error).
    pub id:               Option<String>,
    /// `--status` raw text.
    pub status:           Option<String>,
    /// `--title`.
    pub title:            Option<String>,
    /// `--assignee` (empty string clears).
    pub assignee:         Option<String>,
    /// `--description` (replaces wholesale).
    pub description:      Option<String>,
    /// `--type` raw text.
    pub kind:             Option<String>,
    /// `--priority` raw text.
    pub priority:         Option<String>,
    /// `--add-label` raw comma list.
    pub add_label:        Option<String>,
    /// `--remove-label` raw comma list.
    pub remove_label:     Option<String>,
    /// `--set-labels` raw comma list.
    pub set_labels:       Option<String>,
    /// `--extensions` raw JSON object text.
    pub extensions:       Option<String>,
    /// `--clear-extensions`.
    pub clear_extensions: bool,
    /// JSON envelope output.
    pub json:             bool,
}

/// `update` — mutates one record's fields, bumping `updatedAt`.
pub fn update(ctx: &CommandContext, input: &UpdateInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<SeedRecord, CommandError> {
        let message_of = |text: String| CommandError::new("update", text, json);
        let Some(id) = input.id.clone() else {
            return Err(message_of("usage: sd update <id>".to_owned()));
        };
        let mut store = ctx.open_store().map_err(&message_of)?;
        if store.issue(&id).is_none() {
            return Err(message_of(format!("Issue not found: {id}")));
        }
        let now = timeutil::now_iso();
        {
            let record = store.issue_mut(&id).expect("existence checked above");
            if let Some(status) = input.status.as_deref() {
                let status = Status::parse(status).ok_or_else(|| {
                    message_of(format!(
                        "--status must be open|in_progress|closed, got `{status}`"
                    ))
                })?;
                record.set_status(status);
            }
            if let Some(title) = input.title.as_deref() {
                record.set_title(title);
            }
            match input.assignee.as_deref() {
                Some("") => {
                    record.set_assignee(None);
                }
                Some(assignee) => {
                    record.set_assignee(Some(assignee));
                }
                None => {}
            }
            if let Some(description) = input.description.as_deref() {
                record.set_description(Some(description));
            }
            if let Some(kind) = input.kind.as_deref() {
                let kind = parse_seed_type(kind).map_err(&message_of)?;
                record.set_seed_type(kind);
            }
            if let Some(text) = input.priority.as_deref() {
                let number = parse_priority(text).map_err(&message_of)?;
                record
                    .set_priority(number)
                    .map_err(|error| message_of(error.to_string()))?;
            }
            if let Some(text) = input.add_label.as_deref() {
                let mut labels: Vec<String> = record
                    .labels()
                    .iter()
                    .map(|label| (*label).to_owned())
                    .collect();
                for label in comma_list(text) {
                    if !labels.contains(&label) {
                        labels.push(label);
                    }
                }
                record.set_labels(labels);
            }
            if let Some(text) = input.remove_label.as_deref() {
                let remove = comma_list(text);
                let labels: Vec<String> = record
                    .labels()
                    .into_iter()
                    .filter(|label| !remove.iter().any(|r| r == *label))
                    .map(str::to_owned)
                    .collect();
                if labels.is_empty() {
                    record.remove_field("labels");
                } else {
                    record.set_labels(labels);
                }
            }
            if let Some(text) = input.set_labels.as_deref() {
                let labels = comma_list(text);
                if labels.is_empty() {
                    record.remove_field("labels");
                } else {
                    record.set_labels(labels);
                }
            }
            if let Some(text) = input.extensions.as_deref() {
                let incoming: Value = serde_json::from_str(text)
                    .map_err(|_| message_of("--extensions must be a JSON object".to_owned()))?;
                let Value::Object(incoming) = incoming else {
                    return Err(message_of("--extensions must be a JSON object".to_owned()));
                };
                let mut merged = match record.field("extensions") {
                    Some(Value::Object(existing)) => existing.clone(),
                    _ => Map::new(),
                };
                for (key, value) in incoming {
                    merged.insert(key, value);
                }
                record.set_field("extensions", Value::Object(merged));
            }
            if input.clear_extensions {
                record.remove_field("extensions");
            }
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| message_of(error.to_string()))?;
        Ok(store.issue(&id).expect("record was just updated").clone())
    })();
    match result {
        Ok(record) => {
            let mut out = String::new();
            if json {
                let issue = issue_value(&record);
                push_line(&mut out, &envelope_pretty("update", &[("issue", issue)]));
            } else {
                push_line(&mut out, &format!("✓ Updated {}", record.id()));
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `close`.
#[derive(Clone, Debug, Default)]
pub struct CloseInput {
    /// The ids to close (empty reproduces the usage error).
    pub ids:    Vec<String>,
    /// `--reason` (written as `closeReason`).
    pub reason: Option<String>,
    /// JSON envelope output.
    pub json:   bool,
}

/// `close` — closes one or more records with `closedAt`/`closeReason`.
pub fn close(ctx: &CommandContext, input: &CloseInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<Vec<String>, CommandError> {
        if input.ids.is_empty() {
            return Err(CommandError::new(
                "close",
                "usage: sd close <id> [ids...]",
                json,
            ));
        }
        let mut store = ctx
            .open_store()
            .map_err(|message| CommandError::new("close", message, json))?;
        for id in &input.ids {
            if store.issue(id).is_none() {
                return Err(CommandError::new(
                    "close",
                    format!("Issue not found: {id}"),
                    json,
                ));
            }
        }
        let now = timeutil::now_iso();
        let reason = input.reason.as_deref();
        let mut closed = Vec::with_capacity(input.ids.len());
        for id in &input.ids {
            let record = store.issue_mut(id).expect("existence checked above");
            record.set_status(Status::Closed);
            record.set_field("closedAt", json!(now));
            if let Some(reason) = reason {
                record.set_field("closeReason", json!(reason));
            }
            record.set_field("updatedAt", json!(now));
            closed.push(id.clone());
        }
        store
            .save()
            .map_err(|error| CommandError::new("close", error.to_string(), json))?;
        Ok(closed)
    })();
    match result {
        Ok(closed) => {
            let mut out = String::new();
            if json {
                push_line(
                    &mut out,
                    &envelope_pretty("close", &[("closed", json!(closed))]),
                );
            } else {
                let reason = input.reason.as_deref();
                for id in &closed {
                    match reason {
                        Some(reason) => push_line(&mut out, &format!("✓ Closed {id} — {reason}")),
                        None => push_line(&mut out, &format!("✓ Closed {id}")),
                    }
                }
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}
