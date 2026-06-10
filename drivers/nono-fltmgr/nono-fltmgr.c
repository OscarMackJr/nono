// nono Gap 6b minifilter feasibility spike — extended DriverEntry + pre-create IPC
// Source: structure mirrors github.com/microsoft/Windows-driver-samples
//         filesys/miniFilter/nullFilter [CITED]
//
// Phase 63 deliverable: skeleton DriverEntry (empty Callbacks[], NonoFltUnload, DriverEntry)
// Phase 64 deliverable (DRV-01/DRV-02): this file extends the Phase 63 skeleton with:
//   - NonoPreCreate: IRP_MJ_CREATE pre-operation callback (FLT_PREOP_PENDING + ring-buffer enqueue)
//   - Ring buffer: single-slot NONO_RING_ENTRY with KSPIN_LOCK + KEVENT
//   - NonoWorkerThread: PASSIVE_LEVEL worker that calls FltSendMessage with 500ms finite timeout
//   - FltCreateCommunicationPort call in DriverEntry opening \NonoPolicyPort
//   - Port callbacks: NonoPortConnect, NonoPortDisconnect, NonoPortMessage
//   - NonoInstanceTeardownStart: InstanceTeardownStartCallback for port + IRP drain before unload
//
// BSOD-avoidance contract (see drivers/nono-fltmgr/DESIGN.md — hard pre-code gate D-10):
//   T-63-01: NO ZwCreateFile / NtCreateFile / ZwReadFile / ZwWriteFile anywhere in this file
//   T-63-02: FltSendMessage MUST use timeout.QuadPart = -5000000LL (500ms); STATUS_TIMEOUT = fail-open
//   T-63-03: NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL) at NonoPreCreate entry; ExAllocatePool2 only
//   T-63-04: PathBuffer copy bounded to 259 WCHARs; static assert in nono-fltmgr.h catches layout drift
//   T-63-05: .sys binary is VM-local only; never committed to the repo
//
// Design rules enforced:
//   - Ring buffer lock is RELEASED before FltSendMessage (DESIGN.md Rule 5: no lock across send)
//   - All callback-reachable allocations: ExAllocatePool2(POOL_FLAG_NON_PAGED, ...) only (NX by default)
//   - Worker thread completes ALL pending IRPs before exit (Pitfall A: IRP leak prevention)

#include <fltKernel.h>
#include "nono-fltmgr.h"

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

// Filter handle: registered in DriverEntry, unregistered in NonoFltUnload.
PFLT_FILTER gFilterHandle = NULL;

// Communication port handles: gServerPort is the kernel-side server port opened by
// FltCreateCommunicationPort; gClientPort is the single user-mode connection handle.
PFLT_PORT gServerPort = NULL;
PFLT_PORT gClientPort = NULL;

// Ring buffer: single-slot NONO_RING_ENTRY with a KSPIN_LOCK guard and a KEVENT
// to wake the worker thread. Single-slot is sufficient for the spike (max 1 connection,
// RESEARCH.md Open Question 2 resolution).
typedef struct _NONO_RING_ENTRY {
    // Heap-allocated NONO_IPC_REQUEST payload (allocated in NonoPreCreate via
    // ExAllocatePool2(POOL_FLAG_NON_PAGED); freed in NonoWorkerThread after send).
    PNONO_IPC_REQUEST pRequest;

    // FLT_CALLBACK_DATA for the pending IRP. Used by the worker thread to call
    // FltCompletePendedPreOperation after the policy round-trip.
    PFLT_CALLBACK_DATA Data;

    // TRUE if this slot holds a pending request, FALSE if empty.
    BOOLEAN Occupied;

    // SPAN-B start timestamp (D-02b): QPC tick count captured at enqueue in
    // NonoPreCreate. Kernel-internal — safe to add here (NONO_RING_ENTRY is NOT
    // the ABI-locked wire struct; NONO_IPC_REQUEST in the header stays at 532
    // bytes). The dequeue copy site lifts this into a local before the slot is
    // cleared so SPAN-B end can compute the full pre-op->completion delta.
    LARGE_INTEGER EnqueueQpc;
} NONO_RING_ENTRY, *PNONO_RING_ENTRY;

NONO_RING_ENTRY g_RingEntry;
KSPIN_LOCK      g_RingLock;
KEVENT          g_RingBufferEvent;

