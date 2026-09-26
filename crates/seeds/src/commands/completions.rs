//! `completions` command semantics (sd 0.5.15-compatible surface):
//! emit a shell completion script for the IMPLEMENTED command surface.
//!
//! The reference's scripts enumerate its full command list including
//! commands this build does not implement; ours enumerate the
//! implemented surface only (the help-honesty rule, seeds-25b5) — a
//! documented deviation, so the scripts are generated here from the
//! same command table the help surface describes, never hand-copied
//! from the reference.

// Incremental `push_str(&format!(…))` script assembly reads clearer
// here than chained reassignment across per-shell generators.
#![allow(
    clippy::format_push_string,
    reason = "script assembly is clearer incrementally"
)]

use super::{CommandOutcome, push_line};

/// `seeds completions <shell>`: one of `bash`, `zsh`, `fish`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompletionsInput {
    /// The target shell name.
    pub shell: String,
}

/// The shells a script is emitted for, in the reference's error order.
const SHELLS: &[&str] = &["bash", "zsh", "fish"];

/// A completable command: name, one-line description (the help
/// surface's wording), and its fixed subcommand set when it is a group.
struct CommandSpec {
    name:        &'static str,
    description: &'static str,
    subcommands: &'static [&'static str],
}

/// The implemented command surface in help order.
const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name:        "create",
        description: "Create a new issue",
        subcommands: &[],
    },
    CommandSpec {
        name:        "show",
        description: "Show one or more issues",
        subcommands: &[],
    },
    CommandSpec {
        name:        "list",
        description: "List issues with filters",
        subcommands: &[],
    },
    CommandSpec {
        name:        "ready",
        description: "Show open issues with no unresolved blockers",
        subcommands: &[],
    },
    CommandSpec {
        name:        "update",
        description: "Update issue fields",
        subcommands: &[],
    },
    CommandSpec {
        name:        "close",
        description: "Close one or more issues",
        subcommands: &[],
    },
    CommandSpec {
        name:        "dep",
        description: "Manage issue dependencies",
        subcommands: &["add", "remove", "list"],
    },
    CommandSpec {
        name:        "blocked",
        description: "Show all blocked issues",
        subcommands: &[],
    },
    CommandSpec {
        name:        "block",
        description: "Add a blocker to an issue",
        subcommands: &[],
    },
    CommandSpec {
        name:        "unblock",
        description: "Remove blockers from an issue",
        subcommands: &[],
    },
    CommandSpec {
        name:        "label",
        description: "Manage issue labels",
        subcommands: &["add", "remove", "list", "list-all"],
    },
    CommandSpec {
        name:        "stats",
        description: "Project statistics",
        subcommands: &[],
    },
    CommandSpec {
        name:        "doctor",
        description: "Check project health and data integrity",
        subcommands: &[],
    },
    CommandSpec {
        name:        "prime",
        description: "Output AI agent context",
        subcommands: &[],
    },
    CommandSpec {
        name:        "search",
        description: "Full-text search title + description",
        subcommands: &[],
    },
    CommandSpec {
        name:        "dedupe",
        description: "Report and heal duplicate ids",
        subcommands: &[],
    },
    CommandSpec {
        name:        "sync",
        description: "Stage and commit .seeds/ changes",
        subcommands: &[],
    },
    CommandSpec {
        name:        "plan",
        description: "Plan management",
        subcommands: &[
            "templates",
            "prompt",
            "submit",
            "show",
            "create",
            "adopt",
            "reorder",
            "edit",
            "outcome",
            "review",
        ],
    },
    CommandSpec {
        name:        "init",
        description: "Initialize .seeds/ in current directory",
        subcommands: &[],
    },
    CommandSpec {
        name:        "config",
        description: "Read, write, and inspect .seeds/config.yaml",
        subcommands: &["show", "set", "unset", "schema", "help"],
    },
    CommandSpec {
        name:        "upgrade",
        description: "Upgrade seeds to the latest version from GitHub Releases",
        subcommands: &[],
    },
    CommandSpec {
        name:        "onboard",
        description: "Add seeds section to CLAUDE.md / AGENTS.md",
        subcommands: &[],
    },
    CommandSpec {
        name:        "completions",
        description: "Output shell completion script",
        subcommands: &[],
    },
];

