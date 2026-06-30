---
phase: 98-upst11-divergence-audit
plan: 01
subsystem: audit
tags: [divergence-audit, upstream-sync, ledger, upst11, networkintent, drift-tool]

requires:
  - phase: 94-upst10-divergence-audit
    provides: ledger structural template (frontmatter field set, Reproduction block, Cluster Summary, Carve-out Re-touch Check shape)
  - phase: 85-upst9-divergence-audit
    provides: SHA-not-tag guard precedent, carve-out definition source (CR-01, CR-02, Cluster F)

provides:
  - 98-DIVERGENCE-LEDGER.md with frontmatter, Reproduction block, 8-cluster scaffold (A-H), Noise reconciliation, and Carve-out Re-touch Check (6 surfaces)
  - Drift-tool JSON artifact: ci-logs-local/drift/20260629T000000Z-v0651-v0660-upst11.json (gitignored; regenerable via Reproduction block)
  - Pinned window SHAs: v0.65.1=1d1c88c9, v0.66.0=d817ed53, drift-tool=0834aa66
  - Empirical commit accounting: 14 substantive + 6 noise = 20 total (D-12 closed)
  - Carve-out re-touch results: CR-02 clean, CR-01 clean, Cluster F HIT 5, linux.rs HIT 1 (additive), endpoint-policy HIT 3, v3.2 override clean

affects: [98-upst11-divergence-audit-plan-02, 98-upst11-divergence-audit-plan-03, 99-upst11-absorb]

tech-stack:
  added: []
  patterns:
    - "SHA-not-tag guard: drift tool invoked with explicit 40-char SHAs in --from/--to (D-02 precedent from Phase 85/94)"
    - "Noise reconciliation closes at: substantive + noise = total window commits (D-12 invariant)"
    - "Carve-out re-touch check: explicit result per surface — silence is not evidence of clean"

key-files:
  created:
    - .planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md
  modified: []

key-decisions:
  - "14 substantive commits across 8 clusters (A-H fresh IDs per D-11); 6 noise commits; 20 total"
  - "#1225 NetworkIntent refactor is CLI-side ONLY (no core library changes); conflict is fork CLI vs upstream CLI — actual-diff reveals narrower scope than 260629-toe HIGH-CONFLICT preliminary scan suggested"
  - "Cluster B (4 tool-sandbox commits) won't-sync: fork lacks tool-sandbox/ dir (skipped in Phase 94/95 Cluster B); these are incremental patches to an absent feature"
  - "Cluster A windows-touch=yes confirmed: 7 of the 11 #1225-touched files have cfg(target_os = windows) blocks — cross-target clippy REQUIRED if adopted"
  - "5 Cluster F (proxy carve-out) hits in this window: Clusters A, C, E all touch proxy_runtime.rs/route.rs/reverse.rs/server.rs; Phase 89 guard tests must survive"
  - "D-15 leapfrog floor confirmed: fork bumps to 0.66.1 (not 0.67) — collision-free above upstream 0.66.0; Cluster H won't-sync records this for Phase 100"

patterns-established:
  - "UPST11 ledger follows Phase 94/85 structural template exactly (field names, section order, Carve-out Re-touch Check format)"

requirements-completed: [UPST11-01]

duration: 25min
completed: 2026-06-29
---

# Phase 98 Plan 01: UPST11 Divergence Audit Foundation Summary

**UPST11 window empirically grounded: 14 substantive commits across 8 clusters, drift-tool SHA-pinned to 0834aa66, noise reconciliation closed (14+6=20), and 6-surface carve-out re-touch check completed — Plan 02 actual-diff inspection and Plan 03 NetworkIntent ADR can proceed**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-29T00:00:00Z (approximate)
- **Completed:** 2026-06-29
- **Tasks:** 2 (Tasks 1 and 2 executed in single write pass)
- **Files created:** 1 (98-DIVERGENCE-LEDGER.md) + 1 gitignored artifact (drift JSON)

## Accomplishments

- Fetched upstream (`nolabs-ai/nono`) with tags; resolved v0.65.1 → 1d1c88c9, v0.66.0 → d817ed53; verified both as commit objects
- Ran drift tool against explicit window SHAs (D-02 SHA-not-tag guard satisfied): 14 substantive commits reported; by_category: profile 3, proxy 4, other 13
- Full commit accounting: 20 total, 0 merges, 20 non-merges; 14 substantive + 6 noise = 20 (D-12 closed)
- Created 98-DIVERGENCE-LEDGER.md with all required sections: frontmatter (10 Phase-94 field names), Reproduction block, Cluster Summary scaffold (8 clusters A-H), Excluded as Noise (6 commits), Carve-out Re-touch Check (6 surfaces), Empirical Cross-Check (6 spot-checks)

