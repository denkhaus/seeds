//! dedupe and sync command semantics — native additions beyond sd
//! parity (ADR-0023 additive).

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use super::{CommandContext, CommandError, CommandOutcome, envelope_pretty, push_line};
use crate::{dedupe, timeutil};

/// One file's dedupe outcome for reporting.
struct DedupeFile {
    name:          &'static str,
    duplicates:    Vec<dedupe::Duplicate>,
    dropped_lines: usize,
}

impl DedupeFile {
    fn duplicate_ids(&self) -> usize {
        self.duplicates.len()
    }

    fn duplicates_value(&self) -> Value {
        Value::Array(
            self.duplicates
                .iter()
                .map(|duplicate| {
                    json!({
                        "id": duplicate.id,
                        "count": duplicate.count,
                        "keptUpdatedAt": duplicate.kept_updated_at,
                        "droppedUpdatedAts": duplicate.dropped_updated_ats,
                    })
                })
                .collect(),
        )
    }
}

/// Typed input for `dedupe`.
#[derive(Clone, Debug, Default)]
pub struct DedupeInput {
    /// `--write` (heal the files; report-only otherwise).
    pub write: bool,
    /// JSON envelope output.
    pub json:  bool,
}

/// `dedupe` — detects (and with `--write`, heals) duplicate-id lines in
/// the tracker files; report mode is unsuccessful when duplicates
/// remain.
pub fn dedupe(ctx: &CommandContext, input: &DedupeInput) -> CommandOutcome {
    let json = input.json;
    let write = input.write;
    let result = (|| -> Result<Vec<DedupeFile>, CommandError> {
        let message_of = |text: String| CommandError::new("dedupe", text, json);
        let root = ctx.resolve().map_err(&message_of)?.to_path_buf();
        let mut files = Vec::new();
        for name in dedupe::TRACKER_FILES {
            let path = root.join(name);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue; // a missing store file is empty, nothing to heal
            };
            let lines: Vec<&str> = text
                .lines()
                .map(|line| line.trim_end_matches(['\r', '\n']))
                .filter(|line| !line.is_empty())
                .collect();
            let (healed, duplicates) = dedupe::heal_lines(&lines);
            let dropped_lines = lines.len() - healed.len();
            if write && !duplicates.is_empty() {
                let body = format!("{}\n", healed.join("\n"));
                dedupe::write_atomic(&path, &body).map_err(|source| {
                    message_of(format!("writing {}: {source}", path.display()))
                })?;
            }
            files.push(DedupeFile {
                name,
                duplicates,
                dropped_lines,
            });
        }
        Ok(files)
    })();
    let files = match result {
        Ok(files) => files,
        Err(error) => return error.into_outcome(),
    };

    let duplicate_ids: usize = files.iter().map(DedupeFile::duplicate_ids).sum();
    let dropped_total: usize = files.iter().map(|file| file.dropped_lines).sum();
    let mut out = String::new();
    if json {
        let mut extra = vec![
            ("write", json!(write)),
            (
                "files",
                Value::Array(
                    files
                        .iter()
                        .map(|file| {
                            json!({
                                "file": file.name,
                                "duplicates": file.duplicates_value(),
                            })
                        })
                        .collect(),
                ),
            ),
            ("duplicateIds", json!(duplicate_ids)),
        ];
        if write {
            extra.push(("droppedLines", json!(dropped_total)));
            extra.push(("written", json!(duplicate_ids > 0)));
        }
        push_line(&mut out, &envelope_pretty("dedupe", &extra));
    } else if write {
        for file in files.iter().filter(|file| file.dropped_lines > 0) {
            push_line(
                &mut out,
                &format!(
                    "{}: dropped {} duplicate lines ({} ids)",
                    file.name,
                    file.dropped_lines,
                    file.duplicate_ids()
                ),
            );
        }
        if duplicate_ids == 0 {
            push_line(&mut out, "✓ no duplicate ids found (nothing to write)");
        } else {
            push_line(
                &mut out,
                &format!("✓ healed {duplicate_ids} duplicate ids, dropped {dropped_total} lines"),
            );
        }
    } else {
        for file in &files {
            if file.duplicates.is_empty() {
                continue;
            }
            push_line(
                &mut out,
                &format!("{}: {} duplicate ids", file.name, file.duplicate_ids()),
            );
            for duplicate in &file.duplicates {
                let kept = duplicate
                    .kept_updated_at
                    .as_deref()
                    .unwrap_or("(no updatedAt)");
                let dropped = duplicate
                    .dropped_updated_ats
                    .iter()
                    .map(|value| value.as_deref().unwrap_or("(no updatedAt)"))
                    .collect::<Vec<_>>()
                    .join(", ");
                push_line(
                    &mut out,
                    &format!(
                        "  {} ×{} kept updatedAt={} dropped: {}",
                        duplicate.id, duplicate.count, kept, dropped
                    ),
                );
            }
        }
        if duplicate_ids == 0 {
            push_line(&mut out, "✓ no duplicate ids found");
        }
    }
    // Report mode is gate-friendly: non-zero exactly when duplicates
    // remain; `--write` heals by definition and always succeeds.
    CommandOutcome {
        success: write || duplicate_ids == 0,
        stdout:  out,
        stderr:  String::new(),
    }
}

/// One `seeds sync` outcome, rendered per mode (seeds-540e).
enum SyncOutcome {
    /// Nothing dirty under `.seeds/` — plain and `--dry-run` runs.
    NoChanges,
    /// `--status`: the per-file change preview (never commits).
    Status(String),
    /// `--dry-run` on a dirty store (never commits).
    DryRun { changes: String, message: String },
    /// A commit was created.
    Committed(String),
}

