//! `tpl` command group semantics (sd 0.5.15-compatible, seeds-fb5f):
//! molecules — reusable step templates poured into dependency-chained
//! issue convoys.
//!
//! Semantics pinned against the reference source
//! (tests/fixtures/sd-reference/.../commands/tpl.ts) and live probes:
//! template ids are `tpl-<hex4>` (random, unique); steps carry
//! {title, type, priority}; `pour` instantiates one open issue per
//! step (same timestamp for the whole pour), replaces `{prefix}` in
//! step titles, stamps `convoy: <tpl-id>`, and wires step[i]
//! blocks/step[i+1] blockedBy pairs; `status` aggregates every issue
//! of the convoy across pours. Success envelopes name the SUBCOMMAND
//! ("tpl create", ...); error envelopes name the GROUP ("tpl") - the
//! reference's top-level error handler, mirrored here.

use serde_json::{Value, json};

use super::{CommandContext, CommandError, CommandOutcome, envelope_pretty, push_line};
use crate::{Fields, SeedRecord, SeedType, Status, TemplateRecord, timeutil};

/// The reference's priority labels for one-line listings.
const PRIORITY_NAMES: [&str; 5] = ["Critical", "High", "Medium", "Low", "Backlog"];

/// One `seeds tpl` invocation's parsed subcommand.
#[derive(Clone, Debug)]
pub enum TplSub {
    /// `tpl create --name <text>` - a fresh empty molecule.
    Create {
        /// The template name.
        name: String,
    },
    /// `tpl step add <id> --title <text> [--type] [--priority]`.
    StepAdd {
        /// The template id.
        id:    String,
        /// The step title.
        title: String,
        /// Step type (task|bug|feature|epic), default task.
        kind:  Option<String>,
        /// Step priority 0-4, default 2.
        pri:   Option<String>,
    },
    /// `tpl list` - every template with its step count.
    List,
    /// `tpl show <id>` - one template with its steps.
    Show {
        /// The template id.
        id: String,
    },
    /// `tpl pour <id> --prefix <text>` - instantiate the convoy.
    Pour {
        /// The template id.
        id:     String,
        /// Replaces `{prefix}` placeholders in step titles.
        prefix: String,
    },
    /// `tpl status <id>` - convoy progress across all pours.
    Status {
        /// The template id.
        id: String,
    },
}

/// `seeds tpl`: the molecules group.
#[derive(Clone, Debug)]
pub struct TplInput {
    /// The parsed subcommand.
    pub sub:  TplSub,
    /// Whether success output is the JSON envelope.
    pub json: bool,
}

