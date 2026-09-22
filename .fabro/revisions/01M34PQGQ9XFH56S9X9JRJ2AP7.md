# Revision — run 01M34PQGQ9XFH56S9X9JRJ2AP7

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M34PQGQ9XFH56S9X9JRJ2AP7.md
- seeds filed: none — zero balance credit this pass; all findings already open in the overflow ledger
- balance: 0 / 0 — no credit this pass
- basis: run 01M34PQGQ9XFH56S9X9JRJ2AP7, workflow version 725a90c36ca780e6d3cc4bd915d06c17a06ae7243c691edc70b0bdad5ef0dd82, commit 17af87d82c3a697332dbdb163eaa204178f5446a
- revised_at_commit: 17af87d82c3a697332dbdb163eaa204178f5446a (ADR-0015: engine drift signal for later judgement)

## Findings

### 1. Fix planner-preflight terminality proof so failed/orphaned runs stop flagging as in-flight

- overflow-dup: Exclude engine-terminal runs in planner-preflight in-flight arm (open in 01M30FZ05QPQNMTYPWHKPJBE8J.md, line 23 — the planner-preflight multi-arm that subsumes the in-flight arms of 01M2ZYVQ7CBSY3T0FEYBE9JK74.md lines 16-17; also adjacent: 01M34KAAZWZFEJRSNR36KWD9T1.md line 26 and open seeds-37bc, same failed run 01M34FT28TA8SMPWM8ZW7R9KN3).
- Concrete change: add a tip-age terminality signal in `terminal-tip?`/`in-flight-claims` (tip commit older than the tracker-guard stale threshold 6h = orphaned; shorten or tip-age-key the failed-tip grace below the conductor's 30-min fire interval) plus the planner.md step-4 journal wording. The re-filing pass should fold this arm into the pending planner-preflight multi-arm seed rather than filing a sibling.
- Expected effect: removes ~$0.05-0.10 and 30-45s per post-failure fire and the wrongful-skip hazard (seeds-81cd, run 01M34PQGQ9XFH56S9X9JRJ2AP7).

### 2. Print the mx-id from ml record at record time and pre-declare the conventions domain

- overflow-dup: ml record merge-on-name (open in 01M336MYEF1R5C1JMC6WHFTCQY.md, line 25 — same theme, mulch-CLI `ml record` behavior; the re-filing pass should consolidate the mx-id-print and domain pre-declare arms into one mulch seed alongside the pending upsert-semantics arm).
- Concrete change: `ml record` prints the upserted record's id on stdout; loop-side pre-declares the `conventions` domain in `.mulch/mulch.config.yaml`.
- Expected effect: one shell round saved per implementer pass; `lesson_capture: mx-xxxxxx` becomes mechanical instead of recovered by grepping `.mulch/expertise/conventions.jsonl`.

### 3. Allowlist deliberate routing-named schema warnings and the date-pin warning in prompt-lint

- overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in 01M324AZVMHDSCSAMVP4WTZR1R.md, line 26 — exact same finding, same files, same fabro-0a4c design basis).
- Concrete change: honor routing-intent declaration in schema `description`/marker key in `.fabro/scripts/prompt-lint.nu`; add acknowledgment marker for the `nightly-2026-04-14` date pin.
- Expected effect: gate output carries only real warnings, stopping warning-fatigue.
