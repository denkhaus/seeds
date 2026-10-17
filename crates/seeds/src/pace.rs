//! Pace metrics (seeds-8fc5): the closed-vs-open ratio, the project's
//! pace, and the ETA toward production readiness — reconstructed from
//! the store snapshots (`createdAt`/`closedAt`), no event log.
//!
//! Counting rule (ADR-0002): every closure counts uniformly, whatever
//! its `closeReason` — "closed" means "out of the backlog", and a
//! superseded seed's successor is its own item. The metrics take an
//! injected clock ([`PaceOptions::now`]) so tests can pin time.

// Incremental `push_str(&format!(…))` line building matches the
// rendering layer's shape (render.rs).
#![allow(
    clippy::format_push_string,
    reason = "the pace section is line-assembly output like render.rs"
)]

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{SeedRecord, Status, render, timeutil};

/// The default observation window for the windowed rate, in days.
pub const DEFAULT_WINDOW_DAYS: u32 = 30;

/// The default minimum closures inside the window before an ETA is
/// trusted.
pub const DEFAULT_MIN_CLOSURES: u32 = 5;

const MILLIS_PER_DAY: i64 = 86_400_000;
const MILLIS_PER_WEEK: i64 = 604_800_000;

/// The knobs behind `sd stats --pace`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaceOptions {
    /// Observation window for the windowed rate, in days (`--window`).
    pub window_days: u32,
    /// Minimum closures inside the window for a trusted ETA
    /// (`--min-closures`).
    pub min_closures: u32,
    /// Open seeds carrying any of these labels leave the target
    /// (`--exclude-label`, comma-separated).
    pub exclusion_labels: Vec<String>,
    /// First week of the week table, `YYYY-MM-DD` (`--since`); the
    /// full history when `None`.
    pub since: Option<String>,
    /// Injected clock (`YYYY-MM-DDTHH:MM:SS.mmmZ`); the system clock
    /// when `None`. Not a CLI flag — a library/test seam.
    pub now: Option<String>,
}

impl Default for PaceOptions {
    fn default() -> Self {
        Self {
            window_days: DEFAULT_WINDOW_DAYS,
            min_closures: DEFAULT_MIN_CLOSURES,
            exclusion_labels: Vec::new(),
            since: None,
            now: None,
        }
    }
}

/// One ISO week of the created/closed/opened series.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeekBucket {
    /// The ISO week label (`2026-W38`, UTC).
    pub iso: String,
    /// Seeds created during the week.
    pub created: usize,
    /// Seeds closed during the week.
    pub closed: usize,
    /// `closed - created` (negative while the backlog grows).
    pub net: i64,
    /// Open seeds after the week: a running total over the raw series,
    /// independent of any exclusion list.
    pub open: usize,
}

/// The pace aggregate: windowed and all-time closure rates, median
/// lead time, the open target, the ETA, and the weekly series.
#[derive(Clone, Debug, PartialEq)]
pub struct Pace {
    /// The observation window the windowed rate was taken over.
    pub window_days: u32,
    /// Minimum closures for a trusted ETA (echoed for diagnostics).
    pub min_closures: u32,
    /// Closures per week inside the window.
    pub rate_window: f64,
    /// Closures inside the window.
    pub closures_in_window: usize,
    /// Closures per week over the full history (`None` without
    /// timestamped history).
    pub rate_all_time: Option<f64>,
    /// Closures over the full history.
    pub closures_total: usize,
    /// The history span in weeks (context for the all-time rate).
    pub history_weeks: Option<f64>,
    /// Median `createdAt → closedAt` in hours across closed seeds.
    pub lead_time_median_hours: Option<f64>,
    /// Open seeds that still count toward Production Readiness.
    pub open_to_target: usize,
    /// Open seeds parked by the exclusion list.
    pub excluded_open: usize,
    /// The active exclusion labels (lowercased).
    pub exclusion_labels: Vec<String>,
    /// Projected date of Production Readiness, `YYYY-MM-DD`
    /// (`None` when the window carries too little data).
    pub eta: Option<String>,
    /// Why no ETA is offered: `zero_closures_in_window` or
    /// `insufficient_closures_in_window`.
    pub eta_reason: Option<String>,
    /// The weekly series, oldest first.
    pub weeks: Vec<WeekBucket>,
}

