//! Round-trip compatibility suite against the reference implementation,
//! sd 0.5.15 (the README compat contract, ADR-0023 in denkhaus/fabro).
//!
//! Forward: our writer writes → the reference reads + updates → our
//! fields (and unknown additive fields) survive. Inverse: the reference
//! writes → we load and save byte-identically → it still reads the
//! store.
//!
//! The reference is NOT the PATH `sd`: this repo stopped installing the
//! sd CLI at the self-hosting cutover (seeds-3791). It lives in this
//! suite's fixtures — the wrapper `fixtures/sd-reference/sd` running
//! `@os-eco/seeds-cli@0.5.15` under bun. Provision it with `bun
//! install` inside that fixture directory (see its README.md); the
//! `SEEDS_COMPAT_SD` env var may point at an alternative reference
//! binary. When the reference (or `git`) is unavailable, every test
//! skips with a note, so plain `cargo nextest` runs stay green.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use seeds::{Fields, SeedRecord, SeedType, Status, Store};
use serde_json::{Value, json};

/// The sd-0.5.15 version the reference must report (`sd --version`).
const REFERENCE_VERSION: &str = "0.5.15";

/// Fixture wrapper path: `<crate>/tests/fixtures/sd-reference/sd`.
fn reference_wrapper() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sd-reference/sd")
}

/// Resolves the reference binary: the `SEEDS_COMPAT_SD` env override
/// when set, else the committed fixture wrapper. PATH `sd` is
/// deliberately not consulted — this repo no longer installs it.
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

/// The verified reference: resolved, answering `--version` successfully
/// as sd 0.5.15, with `git` available. `None` means unavailable — skip.
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
         provisioned, or git missing) — compat round-trip skipped; \
         provision with `bun install` in crates/seeds/tests/fixtures/sd-reference"
    );
}

fn temp_repo(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "seeds-compat-{tag}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp repo");
    dir
}

