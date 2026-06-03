---
phase: 62
plan: 11
subsystem: windows-wfp-uninstall-purge
tags: [wfp, windows, uninstall, purge, fail-open, leave-nothing]
completed: "2026-06-02T23:59:30Z"
duration_minutes: 25
tasks_completed: 2
tasks_total: 2
files_created: []
files_modified:
  - crates/nono-cli/src/bin/nono-wfp-service.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
commits:
  - hash: 650b668d
    message: "feat(62-11): add --purge-wfp-objects mode + shared purge_nono_filters helper"
  - hash: df124ca9
    message: "feat(62-11): invoke --purge-wfp-objects fail-open from uninstall before service delete"
dependency_graph:
  requires: [62-09, 62-10]
  provides: [SC4-leave-nothing, SC5-leave-nothing, REQ-WFP-01-uninstall-clean]
  affects: [nono-wfp-service, exec_strategy_windows/network.rs]
tech_stack:
  added: []
  patterns:
    - "fail-open purge: purge Err recorded but never aborts uninstall"
    - "shared filter-purge helper factored from startup sweep"
    - "FwpmSubLayerDeleteByKey0 for persistent sublayer removal"
    - "run_purge generic param keeps uninstall unit-testable without BFE host"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/bin/nono-wfp-service.rs
    - crates/nono-cli/src/exec_strategy_windows/network.rs
decisions:
  - "FAIL-OPEN for uninstall purge (deliberate inverse of run-time fail-CLOSED): purge Err never aborts; WiX Return=ignore. Best-effort cleanup is correct when the service is being removed."
  - "Factor filter-delete loop into purge_nono_filters helper shared by run_startup_sweep and run_purge_wfp_objects — single code path, no logic divergence."
  - "Spawn backend binary as child process (--purge-wfp-objects) rather than linking WFP FFI into nono-cli: keeps WFP objects and NONO_SUBLAYER_GUID in one binary; cleaner coupling."
  - "Non-zero exit from backend binary not treated as run_backend_purge Err — surfaces stdout/stderr as the report text so the audit trail is preserved."
metrics:
  duration: 25
  completed_date: "2026-06-02"
  test_count_added: 3
  requirements_closed: [REQ-WFP-01]
---

# Phase 62 Plan 11: Uninstall WFP Object Purge (--purge-wfp-objects) Summary

WFP uninstall purge closing the SC4/SC5 leave-nothing gap: `--purge-wfp-objects` mode + `FwpmSubLayerDeleteByKey0` + shared `purge_nono_filters` helper + fail-open uninstall integration.

## What Was Built

**Problem:** 62-09 made the WFP session persistent (fixing FWP_E_WRONG_SESSION). Persistent objects survive engine-handle close AND survive `msiexec /x`. After 62-09, uninstalling nono left `NONO_SUBLAYER_GUID` (and any crash-surviving BLOCK filters) in the BFE object store forever — violating the leave-nothing contract (REQ-WFP-01 SC4/SC5, REQ-DRN-01).

**Fix (Task 1 — nono-wfp-service.rs):**
- Factored the filter enumeration+delete loop out of `run_startup_sweep` into `purge_nono_filters(engine: &WfpEngine)`. Both the startup sweep and the new purge mode now call this single code path — no logic divergence.
- Added `run_purge_wfp_objects()`: opens a persistent WFP engine, calls `purge_nono_filters` to remove all NONO_SUBLAYER_GUID filters, then calls `FwpmSubLayerDeleteByKey0(engine, &NONO_SUBLAYER_GUID)`. Tolerates `FWP_E_FILTER_NOT_FOUND` + `FWP_E_SUBLAYER_NOT_FOUND` (idempotent — running with nothing installed is a clean no-op).
- Added `PURGE_WFP_OBJECTS_ARG = "--purge-wfp-objects"` const; wired into `run()` dispatch; updated `print_help()` and missing-mode error string.
- Imported `FwpmSubLayerDeleteByKey0` and `FWP_E_SUBLAYER_NOT_FOUND` from `windows_sys`.
- Fixed stale comment `(62-10) uninstall purge` → `(62-11)` in `open_wfp_engine`.

**Fix (Task 2 — exec_strategy_windows/network.rs):**
- Added `run_purge: P` generic parameter to `uninstall_windows_wfp_with_runner` so it stays mock-testable (no BFE host needed for unit tests).
- At the start of the body (BEFORE service/driver stop+delete), calls `run_purge(&config.backend_binary_path)` FAIL-OPEN: `Err` is logged via `tracing::warn` and recorded as "wfp object purge skipped (best-effort)" in `report.details`; uninstall never returns `Err` due to purge failure.
- Added `run_backend_purge()`: skips absent binary (returns success string), spawns `backend --purge-wfp-objects`; non-zero exit is NOT an error — surfaces stdout/stderr for the audit report.
- Updated all 4 existing uninstall unit tests to pass a mock purge closure.
- Added `uninstall_purge_failure_is_fail_open` test: `Err` purge → `Ok` uninstall + report.details contains "wfp object purge skipped (best-effort)".

