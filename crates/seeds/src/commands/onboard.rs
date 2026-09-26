//! `onboard` command semantics (sd 0.5.15-compatible surface, seeds-adapted
//! section content): write or verify the marker-wrapped tracker section in
//! the project's agent doc.
//!
//! Mechanics mirror the reference exactly — marker pair
//! `<!-- seeds:start -->` .. `<!-- seeds:end -->`, the versioned
//! `seeds-onboard-schema` comment as the update discriminator, CLAUDE.md
//! before AGENTS.md as the target, the `{action,status,file}` envelope
//! keys, and exit code 0 for every non-error mode including `missing`.
//! The section BODY is a documented deviation (README DEVIATIONS): it
//! names the live `seeds` CLI and this implementation, not the retired
//! sd reference.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{CommandError, CommandOutcome, envelope_pretty, push_line};

/// The marker pair delimiting the onboard section.
const MARKER_START: &str = "<!-- seeds:start -->";
const MARKER_END: &str = "<!-- seeds:end -->";

/// The section structure version this build writes and detects. The
/// reference's current structure level; a mismatch (older marker)
/// triggers the update path, a match means `current`/`unchanged`.
const SCHEMA_VERSION: u8 = 7;

/// `seeds onboard`: write or verify the agent-doc tracker section.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OnboardInput {
    /// Whether to only verify presence/status without writing.
    pub check:       bool,
    /// Whether to print the section to stdout instead of touching files.
    pub stdout_only: bool,
    /// Whether success output is the JSON envelope.
    pub json:        bool,
}

/// Runs `onboard` against the project at or above the current
/// directory: `--stdout` prints the section, `--check` reports
/// `current`/`outdated`/`missing` without writing, and the plain mode
/// creates, appends, updates, or leaves the section unchanged.
#[must_use = "the outcome only takes effect when the caller prints it"]
pub fn onboard(input: &OnboardInput) -> CommandOutcome {
    let Some(root) = super::find_seeds_dir()
        .ok()
        .and_then(|seeds| seeds.parent().map(Path::to_path_buf))
    else {
        return CommandError::new(
            "onboard",
            "No .seeds directory found (run from the project root)",
            input.json,
        )
        .into_outcome();
    };

    let section = section_text();

    if input.stdout_only {
        // The reference prints the section itself even with --json.
        let mut stdout = String::new();
        push_line(&mut stdout, &section);
        return CommandOutcome {
            success: true,
            stdout,
            stderr: String::new(),
        };
    }

    let target = target_file(&root);
    let target = target.is_file().then_some(target.as_path());
    if input.check {
        return check_outcome(target, input.json);
    }
    write_outcome(target, &root, &section, input.json)
}

/// The preferred agent-doc: CLAUDE.md when present, else AGENTS.md,
/// else CLAUDE.md as the file to create (the reference's order).
fn target_file(root: &Path) -> PathBuf {
    let claude = root.join("CLAUDE.md");
    if claude.is_file() {
        return claude;
    }
    let agents = root.join("AGENTS.md");
    if agents.is_file() {
        return agents;
    }
    claude
}

