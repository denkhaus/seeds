//! stats and doctor command semantics (sd 0.5.15-compatible).

use serde_json::{Map, Value, json};

use super::{
    CommandContext, CommandError, CommandOutcome, WriteFormat, envelope_pretty, push_line,
};
use crate::{Pace, PaceOptions, Status, SurfacePace, SurfaceReport, WriterLock, doctor, render};

/// Typed input for `stats`.
#[derive(Clone, Debug, Default)]
pub struct StatsInput {
    /// JSON envelope output.
    pub json:    bool,
    /// `--pace`: the pace section (closure rates, lead time, ETA, week
    /// table); absent means the parity surface only.
    pub pace:    Option<PaceOptions>,
    /// `--surface <label>` (seeds-b657): scope the pace to the
    /// records carrying the label — an ad-hoc scope that works with
    /// any label, configured or not; implies `--pace`.
    pub surface: Option<String>,
}

/// `stats` — project statistics by status, type, priority, and label,
/// plus the pace section behind `--pace` (seeds-8fc5).
///
/// The relevance-scoped surface layer (seeds-b657): without
/// `--surface`, the queue-wide pace stays the baseline and the
/// store's configured `surfaces` render as one group each plus an
/// `unclassified` count; with `--surface <label>`, the whole
/// computation runs scoped to that label instead. An unconfigured
/// store gets the setup hint — here, not in `prime`.
pub fn stats(ctx: &CommandContext, input: &StatsInput) -> CommandOutcome {
    let json = input.json;
    let result = (|| -> Result<(render::Stats, Option<PaceView>), CommandError> {
        let store = ctx
            .open_store()
            .map_err(|message| CommandError::new("stats", message, json))?;
        let unresolved = |dep: &str| -> bool {
            store
                .issue(dep)
                .is_none_or(|blocker| blocker.status() != Some(Status::Closed))
        };
        let stats = render::Stats::collect(&store.issues, &unresolved);
        let pace = match input.pace.as_ref() {
            None => None,
            Some(options) => {
                let view = if let Some(label) = input.surface.as_deref() {
                    PaceView::Scoped(
                        SurfacePace::collect(&store.issues, label, options)
                            .map_err(|message| CommandError::new("stats", message, json))?,
                    )
                } else {
                    let pace = Pace::collect(&store.issues, options)
                        .map_err(|message| CommandError::new("stats", message, json))?;
                    let surfaces = if store.config.surfaces.is_empty() {
                        SurfacesView::Hint
                    } else {
                        SurfacesView::Groups(
                            SurfaceReport::collect(&store.issues, &store.config.surfaces, options)
                                .map_err(|message| CommandError::new("stats", message, json))?,
                        )
                    };
                    PaceView::Default { pace, surfaces }
                };
                Some(view)
            }
        };
        Ok((stats, pace))
    })();
    match result {
        Ok((stats, pace)) => {
            let mut out = String::new();
            if json {
                let mut by_type = Map::new();
                for (kind, count) in &stats.by_type {
                    by_type.insert(kind.clone(), json!(count));
                }
                let mut by_priority = Map::new();
                for (level, counts) in &stats.by_priority {
                    // The reference's all-status totals (seeds-4cf6
                    // splits only the text display).
                    by_priority.insert(level.to_string(), json!(counts.total()));
                }
                let mut by_label = Map::new();
                for (label, counts) in &stats.by_label {
                    by_label.insert(label.clone(), json!(counts.total()));
                }
                let mut stats_map = Map::new();
                stats_map.insert("total".to_owned(), json!(stats.total));
                stats_map.insert("open".to_owned(), json!(stats.open));
                stats_map.insert("inProgress".to_owned(), json!(stats.in_progress));
                stats_map.insert("closed".to_owned(), json!(stats.closed));
                stats_map.insert("blocked".to_owned(), json!(stats.blocked));
                stats_map.insert("byType".to_owned(), Value::Object(by_type));
                stats_map.insert("byPriority".to_owned(), Value::Object(by_priority));
                stats_map.insert("byLabel".to_owned(), Value::Object(by_label));
                let mut extra = vec![("stats", Value::Object(stats_map))];
                match &pace {
                    Some(PaceView::Default { pace, surfaces }) => {
                        extra.push(("pace", pace.json_value()));
                        let surfaces = match surfaces {
                            SurfacesView::Groups(report) => report.json_value(),
                            SurfacesView::Hint => json!({ "hint": SURFACES_HINT }),
                        };
                        extra.push(("surfaces", surfaces));
                    }
                    Some(PaceView::Scoped(group)) => {
                        extra.push(("pace", group.scoped_json_value()));
                    }
                    None => {}
                }
                let text = envelope_pretty("stats", &extra);
                out.push_str(&text);
                out.push('\n');
            } else {
                out.push_str(&stats.text());
                match &pace {
                    Some(PaceView::Default { pace, surfaces }) => {
                        out.push('\n');
                        out.push_str(&pace.text());
                        out.push('\n');
                        match surfaces {
                            SurfacesView::Groups(report) => out.push_str(&report.text()),
                            SurfacesView::Hint => {
                                out.push_str("Surfaces\n");
                                out.push_str("  ");
                                out.push_str(SURFACES_HINT);
                                out.push('\n');
                            }
                        }
                    }
                    Some(PaceView::Scoped(group)) => {
                        out.push('\n');
                        out.push_str(&group.text_section());
                    }
                    None => {}
                }
            }
            CommandOutcome::ok(out)
        }
        Err(error) => error.into_outcome(),
    }
}

