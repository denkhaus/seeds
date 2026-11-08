//! `.seeds/config.yaml` reader/writer.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml::{Mapping, Value as Yaml};

use crate::error::Error;
use crate::model::Fields;

fn default_version() -> String {
    "1".to_owned()
}

fn default_max_plan_depth() -> u32 {
    3
}

fn default_vcs_manager() -> VcsManager {
    VcsManager::Git
}

/// Whether the key holds its default (so writes omit it, keeping
/// rewritten configs byte-stable for the reference's stricter schema).
#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if hands the field over by reference"
)]
fn is_default_vcs_manager(manager: &VcsManager) -> bool {
    *manager == VcsManager::Git
}

/// How `seeds sync` hands `.seeds/` changes to the repository's VCS
/// (`.seeds/config.yaml` key `vcs_manager`; absent = `git`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VcsManager {
    /// Plain `git add` + `git commit` (the default, unchanged behavior).
    Git,
    /// GitButler-managed workspace: sync detects and reports changes
    /// but never issues a git write (seeds-5b36).
    GitButler,
}

impl VcsManager {
    /// The config key's string form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::GitButler => "gitbutler",
        }
    }
}

/// The store configuration from `.seeds/config.yaml`. Unknown keys are
/// preserved so a read-modify-write never drops additive config.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// The project name; also the prefix of every seed id.
    pub project:        String,
    /// The store format version (sd writes the string `"1"`).
    #[serde(default = "default_version")]
    pub version:        String,
    /// The maximum plan nesting depth.
    #[serde(default = "default_max_plan_depth")]
    pub max_plan_depth: u32,
    /// How `seeds sync` commits: plain git, or report-only under
    /// GitButler (which cannot tolerate plain git writes).
    #[serde(
        default = "default_vcs_manager",
        skip_serializing_if = "is_default_vcs_manager"
    )]
    pub vcs_manager:    VcsManager,
    /// The store's surface labels for relevance-scoped pace metrics
    /// (seeds-b657): `stats --pace` renders one pace group per
    /// configured label plus an `unclassified` count, and `prime`
    /// carries the groups in its pace line. Empty = no surface
    /// grouping (stats then hints at the setup).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surfaces:       Vec<String>,
    /// Every config key this crate does not model, preserved verbatim.
    #[serde(flatten)]
    pub extra:          Fields,
}

impl Config {
    /// Parses a config from YAML text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] when the YAML is invalid or `project`
    /// is missing.
    pub fn parse(text: &str, path: &std::path::Path) -> Result<Self, Error> {
        serde_yaml::from_str(text).map_err(|source| Error::Config {
            path: path.to_owned(),
            source,
        })
    }

    /// Serializes the config to YAML text with no trailing newline:
    /// serde_yaml already terminates its document with `\n`, and the
    /// chomp hands the final newline to the writer — `Store::save`
    /// appends exactly one, so a saved `config.yaml` never ends in a
    /// blank line (seeds-d2c5).
    #[must_use]
    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(self)
            .expect("a config always serializes")
            .trim_end_matches('\n')
            .to_owned()
    }
}

// ---------------------------------------------------------------------------
// the ONE config serializer: the reference dumper (seeds-a84c)
// ---------------------------------------------------------------------------
// `init` and `config set`/`unset` render `.seeds/config.yaml`
// through this dumper (the reference's style: number-like strings
// double-quoted — `version: "1"` —, block sequences at parent
// indent). Store saves render through the same dumper via
// [`Config::to_reference_yaml`], so no two write paths can
// disagree about the file's bytes again.

/// Whether a value renders inline (no block structure).
pub(crate) fn is_scalar(value: &Yaml) -> bool {
    matches!(
        value,
        Yaml::Null | Yaml::Bool(_) | Yaml::Number(_) | Yaml::String(_)
    )
}

