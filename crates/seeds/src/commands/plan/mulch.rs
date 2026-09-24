//! The mulch coupling of the plan surface (sd 0.5.15 `plan-domain.ts`
//! and `plan-mulch.ts`).
//!
//! Covers domain inference, per-section prior-art enrichment, and the
//! opt-in outbound decision record. All paths are best-effort — any
//! failure yields empty results, never an error.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::SeedRecord;

const PRIOR_ART_LIMIT: usize = 5;

/// sd's well-known section → mulch record types mapping.
fn types_for_section(name: &str, hint: Option<&str>) -> Vec<String> {
    if let Some(hint) = hint.filter(|hint| !hint.is_empty()) {
        return vec![hint.to_owned()];
    }
    match name {
        "approach" => vec!["pattern", "decision"],
        "risks" => vec!["failure"],
        "acceptance" => vec!["guide"],
        _ => Vec::new(),
    }
    .into_iter()
    .map(String::from)
    .collect()
}

/// Resolves `ml` on PATH (sd's `Bun.which("ml")`).
fn ml_binary() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("ml"))
        .find(|candidate| candidate.is_file())
}

/// Infers the mulch domain for a seed: explicit `--domain` first, then
/// seed labels, then path-shaped description refs intersected with the
/// working tree's changed files.
pub(crate) fn infer_domain(
    explicit: Option<&str>,
    seed: &SeedRecord,
    cwd: &Path,
) -> Option<String> {
    if let Some(domain) = explicit.filter(|domain| !domain.is_empty()) {
        return Some(domain.to_owned());
    }
    let ml = ml_binary()?;
    let domains = list_mulch_domains(&ml, cwd)?;
    let domain_set: HashSet<&str> = domains.iter().map(String::as_str).collect();
    for label in seed.labels() {
        if domain_set.contains(label) {
            return Some(label.to_owned());
        }
    }
    let description = seed.description().unwrap_or_default();
    let refs = extract_path_refs(description);
    if refs.is_empty() {
        return None;
    }
    let changed = git_changed_files(cwd);
    let candidates: Vec<&String> = match &changed {
        Some(changed) if !changed.is_empty() => {
            let changed_set: HashSet<&str> = changed.iter().map(String::as_str).collect();
            refs.iter()
                .filter(|r| changed_set.contains(r.as_str()))
                .collect()
        }
        _ => refs.iter().collect(),
    };
    for path in candidates {
        for segment in path.split('/') {
            if domain_set.contains(segment) {
                return Some(segment.to_owned());
            }
        }
    }
    None
}

fn list_mulch_domains(ml: &Path, cwd: &Path) -> Option<Vec<String>> {
    let output = Command::new(ml)
        .args(["--json", "status"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let parsed: Value = serde_json::from_slice(&output.stdout).ok()?;
    let entries = parsed.get("domains")?.as_array()?;
    let mut out = Vec::new();
    for entry in entries {
        if let Some(domain) = entry.get("domain").and_then(Value::as_str) {
            out.push(domain.to_owned());
        }
    }
    Some(out)
}

fn git_changed_files(cwd: &Path) -> Option<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect(),
    )
}

/// sd's path-shaped token matcher: at least one slash, ending in an
/// extension, no spaces.
fn extract_path_refs(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if is_path_shaped(word) {
            push_unique(&mut out, word);
        }
    }
    out
}

fn is_path_shaped(word: &str) -> bool {
    let cleaned = word.trim_end_matches(['.', ',', ';', ':', ')', ']']);
    let mut segments = cleaned.split('/');
    let first = segments.next().unwrap_or_default();
    if first.is_empty() {
        return false;
    }
    let mut last: Option<&str> = None;
    for segment in segments {
        if segment.is_empty() {
            return false;
        }
        last = Some(segment);
    }
    match last {
        Some(last) => last.contains('.'),
        None => false,
    }
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    let cleaned = value
        .trim_end_matches(['.', ',', ';', ':', ')', ']'])
        .to_owned();
    if !list.contains(&cleaned) {
        list.push(cleaned);
    }
}

