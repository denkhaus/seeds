# Improve review — run 01M31EVYN6FD7CD6198J1J7XDS

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (5.2 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 07:54+0000 by revisor `fabro_ask`

---

Evidence base: run events (stage timings, tool calls, failures), the run's journals/implementer summary, and workspace files (`.seeds/issues.jsonl`, `.fabro/scripts/dup-run-check.nu`, `scripts/qualitygate.nu`, `scripts/verify.nu`, develop prompts). Run shape for scale: 301 s wall, $0.269 total — planner 55.9 s / $0.128 (47%), implementer 196.4 s / $0.122 (45%), reviewer 23.1 s / $0.020, tester 5.6 s; 12 implementer shell calls, 3 errors; all stages first-pass green.

I checked all 8 tracker records (`.seeds/issues.jsonl`); open seeds are `seeds-facc`, `seeds-3791`, `seeds-48db`, `seeds-b291`, `seeds-9fa3`. None of these covers recommendations 1–4 below, so each carries a new-seed justification.

## Recommendations, by expected impact

**1. Fix the dup-run-check default base — the claim-race guard is deterministically blind in this repo.**
What happened (from run events seq 102 + implementer journal): the implementer's mandatory preflight `nu .fabro/scripts/dup-run-check.nu seeds-9482 --self ...` returned `verdict: degraded, "fetch failed — fatal: couldn't find remote ref denkhaus"`. Root cause (from workspace file `.fabro/scripts/dup-run-check.nu:209`): `def main [...ids: string, --base: string = "origin/denkhaus", ...]` — a stale port default; PROJECT_FACTS says the merge target is `origin/main`, and line 217 then runs `git fetch origin denkhaus`. The implementer prompt even claims "the script checks the merge-target branch named by PROJECT_FACTS … by default" — false today. The preflight node works only because `planner-preflight.nu` passes its own base.
Change: one line in `.fabro/scripts/dup-run-check.nu` — default `--base` to `origin/main` (and fix the stale usage comment at line 9).
Expected effect: every implementer pass gets a real clean/duplicate verdict from the ~1 s stopgap for the claim-race family (fabro-6b58/fabro-9372) instead of a guaranteed degraded no-op.
New-seed justification: this run's journal painpoint was never filed as a seed (closeout's deferred-action sweep only files `deferred-action:` markers, and this was a painpoint); no open seed names dup-run-check's base.

**2. Bound the planner's `fabro_runs_list` call — one tool result cost ~16 s and ~40k tokens (≈15% of run cost).**
What happened (from run events seq 45–46, 52): the planner's in-flight check called `fabro_runs_list {"workflow":"develop"}` bare; it took 15.86 s and inlined 134 runs of JSON. The next LLM round jumped from 11.2k to 54.0k input tokens (conversation category 48.9k) and cost $0.063 — half the planner's total $0.128. The planner only needs non-terminal runs and open-PR runs; runs from 2026-09-20 11:32 onward were dead weight.
Change: in `.fabro/workflows/develop/prompts/planner.md` step 4 (IN-FLIGHT EXCLUSION), specify the call as `fabro_runs_list workflow="develop" created_since=<now-48h>` (the tool documents `created_since`), then post-filter to non-terminal statuses and open PRs.
Expected effect: on this run's shape, roughly –40k planner input tokens (≈$0.05) and –10–15 s wall per run; the planner drops from 47% of run cost toward ~30%.
New-seed justification: no open seed covers planner token/wall efficiency; `seeds-9fa3` is reviewer-scoped, the others are product-scoped.

