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

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};

/// The implemented parity command surface — the matrix's coverage
/// driver. `create` is covered by the dedicated tailored case (random
/// ids make a plain argv comparison meaningless there).
const IMPLEMENTED_COMMANDS: &[&str] = &[
    "create", "show", "list", "ready", "search", "update", "close", "dep", "prime",
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
    ]
}

#[test]
fn matrix_covers_every_implemented_command() {
    // The convention guard: a command on IMPLEMENTED_COMMANDS without
    // differential coverage fails here, so every implemented (and every
    // future) command carries live differential coverage.
    // Commands whose comparison is TAILORED (not a plain matrix case)
    // are named here with their test:
    const TAILORED: &[(&str, &str)] = &[(
        // Random `<project>-<hex4>` ids and `now` stamps make a plain
        // argv comparison meaningless — the tailored test normalizes.
        "create",
        "differential_create_matches_sd",
    )];
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
