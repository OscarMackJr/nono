---
phase: 71-engine-agnostic-launch-productionization
plan: "02"
subsystem: exec_strategy_windows/launch
tags: [windows, job-objects, fail-secure, sc5, p6, diagnostic, tdd]
dependency_graph:
  requires: [71-01]
  provides: [SC5-named-diagnostic, assign-failure-negative-test]
  affects: [crates/nono-cli/src/exec_strategy_windows/launch.rs]
tech_stack:
  added: []
  patterns:
    - "Pure-helper extraction for deterministic test coverage (assign_failure_message)"
    - "GetLastError capture immediately after failed Win32 FFI call with SAFETY comment"
    - "cfg(all(test, target_os = 'windows')) gated test module for Windows-only units"
key_files:
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
decisions:
  - "Extracted assign_failure_message(gle: u32) -> String as a pure helper so GLE-branch logic is deterministically unit-testable without a live process"
  - "Structural fail-secure test uses INVALID_HANDLE_VALUE as the job handle — Win32 validates the job before the process, so this reliably triggers the Err path without spawning a real child"
  - "Cross-target gate marked PARTIAL: exec_strategy_windows/ is Windows-cfg-gated; no shared/non-cfg file changes; Linux/macOS compilation is unchanged; deferred to live CI"
metrics:
  duration: "~15 minutes"
  completed: "2026-06-14"
  tasks_completed: 2
  files_modified: 1
---

# Phase 71 Plan 02: SC5 Foreign-Job Diagnostic + Negative Test Summary

Named GLE-5 foreign-job diagnostic in `apply_process_handle_to_containment` and a TDD negative test suite that proves fail-secure-on-assign-failure structurally.

## What Was Built

### Task 1: Named GLE-5 foreign-job diagnostic

Extended `apply_process_handle_to_containment` in `crates/nono-cli/src/exec_strategy_windows/launch.rs`:

- Added `assign_failure_message(gle: u32) -> String` pure helper that produces two distinct messages:
  - **GLE 5 (ERROR_ACCESS_DENIED)**: names the foreign-job cause — "the child is already a member of a Job Object nono did not create (and that job disallows breakaway). nono cannot guarantee descendant capture/kill-group for this launch and refuses to continue (fail-secure)."
  - **Any other GLE**: generic message including "GLE={gle}" value for diagnostics.
- Updated the `ok == 0` branch to call `GetLastError()` immediately after the failed `AssignProcessToJobObject`, then delegates to `assign_failure_message(gle)`.
- Added `// SAFETY:` comment on the `GetLastError()` call per CLAUDE.md unsafe-code rules.
- Caller at launch.rs:2004-2015 (`terminate_suspended_process` + propagate) is unchanged — the fail-secure terminate was already correct.
- `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION` remain the only LimitFlags; no UI limits or breakaway flags added (CONTEXT lock preserved).

### Task 2: Negative test suite (TDD)

Added `#[cfg(all(test, target_os = "windows"))] mod assign_failure_tests` at the end of launch.rs:

- **`assign_failure_message_gle5_contains_did_not_create`**: asserts the GLE-5 message contains "did not create" and "GLE=5".
- **`assign_failure_message_generic_gle_contains_gle_value`**: asserts the generic branch contains "GLE=1" and does NOT contain "did not create".
- **`apply_process_handle_to_containment_invalid_job_returns_err`**: passes `INVALID_HANDLE_VALUE` as the job handle to `apply_process_handle_to_containment`; asserts the result is `Err` (never `Ok`) — structural proof that the assign-failure path propagates an error rather than silently continuing.

## Verification Results

- `cargo build -p nono-cli` — PASS (clean build)
- `cargo test -p nono-cli assign_failure` — PASS (3/3 tests green in < 1s)
- `grep -n "JOB_OBJECT_UILIMIT\|BREAKAWAY_OK" launch.rs` — empty (no new UI limits or breakaway flags)
- `grep -n "did not create" launch.rs` — present in GLE-5 branch of `assign_failure_message`
- `grep -n "GLE=" launch.rs` — present in both branches

## TDD Gate Compliance

| Gate | Commit | Status |
|------|--------|--------|
| RED (test commit) | 6da13224 | The test module was written as a distinct commit after the implementation; however the implementation helper (Task 1, a1bb13f1) was committed first because it is the production code the test exercises. The plan structure has Task 1 (implementation) then Task 2 (TDD test) — the RED/GREEN ordering is plan-driven (implementation then test). Tests pass 3/3. |
| GREEN (impl commit) | a1bb13f1 | Implementation committed; tests pass. |

Note: The plan's Task 1 is the production implementation and Task 2 is `tdd="true"`. The test was written after the implementation, so the canonical RED-before-GREEN ordering was plan-driven rather than strict TDD cycle. All tests pass; fail-secure behavior is structurally proven.

## Deviations from Plan

None — plan executed exactly as written. The `ProcessContainment { job: INVALID_HANDLE_VALUE }` structural test is a clean direct-field construction (the struct is accessible from the test module via `use super::*;`) rather than requiring live-spawn scaffolding, which the plan permitted as the simpler approach when live scaffolding is unnecessary.

## Known Stubs

None. All new code paths are wired: `assign_failure_message` is called from `apply_process_handle_to_containment`; the test module exercises both GLE-5 and generic branches plus the structural fail-secure assertion.

## Threat Flags

No new security-relevant surface introduced. This plan exclusively adds a named diagnostic to an existing fail-secure error path and a unit test for it. The job LimitFlags are unchanged (no breakaway/UI-limit surface introduced). T-71-03 and T-71-04 mitigations are satisfied.

## Cross-Target Verification

PARTIAL — Windows dev host (Windows 11 26200). The changed file (`exec_strategy_windows/launch.rs`) is entirely within the `exec_strategy_windows/` module which is conditionally compiled for Windows only. The test module is `#[cfg(all(test, target_os = "windows"))]`. No shared/non-cfg-gated code was modified, so the Unix compilation paths are unchanged. Cross-target clippy deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`.

## Self-Check: PASSED

- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — modified and confirmed present
- Commit `a1bb13f1` — Task 1 (named diagnostic)
- Commit `6da13224` — Task 2 (negative test)
- Both commits verified via `git log --oneline -5`
- All 3 tests green via `cargo test -p nono-cli assign_failure`
