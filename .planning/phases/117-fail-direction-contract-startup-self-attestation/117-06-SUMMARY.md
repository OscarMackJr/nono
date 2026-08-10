---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 06
subsystem: security
tags: [windows, fail-direction, layer-fault-injection, cint-03, dacl, job-object, mandatory-label, restricted-token]

# Dependency graph
requires:
  - phase: 117 (waves 1-2)
    provides: "layer_registry.rs (LayerId rows), NonoError::LayerAttestationFailed, the layer-fault-injection Cargo feature (117-04), and the paired-arms hook idiom shipped for WFP (117-04's WINDOWS_WFP_TEST_FORCE_READY)"
provides:
  - "force_restricted_token_unavailable() seam in restricted_token.rs, short-circuiting create_restricted_token_with_sid to LayerAttestationFailed"
  - "force_mandatory_label_unavailable() seam in labels_guard.rs, short-circuiting AppliedLabelsGuard::snapshot_and_apply"
  - "force_dacl_grant_unavailable() seam in dacl_guard.rs, shared across AppliedDaclGrantsGuard::snapshot_and_apply, AppliedAncestorTraverseGuard::snapshot_and_apply, and AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets"
  - "force_job_object_unavailable() seam in launch.rs, short-circuiting apply_process_handle_to_containment"
  - "8 feature-gated regression tests proving each hook short-circuits before its real OS call"
