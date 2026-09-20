//! Round-trip compatibility suite against the reference implementation,
//! sd 0.5.15 (the README compat contract, ADR-0023 in denkhaus/fabro).
//!
//! Forward: our writer writes → `sd` reads + updates → our fields (and
//! unknown additive fields) survive. Inverse: `sd` writes → we load and
//! save byte-identically → `sd` still reads the store.
//!
//! The suite skips (pass-with-note) when `sd` or `git` is unavailable,
//! so plain `cargo nextest` runs outside the sandbox stay green.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use seeds::{Fields, SeedRecord, SeedType, Status, Store};
use serde_json::{Value, json};

fn have(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn skip_unless_sd_available() -> bool {
    have("sd") && have("git")
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

fn run(dir: &Path, program: &str, args: &[&str]) -> Output {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|error| panic!("spawn {program} {args:?}: {error}"));
    assert!(
        output.status.success(),
        "{program} {args:?} failed ({:?}):\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn json_of(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sd emits JSON on stdout")
}

fn init_repo(tag: &str) -> PathBuf {
    let dir = temp_repo(tag);
    run(&dir, "git", &["init", "-q"]);
    run(&dir, "sd", &["init", "-q"]);
    dir
}

#[test]
fn forward_our_writer_sd_reads_and_updates() {
    if !skip_unless_sd_available() {
        return;
    }

    let dir = init_repo("fwd");
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

    // sd reads the store, updates a known field, and rewrites the file.
    run(&dir, "sd", &["update", &id, "--priority", "0", "-q"]);
    let shown = json_of(&run(&dir, "sd", &["show", &id, "--format", "json"]));

    let issue = &shown["issue"];
    assert_eq!(issue["id"], json!(id), "sd sees our id");
    assert_eq!(issue["title"], json!("Forward compat"), "sd sees our title");
    assert_eq!(issue["priority"], json!(0), "sd applied its update");
    assert_eq!(
        issue["x_custom"],
        json!({"keep": ["me"], "n": 7}),
        "sd preserves our unknown field"
    );

    // And our reader still loads sd's rewrite with everything intact.
    let store = Store::open(&seeds_dir).expect("reopen after sd rewrite");
    let record = store.issue(&id).expect("record by id");
    assert_eq!(record.status(), Some(Status::Open));
    assert_eq!(record.seed_type(), Some(SeedType::Task));
    assert_eq!(
        record.field("x_custom"),
        Some(&json!({"keep": ["me"], "n": 7})),
        "unknown field survives sd's rewrite"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn inverse_sd_writes_we_round_trip_byte_identically() {
    if !skip_unless_sd_available() {
        return;
    }

    let dir = init_repo("inv");
    let seeds_dir = dir.join(".seeds");

    let created = json_of(&run(&dir, "sd", &[
        "create", "--title", "From sd", "--json",
    ]));
    let id = created["id"]
        .as_str()
        .expect("create returns id")
        .to_owned();
    run(&dir, "sd", &[
        "update",
        &id,
        "--description",
        "described",
        "-q",
    ]);
    let other = json_of(&run(&dir, "sd", &["create", "--title", "Other", "--json"]));
    let other_id = other["id"].as_str().expect("create returns id").to_owned();
    run(&dir, "sd", &["dep", "add", &id, &other_id, "--json"]);

    // Our turn: load and save with zero changes must be byte-identical.
    let issues_path = seeds_dir.join("issues.jsonl");
    let before = fs::read_to_string(&issues_path).expect("read sd output");
    let store = Store::open(&seeds_dir).expect("we read sd's store");
    assert_eq!(store.issues.len(), 2, "we see sd's records");
    assert_eq!(
        store.issue(&id).and_then(SeedRecord::description),
        Some("described")
    );
    store.save().expect("our writer saves");
    let after = fs::read_to_string(&issues_path).expect("read our output");
    assert_eq!(
        before, after,
        "load→save must be byte-identical for sd output"
    );

    // sd still reads the store we just wrote.
    let listed = json_of(&run(&dir, "sd", &["list", "--format", "json"]));
    assert_eq!(listed["success"], json!(true));
    assert_eq!(listed["issues"].as_array().map(Vec::len), Some(2));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn forward_config_and_dependencies_agree_with_sd() {
    if !skip_unless_sd_available() {
        return;
    }

    // Dependencies are blockedBy/blocks arrays on the record (the format
    // contract); our model must expose exactly what sd wrote.
    let dir = init_repo("deps");
    let seeds_dir = dir.join(".seeds");
    let first = json_of(&run(&dir, "sd", &["create", "--title", "A", "--json"]));
    let second = json_of(&run(&dir, "sd", &["create", "--title", "B", "--json"]));
    let first_id = first["id"].as_str().expect("id").to_owned();
    let second_id = second["id"].as_str().expect("id").to_owned();
    run(&dir, "sd", &["dep", "add", &first_id, &second_id, "--json"]);

    let store = Store::open(&seeds_dir).expect("load store");
    let record = store.issue(&first_id).expect("record by id");
    assert_eq!(record.blocked_by(), vec![second_id.as_str()]);
    let record = store.issue(&second_id).expect("record by id");
    assert_eq!(record.blocks(), vec![first_id.as_str()]);

    // Config: our save stays readable by sd.
    let mut store = Store::open(&seeds_dir).expect("reload store");
    store.config.max_plan_depth = 5;
    store.save().expect("save config");
    let shown = json_of(&run(&dir, "sd", &["config", "show", "--json"]));
    assert_eq!(
        shown["config"]["max_plan_depth"],
        json!(5),
        "sd reads our config"
    );
    assert_eq!(shown["config"]["project"], json!(store.config.project));
    fs::remove_dir_all(&dir).ok();
}
