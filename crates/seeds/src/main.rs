//! The `seeds` binary: a thin argv/printf wrapper over the library's
//! command layer (seeds-0dfd). argv parsing (commander-style specs,
//! help handling) and printing live here; every command's semantics —
//! validation, store mutation, JSON envelope construction, text
//! rendering — lives in [`seeds::commands`].
//!
//! Parity surface (README compat contract, ADR-0023): create, show,
//! list, ready, update, close, dep add/remove/list, blocked, block,
//! unblock, label add/remove/list/list-all, stats, doctor, prime,
//! search, and the full `plan` decomposition surface (13 subcommands,
//! seeds-de37) — plus `init` and the `config` group
//! (schema/show/set/unset), `sync` with sd-parity behavior and the
//! README-documented deliberate improvements (per-file preview,
//! shortstat commit body), `dedupe` and doctor's
//! `--repair-report` as native additions.
//! Flag names, JSON envelope shapes (`{success, command, …}`), filter and
//! limit semantics, and error behavior (JSON `success:false` plus a
//! non-zero exit, exactly as the pinned reference behaves) mirror
//! `@os-eco/seeds-cli` 0.5.15.

#![allow(
    clippy::print_stdout,
    reason = "printing to stdout is this binary's purpose"
)]
#![allow(clippy::print_stderr, reason = "CLI diagnostics belong on stderr")]

use std::process::ExitCode;

#[cfg(feature = "upgrade")]
use seeds::commands::UpgradeInput;
use seeds::commands::{
    self, BlockInput, BlockedInput, CloseInput, CommandContext, CommandOutcome, CompletionsInput,
    ConfigInput, ConfigSub, CreateInput, DedupeInput, DepAddInput, DepListInput, DepRemoveInput,
    DoctorInput, InitInput, LabelAddInput, LabelListAllInput, LabelListInput, LabelRemoveInput,
    OnboardInput, PrimeInput, QueryCommand, QueryInput, StatsInput, SyncInput, UnblockInput,
    UpdateInput,
};

mod args;
mod helptext;

use args::{OptSpec, Parsed};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    dispatch(&argv)
}

fn dispatch(argv: &[String]) -> ExitCode {
    let Some(command) = argv.first() else {
        println!("{}", helptext::global());
        return ExitCode::SUCCESS;
    };
    let rest = &argv[1..];
    match command.as_str() {
        "-h" | "--help" => {
            println!("{}", helptext::global());
            ExitCode::SUCCESS
        }
        "-v" | "--version" => {
            // Version identity (seeds-e160): report the real crate
            // version; the sd-0.5.15 parity target is a separate line.
            // Release builds carry a build stamp (d54c step 1).
            println!(
                "seeds v{} — Git-native issue tracking",
                env!("CARGO_PKG_VERSION")
            );
            if let Some(sha) = option_env!("SEEDS_BUILD_SHA") {
                println!("build {sha}");
            }
            println!("sd-0.5.15 read+write compatible");
            ExitCode::SUCCESS
        }
        "create" => cmd_create(rest),
        "show" => cmd_show(rest),
        "list" => cmd_list(rest),
        "ready" => cmd_ready(rest),
        "update" => cmd_update(rest),
        "close" => cmd_close(rest),
        "dep" => cmd_dep(rest),
        "blocked" => cmd_blocked(rest),
        "block" => cmd_block(rest),
        "unblock" => cmd_unblock(rest),
        "label" => cmd_label(rest),
        "stats" => cmd_stats(rest),
        "doctor" => cmd_doctor(rest),
        "prime" => cmd_prime(rest),
        "search" => cmd_search(rest),
        "dedupe" => cmd_dedupe(rest),
        "sync" => cmd_sync(rest),
        "plan" => cmd_plan(rest),
        "init" => cmd_init(rest),
        "onboard" => cmd_onboard(rest),
        "completions" => cmd_completions(rest),
        #[cfg(feature = "upgrade")]
        "upgrade" => cmd_upgrade(rest),
        #[cfg(not(feature = "upgrade"))]
        "upgrade" => not_implemented_no_feature("upgrade"),
        "config" => cmd_config(rest),
        other => {
            // Help honesty (seeds-25b5): planned sd-parity commands
            // answer with a clear "not implemented yet", not a generic
            // unknown-command error.
            if helptext::PLANNED.contains(&other) {
                eprintln!(
                    "error: command '{other}' is not implemented yet (planned \
                     sd-0.5.15 parity) — run 'seeds --help' for the \
                     implemented commands"
                );
            } else {
                eprintln!("error: unknown command '{other}'");
            }
            ExitCode::FAILURE
        }
    }
}

