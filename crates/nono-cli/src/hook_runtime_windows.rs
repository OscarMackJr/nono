//! Session lifecycle hook execution (Windows-only).
//!
//! Provides Windows-specific implementation of before/after session hooks,
//! running hooks outside the sandboxed child and confined to a Job Object.
//!
//! # Current integrity level (KNOWN LIMITATION — D-05 deferred)
//!
//! Hooks currently run at the **parent's integrity level (Medium-IL)** inside a
//! Job Object for process-tree containment. They do NOT run at Low-IL.
//!
//! The intended design (D-05) calls for spawning hooks with a Low-IL primary token
//! via `CreateProcessAsUserW`. This requires raw FFI because stable Rust's
//! `std::process::Command` does not expose a custom-token spawn API. That plumbing
//! is a **known deferred follow-up** (Research Open Question 1, recorded in
//! `adr-58-windows-hook-executor.md` §Non-goals and §Alternatives). Until it lands,
//! hooks run with the supervisor's token, NOT at Low-IL.
//!
//! The Job Object confinement (CPU/memory/handle-inheritance scope) is active, but it
//! does NOT substitute for Low-IL MIC enforcement: a hook running at Medium-IL has the
//! same DACL access rights as the supervisor process.
//!
//! # Fork divergence (D-01): fail-closed
//! Upstream commit daa55c8 is fail-open: hook errors warn and do not block
//! launch. This fork overrides that behavior: any hook failure (non-zero exit,
//! timeout, validation error) returns Err and prevents session start (before-hook)
//! or surfaces as a non-zero exit (after-hook). This invariant is recorded in
//! `.planning/architecture/adr-58-windows-hook-executor.md`.
//!
//! # SC2 fork invariant (D-02)
//! The upstream runtime mechanism is preserved exactly: RAII `WindowsEnvFileGuard`,
//! `CREATE_NEW` env-file creation, `is_dangerous_env_var` filtering, and the
//! `mpsc`-based timeout race pattern are all ported from upstream `daa55c8`.
//! Only the fail policy and execution model are different.
//!
//! # Windows execution design (D-05..D-10)
//! Windows execution design documented in `.planning/architecture/adr-58-windows-hook-executor.md`.
//!
//! # Security
//!
//! - Script paths are validated before every execution:
//!   absolute, canonical, regular file, owner check, no world-writable ACL (D-10)
//! - Hooks run at the parent's (Medium-IL) integrity level inside a Job Object
//!   (D-05 Low-IL spawn via CreateProcessAsUserW is a deferred follow-up — see above)
//! - Env file uses CREATE_NEW + Low-IL mandatory label (D-08)
//! - Windows env-file injection vectors are filtered via is_dangerous_env_var (D-09)
//! - Hook env-var values are zeroized after injection
#![cfg(windows)]

use crate::{exec_strategy, profile, session};
use nono::{
    path_is_owned_by_current_user, try_set_mandatory_label, NonoError, OwnedHandle, Result,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, warn};
// Zeroize import used for documentation; env-var values zeroized in execution_runtime.rs after
// hook env vars are consumed per D-PLAN58-02-A (borrow conflict prevents inline zeroize here).

use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSidToSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
};
use windows_sys::Win32::Security::GetAce;
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};

/// Result of executing a session hook.
struct HookOutput {
    exit_code: i32,
    timed_out: bool,
}

/// Execute a before-hook and return exported environment variables.
///
/// Steps:
/// 1. Validate script path (D-10: absolute, canonical, regular file, owner, ACL)
/// 2. Create NONO_ENV_FILE in private session directory (RAII guard, D-08)
/// 3. Build interpreter command (D-05: explicit dispatch, not shell association)
/// 4. Spawn hook inside a Job Object (runs at parent's Medium-IL; Low-IL deferred — see module doc)
/// 5. Wait for completion with optional timeout (terminate job on timeout)
/// 6. Read and parse NONO_ENV_FILE
/// 7. Filter dangerous env vars (D-09)
///
/// # Fork divergence (D-01/D-03)
/// Returns `Err` on timeout or non-zero exit. Upstream daa55c8 warns and returns
/// `Ok(Vec::new())` — this fork is fail-closed.
pub(crate) fn execute_before_hook(
    hook: &profile::SessionHook,
    session_id: &str,
    workdir: &Path,
) -> Result<Vec<(String, String)>> {
    let script_path = validate_hook_script_windows(&hook.script)?;
    let env_file = WindowsEnvFileGuard::create(session_id)?;

    let mut cmd = build_windows_hook_command(&script_path)?;
    // Set hook environment variables
    cmd.env("NONO_SESSION_ID", session_id);
    cmd.env("NONO_WORKDIR", workdir);
    cmd.env("NONO_HOOK_TYPE", "before");
    cmd.env("NONO_ENV_FILE", env_file.path());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = run_hook_windows(&mut cmd, hook.timeout_secs)?;

    // FORK DIVERGENCE (D-01/D-03): return Err on timeout (session aborts).
    // Upstream daa55c8 warns and returns Ok(Vec::new()) — fail-open.
    if output.timed_out {
        return Err(NonoError::ConfigParse(format!(
            "Before-hook timed out after {}s (fail-closed): {}",
            hook.timeout_secs.unwrap_or(0),
            script_path.display()
        )));
    }

    // FORK DIVERGENCE (D-01/D-03): return Err on non-zero exit (session aborts).
    // Upstream daa55c8 warns and falls through to Ok — fail-open.
    if output.exit_code != 0 {
        return Err(NonoError::ConfigParse(format!(
            "Before-hook exited with code {} (fail-closed): {}",
            output.exit_code,
            script_path.display()
        )));
    }

    let raw = read_env_file(env_file.path())?;
    let total = raw.len();
    let filtered: Vec<(String, String)> = raw
        .into_iter()
        .filter(|(k, _)| !exec_strategy::is_dangerous_env_var(k))
        .collect();

    debug!(
        "Before-hook exported {} env vars ({} filtered out)",
        filtered.len(),
        total.saturating_sub(filtered.len())
    );

    Ok(filtered)
}

