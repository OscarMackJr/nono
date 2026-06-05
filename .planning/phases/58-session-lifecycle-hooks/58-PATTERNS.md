# Phase 58: Session Lifecycle Hooks - Pattern Map

**Mapped:** 2026-06-05
**Files analyzed:** 11 (2 new source, 1 new Windows runtime, 1 new ADR, 7 modified)
**Analogs found:** 11 / 11

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/hook_runtime.rs` (NEW) | service, unix-only | event-driven | upstream `daa55c8:crates/nono-cli/src/hook_runtime.rs` | exact port |
| `crates/nono-cli/src/hook_runtime_windows.rs` (NEW) | service, windows-only | event-driven | `exec_strategy_windows/launch.rs` + `dacl_guard.rs` | structural analog |
| `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (MODIFIED) | utility | transform | self (extend existing `is_dangerous_env_var`) | self-analog |
| `crates/nono-cli/src/profile/mod.rs` (MODIFIED) | config/model | CRUD | `windows_low_il_broker` field threading (lines 2184, 2246, 2281, 3171) | exact match |
| `crates/nono-cli/src/policy.rs` (MODIFIED) | config | transform | `to_raw_profile()` at lines 155–212 | exact match |
| `crates/nono-cli/src/sandbox_prepare.rs` (MODIFIED) | service | CRUD | `allowed_env_vars` field pattern (lines 87–95, 529) | exact match |
| `crates/nono-cli/src/launch_runtime.rs` (MODIFIED) | config/model | CRUD | `ExecutionFlags` struct + `defaults()` (lines 162–234) | exact match |
| `crates/nono-cli/src/execution_runtime.rs` (MODIFIED) | service | request-response | upstream `daa55c8:execution_runtime.rs` hook wiring (lines 249–295, 417–424) | exact port |
| `crates/nono-cli/src/main.rs` (MODIFIED) | config | — | `#[cfg(not(target_os = "windows"))] mod learn;` pattern (lines 24–80) | exact match |
| `crates/nono-cli/data/nono-profile.schema.json` (MODIFIED) | config | — | upstream `daa55c8:data/nono-profile.schema.json` `SessionHooks`/`SessionHook` $defs | exact port |
| `.planning/architecture/adr-58-windows-hook-executor.md` (NEW) | ADR | — | `.planning/architecture/broker-trust-anchor.md` | structural match |

---

## Pattern Assignments

---

### `crates/nono-cli/src/hook_runtime.rs` (NEW — unix-only service, event-driven)

**Analog:** upstream commit `daa55c8`, file `crates/nono-cli/src/hook_runtime.rs` (610-line port)

**Gate:** `#[cfg(unix)]` module declaration in `main.rs` (see main.rs section below).

**Module-level doc comment pattern** (fork divergence header required by D-01/D-02):
```rust
//! Session lifecycle hook execution (Unix-only).
//!
//! # Fork divergence (D-01): fail-closed
//! Upstream commit daa55c8 is fail-open: hook errors warn and do not block
//! launch. This fork overrides that behavior: any hook failure (non-zero exit,
//! timeout, validation error) returns Err and prevents session start (before-hook)
//! or surfaces as a non-zero exit (after-hook). This invariant is recorded in
//! `.planning/architecture/adr-58-windows-hook-executor.md`.
```

**Imports pattern** (upstream `daa55c8:hook_runtime.rs` lines 14–26):
```rust
use crate::{exec_strategy, profile, session};
use nono::{NonoError, Result};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{debug, warn};
```

**Core function signatures** (upstream `daa55c8:hook_runtime.rs` lines 58–74, 119–131):
```rust
pub(crate) fn execute_before_hook(
    hook: &profile::SessionHook,
    session_id: &str,
    workdir: &Path,
) -> Result<Vec<(String, String)>> { ... }

pub(crate) fn execute_after_hook(
    hook: &profile::SessionHook,
    session_id: &str,
    workdir: &Path,
    child_exit_code: i32,
) -> Result<()> { ... }
```

**FORK DIVERGENCE: fail-closed vs upstream fail-open** (D-01 — do NOT copy the upstream warn-and-continue pattern verbatim):
```rust
// UPSTREAM (daa55c8, fail-OPEN — do NOT copy verbatim):
if output.timed_out {
    warn!("Before-hook timed out ({}s): {}", ...);
    return Ok(Vec::new());  // <-- upstream returns Ok
}
if output.exit_code != 0 {
    warn!("Before-hook exited with code {}: {}", ...);
}  // <-- upstream falls through to Ok

// FORK (fail-CLOSED per D-01/D-03):
if output.timed_out {
    return Err(NonoError::ConfigParse(format!(
        "Before-hook timed out after {}s (fail-closed): {}",
        hook.timeout_secs.unwrap_or(0),
        script_path.display()
    )));
}
if output.exit_code != 0 {
    return Err(NonoError::ConfigParse(format!(
        "Before-hook exited with code {} (fail-closed): {}",
        output.exit_code,
        script_path.display()
    )));
}
```

