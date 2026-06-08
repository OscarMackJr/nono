---
phase: 64-minifilter-spike-implementation-macos-p1-cherry-pick-wave
plan: "02"
subsystem: kernel-driver
tags: [windows, minifilter, kernel, ipc, wdk, c-driver]
dependency_graph:
  requires: []
  provides:
    - drivers/nono-fltmgr/nono-fltmgr.h (NONO_IPC_REQUEST, NONO_IPC_REPLY, _Static_assert 532)
    - drivers/nono-fltmgr/nono-fltmgr.c (NonoPreCreate, NonoWorkerThread, FltCreateCommunicationPort)
  affects:
    - crates/nono-fltmgr-client/src/lib.rs (Plan 03 Rust user-mode client mirrors NONO_IPC_REQUEST)
    - drivers/nono-fltmgr/nono-fltmgr.vcxproj (builds extended .c + new .h on VM)
tech_stack:
  added: []
  patterns:
    - ring-buffer + worker-thread kernel IPC (DESIGN.md contract)
    - FltCreateCommunicationPort / FltSendMessage kernel↔user pattern
    - ExAllocatePool2(POOL_FLAG_NON_PAGED_NX) for callback-reachable allocations
    - _Static_assert C11 compile-time layout assertion
key_files:
  created:
    - drivers/nono-fltmgr/nono-fltmgr.h
  modified:
    - drivers/nono-fltmgr/nono-fltmgr.c
decisions:
  - "Single-slot ring buffer with KSPIN_LOCK + KEVENT chosen over true circular buffer (RESEARCH.md Open Question 2: sufficient for max-1-connection spike)"
  - "FltBuildDefaultSecurityDescriptor with FLT_PORT_ALL_ACCESS used for spike port ACL (Pitfall F prevention; production will scope to supervisor SID)"
  - "All ZwCreateFile/NtCreateFile/ExAllocatePoolWithTag references are comment-only documentation of the prohibition; zero actual API calls"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-08"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 1
---

# Phase 64 Plan 02: nono-fltmgr IPC Extension Summary

**One-liner:** Kernel minifilter extended with IRP_MJ_CREATE pre-create callback + single-slot ring buffer + 500ms-timeout FltSendMessage worker thread + \NonoPolicyPort FilterCommunicationPort opening in DriverEntry.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create nono-fltmgr.h with NONO_IPC_REQUEST / NONO_IPC_REPLY + _Static_assert | f17fa30f | drivers/nono-fltmgr/nono-fltmgr.h (created) |
| 2 | Extend nono-fltmgr.c with pre-create callback, ring buffer, worker thread, FltCreateCommunicationPort | d7fa2e25 | drivers/nono-fltmgr/nono-fltmgr.c (modified) |

## What Was Built

### Task 1: nono-fltmgr.h (new file)

Created the shared IPC struct header defining:

- `NONO_IPC_REQUEST` with `#pragma pack(push, 1)`: `WCHAR PathBuffer[260]` (520 bytes) + `ULONG ProcessId` (4 bytes) + `ACCESS_MASK DesiredAccess` (4 bytes) + `ULONG Reserved` (4 bytes) = **532 bytes total**
- C11 `_Static_assert(sizeof(NONO_IPC_REQUEST) == 532, "NONO_IPC_REQUEST layout changed")` matching the Plan 01 Rust-side assertion: `size_of::<NonoIpcRequest>() - size_of::<FILTER_MESSAGE_HEADER>() == 532`
- `NONO_IPC_REPLY` with `ULONG Decision` (0=allow, 1=deny)
- C_ASSERT fallback documented in comments for WDK toolchains without C11 `_Static_assert`
- No function declarations (kept in .c per task spec)

### Task 2: nono-fltmgr.c (extended in-place from Phase 63 skeleton)

Extended the 80-line Phase 63 skeleton to a fully-wired minifilter (~600 lines). All Phase 63 code preserved; additions:

**Globals added:**
- `PFLT_PORT gServerPort / gClientPort`: communication port handles
- `NONO_RING_ENTRY g_RingEntry` + `KSPIN_LOCK g_RingLock` + `KEVENT g_RingBufferEvent`: single-slot ring buffer
- `BOOLEAN gWorkerRunning` + `HANDLE gWorkerThreadHandle`: worker thread lifecycle

**NonoPreCreate (new):**
- `NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)` at entry (DESIGN.md T-63-03)
- `FltGetFileNameInformation` + `FltParseFileNameInformation` for normalized path
- `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, sizeof(NONO_IPC_REQUEST), 'onoN')` — zero-initialized by WDK guarantee
- Path copy bounded to 259 WCHARs + null terminator (DESIGN.md T-63-04)
- Spinlock-protected enqueue to single-slot ring buffer; fail-open on back-pressure
- `KeSetEvent` to wake worker thread
- Returns `FLT_PREOP_PENDING` (never `FLT_PREOP_COMPLETE`)

