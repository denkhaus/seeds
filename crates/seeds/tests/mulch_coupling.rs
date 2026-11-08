//! End-to-end contract tests for the seeds↔mulch coupling (seeds-eecf).
//!
//! The plan surface shells out to the `mulch` binary in three places:
//! domain inference (`mulch --json status`), per-section prior-art
//! enrichment (`mulch --json query <domain> --type <kind>`), and the
//! opt-in `--record-decision` write (`mulch record … --evidence-seeds
//! … --json`). Until the native cutover (seeds-26c3) none of this had
//! test coverage — the call shapes were verified by hand once. These
//! tests pin the CONTRACT against the real binary, so drift on either
//! side fails here instead of degrading silently in production (every
//! coupling path is best-effort by design, which is exactly why a
//! broken shape would otherwise go unnoticed).
//!
//! Binary resolution mirrors the sd-reference battery: the
//! `SEEDS_MULCH` env override first (CI exports it, fail-hard there),
//! then `mulch` on PATH (dev machines, toolchain image). Absent both,
//! the battery skips with a note.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

// ---------------------------------------------------------------------------
// binary resolution + run helpers
// ---------------------------------------------------------------------------

fn our_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_seeds"))
}

/// Resolves the mulch binary under test (see the module docs).
fn mulch_binary() -> Option<PathBuf> {
    if let Some(from_env) = std::env::var("SEEDS_MULCH")
        .ok()
        .filter(|value| !value.is_empty())
    {
        let path = PathBuf::from(from_env);
        assert!(
            path.is_file(),
            "SEEDS_MULCH is not a file: {}",
            path.display()
        );
        return Some(path);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("mulch"))
        .find(|candidate| candidate.is_file())
}

#[allow(
    clippy::print_stderr,
    reason = "the skip-with-note contract requires a visible note on stderr"
)]
fn skip_note() {
    eprintln!(
        "note: no mulch binary (SEEDS_MULCH unset, none on PATH) — coupling \
         contract not probed; provision the release artifact and export \
         SEEDS_MULCH to enforce it"
    );
}

/// A unique temp working directory for one test.
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "seeds-mulch-coupling-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Runs one argv in `dir` with the mulch binary's directory FIRST on
/// the child's PATH — the seeds binary must resolve `mulch` there for
/// its coupling shell-outs, whatever the test runner's own PATH holds.
fn run(dir: &Path, program: &Path, args: &[&str], mulch: &Path) -> Output {
    let mulch_dir = mulch.parent().expect("mulch binary has a parent");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let prefixed = std::env::join_paths(
        std::iter::once(mulch_dir.to_owned()).chain(std::env::split_paths(&path)),
    )
    .expect("PATH joins");
    Command::new(program)
        .args(args)
        .current_dir(dir)
        .env("PATH", prefixed)
        .output()
        .unwrap_or_else(|error| panic!("{} {args:?}: {error}", program.display()))
}

fn assert_success(dir: &Path, program: &Path, args: &[&str], output: &Output) {
    assert!(
        output.status.success(),
        "{} {args:?} in {} failed\nstdout: {}\nstderr: {}",
        program.display(),
        dir.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("JSON stdout: {error}\nraw: {output:?}"))
}

/// The one record a fresh `seeds create` appended, read from the store
/// (create mints a random id; the store is its authoritative home).
fn sole_seed_id(dir: &Path) -> String {
    let text = std::fs::read_to_string(dir.join(".seeds/issues.jsonl")).expect("issues.jsonl");
    let line = text
        .lines()
        .rfind(|line| !line.is_empty())
        .expect("one seeded record");
    let record: Value = serde_json::from_str(line).expect("valid JSONL record");
    record["id"].as_str().expect("record id").to_owned()
}

