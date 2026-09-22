# Revision — run 01M338CGYHE24Y5KXBXMB8DJWR

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M338CGYHE24Y5KXBXMB8DJWR.md
- seeds filed: none — zero balance credit this pass; all surviving findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass (no same-pass stale/superseded closes)
- basis: run 01M338CGYHE24Y5KXBXMB8DJWR, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit 4d0d24db5b1f062659255b198d89b9fe4030c2d8
- revised_at_commit: 4d0d24db5b1f062659255b198d89b9fe4030c2d8 (ADR-0015: engine drift signal for later judgement)

## Findings

### Fix tracker-guard.nu empty-glob/missing-ts crash and add a runtime smoke to the loop-asset gate
- overflow-dup: Fix tracker-guard.nu ts crash and restore the stale-claim requeue arm (open in `.fabro/revisions/01M33374S3J9BC6M1FFTNWWAP9.md`)
- Concrete change: `glob`/try-wrapped `ls` plus `$recs | get ts? | last` in `.fabro/workflows/develop/scripts/tracker-guard.nu`; add one runtime smoke case (empty-journal fixture) to the loop-asset section of `scripts/qualitygate.nu`. Effect: guard node green, requeue arm reachable, gate sees runtime regressions. The gate-smoke arm is NEW relative to the open overflow — fold it into that entry when re-filed.

### Emit evidence capture as pageable multi-line output in evidence.nu
- overflow-dup: evidence.nu: emit capture with real newlines so read_file paging works (open in `.fabro/revisions/01M3201Q8GTC0EPT0S5YMK0Z32.md`)
- Concrete change: pretty-print the JSON or emit raw text with newlines in `.fabro/workflows/develop/scripts/evidence.nu`. Effect: reviewer offset/limit paging works on ~100 KB captures; removes approval-on-unread-evidence risk (this run: 2,328-line diff approved only via HEAD-file recovery).

### Force rebuild of changed crates in verify.nu before nextest dispatch
- overflow-dup: Mitigate cargo fingerprint staleness: touch changed sources before cargo in verify.nu (open in `.fabro/revisions/01M32J3AFYGBV109Z58SHHBA1J.md`)
- Concrete change: touch changed sources (or `cargo clean -p <crate>`) before nextest dispatch in `scripts/verify.nu`. Effect: eliminates the stale-binary misdiagnosis class (recurred this run despite prompt rule and lesson mx-876053).

### Route model.rs set_description(None)/set_assignee(None) through remove_field (shift_remove)
- overflow: Route model.rs set_description(None)/set_assignee(None) through remove_field (shift_remove) — route the two `None` branches in `crates/seeds/src/model.rs` through `remove_field` (shift_remove) with a differential case pinning byte-identical stores; effect: closes the last latent deletion-path key-reorder divergence from sd left by run 01M338CGYHE24Y5KXBXMB8DJWR's remove_field fix (searches shift_remove/model find no covering seed). Not duplicated in the open ledger.
