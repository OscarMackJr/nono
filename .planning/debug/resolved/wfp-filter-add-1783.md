---
slug: wfp-filter-add-1783
status: resolved
resolution: "FIXED in plan 62-07 (2026-06-03): the per-SID block-filter SD is wrapped in FWP_BYTE_BLOB so FwpmFilterAdd0 marshals correctly (no more 1783 RPC_X_BAD_STUB_DATA). Verified by the working WFP block on live Win11 (62-HUMAN-UAT.md SC1, exit 6). Threat T-62-17 CLOSED in 62-SECURITY.md."
trigger: "FwpmFilterAdd0 fails with win32 1783 (RPC_X_BAD_STUB_DATA) when nono-wfp-service installs the network-policy block filter. Surfaced during 62-04 HUMAN-UAT after the 62-06 driver-gate fix unblocked the service activation path."
created: 2026-06-02
updated: 2026-06-02
phase: 62
requirements:
  - REQ-WFP-01
---

# Debug: FwpmFilterAdd0 fails 1783 (RPC_X_BAD_STUB_DATA) installing the block filter

> **RESOLVED 2026-06-03.** Fixed in plan 62-07 — the filter-condition SD is now wrapped in `FWP_BYTE_BLOB` so `FwpmFilterAdd0` marshals correctly. Confirmed by the working live-Win11 WFP block (62-HUMAN-UAT.md SC1). Threat T-62-17 CLOSED.

## Symptoms

**Expected:** With the 62-06 driver-gate fix, a non-elevated supervised `nono run --block-net` (machine MSI `nono-v0.57.5-wfp62.2`, nono-wfp-service RUNNING start=auto) reaches the service's WFP activation path and the service installs a kernel WFP block filter, enforcing on the confined child (REQ-WFP-01 SC1).

**Actual (live Win11, 2026-06-02):** The run reaches the service activation IPC (driver gate is gone — progress), then FAILS CLOSED:
```
nono: Platform not supported: Windows WFP service could not install its network-policy filtering probe: request activate_blocked_mode for blocked Windows network access could not install the backend-owned network-policy probe: failed to install WFP network-policy filter (win32 status 1783, 0x000006f7)
```
Fail-secure holds (no unenforced pass-through). But enforcement does not activate, so SC1 cannot pass.

**Error:** win32 1783 = 0x6F7 = RPC_X_BAD_STUB_DATA, returned by `FwpmFilterAdd0` (the WFP engine runs in the BFE service; filter structs are marshaled over RPC, so a malformed/invalid required field fails the stub).

**Timeline:** Exposed 2026-06-02 once 62-06 removed the client driver-gate that previously masked this (the filter-install code was never reached before). The service binary (nono-wfp-service.rs) was NOT changed by 62-06 — this is a pre-existing bug in the filter construction.

**Reproduction:** Machine MSI (wfp62.2) installed + service RUNNING; non-elevated:
`nono run --block-net --allow-cwd -- powershell -Command "try { (Invoke-WebRequest https://api.ipify.org -TimeoutSec 5).Content } catch { 'BLOCKED: ' + $_.Exception.Message }"`

## Leading hypothesis (CONFIRMED)

`add_policy_filter` (crates/nono-cli/src/bin/nono-wfp-service.rs L1374-1377) sets `filter.displayData.name = null_mut()`. WFP requires FWPM_DISPLAY_DATA0.name to be NON-NULL; a null required string is a classic cause of RPC_X_BAD_STUB_DATA (1783) from FwpmFilterAdd0. Dispositive in-file contrast: `create_nono_sublayer` (L1203-1207) builds the same FWPM_DISPLAY_DATA0 with a NON-null name and FwpmSubLayerAdd0 SUCCEEDS (sublayer is added before the filter).

## Investigation scope (full field audit — COMPLETED)

See `## Resolution` for the field-by-field findings.

## Current Focus

- hypothesis: CONFIRMED — filter.displayData.name = null_mut() violates WFP's non-null name requirement → FwpmFilterAdd0 returns RPC_X_BAD_STUB_DATA (1783).
- next_action: none — diagnosis complete (goal = find_root_cause_only). User authors a gap-closure plan.

## Evidence

