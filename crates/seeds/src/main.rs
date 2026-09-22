//! The `seeds` binary: sd 0.5.15-compatible CLI over the format core.
//!
//! Parity surface (README compat contract, ADR-0023): create, show,
//! list, ready, update, close, dep add/remove/list, blocked, block,
//! unblock, label add/remove/list/list-all, stats, doctor, prime,
//! search — plus `sync` with sd-parity behavior and the
//! README-documented deliberate improvements (per-file preview,
//! push-gate safety, shortstat commit body), `dedupe` and doctor's
//! `--repair-report` as native additions.
//! Flag names, JSON envelope shapes (`{success, command, …}`), filter and
//! limit semantics, and error behavior (JSON `success:false` plus a
//! non-zero exit, exactly as the pinned reference behaves) mirror
//! `@os-eco/seeds-cli` 0.5.15.

#![allow(
    clippy::print_stdout,
    reason = "printing to stdout is this binary's purpose"
)]
#![allow(clippy::print_stderr, reason = "CLI diagnostics belong on stderr")]

use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, ExitCode};

use seeds::{Fields, SeedRecord, SeedType, Status, Store};
use serde_json::{Map, Value, json};

mod args;
mod dedupe;
mod doctor;
mod helptext;
mod render;
mod timeutil;

use args::{OptSpec, Parsed};
use render::RenderMode;

/// A command failure: rendered as JSON (`success:false`) or as
/// `Error: …` on stderr, always with a non-zero exit — the reference's
/// observed behavior (its loop scripts depend on the exit code).
struct CommandError {
    command: String,
    message: String,
    json:    bool,
}

impl CommandError {
    fn new(command: &str, message: impl Into<String>, json: bool) -> Self {
        Self {
            command: command.to_owned(),
            message: message.into(),
            json,
        }
    }

