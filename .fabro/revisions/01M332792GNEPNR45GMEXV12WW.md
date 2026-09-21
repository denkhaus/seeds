# Revision — run 01M332792GNEPNR45GMEXV12WW

- status reviewed: succeeded
- review: .fabro/reviews/develop/01M332792GNEPNR45GMEXV12WW.md
- seeds filed: none — zero same-pass close credit (ADR-0022), all findings journaled as overflow
- balance: 0 non-exempt seeds filed / 0 — no credit this pass
- basis: run 01M332792GNEPNR45GMEXV12WW, workflow version ff257a89b9734e9055b2a5c8fcfb380a69ba3f0819c5ec3d09ae9b6f09ab1918, commit 8091b90d24cabf33b70495c5ce93137766d8b2e4
- revised_at_commit: 8091b90d24cabf33b70495c5ce93137766d8b2e4 (ADR-0015: engine drift signal for later judgement)

## Findings

### Fix false duplicate verdict on operator approval commits in dup-run-check.nu
- filed id: none — overflow-to-journal (zero credit)
- overflow-dup: dup-run-check: match seed ids in subject and trailer lines only, never full commit bodies (open in 01M3201Q8GTC0EPT0S5YMK0Z32.md) — same file, same false-duplicate failure mode; the classify-filed regex extension (add \bapproval\b, \boption [a-z]\b, ^operator:; plus a diff-content arm for marker+tracker-only commits) should be consolidated as an arm of that finding when it is re-filed. Basis: operator commit c107c43 classified as foreign implementation and forced verdict duplicate.

### Fix 8-char extension cap truncating Dockerfile.toolchain in anchor_check.nu
- filed id: none — overflow-to-journal (zero credit)
- overflow: Fix 8-char extension cap truncating Dockerfile.toolchain in anchor_check.nu — in `.fabro/workflows/develop/scripts/anchor_check.nu` lines 37 and 157 raise the extension bound from `{1,8}` to `{1,16}` or add a trailing `(?![A-Za-z])` boundary; effect: `anchors_ok` becomes trustworthy, ~1 LLM round + 2 shell calls saved per affected run (observed `.fabro/Dockerfile.toolchai` false missing_file on this run). Distinct from the planner-preflight.nu mid-token truncation overflow (01M326WXXVST02QX4QKQQW9NE7.md line 19) — different file; consolidate both truncation arms into one multi-arm seed when re-filed.

### Stop classifying docs/ as loop churn in evidence.nu
- filed id: none — overflow-to-journal (zero credit)
- overflow: Stop classifying docs/ as loop churn in evidence.nu — drop `"docs/"` from `LOOP_PREFIXES` in `.fabro/workflows/develop/scripts/evidence.nu` line 34 (or exempt paths cited by the in-progress seed body); effect: anomaly section contains only true residue, removing the reviewer's false-adjudication round and Changes-requested bounce tail for doc-scoped seeds (this run's 55-line `docs/workflows-permission.md` deliverable landed in the anomaly section).

### Deduplicate preflight JSON in the planner preamble via preamble_stages_ignore
- filed id: none — overflow-to-journal (zero credit)
- overflow-dup: Stop rendering preflight JSON twice in preambles (open in 01M31Z2XTPMMA78KHR4D9K84E6.md line 28) — same concrete change (add `preflight` to planner node `preamble_stages_ignore`, keep the `output.preflight` context key); that entry already proposes the wider node set (also implementer, tracker_guard, claim_check).

### Forbid pre-reading non-top candidate bodies in planner.md
- filed id: none — overflow-to-journal (zero credit)
- overflow: Forbid pre-reading non-top candidate bodies in planner.md — in `.fabro/workflows/develop/prompts/planner.md` step 2 add one sentence: do not fetch a candidate's full body until you fall through to it; the preflight table's verdict/subject/risk fields suffice for ordering; effect: ~1 LLM round + ~4 KB dead context saved per run (planner fetched seeds-25b5's 4 KB body although the claim went to seeds-37a6).

### Fold terminal-notification enablement into seeds-d2c7's resume procedure
- filed id: none — overflow-to-journal (zero credit)
- overflow: Fold terminal-notification enablement into seeds-d2c7's resume procedure — update open seed seeds-d2c7's body (via `seeds update`) to add "enable `notifications.terminal` for the develop line in run settings" to the resume procedure; effect: terminal events reach the operator without recreating the per-session 8-minute heartbeat (this run's completion and auto-merge PR #25 were silent; `notifications.terminal.enabled: false`). Note for the re-filing pass: this is a `seeds update` on an existing open seed, not a new filing — confirm ownership allows the edit before acting.
