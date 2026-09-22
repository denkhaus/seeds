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

/// Writes `contents` to `path` atomically (seeds-540e): the payload
/// lands in a hidden temp sibling first and is renamed over the
/// target, so a crash can never leave a partial JSONL store behind.
fn write_file(path: &Path, contents: String) -> Result<(), Error> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("store");
    let temp = path.with_file_name(format!(".{name}.tmp"));
    fs::write(&temp, contents).map_err(|source| Error::Io {
        path: temp.clone(),
        source,
    })?;
    match fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(source) => {
            // Best-effort cleanup: the rename failed, so the temp
            // sibling holds the only copy of the new contents.
            let _ = fs::remove_file(&temp);
            Err(Error::Io { path: temp, source })
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "seeds-store-{}-{tag}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ));
        let root = dir.join(".seeds");
        fs::create_dir_all(&root).expect("temp .seeds dir");
        fs::write(
            root.join("config.yaml"),
            "project: \"tst\"\nversion: \"1\"\n",
        )
        .expect("config.yaml");
        root
    }

    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    // seeds-540e: saves go temp-file+rename, so no `.tmp` sibling is
    // ever left behind and no partial JSONL is observable.
    #[test]
    fn save_leaves_no_temp_siblings_and_replaces_fully() {
        let root = temp_store("atomic");
        let store = Store::open(&root).expect("store opens");
        store.save().expect("first save");

        let mut listed: Vec<String> = fs::read_dir(&root)
            .expect("read .seeds")
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        listed.sort();
        assert_eq!(
            listed,
            vec![
                "config.yaml".to_owned(),
                "issues.jsonl".to_owned(),
                "plans.jsonl".to_owned(),
                "templates.jsonl".to_owned(),
            ],
            "no partial-write temp siblings survive the rename"
        );

        // A save over an existing file replaces it wholesale: a longer,
        // record-bearing file is replaced by an empty write — no stale
        // bytes survive the rename.
        fs::write(
            root.join("issues.jsonl"),
            concat!(
                "{\"id\":\"tst-0001\",\"title\":\"a\",\"status\":\"open\",",
                "\"type\":\"task\",\"priority\":2,",
                "\"createdAt\":\"2026-01-01T00:00:00.000Z\",",
                "\"updatedAt\":\"2026-01-01T00:00:00.000Z\"}\n"
            ),
        )
        .expect("seed one record");
        let mut store = Store::open(&root).expect("store reopens");
        assert_eq!(store.issues.len(), 1);
        store.issues.clear();
        store.save().expect("second save");
        let issues = fs::read_to_string(root.join("issues.jsonl")).expect("issues.jsonl");
        assert_eq!(issues, "", "the rename replaced the file exactly");

        fs::remove_dir_all(root.parent().expect("parent")).ok();
    }
}
