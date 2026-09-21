# Revision — run 01M32J3AFYGBV109Z58SHHBA1J

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M32J3AFYGBV109Z58SHHBA1J.md
- seeds filed: seeds-7e57 — Fail fast on publish-blocked workflow-file targets: planner-preflight arm + GitHub App workflows permission (needs-user)
- balance: 0 non-exempt seeds filed / 0 — no credit this pass (no same-pass stale/superseded closes)
- basis: run 01M32J3AFYGBV109Z58SHHBA1J, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit 7eab301a3f7415927b761509708c2a5876c16e33
- revised_at_commit: 7eab301a3f7415927b761509708c2a5876c16e33 (ADR-0015: engine drift signal for later judgement)

## Findings

### Fail fast on publish-blocked workflow-file targets (preflight arm + App workflows permission)
- filed: seeds-7e57 (labels `needs-user,revision`; capability-affecting per ADR-0019 — arm 2 changes what agent pushes may do; implementation awaits explicit user approval)
- concrete change: (a) publish_blocked_risk arm in `.fabro/workflows/develop/scripts/planner-preflight.nu` flagging seeds targeting `.github/workflows/**` so the planner routes Blocked at claim time; (b) operator grants the fabro GitHub App the `workflows` permission
- effect: no unpublishable full cycles (~$1.25, 16.5 min per occurrence), no closed-on-an-unpushed-branch tracker drift (seeds-25b5 re-opened on base)

### Include untracked files in verify.nu touched-crate derivation
- duplicate_of: open overflow `verify.nu: derive touched paths from git status --porcelain so untracked files count` (open in `01M3201Q8GTC0EPT0S5YMK0Z32.md` line 20) — same concrete change, still awaiting balance credit
- change: union `git ls-files --others --exclude-standard` (or `git status --porcelain`) into the touched[] derivation in `.fabro/scripts/verify.nu`; effect: test-only seeds can no longer silently bypass `just verify implementer`

### Require created_since on planner fabro_runs_list calls
- duplicate_of: open overflow `Bound planner in-flight fabro_runs_list with created_since=<now-48h>` (open in `01M31EVYN6FD7CD6198J1J7XDS.md` line 19) — same concrete change
- change: one-line edit in `.fabro/workflows/develop/prompts/planner.md` step 4; effect: ~$0.05 and ~20 s saved every run

### Keep reviewer evidence capture inline (pretty-print + raise preamble_inline_max_kb)
- overflow-dup: evidence.nu newline rendering (open in `01M3201Q8GTC0EPT0S5YMK0Z32.md` line 25)
- overflow-dup: Raise reviewer preamble_inline_max_kb (open in `01M32GNPT143Q82TJ217SNCC6A.md` line 19 — proposed 24; this pass would consolidate both arms and raise to 40, but both themes are already open, so no new overflow entry)
- effect when eventually filed: reviews verify from inline context, zero blob round-trips

### Mitigate cargo fingerprint staleness in verify.nu
- overflow: Mitigate cargo fingerprint staleness: touch changed sources before cargo in verify.nu — in `.fabro/scripts/verify.nu` (stage-implementer), touch the git-diff changed sources of each code-touched crate immediately before its cargo invocation (root fix is engine-side mtime normalization on file-tool writes, denkhaus/fabro); effect: removes the stale-binary re-emits-old-panic misdiagnosis class and ~100 s + 2 LLM rounds per Rust pass (this run's implementer: 829.6 s / $1.048)
- no open overflow and no seed matches (searches fingerprint/mtime/publish clean); zero credit this pass, so it rides the ledger
