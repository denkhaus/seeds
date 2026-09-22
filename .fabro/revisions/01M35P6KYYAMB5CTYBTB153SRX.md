# Revision — run 01M35P6KYYAMB5CTYBTB153SRX

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M35P6KYYAMB5CTYBTB153SRX.md
- seeds filed: seeds-b1e9 — Adjudicate the stale nightly toolchain pin warning (needs-user, capability-affecting, exempt from balance)
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M35P6KYYAMB5CTYBTB153SRX, workflow version c0e3dc086cfa80e8a79408dcf1552842ecd26999e9fc7b8e093985a779cb83c4, commit a8433018fdb947bd7bae775c481580f4ff0face1
- revised_at_commit: a8433018fdb947bd7bae775c481580f4ff0face1 (ADR-0015: engine drift signal for later judgement)

## Findings

**Classify loop-lineage anchors as external in planner preflight** — overflow (no balance credit). In `.fabro/workflows/develop/scripts/planner-preflight.nu` anchor arm, classify anchors naming the loop-lineage graph file (`workflow.fabro`) as `status: "external"` with `anchors_ok: true` + note instead of `missing_file`; effect: one fewer LLM round per lineage seed and no duplicate planner/implementer adjudications (this run, journal lines 4 and 6 duplicate). Distinct from open anchor_check.nu loop-asset-resolution overflow (`01M35GF84EPCHM7FD7AHPDCEG0.md` line 16 — different file/mechanism, same symptom class).

- overflow: Classify loop-lineage anchors as external in planner preflight — in `.fabro/workflows/develop/scripts/planner-preflight.nu` anchor arm, classify `workflow.fabro` anchors as `status: "external"` + `anchors_ok: true` instead of `missing_file`; effect: one fewer LLM round (~4-5s, ~$0.01) and no duplicate journal adjudications per lineage run.

**Stop prompt-lint false positives on routing-contract schemas** — overflow-dup: prompt-lint routing-schema suppression and date-pin marker (open in `01M324AZVMHDSCSAMVP4WTZR1R.md`)

**Adjudicate the stale nightly toolchain pin warning** — filed as seeds-b1e9 (needs-user, capability-affecting: pin bump changes the run-sandbox toolchain image; implementation awaits explicit user approval per ADR-0019).

**Deduplicate the preflight table in the planner prompt** — overflow-dup: Stop rendering preflight JSON twice in preambles (open in `01M31Z2XTPMMA78KHR4D9K84E6.md`)

**Tell the implementer brief-recorded adjudications are settled** — overflow (no balance credit). Distinct from open implementer.md overflows (cost-tier, ml-record chaining, `--content` doc — different mechanisms).

- overflow: Tell the implementer brief-recorded adjudications are settled — one sentence in `.fabro/workflows/develop/prompts/implementer.md` step 1: brief-recorded anchor/path adjudications are SETTLED, journal only NEW contradictions; effect: shorter implementer passes on lineage seeds, journal stops accumulating duplicate adjudications.

**Declare output.planner in planner context_allow_keys** — overflow-dup: Add output.planner to planner context_allow_keys (open in `01M34N0KNQXVXN1A6EDTJG1.md`)

**Trim the reviewer PROJECT_FACTS include to reviewer-relevant facts** — overflow (no balance credit). Reviewer node is engine-forbidden from the tracker yet its 10.5k-token prompt embeds the full `seeds` command table. Distinct from open reviewer.md overflow (`01M3565E7X0BKSTWERSX959JWG.md` line 15 — fs_hide/skill clarification, different mechanism).

- overflow: Trim the reviewer PROJECT_FACTS include to reviewer-relevant facts — render a reviewer variant of the PROJECT_FACTS include in `.fabro/workflows/develop/prompts/reviewer.md`, drop the ~2KB `seeds` command table the reviewer cannot execute; effect: ~2KB less per review prompt and a sharper role boundary.
