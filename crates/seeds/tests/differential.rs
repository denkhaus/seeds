//! Command-level differential battery: the `seeds` binary against the
//! live sd-0.5.15 reference (seeds-25b5) — extends the format
//! round-trips of `compat.rs` to the COMMAND surface.
//!
//! One shared tracker fixture is materialized into a temp dir pair (one
//! directory per binary) for every case; each matrix case runs the SAME
//! argv through both binaries and compares exit code, stdout (parsed as
//! JSON when both sides emit JSON, exact text otherwise — volatile
//! timestamps normalized), and the resulting `issues.jsonl` stores with
//! volatile fields skipped (same drift class as `real_fixture.rs`'s
//! VOLATILE_FIELDS).
//!
//! The matrix is driven by [`IMPLEMENTED_COMMANDS`]: every command on
//! that list must carry at least one case (the coverage-guard test
//! enforces it), so a future command inherits differential coverage by
//! convention — add it to the list and its case in the same change.
//! Deliberate divergences live in the README's DEVIATIONS section and
//! carry their own expectations, never a silent split.
//!
//! Reference provisioning and the skip contract mirror `compat.rs`:
//! the gate provisions the fixture (`scripts/qualitygate.nu`), where a
//! missing reference FAILS; plain local runs keep the skip-with-note.

#![allow(
    clippy::format_push_string,
    reason = "test normalization builds strings incrementally"
)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};

/// The implemented parity command surface — the matrix's coverage
/// driver. `create` is covered by the dedicated tailored case (random
/// ids make a plain argv comparison meaningless there).
const IMPLEMENTED_COMMANDS: &[&str] = &[
    "create",
    "show",
    "list",
    "ready",
    "search",
    "update",
    "close",
    "dep",
    "prime",
    "sync",
    "blocked",
    "block",
    "unblock",
    "label",
    "stats",
    "doctor",
    "plan",
    "init",
    "config",
    "onboard",
    "completions",
];

/// Fields whose values are stamped `now` by both binaries at run time;
/// normalized before every stdout/store comparison. Real content
/// (status, assignee, labels, dependencies) is never in here — the
/// battery must catch divergence in it.
const VOLATILE_FIELDS: &[&str] = &["createdAt", "updatedAt", "closedAt"];

/// The sd-0.5.15 version the reference must report (`sd --version`).
const REFERENCE_VERSION: &str = "0.5.15";

/// Fixture config: same shape as `cli.rs`'s temp stores.
const CONFIG_YAML: &str = "project: \"tst\"\nversion: \"1\"\nmax_plan_depth: 3\n";

/// The shared tracker fixture: six issues covering statuses,
/// priorities, labels, assignees, and both resolved and unresolved
/// dependencies (built identically for both binaries; fixed ids so
/// argv can address them, fixed timestamps so sort orders are stable).
const FIXTURE_IDS: &[&str] = &[
    "tst-0001", "tst-0002", "tst-0003", "tst-0004", "tst-0005", "tst-0006",
];

/// One fixture record: the sd-canonical base shape plus `extra` fields.
fn fixture_record(id: &str, title: &str, extra: Value) -> Value {
    // Distinct createdAt/updatedAt per id (its numeric suffix) so sort
    // orders are deterministic — same convention as cli.rs.
    let suffix = id.rsplit('-').next().unwrap_or("0001");
    let stamp = json!(format!("2026-01-{suffix}T00:00:00.000Z"));
    let mut base = json!({
        "id": id,
        "title": title,
        "status": "open",
        "type": "task",
        "priority": 2,
        "createdAt": stamp,
        "updatedAt": stamp,
    });
    let Value::Object(map) = &mut base else {
        unreachable!("base is an object");
    };
    let Value::Object(extra) = extra else {
        unreachable!("extra is an object");
    };
    for (key, value) in extra {
        map.insert(key, value);
    }
    base
}

fn fixture_records() -> Vec<Value> {
    vec![
        fixture_record(
            "tst-0001",
            "Fix login PARITY bug",
            json!({"status": "open", "type": "bug", "priority": 0,
                   "labels": ["bug", "ui"], "assignee": "alice"}),
        ),
        fixture_record(
            "tst-0002",
            "Write docs",
            json!({"priority": 1, "description": "Mention parity in the guide"}),
        ),
        fixture_record(
            "tst-0003",
            "Refactor core",
            json!({"status": "in_progress", "type": "feature", "priority": 2,
                   "labels": ["bug"], "assignee": "bob"}),
        ),
        fixture_record(
            "tst-0004",
            "Old work",
            json!({"status": "closed", "priority": 3,
                   "closedAt": "2026-01-05T00:00:00.000Z"}),
        ),
        fixture_record(
            "tst-0005",
            "Follow-up",
            json!({"priority": 4, "blockedBy": ["tst-0004"]}),
        ),
        fixture_record(
            "tst-0006",
            "Blocked work",
            json!({"priority": 1, "blockedBy": ["tst-0001"]}),
        ),
    ]
}

/// The fixture as JSONL text — byte-identical for both binaries.
fn issues_jsonl() -> String {
    let mut text = String::new();
    for record in fixture_records() {
        text.push_str(&record.to_string());
        text.push('\n');
    }
    text
}

// ---------------------------------------------------------------------------
// reference resolution (mirrors compat.rs — separate test binaries
// cannot share private helpers)
// ---------------------------------------------------------------------------

/// Fixture wrapper path: `<crate>/tests/fixtures/sd-reference/sd`.
fn reference_wrapper() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sd-reference/sd")
}

/// Resolves the reference binary: the `SEEDS_COMPAT_SD` env override
/// when set, else the committed fixture wrapper.
fn resolve_reference() -> Option<PathBuf> {
    if let Ok(from_env) = std::env::var("SEEDS_COMPAT_SD") {
        let path = PathBuf::from(from_env);
        assert!(
            path.is_file(),
            "SEEDS_COMPAT_SD is not a file: {}",
            path.display()
        );
        return Some(path);
    }
    let wrapper = reference_wrapper();
    wrapper.is_file().then_some(wrapper)
}

