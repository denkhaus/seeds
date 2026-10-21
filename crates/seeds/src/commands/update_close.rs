//! update and close command semantics (sd 0.5.15-compatible).

use serde_json::{Map, Value, json};

use super::{
    CommandContext, CommandError, CommandOutcome, WriteFormat, comma_list, issue_value,
    parse_priority, parse_seed_type, push_line, render_write,
};
use crate::{SeedRecord, Status, timeutil};

/// Typed input for `update`. Option values are raw strings validated
/// library-side; `None` leaves the field untouched.
#[derive(Clone, Debug, Default)]
pub struct UpdateInput {
    /// The target id (`None` reproduces the usage error).
    pub id:                     Option<String>,
    /// `--status` raw text.
    pub status:                 Option<String>,
    /// `--title`.
    pub title:                  Option<String>,
    /// `--assignee` (empty string clears).
    pub assignee:               Option<String>,
    /// `--set-description` (replaces wholesale).
    pub description:            Option<String>,
    /// `--append-description` (appended after one blank line).
    pub append_description:     Option<String>,
    /// `--replace-in-description` (raw JSON `{"find", "replace"}` payload).
    pub replace_in_description: Option<String>,
    /// `--type` raw text.
    pub kind:                   Option<String>,
    /// `--priority` raw text.
    pub priority:               Option<String>,
    /// `--add-label` raw comma list.
    pub add_label:              Option<String>,
    /// `--remove-label` raw comma list.
    pub remove_label:           Option<String>,
    /// `--set-labels` raw comma list.
    pub set_labels:             Option<String>,
    /// `--extensions` raw JSON object text.
    pub extensions:             Option<String>,
    /// `--clear-extensions`.
    pub clear_extensions:       bool,
    /// `--format` raw text (seeds-1579).
    pub format:                 Option<String>,
    /// JSON envelope output.
    pub json:                   bool,
}

/// The change-shape note for a description write: which verb ran and
/// the character sizes it produced.
#[derive(Clone, Copy, Debug)]
enum DescriptionShape {
    /// `--set-description`: whole-field replace, old → new chars.
    Set { old: usize, new: usize },
    /// `--append-description`: added → total chars.
    Appended { added: usize, total: usize },
    /// `--replace-in-description`: one span replaced, old → new chars.
    Replaced { old: usize, new: usize },
}

impl DescriptionShape {
    fn to_line(self) -> String {
        match self {
            Self::Set { old, new } => format!("description: set {old} -> {new} chars"),
            Self::Appended { added, total } => {
                format!("description: appended +{added} (total {total})")
            }
            Self::Replaced { old, new } => {
                format!("description: replaced span ({old} -> {new} chars)")
            }
        }
    }
}

/// The parsed `--replace-in-description` payload: an exact substring
/// to replace once (`find`, non-empty) and its replacement (possibly
/// empty — span deletion).
struct ReplaceIn {
    find:    String,
    replace: String,
}

impl ReplaceIn {
    /// Parses the raw JSON payload text.
    ///
    /// # Errors
    ///
    /// Returns the usage error for non-object payloads, missing or
    /// non-string fields, and an empty `find`.
    fn parse(text: &str) -> Result<Self, String> {
        const USAGE: &str = concat!(
            "--replace-in-description must be a JSON object ",
            r#"{"find": string, "replace": string}"#
        );
        let payload: Value = serde_json::from_str(text).map_err(|_| USAGE.to_owned())?;
        let (Some(find), Some(replace)) = (
            payload.get("find").and_then(Value::as_str),
            payload.get("replace").and_then(Value::as_str),
        ) else {
            return Err(USAGE.to_owned());
        };
        if find.is_empty() {
            return Err("--replace-in-description: find must not be empty".to_owned());
        }
        Ok(Self {
            find:    find.to_owned(),
            replace: replace.to_owned(),
        })
    }
}

/// The canonical triage roles that describe PENDING triage work
/// (docs/agents/triage-labels.md minus `wontfix`, which records a
/// decision already taken rather than an open question).
const PENDING_TRIAGE_LABELS: &[&str] = &[
    "needs-triage",
    "needs-info",
    "ready-for-agent",
    "ready-for-human",
];

