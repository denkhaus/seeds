# Improve review — run 01M32J3AFYGBV109Z58SHHBA1J

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (16.5 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 18:22+0000 by revisor `fabro_ask`

---

All findings below are from this run's events, worker logs, journal entries, and the repo's tracker (`.seeds/issues.jsonl`). Run shape for scale: 16.5 min wall, $1.245 total; implementer = 829.6 s / $1.048 (84% of cost, 789.8 s inference vs 38.9 s tool time); gate green in 5.0 s; seed seeds-25b5 closed — but the run branch was never pushed.

## 1. Fail fast on GitHub-App `workflows`-permission rejections (highest impact — the run's work never landed)
**Evidence:** From worker logs and the `run.completed` event (seq 587): every push after the implementer checkpoint (18:17:04 → 18:18:27, six attempts) was rejected with `refusing to allow a GitHub App to create or update workflow .github/workflows/ci.yml without 'workflows' permission`. Status is `succeeded(publish_blocked)`, `pull_request: null`, final commit `3690d86` exists only in checkpoints — yet closeout already closed seeds-25b5, and that close lives in the same unpushed commit. A 16.5-min/$1.25 cycle produced no PR, and the next run will find the seed still open on `main` and re-implement it.
**Change:** Two-part: (a) grant the fabro GitHub App the `workflows` permission (user/server action — the only real fix); (b) add an arm to `nu .fabro/workflows/develop/scripts/planner-preflight.nu` that flags seeds whose named targets include `.github/workflows/**` as `publish_blocked_risk`, so the planner routes Blocked at claim time instead of burning a full cycle that cannot publish.
**Effect:** No more unpublishable full cycles; recovers ~$1.25 and 16.5 min per occurrence and prevents closed-on-an-unpushed-branch tracker drift.
**Seed:** New-seed justification — no seed in `.seeds/issues.jsonl` covers push/publish/permission failures (grep found none); this is the first observed occurrence (run 01M32J3AFYGBV109Z58SHHBA1J).

## 2. Eliminate the cargo fingerprint-staleness class that burned two implementer re-diagnosis rounds
**Evidence:** Implementer journal painpoint + mx-6f3f12: after `edit_file`, `cargo build` reported "Finished in 0.02s" without recompiling, so the `count`-field parity fix looked ineffective — "cost two re-diagnosis rounds" inside the stage that is already 84% of run cost (49 shell calls, 13 edit_file with 2 errors).
**Change:** File the fix where the implementer suggested: engine-side mtime normalization on file-tool writes (denkhaus/fabro). Interim in-repo mitigation: in `scripts/verify.nu` (`stage-implementer`), `touch` the git-diff changed sources of each code-touched crate immediately before its cargo invocation.
**Effect:** Removes ~100 s + 2 LLM rounds per Rust pass and the "stale binary re-emits old panic" misdiagnosis class entirely.
**Seed:** New-seed justification — recorded only as mulch expertise `mx-6f3f12`, no tracker seed names it (grep confirmed); the fix is engine/loop-asset territory no existing seed covers.

## 3. Bound the planner's `fabro_runs_list` call (one call cost 52% of the planner stage)
**Evidence:** Seq 63–64: `fabro_runs_list {workflow: "develop"}` returned **141 runs across two repos**; the next LLM round (seq 67) ballooned to 44,950 non-cached input tokens / $0.0666 — over half the planner's $0.129 total — plus ~26 s wall, all to conclude "no other open PRs."
**Change:** One-line edit in `.fabro/workflows/develop/prompts/planner.md`, step 4 (IN-FLIGHT EXCLUSION): require `created_since` (e.g. 7 days) on every `fabro_runs_list` call — the tool already supports the parameter; the prompt just never says to use it.
**Effect:** Cuts the planner's in-flight check from ~58k to ~15k input tokens (~$0.05 and ~20 s saved every run).
**Seed:** New-seed justification — no seed covers `fabro_runs_list` payload bounding (grep confirmed).

## 4. Stop blob-ref'ing the evidence capture out of the reviewer's context
**Evidence:** The evidence capture was 35.5 KB — above the reviewer's `preamble_inline_max_kb=16` — so it arrived as a blob ref; the reviewer spent all 5 of its `read_file` calls paging a single-line JSON blob and journaled a painpoint: path differs from the documented layout, "paging by character offset is guesswork," risking one-line truncation.
**Change:** (a) `nu .fabro/workflows/develop/scripts/evidence.nu`: emit the capture pretty-printed/line-broken; (b) in `.fabro/workflows/develop/workflow.fabro` (reviewer node), raise `preamble_inline_max_kb` 16 → 40 — the graph budget is already 48 KB (raised in fabro-1e9f precisely to keep captures inline).
**Effect:** Reviews verify from inline context with zero blob round-trips; removes the truncation risk that triggers `Verification blocked` re-capture cycles.
**Seed:** New-seed justification — no seed covers evidence-blob paging (grep confirmed); seeds-9fa3 is adjacent (guide readability) but distinct.

## 5. Close the untracked-file blind spot in `scripts/verify.nu`'s touched-crate derivation
**Evidence:** Implementer journal painpoint: `compat_commands.rs` (357 lines of new tests) was **untracked**, so the git-diff-based `touched[]` never saw it; the full suite ran only because `main.rs` happened to be tracked-touched. A test-only seed would have verified nothing.
**Change:** In `scripts/verify.nu` (`touched[]`, ~lines 73–96), union `git ls-files --others --exclude-standard` into the diff derivation before classifying code-touched vs test-file-touched crates.
**Effect:** New test files can no longer silently bypass `just verify implementer`; matches the loud-failure spirit of the already-landed layout fix.
**Seed:** New-seed justification — seeds-9482 (closed) fixed only the `crates/<name>` layout for tracked files; the untracked-file arm exists solely as expertise `mx-8e3f7c`, with no open seed (grep confirmed).

## 6. Implement seeds-9fa3 — it's approved, open, and this run shows the friction persisting
**Evidence:** The reviewer node again resolved with `skills.activated: []` and no `use_skill` in its tool list (graph `tools="read_file,grep,glob"`); the reviewer had to load the binding rust-style-guide through raw `read_file` paging while already paying for the evidence blob (rec 4), and seeds-9fa3's body records a prior run approving Rust diffs without the guide.
**Change:** Execute the user-approved options in seeds-9fa3 — add `use_skill` to the reviewer node's `tools` in `.fabro/workflows/develop/workflow.fabro`, or exempt `.fabro/skills/**` from the fs_hide envelope for reviewer read tools.
**Effect:** The standards axis becomes mechanically auditable (guide loads via the sanctioned skill path), reducing per-review tool rounds and the risk of style-blind approvals.

One process note that needs no seed: the engine emitted `run.notice context_update_dropped: output.planner` (seq 84) because the planner emitted an undeclared key — adding `output.planner` to the planner node's `context_allow_keys` (or dropping it from the prompt's contract) silences the drift warning; cosmetic, ~30 seconds of work.
