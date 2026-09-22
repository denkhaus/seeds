# Revision — run 01M35KWYKJDAZCB65TTJPERGC5

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M35KWYKJDAZCB65TTJPERGC5.md
- seeds filed: none — healthy run; zero filing credit this pass (no same-pass stale/superseded closes), all five findings recorded below
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M35KWYKJDAZCB65TTJPERGC5, workflow version c0e3dc086cfa80e8a79408dcf1552842ecd26999e9fc7b8e093985a779cb83c4, commit a72de60a2e3bb95637e85bdc8b7f7fe7a0c9bf1d
- revised_at_commit: a72de60a2e3bb95637e85bdc8b7f7fe7a0c9bf1d (ADR-0015: engine drift signal for later judgement)

## Findings

- overflow-dup: Fix anchor resolver's false missing_file on loop-asset paths (open in 01M35GF84EPCHM7FD7AHPDCEG0.md, line 16 — same change: `resolve-anchor-path` loop-asset search roots in `.fabro/workflows/develop/scripts/anchor_check.nu`)
- overflow-dup: Allowlist intentional prompt-lint warnings to end warning fatigue (open in 01M324AZVMHDSCSAMVP4WTZR1R.md, line 26 — same change: routing-named schema suppression + date-pin reviewed marker in `.fabro/scripts/prompt-lint.nu`)
- overflow-dup: Declare output.planner in the planner node's context_allow_keys (open in 01M34N0KNQSGJQVXN1A6EDTJG1.md, line 26 — same change: `workflow.fabro` planner node `context_allow_keys`)
- overflow: Gate diff-walk HARD_CAP/no-silent-drop behavior with a permanent smoke fixture — add `.fabro/scripts/diff-walk-smoke.nu` (scratch-repo fixture: large new file + small edit → assert no `hard cap hit` and omitted=0; 1.5 MB blob → assert cap trips) and wire it into the loop-asset arm of `scripts/qualitygate.nu` beside lint-nu; effect: the just-fixed seeds-731d invariant becomes deterministically gated, reviewers stop approving self-reported fixture claims (tracker-checked: no diff-walk/smoke-fixture seed, seeds-731d closed and scoped to the discount, seeds-7e0f is a different mechanism)
- overflow: Chain ml record and ml search into one shell call for lesson capture — in `.fabro/workflows/develop/prompts/implementer.md` (Lesson capture section), prescribe `ml record ... && ml search <name> --format json` as ONE shell call, mirroring the documented `seeds update --format` observed-failure pattern; effect: one shell round and one LLM round saved per implementer pass (`ml record` prints no mx-id), lesson_capture stops depending on a second lookup that can pick a duplicate stub (tracker-checked: no mulch/lesson seed; distinct from open `--content` doc overflow 01M326WXXVST02QX4QKQQW9NE7.md line 25 and the mulch-init overflow 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 18 — different mechanisms)