/// Parses `args` against `spec`, honoring `-h/--help` by printing the
/// command's reference help text and exiting successfully.
fn parsed_or_help(args: &[String], spec: &[OptSpec], help: &str) -> Option<Parsed> {
    let mut filtered = Vec::with_capacity(args.len());
    let mut wants_help = false;
    for arg in args {
        if arg == "-h" || arg == "--help" {
            wants_help = true;
        } else {
            filtered.push(arg.clone());
        }
    }
    if wants_help {
        println!("{help}");
        std::process::exit(0);
    }
    match args::parse(spec, &filtered) {
        Ok(parsed) => Some(parsed),
        Err(message) => {
            eprintln!("error: {message}");
            None
        }
    }
}

fn json_mode(parsed: &Parsed) -> bool {
    parsed.flags.contains("json") || parsed.options.get("format").is_some_and(|f| f == "json")
}

/// Prints a command outcome's bytes and maps its success flag to the
/// exit code.
fn report(outcome: &CommandOutcome) -> ExitCode {
    print!("{}", outcome.stdout);
    eprint!("{}", outcome.stderr);
    if outcome.success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn ctx() -> CommandContext {
    CommandContext::from_cwd()
}

fn cmd_create(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::CREATE_SPEC, helptext::CREATE) else {
        return ExitCode::FAILURE;
    };
    let input = CreateInput {
        title:       parsed.options.get("title").cloned(),
        kind:        parsed.options.get("type").cloned(),
        priority:    parsed.options.get("priority").cloned(),
        description: parsed.options.get("description").cloned(),
        labels:      parsed.options.get("labels").cloned(),
        assignee:    parsed.options.get("assignee").cloned(),
        json:        json_mode(&parsed),
    };
    report(&commands::create(&ctx(), &input))
}

fn cmd_show(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::SHOW_SPEC, helptext::SHOW) else {
        return ExitCode::FAILURE;
    };
    let input = commands::ShowInput {
        ids:       parsed.positionals.clone(),
        format:    parsed.options.get("format").cloned(),
        json:      json_mode(&parsed),
        json_flag: parsed.flags.contains("json"),
    };
    report(&commands::show(&ctx(), &input))
}

/// Builds the shared list/ready/search input from a parsed argv.
fn query_input(parsed: &Parsed, command: QueryCommand, respect_schedule: bool) -> QueryInput {
    QueryInput {
        command: Some(command),
        query: parsed.positionals.first().cloned(),
        status: parsed.options.get("status").cloned(),
        kind: parsed.options.get("type").cloned(),
        assignee: parsed.options.get("assignee").cloned(),
        all: parsed.flags.contains("all"),
        label: parsed.options.get("label").cloned(),
        label_any: parsed.options.get("label-any").cloned(),
        unlabeled: parsed.flags.contains("unlabeled"),
        priority: parsed.options.get("priority").cloned(),
        priority_max: parsed.options.get("priority-max").cloned(),
        limit: parsed.options.get("limit").cloned(),
        sort: parsed.options.get("sort").cloned(),
        format: parsed.options.get("format").cloned(),
        json: json_mode(parsed),
        respect_schedule,
    }
}

fn cmd_list(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LIST_SPEC, helptext::LIST) else {
        return ExitCode::FAILURE;
    };
    report(&commands::list(
        &ctx(),
        &query_input(&parsed, QueryCommand::List, false),
    ))
}

fn cmd_ready(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::READY_SPEC, helptext::READY) else {
        return ExitCode::FAILURE;
    };
    let respect_schedule = parsed.flags.contains("respect-schedule");
    report(&commands::ready(
        &ctx(),
        &query_input(&parsed, QueryCommand::Ready, respect_schedule),
    ))
}

fn cmd_search(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::SEARCH_SPEC, helptext::SEARCH) else {
        return ExitCode::FAILURE;
    };
    report(&commands::search(
        &ctx(),
        &query_input(&parsed, QueryCommand::Search, false),
    ))
}