/// `--check`: report the section's presence and schema status. Every
/// answer — including `missing` — exits 0 (the reference's contract).
fn check_outcome(target: Option<&Path>, json: bool) -> CommandOutcome {
    let (status, file) = match target.map(scan_section) {
        Some(Scan::Current) => ("current", target.map(path_value)),
        Some(Scan::Outdated) => ("outdated", target.map(path_value)),
        _ => ("missing", None),
    };
    if json {
        let stdout = envelope_pretty("onboard", &[
            ("status", json!(status)),
            ("file", file.unwrap_or(Value::Null)),
        ]);
        let mut text = String::new();
        push_line(&mut text, &stdout);
        return CommandOutcome {
            success: true,
            stdout:  text,
            stderr:  String::new(),
        };
    }
    let mut stdout = String::new();
    if status == "missing" {
        push_line(&mut stdout, "Status: missing (no CLAUDE.md found)");
    } else {
        let path = target.map_or_else(|| "?".to_owned(), |p| p.display().to_string());
        push_line(&mut stdout, &format!("Status: {status} ({path})"));
    }
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

/// The plain write mode: create, append, update, or report unchanged.
fn write_outcome(target: Option<&Path>, root: &Path, section: &str, json: bool) -> CommandOutcome {
    // No candidate file exists: create CLAUDE.md with the section.
    let Some(target) = target else {
        let path = root.join("CLAUDE.md");
        let body = format!("{section}\n");
        if let Err(error) = std::fs::write(&path, body) {
            return CommandError::new(
                "onboard",
                format!("cannot write {}: {error}", path.display()),
                json,
            )
            .into_outcome();
        }
        return finish("created", &path, "✓ Created {} with seeds section", json);
    };

    let content = match std::fs::read_to_string(target) {
        Ok(content) => content,
        Err(error) => {
            return CommandError::new(
                "onboard",
                format!("cannot read {}: {error}", target.display()),
                json,
            )
            .into_outcome();
        }
    };

    match scan_text(&content) {
        // Section present at the current schema: nothing to do.
        Scan::Current => finish(
            "unchanged",
            target,
            "✓ Seeds section is already up to date",
            json,
        ),
        // Old schema: replace the section between the markers.
        Scan::Outdated => {
            let rewritten = replace_between_markers(&content, section);
            if let Err(error) = std::fs::write(target, rewritten) {
                return CommandError::new(
                    "onboard",
                    format!("cannot write {}: {error}", target.display()),
                    json,
                )
                .into_outcome();
            }
            finish("updated", target, "✓ Updated seeds section in {}", json)
        }
        // No markers yet: append the section, keeping a blank-line seam.
        Scan::Absent => {
            let mut body = content;
            if !body.ends_with('\n') {
                body.push('\n');
            }
            body.push('\n');
            body.push_str(section);
            body.push('\n');
            if let Err(error) = std::fs::write(target, body) {
                return CommandError::new(
                    "onboard",
                    format!("cannot write {}: {error}", target.display()),
                    json,
                )
                .into_outcome();
            }
            finish("appended", target, "✓ Added seeds section to {}", json)
        }
    }
}

/// One outcome line + envelope in the mode's shape.
fn finish(action: &str, path: &Path, message: &str, json: bool) -> CommandOutcome {
    if json {
        let stdout = envelope_pretty("onboard", &[
            ("action", json!(action)),
            ("file", path_value(path)),
        ]);
        let mut text = String::new();
        push_line(&mut text, &stdout);
        return CommandOutcome {
            success: true,
            stdout:  text,
            stderr:  String::new(),
        };
    }
    let filled = message.replace("{}", &path.display().to_string());
    let mut stdout = String::new();
    push_line(&mut stdout, &filled);
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

fn path_value(path: &Path) -> Value {
    Value::from(path.display().to_string())
}

/// What a file's current content says about the onboard section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scan {
    /// Markers present at the current schema version.
    Current,
    /// Markers present at an older schema version.
    Outdated,
    /// No marker pair at all.
    Absent,
}

fn scan_section(path: &Path) -> Scan {
    std::fs::read_to_string(path).map_or(Scan::Absent, |content| scan_text(&content))
}

fn scan_text(content: &str) -> Scan {
    let (Some(start), Some(end)) = (content.find(MARKER_START), content.find(MARKER_END)) else {
        return Scan::Absent;
    };
    if end < start {
        return Scan::Absent;
    }
    let section = &content[start..end];
    if section.contains(&format!("seeds-onboard-schema:{SCHEMA_VERSION}")) {
        Scan::Current
    } else {
        Scan::Outdated
    }
}

/// Replaces the span from the start marker through the end marker
/// (inclusive) with `section`, preserving everything around it.
fn replace_between_markers(content: &str, section: &str) -> String {
    let start = content.find(MARKER_START).unwrap_or(0);
    let end = content
        .find(MARKER_END)
        .map_or(content.len(), |at| at + MARKER_END.len());
    let mut out = String::with_capacity(content.len());
    out.push_str(&content[..start]);
    out.push_str(section);
    out.push_str(&content[end..]);
    out
}

/// The tracker section this build writes: the reference's schema-7
/// structure with the live CLI named `seeds` (README DEVIATIONS), the
/// version from `CARGO_PKG_VERSION`, and this implementation's home.
fn section_text() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "\
{MARKER_START}
## Issue Tracking (Seeds)
<!-- seeds-onboard:v{version} -->
<!-- seeds-onboard-schema:{SCHEMA_VERSION} -->

This project uses [Seeds](https://github.com/denkhaus/seeds) v{version} for git-native issue tracking.

**At the start of every session**, run:
```
seeds prime
```

This injects session context: rules, command reference, and workflows. Pass `--format json|compact|markdown|plain|ids` on any command for agent-friendly output.

**Quick reference:**
- `seeds ready` — Find unblocked work
- `seeds search <query>` — Full-text search across titles + descriptions
- `seeds create --title \"...\" --type task --priority 2` — Create issue
- `seeds update <id> --status in_progress` — Claim work
- `seeds close <id>` — Complete work
- `seeds dep add <id> <depends-on>` — Add dependency between issues
- `seeds sync` — Sync with git (run before pushing)

### Planning
Use `seeds plan` when work is large or ambiguous enough that an LLM benefits from structured decomposition. Submit spawns one child seed per step; `step.blocks` uses forward semantics (step i with `blocks: [j]` means step i blocks step j, and step j gets step i's id in its `blockedBy`).

- `seeds plan templates` — List built-ins (`feature`, `bug`, `refactor`) plus custom templates
- `seeds plan prompt <seed-id>` — Emit a structured prompt the LLM fills
- `seeds plan submit <seed-id> --plan <file>` — Validate + spawn child seeds
- `seeds plan show <pl-id>` — View sections, children, sub-plans
- `seeds plan edit <id> [--name | --section <name> <text> | --step <i> --title/--priority/--type]` — In-place field edits; bumps revision
- `seeds plan outcome <pl-id> --result success|partial|failure` — Record outcome (storage-only)
- `seeds plan review <pl-id> --by <name>` — Record reviewer (informational)

### Before You Finish
1. Close completed issues: `seeds close <id>`
2. File issues for remaining work: `seeds create --title \"...\"`
3. Sync and push: `seeds sync && git push`
{MARKER_END}"
    )
}
