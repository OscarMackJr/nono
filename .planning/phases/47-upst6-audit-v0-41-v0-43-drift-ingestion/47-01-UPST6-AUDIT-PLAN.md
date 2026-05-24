---
phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md
  - .planning/ROADMAP.md
  - .planning/STATE.md
  - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md
autonomous: false
requirements: [REQ-UPST6-01]
tags: [upstream-parity, drift-audit, ledger, windows-touch, upst6, adr-review, cross-cluster-re-export]

must_haves:
  truths:
    - "DIVERGENCE-LEDGER.md exists at .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (D-47-E2 phase-local convention)"
    - "Ledger frontmatter records D-47-A2 reproducibility fields verbatim: range=v0.54.0..v0.57.0, upstream_head_at_audit captured at first commit of Plan 47-01 per D-47-A3, drift_tool_sh_sha=0834aa664fbaf4c5e41af5debece292992211559, drift_tool_ps1_sha=0834aa664fbaf4c5e41af5debece292992211559, drift_tool_invocation locked (verbatim 'make check-upstream-drift ARGS=\"--from v0.54.0 --to v0.57.0 --format json\"'), fork_baseline=v0.54.0 (Phase 43 + 45 UPST5 sync point — Cluster 5 0748cced/5d821c12 + Cluster 2 8b888a1c source migration absorbed 2026-05-18..2026-05-20), date"
    - "Every cluster header carries one of four dispositions: will-sync / fork-preserve / won't-sync / split (D-47-C3 standard 4-disposition vocab; split codified at v2.5 close per feedback_cluster_isolation_invalid)"
    - "Every cluster's commit-row table follows D-47-A5 schema: sha + subject + upstream-tag + categories + files-changed-count + windows-touch (NO absorbed-via: column on UPST6 ledger — that's backfill-only per D-47-C3)"
    - "Every will-sync cluster has a '**Cross-cluster re-export check:**' subsection per D-47-D1/D2/D3 — scans pub use / pub mod / extern crate / pub(crate) declarations on the lead commit; surfaces any cross-cluster re-export deps inline with prereq cluster enumeration"
    - "Any cluster with a detected cross-cluster re-export dep has disposition flipped to 'split' with '**Prerequisite enumeration:**' line listing the prereq cluster's lead commit + re-exported symbol(s) per D-47-D4 default"
    - "Explicit '## ADR review' section present (D-47-E8 MANDATORY); falsifiable via grep -c \"^## ADR review$\" returning 1"
    - "ADR review section verdicts Phase 33 ADR Option A 'continue' with per-cell L/M/H verdicts for 5 dimensions (security / windows / maintenance / divergence / contributor); falsifiable via grep -cE \"^\\| (security|windows|maintenance|divergence|contributor)\" returning at least 5 (D-47-E8 per Phase 42 D-42-C4 inheritance)"
    - "ADR review outcome is one of (a) confirm Option A 'continue', (b) amend with carve-outs, or (c) flag a future-supersede trigger — Phase 47 does NOT supersede Phase 33 ADR (still Accepted)"
    - "Explicit '## Empirical cross-check' section present with ≥4 fork-shared files spot-checked against upstream v0.54.0..v0.57.0 log (D-47-D1 raises Phase 42 ≥3 → ≥4 per Phase 47 SC#3); preferentially samples crates/nono-cli/src/platform.rs, crates/nono/src/trust/, AIPC schema files, and Phase 45 Plan 45-01 source-migration target files per D-47-E12; falsifiable via grep -c \"^## Empirical cross-check\" returning 1"
    - "Empirical cross-check section consolidates per-cluster re-export findings with a '## Cross-cluster re-export deps detected' summary subsection listing all detected edges (source cluster → prereq cluster + symbol) per D-47-D3; falsifiable via grep -c \"^## Cross-cluster re-export deps detected\" returning 1 (or 0 if zero deps detected; document the zero-result inline)"
    - "Total row count across all cluster commit-row tables >= drift-tool total_unique_commits for v0.54.0..v0.57.0 (REQ-UPST6-01 acceptance + D-47-B4 close-gate step 2; exact coverage zero gap)"
    - "ROADMAP UPST7 stub committed per D-47-E11 — auditor picks location: default v2.6 § Future Cycles holding section (recommendation); shape inherits D-42-B4 (title 'UPST7 — Upstream v0.57.0… sync audit' or '… sync execution' per auditor's call, Depends on: Phase 48, Plans: 0 / TBD, ADR cross-reference to docs/architecture/upstream-parity-strategy.md § Future audit cadence)"
    - "ROADMAP.md Phase 47 v2.6 milestone-block entry remains in-progress (will flip to [x] after Plan 47-02 closes per D-47-B4 strict-both-close); Phase Details Plans counter for Phase 47 reflects '1 / 2 plans complete' after Plan 47-01"
    - "STATE.md frontmatter completed_plans counter bumped; STATE.md Last activity stamped with Plan 47-01 close date"
    - "STATE.md Accumulated Context gains a Plan 47-01 close entry under Key Decisions (v2.6) capturing range, lock-sha, cluster count, commit count, disposition breakdown (will-sync/fork-preserve/won't-sync/split counts), windows-touch:yes count, ADR-review verdict outcome, empirical cross-check files sampled, cross-cluster re-export deps detected count, UPST7 stub location decision + commit sha, DCO sign-off"
    - "Drift-tool re-run is idempotent: make check-upstream-drift ARGS=\"--from v0.54.0 --to v0.57.0 --format json\" exits 0 after plan close (D-47-B4 close-gate step 1)"
    - "D-47-E5 / D-47-B4-step-8 Windows-only-files invariant trivially honored: git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/ returns zero files (Plan 47-01 ships zero .rs / .toml / .sh / .ps1 / Makefile edits)"
    - "Strictly silent on post-v0.57.0 commits per D-47-A4: no task references or processes any commit past 10cec984; the 19 known post-v0.57.0 commits (between 10cec984 and upstream/main HEAD 807fca38 at context-capture) are deferred to UPST7"
    - "Raw drift-tool JSON output redirects to ci-logs-local/drift/<timestamp>-v054-v057.json per D-47-E1 / D-33-A2 inherited; NOT committed (ci-logs-local/ is in .gitignore)"
  artifacts:
    - path: ".planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md"
      provides: "Audited inventory of v0.54.0..v0.57.0 fork-vs-upstream divergence with per-cluster dispositions, windows-touch column, explicit ## ADR review section with per-cell L/M/H verdicts on 5 dimensions, ## Empirical cross-check section ≥4 fork-shared files, and ## Cross-cluster re-export deps detected summary subsection consolidating per-cluster re-export findings"
      contains: "## ADR review, ## Empirical cross-check, ## Cross-cluster re-export deps detected, ### Cluster, **Disposition:**, **Cross-cluster re-export check:**, | sha | subject | upstream-tag | categories | files-changed | windows-touch |, security, windows, maintenance, divergence, contributor, drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559, range: v0.54.0..v0.57.0"
    - path: ".planning/ROADMAP.md"
      provides: "Phase 47 Plans counter for plan 47-01 reflects 1 / 2; UPST7 stub committed per D-47-E11 location decision"
      contains: "[x] 47-01-UPST6-AUDIT-PLAN.md, UPST7 — Upstream v0.57.0, Depends on: Phase 48, Plans: 0 / TBD"
    - path: ".planning/STATE.md"
      provides: "Plan 47-01 close entry under Key Decisions (v2.6); completed_plans counter bumped; Last activity stamped"
      contains: "Phase 47 Plan 47-01"
    - path: ".planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md"
      provides: "Plan 47-01 close summary mirroring Phase 42 Plan-01-SUMMARY shape"
      contains: "REQ-UPST6-01, DIVERGENCE-LEDGER, windows-touch, ADR review, Empirical cross-check, Cross-cluster re-export"
  key_links:
    - from: "DIVERGENCE-LEDGER.md frontmatter"
      to: "drift-tool reproducibility (D-47-A2 / D-47-A3)"
      via: "frontmatter records range + upstream_head_at_audit + drift_tool shas + invocation verbatim; re-running the invocation against the same upstream HEAD reproduces the input set"
      pattern: "drift_tool_(sh|ps1)_sha|upstream_head_at_audit|drift_tool_invocation"
    - from: "DIVERGENCE-LEDGER.md ## ADR review section"
      to: "docs/architecture/upstream-parity-strategy.md (Phase 33 ADR, Accepted 2026-05-11, re-confirmed at v2.4 + v2.5 close)"
      via: "ADR review verdicts Option A 'continue' with per-cell L/M/H for 5 dimensions; outcome (a) confirm | (b) amend | (c) flag-future-supersede; does NOT supersede"
      pattern: "## ADR review|Phase 33 ADR|Option A|continue|security|windows|maintenance|divergence|contributor"
    - from: "DIVERGENCE-LEDGER.md per-cluster dispositions"
      to: "Phase 48 UPST6 sync execution input (immutable per D-47-B5)"
      via: "Phase 48 plan-phase consumes the ledger's cluster summary table + per-cluster dispositions + windows-touch column to choose cherry-pick vs D-20 manual-replay; D-47-D4 split flip is the structural prevention against another Cluster-2-style abort"
      pattern: "Disposition:.*will-sync|fork-preserve|won't-sync|split|Target phase:.*UPST6-sync|Phase 48"
    - from: "DIVERGENCE-LEDGER.md ## Empirical cross-check section"
      to: "feedback_cluster_isolation_invalid memory closure (D-47-D1/D2/D3/D4)"
      via: "≥4 fork-shared file walk PLUS explicit re-export surface diff (pub use / pub mod / extern crate / pub(crate)) on every will-sync cluster's lead commit; closes the Phase 43 Cluster 2 8b888a1c empirical-prerequisite-discovery class structurally instead of reactively"
      pattern: "## Empirical cross-check|## Cross-cluster re-export deps detected|pub use|pub mod|extern crate"
    - from: "ROADMAP § UPST7 stub (location TBD at plan-walk per D-47-E11)"
      to: "Phase 33 ADR § Future audit cadence rule (D-47-E6)"
      via: "stub Reference line cites docs/architecture/upstream-parity-strategy.md § Future audit cadence; preserves cadence-rule signal under v2.6 § Future Cycles; UPST7 fires when next upstream release ships or maintainer decides accumulated cherry-pick labor warrants firing"
      pattern: "UPST7 — Upstream v0.57.0|## Future Cycles|Future audit cadence"
