---
phase: 104-smoke-green-cut-the-trusted-signed-release
plan: 01
subsystem: infra
tags: [powershell, authenticode, code-signing, x509, ci-hardening, verify-gate]

# Dependency graph
requires:
  - phase: 101-verify-gate-hardening-azure-profile-confirmation
    provides: scripts/verify-authenticode.ps1 (Assert-TrustedSignature, dual-engine GAS+signtool verify, D-01/D-02/D-03/D-04 invariants, existing test harness)
provides:
  - D-04 diagnostic-flush fix (Write-ChainDiagnostic now reliably runs before the terminating throw under CI's forced $ErrorActionPreference='Stop')
  - Merge-NoCheckOverride: pure, strictly one-directional NoCheck-mode corroboration that prevents a genuine untrusted root from being misclassified as a retry-able transient
  - Two new regression-guard test cases (diagnosticFlushUnderStop, noCheckOverridesTransient), both manually proven via revert-then-restore to actually catch their respective pre-fix bugs
affects: [104-02-publish-crates-neutralization, 104-03-poll-until-green-release-cut, phase-105-multi-registry-publish]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "NoCheck-mode X509Chain as diagnostic-only corroboration alongside an unchanged Online-mode operational chain build — never a new pass condition, only ever narrows retry eligibility toward fail-closed"
    - "-ErrorAction Continue on Write-Error inside any shared PowerShell helper dot-sourced into a `shell: pwsh` GitHub Actions step, so the helper's own diagnostic/cleanup lines are guaranteed to run regardless of the caller's ambient $ErrorActionPreference"

key-files:
  created: []
  modified:
    - scripts/verify-authenticode.ps1
    - scripts/tests/test_verify_authenticode.ps1

key-decisions:
  - "Merge-NoCheckOverride is a separate, directly-unit-testable pure function (not inlined into Get-ChainClassification) so its one-directional invariant (never upgrades a fail to a pass) is independently verifiable"
  - "The NoCheck chain is attached to the classification object as NoCheckChain for future diagnostic use, without requiring Write-ChainDiagnostic to consume it in this plan (deferred, explicitly in-scope-as-attached-only per plan action)"

patterns-established:
  - "Manual FAIL-then-PASS regression-proof recorded in SUMMARY as an explicit substitute for RED-then-GREEN when a plan's test case is added after its corresponding fix (not before)"

requirements-completed: [REL-01]

# Metrics
duration: 3min
completed: 2026-07-03
---

# Phase 104 Plan 01: D-04 Diagnostic-Flush Fix + NoCheck-Mode Untrusted-Root Classification Summary

**Fixed a silent CI diagnostic-swallowing bug (Write-Error terminating before Write-ChainDiagnostic under GitHub Actions' forced `$ErrorActionPreference='Stop'`) and added a NoCheck-mode X509Chain corroboration so a genuine untrusted root is never misclassified as a retry-able transient — both changes are strictly additive and leave the fail-closed `Status -ne 'Valid'` / signtool-exit-code gates byte-for-byte unchanged.**

## Performance

- **Duration:** ~3 min (task execution only; excludes upstream research/planning)
- **Started:** 2026-07-03T17:27:00-04:00 (approx.)
- **Completed:** 2026-07-03T17:30:48-04:00
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- `Assert-TrustedSignature`'s two Strict-mode `Write-Error` calls now carry `-ErrorAction Continue`, guaranteeing `Write-ChainDiagnostic`'s full chain dump and signtool `/pa` output reach the CI log before the terminating `throw`, regardless of the caller's ambient `$ErrorActionPreference`.
- Added `Merge-NoCheckOverride`, a pure, strictly one-directional function that can only move a classification toward `IsUntrustedRoot=$true`/`IsTransient=$false` — never toward a pass — and wired it into `Get-ChainClassification` via a new NoCheck-mode corroborating `X509Chain` build alongside the existing (unchanged) Online-mode build.
- Added `Test-DiagnosticFlushUnderStop` and `Test-NoCheckOverridesTransient` regression-guard cases; both were manually proven (not merely asserted) to catch their respective pre-fix bugs via a revert-then-restore check.
- Full harness now green at 6/6 cases (`pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All` exits 0).
- All grep-checked acceptance criteria confirmed: `Status -ne 'Valid'` and `ExitCode -ne 0` gate conditions appear exactly once each and are byte-for-byte unchanged; `-ErrorAction Continue` appears exactly twice; `RevocationMode]::NoCheck` and `RevocationMode]::Online` each appear exactly once.

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix D-04 diagnostic-flush gap + add NoCheck-mode untrusted-root classification** - `30f97268` (fix)
2. **Task 2: Add regression-guard test cases + full harness re-run** - `8dcb5c10` (test)

**Plan metadata:** (this commit, following SUMMARY.md creation)

_Note: this plan's task ordering is fix-then-test (production fix committed first, regression-guard tests added second with a manual revert-then-restore proof substituting for a literal RED-before-GREEN git-commit ordering) — this is the exact sequence specified by the plan's own `<action>` blocks, not a deviation._

## Files Created/Modified
- `scripts/verify-authenticode.ps1` - Added `-ErrorAction Continue` to the two Strict-mode `Write-Error` calls (D-04 flush fix); added `Merge-NoCheckOverride`; extended `Get-ChainClassification` with a NoCheck-mode corroborating chain build wired through `Merge-NoCheckOverride`
- `scripts/tests/test_verify_authenticode.ps1` - Added `Test-DiagnosticFlushUnderStop` and `Test-NoCheckOverridesTransient` cases, wired into `-Case` `ValidateSet` and the `'All'` dispatch branch (now 6 total cases)

## Decisions Made
- None beyond what the plan specified — implemented exactly as directed by `104-01-PLAN.md`'s `<action>` blocks, which themselves encode the empirically-verified diffs from `104-RESEARCH.md` Pattern 1 and Pattern 2.

## Deviations from Plan

None - plan executed exactly as written. All acceptance criteria (grep counts, line-number ordering, dot-source parse check, harness pass counts) were verified to match exactly as specified.

## Regression-Proof Record (manual FAIL-then-PASS verification)

Performed this session per the plan's Task 2 `<action>`:

1. Temporarily removed `-ErrorAction Continue` from the `Write-Error` call inside the `if ($gas.Status -ne 'Valid')` branch (the GAS-status branch — the only branch `diagnosticFlushUnderStop` exercises, since an unsigned fixture fails GAS status before ever reaching the signtool-exit-code branch).
2. Re-ran `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case diagnosticFlushUnderStop`:
   ```
   FAIL: diagnosticFlushUnderStop: diagnosticFlushUnderStop: expected Write-ChainDiagnostic's
   marker line to appear in captured output before the throw, but it was not found
   (D-04 flush gap regression)
   EXIT:1
   ```
   Confirmed FAIL — the new test would have caught the pre-fix bug.
3. Restored `-ErrorAction Continue` exactly as Task 1 left it.
4. Re-ran `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All`: all 6 cases PASS, exit 0.
5. Confirmed via `git diff scripts/verify-authenticode.ps1` (empty output) that the file is byte-identical to its Task 1 committed state after the revert-then-restore — the manual proof left no stray changes.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `scripts/verify-authenticode.ps1` is hardened and ready for the next real Trusted Signing verify attempt (smoke or release) to be fully legible in the CI log on the first attempt.
- Neither fix resolves the external, Microsoft-side root-certificate propagation gap identified in `104-RESEARCH.md` (that remains genuinely out of this repo's control) — these fixes only ensure the *next* failure (whatever its cause) is correctly diagnosed and logged.
- Plans 104-02 (publish-crates neutralization) and 104-03 (poll-until-green release cut) are unblocked to proceed independently of this plan's scope.

---
*Phase: 104-smoke-green-cut-the-trusted-signed-release*
*Completed: 2026-07-03*

## Self-Check: PASSED

- FOUND: scripts/verify-authenticode.ps1
- FOUND: scripts/tests/test_verify_authenticode.ps1
- FOUND: .planning/phases/104-smoke-green-cut-the-trusted-signed-release/104-01-SUMMARY.md
- FOUND commit: 30f97268
- FOUND commit: 8dcb5c10
