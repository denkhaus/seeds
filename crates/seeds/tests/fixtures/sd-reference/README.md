# sd-0.5.15 compat reference

The round-trip compatibility suite (`crates/seeds/tests/compat.rs`) and
the command-level differential battery
(`crates/seeds/tests/differential.rs`) prove this repo's `seeds`
binary stays read/write compatible with the reference implementation
it reimplements: sd 0.5.15 (`@os-eco/seeds-cli`, the jayminwest CLI
this repo was bootstrapped on — ADR-0023 in denkhaus/fabro). After the
self-hosting cutover (seeds-3791) the repo no longer installs the sd
CLI on PATH, so the reference lives HERE, in the suite fixtures,
pinned by `package.json`.

## Provisioning

Requires `bun` on PATH (the same runtime the pre-cutover mise install
used). Then:

    cd crates/seeds/tests/fixtures/sd-reference
    bun install

That creates `node_modules/` (gitignored) with the pinned package; the
committed `sd` wrapper execs its TypeScript entry under bun. Re-run
`bun install` after changing the pin. A committed `bun.lock` keeps the
install deterministic.

## How the suites use it

- `sd` (this wrapper) must be executable; `compat.rs` and
  `differential.rs` resolve it at `tests/fixtures/sd-reference/sd` and
  verify `--version` answers exactly `0.5.15`.
- The `SEEDS_COMPAT_SD` env var may point at an alternative reference
  binary (absolute path) — useful for testing against other versions.
- When the wrapper is missing, not executable, or bun cannot run it,
  every compat/differential test SKIPS with a note on stderr
  (pass-with-note), so plain `cargo nextest` runs without the
  reference stay green.
- The GATE does not accept that skip: `scripts/qualitygate.nu` and
  `scripts/verify.nu` provision this directory (`bun install
  --frozen-lockfile`) and FAIL when the reference does not answer —
  the same contract as the CI step in `.github/workflows/ci.yml`.
