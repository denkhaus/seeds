# Revision — run 01M34N0KNQSGJQVXN1A6EDTJG1

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M34N0KNQSGJQVXN1A6EDTJG1.md
- seeds filed: none — healthy run (zero credit this pass, survivors journaled as overflow)
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M34N0KNQSGJQVXN1A6EDTJG1, workflow version 725a90c36ca780e6d3cc4bd915d06c17a06ae7243c691edc70b0bdad5ef0dd82, commit 5cda5ee431aabef5473bcc1e3eb111d482e6b0c5
- revised_at_commit: 5cda5ee431aabef5473bcc1e3eb111d482e6b0c5 (ADR-0015: engine drift signal for later judgement)

## Findings

### Exclude terminal runs in planner-preflight in-flight arm (priority 1)

- overflow-dup: Stop planner-preflight journal-claims treating seed mentions as claims (open in 01M34KAAZWZFEJRSNR36KWD9T1.md, line 26 — same theme: provably-terminal runs must not hold ready candidates in the in-flight arm; also open as "Exclude engine-terminal runs in planner-preflight in-flight arm", 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 16). Concrete change from this run: exclude runs whose catalog status is terminal (failed/succeeded without an open PR), not just "branch exists"; expected effect: no false in-flight skips of claimable seeds (this run: seeds-81cd held in_flight via failed run 01M34FT28TA8SMPWM8ZW7R9KN3).

### Stop anchor-arm false positives on prose slash-pairs (priority 2)

- overflow: Anchor-arm prose slash-pair false positives — in `.fabro/workflows/develop/scripts/planner-preflight.nu` anchor arm, don't flag a slash-joined token when its components resolve as existing repo files (or restrict anchor extraction to fenced/quoted paths); effect: `analyze.md/file.md` (prose naming two files that exist under `.fabro/workflows/revisor/prompts/`) stops costing one planner adjudication round plus manual re-verification per residual seed (~$0.02/~8s each), `anchors_ok` stays trustworthy. Distinct from open planner-preflight anchor overflows (bare-path retry 01M30FZ05QPQNMTYPWHKPJBE8J.md line 23, mid-token regex truncation 01M326WXXVST02QX4QKQQW9NE7.md line 19) — different mechanism: prose slash-joining, not path resolution or truncation.

### Baseline-suppress known-intent prompt-lint warnings (priority 2)

- overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in 01M324AZVMHDSCSAMVP4WTZR1R.md, line 26 — same theme and same concrete change: baseline/allowlist for `planner-output.schema.json` and `develop-output.schema.json` warnings; this run's tester rendered 11 standing warnings into the reviewer preamble, hiding new ones).

### Add output.planner to planner context_allow_keys (priority 2)

- overflow: Add output.planner to planner context_allow_keys — in `.fabro/workflows/develop/workflow.fabro`, add `output.planner` to the planner node's context_allow_keys (or stop recording the key); effect: one less run.notice warn per planner pass (this run seq 75) and the fabro-e47c producer-declaration context-contract lint becomes self-consistent. No open seed or open overflow covers planner `context_allow_keys` drift (seeds-a0fa/c760 are different nodes/mechanisms).