fn have(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// The verified reference: resolved, answering `--version` as sd
/// 0.5.15, with `git` available. `None` means unavailable — skip.
fn reference() -> Option<PathBuf> {
    let reference = resolve_reference()?;
    let output = Command::new(&reference).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    assert_eq!(
        version, REFERENCE_VERSION,
        "compat reference must be sd {REFERENCE_VERSION} — reprovision \
         crates/seeds/tests/fixtures/sd-reference (bun install)"
    );
    have("git").then_some(reference)
}

#[allow(
    clippy::print_stderr,
    reason = "the skip-with-note contract requires a visible note on stderr"
)]
fn skip_note() {
    eprintln!(
        "note: sd-0.5.15 reference unavailable (fixtures/sd-reference not \
         provisioned, or git missing) — command differential skipped; \
         provision with `bun install` in crates/seeds/tests/fixtures/sd-reference"
    );
}

// ---------------------------------------------------------------------------
// fixture pair + capture
// ---------------------------------------------------------------------------

/// One per-case tracker pair: identical `.seeds/` stores, one dir per
/// binary. Both dirs are git repos (the reference expects a git
/// worktree; identical environments rule out environment skew).
struct Pair {
    reference_dir: PathBuf,
    ours_dir:      PathBuf,
}

fn materialize(tag: &str, side: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "seeds-differential-{tag}-{side}-{}-{unique}",
        std::process::id()
    ));
    let seeds = dir.join(".seeds");
    fs::create_dir_all(&seeds).expect("create temp .seeds dir");
    fs::write(seeds.join("config.yaml"), CONFIG_YAML).expect("config.yaml");
    fs::write(seeds.join("issues.jsonl"), issues_jsonl()).expect("issues.jsonl");
    fs::write(seeds.join("plans.jsonl"), "").expect("plans.jsonl");
    fs::write(seeds.join("templates.jsonl"), "").expect("templates.jsonl");
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .output()
        .expect("spawn git");
    assert!(
        git.status.success(),
        "git init failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&git.stderr)
    );
    dir
}

fn fixture_pair(tag: &str) -> Pair {
    Pair {
        reference_dir: materialize(tag, "sd"),
        ours_dir:      materialize(tag, "ours"),
    }
}

fn our_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_seeds"))
}

fn capture(dir: &Path, program: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|error| panic!("spawn {} {args:?}: {error}", program.display()))
}

// ---------------------------------------------------------------------------
// normalization + comparison
// ---------------------------------------------------------------------------

/// Recursively replaces volatile-field values with a placeholder so
/// same-moment runs compare equal wherever JSON is compared.
fn normalize_volatile(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if VOLATILE_FIELDS.contains(&key.as_str()) {
                    *inner = Value::String("<volatile>".to_owned());
                } else {
                    normalize_volatile(inner);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_volatile(item);
            }
        }
        _ => {}
    }
}

fn parse_json(text: &str) -> Option<Value> {
    serde_json::from_str(text.trim()).ok()
}

fn store_records(dir: &Path) -> Vec<Value> {
    let text = fs::read_to_string(dir.join(".seeds/issues.jsonl")).expect("issues.jsonl");
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("valid JSONL record"))
        .collect()
}

/// Compares exit code, stdout, and stderr-emptiness class for one argv.
fn assert_output_parity(case: &str, sd: &Output, ours: &Output) {
    assert_eq!(
        sd.status.code(),
        ours.status.code(),
        "{case}: exit code diverged (sd {:?}, ours {:?})\nsd stdout: {}\nours \
         stdout: {}\nsd stderr: {}\nours stderr: {}",
        sd.status.code(),
        ours.status.code(),
        String::from_utf8_lossy(&sd.stdout),
        String::from_utf8_lossy(&ours.stdout),
        String::from_utf8_lossy(&sd.stderr),
        String::from_utf8_lossy(&ours.stderr),
    );
    assert_eq!(
        sd.stderr.is_empty(),
        ours.stderr.is_empty(),
        "{case}: stderr emptiness diverged\nsd stderr: {}\nours stderr: {}",
        String::from_utf8_lossy(&sd.stderr),
        String::from_utf8_lossy(&ours.stderr),
    );
    let sd_text = String::from_utf8_lossy(&sd.stdout);
    let ours_text = String::from_utf8_lossy(&ours.stdout);
    match (parse_json(&sd_text), parse_json(&ours_text)) {
        (Some(mut sd_value), Some(mut mut_ours)) => {
            normalize_volatile(&mut sd_value);
            normalize_volatile(&mut mut_ours);
            assert_eq!(sd_value, mut_ours, "{case}: stdout JSON diverged");
        }
        _ => {
            assert_eq!(
                sd_text.trim(),
                ours_text.trim(),
                "{case}: stdout text diverged"
            );
        }
    }
}

/// Compares the resulting tracker files: same record sequence, equal
/// field-for-field modulo volatile timestamps.
fn assert_store_parity(case: &str, pair: &Pair) {
    let mut sd_records = store_records(&pair.reference_dir);
    let mut ours_records = store_records(&pair.ours_dir);
    for record in &mut sd_records {
        normalize_volatile(record);
    }
    for record in &mut ours_records {
        normalize_volatile(record);
    }
    assert_eq!(
        sd_records, ours_records,
        "{case}: resulting issues.jsonl diverged"
    );
}

/// Runs one argv through both binaries against fresh fixture pairs and
/// asserts full parity: exit code, stdout, and the written store.
fn assert_command_parity(reference: &Path, case: &str, args: &[&str]) {
    let pair = fixture_pair(case);
    let sd = capture(&pair.reference_dir, reference, args);
    let ours = capture(&pair.ours_dir, &our_binary(), args);
    assert_output_parity(case, &sd, &ours);
    assert_store_parity(case, &pair);
    fs::remove_dir_all(&pair.reference_dir).ok();
    fs::remove_dir_all(&pair.ours_dir).ok();
}

