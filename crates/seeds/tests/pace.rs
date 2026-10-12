//! Golden coverage for the pace section (seeds-8fc5): the `stats
//! --pace` JSON contract and text shape against a pinned clock.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use seeds::PaceOptions;
use seeds::commands::{self, CommandContext, StatsInput};
use serde_json::Value;

/// The pinned clock for every golden in this file.
const NOW: &str = "2026-02-01T00:00:00.000Z";

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_store(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "seeds-pace-{}-{tag}-{}",
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
    fs::write(seeds.join("plans.jsonl"), "").expect("plans.jsonl");
    fs::write(seeds.join("templates.jsonl"), "").expect("templates.jsonl");
    dir
}

/// The six-record fixture: a/b/c close inside the 30d window, d closed
/// before it, e/f stay open (f parked on `backlog`).
fn write_fixture(dir: &Path) {
    let record = |id: &str, created: &str, closed: Option<&str>, labels: &[&str]| {
        let mut value = serde_json::json!({
            "id": id,
            "title": format!("seed {id}"),
            "createdAt": format!("{created}T00:00:00.000Z"),
            "updatedAt": format!("{created}T00:00:00.000Z"),
            "priority": 2,
        });
        let Value::Object(map) = &mut value else {
            unreachable!("record is an object");
        };
        match closed {
            Some(day) => {
                map.insert("status".to_owned(), serde_json::json!("closed"));
                map.insert(
                    "closedAt".to_owned(),
                    serde_json::json!(format!("{day}T00:00:00.000Z")),
                );
            }
            None => {
                map.insert("status".to_owned(), serde_json::json!("open"));
            }
        }
        if !labels.is_empty() {
            map.insert("labels".to_owned(), serde_json::json!(labels));
        }
        value
    };
    let records = [
        record("tst-0001", "2026-01-05", Some("2026-01-06"), &[]),
        record("tst-0002", "2026-01-10", Some("2026-01-12"), &[]),
        record("tst-0003", "2026-01-20", Some("2026-01-25"), &[]),
        record("tst-0004", "2025-12-20", Some("2025-12-30"), &[]),
        record("tst-0005", "2026-01-15", None, &[]),
        record("tst-0006", "2026-01-16", None, &["backlog"]),
    ];
    let mut text = String::new();
    for record in records {
        text.push_str(&record.to_string());
        text.push('\n');
    }
    fs::write(dir.join(".seeds/issues.jsonl"), text).expect("issues.jsonl");
}

fn pace_stats(dir: &Path, options: PaceOptions) -> String {
    let ctx = CommandContext::at(dir.join(".seeds"));
    let outcome = commands::stats(&ctx, &StatsInput {
        json: true,
        pace: Some(options),
    });
    assert!(outcome.success, "stats failed: {}", outcome.stderr);
    outcome.stdout
}

#[test]
fn pace_json_is_the_pinned_golden() {
    let dir = temp_store("golden");
    write_fixture(&dir);
    let stdout = pace_stats(&dir, PaceOptions {
        now: Some(NOW.to_owned()),
        min_closures: 5,
        exclusion_labels: vec!["backlog".to_owned()],
        ..PaceOptions::default()
    });
    let value: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
    let pace = &value["pace"];
    // The metric semantics, pinned: 3 windowed closures at 0.7/wk,
    // median lead 84h, one open seed to the target, and (below the
    // 5-closure minimum) an honest null ETA.
    assert_eq!(value["success"], true);
    assert_eq!(value["command"], "stats");
    assert_eq!(pace["window_days"], 30);
    assert_eq!(pace["min_closures"], 5);
    assert_eq!(pace["rate_window"], serde_json::json!(0.7));
    assert_eq!(pace["closures_in_window"], 3);
    assert_eq!(pace["closures_total"], 4);
    assert_eq!(pace["lead_time_median_hours"], 84.0);
    assert_eq!(pace["open_to_target"], 1);
    assert_eq!(pace["excluded_open"], 1);
    assert_eq!(pace["exclusion_labels"], serde_json::json!(["backlog"]));
    assert_eq!(pace["eta"], Value::Null);
    assert_eq!(pace["eta_reason"], "insufficient_closures_in_window");
    // history 2025-12-20 → 2026-02-01 = 43 days ≈ 6.1 weeks.
    assert_eq!(pace["history_weeks"], 6.1);
    let rate_all = pace["rate_all_time"].as_f64().expect("all-time rate");
    assert!(
        (rate_all - 0.7).abs() < 0.1,
        "all-time rate sane: {rate_all}"
    );
    // The weekly series: 7 buckets, first event week 2025-W51, last
    // running open count 2 (raw series, exclusion-independent).
    let weeks = pace["weeks"].as_array().expect("weeks array");
    assert_eq!(weeks.len(), 7);
    assert_eq!(weeks[0]["iso"], "2025-W51");
    assert_eq!(weeks[0]["created"], 1);
    assert_eq!(weeks[6]["iso"], "2026-W05");
    assert_eq!(weeks[6]["open"], 2);
    assert_eq!(weeks[6]["net"], 0);
}

#[test]
fn pace_text_section_carries_the_metrics_and_table() {
    let dir = temp_store("text");
    write_fixture(&dir);
    let ctx = CommandContext::at(dir.join(".seeds"));
    let outcome = commands::stats(&ctx, &StatsInput {
        json: false,
        pace: Some(PaceOptions {
            now: Some(NOW.to_owned()),
            exclusion_labels: vec!["backlog".to_owned()],
            ..PaceOptions::default()
        }),
    });
    assert!(outcome.success, "stats failed: {}", outcome.stderr);
    let text = outcome.stdout;
    // Parity section first, pace section appended.
    assert!(text.starts_with("Project Statistics\n"), "stats leads");
    assert!(text.contains("\nPace\n"), "pace section follows");
    assert!(text.contains("Rate (30d):"), "window label");
    assert!(text.contains("0.7 / week (3 closures in window)"));
    assert!(text.contains("Lead time (median): 84.0h"));
    assert!(text.contains("Open to target:     1 (exclusion: backlog — 1 open excluded)"));
    assert!(text.contains("ETA:                insufficient data (3 < 5 closures in 30d window"));
    assert!(text.contains("Week (ISO, UTC)"));
    assert!(text.contains("2025-W51"));
    assert!(text.contains("2026-W05"));
}

#[test]
fn pace_flag_implies_section_and_window_garbage_is_named() {
    let dir = temp_store("flags");
    write_fixture(&dir);
    // Any pace option implies --pace.
    let ctx = CommandContext::at(dir.join(".seeds"));
    let outcome = commands::stats(&ctx, &StatsInput {
        json: false,
        pace: Some(PaceOptions {
            window_days: 7,
            ..PaceOptions::default()
        }),
    });
    assert!(outcome.success);
    assert!(outcome.stdout.contains("Rate (7d):"));

    // Library-side semantic validation surfaces the offending flag.
    let bad = commands::stats(&ctx, &StatsInput {
        json: false,
        pace: Some(PaceOptions {
            window_days: 0,
            ..PaceOptions::default()
        }),
    });
    assert!(!bad.success);
    assert!(bad.stderr.contains("--window"), "stderr: {}", bad.stderr);
}
