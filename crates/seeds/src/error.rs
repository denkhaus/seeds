//! Error types for the seeds format core.

use std::path::PathBuf;

/// Top-level crate error: store-level failures (I/O, JSONL parsing, config).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O failure on a store file.
    #[error("I/O error on `{path}`: {source}")]
    Io {
        /// The store file being read or written.
        path:   PathBuf,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// A JSONL line failed to parse.
    #[error("invalid JSON in `{file}` line {line}: {source}")]
    JsonLine {
        /// The JSONL file being parsed.
        file:   PathBuf,
        /// The 1-based line number.
        line:   usize,
        /// The underlying JSON failure.
        #[source]
        source: serde_json::Error,
    },
    /// A parsed record violates the format contract (bad id, wrong field type).
    #[error("invalid record in `{file}` line {line}: {message}")]
    Record {
        /// The JSONL file being parsed.
        file:    PathBuf,
        /// The 1-based line number.
        line:    usize,
        /// What is wrong with the record.
        message: String,
    },
    /// The YAML config failed to parse.
    #[error("invalid config `{path}`: {source}")]
    Config {
        /// The config file.
        path:   PathBuf,
        /// The underlying YAML failure.
        #[source]
        source: serde_yaml::Error,
    },
}

/// Failure while validating a single record's fields.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RecordError {
    /// The `id` field is missing or malformed.
    #[error("invalid id: {0}")]
    Id(#[from] IdError),
    /// A known field has the wrong shape for its meaning.
    #[error("field `{field}`: {message}")]
    Field {
        /// The offending field name.
        field:   &'static str,
        /// What is wrong with the value.
        message: String,
    },
}

/// Failure while constructing a validated id newtype.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdError {
    /// Not a valid seed id (`<project>-<hex4>`).
    #[error("invalid seed id `{0}`: expected <project>-<hex4>")]
    Seed(String),
    /// Not a valid plan id (`pl-<hex4>`).
    #[error("invalid plan id `{0}`: expected pl-<hex4>")]
    Plan(String),
    /// Not a valid template id (`tpl-<hex4>`).
    #[error("invalid template id `{0}`: expected tpl-<hex4>")]
    Template(String),
}