fn run(dir: &Path, program: &Path, args: &[&str]) -> Output {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|error| panic!("spawn {} {args:?}: {error}", program.display()));
    assert!(
        output.status.success(),
        "{} {args:?} failed ({:?}):\nstdout: {}\nstderr: {}",
        program.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn json_of(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("reference emits JSON on stdout")
}

fn init_repo(reference: &Path, tag: &str) -> PathBuf {
    let dir = temp_repo(tag);
    run(&dir, Path::new("git"), &["init", "-q"]);
    run(&dir, reference, &["init", "-q"]);
    dir
}

#[test]
fn forward_our_writer_sd_reads_and_updates() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };

    let dir = init_repo(&reference, "fwd");
    let seeds_dir = dir.join(".seeds");

    // We write the record ourselves, unknown field included.
    let store = Store::open(&seeds_dir).expect("open fresh store");
    let project = store.config.project.clone();
    let id = format!("{project}-a1b2");
    let mut fields = Fields::new();
    fields.insert("id".to_owned(), json!(id));
    fields.insert("title".to_owned(), json!("Forward compat"));
    fields.insert("status".to_owned(), json!("open"));
    fields.insert("type".to_owned(), json!("task"));
    fields.insert("priority".to_owned(), json!(2));
    fields.insert("createdAt".to_owned(), json!("2026-09-20T12:00:00.000Z"));
    fields.insert("updatedAt".to_owned(), json!("2026-09-20T12:00:00.000Z"));
    fields.insert("x_custom".to_owned(), json!({"keep": ["me"], "n": 7}));
    let record = SeedRecord::try_from_fields(fields).expect("valid record");
    let mut store = store;
    store.issues.push(record);
    store.save().expect("our writer saves");

    // The reference reads the store, updates a known field, rewrites.
    run(&dir, &reference, &["update", &id, "--priority", "0", "-q"]);
    let shown = json_of(&run(&dir, &reference, &["show", &id, "--format", "json"]));

    let issue = &shown["issue"];
    assert_eq!(issue["id"], json!(id), "the reference sees our id");
    assert_eq!(
        issue["title"],
        json!("Forward compat"),
        "the reference sees our title"
    );
    assert_eq!(
        issue["priority"],
        json!(0),
        "the reference applied its update"
    );
    assert_eq!(
        issue["x_custom"],
        json!({"keep": ["me"], "n": 7}),
        "the reference preserves our unknown field"
    );

    // And our reader still loads its rewrite with everything intact.
    let store = Store::open(&seeds_dir).expect("reopen after reference rewrite");
    let record = store.issue(&id).expect("record by id");
    assert_eq!(record.status(), Some(Status::Open));
    assert_eq!(record.seed_type(), Some(SeedType::Task));
    assert_eq!(
        record.field("x_custom"),
        Some(&json!({"keep": ["me"], "n": 7})),
        "unknown field survives the reference's rewrite"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn inverse_sd_writes_we_round_trip_byte_identically() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };

    let dir = init_repo(&reference, "inv");
    let seeds_dir = dir.join(".seeds");

    let created = json_of(&run(&dir, &reference, &[
        "create", "--title", "From sd", "--json",
    ]));
    let id = created["id"]
        .as_str()
        .expect("create returns id")
        .to_owned();
    run(&dir, &reference, &[
        "update",
        &id,
        "--description",
        "described",
        "-q",
    ]);
    let other = json_of(&run(&dir, &reference, &[
        "create", "--title", "Other", "--json",
    ]));
    let other_id = other["id"].as_str().expect("create returns id").to_owned();
    run(&dir, &reference, &["dep", "add", &id, &other_id, "--json"]);

    // Our turn: load and save with zero changes must be byte-identical.
    let issues_path = seeds_dir.join("issues.jsonl");
    let before = fs::read_to_string(&issues_path).expect("read reference output");
    let store = Store::open(&seeds_dir).expect("we read the reference's store");
    assert_eq!(store.issues.len(), 2, "we see the reference's records");
    assert_eq!(
        store.issue(&id).and_then(SeedRecord::description),
        Some("described")
    );
    store.save().expect("our writer saves");
    let after = fs::read_to_string(&issues_path).expect("read our output");
    assert_eq!(
        before, after,
        "load→save must be byte-identical for reference output"
    );

    // The reference still reads the store we just wrote.
    let listed = json_of(&run(&dir, &reference, &["list", "--format", "json"]));
    assert_eq!(listed["success"], json!(true));
    assert_eq!(listed["issues"].as_array().map(Vec::len), Some(2));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn forward_config_and_dependencies_agree_with_sd() {
    let Some(reference) = reference() else {
        skip_note();
        return;
    };

    // Dependencies are blockedBy/blocks arrays on the record (the format
    // contract); our model must expose exactly what the reference wrote.
    let dir = init_repo(&reference, "deps");
    let seeds_dir = dir.join(".seeds");
    let first = json_of(&run(&dir, &reference, &[
        "create", "--title", "A", "--json",
    ]));
    let second = json_of(&run(&dir, &reference, &[
        "create", "--title", "B", "--json",
    ]));
    let first_id = first["id"].as_str().expect("id").to_owned();
    let second_id = second["id"].as_str().expect("id").to_owned();
    run(&dir, &reference, &[
        "dep", "add", &first_id, &second_id, "--json",
    ]);

    let store = Store::open(&seeds_dir).expect("load store");
    let record = store.issue(&first_id).expect("record by id");
    assert_eq!(record.blocked_by(), vec![second_id.as_str()]);
    let record = store.issue(&second_id).expect("record by id");
    assert_eq!(record.blocks(), vec![first_id.as_str()]);

    // Config: our save stays readable by the reference.
    let mut store = Store::open(&seeds_dir).expect("reload store");
    store.config.max_plan_depth = 5;
    store.save().expect("save config");
    let shown = json_of(&run(&dir, &reference, &["config", "show", "--json"]));
    assert_eq!(
        shown["config"]["max_plan_depth"],
        json!(5),
        "the reference reads our config"
    );
    assert_eq!(shown["config"]["project"], json!(store.config.project));
    fs::remove_dir_all(&dir).ok();
}
