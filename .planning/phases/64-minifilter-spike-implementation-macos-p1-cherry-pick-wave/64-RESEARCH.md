# Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave — Research

**Researched:** 2026-06-08
**Domain:** Windows FltMgr kernel driver (C) + Rust user-mode IPC client + macOS Seatbelt cherry-pick porting
**Confidence:** HIGH (codebase-grounded; every claim sourced from direct file inspection, prior HIGH-confidence planning artifacts, or authoritative MSDN/WDK docs verified in prior phases)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Track A — Driver interception & deny proof (DRV-01)**
- D-01: Deny demonstration = scripted test harness. A small harness attempts to open the deny-target path and asserts the open is refused with Win32 `ERROR_ACCESS_DENIED` (5) / `STATUS_ACCESS_DENIED`. Harness output plus `fltmc instances` / `fltmc filters` captured to Phase 64 SC1 evidence artifact.
- D-02: Deny target = a single dedicated deterministic throwaway path provisioned on the VM (e.g. `C:\nono-deny-test\secret.txt`). One deterministic deny target is sufficient; the user-mode policy client hard-codes the deny rule for this one path.

**Track A — Kernel↔user IPC & Rust client (DRV-02)**
- D-03: `fltmgr_client.rs` lives in a new standalone spike crate that IS a Cargo workspace member, with ALL code `#[cfg(windows)]`. Uses `windows-sys` with the `Win32_Storage_InstallableFileSystems` feature.
- D-04: IPC message = `#[repr(C)] NonoIpcRequest` with minimal POC fields — a fixed-size WCHAR path buffer + originating PID + desired-access/operation — plus static layout assertions on both C and Rust sides. No version/request-id for the spike.
- D-05: Ring-buffer + worker-thread + finite ~500 ms `FltSendMessage` + fail-open-on-`STATUS_TIMEOUT` pattern per `DESIGN.md` (T-63-02). References `drivers/nono-fltmgr/DESIGN.md` as hard pre-code gate.

**Track A — VM, test-signing pipeline & docs (DRV-01 / DRV-03)**
- D-06: VM = fresh Azure VM via Phase 63 scripts (Win11 WDK-paired image, Standard security type, Secure-Boot OFF / HVCI off). Pre-load snapshot before `pnputil /add-driver`.
- D-07: Phase 64 completes DRV-03 = full test-signing pipeline: `makecert → inf2cat → signtool → certmgr → bcdedit /set testsigning on → pnputil /add-driver`, `SERVICE_DEMAND_START`. Capture `fltmc instances`/`fltmc filters` + D-01 deny proof.
- D-08: Altitude = enumerate `fltmc filters` on the fresh VM and pick a non-colliding number in the FSFilter Activity Monitor band (360000–389999), avoiding AV range 320000–329998. Replace `370020` placeholder in INF + `DESIGN.md`.
- D-09: `drivers/README.md` documents both pipelines end-to-end — C driver build+test-sign+load command sequence AND Rust `fltmgr_client` build/run — with exact commands + VM prerequisites. `nono-wfp-driver.sys` placeholder + MSI stay untouched.

**Track B — macOS P1 cherry-pick wave (MACOS-02)**
- D-10: Cherry-pick `8f84d454`, `362ada22`, `8f1b0b74` per C14 disposition, with verbatim D-19 `Upstream-commit:` trailers. Manual port at fork's correct call-site when needed; note site divergence in commit body. Diff-inspect each call-site before applying.
- D-11: Unit tests assert Seatbelt rule ordering (last-match-wins), not mere rule presence. Cover BOTH symlink path AND canonical `/private/etc` path for every affected deny group.
- D-12: macOS cross-target verification runs in Phase 64 — `cargo clippy`/`build` `--target x86_64-apple-darwin` AND `aarch64-apple-darwin`. If darwin cross-toolchain is not installed, mark PARTIAL and defer to live CI.

### Claude's Discretion
- Exact deny-target path string and harness language (PowerShell vs a tiny Rust/C exe)
- Exact `NonoIpcRequest` field widths (path-buffer length, access-mask type) and the static-assert value N
- The chosen altitude number within the Activity-Monitor band (executor picks after `fltmc filters` enumeration)
- The spike crate's name (e.g. `nono-fltmgr-client`) and its exact directory
- Whether the C-side static assertion uses C11 `_Static_assert` or a WDK-compatible compile-time check

### Deferred Ideas (OUT OF SCOPE)
- DRV-04 go/no-go ADR + measured round-trip latency → Phase 65
- MACOS-03 live macOS-host re-validation + green-CI hard gate → Phase 65
- EDR-01/EDR-02 HUMAN-UAT → Phase 66
- DRV-PROD-01 production EV/WHQL driver signing + MSI-bundling → future milestone
- `729697c2` `--trust-proxy-ca` (P2) + non-macOS UPST8 slice → deferred
- `NonoIpcRequest` version/request-id ABI-insurance fields → production ADR
- Official Microsoft altitude assignment → pending (~30 business days from 2026-06-07)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DRV-01 | Test-signed Windows FltMgr minifilter intercepts `IRP_MJ_CREATE` pre-op and denies a targeted file open, demonstrated end-to-end on a Secure-Boot-OFF/HVCI-off VM | Track A implementation: extend Phase 63 skeleton with pre-create callback body; scripted harness asserts `ERROR_ACCESS_DENIED`; `fltmc instances` confirms altitude registration |
| DRV-02 | Minifilter performs user-mode policy round-trip (`FLT_PREOP_PENDING` + `FltSendMessage` over `\NonoPolicyPort`) and a Rust `#[cfg(windows)]` user-mode client receives request and returns allow/deny | Track A implementation: new `nono-fltmgr-client` Cargo workspace member with `Win32_Storage_InstallableFileSystems`; `#[repr(C)] NonoIpcRequest` with static layout assertion |
| DRV-03 (complete) | Reproducible driver build + test-signing pipeline documented | Phase 63 skeleton + Phase 64 full test-signing pipeline + `drivers/README.md` |
| MACOS-02 | P1 macOS security/correctness commits `8f84d454`, `362ada22`, `8f1b0b74` absorbed with verbatim `Upstream-commit:` trailers; Seatbelt rule ordering asserted by unit tests | Track B: cherry-pick in absorption order `8f1b0b74` → `362ada22` → `8f84d454`; ordering tests in `macos.rs`; canonical `/private/etc` path coverage |
</phase_requirements>

---

## Summary

Phase 64 has two fully independent tracks. Track A extends the Phase 63 `drivers/nono-fltmgr/` C skeleton to a working end-to-end spike: adding a real `IRP_MJ_CREATE` pre-create callback, wiring the ring-buffer + worker-thread kernel↔user IPC over `\NonoPolicyPort`, and completing the full test-signing pipeline on a fresh Azure VM. Track B lands the three P1 macOS security/correctness commits from the Phase 63 ledger's C14 cluster with ordering-asserting unit tests.

