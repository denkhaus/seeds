# Dry-run graph walk: capability-block seed -> conductor human gate (seeds-a77c)

Recorded fixture demonstrating the routing this seed introduces. Step
through both graphs by label, exactly as the engine would.

## Setup (from the 2026-09-22 incident class)

Top `seeds ready --assignee fabro` candidate: a seed whose acceptance
criteria require the docker host socket (the seeds-6eb8 image-release
class). The sandbox cannot provide it.

## Walk

1. **develop child, planner node** — planner reads the candidate body,
   recognizes the host/operator capability requirement, does NOT claim
   (`seeds update` never runs; the seed stays open and unassigned).
   Emits journal observation `parked: needs operator seeds-xxxx` and:

   { "outcome": "succeeded", "preferred_next_label": "Needs operator" }

2. **develop child graph** — the planner's edges carry
   `planner -> exit [kind="soft", label="Needs operator",
   condition="preferred_label=\"Needs operator\""]`. The condition
   matches; the child run terminates as SoftStop — NOT Deadlock (the
   deadlock classification is the "Blocked" edge only, contrast
   `.fabro/workflows/develop/workflow.fabro`). One parked child, zero
   implementer cycles.

3. **conductor parent, develop leg** — `fabro_run_wait` returns the
   child terminal state; the leg shell-greps the child journal for
   `parked: needs operator`, finds it, and routes:

   { "outcome": "succeeded", "preferred_next_label": "Needs operator" }

   (validated: the develop-output schema enum includes "Needs operator").

4. **conductor graph** — edge
   `develop -> needs_operator [label="Needs operator",
   condition="preferred_label=\"Needs operator\""]` matches; the run
   enters the `needs_operator` hexagon human gate. The conductor runs
   approval=prompt, so the question lands in the web interviewer dock
   (mirtuell.net run page): ONE human question instead of repeated
   identical fires.

5. **While the question is pending** — the conductor run blocks at the
   gate node. No new develop child is created (the develop leg is past;
   the gate has no child edges). FAIL-CLOSED: no answer never means
   continue. The scheduler's 3-strike auto-disable counts terminal
   failures, and no terminal failure occurs while the gate is pending —
   the class is unreachable through this path.

6. **Timeout path (self-heal)** — after `timeout_seconds=86400` with no
   answer, `human.default_choice="skip"` fires option [S]
   ("Skip seed & continue line"): edge
   `needs_operator -> exit [kind="soft", label="Skip seed & continue
   line", condition="human.choice=\"skip\""]`. The pass ends softly; the
   next cron fire claims the NEXT seed. The parked seed remains open and
   unassigned in the tracker.

7. **Human answers** — [S] skip: same soft exit as the timeout default.
   [P] park: `needs_operator -> exit [kind="soft", label="Park line"]` —
   the line parks for a human. Freeform note: the note is journaled and
   the walk takes the skip semantics (continue line).

## Why the gate lives in the conductor (closeout note)

Develop children are created with `args.auto_approve=true` —
auto-approve answers any gate with the FIRST option. A child-owned gate
would silently self-answer, so children stay gate-free and the
conductor parent (approval=prompt) is the only place the question
reliably reaches a human.

Basis: runs 01M33DEW5PT/01M33DH43, 01M33F5SH/01M33F874,
01M33GWQW/01M33GZ3MD6 (2026-09-22, three identical capability-block
fires, ~$0.90, then 3-strike auto-disable).
