//! Reference help and template texts, mirroring sd 0.5.15 output.

pub(crate) const GLOBAL: &str = "\
seeds v0.5.15 — Git-native issue tracking

Usage: sd <command> [options]

Commands (implemented in this build):
  create            Create a new issue
  show <id> [ids]   Show one or more issues
  list              List issues with filters
  ready             Show open issues with no unresolved blockers
  search <query>    Full-text search title + description
  update <id>       Update issue fields
  close <id> [ids]  Close one or more issues
  dep add           Add an issue dependency (dep remove/list: not yet)
  prime             Output AI agent context
  dedupe            Report and heal duplicate ids in .seeds JSONL stores
  sync              Stage and commit .seeds/ changes

Unimplemented reference commands (init, label, blocked, stats,
doctor, tpl, migrate-from-beads, onboard, upgrade, completions, block,
unblock, plan, config) answer 'not implemented yet' when invoked.

Options:
  -h, --help        Show help
  -v, --version     Print version
  --format <mode>   Output format (markdown|compact|plain|ids|json)
  --json            Alias for --format json
  -q, --quiet       Suppress non-error output
  --verbose         Extra diagnostic output
  --timing          Show command execution time

Run 'sd <command> --help' for command-specific help.";

/// sd-0.5.15 surface commands this build does NOT implement yet (help
/// honesty, seeds-25b5): `seeds --help` lists only implemented commands,
/// and invoking one of these answers with a clear "not implemented yet"
/// message instead of a generic unknown-command error.
pub(crate) const PLANNED: &[&str] = &[
    "init",
    "label",
    "blocked",
    "stats",
    "doctor",
    "tpl",
    "migrate-from-beads",
    "onboard",
    "upgrade",
    "completions",
    "block",
    "unblock",
    "plan",
    "config",
];

pub(crate) const CREATE: &str = "\
Usage: sd create [options]

Create a new issue

