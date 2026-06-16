# Phase 73: AI_AGENT Marker - Pattern Map

**Mapped:** 2026-06-14
**Files analyzed:** 6 (2 new, 4 modified)
**Analogs found:** 6 / 6

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono/src/agent.rs` (NEW) | library, identity | request-response (sync Win32 calls) | `crates/nono/src/sandbox/windows.rs` (token/SID helpers) | role-match (same Win32 token pattern) |
| `crates/nono/src/lib.rs` (MODIFY) | config/re-export | — | `crates/nono/src/lib.rs` itself (existing `#[cfg(windows)]` re-export block) | exact |
| `crates/nono-cli/src/classify_runtime.rs` (NEW) | command runtime | request-response | `crates/nono-cli/src/why_runtime.rs` | exact (same: single-arg query verb, prints to stdout, no subcommands) |
| `crates/nono-cli/src/cli.rs` (MODIFY) | config/CLI | — | `crates/nono-cli/src/cli.rs` `InspectArgs` + `Commands::Why` variant | exact |
| `crates/nono-cli/src/app_runtime.rs` (MODIFY) | routing | — | `crates/nono-cli/src/app_runtime.rs` existing `dispatch_command` | exact |
| `crates/nono-cli/src/main.rs` (MODIFY) | entry-point | — | `crates/nono-cli/src/main.rs` existing `mod` declarations | exact |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` (MODIFY) | service | CRUD | `crates/nono/src/sandbox/windows.rs` `try_set_mandatory_label` (SDDL→SD pattern) | role-match |
| `crates/nono-cli/src/execution_runtime.rs` (MODIFY) | wiring | request-response | `crates/nono-cli/src/execution_runtime.rs` itself (lines 483-488, existing mint path) | exact |

---

## Pattern Assignments

### `crates/nono/src/agent.rs` (NEW — library, identity)

**Primary Analog:** `crates/nono/src/sandbox/windows.rs`

**Imports pattern** (`sandbox/windows.rs` lines 1-40 — extract the subset for `agent.rs`):

```rust
// agent.rs imports (Windows arm) — copy this structure:
use crate::error::{NonoError, Result};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenAppContainerSid,
    TOKEN_APPCONTAINER_INFORMATION, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
// Re-use OwnedHandle from the parent sandbox module — do NOT re-define it:
use crate::sandbox::windows::OwnedHandle;
```

**Module-level cfg-gate pattern** (`sandbox/windows.rs` file as a whole — it is `#[cfg]`-gated at the module declaration level in `lib.rs`, not in the file itself):

```rust
// In crates/nono/src/lib.rs (existing, lines 82-89):
#[cfg(target_os = "windows")]
pub use sandbox::windows::{
    // ... existing exports ...
};
// Pattern: add AgentRegistry, AgentClassification to this block.
// Also add the pub mod agent; declaration to lib.rs (NOT cfg-gated at mod level —
// agent.rs must compile on all platforms; the Windows-specific functions inside it
// are cfg-gated per-function, following the create_low_integrity_primary_token pattern).
```

**OwnedHandle RAII pattern** (`sandbox/windows.rs` lines 489-509):

```rust
pub struct OwnedHandle(pub HANDLE);

impl OwnedHandle {
    #[must_use]
    pub fn raw(&self) -> HANDLE { self.0 }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                // SAFETY: This handle is owned by the wrapper and is closed
                // exactly once on drop; null was checked above.
                let _ = CloseHandle(self.0);
            }
        }
    }
}
```

**Win32 token-open + error pattern** (`sandbox/windows.rs` lines 534-580, `create_low_integrity_primary_token`):

