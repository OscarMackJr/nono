---
slug: wfp-wrong-session-dynamic
status: root_cause_found
trigger: "FwpmFilterAdd0 fails FWP_E_WRONG_SESSION (0x8032000C) because the WFP sublayer and filters are created in SEPARATE FWPM_SESSION_FLAG_DYNAMIC sessions. Surfaced during 62-04 HUMAN-UAT after 62-07/62-08 fixed the marshaling (1783) errors. The hash of the running nono-wfp-service.exe is confirmed = E856DED4... (the fixed binary)."
created: 2026-06-02
updated: 2026-06-02
phase: 62
requirements:
  - REQ-WFP-01
---

# Debug: FWP_E_WRONG_SESSION (dynamic-session sublayer/filter mismatch) + persistent-cleanup audit

## Symptoms

**Expected:** Hash-confirmed fixed service (E856DED4...) running, AUTO_START + RUNNING. A non-elevated `nono run --block-net` installs the kernel WFP block filter and blocks the confined child's outbound TCP (REQ-WFP-01 SC1).

**Actual (live Win11, 2026-06-02):** FAILS CLOSED:
```
nono: Platform not supported: Windows WFP service could not install its network-policy filtering probe: ... failed to install WFP network-policy filter (win32 status 2150760460, 0x8032000c)
```
0x8032000C = FWP_E_WRONG_SESSION. (Progress: past the 1783/RPC_X_BAD_STUB_DATA marshaling errors — the filter struct now marshals; WFP now evaluates and rejects the cross-session sublayer reference.)

**Ground truth:** Installed `C:\Program Files\nono\nono-wfp-service.exe` SHA256 = E856DED4F33D2AA115BB8966DF954BA815EA2E6D2028AB8A6136772997C87CD6 (the 62-07+62-08 binary), `sc qc` START_TYPE=2 AUTO_START, `sc query` STATE=4 RUNNING.

## Leading root cause (CONFIRMED)

`open_wfp_engine` (crates/nono-cli/src/bin/nono-wfp-service.rs L1170-1188) sets `session.flags = FWPM_SESSION_FLAG_DYNAMIC` (L1172). Objects added in a WFP dynamic session are PRIVATE to that session and auto-deleted when its engine handle closes.
- `run_named_pipe_server` (L562-563) opens dynamic session #1 (held for the service lifetime) and `create_nono_sublayer(&engine)` adds NONO_SUBLAYER_GUID IN session #1.
- `install_wfp_policy_filters` (L1450) opens a SEPARATE engine via `open_wfp_engine()` → dynamic session #2 per request, begins a transaction, and `add_policy_filter` (L1400) adds filters with `subLayerKey = NONO_SUBLAYER_GUID`.
- Session #2's `FwpmFilterAdd0` (L1415) references session #1's dynamic sublayer → FWP_E_WRONG_SESSION (object exists but belongs to a different dynamic session).

## Resolution

### Confirmed root cause
The failure is the cross-dynamic-session sublayer reference, exactly as hypothesized. Two distinct `FwpmEngineOpen0` calls each pass `FWPM_SESSION_FLAG_DYNAMIC`:
- Engine #1 (L562, long-lived in `run_named_pipe_server`) creates `NONO_SUBLAYER_GUID`. Because the session is dynamic, that sublayer is a **private, session-scoped object** of engine #1.
- Engine #2 (L1450, per-request in `install_wfp_policy_filters`) calls `FwpmFilterAdd0` with `filter.subLayerKey = NONO_SUBLAYER_GUID` (L1400). WFP finds the sublayer but sees it belongs to a *different* session → rejects with `FWP_E_WRONG_SESSION` (0x8032000C).

