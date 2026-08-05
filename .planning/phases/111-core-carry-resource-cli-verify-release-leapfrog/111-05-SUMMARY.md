---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
plan: 05
subsystem: infra
tags: [cargo, release-engineering, version-bump, powershell, gates]

# Dependency graph
requires:
  - phase: 111-04
    provides: "Combined 108-111 fork-invariant verify pass (both cross-target clippy gates GREEN, D-01/ADR-86 unregressed, full test-suite baseline diffed) confirming the tree is safe to leapfrog"
provides:
  - "All 6 in-repo version-family crates (nono-sandbox, nono-sandbox-cli, nono-sandbox-proxy, nono-shell-broker, nono-fltmgr-client, nono-ffi) at 0.70.0"
  - "6 internal path-dependency version pins bumped in lockstep with their depended-on crates"
  - "Regenerated Cargo.lock with drift limited to the 6 bumped packages"
  - "release-readiness.ps1 and release-dry-run.ps1 asserting/referencing 0.70.0/0.69.0 instead of stale 0.66.1/0.66.0"
affects: [111-06]

# Tech tracking
tech-stack:
  added: []
  patterns: ["prepare-only version leapfrog (bump + gate-script correction in the same wave, never split across phases)"]

key-files:
  created: []
  modified:
    - crates/nono/Cargo.toml
    - crates/nono-cli/Cargo.toml
    - crates/nono-proxy/Cargo.toml
    - crates/nono-shell-broker/Cargo.toml
    - bindings/c/Cargo.toml
    - crates/nono-fltmgr-client/Cargo.toml
    - Cargo.lock
    - scripts/gates/release-readiness.ps1
    - scripts/release-dry-run.ps1

key-decisions:
  - "Left tools/sign-fixture untouched at its independent 0.1.0 versioning scheme, per plan and RESEARCH.md's confirmed 7th-workspace-member exclusion."
  - "Also corrected 2 stale inline comments in release-readiness.ps1 (lines describing ASSERTION (a) and (c)) that still referenced 0.66.1/0.66.0 even though the plan's acceptance criteria only named the two live variable lines — same accuracy standard the plan itself applies to release-dry-run.ps1's cosmetic references."

patterns-established: []

requirements-completed: [RLS-14]

# Metrics
duration: 12min
completed: 2026-08-05
---

# Phase 111 Plan 05: In-Repo 0.70.0 Version Leapfrog Summary

**Bumped all 6 fork-owned workspace crates + their internal path-dep pins from 0.66.1 to 0.70.0, regenerated Cargo.lock with zero third-party drift, and corrected both hardcoded-version release-gate scripts so release-readiness asserts against the real target instead of passing vacuously against a stale one.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-08-04T22:08:00Z
- **Completed:** 2026-08-05T02:20:10Z
- **Tasks:** 2 completed
- **Files modified:** 9

## Accomplishments
- All 6 version-family crates (`nono-sandbox`, `nono-sandbox-cli`, `nono-sandbox-proxy`, `nono-shell-broker`, `nono-fltmgr-client`, `nono-ffi`) confirmed at `0.70.0` via `cargo metadata`; `sign-fixture` confirmed unchanged at `0.1.0`.
- `cargo build --workspace --all-targets` exits 0 against the bumped tree; `git diff Cargo.lock` touches only the 6 bumped package version fields, no incidental dependency upgrades.
- `scripts/gates/release-readiness.ps1`'s `$targetVersion`/`$upstreamHighest` now read `0.70.0`/`0.69.0`; `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` returns verdict `PASS` with `version_family` = "0.70.0 confirmed for all 6 version-family crates" and `leapfrog` = "0.70.0 > 0.69.0 (leapfrog confirmed)".
- `scripts/release-dry-run.ps1`'s 4 cosmetic `0.66.1`/`0.66.0` references (synopsis, description, inline comment, status message) all updated to `0.70.0`; `grep -c "0.66.1"` on the file returns 0.
- D-07 upheld: no `git tag`, no `git push --tags`, no live `cargo publish` executed at any point.

## Task Commits

Each task was committed atomically:

1. **Task 1: Bump the 6 in-repo crate versions + path-dep pins + regenerate Cargo.lock** - `502403ba` (feat)
2. **Task 2: Correct the two hardcoded-version gate scripts + confirm release-readiness GREEN** - `d1c0ad58` (fix)

