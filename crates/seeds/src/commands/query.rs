//! list / ready / search command semantics (sd 0.5.15-compatible) —
//! the shared filter/sort/limit pipeline.

use std::collections::HashSet;

use serde_json::{Value, json};

use super::{
    CommandContext, CommandError, CommandOutcome, comma_list, envelope_pretty, issue_value,
    parse_priority, parse_seed_type, push_line, store_has_unresolved_blockers,
};
use crate::{SeedRecord, SeedType, Status, Store, render, timeutil};

/// The shared filter/sort/limit surface of list, ready, and search.
struct Filters {
    status:       Option<Status>,
    kind:         Option<SeedType>,
    assignee:     Option<String>,
    all:          bool,
    label:        Vec<String>,
    label_any:    Vec<String>,
    unlabeled:    bool,
    priority:     Option<HashSet<u8>>,
    priority_max: Option<u8>,
    limit:        usize,
    sort:         String,
}

impl Filters {
    fn from_input(input: &QueryInput) -> Result<Self, String> {
        let status = input
            .status
            .as_deref()
            .map(|text| {
                Status::parse(text).ok_or_else(|| {
                    format!("--status must be open|in_progress|closed, got `{text}`")
                })
            })
            .transpose()?;
        let kind = input.kind.as_deref().map(parse_seed_type).transpose()?;
        let priority = input
            .priority
            .as_deref()
            .map(|text| {
                text.split(',')
                    .map(parse_priority)
                    .collect::<Result<HashSet<u8>, _>>()
            })
            .transpose()?;
        let priority_max = input
            .priority_max
            .as_deref()
            .map(|text| {
                text.parse::<u8>()
                    .map_err(|_| format!("--priority-max must be 0-4, got `{text}`"))
            })
            .transpose()?
            .filter(|number| *number <= 4);
        let limit = input
            .limit
            .as_deref()
            .map(|text| {
                text.parse::<usize>()
                    .map_err(|_| format!("--limit must be a positive number, got `{text}`"))
            })
            .transpose()?
            .unwrap_or(50);
        Ok(Self {
            status,
            kind,
            assignee: input.assignee.clone(),
            all: input.all,
            label: input.label.as_deref().map_or_default(comma_list),
            label_any: input.label_any.as_deref().map_or_default(comma_list),
            unlabeled: input.unlabeled,
            priority,
            priority_max,
            limit,
            sort: input.sort.clone().unwrap_or_else(|| "priority".to_owned()),
        })
    }

    /// Applies every filter, then sorts, then limits.
    fn apply<'a>(&self, store: &'a Store, scope_ready: bool) -> Vec<&'a SeedRecord> {
        let mut selected: Vec<&SeedRecord> = store
            .issues
            .iter()
            .filter(|record| {
                let status = record.status();
                if scope_ready {
                    // ready: open issues with every blocker resolved.
                    if status != Some(Status::Open) {
                        return false;
                    }
                    if store_has_unresolved_blockers(store, record) {
                        return false;
                    }
                } else if let Some(want) = self.status {
                    if status != Some(want) {
                        return false;
                    }
                } else if !self.all && status == Some(Status::Closed) {
                    return false;
                }
                if let Some(kind) = self.kind
                    && record.seed_type() != Some(kind)
                {
                    return false;
                }
                if let Some(assignee) = &self.assignee
                    && record.assignee() != Some(assignee.as_str())
                {
                    return false;
                }
                let labels = record.labels();
                if self.unlabeled && !labels.is_empty() {
                    return false;
                }
                if !self.label.is_empty()
                    && !self
                        .label
                        .iter()
                        .all(|want| labels.contains(&want.as_str()))
                {
                    return false;
                }
                if !self.label_any.is_empty()
                    && !self
                        .label_any
                        .iter()
                        .any(|want| labels.contains(&want.as_str()))
                {
                    return false;
                }
                let priority = record.priority().map_or(4_u8, crate::Priority::get);
                if let Some(levels) = &self.priority
                    && !levels.contains(&priority)
                {
                    return false;
                }
                if let Some(max) = self.priority_max
                    && priority > max
                {
                    return false;
                }
                true
            })
            .collect();
        let created = |record: &SeedRecord| record.created_at().unwrap_or_default().to_owned();
        match self.sort.as_str() {
            "created" => {
                selected.sort_by_key(|record| std::cmp::Reverse(created(record)));
            }
            "updated" => selected.sort_by(|a, b| {
                b.updated_at()
                    .unwrap_or_default()
                    .cmp(a.updated_at().unwrap_or_default())
            }),
            "id" => selected.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str())),
            // Default: priority ascending, newest first within a level.
            _ => selected.sort_by(|a, b| {
                let pa = a.priority().map_or(4_u8, crate::Priority::get);
                let pb = b.priority().map_or(4_u8, crate::Priority::get);
                pa.cmp(&pb).then(created(b).cmp(&created(a)))
            }),
        }
        selected.truncate(self.limit);
        selected
    }
}