Options:
  --title <text>        Issue title
  --type <type>         Issue type (task|bug|feature|epic) (default: \"task\")
  --priority <n>        Priority 0-4 or P0-P4 (default: \"2\")
  --assignee <name>     Assignee name
  --description <text>  Issue description
  --desc <text>         Issue description (alias for --description)
  --body <text>         Issue description (alias for --description)
  --labels <labels>     Comma-separated labels
  --json                Output as JSON
  -h, --help            display help for command";

pub(crate) const SHOW: &str = "\
Usage: sd show [options] <id> [ids...]

Show one or more issues

Options:
  --format <mode>  Output format (markdown|compact|plain|ids|json)
  --json           Output as JSON (alias for --format json)
  -h, --help       display help for command";

pub(crate) const LIST: &str = "\
Usage: sd list [options]

List issues with filters

Options:
  --status <status>     Filter by status (open|in_progress|closed)
  --type <type>         Filter by type (task|bug|feature|epic)
  --assignee <name>     Filter by assignee
  --all                 Include closed issues (default: only open/in_progress)
  --label <labels>      Filter: must have ALL labels (comma-separated, AND)
  --label-any <labels>  Filter: must have any label (comma-separated, OR)
  --unlabeled           Filter: issues with no labels
  --priority <levels>   Filter by priority (comma-separated, e.g. 0,1 or P0,P1)
  --priority-max <n>    Filter to priority <= n (e.g. --priority-max 1 = P0+P1)
  --limit <n>           Max issues to show (default: \"50\")
  --sort <mode>         Sort order (priority|created|updated|id) (default:
                        \"priority\")
  --format <mode>       Output format (markdown|compact|plain|ids|json)
  --json                Output as JSON (alias for --format json)
  -h, --help            display help for command";

pub(crate) const READY: &str = "\
Usage: sd ready [options]

Show open issues with no unresolved blockers

Options:
  --type <type>         Filter by type (task|bug|feature|epic)
  --assignee <name>     Filter by assignee
  --label <labels>      Filter: must have ALL labels (comma-separated, AND)
  --label-any <labels>  Filter: must have any label (comma-separated, OR)
  --unlabeled           Filter: issues with no labels
  --priority <levels>   Filter by priority (comma-separated, e.g. 0,1 or P0,P1)
  --priority-max <n>    Filter to priority <= n (e.g. --priority-max 1 = P0+P1)
  --limit <n>           Max issues to show (default: \"50\")
  --sort <mode>         Sort order (priority|created|updated|id) (default:
                        \"priority\")
  --format <mode>       Output format (markdown|compact|plain|ids|json)
  --json                Output as JSON (alias for --format json)
  --respect-schedule    Exclude issues with extensions.queued=true or future
                        extensions.scheduledFor
  -h, --help            display help for command";

pub(crate) const SEARCH: &str = "\
Usage: sd search [options] <query>

Full-text search title + description

Arguments:
  query                 Case-insensitive substring to match

Options:
  --status <status>     Filter by status (open|in_progress|closed)
  --type <type>         Filter by type (task|bug|feature|epic)
  --assignee <name>     Filter by assignee
  --label <labels>      Filter: must have ALL labels (comma-separated, AND)
  --label-any <labels>  Filter: must have any label (comma-separated, OR)
  --unlabeled           Filter: issues with no labels
  --priority <levels>   Filter by priority (comma-separated, e.g. 0,1 or P0,P1)
  --priority-max <n>    Filter to priority <= n (e.g. --priority-max 1 = P0+P1)
  --limit <n>           Max issues to show (default: \"50\")
  --sort <mode>         Sort order (priority|created|updated|id) (default:
                        \"priority\")
  --format <mode>       Output format (markdown|compact|plain|ids|json)
  --json                Output as JSON (alias for --format json)
  -h, --help            display help for command";

pub(crate) const UPDATE: &str = "\
Usage: sd update [options] <id>

Update issue fields

Options:
  --status <status>        New status (open|in_progress|closed)
  --title <text>           New title
  --assignee <name>        New assignee
  --description <text>     New description
  --desc <text>            New description (alias for --description)
  --body <text>            New description (alias for --description)
  --type <type>            New type (task|bug|feature|epic)
  --priority <n>           New priority 0-4 or P0-P4
  --add-label <labels>     Add label(s) (comma-separated)
  --remove-label <labels>  Remove label(s) (comma-separated)
  --set-labels <labels>    Set labels (comma-separated, empty to clear)
  --extensions <json>      Shallow-merge JSON object into Issue.extensions
  --clear-extensions       Remove the extensions field
  --json                   Output as JSON
  -h, --help               display help for command";

pub(crate) const CLOSE: &str = "\
Usage: sd close [options] <id> [ids...]

Close one or more issues

Options:
  --reason <text>  Close reason
  --json           Output as JSON
  -h, --help       display help for command";

pub(crate) const DEP: &str = "\
Usage: sd dep [options] [command]

Manage issue dependencies

Options:
  -h, --help                             display help for command

Commands:
  add [options] <issue> <depends-on>     Add a dependency (issue depends on depends-on)
  remove [options] <issue> <depends-on>  Remove a dependency
  list [options] <issue>                 Show dependencies for an issue
  help [command]                         display help for command";

pub(crate) const DEP_ADD: &str = "\
Usage: sd dep add [options] <issue> <depends-on>

Add a dependency (issue depends on depends-on)

Options:
  --json  Output as JSON
  -h, --help  display help for command";

pub(crate) const PRIME: &str = "\
Usage: sd prime [options]

Output AI agent context

Options:
  --compact   Condensed quick-reference output
  --export    Output the default template
  --json      Output as JSON
  -h, --help  display help for command";

pub(crate) const PRIME_FULL: &str = "\
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

pub(crate) const PRIME_COMPACT: &str = "\
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

pub(crate) const DEDUPE: &str = "\
Usage: sd dedupe [options]

Report and heal duplicate record ids in the tracker JSONL stores
(issues, plans, templates) — a native seeds addition beyond sd parity.

Semantics: one record per id per file; the record with the newest
updatedAt wins (ties: later line wins); survivors keep first-seen
order.

Options:
  --write              Apply the heal in place (atomic temp-file + rename)
  --json               Output as JSON
  -h, --help           display help for command";

pub(crate) const SYNC: &str = "\
Usage: sd sync [options]

Stage and commit .seeds/ changes

Options:
  --status    Check status without committing
  --dry-run   Show what would be committed without committing
  --json      Output as JSON
  --force     Commit even when the fabro push gate refuses
  -h, --help  display help for command";

/// The `sd prime --json` sections beyond the five core ones (mode,
/// title, contextRecovery, closeProtocol, rules), captured verbatim
/// from sd 0.5.15 — pinned by the differential battery (seeds-25b5).
pub(crate) const PRIME_JSON_FULL_EXTRA: &str = r#"{"commandGroups":[{"name":"Finding Work","commands":[{"command":"sd ready","description":"Show issues ready to work (no blockers)"},{"command":"sd list --status=open","description":"All open issues"},{"command":"sd list --status=in_progress","description":"Your active work"},{"command":"sd show <id> [<id2> ...]","description":"Detailed issue view; multi-id shows each separated by a divider (`--json` returns `issues: [...]`)"}]},{"name":"Creating & Updating","commands":[{"command":"sd create --title=\"...\" --type=task|bug|feature|epic --priority=2","description":"New issue\n  - Priority: 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog)"},{"command":"sd update <id> --status=in_progress","description":"Claim work"},{"command":"sd update <id> --assignee=username","description":"Assign to someone"},{"command":"sd close <id>","description":"Mark complete"},{"command":"sd close <id1> <id2> ...","description":"Close multiple issues at once"}]},{"name":"Dependencies & Blocking","commands":[{"command":"sd dep add <issue> <depends-on>","description":"Add dependency"},{"command":"sd dep remove <issue> <depends-on>","description":"Remove dependency"},{"command":"sd blocked","description":"Show all blocked issues"}]},{"name":"Labels","commands":[{"command":"sd label add <id> bug ui","description":"Add labels to an issue"},{"command":"sd label remove <id> bug","description":"Remove labels"},{"command":"sd label list <id>","description":"List labels on an issue"},{"command":"sd label list-all","description":"Show all labels in project"},{"command":"sd list --label=bug","description":"Filter by label (AND, comma-separated)"},{"command":"sd list --label-any=bug,ui","description":"Filter by label (OR)"},{"command":"sd list --unlabeled","description":"Issues with no labels"},{"command":"sd create --title=\"...\" --labels=bug,ui","description":"Create with labels"}]},{"name":"Sync & Project Health","commands":[{"command":"sd sync","description":"Stage and commit .seeds/ changes"},{"command":"sd sync --status","description":"Check without committing"},{"command":"sd stats","description":"Project statistics"},{"command":"sd doctor","description":"Check for data integrity issues"}]},{"name":"Planning","notes":["Use `sd plan` when work is large or ambiguous enough to benefit from structured decomposition. The plan spawns one child seed per step; `step.blocks` uses forward semantics (step i with `blocks: [j]` means step i blocks step j). Each step accepts an optional `labels: string[]` field (normalized lowercase/trim/dedup) that flows to the spawned child or merges additively into an adopted seed — useful for tagging agent-spawned children (e.g. `\"labels\": [\"nightwatch\"]`) without follow-up `sd label add` calls. For small, well-scoped tasks, just `sd create` directly."],"commands":[{"command":"sd plan templates","description":"List built-in templates (`feature`, `bug`, `refactor`) plus custom ones"},{"command":"sd plan prompt <seed-id>","description":"Emit prompt JSON for the LLM to fill"},{"command":"sd plan submit <seed-id> --plan <file>","description":"Validate + spawn children"},{"command":"sd plan show <pl-id>","description":"Sections, children, nested sub-plans"},{"command":"sd plan create <seed-id>","description":"Adopt-only plan (zero spawned children); populate via 'sd plan adopt' + 'sd plan reorder'"},{"command":"sd plan adopt <pl-id> <seed-id...> [--step|--at|--before|--after]","description":"Adopt existing seeds into a plan; --at/--before/--after control children position"},{"command":"sd plan reorder <pl-id> <seed-id...>","description":"Set the exact plan.children order (permutation of current children)"},{"command":"sd plan edit <id> [--name|--section <n> <t>|--step <i> --title/--priority/--type]","description":"In-place field edits; bumps revision. Structural changes still need --overwrite."},{"command":"sd plan outcome <pl-id> --result success|partial|failure","description":"Storage-only outcome"},{"command":"sd plan review <pl-id> --by <name>","description":"Optional reviewer (informational)"}]}],"workflows":[{"name":"Starting work","commands":["sd ready                              # Find available work","sd show <id>                          # Review issue details","sd update <id> --status=in_progress   # Claim it"]},{"name":"Completing work","commands":["sd close <id1> <id2> ...    # Close all completed issues at once","sd sync                     # Stage + commit .seeds/","git push                    # Push to remote"]},{"name":"Creating dependent work","commands":["sd create --title=\"Implement feature X\" --type=feature","sd create --title=\"Write tests for X\" --type=task","sd dep add <test-id> <feature-id>   # Tests depend on feature"]}]}"#;

/// The compact-mode `sd prime --compact --json` sections, captured
/// verbatim from sd 0.5.15 (a different section set, not a subset).
pub(crate) const PRIME_JSON_COMPACT: &str = r#"{"mode":"compact","title":"Seeds Quick Reference","commands":[{"command":"sd ready","description":"Find unblocked work"},{"command":"sd show <id> [id...]","description":"View one or more issues"},{"command":"sd create --title \"...\"","description":"Create issue (--type, --priority)"},{"command":"sd update <id> --status in_progress","description":"Claim work"},{"command":"sd close <id>","description":"Complete work"},{"command":"sd dep add <a> <b>","description":"a depends on b"},{"command":"sd blocked","description":"Show blocked issues"},{"command":"sd label add <id> <l...>","description":"Add labels"},{"command":"sd list --label=bug","description":"Filter by label"},{"command":"sd plan prompt <seed>","description":"Plan large/ambiguous work; spawns child seeds"},{"command":"sd plan submit <seed> --plan <file>","description":"Submit + spawn children"},{"command":"sd sync","description":"Stage + commit .seeds/"}],"planningNote":"**Planning:** Use `sd plan` for ambiguous or large work — built-in templates: `feature`, `bug`, `refactor`.","closingNote":"**Before finishing:** `sd close <ids> && sd sync && git push`"}"#;