impl Pace {
    /// Reconstructs the pace from `records` at the clock `options`
    /// carries.
    ///
    /// # Errors
    ///
    /// Returns the offending flag name and value when the options or
    /// the injected clock do not parse (`--window 0`, a malformed
    /// `--since`, an unparsable `now`).
    pub fn collect(records: &[SeedRecord], options: &PaceOptions) -> Result<Self, String> {
        if options.window_days == 0 {
            return Err("--window: must be at least 1 day".to_owned());
        }
        let now_text = options.now.clone().unwrap_or_else(timeutil::now_iso);
        let now_ms = timeutil::parse_iso_millis(&now_text).ok_or_else(|| {
            format!("invalid clock timestamp `{now_text}` (expected YYYY-MM-DDTHH:MM:SS.mmmZ)")
        })?;
        let since_days = options
            .since
            .as_deref()
            .map(|text| {
                let millis = timeutil::parse_iso_millis(text)
                    .ok_or_else(|| format!("--since: expected YYYY-MM-DD, got `{text}`"))?;
                Ok::<i64, String>(millis.div_euclid(MILLIS_PER_DAY))
            })
            .transpose()?;
        let exclusion = &options.exclusion_labels;

        let mut creations: Vec<i64> = Vec::new();
        let mut closures: Vec<i64> = Vec::new();
        let mut lead_times: Vec<f64> = Vec::new();
        let mut open_to_target = 0_usize;
        let mut excluded_open = 0_usize;
        for record in records {
            let created = record.created_at().and_then(timeutil::parse_iso_millis);
            creations.extend(created);
            let closed = if record.status() == Some(Status::Closed) {
                record
                    .field("closedAt")
                    .and_then(Value::as_str)
                    .and_then(timeutil::parse_iso_millis)
            } else {
                None
            };
            if let Some(closed_ms) = closed {
                closures.push(closed_ms);
                if let Some(created_ms) = created
                    && closed_ms >= created_ms
                {
                    lead_times.push((closed_ms - created_ms) as f64 / 3_600_000.0);
                }
            }
            if record.status() != Some(Status::Closed) {
                let parked = record.labels().iter().any(|label| {
                    exclusion
                        .iter()
                        .any(|kept| label.eq_ignore_ascii_case(kept))
                });
                if parked {
                    excluded_open += 1;
                } else {
                    open_to_target += 1;
                }
            }
        }

        let window_ms = i64::from(options.window_days) * MILLIS_PER_DAY;
        let window_start = now_ms.saturating_sub(window_ms);
        let closures_in_window = closures
            .iter()
            .filter(|closed_ms| **closed_ms > window_start && **closed_ms <= now_ms)
            .count();
        let rate_window = closures_in_window as f64 * 7.0 / f64::from(options.window_days);

        let earliest = creations.iter().chain(closures.iter()).min().copied();
        let history_weeks = earliest
            .map(|first| (now_ms.saturating_sub(first) as f64 / (MILLIS_PER_WEEK as f64)).max(0.0));
        let closures_total = closures.len();
        let rate_all_time = history_weeks
            .filter(|weeks| *weeks > 0.0)
            .map(|weeks| closures_total as f64 / weeks);

        lead_times.sort_by(f64::total_cmp);
        let lead_time_median_hours = median(&lead_times);

        let now_days = now_ms.div_euclid(MILLIS_PER_DAY);
        let (eta, eta_reason) = if open_to_target == 0 {
            // Production Readiness reached: ETA is today.
            (Some(timeutil::date_string(now_days)), None)
        } else if closures_in_window < usize::try_from(options.min_closures).unwrap_or(usize::MAX) {
            let reason = if closures_in_window == 0 {
                "zero_closures_in_window"
            } else {
                "insufficient_closures_in_window"
            };
            (None, Some(reason.to_owned()))
        } else if rate_window <= 0.0 {
            (None, Some("zero_closures_in_window".to_owned()))
        } else {
            // open / rate_window * 7 days, folded to exact integer
            // arithmetic: rate_window = closures_in_window * 7 /
            // window_days, so days = open * window / closures.
            let window_scale = usize::try_from(options.window_days).unwrap_or(usize::MAX);
            let horizon_days = open_to_target
                .saturating_mul(window_scale)
                .div_ceil(closures_in_window.max(1));
            let horizon_days = i64::try_from(horizon_days).unwrap_or(i64::MAX);
            (Some(timeutil::date_string(now_days + horizon_days)), None)
        };

        let weeks = weekly_series(&creations, &closures, now_days, since_days);

        Ok(Self {
            window_days: options.window_days,
            min_closures: options.min_closures,
            rate_window,
            closures_in_window,
            rate_all_time,
            closures_total,
            history_weeks,
            lead_time_median_hours,
            open_to_target,
            excluded_open,
            exclusion_labels: exclusion.clone(),
            eta,
            eta_reason,
            weeks,
        })
    }