fn cmd_update(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::UPDATE_SPEC, helptext::UPDATE) else {
        return ExitCode::FAILURE;
    };
    let input = UpdateInput {
        id:               parsed.positionals.first().cloned(),
        status:           parsed.options.get("status").cloned(),
        title:            parsed.options.get("title").cloned(),
        assignee:         parsed.options.get("assignee").cloned(),
        description:      parsed.options.get("description").cloned(),
        kind:             parsed.options.get("type").cloned(),
        priority:         parsed.options.get("priority").cloned(),
        add_label:        parsed.options.get("add-label").cloned(),
        remove_label:     parsed.options.get("remove-label").cloned(),
        set_labels:       parsed.options.get("set-labels").cloned(),
        extensions:       parsed.options.get("extensions").cloned(),
        clear_extensions: parsed.flags.contains("clear-extensions"),
        json:             json_mode(&parsed),
    };
    report(&commands::update(&ctx(), &input))
}

fn cmd_close(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::CLOSE_SPEC, helptext::CLOSE) else {
        return ExitCode::FAILURE;
    };
    let input = CloseInput {
        ids:    parsed.positionals.clone(),
        reason: parsed.options.get("reason").cloned(),
        json:   json_mode(&parsed),
    };
    report(&commands::close(&ctx(), &input))
}

fn cmd_dep(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("add") => cmd_dep_add(&args[1..]),
        Some("remove") => cmd_dep_remove(&args[1..]),
        Some("list") => cmd_dep_list(&args[1..]),
        Some("-h" | "--help") | None => {
            println!("{}", helptext::DEP);
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!(
                "error: dep subcommand '{other}' is not implemented yet \
                 (this build implements `dep add`, `dep remove`, `dep list`)"
            );
            ExitCode::FAILURE
        }
    }
}

fn cmd_dep_add(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEP_ADD_SPEC, helptext::DEP_ADD) else {
        return ExitCode::FAILURE;
    };
    let input = DepAddInput {
        ids:  parsed.positionals.clone(),
        json: json_mode(&parsed),
    };
    report(&commands::dep_add(&ctx(), &input))
}

fn cmd_dep_remove(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEP_REMOVE_SPEC, helptext::DEP_REMOVE) else {
        return ExitCode::FAILURE;
    };
    let input = DepRemoveInput {
        ids:  parsed.positionals.clone(),
        json: json_mode(&parsed),
    };
    report(&commands::dep_remove(&ctx(), &input))
}

fn cmd_dep_list(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEP_LIST_SPEC, helptext::DEP_LIST) else {
        return ExitCode::FAILURE;
    };
    let input = DepListInput {
        id:   parsed.positionals.first().cloned(),
        json: json_mode(&parsed),
    };
    report(&commands::dep_list(&ctx(), &input))
}

fn cmd_blocked(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::BLOCKED_SPEC, helptext::BLOCKED) else {
        return ExitCode::FAILURE;
    };
    let input = BlockedInput {
        format: parsed.options.get("format").cloned(),
        json:   json_mode(&parsed),
    };
    report(&commands::blocked(&ctx(), &input))
}

fn cmd_block(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::BLOCK_SPEC, helptext::BLOCK) else {
        return ExitCode::FAILURE;
    };
    let input = BlockInput {
        id:   parsed.positionals.first().cloned(),
        by:   parsed.options.get("by").cloned(),
        json: json_mode(&parsed),
    };
    report(&commands::block(&ctx(), &input))
}

fn cmd_unblock(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::UNBLOCK_SPEC, helptext::UNBLOCK) else {
        return ExitCode::FAILURE;
    };
    let input = UnblockInput {
        id:   parsed.positionals.first().cloned(),
        from: parsed.options.get("from").cloned(),
        all:  parsed.flags.contains("all"),
        json: json_mode(&parsed),
    };
    report(&commands::unblock(&ctx(), &input))
}

fn cmd_label(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("add") => cmd_label_add(&args[1..]),
        Some("remove") => cmd_label_remove(&args[1..]),
        Some("list") => cmd_label_list(&args[1..]),
        Some("list-all") => cmd_label_list_all(&args[1..]),
        Some("-h" | "--help") => {
            println!("{}", helptext::LABEL);
            ExitCode::SUCCESS
        }
        // A bare `sd label` prints its usage on stderr, exit 1.
        None => {
            eprintln!("{}", helptext::LABEL);
            ExitCode::FAILURE
        }
        Some(other) => {
            eprintln!("error: unknown command '{other}' for 'label'");
            ExitCode::FAILURE
        }
    }
}

