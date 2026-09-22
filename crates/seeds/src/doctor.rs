//! `seeds doctor`: project health and data integrity checks over the
//! raw `.seeds/` files (sd-0.5.15 check surface, seeds-c228).
//!
//! The check list, names, pass messages, and exit semantics mirror the
//! reference; deliberate additions (`--repair-report`, the exact-fix
//! repair lines) are documented in the README's DEVIATIONS section.

// Incremental `push_str(&format!(…))` line building reads clearer here
// than chained reassignment (same exception as render.rs).
#![allow(
    clippy::format_push_string,
    reason = "line assembly over optional fields is clearer incrementally"
)]

use std::collections::HashSet;
use std::path::Path;

use serde_json::{Map, Value};

/// The three JSONL tracker files, in check order.
const TRACKER_FILES: [&str; 3] = ["issues.jsonl", "plans.jsonl", "templates.jsonl"];

/// The merge=union lines `--fix` writes into `.gitattributes`.
const GITATTRIBUTES_ENTRIES: &str = "\
.seeds/issues.jsonl merge=union
.seeds/templates.jsonl merge=union
.seeds/plans.jsonl merge=union
";

/// One doctor check outcome.
pub(crate) struct Check {
    /// The check's stable name (the reference's identifier).
    pub name:    &'static str,
    /// `pass`, `warn`, or `fail`.
    pub status:  CheckStatus,
    /// The headline message.
    pub message: String,
    /// Per-finding detail lines.
    pub details: Vec<String>,
    /// Whether `--fix` can repair this finding.
    pub fixable: bool,
    /// The exact repair for a fixable finding (`--repair-report`).
    pub repair:  Option<String>,
}

/// A check's outcome class.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum CheckStatus {
    /// Healthy.
    Pass,
    /// Suspicious but not fatal.
    Warn,
    /// Broken — doctor exits non-zero.
    Fail,
}

impl CheckStatus {
    fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Pass => "pass",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
        }
    }
}

/// The full doctor report.
pub(crate) struct Report {
    /// Every check, in the reference's fixed order.
    pub checks: Vec<Check>,
    /// Fixes `--fix` applied, as user-facing lines.
    pub fixes:  Vec<String>,
}

impl Report {
    /// Counts of each status class.
    fn counts(&self) -> (usize, usize, usize) {
        let mut pass = 0;
        let mut warn = 0;
        let mut fail = 0;
        for check in &self.checks {
            match check.status {
                CheckStatus::Pass => pass += 1,
                CheckStatus::Warn => warn += 1,
                CheckStatus::Fail => fail += 1,
            }
        }
        (pass, warn, fail)
    }