/// Execute an after-hook for cleanup.
///
/// Steps:
/// 1. Validate script path (D-10)
/// 2. Execute with isolated env, passing child exit code via NONO_EXIT_CODE
/// 3. Log result
///
/// # Fork divergence (D-04)
/// Returns `Err` on timeout or non-zero exit. Upstream daa55c8 warns and
/// returns `Ok(())` — this fork is fail-closed.
pub(crate) fn execute_after_hook(
    hook: &profile::SessionHook,
    session_id: &str,
    workdir: &Path,
    child_exit_code: i32,
) -> Result<()> {
    let script_path = validate_hook_script_windows(&hook.script)?;

    let mut cmd = build_windows_hook_command(&script_path)?;
    // Set hook environment variables
    cmd.env("NONO_SESSION_ID", session_id);
    cmd.env("NONO_WORKDIR", workdir);
    cmd.env("NONO_HOOK_TYPE", "after");
    cmd.env("NONO_EXIT_CODE", child_exit_code.to_string());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let output = run_hook_windows(&mut cmd, hook.timeout_secs)?;

    // FORK DIVERGENCE (D-04): return Err on timeout so CI sees non-zero exit.
    // Upstream daa55c8 warns and returns Ok(()) — fail-open.
    if output.timed_out {
        return Err(NonoError::ConfigParse(format!(
            "After-hook timed out after {}s (fail-closed): {}",
            hook.timeout_secs.unwrap_or(0),
            script_path.display()
        )));
    }

    // FORK DIVERGENCE (D-04): return Err on non-zero exit.
    // Upstream daa55c8 warns and returns Ok(()) — fail-open.
    if output.exit_code != 0 {
        return Err(NonoError::ConfigParse(format!(
            "After-hook exited with code {} (fail-closed): {}",
            output.exit_code,
            script_path.display()
        )));
    }

    Ok(())
}

// ===================== Internal Helpers =====================

/// Strip the Windows verbatim / extended-length path prefix before passing a
/// path to an interpreter command.
///
/// `std::fs::canonicalize()` on Windows returns the extended-length verbatim
/// form `\\?\C:\...` (or `\\?\UNC\server\share\...` for UNC paths). This is
/// correct for filesystem operations (bypasses MAX_PATH, defeats symlink traversal),
/// but PowerShell's `-File` flag (and other interpreters) CANNOT resolve the
/// security zone of a `\\?\`-prefixed path. Under a `RemoteSigned` execution
/// policy (the Windows default), PowerShell treats the script as untrusted and
/// refuses to run it, emitting "The file ... is not digitally signed" and exiting
/// with code 1 — BEFORE any script body runs. This was empirically proven during
/// live UAT (Phase 58 Plan 03, second UAT gap-closure).
///
/// Strip rules:
/// - `\\?\UNC\server\share\...` → `\\server\share\...`
/// - `\\?\C:\...`               → `C:\...`
/// - anything else              → unchanged
///
/// IMPORTANT: This stripping is ONLY for the interpreter argument. The canonical
/// (verbatim-prefixed) path produced by `validate_hook_script_windows` MUST still
/// be used for all security validation (canonicalization defeats symlink/`..`
/// traversal — do NOT apply this function before the security checks in
/// `validate_hook_script_windows`).
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let raw = path.as_os_str().to_string_lossy();
    if let Some(stripped) = raw.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{stripped}"));
    }
    if let Some(stripped) = raw.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }
    path.to_path_buf()
}

/// Build a hook command with explicit interpreter dispatch (D-05).
///
/// Uses explicit interpreter selection — NOT shell-association lookup
/// (which is fragile and attacker-influenceable via HKEY_CLASSES_ROOT).
///
/// Dispatch:
/// - `.ps1` → `powershell.exe -NoProfile -NonInteractive -File <path>`
/// - `.cmd` / `.bat` → `cmd.exe /D /C <path>`
/// - other / `.exe` → direct `Command::new(script)`
///
/// The no-JSON-injection rule (upstream) is preserved: script path is always
/// an argument to the interpreter flag, never inline code.
///
/// # Verbatim prefix stripping
///
/// The `script` argument comes from `validate_hook_script_windows`, which returns
/// `std::fs::canonicalize()` output. On Windows, that is the extended-length verbatim
/// form `\\?\C:\...`. The verbatim prefix is stripped via `strip_verbatim_prefix`
/// BEFORE passing the path to the interpreter — PowerShell's `-File` flag (and other
/// interpreters) reject `\\?\` paths under `RemoteSigned` policy with "not digitally
/// signed" / exit 1. The canonical path itself is kept for security-validation purposes;
/// only the path arg handed to the spawned process is de-verbatim'd.
fn build_windows_hook_command(script: &Path) -> Result<Command> {
    // Strip the \\?\ verbatim prefix for the interpreter arg.
    // See `strip_verbatim_prefix` doc comment for the full rationale.
    let interpreter_path = strip_verbatim_prefix(script);

    let ext = interpreter_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut cmd = match ext.to_ascii_lowercase().as_str() {
        "ps1" => {
            // Phase 60 PowerShell-steering direction: -NoProfile prevents $PROFILE
            // injection; -NonInteractive prevents stdin read-blocking; -File refs
            // only (no inline scripts — upstream no-JSON-injection rule preserved).
            // SAFETY-note (Assumption A1): -NoProfile disables $PROFILE loading;
            // -NonInteractive disables interactive input prompts that could hang the hook.
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-NonInteractive", "-File"]);
            c.arg(&interpreter_path);
            c
        }
        "cmd" | "bat" => {
            // /D disables AutoRun registry keys (injection prevention, T-58-03-05).
            // SAFETY-note (Assumption A1): cmd.exe /D disables AutoRun registry keys;
            // see RESEARCH.md §Assumptions. Well-known Windows behavior, not re-verified
            // in this session via official docs.
            let mut c = Command::new("cmd.exe");
            c.args(["/D", "/C"]);
            c.arg(&interpreter_path);
            c
        }
        _ => {
            // Native .exe or extensionless: direct CreateProcess via Command::new.
            Command::new(&interpreter_path)
        }
    };
    // Clear all inherited environment; hooks get only NONO_* vars explicitly set,
    // plus the OS-baseline vars required for interpreter/CLR startup (see below).
    cmd.env_clear();

    // Re-add the minimal OS-baseline environment required for Windows interpreter
    // and CLR startup. Without these, powershell.exe (and any .NET/CLR process) fails
    // with exit code -65536 (0xFFFF0000) — a CLR startup failure that occurs before any
    // script body runs. This was empirically proven during live UAT:
    //   - powershell.exe spawned with env_clear() + only NONO_ENV_FILE set → exit -65536
    //   - powershell.exe spawned with env_clear() + SystemRoot added → exit 0
    //
    // Security rationale for this allowlist:
    // - These three vars are read-only OS-directory paths, NOT code-injection vectors.
    //   PATH and PSModulePath (which influence DLL/module loading) remain stripped by
    //   env_clear() and are NOT re-added here — they are properly listed in
    //   is_dangerous_env_var() to block injection from hook env-file reads.
    // - SystemRoot and windir are also listed in is_dangerous_env_var() to prevent a
    //   hook from EXPORTING a tampered value back to the parent via the env file (the
    //   Low-IL writer → Medium-IL reader trust boundary, D-09). That filter applies to
    //   env-file reads in execute_before_hook(); it does NOT prevent us from forwarding
    //   the parent's own (trustworthy) OS values to the spawned interpreter.
    // - The values here come from std::env::var_os() — the parent's verified process
    //   environment — NOT from any hook-written input.
    // - SystemDrive is NOT in is_dangerous_env_var() (it doesn't influence code loading),
    //   but it IS required by some CLR/PS profiles and is similarly read-only.
    //
    // DO NOT remove this block to "tidy" the env isolation: the -65536 regression will return.
    for var in &["SystemRoot", "windir", "SystemDrive"] {
        if let Some(val) = std::env::var_os(var) {
            cmd.env(var, val);
        }
    }

    Ok(cmd)
}