/// Renders a mapping in block style at `indent` (no trailing-newline
/// contract: callers append structure themselves).
pub(crate) fn dump_mapping(mapping: &Mapping, indent: usize) -> String {
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
pub(crate) fn dump_scalar(value: &Yaml) -> String {
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
pub(crate) fn dump_inline_fallback(value: &Yaml) -> String {
    serde_json::to_string(&yaml_to_json(value)).unwrap_or_else(|_| "null".to_owned())
}

/// Converts a YAML value to its JSON projection (mappings keep order).
pub(crate) fn yaml_to_json(value: &Yaml) -> Value {
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

impl Config {
    /// The config's YAML bytes in the ONE serializer's style — the
    /// reference dumper that `init` and `config set` write
    /// (seeds-a84c). serde_yaml renders the document (field order,
    /// skipped defaults, flattened extras); the dumper re-renders it
    /// in the reference's style. Output ends with exactly one final
    /// newline (seeds-d2c5).
    #[must_use]
    pub fn to_reference_yaml(&self) -> String {
        let document: Yaml =
            serde_yaml::from_str(&self.to_yaml()).expect("a serialized config always parses back");
        match document {
            Yaml::Mapping(mapping) => dump_mapping(&mapping, 0),
            _ => unreachable!("a config always serializes as a mapping"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_known_and_unknown_keys() {
        let text =
            "project: \"seeds\"\nversion: \"1\"\nmax_plan_depth: 3\nreviewers: [\"operator\"]\n";
        let config = Config::parse(text, std::path::Path::new("config.yaml")).expect("valid");
        assert_eq!(config.project, "seeds");
        assert_eq!(config.version, "1");
        assert_eq!(config.max_plan_depth, 3);
        assert!(config.extra.contains_key("reviewers"));

        let rewritten = Config::parse(&config.to_yaml(), std::path::Path::new("config.yaml"))
            .expect("rewritten config stays valid");
        assert_eq!(rewritten, config);
    }

    #[test]
    fn config_defaults_optional_keys() {
        let config =
            Config::parse("project: demo\n", std::path::Path::new("config.yaml")).expect("valid");
        assert_eq!(config.version, "1");
        assert_eq!(config.max_plan_depth, 3);
        assert_eq!(config.vcs_manager, VcsManager::Git);
        assert!(config.extra.is_empty());
    }

    // seeds-d2c5: to_yaml is chomped — the writer, not the renderer,
    // owns the config file's single final newline. serde_yaml
    // terminates its document with '\n'; leaving that in would make
    // Store::save's format! wrapper add a second (a blank line).
    #[test]
    fn config_to_yaml_carries_no_trailing_newline() {
        let config =
            Config::parse("project: demo\n", std::path::Path::new("config.yaml")).expect("valid");
        let yaml = config.to_yaml();
        assert!(!yaml.ends_with('\n'), "to_yaml must be chomped: {yaml:?}");
    }

    // seeds-b657: the store's surface labels ride in config.yaml as a
    // plain list; an unset store round-trips without the key (byte
    // stability of the rewrite), a set store keeps its labels.
    #[test]
    fn config_reads_and_preserves_surfaces() {
        let config = Config::parse(
            "project: demo\nsurfaces: [shipping, neutral]\n",
            std::path::Path::new("config.yaml"),
        )
        .expect("valid");
        assert_eq!(config.surfaces, vec![
            "shipping".to_owned(),
            "neutral".to_owned()
        ]);

        let plain =
            Config::parse("project: demo\n", std::path::Path::new("config.yaml")).expect("valid");
        assert_eq!(plain.surfaces, Vec::<String>::new());
        assert!(
            !plain.to_yaml().contains("surfaces"),
            "unset surfaces stay absent from rewrites"
        );

        let rewritten = Config::parse(&config.to_yaml(), std::path::Path::new("config.yaml"))
            .expect("rewritten config stays valid");
        assert_eq!(rewritten.surfaces, config.surfaces);
    }

    #[test]
    fn config_round_trips_vcs_manager_modes() {
        for (text, manager) in [
            ("project: demo\nvcs_manager: git\n", VcsManager::Git),
            (
                "project: demo\nvcs_manager: gitbutler\n",
                VcsManager::GitButler,
            ),
        ] {
            let config = Config::parse(text, std::path::Path::new("config.yaml")).expect("valid");
            assert_eq!(config.vcs_manager, manager);
            let rewritten = Config::parse(&config.to_yaml(), std::path::Path::new("config.yaml"))
                .expect("rewritten config stays valid");
            assert_eq!(rewritten, config, "round-trips: {text}");
        }
    }
}