    /// Whether any check failed (doctor exits non-zero).
    pub(crate) fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == CheckStatus::Fail)
    }

    /// The plain-text report (identical shape with and without
    /// `--verbose`: the reference lists only warn/fail lines).
    pub(crate) fn text(&self) -> String {
        let mut out = String::from("\nSeeds Doctor\n\n");
        for check in &self.checks {
            if check.status == CheckStatus::Pass {
                continue;
            }
            let icon = match check.status {
                CheckStatus::Fail => "✗",
                _ => "!",
            };
            out.push_str(&format!("  {icon} {}\n", check.message));
            for detail in &check.details {
                out.push_str(&format!("      {detail}\n"));
            }
        }
        let (pass, warn, fail) = self.counts();
        out.push_str(&format!(
            "\n{pass} passed, {warn} warning(s), {fail} failure(s)\n"
        ));
        if !self.fixes.is_empty() {
            out.push_str("\nFixed:\n");
            for fix in &self.fixes {
                out.push_str(&format!("  ✓ {fix}\n"));
            }
        }
        out
    }

    /// The `--repair-report` appendix: every fixable finding with the
    /// exact repair (the native addition beyond sd parity).
    pub(crate) fn repair_report_text(&self) -> String {
        let mut out = String::from("\nRepairable findings:\n");
        let mut any = false;
        for check in &self.checks {
            if !check.fixable || check.status == CheckStatus::Pass {
                continue;
            }
            let repair = check.repair.as_deref().unwrap_or("(no automatic repair)");
            if check.details.is_empty() {
                out.push_str(&format!("  ! {}\n    fix: {repair}\n", check.message));
                any = true;
            } else {
                for detail in &check.details {
                    out.push_str(&format!("  ! {detail}\n    fix: {repair}\n"));
                }
                any = true;
            }
        }
        if !any {
            out.push_str("  (none — nothing repairable found)\n");
        }
        out
    }

    /// The JSON value of the whole report (machine-readable parity
    /// surface); `include_repairs` (the `--repair-report` mode) adds
    /// the native `repair` fields.
    pub(crate) fn json_value(&self, include_repairs: bool) -> Value {
        let checks: Vec<Value> = self
            .checks
            .iter()
            .map(|check| {
                let mut object = Map::new();
                object.insert("name".to_owned(), Value::String(check.name.to_owned()));
                object.insert(
                    "status".to_owned(),
                    Value::String(check.status.as_str().to_owned()),
                );
                object.insert("message".to_owned(), Value::String(check.message.clone()));
                object.insert(
                    "details".to_owned(),
                    Value::Array(
                        check
                            .details
                            .iter()
                            .map(|detail| Value::String(detail.clone()))
                            .collect(),
                    ),
                );
                object.insert("fixable".to_owned(), Value::Bool(check.fixable));
                if include_repairs && let Some(repair) = &check.repair {
                    object.insert("repair".to_owned(), Value::String(repair.clone()));
                }
                Value::Object(object)
            })
            .collect();
        let (pass, warn, fail) = self.counts();
        let mut summary = Map::new();
        summary.insert("pass".to_owned(), json_number(pass));
        summary.insert("warn".to_owned(), json_number(warn));
        summary.insert("fail".to_owned(), json_number(fail));
        let mut report = Map::new();
        report.insert("checks".to_owned(), Value::Array(checks));
        report.insert("summary".to_owned(), Value::Object(summary));
        if !self.fixes.is_empty() {
            report.insert(
                "fixes".to_owned(),
                Value::Array(
                    self.fixes
                        .iter()
                        .map(|fix| Value::String(fix.clone()))
                        .collect(),
                ),
            );
        }
        Value::Object(report)
    }
}

fn json_number(value: usize) -> Value {
    Value::Number(value.into())
}

fn passed(name: &'static str, message: &str) -> Check {
    Check {
        name,
        status: CheckStatus::Pass,
        message: message.to_owned(),
        details: Vec::new(),
        fixable: false,
        repair: None,
    }
}

/// One raw JSONL line: parsed value or its parse error.
enum Line {
    Ok(Value),
    Malformed(String),
}

/// Reads a tracker file's non-empty lines, parsed or error-tagged.
fn read_lines(root: &Path, file: &str) -> Vec<(usize, Line)> {
    let Ok(text) = std::fs::read_to_string(root.join(file)) else {
        return Vec::new();
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            let number = index + 1;
            let parsed = match serde_json::from_str::<Value>(line) {
                Ok(value) => Line::Ok(value),
                Err(error) => Line::Malformed(error.to_string()),
            };
            (number, parsed)
        })
        .collect()
}

/// Extracts `(id, fields)` from every well-formed object record.
fn records(lines: &[(usize, Line)]) -> Vec<(String, &Map<String, Value>)> {
    lines
        .iter()
        .filter_map(|(_, line)| match line {
            Line::Ok(Value::Object(fields)) => Some(fields),
            _ => None,
        })
        .filter_map(|fields| {
            fields
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_owned(), fields))
        })
        .collect()
}

fn string_array(field: Option<&Value>) -> Option<Vec<String>> {
    match field {
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        _ => None,
    }
}

