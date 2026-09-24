//! `seeds plan` — the decomposition layer over `plans.jsonl`
//! (sd-0.5.15 parity, seeds-de37): templates, prompt, submit, show,
//! validate, outcome, review, edit, create, adopt, reorder, release,
//! list.

#![allow(
    clippy::format_push_string,
    reason = "incremental push_str(&format!(…)) line building reads clearer here, matching render.rs"
)]
mod adopt;
mod backref;
mod edit;
mod list;
mod mulch;
mod prompt;
mod schema;
mod show;
mod submit;
mod templates;

use serde_json::{Value, json};
pub(crate) use templates::load_plan_templates;

use super::{CommandContext, CommandError, CommandOutcome, push_line};
use crate::{Fields, Store, timeutil};

impl Default for PlanSub {
    fn default() -> Self {
        Self::List {
            seed:     None,
            status:   None,
            outcome:  None,
            template: None,
        }
    }
}

/// One `seeds plan` invocation: the subcommand plus its options.
#[derive(Clone, Debug, Default)]
pub struct PlanInput {
    /// The subcommand and its arguments.
    pub sub:  PlanSub,
    /// JSON envelope output (`--json` anywhere in argv, sd's rule).
    pub json: bool,
}

/// The plan subcommands.
#[derive(Clone, Debug)]
pub enum PlanSub {
    /// `plan templates`
    Templates,
    /// `plan prompt <seed-id>`
    Prompt {
        /// The parent seed id.
        seed_id:  String,
        /// `--template` override.
        template: Option<String>,
        /// `--domain` override for prior-art enrichment.
        domain:   Option<String>,
    },
    /// `plan submit <seed-id>`
    Submit {
        /// The parent seed id.
        seed_id:         String,
        /// `--plan <file>` (`-` for stdin).
        plan_file:       String,
        /// stdin contents when `--plan -` was given (read by the binary).
        plan_stdin:      Option<String>,
        /// `--overwrite`.
        overwrite:       bool,
        /// `--record-decision`.
        record_decision: bool,
        /// `--domain` for the decision record.
        domain:          Option<String>,
        /// `--name` override.
        name:            Option<String>,
    },
    /// `plan show <id>` (plan id or seed id).
    Show {
        /// The plan id or seed id.
        id: String,
    },
    /// `plan validate <id>`
    Validate {
        /// The plan id or seed id.
        id: String,
    },
    /// `plan outcome <id>`
    Outcome {
        /// The plan id or seed id.
        id:     String,
        /// `--result` (success|partial|failure).
        result: String,
        /// `--note`.
        note:   Option<String>,
    },
    /// `plan review <id>`
    Review {
        /// The plan id or seed id.
        id: String,
        /// `--by`.
        by: String,
    },
    /// `plan edit <id>`
    Edit {
        /// The plan id or seed id.
        id:       String,
        /// `--name`.
        name:     Option<String>,
        /// `--section <name> <text>`.
        section:  Option<(String, String)>,
        /// `--step <i>` (1-based raw text).
        step:     Option<String>,
        /// `--title` (with `--step`).
        title:    Option<String>,
        /// `--priority` (with `--step`).
        priority: Option<String>,
        /// `--type` (with `--step`).
        kind:     Option<String>,
    },
    /// `plan create <seed-id>`
    Create {
        /// The parent seed id.
        seed_id:  String,
        /// `--name`.
        name:     Option<String>,
        /// `--template`.
        template: Option<String>,
    },
    /// `plan adopt <plan-id> <seed-ids...>`
    Adopt {
        /// The plan id (or parent seed id).
        plan_id:  String,
        /// The seeds to adopt.
        seed_ids: Vec<String>,
        /// `--step <i>`.
        step:     Option<String>,
        /// `--at <i>`.
        at:       Option<String>,
        /// `--before <seed>`.
        before:   Option<String>,
        /// `--after <seed>`.
        after:    Option<String>,
    },
    /// `plan reorder <plan-id> <seed-ids...>`
    Reorder {
        /// The plan id.
        plan_id:  String,
        /// The full children order.
        seed_ids: Vec<String>,
    },
    /// `plan release <plan-id> <seed-ids...>`
    Release {
        /// The plan id.
        plan_id:  String,
        /// The seeds to release.
        seed_ids: Vec<String>,
    },
    /// `plan list`
    List {
        /// `--seed` filter.
        seed:     Option<String>,
        /// `--status` filter.
        status:   Option<String>,
        /// `--outcome` filter.
        outcome:  Option<String>,
        /// `--template` filter.
        template: Option<String>,
    },
}

