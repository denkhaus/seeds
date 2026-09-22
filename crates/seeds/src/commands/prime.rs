//! prime command semantics (sd 0.5.15-compatible): the session-context
//! template and its JSON sections, verbatim from the reference.

use serde_json::{Value, json};

use super::{CommandOutcome, envelope_pretty, push_line};

const PRIME_FULL: &str = "\
# Seeds Workflow Context

> **Context Recovery**: Run `sd prime` after compaction, clear, or new session

# Session Close Protocol

**CRITICAL**: Before saying \"done\" or \"complete\", you MUST run this checklist:

```
[ ] 1. Close completed issues:    sd close <id1> <id2> ...
[ ] 2. File issues for remaining:  sd create --title \"...\"
[ ] 3. Run quality gates:          bun test && bun run lint && bun run typecheck
[ ] 4. Sync and push:              sd sync && git push
[ ] 5. Verify:                     git status (must show \"up to date with origin\")
```

**NEVER skip this.** Work is not done until pushed.

## Core Rules
- **Default**: Use seeds for ALL task tracking (`sd create`, `sd ready`, `sd close`)
- **Prohibited**: Do NOT use TodoWrite, TaskCreate, or markdown files for task tracking
- **Workflow**: Create issues BEFORE writing code, mark in_progress when starting
- Git workflow: run `sd sync` at session end

## Essential Commands

### Finding Work
- `sd ready` — Show issues ready to work (no blockers)
- `sd list --status=open` — All open issues
- `sd list --status=in_progress` — Your active work
- `sd show <id> [<id2> ...]` — Detailed issue view; multi-id shows each separated by a divider (`--json` returns `issues: [...]`)

### Creating & Updating
- `sd create --title=\"...\" --type=task|bug|feature|epic --priority=2` — New issue
  - Priority: 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog)
- `sd update <id> --status=in_progress` — Claim work
- `sd update <id> --assignee=username` — Assign to someone
- `sd close <id>` — Mark complete
- `sd close <id1> <id2> ...` — Close multiple issues at once

### Dependencies & Blocking
- `sd dep add <issue> <depends-on>` — Add dependency
- `sd dep remove <issue> <depends-on>` — Remove dependency
- `sd blocked` — Show all blocked issues

### Labels
- `sd label add <id> bug ui` — Add labels to an issue
- `sd label remove <id> bug` — Remove labels
- `sd label list <id>` — List labels on an issue
- `sd label list-all` — Show all labels in project
- `sd list --label=bug` — Filter by label (AND, comma-separated)
- `sd list --label-any=bug,ui` — Filter by label (OR)
- `sd list --unlabeled` — Issues with no labels
- `sd create --title=\"...\" --labels=bug,ui` — Create with labels

### Sync & Project Health
- `sd sync` — Stage and commit .seeds/ changes
- `sd sync --status` — Check without committing
- `sd stats` — Project statistics
- `sd doctor` — Check for data integrity issues

### Planning
Use `sd plan` when work is large or ambiguous enough to benefit from structured decomposition. The plan spawns one child seed per step; `step.blocks` uses forward semantics (step i with `blocks: [j]` means step i blocks step j). Each step accepts an optional `labels: string[]` field (normalized lowercase/trim/dedup) that flows to the spawned child or merges additively into an adopted seed — useful for tagging agent-spawned children (e.g. `\"labels\": [\"nightwatch\"]`) without follow-up `sd label add` calls. For small, well-scoped tasks, just `sd create` directly.

- `sd plan templates` — List built-in templates (`feature`, `bug`, `refactor`) plus custom ones
- `sd plan prompt <seed-id>` — Emit prompt JSON for the LLM to fill
- `sd plan submit <seed-id> --plan <file>` — Validate + spawn children
- `sd plan show <pl-id>` — Sections, children, nested sub-plans
- `sd plan create <seed-id>` — Adopt-only plan (zero spawned children); populate via 'sd plan adopt' + 'sd plan reorder'
- `sd plan adopt <pl-id> <seed-id...> [--step|--at|--before|--after]` — Adopt existing seeds into a plan; --at/--before/--after control children position
- `sd plan reorder <pl-id> <seed-id...>` — Set the exact plan.children order (permutation of current children)
- `sd plan edit <id> [--name|--section <n> <t>|--step <i> --title/--priority/--type]` — In-place field edits; bumps revision. Structural changes still need --overwrite.
- `sd plan outcome <pl-id> --result success|partial|failure` — Storage-only outcome
- `sd plan review <pl-id> --by <name>` — Optional reviewer (informational)

## Common Workflows

**Starting work:**
```bash
sd ready                              # Find available work
sd show <id>                          # Review issue details
sd update <id> --status=in_progress   # Claim it
```