Track A is the higher-risk track. The Phase 63 skeleton (`nono-fltmgr.c`) compiles but has an empty callbacks array and no communication port — Phase 64 fills both gaps. The BSOD-avoidance contract in `drivers/nono-fltmgr/DESIGN.md` is the hard pre-code gate and must be satisfied before any driver code is written. The key implementation unknowns resolved by this research are: the exact `NonoIpcRequest` field layout and static-assert size N, the precise extension points in the C skeleton, and the Phase 63 Azure test-signing scripts that are directly reusable.

Track B is well-scoped. The Phase 63 DIVERGENCE-LEDGER (63-DIVERGENCE-LEDGER.md) provides diff-inspect notes for all three P1 commits. The most important finding: the absorption order is `8f1b0b74` first (extracts `resolved_workdir`), then `362ada22` (modifies it), then `8f84d454` (the ordering fix, independent). The fork's existing `test_generate_profile_platform_rules_between_reads_and_writes` test asserts the WRONG ordering (deny between reads and writes), which is the pre-fix behavior; Phase 64's new tests must assert the POST-fix ordering (deny AFTER write allows).

**Primary recommendation:** Execute Track B first (no VM dependency, fast iteration on Windows host with darwin cross-target already installed as `x86_64-apple-darwin`); then execute Track A against the Azure VM. The darwin cross-target `x86_64-apple-darwin` is confirmed installed on this dev host; `aarch64-apple-darwin` is NOT installed — D-12 cross-target verification will be PARTIAL (mark in plan, defer `aarch64` to live CI).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Pre-create file interception + deny | Kernel driver (C, WDK) | — | Only FltMgr minifilter can intercept at `IRP_MJ_CREATE` in the kernel; ETW cannot block |
| Allow/deny policy decision | User-mode spike client (Rust, nono workspace) | — | Policy belongs in user mode; kernel sends event, user mode decides, kernel enforces |
| Kernel↔user IPC transport | `\NonoPolicyPort` FilterCommunicationPort (kernel side) + `FilterConnectCommunicationPort`/`FilterGetMessage` (user side) | — | FltMgr FilterCommunicationPort is the documented pattern; cannot reuse WFP pipe (synchronous kernel APC-based, not tokio-layerable) |
| Test-signing pipeline | Azure VM + WDK toolchain | Phase 63 PowerShell scripts (reusable) | Requires TESTSIGNING ON + HVCI OFF; dedicated VM prevents dev host BSOD risk |
| Seatbelt rule emission ordering | nono library (`crates/nono/src/sandbox/macos.rs::generate_profile`) | nono-cli (`crates/nono-cli/src/sandbox_prepare.rs`) | macos.rs owns the profile string; sandbox_prepare.rs owns CWD capability construction |
| Symlink CWD capture | nono-cli (`sandbox_prepare.rs`) | — | `resolved_workdir` function lives in sandbox_prepare.rs; `FsCapability::new_dir` call site is there |

---

## Standard Stack

### Track A — Driver (C/WDK, out-of-workspace)

| Component | Version | Purpose | Notes |
|-----------|---------|---------|-------|
| EWDK ISO (Enterprise WDK) | Used in Phase 63 Azure scripts | Self-contained build environment on VM | Phase 63's `63-vm-runcmd-ewdk-build.ps1` uses EWDK mounted at `C:\ewdk\ewdk.iso`; reuse verbatim |
| FltMgr.lib | Ships with WDK | Import library for FltMgr minifilter API | Linked via `#pragma comment(lib, "FltMgr.lib")` in .vcxproj |
| FltKernel.h | Ships with WDK | All FltMgr kernel APIs | Primary header for pre-create callback, FltCreateCommunicationPort, FltSendMessage |
| Phase 63 .vcxproj/.inf | `drivers/nono-fltmgr/` | MSBuild project template | Extend, do NOT replace; the INF altitude placeholder `370020` is updated per D-08 |

### Track A — User-Mode Client (Rust, Cargo workspace member)

| Component | Version | Purpose | Notes |
|-----------|---------|---------|-------|
| `windows-sys` | 0.59 (existing) | `FilterConnectCommunicationPort`, `FilterGetMessage`, `FilterReplyMessage` | Add feature `Win32_Storage_InstallableFileSystems` to new spike crate's Cargo.toml |
| `FILTER_MESSAGE_HEADER` (windows-sys type) | 0.59 | Message prefix struct in `FilterGetMessage` buffer | Must be the first field in the receive buffer; layout documented in WDK docs |

### Track B — macOS Cherry-picks (existing codebase, no new deps)

| File | Cherry-pick Target | Notes |
|------|--------------------|-------|
| `crates/nono/src/sandbox/macos.rs` | `8f84d454` (ordering fix) | `generate_profile` function, lines ~667–676 |
| `crates/nono-cli/src/sandbox_prepare.rs` | `8f1b0b74` (extract `resolved_workdir`), `362ada22` (add `$PWD` logic) | CWD block, lines ~455–476 |

**Installation (Cargo workspace member for spike crate):**
```bash
# Add to root Cargo.toml [workspace] members:
"crates/nono-fltmgr-client",

# In crates/nono-fltmgr-client/Cargo.toml:
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { workspace = true, features = ["Win32_Storage_InstallableFileSystems"] }
```

**Version verification:** `windows-sys 0.59` is confirmed in `crates/nono-cli/Cargo.toml` L145 [VERIFIED: codebase]; `Win32_Storage_InstallableFileSystems` feature is confirmed present at docs.rs for this version [CITED: STACK.md verified against docs.rs].

---

## Package Legitimacy Audit

No new external packages are introduced by this phase. The only Cargo change is adding a `Win32_Storage_InstallableFileSystems` feature flag to the existing `windows-sys = "0.59"` dependency — no new registry package.

| Package | Change | Disposition |
|---------|--------|-------------|
| `windows-sys 0.59` | New feature flag only (no version change) | Approved — existing dependency, previously vetted |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram (Track A — Kernel↔User IPC)

```
[Process attempts CreateFile("C:\nono-deny-test\secret.txt")]
         |
         v
[FltMgr pre-create callback — runs at APC_LEVEL or below]
         |
         | 1. NT_ASSERT(IRQL <= APC_LEVEL)
         | 2. Allocate ring-buffer slot (NonPagedPoolNx)
         | 3. Copy path + PID into slot (no file I/O here)
         | 4. Signal worker thread (KeSetEvent)
         | 5. Return FLT_PREOP_PENDING (suspend the IRP)
         |
         v
[Ring buffer — kernel memory, NonPagedPoolNx]
         |
         v
[Worker thread — PASSIVE_LEVEL]
         |
         | FltSendMessage(Timeout = -5000000LL = 500ms)
         |   STATUS_TIMEOUT → complete IRP with STATUS_SUCCESS (fail-open)
         |   STATUS_SUCCESS → read decision from reply buffer
         |
         v
[\NonoPolicyPort FilterCommunicationPort]
         |         (kernel APC-based, NOT tokio/async)
         v
[User-mode spike crate (nono-fltmgr-client)]
         |
         | FilterConnectCommunicationPort("\NonoPolicyPort")
         | FilterGetMessage(blocking loop)
         | Receive: NonoIpcRequest { path_buf[260], pid, desired_access }
         | Decision: hard-coded "if path == deny_target → deny"
         | FilterReplyMessage → NTSTATUS decision
         |
         v
[Worker thread receives reply]
         |
         | Allow → complete IRP with STATUS_SUCCESS
         | Deny  → complete IRP with STATUS_ACCESS_DENIED
         v
[CreateFile returns ERROR_ACCESS_DENIED to caller]
```