// ---------------------------------------------------------------------------
// the matrix
// ---------------------------------------------------------------------------

/// One generated comparison case: `command` drives the coverage guard,
/// `args` runs identically through both binaries.
struct Case {
    name:    &'static str,
    command: &'static str,
    args:    &'static [&'static str],
}

/// The differential matrix. Deliberate divergences do NOT belong here
/// — they get their own tailored tests and a DEVIATIONS entry in the
/// README's CLI surface contract.
fn matrix() -> Vec<Case> {
    vec![
        // list: filters, limits, format variants
        Case {
            name:    "list_default",
            command: "list",
            args:    &["list", "--format", "json"],
        },
        Case {
            name:    "list_assignee",
            command: "list",
            args:    &[
                "list",
                "--format",
                "json",
                "--assignee",
                "alice",
                "--sort",
                "id",
            ],
        },
        Case {
            name:    "list_status",
            command: "list",
            args:    &[
                "list",
                "--format",
                "json",
                "--status",
                "in_progress",
                "--sort",
                "id",
            ],
        },
        Case {
            name:    "list_limit",
            command: "list",
            args:    &["list", "--format", "json", "--limit", "2"],
        },
        Case {
            name:    "list_ids_format",
            command: "list",
            args:    &["list", "--format", "ids", "--sort", "id"],
        },
        Case {
            name:    "list_compact_format",
            command: "list",
            args:    &["list", "--format", "compact", "--sort", "id"],
        },
        Case {
            name:    "list_all_includes_closed",
            command: "list",
            args:    &["list", "--all", "--format", "json", "--sort", "id"],
        },
        // ready
        Case {
            name:    "ready_default",
            command: "ready",
            args:    &["ready", "--format", "json"],
        },
        Case {
            name:    "ready_assignee_and_limit",
            command: "ready",
            args:    &[
                "ready",
                "--assignee",
                "alice",
                "--limit",
                "1",
                "--format",
                "json",
            ],
        },
        // show: single, multi, error path
        Case {
            name:    "show_single_json",
            command: "show",
            args:    &["show", "tst-0001", "--format", "json"],
        },
        Case {
            name:    "show_multi_json",
            command: "show",
            args:    &["show", "tst-0001", "tst-0002", "--json"],
        },
        Case {
            name:    "show_multi_partial_json_flag",
            command: "show",
            args:    &["show", "tst-0001", "tst-zzzz", "--json"],
        },
        Case {
            name:    "show_multi_partial_format_json",
            command: "show",
            args:    &["show", "tst-0001", "tst-zzzz", "--format", "json"],
        },
        Case {
            name:    "show_missing_id_error",
            command: "show",
            args:    &["show", "tst-zzzz", "--format", "json"],
        },
        Case {
            name:    "show_missing_id_json_flag",
            command: "show",
            args:    &["show", "tst-zzzz", "--json"],
        },
        Case {
            name:    "show_no_args_error",
            command: "show",
            args:    &["show", "--json"],
        },
        // search: hit, miss, missing-argument error
        Case {
            name:    "search_hit",
            command: "search",
            args:    &["search", "parity", "--format", "json"],
        },
        Case {
            name:    "search_no_hit",
            command: "search",
            args:    &["search", "zzz-nothing", "--format", "json"],
        },
        Case {
            name:    "search_missing_query_error",
            command: "search",
            args:    &["search", "--format", "json"],
        },
        // update: status, assignee, add-label, error path
        Case {
            name:    "update_status",
            command: "update",
            args:    &["update", "tst-0002", "--status", "in_progress", "--json"],
        },
        Case {
            name:    "update_assignee",
            command: "update",
            args:    &["update", "tst-0002", "--assignee", "carol", "--json"],
        },
        Case {
            name:    "update_add_label",
            command: "update",
            args:    &["update", "tst-0003", "--add-label", "x,y", "--json"],
        },
        Case {
            name:    "update_missing_id_error",
            command: "update",
            args:    &["update", "tst-zzzz", "--title", "x", "--json"],
        },
        // close: with reason, closing a blocker, error path
        Case {
            name:    "close_with_reason",
            command: "close",
            args:    &["close", "tst-0002", "--reason", "done here", "--json"],
        },
        Case {
            name:    "close_blocker",
            command: "close",
            args:    &["close", "tst-0001", "--json"],
        },
        Case {
            name:    "close_missing_id_error",
            command: "close",
            args:    &["close", "tst-zzzz", "--json"],
        },
        // dep add: happy path, error path
        Case {
            name:    "dep_add",
            command: "dep",
            args:    &["dep", "add", "tst-0002", "tst-0003", "--json"],
        },
        Case {
            name:    "dep_add_missing_id_error",
            command: "dep",
            args:    &["dep", "add", "tst-zzzz", "tst-0001", "--json"],
        },
        // prime: static templates (exact text) and JSON
        Case {
            name:    "prime_default",
            command: "prime",
            args:    &["prime"],
        },
        Case {
            name:    "prime_json",
            command: "prime",
            args:    &["prime", "--json"],
        },
        Case {
            name:    "prime_compact_json",
            command: "prime",
            args:    &["prime", "--compact", "--json"],
        },
        // blocked: default, formats, JSON envelope (fixture:
        // tst-0006 blocked by open tst-0001; tst-0005's closed
        // blocker resolves and stays out).
        Case {
            name:    "blocked_default",
            command: "blocked",
            args:    &["blocked"],
        },
        Case {
            name:    "blocked_compact",
            command: "blocked",
            args:    &["blocked", "--format", "compact"],
        },
        Case {
            name:    "blocked_ids",
            command: "blocked",
            args:    &["blocked", "--format", "ids"],
        },
        Case {
            name:    "blocked_json",
            command: "blocked",
            args:    &["blocked", "--json"],
        },
        // block / unblock: happy paths, idempotence, error paths
        Case {
            name:    "block_adds_blocker",
            command: "block",
            args:    &["block", "tst-0002", "--by", "tst-0004"],
        },
        Case {
            name:    "block_json",
            command: "block",
            args:    &["block", "tst-0002", "--by", "tst-0004", "--json"],
        },
        Case {
            name:    "block_missing_id_error",
            command: "block",
            args:    &["block", "tst-zzzz", "--by", "tst-0001", "--json"],
        },
        Case {
            name:    "unblock_from",
            command: "unblock",
            args:    &["unblock", "tst-0006", "--from", "tst-0001"],
        },
        Case {
            name:    "unblock_from_json",
            command: "unblock",
            args:    &["unblock", "tst-0006", "--from", "tst-0001", "--json"],
        },
        Case {
            name:    "unblock_not_blocked_error",
            command: "unblock",
            args:    &["unblock", "tst-0006", "--from", "tst-0004", "--json"],
        },
        Case {
            name:    "unblock_all_no_closed",
            command: "unblock",
            args:    &["unblock", "tst-0006", "--all"],
        },
        // label: add/remove/list/list-all, error paths
        Case {
            name:    "label_add",
            command: "label",
            args:    &["label", "add", "tst-0002", "alpha", "beta"],
        },
        Case {
            name:    "label_add_json",
            command: "label",
            args:    &["label", "add", "tst-0002", "alpha", "--json"],
        },
        Case {
            name:    "label_add_missing_error",
            command: "label",
            args:    &["label", "add", "tst-zzzz", "x", "--json"],
        },
        Case {
            name:    "label_remove",
            command: "label",
            args:    &["label", "remove", "tst-0001", "bug"],
        },
        Case {
            name:    "label_remove_all",
            command: "label",
            args:    &["label", "remove", "tst-0001", "bug", "ui"],
        },
        Case {
            name:    "label_remove_json",
            command: "label",
            args:    &["label", "remove", "tst-0001", "bug", "--json"],
        },
        Case {
            name:    "label_list",
            command: "label",
            args:    &["label", "list", "tst-0001"],
        },
        Case {
            name:    "label_list_empty",
            command: "label",
            args:    &["label", "list", "tst-0002"],
        },
        Case {
            name:    "label_list_json",
            command: "label",
            args:    &["label", "list", "tst-0001", "--json"],
        },
        Case {
            name:    "label_list_all",
            command: "label",
            args:    &["label", "list-all"],
        },
        Case {
            name:    "label_list_all_json",
            command: "label",
            args:    &["label", "list-all", "--json"],
        },
        // dep remove / dep list (dep add is covered above)
        Case {
            name:    "dep_remove",
            command: "dep",
            args:    &["dep", "remove", "tst-0006", "tst-0001"],
        },
        Case {
            name:    "dep_remove_json",
            command: "dep",
            args:    &["dep", "remove", "tst-0006", "tst-0001", "--json"],
        },
        Case {
            name:    "dep_remove_missing_error",
            command: "dep",
            args:    &["dep", "remove", "tst-zzzz", "tst-0001", "--json"],
        },
        Case {
            name:    "dep_list",
            command: "dep",
            args:    &["dep", "list", "tst-0006"],
        },
        Case {
            name:    "dep_list_no_deps",
            command: "dep",
            args:    &["dep", "list", "tst-0002"],
        },
        Case {
            name:    "dep_list_json",
            command: "dep",
            args:    &["dep", "list", "tst-0006", "--json"],
        },
        Case {
            name:    "dep_list_missing_error",
            command: "dep",
            args:    &["dep", "list", "tst-zzzz", "--json"],
        },
        // stats: text and JSON (stable keys, encounter-order groups)
        Case {
            name:    "stats_default",
            command: "stats",
            args:    &["stats"],
        },
        Case {
            name:    "stats_json",
            command: "stats",
            args:    &["stats", "--json"],
        },
        Case {
            name:    "stats_format_json",
            command: "stats",
            args:    &["stats", "--format", "json"],
        },
        // doctor: check surface, JSON envelope, exit code (fixture
        // carries one bidirectional mismatch plus the missing
        // .gitattributes warning)
        Case {
            name:    "doctor_default",
            command: "doctor",
            args:    &["doctor"],
        },
        Case {
            name:    "doctor_json",
            command: "doctor",
            args:    &["doctor", "--json"],
        },
        // plan: read-only surface on the empty-plan fixture (the
        // mutating subcommands live in the tailored plan test)
        Case {
            name:    "plan_templates",
            command: "plan",
            args:    &["plan", "templates"],
        },
        Case {
            name:    "plan_templates_json",
            command: "plan",
            args:    &["plan", "templates", "--json"],
        },
        Case {
            name:    "plan_list_empty",
            command: "plan",
            args:    &["plan", "list"],
        },
        Case {
            name:    "plan_list_empty_json",
            command: "plan",
            args:    &["plan", "list", "--json"],
        },
        Case {
            name:    "plan_list_bad_status",
            command: "plan",
            args:    &["plan", "list", "--status", "nope"],
        },
        Case {
            name:    "plan_prompt_bug_json",
            command: "plan",
            args:    &["plan", "prompt", "tst-0001", "--json"],
        },
        Case {
            name:    "plan_prompt_human",
            command: "plan",
            args:    &["plan", "prompt", "tst-0001"],
        },
        Case {
            name:    "plan_show_seed_without_plan",
            command: "plan",
            args:    &["plan", "show", "tst-0002"],
        },
        Case {
            name:    "plan_validate_missing_plan",
            command: "plan",
            args:    &["plan", "validate", "pl-zzzz"],
        },
        // config group: schema emit, show/read, set/unset writes
        Case {
            name:    "config_schema",
            command: "config",
            args:    &["config", "schema"],
        },
        Case {
            name:    "config_schema_compact",
            command: "config",
            args:    &["config", "schema", "--json"],
        },
        Case {
            name:    "config_show",
            command: "config",
            args:    &["config", "show"],
        },
        Case {
            name:    "config_show_json",
            command: "config",
            args:    &["config", "show", "--json"],
        },
        Case {
            name:    "config_show_path_scalar",
            command: "config",
            args:    &["config", "show", "--path", "project"],
        },
        Case {
            name:    "config_show_path_missing",
            command: "config",
            args:    &["config", "show", "--path", "nope"],
        },
        Case {
            name:    "config_show_path_missing_json",
            command: "config",
            args:    &["config", "show", "--path", "nope", "--json"],
        },
        Case {
            name:    "config_set",
            command: "config",
            args:    &["config", "set", "max_plan_depth", "5"],
        },
        Case {
            name:    "config_set_json",
            command: "config",
            args:    &["config", "set", "--json", "project", "demo"],
        },
        Case {
            name:    "config_set_type_error",
            command: "config",
            args:    &["config", "set", "max_plan_depth", "notanint"],
        },
        Case {
            name:    "config_set_unknown_key",
            command: "config",
            args:    &["config", "set", "ghost", "1"],
        },
        Case {
            name:    "config_unset",
            command: "config",
            args:    &["config", "unset", "max_plan_depth"],
        },
        Case {
            name:    "config_unset_missing",
            command: "config",
            args:    &["config", "unset", "ghost"],
        },
        Case {
            name:    "config_unset_missing_json",
            command: "config",
            args:    &["config", "unset", "--json", "plan_templates"],
        },
        // onboard: the generic check surface coincides byte-for-byte;
        // the section CONTENT is a documented deviation (README
        // DEVIATIONS) and is covered by cli.rs instead.
        Case {
            name:    "onboard_check_missing",
            command: "onboard",
            args:    &["onboard", "--check"],
        },
        Case {
            name:    "onboard_check_missing_json",
            command: "onboard",
            args:    &["onboard", "--check", "--json"],
        },
        // completions: error surface coincides; the scripts enumerate
        // the implemented surface (deviation), covered by cli.rs.
        Case {
            name:    "completions_unknown_shell",
            command: "completions",
            args:    &["completions", "tcsh"],
        },
        Case {
            name:    "completions_missing_shell",
            command: "completions",
            args:    &["completions"],
        },
    ]
}

