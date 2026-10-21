//! label command semantics (sd 0.5.15-compatible).

use serde_json::{Map, Value, json};

use super::{CommandContext, CommandError, CommandOutcome, WriteFormat, render_write};
use crate::{render, timeutil};

/// Typed input for `label add`.
#[derive(Clone, Debug, Default)]
pub struct LabelAddInput {
    /// The target id (`None` reproduces the usage error).
    pub id:     Option<String>,
    /// The labels to add (empty reproduces the usage error).
    pub labels: Vec<String>,
    /// `--format` raw text (seeds-1579).
    pub format: Option<String>,
    /// JSON envelope output.
    pub json:   bool,
}

/// `label add` — adds labels to one record.
pub fn label_add(ctx: &CommandContext, input: &LabelAddInput) -> CommandOutcome {
    let mode = WriteFormat::resolve(input.format.as_deref(), input.json);
    let json = mode == WriteFormat::Json;
    let result = (|| -> Result<(String, Vec<String>), CommandError> {
        let Some(id) = input.id.clone() else {
            return Err(CommandError::new(
                "label",
                "usage: sd label add <issue> <labels...>",
                json,
            ));
        };
        if input.labels.is_empty() {
            return Err(CommandError::new(
                "label",
                "usage: sd label add <issue> <labels...>",
                json,
            ));
        }
        let labels = input.labels.clone();
        let mut store = ctx
            .open_store_exclusive()
            .map_err(|message| CommandError::new("label", message, json))?;
        if store.issue(&id).is_none() {
            return Err(CommandError::new(
                "label",
                format!("Issue not found: {id}"),
                json,
            ));
        }
        let now = timeutil::now_iso();
        {
            let record = store.issue_mut(&id).expect("existence checked above");
            let mut existing: Vec<String> =
                record.labels().into_iter().map(str::to_owned).collect();
            for label in &labels {
                if !existing.contains(label) {
                    existing.push(label.clone());
                }
            }
            record.set_labels(existing);
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| CommandError::new("label", error.to_string(), json))?;
        Ok((id, labels))
    })();
    match result {
        Ok((id, labels)) => {
            let labels_value: Vec<Value> = labels.iter().map(|l| json!(l)).collect();
            let extra = [
                ("issueId", json!(&id)),
                ("labels", Value::Array(labels_value)),
            ];
            let ids = [id.clone()];
            let text = format!("✓ Added label(s) {} to {id}\n", labels.join(", "));
            CommandOutcome::ok(render_write(mode, "label add", &extra, &ids, &text))
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `label remove`.
#[derive(Clone, Debug, Default)]
pub struct LabelRemoveInput {
    /// The target id (`None` reproduces the usage error).
    pub id:     Option<String>,
    /// The labels to remove (empty reproduces the usage error).
    pub labels: Vec<String>,
    /// `--format` raw text (seeds-1579).
    pub format: Option<String>,
    /// JSON envelope output.
    pub json:   bool,
}

/// `label remove` — removes labels from one record.
pub fn label_remove(ctx: &CommandContext, input: &LabelRemoveInput) -> CommandOutcome {
    let mode = WriteFormat::resolve(input.format.as_deref(), input.json);
    let json = mode == WriteFormat::Json;
    let result = (|| -> Result<(String, Vec<String>), CommandError> {
        let Some(id) = input.id.clone() else {
            return Err(CommandError::new(
                "label",
                "usage: sd label remove <issue> <labels...>",
                json,
            ));
        };
        if input.labels.is_empty() {
            return Err(CommandError::new(
                "label",
                "usage: sd label remove <issue> <labels...>",
                json,
            ));
        }
        let labels = input.labels.clone();
        let mut store = ctx
            .open_store_exclusive()
            .map_err(|message| CommandError::new("label", message, json))?;
        if store.issue(&id).is_none() {
            return Err(CommandError::new(
                "label",
                format!("Issue not found: {id}"),
                json,
            ));
        }
        let now = timeutil::now_iso();
        {
            let record = store.issue_mut(&id).expect("existence checked above");
            let remaining: Vec<String> = record
                .labels()
                .into_iter()
                .filter(|label| !labels.iter().any(|remove| remove == *label))
                .map(str::to_owned)
                .collect();
            if remaining.is_empty() {
                record.remove_field("labels");
            } else {
                record.set_labels(remaining);
            }
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| CommandError::new("label", error.to_string(), json))?;
        Ok((id, labels))
    })();
    match result {
        Ok((id, labels)) => {
            let labels_value: Vec<Value> = labels.iter().map(|l| json!(l)).collect();
            let extra = [
                ("issueId", json!(&id)),
                ("labels", Value::Array(labels_value)),
            ];
            let ids = [id.clone()];
            let text = format!("✓ Removed label(s) from {id}\n");
            CommandOutcome::ok(render_write(mode, "label remove", &extra, &ids, &text))
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `label list`.
#[derive(Clone, Debug, Default)]
pub struct LabelListInput {
    /// The target id (`None` reproduces the usage error).
    pub id:     Option<String>,
    /// `--format` raw text (seeds-1579).
    pub format: Option<String>,
    /// JSON envelope output.
    pub json:   bool,
}

/// `label list` — one record's labels.
pub fn label_list(ctx: &CommandContext, input: &LabelListInput) -> CommandOutcome {
    let mode = WriteFormat::resolve(input.format.as_deref(), input.json);
    let json = mode == WriteFormat::Json;
    let result = (|| -> Result<crate::SeedRecord, CommandError> {
        let Some(id) = input.id.clone() else {
            return Err(CommandError::new(
                "label",
                "usage: sd label list <issue>",
                json,
            ));
        };
        let store = ctx
            .open_store()
            .map_err(|message| CommandError::new("label", message, json))?;
        store
            .issue(&id)
            .cloned()
            .ok_or_else(|| CommandError::new("label", format!("Issue not found: {id}"), json))
    })();
    match result {
        Ok(record) => {
            let labels: Vec<Value> = record
                .labels()
                .into_iter()
                .map(|label| json!(label))
                .collect();
            let extra = [
                ("issueId", json!(record.id().as_str())),
                ("labels", Value::Array(labels)),
            ];
            // `--format ids`: the record's labels, one per line
            // (seeds-1579).
            let ids: Vec<String> = record.labels().into_iter().map(str::to_owned).collect();
            let text = render::label_list_text(&record);
            CommandOutcome::ok(render_write(mode, "label list", &extra, &ids, &text))
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `label list-all`.
#[derive(Clone, Debug, Default)]
pub struct LabelListAllInput {
    /// `--format` raw text (seeds-1579).
    pub format: Option<String>,
    /// JSON envelope output.
    pub json:   bool,
}

/// `label list-all` — every label in the project with counts.
pub fn label_list_all(ctx: &CommandContext, input: &LabelListAllInput) -> CommandOutcome {
    let mode = WriteFormat::resolve(input.format.as_deref(), input.json);
    let json = mode == WriteFormat::Json;
    let result = (|| -> Result<Vec<(String, usize)>, CommandError> {
        let store = ctx
            .open_store()
            .map_err(|message| CommandError::new("label", message, json))?;
        let mut counts: Vec<(String, usize)> = Vec::new();
        for record in &store.issues {
            for label in record.labels() {
                if let Some(entry) = counts.iter_mut().find(|(existing, _)| existing == label) {
                    entry.1 += 1;
                } else {
                    counts.push((label.to_owned(), 1));
                }
            }
        }
        Ok(counts)
    })();
    match result {
        Ok(counts) => {
            let mut sorted: Vec<&(String, usize)> = counts.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            let labels: Vec<Value> = sorted.iter().map(|(label, _)| json!(label)).collect();
            // The counts object keeps encounter order (the
            // reference's insertion-ordered map).
            let mut counts_map = Map::new();
            for (label, count) in &counts {
                counts_map.insert(label.clone(), json!(count));
            }
            let extra = [
                ("labels", Value::Array(labels)),
                ("counts", Value::Object(counts_map)),
            ];
            // `--format ids`: every label in the project, one per line,
            // in first-appearance order — the order the text table
            // prints (seeds-1579).
            let ids: Vec<String> = counts.iter().map(|(label, _)| label.clone()).collect();
            let text = render::label_list_all_text(&counts);
            CommandOutcome::ok(render_write(mode, "label list-all", &extra, &ids, &text))
        }
        Err(error) => error.into_outcome(),
    }
}
