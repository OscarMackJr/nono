---
phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
plan: 06
subsystem: infra
tags: [windows, wfp, network-enforcement, fail-secure, kernel, service]

requires:
  - phase: 62-01
    provides: "D-03 auto-start hook and BackendServiceStopped handling in install_wfp_network_backend_with_runner"

provides:
  - "build_wfp_probe_status returns Ready on BFE+service Running with no kernel-driver requirement (F-62-UAT-01 fix)"
  - "probe_wfp_backend_status_with_config no longer sc-queries the kernel driver (one fewer SCM round-trip)"
  - "Stale SERVICE placeholder strings retired; DRIVER placeholder strings kept accurate"
  - "Unit tests: test_wfp_ready_without_kernel_driver + 4 fail-secure regression assertions"

affects:
  - "62-04 HUMAN-UAT SC1 (nono run --block-net now reaches FwpmFilterAdd0 service path instead of failing at driver gate)"
  - "REQ-WFP-01 (SC1 unblocked: supervised run can activate WFP enforcement out of the box)"

tech-stack:
  added: []
  patterns:
    - "D-05 service-only model enforced in probe: BFE Running + nono-wfp-service Running is sufficient for Ready; kernel driver is structurally irrelevant to the run-path readiness decision"

key-files:
  created: []
  modified:
    - "crates/nono-cli/src/exec_strategy_windows/network.rs"

key-decisions:
  - "Drop backend_driver_binary_exists and backend_driver params from build_wfp_probe_status — kernel driver is out of scope per D-05 and must not be a precondition of Ready"
  - "Remove the sc query for backend_driver from probe_wfp_backend_status_with_config real branch — saves one SCM round-trip and eliminates the misleading dependency"
  - "Keep WfpProbeStatus driver variants (BackendDriverBinaryMissing/BackendDriverMissing/BackendDriverStopped) and all their describe arms — still used by nono setup --install-wfp-driver diagnostics"
  - "Keep DRIVER-side placeholder strings (L301 driver description, L1204/L1212 driver run-state) — the kernel driver genuinely remains an out-of-scope placeholder; those strings are accurate"
  - "Retire SERVICE-side strings claiming enforcement was unimplemented — service FwpmFilterAdd0 path is fully wired and reachable once Ready is reached"

patterns-established:
  - "Fail-secure precondition audit: when removing a gate, add explicit regression tests for all remaining prerequisites to prove the loosening did not weaken real fail-secure checks"

requirements-completed:
  - REQ-WFP-01

duration: 22min
completed: 2026-06-02
---

# Phase 62 Plan 06: WFP Client Readiness Gate Fix (F-62-UAT-01) Summary

**Removed the out-of-scope kernel-driver gate from `build_wfp_probe_status` so BFE Running + nono-wfp-service Running is sufficient for `WfpProbeStatus::Ready`, unblocking the fully-wired `FwpmFilterAdd0` enforcement path per D-05 service-only model.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-06-02T16:00:00Z
- **Completed:** 2026-06-02T16:22:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Removed `backend_driver_binary_exists` and `backend_driver` params from `build_wfp_probe_status`; deleted the driver gate blocks (binary-exists check + service-state match) that unconditionally preceded `WfpProbeStatus::Ready`
- Updated `probe_wfp_backend_status_with_config`: force-ready branch drops driver args; real branch removes the `run_sc_query(config.backend_driver)` call entirely (one fewer SCM round-trip per run)
- Added `test_wfp_ready_without_kernel_driver` asserting Ready with no driver, plus 4 fail-secure regression tests (BackendBinaryMissing, PlatformServiceStopped, BackendServiceMissing, BackendServiceStopped still short-circuit); all 16 `exec_strategy::network` tests pass
- Retired 3 stale SERVICE-side strings that falsely claimed enforcement was unimplemented; DRIVER strings kept accurate (driver remains out of scope)
- WfpProbeStatus driver variants and all setup-diagnostic describe arms retained (still used by `nono setup --install-wfp-driver` path)

