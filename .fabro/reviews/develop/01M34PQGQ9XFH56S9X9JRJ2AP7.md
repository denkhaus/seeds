# Improve review — run 01M34PQGQ9XFH56S9X9JRJ2AP7

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (4.3 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 14:12+0000 by revisor `fabro_ask`

---

All evidence is in. Here are the recommendations, ordered by expected impact, each grounded in this run (01M34PQGQ9XFH56S9X9JRJ2AP7, seeds-81cd, total $0.290 / 3m35s active; planner alone was $0.167 = 58% of cost and 75.9s = 35% of wall).

---

**1. Fix the preflight in-flight arm's terminal-run discrimination — it fired a false positive that caused the most expensive chain of the run.**
- **What happened** (from run events seq 35→49): the preflight table flagged seeds-81cd `in_flight: true, in_flight_run: 01M34FT28TA8SMPWM8ZW7R9KN3`, but that run's status is `failed` (visible in the runs_list result itself). The planner's first LLM round (seq 44, $0.017) reasoned "Must check fabro_runs_list and skip", triggering a ~20s unbounded runs_list pull. Its own journal records: "the in-flight arm cannot distinguish terminal-failed runs from mid-flight ones".
- **Root cause** (from workspace file `.fabro/workflows/develop/scripts/planner-preflight.nu`, `terminal-tip?` at line 125): terminality is proven only from the branch **tip commit subject** — `closeout`, squash `(#n)`, or `(failed)` past a 60-min grace (`TERMINAL_GRACE_MIN_DEFAULT`). A run that dies engine-side without a `(failed)` tip commit (the comment at lines 119–122 admits failed-run journals end mid-flight) stays "in-flight" for up to 14 days, and even a correct `(failed)` tip poisons every fire within the 60-min grace — the conductor fires every 30 min, so the very next fire after any failure hits this deterministically.
- **Concrete change**: in `planner-preflight.nu` `in-flight-claims`/`terminal-tip?`, add a second terminality signal: a branch whose tip commit is older than the tracker-guard stale threshold (6h, `FABRO_GUARD_STALE_HOURS` — same loop constant) is orphaned, not in-flight; and shorten the failed-tip grace below the conductor's 30-min fire interval or key it to tip age. Also fix the journal phrase: the planner journaled `skipped: in-flight run …` while **not** skipping (it overrode) — have `planner.md` step 4 use an explicit `overrode in-flight flag: run <id> terminal` wording so revisor sweeps don't count false skips.
- **Expected effect**: removes the 20s tool call + ~25k-token run inventory + one LLM round on every post-failure fire (~$0.05–0.10 and 30–45s per affected run), and eliminates the wrongful-skip hazard — a more literal planner would have skipped the only claimable seed and burned the whole fire.
- **Seed**: no existing seed covers the preflight terminality arm — seeds-37bc covers only the runs_list filtering side. *New-seed justification: open seeds (37bc/c760/a0fa/7cd1/800d/9d19) all target other mechanisms; none touches `planner-preflight.nu`'s fabro-32db terminality proof, whose observed miss is documented only in this run's planner journal.*

**2. Bound the planner's mandatory `fabro_runs_list` call with `created_since` — the in-repo arm of an existing seed.**
- **What happened** (run events seq 48–49): the call was `{"workflow": "develop"}` — no `created_since` — returning **156 runs** in ~20s and inflating the planner context (planner conversation: 59.5k of its 64.8k input tokens; stage cost $0.167). The tool description already supports `created_since`; the planner prompt (`planner.md` step 4) never tells it to use one.
- **Concrete change**: as seeds-37bc already specifies for the in-repo arm, edit `.fabro/workflows/develop/prompts/planner.md` step 4 to mandate `created_since=<now-48h>` (plus status/PR-state filtering client-side) on every in-flight `fabro_runs_list` call.
- **Expected effect**: 156 runs → the handful from the last 48h on every develop run; ~20s tool time and a large context chunk recovered even when recommendation 1 doesn't apply (step 4 mandates the call unconditionally).
- **Seed**: **seeds-37bc** (open, P1 — its body explicitly consolidates this exact `created_since=<now-48h>` prompt bound; this run is fresh evidence the waste recurs: 156 runs vs the 142 in its basis).

**3. Make `ml record` print the mx-id at record time.**
- **What happened** (implementer journal, this run): "ml record does not print the mx-id; it had to be recovered by grepping `.mulch/expertise/conventions.jsonl` after the fact — printing the id at record time would save a lookup round." The implementer prompt makes `lesson_capture: mx-xxxxxx` a **required** field on every succeeded pass, so every lesson-capturing pass pays this recovery round — and a lazier agent could transcribe a wrong id. (Related hygiene from the diff: the record landed with description `"probe upsert"` and required adding a `conventions: {}` domain to `.mulch/mulch.config.yaml` mid-seed — pre-declaring the domain would remove that churn.)
- **Concrete change**: `ml record` (mulch CLI, toolchain-side) prints the upserted record's id on stdout; loop-side, pre-declare the `conventions` domain in `.mulch/mulch.config.yaml` so passes don't edit loop config to file a lesson.
- **Expected effect**: one shell round saved per implementer pass and the required `lesson_capture` answer becomes mechanical instead of recovered.
- **Seed**: *New-seed justification: grep of `.seeds/issues.jsonl` shows no seed about `ml record` output (only closed residual seeds-0511, unrelated); the friction is documented solely in this run's implementer journal.*

**4. Quiet the 11 per-run gate warnings that are deliberate design — protect the gate's signal.**
- **What happened** (tester stage output, this run): every `just qualitygate` run prints 10× "top-level property … is routing-named" warnings against `planner-output.schema.json` and `conductor/develop-output.schema.json` — but those properties are **intentional** (the schemas deliberately keep routing semantics per fabro-0a4c, as their own descriptions state) — plus a date-pin warning (`nightly-2026-04-14` older than 45 days) that fires forever because the pin is genuinely load-bearing in PROJECT_FACTS.
- **Concrete change**: in `.fabro/scripts/prompt-lint.nu`, allowlist routing-named properties in schemas that declare routing intent (e.g., honor the schema `description` or a marker key), and support an acknowledgment marker for the date pin (e.g., a comment/marker file recording "pin reviewed <date>").
- **Expected effect**: gate output drops from 11 recurring warnings to only real ones; reviewers (who read the tester section) and operators stop being trained to ignore warnings — zero runtime cost.
- **Seed**: *New-seed justification: no open or closed seed mentions prompt-lint/routing-named warnings (tracker grep); the noise is visible in this run's tester output.*

**5. Decide seeds-c760 (reviewer `skills="discover"` vs tool allow-list) — it cost tokens and misdeclared capability again this run.**
- **What happened** (reviewer session data, this run): the reviewer had 2 skill descriptions injected (194 tokens, incl. the full rust-style-guide summary) while `use_skill` is mechanically denied by `tools="read_file,grep,glob"` — and this seed touched no Rust, so even the legitimate path was irrelevant. One `read_file` call was actually used; verdict quality was unaffected.
- **Concrete change**: none to design — pick arm (a) or (b) as spelled out in the seed and land the one-line `workflow.fabro` reviewer-stanza change.
- **Expected effect**: declared capability matches engine enforcement; small per-review token reduction; removes a standing inconsistency both role journals keep flagging.
- **Seed**: **seeds-c760** (open, `needs-user` — awaiting your (a)/(b) decision; nothing to implement until then).

---

Not recommended despite being visible: the planner's overall prompt size (59.5k conversation tokens) — recommendations 1+2 remove the two largest contributors (runs_list inventory + adjudication rounds), so re-measuring before any prompt-surgery seed is the cheaper order of operations.
