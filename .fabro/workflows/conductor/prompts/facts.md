## FACTS — the repo-specific values this conductor runs on

This block is the ONE place the conductor prompts carry facts about THIS
repository (the leg prompts stay project-agnostic). Porting the loop to
another project means editing this file plus the workflow graph, not the
prompts. A stale value here is loop friction: report it in the journal,
never silently work around it.

- Merge-target branch — the branch this line's run PRs integrate into:
  `origin/main`. This repository has no upstream mirror; `main` IS the
  product line.
- Child target for EVERY `fabro_run_create` — ALWAYS EXPLICIT:
  `{"kind": "git", "repo": "denkhaus/seeds", "branch": "main"}`.
  Omitted targets inherit the parent's RUN BRANCH (a `fabro/run/…` ref
  where branch protection, required checks, and auto-merge do not exist).
- Child environment — the run environment id `seeds-toolchain` (the
  server-side environment resource pointing at this repo's toolchain
  image, built from `.fabro/Dockerfile.toolchain`).
- Stage journal — `.fabro/journal/<run_id>.jsonl`: one JSON line per stage
  completion; the seed id a run claimed is recoverable by grepping it for
  the tracker's seed-id prefix (`seeds-`).
- Revision markers — `.fabro/revisions/<run-id>.md`: a develop run
  without a marker on the base branch is unrevised.
