# Improve review — run 01M30FZ05QPQNMTYPWHKPJBE8J

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (4.7 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-20 22:58+0000 by revisor `fabro_ask`

---

All evidence is in hand. Here are the recommendations, ordered by expected impact. Every claim is grounded in this run's events (`01M30FZ05QPQNMTYPWHKPJBE8J`), its stage journals, or workspace files I read. Existing seeds checked against `.seeds/issues.jsonl`: seeds-e218 (closed this run), seeds-facc, seeds-3791, seeds-48db, seeds-b291, seeds-cd75.

**Run baseline (from run events/conclusion):** 267.7s wall, $0.226 total. Implementer 139.2s/$0.130 (57% of cost), planner 71.9s/$0.062, reviewer 30.6s/$0.035. LLM spend is already well-tuned (all stages reasoning_effort=low, 356k of 415k tokens cache-read); the real losses this run were **deterministic-tool breakage**, which is where the recommendations concentrate.

---

### 1. The quality gate and `just verify` match zero crates in this repo — "gate green" asserted nothing about the changed tests
- **What happened:** the tester ran `just qualitygate` and printed `no crates touched`, finishing green in 5.6s (loop-asset lint + `cargo fmt --check --all` only) — while this run's entire diff was Rust test files (`crates/seeds/tests/real_fixture.rs` + fixtures). Root cause (from workspace file `scripts/qualitygate.nu:51,58`): touched-crate derivation filters `str starts-with 'lib/'` and parses `^lib/(?:apps|components|foundation)/…` — the denkhaus/fabro layout this repo was ported from. This repo uses `crates/<crate>/`. Same bug in `scripts/verify.nu:73–75,82`: the implementer's mandated mechanical lane printed "no lib/ crates touched — nothing to verify" (implementer journal painpoint #2), forcing a hand-run of fmt/clippy/full crate suite. The prior run's review (commit `2e65994`, "gate no-op … (#2)") flagged this as finding #1, and this run's planner journal explicitly left it unowned ("the review's own scope, not this seed's").
- **Change:** in `scripts/qualitygate.nu` and `scripts/verify.nu`, derive crates from `crates/<name>/**` (keep the `lib/…` arms if you want portability); gate on test-file touches too (`is-test-file` already exists in verify.nu).
- **Expected effect:** the deterministic tester actually executes clippy+nextest per touched crate; a red test suite can no longer ship through a green gate; the reviewer's standing assumption "the gate was green" becomes meaningful. Today correctness held only because the implementer deviated to run the suite manually.
- **Seed:** none of seeds-48db/cd75/facc/b291/3791 covers crate detection — **new seed justified: prior review filed the finding but never seeded it; planner journal confirms it is unowned.**

### 2. `dup-run-check.nu` is degraded in this repo — the duplicate-run guard is inoperative
- **What happened:** implementer journal painpoint #1: `nu .fabro/scripts/dup-run-check.nu seeds-e218 --self …` failed with `fatal: couldn't find remote ref denkhaus` — the script's fetch of the merge target (`origin/main`) uses a wrong remote/refspec (fabro-repo heritage). Every implementer pass in this repo runs this preflight and gets a degraded verdict; the claim-race family (fabro-22e4/fabro-6b58) has no working stopgap here.
- **Change:** fix remote-ref resolution in `.fabro/scripts/dup-run-check.nu` (resolve `origin/main` from the actual remote, not an owner-qualified ref).
- **Expected effect:** the ~1s duplicate-run preflight returns real verdicts; planner and implementer stop re-deriving landed-ness by hand (this run's planner spent 4 extra shell rounds + reasoning adjudicating the `duplicate` verdict).
- **Seed:** **new seed justified: no tracker entry covers dup-run-check at all.**

### 3. Auto-merge is enabled in run settings but rejected by GitHub — seeds close while their work never lands on `main`
- **What happened (from run events):** `auto_merge.status: failed` — GraphQL `UNPROCESSABLE: "Auto merge is not allowed for this repository"`. PR #3 (this run's approved, gate-green work) will sit open; meanwhile closeout already closed seeds-e218. The next seed (seeds-facc, now unblocked) builds on a format core that is *not* in `main` — the loop will develop against a base that lacks its own prerequisite.
- **Change:** either enable "Allow auto-merge" in the GitHub repo settings (user decision) or set `pull_request.auto_merge=false` in the develop run template so the loop stops relying on a merge that cannot happen; ideally have the planner's stale-basis/preflight treat "PR open, not merged" as in-flight until this is resolved.
- **Expected effect:** closed-seed state and `main` content stop diverging; no wasted next run re-adjudicating work that is approved-but-unmerged.
- **Seed:** **new-seed justification: engine/GitHub settings decision; no tracker entry covers PR merge flow** (the prior run review also noted it could not fix this from the workspace).

### 4. The reviewer cannot read the binding Rust style guide — the standards axis ran unaudited
- **What happened:** reviewer journal painpoint: `.fabro/skills/rust-style-guide/SKILL.md` is unreachable for its read tools, yet reviewer prompt step 3 makes reading it *before judging* a binding policy. The stage transcripts confirm it: the reviewer's tool allow-list resolved to `glob, grep, read_file, request_user_input` only (no `use_skill` despite `skills="discover"` discovering rust-style-guide), tool_time 2ms, one `read_file` call (the evidence blob). It approved a Rust diff (`real_fixture.rs`) without the guide.
- **Change:** in `workflow.fabro`'s reviewer node, either add `use_skill` to the tools allow-list (skill discovery already works) or ensure `.fabro/skills/**` is readable by its file tools (fs_hide exemption).
- **Expected effect:** the Rust standards axis becomes mechanically auditable; guide violations route Changes-requested instead of depending on the model's memory.
- **Seed:** **new seed justified: no tracker entry touches reviewer capability/tooling.**

### 5. The lesson-capture channel is dead (`ml` unusable) — durable lessons die in run journals
- **What happened:** implementer journal painpoint #3 + `lesson_capture` context key: `ml prime`/`ml record` fail with "No .mulch/ directory found", while `AGENTS.md:44–49` mandates both. This run's genuinely durable lesson (hermetic tracker fixture pairs) survived only as journal text; calls were burned attempting `ml`. seeds-3791 defers the mulch phase deliberately — so the mandate and reality diverge by design.
- **Change:** either `mulch init` in the bootstrap/toolchain image, or add one line to PROJECT_FACTS (`prompts/project-facts.md`) + the implementer prompt's Lesson capture section: "when `.mulch/` is absent, answer `nothing durable — skipped (mulch uninitialized)` without attempting `ml`."
- **Expected effect:** every implementer pass stops wasting 1–2 tool calls on a known-dead channel, and the skip answer stops looking like a policy violation.
- **Seed:** **new-seed justification: seeds-3791 covers the sd cutover, not the ml mandate/degradation mismatch.**

### 6. Preflight anchor checks probe the wrong root — false `missing_file` flags every run
- **What happened:** planner journal painpoint: anchor_flags reported `tests/compat.rs`, `tests/roundtrip.rs`, `tests/fixtures/repo_issues_sd_list.json` as `missing_file` because `.fabro/workflows/develop/scripts/planner-preflight.nu` checked them at repo root; the files exist at `crates/seeds/tests/`. The planner had to disprove the table by hand (`ls crates/seeds/tests` etc., seq 57–59) before trusting the `duplicate` verdict.
- **Change:** in `planner-preflight.nu`, probe `<crate>/`-prefixed candidates as a fallback when the bare path misses (same porting class as rec #1).
- **Expected effect:** the preflight verdict table becomes trustworthy; the planner's landed-ness adjudication drops ~2 re-verification rounds per run.
- **Seed:** **new seed justified: no tracker entry covers preflight anchor resolution.**

### 7. Closeout files non-actionable reviewer notes as P2 bug seeds — tracker noise (seeds-cd75)
- **What happened:** closeout's non-blocking sweep (`.fabro/workflows/develop/scripts/closeout.nu:197–252`) filed seeds-cd75: a P2 *bug*, assigned to fabro, whose whole content is "one volatile half-drift exists but is covered by the retained VOLATILE_FIELDS skip — implementer's claim slightly overstated, not blocking." There is nothing to build; it will now pollute every future `sd ready` candidate scan.
- **Change:** in closeout.nu, require an actionable marker for the sweep (e.g., only observations prefixed `residual:`, or emit them as a `note`-type seed not `bug`), keeping genuinely actionable findings.
- **Expected effect:** the planner pool stays clean; no future run wastes a claim cycle on a nothing-burger.
- **Seed:** **new-seed justification: seeds-cd75 is the artifact of the bug, not a seed for fixing the sweep.**

### 8. Tracker hygiene: seeds-48db is superseded by this run's landed fix
- **What happened:** seeds-48db ("real_fixture volatile-field drift breaks PR #1 CI") demands exactly what this run landed and got approved: hermetic co-captured fixture pair + retained VOLATILE_FIELDS skip (implementation_summary + reviewer verdict `approved`, PR #3). It is unassigned, so the develop line cannot close it.
- **Change:** close seeds-48db as superseded (`superseded: fix landed via PR #3, run 01M30FZ05QPQNMTYPWHKPJBE8J`) — a one-line user action.
- **Expected effect:** the backlog reflects reality; the next planner lap doesn't reconsider a satisfied demand.
- **Seed:** covered — the recommendation *names* seeds-48db itself.

**Not recommended (checked, already good):** reasoning_effort tiers (planner/implementer/reviewer all low, reviewer at $0.035 — steady state per prior calibration); prompt-budget tuning (preambles constant, caching at 86%); the evidence blob-ref detour (24.5KB capture cost the reviewer exactly one paged `read_file` — negligible this run, though rec #1's fix will shrink fixture diffs anyway).