**NonoWorkerThread (new):**
- `KeWaitForSingleObject` loop at PASSIVE_LEVEL
- Spinlock-protected dequeue; lock released BEFORE `FltSendMessage` (DESIGN.md Rule 5)
- `timeout.QuadPart = -5000000LL` (500ms relative timeout — DESIGN.md T-63-02)
- `FltSendMessage(gFilterHandle, &gClientPort, pRequest, sizeof(NONO_IPC_REQUEST), &reply, &replyLen, &timeout)`
- `STATUS_TIMEOUT` or non-success → `STATUS_SUCCESS` (fail-open; spike policy)
- `reply.Decision == 1` → `STATUS_ACCESS_DENIED`; else `STATUS_SUCCESS`
- `ExFreePoolWithTag(pRequest, 'onoN')` then `FltCompletePendingPreOp`
- `PsTerminateSystemThread(STATUS_SUCCESS)` at loop exit

**Port callbacks (new):** `NonoPortConnect` (saves `gClientPort`), `NonoPortDisconnect` (`FltCloseClientPort`), `NonoPortMessage` (no-op return `STATUS_SUCCESS`)

**NonoInstanceTeardownStart (new):** Closes server port + drains ring buffer with fail-open IRP completion (Pitfall A: IRP-leak prevention)

**Callbacks[] (extended):** `{ IRP_MJ_CREATE, 0, NonoPreCreate, NULL }` added before `IRP_MJ_OPERATION_END` sentinel

**FilterRegistration (extended):** `InstanceTeardownStartCallback = NonoInstanceTeardownStart`

**NonoFltUnload (extended):** Signals worker stop + `ObReferenceObjectByHandle` wait + `ZwClose` + `FltCloseCommunicationPort` + `FltCloseClientPort` BEFORE `FltUnregisterFilter`

**DriverEntry (extended):** `KeInitializeEvent` + `KeInitializeSpinLock` + `RtlZeroMemory` + `FltBuildDefaultSecurityDescriptor(FLT_PORT_ALL_ACCESS)` + `FltCreateCommunicationPort(\NonoPolicyPort, maxConn=1)` + `PsCreateSystemThread(NonoWorkerThread)` — all after `FltStartFiltering` success

## Verification Results

All plan verification gates passed:

| Gate | Check | Result |
|------|-------|--------|
| 1 | `nono-fltmgr.h` exists with `_Static_assert(sizeof(NONO_IPC_REQUEST) == 532)` | PASS |
| 2 | `nono-fltmgr.c` includes `nono-fltmgr.h` | PASS |
| 3 | `grep -c "ZwCreateFile\|NtCreateFile" nono-fltmgr.c` == 0 actual calls | PASS (4 matches are comment-only) |
| 4 | `grep -c "ExAllocatePoolWithTag" nono-fltmgr.c` == 0 actual calls | PASS (1 comment-only mention) |
| 5 | `FltSendMessage` appears in `NonoWorkerThread` | PASS (line 304) |
| 6 | `-5000000LL` appears adjacent to `FltSendMessage` | PASS (line 299, 5 lines before the call) |
| 7 | All required functions defined: NonoPreCreate, NonoWorkerThread, NonoPortConnect, NonoPortDisconnect, NonoPortMessage | PASS |
| 8 | `FltCreateCommunicationPort` called in DriverEntry after `FltStartFiltering` | PASS (line 565) |

BSOD-avoidance contract satisfied:
- Zero actual calls to `ZwCreateFile`, `NtCreateFile`, `ZwReadFile`, `NtReadFile`, or `ExAllocatePoolWithTag` in the driver source (all occurrences are in `//` comments documenting the prohibition)
- `FltSendMessage` uses `timeout.QuadPart = -5000000LL` (500ms); NULL timeout not used anywhere
- All callback-reachable allocations use `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, ...)`
- `NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)` at `NonoPreCreate` entry
- Spinlock released BEFORE `FltSendMessage` (DESIGN.md Rule 5)
- `FLT_PREOP_PENDING` returned (not `FLT_PREOP_COMPLETE`)

## Deviations from Plan

None — plan executed exactly as written.

The `ObReferenceObjectByHandle` + `KeWaitForSingleObject` pattern for worker thread join in `NonoFltUnload` is an elaboration of the plan's `KeWaitForSingleObject(gWorkerThreadHandle, ...)` directive. The plan called `KeWaitForSingleObject` directly on `gWorkerThreadHandle` (a `HANDLE`); the WDK requires converting the handle to a `PETHREAD` kernel object pointer first via `ObReferenceObjectByHandle`. This is the correct WDK pattern, not a deviation.

## Known Stubs

None. All code paths complete pending IRPs (fail-open for spike). The single-slot ring buffer and fail-open-on-STATUS_TIMEOUT policy are intentional spike design choices, not stubs — documented in DESIGN.md and RESEARCH.md.

## Threat Flags

The `\NonoPolicyPort` FilterCommunicationPort trust boundary is part of the plan's documented threat model (T-63-01..T-63-04). No new threat surface beyond what the plan anticipated.

| Flag | File | Description |
|------|------|-------------|
| threat_flag: communication-port-acl | drivers/nono-fltmgr/nono-fltmgr.c | `FltBuildDefaultSecurityDescriptor(FLT_PORT_ALL_ACCESS)` allows any user-mode process to connect — acceptable for test-VM-only spike (RESEARCH.md Pitfall F); production must scope SD to supervisor process SID |

## Self-Check: PASSED

- `drivers/nono-fltmgr/nono-fltmgr.h` exists: CONFIRMED
- `drivers/nono-fltmgr/nono-fltmgr.c` modified: CONFIRMED
- Commit `f17fa30f` (Task 1): CONFIRMED
- Commit `d7fa2e25` (Task 2): CONFIRMED