impl SyncOutcome {
    fn report(&self, json: bool) -> CommandOutcome {
        let mut out = String::new();
        match self {
            Self::NoChanges => {
                if json {
                    push_line(
                        &mut out,
                        &envelope_pretty("sync", &[
                            ("committed", json!(false)),
                            ("message", json!("Nothing to commit")),
                        ]),
                    );
                } else {
                    push_line(&mut out, "✓ No changes to commit.");
                }
            }
            Self::Status(changes) => {
                if json {
                    push_line(
                        &mut out,
                        &envelope_pretty("sync", &[
                            ("hasChanges", json!(!changes.is_empty())),
                            ("changes", json!(changes)),
                        ]),
                    );
                } else if changes.is_empty() {
                    push_line(&mut out, "✓ No uncommitted .seeds/ changes.");
                } else {
                    push_line(&mut out, "✓ Uncommitted .seeds/ changes:");
                    push_line(&mut out, changes);
                }
            }
            Self::DryRun { changes, message } => {
                if json {
                    push_line(
                        &mut out,
                        &envelope_pretty("sync", &[
                            ("dryRun", json!(true)),
                            ("wouldCommit", json!(true)),
                            ("message", json!(message)),
                            ("changes", json!(changes)),
                        ]),
                    );
                } else {
                    push_line(&mut out, "✓ Dry run — would commit:");
                    push_line(&mut out, changes);
                    push_line(&mut out, &format!("Commit message: {message}"));
                }
            }
            Self::Committed(message) => {
                if json {
                    push_line(
                        &mut out,
                        &envelope_pretty("sync", &[
                            ("committed", json!(true)),
                            ("message", json!(message)),
                        ]),
                    );
                } else {
                    push_line(&mut out, &format!("✓ Committed: {message}"));
                }
            }
        }
        CommandOutcome::ok(out)
    }
}

/// Typed input for `sync`.
#[derive(Clone, Debug, Default)]
pub struct SyncInput {
    /// `--status` (preview, never commits).
    pub status:  bool,
    /// `--dry-run`.
    pub dry_run: bool,
    /// `--force` (push-gate override).
    pub force:   bool,
    /// JSON envelope output.
    pub json:    bool,
}

/// `sync` — stages and commits `.seeds/` changes in the enclosing git
/// repository.
pub fn sync(ctx: &CommandContext, input: &SyncInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<SyncOutcome, CommandError> {
        let message_of = |text: String| CommandError::new("sync", text, json);
        let seeds_dir = ctx
            .resolve()
            .map_err(|_| message_of("Not in a seeds project. Run `sd init` first.".to_owned()))?;
        let Some(repo) = git_repo_root(seeds_dir.parent().unwrap_or(Path::new("."))) else {
            // The reference's observed behavior outside a git worktree.
            return Ok(SyncOutcome::NoChanges);
        };
        let seeds_path = seeds_dir.to_string_lossy().into_owned();
        let changes = git(&repo, &[
            "status",
            "--porcelain",
            "-uall",
            "--",
            &seeds_path,
        ])
        .map_err(&message_of)?;
        let changes = changes
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if input.status {
            return Ok(SyncOutcome::Status(changes));
        }
        if changes.is_empty() {
            return Ok(SyncOutcome::NoChanges);
        }
        let message = format!("seeds: sync {}", timeutil::today_utc());
        if input.dry_run {
            return Ok(SyncOutcome::DryRun { changes, message });
        }
        // Push-gate safety (seeds-540e): in fabro repos the tracker
        // must not race a running pass — a refused gate blocks the
        // commit; `--force` is the human override.
        if !input.force {
            let gate = repo.join(".fabro").join("scripts").join("push-gate.nu");
            if gate.is_file()
                && let Some(reason) = push_gate_refusal(&repo, &gate)
            {
                return Err(message_of(format!(
                    "push gate refused — not committing .seeds/ while a pass \
                     may be running: {reason} (override with --force)"
                )));
            }
        }
        git(&repo, &["add", "-A", "--", &seeds_path]).map_err(&message_of)?;
        // The shortstat body line makes sync history greppable by size.
        let shortstat = git(&repo, &[
            "diff",
            "--cached",
            "--shortstat",
            "--",
            &seeds_path,
        ])
        .map_err(&message_of)?;
        let shortstat = shortstat.trim();
        let mut commit_args = vec!["commit", "-m", message.as_str()];
        if !shortstat.is_empty() {
            commit_args.push("-m");
            commit_args.push(shortstat);
        }
        git(&repo, &commit_args).map_err(&message_of)?;
        Ok(SyncOutcome::Committed(message))
    })();
    match result {
        Ok(outcome) => outcome.report(json),
        Err(error) => error.into_outcome(),
    }
}

/// Runs `git` in `repo`, returning trimmed stdout; a non-zero exit
/// carries git's stderr.
fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|source| format!("spawning git: {source}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if stderr.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            format!("git {}: {stderr}", args.join(" "))
        })
    }
}

/// The repository root containing `dir`, or `None` outside a worktree.
fn git_repo_root(dir: &Path) -> Option<std::path::PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
}

/// Runs the fabro push gate; `Some(reason)` when it refused (non-zero
/// exit). A gate that cannot run at all (no `nu`) does not block sync
/// — documented in the README's DEVIATIONS section.
fn push_gate_refusal(repo: &Path, gate: &Path) -> Option<String> {
    let output = Command::new("nu")
        .arg(gate)
        .current_dir(repo)
        .output()
        .ok()?;
    if output.status.success() {
        return None;
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let reason = text
        .lines()
        .find(|line| line.contains("GATE REFUSED"))
        .map_or_else(|| "gate exited non-zero".to_owned(), str::to_owned);
    Some(reason)
}