#[test]
fn matrix_covers_every_implemented_command() {
    // The convention guard: a command on IMPLEMENTED_COMMANDS without
    // differential coverage fails here, so every implemented (and every
    // future) command carries live differential coverage.
    // Commands whose comparison is TAILORED (not a plain matrix case)
    // are named here with their test:
    const TAILORED: &[(&str, &str)] = &[
        (
            // Random `<project>-<hex4>` ids and `now` stamps make a plain
            // argv comparison meaningless — the tailored test normalizes.
            "create",
            "differential_create_matches_sd",
        ),
        (
            // sync mutates git history, needs a committer identity, and
            // carries one documented deviation (untracked-dir expansion)
            // — the tailored test pins parity per scenario.
            "sync",
            "differential_sync_matches_sd",
        ),
        (
            // Random pl-/tst- child ids and `now` stamps make a plain
            // argv comparison meaningless — the tailored test runs a
            // full plan lifecycle sequence with ids normalized.
            "plan",
            "differential_plan_matches_sd",
        ),
        (
            // init's output embeds the absolute temp path (one dir per
            // binary) — the tailored test normalizes the dir prefix and
            // compares the bootstrapped .seeds/ tree byte-for-byte.
            "init",
            "differential_init_matches_sd",
        ),
    ];
    let cases = matrix();
    for command in IMPLEMENTED_COMMANDS {
        let in_matrix = cases.iter().any(|case| case.command == *command);
        let tailored = TAILORED.iter().any(|(name, _)| name == command);
        assert!(
            in_matrix || tailored,
            "implemented command '{command}' has no differential case — \
             add one (matrix or tailored)"
        );
    }
    for case in &cases {
        assert!(
            IMPLEMENTED_COMMANDS.contains(&case.command),
            "case '{}' claims command '{}' not on IMPLEMENTED_COMMANDS",
            case.name,
            case.command
        );
    }
}