/// Runs every check over the store rooted at `root` (the `.seeds/`
/// directory). `fix` applies the fixable repairs.
///
/// # Errors
///
/// Returns the message for I/O failures during repairs.
pub(crate) fn run(root: &Path, fix: bool) -> Result<Report, String> {
    let mut fixes = Vec::new();

    // config
    let config_check = match std::fs::read_to_string(root.join("config.yaml")) {
        Ok(text) => match crate::Config::parse(&text, &root.join("config.yaml")) {
            Ok(_) => passed("config", "Config is valid"),
            Err(error) => Check {
                name:    "config",
                status:  CheckStatus::Fail,
                message: "Config is invalid".to_owned(),
                details: vec![error.to_string()],
                fixable: false,
                repair:  None,
            },
        },
        Err(_) => Check {
            name:    "config",
            status:  CheckStatus::Fail,
            message: "config.yaml not found".to_owned(),
            details: Vec::new(),
            fixable: false,
            repair:  None,
        },
    };

    // jsonl-integrity (with the fix: drop malformed lines)
    let mut malformed: Vec<String> = Vec::new();
    let mut files_lines = Vec::new();
    for file in TRACKER_FILES {
        let lines = read_lines(root, file);
        for (number, line) in &lines {
            if let Line::Malformed(error) = line {
                malformed.push(format!("{file} line {number}: {error}"));
            }
        }
        files_lines.push((file, lines));
    }
    let mut jsonl_check = if malformed.is_empty() {
        passed("jsonl-integrity", "All JSONL lines parse correctly")
    } else {
        Check {
            name:    "jsonl-integrity",
            status:  CheckStatus::Fail,
            message: format!("{} malformed line(s) in JSONL files", malformed.len()),
            details: malformed.clone(),
            fixable: true,
            repair:  Some("delete the malformed JSONL line(s)".to_owned()),
        }
    };
    if fix && jsonl_check.status == CheckStatus::Fail {
        for (file, lines) in &files_lines {
            let kept: Vec<String> = lines
                .iter()
                .filter_map(|(_, line)| match line {
                    Line::Ok(value) => Some(value.to_string()),
                    Line::Malformed(_) => None,
                })
                .collect();
            let body = if kept.is_empty() {
                String::new()
            } else {
                format!("{}\n", kept.join("\n"))
            };
            std::fs::write(root.join(file), body)
                .map_err(|error| format!("writing {file}: {error}"))?;
        }
        fixes.push(format!(
            "Removed {} malformed line(s) from tracker files",
            malformed.len()
        ));
        files_lines = files_lines
            .into_iter()
            .map(|(file, _)| (file, read_lines(root, file)))
            .collect();
        jsonl_check.status = CheckStatus::Pass;
        "All JSONL lines parse correctly".clone_into(&mut jsonl_check.message);
        jsonl_check.details.clear();
    }

    let issues_lines = files_lines
        .iter()
        .find(|(file, _)| *file == "issues.jsonl")
        .map(|(_, lines)| lines)
        .expect("issues.jsonl is always scanned");
    let issue_records = records(issues_lines);

    // schema-validation
    let schema_issues: Vec<String> = issue_records
        .iter()
        .filter(|(_id, fields)| {
            fields.get("title").and_then(Value::as_str).is_none()
                || fields.get("status").and_then(Value::as_str).is_none()
        })
        .map(|(id, _)| format!("{id}: missing required field (title/status)"))
        .collect();
    let schema_check = if schema_issues.is_empty() {
        passed("schema-validation", "All issues have valid schema")
    } else {
        Check {
            name:    "schema-validation",
            status:  CheckStatus::Fail,
            message: format!("{} issue(s) with invalid schema", schema_issues.len()),
            details: schema_issues,
            fixable: false,
            repair:  None,
        }
    };

    // duplicate-ids
    let mut seen = HashSet::new();
    let mut duplicate_ids = Vec::new();
    for (id, _) in &issue_records {
        if !seen.insert(id.clone()) && !duplicate_ids.contains(id) {
            duplicate_ids.push(id.clone());
        }
    }
    let duplicate_check = if duplicate_ids.is_empty() {
        passed("duplicate-ids", "No duplicate IDs")
    } else {
        Check {
            name:    "duplicate-ids",
            status:  CheckStatus::Fail,
            message: format!("{} duplicate id(s)", duplicate_ids.len()),
            details: duplicate_ids,
            fixable: true,
            repair:  Some("run `seeds dedupe --write` to heal duplicates".to_owned()),
        }
    };

    // referential-integrity
    let known: HashSet<&str> = issue_records.iter().map(|(id, _)| id.as_str()).collect();
    let mut dangling = Vec::new();
    for (id, fields) in &issue_records {
        for dep in string_array(fields.get("blockedBy")).unwrap_or_default() {
            if !known.contains(dep.as_str()) {
                dangling.push(format!("{id}.blockedBy references unknown {dep}"));
            }
        }
        for dep in string_array(fields.get("blocks")).unwrap_or_default() {
            if !known.contains(dep.as_str()) {
                dangling.push(format!("{id}.blocks references unknown {dep}"));
            }
        }
    }
    let referential_check = if dangling.is_empty() {
        passed(
            "referential-integrity",
            "All dependency references are valid",
        )
    } else {
        Check {
            name:    "referential-integrity",
            status:  CheckStatus::Fail,
            message: format!("{} dangling dependency reference(s)", dangling.len()),
            details: dangling,
            fixable: true,
            repair:  Some(
                "remove the dangling references (or create the missing issues)".to_owned(),
            ),
        }
    };

    // bidirectional-consistency (with the fix: add missing reverse refs)
    let mut mismatches: Vec<Mismatch> = Vec::new();
    for (id, fields) in &issue_records {
        for dep in string_array(fields.get("blockedBy")).unwrap_or_default() {
            if let Some((_, blocker)) = issue_records.iter().find(|(other, _)| *other == dep) {
                let blocks = string_array(blocker.get("blocks")).unwrap_or_default();
                if !blocks.iter().any(|blocked| blocked == id) {
                    mismatches.push(Mismatch::MissingBlocks {
                        dependent: id.clone(),
                        blocker:   dep.clone(),
                    });
                }
            }
        }
    }
    for (id, fields) in &issue_records {
        for blocked in string_array(fields.get("blocks")).unwrap_or_default() {
            if let Some((_, dependent)) = issue_records.iter().find(|(other, _)| *other == blocked)
            {
                let blocked_by = string_array(dependent.get("blockedBy")).unwrap_or_default();
                if !blocked_by.iter().any(|dep| dep == id) {
                    mismatches.push(Mismatch::MissingBlockedBy {
                        blocker:   id.clone(),
                        dependent: blocked.clone(),
                    });
                }
            }
        }
    }
    let mut bidirectional_check = if mismatches.is_empty() {
        passed(
            "bidirectional-consistency",
            "Bidirectional dependencies consistent",
        )
    } else {
        let details: Vec<String> = mismatches.iter().map(Mismatch::detail).collect();
        Check {
            name: "bidirectional-consistency",
            status: CheckStatus::Warn,
            message: format!("{} bidirectional mismatch(es)", details.len()),
            details,
            fixable: true,
            repair: Some(match mismatches.as_slice() {
                [single] => single.repair(),
                many => format!(
                    "add the missing reverse reference(s): {}",
                    many.iter()
                        .map(Mismatch::repair)
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            }),
        }
    };
    if fix && bidirectional_check.status == CheckStatus::Warn {
        let repaired = repair_bidirectional(root, &mismatches)?;
        if repaired > 0 {
            fixes.push(format!(
                "Repaired {repaired} bidirectional dependency mismatch(es)"
            ));
            bidirectional_check.status = CheckStatus::Pass;
            "Bidirectional dependencies consistent".clone_into(&mut bidirectional_check.message);
            bidirectional_check.details.clear();
        }
    }

    // circular-dependencies
    let mut cycles = Vec::new();
    for (start, _) in &issue_records {
        let mut path = vec![start.clone()];
        let mut current = start.clone();
        while let Some((_, fields)) = issue_records.iter().find(|(id, _)| *id == current) {
            let Some(next) = string_array(fields.get("blockedBy"))
                .unwrap_or_default()
                .into_iter()
                .find(|dep| known.contains(dep.as_str()))
            else {
                break;
            };
            if next == *start {
                cycles.push(format!("{} → {} (→ {start})", path.join(" → "), next));
                break;
            }
            if path.contains(&next) || path.len() > issue_records.len() {
                break;
            }
            path.push(next.clone());
            current = next;
        }
    }
    let circular_check = if cycles.is_empty() {
        passed("circular-dependencies", "No circular dependencies")
    } else {
        Check {
            name:    "circular-dependencies",
            status:  CheckStatus::Fail,
            message: format!("{} circular dependency cycle(s)", cycles.len()),
            details: cycles,
            fixable: false,
            repair:  None,
        }
    };

    // label-schema
    let bad_labels: Vec<String> = issue_records
        .iter()
        .filter(|(_id, fields)| {
            fields.contains_key("labels") && string_array(fields.get("labels")).is_none()
        })
        .map(|(id, _)| format!("{id}: labels is not an array of strings"))
        .collect();
    let label_check = if bad_labels.is_empty() {
        passed("label-schema", "All label arrays are valid")
    } else {
        Check {
            name:    "label-schema",
            status:  CheckStatus::Fail,
            message: format!("{} invalid label array(s)", bad_labels.len()),
            details: bad_labels,
            fixable: true,
            repair:  Some("rewrite labels as an array of strings".to_owned()),
        }
    };

    // extensions-schema
    let bad_extensions: Vec<String> = issue_records
        .iter()
        .filter(|(_id, fields)| {
            fields.contains_key("extensions")
                && !matches!(fields.get("extensions"), Some(Value::Object(_)))
        })
        .map(|(id, _)| format!("{id}: extensions is not an object"))
        .collect();
    let extensions_check = if bad_extensions.is_empty() {
        passed("extensions-schema", "All extensions fields are valid")
    } else {
        Check {
            name:    "extensions-schema",
            status:  CheckStatus::Fail,
            message: format!("{} invalid extensions field(s)", bad_extensions.len()),
            details: bad_extensions,
            fixable: true,
            repair:  Some("rewrite extensions as a JSON object".to_owned()),
        }
    };

    // closed-fields-consistency
    let inconsistent: Vec<String> = issue_records
        .iter()
        .filter(|(_id, fields)| {
            let status = fields.get("status").and_then(Value::as_str);
            let has_closed_at = fields.contains_key("closedAt");
            (status == Some("closed")) != has_closed_at
        })
        .map(|(id, _)| format!("{id}: status/closedAt pair inconsistent"))
        .collect();
    let closed_check = if inconsistent.is_empty() {
        passed(
            "closed-fields-consistency",
            "All status/close-metadata pairs are consistent",
        )
    } else {
        Check {
            name:    "closed-fields-consistency",
            status:  CheckStatus::Warn,
            message: format!(
                "{} inconsistent status/close-metadata pair(s)",
                inconsistent.len()
            ),
            details: inconsistent,
            fixable: true,
            repair:  Some("set closedAt on closed issues, drop it on open ones".to_owned()),
        }
    };

    // stale-locks
    let locks: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "lock"))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let locks_check = if locks.is_empty() {
        passed("stale-locks", "No stale lock files")
    } else {
        Check {
            name:    "stale-locks",
            status:  CheckStatus::Warn,
            message: format!("{} stale lock file(s)", locks.len()),
            details: locks,
            fixable: true,
            repair:  Some("delete the stale lock file(s)".to_owned()),
        }
    };

    // gitattributes (with the fix: create the merge=union entries)
    let project_root = root.parent().unwrap_or(Path::new("."));
    let gitattributes = project_root.join(".gitattributes");
    let mut git_check = match std::fs::read_to_string(&gitattributes) {
        Ok(text) => {
            let missing: Vec<&str> = [
                ".seeds/issues.jsonl",
                ".seeds/templates.jsonl",
                ".seeds/plans.jsonl",
            ]
            .into_iter()
            .filter(|entry| !text.contains(entry))
            .collect();
            if missing.is_empty() {
                passed("gitattributes", "Gitattributes merge=union entries present")
            } else {
                Check {
                    name:    "gitattributes",
                    status:  CheckStatus::Warn,
                    message: "Missing merge=union gitattributes entries".to_owned(),
                    details: missing
                        .iter()
                        .map(|entry| format!("{entry} merge=union entry missing"))
                        .collect(),
                    fixable: true,
                    repair:  Some("append the missing merge=union entries".to_owned()),
                }
            }
        }
        Err(_) => Check {
            name:    "gitattributes",
            status:  CheckStatus::Warn,
            message: "Missing merge=union gitattributes entries".to_owned(),
            details: vec![".gitattributes file not found".to_owned()],
            fixable: true,
            repair:  Some("create .gitattributes with the merge=union entries".to_owned()),
        },
    };
    if fix && git_check.status == CheckStatus::Warn {
        let existing = std::fs::read_to_string(&gitattributes).unwrap_or_default();
        let mut body = existing;
        if !body.is_empty() && !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(GITATTRIBUTES_ENTRIES);
        std::fs::write(&gitattributes, body)
            .map_err(|error| format!("writing .gitattributes: {error}"))?;
        fixes.push("Created .gitattributes with merge=union entries".to_owned());
        git_check.status = CheckStatus::Pass;
        "Gitattributes merge=union entries present".clone_into(&mut git_check.message);
        git_check.details.clear();
    }

    Ok(Report {
        checks: vec![
            config_check,
            jsonl_check,
            schema_check,
            duplicate_check,
            referential_check,
            bidirectional_check,
            circular_check,
            label_check,
            extensions_check,
            closed_check,
            locks_check,
            git_check,
        ],
        fixes,
    })
}