/// One prior-art entry surfaced in `plan prompt`'s sections.
#[derive(Clone, Debug)]
pub(crate) struct PriorArtEntry {
    /// The mulch record id.
    pub id:        String,
    /// The record type.
    pub kind:      String,
    /// `name: description` (truncated).
    pub summary:   String,
    /// Rank-synthesized relevance.
    pub relevance: f64,
}

/// Enriches sections with prior art from `ml --json query`. Sections
/// that do not map to a mulch record type stay empty.
pub(crate) fn enrich_prior_art(
    domain: Option<&str>,
    sections: &[(String, Option<String>)],
    cwd: &Path,
) -> Vec<(String, Vec<PriorArtEntry>)> {
    let mut out: Vec<(String, Vec<PriorArtEntry>)> = sections
        .iter()
        .map(|(name, _)| (name.clone(), Vec::new()))
        .collect();
    let Some(domain) = domain else {
        return out;
    };
    let Some(ml) = ml_binary() else {
        return out;
    };
    for (name, hint) in sections {
        let types = types_for_section(name, hint.as_deref());
        if types.is_empty() {
            continue;
        }
        let mut collected = Vec::new();
        for kind in &types {
            collected.extend(query_mulch_records(&ml, domain, kind, cwd));
        }
        collected.truncate(PRIOR_ART_LIMIT);
        if let Some(slot) = out.iter_mut().find(|(slot, _)| slot == name) {
            slot.1 = collected;
        }
    }
    out
}

fn query_mulch_records(ml: &Path, domain: &str, kind: &str, cwd: &Path) -> Vec<PriorArtEntry> {
    let output = match Command::new(ml)
        .args(["--json", "query", domain, "--type", kind])
        .current_dir(cwd)
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return Vec::new(),
    };
    let parsed: Value = match serde_json::from_slice(&output.stdout) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };
    let Some(domains) = parsed.get("domains").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut records: Vec<&Value> = Vec::new();
    for entry in domains {
        if let Some(list) = entry.get("records").and_then(Value::as_array) {
            records.extend(list.iter());
        }
    }
    records.truncate(PRIOR_ART_LIMIT);
    let mut out = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let Some(id) = string_field(record, "id") else {
            continue;
        };
        let kind = string_field(record, "type").unwrap_or_else(|| kind.to_owned());
        let relevance =
            ((PRIOR_ART_LIMIT - index) as f64 / PRIOR_ART_LIMIT as f64 * 100.0).round() / 100.0;
        out.push(PriorArtEntry {
            id,
            kind,
            summary: summarize(record),
            relevance,
        });
    }
    out
}

/// sd's opt-in outbound write: `ml record <domain> --type decision
/// ...`. Never fails the submit; the `Err` text is the stderr warning.
pub(crate) fn record_decision(
    domain: &str,
    plan_id: &str,
    title: &str,
    approach: &str,
    cwd: &Path,
) -> Result<Option<String>, String> {
    let Some(ml) = ml_binary() else {
        return Err("ml not found on PATH; skipping --record-decision".to_owned());
    };
    let output = Command::new(&ml)
        .args([
            "record",
            domain,
            "--type",
            "decision",
            "--title",
            title,
            "--rationale",
            approach,
            "--evidence-seeds",
            plan_id,
            "--json",
        ])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("ml record threw: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr.lines().next().unwrap_or_default();
        let detail = if first.is_empty() {
            String::new()
        } else {
            format!(": {first}")
        };
        return Err(format!("ml record failed{detail}"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if stdout.is_empty() {
        return Ok(None);
    }
    Ok(serde_json::from_str::<Value>(&stdout)
        .ok()
        .and_then(|parsed| {
            parsed
                .get("record")
                .and_then(|record| record.get("id"))
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(String::from)
        }))
}

const SUMMARY_MAX: usize = 240;

fn summarize(record: &Value) -> String {
    let name = string_field(record, "name");
    let description = string_field(record, "description");
    let base = match (&name, &description) {
        (Some(name), Some(description)) => format!("{name}: {description}"),
        (None, Some(description)) => description.clone(),
        (Some(name), None) => name.clone(),
        (None, None) => String::new(),
    };
    if base.chars().count() <= SUMMARY_MAX {
        return base;
    }
    let cut: String = base.chars().take(SUMMARY_MAX - 1).collect();
    format!("{}…", cut.trim_end())
}

fn string_field(record: &Value, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(String::from)
}
