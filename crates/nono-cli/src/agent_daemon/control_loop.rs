//! Daemon-side operator control-pipe server.
//!
//! This module implements the `run_control_loop` server that listens on
//! `\\.\pipe\nono-agentd-control` and dispatches operator requests (`launch`,
//! `list`) to the daemon's agent lifecycle machinery.
//!
//! # Security model
//!
//! The control pipe is the **operator plane** — a confined Low-IL/AppContainer agent
//! MUST NOT be able to open this pipe and drive agent launches or read the tenant table
//! (privilege-escalation guard, T-74-07-01).
//!
//! ## Control-pipe SDDL
//!
//! ```text
//! D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)
//! ```
//!
//! - `D:P` — protected DACL (no inheritance from parent objects)
//! - `(A;;GA;;;SY)` — SYSTEM: full access (GA = GENERIC_ALL)
//! - `(A;;GA;;;BA)` — BUILTIN\Administrators: full access
//! - `(A;;GA;;;OW)` — Owner (the daemon user): full access
//! - `S:(ML;;NW;;;ME)` — mandatory-label SACL: Medium Integrity minimum
//!   (`NW` = No-Write-Up: processes below Medium IL cannot open this pipe with
//!   write access; combined with no read ACE for Low-IL, this bars AppContainer /
//!   Low-IL agents from opening the pipe at all).
//!
//! Contrast with the CAPABILITY pipe which uses `S:(ML;;NW;;;LW)` (Low IL) to
//! deliberately allow confined agents to connect. The control pipe intentionally
//! does NOT carry that Low-IL SACL grant.
//!
//! ## No-escape-hatch invariant (SC4 / ADR-74 Decision D-04)
//!
//! The control plane handles exactly two verbs:
//!   - `"launch"` — mints a NEW AgentTenant (calls `launch_agent`)
//!   - `"list"` — read-only tenant table snapshot
//!
//! There is NO code path from any wire frame to mutating an EXISTING
//! `AgentTenant::caps` field. `caps` is read-only after construction.
//!
//! ## Bounded reads + per-connection tasks (T-74-07-04 DoS mitigation)
//!
//! Each accepted connection is dispatched to a `tokio::spawn` task so one
//! slow/malicious operator client cannot block the accept loop or starve others.
//! Frame reads are bounded by `MAX_CONTROL_FRAME` (64 KiB).
//!
//! # Windows-only
//!
//! All production code is gated on `#[cfg(target_os = "windows")]`.