### Recommended Project Structure

```
drivers/
  nono-fltmgr/             # Out-of-workspace C MSBuild project
    nono-fltmgr.c          # DriverEntry (Phase 63) + Phase 64 additions below:
                           #   - NonoPreCreate (pre-op callback)
                           #   - NonoPortConnect / NonoPortDisconnect / NonoPortMessage
                           #   - ring buffer (g_RingBuffer, g_WorkerThread)
                           #   - FltCreateCommunicationPort call in DriverEntry
    nono-fltmgr.h          # Shared IPC struct: NonoIpcRequest, NonoIpcReply
    nono-fltmgr.vcxproj    # Extend (do not replace)
    nono-fltmgr.inf        # Update altitude from 370020 to chosen value (D-08)
    README.md              # NEW: both pipeline docs (D-09)

crates/
  nono-fltmgr-client/      # NEW: Cargo workspace member (D-03)
    Cargo.toml
    src/
      lib.rs               # #[cfg(windows)] pub fn run_policy_client(deny_path: &str)
                           # FilterConnectCommunicationPort + FilterGetMessage loop
                           # NonoIpcRequest (mirrored #[repr(C)]) + static layout assert
```

### Pattern 1: Pre-Create Callback Extension Point

The Phase 63 skeleton registers an empty `Callbacks[]` array with only the `IRP_MJ_OPERATION_END` sentinel. Phase 64 fills this array with a single pre-create entry:

```c
// Source: DESIGN.md ring-buffer pattern + nullFilter WDK sample
// [CITED: github.com/microsoft/Windows-driver-samples/tree/main/filesys/miniFilter/nullFilter]

CONST FLT_OPERATION_REGISTRATION Callbacks[] = {
    { IRP_MJ_CREATE,
      0,                          // Flags
      NonoPreCreate,              // PreOperation callback
      NULL },                     // PostOperation callback (not needed for deny)
    { IRP_MJ_OPERATION_END }
};

FLT_PREOP_CALLBACK_STATUS
NonoPreCreate(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);

    // DESIGN.md T-63-03: assert IRQL before any allocation
    NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL);

    // Get file name (normalized — resolves symlinks for accurate path matching)
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    NTSTATUS status = FltGetFileNameInformation(
        Data, FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT, &nameInfo);
    if (!NT_SUCCESS(status)) {
        // Fail-open: if we can't get the name, permit the open
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    FltParseFileNameInformation(nameInfo);

    // DESIGN.md T-63-01: NO ZwCreateFile / NtCreateFile here.
    // Enqueue to ring buffer; worker thread does the FltSendMessage.
    NonoEnqueueEvent(nameInfo, Data);
    FltReleaseFileNameInformation(nameInfo);

    // Return PENDING — IRP is suspended until worker thread completes it.
    return FLT_PREOP_PENDING;
}
```

[CITED: DESIGN.md ring-buffer contract; WDK FltGetFileNameInformation docs]

### Pattern 2: FltCreateCommunicationPort Call in DriverEntry

Add after `FltStartFiltering` in the existing `DriverEntry`:

```c
// Source: DESIGN.md §IPC Design, STACK.md §A2
// [CITED: learn.microsoft.com/windows-hardware/drivers/ifs/communication-between-user-mode-and-kernel-mode]
PFLT_PORT gServerPort = NULL;
PFLT_PORT gClientPort = NULL;

// In DriverEntry, after FltStartFiltering succeeds:
UNICODE_STRING portName = RTL_CONSTANT_STRING(L"\\NonoPolicyPort");
OBJECT_ATTRIBUTES oa;
InitializeObjectAttributes(&oa, &portName, OBJ_KERNEL_HANDLE | OBJ_CASE_INSENSITIVE, NULL, NULL);

status = FltCreateCommunicationPort(
    gFilterHandle, &gServerPort, &oa, NULL,
    NonoPortConnect, NonoPortDisconnect, NonoPortMessage,
    1 /* max connections */);
if (!NT_SUCCESS(status)) {
    FltUnregisterFilter(gFilterHandle);
    gFilterHandle = NULL;
    return status;
}
// Also start the worker thread here (NonoWorkerThread)
```

### Pattern 3: `NonoIpcRequest` Struct — Field Sizing (D-04 resolution)

D-04 mandates: fixed-size WCHAR path buffer + PID + desired-access/operation. The WDK convention for path buffers uses `MAX_PATH` (260) WCHARs. The struct must be preceded by `FILTER_MESSAGE_HEADER` on the receive side.

**Recommended field layout (C side):**

```c
// Source: D-04 "minimal POC fields"; WDK FILTER_MESSAGE_HEADER convention
// [CITED: DESIGN.md §V5 Input Validation Note; STACK.md §A3]
#include <fltKernel.h>

#pragma pack(push, 1)
typedef struct _NONO_IPC_REQUEST {
    // Path of the file being opened (null-terminated WCHAR, normalized form)
    WCHAR PathBuffer[260];   // MAX_PATH WCHARs = 520 bytes
    ULONG ProcessId;         // Requestor PID (from IoGetRequestorProcess / PsGetProcessId)
    ACCESS_MASK DesiredAccess; // From Data->Iopb->Parameters.Create.SecurityContext->DesiredAccess
    ULONG Reserved;          // Padding to align to 8 bytes (spike-only; no ABI version field per D-04)
} NONO_IPC_REQUEST, *PNONO_IPC_REQUEST;
#pragma pack(pop)

// Static size assertion (C11 _Static_assert; WDK supports this in VS 2019+)
// Size: 520 (path) + 4 (pid) + 4 (access) + 4 (reserved) = 532 bytes
_Static_assert(sizeof(NONO_IPC_REQUEST) == 532, "NONO_IPC_REQUEST layout changed");

typedef struct _NONO_IPC_REPLY {
    ULONG Decision;  // 0 = allow, 1 = deny
} NONO_IPC_REPLY, *PNONO_IPC_REPLY;
```

**Matching Rust struct (user-mode side):**

```rust
// Source: D-04; windows-sys FILTER_MESSAGE_HEADER layout
// [CITED: STACK.md §A3; docs.rs windows-sys 0.59]
#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::InstallableFileSystems::FILTER_MESSAGE_HEADER;

#[cfg(target_os = "windows")]
#[repr(C, packed(1))]
pub struct NonoIpcRequest {
    // FILTER_MESSAGE_HEADER must be first in the receive buffer
    pub header: FILTER_MESSAGE_HEADER,
    pub path_buffer: [u16; 260],     // MAX_PATH WCHARs
    pub process_id: u32,
    pub desired_access: u32,
    pub reserved: u32,
}

#[cfg(target_os = "windows")]
const _: () = assert!(
    std::mem::size_of::<NonoIpcRequest>() - std::mem::size_of::<FILTER_MESSAGE_HEADER>() == 532,
    "NonoIpcRequest payload size mismatch with C-side NONO_IPC_REQUEST"
);
```