## Fail-Open vs Fail-Closed: Design Rationale

The purge is deliberately **FAIL-OPEN** at uninstall time. This is the inverse of the run-time fail-CLOSED stance:
- At run-time: if WFP setup fails → `Platform not supported` error (fail-CLOSED). No risk of unprotected execution.
- At uninstall: if the purge fails → log warning, continue service deletion (fail-OPEN). The service is going away regardless. A hard failure would brick removal (T-62-24). Worst-case = an orphan sublayer (the pre-fix status quo), never a stuck uninstall.
- The WiX custom action (`CaUninstallWfpServices`) has `Return=ignore`, reinforcing this at the MSI level.

## Leave-Nothing Closure

The full chain is now: `msiexec /x` → `CaUninstallWfpServices` (deferred, LocalSystem, Before=RemoveFiles) → `nono.exe setup --uninstall-wfp` → `uninstall_windows_wfp` → `uninstall_windows_wfp_with_runner` → `run_backend_purge` spawns `nono-wfp-service.exe --purge-wfp-objects` → `run_purge_wfp_objects` deletes all NONO_SUBLAYER_GUID filters + `FwpmSubLayerDeleteByKey0(NONO_SUBLAYER_GUID)` → service + driver stop+delete.

No WiX change required — the purge rides the existing custom action.

## FFI Live-Test Limitation

`FwpmSubLayerDeleteByKey0` and the end-to-end uninstall purge path require an elevated BFE host. Unit tests cover arg dispatch and fail-open contract; the actual WFP object deletion is validated by the live 62-04 SC4/SC5 UAT:
- After a `--block-net` run, confirm `NONO_SUBLAYER_GUID` exists (e.g. `netsh wfp show sublayers`).
- `msiexec /x` the machine MSI → confirm NO nono filters AND NO `NONO_SUBLAYER_GUID` sublayer remain.
- Confirm uninstall SUCCEEDS even if purge is forced to fail (e.g. BFE stopped).

## Deviations from Plan

None — plan executed exactly as written.

## Verification Results

### Automated (Windows host)

| Check | Result |
|-------|--------|
| `cargo build --release -p nono-cli --bin nono-wfp-service` | PASS (exit 0, 49.86s) |
| `cargo build -p nono-cli` | PASS (exit 0) |
| `cargo test -p nono-cli uninstall` | PASS — 5/5 (4 existing + 1 new fail-open) |
| `cargo test -p nono-cli purge` | PASS — 3/3 (1 network.rs + 2 service binary) |
| `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | PASS (clean) |

### Cross-Target Clippy (Unix)

**PARTIAL — deferred to CI.** `nono-wfp-service.rs` and `exec_strategy_windows/` are `#[cfg(target_os = "windows")]`-gated and do not compile under `x86_64-unknown-linux-gnu` or `x86_64-apple-darwin`. Windows-host `cargo clippy` exercises all Windows-gated branches. Cross-target verification deferred per CLAUDE.md and `.planning/templates/cross-target-verify-checklist.md`.

### WiX Non-Change Confirmation

`scripts/build-windows-msi.ps1` was NOT modified (`git diff HEAD -- scripts/build-windows-msi.ps1` produces no output). The purge rides the existing `CaUninstallWfpServices` custom action unchanged.

### Grep Verification

- `--purge-wfp-objects` appears in both `nono-wfp-service.rs` (const + dispatch + tests) and `network.rs` (comment + arg pass).
- `FwpmSubLayerDeleteByKey0` appears in `nono-wfp-service.rs` (import + call + SAFETY comment).
- `run_startup_sweep` and `run_purge_wfp_objects` both call `purge_nono_filters`.

## Threat Coverage

| Threat ID | Disposition |
|-----------|-------------|
| T-62-20 (leave-behind) | MITIGATED: `--purge-wfp-objects` deletes all NONO_SUBLAYER_GUID filters + the sublayer; idempotent (NOT_FOUND = success). Closes SC4/SC5 gap from 62-09. |
| T-62-24 (uninstall brick) | MITIGATED: FAIL-OPEN; purge Err logged but never aborts; WiX `Return=ignore`. |
| T-62-25 (over-broad delete) | MITIGATED: enumeration scoped to NONO_SUBLAYER_GUID via the shared purge_nono_filters helper; sublayer deleted by nono's own GUID key; zero-key skip inherited from startup sweep. |

## Self-Check: PASSED

- `crates/nono-cli/src/bin/nono-wfp-service.rs` — modified, exists
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — modified, exists
- Commit `650b668d` — verified in git log
- Commit `df124ca9` — verified in git log