// ---------------------------------------------------------------------------
// Latency instrumentation (Phase 65 DRV-04, D-01..D-03)
//
// Two QPC-measured spans over denied creates:
//   SPAN-A (D-02a): kernel-IPC round-trip — brackets the FltSendMessage call.
//   SPAN-B (D-02b): full pre-op->IRP-completion — enqueue (NonoPreCreate) to
//                   FltCompletePendedPreOperation (NonoWorkerThread).
//
// KeQueryPerformanceCounter is HAL-only (T-63-01 clean) and any-IRQL (T-63-03).
// Raw ticks are accumulated lock-free (single-slot ring serializes the writers,
// Assumption A2 — no InterlockedIncrement needed) and sorted + dumped once at
// unload (outside any timed span; Pitfall 1: no DbgPrint inside a span).
// Integer-microsecond math only — no kernel floating point.
// ---------------------------------------------------------------------------
LARGE_INTEGER g_PerfFreq;               // QPC ticks/sec, cached once in DriverEntry (fixed at boot)
#define NONO_SAMPLE_MAX 128
LONG64 g_SpanA[NONO_SAMPLE_MAX];        // SPAN-A raw ticks (D-02a)
LONG   g_SpanACount = 0;
LONG64 g_SpanB[NONO_SAMPLE_MAX];        // SPAN-B raw ticks (D-02b)
LONG   g_SpanBCount = 0;

// Worker thread lifecycle controls.
BOOLEAN gWorkerRunning      = FALSE;
HANDLE  gWorkerThreadHandle = NULL;

// Spike scoping (D-02): NonoPreCreate only round-trips creates whose final path
// component matches this name to user mode. WITHOUT this, every file open on the
// system would be routed through the single-slot ring + user-mode round-trip,
// making the desktop unusably slow (appears as a hang). A real capability sandbox
// scopes by policy; the spike scopes to the deterministic deny-target file name.
DECLARE_CONST_UNICODE_STRING(gWatchedFileName, L"secret.txt");

// ---------------------------------------------------------------------------
// Forward declarations
// ---------------------------------------------------------------------------

FLT_PREOP_CALLBACK_STATUS NonoPreCreate(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext);

VOID NonoWorkerThread(_In_ PVOID Context);

NTSTATUS NonoPortConnect(
    _In_ PFLT_PORT ClientPort,
    _In_opt_ PVOID ServerPortCookie,
    _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext,
    _In_ ULONG SizeOfContext,
    _Outptr_result_maybenull_ PVOID *ConnectionCookie);

VOID NonoPortDisconnect(
    _In_opt_ PVOID ConnectionCookie);

NTSTATUS NonoPortMessage(
    _In_opt_ PVOID PortCookie,
    _In_reads_bytes_opt_(InputBufferLength) PVOID InputBuffer,
    _In_ ULONG InputBufferLength,
    _Out_writes_bytes_to_opt_(OutputBufferLength, *ReturnOutputBufferLength) PVOID OutputBuffer,
    _In_ ULONG OutputBufferLength,
    _Out_ PULONG ReturnOutputBufferLength);

VOID NonoInstanceTeardownStart(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_TEARDOWN_FLAGS ReasonFlags);

NTSTATUS NonoFltUnload(_In_ FLT_FILTER_UNLOAD_FLAGS Flags);

NTSTATUS DriverEntry(_In_ PDRIVER_OBJECT DriverObject, _In_ PUNICODE_STRING RegistryPath);

// ---------------------------------------------------------------------------
// Operation callbacks array (Phase 64: pre-create entry added before sentinel)
// ---------------------------------------------------------------------------

CONST FLT_OPERATION_REGISTRATION Callbacks[] = {
    // Phase 64: IRP_MJ_CREATE pre-operation callback.
    // PreOperation = NonoPreCreate; PostOperation = NULL (no post-op needed for deny).
    { IRP_MJ_CREATE,
      0,
      NonoPreCreate,
      NULL },
    // Sentinel — must be last.
    { IRP_MJ_OPERATION_END }
};

// ---------------------------------------------------------------------------
// FLT_REGISTRATION — minifilter self-description presented to FltMgr.
// Phase 64: InstanceTeardownStartCallback added for port + IRP drain before unload.
// ---------------------------------------------------------------------------

CONST FLT_REGISTRATION FilterRegistration = {
    sizeof(FLT_REGISTRATION),      // Size
    FLT_REGISTRATION_VERSION,      // Version
    0,                             // Flags
    NULL,                          // ContextRegistration (none for spike)
    Callbacks,                     // OperationRegistration (pre-create + sentinel)
    NonoFltUnload,                 // FilterUnloadCallback
    NULL,                          // InstanceSetupCallback
    NULL,                          // InstanceQueryTeardownCallback
    NonoInstanceTeardownStart,     // InstanceTeardownStartCallback (Phase 64: port + IRP drain)
    NULL,                          // InstanceTeardownCompleteCallback
    NULL,                          // GenerateFileName
    NULL,                          // NormalizeNameComponent
    NULL,                          // NormalizeContextCleanup
    NULL,                          // TransactionNotificationCallback
    NULL                           // NormalizeNameComponentEx
};