The static assert subtracts `FILTER_MESSAGE_HEADER` from the Rust side because the C side's `NONO_IPC_REQUEST` does not include the header (it is prepended by `FilterGetMessage`). The assertion value `532` must match the C-side `_Static_assert`. [ASSUMED — field sizes derived from WDK types; verify against actual `sizeof` on the VM build]

### Pattern 4: FltSendMessage with Ring Buffer (DESIGN.md contract)

```c
// Worker thread body — runs at PASSIVE_LEVEL
// Source: DESIGN.md §IPC Design Rule 2 (finite timeout mandatory)
// [CITED: DESIGN.md T-63-02; PITFALLS.md Pitfall 3]
static VOID NonoWorkerThread(PVOID Context)
{
    UNREFERENCED_PARAMETER(Context);
    while (gWorkerRunning) {
        // Wait for ring-buffer event (no lock held here — DESIGN.md Rule 5)
        KeWaitForSingleObject(&g_RingBufferEvent, Executive, KernelMode, FALSE, NULL);

        PNONO_RING_ENTRY entry = NonoDequeueEvent();
        if (!entry) continue;

        NONO_IPC_REQUEST req = { 0 };
        // Copy from ring entry (already done at enqueue time; no file I/O)
        RtlCopyMemory(req.PathBuffer, entry->PathBuffer, sizeof(req.PathBuffer));
        req.ProcessId = entry->ProcessId;
        req.DesiredAccess = entry->DesiredAccess;

        NONO_IPC_REPLY reply = { 0 };
        ULONG replyLen = sizeof(reply);

        // DESIGN.md T-63-02: finite timeout = 500ms = -5000000 * 100ns units
        LARGE_INTEGER timeout;
        timeout.QuadPart = -5000000LL;

        NTSTATUS sendStatus = FltSendMessage(
            gFilterHandle, &gClientPort,
            &req, sizeof(req),
            &reply, &replyLen,
            &timeout);

        if (sendStatus == STATUS_TIMEOUT || !NT_SUCCESS(sendStatus)) {
            // DESIGN.md fail-open: permit I/O on timeout (spike only)
            NonoCompleteIrp(entry->Data, STATUS_SUCCESS);
        } else {
            NTSTATUS decision = (reply.Decision == 1)
                ? STATUS_ACCESS_DENIED
                : STATUS_SUCCESS;
            NonoCompleteIrp(entry->Data, decision);
        }
        NonoFreeRingEntry(entry);
    }
    PsTerminateSystemThread(STATUS_SUCCESS);
}
```

### Anti-Patterns to Avoid

- **`ZwCreateFile` / `NtCreateFile` in any callback:** Pitfall 2 recursion BSOD. All data passes through `FltSendMessage` to user mode. `DESIGN.md` T-63-01.
- **NULL timeout in `FltSendMessage`:** Pitfall 3 system hang. Timeout = `-5000000LL` always.
- **`PagedPool` allocation in callback context:** Pitfall 1 IRQL BSOD. Use `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, ...)`.
- **Holding a spinlock across `FltSendMessage`:** IRQL mismatch BSOD. Enqueue atomically, release lock, then send. `DESIGN.md` Rule 5.
- **Altitude in AV range 320000–329998:** Pitfall 5 EDR collision. Choose Activity Monitor band. `DESIGN.md` altitude table.
- **`FLT_PREOP_COMPLETE` in the pre-create callback (not `FLT_PREOP_PENDING`):** A synchronous deny in the callback violates the ring-buffer + worker-thread pattern; use `FLT_PREOP_PENDING` + complete the IRP from the worker thread.
- **Committing the `.sys` binary:** Spike `.sys` is VM-local throwaway. Never committed (DESIGN.md T-63-05).
- **Using the existing WFP named pipe for IPC:** FltMgr FilterCommunicationPort uses synchronous kernel APC-based IPC; it cannot layer on a tokio async named pipe. Use `\NonoPolicyPort` exclusively.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Kernel↔user communication | Custom shared-memory or named-pipe channel | FltMgr `FltCreateCommunicationPort` + `FilterConnectCommunicationPort` | The documented, quota-protected, kernel-safe pattern; handles back-pressure, port lifetime, and ACL |
| WCHAR path string from callback | Manual `PUNICODE_STRING` parsing | `FltGetFileNameInformation` + `FltParseFileNameInformation` | Handles normalized name, reparse points, deduplication cache |
| C-side compile-time size assertion | `#if sizeof(...)` (illegal in C) | `_Static_assert(sizeof(X) == N, "msg")` (C11, VS 2019+/WDK) | Compiler-enforced, zero runtime cost |
| Rust side size assertion | Unit test with `assert_eq!(size_of::<X>(), N)` | `const _: () = assert!(size_of::<X>() == N, "msg")` | Fails at compile time, not test time |
| Test-signing pipeline | Manual per-step execution | Reuse Phase 63 Azure PowerShell scripts verbatim | Tested scripts with 7-minute timeout + re-runnable logic |
| macOS cherry-pick conflict resolution | Guess at correct call-site | Consult 63-DIVERGENCE-LEDGER.md diff-inspect notes | Per-commit notes already identify exact lines and divergence |

**Key insight:** The FltMgr kernel↔user IPC pattern (`FltSendMessage` / `FilterGetMessage`) has exactly one correct implementation shape; diverging from it by even one parameter causes either a BSOD (NULL timeout), silent data corruption (wrong message buffer layout), or failure to connect (wrong port ACL). The DESIGN.md pre-code gate encodes the safe shape; do not improvise.

---

## Track A: Extension Points in the Phase 63 Skeleton

This section gives the planner the exact locations where Phase 64 code hooks into the existing C file.

### `nono-fltmgr.c` — What Exists (Phase 63)

The skeleton has four elements:
1. `gFilterHandle` global (extend: add `gServerPort`, `gClientPort`, `gRingBuffer*`, `gWorkerThread`)
2. `NonoFltUnload` callback (extend: add `FltCloseCommunicationPort(gServerPort)` + worker thread stop signal before `FltUnregisterFilter`)
3. `Callbacks[]` array (extend: replace the sentinel-only array with the pre-create entry + sentinel)
4. `FilterRegistration` struct (extend: add `InstanceTeardownStartCallback` for port cleanup)
5. `DriverEntry` (extend: after `FltStartFiltering`, add `FltCreateCommunicationPort` + worker thread start)

### `nono-fltmgr.inf` — What Changes (Phase 64)

Only one value changes: `Instance1.Altitude = "370020"` becomes the actual non-colliding altitude selected after `fltmc filters` enumeration on the fresh VM (D-08). All other INF content is unchanged.