```rust
// Pattern: every Win32 call follows this shape — call, check return,
// format error with GetLastError, wrap in OwnedHandle immediately.
let mut current_token: HANDLE = std::ptr::null_mut();
let opened = unsafe {
    // SAFETY: We pass a valid mutable out-pointer and request access on the
    // current process token only.
    OpenProcessToken(
        GetCurrentProcess(),
        TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
        &mut current_token,
    )
};
if opened == 0 {
    return Err(NonoError::SandboxInit(format!(
        "Failed to open Windows process token for low-integrity launch (GetLastError={})",
        unsafe { GetLastError() }
    )));
}
let current_token = OwnedHandle(current_token);
```

**`read_process_appcontainer_sid`: non-Windows stub pattern** (`sandbox/windows.rs` — `derive_app_container_sid` exists only in the `#[cfg(windows)]` block; stubs for non-Windows follow this pattern throughout the module):

```rust
// In agent.rs: every public Windows-only function needs BOTH arms:

#[cfg(target_os = "windows")]
pub fn read_process_appcontainer_sid(pid: u32) -> Result<Option<String>> {
    // ... real Win32 implementation (see RESEARCH.md Pattern 1) ...
}

#[cfg(not(target_os = "windows"))]
pub fn read_process_appcontainer_sid(_pid: u32) -> Result<Option<String>> {
    Err(NonoError::UnsupportedPlatform(
        "AppContainer SID classification is Windows-only".into(),
    ))
}
```

**`OwnedAppContainerSid` ownership trap** (`sandbox/windows.rs` lines 693-718 — DO NOT copy this pattern for the `TOKEN_APPCONTAINER_INFORMATION` PSID). The PSID returned inside `GetTokenInformation`'s buffer is owned by the `Vec<u8>` buffer, NOT by `FreeSid`. The `OwnedAppContainerSid` wrapper calls `FreeSid` on drop — **do not wrap the buffer-internal PSID in it**. Extract the string form (via `package_sid_to_string` logic) before dropping the buffer.

**`#[must_use]` annotation pattern** (`sandbox/windows.rs` lines 533, 746):

```rust
#[must_use = "the returned OwnedHandle owns a Win32 HANDLE and must be retained until ..."]
pub fn create_low_integrity_primary_token() -> Result<OwnedHandle> { ... }

// Apply the same to AgentRegistry::classify and read_process_appcontainer_sid:
#[must_use = "fail-secure: callers must act on the classification result"]
pub fn classify(&self, pid: u32) -> AgentClassification { ... }
```

---

### `crates/nono/src/lib.rs` (MODIFY — re-export surface)

**Analog:** `crates/nono/src/lib.rs` itself (lines 48-114)

**Module declaration pattern** (lines 48-62 — how new modules are added):

```rust
// Existing block (lines 48-62); insert `pub mod agent;` here
// (NOT cfg-gated at mod level — agent.rs compiles everywhere;
//  individual functions inside it are cfg-gated per-function):
pub mod capability;
pub mod diagnostic;
pub mod error;
// ... existing mods ...
pub mod agent;  // <-- NEW: insert here, alphabetically
```

**Windows-only re-export pattern** (lines 82-89):

```rust
// Existing block — extend it with AgentRegistry and AgentClassification:
#[cfg(target_os = "windows")]
pub use sandbox::windows::{
    apply_low_il_label_to_token, create_app_container_profile,
    create_low_integrity_primary_token,
    derive_app_container_sid, grant_sid_read_on_path,
    // ... rest of existing exports ...
};
// AgentRegistry/AgentClassification are available on all platforms
// (they just return NotAnAgent on non-Windows), so re-export WITHOUT cfg:
pub use agent::{AgentClassification, AgentRegistry};
```

---

### `crates/nono-cli/src/classify_runtime.rs` (NEW — command runtime, request-response)

**Primary Analog:** `crates/nono-cli/src/why_runtime.rs`

**File structure pattern** (`why_runtime.rs` full file — 235 lines, single `pub(crate) fn run_why(args: WhyArgs) -> Result<()>` entry point):

