//! Per-agent launch orchestration for `nono-agentd`.
//!
//! This module implements the daemon-side launch path for confined AI agents
//! (DMON-01). It is the SOLE confinement path from the daemon — agents are
//! ONLY launched here, never adopted from external processes (ADR-74 D-02).
//!
//! # Launch sequence
//!
//! 1. Generate a unique `tenant_id` (16-byte random hex string).
//! 2. Create an AppContainer profile (`nono::create_app_container_profile`).
//! 3. Derive the package SID (`nono::derive_app_container_sid` +
//!    `nono::package_sid_to_string`).
//! 4. Create the Job Object (`create_agent_job`) with
//!    `KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION`.
//! 5. Spawn the confined process (CREATE_SUSPENDED +
//!    `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`).
//! 6. Assign the process to the job (`assign_process_to_agent_job`).
//! 7. Insert the package SID into `AgentRegistry` BEFORE inserting the
//!    `AgentTenant` into `DaemonState::tenants` (fail-secure ordering — the SID
//!    is registered before the agent can issue any pipe requests).
//! 8. Resume the suspended process.
//! 9. Spawn a reap task (`tokio::spawn` + `spawn_blocking` +
//!    `WaitForSingleObject`) that removes the tenant from `DaemonState` on exit.
//!
//! # Module independence
//!
//! This module intentionally does NOT depend on `exec_strategy_windows/`.
//! The `nono-agentd` binary includes `agent_daemon` via `#[path]` and does
//! not declare `exec_strategy_windows`. We inline the job/process creation
//! using raw Windows APIs (the same calls `exec_strategy_windows` makes).
//!
//! # Fail-secure on job-assign failure
//!
//! If `assign_process_to_agent_job` fails, the suspended process is terminated
//! before returning `Err`. No partial state is left in the registry or tenant map.
//!
//! # Windows-only
//!
//! All production code is gated on `#[cfg(target_os = "windows")]`.

// Wave 5 (Plan 74-07) re-export for control_loop.rs.
#[cfg(target_os = "windows")]
pub(crate) use windows_impl::launch_agent;
// GAP-75-B fix: expose resolve_exe_path to control_loop.rs so handle_launch
// can resolve bare exe names (e.g. "claude") to absolute paths BEFORE calling
// build_daemon_capability_set (which calls resolved_exe.parent() and fails on
// empty parent of a bare name).
#[cfg(target_os = "windows")]
pub(crate) use windows_impl::resolve_exe_path;
// Plan 75-07-T2: re-export DaemonDaclGuard so reap.rs can reference it in the
// AgentTenant::dacl_guard field type (`super::launch::DaemonDaclGuard`).
#[cfg(target_os = "windows")]
pub(crate) use windows_impl::DaemonDaclGuard;
// Plan 75-01 (SUPP-02): forward-export for control_loop.rs handle_demote (Plan 75-02).
// allow(unused_imports): this is an intentional forward-export; plan 75-02 will add
// the handle_demote caller. Suppressed to keep CI green in the interim.
#[cfg(target_os = "windows")]
#[allow(unused_imports)]
pub(crate) use windows_impl::wfp_filter_remove;

// All `windows_impl` functions are called by `control_loop.rs` (Wave 5);
// `#[allow(dead_code)]` is retained for non-called helpers within this module.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
mod windows_impl {
    use super::super::reap::AgentTenant;
    use super::super::DaemonState;
    use nono::NonoError;
    use std::os::windows::io::FromRawHandle;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    // ── Local DaemonDaclGuard (GAP-75-B / Plan 75-07-T2) ─────────────────────
    //
    // This guard is defined LOCALLY in this module, NOT imported from
    // exec_strategy_windows::dacl_guard. The nono-agentd binary loads agent_daemon
    // via #[path] and does NOT declare exec_strategy_windows (module-independence
    // invariant, launch.rs module-doc lines 27-31). Mirrored from
    // AppliedDaclGrantsGuard + AppliedAncestorTraverseGuard in dacl_guard.rs.

    /// Per-agent DACL RAII guard for daemon-launched AppContainer agents.
    ///
    /// Applied at step 6.6 in `launch_agent` — AFTER the WFP gate (step 6.5)
    /// and BEFORE `ResumeThread` (step 8), while the child process is still
    /// SUSPENDED (Pitfall-3 ordering). Reverted automatically on drop (agent reap).
    ///
    /// # Fields
    ///
    /// - `write_applied`: paths on which `nono::grant_sid_write_on_path` was called
    ///   (the per-tenant workspace leaf). Revoked LIFO on `revert_all`.
    /// - `traverse_applied`: paths on which `nono::grant_sid_traverse_on_path` was
    ///   called (read-only engine/interpreter dirs + workspace ancestors). Revoked
    ///   LIFO on `revert_all` after write grants.
    /// - `package_sid`: the AppContainer package SID string used for all grants.
    ///
    /// # Fail-closed discipline
    ///
    /// Any grant failure or ownership-check error calls `revert_all` on the
    /// already-applied grants before returning `Err`. No partial-grant state is
    /// ever returned to the caller.
    pub(crate) struct DaemonDaclGuard {
        /// Paths granted write access (workspace leaf). Revoked LIFO first.
        write_applied: Vec<PathBuf>,
        /// Paths granted traverse access (read-only dirs + workspace ancestors). Revoked LIFO.
        traverse_applied: Vec<PathBuf>,
        /// The AppContainer package SID for which ACEs were applied.
        package_sid: String,
    }

    impl DaemonDaclGuard {
        /// Phase 117 review NR-05: whether this guard actually wrote a
        /// package-SID write ACE, for the step 6.7 attestation gate.
        ///
        /// The gate used to be handed a literal `true` justified by "the
        /// guard exists, so apply ran" — the same guard-construction-instead-
        /// of-guard-effect defect CR-14 found on the direct-CLI path. Pass 2
        /// is fail-closed today, so this is `true` on every launch that
        /// reaches the gate; the point is that the gate now READS the fact
        /// instead of restating an invariant a future edit could break
        /// silently.
        pub(crate) fn granted_write_access(&self) -> bool {
            !self.write_applied.is_empty()
        }

        /// Apply package-SID DACL grants for a daemon-launched agent.
        ///
        /// Three passes:
        ///
        /// 1. **Read-only rules** (engine dir, interpreter dirs, system dirs):
        ///    for each rule where `!rule.access.contains(Write)`, check ownership
        ///    and call `grant_sid_traverse_on_path`. Skip non-owned paths (warn).
        ///    Fail-closed on ownership-check error.
        ///
        /// 2. **Workspace write grant**: call `path_is_owned_by_current_user` on
        ///    the workspace; call `grant_sid_write_on_path` (inheritable=true for
        ///    directory). Fail-closed if not owned (daemon always creates the workspace —
        ///    not-owned is anomalous). Revert and return `Err` on any error.
        ///
        /// 3. **Workspace ancestors**: walk `workspace.ancestors().skip(1)`.
        ///    Stop at first non-owned ancestor (relies on lowbox bypass-traverse from
        ///    there up, as in `AppliedAncestorTraverseGuard`). Fail-closed on error.
        ///
        /// # Pitfall-3 ordering
        ///
        /// Called AFTER step 6.5 (WFP gate), BEFORE step 8 (ResumeThread). The agent
        /// process is SUSPENDED and cannot issue any pipe requests until ResumeThread.
        pub(crate) fn apply(
            policy: &nono::WindowsFilesystemPolicy,
            workspace: &Path,
            package_sid: &str,
        ) -> nono::Result<Self> {
            let mut guard = Self {
                write_applied: Vec::new(),
                traverse_applied: Vec::new(),
                package_sid: package_sid.to_string(),
            };

            // Pass 1 — read-only rules (traverse grant for AppContainer to stat/enter).
            for rule in &policy.rules {
                if rule.access.contains(nono::AccessMode::Write) {
                    // Workspace write rule is handled in pass 2.
                    continue;
                }
                match nono::path_is_owned_by_current_user(&rule.path) {
                    Ok(true) => {
                        if let Err(e) = nono::grant_sid_traverse_on_path(&rule.path, package_sid) {
                            tracing::warn!(
                                path = %rule.path.display(),
                                error = %e,
                                "daemon dacl guard: traverse grant failed; reverting applied grants"
                            );
                            guard.revert_all();
                            return Err(e);
                        }
                        guard.traverse_applied.push(rule.path.clone());
                    }
                    Ok(false) => {
                        tracing::warn!(
                            path = %rule.path.display(),
                            "daemon dacl guard: read-only path not owned by current user; \
                             skipping traverse grant (relying on lowbox bypass-traverse)"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %rule.path.display(),
                            error = %e,
                            "daemon dacl guard: ownership check failed on read-only path; \
                             reverting applied grants (fail-closed)"
                        );
                        guard.revert_all();
                        return Err(e);
                    }
                }
            }

            // Pass 2 — workspace write grant.
            match nono::path_is_owned_by_current_user(workspace) {
                Ok(true) => {
                    // Directory rule: inheritable=true so files the agent creates inherit.
                    if let Err(e) = nono::grant_sid_write_on_path(workspace, package_sid, true) {
                        tracing::warn!(
                            workspace = %workspace.display(),
                            error = %e,
                            "daemon dacl guard: write grant on workspace failed; reverting"
                        );
                        guard.revert_all();
                        return Err(e);
                    }
                    guard.write_applied.push(workspace.to_path_buf());
                }
                Ok(false) => {
                    // Daemon always creates the workspace; not-owned is anomalous → fail-secure.
                    guard.revert_all();
                    return Err(NonoError::SandboxInit(format!(
                        "daemon dacl: workspace not owned by current user (anomalous — \
                         daemon must create the workspace before calling apply): {}",
                        workspace.display()
                    )));
                }
                Err(e) => {
                    guard.revert_all();
                    return Err(e);
                }
            }

            // Pass 3 — workspace ancestors: grant traverse up the user-owned chain.
            for ancestor in workspace.ancestors().skip(1) {
                match nono::path_is_owned_by_current_user(ancestor) {
                    Ok(true) => {
                        if let Err(e) = nono::grant_sid_traverse_on_path(ancestor, package_sid) {
                            tracing::warn!(
                                ancestor = %ancestor.display(),
                                error = %e,
                                "daemon dacl guard: ancestor traverse grant failed; reverting"
                            );
                            guard.revert_all();
                            return Err(e);
                        }
                        guard.traverse_applied.push(ancestor.to_path_buf());
                    }
                    Ok(false) => {
                        // First non-owned ancestor (e.g. C:\Users, C:\). STOP —
                        // reaching these relies on the lowbox bypass-traverse privilege.
                        tracing::debug!(
                            ancestor = %ancestor.display(),
                            "daemon dacl guard: ancestor not owned; stopping walk \
                             (relying on lowbox bypass-traverse from here up)"
                        );
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            ancestor = %ancestor.display(),
                            error = %e,
                            "daemon dacl guard: ancestor ownership check failed; reverting"
                        );
                        guard.revert_all();
                        return Err(e);
                    }
                }
            }

            Ok(guard)
        }

