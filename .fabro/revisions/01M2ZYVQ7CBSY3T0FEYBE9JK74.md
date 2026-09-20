# Revision — run 01M2ZYVQ7CBSY3T0FEYBE9JK74

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M2ZYVQ7CBSY3T0FEYBE9JK74.md
- seeds filed: none — zero balance credit this pass, all findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M2ZYVQ7CBSY3T0FEYBE9JK74, workflow version e3c92fec9a9a6fd9256277d60e06e12dddcacf6779280c9df5c2b2825b813ea8, commit d111245426ac54e475cd18c092f7ec81aac78c39
- revised_at_commit: d111245426ac54e475cd18c092f7ec81aac78c39 (ADR-0015: engine drift signal for later judgement)

## Findings

All seven findings survived dedupe (open seeds cover only CLI parity and sd cutover; nothing targets gate/verify, fixtures, preflight, mulch, dup-run-check, or evidence internals) but the pass has zero same-pass close credit, so each is journaled below as a machine-visible overflow entry for the next pass to file.

- overflow: Fix touched-crate detection in qualitygate/verify scripts — derive touched crates from workspace `Cargo.toml` members in `scripts/qualitygate.nu` and `scripts/verify.nu` (currently filter on `lib/(apps|components|foundation)/` so `crates/` diffs print "GATE GREEN" and `just verify implementer` no-ops) and fail loudly when the diff touches `*.rs` but the crate set is empty; effect: the deterministic gate actually builds/tests every Rust seed and silent no-op gates become impossible. Land coupled with the real_fixture hermeticity fix below.
- overflow: Make real_fixture.rs hermetic via temp copy of the repo tracker — `crates/seeds/tests/real_fixture.rs` reads the live `.seeds/issues.jsonl`; copy the tracker to a temp dir and regenerate/compare fixtures there; effect: removes a guaranteed red once gate detection sees `crates/` (planner claims bump `updatedAt` after fixture capture).
- overflow: Exclude engine-terminal runs in planner-preflight in-flight arm — extend `terminal-tip?` in `.fabro/workflows/develop/scripts/planner-preflight.nu` (lines 96-116) to treat a run whose stage journal records engine failure as terminal; effect: resume-after-failure seeds are claimable on first pass, saving one LLM round per planner lap (run 01M2ZW7TT6KK... cost 15.6s `fabro_runs_list` plus override reasoning).
- overflow: Resolve rescue-branch refs in planner-preflight — extend `planner-preflight.nu` (data fix) to fetch and resolve run-branch refs cited in the top candidate's body and emit the resolved sha in the verdict table; effect: one fewer planner LLM round per resume seed, no reliance on the model rediscovering the fetch idiom (shallow clone materializes other runs' branches only as `FETCH_HEAD`).
- overflow: Init mulch or drop the ml mandate in AGENTS.md — commit a `mulch init` result (`.mulch/`), or strike the `ml` lines from `AGENTS.md` and the implementer prompt's lesson-capture section; effect: `lesson_capture` returns real mx-ids and no per-pass wasted `ml` attempts (currently fails with "No .mulch/ directory found").
- overflow: Fix dup-run-check ref derivation to origin refs/heads/main — `.fabro/scripts/dup-run-check.nu` parses the owner out of the origin URL and fails with "fatal: couldn't find remote ref denkhaus"; derive the merge-target ref from the configured remote (`origin` + `refs/heads/main`) instead; effect: the claim-race guard actually answers clean/duplicate instead of always degrading.
- overflow: Collapse generated files in the evidence capture — in `.fabro/workflows/develop/scripts/evidence.nu`, collapse regenerable files (lockfiles, fixture JSON) to stat-only lines with a 'regenerable' marker; effect: the capture renders inline under the 48KB budget and the reviewer drops the blob-detour rounds (9 read_file calls, 49.4k tokens, $0.137 = 32% of run cost this pass's target).
