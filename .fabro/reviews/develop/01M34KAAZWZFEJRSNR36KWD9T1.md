# Improve review — run 01M34KAAZWZFEJRSNR36KWD9T1

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (8.0 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 13:15+0000 by revisor `fabro_ask`

---

# Recommendations for run 01M34KAAZWZFEJRSNR36KWD9T1 (seed seeds-aa89, $0.51 / 7m59s wall)

All evidence below is from this run's events, stage transcripts, and the journal at `.fabro/journal/01M34KAAZWZFEJRSNR36KWD9T1.jsonl`. Ordered by expected impact.

---

**1. Ship the in-repo arm of seeds-37bc: bound the planner's `fabro_runs_list` call with `created_since`.** (existing seed: **seeds-37bc**, open, P1)
- What happened: planner@1's `fabro_runs_list {workflow: "develop"}` returned **154 runs and took 20.1s of tool time** (13:01:58→13:02:18); the follow-up LLM round was 49,250 input tokens with only 12,288 cache-read (prefix cache broken), $0.074. The planner was 40% of total run cost ($0.204 of $0.513).
- Change: one line in `.fabro/workflows/develop/prompts/planner.md` step 4 — mandate `created_since=<now-48h>` (the tool already supports the param; seeds-37bc's body names exactly this as the in-repo arm while the engine-side filters wait for the fabro rebuild).
- Expected effect: recovers ~$0.07 + 25–30s **per develop run** and preserves the planner's cache prefix — this run re-proved the seed's basis with fresh numbers (154 vs 142 runs).

**2. Add prompt-lint to the implementer's verify lane for prompt-touching diffs.** (new seed — justification: seeds-9482 (closed) fixed only `crates/<name>` derivation in `scripts/verify.nu`; no open seed covers loop-asset prompt files in the verify lane, and mx-f6c95c recorded the lesson, not the mechanism)
- What happened: implementer@1 wrote run-id literals into three prompt files (criterion-3 evidence citations); tester@1 was the **first** signal — GATE RED, 6 prompt-lint errors, `{"hits":[]}` from gatebounce (no known-bug seed for this class). The bounce cost implementer@2 **106.7s / $0.114 ≈ 23% of run wall, 22% of cost**.
- Change: `scripts/verify.nu` (the `just verify implementer` dispatcher) runs `nu .fabro/scripts/prompt-lint.nu` when the diff touches `.fabro/**/prompts/*.md` — same touched-path derivation pattern seeds-9482 introduced for crates. Optionally also fold the implementer's own journaled suggestion: planner briefs pre-state "cite seed ids, never run ids" for prompt-edit criteria.
- Expected effect: prompt-lint failures surface in the implementer's own cheap verify call instead of a full gate-red → gatebounce → implementer lap; eliminates ~110s/$0.11 per occurrence on loop-asset prompt seeds (a recurring seed class in this repo).

**3. Preflight in-flight arm: exclude terminal runs.** (new seed — justification: seeds-37bc covers the `runs_list` tool, not `planner-preflight.nu`'s branch/journal-scan arm; no open seed covers terminal-run exclusion)
- What happened: `output.preflight` marked **all three** ready candidates `in_flight: true, in_flight_run: 01M34FT28TA8SMPWM8ZW7R9KN3` — a run that **failed terminally at 12:02 having claimed nothing** (no journal, no branch, seeds still open). The planner then spent ~4 shell calls + 3 LLM rounds (~45s, roughly half its cost) manually disproving the flags (empty journal grep, journal-dir ls, branch check, `seeds show` ×3) — journaled as a painpoint.
- Change: `.fabro/workflows/develop/scripts/planner-preflight.nu` — drop the in-flight mapping when the mapped run's status is terminal (succeeded/failed/cancelled) or when neither journal nor run branch exists.
- Expected effect: removes the false-positive triage lap from the planner on every run that follows a failed fire (common in this line — the 01M33DH43/F874/GZ3 class); complementary to open **seeds-a0fa** (fabro_ask), which addresses the same re-derivation cost from the consumer side.

**4. Schedule seeds-c760 for the pending user decision: reviewer `skills="discover"` vs tools allow-list.** (existing seed: **seeds-c760**, open, needs-user)
- What happened: reviewer@1 had 204 tokens of skill descriptions injected (`rust-style-guide`, `improve-codebase-architecture`) while `use_skill` is engine-denied — and used **0 tools**, approving from the inline evidence capture (which worked well: 14.5KB capture rendered whole, no blob detour).
- Change: none this run can make (capability-affecting, ADR-0019, awaiting user decision per handoff seeds-7cd1 item 3) — but it should be surfaced to the user now.
- Expected effect: declared capability matches enforcement; ~200+ tokens of dead weight per review disappear.

**5. Planner final output: JSON-only, no preceding prose.** (new seed — justification: searched the tracker for output-cap/length-cap coverage — no hit; smallest item, file it low priority)
- What happened: planner@1 emitted a prose paragraph plus a long JSON (long brief + 3 journal observations) and hit the provider output cap — "response ends before its JSON object closes" — burning one output retry (13:03:20→13:03:31, ~11s, second emit succeeded only after forced compaction).
- Change: `.fabro/workflows/develop/prompts/planner.md` outcome contract — "reply with the JSON object only, no preceding prose; `current_seed_brief` ≤ ~1200 chars; journal entries one line each" (the current text permits "one short paragraph", which is what tipped it over).
- Expected effect: eliminates cap retries that consume `output_retries=2` budget on the node most likely to route the whole run.

---

**Already tracked, no new action:** the reviewer's non-blocking fixture-position note was auto-filed as residual seed **seeds-0511** by closeout; the tracker_guard start-of-run failure was the bug this seed fixed (next runs get the guard green). **What worked and should stay:** fail-open routing past a failed deterministic guard, the gatebounce node, `preamble_inline_max_kb=16` (reviewer needed zero tool round-trips), and reviewer cost discipline ($0.024, 21s).
