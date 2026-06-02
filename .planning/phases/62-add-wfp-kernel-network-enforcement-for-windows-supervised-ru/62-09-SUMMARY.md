---
phase: 62
plan: "09"
subsystem: nono-wfp-service (Windows WFP kernel enforcement)
tags: [wfp, windows, kernel-enforcement, gap-closure, fix]
dependency_graph:
  requires: [62-08]
  provides: [persistent-wfp-session]
  affects: [REQ-WFP-01, REQ-DRN-01]
tech_stack:
  added: []
  patterns: [persistent-wfp-session]
key_files:
  modified:
    - crates/nono-cli/src/bin/nono-wfp-service.rs
decisions:
  - "Use session.flags = 0 (persistent WFP session): sublayer and filters must share the same WFP object namespace across engine handles; dynamic sessions make objects private and the cross-engine reference fails with FWP_E_WRONG_SESSION."
  - "Remove FWPM_SESSION_FLAG_DYNAMIC import: unused after the flag change; removing keeps -D warnings clean."
  - "Defer uninstall purge (FwpmSubLayerDeleteByKey0) to plan 62-10: scoped to unblocking SC1 only."
metrics:
  duration: "~10 minutes"
  completed: "2026-06-02"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 1
---

# Phase 62 Plan 09: Persistent WFP Session (F-62-UAT-04 Fix) Summary

**One-liner:** Drop FWPM_SESSION_FLAG_DYNAMIC so the WFP sublayer and per-request filters share a persistent object namespace, eliminating FWP_E_WRONG_SESSION (0x8032000C).

## What Was Done

### Task 1: Make the WFP session persistent (drop the dynamic flag)

**Root cause (confirmed, debug: wfp-wrong-session-dynamic):**

`open_wfp_engine` set `session.flags = FWPM_SESSION_FLAG_DYNAMIC`. WFP dynamic sessions scope every object added to that session as private — objects are invisible to other engine handles and are auto-deleted when the handle closes. The service architecture opens two distinct engines:

- **Engine #1** (long-lived, `run_named_pipe_server` L562-563): `create_nono_sublayer` adds `NONO_SUBLAYER_GUID` — a private object of this session.
- **Engine #2** (per-request, `install_wfp_policy_filters` L1450): `FwpmFilterAdd0` references `NONO_SUBLAYER_GUID` as `filter.subLayerKey` — but the sublayer belongs to engine #1's private session → WFP returns **FWP_E_WRONG_SESSION (0x8032000C)**.

**Fix (two lines):**

1. `crates/nono-cli/src/bin/nono-wfp-service.rs` L1172: Changed `session.flags = FWPM_SESSION_FLAG_DYNAMIC` to `session.flags = 0` with an explanatory comment. (`zeroed()` already sets this to zero; the explicit assignment makes the intent visible.)

2. Import block L433: Removed `FWPM_SESSION_FLAG_DYNAMIC` from the `windows_sys` `use` block — it was the only use in the file; removing it keeps the build warning-clean under `-D warnings`.

No other behaviour changed: `create_nono_sublayer` (FWP_E_ALREADY_EXISTS-tolerant), `run_startup_sweep`, `install_wfp_policy_filters`, and `remove_wfp_policy_filters` are untouched.

**Commit:** `df823d10`

## Verification

### Automated (Windows-host)

- `cargo build --release -p nono-cli --bin nono-wfp-service` — exits 0 (5.25 s).
- `cargo clippy -p nono-cli --bin nono-wfp-service -- -D warnings -D clippy::unwrap_used` — clean (1.28 s, no unused-import warning).
- Grep confirms `FWPM_SESSION_FLAG_DYNAMIC` no longer appears anywhere in `nono-wfp-service.rs`.
- Grep confirms `session.flags = 0` is present in `open_wfp_engine`.

### Cross-target Linux/macOS clippy

PARTIAL / deferred to CI per CLAUDE.md. `nono-wfp-service.rs` is a Windows-cfg-gated binary; the Windows host cannot exercise Unix cfg branches. The cross-target check is scheduled to run automatically when `main` is pushed to origin via `release.yml`.

### FFI unit-test limitation

`open_wfp_engine` calls `FwpmEngineOpen0` via `unsafe` FFI into BFE (Base Filtering Engine). The BFE is a SYSTEM service that requires an elevated host with WFP available. Unit tests in the Cargo test runner cannot exercise this path without real elevation and a running BFE instance. **The acceptance proof is the live SC1 re-run in 62-04:** the operator rebuilds the MSI off this commit, reinstalls, and re-runs SC1. If `FwpmFilterAdd0` now returns 0 (no FWP_E_WRONG_SESSION) and the confined child's outbound TCP is blocked, the fix is verified.

### Live UAT acceptance

Pending operator re-run of 62-04 SC1 after MSI rebuild from this commit:
- Rebuild machine MSI off updated `nono-wfp-service.exe`.
- `msiexec /i nono-machine-<ver>.msi /quiet`.
- `nono run --block-net -- curl https://example.com` from PowerShell (elevated for WFP policy install).
- Expected: `FwpmFilterAdd0` returns 0; curl exits with connection refused / timeout ("BLOCKED:"), not "Platform not supported: ... FWP_E_WRONG_SESSION".

## Deviations from Plan

None — plan executed exactly as written.

## REQUIRED Follow-up: Plan 62-10 (uninstall purge, REQ-DRN-01)

**This is a known, accepted gap introduced by this plan and MUST be closed before SC4.**

Under the dynamic model, stopping the service auto-deleted all WFP objects (the engine handle close was the cleanup). Under the persistent model, WFP objects survive engine-handle close — by design. However, `msiexec /x` currently only runs `sc stop` + `sc delete` via `uninstall_windows_wfp_with_runner`; it never opens a WFP engine and never deletes `NONO_SUBLAYER_GUID` or its filters.

**Impact:** After `msiexec /x`, a persistent `NONO_SUBLAYER_GUID` sublayer (and any crash-surviving BLOCK filters) remain in the kernel. This violates the REQ-DRN-01 "leave nothing" invariant verified in SC4.

**Mitigation during gap:** crash-surviving filters are BLOCK (fail-closed), so the residue over-restricts rather than under-restricts — no security weakening. The startup sweep (`run_startup_sweep`) removes orphaned filters before the service accepts requests. The sublayer is an empty, benign container after removal of its filters.

**Plan 62-10** will add `FwpmSubLayerDeleteByKey0` (and a final filter sweep) to the uninstall path before SC4.

## Known Stubs

None.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes. The session-flag change affects only the WFP object lifetime scope; all existing cleanup paths (startup sweep, remove-by-key) continue to function correctly under the persistent model.

## Self-Check: PASSED

- [x] `crates/nono-cli/src/bin/nono-wfp-service.rs` modified with `session.flags = 0`
- [x] `FWPM_SESSION_FLAG_DYNAMIC` removed from imports (0 occurrences in file)
- [x] Commit `df823d10` exists: `git log --oneline | head -1` = `df823d10 fix(62-09): make WFP session persistent to eliminate FWP_E_WRONG_SESSION`
- [x] Build exits 0; clippy clean
- [x] DCO sign-off present in commit