#[test]
fn differential_command_matrix_matches_sd() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };
    for case in matrix() {
        assert_command_parity(&reference, case.name, case.args);
    }
}

/// `create` tailors the comparison instead of the plain matrix: both
/// binaries mint a fresh random `<project>-<hex4>` id and stamp `now`,
/// so the case compares the envelopes structurally and the ONE new
/// store record per side with its id normalized.
#[test]
fn differential_create_matches_sd() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };
    let args = [
        "create",
        "--title",
        "New thing",
        "--type",
        "bug",
        "--priority",
        "1",
        "--labels",
        "x,y",
        "--assignee",
        "alice",
        "--description",
        "body",
        "--json",
    ];
    let pair = fixture_pair("create");
    let sd = capture(&pair.reference_dir, &reference, &args);
    let ours = capture(&pair.ours_dir, &our_binary(), &args);
    assert_eq!(
        sd.status.code(),
        ours.status.code(),
        "create: exit code diverged"
    );

    let sd_value = parse_json(&String::from_utf8_lossy(&sd.stdout)).expect("sd create emits JSON");
    let ours_value =
        parse_json(&String::from_utf8_lossy(&ours.stdout)).expect("seeds create emits JSON");
    assert_eq!(sd_value["success"], ours_value["success"]);
    assert_eq!(sd_value["command"], ours_value["command"]);

    assert_eq!(
        new_record_normalized(&pair.reference_dir),
        new_record_normalized(&pair.ours_dir),
        "create: the appended record diverged"
    );
    fs::remove_dir_all(&pair.reference_dir).ok();
    fs::remove_dir_all(&pair.ours_dir).ok();
}

