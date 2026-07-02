---
phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker
plan: 04
subsystem: infra
tags: [release-engineering, powershell, verify-dark, cargo-publish, maturin, npm, release-runbook]

# Dependency graph
requires:
  - phase: 100-01
    provides: workspace + Cargo.lock leapfrogged to 0.66.1 across all 6 version-family crates
  - phase: 100-02
    provides: reconciled release.yml CI pipeline + ADR-100 (publish-crates idempotency, cross-compile trigger)
  - phase: 100-03
    provides: nono-py + nono-ts bumped to 0.66.1; nono-py RouteConfig/ProxyConfig PyPI blocker closed
provides:
  - release-readiness gate re-pointed and re-verified GREEN at 0.66.1 (targetVersion, upstreamHighest, assertion-e Cargo.lock check)
  - release-dry-run.ps1 re-verified exit 0 at 0.66.1 (crates.nono PASS; pypi.maturin_build now PASS; npm.dry_run PASS)
  - RELEASE-RUNBOOK.md fully updated for the 0.66.1 tag, PyPI blocker marked resolved, PUBLIC-repo checklist accurate
affects: [future-release-phases, operator-push-workflow]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "release-readiness gate assertions reference $targetVersion by variable, not a hardcoded literal, so future re-greens at a new version don't silently false-fail"
    - "twine-availability probe checks $LASTEXITCODE, not output text — avoids false-positive matches on error messages that happen to contain the tool name"

key-files:
  created: []
  modified:
    - scripts/gates/release-readiness.ps1
    - scripts/release-dry-run.ps1
    - .planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md

key-decisions:
  - "Assertion (e) in release-readiness.ps1 was rewritten to reference $targetVersion instead of a hardcoded '0.66.0' literal — Cargo.lock now contains zero 0.66.0 occurrences (all 6 crates show 0.66.1), so the old hardcoded check would have false-FAILed the gate at the very version bump this plan exists to re-green"
  - "twine-detection fallback (python -m twine --version) was matching on output text containing the substring 'twine', which false-positived on Python's own 'No module named twine' error message; switched to checking $LASTEXITCODE"
  - "RELEASE-RUNBOOK.md pre-push checklist collapsed the duplicate 'upstream highest' version restatement into a single canonical mention (the gate table's leapfrog row) to satisfy the plan's grep-count=1 acceptance criterion while keeping the document DRY"
  - "PUBLIC-repo checklist language corrected from 'pending Microsoft minifilter-altitude approval' to reflect the actual 2026-07-01 operator decision: approval was received AND go-private was cancelled permanently (not merely deferred)"

patterns-established:
  - "Release gate constants must be variable-referenced end-to-end (not partially hardcoded) so a version bump is a two-line diff, not a hunt through every assertion"

requirements-completed: [RLS-13]

# Metrics
duration: 15min
completed: 2026-07-02
---

# Phase 100 Plan 04: Release-Readiness Gate Re-Green + RELEASE-RUNBOOK.md 0.66.1 Update Summary

**Re-pointed the release-readiness gate's two hard-coded version constants to the 0.66.1 leapfrog, fixed a hardcoded-literal bug in assertion (e) and a false-positive twine-detection bug in release-dry-run.ps1 that the version bump exposed, and brought RELEASE-RUNBOOK.md fully current for the 0.66.1 tag — closing RLS-13 and making the release genuinely one-step-push ready.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-07-02T15:33:20Z (STATE.md session handoff from Plan 03)
- **Completed:** 2026-07-02T15:44:55Z
- **Tasks:** 3 completed
- **Files modified:** 3