```rust
// classify_runtime.rs follows exactly this structure:
use crate::cli::ClassifyArgs;
use nono::{AgentClassification, AgentRegistry, NonoError, Result};
use std::sync::{Arc, Mutex};

pub(crate) fn run_classify(args: ClassifyArgs, registry: Arc<Mutex<AgentRegistry>>) -> Result<()> {
    // 1. Parse the PID from args.pid (u32)
    // 2. Lock the registry (map PoisonError → NonoError::SandboxInit)
    // 3. Call registry.classify(args.pid)
    // 4. Print result to stdout (human-readable; non-authoritative disclaimer)
    // 5. Return Ok(())
    //
    // Fail-secure: if classify returns NotAnAgent, print "not an agent" — never error out
    // (unknown PID is not a program error, it is an expected outcome).
    Ok(())
}
```

**Output formatting pattern** (`why_runtime.rs` lines 219-226 — println! to stdout, eprintln! for errors):

```rust
// In why_runtime.rs (lines 219-226):
if args.json {
    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| NonoError::ConfigParse(format!("JSON serialization failed: {}", e)))?;
    println!("{}", json);
} else {
    print_result(&result);
}

// classify_runtime.rs mirrors this structure for human output:
// println!("PID {}: AI_AGENT", pid) or println!("PID {}: not an agent", pid)
// Always emit a "(NOTE: This check is structural only — not an authorization decision)"
// disclaimer line.
```

**Error propagation pattern** (`why_runtime.rs` lines 109-116 — errors that are NOT user errors use `Err(NonoError::SandboxInit(...))`; failures that ARE expected outcomes return `Ok(())` after printing):

```rust
// why_runtime.rs lines 213-215:
} else {
    return Err(NonoError::ConfigParse(
        "PATH, --path, --host, or --scope is required".to_string(),
    ));
}
// classify_runtime.rs: OpenProcess failure on a nonexistent PID prints an error
// message and returns Err(...) so nono exits non-zero (operator typo path).
```

---

### `crates/nono-cli/src/cli.rs` (MODIFY — Commands enum + ClassifyArgs struct)

**Analog:** `cli.rs` lines 725-743 (`Why(Box<WhyArgs>)` variant + `WhyArgs` struct at lines 2564-2637); also `InspectArgs` at lines 3073-3088 (single positional String arg).

**Command variant pattern** (lines 725-743 — `Why` variant, the simplest single-arg query verb):

```rust
// Existing Why variant (lines 725-743) — copy this exact shape for Classify:
/// Check why a path or network operation would be allowed or denied
#[command(help_template = "\
{about}

\x1b[1mUSAGE\x1b[0m
  nono why [PATH] [flags]

{all-args}
{after-help}")]
#[command(after_help = "\x1b[1mEXAMPLES\x1b[0m
  nono why ~/.ssh/id_rsa                       # ...
")]
Why(Box<WhyArgs>),

// For Classify, the shape is simpler (no Box needed for small args structs;
// see InspectArgs which is NOT boxed — line 868):
/// Classify a process as an AI agent or not (best-effort, non-authoritative)
#[command(after_help = "\x1b[1mEXAMPLES\x1b[0m
  nono classify 1234                           # Classify PID 1234
  nono classify 1234 --json                    # JSON output
\x1b[1mNOTE\x1b[0m
  This check is structural only and is NOT an authorization decision.
  Only the launcher's in-memory registry is authoritative (SC2).
")]
Classify(ClassifyArgs),
```

**Args struct pattern** (`cli.rs` lines 3073-3088, `InspectArgs` — a positional String + optional flags):

```rust
// InspectArgs (lines 3073-3088) — exact template for ClassifyArgs:
#[derive(Parser, Debug)]
pub struct InspectArgs {
    /// Session ID (or prefix)
    pub session: String,   // positional, no #[arg(...)] needed

    #[arg(long)]
    pub json: bool,

    #[arg(long)]
    pub events: bool,
}

// ClassifyArgs follows this EXACT pattern:
#[derive(Parser, Debug)]
pub struct ClassifyArgs {
    /// PID to classify
    pub pid: u32,          // positional u32

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Print help
    #[arg(long, short = 'h', action = clap::ArgAction::Help)]
    pub help: Option<bool>,
}
```

