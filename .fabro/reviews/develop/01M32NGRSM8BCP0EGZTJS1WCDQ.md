# Improve review — run 01M32NGRSM8BCP0EGZTJS1WCDQ

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (8.4 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 19:15+0000 by revisor `fabro_ask`

---

All evidence gathered — run events, stage timings/cost, journal, gate output, tracker, and the revisor's overflow ledger. Run 01M32NGRSM8BCP0EGZTJS1WCDQ in one line: seed seeds-7e57 claimed, implemented, gate green in 3.1s, approved, closed — 8m27s wall, $0.453 total, planner $0.117/86.5s, implementer $0.299/333.8s (70% of wall), zero retries. Recommendations, ordered by expected impact:

---

**1. The publish_blocked_risk arm shipped inert — make permission resolution definitive.**
Evidence (run events + workspace): the implementer's summary and journal state the probe path was "live-verified unknown when gh absent" — `gh` is not in the toolchain image; the marker `.fabro/github-app-workflows-permission` does not exist (glob confirms), and the run's `environment.env` is `{}` (from run snapshot). So `resolve-workflows-permission` returns `"unknown"` in every develop run, and by the arm's own fail-open rule it will never flag anything. The incident it guards against cost $1.245 + 16.5 min last time (seeds-7e57 body).
Change: bake `gh` into `.fabro/Dockerfile.toolchain` (the same file seeds-a9e7 already modifies) **or** set `FABRO_GH_WORKFLOWS_PERMISSION=granted` in the workflow environment env / commit the marker file — one of the three resolution tiers must be live.
Effect: the arm actually fires on a revoked permission instead of silently failing open forever.
Seed: none covers operator config or gh-in-image — **new-seed justification: seeds-7e57 closed the *code* arm only; no open seed covers wiring the permission source (env/marker/gh) the arm reads.**

**2. `fabro_runs_list` returns 142 unfiltered runs for a yes/no in-flight check — filter it server-side.**
Evidence (run events seq 63–67): the planner's in-flight exclusion call took 18.3s of tool time and returned 142 develop runs — including foreign-repo runs (denkhaus/fabro, e.g. 01M30B93PG, 01M3034NW). The follow-up LLM round then consumed 45,292 input tokens / $0.0685 (cache_read only 13k — the giant payload broke the prefix cache) just to conclude "no other in-flight develop run on this repo." That single round is ~47% of the planner's token spend and ~15% of run cost — every run.
Change: add filter params to the `fabro_runs_list` engine tool (status non-terminal, `pull_request.state=open`, `repo_origin_url` match — `created_since` already exists), or return a pre-computed "in-flight conflicts" summary; planner.md step 4 then reads one line instead of a catalog.
Effect: ~$0.07 + ~25–30s recovered per develop run; cache prefix preserved.
Seed: tracker grep for `runs_list|fabro_runs|in-flight` found nothing — **new-seed justification: engine-tool change in denkhaus/fabro; this repo's tracker has no seed for it.**

**3. Closeout filed a junk residual seed from a reviewer note that self-describes as correct.**
Evidence (checkpoint diff at closeout): seeds-ad6c was auto-filed quoting "workflows 'read' classified 'absent' (correct — only 'write' permits workflow pushes) … not blocking" — the observation affirms the behavior is right. seeds-436a ("unused variant kept for totality; acceptable") is the same class, still open and claimable. At this run's own economics, each such seed burns a full cycle (~$0.45, ~8.5 min) when the planner reaches it.
Change: in `.fabro/workflows/develop/scripts/closeout.nu` (residual-filing sweep), file a seed only when the reviewer observation carries an actionable marker (e.g. `residual:` prefix); plain "correct/not blocking" notes go to journal only.
Effect: stops minting unimplementable seeds; planner pool stays clean.
Seed: no tracker seed — **new-seed justification: this is overflow-ledger line `01M30FZ05QPQNMTYPWHKPJBE8J.md` line 24, pending unfixed since 2026-09-20 while this run (seeds-ad6c) and seeds-436a prove recurrence — promote it to a seed.**

**4. Give the implementer a nushell-pitfalls skill gate — 5 shell errors on two known traps.**
Evidence (run events, implementer session): 23 shell calls, 5 errors; the two journal observations (nu 0.115 `complete` doesn't capture missing-binary spawn errors; sourcing a script with `def main` auto-invokes it) were discovered mid-pass and are exactly what a style-guide-first rule would have prevented. Lesson mx-d157ad now exists in `.mulch/expertise/nushell.jsonl` but nothing surfaces it before work starts — the "read the guide FIRST" hard gate covers Rust only, and this seed was pure nushell.
Change: vendor a `.fabro/skills/nushell-pitfalls/SKILL.md` seeded from mx-d157ad + the two observations, and add one line to `.fabro/workflows/develop/prompts/implementer.md` extending the hard gate to loop-asset/nushell seeds.
Effect: each avoided error round saves ~10–15s of the implementer's 322s inference; fewer blind re-runs.
Seed: seeds-9fa3 covers the *reviewer* reading the *rust* guide — **new-seed justification: no seed covers an implementer-side consult path for nushell expertise on non-Rust loop-asset seeds.**

**5. The preflight table is rendered 4+ times across stage prompts — consume it at the planner.**
Evidence (stage prompts in run events): the identical ~1.5KB `output.preflight` JSON appears twice in the planner's own prompt (Completed-stages block + `## Context`), again in the implementer's preamble, and again in the reviewer's — all after the planner already made its claim decision from it.
Change: in `.fabro/workflows/develop/workflow.fabro` (planner node), add `output.preflight` to `context_consume_keys` (the fabro-699f pattern already used for `review_verdict`), or add `preflight` to the implementer/reviewer `preamble_stages_ignore`.
Effect: ~1.5–3KB less input per downstream stage per run, less cache invalidation, no information loss (the table is planning input only).
Seed: tracker grep `preamble|dedup|cache` found nothing relevant — **new-seed justification: preamble-rendering dedup is engine/graph mechanics; no existing seed covers context consumption of preflight output.**

**6. Enforce the bulleted brief shape in the schema.**
Evidence (checkpoint context): `current_seed_brief` shipped as one run-on paragraph with inline `- ` separators — planner.md step 6 demands "BULLETED acceptance criteria, not prose." The implementer's 10-criterion PASS/FAIL report and the reviewer both had to re-parse that wall of text.
Change: add a `"pattern"` on `current_seed_brief` in `.fabro/workflows/develop/schemas/planner-output.schema.json` requiring newline bullets (`\n- `), the same teeth pattern as the existing gate-command ban.
Effect: item-by-item checking becomes mechanical; misparse-driven review ping-pong risk drops.
Seed: none — **new-seed justification: no tracker seed addresses brief structure; the schema already carries the pattern-enforcement precedent (fabro-9ec3 arm 3) this extends.**

**7. Suppress the 11 by-design prompt-lint warnings in the gate.**
Evidence (tester stage output, this run): `prompt-lint: ok — 41 files, 11 warnings` — 10 × "routing-named property" against the two schemas whose routing fields are *deliberate* (fabro-9ec3 arm 3), plus the recurring 2026-04-14 date-pin warning. They fire every run, travel in every tester preamble, and bury real warnings.
Change: in `.fabro/scripts/prompt-lint.nu`, suppress routing-named warnings for schemas referenced by a node's `output_schema` attribute and mark the date pin as reviewed.
Effect: gate output carries only actionable warnings; the tester section that reviewer/implementer preambles re-render shrinks.
Seed: none — **new-seed justification: filed as overflow line `01M324AZVMHDSCSAMVP4WTZR1R.md` line 26 but never promoted; this run re-demonstrates it (11 warns).**

---

Not actionable but observed: planner journaled that seeds-a9e7's seed body cites a truncated path (`.fabro/Dockerfile.toolchai`, flagged `missing_file` in the preflight table) — the fix is a `seeds update --description` stale-spec correction at seeds-a9e7's next claim, per the planner's own stale-basis rule; no new seed needed.
