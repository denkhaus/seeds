# Revision — run 01M33WZF2KMHMP9W8RPJ0SB5X2

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M33WZF2KMHMP9W8RPJ0SB5X2.md
- seeds filed: none — zero balance credit this pass (no same-pass stale/superseded closes); both findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M33WZF2KMHMP9W8RPJ0SB5X2, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit 0906bca82d408cadd7e6448ee9a6123e3e61acf4
- revised_at_commit: 0906bca82d408cadd7e6448ee9a6123e3e61acf4 (ADR-0015: engine drift signal for later judgement)

## Findings

### Catch workflow.fabro parse errors in the implementer verify lane
- overflow: Catch workflow.fabro parse errors in the implementer verify lane — extend `scripts/verify.nu` (the fabro-6e7f dispatcher) with a parse-level tier for non-Rust diffs: when the diff touches `.fabro/workflows/*/workflow.fabro`, run the same `dot -Tcanon` check `scripts/qualitygate.nu` already runs; effect: the red class is caught pre-tester, saving ~3.5 min and ~$0.16 per occurrence. No open seed or open overflow covers the verify.nu loop-asset lane (tracker-checked; run closeout only filed the engine-acceptance follow-up). Basis: run 01M33WZF2KMHMP9W8RPJ0SB5X2 — unquoted dotted DOT attributes passed `just verify implementer` green (touched derives only lib/ and crates/ paths) and the tester went red at 06:39:32 on a workflow.fabro parse error (205 s / $0.156, 20% of run cost) to quote 4 attribute names.

### Raise reviewer preamble_inline_max_kb from 16 to 32
- overflow: Raise reviewer preamble_inline_max_kb from 16 to 32 — in `.fabro/workflows/develop/workflow.fabro` (reviewer stanza) raise `preamble_inline_max_kb` from 16 to 32, still under the 48 KB aggregate budget; effect: ~29 KB churn-heavy loop-asset evidence captures render inline, removing one reviewer tool round-trip per review. No open seed touches preamble rendering budgets (seeds-c760 covers skills declaration, a different mechanism); distinct from open evidence-blob overflows (01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 20, 01M3201Q8GTC0EPT0S5YMK0Z32.md line 25, 01M32GNPT143Q82TJ217SNCC6A.md line 19), which fix capture shape/paging, not the inline ceiling. Basis: run 01M33WZF2KMHMP9W8RPJ0SB5X2 — evidence@1 capture was 29,009 bytes, exceeded the 16 KB per-value ceiling, blob-ref'd, and forced the reviewer's only tool call (a `read_file` detour, 36 s stage).
