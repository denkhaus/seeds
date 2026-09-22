# Improve review — run 01M3565E7X0BKSTWERSX959JWG

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (20.5 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 18:57+0000 by revisor `fabro_ask`

---

# Run 01M3565E7X0BKSTWERSX959JWG — improvement recommendations

**What this run was** (from run events/checkpoints): seed `seeds-0dfd` (2,200-line command-layer lift, +3,050/−2,100 across 15 files), all stages first-pass green, PR #47, 20m33s wall, ~$2.34 LLM cost. Cost/time concentration: implementer@1 = 1,036s active / **$2.108 (90% of run cost)**; planner@1 = 71s / $0.160; reviewer@1 = 64s / $0.074; tester = **10s** (implementer left the tree compile-warm — the role split is working). Tracker state: only `seeds-800d` (operator repair, unassigned) and `seeds-a0fa` (open) remain open; `seeds-0dfd` closed by this run. **None of the findings below are covered by either open seed**, so each carries a new-seed justification.

---

**1. Raise/stratify the evidence capture cap — 2 of 15 diff files reached review as UNSEEN** *(evidence pipe, highest review-integrity impact)*
- **What happened** (from reviewer journal + evidence stage): the capture was 128.6 KB, `diff-walk` stopped at `HARD_CAP` (`evidence.nu:38`, `const HARD_CAP = 128000`) and disclosed "2 of 15 seed-work files omitted"; the reviewer's blob read *also* truncated (~20k tokens omitted), so approval rested on direct worktree reads — a workaround that only works when the tree is clean (it would misfire on a bounce cycle).
- **Change**: `.fabro/workflows/develop/scripts/evidence.nu` — count new-file diffs (verbatim file content, poor compression) at a discounted cost toward `HARD_CAP`, or emit per-file blob chunks the reviewer pages; keep the cap as pathological-input safety only, as its own comment intends.
- **Expected effect**: every seed-work file is reviewable; approvals stop depending on the clean-worktree workaround.
- *New-seed justification: no open seed covers the evidence-capture cap — the reviewer's own fix-idea from this run's journal is unfiled.*

**2. Tell the reviewer it CAN read the rust-style-guide — it judged a 3,050-line Rust diff without the binding standards axis** *(prompting defect, proven by contradiction with the graph)*
- **What happened** (reviewer journal painpoint #2): the reviewer believed `.fabro/skills/rust-style-guide/SKILL.md` was fs_hide-bound and "judged conformance from repo conventions and grep-level checks instead." But `workflow.fabro:399–408` gives the reviewer node **deliberately NO fs_hide** and `tools="read_file,grep,glob,use_skill"`. The misleading text is the PROJECT_FACTS loop-asset bullet (".fabro/ … FILE TOOLS fail on them"), which is unconditional and shared by all nodes; `reviewer.md`'s RUST STANDARDS AXIS section never states the node exemption.
- **Change**: one sentence in `.fabro/workflows/develop/prompts/reviewer.md` (Rust standards axis): "This node carries no fs_hide (`workflow.fabro`) — read the guide directly via `read_file`/`use_skill`; the PROJECT_FACTS loop-asset bullet does not bind this node's reads."
- **Expected effect**: the binding guide is actually loaded on Rust reviews; guide-violation findings become possible instead of memory-based.
- *New-seed justification: seeds-9fa3 (mechanical readability) and seeds-c760 (use_skill allow-list) are both closed/landed; the surviving prompt-prose contradiction is unfiled.*

**3. Make `verify.nu`/`qualitygate.nu` see untracked files — new test files are invisible until staged** *(error handling / deterministic tooling)*
- **What happened** (implementer journal observation #1): `just verify implementer` derived test-file-touched = **empty** for the new `crates/seeds/tests/library.rs`, because `scripts/verify.nu:72` (and `scripts/qualitygate.nu:50`) derive paths from `git diff --name-only` — untracked files never appear. The implementer correctly hand-ran the full crate suite (76/76), but that's exactly the manual detour the mechanical lane exists to remove, and other agents may trust the under-scoped dispatcher.
- **Change**: `scripts/verify.nu:72` + `scripts/qualitygate.nu:50` — union `git diff --name-only $base` with `git ls-files --others --exclude-standard`.
- **Expected effect**: correct test-file-touched derivation for brand-new files; no manual suite runs; gate derivation can't silently skip new files.
- *New-seed justification: seeds-9482 (closed) fixed the crates/<name> layout detection but not untracked-file blindness; nothing open covers it.*

**4. Bound the planner's `fabro_runs_list` call — 162 runs ≈ half the planner's spend** *(tool efficiency)*
- **What happened** (planner tool events seq 51–58): the in-flight check called `fabro_runs_list {workflow: "develop"}` with no bound; the reply ("listed 162 run(s)") blew the planner's input from ~12.1k to ~64.4k tokens the next turn (~$0.08 of its $0.16) — scanning full metadata for conflicts the preflight had already marked `in_flight: false` for both candidates.
- **Change**: one line in `.fabro/workflows/develop/prompts/planner.md` step 4: pass `created_since` (~48h) — older runs are terminal or covered by the preflight's branch-scan arm; the tool documents the parameter.
- **Expected effect**: ~4× fewer input tokens on this step per run (~$0.08/run at current volume), less scan noise.
- *New-seed justification: seeds-a0fa covers adding fabro_ask to planner/implementer — a different tool and purpose; no seed bounds runs_list.*

**5. Force a rebuild before nextest in the verify lane — the stale-binary trap recurred despite two written rules** *(error handling, mechanical > prose)*
- **What happened** (implementer journal observation #2): after an in-place test fix, nextest re-ran a stale binary and re-emitted the OLD failure text until a `touch` forced rebuild — the exact failure class of prompt hard-rule (c) and expertise record `mx-876053`. Rules in prose + memory still let one blind re-diagnosis cycle through this run.
- **Change**: `scripts/verify.nu` nextest dispatch — `touch` the touched test files (or `cargo clean -p <crate>`) immediately before `cargo nextest run` for test-file-touched crates.
- **Expected effect**: the stale-binary class dies mechanically; one fewer ~50s recovery loop per recurrence.
- *New-seed justification: the existing mitigations are prompt prose and an ml record; no tracker seed makes the verify script itself rebuild-safe.*

**6. Declare `output.planner` in the planner's `context_allow_keys` — a warn notice fires every run** *(contract hygiene)*
- **What happened** (run event seq 81): `run.notice warn context_update_dropped: context_allow_keys dropped: output.planner` — the engine's response-dedup writer emits `output.planner`, but the planner stanza's declared contract (`workflow.fabro` ~line 176) doesn't list it.
- **Change**: add `output.planner` to the planner's `context_allow_keys` in `.fabro/workflows/develop/workflow.fabro`.
- **Expected effect**: the notice disappears; the declared contract matches the engine writer — the exact drift-visibility discipline (fabro-900e) the file itself documents.
- *New-seed justification: one-line graph attr fix; no open seed touches the planner stanza's context contract.*

**7. Allowlist the self-inflicted prompt-lint warnings so the real one is visible** *(gate UX)*
- **What happened** (tester output, this run): 11 warnings — 10 are "top-level property … is routing-named" against the two *deliberately* routing-shaped schemas (`planner-output.schema.json`, conductor `develop-output.schema.json`), burying the single actionable signal: `project-facts.md` date pin `2026-04-14` older than 45 days — an honest "is the toolchain pin still load-bearing?" question now trained away.
- **Change**: in the prompt-lint arm of `scripts/qualitygate.nu`, suppress `routing-named` findings for files under `.fabro/workflows/*/schemas/`.
- **Expected effect**: 11→1 warnings; the stale-pin warning regains signal.
- *New-seed justification: lint-noise suppression in the gate script is unfiled; seeds-800d is the conductor packaging repair, unrelated.*

---

**Not recommended (checked, looks right)**: no change to the implementer's cost profile per se — the $2.11/976s is intrinsic to a 3,050-line move-heavy lift executed with the sanctioned mechanical pattern (11 `write_file` for new modules, no per-site edit churn); and no gate changes — the 10s compile-warm tester confirms the implementer/tester verification split is paying off. What I could not inspect: the conductor workflow (`seeds-800d` repair target) and the fabro-side repo — both outside this run's scope.
