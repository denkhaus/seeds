# Improve review — run 01M324AZVMHDSCSAMVP4WTZR1R

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (12.8 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 14:23+0000 by revisor `fabro_ask`

---

All evidence gathered — run events, journal, gate output, tracker, and workspace files. Baseline for grounding: this was a clean first-pass run (zero retries, zero gate bounces): 12m51s wall, $0.812 total, of which the implementer was 9m19s / $0.582 (72% of cost), planner 72s / $0.165, reviewer 70s / $0.065, tester 6.3s, all script nodes <2s combined (from run events / conclusion timings).

## Recommendations, ordered by expected impact

**1. Make `scripts/verify.nu` see untracked files (and force a rebuild before nextest) — the run's one real defect.**
Evidence (run events + implementer/reviewer journals): `verify.nu` line 72 derives the touched-crate classes from `git diff --name-only $base` — tracked files only. The brand-new, not-yet-committed `crates/seeds/tests/dedupe.rs` was invisible, so `just verify implementer` printed `test-file-touched=` and ran only a compile check. Consequences in this run: the implementer had to manually run `cargo nextest run -p seeds` (57/57) — out-of-lane duplicated-tester work that the reviewer then had to adjudicate as "justified substitute" instead of rejecting per its role-boundary rule; both stages filed journal painpoints; lesson `mx-8e3f7c` records a manual workaround. This is exactly the "silent no-op verification" class that seeds-9482 arm 3 was filed to eliminate — the safety net has the same hole one level up. An implementer that *doesn't* notice ships a new test file the verify lane never executed.
Change: in `scripts/verify.nu` `touched []` (line 72), union the diff with `git ls-files --others --exclude-standard` so new test files classify their crate as test-file-touched; while there, touch the derived test files before the nextest invocation (line ~140) to mechanically absorb implementer hard rule (c) — this run's journal records the stale-binary re-run re-emitting an OLD failure, one wasted diagnose cycle.
Effect: new test files trigger the crate suite mechanically; no more manual suite runs, no more reviewer adjudication of out-of-lane work; removes the stale-binary misdiagnosis class.
Seed: **no existing seed covers it** (verified by grep of `.seeds/issues.jsonl` — seeds-9482, now closed, fixed only the `crates/<name>/**` path-prefix derivation from the *tracked* diff). New-seed justification: the untracked-file blindness is a distinct gap first observed in this run (journal painpoints, mx-8e3f7c), same file, two arms.

**2. Route `seeds-9fa3` as verification-only next run — its acceptance criteria already hold.**
Evidence: seeds-9fa3 (open, user-APPROVED 2026-09-21) demands the reviewer be able to read the rust-style-guide via (a) use_skill allow-list or (b) fs_hide exemption. In THIS run the reviewer — tools `read_file,grep,glob`, no `fs_hide` on the node — made 5 `read_file` calls with 0 errors and explicitly judged "against the three named guideline pages (error propagation, collections, testing)" (reviewer observation, run events). The graph comment confirms the no-fs_hide reviewer is deliberate. Direction (b) is satisfied in the worktree today.
Change: next planner pass applies the two-branch rule branch (b) to **seeds-9fa3**: claim it with a verification-only brief naming per-criterion checks (reviewer reads SKILL.md + pages through the current envelope), skipping implementer and tester.
Effect: closes an approved-but-stale seed without a full cycle — on this run's numbers that's ~10 min wall and ~$0.65 of implementer+gate spend avoided (the fabro-9d26 fast path exists precisely for this).

**3. Bound the planner's `fabro_runs_list` call — 42.6k of its 58.7k context tokens came from one advisory check.**
Evidence (run events, planner session): the step-4 in-flight check listed **137 runs**; the planner's conversation-token category jumped 7,557 → 50,173 on the very next turn, and every subsequent turn re-carried it (planner final input 58,739 tokens; stage $0.165 / 71.8s). The check needs only non-terminal runs and open-PR runs — here, one run (itself).
Change: `.fabro/workflows/develop/prompts/planner.md` step 4 — pass `created_since` (the tool supports it; e.g. 7d, comfortably beyond tracker_guard's 6h stale-claim requeue window) so the non-terminal arm sees a bounded set; note in PROJECT_FACTS the durable fix is an engine-side status/PR-state filter with lean output fields (fabro repo).
Effect: planner context roughly halves on every planning pass; cheaper, faster, and the in-flight adjudication reads 5 rows instead of 137.
Seed: **new-seed justification**: no seed in `.seeds/issues.jsonl` mentions `fabro_runs_list`, `created_since`, or planner token budget (verified by grep); the fabro-06e0/91ff lineage cited in the prompt resolves in the origin repo, not this tracker.

**4. Apply the `seeds-3791` stale-path correction now — the anchor flag fires every run until then.**
Evidence: the preflight table in THIS run flagged `seeds-3791` with `missing_file` for `.fabro/Dockerfile.toolchai` (truncated citation of `.fabro/Dockerfile.toolchain`), and the planner journaled that the claiming run "should apply the stale-path correction via sd update before claiming." Meanwhile the flag re-appears in every future preflight while the seed stays a top candidate.
Change: one `sd update seeds-3791 --description "<full corrected body>"` fixing the path citation (per the planner prompt's own intermediate-case rule), done outside a run or by the next planner before claiming.
Effect: preflight table goes clean for that candidate; the claiming run skips one flag-adjudication round and the implementer never hits the wrong path (the fabro-05d0 failure mode the rule cites: ~6 wasted calls re-proving a wrong path).
Seed: **seeds-3791** (the correction is that seed's own pre-claim step).

**5. Scaffold `.mulch/expertise/` on first `ml record` — and stop the config comment strip.**
Evidence (implementer journal + run diff): the first `ml record` crashed with ENOENT because `.mulch/expertise/` didn't exist for the repo's first domain; the implementer recovered via `mkdir -p` + retry (one wasted tool round). Additionally, `ml record` auto-creating the domain rewrote `.mulch/mulch.config.yaml` through the YAML serializer, stripping the header and all ~80 lines of optional-knob documentation — permanent loss of inline docs that the file itself warns about.
Change: loop-side, add `.mulch/expertise/` scaffolding (e.g. in `ml prime` usage per PROJECT_FACTS, or commit the empty domain dir for the `dev-loop` domain now); durable fix is upstream in the mulch CLI (create the dir in addDomain; preserve comments).
Effect: removes first-lesson friction on every new domain; keeps the config's self-documentation intact for the user.
Seed: **new-seed justification**: grep of the tracker shows no mulch/expertise seed; mulch bootstrap friction is loop-asset friction recorded only in this run's journal.

**6. Silence the 10 by-design prompt-lint warnings — gate output signal-to-noise.**
Evidence (tester stage output, run events): every `just qualitygate` prints 11 warnings: 10× "top-level property '…' is routing-named" against `planner-output.schema.json` and `conductor/schemas/develop-output.schema.json` — but those schemas are *deliberately* routing-shaped (fabro-9ec3 arm 3 replaced the bare routing kind with a file schema keeping the routing fields; the gate itself passed them) — plus the recurring `date pin '2026-04-14' older than 45 days` warning on a pin that is load-bearing and unchanged.
Change: `.fabro/scripts/prompt-lint.nu` — recognize schemas referenced by a node's `output_schema` attribute as intentionally routing-shaped (suppress or downgrade to info), and add a reviewed-marker mechanism for the date pin.
Effect: gate output carries only actionable warnings; reviewers reading the tester section stop seeing "warn" lines that are expected design.
Seed: **new-seed justification**: no tracker seed covers lint noise (verified by grep); smallest impact, file it last or fold it into the next loop-targeting seed.

One caveat on what I could not inspect: the implementer's full 540s inference transcript beyond the journal summaries (the event log truncates long transcripts), so per-turn attribution of its 9m19s is limited to the recorded painpoints; and the mulch CLI internals are external to this workspace.
