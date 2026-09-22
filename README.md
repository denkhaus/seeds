# seeds

Native Rust implementation of the [seeds](https://github.com/jayminwest/seeds)
git-native issue-tracker format.

**Format compatibility promise:** read+write drop-in compatible with
`@os-eco/seeds-cli` 0.5.x (`.seeds/` directory: `config.yaml`,
`issues.jsonl`, `plans.jsonl`, `templates.jsonl`). Unknown record fields are
preserved on every write; additive fields are the only sanctioned extension
mechanism. The CLI surface (`seeds create/show/list/ready/update/close/dep/
prime/search`) mirrors the reference tool until this repo's own line
replaces it (self-hosting cutover, ADR-0023 in denkhaus/fabro).

## CLI surface contract

The `seeds` binary mirrors the reference CLI command-for-command where
the reference is the product: flags, JSON envelope shapes
(`{success, command, …}` incl. `count` and show's `results`/`errors`
arrays), filter and limit semantics, exit codes, and written tracker
state are pinned by the live differential battery
(`crates/seeds/tests/differential.rs`) against the provisioned sd-0.5.15
reference. Provisioning lives in `crates/seeds/tests/fixtures/sd-reference`
(see its README); the gate provisions it (`scripts/qualitygate.nu`,
`scripts/verify.nu`) — a missing reference FAILS the gate there, while
plain local `cargo nextest` runs keep the skip-with-note. The battery's
case list is driven by one `IMPLEMENTED_COMMANDS` matrix, so every
future command inherits differential coverage by convention.

### DEVIATIONS

Deliberate divergences from the reference — each documented here and
carrying its own expectation, never a silent split:

- **Help honesty:** `seeds --help` lists only implemented commands;
  unimplemented-but-planned reference commands (`init`, `tpl`,
  `onboard`, `upgrade`, `completions`,
  `plan`, `config`) answer a clear `not implemented yet` message
  instead of the reference's real implementations.
  `migrate-from-beads` is a deliberate **non-goal**: run the reference
  tool (`sd migrate-from-beads`) once on a beads store and switch —
  this crate reads the migrated `.seeds/` result natively.
- **sync per-file staging preview:** `seeds sync --status` /
  `--dry-run` list every changed file individually
  (`git status --porcelain -uall`); the reference collapses untracked
  directories to a single `?? .seeds/` entry. The tracked-file cases
  stay byte-identical (differential-pinned); the untracked-dir
  expansion is the deliberate per-file preview.
- **sync push-gate safety:** when `.fabro/scripts/push-gate.nu`
  exists at the repo root, `seeds sync` runs it before committing and
  exits non-zero with a clear message while the gate refuses — the
  tracker must not race running passes. `--force` overrides; a gate
  that cannot run (no `nu` on PATH) does not block.
- **sync commit body:** the sync commit's body carries the
  `git diff --cached --shortstat` line, making sync history greppable
  by size; the reference commits with a subject only.
- **Atomic tracker writes:** every store save lands via
  temp-file+rename, so a crash can never leave a partial JSONL store
  observable; the reference rewrites files in place.
- **show's `--json` vs `--format json` error quirk (pinned, not
  diverged):** for a single missing id the reference answers a failure
  envelope under `--json` but a plain stderr `Error: …` under
  `--format json` — this build reproduces both forms exactly.
- **stdout JSON key order:** `show`'s issue objects follow the store's
  canonical key order; the reference's projection moves
  `labels`/`assignee` ahead of the timestamps. Parsed content is
  identical (the battery compares parsed JSON, not formatting).
- **Exit codes:** no divergence observed on the covered surface — both
  binaries exit 1 on every error path (including show's partial
  multi-id failure envelope).
- **Canonical write order:** no divergence observed — both writers emit
  identical `issues.jsonl` bytes for every covered mutation (volatile
  timestamps aside).
- **doctor (hygiene batch, seeds-c228):** the check surface (12
  checks, names, pass messages, warn/fail exit semantics) and the
  `--json` envelope are sd-parity — differentially pinned. Three
  deliberate additions/divergences beyond it: (1) `--repair-report`
  (text and JSON) names the exact fix per fixable finding — e.g. the
  bidirectional dep mismatch class gets `add "X" to Y.blocks — seeds
  dep add X Y` — where sd only marks `fixable: true`; (2) malformed
  JSONL detail strings carry this build's parser wording (Rust serde),
  not Node's; (3) `--fix` repairs bidirectional mismatches by adding
  the missing reverse reference and creates the merge=union
  `.gitattributes` (same content as sd), leaving `updatedAt`
  untouched (sd's fix-time stamping is unspecified). The native
  `seeds dedupe` remains the duplicate-id heal.
- **stats (hygiene batch, seeds-c228):** `--json` keeps sd's envelope
  with the stable key set (`total`, `open`, `inProgress`, `closed`,
  `blocked`, `byType`, `byPriority`, `byLabel`) — observed sd 0.5.15
  behavior already matches, so this is pinned parity (differential +
  a native stable-keys test), not a divergence. Group maps preserve
  first-seen encounter order, as sd does.

Format credit: [jayminwest/seeds](https://github.com/jayminwest/seeds) —
this repository is an independent implementation of that format, not a fork.

## Status

First release: tag `v0.1.0` — the ADR-0023 cross-repo sync point, with
the differential battery and round-trip suite green at the tagged commit.

Second release: tag `v0.2.0` — the fabro-088b library-API sync point.
Adds the public `seeds::commands` API (command layer lifted out of the
binary, seeds-0dfd) that the fabro engine binds its native `fabro seeds`
subcommands against; fabro workspace acceptance validated at the tagged
commit.

Bootstrap. Developed autonomously by [fabro](https://github.com/denkhaus/fabro)
lines (dogfooding per ADR-0012/0023): the develop workflow in `.fabro/`
drives all implementation; its own work is tracked in this repo's `.seeds/`
tracker from day one.

## Operator notes

- [GitHub App Workflows permission](docs/workflows-permission.md) — the
  `.fabro/github-app-workflows-permission` marker: what it is, why it is
  currently `absent`, and the staleness contract tied to fabro-11d9.

## Build & test

```
cargo build --workspace
cargo nextest run --workspace
cargo +nightly-2026-04-14 fmt --check --all
cargo +nightly-2026-04-14 clippy --workspace --all-targets -- -D warnings
```

License: MIT OR Apache-2.0.
