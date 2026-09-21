# Revision — run 01M324AZVMHDSCSAMVP4WTZR1R

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M324AZVMHDSCSAMVP4WTZR1R.md
- seeds filed: none — healthy run; all findings already open as overflow or journaled below (no filing credit this pass)
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M324AZVMHDSCSAMVP4WTZR1R, workflow version e3c92fec9a9a6fd9256277d60e06e12dddcacf6779280c9df5c2b2825b813ea8, commit f3372706b722e8b7d3bcfc4abfcb76a291ed74d6
- revised_at_commit: f3372706b722e8b7d3bcfc4abfcb76a291ed74d6 (ADR-0015: engine drift signal for later judgement)

## Findings

### 1. verify.nu: see untracked files in touched-crate derivation and touch test files before nextest
- overflow-dup: verify.nu: derive touched paths from git status --porcelain so untracked files count (open in 01M3201Q8GTC0EPT0S5YMK0Z32.md)
- Concrete change: in `scripts/verify.nu` union `git diff --name-only $base` with `git ls-files --others --exclude-standard` so untracked test files (this run: `crates/seeds/tests/dedupe.rs`) classify their crate as test-file-touched, and touch derived test files before the nextest invocation to avoid stale-binary re-runs.
- Expected effect: new suites run in-lane mechanically (implementer had to manually run `cargo nextest run -p seeds` 57/57 this run). The touch-before-nextest arm extends the open overflow.

### 2. Bound planner fabro_runs_list with created_since in planner.md step 4
- overflow-dup: Bound planner in-flight fabro_runs_list with created_since=<now-48h> (open in 01M31EVYN6FD7CD6198J1J7XDS.md)
- Concrete change: pass `created_since` to the in-flight `fabro_runs_list` check in `.fabro/workflows/develop/prompts/planner.md` step 4; expected effect: planner context roughly halves (this run: 137 runs listed, tokens 7,557→50,173). New evidence (7d beyond tracker_guard's 6h) strengthens the already-open entry.

### 3. Scaffold .mulch/expertise/ for new domains and preserve mulch.config.yaml comments
- overflow-dup: Init mulch or drop the ml mandate in AGENTS.md (open in 01M2ZYVQ7CBSY3T0FEYBE9JK74.md)
- Concrete change: scaffold `.mulch/expertise/` on first record plus upstream mulch CLI fixes (create dir in addDomain, comment-preserving serializer); expected effect: removes first-lesson ENOENT crash and stops the YAML serializer stripping ~80 lines of config docs.

### 4. prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed
- overflow: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed — in `.fabro/scripts/prompt-lint.nu`, treat schemas referenced by a node's output_schema attribute as intentionally routing-shaped (suppress/downgrade the 10 warnings against `planner-output.schema.json` and `conductor/schemas/develop-output.schema.json`, deliberate per fabro-9ec3 arm 3) and add a reviewed-marker for the load-bearing 2026-04-14 date-pin warning; effect: qualitygate output carries only actionable warnings (this run's tester section printed 11 expected warns).
