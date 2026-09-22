# Revision — run 01M33BT0G3J6V2ECKDQ71DQ4PE

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M33BT0G3J6V2ECKDQ71DQ4PE.md
- seeds filed: seeds-c760 — Resolve reviewer node's inert skills=discover declaration (needs-user, capability-affecting, exempt from balance)
- balance: 0 non-exempt seeds filed / 0 — no credit this pass (seeds-c760 is needs-user, exempt per ADR-0022)
- basis: run 01M33BT0G3J6V2ECKDQ71DQ4PE, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit fbf8b219e296b24fcacbb24c47ee887ff220cc13
- revised_at_commit: fbf8b219e296b24fcacbb24c47ee887ff220cc13 (ADR-0015: engine drift signal for later judgement)

## Findings

### Fix tracker-guard.nu null-ts crash that fail-opens both guard arms — overflow-dup (not re-journaled)
overflow-dup: Fix tracker-guard.nu ts crash and restore the stale-claim requeue arm (open in 01M33374S3J9BC6M1FFTNWWAP9.md line 30). Same theme already open; no new `- overflow:` entry per the ledger rule.

### Add guarded subsumption branch to the planner two-branch rule + Surface basis_sha_resolvable per candidate — consolidated, overflowed (no credit this pass)
- overflow: Planner preflight arms: subsumption branch + basis-sha resolvability — in `.fabro/workflows/develop/prompts/planner.md` (two-branch rule, fabro-d183) and `.fabro/workflows/develop/scripts/planner-preflight.nu`, (arm a) add branch (c): when a merged run-PR commit of another seed (Fabro-Run trailer + `(#n)` subject — mechanically checkable) provably touches the candidate's named paths and the planner judges the criteria subsumed, superseded-close naming that sha and route 'Already landed'; have preflight list such subsumption candidates per seed row; (arm b) add a `basis_sha_resolvable` field per candidate row (one `git cat-file -e <sha>`) plus one planner.md line: an in-clone-unresolvable basis sha means judge the current tree only — never probe for the sha; effect: ~$0.35 + ~3 min + one content-free PR saved per stale-suffix seed (seeds-9fa3's fix had already landed via 254ceda) and 2-3 LLM rounds saved per stale-basis candidate (run 01M33BT0G3J6V2ECKDQ71DQ4PE planner's `git show 7327f847…` died with `fatal: bad revision`).

### Resolve the reviewer node's inert skills=discover declaration — filed as seeds-c760
Capability-affecting (ADR-0019): tool-allowlist change, `needs-user,revision` labels, stays for user assignment; cross-references open seeds-9fa3 (readability mechanism, not superseded).
