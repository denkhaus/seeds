//! Text rendering for list/ready/search and show output, matching sd
//! 0.5.15's line shapes.

// Incremental `push_str(&format!(…))` line building reads clearer here
// than chained reassignment across a dozen optional fields.
#![allow(
    clippy::format_push_string,
    reason = "line assembly over optional fields is clearer incrementally"
)]

use crate::{SeedRecord, SeedType, Status};

/// The `--format` output modes (JSON is handled by the envelope path).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RenderMode {
    /// The rich default view with status icons.
    Markdown,
    /// `id Priority statusWord title` lines, no footer.
    Compact,
    /// `- id · title   [Priority · type]` lines with footer.
    Plain,
    /// One id per line, no footer.
    Ids,
    /// Unused by callers (JSON goes through the envelope path) but kept
    /// exhaustive so `render_mode` stays total.
    Json,
}

const PRIORITY_NAMES: [&str; 5] = ["Critical", "High", "Medium", "Low", "Backlog"];

/// Maps a `--format` value to its render mode (JSON goes through the
/// envelope path).
pub(crate) fn render_mode(format: Option<&str>) -> RenderMode {
    match format {
        Some("compact") => RenderMode::Compact,
        Some("plain") => RenderMode::Plain,
        Some("ids") => RenderMode::Ids,
        Some("json") => RenderMode::Json,
        // The default plus `markdown` share the reference's rich view.
        _ => RenderMode::Markdown,
    }
}

fn priority_name(record: &SeedRecord) -> &'static str {
    PRIORITY_NAMES[usize::from(record.priority().map_or(4, crate::Priority::get))]
}

/// The status icon of the rich list line.
fn icon(record: &SeedRecord, blocked: bool) -> &'static str {
    match record.status() {
        Some(Status::InProgress) => ">",
        Some(Status::Closed) => "x",
        _ if blocked => "!",
        _ => "-",
    }
}

fn type_word(record: &SeedRecord) -> &'static str {
    record.seed_type().map_or("task", SeedType::as_str)
}

/// Renders one rich list line: `icon id · title   [Prio · type] …`.
fn rich_line(record: &SeedRecord, blocked: bool) -> String {
    let mut line = format!(
        "{} {} · {}   [{} · {}]",
        icon(record, blocked),
        record.id(),
        record.title(),
        priority_name(record),
        type_word(record),
    );
    if record.status() == Some(Status::Open) && blocked {
        line.push_str(" [blocked]");
    }
    if let Some(assignee) = record.assignee() {
        line.push_str(&format!(" · @{assignee}"));
    }
    let labels = record.labels();
    if !labels.is_empty() {
        line.push_str(&format!(" {{{}}}", labels.join(", ")));
    }
    line
}

/// Renders one plain line: `- id · title   [Prio · type]`.
fn plain_line(record: &SeedRecord) -> String {
    format!(
        "- {} · {}   [{} · {}]",
        record.id(),
        record.title(),
        priority_name(record),
        type_word(record),
    )
}

/// Renders one compact line: `id Prio statusWord title`.
fn compact_line(record: &SeedRecord, blocked: bool) -> String {
    let status_word = match record.status() {
        Some(Status::Closed) => "closed",
        _ if blocked => "blocked",
        Some(Status::InProgress) => "in_progress",
        _ => "open",
    };
    format!(
        "{} {} {} {}",
        record.id(),
        priority_name(record),
        status_word,
        record.title(),
    )
}

/// The footer count per command (`issue(s)`, `ready issue(s)`,
/// `match(es)`, `blocked issue(s)`).
fn footer(command: &str, count: usize) -> String {
    match command {
        "ready" => format!("{count} ready issue(s)"),
        "search" => format!("{count} match(es)"),
        "blocked" => format!("{count} blocked issue(s)"),
        _ => format!("{count} issue(s)"),
    }
}

/// Renders a list of records in the requested mode. `is_unresolved`
/// resolves `blockedBy` ids against the same store the records came
/// from.
pub(crate) fn list_text(
    records: &[SeedRecord],
    mode: RenderMode,
    is_unresolved: &dyn Fn(&str) -> bool,
    command: &str,
) -> String {
    let blocked: Vec<bool> = records
        .iter()
        .map(|record| record.blocked_by().iter().any(|dep| is_unresolved(dep)))
        .collect();
    match mode {
        RenderMode::Ids => {
            let mut text = String::new();
            for record in records {
                text.push_str(record.id().as_str());
                text.push('\n');
            }
            text
        }
        RenderMode::Compact => {
            let mut text = String::new();
            for (record, blocked) in records.iter().zip(blocked) {
                text.push_str(&compact_line(record, blocked));
                text.push('\n');
            }
            text
        }
        RenderMode::Markdown | RenderMode::Json | RenderMode::Plain => {
            let mut text = String::new();
            for (record, blocked) in records.iter().zip(blocked) {
                let line = match mode {
                    RenderMode::Plain => plain_line(record),
                    _ => rich_line(record, blocked),
                };
                text.push_str(&line);
                text.push('\n');
            }
            text.push('\n');
            text.push_str(&footer(command, records.len()));
            text.push('\n');
            text
        }
    }
}

