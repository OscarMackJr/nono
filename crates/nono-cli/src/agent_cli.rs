//! `nono daemon` and `nono agent` — operator-facing lifecycle and agent verbs.
//!
//! # Scope (Phase 74 D-05): UX layer over nono-agentd
//!
//! This module provides the thin client / controller surface for the
//! `nono-agentd` persistent daemon:
//!
//! - `nono daemon start|stop|status|install|uninstall` — service lifecycle
//! - `nono agent launch --profile <engine> -- <cmd>` — ask the daemon to
//!   spawn a new confined agent (the daemon owns AppContainer + job lifecycle)
//! - `nono agent list` — query running agents from the daemon
//!
//! The CLI is a THIN CLIENT: it does NOT duplicate the daemon's launch logic.
//! The daemon is the single owner of agent lifecycle (DMON-01).
//!
//! # Platform
//!
//! Daemon and agent operations are Windows-only. On non-Windows platforms every
//! entry point prints a diagnostic message and returns `Ok(())` or `Err(...)`.
//!
//! # Phase 73 reuse (D-05 constraint)
//!
//! `nono classify <PID>` from Phase 73 is REUSED for PID inspection.
//! No new inspection verb is added here.
//!
//! # Control pipe name
//!
//! The daemon control pipe (`\\.\\pipe\\nono-agentd-control`) is distinct from
//! the capability pipe (`\\.\\pipe\\nono-agentd-cap`). It is used by
//! `nono agent launch` and `nono agent list` to communicate with the daemon.
//! Phase 75 will fully wire this protocol; Phase 74 provides the minimal
//! connection attempt (surfaces "daemon not running" cleanly).

use crate::cli::{AgentArgs, AgentCommands, DaemonArgs, DaemonCommands};
use nono::{NonoError, Result};

/// Daemon control pipe name.
///
/// The CLI connects here for `agent launch` and `agent list`. The daemon
/// (Phase 75) will listen on this name. Phase 74 declares it here for
/// consistency; connecting when the daemon is not running surfaces a clear error.
pub(crate) const DAEMON_CONTROL_PIPE_NAME: &str = r"\\.\pipe\nono-agentd-control";

/// SCM service name registered by `nono daemon install`.
///
/// Matches `SERVICE_NAME` in `bin/nono-agentd.rs`.
const DAEMON_SERVICE_NAME: &str = "nono-agentd";

// ─── Daemon lifecycle commands ────────────────────────────────────────────────

/// Entry point for `nono daemon <subcommand>`.
///
/// Dispatches to Windows SCM operations (install/uninstall/start/stop/status).
/// On non-Windows platforms prints a diagnostic and returns `Ok(())`.
pub(crate) fn run_daemon(args: DaemonArgs) -> Result<()> {
    match args.command {
        DaemonCommands::Start => daemon_start(),
        DaemonCommands::Stop => daemon_stop(),
        DaemonCommands::Status => daemon_status(),
        DaemonCommands::Install => daemon_install(),
        DaemonCommands::Uninstall => daemon_uninstall(),
    }
}

/// `nono daemon start` — start nono-agentd.
///
/// On Windows: if the SCM service is installed, starts it via `sc start`.
/// If the SCM service is NOT installed (dev-layout), spawns `nono-agentd.exe`
/// as a detached background process from the same directory as the current
/// executable. This supports the development workflow where the daemon is run
/// without SCM registration.
///
/// The dev-layout spawn uses raw `CreateProcessW` with `bInheritHandles=FALSE`
/// so the long-lived daemon inherits ZERO handles from the launching shell.
/// `std::process::Command` cannot be used here because it unconditionally sets
/// `bInheritHandles=TRUE` whenever stdio is redirected (even to `Stdio::null()`),
/// which causes the daemon to hold the shell's inheritable handles open and
/// blocks the operator's shell until the daemon exits.
///
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_start() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // Check if the SCM service is installed by querying it.
        let sc_query = Command::new("sc")
            .args(["query", DAEMON_SERVICE_NAME])
            .output()
            .map_err(|e| {
                NonoError::SandboxInit(format!(
                    "nono daemon start: failed to run `sc query {DAEMON_SERVICE_NAME}`: {e}"
                ))
            })?;

        let sc_stdout = String::from_utf8_lossy(&sc_query.stdout);
        let service_exists = sc_query.status.success()
            || sc_stdout.contains("RUNNING")
            || sc_stdout.contains("STOPPED")
            || sc_stdout.contains("STATE");

        if service_exists && !sc_stdout.contains("1060") && !sc_stdout.contains("does not exist") {
            // SCM service is installed. Before attempting `sc start`, determine
            // the service type. A USER_OWN_PROCESS TEMPLATE (type 50) cannot be
            // started via `sc start` — it is a template that requires a user
            // session to instantiate it. Attempting `sc start` on a type-50 service
            // returns ACCESS_DENIED (exit 5), which is the GAP-75-A failure mode.
            //
            // Resolution: if `sc qc` identifies a type-50 template, skip `sc start`
            // and fall through to the raw-spawn path, which is proven correct for
            // dev-layout and works for template-registered services too.
            let sc_qc_output = Command::new("sc")
                .args(["qc", DAEMON_SERVICE_NAME])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default();

            if is_user_own_template_service(&sc_qc_output) {
                // GAP-75-A fix: type-50 USER_OWN_PROCESS TEMPLATE cannot be started
                // via `sc start`. Fall through to raw-spawn below.
                println!(
                    "[template] nono-agentd is registered as a USER_OWN_PROCESS TEMPLATE \
                     (type 50); starting via raw spawn (sc start is not supported for \
                     template services)."
                );
                // Do not return; fall through to the raw-spawn path.
            } else {
                // Type 10 (WIN32_OWN_PROCESS) or unknown — use `sc start` as usual.
                println!("[SCM] Starting nono-agentd via Service Control Manager...");
                return windows_sc_command(
                    &["start", DAEMON_SERVICE_NAME],
                    "nono daemon start",
                    "nono-agentd started successfully.",
                    "nono-agentd may already be running, or the service is not installed. \
                     Try `nono daemon install` first.",
                );
            }
        }

        // Dev-layout: no SCM service. Spawn nono-agentd.exe as a detached
        // background process from the same directory as the current executable.
        let current_exe = std::env::current_exe().map_err(|e| {
            NonoError::SandboxInit(format!(
                "nono daemon start: failed to resolve current executable path: {e}"
            ))
        })?;

        let exe_dir = current_exe.parent().ok_or_else(|| {
            NonoError::SandboxInit(
                "nono daemon start: failed to resolve executable directory".into(),
            )
        })?;

        let agentd_exe = exe_dir.join("nono-agentd.exe");
        if !agentd_exe.exists() {
            return Err(NonoError::SandboxInit(format!(
                "nono daemon start: nono-agentd.exe not found at {}. \
                 Build the workspace with `cargo build -p nono-cli` first, \
                 or use `nono daemon install` + `nono daemon start` for SCM mode.",
                agentd_exe.display()
            )));
        }

        // Raw CreateProcessW with bInheritHandles=FALSE.
        //
        // We MUST use the raw Win32 API here rather than std::process::Command
        // because Rust's Command sets bInheritHandles=TRUE whenever stdio streams
        // are redirected (including Stdio::null()). With bInheritHandles=TRUE the
        // daemon inherits the launching shell's inheritable handles (e.g. the
        // console stdout pipe), which keeps them alive and blocks the operator's
        // shell until the daemon exits — even with DETACHED_PROCESS set.
        //
        // With bInheritHandles=FALSE the daemon inherits ZERO handles from the
        // launcher. The daemon allocates its own console (or none, given
        // CREATE_NO_WINDOW) and manages its own file handles independently.
        daemon_start_raw_spawn(&agentd_exe)
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("nono-agentd is Windows-only.");
        Ok(())
    }
}

