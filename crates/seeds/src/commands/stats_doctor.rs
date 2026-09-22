//! stats and doctor command semantics (sd 0.5.15-compatible).

use serde_json::{Map, Value, json};

use super::{CommandContext, CommandError, CommandOutcome, envelope_pretty};
use crate::{Status, doctor, render};

/// Typed input for `stats`.
#[derive(Clone, Debug, Default)]
pub struct StatsInput {
    /// JSON envelope output.
    pub json: bool,
}

/// `stats` — project statistics by status, type, priority, and label.
pub fn stats(ctx: &CommandContext, input: &StatsInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<render::Stats, CommandError> {
        let store = ctx
            .open_store()
            .map_err(|message| CommandError::new("stats", message, json))?;
        let unresolved = |dep: &str| -> bool {
            store
                .issue(dep)
                .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
        };
        Ok(render::Stats::collect(&store.issues, &unresolved))
    })();
    match result {
        Ok(stats) => {
            let mut out = String::new();
            if json {
                let mut by_type = Map::new();
                for (kind, count) in &stats.by_type {
                    by_type.insert(kind.clone(), json!(count));
                }
                let mut by_priority = Map::new();
                for (level, count) in &stats.by_priority {
                    by_priority.insert(level.to_string(), json!(count));
                }
                let mut by_label = Map::new();
                for (label, count) in &stats.by_label {
                    by_label.insert(label.clone(), json!(count));
                }
                let mut stats_map = Map::new();
                stats_map.insert("total".to_owned(), json!(stats.total));
                stats_map.insert("open".to_owned(), json!(stats.open));
                stats_map.insert("inProgress".to_owned(), json!(stats.in_progress));
                stats_map.insert("closed".to_owned(), json!(stats.closed));
                stats_map.insert("blocked".to_owned(), json!(stats.blocked));
                stats_map.insert("byType".to_owned(), Value::Object(by_type));
                stats_map.insert("byPriority".to_owned(), Value::Object(by_priority));
                stats_map.insert("byLabel".to_owned(), Value::Object(by_label));
                let text = envelope_pretty("stats", &[("stats", Value::Object(stats_map))]);
                out.push_str(&text);
                out.push('\n');
            } else {
                out.push_str(&stats.text());
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// Typed input for `doctor`.
#[derive(Clone, Debug, Default)]
pub struct DoctorInput {
    /// `--fix` (apply repairs).
    pub fix:           bool,
    /// `--repair-report` (append the repair plan in text mode).
    pub repair_report: bool,
    /// JSON envelope output.
    pub json:          bool,
}

/// `doctor` — store integrity checks; unsuccessful when any check
/// fails.
pub fn doctor(ctx: &CommandContext, input: &DoctorInput) -> CommandOutcome {
    let json = input.json;
    let fix = input.fix;
    let repair_report = input.repair_report;
    let result = (|| -> Result<doctor::Report, CommandError> {
        let root = ctx
            .resolve()
            .map_err(|message| CommandError::new("doctor", message, json))?;
        doctor::run(root, fix).map_err(|message| CommandError::new("doctor", message, json))
    })();
    match result {
        Ok(report) => {
            let mut out = String::new();
            if json {
                let mut envelope = Map::new();
                envelope.insert("success".to_owned(), json!(!report.has_failures()));
                envelope.insert("command".to_owned(), json!("doctor"));
                let Value::Object(report_map) = report.json_value(repair_report) else {
                    unreachable!("report json is an object");
                };
                for (key, value) in report_map {
                    envelope.insert(key, value);
                }
                let text = serde_json::to_string_pretty(&Value::Object(envelope))
                    .expect("envelope always serializes");
                out.push_str(&text);
                out.push('\n');
            } else {
                out.push_str(&report.text());
                if repair_report {
                    out.push_str(&report.repair_report_text());
                }
            }
            CommandOutcome {
                success: !report.has_failures(),
                stdout:  out,
                stderr:  String::new(),
            }
        }
        Err(error) => error.into_outcome(),
    }
}