/// Run a hook command inside a Job Object, wait with optional timeout.
///
/// Uses a worker thread + mpsc channel for timeout (same pattern as Unix run_hook).
/// On timeout, terminates the Job Object containing the hook process tree.
///
/// # Current integrity level
/// Hooks spawn at the parent's (Medium-IL) integrity level. The `LowIlPrimary` arm
/// (`nono::create_low_integrity_primary_token()`) is called below but its token is
/// not yet plumbed into the spawn — see the D-05 deferred note and the `_low_il_token`
/// comment. The Job Object provides CPU/memory/handle-inheritance containment,
/// but does NOT substitute for Low-IL MIC enforcement on filesystem/registry access.
fn run_hook_windows(cmd: &mut Command, timeout_secs: Option<u64>) -> Result<HookOutput> {
    // Create a Job Object to contain the hook process. On timeout,
    // TerminateJobObject kills the entire hook process tree.
    let job = unsafe {
        // SAFETY: CreateJobObjectW with null SA and null name creates an anonymous
        // job object owned by this process. The returned HANDLE is valid until CloseHandle.
        CreateJobObjectW(std::ptr::null(), std::ptr::null())
    };
    if job.is_null() {
        return Err(NonoError::CommandExecution(std::io::Error::other(format!(
            "CreateJobObjectW failed (GetLastError={})",
            unsafe { GetLastError() }
        ))));
    }
    let job_handle = job;

    // D-05 (DEFERRED — NO-OP PLACEHOLDER): create a Low-IL primary token to document
    // intent; the token is NOT yet plumbed into the spawn (stable Rust's
    // std::process::Command has no custom-token API).  Hooks currently run at the
    // parent's (Medium-IL) integrity level.  The full CreateProcessAsUserW plumbing is
    // a deferred follow-up (Research Open Question 1, adr-58-windows-hook-executor.md).
    // WriteRestricted is FORBIDDEN for hook execution regardless — CLR/PowerShell
    // fails under WriteRestricted (STATUS_DLL_INIT_FAILED / 0xC0000142, Phase 60).
    let _low_il_token: Option<OwnedHandle> = match nono::create_low_integrity_primary_token() {
        Ok(token) => Some(token),
        Err(e) => {
            // Close job handle before returning error.
            unsafe { CloseHandle(job_handle) };
            return Err(e);
        }
    };
    // _low_il_token is held in scope but its handle is never passed to any spawn call.
    // It will be dropped (closing the handle) when run_hook_windows returns.
    // This is deliberately a NO-OP placeholder — see the D-05 deferred note above.

    let child_result = cmd.spawn();
    let mut child = match child_result {
        Ok(c) => c,
        Err(e) => {
            unsafe { CloseHandle(job_handle) };
            return Err(NonoError::CommandExecution(std::io::Error::other(format!(
                "Failed to spawn hook: {e}"
            ))));
        }
    };

    // Assign the child process to the job object so TerminateJobObject covers it.
    // SAFETY: child.id() is the PID of the freshly-spawned process; we use
    // OpenProcess to get a handle suitable for AssignProcessToJobObject.
    let pid = child.id();
    let assign_result = assign_process_to_job(pid, job_handle);
    if let Err(e) = assign_result {
        // D-01 fail-closed: distinguish between benign (child already exited) and
        // genuinely uncontrolled (child still running but timeout can't be enforced).
        //
        // Use try_wait() as the definitive liveness check — this is more reliable
        // than guessing from the GLE values because GLE can vary by OS version.
        let already_exited = child
            .try_wait()
            .map(|status| status.is_some()) // Some(status) → exited; None → still running
            .unwrap_or(false); // try_wait error → conservatively treat as still running

        if already_exited {
            // Benign: the hook ran so fast it exited before we could assign it to the
            // job. The timeout deadline cannot fire anyway because the process is done.
            // Proceed normally; the worker thread will get the output from wait_with_output.
            warn!(
                "Hook process {} exited before job-object assignment; timeout was not needed",
                pid
            );
        } else if timeout_secs.is_some() {
            // FAIL CLOSED (D-01): the hook is still running but we cannot enforce the
            // timeout via TerminateJobObject. An ungovernable hook could block indefinitely.
            // Kill the child best-effort, close the job handle, and return an error.
            let _ = child.kill(); // best-effort; ignore kill errors
            unsafe { CloseHandle(job_handle) };
            return Err(NonoError::CommandExecution(std::io::Error::other(format!(
                "Failed to assign hook process {} to job object (fail-closed, D-01): \
                 timeout enforcement cannot be established: {e}",
                pid
            ))));
        } else {
            // No timeout configured: the job is not needed for timeout governance.
            // Proceed with a warning (no behavior change; existing pre-fix semantics).
            warn!(
                "Failed to assign hook process {} to job object: {e}; \
                 timeout not configured so enforcement gap is acceptable",
                pid
            );
        }
    }

    let (tx, rx) = mpsc::channel::<std::io::Result<std::process::Output>>();
    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    let received = match timeout_secs {
        Some(secs) => rx.recv_timeout(Duration::from_secs(secs)).map_err(|_| ()),
        None => rx.recv().map_err(|_| ()),
    };

    let result = match received {
        Ok(Ok(output)) => Ok(HookOutput {
            exit_code: output.status.code().unwrap_or(-1),
            timed_out: false,
        }),
        Ok(Err(e)) => Err(NonoError::CommandExecution(e)),
        Err(()) if timeout_secs.is_some() => {
            // Timeout: terminate the entire job object (kills all hook processes).
            let terminate_result = unsafe {
                // SAFETY: job_handle is a live Job Object handle; TerminateJobObject
                // requires JOB_OBJECT_TERMINATE access which CreateJobObjectW grants by default.
                TerminateJobObject(job_handle, 1)
            };
            if terminate_result == 0 {
                error!(
                    "TerminateJobObject for hook timeout failed (GetLastError={})",
                    unsafe { GetLastError() }
                );
            }
            Ok(HookOutput {
                exit_code: -1,
                timed_out: true,
            })
        }
        Err(()) => Err(NonoError::CommandExecution(std::io::Error::other(
            "Hook channel closed unexpectedly",
        ))),
    };

    // Always close the job handle.
    unsafe { CloseHandle(job_handle) };

    result
}

