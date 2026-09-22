# Improve review — run 01M35QB2NPCYNCFVGK190EH9ZM

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (3.0 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 23:39+0000 by revisor `fabro_ask`

---

All evidence is gathered (run events/checkpoints, stage conclusion timings+cost, journal, worker warn logs, tracker state). Here are the recommendations, ordered by expected impact, each grounded in this run (`01M35QB2NPCYNCFVGK190EH9ZM`, seed `seeds-7e0f`, 162.7s wall / 143.0s active / $0.148 total).

Tracker check first: only two seeds are open — `seeds-37bc` (engine-side runs_list filters) and `seeds-b1e9` (toolchain-pin lint warning, needs-user). Everything below that isn't one of those carries a new-seed justification.

---

**1. The quality gate cannot functionally test its own derivation scripts — add a fixture smoke for `touched`/`touched-crates`.**
What happened: this run's seed *changed the gate's own derivation* (`scripts/verify.nu`, `scripts/qualitygate.nu`), yet the tester gate ran 3.25s with output "no crates touched" — only lint-nu, graphviz parse, and `cargo fmt --check --all` (from run events, tester stage). The union behavior was proven only by the implementer's scratch-file probe, which was deleted; the reviewer then re-verified by *reading code*, not behavior (reviewer journal: "Verified both derivations directly … union applied identically").
Change: add a deterministic derivation smoke to the gate (temp fixture repo with one tracked change + one untracked test file → assert the crate lands in both code and tests sets), following the existing idiom in `.fabro/scripts/dup-run-check-fixtures.nu` / `tracker-guard-smoke.nu` — wired into `scripts/qualitygate.nu`.
Effect: future edits to the derivation (exactly the class `seeds-7e0f` fixed) are regression-gated; reviewers stop approving gate-behavior changes from prose + code reading alone.
New-seed justification: `seeds-7e0f` (closed) was the fix itself; no open or closed seed adds a regression test for the derivation, and the two open seeds (`seeds-37bc`, `seeds-b1e9`) cover other ground.

**2. Implementer prompt: give the exact `ml record` invocation — omitting `--resolution` cost a tool error and a duplicate expertise record this run.**
What happened: the implementer journaled "ml record for type failure requires --resolution (retry hint is clear but the first call fails); existing record mx-8e3f7c already carried this lesson — filed a duplicate then merged via ml outcome + ml delete" — one of its 2 errored shell calls (of 14), plus merge/delete cleanup rounds, inside the stage that was **58% of run cost ($0.086 of $0.148) and 53% of wall (86.1s)** (from run conclusion + implementer journal). The prompt's step 6 only shows `ml record <domain> --type ... --description ...`.
Change: one line in `.fabro/workflows/develop/prompts/implementer.md` step 6 / Lesson-capture section: failure-type records require `--resolution "<how it was resolved>"` — include it in the shown command form.
Effect: first-call success; eliminates the duplicate-stub-and-merge detour (the exact risk the one-record rule exists to prevent) in the most expensive stage.
New-seed justification: no seed in the tracker covers ml/mulch CLI flag guidance (checked open list and closed bodies surfaced in this run's diffs).

**3. Shrink the implementer prompt by moving the Rust verification policy into the already-mandatory style-guide skill.**
What happened: implementer inference was 81.5s vs 4.1s tool time (~20:1) on a 2-file, 10-line Nushell diff, with 16,948 input tokens — step 4's ~1,900-word Rust gate policy (fmt/clippy/feature-scope/nextest rules) was 100% irrelevant to this non-Rust seed yet paid on every token (from stage usage and prompt text).
Change: in `.fabro/workflows/develop/prompts/implementer.md`, compress step 4 to ~5 lines and move the detailed Rust policy into `.fabro/skills/rust-style-guide/` (a page the prompt already makes mandatory reading for any Rust seed).
Effect: ~3–4k input tokens saved per implementer pass (~$0.01–0.02 and seconds of TTFT per run, more on bounce cycles); zero coverage loss on Rust runs because the skill read is a hard gate.
New-seed justification: `seeds-7212` (closed) fixed the *reviewer's* guide-loading text only; no seed covers implementer prompt size or step-4 relocation.

**4. Planner: don't re-verify anchors the preflight already cleared.**
What happened: the preflight table said `anchors_ok: true, anchor_flags: []` for `seeds-7e0f` (run events, seq 29), yet the planner still spent a shell round re-reading `scripts/verify.nu:72` / `scripts/qualitygate.nu` by hand (seq 51–53, ~6s + one LLM round) — the prompt's fabro-9ec3 policy ("adjudicate flagged candidates") never says a *clean* table is final.
Change: one sentence in `.fabro/workflows/develop/prompts/planner.md` step 3: "when `anchors_ok=true` and `anchor_flags=[]`, the basis resolves mechanically — do not re-open the cited files."
Effect: one redundant LLM round removed per clean-basis run (~6s, ~$0.006); enforces the existing standing policy.
New-seed justification: no tracker seed covers preflight clean-table trust; `seeds-8795` (closed) covered the adjacent runs_list skip.

**5. Error handling (engine-side): the zai protocol drops tool-result error flags — fall back to text encoding and log once.**
What happened: 18 of this run's 27 warn log lines are `unsupported_control: this provider protocol does not support the tool result error flag` (9 implementer tool rounds ×2, from worker logs) — meaning tool failures (like the ml-record error in #2) reach glm-5.3 *without* the structured error signal, and every such round spams two warnings.
Change: engine (denkhaus/fabro, `pebble_coding_agent`): when the provider lacks the error flag, prefix the failure text into the tool result ("ERROR: …") and emit the protocol warning once per session, not per round.
Effect: agent-visible failures become salient on this provider (faster self-correction, fewer wasted rounds); log noise drops from 18 to ~2 lines per run.
New-seed justification: no seed covers provider error-flag degradation; the two open seeds are unrelated.

**6. UX: terminal notifications are still disabled — this run's completion and PR #61 auto-merge were silent.**
What happened: run settings show `notifications.terminal.enabled: false` (from run spec); the run finished 23:34:14 with auto-merge PR #61 opened, and the operator only learns this by polling. This is a recurrence of the finding in `.fabro/reviews/develop/01M332792GNEPNR45VMEXV12WW.md` §6, whose vehicle was "fold into seeds-d2c7" — but `seeds-d2c7` is no longer open (tracker grep shows only `seeds-37bc` and `seeds-b1e9` open).
Change: enable `[run.notifications.terminal]` for the develop line in `.fabro/workflows/develop/workflow.toml` (it is currently explicitly disabled there, lines 83–86).
Effect: terminal events (including silent-death classes documented in mx-17c0b6) reach the operator without the per-session heartbeat polling.
New-seed justification: the prior vehicle seed (`seeds-d2c7`) is closed and the documented recommendation never landed; no open seed covers it.

**7. Graph hygiene: add `output.planner` to the planner node's `context_allow_keys`.**
What happened: every planner pass ends with a `context_update_dropped: output.planner` warning (run notice seq 67; worker log) because the engine's response-dedup writes that key while the planner node declares only `current_seed_id,current_seed_title,current_seed_brief,review_verdict,journal`.
Change: one attribute edit in `.fabro/workflows/develop/workflow.fabro`, planner node: `context_allow_keys="...,output.planner"`.
Effect: removes a guaranteed warn from every develop run's event stream; cleaner signal for the fire-drift/noise triage the loop already does.
New-seed justification: no seed covers this allow-keys/dedup mismatch.

---

Also noted, already covered (no new work): the toolchain-pin lint warning fired again in this run's gate output ("date pin '2026-04-14' older than 45 days") — that is exactly open seed `seeds-b1e9` (needs-user decision); and the planner correctly *skipped* the unbounded `fabro_runs_list` call because the preflight table was clean, confirming the landed `seeds-8795` prompt arm works — open seed `seeds-37bc` remains the engine-side follow-up and did not regress this run.

Sources: run events/checkpoints and conclusion timings+usage (`fabro_run_events`, `fabro_run_get`), stage journals in `.fabro/journal/01M35QB2NPCYNCFVGK190EH9ZM.jsonl` (via run diff), worker warn logs (`fabro_run_logs`), and workspace files `.seeds/issues.jsonl`, `.fabro/workflows/develop/workflow.fabro`, `.fabro/workflows/develop/workflow.toml`, `.fabro/reviews/develop/01M332792GNEPNR45VMEXV12WW.md`.
