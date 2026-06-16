---
phase: 75-supplementary-controls-secondary-engines
plan: "06"
subsystem: daemon
tags: [windows, daemon, scm, service-type, gap-closure, agent-cli]

# Dependency graph
requires:
  - phase: 74-persistent-multi-tenant-daemon
    provides: daemon_start / daemon_start_raw_spawn infrastructure; DAEMON_SERVICE_NAME constant; windows_sc_command helper

provides:
  - is_user_own_template_service(&str) -> bool predicate (Windows-cfg-gated)
  - type-50 USER_OWN_PROCESS TEMPLATE detection guard in daemon_start
  - raw-spawn fallback for template-registered services
  - 3 unit tests covering all three branch-selection paths

affects: [75-07, Phase 75 UAT SC4 re-run]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "sc qc best-effort type-check: failure -> empty string -> false -> conservative fallback (never silent succeed)"
    - "is_user_own_template_service predicate extracted for unit-testability (no subprocess needed in tests)"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/agent_cli.rs

key-decisions:
  - "GAP-75-A: type-50 USER_OWN_PROCESS TEMPLATE cannot start via sc start (exits 5 ACCESS_DENIED); raw-spawn is the correct path"
  - "sc qc failure is best-effort: empty output -> is_user_own_template_service returns false -> sc start attempted -> real error surfaces to operator (conservative, never silent succeed)"
  - "Double-space vs single-space in Windows TYPE label: Windows outputs 'USER_OWN_PROCESS TEMPLATE' with single space before TEMPLATE; predicate and test string align on single space"

patterns-established:
  - "Extract subprocess-parsed predicates into named functions gated #[cfg(target_os = 'windows')] for unit-testability with synthetic strings"

requirements-completed: [SUPP-01, SUPP-02, SUPP-03]

# Metrics
duration: 25min
completed: 2026-06-16
---

# Phase 75 Plan 06: daemon_start type-50 detection guard Summary

**`daemon_start` now detects USER_OWN_PROCESS TEMPLATE (type 50) via `sc qc` and falls through to raw-spawn instead of calling `sc start`, closing GAP-75-A (ACCESS_DENIED exit 5 when a template service is registered)**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-16T00:00:00Z
- **Completed:** 2026-06-16T00:25:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added `is_user_own_template_service(&str) -> bool` predicate that checks for the verbatim Windows type-50 label `"USER_OWN_PROCESS TEMPLATE"` in `sc qc` stdout
- Modified `daemon_start` to run `sc qc nono-agentd` after the service-exists gate passes; if the service is a type-50 template, prints a diagnostic and falls through to `daemon_start_raw_spawn` (skipping `sc start`)
- sc qc failure is treated as best-effort: `unwrap_or_default()` on the output gives empty string, predicate returns false, sc start is attempted normally (real error surfaces to operator)
- Added 3 unit tests covering all three branch-selection paths: type-50 template, type-10 WIN32_OWN_PROCESS, and no-service (sc query 1060)
- All 3 tests pass; `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` clean; `cargo fmt --all -- --check` clean

## Task Commits

1. **Task 1: Add type-50 detection guard to daemon_start + unit tests** - `a2c44c3f` (fix)

## Files Created/Modified

- `crates/nono-cli/src/agent_cli.rs` - Added `is_user_own_template_service` predicate + type-50 guard in `daemon_start` + 3 unit tests

## Decisions Made

- **Conservative sc qc failure handling:** if `sc qc` fails to execute, the predicate returns false and sc start is attempted — the operator sees the real sc start error rather than a silent misleading success or fallback. This matches the plan's T-75-06-03 mitigation.
- **Single-space string for type label:** The verbatim Windows type-50 output is `TYPE : 50  USER_OWN_PROCESS TEMPLATE` (two spaces after `: 50`, then one space before `TEMPLATE`). The predicate substring `"USER_OWN_PROCESS TEMPLATE"` (single space) correctly matches this.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test string used double-space before TEMPLATE, predicate uses single-space**

- **Found during:** Task 1, first test run
- **Issue:** The synthetic sc qc output in `daemon_start_uses_raw_spawn_for_type50_template` used `"USER_OWN_PROCESS  TEMPLATE"` (double space) while the predicate checks `"USER_OWN_PROCESS TEMPLATE"` (single space, matching actual Windows output). Test failed on first run.
- **Fix:** Corrected the test string to use single space before `TEMPLATE`, matching the verbatim Windows format confirmed in the plan's `<interfaces>` block and UAT notes.
- **Files modified:** `crates/nono-cli/src/agent_cli.rs`
- **Verification:** All 3 tests pass after fix
- **Committed in:** `a2c44c3f` (included in the same task commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - test string typo, double-space vs single-space)
**Impact on plan:** Trivial typo in test synthetic string; predicate and production code were correct from the start. No scope creep.

## Issues Encountered

None - the only issue was the double-space typo in the test string (auto-fixed per Rule 1).

## Cross-Target Clippy Status

PARTIAL — `is_user_own_template_service` and the three new tests are `#[cfg(target_os = "windows")]`-gated. Cross-target clippy for Linux/macOS CANNOT be run on this Windows 11 dev host (no cross C toolchain). Deferred to live CI per CLAUDE.md cross-target-verify-checklist.

## Threat Surface Scan

No new network endpoints, auth paths, or file access patterns introduced. sc qc output is parsed via bounded string search (no regex, no shell expansion). The raw-spawn fallback inherits the operator's session — no privilege change vs. the existing no-service path (T-75-06-02 accepted). No new threat flags.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-75-A is closed. `nono daemon start` will now fall through to raw-spawn when a type-50 template is registered, avoiding the exit-5 ACCESS_DENIED dead end.
- SC4 UAT re-run can now be attempted: install the template service via `nono daemon install`, confirm `sc qc nono-agentd` shows `TYPE : 50`, then run `nono daemon start` and verify it reaches RUNNING.
- Plan 75-07 (GAP-75-B, capability-less launch) is the remaining gap-closure plan before Phase 75 is complete.

## Self-Check: PASSED

- `crates/nono-cli/src/agent_cli.rs` exists and contains `is_user_own_template_service`
- Commit `a2c44c3f` exists: `git log --oneline --all | grep a2c44c3f` confirmed
- 3 tests PASS: `daemon_start_uses_raw_spawn_for_type50_template`, `daemon_start_uses_sc_start_for_type10`, `daemon_start_uses_raw_spawn_for_no_service`
- `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used`: 0 warnings
- `cargo fmt --all -- --check`: 0 diff

---
*Phase: 75-supplementary-controls-secondary-engines*
*Completed: 2026-06-16*