## Accomplishments
- `release-readiness` gate re-run GREEN at 0.66.1 via `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (exit 0, verdict PASS, all 5 assertions green)
- `release-dry-run.ps1` re-run GREEN (exit 0): `crates.nono` PASS, `crates.nono-proxy`/`crates.nono-cli` PRE_PUBLISH_REGISTRY_BLOCKED (expected), `pypi.maturin_build` PASS (blocker closed by Plan 03), `pypi.twine_check` SKIP (toolchain absent, correctly detected), `npm.dry_run` PASS
- RELEASE-RUNBOOK.md fully updated for the 0.66.1 tag: title, checklist, gate table, git tag in push sequence, "What release.yml Does" section, Known Pre-Release Blockers table (row 1 resolved), closing Reminder summary; only the intentional "upstream highest 0.66.0" comparison remains (`grep -c '0.66.0'` = 1)

## Task Commits

Each task was committed atomically:

1. **Task 1: Re-point release-readiness gate constants and confirm PASS at 0.66.1** - `7e67d9db` (fix)
2. **Task 2: Re-run release-dry-run.ps1 and confirm exit 0 at 0.66.1** - `34adcb12` (fix)
3. **Task 3: Update RELEASE-RUNBOOK.md for the 0.66.1 tag** - `c827340b` (docs)

## Files Created/Modified
- `scripts/gates/release-readiness.ps1` - `$targetVersion`/`$upstreamHighest` re-pointed to 0.66.1/0.66.0; assertion (e) rewritten to reference `$targetVersion` instead of a hardcoded literal
- `scripts/release-dry-run.ps1` - header docstring + inline prose updated to 0.66.1; twine-availability detection fixed to check `$LASTEXITCODE` instead of output text
- `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md` - every 0.66.0 reference updated to 0.66.1 except the intentional leapfrog-comparison row; PyPI blocker marked resolved; PUBLIC-repo checklist language corrected for the settled altitude-approval decision

## Decisions Made
- Assertion (e)'s hardcoded `'0.66.0'` literal was rewritten to reference the `$targetVersion` variable (see Deviations below) — this is now the durable pattern for future version bumps.
- Collapsed the RELEASE-RUNBOOK.md pre-push checklist's duplicate "upstream highest" mention into a single canonical row (the gate table) to hit the plan's literal `grep -c '0.66.0'` == 1 acceptance criterion without losing information — the checklist item now points the operator at the gate table instead of restating the number.
- PUBLIC-repo checklist language corrected to state the repo stays PUBLIC permanently (operator decision 2026-07-01, minifilter-altitude approval received, go-private cancelled) rather than "pending approval" — matches STATE.md Blockers/Concerns.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] release-readiness.ps1 assertion (e) hardcoded a stale literal instead of `$targetVersion`**
- **Found during:** Task 1, immediately after editing the two constants and before running the verify command
- **Issue:** Assertion (e) checked `$lockContent -notmatch '0\.66\.0'` using a hardcoded literal string rather than referencing the `$targetVersion` variable used by assertions (a) and (c). After Plan 01's version bump, Cargo.lock contains zero `0.66.0` occurrences (confirmed via `grep -c '0\.66\.0' Cargo.lock` returning 0; all 6 workspace crates now show `0.66.1`). Left as-written, bumping `$targetVersion` to `0.66.1` in this task would have caused assertion (e) to FAIL the gate — directly contradicting the plan's stated truth ("release-readiness gate PASSes with $targetVersion=0.66.1").
- **Fix:** Rewrote the assertion to use `[regex]::Escape($targetVersion)` in both the match and the count, and updated the corresponding `$detail['cargo_lock']` messages to interpolate `$targetVersion` instead of the literal string. Also refreshed two stale prose comments (assertion a and c headers) referencing the old version numbers.
- **Files modified:** `scripts/gates/release-readiness.ps1`
- **Verification:** `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` exits 0, verdict PASS, `detail.cargo_lock` reports `"0.66.1 found (6 occurrence(s)) in Cargo.lock"`.
- **Committed in:** `7e67d9db` (Task 1 commit)

**2. [Rule 1 - Bug] release-dry-run.ps1 twine-detection false-positived on an error message**
- **Found during:** Task 2, first re-run of `release-dry-run.ps1` after the docstring update — `pypi.twine_check` reported `FAIL: twine check exited 1` with output `No module named twine`
- **Issue:** The fallback twine-detection path ran `python -m twine --version 2>&1 | Out-String` and checked `$pyTwineCheck -match "twine"` to decide whether twine was usable. When the `twine` Python module is absent, `python -m twine --version` still prints `"...python.exe: No module named twine"` to stderr — which itself contains the substring `"twine"`, so the plain-text match incorrectly reported twine as available. The script then attempted `python -m twine check ...`, which failed for real, producing a hard FAIL where the correct outcome is `SKIP_HOST_UNAVAILABLE` (consistent with the CLI-absent branch and the verify-dark SKIP convention documented in the script's own `.DESCRIPTION`). This directly violated the plan's Task 2 acceptance criterion "No result key shows FAIL".
- **Fix:** Replaced the text-match check with an exit-code check: capture `$LASTEXITCODE` after the `python -m twine --version` invocation and treat `0` as available, non-zero as absent. Added an inline comment documenting the false-positive hazard for future maintainers.
- **Files modified:** `scripts/release-dry-run.ps1`
- **Verification:** Confirmed `python -m twine --version` exits 1 with the misleading message on this host; re-ran `pwsh -File scripts/release-dry-run.ps1` after the fix — now reports `SKIP [SKIP_HOST_UNAVAILABLE]: twine: command not found` and the overall run exits 0 with zero FAIL keys (`PASS: Hard failures: 0. Blocked: 2 (pre-publish). Skipped: 1 (toolchain absent).`).
- **Committed in:** `34adcb12` (Task 2 commit)

**3. [Rule 1 - Bug] RELEASE-RUNBOOK.md stale "pending approval" / "fix blocker first" phrasing**
- **Found during:** Task 3, full-file review against STATE.md Blockers/Concerns and Phase 100 Plan 03's SUMMARY
- **Issue:** Two spots in the runbook described settled facts as still-open: (a) the PUBLIC-repo pre-push checklist and closing Reminder both said the repo was "PUBLIC pending Microsoft minifilter-altitude approval," but STATE.md records the altitude approval was RECEIVED 2026-07-01 and the operator subsequently decided NOT to go private (retired, not deferred); (b) Step 5's PyPI instructions said "Fix the RouteConfig blocker in nono-py first," but Plan 03 already closed that blocker.
- **Fix:** Reworded both the checklist bullet and the closing Reminder to state the repo stays PUBLIC permanently by operator decision, and updated Step 5 to note the blocker is already resolved (citing Plan 03) rather than instructing the operator to fix it.
- **Files modified:** `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md`
- **Verification:** Manual re-read; no remaining "pending approval" or "fix ... first" phrasing that contradicts STATE.md's current Blockers/Concerns section.
- **Committed in:** `c827340b` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 — bugs uncovered by re-running the gate/dry-run at the new version)
**Impact on plan:** All three fixes were necessary for the plan's own acceptance criteria to be satisfiable (gate PASS, dry-run zero-FAIL, accurate runbook); no scope creep — each fix stayed within the file already being edited by its task.

## Issues Encountered
None beyond the three auto-fixed deviations above — no unresolved issues.

## User Setup Required
None - no external service configuration required. This plan runs no live push or publish command; the actual tag push and registry publish remain operator-gated per the plan's threat model and D-09.

## Next Phase Readiness
- Both mandatory pre-push gates (`release-readiness` + `release-dry-run.ps1`) are GREEN at 0.66.1 on this tree.
- RELEASE-RUNBOOK.md is accurate and ready to hand to the operator for the actual push sequence (Steps 3-6), which remain manual/outside this milestone's scope.
- Remaining non-blocking item: twine is absent on this dev host (`pip install twine` or `uv add twine` in the nono-py dev env) — moot for the dry-run gate (correctly SKIPped) but needed before a live `twine upload` in Step 5.
- Phase 100 plans 01-04 are now all complete; STATE.md/ROADMAP.md updates follow this SUMMARY per the executor's shared-artifact ownership.

## Self-Check: PASSED

All 3 modified files confirmed present on disk; all 3 task commit hashes (`7e67d9db`, `34adcb12`, `c827340b`) confirmed present in `git log --oneline --all`.

---
*Phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker*
*Completed: 2026-07-02*
