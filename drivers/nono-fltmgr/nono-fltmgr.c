// nono Gap 6b minifilter feasibility spike — skeleton DriverEntry
// Source: structure mirrors github.com/microsoft/Windows-driver-samples
//         filesys/miniFilter/nullFilter [CITED]
//
// Phase 63 deliverable: this file COMPILES to nono-fltmgr.sys on the test VM.
// It does NO file I/O, NO pre-create callback, NO communication port.
// Those are Phase 64 (DRV-01/DRV-02).
//
// BSOD-avoidance contract (see drivers/nono-fltmgr/DESIGN.md):
//   - No kernel file-open APIs (Pitfall 2: recursion BSOD would result)
//   - No filter communication port (Phase 64 only)
//   - Operation callbacks array is EMPTY (IRP_MJ_OPERATION_END only)
//   - NonoFltUnload registered so the filter can be cleanly removed (Pitfall 3)

#include <fltKernel.h>

PFLT_FILTER gFilterHandle = NULL;

// NonoFltUnload — called when the driver is unloaded (e.g. sc stop nono-fltmgr).
// PITFALLS Pitfall 3: a registered unload callback lets the driver un-register
// cleanly so queued messages never block on user-mode exit.
NTSTATUS
NonoFltUnload(_In_ FLT_FILTER_UNLOAD_FLAGS Flags)
{
    UNREFERENCED_PARAMETER(Flags);
    if (gFilterHandle != NULL) {
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
    }
    return STATUS_SUCCESS;
}

// Phase 63: EMPTY operation-callbacks array.
// The pre-create IRP_MJ_CREATE pre-op, ring-buffer IPC, and FltSendMessage
// round-trip are Phase 64 (DRV-01/DRV-02). This sentinel entry is all that
// is needed for the skeleton to compile and register.
CONST FLT_OPERATION_REGISTRATION Callbacks[] = {
    { IRP_MJ_OPERATION_END }
};

// FLT_REGISTRATION — minifilter self-description presented to FltMgr.
// NonoFltUnload is the only non-NULL callback; all instance callbacks are NULL
// because the Phase 63 skeleton performs no per-instance or per-I/O actions.
CONST FLT_REGISTRATION FilterRegistration = {
    sizeof(FLT_REGISTRATION),   // Size
    FLT_REGISTRATION_VERSION,   // Version
    0,                          // Flags
    NULL,                       // ContextRegistration (none for skeleton)
    Callbacks,                  // OperationRegistration (empty — Phase 64 fills it)
    NonoFltUnload,              // FilterUnloadCallback (registered to allow clean removal)
    NULL,                       // InstanceSetupCallback
    NULL,                       // InstanceQueryTeardownCallback
    NULL,                       // InstanceTeardownStartCallback
    NULL,                       // InstanceTeardownCompleteCallback
    NULL,                       // GenerateFileName
    NULL,                       // NormalizeNameComponent
    NULL,                       // NormalizeContextCleanup
    NULL,                       // TransactionNotificationCallback
    NULL                        // NormalizeNameComponentEx
};

// DriverEntry — minifilter entrypoint.
// Registers the filter and starts filtering. On failure, unregisters cleanly.
// No file I/O, no resource allocation beyond the filter handle (Phase 63 scope).
NTSTATUS
DriverEntry(_In_ PDRIVER_OBJECT DriverObject, _In_ PUNICODE_STRING RegistryPath)
{
    UNREFERENCED_PARAMETER(RegistryPath);

    NTSTATUS status = FltRegisterFilter(DriverObject, &FilterRegistration, &gFilterHandle);
    if (NT_SUCCESS(status)) {
        status = FltStartFiltering(gFilterHandle);
        if (!NT_SUCCESS(status)) {
            FltUnregisterFilter(gFilterHandle);
            gFilterHandle = NULL;
        }
    }
    return status;
}
