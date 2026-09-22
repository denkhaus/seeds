# Improve review — run 01M336MYEF1R5C1JMC6WHFTCQY

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (18.3 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 00:23+0000 by revisor `fabro_ask`

---

All evidence gathered — run events, checkpoints, stage transcripts, journal records, gate output, and the tracker (`.seeds/issues.jsonl`) checked for existing coverage. Run profile for grounding: **18.2 min wall, $2.29 total** (planner 96 s / $0.229; implementer 884 s / $1.976 = **86 % of cost**, 75 shell calls with 3 errors; reviewer 56 s / $0.089; gate green in 6.2 s warm). Every number below is from run events/checkpoints or the named workspace file.

## Recommendations, by expected impact

**1. Fix the `tracker_guard` crash — it fails on every run and disables two guard arms.**
What happened: the stage failed deterministically at 00:01:05 (exit 1): `.fabro/workflows/develop/scripts/tracker-guard.nu:166` — `$recs | get ts | last` throws `column_not_found` when any journal line parses to a record lacking `ts`; the `:152` ls error surfaces as eval noise. Fail-open saved the run, but the reviewer journal states the guard is "currently decorative": the drained-tracker fast exit (fabro-0da8, saves a full planner lap when the tracker empties) and the stale-claim requeue (fabro-d9f7) never execute, and the full error block was re-rendered into the planner, implementer, and reviewer preambles (all three journaled it; prior run 01M33374S did too — second consecutive occurrence).
Change: in `tracker-guard.nu:166`, filter before reading — `$recs | where {|r| $r.ts? != null} | get ts | last` (nushell's own help text suggestion), and make the `:152` glob a guarded `try { ls ... }`.
Expected effect: guard arms live again on every develop run; no failed-stage noise in three downstream prompts; kills a per-run red herring reviewers must adjudicate.
**New-seed justification:** no open seed touches tracker-guard.nu (tracker scanned — seeds-37bc is fabro_runs_list, seeds-81cd is closeout); the friction is recurring and journaled twice with no seed filed.

**2. File the sandbox mtime-1970 bug as an engine-demand seed — it is the recurring tax on the 86 %-cost stage.**
What happened: the implementer re-hit the stale-binary defect mid-lap: a new expertise record `mx-876053` (00:16:48, "touch was not always enough") followed 19 s later by `mx-7aab64` (00:17:07, "reconfirmed… cargo clean -p reliably forces the rebuild") — two lesson filings for one incident, plus part of the 3 shell errors, inside the 786 s-inference lap. Root cause per `mx-dd17ee` (workspace file `.mulch/expertise/testing.jsonl`): file-tool writes leave mtime 1970-01-01 in the sandbox clone, so cargo re-executes stale binaries. Third occurrence across two runs; the prompt's hard rule (c) mitigates but the defect persists.
Change: file a seed demanding the engine fix in denkhaus/fabro (sandbox driver must set real mtimes on write) with an in-repo interim arm: `scripts/verify.nu` `touch`es touched-crate sources before invoking cargo.
Expected effect: removes the recurring blind re-run/diagnosis loop from the dominant stage; the lesson-capture channel stops filling with duplicates of the same lesson.
**New-seed justification:** the defect lives only in mx- records and prompt lore; no tracker seed exists, and seeds-d2c7 sets the precedent of tracking engine demands in this tracker.

**3. Stop head-truncating verify failure output — it forced blind test re-runs this run.**
What happened: implementer journal painpoint: `scripts/verify.nu:168` truncates nextest failure output to the first 3000 chars (`str substring 0..3000` — verified in the file); with a 70-test suite the failing test's name/detail was cut off entirely ("only PASS lines visible before the cutoff"), forcing manual `cargo nextest run --no-fail-fast` re-runs to identify it.
Change: in `scripts/verify.nu:168` (and the 2000-char compile-stderr cut at `:154`), print the FAIL/error lines — e.g. `$out | lines | where {|l| $l =~ "FAIL|^error"} | str join "\n"` — or the tail, not the head.
Expected effect: first-failure diagnosis in one shot inside the most expensive stage; each avoided blind re-run saves a compile-warm test cycle plus an LLM round.
**New-seed justification:** seeds-9482 (closed) fixed touched-crate *detection* in the same file; no seed covers output truncation.

**4. Let the reviewer read evidence inline: raise `preamble_inline_max_kb` on the reviewer node.**
What happened: the evidence capture was 44,540 bytes (evidence stage `output_bytes`), but the reviewer node caps a single inline value at `preamble_inline_max_kb=16` (`.fabro/workflows/develop/workflow.fabro`, set by fabro-9467 when captures were ~14 KB). Result: a 45.5 KB blob ref the reviewer had to page through manually with 7 `read_file` calls over a JSON-escaped string (reviewer journal painpoint), at 3.2 % context-window use — the window was never the constraint.
Change: in `workflow.fabro`, reviewer node `preamble_inline_max_kb=16` → `64` (or have `evidence.nu` materialize a raw `.txt` instead of a JSON-escaped blob).
Expected effect: zero tool round-trips per review, faster approval on first pass, and removes the "unread blob ref" risk that would otherwise route a full evidence re-capture cycle (Verification blocked).
**New-seed justification:** no seed covers evidence-capture sizing; fabro-9467's 16 KB premise is disproven by this run's 44.5 KB capture and needs its own update.

**5. Bound/filter the planner's `fabro_runs_list` call.**
What happened: the planner's single `fabro_runs_list` tool call took **19.2 s** (00:01:16.611 → 00:01:35.783, run events) and returned 146 unfiltered runs (including foreign-repo and long-terminated ones), followed by an extra LLM round to answer a yes/no in-flight question — a recurrence of the exact incident that seeded this issue.
Change: implement **seeds-37bc** (open, P1): server-side filters on the engine tool (status non-terminal, PR open), with the in-repo interim arm — a `created_since=<now-48h>` bound in `.fabro/workflows/develop/prompts/planner.md` step 4 — shipped first if the engine is out of reach.
Expected effect: ~25–30 s and one cache-breaking LLM round recovered per develop run, per the seed's own basis; proven twice now.
**Existing seed: seeds-37bc.**

**6. Make `ml record` actually upsert, and dedupe the stale-binary records.**
What happened: implementer journal painpoint: `ml record --name` did NOT merge into `mx-dd17ee` — it created stub `mx-7aab64`, and an earlier unnamed filing created `mx-876053`; **three near-duplicate records now exist**, and the implementer prompt's hard rule ("`ml record` upserts by `--name`, merging outcomes") documents behavior the tool does not have, so every future pass re-learns this.
Change: seed for the mulch CLI (`ml record`): true merge-on-`--name` or enforced search-before-record; plus a one-time `ml record`/`seeds`-side dedupe collapsing `mx-dd17ee`/`mx-876053`/`mx-7aab64` into one record; amend the implementer.md lesson-capture paragraph to match real behavior.
Expected effect: `ml search` stops returning duplicate stubs that starve the real record of confirmation evidence (the prompt's own stated failure mode).
**New-seed justification:** no seed covers ml/mulch tooling; the tracker's only mulch reference (seeds-3791) defers it to "the mulch phase" without covering the upsert defect.

**7. Add a one-line chaining rule for planner basis probes.**
What happened: the planner spent 5 LLM rounds on probes that are reads: three separate rounds checking sd/sd-ref/sync presence (events seq 59, 65, 71) and two rounds reading the style-guide TOC (seq 83–96). The fabro-866a "one chained shell call" discipline exists in the implementer prompt but not the planner's; the planner lap was 96 s / $0.229.
Change: in `.fabro/workflows/develop/prompts/planner.md` step 3 (stale-basis check), add: "Dispatch basis probes and the style-guide TOC lookup as ONE chained shell call each (fabro-866a), never one round per read."
Expected effect: ~3–4 fewer LLM rounds per claim (~20–30 s and a few cache-breaking rounds per run); trivially safe since chaining is read-only.
**New-seed justification:** no seed covers planner-prompt probe discipline; fabro-866a's scope in the tracker is implementer/implementer-prompt only.

**Not recommended (worked as designed, worth keeping):** the implementer/tester role split left the gate compile-warm — `just qualitygate` ran green in 6.2 s vs the ~15 min cold worst case the graph comment cites; the planner's pre-claim seed-body correction (sd already has `--status`/`--dry-run`) prevented a wrong-novelty implementation; and closeout filed no unimplementable residual seeds this run.