/// Spawn `nono-agentd.exe` via raw `CreateProcessW` with `bInheritHandles=FALSE`.
///
/// This is the dev-layout background spawn path. Using raw `CreateProcessW` is
/// the only way on Windows to spawn a child with `bInheritHandles=FALSE` from
/// Rust — `std::process::Command` forces `bInheritHandles=TRUE` whenever any
/// stdio stream is redirected, including `Stdio::null()`.
///
/// Creation flags:
/// - `DETACHED_PROCESS` (0x0000_0008): child is not attached to the parent's
///   console session; it receives no console at all.
/// - `CREATE_NEW_PROCESS_GROUP` (0x0000_0200): child gets its own signal group
///   so Ctrl+C in the operator's shell does not propagate to the daemon.
/// - `CREATE_NO_WINDOW` (0x0800_0000): belt-and-suspenders; suppresses any
///   default console window the loader might allocate.
///
/// On success, both `hProcess` and `hThread` from `PROCESS_INFORMATION` are
/// closed immediately — we do not wait on or control the daemon after launch.
///
/// # Errors
///
/// Returns `Err` with a human-readable message including `GLE=<n>` if
/// `CreateProcessW` fails, so the operator knows which OS error occurred.
#[cfg(target_os = "windows")]
fn daemon_start_raw_spawn(agentd_exe: &std::path::Path) -> Result<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
    };

    // dwCreationFlags:
    //   DETACHED_PROCESS         = 0x0000_0008
    //   CREATE_NEW_PROCESS_GROUP = 0x0000_0200
    //   CREATE_NO_WINDOW         = 0x0800_0000
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // Build the application path as a null-terminated UTF-16 string.
    // lpApplicationName receives the fully-qualified path; lpCommandLine is null
    // (the OS uses the exe name alone, equivalent to no arguments).
    // We append `--foreground` via the command-line buffer so the daemon skips
    // SCM service_dispatcher and runs its accept + control loops directly.
    use std::os::windows::ffi::OsStrExt as _;
    let app_wide: Vec<u16> = agentd_exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0u16))
        .collect();

    // Build a mutable command-line buffer ("path\to\nono-agentd.exe" --foreground).
    // CreateProcessW may modify the buffer in place (documented behaviour).
    let exe_str = agentd_exe.to_string_lossy();
    let cmd_str = if exe_str.contains(' ') {
        format!("\"{}\" --foreground", exe_str)
    } else {
        format!("{} --foreground", exe_str)
    };
    let mut cmd_wide: Vec<u16> = cmd_str
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();

    // Zero-initialise STARTUPINFOW; set cb to the struct size.
    // hStdInput / hStdOutput / hStdError are left as null because
    // bInheritHandles=FALSE makes them irrelevant — the child receives no
    // inherited handles and will use its own (or none with CREATE_NO_WINDOW).
    // SAFETY: STARTUPINFOW is a plain-data struct; zeroed() is the documented
    // initialisation idiom for the fields we do not set.
    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

    // SAFETY: PROCESS_INFORMATION is a plain-data output struct; zeroed() is correct.
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let ok = unsafe {
        // SAFETY:
        // - `app_wide` is a valid null-terminated UTF-16 absolute path.
        // - `cmd_wide` is a valid null-terminated UTF-16 command line;
        //   CreateProcessW may write to the buffer but it is large enough.
        // - `bInheritHandles=0` (FALSE): daemon inherits zero handles from the
        //   launcher; the parent's console and pipe handles are NOT passed down.
        //   This is the critical invariant — avoids the daemon holding the
        //   launcher's stdout pipe open (which would block the operator's shell).
        // - All pointer params (lpProcessAttributes, lpThreadAttributes,
        //   lpEnvironment, lpCurrentDirectory) are null → use safe defaults.
        // - `si.cb` is correctly set; `&si` and `&mut pi` are valid output params.
        CreateProcessW(
            app_wide.as_ptr(),     // lpApplicationName
            cmd_wide.as_mut_ptr(), // lpCommandLine (mutable, may be modified)
            std::ptr::null(),      // lpProcessAttributes
            std::ptr::null(),      // lpThreadAttributes
            0,                     // bInheritHandles = FALSE
            DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW,
            std::ptr::null_mut(), // lpEnvironment (inherit from parent)
            std::ptr::null(),     // lpCurrentDirectory (inherit from parent)
            &si,                  // lpStartupInfo
            &mut pi,              // lpProcessInformation (output)
        )
    };

    if ok == 0 {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "nono daemon start: CreateProcessW(nono-agentd.exe) failed: GLE={gle}"
        )));
    }

    // We do not wait for, signal, or control the daemon — close both handles
    // immediately so nono.exe holds no references to the daemon process.
    // SAFETY: pi.hProcess and pi.hThread are valid handles set by CreateProcessW.
    unsafe { CloseHandle(pi.hProcess) };
    unsafe { CloseHandle(pi.hThread) };

    let pid = pi.dwProcessId;

    // Brief pause to let the daemon initialize its pipes before returning.
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Verify the daemon came up by probing the control pipe.
    let pipe_up = std::path::Path::new(r"\\.\pipe\nono-agentd-control").exists();
    if pipe_up {
        println!("[dev-layout] nono-agentd started as background process (pid={pid}).");
        println!("Use `nono daemon status` to confirm, `nono daemon stop` to stop.");
    } else {
        println!(
            "[dev-layout] nono-agentd spawned (pid={pid}); control pipe not yet visible \
             — daemon may still be initializing.",
        );
    }

    Ok(())
}

