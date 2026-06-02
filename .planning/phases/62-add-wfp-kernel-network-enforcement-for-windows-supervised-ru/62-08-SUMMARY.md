---
phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
plan: "08"
subsystem: infra
tags: [wfp, windows, kernel, network-enforcement, ffi, windows-sys]

requires:
  - phase: 62-07
    provides: "non-null displayData.name in add_policy_filter (first 1783 cause fixed)"

provides:
  - "FWP_BYTE_BLOB-wrapped self-relative SD for ALE_USER_ID condition in add_policy_filter"
  - "GetSecurityDescriptorLength import in nono-wfp-service Win32::Security block"

affects:
  - 62-04-HUMAN-UAT (SC1 re-run required after MSI rebuild)
  - phase-62-close-gate

tech-stack:
  added: []
  patterns:
    - "FWP_SECURITY_DESCRIPTOR_TYPE condition: always wrap SD in a function-scope FWP_BYTE_BLOB; never pass raw PSECURITY_DESCRIPTOR to FWP_CONDITION_VALUE0_0.sd"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/bin/nono-wfp-service.rs

key-decisions:
  - "sd_blob declared before conditions Vec so its pointer outlives the FwpmFilterAdd0 RPC call"
  - "GetSecurityDescriptorLength used (not a hardcoded size) because the self-relative SD length is not statically known"
  - "No FFI unit test added for FwpmFilterAdd0 path — requires elevated BFE host; live UAT is the acceptance proof"
  - "Cross-target Linux/macOS clippy PARTIAL — deferred to CI per CLAUDE.md (Windows-only cfg binary)"

patterns-established:
  - "FWP blob wrapping: declare lifetime-correct FWP_BYTE_BLOB local before any Vec/struct that stores pointers into it"

requirements-completed:
  - REQ-WFP-01

duration: 8min
completed: 2026-06-02
---

# Phase 62 Plan 08: F-62-UAT-03 Fix — ALE_USER_ID SD wrapped in FWP_BYTE_BLOB Summary

**FwpmFilterAdd0 RPC_X_BAD_STUB_DATA (win32 1783) eliminated by wrapping the self-relative SD in a function-scope FWP_BYTE_BLOB — windows-sys 0.59 types FWP_CONDITION_VALUE0_0.sd as `*mut FWP_BYTE_BLOB`, not a raw PSECURITY_DESCRIPTOR**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-06-02T00:00:00Z
- **Completed:** 2026-06-02
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added `GetSecurityDescriptorLength` to the `Win32::Security` import block in `nono-wfp-service.rs`
- Declared a function-scope `sd_blob: FWP_BYTE_BLOB` in `add_policy_filter` BEFORE the `conditions` Vec, so its pointer remains valid through the `FwpmFilterAdd0` RPC call
- Populated `sd_blob.size = GetSecurityDescriptorLength(security_descriptor)` and `sd_blob.data = security_descriptor as *mut u8` in the SD branch
- Changed `sd: security_descriptor as *mut _` (bug: raw SD) to `sd: &mut sd_blob` (correct: pointer to FWP_BYTE_BLOB wrapping the SD)
- All other conditions/fields untouched: app-id (byteBlob), port, flags, weight, displayData.name (62-07 fix), action, layerKey, subLayerKey

## windows-sys 0.59 Evidence

The type is dispositive. `windows-sys 0.59` (`Win32::NetworkManagement::WindowsFilteringPlatform`):

```rust
pub union FWP_CONDITION_VALUE0_0 {
    pub uint16: u16,
    pub uint32: u32,
    pub uint64: *mut u64,
    pub byteBlob: *mut FWP_BYTE_BLOB,
    pub sd: *mut FWP_BYTE_BLOB,   // <-- NOT *mut c_void
    // ...
}
```