/// One bidirectional dependency mismatch.
enum Mismatch {
    /// `dependent.blockedBy` names `blocker`, but `blocker.blocks`
    /// misses `dependent`.
    MissingBlocks {
        dependent: String,
        blocker:   String,
    },
    /// `blocker.blocks` names `dependent`, but `dependent.blockedBy`
    /// misses `blocker`.
    MissingBlockedBy {
        blocker:   String,
        dependent: String,
    },
}

impl Mismatch {
    fn detail(&self) -> String {
        match self {
            Mismatch::MissingBlocks { dependent, blocker } => format!(
                "{dependent}.blockedBy has {blocker}, but {blocker}.blocks missing {dependent}"
            ),
            Mismatch::MissingBlockedBy { blocker, dependent } => format!(
                "{blocker}.blocks has {dependent}, but {dependent}.blockedBy missing {blocker}"
            ),
        }
    }

    fn repair(&self) -> String {
        match self {
            Mismatch::MissingBlocks { dependent, blocker } => format!(
                "add \"{dependent}\" to {blocker}.blocks — `seeds dep add {dependent} {blocker}`"
            ),
            Mismatch::MissingBlockedBy { blocker, dependent } => format!(
                "add \"{blocker}\" to {dependent}.blockedBy — `seeds dep add {dependent} {blocker}`"
            ),
        }
    }
}

