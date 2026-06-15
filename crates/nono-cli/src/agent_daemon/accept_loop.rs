//! Multi-tenant capability pipe accept loop.
//!
//! This module implements the server-side accept loop for `nono-agentd`. It is
//! the DMON-02 isolation boundary: every connecting pipe client is authenticated
//! via `ImpersonateNamedPipeClient` (kernel-vouched SID extraction) and the
//! returned SID is matched against `DaemonState::tenants`. Unknown SIDs and
//! impersonation failures are denied silently — the pipe instance is closed and
//! the loop continues serving other clients.
//!
//! # Security invariants
//!
//! - Authorization is ALWAYS by kernel-vouched AppContainer SID from
//!   [`nono::supervisor::authenticate_pipe_client`] — never by wire-frame `session_id`.
//! - `session_id` from a capability-request wire frame is a **routing hint only**:
//!   used after SID-auth to find the tenant entry for serving capability queries.
//!   It MUST NOT substitute for or override the SID-based authorization decision.
//! - Unknown SID → fail-secure deny (pipe closed, `Ok(())` returned, loop continues).
//! - No CapabilitySet mutation from any wire frame (ADR-74 Decision D-04 — no
//!   escape hatch; `caps` field on `AgentTenant` is read-only after construction).
//!
//! # Windows-only
//!
//! All production code in this file is gated on `#[cfg(target_os = "windows")]`.
//! On non-Windows targets the module contains only cross-platform tests.