/// Runs `completions`: prints the shell's script, or the reference's
/// unknown-shell error (stderr, exit 1).
#[must_use = "the outcome only takes effect when the caller prints it"]
pub fn completions(input: &CompletionsInput) -> CommandOutcome {
    let script = match input.shell.as_str() {
        "bash" => bash_script(),
        "zsh" => zsh_script(),
        "fish" => fish_script(),
        other => {
            return CommandOutcome {
                success: false,
                stdout:  String::new(),
                stderr:  format!(
                    "✗ Unknown shell: {other}. Supported: {}\n",
                    SHELLS.join(", ")
                ),
            };
        }
    };
    let mut stdout = String::new();
    stdout.push_str(&script);
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

/// Every command name in help order — the completion word list.
fn command_words() -> String {
    COMMANDS
        .iter()
        .map(|command| command.name)
        .collect::<Vec<_>>()
        .join(" ")
}

fn bash_script() -> String {
    let mut out = String::new();
    push_line(&mut out, "# bash completion for seeds");
    out.push_str(
        "\
_seeds_completions() {
    local cur prev
    cur=\"${COMP_WORDS[COMP_CWORD]}\"
    prev=\"${COMP_WORDS[COMP_CWORD-1]}\"

    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W \"",
    );
    out.push_str(&command_words());
    out.push_str(
        "\" -- \"$cur\") )\n        return 0\n    fi\n\n    case \"${COMP_WORDS[1]}\" in\n",
    );
    for command in COMMANDS
        .iter()
        .filter(|command| !command.subcommands.is_empty())
    {
        out.push_str(&format!("        {})\n", command.name));
        out.push_str("            if [[ ${COMP_CWORD} -eq 2 ]]; then\n");
        out.push_str(&format!(
            "                COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )\n",
            command.subcommands.join(" ")
        ));
        out.push_str("            fi\n            ;;\n");
    }
    out.push_str("    esac\n}\ncomplete -F _seeds_completions seeds\n");
    out
}

fn zsh_script() -> String {
    let mut out = String::new();
    push_line(&mut out, "#compdef seeds");
    out.push_str(
        "\
_seeds() {
    local -a commands
    commands=(
",
    );
    for command in COMMANDS {
        out.push_str(&format!(
            "        '{}:{}'\n",
            command.name, command.description
        ));
    }
    out.push_str("    )\n    _describe 'command' commands\n\n    case \"$words[2]\" in\n");
    for command in COMMANDS
        .iter()
        .filter(|command| !command.subcommands.is_empty())
    {
        out.push_str(&format!("        {})\n", command.name));
        out.push_str(&format!(
            "            local -a subs\n            subs=({})\n            _describe 'subcommand' subs\n            ;;\n",
            command
                .subcommands
                .iter()
                .map(|sub| format!("'{sub}'"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    out.push_str("    esac\n}\n\n_seeds \"$@\"\n");
    out
}

fn fish_script() -> String {
    let mut out = String::new();
    push_line(&mut out, "# fish completions for seeds");
    out.push_str("complete -c seeds -f\n\n");
    for command in COMMANDS {
        out.push_str(&format!(
            "complete -c seeds -n \"__fish_use_subcommand\" -a {} -d '{}'\n",
            command.name, command.description
        ));
    }
    out.push('\n');
    for command in COMMANDS
        .iter()
        .filter(|command| !command.subcommands.is_empty())
    {
        for sub in command.subcommands {
            out.push_str(&format!(
                "complete -c seeds -n \"__fish_seen_subcommand_from {}\" -a {}\n",
                command.name, sub
            ));
        }
    }
    out
}