---

<objective>
Run the D-47-A1-locked drift-tool invocation against the v0.54.0..v0.57.0 range, lock `upstream_head_at_audit` at first commit of this plan (D-47-A3), curate `DIVERGENCE-LEDGER.md` mirroring Phase 42 two-tier shape with: (a) the D-47-A5 commit-row schema including the `windows-touch` column, (b) the D-47-D1..D4 cross-cluster re-export hardening on every `will-sync` cluster's lead commit with default flip-to-`split` on detected deps, (c) an explicit `## ADR review` section with per-cell L/M/H verdicts on the 5 Phase 33 ADR dimensions (D-47-E8 MANDATORY), (d) an explicit `## Empirical cross-check` section spot-checking ≥4 fork-shared files (D-47-D1 raises Phase 42 ≥3 → ≥4) preferentially sampling Phase 43 + 45 absorption surfaces per D-47-E12, (e) a `## Cross-cluster re-export deps detected` summary subsection consolidating per-cluster re-export findings (D-47-D3), queue a UPST7 placeholder per D-47-E11, update STATE.md, and ship the SUMMARY.

Purpose: REQ-UPST6-01 demands a falsifiable, disposition-complete divergence inventory before Phase 48 UPST6 sync execution can begin. Plan 47-01's ledger is the binding input for Phase 48 (analog to how Phase 42's ledger was Phase 43's input). ~75-commit evidence base (substantially larger than Phase 42's 18) makes the ADR review's per-cell L/M/H verdict particularly load-bearing; with ~75 commits spanning 3 minor releases (v0.55.0, v0.56.0, v0.57.0), the cross-cluster re-export risk (Phase 43 Cluster 2 class) is substantially higher — D-47-D1..D4 close this lesson structurally rather than reactively.

Output: 4 files committed across atomic commits per the Phase 42 precedent:
1. `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` (NEW, ~250-350 lines per CONTEXT § Existing Code Insights size estimate for ~75 commits / 7-10 clusters)
2. `.planning/ROADMAP.md` (modified — Phase 47 Plans counter reflects 1/2; UPST7 stub appended at D-47-E11-chosen location)
3. `.planning/STATE.md` (modified — frontmatter bump + Plan 47-01 close entry under Key Decisions (v2.6); Last activity stamped)
4. `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md` (NEW)
5. ZERO `.rs` / `.toml` / `.sh` / `.ps1` / `Makefile` edits (D-47-E5 / D-47-B4 step 8 trivially honored).

Auto-mode flag for executor: `autonomous: false`. Audit-walk decisions (cluster grouping, disposition choice, ADR L/M/H verdicts, re-export surface interpretation, `split` flip judgments) require human-in-the-loop judgment. Mechanical scaffolding tasks (frontmatter, drift-tool invocation, grep verifications, ROADMAP stub append, STATE update, SUMMARY commit) are auto-runnable.
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
@.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md
@.planning/phases/42-upst5-audit/42-01-PLAN.md
@.planning/phases/42-upst5-audit/42-01-SUMMARY.md
@docs/architecture/upstream-parity-strategy.md
@.planning/templates/upstream-sync-quick.md

