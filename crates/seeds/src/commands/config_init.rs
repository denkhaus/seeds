//! `init` and `config` command semantics (sd 0.5.15-compatible):
//! `.seeds/` bootstrap and `config.yaml` read/write.
//!
//! The config subcommands operate on the config document as an ordered
//! YAML value (not the typed [`crate::config::Config`]) so unknown keys
//! and their file order survive every read-modify-write — the format's
//! additive-fields rule. YAML rendering mirrors the reference's dumper
//! (number-like strings double-quoted, block sequences at parent
//! indent), keeping `config show` text and rewritten files byte-stable.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use serde_yaml::{Mapping, Value as Yaml};

use super::{CommandContext, CommandError, CommandOutcome, envelope_pretty, push_line};

/// The reference's JSON Schema for `.seeds/config.yaml`, emitted
/// verbatim by `config schema`.
const CONFIG_SCHEMA: &str = include_str!("config_schema.json");

/// The top-level config keys the schema models; `set` creates paths
/// only under these (the reference's `additionalProperties: false`).
const KNOWN_KEYS: &[&str] = &["project", "version", "max_plan_depth", "plan_templates"];

// ---------------------------------------------------------------------------
// init
// ---------------------------------------------------------------------------

/// `seeds init`: bootstrap `.seeds/` in the current directory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InitInput {
    /// Whether success output is the JSON envelope.
    pub json: bool,
}

/// Bootstraps `.seeds/` in the current directory (config.yaml, empty
/// issues/plans/templates JSONL, `.gitignore`), the reference's layout.
///
/// A pre-existing `.seeds/` directory is reported as already
/// initialized and left untouched.
#[must_use = "the outcome only takes effect when the caller prints it"]
pub fn init(input: &InitInput) -> CommandOutcome {
    let Ok(cwd) = std::env::current_dir() else {
        return CommandError::new("init", "cannot determine the current directory", input.json)
            .into_outcome();
    };
    let dir = cwd.join(".seeds");
    if input.json {
        let stdout = envelope_pretty("init", &[("dir", Value::from(dir.display().to_string()))]);
        let mut text = String::new();
        push_line(&mut text, &stdout);
        return CommandOutcome {
            success: true,
            stdout:  text,
            stderr:  String::new(),
        };
    }
    if dir.is_dir() {
        let mut text = String::new();
        push_line(
            &mut text,
            &format!("✓ Already initialized: {}", dir.display()),
        );
        return CommandOutcome {
            success: true,
            stdout:  text,
            stderr:  String::new(),
        };
    }
    if let Err(message) = write_init_files(&dir, &cwd) {
        return CommandError::new("init", message, input.json).into_outcome();
    }
    let mut text = String::new();
    push_line(
        &mut text,
        &format!("✓ Initialized .seeds/ in {}", cwd.display()),
    );
    CommandOutcome {
        success: true,
        stdout:  text,
        stderr:  String::new(),
    }
}