/// The one plan a `plan submit` wrote, read from the store.
fn sole_plan_id(dir: &Path) -> String {
    let text = std::fs::read_to_string(dir.join(".seeds/plans.jsonl")).expect("plans.jsonl");
    let line = text
        .lines()
        .rfind(|line| !line.is_empty())
        .expect("one plan record");
    let record: Value = serde_json::from_str(line).expect("valid plan record");
    record["id"].as_str().expect("plan id").to_owned()
}

/// Boots both stores in one temp dir: `seeds init` + `mulch init`, and
/// one seed to plan against.
fn boot_pair(tag: &str, mulch: &Path) -> PathBuf {
    let dir = temp_dir(tag);
    let seeds = our_binary();
    let out = run(&dir, &seeds, &["init"], mulch);
    assert_success(&dir, &seeds, &["init"], &out);
    let out = run(&dir, mulch, &["init"], mulch);
    assert_success(&dir, mulch, &["init"], &out);
    let out = run(
        &dir,
        &seeds,
        &["create", "--title", "Coupling probe"],
        mulch,
    );
    assert_success(&dir, &seeds, &["create"], &out);
    dir
}

/// Records one pattern into `domain` and returns its mx-id. The
/// domain is created explicitly first: a record into a FRESH domain
/// prints an auto-create notice line before the JSON envelope, and the
/// coupling (like these tests) parses pure JSON — the contract holds
/// for existing domains.
fn record_pattern(dir: &Path, mulch: &Path, domain: &str, name: &str) -> String {
    let out = run(dir, mulch, &["add", domain], mulch);
    assert_success(dir, mulch, &["add"], &out);
    let out = run(
        dir,
        mulch,
        &[
            "record",
            domain,
            "--type",
            "pattern",
            "--name",
            name,
            "--description",
            "recorded by the coupling battery",
            "--json",
        ],
        mulch,
    );
    assert_success(dir, mulch, &["record"], &out);
    json_stdout(&out)["record"]["id"]
        .as_str()
        .expect("record.id")
        .to_owned()
}

// ---------------------------------------------------------------------------
// the envelope contracts (the exact shapes the src parsers consume)
// ---------------------------------------------------------------------------

/// `mulch --json status` must answer the shape `list_mulch_domains`
/// parses: a `domains` array whose entries carry a string `domain`.
#[test]
fn mulch_status_envelope_matches_coupling_parser() {
    let Some(mulch) = mulch_binary() else {
        skip_note();
        return;
    };
    let dir = boot_pair("status", &mulch);
    record_pattern(&dir, &mulch, "statusdom", "probe");

    let out = run(&dir, &mulch, &["--json", "status"], &mulch);
    assert_success(&dir, &mulch, &["--json", "status"], &out);
    let parsed = json_stdout(&out);
    let domains: Vec<&str> = parsed["domains"]
        .as_array()
        .expect("domains array")
        .iter()
        .map(|entry| entry["domain"].as_str().expect("string domain"))
        .collect();
    assert!(
        domains.contains(&"statusdom"),
        "the recorded domain must appear in the status list: {domains:?}"
    );
}