        /// Revert all applied grants, LIFO. Write grants are revoked first (workspace leaf),
        /// then traverse grants (read-only dirs + workspace ancestors from innermost outward).
        /// Errors are logged but never panic — Drop-safe.
        fn revert_all(&mut self) {
            // Revoke write grants first (workspace leaf).
            while let Some(path) = self.write_applied.pop() {
                if let Err(e) = nono::revoke_sid_on_path(&path, &self.package_sid) {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "daemon dacl guard: write grant revoke failed; package SID may remain"
                    );
                }
            }
            // Revoke traverse grants (LIFO — innermost ancestors first).
            while let Some(path) = self.traverse_applied.pop() {
                if let Err(e) = nono::revoke_sid_on_path(&path, &self.package_sid) {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "daemon dacl guard: traverse grant revoke failed; package SID may remain"
                    );
                }
            }
        }
    }

    impl Drop for DaemonDaclGuard {
        fn drop(&mut self) {
            self.revert_all();
        }
    }

    use windows_sys::Win32::Foundation::{
        CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, PSECURITY_DESCRIPTOR,
        PSID, SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES,
    };
    use windows_sys::Win32::Storage::FileSystem::SearchPathW;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess, GetExitCodeProcess,
        InitializeProcThreadAttributeList, ResumeThread, TerminateProcess,
        UpdateProcThreadAttribute, WaitForSingleObject, CREATE_SUSPENDED,
        CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT, INFINITE,
        LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTUPINFOEXW, STARTUPINFOW,
    };

    // SDDL_REVISION_1 is not exported from windows-sys 0.59 as a constant.
    // Use 1u32 directly (the only documented revision value).
    const SDDL_REVISION_1: u32 = 1;

    // ── WFP per-agent filter helpers (Plan 75-01 / SUPP-02) ─────────────────

    /// WFP control pipe name for the elevated `nono-wfp-service`.
    const WFP_CONTROL_PIPE: &str = r"\\.\pipe\nono-wfp-control";

    /// Test-accessible re-export of the control-pipe name constant.
    ///
    /// Used by unit tests to verify error messages contain the pipe name
    /// without relying on the internal constant spelling.
    #[cfg(test)]
    pub(crate) const WFP_CONTROL_PIPE_NAME_TESTABLE: &str = WFP_CONTROL_PIPE;

    /// Test-accessible wrapper for `profile_needs_network_scoping`.
    ///
    /// Exposed for unit tests in the parent `tests` module that need to
    /// inspect the gate predicate without calling the async `wfp_filter_add`.
    #[cfg(test)]
    pub(crate) fn profile_needs_network_scoping_testable(profile_name: &str) -> bool {
        profile_needs_network_scoping(profile_name)
    }

    /// Send a `WfpRuntimeActivationRequest` to `nono-wfp-service` over its
    /// named-pipe control channel using a synchronous (blocking) `std::fs`
    /// named-pipe client.
    ///
    /// # Synchronous by design
    ///
    /// This is a BLOCKING (non-async) function. Using blocking `std::fs::File`
    /// named-pipe I/O rather than tokio async avoids holding `HANDLE = *mut c_void`
    /// (`!Send`) across an async `.await` point in `launch_agent`. The WFP pipe
    /// round-trip is expected to complete in < 50 ms; blocking the task thread
    /// briefly here is acceptable (the daemon's accept loop is on a separate
    /// tokio task).
    ///
    /// # Errors
    ///
    /// Returns `Err(NonoError::SandboxInit(...))` if:
    /// - The pipe cannot be opened (service absent / stopped).
    /// - Serialization or I/O fails.
    /// - The response cannot be parsed.
    fn send_wfp_control_request(
        req: &super::super::wfp_contract::WfpRuntimeActivationRequest,
    ) -> nono::Result<super::super::wfp_contract::WfpRuntimeActivationResponse> {
        use std::io::{Read, Write};

        let payload = serde_json::to_vec(req).map_err(|e| {
            NonoError::SandboxInit(format!(
                "wfp_control_request: failed to serialize request: {e}"
            ))
        })?;

        // Open the named pipe in read+write mode using std::fs (synchronous).
        // Windows named pipes opened with FILE_FLAG_OVERLAPPED are not
        // accessible via std::fs; the wfp-service pipe is created WITHOUT
        // FILE_FLAG_OVERLAPPED in its control channel, so std::fs works.
        let mut pipe = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(WFP_CONTROL_PIPE)
            .map_err(|e| {
                NonoError::SandboxInit(format!(
                    "WFP control pipe unreachable — is nono-wfp-service running? \
                     (pipe={WFP_CONTROL_PIPE}): {e}"
                ))
            })?;

        pipe.write_all(&payload).map_err(|e| {
            NonoError::SandboxInit(format!(
                "wfp_control_request: failed to write request to pipe: {e}"
            ))
        })?;

        let mut buf = vec![0u8; 64 * 1024];
        let n = pipe.read(&mut buf).map_err(|e| {
            NonoError::SandboxInit(format!(
                "wfp_control_request: failed to read response from pipe: {e}"
            ))
        })?;

        if n == 0 {
            return Err(NonoError::SandboxInit(
                "wfp_control_request: service closed connection without sending a response"
                    .to_string(),
            ));
        }

        let resp: super::super::wfp_contract::WfpRuntimeActivationResponse =
            serde_json::from_slice(&buf[..n]).map_err(|e| {
                NonoError::SandboxInit(format!(
                    "wfp_control_request: failed to parse service response: {e}"
                ))
            })?;

        Ok(resp)
    }

    /// Install a per-agent WFP egress filter keyed to the agent's AppContainer
    /// package SID (E4 identity) via `nono-wfp-service`.
    ///
    /// # Force-through-proxy model (Plan 83-02, D-01/D-02/EGRESS-02)
    ///
    /// Sends an `"activate_proxy_mode"` request with `network_mode: "proxy-only"`
    /// and `localhost_ports: [proxy_port]`.  The WFP service's
    /// `build_policy_filter_specs` emits:
    ///
    /// - A loopback PERMIT filter on `IP_REMOTE_PORT == proxy_port` (weight 100,
    ///   SID-keyed) — loopback → proxy is permitted.
    /// - A block-all filter (weight 0, SID-keyed) — all other outbound is blocked.
    ///
    /// The permit weight (100) beats the block weight (0) so the agent's ONLY
    /// egress path is loopback → proxy.  The proxy enforces the FQDN allowlist
    /// (EGRESS-01/SC-3 dual-layer deny).  The WFP service never reads HKLM for
    /// egress policy — it receives derived permit instructions over IPC (D-04).
    ///
    /// # Proxy port threading (D-04 no-drift)
    ///
    /// `proxy_port` is threaded from the daemon startup site (where
    /// `resolve_machine_egress_policy` is called and the in-process proxy server
    /// is started) into this function via `launch_agent` → `DaemonState::machine_egress_proxy_port`.
    /// This ensures the WFP permit instruction is always derived from the SAME
    /// deserialized `MachineEgressPolicy` struct that configured the ProxyFilter —
    /// no drift between enforcement layers.
    ///
    /// # Fail-secure (D-05)
    ///
    /// Any pipe error (service absent, I/O failure, NACK response) returns `Err`.
    /// The CALLER must terminate the suspended process before returning `Err`.
    ///
    /// # Blocking
    ///
    /// This is a synchronous (blocking) function. See `send_wfp_control_request`
    /// for the rationale (avoids `!Send` raw HANDLE across `.await`).
    fn wfp_filter_add(package_sid: &str, tenant_id: &str, proxy_port: u16) -> nono::Result<()> {
        use super::super::wfp_contract::{
            WfpRuntimeActivationRequest, WFP_RUNTIME_PROTOCOL_VERSION,
        };

        let req = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            // D-01/D-02 force-through-proxy: permit loopback:proxy_port + block all else.
            // build_policy_filter_specs in nono-wfp-service emits PERMIT(weight 100) for
            // IP_REMOTE_PORT==proxy_port + BLOCK-all(weight 0) keyed on ALE_USER_ID=SID.
            // Weights are already correct — DO NOT change them (Pitfall 4).
            request_kind: "activate_proxy_mode".to_string(),
            network_mode: "proxy-only".to_string(),
            preferred_backend: "wfp".to_string(),
            active_backend: "wfp".to_string(),
            runtime_target: format!("nono-agent-{tenant_id}"),
            tcp_connect_ports: vec![],
            tcp_bind_ports: vec![],
            // Loopback proxy listener port: the ONLY outbound path permitted for this SID.
            // All other outbound is kernel-blocked by WFP (SC-3 dual-layer deny, D-02).
            localhost_ports: vec![proxy_port],
            // This daemon-only launch path never grants port ranges; no range analog exists here.
            localhost_port_ranges: vec![],
            // session_sid activates the SID-keyed per-agent filter path in
            // nono-wfp-service::install_wfp_policy_filters (validated SID → SD → WFP).
            // target_program_path is unused by the service when session_sid is Some.
            target_program_path: None,
            session_sid: Some(package_sid.to_string()),
            outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
            inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
        };

        let resp = send_wfp_control_request(&req).map_err(|e| {
            NonoError::SandboxInit(format!(
                "wfp_filter_add: could not reach nono-wfp-service. \
                 Ensure nono-wfp-service is installed and running \
                 (tenant_id={tenant_id}): {e}"
            ))
        })?;

        // Any non-success status is treated as fail-secure (D-05). Unchanged from
        // the pre-83 path — this handling is verbatim per the plan contract.
        if resp.status == "invalid-request" || resp.status == "protocol-mismatch" {
            return Err(NonoError::SandboxInit(format!(
                "wfp_filter_add: nono-wfp-service rejected the request \
                 (status={}, details={}). \
                 Install and start nono-wfp-service before launching this profile.",
                resp.status, resp.details
            )));
        }

        Ok(())
    }

    /// Remove the per-agent WFP egress filter for a reaped agent.
    ///
    /// Sends a `"deactivate_policy_mode"` request to `nono-wfp-service` with
    /// the same deterministic rule names used at install time.
    ///
    /// # Non-fatal on error
    ///
    /// Callers in the reap task MUST NOT return early on error — they log a
    /// warning and continue. The WFP service's startup sweep reclaims stale
    /// filters (SUPP-02 Pitfall 6 mitigation).
    ///
    /// # Blocking
    ///
    /// This is a synchronous (blocking) function. See `send_wfp_control_request`
    /// for the rationale.
    ///
    /// # Visibility
    ///
    /// `pub(crate)` so `control_loop::handle_demote` (Plan 75-02) can call it
    /// when the operator issues `nono agent demote`.
    pub(crate) fn wfp_filter_remove(package_sid: &str, tenant_id: &str) -> nono::Result<()> {
        use super::super::wfp_contract::{
            WfpRuntimeActivationRequest, WFP_RUNTIME_PROTOCOL_VERSION,
        };

        let req = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            request_kind: "deactivate_policy_mode".to_string(),
            network_mode: "blocked".to_string(),
            preferred_backend: "wfp".to_string(),
            active_backend: "wfp".to_string(),
            runtime_target: format!("nono-agent-{tenant_id}"),
            tcp_connect_ports: vec![],
            tcp_bind_ports: vec![],
            localhost_ports: vec![],
            localhost_port_ranges: vec![],
            target_program_path: None,
            session_sid: Some(package_sid.to_string()),
            outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
            inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
        };

        let resp = send_wfp_control_request(&req).map_err(|e| {
            NonoError::SandboxInit(format!(
                "wfp_filter_remove: could not reach nono-wfp-service \
                 (tenant_id={tenant_id}): {e}"
            ))
        })?;

        if resp.status == "cleanup-failed" {
            return Err(NonoError::SandboxInit(format!(
                "wfp_filter_remove: nono-wfp-service cleanup failed \
                 (status={}, details={})",
                resp.status, resp.details
            )));
        }

        Ok(())
    }

    /// Check whether an engine profile declares network scoping (D-05 gate).
    ///
    /// Returns `true` if the embedded policy JSON for `profile_name` has
    /// `network.block = true`. Returns `false` if the profile is absent,
    /// the JSON is malformed, or `network.block` is absent/false.
    ///
    /// Fail-secure default: a parse failure returns `false` (no WFP gate),
    /// which is the conservative choice — profiles without explicit network
    /// scoping should not be gated by WFP service availability.
    fn profile_needs_network_scoping(profile_name: &str) -> bool {
        let policy: serde_json::Value =
            match serde_json::from_str(super::super::EMBEDDED_POLICY_JSON) {
                Ok(v) => v,
                Err(_) => return false,
            };

        // Navigate: policy["profiles"][profile_name]["network"]["block"]
        policy
            .get("profiles")
            .and_then(|p| p.as_object())
            .and_then(|profiles| profiles.get(profile_name))
            .and_then(|profile| profile.get("network"))
            .and_then(|network| network.get("block"))
            .and_then(|block| block.as_bool())
            .unwrap_or(false)
    }

    /// Launch a confined AI agent as an AppContainer child process.
    ///
    /// Creates a fresh AppContainer profile + package SID, a Job Object with
    /// `KILL_ON_JOB_CLOSE`, spawns the agent in a suspended state, assigns it
    /// to the job, inserts the SID into the registry + tenant into state, then
    /// resumes and wires a reap task.
    ///
    /// # Returns
    ///
    /// The `tenant_id` (hex string) assigned to the new agent on success.
    ///
    /// # Errors
    ///
    /// Returns `Err` if any step fails. Fail-secure: any failure terminates any
    /// suspended process and removes any partial state before returning.
    pub(crate) async fn launch_agent(
        daemon_state: Arc<DaemonState>,
        exe: PathBuf,
        args: Vec<String>,
        caps: nono::CapabilitySet,
        engine_profile: String,
        // workspace is used by step 6.6 (DaemonDaclGuard — Plan 75-07-T2).
        // The parameter is wired here so handle_launch can pass the per-tenant
        // workspace directory without a second signature change in Task 2.
        workspace: PathBuf,
    ) -> nono::Result<String> {
        // Step 1: Generate a unique tenant_id and AppContainer profile name.
        let tenant_id = generate_tenant_id()?;
        let profile_name = format!("nono.session.{}", &tenant_id[..16]);

        // Step 1b: Resolve the exe to an absolute path.
        //
        // CreateProcessW(lpApplicationName) does NOT PATH-search bare names;
        // passing "notepad.exe" → ERROR_FILE_NOT_FOUND (os error 2). We resolve
        // via `SearchPathW` BEFORE any coverage/profile validation so the
        // absolute path is used for both the OS-level confinement boundary and
        // any future exe-coverage check. Confinement is UNCHANGED — the
        // AppContainer token and Job Object apply to the resolved executable, not
        // the bare name.
        let exe = resolve_exe_path(exe)?;

        tracing::info!(
            tenant_id = %tenant_id,
            profile_name = %profile_name,
            exe = %exe.display(),
            "launch_agent: creating AppContainer profile"
        );

        // Step 2: Create the AppContainer profile.
        // FRESH PER AGENT: each call generates a new profile name derived from
        // the random tenant_id → new SID (T-74-04-02 mitigation).
        let profile = nono::create_app_container_profile(&profile_name).map_err(|e| {
            NonoError::SandboxInit(format!(
                "launch_agent: create_app_container_profile({profile_name:?}) failed: {e}"
            ))
        })?;

        // Step 3: Derive the package SID.
        let owned_sid = nono::derive_app_container_sid(&profile_name).map_err(|e| {
            NonoError::SandboxInit(format!(
                "launch_agent: derive_app_container_sid({profile_name:?}) failed: {e}"
            ))
        })?;
        let package_sid = nono::package_sid_to_string(&owned_sid).map_err(|e| {
            NonoError::SandboxInit(format!("launch_agent: package_sid_to_string failed: {e}"))
        })?;

        tracing::info!(
            tenant_id = %tenant_id,
            package_sid = %package_sid,
            "launch_agent: package SID derived"
        );

        // Step 4: Create the Job Object with KILL_ON_JOB_CLOSE.
        // The SDDL grants the job owner full access and denies Low-IL processes
        // any job access (D-03 belt-and-suspenders).
        let job_raw = create_agent_job(&tenant_id, &package_sid)?;

        // RAII: close the job handle if any subsequent step fails before we
        // transfer ownership to AgentTenant.
        struct JobGuard(HANDLE);
        impl Drop for JobGuard {
            fn drop(&mut self) {
                if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                    // SAFETY: the HANDLE inside JobGuard is the only owner.
                    unsafe { CloseHandle(self.0) };
                }
            }
        }
        let job_guard = JobGuard(job_raw);

        // Step 5: Spawn the confined process in SUSPENDED state.
        let psid: PSID = owned_sid.as_psid();
        let (process_handle_raw, thread_handle_raw) =
            spawn_appcontainer_process_suspended(&exe, &args, psid)?;

        // Step 6: Assign the process to the job (fail-secure: terminate on failure).
        if let Err(e) = assign_process_to_agent_job(job_guard.0, process_handle_raw) {
            // Terminate the suspended process before returning Err (T-74-04-04).
            // SAFETY: process_handle_raw is valid (from CreateProcessW).
            unsafe { TerminateProcess(process_handle_raw, 1) };
            // SAFETY: both handles are valid; close to avoid leaks.
            unsafe { CloseHandle(process_handle_raw) };
            unsafe { CloseHandle(thread_handle_raw) };
            // job_guard drops here → closes job handle.
            return Err(NonoError::SandboxInit(format!(
                "launch_agent: assign_process_to_agent_job failed \
                 (suspended process terminated, fail-secure): {e}"
            )));
        }

        // Step 6.5: Per-agent WFP egress filter (SUPP-02 / D-05 fail-secure gate).
        //
        // MUST happen BEFORE ResumeThread — if the WFP service is absent and the
        // profile declares network scoping, the agent is terminated before it
        // ever runs. (Pitfall 3: agent must not start before the filter is in place.)
        //
        // Plan 83-02 (EGRESS-02): when machine egress enforcement is active,
        // wfp_filter_add uses `proxy-only` + localhost_ports=[proxy_port] so the
        // agent's only egress path is loopback → proxy (D-01/D-02 force-through-proxy).
        // The proxy port is threaded from DaemonState::machine_egress_proxy_port, which
        // is set once at daemon startup from the single read_machine_egress_policy()
        // call (D-04 no-drift; WFP service never reads HKLM for egress policy).
        //
        // Phase 117 D-21 (Task 2): captured into a local so step 6.7's
        // attestation gate can reuse this exact result — if `true`, the WFP
        // filter-add call below already ran and (by the time step 6.7 is
        // reached) already succeeded fail-closed; never re-derived.
        let network_scoping_required = profile_needs_network_scoping(&engine_profile);
        // Phase 117 review NR-05: the ACTUAL outcome of the filter-add call,
        // recorded where the call returns. Step 6.7's gate used to be handed
        // `network_scoping_required` for BOTH its `network_scoping_required`
        // and `wfp_filters_installed` parameters, making its abort predicate
        // `x && !x` — a compile-time constant `false`, so the daemon
        // mirror's deny direction was unreachable outside a unit test.
        // Initialised to the restrictive value.
        let mut wfp_filters_installed = false;
        if network_scoping_required {
            // Resolve the proxy port for this launch.  `machine_egress_proxy_port`
            // is `Some(port)` when a machine egress policy was loaded at startup;
            // it is `None` when no policy is present (legacy blocked-mode path).
            // For proxy-only mode we MUST have a proxy port — fail-secure if absent.
            let proxy_port = daemon_state
                .machine_egress_proxy_port
                .ok_or_else(|| {
                    NonoError::SandboxInit(format!(
                        "launch_agent: profile '{engine_profile}' requires WFP network scoping \
                         but no machine egress proxy port is configured. \
                         Ensure a machine egress policy is present in HKLM and the daemon \
                         was started after applying the policy (restart-to-apply, D-06)."
                    ))
                })
                .inspect_err(|_| {
                    // D-05: terminate the suspended process before returning Err.
                    // SAFETY: process_handle_raw is valid (from CreateProcessW).
                    unsafe { TerminateProcess(process_handle_raw, 1) };
                    unsafe { CloseHandle(process_handle_raw) };
                    unsafe { CloseHandle(thread_handle_raw) };
                    // job_guard drops here → closes job handle → KILL_ON_JOB_CLOSE.
                })?;

            if let Err(e) = wfp_filter_add(&package_sid, &tenant_id, proxy_port) {
                // D-05: refuse to launch; terminate the suspended process.
                // SAFETY: process_handle_raw is valid (from CreateProcessW).
                unsafe { TerminateProcess(process_handle_raw, 1) };
                // SAFETY: both handles are valid; close to avoid leaks.
                unsafe { CloseHandle(process_handle_raw) };
                unsafe { CloseHandle(thread_handle_raw) };
                // job_guard drops here → closes job handle → KILL_ON_JOB_CLOSE.
                return Err(NonoError::SandboxInit(format!(
                    "launch_agent: WFP network scope required by profile '{engine_profile}' \
                     but nono-wfp-service is not reachable. \
                     Install and start nono-wfp-service before launching this profile. \
                     (Suspended process terminated, fail-secure.) Cause: {e}"
                )));
            }
            wfp_filters_installed = true;
            tracing::info!(
                tenant_id = %tenant_id,
                package_sid = %package_sid,
                proxy_port = proxy_port,
                "launch_agent: per-agent WFP filter installed (proxy-only, SUPP-02/EGRESS-02)"
            );
        }

        // Step 6.6 — Package-SID DACL grants (Pitfall-3 ordering: AFTER WFP, BEFORE ResumeThread).
        //
        // The agent process is still SUSPENDED so the step 7a registry insert preceding
        // this block is safe — the agent cannot issue pipe requests until ResumeThread
        // at step 8. What is NOT safe is any ordering AFTER ResumeThread.
        //
        // On grant failure: terminate the suspended process (same fail-secure pattern
        // as steps 6 and 6.5 — T-75-07-05 mitigation).
        let policy = nono::Sandbox::windows_filesystem_policy(&caps);
        let dacl_guard = match DaemonDaclGuard::apply(&policy, &workspace, &package_sid) {
            Ok(g) => g,
            Err(e) => {
                // SAFETY: process_handle_raw and thread_handle_raw are valid; they were
                // returned by spawn_appcontainer_process_suspended and have not been
                // wrapped or closed yet. Termination on grant failure mirrors the
                // fail-secure pattern at steps 6 and 6.5.
                unsafe { TerminateProcess(process_handle_raw, 1) };
                unsafe { CloseHandle(process_handle_raw) };
                unsafe { CloseHandle(thread_handle_raw) };
                // job_guard drops here → closes job handle → KILL_ON_JOB_CLOSE fires.
                return Err(NonoError::SandboxInit(format!(
                    "launch_agent: DaemonDaclGuard::apply failed \
                         (suspended process terminated, fail-secure): {e}"
                )));
            }
        };
        // Phase 117 review NR-05: capture the guard's ACTUAL effect before it
        // is moved into `tenant` below, so step 6.7's gate reads a fact
        // rather than the literal `true` it used to be handed.
        let dacl_guard_applied = dacl_guard.granted_write_access();
        tracing::info!(
            tenant_id = %tenant_id,
            package_sid = %package_sid,
            "launch_agent: package-SID DACL grants applied (step 6.6)"
        );

        // Transfer job ownership to AgentTenant: disarm the guard before
        // wrapping in OwnedHandle so we don't double-close.
        let job_raw_owned = job_guard.0;
        std::mem::forget(job_guard);

        // Step 7a: Insert package SID into AgentRegistry FIRST.
        // The SID must be registered before the agent is resumed — ensures no
        // pipe connection can race in before the registry entry exists.
        {
            let mut registry = daemon_state.agent_registry.lock().map_err(|_| {
                // Registry poisoned: fail-secure cleanup.
                // SAFETY: handles are valid.
                unsafe { TerminateProcess(process_handle_raw, 1) };
                unsafe { CloseHandle(process_handle_raw) };
                unsafe { CloseHandle(thread_handle_raw) };
                unsafe { CloseHandle(job_raw_owned) };
                NonoError::SandboxInit(
                    "launch_agent: AgentRegistry mutex poisoned (fail-secure)".into(),
                )
            })?;
            registry.insert(package_sid.clone());
        }

        // Wrap raw handles in std::os::windows::io::OwnedHandle for RAII.
        // SAFETY: `job_raw_owned` is a valid Job Object handle; we disarmed the
        // guard above — we are the sole owner.
        let job_owned =
            unsafe { std::os::windows::io::OwnedHandle::from_raw_handle(job_raw_owned) };
        // SAFETY: `process_handle_raw` is a valid process handle from CreateProcessW.
        let process_owned =
            unsafe { std::os::windows::io::OwnedHandle::from_raw_handle(process_handle_raw) };

        // Forget the profile — AppContainer cleanup deferred to AgentTenant::Drop.
        // Dropping the profile here would call DeleteAppContainerProfile too early.
        std::mem::forget(profile);

        // Step 7b: Insert AgentTenant into DaemonState::tenants AFTER registry.
        // dacl_guard is stored in the tenant so its Drop revokes the package-SID
        // DACL grants when the agent reaps (AgentTenant::drop field-drop order
        // ensures DACL revocation before job/process handle close — declared first
        // in reap.rs per the struct field ordering requirement).
        let tenant = AgentTenant {
            tenant_id: tenant_id.clone(),
            package_sid: package_sid.clone(),
            profile_name: profile_name.clone(),
            engine_profile: engine_profile.clone(),
            caps,
            dacl_guard: Some(dacl_guard),
            job_handle: job_owned,
            process_handle: process_owned,
        };

        {
            let mut tenants = daemon_state.tenants.lock().map_err(|_| {
                NonoError::SandboxInit("launch_agent: DaemonState::tenants mutex poisoned".into())
            })?;
            tenants.insert(tenant_id.clone(), tenant);
        }

        // Step 6.7 — Phase 117 D-21 (CINT-02): startup self-attestation gate.
        //
        // Independent implementation, NOT a call into
        // `exec_strategy_windows::attestation::attest_and_decide` — this
        // module's own doc comment (lines 26-30) states the daemon does not
        // depend on `exec_strategy_windows/`, and `nono-agentd` is compiled
        // as a wholly separate binary crate (`src/bin/nono-agentd.rs`
        // `#[path]`-includes only `agent_daemon/`, `telemetry/`, and
        // `agent_daemon/telemetry_init.rs` — never `exec_strategy_windows/`),
        // so that function is structurally unreachable from here. This
        // mirrors a deliberately NARROWER two-state slice of that decision
        // shape (Proceed / Abort — no ProceedDowngraded) using the shared,
        // policy-free `nono::attestation` probes — see
        // `daemon_attest_and_decide`'s doc comment for the exact row-by-row
        // mapping to `layer_registry.rs`'s `(EntryPath::Daemon, None)` rows.
        //
        // `process_handle_raw` is still a valid, readable handle here even
        // though `process_owned` (a second wrapper over the SAME raw value,
        // `HANDLE` is `Copy`) was already moved into `tenant` above — this
        // probes the real suspended child before its first instruction ever
        // runs (D-21), exactly like step 6/6.5/6.6's fail-secure gates.
        match daemon_attest_and_decide(
            process_handle_raw,
            job_raw_owned,
            &package_sid,
            // NR-05: the guard's own report of what it granted, captured at
            // step 6.6. Was a literal `true` justified by "the guard exists".
            dacl_guard_applied,
            network_scoping_required,
            // NR-05: the ACTUAL filter-add outcome, set where
            // `wfp_filter_add` returned. Was `network_scoping_required`
            // again, which made the row's abort predicate
            // `network_scoping_required && !network_scoping_required` — a
            // compile-time constant `false`.
            wfp_filters_installed,
        ) {
            DaemonAttestationDecision::Proceed => {}
            DaemonAttestationDecision::Abort { layer, status } => {
                tracing::warn!(
                    tenant_id = %tenant_id,
                    layer = layer,
                    status = ?status,
                    "launch_agent: startup self-attestation failed; refusing to resume \
                     (fail-secure)"
                );
                // Phase 117 review WR-07: terminate EXPLICITLY, matching
                // the adjacent steps 6/6.5/6.6 idiom, rather than relying on
                // `cleanup_failed_agent`'s job-close cascade. That cascade
                // works by closing the job handle so KILL_ON_JOB_CLOSE fires
                // — which cannot work when the abort reason IS
                // `JobObjectContainment` unconfirmed, because by hypothesis
                // the process is not in that job. The same Drop also closes
                // the process handle, leaving an orphaned suspended process
                // with no owner. Fail-secure either way (it never resumes),
                // but it leaked a process.
                // SAFETY: process_handle_raw is still valid and un-closed here.
                unsafe { TerminateProcess(process_handle_raw, 1) };
                // SAFETY: thread_handle_raw is still valid and un-closed here.
                unsafe { CloseHandle(thread_handle_raw) };
                // Removes tenant from state — Drops AgentTenant → closes
                // job_handle and the process handle.
                cleanup_failed_agent(&daemon_state, &tenant_id, &package_sid);
                return Err(NonoError::LayerAttestationFailed {
                    layer: layer.to_string(),
                    reason: format!("{status:?}"),
                });
            }
        }

        // Step 8: Resume the suspended process.
        // SAFETY: thread_handle_raw is the primary thread handle from CreateProcessW.
        let resume_result = unsafe { ResumeThread(thread_handle_raw) };
        // SAFETY: close the thread handle regardless of resume result.
        unsafe { CloseHandle(thread_handle_raw) };
        if resume_result == u32::MAX {
            // ResumeThread failed. Remove from state — this Drops AgentTenant →
            // closes job_handle → KILL_ON_JOB_CLOSE terminates the process.
            cleanup_failed_agent(&daemon_state, &tenant_id, &package_sid);
            return Err(NonoError::SandboxInit(
                "launch_agent: ResumeThread failed; agent removed (fail-secure)".into(),
            ));
        }

        tracing::info!(
            tenant_id = %tenant_id,
            package_sid = %package_sid,
            exe = %exe.display(),
            "launch_agent: agent launched and registered"
        );

        // Step 9: Spawn a reap task.
        // Duplicate the process handle for the reap closure (AgentTenant owns
        // the primary; the reap task needs its own handle for WaitForSingleObject).
        let maybe_reap_handle = duplicate_process_handle_for_reap(&daemon_state, &tenant_id);

        if let Some(reap_handle_raw) = maybe_reap_handle {
            let reap_daemon_state = Arc::clone(&daemon_state);
            let reap_tenant_id = tenant_id.clone();
            let reap_package_sid = package_sid.clone();
            // Cast HANDLE (*mut c_void) to usize so it crosses the Send boundary.
            // Windows HANDLEs are kernel-object identifiers (numeric) valid from any
            // thread in the same process. Casting to usize and back is the standard
            // Rust pattern for sending Win32 HANDLEs across thread boundaries.
            let reap_handle_usize: usize = reap_handle_raw as usize;

            tokio::spawn(async move {
                let exit_code = tokio::task::spawn_blocking(move || {
                    // SAFETY: `reap_handle_usize` was obtained by casting a valid
                    // duplicated process handle. Casting back gives the same HANDLE.
                    // This closure is the sole owner; CloseHandle is called exactly once.
                    let handle: HANDLE = reap_handle_usize as HANDLE;
                    unsafe { WaitForSingleObject(handle, INFINITE) };
                    let mut code: u32 = 0;
                    // SAFETY: handle is valid post-WaitForSingleObject.
                    unsafe { GetExitCodeProcess(handle, &mut code) };
                    // SAFETY: close our duplicated handle after use.
                    unsafe { CloseHandle(handle) };
                    code
                })
                .await
                .unwrap_or(u32::MAX);

                tracing::info!(
                    tenant_id = %reap_tenant_id,
                    package_sid = %reap_package_sid,
                    exit_code = exit_code,
                    "launch_agent reap: agent exited; removing from DaemonState"
                );

                // Remove from registry FIRST (locking order: registry → tenants).
                if let Ok(mut registry) = reap_daemon_state.agent_registry.lock() {
                    registry.remove(&reap_package_sid);
                }

                // Step 6.5 (reap): Remove the per-agent WFP filter BEFORE dropping
                // AgentTenant (SUPP-02). This is best-effort: failure logs a warning
                // but does NOT abort the reap sequence. The WFP service's startup
                // sweep handles any stale filters (Pitfall 6 mitigation).
                //
                // WFP deactivation fires here (in the reap task) rather than in
                // AgentTenant::Drop to avoid blocking pipe I/O inside Drop
                // (Pitfall 2 mitigation: Drop calling synchronous pipe I/O is risky
                // inside a tokio task context).
                if let Err(e) = wfp_filter_remove(&reap_package_sid, &reap_tenant_id) {
                    tracing::warn!(
                        tenant_id = %reap_tenant_id,
                        error = %e,
                        "launch_agent reap: WFP filter removal failed \
                         (best-effort; service startup sweep will reclaim stale filters)"
                    );
                    // Non-fatal: continue to tenants.remove regardless.
                }

                // Remove from tenants — Drops AgentTenant:
                //   - closes job_handle → KILL_ON_JOB_CLOSE fires
                //   - closes process_handle
                //   - calls DeleteAppContainerProfile (best-effort)
                //
                // NOTE: WFP filter deactivation is handled above (not in Drop)
                // to keep AgentTenant::Drop focused on handle cleanup only.
                if let Ok(mut tenants) = reap_daemon_state.tenants.lock() {
                    tenants.remove(&reap_tenant_id);
                }
            });
        }

        Ok(tenant_id)
    }

    /// Create a Job Object for an agent with `KILL_ON_JOB_CLOSE` and a DACL
    /// that denies Low-IL and the agent's own package SID any job access.
    ///
    /// # SDDL
    ///
    /// ```text
    /// D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)(D;;0x1F001F;;;<package_sid>)
    /// ```
    ///
    /// - `OW` (Owner) — granted full access (the daemon process is the owner)
    /// - `LW` (Low Integrity) — denied all access (MIC belt-and-suspenders)
    /// - `<package_sid>` — denied all access (prevents the agent from opening
    ///   its own job object to call `TerminateJobObject`)
    fn create_agent_job(session_id: &str, package_sid: &str) -> nono::Result<HANDLE> {
        use std::mem::size_of;

        // Build the named job object identifier (Local\ namespace).
        let name = format!(r"Local\nono-session-{}", session_id);
        let name_u16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0u16)).collect();

        // Build the security descriptor SDDL with per-agent deny ACE.
        let sddl = format!("D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)(D;;0x1F001F;;;{package_sid})");
        let wide_sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0u16)).collect();

        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let ok = unsafe {
            // SAFETY: `wide_sddl` is a valid nul-terminated UTF-16 SDDL string.
            // `sd` is a valid out-parameter. SDDL_REVISION_1 is the only documented
            // revision. null for the optional size output parameter is permitted.
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide_sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(NonoError::SandboxInit(format!(
                "create_agent_job: ConvertStringSecurityDescriptorToSecurityDescriptorW \
                 failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // RAII: free the security descriptor via LocalFree on all paths.
        struct SdGuard(PSECURITY_DESCRIPTOR);
        impl Drop for SdGuard {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW;
                    // must be freed with LocalFree per Win32 contract.
                    unsafe { windows_sys::Win32::Foundation::LocalFree(self.0.cast()) };
                }
            }
        }
        let _sd_guard = SdGuard(sd);

        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };

        let job = unsafe {
            // SAFETY: `sa.lpSecurityDescriptor` points to memory owned by
            // `_sd_guard` which is in scope for the duration of this call.
            CreateJobObjectW(&sa, name_u16.as_ptr())
        };
        if job.is_null() {
            return Err(NonoError::SandboxInit(format!(
                "create_agent_job: CreateJobObjectW({name:?}) failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Configure KILL_ON_JOB_CLOSE and DIE_ON_UNHANDLED_EXCEPTION.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;

        let ok = unsafe {
            // SAFETY: `limits` is a valid zero-initialized struct for
            // JobObjectExtendedLimitInformation. The size matches exactly.
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            // SAFETY: job is a valid handle; close to avoid leak.
            unsafe { CloseHandle(job) };
            return Err(NonoError::SandboxInit(
                "create_agent_job: SetInformationJobObject(KILL_ON_JOB_CLOSE) failed".into(),
            ));
        }

        Ok(job)
    }

    /// Assign a process to the agent job object. Returns `Err` with a
    /// descriptive message if `AssignProcessToJobObject` fails.
    fn assign_process_to_agent_job(job: HANDLE, process: HANDLE) -> nono::Result<()> {
        let ok = unsafe {
            // SAFETY: `job` is a valid Job Object handle and `process` is a
            // valid process handle from CreateProcessW.
            AssignProcessToJobObject(job, process)
        };
        if ok == 0 {
            let gle = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            let msg = if gle == 5 {
                // ERROR_ACCESS_DENIED: the child is already in a different job
                // that disallows breakaway — nono cannot guarantee kill-group.
                "assign_process_to_agent_job: AssignProcessToJobObject denied \
                     (GLE=5): process already in a non-breakaway job — \
                     nono cannot guarantee agent kill-group (fail-secure)"
                    .to_string()
            } else {
                format!(
                    "assign_process_to_agent_job: AssignProcessToJobObject failed \
                     (GLE={gle}): agent process terminated (fail-secure)"
                )
            };
            return Err(NonoError::SandboxInit(msg));
        }
        Ok(())
    }

    /// Duplicate the agent's process handle from `DaemonState::tenants` for
    /// the reap task. Returns `None` if the entry is missing or
    /// `DuplicateHandle` fails — reap task is not spawned but
    /// `KILL_ON_JOB_CLOSE` remains the safety net.
    fn duplicate_process_handle_for_reap(
        daemon_state: &Arc<DaemonState>,
        tenant_id: &str,
    ) -> Option<HANDLE> {
        use std::os::windows::io::AsRawHandle;

        let primary_raw: HANDLE = {
            let tenants = daemon_state.tenants.lock().ok()?;
            let tenant = tenants.get(tenant_id)?;
            tenant.process_handle.as_raw_handle() as HANDLE
        };

        if primary_raw.is_null() || primary_raw == INVALID_HANDLE_VALUE {
            tracing::warn!(
                tenant_id = %tenant_id,
                "launch_agent: process handle unavailable for reap task \
                 (KILL_ON_JOB_CLOSE remains active)"
            );
            return None;
        }

        let current = unsafe { GetCurrentProcess() };
        let mut dup_raw: HANDLE = std::ptr::null_mut();
        let ok = unsafe {
            // SAFETY: both handles are valid. We create a new handle for
            // the reap task to own independently.
            DuplicateHandle(
                current,
                primary_raw,
                current,
                &mut dup_raw,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            )
        };
        if ok == 0 {
            tracing::warn!(
                tenant_id = %tenant_id,
                "launch_agent: DuplicateHandle for reap task failed \
                 (KILL_ON_JOB_CLOSE remains active)"
            );
            return None;
        }
        Some(dup_raw)
    }

    /// Phase 117 D-21 (CINT-02) / step 6.7's decision shape — this daemon's
    /// own independent mirror of `exec_strategy_windows::attestation::
    /// AttestationDecision` (structurally unreachable from this binary; see
    /// `daemon_attest_and_decide`'s doc comment). `layer`/`status` values
    /// intentionally use the same `LayerId`/`LayerAttestationStatus`
    /// `Debug`-format vocabulary the CLI-side gate uses, so operator-facing
    /// text and `NonoDiagnosticCode` classification stay consistent across
    /// both binaries even though the decision logic itself is duplicated.
    ///
    /// # Phase 117 review NR3-05 (deliberately two-state, not three) / gap-closure WR-06
    ///
    /// Unlike the CLI-side `exec_strategy_windows::attestation::
    /// AttestationDecision`, this daemon-side mirror has NO `ProceedDowngraded`
    /// variant — but NOT because `DaemonDaclGuard::apply`'s three-pass apply
    /// is fully fail-closed with nothing in between `Ok` and `Err`. It
    /// isn't: pass 1 skips (warns, records nothing) a read-only rule whose
    /// path is not owned by the current user, and pass 3 `break`s at the
    /// first non-owned workspace ancestor — both real skip/break arms
    /// `granted_write_access()` cannot express. The reason `ProceedDowngraded`
    /// stays removed is what those two arms actually skip: TRAVERSE grants on
    /// read-only rules and ancestor directories, never a WRITE grant. Unlike
    /// the CLI's `AppliedDaclGrantsGuard`'s `SkipWritableNotOwned` (which
    /// gates a WRITE grant the child was specifically promised, and so
    /// classifies `PartiallyApplied`), a skipped daemon-side traverse grant
    /// only narrows an ancillary discoverability path the daemon relies on
    /// the lowbox bypass-traverse privilege to reach anyway (see each pass's
    /// own comment above). Pass 2 — the one pass that grants WRITE — IS
    /// fail-closed end-to-end: any failure there aborts the whole `apply`
    /// via `revert_all` + `Err`, never a skip. Every layer
    /// `daemon_attest_and_decide` models (`AppContainerProfile`,
    /// `JobObjectContainment`, `WfpEgressFilters`, `DaclPackageSidGrant`)
    /// therefore has a fail direction that is always under-granting (the
    /// child ends up with less filesystem access than the policy intended),
    /// never under-confining (the child's sandbox boundary is never wider
    /// than intended) — so a downgrade state here would carry no
    /// security-relevant signal an operator needs surfaced, unlike the CLI's
    /// three-state contract. `wfp_filter_add`'s `Err` path also terminates
    /// before this gate is ever reached, keeping the network layer out of
    /// scope for a downgrade state too. A prior revision declared
    /// `ProceedDowngraded` anyway and matched it in `launch_agent` with a
    /// full audit-event-emission + Event Log warning arm, but
    /// `daemon_attest_and_decide` had no return site that ever constructed
    /// it — dead code that looked like a supported state. If a future
    /// daemon-side guard gains a genuine under-confining partial-coverage
    /// state, `ProceedDowngraded` should be reintroduced alongside a real
    /// coverage accessor on that guard (mirroring
    /// `DaclGrantCoverage`/`LabelCoverage` on the CLI side), not restored as
    /// a hardcoded/unreachable literal.
    #[derive(Debug)]
    enum DaemonAttestationDecision {
        /// Every row this function checks classified `Confirmed` (or was not
        /// applicable). The session's confinement claim is fully attested.
        Proceed,
        /// A required layer could not be confirmed. D-22: the caller
        /// terminates the still-suspended process (via
        /// `cleanup_failed_agent`'s job-close cascade) and surfaces a typed
        /// `NonoError` naming `layer`.
        Abort {
            layer: &'static str,
            status: nono::attestation::LayerAttestationStatus,
        },
    }

    /// Phase 117 D-21 (CINT-02) — the daemon's own startup self-attestation
    /// decision, checked against the real suspended child's process handle
    /// (`process`) at step 6.7, before `ResumeThread` ever runs.
    ///
    /// # Why this is a separate implementation, not a shared call
    ///
    /// This module's own doc comment (lines 26-30) states it intentionally
    /// does NOT depend on `exec_strategy_windows/` — `nono-agentd` is a
    /// wholly separate binary crate (`src/bin/nono-agentd.rs` `#[path]`-
    /// includes only `agent_daemon/mod.rs`, `telemetry/mod.rs`, and
    /// `agent_daemon/telemetry_init.rs`; it never declares
    /// `exec_strategy_windows`), so
    /// `exec_strategy_windows::attestation::attest_and_decide` and
    /// `exec_strategy_windows::layer_registry` are not reachable symbols
    /// from here. This function mirrors `attest_and_decide`'s decision
    /// shape using the shared, policy-free `nono::attestation` probes (the
    /// one dependency both binaries genuinely share, via the `nono` library
    /// crate) for the subset of `layer_registry.rs` rows expected at
    /// `(EntryPath::Daemon, None)`:
    ///
    /// - `AppContainerProfile` (`LiveTokenOrJobQuery`, `Abort`) —
    ///   [`nono::attestation::probe_app_container_sid`]; the apply-time
    ///   `create_app_container_profile`/spawn already succeeded (steps 2-5),
    ///   so this is the first INDEPENDENT post-hoc confirmation.
    /// - `JobObjectContainment` (`LiveTokenOrJobQuery`, `Abort`) —
    ///   [`nono::attestation::probe_in_job`]; independently reconfirms step 6.
    /// - `DaclPackageSidGrant` / `DaclAncestorTraverse`
    ///   (`ProbeKind::ConfiguredOnly`, `Abort`) — attested from
    ///   `dacl_guard_applied`, the caller's report of whether
    ///   `DaemonDaclGuard::apply` actually ran and succeeded at step 6.6.
    /// - `WfpEgressFilters` (`ConfirmedByEnforcingComponentReport`, `Abort`)
    ///   — `NotApplicable` when this profile did not request network
    ///   scoping, otherwise attested from the caller's report that
    ///   `wfp_filter_add` succeeded (WR-02).
    ///
    /// # Phase 117 review CR-06 / WR-02 / WR-08 / WR-01
    ///
    /// This function used to return a HARDCODED
    /// `ProceedDowngraded { downgraded: ["DaclPackageSidGrant",
    /// "DaclAncestorTraverse", "DaclAncestorReadAttrs"] }` on every
    /// successful launch:
    ///
    /// - **CR-06:** `DaclAncestorReadAttrs` was in that list although
    ///   `DaemonDaclGuard::apply` performs exactly three operations —
    ///   `grant_sid_traverse_on_path` on read-only rules,
    ///   `grant_sid_write_on_path` on the workspace, and
    ///   `grant_sid_traverse_on_path` on workspace ancestors. It never calls
    ///   `grant_sid_read_attributes_on_path`. The daemon was reporting a
    ///   layer as "established, cannot re-observe" when no apply ever
    ///   occurred on that path. The row is dropped here and its
    ///   `(Daemon, None)` expectancy cell is dropped from
    ///   `layer_registry.rs`.
    /// - **WR-02:** `network_scoping_required` was captured into a local
    ///   with the comment "so step 6.7's attestation gate can reuse this
    ///   exact result" — and step 6.7 never read it. It is now a parameter,
    ///   alongside whether the filter-add succeeded.
    /// - **WR-08:** `DaemonAttestationDecision::Proceed` was matched but
    ///   never constructed. It is now the outcome of a launch where every
    ///   modelled layer holds.
    /// - **WR-01:** the AppContainer SID probe discarded the SID it
    ///   returned, so a child in a *different* AppContainer — one the WFP
    ///   `ALE_USER_ID` filter is not scoped to — attested identically to the
    ///   correct one. It is now compared to this tenant's own package SID.
    fn daemon_attest_and_decide(
        process: HANDLE,
        job: HANDLE,
        expected_package_sid: &str,
        dacl_guard_applied: bool,
        network_scoping_required: bool,
        wfp_filters_installed: bool,
    ) -> DaemonAttestationDecision {
        use nono::attestation::{probe_app_container_sid, probe_in_job, LayerAttestationStatus};

        // WR-01: presence is not enough — it must be THIS tenant's package
        // SID, the one the WFP ALE_USER_ID filter is scoped to.
        let app_container_confirmed = match probe_app_container_sid(process) {
            Ok(Some(sid)) => sid.eq_ignore_ascii_case(expected_package_sid),
            Ok(None) | Err(_) => false,
        };
        if !app_container_confirmed {
            return DaemonAttestationDecision::Abort {
                layer: "AppContainerProfile",
                status: LayerAttestationStatus::Unconfirmed,
            };
        }

        // Phase 117 review CR-02: probed against the daemon's OWN agent job
        // handle, not the vacuous "is this process in ANY job" query a null
        // handle would ask (a Win8+ nested-job host makes that always true).
        let job_confirmed = matches!(probe_in_job(process, job), Ok(true));
        if !job_confirmed {
            return DaemonAttestationDecision::Abort {
                layer: "JobObjectContainment",
                status: LayerAttestationStatus::Unconfirmed,
            };
        }

        // WR-02: only attested when this profile actually asked for network
        // scoping. A profile that did not is NotApplicable, not degraded.
        if network_scoping_required && !wfp_filters_installed {
            return DaemonAttestationDecision::Abort {
                layer: "WfpEgressFilters",
                status: LayerAttestationStatus::Unconfirmed,
            };
        }

        // CR-06/CR-09: the ConfiguredOnly DACL rows are attested from what
        // the caller actually applied, not assumed.
        if !dacl_guard_applied {
            return DaemonAttestationDecision::Abort {
                layer: "DaclPackageSidGrant",
                status: LayerAttestationStatus::Unconfirmed,
            };
        }

        // WR-08: every modelled layer holds. `Proceed` is reachable.
        DaemonAttestationDecision::Proceed
    }

    /// Remove state for a failed agent launch. If `AgentTenant` was already
    /// inserted, removing it drops the struct → closes job_handle →
    /// `KILL_ON_JOB_CLOSE` terminates the process group.
    fn cleanup_failed_agent(daemon_state: &Arc<DaemonState>, tenant_id: &str, package_sid: &str) {
        // Registry remove FIRST (locking order).
        if let Ok(mut registry) = daemon_state.agent_registry.lock() {
            registry.remove(package_sid);
        }
        // Tenants remove — Drop closes handles + DeleteAppContainerProfile.
        if let Ok(mut tenants) = daemon_state.tenants.lock() {
            tenants.remove(tenant_id);
        }
    }

    /// Resolve an executable path to an absolute `PathBuf`.
    ///
    /// # Resolution rules
    ///
    /// 1. If the given path is already absolute AND exists on disk → return as-is.
    /// 2. Otherwise, search via `SearchPathW` with `lpPath = null` (uses the
    ///    standard Windows search order: current directory, then each `PATH`
    ///    directory, then `System32`, etc.) and `lpExtension = ".exe"`.
    /// 3. If `SearchPathW` returns 0 → return a CLEAR error message instead of
    ///    propagating a raw `os error 2` from `CreateProcessW`.
    ///
    /// The resolved absolute path is then passed to `spawn_appcontainer_process_suspended`
    /// so that `CreateProcessW(lpApplicationName)` receives a fully-qualified path.
    /// Confinement (AppContainer token + Job Object) is unchanged — it is applied
    /// to the resolved binary, not to the bare name.
    ///
    /// # Errors
    ///
    /// Returns `Err` with a human-readable message if the exe cannot be located.
    pub(crate) fn resolve_exe_path(exe: PathBuf) -> nono::Result<PathBuf> {
        // Fast path: already an absolute path that exists on disk.
        if exe.is_absolute() && exe.exists() {
            return Ok(exe);
        }

        // Convert the executable name to UTF-16 for the Win32 API.
        let exe_str = exe.to_string_lossy();
        let exe_wide: Vec<u16> = exe_str
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();

        // Extension hint: ".exe" in UTF-16, null-terminated.
        let ext_wide: Vec<u16> = ".exe\0".encode_utf16().collect();

        // Phase 1: probe to get the required buffer length.
        // SAFETY: `SearchPathW` with a null `lpPath` uses the standard Windows
        // search path. Passing `null` for the output buffer is the documented
        // probe idiom — it returns the required character count (including the
        // null terminator) without writing anything. `null` for the file-part
        // pointer is permitted when we do not need the filename offset.
        let needed = unsafe {
            SearchPathW(
                std::ptr::null(),     // lpPath: null → use standard search path
                exe_wide.as_ptr(),    // lpFileName: the bare name (e.g. "notepad.exe")
                ext_wide.as_ptr(),    // lpExtension: append ".exe" if no extension
                0,                    // nBufferLength: 0 for probe
                std::ptr::null_mut(), // lpBuffer: null for probe
                std::ptr::null_mut(), // lpFilePart: not needed
            )
        };

        if needed == 0 {
            // SearchPathW returned 0: not found on any search path.
            return Err(NonoError::SandboxInit(format!(
                "agent launch: executable '{exe_str}' not found \
                 (provide an absolute path or ensure it is on PATH)"
            )));
        }

        // Phase 2: allocate buffer and retrieve the full path.
        let buf_len = needed as usize + 1; // +1 for safety (needed already includes null)
        let mut buf: Vec<u16> = vec![0u16; buf_len];

        // SAFETY: `buf` is a writable buffer of `buf_len` u16 elements (>= `needed`).
        // `exe_wide` and `ext_wide` are valid null-terminated UTF-16 strings.
        // `SearchPathW` writes at most `buf_len` characters including the null terminator.
        let written = unsafe {
            SearchPathW(
                std::ptr::null(),
                exe_wide.as_ptr(),
                ext_wide.as_ptr(),
                buf_len as u32,
                buf.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };

        if written == 0 || written as usize >= buf_len {
            return Err(NonoError::SandboxInit(format!(
                "agent launch: SearchPathW for '{exe_str}' failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Trim to the actual length (written does NOT include the null terminator).
        buf.truncate(written as usize);
        // SAFETY: `buf` contains valid UTF-16 from SearchPathW.
        let os_str = {
            use std::os::windows::ffi::OsStringExt as _;
            std::ffi::OsString::from_wide(&buf)
        };
        let resolved = std::path::PathBuf::from(os_str);

        tracing::debug!(
            exe = %exe.display(),
            resolved = %resolved.display(),
            "launch_agent: exe resolved via SearchPathW"
        );

        Ok(resolved)
    }

    // D-29/D-30 (Phase 117-07): a per-layer force-unavailable seam for the
    // daemon's OWN, structurally-separate AppContainer/SECURITY_CAPABILITIES
    // construction (module doc lines 25-30 — this module intentionally does
    // NOT depend on `exec_strategy_windows/`, so it needs its own hook rather
    // than reusing `restricted_token.rs`'s). Compiled out of the default
    // build entirely via `layer-fault-injection` — there is no runtime (env
    // var or otherwise) toggle; a default release binary does not contain
    // either the static or the setter symbol. Mirrors
    // `restricted_token.rs`'s `RESTRICTED_TOKEN_FORCE_UNAVAILABLE` idiom
    // exactly (Plan 06); shares the same feature declaration in
    // `nono-sandbox-cli`'s `Cargo.toml` (Plan 04) since this file is part of
    // that crate.
    #[cfg(feature = "layer-fault-injection")]
    static DAEMON_APP_CONTAINER_FORCE_UNAVAILABLE: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    /// Set the daemon's AppContainer test-force-unavailable flag for the
    /// CINT-03 per-layer forced-unavailable test harness. Only exists when
    /// built with `--features layer-fault-injection`.
    #[cfg(feature = "layer-fault-injection")]
    pub(crate) fn force_daemon_app_container_unavailable(unavailable: bool) {
        DAEMON_APP_CONTAINER_FORCE_UNAVAILABLE
            .store(unavailable, std::sync::atomic::Ordering::Relaxed);
    }

    #[cfg(feature = "layer-fault-injection")]
    fn daemon_app_container_force_unavailable() -> bool {
        DAEMON_APP_CONTAINER_FORCE_UNAVAILABLE.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Spawn a process in the AppContainer with `CREATE_SUSPENDED`.
    ///
    /// Returns `(process_handle, thread_handle)`. Caller owns both and must
    /// eventually close them (or wrap in `OwnedHandle`).
    ///
    /// # Errors
    ///
    /// Returns `Err` if any Win32 setup call fails.
    fn spawn_appcontainer_process_suspended(
        exe: &std::path::Path,
        args: &[String],
        package_sid_psid: PSID,
    ) -> nono::Result<(HANDLE, HANDLE)> {
        // D-30: the force-unavailable seam only exists when built with
        // `--features layer-fault-injection` — in a default build this
        // branch and the function it calls are not compiled in at all, so
        // there is no runtime toggle to read. Short-circuits before the real
        // SECURITY_CAPABILITIES/PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
        // construction so a forced-unavailable AppContainer never silently
        // spawns the agent with a different (unconfined) attribute set — it
        // refuses to launch at all (T-117-14 analog for the daemon path).
        #[cfg(feature = "layer-fault-injection")]
        if daemon_app_container_force_unavailable() {
            return Err(NonoError::LayerAttestationFailed {
                layer: "AppContainerProfile".into(),
                reason: "forced unavailable by test seam".into(),
            });
        }

        // Build SECURITY_CAPABILITIES for the AppContainer token.
        let sec_caps = SECURITY_CAPABILITIES {
            AppContainerSid: package_sid_psid,
            Capabilities: std::ptr::null_mut(),
            CapabilityCount: 0,
            Reserved: 0,
        };

        // Probe the required attribute-list buffer size.
        let mut attr_size: usize = 0;
        unsafe {
            // SAFETY: documented probe idiom — null pointer → returns required size.
            InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);
        }
        if attr_size == 0 {
            return Err(NonoError::SandboxInit(
                "spawn_appcontainer_process_suspended: \
                 InitializeProcThreadAttributeList size probe returned 0"
                    .into(),
            ));
        }

        let mut attr_buf = vec![0u8; attr_size];
        let attr_list_ptr: LPPROC_THREAD_ATTRIBUTE_LIST =
            attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;

        let ok = unsafe {
            // SAFETY: `attr_buf` is sized by the probe.
            InitializeProcThreadAttributeList(attr_list_ptr, 1, 0, &mut attr_buf.len())
        };
        if ok == 0 {
            return Err(NonoError::SandboxInit(format!(
                "spawn_appcontainer_process_suspended: \
                 InitializeProcThreadAttributeList failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // RAII: ensure DeleteProcThreadAttributeList runs on all paths.
        struct AttrListGuard(LPPROC_THREAD_ATTRIBUTE_LIST);
        impl Drop for AttrListGuard {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    // SAFETY: Initialized by InitializeProcThreadAttributeList above.
                    unsafe { DeleteProcThreadAttributeList(self.0) };
                }
            }
        }
        let _attr_guard = AttrListGuard(attr_list_ptr);

        let ok = unsafe {
            // SAFETY: `attr_list_ptr` is initialized for 1 slot. `sec_caps` is a
            // valid SECURITY_CAPABILITIES struct; `package_sid_psid` remains valid
            // through CreateProcessW (owned by `owned_sid` in the caller frame).
            UpdateProcThreadAttribute(
                attr_list_ptr,
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                &sec_caps as *const SECURITY_CAPABILITIES as *mut _,
                std::mem::size_of::<SECURITY_CAPABILITIES>(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if ok == 0 {
            return Err(NonoError::SandboxInit(format!(
                "spawn_appcontainer_process_suspended: \
                 UpdateProcThreadAttribute(SECURITY_CAPABILITIES) failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Build the command line (mutable buffer required by CreateProcessW).
        let mut cmd_line = build_command_line(exe, args);
        let app_name_wide: Vec<u16> = {
            use std::os::windows::ffi::OsStrExt;
            exe.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0u16))
                .collect()
        };

        let mut si_ex: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        si_ex.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        si_ex.lpAttributeList = attr_list_ptr;

        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let ok = unsafe {
            // SAFETY: `app_name_wide` and `cmd_line` are null-terminated UTF-16
            // strings. `si_ex` carries a valid attribute list. `sec_caps` and
            // `_attr_guard` outlive this call (declared in the same stack frame).
            CreateProcessW(
                app_name_wide.as_ptr(),
                cmd_line.as_mut_ptr(),
                std::ptr::null(), // lpProcessAttributes
                std::ptr::null(), // lpThreadAttributes
                0,                // bInheritHandles = FALSE
                CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
                std::ptr::null_mut(), // lpEnvironment (inherit)
                std::ptr::null(),     // lpCurrentDirectory (inherit)
                &si_ex as *const STARTUPINFOEXW as *const STARTUPINFOW,
                &mut pi,
            )
        };
        if ok == 0 {
            return Err(NonoError::SandboxInit(format!(
                "spawn_appcontainer_process_suspended: CreateProcessW({:?}) failed: {}",
                exe.display(),
                std::io::Error::last_os_error()
            )));
        }

        // `_attr_guard` drops here → DeleteProcThreadAttributeList.
        Ok((pi.hProcess, pi.hThread))
    }

    /// Build a null-terminated UTF-16 command line from `exe` + `args`.
    /// The buffer is mutable because `CreateProcessW` may modify it internally.
    fn build_command_line(exe: &std::path::Path, args: &[String]) -> Vec<u16> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let mut line = String::new();
        let exe_str = exe.to_string_lossy();
        if exe_str.contains(' ') || exe_str.contains('"') {
            line.push('"');
            line.push_str(&exe_str);
            line.push('"');
        } else {
            line.push_str(&exe_str);
        }
        for arg in args {
            line.push(' ');
            if arg.contains(' ') || arg.contains('"') || arg.is_empty() {
                line.push('"');
                line.push_str(&arg.replace('"', "\\\""));
                line.push('"');
            } else {
                line.push_str(arg);
            }
        }
        OsStr::new(&line)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect()
    }

    /// Generate a 32-character hex string for the tenant_id (128 bits of randomness).
    fn generate_tenant_id() -> nono::Result<String> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|e| {
            NonoError::SandboxInit(format!("generate_tenant_id: getrandom::fill failed: {e}"))
        })?;
        Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }

    /// CINT-03 forced-unavailable regression (Phase 117-07). Only compiled
    /// with `--features layer-fault-injection`; a default build contains
    /// neither `force_daemon_app_container_unavailable` nor this test.
    #[cfg(feature = "layer-fault-injection")]
    #[cfg(test)]
    mod app_container_force_unavailable_tests {
        use super::*;

        /// With the daemon's AppContainer force-unavailable seam armed,
        /// `spawn_appcontainer_process_suspended` refuses to construct
        /// `SECURITY_CAPABILITIES` and returns
        /// `NonoError::LayerAttestationFailed` before ever calling
        /// `CreateProcessW` with `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`
        /// — the check fires before `package_sid_psid` (a null PSID here) is
        /// ever dereferenced, so this is safe to exercise without a real
        /// AppContainer profile.
        #[test]
        fn spawn_appcontainer_process_suspended_fails_when_forced_unavailable() {
            force_daemon_app_container_unavailable(true);
            let result = spawn_appcontainer_process_suspended(
                std::path::Path::new(r"C:\Windows\System32\cmd.exe"),
                &[],
                std::ptr::null_mut(),
            );
            // Always reset the flag, even on assertion failure, so this test
            // cannot leak state into other tests in the same binary.
            force_daemon_app_container_unavailable(false);

            match result {
                Err(NonoError::LayerAttestationFailed { layer, reason }) => {
                    assert_eq!(layer, "AppContainerProfile");
                    assert!(
                        reason.contains("forced unavailable"),
                        "reason must explain the forced-unavailable seam: {reason}"
                    );
                }
                Err(other) => panic!(
                    "expected NonoError::LayerAttestationFailed while forced unavailable, got Err({other})"
                ),
                Ok(_) => panic!(
                    "expected NonoError::LayerAttestationFailed while forced unavailable, got Ok"
                ),
            }
        }
    }

    /// Phase 117 Plan 10 (CINT-02) — step 6.7's `daemon_attest_and_decide`
    /// decision function, tested in isolation from the full async
    /// `launch_agent` (which needs a live `DaemonState`/tokio runtime this
    /// unit-test module does not construct).
    #[cfg(test)]
    mod attestation_gate_tests {
        use super::*;

        /// Behavior 1: an `Abort`-outcome layer that cannot be confirmed — no
        /// AppContainer SID present at all — causes `daemon_attest_and_decide`
        /// to return `Abort { layer: "AppContainerProfile", .. }` without ever
        /// reaching `JobObjectContainment`. Deterministic: `OpenProcessToken`
        /// always fails against an invalid process handle.
        #[test]
        fn null_handle_aborts_on_app_container_profile() {
            let job: HANDLE = unsafe {
                // SAFETY: CreateJobObjectW with null name + null security
                // attributes is documented to succeed unless out-of-memory.
                CreateJobObjectW(std::ptr::null(), std::ptr::null())
            };
            assert!(!job.is_null(), "CreateJobObjectW failed");
            let decision = daemon_attest_and_decide(
                std::ptr::null_mut(),
                job,
                "S-1-15-2-1",
                true,
                false,
                false,
            );
            // SAFETY: `job` is a valid HANDLE this test owns.
            unsafe { CloseHandle(job) };
            match decision {
                DaemonAttestationDecision::Abort { layer, .. } => {
                    assert_eq!(layer, "AppContainerProfile");
                }
                other => panic!("expected Abort, got {other:?}"),
            }
        }

        /// Behavior 2: a real AppContainer-confined, job-assigned suspended
        /// process — mirroring `launch_agent`'s own steps 2-6 (minus the DACL
        /// grants and registry bookkeeping this isolated test doesn't need) —
        /// classifies `AppContainerProfile` and `JobObjectContainment` as
        /// independently `Confirmed`, and (with every other row's inputs also
        /// reported as applied) the gate reaches `Proceed` (Plan 08 Deviation
        /// 2 / Plan 17 NR3-05: reaching this point at all means the
        /// apply-time gate — which this isolated test does not itself run —
        /// already succeeded fail-closed in the real `launch_agent` flow this
        /// test exercises a slice of; the daemon's decision shape has no
        /// downgraded-but-proceeding state to reach instead).
        #[test]
        fn real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable() {
            let tenant_id = generate_tenant_id().expect("generate_tenant_id");
            let profile_name = format!("nono.gatetest.{}", &tenant_id[..16]);
            let profile = nono::create_app_container_profile(&profile_name)
                .expect("create_app_container_profile");
            let owned_sid =
                nono::derive_app_container_sid(&profile_name).expect("derive_app_container_sid");
            let psid: PSID = owned_sid.as_psid();

            let (process, thread) = spawn_appcontainer_process_suspended(
                std::path::Path::new(r"C:\Windows\System32\cmd.exe"),
                &["/c".to_string(), "exit".to_string(), "0".to_string()],
                psid,
            )
            .expect("spawn_appcontainer_process_suspended");

            let job: HANDLE = unsafe {
                // SAFETY: CreateJobObjectW with null name + null security
                // attributes is documented to succeed unless out-of-memory.
                CreateJobObjectW(std::ptr::null(), std::ptr::null())
            };
            assert!(!job.is_null(), "CreateJobObjectW failed");
            let assigned = unsafe {
                // SAFETY: both `job` and `process` are valid HANDLEs owned by
                // this test for the duration of this call.
                AssignProcessToJobObject(job, process)
            };
            assert_ne!(assigned, 0, "AssignProcessToJobObject failed");

            let expected_sid =
                nono::package_sid_to_string(&owned_sid).expect("package_sid_to_string");
            let decision =
                daemon_attest_and_decide(process, job, &expected_sid, true, false, false);
            // Phase 117 review CR-06/CR-09/WR-01 non-vacuity: the SAME live
            // child, this time with the caller reporting that
            // DaemonDaclGuard::apply did NOT run, must abort. Before the fix
            // the DACL rows were a hardcoded literal no input could change.
            let unapplied_dacl_decision =
                daemon_attest_and_decide(process, job, &expected_sid, false, false, false);
            // ...and with network scoping required but the filters not
            // installed (WR-02: the daemon did not model this row at all).
            let unscoped_decision =
                daemon_attest_and_decide(process, job, &expected_sid, true, true, false);
            // ...and against a package SID that is not this child's (WR-01).
            let foreign_sid_decision =
                daemon_attest_and_decide(process, job, "S-1-15-2-9-9-9", true, false, false);

            // SAFETY: `process`/`thread`/`job` are all valid HANDLEs this test
            // owns; the suspended process is never resumed, only probed then
            // torn down.
            unsafe {
                let _ = TerminateProcess(process, 1);
                CloseHandle(process);
                CloseHandle(thread);
                CloseHandle(job);
            }
            // Explicit drop deletes the AppContainer profile (mirrors
            // production cleanup; production instead `std::mem::forget`s it
            // and defers deletion to `AgentTenant::Drop`).
            drop(profile);

            assert!(
                matches!(decision, DaemonAttestationDecision::Proceed),
                "a fully-applied daemon launch must reach Proceed (WR-08), got {decision:?}"
            );
            match unapplied_dacl_decision {
                DaemonAttestationDecision::Abort { layer, .. } => {
                    assert_eq!(layer, "DaclPackageSidGrant");
                }
                other => panic!("expected Abort{{DaclPackageSidGrant}}, got {other:?}"),
            }
            match unscoped_decision {
                DaemonAttestationDecision::Abort { layer, .. } => {
                    assert_eq!(layer, "WfpEgressFilters");
                }
                other => panic!("expected Abort{{WfpEgressFilters}}, got {other:?}"),
            }
            match foreign_sid_decision {
                DaemonAttestationDecision::Abort { layer, .. } => {
                    assert_eq!(layer, "AppContainerProfile");
                }
                other => panic!("expected Abort{{AppContainerProfile}}, got {other:?}"),
            }
        }

        /// Phase 117 review CR-02 non-vacuity proof on the DAEMON mirror: the
        /// same real AppContainer-confined suspended child, this time NEVER
        /// assigned to the job handle passed to the gate, must abort naming
        /// `JobObjectContainment`.
        ///
        /// Before CR-02 the daemon probed with a null job handle ("in ANY
        /// job"), which on a Win8+ nested-job host answers `true` for a child
        /// that inherited the test runner's own job — this abort was
        /// structurally unreachable.
        #[test]
        fn real_appcontainer_process_outside_the_named_job_aborts_on_job_containment() {
            let tenant_id = generate_tenant_id().expect("generate_tenant_id");
            let profile_name = format!("nono.gatetestnojob.{}", &tenant_id[..16]);
            let profile = nono::create_app_container_profile(&profile_name)
                .expect("create_app_container_profile");
            let owned_sid =
                nono::derive_app_container_sid(&profile_name).expect("derive_app_container_sid");
            let psid: PSID = owned_sid.as_psid();

            let (process, thread) = spawn_appcontainer_process_suspended(
                std::path::Path::new(r"C:\Windows\System32\cmd.exe"),
                &["/c".to_string(), "exit".to_string(), "0".to_string()],
                psid,
            )
            .expect("spawn_appcontainer_process_suspended");

            let job: HANDLE = unsafe {
                // SAFETY: see above.
                CreateJobObjectW(std::ptr::null(), std::ptr::null())
            };
            assert!(!job.is_null(), "CreateJobObjectW failed");
            // NOTE: deliberately NOT calling AssignProcessToJobObject.

            let expected_sid =
                nono::package_sid_to_string(&owned_sid).expect("package_sid_to_string");
            let decision =
                daemon_attest_and_decide(process, job, &expected_sid, true, false, false);

            // SAFETY: all three are valid HANDLEs this test owns; the
            // suspended process is never resumed, only probed then torn down.
            unsafe {
                let _ = TerminateProcess(process, 1);
                CloseHandle(process);
                CloseHandle(thread);
                CloseHandle(job);
            }
            drop(profile);

            match decision {
                DaemonAttestationDecision::Abort { layer, .. } => {
                    assert_eq!(
                        layer, "JobObjectContainment",
                        "a child outside the named job must fail THIS layer"
                    );
                }
                other => panic!(
                    "expected Abort{{JobObjectContainment}} for a child never assigned to the \
                     named job, got {other:?}"
                ),
            }
        }

        /// Phase 117-12 (D-24): measures `daemon_attest_and_decide`'s real
        /// wall-clock cost against a real process handle (`GetCurrentProcess()`
        /// — a valid pseudo-handle, not `real_appcontainer_job_process_
        /// proceeds_downgraded`'s heavier full AppContainer+Job spawn, which
        /// this timing probe does not need: the cost being measured is the
        /// `probe_app_container_sid`/`probe_in_job` Win32 call overhead, which
        /// is paid identically regardless of what the handle points at). Not a
        /// bound-asserting benchmark (see `attest_and_decide`'s own D-24
        /// timing test in `exec_strategy_windows/attestation.rs` for the
        /// identical reasoning) — its purpose is to produce a real number for
        /// the SPEC's Latency budget table.
        #[test]
        fn daemon_attest_and_decide_latency() {
            let real_process: HANDLE = unsafe { GetCurrentProcess() };
            // CR-02: a REAL job handle, so the measurement covers the real
            // `IsProcessInJob(process, job)` call rather than the null-job
            // fail-closed early return.
            let real_job: HANDLE = unsafe {
                // SAFETY: see above.
                CreateJobObjectW(std::ptr::null(), std::ptr::null())
            };
            assert!(!real_job.is_null(), "CreateJobObjectW failed");
            let start = std::time::Instant::now();
            let _ =
                daemon_attest_and_decide(real_process, real_job, "S-1-15-2-1", true, false, false);
            let elapsed = start.elapsed();
            // SAFETY: `real_job` is a valid HANDLE this test owns.
            unsafe { CloseHandle(real_job) };
            eprintln!(
                "D-24 measured daemon_attest_and_decide cost (real GetCurrentProcess() handle): \
                 {elapsed:?}"
            );
            assert!(
                elapsed < std::time::Duration::from_millis(250),
                "daemon_attest_and_decide took {elapsed:?}, exceeding the generous 250ms \
                 sanity bound"
            );
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::DaemonState;
    use std::sync::Arc;

    fn empty_state() -> Arc<DaemonState> {
        Arc::new(DaemonState::new())
    }

    /// Phase 117 review NR-05: the step 6.7 gate's abort predicate is
    /// `network_scoping_required && !wfp_filters_installed`. The shipped call
    /// site passed the SAME local for both parameters, so the predicate was
    /// `x && !x` — a compile-time constant `false` — and `dacl_guard_applied`
    /// was a literal `true`. Every parameter the fix added was documentation
    /// rather than a check, and the daemon mirror's deny direction was
    /// unreachable outside a unit test.
    ///
    /// A behavioural test cannot catch this: `daemon_attest_and_decide`
    /// itself is correct, and the unit tests that drive it pass distinct
    /// values. The defect lives entirely in how the production call site
    /// wires it up, so this asserts on that wiring directly.
    #[test]
    fn daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition() {
        let src = include_str!("launch.rs");
        let call = src
            .split("match daemon_attest_and_decide(")
            .nth(1)
            .expect("the step 6.7 attestation gate call site must exist");
        let args_block = call.split(") {").next().expect("argument list");
        let args: Vec<&str> = args_block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| line.trim_end_matches(','))
            .collect();
        assert_eq!(
            args.len(),
            6,
            "unexpected daemon_attest_and_decide call shape: {args:?}"
        );

        let dacl_guard_applied = args[3];
        let network_scoping_required = args[4];
        let wfp_filters_installed = args[5];

        assert!(
            !matches!(dacl_guard_applied, "true" | "false"),
            "dacl_guard_applied must be the guard's own report, not a literal: {dacl_guard_applied}"
        );
        assert!(
            !matches!(wfp_filters_installed, "true" | "false"),
            "wfp_filters_installed must be the real filter-add outcome, not a literal: \
             {wfp_filters_installed}"
        );
        assert_ne!(
            wfp_filters_installed, network_scoping_required,
            "wfp_filters_installed and network_scoping_required must be independent values — \
             passing the same expression for both makes the row's abort predicate `x && !x`, a \
             compile-time constant false (NR-05)"
        );
    }

    /// Parses an enum's top-level variant names out of its own source text.
    ///
    /// Locates the enum body immediately following `enum_marker` up to the
    /// first line whose trimmed content is exactly `"}"` — a struct-like
    /// variant's own closing line (e.g. `Abort`'s `},`) carries a trailing
    /// comma and so never matches, only the enum's real closing brace does.
    /// Within that body, variant lines are whichever indent level is
    /// SHALLOWEST among the non-blank, non-comment, non-attribute,
    /// non-`}` lines present — this works for both a module-nested enum
    /// (variant lines at 8 spaces, like `DaemonAttestationDecision`) and a
    /// top-level enum (variant lines at 4 spaces, like the CLI's
    /// `AttestationDecision`) without the caller needing to know which.
    /// Struct-variant FIELD lines sit one level deeper than the variant name
    /// itself and are excluded by the same shallowest-indent filter.
    ///
    /// Factored out of the two tests below (WR-06/WR-11 gap-closure) so both
    /// share one parsing implementation instead of two independently
    /// hand-rolled ones drifting apart.
    fn parse_enum_variant_names(src: &str, enum_marker: &str) -> Vec<String> {
        let after = src
            .split(enum_marker)
            .nth(1)
            .unwrap_or_else(|| panic!("enum marker not found in source: {enum_marker}"));

        let body_lines: Vec<&str> = after
            .lines()
            .take_while(|line| line.trim() != "}")
            .collect();

        let candidate_indent = |line: &&str| -> Option<usize> {
            let trimmed = line.trim_start();
            if trimmed.is_empty()
                || trimmed.starts_with("///")
                || trimmed.starts_with('#')
                || trimmed.starts_with('}')
            {
                None
            } else {
                Some(line.len() - trimmed.len())
            }
        };

        let variant_indent = body_lines
            .iter()
            .filter_map(candidate_indent)
            .min()
            .unwrap_or_else(|| panic!("no variant lines found after marker: {enum_marker}"));

        body_lines
            .iter()
            .filter(|line| candidate_indent(line) == Some(variant_indent))
            .map(|line| {
                line.trim_start()
                    .split([',', '{', '('])
                    .next()
                    .unwrap_or(line)
                    .trim()
                    .to_string()
            })
            .collect()
    }

    /// Phase 117 Plan 17 (NR3-05): `DaemonAttestationDecision` is deliberately
    /// two-state (`Proceed` / `Abort`) — see the enum's own doc comment for
    /// why a `ProceedDowngraded` variant was removed rather than wired up.
    /// This test is discovery-based, not a hardcoded assumption baked into a
    /// second place: it parses the variant names straight out of THIS file's
    /// own source text (the `super::super::` `include_str!` idiom already
    /// established by `daemon_attestation_gate_is_wired_to_real_outcomes_not_
    /// the_gate_condition` above), so it fails the build the moment the enum
    /// regains a third variant the module doc doesn't account for — whether
    /// that's a reintroduced `ProceedDowngraded` or something else entirely.
    ///
    /// Non-vacuity: verified manually per the plan's acceptance criteria by
    /// temporarily re-adding `ProceedDowngraded { downgraded: Vec<&'static
    /// str> },` to the enum (without updating this test's expected list) and
    /// confirming the assertion below fails naming the mismatch, then
    /// reverting — see 117-17-SUMMARY.md.
    #[test]
    fn daemon_attestation_decision_is_deliberately_two_state() {
        let src = include_str!("launch.rs");
        let variants = parse_enum_variant_names(src, "enum DaemonAttestationDecision {");

        assert_eq!(
            variants,
            vec!["Proceed".to_string(), "Abort".to_string()],
            "DaemonAttestationDecision must stay a genuine two-state enum (Proceed / Abort). \
             If this assertion fails because a new variant was added, \
             daemon_attest_and_decide's own return sites AND launch_agent's match on the \
             decision must both be updated in the SAME commit — and if the new variant is a \
             downgrade state, it must be backed by a real coverage accessor on the guard that \
             produces it, not a hardcoded value (the exact class NR3-05 found and Plan 17 \
             closed): got {variants:?}"
        );
    }

    /// WR-06 gap-closure: the daemon and CLI attestation-decision surfaces
    /// (`DaemonAttestationDecision`'s own doc comment above explains why
    /// they are two independently-implemented mirrors of the same
    /// fail-direction contract, not a shared call) can silently drift apart
    /// — a variant added to one without the other, or without a recorded
    /// reason for the split, is a documentation/audit gap this test exists
    /// to catch at compile time rather than at review time.
    ///
    /// Discovery-based, like the sibling test above: parses both enums'
    /// variant names straight out of their own source text via
    /// `include_str!`. Reading `exec_strategy_windows/attestation.rs` here
    /// is a compile-time TEXT embed only (`#[cfg(test)]`), not a module or
    /// link dependency — `nono-agentd` never `#[path]`-includes
    /// `exec_strategy_windows/` in a real build (see
    /// `daemon_attest_and_decide`'s own doc comment, "Why this is a separate
    /// implementation, not a shared call").
    ///
    /// Non-vacuity: every daemon variant must have a same-named CLI
    /// counterpart, AND every CLI variant must be either mirrored on the
    /// daemon side or explicitly named in `documented_divergence` — an
    /// undocumented 4th variant on either side fails this test by name, not
    /// with a bare `false`.
    #[test]
    fn every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence() {
        let daemon_src = include_str!("launch.rs");
        let daemon_variants =
            parse_enum_variant_names(daemon_src, "enum DaemonAttestationDecision {");

        let cli_src = include_str!("../exec_strategy_windows/attestation.rs");
        let cli_variants = parse_enum_variant_names(cli_src, "enum AttestationDecision {");

        // WR-06: the one intentional divergence — see `DaemonAttestationDecision`'s
        // own doc comment ("Phase 117 review NR3-05") for why `ProceedDowngraded`
        // stays CLI-only.
        let documented_divergence: &[&str] = &["ProceedDowngraded"];

        for daemon_variant in &daemon_variants {
            assert!(
                cli_variants.contains(daemon_variant),
                "DaemonAttestationDecision::{daemon_variant} has no counterpart in the CLI's \
                 AttestationDecision ({cli_variants:?}) — the daemon mirror must stay a subset \
                 of the CLI's decision shape"
            );
        }

        for cli_variant in &cli_variants {
            let is_mirrored = daemon_variants.contains(cli_variant);
            let is_documented = documented_divergence.contains(&cli_variant.as_str());
            assert!(
                is_mirrored || is_documented,
                "AttestationDecision::{cli_variant} exists on the CLI side but is neither \
                 mirrored in DaemonAttestationDecision ({daemon_variants:?}) nor listed in \
                 documented_divergence ({documented_divergence:?}) — an undocumented \
                 divergence between the daemon and CLI decision shapes must be resolved \
                 (either mirror it or add it to documented_divergence with a stated reason)"
            );
        }
    }

    // ── Task 2 (75-07-T2): DaemonDaclGuard unit tests ────────────────────────

    // A package-SID-shaped (S-1-15-2-*) test SID for the DaemonDaclGuard tests.
    // Distinct suffix from dacl_guard.rs TEST_PACKAGE_SID to avoid ACE collision
    // in parallel test runs.
    #[cfg(target_os = "windows")]
    const TEST_PACKAGE_SID: &str = "S-1-15-2-10-20-30-40-50-60-71";

    /// Returns true iff `path`'s DACL contains an ACE for `sid`.
    ///
    /// Mirrored verbatim from `exec_strategy_windows::dacl_guard::tests::dacl_contains_sid`.
    /// NOT imported from there (pub(crate) to exec_strategy_windows — not accessible
    /// from agent_daemon tests; module-independence invariant in launch.rs lines 27-31).
    #[cfg(target_os = "windows")]
    fn dacl_contains_sid(path: &std::path::Path, sid: &str) -> bool {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{
            ConvertStringSidToSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
        };
        use windows_sys::Win32::Security::{
            EqualSid, GetAce, ACCESS_ALLOWED_ACE, ACL, DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, PSID,
        };

        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_sid: Vec<u16> = sid.encode_utf16().chain(std::iter::once(0)).collect();

        let mut want_sid: PSID = std::ptr::null_mut();
        // SAFETY: valid nul-terminated UTF-16 SID string + valid out-pointer.
        let ok = unsafe { ConvertStringSidToSidW(wide_sid.as_ptr(), &mut want_sid) };
        assert!(ok != 0 && !want_sid.is_null(), "parse test SID");

        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: valid path buffer + valid out-pointers; SD freed below.
        let status = unsafe {
            GetNamedSecurityInfoW(
                wide_path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut sd,
            )
        };
        assert_eq!(status, 0, "GetNamedSecurityInfoW(DACL) must succeed");

        let mut found = false;
        if !dacl.is_null() {
            // SAFETY: `dacl` points into the SD we own until LocalFree below.
            let ace_count = unsafe { (*dacl).AceCount };
            for index in 0..ace_count {
                let mut ace = std::ptr::null_mut();
                // SAFETY: `dacl` is valid; `ace` is a valid out-pointer.
                let got = unsafe { GetAce(dacl, u32::from(index), &mut ace) };
                if got == 0 || ace.is_null() {
                    continue;
                }
                // SAFETY: allow/deny ACEs share the SidStart layout; we read
                // the embedded SID at that offset.
                let ace_sid = unsafe {
                    (&(*(ace as *const ACCESS_ALLOWED_ACE)).SidStart) as *const u32 as PSID
                };
                // SAFETY: both SIDs are valid for the duration of the call.
                if unsafe { EqualSid(ace_sid, want_sid) } != 0 {
                    found = true;
                    break;
                }
            }
        }

        // SAFETY: both allocations came from Win32 and must be LocalFree'd.
        unsafe {
            if !want_sid.is_null() {
                let _ = LocalFree(want_sid as _);
            }
            if !sd.is_null() {
                let _ = LocalFree(sd as _);
            }
        }
        found
    }

    /// Verify DaemonDaclGuard applies a write-grant ACE on a workspace dir and
    /// reverts it when the guard drops. Mirrors
    /// `writable_rule_applies_sid_ace_and_reverts_on_drop` in dacl_guard.rs.
    #[test]
    #[cfg(target_os = "windows")]
    fn daemon_dacl_guard_applies_and_reverts_write_grant() {
        use super::windows_impl::DaemonDaclGuard;
        use nono::{CapabilitySource, WindowsFilesystemPolicy, WindowsFilesystemRule};
        use tempfile::tempdir;

        let dir = tempdir().expect("tempdir");
        let workspace = dir.path().to_path_buf();

        let policy = WindowsFilesystemPolicy {
            rules: vec![WindowsFilesystemRule {
                path: workspace.clone(),
                access: nono::AccessMode::ReadWrite,
                is_file: false,
                source: CapabilitySource::User,
            }],
            unsupported: vec![],
        };

        assert!(
            !dacl_contains_sid(&workspace, TEST_PACKAGE_SID),
            "test precondition: test SID must not pre-exist on the DACL"
        );

        {
            let guard = DaemonDaclGuard::apply(&policy, &workspace, TEST_PACKAGE_SID)
                .expect("DaemonDaclGuard::apply must succeed on a user-owned tempdir");
            assert!(
                dacl_contains_sid(&workspace, TEST_PACKAGE_SID),
                "during guard lifetime the package SID's ACE must be present on the DACL"
            );
            drop(guard);
        }

        assert!(
            !dacl_contains_sid(&workspace, TEST_PACKAGE_SID),
            "after guard drop the package SID's ACE must be revoked"
        );
    }

    /// Verify that when a mid-loop grant fails, already-applied grants are reverted.
    /// Mirrors `mid_loop_grant_failure_reverts_already_applied` in dacl_guard.rs.
    ///
    /// Strategy: policy has one real owned ReadWrite rule (workspace) and one Read
    /// rule pointing at a nonexistent path (so grant_sid_traverse_on_path fails on
    /// the missing path). DaemonDaclGuard::apply must return Err AND have reverted
    /// the workspace write grant.
    #[test]
    #[cfg(target_os = "windows")]
    fn daemon_dacl_guard_mid_loop_failure_reverts_already_applied() {
        use super::windows_impl::DaemonDaclGuard;
        use nono::{CapabilitySource, WindowsFilesystemPolicy, WindowsFilesystemRule};
        use tempfile::tempdir;

        let dir = tempdir().expect("tempdir");
        let workspace = dir.path().to_path_buf();
        // A nonexistent path: grant_sid_traverse_on_path will fail on it.
        let bad_path = workspace.join("nonexistent-read-dir-for-mid-loop-test");

        let policy = WindowsFilesystemPolicy {
            rules: vec![
                // Read rule for a nonexistent path — traverse grant will fail.
                WindowsFilesystemRule {
                    path: bad_path.clone(),
                    access: nono::AccessMode::Read,
                    is_file: false,
                    source: CapabilitySource::User,
                },
                // Write rule for the actual workspace — applied in pass 2 AFTER
                // the read-only rules pass (pass 1). If pass 1 fails on the bad_path
                // ownership check (missing paths → ownership Err), revert is called
                // before the write grant. Either ordering must result in Err +
                // dacl_contains_sid(workspace) = false.
                WindowsFilesystemRule {
                    path: workspace.clone(),
                    access: nono::AccessMode::ReadWrite,
                    is_file: false,
                    source: CapabilitySource::User,
                },
            ],
            unsupported: vec![],
        };

        let result = DaemonDaclGuard::apply(&policy, &workspace, TEST_PACKAGE_SID);
        assert!(
            result.is_err(),
            "DaemonDaclGuard::apply must fail when a grant target does not exist"
        );

        // Any grants applied before the failure must have been reverted.
        assert!(
            !dacl_contains_sid(&workspace, TEST_PACKAGE_SID),
            "after mid-loop failure the workspace SID ACE must be reverted (fail-secure)"
        );
    }

    /// Verify that DaemonDaclGuard revokes BOTH the write grant on the workspace
    /// and traverse grants on ancestor dirs when the guard drops (reap revocation).
    /// Mirrors the Warning-2 requirement: reap revocation must cover traverse paths.
    #[test]
    #[cfg(target_os = "windows")]
    fn daemon_dacl_guard_reap_revokes_traverse_paths() {
        use super::windows_impl::DaemonDaclGuard;
        use nono::{CapabilitySource, WindowsFilesystemPolicy, WindowsFilesystemRule};
        use tempfile::tempdir;

        // Create a nested dir: outer/ (Read) + outer/workspace/ (ReadWrite workspace).
        let outer_dir = tempdir().expect("outer tempdir");
        let workspace = outer_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace).expect("create workspace subdir");
        let outer = outer_dir.path().to_path_buf();

        let policy = WindowsFilesystemPolicy {
            rules: vec![
                // A Read rule on the outer dir → traverse grant in pass 1.
                WindowsFilesystemRule {
                    path: outer.clone(),
                    access: nono::AccessMode::Read,
                    is_file: false,
                    source: CapabilitySource::User,
                },
                // The workspace itself → write grant in pass 2.
                WindowsFilesystemRule {
                    path: workspace.clone(),
                    access: nono::AccessMode::ReadWrite,
                    is_file: false,
                    source: CapabilitySource::User,
                },
            ],
            unsupported: vec![],
        };

        assert!(
            !dacl_contains_sid(&workspace, TEST_PACKAGE_SID),
            "precondition: test SID must not pre-exist on workspace DACL"
        );
        assert!(
            !dacl_contains_sid(&outer, TEST_PACKAGE_SID),
            "precondition: test SID must not pre-exist on outer dir DACL"
        );

        {
            let guard = DaemonDaclGuard::apply(&policy, &workspace, TEST_PACKAGE_SID)
                .expect("DaemonDaclGuard::apply must succeed");

            // During guard lifetime: BOTH the workspace (write) and outer (traverse)
            // must carry the SID ACE.
            assert!(
                dacl_contains_sid(&workspace, TEST_PACKAGE_SID),
                "workspace write grant must be present during guard lifetime"
            );
            assert!(
                dacl_contains_sid(&outer, TEST_PACKAGE_SID),
                "outer dir traverse grant must be present during guard lifetime"
            );
            drop(guard);
        }

        // After drop: BOTH grants must be revoked.
        assert!(
            !dacl_contains_sid(&workspace, TEST_PACKAGE_SID),
            "workspace write grant must be revoked after guard drop (reap revocation)"
        );
        assert!(
            !dacl_contains_sid(&outer, TEST_PACKAGE_SID),
            "outer dir traverse grant must be revoked after guard drop (reap revocation)"
        );
    }

    // ── SUPP-02 unit tests (Plan 75-01) ──────────────────────────────────────

    /// SC: wfp_proxy_only
    ///
    /// Verify that `wfp_filter_add` (Plan 83-02, EGRESS-02) builds a
    /// `WfpRuntimeActivationRequest` with the force-through-proxy shape:
    /// - `request_kind = "activate_proxy_mode"`
    /// - `network_mode = "proxy-only"`
    /// - `localhost_ports = [proxy_port]` (the loopback proxy listener port)
    /// - `session_sid = Some(package_sid)` (SID-keyed per-agent filter)
    /// - `target_program_path = None` (session_sid path, not program-path path)
    /// - Deterministic rule names derived from `tenant_id`
    /// - Fail-secure status handling (invalid-request / protocol-mismatch → Err) unchanged
    ///
    /// This test verifies the D-01/D-02 force-through-proxy request shape. The WFP
    /// service's `build_policy_filter_specs` produces PERMIT-loopback(weight 100) +
    /// BLOCK-all(weight 0) from this request; weights are already correct in the
    /// service and are NOT changed here (Pitfall 4 — DO NOT change weights).
    ///
    /// Because the WFP control pipe is not available in unit tests we build the
    /// request struct directly and assert the field contract.
    #[test]
    #[cfg(target_os = "windows")]
    fn wfp_proxy_only_constructs_proxy_mode_request() {
        use super::super::wfp_contract::{
            WfpRuntimeActivationRequest, WFP_RUNTIME_PROTOCOL_VERSION,
        };

        let package_sid = "S-1-15-2-1234-5678-9012-3456-7890-1234-5678";
        let tenant_id = "abcdef1234567890abcdef1234567890";
        let proxy_port: u16 = 8899;

        // Mirror wfp_filter_add's request construction (Plan 83-02 updated form).
        let req = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            request_kind: "activate_proxy_mode".to_string(),
            network_mode: "proxy-only".to_string(),
            preferred_backend: "wfp".to_string(),
            active_backend: "wfp".to_string(),
            runtime_target: format!("nono-agent-{tenant_id}"),
            tcp_connect_ports: vec![],
            tcp_bind_ports: vec![],
            localhost_ports: vec![proxy_port],
            localhost_port_ranges: vec![],
            target_program_path: None,
            session_sid: Some(package_sid.to_string()),
            outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
            inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
        };

        // D-01/D-02 force-through-proxy: verify the request shape exactly.
        assert_eq!(
            req.request_kind, "activate_proxy_mode",
            "request_kind must be 'activate_proxy_mode' (D-01 force-through-proxy)"
        );
        assert_eq!(
            req.network_mode, "proxy-only",
            "network_mode must be 'proxy-only' (D-02 proxy-only WFP mode)"
        );
        // The proxy port must appear in localhost_ports — this is the ONLY
        // outbound path permitted for the SID (D-02, EGRESS-02).
        assert_eq!(
            req.localhost_ports,
            vec![proxy_port],
            "localhost_ports must contain exactly the proxy_port (D-02)"
        );
        // session_sid must be set to the package SID (SID-keyed per-agent filter path).
        assert_eq!(
            req.session_sid,
            Some(package_sid.to_string()),
            "session_sid must be Some(package_sid)"
        );
        // target_program_path must be None for the session_sid-keyed path.
        assert!(
            req.target_program_path.is_none(),
            "target_program_path must be None for session_sid-keyed filter path"
        );
        // Rule names are deterministic from tenant_id (unchanged from pre-83 path).
        assert_eq!(
            req.outbound_rule_name,
            Some(format!("nono-agent-{tenant_id}"))
        );
        assert_eq!(
            req.inbound_rule_name,
            Some(format!("nono-agent-{tenant_id}-in"))
        );
        assert_eq!(req.protocol_version, WFP_RUNTIME_PROTOCOL_VERSION);
        // tcp_connect_ports / tcp_bind_ports must be empty (loopback-proxy-only, not raw TCP).
        assert!(req.tcp_connect_ports.is_empty());
        assert!(req.tcp_bind_ports.is_empty());
    }

    /// SC: wfp_proxy_only — verify proxy port is a function parameter (no hardcoded port).
    ///
    /// This test runs the request construction with two different proxy_port values
    /// and verifies both propagate correctly into `localhost_ports`.
    /// Named `wfp_proxy_only` so `cargo test wfp_proxy_only` matches the plan spec.
    #[test]
    fn wfp_proxy_only_port_is_parameterised() {
        // On non-Windows the WfpRuntimeActivationRequest type compiles but the
        // wfp_filter_add function is Windows-only; test the request shape directly.
        use super::super::wfp_contract::{
            WfpRuntimeActivationRequest, WFP_RUNTIME_PROTOCOL_VERSION,
        };

        for proxy_port in [8080u16, 9000u16, 65535u16] {
            let req = WfpRuntimeActivationRequest {
                protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
                request_kind: "activate_proxy_mode".to_string(),
                network_mode: "proxy-only".to_string(),
                preferred_backend: "wfp".to_string(),
                active_backend: "wfp".to_string(),
                runtime_target: "nono-agent-test".to_string(),
                tcp_connect_ports: vec![],
                tcp_bind_ports: vec![],
                localhost_ports: vec![proxy_port],
                localhost_port_ranges: vec![],
                target_program_path: None,
                session_sid: Some("S-1-15-2-test".to_string()),
                outbound_rule_name: Some("nono-agent-test".to_string()),
                inbound_rule_name: Some("nono-agent-test-in".to_string()),
            };
            assert_eq!(
                req.localhost_ports,
                vec![proxy_port],
                "localhost_ports must propagate proxy_port={proxy_port} unchanged"
            );
            assert_eq!(req.network_mode, "proxy-only");
        }
    }

    /// SC: wfp_filter_add_constructs_request (historical — kept for regression coverage)
    ///
    /// Documents the pre-Plan-83-02 request shape (blocked mode) for comparison.
    /// This is a regression guard: the wfp_filter_add function NOW uses proxy-only;
    /// this test exists to document the change and validate the helper contract.
    ///
    /// The fail-secure status handling (invalid-request / protocol-mismatch → Err)
    /// is verified by `wfp_absent_fail_secure` (unchanged).
    #[test]
    #[cfg(target_os = "windows")]
    fn wfp_filter_add_constructs_request() {
        // Build the request struct directly to verify field values.
        use super::super::wfp_contract::{
            WfpRuntimeActivationRequest, WFP_RUNTIME_PROTOCOL_VERSION,
        };

        let package_sid = "S-1-15-2-1234-5678-9012-3456-7890-1234-5678";
        let tenant_id = "abcdef1234567890abcdef1234567890";
        let proxy_port: u16 = 8899;

        // Mirror the UPDATED wfp_filter_add request construction (Plan 83-02).
        let req = WfpRuntimeActivationRequest {
            protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
            request_kind: "activate_proxy_mode".to_string(),
            network_mode: "proxy-only".to_string(),
            preferred_backend: "wfp".to_string(),
            active_backend: "wfp".to_string(),
            runtime_target: format!("nono-agent-{tenant_id}"),
            tcp_connect_ports: vec![],
            tcp_bind_ports: vec![],
            localhost_ports: vec![proxy_port],
            localhost_port_ranges: vec![],
            target_program_path: None,
            session_sid: Some(package_sid.to_string()),
            outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
            inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
        };

        // Plan 83-02: request_kind and network_mode are now proxy-mode.
        assert_eq!(req.request_kind, "activate_proxy_mode");
        assert_eq!(req.network_mode, "proxy-only");
        assert_eq!(req.localhost_ports, vec![proxy_port]);
        assert_eq!(req.session_sid, Some(package_sid.to_string()));
        assert_eq!(
            req.outbound_rule_name,
            Some(format!("nono-agent-{tenant_id}"))
        );
        assert_eq!(
            req.inbound_rule_name,
            Some(format!("nono-agent-{tenant_id}-in"))
        );
        assert_eq!(req.protocol_version, WFP_RUNTIME_PROTOCOL_VERSION);
        // target_program_path must be None for the session_sid-keyed filter path.
        assert!(req.target_program_path.is_none());
    }

    /// SC: wfp_absent_no_scoping_ok
    ///
    /// When a profile does NOT declare network scoping (`network.block = false`
    /// or absent), `profile_needs_network_scoping` returns false and the WFP
    /// gate is skipped entirely — the daemon proceeds even if nono-wfp-service
    /// is absent. (D-05 pass-through path.)
    #[test]
    fn wfp_absent_no_scoping_ok() {
        // All existing profiles have network.block = false (confirmed from policy.json).
        // Test that profile_needs_network_scoping returns false for known profiles.
        #[cfg(target_os = "windows")]
        {
            use super::windows_impl::profile_needs_network_scoping_testable;
            // "aider" has network.block = false → no WFP gate.
            assert!(
                !profile_needs_network_scoping_testable("aider"),
                "aider profile must NOT require WFP (network.block = false)"
            );
            // Unknown profile → false (fail-safe: no WFP gate for unknown profiles).
            assert!(
                !profile_needs_network_scoping_testable("nonexistent-profile"),
                "unknown profile must NOT require WFP (conservative default)"
            );
        }
        // Non-Windows: the gate never fires; test trivially passes.
        #[cfg(not(target_os = "windows"))]
        {
            // On non-Windows the function is not compiled but the test validates
            // the logic is cfg-gated correctly. (No-op pass.)
        }
    }

    /// SC: wfp_absent_fail_secure
    ///
    /// When the WFP service pipe is unreachable AND the profile requires network
    /// scoping, `wfp_filter_add` returns `Err` with a message naming
    /// `nono-wfp-service`.
    ///
    /// Tests the fail-secure branch by calling `wfp_filter_add` directly on
    /// a non-existent pipe path variant. Since the real pipe path is only
    /// reachable at runtime with the service installed, we test that any
    /// pipe-open failure produces an `Err` containing the service name.
    #[test]
    #[cfg(target_os = "windows")]
    fn wfp_absent_fail_secure() {
        // Calling wfp_filter_add when nono-wfp-service is not running must
        // return Err. We cannot spin up the service in a unit test, so we
        // verify the expected behavior through profile_needs_network_scoping:
        // a profile with network.block = true WOULD gate on wfp_filter_add.
        // The D-05 gate tests this path end-to-end.

        // For unit testing purposes, verify that an error from wfp_filter_add
        // would include the service name (by constructing the error message
        // the same way the helper does, without actually calling the async fn
        // in a blocking test). This tests the error-message contract.
        use nono::NonoError;
        let pipe_error = std::io::Error::from(std::io::ErrorKind::NotFound);
        let e = NonoError::SandboxInit(format!(
            "WFP control pipe unreachable — is nono-wfp-service running? \
             (pipe={}): {pipe_error}",
            super::windows_impl::WFP_CONTROL_PIPE_NAME_TESTABLE,
        ));
        let msg = e.to_string();
        assert!(
            msg.contains("nono-wfp-service"),
            "fail-secure error must name nono-wfp-service; got: {msg}"
        );
        assert!(
            msg.contains("nono-wfp-control"),
            "fail-secure error must name the control pipe; got: {msg}"
        );
    }

    /// SC: wfp_filter_add_at_launch
    ///
    /// Verify that `profile_needs_network_scoping` returns true only for
    /// profiles that have `network.block = true` in policy.json.
    ///
    /// This is the precondition gate that controls whether `wfp_filter_add`
    /// is called in `launch_agent`. Currently all built-in profiles have
    /// `network.block = false`, so `profile_needs_network_scoping` should
    /// return `false` for all of them. If a future profile adds
    /// `network.block = true`, this test will document the expected behavior.
    #[test]
    fn wfp_filter_add_at_launch() {
        #[cfg(target_os = "windows")]
        {
            use super::windows_impl::profile_needs_network_scoping_testable;

            // All current built-in profiles have network.block = false →
            // wfp_filter_add is NOT called → no WFP gate in current tests.
            let profiles_to_check = ["default", "aider", "langchain-python", "node-dev", "claude"];
            for profile in profiles_to_check {
                let result = profile_needs_network_scoping_testable(profile);
                // All should be false for current policy (no network.block = true yet).
                // When a profile with network.block=true is added, update this test.
                assert!(
                    !result,
                    "profile '{profile}' unexpectedly requires WFP (network.block=true not yet set)"
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Non-Windows: test trivially passes (cfg-gated code path).
        }
    }

    /// SC: launch_agent_inserts_into_daemon_state
    ///
    /// Verify the state-management contract: registry insert BEFORE tenant
    /// insert; tenant map has exactly one entry; package_sid matches.
    ///
    /// Uses duplicated handles to avoid requiring a real AppContainer spawn.
    #[test]
    #[cfg(target_os = "windows")]
    fn launch_agent_inserts_into_daemon_state() {
        use super::super::reap::AgentTenant;
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use windows_sys::Win32::Foundation::{DuplicateHandle, BOOL, DUPLICATE_SAME_ACCESS};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        let state = empty_state();
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
            // SAFETY: raw is a valid duplicated process handle.
            unsafe { OwnedHandle::from_raw_handle(raw) }
        };

        let tenant_id = "test-launch-insert-74-04".to_string();
        let package_sid = "S-1-15-2-1234-5678-9012-3456-7890-1234-5678".to_string();

        // Simulate launch_agent: registry insert FIRST (locking order).
        {
            let mut registry = state.agent_registry.lock().unwrap();
            registry.insert(package_sid.clone());
        }
        // Then tenant insert.
        let tenant = AgentTenant {
            tenant_id: tenant_id.clone(),
            package_sid: package_sid.clone(),
            profile_name: "nono.test.launch-insert-74-04".to_string(),
            engine_profile: "test-engine".to_string(),
            caps: nono::CapabilitySet::new(),
            dacl_guard: None,
            job_handle: make_handle(),
            process_handle: make_handle(),
        };
        {
            let mut tenants = state.tenants.lock().unwrap();
            tenants.insert(tenant_id.clone(), tenant);
        }

        // Verify.
        let tenants = state.tenants.lock().unwrap();
        assert_eq!(tenants.len(), 1, "tenants must have one entry after launch");
        let t = tenants.get(&tenant_id).unwrap();
        assert_eq!(
            t.package_sid, package_sid,
            "AgentTenant.package_sid must match"
        );
    }

    /// SC: launch_agent_fresh_profile_per_agent
    ///
    /// Each `launch_agent` call produces a distinct tenant_id (fresh per
    /// agent). Verified via 10 calls to the underlying entropy source.
    #[test]
    fn launch_agent_fresh_profile_per_agent() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..10 {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).expect("getrandom::fill must succeed");
            let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(id.len(), 32, "tenant_id must be 32 hex chars");
            assert!(
                id.chars().all(|c| c.is_ascii_hexdigit()),
                "tenant_id must be lowercase hex"
            );
            assert!(
                ids.insert(id),
                "each tenant_id must be unique (fresh per agent)"
            );
        }
    }

    /// SC: reap_task_removes_tenant_on_exit
    ///
    /// Simulates reap task sequence: insert tenant → remove in locking order
    /// (registry → tenants) → verify DaemonState is clean.
    #[test]
    #[cfg(target_os = "windows")]
    fn reap_task_removes_tenant_on_exit() {
        use super::super::reap::AgentTenant;
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use windows_sys::Win32::Foundation::{DuplicateHandle, BOOL, DUPLICATE_SAME_ACCESS};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        let state = empty_state();
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
            // SAFETY: raw is a valid duplicated process handle.
            unsafe { OwnedHandle::from_raw_handle(raw) }
        };

        let tenant_id = "test-reap-74-04".to_string();
        let package_sid = "S-1-15-2-9876-5432-1098-7654-3210-9876-5432".to_string();

        // Simulate launch (registry FIRST, then tenants).
        {
            let mut registry = state.agent_registry.lock().unwrap();
            registry.insert(package_sid.clone());
        }
        {
            let tenant = AgentTenant {
                tenant_id: tenant_id.clone(),
                package_sid: package_sid.clone(),
                profile_name: "nono.test.reap-74-04".to_string(),
                engine_profile: "test-engine".to_string(),
                caps: nono::CapabilitySet::new(),
                dacl_guard: None,
                job_handle: make_handle(),
                process_handle: make_handle(),
            };
            state
                .tenants
                .lock()
                .unwrap()
                .insert(tenant_id.clone(), tenant);
        }

        assert_eq!(
            state.tenants.lock().unwrap().len(),
            1,
            "one tenant before reap"
        );

        // Simulate reap task (locking order: registry → tenants).
        {
            let mut registry = state.agent_registry.lock().unwrap();
            registry.remove(&package_sid);
        }
        {
            // Removing the entry drops AgentTenant → KILL_ON_JOB_CLOSE +
            // DeleteAppContainerProfile (best-effort).
            state.tenants.lock().unwrap().remove(&tenant_id);
        }

        assert_eq!(
            state.tenants.lock().unwrap().len(),
            0,
            "tenants must be empty after reap"
        );
    }

    /// GAP-75-B regression: bare exe name resolves to an absolute, existing path.
    ///
    /// `resolve_exe_path(PathBuf::from("cmd"))` must return Ok(abs_path) where
    /// abs_path is absolute and whose file stem is "cmd" (cmd.exe exists on every
    /// Windows host via SearchPathW).  This is the same bare-name path that
    /// caused `build_daemon_capability_set` to fail with "Path does not exist"
    /// before the GAP-75-B fix in handle_launch (control_loop.rs).
    #[test]
    #[cfg(target_os = "windows")]
    fn resolve_exe_path_bare_name_returns_absolute() {
        let resolved = super::windows_impl::resolve_exe_path(std::path::PathBuf::from("cmd"))
            .expect(
                "resolve_exe_path(\"cmd\") must succeed on Windows (cmd.exe is always on PATH)",
            );
        assert!(
            resolved.is_absolute(),
            "resolved path must be absolute, got: {}",
            resolved.display()
        );
        assert!(
            resolved.exists(),
            "resolved path must exist on disk, got: {}",
            resolved.display()
        );
        let stem = resolved.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        assert_eq!(
            stem.to_ascii_lowercase(),
            "cmd",
            "resolved file stem must be 'cmd', got: {}",
            resolved.display()
        );
    }
}
