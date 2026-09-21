# Revision — run 01M31Z2XTPMMA78KHR4D9K84E6

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M31Z2XTPMMA78KHR4D9K84E6.md
- seeds filed: none — zero balance credit this pass (no same-pass stale/superseded closes); all findings journaled as overflow for the next pass
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M31Z2XTPMMA78KHR4D9K84E6, workflow version e3c92fec9a9a6fd9256277d60e06e12dddcacf6779280c9df5c2b2825b813ea8, commit 41f36c70622450326f394917a0801910be85d6fc
- revised_at_commit: 41f36c70622450326f394917a0801910be85d6fc (ADR-0015: engine drift signal for later judgement)

## Findings

### Initialize the `.mulch/` store or document the ml-record skip in project-facts
overflow-dup: Init mulch or drop the ml mandate in AGENTS.md (open in 01M2ZYVQ7CBSY3T0FEYBE9JK74.md) — same theme already open; adds a third remediation arm (project-facts.md one-line skip) to the existing entry.

### Fix backtick-citation claim extraction in `anchor_check.nu` extract-anchors
- overflow: Fix backtick-citation claim extraction in `anchor_check.nu` extract-anchors — in `.fabro/workflows/develop/scripts/anchor_check.nu` `extract-anchors`, advance past a leading quote char in `tail` before applying `claim_pat`, so backtick-wrapped path citations stop capturing prose as the claim; effect: `anchors_ok` becomes trustworthy, planners stop spending ~1 LLM round + 2 shell calls re-verifying correct anchors (preflight flagged all three seeds-69ae anchors while paths/lines were correct, run 01M31Z2XTPMMA78KHR4D9K84E6).

### Capture implementer verification outputs in `evidence.nu` for behavioral criteria
- overflow: Capture implementer verification outputs in `evidence.nu` for behavioral criteria — extend `.fabro/workflows/develop/scripts/evidence.nu` to append the implementer's verification-lane command outputs, or add a line to `.fabro/workflows/develop/prompts/reviewer.md` that preflight verdicts can never satisfy a bare-default criterion; effect: closes a class of approvals based on claims rather than evidence (reviewer ratified the bare-`--base` criterion from a preflight row produced with an explicit `--base`, run 01M31Z2XTPMMA78KHR4D9K84E6). Distinct from the open evidence-capture overflow (blob collapsing, 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 20) — different mechanism, same file.

### Make planner's `fabro_runs_list` call conditional on degraded preflight arms
overflow-dup: Bound planner in-flight fabro_runs_list with created_since=<now-48h> (open in 01M31EVYN6FD7CD6198J1J7XDS.md) — same planner step-4 mandate theme already open; this adds the conditional-only-when-degraded arm.

### Cost-tier the implementer for micro-seeds (no Rust, ≤3 single-line edits)
- overflow: Cost-tier the implementer for micro-seeds — in `.fabro/workflows/develop/prompts/implementer.md`, let trivial-tier briefs (no Rust, ≤3 single-line edits) run recon+edit+verify in at most 3 chained shell rounds; effect: roughly halve implementer wall and cost (~50s, ~$0.03 saved) on this seed class — a 3-line no-Rust seed cost 104.4s / $0.070 (56% of run spend) in run 01M31Z2XTPMMA78KHR4D9K84E6.

### Stop rendering preflight JSON twice in planner/implementer preambles
- overflow: Stop rendering preflight JSON twice in preambles — in `.fabro/workflows/develop/workflow.fabro`, add `preflight` (and `tracker_guard`, `claim_check`) to `preamble_stages_ignore` on the planner and implementer nodes (the `## Context` key keeps the data); effect: ~1.5–3KB less prompt per LLM round (~$0.01/run, more on multi-cycle runs) at zero information loss, run 01M31Z2XTPMMA78KHR4D9K84E6.
