# Revision — run 01M30FZ05QPQNMTYPWHKPJBE8J

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M30FZ05QPQNMTYPWHKPJBE8J.md
- seeds filed: seeds-9482 — Quality gate: derive touched crates from `crates/<name>/**` in `scripts/qualitygate.nu` and `scripts/verify.nu` (multi-arm, subsumes overflow line 14 of `01M2ZYVQ7CBSY3T0FEYBE9JK74.md`); seeds-9fa3 — Reviewer node: make rust-style-guide readable (needs-user, capability-affecting, exempt from balance)
- balance: 1 non-exempt seed filed / 1 same-pass stale close — seeds-cd75 (non-actionable residual note, @fabro-owned)
- basis: run 01M30FZ05QPQNMTYPWHKPJBE8J, workflow version e3c92fec9a9a6fd9256277d60e06e12dddcacf6779280c9df5c2b2825b813ea8, commit 7327f84768922f5c3c6ec1054280275bceb6e95a
- revised_at_commit: 7327f84768922f5c3c6ec1054280275bceb6e95a (ADR-0015: engine drift signal for later judgement)

## Findings

### Filed

- **Quality gate crate detection** — filed seeds-9482 (P1, multi-arm: `scripts/qualitygate.nu` derive crates from `crates/<name>/**`, same for `scripts/verify.nu` incl. test-file touches, loud failure on empty crate set with `*.rs` diffs). Effect: the tester gate actually runs clippy+nextest per touched crate; a red suite can no longer ship through a green gate. Absorbed open overflow line 14 (same change, `Cargo.toml`-members variant noted there). Prior review finding #1 (commit `2e65994`) was never seeded — now covered.
- **Reviewer style-guide access** — filed seeds-9fa3 (P2, needs-user + revision, ADR-0019): add `use_skill` to the reviewer node tools or exempt `.fabro/skills/**` from fs_hide for reviewer read tools. implementation awaits explicit user approval. Exempt from the filing balance (needs-user).

### Closed (same-pass credit)

- seeds-cd75 — stale close: non-blocking volatile half-drift covered by the retained VOLATILE_FIELDS skip; no actionable work. Reason-before-close note appended to the tracker record.

### Overflow (survived dedupe, no balance credit)

- overflow: planner-preflight multi-arm — in `​.fabro/workflows/develop/scripts/planner-preflight.nu`, (a) retry bare-path anchor misses with `<crate>/`-prefixed candidates (this run reported `tests/compat.rs` etc. as missing_file though they live under `crates/seeds/tests/`), (b) treat approved-but-unmerged PRs as in-flight, not landed-or-duplicate (auto_merge failed: GraphQL UNPROCESSABLE, run events); subsumes the in-flight/rescue-ref arms of open overflows lines 16-17 of `01M2ZYVQ7CBSY3T0FEYBE9JK74.md`; effect: preflight verdict table becomes trustworthy, ~2 planner re-verification rounds per run drop.
- overflow: closeout sweep files only actionable residuals — in `​.fabro/workflows/develop/scripts/closeout.nu` (lines 197-252), require an actionable marker (e.g. `residual:` prefix) or emit non-actionable notes as a non-bug type; effect: planner pool stays clean, no claim cycle wasted on nothing-burgers like seeds-cd75.
- overflow-dup: Fix `dup-run-check.nu` remote-ref resolution (open in `01M2ZYVQ7CBSY3T0FEYBE9JK74.md`, line 19 — resolve merge target via `origin` + `refs/heads/main` instead of owner-qualified ref; reproduced this pass, verdict degraded "fetch failed").
- overflow-dup: Document the no-`.mulch/` ml skip in PROJECT_FACTS and implementer prompt (open in `01M2ZYVQ7CBSY3T0FEYBE9JK74.md`, line 18 — this pass adds the lighter variant: when `.mulch/` is absent, answer "nothing durable — skipped (mulch uninitialized)" without invoking `ml`).

### Not pursued

- seeds-48db (real_fixture volatile drift): stale candidate — hermetic co-captured fixture pair landed via PR #3 (`6b87b00`) — but seed is unassigned; revisor cannot close (ADR-0018 D2). Left for the human gate / closeout.
