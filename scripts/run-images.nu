#!/usr/bin/env nu
# Build the run image the server-managed environment references, on
# demand (`just run-images`). Rebuild only when the Dockerfile content
# hash changed; the hash rides as an image label, so unchanged files
# are near-instant no-ops. `--push` (`just image-release`) additionally
# pushes to ghcr.io/denkhaus/seeds-toolchain:<git-sha12> — the tag form
# the server-managed environment pins (same convention as the origin
# loop's fabro-toolchain push in denkhaus/fabro scripts/run-images.nu;
# simplified: no cargo-chef cook context. Distribution cutover
# (seeds-4eb4, d54c step 5): the tracker binary comes from the pinned
# GitHub Releases artifact (ARG SEEDS_VERSION in the Dockerfile); the
# Dockerfile still COPYs the crate sources - but only for the sd-ref
# fixture warm layer. The build context stays the REPO ROOT (pruned by
# the root .dockerignore) and the rebuild gate keeps hashing the
# Dockerfile PLUS the copied sources (Cargo.toml, Cargo.lock, crates/)
# via `git ls-files -s` blob hashes - a crate change with an unchanged
# Dockerfile still rebuilds (fixture sync), and a SEEDS_VERSION bump
# changes the Dockerfile itself.
#
# Requires a ghcr.io docker login with write:packages:
#   gh auth refresh -s write:packages
#   gh auth token | docker login ghcr.io -u denkhaus --password-stdin
#
# Operator runbook (docker-equipped host — NOT runnable in the dev-loop
# sandbox; deferred from seeds-b56f):
#   1. gh auth refresh -s write:packages
#      gh auth token | docker login ghcr.io -u denkhaus --password-stdin
#   2. just image-release   # builds .fabro/Dockerfile.toolchain, pushes
#      # ghcr.io/denkhaus/seeds-toolchain:<sha12> at current HEAD
#   3. Verify the pushed tag exists and works:
#      docker run --rm ghcr.io/denkhaus/seeds-toolchain:<sha12> seeds --version
#   4. Final step is OUTSIDE this repo: the server-managed environment's
#      pinned toolchain tag must be repinned to the new <sha12> by the
#      operator (seeds-b56f deferred follow-up from seeds-3791).

def build-one [dockerfile: string, tag: string, push: bool] {
    if not ($dockerfile | path exists) {
        print $"run-images: skip ($tag) \(($dockerfile) missing\)"
        return
    }
    # Content hash: Dockerfile text plus the crate sources the image
    # bakes in (git blob hashes from the index — working-tree clean is
    # assumed when releasing an image).
    let dockerfile_hash = (open --raw $dockerfile | hash sha256)
    let sources = (^git ls-files -s -- Cargo.toml Cargo.lock crates | str trim)
    let hash = ($"($dockerfile_hash)\n($sources)" | hash sha256)
    let label = "sh.seeds.toolchain.sha256"
    let wanted = $"($label)=($hash)"
    let inspect = (do {
        ^docker image inspect $tag --format $"{{index .Config.Labels \"($label)\"}}"
    } | complete)
    if $inspect.exit_code == 0 {
        let current = ($inspect.stdout | str trim)
        if $current == $hash {
            # Up to date LOCALLY - but with --push the registry tag is
            # the contract (seeds-7128): a never-pushed or failed push
            # must retry, so probe the remote before returning.
            if not $push {
                print $"run-images: ($tag) up to date \(sha ($hash | str substring 0..11)\)"
                return
            }
            let sha12 = (git rev-parse --short=12 HEAD | str trim)
            let remote = $"ghcr.io/denkhaus/seeds-toolchain:($sha12)"
            let manifest = (do {
                ^docker manifest inspect $remote
            } | complete)
            if $manifest.exit_code == 0 {
                print $"run-images: ($tag) up to date \(sha ($hash | str substring 0..11)\), registry tag ($sha12) present"
                return
            }
            print $"run-images: ($tag) up to date locally, but registry tag ($sha12) is missing - pushing"
            docker tag $tag $remote
            print $"run-images: pushing ($remote) ..."
            docker push $remote
            print $"run-images: pushed ($remote) — server-managed environments pin this sha tag"
            return
        }
    }
    print $"run-images: building ($tag) from ($dockerfile) ..."
    # Repo root as context: the Dockerfile COPYs crate source (the root
    # .dockerignore prunes everything else out of the context upload).
    let context = "."
    ^docker build --file $dockerfile --tag $tag --label $wanted $context
    print $"run-images: ($tag) built \(sha ($hash | str substring 0..11)\)"
    if $push {
        let sha12 = (git rev-parse --short=12 HEAD | str trim)
        let remote = $"ghcr.io/denkhaus/seeds-toolchain:($sha12)"
        docker tag $tag $remote
        print $"run-images: pushing ($remote) ..."
        docker push $remote
        print $"run-images: pushed ($remote) — server-managed environments pin this sha tag"
    }
}

def main [--push] {
    build-one ".fabro/Dockerfile.toolchain" "seeds-toolchain:noble" $push
}
