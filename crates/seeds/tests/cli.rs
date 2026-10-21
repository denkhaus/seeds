//! CLI parity suite: the `seeds` binary against the pinned reference
//! behavior of sd 0.5.15 — flag surface, JSON envelope shapes, filter
//! and limit semantics, mutation writes, and error reporting.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_store(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "seeds-cli-{}-{tag}-{}",
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
    fs::write(seeds.join("issues.jsonl"), "").expect("issues.jsonl");
    fs::write(seeds.join("plans.jsonl"), "").expect("plans.jsonl");
    fs::write(seeds.join("templates.jsonl"), "").expect("templates.jsonl");
    dir
}

fn write_records(dir: &Path, records: &[Value]) {
    let mut text = String::new();
    for record in records {
        text.push_str(&record.to_string());
        text.push('\n');
    }
    fs::write(dir.join(".seeds/issues.jsonl"), text).expect("issues.jsonl");
}

fn read_records(dir: &Path) -> Vec<Value> {
    let text = fs::read_to_string(dir.join(".seeds/issues.jsonl")).expect("issues.jsonl");
    text.lines()
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()
        .expect("valid JSONL")
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_seeds"))
        .args(args)
        .current_dir(dir)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("binary runs")
}

fn stdout_json(output: &Output) -> Value {
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8 stdout");
    serde_json::from_str(text.trim()).expect("stdout is one JSON object")
}

fn ids_of(value: &Value) -> Vec<String> {
    value["issues"]
        .as_array()
        .expect("issues array")
        .iter()
        .map(|issue| issue["id"].as_str().expect("id").to_owned())
        .collect()
}

fn record(id: &str, title: &str, extra: Value) -> Value {
    // Distinct createdAt/updatedAt per id (its numeric suffix) so sort
    // orders are deterministic.
    let suffix = id.rsplit('-').next().unwrap_or("1");
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
    match extra {
        Value::Object(fields) => {
            let Value::Object(map) = &mut base else {
                unreachable!("base is an object");
            };
            for (key, value) in fields {
                map.insert(key, value);
            }
            base
        }
        _ => base,
    }
}

/// The standard fixture: six issues covering statuses, priorities,
/// labels, assignees, and both resolved and unresolved dependencies.
fn standard() -> Vec<Value> {
    vec![
        record(
            "tst-0001",
            "Fix login PARITY bug",
            json!({"type": "bug", "priority": 0, "labels": ["bug", "ui"], "assignee": "alice"}),
        ),
        record(
            "tst-0002",
            "Write docs",
            json!({"priority": 1, "description": "Mention parity in the guide"}),
        ),
        record(
            "tst-0003",
            "Refactor core",
            json!({"status": "in_progress", "type": "feature", "priority": 2,
                   "labels": ["bug"], "assignee": "bob"}),
        ),
        record(
            "tst-0004",
            "Old work",
            json!({"status": "closed", "priority": 3}),
        ),
        record(
            "tst-0005",
            "Follow-up",
            json!({"priority": 4, "blockedBy": ["tst-0004"]}),
        ),
        record(
            "tst-0006",
            "Blocked work",
            json!({"priority": 1, "blockedBy": ["tst-0001"]}),
        ),
    ]
}

// ---------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------

#[test]
fn show_single_json_uses_issue_with_reference_key_order() {
    let dir = temp_store("show-single");
    write_records(&dir, &standard());
    let output = run(&dir, &["show", "tst-0001", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["success"], json!(true));
    assert_eq!(value["command"], json!("show"));
    assert_eq!(value["issue"]["id"], json!("tst-0001"));
    let keys: Vec<&str> = value["issue"]
        .as_object()
        .expect("issue object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys[..7], [
        "id",
        "title",
        "status",
        "type",
        "priority",
        "createdAt",
        "updatedAt"
    ]);
}

#[test]
fn show_multi_json_uses_issues_array() {
    let dir = temp_store("show-multi");
    write_records(&dir, &standard());
    let value = stdout_json(&run(&dir, &["show", "tst-0001", "tst-0002", "--json"]));
    assert_eq!(value["command"], json!("show"));
    assert_eq!(ids_of(&value), vec!["tst-0001", "tst-0002"]);
}

#[test]
fn show_missing_id_reports_stderr_error_and_nonzero_exit() {
    let dir = temp_store("show-missing");
    write_records(&dir, &standard());
    let output = run(&dir, &["show", "tst-zzzz", "--format", "json"]);
    // sd 0.5.15 reports show errors on stderr (`Error: …`) with an
    // EMPTY stdout — even in --format json mode, no JSON envelope on
    // this path (pinned by the differential battery, seeds-25b5).
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "stdout stays empty: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8 stderr");
    assert!(stderr.contains("Issue not found"), "stderr: {stderr}");
}

// ---------------------------------------------------------------------------
// list / ready / search
// ---------------------------------------------------------------------------

#[test]
fn list_json_envelope_and_default_scope() {
    let dir = temp_store("list-default");
    write_records(&dir, &standard());
    let value = stdout_json(&run(&dir, &["list", "--format", "json"]));
    assert_eq!(value["success"], json!(true));
    assert_eq!(value["command"], json!("list"));
    // Default scope excludes closed; sorted by priority asc.
    assert_eq!(ids_of(&value), vec![
        "tst-0001", "tst-0006", "tst-0002", "tst-0003", "tst-0005"
    ]);
}

#[test]
fn list_all_includes_closed() {
    let dir = temp_store("list-all");
    write_records(&dir, &standard());
    let value = stdout_json(&run(&dir, &[
        "list", "--all", "--format", "json", "--sort", "id",
    ]));
    assert_eq!(ids_of(&value).len(), 6);
}

#[test]
fn list_assignee_filter() {
    let dir = temp_store("list-assignee");
    write_records(&dir, &standard());
    let value = stdout_json(&run(&dir, &[
        "list",
        "--format",
        "json",
        "--assignee",
        "alice",
        "--sort",
        "id",
    ]));
    assert_eq!(ids_of(&value), vec!["tst-0001"]);
}

#[test]
fn list_limit_defaults_to_50_and_200_lifts_it() {
    let dir = temp_store("list-limit");
    let records: Vec<Value> = (0..55)
        .map(|number| record(&format!("tst-{number:04}"), "bulk", json!({})))
        .collect();
    write_records(&dir, &records);
    let value = stdout_json(&run(&dir, &["list", "--format", "json"]));
    assert_eq!(ids_of(&value).len(), 50, "default limit is 50");
    let value = stdout_json(&run(&dir, &["list", "--format", "json", "--limit", "200"]));
    assert_eq!(ids_of(&value).len(), 55, "--limit 200 shows everything");
    let value = stdout_json(&run(&dir, &["list", "--format", "json", "--limit", "3"]));
    assert_eq!(ids_of(&value).len(), 3);
}

#[test]
fn ready_lists_open_unblocked_only() {
    let dir = temp_store("ready");
    write_records(&dir, &standard());
    let value = stdout_json(&run(&dir, &["ready", "--format", "json"]));
    // tst-0006 is blocked by open tst-0001; tst-0005's blocker is
    // closed, so it is ready; closed and in_progress are out.
    // Priority asc: tst-0001 (p0), tst-0002 (p1), tst-0005 (p4).
    assert_eq!(ids_of(&value), vec!["tst-0001", "tst-0002", "tst-0005"]);
}

#[test]
fn ready_respects_assignee_and_limit() {
    let dir = temp_store("ready-assignee");
    write_records(&dir, &standard());
    let value = stdout_json(&run(&dir, &[
        "ready",
        "--assignee",
        "bob",
        "--format",
        "json",
    ]));
    // tst-0003 is in_progress, not open: no ready work for bob.
    assert_eq!(ids_of(&value), Vec::<String>::new());
}

#[test]
fn search_matches_title_and_description_case_insensitively() {
    let dir = temp_store("search");
    write_records(&dir, &standard());
    let value = stdout_json(&run(&dir, &["search", "parity", "--format", "json"]));
    assert_eq!(value["command"], json!("search"));
    assert_eq!(value["query"], json!("parity"));
    assert_eq!(ids_of(&value), vec!["tst-0001", "tst-0002"]);
}

