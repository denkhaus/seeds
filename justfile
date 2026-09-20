# Task runner — thin launchers, logic lives in nushell (scripts/).

default:
    @just --list

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

# Build the run-sandbox toolchain image (context: repo root; the image
# bakes no repo binaries at bootstrap — see .fabro/Dockerfile.toolchain).
image:
    docker build -f .fabro/Dockerfile.toolchain -t seeds-toolchain:latest .
