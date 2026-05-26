# Phase 51: No-PTY Low-IL broker + token routing + write-deny preservation — Pattern Map

**Mapped:** 2026-05-26
**Files analyzed:** 6 new/modified files
**Analogs found:** 6 / 6

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | service (Windows token cascade + spawn wiring) | request-response | Self — existing `BrokerLaunch` arm in the same file | exact |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | config/struct | transform | Self — existing `ExecConfig` struct fields (`session_sid`, `session_token`) | exact |
| `crates/nono-cli/src/profile/mod.rs` | config/model | transform | Self — existing `unsafe_macos_seatbelt_rules` field + `merge_profiles` | exact |
| `crates/nono-cli/data/policy.json` | config | transform | Self — `claude-code` profile, `"interactive": true` entry | exact |
| `crates/nono-cli/data/nono-profile.schema.json` | config | transform | Self — `"unsafe_macos_seatbelt_rules"` schema entry | exact |
| `crates/nono-shell-broker/src/main.rs` | service (broker binary) | request-response | Self — existing `parse_args` / `run` / `parse_args_tests` patterns | exact |

---

## Pattern Assignments

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` — new `BrokerLaunchNoPty` variant + cascade arm + spawn wiring + integration test

**Analog:** Same file — `WindowsTokenArm::BrokerLaunch` arm, `select_windows_token_arm`, `DetachedStdioPipes`, `broker_dispatch_tests`.

---

#### Pattern A: `WindowsTokenArm` enum — add `BrokerLaunchNoPty` variant

**Source:** `launch.rs:1057-1101` (enum definition and doc block)

```rust
// Existing enum (lines 1073-1101):
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsTokenArm {
    Null,
    WriteRestricted,
    LowIlPrimary,
    BrokerLaunch,
    // NEW variant lands here (D-06):
    // BrokerLaunchNoPty,
}
```

New variant to add immediately after `BrokerLaunch` (line ~1100):

```rust
/// Phase 51 D-06: non-PTY broker path. Token construction is identical to
/// `BrokerLaunch` (null h_token; broker self-degrades to Low IL). Distinct
/// variant so the downstream spawn wiring uses anonymous-pipe stdio instead
/// of ConPTY pipes, and so PTY-path tests keep asserting `BrokerLaunch`
/// (structurally proving Phase 31 PTY path is untouched).
BrokerLaunchNoPty,
```

---

#### Pattern B: `select_windows_token_arm` — add `prefers_low_il_broker` parameter + new branch

**Source:** `launch.rs:1106-1137` (current pure function signature and body)

Current signature (line 1106):
```rust
pub(super) fn select_windows_token_arm(
    is_detached: bool,
    has_pty: bool,
    has_session_sid: bool,
    caps_demand_low_il: bool,
) -> WindowsTokenArm {
    if is_detached {
        WindowsTokenArm::Null
    } else if has_pty {
        WindowsTokenArm::BrokerLaunch
    } else if has_session_sid {
        WindowsTokenArm::WriteRestricted
    } else if caps_demand_low_il {
        WindowsTokenArm::LowIlPrimary
    } else {
        WindowsTokenArm::Null
    }
}
```

New signature and branch (insert between `has_pty` and `has_session_sid` arms):

```rust
pub(super) fn select_windows_token_arm(
    is_detached: bool,
    has_pty: bool,
    has_session_sid: bool,
    caps_demand_low_il: bool,
    prefers_low_il_broker: bool,  // NEW — Phase 51 D-02
) -> WindowsTokenArm {
    if is_detached {
        WindowsTokenArm::Null
    } else if has_pty {
        WindowsTokenArm::BrokerLaunch
    } else if prefers_low_il_broker && has_session_sid {
        // Phase 51: non-PTY supervised path with profile opt-in routes through
        // broker Low-IL arm instead of WriteRestricted.
        // WriteRestricted remains reachable when prefers_low_il_broker=false (REQ-WSRH-02).
        WindowsTokenArm::BrokerLaunchNoPty
    } else if has_session_sid {
        WindowsTokenArm::WriteRestricted
    } else if caps_demand_low_il {
        WindowsTokenArm::LowIlPrimary
    } else {
        WindowsTokenArm::Null
    }
}
```

**Call site update** (`launch.rs:1180-1185`):
```rust
let arm = select_windows_token_arm(
    is_windows_detached_launch,
    pty.is_some(),
    config.session_sid.is_some(),
    should_use_low_integrity_windows_launch(config.caps),
    config.prefers_low_il_broker,    // NEW — Phase 51 D-02
);
```

**Token match arm** — add alongside `BrokerLaunch` arm (`launch.rs:1214-1227`):
```rust
WindowsTokenArm::BrokerLaunchNoPty => {
    // Phase 51 D-06: identical token selection to BrokerLaunch — null h_token;
    // broker self-degrades to Low IL. The variant only signals the downstream
    // spawn wiring (anonymous-pipe stdio, no ConPTY).
    _restricted_holder = None;
    _low_integrity_holder = None;
    std::ptr::null_mut()
}
```

---

#### Pattern C: `spawn_windows_child` — `BrokerLaunchNoPty` spawn branch

**Analog:** `BrokerLaunch` arm in the `if let Some(pty_pair) = pty {` block (`launch.rs:1261-1578`). The new `BrokerLaunchNoPty` arm lives in the `else` branch (the `pty.is_none()` path, line ~1579 onward). It mirrors the `BrokerLaunch` arm exactly but substitutes `DetachedStdioPipes` for `pty_pair.input_write` / `pty_pair.output_read`.

**Step-by-step structure to copy** (annotated against BrokerLaunch source):

1. **Broker path resolution** — copy `launch.rs:1267-1281` verbatim:
   ```rust
   let nono_exe = std::env::current_exe().map_err(|e| {
       NonoError::SandboxInit(format!(
           "Failed to resolve current_exe for broker location: {e}"
       ))
   })?;
   let exe_dir = nono_exe.parent().ok_or_else(|| {
       NonoError::SandboxInit(format!(
           "Failed to resolve parent dir for broker location: {}",
           nono_exe.display()
       ))
   })?;
   let broker_path = exe_dir.join("nono-shell-broker.exe");
   if !broker_path.exists() {
       return Err(NonoError::BrokerNotFound { path: broker_path });
   }
   ```

2. **Authenticode check** — copy `launch.rs:1289-1297` verbatim (same security invariant as BrokerLaunch):
   ```rust
   if !is_dev_build_layout(&nono_exe) {
       verify_broker_authenticode(&nono_exe, &broker_path)?;
   } else {
       tracing::info!(target: "broker_authenticode", "skipping broker Authenticode verify: dev-build layout");
   }
   ```

3. **Create `DetachedStdioPipes`** (replaces ConPTY pipe handles, `launch.rs:72`):
   ```rust
   let mut pipes = DetachedStdioPipes::create()?;
   let inherit_handles: [HANDLE; 3] = [
       pipes.stdin_read,
       pipes.stdout_write,
       pipes.stderr_write,
   ];
   ```

4. **Flip child-end handles to inheritable** (mirror `launch.rs:1314-1337` pattern, adapted for 3 handles):
   ```rust
   for h in &inherit_handles {
       let ok = unsafe {
           // SAFETY: Each handle is a child-end owned by `pipes`; setting
           // HANDLE_FLAG_INHERIT allows the broker to pass them to its child
           // via PROC_THREAD_ATTRIBUTE_HANDLE_LIST.
           SetHandleInformation(*h, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT)
       };
       if ok == 0 {
           let last = unsafe { GetLastError() };
           for cleanup_h in &inherit_handles {
               unsafe { let _ = SetHandleInformation(*cleanup_h, HANDLE_FLAG_INHERIT, 0); }
           }
           return Err(NonoError::SandboxInit(format!(
               "SetHandleInformation(HANDLE_FLAG_INHERIT) failed (error={last})"
           )));
       }
   }
   ```

5. **PROC_THREAD_ATTRIBUTE_HANDLE_LIST** — copy `launch.rs:1339-1397` verbatim (substitute 3-element array for 2-element):
   ```rust
   // Probe size, allocate buffer, Initialize, Update, ... see launch.rs:1339-1397
   // key difference: inherit_handles is [HANDLE; 3] (not [HANDLE; 2])
   UpdateProcThreadAttribute(
       attr_list, 0,
       PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
       inherit_handles.as_ptr() as *mut _,
       std::mem::size_of_val(&inherit_handles[..]),
       std::ptr::null_mut(), std::ptr::null_mut(),
   )
   ```

6. **Build broker command line** — mirror `launch.rs:1399-1418`, add `--no-pty` flag:
   ```rust
   let mut broker_args: Vec<std::ffi::OsString> = Vec::new();
   broker_args.push(std::ffi::OsString::from("--shell"));
   broker_args.push(launch_program.as_os_str().to_owned());
   for a in cmd_args {
       broker_args.push(std::ffi::OsString::from("--shell-arg"));
       broker_args.push(std::ffi::OsString::from(a));
   }
   broker_args.push(std::ffi::OsString::from("--no-pty"));   // NEW — Phase 51
   for h in &inherit_handles {
       broker_args.push(std::ffi::OsString::from("--inherit-handle"));
       broker_args.push(std::ffi::OsString::from(format!("0x{:016x}", *h as usize)));
   }
   broker_args.push(std::ffi::OsString::from("--cwd"));
   broker_args.push(current_dir.as_os_str().to_owned());
   let mut broker_command_line = build_broker_command_line(&broker_path, &broker_args);
   ```

7. **`CreateProcessW`** — copy `launch.rs:1421-1470` verbatim (same CREATE_SUSPENDED + EXTENDED_STARTUPINFO_PRESENT + bInheritHandles=1 shape).

8. **`DeleteProcThreadAttributeList` + unflip handles + `close_child_ends`**:
   ```rust
   unsafe { DeleteProcThreadAttributeList(attr_list); }
   for h in &inherit_handles {
       unsafe { let _ = SetHandleInformation(*h, HANDLE_FLAG_INHERIT, 0); }
   }
   unsafe { pipes.close_child_ends(); }
   // Return pipes alongside child so supervisor relay can forward stdout/stderr:
   detached_stdio = Some(pipes);
   ```

**Critical difference from detached path** (`launch.rs:1585`): The existing detached path gates `DetachedStdioPipes::create()` on `is_windows_detached_launch`. The new `BrokerLaunchNoPty` arm must NOT gate on `is_windows_detached_launch` — it creates pipes unconditionally on the non-detached supervised path.

---

#### Pattern D: `pty_token_gate_tests` — new unit test + update existing tests

**Source:** `launch.rs:1885-1977` (all 7 existing `pty_token_gate_tests` tests)

All 7 existing tests call `select_windows_token_arm` with 4 arguments. After the signature change they must each gain a 5th argument `/* prefers_low_il_broker */ false`. Example existing test (line 1934):

```rust
#[test]
fn pty_none_with_session_sid_selects_write_restricted() {
    let arm = select_windows_token_arm(
        /* is_detached */ false, /* has_pty */ false,
        /* has_session_sid */ true, /* caps_demand_low_il */ false,
        /* prefers_low_il_broker */ false,  // ADD THIS to all 7 existing tests
    );
    assert_eq!(arm, WindowsTokenArm::WriteRestricted);
}
```

New test to add (REQ-WSRH-02 positive case):

```rust
#[test]
fn pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty() {
    let arm = select_windows_token_arm(
        /* is_detached */ false, /* has_pty */ false,
        /* has_session_sid */ true, /* caps_demand_low_il */ false,
        /* prefers_low_il_broker */ true,
    );
    assert_eq!(arm, WindowsTokenArm::BrokerLaunchNoPty);
}
```

---

#### Pattern E: `write_deny_low_il_broker_no_pty_tests` — new integration test module

**Source:** `launch.rs:2344-2608` (`broker_dispatch_tests` module — exact structural precedent)

Module header (same cfg gate as `broker_dispatch_tests` and `low_integrity_primary_token_tests`):

```rust
#[cfg(all(test, target_os = "windows"))]
#[allow(clippy::unwrap_used)]
mod write_deny_low_il_broker_no_pty_tests {
    use std::path::PathBuf;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, GetExitCodeProcess, ResumeThread, TerminateProcess,
        WaitForSingleObject, CREATE_SUSPENDED, INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
    };
    // ...
}
```

**Broker artifact resolution** — copy `broker_dispatch_tests` two-candidate lookup verbatim (`launch.rs:2430-2459`) — same workspace-relative logic, same hard-fail (`panic!`) on missing artifact (D-08 no-silent-skip):

```rust
let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
let workspace_root = PathBuf::from(&manifest).join("..").join("..");
let candidate_triple = workspace_root
    .join("target").join("x86_64-pc-windows-msvc").join("release")
    .join("nono-shell-broker.exe");