/// Clears the pending triage-role labels off a record that just went
/// `closed` (seeds-8666): closing answers the triage question, so the
/// roles would only preempt it and inflate every label aggregate with
/// history. A record left without labels loses the field entirely —
/// the shape `update --remove-label` writes. Deliberate deviation from
/// sd (which keeps every label); README DEVIATIONS carries the entry.
fn strip_triage_labels(record: &mut SeedRecord) {
    if record.field("labels").is_none() {
        return;
    }
    let kept: Vec<String> = record
        .labels()
        .iter()
        .filter(|label| !PENDING_TRIAGE_LABELS.contains(label))
        .map(|label| (*label).to_owned())
        .collect();
    if kept.is_empty() {
        record.remove_field("labels");
    } else {
        record.set_labels(kept);
    }
}

/// Appends `text` to `existing`, separated by exactly one blank line;
/// a missing or blank `existing` yields `text` alone.
fn join_appended(existing: Option<&str>, text: &str) -> String {
    match existing.map(str::trim_end).filter(|head| !head.is_empty()) {
        Some(head) => format!("{head}\n\n{text}"),
        None => text.to_owned(),
    }
}

/// `update` — mutates one record's fields, bumping `updatedAt`.
pub fn update(ctx: &CommandContext, input: &UpdateInput) -> CommandOutcome {
    let mode = WriteFormat::resolve(input.format.as_deref(), input.json);
    let json = mode == WriteFormat::Json;
    let result = (|| -> Result<(SeedRecord, Option<DescriptionShape>), CommandError> {
        let message_of = |text: String| CommandError::new("update", text, json);
        let Some(id) = input.id.clone() else {
            return Err(message_of("usage: sd update <id>".to_owned()));
        };
        let verb_count = [
            input.description.is_some(),
            input.append_description.is_some(),
            input.replace_in_description.is_some(),
        ]
        .into_iter()
        .filter(|set| *set)
        .count();
        if verb_count > 1 {
            return Err(message_of(
                "--set-description, --append-description, and --replace-in-description are mutually exclusive"
                    .to_owned(),
            ));
        }
        if input
            .append_description
            .as_deref()
            .is_some_and(|text| text.trim().is_empty())
        {
            return Err(message_of("--append-description requires text".to_owned()));
        }
        let mut store = ctx.open_store_exclusive().map_err(&message_of)?;
        if store.issue(&id).is_none() {
            return Err(message_of(format!("Issue not found: {id}")));
        }
        let now = timeutil::now_iso();
        let mut description_shape = None;
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
                let shape = DescriptionShape::Set {
                    old: record.description().map_or(0, |text| text.chars().count()),
                    new: description.chars().count(),
                };
                record.set_description(Some(description));
                description_shape = Some(shape);
            }
            if let Some(text) = input.replace_in_description.as_deref() {
                let replace = ReplaceIn::parse(text).map_err(message_of)?;
                let description = record.description().unwrap_or("").to_owned();
                let occurrences = description.match_indices(&replace.find).count();
                match occurrences {
                    0 => {
                        let snippet: String = replace.find.chars().take(57).collect();
                        return Err(message_of(format!(
                            "--replace-in-description: text not found: \"{snippet}\""
                        )));
                    }
                    1 => {}
                    many => {
                        return Err(message_of(format!(
                            "--replace-in-description: {many} occurrences - make the match unique"
                        )));
                    }
                }
                let old = description.chars().count();
                let mut replaced = description;
                let at = replaced.find(&replace.find).expect("one occurrence above");
                replaced.replace_range(at..at + replace.find.len(), &replace.replace);
                let shape = DescriptionShape::Replaced {
                    old,
                    new: replaced.chars().count(),
                };
                record.set_description(Some(&replaced));
                description_shape = Some(shape);
            }
            if let Some(text) = input.append_description.as_deref() {
                let appended = text.trim();
                let joined = join_appended(record.description(), appended);
                let shape = DescriptionShape::Appended {
                    added: appended.chars().count(),
                    total: joined.chars().count(),
                };
                record.set_description(Some(&joined));
                description_shape = Some(shape);
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
            // Close hygiene runs LAST, after every label verb: a
            // record that ends this write `closed` never keeps a
            // pending triage role — not via `--status closed`, not via
            // `--add-label`, and not from labels it still carried
            // (seeds-8666).
            if record.status() == Some(Status::Closed) {
                strip_triage_labels(record);
            }
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| message_of(error.to_string()))?;
        let record = store.issue(&id).expect("record was just updated").clone();
        Ok((record, description_shape))
    })();
    match result {
        Ok((record, description_shape)) => {
            let extra = [("issue", issue_value(&record))];
            let ids = [record.id().to_string()];
            let mut text = String::new();
            push_line(&mut text, &format!("✓ Updated {}", record.id()));
            if let Some(shape) = description_shape {
                push_line(&mut text, &shape.to_line());
            }
            CommandOutcome::ok(render_write(mode, "update", &extra, &ids, &text))
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `close`.
#[derive(Clone, Debug, Default)]
pub struct CloseInput {
    /// The ids to close (empty reproduces the usage error).
    pub ids:     Vec<String>,
    /// `--reason` (written as `closeReason`).
    pub reason:  Option<String>,
    /// `--comment`: appended to each closed record's description after
    /// one blank line — the closing remark in one call instead of
    /// `update --append-description` + `close` (seeds-1579).
    pub comment: Option<String>,
    /// `--format` raw text (seeds-1579).
    pub format:  Option<String>,
    /// JSON envelope output.
    pub json:    bool,
}

/// `close` — closes one or more records with `closedAt`/`closeReason`.
pub fn close(ctx: &CommandContext, input: &CloseInput) -> CommandOutcome {
    let mode = WriteFormat::resolve(input.format.as_deref(), input.json);
    let json = mode == WriteFormat::Json;
    let result = (|| -> Result<(Vec<String>, usize), CommandError> {
        if input.ids.is_empty() {
            return Err(CommandError::new(
                "close",
                "usage: sd close <id> [ids...]",
                json,
            ));
        }
        if input
            .comment
            .as_deref()
            .is_some_and(|text| text.trim().is_empty())
        {
            return Err(CommandError::new("close", "--comment requires text", json));
        }
        let mut store = ctx
            .open_store_exclusive()
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
        let comment = input.comment.as_deref().map(str::trim);
        let mut closed = Vec::with_capacity(input.ids.len());
        let mut commented = 0_usize;
        for id in &input.ids {
            let record = store.issue_mut(id).expect("existence checked above");
            record.set_status(Status::Closed);
            record.set_field("closedAt", json!(now));
            if let Some(reason) = reason {
                record.set_field("closeReason", json!(reason));
            }
            // The closing remark lands with the close itself
            // (seeds-1579) — one call instead of append + close.
            if let Some(text) = comment {
                let joined = join_appended(record.description(), text);
                record.set_description(Some(&joined));
                commented += 1;
            }
            // Close hygiene (seeds-8666): pending triage roles go with
            // the closure — see `strip_triage_labels`.
            strip_triage_labels(record);
            record.set_field("updatedAt", json!(now));
            closed.push(id.clone());
        }
        store
            .save()
            .map_err(|error| CommandError::new("close", error.to_string(), json))?;
        Ok((closed, commented))
    })();
    match result {
        Ok((closed, commented)) => {
            let extra = [("closed", json!(&closed))];
            let mut text = String::new();
            let reason = input.reason.as_deref();
            for id in &closed {
                match reason {
                    Some(reason) => push_line(&mut text, &format!("✓ Closed {id} — {reason}")),
                    None => push_line(&mut text, &format!("✓ Closed {id}")),
                }
            }
            if commented > 0 {
                push_line(
                    &mut text,
                    &format!("+ comment appended to {commented} issue(s)"),
                );
            }
            CommandOutcome::ok(render_write(mode, "close", &extra, &closed, &text))
        }
        Err(error) => error.into_outcome(),
    }
}
