# Improve review — run 01M338CGYHE24Y5KXBXMB8DJWR

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (20.1 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-22 00:57+0000 by revisor `fabro_ask`

---

# Recommendations for the develop workflow — grounded in run 01M338CGYHE24Y5KXBXMB8DJWR

Run profile (from run events): 19.6 min wall, $3.03 total, all stages first-pass green — but with four mechanical failures the run had to route around: tracker_guard crashed at stage 1, the reviewer couldn't page the evidence blob, the reviewer couldn't read the mandated style guide, and the implementer manually provisioned the sd reference. Implementer dominated: 16.9 min wall / $2.76 (91% of cost), 949 s inference vs 64 s tool time.

Ordered by expected impact:

**1. Fix the tracker_guard crash and make the gate's loop-asset battery catch it** — *Error handling / graph design*
- What happened: the first node of the run **failed** — `nu .fabro/workflows/develop/scripts/tracker-guard.nu` exited 1 in 99 ms (`eval_block_with_input` on the empty-glob `ls` at line 152, `column_not_found: ts` at line 166), journaled as a painpoint by **both** the planner and implementer. The fail-open edge saved the run, but the stale-claim requeue arm (fabro-d9f7) living in that script silently never executed, and the tester gate still printed "lint-nu: green / loop-asset scripts green" — the battery is lint-only and blind to this runtime failure.
- Change: in `tracker-guard.nu:152-166`, use `glob` (or keep `ls` inside the existing try/catch properly) and `$recs | get ts? | last`; add one runtime smoke case to the loop-asset section of `scripts/qualitygate.nu` that runs tracker-guard against an empty journal dir fixture.
- Expected effect: guard node green every run, the stale-claim requeue arm actually reachable, and the gate able to see loop-script runtime regressions.
- Seed: **new seed needed** — tracker grep shows tracker-guard only as acceptance text inside closed seeds (seeds-facc, seeds-c228); no seed covers its runtime behavior or the gate smoke gap.

**2. Emit the evidence capture as pageable multi-line output** — *Graph design / UX*
- What happened: the 100 KB evidence capture (evidence@1 output) blob-ref'd, and per the reviewer's journal painpoint it is "a single-line JSON string, so read_file line-offset paging cannot split it — ~13k tokens of mid-diff (main.rs/model.rs/helptext/render) were unreadable; offset reads returned empty." Recovery via direct HEAD reads only worked because the worktree was clean — it will **not** work on changes-requested re-plan cycles, exactly where residue must be verified. The reviewer approved a 2,328-line diff with the middle of the diff mechanically unseen.
- Change: `.fabro/workflows/develop/scripts/evidence.nu` — pretty-print the JSON (or emit raw text) with newlines so `read_file` offset/limit paging works as the reviewer prompt assumes.
- Expected effect: every large-seed review verifies the full diff instead of relying on a clean-worktree workaround; removes the risk of an approval on unread evidence.
- Seed: **new seed needed** — no tracker seed mentions `evidence.nu`, blob paging, or capture format (the fabro-9467/fabro-1e9f lore in workflow.fabro is fabro-repo lineage, not this tracker).

**3. Implement seeds-9fa3 — make the rust-style-guide readable by the reviewer** — *Tool usage*
- What happened: the reviewer prompt makes reading `.fabro/skills/rust-style-guide/SKILL.md` binding policy, but the reviewer session's resolved tools were only `read_file, grep, glob`, the path is fs_hide-bound, and `skills.activated: []` — the run snapshot shows the skill was never loadable. The reviewer journaled it verbatim: "the mandated standards-axis read is mechanically impossible at review time."
- Change: the `reviewer` node in `.fabro/workflows/develop/workflow.fabro` — either add `use_skill` to its tools allow-list or exempt `.fabro/skills/**` from fs_hide for reviewer reads (both directions pre-approved in the seed).
- Expected effect: the standards axis becomes mechanically real; guide violations route Changes-requested instead of depending on model memory, for every Rust seed.
- Seed: **seeds-9fa3** (open, P2, user-approved 2026-09-21, ready in `seeds ready`).

**4. Close the seeds-a9e7 image-provisioning loop** — *Tool usage / cost*
- What happened: the implementer's journal observation: "sd-0.5.15 reference was unprovisioned in this sandbox (node_modules absent) — `bun install` in crates/seeds/tests/fixtures/sd-reference is required before differential suites answer anything but skip-with-note." The implementer paid a manual network provisioning step mid-pass (inside the 16.9-min dominant stage), and without it the differential smoke this seed was graded on would have silently skipped.
- Change: finish seeds-a9e7's remaining scope — the sd-ref install + image-time fixture provisioning in `.fabro/Dockerfile.toolchain`, then repin `seeds-toolchain`. Note the discrepancy to verify: this run already ran on the supposedly-pinned image `089b8b826c01`, yet node_modules was absent (seeds-d2c7 "IMMEDIATE ATTENTION 1" predicts exactly this failure mode).
- Expected effect: differential coverage live from the first implementer shell call; one network provisioning step and one skip-with-note risk removed from every Rust-seed run.
- Seed: **seeds-a9e7** (in_progress; acceptance criterion "sandbox built from the new image …" demonstrably not holding in this run).

**5. Implement seeds-37bc's in-repo arm — bound the planner's `fabro_runs_list` call** — *Tool efficiency / cost*
- What happened: planner@1 called `fabro_runs_list` unfiltered and pulled **147 runs** (event seq 70), then needed a follow-up LLM round of **46,871 input tokens / $0.0701** (seq 73) to answer a yes/no in-flight question — 55% of the planner's $0.127 total, and it broke the planner's cache prefix.
- Change: per the seed's in-repo arm, point `.fabro/workflows/develop/prompts/planner.md` step 4 at a bounded call (`created_since=<now-48h>` until the engine-side filter params land).
- Expected effect: ~$0.07 + ~25-30 s recovered per develop run; planner prefix cache preserved.
- Seed: **seeds-37bc** (open, P1, currently unassigned — needs assigning to enter the line).

**6. Mechanize the stale-binary rule in `scripts/verify.nu`** — *Error handling / prompting*
- What happened: implementer journal observation: "Stale-binary trap recurred (hard rule c): after editing doctor.rs, a no-rebuild nextest re-run re-emitted the OLD repair-format output and cost a misdiagnosis cycle." The prompt rule and lesson mx-876053 (`cargo clean -p seeds`) both already existed — prompt compliance didn't prevent it.
- Change: in `scripts/verify.nu`, before dispatching nextest for test-file-touched crates, force a rebuild of changed crates (`cargo clean -p <crate>` or touch the changed sources) so the rule no longer depends on the model.
- Expected effect: eliminates the stale-binary misdiagnosis cycle class inside the run's most expensive stage; each recurrence cost on the order of a minute of the 16.9-min implementer pass.
- Seed: **new seed needed** — closed seeds-9482 touched verify.nu's touched-crate *detection* only; no seed covers rebuild forcing (grep for rebuild/`clean -p` finds nothing open).

**7. File the model.rs `set_description(None)/set_assignee(None)` swap_remove follow-up** — *Product hygiene*
- What happened: the reviewer explicitly observed (journal, non-blocking): "model.rs set_description(None)/set_assignee(None) still use Map::remove (swap_remove) instead of remove_field — pre-existing latent key-reorder on field deletion, same class the seed fixed for remove_field; candidate follow-up seed." This run's diff fixed `remove_field` to `shift_remove` but left the two adjacent mutators on swap_remove — a silent key-reorder divergence from sd that survived an approving review.
- Change: `crates/seeds/src/model.rs` — route both `None` branches through `remove_field` (shift_remove) with a differential case pinning byte-identical stores.
- Expected effect: closes the last latent tracker-key-reorder divergence in the deletion path, keeping `seeds update --set-labels ""`/assignee-clear writes byte-identical to sd.
- Seed: **new seed needed** — the reviewer marked it "candidate follow-up seed"; no tracker seed covers it (grep for swap_remove/shift_remove in issues.jsonl: no matches).

One cross-cutting note: recommendations 1–4 all attack the same asymmetry this run exposed — deterministic nodes and guard scripts fail *silently* (fail-open) while the expensive agent stages route around them. Fixing the cheap deterministic layer first is the highest-leverage spend: the four failures above cost zero aborts but each taxed the agent stages that consume 99% of run cost.