### `nono-fltmgr.vcxproj` — No Changes Needed

The Phase 63 vcxproj already targets Release|x64 with WDK toolchain. No changes needed for Phase 64 (same source file extended in-place).

---

## Track B: macOS Cherry-Pick Execution Details

### Absorption Order (from 63-DIVERGENCE-LEDGER.md C14)

**MANDATORY: `8f1b0b74` FIRST, then `362ada22`, then `8f84d454`**

Rationale from diff-inspect: `8f1b0b74` extracts the `resolved_workdir` helper function from the inline CWD expression; `362ada22` then modifies that function to add `$PWD` preference logic. Applying `362ada22` before `8f1b0b74` would target a function that does not yet exist.

`8f84d454` (the ordering fix in `macos.rs`) is independent of both and can be applied in any relative position, but absorbing all three in this order matches the upstream commit series and simplifies any conflict resolution.

### Per-Commit Call-Site Details

**`8f1b0b74` — "preserve symlink path when adding CWD capability on macOS"**
- **File:** `crates/nono-cli/src/sandbox_prepare.rs`
- **Current fork call-site (confirmed by direct read, 2026-06-08):**
  ```rust
  // Lines ~455-476 in sandbox_prepare.rs:
  let cwd_canonical = workdir.canonicalize()...;
  if !caps.path_covered_with_access(&cwd_canonical, access) {
      ...
      let cap = FsCapability::new_dir(cwd_canonical.clone(), access)?;
      caps.add_fs(cap);
  ```
  The fork uses `cwd_canonical` (already-resolved path) for `FsCapability::new_dir` — this is the pre-fix pattern. The symlink path (`workdir`) is never passed to `FsCapability`.
- **What the commit adds:** `resolved_workdir` function + `#[cfg(target_os = "macos")]` block that emits a second `FsCapability::new_dir(workdir, access)` when `workdir != workdir.canonicalize()`.
- **Apply cleanly?** YES — the targeted lines exist verbatim in the fork. [VERIFIED: direct source read 2026-06-08]

**`362ada22` — "use $PWD to capture symlink CWD without --workdir"**
- **File:** `crates/nono-cli/src/sandbox_prepare.rs`
- **Current fork call-site:** The fork does NOT have a `resolved_workdir` function; it uses the inline expression `args.workdir.clone().or_else(|| std::env::current_dir().ok()).unwrap_or_else(|| PathBuf::from("."))` — this is equivalent to the pre-fix upstream code.
- **What the commit modifies:** The `resolved_workdir` function (introduced by `8f1b0b74`) to add `std::env::var("PWD").ok().map(PathBuf::from)` preference before `current_dir()`.
- **Apply cleanly?** YES, **after** `8f1b0b74` is applied. Cannot apply before. [VERIFIED: 63-DIVERGENCE-LEDGER.md diff-inspect + direct source read 2026-06-08]

**`8f84d454` — "emit platform rules after user write allows"**
- **File:** `crates/nono/src/sandbox/macos.rs`
- **Current fork call-site (confirmed by direct read, 2026-06-08):**
  ```
  Lines ~667-676: platform_rules() loop emits BETWEEN read-allows (lines ~640-654)
  and write-allows (lines ~702-717). The comment says:
  "Platform deny rules are placed BETWEEN read and write rules."
  ```
  This is the PRE-FIX ordering. The current unit test `test_generate_profile_platform_rules_between_reads_and_writes` (line 998) ASSERTS this wrong ordering (`read_pos < deny_pos < write_pos`).
- **What the commit does:** Moves the `platform_rules()` loop to AFTER the write-allows loop.
- **Apply cleanly?** YES — block reordering within `generate_profile`, no new symbols introduced. [VERIFIED: direct source read 2026-06-08]
- **CRITICAL:** The existing test `test_generate_profile_platform_rules_between_reads_and_writes` will FAIL after applying this commit (it asserts the old ordering). Phase 64 must:
  1. Apply the commit
  2. Update this existing test to assert the NEW ordering: `read_pos < write_pos < deny_pos`
  3. Add new tests per D-11 (symlink + canonical `/private/etc` path coverage)

### D-11 Ordering Test Requirements

New tests in `crates/nono/src/sandbox/macos.rs` (module `tests`):

```rust
// Test: platform deny rules appear AFTER write allows (post-fix ordering)
fn test_platform_rules_after_write_allows() {
    // Add a ReadWrite capability + a platform deny rule
    // Assert: write_pos < deny_pos (deny comes after write)
}

// Test: symlink path coverage — platform deny must block both /etc/... and /private/etc/...
fn test_platform_deny_covers_symlink_and_canonical_path() {
    // Add platform rule "(deny file-read* (literal \"/etc/passwd\"))"
    // and "(deny file-read* (literal \"/private/etc/passwd\"))"
    // Assert both strings appear in the profile
}

// Update existing test:
// test_generate_profile_platform_rules_between_reads_and_writes
// Change assertion from: read_pos < deny_pos < write_pos
// To: read_pos < write_pos < deny_pos (post-fix: deny AFTER write)
```

**Why the ordering matters:** Seatbelt last-match-wins. If `(allow file-write* (subpath "/home/alice"))` appears AFTER `(deny file-write-data (subpath "/home/alice/.ssh"))`, the allow wins and the deny is silently ignored. The fix ensures deny rules come last. [CITED: PITFALLS.md Pitfall 10]

---

## Common Pitfalls

### Pitfall A: FLT_PREOP_PENDING With No Completion — IRP Leak

**What goes wrong:** The pre-create callback returns `FLT_PREOP_PENDING` (suspending the IRP) but the worker thread exits or fails before calling `FltCompletePendingPreOp` / setting the IRP's `IoStatus`. The IRP is stuck forever; the calling thread is frozen.

**How to avoid:** The worker thread must ALWAYS complete the pending IRP — either allow (`STATUS_SUCCESS`) or deny (`STATUS_ACCESS_DENIED`) or fall through with fail-open (`STATUS_SUCCESS`) on `STATUS_TIMEOUT`. Add the `InstanceTeardownStartCallback` to drain the ring buffer and complete all pending IRPs before unload. [CITED: DESIGN.md Design Rule 2; PITFALLS.md Pitfall 3]

**Warning signs:** Explorer.exe hangs after driver load; `!irp` in WinDbg shows an `IRP_MJ_CREATE` stuck with no completion pending.

### Pitfall B: `FILTER_MESSAGE_HEADER` Missing from Rust Receive Buffer

**What goes wrong:** `FilterGetMessage` writes `FILTER_MESSAGE_HEADER` as a prefix before the payload. If the Rust receive buffer is declared as just `NonoIpcRequest` without the header, the fields are shifted by `sizeof(FILTER_MESSAGE_HEADER)` bytes and all path data is garbage.

**How to avoid:** The Rust receive struct must have `FILTER_MESSAGE_HEADER` as its FIRST field. The C-side `NONO_IPC_REQUEST` does NOT include the header (it is the message payload); the Rust side must include it in the `FilterGetMessage` buffer. The static-assert must account for this: assert the payload size (excluding header) matches the C-side struct. [CITED: STACK.md §A3; WDK FilterGetMessage docs]

