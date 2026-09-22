# Revision — run 01M354EMKT2P2XD1QSC51NJSTM

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M354EMKT2P2XD1QSC51NJSTM.md
- seeds filed: none — zero balance credit this pass (no same-pass stale/superseded closes); surviving findings journaled below
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M354EMKT2P2XD1QSC51NJSTM, workflow version 725a90c36ca780e6d3cc4bd915d06c17a06ae7243c691edc70b0bdad5ef0dd82, commit 452b3c42ad7180ee35e9903d498f9213cc5951dc
- revised_at_commit: 452b3c42ad7180ee35e9903d498f9213cc5951dc (ADR-0015: engine drift signal for later judgement)

## Findings

- Loop-asset verify lane for `scripts/verify.nu` — overflow-dup: Catch workflow.fabro parse errors in the implementer verify lane (open in 01M33WZF2KMHMP9W8RPJ0SB5X2.md line 13). This run's variant generalizes it: a loop-asset-only diff (`.fabro/workflows/**`, `.fabro/scripts/**`) should dispatch the same graphviz parse plus prompt-lint the gate runs (e.g. `.fabro/scripts/graph-parse.nu`); effect: malformed workflow graphs fail at the implementer in seconds instead of a gate-red bounce cycle.

- Silence chronic error-channel noise — multi-arm finding; per-arm disposition:
  - `.codex/instructions.md` absence — overflow-dup: Quiet the .codex/instructions.md absence probe in worker logs (open in 01M32GNPT143Q82TJ217SNCC6A.md line 31). This run: 6 ERROR lines.
  - planner dedup/context_allow_keys drift — overflow-dup: Add output.planner to planner context_allow_keys (open in 01M34N0KNQSGJQVXN1A6EDTJG1.md line 26).
  - conditional gatebounce WARN — survives, journaled below as new overflow.

- overflow: Mark output.gatebounce conditional in the develop graph — in `.fabro/workflows/develop/workflow.fabro` preamble config, declare the `output.gatebounce` key conditionally (or add it to an allow-keys variant for gatebounce outcomes) so green runs stop emitting `preamble_allow_keys entry absent node=implementer key=output.gatebounce` WARN; effect: empty warn/error worker logs again mean nothing wrong (seeds-7cd1 silent-stall night incident is the demonstrated downside). Basis: run 01M354EMKT2P2XD1QSC51NJSTM worker logs, 1x WARN, absent by design on green runs.

- Classify cross-repo lore anchors as expected in planner-preflight — overflow: Classify cross-repo lore anchors as expected in planner-preflight — in `.fabro/workflows/develop/scripts/planner-preflight.nu`, classify anchor paths under the documented fabro-lore namespaces (`docs/lab/adr/`, `docs/lab/fabro/`) as `cross_repo_expected` instead of `missing_file`; effect: removes one recurring planner LLM adjudication round (~5-10s) per run while seeds-a0fa (which cites such anchors) remains a top `seeds ready` candidate (planner journal seq 53 this run). Distinct from open preflight anchor overflows (bare-path retry 01M30FZ05QPQNMTYPWHKPJBE8J.md line 23, regex truncation 01M326WXXVST02QX4QKQQW9NE7.md line 19, slash-join 01M34N0KNQSGJQVXN1A6EDTJG1.md line 18): those fix path resolution, this classifies cross-repo namespaces.

- Allowlist by-design prompt-lint warnings — overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in 01M324AZVMHDSCSAMVP4WTZR1R.md line 26). Same concrete change: allowlist for `planner-output.schema.json`/`develop-output.schema.json` routing-named properties plus a reviewed marker for the load-bearing `2026-04-14` pin in `project-facts.md`; effect: gate log stops shipping the same 11 by-design warnings every run.
