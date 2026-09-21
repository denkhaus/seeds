# Revision — run 01M326WXXVST02QX4QKQQW9NE7

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M326WXXVST02QX4QKQQW9NE7.md
- seeds filed: none — zero credit this pass, all findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass (no same-pass stale or superseded closes)
- basis: run 01M326WXXVST02QX4QKQQW9NE7, workflow version e3c92fec9a9a6fd9256277d60e06e12dddcacf6779280c9df5c2b2825b813ea8, commit 7909ab10e7c1e58db53f0551db8bb2b73107d616
- revised_at_commit: 7909ab10e7c1e58db53f0551db8bb2b73107d616 (ADR-0015: engine drift signal for later judgement)

## Findings

### Sequence image repin behind revisor cutover: add dep edge seeds-8bb5 → seeds-b56f
- overflow: Sequence image repin behind revisor cutover: add dep edge seeds-8bb5 → seeds-b56f — run `seeds dep add seeds-8bb5 seeds-b56f` in the tracker so the toolchain-image rebuild/repin cannot be offered by `seeds ready` until the revisor workflow sd→seeds cutover closes; effect: prevents the revisor loop being stranded on an image without PATH sd (whole-sibling-workflow outage), zero code change. Basis: this run's reviewer journal and closeout filed both facts as independent seeds with no ordering guard; neither existing seed covers the dependency edge itself.

### Make develop evidence capture pageable: multi-line records in evidence.nu
- overflow-dup: evidence.nu: emit capture with real newlines so read_file paging works (open in 01M3201Q8GTC0EPT0S5YMK0Z32.md) — same concrete change (emit capture with real newlines between records in `.fabro/workflows/develop/scripts/evidence.nu` instead of one JSON-escaped line), same expected effect (reviewer can page the capture with `read_file` offset/limit; this run's reviewer could not read the middle ~72KB of a 119KB capture and compensated with re-derived repo greps).

### Fix planner-preflight anchor regex truncating cited paths mid-token
- overflow: Fix planner-preflight anchor regex truncating cited paths mid-token — fix the path-token regex in the anchor-extraction arm of `.fabro/workflows/develop/scripts/planner-preflight.nu` (it truncated `.fabro/Dockerfile.toolchain` to `.fabro/Dockerfile.toolchai` and flagged a false missing_file) and add a regression case to `.fabro/scripts/planner-preflight-anchor-fixtures.nu`; effect: anchors_ok becomes trustworthy and the planner stops burning an LLM round plus shell probes (~30–60s) re-adjudicating false positives. Distinct from the open planner-preflight multi-arm overflow (01M30FZ05QPQNMTYPWHKPJBE8J.md line 23, bare-path retry + PR in-flight) and the anchor_check.nu claim-extraction overflow (01M31Z2XTPMMA78KHR4D9K84E6.md line 16): this is a mid-token truncation bug, a different mechanism. Basis: twice-reported in this run (planner events seq 52–70 and implementer journal re-report).

### Add plan/template id read-path parity probe to seeds compat tests
- overflow: Add plan/template-id probe to seeds compat tests — add a plan/template-id probe to `crates/seeds/tests/compat.rs` against the fixture reference (mirroring the existing read_path_accepts_any_non_empty_id test) and relax the pl-/tpl-hex4 enforcement in `crates/seeds/src/model.rs` and `crates/seeds/src/id.rs` read paths only if the probe shows divergence from sd 0.5.15; effect: converts a known-unprobed gate-red risk into a sub-second deterministic test. Basis: this run's implementer journal records that seeds-3791's repair deliberately made only the SeedRecord path lenient and left plan/template paths strict and unprobed (lesson mx-eca078, no follow-up).

### Document required --content flag for ml record convention entries
- overflow: Document required `--content` flag for ml record convention entries — one-line doc edits in `AGENTS.md` (Mulch section) and `.fabro/workflows/develop/prompts/implementer.md` step 6 to include `--content` in the documented `ml record --type convention` form; effect: first-call success on the mandatory lesson-capture step instead of a recurring failed shell call in every run that records a convention. Basis: this run's implementer journal shows the only failed call was `ml record --type convention` rejected with "missing required flag(s): --content" while both docs show a form without it.

### Baseline the 11 known-intentional prompt-lint warnings
- overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in 01M324AZVMHDSCSAMVP4WTZR1R.md line 26) — same concrete change (allowlist/annotation in `.fabro/scripts/prompt-lint.nu` for the intentional `nightly-2026-04-14` pin and the routing-named schemas fabro-9ec3/fabro-0a4c), same expected effect (prompt-lint reports zero standing warnings so new gate-log warnings are actionable; this run's tester gate showed 40 files, 11 warnings, all known-intentional).