/// The one record a create appended, id normalized to `<new>`.
fn new_record_normalized(dir: &Path) -> Value {
    let mut record = store_records(dir)
        .into_iter()
        .find(|record| {
            record["id"]
                .as_str()
                .is_some_and(|id| !FIXTURE_IDS.contains(&id))
        })
        .expect("exactly one new record");
    let id = record["id"].as_str().expect("id").to_owned();
    let suffix = id.rsplit_once('-').expect("project-prefixed id").1;
    assert_eq!(suffix.len(), 4, "fresh hex4 id: {id}");
    normalize_volatile(&mut record);
    record["id"] = Value::String("<new>".to_owned());
    record
}

/// `init` tailors the comparison: the output embeds each side's
/// absolute temp dir, and the bootstrapped config.yaml names the dir.
/// Both sides get identically-named working dirs, so normalizing the
/// parent prefix makes stdout comparable and the `.seeds/` tree
/// byte-comparable — fresh init, idempotent re-init, and `--json`.
#[test]
fn differential_init_matches_sd() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };
    let make_side = |side: &str| -> PathBuf {
        let parent = std::env::temp_dir().join(format!(
            "seeds-differential-init-{side}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&parent);
        let dir = parent.join("initcase");
        fs::create_dir_all(&dir).expect("create temp working dir");
        let git = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&dir)
            .output()
            .expect("spawn git");
        assert!(git.status.success(), "git init failed in {}", dir.display());
        dir
    };
    let sd_dir = make_side("sd");
    let ours_dir = make_side("ours");

    for args in [vec!["init"], vec!["init"], vec!["init", "--json"]] {
        let case = args.join(" ");
        let sd = capture(&sd_dir, &reference, &args);
        let ours = capture(&ours_dir, &our_binary(), &args);
        assert_eq!(
            sd.status.code(),
            ours.status.code(),
            "{case}: exit code diverged"
        );
        assert_eq!(
            sd.stderr.is_empty(),
            ours.stderr.is_empty(),
            "{case}: stderr emptiness diverged"
        );
        let normalize =
            |text: &str, dir: &Path| text.replace(dir.to_string_lossy().as_ref(), "<dir>");
        assert_eq!(
            normalize(&String::from_utf8_lossy(&sd.stdout), &sd_dir),
            normalize(&String::from_utf8_lossy(&ours.stdout), &ours_dir),
            "{case}: stdout diverged after dir normalization"
        );
    }

    for file in [
        ".gitignore",
        "config.yaml",
        "issues.jsonl",
        "plans.jsonl",
        "templates.jsonl",
    ] {
        let sd_file = fs::read(sd_dir.join(".seeds").join(file)).expect("sd .seeds/{file}");
        let ours_file = fs::read(ours_dir.join(".seeds").join(file)).expect("ours .seeds/{file}");
        assert_eq!(sd_file, ours_file, "init: .seeds/{file} diverged");
    }
    fs::remove_dir_all(&sd_dir).ok();
    fs::remove_dir_all(&ours_dir).ok();
}

/// `sync` tailors the comparison (seeds-540e): it mutates git history,
/// needs a committer identity, and carries one documented deviation —
/// untracked directories are expanded to per-file entries. This test
/// pins parity per scenario: commit path (stdout, store, commit
/// subject), no-op path, `--status`/`--dry-run` on tracked changes,
/// JSON envelopes, and the not-in-a-project error; the untracked-dir
/// expansion is asserted as the deliberate deviation.
#[test]
fn differential_sync_matches_sd() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };
    let pair = fixture_pair("sync");
    for dir in [&pair.reference_dir, &pair.ours_dir] {
        for (key, value) in [
            ("user.email", "diff@t.local"),
            ("user.name", "Diff Battery"),
        ] {
            let ok = Command::new("git")
                .args(["config", key, value])
                .current_dir(dir)
                .output()
                .expect("spawn git config")
                .status
                .success();
            assert!(ok, "git config {key} failed in {}", dir.display());
        }
    }

    // Untracked store: the deliberate per-file preview deviation —
    // sd collapses to `?? .seeds/`, ours lists every file.
    let sd_status = capture(&pair.reference_dir, &reference, &["sync", "--status"]);
    let ours_status = capture(&pair.ours_dir, &our_binary(), &["sync", "--status"]);
    assert_eq!(sd_status.status.code(), ours_status.status.code());
    let sd_text = String::from_utf8_lossy(&sd_status.stdout);
    let ours_text = String::from_utf8_lossy(&ours_status.stdout);
    assert!(
        sd_text.contains("?? .seeds/"),
        "sd collapses untracked dirs: {sd_text}"
    );
    assert!(
        ours_text.contains(".seeds/config.yaml") && ours_text.contains(".seeds/issues.jsonl"),
        "ours expands untracked dirs per file: {ours_text}"
    );

    // Commit path: identical stdout, exit code, store, and subject.
    let sd = capture(&pair.reference_dir, &reference, &["sync"]);
    let ours = capture(&pair.ours_dir, &our_binary(), &["sync"]);
    assert_output_parity("sync_commit", &sd, &ours);
    assert_store_parity("sync_commit", &pair);
    assert_eq!(
        commit_subject(&pair.reference_dir),
        commit_subject(&pair.ours_dir)
    );

    // No-op path (clean tree) and its JSON envelope.
    for args in [
        vec!["sync"],
        vec!["sync", "--dry-run"],
        vec!["sync", "--status"],
        vec!["sync", "--json"],
    ] {
        let sd = capture(&pair.reference_dir, &reference, &args);
        let ours = capture(&pair.ours_dir, &our_binary(), &args);
        assert_output_parity(&format!("sync_clean_{}", args.join("_")), &sd, &ours);
    }

    // Tracked modification: per-file listings are byte-identical here
    // (no untracked-dir collapsing involved) — full parity holds for
    // --status, --dry-run, and the JSON commit envelope.
    let extra = fixture_record("tst-0007", "Late addition", serde_json::json!({})).to_string();
    for dir in [&pair.reference_dir, &pair.ours_dir] {
        let path = dir.join(".seeds/issues.jsonl");
        let mut text = fs::read_to_string(&path).expect("issues.jsonl");
        text.push_str(&extra);
        text.push('\n');
        fs::write(path, text).expect("append record");
    }
    for args in [
        vec!["sync", "--status"],
        vec!["sync", "--status", "--json"],
        vec!["sync", "--dry-run"],
        vec!["sync", "--json"],
    ] {
        let sd = capture(&pair.reference_dir, &reference, &args);
        let ours = capture(&pair.ours_dir, &our_binary(), &args);
        assert_output_parity(&format!("sync_dirty_{}", args.join("_")), &sd, &ours);
    }
    assert_store_parity("sync_dirty", &pair);
    assert_eq!(
        commit_subject(&pair.reference_dir),
        commit_subject(&pair.ours_dir)
    );

    // Not-in-a-seeds-project error parity (fresh dirs, no .seeds).
    let bare = std::env::temp_dir().join(format!(
        "seeds-differential-sync-bare-{}",
        std::process::id()
    ));
    fs::create_dir_all(&bare).expect("bare dir");
    let sd = capture(&bare, &reference, &["sync"]);
    let ours = capture(&bare, &our_binary(), &["sync"]);
    assert_eq!(sd.status.code(), ours.status.code());
    assert_eq!(
        String::from_utf8_lossy(&sd.stderr).trim(),
        String::from_utf8_lossy(&ours.stderr).trim(),
        "not-in-a-project error diverged"
    );
    fs::remove_dir_all(&bare).ok();
    fs::remove_dir_all(&pair.reference_dir).ok();
    fs::remove_dir_all(&pair.ours_dir).ok();
}

