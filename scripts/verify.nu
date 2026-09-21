#!/usr/bin/env nu
# Deterministic verification dispatcher (fabro-6e7f, user directive
# 2026-09-14): ONE entry point that decides WHICH checks to run from the
# calling agent's workflow STEP plus the current diff — replacing the
# prompt-encoded scope rules that drifted (fabro-0d56, 01M20T9S8).
#
# Stages:
#   implementer — crate-scoped fmt + clippy (default features, -D warnings)
#                 on every code-touched crate; the FULL crate suite ONLY
#                 for test-file-touched crates; a compile check for
#                 code-touched-only crates. Never the gate, never the
#                 workspace suite (the deterministic tester owns both).
# Exit 0 = all checks green. First red stops (qualitygate style:
# `do { ^cmd } | complete`).
#
# Touched-crate + base detection reuse the qualitygate.nu pattern. The
# stage arrives as an ARGUMENT in v1; engine-side FABRO_STAGE env
# injection is the follow-up fork recorded in fabro-6e7f.

const PINNED_TOOLCHAIN = "nightly-2026-04-14"

# A test file by the repo's layout: tests/ dirs, *_tests.rs companions,
# or a tests.rs module file inside src/.
def is-test-file [p: string] {
    ($p | str contains '/tests/') or ($p | str ends-with '_tests.rs') or ($p | str ends-with '/tests.rs')
}

def current-branch [] {
    (git branch --show-current | str trim)
}

def run-base [] {
    let run_id = (
        current-branch
        | parse --regex 'fabro/run/(?P<id>[^/]+)$'
        | get -o id.0
        | default '')
    if ($run_id | is-not-empty) {
        let subjects = (git log --format='%H %s' -n 40)
        let checkpoint = ($subjects | lines | where {|l|
            # Plain-string concatenation, NOT $'...' interpolation: an
            # interpolated string parses every (...) — including the regex
            # group (?P<sha>...) — as a subexpression (parse bug found by
            # run 01M2GVW7GGGB's implementer; lint-nu guards the class).
            ($l | parse --regex ('^(?P<sha>[0-9a-f]+) fabro\(' + $run_id + '\): ') | get -o sha.0 | is-not-empty)
        })
        if ($checkpoint | is-not-empty) {
            let sha = ($checkpoint | first | parse --regex '^(?P<sha>[0-9a-f]+)' | get sha.0)
            {base: (git rev-parse $"($sha)^"), hit: true}
        } else {
            {base: (git rev-parse HEAD), hit: false}
        }
    } else {
        # Host / non-run checkout: diff against origin's branch tip when
        # available, else HEAD (no uncommitted-work signal on a clean tree).
        let br = (current-branch)
        let origin = $"origin/($br)"
        let has_origin = (do { git rev-parse --verify --quiet $origin } | complete | get exit_code) == 0
        if $has_origin {
            {base: (git rev-parse $origin), hit: true}
        } else {
            {base: (git rev-parse HEAD), hit: false}
        }
    }
}

# Crates touched by the diff, split into {code: [...], tests: [...]}:
# `tests` = crates with at least one test-file change, `code` = every
# touched crate (tests included — fmt/clippy cover them too).
def touched [] {
    let base = (run-base).base
    let paths = (git diff --name-only $base | lines | compact)
    # crates/<name>/** is this repo's layout (seeds-9482); the lib/... arms
    # stay for portability of the pattern.
    let crate_paths = ($paths | where {|p| ($p | str starts-with 'lib/') or ($p | str starts-with 'crates/') })
    let crates = ($crate_paths
        | each {|p| $p | parse --regex '^(?:lib/(?:apps|components|foundation)|crates)/(?P<crate>[^/]+)/' }
        | flatten
        | get -o crate
        | uniq
        | compact)
    let test_crates = ($crate_paths
        | where {|p| is-test-file $p }
        | each {|p| $p | parse --regex '^(?:lib/(?:apps|components|foundation)|crates)/(?P<crate>[^/]+)/' }
        | flatten
        | get -o crate
        | uniq
        | compact)
    # Loud failure (seeds-9482): an all-Rust diff that derives no crate used
    # to fall through to "nothing to verify" and exit green.
    let rs_touched = ($paths | where {|p| $p | str ends-with '.rs' } | is-not-empty)
    if $rs_touched and ($crates | is-empty) {
        print "verify: FAIL diff touches *.rs files but no workspace crate could be derived (expected lib/... or crates/<name>/... paths)"
        exit 1
    }
    {code: $crates, tests: $test_crates}
}

