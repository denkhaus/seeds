# Revision — run 01M35J62BSHH63XKTT34WP1SR3

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M35J62BSHH63XKTT34WP1SR3.md
- seeds filed: none — zero balance credit (0 same-pass stale/superseded closes), all surviving findings journaled below
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M35J62BSHH63XKTT34WP1SR3, workflow version c0e3dc086cfa80e8a79408dcf1552842ecd26999e9fc7b8e093985a779cb83c4, commit 1df2d69b8f6ab6fda087f8c61777974df2224b49
- revised_at_commit: 1df2d69b8f6ab6fda087f8c61777974df2224b49 (ADR-0015: engine drift signal for later judgement)

## Findings

### Raise reviewer preamble line cap so anomaly sections render whole
overflow: Raise reviewer preamble line cap so anomaly sections render whole — change `preamble_output_max_lines` 200→300 on the reviewer node in `.fabro/workflows/develop/workflow.fabro` (or bound the anomaly section in `.fabro/workflows/develop/scripts/evidence.nu` so it fits); effect: mixed captures with 6+ loop-churn files render whole and approvals stop resting on unseen diffs (this run's reviewer preamble ended "(14 lines omitted)", cutting the entire README.md diff; reviewer made 0 tool calls and approved unseen). Adjacent to open seeds-731d (node preamble cap vs its HARD_CAP — file stage may extend 731d instead). Distinct from open overflow "Raise reviewer preamble_inline_max_kb from 16 to 32" (01M33WZF2KMHMP9W8RPJ0SB5X2.md line 16 — per-value KB ceiling, different knob).

### Teach planner-preflight anchor arm loop-asset paths and negative anchors
overflow-dup: Resolve loop-asset filenames in anchor_check.nu (open in 01M35GF84EPCHM7FD7AHPDCEG0.md) — same theme (loop-asset anchor false positives; that entry cites this run's exact `evidence.nu:38`/`workflow.fabro:399` flags). New arm not covered there: teach `planner-preflight.nu` itself loop-asset resolution plus a `!`-prefix must-NOT-exist anchor syntax (the seeds-e5af invariant WANTS hooks absent; this run burned 4 of the planner's 6 shell calls hand-adjudicating 3 of 4 false flags).

### Auto-discover fixture batteries in qualitygate.nu
overflow: Auto-discover fixture batteries in qualitygate.nu — glob-discover `.fabro/scripts/*-fixtures.nu` in `scripts/qualitygate.nu` (~line 153) instead of the hardcoded batteries list; effect: new batteries self-wire and the recurring adjacent loop-asset edit plus anomaly adjudication disappears (wiring `push-gate-fixtures.nu` this run required that adjacent edit, which landed in the evidence anomaly section). Distinct from open seeds-7e0f (untracked-file derivation) and from open overflow "Gate self-test fixture battery for touched-crate derivation" (01M31EVYN6FD7CD6198J1J7XDS.md line 20 — adds a battery, doesn't discover batteries).

### Add a nushell skill and gate the implementer on it
overflow: Add a nushell skill and gate the implementer on it — add `.fabro/skills/nushell/SKILL.md` distilled from mx-d157ad/mx-d0eb14 (auto-discovered via skills="discover") plus one sentence in `.fabro/workflows/develop/prompts/implementer.md` mirroring the Rust rule (read the nushell skill first when the seed touches `*.nu`); effect: the multiline-bare-argument parse failure class stops recurring in the costliest stage (3 failed shell calls + 3 wasted LLM rounds = $0.168 of $0.240 run cost, lesson then recorded twice). No open seed or overflow covers it.

### Declare engine dedup keys in planner context_allow_keys
overflow-dup: Add output.planner to planner context_allow_keys (open in 01M34N0KNQSGJQVXN1A6EDTJG1.md line 26) — identical change; this run re-confirms (worker log seq 85 `context_update_dropped: output.planner`). Extend the fix to cover `response.planner` too.

### Quiet prompt-lint false alarms and support pinned-until markers
overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in 01M324AZVMHDSCSAMVP4WTZR1R.md line 26) — identical theme; this run re-confirms (10 routing-named false warnings + 1 real date-pin warning).

### Handoff seed: engine noise — codex probe, unsupported_control, strict PR JSON
overflow: Handoff seed for engine noise — codex probe, unsupported_control, strict PR JSON — file a handoff-type seed (precedent seeds-800d) directing the operator to push fabro-repo follow-ups: skip the `.codex/instructions.md` probe quietly, add unsupported_control to the zai restraint list, make non-strict JSON the default fallback for PR content generation; effect: worker logs become triageable and PR-body generation stops depending on a silent retry (6× ERROR codex-probe, 16× WARN unsupported_control, 1× strict-JSON failure this run). Partially overlaps open overflow "Commit a minimal .codex/instructions.md" (01M35GF84EPCHM7FD7AHPDCEG0.md line 28, seeds-repo file fix) and "Fix PR-content generation fallback in workflow.toml" (same file line 22, workflow.toml fix) — the handoff targets the engine side, a different fix locus.
