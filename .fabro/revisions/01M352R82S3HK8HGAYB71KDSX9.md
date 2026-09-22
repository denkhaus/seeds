# Revision — run 01M352R82S3HK8HGAYB71KDSX9

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M352R82S3HK8HGAYB71KDSX9.md
- seeds filed: none — zero filing credit this pass (0 same-pass stale/superseded closes); all surviving findings journaled as overflow
- balance: 0 / 0 — no credit this pass
- basis: run 01M352R82S3HK8HGAYB71KDSX9, workflow version 725a90c36ca780e6d3cc4bd915d06c17a06ae7243c691edc70b0bdad5ef0dd82, commit f753aa9f433d9a52b05b5f09e8dab9087bd9230d
- revised_at_commit: f753aa9f433d9a52b05b5f09e8dab9087bd9230d (ADR-0015: engine drift signal for later judgement)

## Findings

### Capture remote-ref state in develop evidence capture — overflow (no credit)
Reviewer could not verify the v0.1.0 tag; 4 of 9 acceptance criteria rested on the implementer's `ls-remote` report (reviewer sandbox has no shell/git). No existing seed or open overflow covers this mechanism (open evidence.nu overflows cover blob collapse, newline emission, docs/ classification, verification-output capture — different arms).

- overflow: Capture remote-ref state in develop evidence capture — append a bounded remote-refs section to the capture header in `.fabro/workflows/develop/scripts/evidence.nu` (`git ls-remote --tags origin` plus `git tag -n99 -l <tag>` for spec-named tags); effect: the reviewer verifies remote-state claims by reading the capture, closing the trust gap on release/git-op seeds.

### Verify-before-push rule for immutable remote refs in implementer prompt — overflow (no credit)
Implementer cut the first tag with a typo'd message and had to delete/re-push; the brief's verify step runs after push. No existing seed or open overflow covers release/git-op prompt hygiene.

- overflow: Verify-before-push rule for immutable remote refs in implementer prompt — add one sentence to the hard-rules block of `.fabro/workflows/develop/prompts/implementer.md`: render immutable remote refs locally (`git tag -n99 -l <tag>`) before `git push origin <tag>`; effect: no delete/re-push cycles on future release seeds.

### Add engine response keys to planner context_allow_keys — duplicate of open overflow
- overflow-dup: Add output.planner to planner context_allow_keys (open in 01M34N0KNQSGJQVXN1A6EDTJG1.md)

### Allowlist intentional routing schemas in prompt-lint — duplicate of open overflow
- overflow-dup: prompt-lint.nu: suppress by-design routing-named schema warnings and mark date pin reviewed (open in 01M324AZVMHDSCSAMVP4WTZR1R.md)