#[test]
fn filter_flag_combinations_table() {
    struct Case {
        name: &'static str,
        args: &'static [&'static str],
        want: Vec<&'static str>,
    }
    let cases = [
        Case {
            name: "status filter",
            args: &["list", "--status", "in_progress", "--format", "json"],
            want: vec!["tst-0003"],
        },
        Case {
            name: "label AND",
            args: &[
                "list", "--label", "bug,ui", "--format", "json", "--sort", "id",
            ],
            want: vec!["tst-0001"],
        },
        Case {
            name: "label-any OR",
            args: &[
                "list",
                "--label-any",
                "ui",
                "--format",
                "json",
                "--sort",
                "id",
            ],
            want: vec!["tst-0001"],
        },
        Case {
            name: "unlabeled",
            args: &["list", "--unlabeled", "--format", "json", "--sort", "id"],
            want: vec!["tst-0002", "tst-0005", "tst-0006"],
        },
        Case {
            name: "priority levels",
            args: &[
                "list",
                "--priority",
                "0,1",
                "--format",
                "json",
                "--sort",
                "id",
            ],
            want: vec!["tst-0001", "tst-0002", "tst-0006"],
        },
        Case {
            name: "priority-max",
            args: &[
                "list",
                "--priority-max",
                "1",
                "--format",
                "json",
                "--sort",
                "id",
            ],
            want: vec!["tst-0001", "tst-0002", "tst-0006"],
        },
        Case {
            name: "P-form priority",
            args: &["list", "--priority", "P0", "--format", "json"],
            want: vec!["tst-0001"],
        },
        Case {
            name: "type filter",
            args: &["list", "--type", "bug", "--format", "json"],
            want: vec!["tst-0001"],
        },
    ];
    for case in cases {
        let dir = temp_store("filters");
        write_records(&dir, &standard());
        let value = stdout_json(&run(&dir, case.args));
        assert_eq!(
            ids_of(&value),
            case.want
                .iter()
                .map(|id| (*id).to_owned())
                .collect::<Vec<_>>(),
            "case: {}",
            case.name
        );
    }
}