<!-- Phase 43 split-disposition precedent (the empirical discovery that proved Phase 42 cluster isolation invalid; D-47-D1..D4 close this lesson structurally) -->
@.planning/phases/43-upst5-sync-execution/43-01-EDITION-2024-FOUNDATION-SUMMARY.md
@.planning/phases/43-upst5-sync-execution/43-01b-EDITION-WORKSPACE-ONLY-SUMMARY.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fetch upstream tags + lock upstream_head_at_audit (mechanical preamble)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-A1, D-47-A2, D-47-A3 — first-commit-of-Plan-47-01 lock timing)
    - .planning/phases/42-upst5-audit/42-01-PLAN.md (Task 1 mechanical preamble precedent)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-LOCK-NOTES.md</files>
  <action>
    Run `git fetch upstream --tags` to refresh upstream/main HEAD and tag pointers. Then capture the post-fetch upstream/main sha via `git rev-parse upstream/main`. Verify the four UPST6 anchor tags resolve locally: v0.54.0 (must equal `6b00932f`), v0.55.0 (must equal `35f9fea2`), v0.56.0 (must equal `b251c72f`), v0.57.0 (must equal `10cec984`).

    Write a holding-file `47-01-LOCK-NOTES.md` in the phase dir capturing verbatim:
    - `upstream_head_at_audit: <40-char post-fetch sha>` (D-47-A3 captured-at-first-commit-of-Plan)
    - `v0.54.0_sha: 6b00932f` (assert)
    - `v0.55.0_sha: 35f9fea2` (assert)
    - `v0.56.0_sha: b251c72f` (assert)
    - `v0.57.0_sha: 10cec984` (assert)
    - `fetch_date: <UTC date>` (timestamp the fetch act)

    This file is the source-of-truth for Task 3's frontmatter `upstream_head_at_audit` field and gives Plan 47-02 a referenceable record of when UPST6 audit fetched. Commit this file standalone with DCO sign-off.
  </action>
  <verify>
    <automated>test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-LOCK-NOTES.md && grep -q "^upstream_head_at_audit: [a-f0-9]\{40\}$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-LOCK-NOTES.md && grep -q "^v0.57.0_sha: 10cec984" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-LOCK-NOTES.md</automated>
  </verify>
  <acceptance_criteria>
    - `git fetch upstream --tags` exits 0
    - `git rev-parse v0.54.0` returns a sha starting with `6b00932f`
    - `git rev-parse v0.55.0` returns a sha starting with `35f9fea2`
    - `git rev-parse v0.56.0` returns a sha starting with `b251c72f`
    - `git rev-parse v0.57.0` returns a sha starting with `10cec984`
    - `47-01-LOCK-NOTES.md` exists with `upstream_head_at_audit:` line carrying a 40-char hex sha
    - Lock-notes commit signed with `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
  </acceptance_criteria>
  <done>upstream_head_at_audit locked to file; all four UPST6 anchor tags resolved; commit signed.</done>
</task>

<task type="auto">
  <name>Task 2: Run drift-tool for v0.54.0..v0.57.0 and redirect JSON to ci-logs-local/drift/ (mechanical preamble)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-A2 drift_tool_invocation; D-47-E1 raw-JSON-not-committed)
    - .planning/phases/42-upst5-audit/42-01-PLAN.md (Task 2 drift-tool invocation precedent)
    - scripts/check-upstream-drift.sh (drift-tool source for context; sha 0834aa664fbaf4c5e41af5debece292992211559 invariant)
  </read_first>
  <files>ci-logs-local/drift/&lt;UTC-timestamp&gt;-v054-v057.json</files>
  <action>
    Ensure `ci-logs-local/drift/` exists (mkdir -p). Run the locked drift-tool invocation EXACTLY:
    `make check-upstream-drift ARGS="--from v0.54.0 --to v0.57.0 --format json" > ci-logs-local/drift/&lt;UTC-timestamp&gt;-v054-v057.json`

    If `make` is not on PATH (Windows host), fall back to direct script dispatch per Phase 33/39/42 precedent:
    `bash scripts/check-upstream-drift.sh --from v0.54.0 --to v0.57.0 --format json > ci-logs-local/drift/&lt;UTC-timestamp&gt;-v054-v057.json`

    Verify the script sha matches the invariant pinned in CONTEXT D-47-A2 / D-47-E1:
    `sha256sum scripts/check-upstream-drift.sh` (or platform-equivalent) must produce `0834aa664fbaf4c5e41af5debece292992211559` — if it does NOT, ABORT and surface via AskUserQuestion (the reproducibility pin is structurally invalidated; cannot proceed without explicit auditor decision per D-47-E10).

    Capture from the JSON: `total_unique_commits` (becomes the ledger row-count target per D-47-B4 step 2), category distribution (informs cluster grouping in Task 4), and the per-commit metadata (sha, subject, files_changed, categories). DO NOT commit the JSON; `ci-logs-local/` is in `.gitignore` per D-47-E1 / D-33-A2 inherited.
  </action>
  <verify>
    <automated>test -f ci-logs-local/drift/*-v054-v057.json && grep -q "total_unique_commits" ci-logs-local/drift/*-v054-v057.json && git check-ignore -q ci-logs-local/drift/*-v054-v057.json</automated>
  </verify>
  <acceptance_criteria>
    - JSON file exists under `ci-logs-local/drift/` matching glob `*-v054-v057.json`
    - JSON contains `total_unique_commits` field (drift-tool schema invariant)
    - `git check-ignore` confirms the file is ignored (not staged)
    - `scripts/check-upstream-drift.sh` sha equals `0834aa664fbaf4c5e41af5debece292992211559` (D-47-A2 reproducibility pin)
    - Drift-tool exit code is 0
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified by this task (D-47-E5 invariant)
  </acceptance_criteria>
  <done>Drift-tool JSON output captured to ignored path; `total_unique_commits` available for Task 4 row-count gate.</done>
</task>

<task type="auto">
  <name>Task 3: Write ledger frontmatter + Headline + Reproduction + Cluster Summary scaffold</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-A2 frontmatter fields; D-47-E3 two-tier structure; D-47-B4 close-gate)
    - .planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md (worked template — frontmatter shape, Headline paragraph, Reproduction block, Cluster Summary table)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-LOCK-NOTES.md (read upstream_head_at_audit from Task 1)
    - ci-logs-local/drift/*-v054-v057.json (read total_unique_commits + category preview for cluster-count placeholder)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</files>
  <action>
    Create `DIVERGENCE-LEDGER.md` with frontmatter (verbatim values from CONTEXT D-47-A2):

    ```
    ---
    phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion
    plan: 01
    ledger_type: upst6-audit
    range: v0.54.0..v0.57.0
    upstream_head_at_audit: &lt;sha from 47-01-LOCK-NOTES.md&gt;
    drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
    drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
    drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.54.0 --to v0.57.0 --format json"'
    fork_baseline: v0.54.0 (Phase 43 + 45 UPST5 sync point — Cluster 5 0748cced/5d821c12 + Cluster 2 8b888a1c source migration absorbed 2026-05-18..2026-05-20)
    date: &lt;ship date YYYY-MM-DD&gt;
    ---
    ```

    Then add these top-level sections (placeholders for auditor to fill in Task 4):

    1. `# Phase 47 UPST6 Audit — Upstream v0.54.0..v0.57.0 Divergence Ledger`
    2. `## Headline` — one-paragraph audit summary. Placeholder reads: "TBD at audit-walk close — auditor fills with cluster count, total commit count, disposition breakdown, windows-touch:yes count, and ADR-review outcome verdict."
    3. `## Reproduction` — block recording the drift-tool invocation verbatim, the JSON output path (ci-logs-local/drift/*-v054-v057.json, ignored), and an `auditor-rerun:` line documenting how a fresh auditor reproduces the inventory against the same `range` + `upstream_head_at_audit` + `drift_tool_sh_sha`.
    4. `## Cluster Summary` — markdown table header only, columns: `cluster_id | theme | commits | disposition | windows-touch | rationale`. Body rows placeholder: `<!-- auditor fills in Task 4 -->`.
    5. Stub cluster sections placeholder: `<!-- ### Cluster 1: ... (auditor fills in Task 4) -->`. Include a count-placeholder comment matching the drift-tool category preview (estimate 7-10 clusters per CONTEXT § Drift signal preview).
    6. Stub `## ADR review` header (D-47-E8 placeholder; body filled in Task 6).
    7. Stub `## Empirical cross-check` header (D-47-D1 placeholder; body filled in Task 7).
    8. Stub `## Cross-cluster re-export deps detected` header (D-47-D3 placeholder; body filled in Task 7).

    Commit with DCO sign-off: `docs(47-01): scaffold UPST6 divergence ledger frontmatter + section headers`.
  </action>
  <verify>
    <automated>test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md && grep -q "^range: v0.54.0..v0.57.0$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md && grep -q "^drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md && grep -q "^## ADR review$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md && grep -q "^## Empirical cross-check$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md && grep -q "^## Cross-cluster re-export deps detected$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</automated>
  </verify>
  <acceptance_criteria>
    - `DIVERGENCE-LEDGER.md` exists at the canonical path
    - Frontmatter contains `range: v0.54.0..v0.57.0` (exact match D-47-A1)
    - Frontmatter contains `upstream_head_at_audit:` with a 40-char hex sha (matches Task 1 lock)
    - Frontmatter contains `drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559` (exact match D-47-A2 reproducibility pin)
    - Frontmatter contains `drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559`
    - Frontmatter contains `drift_tool_invocation:` with verbatim invocation string from D-47-A2
    - Frontmatter contains `fork_baseline:` referencing Phase 43 + 45 UPST5 sync point per D-47-A2
    - Headers `## Headline`, `## Reproduction`, `## Cluster Summary`, `## ADR review`, `## Empirical cross-check`, `## Cross-cluster re-export deps detected` all present (grep-confirmable)
    - Cluster Summary table has header row with exact columns: `cluster_id | theme | commits | disposition | windows-touch | rationale`
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>Ledger scaffold committed with all D-47-A2 frontmatter fields and all D-47-D1/D3/E8 mandatory section headers present as placeholders.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 4: Audit-walk — cluster grouping + per-cluster sections + dispositions + windows-touch + commit-row tables (HUMAN-IN-THE-LOOP)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-A5 row schema; D-47-C3 4-disposition vocab including split; D-47-E3 two-tier structure)
    - .planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md (worked template — cluster headers + nested commit-row tables; per-cluster Disposition + Rationale blocks)
    - ci-logs-local/drift/*-v054-v057.json (read full commit inventory)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (current state from Task 3)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</files>
  <action>
    **HUMAN AUDIT-WALK REQUIRED.** Auto-runner cannot make substantive cluster-grouping or disposition decisions.

    For each upstream commit in v0.54.0..v0.57.0 (from drift-tool JSON):
    1. **Cluster grouping** — group commits into themed clusters (auditor's judgment; CONTEXT § Drift signal preview suggests 7-10 clusters likely covering: pack-management evolution, platform-detection/registry deltas, AIPC schema evolution, trust/signing/sigstore-verify bumps, snapshot validation iteration, standard dep bumps, macOS lint fixes, 3 release commits per Phase 34/40 release-ride convention).
    2. **Per-cluster section** — write `### Cluster N: <theme>` with these blocks IN ORDER:
       - `**Commits:**` count + per-commit subject preview
       - `**Disposition:**` one of `will-sync` / `fork-preserve` / `won't-sync` / `split` (D-47-C3 standard 4-disposition vocab)
       - `**Windows-touch:**` `yes` or `no` per D-47-A5 column; apply D-47-A5 heuristic (substring 'windows' in files_changed OR pinned list {platform.rs, registry.rs, wfp/*, win_*.rs} OR commit-subject keywords 'windows|wfp|registry|wsa|ntdll|kernel32')
       - `**Rationale:**` one-paragraph justification for disposition
       - **For `will-sync` clusters ONLY:** insert placeholder `**Cross-cluster re-export check:**` subsection (filled in Task 5)
       - Commit-row table with D-47-A5 schema: `| sha | subject | upstream-tag | categories | files-changed | windows-touch |`
    3. **Conservative defaults:**
       - `windows-touch: yes` clusters default to `fork-preserve` unless empty fork-side proven by diff inspection (D-42-C3 inherited)
       - Release commits (v0.55.0 `35f9fea2` / v0.56.0 `b251c72f` / v0.57.0 `10cec984`) handled per Phase 34/40 release-ride convention (CHANGELOG-only; drop Cargo.toml + Cargo.lock version bumps)
    4. **Cluster Summary table population** — fill the table body created in Task 3.
    5. **Headline paragraph population** — fill with cluster count, total commit count, disposition breakdown (will-sync/fork-preserve/won't-sync/split counts), windows-touch:yes count, and a TBD-pending-Task-6-ADR-outcome verdict line.
    6. **Row-count gate verification** — sum total commit-rows across all cluster tables; MUST be >= drift-tool `total_unique_commits` (D-47-B4 step 2). If short, auditor surfaces a gap and re-walks the inventory.

    Commit with DCO sign-off: `docs(47-01): populate UPST6 cluster sections with dispositions + windows-touch column`.

    **Resume signal:** Type "audit-walk complete" or describe blockers (e.g., "cluster N disposition unclear — see Task 5 re-export scan first").
  </action>
  <verify>
    <automated>grep -c "^### Cluster " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }' && grep -cE "^\*\*Disposition:\*\* (will-sync|fork-preserve|won't-sync|split)$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }' && grep -cE "^\*\*Windows-touch:\*\* (yes|no)$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }'</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "^### Cluster " DIVERGENCE-LEDGER.md` returns N where N matches auditor-claimed cluster count from Cluster Summary table
    - Every cluster section has `**Disposition:**` line with one of exactly four values: `will-sync` / `fork-preserve` / `won't-sync` / `split`
    - Every cluster section has `**Windows-touch:**` line with `yes` or `no`
    - Every cluster section has `**Rationale:**` paragraph (non-empty)
    - Every cluster has a commit-row table with header `| sha | subject | upstream-tag | categories | files-changed | windows-touch |`
    - Cluster Summary table body populated (no `<!-- auditor fills -->` placeholder remaining)
    - Sum of commit-rows across all cluster tables >= drift-tool `total_unique_commits` for v0.54.0..v0.57.0
    - Headline paragraph populated with cluster count + disposition breakdown + windows-touch:yes count
    - No commit row references any sha past `10cec984` (D-47-A4 strictly-silent on post-v0.57.0)
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>All cluster sections fully populated with disposition + windows-touch + rationale + commit-row table; row-count gate satisfied; Cluster Summary table body filled.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 5: Cross-cluster re-export scan on every will-sync cluster's lead commit + default flip-to-split on detection (HUMAN-IN-THE-LOOP)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-D1, D-47-D2, D-47-D3, D-47-D4 re-export hardening; feedback_cluster_isolation_invalid lesson)
    - .planning/phases/43-upst5-sync-execution/43-01-EDITION-2024-FOUNDATION-SUMMARY.md (the empirical Cluster 2 discovery that proves this scan structurally necessary)
    - .planning/phases/43-upst5-sync-execution/43-01b-EDITION-WORKSPACE-ONLY-SUMMARY.md (split-disposition execution mechanism precedent — workspace-only mechanical edits, source-migration deferred)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (current state from Task 4)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</files>
  <action>
    **HUMAN INTERPRETATION REQUIRED.** Auto-runner cannot judge whether a `pub use` / `pub mod` / `extern crate` / `pub(crate)` match is a cross-cluster dependency vs an intra-cluster re-export.

    For EVERY `will-sync` cluster identified in Task 4 (D-47-D2 — uniform discipline; no foundation/size threshold):
    1. Identify the cluster's **lead commit** (auditor's judgment — typically the foundation/largest commit of the cluster).
    2. Run `git show --stat <lead-commit-sha>` to list files touched.
    3. For each file in the lead commit, run targeted greps:
       - `git show <lead-commit-sha>:<file> | grep -nE "^pub use "` (cross-module re-exports)
       - `git show <lead-commit-sha>:<file> | grep -nE "^pub mod "` (module declarations exposing prerequisite-cluster types)
       - `git show <lead-commit-sha>:<file> | grep -nE "^extern crate "` (rare in 2024 edition; check anyway)
       - `git show <lead-commit-sha>:<file> | grep -nE "pub\(crate\) "` (crate-internal re-exports that may pull from prereq cluster's files)
    4. For each match: trace the imported/re-exported symbol back to its definition. If the definition lives in ANOTHER cluster (within v0.54.0..v0.57.0 range), this is a CROSS-CLUSTER RE-EXPORT DEP — analogous to Phase 43 Cluster 2's `public_key_id_hex` + `sign_statement_bundle` discovery.
    5. Write a `**Cross-cluster re-export check:**` subsection in the cluster's body (under Disposition + Rationale block, before the commit-row table):
       - If clean: `**Cross-cluster re-export check:** Clean — scanned lead commit <sha> for pub use / pub mod / extern crate / pub(crate) declarations; no cross-cluster deps detected.`
       - If dirty: `**Cross-cluster re-export check:** CROSS-CLUSTER DEP DETECTED. Lead commit <sha> re-exports <symbol> in <file> from prerequisite cluster <cluster-id> (lead commit <prereq-sha>). Per D-47-D4 default, disposition flipped from will-sync → split.` Then add `**Prerequisite enumeration:**` line listing prereq cluster's lead commit + re-exported symbol(s).
    6. **D-47-D4 default flip:** Any cluster where the scan surfaces a cross-cluster dep MUST have its `**Disposition:**` line changed from `will-sync` to `split`. Update the Cluster Summary table accordingly. Mechanically-resolvable portion (workspace edits, trivial absorbable surface) is fork-authored in Phase 48; cross-cluster source migration deferred until the prereq cluster is absorbed (typically a subsequent UPST cycle).
    7. **Consolidate findings** in the `## Cross-cluster re-export deps detected` summary subsection (D-47-D3): list all detected edges as `source-cluster-id → prereq-cluster-id (symbol)`. If zero deps detected across all `will-sync` clusters, document explicitly: `No cross-cluster re-export deps detected across N will-sync clusters scanned.`

    Commit with DCO sign-off: `docs(47-01): add cross-cluster re-export scan + apply D-47-D4 split flips`.

    **Resume signal:** Type "re-export scan complete" or "flipped N clusters to split — see Cluster Summary".
  </action>
  <verify>
    <automated>grep -c "^\*\*Cross-cluster re-export check:\*\*" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 &gt;= 1) ? 0 : 1 }' && grep -q "^## Cross-cluster re-export deps detected$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</automated>
  </verify>
  <acceptance_criteria>
    - Every `will-sync` cluster (from Task 4 Cluster Summary table) has a `**Cross-cluster re-export check:**` subsection
    - Each subsection explicitly references the lead commit sha + scan patterns (`pub use` / `pub mod` / `extern crate` / `pub(crate)`)
    - Any cluster with a detected cross-cluster dep has its `**Disposition:**` line flipped to `split` AND a `**Prerequisite enumeration:**` line listing prereq cluster's lead commit + re-exported symbol(s) per D-47-D4
    - `## Cross-cluster re-export deps detected` summary subsection populated (either consolidated edge list OR explicit zero-detected statement)
    - Cluster Summary table reflects any disposition flips from Task 4
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>Re-export scan complete on every will-sync cluster; D-47-D4 flips applied; Cross-cluster summary subsection populated; feedback_cluster_isolation_invalid lesson structurally closed for UPST6 cycle.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 6: Write ## ADR review section with per-cell L/M/H verdicts on 5 dimensions (HUMAN ADR JUDGMENT)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-E8 ADR review MANDATORY; 5-dimension table; 3 outcome options)
    - docs/architecture/upstream-parity-strategy.md (Phase 33 ADR Option A 'continue' — LOCKED Accepted; auditor verdicts but does NOT supersede)
    - .planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md (Phase 42 worked ## ADR review section with per-cell L/M/H verdicts — the verbatim template Plan 47-01 mirrors)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (current state from Tasks 4-5)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</files>
  <action>
    **HUMAN ADR JUDGMENT REQUIRED.** Auto-runner cannot make ADR L/M/H verdicts based on ~75-commit evidence.

    Write the `## ADR review` section body (header already exists as Task 3 placeholder):

    1. **Preamble paragraph:** Reference Phase 33 ADR `docs/architecture/upstream-parity-strategy.md` Option A `continue` (Accepted 2026-05-11, re-confirmed v2.4 + v2.5 close). State that Phase 47 UPST6 is the ~75-commit evidence base — the largest single audit cycle yet (vs Phase 42's 18, Phase 39's 22, Phase 33's 97 but cumulative).

    2. **5-dimension per-cell L/M/H verdict table** with EXACTLY these row labels (D-47-E8 falsifiability gate):
       ```
       | dimension | verdict | rationale |
       |-----------|---------|-----------|
       | security  | L/M/H   | <evidence from ~75 commits — e.g., snapshot validation iteration, sigstore-verify bumps, trust/signing scope> |
       | windows   | L/M/H   | <evidence — Phase 43 + 45 absorbed Cluster 2 source migration + Cluster 5 platform detection; v0.55+ iteration risk> |
       | maintenance | L/M/H | <evidence — cherry-pick labor for ~75 commits vs deferral cost> |
       | divergence  | L/M/H | <evidence — count of fork-preserve + split clusters vs will-sync> |
       | contributor | L/M/H | <evidence — fork's ability to contribute back via Phase 48 umbrella PR; project_cross_fork_pr_pattern reference> |
       ```

    3. **Outcome verdict** — auditor chooses ONE of three (D-47-E8 menu):
       - **(a) Confirm Option A `continue`** — write `**Outcome:** (a) Confirm. Phase 33 ADR Option A 'continue' remains the right call.` Recommended provisional verdict per CONTEXT § Claude's Discretion (auditor confirms or revises).
       - **(b) Amend with carve-outs** — write `**Outcome:** (b) Amend with carve-outs.` Then enumerate carve-outs (e.g., "carve out cluster N's foundation pattern for fork-preserve review every UPST cycle").
       - **(c) Flag a future-supersede trigger** — write `**Outcome:** (c) Flag future-supersede trigger.` Then describe the trigger condition (e.g., "if next UPST cycle (UPST7) surfaces > 50% fork-preserve / split cluster ratio, propose Option B in a superseding ADR").

    4. **Phase 47 does NOT supersede Phase 33 ADR.** The ADR stays `Status: Accepted` regardless of verdict outcome. If outcome is (c), the future-supersede trigger is a FLAG for a future phase, not a Phase 47 inline edit.

    Commit with DCO sign-off: `docs(47-01): add ## ADR review section with per-cell L/M/H verdicts`.

    **Resume signal:** Type "ADR review complete — outcome (a|b|c)".
  </action>
  <verify>
    <automated>grep -c "^## ADR review$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 == 1) ? 0 : 1 }' && grep -cE "^\| (security|windows|maintenance|divergence|contributor) " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 &gt;= 5) ? 0 : 1 }' && grep -qE "^\*\*Outcome:\*\* \((a|b|c)\)" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "^## ADR review$" DIVERGENCE-LEDGER.md` returns exactly 1 (D-47-E8 MANDATORY single section)
    - `grep -cE "^\| (security|windows|maintenance|divergence|contributor) " DIVERGENCE-LEDGER.md` returns >= 5 (per-cell verdict table with all 5 dimensions present)
    - Each dimension row contains an L/M/H verdict value
    - `**Outcome:**` line present with one of three values: `(a) Confirm`, `(b) Amend with carve-outs`, `(c) Flag future-supersede trigger`
    - Phase 33 ADR is referenced explicitly (grep for `Phase 33 ADR` or `upstream-parity-strategy.md`)
    - `docs/architecture/upstream-parity-strategy.md` was NOT modified by this task (verdict does not supersede)
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>## ADR review section populated with per-cell L/M/H verdicts on 5 dimensions + outcome verdict; Phase 33 ADR stays Accepted.</done>
</task>

<task type="checkpoint:human-action">
  <name>Task 7: Write ## Empirical cross-check section (≥4 fork-shared files) + consolidate cross-cluster re-export summary (HUMAN FILE-WALK)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-D1 ≥4 files; D-47-D3 consolidation; D-47-E12 preferential sampling — Phase 43 + 45 absorption surfaces)
    - .planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md (Phase 42 ## Empirical cross-check ≥3-file precedent — Plan 47-01 raises to ≥4)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md (current state from Tasks 4-6 — Cross-cluster summary subsection populated by Task 5; consolidate here)
  </read_first>
  <files>.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</files>
  <action>
    **HUMAN FILE-WALK REQUIRED.** Auto-runner cannot judge which files matter most for the empirical cross-check.

    Write the `## Empirical cross-check` section body (header already exists as Task 3 placeholder):

    1. **Preamble paragraph:** Explain the empirical cross-check purpose — spot-check fork-shared files against upstream v0.54.0..v0.57.0 log to detect any upstream commits the drift tool's D-11 path filter (which excludes `*_windows.rs` + `crates/nono-cli/src/exec_strategy_windows/`) may have missed. Closes the `feedback_cluster_isolation_invalid` lesson hardened at v2.5 close.

    2. **File walk — ≥4 fork-shared files** (D-47-D1 raises Phase 42 ≥3 → ≥4 per Phase 47 SC#3). Preferentially sample per D-47-E12:
       - `crates/nono-cli/src/platform.rs` (Phase 43 Cluster 5 absorption surface — `0748cced` + `5d821c12`; v0.55+ iteration high-risk surface)
       - `crates/nono/src/trust/` (Phase 43 Cluster 2 + Phase 45 Plan 45-01 source-migration target — `8b888a1c` ancestor surface; trust/signing high-risk surface)
       - AIPC schema files (e.g., `crates/nono/src/aipc/`, `crates/nono-cli/src/aipc_sdk.rs`) — Phase 45 Plan 45-02 REQ-AIPC-G04-01 wire-protocol tightening surface; v0.55+ schema evolution risk
       - Phase 45 Plan 45-01 source-migration target files (commits `f640528a..d21399e3`) — most-recently-absorbed Edition 2024 surface; v0.55+ may iterate

    3. **Per-file walk format** (auditor picks ≥4):
       ```
       ### File: <path>
       - Walked upstream log: `git log v0.54.0..v0.57.0 -- <path>`
       - Commits touching this file in range: <count>
       - Cluster mapping: <which cluster(s) in this ledger cover these commits>
       - Drift-tool coverage: <PASS — drift tool caught all> / <FAIL — drift tool missed sha XXXXXXXX (file outside D-11 path filter or category mis-tag); see D-47-E10 follow-up spawn>
       ```

    4. **Drift-tool gap surfacing** — if any file walk reveals an upstream commit the drift tool missed, document the gap inline + spawn a `.planning/quick/YYMMDD-xxx-upstream-drift-tool-fix/` quick-task per D-47-E10 (Plan 47-01 itself stays untouched to preserve `drift_tool_sh_sha` reproducibility).

    5. **Cross-cluster re-export consolidation** — verify the `## Cross-cluster re-export deps detected` summary subsection (populated in Task 5) is present and lists all detected edges. If Task 5 found zero deps, the subsection should explicitly say so.

    Commit with DCO sign-off: `docs(47-01): add ## Empirical cross-check section + ≥4 file walks + cross-cluster consolidation`.

    **Resume signal:** Type "empirical cross-check complete — N files walked".
  </action>
  <verify>
    <automated>grep -c "^## Empirical cross-check$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 == 1) ? 0 : 1 }' && grep -c "^### File: " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md | awk '{ exit ($1 &gt;= 4) ? 0 : 1 }' && grep -q "^## Cross-cluster re-export deps detected$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "^## Empirical cross-check$" DIVERGENCE-LEDGER.md` returns exactly 1
    - `grep -c "^### File: " DIVERGENCE-LEDGER.md` returns >= 4 (D-47-D1 raised threshold per Phase 47 SC#3)
    - At least one walked file is from `crates/nono-cli/src/platform.rs` OR `crates/nono/src/trust/` OR an AIPC schema file OR a Phase 45 Plan 45-01 source-migration target (D-47-E12 preferential sampling honored)
    - Each `### File:` block contains a coverage verdict (PASS or FAIL)
    - If any FAIL verdict present, a `.planning/quick/*-upstream-drift-tool-fix/` quick-task is spawned (D-47-E10)
    - `## Cross-cluster re-export deps detected` subsection (from Task 5) is consolidated under or adjacent to the Empirical cross-check section
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>## Empirical cross-check populated with ≥4 file walks honoring D-47-E12 preferential sampling; drift-tool gaps surfaced if any; Cross-cluster re-export deps detected subsection consolidated.</done>
</task>

<task type="auto">
  <name>Task 8: Append UPST7 stub to ROADMAP.md (mechanical scaffolding)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-E11 — UPST7 stub location + shape; default v2.6 § Future Cycles)
    - .planning/ROADMAP.md (find correct insertion point — § Future Cycles holding section if exists, else v2.6 backlog area)
    - docs/architecture/upstream-parity-strategy.md (for ADR cross-reference URL)
  </read_first>
  <files>.planning/ROADMAP.md</files>
  <action>
    Append a UPST7 placeholder phase entry per D-47-E11 shape:

    ```
    ### UPST7 — Upstream v0.57.0… sync audit (placeholder)

    **Goal**: Audit upstream `v0.57.0..<next-tag>` divergence per the Phase 33 ADR `continue` cadence rule. Inherits the audit-shape template from Phase 33 + 39 + 42 + 47 verbatim. Title may flip from `sync audit` to `sync execution` if next cycle's commit set is small enough to skip a dedicated audit (auditor's call at UPST7 plan-phase).
    **Depends on**: Phase 48 (UPST6 sync execution must close before UPST7 audit; cadence rule preserves linear ordering)
    **Plans**: 0 / TBD
    **Reference**: `docs/architecture/upstream-parity-strategy.md` § Future audit cadence

    UPST7 fires when next upstream release ships OR maintainer decides the accumulated cherry-pick labor (19 known post-v0.57.0 commits at Phase 47 context-capture time on 2026-05-23; will grow before UPST7 fires) warrants absorbing.
    ```

    Default insertion location: under a `## Future Cycles` heading in the v2.6 milestone section. If `## Future Cycles` does not exist, create it at the bottom of the v2.6 milestone section before any post-v2.6 milestone block. Title wording defaults to `… sync audit`; auditor may flip to `… sync execution` per D-47-E11 + CONTEXT § Claude's Discretion if UPST6 ledger shape suggests next cycle could skip a dedicated audit.

    Also update Phase 47's Plans counter: `Plans: 1 / 2` (will flip to `2 / 2` after Plan 47-02 closes). Mark Plan 47-01 entry as `[x] 47-01-UPST6-AUDIT-PLAN.md` in the Phase 47 plan list.

    Commit with DCO sign-off: `docs(47-01): append UPST7 stub + update Phase 47 plans counter`.
  </action>
  <verify>
    <automated>grep -q "^### UPST7 — Upstream v0.57.0" .planning/ROADMAP.md &amp;&amp; grep -q "Depends on.*Phase 48" .planning/ROADMAP.md &amp;&amp; grep -q "Future audit cadence" .planning/ROADMAP.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "^### UPST7 — Upstream v0.57.0" ROADMAP.md` succeeds
    - UPST7 stub contains `Depends on: Phase 48` (D-47-E11 dependency)
    - UPST7 stub contains `Plans: 0 / TBD` (D-47-E11 shape)
    - UPST7 stub contains a reference to `docs/architecture/upstream-parity-strategy.md § Future audit cadence`
    - Phase 47 Plans counter reflects `1 / 2` (or equivalent indicating Plan 47-01 complete)
    - Plan 47-01 marked `[x]` in Phase 47 plan list
    - No file under `crates/`, `bindings/`, `scripts/`, or `Makefile` modified
  </acceptance_criteria>
  <done>ROADMAP.md updated with UPST7 stub + Phase 47 plans counter bump.</done>
</task>

<task type="auto">
  <name>Task 9: Close-gate verification + STATE.md update + Plan 47-01 SUMMARY (mechanical close)</name>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-CONTEXT.md (D-47-B4 8-step close-gate)
    - .planning/STATE.md (current frontmatter + Accumulated Context format)
    - .planning/phases/42-upst5-audit/42-01-SUMMARY.md (SUMMARY shape precedent)
    - $HOME/.claude/get-shit-done/templates/summary.md (SUMMARY template)
  </read_first>
  <files>
    .planning/STATE.md, .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md
  </files>
  <action>
    **Step 1 — Run all 8 close-gate checks** (D-47-B4 for Plan 47-01 scope):
    1. `make check-upstream-drift ARGS="--from v0.54.0 --to v0.57.0 --format json"` exits 0 (drift-tool idempotence re-run; output to new ci-logs-local/ file; NOT committed)
    2. Ledger row count >= drift-tool `total_unique_commits` for v0.54.0..v0.57.0 (compare sum of commit-rows in cluster tables vs JSON `total_unique_commits` field)
    3. Every cluster has disposition (`will-sync` / `fork-preserve` / `won't-sync` / `split`) + one-line rationale (grep)
    4. `## ADR review` section present with per-cell L/M/H verdicts on 5 dimensions (grep)
    5. `## Empirical cross-check` ≥4 files (grep) + `## Cross-cluster re-export deps detected` summary subsection present (grep)
    6. ROADMAP UPST7 stub committed (grep `### UPST7 — Upstream v0.57.0`)
    7. STATE.md updated (Step 2 below)
    8. `git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/` returns zero lines (Plan 47-01 ships zero source edits — D-47-E5 invariant)

    **Step 2 — Update STATE.md:**
    - Bump frontmatter `completed_plans` counter (current 12 → 13 after Plan 47-01)
    - Update `last_activity` to today's date
    - Update `## Current Position` — Phase 47 still in-progress (Plan 47-02 pending per D-47-B4 strict-both-close gate); Last activity stamped
    - Append a `Plan 47-01 close entry` under `## Accumulated Context > Key Decisions (v2.6)` capturing:
      - Range: v0.54.0..v0.57.0
      - upstream_head_at_audit: <40-char sha>
      - Cluster count + commit count + disposition breakdown (will-sync/fork-preserve/won't-sync/split counts)
      - windows-touch:yes count
      - ADR-review outcome verdict (a/b/c per Task 6)
      - Empirical cross-check files sampled count
      - Cross-cluster re-export deps detected count (split-flips applied)
      - UPST7 stub location decision + commit sha
      - DCO sign-off attestation: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

    **Step 3 — Create Plan 47-01 SUMMARY.md** at `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md` mirroring Phase 42 Plan 42-01-SUMMARY shape:
    - Frontmatter: `plan: 01`, `phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion`, `status: complete`, `requirements: [REQ-UPST6-01]`, `date: <ship date>`, `must_haves_verified: <count from must_haves.truths>`
    - Sections: `## Summary`, `## Artifacts Created`, `## Close-Gate Verification` (8 checks from Step 1 with pass/fail per check), `## Disposition Breakdown` (table), `## ADR Review Outcome`, `## Cross-cluster Re-export Findings`, `## Empirical Cross-Check Files`, `## Next Steps` (Plan 47-02 dependency)

    Commit with DCO sign-off: `docs(47-01): close-gate verification + STATE.md update + Plan 47-01 SUMMARY`.
  </action>
  <verify>
    <automated>test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md &amp;&amp; grep -q "^status: complete$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md &amp;&amp; grep -q "Plan 47-01" .planning/STATE.md &amp;&amp; bash -c 'cd /c/Users/OMack/Nono &amp;&amp; git diff --name-only HEAD~6..HEAD -- crates/ bindings/ scripts/ | wc -l | awk "{ exit (\$1 == 0) ? 0 : 1 }"'</automated>
  </verify>
  <acceptance_criteria>
    - All 8 D-47-B4 close-gate checks pass (drift-tool re-run idempotent; row-count gate satisfied; mandatory sections present; ROADMAP stub committed; STATE updated; zero source edits)
    - `47-01-SUMMARY.md` exists with `status: complete` in frontmatter
    - SUMMARY contains sections: `## Summary`, `## Artifacts Created`, `## Close-Gate Verification`, `## Disposition Breakdown`, `## ADR Review Outcome`, `## Cross-cluster Re-export Findings`, `## Empirical Cross-Check Files`, `## Next Steps`
    - STATE.md frontmatter `completed_plans` counter bumped
    - STATE.md Last activity stamped to today
    - STATE.md `## Accumulated Context` gains Plan 47-01 close entry referencing range, lock-sha, cluster count, disposition breakdown, ADR outcome, cross-cluster deps detected, UPST7 stub location
    - `git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/ | wc -l` returns 0 (Plan 47-01 trivially honors D-47-E5)
    - All commits in Plan 47-01 signed with `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
  </acceptance_criteria>
  <done>Plan 47-01 close-gate fully verified; STATE.md + SUMMARY.md committed; Phase 47 still in-progress pending Plan 47-02; D-47-E5 zero-source-edits invariant trivially honored.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Auditor → ledger artifact | Audit-walk decisions captured in markdown; reader trusts auditor's cluster grouping + disposition + L/M/H verdicts |
| Drift-tool → ledger frontmatter | `drift_tool_sh_sha` invariant lets future auditors reproduce input set against same range + HEAD |
| Phase 47 ledger → Phase 48 planner | Per-cluster dispositions are immutable input for Phase 48 cherry-pick selection |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-47-01 | I (Integrity) | DIVERGENCE-LEDGER.md frontmatter `drift_tool_sh_sha` | mitigate | Task 2 asserts `sha256sum scripts/check-upstream-drift.sh` == `0834aa664fbaf4c5e41af5debece292992211559`; ABORT + AskUserQuestion if mismatch (D-47-E10) |
| T-47-02 | I (Integrity) | Cluster disposition assignment (will-sync vs split flip) | mitigate | Task 5 D-47-D4 default flip-to-split on detected cross-cluster re-export dep; D-47-D2 uniform discipline across every will-sync cluster's lead commit (no foundation/size threshold) |
| T-47-03 | I (Integrity) | ADR review outcome verdict | mitigate | Task 6 records Phase 33 ADR `Status: Accepted` invariant explicitly; outcome (c) `future-supersede trigger` is a flag, not an inline ADR edit |
| T-47-04 | R (Repudiation) | Per-commit attribution in ledger | mitigate | Every commit row carries verbatim upstream sha + subject (drift-tool source); DCO sign-off on every Plan 47-01 commit (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`) |
| T-47-05 | D (Denial of Service) | Drift-tool unavailability | accept | If `make` not on PATH, fallback to `bash scripts/check-upstream-drift.sh` per Phase 33/39/42 precedent; no further escalation needed |
| T-47-06 | C (Confidentiality) | Raw drift-tool JSON output | mitigate | D-47-E1 / D-33-A2 inherited — JSON redirected to `ci-logs-local/drift/` per `.gitignore`; never committed; ledger is the canonical artifact |
| T-47-07 | E (Elevation of Privilege) | N/A — audit-only phase | accept | Phase 47 ships ZERO `.rs` / `.toml` / `.sh` / `.ps1` / `Makefile` edits (D-47-E5 / D-47-B4 step 8); no runtime change; structurally cannot elevate privilege |
| T-47-08 | S (Spoofing) | UPST7 stub authenticity | accept | UPST7 stub is a placeholder for a future audit cycle; auditor sign-off via DCO on Task 8 commit; future UPST7 plan-phase re-validates |

## Structural Mitigations (audit-phase invariants)

- **Confidentiality:** No new credentials, tokens, or secrets handled. Drift-tool JSON output redirects to `ci-logs-local/drift/` per `.gitignore` (D-47-E1 / D-33-A2 inherited) and is NOT committed.
- **Integrity:** Ledger artifacts are the audit-of-record. Reproducibility against tag pair (v0.54.0..v0.57.0) + `drift_tool_sh_sha 0834aa66` invariant means another auditor re-running the same invocation against the same locked HEAD produces a verifiable ledger. Frontmatter captures both `range` AND `upstream_head_at_audit` (D-47-A2).
- **Availability:** Zero runtime change. Plan 47-01 cannot break any user-facing surface.
- **Audit:** All ledger commits use DCO sign-off (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` — fork co-author convention).
- **Supply chain:** Drift-tool sha pin in frontmatter prevents silent tool drift; if `scripts/check-upstream-drift.sh` mutates mid-phase, Task 2 ABORTS with AskUserQuestion (D-47-E10).
</threat_model>

<verification>
1. Run `bash -c 'grep -q "^range: v0.54.0..v0.57.0$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md'` — exits 0
2. Run `bash -c 'grep -c "^### Cluster " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md'` — returns N >= 1 matching auditor-claimed cluster count
3. Run `bash -c 'grep -c "^## ADR review$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md'` — returns 1
4. Run `bash -c 'grep -cE "^\| (security|windows|maintenance|divergence|contributor) " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md'` — returns >= 5
5. Run `bash -c 'grep -c "^### File: " .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md'` — returns >= 4
6. Run `bash -c 'grep -q "^## Cross-cluster re-export deps detected$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md'` — exits 0
7. Run `bash -c 'grep -q "^### UPST7 — Upstream v0.57.0" .planning/ROADMAP.md'` — exits 0
8. Run `bash -c 'cd /c/Users/OMack/Nono && git diff --name-only HEAD~9..HEAD -- crates/ bindings/ scripts/ | wc -l'` — returns 0 (D-47-E5 invariant; Plan 47-01 ships zero source edits across all 9 tasks' commits)
9. Run `make check-upstream-drift ARGS="--from v0.54.0 --to v0.57.0 --format json" > /dev/null` — exits 0 (drift-tool re-run idempotent)
10. Run `bash -c 'test -f .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md && grep -q "^status: complete$" .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md'` — exits 0
</verification>

<success_criteria>
- REQ-UPST6-01 satisfied: DIVERGENCE-LEDGER.md exists at canonical path with all D-47-B4 close-gate conditions met (Plan 47-01 scope; Plan 47-02 completes the phase-level close)
- All grep-confirmable mandatory sections present (`## ADR review`, `## Empirical cross-check`, `## Cross-cluster re-export deps detected`)
- Every cluster has disposition (will-sync / fork-preserve / won't-sync / split) + windows-touch (yes/no) + rationale + (for will-sync) cross-cluster re-export check
- Any detected cross-cluster re-export dep triggered D-47-D4 default flip-to-split with prerequisite enumeration
- ADR review outcome verdict (a/b/c) recorded; Phase 33 ADR stays Accepted
- Empirical cross-check walked ≥4 fork-shared files preferentially sampling Phase 43 + 45 absorption surfaces per D-47-E12
- ROADMAP UPST7 stub committed at D-47-E11-chosen location
- `git diff --name-only HEAD~N..HEAD -- crates/ bindings/ scripts/ | wc -l` == 0 (D-47-E5 zero-source-edits invariant trivially honored)
- All Plan 47-01 commits signed with `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
- Plan 47-01 SUMMARY.md created with `status: complete`; STATE.md updated; Plan 47-02 unblocked
</success_criteria>

<output>
After completion, create `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md` per Task 9 Step 3.
</output>