WFP requires `FWP_SECURITY_DESCRIPTOR_TYPE` values to carry a pointer to an `FWP_BYTE_BLOB`
whose `data` points to a self-relative SD and `size` is its byte length. Passing a raw
`PSECURITY_DESCRIPTOR` makes the BFE RPC stub attempt to dereference the SD bytes as an
`FWP_BYTE_BLOB` header, producing malformed size/data fields — hence RPC_X_BAD_STUB_DATA (1783).

`ConvertStringSecurityDescriptorToSecurityDescriptorW` (the W variant) returns a self-relative SD,
so `GetSecurityDescriptorLength` is the correct size query.

This was the SECOND independent 1783 cause on the `--block-net` path. Plan 62-07 fixed the first
(null `displayData.name`). Both were pre-existing bugs that became reachable only after 62-06
removed the client driver-gate (ref: `.planning/debug/wfp-filter-add-1783.md`).

## Task Commits

1. **Task 1: Wrap ALE_USER_ID SD condition in FWP_BYTE_BLOB** - `19dcf3e5` (fix)

**Plan metadata:** (see state update commit)

## Files Modified

- `crates/nono-cli/src/bin/nono-wfp-service.rs` — `GetSecurityDescriptorLength` added to import; `sd_blob` local + `GetSecurityDescriptorLength` call + `sd: &mut sd_blob` in `add_policy_filter`

## Decisions Made

- **sd_blob declared before conditions Vec:** The blob's address is stored inside a `FWPM_FILTER_CONDITION0` that is used at the `FwpmFilterAdd0` call site. Declaring it first ensures it is not dropped before the call.
- **GetSecurityDescriptorLength not hardcoded:** The self-relative SD length is not known at compile time; using the Win32 API is the correct approach and matches WFP documentation.
- **No FFI unit test for FwpmFilterAdd0:** The function is an unsafe FFI call to the BFE engine over RPC. It cannot be unit-tested without: (a) an elevated process, (b) the BFE service running, (c) an installed WFP engine session. The existing `add_policy_filter` function has no unit test for this reason. Acceptance proof is the live 62-04 UAT re-run (SC1): FwpmFilterAdd0 returns 0, confined child runs, outbound TCP is blocked.

## Deviations from Plan

None — plan executed exactly as written. The fix matches the documented action in 62-08-PLAN.md verbatim.

## Cross-Target Clippy Note

Windows-host `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` is CLEAN.

Cross-target Linux/macOS clippy for this Windows-only cfg binary (`#[cfg(target_os = "windows")]`) is PARTIAL — deferred to CI per CLAUDE.md `Cross-target clippy verification` rule. The changed code is inside a `#[cfg(target_os = "windows")]` block and imports `windows_sys` APIs that have no Linux/macOS analogue; the Windows-host build is the authoritative verification path for this binary.

## Issues Encountered

None.

## Next Phase Readiness

- Service binary rebuilt (`cargo build --release -p nono-cli --bin nono-wfp-service` exits 0)
- Phase 62 close gate: orchestrator must rebuild the machine MSI off the updated `nono-wfp-service.exe` and operator must re-run 62-04 SC1 (`nono run --block-net --allow-cwd -- powershell -Command "try { ... } catch { 'BLOCKED: ...' }"`). Expected: `FwpmFilterAdd0` returns 0; confined child runs; outbound TCP blocked.
- After UAT PASS: Phase 62 can be closed and Phase 54 (UPST7 audit) proceeds.

## Self-Check: PASSED

- `crates/nono-cli/src/bin/nono-wfp-service.rs` — modified (verified by build)
- Commit `19dcf3e5` — exists (`git rev-parse --short HEAD` confirmed)
- `cargo build --release -p nono-cli --bin nono-wfp-service` — exits 0
- `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` — clean
- `GetSecurityDescriptorLength` in import: confirmed
- `sd: &mut sd_blob` (not `sd: security_descriptor as *mut _`): confirmed
- No other condition/field changed: confirmed

---
*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Completed: 2026-06-02*
