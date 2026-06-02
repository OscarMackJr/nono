---
phase: 62
plan: "07"
subsystem: nono-wfp-service
tags:
  - wfp
  - windows
  - gap-closure
  - security
dependency_graph:
  requires:
    - 62-06
  provides:
    - REQ-WFP-01 (unblocked)
  affects:
    - crates/nono-cli/src/bin/nono-wfp-service.rs
tech_stack:
  added: []
  patterns:
    - "Lifetime-correct UTF-16 PWSTR binding before FWPM struct construction (mirrors create_nono_sublayer)"
key_files:
  modified:
    - crates/nono-cli/src/bin/nono-wfp-service.rs
decisions:
  - "Changed exactly one field (displayData.name) as confirmed by the 7-point FWPM field audit; no other FWPM field touched"
  - "name_wide bound as a let-local BEFORE FWPM_FILTER0 construction to guarantee pointer liveness across FwpmFilterAdd0"
  - "description stays null_mut() — FWPM_DISPLAY_DATA0.description is optional; only name is required"
metrics:
  duration_minutes: 8
  completed: "2026-06-02"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 1
---

# Phase 62 Plan 07: Fix null displayData.name in add_policy_filter (F-62-UAT-02) Summary

## One-liner

Set a non-null UTF-16 display name in `add_policy_filter` so `FwpmFilterAdd0` no longer fails with `RPC_X_BAD_STUB_DATA` (win32 1783), unblocking WFP block-filter installation for REQ-WFP-01.

## What Was Built

Gap-closure for F-62-UAT-02 (root-caused in debug session `wfp-filter-add-1783`). A single field change in `add_policy_filter` (`crates/nono-cli/src/bin/nono-wfp-service.rs`):

1. Added `let name_wide = to_utf16_null(std::ffi::OsStr::new("nono Network Policy Filter"));` **before** `let mut filter: FWPM_FILTER0 = zeroed();` so the buffer's lifetime covers the entire `FwpmFilterAdd0` call.
2. Changed `filter.displayData.name` from `null_mut()` to `name_wide.as_ptr() as *mut _`.
3. Added a comment citing the root cause (`RPC_X_BAD_STUB_DATA` / win32 1783), the debug artifact (`wfp-filter-add-1783`), and the mirrored working pattern (`create_nono_sublayer`).

No other FWPM field was changed. The 7-point field audit in `wfp-filter-add-1783.md` confirmed that weight (`FWP_VALUE0` union + lifetime), all four condition value-type/field-key pairings (`ALE_USER_ID`/`ALE_APP_ID`/port/flags), `numFilterConditions`/array, `layerKey`/`subLayerKey`, action union, and pointer liveness of `app_id_blob`/`security_descriptor`/`conditions`/`weight_value` are all well-formed — `displayData.name = null_mut()` was the ONLY defect.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo build --release -p nono-cli --bin nono-wfp-service` | PASS (exits 0, 3.95 s) |
| `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | PASS (clean) |
| `grep 'name: null_mut()' nono-wfp-service.rs` | No matches — null name is gone |
| Cross-target Linux/macOS clippy | PARTIAL — deferred to CI (Windows-only `cfg` binary; see CLAUDE.md cross-target policy) |

## Unit-Test Limitation

`FwpmFilterAdd0` cannot be unit-tested without an elevated Windows host running the Base Filtering Engine (BFE) service. The existing `add_policy_filter` function has no unit test for this reason — it is all unsafe FFI against a kernel service. Fabricating a test that stubs or mocks the BFE would not meaningfully exercise the RPC marshaling path where this defect lives.

**Acceptance proof:** The live 62-04 HUMAN-UAT SC1 re-run after rebuilding + reinstalling the machine MSI (nono-wfp-service binary) is the definitive acceptance gate. The expected outcome is `FwpmFilterAdd0` returning 0 (success) instead of 1783, and the confined child's outbound TCP being blocked by the kernel WFP filter (BLOCKED:/timeout, not an IP).

## Deployment Notes

This fix is in the **nono-wfp-service binary**. Acceptance requires:
1. Rebuild the machine MSI (`scripts/build-windows-msi.ps1 --machine`) off this commit.
2. Stop the service: `sc stop nono-wfp-service` (elevated).
3. Reinstall: `msiexec /i nono-machine-*.msi /qn` (elevated).
4. Re-run the live UAT repro (62-04 SC1) from a non-elevated shell.

## Deviations from Plan

None — plan executed exactly as written.

## Threat Flags

No new threat surface introduced. The only change is the cosmetic display-name field; layer, sublayer, conditions, action, and weight are unchanged.

## Self-Check: PASSED

- Modified file: `crates/nono-cli/src/bin/nono-wfp-service.rs` — confirmed present and edited correctly.
- Commit `6115a3ec` exists: `git log --oneline | head -1` → `6115a3ec fix(62-07): set non-null displayData.name in add_policy_filter (F-62-UAT-02)`.
- Build: exits 0. Clippy: clean. Null-name grep: no matches.
