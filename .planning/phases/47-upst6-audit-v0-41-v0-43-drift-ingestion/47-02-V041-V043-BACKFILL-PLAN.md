---
phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion
plan: 02
type: execute
wave: 2
depends_on: [47-01]
files_modified:
  - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md
  - .planning/ROADMAP.md
  - .planning/STATE.md
  - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md
autonomous: false
requirements: [REQ-DRIFT-INGEST-01]
tags: [upstream-parity, drift-audit, ledger, backfill, drift-ingest, absorbed-via, paper-trail]

must_haves:
  truths:
    - "DIVERGENCE-LEDGER-v041-v043-backfill.md exists at .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md (D-47-E2 phase-local convention; alongside DIVERGENCE-LEDGER.md from Plan 47-01)"
    - "Ledger frontmatter records D-47-A2 reproducibility fields verbatim adapted for backfill range: range=v0.41.0..v0.43.0, upstream_head_at_audit captured at first commit of Plan 47-02 per D-47-A3, drift_tool_sh_sha=0834aa664fbaf4c5e41af5debece292992211559, drift_tool_ps1_sha=0834aa664fbaf4c5e41af5debece292992211559, drift_tool_invocation locked (verbatim 'make check-upstream-drift ARGS=\"--from v0.41.0 --to v0.43.0 --format json\"'), framing='backfill-cleanup, not parity-sync (per REQ-DRIFT-INGEST-01)', deferral_origin='v2.3 scope-lock 2026-04-29', date"
    - "Every cluster header carries one of four dispositions: will-sync / fork-preserve / won't-sync / split (D-47-C3 standard 4-disposition vocab — same as UPST6 ledger; backfill uses absorbed-via: column for per-commit traceability)"
    - "Every cluster's commit-row table follows D-47-A5 schema PLUS absorbed-via: column per D-47-C3: sha + subject + upstream-tag + categories + files-changed-count + windows-touch + absorbed-via"
    - "Every commit row has an absorbed-via: column value drawn from the 6 standard values per D-47-C3: phase-22-plan-XX-commit-XXXXXXXX, phase-34-plan-XX-commit-XXXXXXXX, unmatched, intentionally-skipped, fork-divergence, ambiguous-see-cluster-rationale"
    - "Backfill ledger DOES NOT have a '## ADR review' section per D-47-C4 (retroactive paper-trail on 2-year-old range does not warrant fresh Option A 'continue' verdict); falsifiable via grep -c \"^## ADR review$\" returning 0"
    - "Explicit '## Empirical cross-check' section present with ≥4 fork-shared files per D-47-D1 (Phase 47 SC#3 closure across BOTH ledgers); preferentially samples files most-likely-touched-by-Phase-22/34 absorption per D-47-C4 + CONTEXT § Claude's Discretion (crates/nono/src/policy.rs, crates/nono/src/audit.rs, crates/nono-cli/src/profile/, etc.); falsifiable via grep -c \"^## Empirical cross-check\" returning 1"
    - "Total row count across all cluster commit-row tables >= drift-tool total_unique_commits for v0.41.0..v0.43.0 (REQ-DRIFT-INGEST-01 acceptance + D-47-B4 close-gate step 2; exact coverage zero gap)"
    - "Any absorbed-via: unmatched rows are flagged at the end of the ledger under a '## Phase 48 hand-off' subsection per D-47-C1 'surface any missed cherry-picks' framing; flagged for Phase 48 absorption alongside UPST6 work"
    - "Phase 47 ROADMAP entry flipped to [x] with '(completed YYYY-MM-DD)' appended; Phase Details Plans counter flipped to '2 / 2 plans complete' with both Plan 47-01 + 47-02 marked [x] (D-47-B4 strict-both-close gate satisfied at Plan 47-02 close)"
    - "STATE.md frontmatter completed_plans counter bumped; STATE.md Current Position flipped to Phase 47 (upst6-audit-v0-41-v0-43-drift-ingestion) Complete — ready for verification; Last activity stamped"
    - "STATE.md Accumulated Context gains a Plan 47-02 close entry under Key Decisions (v2.6) capturing range, lock-sha, cluster count, commit count, disposition breakdown, windows-touch:yes count, absorbed-via distribution (phase-22 count / phase-34 count / unmatched count / intentionally-skipped count / fork-divergence count / ambiguous count), empirical cross-check files sampled, Phase 48 hand-off candidate count, DCO sign-off"
    - "Drift-tool re-run is idempotent: make check-upstream-drift ARGS=\"--from v0.41.0 --to v0.43.0 --format json\" exits 0 after plan close (D-47-B4 close-gate step 1 applied to backfill range)"
    - "D-47-E5 / D-47-B4-step-8 Windows-only-files invariant trivially honored: git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/ returns zero files (Plan 47-02 ships zero .rs / .toml / .sh / .ps1 / Makefile edits)"
    - "Raw drift-tool JSON output redirects to ci-logs-local/drift/<timestamp>-v041-v043.json per D-47-E1 / D-33-A2 inherited; NOT committed (ci-logs-local/ is in .gitignore)"
    - "Phase 47 phase-level close satisfied per D-47-B4 strict-both-close gate (Plan 47-01 + Plan 47-02 ledgers BOTH disposition-complete); REQ-UPST6-01 AND REQ-DRIFT-INGEST-01 both satisfied at the same close-event"
  artifacts:
    - path: ".planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md"
      provides: "Audited backfill inventory of v0.41.0..v0.43.0 fork-vs-upstream divergence with per-cluster dispositions, windows-touch column, absorbed-via: per-commit traceability against Phase 22 + 34 historical absorption record, ## Empirical cross-check section ≥4 fork-shared files, and ## Phase 48 hand-off subsection listing any unmatched candidates"
      contains: "## Empirical cross-check, ## Phase 48 hand-off, ### Cluster, **Disposition:**, | sha | subject | upstream-tag | categories | files-changed | windows-touch | absorbed-via |, absorbed-via:, framing: backfill-cleanup, deferral_origin: v2.3 scope-lock 2026-04-29, range: v0.41.0..v0.43.0, drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559"
    - path: ".planning/ROADMAP.md"
      provides: "Phase 47 v2.6 milestone-block entry flipped to [x] with completion date; Phase 47 Plans counter flipped to 2/2; Plan 47-02 marked [x]"
      contains: "[x] **Phase 47: UPST6 audit + v0.41–v0.43 drift ingestion**, Plans: 2 / 2 plans complete, [x] 47-02-V041-V043-BACKFILL-PLAN.md"
    - path: ".planning/STATE.md"
      provides: "Plan 47-02 close entry under Key Decisions (v2.6); completed_plans counter bumped; Current Position flipped to Phase 47 ready for verification"
      contains: "Phase 47 Plan 47-02, Phase 47 Complete"
    - path: ".planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md"
      provides: "Plan 47-02 close summary mirroring Phase 42 Plan-01-SUMMARY shape adapted for backfill"
      contains: "REQ-DRIFT-INGEST-01, DIVERGENCE-LEDGER-v041-v043-backfill, absorbed-via, Empirical cross-check, Phase 48 hand-off"
  key_links:
    - from: "DIVERGENCE-LEDGER-v041-v043-backfill.md frontmatter"
      to: "drift-tool reproducibility (D-47-A2 / D-47-A3 adapted for backfill)"
      via: "frontmatter records range + upstream_head_at_audit + drift_tool shas + invocation verbatim; backfill range is fully historical (v0.41.0..v0.43.0) so HEAD-anchor is informational, but schema uniformity with UPST6 ledger is preserved per D-47-A3"
      pattern: "drift_tool_(sh|ps1)_sha|upstream_head_at_audit|drift_tool_invocation|framing:.*backfill-cleanup|deferral_origin:.*v2.3 scope-lock"
    - from: "DIVERGENCE-LEDGER-v041-v043-backfill.md commit-row absorbed-via: column"
      to: "Phase 22 + 34 historical absorption record (pre-D-19 trailer convention era)"
      via: "subject-line + diff fingerprint match against fork main per D-47-C2; manual auditor judgment for ambiguous matches; robust against trailer-less pre-D-19 absorption (only 11 unique Upstream-commit: trailers in fork main currently)"
      pattern: "absorbed-via:.*(phase-22-plan|phase-34-plan|unmatched|intentionally-skipped|fork-divergence|ambiguous-see-cluster-rationale)"
    - from: "DIVERGENCE-LEDGER-v041-v043-backfill.md ## Phase 48 hand-off subsection"
      to: "Phase 48 UPST6 sync execution input (unmatched candidates for absorption alongside UPST6 work)"
      via: "Phase 48 plan-phase reads this subsection in parallel with UPST6 ledger's per-cluster dispositions; absorbs any unmatched candidates as new cherry-picks alongside the v0.54.0+ work"
      pattern: "## Phase 48 hand-off|absorbed-via: unmatched|Phase 48"
    - from: "DIVERGENCE-LEDGER-v041-v043-backfill.md ## Empirical cross-check section"
      to: "feedback_cluster_isolation_invalid memory closure (D-47-C4 + D-47-D1 ≥4 files retroactive application)"
      via: "≥4 fork-shared file walk on backfill range; preferentially samples Phase 22/34-era files (crates/nono/src/policy.rs, crates/nono/src/audit.rs, crates/nono-cli/src/profile/); closes the cluster-isolation-invalid lesson retroactively (Phase 34 may have hit re-export deps it didn't recognize)"
      pattern: "## Empirical cross-check|crates/nono/src/(policy|audit)|crates/nono-cli/src/profile"
    - from: "ROADMAP § Phase 47 close flip ([x] completed YYYY-MM-DD)"
      to: "D-47-B4 strict-both-close gate satisfied (Plan 47-01 + Plan 47-02 BOTH disposition-complete)"
      via: "Plan 47-02 is the last plan of Phase 47; closing it satisfies the gate per the user's explicit rejection of load-bearing-UPST6-only; REQ-UPST6-01 + REQ-DRIFT-INGEST-01 both closed at the same close-event"
      pattern: "\\[x\\] \\*\\*Phase 47:.*completed|2 / 2 plans complete"