/// `mulch --json query <domain> --type <kind>` must answer the shape
/// `query_mulch_records` + `summarize` parse: `domains[].records[]`
/// with a non-empty string `id`, the `type` requested, and
/// `name`/`description` strings.
#[test]
fn mulch_query_envelope_matches_coupling_parser() {
    let Some(mulch) = mulch_binary() else {
        skip_note();
        return;
    };
    let dir = boot_pair("query", &mulch);
    record_pattern(&dir, &mulch, "querydom", "query-probe");

    let out = run(
        &dir,
        &mulch,
        &["--json", "query", "querydom", "--type", "pattern"],
        &mulch,
    );
    assert_success(&dir, &mulch, &["--json", "query"], &out);
    let parsed = json_stdout(&out);
    let records: Vec<&Value> = parsed["domains"]
        .as_array()
        .expect("domains array")
        .iter()
        .flat_map(|entry| entry["records"].as_array().expect("records array").iter())
        .collect();
    let probe = records
        .iter()
        .find(|record| record["name"].as_str() == Some("query-probe"))
        .unwrap_or_else(|| panic!("the recorded pattern must surface: {records:?}"));
    assert_ne!(probe["id"].as_str().unwrap_or_default(), "");
    assert_eq!(probe["type"], "pattern");
    assert!(
        probe["description"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );
}

// ---------------------------------------------------------------------------
// the two end-to-end flows through the seeds CLI
// ---------------------------------------------------------------------------

/// `seeds plan prompt --domain <d> --json` must surface recorded
/// expertise as section prior_art: the approach section of the bug
/// template maps to the pattern/decision record types.
#[test]
fn plan_prompt_enriches_prior_art_through_mulch() {
    let Some(mulch) = mulch_binary() else {
        skip_note();
        return;
    };
    let dir = boot_pair("prompt", &mulch);
    let seed = sole_seed_id(&dir);
    let mx = record_pattern(&dir, &mulch, "promptdom", "approach-pattern");

    let out = run(
        &dir,
        &our_binary(),
        &[
            "plan",
            "prompt",
            &seed,
            "--template",
            "bug",
            "--domain",
            "promptdom",
            "--json",
        ],
        &mulch,
    );
    assert_success(&dir, &our_binary(), &["plan", "prompt"], &out);
    let parsed = json_stdout(&out);
    let sections = parsed["plan_request"]["sections"]
        .as_array()
        .expect("sections array");
    let approach = sections
        .iter()
        .find(|section| section["name"] == "approach")
        .expect("the bug template carries an approach section");
    let prior_art = approach["prior_art"].as_array().expect("prior_art array");
    let ids: Vec<&str> = prior_art
        .iter()
        .map(|entry| entry["id"].as_str().expect("prior-art id"))
        .collect();
    assert!(
        ids.contains(&mx.as_str()),
        "the recorded pattern must ride the prompt's prior_art: {ids:?}"
    );
}

/// `seeds plan submit --record-decision --domain <d>` must write a
/// decision record whose `evidence.seeds` names the fresh plan id —
/// the coupling's only outbound write.
#[test]
fn plan_submit_record_decision_writes_mulch_record() {
    let Some(mulch) = mulch_binary() else {
        skip_note();
        return;
    };
    let dir = boot_pair("submit", &mulch);
    let seed = sole_seed_id(&dir);
    let context = "c".repeat(60);
    let plan = serde_json::json!({
        "template": "feature",
        "sections": {
            "context": context,
            "approach": "Drive the coupling battery through the real binary.",
            "steps": [{"title": "Step one"}, {"title": "Step two"}],
            "acceptance": ["the decision record exists"],
        },
    });
    std::fs::write(dir.join("plan.json"), plan.to_string()).expect("write plan.json");

    let out = run(
        &dir,
        &our_binary(),
        &[
            "plan",
            "submit",
            &seed,
            "--plan",
            "plan.json",
            "--record-decision",
            "--domain",
            "submitdom",
            "--json",
        ],
        &mulch,
    );
    assert_success(&dir, &our_binary(), &["plan", "submit"], &out);
    let plan_id = sole_plan_id(&dir);

    let query = run(
        &dir,
        &mulch,
        &["--json", "query", "submitdom", "--type", "decision"],
        &mulch,
    );
    assert_success(&dir, &mulch, &["--json", "query"], &query);
    let parsed = json_stdout(&query);
    let records: Vec<&Value> = parsed["domains"]
        .as_array()
        .expect("domains array")
        .iter()
        .flat_map(|entry| entry["records"].as_array().expect("records array").iter())
        .collect();
    let decision = records
        .iter()
        .find(|record| record["evidence"]["seeds"] == plan_id.as_str())
        .unwrap_or_else(|| {
            panic!("no decision record names the plan id {plan_id} in evidence.seeds: {records:?}")
        });
    assert_eq!(decision["type"], "decision");
}
