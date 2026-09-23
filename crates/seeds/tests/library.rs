//! Library API suite: `seeds::commands` as an embedded (compile-in)
//! consumer uses it — typed inputs, explicit store path, outcome
//! strings instead of process exits (seeds-0dfd).

use std::fs;
use std::path::PathBuf;

use seeds::commands::{
    self, CommandContext, CreateInput, DepAddInput, QueryCommand, QueryInput, ShowInput,
    UpdateInput,
};
use serde_json::Value;

fn temp_store(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("seeds-lib-{}-{tag}-{}", std::process::id(), tag));
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
    dir.join(".seeds")
}

/// Creates one seed and returns its id.
fn create_seed(ctx: &CommandContext, title: &str) -> String {
    let outcome = commands::create(ctx, &CreateInput {
        title: Some(title.to_owned()),
        json: true,
        ..Default::default()
    });
    assert!(outcome.success, "create failed: {}", outcome.stderr);
    let envelope: Value = serde_json::from_str(&outcome.stdout).expect("create emits JSON");
    envelope["id"]
        .as_str()
        .expect("create envelope carries id")
        .to_owned()
}

#[test]
fn typed_inputs_produce_the_reference_envelopes() {
    let dir = temp_store("envelopes");
    let ctx = CommandContext::at(&dir);

    // create → {success, command, id}
    let id = create_seed(&ctx, "Alpha");
    let outcome = commands::create(&ctx, &CreateInput {
        title: Some("Beta".to_owned()),
        kind: Some("bug".to_owned()),
        labels: Some("x,y".to_owned()),
        json: true,
        ..Default::default()
    });
    let envelope: Value = serde_json::from_str(&outcome.stdout).expect("envelope JSON");
    assert_eq!(envelope["success"], true);
    assert_eq!(envelope["command"], "create");
    let beta_id = envelope["id"].as_str().expect("id").to_owned();

    // show → {success, command, issue} with the canonical projection.
    let outcome = commands::show(&ctx, &ShowInput {
        ids: vec![id.clone()],
        json: true,
        ..Default::default()
    });
    let envelope: Value = serde_json::from_str(&outcome.stdout).expect("envelope JSON");
    assert_eq!(envelope["issue"]["title"], "Alpha");
    assert_eq!(envelope["issue"]["status"], "open");

    // dep add blocks ready until closed.
    let outcome = commands::dep_add(&ctx, &DepAddInput {
        ids:  vec![id.clone(), beta_id.clone()],
        json: true,
    });
    assert!(outcome.success);
    let outcome = commands::ready(&ctx, &QueryInput {
        command: Some(QueryCommand::Ready),
        json: true,
        ..Default::default()
    });
    let envelope: Value = serde_json::from_str(&outcome.stdout).expect("envelope JSON");
    let ready_ids: Vec<&str> = envelope["issues"]
        .as_array()
        .expect("issues array")
        .iter()
        .map(|issue| issue["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ready_ids,
        vec![beta_id.as_str()],
        "only the blocker is ready"
    );

    // close the blocker, then ready sees the issue.
    let outcome = commands::close(&ctx, &commands::CloseInput {
        ids: vec![beta_id],
        json: true,
        ..Default::default()
    });
    assert!(outcome.success);
    let outcome = commands::ready(&ctx, &QueryInput {
        command: Some(QueryCommand::Ready),
        json: true,
        ..Default::default()
    });
    let envelope: Value = serde_json::from_str(&outcome.stdout).expect("envelope JSON");
    let ready_ids: Vec<&str> = envelope["issues"]
        .as_array()
        .expect("issues array")
        .iter()
        .map(|issue| issue["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ready_ids, vec![id.as_str()], "the unblocked issue is ready");

    // update mutates and returns the updated projection.
    let outcome = commands::update(&ctx, &UpdateInput {
        id: Some(id.clone()),
        status: Some("in_progress".to_owned()),
        json: true,
        ..Default::default()
    });
    let envelope: Value = serde_json::from_str(&outcome.stdout).expect("envelope JSON");
    assert_eq!(envelope["issue"]["status"], "in_progress");
}

#[test]
fn library_failures_are_outcomes_not_exits() {
    let dir = temp_store("failures");
    let ctx = CommandContext::at(&dir);

    // A missing target is a JSON failure envelope with success:false —
    // no process exit, no panic.
    let outcome = commands::show(&ctx, &ShowInput {
        ids: vec!["tst-zzzz".to_owned()],
        json: true,
        json_flag: true,
        ..Default::default()
    });
    assert!(!outcome.success);
    let envelope: Value = serde_json::from_str(&outcome.stdout).expect("failure envelope JSON");
    assert_eq!(envelope["success"], false);
    assert_eq!(envelope["command"], "show");
    assert_eq!(
        envelope["error"].as_str().expect("error text"),
        "Issue not found: tst-zzzz"
    );

    // A missing title keeps the reference's stderr usage error shape.
    let outcome = commands::create(&ctx, &CreateInput::default());
    assert!(!outcome.success);
    assert_eq!(
        outcome.stderr.trim(),
        "error: required option '--title <text>' not specified"
    );
    assert_eq!(outcome.stdout, "");

    // An explicit context isolates the caller from the cwd: a bogus
    // directory reports the store-open failure instead of walking up.
    let nowhere = CommandContext::at(dir.join("nowhere"));
    let outcome = commands::ready(&nowhere, &QueryInput {
        command: Some(QueryCommand::Ready),
        json: true,
        ..Default::default()
    });
    assert!(!outcome.success);
    assert!(outcome.stdout.contains("\"command\": \"ready\""));
}
