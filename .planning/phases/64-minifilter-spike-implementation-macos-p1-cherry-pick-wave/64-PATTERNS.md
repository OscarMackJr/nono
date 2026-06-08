# Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave - Pattern Map

**Mapped:** 2026-06-08
**Files analyzed:** 8 new/modified files
**Analogs found:** 7 / 8 (1 no-analog: `drivers/nono-fltmgr/nono-fltmgr.c` extension — analog is the Phase 63 skeleton itself)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-fltmgr-client/Cargo.toml` | config | — | `crates/nono-shell-broker/Cargo.toml` | exact (small Windows-only crate, `windows-sys` dep, workspace member) |
| `crates/nono-fltmgr-client/src/lib.rs` | service | request-response | `crates/nono/src/sandbox/windows.rs` + `crates/nono-shell-broker/src/main.rs` | role-match (`#[cfg(windows)]` FFI loop, `windows_sys` imports, `SAFETY:` comments) |
| `Cargo.toml` (root, modify) | config | — | `Cargo.toml` itself | exact (add member to `[workspace] members`) |
| `drivers/nono-fltmgr/nono-fltmgr.c` (extend) | service | request-response | `drivers/nono-fltmgr/nono-fltmgr.c` Phase 63 skeleton | exact (extend in-place per RESEARCH extension-points map) |
| `drivers/nono-fltmgr/nono-fltmgr.inf` (modify) | config | — | `drivers/nono-fltmgr/nono-fltmgr.inf` Phase 63 file | exact (single value change: altitude placeholder) |
| `crates/nono/src/sandbox/macos.rs` (modify) | service | transform | `crates/nono/src/sandbox/macos.rs` (existing tests in same file) | exact |
| `crates/nono-cli/src/sandbox_prepare.rs` (modify) | service | transform | `crates/nono-cli/src/sandbox_prepare.rs` (CWD block lines 455–481) | exact |
| `drivers/README.md` (new) | config | — | no close analog (first drivers/ documentation file) | none |

---

## Pattern Assignments

### `crates/nono-fltmgr-client/Cargo.toml` (config — new Cargo workspace member)

**Analog:** `crates/nono-shell-broker/Cargo.toml`

**Workspace-package inheritance pattern** (lines 1–14 of nono-shell-broker/Cargo.toml):
```toml
[package]
name = "nono-shell-broker"
version = "0.62.2"
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Medium-IL broker for spawning Low-IL nono shell children on Windows"
keywords = ["sandbox", "security", "windows", "broker"]
categories = ["os::windows-apis"]
```

**Windows-only `windows-sys` dep pattern** (lines 24–31 of nono-shell-broker/Cargo.toml):
```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = [
    "Win32_Foundation",
    "Win32_Security",
    "Win32_System_Threading",
    "Win32_System_Console",
    "Win32_System_SystemServices",
] }
```

**For the spike crate, adapt to:**
```toml
[package]
name = "nono-fltmgr-client"
version = "0.62.2"
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Spike: user-mode policy client for nono minifilter IPC (Windows only)"
publish = false

[lib]
name = "nono_fltmgr_client"
path = "src/lib.rs"

[[bin]]
name = "nono_fltmgr_client"
path = "src/main.rs"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = [
    "Win32_Foundation",
    "Win32_Storage_InstallableFileSystems",
] }

[lints]
workspace = true
```

Key points:
- `publish = false` (spike crate, not for crates.io)
- `[lints] workspace = true` — required by all workspace members (enforces `clippy::unwrap_used`)
- `Win32_Storage_InstallableFileSystems` is the new feature (not in any existing crate)
- `[[bin]] nono_fltmgr_client` (→ `src/main.rs`) is required for the Plan 04 VM round-trip proof — a thin CLI wrapper that takes the deny-target path as `argv[1]` and calls `run_policy_client` (no `.unwrap()`/`.expect()`, `#[cfg(windows)]`)

---

### `crates/nono-fltmgr-client/src/lib.rs` (service, request-response — new file)

**Analog A:** `crates/nono/src/sandbox/windows.rs` (lines 1–40) — `#[cfg(windows)]` module header + `windows_sys` import style + `SAFETY:` comment discipline

**Analog B:** `crates/nono-shell-broker/src/main.rs` (lines 24–55) — `#[cfg(not(windows))]` stub + `#[cfg(windows)]` module + `windows_sys` import block + unsafe FFI loop pattern