---

<objective>
Run the D-47-A2-locked drift-tool invocation against the v0.41.0..v0.43.0 backfill range, lock `upstream_head_at_audit` at first commit of this plan (D-47-A3 — informational for historical range; schema uniformity per D-47-A3), curate `DIVERGENCE-LEDGER-v041-v043-backfill.md` mirroring Plan 47-01 ledger shape with: (a) the D-47-A5 commit-row schema PLUS the D-47-C3 `absorbed-via:` column reconstructing per-commit traceability against Phase 22 + 34 historical absorption record via D-47-C2 subject-line + diff fingerprint match, (b) standard 4-disposition vocab per D-47-C3 (`will-sync` / `fork-preserve` / `won't-sync` / `split`), (c) NO `## ADR review` section per D-47-C4 (retroactive paper-trail does not warrant fresh ADR verdict), (d) an explicit `## Empirical cross-check` section ≥4 fork-shared files per D-47-C4 + D-47-D1 (Phase 47 SC#3 closure across BOTH ledgers; preferentially sample Phase 22/34-era files), (e) a `## Phase 48 hand-off` subsection at the end of the ledger flagging any `absorbed-via: unmatched` rows for Phase 48 absorption alongside UPST6 work per D-47-C1.

Purpose: REQ-DRIFT-INGEST-01 was deferred at v2.3 scope-lock 2026-04-29 (second slip unacceptable per D-47-B4 strict-both-close rejection rationale). Plan 47-02 closes the deferral and serves as the first real load of the v2.2 Phase 24 DRIFT-01/02 tooling on a long-deferred range — exactly the scenario most likely to surface drift-tool category miscategorizations or file-filter blind spots per D-47-E10. Backfill is framed as "backfill-cleanup, not parity-sync" per ROADMAP SC#4 + CONTEXT § Phase Boundary; per-commit `absorbed-via:` column reconstructs the historical Phase 22 + 34 absorption record using subject-line + diff fingerprint match (D-47-C2) since Phase 22/34 pre-dated D-19 trailer convention (only 11 unique `Upstream-commit:` trailers exist in fork main).

Output: 4 files committed across atomic commits per the Plan 47-01 precedent:
1. `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md` (NEW, ~120-180 lines per CONTEXT § Existing Code Insights size estimate for 19 commits / 4-6 clusters)
2. `.planning/ROADMAP.md` (modified — Phase 47 v2.6 entry flipped to [x] with completion date; Plans counter flipped to 2/2; Plan 47-02 marked [x])
3. `.planning/STATE.md` (modified — frontmatter bump + Current Position flipped to Phase 47 complete + Plan 47-02 close entry under Key Decisions (v2.6))
4. `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md` (NEW)
5. ZERO `.rs` / `.toml` / `.sh` / `.ps1` / `Makefile` edits (D-47-E5 / D-47-B4 step 8 trivially honored).

Auto-mode flag for executor: `autonomous: false`. Audit-walk decisions (cluster grouping, disposition choice, absorbed-via reconstruction via subject + diff fingerprint judgment, Phase 48 hand-off candidate flagging) require human-in-the-loop judgment. Mechanical scaffolding tasks (frontmatter, drift-tool invocation, grep verifications, ROADMAP flip, STATE update, SUMMARY commit) are auto-runnable.

**Sequential plan ordering per D-47-B3:** Plan 47-02 depends on Plan 47-01 close. Plan 47-01 must be `status: complete` before Plan 47-02 starts.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md
@.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md
@.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md
@.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md
@.planning/phases/42-upst5-audit/42-01-PLAN.md

<!-- Phase 22 + 34 historical absorption record (READ for absorbed-via: column reconstruction per D-47-C2 subject + diff fingerprint match) -->
<!-- Note: Phase 22 + 34 pre-dated D-19 trailer convention; only 11 unique Upstream-commit: trailers exist in fork main currently -->
<!-- Auditor reads each Plan 22-XX SUMMARY and Plan 34-XX SUMMARY at audit-walk time to enumerate the historical claimed-absorption -->
</context>

<tasks>

<task type="auto">
  <name>Task 1: Re-capture upstream_head_at_audit + run drift-tool for backfill v0.41.0..v0.43.0 (mechanical preamble)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-A2 frontmatter; D-47-A3 first-commit-of-Plan-47-02 lock timing; D-47-E1 raw-JSON-not-committed)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md (verify Plan 47-01 status: complete before starting Plan 47-02 per D-47-B3 sequential ordering)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-LOCK-NOTES.md (referential — Plan 47-01's HEAD lock-sha for cross-ledger correlation)
    - scripts/check-upstream-drift.sh (drift-tool source; sha 0834aa664fbaf4c5e41af5debece292992211559 invariant)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-LOCK-NOTES.md, ci-logs-local/drift/&lt;UTC-timestamp&gt;-v041-v043.json</files>
  <action>
    **Precondition check:** Verify Plan 47-01 closed with `status: complete` by reading `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md`. If not complete, ABORT and surface via AskUserQuestion (D-47-B3 sequential ordering).

    **Step 1 — Re-fetch upstream tags + lock HEAD:**
    Run `git fetch upstream --tags` to refresh upstream/main HEAD (may have shifted since Plan 47-01 lock). Capture post-fetch `git rev-parse upstream/main` sha. Verify the two backfill anchor tags resolve locally: v0.41.0 and v0.43.0 (resolve actual shas via `git rev-parse v0.41.0` + `git rev-parse v0.43.0`; record verbatim).

    Write `47-02-LOCK-NOTES.md` in the phase dir capturing verbatim:
    - `upstream_head_at_audit: <40-char post-fetch sha>` (D-47-A3 captured-at-first-commit-of-Plan-47-02; informational for historical range — schema uniformity)
    - `v0.41.0_sha: <resolved sha>` (record verbatim)
    - `v0.43.0_sha: <resolved sha>` (record verbatim)
    - `fetch_date: <UTC date>`
    - `plan_47_01_head_at_audit: <sha from 47-01-LOCK-NOTES.md for cross-ledger correlation>`

    **Step 2 — Verify drift-tool sha invariant:**
    `sha256sum scripts/check-upstream-drift.sh` (or platform-equivalent) MUST produce `0834aa664fbaf4c5e41af5debece292992211559` — if it does NOT, ABORT and surface via AskUserQuestion (D-47-E10).

    **Step 3 — Run drift-tool for backfill range:**
    Ensure `ci-logs-local/drift/` exists (mkdir -p). Run the locked invocation EXACTLY:
    `make check-upstream-drift ARGS="--from v0.41.0 --to v0.43.0 --format json" > ci-logs-local/drift/&lt;UTC-timestamp&gt;-v041-v043.json`

    If `make` is not on PATH (Windows host), fall back to:
    `bash scripts/check-upstream-drift.sh --from v0.41.0 --to v0.43.0 --format json > ci-logs-local/drift/&lt;UTC-timestamp&gt;-v041-v043.json`

    Capture from JSON: `total_unique_commits` (ledger row-count target per D-47-B4 step 2; expected ~19 per CONTEXT § Drift signal preview), category distribution, per-commit metadata. DO NOT commit JSON; `ci-logs-local/` is in `.gitignore` per D-47-E1.

    Commit `47-02-LOCK-NOTES.md` standalone with DCO sign-off.
  </action>
  <verify>
    <automated>test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-LOCK-NOTES.md &amp;&amp; grep -q "^upstream_head_at_audit: [a-f0-9]\{40\}$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-LOCK-NOTES.md &amp;&amp; test -f ci-logs-local/drift/*-v041-v043.json &amp;&amp; grep -q "total_unique_commits" ci-logs-local/drift/*-v041-v043.json</automated>
  </verify>
  <acceptance_criteria>
    - Plan 47-01 verified complete (`grep -q "^status: complete$" 47-01-SUMMARY.md` succeeds)
    - `git fetch upstream --tags` exits 0
    - `git rev-parse v0.41.0` resolves (sha captured)
    - `git rev-parse v0.43.0` resolves (sha captured)
    - `47-02-LOCK-NOTES.md` exists with `upstream_head_at_audit:` line carrying 40-char hex sha
    - `scripts/check-upstream-drift.sh` sha equals `0834aa664fbaf4c5e41af5debece292992211559` (D-47-A2 reproducibility pin)
    - JSON file exists under `ci-logs-local/drift/` matching glob `*-v041-v043.json`
    - JSON contains `total_unique_commits` field
    - `git check-ignore` confirms JSON is ignored
    - Drift-tool exit code is 0
    - Lock-notes commit signed with `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>Backfill drift-tool JSON captured; HEAD locked; sequential ordering precondition verified.</done>
</task>

<task type="auto">
  <name>Task 2: Write backfill ledger frontmatter + Headline + Reproduction + Cluster Summary scaffold</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-A2 frontmatter fields; D-47-C1 backfill-cleanup framing; D-47-E3 two-tier structure; D-47-B4 close-gate)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (UPST6 ledger from Plan 47-01 — schema reference)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-LOCK-NOTES.md (read upstream_head_at_audit from Task 1)
    - ci-logs-local/drift/*-v041-v043.json (read total_unique_commits + category preview for cluster-count placeholder)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md</files>
  <action>
    Create `DIVERGENCE-LEDGER-v041-v043-backfill.md` with frontmatter (verbatim values from CONTEXT D-47-A2 + D-47-C1 backfill-cleanup framing):

    ```
    ---
    phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion
    plan: 02
    ledger_type: drift-ingest-backfill
    range: v0.41.0..v0.43.0
    upstream_head_at_audit: &lt;sha from 47-02-LOCK-NOTES.md&gt;
    drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
    drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
    drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.41.0 --to v0.43.0 --format json"'
    framing: 'backfill-cleanup, not parity-sync (per REQ-DRIFT-INGEST-01)'
    deferral_origin: 'v2.3 scope-lock 2026-04-29'
    historical_absorption_phases: [22, 34]
    pre_d19_trailer_era: true
    date: &lt;ship date YYYY-MM-DD&gt;
    ---
    ```

    Then add these top-level sections (placeholders for auditor to fill in Tasks 3-5):

    1. `# Phase 47 v0.41–v0.43 Backfill Drift Ingestion Ledger`
    2. `## Headline` — one-paragraph backfill-cleanup framing. Emphasize: NOT parity-sync, retroactive paper-trail on the long-deferred range (v2.3 scope-lock 2026-04-29), reconstructs absorption record against Phase 22 + 34 historical claims via D-47-C2 subject-line + diff fingerprint match (pre-D-19 trailer convention era — only 11 unique `Upstream-commit:` trailers exist in fork main). Placeholder reads: "TBD at audit-walk close — auditor fills with cluster count, total commit count, disposition breakdown, absorbed-via distribution, and Phase 48 hand-off candidate count."
    3. `## Reproduction` — block recording the drift-tool invocation verbatim, the JSON output path (ci-logs-local/drift/*-v041-v043.json, ignored), an `auditor-rerun:` line, and a note that backfill range is fully historical so HEAD-anchor is informational (schema uniformity per D-47-A3).
    4. `## Cluster Summary` — markdown table header only, columns: `cluster_id | theme | commits | disposition | windows-touch | absorbed-via-summary | rationale`. Body rows placeholder: `<!-- auditor fills in Task 3 -->`.
    5. Stub cluster sections placeholder: `<!-- ### Cluster 1: ... (auditor fills in Task 3) -->`. Estimate 4-6 clusters per CONTEXT § Drift signal preview (19 commits).
    6. Stub `## Empirical cross-check` header (D-47-C4 + D-47-D1 placeholder; body filled in Task 5).
    7. Stub `## Phase 48 hand-off` header (D-47-C1 placeholder; body filled in Task 6).
    8. **NO `## ADR review` section** (D-47-C4 — backfill ledger SKIPS this per locked decision; negative-assertion: grep MUST return 0).

    Commit with DCO sign-off: `docs(47-02): scaffold v041-v043 backfill drift ingestion ledger frontmatter + section headers`.
  </action>
  <verify>
    <automated>test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; grep -q "^range: v0.41.0..v0.43.0$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; grep -q "^drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; grep -q "^framing: 'backfill-cleanup, not parity-sync" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; grep -q "^## Empirical cross-check$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; grep -q "^## Phase 48 hand-off$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; ! grep -q "^## ADR review$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md</automated>
  </verify>
  <acceptance_criteria>
    - `DIVERGENCE-LEDGER-v041-v043-backfill.md` exists at the canonical path
    - Frontmatter contains `range: v0.41.0..v0.43.0` (exact match)
    - Frontmatter contains `upstream_head_at_audit:` with 40-char hex sha (matches Task 1 lock)
    - Frontmatter contains `drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559` (exact match D-47-A2 reproducibility pin)
    - Frontmatter contains `framing: 'backfill-cleanup, not parity-sync (per REQ-DRIFT-INGEST-01)'` (D-47-C1 framing)
    - Frontmatter contains `deferral_origin: 'v2.3 scope-lock 2026-04-29'`
    - Frontmatter contains `historical_absorption_phases: [22, 34]`
    - Headers `## Headline`, `## Reproduction`, `## Cluster Summary`, `## Empirical cross-check`, `## Phase 48 hand-off` all present (grep-confirmable)
    - **NEGATIVE assertion: `## ADR review` header is ABSENT** (D-47-C4 — `! grep -q "^## ADR review$"` succeeds)
    - Cluster Summary table has header row with exact columns: `cluster_id | theme | commits | disposition | windows-touch | absorbed-via-summary | rationale`
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>Backfill ledger scaffold committed with all D-47-A2 + D-47-C1 frontmatter fields, all D-47-C4 mandatory section headers present, AND D-47-C4 NEGATIVE assertion (no ## ADR review) holds.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 3: Audit-walk — cluster grouping + per-cluster sections + dispositions + windows-touch + commit-row tables (HUMAN-IN-THE-LOOP)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-A5 row schema; D-47-C3 4-disposition vocab; D-47-E3 two-tier structure; D-47-E10 drift-tool bug surfacing on first real load)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (Plan 47-01 UPST6 ledger for cluster shape reference)
    - .planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md (Phase 42 cluster shape reference)
    - ci-logs-local/drift/*-v041-v043.json (read full commit inventory; expected ~19 commits)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md (current state from Task 2)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md</files>
  <action>
    **HUMAN AUDIT-WALK REQUIRED.** Auto-runner cannot make substantive cluster-grouping or disposition decisions.

    For each upstream commit in v0.41.0..v0.43.0 (from drift-tool JSON):
    1. **Cluster grouping** — group commits into themed clusters (auditor's judgment; CONTEXT § Drift signal preview estimates 4-6 clusters for 19 commits).
    2. **Per-cluster section** — write `### Cluster N: <theme>` with these blocks IN ORDER:
       - `**Commits:**` count + per-commit subject preview
       - `**Disposition:**` one of `will-sync` / `fork-preserve` / `won't-sync` / `split` (D-47-C3 standard 4-disposition vocab — SAME as UPST6)
       - `**Windows-touch:**` `yes` or `no` per D-47-A5 column heuristic
       - `**Rationale:**` one-paragraph justification for disposition (backfill framing: rationale focuses on already-absorbed status per Phase 22/34 vs missed-cherry-pick vs intentionally-skipped vs fork-divergence)
       - Commit-row table with D-47-A5 schema PLUS D-47-C3 `absorbed-via:` column: `| sha | subject | upstream-tag | categories | files-changed | windows-touch | absorbed-via |`
       - `absorbed-via:` column body cells left as placeholder `<TBD-Task-4>` (filled in Task 4)
    3. **Drift-tool bug surfacing (D-47-E10 ALERT):** Backfill ledger is the most likely source of drift-tool feedback per CONTEXT § Decisions D-47-E10 — first real load on a long-deferred range is exactly the scenario where tool gaps surface. If auditor discovers a category miscategorization or file-filter blind spot mid-walk, document inline AND spawn `.planning/quick/YYMMDD-xxx-upstream-drift-tool-fix/` quick-task. Plan 47-02 stays untouched to preserve `drift_tool_sh_sha` reproducibility.
    4. **Cluster Summary table population** — fill the table body created in Task 2. `absorbed-via-summary` column gets a placeholder distribution (e.g., `mostly-phase-34`) — refined in Task 4.
    5. **Headline paragraph population** — fill with cluster count, total commit count, disposition breakdown (TBD-Task-4-pending absorbed-via distribution), windows-touch:yes count.
    6. **Row-count gate verification** — sum total commit-rows across all cluster tables; MUST be >= drift-tool `total_unique_commits` (D-47-B4 step 2). If short, auditor surfaces a gap and re-walks the inventory.

    Commit with DCO sign-off: `docs(47-02): populate v041-v043 backfill cluster sections with dispositions + windows-touch + commit-row tables (absorbed-via TBD)`.

    **Resume signal:** Type "backfill audit-walk complete" or describe drift-tool bugs surfaced.
  </action>
  <verify>
    <automated>grep -c "^### Cluster " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }' &amp;&amp; grep -cE "^\*\*Disposition:\*\* (will-sync|fork-preserve|won't-sync|split)$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }' &amp;&amp; grep -cE "^\*\*Windows-touch:\*\* (yes|no)$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }' &amp;&amp; grep -q "| absorbed-via |" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "^### Cluster " backfill.md` returns N where N matches auditor-claimed cluster count from Cluster Summary table (~4-6 expected)
    - Every cluster section has `**Disposition:**` line with one of exactly four values: `will-sync` / `fork-preserve` / `won't-sync` / `split`
    - Every cluster section has `**Windows-touch:**` line with `yes` or `no`
    - Every cluster section has `**Rationale:**` paragraph (non-empty; framed as backfill-cleanup)
    - Every cluster has a commit-row table with header `| sha | subject | upstream-tag | categories | files-changed | windows-touch | absorbed-via |` (absorbed-via column present even if cells are `<TBD-Task-4>` placeholders)
    - Cluster Summary table body populated
    - Sum of commit-rows across all cluster tables >= drift-tool `total_unique_commits` for v0.41.0..v0.43.0 (~19 expected)
    - Any drift-tool bug surfaced has a corresponding `.planning/quick/*-upstream-drift-tool-fix/` quick-task spawned (D-47-E10)
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>All backfill cluster sections populated with disposition + windows-touch + rationale + commit-row table (absorbed-via column present as placeholder); row-count gate satisfied; Cluster Summary table body filled.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 4: Reconstruct absorbed-via: column via subject-line + diff fingerprint match against fork main (HUMAN MANUAL JUDGMENT)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-C2 subject+fingerprint match methodology; D-47-C3 6 standard absorbed-via values)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/ (Phase 22 plan SUMMARYs — historical absorption record; v0.38..v0.40 scope but may have spilled into v0.41 per CONTEXT § canonical_refs)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/ (Phase 34 plan SUMMARYs — explicit v0.41..v0.52 absorption record; primary source for absorbed-via reconstruction)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md (current state from Task 3; <TBD-Task-4> cells to fill)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md</files>
  <action>
    **HUMAN MANUAL JUDGMENT REQUIRED.** Pre-D-19 absorption (only 11 unique `Upstream-commit:` trailers exist in fork main) means subject + fingerprint match is the load-bearing detection methodology; auto-runner cannot make ambiguous-match calls.

    For each commit row in each cluster's commit-row table:
    1. **Subject-line match against fork main** — run `git log main --grep="<upstream-subject-substring>"` to find a fork-side commit with matching subject line. May surface multiple candidates if Phase 22/34 reworded the subject on absorption.
    2. **Diff fingerprint match** — for each candidate fork-side commit, compare `git show <upstream-sha> --stat` vs `git show <fork-main-sha> --stat`; if files-touched + lines-changed match within tolerance, treat as absorbed.
    3. **Cross-reference Phase 22/34 SUMMARYs** — read each `.planning/phases/22-*/SUMMARY*.md` and `.planning/phases/34-*/SUMMARY*.md` to enumerate the claimed-absorption per plan. May surface plan-claim drift vs actual absorbed commits.
    4. **Assign one of 6 standard values** to each commit row's `absorbed-via:` column per D-47-C3:
       - `phase-22-plan-XX-commit-XXXXXXXX` → already-absorbed via Phase 22 UPST2; subject+fingerprint match against fork main
       - `phase-34-plan-XX-commit-XXXXXXXX` → already-absorbed via Phase 34 UPST3; subject+fingerprint match against fork main
       - `unmatched` → no fork-side commit matches subject OR fingerprint; missed by Phase 22/34; **CANDIDATE for Phase 48 absorption** (flagged in Task 6 Phase 48 hand-off subsection)
       - `intentionally-skipped` → never absorbed by design (e.g., upstream-only macOS lint fix not affecting fork's CI); cluster rationale documents skip reason
       - `fork-divergence` → fork chose different implementation; `fork-preserve` cluster disposition with rationale
       - `ambiguous-see-cluster-rationale` → subject matched but diff fingerprint partially differs (e.g., Phase 22/34 absorbed with fork-side rework); per-commit rationale block in cluster body captures disambiguation
    5. **Update Cluster Summary table** — refine `absorbed-via-summary` column with actual distribution (e.g., `12 phase-34 / 4 phase-22 / 2 unmatched / 1 intentionally-skipped`).
    6. **Update Headline paragraph** — add absorbed-via distribution numbers to the headline summary.

    Commit with DCO sign-off: `docs(47-02): reconstruct absorbed-via column via subject + diff fingerprint match against Phase 22/34`.

    **Resume signal:** Type "absorbed-via reconstruction complete — N unmatched candidates surfaced for Phase 48 hand-off" or describe ambiguous-match blockers.
  </action>
  <verify>
    <automated>! grep -q "&lt;TBD-Task-4&gt;" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; grep -cE "absorbed-via: (phase-22-plan-|phase-34-plan-|unmatched|intentionally-skipped|fork-divergence|ambiguous-see-cluster-rationale)" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }'</automated>
  </verify>
  <acceptance_criteria>
    - NEGATIVE: no `<TBD-Task-4>` placeholder remains in the ledger (`! grep -q "<TBD-Task-4>"` succeeds)
    - Every commit row in every cluster's commit-row table has an `absorbed-via:` value drawn from the 6 standard set per D-47-C3
    - At least one commit row carries `phase-22-plan-XX-commit-XXXXXXXX` OR `phase-34-plan-XX-commit-XXXXXXXX` (historical absorption traceability — Phase 22/34 were the explicit absorption phases for this range)
    - Cluster Summary table `absorbed-via-summary` column refined with actual distribution (not placeholder)
    - Headline paragraph mentions absorbed-via distribution
    - Any `ambiguous-see-cluster-rationale` rows have per-commit disambiguation in the cluster's rationale block
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>All commit rows have absorbed-via assignments; Phase 22/34 historical record reconstructed; ambiguous-match rationale captured; unmatched candidates count is the input for Task 6 Phase 48 hand-off.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 5: Write ## Empirical cross-check section (≥4 fork-shared files; preferentially Phase 22/34-era files) (HUMAN FILE-WALK)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-C4 backfill ledger empirical cross-check YES; D-47-D1 ≥4 files; CONTEXT § Claude's Discretion preferential sampling for backfill — crates/nono/src/policy.rs, crates/nono/src/audit.rs, crates/nono-cli/src/profile/)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/ (Phase 22 absorption scope — which files were most-touched)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/ (Phase 34 absorption scope — which files were most-touched)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md (current state from Tasks 3-4)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md</files>
  <action>
    **HUMAN FILE-WALK REQUIRED.** Auto-runner cannot judge which files matter most for the backfill empirical cross-check.

    Write the `## Empirical cross-check` section body (header already exists as Task 2 placeholder):

    1. **Preamble paragraph:** Explain the empirical cross-check purpose for backfill — spot-check fork-shared files against upstream v0.41.0..v0.43.0 log to detect any upstream commits the drift tool's D-11 path filter may have missed, AND retroactively close the `feedback_cluster_isolation_invalid` lesson (Phase 34 may have hit re-export deps it didn't recognize during the pre-D-47-D1..D4 era). D-47-C4 explicitly retains empirical cross-check on backfill while skipping `## ADR review`.

    2. **File walk — ≥4 fork-shared files** (D-47-D1 raised threshold). Preferentially sample per CONTEXT § Claude's Discretion + D-47-E12 (adapted for backfill):
       - `crates/nono/src/policy.rs` (Phase 22/34-era hot zone — policy.json + group resolver evolution)
       - `crates/nono/src/audit.rs` (Phase 22/34-era hot zone — audit event surface evolution)
       - `crates/nono-cli/src/profile/` (Phase 22/34-era hot zone — profile resolution + group-policy mapping; choose one specific file from this directory)
       - One additional auditor's-choice file most-likely-touched-by-Phase-22/34-absorption (e.g., `crates/nono/src/capability.rs`, `crates/nono-cli/src/exec_strategy.rs`)

    3. **Per-file walk format**:
       ```
       ### File: <path>
       - Walked upstream log: `git log v0.41.0..v0.43.0 -- <path>`
       - Commits touching this file in range: <count>
       - Cluster mapping: <which cluster(s) in this ledger cover these commits>
       - absorbed-via status sample: <e.g., "all 3 commits → phase-34-plan-02-commit-XXXXXXXX">
       - Drift-tool coverage: <PASS / FAIL — see D-47-E10 follow-up spawn if FAIL>
       ```

    4. **Drift-tool gap surfacing** — if any file walk reveals an upstream commit the drift tool missed, document inline + spawn `.planning/quick/YYMMDD-xxx-upstream-drift-tool-fix/` quick-task per D-47-E10 (Plan 47-02 itself stays untouched).

    5. **NO `## Cross-cluster re-export deps detected` subsection on backfill ledger** — D-47-D1's re-export surface diff requirement is UPST6-only per CONTEXT § Phase Boundary (backfill commits are historical, not load-bearing for Phase 48 wave structure; re-export scan applies to `will-sync` clusters in cycles where cherry-pick is the forward action — backfill `will-sync` rows are retroactive paper-trail). If backfill ledger has any `absorbed-via: unmatched` rows that Phase 48 will cherry-pick forward, those will get re-export scan treatment at Phase 48 plan-phase time, not Phase 47.

    Commit with DCO sign-off: `docs(47-02): add ## Empirical cross-check section + ≥4 file walks on backfill range`.

    **Resume signal:** Type "backfill empirical cross-check complete — N files walked, M drift-tool gaps surfaced".
  </action>
  <verify>
    <automated>grep -c "^## Empirical cross-check$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md | awk '{ exit ($1 == 1) ? 0 : 1 }' &amp;&amp; grep -c "^### File: " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md | awk '{ exit ($1 &gt;= 4) ? 0 : 1 }'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "^## Empirical cross-check$" backfill.md` returns exactly 1
    - `grep -c "^### File: " backfill.md` returns >= 4 (D-47-D1 raised threshold retained on backfill per D-47-C4)
    - At least one walked file is from the CONTEXT § Claude's Discretion preferential sample set: `crates/nono/src/policy.rs` OR `crates/nono/src/audit.rs` OR `crates/nono-cli/src/profile/*`
    - Each `### File:` block contains: walked-log invocation, commits-count, cluster mapping, absorbed-via status sample, coverage verdict (PASS/FAIL)
    - If any FAIL verdict present, a `.planning/quick/*-upstream-drift-tool-fix/` quick-task is spawned (D-47-E10)
    - NEGATIVE assertion still holds: `! grep -q "^## ADR review$"` (D-47-C4)
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>## Empirical cross-check populated with ≥4 file walks; drift-tool gaps surfaced if any (high probability on first real load per D-47-E10); D-47-C4 NEGATIVE assertion (no ADR review) preserved.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 6: Flag absorbed-via: unmatched rows for Phase 48 consideration (HUMAN PHASE-48 HAND-OFF)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-C1 backfill purpose includes surface-missed-cherry-picks for Phase 48)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md (current state from Tasks 3-5; read all absorbed-via: unmatched rows)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (Plan 47-01 UPST6 ledger — Phase 48 hand-off semantic reference)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md</files>
  <action>
    **HUMAN JUDGMENT REQUIRED.** Auto-runner cannot judge cherry-pick worthiness or generate per-candidate rationale.

    Write the `## Phase 48 hand-off` section body (header already exists as Task 2 placeholder):

    1. **Preamble paragraph:** Explain the hand-off purpose — backfill ledger surfaces any commits Phase 22/34 missed (since both pre-dated D-19 trailer convention). Per D-47-C1 + ROADMAP SC#4, missed-cherry-pick candidates are flagged for Phase 48 absorption alongside UPST6 work. Most-likely outcome per CONTEXT § Drift signal preview: "resolves the deferral by confirming no fork-side action needed" (0..few candidates).

    2. **Candidate table** — list every commit row whose `absorbed-via:` value is `unmatched` in this format:
       ```
       | upstream-sha | subject | upstream-tag | cluster | windows-touch | recommendation-for-Phase-48 |
       |--------------|---------|--------------|---------|---------------|------------------------------|
       | XXXXXXXX     | ...     | v0.4X.0      | N       | yes/no        | cherry-pick / D-20-replay / defer-to-UPST7 / drop-not-relevant |
       ```

    3. **Per-candidate rationale** — for each row in the candidate table, write a `### Candidate: <upstream-sha>` block with:
       - **Why missed by Phase 22/34:** brief explanation (subject doesn't match any fork-side commit, file paths fall outside Phase 22/34 scope, etc.)
       - **Windows-touch implications:** if `yes`, default to `fork-preserve` review per D-42-C3 inheritance
       - **Recommendation for Phase 48:** cherry-pick (with rationale) / D-20 manual replay (with rationale) / defer to UPST7 (with rationale) / drop as not-relevant (with rationale)

    4. **Zero-unmatched case:** If Task 4 found ZERO `absorbed-via: unmatched` rows, the Phase 48 hand-off section explicitly states:
       ```
       ## Phase 48 hand-off

       Zero `absorbed-via: unmatched` rows detected across the v0.41.0..v0.43.0 backfill ledger. Phase 22 + 34 absorbed the full range; the REQ-DRIFT-INGEST-01 deferral resolves with "no fork-side action needed" per ROADMAP SC#4 most-likely-outcome.

       Phase 48 has NO backfill candidates to absorb alongside UPST6 work.
       ```

    Commit with DCO sign-off: `docs(47-02): flag N absorbed-via:unmatched rows for Phase 48 hand-off (or document zero-unmatched closure)`.

    **Resume signal:** Type "Phase 48 hand-off complete — N candidates flagged" or "zero-unmatched closure documented".
  </action>
  <verify>
    <automated>grep -c "^## Phase 48 hand-off$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md | awk '{ exit ($1 == 1) ? 0 : 1 }'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "^## Phase 48 hand-off$" backfill.md` returns exactly 1
    - Phase 48 hand-off section has a preamble paragraph explaining purpose (referencing D-47-C1)
    - If any `absorbed-via: unmatched` rows exist in cluster tables, each is enumerated in the hand-off candidate table with sha + subject + cluster + windows-touch + recommendation
    - Each enumerated candidate has a `### Candidate:` per-candidate rationale block with why-missed + windows-touch implications + Phase 48 recommendation
    - If zero unmatched rows, hand-off section explicitly documents the zero-unmatched closure
    - Hand-off candidate count is internally consistent: number of `### Candidate:` blocks == number of `absorbed-via: unmatched` rows in cluster tables (or zero-closure documented)
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>Phase 48 hand-off subsection populated; unmatched candidates flagged with per-candidate recommendations OR zero-unmatched closure documented; Phase 48 plan-phase has immutable input for backfill candidate consideration.</done>
</task>

<task type="auto">
  <name>Task 7: Close-gate verification + ROADMAP Phase 47 flip + STATE.md final update + Plan 47-02 SUMMARY + Phase 47 close (mechanical close)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-B4 8-step close-gate applied to backfill ledger; D-47-B4 strict-both-close phase-level gate)
    - .planning/STATE.md (current frontmatter + Accumulated Context format)
    - .planning/ROADMAP.md (Phase 47 entry to flip to [x])
    - .planning/phases/42-upst5-audit/42-01-SUMMARY.md (SUMMARY shape precedent)
    - $HOME/.claude/get-shit-done/templates/summary.md (SUMMARY template)
  </read_first>
  <files>
    .planning/ROADMAP.md, .planning/STATE.md, .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md
  </files>
  <action>
    **Step 1 — Run all 8 close-gate checks** (D-47-B4 applied to backfill ledger):
    1. `make check-upstream-drift ARGS="--from v0.41.0 --to v0.43.0 --format json"` exits 0 (drift-tool idempotence re-run on backfill range; output to new ci-logs-local/ file; NOT committed)
    2. Backfill ledger row count >= drift-tool `total_unique_commits` for v0.41.0..v0.43.0 (compare sum of commit-rows in cluster tables vs JSON `total_unique_commits` field; expected ~19)
    3. Every cluster has disposition + rationale (grep)
    4. **D-47-C4 NEGATIVE assertion:** `## ADR review` section ABSENT (`! grep -q "^## ADR review$" backfill.md` succeeds)
    5. `## Empirical cross-check` ≥4 files (grep)
    6. ROADMAP Phase 47 entry flipped to [x] + Plans counter 2/2 + UPST7 stub still present from Plan 47-01 (Step 3 below)
    7. STATE.md updated (Step 4 below)
    8. `git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/` returns zero lines (Plan 47-02 ships zero source edits — D-47-E5 invariant)

    **Step 2 — Verify Phase 47 phase-level close gate (D-47-B4 strict-both-close):**
    - Plan 47-01 SUMMARY exists with `status: complete` (verified in Plan 47-02 Task 1)
    - Plan 47-02 will close with `status: complete` at this Task 7 close
    - BOTH ledgers disposition-complete: REQ-UPST6-01 AND REQ-DRIFT-INGEST-01 BOTH satisfied at same close-event per user's explicit rejection of load-bearing-UPST6-only

    **Step 3 — Update ROADMAP.md:**
    - Flip Phase 47 v2.6 milestone-block entry from `[ ]` to `[x]` with `(completed YYYY-MM-DD)` appended
    - Flip Phase 47 Plans counter from `1 / 2` to `2 / 2 plans complete`
    - Mark Plan 47-02 as `[x] 47-02-V041-V043-BACKFILL-PLAN.md` in the Phase 47 plan list
    - Update the v2.6 milestone progress section if a progress table exists (e.g., `Phase 47. UPST6 audit + v0.41–v0.43 drift ingestion | 2/2 | Complete | YYYY-MM-DD`)
    - Verify UPST7 stub from Plan 47-01 Task 8 still present (no inadvertent removal)

    **Step 4 — Update STATE.md:**
    - Bump frontmatter `completed_plans` counter (now Plan 47-01 + Plan 47-02 = 14 total)
    - Bump frontmatter `total_plans` if not already accounting for Plan 47-02
    - Bump frontmatter `progress.percent`
    - Update `last_activity` to today's date
    - Update `## Current Position` — flip Phase from `49` (per current STATE) to `47` (upst6-audit-v0-41-v0-43-drift-ingestion); Status: Complete — Phase 47 closed YYYY-MM-DD; both REQs satisfied at same close-event
    - Append Plan 47-02 close entry + Phase 47 phase-level close entry under `## Accumulated Context > Key Decisions (v2.6)` capturing:
      - Plan 47-02 specifics: backfill range v0.41.0..v0.43.0, upstream_head_at_audit, cluster count, commit count, disposition breakdown, windows-touch:yes count, absorbed-via distribution (phase-22/phase-34/unmatched/intentionally-skipped/fork-divergence/ambiguous counts), empirical cross-check files sampled, Phase 48 hand-off candidate count
      - Phase 47 phase-level: REQ-UPST6-01 + REQ-DRIFT-INGEST-01 both satisfied at same close-event per D-47-B4 strict-both-close gate; v2.3 scope-lock 2026-04-29 REQ-DRIFT-INGEST-01 deferral closed
      - DCO sign-off attestation: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

    **Step 5 — Create Plan 47-02 SUMMARY.md** at `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md` mirroring Plan 47-01 SUMMARY shape adapted for backfill:
    - Frontmatter: `plan: 02`, `phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion`, `status: complete`, `requirements: [REQ-DRIFT-INGEST-01]`, `date: <ship date>`, `must_haves_verified: <count from must_haves.truths>`
    - Sections: `## Summary`, `## Artifacts Created`, `## Close-Gate Verification` (8 checks from Step 1 with pass/fail per check; explicitly include D-47-C4 NEGATIVE assertion), `## Disposition Breakdown` (table), `## Absorbed-via Distribution` (table — phase-22/phase-34/unmatched/intentionally-skipped/fork-divergence/ambiguous counts), `## Empirical Cross-Check Files`, `## Phase 48 Hand-off Candidates` (table or zero-unmatched closure), `## Phase 47 Phase-Level Close` (asserts D-47-B4 strict-both-close gate satisfied), `## Next Steps` (Phase 48 plan-phase consumes both ledgers)

    Commit with DCO sign-off: `docs(47-02): close-gate verification + ROADMAP Phase 47 flip + STATE.md update + Plan 47-02 SUMMARY + Phase 47 phase-level close`.
  </action>
  <verify>
    <automated>test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md &amp;&amp; grep -q "^status: complete$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md &amp;&amp; grep -q "Plan 47-02" .planning/STATE.md &amp;&amp; grep -q "\[x\] \*\*Phase 47:" .planning/ROADMAP.md &amp;&amp; ! grep -q "^## ADR review$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md &amp;&amp; bash -c 'cd /c/Users/OMack/Nono &amp;&amp; git diff --name-only HEAD~5..HEAD -- crates/ bindings/ scripts/ | wc -l | awk "{ exit (\$1 == 0) ? 0 : 1 }"'</automated>
  </verify>
  <acceptance_criteria>
    - All 8 D-47-B4 close-gate checks pass (drift-tool re-run idempotent on backfill range; row-count gate satisfied; absorbed-via column complete; D-47-C4 NEGATIVE assertion holds; Empirical cross-check ≥4 files; ROADMAP flipped; STATE updated; zero source edits)
    - `47-02-SUMMARY.md` exists with `status: complete` in frontmatter
    - SUMMARY contains sections: `## Summary`, `## Artifacts Created`, `## Close-Gate Verification`, `## Disposition Breakdown`, `## Absorbed-via Distribution`, `## Empirical Cross-Check Files`, `## Phase 48 Hand-off Candidates`, `## Phase 47 Phase-Level Close`, `## Next Steps`
    - ROADMAP.md Phase 47 entry shows `[x] **Phase 47:` with `(completed YYYY-MM-DD)` appended
    - ROADMAP.md Phase 47 Plans counter shows `2 / 2 plans complete`
    - ROADMAP.md Plan 47-02 entry marked `[x] 47-02-V041-V043-BACKFILL-PLAN.md`
    - STATE.md frontmatter `completed_plans` counter bumped (Plan 47-01 + Plan 47-02 both reflected)
    - STATE.md `## Current Position` flipped to Phase 47 Complete
    - STATE.md `## Accumulated Context` gains Plan 47-02 close entry + Phase 47 phase-level close entry referencing D-47-B4 strict-both-close gate satisfaction
    - `! grep -q "^## ADR review$" backfill.md` succeeds (D-47-C4 NEGATIVE assertion preserved through close)
    - `git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/ | wc -l` returns 0 across all Plan 47-02 commits (D-47-E5 invariant)
    - All Plan 47-02 commits signed with `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
  </acceptance_criteria>
  <done>Plan 47-02 close-gate fully verified; ROADMAP Phase 47 flipped to [x]; STATE.md Current Position flipped to Phase 47 Complete; SUMMARY.md committed; Phase 47 phase-level close satisfied per D-47-B4 strict-both-close gate; REQ-UPST6-01 + REQ-DRIFT-INGEST-01 both closed; D-47-E5 zero-source-edits invariant trivially honored; Phase 48 plan-phase unblocked.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Auditor → backfill ledger artifact | Audit-walk decisions captured in markdown; reader trusts auditor's cluster grouping + disposition + absorbed-via reconstruction |
| Drift-tool → backfill ledger frontmatter | `drift_tool_sh_sha` invariant lets future auditors reproduce input set against same range |
| Phase 22/34 historical record → absorbed-via column | Subject-line + diff fingerprint match (D-47-C2) reconstructs Phase 22/34 absorption claims; pre-D-19 trailer convention era constrains methodology |
| Backfill ledger → Phase 48 planner | `absorbed-via: unmatched` rows flagged for Phase 48 absorption alongside UPST6 work per D-47-C1 |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-47-02-01 | I (Integrity) | DIVERGENCE-LEDGER-v041-v043-backfill.md frontmatter `drift_tool_sh_sha` | mitigate | Task 1 asserts `sha256sum scripts/check-upstream-drift.sh` == `0834aa664fbaf4c5e41af5debece292992211559`; ABORT + AskUserQuestion if mismatch (D-47-E10) |
| T-47-02-02 | I (Integrity) | absorbed-via column reconstruction accuracy | mitigate | Task 4 manual subject-line + diff fingerprint match per D-47-C2; ambiguous matches use `ambiguous-see-cluster-rationale` value with per-commit disambiguation in cluster body; auditor judgment captured in commit |
| T-47-02-03 | I (Integrity) | D-47-C4 NEGATIVE assertion (no ## ADR review) | mitigate | Task 2 + Task 7 close-gate explicitly grep `! grep -q "^## ADR review$"` succeeds; preserved through close |
| T-47-02-04 | R (Repudiation) | Per-commit absorption attribution | mitigate | Every commit row carries verbatim upstream sha + Phase 22/34 absorption claim (or unmatched); DCO sign-off on every Plan 47-02 commit (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`) |
| T-47-02-05 | D (Denial of Service) | Drift-tool unavailability on backfill range | accept | If `make` not on PATH, fallback to `bash scripts/check-upstream-drift.sh`; if backfill range no longer resolvable due to upstream tag retraction (unlikely), surface via AskUserQuestion |
| T-47-02-06 | C (Confidentiality) | Raw drift-tool JSON output | mitigate | D-47-E1 inherited — JSON redirected to `ci-logs-local/drift/*-v041-v043.json` per `.gitignore`; never committed |
| T-47-02-07 | E (Elevation of Privilege) | N/A — audit-only phase | accept | Plan 47-02 ships ZERO source edits (D-47-E5); no runtime change; structurally cannot elevate privilege |
| T-47-02-08 | T (Tampering) | Phase 22/34 historical record drift | mitigate | Task 4 reads Phase 22 + Phase 34 SUMMARYs directly from git history; SUMMARYs are committed artifacts in `.planning/phases/22-*/` and `.planning/phases/34-*/`; tampering would require force-push to main which is gated by branch protection |