let candidate_default = workspace_root
    .join("target").join("release").join("nono-shell-broker.exe");
let broker_path = if candidate_triple.exists() {
    candidate_triple
} else if candidate_default.exists() {
    candidate_default
} else {
    panic!(
        "nono-shell-broker.exe missing ... cannot be silently skipped — see Phase 41 CR-04 disposition."
    );
};
```

**Temp fixture** (D-08: `%USERPROFILE%` / `%TEMP%`, NOT drive root):
```rust
let fixture = std::env::temp_dir().join(format!("nono-test-write-deny-{}.tmp", std::process::id()));
std::fs::write(&fixture, b"sentinel").expect("create fixture file");
// No explicit label needed: %TEMP% files are Medium IL by default;
// Low-IL child write will be denied by MIC pre-DACL kernel check.
```

**Spawn + assert** (follow `broker_launch_assigns_child_to_job_object` structure, `launch.rs:2497-2608`):
```rust
// Build broker command with --no-pty + 3 pipe handles + cmd that tries to write fixture:
// "broker.exe" --shell cmd.exe --shell-arg /c --shell-arg "echo x > <fixture>" --no-pty
//   --inherit-handle <stdin_r> --inherit-handle <stdout_w> --inherit-handle <stderr_w> --cwd <cwd>
let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
// CreateProcessW(broker, CREATE_SUSPENDED, ...)
// AssignProcessToJobObject (D-04 ordering from broker_dispatch_tests)
// ResumeThread
// WaitForSingleObject(INFINITE)
// GetExitCodeProcess → assert non-zero (cmd.exe echoes "Access is denied." and exits 1)
// OR read fixture → assert not modified from "sentinel"
```

**Cleanup** — close all handles; delete fixture file (same RAII-like pattern as `broker_dispatch_tests`).

---

### `crates/nono-cli/src/exec_strategy_windows/mod.rs` — add `prefers_low_il_broker` field to `ExecConfig`

**Analog:** `ExecConfig` struct fields `session_sid` (line 136), `session_token` (lines 141-142), `cap_pipe_rendezvous_path` (lines 143-144) — all are Windows-only-meaningful fields added to the shared struct.

**Source:** `mod.rs:129-166` (`ExecConfig` struct definition)

Add immediately after `session_sid` (line 136) or at end of struct before closing brace:

```rust
/// Phase 51 D-02: when `true`, routes non-PTY supervised launches through
/// `WindowsTokenArm::BrokerLaunchNoPty` instead of `WriteRestricted`.
/// Sourced from `profile.windows_low_il_broker`. Windows-only-meaningful;
/// the field exists on all platforms but is only consumed under
/// `#[cfg(target_os = "windows")]` paths (same pattern as `session_sid`).
pub prefers_low_il_broker: bool,
```

**Construction site** (`execution_runtime.rs:372-385`, Windows-gated `ExecConfig { ... }` literal):

Add `prefers_low_il_broker: profile.windows_low_il_broker,` to the struct literal. The `profile` variable is already in scope at that point (search context at line ~379 shows `session_sid: Some(exec_strategy::generate_session_sid())`).

---

### `crates/nono-cli/src/profile/mod.rs` — add `windows_low_il_broker` field

**Analog:** `unsafe_macos_seatbelt_rules` — platform-gated opt-in field (lines 2059-2072 in `Profile`, line 2132 in `ProfileDeserialize`, line 3028-3030 in `merge_profiles`).

#### 1. `Profile` struct — add field (after `unsafe_macos_seatbelt_rules`, line 2072):

```rust
/// Windows-only. When `true`, routes non-PTY supervised launches through the
/// Low-IL broker arm (`WindowsTokenArm::BrokerLaunchNoPty`) instead of the
/// `WRITE_RESTRICTED` arm. Preserves mandatory-label `NO_WRITE_UP` write-deny
/// while removing the restricting-SID double-gate that causes
/// `STATUS_DLL_INIT_FAILED` in heavy-runtime children (Electron, CLR, Node SEA).
/// Ignored on Linux and macOS (no-op; deserialize-only).
/// Only set in the `claude-code` built-in profile for v2.7.
#[serde(default)]
pub windows_low_il_broker: bool,
```

#### 2. `ProfileDeserialize` struct — add field (after `unsafe_macos_seatbelt_rules`, line 2132):

```rust
#[serde(default)]
windows_low_il_broker: bool,
```

**Note:** `ProfileDeserialize` has `#[serde(deny_unknown_fields)]` at line 2091. Both structs MUST be updated in the same commit or policy.json deserialization will fail with "unknown field" error.