/// Renders the detailed show view (default mode); other modes reuse the
/// list line shapes.
pub(crate) fn show_text(records: &[SeedRecord], mode: RenderMode) -> String {
    match mode {
        RenderMode::Markdown | RenderMode::Json => {
            records
                .iter()
                .map(detail)
                .collect::<Vec<_>>()
                .join("\n---\n\n")
                + "\n"
        }
        RenderMode::Compact => list_text(records, RenderMode::Compact, &|_| false, "show"),
        RenderMode::Plain => list_text(records, RenderMode::Plain, &|_| false, "show"),
        RenderMode::Ids => list_text(records, RenderMode::Ids, &|_| false, "show"),
    }
}

/// The detailed single-record view, mirroring sd's field widths.
fn detail(record: &SeedRecord) -> String {
    let mut text = format!(
        "{}  {}\n",
        record.id(),
        record.status().map_or("", Status::as_str),
    );
    text.push_str(&format!("Title:    {}\n", record.title()));
    text.push_str(&format!(
        "Type:     {}   Priority: {}\n",
        type_word(record),
        priority_name(record),
    ));
    if let Some(assignee) = record.assignee() {
        text.push_str(&format!("Assignee: {assignee}\n"));
    }
    let labels = record.labels();
    if !labels.is_empty() {
        text.push_str(&format!("Labels:   {}\n", labels.join(", ")));
    }
    if let Some(description) = record.description() {
        text.push('\n');
        text.push_str(description);
        text.push('\n');
    }
    let blocked_by = record.blocked_by();
    if !blocked_by.is_empty() {
        text.push_str(&format!("Blocked by: {}\n", blocked_by.join(", ")));
    }
    let blocks = record.blocks();
    if !blocks.is_empty() {
        text.push_str(&format!("Blocks:     {}\n", blocks.join(", ")));
    }
    if let Some(reason) = record.field("closeReason").and_then(|value| value.as_str()) {
        text.push_str(&format!("Reason:   {reason}\n"));
    }
    text.push_str(&format!(
        "Created:  {}\n",
        record.created_at().unwrap_or_default(),
    ));
    text.push_str(&format!(
        "Updated:  {}\n",
        record.updated_at().unwrap_or_default(),
    ));
    if let Some(closed_at) = record.field("closedAt").and_then(|value| value.as_str()) {
        text.push_str(&format!("Closed:   {closed_at}\n"));
    }
    text
}

// ---------------------------------------------------------------------------
// dep list / label / stats text (hygiene batch, seeds-c228)
// ---------------------------------------------------------------------------

/// Renders `dep list` for one record: rich lines per blocker and per
/// blocked issue, recomputing each line's blocked state from the same
/// store the records came from.
pub(crate) fn dep_list_text(
    record: &SeedRecord,
    store: &crate::Store,
    is_unresolved: &dyn Fn(&str) -> bool,
) -> String {
    let mut text = format!("{} dependencies:\n", record.id());
    let mut any = false;
    let blocked_by = record.blocked_by();
    if !blocked_by.is_empty() {
        text.push_str("  Blocked by:\n");
        for dep in blocked_by {
            if let Some(blocker) = store.issue(dep) {
                let blocked = blocker.blocked_by().iter().any(|d| is_unresolved(d));
                text.push_str(&format!("    {}\n", rich_line(blocker, blocked)));
                any = true;
            }
        }
    }
    let blocks = record.blocks();
    if !blocks.is_empty() {
        text.push_str("  Blocks:\n");
        for dependent in blocks {
            if let Some(dependent_record) = store.issue(dependent) {
                let blocked = dependent_record
                    .blocked_by()
                    .iter()
                    .any(|d| is_unresolved(d));
                text.push_str(&format!("    {}\n", rich_line(dependent_record, blocked)));
                any = true;
            }
        }
    }
    if !any {
        text.push_str("  No dependencies.\n");
    }
    text
}

