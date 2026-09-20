//! Real-fixture agreement: our reader and `sd` must agree on every record
//! of this repository's own tracker (`.seeds/issues.jsonl`).
//!
//! The fixture (`fixtures/repo_issues_sd_list.json`) is a snapshot of
//! `sd list --all --format json --limit 200` over the same file, generated
//! by the fixture step; the test compares field-by-field.

use std::path::PathBuf;

use seeds::Store;
use serde_json::Value;

const SD_LIST_SNAPSHOT: &str = include_str!("fixtures/repo_issues_sd_list.json");

fn repo_seeds_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.seeds")
        .canonicalize()
        .expect("repo .seeds/ directory")
}

#[test]
fn our_reader_agrees_with_sd_on_every_repo_record() {
    let store = Store::open(repo_seeds_dir()).expect("read this repo's tracker");
    assert!(
        !store.issues.is_empty(),
        "the repo tracker must have records to compare"
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