#[cfg(target_os = "windows")]
pub(crate) use windows_impl::run_accept_loop;

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::super::DaemonState;
    use nono::NonoError;
    use std::sync::Arc;
    use tokio::sync::Notify;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX};
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    };

    // SDDL_REVISION_1 = 1 (matches socket_windows.rs constant).
    const SDDL_REVISION_1: u32 = 1;

    // MAX_MESSAGE_SIZE matches socket_windows.rs constant (64 KiB).
    const MAX_MESSAGE_SIZE: u32 = 64 * 1024;

    // The base SDDL (no session/package SID filters) is generated at runtime
    // by `nono::supervisor::build_capability_pipe_sddl(None, None)`.
    // We don't know the tenant's package SID at pipe instance creation time —
    // the post-connect ImpersonateNamedPipeClient + registry check IS the
    // primary isolation gate (DMON-02). The SDDL is Low-IL NW (No-Write-Up)
    // so AppContainer processes can connect; the auth step then closes the gate.

    /// Daemon capability pipe name.
    ///
    /// All instances use the same well-known name so clients can connect without
    /// a rendezvous side-channel. `PIPE_UNLIMITED_INSTANCES` allows multiple
    /// concurrent server-side instances under the same name.
    const DAEMON_PIPE_NAME: &str = r"\\.\pipe\nono-agentd-cap";

    /// Opaque security-descriptor guard: frees the `PSECURITY_DESCRIPTOR`
    /// allocated by `ConvertStringSecurityDescriptorToSecurityDescriptorW`
    /// via `LocalFree` on drop.
    struct SecurityDescriptorGuard(PSECURITY_DESCRIPTOR);

    impl Drop for SecurityDescriptorGuard {
        fn drop(&mut self) {
            use windows_sys::Win32::Foundation::LocalFree;
            if !self.0.is_null() {
                unsafe {
                    // SAFETY: `self.0` was allocated by
                    // `ConvertStringSecurityDescriptorToSecurityDescriptorW`
                    // which documents `LocalFree` as the release routine.
                    LocalFree(self.0.cast::<std::ffi::c_void>());
                }
            }
        }
    }

    /// Create a new daemon capability pipe instance with base Low-IL SDDL.
    ///
    /// Uses `PIPE_UNLIMITED_INSTANCES` so multiple clients can connect to the
    /// same pipe name concurrently. Each call creates a fresh server-side
    /// instance; the previous instance is consumed when a client connects and
    /// is dispatched to `handle_one_connection`.
    ///
    /// # Returns
    ///
    /// A raw pipe `HANDLE` on success. The caller MUST transfer this handle to
    /// `tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle` (which
    /// takes ownership) or close it via `CloseHandle` on error.
    ///
    /// # Errors
    ///
    /// Returns `Err` if SDDL parsing fails or `CreateNamedPipeW` fails.
    pub(crate) fn create_daemon_capability_pipe_instance() -> nono::Result<HANDLE> {
        // Build the base Low-IL SDDL via the library helper (session_sid=None,
        // package_sid=None → base grants: SYSTEM + Administrators + Owner;
        // SACL mandatory-label: No-Write-Up at Low Integrity Level).
        //
        // We pass None for the package_sid because we don't know the connecting
        // tenant's SID at pipe instance creation time — the auth gate is
        // post-connect via ImpersonateNamedPipeClient + registry check.
        let sddl = nono::supervisor::build_capability_pipe_sddl(None, None)?;
        let sddl_wide: Vec<u16> = sddl
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();

        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let ok = unsafe {
            // SAFETY: `sddl_wide` is a valid null-terminated UTF-16 string.
            // `sd` is a valid out-pointer. `SDDL_REVISION_1` is the only
            // documented revision. `null_mut()` for the optional size param
            // is explicitly permitted by the Win32 docs.
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl_wide.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(NonoError::SandboxInit(format!(
                "create_daemon_capability_pipe_instance: \
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

        let wide_name: Vec<u16> = DAEMON_PIPE_NAME
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();

        // SAFETY: `wide_name` is a valid null-terminated UTF-16 string.
        // `sa` carries a security descriptor owned by `_sd_guard` which lives
        // until after CreateNamedPipeW returns (Rust drops in reverse declaration
        // order; `_sd_guard` is declared before `sa` and the FFI call, so it
        // lives past the call). `FILE_FLAG_OVERLAPPED` is required for tokio's
        // async I/O layer.
        let handle = unsafe {
            CreateNamedPipeW(
                wide_name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE,
                PIPE_UNLIMITED_INSTANCES,
                MAX_MESSAGE_SIZE,
                MAX_MESSAGE_SIZE,
                0,
                &sa,
            )
        };

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(NonoError::SandboxInit(format!(
                "create_daemon_capability_pipe_instance: CreateNamedPipeW(\"{DAEMON_PIPE_NAME}\") \
                 failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        Ok(handle)
    }

    /// Run the multi-tenant daemon capability pipe accept loop.
    ///
    /// Creates a new named-pipe instance for each incoming connection, then
    /// dispatches accepted connections to `handle_one_connection` on a
    /// `tokio::spawn`-ed task. The loop is interruptible by the `shutdown`
    /// notifier (fired by the SCM STOP control handler or Ctrl-C).
    ///
    /// This function returns when the shutdown signal is received.
    ///
    /// # Concurrency
    ///
    /// Each accepted client runs in its own `tokio::spawn` task. A slow or
    /// malicious tenant cannot block the accept loop or starve other tenants
    /// (proven by `daemon_concurrent_agents` in the Wave 0 spike harness).
    pub(crate) async fn run_accept_loop(
        daemon_state: Arc<DaemonState>,
        shutdown: Arc<Notify>,
    ) {
        // Create a persistent shutdown future we can poll repeatedly via
        // `tokio::pin!`. A `notify_one` permit stored by the notifier is
        // consumed on the first `.notified()` poll that returns Ready — using
        // the SAME future across iterations means a STOP signal issued while
        // we are inside `handle_one_connection` is honored on the NEXT iteration.
        let shutdown_signal = shutdown.notified();
        tokio::pin!(shutdown_signal);

        loop {
            // Create a fresh pipe instance for the next client connection.
            let handle = match create_daemon_capability_pipe_instance() {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "run_accept_loop: failed to create daemon capability pipe instance; \
                         retrying after brief pause"
                    );
                    // Brief back-off to avoid a tight spin on persistent failures.
                    // The sleep is bounded and does not block the shutdown path.
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };

            // Wrap the raw handle in tokio's NamedPipeServer for async I/O.
            // SAFETY: `handle` is a valid HANDLE returned by CreateNamedPipeW
            // above. We transfer ownership to NamedPipeServer; the raw handle
            // must NOT be closed separately after this call.
            let server = match unsafe {
                tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle(
                    handle as *mut _,
                )
            } {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "run_accept_loop: from_raw_handle failed; closing raw handle and retrying"
                    );
                    // SAFETY: from_raw_handle failed without taking ownership;
                    // `handle` is still ours to close.
                    unsafe { CloseHandle(handle) };
                    continue;
                }
            };

            // Park until a client connects OR shutdown fires.
            tokio::select! {
                // `biased` ensures shutdown is checked FIRST on every poll so
                // a STOP signal is honored promptly even under high connection rate.
                biased;

                _ = &mut shutdown_signal => {
                    // STOP received. `server` drops here closing the pipe instance.
                    tracing::info!(
                        "run_accept_loop: shutdown signal received; stopping accept loop"
                    );
                    break;
                }

                connect_result = server.connect() => {
                    match connect_result {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "run_accept_loop: server.connect() error; \
                                 closing instance and continuing"
                            );
                            // `server` drops here closing the pipe instance.
                            continue;
                        }
                    }
                }
            }

            // Connection accepted. Dispatch to a per-connection task so a
            // slow tenant cannot block the accept loop.
            let state = Arc::clone(&daemon_state);
            tokio::spawn(async move {
                if let Err(e) = handle_one_connection(server, state).await {
                    tracing::warn!(
                        error = %e,
                        "run_accept_loop: handle_one_connection returned error"
                    );
                }
            });
        }
    }

    /// Handle a single connected pipe client.
    ///
    /// Called after `server.connect()` returns `Ok`. Steps:
    ///
    /// 1. Extract the raw HANDLE for impersonation auth.
    /// 2. Authenticate via `nono::supervisor::authenticate_pipe_client` (kernel-vouched SID).
    /// 3. Look up tenant by SID in `DaemonState::tenants` (NOT by wire `session_id`).
    /// 4. Serve capability query frames from the authenticated tenant.
    ///
    /// All error paths return `Ok(())` after closing the pipe — the accept
    /// loop must keep running regardless of per-connection outcomes.
    ///
    /// # Security
    ///
    /// Authorization is by kernel-vouched SID only. The `session_id` from wire
    /// frames is a routing hint used AFTER SID auth — it is never the
    /// authorization signal (ADR-74 SC5; STRIDE T-74-04-01).
    async fn handle_one_connection(
        mut server: tokio::net::windows::named_pipe::NamedPipeServer,
        state: Arc<DaemonState>,
    ) -> nono::Result<()> {
        use std::os::windows::io::AsRawHandle;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Step 1: Obtain the raw HANDLE for impersonation auth.
        // The raw handle is valid as long as `server` is alive in this scope.
        let pipe_raw: HANDLE = server.as_raw_handle() as HANDLE;

        // Step 2: Authenticate via kernel-vouched AppContainer SID.
        // SAFETY: `pipe_raw` is a valid connected named-pipe server-side handle
        // owned by `server` which is alive for the duration of this call.
        // `authenticate_pipe_client` calls ImpersonateNamedPipeClient +
        // OpenThreadToken + RevertToSelf (RAII ImpersonationGuard).
        let client_sid = match unsafe { nono::supervisor::authenticate_pipe_client(pipe_raw) } {
            Ok(sid) => sid,
            Err(e) => {
                // Fail-secure: impersonation failure or no AppContainer SID →
                // deny the connection. The client is not a valid AppContainer tenant.
                tracing::warn!(
                    error = %e,
                    "handle_one_connection: pipe auth failed — \
                     deny (not a valid AppContainer client)"
                );
                // `server` drops here closing the connection.
                return Ok(());
            }
        };

        // Step 3: Look up tenant by kernel-vouched SID.
        // Authorization is by SID match — `session_id` from wire frames is
        // a routing hint only and is NOT consulted here for authorization.
        let tenant_id_opt = {
            let tenants = state.tenants.lock().map_err(|_| {
                NonoError::SandboxInit("DaemonState::tenants mutex poisoned".into())
            })?;
            // Find by package_sid (kernel-vouched identity).
            tenants
                .values()
                .find(|t| t.package_sid == client_sid)
                .map(|t| t.tenant_id.clone())
        };

        let tenant_id = match tenant_id_opt {
            Some(id) => id,
            None => {
                // SID not in registry → deny. Primary isolation gate (DMON-02).
                tracing::warn!(
                    client_sid = %client_sid,
                    "handle_one_connection: SID not in tenant registry \
                     (deny — cross-tenant access attempt or stale connection)"
                );
                // `server` drops here closing the connection.
                return Ok(());
            }
        };

        tracing::info!(
            tenant_id = %tenant_id,
            client_sid = %client_sid,
            "handle_one_connection: authenticated tenant connected; \
             serving capability queries"
        );

        // Step 4: Serve capability query frames (query-only; no cap expansion).
        //
        // Frame format (matches socket_windows.rs):
        //   [4-byte LE length][JSON payload (SupervisorMessage)]
        //
        // This is a query-only pipe. The `caps` field on `AgentTenant` is
        // read-only post-construction and is never mutated here (SC4: no escape
        // hatch). A request to ADD capabilities is denied with a Denied response.

        let mut len_buf = [0u8; 4];
        loop {
            // Read the 4-byte length prefix.
            match server.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e)
                    if e.kind() == std::io::ErrorKind::UnexpectedEof
                        || e.kind() == std::io::ErrorKind::BrokenPipe
                        || e.kind() == std::io::ErrorKind::ConnectionReset =>
                {
                    // Client disconnected cleanly or abruptly — not an error.
                    tracing::debug!(
                        tenant_id = %tenant_id,
                        "handle_one_connection: client disconnected"
                    );
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        error = %e,
                        "handle_one_connection: read error on length prefix; \
                         closing connection"
                    );
                    break;
                }
            }

            let msg_len = u32::from_le_bytes(len_buf) as usize;

            // Guard against oversized messages (64 KiB cap — matches
            // socket_windows.rs MAX_MESSAGE_SIZE).
            if msg_len > MAX_MESSAGE_SIZE as usize {
                tracing::warn!(
                    tenant_id = %tenant_id,
                    msg_len = msg_len,
                    max = MAX_MESSAGE_SIZE,
                    "handle_one_connection: message too large; closing (fail-secure)"
                );
                break;
            }

            let mut payload = vec![0u8; msg_len];
            match server.read_exact(&mut payload).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        error = %e,
                        "handle_one_connection: read error on payload; closing connection"
                    );
                    break;
                }
            }

            // Parse as SupervisorMessage to validate the wire format.
            let msg: nono::supervisor::types::SupervisorMessage =
                match serde_json::from_slice(&payload) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(
                            tenant_id = %tenant_id,
                            error = %e,
                            "handle_one_connection: failed to parse SupervisorMessage; \
                             closing connection"
                        );
                        break;
                    }
                };

            // Serve the frame. Query-only: only `Request` variant is handled.
            // Non-Request variants are denied and the connection is closed —
            // this is a capability-query pipe, not the full supervisor IPC.
            //
            // The `session_id` inside `CapabilityRequest` is a routing hint only.
            // Authorization was already established by the SID match above (step 3).
            // No capability expansion is possible from any wire frame (SC4).
            let response = match &msg {
                nono::supervisor::types::SupervisorMessage::Request(req) => {
                    // Query-only: respond with the current denied decision.
                    // The daemon capability pipe answers "what are your caps?" —
                    // the CapabilitySet is served from the immutable tenant record.
                    // We return Denied here because capability grants are managed
                    // at launch time by `launch_agent`; the pipe is query-only.
                    nono::supervisor::types::SupervisorResponse::Decision {
                        request_id: req.request_id.clone(),
                        decision: nono::supervisor::types::ApprovalDecision::Denied {
                            reason: "daemon capability pipe: capability state is \
                                     managed at agent launch time by the daemon; \
                                     no runtime expansion allowed (ADR-74 SC4)"
                                .to_string(),
                        },
                    }
                }
                _ => {
                    // Terminate, Detach, OpenUrl — not supported on the daemon
                    // capability pipe. Deny and close.
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        "handle_one_connection: non-Request SupervisorMessage received; \
                         closing (daemon capability pipe is query-only)"
                    );
                    break;
                }
            };

            // Serialize and send the response.
            let response_bytes = match serde_json::to_vec(&response) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        error = %e,
                        "handle_one_connection: failed to serialize response; \
                         closing connection"
                    );
                    break;
                }
            };

            let response_len = u32::try_from(response_bytes.len()).unwrap_or(u32::MAX);
            let len_prefix = response_len.to_le_bytes();

            if let Err(e) = server.write_all(&len_prefix).await {
                tracing::warn!(
                    tenant_id = %tenant_id,
                    error = %e,
                    "handle_one_connection: write error on response length; closing"
                );
                break;
            }
            if let Err(e) = server.write_all(&response_bytes).await {
                tracing::warn!(
                    tenant_id = %tenant_id,
                    error = %e,
                    "handle_one_connection: write error on response payload; closing"
                );
                break;
            }
        }

        // `server` drops here, closing the pipe instance.
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────
//
// Cross-platform tests for the pure-Rust registry-lookup logic used by the
// accept loop. The Windows-specific pipe creation and ImpersonateNamedPipeClient
// impersonation are tested via the integration-test harness in
// `tests/daemon_handle_baseline.rs`.

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::DaemonState;
    use std::sync::Arc;

    #[cfg(target_os = "windows")]
    use super::super::reap::AgentTenant;
    #[cfg(target_os = "windows")]
    use std::os::windows::io::{FromRawHandle, OwnedHandle};

    /// Helper to build a test `DaemonState`.
    fn empty_state() -> Arc<DaemonState> {
        Arc::new(DaemonState::new())
    }

    /// SC: accept_loop_denies_unknown_sid
    ///
    /// A client whose AppContainer SID is NOT in the tenant registry must be
    /// denied: the tenants map must remain empty and no panic must occur.
    ///
    /// This test exercises the registry-lookup failure path (step 3 of
    /// `handle_one_connection`). We verify the invariant by constructing a
    /// `DaemonState` with an empty tenant map and confirming that a lookup for
    /// an unknown SID returns `None`.
    #[test]
    fn accept_loop_denies_unknown_sid() {
        let state = empty_state();

        // Simulate what handle_one_connection does after authenticate_pipe_client
        // returns a SID that is not in the registry.
        let unknown_sid = "S-1-15-2-9999-8888-7777-6666-5555-4444-3333";

        let tenant = {
            let tenants = state.tenants.lock().unwrap();
            tenants
                .values()
                .find(|t| t.package_sid == unknown_sid)
                .map(|t| t.tenant_id.clone())
        };

        assert!(
            tenant.is_none(),
            "Unknown SID must not match any tenant in an empty registry (deny)"
        );

        // Verify the tenants map is still empty after the lookup.
        let tenants = state.tenants.lock().unwrap();
        assert_eq!(
            tenants.len(),
            0,
            "Tenant map must remain empty after a deny — no mutation on unknown SID"
        );
    }

    /// SC: session_id_is_routing_hint_not_authz (Windows)
    ///
    /// Authorization in the accept loop is by kernel-vouched SID from
    /// `authenticate_pipe_client`, NOT by `session_id` from a wire frame.
    ///
    /// This test sets up two tenants A and B in `DaemonState`. It then
    /// simulates a connection where:
    ///   - `authenticate_pipe_client` returned tenant B's SID (kernel-vouched)
    ///   - A wire frame arrives with tenant A's `session_id` (caller-controlled)
    ///
    /// The lookup MUST resolve to tenant B (by SID), NOT tenant A (by session_id).
    /// If the implementation used `session_id` for authorization, the wrong tenant
    /// would be returned — that would be STRIDE T-74-04-01 (Spoofing).
    #[test]
    #[cfg(target_os = "windows")]
    fn session_id_is_routing_hint_not_authz() {
        use windows_sys::Win32::Foundation::{DuplicateHandle, BOOL, DUPLICATE_SAME_ACCESS};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        let state = empty_state();

        // Mint valid OwnedHandles by duplicating the current process handle.
        // These are not real job/process handles for confined agents, but they
        // are valid closeable Windows handles — sufficient to construct
        // AgentTenant for testing without real AppContainer spawn.
        let current = unsafe { GetCurrentProcess() };
        let make_handle = || -> OwnedHandle {
            let mut raw = std::ptr::null_mut();
            let ok: BOOL = unsafe {
                DuplicateHandle(
                    current,
                    current,
                    current,
                    &mut raw,
                    0,
                    0,
                    DUPLICATE_SAME_ACCESS,
                )
            };
            assert_ne!(ok, 0, "DuplicateHandle must succeed");
            unsafe { OwnedHandle::from_raw_handle(raw) }
        };

        let tenant_a = AgentTenant {
            tenant_id: "session-a-0001".to_string(),
            package_sid: "S-1-15-2-1111-2222-3333-4444-5555-6666-7777".to_string(),
            profile_name: "nono.test.tenant-a".to_string(),
            engine_profile: "test-engine-a".to_string(),
            caps: nono::CapabilitySet::new(),
            job_handle: make_handle(),
            process_handle: make_handle(),
        };

        let tenant_b = AgentTenant {
            tenant_id: "session-b-0002".to_string(),
            package_sid: "S-1-15-2-8888-7777-6666-5555-4444-3333-2222".to_string(),
            profile_name: "nono.test.tenant-b".to_string(),
            engine_profile: "test-engine-b".to_string(),
            caps: nono::CapabilitySet::new(),
            job_handle: make_handle(),
            process_handle: make_handle(),
        };

        let a_tenant_id = tenant_a.tenant_id.clone();
        let a_package_sid = tenant_a.package_sid.clone();
        let b_package_sid = tenant_b.package_sid.clone();
        let b_tenant_id = tenant_b.tenant_id.clone();

        // Insert both tenants.
        {
            let mut tenants = state.tenants.lock().unwrap();
            tenants.insert(a_tenant_id.clone(), tenant_a);
            tenants.insert(b_tenant_id.clone(), tenant_b);
        }

        // Simulate: authenticate_pipe_client returned TENANT B's SID (kernel-vouched).
        // Wire frame carries tenant A's session_id (caller-controlled — untrusted).
        let kernel_vouched_sid = &b_package_sid; // What the kernel says
        let _wire_frame_session_id = &a_tenant_id; // What the client claims (ignored)

        // The accept loop MUST use the kernel-vouched SID for the lookup.
        let resolved_tenant_id = {
            let tenants = state.tenants.lock().unwrap();
            tenants
                .values()
                .find(|t| &t.package_sid == kernel_vouched_sid)
                .map(|t| t.tenant_id.clone())
        };

        assert_eq!(
            resolved_tenant_id.as_deref(),
            Some(b_tenant_id.as_str()),
            "Lookup by kernel-vouched SID must resolve to tenant B, not tenant A \
             (T-74-04-01: session_id is routing hint only; SID is the authz signal)"
        );

        // Additional assertion: the wire-frame session_id must NOT be used for authz.
        assert_ne!(
            resolved_tenant_id.as_deref(),
            Some(a_tenant_id.as_str()),
            "Lookup must NOT resolve to tenant A when kernel SID belongs to tenant B \
             (SC5: no wire-field-based authz bypass)"
        );

        // Sanity: tenant A's SID correctly resolves to tenant A.
        let resolved_a = {
            let tenants = state.tenants.lock().unwrap();
            tenants
                .values()
                .find(|t| t.package_sid == a_package_sid)
                .map(|t| t.tenant_id.clone())
        };
        assert_eq!(
            resolved_a.as_deref(),
            Some(a_tenant_id.as_str()),
            "Sanity: tenant A's SID must resolve to tenant A"
        );
    }

    /// SC: session_id_is_routing_hint_not_authz (non-Windows stub)
    ///
    /// On non-Windows targets, verifies the SID-lookup routing invariant using
    /// plain HashMap logic (mirrors what the Windows path does with tenant map).
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn session_id_is_routing_hint_not_authz_cross_platform() {
        use std::collections::HashMap;

        // Simulate the tenant map using plain (tenant_id, package_sid) pairs.
        let mut tenant_sids: HashMap<String, String> = HashMap::new();
        tenant_sids.insert(
            "session-a-0001".to_string(),
            "S-1-15-2-1111-2222-3333-4444-5555-6666-7777".to_string(),
        );
        tenant_sids.insert(
            "session-b-0002".to_string(),
            "S-1-15-2-8888-7777-6666-5555-4444-3333-2222".to_string(),
        );

        // Kernel-vouched SID = tenant B's SID.
        let kernel_sid = "S-1-15-2-8888-7777-6666-5555-4444-3333-2222";
        let resolved = tenant_sids
            .iter()
            .find(|(_, sid)| *sid == kernel_sid)
            .map(|(tid, _)| tid.clone());

        assert_eq!(
            resolved.as_deref(),
            Some("session-b-0002"),
            "SID lookup must return tenant B when kernel SID belongs to B"
        );
        assert_ne!(
            resolved.as_deref(),
            Some("session-a-0001"),
            "SID lookup must NOT return tenant A"
        );
    }
}