// ---------------------------------------------------------------------------
// NonoPreCreate — IRP_MJ_CREATE pre-operation callback
//
// Runs at APC_LEVEL or below. Must NOT call any kernel file-open APIs
// (DESIGN.md T-63-01: ZwCreateFile / NtCreateFile causes recursive BSOD).
// Returns FLT_PREOP_PENDING to suspend the IRP. The worker thread completes it.
//
// Fail-open contract: on any error (name lookup, allocation, back-pressure),
// return FLT_PREOP_SUCCESS_NO_CALLBACK to permit the I/O. This is the spike
// policy; production ADR decides fail-direction.
// ---------------------------------------------------------------------------
FLT_PREOP_CALLBACK_STATUS
NonoPreCreate(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);

    // DESIGN.md T-63-03: assert IRQL at callback entry before any allocation or lock.
    NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL);

    // Fail-open fast path: if no user-mode policy client is connected, the filter is
    // transparent. This avoids pending (and thus stalling) I/O when the driver is
    // loaded but no client has connected yet. Once a client connects, NonoPortConnect
    // sets gClientPort and creates are evaluated. The unlocked pointer read is a
    // benign race (worst case: one create near the connect/disconnect edge is
    // evaluated or not); acceptable for the spike.
    if (gClientPort == NULL) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    // Get the OPENED file name. In a pre-create callback the create has NOT yet been
    // processed by the file system, so requesting FLT_FILE_NAME_NORMALIZED here can
    // DEADLOCK: normalization may issue I/O that re-enters NonoPreCreate (system-wide
    // hang once a client is connected). FLT_FILE_NAME_OPENED is the name form that is
    // safe to query in pre-create. On failure: fail-open (no deny without the name).
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    NTSTATUS status = FltGetFileNameInformation(
        Data,
        FLT_FILE_NAME_OPENED | FLT_FILE_NAME_QUERY_DEFAULT,
        &nameInfo);
    if (!NT_SUCCESS(status)) {
        // Fail-open: cannot determine the file path; permit the I/O.
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    // Parse the name information to populate nameInfo->Name / FinalComponent.
    FltParseFileNameInformation(nameInfo);

    // Spike scoping: ignore every create except the watched deny-target file name.
    // FinalComponent is the leaf (e.g. "secret.txt"). Non-matching creates pass
    // through immediately (fail-open), so ONLY the target is round-tripped to user
    // mode — this keeps the rest of the system fast instead of routing every open
    // through the single-slot ring. Case-insensitive (Windows paths are).
    if (!RtlEqualUnicodeString(&nameInfo->FinalComponent, &gWatchedFileName, TRUE)) {
        FltReleaseFileNameInformation(nameInfo);
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    // DESIGN.md T-63-01: DO NOT call ZwCreateFile, NtCreateFile, or any file I/O API here.
    // Allocate ring-buffer payload from NonPagedPoolNx (DESIGN.md T-63-03: required for
    // callback-reachable allocations; PagedPool is forbidden at APC_LEVEL or above).
    PNONO_IPC_REQUEST pReq = (PNONO_IPC_REQUEST)ExAllocatePool2(
        POOL_FLAG_NON_PAGED,  // non-paged + no-execute (NX) by default in the POOL_FLAGS scheme
        sizeof(NONO_IPC_REQUEST),
        'onoN');  // Pool tag 'NoNo' reversed per WDK convention
    if (pReq == NULL) {
        // Fail-open: allocation failure; permit the I/O.
        FltReleaseFileNameInformation(nameInfo);
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    // ExAllocatePool2 guarantees zero-initialized memory (unlike deprecated ExAllocatePoolWithTag).
    // Copy path from nameInfo->Name.Buffer, bounded to 259 WCHARs + null terminator.
    // DESIGN.md T-63-04: fixed-size buffer prevents overflow.
    ULONG pathLen = nameInfo->Name.Length / sizeof(WCHAR);
    if (pathLen > 259) {
        pathLen = 259;  // Clamp to MAX_PATH - 1 to preserve null terminator slot.
    }
    RtlCopyMemory(pReq->PathBuffer, nameInfo->Name.Buffer, pathLen * sizeof(WCHAR));
    pReq->PathBuffer[pathLen] = L'\0';  // Null-terminate.

    // Set requestor PID from the current process.
    pReq->ProcessId = (ULONG)(ULONG_PTR)PsGetCurrentProcessId();

    // Set desired-access mask from the IRP parameters.
    pReq->DesiredAccess = Data->Iopb->Parameters.Create.SecurityContext->DesiredAccess;

    // Reserved field is already zeroed by ExAllocatePool2.

    // Acquire ring-buffer spinlock to enqueue atomically.
    // DESIGN.md Rule 5: the lock is RELEASED before FltSendMessage (the worker sends, not here).
    KIRQL oldIrql;
    KeAcquireSpinLock(&g_RingLock, &oldIrql);

    if (g_RingEntry.Occupied) {
        // Back-pressure: the single slot is busy with another round-trip. We only
        // reach here for the watched deny-target file (pre-filtered above), whose
        // policy is DENY, so fail CLOSED (deny) rather than fail-open. This makes the
        // deny deterministic despite single-slot contention (the slot-winning open
        // still does the full user-mode round-trip, so DRV-02 is still proven) and is
        // the fail-secure choice for a security filter. RESEARCH.md Open Question 2:
        // single-slot is the spike design choice.
        KeReleaseSpinLock(&g_RingLock, oldIrql);
        ExFreePoolWithTag(pReq, 'onoN');
        FltReleaseFileNameInformation(nameInfo);
        Data->IoStatus.Status      = STATUS_ACCESS_DENIED;
        Data->IoStatus.Information = 0;
        return FLT_PREOP_COMPLETE;
    }

    // Enqueue: store request + callback data in the ring slot.
    g_RingEntry.pRequest = pReq;
    g_RingEntry.Data     = Data;
    g_RingEntry.Occupied = TRUE;

    // SPAN-B start (D-02b): timestamp the full pre-op->completion span at enqueue,
    // still under the lock so it pairs atomically with the slot it rides.
    // KeQueryPerformanceCounter is any-IRQL (T-63-03) — safe at <= APC_LEVEL here.
    g_RingEntry.EnqueueQpc = KeQueryPerformanceCounter(NULL);

    // Lock released before signaling (DESIGN.md Rule 5).
    KeReleaseSpinLock(&g_RingLock, oldIrql);

    // Release file name info (no longer needed after copy).
    FltReleaseFileNameInformation(nameInfo);

    // Wake the worker thread.
    KeSetEvent(&g_RingBufferEvent, IO_NO_INCREMENT, FALSE);

    // Return PENDING — IRP is suspended until NonoWorkerThread calls FltCompletePendedPreOperation.
    // MUST NOT return FLT_PREOP_COMPLETE from here (see RESEARCH.md Anti-Patterns).
    return FLT_PREOP_PENDING;
}

// ---------------------------------------------------------------------------
// NonoWorkerThread — PASSIVE_LEVEL worker for FltSendMessage round-trip
//
// DESIGN.md Rule 2: FltSendMessage with finite 500ms timeout mandatory.
// DESIGN.md Rule 5: spinlock NOT held across FltSendMessage (released at enqueue).
// All IRPs enqueued by NonoPreCreate are always completed here (Pitfall A guard).
// ---------------------------------------------------------------------------
VOID
NonoWorkerThread(
    _In_ PVOID Context)
{
    UNREFERENCED_PARAMETER(Context);

    while (gWorkerRunning) {
        // Wait for ring-buffer signal. PASSIVE_LEVEL guaranteed by KeWaitForSingleObject
        // called from a system thread context (no lock held here).
        // Infinite wait is safe: event is signaled by NonoPreCreate (enqueue) or
        // NonoFltUnload (stop signal via gWorkerRunning = FALSE + KeSetEvent).
        KeWaitForSingleObject(
            &g_RingBufferEvent,
            Executive,
            KernelMode,
            FALSE,
            NULL);

        // Check stop condition: gWorkerRunning may have been cleared by NonoFltUnload.
        if (!gWorkerRunning) {
            break;
        }

        // Dequeue from the ring buffer under the spinlock.
        KIRQL oldIrql;
        KeAcquireSpinLock(&g_RingLock, &oldIrql);

        if (!g_RingEntry.Occupied) {
            // Spurious wakeup or stop-signal path. Release and loop.
            KeReleaseSpinLock(&g_RingLock, oldIrql);
            continue;
        }

        // Extract the pending request and callback data, then clear the slot atomically.
        PNONO_IPC_REQUEST pRequest     = g_RingEntry.pRequest;
        PFLT_CALLBACK_DATA pendingData = g_RingEntry.Data;
        // SPAN-B start (D-02b): lift the enqueue timestamp into a local before the
        // slot is cleared, so SPAN-B end (after completion) can compute the delta.
        LARGE_INTEGER localEnqueueQpc  = g_RingEntry.EnqueueQpc;
        g_RingEntry.pRequest  = NULL;
        g_RingEntry.Data      = NULL;
        g_RingEntry.Occupied  = FALSE;

        KeReleaseSpinLock(&g_RingLock, oldIrql);
        // DESIGN.md Rule 5: spinlock released BEFORE FltSendMessage call below.

        // Prepare the reply buffer and timeout.
        NONO_IPC_REPLY reply = { 0 };
        ULONG replyLen       = sizeof(reply);

        // DESIGN.md T-63-02: finite 500ms timeout. Negative value = relative time in
        // 100-nanosecond units. -5000000 * 100ns = 500ms. NULL timeout is FORBIDDEN.
        LARGE_INTEGER timeout;
        timeout.QuadPart = -5000000LL;

        // SPAN-A start (D-02a): timestamp immediately before the kernel-IPC round-trip.
        // No DbgPrint/logging between a0 and a1 (Pitfall 1: logging dominates the span).
        LARGE_INTEGER a0 = KeQueryPerformanceCounter(NULL);

        // FltSendMessage: send the IPC request to the user-mode policy client.
        // gClientPort is NULL if no user-mode client is connected -> FltSendMessage
        // returns a non-success status, triggering the fail-open path below.
        NTSTATUS sendStatus = FltSendMessage(
            gFilterHandle,
            &gClientPort,
            pRequest,
            sizeof(NONO_IPC_REQUEST),
            &reply,
            &replyLen,
            &timeout);

        // SPAN-A end (D-02a): record (a1 - a0) ONLY on the non-timeout path. A timeout
        // measures the 500ms fail-open envelope, not the real round-trip — excluded.
        LARGE_INTEGER a1 = KeQueryPerformanceCounter(NULL);
        if (sendStatus != STATUS_TIMEOUT && g_SpanACount < NONO_SAMPLE_MAX) {
            g_SpanA[g_SpanACount++] = a1.QuadPart - a0.QuadPart;
        }

        // Decide allow vs deny. Fail-open on timeout or any send error (DESIGN.md
        // T-63-02). Only an explicit reply.Decision == 1 denies.
        BOOLEAN deny = (sendStatus != STATUS_TIMEOUT)
                       && NT_SUCCESS(sendStatus)
                       && (reply.Decision == 1);

        // Free the ring-buffer payload (allocated in NonoPreCreate with ExAllocatePool2).
        ExFreePoolWithTag(pRequest, 'onoN');
        pRequest = NULL;

        // Complete the pended create. The completion STATUS is NOT the same as the
        // FLT_PREOP_* code:
        //   DENY  -> FLT_PREOP_COMPLETE + STATUS_ACCESS_DENIED: we finish the create
        //            ourselves; the file is never opened and the op is NOT passed down.
        //   ALLOW -> FLT_PREOP_SUCCESS_NO_CALLBACK: RESUME the create down the stack so
        //            the file system actually opens the file. Using FLT_PREOP_COMPLETE
        //            with STATUS_SUCCESS here would "complete" the create WITHOUT opening
        //            anything, surfacing to the caller as "The parameter is incorrect".
        if (deny) {
            pendingData->IoStatus.Status      = STATUS_ACCESS_DENIED;
            pendingData->IoStatus.Information  = 0;
            FltCompletePendedPreOperation(pendingData, FLT_PREOP_COMPLETE, NULL);
            // SPAN-B end (D-02b): full pre-op->completion delta, measured only on the
            // deny path (the round-tripped, denied creates DRV-04 cares about).
            LARGE_INTEGER b1 = KeQueryPerformanceCounter(NULL);
            if (g_SpanBCount < NONO_SAMPLE_MAX) {
                g_SpanB[g_SpanBCount++] = b1.QuadPart - localEnqueueQpc.QuadPart;
            }
        } else {
            FltCompletePendedPreOperation(pendingData, FLT_PREOP_SUCCESS_NO_CALLBACK, NULL);
        }
    }

    // Worker thread termination. PsTerminateSystemThread does not return.
    PsTerminateSystemThread(STATUS_SUCCESS);
}

// ---------------------------------------------------------------------------
// Port callbacks for \NonoPolicyPort
// ---------------------------------------------------------------------------

// NonoPortConnect — called when a user-mode client connects to \NonoPolicyPort.
// Saves the ClientPort handle for use in FltSendMessage (gClientPort).
NTSTATUS
NonoPortConnect(
    _In_ PFLT_PORT ClientPort,
    _In_opt_ PVOID ServerPortCookie,
    _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext,
    _In_ ULONG SizeOfContext,
    _Outptr_result_maybenull_ PVOID *ConnectionCookie)
{
    UNREFERENCED_PARAMETER(ServerPortCookie);
    UNREFERENCED_PARAMETER(ConnectionContext);
    UNREFERENCED_PARAMETER(SizeOfContext);
    UNREFERENCED_PARAMETER(ConnectionCookie);

    // Save the client port handle. FltSendMessage uses &gClientPort.
    gClientPort = ClientPort;
    return STATUS_SUCCESS;
}

// NonoPortDisconnect — called when the user-mode client disconnects.
// Closes the client port handle and nulls gClientPort so FltSendMessage
// returns a non-success status, triggering the fail-open path.
VOID
NonoPortDisconnect(
    _In_opt_ PVOID ConnectionCookie)
{
    UNREFERENCED_PARAMETER(ConnectionCookie);

    if (gClientPort != NULL) {
        FltCloseClientPort(gFilterHandle, &gClientPort);
        gClientPort = NULL;
    }
}

// NonoPortMessage — user-to-kernel message callback (not used in the spike).
// The spike's message direction is kernel->user via FltSendMessage. This callback
// handles user->kernel messages, which the spike does not send. Return STATUS_SUCCESS.
NTSTATUS
NonoPortMessage(
    _In_opt_ PVOID PortCookie,
    _In_reads_bytes_opt_(InputBufferLength) PVOID InputBuffer,
    _In_ ULONG InputBufferLength,
    _Out_writes_bytes_to_opt_(OutputBufferLength, *ReturnOutputBufferLength) PVOID OutputBuffer,
    _In_ ULONG OutputBufferLength,
    _Out_ PULONG ReturnOutputBufferLength)
{
    UNREFERENCED_PARAMETER(PortCookie);
    UNREFERENCED_PARAMETER(InputBuffer);
    UNREFERENCED_PARAMETER(InputBufferLength);
    UNREFERENCED_PARAMETER(OutputBuffer);
    UNREFERENCED_PARAMETER(OutputBufferLength);

    *ReturnOutputBufferLength = 0;
    return STATUS_SUCCESS;
}

// ---------------------------------------------------------------------------
// NonoInstanceTeardownStart — InstanceTeardownStartCallback
//
// Called before EACH per-volume instance is torn down. Drains any pending IRP
// from the (global, single-slot) ring buffer so a tearing-down instance never
// leaks a pended IRP (PITFALLS Pitfall A).
//
// IMPORTANT: this callback fires PER INSTANCE (per volume), so it must NOT touch
// driver-global resources like the communication port. Closing gServerPort here
// destroyed \NonoPolicyPort for the whole driver the first time ANY volume's
// instance tore down — the user-mode client then gets ERROR_FILE_NOT_FOUND
// (0x80070002) on connect. The server port is a driver-wide resource: created
// once in DriverEntry, closed once in NonoFltUnload.
// ---------------------------------------------------------------------------
VOID
NonoInstanceTeardownStart(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_TEARDOWN_FLAGS ReasonFlags)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(ReasonFlags);

    // Do NOT close gServerPort here (see header comment) — it is driver-wide and
    // is closed in NonoFltUnload, not per-instance.

    // Drain any pending IRP in the ring buffer (Pitfall A: IRP-leak prevention).
    KIRQL oldIrql;
    KeAcquireSpinLock(&g_RingLock, &oldIrql);
    if (g_RingEntry.Occupied) {
        PNONO_IPC_REQUEST pRequest     = g_RingEntry.pRequest;
        PFLT_CALLBACK_DATA pendingData = g_RingEntry.Data;
        g_RingEntry.pRequest  = NULL;
        g_RingEntry.Data      = NULL;
        g_RingEntry.Occupied  = FALSE;
        KeReleaseSpinLock(&g_RingLock, oldIrql);

        // Fail-open: RESUME the pended create down the stack (FLT_PREOP_SUCCESS_NO_CALLBACK)
        // so the file is actually opened. Do NOT use FLT_PREOP_COMPLETE + STATUS_SUCCESS —
        // that finishes the create without opening anything ("parameter is incorrect").
        if (pendingData != NULL) {
            FltCompletePendedPreOperation(pendingData, FLT_PREOP_SUCCESS_NO_CALLBACK, NULL);
        }
        if (pRequest != NULL) {
            ExFreePoolWithTag(pRequest, 'onoN');
        }
    } else {
        KeReleaseSpinLock(&g_RingLock, oldIrql);
    }
}

// ---------------------------------------------------------------------------
// NonoDumpSpan — sort one latency span and DbgPrint its min/median/p99 (Phase 65)
//
// Called from NonoFltUnload AFTER the worker thread is joined, so there is no
// concurrent writer to g_SpanA/g_SpanB and no lock is needed. This is OUTSIDE
// any timed span (Pitfall 1), so DbgPrint here is safe.
//
// Integer-microsecond math only (no kernel floating point): us = ticks *
// 1000000 / freq. In-place insertion sort over <= NONO_SAMPLE_MAX (128) samples.
// ---------------------------------------------------------------------------
static VOID
NonoDumpSpan(_In_ PCSTR Label, _Inout_updates_(Count) LONG64 *Samples, _In_ LONG Count, _In_ LONG64 FreqQpc)
{
    if (Count <= 0 || FreqQpc <= 0) {
        DbgPrint("[nono-fltmgr] %s: no samples (count=%ld)\n", Label, Count);
        return;
    }

    // In-place insertion sort (ascending). Count is bounded to NONO_SAMPLE_MAX.
    for (LONG i = 1; i < Count; i++) {
        LONG64 key = Samples[i];
        LONG j = i - 1;
        while (j >= 0 && Samples[j] > key) {
            Samples[j + 1] = Samples[j];
            j--;
        }
        Samples[j + 1] = key;
    }

    // Percentile indices (nearest-rank, clamped to the last sample).
    LONG p99Idx = (Count * 99) / 100;
    if (p99Idx >= Count) {
        p99Idx = Count - 1;
    }

    LONG64 minUs    = Samples[0]          * 1000000LL / FreqQpc;
    LONG64 medianUs = Samples[Count / 2]  * 1000000LL / FreqQpc;
    LONG64 p99Us    = Samples[p99Idx]     * 1000000LL / FreqQpc;

    DbgPrint("[nono-fltmgr] %s: iterations=%ld min=%lld us median=%lld us p99=%lld us (freq=%lld)\n",
             Label, Count, minUs, medianUs, p99Us, FreqQpc);
}

// ---------------------------------------------------------------------------
// NonoFltUnload — called when the driver is unloaded (e.g. sc stop nono-fltmgr)
//
// Cleanup order (PITFALLS Pitfall 3 / DESIGN.md T-63-02):
//   1. Signal worker thread to stop and wait for it to exit.
//   2. Close the communication port (if not already closed by teardown).
//   3. Unregister the filter.
// ---------------------------------------------------------------------------
NTSTATUS
NonoFltUnload(
    _In_ FLT_FILTER_UNLOAD_FLAGS Flags)
{
    UNREFERENCED_PARAMETER(Flags);

    // Signal the worker thread to stop and wake it so it exits the wait loop.
    gWorkerRunning = FALSE;
    KeSetEvent(&g_RingBufferEvent, IO_NO_INCREMENT, FALSE);

    // Wait for the worker thread to terminate.
    if (gWorkerThreadHandle != NULL) {
        // Obtain the PETHREAD from the handle, wait for it to exit, then close.
        PETHREAD pThread = NULL;
        NTSTATUS status = ObReferenceObjectByHandle(
            gWorkerThreadHandle,
            SYNCHRONIZE,
            *PsThreadType,
            KernelMode,
            (PVOID *)&pThread,
            NULL);
        if (NT_SUCCESS(status)) {
            KeWaitForSingleObject(pThread, Executive, KernelMode, FALSE, NULL);
            ObDereferenceObject(pThread);
        }
        ZwClose(gWorkerThreadHandle);
        gWorkerThreadHandle = NULL;
    }

    // Latency dump (Phase 65 D-02/D-03): the worker is now joined, so g_SpanA/g_SpanB
    // have no concurrent writer and no lock is required. Emitted here (outside any
    // timed span) via DbgPrint — collect from the kernel debugger / DebugView. The
    // single-slot ring serialized the accumulation (no InterlockedIncrement needed).
    NonoDumpSpan("SPAN-A kernel-IPC round-trip", g_SpanA, g_SpanACount, g_PerfFreq.QuadPart);
    NonoDumpSpan("SPAN-B full pre-op->completion", g_SpanB, g_SpanBCount, g_PerfFreq.QuadPart);

    // Close the communication server port (if not already closed by InstanceTeardownStart).
    if (gServerPort != NULL) {
        FltCloseCommunicationPort(gServerPort);
        gServerPort = NULL;
    }

    // Close any lingering client port.
    if (gClientPort != NULL) {
        FltCloseClientPort(gFilterHandle, &gClientPort);
        gClientPort = NULL;
    }

    // Unregister the filter. Must be last — all pending IRPs must be drained first.
    if (gFilterHandle != NULL) {
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
    }

    return STATUS_SUCCESS;
}

// ---------------------------------------------------------------------------
// DriverEntry — minifilter entrypoint
//
// Registers the filter, starts filtering, creates the communication port, and
// starts the worker thread. Cleans up in reverse order on any failure.
//
// DESIGN.md T-63-01: NO ZwCreateFile / NtCreateFile / file I/O anywhere below.
// ---------------------------------------------------------------------------
NTSTATUS
DriverEntry(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PUNICODE_STRING RegistryPath)
{
    UNREFERENCED_PARAMETER(RegistryPath);

    NTSTATUS status;

    // Register the minifilter with FltMgr.
    status = FltRegisterFilter(DriverObject, &FilterRegistration, &gFilterHandle);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    // NOTE: FltStartFiltering is deliberately deferred to the END of DriverEntry
    // (after the ring buffer, comm port, and worker thread are ready). Starting it
    // here would let IRP_MJ_CREATE reach NonoPreCreate before the ring spinlock is
    // initialized and before any worker exists to complete pended IRPs -> system hang
    // on load. This was the cause of the fltmc-load hang.

    // Initialize ring buffer synchronization primitives and zero the ring entry.
    KeInitializeEvent(&g_RingBufferEvent, SynchronizationEvent, FALSE);
    KeInitializeSpinLock(&g_RingLock);
    RtlZeroMemory(&g_RingEntry, sizeof(g_RingEntry));

    // Latency instrumentation (D-01): cache the QPC frequency once. ticks/sec is
    // fixed at boot, so caching avoids re-querying it inside any timed span. The
    // accumulators (g_SpanA/g_SpanB) are zero-initialized as file-scope globals.
    (void)KeQueryPerformanceCounter(&g_PerfFreq);

    // Build a security descriptor for the communication port.
    // FLT_PORT_ALL_ACCESS allows any user-mode process to connect (spike only).
    // RESEARCH.md Pitfall F: FltBuildDefaultSecurityDescriptor is required; without
    // an explicit SD, only kernel callers can connect and FilterConnectCommunicationPort
    // returns ACCESS_DENIED from user mode.
    PSECURITY_DESCRIPTOR sd = NULL;
    status = FltBuildDefaultSecurityDescriptor(&sd, FLT_PORT_ALL_ACCESS);
    if (!NT_SUCCESS(status)) {
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
        return status;
    }

    // Build the communication port name and object attributes.
    UNICODE_STRING portName = RTL_CONSTANT_STRING(L"\\NonoPolicyPort");
    OBJECT_ATTRIBUTES oa;
    InitializeObjectAttributes(
        &oa,
        &portName,
        OBJ_KERNEL_HANDLE | OBJ_CASE_INSENSITIVE,
        NULL,
        sd);

    // Create the communication port. Max 1 connection (spike: single-slot ring buffer).
    status = FltCreateCommunicationPort(
        gFilterHandle,
        &gServerPort,
        &oa,
        NULL,               // ServerPortCookie (unused)
        NonoPortConnect,
        NonoPortDisconnect,
        NonoPortMessage,
        1);                 // MaxConnections

    FltFreeSecurityDescriptor(sd);

    if (!NT_SUCCESS(status)) {
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
        return status;
    }

    // Start the worker thread. Runs at PASSIVE_LEVEL and performs FltSendMessage
    // round-trips outside the pre-create callback context.
    gWorkerRunning = TRUE;
    status = PsCreateSystemThread(
        &gWorkerThreadHandle,
        THREAD_ALL_ACCESS,
        NULL,               // ObjectAttributes
        NULL,               // ProcessHandle (use System process)
        NULL,               // ClientId
        NonoWorkerThread,
        NULL);              // Context

    if (!NT_SUCCESS(status)) {
        gWorkerRunning = FALSE;
        FltCloseCommunicationPort(gServerPort);
        gServerPort = NULL;
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
        return status;
    }

    // Start filtering LAST. The ring buffer, comm port, and worker thread are all
    // ready now, so no IRP_MJ_CREATE can reach NonoPreCreate before the driver can
    // handle and complete it. (This ordering is the fix for the fltmc-load hang.)
    status = FltStartFiltering(gFilterHandle);
    if (!NT_SUCCESS(status)) {
        // Tear down in reverse: stop + join the worker, close the port, unregister.
        gWorkerRunning = FALSE;
        KeSetEvent(&g_RingBufferEvent, IO_NO_INCREMENT, FALSE);
        if (gWorkerThreadHandle != NULL) {
            PETHREAD pThread = NULL;
            if (NT_SUCCESS(ObReferenceObjectByHandle(
                    gWorkerThreadHandle, SYNCHRONIZE, *PsThreadType,
                    KernelMode, (PVOID *)&pThread, NULL))) {
                KeWaitForSingleObject(pThread, Executive, KernelMode, FALSE, NULL);
                ObDereferenceObject(pThread);
            }
            ZwClose(gWorkerThreadHandle);
            gWorkerThreadHandle = NULL;
        }
        if (gServerPort != NULL) {
            FltCloseCommunicationPort(gServerPort);
            gServerPort = NULL;
        }
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
        return status;
    }

    return STATUS_SUCCESS;
}
