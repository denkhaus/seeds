//! `plan templates` and `plan list` (sd 0.5.15).

#![allow(
    clippy::format_push_string,
    reason = "incremental push_str(&format!(…)) line building reads clearer here, matching render.rs"
)]
use serde_json::{Value, json};

use super::{load_plan_templates, plan_error};
use crate::Store;
use crate::commands::CommandOutcome;

pub(super) fn templates(store: &Store, json: bool) -> CommandOutcome {
    let templates = match load_plan_templates(store) {
        Ok(templates) => templates,
        Err(message) => return plan_error(message, json),
    };
    let mut names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    names.sort_unstable();
    let entries: Vec<Value> = names
        .iter()
        .map(|name| {
            let template = templates.iter().find(|t| t.name == *name);
            json!({
                "name": name,
                "description": template
                    .and_then(|t| t.description.as_deref())
                    .unwrap_or(""),
            })
        })
        .collect();
    if json {
        let envelope = json!({
            "success": true,
            "command": "plan templates",
            "templates": entries,
            "count": entries.len(),
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
    let mut out = String::from("Available templates:\n");
    for entry in &entries {
        let name = entry["name"].as_str().unwrap_or_default();
        let description = entry["description"].as_str().unwrap_or_default();
        if description.is_empty() {
            out.push_str(&format!("  {name}\n"));
        } else {
            out.push_str(&format!("  {name}  {description}\n"));
        }
    }
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}

const VALID_PLAN_STATUSES: &[&str] = &["draft", "approved", "active", "done"];
const VALID_PLAN_OUTCOMES: &[&str] = &["success", "partial", "failure"];

pub(super) fn run_list(
    store: &Store,
    seed: Option<&str>,
    status: Option<&str>,
    outcome: Option<&str>,
    template: Option<&str>,
    json: bool,
) -> CommandOutcome {
    if let Some(status) = &status
        && !VALID_PLAN_STATUSES.contains(status)
    {
        return plan_error(
            format!(
                "Invalid --status value: {status}. Valid: {}",
                VALID_PLAN_STATUSES.join("|")
            ),
            json,
        );
    }
    if let Some(outcome) = &outcome
        && !VALID_PLAN_OUTCOMES.contains(outcome)
    {
        return plan_error(
            format!(
                "Invalid --outcome value: {outcome}. Valid: {}",
                VALID_PLAN_OUTCOMES.join("|")
            ),
            json,
        );
    }
    let mut filtered: Vec<&crate::model::PlanRecord> = store
        .plans
        .iter()
        .filter(|record| {
            seed.as_ref().is_none_or(|seed| record.seed() == Some(seed))
                && status.as_ref().is_none_or(|status| {
                    record.field("status").and_then(Value::as_str) == Some(status)
                })
                && outcome.as_ref().is_none_or(|outcome| {
                    record.field("outcome").and_then(Value::as_str) == Some(outcome)
                })
                && template.as_ref().is_none_or(|template| {
                    record.field("template").and_then(Value::as_str) == Some(template)
                })
        })
        .collect();
    filtered.sort_by(|a, b| {
        // sd sorts newest-first on createdAt (stable on ties).
        b.created_at()
            .unwrap_or("")
            .cmp(a.created_at().unwrap_or(""))
    });
    if json {
        let plans: Vec<Value> = filtered
            .iter()
            .map(|record| Value::Object(record.fields().clone()))
            .collect();
        let envelope = json!({
            "success": true,
            "command": "plan list",
            "plans": plans,
            "count": filtered.len(),
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
    if filtered.is_empty() {
        return CommandOutcome {
            success: true,
            stdout:  "No plans match.\n".to_owned(),
            stderr:  String::new(),
        };
    }
    let mut out = String::new();
    for record in filtered {
        let name_part = match record.name() {
            Some(name) => format!("  {}", truncate_pad(name, 40)),
            None => format!("  {}", truncate_pad("(unnamed)", 40)),
        };
        let outcome_part = record
            .field("outcome")
            .and_then(Value::as_str)
            .map_or_default(|outcome| format!(" ({outcome})"));
        out.push_str(&format!(
            "{}  {}  rev {}{}  {}  seed={}  children={}{}  {}\n",
            record.id(),
            record
                .field("status")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            record.revision().unwrap_or(1),
            name_part,
            record
                .field("template")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            record.seed().unwrap_or_default(),
            record.children().len(),
            outcome_part,
            record.created_at().unwrap_or_default(),
        ));
    }
    CommandOutcome {
        success: true,
        stdout:  out,
        stderr:  String::new(),
    }
}

fn truncate_pad(value: &str, width: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= width {
        let text: String = chars.into_iter().collect();
        format!("{text:<width$}")
    } else {
        let cut: String = chars[..width - 1].iter().collect();
        format!("{cut}…")
    }
}