    /// The one-line pace summary for embedded surfaces (`prime`):
    /// windowed rate, open target, and the ETA or its honest absence.
    #[must_use]
    pub fn line(&self) -> String {
        let eta = self
            .eta
            .clone()
            .unwrap_or_else(|| "n/a (insufficient data)".to_owned());
        format!(
            "{:.1} closures/wk ({}d) · {} open to target · ETA {}",
            round1(self.rate_window),
            self.window_days,
            self.open_to_target,
            eta
        )
    }

    /// The `pace` object of the `stats --json` envelope.
    #[must_use]
    pub fn json_value(&self) -> Value {
        json!({
            "window_days": self.window_days,
            "min_closures": self.min_closures,
            "rate_window": round1(self.rate_window),
            "closures_in_window": self.closures_in_window,
            "rate_all_time": self.rate_all_time.map(round1),
            "closures_total": self.closures_total,
            "history_weeks": self.history_weeks.map(round1),
            "lead_time_median_hours": self.lead_time_median_hours.map(round1),
            "open_to_target": self.open_to_target,
            "excluded_open": self.excluded_open,
            "exclusion_labels": self.exclusion_labels,
            "eta": self.eta,
            "eta_reason": self.eta_reason,
            "weeks": self
                .weeks
                .iter()
                .map(|week| json!({
                    "iso": week.iso,
                    "created": week.created,
                    "closed": week.closed,
                    "net": week.net,
                    "open": week.open,
                }))
                .collect::<Vec<Value>>(),
        })
    }

    /// The `Pace` section plus the week table (text mode).
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::from("Pace\n");
        let window_label = format!("Rate ({}d):", self.window_days);
        out.push_str(&format!(
            "  {}{:.1} / week ({} closures in window)\n",
            render::pad_field(&window_label, 20),
            self.rate_window,
            self.closures_in_window
        ));
        let all_time = self.rate_all_time.zip(self.history_weeks).map_or_else(
            || "n/a (no timestamped history)".to_owned(),
            |(rate, weeks)| {
                format!(
                    "{rate:.1} / week ({} closures in {weeks:.1} weeks)",
                    self.closures_total
                )
            },
        );
        out.push_str(&format!(
            "  {}{}\n",
            render::pad_field("Rate (all-time):", 20),
            all_time
        ));
        let lead = self.lead_time_median_hours.map_or_else(
            || "n/a (no closed seeds)".to_owned(),
            |hours| format!("{hours:.1}h"),
        );
        out.push_str(&format!(
            "  {}{}\n",
            render::pad_field("Lead time (median):", 20),
            lead
        ));
        let exclusion = if self.exclusion_labels.is_empty() {
            "none".to_owned()
        } else {
            format!(
                "{} — {} open excluded",
                self.exclusion_labels.join(", "),
                self.excluded_open
            )
        };
        out.push_str(&format!(
            "  {}{} (exclusion: {})\n",
            render::pad_field("Open to target:", 20),
            self.open_to_target,
            exclusion
        ));
        out.push_str(&format!(
            "  {}{}\n",
            render::pad_field("ETA:", 20),
            self.eta_text()
        ));
        if !self.weeks.is_empty() {
            out.push('\n');
            out.push_str(&week_row(
                "Week (ISO, UTC)",
                "created",
                "closed",
                "net",
                "open",
            ));
            for week in &self.weeks {
                out.push_str(&week_row(
                    &week.iso,
                    &week.created.to_string(),
                    &week.closed.to_string(),
                    &week.net.to_string(),
                    &week.open.to_string(),
                ));
            }
        }
        out
    }

    /// The ETA line: the projected date, or the honest "insufficient
    /// data" wording with the all-time fallback hint.
    fn eta_text(&self) -> String {
        let hint = self.rate_all_time.map_or_else(String::new, |rate| {
            format!("; all-time {rate:.1}/wk — try a wider --window")
        });
        self.eta.as_ref().map_or_else(
            || match self.eta_reason.as_deref() {
                Some("zero_closures_in_window") => format!(
                    "insufficient data ({} closures in {}d window{})",
                    self.closures_in_window, self.window_days, hint
                ),
                _ => format!(
                    "insufficient data ({} < {} closures in {}d window{})",
                    self.closures_in_window, self.min_closures, self.window_days, hint
                ),
            },
            String::clone,
        )
    }
}

/// The median of a sorted non-empty-when-used slice: the middle value,
/// or the mean of the two middle values.
fn median(sorted: &[f64]) -> Option<f64> {
    let mid = sorted.len() / 2;
    if sorted.is_empty() {
        None
    } else if sorted.len().is_multiple_of(2) {
        Some(sorted[mid - 1].midpoint(sorted[mid]))
    } else {
        Some(sorted[mid])
    }
}

