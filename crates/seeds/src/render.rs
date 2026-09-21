//! Text rendering for list/ready/search and show output, matching sd
//! 0.5.15's line shapes.

// Incremental `push_str(&format!(…))` line building reads clearer here
// than chained reassignment across a dozen optional fields.
#![allow(
    clippy::format_push_string,
    reason = "line assembly over optional fields is clearer incrementally"
)]

use seeds::{SeedRecord, SeedType, Status};

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

fn priority_name(record: &SeedRecord) -> &'static str {
    PRIORITY_NAMES[usize::from(record.priority().map_or(4, seeds::Priority::get))]
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
/// `match(es)`).
fn footer(command: &str, count: usize) -> String {
    match command {
        "ready" => format!("{count} ready issue(s)"),
        "search" => format!("{count} match(es)"),
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