fn cmd_label_add(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_ADD_SPEC, helptext::LABEL_ADD) else {
        return ExitCode::FAILURE;
    };
    let input = LabelAddInput {
        id:     parsed.positionals.first().cloned(),
        labels: parsed.positionals[1..].to_vec(),
        json:   json_mode(&parsed),
    };
    report(&commands::label_add(&ctx(), &input))
}

fn cmd_label_remove(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_REMOVE_SPEC, helptext::LABEL_REMOVE) else {
        return ExitCode::FAILURE;
    };
    let input = LabelRemoveInput {
        id:     parsed.positionals.first().cloned(),
        labels: parsed.positionals[1..].to_vec(),
        json:   json_mode(&parsed),
    };
    report(&commands::label_remove(&ctx(), &input))
}

fn cmd_label_list(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_LIST_SPEC, helptext::LABEL_LIST) else {
        return ExitCode::FAILURE;
    };
    let input = LabelListInput {
        id:   parsed.positionals.first().cloned(),
        json: json_mode(&parsed),
    };
    report(&commands::label_list(&ctx(), &input))
}

fn cmd_label_list_all(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::LABEL_LIST_ALL_SPEC, helptext::LABEL_LIST_ALL)
    else {
        return ExitCode::FAILURE;
    };
    let input = LabelListAllInput {
        json: json_mode(&parsed),
    };
    report(&commands::label_list_all(&ctx(), &input))
}

fn cmd_stats(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::STATS_SPEC, helptext::STATS) else {
        return ExitCode::FAILURE;
    };
    let input = StatsInput {
        json: json_mode(&parsed),
    };
    report(&commands::stats(&ctx(), &input))
}

fn cmd_doctor(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DOCTOR_SPEC, helptext::DOCTOR) else {
        return ExitCode::FAILURE;
    };
    let input = DoctorInput {
        fix:           parsed.flags.contains("fix"),
        repair_report: parsed.flags.contains("repair-report"),
        json:          json_mode(&parsed),
    };
    report(&commands::doctor(&ctx(), &input))
}

fn cmd_prime(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::PRIME_SPEC, helptext::PRIME) else {
        return ExitCode::FAILURE;
    };
    let input = PrimeInput {
        compact: parsed.flags.contains("compact"),
        json:    json_mode(&parsed),
    };
    report(&commands::prime(&input))
}

fn cmd_dedupe(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::DEDUPE_SPEC, helptext::DEDUPE) else {
        return ExitCode::FAILURE;
    };
    let input = DedupeInput {
        write: parsed.flags.contains("write"),
        json:  json_mode(&parsed),
    };
    report(&commands::dedupe(&ctx(), &input))
}

fn cmd_sync(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::SYNC_SPEC, helptext::SYNC) else {
        return ExitCode::FAILURE;
    };
    let input = SyncInput {
        status:  parsed.flags.contains("status"),
        dry_run: parsed.flags.contains("dry-run"),
        json:    json_mode(&parsed),
    };
    report(&commands::sync(&ctx(), &input))
}

// -- init + config group ----------------------------------------------------

fn cmd_init(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::INIT_SPEC, helptext::INIT) else {
        return ExitCode::FAILURE;
    };
    let input = InitInput {
        json: json_mode(&parsed),
    };
    report(&commands::init(&input))
}

fn cmd_onboard(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::ONBOARD_SPEC, helptext::ONBOARD) else {
        return ExitCode::FAILURE;
    };
    let input = OnboardInput {
        check:       parsed.flags.contains("check"),
        stdout_only: parsed.flags.contains("stdout"),
        json:        json_mode(&parsed),
    };
    report(&commands::onboard(&input))
}

#[cfg(feature = "upgrade")]
fn cmd_upgrade(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::UPGRADE_SPEC, helptext::UPGRADE) else {
        return ExitCode::FAILURE;
    };
    let input = UpgradeInput {
        check: parsed.flags.contains("check"),
        json:  json_mode(&parsed),
    };
    report(&commands::upgrade(&input))
}

