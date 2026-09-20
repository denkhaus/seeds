# Improve review — run 01M2ZYVQ7CBSY3T0FEYBE9JK74

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (5.9 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-20 18:44+0000 by revisor `fabro_ask`

---

All evidence below is from this run's events/checkpoints (stage timings, journal painpoints, gate output) and the workspace files they name. Run shape for scale: 5m35s wall, $0.424 total (planner $0.160 / implementer $0.127 / reviewer $0.137); the deterministic nodes cost 229ms–6.3s each — 92% of wall was the three LLM stages. Tracker check: only `seeds-facc` and `seeds-3791` are open; `seeds-e218` was closed by this run. Neither open seed covers loop-machinery fixes, so most recommendations carry a new-seed justification.

## Recommendations, by expected impact

**1. Fix touched-crate detection — the quality gate was a no-op for a 1,735-line Rust diff.**
- What happened: tester ran 6.3s and printed `no crates touched … GATE GREEN` (from run events, tester output) even though the seed landed ~1,700 lines in `crates/`. The reviewer journaled that "crate-greenness rests solely on the implementer's spec-mandated nextest run." Root cause confirmed in workspace: `scripts/qualitygate.nu:51–58` and `scripts/verify.nu:73–93` filter paths on `lib/(apps|components|foundation)/` — the fabro repo's layout, not this repo's `crates/`. The implementer also hit the sibling: `just verify implementer` answered "no lib/ crates touched — nothing to verify."
- Change: derive touched crates from the workspace manifest (`Cargo.toml` `members`) in both scripts, and make `qualitygate.nu` **fail loudly** when the diff touches `*.rs` but the touched-crate set is empty.
- Expected effect: the deterministic gate actually compiles/tests every Rust seed; silent no-op gates become impossible.
- New-seed justification: no existing seed covers gate/verify internals — `seeds-facc` is CLI parity of the `seeds` binary, `seeds-3791` is the sd cutover.

**2. Make `real_fixture.rs` hermetic before fixing #1, or the next run goes gate-red.**
- What happened: the implementer had to regenerate `crates/seeds/tests/fixtures/repo_issues_sd_list.json` mid-pass because the planner's 17:48 claim bumped `updatedAt` after the 17:02 capture (from `implementation_summary` + implementer journal). Then closeout closed the seed *after* the tester — the committed snapshot still says `in_progress`, the live tracker says `closed` (visible in the closeout diff). The implementer's journal flags exactly this race.
- Change: in `crates/seeds/tests/real_fixture.rs`, copy the repo tracker to a temp dir (and regenerate/compare there) instead of reading the live `.seeds/issues.jsonl`.
- Expected effect: removes a guaranteed red on the next run's suite once recommendation #1 makes the gate see `crates/` — the two must land together.
- New-seed justification: `seeds-e218` is closed and scoped to the reader/writer itself; test hermeticity is QA work no open seed covers.

**3. Merge PR #1 (or fix the auto-merge setting) — main is missing the foundation the next seed builds on.**
- What happened: from the run summary, `auto_merge` failed with `Auto merge is not allowed for this repository`; PR #1 is open and `seeds-e218` is closed. `seeds-facc` (CLI parity *over the format core*) is now unblocked and will be the next claim — but its implementer starts from a `main` that lacks `crates/seeds/src/*` entirely.
- Change: user merges PR #1 now; going forward either enable auto-merge on the repo or set `pull_request.auto_merge=false` in the run settings to stop the per-run failed step.
- Expected effect: the next run builds on landed code instead of re-deriving or cherry-picking; no recurring auto-merge error in every run's tail.

**4. `planner-preflight.nu` in-flight arm: exclude engine-terminal runs.**
- What happened: the preflight table marked `seeds-e218` `in_flight: true` via run `01M2ZW7TT6KK…` — a **failed** run (terminal, no PR), because its branch tip subject is `implementer (succeeded)` and `terminal-tip?` only proves terminality from closeout/failed tips (workspace: `.fabro/workflows/develop/scripts/planner-preflight.nu:96–116`). The planner burned a 15.6s `fabro_runs_list` call plus extra reasoning to override it; a lazier planner would have skipped the only ready seed and routed Tracker empty.
- Change: extend `terminal-tip?` to also treat a run whose own stage-journal records engine failure (or whose last journal node is terminal) as terminal — data fix in the script, not new prompt prose (per the fabro-9ec3 standing policy already cited in the planner prompt).
- Expected effect: resume-after-failure seeds are claimable on the first pass; saves ~16s of tool time and one LLM round per planner lap.
- New-seed justification: no open seed covers preflight arms; `seeds-facc`'s "tracker scripts still work" clause is about pointing them at the new binary, not their logic.

**5. Init mulch (or drop the `ml` mandate) — lesson capture is dead in this repo.**
- What happened: implementer journal + `lesson_capture` key: `ml prime`/`ml record` fail with "No .mulch/ directory found" while `AGENTS.md:44–45` mandates both; this run's durable lesson (live-tracker fixture staleness) survived only as journal text, and the implementer wasted calls attempting `ml`.
- Change: commit a `mulch init` result (`.mulch/`), or strike the `ml prime`/`ml record` lines from `AGENTS.md` and the implementer prompt's lesson-capture section.
- Expected effect: `lesson_capture` returns real mx-ids again; no per-pass wasted `ml` attempts. (Checked `seeds-3791` — it only says mulch "stays until the mulch phase," so no seed covers initializing it.)

**6. Fix `dup-run-check.nu`'s fetch ref derivation — the implementer's duplicate-run guard gave no signal.**
- What happened: implementer journal: verdict `degraded`, reason `fetch failed: fatal: couldn't find remote ref denkhaus` — the script assumes a remote/ref shape this origin doesn't serve (file: `.fabro/scripts/dup-run-check.nu`).
- Change: derive the merge-target ref from the configured remote (`origin` + `refs/heads/main`) instead of parsing the owner out of the origin URL.
- Expected effect: the claim-race stopgap actually answers clean/duplicate in this repo instead of always degrading.
- New-seed justification: no existing seed covers dup-run-check internals.

**7. Shrink the evidence capture (or raise the reviewer's inline budget) — 32% of run cost was the reviewer paging a blob.**
- What happened: the evidence capture was 66.9KB — above the 48KB graph budget — so it blob-ref'd; the reviewer made 9 `read_file` calls to page it, burning 49.4k input tokens, 84.6s inference, $0.137 (from stage usage). Much of the bulk is machine-generated: `Cargo.lock` (+170 lines) and the 59-line fixture JSON that duplicates tracker content.
- Change: in `.fabro/workflows/develop/scripts/evidence.nu`, collapse generated files (lockfiles, fixture JSON) to stat-only lines with a "regenerable" marker; alternatively raise the reviewer node's `preamble_inline_max_kb` (currently 16) above the typical capture size.
- Expected effect: the capture renders inline, the reviewer drops the blob-detour tool rounds, cutting roughly a quarter to a third of review cost per run.
- New-seed justification: no open seed covers the evidence pipe.

**8. Resolve rescue branches in the preflight — the planner wasted a round discovering `FETCH_HEAD`.**
- What happened: run events seq 63→68: the planner's first `git log origin/fabro/run/01M2ZW7TT6KK…` failed (`unknown revision`), because the shallow clone only materializes other runs' branches as `FETCH_HEAD` after an explicit fetch; it journaled this as a painpoint.
- Change: extend `planner-preflight.nu` (not prompt prose) to fetch and resolve any run-branch refs cited in the top candidate's body, emitting the resolved sha in the verdict table.
- Expected effect: one fewer planner LLM round per resume seed, and no reliance on the model rediscovering the fetch idiom.
- New-seed justification: no existing seed covers preflight branch resolution.

One note on ordering: #1 and #2 are coupled — fixing crate detection without making the fixture test hermetic converts today's silent gate into tomorrow's guaranteed red, so ship them as one change. I could not inspect the engine-side auto-merge configuration (outside this run's scope); the fix there is a repo/settings decision, not a workspace file.
