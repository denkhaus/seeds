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
        assert!(config.extra.is_empty());
    }
}