**Module-level cfg guard pattern** (from nono-shell-broker/src/main.rs lines 24–32):
```rust
#[cfg(not(windows))]
fn main() {
    eprintln!(
        "nono-shell-broker is a Windows-only binary; \
         this build target should not ship it. \
         Phase 31 D-05: cross-compile parity stub."
    );
    std::process::exit(1);
}

#[cfg(windows)]
mod broker {
    // ... all Windows-specific code here
```

**For the spike lib, the top-level pattern is:**
```rust
//! `nono-fltmgr-client` — user-mode policy client for the nono minifilter spike.
//!
//! ALL code is `#[cfg(windows)]`. Compiles to an empty crate on Linux/macOS.
//! Phase 64 DRV-02 spike: connects to `\NonoPolicyPort`, receives `NonoIpcRequest`,
//! returns allow/deny decision.

// On non-Windows targets, the crate compiles to nothing.
#[cfg(windows)]
mod client { ... }
```

**`windows_sys` import style** (from nono/src/sandbox/windows.rs lines 9–39):
```rust
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::Security::...;
```

**`SAFETY:` comment pattern** (from nono-shell-broker and windows.rs):
```rust
// SAFETY: FilterConnectCommunicationPort takes a null-terminated wide string port name.
// `port_name` is a `Vec<u16>` terminated with `0`; no other thread modifies it.
let port = unsafe {
    FilterConnectCommunicationPort(port_name.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null_mut())
};
```

**`#[repr(C)]` struct pattern with compile-time layout assertion** (from bindings/c/src/types.rs lines 43–54 and RESEARCH.md Pattern 3):
```rust
// bindings/c/src/types.rs uses #[repr(C)] for all FFI types:
#[repr(C)]
pub struct NonoQueryResult {
    pub status: NonoQueryStatus,
    pub reason: NonoQueryReason,
    // ...
}

// For the IPC struct, use #[repr(C, packed(1))] matching the C-side #pragma pack(push,1):
#[cfg(windows)]
#[repr(C, packed(1))]
pub struct NonoIpcRequest {
    pub header: windows_sys::Win32::Storage::InstallableFileSystems::FILTER_MESSAGE_HEADER,
    pub path_buffer: [u16; 260],
    pub process_id: u32,
    pub desired_access: u32,
    pub reserved: u32,
}

// Compile-time layout assertion (preferred over unit test per RESEARCH.md "Don't Hand-Roll"):
#[cfg(windows)]
const _: () = assert!(
    std::mem::size_of::<NonoIpcRequest>()
        - std::mem::size_of::<windows_sys::Win32::Storage::InstallableFileSystems::FILTER_MESSAGE_HEADER>()
        == 532,
    "NonoIpcRequest payload size mismatch with C-side NONO_IPC_REQUEST"
);
```

**Error propagation pattern** — use `Result<(), Box<dyn std::error::Error>>` for the spike (no `NonoError` dependency; spike crate intentionally does not depend on `nono`):
```rust
#[cfg(windows)]
pub fn run_policy_client(deny_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // ...
    if port == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return Err("FilterConnectCommunicationPort failed".into());
    }
    // ...
}
```

**No `.unwrap()` / `.expect()` policy** (CLAUDE.md Unwrap Policy, enforced by `[lints] workspace = true`): use `?` propagation or explicit `if result != 0 { return Err(...) }` for FFI calls that return HRESULT/BOOL.

---

### `Cargo.toml` root (config — modify `[workspace] members`)

**Analog:** `Cargo.toml` itself (lines 1–10)

**Current members block** (lines 1–10 of Cargo.toml):
```toml
[workspace]
resolver = "2"
members = [
    "crates/nono",
    "crates/nono-cli",
    "crates/nono-proxy",
    "crates/nono-shell-broker",
    "bindings/c",
    "tools/sign-fixture",
]
```

**Modified members block** — add `"crates/nono-fltmgr-client"` in alphabetical position among `crates/` members:
```toml
[workspace]
resolver = "2"
members = [
    "crates/nono",
    "crates/nono-cli",
    "crates/nono-fltmgr-client",
    "crates/nono-proxy",
    "crates/nono-shell-broker",
    "bindings/c",
    "tools/sign-fixture",
]
```

No other changes to root `Cargo.toml`.

---