affects: [117-12 (per-layer forced-unavailable tests consume these seams end-to-end), 117-07 (AppContainer broker-side hook, sibling seam)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Paired-arms force-unavailable seam: #[cfg(feature = \"layer-fault-injection\")] static AtomicBool + pub(crate) setter + private getter + short-circuit Err at the top of the real apply function — purely additive, no #[cfg(not(...))] stub needed since the hook adds a branch rather than replacing production logic"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
    - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
    - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs

key-decisions:
  - "One shared DACL_GRANT_FORCE_UNAVAILABLE flag covers all three DACL apply functions (dacl_guard.rs), per the plan's explicit design allowance — they share one LayerId family (DaclSessionSidGrant/DaclPackageSidGrant/DaclAncestorTraverse/DaclAncestorReadAttrs) and CINT-03's per-layer tests still assert distinct outcomes by calling each apply function directly"
  - "AppliedDaclGrantsGuard::snapshot_and_apply's hook error uses layer name \"DaclSessionSidGrant\" (the function is the sole call site for both DaclSessionSidGrant and DaclPackageSidGrant per layer_registry.rs's shared citation \"dacl_guard.rs:92\")"
  - "The nono::NonoError import in dacl_guard.rs is itself #[cfg(feature = \"layer-fault-injection\")]-gated (a separate use line) to avoid an unused-import warning under -D warnings in the default build, since the top-level module only references it inside the feature-gated short-circuit blocks"

patterns-established:
  - "Force-unavailable hooks always short-circuit BEFORE the real OS call and BEFORE any RAII guard is constructed, so the existing Drop-order revert discipline never sees partial state to clean up (T-117-13 mitigation)"

requirements-completed: [CINT-03]

# Metrics
duration: 22min
completed: 2026-08-10
---

# Phase 117 Plan 06: Force-Unavailable Hooks for the Four Remaining CLI-Side Layers Summary

**Added compiled-out `layer-fault-injection`-gated force-unavailable seams to restricted token, mandatory integrity label, DACL grants (3 functions, 1 shared flag), and Job Object containment, following Plan 04's shipped WFP paired-arms idiom exactly.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-08-10T01:20:13+01:00 (prior wave-2 checkpoint commit)
- **Completed:** 2026-08-10T01:38:40+01:00
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- All 6 registry rows in D-29's scope for this plan (RestrictedToken, MandatoryIntegrityLabel, DaclSessionSidGrant, DaclPackageSidGrant, DaclAncestorTraverse, DaclAncestorReadAttrs) now have a compiled-out force-unavailable seam, plus JobObjectContainment
- Every hook short-circuits to `NonoError::LayerAttestationFailed { layer, reason }` before the real Win32 call, so no partial guard/token state is ever constructed under fault injection
- Verified on both feature sets (default and `--features layer-fault-injection`) via `cargo build`, `cargo test --bin nono` (host is win32, so Windows-gated tests ran for real), `cargo clippy -D warnings -D clippy::unwrap_used`, and `cargo fmt --all -- --check`

## Task Commits

Each task was committed atomically:

1. **Task 1: Force-unavailable hooks for restricted token and mandatory label** - `6adc8b90` (feat)
2. **Task 2: Force-unavailable hooks for DACL grants and Job Object containment** - `53dc7fbd` (feat)

**Plan metadata:** (this commit) `docs(117-06): complete force-unavailable-hooks-remaining-layers plan`

## Files Created/Modified
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` - `force_restricted_token_unavailable()` seam short-circuiting `create_restricted_token_with_sid`
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` - `force_mandatory_label_unavailable()` seam short-circuiting `AppliedLabelsGuard::snapshot_and_apply`
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` - `force_dacl_grant_unavailable()` seam (one flag, three hooked functions: `AppliedDaclGrantsGuard::snapshot_and_apply`, `AppliedAncestorTraverseGuard::snapshot_and_apply`, `AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets`)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - `force_job_object_unavailable()` seam short-circuiting `apply_process_handle_to_containment`

## Decisions Made
- Kept the interfaces section's exact idiom (static `AtomicBool` + `pub(crate)` setter + private getter + top-of-function `Err` short-circuit), no `#[cfg(not(...))]` stub, per the plan's explicit note that these hooks are purely additive unlike WFP's replacement of always-present logic.
- Used one shared flag for the three DACL functions (plan-sanctioned), naming the shared `AppliedDaclGrantsGuard::snapshot_and_apply` hook's layer `"DaclSessionSidGrant"` since it is the sole call site cited for both `DaclSessionSidGrant` and `DaclPackageSidGrant` registry rows.
- Gated the `nono::NonoError` import in `dacl_guard.rs` behind `#[cfg(feature = "layer-fault-injection")]` (a second, separate `use` line) since the top-level module code only references it inside feature-gated blocks — avoids an unused-import warning under `-D warnings` in the default build without touching the test module's own unconditional `NonoError` import.

## Deviations from Plan

None - plan executed exactly as written. The plan's acceptance-criteria commands assumed a `--lib` target; `nono-cli` (package `nono-sandbox-cli`) is binary-only (no `[lib]` section, two `[[bin]]` targets), so verification ran via `--bin nono` instead — same test binary, same coverage, not a deviation in substance (module path is `exec_strategy::` under the `#[path = "exec_strategy_windows/mod.rs"] mod exec_strategy;` Windows alias in `main.rs`, not `exec_strategy_windows::` as the plan's literal command text used).

## Issues Encountered
- Initial test compile failed with `RestrictedToken` not implementing `Debug` when the forced-unavailable test tried to print the whole `Result` via `{other:?}` — fixed by matching `Err`/`Ok` separately instead of requiring `Debug` on the success type (no plan or production code change needed).
- Initial `dacl_guard.rs` build produced an unused-import warning for `NonoError` in the default (non-feature) build — fixed by feature-gating the import itself.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All four CLI-side layers this plan owned now have the compiled-out seam Plan 12's per-layer CINT-03 tests will drive end-to-end through the real launch path.
- Plan 07 (AppContainer, broker/daemon-side) is a sibling seam not touched here, per this plan's scope note.
- `crates/nono-cli/tests/layer_registry_selfcheck.rs`'s `registry_call_sites_exist` test only verifies cited files exist (not exact line numbers, per its own doc comment on line ~135-137), so the line-number drift introduced by inserting hook code ahead of the hooked functions does not fail any test — still worth a future SC4 discrepancy note if a later plan tightens that check.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*

## Self-Check: PASSED

All 4 modified source files and the SUMMARY.md itself confirmed present on disk; commits `6adc8b90`, `53dc7fbd`, and `78655c37` confirmed in `git log --oneline --all`.