### Other 0x8032000C causes RULED OUT
- **Read-only transaction:** `WfpTransaction::begin` calls `FwpmTransactionBegin0(engine.0, 0)` (L917) — flags=0 = read-write. Not the cause.
- **Sublayer-not-found:** that would be `FWP_E_SUBLAYER_NOT_FOUND` (0x80320007), a different code. The sublayer DOES exist (in session #1), hence WRONG_SESSION rather than NOT_FOUND. Confirms cross-session, not absence.
- **Weight / filter conditions:** those produce distinct error codes (the 1783/RPC marshaling class was the prior, already-fixed failure). The struct now marshals and reaches WFP evaluation.

### Correct session model: PERSISTENT
Dropping `FWPM_SESSION_FLAG_DYNAMIC` at `open_wfp_engine` L1172 (leaving `session.flags = 0`) makes every object PERSISTENT (process-independent, surviving engine-handle close). Then:
- The sublayer created by engine #1 persists in the kernel, visible to any later engine handle.
- Filters added by engine #2 (and removed by other per-request engines) all reference the same persistent sublayer → no WRONG_SESSION.
This is consistent with the whole architecture, which only makes sense for persistent objects: a startup orphan sweep, deterministic-key remove-by-IPC, and an MSI-uninstall custom action would all be pointless for auto-cleaning dynamic objects.

### CLEANUP AUDIT (the gate) — per-path verdict

**(a) `run_startup_sweep` (L233-400): COVERS filters, leaves sublayer (acceptable-but-noted).**
- Enumerates ALL filters (`FwpmFilterEnum0`), filters to `NONO_SUBLAYER_GUID` by `subLayerKey` (L331-337), skips zero-key/system filters (fail-secure, L341-353), and deletes each via `FwpmFilterDeleteByKey0` (L357). Treats `FWP_E_FILTER_NOT_FOUND` as success. So under PERSISTENT it DOES remove orphaned filters from a prior crash. COVERS filters.
- It does NOT delete the sublayer. Under the *current* DYNAMIC model the sublayer never survives a crash (engine-close auto-deletes it), so the sweep's `FwpmSubLayerGetByKey0` existence check (L256-266) short-circuits to a clean summary. Under PERSISTENT the sublayer WILL survive — the sweep will find it, sweep its filters, and leave the empty sublayer in place (intentional: it is reused for the service's lifetime). A persistent empty sublayer is a benign reusable container, not an active filter — but see (b)/(c) for the leave-nothing implication.

**(b) `remove_wfp_policy_filters` (L1488-1516): COVERS filters by key, intentionally leaves the sublayer.**
- Opens its own engine, begins a read-write transaction, deletes each spec filter by deterministic key (L1498), commits. Removes the session's filters cleanly. The sublayer is deliberately retained for reuse across sessions. Within a running-service lifetime this is correct (no per-session sublayer churn). The leftover sublayer is NOT flagged by `netsh wfp show filters` (that lists *filters*, of which there are none after remove); it would only appear under `netsh wfp show sublayers`. Whether SC4 cares depends on whether Phase 53's leave-nothing check inspects sublayers — see (c).

**(c) MSI uninstall → `nono.exe setup --uninstall-wfp` → `uninstall_windows_wfp` (network.rs L1300): GAP under PERSISTENT.**
- The chain is: `dist/windows/nono-machine.wxs` `CaUninstallWfpServices` (L25-33, `ExeCommand="nono.exe setup --uninstall-wfp"`, deferred, Before=RemoveFiles, on REMOVE=ALL) → `setup.rs::uninstall_windows_wfp` (L232) → `exec_strategy::uninstall_windows_wfp` → `uninstall_windows_wfp_with_runner` (network.rs L1262).
- That function ONLY does `sc stop` + `sc delete` for the user-mode service and the kernel driver (`remove_single_windows_service`). It NEVER opens a WFP engine, NEVER deletes filters, and NEVER deletes the sublayer.
- Under the *current* DYNAMIC model this was sufficient: stopping the service closes engine #1, auto-deleting the sublayer; any in-flight dynamic filters die with their per-request engines. **Under PERSISTENT, stopping/deleting the service leaves the persistent `NONO_SUBLAYER_GUID` sublayer (and any filters that outlived a crash) in the kernel after `msiexec /x`.** That is a genuine leave-behind versus the Phase 53 / REQ-DRN-01 "leave nothing" invariant. `FwpmSubLayerDeleteByKey0` (or `FwpmSubLayerDeleteByKey`) appears NOWHERE in the codebase — nothing ever deletes the sublayer. **This is the GAP.**

**(d) Service CRASH with persistent filters: COVERED by the startup sweep; transient gap is fail-secure.**
- On crash, persistent filters survive. They are BLOCK/deny filters (fail-closed enforcement), so a surviving filter over-restricts rather than under-restricts — no security weakening during the gap. On next service start, `run_startup_sweep` runs *before* the pipe server accepts requests (L557-560) and removes all orphaned nono filters. So crash recovery is sound. The only residue is the empty persistent sublayer (benign; folds into the (c) GAP at uninstall time, not a runtime hazard).