### `drivers/nono-fltmgr/nono-fltmgr.c` (service, request-response — extend Phase 63 skeleton)

**Analog:** `drivers/nono-fltmgr/nono-fltmgr.c` Phase 63 skeleton (the file itself — extend in-place)

**Phase 63 skeleton structure to preserve and extend** (all 80 lines of nono-fltmgr.c):
- Keep: `#include <fltKernel.h>`, `gFilterHandle` global, `NonoFltUnload` body, `FilterRegistration` struct, `DriverEntry` body
- Extend: `Callbacks[]` array — replace `{ IRP_MJ_OPERATION_END }` sentinel with pre-create entry + sentinel
- Extend: `DriverEntry` — add `FltCreateCommunicationPort` + worker thread start AFTER `FltStartFiltering`
- Extend: `NonoFltUnload` — add `FltCloseCommunicationPort(gServerPort)` + worker stop BEFORE `FltUnregisterFilter`
- Add: `FilterRegistration.InstanceTeardownStartCallback` entry for port cleanup
- Add: new globals `gServerPort`, `gClientPort`, ring-buffer + worker-thread state
- Add: `nono-fltmgr.h` (new header file) for the shared `NONO_IPC_REQUEST` / `NONO_IPC_REPLY` structs

**Extension point 1 — Callbacks[] array** (from RESEARCH.md Pattern 1):
```c
// Replace the Phase 63 sentinel-only array:
// BEFORE:
CONST FLT_OPERATION_REGISTRATION Callbacks[] = {
    { IRP_MJ_OPERATION_END }
};
// AFTER:
CONST FLT_OPERATION_REGISTRATION Callbacks[] = {
    { IRP_MJ_CREATE,
      0,
      NonoPreCreate,
      NULL },
    { IRP_MJ_OPERATION_END }
};
```

**Extension point 2 — DriverEntry** (from RESEARCH.md Pattern 2 + Phase 63 skeleton lines 65–79):
```c
// After FltStartFiltering succeeds, add:
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
// Also start worker thread here (PsCreateSystemThread → NonoWorkerThread)
```

**Extension point 3 — NonoFltUnload** (from Phase 63 skeleton lines 22–31):
```c
// BEFORE:
NTSTATUS NonoFltUnload(_In_ FLT_FILTER_UNLOAD_FLAGS Flags) {
    UNREFERENCED_PARAMETER(Flags);
    if (gFilterHandle != NULL) {
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
    }
    return STATUS_SUCCESS;
}
// AFTER — add port close + worker stop BEFORE FltUnregisterFilter:
    if (gServerPort != NULL) {
        FltCloseCommunicationPort(gServerPort);
        gServerPort = NULL;
    }
    // Signal worker thread to stop + KeWaitForSingleObject(gWorkerThread, ...)
    // Then FltUnregisterFilter as before
```

**BSOD-avoidance rules** (from RESEARCH.md Anti-Patterns — these are non-negotiable for the planner):
- NO `ZwCreateFile`/`NtCreateFile` anywhere in driver code (Pitfall 2 recursion BSOD)
- NULL timeout for `FltSendMessage` is forbidden (Pitfall 3 hang); always use `-5000000LL`
- All callback-reachable allocations use `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, ...)` (Pitfall 1 IRQL BSOD)
- `NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)` at top of `NonoPreCreate`
- Return `FLT_PREOP_PENDING` (not `FLT_PREOP_COMPLETE`) to defer policy decision to worker thread

**IPC struct — nono-fltmgr.h** (from RESEARCH.md Pattern 3):
```c
#pragma pack(push, 1)
typedef struct _NONO_IPC_REQUEST {
    WCHAR PathBuffer[260];   // MAX_PATH WCHARs = 520 bytes
    ULONG ProcessId;         // 4 bytes
    ACCESS_MASK DesiredAccess; // 4 bytes
    ULONG Reserved;          // 4 bytes (spike padding)
} NONO_IPC_REQUEST, *PNONO_IPC_REQUEST;
#pragma pack(pop)
// C11 _Static_assert (VS 2019+ / WDK); fallback: C_ASSERT(sizeof(...) == N) WDK macro
_Static_assert(sizeof(NONO_IPC_REQUEST) == 532, "NONO_IPC_REQUEST layout changed");

typedef struct _NONO_IPC_REPLY {
    ULONG Decision;  // 0 = allow, 1 = deny
} NONO_IPC_REPLY, *PNONO_IPC_REPLY;
```

