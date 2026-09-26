# seeds

Native Rust implementation of the [seeds](https://github.com/jayminwest/seeds)
git-native issue-tracker format.

**Format compatibility promise:** read+write drop-in compatible with
`@os-eco/seeds-cli` 0.5.x (`.seeds/` directory: `config.yaml`,
`issues.jsonl`, `plans.jsonl`, `templates.jsonl`). Unknown record fields are
preserved on every write; additive fields are the only sanctioned extension
mechanism. The CLI surface (`seeds create/show/list/ready/update/close/dep/
prime/search/plan/config/init/onboard/completions`) mirrors the
reference tool until this repo's own line replaces it (self-hosting
cutover).

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
  unimplemented-but-planned reference commands (`tpl`) answer a clear
  `not implemented yet` message instead of the reference's real
  implementations. `plan` graduated with the full decomposition
  surface (seeds-de37); `init` and the `config` group
  (schema/show/set/unset) graduated with sd parity (seeds-c813);
  `onboard` and `completions` graduated with sd-parity mechanics
  (seeds-d9f8). `migrate-from-beads` is a deliberate **non-goal**: run
  the reference tool (`sd migrate-from-beads`) once on a beads store
  and switch — this crate reads the migrated `.seeds/` result
  natively. **`upgrade`** (seeds-bdcb): same surface as the
  reference (`--check` exits 1 when outdated; `--json` carries
  `{current, latest, upToDate}`), deliberately different mechanics —
  the reference upgrades through npm; ours is an in-process self-update
  against the GitHub Releases: download the target's tar.gz, verify it
  against the release's `SHASUMS.txt` **on top of TLS**, swap the
  binary beside itself with an atomic rename. Implementation note: the
  d54c decision named the `self_update` crate, which cannot expose its
  download for SHASUM verification — the decided verification outcome
  won, so the stack is hand-rolled and minimal (ureq + rustls/ring,
  sha2, flate2, tar) behind the default-on `upgrade` cargo feature; a
  `--no-default-features` build keeps the core offline-pure. Managed
  installs do not self-replace: image-baked (/usr, /opt → `just
  run-images`), cargo-installed (`~/.cargo/bin` → `cargo install
  --locked seeds`), mise-managed (`mise upgrade cargo:seeds`) each get
  pointed at their own channel.
- **onboard section content:** the marker mechanics
  (`seeds:start`/`seeds:end`, the versioned `seeds-onboard-schema`
  comment, CLAUDE.md-before-AGENTS.md targeting, the
  `{action,status,file}` envelopes, exit codes) are differential-pinned
  against the reference; the section BODY names the live `seeds` CLI
  and this implementation's version/home instead of the retired `sd`
  reference wording.
- **completions surface:** the shell scripts enumerate the implemented
  command surface only (the help-honesty rule above), not the
  reference's full list; the error surface (`unknown shell`, missing
  argument) is differential-pinned.
- **config additive-fields writes:** `seeds config set/unset` preserve
  unknown top-level keys already present in `config.yaml` (and keep
  file order) instead of failing the reference's
  `additionalProperties: false` validation — the same additive-fields
  rule the store applies to record fields. Setting a NEW unknown
  top-level key still fails exactly like the reference, and the
  schema's top-level required/type/minimum rules are enforced
  verbatim; deep `plan_templates` section validation is not
  replicated.
- **sync per-file staging preview:** `seeds sync --status` /
  `--dry-run` list every changed file individually
  (`git status --porcelain -uall`); the reference collapses untracked
  directories to a single `?? .seeds/` entry. The tracked-file cases
  stay byte-identical (differential-pinned); the untracked-dir
  expansion is the deliberate per-file preview.
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
- **plan (decomposition surface, seeds-de37):** all 13 subcommands
  (`templates`, `prompt`, `submit`, `show`, `validate`, `outcome`,
  `review`, `edit`, `create`, `adopt`, `reorder`, `release`, `list`)
  are sd-parity — differentially pinned by the tailored
  `differential_plan_matches_sd` lifecycle case plus the read-only
  matrix cases. The mulch coupling (prior-art enrichment in `prompt`,
  `--record-decision` on `submit`) is implemented as real best-effort
  `ml` shell-outs, same as sd. Two deliberate divergences: (1) plan
  status lifecycle recompute — sd's `update`/`close` recompute
  `approved → active → done` from child statuses on every status
  change; this build does not yet wire that (a plan's `status` changes
  only via submit/create writes); (2) `Invalid JSON in plan file:`
  error detail carries this build's serde parser wording, not Bun's.

Format credit: [jayminwest/seeds](https://github.com/jayminwest/seeds) —
this repository is an independent implementation of that format, not a fork.

## Status

A faster drop-in replacement for the sd CLI: same `.seeds/` format,
same command surface, differential-pinned parity. The two main
advantages: native speed, and seamless integration into Rust
codebases via the public `seeds::commands` library API (embed the
tracker, no shelling out). No coupling to any outer automation; this
repo's own develop loop consumes `seeds`, never the reverse.

Second release: tag `v0.2.0` — adds the public `seeds::commands`
library API (the command layer lifted out of the binary), letting
external consumers embed the tracker natively instead of shelling out;
validated at the tagged commit.

First release: tag `v0.1.0` — the differential battery and round-trip
suite green at the tagged commit.

All work is tracked in this repo's own `.seeds/` tracker.

## Installation

The canonical path — one line, no toolchain (prebuilt, musl-static,
SHASUMS.txt verified on top of TLS, installs to `~/.local/bin`;
override with `SEEDS_INSTALL_DIR`, pin with `SEEDS_VERSION` or the
first argument):

```
curl -fsSL https://github.com/denkhaus/seeds/releases/latest/download/install.sh | sh
```

Via mise, either the release binaries or the crates.io source build:

- `mise use -g github:denkhaus/seeds` (release binaries)
- `mise use -g cargo:seeds@<version>` (compiles from crates.io)

Plain Cargo from crates.io: `cargo install --locked seeds`. The plain
asset alternative: `gh release download` from
[the releases](https://github.com/denkhaus/seeds/releases).

Local development keeps the self-hosting install (seeds-3791):
`cargo install --path crates/seeds` — rustup's cargo bin dir is already
on PATH, so the repo's `.mise.toml` carries no tracker entry.

Self-update (live since v0.4.0): `seeds upgrade` performs the
in-process update from the releases (`--check` reports without
installing); managed installs (image-baked, cargo-installed,
mise-managed) are pointed at their own channel instead. The
toolchain image consumes the pinned release artifact
(`ARG SEEDS_VERSION` in `.fabro/Dockerfile.toolchain`, seeds-d54c
step 5) — bumping it rebuilds via the content-hash gate.

Publishing is tag-driven: pushing a `v*` tag whose version matches the
workspace `Cargo.toml` publishes to crates.io via
`.github/workflows/publish.yml` (needs the `CARGO_REGISTRY_TOKEN`
secret; tags and crate version are equal, strictly).

## Push gate (operators)

`lefthook.yml` runs `nu .fabro/scripts/push-gate.nu` as a pre-push hook:
pushes to `main` refuse while runs are active or run-PRs are open
(quota-parked runs do not refuse). Operators enable it once per checkout
with `lefthook install`; `git push --no-verify` is the human escape.
Run sandboxes never install hooks — that invariant keeps the develop
line's own stage pushes unstalled.

## Build & test

```
cargo build --workspace
cargo nextest run --workspace
cargo +nightly-2026-09-22 fmt --check --all
cargo +nightly-2026-09-22 clippy --workspace --all-targets -- -D warnings
```

License: MIT OR Apache-2.0.
