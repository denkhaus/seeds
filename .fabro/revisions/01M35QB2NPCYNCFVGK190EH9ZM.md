# Revision — run 01M35QB2NPCYNCFVGK190EH9ZM

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M35QB2NPCYNCFVGK190EH9ZM.md
- seeds filed: none — zero credit this pass, all findings ride the overflow ledger
- balance: 0 non-exempt seeds filed / 0 — no credit this pass (no same-pass stale/superseded closes; every cited tracker seed is already closed)
- basis: run 01M35QB2NPCYNCFVGK190EH9ZM, workflow version 70db8097abcd619491dd6ba3be0b77ff5900638624f4f1797ac8ffffcf16d0d8, commit af593826abba6abf380090c47f07256ac961943c
- revised_at_commit: af593826abba6abf380090c47f07256ac961943c (ADR-0015: engine drift signal for later judgement)

## Findings

- Add a fixture smoke test for the gate's touched-crate/test derivation (seed `seeds-7e0f` follow-up regression test for `scripts/verify.nu` / `scripts/qualitygate.nu`): OVERFLOW-DUP — already open in `01M31EVYN6FD7CD6198J1J7XDS.md` line 20 ("Gate self-test fixture battery for touched-crate derivation"); the next pass with credit should file one consolidated seed covering both this run's union-behavior evidence (untracked test file, gate printed "no crates touched") and the checked-in fixture battery framing.
- Show the exact `ml record` invocation including `--resolution` in implementer step 6 (`.fabro/workflows/develop/prompts/implementer.md`): OVERFLOW-DUP — open in `01M326WXXVST02QX4QKQQW9NE7.md` line 25 (missing `--content` flag doc, same fix location and same class: ml record flag documentation); consolidate both flag corrections (`--content`, `--resolution`) into one seed when re-filed.
- Move the Rust verification policy out of implementer step 4 into the rust-style-guide skill: NEW OVERFLOW —
  - overflow: Move the Rust verification policy out of implementer step 4 into the rust-style-guide skill — compress step 4 of `.fabro/workflows/develop/prompts/implementer.md` to ~5 lines and move the ~1,900-word Rust gate policy into `.fabro/skills/rust-style-guide/` (already mandatory reading for Rust seeds); effect: ~3-4k input tokens saved per implementer pass with zero coverage loss on Rust runs (this run burned 16,948 input tokens / 81.5s inference vs 4.1s tool time on a 2-file Nushell diff where the policy was 100% irrelevant). Distinct from open prompt-size overflows (01M31Z2XTPMMA78KHR4D9K84E6.md line 25 cost-tiering, 01M35P6KYYAMB5CTYBTB153SRX.md line 30 reviewer facts trim, 01M35GF84EPCHM7FD7AHPDCEG0.md line 25 planner fast path) — different prompt, different mechanism (content relocation, not tiering or trimming).
- Tell the planner a clean preflight anchor table is final: OVERFLOW-DUP — open in `01M35GF84EPCHM7FD7AHPDCEG0.md` line 25 (clean-table fast path in `planner.md`); the trust sentence is one arm of that same consolidation candidate.
- Engine: fall back to text-encoded tool-result errors when the provider lacks the error flag (`pebble_coding_agent` in denkhaus/fabro): NEW OVERFLOW (cross-repo) —
  - overflow: Engine — fall back to text-encoded tool-result errors when the provider lacks the error flag — in `pebble_coding_agent` (denkhaus/fabro), when the provider protocol lacks the tool-result error flag, prefix the failure text into the tool result ("ERROR: ...") and emit the `unsupported_control` warning once per session instead of per round; effect: tool failures like the ml-record error reach the model with a usable signal, and 18-of-27 warn-line noise (9 implementer rounds x2) collapses to one per session. Cross-repo target — file as a fabro-side demand/pattern entry; no seeds-repo code change.
- Enable terminal notifications for the develop line (`[run.notifications.terminal]` in `.fabro/workflows/develop/workflow.toml`, currently disabled): OVERFLOW-DUP — open in `01M332792GNEPNR45VMEXV12WW.md` line 34 (fold into the resume procedure; vehicle seed seeds-d2c7 has since closed WITHOUT the change landing, so the next pass with credit should file it directly — recur count now 2).
- Add `output.planner` to the planner node's `context_allow_keys` (`.fabro/workflows/develop/workflow.fabro`): OVERFLOW-DUP — open in `01M32GNPT143Q82TJ217SNCC6A.md` line 27 AND `01M34N0KNQSGJQVXN1A6EDTJG1.md` line 26 (identical finding, twice-open); this run's notice (seq 67) is the third occurrence — highest-confidence single-line win in the ledger, consolidate to one seed when re-filed.

## Dedupe notes

- `duplicate_of`: none (no tracker-seed duplicates; matches were open overflows, handled as links above per the ledger rule).
- Supersession candidates: none.