/// Creates the `.seeds/` file set: the config template (project named
/// after the current directory) and the three empty JSONL stores plus
/// the lock-file gitignore.
fn write_init_files(dir: &Path, cwd: &Path) -> Result<(), String> {
    let project = cwd.file_name().map_or_else(
        || "seeds".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    let files = [
        (
            "config.yaml",
            format!("project: \"{project}\"\nversion: \"1\"\nmax_plan_depth: 3\n"),
        ),
        ("issues.jsonl", String::new()),
        ("plans.jsonl", String::new()),
        ("templates.jsonl", String::new()),
        (".gitignore", "*.lock\n".to_owned()),
    ];
    for (name, body) in files {
        let path = dir.join(name);
        fs::write(&path, body)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

/// One `seeds config` subcommand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigSub {
    /// Emit the config JSON Schema (`--json` compacts it).
    Schema,
    /// Print the config, or the value at a dot-`path`.
    Show {
        /// Dot-path to read (e.g. `plan_templates.feature.sections`).
        path: Option<String>,
    },
    /// Set the value at a dot-`path`; `value` is YAML-parsed.
    Set {
        /// Dot-path to write.
        path:  String,
        /// The raw value text, parsed as YAML.
        value: String,
    },
    /// Remove the value at a dot-`path`.
    Unset {
        /// Dot-path to remove.
        path: String,
    },
}

/// `seeds config`: read, write, and inspect `.seeds/config.yaml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigInput {
    /// Which subcommand to run.
    pub sub:  ConfigSub,
    /// Whether output is the JSON envelope form.
    pub json: bool,
}

/// Runs one config subcommand against the context's `.seeds/config.yaml`.
#[must_use = "the outcome only takes effect when the caller prints it"]
pub fn config(ctx: &CommandContext, input: &ConfigInput) -> CommandOutcome {
    match &input.sub {
        ConfigSub::Schema => schema(input.json),
        ConfigSub::Show { path } => show(ctx, path.as_deref(), input.json),
        ConfigSub::Set { path, value } => set(ctx, path, value, input.json),
        ConfigSub::Unset { path } => unset(ctx, path, input.json),
    }
}

/// The loaded config document plus the file it came from.
struct ConfigDoc {
    path: PathBuf,
    root: Mapping,
}

/// Reads and parses the context's config.yaml.
fn load_doc(ctx: &CommandContext, json: bool) -> Result<ConfigDoc, CommandOutcome> {
    let root = ctx
        .resolve()
        .map_err(|message| CommandError::new("config", message, json).into_outcome())?
        .to_owned();
    let path = root.join("config.yaml");
    let text = fs::read_to_string(&path).map_err(|error| {
        CommandError::new(
            "config",
            format!("cannot read {}: {error}", path.display()),
            json,
        )
        .into_outcome()
    })?;
    let parsed: Yaml = serde_yaml::from_str(&text).map_err(|error| {
        CommandError::new(
            "config",
            format!("invalid {}: {error}", path.display()),
            json,
        )
        .into_outcome()
    })?;
    let Yaml::Mapping(root) = parsed else {
        return Err(CommandError::new(
            "config",
            format!("{} must contain a YAML mapping", path.display()),
            json,
        )
        .into_outcome());
    };
    Ok(ConfigDoc { path, root })
}

/// Writes the config document back, preserving every key and order.
fn write_doc(doc: &ConfigDoc) -> Result<(), String> {
    let mut body = dump_mapping(&doc.root, 0);
    if !body.ends_with('\n') {
        body.push('\n');
    }
    fs::write(&doc.path, body)
        .map_err(|error| format!("cannot write {}: {error}", doc.path.display()))
}

/// `config schema`: the reference's schema document, pretty by default
/// and compact under `--json`.
fn schema(json: bool) -> CommandOutcome {
    let mut stdout = String::new();
    if json {
        let compact: Value =
            serde_json::from_str(CONFIG_SCHEMA).expect("the embedded config schema is valid JSON");
        push_line(
            &mut stdout,
            &serde_json::to_string(&compact).expect("the schema always serializes"),
        );
    } else {
        stdout.push_str(CONFIG_SCHEMA);
    }
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

/// `config show`: the whole config, or the value at a dot-path.
fn show(ctx: &CommandContext, path: Option<&str>, json: bool) -> CommandOutcome {
    let doc = match load_doc(ctx, json) {
        Ok(doc) => doc,
        Err(outcome) => return outcome,
    };
    let Some(path) = path else {
        if json {
            return ok_json("config show", &[(
                "config",
                yaml_to_json(&Yaml::Mapping(doc.root)),
            )]);
        }
        let mut stdout = String::new();
        stdout.push_str(&dump_mapping(&doc.root, 0));
        return CommandOutcome {
            success: true,
            stdout,
            stderr: String::new(),
        };
    };
    let segments = split_path(path);
    match get_path(&Yaml::Mapping(doc.root), &segments) {
        Some(value) => {
            if json {
                return ok_json("config show", &[
                    ("path", Value::from(path)),
                    ("value", yaml_to_json(value)),
                ]);
            }
            let mut stdout = String::new();
            push_line(&mut stdout, &render_value(value));
            CommandOutcome {
                success: true,
                stdout,
                stderr: String::new(),
            }
        }
        None => CommandError::new("config", format!("Path not found: {path}"), json).into_outcome(),
    }
}

/// `config set`: YAML-parse the value, write it at the dot-path,
/// validate, and persist (unknown keys preserved).
fn set(ctx: &CommandContext, path: &str, value: &str, json: bool) -> CommandOutcome {
    let mut doc = match load_doc(ctx, json) {
        Ok(doc) => doc,
        Err(outcome) => return outcome,
    };
    let parsed: Yaml = match serde_yaml::from_str(value) {
        Ok(parsed) => parsed,
        Err(error) => {
            return CommandError::new("config", format!("{error}"), json).into_outcome();
        }
    };
    let segments = split_path(path);
    if let Some(first) = segments.first()
        && !KNOWN_KEYS.contains(&first.as_str())
    {
        let message = format!(
            "Config validation failed:\n  (root): additionalProperties \
             (additionalProperty={first})"
        );
        return CommandError::new("config", message, json).into_outcome();
    }
    set_path(&mut doc.root, &segments, parsed);
    if let Some(message) = validation_error(&doc.root) {
        return CommandError::new("config", message, json).into_outcome();
    }
    let written = get_path(&Yaml::Mapping(doc.root.clone()), &segments)
        .cloned()
        .unwrap_or(Yaml::Null);
    if let Err(message) = write_doc(&doc) {
        return CommandError::new("config", message, json).into_outcome();
    }
    if json {
        return ok_json("config set", &[
            ("path", Value::from(path)),
            ("value", yaml_to_json(&written)),
        ]);
    }
    let mut stdout = String::new();
    push_line(
        &mut stdout,
        &format!("✓ Set {path} = {}", render_value(&written)),
    );
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

/// `config unset`: remove the dot-path, validate, persist. A missing
/// path is a successful no-op (the reference's behavior).
fn unset(ctx: &CommandContext, path: &str, json: bool) -> CommandOutcome {
    let mut doc = match load_doc(ctx, json) {
        Ok(doc) => doc,
        Err(outcome) => return outcome,
    };
    let segments = split_path(path);
    if get_path(&Yaml::Mapping(doc.root.clone()), &segments).is_none() {
        if json {
            return ok_json("config unset", &[
                ("path", Value::from(path)),
                ("removed", Value::Bool(false)),
            ]);
        }
        let mut stdout = String::new();
        push_line(&mut stdout, &format!("No such path: {path}"));
        return CommandOutcome {
            success: true,
            stdout,
            stderr: String::new(),
        };
    }
    let mut updated = doc.root.clone();
    remove_path(&mut updated, &segments);
    if let Some(message) = validation_error(&updated) {
        return CommandError::new("config", message, json).into_outcome();
    }
    doc.root = updated;
    if let Err(message) = write_doc(&doc) {
        return CommandError::new("config", message, json).into_outcome();
    }
    if json {
        return ok_json("config unset", &[
            ("path", Value::from(path)),
            ("removed", Value::Bool(true)),
        ]);
    }
    let mut stdout = String::new();
    push_line(&mut stdout, &format!("✓ Unset {path}"));
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

/// A successful JSON envelope with extra fields, newline-terminated.
fn ok_json(command: &str, extra: &[(&str, Value)]) -> CommandOutcome {
    let mut stdout = String::new();
    push_line(&mut stdout, &envelope_pretty(command, extra));
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

// ---------------------------------------------------------------------------
// validation (the schema's top-level rules)
// ---------------------------------------------------------------------------

/// Validates the top-level config keys, returning the reference's
/// `Config validation failed:` message on the first violation class.
///
/// Unknown keys are NOT rejected here: this crate preserves them on
/// every write (the additive-fields rule; see the README's DEVIATIONS).
fn validation_error(root: &Mapping) -> Option<String> {
    let mut errors = Vec::new();
    for key in ["project", "version"] {
        match root.get(Yaml::String(key.to_owned())) {
            None => errors.push(format!("{key}: add a '{key}' field")),
            Some(Yaml::String(_)) => {}
            Some(_) => errors.push(format!("{key}: '{key}' must be a string")),
        }
    }
    match root.get(Yaml::String("max_plan_depth".to_owned())) {
        None => {}
        Some(Yaml::Number(number)) if number.is_i64() => {
            if number.as_i64().is_some_and(|depth| depth < 1) {
                errors.push(
                    "max_plan_depth: 'max_plan_depth': minimum (comparison=>=, limit=1)".to_owned(),
                );
            }
        }
        Some(_) => errors.push("max_plan_depth: 'max_plan_depth' must be a integer".to_owned()),
    }
    if let Some(templates) = root.get(Yaml::String("plan_templates".to_owned()))
        && !matches!(templates, Yaml::Mapping(_))
    {
        errors.push("plan_templates: 'plan_templates' must be a object".to_owned());
    }
    if errors.is_empty() {
        return None;
    }
    let mut message = format!("Config validation failed:\n  {}", errors[0]);
    for error in &errors[1..] {
        message.push_str("\n  ");
        message.push_str(error);
    }
    Some(message)
}

// ---------------------------------------------------------------------------
// dot-path access
// ---------------------------------------------------------------------------

/// Splits a dot-path into segments.
fn split_path(path: &str) -> Vec<String> {
    path.split('.').map(str::to_owned).collect()
}

/// Walks a dot-path through nested mappings.
fn get_path<'a>(root: &'a Yaml, segments: &[String]) -> Option<&'a Yaml> {
    let mut current = root;
    for segment in segments {
        let Yaml::Mapping(mapping) = current else {
            return None;
        };
        current = mapping.get(Yaml::String(segment.clone()))?;
    }
    Some(current)
}

/// Writes `value` at the dot-path, creating intermediate mappings.
fn set_path(root: &mut Mapping, segments: &[String], value: Yaml) {
    let Some((last, parents)) = segments.split_last() else {
        return;
    };
    let mut current: &mut Mapping = root;
    for segment in parents {
        let key = Yaml::String(segment.clone());
        if !matches!(current.get(&key), Some(Yaml::Mapping(_)) | None) {
            current.insert(key.clone(), Yaml::Mapping(Mapping::new()));
        }
        let next = current
            .entry(key)
            .or_insert_with(|| Yaml::Mapping(Mapping::new()));
        let Yaml::Mapping(mapping) = next else {
            return;
        };
        current = mapping;
    }
    current.insert(Yaml::String(last.clone()), value);
}

/// Removes the dot-path's leaf; returns whether it was present.
fn remove_path(root: &mut Mapping, segments: &[String]) -> bool {
    let Some((last, parents)) = segments.split_last() else {
        return false;
    };
    let Some(parent) = get_path_mut(root, parents) else {
        return false;
    };
    parent.remove(Yaml::String(last.clone())).is_some()
}

/// Walks a dot-path to the parent mapping of the leaf, mutably.
fn get_path_mut<'a>(root: &'a mut Mapping, segments: &[String]) -> Option<&'a mut Mapping> {
    let mut current: &mut Mapping = root;
    for segment in segments {
        let next = current.get_mut(Yaml::String(segment.clone()))?;
        let Yaml::Mapping(mapping) = next else {
            return None;
        };
        current = mapping;
    }
    Some(current)
}

// ---------------------------------------------------------------------------
// YAML rendering (the reference dumper's style)
// ---------------------------------------------------------------------------

/// Whether a value renders inline (no block structure).
fn is_scalar(value: &Yaml) -> bool {
    matches!(
        value,
        Yaml::Null | Yaml::Bool(_) | Yaml::Number(_) | Yaml::String(_)
    )
}

/// Renders a mapping in block style at `indent` (no trailing-newline
/// contract: callers append structure themselves).
fn dump_mapping(mapping: &Mapping, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut out = String::new();
    for (key, value) in mapping {
        out.push_str(&pad);
        out.push_str(&dump_scalar(key));
        out.push(':');
        append_block_value(&mut out, value, indent);
    }
    out
}

/// Renders a sequence in block style, dashes at the parent's indent.
fn dump_sequence(sequence: &[Yaml], indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut out = String::new();
    for item in sequence {
        out.push_str(&pad);
        out.push('-');
        match item {
            value if is_scalar(value) => {
                out.push(' ');
                out.push_str(&dump_scalar(value));
                out.push('\n');
            }
            Yaml::Mapping(mapping) if mapping.is_empty() => out.push_str(" {}\n"),
            Yaml::Mapping(mapping) => {
                out.push(' ');
                let inner = dump_mapping(mapping, indent + 2);
                // First key joins the dash line; the rest keep their pad.
                out.push_str(inner.trim_start_matches(' '));
            }
            Yaml::Sequence(inner) if inner.is_empty() => out.push_str(" []\n"),
            Yaml::Sequence(inner) => {
                out.push('\n');
                out.push_str(&dump_sequence(inner, indent + 2));
            }
            other => {
                out.push(' ');
                out.push_str(&dump_scalar(other));
                out.push('\n');
            }
        }
    }
    out
}

/// Appends `: value` structure after a key, reference-style: scalars
/// inline, empty collections inline, nested blocks on following lines.
fn append_block_value(out: &mut String, value: &Yaml, indent: usize) {
    match value {
        value if is_scalar(value) => {
            out.push(' ');
            out.push_str(&dump_scalar(value));
            out.push('\n');
        }
        Yaml::Mapping(mapping) if mapping.is_empty() => out.push_str(" {}\n"),
        Yaml::Mapping(mapping) => {
            out.push('\n');
            out.push_str(&dump_mapping(mapping, indent + 2));
        }
        Yaml::Sequence(sequence) if sequence.is_empty() => out.push_str(" []\n"),
        Yaml::Sequence(sequence) => {
            out.push('\n');
            out.push_str(&dump_sequence(sequence, indent));
        }
        other => {
            out.push(' ');
            out.push_str(&dump_scalar(other));
            out.push('\n');
        }
    }
}

/// Renders one scalar: plain when YAML round-trips it as a string,
/// double-quoted otherwise (the reference dumper's rule).
fn dump_scalar(value: &Yaml) -> String {
    match value {
        Yaml::Null => "null".to_owned(),
        Yaml::Bool(flag) => flag.to_string(),
        Yaml::Number(number) => number.to_string(),
        Yaml::String(text) => {
            if renders_plain(text) {
                text.clone()
            } else {
                format!("{text:?}")
            }
        }
        other => dump_inline_fallback(other),
    }
}

/// Whether `text` can be emitted unquoted and re-parsed as the same
/// string (no embedded newlines, not number/bool/null-like).
fn renders_plain(text: &str) -> bool {
    !text.is_empty()
        && !text.contains('\n')
        && serde_yaml::from_str::<Yaml>(text)
            .is_ok_and(|parsed| parsed == Yaml::String(text.to_owned()))
}

/// Non-scalar values reaching a scalar slot render as flow JSON.
fn dump_inline_fallback(value: &Yaml) -> String {
    serde_json::to_string(&yaml_to_json(value)).unwrap_or_else(|_| "null".to_owned())
}

/// Renders a path value for text output: scalars plain, collections as
/// the reference prints them (sequences compact JSON, mappings YAML).
fn render_value(value: &Yaml) -> String {
    match value {
        value if is_scalar(value) => dump_scalar(value),
        Yaml::Sequence(_) => dump_inline_fallback(value),
        Yaml::Mapping(mapping) => {
            let dumped = dump_mapping(mapping, 0);
            dumped.trim_end_matches('\n').to_owned()
        }
        other => dump_scalar(other),
    }
}

/// Converts a YAML value to its JSON projection (mappings keep order).
fn yaml_to_json(value: &Yaml) -> Value {
    match value {
        Yaml::Null => Value::Null,
        Yaml::Bool(flag) => Value::Bool(*flag),
        Yaml::Number(number) => number
            .as_i64()
            .map(Value::from)
            .or_else(|| {
                number
                    .as_f64()
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
            })
            .unwrap_or(Value::Null),
        Yaml::String(text) => Value::String(text.clone()),
        Yaml::Sequence(items) => Value::Array(items.iter().map(yaml_to_json).collect()),
        Yaml::Mapping(mapping) => Value::Object(
            mapping
                .iter()
                .map(|(key, value)| (mapping_key(key), yaml_to_json(value)))
                .collect(),
        ),
        Yaml::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

/// The JSON object key for a YAML mapping key.
fn mapping_key(key: &Yaml) -> String {
    match key {
        Yaml::String(text) => text.clone(),
        other => dump_scalar(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temp `.seeds/` store with the given config body.
    fn store(tag: &str, config: &str) -> (PathBuf, CommandContext) {
        let dir =
            std::env::temp_dir().join(format!("seeds-config-init-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let seeds = dir.join(".seeds");
        fs::create_dir_all(&seeds).expect("create temp .seeds");
        fs::write(seeds.join("config.yaml"), config).expect("write config");
        fs::write(seeds.join("issues.jsonl"), "").expect("write issues");
        (dir, CommandContext::at(seeds))
    }

    const FIXTURE: &str = "project: \"tst\"\nversion: \"1\"\nmax_plan_depth: 3\n";

    #[test]
    fn show_text_matches_reference_dump_style() {
        let (_dir, ctx) = store("show", FIXTURE);
        let outcome = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Show { path: None },
            json: false,
        });
        assert!(outcome.success);
        assert_eq!(
            outcome.stdout,
            "project: tst\nversion: \"1\"\nmax_plan_depth: 3\n"
        );
    }

    #[test]
    fn show_json_envelope_carries_the_whole_document() {
        let (_dir, ctx) = store("show-json", FIXTURE);
        let outcome = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Show { path: None },
            json: true,
        });
        let value: Value = serde_json::from_str(&outcome.stdout).expect("envelope parses");
        assert_eq!(value["command"], "config show");
        assert_eq!(value["config"]["version"], "1");
        assert_eq!(value["config"]["max_plan_depth"], 3);
    }

    #[test]
    fn show_path_renders_scalars_sequences_and_maps() {
        let (_dir, ctx) = store(
            "show-path",
            "project: tst\nversion: \"1\"\nmax_plan_depth: 3\nreviewers:\n  - a\n  - b\nweird:\n  a: 1\n",
        );
        for (path, expected) in [
            ("project", "tst\n"),
            ("max_plan_depth", "3\n"),
            ("reviewers", "[\"a\",\"b\"]\n"),
            ("weird", "a: 1\n"),
        ] {
            let outcome = config(&ctx, &ConfigInput {
                sub:  ConfigSub::Show {
                    path: Some(path.to_owned()),
                },
                json: false,
            });
            assert_eq!(outcome.stdout, expected, "path {path}");
        }
    }

    #[test]
    fn show_path_missing_fails_with_reference_message() {
        let (_dir, ctx) = store("show-missing", FIXTURE);
        let outcome = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Show {
                path: Some("nope.deep".to_owned()),
            },
            json: false,
        });
        assert!(!outcome.success);
        assert_eq!(outcome.stderr, "Error: Path not found: nope.deep\n");
    }

    #[test]
    fn set_parses_yaml_and_preserves_unknown_keys() {
        let (dir, ctx) = store(
            "set",
            "project: tst\nversion: \"1\"\nmax_plan_depth: 3\nreviewers:\n  - operator\n",
        );
        let outcome = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Set {
                path:  "max_plan_depth".to_owned(),
                value: "5".to_owned(),
            },
            json: false,
        });
        assert!(outcome.success);
        assert_eq!(outcome.stdout, "✓ Set max_plan_depth = 5\n");
        let rewritten =
            fs::read_to_string(dir.join(".seeds/config.yaml")).expect("rewritten config");
        assert_eq!(
            rewritten,
            "project: tst\nversion: \"1\"\nmax_plan_depth: 5\nreviewers:\n- operator\n"
        );
    }

    #[test]
    fn set_creates_nested_paths_under_known_keys() {
        let (dir, ctx) = store("set-nested", FIXTURE);
        let outcome = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Set {
                path:  "plan_templates.feature.description".to_owned(),
                value: "Custom".to_owned(),
            },
            json: false,
        });
        assert!(outcome.success, "stderr: {}", outcome.stderr);
        let rewritten =
            fs::read_to_string(dir.join(".seeds/config.yaml")).expect("rewritten config");
        assert!(rewritten.contains("plan_templates:\n  feature:\n    description: Custom\n"));
    }

    #[test]
    fn set_rejects_unknown_top_level_keys() {
        let (_dir, ctx) = store("set-unknown", FIXTURE);
        let outcome = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Set {
                path:  "ghost".to_owned(),
                value: "1".to_owned(),
            },
            json: true,
        });
        assert!(!outcome.success);
        let value: Value = serde_json::from_str(&outcome.stdout).expect("envelope parses");
        assert_eq!(value["command"], "config");
        assert_eq!(
            value["error"],
            "Config validation failed:\n  (root): additionalProperties (additionalProperty=ghost)"
        );
    }

    #[test]
    fn set_type_errors_mirror_reference_messages() {
        let (_dir, ctx) = store("set-type", FIXTURE);
        for (path, value, expected) in [
            ("version", "2", "'version' must be a string"),
            (
                "max_plan_depth",
                "notanint",
                "'max_plan_depth' must be a integer",
            ),
        ] {
            let outcome = config(&ctx, &ConfigInput {
                sub:  ConfigSub::Set {
                    path:  path.to_owned(),
                    value: value.to_owned(),
                },
                json: false,
            });
            assert!(!outcome.success, "{path} should fail");
            assert!(
                outcome.stderr.contains(expected),
                "{path}: {}",
                outcome.stderr
            );
        }
    }

    #[test]
    fn unset_removes_and_reports_missing_paths_as_noop() {
        let (dir, ctx) = store("unset", FIXTURE);
        let removed = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Unset {
                path: "max_plan_depth".to_owned(),
            },
            json: false,
        });
        assert!(removed.success);
        assert_eq!(removed.stdout, "✓ Unset max_plan_depth\n");
        let rewritten =
            fs::read_to_string(dir.join(".seeds/config.yaml")).expect("rewritten config");
        assert_eq!(rewritten, "project: tst\nversion: \"1\"\n");

        let missing = config(&ctx, &ConfigInput {
            sub:  ConfigSub::Unset {
                path: "ghost".to_owned(),
            },
            json: false,
        });
        assert!(missing.success);
        assert_eq!(missing.stdout, "No such path: ghost\n");
    }

    #[test]
    fn unset_required_key_fails_validation() {
        let (_dir, ctx) = store("unset-required", FIXTURE);
        for key in ["project", "version"] {
            let outcome = config(&ctx, &ConfigInput {
                sub:  ConfigSub::Unset {
                    path: key.to_owned(),
                },
                json: false,
            });
            assert!(!outcome.success, "unset {key} should fail");
            assert!(
                outcome.stderr.contains(&format!("add a '{key}' field")),
                "unset {key}: {}",
                outcome.stderr
            );
        }
    }

    #[test]
    fn schema_outputs_pretty_and_compact_forms() {
        let pretty = config(&CommandContext::from_cwd(), &ConfigInput {
            sub:  ConfigSub::Schema,
            json: false,
        });
        assert!(pretty.stdout.starts_with("{\n  \"$schema\""));
        let compact = config(&CommandContext::from_cwd(), &ConfigInput {
            sub:  ConfigSub::Schema,
            json: true,
        });
        assert_eq!(compact.stdout.matches('\n').count(), 1);
        let pretty_value: Value = serde_json::from_str(pretty.stdout.trim()).expect("pretty JSON");
        let compact_value: Value =
            serde_json::from_str(compact.stdout.trim()).expect("compact JSON");
        assert_eq!(pretty_value, compact_value);
    }

    #[test]
    fn init_bootstraps_the_reference_layout_idempotently() {
        let parent = std::env::temp_dir().join(format!("seeds-init-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&parent);
        fs::create_dir_all(&parent).expect("create parent");
        let cwd = parent.join("initcase");
        fs::create_dir_all(&cwd).expect("create cwd");

        let guard = current_dir_guard(&cwd);
        let fresh = init(&InitInput { json: false });
        assert!(fresh.success, "stderr: {}", fresh.stderr);
        assert!(fresh.stdout.starts_with("✓ Initialized .seeds/ in "));

        let config = fs::read_to_string(cwd.join(".seeds/config.yaml")).expect("config.yaml");
        assert_eq!(
            config,
            "project: \"initcase\"\nversion: \"1\"\nmax_plan_depth: 3\n"
        );
        assert_eq!(
            fs::read_to_string(cwd.join(".seeds/.gitignore")).expect("gitignore"),
            "*.lock\n"
        );
        for store in ["issues.jsonl", "plans.jsonl", "templates.jsonl"] {
            assert_eq!(
                fs::read_to_string(cwd.join(".seeds").join(store)).expect(store),
                ""
            );
        }

        let already = init(&InitInput { json: true });
        assert!(already.success);
        let value: Value = serde_json::from_str(&already.stdout).expect("envelope parses");
        assert_eq!(value["command"], "init");
        assert_eq!(value["dir"], cwd.join(".seeds").display().to_string());
        drop(guard);
        let _ = fs::remove_dir_all(&parent);
    }

    /// Runs the body with `dir` as the process cwd, restoring after.
    struct CwdGuard(PathBuf);

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

    fn current_dir_guard(dir: &Path) -> CwdGuard {
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(dir).expect("enter cwd");
        CwdGuard(previous)
    }
}