    fn report(&self) -> ExitCode {
        if self.json {
            let mut envelope = Map::new();
            envelope.insert("success".to_owned(), Value::Bool(false));
            envelope.insert("command".to_owned(), json!(self.command));
            envelope.insert("error".to_owned(), json!(self.message));
            println!(
                "{}",
                serde_json::to_string_pretty(&Value::Object(envelope))
                    .expect("envelope always serializes")
            );
        } else {
            eprintln!("Error: {}", self.message);
        }
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    dispatch(&argv)
}

fn dispatch(argv: &[String]) -> ExitCode {
    let Some(command) = argv.first() else {
        println!("{}", helptext::GLOBAL);
        return ExitCode::SUCCESS;
    };
    let rest = &argv[1..];
    match command.as_str() {
        "-h" | "--help" => {
            println!("{}", helptext::GLOBAL);
            ExitCode::SUCCESS
        }
        "-v" | "--version" => {
            println!("seeds v0.5.15 — Git-native issue tracking");
            ExitCode::SUCCESS
        }
        "create" => run_create(rest),
        "show" => run_show(rest),
        "list" => run_list(rest),
        "ready" => run_ready(rest),
        "update" => run_update(rest),
        "close" => run_close(rest),
        "dep" => run_dep(rest),
        "blocked" => run_blocked(rest),
        "block" => run_block(rest),
        "unblock" => run_unblock(rest),
        "label" => run_label(rest),
        "stats" => run_stats(rest),
        "doctor" => run_doctor(rest),
        "prime" => run_prime(rest),
        "search" => run_search(rest),
        "dedupe" => run_dedupe(rest),
        "sync" => run_sync(rest),
        other => {
            // Help honesty (seeds-25b5): planned sd-parity commands
            // answer with a clear "not implemented yet", not a generic
            // unknown-command error.
            if helptext::PLANNED.contains(&other) {
                eprintln!(
                    "error: command '{other}' is not implemented yet (planned \
                     sd-0.5.15 parity) — run 'seeds --help' for the \
                     implemented commands"
                );
            } else {
                eprintln!("error: unknown command '{other}'");
            }
            ExitCode::FAILURE
        }
    }
}

/// Parses `args` against `spec`, honoring `-h/--help` by printing the
/// command's reference help text and exiting successfully.
fn parsed_or_help(args: &[String], spec: &[OptSpec], help: &str) -> Option<Parsed> {
    let mut filtered = Vec::with_capacity(args.len());
    let mut wants_help = false;
    for arg in args {
        if arg == "-h" || arg == "--help" {
            wants_help = true;
        } else {
            filtered.push(arg.clone());
        }
    }
    if wants_help {
        println!("{help}");
        std::process::exit(0);
    }
    match args::parse(spec, &filtered) {
        Ok(parsed) => Some(parsed),
        Err(message) => {
            eprintln!("error: {message}");
            None
        }
    }
}

/// Finds the `.seeds/` directory at or above the current directory.
fn find_seeds_dir() -> Result<std::path::PathBuf, String> {
    let mut dir = std::env::current_dir().map_err(|error| error.to_string())?;
    loop {
        let candidate = dir.join(".seeds");
        if candidate.join("config.yaml").is_file() {
            return Ok(candidate);
        }
        if !dir.pop() {
            return Err("No .seeds directory found (run from the project root)".to_owned());
        }
    }
}

/// Finds the `.seeds/` store at or above the current directory.
fn open_store() -> Result<Store, String> {
    let root = find_seeds_dir()?;
    Store::open(&root).map_err(|error| error.to_string())
}

fn json_mode(parsed: &Parsed) -> bool {
    parsed.flags.contains("json") || parsed.options.get("format").is_some_and(|f| f == "json")
}

fn render_mode(parsed: &Parsed) -> RenderMode {
    match parsed.options.get("format").map(String::as_str) {
        Some("compact") => RenderMode::Compact,
        Some("plain") => RenderMode::Plain,
        Some("ids") => RenderMode::Ids,
        Some("json") => RenderMode::Json,
        // The default plus `markdown` share the reference's rich view.
        _ => RenderMode::Markdown,
    }
}

/// A fresh seed id: `<project>-<hex4>`, unique within the store.
fn fresh_seed_id(store: &Store) -> String {
    let prefix = format!("{}-", store.config.project);
    loop {
        let id = format!("{prefix}{}", timeutil::random_hex4());
        if store.issue(&id).is_none() {
            return id;
        }
    }
}

/// The sd-canonical JSON projection of one record: known fields only,
/// in the reference's key order, absent fields omitted.
fn issue_value(record: &SeedRecord) -> Value {
    let fields = record.fields();
    let mut out = Map::new();
    for key in [
        "id",
        "title",
        "status",
        "type",
        "priority",
        "createdAt",
        "updatedAt",
        "closedAt",
        "description",
        "labels",
        "blockedBy",
        "blocks",
        "assignee",
        "closeReason",
    ] {
        if let Some(value) = fields.get(key) {
            out.insert(key.to_owned(), value.clone());
        }
    }
    Value::Object(out)
}

/// Builds the `{success, command, …}` envelope pretty-printed as the
/// reference emits it.
fn envelope_pretty(command: &str, extra: &[(&str, Value)]) -> String {
    let mut envelope = Map::new();
    envelope.insert("success".to_owned(), Value::Bool(true));
    envelope.insert("command".to_owned(), json!(command));
    for (key, value) in extra {
        envelope.insert((*key).to_owned(), value.clone());
    }
    serde_json::to_string_pretty(&Value::Object(envelope)).expect("envelope always serializes")
}

fn parse_seed_type(text: &str) -> Result<SeedType, String> {
    SeedType::parse(text)
        .ok_or_else(|| format!("--type must be task|bug|feature|epic, got `{text}`"))
}

fn parse_priority(text: &str) -> Result<u8, String> {
    let digits = text
        .strip_prefix('P')
        .or_else(|| text.strip_prefix('p'))
        .unwrap_or(text);
    let message = format!("--priority must be 0-4 or P0-P4, got `{text}`");
    let number: u8 = digits.parse().map_err(|_| message.clone())?;
    if number <= 4 {
        Ok(number)
    } else {
        Err(message)
    }
}

fn comma_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

fn run_create(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::CREATE_SPEC, helptext::CREATE) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<String, CommandError> {
        let Some(title) = parsed.options.get("title") else {
            // The reference reports missing required options on stderr,
            // without a JSON envelope.
            eprintln!("error: required option '--title <text>' not specified");
            std::process::exit(1);
        };
        let message_of = |text: &str| CommandError::new("create", text, json);
        let seed_type = parse_seed_type(parsed.options.get("type").map_or("task", String::as_str))
            .map_err(|message| message_of(&message))?;
        let priority = parsed
            .options
            .get("priority")
            .map_or(Ok(2_u8), |text| parse_priority(text))
            .map_err(|message| message_of(&message))?;

        let mut store = open_store().map_err(|message| message_of(&message))?;
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
        if let Some(description) = parsed.options.get("description") {
            fields.insert("description".to_owned(), json!(description));
        }
        if let Some(labels) = parsed
            .options
            .get("labels")
            .map(|text| comma_list(text))
            .filter(|labels| !labels.is_empty())
        {
            fields.insert("labels".to_owned(), json!(labels));
        }
        if let Some(assignee) = parsed.options.get("assignee") {
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
            if json {
                println!("{}", envelope_pretty("create", &[("id", json!(id))]));
            } else {
                println!("✓ Created {id}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------

fn run_show(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::SHOW_SPEC, helptext::SHOW) else {
        return ExitCode::FAILURE;
    };
    if parsed.positionals.is_empty() {
        // sd: a usage error on stderr, no envelope — in every mode.
        eprintln!("error: missing required argument 'id'");
        return ExitCode::FAILURE;
    }
    let json = json_mode(&parsed);
    // The reference's show quirk (DEVIATIONS-pinned, seeds-25b5): for a
    // SINGLE missing id, `--json` answers a failure envelope while
    // `--format json` reports on stderr — the alias is not equivalent
    // there. Every show error path exits 1.
    let json_flag = parsed.flags.contains("json");
    let store = match open_store() {
        Ok(store) => store,
        Err(message) => {
            if json {
                let error = CommandError::new("show", message, true);
                return error.report();
            }
            eprintln!("Error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let mut found = Vec::with_capacity(parsed.positionals.len());
    let mut missing = Vec::new();
    for id in &parsed.positionals {
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
            if found.len() == 1 {
                println!(
                    "{}",
                    envelope_pretty("show", &[("issue", issues[0].clone())])
                );
            } else {
                println!(
                    "{}",
                    envelope_pretty("show", &[
                        ("issues", json!(issues)),
                        ("results", json!(results)),
                    ])
                );
            }
            return ExitCode::SUCCESS;
        }
        if found.is_empty() && missing.len() == 1 {
            // Single missing id: envelope only for the `--json` form.
            if json_flag {
                let error = CommandError::new("show", not_found(&missing[0]), true);
                return error.report();
            }
            eprintln!("Error: {}", not_found(&missing[0]));
            return ExitCode::FAILURE;
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
        println!(
            "{}",
            serde_json::to_string_pretty(&Value::Object(failure))
                .expect("envelope always serializes")
        );
        return ExitCode::FAILURE;
    }
    if missing.is_empty() {
        let text = render::show_text(&found, render_mode(&parsed));
        print!("{text}");
        return ExitCode::SUCCESS;
    }
    if found.is_empty() && missing.len() == 1 {
        eprintln!("Error: {}", not_found(&missing[0]));
        return ExitCode::FAILURE;
    }
    // Partial multi-show: the found records, then a per-missing ✗ line.
    let text = render::show_text(&found, render_mode(&parsed));
    print!("{text}");
    for id in &missing {
        eprintln!("✗ {id}: {}", not_found(id));
    }
    ExitCode::FAILURE
}

// ---------------------------------------------------------------------------
// list / ready / search (shared filter pipeline)
// ---------------------------------------------------------------------------

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
    fn from_parsed(parsed: &Parsed) -> Result<Self, String> {
        let status = parsed
            .options
            .get("status")
            .map(|text| {
                Status::parse(text).ok_or_else(|| {
                    format!("--status must be open|in_progress|closed, got `{text}`")
                })
            })
            .transpose()?;
        let kind = parsed
            .options
            .get("type")
            .map(|text| parse_seed_type(text))
            .transpose()?;
        let priority = parsed
            .options
            .get("priority")
            .map(|text| {
                text.split(',')
                    .map(parse_priority)
                    .collect::<Result<HashSet<u8>, _>>()
            })
            .transpose()?;
        let priority_max = parsed
            .options
            .get("priority-max")
            .map(|text| {
                text.parse::<u8>()
                    .map_err(|_| format!("--priority-max must be 0-4, got `{text}`"))
            })
            .transpose()?
            .filter(|number| *number <= 4);
        let limit = parsed
            .options
            .get("limit")
            .map(|text| {
                text.parse::<usize>()
                    .map_err(|_| format!("--limit must be a positive number, got `{text}`"))
            })
            .transpose()?
            .unwrap_or(50);
        Ok(Self {
            status,
            kind,
            assignee: parsed.options.get("assignee").cloned(),
            all: parsed.flags.contains("all"),
            label: parsed
                .options
                .get("label")
                .map(|text| comma_list(text))
                .unwrap_or_default(),
            label_any: parsed
                .options
                .get("label-any")
                .map(|text| comma_list(text))
                .unwrap_or_default(),
            unlabeled: parsed.flags.contains("unlabeled"),
            priority,
            priority_max,
            limit,
            sort: parsed
                .options
                .get("sort")
                .cloned()
                .unwrap_or_else(|| "priority".to_owned()),
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
                let priority = record.priority().map_or(4_u8, seeds::Priority::get);
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
                let pa = a.priority().map_or(4_u8, seeds::Priority::get);
                let pb = b.priority().map_or(4_u8, seeds::Priority::get);
                pa.cmp(&pb).then(created(b).cmp(&created(a)))
            }),
        }
        selected.truncate(self.limit);
        selected
    }
}

fn store_has_unresolved_blockers(store: &Store, record: &SeedRecord) -> bool {
    record.blocked_by().iter().any(|dep| {
        store
            .issue(dep)
            .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
    })
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

fn run_list(args: &[String]) -> ExitCode {
    run_query(args, args::LIST_SPEC, helptext::LIST, "list", false)
}

fn run_ready(args: &[String]) -> ExitCode {
    let wants_schedule = args.iter().any(|arg| arg == "--respect-schedule");
    run_query(
        args,
        args::READY_SPEC,
        helptext::READY,
        "ready",
        wants_schedule,
    )
}

fn run_search(args: &[String]) -> ExitCode {
    run_query(args, args::SEARCH_SPEC, helptext::SEARCH, "search", false)
}

fn run_query(
    args: &[String],
    spec: &[OptSpec],
    help: &str,
    command: &str,
    respect_schedule: bool,
) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, spec, help) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<(Vec<SeedRecord>, Option<String>), CommandError> {
        let filters = Filters::from_parsed(&parsed)
            .map_err(|message| CommandError::new(command, message, json))?;
        let store = open_store().map_err(|message| CommandError::new(command, message, json))?;
        let query = parsed.positionals.first().cloned();
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
        if respect_schedule {
            selected.retain(|record| !scheduled_out(record));
        }
        let owned: Vec<SeedRecord> = selected.into_iter().cloned().collect();
        Ok((owned, query))
    })();
    match result {
        Ok((records, query)) => {
            let mode = render_mode(&parsed);
            if json {
                let issues: Vec<Value> = records.iter().map(issue_value).collect();
                // The reference's query envelopes end with the result
                // count (pinned by the differential battery, seeds-25b5).
                let mut extra: Vec<(&str, Value)> =
                    vec![("issues", json!(issues)), ("count", json!(issues.len()))];
                if let Some(query) = &query {
                    extra.insert(0, ("query", json!(query)));
                }
                println!("{}", envelope_pretty(command, &extra));
            } else {
                let store = open_store();
                let unresolved = |dep: &str| -> bool {
                    store.as_ref().map_or(true, |store| {
                        store
                            .issue(dep)
                            .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
                    })
                };
                let text = render::list_text(&records, mode, &unresolved, command);
                print!("{text}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

fn run_update(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::UPDATE_SPEC, helptext::UPDATE) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<SeedRecord, CommandError> {
        let message_of = |text: String| CommandError::new("update", text, json);
        let Some(id) = parsed.positionals.first() else {
            return Err(message_of("usage: sd update <id>".to_owned()));
        };
        let mut store = open_store().map_err(&message_of)?;
        if store.issue(id).is_none() {
            return Err(message_of(format!("Issue not found: {id}")));
        }
        let now = timeutil::now_iso();
        {
            let record = store.issue_mut(id).expect("existence checked above");
            if let Some(status) = parsed.options.get("status") {
                let status = Status::parse(status).ok_or_else(|| {
                    message_of(format!(
                        "--status must be open|in_progress|closed, got `{status}`"
                    ))
                })?;
                record.set_status(status);
            }
            if let Some(title) = parsed.options.get("title") {
                record.set_title(title);
            }
            match parsed.options.get("assignee") {
                Some(assignee) if assignee.is_empty() => {
                    record.set_assignee(None);
                }
                Some(assignee) => {
                    record.set_assignee(Some(assignee));
                }
                None => {}
            }
            if let Some(description) = parsed.options.get("description") {
                record.set_description(Some(description));
            }
            if let Some(kind) = parsed.options.get("type") {
                let kind = parse_seed_type(kind).map_err(&message_of)?;
                record.set_seed_type(kind);
            }
            if let Some(text) = parsed.options.get("priority") {
                let number = parse_priority(text).map_err(&message_of)?;
                record
                    .set_priority(number)
                    .map_err(|error| message_of(error.to_string()))?;
            }
            if let Some(text) = parsed.options.get("add-label") {
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
            if let Some(text) = parsed.options.get("remove-label") {
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
            if let Some(text) = parsed.options.get("set-labels") {
                let labels = comma_list(text);
                if labels.is_empty() {
                    record.remove_field("labels");
                } else {
                    record.set_labels(labels);
                }
            }
            if let Some(text) = parsed.options.get("extensions") {
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
            if parsed.flags.contains("clear-extensions") {
                record.remove_field("extensions");
            }
            record.set_field("updatedAt", json!(now));
        }
        store
            .save()
            .map_err(|error| message_of(error.to_string()))?;
        Ok(store.issue(id).expect("record was just updated").clone())
    })();
    match result {
        Ok(record) => {
            if json {
                let issue = issue_value(&record);
                println!("{}", envelope_pretty("update", &[("issue", issue)]));
            } else {
                println!("✓ Updated {}", record.id());
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// close
// ---------------------------------------------------------------------------

fn run_close(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::CLOSE_SPEC, helptext::CLOSE) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<Vec<String>, CommandError> {
        if parsed.positionals.is_empty() {
            return Err(CommandError::new(
                "close",
                "usage: sd close <id> [ids...]",
                json,
            ));
        }
        let mut store =
            open_store().map_err(|message| CommandError::new("close", message, json))?;
        for id in &parsed.positionals {
            if store.issue(id).is_none() {
                return Err(CommandError::new(
                    "close",
                    format!("Issue not found: {id}"),
                    json,
                ));
            }
        }
        let now = timeutil::now_iso();
        let reason = parsed.options.get("reason");
        let mut closed = Vec::with_capacity(parsed.positionals.len());
        for id in &parsed.positionals {
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
            if json {
                println!("{}", envelope_pretty("close", &[("closed", json!(closed))]));
            } else {
                let reason = parsed.options.get("reason");
                for id in &closed {
                    match reason {
                        Some(reason) => println!("✓ Closed {id} — {reason}"),
                        None => println!("✓ Closed {id}"),
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// dep add
// ---------------------------------------------------------------------------

fn run_dep(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("add") => run_dep_add(&args[1..]),
        Some("remove") => run_dep_remove(&args[1..]),
        Some("list") => run_dep_list(&args[1..]),
        Some("-h" | "--help") | None => {
            println!("{}", helptext::DEP);
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!(
                "error: dep subcommand '{other}' is not implemented yet \
                 (this build implements `dep add`, `dep remove`, `dep list`)"
            );
            ExitCode::FAILURE
        }
    }
}

fn run_dep_add(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEP_ADD_SPEC, helptext::DEP_ADD) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<(String, String), CommandError> {
        if parsed.positionals.len() != 2 {
            return Err(CommandError::new(
                "dep",
                "usage: sd dep add <issue> <depends-on>",
                json,
            ));
        }
        let issue_id = parsed.positionals[0].clone();
        let depends_on_id = parsed.positionals[1].clone();
        let mut store = open_store().map_err(|message| CommandError::new("dep", message, json))?;
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
            if json {
                println!(
                    "{}",
                    envelope_pretty("dep add", &[
                        ("issueId", json!(issue_id)),
                        ("dependsOnId", json!(depends_on_id)),
                    ],)
                );
            } else {
                println!("Added dependency: {issue_id} → {depends_on_id}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// dep remove / dep list (hygiene batch, seeds-c228)
// ---------------------------------------------------------------------------

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

fn run_dep_remove(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEP_REMOVE_SPEC, helptext::DEP_REMOVE) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<(String, String), CommandError> {
        if parsed.positionals.len() != 2 {
            return Err(CommandError::new(
                "dep",
                "usage: sd dep remove <issue> <depends-on>",
                json,
            ));
        }
        let issue_id = parsed.positionals[0].clone();
        let depends_on_id = parsed.positionals[1].clone();
        let mut store = open_store().map_err(|message| CommandError::new("dep", message, json))?;
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
            if json {
                println!(
                    "{}",
                    envelope_pretty("dep remove", &[
                        ("issueId", json!(issue_id)),
                        ("dependsOnId", json!(depends_on_id)),
                    ])
                );
            } else {
                println!("Removed dependency: {issue_id} → {depends_on_id}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

fn run_dep_list(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEP_LIST_SPEC, helptext::DEP_LIST) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<SeedRecord, CommandError> {
        let Some(id) = parsed.positionals.first() else {
            return Err(CommandError::new("dep", "usage: sd dep list <issue>", json));
        };
        let store = open_store().map_err(|message| CommandError::new("dep", message, json))?;
        store
            .issue(id)
            .cloned()
            .ok_or_else(|| CommandError::new("dep", format!("Issue not found: {id}"), json))
    })();
    match result {
        Ok(record) => {
            if json {
                let blocked_by: Vec<Value> = record
                    .blocked_by()
                    .into_iter()
                    .map(|id| json!(id))
                    .collect();
                let blocks: Vec<Value> = record.blocks().into_iter().map(|id| json!(id)).collect();
                println!(
                    "{}",
                    envelope_pretty("dep list", &[
                        ("issueId", json!(record.id().as_str())),
                        ("blockedBy", Value::Array(blocked_by)),
                        ("blocks", Value::Array(blocks)),
                    ])
                );
            } else {
                let store = open_store().expect("store opened above");
                let unresolved = |dep: &str| -> bool {
                    store
                        .issue(dep)
                        .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
                };
                print!("{}", render::dep_list_text(&record, &store, &unresolved));
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// blocked / block / unblock
// ---------------------------------------------------------------------------

fn run_blocked(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::BLOCKED_SPEC, helptext::BLOCKED) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<Vec<SeedRecord>, CommandError> {
        let store = open_store().map_err(|message| CommandError::new("blocked", message, json))?;
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
            if json {
                let issues: Vec<Value> = records.iter().map(issue_value).collect();
                println!(
                    "{}",
                    envelope_pretty("blocked", &[
                        ("issues", json!(issues)),
                        ("count", json!(issues.len())),
                    ])
                );
            } else {
                let mode = render_mode(&parsed);
                let store = open_store();
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
                    println!("No blocked issues.");
                } else {
                    print!("{text}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

fn run_block(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::BLOCK_SPEC, helptext::BLOCK) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<(String, String), CommandError> {
        let Some(id) = parsed.positionals.first() else {
            eprintln!("error: missing required argument 'id'");
            std::process::exit(1);
        };
        let Some(blocker_id) = parsed.options.get("by") else {
            eprintln!("Error: Usage: sd block <id> --by <blocker-id>");
            std::process::exit(1);
        };
        let (id, blocker_id) = (id.clone(), blocker_id.clone());
        let mut store =
            open_store().map_err(|message| CommandError::new("block", message, json))?;
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
            if json {
                println!(
                    "{}",
                    envelope_pretty("block", &[
                        ("issueId", json!(id)),
                        ("blockerId", json!(blocker_id)),
                    ])
                );
            } else {
                println!("{id} is now blocked by {blocker_id}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

fn run_unblock(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::UNBLOCK_SPEC, helptext::UNBLOCK) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<(String, Vec<String>), CommandError> {
        let Some(id) = parsed.positionals.first() else {
            eprintln!("error: missing required argument 'id'");
            std::process::exit(1);
        };
        let id = id.clone();
        let from = parsed.options.get("from").cloned();
        let all = parsed.flags.contains("all");
        if from.is_none() && !all {
            eprintln!("Error: Usage: sd unblock <id> [--from <blocker-id> | --all]");
            std::process::exit(1);
        }
        let mut store =
            open_store().map_err(|message| CommandError::new("unblock", message, json))?;
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
            if json {
                let removed_value: Vec<Value> = removed.iter().map(|r| json!(r)).collect();
                println!(
                    "{}",
                    envelope_pretty("unblock", &[
                        ("issueId", json!(id)),
                        ("removed", Value::Array(removed_value)),
                    ])
                );
            } else if removed.is_empty() {
                println!("No closed blockers to remove from {id}.");
            } else {
                println!("{id} unblocked from {}", removed.join(", "));
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// label
// ---------------------------------------------------------------------------

fn run_label(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("add") => run_label_add(&args[1..]),
        Some("remove") => run_label_remove(&args[1..]),
        Some("list") => run_label_list(&args[1..]),
        Some("list-all") => run_label_list_all(&args[1..]),
        Some("-h" | "--help") => {
            println!("{}", helptext::LABEL);
            ExitCode::SUCCESS
        }
        // A bare `sd label` prints its usage on stderr, exit 1.
        None => {
            eprintln!("{}", helptext::LABEL);
            ExitCode::FAILURE
        }
        Some(other) => {
            eprintln!("error: unknown command '{other}' for 'label'");
            ExitCode::FAILURE
        }
    }
}

fn run_label_add(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_ADD_SPEC, helptext::LABEL_ADD) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<(String, Vec<String>), CommandError> {
        let Some(id) = parsed.positionals.first() else {
            return Err(CommandError::new(
                "label",
                "usage: sd label add <issue> <labels...>",
                json,
            ));
        };
        let id = id.clone();
        if parsed.positionals.len() < 2 {
            return Err(CommandError::new(
                "label",
                "usage: sd label add <issue> <labels...>",
                json,
            ));
        }
        let labels = parsed.positionals[1..].to_vec();
        let mut store =
            open_store().map_err(|message| CommandError::new("label", message, json))?;
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
            if json {
                let labels_value: Vec<Value> = labels.iter().map(|l| json!(l)).collect();
                println!(
                    "{}",
                    envelope_pretty("label add", &[
                        ("issueId", json!(id)),
                        ("labels", Value::Array(labels_value)),
                    ])
                );
            } else {
                println!("✓ Added label(s) {} to {id}", labels.join(", "));
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

fn run_label_remove(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_REMOVE_SPEC, helptext::LABEL_REMOVE) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<(String, Vec<String>), CommandError> {
        let Some(id) = parsed.positionals.first() else {
            return Err(CommandError::new(
                "label",
                "usage: sd label remove <issue> <labels...>",
                json,
            ));
        };
        let id = id.clone();
        if parsed.positionals.len() < 2 {
            return Err(CommandError::new(
                "label",
                "usage: sd label remove <issue> <labels...>",
                json,
            ));
        }
        let labels = parsed.positionals[1..].to_vec();
        let mut store =
            open_store().map_err(|message| CommandError::new("label", message, json))?;
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
            if json {
                let labels_value: Vec<Value> = labels.iter().map(|l| json!(l)).collect();
                println!(
                    "{}",
                    envelope_pretty("label remove", &[
                        ("issueId", json!(id)),
                        ("labels", Value::Array(labels_value)),
                    ])
                );
            } else {
                println!("✓ Removed label(s) from {id}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

fn run_label_list(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_LIST_SPEC, helptext::LABEL_LIST) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<SeedRecord, CommandError> {
        let Some(id) = parsed.positionals.first() else {
            return Err(CommandError::new(
                "label",
                "usage: sd label list <issue>",
                json,
            ));
        };
        let store = open_store().map_err(|message| CommandError::new("label", message, json))?;
        store
            .issue(id)
            .cloned()
            .ok_or_else(|| CommandError::new("label", format!("Issue not found: {id}"), json))
    })();
    match result {
        Ok(record) => {
            if json {
                let labels: Vec<Value> = record
                    .labels()
                    .into_iter()
                    .map(|label| json!(label))
                    .collect();
                println!(
                    "{}",
                    envelope_pretty("label list", &[
                        ("issueId", json!(record.id().as_str())),
                        ("labels", Value::Array(labels)),
                    ])
                );
            } else {
                print!("{}", render::label_list_text(&record));
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

fn run_label_list_all(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_LIST_ALL_SPEC, helptext::LABEL_LIST_ALL)
    else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<Vec<(String, usize)>, CommandError> {
        let store = open_store().map_err(|message| CommandError::new("label", message, json))?;
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
            if json {
                let mut sorted: Vec<&(String, usize)> = counts.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                let labels: Vec<Value> = sorted.iter().map(|(label, _)| json!(label)).collect();
                // The counts object keeps encounter order (the
                // reference's insertion-ordered map).
                let mut counts_map = Map::new();
                for (label, count) in &counts {
                    counts_map.insert(label.clone(), json!(count));
                }
                println!(
                    "{}",
                    envelope_pretty("label list-all", &[
                        ("labels", Value::Array(labels)),
                        ("counts", Value::Object(counts_map)),
                    ])
                );
            } else {
                print!("{}", render::label_list_all_text(&counts));
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// stats
// ---------------------------------------------------------------------------

fn run_stats(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::STATS_SPEC, helptext::STATS) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let result = (|| -> Result<render::Stats, CommandError> {
        let store = open_store().map_err(|message| CommandError::new("stats", message, json))?;
        let unresolved = |dep: &str| -> bool {
            store
                .issue(dep)
                .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
        };
        Ok(render::Stats::collect(&store.issues, &unresolved))
    })();
    match result {
        Ok(stats) => {
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
                println!(
                    "{}",
                    envelope_pretty("stats", &[("stats", Value::Object(stats_map))])
                );
            } else {
                print!("{}", stats.text());
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// doctor
// ---------------------------------------------------------------------------

fn run_doctor(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DOCTOR_SPEC, helptext::DOCTOR) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let fix = parsed.flags.contains("fix");
    let repair_report = parsed.flags.contains("repair-report");
    let result = (|| -> Result<doctor::Report, CommandError> {
        let root =
            find_seeds_dir().map_err(|message| CommandError::new("doctor", message, json))?;
        doctor::run(&root, fix).map_err(|message| CommandError::new("doctor", message, json))
    })();
    match result {
        Ok(report) => {
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
                println!(
                    "{}",
                    serde_json::to_string_pretty(&Value::Object(envelope))
                        .expect("envelope always serializes")
                );
            } else {
                print!("{}", report.text());
                if repair_report {
                    print!("{}", report.repair_report_text());
                }
            }
            if report.has_failures() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => error.report(),
    }
}

// ---------------------------------------------------------------------------
// prime
// ---------------------------------------------------------------------------

fn run_prime(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::PRIME_SPEC, helptext::PRIME) else {
        return ExitCode::FAILURE;
    };
    if json_mode(&parsed) {
        let compact = parsed.flags.contains("compact");
        // The reference's JSON sections (differential-pinned,
        // seeds-25b5): full mode is the five core sections PLUS the
        // captured commandGroups/workflows; compact mode is a different
        // section set entirely.
        let sections = if compact {
            serde_json::from_str::<Value>(helptext::PRIME_JSON_COMPACT)
                .expect("captured compact sections are valid JSON")
        } else {
            let mut sections = json!({
                "mode": "full",
                "title": "Seeds Workflow Context",
                "contextRecovery": "Run `sd prime` after compaction, clear, or new session",
                "closeProtocol": {
                    "warning": "Before saying \"done\" or \"complete\", you MUST run this checklist:",
                    "steps": [
                        "Close completed issues:    sd close <id1> <id2> ...",
                        "File issues for remaining:  sd create --title \"...\"",
                        "Run quality gates:          bun test && bun run lint && bun run typecheck",
                        "Sync and push:              sd sync && git push",
                        "Verify:                     git status (must show \"up to date with origin\")"
                    ],
                    "footer": "**NEVER skip this.** Work is not done until pushed."
                },
                "rules": [
                    "**Default**: Use seeds for ALL task tracking (`sd create`, `sd ready`, `sd close`)",
                    "**Prohibited**: Do NOT use TodoWrite, TaskCreate, or markdown files for task tracking",
                    "**Workflow**: Create issues BEFORE writing code, mark in_progress when starting",
                    "Git workflow: run `sd sync` at session end"
                ]
            });
            let extra = serde_json::from_str::<Value>(helptext::PRIME_JSON_FULL_EXTRA)
                .expect("captured extra sections are valid JSON");
            let Value::Object(map) = &mut sections else {
                unreachable!("sections is an object");
            };
            let Value::Object(extra) = extra else {
                unreachable!("captured extra sections are an object");
            };
            for (key, value) in extra {
                map.insert(key, value);
            }
            sections
        };
        // The reference's envelope carries BOTH the structured sections
        // and the raw template text under `content` (full or compact
        // template matching the mode, trailing newline included).
        let content = format!(
            "{}\n",
            if compact {
                helptext::PRIME_COMPACT
            } else {
                helptext::PRIME_FULL
            }
        );
        println!(
            "{}",
            envelope_pretty("prime", &[
                ("sections", sections),
                ("content", json!(content)),
            ])
        );
    } else if parsed.flags.contains("compact") {
        println!("{}", helptext::PRIME_COMPACT);
    } else {
        // `--export` emits the default (full) template, like the reference.
        println!("{}", helptext::PRIME_FULL);
    }
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// dedupe (native, beyond sd parity — ADR-0023 additive)
// ---------------------------------------------------------------------------

/// One file's dedupe outcome for reporting.
struct DedupeFile {
    name:          &'static str,
    duplicates:    Vec<dedupe::Duplicate>,
    dropped_lines: usize,
}

impl DedupeFile {
    fn duplicate_ids(&self) -> usize {
        self.duplicates.len()
    }

    fn duplicates_value(&self) -> Value {
        Value::Array(
            self.duplicates
                .iter()
                .map(|duplicate| {
                    json!({
                        "id": duplicate.id,
                        "count": duplicate.count,
                        "keptUpdatedAt": duplicate.kept_updated_at,
                        "droppedUpdatedAts": duplicate.dropped_updated_ats,
                    })
                })
                .collect(),
        )
    }
}

fn run_dedupe(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEDUPE_SPEC, helptext::DEDUPE) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let write = parsed.flags.contains("write");
    let result = (|| -> Result<Vec<DedupeFile>, CommandError> {
        let message_of = |text: String| CommandError::new("dedupe", text, json);
        let root = find_seeds_dir().map_err(&message_of)?;
        let mut files = Vec::new();
        for name in dedupe::TRACKER_FILES {
            let path = root.join(name);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue; // a missing store file is empty, nothing to heal
            };
            let lines: Vec<&str> = text
                .lines()
                .map(|line| line.trim_end_matches(['\r', '\n']))
                .filter(|line| !line.is_empty())
                .collect();
            let (healed, duplicates) = dedupe::heal_lines(&lines);
            let dropped_lines = lines.len() - healed.len();
            if write && !duplicates.is_empty() {
                let body = format!("{}\n", healed.join("\n"));
                dedupe::write_atomic(&path, &body).map_err(|source| {
                    message_of(format!("writing {}: {source}", path.display()))
                })?;
            }
            files.push(DedupeFile {
                name,
                duplicates,
                dropped_lines,
            });
        }
        Ok(files)
    })();
    let files = match result {
        Ok(files) => files,
        Err(error) => return error.report(),
    };

    let duplicate_ids: usize = files.iter().map(DedupeFile::duplicate_ids).sum();
    let dropped_total: usize = files.iter().map(|file| file.dropped_lines).sum();
    if json {
        let mut extra = vec![
            ("write", json!(write)),
            (
                "files",
                Value::Array(
                    files
                        .iter()
                        .map(|file| {
                            json!({
                                "file": file.name,
                                "duplicates": file.duplicates_value(),
                            })
                        })
                        .collect(),
                ),
            ),
            ("duplicateIds", json!(duplicate_ids)),
        ];
        if write {
            extra.push(("droppedLines", json!(dropped_total)));
            extra.push(("written", json!(duplicate_ids > 0)));
        }
        println!("{}", envelope_pretty("dedupe", &extra));
    } else if write {
        for file in files.iter().filter(|file| file.dropped_lines > 0) {
            println!(
                "{}: dropped {} duplicate lines ({} ids)",
                file.name,
                file.dropped_lines,
                file.duplicate_ids()
            );
        }
        if duplicate_ids == 0 {
            println!("✓ no duplicate ids found (nothing to write)");
        } else {
            println!("✓ healed {duplicate_ids} duplicate ids, dropped {dropped_total} lines");
        }
    } else {
        for file in &files {
            if file.duplicates.is_empty() {
                continue;
            }
            println!("{}: {} duplicate ids", file.name, file.duplicate_ids());
            for duplicate in &file.duplicates {
                let kept = duplicate
                    .kept_updated_at
                    .as_deref()
                    .unwrap_or("(no updatedAt)");
                let dropped = duplicate
                    .dropped_updated_ats
                    .iter()
                    .map(|value| value.as_deref().unwrap_or("(no updatedAt)"))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "  {} ×{} kept updatedAt={} dropped: {}",
                    duplicate.id, duplicate.count, kept, dropped
                );
            }
        }
        if duplicate_ids == 0 {
            println!("✓ no duplicate ids found");
        }
    }
    // Report mode is gate-friendly: non-zero exactly when duplicates
    // remain; `--write` heals by definition and always succeeds.
    if !write && duplicate_ids > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

// ---------------------------------------------------------------------------
// sync
// ---------------------------------------------------------------------------

/// One `seeds sync` outcome, rendered per mode (seeds-540e).
enum SyncOutcome {
    /// Nothing dirty under `.seeds/` — plain and `--dry-run` runs.
    NoChanges,
    /// `--status`: the per-file change preview (never commits).
    Status(String),
    /// `--dry-run` on a dirty store (never commits).
    DryRun { changes: String, message: String },
    /// A commit was created.
    Committed(String),
}

impl SyncOutcome {
    fn report(&self, json: bool) -> ExitCode {
        match self {
            SyncOutcome::NoChanges => {
                if json {
                    println!(
                        "{}",
                        envelope_pretty("sync", &[
                            ("committed", json!(false)),
                            ("message", json!("Nothing to commit"))
                        ],)
                    );
                } else {
                    println!("✓ No changes to commit.");
                }
            }
            SyncOutcome::Status(changes) => {
                if json {
                    println!(
                        "{}",
                        envelope_pretty("sync", &[
                            ("hasChanges", json!(!changes.is_empty())),
                            ("changes", json!(changes)),
                        ],)
                    );
                } else if changes.is_empty() {
                    println!("✓ No uncommitted .seeds/ changes.");
                } else {
                    println!("✓ Uncommitted .seeds/ changes:");
                    println!("{changes}");
                }
            }
            SyncOutcome::DryRun { changes, message } => {
                if json {
                    println!(
                        "{}",
                        envelope_pretty("sync", &[
                            ("dryRun", json!(true)),
                            ("wouldCommit", json!(true)),
                            ("message", json!(message)),
                            ("changes", json!(changes)),
                        ],)
                    );
                } else {
                    println!("✓ Dry run — would commit:");
                    println!("{changes}");
                    println!("Commit message: {message}");
                }
            }
            SyncOutcome::Committed(message) => {
                if json {
                    println!(
                        "{}",
                        envelope_pretty("sync", &[
                            ("committed", json!(true)),
                            ("message", json!(message))
                        ],)
                    );
                } else {
                    println!("✓ Committed: {message}");
                }
            }
        }
        ExitCode::SUCCESS
    }
}

fn run_sync(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::SYNC_SPEC, helptext::SYNC) else {
        return ExitCode::FAILURE;
    };
    let json = json_mode(&parsed);
    let status = parsed.flags.contains("status");
    let dry_run = parsed.flags.contains("dry-run");
    let force = parsed.flags.contains("force");
    let result = (|| -> Result<SyncOutcome, CommandError> {
        let message_of = |text: String| CommandError::new("sync", text, json);
        let seeds_dir = find_seeds_dir()
            .map_err(|_| message_of("Not in a seeds project. Run `sd init` first.".to_owned()))?;
        let Some(repo) = git_repo_root(seeds_dir.parent().unwrap_or(Path::new("."))) else {
            // The reference's observed behavior outside a git worktree.
            return Ok(SyncOutcome::NoChanges);
        };
        let seeds_path = seeds_dir.to_string_lossy().into_owned();
        let changes = git(&repo, &[
            "status",
            "--porcelain",
            "-uall",
            "--",
            &seeds_path,
        ])
        .map_err(message_of)?;
        let changes = changes
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if status {
            return Ok(SyncOutcome::Status(changes));
        }
        if changes.is_empty() {
            return Ok(SyncOutcome::NoChanges);
        }
        let message = format!("seeds: sync {}", timeutil::today_utc());
        if dry_run {
            return Ok(SyncOutcome::DryRun { changes, message });
        }
        // Push-gate safety (seeds-540e): in fabro repos the tracker
        // must not race a running pass — a refused gate blocks the
        // commit; `--force` is the human override.
        if !force {
            let gate = repo.join(".fabro").join("scripts").join("push-gate.nu");
            if gate.is_file()
                && let Some(reason) = push_gate_refusal(&repo, &gate)
            {
                return Err(message_of(format!(
                    "push gate refused — not committing .seeds/ while a pass \
                     may be running: {reason} (override with --force)"
                )));
            }
        }
        git(&repo, &["add", "-A", "--", &seeds_path]).map_err(message_of)?;
        // The shortstat body line makes sync history greppable by size.
        let shortstat = git(&repo, &[
            "diff",
            "--cached",
            "--shortstat",
            "--",
            &seeds_path,
        ])
        .map_err(message_of)?;
        let shortstat = shortstat.trim();
        let mut commit_args = vec!["commit", "-m", message.as_str()];
        if !shortstat.is_empty() {
            commit_args.push("-m");
            commit_args.push(shortstat);
        }
        git(&repo, &commit_args).map_err(message_of)?;
        Ok(SyncOutcome::Committed(message))
    })();
    match result {
        Ok(outcome) => outcome.report(json),
        Err(error) => error.report(),
    }
}

/// Runs `git` in `repo`, returning trimmed stdout; a non-zero exit
/// carries git's stderr.
fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|source| format!("spawning git: {source}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if stderr.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            format!("git {}: {stderr}", args.join(" "))
        })
    }
}

/// The repository root containing `dir`, or `None` outside a worktree.
fn git_repo_root(dir: &Path) -> Option<std::path::PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
}

/// Runs the fabro push gate; `Some(reason)` when it refused (non-zero
/// exit). A gate that cannot run at all (no `nu`) does not block sync
/// — documented in the README's DEVIATIONS section.
fn push_gate_refusal(repo: &Path, gate: &Path) -> Option<String> {
    let output = Command::new("nu")
        .arg(gate)
        .current_dir(repo)
        .output()
        .ok()?;
    if output.status.success() {
        return None;
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let reason = text
        .lines()
        .find(|line| line.contains("GATE REFUSED"))
        .map_or_else(|| "gate exited non-zero".to_owned(), str::to_owned);
    Some(reason)
}
