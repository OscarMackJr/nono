---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 23
subsystem: infra
tags: [windows, dacl, attestation, layer-registry, startup-self-attestation, wr-07]

# Dependency graph
requires:
  - phase: 117-21
    provides: "D-28 private-channel gating for downgrade-warn layer names in launch.rs"
provides:
  - "AppliedAncestorTraverseGuard::application() and AppliedAncestorReadAttributesGuard::application() are genuinely 3-valued (NotApplicable / PartiallyApplied / Applied), driven by a new walked: bool tracking whether the walk considered any ancestor at all"
  - "A gate-level regression test proving a PartiallyApplied DaclAncestorTraverse row reaches AttestationDecision::ProceedDowngraded naming the layer specifically, through the real layer_registry + attest_and_decide"
affects: [117-tool-sandbox-parity-followups, windows-composite-integrity-attestation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "walked: bool set as the FIRST statement of an ancestor-walk loop body (before any branch) distinguishes 'the walk ran but found nothing' from 'the walk had nothing to consider' — same shape now used by both ancestor guards"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs

key-decisions:
  - "walked is set as the first statement inside the ancestor loop body (before the ownership match / dedup continue), so it is true iff Path::ancestors().skip(1) yielded at least one item — independent of which branch (owned/non-owned/error) that item took"
  - "The rootless-walk test uses the SystemDrive env var + trailing separator (e.g. C:\\\\) as the input whose .ancestors().skip(1) is empty, with an explicit precondition assertion (count() == 0) so the test fails loudly if that assumption ever breaks on a future host"
  - "Task 2's test drives both attestation::attest_and_decide directly (to assert the downgraded Vec names LayerId::DaclAncestorTraverse) AND apply_startup_attestation_gate (to assert Ok(())) against the REAL layer_registry, exceeding the plan's minimum bar of asserting only the gate's Result — this is a stronger, non-vacuous proof that the row participates in the decision rather than merely not erroring"
  - "Cross-target clippy gate determined OUT OF SCOPE per .planning/templates/cross-target-verify-checklist.md's explicit exclusion: all three modified files live under exec_strategy_windows/ and have NO Unix counterpart (crates/nono-cli/src/exec_strategy/ contains entirely different files: env_sanitization.rs, supervisor_linux.rs, supervisor_macos.rs) — see Cross-Target Verification section below"

requirements-completed: [CINT-02]

# Metrics
duration: 35min
completed: 2026-08-11
---

# Phase 117 Plan 23: Ancestor-Guard 3-State Coverage Reporting (WR-07) Summary

**`AppliedAncestorTraverseGuard`/`AppliedAncestorReadAttributesGuard::application()` now distinguish "walked but granted nothing" (`PartiallyApplied`, visibly downgrades) from "nothing to walk" (`NotApplicable`), closing the WR-07 green-by-absence gap with a gate-level test proving the new state reaches `ProceedDowngraded` naming the layer.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-08-11T01:20:00Z (approx)
- **Completed:** 2026-08-11T02:00:30Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Both ancestor-DACL guards (`AppliedAncestorTraverseGuard`, `AppliedAncestorReadAttributesGuard`) now carry a `walked: bool` set on the first loop iteration regardless of which branch (owned/non-owned/error) is taken, making `.application()` genuinely three-valued instead of collapsing "walked-but-empty" and "nothing-to-walk" into the same `NotApplicable`.
- Renamed and retargeted both guards' existing `*_reports_not_applicable_when_nothing_owned_to_grant` tests to assert `PartiallyApplied` for the walked-but-empty scenario (their System32-parent scenario is unchanged), and added a new `*_reports_not_applicable_for_a_rootless_walk` test per guard proving the narrower, correct `NotApplicable` condition (a drive root with zero ancestors to consider).
- Corrected `mod.rs`'s `applied_layers()` doc comment to state the new 3-state ancestor-walk rule.
- Added `dacl_ancestor_traverse_row_reports_partially_applied_from_a_real_gate` in `launch.rs`, mirroring the existing `partially_applied_launch_is_downgraded_not_silently_passed` precedent: drives a real suspended `cmd.exe` + real Job Object through both `attestation::attest_and_decide` (asserting the `ProceedDowngraded { downgraded }` vec names `LayerId::DaclAncestorTraverse` specifically) and `apply_startup_attestation_gate` (asserting `Ok(())`).

## Task Commits

Each task was committed atomically:

1. **Task 1: Add walked tracking and a 3-state application() to both ancestor guards; correct the mod.rs doc comment** - `cbd30c8b` (fix)
2. **Task 2: Add a gate-level regression test proving the new PartiallyApplied state reaches ProceedDowngraded, not silence** - `f2f816e2` (test)

**Plan metadata:** (this commit) `docs: complete plan`

## Files Created/Modified
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` - added `walked: bool` to both ancestor guards; 3-branch `application()` on each; renamed/retargeted 2 tests, added 2 new rootless-walk tests
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - corrected the `applied_layers()` doc comment describing the ancestor-guard coverage rule
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - added `dacl_ancestor_traverse_row_reports_partially_applied_from_a_real_gate`

## Decisions Made
- `walked` is set as the very first statement inside each ancestor-walk loop body, ahead of the ownership match (traverse guard) and ahead of the dedup `continue` (read-attrs guard, per the plan's explicit instruction) — this guarantees `walked` reflects "at least one ancestor was considered" independent of whether it was owned, non-owned, erroring, or a dedup-skip.
- The rootless-walk tests assert a precondition (`drive_root.ancestors().skip(1).count() == 0`) before asserting on `walked`/`application()`, so the test fails with a clear message rather than silently passing vacuously if `SystemDrive` ever resolves to something with ancestors on a future host shape.
- Task 2's test goes beyond the plan's minimum acceptance bar (which only required asserting `apply_startup_attestation_gate`'s `Ok(())`) by also calling `attestation::attest_and_decide` directly and asserting the returned `downgraded` vec names `LayerId::DaclAncestorTraverse` — this is the durable-lesson-driven choice (see `feedback_verify_predicate_width_not_just_placement`): a test that only checks `Ok(())` cannot distinguish "downgraded but visible" from "silently treated as fully confirmed", since both would return `Ok(())` at the gate. Asserting the `LayerId` inside `ProceedDowngraded` is what makes the test non-vacuous proof that the row participates in the decision rather than merely not erroring.

## Deviations from Plan

None — plan executed exactly as written. Both tasks matched the plan's `<action>`/`<acceptance_criteria>` precisely; the one addition (asserting `attest_and_decide`'s `downgraded` vec directly in Task 2, rather than relying solely on the gate's `Ok(())`) was explicitly offered as an equally-valid option by the plan's own `<behavior>` block ("OR ... assert via the `AttestationDecision` returned by the lower-level `attest_and_decide`/`decide_from_entries` call this gate wraps") and strengthens the test rather than deviating from it.

## Issues Encountered

None. `cargo fmt` reformatted three multi-line `assert!` calls and one long `snapshot_and_apply` call to satisfy the project's line-width rules after the new test code was written — resolved by running `cargo fmt -p nono-sandbox-cli` before commit; no logic changes.

## Cross-Target Verification

This plan's three modified files (`dacl_guard.rs`, `mod.rs`, `launch.rs`) all live under `crates/nono-cli/src/exec_strategy_windows/`. Per `.planning/templates/cross-target-verify-checklist.md` § Scope: "Does NOT apply to: Pure Windows-only files (e.g. anything under `crates/nono-cli/src/exec_strategy_windows/` that has NO Unix counterpart)". Verified this exclusion applies: the Unix build (`#[cfg(not(target_os = "windows"))] mod exec_strategy;` in `main.rs`) points at `crates/nono-cli/src/exec_strategy/`, which contains an entirely different file set (`env_sanitization.rs`, `supervisor_linux.rs`, `supervisor_macos.rs`) with no `dacl_guard.rs`, `launch.rs`, or `mod.rs` counterparts. None of this plan's edits touch any `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` / `#[cfg(any(target_os = "linux", target_os = "macos"))]` block. **Cross-target linux-gnu/apple-darwin clippy gates were therefore not run — determined out of scope, not skipped or deferred.**

Gates that WERE run on the Windows host and are in scope for these files:
- `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection exec_strategy::dacl_guard` — 16/16 passed
- `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection exec_strategy::launch::attestation_gate_tests::` — 64/64 passed (includes the new `dacl_ancestor_traverse_row_reports_partially_applied_from_a_real_gate`)
- `cargo clippy -p nono-sandbox-cli --bin nono --all-features -- -D warnings -D clippy::unwrap_used` — clean
- `cargo fmt --check -p nono-sandbox-cli` — clean

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

WR-07 is closed: both ancestor guards' `application()` methods are genuinely three-valued, driven by real walk state, and a gate-level test proves the new `PartiallyApplied` state produces a visible downgrade rather than a silent drop. No known follow-ups from this plan. STATE.md/ROADMAP.md/REQUIREMENTS.md updates for CINT-02 are owned by the orchestrator, not this plan.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-23-SUMMARY.md`
- FOUND: `cbd30c8b` (Task 1 commit)
- FOUND: `f2f816e2` (Task 2 commit)
- FOUND: `49c1c3dd` (SUMMARY commit)