## Task Commits

Tasks 1 and 2 were executed in a single write pass (both tasks produce content in the same ledger file; combined into one atomic commit):

1. **Task 1 + Task 2: Divergence ledger, reproduction block, noise reconciliation, cluster scaffold** - `bfb008bc` (docs)

**Plan metadata:** (this commit, created below)

## Files Created/Modified

- `.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` - Full UPST11 divergence ledger (359 lines): frontmatter, Reproduction block, 8-cluster scaffold, noise reconciliation, carve-out re-touch check, empirical cross-check, ADR review first-pass
- `ci-logs-local/drift/20260629T000000Z-v0651-v0660-upst11.json` - Drift tool JSON output for v0.65.1..v0.66.0 (gitignored; regenerable via ledger Reproduction block)

## Decisions Made

- Tasks 1 and 2 executed together in one write pass (both target the same ledger file); documented as single commit `bfb008bc`
- #1225 NetworkIntent refactor scope: actual-diff shows CLI-only (11 files in nono-cli/src/, NO core library changes); the 260629-toe "HIGH-CONFLICT" designation remains valid because the fork's CLI diverges from upstream's CLI, but the scope is narrower than initially suggested
- Cluster B (4 tool-sandbox commits) all touch `crates/nono-cli/src/tool-sandbox/` which is absent in the fork (Phase 94/95 skipped this feature) → won't-sync disposition for the cluster
- Cluster A windows-touch=yes: grep confirms 7 of 11 #1225-touched files have `cfg(target_os = "windows")` blocks — cross-target clippy is REQUIRED for Phase 99 if Cluster A is adopted
- 6 noise commits confirmed: 1 test-files-only (#1213), 3 CI-yaml-only (#1259/#1251/#1245), 1 data-dir-docs (#1247), 1 non-core-dep-bump (#1232); all correctly outside the drift tool's path filter

## Deviations from Plan

None — plan executed as specified. The two tasks were combined into a single ledger write (not a rule-deviation; both tasks target the same file and the write was done as one atomic operation). All acceptance criteria for both tasks verified via automated checks before commit.

## Issues Encountered

None. Upstream fetch, SHA resolution, drift tool run, and carve-out checks all succeeded on first attempt.

## User Setup Required

None — this is a docs-only audit plan. No external service configuration required.

## Next Phase Readiness

- **Plan 02 (cluster actual-diff inspection):** Can begin immediately from the populated ledger. Inputs: cluster scaffold (8 clusters A-H), carve-out check results (Cluster F HIT 5 commits, linux.rs additive, endpoint-policy HIT 3), explicit window SHAs pinned in Reproduction block
- **Plan 03 (NetworkIntent ADR-98):** Can begin after Plan 02 confirms the actual-diff shape of `72bcfd66` (#1225). The ADR needs: (a) actual PR diff already confirmed CLI-only, (b) Plan 02's inspection of which fork files conflict, (c) the adopt vs carve-out recommendation
- **Plan 04 (carve-out re-touch):** All 6 surface checks completed in this plan (D-07 satisfied at ledger level); Plan 04 does deeper inspect on the hits

**Blockers/Concerns carrying into Plan 02:**
- Cluster A (NetworkIntent) windows-touch=yes: Phase 99 absorb MUST run cross-target clippy if adopted
- Cluster C (#983 HTTP/2) is a large multi-file commit — tls_intercept/ split work is non-trivial
- Cluster F (5 proxy carve-out hits): all must be split-inspected individually in Plan 02

## Self-Check: PASSED

```
LEDGER file:       .planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md → FOUND
## Reproduction:   grep -q '## Reproduction' → FOUND
drift_tool_invocation: grep -q 'drift_tool_invocation' → FOUND
DRIFT JSON:        ci-logs-local/drift/20260629T000000Z-v0651-v0660-upst11.json → FOUND (gitignored, local only)
## Excluded as Noise: grep -q '## Excluded as Noise' → FOUND
## Cluster Summary: grep -q '## Cluster Summary' → FOUND
won't-sync:        grep -q "won't-sync" → FOUND
completeness:      grep -qi 'completeness' → FOUND
Task commit:       bfb008bc → FOUND (git log --oneline -1)
```

---
*Phase: 98-upst11-divergence-audit*
*Completed: 2026-06-29*
