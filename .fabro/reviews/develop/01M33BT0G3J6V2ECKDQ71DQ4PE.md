# Improve review — run 01M33BT0G3J6V2ECKDQ71DQ4PE

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (4.1 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 01:41+0000 by revisor `fabro_ask`

---

Recommendations for the Develop workflow, grounded in run `01M33BT0G3J6V2ECKDQ71DQ4PE` (all timings/costs from run events and stage journals; seed statuses from `.seeds/issues.jsonl`). Ordered by expected impact.

---

**1. Bound the planner's `fabro_runs_list` call — covered by existing seed `seeds-37bc` (open, P1).**
What happened: the planner's in-flight check (planner@1, seq 48–49) called `fabro_runs_list` unfiltered — 18.2 s tool time returning **148 runs**. The injected JSON jumped the next LLM round's input from 11,849 → 47,326 tokens ($0.0697 for that round alone) and then sat in context for the remaining ~9 rounds. The planner was 152 s wall / **$0.336 of the run's $0.367 (92%)**, and its entire conclusion was "the only non-terminal run is this one."
Change: implement the in-repo arm of seeds-37bc — edit `.fabro/workflows/develop/prompts/planner.md` step 4 to require `created_since` (e.g. now−48h) on every `fabro_runs_list` call (engine-side status filters are the cross-repo arm the same seed tracks).
Expected effect: ~$0.07 and 25–30 s recovered per develop run (seed's own basis estimate from run 01M32NGRSM8BCP0EGZTJS1WCDQ; this run reproduces it exactly), and the planner's cache prefix stays intact.

**2. Fix the `tracker-guard.nu` crash that silently disabled both guard arms — new seed needed.**
What happened: tracker_guard@1 failed deterministically at 01:31:13 (nushell `column_not_found 'ts'` at `tracker-guard.nu:163-166`, ls-glob source error at 152:24, exit 1, `max_retries=0`, fail-open). Both purposes of the node are dead every run: the drained-tracker zero-token fast exit and the stale-claim requeue (fabro-d9f7). Worse, the 1.5 KB nushell error was replayed verbatim into **both** the planner's and reviewer's preambles ("## Stage: tracker_guard — Status: failed"), and the planner burned a journal painpoint + reasoning noticing it (its fix idea: `$recs | where {|r| $r.ts? != null }` before `get ts`). The reviewer independently journaled "loop friction worth a seed."
Change: `.fabro/workflows/develop/scripts/tracker-guard.nu:166` — filter null-`ts` records before `get ts`; make the journal glob failure non-fatal.
Expected effect: guard arms live again (stale claims requeue, drained tracker exits at zero LLM cost), and the failed-stage noise leaves every downstream prompt.
New-seed justification: no open seed covers tracker-guard.nu's ts-column crash — the tracker mentions tracker-guard only inside closed seeds' smoke batteries (seeds-facc, seeds-c228); both this run's planner and reviewer journals flag it as unfixed.

**3. Stale-suffix seeds whose fix already landed still burn a full cycle + a PR — new seed needed.**
What happened: seeds-9fa3's fix had already landed via the seeds-3791 rewrite (commit 254ceda, merged one day before this run), but no commit *names* seeds-9fa3, so preflight said "clean" and the two-branch rule forced the verification-only route: full planner lap (152 s) + evidence + reviewer (30 s) + **PR #33 with auto-merge and a glm-4.7-written description**, all to adjudicate a 1-line tracker status flip. Total: $0.367, 3.5 min, one content-free PR.
Change: extend the two-branch rule in `.fabro/workflows/develop/prompts/planner.md` (fabro-d183) with a guarded branch (c): when a *merged run-PR commit of another seed* (Fabro-Run trailer, `(#n)` subject — mechanically checkable, unlike the fabro-395b subject-match incident) provably touches the candidate's named paths and the planner judges the criteria subsumed → superseded-close naming that sha and route "Already landed". Have `.fabro/workflows/develop/scripts/planner-preflight.nu` list such subsumption candidates per seed row so the planner doesn't grep for them.
Expected effect: ~$0.35 + ~3 min + one PR saved per stale-suffix seed; the close stays review-visible via the mandatory self-explaining reason string.
New-seed justification: no existing seed extends the two-branch rule to subsumption (seeds-9fa3 was one instance, now closed by this very run; the rule change itself is untracked).

**4. Tell the planner when a basis sha is unresolvable in the shallow clone — new seed needed.**
What happened: seeds-9fa3's `Basis:` cites commit `7327f847…`; the run's clone doesn't contain it. The planner's `git show 7327f847:…` died with `fatal: bad revision` (seq 108–109, the run's only errored tool call), then it spent **two more LLM rounds** (`cat-file`, `git log --all | wc -l`, `git branch -a`) proving the sha absent before concluding "judge against the current tree" (~15 s, ~$0.04 — the answer a script could have stated for free).
Change: add a `basis_sha_resolvable` field (one `git cat-file -e <sha>`) to each candidate row in `.fabro/workflows/develop/scripts/planner-preflight.nu`, plus one line in planner.md step 3: "an in-clone-unresolvable basis sha means judge the current tree only — never probe for the sha."
Expected effect: eliminates 2–3 LLM rounds per stale-basis candidate (~$0.04, ~15 s this run; recurs for every legacy seed).
New-seed justification: no seed covers basis-sha resolvability in the depth-limited clone (tracker grep for shallow/clone/basis found nothing open).

**5. Retire `seeds-6eb8` from the ready queue — action on existing seed `seeds-6eb8`.**
What happened: it's a human-ops residual (docker host + gh packages credentials, per its own body) yet sits assigned to fabro and was listed **first** in `seeds ready` (seq 46). Every planner lap re-evaluates it: this run spent a `seeds show seeds-6eb8` call, a docker/gh availability probe, a reasoning round, and a journal skip-note on it — and will again next run (planner journal: "user should either run `just image-release` on a docker host or keep it parked as a user-action seed").
Change: user action on seeds-6eb8 — either execute its body (`just image-release` + repin per the runbook in seeds-d2c7) or unassign it so it leaves `seeds ready --assignee fabro` (the tracker's ownership switch).
Expected effect: one candidate, one shell call, and one reasoning round removed from every future develop run, and the ready list stops offering an unimplementable seed at top priority.

**6. Make the reviewer's `skills="discover"` real or remove it — new seed needed (user pre-approval exists).**
What happened: the reviewer session discovered `rust-style-guide` as an available skill (seq 157), but the node's allow-list `tools="read_file,grep,glob"` (fabro-269d) denies `use_skill`, so discovery is inert — both the planner ("discovery is inert") and the reviewer journaled this inconsistency this run. The guide is actually read via `read_file` (direction (b), which this run approved), so the skill machinery is dead weight injecting descriptions each review.
Change: `.fabro/workflows/develop/workflow.fabro`, reviewer stanza — either add `use_skill` to the allow-list (direction (a) of seeds-9fa3, explicitly user-approved 2026-09-21 in that seed's body) or drop `skills="discover"`.
Expected effect: declared capability matches engine enforcement; removes a misleading surface and the per-review skill-description injection.
New-seed justification: seeds-9fa3 (which carried the pre-approval for direction (a)) was closed by this run on direction (b) alone, so no open seed tracks the residual; a new seed can cite the recorded user approval to satisfy ADR-0019 without a fresh decision.

---

Minor observation (not a ranked rec): `run.notice context_update_dropped: output.planner` (seq 139) shows the planner's allow-keys contract doesn't declare `output.planner` while the engine dedups it into `response.planner` — harmless here, but worth folding into whichever seed next touches planner context contracts (from run events).
