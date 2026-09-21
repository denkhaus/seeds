# Improve review — run 01M32AAXM9Y4XDK3TFTREAZAMJ

- workflow: develop
- branch integrated: this revisor pass (unmerged until approved)
- status: succeeded (5.1 min, revisor pass — reason and cost in run detail)
- generated: 2026-09-21 15:56+0000 by revisor `fabro_ask`

---

Grounding first, from run events and the tracker file (all 13 seeds in `.seeds/issues.jsonl` checked for coverage): this run claimed **seeds-b56f** (toolchain image rebuild), burned a deterministic implementer failure discovering the sandbox has no container runtime, re-planned to a comment-only runbook, and closed green — **4m49s wall, $0.28**, of which planner (2 visits, $0.169 = 60% of cost) and the failed implementer@1 + re-plan round (~130s, $0.09 = 32% of cost) are the recoverable waste.

## Recommendations, by expected impact

**1. Add an environment-capability arm to the planner preflight (docker/registry probe before claim).**
What happened: planner@1's reasoning shows it *guessed* push might work ("GITHUB_TOKEN may exist via credential bridge") and claimed seeds-b56f; implementer@1 then spent its whole pass (73.5s, $0.054) probing docker/podman/nerdctl/buildah/crane/docker.sock/registry tokens before failing deterministically, forcing a planner@2 re-plan (56.4s, $0.036). Both passes journaled the exact fix: *"the planner preflight should probe for a container runtime and flag such seeds before a cycle starts."*
Change: new arm in `.fabro/workflows/develop/scripts/planner-preflight.nu` (recorded in its STANDING POLICY header, fabro-9ec3 pattern): probe for docker/podman/buildah/crane + `/var/run/docker.sock` once; when absent and a candidate body cites `just image-release` / `docker build|push` / ghcr, set `env_blocked: true` in the verdict table (same mechanics as the existing `anchors_ok`/`in_flight` flags) so the planner re-scopes at claim time instead of after a failed cycle.
Effect: removes the measured ~130s/$0.09 round for every environment-blocked seed — and **seeds-6eb8**, now open in the tracker with the identical shape, would be re-scoped with zero implementer passes.
*New-seed justification: no tracker seed covers a preflight capability probe (open seeds are seeds-8bb5 revisor-sd cutover, seeds-436a RenderMode residual, seeds-9fa3 reviewer guide readability, seeds-6eb8 the operator action itself); the journal painpoint proposing it files nothing by design.*

**2. Stop the deferred-action sweep from filing operator-only actions as fabro-assigned seeds.**
What happened: seeds-b56f was *itself* a deferred-action filing (from run 01M326WX); this run re-deferred the same action, and closeout filed **seeds-6eb8** (tracker line 13) — assigned to `fabro`, describing the same docker build+push+repin. The loop is self-cloning: the next run will claim seeds-6eb8 and repeat. Precedent that the sweep over-files: seeds-cd75 was closed "stale: non-actionable residual … the closeout non-blocking sweep that filed it is itself flagged for an actionable-marker fix."
Change: in `.fabro/workflows/develop/scripts/closeout.nu` (fabro-7aac sweep): when a marker action needs host-only capabilities, file it with a `needs-operator` label and **no fabro assignee** (the loop's own FAIL-CLOSED rule then routes it to the user), and skip filing when the action text duplicates the closed seed's own action. Immediate hygiene: reassign/close seeds-6eb8 — the operator runbook it would re-add is already in `scripts/run-images.nu` (12 lines added this run).
Effect: breaks the deferred-action → seed → deferred-action chain; next run works a real seed.
*Seed: names known seed seeds-6eb8 for the tracker action; no seed covers the closeout.nu classification change itself.*

**3. Bound the in-flight `fabro_runs_list` call with `created_since`.**
What happened: planner@1's runs-list call (events seq 48–49) took ~16.8s and returned **all 139 develop runs**, including 2026-09-20 fabro-origin and archived failures; conversation tokens were 50.8k of the 57.2k input for a check that only needs open-PR and non-terminal runs.
Change: one line in `.fabro/workflows/develop/prompts/planner.md`, step 4 (IN-FLIGHT EXCLUSION): pass `created_since` ≈ 7d (the tool already supports it) on the `fabro_runs_list` invocation.
Effect: cuts the largest single context injection plus a ~17s round from every first planner pass — planner is already the costliest stage ($0.169, 60% of this run).
*New-seed justification: no tracker seed mentions fabro_runs_list bounding (checked all 13).*

**4. Validate cited justfile recipe names in the preflight anchor arm; fix AGENTS.md.**
What happened: seed lineage and AGENTS.md say recipe `image`; the real recipes are `run-images` (justfile:22) and `image-release` (:30). Journaled twice — implementer@1: "will mislead a literal-match verifier"; planner@2: "A mechanical preflight arm checking cited recipe names against the justfile would catch this." The planner had to hand-annotate the re-plan brief instead.
Change: extend the existing anchor arm in `planner-preflight.nu` to extract `just <recipe>` tokens from candidate bodies and diff them against `just --summary`; flag mismatches as `anchor_flags` so the planner applies its existing `seeds update --description` stale-spec correction *before* claiming. Pair with the one-word AGENTS.md fix (`just image` → `just run-images`).
Effect: rotted command names are corrected mechanically at claim time instead of being re-discovered by every downstream pass.
*New-seed justification: journaled twice this run with the exact fix, nothing filed; seeds-69ae was the same drift class but for git branch names and is closed.*

**5. Inline grep evidence for fs_hide-cited criteria in churn-only captures.**
What happened: reviewer@1 approved with **zero tool calls** (23s) while journaling it "cannot independently verify justfile recipe wiring (justfile is fs_hide-bound to file tools) … had to be accepted from the implementer's per-criterion report."
Change: in `.fabro/workflows/develop/scripts/evidence.nu`, after the churn section, when seed-work count is zero, grep the brief-cited fs_hide paths (justfile, `scripts/**`) and inline the matching lines into the capture (bounded, like the integrity header).
Effect: the reviewer verifies wiring from the capture rather than trusting the implementer's PASS report — closing the only unverified axis in this run's approval.
*New-seed justification: nearest seed seeds-9fa3 targets the reviewer node's guide readability (use_skill/fs_hide exemption) — a different node and mechanism; nothing covers capture-side inlining.*

**What already worked (don't touch):** deterministic nodes were all <2s (tracker_guard 98ms, preflight 1.5s, gate 2.9s, evidence 157ms, closeout 151ms); the 48KB preamble budget kept the 3KB evidence capture fully inline — no blob detour; the journal contract was answered on every pass; cycle guards were never needed. Limits: I inspected run events, the graph/prompts config, and the tracker file; I did not re-probe the sandbox itself (capability findings above come from the implementer's probes recorded in run events).
