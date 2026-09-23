# AGENTS.md — seeds

Native Rust implementation of the seeds git-native issue-tracker format.
Format-compatibility contract and product direction live in `README.md`;
the deciding record is ADR-0023 in denkhaus/fabro.

## Build and test

- `cargo build --workspace` — build
- `cargo nextest run --workspace` — all tests
- `cargo nextest run -p seeds -- <test_name>` — single test
- `cargo +nightly-2026-09-22 fmt --check --all` — format check (pinned
  nightly; install with `rustup toolchain install nightly-2026-09-22
  --profile minimal --component clippy,rustfmt`)
- `cargo +nightly-2026-09-22 clippy --workspace --all-targets -- -D warnings`
- `just qualitygate` — the develop loop's touched-crates gate
- `just image` — build the run-sandbox toolchain image

The Rust toolchain is owned by rustup (pinned `nightly-2026-09-22`);
mise owns just/bun/nushell/ripgrep and (bootstrap phase) the jayminwest
ml CLI. The tracker is this repo's own `seeds` binary (self-hosting
cutover, seeds-3791): `cargo install --path crates/seeds` puts it on
PATH via rustup's cargo bin; the toolchain image bakes it in.

## Issue tracking (Seeds)

Work is tracked in Seeds (the `seeds` binary, built from `crates/seeds`,
git-native in `.seeds/`), not GitHub Issues. Seed ids carry the prefix
`seeds-`.

- **Session start:** run `seeds prime`.
- **Filers file UNASSIGNED.** Agents that file seeds create them without
  `--assignee` — new seeds land unassigned in the backlog.
- **The develop line only works on seeds assigned to `fabro`.** The
  planner lists candidates with `seeds ready --assignee fabro --limit 200`.
  Assignment is the user's ownership switch (veto: reassign or unassign).
- **Claim:** `seeds update <id> --status in_progress --assignee fabro`.
- **Close:** never by hand from a run — the deterministic Closeout step
  closes approved seeds; the planner's one exception is the superseded
  close with a mandatory `--reason`.
- Supported read path: `seeds show <id> --format json`. Never parse
  `.seeds/issues.jsonl` by hand.
- **Never parse raw tracker files; never invent seeds flags.**

## Expertise (Mulch)

- **Session start:** run `ml prime`.
- Before finishing a task, record durable insights (`ml record <domain>
  --type <convention|pattern|failure|decision|reference|guide>
  --description "..."`); skip when nothing surfaced. Upserts by `--name`
  merge outcomes — amend the existing record instead of filing a second
  one for the same lesson.

## Workflow assets

The develop workflow lives in `.fabro/workflows/develop/` (graph, prompts,
scripts, schemas) plus `.fabro/scripts/` and `.fabro/skills/` (vendored
rust-style-guide, improve-codebase-architecture — the only skills a run's
agent stages may load). All loop-asset evolution happens through the
develop line itself (ADR-0012/0013 in denkhaus/fabro): report friction in
the journal, never fix loop assets in-pass outside a seed that targets
them. Run PRs integrate into `main` — there is no upstream mirror.

`.fabro/`, `.seeds/`, `.mulch/`, `scripts/`, and `justfile` are fs_hide
bound for file tools in runs; the shell reads and writes them normally
(grep, sed, cat, python3 heredocs), and `seeds`, `ml`, `just` keep
working.

## Clone layout

The engine's clone contract places this repository at
`/repos/denkhaus/seeds` with a `/workspace/seeds` execution symlink; the
toolchain image's mise trust pins exactly that path
(`.fabro/Dockerfile.toolchain`).

## Rust style

`.fabro/skills/rust-style-guide/SKILL.md` is the binding coding policy for
every Rust diff — read it before writing or reviewing Rust. The workspace
lints in `Cargo.toml` mirror it mechanically.

## Push gate (lefthook, operator checkouts only)

`lefthook.yml` wires `nu .fabro/scripts/push-gate.nu` as a pre-push hook
(seeds-e5af): pushes refuse while runs are active or run-PRs are open.
Quota-parked runs (`blocked(quota_rate_limit)`) do NOT refuse — parked is
not active. Operators activate the hook once per checkout with
`lefthook install`; `git push --no-verify` is the documented human escape.

INVARIANT: run sandboxes NEVER install hooks — a toolchain/bootstrap
`lefthook install` or a `core.hooksPath` override would stall the whole
develop line (every stage push would hit the gate seeing its own active
run). Never add hook installation to `.fabro/Dockerfile*` or
`.github/workflows/`.