_No plan-metadata commit yet — this SUMMARY.md + STATE.md/ROADMAP.md update lands in the final commit._

## Files Created/Modified
- `crates/nono/Cargo.toml` - `[package] version` 0.66.1 → 0.70.0
- `crates/nono-cli/Cargo.toml` - `[package] version` + 3 path-dep version pins (`nono`, `nono-proxy`, `nono-shell-broker`) 0.66.1 → 0.70.0
- `crates/nono-proxy/Cargo.toml` - `[package] version` + `nono` path-dep pin 0.66.1 → 0.70.0
- `crates/nono-shell-broker/Cargo.toml` - `[package] version` + `nono` path-dep pin 0.66.1 → 0.70.0
- `bindings/c/Cargo.toml` - `[package] version` + `nono` path-dep pin 0.66.1 → 0.70.0
- `crates/nono-fltmgr-client/Cargo.toml` - `[package] version` 0.66.1 → 0.70.0 (no path-dep on `nono`)
- `Cargo.lock` - regenerated; 6 package version-field changes, zero third-party drift
- `scripts/gates/release-readiness.ps1` - `$targetVersion`/`$upstreamHighest` → `0.70.0`/`0.69.0`; 2 stale inline comments corrected
- `scripts/release-dry-run.ps1` - 4 cosmetic `0.66.1`/`0.66.0` references corrected to `0.70.0`

## Decisions Made
- `tools/sign-fixture` deliberately left untouched (independent `0.1.0` CI-tooling versioning, confirmed via `Read` before any edit).
- Extended Task 2's edit scope by 2 lines beyond the plan's literal action text (the two `ASSERTION (a)`/`ASSERTION (c)` inline comments in `release-readiness.ps1` that also spelled out `0.66.1`/`0.66.0`) — same "not left stale" standard the plan already applies to `release-dry-run.ps1`'s 4 cosmetic references; classified as Rule 1 (accuracy/bug fix), not a deviation requiring approval.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected 2 additional stale-version inline comments in release-readiness.ps1 not named in the plan's literal action text**
- **Found during:** Task 2
- **Issue:** Lines 91 and 166 of `scripts/gates/release-readiness.ps1` carry inline comments (`# ASSERTION (a): cargo metadata reports all version-family crates at 0.66.1` and `# ASSERTION (c): leapfrog — 0.66.1 is strictly greater than upstream 0.66.0`) that still referenced the old version even after the live `$targetVersion`/`$upstreamHighest` variables were corrected. The plan's action text named only lines 76-77; these two comments were discovered via a broader `grep -n "0.66"` sweep run to confirm no stale references remained anywhere in the file.
- **Fix:** Updated both comments to read `0.70.0`/`0.69.0`, matching the corrected assertion logic they describe.
- **Files modified:** `scripts/gates/release-readiness.ps1`
- **Verification:** `grep -c "0.66" scripts/gates/release-readiness.ps1` returns 0; `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` still returns `PASS` after the edit.
- **Committed in:** `d1c0ad58` (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 - accuracy/bug)
**Impact on plan:** Purely cosmetic comment correction co-located with the plan's own mandated edit to the same file. No scope creep — matches the plan's own "not left stale" standard applied elsewhere in the same task.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- This repo is fully at `0.70.0`; `release-readiness.ps1` correctly asserts against it.
- Plan 111-06 (Wave 4) is unblocked: it bumps the sibling binding repos `../nono-py` and `../nono-ts` to `0.70.0`, deliberately sequenced after this plan because `../nono-ts`'s `Cargo.toml` pins an explicit `version = "0.66"` requirement on its path-dependency to this repo's `nono` crate, which would stop resolving once this repo's crate reports `0.70.0` — that pin is corrected in 111-06, not here.
- No blockers. D-07 prepare-only posture intact: no tag pushed, no live publish executed by this plan.

---
*Phase: 111-core-carry-resource-cli-verify-release-leapfrog*
*Completed: 2026-08-05*

## Self-Check: PASSED

All 9 modified files confirmed present on disk. All 3 commits (`502403ba`, `d1c0ad58`, `29729b46`) confirmed present in `git log --oneline --all`.
