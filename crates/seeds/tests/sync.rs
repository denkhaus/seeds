//! `seeds sync` improvement suite (seeds-540e): the deliberate
//! behaviors beyond sd parity — the per-file staging preview, the
//! shortstat commit body, and the no-op path. Parity itself is pinned
//! by the tailored differential case (`differential.rs`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value;

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "seeds-sync-{}-{tag}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    let seeds = dir.join(".seeds");
    fs::create_dir_all(&seeds).expect("temp .seeds dir");
    fs::write(
        seeds.join("config.yaml"),
        "project: \"tst\"\nversion: \"1\"\n",
    )
    .expect("config.yaml");
    fs::write(seeds.join("issues.jsonl"), "").expect("issues.jsonl");
    git_ok(&dir, &["init", "-q"]);
    git_ok(&dir, &["config", "user.email", "sync@t.local"]);
    git_ok(&dir, &["config", "user.name", "Sync Suite"]);
    dir
}

fn git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git")
}

fn git_ok(dir: &Path, args: &[&str]) -> String {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_seeds"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("binary runs")
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// UTC today — the date component of the binary's commit message.
fn utc_today() -> String {
    let output = Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .expect("spawn date");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn commit_count(dir: &Path) -> usize {
    let output = git(dir, &["rev-list", "--count", "HEAD"]);
    if output.status.success() {
        text(&output).parse().expect("numeric count")
    } else {
        0
    }
}

fn commit_message(dir: &Path) -> String {
    git_ok(dir, &["log", "-1", "--format=%B"])
}

const ISSUE_LINE: &str = "{\"id\":\"tst-0001\",\"title\":\"a\",\"status\":\"open\",\
                          \"type\":\"task\",\"priority\":2,\
                          \"createdAt\":\"2026-01-01T00:00:00.000Z\",\
                          \"updatedAt\":\"2026-01-01T00:00:00.000Z\"}\n";

#[test]
fn sync_commits_with_shortstat_body_and_noop_path() {
    let dir = temp_repo("commit");
    fs::write(dir.join(".seeds/issues.jsonl"), ISSUE_LINE).expect("issues.jsonl");

    let output = run(&dir, &["sync"]);
    assert!(output.status.success(), "stdout: {}", text(&output));
    assert_eq!(
        text(&output),
        format!("✓ Committed: seeds: sync {}", utc_today())
    );
    let message = commit_message(&dir);
    assert!(
        message.starts_with(&format!("seeds: sync {}\n", utc_today())),
        "the commit subject matches: {message}"
    );
    assert!(
        message.contains("files changed") || message.contains("file changed"),
        "the commit body carries the shortstat line: {message}"
    );
    assert!(
        message.contains("insertion") || message.contains("deletion"),
        "the shortstat body is greppable: {message}"
    );

    // No-op: nothing dirty under .seeds/, non-.seeds dirt is ignored.
    fs::write(dir.join("readme.md"), "hi\n").expect("readme");
    let output = run(&dir, &["sync"]);
    assert!(output.status.success());
    assert_eq!(text(&output), "✓ No changes to commit.");
    assert_eq!(commit_count(&dir), 1, "the no-op path creates no commit");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn sync_status_shows_per_file_preview() {
    let dir = temp_repo("status");
    let baseline = run(&dir, &["sync"]);
    assert!(
        baseline.status.success(),
        "baseline commit: {}",
        text(&baseline)
    );

    let output = run(&dir, &["sync", "--status"]);
    assert!(output.status.success());
    assert_eq!(text(&output), "✓ No uncommitted .seeds/ changes.");
    // Dirty: per-file lines, not sd's collapsed `?? .seeds/` entry.
    fs::write(dir.join(".seeds/plans.jsonl"), "x\n").expect("plans");
    let output = run(&dir, &["sync", "--status"]);
    assert!(output.status.success());
    let stdout = text(&output);
    assert!(stdout.contains("✓ Uncommitted .seeds/ changes:"));
    assert!(stdout.contains(".seeds/plans.jsonl"), "per file: {stdout}");
    assert!(
        !stdout.lines().any(|line| line == "?? .seeds/"),
        "untracked dirs are expanded, not collapsed: {stdout}"
    );
    assert_eq!(commit_count(&dir), 1, "--status never commits");

    // --dry-run previews the message without committing.
    let output = run(&dir, &["sync", "--dry-run"]);
    assert!(output.status.success());
    let stdout = text(&output);
    assert!(stdout.contains("✓ Dry run — would commit:"));
    assert!(stdout.contains("Commit message: seeds: sync "));
    assert_eq!(commit_count(&dir), 1, "--dry-run never commits");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn sync_json_envelopes() {
    let dir = temp_repo("json");

    let output = run(&dir, &["sync", "--json"]);
    assert!(output.status.success());
    let value: Value = serde_json::from_str(&text(&output)).expect("JSON envelope");
    assert_eq!(value["success"], Value::Bool(true));
    assert_eq!(value["command"], json_str("sync"));
    assert_eq!(value["committed"], Value::Bool(true));
    assert_eq!(
        value["message"],
        json_str(&format!("seeds: sync {}", utc_today()))
    );

    // No-op envelope.
    let output = run(&dir, &["sync", "--json"]);
    assert!(output.status.success());
    let value: Value = serde_json::from_str(&text(&output)).expect("JSON envelope");
    assert_eq!(value["committed"], Value::Bool(false));
    assert_eq!(value["message"], json_str("Nothing to commit"));

    // Status envelope.
    fs::write(dir.join(".seeds/issues.jsonl"), ISSUE_LINE).expect("dirty");
    let output = run(&dir, &["sync", "--status", "--json"]);
    assert!(output.status.success());
    let value: Value = serde_json::from_str(&text(&output)).expect("JSON envelope");
    assert_eq!(value["hasChanges"], Value::Bool(true));
    let changes = value["changes"].as_str().expect("changes string");
    assert!(
        changes.contains(".seeds/issues.jsonl"),
        "per file: {changes}"
    );

    fs::remove_dir_all(&dir).ok();
}

fn json_str(s: &str) -> Value {
    Value::String(s.to_owned())
}
