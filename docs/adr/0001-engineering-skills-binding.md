# ADR-0001: Engineering-skills binding — own Seeds tracker as the skills' issue tracker, single-context domain docs

- Status: Accepted
- Date: 2026-10-03
- Deciders: user (skill-setup directive), agent (setup-matt-pocock-skills run)
- Related: ADR-0023 in denkhaus/fabro (product deciding record), AGENTS.md (tracker conduct)

## Context

The engineering skills (to-tickets, triage, to-spec, wayfinder, …) need a
per-repo binding: where issues live, which label vocabulary triage uses,
and where domain docs (CONTEXT.md, ADRs) live. This repo predates that
binding: AGENTS.md documents the Seeds tracker, but the skills' config
surface (docs/agents/) did not exist and this repo carried no ADR
directory of its own.

## Decision

1. **Issue tracker**: the repo's own Seeds store (`.seeds/`, `seeds`
   CLI) — NOT GitHub Issues. All skill tracker operations map onto
   `seeds` commands (see `docs/agents/issue-tracker.md`, including
   wayfinding equivalents).
2. **Triage labels**: the five canonical roles as-is
   (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`,
   `wontfix`) — applied via `--labels` / `--set-labels`.
3. **Domain docs**: single-context — one `CONTEXT.md` and `docs/adr/`
   at the repo root; consumer rules in `docs/agents/domain.md`; both
   are created lazily via /domain-modeling.

## Consequences

- The skills call the `seeds` CLI, never `gh issue …`, for this repo.
- This `docs/adr/` is the repo-local decision record; platform-level
  records (the format contract, freeze policy) remain in denkhaus/fabro
  (ADR-0023) — this ADR series records repo-local decisions only.
- Switching trackers later means editing `docs/agents/issue-tracker.md`
  and re-running the setup skill.
