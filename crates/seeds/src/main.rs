//! The `seeds` binary: sd 0.5.15-compatible CLI over the format core.
//!
//! Nine-command parity surface (README compat contract, ADR-0023):
//! create, show, list, ready, update, close, dep add, prime, search.
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
use std::process::ExitCode;

use seeds::{Fields, SeedRecord, SeedType, Status, Store};
use serde_json::{Map, Value, json};

mod args;
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
        "prime" => run_prime(rest),
        "search" => run_search(rest),
        other => {
            eprintln!("error: unknown command '{other}'");
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

/// Finds the `.seeds/` store at or above the current directory.
fn open_store() -> Result<Store, String> {
    let mut dir = std::env::current_dir().map_err(|error| error.to_string())?;
    loop {
        let candidate = dir.join(".seeds");
        if candidate.join("config.yaml").is_file() {
            return Store::open(&candidate).map_err(|error| error.to_string());
        }
        if !dir.pop() {
            return Err("No .seeds directory found (run from the project root)".to_owned());
        }
    }
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
    let json = json_mode(&parsed);
    let result = (|| -> Result<Vec<SeedRecord>, CommandError> {
        if parsed.positionals.is_empty() {
            return Err(CommandError::new(
                "show",
                "usage: sd show <id> [ids...]",
                json,
            ));
        }
        let store = open_store().map_err(|message| CommandError::new("show", message, json))?;
        let mut records = Vec::with_capacity(parsed.positionals.len());
        for id in &parsed.positionals {
            let record = store
                .issue(id)
                .ok_or_else(|| CommandError::new("show", format!("Issue not found: {id}"), json))?;
            records.push(record.clone());
        }
        Ok(records)
    })();
    match result {
        Ok(records) => {
            if json {
                if records.len() == 1 {
                    let issue = issue_value(&records[0]);
                    println!("{}", envelope_pretty("show", &[("issue", issue)]));
                } else {
                    let issues: Vec<Value> = records.iter().map(issue_value).collect();
                    println!("{}", envelope_pretty("show", &[("issues", json!(issues))]));
                }
            } else {
                let text = render::show_text(&records, render_mode(&parsed));
                print!("{text}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => error.report(),
    }
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
            return Err(CommandError::new(command, "usage: sd search <query>", json));
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
                let mut extra: Vec<(&str, Value)> = vec![("issues", json!(issues))];
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
        Some("-h" | "--help") | None => {
            println!("{}", helptext::DEP);
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("error: unknown dep subcommand '{other}' (this build implements `dep add`)");
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
// prime
// ---------------------------------------------------------------------------

fn run_prime(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::PRIME_SPEC, helptext::PRIME) else {
        return ExitCode::FAILURE;
    };
    if json_mode(&parsed) {
        let compact = parsed.flags.contains("compact");
        let sections = json!({
            "mode": if compact { "compact" } else { "full" },
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
        println!("{}", envelope_pretty("prime", &[("sections", sections)]));
    } else if parsed.flags.contains("compact") {
        println!("{}", helptext::PRIME_COMPACT);
    } else {
        // `--export` emits the default (full) template, like the reference.
        println!("{}", helptext::PRIME_FULL);
    }
    ExitCode::SUCCESS
}
