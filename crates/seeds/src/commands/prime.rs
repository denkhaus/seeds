//! prime command semantics (sd 0.5.15-compatible): the session-context
//! template and its JSON sections, captured from the reference and
//! renamed to the live `seeds` CLI (documented deviation, README
//! DEVIATIONS — agents have no `sd` on PATH since the self-hosting
//! cutover).

use serde_json::{Value, json};

use super::{CommandContext, CommandOutcome, envelope_pretty, push_line};
use crate::{Pace, PaceOptions};

const PRIME_FULL: &str = "\
# Seeds Workflow Context

> **Context Recovery**: Run `seeds prime` after compaction, clear, or new session

# Session Close Protocol

**CRITICAL**: Before saying \"done\" or \"complete\", you MUST run this checklist:

```
[ ] 1. Close completed issues:    seeds close <id1> <id2> ...
[ ] 2. File issues for remaining:  seeds create --title \"...\"
[ ] 3. Run quality gates:          bun test && bun run lint && bun run typecheck
[ ] 4. Sync and push:              seeds sync && git push
[ ] 5. Verify:                     git status (must show \"up to date with origin\")
```

**NEVER skip this.** Work is not done until pushed.

## Core Rules
- **Default**: Use seeds for ALL task tracking (`seeds create`, `seeds ready`, `seeds close`)
- **Prohibited**: Do NOT use TodoWrite, TaskCreate, or markdown files for task tracking
- **Workflow**: Create issues BEFORE writing code, mark in_progress when starting
- Git workflow: run `seeds sync` at session end

## Essential Commands

### Finding Work
- `seeds ready` — Show issues ready to work (no blockers)
- `seeds list --status=open` — All open issues
- `seeds list --status=in_progress` — Your active work
- `seeds show <id> [<id2> ...]` — Detailed issue view; multi-id shows each separated by a divider (`--json` returns `issues: [...]`)

### Creating & Updating
- `seeds create --title=\"...\" --type=task|bug|feature|epic --priority=2` — New issue
  - Priority: 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog)
- `seeds update <id> --status=in_progress` — Claim work
- `seeds update <id> --assignee=username` — Assign to someone
- `seeds close <id>` — Mark complete
- `seeds close <id1> <id2> ...` — Close multiple issues at once

### Dependencies & Blocking
- `seeds dep add <issue> <depends-on>` — Add dependency
- `seeds dep remove <issue> <depends-on>` — Remove dependency
- `seeds blocked` — Show all blocked issues

### Labels
- `seeds label add <id> bug ui` — Add labels to an issue
- `seeds label remove <id> bug` — Remove labels
- `seeds label list <id>` — List labels on an issue
- `seeds label list-all` — Show all labels in project
- `seeds list --label=bug` — Filter by label (AND, comma-separated)
- `seeds list --label-any=bug,ui` — Filter by label (OR)
- `seeds list --unlabeled` — Issues with no labels
- `seeds create --title=\"...\" --labels=bug,ui` — Create with labels

### Sync & Project Health
- `seeds sync` — Stage and commit .seeds/ changes
- `seeds sync --status` — Check without committing
- `seeds stats` — Project statistics
- `seeds doctor` — Check for data integrity issues

### Planning
Use `seeds plan` when work is large or ambiguous enough to benefit from structured decomposition. The plan spawns one child seed per step; `step.blocks` uses forward semantics (step i with `blocks: [j]` means step i blocks step j). Each step accepts an optional `labels: string[]` field (normalized lowercase/trim/dedup) that flows to the spawned child or merges additively into an adopted seed — useful for tagging agent-spawned children (e.g. `\"labels\": [\"nightwatch\"]`) without follow-up `seeds label add` calls. For small, well-scoped tasks, just `seeds create` directly.

- `seeds plan templates` — List built-in templates (`feature`, `bug`, `refactor`) plus custom ones
- `seeds plan prompt <seed-id>` — Emit prompt JSON for the LLM to fill
- `seeds plan submit <seed-id> --plan <file>` — Validate + spawn children
- `seeds plan show <pl-id>` — Sections, children, nested sub-plans
- `seeds plan create <seed-id>` — Adopt-only plan (zero spawned children); populate via 'seeds plan adopt' + 'seeds plan reorder'
- `seeds plan adopt <pl-id> <seed-id...> [--step|--at|--before|--after]` — Adopt existing seeds into a plan; --at/--before/--after control children position
- `seeds plan reorder <pl-id> <seed-id...>` — Set the exact plan.children order (permutation of current children)
- `seeds plan edit <id> [--name|--section <n> <t>|--step <i> --title/--priority/--type]` — In-place field edits; bumps revision. Structural changes still need --overwrite.
- `seeds plan outcome <pl-id> --result success|partial|failure` — Storage-only outcome
- `seeds plan review <pl-id> --by <name>` — Optional reviewer (informational)