**Completing work:**
```bash
sd close <id1> <id2> ...    # Close all completed issues at once
sd sync                     # Stage + commit .seeds/
git push                    # Push to remote
```

**Creating dependent work:**
```bash
sd create --title=\"Implement feature X\" --type=feature
sd create --title=\"Write tests for X\" --type=task
sd dep add <test-id> <feature-id>   # Tests depend on feature
```";

const PRIME_COMPACT: &str = "\
# Seeds Quick Reference

```
sd ready                  # Find unblocked work
sd show <id> [id...]      # View one or more issues
sd create --title \"...\"   # Create issue (--type, --priority)
sd update <id> --status in_progress # Claim work
sd close <id>             # Complete work
sd dep add <a> <b>        # a depends on b
sd blocked                # Show blocked issues
sd label add <id> <l...>  # Add labels
sd list --label=bug       # Filter by label
sd plan prompt <seed>     # Plan large/ambiguous work; spawns child seeds
sd plan submit <seed> --plan <file> # Submit + spawn children
sd sync                   # Stage + commit .seeds/
```

**Planning:** Use `sd plan` for ambiguous or large work — built-in templates: `feature`, `bug`, `refactor`.

**Before finishing:** `sd close <ids> && sd sync && git push`";

const PRIME_JSON_FULL_EXTRA: &str = r#"{"commandGroups":[{"name":"Finding Work","commands":[{"command":"sd ready","description":"Show issues ready to work (no blockers)"},{"command":"sd list --status=open","description":"All open issues"},{"command":"sd list --status=in_progress","description":"Your active work"},{"command":"sd show <id> [<id2> ...]","description":"Detailed issue view; multi-id shows each separated by a divider (`--json` returns `issues: [...]`)"}]},{"name":"Creating & Updating","commands":[{"command":"sd create --title=\"...\" --type=task|bug|feature|epic --priority=2","description":"New issue\n  - Priority: 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog)"},{"command":"sd update <id> --status=in_progress","description":"Claim work"},{"command":"sd update <id> --assignee=username","description":"Assign to someone"},{"command":"sd close <id>","description":"Mark complete"},{"command":"sd close <id1> <id2> ...","description":"Close multiple issues at once"}]},{"name":"Dependencies & Blocking","commands":[{"command":"sd dep add <issue> <depends-on>","description":"Add dependency"},{"command":"sd dep remove <issue> <depends-on>","description":"Remove dependency"},{"command":"sd blocked","description":"Show all blocked issues"}]},{"name":"Labels","commands":[{"command":"sd label add <id> bug ui","description":"Add labels to an issue"},{"command":"sd label remove <id> bug","description":"Remove labels"},{"command":"sd label list <id>","description":"List labels on an issue"},{"command":"sd label list-all","description":"Show all labels in project"},{"command":"sd list --label=bug","description":"Filter by label (AND, comma-separated)"},{"command":"sd list --label-any=bug,ui","description":"Filter by label (OR)"},{"command":"sd list --unlabeled","description":"Issues with no labels"},{"command":"sd create --title=\"...\" --labels=bug,ui","description":"Create with labels"}]},{"name":"Sync & Project Health","commands":[{"command":"sd sync","description":"Stage and commit .seeds/ changes"},{"command":"sd sync --status","description":"Check without committing"},{"command":"sd stats","description":"Project statistics"},{"command":"sd doctor","description":"Check for data integrity issues"}]},{"name":"Planning","notes":["Use `sd plan` when work is large or ambiguous enough to benefit from structured decomposition. The plan spawns one child seed per step; `step.blocks` uses forward semantics (step i with `blocks: [j]` means step i blocks step j). Each step accepts an optional `labels: string[]` field (normalized lowercase/trim/dedup) that flows to the spawned child or merges additively into an adopted seed — useful for tagging agent-spawned children (e.g. `\"labels\": [\"nightwatch\"]`) without follow-up `sd label add` calls. For small, well-scoped tasks, just `sd create` directly."],"commands":[{"command":"sd plan templates","description":"List built-in templates (`feature`, `bug`, `refactor`) plus custom ones"},{"command":"sd plan prompt <seed-id>","description":"Emit prompt JSON for the LLM to fill"},{"command":"sd plan submit <seed-id> --plan <file>","description":"Validate + spawn children"},{"command":"sd plan show <pl-id>","description":"Sections, children, nested sub-plans"},{"command":"sd plan create <seed-id>","description":"Adopt-only plan (zero spawned children); populate via 'sd plan adopt' + 'sd plan reorder'"},{"command":"sd plan adopt <pl-id> <seed-id...> [--step|--at|--before|--after]","description":"Adopt existing seeds into a plan; --at/--before/--after control children position"},{"command":"sd plan reorder <pl-id> <seed-id...>","description":"Set the exact plan.children order (permutation of current children)"},{"command":"sd plan edit <id> [--name|--section <n> <t>|--step <i> --title/--priority/--type]","description":"In-place field edits; bumps revision. Structural changes still need --overwrite."},{"command":"sd plan outcome <pl-id> --result success|partial|failure","description":"Storage-only outcome"},{"command":"sd plan review <pl-id> --by <name>","description":"Optional reviewer (informational)"}]}],"workflows":[{"name":"Starting work","commands":["sd ready                              # Find available work","sd show <id>                          # Review issue details","sd update <id> --status=in_progress   # Claim it"]},{"name":"Completing work","commands":["sd close <id1> <id2> ...    # Close all completed issues at once","sd sync                     # Stage + commit .seeds/","git push                    # Push to remote"]},{"name":"Creating dependent work","commands":["sd create --title=\"Implement feature X\" --type=feature","sd create --title=\"Write tests for X\" --type=task","sd dep add <test-id> <feature-id>   # Tests depend on feature"]}]}"#;