/// The `--pace` output shape: the queue-wide baseline plus the
/// configured surface layer, or one ad-hoc `--surface` scope standing
/// alone.
enum PaceView {
    /// The queue-wide pace with the configured groups (or the setup
    /// hint when the store declares no surfaces).
    Default {
        pace:     Pace,
        surfaces: SurfacesView,
    },
    /// One ad-hoc scoped group (`--surface <label>`).
    Scoped(SurfacePace),
}

/// The configured-surface layer of the default pace view.
enum SurfacesView {
    Groups(SurfaceReport),
    Hint,
}

/// The setup reminder when the store declares no surfaces
/// (seeds-b657): `stats --pace` is the operator surface, so the hint
/// lives here — `prime` stays silent.
const SURFACES_HINT: &str = "not configured — set 'surfaces: [ <label>, ... ]' in .seeds/config.yaml for relevance-scoped pace";

/// Typed input for `doctor`.
#[derive(Clone, Debug, Default)]
pub struct DoctorInput {
    /// `--fix` (apply repairs).
    pub fix:           bool,
    /// `--repair-report` (append the repair plan in text mode).
    pub repair_report: bool,
    /// `--format` raw text (seeds-1579).
    pub format:        Option<String>,
    /// JSON envelope output.
    pub json:          bool,
}

/// `doctor` — store integrity checks; unsuccessful when any check
/// fails.
pub fn doctor(ctx: &CommandContext, input: &DoctorInput) -> CommandOutcome {
    let mode = WriteFormat::resolve(input.format.as_deref(), input.json);
    let json = mode == WriteFormat::Json;
    let fix = input.fix;
    let repair_report = input.repair_report;
    let result = (|| -> Result<doctor::Report, CommandError> {
        let root = ctx
            .resolve()
            .map_err(|message| CommandError::new("doctor", message, json))?;
        // Repairs rewrite store files: one writer at a time
        // (seeds-0d7b). The report-only mode stays lock-free.
        let _lock = if fix {
            Some(
                WriterLock::acquire(root)
                    .map_err(|error| CommandError::new("doctor", error.to_string(), json))?,
            )
        } else {
            None
        };
        doctor::run(root, fix).map_err(|message| CommandError::new("doctor", message, json))
    })();
    match result {
        Ok(report) => {
            let mut out = String::new();
            match mode {
                WriteFormat::Json => {
                    let mut envelope = Map::new();
                    envelope.insert("success".to_owned(), json!(!report.has_failures()));
                    envelope.insert("command".to_owned(), json!("doctor"));
                    let Value::Object(report_map) = report.json_value(repair_report) else {
                        unreachable!("report json is an object");
                    };
                    for (key, value) in report_map {
                        envelope.insert(key, value);
                    }
                    let text = serde_json::to_string_pretty(&Value::Object(envelope))
                        .expect("envelope always serializes");
                    out.push_str(&text);
                    out.push('\n');
                }
                // `--format ids`: the checks that did not pass, one per
                // line — the report's stable check names (seeds-1579).
                WriteFormat::Ids => {
                    for check in &report.checks {
                        if check.status != doctor::CheckStatus::Pass {
                            push_line(&mut out, check.name);
                        }
                    }
                }
                WriteFormat::Text => {
                    out.push_str(&report.text());
                    if repair_report {
                        out.push_str(&report.repair_report_text());
                    }
                }
            }
            CommandOutcome {
                success: !report.has_failures(),
                stdout:  out,
                stderr:  String::new(),
            }
        }
        Err(error) => error.into_outcome(),
    }
}
