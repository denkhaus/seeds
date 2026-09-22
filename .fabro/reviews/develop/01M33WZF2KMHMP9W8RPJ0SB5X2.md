# Improve review — run 01M33WZF2KMHMP9W8RPJ0SB5X2

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (13.4 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 06:50+0000 by revisor `fabro_ask`

---

All evidence is in hand. Here are the recommendations, grounded in what actually happened in run `01M33WZF2KMHMP9W8RPJ0SB5X2` (seed `seeds-a77c`, HITL pilot; 13m26s wall, $0.770 LLM total; one gate-red bounce; tracker_guard failed; PR #35). Sources: run events/checkpoints, stage transcripts, and workspace files `scripts/verify.nu`, `scripts/qualitygate.nu`, `.seeds/issues.jsonl`.

---

**1. Fix the tracker-guard crash — it fails in every run, kills the stale-claim requeue arm, and pollutes every stage prompt. → seed `seeds-aa89`**

- **What happened here:** `tracker_guard` failed at 06:31:18 (exit 1, `nu::shell::column_not_found 'ts'` at `tracker-guard.nu:166`, plus the `ls`-glob `eval_block_with_input` at `:152` — from run events). The full ~1.5 KB error trace was then re-rendered into the preamble of **all four** subsequent agent stages (planner, implementer@1, implementer@2, reviewer — visible in each `stage.prompt`), and three separate roles journaled the identical painpoint. This is the **8th consecutive** develop run with this failure (7 listed in the seed body + this one).
- **Change:** implement open seed **`seeds-aa89`** (fabro-assigned, was 2nd in this run's ready list): optional access `$recs | get ts?` / filter `$r.ts? != null` in `tracker-guard.nu:166`, robust glob catch at `:152`, plus its regression fixture. Extend its acceptance criteria to also cover the `:152` glob arm the planner journaled.
- **Expected effect:** guard stage green again; the dead stale-claim requeue arm (in_progress seeds orphaned >6h are currently never requeued — a line-stall risk) revives; ~1.5 KB of repeated error noise removed from every agent context; stops the triple-duplicate painpoint journaling each run.

**2. Add the qualitygate's graph-parse arm to the implementer verify lane — this run burned a full bounce cycle on a 4-line fix the lane could have caught pre-tester. → new seed**

- **What happened here:** implementer@1 wrote the `needs_operator` gate with unquoted dotted DOT attributes (`human.question=` …); its mechanical lane `just verify implementer` returned green with "no lib/ crates touched" — because `scripts/verify.nu`'s `touched` derives **only** `lib/`/`crates/` paths, so a loop-asset-only diff is a no-op (confirmed from workspace file `scripts/verify.nu:70-97`). The tester then went red at 06:39:32: `workflow graph PARSE FAILED: conductor/workflow.fabro syntax error in line 103 near '.'`. The bounce (gatebounce → implementer@2) cost **205 s wall / $0.156 / 14 messages** to quote 4 attribute names — 20% of run cost for a sub-second parse check.
- **Change:** new seed — extend `scripts/verify.nu` (the fabro-6e7f dispatcher) with the parse-level tier for non-Rust diffs: when the diff touches `.fabro/workflows/*/workflow.fabro`, run the same `dot -Tcanon` check `scripts/qualitygate.nu:173-204` already runs. This is exactly the repo's own standing policy (fabro-9ec3: mechanically-checkable invariants land as script checks, never prompt prose — the implementer's improvised grep/regex "structural checks" are precisely what missed it).
- **Expected effect:** this red class (any workflow.fabro edit) is caught in the implementer lane; saves ~3.5 min + ~$0.16 per occurrence and keeps the tester's first gate green.
- **New-seed justification:** the painpoint was journaled (implementer@2) but closeout only files `deferred-action:` markers as seeds (it filed `seeds-d59d` for the engine-acceptance follow-up only); no open seed in `.seeds/issues.jsonl` covers `verify.nu`'s loop-asset lane.

**3. Bound the planner's in-flight `fabro_runs_list` call — 152 unfiltered runs for a yes/no question, ~26% of run cost in the planner stage. → seed `seeds-37bc` (in-repo arm)**

- **What happened here:** planner's single `fabro_runs_list` call used args `{"workflow": "develop"}` only (event seq 48) and returned **"listed 152 run(s)"** as one JSON blob; the round trip took ~17.7 s (06:31:31.9 → 06:31:49.6) and the planner conversation jumped to ~60k tokens. Planner totals: 92.6 s inference / **$0.204 = 26.5% of run cost** — to answer "any in-flight run besides me?" (answer: none).
- **Change:** implement the in-repo arm of open seed **`seeds-37bc`** now: its body explicitly sanctions this — "if denkhaus/fabro is out of reach … implement the planner.md `created_since` bound as the in-repo arm." One-line edit in `.fabro/workflows/develop/prompts/planner.md` step 4 (IN-FLIGHT EXCLUSION): pass `created_since` (48 h per the consolidated overflow note in the seed) on every `fabro_runs_list` call; engine-side filters follow later under the same seed.
- **Expected effect:** planner's largest single tool payload shrinks from a 152-run inventory to recent runs; the seed's own basis estimates ~$0.07 + 25-30 s recovered per develop run with the cache prefix preserved.

**4. Raise the reviewer's `preamble_inline_max_kb` 16 → 32 so per-seed evidence captures render inline again. → new seed**

- **What happened here:** the evidence capture was **29.1 KB** (`evidence@1` output_bytes 29,009 — a churn-heavy loop-asset seed diff), exceeding the reviewer's 16 KB per-value inline ceiling despite the 48 KB graph budget, so it blob-ref'd and the reviewer paid a `read_file` detour (its only tool call; 36 s stage). The 16 KB value (fabro-9467) was sized when captures were ~14 KB; this run proves the churn-only-seed class exceeds it.
- **Change:** new seed — in `.fabro/workflows/develop/workflow.fabro`, reviewer stanza, `preamble_inline_max_kb=16` → `32` (still under the 48 KB aggregate budget).
- **Expected effect:** ~29 KB captures render inline; one fewer tool round-trip per review, keeping first-pass approvals at context-only speed (this review was otherwise exactly that).
- **New-seed justification:** no open seed touches preamble rendering budgets; `seeds-c760`/`seeds-9fa3` cover the reviewer's skills declaration and readability, not the evidence inline ceiling.

---

**Ordering rationale:** #1 hits every develop run and restores a dead safety arm; #2 eliminates a demonstrated ~$0.16/3.5-min bounce class for all graph-editing seeds; #3 trims the largest per-run planner cost; #4 is a small constant saving per review.

Already tracked, no new recommendation needed: the unverified engine acceptance of the quoted `human.*` gate attributes is filed as **`seeds-d59d`** (execute it on the first live capability-block fire); the residual-filing marker gate is **`seeds-81cd`** (this run's closeout correctly filed only the one `deferred-action:` seed, but the class it fixes is still open).
