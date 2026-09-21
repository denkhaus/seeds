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

Format credit: [jayminwest/seeds](https://github.com/jayminwest/seeds) —
this repository is an independent implementation of that format, not a fork.

## Status

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