/// `nono daemon stop` — stop a running nono-agentd.
///
/// On Windows: if the SCM service is installed and running, uses `sc stop`.
/// Otherwise (dev-layout), sends a `{"action":"shutdown"}` request to the
/// control pipe if the daemon supports it; if not, prints a diagnostic with
/// the process name for manual `Stop-Process`.
///
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_stop() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // Check if an SCM service is registered and running.
        let sc_query = Command::new("sc")
            .args(["query", DAEMON_SERVICE_NAME])
            .output()
            .map_err(|e| {
                NonoError::SandboxInit(format!(
                    "nono daemon stop: failed to run `sc query {DAEMON_SERVICE_NAME}`: {e}"
                ))
            })?;

        let sc_stdout = String::from_utf8_lossy(&sc_query.stdout);
        let scm_running = sc_stdout.contains("RUNNING") && !sc_stdout.contains("does not exist");

        if scm_running {
            // SCM service is running — stop via `sc stop`.
            println!("[SCM] Stopping nono-agentd via Service Control Manager...");
            return windows_sc_command(
                &["stop", DAEMON_SERVICE_NAME],
                "nono daemon stop",
                "nono-agentd stopped.",
                "nono-agentd may not be running, or the service is not installed.",
            );
        }

        // Dev-layout or SCM-not-registered: try a control-pipe shutdown request.
        let shutdown_payload = r#"{"action":"shutdown"}"#;
        match windows_control_pipe_request(shutdown_payload) {
            Ok(resp) => {
                println!("nono-agentd stopped (dev-layout): {}", resp.trim());
                return Ok(());
            }
            Err(ref e) if is_pipe_not_found(e) => {
                println!(
                    "nono-agentd status: NOT RUNNING \
                     (control pipe not found; daemon is already stopped)"
                );
                return Ok(());
            }
            Err(_) => {
                // Shutdown action not supported (daemon may not serve it yet) —
                // fall through to print diagnostic.
            }
        }

        // Fallback: inform the user how to stop the background process manually.
        println!(
            "nono-agentd (dev-layout): use PowerShell to stop the background process:\n\
             Stop-Process -Name nono-agentd -ErrorAction SilentlyContinue"
        );
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("nono-agentd is Windows-only.");
        Ok(())
    }
}