- timestamp: 2026-06-02 — Live Win11 repro: FwpmFilterAdd0 returns win32 1783 (RPC_X_BAD_STUB_DATA) on the block-filter install; confined child never ran; fail-closed.
- timestamp: 2026-06-02 — Static: add_policy_filter L1374-1377 sets displayData.name = null_mut(); create_nono_sublayer L1203-1207 sets a non-null name and FwpmSubLayerAdd0 succeeds.
- timestamp: 2026-06-02 — Type verify (windows-sys 0.59 src): FWPM_DISPLAY_DATA0.name is PWSTR (required string, marshaled by the RPC stub). FWP_VALUE0_0.uint64 is `*mut u64` and FWP_CONDITION_VALUE0_0 members {sd, byteBlob, uint16, uint32} are the correct typed union members — all other fields well-formed.

## Eliminated

- weight FWP_VALUE0: `r#type=FWP_UINT64` with `uint64: &mut weight_value` is CORRECT — in windows-sys 0.59 FWP_VALUE0_0.uint64 is `*mut u64` (FWP_UINT64 is stored by pointer in FWP_VALUE0). `weight_value` is a function-local `let mut` that outlives the FwpmFilterAdd0 call. Not a defect.
- condition value-type vs field-key matches: ALE_USER_ID+FWP_SECURITY_DESCRIPTOR_TYPE(sd), ALE_APP_ID+FWP_BYTE_BLOB_TYPE(byteBlob), IP_REMOTE/LOCAL_PORT+FWP_UINT16(uint16), FLAGS+FWP_UINT32(uint32)+FWP_MATCH_FLAGS_ALL_SET — all type/member pairings correct. Not a defect.
- numFilterConditions/array: `conditions.len() as u32` paired with `conditions.as_mut_ptr()`; the Vec outlives the call. The block path always pushes ≥1 condition (sd or app_id, plus port/flags), so no empty-conditions concern. Not a defect.
- layerKey/subLayerKey: spec.layer_key is a real FWPM_LAYER_ALE_AUTH_* GUID; subLayerKey = NONO_SUBLAYER_GUID is added first via create_nono_sublayer (succeeds). Not a defect.
- action FWPM_ACTION0: BLOCK/PERMIT with filterType=zero_guid() is correct for non-callout actions (filterType only meaningful for CALLOUT actions). Not a defect.
- pointer/lifetime liveness across the call: app_id_blob/security_descriptor are owned by RAII guards held in install_wfp_policy_filters across the loop; conditions Vec and weight_value are function-locals in add_policy_filter; all outlive FwpmFilterAdd0. Not a defect (the only missing live pointer is the to-be-added name buffer).

## Resolution

**Root cause (CONFIRMED):** `add_policy_filter` passes `filter.displayData.name = null_mut()`. FWPM_DISPLAY_DATA0.name is a required PWSTR; FwpmFilterAdd0 marshals it over RPC to the BFE engine, and a NULL required string fails the RPC stub with RPC_X_BAD_STUB_DATA (win32 1783). The in-file sublayer (non-null name → FwpmSubLayerAdd0 succeeds) is the dispositive contrast, and the live Win11 repro returns exactly 1783.

**Field audit classification: SMALL.** The ONLY malformed field is `displayData.name` (NULL). Every other audited field — weight FWP_VALUE0 union member + lifetime, all four condition value-type/field-key pairings, numFilterConditions/array, layerKey/subLayerKey, action union, and pointer liveness of app_id_blob/security_descriptor/conditions/weight_value — is well-formed (see ## Eliminated). No additional 1783-class defect exists.

**Recommended minimal fix (NOT applied — diagnosis only):**
1. In `add_policy_filter`, build a non-null UTF-16 name buffer with `to_utf16_null(...)` (e.g. derived from spec/rule, mirroring create_nono_sublayer L1203-1207) and set `filter.displayData.name = name_wide.as_ptr() as *mut _`.
2. Bind that buffer to a `let` local BEFORE constructing `filter` so it outlives the FwpmFilterAdd0 call (lifetime is the one pointer-liveness gap noted above). Optionally also set a non-null `description`, but only `name` is required.

**Deployment note:** This is the SERVICE binary (nono-wfp-service.rs). A fix requires a service rebuild + MSI reinstall (machine MSI) and service restart — not just a nono.exe relink. Flag this in the gap-closure plan and re-run the live UAT repro to confirm SC1.

**fix:** not applied (goal = find_root_cause_only).