/// Runs one `tpl` subcommand against the context's store.
#[must_use = "the outcome only takes effect when the caller prints it"]
pub fn tpl(ctx: &CommandContext, input: &TplInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<String, CommandError> {
        let mut store = ctx
            .open_store()
            .map_err(|message| CommandError::new("tpl", &message, json))?;
        match &input.sub {
            TplSub::Create { name } => {
                let id = fresh_tpl_id(&store);
                let mut fields = Fields::new();
                fields.insert("id".to_owned(), json!(id));
                fields.insert("name".to_owned(), json!(name));
                fields.insert("steps".to_owned(), json!([]));
                let record = TemplateRecord::try_from_fields(fields)
                    .map_err(|error| CommandError::new("tpl", error.to_string(), json))?;
                store.templates.push(record);
                store
                    .save()
                    .map_err(|error| CommandError::new("tpl", error.to_string(), json))?;
                Ok(if json {
                    envelope_pretty("tpl create", &[("id", json!(id))])
                } else {
                    format!("✓ Created template {id}: {name}\n")
                })
            }
            TplSub::StepAdd {
                id,
                title,
                kind,
                pri,
            } => {
                let type_text = kind.as_deref().unwrap_or("task");
                let seed_type = SeedType::parse(type_text).ok_or_else(|| {
                    CommandError::new(
                        "tpl",
                        format!("Invalid --type value: {type_text}. Valid: task|bug|feature|epic"),
                        json,
                    )
                })?;
                let priority = pri
                    .as_deref()
                    .map_or(Ok(2_u8), super::parse_priority)
                    .map_err(|message| CommandError::new("tpl", &message, json))?;
                let index = store
                    .templates
                    .iter()
                    .position(|record| record.id().as_str() == id)
                    .ok_or_else(|| {
                        CommandError::new("tpl", format!("Template not found: {id}"), json)
                    })?;
                let record = &mut store.templates[index];
                let mut steps = record.steps().cloned().unwrap_or_default();
                steps.push(json!({
                    "title":    title,
                    "type":     seed_type.as_str(),
                    "priority": priority,
                }));
                let step_count = steps.len();
                record.fields_mut().insert("steps".to_owned(), json!(steps));
                store
                    .save()
                    .map_err(|error| CommandError::new("tpl", error.to_string(), json))?;
                Ok(if json {
                    envelope_pretty("tpl step add", &[
                        ("id", json!(id)),
                        ("stepCount", json!(step_count)),
                    ])
                } else {
                    format!("✓ Added step {step_count} to {id}: \"{title}\"\n")
                })
            }
            TplSub::List => {
                if json {
                    let templates: Vec<Value> =
                        store.templates.iter().map(template_value).collect();
                    let count = templates.len();
                    return Ok(envelope_pretty("tpl list", &[
                        ("templates", json!(templates)),
                        ("count", json!(count)),
                    ]));
                }
                if store.templates.is_empty() {
                    return Ok("No templates.\n".to_owned());
                }
                let mut text = String::new();
                for record in &store.templates {
                    let steps = record.steps().map_or(0, Vec::len);
                    push_line(
                        &mut text,
                        &format!("{}  {}  ({} steps)", record.id(), name_of(record), steps),
                    );
                }
                Ok(text)
            }
            TplSub::Show { id } => {
                let record = store
                    .templates
                    .iter()
                    .find(|record| record.id().as_str() == id)
                    .ok_or_else(|| {
                        CommandError::new("tpl", format!("Template not found: {id}"), json)
                    })?;
                if json {
                    return Ok(envelope_pretty("tpl show", &[(
                        "template",
                        template_value(record),
                    )]));
                }
                let mut text = String::new();
                push_line(&mut text, &format!("{}  {}", record.id(), name_of(record)));
                let steps = record.steps().cloned().unwrap_or_default();
                push_line(&mut text, &format!("Steps ({}):", steps.len()));
                for (index, step) in steps.iter().enumerate() {
                    let kind = step["type"].as_str().unwrap_or("task");
                    let pri = step["priority"].as_u64().unwrap_or(2);
                    push_line(
                        &mut text,
                        &format!(
                            "  {}. {}  [{} P{}]",
                            index + 1,
                            step["title"].as_str().unwrap_or(""),
                            kind,
                            pri
                        ),
                    );
                }
                Ok(text)
            }
            TplSub::Pour { id, prefix } => {
                let record = store
                    .templates
                    .iter()
                    .find(|record| record.id().as_str() == id)
                    .ok_or_else(|| {
                        CommandError::new("tpl", format!("Template not found: {id}"), json)
                    })?;
                let steps = record.steps().cloned().unwrap_or_default();
                if steps.is_empty() {
                    return Err(CommandError::new(
                        "tpl",
                        format!("Template {id} has no steps"),
                        json,
                    ));
                }
                let now = timeutil::now_iso();
                let mut created: Vec<SeedRecord> = Vec::new();
                for step in &steps {
                    let title = step["title"]
                        .as_str()
                        .unwrap_or("")
                        .replace("{prefix}", prefix);
                    let issue_id = fresh_issue_id(&store, &created);
                    let mut fields = Fields::new();
                    fields.insert("id".to_owned(), json!(issue_id));
                    fields.insert("title".to_owned(), json!(title));
                    fields.insert("status".to_owned(), json!("open"));
                    fields.insert(
                        "type".to_owned(),
                        json!(step["type"].as_str().unwrap_or("task")),
                    );
                    fields.insert(
                        "priority".to_owned(),
                        json!(step["priority"].as_u64().unwrap_or(2)),
                    );
                    fields.insert("createdAt".to_owned(), json!(now));
                    fields.insert("updatedAt".to_owned(), json!(now));
                    fields.insert("convoy".to_owned(), json!(id));
                    let record = SeedRecord::try_from_fields(fields)
                        .map_err(|error| CommandError::new("tpl", error.to_string(), json))?;
                    created.push(record);
                }
                // Chain: step[i] blocks step[i+1]; step[i+1] is blocked
                // by step[i] - both arrays, the reference's shape. The
                // keys land after `convoy` (ordered-map insertion),
                // matching the reference's field order.
                for index in 1..created.len() {
                    let prev_id = created[index - 1].id().as_str().to_owned();
                    let curr_id = created[index].id().as_str().to_owned();
                    created[index - 1]
                        .fields_mut()
                        .insert("blocks".to_owned(), json!([curr_id]));
                    created[index]
                        .fields_mut()
                        .insert("blockedBy".to_owned(), json!([prev_id]));
                }
                let ids: Vec<String> = created.iter().map(|r| r.id().as_str().to_owned()).collect();
                store.issues.append(&mut created);
                store
                    .save()
                    .map_err(|error| CommandError::new("tpl", error.to_string(), json))?;
                Ok(if json {
                    envelope_pretty("tpl pour", &[("ids", json!(ids))])
                } else {
                    let mut text = String::new();
                    push_line(
                        &mut text,
                        &format!("✓ Poured template {id} — created {} issues", ids.len()),
                    );
                    for issue_id in &ids {
                        push_line(&mut text, &format!("  {issue_id}"));
                    }
                    text
                })
            }
            TplSub::Status { id } => {
                let convoy: Vec<&SeedRecord> = store
                    .issues
                    .iter()
                    .filter(|record| {
                        record
                            .fields()
                            .get("convoy")
                            .and_then(Value::as_str)
                            .is_some_and(|convoy| convoy == id)
                    })
                    .collect();
                if convoy.is_empty() {
                    return Ok(if json {
                        envelope_pretty("tpl status", &[
                            ("templateId", json!(id)),
                            ("total", json!(0)),
                            ("issues", json!([])),
                        ])
                    } else {
                        format!("No issues found for convoy {id}\n")
                    });
                }
                let closed = |record: &SeedRecord| record.status() == Some(Status::Closed);
                let completed = convoy.iter().filter(|record| closed(record)).count();
                let in_progress = convoy
                    .iter()
                    .filter(|record| record.status() == Some(Status::InProgress))
                    .count();
                let blocked = convoy
                    .iter()
                    .filter(|record| {
                        !closed(record)
                            && record
                                .blocked_by()
                                .iter()
                                .any(|dep| store.issue(dep).is_none_or(|blocker| !closed(blocker)))
                    })
                    .count();
                let issue_ids: Vec<&str> =
                    convoy.iter().map(|record| record.id().as_str()).collect();
                if json {
                    return Ok(envelope_pretty("tpl status", &[(
                        "status",
                        json!({
                            "templateId": id,
                            "total":      convoy.len(),
                            "completed":  completed,
                            "inProgress": in_progress,
                            "blocked":    blocked,
                            "issues":     issue_ids,
                        }),
                    )]));
                }
                let mut text = String::new();
                push_line(&mut text, &format!("Convoy: {id}"));
                push_line(&mut text, &format!("  Total:       {}", convoy.len()));
                push_line(&mut text, &format!("  Completed:   {completed}"));
                push_line(&mut text, &format!("  In progress: {in_progress}"));
                push_line(&mut text, &format!("  Blocked:     {blocked}"));
                push_line(&mut text, "  Issues:");
                for record in &convoy {
                    let is_blocked = !closed(record)
                        && record
                            .blocked_by()
                            .iter()
                            .any(|dep| store.issue(dep).is_none_or(|blocker| !closed(blocker)));
                    push_line(&mut text, &format!("    {}", one_line(record, is_blocked)));
                }
                Ok(text)
            }
        }
    })();
    match result {
        Ok(stdout) => CommandOutcome {
            success: true,
            stdout,
            stderr: String::new(),
        },
        Err(error) => error.into_outcome(),
    }
}