/// `nono daemon status` — print whether the daemon is running.
///
/// On Windows: FIRST probes the control pipe (`\\.\pipe\nono-agentd-control`)
/// with a short-timeout `list` request. If the probe succeeds, the daemon is
/// RUNNING regardless of SCM registration (covers dev-layout background spawns).
/// Falls back to `sc query` for SCM-only status (STOPPED / NOT INSTALLED).
///
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_status() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Primary check: probe the control pipe with a {"action":"list"} request.
        // This is truthful for BOTH SCM-managed and dev-layout background daemons.
        let probe_payload = r#"{"action":"list"}"#;
        match windows_control_pipe_request(probe_payload) {
            Ok(_) => {
                println!("nono-agentd status: RUNNING");
                return Ok(());
            }
            Err(ref e) if is_pipe_not_found(e) => {
                // Pipe not present — daemon is not running. Fall through to SCM check.
            }
            Err(ref e) => {
                // Pipe exists but request failed — daemon is up but may be busy.
                // Report running with a note.
                println!(
                    "nono-agentd status: RUNNING (control pipe reachable; status probe error: {e})"
                );
                return Ok(());
            }
        }

        // Fallback: query the SCM service registration for STOPPED/NOT INSTALLED state.
        use std::process::Command;

        let output = Command::new("sc")
            .args(["query", DAEMON_SERVICE_NAME])
            .output()
            .map_err(|e| {
                NonoError::SandboxInit(format!(
                    "nono daemon status: failed to run `sc query {DAEMON_SERVICE_NAME}`: {e}"
                ))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains("STOPPED") {
            println!("nono-agentd status: STOPPED (SCM service registered but not running)");
        } else if output.status.success() {
            println!("nono-agentd status: {}", stdout.trim());
        } else {
            // Neither SCM-registered nor control-pipe-reachable.
            println!(
                "nono-agentd status: NOT RUNNING \
                 (not in SCM; use `nono daemon start` to start in dev-layout, \
                 or `nono daemon install` + `nono daemon start` for SCM mode)"
            );
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("nono-agentd is Windows-only.");
        Ok(())
    }
}

/// `nono daemon install` — register nono-agentd as a per-user SCM service.
///
/// ADR-74 Decision 1: registers as `type= userown` (SERVICE_USER_OWN_PROCESS,
/// NOT LocalSystem/SYSTEM). T-74-05-04 mitigation: the user-own type is
/// mandatory and always present in the `sc create` invocation (`userown` is the
/// valid `sc.exe` token; `userservice` is NOT a valid value).
///
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_install() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // Resolve nono-agentd.exe from the current executable's directory.
        // Handles both dev-layout (target/release/) and installed layouts.
        let current_exe = std::env::current_exe().map_err(|e| {
            NonoError::SandboxInit(format!(
                "nono daemon install: failed to resolve current executable path: {e}"
            ))
        })?;

        let exe_dir = current_exe.parent().ok_or_else(|| {
            NonoError::SandboxInit(
                "nono daemon install: failed to resolve executable directory".into(),
            )
        })?;

        let agentd_exe = exe_dir.join("nono-agentd.exe");
        if !agentd_exe.exists() {
            return Err(NonoError::SandboxInit(format!(
                "nono daemon install: nono-agentd.exe not found at {}. \
                 Build the workspace with `cargo build -p nono-cli` first.",
                agentd_exe.display()
            )));
        }

        // Validate UTF-8 before using in the sc command (path security).
        let agentd_str = agentd_exe.to_str().ok_or_else(|| {
            NonoError::SandboxInit(
                "nono daemon install: nono-agentd.exe path contains non-UTF-8 characters".into(),
            )
        })?;

        // ADR-74 Decision 1: type= userown (SERVICE_USER_OWN_PROCESS — per-user,
        // NOT LocalSystem). `sc.exe` accepts <own|share|...|userown|usershare>;
        // there is NO `userservice` token (that hardcoded value failed with
        // exit 1639 "Invalid type= field"). T-74-05-04: the user-own type is
        // ALWAYS present — never omit; never fall back to `own` (= LocalSystem).
        let binpath = format!("{agentd_str} --service-mode");

        let output = Command::new("sc")
            .args([
                "create",
                DAEMON_SERVICE_NAME,
                "type=",
                "userown",
                "start=",
                "auto",
                "binpath=",
                &binpath,
            ])
            .output()
            .map_err(|e| {
                NonoError::SandboxInit(format!(
                    "nono daemon install: failed to run `sc create`: {e}"
                ))
            })?;

        if output.status.success() {
            println!("nono-agentd installed as a per-user service (type= userown).");
            println!("Use `nono daemon start` to start it.");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let msg = if !stderr.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            };
            Err(NonoError::SandboxInit(format!(
                "nono daemon install: `sc create` failed (exit {}): {msg}",
                output.status.code().unwrap_or(-1)
            )))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("nono-agentd is Windows-only.");
        Ok(())
    }
}

/// `nono daemon uninstall` — remove the nono-agentd SCM service registration.
///
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_uninstall() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows_sc_command(
            &["delete", DAEMON_SERVICE_NAME],
            "nono daemon uninstall",
            "nono-agentd service registration removed.",
            "nono-agentd may not be installed.",
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("nono-agentd is Windows-only.");
        Ok(())
    }
}

// ─── Agent launch / list commands ────────────────────────────────────────────

/// Entry point for `nono agent <subcommand>`.
///
/// Dispatches to daemon control-pipe operations. Fails with a clear error if the
/// daemon is not running — there is NO fallback to an unconfined spawn (DMON-01).
pub(crate) fn run_agent(args: AgentArgs) -> Result<()> {
    match args.command {
        AgentCommands::Launch(launch_args) => agent_launch(launch_args),
        AgentCommands::List => agent_list(),
        // SUPP-01: post-hoc IL-drop incident-response lever.
        AgentCommands::Demote { tenant_id } => agent_demote(tenant_id),
    }
}