---

### `crates/nono-cli/src/app_runtime.rs` (MODIFY — routing)

**Analog:** `app_runtime.rs` lines 41-148, `dispatch_command` function.

**Dispatch pattern** (lines 47-148 — every arm follows this shape):

```rust
// Existing arm (line 58):
Commands::Why(args) => run_command_with_update(update_handle, silent, || run_why(*args)),

// New arm for Classify (insert alphabetically or after Why):
Commands::Classify(args) => {
    run_command_with_update(update_handle, silent, || {
        classify_runtime::run_classify(args, registry.clone())
    })
}
// Note: `registry` is the Arc<Mutex<AgentRegistry>> created at the top of
// dispatch_command (or passed in). If the registry is threaded into dispatch_command
// as a parameter, follow the `silent` / `internal_supervisor` pattern (lines 42-46).
```

**Import pattern** (`app_runtime.rs` lines 1-22 — every runtime module is imported here):

```rust
// Existing (line 20):
use crate::why_runtime::run_why;

// New:
use crate::classify_runtime;  // or: use crate::classify_runtime::run_classify;
```

---

### `crates/nono-cli/src/main.rs` (MODIFY — module registration)

**Analog:** `main.rs` lines 1-110 (module declaration block).

**Module declaration pattern** (lines 24-110 — Windows-only modules use `#[cfg(target_os = "windows")]`):

```rust
// Existing Windows-only module (lines 24-25):
#[cfg(target_os = "windows")]
mod exec_identity_windows;

// classify_runtime compiles on all platforms (run_classify returns
// UnsupportedPlatform on non-Windows via the AgentRegistry stub),
// so it is NOT cfg-gated at the mod level:
mod classify_runtime;  // insert alphabetically with other *_runtime.rs mods
```

---

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` (MODIFY — job ACL hardening)

**Primary Analog:** `crates/nono/src/sandbox/windows.rs` `try_set_mandatory_label` (lines 1040-1079) — the `ConvertStringSecurityDescriptorToSecurityDescriptorW` + `OwnedSecurityDescriptor` pattern.

**SDDL → SecurityDescriptor pattern** (`sandbox/windows.rs` lines 1051-1080):

```rust
// try_set_mandatory_label (lines 1051-1080) — EXACT pattern to copy for job ACL:
let sddl = format!("S:(ML;;0x{mask:X};;;LW)");
let wide_sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();

let mut security_descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
let ok = unsafe {
    // SAFETY: `wide_sddl` is a valid nul-terminated UTF-16 buffer; ...
    ConvertStringSecurityDescriptorToSecurityDescriptorW(
        wide_sddl.as_ptr(),
        SDDL_REVISION_1,
        &mut security_descriptor,
        std::ptr::null_mut(),
    )
};
if ok == 0 {
    let hresult = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    return Err(NonoError::SandboxInit(format!(
        "ConvertStringSecurityDescriptorToSecurityDescriptorW for job ACL failed (GLE={})",
        hresult
    )));
}
let _sd_guard = OwnedSecurityDescriptor(security_descriptor);
// Pass `security_descriptor` into SECURITY_ATTRIBUTES.lpSecurityDescriptor.
// LocalFree is handled by _sd_guard Drop.
```

**Job creation call site** (`launch.rs` lines 189-244, `create_process_containment`):

```rust
// Current null SA at line 199 (to be replaced):
CreateJobObjectW(
    std::ptr::null(),          // <-- replace with &sa (SECURITY_ATTRIBUTES)
    name_u16.as_ref()...
)

