// nono-fltmgr.h — Shared IPC struct definitions for the nono minifilter spike
//
// This header defines NONO_IPC_REQUEST and NONO_IPC_REPLY for the kernel
// FilterCommunicationPort (\NonoPolicyPort) round-trip between the minifilter
// driver (kernel mode) and the Rust user-mode policy client (nono-fltmgr-client).
//
// BSOD-avoidance contract: see drivers/nono-fltmgr/DESIGN.md (hard pre-code gate).
//
// Rust-side mirror: crates/nono-fltmgr-client/src/lib.rs NonoIpcRequest.
// The _Static_assert / C_ASSERT below MUST match the Rust-side layout assertion:
//   size_of::<NonoIpcRequest>() - size_of::<FILTER_MESSAGE_HEADER>() == 532
//
// Phase 64 DRV-01/DRV-02 spike. NOT production-ready; test-VM use only.
// See DESIGN.md T-63-05: the compiled .sys is never committed to this repository.

#pragma once

#include <fltKernel.h>

// ---------------------------------------------------------------------------
// NONO_IPC_REQUEST — kernel-to-user message payload
//
// Sent by the worker thread via FltSendMessage to the user-mode policy client.
// On the Rust (user-mode) receive side, FILTER_MESSAGE_HEADER is prepended by
// FilterGetMessage — it is NOT included here (kernel sends only the payload).
//
// #pragma pack(push, 1) ensures no padding bytes between fields:
//   WCHAR PathBuffer[260] = 260 * 2 = 520 bytes
//   ULONG ProcessId       =           4 bytes
//   ACCESS_MASK DesiredAccess =        4 bytes  (ACCESS_MASK = ULONG)
//   ULONG Reserved        =           4 bytes  (spike alignment padding)
//   Total:                           532 bytes
// ---------------------------------------------------------------------------
#pragma pack(push, 1)
typedef struct _NONO_IPC_REQUEST {
    // Null-terminated normalized path of the file being opened (MAX_PATH WCHARs).
    // Populated from FltGetFileNameInformation(FLT_FILE_NAME_NORMALIZED) in the
    // pre-create callback. Bounded copy: at most 259 WCHARs + null terminator.
    WCHAR PathBuffer[260];       // 520 bytes

    // PID of the process attempting the file open. Set from PsGetCurrentProcessId()
    // cast to ULONG in the pre-create callback.
    ULONG ProcessId;             // 4 bytes

    // Desired-access mask from the IRP. Set from
    // Data->Iopb->Parameters.Create.SecurityContext->DesiredAccess.
    ACCESS_MASK DesiredAccess;   // 4 bytes

    // Spike-only padding field for 8-byte alignment and future extensibility.
    // No ABI version field per D-04 (minimal spike design).
    ULONG Reserved;              // 4 bytes
} NONO_IPC_REQUEST, *PNONO_IPC_REQUEST;
#pragma pack(pop)

// Compile-time size assertion (C11 _Static_assert; supported in VS 2019+ / WDK).
// If the toolchain does not support _Static_assert, fall back to the WDK macro:
//   C_ASSERT(sizeof(NONO_IPC_REQUEST) == 532);
// The value 532 MUST match the Rust-side: size_of::<NonoIpcRequest>() - size_of::<FILTER_MESSAGE_HEADER>() == 532
_Static_assert(sizeof(NONO_IPC_REQUEST) == 532, "NONO_IPC_REQUEST layout changed");

// C_ASSERT fallback (uncomment if _Static_assert is not recognized by the WDK toolchain):
// C_ASSERT(sizeof(NONO_IPC_REQUEST) == 532);

// ---------------------------------------------------------------------------
// NONO_IPC_REPLY — user-to-kernel reply payload
//
// Sent by the user-mode policy client via FilterReplyMessage.
// Received by the worker thread after FltSendMessage returns STATUS_SUCCESS.
//
// Decision values:
//   0 = allow  — worker completes the IRP with STATUS_SUCCESS
//   1 = deny   — worker completes the IRP with STATUS_ACCESS_DENIED
//
// All other values default to allow (fail-open) for spike safety.
// ---------------------------------------------------------------------------
typedef struct _NONO_IPC_REPLY {
    // 0 = allow, 1 = deny. Out-of-range values treated as allow (fail-open).
    ULONG Decision;
} NONO_IPC_REPLY, *PNONO_IPC_REPLY;
