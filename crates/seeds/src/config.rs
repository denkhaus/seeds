//! `.seeds/config.yaml` reader/writer.

use serde::{Deserialize, Serialize};

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

    /// Serializes the config to YAML text (no trailing newline).
    #[must_use]
    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(self).expect("a config always serializes")
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