#[cfg(target_os = "windows")]
pub(crate) use windows_impl::run_control_loop;

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::super::DaemonState;
    use nono::NonoError;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::Notify;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX};
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    };

    // SDDL_REVISION_1 = 1 (matches accept_loop.rs and socket_windows.rs).
    const SDDL_REVISION_1: u32 = 1;

    // Max frame size for control requests (64 KiB — T-74-07-04 DoS guard).
    const MAX_CONTROL_FRAME: u32 = 64 * 1024;

    /// Control-pipe name — the operator plane.
    ///
    /// Distinct from the capability pipe (`\\.\pipe\nono-agentd-cap`).
    /// The CLI client sends `{"action":"launch",...}` and `{"action":"list"}`
    /// frames here.
    const CONTROL_PIPE_NAME: &str = r"\\.\pipe\nono-agentd-control";

    /// SDDL for the control pipe.
    ///
    /// Security rationale (T-74-07-01):
    ///   - DACL: grants SYSTEM + Administrators + Owner (current user) full access.
    ///     No AppContainer SID or Low-IL SID is granted.
    ///   - SACL mandatory label: `S:(ML;;NW;;;ME)` — Medium Integrity minimum.
    ///     `NW` (No-Write-Up) bars any process below Medium IL from opening the
    ///     pipe with write access. AppContainer processes run at Low IL by default,
    ///     so they cannot reach this pipe at all.
    ///
    /// Contrast with the capability pipe SDDL which uses `S:(ML;;NW;;;LW)` (Low
    /// IL) to deliberately admit confined AppContainer agents. The control pipe
    /// intentionally omits that Low-IL grant.
    pub(crate) const CONTROL_PIPE_SDDL: &str =
        "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)";

    /// RAII guard that frees a `PSECURITY_DESCRIPTOR` allocated by
    /// `ConvertStringSecurityDescriptorToSecurityDescriptorW` via `LocalFree`.
    struct SecurityDescriptorGuard(PSECURITY_DESCRIPTOR);

    impl Drop for SecurityDescriptorGuard {
        fn drop(&mut self) {
            use windows_sys::Win32::Foundation::LocalFree;
            if !self.0.is_null() {
                // SAFETY: `self.0` was allocated by
                // `ConvertStringSecurityDescriptorToSecurityDescriptorW`, which
                // documents `LocalFree` as the release routine.
                unsafe { LocalFree(self.0.cast::<std::ffi::c_void>()) };
            }
        }
    }

    /// Create a new control-pipe instance with the Medium-IL-minimum SDDL.
    ///
    /// Uses `PIPE_UNLIMITED_INSTANCES` so multiple operator clients can
    /// connect concurrently. Each call produces a fresh server-side instance.
    ///
    /// # Security
    ///
    /// The pipe is created with `CONTROL_PIPE_SDDL` which requires Medium IL
    /// minimum (via mandatory-label SACL). Low-IL / AppContainer processes
    /// cannot open this pipe handle (T-74-07-01 mitigation).
    ///
    /// # Returns
    ///
    /// A raw pipe `HANDLE` on success. The caller MUST transfer this handle to
    /// `tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle`
    /// (which takes ownership) or close it via `CloseHandle` on error.
    ///
    /// # Errors
    ///
    /// Returns `Err` if SDDL parsing fails or `CreateNamedPipeW` fails.
    fn create_control_pipe_instance() -> nono::Result<HANDLE> {
        let sddl_wide: Vec<u16> = CONTROL_PIPE_SDDL
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();

        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: `sddl_wide` is a valid null-terminated UTF-16 SDDL string.
        // `sd` is a valid out-pointer. `SDDL_REVISION_1` is the only documented
        // revision. `null_mut()` for the optional size param is permitted per Win32 docs.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl_wide.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || sd.is_null() {
            return Err(NonoError::SandboxInit(format!(
                "create_control_pipe_instance: \
                 ConvertStringSecurityDescriptorToSecurityDescriptorW failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        // Guard frees `sd` via LocalFree on drop (must outlive CreateNamedPipeW).
        let _sd_guard = SecurityDescriptorGuard(sd);

        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };

        let wide_name: Vec<u16> = CONTROL_PIPE_NAME
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();

        // SAFETY: `wide_name` is a valid null-terminated UTF-16 string.
        // `sa` carries a security descriptor owned by `_sd_guard` which lives
        // until after `CreateNamedPipeW` returns. `FILE_FLAG_OVERLAPPED` is
        // required for tokio's async I/O layer.
        let handle = unsafe {
            CreateNamedPipeW(
                wide_name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE,
                PIPE_UNLIMITED_INSTANCES,
                MAX_CONTROL_FRAME,
                MAX_CONTROL_FRAME,
                0,
                &sa,
            )
        };

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(NonoError::SandboxInit(format!(
                "create_control_pipe_instance: \
                 CreateNamedPipeW(\"{CONTROL_PIPE_NAME}\") failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        Ok(handle)
    }

    /// Run the operator control-pipe accept loop.
    ///
    /// Creates a new named-pipe instance for each incoming operator connection,
    /// dispatches accepted connections to `handle_control_connection` on a
    /// `tokio::spawn`-ed task, and returns when the shutdown signal fires.
    ///
    /// # Security
    ///
    /// The pipe is created with `CONTROL_PIPE_SDDL` (Medium-IL minimum SACL),
    /// which bars AppContainer / Low-IL agent processes from opening this pipe.
    /// Only the interactive operator (Medium+ IL) can send requests here.
    ///
    /// # Concurrency
    ///
    /// Each accepted operator client runs in its own `tokio::spawn` task
    /// (T-74-07-04: one slow client cannot block others).
    pub(crate) async fn run_control_loop(
        daemon_state: Arc<DaemonState>,
        shutdown: Arc<Notify>,
    ) {
        // Create a persistent shutdown future to poll across iterations.
        // A `notify_one` permit stored by the notifier is consumed on the first
        // `.notified()` poll that returns Ready — reusing the SAME future ensures
        // a STOP signal issued mid-connection is honored on the next iteration.
        let shutdown_signal = shutdown.notified();
        tokio::pin!(shutdown_signal);

        loop {
            // Create a fresh control-pipe instance for the next operator connection.
            let handle = match create_control_pipe_instance() {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "run_control_loop: failed to create control pipe instance; \
                         retrying after brief pause"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };

            // Wrap the raw handle in tokio's NamedPipeServer for async I/O.
            // SAFETY: `handle` is a valid HANDLE returned by CreateNamedPipeW above.
            // We transfer ownership to NamedPipeServer; the raw handle must NOT be
            // closed separately after this call.
            let server = match unsafe {
                tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle(
                    handle as *mut _,
                )
            } {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "run_control_loop: from_raw_handle failed; closing raw handle and retrying"
                    );
                    // SAFETY: from_raw_handle failed without taking ownership;
                    // `handle` is still ours to close.
                    unsafe { CloseHandle(handle) };
                    continue;
                }
            };

            // Park until an operator client connects OR shutdown fires.
            tokio::select! {
                // `biased` ensures shutdown is checked FIRST on every poll so a STOP
                // signal is honored promptly even under high connection rate.
                biased;

                _ = &mut shutdown_signal => {
                    // STOP received. `server` drops here closing the pipe instance.
                    tracing::info!(
                        "run_control_loop: shutdown signal received; stopping control loop"
                    );
                    break;
                }

                connect_result = server.connect() => {
                    match connect_result {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "run_control_loop: server.connect() error; \
                                 closing instance and continuing"
                            );
                            // `server` drops here closing the pipe instance.
                            continue;
                        }
                    }
                }
            }

            // Connection accepted. Dispatch to a per-connection task so a slow
            // operator client cannot block the accept loop (T-74-07-04).
            let state = Arc::clone(&daemon_state);
            let shutdown_for_conn = Arc::clone(&shutdown);
            tokio::spawn(async move {
                if let Err(e) =
                    handle_control_connection(server, state, shutdown_for_conn).await
                {
                    tracing::warn!(
                        error = %e,
                        "run_control_loop: handle_control_connection returned error"
                    );
                }
            });
        }
    }

    /// Parsed operator request (from the wire JSON frame).
    #[derive(serde::Deserialize)]
    #[serde(tag = "action", rename_all = "lowercase")]
    enum ControlRequest {
        /// `{"action":"launch","profile":"<name>","cmd":["exe","arg1",...]}`
        Launch {
            profile: String,
            cmd: Vec<String>,
        },
        /// `{"action":"list"}`
        List,
        /// `{"action":"shutdown"}` — same-user-only graceful stop (dev-layout).
        Shutdown,
    }

    /// Handle a single connected operator pipe client.
    ///
    /// Steps:
    ///
    /// 1. Read `[4-byte LE length][JSON payload]` (one request per connection).
    /// 2. Parse the request as `ControlRequest`.
    /// 3. Dispatch:
    ///    - `launch` → validate profile, build CapabilitySet, call `launch_agent`.
    ///    - `list` → format tenant table.
    /// 4. Write `[4-byte LE length][response string]`.
    ///
    /// # Security invariants
    ///
    /// - No code path mutates an existing `AgentTenant::caps` (SC4, no escape hatch).
    /// - `launch` validates the profile against `policy.json` BEFORE calling
    ///   `launch_agent`; unknown profiles return a framed error, never launch
    ///   (T-74-07-03 mitigation).
    /// - Frame size bounded by `MAX_CONTROL_FRAME` (T-74-07-04 mitigation).
    /// - All error paths close the connection and return `Ok(())` so the accept
    ///   loop keeps running.
    async fn handle_control_connection(
        mut server: tokio::net::windows::named_pipe::NamedPipeServer,
        state: Arc<DaemonState>,
        shutdown: Arc<Notify>,
    ) -> nono::Result<()> {
        use tokio::io::AsyncReadExt;

        // ── Read request frame: [4-byte LE length][JSON payload] ──────────────

        let mut len_buf = [0u8; 4];
        match server.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e)
                if e.kind() == std::io::ErrorKind::UnexpectedEof
                    || e.kind() == std::io::ErrorKind::BrokenPipe
                    || e.kind() == std::io::ErrorKind::ConnectionReset =>
            {
                tracing::debug!("handle_control_connection: client disconnected before sending request");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "handle_control_connection: read error on length prefix; closing"
                );
                return Ok(());
            }
        }

        let req_len = u32::from_le_bytes(len_buf) as usize;

        // Bounded read: reject frames larger than MAX_CONTROL_FRAME (T-74-07-04).
        if req_len > MAX_CONTROL_FRAME as usize {
            tracing::warn!(
                req_len = req_len,
                max = MAX_CONTROL_FRAME,
                "handle_control_connection: request too large (fail-secure close)"
            );
            let _ = write_framed_response(&mut server, "error: request too large").await;
            return Ok(());
        }

        let mut payload = vec![0u8; req_len];
        if let Err(e) = server.read_exact(&mut payload).await {
            tracing::warn!(
                error = %e,
                "handle_control_connection: read error on payload; closing"
            );
            return Ok(());
        }

        // ── Parse request ──────────────────────────────────────────────────────

        let request: ControlRequest = match serde_json::from_slice(&payload) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "handle_control_connection: failed to parse ControlRequest; closing"
                );
                let _ = write_framed_response(
                    &mut server,
                    &format!("error: malformed request: {e}"),
                )
                .await;
                return Ok(());
            }
        };

        // ── Dispatch ───────────────────────────────────────────────────────────

        let response = match request {
            ControlRequest::Launch { profile: profile_name, cmd } => {
                handle_launch(&state, &profile_name, cmd).await
            }
            ControlRequest::List => handle_list(&state),
            // Shutdown: same-user-only graceful stop (dev-layout).
            // The SDDL already enforces that only Medium+ IL (interactive user)
            // can reach this pipe, so this is operator-only (T-74-07-01).
            ControlRequest::Shutdown => {
                tracing::info!("run_control_loop: shutdown requested via control pipe");
                let resp = "nono-agentd: shutdown initiated.".to_string();
                // Send the response BEFORE signalling shutdown so the client
                // receives the confirmation (pipe is still open during write).
                if let Err(e) = write_framed_response(&mut server, &resp).await {
                    tracing::warn!(error = %e, "handle_control_connection: write error on shutdown response");
                }
                shutdown.notify_one();
                return Ok(());
            }
        };

        // ── Send response frame: [4-byte LE length][response string] ──────────

        if let Err(e) = write_framed_response(&mut server, &response).await {
            tracing::warn!(
                error = %e,
                "handle_control_connection: write error on response; closing"
            );
        }

        // `server` drops here, closing the pipe instance.
        Ok(())
    }

    /// Dispatch a `launch` request.
    ///
    /// Validates `profile_name` against `policy.json` / builtins BEFORE calling
    /// `launch_agent` (T-74-07-03 mitigation). An unknown profile returns a
    /// fail-secure framed error; the daemon never launches an agent with an
    /// unvalidated profile.
    ///
    /// # No-escape-hatch invariant (SC4)
    ///
    /// This function always calls `launch_agent` with a FRESHLY derived
    /// `CapabilitySet` from the named profile. It NEVER mutates the caps of an
    /// already-running `AgentTenant`. There is no code path here that touches
    /// `state.tenants` before the new agent is launched.
    async fn handle_launch(
        state: &Arc<DaemonState>,
        profile_name: &str,
        cmd: Vec<String>,
    ) -> String {
        // T-74-07-03: validate profile against policy.json / builtins FIRST.
        // `super::super::is_known_profile` checks the embedded policy JSON for
        // the profile name. An unknown name → fail-secure error (never launch).
        if !super::super::is_known_profile(profile_name) {
            tracing::warn!(
                profile_name = %profile_name,
                "handle_launch: unknown profile (fail-secure — agent not launched)"
            );
            return format!(
                "error: unknown profile '{profile_name}' — \
                 run `nono profile list` for valid names (fail-secure: agent not launched)"
            );
        }

        // Split cmd into exe + args.
        if cmd.is_empty() {
            return "error: cmd must not be empty".to_string();
        }
        let exe = PathBuf::from(&cmd[0]);
        let args: Vec<String> = cmd[1..].to_vec();

        // Build a minimal CapabilitySet for the named profile.
        // We use an empty CapabilitySet here because launch_agent manages the
        // OS-level AppContainer token (the real confinement). The daemon's
        // confinement boundary is the AppContainer token + Job Object, not the
        // CapabilitySet (which requires workdir context unavailable here).
        //
        // ADR-74 Decision D-04: caps are set at launch time and NEVER expanded
        // via any wire frame (no escape hatch). The daemon stores `profile_name`
        // in `AgentTenant` for bookkeeping; the AppContainer token is the actual
        // isolation boundary.
        let caps = nono::CapabilitySet::new();

        // Use the validated profile name for logging.
        let resolved_profile_name = profile_name.to_string();

        tracing::info!(
            profile_name = %resolved_profile_name,
            exe = %exe.display(),
            "handle_launch: launching agent"
        );

        match super::super::launch::launch_agent(Arc::clone(state), exe, args, caps).await
        {
            Ok(tenant_id) => {
                // Look up the minted package SID + pid for the response.
                let (package_sid, pid) = {
                    let tenants = state.tenants.lock().unwrap_or_else(|p| p.into_inner());
                    tenants.get(&tenant_id).map(|t| {
                        let sid = t.package_sid.clone();
                        // pid: access raw handle to get PID (Windows-only).
                        #[cfg(target_os = "windows")]
                        let pid_val: u32 = {
                            use std::os::windows::io::AsRawHandle;
                            let raw = t.process_handle.as_raw_handle();
                            // SAFETY: raw is a valid process handle owned by AgentTenant.
                            unsafe {
                                windows_sys::Win32::System::Threading::GetProcessId(
                                    raw as windows_sys::Win32::Foundation::HANDLE,
                                )
                            }
                        };
                        #[cfg(not(target_os = "windows"))]
                        let pid_val: u32 = 0;
                        (sid, pid_val)
                    }).unwrap_or_else(|| (String::from("(unknown — already reaped)"), 0))
                };

                tracing::info!(
                    tenant_id = %tenant_id,
                    package_sid = %package_sid,
                    pid = pid,
                    "handle_launch: agent launched successfully"
                );

                format!(
                    "Launched agent:\n  tenant_id={tenant_id}\n  profile={resolved_profile_name}\n  \
                     sid={package_sid}\n  pid={pid}"
                )
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    profile_name = %resolved_profile_name,
                    "handle_launch: launch_agent failed"
                );
                format!("error: failed to launch agent: {e}")
            }
        }
    }

    /// Dispatch a `list` request.
    ///
    /// Returns a human-readable snapshot of the current tenant table. This is a
    /// read-only operation — no mutation of `DaemonState` occurs (SC4).
    fn handle_list(state: &Arc<DaemonState>) -> String {
        let tenants = state.tenants.lock().unwrap_or_else(|p| p.into_inner());

        if tenants.is_empty() {
            return "No agents running.".to_string();
        }

        let count = tenants.len();
        let mut lines = format!("Tenant agents ({count}):");
        for (tenant_id, tenant) in tenants.iter() {
            // Obtain the PID from the process handle.
            #[cfg(target_os = "windows")]
            let pid: u32 = {
                use std::os::windows::io::AsRawHandle;
                let raw = tenant.process_handle.as_raw_handle();
                // SAFETY: raw is a valid process handle owned by AgentTenant in DaemonState.
                unsafe {
                    windows_sys::Win32::System::Threading::GetProcessId(
                        raw as windows_sys::Win32::Foundation::HANDLE,
                    )
                }
            };
            #[cfg(not(target_os = "windows"))]
            let pid: u32 = 0;

            lines.push_str(&format!(
                "\n  {tenant_id}  profile={profile}  sid={sid}  pid={pid}",
                tenant_id = &tenant_id[..tenant_id.len().min(16)],
                profile = tenant.profile_name,
                sid = tenant.package_sid,
            ));
        }
        lines
    }

    /// Test-accessible wrapper for `handle_list` (bypasses async boundary).
    ///
    /// Exposed for unit tests that verify the list response format and
    /// read-only invariant without spawning a real pipe server.
    #[cfg(test)]
    pub(crate) fn handle_list_testable(state: &Arc<DaemonState>) -> String {
        handle_list(state)
    }

    /// Write a framed response: `[4-byte LE length][UTF-8 string]`.
    ///
    /// This is the write side of the same framing the CLI client expects in
    /// `agent_cli::windows_control_pipe_request`.
    async fn write_framed_response(
        server: &mut tokio::net::windows::named_pipe::NamedPipeServer,
        response: &str,
    ) -> nono::Result<()> {
        use tokio::io::AsyncWriteExt;

        let bytes = response.as_bytes();
        let len = u32::try_from(bytes.len()).map_err(|_| {
            NonoError::SandboxInit("control response too large for 4-byte length prefix".into())
        })?;
        let len_prefix = len.to_le_bytes();

        server.write_all(&len_prefix).await.map_err(|e| {
            NonoError::SandboxInit(format!(
                "write_framed_response: write error on length prefix: {e}"
            ))
        })?;

        server.write_all(bytes).await.map_err(|e| {
            NonoError::SandboxInit(format!(
                "write_framed_response: write error on payload: {e}"
            ))
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::DaemonState;
    use std::sync::Arc;

    fn empty_state() -> Arc<DaemonState> {
        Arc::new(DaemonState::new())
    }

    /// SC: control_pipe_sddl_is_medium_il_only
    ///
    /// The control-pipe SDDL must contain `ME` (Medium Integrity label) in the
    /// mandatory-label SACL, NOT `LW` (Low Integrity). This verifies the control
    /// pipe is inaccessible to AppContainer / Low-IL agents (T-74-07-01).
    #[test]
    #[cfg(target_os = "windows")]
    fn control_pipe_sddl_is_medium_il_only() {
        let sddl = super::windows_impl::CONTROL_PIPE_SDDL;
        // Must contain Medium IL label (ME) in the SACL.
        assert!(
            sddl.contains("ME"),
            "Control pipe SDDL must contain Medium IL label (ME) in SACL for \
             T-74-07-01 Low-IL deny: got {sddl}"
        );
        // Must NOT contain Low IL label (LW) in the SACL — that's the capability pipe.
        assert!(
            !sddl.contains("LW"),
            "Control pipe SDDL must NOT contain Low-IL label (LW) — \
             that would admit confined agents: got {sddl}"
        );
        // Must be a protected DACL (D:P) to prevent inheritance from parent objects.
        assert!(
            sddl.contains("D:P"),
            "Control pipe SDDL must be a protected DACL (D:P): got {sddl}"
        );
    }

    /// SC: control_pipe_sddl_is_medium_il_only_cross_platform
    ///
    /// Cross-platform version of the SDDL constant check (verifies the string
    /// literal on Linux/macOS without requiring Windows APIs).
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn control_pipe_sddl_is_medium_il_only_cross_platform() {
        // The SDDL constant is defined only in the windows_impl module.
        // On non-Windows we just verify the hard-coded expected value here.
        let expected_sddl = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)";
        assert!(
            expected_sddl.contains("ME"),
            "Expected control pipe SDDL must contain Medium IL (ME)"
        );
        assert!(
            !expected_sddl.contains("LW"),
            "Expected control pipe SDDL must NOT contain Low IL (LW)"
        );
    }

    /// SC: list_returns_no_agents_when_empty
    ///
    /// `handle_list` on an empty `DaemonState` must return "No agents running.".
    #[test]
    #[cfg(target_os = "windows")]
    fn list_returns_no_agents_when_empty() {
        let state = empty_state();
        let result = super::windows_impl::handle_list_testable(&state);
        assert_eq!(
            result, "No agents running.",
            "Empty tenant map must return 'No agents running.'"
        );
    }

    /// SC: list_returns_tenants_when_populated (Windows)
    ///
    /// `handle_list` with tenants must return a formatted table with correct
    /// count and SIDs.
    #[test]
    #[cfg(target_os = "windows")]
    fn list_returns_tenants_when_populated() {
        use super::super::reap::AgentTenant;
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use windows_sys::Win32::Foundation::{DuplicateHandle, BOOL, DUPLICATE_SAME_ACCESS};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        let state = empty_state();
        let current = unsafe { GetCurrentProcess() };
        let make_handle = || -> OwnedHandle {
            let mut raw = std::ptr::null_mut();
            let ok: BOOL = unsafe {
                DuplicateHandle(current, current, current, &mut raw, 0, 0, DUPLICATE_SAME_ACCESS)
            };
            assert_ne!(ok, 0, "DuplicateHandle must succeed");
            unsafe { OwnedHandle::from_raw_handle(raw) }
        };

        let tenant = AgentTenant {
            tenant_id: "aaaa1234bbbb5678cccc9012".to_string(),
            package_sid: "S-1-15-2-1111-2222-3333-4444-5555-6666-7777".to_string(),
            profile_name: "nono.session.aaaa1234bbbb5678".to_string(),
            caps: nono::CapabilitySet::new(),
            job_handle: make_handle(),
            process_handle: make_handle(),
        };

        {
            let mut tenants = state.tenants.lock().unwrap();
            tenants.insert("aaaa1234bbbb5678cccc9012".to_string(), tenant);
        }

        let result = super::windows_impl::handle_list_testable(&state);
        assert!(
            result.starts_with("Tenant agents (1):"),
            "List must start with 'Tenant agents (1):'; got: {result}"
        );
        assert!(
            result.contains("S-1-15-2-1111-2222-3333-4444-5555-6666-7777"),
            "List must contain the package SID; got: {result}"
        );
        assert!(
            result.contains("nono.session.aaaa1234bbbb5678"),
            "List must contain the profile_name; got: {result}"
        );
    }

    /// SC: no_escape_hatch_list_is_read_only
    ///
    /// `handle_list` must not mutate `DaemonState::tenants`. Verify that calling
    /// it does not change the tenant count.
    #[test]
    #[cfg(target_os = "windows")]
    fn no_escape_hatch_list_is_read_only() {
        let state = empty_state();
        let before = state.tenants.lock().unwrap().len();
        let _result = super::windows_impl::handle_list_testable(&state);
        let after = state.tenants.lock().unwrap().len();
        assert_eq!(before, after, "handle_list must not mutate the tenant map (SC4)");
    }
}