/// The HEAD commit subject in `dir`.
fn commit_subject(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%s"])
        .current_dir(dir)
        .output()
        .expect("spawn git log");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

// ---------------------------------------------------------------------------
// plan lifecycle (tailored): the mutating subcommands mint random
// pl-<hex4> plan ids and tst-<hex4> child ids and stamp `now`, so the
// comparison normalizes both (first-appearance order) exactly like the
// create case normalizes its single fresh id.
// ---------------------------------------------------------------------------

/// Replaces every non-fixture seed id and every plan id with a
/// first-appearance placeholder (`<child-N>` / `<plan-N>`), recursively.
fn normalize_ids(value: &mut Value, child_ids: &mut Vec<String>, plan_ids: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (_, inner) in map {
                normalize_ids(inner, child_ids, plan_ids);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_ids(item, child_ids, plan_ids);
            }
        }
        Value::String(text) => {
            let is_seed = text.len() == 8
                && text.starts_with("tst-")
                && !FIXTURE_IDS.contains(&text.as_str())
                && text[4..]
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
            let is_plan = text.len() == 7 && text.starts_with("pl-") && {
                let hex = &text[3..];
                !hex.is_empty()
                    && hex.len() == 4
                    && hex
                        .chars()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            };
            if is_seed {
                let slot = slot_of(child_ids, text);
                *text = format!("<child-{slot}>");
            } else if is_plan {
                let slot = slot_of(plan_ids, text);
                *text = format!("<plan-{slot}>");
            } else {
                // Embedded ids (backref descriptions) normalize by
                // substring scan, first-appearance order.
                *text = replace_embedded_plans(text, plan_ids);
            }
        }
        _ => {}
    }
}

