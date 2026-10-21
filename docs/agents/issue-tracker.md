# Issue tracker: Seeds (this repo's own `.seeds/` store)

Issues and specs for this repo live in the repo's own Seeds tracker
(git-native, `.seeds/`), driven by the `seeds` CLI — **not** GitHub
Issues (explicitly unused, see `AGENTS.md`). Seed ids carry the prefix
`seeds-`.

## Conventions

- **Create**: `seeds create --title "..." --type <task|bug|feature|epic> --priority <0-4> --description "..."`. Filers file UNASSIGNED: never pass `--assignee` — new seeds land in the backlog; assignment is the operator's ownership switch.
- **Read one**: `seeds show <id> --format json` — the supported read path; never parse `.seeds/issues.jsonl` by hand.
- **List / queue**: `seeds ready --assignee fabro --limit 200` (unblocked, line-assigned); `seeds list --format json --assignee fabro --limit 200` for the full picture. Always `--limit 200` — the default 50 silently truncates.
- **Claim**: `seeds update <id> --status in_progress --assignee fabro`.
- **Body verbs** (seeds-0297/seeds-4c65): `seeds update <id> --append-description "<text>"` extends the narrative (appends, one blank line between — never re-emit the whole body); `seeds update <id> --set-description "<text>"` replaces wholesale (amend only this way when you truly mean a rewrite); `seeds update <id> --replace-in-description '{"find": "...", "replace": "..."}'` corrects one exact span (fails loud on not-found/multiple). The old `--description`/`--desc`/`--body` aliases are gone from `update`.
- **Labels**: `--labels a,b` on create; `seeds update <id> --set-labels a,b` relabels.
- **Close**: never hand-close from a run — the deterministic Closeout step closes approved seeds; the planner's one exception is the superseded close with a mandatory `--reason`. Operator-surface sessions (no run) may close with a reason. Closing remarks that belong in the narrative ride `--comment "<text>"` (appended to the description in the same call); `--reason` is the `closeReason` field.
- **Machine-readable output** (seeds-1579): `--format json` (≡ `--json`) works on every read AND write command; `--format ids` prints one identifier per line — the addressed issue id for writes, the closed ids for `close`, the new id for `create`, the non-passing check names for `doctor`. Scripts parse `--format ids`, never the human text.
- **Dependencies**: `seeds dep add <issue> <depends-on>` / `seeds dep remove <issue> <depends-on>` / `seeds dep list <issue>`.
- **Search**: `seeds search "<keyword>" --format compact` (AND-strict — one keyword per query).

## Ownership

The develop line works EXCLUSIVELY on seeds assigned to `fabro`
(`--assignee fabro` on every list/ready call). Unassigned seeds
are the backlog; the user's veto is reassign or unassign.

## When a skill says "publish to the issue tracker"

Run `seeds create` (unassigned).

## When a skill says "fetch the relevant ticket"

Run `seeds show <id> --format json`.

## Wayfinding operations (for `/wayfinder`)

The map is a single seed; child tickets are seeds linked by dependencies.

- **Map**: one seed labelled `wayfinder:map` (`seeds create --labels wayfinder:map --title "..."`), body holding Notes / Decisions-so-far / Fog.
- **Child ticket**: `seeds create --labels wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`), first body line `Part of: <map-id>`; link to the map with `seeds dep add <child> <map>`.
- **Blocking**: `seeds dep add <child> <blocker>` (native tracker dependencies; `seeds ready` already excludes blocked seeds).
- **Frontier query**: `seeds ready --limit 200` (open + unblocked), drop assigned ones; first in priority order wins.
- **Claim**: `seeds update <id> --status in_progress --assignee <driver>`.
- **Resolve**: append the answer to the body (`seeds update <id> --append-description "<answer>"`), then close with a reason (`seeds close <id> --reason "wayfinder: <summary>"` — operator-surface close, see above); append the context pointer to the map body with `--append-description`.
