# Revision — run 01M35GF84EPCHM7FD7AHPDCEG0

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M35GF84EPCHM7FD7AHPDCEG0.md
- seeds filed: none — zero balance credit this pass (0 same-pass stale/superseded closes), all surviving findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M35GF84EPCHM7FD7AHPDCEG0, workflow version e5dfb8c2b48eddbbf2dc1161498700bb6d7bf007d60c65b1c08e3ffb82cd3066, commit 335b177f5fb3ddad1ebcb2739381f40d394997a5
- revised_at_commit: 335b177f5fb3ddad1ebcb2739381f40d394997a5 (ADR-0015: engine drift signal for later judgement)

## Findings

### Emit a deterministic runs_list_skippable field in planner preflight
- overflow-dup: Bound planner in-flight fabro_runs_list with created_since=<now-48h> (open in `.fabro/revisions/01M31EVYN6FD7CD6198J1J7XDS.md` line 19) — same theme (planner `fabro_runs_list` cost, both target `planner.md` step 4), different mechanism (deterministic skippable flag from preflight vs server-side time bound). Next pass should decide one mechanism or consolidate both arms into one preflight seed.

### Resolve loop-asset filenames in anchor_check.nu
- overflow: Resolve loop-asset filenames in anchor_check.nu — extend `resolve-anchor-path` in `.fabro/workflows/develop/scripts/anchor_check.nu` with loop-asset search roots (`.fabro/workflows/develop/{scripts,prompts,schemas}`, `.fabro/scripts`, `scripts/`) mirroring the member-src-roots pattern, and report the resolved base; effect: stops false `missing_file` flags on loop-asset seeds (this run falsely flagged `seeds-731d` (`evidence.nu:38`) and `seeds-7212` (`workflow.fabro:399`), both files exist — the next claims of those open P2 candidates hit the same flag). Distinct from the open anchor_check.nu claim-extraction overflow (`01M31Z2XTPMMA78KHR4D9K84E6.md` line 16 — backtick citation parsing, different mechanism).

### Allowlist by-design prompt-lint warnings
- overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in `.fabro/revisions/01M324AZVMHDSCSAMVP4WTZR1R.md` line 26) — same theme, same mechanism (checks 3/5 of `.fabro/scripts/prompt-lint.nu`, allowlist routing-named keys in `*/schemas/*-output.schema.json` plus reviewed marker for the pinned `nightly-2026-04-14` date warn); this run's evidence (11 intentional warnings) reconfirms it.

### Fix PR-content generation fallback in workflow.toml
- overflow: Fix PR-content generation fallback in workflow.toml — in `.fabro/workflows/develop/workflow.toml` `[run.pull_request]`, drop the LLM body step in favor of the deterministic skeleton the engine already produces as salvage, or route generation through a JSON-reliable path and update the stale 'clean JSON' comment on the `zai:glm-4.7` setting; effect: ~10–20s off every run's terminal tail and consistent PR bodies (this run's logs at 21:33:42/21:33:49: structured generation failed, non-strict retry was not JSON, prose salvaged with a deterministic title — two wasted LLM calls). No open seed or open overflow covers the PR-content generation path.

### Add a clean-table fast path to planner.md
- overflow: Add a clean-table fast path to planner.md — prepend a ~10-line CLEAN TABLE FAST PATH to `.fabro/workflows/develop/prompts/planner.md` (healthy table + top candidate `clean`/`anchors_ok` + no `review_feedback` → `seeds show` → claim → bulleted brief), keeping the full 32-line procedure for dirty cases; effect: shrinks the dominant planner stage's reasoning scope (this run's planner burned 41.7s / 35.5k input tokens on a healthy table to produce a 682-token claim — 49% of run cost, 35% of wall time). Distinct from open `seeds-7212` (reviewer prompt), open `seeds-37bc` (engine filter), and open planner.md overflows (token bounding, probe discipline, sweep authority — different mechanisms).

### Commit a minimal .codex/instructions.md
- overflow: Commit a minimal .codex/instructions.md — commit a one-line `.codex/instructions.md` at the repo root pointing to `AGENTS.md`; effect: removes 6 ERROR-level lines per run (2 per agent session) so severity-ERROR monitoring pages only on real failures (this run's worker logs logged `File "/workspace/seeds/.codex/instructions.md" was not found` at ERROR twice per session init — 6 of 16 warn-or-worse lines; closed `seeds-7cd1` documented the noise but filed no fix). No open seed or open overflow covers this.