/// Replaces every embedded `pl-<hex4>` token with its
/// first-appearance placeholder.
fn replace_embedded_plans(text: &str, plan_ids: &mut Vec<String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(pos) = rest.find("pl-") {
        let after = &rest[pos + 3..];
        let bytes = after.as_bytes();
        if bytes.len() >= 4
            && bytes[..4]
                .iter()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            let hex = std::str::from_utf8(&bytes[..4]).expect("ascii hex");
            let slot = slot_of(plan_ids, &format!("pl-{hex}"));
            out.push_str(&rest[..pos]);
            out.push_str(&format!("<plan-{slot}>"));
            rest = &after[4..];
        } else {
            out.push_str(&rest[..pos + 3]);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// The first-appearance slot of `id`, appending it when new.
fn slot_of(ids: &mut Vec<String>, id: &str) -> usize {
    ids.iter().position(|known| known == id).unwrap_or_else(|| {
        ids.push(id.to_owned());
        ids.len() - 1
    })
}

/// Normalizes one parsed JSON value: volatile fields + fresh ids.
fn normalized_json(
    value: &Value,
    child_ids: &mut Vec<String>,
    plan_ids: &mut Vec<String>,
) -> Value {
    let mut out = value.clone();
    normalize_volatile(&mut out);
    normalize_ids(&mut out, child_ids, plan_ids);
    out
}

/// Normalizes human (non-JSON) plan output: fresh ids and ISO
/// timestamps become placeholders, in first-appearance order.
fn normalized_text(text: &str, child_ids: &mut Vec<String>, plan_ids: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut buffer = String::new();
    for token in text.split(|c: char| c.is_whitespace()) {
        if buffer.is_empty() {
            buffer.push_str(token);
        }
        let mut value = Value::String(buffer.clone());
        normalize_ids(&mut value, child_ids, plan_ids);
        let normalized = value.as_str().unwrap_or_default().to_owned();
        // Timestamp-shaped tokens collapse wholesale.
        let normalized = if normalized.len() == 24
            && normalized.ends_with('Z')
            && normalized.as_bytes()[4] == b'-'
            && normalized.as_bytes()[10] == b'T'
        {
            "<volatile>".to_owned()
        } else {
            normalized
        };
        out.push_str(&normalized);
        out.push(' ');
        buffer.clear();
    }
    out.trim().to_owned()
}

/// `plan` tailors the comparison: a full lifecycle sequence (submit,
/// show, list, outcome, review, edit, invalid submit, overwrite,
/// create/adopt/reorder/release) runs through both binaries; every
/// step's exit code and stdout JSON are compared with volatile
/// timestamps and freshly minted ids normalized, then the final
/// issues.jsonl + plans.jsonl stores are compared the same way.
#[test]
fn differential_plan_matches_sd() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };
    let pair = fixture_pair("plan");
    let context = "c".repeat(60);
    let plan_json = serde_json::json!({
        "template": "feature",
        "sections": {
            "context": context,
            "approach": "Old approach.",
            "steps": [
                {"title": "Step one"},
                {"title": "Step two", "type": "bug", "priority": 1, "blocks": [1]},
            ],
            "acceptance": ["it works"],
        },
    });
    let rewrite_json = serde_json::json!({
        "template": "feature",
        "sections": {
            "context": context,
            "approach": "Rewritten approach.",
            "steps": [
                {"title": "Step one"},
                {"title": "Brand new"},
            ],
            "acceptance": ["ok"],
        },
    });
    let bad_json = serde_json::json!({
        "template": "feature",
        "sections": {"context": "short", "steps": [{"title": "a"}], "acceptance": []},
    });
    for dir in [&pair.reference_dir, &pair.ours_dir] {
        std::fs::write(dir.join("plan.json"), format!("{plan_json}")).expect("write plan.json");
        std::fs::write(dir.join("rewrite.json"), format!("{rewrite_json}"))
            .expect("write rewrite.json");
        std::fs::write(dir.join("bad.json"), format!("{bad_json}")).expect("write bad.json");
    }

    let sequence: Vec<Vec<&str>> = vec![
        vec![
            "plan",
            "submit",
            "tst-0001",
            "--plan",
            "plan.json",
            "--json",
        ],
        vec!["plan", "show", "tst-0001", "--json"],
        vec!["plan", "show", "tst-0001"],
        vec!["plan", "list", "--json"],
        vec![
            "plan", "list", "--seed", "tst-0001", "--status", "approved", "--json",
        ],
        vec!["plan", "validate", "tst-0001", "--json"],
        vec![
            "plan", "outcome", "tst-0001", "--result", "partial", "--note", "n", "--json",
        ],
        vec!["plan", "review", "tst-0001", "--by", "alice", "--json"],
        vec!["plan", "edit", "tst-0001", "--name", "Renamed", "--json"],
        vec![
            "plan",
            "edit",
            "tst-0001",
            "--section",
            "approach",
            "New approach text.",
            "--json",
        ],
        vec![
            "plan",
            "edit",
            "tst-0001",
            "--step",
            "1",
            "--title",
            "S1",
            "--priority",
            "0",
            "--type",
            "bug",
            "--json",
        ],
        vec!["plan", "submit", "tst-0001", "--plan", "bad.json", "--json"],
        vec![
            "plan",
            "submit",
            "tst-0001",
            "--plan",
            "rewrite.json",
            "--overwrite",
            "--json",
        ],
        vec!["plan", "show", "tst-0001", "--json"],
        vec![
            "plan",
            "create",
            "tst-0002",
            "--name",
            "Adopt plan",
            "--json",
        ],
        vec!["plan", "adopt", "tst-0002", "tst-0005", "--json"],
        vec![
            "plan", "adopt", "tst-0002", "--before", "tst-0005", "tst-0003", "--json",
        ],
        vec![
            "plan", "reorder", "tst-0002", "tst-0003", "tst-0005", "--json",
        ],
        vec!["plan", "release", "tst-0002", "tst-0005", "--json"],
        vec!["plan", "list", "--json"],
    ];

    let mut sd_children: Vec<String> = Vec::new();
    let mut sd_plans: Vec<String> = Vec::new();
    let mut ours_children: Vec<String> = Vec::new();
    let mut ours_plans: Vec<String> = Vec::new();
    for args in &sequence {
        let case = format!("plan_{}", args.join("_"));
        let sd = capture(&pair.reference_dir, &reference, args);
        let ours = capture(&pair.ours_dir, &our_binary(), args);
        assert_eq!(
            sd.status.code(),
            ours.status.code(),
            "{case}: exit code diverged\nsd stderr: {}\nours stderr: {}",
            String::from_utf8_lossy(&sd.stderr),
            String::from_utf8_lossy(&ours.stderr),
        );
        let sd_text = String::from_utf8_lossy(&sd.stdout);
        let ours_text = String::from_utf8_lossy(&ours.stdout);
        match (parse_json(&sd_text), parse_json(&ours_text)) {
            (Some(sd_value), Some(ours_value)) => {
                assert_eq!(
                    normalized_json(&sd_value, &mut sd_children, &mut sd_plans),
                    normalized_json(&ours_value, &mut ours_children, &mut ours_plans),
                    "{case}: stdout JSON diverged"
                );
            }
            _ => {
                assert_eq!(
                    normalized_text(&sd_text, &mut sd_children, &mut sd_plans),
                    normalized_text(&ours_text, &mut ours_children, &mut ours_plans),
                    "{case}: stdout text diverged"
                );
            }
        }
    }

    // Final stores: same record sequence, normalized ids + timestamps.
    for file in ["issues.jsonl", "plans.jsonl"] {
        let read = |dir: &Path| -> Vec<Value> {
            let text = fs::read_to_string(dir.join(".seeds").join(file)).unwrap_or_default();
            text.lines()
                .filter(|line| !line.is_empty())
                .map(|line| serde_json::from_str(line).expect("valid JSONL record"))
                .map(|mut record| {
                    normalize_volatile(&mut record);
                    record
                })
                .collect()
        };
        let mut sd_records = read(&pair.reference_dir);
        let mut ours_records = read(&pair.ours_dir);
        for record in &mut sd_records {
            normalize_ids(record, &mut sd_children, &mut sd_plans);
        }
        for record in &mut ours_records {
            normalize_ids(record, &mut ours_children, &mut ours_plans);
        }
        assert_eq!(
            sd_records, ours_records,
            "plan lifecycle: resulting {file} diverged"
        );
    }
    fs::remove_dir_all(&pair.reference_dir).ok();
    fs::remove_dir_all(&pair.ours_dir).ok();
}