// Target shape (D-03):
// 1. Build SDDL: "D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)"
//    optionally append "(D;;0x1F001F;;;<package_sid>)" if package_sid is Some.
// 2. ConvertStringSecurityDescriptorToSecurityDescriptorW → sd_ptr
// 3. let sa = SECURITY_ATTRIBUTES { nLength: size_of::<SA> as u32,
//                                    lpSecurityDescriptor: sd_ptr, bInheritHandle: 0 };
// 4. CreateJobObjectW(&sa, name_ptr)
// 5. LocalFree(sd_ptr) after CreateJobObjectW returns (whether success or fail)
//    — or use OwnedSecurityDescriptor guard as in try_set_mandatory_label.
```

**SDDL for job ACL** (from RESEARCH.md Pattern 3 — use hex rights, not SDDL mnemonics):

```
D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)
```
`D:P` = protected DACL (no inheritance). `(A;;0x1F001F;;;OW)` = ALLOW all job access to the Object Owner. `(D;;0x1F001F;;;LW)` = DENY all job access to the Low Integrity label. If `package_sid` is available, append `(D;;0x1F001F;;;<package_sid_sddl_string>)`.

**Signature refactor** (`launch.rs` lines 189, 818, 892 — the three call sites):

```rust
// Current signature (line 189):
pub(super) fn create_process_containment(session_id: Option<&str>) -> Result<ProcessContainment>

// New signature (Option A from RESEARCH.md):
pub(super) fn create_process_containment(
    session_id: Option<&str>,
    package_sid: Option<&str>,    // <-- thread through from ExecConfig
) -> Result<ProcessContainment>

// Call sites at lines ~818 and ~892 (inside spawn_windows_child branches)
// already have access to config.package_sid — thread it through.
```

---

### `crates/nono-cli/src/execution_runtime.rs` (MODIFY — mint→registry wiring)

**Analog:** `execution_runtime.rs` itself, lines 482-530 (the existing mint path).

**Exact wiring point** (lines 482-488 — INSERT after line 488):

```rust
// Existing code (lines 482-488):
#[cfg(target_os = "windows")]
let windows_app_container_name = exec_strategy::generate_app_container_name();
#[cfg(target_os = "windows")]
let windows_package_sid = {
    let psid = nono::derive_app_container_sid(&windows_app_container_name)?;
    nono::package_sid_to_string(&psid)?
};

// INSERT after line 488 (inside the #[cfg(target_os = "windows")] block):
#[cfg(target_os = "windows")]
{
    registry
        .lock()
        .map_err(|_| NonoError::SandboxInit("AgentRegistry mutex poisoned".into()))?
        .insert(windows_package_sid.clone());
}
```

**`map_err` on `PoisonError` pattern** (required by `clippy::unwrap_used` — no `.unwrap()` on Mutex::lock):

```rust
// Pattern used throughout nono-cli for fallible locks:
some_mutex
    .lock()
    .map_err(|_| NonoError::SandboxInit("... mutex poisoned".into()))?
```

---

## Shared Patterns

### Win32 Error Reporting
**Source:** `crates/nono/src/sandbox/windows.rs`, every Win32 call (e.g. lines 545-549, 570-575)
**Apply to:** All Win32 calls in `agent.rs` and `launch.rs` job-ACL code

```rust
// Uniform shape — no variation:
if return_value == 0 {
    return Err(NonoError::SandboxInit(format!(
        "<function_name>(<context>) failed (GetLastError={})",
        unsafe { GetLastError() }
    )));
}
```

### cfg-gate + non-Windows stub
**Source:** `crates/nono/src/sandbox/windows.rs` (entire file is inside `#[cfg(target_os = "windows")]` in `lib.rs`'s re-export block; individual fn stubs follow the pattern below)
**Apply to:** Every new public function in `agent.rs` that calls Win32 APIs