# sd-0.5.15 compat reference provisioning (seeds-25b5): the seeds
# crate's compat/differential suites skip-with-note when the reference
# is missing — before this stage RUNS that crate's suite, provision it
# via the pinned bun.lock and fail hard when it does not answer 0.5.15
# (silent skips are gate-unacceptable; local nextest keeps the note).
def provision-sd-reference [] {
    print '== provisioning sd-0.5.15 compat reference (seeds compat + differential suites) =='
    let dir = 'crates/seeds/tests/fixtures/sd-reference'
    let install = (do { cd $dir; ^bun install --frozen-lockfile } | complete)
    let version = (do { cd $dir; ^./sd --version } | complete)
    let ok = ($install.exit_code == 0) and ($version.exit_code == 0) and (($version.stdout | str trim) == '0.5.15')
    if not $ok {
        print "verify: FAIL sd-0.5.15 reference provisioning — provision crates/seeds/tests/fixtures/sd-reference (bun install); see its README.md"
        print ($install.stdout + $install.stderr | str trim -r -c "\n" | lines | last 10)
        return false
    }
    print 'verify: PASS sd-0.5.15 reference answers'
    true
}

def stage-implementer [] {
    let t = (touched)
    if ($t.code | is-empty) {
        print "verify: no lib/ crates touched — nothing to verify (parse-level checks belong to the brief)"
        return
    }
    print $"verify: code-touched=($t.code | str join ', ') test-file-touched=($t.tests | str join ', ')"
    mut failed = false
    for c in $t.code {
        print $"== fmt -p ($c) =="
        let res = (do { ^cargo $'+($PINNED_TOOLCHAIN)' fmt -p $c --check } | complete)
        if $res.exit_code != 0 {
            print $res.stderr
            print $"verify: FAIL fmt ($c)"
            $env.LAST_VERIFY_FAIL = "fmt"
            return
        }
        print $"verify: PASS fmt ($c)"
    }
    for c in $t.code {
        let label = ("== clippy -p " + $c + " (default features, deny warnings) ==")
        print $label
        let res = (do { ^cargo $'+($PINNED_TOOLCHAIN)' clippy -p $c --all-targets -- -D warnings } | complete)
        if $res.exit_code != 0 {
            print ($res.stderr | str substring 0..2000)
            print $"verify: FAIL clippy ($c)"
            return
        }
        print $"verify: PASS clippy ($c)"
    }
    let code_only = ($t.code | where {|c| $c not-in $t.tests })
    for c in $code_only {
        print $"== compile check -p ($c) =="
        let res = (do { ^cargo check -p $c --quiet } | complete)
        if $res.exit_code != 0 {
            print ($res.stderr | str substring 0..2000)
            print $"verify: FAIL compile ($c)"
            return
        }
        print $"verify: PASS compile ($c)"
    }
    if ('seeds' in $t.tests) {
        if not (provision-sd-reference) { return }
    }
    for c in $t.tests {
        let label = ("== nextest -p " + $c + " (full crate suite: test files touched) ==")
        print $label
        let res = (do { ^cargo nextest run -p $c --no-fail-fast } | complete)
        if $res.exit_code != 0 {
            print ($res.stdout + $res.stderr | str substring 0..3000)
            print $"verify: FAIL nextest ($c)"
            return
        }
        print $"verify: PASS nextest ($c)"
    }
    print "verify: implementer stage green"
}

def main [stage: string] {
    match $stage {
        'implementer' => { stage-implementer }
        _ => {
            print $"verify: unknown stage '($stage)' (v1: implementer)"
            exit 2
        }
    }
}
