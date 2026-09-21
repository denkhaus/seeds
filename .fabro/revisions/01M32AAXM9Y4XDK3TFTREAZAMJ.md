# Revision — run 01M32AAXM9Y4XDK3TFTREAZAMJ

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M32AAXM9Y4XDK3TFTREAZAMJ.md
- seeds filed: none — zero balance credit this pass (no same-pass stale/superseded closes); all surviving findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass (the only close candidate, seeds-6eb8, is an implemented close — its runbook landed in run 01M32AAXM9Y4XDK3TFTREAZAMJ — which never earns credit and belongs to deterministic Closeout, not the revisor)
- basis: run 01M32AAXM9Y4XDK3TFTREAZAMJ, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit c7c16955d1222142077e7dfab5602d8040af39dc
- revised_at_commit: c7c16955d1222142077e7dfab5602d8040af39dc (ADR-0015: engine drift signal for later judgement)

## Findings

### Add environment-capability probe to planner preflight (priority 1)
overflow-to-journal — no existing seed and no open overflow covers it. Concrete change: in `.fabro/workflows/develop/scripts/planner-preflight.nu` add a probe arm (STANDING POLICY header, fabro-9ec3 pattern) that checks once for docker/podman/buildah/crane plus `/var/run/docker.sock`; when absent and a candidate body cites just image-release, docker build|push, or ghcr, set env_blocked in the verdict table (same mechanics as anchors_ok/in_flight) so the planner re-scopes at claim time. Effect: saves ~130s/$0.09 per container-runtime run (implementer@1 deterministic fail + planner@2 re-plan in this run); seeds-6eb8 would have been re-scoped with zero implementer passes.

### Classify operator-only deferred actions in closeout sweep (priority 1)
overflow-to-journal — distinct from the open closeout-sweep overflow (actionability gating, `01M30FZ05QPQNMTYPWHKPJBE8J.md` line 24); same file, different mechanism, cross-referenced. Concrete change: in `.fabro/workflows/develop/scripts/closeout.nu` (fabro-7aac sweep), when a marker action needs host-only capabilities, file it with a needs-operator label and no fabro assignee, and skip filing when the action text duplicates the closed seed's own action. Effect: breaks the self-cloning deferred-action loop (this run re-filed the closed seed's own docker build+push+repin as seeds-6eb8 assigned to fabro). Also: seeds-6eb8 is implemented (runbook landed in `scripts/run-images.nu` this run) — left open for deterministic Closeout, journal notes it.

### Bound in-flight fabro_runs_list with created_since in planner prompt (priority 2)
overflow-dup: Bound planner in-flight fabro_runs_list with created_since=<now-48h> (open in `01M31EVYN6FD7CD6198J1J7XDS.md`)

### Validate cited justfile recipe names in preflight anchor arm (priority 2)
overflow-to-journal — no existing seed or open overflow covers recipe-name validation. Concrete change: extend the anchor arm in `.fabro/workflows/develop/scripts/planner-preflight.nu` to extract `just <recipe>` tokens from candidate bodies and diff them against `just --summary`, flagging mismatches as anchor_flags; pair with the one-word `AGENTS.md` fix (`just image` → `just run-images`). Effect: planner applies the existing stale-spec correction before claiming; lineage and AGENTS.md cite recipe `image` while the real recipes are `run-images` (`justfile:22`) and `image-release` (`justfile:30`).

### Inline grep evidence for fs_hide-cited criteria in churn-only captures (priority 2)
overflow-to-journal — third evidence.nu capture change but a distinct mechanism (fs_hide inlining vs blob-collapse `01M2ZYVQ7CBSY3T0FEYBE9JK74.md` line 20 and newline emission `01M3201Q8GTC0EPT0S5YMK0Z32.md` line 25); all three are candidates for one evidence.nu multi-arm seed when balance allows. Concrete change: in `.fabro/workflows/develop/scripts/evidence.nu`, after the churn section, when seed-work count is zero, grep the brief-cited fs_hide paths (`justfile`, `scripts/**`) and inline matching lines into the capture, bounded like the integrity header. Effect: closes the reviewer's zero-tool-call approval gap (reviewer@1 approved in 23s unable to verify justfile wiring).

## Overflow ledger (machine-visible, ADR-0022 / fabro-552a)

- overflow: Add environment-capability probe to planner preflight — probe arm in `.fabro/workflows/develop/scripts/planner-preflight.nu` checking docker/podman/buildah/crane + `/var/run/docker.sock` once, setting env_blocked in the verdict table when a candidate cites container-runtime work; effect: ~130s/$0.09 saved per container-runtime run (this run's implementer@1 deterministic fail + planner@2 re-plan)
- overflow: Classify operator-only deferred actions in closeout sweep — in `.fabro/workflows/develop/scripts/closeout.nu`, host-only marker actions file with needs-operator label and no fabro assignee, skip duplicates of the closed seed's own action; effect: breaks the seeds-6eb8 self-cloning deferred-action loop (distinct mechanism from open overflow `01M30FZ05QPQNMTYPWHKPJBE8J.md` line 24, same file)
- overflow-dup: Bound planner in-flight fabro_runs_list with created_since=<now-48h> (open in `01M31EVYN6FD7CD6198J1J7XDS.md`)
- overflow: Validate cited justfile recipe names in preflight anchor arm — extract `just <recipe>` tokens from candidate bodies and diff against `just --summary` in the anchor arm of `.fabro/workflows/develop/scripts/planner-preflight.nu`, flag mismatches as anchor_flags; plus one-word `AGENTS.md` fix (`just image` → `just run-images`); effect: planner corrects stale recipe citations before claiming (lineage cites `image`, real recipes are `run-images`/`image-release`)
- overflow: Inline grep evidence for fs_hide-cited criteria in churn-only captures — in `.fabro/workflows/develop/scripts/evidence.nu`, grep brief-cited fs_hide paths (`justfile`, `scripts/**`) into the capture when seed-work count is zero; effect: reviewer can verify fs_hide-bound wiring without the repo-read fallback (reviewer@1 approved with zero tool calls)

## Notes for the next pass

- seeds-6eb8 is implemented (runbook landed in run 01M32AAXM9Y4XDK3TFTREAZAMJ, PR #15) but still open — deterministic Closeout should close it; revisor cannot (implemented closes earn no credit and supersession does not apply).
- Four new open overflows + one dup-link this pass; the evidence.nu capture theme now has three open entries (blob-collapse, newline emission, fs_hide inline) that belong in one multi-arm seed once balance allows.
