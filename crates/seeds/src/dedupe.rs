//! Native `seeds dedupe`: per-file duplicate-id heal for the tracker's
//! JSONL stores (issues, plans, templates) — an additive command beyond
//! sd parity (ADR-0023).
//!
//! One record per id per file: the occurrence with the newest
//! `updatedAt` wins (ties break to the later line; a missing
//! `updatedAt` counts as oldest), and the surviving lines keep
//! first-seen order. Lines are treated as opaque JSON text — a heal
//! re-serializes nothing, so kept records keep their original bytes.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// The tracker JSONL files `dedupe` covers, in report order.
pub(crate) const TRACKER_FILES: [&str; 3] = ["issues.jsonl", "plans.jsonl", "templates.jsonl"];

/// One duplicated id within a file: how many times it occurred, which
/// `updatedAt` survived, and the dropped occurrences' `updatedAt`
/// values in line order.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Duplicate {
    pub id:                  String,
    pub count:               usize,
    pub kept_updated_at:     Option<String>,
    pub dropped_updated_ats: Vec<Option<String>>,
}

/// Per-id accumulator while scanning one file's lines.
struct Group<'a> {
    id:              &'a str,
    first:           usize,
    count:           usize,
    best_index:      usize,
    best_updated_at: Option<&'a str>,
    dropped:         Vec<Option<&'a str>>,
}

impl<'a> Group<'a> {
    fn new(id: &'a str, index: usize, updated_at: Option<&'a str>) -> Self {
        Self {
            id,
            first: index,
            count: 1,
            best_index: index,
            best_updated_at: updated_at,
            dropped: Vec::new(),
        }
    }
}

/// Heals one file's non-empty lines: keeps, per record id, the
/// occurrence with the newest `updatedAt` (ties: later line wins) and
/// returns the kept line texts in first-seen order plus one
/// [`Duplicate`] per duplicated id, in first-seen order. Lines that
/// fail to parse or carry no `id` pass through ungrouped.
pub(crate) fn heal_lines(lines: &[&str]) -> (Vec<String>, Vec<Duplicate>) {
    // Parsed values stay alive for the whole function so the borrowed
    // ids and timestamps below have one stable owner.
    let parsed_lines: Vec<Option<Value>> = lines
        .iter()
        .map(|line| serde_json::from_str(line).ok())
        .collect();
    // id and updatedAt per line, parallel to `lines`; None marks a
    // pass-through line (unparseable or id-less).
    let mut ids: Vec<Option<&str>> = Vec::with_capacity(lines.len());
    let mut groups: HashMap<&str, Group> = HashMap::new();

    for (index, parsed) in parsed_lines.iter().enumerate() {
        let str_field = |name: &str| {
            parsed
                .as_ref()
                .and_then(|value| value.get(name))
                .and_then(Value::as_str)
        };
        let (id, updated_at) = (str_field("id"), str_field("updatedAt"));
        ids.push(id);
        let Some(id) = id else { continue };
        let group = groups
            .entry(id)
            .or_insert_with(|| Group::new(id, index, updated_at));
        if index == group.first {
            continue; // the occurrence above just initialized the group
        }
        group.count += 1;
        // A candidate at a later index beats the best on `>=` — an
        // equal `updatedAt` resolves to the later line.
        let newer = match (updated_at, group.best_updated_at) {
            (Some(_), None) => true,
            (Some(candidate), Some(best)) => candidate >= best,
            (None, _) => false,
        };
        if newer {
            group.dropped.push(group.best_updated_at);
            group.best_updated_at = updated_at;
            group.best_index = index;
        } else {
            group.dropped.push(updated_at);
        }
    }

    let mut ordered: Vec<&Group<'_>> = groups.values().collect();
    ordered.sort_by_key(|group| group.first);

    let mut healed = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let Some(id) = ids[index] else {
            healed.push((*line).to_owned());
            continue;
        };
        let group = &groups[id];
        if index == group.first {
            healed.push(lines[group.best_index].to_owned());
        } // repeat occurrences drop
    }

    let duplicates = ordered
        .into_iter()
        .filter(|group| group.count > 1)
        .map(|group| Duplicate {
            id:                  group.id.to_owned(),
            count:               group.count,
            kept_updated_at:     group.best_updated_at.map(str::to_owned),
            dropped_updated_ats: group
                .dropped
                .iter()
                .map(|value| value.map(str::to_owned))
                .collect(),
        })
        .collect();
    (healed, duplicates)
}