---

### `drivers/nono-fltmgr/nono-fltmgr.inf` (config — modify altitude placeholder)

**Analog:** `drivers/nono-fltmgr/nono-fltmgr.inf` itself

**Current value** (line 71):
```ini
Instance1.Altitude  = "370020"
```

**Modified value** — executor picks a non-colliding number in 360000–389999 after `fltmc filters` enumeration on the fresh VM (D-08). Planner writes placeholder `TBD_ALTITUDE` and notes the executor must substitute it:
```ini
Instance1.Altitude  = "TBD_ALTITUDE"
```

No other INF content changes. All other fields (ServiceType = 2, StartType = 3, LoadOrderGroup, etc.) are correct as-is.

---

### `crates/nono/src/sandbox/macos.rs` (service, transform — cherry-pick `8f84d454` + D-11 tests)

**Analog:** `crates/nono/src/sandbox/macos.rs` (the file itself — test module and generate_profile function)

**Current ordering in `generate_profile`** (lines 667–676 — PRE-FIX, will be CHANGED by cherry-pick):
```rust
// SECURITY: Platform deny rules are placed BETWEEN read and write rules.
// ...
for rule in caps.platform_rules() {
    profile.push_str(rule);
    profile.push('\n');
}
```

**Target ordering after cherry-pick `8f84d454`** — move `platform_rules()` loop to AFTER the write-allows loop (lines 702–717). The comment must be updated to reflect the corrected ordering rationale (deny AFTER write allows so deny wins in last-match-wins semantics).

**Existing test that MUST be updated** (lines 997–1030 — currently asserts WRONG ordering):
```rust
#[test]
fn test_generate_profile_platform_rules_between_reads_and_writes() {
    // ... (setup identical, only assertion changes)
    // BEFORE (wrong — pre-fix): read_pos < deny_pos < write_pos
    assert!(read_pos < deny_pos, ...);
    assert!(deny_pos < write_pos, ...);
    // AFTER (correct — post-fix): read_pos < write_pos < deny_pos
    assert!(read_pos < write_pos, "read rules must come before write rules");
    assert!(write_pos < deny_pos, "platform deny rules must come AFTER write rules (last-match-wins)");
}
```

**Pattern for new D-11 ordering test** (copy structure from `test_generate_profile_gpu_rules_between_reads_and_writes` at lines 1842–1876 — closest analog in the same file):
```rust
#[test]
fn test_platform_rules_after_write_allows() {
    let mut caps = CapabilitySet::new();
    caps.add_fs(FsCapability {
        original: PathBuf::from("/test"),
        resolved: PathBuf::from("/test"),
        access: AccessMode::ReadWrite,
        is_file: false,
        source: CapabilitySource::User,
    });
    caps.add_platform_rule("(deny file-write-unlink)").unwrap();

    let profile = generate_profile(&caps).unwrap();

    let read_pos = profile
        .find("(allow file-read* (subpath \"/test\"))")
        .expect("read rule not found");
    let write_pos = profile
        .find("(allow file-write* (subpath \"/test\"))")
        .expect("write rule not found");
    let deny_pos = profile
        .find("(deny file-write-unlink)")
        .expect("deny rule not found");

    assert!(read_pos < write_pos, "read rules must come before write rules");
    assert!(
        write_pos < deny_pos,
        "platform deny rules must come AFTER write allows (last-match-wins: deny overrides write)"
    );
}
```

**Pattern for new D-11 symlink+canonical path coverage test** (copy `assert!(profile.contains(...))` style from `test_generate_profile_denies_keychain_mach_by_default` at lines 1147–1160):
```rust
#[test]
fn test_platform_deny_covers_symlink_and_canonical_path() {
    let mut caps = CapabilitySet::new();
    // Add BOTH the symlink path and the canonical path as platform rules
    caps.add_platform_rule("(deny file-read* (literal \"/etc/passwd\"))").unwrap();
    caps.add_platform_rule("(deny file-read* (literal \"/private/etc/passwd\"))").unwrap();

    let profile = generate_profile(&caps).unwrap();

    assert!(
        profile.contains("(deny file-read* (literal \"/etc/passwd\"))"),
        "symlink path /etc/passwd must appear in profile"
    );
    assert!(
        profile.contains("(deny file-read* (literal \"/private/etc/passwd\"))"),
        "canonical path /private/etc/passwd must appear in profile"
    );
}
```