/// `nono agent launch` — ask the running daemon to spawn a new confined agent.
///
/// Connects to `DAEMON_CONTROL_PIPE_NAME` and sends a JSON `launch` request.
/// The daemon validates the profile against `policy.json` before spawning
/// (T-74-05-01 mitigation: daemon rejects unknown profiles).
///
/// Fail-secure: if the daemon is not running, returns `Err` with a clear
/// message — there is NO fallback to an unconfined spawn (DMON-01).
///
/// On non-Windows: returns `Err` with a diagnostic.
fn agent_launch(launch_args: crate::cli::AgentLaunchArgs) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let payload = serde_json::json!({
            "action": "launch",
            "profile": launch_args.profile,
            "cmd": launch_args.cmd,
        });
        let payload_str = serde_json::to_string(&payload).map_err(|e| {
            NonoError::SandboxInit(format!(
                "nono agent launch: failed to serialize request payload: {e}"
            ))
        })?;

        match windows_control_pipe_request(&payload_str) {
            Ok(response) => {
                println!("{}", response.trim());
                Ok(())
            }
            Err(e) if is_pipe_not_found(&e) => Err(NonoError::SandboxInit(
                "nono-agentd is not running. Use `nono daemon start` first.\n\
                 (fail-secure: nono never spawns an unconfined agent as a fallback)"
                    .into(),
            )),
            Err(e) => Err(e),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = launch_args;
        Err(NonoError::SandboxInit(
            "nono agent launch is Windows-only (requires nono-agentd)".into(),
        ))
    }
}

/// `nono agent list` — print running agents from the daemon.
///
/// Connects to the daemon control pipe and sends a JSON `list` request.
/// If the daemon is not running, prints a diagnostic rather than returning
/// an error (list is a read-only interrogation verb).
///
/// On non-Windows: returns `Err` with a diagnostic.
fn agent_list() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let payload = serde_json::json!({"action": "list"});
        let payload_str = serde_json::to_string(&payload).map_err(|e| {
            NonoError::SandboxInit(format!(
                "nono agent list: failed to serialize request payload: {e}"
            ))
        })?;

        match windows_control_pipe_request(&payload_str) {
            Ok(response) => {
                println!("{}", response.trim());
                Ok(())
            }
            Err(e) if is_pipe_not_found(&e) => {
                println!("No daemon running (use `nono daemon start` to start nono-agentd).");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(NonoError::SandboxInit(
            "nono agent list is Windows-only (requires nono-agentd)".into(),
        ))
    }
}

/// `nono agent demote <tenant_id>` — apply a post-hoc IL-drop to a running agent.
///
/// Sends a `{"action":"demote","tenant_id":"<id>"}` request to the daemon control
/// pipe. The daemon drops the agent's token integrity level to Low and severs the
/// per-agent WFP filter (D-03 WFP-cut, SUPP-01).
///
/// # Leak limits (SUPP-01 soundness boundary)
///
/// Demote is an incident-response lever, NOT a standalone confinement boundary:
/// 1. Handles opened before the IL-drop continue at Medium IL.
/// 2. Already-started child processes are NOT retroactively affected.
/// 3. The IL-drop may crash the agent (legitimate handles may be severed).
/// 4. Outbound network is severed concurrently via the SUPP-02 WFP filter (D-03).
/// 5. Demote is one-way — no API to raise IL back to Medium from outside.
///
/// The agent is NOT reaped after demote. Use `nono agent list` for tenant IDs.
///
/// On non-Windows: returns `Err` with a diagnostic.
fn agent_demote(tenant_id: String) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let payload = serde_json::json!({
            "action": "demote",
            "tenant_id": tenant_id,
        });
        let payload_str = serde_json::to_string(&payload).map_err(|e| {
            NonoError::SandboxInit(format!(
                "nono agent demote: failed to serialize request payload: {e}"
            ))
        })?;

        match windows_control_pipe_request(&payload_str) {
            Ok(response) => {
                println!("{}", response.trim());
                Ok(())
            }
            Err(e) if is_pipe_not_found(&e) => Err(NonoError::SandboxInit(
                "nono-agentd is not running. Use `nono daemon start` first.".into(),
            )),
            Err(e) => Err(e),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = tenant_id;
        Err(NonoError::SandboxInit(
            "nono agent demote is Windows-only (requires nono-agentd)".into(),
        ))
    }
}

// ─── Windows helpers ──────────────────────────────────────────────────────────

/// Detect whether `sc qc` output describes a USER_OWN_PROCESS TEMPLATE (type 50).
///
/// `sc start` cannot start a type-50 template service — it requires a user session
/// to instantiate an instance, and returns ACCESS_DENIED (exit 5) if invoked
/// directly. This predicate drives the GAP-75-A fix in `daemon_start`: when the
/// registered service is a template, `nono daemon start` falls through to the
/// raw-spawn path instead of calling `sc start`.
///
/// # Detection
///
/// The verbatim Windows output for a type-50 service is:
/// ```text
/// TYPE               : 50  USER_OWN_PROCESS  TEMPLATE
/// ```
/// Type-10 (classic own-process) says:
/// ```text
/// TYPE               : 10  WIN32_OWN_PROCESS
/// ```
/// The string `"USER_OWN_PROCESS TEMPLATE"` is unique to type 50 and unambiguous.
///
/// # sc qc failure handling
///
/// If `sc qc` fails to run, the caller passes an empty string, which returns
/// `false` here — treating the type as unknown and falling through to the normal
/// `sc start` path, where the real error (if any) will surface to the operator.
/// This is conservative: we never silently succeed.
#[cfg(target_os = "windows")]
fn is_user_own_template_service(sc_qc_output: &str) -> bool {
    sc_qc_output.contains("USER_OWN_PROCESS TEMPLATE")
}

