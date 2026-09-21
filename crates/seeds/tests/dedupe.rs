//! `seeds dedupe` integration suite: report mode (gate-friendly exit
//! codes), `--write` healing (byte-exact, atomic, idempotent), and the
//! `--json` envelope — the native tracker-heal command beyond sd
//! parity (ADR-0023 additive).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_store(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "seeds-dedupe-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    let seeds = dir.join(".seeds");
    fs::create_dir_all(&seeds).expect("temp .seeds dir");
    fs::write(
        seeds.join("config.yaml"),
        "project: \"tst\"\nversion: \"1\"\nmax_plan_depth: 3\n",
    )
    .expect("config.yaml");
    dir
}

/// Writes raw JSONL `lines` to `file` (issues/plans/templates) inside
/// the store, preserving each line's exact bytes.
fn write_lines(dir: &Path, file: &str, lines: &[String]) {
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    fs::write(dir.join(".seeds").join(file), text).expect(file);
}

fn read_file(dir: &Path, file: &str) -> String {
    fs::read_to_string(dir.join(".seeds").join(file)).expect(file)
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_seeds"))
        .args(args)
        .current_dir(dir)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_str(stdout(output).trim()).expect("stdout is one JSON object")
}

fn record(id: &str, updated_at: &str, title: &str) -> String {
    json!({
        "id": id,
        "title": title,
        "status": "open",
        "type": "task",
        "priority": 2,
        "createdAt": "2026-01-01T00:00:00.000Z",
        "updatedAt": updated_at,
    })
    .to_string()
}

/// issues.jsonl with one id tripled (newest is NOT the last line), one
/// id tied on updatedAt (later line must win), and one clean id.
fn duplicated_issues() -> Vec<String> {
    vec![
        record("tst-0001", "2026-01-02T00:00:00.000Z", "one-old"),
        record("tst-0002", "2026-01-03T00:00:00.000Z", "two-clean"),
        record("tst-0003", "2026-01-04T00:00:00.000Z", "three-first"),
        record("tst-0001", "2026-01-06T00:00:00.000Z", "one-newest"),
        record("tst-0001", "2026-01-05T00:00:00.000Z", "one-mid"),
        record("tst-0003", "2026-01-04T00:00:00.000Z", "three-later"),
    ]
}

/// The union-newest expectation for [`duplicated_issues`]: first-seen
/// order, newest record per id, later line on updatedAt ties.
fn healed_issues() -> String {
    let kept = [
        record("tst-0001", "2026-01-06T00:00:00.000Z", "one-newest"),
        record("tst-0002", "2026-01-03T00:00:00.000Z", "two-clean"),
        record("tst-0003", "2026-01-04T00:00:00.000Z", "three-later"),
    ];
    let mut text = String::new();
    for line in kept {
        text.push_str(&line);
        text.push('\n');
    }
    text
}

#[test]
fn report_prints_summary_and_exits_one_when_duplicates_exist() {
    let dir = temp_store("report");
    write_lines(&dir, "issues.jsonl", &duplicated_issues());

    let output = run(&dir, &["dedupe"]);
    assert_eq!(output.status.code(), Some(1), "duplicates must exit 1");
    let text = stdout(&output);
    assert!(
        text.contains("issues.jsonl: 2 duplicate ids"),
        "got: {text}"
    );
    assert!(
        text.contains("tst-0001 ×3 kept updatedAt=2026-01-06T00:00:00.000Z"),
        "got: {text}"
    );
    assert!(
        text.contains("dropped: 2026-01-02T00:00:00.000Z, 2026-01-05T00:00:00.000Z"),
        "got: {text}"
    );
    assert!(
        text.contains("tst-0003 ×2 kept updatedAt=2026-01-04T00:00:00.000Z"),
        "got: {text}"
    );
    assert!(
        !text.contains("tst-0002"),
        "clean ids stay unreported: {text}"
    );
    // Report mode never rewrites the file.
    let after = read_file(&dir, "issues.jsonl");
    let mut original = String::new();
    for line in duplicated_issues() {
        original.push_str(&line);
        original.push('\n');
    }
    assert_eq!(after, original);
}