**EnvFileGuard pattern** (upstream `daa55c8:hook_runtime.rs` lines ~282–340 — port verbatim; Unix `mode(0o600)` + `create_new(true)` + RAII Drop with zero-then-unlink):
```rust
struct EnvFileGuard {
    path: PathBuf,
}

impl EnvFileGuard {
    fn create(session_id: &str) -> Result<Self> {
        let sessions_dir = session::ensure_sessions_dir()?;
        let session_env_dir = sessions_dir.join(session_id);
        std::fs::create_dir_all(&session_env_dir).map_err(|e| {
            NonoError::ConfigParse(format!("Failed to create session env directory {}: {e}", ...))
        })?;
        #[cfg(unix)]
        { let _ = std::fs::set_permissions(&session_env_dir, std::fs::Permissions::from_mode(0o700)); }
        let path = session_env_dir.join("env");
        #[cfg(unix)]
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| NonoError::ConfigParse(format!("Failed to create env file: {e}")))?;
        Ok(Self { path })
    }
}

impl Drop for EnvFileGuard {
    fn drop(&mut self) {
        if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(&self.path)
            && let Ok(metadata) = file.metadata()
        {
            use std::io::Write;
            let zeros = vec![0u8; metadata.len() as usize];
            let _ = file.write_all(&zeros);
            let _ = file.sync_all();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}
```

**run_hook with mpsc timeout** (upstream `daa55c8:hook_runtime.rs` lines ~368–410):
```rust
fn run_hook(cmd: &mut Command, timeout_secs: Option<u64>) -> Result<HookOutput> {
    let child = cmd.spawn().map_err(|e| {
        NonoError::CommandExecution(std::io::Error::other(format!("Failed to spawn hook: {e}")))
    })?;
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || { let _ = tx.send(child.wait_with_output()); });
    let received = match timeout_secs {
        Some(secs) => rx.recv_timeout(Duration::from_secs(secs)).map_err(|_| ()),
        None => rx.recv().map_err(|_| ()),
    };
    match received {
        Ok(Ok(output)) => Ok(HookOutput { exit_code: output.status.code().unwrap_or(-1), timed_out: false }),
        Ok(Err(e)) => Err(NonoError::CommandExecution(e)),
        Err(()) if timeout_secs.is_some() => {
            kill_process_group(pid);
            Ok(HookOutput { exit_code: -1, timed_out: true })
        }
        Err(()) => Err(NonoError::CommandExecution(std::io::Error::other("Hook channel closed unexpectedly"))),
    }
}
```

**kill_process_group** (upstream `daa55c8:hook_runtime.rs` lines ~411–421 — port verbatim):
```rust
fn kill_process_group(pid: u32) {
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;
    let pgid = Pid::from_raw(pid as i32);
    let _ = killpg(pgid, Signal::SIGTERM);
    thread::sleep(Duration::from_millis(100));
    let _ = killpg(pgid, Signal::SIGKILL);
}
```

**build_hook_command with setpgid pre_exec** (upstream `daa55c8:hook_runtime.rs` lines ~162–212):
```rust
fn build_hook_command(script: &Path, session_id: &str, workdir: &Path, kind: &HookKind<'_>) -> Command {
    let mut cmd = Command::new(script);
    cmd.env_clear();
    cmd.env("NONO_SESSION_ID", session_id);
    cmd.env("NONO_WORKDIR", workdir);
    cmd.env("NONO_HOOK_TYPE", kind.type_env());
    cmd.stdin(Stdio::null());
    cmd.stderr(Stdio::piped());
    // ... kind-specific env
    // SAFETY: setpgid(0,0) places the child in its own process group for
    // clean timeout killing. POSIX guarantees setpgid is async-signal-safe.
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            let _ = nix::unistd::setpgid(nix::unistd::Pid::from_raw(0), nix::unistd::Pid::from_raw(0));
            Ok(())
        });
    }
    cmd
}
```

