# Improve review — run 01M33374S3J9BC6M1FFTNWWAP9

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (23.2 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 23:28+0000 by revisor `fabro_ask`

---

All evidence below is from this run's events/checkpoints (run 01M33374S3J9BC6M1FFTNWWAP9, 23.2 min, $3.40 total; implementer = $3.11 / 91% of cost and 19.5 of 23 min) and its stage journals. Tracker checked: seeds-37bc, seeds-9fa3, seeds-81cd, seeds-6eb8, seeds-a9e7, seeds-c228, seeds-540e are the open seeds; none besides those below covers these findings.

## Recommendations, by expected impact

**1. Make `dup-run-check.nu` weight files touched before verdict `duplicate`.**
What happened: preflight flagged seeds-25b5 `duplicate` off commit 3d5b866b, which landed *only* `.github/workflows/ci.yml` (its own message says "the loop keeps the code parts"). The implementer re-ran the check, got the same false positive, and per prompt rules a `duplicate` verdict routes **Blocked with no worktree changes** — one mis-parse away from wasting the entire $3.40/23-min run and stranding the priority-1 seed that blocks seeds-c228 and seeds-540e (implementer journal, painpoint 2).
Change: in `.fabro/scripts/dup-run-check.nu` (classification around lines 81–91, mirrored by the arm in `.fabro/workflows/develop/scripts/planner-preflight.nu`), an implementation match must touch product code (`git show --name-only` filtered to `crates/**`, `scripts/**`, `.fabro/**`); a `.github`-only or doc-only match downgrades to an advisory `partial_landing` row, never `duplicate`.
Effect: removes the wrongful-Blocked/re-implementation class for every seed with a CI-only or bookkeeping commit naming it.
*New-seed justification: seeds-69ae (closed) fixed only the default `--base` branch; the files-touched weighting has no open seed — the fabro-395b lineage lives in the foreign fabro tracker.*

**2. Fix the sandbox epoch-mtime bug (or mandate the touch workaround in `implementer.md`).**
What happened: file-tool writes left mtime 1970-01-01, cargo considered everything fresh, and a stale test binary re-emitted old failure text — "three blind re-runs of a differential test before diagnosis" inside the run's dominant stage (implementer journal; lesson mx-dd17ee). Those re-runs are the largest single identifiable waste in the $3.11 implementer bill.
Change: engine fix in denkhaus/fabro (file-tool write path must set real mtimes); in-repo stopgap now: extend hard rule (c) in `.fabro/workflows/develop/prompts/implementer.md` to "touch every file-tool-edited source in the same shell call as the build/test".
Effect: eliminates the stale-binary re-run class (~minutes + repeated compile tokens per Rust seed) until the engine fix lands.
*New-seed justification: mx-dd17ee is a lesson record, not a seed; no open seeds- id covers sandbox file-tool mtimes.*

**3. Fix the `tracker-guard.nu` crash (two lines).**
What happened: the guard failed at `tracker-guard.nu:166` — `$recs | get ts | last` breaks when a journal record lacks `ts` (the nu help even suggests `ts?`). Fail-open saved the run, but (a) the stale-claim requeue arm (fabro-d9f7) silently never ran — a stale `in_progress` claim now strands a seed until the 6h human threshold, and (b) the 1.5 KB nu error was re-rendered into the preamble of every downstream stage (planner, implementer, reviewer) and journaled twice.
Change: line 166 → optional access (`$recs | get -o ts | where {|t| $t != null} | last` with a `default null` guard), same pattern for the terminal check on 172.
Effect: restores the requeue safety arm on every run and removes recurring preamble noise/journal duplication.
*New-seed justification: no open seed mentions tracker-guard.nu; seeds-9fa3/81cd target other scripts.*

**4. Materialize evidence blobs as multi-line text so reviewer paging works.**
What happened: the 69.9 KB evidence capture blob-ref'd; the blob is a single-line JSON string, so `read_file` offset/limit paging cannot reach the middle — ~5 KB of diff (differential.rs top, main.rs prime section, cli.rs) was unreadable and the reviewer re-verified those regions from repo files (reviewer journal painpoint). One step worse, this exact situation is what the "Verification blocked" edge exists for — a re-capture cycle costs a full reviewer visit.
Change: engine-side blob materialization (denkhaus/fabro) should write pretty-printed/verbatim multi-line files; the reviewer prompt already prescribes offset/limit paging that the current format defeats.
Effect: reviews of large diffs stop paying extra tool rounds and stop risking evidence-delivery re-cycles.
*New-seed justification: engine blob layout has no seeds- seed (the fabro-1e9f budget raise was about inline limits, not blob file format).*

**5. Disambiguate the `publish_blocked_risk` routing rule.**
What happened: the preflight legend says "planner should route Blocked at claim time" while the planner's outcome contract reserves `Blocked` for cycle-guard deadlocks. The planner burned a long deliberation round (23:01:57→23:02:24, its heaviest at 1,395 reasoning tokens) reconciling the two, and only saved the run by discovering the workflow-file part was already landed in 3d5b866b — an adjudication the prompt never told it how to make.
Change: one sentence in the legend emitted by `.fabro/workflows/develop/scripts/planner-preflight.nu` (and mirrored in `planner.md` step 3/4): "route Blocked only when the seed's REMAINING acceptance criteria require `.github/workflows/**` edits; when landed commits already cover the workflow-file part, journal the adjudication and claim normally."
Effect: removes a ~30 s reasoning round per flagged seed and prevents a wrongful run-failing Block on the top-priority seed.
*New-seed justification: seeds-7e57 (closed) added the probe arm, not the adjudication rule; no open seed covers the legend/contract conflict.*

**6. Implement seeds-37bc — bound the planner's `fabro_runs_list` call.**
What happened: the planner's in-flight check pulled **145 unfiltered runs in a 17.6 s tool call** (23:01:22.5→23:01:40.1) and then reasoned over the whole inventory — a fresh recurrence of exactly what seeds-37bc records (18.3 s / 142 runs / broken cache prefix in run 01M32NGRSM8BC).
Change: as the seed specifies — server-side filters in the engine tool, or the in-repo arm: `planner.md` step 4 mandates `created_since` (last 48 h) plus status/PR-state filters.
Effect: ~$0.07 and 25–30 s recovered per develop run; planner cache prefix preserved. **Seed: seeds-37bc** (open, priority 1).

**7. Implement seeds-9fa3 — give the reviewer a guaranteed guide read.**
What happened: the brief named four style-guide pages (`guide: testing-and-doctests, property-tests…, option-and-result-idioms, error-propagation…`), but the reviewer's tool list again resolved to glob/grep/read_file only (no `use_skill`), and its five tool calls went to the blob plus differential.rs/main.rs/cli.rs — no guide read is visible anywhere in the stage record. The approve relied on the implementer's PASS report for the standards axis.
Change: exactly what seeds-9fa3 (user-approved) proposes — add `use_skill` to the reviewer node's tools allow-list in `.fabro/workflows/develop/workflow.fabro`, or exempt `.fabro/skills/**` from reviewer reads.
Effect: the binding Rust-standards axis becomes mechanically satisfiable and auditable instead of memory-dependent. **Seed: seeds-9fa3** (open, approved 2026-09-21).

One non-finding worth keeping: the gate ran in **6.2 s** because the implementer's crate-scoped `just verify implementer` kept it compile-warm — that policy is working exactly as designed and needs no change.