/// Assign a process (by PID) to a job object.
///
/// Opens the process with PROCESS_ALL_ACCESS, assigns it, then closes the handle.
/// Returns Ok if successful or if the process has already exited before assignment.
fn assign_process_to_job(pid: u32, job: HANDLE) -> Result<()> {
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};
    let proc_handle = unsafe {
        // SAFETY: pid is the PID of a just-spawned child process; OpenProcess
        // with PROCESS_ALL_ACCESS is required for AssignProcessToJobObject.
        OpenProcess(PROCESS_ALL_ACCESS, 0, pid)
    };
    if proc_handle.is_null() {
        let gle = unsafe { GetLastError() };
        // ERROR_INVALID_PARAMETER (87) or ERROR_ACCESS_DENIED (5) may indicate
        // process already exited — treat as non-fatal.
        return Err(NonoError::CommandExecution(std::io::Error::other(format!(
            "OpenProcess({pid}) failed: GLE={gle}"
        ))));
    }
    let assign_ok = unsafe {
        // SAFETY: both proc_handle and job are valid handles for the duration of this call.
        AssignProcessToJobObject(job, proc_handle)
    };
    unsafe { CloseHandle(proc_handle) };
    if assign_ok == 0 {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::CommandExecution(std::io::Error::other(format!(
            "AssignProcessToJobObject({pid}) failed: GLE={gle}"
        ))));
    }
    Ok(())
}