**Test module: ENV_LOCK + isolated_home pattern** (upstream `daa55c8:hook_runtime.rs` lines ~415–432 — port VERBATIM per Pitfall 5):
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn isolated_home() -> (
        std::sync::MutexGuard<'static, ()>,
        crate::test_env::EnvVarGuard,
        TempDir,
    ) {
        let lock = match crate::test_env::ENV_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let home = TempDir::new().unwrap();
        let home_str = home.path().to_str().unwrap();
        let env = crate::test_env::EnvVarGuard::set_all(&[("HOME", home_str)]);
        (lock, env, home)
    }
    // ... tests follow
}
```

**Critical test renaming** (Pitfall 3 — upstream test name changed to assert fail-closed):

- Replace `test_execute_before_hook_fail_open` → `test_execute_before_hook_fail_closed`; assert `Err(_)` on exit 1.
- Replace `test_execute_before_hook_timeout` → `test_execute_before_hook_timeout_fail_closed`; assert `Err(_)` on timeout.
- Add `test_execute_after_hook_fail_closed`; assert `Err(_)` on after-hook exit 1.

---

### `crates/nono-cli/src/hook_runtime_windows.rs` (NEW — windows-only service, event-driven)

**Analog:** `crates/nono-cli/src/exec_strategy_windows/launch.rs` (primary) + `dacl_guard.rs` + `labels_guard.rs`

**Gate:** `#[cfg(windows)]` module declaration in `main.rs`.

**Imports pattern** (derive from `dacl_guard.rs` lines 42–47 and `launch.rs` lines 1–13):
```rust
use crate::{exec_strategy, profile, session};
use nono::{
    path_is_owned_by_current_user, try_set_mandatory_label, NonoError, Result,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, warn};
```

**Public API surface** (mirrors Unix `hook_runtime.rs` — same function signatures, different implementations):
```rust
pub(crate) fn execute_before_hook(
    hook: &profile::SessionHook,
    session_id: &str,
    workdir: &Path,
) -> Result<Vec<(String, String)>>

pub(crate) fn execute_after_hook(
    hook: &profile::SessionHook,
    session_id: &str,
    workdir: &Path,
    child_exit_code: i32,
) -> Result<()>
```

**Windows interpreter dispatch pattern** (D-05 / Pitfall 4 — direct spawn via `Command`, NOT `spawn_windows_child`):
```rust
fn build_windows_hook_command(script: &Path) -> Result<Command> {
    let ext = script.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut cmd = match ext.to_ascii_lowercase().as_str() {
        "ps1" => {
            // Phase 60 PowerShell-steering direction: -NoProfile prevents $PROFILE
            // injection; -NonInteractive prevents stdin read-blocking; -File refs
            // only (no inline scripts — upstream no-JSON-injection rule preserved).
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-NonInteractive", "-File"]);
            c.arg(script);
            c
        }
        "cmd" | "bat" => {
            // /D disables AutoRun registry key execution (injection prevention).
            // Verify cmd.exe /D AutoRun behavior via Windows docs before implementing.
            let mut c = Command::new("cmd.exe");
            c.args(["/D", "/C"]);
            c.arg(script);
            c
        }
        _ => Command::new(script), // .exe or extensionless: direct CreateProcess
    };
    cmd.env_clear();
    Ok(cmd)
}
```

**LowIlPrimary direct spawn** (D-05; uses `nono::create_low_integrity_primary_token()` directly — NOT `spawn_windows_child`; analog: `launch.rs:1116–1125` documents `LowIlPrimary` semantics):
```rust
// The hook executor does NOT call spawn_windows_child / execute_supervised_runtime.
// It constructs its own low-IL primary token and spawns the hook directly.
// This is the correct parallel structure to the Unix side, which also does not
// use the supervised runtime for hooks.
//
// Source analog: nono::create_low_integrity_primary_token() — see
// exec_strategy_windows/launch.rs:1211 comment ("D-06: lifted into nono crate")
// for the canonical import path.
```

**Timeout via terminate_job_object** (`launch.rs:290`):
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:290
// Use pub(super) visibility — hook_runtime_windows.rs lives in the same
// crate but not in exec_strategy_windows/. Either expose as pub(crate) from
// launch.rs, or replicate the TerminateJobObject call locally with SAFETY doc.
pub(super) fn terminate_job_object(job: HANDLE, exit_code: u32) -> Result<()>
```

**WindowsEnvFileGuard: CREATE_NEW + Low-IL label** (D-08; derive from `dacl_guard.rs` Drop/RAII pattern + `labels_guard.rs:83` for `try_set_mandatory_label`):
```rust
struct WindowsEnvFileGuard {
    path: PathBuf,
}

