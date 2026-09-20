//! The `.seeds/` store: loading and saving the four store files.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::Error;
use crate::model::{Fields, PlanRecord, SeedRecord, TemplateRecord};

/// A loaded `.seeds/` store.
///
/// `Store::open` reads `config.yaml` plus the three JSONL files;
/// `save` writes all four back. Records round-trip with their exact
/// field ordering and unknown fields intact.
#[derive(Clone, Debug)]
pub struct Store {
    root:          PathBuf,
    /// The parsed `config.yaml`.
    pub config:    Config,
    /// The `issues.jsonl` records.
    pub issues:    Vec<SeedRecord>,
    /// The `plans.jsonl` records.
    pub plans:     Vec<PlanRecord>,
    /// The `templates.jsonl` records.
    pub templates: Vec<TemplateRecord>,
}

impl Store {
    /// Opens the store rooted at `root` (the `.seeds/` directory itself).
    ///
    /// A missing JSONL file counts as empty; a missing or invalid
    /// `config.yaml` is an error — it anchors the store's identity.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] on I/O, YAML, or record-validation failures.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, Error> {
        let root = root.as_ref();
        let config_path = root.join("config.yaml");
        let config_text = read_file(&config_path)?;
        let config = Config::parse(&config_text, &config_path)?;

        let issues = load_jsonl(root, "issues.jsonl", SeedRecord::try_from_fields)?;
        let plans = load_jsonl(root, "plans.jsonl", PlanRecord::try_from_fields)?;
        let templates = load_jsonl(root, "templates.jsonl", TemplateRecord::try_from_fields)?;

        Ok(Self {
            root: root.to_owned(),
            config,
            issues,
            plans,
            templates,
        })
    }

    /// Writes all four store files back.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] on I/O failures.
    pub fn save(&self) -> Result<(), Error> {
        let config_path = self.root.join("config.yaml");
        write_file(&config_path, format!("{}\n", self.config.to_yaml()))?;

        save_jsonl(
            &self.root,
            "issues.jsonl",
            &self.issues,
            SeedRecord::to_json_line,
        )?;
        save_jsonl(
            &self.root,
            "plans.jsonl",
            &self.plans,
            PlanRecord::to_json_line,
        )?;
        save_jsonl(
            &self.root,
            "templates.jsonl",
            &self.templates,
            TemplateRecord::to_json_line,
        )?;
        Ok(())
    }

    /// The store root (the `.seeds/` directory).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Finds an issue record by id.
    #[must_use]
    pub fn issue(&self, id: &str) -> Option<&SeedRecord> {
        self.issues.iter().find(|record| record.id().as_str() == id)
    }

    /// Mutably finds an issue record by id.
    #[must_use]
    pub fn issue_mut(&mut self, id: &str) -> Option<&mut SeedRecord> {
        self.issues
            .iter_mut()
            .find(|record| record.id().as_str() == id)
    }
}

fn read_file(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })
}

fn write_file(path: &Path, contents: String) -> Result<(), Error> {
    fs::write(path, contents).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })
}

/// Parses one JSONL file into validated records. A missing file is empty.
fn load_jsonl<T, F>(root: &Path, name: &str, build: F) -> Result<Vec<T>, Error>
where
    F: Fn(Fields) -> Result<T, crate::error::RecordError>,
{
    let path = root.join(name);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };

    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            continue;
        }
        let fields: Fields = serde_json::from_str(line).map_err(|source| Error::JsonLine {
            file: path.clone(),
            line: index + 1,
            source,
        })?;
        let record = build(fields).map_err(|error| Error::Record {
            file:    path.clone(),
            line:    index + 1,
            message: error.to_string(),
        })?;
        records.push(record);
    }
    Ok(records)
}

/// Writes records as compact JSONL with a trailing newline per line;
/// an empty collection writes an empty file, matching sd.
fn save_jsonl<T>(
    root: &Path,
    name: &str,
    records: &[T],
    render: impl Fn(&T) -> String,
) -> Result<(), Error> {
    let path = root.join(name);
    if records.is_empty() {
        return write_file(&path, String::new());
    }
    let mut text = String::new();
    for record in records {
        text.push_str(&render(record));
        text.push('\n');
    }
    write_file(&path, text)
}
