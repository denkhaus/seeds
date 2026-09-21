# Improve review — run 01M332792GNEPNR45GMEXV12WW

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (3.5 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 22:52+0000 by revisor `fabro_ask`

---

All evidence gathered. Here are the recommendations, grounded in what this run actually did.

## Run baseline (for impact math)

From run events and the conclusion record: 3m09s wall, $0.179 total — planner $0.078/66.8s (43%), implementer $0.074/76.7s (41%), reviewer $0.027/20.4s with **zero tool calls**, gate 3.0s ("no crates touched"), zero retries, first-pass approve on seed seeds-37a6. The loop's happy path works. Everything below is friction the run's own journal and transcripts recorded.

---

### 1. Fix the false `duplicate` verdict on approval/bookkeeping commits in `dup-run-check.nu`
**What happened:** Commit c107c43 ("operator: workflows-permission marker 'absent' + seeds-37a6 approval (option a) (#23)") landed only the marker file + a tracker sync, yet both the preflight and the implementer's mandatory dup-run-check returned verdict `duplicate`. The planner adjudicated it with `git log`/`git show` (events seq 60–62) and pre-adjudicated it in the brief; the implementer's hard rule says parse `duplicate` **mechanically → route Blocked** — it correctly overrode, but only after re-inspecting the commit (implementer journal painpoint). One brittle-model pass away from wrongly Blocked-ing a legitimate cycle.
**Change:** `.fabro/scripts/dup-run-check.nu`, `classify-filed` (lines 89–93): extend the filed-only regex with the observed bookkeeping shapes (`\bapproval\b`, `\boption [a-z]\b`, `^operator:`), and/or add a diff-content arm — a match whose diff touches only `.seeds/issues.jsonl` + declarative marker/config files classifies filed-only. The fabro-395b arm already did exactly this for reopen/verify subjects; this is the same conservative direction.
**Effect:** Eliminates double LLM adjudication of partially-landed seeds (~2 rounds ≈ 10–15s/$0.02 when triggered) and removes the wrong-Blocked failure mode on every future approval-marker commit.
**Seed:** none exists — seeds-69ae (closed) fixed only the default `--base`; new-seed justification: *no open seed covers verdict classification of approval/operator-marker commits; fabro-395b is fabro-repo lore, not this tracker.*

### 2. Fix the 8-char extension cap that truncated `.fabro/Dockerfile.toolchain` → `.fabro/Dockerfile.toolchai`
**What happened:** The preflight emitted `anchors_ok: false` / `missing_file` with the mangled path for the claimed seed. Root cause (from workspace file `.fabro/workflows/develop/scripts/anchor_check.nu`): both regexes cap extensions at `{1,8}` — `toolchain` is **9 letters**, so the match stops at `toolchai` and the tail `n` falls outside (lines 37 and 157). The planner burned a reasoning round + 2 shell calls proving the real file exists (seq 94–97); the implementer re-confirmed it again (its journal observation). The planner prompt says flagged anchors are adjudicated *instead of* re-opening files — a wrong flag forces exactly the file-reopening the script exists to prevent, and it will fire on **every** seed citing that Dockerfile (seeds-a9e7 lineage, seeds-37bc, …).
**Change:** `anchor_check.nu` lines 37 and 157: raise the bound (e.g. `{1,16}`) or add a trailing `(?![A-Za-z])` boundary so the extension is maximal.
**Effect:** `anchors_ok` becomes trustworthy again; saves ~1 LLM round + 2 shell calls (~5–10s/$0.01) per affected run and stops seeding the preflight table with a false signal the planner must adjudicate.
**Seed:** none exists — new-seed justification: *grep of `.seeds/issues.jsonl` shows no seed mentioning the anchor checker; both this run's planner and implementer journaled the suggestion but nobody filed it.*

### 3. Stop classifying `docs/` as loop churn in `evidence.nu`
**What happened:** `docs/workflows-permission.md` (55 lines — the seed's **primary deliverable**) and the spec-required `planner-preflight.nu` comment reword landed in the evidence capture's "anomaly: changed files NOT named by the seed spec" section (evidence stage output, seq 207), because `LOOP_PREFIXES` at `.fabro/workflows/develop/scripts/evidence.nu:34` hard-codes `"docs/"`. The reviewer had to adjudicate the seed's own output as potential residue (reviewer journal painpoint) — and reviewer.md instructs it to "reject on residue the implementer does not explain," so the tail risk is a full Changes-requested bounce (~3.5 min / $0.05–0.15 at this run's rates) over nothing.
**Change:** `evidence.nu:34`: drop `"docs/"` from `LOOP_PREFIXES` (this repo now has product docs — `docs/` was empty when the constant was written), or better, exempt any path cited by the in-progress seed body from churn classification.
**Effect:** The anomaly section again contains only true residue; removes the reviewer's false-adjudication round and the bounce tail risk for every doc-scoped seed.
**Seed:** none exists — new-seed justification: *no tracker seed covers evidence.nu's path classifier; the reviewer filed it only as a journal painpoint this run.*

### 4. Deduplicate the preflight JSON in the planner preamble
**What happened:** The planner@1 prompt (stage.prompt, seq 35/41) renders the ~2 KB preflight verdict JSON **twice** — once as the preflight stage's script output under "## Completed stages", once as `output.preflight` under "## Context". That's ~600 tokens of pure duplication in every planner lap (18.7k input tokens this run), plus a misparse surface (two copies that could diverge).
**Change:** `workflow.fabro`, planner node: `preamble_stages_ignore="planner,implementer,evidence,preflight"` — the stage section drops; the `output.preflight` context key (the one the prompt actually reads) stays.
**Effect:** ~600 tokens saved per planner lap (~3% of planner input), zero behavior change.
**Seed:** none exists — new-seed justification: *the graph comment defers stage-section dedup to "engine-side" work, but the in-repo `preamble_stages_ignore` arm is unclaimed by any seed.*

### 5. Don't pre-read non-top candidate bodies the preflight already classified
**What happened:** The planner fetched seeds-25b5's full 4 KB body (seq 51–53) even though the preflight table already carried its verdict, subject, and `publish_blocked_risk: true`, and the claim went to seeds-37a6 (top of `seeds ready`) anyway. One wasted shell call + one LLM round carrying 4 KB of dead context. (Related: the two errored `grep docs/` probes, seq 58/68 — each also emitted an `unsupported_control` provider warning — would have been avoided by the same cheapest-first discipline.)
**Change:** `.fabro/workflows/develop/prompts/planner.md` step 2: add one sentence — "do not fetch a candidate's full body until you fall through to it; the preflight table's verdict/subject/risk fields suffice for ordering."
**Effect:** ~1 LLM round + ~4 KB context per run (~5s, ~$0.01), keeps the planner's cache prefix cleaner.
**Seed:** none exists — new-seed justification: *seeds-37bc covers the in-flight runs_list arm of planner cost, not candidate-body pre-reading; no seed covers this.*

### 6. Enable terminal run notifications instead of the manual heartbeat
**What happened:** Run settings show `notifications.terminal.enabled: false`, so this run's completion (and PR #25, auto-merge enabled) was silent — the operator learns about it only via the session-scoped 8-minute RLM heartbeat polling recipe documented in seeds-d2c7's handoff body ("gate + tips + open PRs"), which dies with each session and must be recreated.
**Change:** Run settings (this workflow's spec, `notifications.terminal`): enable terminal events for the develop line; keep the heartbeat only for mid-run painpoint reporting.
**Effect:** Terminal events (success/deadlock/publish-blocked) reach the operator without recreating the heartbeat each session — the difference between hearing about the next `publish_blocked`-class incident (cf. run 01M32J3AF, $1.245/16.5 min discovered late) immediately vs. at the next poll.
**Seed:** seeds-d2c7 (open handoff seed) is the vehicle — fold "enable terminal notifications" into its resume procedure; no dedicated seed exists.

---

**What to keep as-is:** the preflight verdict table, inline per-criterion PASS/FAIL report, and the complete evidence capture all paid for themselves this run — reviewer approved in 20.4s with zero tool calls, and the planner's duplicate-adjudication (recommendation 1's trigger) was *correct* even though the script's verdict was wrong. Fix the deterministic helpers' precision, not the LLM adjudication layer above them.
