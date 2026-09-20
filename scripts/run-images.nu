#!/usr/bin/env nu
# Build the run image the server-managed environment references, on
# demand (`just run-images`). Rebuild only when the Dockerfile content
# hash changed; the hash rides as an image label, so unchanged files
# are near-instant no-ops. `--push` (`just image-release`) additionally
# pushes to ghcr.io/denkhaus/seeds-toolchain:<git-sha12> — the tag form
# the server-managed environment pins (same convention as the origin
# loop's fabro-toolchain push in denkhaus/fabro scripts/run-images.nu;
# simplified: the seeds bootstrap toolchain stages no binaries and no
# cargo-chef cook context — nothing is COPYed from the build context).
#
# Requires a ghcr.io docker login with write:packages:
#   gh auth refresh -s write:packages
#   gh auth token | docker login ghcr.io -u denkhaus --password-stdin

def build-one [dockerfile: string, tag: string, push: bool] {
    if not ($dockerfile | path exists) {
        print $"run-images: skip ($tag) \(($dockerfile) missing\)"
        return
    }
    let content = (open --raw $dockerfile)
    let hash = ($content | hash sha256)
    let label = "sh.seeds.toolchain.sha256"
    let wanted = $"($label)=($hash)"
    let inspect = (do {
        ^docker image inspect $tag --format $"{{index .Config.Labels \"($label)\"}}"
    } | complete)
    if $inspect.exit_code == 0 {
        let current = ($inspect.stdout | str trim)
        if $current == $hash {
            print $"run-images: ($tag) up to date \(sha ($hash | str substring 0..11)\)"
            return
        }
    }
    print $"run-images: building ($tag) from ($dockerfile) ..."
    let context = ($dockerfile | path dirname)
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
