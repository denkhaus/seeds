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
  dep add           Add an issue dependency
  dep remove        Remove an issue dependency
  dep list          Show dependencies for an issue
  blocked           Show all blocked issues
  block <id>        Add a blocker to an issue
  unblock <id>      Remove blockers from an issue
  label add/remove  Manage issue labels (list, list-all)
  stats             Project statistics
  doctor            Check project health and data integrity
  prime             Output AI agent context
  dedupe            Report and heal duplicate ids in .seeds JSONL stores
  sync              Stage and commit .seeds/ changes

Unimplemented reference commands (init, tpl, migrate-from-beads,
onboard, upgrade, completions, plan, config) answer 'not implemented
yet' when invoked.

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
    "tpl",
    "migrate-from-beads",
    "onboard",
    "upgrade",
    "completions",
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

pub(crate) const DEP_REMOVE: &str = "\
Usage: sd dep remove [options] <issue> <depends-on>

Remove a dependency

Options:
  --json  Output as JSON
  -h, --help  display help for command";

pub(crate) const DEP_LIST: &str = "\
Usage: sd dep list [options] <issue>

Show dependencies for an issue

Options:
  --json  Output as JSON
  -h, --help  display help for command";

pub(crate) const BLOCKED: &str = "\
Usage: sd blocked [options]

Show all blocked issues

Options:
  --format <mode>  Output format (markdown|compact|plain|ids|json)
  --json           Output as JSON (alias for --format json)
  -h, --help       display help for command";

pub(crate) const BLOCK: &str = "\
Usage: sd block [options] <id>

Add a blocker to an issue

Arguments:
  id                 Issue ID to block

Options:
  --by <blocker-id>  Issue that blocks this issue
  --json             Output as JSON
  -h, --help         display help for command";

pub(crate) const UNBLOCK: &str = "\
Usage: sd unblock [options] <id>

Remove blockers from an issue

Arguments:
  id                   Issue ID to unblock

Options:
  --from <blocker-id>  Remove a specific blocker
  --all                Remove all closed blockers
  --json               Output as JSON
  -h, --help           display help for command";

pub(crate) const LABEL: &str = "\
Usage: sd label [options] [command]

Manage issue labels

Options:
  -h, --help                            display help for command

Commands:
  add [options] <issue> <labels...>     Add labels to an issue
  remove [options] <issue> <labels...>  Remove labels from an issue
  list [options] <issue>                List labels on an issue
  list-all [options]                    List all labels used in the project
  help [command]                        display help for command";

pub(crate) const LABEL_ADD: &str = "\
Usage: sd label add [options] <issue> <labels...>

Add labels to an issue

Options:
  --json      Output as JSON
  -h, --help  display help for command";

pub(crate) const LABEL_REMOVE: &str = "\
Usage: sd label remove [options] <issue> <labels...>

Remove labels from an issue

Options:
  --json      Output as JSON
  -h, --help  display help for command";

pub(crate) const LABEL_LIST: &str = "\
Usage: sd label list [options] <issue>

List labels on an issue

Options:
  --json      Output as JSON
  -h, --help  display help for command";

pub(crate) const LABEL_LIST_ALL: &str = "\
Usage: sd label list-all [options]

List all labels used in the project

Options:
  --json      Output as JSON
  -h, --help  display help for command";

pub(crate) const STATS: &str = "\
Usage: sd stats [options]

Project statistics

Options:
  --format <mode>  Output format (markdown|compact|plain|ids|json)
  --json           Output as JSON (alias for --format json)
  -h, --help       display help for command";

pub(crate) const DOCTOR: &str = "\
Usage: sd doctor [options]

Check project health and data integrity

Options:
  --fix            Auto-fix fixable issues
  --verbose        Show all check results including passes
  --repair-report  Name the exact repair for each fixable finding
                   (native addition beyond sd parity)
  --json           Output as JSON
  -h, --help       display help for command";

pub(crate) const PRIME: &str = "\
Usage: sd prime [options]

Output AI agent context

Options:
  --compact   Condensed quick-reference output
  --export    Output the default template
  --json      Output as JSON
  -h, --help  display help for command";

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
  -h, --help  display help for command";