**3. Resolve the lesson-capture deadlock — 2 of the implementer's 3 shell errors were `.mulch/` absence.**
What happened (from run events seq 159, 166, 173 + implementer journal): `ml record` and `ml prime` both died with "No .mulch/ directory found. Run `mulch init`" (exit 1); the implementer then spent two extra rounds `ls`-ing `/workspace/seeds` and `/repos/denkhaus/seeds` to prove the directory doesn't exist, then answered "nothing durable — skipped". The implementer prompt makes the mx-id-or-skip answer mandatory every pass, so this friction repeats on every run until fixed.
Change: either commit a `.mulch/` (one `mulch init`) — or, cheaper and loop-local, amend the "Lesson capture" section of `.fabro/workflows/develop/prompts/implementer.md`: "if `ml record` fails with 'No .mulch/ directory', answer `nothing durable — skipped (mulch absent)` and journal it once — do not investigate further."
Expected effect: removes 2 failed calls + ~2 recovery rounds (~20–30 s and a slice of the implementer's 193 s inference) per run, and stops normalizing a broken required step.
New-seed justification: `seeds-3791` explicitly defers mulch ("mulch CLI stays until the mulch phase") and covers sd→seeds cutover, not the missing `.mulch/` store or the prompt's dead requirement.

**4. Give the gate a fixtures self-test for touched-crate derivation — this run's gate never executed the code seeds-9482 changed.**
What happened (from run events): the tester (5.6 s) printed "no crates touched" on this scripts-only diff — the new `crates/<name>/**` regex and loud-failure guard ran zero times under the gate; the ONLY verification was the implementer hand-rolling scratch files, which hit the false-negative trap (`git diff --name-only` ignores untracked files until `git add -N` — journaled observation) and a `git rm --intent-to-add` misuse (exit 129, seq 138 → recovery round seq 145). Meanwhile the same derive regex + guard now exists as two hand-synced copies (`scripts/qualitygate.nu` `touched-crates`, `scripts/verify.nu` `touched`) — and copy drift is precisely the failure class that produced seeds-9482.
Change: add a checked-in fixture battery (pattern already exists: `.fabro/scripts/dup-run-check-fixtures.nu`) that feeds synthetic path-lists through the derivation and asserts `["seeds"]` and the empty-set→exit-1 guard, wired into `check-loop-assets` in `scripts/qualitygate.nu`; optionally extract the shared regex/guard into one sourced `.nu` module both scripts import.
Expected effect: the gate deterministically proves its own touched-set logic every run (including scripts-only runs like this one), eliminating the scratch-file verification lane and its error class, and preventing the two copies from drifting again.
New-seed justification: seeds-9482 closed with the fix but scoped no self-test battery; no open seed covers gate self-testing.

**5. Land `seeds-9fa3` before the next Rust-heavy seed — this run's reviewer approved with zero tool calls.**
What happened (from run events, reviewer@1): the reviewer made 0 tool calls (tool_time 0 ms) and approved in 22.9 s purely from the inline evidence — legitimate here because the diff was two `.nu` scripts and the brief said "rust-style-guide pages not applicable". But the same reviewer node mechanically cannot read `.fabro/skills/rust-style-guide/SKILL.md` (no `use_skill`, fs_hide-bound), so the first Rust diff will be judged guide-blind exactly as in run 01M30FZ05QPQ.
Change: implement existing seed **seeds-9fa3** (user-approved 2026-09-21; direction (a) add `use_skill` to the reviewer tools allow-list in `.fabro/workflows/develop/workflow.fabro`, or (b) exempt `.fabro/skills/**` from fs_hide).
Expected effect: the Rust standards axis becomes mechanically auditable before `seeds-facc` (CLI parity, now unblocked by this run's close and the likely next claim) ships Rust diffs through review.

**6. Assign `seeds-48db` or PR #1 stays merge-blocked forever (user-experience/ops).**
What happened (from this run's tracker diffs in run events): closing seeds-9482 unblocked only `seeds-facc`. `seeds-48db` (real_fixture volatile-field drift, the CI-red blocker on PR #1) is open and carries **no assignee** — under the loop's fail-closed ownership rule the develop line will never pick it up, while PR #1 sits red.
Change: user action — assign `seeds-48db` to `fabro` (or fix it manually on the PR #1 branch per the seed's note).
Expected effect: the first format-core merge unblocks; zero loop changes required.

One caveat on what I could not inspect: PR #1/#5 CI state and GitHub-side merge results are outside this run's events; the PR #1 claim above comes from the seed body and tracker state visible in this run's diffs.