/// `--respect-schedule`: exclude queued or future-scheduled issues (the
/// `extensions` field sd writes for scheduled work).
fn scheduled_out(record: &SeedRecord) -> bool {
    let Some(Value::Object(extensions)) = record.field("extensions") else {
        return false;
    };
    if extensions.get("queued") == Some(&Value::Bool(true)) {
        return true;
    }
    match extensions.get("scheduledFor") {
        Some(Value::String(when)) => when.as_str() > timeutil::now_iso().as_str(),
        _ => false,
    }
}

/// Which command runs the shared query pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryCommand {
    /// `list`.
    List,
    /// `ready`.
    Ready,
    /// `search`.
    Search,
}

impl QueryCommand {
    fn as_str(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Ready => "ready",
            Self::Search => "search",
        }
    }
}

/// Typed input for `list`, `ready`, and `search` — their shared flag
/// surface, values still raw strings (validated library-side).
#[derive(Clone, Debug, Default)]
pub struct QueryInput {
    /// Which query command is running.
    pub command:          Option<QueryCommand>,
    /// The search needle (`search` positional).
    pub query:            Option<String>,
    /// `--status` raw text.
    pub status:           Option<String>,
    /// `--type` raw text.
    pub kind:             Option<String>,
    /// `--assignee`.
    pub assignee:         Option<String>,
    /// `--all`.
    pub all:              bool,
    /// `--label` raw comma list.
    pub label:            Option<String>,
    /// `--label-any` raw comma list.
    pub label_any:        Option<String>,
    /// `--unlabeled`.
    pub unlabeled:        bool,
    /// `--priority` raw comma list.
    pub priority:         Option<String>,
    /// `--priority-max` raw text.
    pub priority_max:     Option<String>,
    /// `--limit` raw text.
    pub limit:            Option<String>,
    /// `--sort`.
    pub sort:             Option<String>,
    /// `--format` raw text.
    pub format:           Option<String>,
    /// JSON envelope output.
    pub json:             bool,
    /// `--respect-schedule` (ready only).
    pub respect_schedule: bool,
}

/// `list` — the shared query pipeline over all (default: non-closed)
/// issues.
pub fn list(ctx: &CommandContext, input: &QueryInput) -> CommandOutcome {
    query(ctx, input, QueryCommand::List)
}

/// `ready` — open, unblocked issues.
pub fn ready(ctx: &CommandContext, input: &QueryInput) -> CommandOutcome {
    query(ctx, input, QueryCommand::Ready)
}

/// `search` — substring match over titles and descriptions.
pub fn search(ctx: &CommandContext, input: &QueryInput) -> CommandOutcome {
    query(ctx, input, QueryCommand::Search)
}

/// The shared query pipeline behind list/ready/search.
pub fn query(ctx: &CommandContext, input: &QueryInput, command: QueryCommand) -> CommandOutcome {
    let command = command.as_str();
    let json = input.json;
    let result = (|| -> Result<(Vec<SeedRecord>, Option<String>), CommandError> {
        let filters = Filters::from_input(input)
            .map_err(|message| CommandError::new(command, message, json))?;
        let store = ctx
            .open_store()
            .map_err(|message| CommandError::new(command, message, json))?;
        let query = input.query.clone();
        if command == "search" && query.is_none() {
            // sd reports a missing search argument on stderr without a
            // JSON envelope, even in --format json mode (differential
            // battery, seeds-25b5).
            return Err(CommandError::new(
                command,
                "missing required argument 'query'",
                false,
            ));
        }
        let mut selected = filters.apply(&store, command == "ready");
        if command == "search" {
            let needle = query.as_deref().unwrap_or_default().to_lowercase();
            selected.retain(|record| {
                let title = record.title().to_lowercase();
                let description = record.description().unwrap_or_default().to_lowercase();
                title.contains(&needle) || description.contains(&needle)
            });
        }
        if input.respect_schedule {
            selected.retain(|record| !scheduled_out(record));
        }
        let owned: Vec<SeedRecord> = selected.into_iter().cloned().collect();
        Ok((owned, query))
    })();
    match result {
        Ok((records, query)) => {
            let mode = render::render_mode(input.format.as_deref());
            let mut out = String::new();
            if json {
                let issues: Vec<Value> = records.iter().map(issue_value).collect();
                // The reference's query envelopes end with the result
                // count (pinned by the differential battery, seeds-25b5).
                let mut extra: Vec<(&str, Value)> =
                    vec![("issues", json!(issues)), ("count", json!(issues.len()))];
                if let Some(query) = &query {
                    extra.insert(0, ("query", json!(query)));
                }
                push_line(&mut out, &envelope_pretty(command, &extra));
            } else {
                let store = ctx.open_store();
                let unresolved = |dep: &str| -> bool {
                    store.as_ref().map_or(true, |store| {
                        store
                            .issue(dep)
                            .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
                    })
                };
                let text = render::list_text(&records, mode, &unresolved, command);
                out.push_str(&text);
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}
