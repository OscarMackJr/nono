---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 14
subsystem: windows-sandbox
tags: [windows, dacl-guard, seatbelt-analog, fail-direction-contract, attestation, clippy-cross-target]

# Dependency graph
requires:
  - phase: 117 (waves 1-5, 117-01..117-12)
    provides: layer registry, LayerAttestationFailed error variant, LayerApplication enum with NotApplicable variant, CR-14 "report guard effect, not guard construction" pattern established on AppliedLabelsGuard/AppliedDaclGrantsGuard
  - plan: 117-13
    provides: reference shape for the "report guard effect, not guard construction" fix pattern (this plan applies the same shape to the two remaining structurally-identical guards CR-14 left behind)
provides:
  - "AppliedAncestorTraverseGuard::application() and AppliedAncestorReadAttributesGuard::application() coverage accessors"
  - "mod.rs::applied_layers() dacl_ancestor_traverse/dacl_ancestor_read_attrs derived from real guard coverage, not Option::is_some()"
affects: [117-18, 117-19]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Empty-grant-set disambiguation for ancestor-walk guards with no distinct skip arm: applied.is_empty() -> NotApplicable (legitimate full coverage of an empty contract), non-empty -> Applied; guard-never-ran (None in the caller) stays NotApplied via map_or's default arm"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs

key-decisions:
  - "application() on both ancestor guards returns NotApplicable (not NotApplied) for an empty applied set, distinguishing 'the walk legitimately found nothing user-owned to grant' from 'the guard never ran at all' — a third state mod.rs's None case already owns"
  - "Both accessors take &self (read-only), unlike DaclGrantCoverage::application which takes self by value — DaclGrantCoverage is a small Copy struct returned by coverage(), while the ancestor guards' applied: Vec<PathBuf> field is not Copy and the guard is still alive (borrowed) when mod.rs calls the accessor"
  - "Negative-case tests use a leaf under %SystemRoot%\\System32 rather than trying to construct a non-owned tempdir: System32 is deterministically TrustedInstaller/SYSTEM-owned and readable by an unprivileged token (GetNamedSecurityInfoW only needs READ_CONTROL), matching the existing path_is_owned_by_current_user_returns_false_for_system_windows_dir precedent in crates/nono/src/sandbox/windows.rs. The leaf path itself is never touched by the walk (skip(1) skips index 0), so it need not exist on disk."
  - "Positive-case coverage assertions were added to the two EXISTING _grants_owned_ancestors_and_reverts_on_drop tests rather than duplicated into new test functions, since those tests already construct the 'granted something' scenario; only the genuinely-empty negative case needed brand-new test fixtures"

patterns-established:
  - "Ancestor-walk ownership-gated guards (no distinct skip-vs-apply arm) report coverage via applied.is_empty() -> NotApplicable, matching the vocabulary DaclGrantCoverage::application() established for guards that DO have a distinct skip arm"

requirements-completed: []  # CINT-02 spans 117-13..117-17; not marked complete until all wave-6 plans land (repo convention — see 117-13-SUMMARY.md)

# Metrics
duration: ~20min
completed: 2026-08-10
---

# Phase 117 Plan 14: Ancestor DACL Guard Coverage Reporting (NR3-02) Summary

**`AppliedAncestorTraverseGuard` and `AppliedAncestorReadAttributesGuard` now expose an `application()` coverage accessor mirroring `DaclGrantCoverage::application()`'s shape, and `mod.rs::applied_layers()` reads it instead of `Option::is_some()` on a field `execution_runtime.rs` sets `Some(..)` unconditionally on every shipped `DirectCli` launch.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-08-10
- **Completed:** 2026-08-10
- **Tasks:** 2/2
- **Files modified:** 2

## Accomplishments
- Closed NR3-02: both ancestor-DACL attestation rows (`dacl_ancestor_traverse`, `dacl_ancestor_read_attrs`) now derive their `LayerApplication` from the guard's own recorded `applied: Vec<PathBuf>` state instead of the constant-true `Option::is_some()` the previous shape used
- The three genuinely distinct states — "guard never ran" (`NotApplied`), "walk found nothing owned to grant" (`NotApplicable`), "walk granted at least one ancestor" (`Applied`) — are each reachable and pinned by a passing test
- Deleted the now-dead `application_of` helper (its only two callers were the two lines this plan replaced)
- Confirmed via the full `attestation::` + `launch::attestation_gate_tests::` suite (48 tests) that no currently-shipped launch's decision changed — both rows still report `Applied` on every real `DirectCli` AppContainer launch today

