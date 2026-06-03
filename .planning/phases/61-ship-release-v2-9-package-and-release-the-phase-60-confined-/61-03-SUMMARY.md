---
phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined
plan: 03
subsystem: infra
tags: [release, signing, git-ancestry, github-release, ci-pipeline]

requires:
  - phase: 61-01
    provides: version bump to 0.58.0 and CHANGELOG [0.58.0] section
  - phase: 61-02
    provides: D-09 verification record for todo drain

provides:
  - "61-PRETAG-READINESS.md: go/no-go gate for v0.58.0 Wave 3 tag push"
  - "D-08 ancestry: all 4 untagged v2.7 drain-fix commits confirmed present in release tree"
  - "D-07 clearance: v0.57.4 unsigned-payload release confirmed absent"
  - "D-02 pre-flight: release.yml fail-closed signing guard confirmed; operator sign-off required"
  - "Driver-sys gate: nono-wfp-driver.sys presence confirmed for release.yml Package step"

affects: [61-04-tag-push]

tech-stack:
  added: []
  patterns:
    - "Pre-tag readiness gate: ancestry check + release hygiene + signing pre-flight before any tag push"

key-files:
  created:
    - .planning/phases/61-ship-release-v2-9-package-and-release-the-phase-60-confined-/61-PRETAG-READINESS.md
    - .planning/phases/61-ship-release-v2-9-package-and-release-the-phase-60-confined-/.gitkeep
  modified: []

key-decisions:
  - "D-08 verified: d8b7ce00, 005b4c9e, 0cbeb3be, b852826b are all ancestors of HEAD (main)"
  - "D-07 verified: v0.57.4 is absent from GitHub releases; no deletion action required"
  - "D-02 pre-flight: release.yml fail-closed signing guard at line 124 confirmed; both secret NAMES present; operator must confirm secret VALUES before tag push"
  - "READY TO TAG: YES — all automated checks green; pending operator secret-value confirmation"

patterns-established:
  - "Pre-tag readiness record pattern: run ancestry/release/signing checks before any tag that triggers CI release"

requirements-completed: [REQ-RLS-03]

duration: 15min
completed: 2026-06-03
---

# Phase 61 Plan 03: Pre-Tag Readiness Gate Summary

**v0.58.0 pre-tag readiness record written: all 4 drain-fix ancestry checks green, v0.57.4 unsigned-payload release confirmed absent, release.yml fail-closed signing guard confirmed, nono-wfp-driver.sys present.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-06-03T17:00:00Z
- **Completed:** 2026-06-03T17:15:00Z
- **Tasks:** 2 (verification + readiness record)
- **Files modified:** 1 created + 1 gitkeep

## Accomplishments

- All 4 untagged v2.7 drain-fix commits (`d8b7ce00`, `005b4c9e`, `0cbeb3be`, `b852826b`) confirmed as ancestors of HEAD via `git merge-base --is-ancestor`
- v0.57.4 superseded (unsigned-payload MSI) GitHub release confirmed absent — no distribution hazard; only v0.57.5 is public
- release.yml "Check signing secrets (Windows)" fail-closed guard confirmed at line 124; both `WINDOWS_SIGNING_CERT` + `WINDOWS_SIGNING_CERT_PASSWORD` secret NAMES confirmed present via `gh secret list`
- `crates/nono-cli/data/windows/nono-wfp-driver.sys` presence confirmed; release.yml Package gate at line 193 will not fail closed
- Created `61-PRETAG-READINESS.md` (124 lines) with structured go/no-go checklist — **READY TO TAG: YES**

## Task Commits

Both tasks committed atomically in a single commit (Task 1 was verification-only, no files):

1. **Tasks 1+2: Verify drain-fix ancestry / write pre-tag readiness record** - `e5df94c9` (feat)

**Plan metadata:** (this summary commit)

## Files Created/Modified

- `.planning/phases/61-ship-release-v2-9-package-and-release-the-phase-60-confined-/61-PRETAG-READINESS.md` - Pre-tag readiness record with D-08/D-07/D-02/driver-sys checks and READY-TO-TAG line
- `.planning/phases/61-ship-release-v2-9-package-and-release-the-phase-60-confined-/.gitkeep` - Phase directory marker

## Decisions Made

All four checks resolved to GREEN:
- D-08: ancestry checks use `git merge-base --is-ancestor` (exact, not string comparison)
- D-07: `gh release view v0.57.4` returns "release not found"; only v0.57.5 is Latest
- D-02: fail-closed guard exists AND secret names are configured; operator must confirm values
- Driver-sys: file is present in the checked-in tree

## Deviations from Plan

None — plan executed exactly as written. The RESEARCH-validated expectation (v0.57.4 absent, all 4 SHAs ancestral) was confirmed exactly. The `gh secret list` corroboration step found both secret names present, matching the expected state from Phase 53.

## Issues Encountered

None. All automated checks ran cleanly via the Bash tool. The `gh secret list` call required specifying `-R oscarmackjr-twg/nono` due to the "multiple remotes detected" environment — handled inline.

## User Setup Required

**Operator action required before the Wave 3 tag push (61-04):**
Confirm that `WINDOWS_SIGNING_CERT` and `WINDOWS_SIGNING_CERT_PASSWORD` repository secrets
contain valid production signing material (NOT the POC cert `319E507E...`).
The CLI cannot read secret values — this is a manual operator confirmation step.
The fail-closed guard will surface missing/invalid secrets as a loud CI failure, but
confirming before the push avoids a wasted release.yml run.

## Next Phase Readiness

- **61-04 (tag push):** 61-PRETAG-READINESS.md shows READY TO TAG: YES. Operator must confirm signing secret values, then push `v0.58.0` and `v2.9` tags to trigger release.yml.
- No blockers. All four readiness gates are green.

---
*Phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined-*
*Completed: 2026-06-03*