#[test]
fn ids_and_compact_formats_match_reference_shapes() {
    let dir = temp_store("formats");
    write_records(&dir, &standard());
    let output = run(&dir, &["list", "--format", "ids", "--sort", "id"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        text.starts_with("tst-0001\n"),
        "ids mode: one id per line, no footer"
    );
    assert!(!text.contains("issue(s)"));

    let output = run(&dir, &["list", "--format", "compact", "--sort", "id"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        text.starts_with("tst-0001 Critical open Fix login PARITY bug\n"),
        "compact mode shape: {text}"
    );
    assert!(text.contains("tst-0006 High blocked Blocked work\n"));

    let output = run(&dir, &["list", "--sort", "id", "--limit", "1"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        text.starts_with("- tst-0001 · Fix login PARITY bug") || text.starts_with("! tst-0001"),
        "default rich line shape: {text}"
    );
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

#[test]
fn create_json_and_written_record() {
    let dir = temp_store("create");
    write_records(&dir, &standard());
    let output = run(&dir, &[
        "create",
        "--title",
        "New thing",
        "--type",
        "bug",
        "--priority",
        "P1",
        "--labels",
        "x, y",
        "--assignee",
        "alice",
        "--description",
        "body",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["success"], json!(true));
    assert_eq!(value["command"], json!("create"));
    let id = value["id"].as_str().expect("id").to_owned();
    assert!(id.starts_with("tst-"), "project-prefixed id: {id}");
    assert_eq!(id.len(), 8, "tst- + hex4");

    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!(id))
        .expect("record stored");
    assert_eq!(stored["title"], json!("New thing"));
    assert_eq!(stored["status"], json!("open"));
    assert_eq!(stored["type"], json!("bug"));
    assert_eq!(stored["priority"], json!(1));
    assert_eq!(stored["labels"], json!(["x", "y"]));
    assert_eq!(stored["assignee"], json!("alice"));
    assert_eq!(stored["description"], json!("body"));
    assert_eq!(stored["createdAt"], stored["updatedAt"]);
    assert!(stored["createdAt"].as_str().expect("ts").ends_with('Z'));
}

#[test]
fn create_defaults_and_missing_title() {
    let dir = temp_store("create-defaults");
    write_records(&dir, &standard());
    let output = run(&dir, &["create", "--title", "Plain"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(text.starts_with("✓ Created tst-"), "default output: {text}");

    let output = run(&dir, &["create"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
    assert!(stderr.contains("--title"), "stderr: {stderr}");
}

#[test]
fn create_rejects_bad_priority_and_type() {
    let dir = temp_store("create-bad");
    write_records(&dir, &standard());
    let output = run(&dir, &["create", "--title", "x", "--priority", "9"]);
    assert_eq!(output.status.code(), Some(1));
    let output = run(&dir, &["create", "--title", "x", "--type", "chore"]);
    assert_eq!(output.status.code(), Some(1));
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn update_applies_fields_and_json_shape() {
    let dir = temp_store("update");
    write_records(&dir, &standard());
    let output = run(&dir, &[
        "update",
        "tst-0002",
        "--status",
        "in_progress",
        "--title",
        "Renamed",
        "--set-description",
        "via alias",
        "--add-label",
        "a,b",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["command"], json!("update"));
    assert_eq!(value["issue"]["id"], json!("tst-0002"));
    assert_eq!(value["issue"]["status"], json!("in_progress"));
    assert_eq!(value["issue"]["title"], json!("Renamed"));
    assert_eq!(value["issue"]["description"], json!("via alias"));
    assert_eq!(value["issue"]["labels"], json!(["a", "b"]));

    // Label algebra on top.
    run(&dir, &[
        "update",
        "tst-0002",
        "--add-label",
        "a",
        "--remove-label",
        "b",
    ]);
    let value = stdout_json(&run(&dir, &["show", "tst-0002", "--format", "json"]));
    assert_eq!(value["issue"]["labels"], json!(["a"]));

    run(&dir, &["update", "tst-0002", "--set-labels", ""]);
    let value = stdout_json(&run(&dir, &["show", "tst-0002", "--format", "json"]));
    assert!(
        value["issue"].get("labels").is_none(),
        "empty set-labels clears"
    );
}

#[test]
fn update_extensions_merge_and_clear() {
    let dir = temp_store("update-ext");
    write_records(&dir, &standard());
    run(&dir, &[
        "update",
        "tst-0002",
        "--extensions",
        "{\"queued\":true,\"note\":\"a\"}",
    ]);
    run(&dir, &[
        "update",
        "tst-0002",
        "--extensions",
        "{\"note\":\"b\"}",
    ]);
    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!("tst-0002"))
        .expect("record");
    // Shallow merge: queued survives, note is overwritten.
    assert_eq!(stored["extensions"], json!({"queued": true, "note": "b"}));

    run(&dir, &["update", "tst-0002", "--clear-extensions"]);
    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!("tst-0002"))
        .expect("record");
    assert!(
        stored.get("extensions").is_none(),
        "clear-extensions removes"
    );
}

#[test]
fn update_description_verbs_append_and_change_shape() {
    let dir = temp_store("update-desc-verbs");
    write_records(&dir, &standard());

    // Set names its change shape: old -> new chars (tst-0002 starts
    // with the 27-char "Mention parity in the guide").
    let output = run(&dir, &[
        "update",
        "tst-0002",
        "--set-description",
        "Original body.\n\n",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[0], "✓ Updated tst-0002");
    assert_eq!(lines[1], "description: set 27 -> 16 chars");

    // Append joins with exactly one blank line: trailing whitespace on
    // the existing text is trimmed, the appended text both sides.
    let output = run(&dir, &[
        "update",
        "tst-0002",
        "--append-description",
        " Evidence block. ",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[1], "description: appended +15 (total 31)");
    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!("tst-0002"))
        .expect("record");
    assert_eq!(
        stored["description"],
        json!("Original body.\n\nEvidence block.")
    );

    // Append onto a record without a description: first content, no
    // leading blank line, shape line still reports the total.
    let output = run(&dir, &[
        "update",
        "tst-0003",
        "--append-description",
        "First note",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[1], "description: appended +10 (total 10)");
    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!("tst-0003"))
        .expect("record");
    assert_eq!(stored["description"], json!("First note"));
}

#[test]
fn update_description_verbs_reject_bad_input() {
    let dir = temp_store("update-desc-verbs-errors");
    write_records(&dir, &standard());

    let output = run(&dir, &[
        "update",
        "tst-0002",
        "--set-description",
        "a",
        "--append-description",
        "b",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
    assert!(stderr.contains("mutually exclusive"), "stderr: {stderr}");

    let output = run(&dir, &["update", "tst-0002", "--append-description", "  "]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
    assert!(
        stderr.contains("--append-description requires text"),
        "stderr: {stderr}"
    );
}

#[test]
fn update_description_set_aliases_are_gone() {
    let dir = temp_store("update-alias-purge");
    write_records(&dir, &standard());
    // The pre-verb aliases fail loud on both write commands (the
    // documented deviation, seeds-0297).
    for (command, id_args, flag) in [
        ("update", vec!["tst-0002"], "--description"),
        ("update", vec!["tst-0002"], "--desc"),
        ("update", vec!["tst-0002"], "--body"),
        ("create", vec![], "--desc"),
        ("create", vec![], "--body"),
    ] {
        let mut args = vec![command];
        args.extend(id_args);
        args.extend([flag, "x"]);
        let output = run(&dir, &args);
        assert_eq!(output.status.code(), Some(1), "{flag} on {command} fails");
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
        assert!(
            stderr.contains(&format!("unknown option '{flag}'")),
            "{flag} on {command}: stderr: {stderr}"
        );
    }
}

#[test]
fn update_replace_in_description_span_surgery() {
    let dir = temp_store("update-replace-in");
    write_records(&dir, &standard());
    run(&dir, &[
        "update",
        "tst-0002",
        "--set-description",
        "Demand line.\nBasis: hash-aaaa\nDesign: keep",
    ]);

    // Exactly one occurrence: replaced, shape line names the sizes
    // (42 chars -> 42 chars, same-length hash swap).
    let output = run(&dir, &[
        "update",
        "tst-0002",
        "--replace-in-description",
        r#"{"find": "hash-aaaa", "replace": "hash-bbbb"}"#,
    ]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[0], "✓ Updated tst-0002");
    assert_eq!(lines[1], "description: replaced span (42 -> 42 chars)");
    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!("tst-0002"))
        .expect("record");
    assert_eq!(
        stored["description"],
        json!("Demand line.\nBasis: hash-bbbb\nDesign: keep")
    );

    // Empty replace deletes the span (span semantics: the trailing
    // newline stays, 42 -> 30 chars).
    let output = run(&dir, &[
        "update",
        "tst-0002",
        "--replace-in-description",
        r#"{"find": "\nDesign: keep", "replace": ""}"#,
    ]);
    assert_eq!(output.status.code(), Some(0));
    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!("tst-0002"))
        .expect("record");
    assert_eq!(
        stored["description"],
        json!("Demand line.\nBasis: hash-bbbb")
    );
}

#[test]
fn update_replace_in_description_fails_loud() {
    let dir = temp_store("update-replace-in-errors");
    write_records(&dir, &standard());
    run(&dir, &[
        "update",
        "tst-0002",
        "--set-description",
        "one tag and tag again",
    ]);

    let cases: Vec<(&[&str], &str)> = vec![
        (
            &[
                "update",
                "tst-0002",
                "--replace-in-description",
                r#"{"find": "tag", "replace": "x"}"#,
            ],
            "2 occurrences",
        ),
        (
            &[
                "update",
                "tst-0002",
                "--replace-in-description",
                r#"{"find": "missing", "replace": "x"}"#,
            ],
            "text not found",
        ),
        (
            &[
                "update",
                "tst-0002",
                "--replace-in-description",
                r#"{"find": "", "replace": "x"}"#,
            ],
            "find must not be empty",
        ),
        (
            &["update", "tst-0002", "--replace-in-description", "not json"],
            "must be a JSON object",
        ),
        (
            &[
                "update",
                "tst-0002",
                "--replace-in-description",
                r#"{"find": "one"}"#,
            ],
            "must be a JSON object",
        ),
        (
            &[
                "update",
                "tst-0002",
                "--set-description",
                "x",
                "--replace-in-description",
                r#"{"find": "y", "replace": "z"}"#,
            ],
            "mutually exclusive",
        ),
    ];
    for (args, needle) in cases {
        let output = run(&dir, args);
        assert_eq!(output.status.code(), Some(1), "{args:?} fails");
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
        assert!(stderr.contains(needle), "{args:?}: stderr: {stderr}");
    }
}

/// `update` reports a missing id as a failure envelope (seeds-1579
/// dropped the old no-`--format` boundary: the flag is accepted now,
/// see `write_commands_accept_format_ids_and_json`).
#[test]
fn update_reports_missing_issue_as_failure_envelope() {
    let dir = temp_store("update-flags");
    write_records(&dir, &standard());
    let output = run(&dir, &["update", "tst-zzzz", "--title", "x", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let value = stdout_json(&output);
    assert_eq!(value["success"], json!(false));
    assert_eq!(value["command"], json!("update"));
}

// ---------------------------------------------------------------------------
// close
// ---------------------------------------------------------------------------

#[test]
fn close_json_reason_and_written_fields() {
    let dir = temp_store("close");
    write_records(&dir, &standard());
    let output = run(&dir, &[
        "close",
        "tst-0001",
        "tst-0002",
        "--reason",
        "done here",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["command"], json!("close"));
    assert_eq!(value["closed"], json!(["tst-0001", "tst-0002"]));

    let stored: Vec<Value> = read_records(&dir)
        .into_iter()
        .filter(|record| record["id"] == json!("tst-0001"))
        .collect();
    let stored = &stored[0];
    assert_eq!(stored["status"], json!("closed"));
    assert_eq!(stored["closeReason"], json!("done here"));
    assert!(
        stored["closedAt"]
            .as_str()
            .expect("closedAt")
            .ends_with('Z')
    );

    // Default text output shape.
    let output = run(&dir, &["close", "tst-0005"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert_eq!(text.trim_end(), "✓ Closed tst-0005");
}

#[test]
fn close_missing_issue_fails_without_mutating() {
    let dir = temp_store("close-missing");
    write_records(&dir, &standard());
    let output = run(&dir, &["close", "tst-0001", "tst-zzzz", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let value = stdout_json(&output);
    assert_eq!(value["success"], json!(false));
    assert_eq!(value["command"], json!("close"));
    let stored = read_records(&dir)
        .into_iter()
        .find(|record| record["id"] == json!("tst-0001"))
        .expect("record");
    assert_eq!(stored["status"], json!("open"), "valid id stays untouched");
}

/// Close hygiene (seeds-8666): closing clears the pending triage roles
/// while every other label — `wontfix` among them — survives, and a
/// record left without labels loses the field entirely. The text
/// surface stays exactly the reference's (the deviation is store-side,
/// README DEVIATIONS).
#[test]
fn close_strips_pending_triage_labels() {
    let dir = temp_store("close-triage");
    write_records(&dir, &[
        record(
            "tst-0001",
            "Triaged work",
            json!({"labels": ["needs-triage", "ready-for-agent", "bug", "wontfix"]}),
        ),
        record(
            "tst-0002",
            "Only triage roles",
            json!({"labels": ["needs-info"]}),
        ),
        record(
            "tst-0003",
            "Already clean",
            json!({"labels": ["backend"], "status": "closed",
                   "closedAt": "2026-01-05T00:00:00.000Z"}),
        ),
    ]);
    let output = run(&dir, &["close", "tst-0001", "tst-0002"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert_eq!(
        text.trim_end(),
        "✓ Closed tst-0001\n✓ Closed tst-0002",
        "silent deviation: no extra close output"
    );

    let records = read_records(&dir);
    let find = |id: &str| {
        records
            .iter()
            .find(|record| record["id"] == json!(id))
            .expect("record")
    };
    assert_eq!(find("tst-0001")["status"], json!("closed"));
    assert_eq!(
        find("tst-0001")["labels"],
        json!(["bug", "wontfix"]),
        "pending roles dropped, decision label and content labels kept"
    );
    assert!(
        find("tst-0002").get("labels").is_none(),
        "all-labels record loses the field: {}",
        find("tst-0002")
    );
    assert_eq!(
        find("tst-0003")["labels"],
        json!(["backend"]),
        "untouched record keeps its labels"
    );
}

/// Close hygiene also holds on the other two write paths that can
/// leave a record `closed` (seeds-8666): `update --status closed` and
/// a plain write against an already-closed record.
#[test]
fn update_write_on_closed_record_strips_triage_labels() {
    let dir = temp_store("update-triage");
    write_records(&dir, &[
        record(
            "tst-0001",
            "Closing through update",
            json!({"labels": ["ready-for-human", "cli"]}),
        ),
        record(
            "tst-0002",
            "Stale roles on history",
            json!({"status": "closed", "closedAt": "2026-01-05T00:00:00.000Z",
                   "labels": ["ready-for-agent", "parity"]}),
        ),
    ]);
    let output = run(&dir, &[
        "update", "tst-0001", "--status", "closed", "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let output = run(&dir, &["update", "tst-0002", "--title", "Renamed"]);
    assert_eq!(output.status.code(), Some(0));

    let records = read_records(&dir);
    let find = |id: &str| {
        records
            .iter()
            .find(|record| record["id"] == json!(id))
            .expect("record")
    };
    assert_eq!(find("tst-0001")["labels"], json!(["cli"]));
    assert_eq!(find("tst-0002")["labels"], json!(["parity"]));

    // An open record keeps every label — the rule keys on the closed
    // status only.
    let dir = temp_store("update-triage-open");
    write_records(&dir, &[record(
        "tst-0001",
        "Still open",
        json!({"labels": ["needs-triage"]}),
    )]);
    let output = run(&dir, &["update", "tst-0001", "--title", "Renamed"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(read_records(&dir)[0]["labels"], json!(["needs-triage"]));
}

/// `close --comment` writes the closing remark in one call
/// (seeds-1579): appended after one blank line to every closed record,
/// reported by a single confirmation line in text mode, while the
/// envelope keeps the reference's shape. A blank comment fails before
/// anything is written.
#[test]
fn close_comment_appends_the_remark_in_one_call() {
    let dir = temp_store("close-comment");
    write_records(&dir, &[
        record("tst-0001", "First", json!({"description": "Body line"})),
        record("tst-0002", "Second", json!({})),
    ]);
    let output = run(&dir, &[
        "close",
        "tst-0001",
        "tst-0002",
        "--reason",
        "done here",
        "--comment",
        "Shipped in 0.9.2",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["closed"], json!(["tst-0001", "tst-0002"]));

    let records = read_records(&dir);
    assert_eq!(
        records[0]["description"],
        json!("Body line\n\nShipped in 0.9.2"),
        "existing description keeps its text, comment after one blank line"
    );
    assert_eq!(records[0]["closeReason"], json!("done here"));
    assert_eq!(records[0]["status"], json!("closed"));
    assert_eq!(
        records[1]["description"],
        json!("Shipped in 0.9.2"),
        "a record without a description gets the comment alone"
    );

    // Text mode: the close lines stay reference-shaped, the appended
    // comment gets its own confirmation.
    let dir = temp_store("close-comment-text");
    write_records(&dir, &[record("tst-0001", "First", json!({}))]);
    let output = run(&dir, &["close", "tst-0001", "--comment", "Wrapped up"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).expect("utf-8");
    assert_eq!(
        text.trim_end(),
        "✓ Closed tst-0001\n+ comment appended to 1 issue(s)"
    );

    // A blank comment is rejected before the store is touched.
    let dir = temp_store("close-comment-blank");
    write_records(&dir, &[record("tst-0001", "First", json!({}))]);
    let output = run(&dir, &["close", "tst-0001", "--comment", "   "]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    assert!(stderr.contains("--comment requires text"), "{stderr}");
    assert_eq!(read_records(&dir)[0]["status"], json!("open"));
}

/// `--priority` accepts every shape the output prints (seeds-1579):
/// numbers, `P`-codes, level names, and the shown `P2 Medium` form —
/// on writes and on the filters that share the parser.
#[test]
fn priority_accepts_numbers_names_and_the_shown_form() {
    // The filter path shares `parse_priority`.
    let dir = temp_store("priority-filter");
    write_records(&dir, &standard());
    let output = run(&dir, &["list", "--priority", "Medium", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["count"], json!(1), "only tst-0003 is P2: {value}");
    assert_eq!(value["issues"][0]["id"], json!("tst-0003"));

    let dir = temp_store("priority-names");
    write_records(&dir, &standard());
    for (value, expected) in [
        ("0", 0),
        ("P4", 4),
        ("critical", 0),
        ("High", 1),
        ("medium", 2),
        ("P2 Medium", 2),
        ("p3 low", 3),
        (" BACKLOG ", 4),
    ] {
        let output = run(&dir, &["create", "--title", "probe", "--priority", value]);
        assert_eq!(output.status.code(), Some(0), "{value} is accepted");
        // Canonical store order (seeds-e29a): the new record sits at
        // its id's position, so look it up by the reported id.
        let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
        let id = stdout
            .trim()
            .strip_prefix("✓ Created ")
            .unwrap_or_else(|| panic!("create reports the id: {stdout}"))
            .to_owned();
        let created = read_records(&dir)
            .iter()
            .find(|record| record["id"] == json!(id))
            .expect("the created record is in the store")["priority"]
            .clone();
        assert_eq!(created, json!(expected), "{value} means P{expected}");
    }
}

/// The `--priority` error names the accepted shapes (seeds-1579) and
/// rejects unknown names, out-of-range codes, and mismatched pairs.
#[test]
fn priority_rejects_unknown_names_and_mismatched_pairs() {
    let dir = temp_store("priority-bad");
    write_records(&dir, &standard());
    for value in ["5", "P5", "P2 High", "mediumish", "2 nope"] {
        let output = run(&dir, &["create", "--title", "probe", "--priority", value]);
        assert_eq!(output.status.code(), Some(1), "{value} is rejected");
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
        assert!(stderr.contains("--priority must be 0-4, P0-P4"), "{stderr}");
        assert!(stderr.contains("P2 Medium"), "{stderr}");
    }
    assert_eq!(read_records(&dir).len(), 6, "no rejected create wrote");
}

/// `--format` spans the write commands too (seeds-1579): `ids` prints
/// the affected identifiers one per line (the README names what an id
/// is per command), `json` equals `--json`, and the human modes land on
/// the default text — unknown modes fall back to it the way the read
/// side falls back to markdown.
#[test]
fn write_commands_accept_format_ids_and_json() {
    let dir = temp_store("write-format");
    write_records(&dir, &standard());

    // create: the new id and nothing else.
    let output = run(&dir, &[
        "create", "--title", "Fresh", "--labels", "x,y", "--format", "ids",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let new_id = read_records(&dir).last().expect("create appends a record")["id"]
        .as_str()
        .expect("id")
        .to_owned();
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        format!("{new_id}\n")
    );

    // update / label add / dep add / block / unblock: the addressed id.
    for args in [
        vec![
            "update",
            new_id.as_str(),
            "--title",
            "Renamed",
            "--format",
            "ids",
        ],
        vec!["label", "add", new_id.as_str(), "z", "--format", "ids"],
        vec!["dep", "add", new_id.as_str(), "tst-0001", "--format", "ids"],
        vec![
            "block",
            new_id.as_str(),
            "--by",
            "tst-0001",
            "--format",
            "ids",
        ],
        vec!["unblock", new_id.as_str(), "--all", "--format", "ids"],
    ] {
        let output = run(&dir, &args);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert_eq!(
            String::from_utf8(output.stdout).expect("utf-8"),
            format!("{new_id}\n"),
            "{args:?} prints the addressed id"
        );
    }

    // close: every closed id, in order.
    let output = run(&dir, &[
        "close",
        "tst-0002",
        new_id.as_str(),
        "--format",
        "ids",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        format!("tst-0002\n{new_id}\n")
    );

    // dep list / label list / label list-all: the ids they report on.
    let output = run(&dir, &["dep", "list", new_id.as_str(), "--format", "ids"]);
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        "tst-0001\n",
        "dep list prints the record's dependency graph"
    );
    let output = run(&dir, &["label", "list", new_id.as_str(), "--format", "ids"]);
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        "x\ny\nz\n"
    );
    let output = run(&dir, &["label", "list-all", "--format", "ids"]);
    let all = String::from_utf8(output.stdout).expect("utf-8");
    for label in ["x\n", "y\n", "z\n"] {
        assert!(all.contains(label), "list-all ids carry {label:?}: {all}");
    }

    // `--format json` ≡ `--json`, the human modes share the text
    // surface, and an unknown mode falls back to it.
    let json_flag = run(&dir, &["update", "tst-0003", "--title", "A", "--json"]);
    let json_format = run(&dir, &[
        "update", "tst-0003", "--title", "B", "--format", "json",
    ]);
    let flag_value = stdout_json(&json_flag);
    let format_value = stdout_json(&json_format);
    assert_eq!(format_value["success"], json!(true));
    assert_eq!(format_value["command"], flag_value["command"]);
    let flag_issue = flag_value["issue"].as_object().expect("issue");
    let format_issue = format_value["issue"].as_object().expect("issue");
    assert_eq!(
        format_issue.keys().collect::<Vec<_>>(),
        flag_issue.keys().collect::<Vec<_>>(),
        "--format json carries the same envelope shape as --json"
    );
    assert_eq!(
        format_value["issue"]["title"],
        json!("B"),
        "the write landed"
    );
    for mode in ["markdown", "plain", "bogus"] {
        let output = run(&dir, &[
            "update", "tst-0003", "--title", "C", "--format", mode,
        ]);
        assert_eq!(output.status.code(), Some(0), "{mode} falls back to text");
        assert_eq!(
            String::from_utf8(output.stdout).expect("utf-8"),
            "✓ Updated tst-0003\n",
            "{mode} lands on the default text"
        );
    }
}

/// `doctor --format ids` lists the checks that did not pass — the
/// report's stable names, empty on a healthy store (seeds-1579).
#[test]
fn doctor_format_ids_lists_non_passing_checks() {
    let dir = temp_store("doctor-format-ids");
    write_records(&dir, &standard());
    let output = run(&dir, &["doctor", "--format", "ids"]);
    assert_eq!(output.status.code(), Some(0), "no check FAILED here");
    let listed = String::from_utf8(output.stdout).expect("utf-8");
    assert!(
        listed
            .lines()
            .all(|line| !line.is_empty() && !line.contains(' ')),
        "one stable check name per line: {listed}"
    );

    // A malformed line trips the `jsonl-integrity` failure, so its
    // name joins the list.
    let path = dir.join(".seeds/issues.jsonl");
    let mut text = fs::read_to_string(&path).expect("issues.jsonl");
    text.push_str("not json at all\n");
    fs::write(&path, text).expect("issues.jsonl");
    let output = run(&dir, &["doctor", "--format", "ids"]);
    assert_eq!(output.status.code(), Some(1));
    let listed = String::from_utf8(output.stdout).expect("utf-8");
    assert!(listed.contains("jsonl-integrity"), "{listed}");
}

// ---------------------------------------------------------------------------
// dep add
// ---------------------------------------------------------------------------

#[test]
fn dep_add_writes_both_directions_and_json_shape() {
    let dir = temp_store("dep-add");
    write_records(&dir, &standard());
    let output = run(&dir, &["dep", "add", "tst-0002", "tst-0003", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    assert_eq!(value["command"], json!("dep add"));
    assert_eq!(value["issueId"], json!("tst-0002"));
    assert_eq!(value["dependsOnId"], json!("tst-0003"));

    let records = read_records(&dir);
    let issue = records
        .iter()
        .find(|record| record["id"] == json!("tst-0002"))
        .expect("issue");
    assert_eq!(issue["blockedBy"], json!(["tst-0003"]));
    let dependency = records
        .iter()
        .find(|record| record["id"] == json!("tst-0003"))
        .expect("dependency");
    assert_eq!(dependency["blocks"], json!(["tst-0002"]));

    // tst-0002 is now blocked and drops out of ready.
    let value = stdout_json(&run(&dir, &["ready", "--format", "json"]));
    assert!(!ids_of(&value).contains(&"tst-0002".to_owned()));

    let output = run(&dir, &["dep", "add", "tst-0002", "tst-0003"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert_eq!(text.trim_end(), "Added dependency: tst-0002 → tst-0003");
}

#[test]
fn dep_add_missing_issue_reports_error() {
    let dir = temp_store("dep-missing");
    write_records(&dir, &standard());
    let output = run(&dir, &["dep", "add", "tst-zzzz", "tst-0001", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let value = stdout_json(&output);
    assert_eq!(value["success"], json!(false));
    assert_eq!(value["command"], json!("dep"));
}

// ---------------------------------------------------------------------------
// prime
// ---------------------------------------------------------------------------

#[test]
fn prime_outputs_context_compact_and_json() {
    let dir = temp_store("prime");
    write_records(&dir, &standard());
    let output = run(&dir, &["prime"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(text.starts_with("# Seeds Workflow Context"), "full header");

    let output = run(&dir, &["prime", "--compact"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        text.starts_with("# Seeds Quick Reference"),
        "compact header"
    );

    let value = stdout_json(&run(&dir, &["prime", "--json"]));
    assert_eq!(value["command"], json!("prime"));
    assert_eq!(value["sections"]["mode"], json!("full"));

    // The round-trip pace line (seeds-8fc5): one compact line under
    // the anchor of each text template, one top-level `pace` key in the
    // JSON envelope — no week table, no full metric block.
    for args in [&["prime"][..], &["prime", "--compact"][..]] {
        let output = run(&dir, args);
        assert_eq!(output.status.code(), Some(0), "args: {args:?}");
        let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
        assert!(
            text.contains("> **Pace**: "),
            "pace line in {args:?}: {text}"
        );
        assert!(text.contains("open to target"), "{text}");
        assert!(!text.contains("Week (ISO, UTC)"), "no table: {text}");
    }
    let value = stdout_json(&run(&dir, &["prime", "--json"]));
    let pace = value["pace"].as_str().expect("pace key in envelope");
    assert!(pace.contains("open to target"), "{pace}");
    assert!(pace.contains("ETA"), "{pace}");
    assert!(
        pace.contains("k "),
        "pace coefficient rides the line: {pace}"
    );
    // The content mirror stays the static template (documented shape).
    let content = value["content"].as_str().expect("content mirror");
    assert!(!content.contains("> **Pace**:"), "content stays static");

    // The examples name the live `seeds` CLI (README DEVIATIONS):
    // nothing provides `sd` on PATH since the self-hosting cutover,
    // so an `sd` example would send agents at a missing command.
    let output = run(&dir, &["prime"]);
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        text.contains("Run `seeds prime` after compaction"),
        "{text}"
    );
    assert!(text.contains("`seeds ready`"), "{text}");
    assert!(!text.contains("`sd "), "no retired sd commands: {text}");
    let value = stdout_json(&run(&dir, &["prime", "--json"]));
    assert_eq!(
        value["sections"]["commandGroups"][0]["commands"][0]["command"],
        json!("seeds ready")
    );
}

// ---------------------------------------------------------------------------
// help / version
// ---------------------------------------------------------------------------

#[test]
fn help_and_version_match_reference_surface() {
    let dir = temp_store("help");
    write_records(&dir, &standard());
    let output = run(&dir, &["--version"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    // Version identity (seeds-e160): the real crate version from
    // CARGO_PKG_VERSION, with the sd-parity target on its own line.
    assert!(stdout.contains(concat!("v", env!("CARGO_PKG_VERSION"))));
    assert!(stdout.contains("sd-0.5.15 read+write compatible"));

    for (command, needle) in [
        ("create", "--priority <n>"),
        ("list", "--priority-max <n>"),
        ("ready", "--respect-schedule"),
        ("update", "--clear-extensions"),
        ("update", "--append-description <text>"),
        ("search", "Case-insensitive substring"),
    ] {
        let output = run(&dir, &[command, "--help"]);
        assert_eq!(output.status.code(), Some(0), "{command} --help exits 0");
        assert!(
            String::from_utf8(output.stdout.clone())
                .expect("utf-8")
                .contains(needle),
            "{command} --help mentions {needle}"
        );
    }
}

// ---------------------------------------------------------------------------
// help honesty (seeds-25b5)
// ---------------------------------------------------------------------------

#[test]
fn global_help_lists_only_implemented_commands() {
    let dir = temp_store("help-honesty");
    write_records(&dir, &standard());
    let output = run(&dir, &["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    for command in [
        "create",
        "show",
        "list",
        "ready",
        "search",
        "update",
        "close",
        "dep",
        "prime",
        "dedupe",
        "blocked",
        "block",
        "unblock",
        "label",
        "stats",
        "doctor",
        "init",
        "config",
        "upgrade",
        "onboard",
        "completions",
    ] {
        assert!(text.contains(command), "--help lists {command}");
    }
    // Unimplemented reference commands must not appear as listed
    // commands (only inside the explicit not-implemented note).
    let commands_section = text
        .split("Unimplemented reference commands")
        .next()
        .unwrap_or_default();
    // The one remaining unimplemented reference command (the
    // documented non-goal) must not appear as a listed command.
    assert!(
        !commands_section.contains("migrate-from-beads"),
        "--help must not list 'migrate-from-beads' as a command"
    );
}

#[test]
fn planned_commands_answer_not_implemented_yet() {
    let dir = temp_store("planned");
    write_records(&dir, &standard());
    // The one remaining planned-command holdout answers honestly.
    {
        let output = run(&dir, &["migrate-from-beads"]);
        assert_eq!(output.status.code(), Some(1), "migrate-from-beads exits 1");
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
        assert!(
            stderr.contains("not implemented yet"),
            "migrate-from-beads stderr: {stderr}"
        );
    }
    // The hygiene batch graduated: these must NOT answer
    // not-implemented anymore (help honesty, seeds-c228; plan
    // graduated with the full decomposition surface, seeds-de37;
    // config/init graduated with sd parity, seeds-c813;
    // onboard/completions graduated with sd-parity mechanics,
    // seeds-d9f8; upgrade landed as the real self-update, seeds-bdcb;
    // tpl graduated as the molecules group, seeds-fb5f).
    for command in [
        "blocked",
        "block",
        "unblock",
        "stats",
        "doctor",
        "plan",
        "config",
        "init",
        "onboard",
        "completions",
        "upgrade",
        "tpl",
    ] {
        let output = run(&dir, &[command]);
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
        assert!(
            !stderr.contains("not implemented yet"),
            "{command} is implemented: {stderr}"
        );
    }
    let output = run(&dir, &["label", "list", "tst-0001"]);
    assert_eq!(output.status.code(), Some(0));
    // A genuinely unknown command keeps the generic error.
    let output = run(&dir, &["frobnicate"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
    assert!(stderr.contains("unknown command"), "stderr: {stderr}");
    assert!(!stderr.contains("not implemented yet"));
}

// ---------------------------------------------------------------------------
// hygiene batch deviations (seeds-c228): doctor and stats beyond parity
// ---------------------------------------------------------------------------

/// One deliberate bidirectional mismatch (tst-0006 → tst-0001 without
/// the reverse side) and a consistent closed record — the minimal
/// doctor fixture for the deviation tests.
fn doctor_fixture() -> Vec<Value> {
    vec![
        record(
            "tst-0001",
            "Fix login bug",
            json!({"type": "bug", "priority": 0, "labels": ["bug"], "assignee": "alice"}),
        ),
        record(
            "tst-0004",
            "Old work",
            json!({"status": "closed", "priority": 3, "closedAt": "2026-01-05T00:00:00.000Z"}),
        ),
        record(
            "tst-0006",
            "Blocked work",
            json!({"priority": 1, "blockedBy": ["tst-0001"]}),
        ),
    ]
}

/// `doctor --json` keeps sd's envelope (checks/summary) and adds
/// machine-readable `repair` fields only under `--repair-report` — the
/// documented DEVIATIONS addition.
#[test]
fn doctor_json_is_machine_readable_and_repair_report_names_the_fix() {
    let dir = temp_store("doctor-deviation");
    // The fixture's tst-0006.blockedBy lacks the tst-0001.blocks
    // reverse side — the live bidirectional mismatch class.
    write_records(&dir, &doctor_fixture());

    let plain = run(&dir, &["doctor", "--json"]);
    assert_eq!(plain.status.code(), Some(0));
    let value = stdout_json(&plain);
    assert_eq!(value["success"], json!(true));
    let checks = value["checks"].as_array().expect("checks array");
    let names: Vec<&str> = checks
        .iter()
        .map(|check| check["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec![
        "config",
        "jsonl-integrity",
        "schema-validation",
        "duplicate-ids",
        "referential-integrity",
        "bidirectional-consistency",
        "circular-dependencies",
        "label-schema",
        "extensions-schema",
        "closed-fields-consistency",
        "stale-locks",
        "gitattributes",
    ]);
    // Pure --json: no repair keys (parity surface).
    assert!(checks.iter().all(|check| check.get("repair").is_none()));

    let report = run(&dir, &["doctor", "--repair-report", "--json"]);
    assert_eq!(report.status.code(), Some(0));
    let value = stdout_json(&report);
    let bidirectional = value["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .find(|check| check["name"] == "bidirectional-consistency")
        .expect("bidirectional check")
        .clone();
    assert_eq!(bidirectional["status"], json!("warn"));
    // The repair names the exact fix (the facc/9482 class).
    let repair = bidirectional["repair"].as_str().expect("repair present");
    assert!(
        repair.contains("add \"tst-0006\" to tst-0001.blocks"),
        "repair names the exact fix: {repair}"
    );

    // Text mode: the repair appendix follows the summary.
    let text = run(&dir, &["doctor", "--repair-report"]);
    let stdout = String::from_utf8(text.stdout.clone()).expect("utf-8");
    assert!(stdout.contains("Repairable findings:"));
    assert!(stdout.contains("fix: add \"tst-0006\" to tst-0001.blocks"));
}

/// `doctor --fix` repairs the bidirectional mismatch and creates the
/// merge=union `.gitattributes`; a second run is clean.
#[test]
fn doctor_fix_repairs_mismatches_and_gitattributes() {
    let dir = temp_store("doctor-fix");
    write_records(&dir, &doctor_fixture());

    let output = run(&dir, &["doctor", "--fix"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("✓ Repaired 1 bidirectional dependency mismatch(es)"),
        "fix line: {stdout}"
    );
    assert!(stdout.contains("✓ Created .gitattributes with merge=union entries"));
    let gitattributes =
        fs::read_to_string(dir.join(".gitattributes")).expect(".gitattributes written");
    assert!(gitattributes.contains(".seeds/issues.jsonl merge=union"));

    let again = run(&dir, &["doctor"]);
    assert_eq!(again.status.code(), Some(0));
    let stdout = String::from_utf8(again.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("12 passed, 0 warning(s), 0 failure(s)"),
        "{stdout}"
    );

    // The store repair added the reverse blocks entry.
    let repaired = read_records(&dir);
    let blocker = repaired
        .iter()
        .find(|record| record["id"] == "tst-0001")
        .expect("tst-0001");
    assert_eq!(blocker["blocks"], json!(["tst-0006"]));
}

/// The seeds-cbee fixture: every check that declares `fixable` but
/// had no apply block — a duplicate id pair, a dangling reference,
/// string labels, a non-object extensions, a closed record without
/// `closedAt`, open close metadata, and a stale lock file.
fn doctor_corrupt_fixture() -> Vec<Value> {
    vec![
        record(
            "tst-0001",
            "Survivor",
            json!({
                "status": "closed",
                "updatedAt": "2026-01-02T00:00:00.000Z",
                "labels": "bug, ui, ui",
                "extensions": "not-an-object",
            }),
        ),
        record(
            "tst-0001",
            "Older duplicate",
            json!({"updatedAt": "2026-01-01T00:00:00.000Z"}),
        ),
        record(
            "tst-0002",
            "Dangling ref with open close metadata",
            json!({
                "blockedBy": ["tst-9999", "tst-0001"],
                "closedAt": "2026-01-05T00:00:00.000Z",
                "closeReason": "stale metadata",
            }),
        ),
    ]
}

/// Every `fixable: true` declaration now carries an apply block
/// (seeds-cbee): `doctor --fix` heals duplicates, dangling refs,
/// label/extensions shapes, status/closedAt pairs, and stale locks —
/// a second run is green, the pre-fix `fix:` lines without effect
/// are gone.
#[test]
fn doctor_fix_applies_every_declared_repair() {
    let dir = temp_store("doctor-fix-apply");
    write_records(&dir, &doctor_corrupt_fixture());
    fs::write(dir.join(".seeds/stale.lock"), "").expect("stale lock file");

    // Report-only: the repair report names the repairs that will
    // apply — the same six checks the run heals.
    let report = run(&dir, &["doctor", "--repair-report"]);
    assert_eq!(report.status.code(), Some(1));
    let stdout = String::from_utf8(report.stdout.clone()).expect("utf-8");
    assert!(stdout.contains("fix: keep the newest occurrence per duplicate id"));
    assert!(stdout.contains("fix: remove the dangling references"));
    assert!(stdout.contains("fix: rewrite labels as an array of strings"));
    assert!(stdout.contains("fix: rewrite extensions as a JSON object"));
    assert!(stdout.contains("fix: set closedAt on closed issues, drop it on open ones"));
    assert!(stdout.contains("fix: delete the stale lock file(s)"));

    let output = run(&dir, &["doctor", "--fix"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    for expected in [
        "✓ Deduped 1 duplicate occurrence(s) (newest updatedAt kept)",
        "✓ Removed 1 dangling dependency reference(s)",
        "✓ Repaired 1 bidirectional dependency mismatch(es)",
        "✓ Rewrote 1 invalid label array(s)",
        "✓ Reset 1 invalid extensions field(s) to empty object(s)",
        "✓ Set closedAt on 1 closed issue(s)",
        "✓ Dropped close metadata on 1 open issue(s)",
        "✓ Deleted 1 stale lock file(s)",
        "✓ Created .gitattributes with merge=union entries",
    ] {
        assert!(
            stdout.contains(expected),
            "missing fix line `{expected}`:
{stdout}"
        );
    }

    // The store healed record by record.
    let repaired = read_records(&dir);
    assert_eq!(repaired.len(), 2, "duplicate dropped: {repaired:?}");
    let survivor = repaired
        .iter()
        .find(|record| record["id"] == "tst-0001")
        .expect("survivor");
    assert_eq!(survivor["title"], "Survivor", "newest occurrence won");
    assert_eq!(survivor["labels"], json!(["bug", "ui"]));
    assert_eq!(survivor["extensions"], json!({}));
    assert_eq!(survivor["status"], "closed");
    assert!(
        survivor["closedAt"]
            .as_str()
            .is_some_and(|stamp| !stamp.is_empty()),
        "closedAt set: {survivor}"
    );
    assert_eq!(survivor["blocks"], json!(["tst-0002"]), "reverse ref added");
    let dangling = repaired
        .iter()
        .find(|record| record["id"] == "tst-0002")
        .expect("tst-0002");
    assert_eq!(
        dangling["blockedBy"],
        json!(["tst-0001"]),
        "dangling ref removed"
    );
    assert!(
        dangling.get("closedAt").is_none(),
        "open close metadata dropped"
    );
    assert!(dangling.get("closeReason").is_none());
    assert!(
        !dir.join(".seeds/stale.lock").exists(),
        "stale lock deleted"
    );

    // The acceptance bar: green after one --fix.
    let again = run(&dir, &["doctor"]);
    assert_eq!(again.status.code(), Some(0));
    let stdout = String::from_utf8(again.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("12 passed, 0 warning(s), 0 failure(s)"),
        "{stdout}"
    );
}

/// The label repair repairs the shape, not the spelling: a prose
/// string coerces (case kept, trimmed, deduped), a labels value with
/// nothing to keep drops the field (sd omits empty labels), and
/// mixed arrays are conforming (sd parity: `string_array` reads their
/// string items) — the repair never touches them.
#[test]
fn doctor_fix_label_repair_coerces_prose_drops_junk_keeps_arrays() {
    let dir = temp_store("doctor-label-repair");
    write_records(&dir, &[
        record(
            "tst-0003",
            "Prose labels",
            json!({"labels": " Bug , ui, ui, "}),
        ),
        record("tst-0004", "Number labels", json!({"labels": 42})),
        record(
            "tst-0005",
            "Mixed array",
            json!({"labels": [1, "keep", ""]}),
        ),
    ]);

    let output = run(&dir, &["doctor", "--fix"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("✓ Rewrote 2 invalid label array(s)"),
        "{stdout}"
    );

    let repaired = read_records(&dir);
    let prose = repaired
        .iter()
        .find(|record| record["id"] == "tst-0003")
        .expect("tst-0003");
    assert_eq!(
        prose["labels"],
        json!(["Bug", "ui"]),
        "trimmed, deduped, case kept"
    );
    let number = repaired
        .iter()
        .find(|record| record["id"] == "tst-0004")
        .expect("tst-0004");
    assert!(number.get("labels").is_none(), "junk type drops the field");
    let mixed = repaired
        .iter()
        .find(|record| record["id"] == "tst-0005")
        .expect("tst-0005");
    assert_eq!(
        mixed["labels"],
        json!([1, "keep", ""]),
        "mixed arrays conform (parity) and stay untouched"
    );

    let again = run(&dir, &["doctor"]);
    assert_eq!(again.status.code(), Some(0));
}

/// A malformed JSONL line is a doctor FAILURE (non-zero exit), keeping
/// sd's check name and exit semantics (detail wording is the
/// documented Rust-side divergence).
#[test]
fn doctor_fails_on_malformed_jsonl() {
    let dir = temp_store("doctor-fail");
    write_records(&dir, &standard());
    let mut text = fs::read_to_string(dir.join(".seeds/issues.jsonl")).expect("issues.jsonl");
    text.push_str("not json at all\n");
    fs::write(dir.join(".seeds/issues.jsonl"), text).expect("append malformed line");

    let output = run(&dir, &["doctor"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("✗ 1 malformed line(s) in JSONL files"),
        "{stdout}"
    );
    assert!(stdout.contains("1 failure(s)"), "{stdout}");
}

/// write-safety (seeds-36cd, native addition): the writer is atomic
/// (temp sibling, then rename) under the advisory writer lock, so the
/// two observable signatures of a lost or concurrent write are a
/// stray `.*.tmp` sibling and a held `.lock`. The check is OMITTED
/// when neither is present — a healthy store keeps the reference's
/// check list and pass counts — and `--fix` deletes the interrupted
/// write's leftover.
#[test]
fn doctor_write_safety_reports_interrupted_writes() {
    let dir = temp_store("doctor-write-safety");
    write_records(&dir, &doctor_fixture());
    // Heal the fixture first (bidirectional repair + .gitattributes):
    // the baseline for this check is the fully healthy store.
    let healed = run(&dir, &["doctor", "--fix"]);
    assert_eq!(healed.status.code(), Some(0));

    // Healthy: nothing to report, so the check does not exist — the
    // reference's fixed list keeps its counts and its names.
    let output = run(&dir, &["doctor"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("12 passed, 0 warning(s), 0 failure(s)"),
        "healthy store keeps the reference's counts: {stdout}"
    );
    let ids = run(&dir, &["doctor", "--format", "ids"]);
    assert!(
        String::from_utf8(ids.stdout)
            .expect("utf-8")
            .trim()
            .is_empty(),
        "no check names on a healthy store"
    );

    // A writer that died between writing the temp sibling and renaming
    // it leaves exactly that sibling behind (seeds-36cd incident).
    fs::write(dir.join(".seeds/.issues.jsonl.tmp"), "{}\n").expect("temp sibling");
    let output = run(&dir, &["doctor"]);
    assert_eq!(output.status.code(), Some(0), "a stray temp is a warning");
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("1 interrupted-write temp file(s)"),
        "{stdout}"
    );
    assert!(
        stdout.contains(".issues.jsonl.tmp — interrupted write"),
        "{stdout}"
    );

    // The check NAME is a machine surface only.
    let ids = run(&dir, &["doctor", "--format", "ids"]);
    assert_eq!(
        String::from_utf8(ids.stdout).expect("utf-8"),
        "write-safety\n"
    );

    // --fix deletes the leftover; the store file itself is the last
    // complete state and stays untouched.
    let output = run(&dir, &["doctor", "--fix"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(
        stdout.contains("✓ Deleted 1 interrupted-write temp file(s)"),
        "{stdout}"
    );
    assert!(!dir.join(".seeds/.issues.jsonl.tmp").exists());
    assert_eq!(read_records(&dir).len(), 3, "the store is untouched");
    let again = run(&dir, &["doctor"]);
    let stdout = String::from_utf8(again.stdout).expect("utf-8");
    assert!(
        stdout.contains("12 passed, 0 warning(s), 0 failure(s)"),
        "quiet again after the fix: {stdout}"
    );
}

/// `stats --json` emits the stable key set (documented DEVIATIONS
/// addition; the parity shape is differentially pinned).
#[test]
fn stats_json_has_stable_keys() {
    let dir = temp_store("stats-keys");
    write_records(&dir, &standard());
    let output = run(&dir, &["stats", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let value = stdout_json(&output);
    let stats = &value["stats"];
    for key in [
        "total",
        "open",
        "inProgress",
        "closed",
        "blocked",
        "byType",
        "byPriority",
        "byLabel",
    ] {
        assert!(stats.get(key).is_some(), "stats.{key} present");
    }
    assert_eq!(stats["total"], json!(6));
    assert_eq!(stats["open"], json!(4));
    assert_eq!(stats["inProgress"], json!(1));
    assert_eq!(stats["closed"], json!(1));
    assert_eq!(stats["blocked"], json!(1));
}

/// `stats --pace` renders the pace section (seeds-8fc5): the JSON
/// contract keys next to the parity stats, an honest null ETA when the
/// window carries no closures, and the week table in text mode.
#[test]
fn stats_pace_renders_section_and_contract() {
    let dir = temp_store("stats-pace");
    write_records(&dir, &standard());
    let output = run(&dir, &["stats", "--pace"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(text.starts_with("Project Statistics\n"), "{text}");
    assert!(text.contains("\nPace\n"), "{text}");
    assert!(text.contains("Rate (30d):"), "{text}");
    assert!(text.contains("Pace k:"), "{text}");
    assert!(text.contains("Lead time (median):"), "{text}");
    assert!(text.contains("Open age (median):"), "{text}");
    assert!(text.contains("Open to target:"), "{text}");
    assert!(text.contains("Week (ISO, UTC)"), "{text}");

    let value = stdout_json(&run(&dir, &["stats", "--pace", "--json"]));
    let pace = &value["pace"];
    for key in [
        "window_days",
        "min_closures",
        "rate_window",
        "closures_in_window",
        "creations_in_window",
        "k",
        "drain_rate_per_week",
        "rate_all_time",
        "closures_total",
        "history_weeks",
        "lead_time_median_hours",
        "open_age_median_days",
        "open_to_target",
        "excluded_open",
        "exclusion_labels",
        "eta",
        "eta_reason",
        "eta_basis",
        "weeks",
    ] {
        assert!(pace.get(key).is_some(), "pace.{key} present");
    }
    // The fixture's only close carries no closedAt: zero windowed
    // closures → a null ETA that says why instead of guessing.
    assert_eq!(pace["closures_in_window"], json!(0));
    assert_eq!(pace["eta"], Value::Null);
    assert_eq!(pace["eta_reason"], "zero_closures_in_window");
    // The parity stats ride along unchanged.
    assert_eq!(value["stats"]["total"], json!(6));
}

/// Any pace option implies `--pace`; garbage values name the flag on
/// stderr; the help surface documents the whole group.
#[test]
fn stats_pace_options_imply_section_and_garbage_is_named() {
    let dir = temp_store("stats-pace-flags");
    write_records(&dir, &standard());
    let output = run(&dir, &["stats", "--window", "7"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(text.contains("Rate (7d):"), "{text}");

    for bad_args in [
        &["stats", "--window", "soon"][..],
        &["stats", "--window", "0"][..],
        &["stats", "--min-closures", "many"][..],
    ] {
        let bad = run(&dir, bad_args);
        assert_eq!(bad.status.code(), Some(1), "args: {bad_args:?}");
        let stderr = String::from_utf8(bad.stderr.clone()).expect("utf-8");
        let flag = bad_args[1];
        assert!(stderr.contains(flag), "stderr names {flag}: {stderr}");
    }

    let help = run(&dir, &["stats", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    let text = String::from_utf8(help.stdout.clone()).expect("utf-8");
    for flag in [
        "--pace",
        "--window",
        "--min-closures",
        "--exclude-label",
        "--since",
    ] {
        assert!(text.contains(flag), "help lists {flag}");
    }
}

// ---------------------------------------------------------------------------
// stats/list display deviations (seeds-4cf6)
// ---------------------------------------------------------------------------

/// stats text splits `By Priority`/`By Label` into `N open / M closed`
/// (README DEVIATIONS) while JSON keeps the reference's all-status
/// totals.
#[test]
fn stats_text_splits_open_and_closed() {
    let dir = temp_store("stats-split");
    write_records(&dir, &standard());
    let output = run(&dir, &["stats"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).expect("utf-8");
    // P3 carries only the closed tst-0004; the bug label spans
    // tst-0001 (open) + tst-0003 (in_progress), nothing closed.
    assert!(text.contains("0 open / 1 closed"), "closed row: {text}");
    assert!(text.contains("2 open / 0 closed"), "live rows: {text}");
    assert!(
        text.contains("1 open / 0 closed"),
        "single live row: {text}"
    );

    let value = stdout_json(&run(&dir, &["stats", "--json"]));
    assert_eq!(value["stats"]["byLabel"]["bug"], json!(2));
    assert_eq!(value["stats"]["byPriority"]["3"], json!(1));
}

/// The query footers name the `--limit` cap (README DEVIATIONS):
/// `shown of total` when the cap cut rows, the reference's plain
/// count otherwise — shared by list/ready/search through the one
/// pipeline.
#[test]
fn list_footer_names_the_limit_cap() {
    let dir = temp_store("limit-cap");
    write_records(&dir, &standard());
    // Default list scope: five non-closed records; limit 2 truncates.
    let output = run(&dir, &["list", "--limit", "2", "--sort", "id"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).expect("utf-8");
    assert!(text.contains("\n2 of 5 issue(s)"), "{text}");

    // Without truncation the footer keeps the reference wording.
    let output = run(&dir, &["list", "--sort", "id"]);
    let text = String::from_utf8(output.stdout).expect("utf-8");
    assert!(text.contains("\n5 issue(s)"), "{text}");
    assert!(
        !text.contains(" of "),
        "plain count when untruncated: {text}"
    );

    // ready shares the pipeline and names its own cap.
    let output = run(&dir, &["ready", "--limit", "1"]);
    let text = String::from_utf8(output.stdout).expect("utf-8");
    assert!(text.contains("\n1 of 3 ready issue(s)"), "{text}");
}

// ---------------------------------------------------------------------------
// onboard (seeds-d9f8)
// ---------------------------------------------------------------------------

fn onboard_section(dir: &Path, file: &str) -> String {
    fs::read_to_string(dir.join(file)).expect("agent doc")
}

#[test]
fn onboard_creates_claude_md_with_marked_section() {
    let dir = temp_store("onboard_create");
    write_records(&dir, &standard());
    let output = run(&dir, &["onboard"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(stdout.contains("✓ Created"), "created message: {stdout}");
    let doc = onboard_section(&dir, "CLAUDE.md");
    assert!(doc.contains("<!-- seeds:start -->"));
    assert!(doc.contains("<!-- seeds:end -->"));
    // Version identity: the section carries the real crate version and
    // the schema comment that drives update detection.
    assert!(doc.contains(&format!(
        "<!-- seeds-onboard:v{} -->",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(doc.contains("<!-- seeds-onboard-schema:7 -->"));
    // Naming deviation (README DEVIATIONS): the live CLI is `seeds`.
    assert!(doc.contains("```\nseeds prime\n```"));
    assert!(!doc.contains("`sd "));
}

#[test]
fn onboard_is_idempotent_and_reports_unchanged() {
    let dir = temp_store("onboard_twice");
    write_records(&dir, &standard());
    run(&dir, &["onboard"]);
    let output = run(&dir, &["onboard"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(stdout.contains("already up to date"));
    let json = stdout_json(&run(&dir, &["onboard", "--json"]));
    assert_eq!(json["action"], json!("unchanged"));
    assert_eq!(json["command"], json!("onboard"));
}

#[test]
fn onboard_appends_to_existing_doc_and_falls_back_to_agents_md() {
    let dir = temp_store("onboard_append");
    write_records(&dir, &standard());
    fs::write(dir.join("AGENTS.md"), "existing docs\n").expect("AGENTS.md");
    let output = run(&dir, &["onboard"]);
    assert_eq!(output.status.code(), Some(0));
    let doc = onboard_section(&dir, "AGENTS.md");
    assert!(doc.starts_with("existing docs"));
    assert!(doc.contains("<!-- seeds:start -->"));
    let json = stdout_json(&run(&dir, &["onboard", "--json"]));
    assert_eq!(json["action"], json!("unchanged"));
    let path = json["file"].as_str().expect("file").to_owned();
    assert!(
        path.ends_with("AGENTS.md"),
        "file targets AGENTS.md: {path}"
    );
}

#[test]
fn onboard_updates_stale_schema_and_checks_status() {
    let dir = temp_store("onboard_update");
    write_records(&dir, &standard());
    run(&dir, &["onboard"]);
    let doc_path = dir.join("CLAUDE.md");
    let stale = onboard_section(&dir, "CLAUDE.md").replace("schema:7", "schema:6");
    fs::write(&doc_path, stale).expect("rewrite");

    let check = run(&dir, &["onboard", "--check"]);
    assert!(
        String::from_utf8(check.stdout.clone())
            .expect("utf-8")
            .contains("Status: outdated")
    );
    let check_json = stdout_json(&run(&dir, &["onboard", "--check", "--json"]));
    assert_eq!(check_json["status"], json!("outdated"));

    let update = run(&dir, &["onboard"]);
    assert!(
        String::from_utf8(update.stdout.clone())
            .expect("utf-8")
            .contains("✓ Updated seeds section in")
    );
    assert!(onboard_section(&dir, "CLAUDE.md").contains("schema:7"));
    let json = stdout_json(&run(&dir, &["onboard", "--json"]));
    assert_eq!(json["action"], json!("unchanged"));
}

#[test]
fn onboard_check_missing_reports_and_exits_zero() {
    let dir = temp_store("onboard_missing");
    write_records(&dir, &standard());
    let output = run(&dir, &["onboard", "--check"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(stdout.contains("Status: missing (no CLAUDE.md found)"));
    let json = stdout_json(&run(&dir, &["onboard", "--check", "--json"]));
    assert_eq!(json["status"], json!("missing"));
    assert!(json["file"].is_null());
}

#[test]
fn onboard_stdout_prints_section_without_writing() {
    let dir = temp_store("onboard_stdout");
    write_records(&dir, &standard());
    let output = run(&dir, &["onboard", "--stdout"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf-8");
    assert!(stdout.contains("<!-- seeds:start -->"));
    assert!(!dir.join("CLAUDE.md").exists());
    assert!(!dir.join("AGENTS.md").exists());
}

// ---------------------------------------------------------------------------
// completions (seeds-d9f8)
// ---------------------------------------------------------------------------

#[test]
fn completions_emit_the_implemented_surface_per_shell() {
    let dir = temp_store("completions");
    write_records(&dir, &standard());
    for shell in ["bash", "zsh", "fish"] {
        let output = run(&dir, &["completions", shell]);
        assert_eq!(output.status.code(), Some(0), "{shell} exits 0");
        let script = String::from_utf8(output.stdout.clone()).expect("utf-8");
        for command in [
            "create",
            "show",
            "list",
            "ready",
            "update",
            "close",
            "dep",
            "blocked",
            "block",
            "unblock",
            "label",
            "stats",
            "doctor",
            "prime",
            "search",
            "dedupe",
            "sync",
            "plan",
            "init",
            "config",
            "upgrade",
            "onboard",
            "completions",
        ] {
            assert!(script.contains(command), "{shell} lists {command}");
        }
        // Help honesty (seeds-25b5): the retired reference surface stays
        // out of the completions.
        assert!(!script.contains("migrate-from-beads"));
        assert!(script.contains("seeds"), "{shell} names the seeds binary");
    }
    // Group subcommands are completable.
    let bash = String::from_utf8(run(&dir, &["completions", "bash"]).stdout).expect("utf-8");
    assert!(bash.contains("add remove list"));
}

#[test]
fn completions_reject_unknown_shell_and_missing_argument() {
    let dir = temp_store("completions_bad");
    write_records(&dir, &standard());
    let unknown = run(&dir, &["completions", "tcsh"]);
    assert_eq!(unknown.status.code(), Some(1));
    let stderr = String::from_utf8(unknown.stderr.clone()).expect("utf-8");
    assert_eq!(
        stderr.trim(),
        "✗ Unknown shell: tcsh. Supported: bash, zsh, fish"
    );
    assert_eq!(unknown.stdout.len(), 0);

    let missing = run(&dir, &["completions"]);
    assert_eq!(missing.status.code(), Some(1));
    let stderr = String::from_utf8(missing.stderr.clone()).expect("utf-8");
    assert_eq!(stderr.trim(), "error: missing required argument 'shell'");
}