/// The compact-mode `sd prime --compact --json` sections, captured
/// verbatim from sd 0.5.15 (a different section set, not a subset).
const PRIME_JSON_COMPACT: &str = r#"{"mode":"compact","title":"Seeds Quick Reference","commands":[{"command":"sd ready","description":"Find unblocked work"},{"command":"sd show <id> [id...]","description":"View one or more issues"},{"command":"sd create --title \"...\"","description":"Create issue (--type, --priority)"},{"command":"sd update <id> --status in_progress","description":"Claim work"},{"command":"sd close <id>","description":"Complete work"},{"command":"sd dep add <a> <b>","description":"a depends on b"},{"command":"sd blocked","description":"Show blocked issues"},{"command":"sd label add <id> <l...>","description":"Add labels"},{"command":"sd list --label=bug","description":"Filter by label"},{"command":"sd plan prompt <seed>","description":"Plan large/ambiguous work; spawns child seeds"},{"command":"sd plan submit <seed> --plan <file>","description":"Submit + spawn children"},{"command":"sd sync","description":"Stage + commit .seeds/"}],"planningNote":"**Planning:** Use `sd plan` for ambiguous or large work — built-in templates: `feature`, `bug`, `refactor`.","closingNote":"**Before finishing:** `sd close <ids> && sd sync && git push`"}"#;

/// Typed input for `prime`.
#[derive(Clone, Debug, Default)]
pub struct PrimeInput {
    /// `--compact` (the compact template and section set).
    pub compact: bool,
    /// JSON envelope output.
    pub json:    bool,
}

/// `prime` — emits the session-context template (text or JSON).
pub fn prime(input: &PrimeInput) -> CommandOutcome {
    let mut out = String::new();
    if input.json {
        let compact = input.compact;
        // The reference's JSON sections (differential-pinned,
        // seeds-25b5): full mode is the five core sections PLUS the
        // captured commandGroups/workflows; compact mode is a different
        // section set entirely.
        let sections = if compact {
            serde_json::from_str::<Value>(PRIME_JSON_COMPACT)
                .expect("captured compact sections are valid JSON")
        } else {
            let mut sections = json!({
                "mode": "full",
                "title": "Seeds Workflow Context",
                "contextRecovery": "Run `sd prime` after compaction, clear, or new session",
                "closeProtocol": {
                    "warning": "Before saying \"done\" or \"complete\", you MUST run this checklist:",
                    "steps": [
                        "Close completed issues:    sd close <id1> <id2> ...",
                        "File issues for remaining:  sd create --title \"...\"",
                        "Run quality gates:          bun test && bun run lint && bun run typecheck",
                        "Sync and push:              sd sync && git push",
                        "Verify:                     git status (must show \"up to date with origin\")"
                    ],
                    "footer": "**NEVER skip this.** Work is not done until pushed."
                },
                "rules": [
                    "**Default**: Use seeds for ALL task tracking (`sd create`, `sd ready`, `sd close`)",
                    "**Prohibited**: Do NOT use TodoWrite, TaskCreate, or markdown files for task tracking",
                    "**Workflow**: Create issues BEFORE writing code, mark in_progress when starting",
                    "Git workflow: run `sd sync` at session end"
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
        // template matching the mode, trailing newline included).
        let content = format!("{}\n", if compact { PRIME_COMPACT } else { PRIME_FULL });
        push_line(
            &mut out,
            &envelope_pretty("prime", &[
                ("sections", sections),
                ("content", json!(content)),
            ]),
        );
    } else if input.compact {
        push_line(&mut out, PRIME_COMPACT);
    } else {
        // `--export` emits the default (full) template, like the reference.
        push_line(&mut out, PRIME_FULL);
    }
    CommandOutcome::ok(out)
}