/// One padded table row (the `stats` column layout).
fn week_row(name: &str, created: &str, closed: &str, net: &str, open: &str) -> String {
    format!("{name:<16}{created:>8}{closed:>8}{net:>6}{open:>7}\n")
}

/// Rounds to one decimal for stable, readable metric output.
fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Buckets creations and closures into ISO weeks from the first event
/// up to the week of `now_days`, carrying the running open count;
/// `since_days` truncates the displayed rows (the count still runs
/// through the truncated weeks).
fn weekly_series(
    creations: &[i64],
    closures: &[i64],
    now_days: i64,
    since_days: Option<i64>,
) -> Vec<WeekBucket> {
    let bucket_of = |events: &[i64]| -> BTreeMap<i64, usize> {
        let mut buckets: BTreeMap<i64, usize> = BTreeMap::new();
        for millis in events {
            let week = timeutil::monday_of_week(millis.div_euclid(MILLIS_PER_DAY));
            *buckets.entry(week).or_default() += 1;
        }
        buckets
    };
    let created_by_week = bucket_of(creations);
    let closed_by_week = bucket_of(closures);
    let Some(first) = creations.iter().chain(closures.iter()).min() else {
        return Vec::new();
    };
    let mut week = timeutil::monday_of_week(first.div_euclid(MILLIS_PER_DAY));
    let last = timeutil::monday_of_week(now_days);
    let mut open: i64 = 0;
    let mut series = Vec::new();
    while week <= last {
        let created = created_by_week.get(&week).copied().unwrap_or(0);
        let closed = closed_by_week.get(&week).copied().unwrap_or(0);
        open += i64::try_from(created).unwrap_or(0) - i64::try_from(closed).unwrap_or(0);
        open = open.max(0);
        let visible = since_days.is_none_or(|since| week + 6 >= since);
        if visible {
            series.push(WeekBucket {
                iso: timeutil::iso_week_label(week),
                created,
                closed,
                net: i64::try_from(closed).unwrap_or(0) - i64::try_from(created).unwrap_or(0),
                open: usize::try_from(open).unwrap_or(0),
            });
        }
        week += 7;
    }
    series
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Fixed clock for every pace test: 2026-02-01T00:00:00Z.
    const NOW: &str = "2026-02-01T00:00:00.000Z";

    fn seed(_id: &str, fields: Value) -> SeedRecord {
        let Value::Object(map) = fields else {
            panic!("test record must be an object");
        };
        SeedRecord::try_from_fields(map).expect("valid test record")
    }

    /// a/b/c close inside the 30d window, d closed before it, e/f stay
    /// open (f parked on `backlog`).
    fn fixture() -> Vec<SeedRecord> {
        let made = |id: &str, created: &str, closed: Option<&str>, labels: Vec<&str>| {
            let mut fields = json!({
                "id": id,
                "title": format!("seed {id}"),
                "createdAt": format!("{created}T00:00:00.000Z"),
            });
            let Value::Object(map) = &mut fields else {
                unreachable!("fields is an object");
            };
            match closed {
                Some(day) => {
                    map.insert("status".to_owned(), json!("closed"));
                    map.insert("closedAt".to_owned(), json!(format!("{day}T00:00:00.000Z")));
                }
                None => {
                    map.insert("status".to_owned(), json!("open"));
                }
            }
            if !labels.is_empty() {
                map.insert("labels".to_owned(), json!(labels));
            }
            seed(id, fields)
        };
        vec![
            made("tst-0001", "2026-01-05", Some("2026-01-06"), vec![]),
            made("tst-0002", "2026-01-10", Some("2026-01-12"), vec![]),
            made("tst-0003", "2026-01-20", Some("2026-01-25"), vec![]),
            made("tst-0004", "2025-12-20", Some("2025-12-30"), vec![]),
            made("tst-0005", "2026-01-15", None, vec![]),
            made("tst-0006", "2026-01-16", None, vec!["backlog"]),
        ]
    }

    fn pace_with(options: &PaceOptions) -> Pace {
        Pace::collect(&fixture(), options).expect("fixture options are valid")
    }

    #[test]
    fn windowed_rate_leads_and_parks_the_excluded_open_seed() {
        let pace = pace_with(&PaceOptions {
            now: Some(NOW.to_owned()),
            exclusion_labels: vec!["backlog".to_owned()],
            ..PaceOptions::default()
        });
        // 3 closures in (2026-01-02, 2026-02-01]; 3 * 7 / 30 = 0.7.
        assert_eq!(pace.closures_in_window, 3);
        assert!((pace.rate_window - 0.7).abs() < 1e-9);
        // Lead times: 24h, 48h, 120h, 240h → median (48 + 120) / 2.
        assert_eq!(pace.lead_time_median_hours, Some(84.0));
        // e counts toward the target, f is parked by the exclusion.
        assert_eq!(pace.open_to_target, 1);
        assert_eq!(pace.excluded_open, 1);
        assert_eq!(pace.closures_total, 4);
        let rate_all = pace.rate_all_time.expect("history exists");
        let weeks = pace.history_weeks.expect("history exists");
        assert!((rate_all - 4.0 / weeks).abs() < 1e-9);
    }

    #[test]
    fn window_below_the_minimum_reports_insufficient_data() {
        let pace = pace_with(&PaceOptions {
            now: Some(NOW.to_owned()),
            min_closures: 5,
            ..PaceOptions::default()
        });
        assert_eq!(pace.eta, None);
        assert_eq!(
            pace.eta_reason.as_deref(),
            Some("insufficient_closures_in_window")
        );
    }

    #[test]
    fn sufficient_window_projects_the_target_date() {
        let pace = pace_with(&PaceOptions {
            now: Some(NOW.to_owned()),
            min_closures: 3,
            ..PaceOptions::default()
        });
        // rate 0.7/wk, 2 open seeds → ceil(2 * 30 / 3) = 20 days.
        assert_eq!(pace.eta.as_deref(), Some("2026-02-21"));
        assert_eq!(pace.eta_reason, None);
    }

    #[test]
    fn a_drained_target_is_ready_today() {
        let mut parked = fixture();
        // Park the one remaining unexcluded open seed too.
        parked.retain(|record| {
            record.status() == Some(Status::Closed) || record.labels().contains(&"backlog")
        });
        let pace = Pace::collect(
            &parked,
            &PaceOptions {
                now: Some(NOW.to_owned()),
                exclusion_labels: vec!["backlog".to_owned()],
                ..PaceOptions::default()
            },
        )
        .expect("options valid");
        assert_eq!(pace.open_to_target, 0);
        assert_eq!(pace.eta.as_deref(), Some("2026-02-01"));
        assert_eq!(pace.eta_reason, None);
    }

    #[test]
    fn zero_window_is_rejected() {
        let error = Pace::collect(
            &fixture(),
            &PaceOptions {
                window_days: 0,
                now: Some(NOW.to_owned()),
                ..PaceOptions::default()
            },
        )
        .expect_err("window must be at least one day");
        assert!(error.contains("--window"), "names the flag: {error}");
    }

    #[test]
    fn malformed_since_is_rejected() {
        let error = Pace::collect(
            &fixture(),
            &PaceOptions {
                now: Some(NOW.to_owned()),
                since: Some("someday".to_owned()),
                ..PaceOptions::default()
            },
        )
        .expect_err("since must parse");
        assert!(error.contains("--since"), "names the flag: {error}");
    }

    #[test]
    fn weeks_run_monday_to_monday_with_a_running_open_count() {
        let pace = pace_with(&PaceOptions {
            now: Some(NOW.to_owned()),
            ..PaceOptions::default()
        });
        let iso: Vec<&str> = pace.weeks.iter().map(|week| week.iso.as_str()).collect();
        // First event 2025-12-20 (Saturday) → week of 2025-12-15.
        assert_eq!(iso.first(), Some(&"2025-W51"));
        // Now 2026-02-01 (Sunday) → week of 2026-01-26.
        assert_eq!(iso.last(), Some(&"2026-W05"));
        assert_eq!(iso.len(), 7);
        let last = pace.weeks.last().expect("series non-empty");
        // 6 created, 4 closed over the series.
        assert_eq!(last.open, 2);
        let first = pace.weeks.first().expect("series non-empty");
        assert_eq!(first.created, 1, "2025-12-20 lands in 2025-W51");
    }

    #[test]
    fn since_truncates_rows_but_not_the_running_count() {
        let pace = pace_with(&PaceOptions {
            now: Some(NOW.to_owned()),
            since: Some("2026-01-15".to_owned()),
            ..PaceOptions::default()
        });
        let iso: Vec<&str> = pace.weeks.iter().map(|week| week.iso.as_str()).collect();
        // Week of 2026-01-12 (Monday) contains 2026-01-15 → kept.
        assert_eq!(iso.first(), Some(&"2026-W03"));
        let last = pace.weeks.last().expect("series non-empty");
        assert_eq!(last.open, 2, "running count survives truncation");
    }
}