## Task Commits

Each task was committed atomically:

1. **Task 1: application() accessor on both ancestor guards** - `87faaf18` (fix)
2. **Task 2: Wire mod.rs::applied_layers() to the new accessors; delete application_of** - `361c6539` (fix)

## Files Created/Modified
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` - Added `pub(crate) fn application(&self) -> layer_registry::LayerApplication` to both `AppliedAncestorTraverseGuard` and `AppliedAncestorReadAttributesGuard` (NotApplicable when `applied.is_empty()`, else Applied); added 2 new negative-direction regression tests (`ancestor_traverse_application_reports_not_applicable_when_nothing_owned_to_grant`, `ancestor_read_attrs_application_reports_not_applicable_when_nothing_owned_to_grant`) using a `%SystemRoot%\System32`-rooted leaf to deterministically produce an empty grant set; extended the 2 existing positive `_grants_owned_ancestors_and_reverts_on_drop` tests with an `Applied` assertion
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - `PreparedWindowsLaunch::applied_layers()`'s `dacl_ancestor_traverse`/`dacl_ancestor_read_attrs` fields now call `self._applied_ancestor_traverse.as_ref().map_or(NotApplied, AppliedAncestorTraverseGuard::application)` (and the read-attrs equivalent); updated the field doc comment to state the corrected reasoning; deleted the `application_of` helper function entirely

## Decisions Made
- `application()` returns `NotApplicable` (not `NotApplied`) for an empty grant set — the walk legitimately found nothing user-owned to grant, which is full coverage of an empty contract, distinct from the guard never running at all
- Both accessors take `&self` (unlike `DaclGrantCoverage::application(self)`, which takes ownership of a small `Copy` struct) — the ancestor guards' `applied: Vec<PathBuf>` field is not `Copy` and `mod.rs` calls the accessor while the guard is still alive and owned by `Option<Guard>`
- Negative-case test fixtures use `%SystemRoot%\System32` (deterministically non-owned, readable) rather than attempting to synthesize a non-owned tempdir — matches the existing `path_is_owned_by_current_user_returns_false_for_system_windows_dir` precedent in the core library's test suite
- Positive-case assertions were folded into the existing `_grants_owned_ancestors_and_reverts_on_drop` tests rather than duplicated, since those tests already construct the "granted something" scenario end to end

## Deviations from Plan

None - plan executed exactly as written. The plan's own `<interfaces>` and `<action>` sections described the exact accessor shape, call-site replacement, and doc-comment update landed here; no bugs, missing functionality, or blocking issues were discovered during execution.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Cross-Target Clippy

**DOES NOT APPLY.** Per the plan's `<verification>` section: `grep -n 'cfg(target_os = "linux"\|cfg(target_os = "macos"' crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs crates/nono-cli/src/exec_strategy_windows/mod.rs` returns zero hits (re-confirmed live during this execution). Neither file contains a Unix cfg branch, and neither is under `bindings/c/src/` or the Unix `exec_strategy/` directory — both files are entirely under `exec_strategy_windows/`. No PARTIAL→CI claim is made; this is a genuine, evidence-based exemption per CLAUDE.md's own cross-target-verify rule (which scopes to files containing Unix cfg branches or under the listed directories), not a fallback.

Ran on this host instead, all clean:
- `cargo build --workspace --all-targets`
- `cargo clippy --workspace --all-targets --features layer-fault-injection -- -D warnings -D clippy::unwrap_used`
- `cargo fmt --all -- --check`
- `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 dacl_guard::` (14/14 passing, including the 4 new/extended assertions)
- `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 attestation:: launch::attestation_gate_tests::` (48/48 passing, unchanged)

## Next Phase Readiness

Wave 6 continues with `117-15`, `117-16`, `117-17` (all `depends_on: []`, mutually independent of each other and of `117-13`/`117-14`). Wave 7's `117-18` depends on `117-14`/`117-15`/`117-16`. Wave 8's `117-19` depends on all six wave-6/7 plans including this one. No blockers for the remaining gap-closure plans from this plan's work — `117-14` touched only `dacl_guard.rs` and `mod.rs`, both listed in its own `files_modified`, disjoint from `117-15`/`117-16`/`117-17`'s file sets per the wave-6 parallel-and-disjoint-files framing.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs`
- FOUND: `crates/nono-cli/src/exec_strategy_windows/mod.rs`
- FOUND: commit `87faaf18` (Task 1)
- FOUND: commit `361c6539` (Task 2)
