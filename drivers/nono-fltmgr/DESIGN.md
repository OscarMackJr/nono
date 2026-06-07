# nono-fltmgr Minifilter Spike: BSOD-Avoidance Pre-Code Gate

**Status:** Accepted (pre-code gate)
**Date:** 2026-06-06
**Phase:** 63 (v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity)
**Requirement:** DRV-03 (partial)
**Related ADR:** [Minifilter Spike Design Pointer](../../docs/architecture/minifilter-spike-design-pointer.md)

## Context

Phase 63 scaffolds the `drivers/nono-fltmgr/` WDK source skeleton — the compile-ready foundation
for the Gap 6b minifilter feasibility spike (Phase 64). A kernel minifilter driver runs in ring 0
and can BSOD or permanently hang the host machine if incorrectly written. Four failure modes are
known from prior art and PITFALLS research:

1. **Recursive file I/O BSOD (Pitfall 2):** A pre-create callback that opens a file via kernel
   I/O APIs triggers itself recursively until the kernel stack overflows.
2. **Infinite `FltSendMessage` hang (Pitfall 3):** A blocking, infinite-timeout send to user mode
   hangs every I/O request on the affected volume if the supervisor process exits or stalls.
3. **IRQL violation BSOD (Pitfall 1):** Memory allocation at the wrong IRQL, or taking a mutex
   across an `FltSendMessage` call, causes an immediate system check.
4. **Altitude collision with EDR (Pitfall 5):** Registering at an altitude in the AV/EDR range
   (320000–329998) can blind an EDR or fail the driver's own registration.

This document is the **hard pre-code gate (D-10)** that specifies the safety contract for every
line of Phase 64 driver code. No driver logic may be written until the implementer has read and
understood this document. The Phase 63 skeleton (`nono-fltmgr.c`) enforces the gate structurally:
it registers an EMPTY operation-callbacks array and performs no file I/O — code cannot be added
without an explicit Phase 64 task that references this contract.

## Threat Register (STRIDE)

| Threat ID | Pattern | STRIDE | Mitigation Required |
|-----------|---------|--------|---------------------|
| T-63-01 | Driver-originated recursive file I/O → stack-overflow BSOD | Denial of Service | No kernel file-open APIs (`ZwCreateFile` / `NtCreateFile`) anywhere in the driver. If internal I/O is ever unavoidable, use `FltCreateFile` on the driver's own minifilter instance only — this breaks the recursion. |
| T-63-02 | Infinite `FltSendMessage` → host hang | Denial of Service | `FltSendMessage` MUST use a finite timeout (~500 ms). On `STATUS_TIMEOUT`: fail-open (permit the I/O and log the miss). **Production ADR revisit:** fail-direction for a production driver is a separate decision; the spike uses fail-open to avoid locking the test VM during development. |
| T-63-03 | IRQL violation in callback → BSOD | Denial of Service | All callback-reachable memory allocations must use `NonPagedPoolNx`. Insert `NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)` at the entry of every callback that may block. No mutex held across an `FltSendMessage` call. |
| T-63-04 | Altitude in AV/EDR range → EDR blinded / registration failure | Tampering / Elevation | Use FSFilter Activity Monitor band (360000–389999). NEVER use AV range 320000–329998. Request an official altitude assignment from Microsoft (fsfcomm@microsoft.com) before any production use. |
| T-63-05 | Spike `.sys` leaking into main repo or MSI | Tampering | The spike `.sys` is a VM-local throwaway compile artifact. It is NOT committed, NOT bundled in the MSI. The existing `nono-wfp-driver.sys` placeholder in `crates/nono-cli/data/windows/` is a separate driver and must not be modified by this spike. |

## IPC Design: Ring-Buffer + Worker-Thread Pattern

The Phase 64 driver MUST implement kernel↔user IPC using the **ring-buffer + worker-thread**
pattern. This is not optional — it is the only layout that avoids all three BSOD failure modes
simultaneously.

### Architecture

```
   [Pre-create callback — runs at APC_LEVEL or below]
         |
         | Enqueue event descriptor to ring buffer
         | (NonPagedPoolNx allocation; bounded size; lock-free or spinlock)
         |
         v
   [Ring buffer — kernel memory, NonPagedPoolNx]
         |
         | Worker thread wakes on KeSetEvent / ExSetWorkItem
         v
   [Worker thread — runs at PASSIVE_LEVEL]
         |
         | FltSendMessage(timeout = ~500 ms)
         |   STATUS_TIMEOUT → permit I/O + log miss (fail-open; spike)
         |   STATUS_SUCCESS → process grant/deny decision from user mode
         v
   [User-mode supervisor — nono-cli / nono-wfp-service]
```

