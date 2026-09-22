# Revision — run 01M336MYEF1R5C1JMC6WHFTCQY

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M336MYEF1R5C1JMC6WHFTCQY.md
- seeds filed: none — zero credit this pass (0 same-pass stale/superseded closes); three findings duplicate open overflows, three ride as new overflows
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M336MYEF1R5C1JMC6WHFTCQY, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit e2aef9a6f5cfed8f07d8ad6e27bb5d3470c07c0a
- revised_at_commit: e2aef9a6f5cfed8f07d8ad6e27bb5d3470c07c0a (ADR-0015: engine drift signal for later judgement)

## Findings

### Fix tracker-guard.nu column_not_found crash (ts filter + guarded ls)
- overflow-dup: Fix tracker-guard.nu ts crash and restore the stale-claim requeue arm (open in 01M33374S3J9BC6M1FFTNWWAP9.md, line 30) — same file, same ts-crash mechanism (optional access + null guard at `tracker-guard.nu` :166/:172); this pass's evidence adds the guarded `ls` glob at `:152` for the re-filing pass to fold in.

### Engine demand: sandbox mtime 1970 on file-tool writes, cargo re-runs stale binaries
- overflow-dup: Mitigate cargo fingerprint staleness: touch changed sources before cargo in verify.nu (open in 01M32J3AFYGBV109Z58SHHBA1J.md, line 31) — same interim arm (touch changed sources before cargo) and the same engine-side root cause; this pass adds the engine-demand framing (denkhaus/fabro sandbox driver must set real mtimes) and the duplicate-lesson-filing evidence (`mx-876053`/`mx-7aab64` vs `mx-dd17ee`) for the re-filing pass.

### Stop head-truncating verify failure output; print FAIL/error lines instead
- overflow: verify.nu failure output — in `.fabro/scripts/verify.nu` (also `scripts/verify.nu` referenced by prompts) emit the FAIL/error lines (`$out | lines | where {|l| $l =~ "FAIL|^error"} | str join "\n"`) or the tail instead of `str substring 0..3000` at `:168` (and the 2000-char compile-stderr cut at `:154`); effect: with the 70-test suite the failing test name was cut off entirely in run 01M336MYEF1R5C1JMC6WHFTCQY, forcing a blind `cargo nextest run --no-fail-fast` re-run inside the dominant (86% of cost) implementer stage; first-failure diagnosis in one shot. Closed seeds-9482 fixed touched-crate detection in the same file, not output truncation. No open overflow covers truncation (01M3201Q8GTC0EPT0S5YMK0Z32.md line 20 is touched-path derivation — different mechanism).

### Raise reviewer preamble_inline_max_kb 16 → 64 or emit raw evidence txt
- overflow-dup: Raise reviewer preamble_inline_max_kb from 16 to 24 (open in 01M32GNPT143Q82TJ217SNCC6A.md, line 19) — same render-cap mechanism; this pass's evidence (44,540-byte capture, 7 manual read_file calls) argues for the higher 64 value or a raw `.txt` capture; the re-filing pass should take the stronger number.

### Make ml record merge-on-name real and dedupe the stale-binary records
- overflow: ml record merge-on-name — seed mulch-CLI work (true merge-on-`--name` upsert or enforced search-before-record), a one-time dedupe collapsing `mx-dd17ee`/`mx-876053`/`mx-7aab64` into one record, and amend the implementer.md lesson-capture paragraph to match real behavior; effect: `ml search` stops returning duplicate stubs that starve the real record of confirmation evidence (implementer prompt documents upsert behavior the tool lacks — journaled painpoint in run 01M336MYEF1R5C1JMC6WHFTCQY). Distinct from open ml overflows (01M326WXXVST02QX4QKQQW9NE7.md line 25 `--content` flag doc; 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 18 mulch init): those fix docs/init, this fixes upsert semantics and the record dedupe.

### Add fabro-866a chaining rule for planner basis probes
- overflow: planner probe chaining — in `.fabro/workflows/develop/prompts/planner.md` step 3 (stale-basis check), add the fabro-866a discipline: dispatch basis probes and the style-guide TOC lookup as ONE chained shell call each, never one round per read; effect: ~3–4 fewer LLM rounds (~20–30 s) per claim (run 01M336MYEF1R5C1JMC6WHFTCQY planner spent 5 read-only rounds: three on sd/sd-ref/sync presence, two on the style-guide TOC); read-only and trivially safe. No seed or open overflow covers planner probe discipline (existing planner-prompt overflows are token bounding, body pre-reading, and claim verification — different mechanisms).
