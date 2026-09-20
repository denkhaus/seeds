//! Real-fixture agreement: our reader and `sd` must agree on every record
//! of this repository's own tracker format.
//!
//! Hermetic fixture pair (run review 01M2ZYVQ, finding #2): the tracker
//! copy (`fixtures/repo_seeds/`) and the `sd list --all --format json
//! --limit 200` snapshot (`fixtures/repo_issues_sd_list.json`) are
//! co-captured at one point in time, so live-tracker drift (claim/close
//! timestamps, status transitions, record-count growth) can never break
//! the test. The test materializes the captured store into a temp dir
//! and compares field-by-field.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, process};

use seeds::Store;
use serde_json::Value;

const SD_LIST_SNAPSHOT: &str = include_str!("fixtures/repo_issues_sd_list.json");
const ISSUES_JSONL: &str = include_str!("fixtures/repo_seeds/issues.jsonl");
const CONFIG_YAML: &str = include_str!("fixtures/repo_seeds/config.yaml");

/// Materializes the captured tracker into a fresh temp `.seeds/` dir.
fn captured_seeds_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("seeds-real-fixture-{}-{unique}", process::id()));
    fs::create_dir_all(&dir).expect("create temp .seeds dir");
    fs::write(dir.join("issues.jsonl"), ISSUES_JSONL).expect("write captured issues.jsonl");
    fs::write(dir.join("config.yaml"), CONFIG_YAML).expect("write captured config.yaml");
    dir
}

#[test]
fn our_reader_agrees_with_sd_on_every_repo_record() {
    // The fixture pair is co-captured, so even volatile fields agree at
    // capture time; the stable-projection skip is retained so a future
    // regeneration of one half alone (snapshot or tracker copy) still
    // compares safely. blockedBy/closedAt drift with lifecycle too:
    // closing a blocker removes its dep edges from the file (sd close
    // housekeeping).
    const VOLATILE_FIELDS: &[&str] = &[
        "updatedAt",
        "createdAt",
        "status",
        "assignee",
        "blockedBy",
        "closedAt",
    ];

    let store = Store::open(captured_seeds_dir()).expect("read the captured tracker");
    assert!(
        !store.issues.is_empty(),
        "the captured tracker must have records to compare"
    );

    let snapshot: Value = serde_json::from_str(SD_LIST_SNAPSHOT).expect("valid snapshot JSON");
    let reported = snapshot["issues"]
        .as_array()
        .expect("sd list JSON shape {success, command, issues}");
    assert_eq!(
        reported.len(),
        store.issues.len(),
        "sd and our reader must see the same number of records"
    );

    for issue in reported {
        let id = issue["id"].as_str().expect("sd always writes id");
        let record = store
            .issue(id)
            .unwrap_or_else(|| panic!("record {id} visible to sd but not to our reader"));
        for (key, expected) in issue.as_object().expect("issue is an object") {
            if VOLATILE_FIELDS.contains(&key.as_str()) {
                continue;
            }
            if let Some(actual) = record.field(key) {
                assert_eq!(
                    actual, expected,
                    "field {key} of {id} differs between sd and our reader"
                );
            } else {
                panic!("field {key} of {id} missing from our record");
            }
        }
    }
}