```rust
#[cfg(target_os = "windows")]
pub fn my_windows_fn(arg: T) -> Result<R> { /* real impl */ }

#[cfg(not(target_os = "windows"))]
pub fn my_windows_fn(_arg: T) -> Result<R> {
    Err(NonoError::UnsupportedPlatform(
        "This function is Windows-only".into(),
    ))
}
```

### `#[must_use]` on security-critical Results
**Source:** `crates/nono/src/sandbox/windows.rs` lines 533, 746
**Apply to:** `AgentRegistry::classify`, `read_process_appcontainer_sid`

```rust
#[must_use = "fail-secure: callers must act on the classification result"]
pub fn classify(&self, pid: u32) -> AgentClassification { ... }
```

### Fail-secure default
**Source:** CLAUDE.md § Fail Secure; `why_runtime.rs` line 104 (unknown state → explicit "not sandboxed" message, not an error propagated as panic)
**Apply to:** `AgentRegistry::classify` return value, `classify_runtime::run_classify` output

Unknown or unclassifiable PIDs MUST return/print `NotAnAgent`, never `AiAgent`. `Err` paths in `classify` MUST map to `NotAnAgent` (swallow the error and return the safe default), except for genuine programmer errors (poisoned mutex) which propagate as `Err`.

### SDDL + `OwnedSecurityDescriptor` guard
**Source:** `crates/nono/src/sandbox/windows.rs` lines 1051-1080
**Apply to:** Job ACL construction in `launch.rs`

The `LocalFree(sd)` call MUST be paired with every code path after `ConvertStringSecurityDescriptorToSecurityDescriptorW` succeeds. Use an `OwnedSecurityDescriptor` RAII guard (already defined in `sandbox/windows.rs`) or call `LocalFree` manually in both the success path and all early-exit paths.

---

## No Analog Found

All files have close analogs. No entries.

---

## Metadata

**Analog search scope:** `crates/nono/src/`, `crates/nono-cli/src/`, `crates/nono-cli/src/exec_strategy_windows/`
**Files read:** 10 source files
**Pattern extraction date:** 2026-06-14

### Key line-number references (confirmed by Read)

| Pattern | File | Lines |
|---------|------|-------|
| `OwnedHandle` RAII | `crates/nono/src/sandbox/windows.rs` | 489-509 |
| `create_low_integrity_primary_token` (Win32 token open) | `crates/nono/src/sandbox/windows.rs` | 534-581 |
| `OwnedAppContainerSid` (FreeSid on Drop) | `crates/nono/src/sandbox/windows.rs` | 693-718 |
| `derive_app_container_sid` (Win32 SID derive) | `crates/nono/src/sandbox/windows.rs` | 747-769 |
| `package_sid_to_string` (ConvertSidToStringSidW) | `crates/nono/src/sandbox/windows.rs` | 789-820 |
| `try_set_mandatory_label` (SDDL→SD pattern) | `crates/nono/src/sandbox/windows.rs` | 1040-1079 |
| `create_process_containment` (job creation, null SA) | `crates/nono-cli/src/exec_strategy_windows/launch.rs` | 189-245 |
| Mint path (`windows_package_sid` in scope) | `crates/nono-cli/src/execution_runtime.rs` | 482-530 |
| `Commands` enum (Why variant — exact template) | `crates/nono-cli/src/cli.rs` | 725-743 |
| `InspectArgs` (positional String + flags) | `crates/nono-cli/src/cli.rs` | 3073-3088 |
| `dispatch_command` (Why arm — exact template) | `crates/nono-cli/src/app_runtime.rs` | 47-58 |
| `why_runtime.rs` (full runtime file structure) | `crates/nono-cli/src/why_runtime.rs` | 1-235 |
| `#[cfg(windows)]` re-export block in lib.rs | `crates/nono/src/lib.rs` | 82-89 |
| Module declarations (cfg-gated pattern) | `crates/nono-cli/src/main.rs` | 24-110 |