### Design Rules

1. **No driver-originated file I/O.** The pre-create callback and the worker thread MUST NOT
   call `ZwCreateFile`, `NtCreateFile`, or any other kernel file-open API. All communication
   is via `FltSendMessage`. If log-to-disk is ever needed, route it through user mode.

2. **Finite `FltSendMessage` timeout.** The `Timeout` parameter to `FltSendMessage` MUST be
   set to a negative value representing ~500 ms (e.g., `-5000000LL` in 100-ns units).
   The call MUST handle `STATUS_TIMEOUT` by permitting the I/O request and incrementing a
   missed-event counter. An infinite timeout (`NULL` timeout pointer) is forbidden — it locks
   every I/O on the volume if user mode is unresponsive.

3. **`NonPagedPoolNx` for all callback-reachable allocations.** Any structure allocated in a
   pre-create callback or passed to the worker thread must be allocated with
   `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, ...)` (WDK 21H1+) or
   `ExAllocatePoolWithTag(NonPagedPoolNx, ...)` (legacy). `PagedPool` is forbidden in
   callbacks that may run at IRQL > PASSIVE_LEVEL.

4. **IRQL assertions.** Insert `NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)` at the entry of
   every callback that allocates memory or acquires a lock. The worker thread runs at
   `PASSIVE_LEVEL` and is the correct site for the `FltSendMessage` call.

5. **No lock held across `FltSendMessage`.** Holding a spinlock or fast mutex across
   `FltSendMessage` causes an IRQL-mismatch BSOD if the send blocks. Enqueue atomically,
   release any lock, then send.

6. **Communication port name:** `\NonoPolicyPort` (dedicated FilterCommunicationPort; does NOT
   reuse the WFP named pipe — FltMgr IPC is synchronous kernel APC-based and cannot layer on
   an async named pipe or tokio).

## Altitude Configuration

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Altitude band | FSFilter Activity Monitor (360000–389999) | Activity-Monitor drivers observe I/O without blocking it; appropriate for a spike that intercepts + denies via user-mode decision |
| AV range to avoid | 320000–329998 | Registering in this range collides with EDR/AV drivers, causing registration failure or blinding the EDR |
| Phase 63 placeholder | 370020 (nullFilter default) | Temporary; Phase 64 MUST enumerate `fltmc filters` on the test VM and pick a non-colliding number in the Activity-Monitor band |
| Microsoft assignment | **PENDING** — email sent to fsfcomm@microsoft.com on: *(date to be filled by Plan 63-02 when the email is sent)* | An official altitude is required before any non-disposable deployment |

## Scope Boundary

### In Phase 63 (this document + skeleton only)

- Empty callbacks array (`IRP_MJ_OPERATION_END` sentinel)
- `DriverEntry` / `FltRegisterFilter` / `FltStartFiltering` / `NonoFltUnload`
- INF with `SERVICE_DEMAND_START`, FSFilter Activity Monitor altitude placeholder

### In Phase 64 (DRV-01 / DRV-02 — NOT this phase)

- Pre-create callback body (`IRP_MJ_CREATE` pre-op)
- `FltCreateCommunicationPort` + `FltSendMessage` + ring-buffer implementation
- Test-signing pipeline + `pnputil /add-driver` install on VM
- Altitude selection after `fltmc filters` enumeration

## V5 Input Validation Note (Future — Phase 64)

The kernel↔user IPC message (`#[repr(C)]` struct exchanged over `\NonoPolicyPort`) must include
a static layout assertion (`static_assert(sizeof(NonoIpcRequest) == N, "layout changed")`) to
catch accidental ABI drift between the driver and the user-mode supervisor. This is a Phase 64
deliverable; the placeholder is documented here so it is not forgotten.

## References

- PITFALLS.md: Pitfalls 1–5 (kernel BSOD triad, scope creep, altitude/EDR)
- RESEARCH.md §Security Domain: STRIDE table source
- `nullFilter` WDK sample: `github.com/microsoft/Windows-driver-samples/tree/main/filesys/miniFilter/nullFilter`
- FltMgr altitude bands: `learn.microsoft.com/windows-hardware/drivers/ifs/load-order-groups-and-altitudes-for-minifilter-drivers`
- Microsoft altitude assignment: `fsfcomm@microsoft.com`
