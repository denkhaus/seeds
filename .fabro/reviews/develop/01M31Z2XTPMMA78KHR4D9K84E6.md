# Improve review — run 01M31Z2XTPMMA78KHR4D9K84E6

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (3.6 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 12:39+0000 by revisor `fabro_ask`

---

All facts below are from this run's event record (stage transcripts, gate output, journals, timings, usage) plus direct reads of the loop-asset files in the workspace. Run shape for context: 196.5s wall, $0.1243 total; planner 48.8s/$0.037, implementer 104.4s/$0.070 (56% of cost), tester 5.5s, reviewer 16.8s/$0.017 with zero tool calls — a clean single-cycle approval of seeds-69ae (3-line loop-asset fix). What worked, worked well; the recommendations are the residual friction, ordered by expected impact.

## Recommendations

**1. Initialize the `.mulch/` store or document the skip — lesson capture fails every implementer pass.**
- Evidence: implementer@1 burned shell calls on `ml prime` and `ml record`, both erroring "No .mulch/ directory found. Run `mulch init`" (1 of its 9 shell calls errored; implementer journal painpoint); `lesson_capture` degraded to "skipped (ml record impossible)"; the reviewer re-flagged the same defect in its journal. I confirmed `.mulch/` does not exist in the workspace.
- Change: run `mulch init` once as a loop-asset change, or add one line to `.fabro/workflows/develop/prompts/project-facts.md` ("no `.mulch/` store in this repo — treat `ml` failure as a documented skip, never attempt `ml record`").
- Expected effect: removes 1–2 wasted calls + 1 error per implementer pass, every run, and makes the mandated lesson-capture contract satisfiable instead of permanently dead.
- Seed: **new-seed justification** — no seed in `.seeds/issues.jsonl` mentions mulch/ml store initialization (seeds-3791 only *keeps* the mulch CLI during the sd cutover; seeds-9fa3 is reviewer skill readability).

**2. Fix anchor-claim extraction for backtick-wrapped citations in `anchor_check.nu` — the preflight false-flagged all three anchors of the top candidate.**
- Evidence: `output.preflight` marked seeds-69ae `anchors_ok:false` with claims like `" — default "` (prose, not code); the planner then spent an extra LLM round + 2 shell calls (12:30:04→12:30:16, ~12s, sed re-reads) adjudicating, and journaled "paths/lines are correct". Root cause is visible in the file: when a seed cites `` `.fabro/scripts/dup-run-check.nu:209` `` the extracted tail begins with the citation's own *closing* backtick, so `claim_pat` in `extract-anchors` captures the prose between that backtick and the next one as the "claim", which can never appear on the cited line → guaranteed false `mismatch`.
- Change: in `.fabro/workflows/develop/scripts/anchor_check.nu` `extract-anchors`, advance past a leading quote char in `tail` before applying `claim_pat` (the existing `norm-ws` already handles wrapping; this is the remaining bug).
- Expected effect: `anchors_ok` becomes trustworthy for the common backticked-citation style; planners stop re-verifying flagged-but-correct anchors (~1 LLM round + 2 calls per affected run), which is exactly the round-saving the preflight node exists to provide.

**3. Give the reviewer real evidence for behavioral criteria — it approved this seed's core criterion on a wrong basis.**
- Evidence: the seed's headline criterion is "bare invocation with NO `--base` returns a clean verdict." The reviewer journal says "behavioral acceptance taken from this run's own preflight output (clean verdict, no degraded)" — but that preflight row was produced by `planner-preflight.nu` with an explicit `--base`, *before* the fix; the implementer's own bare invocation at 12:30:48 still returned `degraded: fatal: couldn't find remote ref denkhaus`. The only bare-invocation proof was the implementer's PASS line inside `implementation_summary` — a claim, not evidence, which the reviewer prompt itself says to distrust.
- Change: extend `.fabro/workflows/develop/scripts/evidence.nu` to append the implementer's verification-lane command outputs (or, minimally, add to `.fabro/workflows/develop/prompts/reviewer.md`: "`output.preflight` verdicts are explicit-`--base` results and can never satisfy a bare-default criterion — demand the command output in evidence").
- Expected effect: closes a class of approvals where a behavioral criterion is ratified from an unrelated green signal; costs one extra capture section (~seconds, no LLM).

**4. Scope the planner's `fabro_runs_list` mandate to degraded preflight arms.**
- Evidence: planner.md step 4 mandates calling `fabro_runs_list` "BEFORE the claim" on every run; planner@1 made **zero** such calls (its tools summary shows 5 shell calls only, `fabro_runs_list` invoked:false) and nothing was missed — the healthy preflight table already carried `in_flight:false` for all three candidates. The mandate is unenforced dead weight the model already silently ignores.
- Change: in `.fabro/workflows/develop/prompts/planner.md` step 4, make the call conditional: required only when `output.preflight` is absent/degraded or lacks in-flight data; the preflight's in-flight arm is authoritative otherwise.
- Expected effect: prompt matches observed reality (no fake compliance), saves a mandated tool call + potential LLM round per healthy claim, and keeps the strict fallback exactly where the branch scan can't see (open PRs / pre-PR runs).

**5. Add a micro-seed lane to the implementer — a 3-line, no-Rust seed cost 104s and 56% of run spend.**
- Evidence: implementer@1 = 104.4s wall, $0.0697, 101.3s inference vs 2.6s tool time across 10 messages — the fixed ~15k-token standing prompt re-sent per LLM round dominates a trivial diff. The brief even pre-listed exact lines (from the preflight anchors), yet the full ceremony (style-gate check, verify lane, lesson-capture attempt) applied unchanged.
- Change: in `.fabro/workflows/develop/prompts/implementer.md`, cost-tier the pass itself: briefs with no Rust and ≤3 named single-line edits run recon+edit+verify in at most 3 chained shell rounds (the chaining rule fabro-866a already exists; this caps the round count for the trivial tier).
- Expected effect: roughly halve implementer wall and cost on this seed class (~50s, ~$0.03 per run) — the single biggest per-run UX lever visible in this record.
- Seed: **new-seed justification** — no existing seed covers implementer cost-tiering (closed seeds-9482 was gate/verify crate derivation; open seeds are CLI parity, cutover, dedupe, reviewer skills, CI fixture).

**6. Stop rendering the preflight JSON twice per downstream preamble (graph hygiene).**
- Evidence: in both the planner and implementer `stage.prompt` texts, the 1.5KB `output.preflight` table appears verbatim twice — once in "## Completed stages", once in "## Context" — and is re-sent on every LLM round of a 5-round stage.
- Change: in `.fabro/workflows/develop/workflow.fabro`, add `preflight` (and `tracker_guard`, `claim_check`) to `preamble_stages_ignore` on the planner and implementer nodes; the `## Context` key keeps the data.
- Expected effect: ~1.5–3KB less prompt per round (~$0.01/run here, more on multi-cycle runs) at zero information loss.
- Seed: **new-seed justification** — preamble-rendering dedup is engine/graph plumbing not covered by any open seed (seeds-9fa3 covers reviewer tool allow-list only).

What I'd explicitly *not* change based on this run: the graph topology (single-cycle approve, 0 retries, guards unused but cheap — claim_check 86ms, tracker_guard 211ms), the gate (5.5s, correctly ran loop-asset `lint-nu` on the no-crate diff, post-seeds-9482 behavior is right), and the reviewer's verification-economy stance (16.8s, $0.017, approved from context) — only its evidence basis needs the fix in #3.
