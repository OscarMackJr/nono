---
phase: 99-upstream-absorb-fork-invariant-verify
plan: "01"
subsystem: planning-docs
tags: [scope-reconciliation, ledger-alignment, roadmap, requirements]
dependency_graph:
  requires: [98-DIVERGENCE-LEDGER.md]
  provides: [Phase 99 SC text aligned to ledger reality]
  affects: [ROADMAP.md Phase 99, REQUIREMENTS.md UPST11-02/03]
tech_stack:
  added: []
  patterns: [per-PR→cluster traceability table]
key_files:
  modified:
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
decisions:
  - "D-01/D-02 applied: Phase 98 DIVERGENCE-LEDGER is the binding scope for Phase 99 SC"
  - "tool-sandbox Cluster B (#1268/#1271/#1253/#1249) is won't-sync — fork lacks tool-sandbox/ dir"
  - "Clusters E/F/G are in-scope will-sync; criterion/#1251/#1247 annotated as out-of-filter noise"
  - "PR→cluster mapping table appended to UPST11-03 body for full audit traceability (18 rows)"
metrics:
  duration: "3 minutes"
  completed: "2026-06-30"
  tasks_completed: 2
  files_modified: 2
---

# Phase 99 Plan 01: D-02 SC/REQUIREMENTS Reconciliation Summary

**One-liner:** Reconciled Phase 99 ROADMAP SC #1/#2 and REQUIREMENTS UPST11-02/03 from stale 260629-toe PR list to Phase 98 ledger reality — NetworkIntent Cluster A is the headline will-sync; tool-sandbox Cluster B is won't-sync; full PR→cluster traceability table appended.

## What Was Built

Corrected stale Phase 99 planning documents so the Phase 99 verifier (Plan 07) checks against
ledger truth rather than the preliminary 260629-toe quick-task PR enumeration.

**ROADMAP.md Phase 99 SC #1** — replaced a flat list mentioning tool-sandbox as an absorb target
with a per-cluster mapping: Cluster A full-sync-adopt (72bcfd66 NetworkIntent + d457ecc3
contradictory-flag guard), Cluster C split (cdeeb5b9/46bcfbb9/08ca19a8), Cluster D (5b8e94da),
with an explicit won't-sync sentence for tool-sandbox Cluster B.

**ROADMAP.md Phase 99 SC #2** — replaced the stale dep/CI/docs list with the three in-scope
will-sync clusters (E org-ref c808f000, F sigstore-trust-root 2e64798d, G proxy-docs a4d68189)
and annotated criterion #1232 / CI-yaml #1251 / docs #1247 as out-of-drift-filter noise to be
reconciled in Phase 100.

**REQUIREMENTS.md UPST11-02** — replaced tool-sandbox as the lead item with Cluster A
full-sync-adopt (ADR-98): 72bcfd66 (#1225 NetworkIntent) + d457ecc3 (#1263 contradictory-flag
guard); added explicit won't-sync annotation for Cluster B with rationale; kept Cluster C/D/tests
with cluster labels.

**REQUIREMENTS.md UPST11-03** — annotated criterion #1232, CI-yaml #1251, data-dir #1247 as N/A
with out-of-filter rationale; kept Clusters E/F/G as in-scope; appended a full 18-row
PR→cluster mapping table with SHAs and dispositions (IN SCOPE / WON'T-SYNC / OUT-OF-FILTER)
so no PR from the preliminary list is silently dropped.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Reconcile ROADMAP.md Phase 99 SC #1/#2 to ledger reality | eca98934 | .planning/ROADMAP.md |
| 2 | Reconcile REQUIREMENTS.md UPST11-02/UPST11-03 to ledger reality | 462a1587 | .planning/REQUIREMENTS.md |

## Verification Results

All checks passed:

| Check | Result |
|-------|--------|
| grep "NetworkIntent" in ROADMAP.md | 6 matches |
| grep "won't-sync" with tool-sandbox context in ROADMAP.md | 1 match |
| grep "72bcfd66" in REQUIREMENTS.md | 2 matches |
| grep "PR #1225" in REQUIREMENTS.md | 1 match (in mapping table) |
| grep "#1268" with WON'T-SYNC in REQUIREMENTS.md | 1 match |
| UPST11-04 text unchanged | confirmed |

## Deviations from Plan

None — plan executed exactly as written.

The PR→cluster mapping table was formatted as a markdown table (| PR #NNNN | SHA | Cluster | ... |)
rather than the plain-text `PR #NNNN SHA xxx → Cluster Y` format in the plan action prose. The
table format is more readable and satisfies the same grep acceptance criteria (grep for "PR #1225"
matches the table row `| PR #1225 | 72bcfd66 | A | IN SCOPE full-sync-adopt | NetworkIntent |`).

## Known Stubs

None. This plan produces only planning document corrections; no stub code was introduced.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes. Documentation-only
plan; no threat surface introduced.

## Self-Check: PASSED

Files verified:
- .planning/ROADMAP.md — modified, verified present
- .planning/REQUIREMENTS.md — modified, verified present

Commits verified:
- eca98934 — Task 1 (ROADMAP.md)
- 462a1587 — Task 2 (REQUIREMENTS.md)

Both commit hashes confirmed in `git log --oneline -5`.