## Task Commits

1. **Task 1: RED — add failing tests with new 3-param signature** - `f8f24aef` (test)
2. **Task 1 GREEN + Task 2: production gate removal + stale string retirement** - `dd609bff` (fix)

**Plan metadata:** (created below in final commit)

_Note: RED test commit confirmed compile failure (5-arg vs 3-arg) before GREEN production change landed. GREEN confirmed all 16 tests pass._

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/network.rs` — Removed driver gate from `build_wfp_probe_status` (L344-383), dropped driver sc-query from `probe_wfp_backend_status_with_config` (L386-415), retired 3 stale SERVICE strings (L794, L1375, L1382), added 5 unit tests (L2249-2308)

## Decisions Made

- Drop both `backend_driver_binary_exists: bool` and `backend_driver: WindowsServiceState` params. Per D-05 the kernel driver is out of scope and was never the enforcement primitive; dropping its probe removes a misleading dependency, not a security check (T-62-13 accept).
- Retain WfpProbeStatus driver variants — they are still reachable from the setup-diagnostics path (`describe_wfp_setup_state`, `describe_wfp_runtime_activation_failure`). After this change they are simply unreachable from `build_wfp_probe_status`, which is correct.
- Retire SERVICE strings only; DRIVER strings (L301/L1204/L1212) are factually accurate (placeholder driver) and must not change.

## Deviations from Plan

None — plan executed exactly as written. All tasks completed within planned scope.

## Threat Model Compliance (T-62-12)

All fail-secure prerequisites preserved:
- `BackendBinaryMissing` still short-circuits before any service checks
- `PlatformServiceMissing`/`PlatformServiceStopped` still short-circuit before `BackendService` checks
- `BackendServiceMissing`/`BackendServiceStopped` still short-circuit before `Ready`
- `Ready` remains the SOLE path to the activation IPC in `install_wfp_network_backend_with_runner`
- No path returns `Ok(None)` (unenforced) when `network.block` is set

Regression test coverage added for every preserved precondition (4 fail-secure tests).

## Cross-Target Clippy Note

Windows-host `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` exits clean (0 warnings from dropped params). Cross-target Linux/macOS clippy for this `#[cfg(target_os = "windows")]`-gated file is **PARTIAL** — deferred to CI per CLAUDE.md § Coding Standards (cross-target verify checklist). The file contains no cfg-gated Unix branches, so the Windows-host clippy is the primary quality gate.

## Issues Encountered

None — the root cause was fully documented in `wfp-driver-gate-placeholder.md` before this plan ran. The fix was straightforward.

## Next Phase Readiness

- REQ-WFP-01 SC1 is now unblocked: a machine-MSI supervised `nono run --block-net` should reach the service's `FwpmFilterAdd0` activation path instead of failing closed at the driver gate
- Live re-verification via the 62-04 HUMAN-UAT SC1 repro is the next step (NOT in this plan — recorded in 62-04 HUMAN-UAT)
- An MSI rebuild off the updated binary is required before the 62-04 repro (current MSI has the old driver-gated binary)

---
*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Completed: 2026-06-02*

## Self-Check: PASSED

- [FOUND] `crates/nono-cli/src/exec_strategy_windows/network.rs` — modified
- [FOUND] Commit `f8f24aef` (RED test commit) — confirmed in git log
- [FOUND] Commit `dd609bff` (GREEN + string fix) — confirmed in git log
- [VERIFIED] All 16 `exec_strategy::network` tests pass
- [VERIFIED] `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` exits 0
- [VERIFIED] No `backend_driver` ref in `build_wfp_probe_status` or `probe_wfp_backend_status_with_config`
- [VERIFIED] No SERVICE strings matching "runtime activation is still not implemented" or "placeholder service host"
- [VERIFIED] DRIVER strings at L301/L1194/L1202 unchanged
