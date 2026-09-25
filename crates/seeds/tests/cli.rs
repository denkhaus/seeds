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
        "--desc",
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
fn update_takes_no_format_flag_and_reports_missing_issue() {
    let dir = temp_store("update-flags");
    write_records(&dir, &standard());
    // The reference's update has no --format: unknown option, exit 1.
    let output = run(&dir, &["update", "tst-0002", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));

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
    assert!(
        String::from_utf8(output.stdout.clone())
            .expect("utf-8")
            .contains("v0.5.15")
    );

    for (command, needle) in [
        ("create", "--priority <n>"),
        ("list", "--priority-max <n>"),
        ("ready", "--respect-schedule"),
        ("update", "--clear-extensions"),
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
        "create", "show", "list", "ready", "search", "update", "close", "dep", "prime", "dedupe",
        "blocked", "block", "unblock", "label", "stats", "doctor",
    ] {
        assert!(text.contains(command), "--help lists {command}");
    }
    // Unimplemented reference commands must not appear as listed
    // commands (only inside the explicit not-implemented note).
    let commands_section = text
        .split("Unimplemented reference commands")
        .next()
        .unwrap_or_default();
    for absent in ["tpl", "onboard", "migrate-from-beads"] {
        assert!(
            !commands_section.contains(absent),
            "--help must not list unimplemented '{absent}' as a command"
        );
    }
}

#[test]
fn planned_commands_answer_not_implemented_yet() {
    let dir = temp_store("planned");
    write_records(&dir, &standard());
    for command in ["tpl", "upgrade", "completions"] {
        let output = run(&dir, &[command]);
        assert_eq!(output.status.code(), Some(1), "{command} exits 1");
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf-8");
        assert!(
            stderr.contains("not implemented yet"),
            "{command} stderr: {stderr}"
        );
    }
    // The hygiene batch graduated: these must NOT answer
    // not-implemented anymore (help honesty, seeds-c228; plan
    // graduated with the full decomposition surface, seeds-de37;
    // config/init graduated with sd parity, seeds-c813).
    for command in [
        "blocked", "block", "unblock", "stats", "doctor", "plan", "config", "init",
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
