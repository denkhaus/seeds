# ADR-0002: Pace metrics — uniform close counting from snapshots, no event log

- Status: Accepted
- Date: 2026-10-12
- Deciders: user (grilling session, three rounds confirmed), agent (seeds-8fc5)
- Related: seeds-8fc5, `GLOSSARY.md` (Pace, Lead Time, Production Readiness, ETA), README format-compatibility contract

## Context

`sd stats --pace` (seeds-8fc5) reports closure rates, median lead time,
and an ETA toward production readiness. The store carries only
snapshots — `createdAt`, `closedAt`, `status` — and `closeReason` is
free prose, not an enum. Two questions had to be settled before any
number could mean anything: which closures count, and where the
time-series comes from.

## Decision

1. **Every closure counts uniformly.** A seed closed as `superseded`,
   `stale`, or any other prose reason drains the backlog like any
   other close; its successor is its own seed and must be closed in
   turn. Rationale: the ETA identity `open ÷ rate` only holds when
   numerator and denominator share one scope — excluding closes from
   the rate while they still reduce the open count would make the
   projection inconsistent. "Closed" means "out of the backlog", full
   stop.
2. **Snapshot reconstruction, no event log.** The weekly series and
   both rates are derived from `createdAt`/`closedAt` at a clock the
   caller injects (`PaceOptions::now`). The format gains no new field
   and no `.seeds/events.jsonl` — the compat contract (ADR-0023 in
   denkhaus/fabro) stays untouched.
3. **An honest null ETA.** The windowed rate is authoritative only
   with at least `--min-closures` (default 5) closures inside
   `--window` (default 30 days); below that the JSON reports
   `eta: null` with an `eta_reason` and the markdown prints
   "insufficient data" plus the all-time hint, rather than a number
   that swings wildly week to week.
4. **[Amended 2026-10-21, design v2] The ETA projects net drain, not
   gross throughput.** The v1 identity `open ÷ rate_window` silently
   assumed zero future creations — optimistic exactly when the backlog
   grows (the failure mode fabro's friction-score drain-deficit
   measures). v2 reports the **pace coefficient**
   `k = closures_in_window / creations_in_window` (null with
   `no_creations_in_window` when the window saw no creations) and the
   **Drain Rate** `(closures − creations) × 7 / window_days`, and the
   ETA becomes `open_to_target ÷ Drain Rate` — `eta: null` with
   `eta_reason: "backlog_not_shrinking"` while the drain is not
   positive. Uniform close counting (decision 1) still underwrites
   the identity: numerator and denominator keep one scope. The gross
   windowed rate stays visible as `Rate (30d)`; there is exactly one
   ETA. Also added: median open age of non-closed seeds (`Open age
   (median)`), measurement only — no verdict bands in the tracker;
   bands are loop policy (fabro architect), not tracker policy.

## Considered Options

- **Structured `closeKind` field** (done vs. discarded closes): the
  cleanest semantics, but a format extension the compat contract makes
  hard to reverse, and the distinction buys lead-time insight, not
  pace. Deferred to a future diff with its own ADR if wanted.
- **`closeReason` prefix heuristics** (`superseded`/`stale`/…): broken
  by construction — the reason is unstructured prose that varies per
  writer, and a silent misclassification would corrupt both metrics.
- **Ereignis-Log (`.seeds/events.jsonl`)**: would enable true cycle
  time and reopen history, at the cost of a second writer contract and
  migration for every existing store. Current needs (lead time,
  backlog drain) are fully served by the snapshots.

## Consequences

- Reopened seeds show only their latest close (`closedAt` is reset on
  reopen and dropped on open); historical reopen churn is invisible to
  the metrics — accepted, documented here.
- The week table's `open` column is the raw series (all open seeds);
  `open_to_target` applies the `--exclude-label` list. The two can
  differ, deliberately.
- Changing the counting rule later rewrites the meaning of every
  previously emitted report — treat it as a breaking semantic change.