/// Renders `label list` for one record.
pub(crate) fn label_list_text(record: &SeedRecord) -> String {
    let labels = record.labels();
    if labels.is_empty() {
        return format!("{} has no labels.\n", record.id());
    }
    let mut text = format!("{} labels:\n", record.id());
    for label in labels {
        text.push_str(&format!("  {label}\n"));
    }
    text
}

/// Renders `label list-all`: alphabetically sorted labels with counts,
/// padded to the reference's column, plus the footer count.
pub(crate) fn label_list_all_text(counts: &[(String, usize)]) -> String {
    if counts.is_empty() {
        return "No labels found.\n".to_owned();
    }
    let mut sorted = counts.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut text = String::new();
    for (label, count) in &sorted {
        text.push_str(&format!("  {}{}\n", pad_field(label, 21), count));
    }
    text.push('\n');
    text.push_str(&format!("{} label(s)\n", sorted.len()));
    text
}

/// Pads `text` to `width` with at least one trailing space (the
/// reference's `padEnd` layout for label/type/priority columns).
fn pad_field(text: &str, width: usize) -> String {
    let padding = width.saturating_sub(text.chars().count()).max(1);
    format!("{text}{}", " ".repeat(padding))
}

/// The aggregate counts behind `stats` (encounter order preserved —
/// the reference groups by first appearance, not alphabetically).
pub(crate) struct Stats {
    /// Total issue count.
    pub total:       usize,
    /// `open` issues.
    pub open:        usize,
    /// `in_progress` issues.
    pub in_progress: usize,
    /// `closed` issues.
    pub closed:      usize,
    /// Non-closed issues with unresolved blockers.
    pub blocked:     usize,
    /// Type name → count, first-seen order.
    pub by_type:     Vec<(String, usize)>,
    /// Priority level → count, first-seen order.
    pub by_priority: Vec<(u8, usize)>,
    /// Label → count, first-seen order.
    pub by_label:    Vec<(String, usize)>,
}

fn bump(map: &mut Vec<(String, usize)>, key: &str) {
    if let Some(entry) = map.iter_mut().find(|(existing, _)| existing == key) {
        entry.1 += 1;
    } else {
        map.push((key.to_owned(), 1));
    }
}

impl Stats {
    /// Aggregates the counts over `records`; `is_unresolved` resolves
    /// `blockedBy` ids against the store the records came from.
    pub(crate) fn collect(records: &[SeedRecord], is_unresolved: &dyn Fn(&str) -> bool) -> Self {
        let mut stats = Stats {
            total:       records.len(),
            open:        0,
            in_progress: 0,
            closed:      0,
            blocked:     0,
            by_type:     Vec::new(),
            by_priority: Vec::new(),
            by_label:    Vec::new(),
        };
        for record in records {
            match record.status() {
                Some(Status::Open) => stats.open += 1,
                Some(Status::InProgress) => stats.in_progress += 1,
                Some(Status::Closed) => stats.closed += 1,
                None => {}
            }
            let unresolved = record.blocked_by().iter().any(|dep| is_unresolved(dep));
            if unresolved && record.status() != Some(Status::Closed) {
                stats.blocked += 1;
            }
            bump(&mut stats.by_type, type_word(record));
            let priority = record.priority().map_or(4_u8, crate::Priority::get);
            if let Some(entry) = stats
                .by_priority
                .iter_mut()
                .find(|(existing, _)| *existing == priority)
            {
                entry.1 += 1;
            } else {
                stats.by_priority.push((priority, 1));
            }
            for label in record.labels() {
                bump(&mut stats.by_label, label);
            }
        }
        stats
    }

    /// Renders the parity text view (`Project Statistics` layout).
    pub(crate) fn text(&self) -> String {
        let mut out = String::from("Project Statistics\n");
        for (key, value) in [
            ("Total:", self.total),
            ("Open:", self.open),
            ("In progress:", self.in_progress),
            ("Closed:", self.closed),
            ("Blocked:", self.blocked),
        ] {
            out.push_str(&format!("  {}{}\n", pad_field(key, 13), value));
        }
        out.push('\n');
        out.push_str("By Type\n");
        for (kind, count) in &self.by_type {
            out.push_str(&format!("  {}{}\n", pad_field(kind, 11), count));
        }
        out.push('\n');
        out.push_str("By Priority\n");
        for (level, count) in &self.by_priority {
            let name = format!("P{level} {}", PRIORITY_NAMES[usize::from(*level)]);
            out.push_str(&format!("  {}{}\n", pad_field(&name, 14), count));
        }
        if !self.by_label.is_empty() {
            out.push('\n');
            out.push_str("By Label\n");
            for (label, count) in &self.by_label {
                out.push_str(&format!("  {}{}\n", pad_field(label, 16), count));
            }
        }
        out
    }
}