impl WindowsEnvFileGuard {
    fn create(session_id: &str) -> Result<Self> {
        let sessions_dir = session::ensure_sessions_dir()?;
        let session_env_dir = sessions_dir.join(session_id);
        std::fs::create_dir_all(&session_env_dir).map_err(|e| {
            NonoError::ConfigParse(format!("Failed to create session dir: {e}"))
        })?;
        let path = session_env_dir.join("env");
        // CREATE_NEW: fails if file exists (equivalent to O_EXCL on Unix).
        // std::fs::OpenOptions::create_new(true) maps to CREATE_NEW disposition on Windows.
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| NonoError::ConfigParse(format!("Failed to create env file: {e}")))?;
        // Apply Low-IL mandatory label so only Low-IL+ processes (the hook + the
        // Medium-IL parent) can access the file. Primary gate for env-file trust boundary.
        // Source: nono::try_set_mandatory_label — same call used in labels_guard.rs:83
        nono::try_set_mandatory_label(&path, nono::MandatoryLabelRid::Low, ...)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path { &self.path }
}

impl Drop for WindowsEnvFileGuard {
    fn drop(&mut self) {
        // Zero-then-delete: mirror Unix EnvFileGuard's zeroize-on-drop contract
        if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(&self.path)
            && let Ok(metadata) = file.metadata()
        {
            use std::io::Write;
            let zeros = vec![0u8; metadata.len() as usize];
            let _ = file.write_all(&zeros);
            let _ = file.sync_all();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}
```

**validate_hook_script_windows pattern** (D-10; `path_is_owned_by_current_user` gating from `labels_guard.rs:66–80`):
```rust
fn validate_hook_script_windows(path: &Path) -> Result<PathBuf> {
    // 1. Absolute path — use Path::is_absolute(), NOT string starts_with (CLAUDE.md footgun)
    if !path.is_absolute() {
        return Err(NonoError::ConfigParse("Hook script path must be absolute".into()));
    }
    // 2. Canonical — std::fs::canonicalize adds \\?\ prefix on Windows automatically
    let canonical = path.canonicalize()
        .map_err(|e| NonoError::ConfigParse(format!("Hook script not found: {}: {e}", path.display())))?;
    // 3. Regular file
    let meta = canonical.metadata()
        .map_err(|e| NonoError::ConfigParse(format!("Cannot read hook metadata: {e}")))?;
    if !meta.is_file() {
        return Err(NonoError::ConfigParse("Hook script is not a regular file".into()));
    }
    // 4. Owner check — reuse existing helper (pattern from labels_guard.rs:66-80)
    // Analog: labels_guard.rs ALWAYS pairs path_is_owned_by_current_user with
    // an effective-rights check per feedback_windows_mandatory_label_write_owner.
    match nono::path_is_owned_by_current_user(&canonical)? {
        false => return Err(NonoError::ConfigParse("Hook script not owned by current user".into())),
        true => {}
    }
    // 5. No world-writable or lower-IL-writable ACL on file AND parent
    //    Use GetEffectiveRightsFromAclW to verify no Write ACE for Everyone (S-1-1-0)
    // 6. Mandatory-label check (consistency gate, not an apply)
    Ok(canonical)
}
```

---

### `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (MODIFIED — utility, transform)

**Analog:** Self — extend `is_dangerous_env_var()` at `env_sanitization.rs:14`

**Current function signature** (`env_sanitization.rs:14`):
```rust
pub(crate) fn is_dangerous_env_var(key: &str) -> bool {
    // ... existing Unix vars using == exact match ...
    || key.starts_with("OP_SESSION_")
}
```

**Extension pattern for Windows danger vars** (D-09; use `env_key_matches` for case-insensitive Windows comparison, matching existing pattern at `env_sanitization.rs:55–61`):
```rust
// Windows hook env-file injection vectors (Low-IL-writer → Medium-IL-reader gap).
// On Windows, env-var comparison is case-insensitive — use eq_ignore_ascii_case.
// Source: env_sanitization.rs:55-61 — env_key_matches already uses eq_ignore_ascii_case on Windows.
// Extend is_dangerous_env_var directly (single source of truth per RESEARCH.md "Don't Hand-Roll").
|| key.eq_ignore_ascii_case("PATH")       // executable resolution hijacking
|| key.eq_ignore_ascii_case("PATHEXT")    // extension association hijacking
|| key.eq_ignore_ascii_case("COMSPEC")    // cmd interpreter redirect
|| key.eq_ignore_ascii_case("PSModulePath")           // PowerShell module injection
|| key.eq_ignore_ascii_case("PSModuleAnalysisCachePath") // PS analysis cache poisoning
|| key.eq_ignore_ascii_case("__PSLockdownPolicy")     // PS constrained-language bypass
|| key.eq_ignore_ascii_case("SystemRoot") // system DLL resolution redirect
|| key.eq_ignore_ascii_case("windir")     // system directory redirect
|| key.eq_ignore_ascii_case("TEMP")       // temp file redirect from parent perspective
|| key.eq_ignore_ascii_case("TMP")        // same
```

**New test pattern** (follow existing `env_sanitization.rs:222–231` Windows test block):
```rust
#[cfg(target_os = "windows")]
#[test]
fn test_windows_dangerous_vars_blocked() {
    assert!(is_dangerous_env_var("PATH"));
    assert!(is_dangerous_env_var("Path"));     // case-insensitive
    assert!(is_dangerous_env_var("PATHEXT"));
    assert!(is_dangerous_env_var("COMSPEC"));
    assert!(is_dangerous_env_var("PSModulePath"));
    assert!(is_dangerous_env_var("__PSLockdownPolicy"));
    assert!(is_dangerous_env_var("SystemRoot"));
    assert!(is_dangerous_env_var("windir"));
    assert!(is_dangerous_env_var("TEMP"));
    assert!(is_dangerous_env_var("TMP"));
}
```

---

### `crates/nono-cli/src/profile/mod.rs` (MODIFIED — config/model, CRUD)

**Analog:** `windows_low_il_broker` field threading — the exact 4-location lockstep pattern.

**Location 1: `Profile` struct** (analog: `profile/mod.rs:2183–2184`):
```rust
// Phase 58: session lifecycle hooks. Schema-cross-platform (deserializes on all
// platforms); runtime execution is platform-gated in hook_runtime.rs/hook_runtime_windows.rs.
#[serde(default)]
pub session_hooks: profile::SessionHooks,
```

**Location 2: `ProfileDeserialize` struct** (analog: `profile/mod.rs:2245–2246`):
```rust
// deny_unknown_fields on ProfileDeserialize requires this entry or the field
// silently errors on deserialization from JSON. Cross-platform: must appear here
// even though execution is platform-gated.
#[serde(default)]
session_hooks: SessionHooks,
```

**Location 3: `From<ProfileDeserialize> for Profile`** (analog: `profile/mod.rs:2281`):
```rust
// Source: profile/mod.rs:2286-2289 "Exhaustively enumerated here so rustc's
// struct-literal completeness check catches any future field additions."
session_hooks: raw.session_hooks,
```

**Location 4: `merge_profiles`** (analog: `profile/mod.rs:3171`; use Option-semantics, not OR-semantics, per upstream daa55c8 and RESEARCH.md):
```rust
// Child overrides base per-hook slot. Option-semantics: child wins per slot.
// Source: upstream daa55c8 profile/mod.rs merge semantics.
session_hooks: SessionHooks {
    before: child.session_hooks.before.or(base.session_hooks.before),
    after: child.session_hooks.after.or(base.session_hooks.after),
},
```

**New structs** (upstream `daa55c8:profile/mod.rs` lines 1147–1173 — port verbatim):
```rust
/// A single session lifecycle hook configuration.
///
/// Defines a script to execute before or after the sandboxed session.
/// Scripts run outside the sandbox. On Windows, confined to Low-IL via broker
/// (see `.planning/architecture/adr-58-windows-hook-executor.md`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionHook {
    pub script: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

/// Session lifecycle hooks for a profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionHooks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<SessionHook>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<SessionHook>,
}
```

**CRITICAL (Pitfall 1):** `SessionHooks` / `SessionHook` are for session-lifecycle execution. They are NOT `HooksConfig` / `HookConfig` (which appear at `profile/mod.rs:2230`, the Claude Code PreToolUse/PostToolUse install config). Keep them separate.

**Test module pattern** (follow `windows_low_il_broker_tests` at `profile/mod.rs:7637` for the new session_hooks test block):
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod session_hooks_tests {
    // test_session_hooks_basic_deserialize
    // test_session_hooks_rejects_unknown_field        (deny_unknown_fields on SessionHook)
    // test_merge_profiles_session_hooks_child_overrides_per_field
    // test_merge_profiles_session_hooks_child_inherits_when_absent
}
```

---

### `crates/nono-cli/src/policy.rs` (MODIFIED — config, transform)

**Analog:** `policy.rs:155–212` — `to_raw_profile()` struct literal.

**ProfileDef struct field addition** (analog: `policy.rs:146–150`):
```rust
// Phase 58: session lifecycle hooks from policy.json built-in profiles.
// Built-in profiles may declare session_hooks in policy.json; forwarded verbatim.
#[serde(default)]
pub session_hooks: profile::SessionHooks,
```

**to_raw_profile() addition** (analog: `policy.rs:208–210` `windows_low_il_broker` forwarding):
```rust
// Phase 58: forward session_hooks from policy.json built-in profile verbatim.
session_hooks: self.session_hooks.clone(),
```

The `to_raw_profile()` function at `policy.rs:155` produces a `profile::Profile` struct literal. Add `session_hooks` alongside `windows_low_il_broker` (line 210) to maintain the completeness invariant.

---

### `crates/nono-cli/src/sandbox_prepare.rs` (MODIFIED — service, CRUD)

**Analog:** `sandbox_prepare.rs:87–95` (`allowed_env_vars` / `denied_env_vars` field pattern) and `sandbox_prepare.rs:527–532` (struct-literal construction site).

**PreparedSandbox struct field addition** (analog: `sandbox_prepare.rs:87–95`):
```rust
/// Phase 58: session lifecycle hooks carried forward from loaded profile
/// for dispatch in execution_runtime::execute_sandboxed.
pub(crate) session_hooks: profile::SessionHooks,
```

**Struct literal construction** (analog: `sandbox_prepare.rs:527–532`):
```rust
// Phase 58: carry session_hooks from the loaded profile into PreparedSandbox.
// Follows the allowed_env_vars pattern at lines 527-532.
session_hooks: loaded_profile
    .as_ref()
    .map(|p| p.session_hooks.clone())
    .unwrap_or_default(),
```

---

### `crates/nono-cli/src/launch_runtime.rs` (MODIFIED — config/model, CRUD)

**Analog:** `launch_runtime.rs:162–204` (`ExecutionFlags` struct fields) and `launch_runtime.rs:206–235` (`ExecutionFlags::defaults()`).

**ExecutionFlags struct field addition** (analog: `launch_runtime.rs:190–196` pattern with `#[cfg_attr]` comment when platform-gated; `session_hooks` is NOT platform-gated per RESEARCH.md anti-pattern):
```rust
/// Phase 58: session lifecycle hooks to execute before/after the sandboxed child.
/// Runtime dispatch is platform-specific (hook_runtime.rs / hook_runtime_windows.rs)
/// but the field is cross-platform so profiles round-trip cleanly on all platforms.
pub(crate) session_hooks: profile::SessionHooks,
```

**ExecutionFlags::defaults() addition** (analog: `launch_runtime.rs:220–233`):
```rust
session_hooks: profile::SessionHooks::default(),
```

**LaunchPlan / prepare_run_launch_plan wiring** (upstream `daa55c8:launch_runtime.rs` added a 3-line wiring block — wire `prepared.session_hooks` into `flags.session_hooks`):
```rust
// Phase 58: wire session_hooks from PreparedSandbox into ExecutionFlags.
// Source: upstream daa55c8 launch_runtime.rs 3-line addition.
flags.session_hooks = prepared.session_hooks.clone();
```

---

### `crates/nono-cli/src/execution_runtime.rs` (MODIFIED — service, request-response)

**Analog:** upstream `daa55c8:execution_runtime.rs` hook wiring lines 249–295 (before-hook) and 417–424 (after-hook). The fork's `execution_runtime.rs` currently has NEITHER — both are absent (confirmed: `grep session_hooks` returns no matches).

**Before-hook wiring block** (upstream `daa55c8:execution_runtime.rs` lines 249–295, FORK DIVERGENCE at error handling — upstream uses `warn!` + `Vec::new()`, fork uses `?` to propagate):
```rust
// Session id shared across before- and after-hook so paired setup/teardown
// scripts see the same NONO_SESSION_ID. Only allocated when at least one
// hook is configured.
let hook_session_id: Option<String> =
    (flags.session_hooks.before.is_some() || flags.session_hooks.after.is_some()).then(|| {
        std::env::var(DETACHED_SESSION_ID_ENV)
            .ok()
            .filter(|id| !id.is_empty())
            .unwrap_or_else(session::generate_session_id)
    });

// ---- Before-hook execution ----
// FORK DIVERGENCE (D-01/D-03): before-hook failure returns Err (session aborts).
// Upstream warns and continues (fail-open); fork propagates Err (fail-closed).
let hook_env_vars_owned: Vec<(String, String)> = if let Some((before, session_id)) =
    flags.session_hooks.before.as_ref().zip(hook_session_id.as_deref())
{
    #[cfg(unix)]
    {
        crate::hook_runtime::execute_before_hook(before, session_id, &current_dir)?
        // ^ note: ? propagates Err, not warn-and-continue
    }
    #[cfg(windows)]
    {
        crate::hook_runtime_windows::execute_before_hook(before, session_id, &current_dir)?
    }
    #[cfg(not(any(unix, windows)))]
    { Vec::new() }
} else {
    Vec::new()
};

// Hook env vars have lowest priority: prepend so secrets and proxy override.
for (key, value) in hook_env_vars_owned.iter().rev() {
    env_vars.insert(0, (key.as_str(), value.as_str()));
}
```

**After-hook wiring block** (upstream `daa55c8:execution_runtime.rs` lines 417–424, FORK DIVERGENCE per D-04 — after-hook failure is loud + Err, not warn-and-swallow):
```rust
// ---- After-hook execution ----
// FORK DIVERGENCE (D-04): after-hook failure propagates Err so CI sees non-zero exit.
// Mirror the fork's diagnostic-footer pattern for the loud error.
if let Some((after, session_id)) =
    flags.session_hooks.after.as_ref().zip(hook_session_id.as_deref())
{
    #[cfg(unix)]
    crate::hook_runtime::execute_after_hook(after, session_id, &current_dir, exit_code)?;
    #[cfg(windows)]
    crate::hook_runtime_windows::execute_after_hook(after, session_id, &current_dir, exit_code)?;
}
```

**Import addition** (analog: `execution_runtime.rs` line 1 which uses `#[cfg(not(target_os = "windows"))]` for Unix-only imports — mirror for hook_runtime):
```rust
#[cfg(unix)]
use crate::hook_runtime;
#[cfg(windows)]
use crate::hook_runtime_windows;
```

---

### `crates/nono-cli/src/main.rs` (MODIFIED — module declarations)

**Analog:** `main.rs:24–80` — existing platform-gated module patterns.

**Module declaration pattern** (analog: `main.rs:41–45` `learn` pattern for Unix-only with Windows alias; analog: `main.rs:26–30` for `#[cfg(not)]` / `#[cfg]` / `#[path]` triple):
```rust
// Session lifecycle hook runtime (Phase 58).
// Unix runtime (cfg(unix)) — gated unix-only per upstream 1335351.
#[cfg(unix)]
mod hook_runtime;
// Windows runtime (cfg(windows)) — net-new fork work; real implementation,
// NOT an empty stub (unlike upstream's #[cfg(not(unix))] no-op branch).
#[cfg(windows)]
mod hook_runtime_windows;
```

**Placement:** Insert alphabetically with the `h` modules, after `mod hooks;` (line 33).

---

### `crates/nono-cli/data/nono-profile.schema.json` (MODIFIED — config)

**Analog:** upstream `daa55c8:data/nono-profile.schema.json` — port the `session_hooks` property + `SessionHooks`/`SessionHook` `$defs` verbatim.

**Property addition in `"properties"` block** (after the `"hooks"` entry at schema line 67):
```json
"session_hooks": {
  "$ref": "#/$defs/SessionHooks",
  "description": "Session lifecycle hooks. Scripts run outside the sandbox with host privileges before and after the sandboxed process."
}
```

**`$defs` additions** (upstream `daa55c8` schema lines 762–800; insert after `HookConfig` def at current schema line 763):
```json
"SessionHooks": {
  "type": "object",
  "description": "Session lifecycle hooks. before runs before the sandboxed child is forked; after runs after it exits. Both run outside the sandbox with host privileges.",
  "additionalProperties": false,
  "properties": {
    "before": {
      "$ref": "#/$defs/SessionHook",
      "description": "Hook executed before the sandboxed process starts. May export environment variables to the sandboxed child via NONO_ENV_FILE."
    },
    "after": {
      "$ref": "#/$defs/SessionHook",
      "description": "Hook executed after the sandboxed process exits. Receives the child exit code via NONO_EXIT_CODE."
    }
  }
},
"SessionHook": {
  "type": "object",
  "description": "A single session lifecycle hook configuration.",
  "additionalProperties": false,
  "required": ["script"],
  "properties": {
    "script": {
      "type": "string",
      "description": "Absolute path to the hook script. Must be an executable regular file owned by the current user or root, not located in a world-writable directory. Validated at execution time."
    },
    "timeout_secs": {
      "type": "integer",
      "minimum": 1,
      "description": "Optional timeout in seconds. If set, the hook is killed after this duration. If absent, no timeout is enforced."
    }
  }
}
```

---

### `.planning/architecture/adr-58-windows-hook-executor.md` (NEW — ADR)

**Analog:** `docs/architecture/broker-trust-anchor.md` (Phase 32 ADR) — follow this structure.

**Required header structure** (from `docs/architecture/broker-trust-anchor.md:1–8`):
```markdown
# Windows Session Lifecycle Hook Executor

**Status:** Accepted
**Date:** <date>
**Phase:** 58 (session-lifecycle-hooks)
**Decision IDs:** D-05, D-06, D-07, D-08, D-09, D-10
**Related ADR:** [broker-trust-anchor.md](../../docs/architecture/broker-trust-anchor.md)
  (Phase 32 — establishes `LowIlPrimary` arm and its trust invariants)
```

**Required sections per CONTEXT.md and RESEARCH.md:**
1. **Context** — Why Windows hooks diverge from upstream's "host-privileged, outside sandbox" semantics.
2. **Goals** — Enumerate D-05–D-10 verbatim.
3. **Non-goals** — Explicitly call out the upstream fail-open behavior as NOT adopted.
4. **Decision Table** — At minimum: `LowIlPrimary` arm vs `WriteRestricted` (D-05); env-file Low-IL label vs DACL-only (D-08); fail-closed vs fail-open (D-01).
5. **Trust Boundary** — The Low-IL-writer → Medium-IL-reader gap; mitigation = restrictive ACL + mandatory label + `is_dangerous_env_var()` filter.
6. **Invariants** — Mandatory-label enforcement; no host-trusted hook execution; session-dir+cwd-only scope; script-file references only; fail-closed divergence from upstream.
7. **Fork Divergence Record** — Cite D-01/D-02 explicitly; state this is the canonical record per D-02's requirement.
8. **Alternatives Considered** — `WriteRestricted` (D-05 rejected: CLR fails 0xC0000142); `BrokerLaunchNoPty` (rejected for hooks: hooks are short-lived, no PTY needed); profile-level fail-open toggle (rejected: D-01 chose unconditional fail-closed).

**File path:** `.planning/architecture/adr-58-windows-hook-executor.md`
(Consistent with `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` location precedent; note `docs/architecture/` also holds ADRs but the `.planning/architecture/` location is the established v2.6+ convention per D-46-A2.)

---

## Shared Patterns

### Session-Dir Path Construction
**Source:** `crates/nono-cli/src/session.rs:409–434`
**Apply to:** `hook_runtime.rs` (`EnvFileGuard::create`) and `hook_runtime_windows.rs` (`WindowsEnvFileGuard::create`)
```rust
// Use session::ensure_sessions_dir() — handles NONO_TEST_HOME override,
// 0o700 permissions on Unix, and cross-platform path construction.
// Already used in execution_runtime.rs via the session::generate_session_id call.
let sessions_dir = session::ensure_sessions_dir()?;
let session_env_dir = sessions_dir.join(session_id);
```

### RAII Drop Pattern (zeroize + unlink)
**Source:** `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` (Drop pattern) + upstream `daa55c8:hook_runtime.rs` (`EnvFileGuard::Drop`)
**Apply to:** `EnvFileGuard` (Unix) and `WindowsEnvFileGuard` (Windows)

The Drop contract: zero-fill file contents, then unlink. Prevents env-file contents from being readable after the hook exits even if the OS delays unlink. Both platforms implement identically.

### Error Propagation via `?`
**Source:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:82–100` (fail-closed pattern)
**Apply to:** All new functions in `hook_runtime.rs` and `hook_runtime_windows.rs`

Per CLAUDE.md: no `.unwrap()` / `.expect()` in production paths. All errors propagate via `?`. Tests use `#[allow(clippy::unwrap_used)]`.

### ENV_LOCK for Tests That Mutate HOME
**Source:** `crates/nono-cli/src/test_env.rs:10–16`
**Apply to:** All tests in `hook_runtime.rs` and `hook_runtime_windows.rs` that call `session::ensure_sessions_dir()` (which reads HOME via `nono_home_dir()`).

```rust
// From test_env.rs:12-16 — use lock_env() or acquire ENV_LOCK directly:
let lock = match crate::test_env::ENV_LOCK.lock() {
    Ok(g) => g,
    Err(poisoned) => poisoned.into_inner(),
};
// Then set HOME via EnvVarGuard::set_all
```

### path_is_owned_by_current_user + Effective-Rights Pair
**Source:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:66–80`
**Apply to:** `validate_hook_script_windows` (D-10)

Per `feedback_windows_mandatory_label_write_owner`: ownership check alone is insufficient. ALWAYS pair `path_is_owned_by_current_user` with an effective-rights mask check. The labels_guard pattern is the established discipline.

### `#[cfg_attr(not(...), allow(dead_code))]` for Platform-Gated Fields
**Source:** `crates/nono-cli/src/launch_runtime.rs:177–191`
**Apply to:** `ExecutionFlags.session_hooks` if the field is only consumed inside platform-gated blocks.

The existing pattern:
```rust
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) interactive_shell: bool,
```
Mirror for `session_hooks` if the field is only read inside `#[cfg(unix)]` / `#[cfg(windows)]` blocks in `execution_runtime.rs`. If it is always read (via platform dispatch), no attribute is needed.

---

## No Analog Found

All files have close analogs in the codebase. No items in this table.

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/`, `crates/nono-cli/data/`, `.planning/architecture/`, `docs/architecture/`, upstream commits `daa55c8` and `1335351` (via `git show`)
**Files scanned:** 14 source files + 2 upstream commit trees
**Pattern extraction date:** 2026-06-05