#[test]
fn write_heals_in_place_to_union_newest_bytes() {
    let dir = temp_store("write");
    write_lines(&dir, "issues.jsonl", &duplicated_issues());
    // plans and templates heal under the same record-id rule.
    write_lines(&dir, "plans.jsonl", &[
        record("plan-01", "2026-01-01T00:00:00.000Z", "old"),
        record("plan-01", "2026-01-02T00:00:00.000Z", "new"),
    ]);
    write_lines(&dir, "templates.jsonl", &[
        record("tpl-01", "2026-01-01T00:00:00.000Z", "old"),
        record("tpl-01", "2026-01-02T00:00:00.000Z", "new"),
        record("tpl-02", "2026-01-01T00:00:00.000Z", "only"),
    ]);

    let output = run(&dir, &["dedupe", "--write"]);
    assert_eq!(output.status.code(), Some(0), "--write always exits 0");
    let text = stdout(&output);
    assert!(
        text.contains("issues.jsonl: dropped 3 duplicate lines (2 ids)"),
        "got: {text}"
    );
    assert!(
        text.contains("✓ healed 4 duplicate ids, dropped 5 lines"),
        "got: {text}"
    );

    assert_eq!(read_file(&dir, "issues.jsonl"), healed_issues());
    assert_eq!(
        read_file(&dir, "plans.jsonl"),
        format!("{}\n", record("plan-01", "2026-01-02T00:00:00.000Z", "new"))
    );
    assert_eq!(
        read_file(&dir, "templates.jsonl"),
        format!(
            "{}\n{}\n",
            record("tpl-01", "2026-01-02T00:00:00.000Z", "new"),
            record("tpl-02", "2026-01-01T00:00:00.000Z", "only"),
        )
    );
    assert!(
        !dir.join(".seeds/issues.dedupe-tmp").exists(),
        "temp file must be renamed away"
    );
}

#[cfg(unix)]
#[test]
fn write_is_idempotent_and_leaves_a_clean_file_untouched() {
    use std::os::unix::fs::MetadataExt;

    let dir = temp_store("idempotent");
    write_lines(&dir, "issues.jsonl", &duplicated_issues());
    let healed = healed_issues();
    run(&dir, &["dedupe", "--write"]);
    assert_eq!(read_file(&dir, "issues.jsonl"), healed);

    // Second run on the healed file: exit 0, no rewrite (inode and
    // bytes unchanged).
    let before = fs::metadata(dir.join(".seeds/issues.jsonl")).expect("metadata");
    let output = run(&dir, &["dedupe", "--write"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).contains("nothing to write"),
        "got: {}",
        stdout(&output)
    );
    let after = fs::metadata(dir.join(".seeds/issues.jsonl")).expect("metadata");
    assert_eq!(
        before.ino(),
        after.ino(),
        "clean file must not be rewritten"
    );
    assert_eq!(read_file(&dir, "issues.jsonl"), healed);
}

#[test]
fn clean_store_exits_zero_in_both_modes() {
    let dir = temp_store("clean");
    write_lines(&dir, "issues.jsonl", &[
        record("tst-0001", "2026-01-01T00:00:00.000Z", "one"),
        record("tst-0002", "2026-01-02T00:00:00.000Z", "two"),
    ]);

    let report = run(&dir, &["dedupe"]);
    assert_eq!(report.status.code(), Some(0));
    assert!(stdout(&report).contains("✓ no duplicate ids found"));

    let write = run(&dir, &["dedupe", "--write"]);
    assert_eq!(write.status.code(), Some(0));
    assert!(stdout(&write).contains("nothing to write"));
}

#[test]
fn json_report_and_write_envelopes_follow_the_command_style() {
    let dir = temp_store("json");
    write_lines(&dir, "issues.jsonl", &duplicated_issues());

    let report = run(&dir, &["dedupe", "--json"]);
    assert_eq!(report.status.code(), Some(1), "duplicates still exit 1");
    let value = stdout_json(&report);
    assert_eq!(value["success"], json!(true));
    assert_eq!(value["command"], json!("dedupe"));
    assert_eq!(value["write"], json!(false));
    assert_eq!(value["duplicateIds"], json!(2));
    let issue_file = &value["files"][0];
    assert_eq!(issue_file["file"], json!("issues.jsonl"));
    assert_eq!(issue_file["duplicates"][0]["id"], json!("tst-0001"));
    assert_eq!(issue_file["duplicates"][0]["count"], json!(3));
    assert_eq!(
        issue_file["duplicates"][0]["keptUpdatedAt"],
        json!("2026-01-06T00:00:00.000Z")
    );
    assert_eq!(
        issue_file["duplicates"][0]["droppedUpdatedAts"],
        json!(["2026-01-02T00:00:00.000Z", "2026-01-05T00:00:00.000Z"])
    );

    let write = run(&dir, &["dedupe", "--json", "--write"]);
    assert_eq!(write.status.code(), Some(0));
    let value = stdout_json(&write);
    assert_eq!(value["success"], json!(true));
    assert_eq!(value["write"], json!(true));
    assert_eq!(value["written"], json!(true));
    assert_eq!(value["droppedLines"], json!(3));
    assert_eq!(read_file(&dir, "issues.jsonl"), healed_issues());
}