## Common Workflows

**Starting work:**
```bash
seeds ready                              # Find available work
seeds show <id>                          # Review issue details
seeds update <id> --status=in_progress   # Claim it
```

**Completing work:**
```bash
seeds close <id1> <id2> ...    # Close all completed issues at once
seeds sync                     # Stage + commit .seeds/
git push                    # Push to remote
```

**Creating dependent work:**
```bash
seeds create --title=\"Implement feature X\" --type=feature
seeds create --title=\"Write tests for X\" --type=task
seeds dep add <test-id> <feature-id>   # Tests depend on feature
```";

const PRIME_COMPACT: &str = "\
# Seeds Quick Reference

```
seeds ready                  # Find unblocked work
seeds show <id> [id...]      # View one or more issues
seeds create --title \"...\"   # Create issue (--type, --priority)
seeds update <id> --status in_progress # Claim work
seeds close <id>             # Complete work
seeds dep add <a> <b>        # a depends on b
seeds blocked                # Show blocked issues
seeds label add <id> <l...>  # Add labels
seeds list --label=bug       # Filter by label
seeds plan prompt <seed>     # Plan large/ambiguous work; spawns child seeds
seeds plan submit <seed> --plan <file> # Submit + spawn children
seeds sync                   # Stage + commit .seeds/
```

**Planning:** Use `seeds plan` for ambiguous or large work — built-in templates: `feature`, `bug`, `refactor`.

**Before finishing:** `seeds close <ids> && seeds sync && git push`";

const PRIME_JSON_FULL_EXTRA: &str = r#"{"commandGroups":[{"name":"Finding Work","commands":[{"command":"seeds ready","description":"Show issues ready to work (no blockers)"},{"command":"seeds list --status=open","description":"All open issues"},{"command":"seeds list --status=in_progress","description":"Your active work"},{"command":"seeds show <id> [<id2> ...]","description":"Detailed issue view; multi-id shows each separated by a divider (`--json` returns `issues: [...]`)"}]},{"name":"Creating & Updating","commands":[{"command":"seeds create --title=\"...\" --type=task|bug|feature|epic --priority=2","description":"New issue\n  - Priority: 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog)"},{"command":"seeds update <id> --status=in_progress","description":"Claim work"},{"command":"seeds update <id> --assignee=username","description":"Assign to someone"},{"command":"seeds close <id>","description":"Mark complete"},{"command":"seeds close <id1> <id2> ...","description":"Close multiple issues at once"}]},{"name":"Dependencies & Blocking","commands":[{"command":"seeds dep add <issue> <depends-on>","description":"Add dependency"},{"command":"seeds dep remove <issue> <depends-on>","description":"Remove dependency"},{"command":"seeds blocked","description":"Show all blocked issues"}]},{"name":"Labels","commands":[{"command":"seeds label add <id> bug ui","description":"Add labels to an issue"},{"command":"seeds label remove <id> bug","description":"Remove labels"},{"command":"seeds label list <id>","description":"List labels on an issue"},{"command":"seeds label list-all","description":"Show all labels in project"},{"command":"seeds list --label=bug","description":"Filter by label (AND, comma-separated)"},{"command":"seeds list --label-any=bug,ui","description":"Filter by label (OR)"},{"command":"seeds list --unlabeled","description":"Issues with no labels"},{"command":"seeds create --title=\"...\" --labels=bug,ui","description":"Create with labels"}]},{"name":"Sync & Project Health","commands":[{"command":"seeds sync","description":"Stage and commit .seeds/ changes"},{"command":"seeds sync --status","description":"Check without committing"},{"command":"seeds stats","description":"Project statistics"},{"command":"seeds doctor","description":"Check for data integrity issues"}]},{"name":"Planning","notes":["Use `seeds plan` when work is large or ambiguous enough to benefit from structured decomposition. The plan spawns one child seed per step; `step.blocks` uses forward semantics (step i with `blocks: [j]` means step i blocks step j). Each step accepts an optional `labels: string[]` field (normalized lowercase/trim/dedup) that flows to the spawned child or merges additively into an adopted seed — useful for tagging agent-spawned children (e.g. `\"labels\": [\"nightwatch\"]`) without follow-up `seeds label add` calls. For small, well-scoped tasks, just `seeds create` directly."],"commands":[{"command":"seeds plan templates","description":"List built-in templates (`feature`, `bug`, `refactor`) plus custom ones"},{"command":"seeds plan prompt <seed-id>","description":"Emit prompt JSON for the LLM to fill"},{"command":"seeds plan submit <seed-id> --plan <file>","description":"Validate + spawn children"},{"command":"seeds plan show <pl-id>","description":"Sections, children, nested sub-plans"},{"command":"seeds plan create <seed-id>","description":"Adopt-only plan (zero spawned children); populate via 'seeds plan adopt' + 'seeds plan reorder'"},{"command":"seeds plan adopt <pl-id> <seed-id...> [--step|--at|--before|--after]","description":"Adopt existing seeds into a plan; --at/--before/--after control children position"},{"command":"seeds plan reorder <pl-id> <seed-id...>","description":"Set the exact plan.children order (permutation of current children)"},{"command":"seeds plan edit <id> [--name|--section <n> <t>|--step <i> --title/--priority/--type]","description":"In-place field edits; bumps revision. Structural changes still need --overwrite."},{"command":"seeds plan outcome <pl-id> --result success|partial|failure","description":"Storage-only outcome"},{"command":"seeds plan review <pl-id> --by <name>","description":"Optional reviewer (informational)"}]}],"workflows":[{"name":"Starting work","commands":["seeds ready                              # Find available work","seeds show <id>                          # Review issue details","seeds update <id> --status=in_progress   # Claim it"]},{"name":"Completing work","commands":["seeds close <id1> <id2> ...    # Close all completed issues at once","seeds sync                     # Stage + commit .seeds/","git push                    # Push to remote"]},{"name":"Creating dependent work","commands":["seeds create --title=\"Implement feature X\" --type=feature","seeds create --title=\"Write tests for X\" --type=task","seeds dep add <test-id> <feature-id>   # Tests depend on feature"]}]}"#;