**Warning signs:** Path string in `NonoIpcRequest.path_buffer` is all zeros or garbage on the Rust side.

### Pitfall C: Existing macOS Test Asserts Wrong Ordering

**What goes wrong:** `test_generate_profile_platform_rules_between_reads_and_writes` in `macos.rs` (line 998) currently ASSERTS the pre-fix ordering (`read_pos < deny_pos < write_pos`). After cherry-picking `8f84d454`, this test will fail. If the planner treats this as a "broken test" requiring a skip annotation rather than a fix, the ordering regression is hidden.

**How to avoid:** The cherry-pick task must update this test to assert `read_pos < write_pos < deny_pos`. This is not a test to skip — it is the primary validator of the security fix. [VERIFIED: direct source read 2026-06-08, lines 998–1030]

### Pitfall D: `aarch64-apple-darwin` Target Not Installed (D-12 PARTIAL)

**What goes wrong:** D-12 requires `cargo clippy --target aarch64-apple-darwin`. The `aarch64-apple-darwin` target is NOT installed on this dev host (only `x86_64-apple-darwin` is installed). Attempting `cargo clippy --target aarch64-apple-darwin` will fail.

**How to avoid:** Run `x86_64-apple-darwin` locally (it is installed). Mark `aarch64-apple-darwin` as PARTIAL in the verification artifact, deferring to live CI per the CLAUDE.md cross-target MUST rule. Do NOT skip `x86_64-apple-darwin`. [VERIFIED: `rustup target list --installed` output 2026-06-08]

### Pitfall E: Altitude Collision on Fresh VM

**What goes wrong:** The fresh Azure VM may have Sysmon (altitude 385201) or Windows Defender (various altitudes in the Activity Monitor range) pre-installed. Using the placeholder `370020` without checking may collide.

**How to avoid:** Run `fltmc filters` on the fresh VM BEFORE installing the spike driver. The altitude enumeration is a required step before `pnputil /add-driver`. Pick the lowest unused number in 360000–389999 that is not occupied. [CITED: DESIGN.md altitude table; PITFALLS.md Pitfall 5]

**Warning signs:** `pnputil /add-driver` reports success but `fltmc instances` does not show the driver. Check Event Viewer for FilterManager Event ID 3.

### Pitfall F: FltCreateCommunicationPort ACL Allows Only LocalSystem

**What goes wrong:** `FltCreateCommunicationPort` uses a default security descriptor that only allows kernel callers. The user-mode `FilterConnectCommunicationPort` call from a standard user process returns `ERROR_ACCESS_DENIED` (not the same as the IRP deny — this is a connection permission error).

**How to avoid:** Pass an explicit security descriptor to `FltCreateCommunicationPort` that grants GENERIC_READ + GENERIC_WRITE to the current user (or Everyone for the spike). The nullFilter sample demonstrates this with `FltBuildDefaultSecurityDescriptor`. For the spike, a permissive descriptor is acceptable; production would scope it to the supervisor process's SID. [ASSUMED — based on WDK documentation pattern; verify against actual connection test on VM]

---

## Code Examples

### FilterGetMessage Loop (Rust user-mode client)

```rust
// Source: STACK.md §A3; docs.rs windows-sys 0.59 Win32::Storage::InstallableFileSystems
// [CITED: STACK.md A3]
#[cfg(target_os = "windows")]
pub fn run_policy_client(deny_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::InstallableFileSystems::{
        FilterConnectCommunicationPort, FilterGetMessage, FilterReplyMessage,
        FILTER_MESSAGE_HEADER, FILTER_REPLY_HEADER,
    };

    let port_name: Vec<u16> = OsStr::new("\\NonoPolicyPort")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: FilterConnectCommunicationPort takes a null-terminated wide string port name
    let port = unsafe {
        FilterConnectCommunicationPort(port_name.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null_mut())
    };
    if port == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return Err("FilterConnectCommunicationPort failed".into());
    }

    loop {
        let mut buf = NonoIpcRequest::default(); // FILTER_MESSAGE_HEADER + payload
        let buf_size = std::mem::size_of::<NonoIpcRequest>() as u32;
        // SAFETY: buf is a valid, aligned buffer of the correct size
        let hr = unsafe {
            FilterGetMessage(port, &mut buf.header as *mut _, buf_size, std::ptr::null_mut())
        };
        if hr != 0 { break; }

        // Check if path matches deny target
        let path = String::from_utf16_lossy(&buf.path_buffer);
        let path = path.trim_end_matches('\0');
        let decision: u32 = if path.eq_ignore_ascii_case(deny_path) { 1 } else { 0 };

        // Send reply
        #[repr(C)]
        struct Reply { header: FILTER_REPLY_HEADER, decision: u32 }
        let mut reply = Reply {
            header: FILTER_REPLY_HEADER {
                Status: 0,
                MessageId: buf.header.MessageId,
            },
            decision,
        };
        // SAFETY: reply is a valid, aligned buffer
        unsafe { FilterReplyMessage(port, &mut reply.header as *mut _, std::mem::size_of::<Reply>() as u32) };
    }
    Ok(())
}
```

### Scripted Deny Harness (PowerShell — D-01)