#### 3. `impl From<ProfileDeserialize> for Profile` — add to exhaustive mapping (line ~2171):

```rust
// After allow_parent_of_protected line (line 2171):
windows_low_il_broker: raw.windows_low_il_broker,
```

#### 4. `merge_profiles` — add bool merge after `unsafe_macos_seatbelt_rules` (line 3028):

**Precedent for bool scalar fields** (line 3006): `interactive: base.interactive || child.interactive` — OR semantics for opt-in flags (either base or child enables it).

```rust
// After unsafe_macos_seatbelt_rules: dedup_append(...) (line 3030):
windows_low_il_broker: base.windows_low_il_broker || child.windows_low_il_broker,
```

---

### `crates/nono-cli/data/policy.json` — add field to `claude-code` profile

**Analog:** `"interactive": true` at line 728 in the `claude-code` profile.

Add after `"interactive": true` (line 728):

```json
"windows_low_il_broker": true
```

**Important:** This field must be added to `ProfileDeserialize` and `Profile` first (Pitfall 5). If policy.json is updated before the Rust structs, startup crashes with "unknown field: windows_low_il_broker".

---

### `crates/nono-cli/data/nono-profile.schema.json` — add schema entry

**Analog:** `"unsafe_macos_seatbelt_rules"` entry at lines 99-103:

```json
"unsafe_macos_seatbelt_rules": {
  "type": "array",
  "items": { "type": "string" },
  "description": "macOS-only. Expert escape hatch..."
}
```

New entry (add after `"unsafe_macos_seatbelt_rules"` block):

```json
"windows_low_il_broker": {
  "type": "boolean",
  "description": "Windows-only. Routes non-PTY supervised launches through the Low-IL broker arm instead of WRITE_RESTRICTED, eliminating STATUS_DLL_INIT_FAILED for heavy-runtime children (Electron, CLR, Node SEA). Ignored on Linux and macOS."
}
```

---

### `crates/nono-shell-broker/src/main.rs` — add `--no-pty` mode

**Analog:** Existing `parse_args` / `BrokerArgs` / `run` / `parse_args_tests` in same file.

#### 1. `BrokerArgs` struct — add `no_pty` field (after `cwd`, line ~58):

```rust
/// Phase 51: when `true`, the broker's child is spawned with
/// `STARTF_USESTDHANDLES` binding the three `inherit_handles` as stdio
/// instead of inheriting the broker's console. Set by `--no-pty` flag.
pub no_pty: bool,
```

#### 2. `parse_args` — add `--no-pty` match arm (in the `match flag_str.as_ref()` block, lines 74-121):

```rust
"--no-pty" => {
    no_pty = true;
}
```