/// The compact-mode `seeds prime --compact --json` sections, captured
/// from sd 0.5.15 (a different section set, not a subset) with the
/// command examples renamed to the live `seeds` CLI (README
/// DEVIATIONS).
const PRIME_JSON_COMPACT: &str = r#"{"mode":"compact","title":"Seeds Quick Reference","commands":[{"command":"seeds ready","description":"Find unblocked work"},{"command":"seeds show <id> [id...]","description":"View one or more issues"},{"command":"seeds create --title \"...\"","description":"Create issue (--type, --priority)"},{"command":"seeds update <id> --status in_progress","description":"Claim work"},{"command":"seeds close <id>","description":"Complete work"},{"command":"seeds dep add <a> <b>","description":"a depends on b"},{"command":"seeds blocked","description":"Show blocked issues"},{"command":"seeds label add <id> <l...>","description":"Add labels"},{"command":"seeds list --label=bug","description":"Filter by label"},{"command":"seeds plan prompt <seed>","description":"Plan large/ambiguous work; spawns child seeds"},{"command":"seeds plan submit <seed> --plan <file>","description":"Submit + spawn children"},{"command":"seeds sync","description":"Stage + commit .seeds/"}],"planningNote":"**Planning:** Use `seeds plan` for ambiguous or large work — built-in templates: `feature`, `bug`, `refactor`.","closingNote":"**Before finishing:** `seeds close <ids> && seeds sync && git push`"}"#;

/// Typed input for `prime`.
#[derive(Clone, Debug, Default)]
pub struct PrimeInput {
    /// `--compact` (the compact template and section set).
    pub compact: bool,
    /// JSON envelope output.
    pub json:    bool,
}

/// The full template's context-recovery line — the anchor where the
/// pace line lands in full mode.
const CONTEXT_RECOVERY: &str =
    "> **Context Recovery**: Run `seeds prime` after compaction, clear, or new session";

/// The compact template's title line — its pace-line anchor.
const COMPACT_TITLE: &str = "# Seeds Quick Reference";