/// Runs one `seeds plan` invocation.
pub fn plan(ctx: &CommandContext, input: &PlanInput) -> CommandOutcome {
    let json = input.json;
    let fail = move |message: String| CommandError::new("plan", message, json);
    let mut store = match ctx.open_store() {
        Ok(store) => store,
        Err(message) => return fail(message).into_outcome(),
    };
    match &input.sub {
        PlanSub::Templates => list::templates(&store, json),
        PlanSub::Prompt {
            seed_id,
            template,
            domain,
        } => prompt::run(
            &store,
            seed_id,
            template.as_deref(),
            domain.as_deref(),
            json,
        ),
        PlanSub::Submit {
            seed_id,
            plan_file,
            plan_stdin,
            overwrite,
            record_decision,
            domain,
            name,
        } => submit::run(
            &mut store,
            seed_id,
            plan_file,
            plan_stdin.as_deref(),
            *overwrite,
            *record_decision,
            domain.as_deref(),
            name.as_deref(),
            json,
        ),
        PlanSub::Show { id } => show::run_show(&store, id, json),
        PlanSub::Validate { id } => show::run_validate(&store, id, json),
        PlanSub::Outcome { id, result, note } => {
            edit::run_outcome(&mut store, id, result, note.as_deref(), json)
        }
        PlanSub::Review { id, by } => edit::run_review(&mut store, id, by, json),
        PlanSub::Edit {
            id,
            name,
            section,
            step,
            title,
            priority,
            kind,
        } => edit::run_edit(
            &mut store,
            id,
            name.as_deref(),
            section.as_ref().map(|(a, b)| (a.as_str(), b.as_str())),
            step.as_deref(),
            title.as_deref(),
            priority.as_deref(),
            kind.as_deref(),
            json,
        ),
        PlanSub::Create {
            seed_id,
            name,
            template,
        } => adopt::run_create(
            &mut store,
            seed_id,
            name.as_deref(),
            template.as_deref(),
            json,
        ),
        PlanSub::Adopt {
            plan_id,
            seed_ids,
            step,
            at,
            before,
            after,
        } => adopt::run_adopt(
            &mut store,
            plan_id,
            seed_ids,
            step.as_deref(),
            at.as_deref(),
            before.as_deref(),
            after.as_deref(),
            json,
        ),
        PlanSub::Reorder { plan_id, seed_ids } => {
            adopt::run_reorder(&mut store, plan_id, seed_ids, json)
        }
        PlanSub::Release { plan_id, seed_ids } => {
            adopt::run_release(&mut store, plan_id, seed_ids, json)
        }
        PlanSub::List {
            seed,
            status,
            outcome,
            template,
        } => list::run_list(
            &store,
            seed.as_deref(),
            status.as_deref(),
            outcome.as_deref(),
            template.as_deref(),
            json,
        ),
    }
}

// -- shared helpers ---------------------------------------------------------

/// A thrown-style plan error (rendered as sd's top-level handler:
/// failure envelope on stdout with `--json`, `Error: …` on stderr
/// otherwise).
pub(crate) type PlanError = String;

pub(crate) fn plan_error(message: impl Into<String>, json: bool) -> CommandOutcome {
    CommandError::new("plan", message, json).into_outcome()
}

/// Resolves a plan-id-or-seed-id argument to a plan id (sd's
/// `resolvePlanIdArg`).
pub(crate) fn resolve_plan_id(store: &Store, arg: &str) -> Result<String, PlanError> {
    if arg.starts_with("pl-") {
        return Ok(arg.to_owned());
    }
    let Some(seed) = store.issue(arg) else {
        return Err(format!(
            "Plan not found: {arg}. Run 'sd plan list' to see available plans."
        ));
    };
    let Some(plan_id) = seed.field("plan_id").and_then(Value::as_str) else {
        return Err(format!(
            "Seed {arg} has no plan. Submit one with 'sd plan submit {arg} --plan <file>'."
        ));
    };
    Ok(plan_id.to_owned())
}

/// Finds a plan record by id.
pub(crate) fn plan_by_id(store: &Store, plan_id: &str) -> Result<usize, PlanError> {
    store
        .plans
        .iter()
        .position(|record| record.id().as_str() == plan_id)
        .ok_or_else(|| {
            format!("Plan not found: {plan_id}. Run 'sd plan list' to see available plans.")
        })
}

/// A fresh plan id (`pl-<hex4>`), unique within the plan store.
pub(crate) fn fresh_plan_id(store: &Store) -> String {
    loop {
        let id = format!("pl-{}", timeutil::random_hex4());
        if !store.plans.iter().any(|record| record.id().as_str() == id) {
            return id;
        }
    }
}

/// A fresh seed id for a spawned plan child, unique against every
/// issue id plus the ids already assigned in this spawn pass.
pub(crate) fn fresh_child_id(store: &Store, taken: &mut Vec<String>) -> String {
    let existing: Vec<&str> = store
        .issues
        .iter()
        .map(|record| record.id().as_str())
        .collect();
    loop {
        let id = format!("{}-{}", store.config.project, timeutil::random_hex4());
        if !existing.contains(&id.as_str()) && !taken.contains(&id) {
            taken.push(id.clone());
            return id;
        }
    }
}