```powershell
# D-01 deny proof harness: assert CreateFile on deny-target returns ERROR_ACCESS_DENIED
# Source: D-02 (deterministic throwaway path); D-01 (scripted + assert exact Win32 error)
# [CITED: 64-CONTEXT.md D-01/D-02]
$denyPath = "C:\nono-deny-test\secret.txt"
New-Item -ItemType Directory -Force "C:\nono-deny-test" | Out-Null
Set-Content -Path $denyPath -Value "deny-target"

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class FileTest {
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern IntPtr CreateFile(string lpFileName, uint dwDesiredAccess,
        uint dwShareMode, IntPtr lpSecurityAttributes, uint dwCreationDisposition,
        uint dwFlagsAndAttributes, IntPtr hTemplateFile);
    public static int LastError() { return Marshal.GetLastWin32Error(); }
}
'@

$h = [FileTest]::CreateFile($denyPath, 0x80000000, 0, [IntPtr]::Zero, 3, 0, [IntPtr]::Zero)
$err = [FileTest]::LastError()

if ($h.ToInt64() -eq -1 -and $err -eq 5) {
    Write-Output "SC1 PASS: CreateFile denied with ERROR_ACCESS_DENIED (5) as expected"
} else {
    Write-Output ("SC1 FAIL: h={0} err={1}" -f $h, $err)
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Seatbelt platform rules between reads and writes | Platform rules AFTER write allows (8f84d454) | v0.58.0 | Security: deny-after-allow ordering ensures targeted denies win over user write-allows |
| `current_dir()` only for CWD capability | `$PWD` preferred, falls back to `current_dir()` (362ada22) | v0.58.0 | Correctness: preserves symlink path in CWD capability so Seatbelt allows the symlink traversal |
| CWD capability uses canonical path only | CWD capability emits BOTH symlink and canonical path on macOS (8f1b0b74) | v0.58.0 | Correctness: prevents EPERM when CWD is reached via a symlink (e.g. /tmp → /private/tmp) |

**Deprecated/outdated:**
- `ExAllocatePoolWithTag(NonPagedPoolNx, ...)`: Deprecated API in WDK 21H1+. Use `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, size, tag)` instead. The WDK in the EWDK image will emit a deprecation warning; switch to `ExAllocatePool2`. [CITED: PITFALLS.md Pitfall 1 footnote; WDK migration guide]
- `FLT_PREOP_COMPLETE` return from callback for deny: Valid but bypasses the ring-buffer pattern; forces synchronous policy evaluation at callback time. Use `FLT_PREOP_PENDING` instead.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `NonoIpcRequest` static-assert value = 532 bytes (260 WCHARs × 2 + 4 + 4 + 4, packed) | Standard Stack / Code Examples | Mismatch between C and Rust static asserts; both would fail at compile time → caught before VM test |
| A2 | `FltCreateCommunicationPort` with default SD allows only kernel callers; needs explicit SD for user-mode connection | Common Pitfalls (Pitfall F) | `FilterConnectCommunicationPort` from user-mode returns `ERROR_ACCESS_DENIED` → driver loads but client cannot connect; easily fixed on VM |
| A3 | `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, ...)` is available in the EWDK image used by Phase 63 scripts | Code Examples | May need to use `ExAllocatePoolWithTag(NonPagedPoolNx, ...)` fallback; both produce correct NonPagedPoolNx allocation |
| A4 | `_Static_assert` is supported in the WDK/VS build environment used by Phase 63 EWDK | Code Examples | May need `C_ASSERT(sizeof(...) == N)` (WDK macro alternative) or `STATIC_ASSERT` |

**If this table is non-empty:** Claims A1–A4 are sizing/API details that are straightforward to verify on the VM build and do not affect architectural decisions. The planner should add a "verify static assert compiles" step in the Track A wave.

---

## Open Questions (RESOLVED)

> All three questions below carry an inline recommendation that the Phase 64 plans implement
> (port-ACL fallback → Plan 04 checkpoint; single-slot ring buffer → Plan 02; message-id
> correlation trivial for the single-connection spike). None are open blockers for planning.

1. **Port ACL for `FltCreateCommunicationPort` in the spike context**
   - What we know: The nullFilter sample uses `FltBuildDefaultSecurityDescriptor` which restricts to the process SID.
   - What's unclear: For the spike, the user-mode client runs as the currently logged-in user; the SD must grant that user access. The `FltBuildDefaultSecurityDescriptor` with `FLT_PORT_ALL_ACCESS` for a standard user may or may not work depending on integrity level.
   - Recommendation: During Track A implementation, if `FilterConnectCommunicationPort` fails, add `Everyone` DACL to the SD as the unblocking step (acceptable for a spike).

2. **Ring buffer implementation complexity for the spike**
   - What we know: DESIGN.md requires a ring buffer but the spike needs only one concurrent connection (D-05: `max connections = 1`).
   - What's unclear: A true ring buffer (lock-free or spinlock-protected) is correct but adds 100+ lines. For a one-connection spike, a simpler single-slot queue with a KEVENT suffices.
   - Recommendation: Implement a single-slot "ring buffer" with a KEVENT and a KSPIN_LOCK for the spike. This satisfies DESIGN.md's intent (no blocking in callback context; worker thread does the send) without over-engineering. Planner can note this as an acceptable simplification.

3. **`FILTER_MESSAGE_HEADER.MessageId` vs correlation tracking**
   - What we know: `FilterGetMessage` fills `FILTER_MESSAGE_HEADER.MessageId` with a unique ID; `FilterReplyMessage` echoes it back to complete the specific pending `FltSendMessage`.
   - What's unclear: The current spike design handles one in-flight request at a time (single-slot, max 1 connection). With `max connections = 1`, there is only one pending `FltSendMessage` at a time, so `MessageId` correlation is trivially correct.
   - Recommendation: The planner does not need to address multi-flight correlation for the spike.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `x86_64-apple-darwin` rust target | D-12 macOS cross-target clippy | ✓ | Installed (rustup confirmed) | — |
| `aarch64-apple-darwin` rust target | D-12 macOS cross-target clippy | ✗ | Not installed | Mark D-12 PARTIAL; defer aarch64 to live CI |
| Azure VM (Win11, Standard security, Secure-Boot OFF) | Track A driver build + test-sign | Requires provisioning | Via Phase 63 Azure scripts | Phase 63 scripts reusable verbatim |
| EWDK ISO on VM | Track A EWDK driver build | Requires download via `63-vm-runcmd-ewdk-download.ps1` | Phase 63 script downloads it | No fallback (WDK required) |
| `az` CLI on dev host | Azure VM provisioning | Not verified | Unknown | User provisions VM manually using `63-preflight-azure.ps1` as reference |

**Missing dependencies with no fallback:**
- EWDK on the Azure VM — required for driver build; `63-vm-runcmd-ewdk-download.ps1` handles this automatically.

**Missing dependencies with fallback:**
- `aarch64-apple-darwin`: `x86_64-apple-darwin` is installed and sufficient for local cross-target verification; `aarch64` defers to CI.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (cargo test) |
| Config file | None — workspace-level |
| Quick run (macOS-related) | `cargo test -p nono -- sandbox::macos -x` |
| Full suite command | `make test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DRV-01 | Scripted harness opens deny-target path; asserts `ERROR_ACCESS_DENIED` (5) | Manual (VM-only) | Harness script on Azure VM | ❌ Wave 0 (new script) |
| DRV-01 | `fltmc instances` shows driver at chosen altitude | Manual (VM-only) | VM-side PowerShell check | ❌ Wave 0 (new script) |
| DRV-02 | `NonoIpcRequest` Rust struct size matches C-side `_Static_assert` | Unit (compile-time) | `cargo build -p nono-fltmgr-client` | ❌ Wave 0 (new crate) |
| DRV-02 | User-mode client receives path+PID from driver and returns allow/deny | Integration (VM-only) | Manual on Azure VM | Manual-only |
| DRV-03 | Driver builds and loads on test VM | Manual (VM-only) | `fltmc instances` output | Manual-only |
| MACOS-02 | Platform deny rules appear AFTER write-allows in generated profile | Unit | `cargo test -p nono -- sandbox::macos::tests::test_platform_rules_after_write_allows` | ❌ Wave 0 (new test) |
| MACOS-02 | Seatbelt profile covers both `/etc/...` and `/private/etc/...` for deny rules | Unit | `cargo test -p nono -- sandbox::macos::tests::test_platform_deny_symlink_and_canonical` | ❌ Wave 0 (new test) |
| MACOS-02 | Existing ordering test updated to assert post-fix ordering | Unit | `cargo test -p nono -- sandbox::macos::tests::test_generate_profile_platform_rules_between_reads_and_writes` | ✅ exists but wrong assertion |
| MACOS-02 | `cargo clippy --target x86_64-apple-darwin` passes on cherry-picked files | Cross-target build | `cargo clippy -p nono -p nono-cli --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ target installed |

### Sampling Rate

- **Per task commit:** `cargo test -p nono -- sandbox::macos` (Track B tasks); Track A has no automated tests (VM-only)
- **Per wave merge:** `make test` (Track B wave); Track A wave: VM-side `fltmc instances` + deny harness
- **Phase gate:** All Track B unit tests green + `x86_64-apple-darwin` clippy green + Track A VM evidence captured before `/gsd:verify-work`

### Wave 0 Gaps (must exist before implementation begins)

Track B:
- [ ] `crates/nono/src/sandbox/macos.rs` — new tests: `test_platform_rules_after_write_allows`, `test_platform_deny_symlink_and_canonical`
- [ ] Update existing test `test_generate_profile_platform_rules_between_reads_and_writes` to assert post-fix ordering

Track A:
- [ ] `crates/nono-fltmgr-client/` — new Cargo workspace member (Cargo.toml + `src/lib.rs` skeleton)
- [ ] Root `Cargo.toml` — add `"crates/nono-fltmgr-client"` to `[workspace] members`

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Spike has no auth surface |
| V3 Session Management | no | No session state |
| V4 Access Control | yes (core feature) | Kernel pre-create deny; Seatbelt last-match-wins ordering |
| V5 Input Validation | yes | `FltGetFileNameInformation` (normalized name); `NonoIpcRequest` fixed-size buffer (no unbounded string copy); static layout assertion |
| V6 Cryptography | no | No crypto in spike |

### Known Threat Patterns for This Phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| IRQL violation in callback (Pitfall 1) | Denial of Service | `NonPagedPoolNx` only; `NT_ASSERT(IRQL <= APC_LEVEL)`; worker thread for FltSendMessage |
| Own-I/O recursion (Pitfall 2) | Denial of Service | No `ZwCreateFile`/`NtCreateFile` anywhere in driver |
| Blocking FltSendMessage (Pitfall 3) | Denial of Service | Finite 500ms timeout; fail-open on STATUS_TIMEOUT |
| Altitude collision with EDR (Pitfall 5) | Tampering / Elevation | `fltmc filters` enumeration before load; Activity Monitor band only |
| Seatbelt deny-after-allow bypass (Pitfall 10) | Elevation of Privilege | Platform rules emitted AFTER write-allows; ordering unit tests assert position |
| macOS `/private/etc` symlink bypass (Pitfall 11) | Information Disclosure | Emit both symlink and canonical path for every macOS deny literal |
| `NonoIpcRequest` buffer overflow in kernel | Tampering | Fixed-size path buffer (MAX_PATH = 260 WCHARs); `FltGetFileNameInformation` truncates to buffer size |

---

## Sources

### Primary (HIGH confidence — direct codebase inspection 2026-06-08)

- `drivers/nono-fltmgr/nono-fltmgr.c` — Phase 63 skeleton: DriverEntry, empty Callbacks[], NonoFltUnload
- `drivers/nono-fltmgr/nono-fltmgr.inf` — altitude placeholder `370020`, INF structure
- `drivers/nono-fltmgr/DESIGN.md` — BSOD-avoidance pre-code gate (T-63-01..05), ring-buffer + worker-thread IPC contract, finite timeout rule, altitude band
- `crates/nono/src/sandbox/macos.rs` lines 640–717 — `generate_profile` rule emission order (platform rules currently between reads and writes, pre-fix)
- `crates/nono/src/sandbox/macos.rs` lines 997–1030 — `test_generate_profile_platform_rules_between_reads_and_writes` (asserts wrong ordering — must be updated)
- `crates/nono-cli/src/sandbox_prepare.rs` lines ~233–476 — CWD workdir/current_dir inline expression (pre-fix pattern), `FsCapability::new_dir(cwd_canonical.clone(), access)` call site
- `.planning/phases/63-minifilter-spike-groundwork-macos-divergence-ledger-audit/63-DIVERGENCE-LEDGER.md` — C14 diff-inspect notes for `8f84d454`, `362ada22`, `8f1b0b74`; absorption order; call-site match verification
- `.planning/phases/63-.../63-vm-runcmd-ewdk-build.ps1` — reusable EWDK build script for Azure VM
- `Cargo.toml` — workspace members (confirmed 6 members); `rust-version = "1.95"`

### Primary (HIGH confidence — prior planning artifacts)

- `.planning/research/PITFALLS.md` — Pitfalls 1–11 with prevention/warning signs [HIGH: codebase-grounded, 2026-06-06]
- `.planning/research/STACK.md` — WDK toolchain, `Win32_Storage_InstallableFileSystems` feature confirmed in docs.rs, FilterCommunicationPort API surface [HIGH: 2026-06-06]
- `.planning/research/ARCHITECTURE.md` — component boundary, spike C vs Rust split, WFP pipe independence, `fltmgr_client.rs` placement [HIGH: 2026-06-06]

### Secondary (MEDIUM confidence — cited official sources verified in prior phases)

- [Microsoft WDK: FltSendMessage](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/fltkernel/nf-fltkernel-fltsendmessage) — timeout parameter, STATUS_TIMEOUT behavior [CITED: STACK.md]
- [Microsoft WDK: Communication Between User-Mode and Minifilters](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/communication-between-user-mode-and-kernel-mode) — FilterCommunicationPort API shape [CITED: STACK.md]
- [nullFilter WDK sample](https://github.com/microsoft/Windows-driver-samples/tree/main/filesys/miniFilter/nullFilter) — altitude 370020, INF structure, callback array shape [CITED: nono-fltmgr.c source header comment]
- [docs.rs: windows-sys 0.59 Win32::Storage::InstallableFileSystems](https://docs.rs/windows-sys/latest/windows_sys/Win32/Storage/InstallableFileSystems/index.html) — FilterConnectCommunicationPort, FilterGetMessage, FilterReplyMessage confirmed [CITED: STACK.md]

---

## Metadata

**Confidence breakdown:**

| Area | Level | Reason |
|------|-------|--------|
| Track A extension points (C skeleton) | HIGH | Direct file read of nono-fltmgr.c; exact function names and structure confirmed |
| Track A IPC struct sizing | MEDIUM | Field layout derived from WDK types and WDK conventions; marked ASSUMED for static-assert value; trivially verified on VM build |
| Track A test-signing pipeline | HIGH | Phase 63 scripts proven functional in prior phase; reuse verbatim |
| Track B cherry-pick call-sites | HIGH | Direct source read of macos.rs + sandbox_prepare.rs; 63-DIVERGENCE-LEDGER.md diff-inspect notes confirmed |
| Track B ordering test update | HIGH | Existing test identified; wrong assertion confirmed by direct read |
| macOS cross-target target availability | HIGH | `rustup target list --installed` run 2026-06-08 |

**Research date:** 2026-06-08
**Valid until:** 2026-07-08 (stable WDK/FltMgr APIs; macOS cherry-pick state is current)
