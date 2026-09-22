# Revision — run 01M34KAAZWZFEJRSNR36KWD9T1

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M34KAAZWZFEJNR36KWD9T1.md (as reported in revision_findings)
- seeds filed: none — zero balance credit this pass; all findings journalled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M34KAAZWZFEJRSNR36KWD9T1, workflow version 162ca95d7a3f1770597185909b6440d719cbc8cda98ec495e60bde6760c902f7, commit aa5968913fb466e492154d752e9567121b93e73d
- revised_at_commit: aa5968913fb466e492154d752e9567121b93e73d (ADR-0015: engine drift signal for later judgement)

## Findings

### Run prompt-lint in the implementer verify lane for prompt-touching diffs
- overflow-to-journal (no credit; not journalled here)
- Change: extend touched-path derivation in `scripts/verify.nu` so diffs touching `.fabro/**/prompts/*.md` also run `nu .fabro/scripts/prompt-lint.nu`; optionally planner briefs "cite seed ids, never run ids" for prompt-edit criteria. Effect: prompt-lint failures surface in the implementer's cheap verify instead of gatebounce (~106.7s / $0.114 per occurrence). Complementary to closed seeds-9482 (crates only); distinct from the open verify-lane overflow (01M33WZF2KMHMP9W8RPJ0SB5X2.md line 13, workflow.fabro parse tier — same file, different lane).

### Stop planner-preflight journal-claims from treating seed mentions as claims
- overflow-to-journal (no credit; not journalled here)
- Change: in `.fabro/workflows/develop/scripts/planner-preflight.nu` (~166-181), match only genuine claim records, not arbitrary journal text; and/or drop in-flight mapping when the mapped run is provably terminal. Effect: removes ~45s false-positive triage lap (terminally-failed run 01M34FT28TA8SMPWM8ZW7R9KN3 held three ready candidates in_flight). Complementary to open seeds-a0fa and seeds-37bc (different arms); distinct from open preflight overflows (engine-terminal tip check, multi-arm, legend).

### Make the planner final output JSON-only with no preceding prose
- overflow-dup: Enforce bulleted current_seed_brief shape in the planner output schema (open in 01M32NGRSM8BCP0EGZTJS1WCDQ.md) — same planner-output-shape theme (schema enforces shape mechanically; the prose-before-JSON cap-retry detail merges into that arm when re-filed).

## Overflow ledger

- overflow: Run prompt-lint in the implementer verify lane for prompt-touching diffs — extend `scripts/verify.nu` touched-path derivation to run `nu .fabro/scripts/prompt-lint.nu` when the diff touches `.fabro/**/prompts/*.md`; effect: prompt-lint failures surface in the implementer's cheap verify instead of a gatebounce lap (~106.7s / $0.114, ~23% of run wall in 01M34KAAZWZFEJRSNR36KWD9T1); closed seeds-9482 covered only crate derivation; distinct from open verify-lane overflow 01M33WZF2KMHMP9W8RPJ0SB5X2.md line 13 (parse tier).
- overflow: Stop planner-preflight journal-claims treating seed mentions as claims — in `.fabro/workflows/develop/scripts/planner-preflight.nu` (~166-181) match only genuine claim records and/or drop in-flight mapping for provably-terminal runs; effect: removes the planner's false-positive triage lap (~45s, half of the most expensive node's cost in 01M34KAAZWZFEJRSNR36KWD9T1; terminally-failed 01M34FT28TA8SMPWM8ZW7R9KN3 falsely held three ready candidates); complementary to open seeds-a0fa/seeds-37bc, distinct from open preflight overflows.
- overflow-dup: Enforce bulleted current_seed_brief shape in the planner output schema (open in 01M32NGRSM8BCP0EGZTJS1WCDQ.md) — planner JSON-only-no-prose output contract (planner.md outcome contract; one output-cap retry ~11s in 01M34KAAZWZFEJRSNR36KWD9T1) folds into that arm when re-filed.