/// Applies the bidirectional repairs by rewriting `issues.jsonl`
/// fields directly (the store API would re-validate everything; the
/// doctor must heal precisely the named mismatches).
fn repair_bidirectional(root: &Path, mismatches: &[Mismatch]) -> Result<usize, String> {
    let path = root.join("issues.jsonl");
    let text =
        std::fs::read_to_string(&path).map_err(|error| format!("reading issues.jsonl: {error}"))?;
    let mut repaired = 0;
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(mut value) = serde_json::from_str::<Value>(line) else {
            lines.push(line.to_owned());
            continue;
        };
        let Some(fields) = value.as_object_mut() else {
            lines.push(line.to_owned());
            continue;
        };
        let Some(id) = fields.get("id").and_then(Value::as_str).map(str::to_owned) else {
            lines.push(line.to_owned());
            continue;
        };
        for mismatch in mismatches {
            match mismatch {
                Mismatch::MissingBlocks { dependent, blocker } if blocker == &id => {
                    repaired += usize::from(push_unique(fields, "blocks", dependent));
                }
                Mismatch::MissingBlockedBy { blocker, dependent } if dependent == &id => {
                    repaired += usize::from(push_unique(fields, "blockedBy", blocker));
                }
                _ => {}
            }
        }
        lines.push(serde_json::to_string(&value).expect("record serializes"));
    }
    let body = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };
    std::fs::write(&path, body).map_err(|error| format!("writing issues.jsonl: {error}"))?;
    Ok(repaired)
}

/// Pushes `id` onto the string-array field `name` (creating it when
/// absent); `false` when already present or the current record is not
/// the target.
fn push_unique(fields: &mut Map<String, Value>, name: &str, id: &str) -> bool {
    let mut items = string_array(fields.get(name)).unwrap_or_default();
    if items.iter().any(|existing| existing == id) {
        return false;
    }
    items.push(id.to_owned());
    let value: Vec<Value> = items.into_iter().map(Value::String).collect();
    fields.insert(name.to_owned(), Value::Array(value));
    true
}
