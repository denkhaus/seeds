# Improve review — run 01M352R82S3HK8HGAYB71KDSX9

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (3.3 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 17:39+0000 by revisor `fabro_ask`

---

All evidence below is from this run's events/checkpoints (planner/implementer/reviewer transcripts, gate output, journal records, per-stage timings and cost), cross-checked against the tracker (`.seeds/issues.jsonl` — currently 4 open seeds: `seeds-37bc`, `seeds-c760`, `seeds-a0fa`, `seeds-800d`). Run totals: 3m19s wall, $0.199, planner = 61s/$0.121 (61% of cost), implementer = 79s/$0.061, reviewer = 14s/$0.017.

## Recommendations, ordered by expected impact

**1. Bound the planner's `fabro_runs_list` call — seed `seeds-37bc` (open, P1, exact match).**
What happened: the planner's in-flight check pulled **160 unfiltered runs** (tool call 17:31:52→17:32:13, ~20.5s = 34% of planner wall time); the next LLM round jumped to 51,103 input tokens / $0.0759 for that single turn — 38% of total run cost — to answer one yes/no question ("is anything in-flight?"). This is a fresh recurrence of the exact basis recorded in `seeds-37bc` (142 runs, ~$0.07, run 01M32NGRSM8).
Change: implement `seeds-37bc`'s in-repo arm now — `.fabro/workflows/develop/prompts/planner.md` step 4, mandate `created_since=<now-48h>` (the tool already supports it; stale runs can't hold a seed anyway); engine-side filters when the fabro rebuild lands.
Effect: ~$0.07–0.08 and ~25–30s off **every** develop run, planner prefix cache preserved.

**2. Capture remote-ref state in the evidence pipe — new seed needed.**
What happened: the reviewer approved seeds-9d19 while explicitly journaling it could **not verify the tag at all**: "the tag's existence/message was judged from the implementer's ls-remote verification report because the reviewer sandbox has no shell/git access (fabro-269d) — tag claims are inherently unverifiable with read-only file tools." Four of nine acceptance criteria (tag exists, annotated, pushed, message content) rested on trusting the implementer's report.
Change: `.fabro/workflows/develop/scripts/evidence.nu` — append a bounded "remote refs" section (`git ls-remote --tags origin` + `git tag -n99 -l <tag>` for tags named in the spec) to the capture header.
Effect: the reviewer verifies remote-state claims from the capture directly (it can `read_file` the capture); closes the trust gap on every release/git-op seed.
New-seed justification: no open seed touches evidence.nu's capture scope (37bc=catalog filter, c760=reviewer tools, a0fa=fabro_ask, 800d=conductor graph).

**3. Stop the planner/implementer guessing engine capability — seed `seeds-a0fa` (open, fabro-assigned).**
What happened: the planner's reasoning trace deliberated ~2 LLM rounds on *"does pushing a tag to origin need operator capability? … Plausible claimable"*, and the implementer re-derived the same question in its first reasoning block — both re-deriving credential-bridge semantics from scratch, exactly the class `seeds-a0fa` was filed for (basis: 3 planner passes × ~$0.15). A wrong guess here would have misrouted to "Needs operator" or "Blocked".
Change: implement `seeds-a0fa` (add read-only `fabro_ask` to planner+implementer `fabro_tools`, budget-capped); cheap interim arm: one PROJECT_FACTS bullet in `.fabro/workflows/develop/prompts/project-facts.md` recording confirmed bridge capabilities (branch push ✓, tag push ✓ as of run 01M352R82S3).
Effect: capability questions get answered instead of guessed; misroute risk and re-derivation spend drop. Note for the claiming run: preflight flags a0fa's anchor `docs/lab/adr/0011-…md` missing_file — that ADR lives in denkhaus/fabro (cross-repo citation, not rot); annotate, don't re-verify.

**4. Execute the decided reviewer tools fix — seed `seeds-c760` (open, user-DECIDED option a).**
What happened: the reviewer session discovered 2 skills (`rust-style-guide`, `improve-codebase-architecture` — descriptions injected every review) but `use_skill` is absent from the node's `tools="read_file,grep,glob"` allow-list: `activated: []`, discovery inert, exactly as the seed describes.
Change: one line in `.fabro/workflows/develop/workflow.fabro` (reviewer stanza): `tools="read_file,grep,glob,use_skill"` — the user decision is already recorded in the seed body (2026-09-22: option a).
Effect: declared capability matches engine enforcement; skill-based guide loading works; the misleading surface flagged in two roles' journals disappears.

**5. Verify-before-push for immutable remote refs — new seed needed.**
What happened: implementer journal: "First tag cut carried a typo in its message (@os0.5.x vs @os-eco/seeds-cli 0.5.x); deleted and re-pushed within seconds — tag objects are immutable… worth catching before push." The brief's verify step (`ls-remote`) runs *after* push.
Change: one sentence in `.fabro/workflows/develop/prompts/implementer.md` (hard-rules block): before pushing an immutable remote ref, render it locally first (`git tag -n99 -l <tag>` before `git push origin <tag>`).
Effect: no delete/re-push cycles on future release seeds; the observation currently lives only in a journal nobody re-reads.
New-seed justification: no open seed covers release/git-op prompt hygiene; smallest durable fix is this one-line prompt rule.

**6. Fix the planner context-contract drift — new seed needed.**
What happened: `run.notice` warn (seq 72) on the planner pass: `context_update_dropped: output.planner` — the engine's response-dedup key isn't in the node's `context_allow_keys="current_seed_id,current_seed_title,current_seed_brief,review_verdict,journal"`, so the declared-contract lint (fabro-900e pattern) fires every run.
Change: add the engine-generated response keys to the planner node's `context_allow_keys` in `.fabro/workflows/develop/workflow.fabro`.
Effect: recurring warn notice gone; contract lint regains signal (a future real drift would stand out).
New-seed justification: no open seed covers planner `context_allow_keys` drift.

**7. Allowlist intentional routing schemas in prompt-lint — new seed needed.**
What happened: this run's gate printed `prompt-lint: ok — 42 files, 11 warnings`, 10 of which are "routing-named property … activates routing semantics" against `schemas/planner-output.schema.json` and `conductor/schemas/develop-output.schema.json` — schemas that **deliberately** keep routing fields (fabro-9ec3 design); the 11th is the date-pin warning on a load-bearing toolchain pin.
Change: the prompt-lint arm in `scripts/qualitygate.nu`: recognize schemas that intentionally extend the routing kind (marker or filename allowlist) so they stop warning.
Effect: gate output carries signal again — chronic 11-warning noise stops masking real lint regressions.
New-seed justification: no open seed covers qualitygate prompt-lint noise.

**What already worked (no change warranted):** the doc-only cost tier held (gate green in 3.2s, "no crates touched"); the preflight table was healthy and consumed without re-verification; claim→implement→gate→review→close ran with zero retries and zero cycles. The one systemic waste and the one trust gap are items 1 and 2.
