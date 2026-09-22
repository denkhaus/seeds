# Improve review — run 01M34N0KNQSGJQVXN1A6EDTJG1

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (2.8 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 13:40+0000 by revisor `fabro_ask`

---

All evidence below is from this run's events (run 01M34N0KNQSGJQVXN1A6EDTJG1, seed seeds-0511, total $0.206 / 155 s wall, all 10 stages green, 0 retries) and the tracker at `/workspace/seeds/.seeds/issues.jsonl`. I checked the tracker first; seed mappings are noted per item.

## 1. Bound the planner's `fabro_runs_list` call — the single biggest cost lever in this run
**What happened (run events):** the planner's seq-49 `fabro_runs_list` returned an unfiltered inventory of **155 runs**; the very next LLM turn (seq 52) cost **$0.0751 with 49,915 input tokens** (~4× every other planner turn: $0.017–0.020), because ~45k tokens of run-list JSON landed in the conversation. The planner finished at **$0.149 = 73% of the whole run's $0.206** and 71.6 s of a 155 s run — to plan a 7-line fixture reorder. The in-flight question it answered is a yes/no over "open PR or non-terminal status".
**Change:** in `.fabro/workflows/develop/prompts/planner.md` step 4, mandate `created_since` (~48 h) on every `fabro_runs_list` call now (in-repo arm), and the server-side status/PR-state filters when the fabro rebuild lands.
**Expected effect:** ~$0.06–0.08 and 10–20 s recovered per develop run; planner cache prefix stops being broken by a giant mid-conversation tool result.
**Seed: seeds-37bc** (open, P1) covers exactly this — engine filters + planner.md pointer, with the `created_since` prompt-side arm as the fallback. This run is fresh recurrence evidence for it.

## 2. Preflight in-flight arm marks terminal-failed runs as "in-flight" (error handling)
**What happened (run events):** the preflight verdict flagged **seeds-81cd `in_flight: true` via run 01M34FT28TA8SMPWM8ZW7R9KN3** — but this same run's `fabro_runs_list` (seq 49) shows that run as `failed`, completed 12:02:09. The planner journaled it ("branch-scan in-flight arm may be over-retentive") and moved on only because seeds-0511 was the top candidate. Had seeds-0511 not existed, the only other ready seed would have been skipped and the run routed Tracker empty — a full burned fire (the exact waste class of the 2026-09-22 night incident).
**Change:** in `.fabro/workflows/develop/scripts/planner-preflight.nu`, the in-flight branch-scan arm must exclude runs whose catalog status is terminal (failed/succeeded without an open PR), not just "branch exists".
**Expected effect:** no false in-flight skips of claimable seeds; kills a misclassification that silently narrows the candidate pool every run.
**Seed:** none covers it — seeds-81cd is the closeout residual-marker gate, seeds-37bc is the tool payload. **New-seed justification:** the finding exists only as this run's planner journal observation, and journal-only findings die with the run per the loop's own rule.

## 3. Preflight anchor arm false-positives on prose slash-pairs from finding text
**What happened (run events):** the verdict table flagged `analyze.md/file.md` as `missing_file` on seeds-0511. That string is prose from the seed body ("revisor analyze.md/file.md" — two files that both exist under `.fabro/workflows/revisor/prompts/`). The planner burned a reasoning round diagnosing it as a false positive and then manually re-verified `tracker-guard-smoke.nu` lines 116–131 anyway (shell calls at seq 53/59). Residual seeds are auto-filed from reviewer finding text, so this recurs on every residual seed.
**Change:** in the same `planner-preflight.nu` anchor arm, don't flag a slash-joined token when its components resolve as existing repo files (or restrict anchor extraction to fenced/quoted paths).
**Expected effect:** removes one planner adjudication detour (~$0.02, ~8 s) per residual seed and keeps `anchors_ok` trustworthy.
**Seed:** none — **new-seed justification:** first observed this run; the anchor arm (fabro-7daf/fabro-9ec3 lore) has no open seed for tokenization false positives.

## 4. Baseline-suppress the 11 standing gate warnings (UX / alarm fatigue)
**What happened (run events):** the tester output — rendered verbatim into the reviewer's preamble (visible in reviewer@1's prompt) — carried **11 warnings: 10× "routing-named" on `planner-output.schema.json` + `develop-output.schema.json` (intended by the fabro-9ec3 arm-3 design) and 1× date-pin**. A genuinely new warning would hide among ten known ones.
**Change:** in `.fabro/scripts/prompt-lint.nu` (invoked via `scripts/qualitygate.nu`), add a baseline/allowlist for the two known-intent schema files (or an inline lint-ignore marker in the schemas); print/fail only on delta.
**Expected effect:** new lint regressions become visible instead of drowned; ~1 KB less noise in every reviewer preamble.
**Seed:** none — **new-seed justification:** no open seed addresses prompt-lint output hygiene; seeds-aa89 (closed) covered tracker-guard schema handling, not lint warning baselining.

## 5. Fix the per-run `context_update_dropped: output.planner` warning (graph hygiene)
**What happened (run events):** a `run.notice` (warn, seq 75) fired immediately after the planner completed: the engine records the planner's response under `output.planner`, but the planner node's `context_allow_keys` (`current_seed_id,current_seed_title,current_seed_brief,review_verdict,journal`) doesn't list it — so it's dropped with a warning on **every** planner pass, defeating the fabro-e47c producer-declaration lint the node opts into.
**Change:** add `output.planner` to the planner's `context_allow_keys` in `.fabro/workflows/develop/workflow.fabro` (or stop recording the key).
**Expected effect:** one less warn notice per run; the context-contract lint becomes self-consistent instead of permanently noisy.
**Seed:** none — **new-seed justification:** config-level drop noticed only in this run's event stream; no existing seed covers context_allow_keys drift on the planner.

**What I would *not* change based on this run:** the reviewer (14.3 s, $0.017, zero tool calls, first-pass approval from the evidence capture) and the cheap gate (3.3 s, "no crates touched") performed exactly as designed; the implementer's one-chained-recon + shell-heredoc edit pattern (3 shell calls, 0.8 s tool time) also matched the fabro-866a policy — no recommendation warranted there.
