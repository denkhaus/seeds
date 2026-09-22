# Improve review — run 01M35GF84EPCHM7FD7AHPDCEG0

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (2.7 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 21:39+0000 by revisor `fabro_ask`

---

All evidence is in hand. Here are the recommendations, ordered by expected impact, each grounded in what this run actually did (a clean 2-minute, $0.157 cycle that claimed and closed `seeds-8795` — a one-line edit to `planner.md` step 4; PR #53).

---

**1. Mechanize the "skip runs_list when the preflight table is clean" decision in the preflight script — the prompt-only fix failed its first live test one stage before it landed.**
- **What happened:** The preflight verdict table (run checkpoint, `output.preflight`) marked `in_flight: false` for all four candidates — yet the planner, which had just written the skip clause into `planner.md` step 4 as this run's own seed work, still called `fabro_runs_list` (67 runs pulled into context). Its own journal admits it: "the call returned 67 runs (bounded here only because I pre-applied the seed's proposed created_since bound)." Voluntary prose compliance failed on first contact. The planner stage was 48.8s / $0.077 — 49% of total run cost.
- **Change:** In `.fabro/workflows/develop/scripts/planner-preflight.nu`, emit a table-level deterministic field (e.g. `runs_list_skippable: true` when the table is healthy and all candidates are `in_flight: false`), and reduce `planner.md` step 4 to "if `runs_list_skippable` is true, do not call the tool." This follows the loop's own standing policy (fabro-9ec3: mechanically-checkable invariants live in the preflight script, never as prose).
- **Expected effect:** Removes the runs_list tool round and its 67–162-run context bloat entirely on clean-table runs (~10–30s and ~$0.03–0.10 per develop run, per the measurements cited in seeds-8795).
- **Seed:** New seed needed — **seeds-8795 (closed) covered only the prompt arm; seeds-37bc (open) covers only engine-side tool filter params. The preflight-script arm is a third tier neither names.**

**2. Teach `anchor_check.nu` to resolve bare loop-asset filenames — this run's preflight falsely flagged 2 of 4 candidates `missing_file`.**
- **What happened:** The preflight flagged `seeds-731d` (`evidence.nu:38 → missing_file`) and `seeds-7212` (`workflow.fabro:399 → missing_file`). Both files exist — I verified `.fabro/workflows/develop/scripts/evidence.nu` and `.fabro/workflows/develop/workflow.fabro` in the workspace. `resolve-anchor-path` (anchor_check.nu:92–100) tries only the repo root and Rust workspace member `src/` roots (fabro-7611); bare loop-asset filenames never resolve.
- **Change:** In `anchor_check.nu`, add loop-asset search roots to the resolution stage (`.fabro/workflows/develop/{scripts,prompts,schemas}`, `.fabro/scripts`, `scripts/`) exactly mirroring the existing member-src-roots pattern, and report the resolved `base` so flags stay trustworthy.
- **Expected effect:** Stops false flags on loop-asset seeds. This is near-certain friction, not hypothetical: `seeds-731d` and `seeds-7212` are the open fabro-assigned P2 queue — whichever becomes the next top candidate gets flagged `missing_file` at claim time, forcing the planner to hand-re-verify paths (the exact wasted-calls class fabro-05d0 documented) or risk mis-adjudicating the stale-basis check.
- **Seed:** New seed needed — **no open or closed seed covers anchor path resolution for loop assets; fabro-7611 (the crate-relative fix) is fabro-repo lineage, and the only tracker hits for "anchor" are unrelated closed seeds.**

**3. Allowlist the by-design `prompt-lint` warnings — every gate log ships 11 warnings, 11 of which are intentional.**
- **What happened:** The tester output this run shows `prompt-lint: ok — 43 files, 11 warnings`: 10× "routing-named property" on `planner-output.schema.json` and the conductor `develop-output.schema.json` (those routing fields are deliberate — fabro-9ec3 arm 3 / fabro-0a4c), plus the date-pin warn on the intentionally pinned `nightly-2026-04-14` toolchain. The emitters are `.fabro/scripts/prompt-lint.nu` checks 3 and 5 (lines 93–107, 151–155).
- **Change:** In `prompt-lint.nu`, add an allowlist for routing-named top-level keys in `*/schemas/*-output.schema.json` files that declare routing intent (or downgrade those to an "intentional" info line), and let a reviewed-marker (e.g. a comment in `project-facts.md`) silence the date-pin warn.
- **Expected effect:** Gate logs carry only actionable warnings; a real prompt-lint regression (the check exists to catch accidental routing activations) stops being buried in static that reviewers and the operator monitoring recipe train themselves to skip.
- **Seed:** New seed needed — **no seed in the tracker covers prompt-lint warning policy (only unrelated closed seeds-0511 mentions prompt-lint in passing).**

**4. Fix PR-content generation: the configured "clean JSON" PR model failed structured output and fell back to salvage.**
- **What happened:** Run logs at 21:33:42 and 21:33:49: "PR content structured generation failed; retrying once without strict JSON output… retry was not JSON; salvaging prose body with deterministic title." The `[run.pull_request] model = "zai:glm-4.7"` setting (`.fabro/workflows/develop/workflow.toml:71`) was chosen precisely because its comment claims "clean JSON for PR titling" — that premise failed this run, costing two LLM calls and ~20s of the terminal tail (closeout done 21:33:31, run completed 21:33:54).
- **Change:** In `workflow.toml` `[run.pull_request]`, either drop the LLM body step in favor of the deterministic skeleton the engine already produces as salvage (PR #53's one-line seed diff needs little prose), or route generation through a JSON-reliable path and update the stale comment.
- **Expected effect:** ~10–20s off every run's terminal tail and consistent PR bodies instead of salvaged prose.
- **Seed:** New seed needed — **no seed in this tracker covers PR-content generation; the glm-4.7 note lives only in fabro-6a5a lore in the workflow.toml comment (fabro-repo lineage), not as an actionable seeds- tracker item.**

**5. Add a condensed clean-table fast path to the top of `planner.md` (prompting strategy).**
- **What happened:** The planner burned 41.7s of inference (35.5k input tokens) to claim the top P1 seed on a healthy preflight table with `anchors_ok: true`, no review verdict pending, and a pre-written seed body — then produced a 682-token response. Most of the 32-line "Plan the next seed" procedure (stale-basis hand-adjudication, two-branch rule, capability-block routes) was irrelevant to this pass but sat in the reasoning path. Planner = 49% of run cost, 35% of wall time.
- **Change:** In `.fabro/workflows/develop/prompts/planner.md`, prepend a ~10-line "CLEAN TABLE FAST PATH" (table healthy + top candidate `clean`/`anchors_ok` + no `review_feedback` → `seeds show` → claim → bulleted brief, skip the adjudication prose), keeping the full procedure for the dirty cases.
- **Expected effect:** Shrinks planner reasoning scope on the most common path; even a 30–40% cut of the planner's 48.8s/$0.077 is the largest single-stage saving available in this graph (implementer and reviewer are already lean at 57s/$0.061 and 11.9s/$0.019).
- **Seed:** New seed needed — **seeds-7212 (open) tunes the reviewer prompt; no seed covers planner-prompt fast-pathing.**

**6. Silence the per-session `.codex/instructions.md` ERROR noise.**
- **What happened:** Worker logs show `File "/workspace/seeds/.codex/instructions.md" was not found` logged at ERROR twice per agent session init — 3 agent sessions = 6 of this run's 16 warn-or-worse lines. It's documented as known noise in the closed handoff seeds-7cd1, and the operator monitoring recipe greps severity ERROR, so pure noise dilutes real failure signal.
- **Change:** Commit a minimal `.codex/instructions.md` at the repo root (a one-line pointer to `AGENTS.md`) — the engine reads it from the worktree, so the file's absence is repo-fixable, not engine-only.
- **Expected effect:** Removes 6 ERROR lines per run; ERROR-level monitoring (rootprint severity ERROR per the operator recipe) pages only on real failures.
- **Seed:** New seed needed — **seeds-7cd1 (closed handoff) documents the noise but filed no actionable seed; nothing open covers it.**

---

**Already seeded, verified not re-filed:** the open queue itself captures the other findings a reviewer of this run would reach for — `seeds-731d` (evidence HARD_CAP blob paging; this run's 9.6KB capture stayed inline under the 16KB budget, so it didn't bite), `seeds-7212` (reviewer fs_hide exemption), `seeds-7e0f` (untracked-file gate derivation; irrelevant here since the diff touched no new files).

**One thing worth keeping as-is:** the reviewer approved in 11.9s with zero tool calls because the evidence capture (9.6KB) rendered inline — direct validation that the `preamble_inline_max_kb=16` fix works; don't lower that budget.

Sources: run checkpoints/stage records and planner/implementer/reviewer journals (from run events via `fabro_run_get`), worker log lines (from `fabro_run_logs`), tester gate output (from run events), tracker state (from workspace file `.seeds/issues.jsonl`), and the named workspace files `anchor_check.nu`, `prompt-lint.nu`, `workflow.toml`.
