# Improve review — run 01M354EMKT2P2XD1QSC51NJSTM

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (2.6 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 18:08+0000 by revisor `fabro_ask`

---

Bottom line first: this run was healthy — 2m33s wall, $0.226 total, zero retries, first‑pass approve, gate green in 3.2s (all from run events / conclusion). The seed itself (seeds‑c760, a 6‑line reviewer‑stanza edit) was the cheapest possible kind of work, which makes the frictions that DID occur stand out clearly. Recommendations, ordered by expected impact:

---

**1. Bound the planner's `fabro_runs_list` call — it was 48% of planner cost for an already‑answered question. → seed seeds‑37bc**
- **What happened (from run events):** the preflight table already marked both candidates `in_flight: false`, yet the planner still ran `fabro_runs_list {workflow:"develop"}` (seq 57–58): 20.5s of tool time returning **161 runs**, inflating the conversation from ~9.4k to ~60.7k tokens. The next LLM round alone cost $0.0776 (seq 61) — 48% of the planner's $0.162, which is itself **72% of the whole run's cost** — and every later round (claim, brief, final JSON) paid the inflated base (~$0.019–0.021 each). Planner wall time 58.8s = 51% of the run's active time.
- **Change:** `.fabro/workflows/develop/prompts/planner.md` step 4 — mandate `created_since=<now-48h>` (the param already exists per the tool description) and skip the call entirely when the preflight table already says `in_flight: false` for all top candidates; engine-side filters per the seed's engine arm when reachable.
- **Effect:** ~$0.08–0.10 + 25–30s recovered per develop run, prefix cache preserved. This run is fresh recurrence evidence on top of the seed's basis run.

**2. Give the implementer a deterministic verify lane for loop-asset-only seeds. → new seed needed**
- **What happened (implementer@1 journal, this run):** `just verify implementer` answered "no lib/ crates touched — nothing to verify", and since no standalone workflow-graph linter exists (only `prompt-lint.nu`), the implementer improvised an ad-hoc bracket-balance parse — while the tester gate *does* run "workflow graphs parse (graphviz)" (tester output). So the one check that actually validates this seed class runs only after the implementer reports success.
- **Change:** `scripts/verify.nu` — add a loop-asset arm: when no lib crates are touched but `*.fabro` / `prompts/**` / `.fabro/scripts` changed, dispatch the same graphviz parse (+ prompt-lint) the gate uses, as `nu .fabro/scripts/graph-parse.nu <file>`.
- **Effect:** deterministic implementer verification for loop-asset seeds; a malformed graph fails in seconds at the implementer instead of surfacing as a gate-red bounce cycle later.
- **New-seed justification:** implementer@1's journal of this run names the gap verbatim; no open seed covers the implementer verify lane (seeds‑25b5 is the CLI differential battery — different scope).

**3. Teach the preflight that cross-repo anchors are expected, not stale. → new seed needed**
- **What happened (from run events):** the preflight flagged seeds‑a0fa `anchors_ok: false, missing_file: docs/lab/adr/0011-…`, and the planner burned a reasoning round plus a journal observation adjudicating it as "cross-repo citation into denkhaus/fabro — expected to be absent" (seq 53, planner journal). seeds‑a0fa is still open and listed first in `seeds ready`, so **every future run re-adjudicates the same flag**.
- **Change:** `.fabro/workflows/develop/scripts/planner-preflight.nu` anchor arm — classify paths under the documented fabro-lore namespace (`docs/lab/adr/`, `docs/lab/fabro/`, per PROJECT_FACTS "Loop lineage") as `cross_repo_expected` instead of `missing_file`, so they don't drag `anchors_ok` false.
- **Effect:** removes one recurring LLM adjudication round per run while a0fa stays a candidate (~5–10s + a journal line each time).
- **New-seed justification:** no open seed covers preflight anchor classification; fabro‑7daf/fabro‑9ec3 are fabro-repo lineage, not this tracker.

**4. Acknowledge the 10 by-design prompt-lint warnings so the gate log carries signal. → new seed needed**
- **What happened (tester gate output, this run):** `prompt-lint: ok — 42 files, 11 warnings` — 10 of them "top-level property … is routing-named" against `planner-output.schema.json` and `develop-output.schema.json`, which is **deliberate** (the planner schema's own description says it keeps routing semantics on purpose), plus the date-pin warning on `project-facts.md`'s `2026-04-14`, which is the load-bearing toolchain pin. Every gate run ships all 11.
- **Change:** prompt-lint config (in `scripts/`/`.fabro/scripts/` where the linter lives) — add an acknowledged-by-design allowlist for routing-named properties in those two schemas, and a load-bearing marker syntax for the pin date.
- **Effect:** gate tail drops to warnings that actually need action; reviewers and bounced implementers reading the gate log stop re-triaging known-by-design noise.
- **New-seed justification:** no seed covers prompt-lint warning hygiene (checked tracker; seeds‑25b5 is the command-matrix battery).

**5. Silence the chronic error-channel noise before it masks a real incident. → new seed needed**
- **What happened (from worker logs, this run):** 9 warn/error lines, **all chronic noise**: 6× ERROR `File "/workspace/seeds/.codex/instructions.md" was not found` (2 per agent session × 3 sessions), 1× WARN `preamble_allow_keys entry absent … node=implementer key=output.gatebounce` (expected absent on every green run — gatebounce only writes that key on a red bounce), 1× WARN `context_update_dropped: output.planner` (allow-list drift between the engine's response-dedup key and the planner's `context_allow_keys`), 1× WARN PR-generation retry (recovered). The seeds‑7cd1 handoff already documents the `.codex` ERRORs as "known noise" — it was never fixed.
- **Change:** (a) repo: ship a minimal `.codex/instructions.md` or make the session-init probe non-erroring; (b) graph: mark `output.gatebounce` as conditional so the absent-key WARN fires only on red-bounce paths; (c) planner stanza in `workflow.fabro`: reconcile the dedup key with `context_allow_keys`.
- **Effect:** an empty error log once again means "nothing wrong" — the night incident in seeds‑7cd1 (silent stall under routine noise) is the demonstrated downside of leaving this.
- **New-seed justification:** the handoff documents the noise but no seed tracks eliminating it.

---

One non-recommendation observation: the loop's own defenses all worked this run — preflight verdicts were correct, `claim_check` passed in 92ms, evidence rendered inline (3.6KB, no blob detour), the reviewer verified by direct read in one tool call, and closeout closed exactly the claimed seed. The cost center and the friction are concentrated in the planner stage, which is why recommendation 1 is the one to do first — it's also the only one with an open, ready-made seed (seeds‑37bc, currently unassigned).