And initialize `let mut no_pty: bool = false;` at the top of `parse_args` alongside the other `let mut` declarations (line ~65-68).

Wire into the returned struct:
```rust
Ok(BrokerArgs {
    shell_path,
    shell_args,
    inherit_handles,
    cwd,
    no_pty,  // NEW
})
```

**Compatibility with BROKER-CR-02/03:** The `--no-pty` flag does not affect null/invalid handle rejection (CR-02, line 103) or empty-list rejection (CR-03, line 132). Both guards remain active and are satisfied by the 3 pipe handles passed by the caller.

#### 3. `run` function — add `STARTF_USESTDHANDLES` branch (in `run`, after `AllocConsole` probe, before `CreateProcessAsUserW`):

**Source:** `main.rs:255-268` (existing `STARTUPINFOEXW` initialization).

```rust
let mut startup_info_ex: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
startup_info_ex.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
startup_info_ex.lpAttributeList = attr_list;

// Phase 51: when --no-pty, bind the three pipe handles as child stdio.
// Without STARTF_USESTDHANDLES the child writes to the broker's console
// (or null) and the supervisor relay never receives output (Pitfall 7).
if args.no_pty && args.inherit_handles.len() >= 3 {
    startup_info_ex.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup_info_ex.StartupInfo.hStdInput  = args.inherit_handles[0];
    startup_info_ex.StartupInfo.hStdOutput = args.inherit_handles[1];
    startup_info_ex.StartupInfo.hStdError  = args.inherit_handles[2];
}
```