/// sd's `normalizeLabels`: lowercase, trim, drop empties.
pub(crate) fn normalize_labels(raw: &[String]) -> Vec<String> {
    raw.iter()
        .map(|label| label.trim().to_lowercase())
        .filter(|label| !label.is_empty())
        .collect()
}

/// Additively merges step labels into an adopted seed's labels; `None`
/// when there is nothing to add.
pub(crate) fn merge_adopted_labels(
    existing: Option<&Value>,
    step_labels: Option<&Vec<String>>,
) -> Option<Vec<String>> {
    let step_labels = step_labels?;
    if step_labels.is_empty() {
        return None;
    }
    let normalized = normalize_labels(step_labels);
    if normalized.is_empty() {
        return None;
    }
    let current: Vec<String> = existing.and_then(Value::as_array).map_or_default(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect()
    });
    let mut merged = current.clone();
    for label in normalized {
        if !merged.contains(&label) {
            merged.push(label);
        }
    }
    if merged.len() == current.len() && merged == current {
        return None;
    }
    Some(merged)
}

/// sd's plan-name normalization: empty/whitespace means "absent".
pub(crate) fn normalize_plan_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(String::from)
}

/// Builds the fields of a freshly spawned plan child, in sd's exact
/// serialization order.
pub(crate) fn child_fields(
    id: &str,
    title: &str,
    kind: &str,
    priority: u64,
    step_index: usize,
    description: &str,
    now: &str,
) -> Fields {
    let mut fields = Fields::new();
    fields.insert("id".to_owned(), json!(id));
    fields.insert("title".to_owned(), json!(title));
    fields.insert("status".to_owned(), json!("open"));
    fields.insert("type".to_owned(), json!(kind));
    fields.insert("priority".to_owned(), json!(priority));
    fields.insert("plan_step_index".to_owned(), json!(step_index));
    fields.insert("description".to_owned(), json!(description));
    fields.insert("createdAt".to_owned(), json!(now));
    fields.insert("updatedAt".to_owned(), json!(now));
    fields
}

/// Builds the fields of a fresh plan row, in sd's order (`name` last).
pub(crate) fn plan_fields(
    id: &str,
    seed_id: &str,
    template: &str,
    status: &str,
    revision: u64,
    sections: Value,
    children: &[String],
    now: &str,
) -> Fields {
    let mut fields = Fields::new();
    fields.insert("id".to_owned(), json!(id));
    fields.insert("seed".to_owned(), json!(seed_id));
    fields.insert("template".to_owned(), json!(template));
    fields.insert("status".to_owned(), json!(status));
    fields.insert("revision".to_owned(), json!(revision));
    fields.insert("sections".to_owned(), sections);
    fields.insert("children".to_owned(), json!(children));
    fields.insert("createdAt".to_owned(), json!(now));
    fields.insert("updatedAt".to_owned(), json!(now));
    fields
}

/// Appends `id` to a record's string-array field unless present;
/// returns whether the field changed.
pub(crate) fn append_unique_field(record_fields: &mut Fields, key: &str, id: &str) -> bool {
    let current: Vec<String> = record_fields
        .get(key)
        .and_then(Value::as_array)
        .map_or_default(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        });
    if current.iter().any(|entry| entry == id) {
        return false;
    }
    let mut next = current;
    next.push(id.to_owned());
    record_fields.insert(key.to_owned(), json!(next));
    true
}

/// Removes `id` from a record's string-array field, dropping the field
/// when the result is empty.
pub(crate) fn remove_list_value(record_fields: &mut Fields, key: &str, id: &str) {
    let Some(Value::Array(items)) = record_fields.get(key) else {
        return;
    };
    let current: Vec<String> = items
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect();
    let next: Vec<String> = current
        .iter()
        .filter(|entry| *entry != id)
        .cloned()
        .collect();
    if next.len() == current.len() {
        return;
    }
    if next.is_empty() {
        record_fields.remove(key);
    } else {
        record_fields.insert(key.to_owned(), json!(next));
    }
}

/// The duplicate entries in an id list.
pub(crate) fn find_duplicates(ids: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut dupes = Vec::new();
    for id in ids {
        if !seen.insert(id) && !dupes.contains(id) {
            dupes.push(id.clone());
        }
    }
    dupes
}

/// sd's `--step` parsing: 1-based on the CLI, 0-based internally.
pub(crate) fn parse_step_flag(raw: &str) -> Result<usize, PlanError> {
    let n: usize = raw
        .trim()
        .parse()
        .map_err(|_| format!("--step must be a positive integer (got: {raw})."))?;
    if n < 1 || raw.trim() != n.to_string() {
        return Err(format!("--step must be a positive integer (got: {raw})."));
    }
    Ok(n - 1)
}

/// sd's success line: `✓ <message>`.
pub(crate) fn success_line(out: &mut String, message: &str) {
    push_line(out, &format!("✓ {message}"));
}