/// Writes `contents` to `path` via a temp file in the same directory
/// followed by a rename, so a crash never leaves a half-written store.
pub(crate) fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let temp = path.with_extension("dedupe-tmp");
    std::fs::write(&temp, contents)?;
    std::fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn line(id: &str, updated_at: Option<&str>, tag: &str) -> String {
        let mut record = serde_json::Map::new();
        record.insert("id".to_owned(), json!(id));
        record.insert("title".to_owned(), json!(tag));
        if let Some(updated_at) = updated_at {
            record.insert("updatedAt".to_owned(), json!(updated_at));
        }
        serde_json::Value::Object(record).to_string()
    }

    fn borrowed(lines: &[String]) -> Vec<&str> {
        lines.iter().map(String::as_str).collect()
    }

    #[test]
    fn keeps_newest_per_id_in_first_seen_order() {
        let input = vec![
            line("a-1", Some("2026-01-02T00:00:00.000Z"), "a-old"),
            line("b-1", Some("2026-01-03T00:00:00.000Z"), "b"),
            line("a-1", Some("2026-01-05T00:00:00.000Z"), "a-new"),
            line("a-1", Some("2026-01-04T00:00:00.000Z"), "a-mid"),
        ];
        let (healed, duplicates) = heal_lines(&borrowed(&input));
        assert_eq!(healed, vec![
            line("a-1", Some("2026-01-05T00:00:00.000Z"), "a-new"),
            line("b-1", Some("2026-01-03T00:00:00.000Z"), "b"),
        ]);
        assert_eq!(duplicates, vec![Duplicate {
            id:                  "a-1".to_owned(),
            count:               3,
            kept_updated_at:     Some("2026-01-05T00:00:00.000Z".to_owned()),
            dropped_updated_ats: vec![
                Some("2026-01-02T00:00:00.000Z".to_owned()),
                Some("2026-01-04T00:00:00.000Z".to_owned()),
            ],
        }]);
    }

    #[test]
    fn tie_breaks_to_the_later_line() {
        let stamp = "2026-01-05T00:00:00.000Z";
        let input = vec![
            line("a-1", Some(stamp), "first"),
            line("a-1", Some(stamp), "later"),
        ];
        let (healed, duplicates) = heal_lines(&borrowed(&input));
        assert_eq!(healed, vec![line("a-1", Some(stamp), "later")]);
        assert_eq!(duplicates, vec![Duplicate {
            id:                  "a-1".to_owned(),
            count:               2,
            kept_updated_at:     Some(stamp.to_owned()),
            dropped_updated_ats: vec![Some(stamp.to_owned())],
        }]);
    }

    #[test]
    fn missing_updated_at_counts_as_oldest() {
        let input = vec![
            line("a-1", None, "no-stamp"),
            line("a-1", Some("2026-01-01T00:00:00.000Z"), "stamped"),
        ];
        let (healed, duplicates) = heal_lines(&borrowed(&input));
        assert_eq!(healed.len(), 1);
        assert!(healed[0].contains("stamped"));
        assert_eq!(duplicates[0].dropped_updated_ats, vec![None]);
    }

    #[test]
    fn passes_through_lines_without_a_parseable_id() {
        let oddity = "{\"note\": \"not a record\"}";
        let input = vec![
            oddity.to_owned(),
            line("a-1", Some("2026-01-01T00:00:00.000Z"), "a"),
        ];
        let (healed, duplicates) = heal_lines(&borrowed(&input));
        assert_eq!(healed, vec![oddity.to_owned(), input[1].clone()]);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn clean_input_is_unchanged() {
        let input = vec![
            line("a-1", Some("2026-01-01T00:00:00.000Z"), "a"),
            line("b-1", None, "b"),
        ];
        let (healed, duplicates) = heal_lines(&borrowed(&input));
        assert_eq!(healed, input);
        assert!(duplicates.is_empty());
    }
}