### Scoped fix recommendation
**flag + cleanup addition (NOT flag-only). Classify: SMALL, but requires one cleanup addition.**
1. Remove `FWPM_SESSION_FLAG_DYNAMIC` at L1172 (set `session.flags = 0`) — the one-line core fix.
2. Add a sublayer-deletion to the UNINSTALL path so `msiexec /x` truly leaves nothing: have `nono.exe setup --uninstall-wfp` (or the service binary it can invoke) open a WFP engine, sweep+delete any remaining `NONO_SUBLAYER_GUID` filters, then `FwpmSubLayerDeleteByKey0(engine, &NONO_SUBLAYER_GUID)`. This is the gap closer for REQ-DRN-01.
   - Note: `uninstall_windows_wfp_with_runner` is pure `sc` plumbing in nono-cli; the WFP-object deletion logic (engine open + filter sweep + sublayer delete) already exists in the SERVICE binary (`run_startup_sweep` + a needed `FwpmSubLayerDeleteByKey0`). Cleanest design: add a `--purge-wfp-objects` (or reuse a one-shot mode) on `nono-wfp-service.exe` that the uninstall custom action invokes BEFORE `sc delete`, OR port the engine/sweep/sublayer-delete into the CLI uninstall path. Either way it is a bounded addition.
3. (Optional hardening) Have the startup sweep / per-session remove leave the sublayer as-is during normal operation (correct), and only delete the sublayer in the dedicated uninstall/purge path — so runtime sessions keep reusing one sublayer while uninstall guarantees zero residue.

**Implementation cost note:** fix is the SERVICE binary (+ the uninstall CLI/custom-action). Requires a service rebuild + version-bumped MSI reinstall + a re-run of the live UAT (incl. an `msiexec /x` + `netsh wfp show sublayers`/`show filters` leave-nothing check for SC4).

## Current Focus

- hypothesis: CONFIRMED — FWPM_SESSION_FLAG_DYNAMIC makes the sublayer (session #1) and filters (session #2) live in different private sessions → FWP_E_WRONG_SESSION. Correct model is PERSISTENT.
- next_action: DIAGNOSIS COMPLETE (find_root_cause_only). No fix applied. Recommended: flag removal + a sublayer-deletion added to the uninstall path (gap (c)) to preserve leave-nothing.

## Evidence

- timestamp: 2026-06-02 — Live Win11, hash-confirmed fixed binary: FwpmFilterAdd0 returns 0x8032000C FWP_E_WRONG_SESSION; confined child never ran; fail-closed.
- timestamp: 2026-06-02 — Static: open_wfp_engine L1172 FWPM_SESSION_FLAG_DYNAMIC; sublayer created in run_named_pipe_server engine #1 (L562-563); filters added in install_wfp_policy_filters engine #2 (L1450) referencing NONO_SUBLAYER_GUID (L1400) → separate dynamic sessions.
- timestamp: 2026-06-02 — Static: install (L1450) and remove (L1493) each call open_wfp_engine() → distinct per-request dynamic sessions, neither is engine #1. add_policy_filter (L1415) FwpmFilterAdd0 with subLayerKey=NONO_SUBLAYER_GUID.
- timestamp: 2026-06-02 — Static: WfpTransaction::begin uses FwpmTransactionBegin0(handle, 0) = read-write (L917); rules out read-only-transaction WRONG_SESSION.
- timestamp: 2026-06-02 — Static: run_startup_sweep (L233-400) enumerates+deletes filters under NONO_SUBLAYER_GUID but never deletes the sublayer. Existence pre-check L256-266.
- timestamp: 2026-06-02 — Static: remove_wfp_policy_filters (L1488) deletes filters by deterministic key, retains the sublayer for reuse.
- timestamp: 2026-06-02 — Static: uninstall chain wxs CaUninstallWfpServices (L25-33) → setup.rs uninstall_windows_wfp (L232) → network.rs uninstall_windows_wfp_with_runner (L1262): only `sc stop`/`sc delete` of service+driver; NO WFP engine open, NO filter/sublayer delete.
- timestamp: 2026-06-02 — Static: cleanup_stale_network_enforcement_artifacts (network.rs L176) only removes legacy netsh advfirewall rules + temp staging dirs; no WFP objects.
- timestamp: 2026-06-02 — Static: FwpmSubLayerDeleteByKey0 appears NOWHERE in crates/. Nothing ever deletes NONO_SUBLAYER_GUID. GAP for PERSISTENT model at uninstall.

## Eliminated

- Read-only transaction as the WRONG_SESSION cause — FwpmTransactionBegin0 flags=0 (read-write).
- Sublayer-not-found — that is FWP_E_SUBLAYER_NOT_FOUND (0x80320007); sublayer DOES exist (in session #1), hence WRONG_SESSION.
- Weight/filter-condition marshaling — that class was the already-fixed 1783/RPC_X_BAD_STUB_DATA failure; struct now marshals and reaches WFP evaluation.
- Crash-recovery as a security hazard — surviving filters are BLOCK (fail-closed); startup sweep removes them before serving requests.
