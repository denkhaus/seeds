# Task runner — thin launchers, logic lives in nushell (scripts/).

default:
    @just --list

# Build all workspace crates (debug profile).
build:
    cargo build --workspace

# Build and install the seeds CLI globally. cargo's bin dir is on PATH
# via rustup, so `seeds` is available everywhere (self-hosting cutover,
# seeds-3791). `--force` reinstalls over an existing same-version binary.
install: build
    cargo install --path crates/seeds --locked --force

# Touched-crates quality gate (the deterministic tester step calls this).
qualitygate:
    nu scripts/qualitygate.nu

# Stage-scoped verification dispatcher; `just verify implementer` is the
# implementer's one mechanical verification call.
verify stage:
    nu scripts/verify.nu {{ stage }}

# Run a workflow end to end: create+start+attach, wait, integrate
# (thin wrapper — logic lives in scripts/run_workflow.nu).
run *args:
    nu scripts/run_workflow.nu {{ args }}

# Build the run image the server-managed environment references
# (toolchain); content-hash gated, near-instant no-op when unchanged.
run-images:
    nu scripts/run-images.nu

# Build + push the toolchain image to GHCR — server-managed environments
# pin the pushed sha tag (ghcr.io/denkhaus/seeds-toolchain:<sha12>).
# Requires a ghcr.io docker login with write:packages:
#   gh auth refresh -s write:packages
#   gh auth token | docker login ghcr.io -u denkhaus --password-stdin
image-release:
    nu scripts/run-images.nu --push