## Structural Mitigations (audit-phase invariants)

- **Confidentiality:** No new credentials, tokens, or secrets handled. Drift-tool JSON redirected to `ci-logs-local/drift/` per `.gitignore`; not committed.
- **Integrity:** Backfill ledger is the audit-of-record. Reproducibility against tag pair (v0.41.0..v0.43.0) + `drift_tool_sh_sha 0834aa66` invariant means another auditor re-running the same invocation produces a verifiable ledger. Frontmatter captures `framing: 'backfill-cleanup, not parity-sync'` + `deferral_origin: 'v2.3 scope-lock 2026-04-29'` for historical context.
- **Availability:** Zero runtime change. Plan 47-02 cannot break any user-facing surface.
- **Audit:** All ledger commits use DCO sign-off (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` — fork co-author convention). Phase 22/34 historical absorption claims preserved in cited SUMMARYs.
- **Supply chain:** Drift-tool sha pin in frontmatter prevents silent tool drift; Task 1 ABORTS on mismatch.
- **Phase-level close discipline:** D-47-B4 strict-both-close gate prevents either REQ from slipping; user explicitly rejected load-bearing-UPST6-only and stage-gate alternatives because REQ-DRIFT-INGEST-01 was already deferred at v2.3 scope-lock (second slip unacceptable).
</threat_model>

<verification>
1. Run `bash -c 'grep -q "^range: v0.41.0..v0.43.0$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md'` — exits 0
2. Run `bash -c 'grep -q "^framing: .backfill-cleanup, not parity-sync" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md'` — exits 0
3. Run `bash -c 'grep -c "^### Cluster " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md'` — returns N >= 1 (~4-6 expected)
4. Run `bash -c 'grep -c "absorbed-via: (phase-22-plan-|phase-34-plan-|unmatched|intentionally-skipped|fork-divergence|ambiguous-see-cluster-rationale)" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md'` — returns >= 1
5. Run `bash -c '! grep -q "^## ADR review$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md'` — succeeds (D-47-C4 NEGATIVE assertion)
6. Run `bash -c 'grep -c "^### File: " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md'` — returns >= 4
7. Run `bash -c 'grep -q "^## Phase 48 hand-off$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER-v041-v043-backfill.md'` — exits 0
8. Run `bash -c 'grep -q "\[x\] \*\*Phase 47:" .planning/ROADMAP.md'` — exits 0 (Phase 47 flipped at Plan 47-02 close)
9. Run `bash -c 'cd /c/Users/OMack/Nono && git diff --name-only HEAD~7..HEAD -- crates/ bindings/ scripts/ | wc -l'` — returns 0 (D-47-E5 invariant; Plan 47-02 ships zero source edits across all 7 tasks)
10. Run `make check-upstream-drift ARGS="--from v0.41.0 --to v0.43.0 --format json" > /dev/null` — exits 0 (drift-tool re-run idempotent on backfill range)
11. Run `bash -c 'test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md && grep -q "^status: complete$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md'` — exits 0
12. Run `bash -c 'grep -q "Phase 47 Complete" .planning/STATE.md'` — exits 0 (Current Position flipped)
</verification>

<success_criteria>
- REQ-DRIFT-INGEST-01 satisfied: DIVERGENCE-LEDGER-v041-v043-backfill.md exists at canonical path with all D-47-B4 close-gate conditions met (modulo D-47-C4 ADR-review skip)
- Every commit row has an `absorbed-via:` column value drawn from the 6 standard values per D-47-C3
- `## Empirical cross-check` section present with ≥4 fork-shared files (preferentially Phase 22/34-era hot zones per CONTEXT § Claude's Discretion)
- D-47-C4 NEGATIVE assertion holds: NO `## ADR review` section in backfill ledger (preserved through close-gate)
- `## Phase 48 hand-off` subsection lists 0..N unmatched-row candidates with per-candidate recommendations (or zero-unmatched closure documented)
- ROADMAP Phase 47 v2.6 entry flipped to [x] with completion date; Plans counter shows 2/2 complete
- STATE.md Current Position flipped to Phase 47 Complete; Plan 47-02 close entry + Phase 47 phase-level close entry recorded under Key Decisions (v2.6)
- `git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/ | wc -l` == 0 (D-47-E5 zero-source-edits invariant trivially honored)
- All Plan 47-02 commits signed with `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
- Plan 47-02 SUMMARY.md created with `status: complete`
- **Phase 47 phase-level close satisfied per D-47-B4 strict-both-close gate:** REQ-UPST6-01 (closed at Plan 47-01 close) AND REQ-DRIFT-INGEST-01 (closed at Plan 47-02 close) BOTH satisfied at the same phase-close event
- v2.3 scope-lock 2026-04-29 REQ-DRIFT-INGEST-01 deferral CLOSED; Phase 48 plan-phase unblocked with both ledgers as immutable input
</success_criteria>

<output>
After completion, create `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-02-SUMMARY.md` per Task 7 Step 5.
</output>
