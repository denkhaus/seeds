# Revision — run 01M32NGRSM8BCP0EGZTJS1WCDQ

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M32NGRSM8BCP0EGZTJS1WCDQ.md
- seeds filed:
  - seeds-37a6 — Wire a live workflows-permission source for the seeds-7e57 publish_blocked_risk arm (capability-affecting, needs-user)
  - seeds-81cd — Gate closeout residual-seed filing on an actionable marker
  - seeds-37bc — Filter the planner's in-flight runs_list server-side (engine) and point planner.md at it
- balance: 2 non-exempt seeds filed (seeds-81cd, seeds-37bc) / 2 same-pass superseded closes: seeds-ad6c, seeds-436a (both @fabro-owned, superseded by seeds-81cd). seeds-37a6 is needs-user (ADR-0018 D3) and sits outside the balance per ADR-0022.
- basis: run 01M32NGRSM8BCP0EGZTJS1WCDQ, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit 53afd4bbd951371f781ceb9a4a7808151fa895c3
- revised_at_commit: 53afd4bbd951371f781ceb9a4a7808151fa895c3 (ADR-0015: engine drift signal for later judgement)

## Findings

### Wire a live workflows-permission source (seeds-7e57 arm) — filed as seeds-37a6
Capability-affecting (ADR-0019): bakes a binary into `.fabro/Dockerfile.toolchain` or sets workflow env. Filed `needs-user,revision` with engine-mediated, read-only fix directions only (declarative marker file or user-provisioned env config; no raw gh+token). Effect: the publish_blocked_risk arm actually fires on revoked permissions instead of failing open every run.

### Gate closeout residual filing on an actionable marker — filed as seeds-81cd
Change: `.fabro/workflows/develop/scripts/closeout.nu` residual sweep files only `residual:`-marked observations; plain correct/not-blocking notes go to journal. Rehydrates open overflow line 24 of `01M30FZ05QPQNMTYPWHKPJBE8J.md`. Consume of that ledger entry FAILED — `overflow-ledger.nu` validates seed ids against the pre-cutover `fabro-xxxx` prefix and rejects `seeds-81cd`; the entry remains open in the ledger, next pass must NOT re-file it (already covered by seeds-81cd). Effect: stops minting unimplementable seeds; funded this pass's balance by supersession-closing seeds-ad6c and seeds-436a (reason-before-close chained, closure notes in the tracker records).

### Filter planner in-flight runs_list server-side — filed as seeds-37bc
Change: server-side filter params on the `fabro_runs_list` engine tool (non-terminal status, open-PR, repo match) + `planner.md` step 4 pointer; consolidates the open client-side overflow line 19 of `01M31EVYN6FD7CD6198J1J7XDS.md` (created_since bound). Consume likewise FAILED on the stale `fabro-` prefix — do not re-file that entry. Effect: ~$0.07 + 25–30s per develop run, planner cache prefix preserved.

### Add a nushell-pitfalls skill gate for the implementer — overflow (no credit left)
- overflow: Add a nushell-pitfalls skill gate for the implementer on loop-asset seeds — vendor `.fabro/skills/nushell-pitfalls/SKILL.md` seeded from lesson mx-d157ad plus the two observed traps (nu 0.115 `complete` misses missing-binary spawn errors; sourcing a script with `def main` auto-invokes it), and extend the read-the-guide-FIRST hard gate in `.fabro/workflows/develop/prompts/implementer.md` to loop-asset/nushell seeds; effect: ~10–15s saved per avoided error round, fewer blind re-runs (run 01M32NGRSM8BCP0EGZTJS1WCDQ implementer: 5 shell errors across 23 calls on exactly these two traps).

### Enforce bulleted current_seed_brief shape — overflow (no credit left)
- overflow: Enforce bulleted current_seed_brief shape in the planner output schema — add a `"pattern"` on `current_seed_brief` in `.fabro/workflows/develop/schemas/planner-output.schema.json` requiring newline bullets (`\n- `), same teeth as the existing gate-command ban; effect: implementer PASS/FAIL checking becomes mechanical and misparse-driven review ping-pong risk drops (this run's brief shipped as one run-on paragraph despite planner.md step 6).

### Consume output.preflight at the planner — overflow-dup (already open)
- overflow-dup: Stop rendering preflight JSON twice in preambles (open in 01M31Z2XTPMMA78KHR4D9K84E6.md line 28) — this pass's finding adds the `context_consume_keys` variant on the planner node in `workflow.fabro` (fabro-699f pattern) as an alternative to `preamble_stages_ignore`; the open entry already covers the theme, next pass may consolidate both arms into one filing.

### Suppress by-design prompt-lint warnings — overflow-dup (already open)
- overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in 01M324AZVMHDSCSAMVP4WTZR1R.md line 26) — this run re-demonstrated it (11 warnings, 10 routing-named + date pin); no new entry needed.