/// The upgrade arm when compiled without the `upgrade` feature: the
/// offline-pure core still answers honestly.
#[cfg(not(feature = "upgrade"))]
fn not_implemented_no_feature(command: &str) -> ExitCode {
    eprintln!(
        "error: command '{command}' is not implemented in this build \
         (compiled without the `upgrade` feature) — run 'seeds --help' for \
         the implemented commands"
    );
    ExitCode::FAILURE
}

fn cmd_completions(args: &[String]) -> ExitCode {
    let Some(parsed) = parsed_or_help(args, args::COMPLETIONS_SPEC, helptext::COMPLETIONS) else {
        return ExitCode::FAILURE;
    };
    let Some(shell) = parsed.positionals.first().cloned() else {
        eprintln!("error: missing required argument 'shell'");
        return ExitCode::FAILURE;
    };
    report(&commands::completions(&CompletionsInput { shell }))
}

fn cmd_config(args: &[String]) -> ExitCode {
    let Some(sub) = args.first().cloned() else {
        eprintln!("{}", helptext::CONFIG);
        return ExitCode::FAILURE;
    };
    if sub == "-h" || sub == "--help" {
        println!("{}", helptext::CONFIG);
        return ExitCode::SUCCESS;
    }
    let rest = &args[1..];
    if sub == "help" {
        let text = match rest.first().map(String::as_str) {
            Some("schema") => helptext::CONFIG_SCHEMA,
            Some("show") => helptext::CONFIG_SHOW,
            Some("set") => helptext::CONFIG_SET,
            Some("unset") => helptext::CONFIG_UNSET,
            _ => helptext::CONFIG,
        };
        println!("{text}");
        return ExitCode::SUCCESS;
    }
    let (input, json) = match sub.as_str() {
        "schema" => {
            let Some(parsed) =
                parsed_or_help(rest, args::CONFIG_SCHEMA_SPEC, helptext::CONFIG_SCHEMA)
            else {
                return ExitCode::FAILURE;
            };
            (ConfigSub::Schema, json_mode(&parsed))
        }
        "show" => {
            let Some(parsed) = parsed_or_help(rest, args::CONFIG_SHOW_SPEC, helptext::CONFIG_SHOW)
            else {
                return ExitCode::FAILURE;
            };
            (
                ConfigSub::Show {
                    path: parsed.options.get("path").cloned(),
                },
                json_mode(&parsed),
            )
        }
        "set" => {
            let Some(parsed) = parsed_or_help(rest, args::CONFIG_SET_SPEC, helptext::CONFIG_SET)
            else {
                return ExitCode::FAILURE;
            };
            match (parsed.positionals.first(), parsed.positionals.get(1)) {
                (Some(path), Some(value)) => (
                    ConfigSub::Set {
                        path:  path.clone(),
                        value: value.clone(),
                    },
                    json_mode(&parsed),
                ),
                (None, _) => return config_missing_arg("<path>"),
                (Some(_), None) => return config_missing_arg("<value>"),
            }
        }
        "unset" => {
            let Some(parsed) =
                parsed_or_help(rest, args::CONFIG_UNSET_SPEC, helptext::CONFIG_UNSET)
            else {
                return ExitCode::FAILURE;
            };
            let Some(path) = parsed.positionals.first().cloned() else {
                return config_missing_arg("<path>");
            };
            (ConfigSub::Unset { path }, json_mode(&parsed))
        }
        other => {
            eprintln!("{}", helptext::CONFIG);
            eprintln!("error: unknown command '{other}'");
            return ExitCode::FAILURE;
        }
    };
    report(&commands::config(&ctx(), &ConfigInput { sub: input, json }))
}

/// The config group's error for a missing positional argument (sd's
/// commander wording).
fn config_missing_arg(name: &str) -> ExitCode {
    eprintln!("error: missing required argument '{name}'");
    ExitCode::FAILURE
}

// -- plan group -------------------------------------------------------------

use seeds::commands::{PlanInput, PlanSub};

/// The plan group's error for a missing required argument (sd's
/// commander wording).
fn missing_arg(name: &str) -> ExitCode {
    eprintln!("error: missing required argument '{name}'");
    ExitCode::FAILURE
}

/// sd's required-option error wording.
fn missing_required(long: &str, meta: &str) -> ExitCode {
    eprintln!("error: required option '--{long} {meta}' not specified");
    ExitCode::FAILURE
}

