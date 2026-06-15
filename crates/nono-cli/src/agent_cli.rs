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

/// `nono daemon start` — start nono-agentd via the SCM.
///
/// On Windows: runs `sc start nono-agentd`. Phase 75 may replace this with
/// a proper SCM Rust API call or direct control-pipe request.
///
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_start() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows_sc_command(
            &["start", DAEMON_SERVICE_NAME],
            "nono daemon start",
            "nono-agentd started successfully.",
            "nono-agentd may already be running, or the service is not installed. \
             Try `nono daemon install` first.",
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("nono-agentd is Windows-only.");
        Ok(())
    }
}

/// `nono daemon stop` — send a STOP to the SCM service.
///
/// On Windows: runs `sc stop nono-agentd`. Phase 75 may add a direct
/// control-pipe stop message.
///
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_stop() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows_sc_command(
            &["stop", DAEMON_SERVICE_NAME],
            "nono daemon stop",
            "nono-agentd stopped.",
            "nono-agentd may not be running, or the service is not installed.",
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("nono-agentd is Windows-only.");
        Ok(())
    }
}

/// `nono daemon status` — print whether the daemon is running.
///
/// On Windows: queries the SCM via `sc query`.
/// On non-Windows: prints a diagnostic and returns `Ok(())`.
fn daemon_status() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
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

        if output.status.success() && stdout.contains("RUNNING") {
            println!("nono-agentd status: RUNNING");
        } else if stdout.contains("STOPPED") {
            println!("nono-agentd status: STOPPED");
        } else if output.status.success() {
            println!("nono-agentd status: {}", stdout.trim());
        } else {
            println!("nono-agentd status: NOT INSTALLED (use `nono daemon install`)");
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
/// ADR-74 Decision 1: registers as `type= userservice` (SERVICE_USER_OWN_PROCESS,
/// NOT LocalSystem/SYSTEM). T-74-05-04 mitigation: `type= userservice` is
/// mandatory and always present in the `sc create` invocation.
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

        // ADR-74 Decision 1: type= userservice (per-user, NOT LocalSystem).
        // T-74-05-04: `type= userservice` is ALWAYS present — never omit.
        let binpath = format!("{agentd_str} --service-mode");

        let output = Command::new("sc")
            .args([
                "create",
                DAEMON_SERVICE_NAME,
                "type=",
                "userservice",
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
            println!("nono-agentd installed as a per-user service (type= userservice).");
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

// ─── Windows helpers ──────────────────────────────────────────────────────────

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

    let output = Command::new("sc")
        .args(sc_args)
        .output()
        .map_err(|e| {
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
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    };
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
    let payload_len: u32 = u32::try_from(payload_bytes.len()).map_err(|_| {
        NonoError::SandboxInit("nono agent: request payload too large".into())
    })?;
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
    msg.contains("GLE=2")
        || msg.contains("pipe not available")
        || msg.contains("not available")
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
}
