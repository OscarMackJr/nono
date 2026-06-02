---
phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
plan: "01"
subsystem: windows-networking
tags: [wfp, windows, network-enforcement, service-lifecycle, sandboxing]

# Dependency graph
requires:
  - phase: 60-sandbox-the-tools
    provides: "install_wfp_network_backend_with_runner injectable-runner pattern; WfpProbeStatus types; WindowsWfpStartReport type"
provides:
  - "D-03 auto-start hook in install_wfp_network_backend_with_runner: intercepts BackendServiceStopped, attempts start, re-probes, falls through to IPC or fails closed with remediation message"
  - "start_service_fn injectable parameter enabling mock-based D-03 unit testing without elevation"
  - "build_wfp_service_create_args aligned to start=auto (nono setup --register-wfp-service no longer silently reverts MSI posture)"
  - "Accurate production description in build_wfp_service_description_args"
  - "3 new D-03 unit tests: test_wfp_autostart_on_stopped, test_wfp_autostart_fail_remediation_message, test_wfp_non_stopped_status_unchanged"
affects: [62-02, 62-03, 62-04, windows-supervised-runs, wfp-enforcement]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-03 attempt-then-handle pattern: call start_service_fn, handle Ok/Err rather than pre-checking elevation (avoids TOCTOU)"
    - "Triple-generic injectable runner: <P, R, S> with S: Fn(&WfpProbeConfig) -> Result<WindowsWfpStartReport> extends the existing _with_runner testability pattern"
    - "Re-probe-after-start: on successful auto-start, re-call probe_fn to get fresh WfpProbeStatus before proceeding to IPC activation"

key-files:
  created: []
  modified:
    - "crates/nono-cli/src/exec_strategy_windows/network.rs"

key-decisions:
  - "start_service_fn: S where S: Fn(&WfpProbeConfig) -> Result<WindowsWfpStartReport> chosen over a dual-runner approach to keep generic count minimal and align with the existing _with_runner wrapper pattern (public wrapper closes over run_sc_query + run_sc_command)"
  - "D-03 fail-closed error inlined (not delegated to describe_wfp_runtime_activation_failure) so the remediation wording is fully under control and unambiguous"
  - "describe_wfp_runtime_activation_failure BackendServiceStopped arm updated to post-D-03 wording (names elevated remediation command); this arm is now only reached from non-D-03 call sites"
  - "build_wfp_service_create_args changed to start=auto per open-question resolution: both MSI and manual setup paths now use auto-start, preventing regression if nono setup --register-wfp-service runs post-install"

patterns-established:
  - "Triple-injectable runner functions: <P, R, S> with a public wrapper that closes over real system calls for each injectable"

requirements-completed:
  - REQ-WFP-01

# Metrics
duration: 25min
completed: "2026-06-02"
---

# Phase 62 Plan 01: D-03 Auto-Start Hook + start=auto Alignment Summary

**D-03 runtime auto-start hook wired into install_wfp_network_backend_with_runner via injectable start_service_fn; BackendServiceStopped now triggers sc-start-then-reprobe before fail-closed; build_wfp_service_create_args aligned to start=auto**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-02T14:04:43Z
- **Completed:** 2026-06-02T14:15:13Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Added `start_service_fn: S where S: Fn(&WfpProbeConfig) -> Result<WindowsWfpStartReport>` as a third generic parameter to `install_wfp_network_backend_with_runner`, extending the existing injectable-runner testability pattern
- Implemented D-03 auto-start interception block: `BackendServiceStopped` now calls `start_service_fn`, re-probes on `Ok`, falls through to IPC activation if `Ready`, or returns `Err(UnsupportedPlatform)` with a message naming `nono setup --start-wfp-service` and "elevated" on failure. `BackendServiceMissing` and all other non-Stopped statuses are unaffected.
- Updated `install_wfp_network_backend` public wrapper to pass `|cfg| start_windows_wfp_service_with_runner(cfg, run_sc_query, run_sc_command)` as the third argument
- Added 3 mock-based D-03 unit tests (no elevation required): `test_wfp_autostart_on_stopped`, `test_wfp_autostart_fail_remediation_message`, `test_wfp_non_stopped_status_unchanged` — all pass
- Changed `build_wfp_service_create_args` start= value from `"demand"` to `"auto"` so `nono setup --register-wfp-service` cannot silently revert the MSI's `start=auto` posture
- Replaced stale placeholder description in `build_wfp_service_description_args` with accurate production text

## Task Commits

Each task was committed atomically:

1. **Task 1: D-03 hook + start_service_fn + 3 unit tests** - `096bd1bd` (feat)
2. **Task 2: start=auto + description fix** - `7da6d5d8` (feat)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/network.rs` - D-03 hook, start_service_fn parameter, updated wrapper, 3 new tests, start=auto, accurate description

## Decisions Made

- `start_service_fn` typed as `Fn(&WfpProbeConfig) -> Result<WindowsWfpStartReport>` (single parameter, returns the full start report) rather than `Fn(&WfpProbeConfig, &[String]) -> Result<String>` — keeps generics minimal; the public wrapper closes over `run_sc_query` + `run_sc_command`
- D-03 fail-closed error message inlined (not routed through `describe_wfp_runtime_activation_failure`) for precision: exactly names the service, "could not be started automatically", "elevated", and `nono setup --start-wfp-service`
- `describe_wfp_runtime_activation_failure` `BackendServiceStopped` arm updated to post-D-03 wording; this arm is now reached only if D-03 was not invoked (e.g., direct callers, future non-D-03 paths)
- `build_wfp_service_create_args` changed to `start=auto` per the open-question resolution in RESEARCH.md

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

The existing tests `install_wfp_network_backend_returns_guard_on_enforced_pending_cleanup` and `install_wfp_network_backend_returns_error_on_prerequisites_missing` called the now-5-param function with 5 arguments and needed a `mock_start` parameter added. This was an expected consequence of the signature change and was handled in the GREEN phase as part of the plan.

## Cross-Target Clippy Note

All changes are inside `exec_strategy_windows/` which is Windows-cfg-gated. Per CLAUDE.md, Unix cross-target clippy verification (Linux/macOS) is PARTIAL — deferred to CI. Windows-host check (`cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used`) passed clean.

## Source Assertions (Post-Task Verification)

- `"auto"` in `build_wfp_service_create_args` context: 1 occurrence (line 260)
- `nono setup --start-wfp-service` in network.rs: 5 occurrences (D-03 error message + updated BackendServiceStopped arm + 2 test assertions + prior arm text)
- `Placeholder` in `build_wfp_service_description_args`: 0 occurrences (stale description gone)
- `start_service_fn` in network.rs: 9 occurrences (parameter declaration, bound, call site, wrapper closure, 3 test mock declarations, 2 existing-test mock_start declarations)

## Known Stubs

None - all production paths wire real functions. The `start_windows_wfp_service_with_runner` called in the D-03 path is fully implemented.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. The D-03 hook exclusively uses existing `start_windows_wfp_service_with_runner` (which calls `sc start` via `run_sc_command`) — no new attack surface introduced.

## Next Phase Readiness

- Plan 62-02 (SDDL update + WiX start=auto + ServiceConfig) can proceed immediately
- D-03 foundation (auto-start hook) is in place; plan 62-02 adds the pipe ACL fix that allows non-elevated `nono.exe` to reach the service
- Plan 62-03 (integration test + human UAT prep) depends on both 62-01 and 62-02

---
*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Completed: 2026-06-02*
