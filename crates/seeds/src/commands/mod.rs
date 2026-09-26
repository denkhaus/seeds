//! The command layer: sd 0.5.15-compatible command semantics as a
//! public library API (seeds-0dfd), shaped for external compile-in
//! consumers that embed the tracker natively.
//!
//! argv parsing and printing stay with the binary; everything a command
//! DOES — validation, defaults, store mutation, and the `{success,
//! command, …}` JSON envelopes — lives here. Every command function
//! takes a [`CommandContext`] (which `.seeds/` directory to act on) and
//! a typed input struct, and returns a [`CommandOutcome`] carrying the
//! exact stdout/stderr bytes the CLI prints plus a success flag —
//! library calls never print and never exit the process.
//!
//! Output shapes are byte-stable against the reference (pinned by the
//! differential battery): envelopes serialize exactly as the binary
//! printed them before the lift.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::{SeedRecord, SeedType, Status, Store, timeutil};

mod completions;
mod config_init;
mod create_show;
mod dedupe_sync;
mod dep;
mod label;
mod onboard;
mod plan;
mod prime;
mod query;
mod stats_doctor;
mod tpl;
mod update_close;
#[cfg(feature = "upgrade")]
mod upgrade;

pub use completions::{CompletionsInput, completions};
pub use config_init::{ConfigInput, ConfigSub, InitInput, config, init};
pub use create_show::{CreateInput, ShowInput, create, show};
pub use dedupe_sync::{DedupeInput, SyncInput, dedupe, sync};
pub use dep::{
    BlockInput, BlockedInput, DepAddInput, DepListInput, DepRemoveInput, UnblockInput, block,
    blocked, dep_add, dep_list, dep_remove, unblock,
};
pub use label::{
    LabelAddInput, LabelListAllInput, LabelListInput, LabelRemoveInput, label_add, label_list,
    label_list_all, label_remove,
};
pub use onboard::{OnboardInput, onboard};
pub use plan::{PlanInput, PlanSub, plan};
pub use prime::{PrimeInput, prime};
pub use query::{QueryCommand, QueryInput, list, query, ready, search};
pub use stats_doctor::{DoctorInput, StatsInput, doctor, stats};
pub use tpl::{TplInput, TplSub, tpl};
pub use update_close::{CloseInput, UpdateInput, close, update};
#[cfg(feature = "upgrade")]
pub use upgrade::{InstallClass, UpgradeInput, classify_install, upgrade};

/// One command's full observable result: the bytes to print and the
/// process-exit meaning. The binary prints `stdout`/`stderr` verbatim
/// and maps `success` to its exit code; the fields are the byte-stable
/// contract, so they are plain and public.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandOutcome {
    /// The success meaning of the command (exit 0 when true).
    pub success: bool,
    /// The exact bytes the CLI prints on stdout (newlines included).
    pub stdout:  String,
    /// The exact bytes the CLI prints on stderr (newlines included).
    pub stderr:  String,
}

impl CommandOutcome {
    /// A successful outcome with the given stdout and no stderr.
    fn ok(stdout: String) -> Self {
        Self {
            success: true,
            stdout,
            stderr: String::new(),
        }
    }
}

/// Where a command acts: an explicit `.seeds/` directory (compile-in
/// callers), or the one found at or above the current directory (the
/// CLI's behavior).
#[derive(Clone, Debug, Default)]
pub struct CommandContext {
    seeds_dir: Option<PathBuf>,
}

impl CommandContext {
    /// Acts on the `.seeds/` directory at `dir` (compile-in entry point).
    #[must_use = "the context only takes effect when passed to a command"]
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self {
            seeds_dir: Some(dir.into()),
        }
    }

    /// Acts on the `.seeds/` directory at or above the current
    /// directory, the reference's resolution rule.
    #[must_use = "the context only takes effect when passed to a command"]
    pub fn from_cwd() -> Self {
        Self {
            seeds_dir: find_seeds_dir().ok(),
        }
    }

    /// The resolved `.seeds/` directory.
    fn resolve(&self) -> Result<&Path, String> {
        self.seeds_dir.as_deref().map_or_else(
            || Err("No .seeds directory found (run from the project root)".to_owned()),
            Ok,
        )
    }

    /// Opens the store at the context's `.seeds/` directory.
    fn open_store(&self) -> Result<Store, String> {
        let root = self.resolve()?;
        Store::open(root).map_err(|error| error.to_string())
    }
}

/// Finds the `.seeds/` directory at or above the current directory.
fn find_seeds_dir() -> Result<PathBuf, String> {
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

/// A command failure: rendered as a JSON failure envelope or an
/// `Error: …` stderr line, always unsuccessful — the reference's
/// observed behavior.
struct CommandError {
    command:    String,
    message:    String,
    json:       bool,
    raw_stderr: bool,
}

impl CommandError {
    fn new(command: &str, message: impl Into<String>, json: bool) -> Self {
        Self {
            command: command.to_owned(),
            message: message.into(),
            json,
            raw_stderr: false,
        }
    }

    /// A bare usage error printed verbatim on stderr (no `Error:`
    /// prefix, no envelope) — the reference's missing-required-option
    /// behavior.
    fn stderr_only(message: &str) -> Self {
        Self {
            command:    String::new(),
            message:    message.to_owned(),
            json:       false,
            raw_stderr: true,
        }
    }

    /// Renders the failure exactly as the binary reported it.
    fn into_outcome(self) -> CommandOutcome {
        if self.raw_stderr {
            return CommandOutcome {
                success: false,
                stdout:  String::new(),
                stderr:  format!("{}\n", self.message),
            };
        }
        if self.json {
            let mut envelope = Map::new();
            envelope.insert("success".to_owned(), Value::Bool(false));
            envelope.insert("command".to_owned(), json!(self.command));
            envelope.insert("error".to_owned(), json!(self.message));
            let mut stdout = serde_json::to_string_pretty(&Value::Object(envelope))
                .expect("envelope always serializes");
            stdout.push('\n');
            CommandOutcome {
                success: false,
                stdout,
                stderr: String::new(),
            }
        } else {
            CommandOutcome {
                success: false,
                stdout:  String::new(),
                stderr:  format!("Error: {}\n", self.message),
            }
        }
    }
}

/// Appends `text` and a trailing newline to `buf` — stdout/stderr line
/// accumulation.
fn push_line(buf: &mut String, text: &str) {
    buf.push_str(text);
    buf.push('\n');
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
pub(crate) fn issue_value(record: &SeedRecord) -> Value {
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

pub(crate) fn parse_seed_type(text: &str) -> Result<SeedType, String> {
    SeedType::parse(text)
        .ok_or_else(|| format!("--type must be task|bug|feature|epic, got `{text}`"))
}

pub(crate) fn parse_priority(text: &str) -> Result<u8, String> {
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

pub(crate) fn comma_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether any blocker of `record` is unresolved (missing from the
/// store or not closed).
pub(crate) fn store_has_unresolved_blockers(store: &Store, record: &SeedRecord) -> bool {
    record.blocked_by().iter().any(|dep| {
        store
            .issue(dep)
            .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
    })
}