/// The captured `--section` pair plus the remaining argv.
type SectionCapture = (Option<(String, String)>, Vec<String>);

/// Captures commander's variadic `--section <name-and-text...>`: the
/// values following `--section` up to the next `--flag`.
fn capture_section(args: &[String]) -> Result<SectionCapture, String> {
    let Some(index) = args.iter().position(|arg| arg == "--section") else {
        return Ok((None, args.to_vec()));
    };
    let mut rest: Vec<String> = args[..index].to_vec();
    let mut captured: Vec<String> = Vec::new();
    let mut cursor = index + 1;
    while let Some(arg) = args.get(cursor) {
        if arg.starts_with("--") {
            break;
        }
        captured.push(arg.clone());
        cursor += 1;
    }
    if captured.len() < 2 {
        return Err(
            "--section requires two arguments: --section <name> <text> (quote the text)."
                .to_owned(),
        );
    }
    if captured.len() > 2 {
        return Err(
            "--section received more than two arguments. Quote the text: --section <name> \
             \"<text>\"."
                .to_owned(),
        );
    }
    let section = (captured[0].clone(), captured[1].clone());
    rest.extend(args[cursor..].iter().cloned());
    Ok((Some(section), rest))
}

fn cmd_plan(args: &[String]) -> ExitCode {
    let Some(sub) = args.first().cloned() else {
        println!("{}", helptext::PLAN);
        return ExitCode::FAILURE;
    };
    if sub == "-h" || sub == "--help" {
        println!("{}", helptext::PLAN);
        return ExitCode::SUCCESS;
    }
    let rest = &args[1..];
    let (input, json) = match sub.as_str() {
        "templates" => {
            let Some(parsed) =
                parsed_or_help(rest, args::PLAN_TEMPLATES_SPEC, helptext::PLAN_TEMPLATES)
            else {
                return ExitCode::FAILURE;
            };
            (PlanSub::Templates, json_mode(&parsed))
        }
        "prompt" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_PROMPT_SPEC, helptext::PLAN_PROMPT)
            else {
                return ExitCode::FAILURE;
            };
            let Some(seed_id) = parsed.positionals.first().cloned() else {
                return missing_arg("seed-id");
            };
            (
                PlanSub::Prompt {
                    seed_id,
                    template: parsed.options.get("template").cloned(),
                    domain: parsed.options.get("domain").cloned(),
                },
                json_mode(&parsed),
            )
        }
        "submit" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_SUBMIT_SPEC, helptext::PLAN_SUBMIT)
            else {
                return ExitCode::FAILURE;
            };
            let Some(seed_id) = parsed.positionals.first().cloned() else {
                return missing_arg("seed-id");
            };
            let Some(plan_file) = parsed.options.get("plan").cloned() else {
                return missing_required("plan", "<file>");
            };
            let plan_stdin = if plan_file == "-" {
                use std::io::Read as _;
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .expect("stdin is readable");
                Some(buffer)
            } else {
                None
            };
            (
                PlanSub::Submit {
                    seed_id,
                    plan_file,
                    plan_stdin,
                    overwrite: parsed.flags.contains("overwrite"),
                    record_decision: parsed.flags.contains("record-decision"),
                    domain: parsed.options.get("domain").cloned(),
                    name: parsed.options.get("name").cloned(),
                },
                json_mode(&parsed),
            )
        }
        "show" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_ID_SPEC, helptext::PLAN_SHOW) else {
                return ExitCode::FAILURE;
            };
            let Some(id) = parsed.positionals.first().cloned() else {
                return missing_arg("id");
            };
            (PlanSub::Show { id }, json_mode(&parsed))
        }
        "validate" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_ID_SPEC, helptext::PLAN_VALIDATE)
            else {
                return ExitCode::FAILURE;
            };
            let Some(id) = parsed.positionals.first().cloned() else {
                return missing_arg("id");
            };
            (PlanSub::Validate { id }, json_mode(&parsed))
        }
        "outcome" => {
            let Some(parsed) =
                parsed_or_help(rest, args::PLAN_OUTCOME_SPEC, helptext::PLAN_OUTCOME)
            else {
                return ExitCode::FAILURE;
            };
            let Some(id) = parsed.positionals.first().cloned() else {
                return missing_arg("id");
            };
            let Some(result) = parsed.options.get("result").cloned() else {
                return missing_required("result", "<value>");
            };
            (
                PlanSub::Outcome {
                    id,
                    result,
                    note: parsed.options.get("note").cloned(),
                },
                json_mode(&parsed),
            )
        }
        "review" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_REVIEW_SPEC, helptext::PLAN_REVIEW)
            else {
                return ExitCode::FAILURE;
            };
            let Some(id) = parsed.positionals.first().cloned() else {
                return missing_arg("id");
            };
            let Some(by) = parsed.options.get("by").cloned() else {
                return missing_required("by", "<name>");
            };
            (PlanSub::Review { id, by }, json_mode(&parsed))
        }
        "edit" => {
            let (section, filtered) = match capture_section(rest) {
                Ok(captured) => captured,
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::FAILURE;
                }
            };
            let Some(parsed) = parsed_or_help(&filtered, args::PLAN_EDIT_SPEC, helptext::PLAN_EDIT)
            else {
                return ExitCode::FAILURE;
            };
            let Some(id) = parsed.positionals.first().cloned() else {
                return missing_arg("id");
            };
            (
                PlanSub::Edit {
                    id,
                    name: parsed.options.get("name").cloned(),
                    section,
                    step: parsed.options.get("step").cloned(),
                    title: parsed.options.get("title").cloned(),
                    priority: parsed.options.get("priority").cloned(),
                    kind: parsed.options.get("type").cloned(),
                },
                json_mode(&parsed),
            )
        }
        "create" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_CREATE_SPEC, helptext::PLAN_CREATE)
            else {
                return ExitCode::FAILURE;
            };
            let Some(seed_id) = parsed.positionals.first().cloned() else {
                return missing_arg("seed-id");
            };
            (
                PlanSub::Create {
                    seed_id,
                    name: parsed.options.get("name").cloned(),
                    template: parsed.options.get("template").cloned(),
                },
                json_mode(&parsed),
            )
        }
        "adopt" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_ADOPT_SPEC, helptext::PLAN_ADOPT)
            else {
                return ExitCode::FAILURE;
            };
            let Some(plan_id) = parsed.positionals.first().cloned() else {
                return missing_arg("plan-id");
            };
            let seed_ids = parsed.positionals[1..].to_vec();
            if seed_ids.is_empty() {
                return missing_arg("seed-ids");
            }
            (
                PlanSub::Adopt {
                    plan_id,
                    seed_ids,
                    step: parsed.options.get("step").cloned(),
                    at: parsed.options.get("at").cloned(),
                    before: parsed.options.get("before").cloned(),
                    after: parsed.options.get("after").cloned(),
                },
                json_mode(&parsed),
            )
        }
        "reorder" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_ID_SPEC, helptext::PLAN_REORDER)
            else {
                return ExitCode::FAILURE;
            };
            let Some(plan_id) = parsed.positionals.first().cloned() else {
                return missing_arg("plan-id");
            };
            let seed_ids = parsed.positionals[1..].to_vec();
            if seed_ids.is_empty() {
                return missing_arg("seed-ids");
            }
            (PlanSub::Reorder { plan_id, seed_ids }, json_mode(&parsed))
        }
        "release" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_ID_SPEC, helptext::PLAN_RELEASE)
            else {
                return ExitCode::FAILURE;
            };
            let Some(plan_id) = parsed.positionals.first().cloned() else {
                return missing_arg("plan-id");
            };
            let seed_ids = parsed.positionals[1..].to_vec();
            if seed_ids.is_empty() {
                return missing_arg("seed-ids");
            }
            (PlanSub::Release { plan_id, seed_ids }, json_mode(&parsed))
        }
        "list" => {
            let Some(parsed) = parsed_or_help(rest, args::PLAN_LIST_SPEC, helptext::PLAN_LIST)
            else {
                return ExitCode::FAILURE;
            };
            (
                PlanSub::List {
                    seed:     parsed.options.get("seed").cloned(),
                    status:   parsed.options.get("status").cloned(),
                    outcome:  parsed.options.get("outcome").cloned(),
                    template: parsed.options.get("template").cloned(),
                },
                json_mode(&parsed),
            )
        }
        other => {
            eprintln!("error: unknown command 'plan {other}'");
            return ExitCode::FAILURE;
        }
    };
    report(&commands::plan(&ctx(), &PlanInput { sub: input, json }))
}