Import needed: add `STARTF_USESTDHANDLES` to the `use windows_sys::Win32::System::Threading::{...}` block (line 46-49 of broker).

#### 4. New broker unit tests (in `parse_args_tests` module):

**Source:** Existing test shape from `parse_args_tests` module (`main.rs:353-549`). Copy the `argv` helper and `os` helper from line 358-368.

```rust
#[test]
fn parse_args_no_pty_flag_accepted() {
    let raw = argv(&[
        "--shell", r"C:\foo.exe",
        "--inherit-handle", "0x0000000000000100",
        "--inherit-handle", "0x0000000000000200",
        "--inherit-handle", "0x0000000000000300",
        "--no-pty",
        "--cwd", r"C:\",
    ]);
    let args = parse_args(&raw).expect("--no-pty should be accepted");
    assert!(args.no_pty, "--no-pty must set no_pty=true");
    assert_eq!(args.inherit_handles.len(), 3);
}

#[test]
fn parse_args_no_pty_absent_defaults_false() {
    let raw = argv(&[
        "--shell", r"C:\foo.exe",
        "--inherit-handle", "0x0000000000000100",
        "--cwd", r"C:\",
    ]);
    let args = parse_args(&raw).expect("args without --no-pty must parse");
    assert!(!args.no_pty, "no_pty must default to false when flag absent");
}
```

---

## Shared Patterns

### Error handling — `NonoError::SandboxInit(format!(...))` + `?` propagation
**Source:** `launch.rs:1333-1336`, `launch.rs:1364-1367`, `launch.rs:1394-1397`, `main.rs:212-215`
**Apply to:** All new unsafe-Win32-call error paths in `launch.rs` and `main.rs`

```rust
return Err(NonoError::SandboxInit(format!(
    "<descriptive message> (error={last})"
)));
```

### SAFETY doc requirement
**Source:** Every `unsafe {}` block in the BrokerLaunch arm (`launch.rs:1315-1332`, `1341-1346`, etc.) and broker `run()` (`main.rs:182-185`, `203-206`, etc.)
**Apply to:** All new `unsafe {}` blocks — every Win32 API call (SetHandleInformation, InitializeProcThreadAttributeList, UpdateProcThreadAttribute, CreateProcessW, DeleteProcThreadAttributeList)

Pattern:
```rust
unsafe {
    // SAFETY: <reason handles/pointers are valid + ownership invariants>
    SomeWin32Api(...)
}
```

### Handle cleanup discipline — revert inheritance flags on ALL paths
**Source:** `launch.rs:1460-1468` (T-31-17 mitigation — unmark ConPTY handles after CreateProcessW on BOTH success and failure)
**Apply to:** The 3 pipe `inherit_handles` in the new `BrokerLaunchNoPty` arm

