# Improve review — run 01M326WXXVST02QX4QKQQW9NE7

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (15.3 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 15:05+0000 by revisor `fabro_ask`

---

# Workflow improvement recommendations — run `01M326WXXVST02QX4QKQQW9NE7` (seeds-3791, sd→seeds cutover)

Run facts used below (from run events + conclusion): 15m20s wall, **$1.41 total**; implementer **672s / $1.01 (72% of cost, 648s inference vs 23s tool)**; planner 108s/$0.23; reviewer 90s/$0.17; tester gate green in **4.0s** (compile-warm from the implementer's verify lane — that part works). Zero retries, zero gate bounces. The recommendations target what actually hurt.

---

**1. Sequence the image repin behind the revisor cutover — the revisor loop breaks otherwise.**
What happened: the reviewer journaled that `.fabro/workflows/revisor/` still invokes `sd` throughout, while this seed removed sd from `.mise.toml` and the toolchain image; closeout filed both facts as **seeds-8bb5** (revisor cutover) and **seeds-b56f** (rebuild+repin image). They are currently independent seeds with no dependency edge — nothing stops the repin landing first, which strands the revisor loop on an image without PATH `sd`.
Change: in the tracker run `seeds dep add seeds-8bb5 seeds-b56f` (seeds-8bb5 blocks seeds-b56f), so `seeds ready` can't offer the repin until the revisor cutover closes.
Expected effect: eliminates a whole-sibling-workflow outage; zero code change.
Seed: **seeds-8bb5** and **seeds-b56f** (both filed by this run's closeout).

**2. Make the evidence capture pageable — the reviewer could not read the middle ~72KB of a 119KB capture.**
What happened: from the reviewer journal, the evidence blob (`5289b0ff…json`) is a single-line JSON string, so `read_file` offset/limit cannot page it; the loop-churn diff section (the section the reviewer prompt *requires* it to adjudicate file-by-file in mixed captures) was unreachable, and it compensated with direct repo greps — approval rested on re-derived evidence, not the capture.
Change: `.fabro/workflows/develop/scripts/evidence.nu` — emit the capture with real newlines between records (multi-line text, not one JSON-escaped string); engine-side blob materialization (denkhaus/fabro) is the alternative.
Expected effect: reviewers verify the capture itself; removes the "unverifiable middle" failure mode that would otherwise eventually route a legitimate `Verification blocked` cycle.
Seed: no existing seed covers it — **new-seed justification: seeds-9fa3 covers reviewer *tool* availability, not blob pageability; no seed touches evidence.nu's single-line encoding.**

**3. Fix the planner-preflight anchor regex that truncates cited paths mid-token.**
What happened: from `output.preflight`, the anchor arm truncated `.fabro/Dockerfile.toolchain` to `.fabro/Dockerfile.toolchai` and flagged `missing_file`. The planner burned a full LLM round plus shell probes (events seq 52–70) adjudicating the false positive, then journaled it; the implementer re-journaled the same painpoint. It will recur for every seed citing a path that trips the regex.
Change: `.fabro/workflows/develop/scripts/planner-preflight.nu` anchor-extraction arm (path token regex), plus a regression case in `.fabro/scripts/planner-preflight-anchor-fixtures.nu`.
Expected effect: `anchors_ok` becomes trustworthy; the planner stops re-verifying flagged anchors by hand (~30–60s and several calls per affected run).
Seed: no existing seed — **new-seed justification: journal painpoints have no closeout sweep (only reviewer non-blocking findings and deferred-action markers become seeds), so this twice-reported bug has no seed.**

**4. Implement seeds-9fa3 now: add `use_skill` to the reviewer's tool allow-list.**
What happened: this run's reviewer judged the Rust diff "against the vendored guide (newtype + testing pages)", but its resolved tools were still only `glob, grep, read_file, request_user_input` (`use_skill` absent from the allow-list; the rust-style-guide skill was *available* but never activated). Compliance with the binding "read the guide first" policy still depends on the model choosing to `read_file` the guide.
Change: `.fabro/workflows/develop/workflow.fabro`, reviewer node — `tools="read_file,grep,glob,use_skill"` (direction (a) of the seed).
Expected effect: the standards axis loads deterministically; guide violations route Changes-requested on mechanism, not memory. User approval already recorded in the seed body.
Seed: **seeds-9fa3** (open, user-approved 2026-09-21).

**5. Probe plan/template read-path parity before it gate-reds a future cycle.**
What happened: from the implementer journal, the cutover's adjacent repair made `SeedRecord` id-loading lenient (sd 0.5.15 validates nothing on load — this exact strictness had gate-redded the cutover via dup-run-check-fixtures), but "Plan/template record read paths still enforce pl-/tpl-hex4 — no evidence of drift (unprobed), left strict deliberately." That is the same divergence class, left latent, with a recorded lesson (mx-eca078) but no follow-up.
Change: add a plan/template-id probe to `crates/seeds/tests/compat.rs` against the fixture reference (like the existing `read_path_accepts_any_non_empty_id` test); relax `crates/seeds/src/model.rs`/`id.rs` plan/template read paths only if the probe shows divergence.
Expected effect: converts a known-unprobed gate-red risk into a sub-second deterministic test.
Seed: no existing seed — **new-seed justification: seeds-3791's repair deliberately covered only the SeedRecord path; the unprobed plan/template half died with the seed (journal-only observation, no sweep).**

**6. Fix the `ml record` documentation that cost the implementer its only failed call.**
What happened: from the implementer journal, the first `ml record --type convention` call failed with "convention records are missing required flag(s): --content" because `AGENTS.md`'s Mulch section and `implementer.md` step 6 document a form without `--content`.
Change: one-line doc edits in `AGENTS.md` (Mulch section) and `.fabro/workflows/develop/prompts/implementer.md` step 6 — include `--content` in the documented form (or note it's required for conventions).
Expected effect: first-call success on the mandatory lesson-capture step; removes a recurring failed shell call every run that records a convention.
Seed: no existing seed — **new-seed justification: doc-only loop-asset fix reported solely as an implementer painpoint; painpoints have no sweep into the tracker.**

**7. Silence the 11 known-intentional prompt-lint warnings so gate output carries signal.**
What happened: the tester gate output (from run events) shows `prompt-lint: ok — 40 files, 11 warnings`: the intentional `nightly-2026-04-14` pin ("older than 45 days — still load-bearing?" — it is, per AGENTS.md) plus 10 "routing-named property" warnings on the two schemas that *deliberately* use routing fields (fabro-9ec3/fabro-0a4c). All 11 re-fire every gate run and are all known-intentional.
Change: `.fabro/scripts/prompt-lint.nu` — add an allowlist/annotation for the pinned toolchain date and the two intentionally routing-named schemas.
Expected effect: prompt-lint drops to zero warnings; any *new* warning in a gate log becomes actionable instead of buried in standing noise.
Seed: no existing seed — **new-seed justification: seeds-69ae (closed) fixed branch-name drift in dup-run-check, not lint-warning baselining; nothing covers prompt-lint noise.**

---

**Not recommended for change** (evidence it's already working): the implementer verify-lane design — the tester gate ran green in **4.0s** because the implementer's `just verify implementer` left the build compile-warm; and the deterministic guard/preflight/claim_check/closeout nodes cost **<3s combined**, vindicating the script-node pattern. The implementer's 648s-inference/23s-tool ratio on a mechanical 24-file rewrite is worth a follow-up measurement, but this run gives no clean lever beyond the already-encoded mechanical-transform rule, so I'm not dressing that up as a recommendation.

**Could not inspect:** the PR body/auto-merge outcome for PR #13 (state was still null in run metadata at capture time), and whether the reviewer's `read_file` calls actually included the style-guide pages (tool outputs don't name their targets) — hence recommendation 4 regardless.