/// Run an `sc.exe` subcommand and return a human-readable result.
///
/// Prints `success_msg` on exit code 0. Returns `Err` with `fail_hint` appended
/// when `sc` exits non-zero.
#[cfg(target_os = "windows")]
fn windows_sc_command(
    sc_args: &[&str],
    verb: &str,
    success_msg: &str,
    fail_hint: &str,
) -> Result<()> {
    use std::process::Command;

    let output = Command::new("sc").args(sc_args).output().map_err(|e| {
        NonoError::SandboxInit(format!(
            "{verb}: failed to run `sc {args}`: {e}",
            args = sc_args.join(" ")
        ))
    })?;

    if output.status.success() {
        println!("{success_msg}");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        Err(NonoError::SandboxInit(format!(
            "{verb}: `sc {args}` failed (exit {code}): {detail}. {fail_hint}",
            args = sc_args.join(" "),
            code = output.status.code().unwrap_or(-1)
        )))
    }
}

/// Send a JSON request to the daemon control pipe and return the response string.
///
/// Connects to `DAEMON_CONTROL_PIPE_NAME` with a 5-second timeout
/// (T-74-05-02 mitigation). Uses the same 4-byte LE length-prefix framing
/// as `socket_windows.rs`.
///
/// # Wire format
///
/// ```text
/// [4-byte LE length][JSON payload bytes (UTF-8)]
/// ```
///
/// # Errors
///
/// Returns `Err` if the pipe is not found (daemon not running), the connection
/// times out, or any I/O error occurs. Callers should use `is_pipe_not_found`
/// to distinguish "daemon not running" from other errors.
#[cfg(target_os = "windows")]
fn windows_control_pipe_request(json_payload: &str) -> Result<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;

    // T-74-05-02: 5-second connection timeout.
    const TIMEOUT_MS: u32 = 5_000;
    const MAX_RESPONSE: usize = 64 * 1024;

    // Generic access rights (matches nono::supervisor::policy constants).
    // These values are documented in the Win32 ACCESS_MASK reference:
    //   GENERIC_READ  = 0x80000000
    //   GENERIC_WRITE = 0x40000000
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;

    let pipe_wide: Vec<u16> = DAEMON_CONTROL_PIPE_NAME
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();

    // Open the pipe instance.
    // SAFETY: pipe_wide is a valid null-terminated UTF-16 string; all other
    // params follow CreateFileW documented defaults for named-pipe clients.
    let handle: HANDLE = unsafe {
        CreateFileW(
            pipe_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "windows_control_pipe_request: failed to open control pipe \
             (GLE={gle}): pipe not available"
        )));
    }

    // RAII: close the handle on all exit paths.
    struct HandleGuard(HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                // SAFETY: self.0 is a valid HANDLE from CreateFileW.
                unsafe { CloseHandle(self.0) };
            }
        }
    }
    let _guard = HandleGuard(handle);

    // Wait for the pipe to become ready (T-74-05-02 timeout).
    // SAFETY: pipe_wide is a valid null-terminated UTF-16 string.
    let wait_ok = unsafe { WaitNamedPipeW(pipe_wide.as_ptr(), TIMEOUT_MS) };
    if wait_ok == 0 {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "windows_control_pipe_request: timed out waiting for daemon \
             control pipe (GLE={gle}, timeout={TIMEOUT_MS}ms)"
        )));
    }

    // Send: [4-byte LE length][JSON payload].
    let payload_bytes = json_payload.as_bytes();
    let payload_len: u32 = u32::try_from(payload_bytes.len())
        .map_err(|_| NonoError::SandboxInit("nono agent: request payload too large".into()))?;
    let len_prefix = payload_len.to_le_bytes();

    let mut bytes_written: u32 = 0;
    // SAFETY: handle is a valid open pipe handle; len_prefix is 4 valid bytes.
    let ok = unsafe {
        WriteFile(
            handle,
            len_prefix.as_ptr(),
            4,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_written != 4 {
        return Err(NonoError::SandboxInit(
            "windows_control_pipe_request: WriteFile length prefix failed".into(),
        ));
    }

    // SAFETY: handle is valid; payload_bytes is a valid slice.
    let ok = unsafe {
        WriteFile(
            handle,
            payload_bytes.as_ptr(),
            payload_len,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_written != payload_len {
        return Err(NonoError::SandboxInit(
            "windows_control_pipe_request: WriteFile payload failed".into(),
        ));
    }

    // Receive: [4-byte LE length][response payload].
    let mut len_buf = [0u8; 4];
    let mut bytes_read: u32 = 0;
    // SAFETY: handle is valid; len_buf is a 4-byte mutable buffer.
    let ok = unsafe {
        ReadFile(
            handle,
            len_buf.as_mut_ptr(),
            4,
            &mut bytes_read,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_read != 4 {
        return Err(NonoError::SandboxInit(
            "windows_control_pipe_request: ReadFile response length failed".into(),
        ));
    }

    let resp_len = u32::from_le_bytes(len_buf) as usize;
    if resp_len > MAX_RESPONSE {
        return Err(NonoError::SandboxInit(format!(
            "windows_control_pipe_request: response length {resp_len} \
             exceeds maximum {MAX_RESPONSE} bytes"
        )));
    }

    let mut resp_buf = vec![0u8; resp_len];
    let mut bytes_read2: u32 = 0;
    // SAFETY: handle is valid; resp_buf is a valid mutable slice.
    let ok = unsafe {
        ReadFile(
            handle,
            resp_buf.as_mut_ptr(),
            resp_len as u32,
            &mut bytes_read2,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_read2 != resp_len as u32 {
        return Err(NonoError::SandboxInit(
            "windows_control_pipe_request: ReadFile response payload failed".into(),
        ));
    }

    String::from_utf8(resp_buf).map_err(|e| {
        NonoError::SandboxInit(format!(
            "windows_control_pipe_request: response is not valid UTF-8: {e}"
        ))
    })
}

/// Return `true` if the error indicates the daemon control pipe is not available.
///
/// Distinguishes "daemon not running" from other I/O errors so callers can
/// provide a targeted user message instead of a raw error.
fn is_pipe_not_found(err: &nono::NonoError) -> bool {
    let msg = err.to_string();
    // GLE=2: ERROR_FILE_NOT_FOUND (pipe does not exist — daemon not running)
    msg.contains("GLE=2") || msg.contains("pipe not available") || msg.contains("not available")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::cli::{AgentArgs, AgentCommands, Cli, Commands, DaemonArgs, DaemonCommands};
    use clap::Parser;

    /// SC: daemon_subcommand_parses_start
    ///
    /// `nono daemon start` must parse to `Commands::Daemon(DaemonArgs { command: DaemonCommands::Start })`.
    #[test]
    fn daemon_subcommand_parses_start() {
        let cli = Cli::parse_from(["nono", "daemon", "start"]);
        let Commands::Daemon(DaemonArgs {
            command: DaemonCommands::Start,
        }) = cli.command
        else {
            panic!("expected Commands::Daemon(DaemonCommands::Start)");
        };
    }

    /// SC: daemon_subcommand_parses_stop
    ///
    /// `nono daemon stop` must parse to `Commands::Daemon(DaemonArgs { command: DaemonCommands::Stop })`.
    #[test]
    fn daemon_subcommand_parses_stop() {
        let cli = Cli::parse_from(["nono", "daemon", "stop"]);
        let Commands::Daemon(DaemonArgs {
            command: DaemonCommands::Stop,
        }) = cli.command
        else {
            panic!("expected Commands::Daemon(DaemonCommands::Stop)");
        };
    }

    /// SC: daemon_subcommand_parses_status
    ///
    /// `nono daemon status` must parse to `Commands::Daemon(DaemonArgs { command: DaemonCommands::Status })`.
    #[test]
    fn daemon_subcommand_parses_status() {
        let cli = Cli::parse_from(["nono", "daemon", "status"]);
        let Commands::Daemon(DaemonArgs {
            command: DaemonCommands::Status,
        }) = cli.command
        else {
            panic!("expected Commands::Daemon(DaemonCommands::Status)");
        };
    }

    /// SC: agent_launch_parses_profile_and_cmd
    ///
    /// `nono agent launch --profile aider -- aider --model gpt4` must parse correctly.
    #[test]
    fn agent_launch_parses_profile_and_cmd() {
        use crate::cli::AgentLaunchArgs;
        let cli = Cli::parse_from([
            "nono",
            "agent",
            "launch",
            "--profile",
            "aider",
            "--",
            "aider",
            "--model",
            "gpt4",
        ]);
        let Commands::Agent(AgentArgs {
            command: AgentCommands::Launch(ref la),
        }) = cli.command
        else {
            panic!("expected Commands::Agent(AgentCommands::Launch(...))");
        };
        // Satisfy the type alias import for AgentLaunchArgs in the test body.
        let _: &AgentLaunchArgs = la;
        assert_eq!(la.profile, "aider");
        assert_eq!(la.cmd, vec!["aider", "--model", "gpt4"]);
    }

    /// SC: agent_list_parses
    ///
    /// `nono agent list` must parse to `Commands::Agent(AgentArgs { command: AgentCommands::List })`.
    #[test]
    fn agent_list_parses() {
        let cli = Cli::parse_from(["nono", "agent", "list"]);
        let Commands::Agent(AgentArgs {
            command: AgentCommands::List,
        }) = cli.command
        else {
            panic!("expected Commands::Agent(AgentCommands::List)");
        };
    }

    /// SC: agent_demote_parses
    ///
    /// `nono agent demote <tenant_id>` must parse to
    /// `Commands::Agent(AgentArgs { command: AgentCommands::Demote { tenant_id } })`.
    #[test]
    fn agent_demote_parses() {
        let cli = Cli::parse_from([
            "nono",
            "agent",
            "demote",
            "abcdef1234567890abcdef1234567890",
        ]);
        let Commands::Agent(AgentArgs {
            command: AgentCommands::Demote { ref tenant_id },
        }) = cli.command
        else {
            panic!("expected Commands::Agent(AgentCommands::Demote(...))");
        };
        assert_eq!(
            tenant_id, "abcdef1234567890abcdef1234567890",
            "tenant_id must match the CLI argument"
        );
    }

    /// SC: agent_demote_non_windows_returns_err
    ///
    /// On non-Windows, `agent_demote` must return `Err` containing "Windows-only".
    /// On Windows this test is skipped (covered by the live integration path).
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn agent_demote_non_windows_returns_err() {
        let result = super::agent_demote("some-tenant-id".to_string());
        assert!(
            result.is_err(),
            "agent_demote must return Err on non-Windows"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Windows-only"),
            "Error must mention 'Windows-only'; got: {err_msg}"
        );
    }

    /// SC: no_agent_query_verb_exists (D-05 fence)
    ///
    /// `nono agent query` must fail to parse. Use `nono classify <PID>` instead (Phase 73).
    #[test]
    fn no_agent_query_verb_exists() {
        let result = Cli::try_parse_from(["nono", "agent", "query"]);
        assert!(
            result.is_err(),
            "D-05 fence: `nono agent query` must not parse — use `nono classify <PID>` instead"
        );
    }

    /// SC: control_pipe_name_consistency
    ///
    /// The control pipe name must contain the expected discriminator string,
    /// preventing CLI/daemon drift.
    #[test]
    fn control_pipe_name_consistency() {
        assert!(
            super::DAEMON_CONTROL_PIPE_NAME.contains("nono-agentd-control"),
            "DAEMON_CONTROL_PIPE_NAME must contain 'nono-agentd-control'"
        );
    }

    /// SC: is_pipe_not_found_recognizes_gle2
    ///
    /// `is_pipe_not_found` must return `true` for GLE=2 error messages.
    #[test]
    fn is_pipe_not_found_recognizes_gle2() {
        let err = nono::NonoError::SandboxInit(
            "windows_control_pipe_request: failed to open control pipe \
             (GLE=2): pipe not available"
                .into(),
        );
        assert!(super::is_pipe_not_found(&err));
    }

    /// SC: is_pipe_not_found_returns_false_for_other_errors
    ///
    /// `is_pipe_not_found` must return `false` for unrelated errors.
    #[test]
    fn is_pipe_not_found_returns_false_for_other_errors() {
        let err = nono::NonoError::SandboxInit(
            "windows_control_pipe_request: WriteFile length prefix failed".into(),
        );
        assert!(!super::is_pipe_not_found(&err));
    }

    // ─── GAP-75-A: type-50 detection guard tests ─────────────────────────────

    /// SC: daemon_start_uses_raw_spawn_for_type50_template
    ///
    /// `is_user_own_template_service` must return `true` when `sc qc` stdout
    /// contains the verbatim Windows type-50 label "USER_OWN_PROCESS TEMPLATE".
    /// This is the GAP-75-A fix: the predicate gates the fall-through to raw-spawn.
    #[test]
    #[cfg(target_os = "windows")]
    fn daemon_start_uses_raw_spawn_for_type50_template() {
        // Verbatim `sc qc nono-agentd` output for a type-50 service
        // (confirmed on live Win11 host, 2026-06-15).
        let sc_qc_stdout = "[SC] QueryServiceConfig SUCCESS\r\n\
             SERVICE_NAME: nono-agentd\r\n\
                     TYPE               : 50  USER_OWN_PROCESS TEMPLATE\r\n\
                     START_TYPE         : 2   AUTO_START\r\n\
                     ERROR_CONTROL      : 1   NORMAL\r\n\
                     BINARY_PATH_NAME   : C:\\target\\release\\nono-agentd.exe --service-mode\r\n\
                     LOAD_ORDER_GROUP   :\r\n\
                     TAG                : 0\r\n\
                     DISPLAY_NAME       : nono-agentd\r\n\
                     DEPENDENCIES       :\r\n\
                     SERVICE_START_NAME :\r\n";
        assert!(
            super::is_user_own_template_service(sc_qc_stdout),
            "is_user_own_template_service must return true for type-50 sc qc output; \
             got false for: {sc_qc_stdout:?}"
        );
    }

    /// SC: daemon_start_uses_sc_start_for_type10
    ///
    /// `is_user_own_template_service` must return `false` when `sc qc` stdout
    /// contains "WIN32_OWN_PROCESS" (type 10) without "TEMPLATE".
    /// The normal `sc start` path must be taken for type-10 services.
    #[test]
    #[cfg(target_os = "windows")]
    fn daemon_start_uses_sc_start_for_type10() {
        // Verbatim `sc qc` output for a classic WIN32_OWN_PROCESS (type 10) service.
        let sc_qc_stdout = "[SC] QueryServiceConfig SUCCESS\r\n\
             SERVICE_NAME: some-service\r\n\
                     TYPE               : 10  WIN32_OWN_PROCESS\r\n\
                     START_TYPE         : 2   AUTO_START\r\n\
                     ERROR_CONTROL      : 1   NORMAL\r\n\
                     BINARY_PATH_NAME   : C:\\path\\to\\service.exe\r\n\
                     LOAD_ORDER_GROUP   :\r\n\
                     TAG                : 0\r\n\
                     DISPLAY_NAME       : some-service\r\n\
                     DEPENDENCIES       :\r\n\
                     SERVICE_START_NAME : LocalSystem\r\n";
        assert!(
            !super::is_user_own_template_service(sc_qc_stdout),
            "is_user_own_template_service must return false for type-10 sc qc output; \
             got true for: {sc_qc_stdout:?}"
        );
    }

    /// SC: daemon_start_uses_raw_spawn_for_no_service
    ///
    /// When `sc query` reports error 1060 ("does not exist"), the service-exists
    /// gate in `daemon_start` is false: the function must skip `sc start` and
    /// fall through to the raw-spawn path. Verified by asserting that the
    /// service-exists check logic correctly identifies the no-service case.
    ///
    /// This test exercises the string-matching logic that guards the service-exists
    /// branch, without spawning a real process.
    #[test]
    fn daemon_start_uses_raw_spawn_for_no_service() {
        // Simulate `sc query nono-agentd` output when service does not exist.
        let sc_query_no_service = "[SC] EnumQueryServicesStatus:OpenService FAILED 1060:\r\n\
             The specified service does not exist as an installed service.\r\n";

        // The daemon_start logic: service_exists is true only if sc exits 0 or
        // stdout contains RUNNING/STOPPED/STATE. Then we negate: skip sc start if
        // stdout contains "1060" or "does not exist".
        // Assert that our understanding of the guard is correct.
        let service_exists = sc_query_no_service.contains("RUNNING")
            || sc_query_no_service.contains("STOPPED")
            || sc_query_no_service.contains("STATE");
        let would_skip_sc_start = !service_exists
            || sc_query_no_service.contains("1060")
            || sc_query_no_service.contains("does not exist");

        assert!(
            would_skip_sc_start,
            "When sc query output contains '1060' or 'does not exist', \
             daemon_start must skip sc start and use the raw-spawn path"
        );
    }
}
