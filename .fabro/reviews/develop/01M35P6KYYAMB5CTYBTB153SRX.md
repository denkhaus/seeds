# Improve review — run 01M35P6KYYAMB5CTYBTB153SRX

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (1.7 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 23:18+0000 by revisor `fabro_ask`

---

All evidence is in hand: full stage transcripts/timings/usage from run events and checkpoints, the tester (gate) output, the stage journal, and the tracker (2 open seeds: `seeds-7e0f`, `seeds-37bc`; everything else closed, including this run's `seeds-7212`).

**Run baseline (for grounding):** 92.1s wall / 74.4s active / $0.0890 total. Planner 31.2s ($0.0389, 44% of cost), implementer 28.0s ($0.0335), reviewer 9.9s ($0.0165, 0 tool calls). All command stages <4s, zero retries, first-pass approve, PR #59 auto-merge. Things that worked and need **no** change: the closed `seeds-8795` arm held (planner made zero `fabro_runs_list` calls — preflight table was clean); evidence stayed inline at 4.9KB under the 48KB budget (no blob detour); implementer ran 3 shell calls total with correct cost-tiered verify ("no lib/ crates touched").

## Recommendations, by expected impact

**1. Claim `seeds-7e0f` next run — the gate's touched-crate derivation is the loop's load-bearing blind spot.**
*Evidence:* this run's tester printed "no crates touched" and `just verify implementer` printed "no lib/ crates touched — nothing to verify" (from tester output and implementer transcript) — both derived from exactly the `git diff --name-only $base` path in `scripts/verify.nu` (touched) and `scripts/qualitygate.nu` (touched-crates). Correct for a markdown-only seed, but the same derivation silently returns EMPTY for any seed adding *new* Rust files, which skips fmt/clippy/tests on precisely the riskiest diffs.
*Change:* the seed's own fix — union `git ls-files --others --exclude-standard` into both scripts.
*Effect:* gate and verify can never silently skip new files; no more manual full-suite detours. **Seed: seeds-7e0f (open, the only open in-repo seed).**

**2. Teach the preflight anchor arm that loop-lineage anchors are cross-repo.**
*Evidence:* preflight flagged `workflow.fabro:399` as `missing_file` (from run events, output.preflight); the planner then burned a tool round + journal line adjudicating "cross-repo, resolves" (journal line 4), and the implementer **re-derived the identical adjudication** and journaled it again (journal line 6). Two LLM adjudications of one deterministic fact; the seed body itself carries a per-seed "Note:" workaround, so every future lineage seed repeats this.
*Change:* `.fabro/workflows/develop/scripts/planner-preflight.nu` anchor arm — classify anchors naming the loop-lineage graph file (`workflow.fabro`, per the PROJECT_FACTS loop-lineage bullet) as `status: "external"` with `anchors_ok: true` + note, instead of `missing_file`.
*Effect:* one fewer LLM round (~4–5s, ~$0.01) and no duplicate journal records on every run claiming a lineage seed. **New seed — no existing seed covers anchor-flag semantics; seeds-7212 only patched the symptom per-seed in its own body.**

**3. Make gate warnings signal again: stop the 10 routing-contract schema false positives.**
*Evidence:* this run's tester output carried 11 warnings; 10 are prompt-lint complaining that `planner-output.schema.json` and `conductor develop-output.schema.json` have "routing-named" top-level properties — which is their entire deliberate design (fabro-9ec3 arm 3, per the schema descriptions). Nobody owns green-run warnings (correctly — the reviewer judges the seed, not the gate), so they ride every run as ignored noise and mask the one real warning (see #4).
*Change:* the prompt-lint arm in the gate's loop-asset checks (via `scripts/qualitygate.nu`): accept routing-named props in schemas marked as intentional routing contracts (e.g. an `"x-routing-contract": true` marker or a path allowlist for the two schema files).
*Effect:* warning count drops 11→1 per run; the remaining warning regains attention. **New seed — nothing in the tracker covers gate-warning hygiene.**

**4. Adjudicate the stale Rust toolchain pin the gate keeps asking about.**
*Evidence:* the one substantive warning in this run's gate output: "project-facts.md: date pin '2026-04-14' older than 45 days — still load-bearing?" — it has scrolled past unowned in every gate run since the pin aged out, including this one.
*Change:* file a decision seed: either bump `nightly-2026-04-14` in `.fabro/workflows/develop/prompts/project-facts.md` (+ the graph comment citing it), or add a "load-bearing until <date>" marker the lint reads so the warning silences deliberately. Pin choice is a user/toolchain decision — the seed should carry that USER DECISION note.
*Effect:* a real staleness question gets an answer instead of being re-printed ~48×/day. **New seed — no existing seed covers the pin; it needs a user decision, which is exactly what a seed records.**

**5. Stop double-rendering the preflight table in the planner prompt.**
*Evidence:* planner@1's prompt contains the full preflight JSON **twice** — once in "## Completed stages → preflight Output" and once in "## Context → output.preflight" (from the planner stage prompt, run events seq 35) — inside a 14.6k-input-token stage whose decision was a two-candidate claim.
*Change:* `workflow.fabro`, planner node: add `preamble_allow_keys` that excludes `output.preflight` (the table still arrives once via the preflight stage section the prompt already tells the planner to read first), mirroring the allow-keys discipline the implementer/reviewer nodes already carry.
*Effect:* ~1.3KB less on every planner round; the verdict table appears exactly once. **New seed — the graph comment already names "residual duplication … engine-side dedup work"; this is the cheap graph-side half, uncovered by any seed.**

**6. Tell the implementer that brief-recorded adjudications are settled.**
*Evidence:* the brief carried the `workflow.fabro:399` adjudication as an explicit Note, yet implementer@1 re-derived and re-journaled it (journal line 6 duplicates planner's line 4) — wasted reasoning and journal noise for the improve workflow that scans these files.
*Change:* one sentence in `.fabro/workflows/develop/prompts/implementer.md` step 1: "Anchor/path adjudications the brief already records are SETTLED — proceed without re-deriving or re-journaling them; journal only NEW contradictions."
*Effect:* shorter implementer passes on lineage seeds; journal stops accumulating duplicate adjudications. **New seed — no existing seed covers implementer brief-handling; seeds-a0fa (fabro_ask budgeting) is the nearest neighbor and doesn't touch this.**

**7. Silence the recurring `context_update_dropped` notice.**
*Evidence:* run notice seq 73 (warn): "context_allow_keys dropped: output.planner" — the engine's response-dedup kept `response.planner` but its undeclared twin `output.planner` gets dropped with a warn on every planner completion.
*Change:* one line in `workflow.fabro`: add `output.planner` to the planner node's `context_allow_keys` (or, engine-side, stop emitting the undeclared twin).
*Effect:* clean notice stream; the fabro-e47c producer lint stops crying wolf. **New seed — one-liner not covered by any existing seed.**

**8. Slim the reviewer's PROJECT_FACTS include to reviewer-relevant facts.**
*Evidence:* the reviewer is engine-forbidden from the tracker (`tools="read_file,grep,glob,use_skill"`, prompt: "do not touch the tracker") yet its 10.5k-token prompt embeds the full `seeds` command table (~2KB of instructions for operations it can never perform) — from the reviewer@1 prompt text in run events.
*Change:* render a reviewer variant of the PROJECT_FACTS include (drop the tracker command table; keep loop-asset paths, merge-target, gate, journal path) in `.fabro/workflows/develop/prompts/reviewer.md`.
*Effect:* ~2KB less per review prompt and a sharper role boundary. **New seed — seeds-7212 (this run) fixed reviewer.md's fs_hide sentence; nothing covers include trimming.**

One deliberate non-recommendation: `seeds-37bc` (engine-side `fabro_runs_list` filters) stayed open but this run shows its client-side arm (`seeds-8795`, closed) already eliminating the unbounded call on the clean-preflight path — leave it for the next fabro engine session rather than the develop line.