**Test module header** (lines 852–857 — copy verbatim for new tests):
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::capability::{CapabilitySource, FsCapability};
    use std::path::PathBuf;
```

---

### `crates/nono-cli/src/sandbox_prepare.rs` (service, transform — cherry-pick `8f1b0b74` + `362ada22`)

**Analog:** `crates/nono-cli/src/sandbox_prepare.rs` itself (CWD block lines 455–481)

**Current CWD block** (lines 455–481 — PRE-FIX, will be CHANGED by cherry-pick):
```rust
if let Some(access) = cwd_access {
    let cwd_canonical =
        workdir
            .canonicalize()
            .map_err(|e| NonoError::PathCanonicalization {
                path: workdir.clone(),
                source: e,
            })?;

    if !caps.path_covered_with_access(&cwd_canonical, access) {
        if args.allow_cwd {
            info!("Auto-including CWD with {} access (--allow-cwd)", access);
            let cap = FsCapability::new_dir(cwd_canonical.clone(), access)?;
            caps.add_fs(cap);
        } else if silent {
            return Err(NonoError::CwdPromptRequired);
        } else {
            let confirmed = output::prompt_cwd_sharing(&cwd_canonical, &access)?;
            if confirmed {
                let cap = FsCapability::new_dir(cwd_canonical.clone(), access)?;
                caps.add_fs(cap);
            } else {
                info!("User declined CWD sharing. Continuing without automatic CWD access.");
            }
        }
        caps.deduplicate();
    }
}
```

**What `8f1b0b74` adds** — extract a `resolved_workdir` function and add a `#[cfg(target_os = "macos")]` block that emits a second `FsCapability::new_dir(workdir, access)` when `workdir != cwd_canonical`. The fork's exact call-site matches (RESEARCH.md per-commit detail, HIGH confidence).

**What `362ada22` modifies** — the `resolved_workdir` helper (introduced by `8f1b0b74`) gains `$PWD` preference: try `std::env::var("PWD").ok().map(PathBuf::from)` before `current_dir()`.

**`#[cfg(target_os = "macos")]` block pattern** (from macos.rs — use cfg-gated platform block for the symlink second-grant):
```rust
#[cfg(target_os = "macos")]
{
    // When workdir is a symlink (e.g. /tmp -> /private/tmp), the canonical path
    // differs. Emit both so Seatbelt allows traversal via the symlink path.
    if workdir != cwd_canonical {
        let symlink_cap = FsCapability::new_dir(workdir.clone(), access)?;
        caps.add_fs(symlink_cap);
    }
}
```

**`std::env::var("PWD")` pattern** (from RESEARCH.md `362ada22` detail):
```rust
fn resolved_workdir(args: &SandboxArgs) -> PathBuf {
    args.workdir
        .clone()
        .or_else(|| {
            // $PWD preserves the symlink path; current_dir() resolves it.
            // Prefer $PWD on macOS so the CWD capability covers the symlink form.
            std::env::var("PWD").ok().map(PathBuf::from)
        })
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}
```

Note: The `unwrap_or_else(|| PathBuf::from("."))` final fallback is the existing pattern (line 237 of sandbox_prepare.rs) and is acceptable here because the fallback is a non-security-critical default value, not a security config load failure.

---

### `drivers/README.md` (new documentation file)

**No analog** — this is the first documentation file under `drivers/`. The planner should use the canonical reference `drivers/nono-fltmgr/DESIGN.md` as the structural model for section headings and the Phase 63 Azure PowerShell scripts (listed in CONTEXT.md `canonical_refs`) as the source of the exact command sequences for the pipeline documentation.

Content scope per D-09:
1. C driver build + test-sign + load pipeline (exact commands, VM prerequisites)
2. Rust `nono-fltmgr-client` build/run pipeline (exact commands)
3. Note that `nono-wfp-driver.sys` placeholder and MSI are untouched

---

## Shared Patterns

### `#[cfg(windows)]` Guard Discipline
**Source:** `crates/nono-shell-broker/src/main.rs` lines 24–32, `crates/nono/src/sandbox/windows.rs` lines 1–8
**Apply to:** `crates/nono-fltmgr-client/src/lib.rs`

