# Revision — run 01M33374S3J9BC6M1FFTNWWAP9

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M33374S3J9BC6M1FFTNWWAP9.md
- seeds filed: none — zero balance credit this pass (no same-pass stale/superseded closes); all five surviving findings journaled as overflow below
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M33374S3J9BC6M1FFTNWWAP9, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit 4082ec06cc18aa72222375a4e3184a8071b50679
- revised_at_commit: 4082ec06cc18aa72222375a4e3184a8071b50679 (ADR-0015: engine drift signal for later judgement)

## Findings

### Weight files touched in dup-run-check before a duplicate verdict
- filed id: overflow (no credit this pass)
- concrete change: in `.fabro/scripts/dup-run-check.nu` (classification, lines ~81-91) and the mirrored arm in `.fabro/workflows/develop/scripts/planner-preflight.nu`, require an implementation match to touch product code (`crates/**`, `scripts/**`, `.fabro/**` via `git show --name-only`); `.github`-only or doc-only matches downgrade to advisory `partial_landing`; a self-closure match takes precedence over foreign partial matches. effect: removes the wrongful-Blocked/re-implementation class for every seed with a CI-only or bookkeeping commit naming it (preflight flagged seeds-25b5 `duplicate` off commit 3d5b866 which landed only `.github/workflows/ci.yml`, while closing commit 8fd32e9 carries closure `self`; seeds-69ae closed — default `--base` only, fabro-395b lineage is foreign tracker). Distinct mechanism from the open overflows on `dup-run-check` (match-scope 01M3201Q8GTC0EPT0S5YMK0Z32.md line 15, ref-derivation 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 19) — consolidate into one multi-arm `dup-run-check` seed when re-filed.

### Stop sandbox epoch-mtime stale-binary re-runs via implementer prompt stopgap
- overflow-dup: Mitigate cargo fingerprint staleness: touch changed sources before cargo in verify.nu (open in 01M32J3AFYGBV109Z58SHHBA1J.md) — this run's variant adds the `implementer.md` hard-rule-(c) stopgap arm (touch every file-tool-edited source in the same shell call as the build/test) and the root-cause engine fix (file-tool write path must set real mtimes, denkhaus/fabro); merge as arms of one seed when re-filed. Lesson mx-dd17ee records the class; no open seeds- id covers it.

### Fix tracker-guard.nu ts crash and restore the stale-claim requeue arm
- filed id: overflow (no credit this pass)
- concrete change: in `.fabro/workflows/develop/scripts/tracker-guard.nu` lines 166 and 172, replace `$recs | get ts | last` with optional access (`$recs | get -o ts | where {|t| $t != null} | last` plus a `default null` guard). effect: restores the stale-claim requeue safety arm on every run and removes the recurring 1.5 KB nu-error preamble noise and journal duplication (guard crashed at `tracker-guard.nu:166` when a journal record lacked `ts`; no open seed mentions tracker-guard.nu — seeds-9fa3/81cd target other scripts).

### Materialize evidence blobs as multi-line text so reviewer paging works
- filed id: overflow (no credit this pass)
- concrete change: engine-side (denkhaus/fabro) blob materialization writes pretty-printed or verbatim multi-line files so the reviewer prompt's prescribed `read_file` offset/limit paging reaches the middle of the 69.9 KB capture. effect: large-diff reviews stop paying extra tool rounds and stop risking evidence-delivery re-cycles (~5 KB of diff unreadable this run; reviewer re-verified from repo files). Complementary to the open repo-side evidence.nu overflows (blob collapsing 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 20; newline rendering 01M3201Q8GTC0EPT0S5YMK0Z32.md line 25; render-cap 01M32GNPT143Q82TJ217SNCC6A.md line 19) — different target (engine blob layout, no seeds- seed covers it; fabro-1e9f was inline limits).

## Overflow (canonical machine-visible entries)

- overflow: Weight files touched in dup-run-check before a duplicate verdict — require implementation matches in `.fabro/scripts/dup-run-check.nu` (lines ~81-91) and the mirrored `planner-preflight.nu` arm to touch product code (`crates/**`, `scripts/**`, `.fabro/**` via `git show --name-only`), downgrade `.github`-only/doc-only matches to advisory `partial_landing`, let self-closure match take precedence; effect: removes the wrongful-Blocked/re-implementation class for seeds with CI-only or bookkeeping commits naming them (seeds-25b5 flagged `duplicate` off 3d5b866 which landed only `.github/workflows/ci.yml`; consolidate with open dup-run-check overflows 01M3201Q8GTC0EPT0S5YMK0Z32.md line 15 and 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 19 when re-filed).
- overflow: Fix tracker-guard.nu ts crash and restore the stale-claim requeue arm — optional access `$recs | get -o ts | where {|t| $t != null} | last` with `default null` guard at `.fabro/workflows/develop/scripts/tracker-guard.nu` lines 166 and 172; effect: requeue safety arm runs every run and the recurring 1.5 KB nu-error preamble noise plus journal duplication disappear.
- overflow: Materialize evidence blobs as multi-line text so reviewer paging works — engine-side (denkhaus/fabro) blob materialization writes pretty-printed/verbatim multi-line files; effect: `read_file` offset/limit paging reaches the middle of the 69.9 KB capture, large-diff reviews stop paying extra tool rounds and evidence-delivery re-cycles (complementary to open evidence.nu overflows: 01M2ZYVQ7CBSY3T0FEYBE9JK74.md line 20, 01M3201Q8GTC0EPT0S5YMK0Z32.md line 25, 01M32GNPT143Q82TJ217SNCC6A.md line 19).
- overflow: Disambiguate the publish_blocked_risk routing rule in the preflight legend — one legend sentence in `.fabro/workflows/develop/scripts/planner-preflight.nu` mirrored in `planner.md` steps 3/4: route Blocked only when the seed's REMAINING acceptance criteria require `.github/workflows/**` edits, else journal the adjudication and claim normally; effect: removes a ~30 s planner reasoning round per flagged seed and prevents wrongful run-failing Blocks on top-priority seeds.

### Disambiguate the publish_blocked_risk routing rule in the preflight legend
- filed id: overflow (no credit this pass)
- concrete change: add one sentence to the legend emitted by `.fabro/workflows/develop/scripts/planner-preflight.nu`, mirrored in `.fabro/workflows/develop/prompts/planner.md` steps 3/4: route Blocked only when the seed's REMAINING acceptance criteria require `.github/workflows/**` edits; when landed commits already cover the workflow-file part, journal the adjudication and claim normally. effect: removes a ~30 s / 1,395-reasoning-token planner round per flagged seed and prevents a wrongful run-failing Block on top-priority seeds (planner's heaviest round 23:01:57-23:02:24 reconciling legend vs outcome contract; seeds-7e57 closed added the probe arm, not the adjudication rule).