/// `prime` — emits the session-context template (text or JSON) with
/// the one-line pace summary of the open store (seeds-8fc5): a round
/// trip starts by knowing where the project stands. The pace line is
/// best-effort — a context without a readable store emits the
/// reference-faithful template unchanged.
pub fn prime(ctx: &CommandContext, input: &PrimeInput) -> CommandOutcome {
    let pace = pace_line(ctx);
    let mut out = String::new();
    if input.json {
        let compact = input.compact;
        // The reference's JSON section SHAPES (seeds-25b5): full mode
        // is the five core sections PLUS the captured
        // commandGroups/workflows; compact mode is a different section
        // set entirely. The command examples inside name the live
        // `seeds` CLI (documented deviation, README DEVIATIONS).
        let sections = if compact {
            serde_json::from_str::<Value>(PRIME_JSON_COMPACT)
                .expect("captured compact sections are valid JSON")
        } else {
            let mut sections = json!({
                "mode": "full",
                "title": "Seeds Workflow Context",
                "contextRecovery": "Run `seeds prime` after compaction, clear, or new session",
                "closeProtocol": {
                    "warning": "Before saying \"done\" or \"complete\", you MUST run this checklist:",
                    "steps": [
                        "Close completed issues:    seeds close <id1> <id2> ...",
                        "File issues for remaining:  seeds create --title \"...\"",
                        "Run quality gates:          bun test && bun run lint && bun run typecheck",
                        "Sync and push:              seeds sync && git push",
                        "Verify:                     git status (must show \"up to date with origin\")"
                    ],
                    "footer": "**NEVER skip this.** Work is not done until pushed."
                },
                "rules": [
                    "**Default**: Use seeds for ALL task tracking (`seeds create`, `seeds ready`, `seeds close`)",
                    "**Prohibited**: Do NOT use TodoWrite, TaskCreate, or markdown files for task tracking",
                    "**Workflow**: Create issues BEFORE writing code, mark in_progress when starting",
                    "Git workflow: run `seeds sync` at session end"
                ]
            });
            let extra = serde_json::from_str::<Value>(PRIME_JSON_FULL_EXTRA)
                .expect("captured extra sections are valid JSON");
            let Value::Object(map) = &mut sections else {
                unreachable!("sections is an object");
            };
            let Value::Object(extra) = extra else {
                unreachable!("captured extra sections are an object");
            };
            for (key, value) in extra {
                map.insert(key, value);
            }
            sections
        };
        // The reference's envelope carries BOTH the structured sections
        // and the raw template text under `content` (full or compact
        // template matching the mode, trailing newline included). The
        // pace line rides as its own top-level key BEFORE `content`, so
        // dropping that one line yields the reference envelope shape
        // (the tailored differential strips it).
        let content = format!("{}\n", if compact { PRIME_COMPACT } else { PRIME_FULL });
        let mut extra = vec![("sections", sections)];
        if let Some(pace) = &pace {
            extra.push(("pace", json!(pace)));
        }
        extra.push(("content", json!(content)));
        push_line(&mut out, &envelope_pretty("prime", &extra));
    } else if input.compact {
        push_line(
            &mut out,
            &with_pace_line(PRIME_COMPACT, COMPACT_TITLE, pace.as_deref()),
        );
    } else {
        // `--export` emits the default (full) template, like the reference.
        push_line(
            &mut out,
            &with_pace_line(PRIME_FULL, CONTEXT_RECOVERY, pace.as_deref()),
        );
    }
    CommandOutcome::ok(out)
}

/// The compact pace line for `prime`: default window (30d), the open
/// target, and the ETA — `None` when the store will not open or the
/// metrics will not compute (prime must render regardless).
fn pace_line(ctx: &CommandContext) -> Option<String> {
    let store = ctx.open_store().ok()?;
    Pace::collect(&store.issues, &PaceOptions::default())
        .ok()
        .map(|pace| pace.line())
}

/// Inserts `> **Pace**: …` directly below `marker` in `template`;
/// without a pace line (or a missing anchor, caught by tests) the
/// template comes back untouched.
fn with_pace_line(template: &str, marker: &str, pace: Option<&str>) -> String {
    let Some(pace) = pace else {
        return template.to_owned();
    };
    let anchor = format!("{marker}\n");
    template.find(&anchor).map_or_else(
        || template.to_owned(),
        |index| {
            let mut out = String::with_capacity(template.len() + pace.len() + 24);
            out.push_str(&template[..index + anchor.len()]);
            let line = format!("> **Pace**: {pace}\n");
            out.push_str(&line);
            out.push_str(&template[index + anchor.len()..]);
            out
        },
    )
}