All platform-specific code lives inside `#[cfg(windows)]` blocks or modules. A `#[cfg(not(windows))]` stub at the crate root ensures the crate compiles to an empty, warning-free artifact on Linux/macOS CI.

### `windows_sys` Import Organization
**Source:** `crates/nono/src/sandbox/windows.rs` lines 18–39, `crates/nono-shell-broker/src/main.rs` lines 36–55
**Apply to:** `crates/nono-fltmgr-client/src/lib.rs`

Imports are grouped by `windows_sys::Win32::` module. Use explicit item imports (not glob `use windows_sys::*`). Feature flags in `Cargo.toml` must include every module referenced in imports.

```rust
// Pattern: grouped by Win32 sub-module
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::InstallableFileSystems::{
    FilterConnectCommunicationPort, FilterGetMessage, FilterReplyMessage,
    FILTER_MESSAGE_HEADER, FILTER_REPLY_HEADER,
};
```

### `SAFETY:` Comment Convention
**Source:** `crates/nono/src/sandbox/windows.rs` (multiple `unsafe` blocks), `crates/nono-shell-broker/src/main.rs`
**Apply to:** `crates/nono-fltmgr-client/src/lib.rs` (every `unsafe` block)

Every `unsafe` block must be preceded by a `// SAFETY:` comment explaining the preconditions that make the call safe. This is enforced by CLAUDE.md ("Unsafe Code: Restrict to FFI; must be wrapped in safe APIs with `// SAFETY:` docs").

### Workspace `[lints]` Inheritance
**Source:** All existing crate `Cargo.toml` files (final section)
**Apply to:** `crates/nono-fltmgr-client/Cargo.toml`

```toml
[lints]
workspace = true
```

This single line inherits `clippy::unwrap_used = "deny"` from the workspace manifest. All new crates must include it.

### Compile-Time Size Assertions (prefer over runtime)
**Source:** RESEARCH.md "Don't Hand-Roll" section
**Apply to:** `crates/nono-fltmgr-client/src/lib.rs` (`NonoIpcRequest` layout assertion)

Use `const _: () = assert!(std::mem::size_of::<T>() == N, "msg")` (fails at compile time). Do NOT use `#[test] fn check_size() { assert_eq!(...) }` for layout assertions on FFI-crossing structs.

### Seatbelt Test Module Pattern
**Source:** `crates/nono/src/sandbox/macos.rs` lines 852–857
**Apply to:** New tests in `macos.rs` (D-11 ordering tests)

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::capability::{CapabilitySource, FsCapability};
    use std::path::PathBuf;

    #[test]
    fn test_name() {
        // ... setup, generate_profile(&caps).unwrap(), assertions
    }
}
```

`#[allow(clippy::unwrap_used)]` is the approved exception for test modules (CLAUDE.md Unwrap Policy).

### Ordering Test Assertion Style
**Source:** `crates/nono/src/sandbox/macos.rs` lines 1842–1876 (`test_generate_profile_gpu_rules_between_reads_and_writes`)
**Apply to:** New D-11 ordering tests in `macos.rs`

```rust
let thing_pos = profile
    .find("(rule string)")
    .expect("rule not found");
assert!(
    pos_a < pos_b,
    "human-readable ordering invariant message"
);
```

Use `.find()` + `.expect()` (not `.unwrap()`) to get a meaningful failure message when a rule is missing. The assertion message must state the ordering invariant in plain English for diagnosability.

### DCO Sign-off
**Source:** CLAUDE.md "Commits: All commits must include a DCO sign-off line"
**Apply to:** All commits for Phase 64

```
Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
```

For cherry-picks, also include the verbatim `Upstream-commit:` trailer per CONTEXT.md D-10.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `drivers/README.md` | config | — | First documentation file under `drivers/`; no Markdown docs exist there. Use `drivers/nono-fltmgr/DESIGN.md` as structural inspiration and Phase 63 Azure scripts as the command source. |

---

## Metadata

**Analog search scope:** `crates/nono/src/sandbox/`, `crates/nono-cli/src/`, `crates/nono-shell-broker/src/`, `crates/*/Cargo.toml`, `bindings/c/src/types.rs`, `drivers/nono-fltmgr/`, root `Cargo.toml`
**Files scanned:** 14 source files read directly; workspace glob used for Cargo.toml inventory
**Pattern extraction date:** 2026-06-08
