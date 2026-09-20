//! Byte-fidelity round-trip property tests: records carrying unknown
//! (additive) fields must survive load→save intact, including ordering.

use std::fs;
use std::path::PathBuf;

use seeds::{Fields, SeedRecord, Status, Store};
use serde_json::{Value, json};

fn temp_store(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("seeds-roundtrip-{tag}-{}", std::process::id()));
    let seeds_dir = dir.join(".seeds");
    fs::create_dir_all(&seeds_dir).expect("create temp store");
    fs::write(
        seeds_dir.join("config.yaml"),
        "project: \"demo\"\nversion: \"1\"\nmax_plan_depth: 3\n",
    )
    .expect("write config");
    seeds_dir
}

/// Injected unknown-field payloads: additive extension shapes the format
/// must tolerate.
fn unknown_payloads() -> Vec<(&'static str, Value)> {
    vec![
        ("x_string", json!("plain")),
        ("x_unicode", json!("ä — 🌱")),
        ("x_int", json!(42)),
        ("x_float", json!(1.25)),
        ("x_bool", json!(false)),
        ("x_null", Value::Null),
        ("x_array", json!([1, "two", {"three": 3}])),
        (
            "x_object",
            json!({"nested": {"deep": [true, null]}, "k": "v"}),
        ),
    ]
}

#[test]
fn unknown_fields_round_trip_byte_for_byte() {
    for (name, payload) in unknown_payloads() {
        let root = temp_store(&format!("byte-{name}"));
        let mut fields = Fields::new();
        fields.insert("id".to_owned(), json!("demo-a1b2"));
        fields.insert("title".to_owned(), json!("Carrier"));
        fields.insert("status".to_owned(), json!("open"));
        fields.insert(name.to_owned(), payload);
        let line = SeedRecord::try_from_fields(fields)
            .expect("valid record")
            .to_json_line();

        let before = format!("{line}\n");
        fs::write(root.join("issues.jsonl"), &before).expect("write issues");

        let store = Store::open(&root).expect("load store");
        store.save().expect("save store");
        let after = fs::read_to_string(root.join("issues.jsonl")).expect("read issues");

        assert_eq!(before, after, "unknown field {name} must round-trip intact");
        fs::remove_dir_all(root.parent().expect("parent")).ok();
    }
}

#[test]
fn unknown_fields_survive_read_modify_write() {
    let root = temp_store("rmw");
    let mut fields = Fields::new();
    fields.insert("id".to_owned(), json!("demo-c3d4"));
    fields.insert("title".to_owned(), json!("Mutable"));
    fields.insert("status".to_owned(), json!("open"));
    fields.insert("priority".to_owned(), json!(2));
    fields.insert(
        "x_custom".to_owned(),
        json!({"keep": ["me"], "deep": {"n": 1}}),
    );
    fs::write(
        root.join("issues.jsonl"),
        format!(
            "{}\n",
            SeedRecord::try_from_fields(fields)
                .expect("valid record")
                .to_json_line()
        ),
    )
    .expect("write issues");

    let mut store = Store::open(&root).expect("load store");
    let record = store.issue_mut("demo-c3d4").expect("record by id");
    record.set_status(Status::InProgress);
    record.set_priority(0).expect("valid priority");
    record.set_labels(["loop"]);
    store.save().expect("save store");

    let store = Store::open(&root).expect("reload store");
    let record = store.issue("demo-c3d4").expect("record by id");
    assert_eq!(record.status(), Some(Status::InProgress));
    assert_eq!(record.priority().map(seeds::Priority::get), Some(0));
    assert_eq!(record.labels(), vec!["loop"]);
    assert_eq!(
        record.field("x_custom"),
        Some(&json!({"keep": ["me"], "deep": {"n": 1}})),
        "unknown fields must survive read-modify-write"
    );
    fs::remove_dir_all(root.parent().expect("parent")).ok();
}

#[test]
fn store_round_trips_this_repo_layout() {
    // Synthetic store mirroring this repo: empty plans/templates files
    // must save back as empty files, and a plan/template record with
    // unknown fields must survive like issues do.
    let root = temp_store("layout");
    fs::write(root.join("issues.jsonl"), "").expect("empty issues");
    let mut plan = Fields::new();
    plan.insert("id".to_owned(), json!("pl-3ad6"));
    plan.insert("seed".to_owned(), json!("demo-e5f6"));
    plan.insert("status".to_owned(), json!("approved"));
    plan.insert("x_extra".to_owned(), json!("kept"));
    let plan_line = seeds::PlanRecord::try_from_fields(plan)
        .expect("valid plan")
        .to_json_line();
    fs::write(root.join("plans.jsonl"), format!("{plan_line}\n")).expect("write plans");
    let mut template = Fields::new();
    template.insert("id".to_owned(), json!("tpl-5643"));
    template.insert("name".to_owned(), json!("review"));
    template.insert("steps".to_owned(), json!([{"title": "Do it"}]));
    let template_line = seeds::TemplateRecord::try_from_fields(template)
        .expect("valid template")
        .to_json_line();
    fs::write(root.join("templates.jsonl"), format!("{template_line}\n")).expect("write templates");

    let store = Store::open(&root).expect("load store");
    store.save().expect("save store");

    assert_eq!(
        fs::read_to_string(root.join("issues.jsonl")).expect("read issues"),
        "",
        "empty JSONL saves as an empty file"
    );
    assert_eq!(
        fs::read_to_string(root.join("plans.jsonl")).expect("read plans"),
        format!("{plan_line}\n")
    );
    assert_eq!(
        fs::read_to_string(root.join("templates.jsonl")).expect("read templates"),
        format!("{template_line}\n")
    );
    fs::remove_dir_all(root.parent().expect("parent")).ok();
}
