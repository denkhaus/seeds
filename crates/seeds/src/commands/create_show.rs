//! create and show command semantics (sd 0.5.15-compatible).

use serde_json::{Map, Value, json};

use super::{
    CommandContext, CommandError, CommandOutcome, comma_list, envelope_pretty, fresh_seed_id,
    issue_value, parse_priority, parse_seed_type, push_line,
};
use crate::{Fields, SeedRecord, render, timeutil};

/// Typed input for `create`.
#[derive(Clone, Debug, Default)]
pub struct CreateInput {
    /// The title (required; `None` reproduces the reference's stderr
    /// usage error).
    pub title:       Option<String>,
    /// `--type` raw text (validated library-side).
    pub kind:        Option<String>,
    /// `--priority` raw text (validated library-side).
    pub priority:    Option<String>,
    /// `--description`/`--desc`/`--body` (aliases folded by the caller).
    pub description: Option<String>,
    /// `--labels` raw comma list.
    pub labels:      Option<String>,
    /// `--assignee`.
    pub assignee:    Option<String>,
    /// JSON envelope output.
    pub json:        bool,
}

/// `create` — appends a record to the store.
pub fn create(ctx: &CommandContext, input: &CreateInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<String, CommandError> {
        let Some(title) = input.title.clone() else {
            // The reference reports missing required options on stderr,
            // without a JSON envelope.
            return Err(CommandError::stderr_only(
                "error: required option '--title <text>' not specified",
            ));
        };
        let message_of = |text: &str| CommandError::new("create", text, json);
        let seed_type = parse_seed_type(input.kind.as_deref().unwrap_or("task"))
            .map_err(|message| message_of(&message))?;
        let priority = input
            .priority
            .as_deref()
            .map_or(Ok(2_u8), parse_priority)
            .map_err(|message| message_of(&message))?;

        let mut store = ctx.open_store().map_err(|message| message_of(&message))?;
        let id = fresh_seed_id(&store);
        let now = timeutil::now_iso();
        let mut fields = Fields::new();
        fields.insert("id".to_owned(), json!(id));
        fields.insert("title".to_owned(), json!(title));
        fields.insert("status".to_owned(), json!("open"));
        fields.insert("type".to_owned(), json!(seed_type.as_str()));
        fields.insert("priority".to_owned(), json!(priority));
        fields.insert("createdAt".to_owned(), json!(now));
        fields.insert("updatedAt".to_owned(), json!(now));
        if let Some(description) = &input.description {
            fields.insert("description".to_owned(), json!(description));
        }
        if let Some(labels) = input
            .labels
            .as_deref()
            .map(comma_list)
            .filter(|labels| !labels.is_empty())
        {
            fields.insert("labels".to_owned(), json!(labels));
        }
        if let Some(assignee) = &input.assignee {
            fields.insert("assignee".to_owned(), json!(assignee));
        }
        let record =
            SeedRecord::try_from_fields(fields).map_err(|error| message_of(&error.to_string()))?;
        store.issues.push(record);
        store
            .save()
            .map_err(|error| message_of(&error.to_string()))?;
        Ok(id)
    })();
    match result {
        Ok(id) => {
            let mut out = String::new();
            if json {
                push_line(&mut out, &envelope_pretty("create", &[("id", json!(id))]));
            } else {
                push_line(&mut out, &format!("✓ Created {id}"));
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `show`.
#[derive(Clone, Debug, Default)]
pub struct ShowInput {
    /// The ids to show.
    pub ids:       Vec<String>,
    /// `--format` raw text.
    pub format:    Option<String>,
    /// JSON envelope output (`--json` or `--format json`).
    pub json:      bool,
    /// Whether `--json` (not just `--format json`) was passed — the
    /// reference's single-missing-id quirk distinguishes them.
    pub json_flag: bool,
}

/// `show` — one or more records, JSON envelope or rendered text.
pub fn show(ctx: &CommandContext, input: &ShowInput) -> CommandOutcome {
    if input.ids.is_empty() {
        // sd: a usage error on stderr, no envelope — in every mode.
        return CommandOutcome {
            success: false,
            stdout:  String::new(),
            stderr:  "error: missing required argument 'id'\n".to_owned(),
        };
    }
    let json = input.json;
    // The reference's show quirk (DEVIATIONS-pinned, seeds-25b5): for a
    // SINGLE missing id, `--json` answers a failure envelope while
    // `--format json` reports on stderr — the alias is not equivalent
    // there. Every show error path exits 1.
    let json_flag = input.json_flag;
    let store = match ctx.open_store() {
        Ok(store) => store,
        Err(message) => {
            if json {
                return CommandError::new("show", message, true).into_outcome();
            }
            return CommandOutcome {
                success: false,
                stdout:  String::new(),
                stderr:  format!("Error: {message}\n"),
            };
        }
    };
    let mut found = Vec::with_capacity(input.ids.len());
    let mut missing = Vec::new();
    for id in &input.ids {
        match store.issue(id) {
            Some(record) => found.push(record.clone()),
            None => missing.push(id.clone()),
        }
    }
    let not_found = |id: &str| format!("Issue not found: {id}");
    if json {
        let issues: Vec<Value> = found.iter().map(issue_value).collect();
        let results: Vec<Value> = found
            .iter()
            .map(|record| json!({ "issue": issue_value(record) }))
            .collect();
        if missing.is_empty() {
            let mut out = String::new();
            if found.len() == 1 {
                push_line(
                    &mut out,
                    &envelope_pretty("show", &[("issue", issues[0].clone())]),
                );
            } else {
                push_line(
                    &mut out,
                    &envelope_pretty("show", &[
                        ("issues", json!(issues)),
                        ("results", json!(results)),
                    ]),
                );
            }
            return CommandOutcome::ok(out);
        }
        if found.is_empty() && missing.len() == 1 {
            // Single missing id: envelope only for the `--json` form.
            if json_flag {
                return CommandError::new("show", not_found(&missing[0]), true).into_outcome();
            }
            return CommandOutcome {
                success: false,
                stdout:  String::new(),
                stderr:  format!("Error: {}\n", not_found(&missing[0])),
            };
        }
        // Multi-show with missing ids: a partial failure envelope with
        // per-id errors; every error path exits 1.
        let errors: Vec<Value> = missing
            .iter()
            .map(|id| json!({ "id": id, "error": not_found(id) }))
            .collect();
        let mut failure = Map::new();
        failure.insert("success".to_owned(), Value::Bool(false));
        failure.insert("command".to_owned(), json!("show"));
        failure.insert("issues".to_owned(), json!(issues));
        failure.insert("results".to_owned(), json!(results));
        failure.insert("errors".to_owned(), json!(errors));
        let mut stdout = serde_json::to_string_pretty(&Value::Object(failure))
            .expect("envelope always serializes");
        stdout.push('\n');
        return CommandOutcome {
            success: false,
            stdout,
            stderr: String::new(),
        };
    }
    if missing.is_empty() {
        let text = render::show_text(&found, render::render_mode(input.format.as_deref()));
        return CommandOutcome::ok(text);
    }
    if found.is_empty() && missing.len() == 1 {
        return CommandOutcome {
            success: false,
            stdout:  String::new(),
            stderr:  format!("Error: {}\n", not_found(&missing[0])),
        };
    }
    // Partial multi-show: the found records, then a per-missing ✗ line.
    let out = render::show_text(&found, render::render_mode(input.format.as_deref()));
    let mut err = String::new();
    for id in &missing {
        push_line(&mut err, &format!("✗ {id}: {}", not_found(id)));
    }
    CommandOutcome {
        success: false,
        stdout:  out,
        stderr:  err,
    }
}
