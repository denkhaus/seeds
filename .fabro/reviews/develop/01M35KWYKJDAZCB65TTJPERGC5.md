# Improve review — run 01M35KWYKJDAZCB65TTJPERGC5

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (3.4 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 22:39+0000 by revisor `fabro_ask`

---

All recommendations below are grounded in run `01M35KWYKJDAZCB65TTJPERGC5` (all 10 stages first-pass green, 3m11s wall, **$0.184 total**: planner 35.9s/$0.046, implementer 116s/$0.116 = 63% of cost, reviewer 15.8s/$0.022, tester 3.3s — from run events). Tracker check done: the only open seeds are **seeds-7e0f** (untracked-file gate derivation), **seeds-7212** (reviewer no-fs_hide exemption), and **seeds-37bc** (engine-side runs_list filter); none of them cover items 1–5 below, and this run adds no new grounding for them (no Rust touched; the planner correctly skipped `fabro_runs_list` because the preflight table was `in_flight:false` throughout).

## 1. Fix the anchor resolver's false `missing_file` on loop-asset paths — highest impact
- **What happened**: the preflight flagged `evidence.nu:38 missing_file` (seeds-731d) and `workflow.fabro:399 missing_file` (seeds-7212) — both files exist (from run events, `output.preflight`). The planner then burned two extra shell rounds + reasoning re-proving the flags wrong (seq 56–65: `rg --files` + `sed -n 280,400p`, ~7s of its 36s and roughly a third of planner reasoning), and both planner **and** implementer journaled the same painpoint (duplicate journal entries risk double-filing downstream).
- **Change**: `.fabro/workflows/develop/scripts/anchor_check.nu` — `resolve-anchor-path` (line 92) resolves only against repo root and workspace-member `src/` roots. Extend it with the same develop-workflow roots `check-bare-paths` (line 173) already uses: `.fabro/workflows/develop` **and its `scripts/` subdir**.
- **Expected effect**: preflight verdicts become trustworthy again (its whole purpose per fabro-a32f); planner stops re-opening files by hand; ~2 LLM rounds + one journal painpoint per run removed while seeds cite loop-asset anchors.
- **Seed**: none exists (tracker greps for anchor check/resolver/missing_file: no hits). *New-seed justification: the false-positive class (anchored `path:line` citations of loop-asset files) is distinct from seeds-7e0f/7212/37bc and from closed seeds-69ae (wrong default branch) — no open or closed seed covers anchor path resolution.*

## 2. Gate the diff-walk behavior permanently — the run's core change is enforced nowhere
- **What happened**: seeds-731d landed the HARD_CAP discount + silent-drop fix, but the implementer verified it with throwaway `/tmp` scratch fixtures; the tester gate only ran `lint-nu` over 27 scripts ("no crates touched"), and the reviewer approved while explicitly noting "fixture claims in the summary are not independently re-runnable here" (from reviewer journal). A future edit to `diff-walk` could silently regress the no-silent-drop invariant (mx-0b6d3c) with a green gate.
- **Change**: add `.fabro/scripts/diff-walk-smoke.nu` (a permanent scratch-repo fixture: large new file + small edit → assert no `hard cap hit`, omitted=0; 1.5 MB blob → assert cap trips) and wire it into the loop-asset arm of `scripts/qualitygate.nu` next to the lint-nu check.
- **Expected effect**: the invariant the run just fixed becomes deterministically gated; reviewers stop approving self-reported fixture claims on evidence-pipe changes.
- **Seed**: *New-seed justification: seeds-731d is closed and its scope was the discount itself, not a permanent test; no seed covers a diff-walk/evidence smoke battery.*

## 3. Triage the gate's 11 standing prompt-lint warnings — warning fatigue
- **What happened**: tester output this run: `prompt-lint: ok — 44 files, 11 warnings`, including the date-pin warning (`nightly-2026-04-14` older than 45 days — still load-bearing) and 10× "routing-named property" warnings on `planner-output.schema.json` and the conductor schema — both schemas are *deliberately* routing-contract schemas (fabro-9ec3, per workflow.fabro comments). Every gate run re-emits known-benign warnings.
- **Change**: `.fabro/scripts/prompt-lint.nu` — allowlist the two intentional routing schemas (or require an explicit marker before warning), and let the date-pin warning be acknowledged via an operator marker so it fires only when the pin actually changes.
- **Expected effect**: gate output returns to zero known-benign warnings, so the next real prompt-lint regression is visible instead of buried in noise.
- **Seed**: *New-seed justification: no tracker seed addresses prompt-lint warning hygiene; seeds-7e0f is about file-derivation in verify/qualitygate, a different arm.*

## 4. Declare `output.planner` in the planner's context_allow_keys
- **What happened**: run event seq 79, warn notice: `context_update_dropped: output.planner` — the engine's response-dedup writes `output.planner`/`response.planner` into the planner's context_updates, but the planner node's `context_allow_keys` (workflow.fabro, planner node) doesn't list it, so it's dropped with a warn on every planner pass.
- **Change**: `.fabro/workflows/develop/workflow.fabro`, planner node — add `output.planner` to `context_allow_keys` (fabro-900e producer-declaration pattern).
- **Expected effect**: removes one spurious run-notice warn per run; keeps the fabro-900e drift lint meaningful (right now it cries wolf every pass, which is exactly how real drift gets ignored).
- **Seed**: *New-seed justification: a one-line graph-attr fix observed only as warn noise this run; no seed covers planner context-key declarations.*

## 5. Kill the extra `ml search` round for lesson capture
- **What happened**: implementer journal painpoint 2 — "`ml record` prints only 'Recorded pattern in dev-loop' with no mx-id; capturing the id for lesson_capture required an extra `ml search` round." One avoidable shell round per implementer pass (implementer already ran 13 shell calls with 113s inference vs 2.5s tool time — every removable round is ~9s of inference-dominated wall).
- **Change**: `ml` is the pinned external mulch CLI (`npm:@os-eco/mulch-cli` in `.mise.toml:18`), so the in-repo arm is `.fabro/workflows/develop/prompts/implementer.md` (Lesson capture section): prescribe the chained form `ml record ... && ml search <name> --format json` as ONE shell call, mirroring the documented `seeds update --format` observed-failure pattern; the mx-id echo itself is an operator demand to the mulch CLI.
- **Expected effect**: one shell round + one LLM round saved per implementer pass; `lesson_capture` stops depending on a second lookup that can pick a duplicate stub record.
- **Seed**: *New-seed justification: no seed covers the mulch CLI's record/search round trip; it's toolchain-side friction first observed in this run's journal.*

**Not recommended from this run**: seeds-7e0f and seeds-7212 remain valid open seeds but this run supplies no new evidence for them (Nushell-only diff, reviewer used 0 tools because the evidence capture was complete — seeds-731d's fix working as intended, incidentally: the reviewer judged from the full inline loop-work diff with zero blob detours).