```rust
// Unmark on success path:
for h in &inherit_handles {
    unsafe { let _ = SetHandleInformation(*h, HANDLE_FLAG_INHERIT, 0); }
}
// Also call on the early-return error path before every `return Err(...)`.
```

### `#[serde(default)]` for cross-platform-safe profile fields
**Source:** `profile/mod.rs:2071-2072` (`unsafe_macos_seatbelt_rules`) + `profile/mod.rs:2132` (same in `ProfileDeserialize`)
**Apply to:** New `windows_low_il_broker: bool` field in both `Profile` and `ProfileDeserialize`

### Test module cfg gate pattern
**Source:** `launch.rs:1979-1980` (`low_integrity_primary_token_tests`), `launch.rs:2344-2345` (`broker_dispatch_tests`)
**Apply to:** `write_deny_low_il_broker_no_pty_tests` module header

```rust
#[cfg(all(test, target_os = "windows"))]
#[allow(clippy::unwrap_used)]
mod write_deny_low_il_broker_no_pty_tests { ... }
```

Pure-logic tests (`pty_token_gate_tests`) use `#[cfg(test)]` only — they run on all platforms since `select_windows_token_arm` has no `#[cfg(windows)]` gate itself.

---

## Supervisor Relay — Open Question Verification

**Open Question 3 (RESEARCH.md):** Does `execute_supervised`'s relay logic handle `Some(detached_stdio_pipes)` on the non-detached supervised path?

**Finding from reading `mod.rs:800-811` and `supervisor.rs:424-441`:**

`execute_supervised` unconditionally calls `runtime.attach_detached_stdio(detached_stdio)` on line 811, regardless of `is_windows_detached_launch`. Then `start_streaming()` (line 813) calls `start_logging()` which reads `self.detached_stdio.as_ref().map(|s| s.stdout_read as usize)` (supervisor.rs:627-631). The source handle selection at line 654 is:

```rust
let source_handle: usize = if pty_output_read != 0 {
    pty_output_read
} else {
    stdout_read  // ← picks up detached_stdio on non-PTY path
};
```

**Conclusion: A1 is CONFIRMED VALID.** The relay machinery is NOT gated on `is_windows_detached_launch`. When `BrokerLaunchNoPty` returns `Some(pipes)`, `attach_detached_stdio` wires the pipe into the relay and `start_logging`'s bridge thread will read from `pipes.stdout_read` correctly. **No gate-relaxation task is needed.** The planner does NOT need to add a separate relay-wiring task.

---

## No Analog Found

None. All files being modified have close analogs within the same files (same-file precedents within the BrokerLaunch arm, the unsafe_macos_seatbelt_rules precedent, the broker_dispatch_tests precedent).

---

## Cross-Target Clippy Classification

| File | Platform scope | Cross-target clippy required? |
|---|---|---|
| `exec_strategy_windows/launch.rs` | Windows-only (under `exec_strategy_windows/`) | No — pure Windows-only file per CLAUDE.md scope exception |
| `exec_strategy_windows/mod.rs` | Windows-only struct (`ExecConfig` lives only under `exec_strategy_windows/`) | No — pure Windows-only directory |
| `profile/mod.rs` | Cross-platform (compiled on Linux/macOS/Windows) | **YES** — new `windows_low_il_broker: bool` field; verify no dead_code lint on Linux/macOS targets |
| `data/policy.json` | Data file (no Rust compilation) | No |
| `data/nono-profile.schema.json` | Data file (no Rust compilation) | No |
| `nono-shell-broker/src/main.rs` | Windows-only binary (non-Windows stub at line 24-32) | No — non-Windows target just hits the `fn main()` stub |

**Cross-target clippy commands** (REQ-WSRH-05):
```bash
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```
These are required before phase close. If cross-toolchains are unavailable, mark REQ-WSRH-05 PARTIAL per `.planning/templates/cross-target-verify-checklist.md`.

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/exec_strategy_windows/`, `crates/nono-cli/src/profile/`, `crates/nono-cli/data/`, `crates/nono-shell-broker/src/`, `crates/nono-cli/src/execution_runtime.rs`
**Files read:** 9 source files
**Pattern extraction date:** 2026-05-26