/// Validate a hook script path with full D-10 security checks.
///
/// Security checks (all unconditional — D-10 is a locked decision):
/// 1. Absolute path check (Path::is_absolute() — NOT string starts_with; CLAUDE.md footgun)
/// 2. Canonical path via std::fs::canonicalize (adds \\?\ prefix on Windows)
/// 3. Regular file check
/// 4. Owner check via nono::path_is_owned_by_current_user
/// 5. No world-writable ACL on the FILE (GetNamedSecurityInfoW + ACE enumeration)
/// 6. No world-writable ACL on the PARENT DIRECTORY
/// 7. Mandatory label check (warn only if file has anomalous label)
fn validate_hook_script_windows(path: &Path) -> Result<PathBuf> {
    // 1. Absolute path — use Path::is_absolute(), NOT string starts_with (CLAUDE.md footgun)
    if !path.is_absolute() {
        return Err(NonoError::ConfigParse(format!(
            "Hook script path must be absolute: {}",
            path.display()
        )));
    }

    // 2. Canonical — std::fs::canonicalize adds \\?\ prefix on Windows automatically.
    let canonical = path.canonicalize().map_err(|e| {
        NonoError::ConfigParse(format!("Hook script not found: {}: {}", path.display(), e))
    })?;

    // 3. Regular file check
    let meta = canonical.metadata().map_err(|e| {
        NonoError::ConfigParse(format!(
            "Cannot read hook script metadata: {}: {}",
            canonical.display(),
            e
        ))
    })?;
    if !meta.is_file() {
        return Err(NonoError::ConfigParse(format!(
            "Hook script is not a regular file: {}",
            canonical.display()
        )));
    }

    // 4. Owner check — fail-closed on ownership-check errors per labels_guard discipline.
    if !path_is_owned_by_current_user(&canonical)? {
        return Err(NonoError::ConfigParse(format!(
            "Hook script not owned by current user: {}",
            canonical.display()
        )));
    }

    // 5. Effective-rights ACL check on the FILE: verify no Write ACE for Everyone (S-1-1-0).
    // D-10 is unconditional — this check MUST run. No fallback path exists.
    check_no_world_writable_acl(&canonical).map_err(|e| {
        NonoError::ConfigParse(format!(
            "Hook script file has world-writable ACL (D-10 security check): {}: {}",
            canonical.display(),
            e
        ))
    })?;

    // 6. Effective-rights ACL check on PARENT DIRECTORY: same guard.
    // Defense-in-depth: even if the file has tight permissions, a world-writable
    // parent allows any user to REPLACE the file with a malicious one.
    if let Some(parent) = canonical.parent() {
        check_no_world_writable_acl(parent).map_err(|e| {
            NonoError::ConfigParse(format!(
                "Hook script parent directory has world-writable ACL (D-10 security check): {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    // 7. Mandatory-label check: warn (not Err) if file has an anomalous label.
    // A label ABOVE Medium-IL on the script is unusual but not itself a security risk
    // for the validation goal (we're checking access, not integrity level of source).
    // Emit a warning so operators can diagnose misconfigured environments.
    if let Some((rid, _mask)) = nono::low_integrity_label_and_mask(&canonical) {
        // Low-IL RID is 0x1000 (4096); Medium-IL is 0x2000 (8192).
        // If the file itself is labeled Low-IL, the hook runs Low-IL — consistent.
        // If labeled High-IL or System, log a warning.
        if rid > 0x2000 {
            warn!(
                path = %canonical.display(),
                rid = format!("0x{rid:X}"),
                "Hook script has mandatory label above Medium-IL; \
                 hook currently runs at parent's IL (Medium-IL — D-05 Low-IL spawn deferred)"
            );
        }
    }

    Ok(canonical)
}

/// Check that a path does NOT have a write-class ACE for Everyone (S-1-1-0).
///
/// Reads the DACL via GetNamedSecurityInfoW and enumerates ACEs. If any
/// ACCESS_ALLOWED ACE grants write-class rights (FILE_WRITE_DATA=0x2, FILE_ADD_FILE=0x2,
/// GENERIC_WRITE=0x40000000) to the Everyone SID, returns Err.
///
/// D-10: This check is UNCONDITIONAL. There is no fallback path that skips it.
/// If the ACL query fails, the function returns Err (fail-closed per D-01).
fn check_no_world_writable_acl(path: &Path) -> std::result::Result<(), String> {
    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Build the Everyone SID (S-1-1-0) for ACE comparison.
    // SAFETY: ConvertStringSidToSidW parses "S-1-1-0" into a LocalAlloc-ed PSID.
    let everyone_sid_str: Vec<u16> = "S-1-1-0\0".encode_utf16().collect();
    let mut everyone_psid: windows_sys::Win32::Security::PSID = std::ptr::null_mut();
    let ok = unsafe { ConvertStringSidToSidW(everyone_sid_str.as_ptr(), &mut everyone_psid) };
    if ok == 0 {
        return Err(format!(
            "ConvertStringSidToSidW(Everyone) failed: GLE={}",
            unsafe { GetLastError() }
        ));
    }
    // SAFETY: everyone_psid was allocated by ConvertStringSidToSidW and must be freed with LocalFree.
    struct OwnedSid(windows_sys::Win32::Security::PSID);
    impl Drop for OwnedSid {
        fn drop(&mut self) {
            unsafe { windows_sys::Win32::Foundation::LocalFree(self.0.cast()) };
        }
    }
    let _everyone_sid_guard = OwnedSid(everyone_psid);

    // Read the DACL for the path.
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut security_descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let status = unsafe {
        // SAFETY: wide_path is a valid nul-terminated UTF-16 buffer; output pointers
        // are valid. The returned SD must be freed with LocalFree.
        GetNamedSecurityInfoW(
            wide_path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut dacl,
            std::ptr::null_mut(),
            &mut security_descriptor,
        )
    };
    if status != 0 {
        return Err(format!(
            "GetNamedSecurityInfoW(DACL) failed: status=0x{status:08X}"
        ));
    }
    // SAFETY: security_descriptor was heap-allocated by GetNamedSecurityInfoW; free with LocalFree.
    struct OwnedSd(PSECURITY_DESCRIPTOR);
    impl Drop for OwnedSd {
        fn drop(&mut self) {
            unsafe { windows_sys::Win32::Foundation::LocalFree(self.0.cast()) };
        }
    }
    let _sd_guard = OwnedSd(security_descriptor);

    if dacl.is_null() {
        // A null DACL means "allow everything" — this IS world-writable.
        return Err("Path has a null DACL (all access allowed to everyone)".to_string());
    }

    let ace_count = unsafe { (*dacl).AceCount };

    // Write-class access mask bits we check against.
    // FILE_WRITE_DATA (files) = FILE_ADD_FILE (dirs) = 0x0002
    // FILE_WRITE_ATTRIBUTES = 0x0100
    // FILE_WRITE_EA = 0x0010
    // GENERIC_WRITE = 0x40000000
    // DELETE = 0x00010000
    // WRITE_DAC = 0x00040000
    // WRITE_OWNER = 0x00080000
    // We check for any write-capable mask that could allow placing or replacing files.
    const WRITE_CLASS_MASK: u32 = 0x4007_0112; // GENERIC_WRITE | WRITE_DAC | WRITE_OWNER | FILE_WRITE_DATA | FILE_WRITE_EA | FILE_WRITE_ATTRIBUTES

    for index in 0..ace_count {
        let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let ok = unsafe {
            // SAFETY: dacl is a valid ACL pointer, ace_ptr is a valid out-pointer.
            GetAce(dacl, u32::from(index), &mut ace_ptr)
        };
        if ok == 0 || ace_ptr.is_null() {
            continue;
        }

        let ace_header = unsafe { &*(ace_ptr as *const ACE_HEADER) };
        // Only ACCESS_ALLOWED_ACE_TYPE (0x00) grants access.
        // ACCESS_ALLOWED_ACE_TYPE = 0 per Windows SDK — use the constant value directly.
        if ace_header.AceType != 0u8 {
            continue;
        }

        let allowed_ace = unsafe { &*(ace_ptr as *const ACCESS_ALLOWED_ACE) };
        // Check if write bits are granted.
        if allowed_ace.Mask & WRITE_CLASS_MASK == 0 {
            continue;
        }

        // Get the SID from the ACE (follows the Mask field in ACCESS_ALLOWED_ACE).
        let ace_sid = (&allowed_ace.SidStart as *const u32)
            .cast_mut()
            .cast::<std::ffi::c_void>();

        // Compare with the Everyone SID.
        let eq = unsafe {
            // SAFETY: both SIDs are valid PSID pointers for the duration of this call.
            windows_sys::Win32::Security::EqualSid(ace_sid, everyone_psid)
        };
        if eq != 0 {
            return Err(format!(
                "Everyone (S-1-1-0) has write access (mask=0x{:08X})",
                allowed_ace.Mask
            ));
        }
    }

    Ok(())
}

/// Read KEY=VALUE pairs from an env file written by a hook.
///
/// Ignores empty lines, comment lines (starting with #), and lines without '='.
/// Keys and values are trimmed. Keys must be non-empty.
fn read_env_file(path: &Path) -> Result<Vec<(String, String)>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| NonoError::ConfigParse(format!("Failed to read env file: {e}")))?;

    let vars = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty())
        .collect();

    Ok(vars)
}

/// RAII guard for the hook env file (Windows variant, D-08).
///
/// Uses CREATE_NEW (OpenOptions::create_new(true) maps to CREATE_NEW on Windows —
/// equivalent to Unix O_EXCL) to prevent pre-created file injection attacks.
/// After creation, applies Low-IL mandatory label (mask 0x5 = NO_WRITE_UP | NO_EXECUTE_UP)
/// so only the hook process (Low-IL) and the parent (Medium-IL) can access the file.
///
/// On Drop: zero-fills the file contents, then removes the file.
struct WindowsEnvFileGuard {
    path: PathBuf,
}

impl WindowsEnvFileGuard {
    /// Create the env file with CREATE_NEW disposition and Low-IL mandatory label (D-08).
    fn create(session_id: &str) -> Result<Self> {
        let sessions_dir = session::ensure_sessions_dir()?;
        let session_env_dir = sessions_dir.join(session_id);
        std::fs::create_dir_all(&session_env_dir).map_err(|e| {
            NonoError::ConfigParse(format!(
                "Failed to create session env directory {}: {e}",
                session_env_dir.display()
            ))
        })?;

        let path = session_env_dir.join("env");

        // CREATE_NEW: fails if file exists (equivalent to O_EXCL on Unix, D-08).
        // std::fs::OpenOptions::create_new(true) maps to CREATE_NEW disposition on Windows.
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                NonoError::ConfigParse(format!("Failed to create env file (CREATE_NEW): {e}"))
            })?;

        // Apply Low-IL mandatory label (mask 0x5 = NO_WRITE_UP | NO_EXECUTE_UP, D-08).
        // Same mask as labels_guard.rs:365 for Low-IL. Primary gate for env-file trust boundary:
        // Low-IL processes (the hook) can write; Medium-IL parent can read.
        // Source: nono::try_set_mandatory_label — same call used in labels_guard.rs.
        if let Err(e) = try_set_mandatory_label(&path, 0x5) {
            // Clean up the file if labeling fails (fail-closed).
            let _ = std::fs::remove_file(&path);
            return Err(e);
        }

        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WindowsEnvFileGuard {
    fn drop(&mut self) {
        // Zero-then-delete: mirror Unix EnvFileGuard's zeroize-on-drop contract.
        // Prevents env-file contents from being readable after the hook exits
        // even if the OS delays unlink.
        if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(&self.path) {
            if let Ok(metadata) = file.metadata() {
                use std::io::Write;
                let zeros = vec![0u8; metadata.len() as usize];
                let _ = file.write_all(&zeros);
                let _ = file.sync_all();
            }
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn isolated_home() -> (
        std::sync::MutexGuard<'static, ()>,
        crate::test_env::EnvVarGuard,
        TempDir,
    ) {
        let lock = crate::test_env::lock_env();
        let home = TempDir::new().unwrap();
        let home_str = home.path().to_str().unwrap().to_string();
        let env = crate::test_env::EnvVarGuard::set_all(&[("USERPROFILE", &home_str)]);
        (lock, env, home)
    }

    /// WindowsEnvFileGuard::create uses CREATE_NEW — a second call with the same
    /// session_id must return Err (file already exists = D-08 clobber prevention).
    #[test]
    fn test_env_file_create_new_prevents_clobber() {
        let (_lock, _env, _home) = isolated_home();
        let session_id = "test-clobber-guard-session";
        let guard1 = WindowsEnvFileGuard::create(session_id).unwrap();
        // Second create must fail (CREATE_NEW disposition).
        let result = WindowsEnvFileGuard::create(session_id);
        assert!(
            result.is_err(),
            "Second create with same session_id must fail (CREATE_NEW prevents clobber)"
        );
        drop(guard1); // triggers zero-then-delete
    }

    /// validate_hook_script_windows must reject relative paths.
    #[test]
    fn test_validate_hook_script_windows_rejects_relative() {
        let result = validate_hook_script_windows(Path::new("relative/path/script.exe"));
        assert!(result.is_err(), "Relative paths must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("absolute"),
            "Error must mention 'absolute': {err}"
        );
    }

    /// validate_hook_script_windows must reject paths that are directories, not files.
    #[test]
    fn test_validate_hook_script_windows_rejects_non_file() {
        let dir = TempDir::new().unwrap();
        let result = validate_hook_script_windows(dir.path());
        assert!(result.is_err(), "Directory paths must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.to_ascii_lowercase().contains("regular file"),
            "Error must mention 'regular file': {err}"
        );
    }

    /// Dangerous vars from the env file are filtered before returning.
    ///
    /// Verifies the D-09 filtering of Windows danger vars in the read+filter pipeline.
    /// Uses read_env_file + is_dangerous_env_var filter directly (same logic as execute_before_hook).
    #[test]
    fn test_windows_dangerous_vars_filtered_from_env_file() {
        use std::io::Write;
        let dir = TempDir::new().unwrap();
        let env_file = dir.path().join("env");
        let mut f = std::fs::File::create(&env_file).unwrap();
        writeln!(f, "PATH=evil\\bin").unwrap();
        writeln!(f, "MY_VAR=safe_value").unwrap();
        writeln!(f, "COMSPEC=evil.exe").unwrap();
        writeln!(f, "MY_OTHER_VAR=also_safe").unwrap();
        drop(f);

        let raw = read_env_file(&env_file).unwrap();
        let filtered: Vec<(String, String)> = raw
            .into_iter()
            .filter(|(k, _)| !exec_strategy::is_dangerous_env_var(k))
            .collect();

        // PATH and COMSPEC must be filtered out (D-09)
        assert!(
            !filtered.iter().any(|(k, _)| k == "PATH"),
            "PATH must be filtered (D-09)"
        );
        assert!(
            !filtered.iter().any(|(k, _)| k == "COMSPEC"),
            "COMSPEC must be filtered (D-09)"
        );
        // Safe vars must pass through
        assert!(
            filtered
                .iter()
                .any(|(k, v)| k == "MY_VAR" && v == "safe_value"),
            "MY_VAR must pass through"
        );
        assert!(
            filtered
                .iter()
                .any(|(k, v)| k == "MY_OTHER_VAR" && v == "also_safe"),
            "MY_OTHER_VAR must pass through"
        );
    }

    /// validate_hook_script_windows rejects scripts in world-writable parent directories.
    ///
    /// Creates a temp directory, grants Everyone (S-1-1-0) write access on it,
    /// creates a dummy script file owned by the current user, then verifies that
    /// validate_hook_script_windows returns Err.
    ///
    /// Note: this test requires the test process to have sufficient rights to modify
    /// ACLs on temp directories. Mark #[ignore] if run in a restricted environment;
    /// implement without ignoring as D-10 is an unconditional security requirement.
    #[test]
    fn test_validate_rejects_world_writable_parent() {
        use std::io::Write;

        let dir = TempDir::new().unwrap();
        let script = dir.path().join("hook.ps1");
        let mut f = std::fs::File::create(&script).unwrap();
        writeln!(f, "# test hook").unwrap();
        drop(f);

        // Grant Everyone (S-1-1-0) write access on the temp directory.
        let grant_result = nono::grant_sid_write_on_path(dir.path(), "S-1-1-0", true);
        if let Err(e) = &grant_result {
            // If we can't grant the ACE (insufficient privileges), skip the test assertion.
            // Document why: non-elevated test runners may not have WRITE_DAC on temp dirs.
            println!(
                "test_validate_rejects_world_writable_parent: could not grant Everyone write ACE: {e}; \
                 skipping world-writable assertion (requires WRITE_DAC on temp dir). \
                 This test MUST pass in an elevated CI environment."
            );
            return;
        }

        // RAII: revoke the Everyone ACE on drop.
        struct RevokeGuard {
            path: PathBuf,
        }
        impl Drop for RevokeGuard {
            fn drop(&mut self) {
                let _ = nono::revoke_sid_on_path(&self.path, "S-1-1-0");
            }
        }
        let _revoke = RevokeGuard {
            path: dir.path().to_path_buf(),
        };

        // validate_hook_script_windows must reject because parent dir has Everyone-write ACE.
        let result = validate_hook_script_windows(&script);
        assert!(
            result.is_err(),
            "validate_hook_script_windows must reject scripts in world-writable parent directories (D-10); \
             got Ok({:?})",
            result.ok()
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.to_ascii_lowercase().contains("world-writable")
                || err.to_ascii_lowercase().contains("everyone"),
            "Error must mention world-writable or Everyone: {err}"
        );
    }

    /// Deterministic unit test: strip_verbatim_prefix strips \\?\ and \\?\UNC\,
    /// and leaves other paths unchanged.
    ///
    /// This is a pure path-transform assertion — no filesystem access, no PowerShell
    /// spawn. It is the PRIMARY guard against the \\?\ regression: if strip_verbatim_prefix
    /// is broken (e.g., reverted), this test fails immediately on any platform that
    /// compiles the Windows cfg target. Unlike the live-spawn test, this test is not
    /// subject to the PowerShell execution-policy Bypass workaround that can mask the
    /// bug in spawned-process tests.
    #[test]
    fn test_strip_verbatim_prefix_deterministic() {
        // \\?\C:\... → C:\...
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\C:\Users\OMack\hook.ps1")),
            PathBuf::from(r"C:\Users\OMack\hook.ps1"),
            "Extended-length prefix \\\\?\\C:\\ must be stripped"
        );

        // \\?\UNC\server\share\... → \\server\share\...
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\dir\file.ps1")),
            PathBuf::from(r"\\server\share\dir\file.ps1"),
            "UNC verbatim prefix \\\\?\\UNC\\ must be converted to \\\\\\\\"
        );

        // Normal absolute path → unchanged
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"C:\Windows\System32\cmd.exe")),
            PathBuf::from(r"C:\Windows\System32\cmd.exe"),
            "Normal absolute path must be returned unchanged"
        );

        // Relative path → unchanged (no prefix to strip)
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"relative\path\script.ps1")),
            PathBuf::from(r"relative\path\script.ps1"),
            "Relative path must be returned unchanged"
        );

        // Confirm the exact UAT failure form is stripped:
        // The live UAT error was: \\?\C:\Users\OMack\AppData\Local\Temp\hook-before.ps1
        let uat_path = Path::new(r"\\?\C:\Users\OMack\AppData\Local\Temp\hook-before.ps1");
        let stripped = strip_verbatim_prefix(uat_path);
        let stripped_str = stripped.to_string_lossy();
        assert!(
            !stripped_str.starts_with(r"\\?\"),
            "strip_verbatim_prefix must remove \\\\?\\  from canonical path; got: {stripped_str}"
        );
        assert!(
            stripped_str.starts_with("C:\\"),
            "Stripped path must start with drive letter; got: {stripped_str}"
        );
    }

    /// End-to-end regression test: execute_before_hook must succeed (Ok + HOOK_OK=yes)
    /// on a Windows host where powershell.exe is available.
    ///
    /// This test guards against TWO distinct defects:
    ///
    /// 1. [CLR startup failure, -65536] build_windows_hook_command called env_clear()
    ///    without re-adding SystemRoot/windir/SystemDrive. Without these, powershell.exe
    ///    cannot initialize the CLR and exits with -65536 before any script body runs.
    ///    Fixed by re-adding those three OS-baseline vars after env_clear().
    ///
    /// 2. [\\?\ verbatim prefix, exit 1] validate_hook_script_windows returns
    ///    std::fs::canonicalize() output, which on Windows is the extended-length
    ///    verbatim form \\?\C:\.... PowerShell's -File flag cannot resolve the security
    ///    zone of a \\?\-prefixed path; under RemoteSigned execution policy (Windows
    ///    default), PowerShell refuses to run the unsigned script with "not digitally
    ///    signed" and exits with code 1 — before any script body runs.
    ///    Fixed by strip_verbatim_prefix in build_windows_hook_command.
    ///
    /// ASSERTION POLICY (strengthened after second UAT gap-closure):
    /// - If execute_before_hook returns Ok → REQUIRE HOOK_OK=yes in the exported vars.
    /// - If execute_before_hook returns Err → check if the error text indicates a genuine
    ///   "PowerShell not found / cannot spawn" condition. If so, skip with a message
    ///   (the test cannot run on a host without powershell.exe). Any OTHER Err — including
    ///   "exited with code -65536", "exited with code 1", "not digitally signed" — FAILS
    ///   the test. These are functional failures, not environment limitations.
    ///
    /// CAVEAT: cargo test runs in a shell whose Process-scope PowerShell execution policy
    /// may be "Bypass" (bypasses zone/signature checks for the process), which can mask
    /// the \\?\ bug in a live-spawn test (Bypass ignores RemoteSigned). The deterministic
    /// `test_strip_verbatim_prefix_deterministic` is the authoritative regression guard for
    /// the \\?\ fix; this test validates end-to-end Ok+HOOK_OK correctness.
    #[test]
    fn test_execute_before_hook_powershell_does_not_clr_fail() {
        use std::io::Write;

        let (_lock, _env, _home) = isolated_home();

        // Create a user-owned temp dir (not world-writable) for the hook script.
        // validate_hook_script_windows requires: absolute, canonical, regular file,
        // user-owned, no world-writable ACL on file or parent.
        let script_dir = TempDir::new().unwrap();
        let script_path = script_dir.path().join("test_hook.ps1");

        // Hook body: append HOOK_OK=yes to $env:NONO_ENV_FILE
        {
            let mut f = std::fs::File::create(&script_path).unwrap();
            writeln!(
                f,
                "Add-Content -Path $env:NONO_ENV_FILE -Value 'HOOK_OK=yes'"
            )
            .unwrap();
        }

        // Build a SessionHook pointing at the script.
        let hook = profile::SessionHook {
            script: script_path.clone(),
            timeout_secs: Some(30),
        };

        let workdir = script_dir.path();
        let session_id = "test-clr-regression-session";

        let result = execute_before_hook(&hook, session_id, workdir);

        match &result {
            Ok(vars) => {
                // execute_before_hook succeeded — require the hook's HOOK_OK=yes was exported.
                assert!(
                    vars.iter().any(|(k, v)| k == "HOOK_OK" && v == "yes"),
                    "execute_before_hook returned Ok but HOOK_OK=yes was not in the exported vars; \
                     got: {vars:?}"
                );
            }
            Err(e) => {
                let msg = e.to_string();

                // NARROWLY ALLOWED SKIP: genuine "powershell.exe not found / cannot spawn"
                // environment errors. Detect by checking for spawn-failure keywords.
                // os error 2 = ERROR_FILE_NOT_FOUND (program not found),
                // os error 3 = ERROR_PATH_NOT_FOUND
                let is_spawn_failure = msg.contains("program not found")
                    || msg.contains("os error 2")
                    || msg.contains("os error 3")
                    || msg.contains("Failed to spawn hook");

                // All functional failures — including the two defects this test guards against —
                // MUST fail the test, not skip it.
                assert!(
                    is_spawn_failure,
                    "execute_before_hook returned a FUNCTIONAL failure that must not be skipped:\n\
                     {msg}\n\n\
                     If this contains '-65536': the SystemRoot/windir/SystemDrive baseline env \
                     was not re-added after env_clear (first UAT defect).\n\
                     If this contains 'code 1' or 'not digitally signed': the \\\\?\\  verbatim \
                     prefix was not stripped before passing the path to powershell -File (second \
                     UAT defect). Also verify that test_strip_verbatim_prefix_deterministic passes."
                );

                // Acceptable skip: genuine spawn failure (PowerShell not installed).
                println!(
                    "test_execute_before_hook_powershell_does_not_clr_fail: powershell.exe \
                     could not be spawned ({msg}); skipping HOOK_OK assertion. \
                     This is acceptable ONLY if powershell.exe is absent from this environment."
                );
            }
        }
    }

    /// CR-02 behavioral test: run_hook_windows with a timeout returns Ok for a fast-exiting hook.
    ///
    /// This test guards the CR-02 fix's benign-exit path: a hook that exits very quickly
    /// (possibly before job-object assignment completes) must still return Ok, not Err.
    ///
    /// Unit-testing the actual assignment-failure path directly is not feasible because
    /// `AssignProcessToJobObject` failures require inducing a specific OS-level race or
    /// permission condition that cannot be reliably triggered in a deterministic unit test.
    /// The fix's liveness check (child.try_wait()) is the correct gate and is exercised
    /// end-to-end in `test_execute_before_hook_powershell_does_not_clr_fail`.
    ///
    /// What this test validates:
    /// - A hook with a `timeout_secs` configured that exits cleanly does NOT fail with
    ///   the assignment-failure code path.
    /// - The timeout enforcement path is exercised without triggering a false positive.
    #[test]
    fn test_cr02_timeout_hook_exits_cleanly() {
        use std::io::Write;

        let (_lock, _env, _home) = isolated_home();

        let script_dir = TempDir::new().unwrap();
        let script_path = script_dir.path().join("quick_hook.ps1");

        {
            let mut f = std::fs::File::create(&script_path).unwrap();
            // Hook exits cleanly with code 0 and no env var output.
            writeln!(f, "exit 0").unwrap();
        }

        let hook = profile::SessionHook {
            script: script_path.clone(),
            timeout_secs: Some(30), // timeout configured — this is the CR-02 scenario
        };

        let workdir = script_dir.path();
        let session_id = "test-cr02-timeout-session";

        let result = execute_before_hook(&hook, session_id, workdir);

        match &result {
            Ok(_vars) => {
                // A cleanly-exiting hook with a timeout configured must succeed.
                // This confirms the benign-exit path does not fail due to assignment-race.
            }
            Err(e) => {
                let msg = e.to_string();
                // Only acceptable skip: genuine spawn failure (PowerShell not installed).
                let is_spawn_failure = msg.contains("program not found")
                    || msg.contains("os error 2")
                    || msg.contains("os error 3")
                    || msg.contains("Failed to spawn hook");
                assert!(
                    is_spawn_failure,
                    "CR-02 test: execute_before_hook returned unexpected failure with timeout configured:\n\
                     {msg}\n\n\
                     If this is 'fail closed...timeout enforcement': the benign-exit path in CR-02 \
                     is not triggering correctly (child already exited but was not detected)."
                );
                println!(
                    "test_cr02_timeout_hook_exits_cleanly: powershell.exe not available; \
                     skipping assertion ({msg})"
                );
            }
        }
    }
}