/// The sd one-line issue shape used by convoy listings (icon, id,
/// title, priority/type, blocked marker).
fn one_line(record: &SeedRecord, blocked: bool) -> String {
    let icon = match record.status() {
        Some(Status::Closed) => "x",
        Some(Status::InProgress) => ">",
        _ if blocked => "!",
        _ => "-",
    };
    let priority = PRIORITY_NAMES[usize::from(record.priority().map_or(4, crate::Priority::get))];
    let kind = record.seed_type().map_or("task", SeedType::as_str);
    let blocked_text = if blocked { " [blocked]" } else { "" };
    format!(
        "{icon} {} · {}   [{} · {}]{blocked_text}",
        record.id(),
        record.title(),
        priority,
        kind
    )
}

fn name_of(record: &TemplateRecord) -> &str {
    record
        .fields()
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
}

/// The sd-canonical JSON projection of one template.
fn template_value(record: &TemplateRecord) -> Value {
    let fields = record.fields();
    let mut out = Fields::new();
    for key in ["id", "name", "steps"] {
        if let Some(value) = fields.get(key) {
            out.insert(key.to_owned(), value.clone());
        }
    }
    Value::Object(out)
}

/// A fresh issue id for a pour: `<project>-<hex4>`, unique against
/// both the store and the same pour's in-flight batch.
fn fresh_issue_id(store: &crate::Store, created: &[SeedRecord]) -> String {
    let prefix = format!("{}-", store.config.project);
    loop {
        let id = format!("{prefix}{}", timeutil::random_hex4());
        let taken = store
            .issues
            .iter()
            .chain(created.iter())
            .any(|record| record.id().as_str() == id);
        if !taken {
            return id;
        }
    }
}

/// A fresh template id: `tpl-<hex4>`, unique within the store.
fn fresh_tpl_id(store: &crate::Store) -> String {
    loop {
        let id = format!("tpl-{}", timeutil::random_hex4());
        if !store
            .templates
            .iter()
            .any(|record| record.id().as_str() == id)
        {
            return id;
        }
    }
}
