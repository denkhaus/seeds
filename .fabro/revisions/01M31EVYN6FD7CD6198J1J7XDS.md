# Revision — run 01M31EVYN6FD7CD6198J1J7XDS

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M31EVYN6FD7CD6198J1J7XDS.md
- seeds filed: none — zero balance credit this pass (no same-pass stale/superseded closes); all surviving findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M31EVYN6FD7CD6198J1J7XDS, workflow version e3c92fec9a9a6fd9256277d60e06e12dddcacf6779280c9df5c2b2825b813ea8, commit 2c5dd2748a972ee7c9f4ca3cde4be1b7ab3761bf
- revised_at_commit: 2c5dd2748a972ee7c9f4ca3cde4be1b7ab3761bf (ADR-0015: engine drift signal for later judgement)

## Findings

- Fix dup-run-check default base to origin/main — `.fabro/scripts/dup-run-check.nu` line 209: default `--base` from `origin/denkhaus` to `origin/main`, plus stale usage comment at line 9; effect: implementer/revisor preflights get a real clean/duplicate verdict instead of a guaranteed degraded no-op (reproduced: bare invocation degraded with "couldn't find remote ref denkhaus"; explicit `--base origin/main` returned clean). Not filed — overflow-dup: Fix dup-run-check ref derivation to origin refs/heads/main (open in 01M2ZYVQ7CBSY3T0FEYBE9JK74.md)
- Bound planner in-flight fabro_runs_list with created_since=<now-48h> — `.fabro/workflows/develop/prompts/planner.md` step 4 (IN-FLIGHT EXCLUSION): specify `fabro_runs_list workflow="develop" created_since=<now-48h>` then post-filter to non-terminal statuses and open PRs; effect: ~-40k planner input tokens (≈$0.05) and -10–15 s wall per run (bare call inlined 134 runs in 15.86 s; planner was 47% of run cost). No open seed covers planner token/wall efficiency (seeds-facc is CLI parity, seeds-9fa3 reviewer-scoped). Not filed — overflow below.
- Unblock mandatory lesson capture when `.mulch/` is absent — amend 'Lesson capture' in `.fabro/workflows/develop/prompts/implementer.md`: on `ml record`/`ml prime` failing with 'No .mulch/ directory found', answer `nothing durable — skipped (mulch absent)` and journal once; or commit a `.mulch/` via `mulch init`; effect: removes 2 failed calls + ~2 recovery rounds (~20–30 s) per run (2 of 3 implementer shell errors this run were exactly this). Not filed — overflow-dup: Init mulch or drop the ml mandate in AGENTS.md (open in 01M2ZYVQ7CBSY3T0FEYBE9JK74.md)
- Gate self-test: fixture battery for touched-crate derivation — add a checked-in fixture battery (pattern: `.fabro/scripts/dup-run-check-fixtures.nu`) feeding synthetic path-lists through the crates/<name> derivation, asserting `["seeds"]` and the empty-set→exit-1 guard, wired into `check-loop-assets` in `scripts/qualitygate.nu`; optionally extract shared regex/guard into one sourced `.nu` module imported by both `scripts/qualitygate.nu` and `scripts/verify.nu`; effect: gate proves its own touched-set logic every run (including scripts-only diffs where it printed 'no crates touched' and changed code never executed) and prevents the two hand-synced copies drifting (failure class behind seeds-9482, which shipped the fix but no self-test). Not filed — overflow below.

## Overflow (ADR-0022 — survived dedupe, no balance credit this pass)

- overflow: Bound planner in-flight fabro_runs_list with created_since=<now-48h> — `.fabro/workflows/develop/prompts/planner.md` step 4: specify `fabro_runs_list workflow="develop" created_since=<now-48h>` then post-filter to non-terminal statuses and open PRs; effect: ~-40k planner input tokens (≈$0.05) and -10–15 s wall per run.
- overflow: Gate self-test fixture battery for touched-crate derivation — add checked-in fixture battery feeding synthetic path-lists through the crates/<name> derivation (assert `["seeds"]` + empty-set→exit-1 guard) wired into `check-loop-assets` in `scripts/qualitygate.nu`; effect: gate self-proves touched-set logic every run and stops hand-synced copies drifting (class behind seeds-9482).
