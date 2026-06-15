//! Agent tenant RAII lifecycle management.
//!
//! This module defines [`AgentTenant`], the owning struct for a single confined
//! AI agent managed by the daemon. Dropping an `AgentTenant` deterministically:
//!
//! 1. Closes `job_handle` — because the job was created with `KILL_ON_JOB_CLOSE`,
//!    this fires the kernel kill signal for the entire agent process group. Agents
//!    die with the daemon (ADR-74 Decision D-03).
//! 2. Closes `process_handle` — releases the wait handle.
//! 3. Calls `DeleteAppContainerProfile` to clean up the HKCU registry entry for
//!    the agent's AppContainer profile (best-effort; logs a warning on failure,
//!    never panics — the daemon must stay up even if cleanup partially fails).
//!
//! `AgentRegistry::remove` is the CALLER's responsibility and MUST be called
//! before dropping `AgentTenant` to ensure the registry SID set is cleaned up
//! atomically before the handles are closed. See `agent_daemon/mod.rs` for the
//! canonical remove-then-drop sequence.

// ─── Windows implementation ───────────────────────────────────────────────────

/// Owning struct for a single confined AI agent managed by `nono-agentd`.
///
/// `AgentTenant` is RAII: when it is dropped (either by explicit removal from
/// `DaemonState::tenants` or when the daemon exits), all associated OS
/// resources are released in a deterministic, panic-free sequence (see module
/// doc).
///
/// # Caller contract before `drop`
///
/// `AgentRegistry::remove(&self.package_sid)` MUST be called on the
/// `DaemonState::agent_registry` mutex BEFORE this struct is dropped from
/// `DaemonState::tenants`. This ensures the authorization registry is updated
/// before the process group is killed, preventing a narrow window where a
/// recycled SID could match a stale registry entry.
///
/// # `job_handle` and `KILL_ON_JOB_CLOSE`
///
/// The job object MUST have been created with the
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` extended limit set (enforced in
/// `agent_daemon::launch`). When `job_handle` is closed by `OwnedHandle::drop`,
/// the Windows kernel sends `SIGKILL` to all processes in the job (the
/// agent and any grandchildren it spawned).
pub(crate) struct AgentTenant {
    /// Session-scoped unique identifier assigned at spawn time.
    ///
    /// Used as the key in `DaemonState::tenants` and in diagnostic log fields.
    pub tenant_id: String,

    /// AppContainer package SID in SDDL form (`S-1-15-2-...`).
    ///
    /// This is the SID minted at spawn time and inserted into
    /// `DaemonState::agent_registry`. Callers MUST call
    /// `AgentRegistry::remove(&self.package_sid)` before dropping this struct.
    pub package_sid: String,

    /// AppContainer moniker (e.g. `nono.session.<uuid>`).
    ///
    /// Retained so that `Drop` can call `DeleteAppContainerProfile` with the
    /// same name that was passed to `CreateAppContainerProfile` at spawn time.
    pub profile_name: String,

    /// Engine profile requested by the operator (e.g. `"aider"`).
    ///
    /// This is the human-readable profile name from `policy.json` that the
    /// operator passed to `nono agent launch --profile <name>`. It is distinct
    /// from `profile_name` (the internal AppContainer moniker `nono.session.<id>`).
    /// `handle_list` displays this field so operators see `profile=aider` rather
    /// than the opaque `nono.session.<id>` string.
    pub engine_profile: String,

    /// The capability grant for this agent.
    ///
    /// Immutable after `AgentTenant` is constructed. The daemon MUST NOT
    /// mutate `caps` in response to any wire request (ADR-74 Decision D-04:
    /// no escape hatch; query-only pipe).
    ///
    /// Read by `agent_daemon::accept_loop` when serving capability queries.
    /// `#[allow(dead_code)]` because clippy cannot see the read in the binary
    /// compilation unit (accessed via tests + accept_loop's query path).
    #[allow(dead_code)]
    pub caps: nono::CapabilitySet,

    /// Job object handle for the agent's process group.
    ///
    /// MUST have `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` set. When this
    /// `OwnedHandle` drops, the kernel kills the entire agent process group.
    ///
    /// Closed automatically by `OwnedHandle::drop` in `AgentTenant::drop`.
    /// `#[expect(dead_code)]` because clippy's dead_code analysis does not
    /// consider implicit destruction (Drop) as a "read" of the field.
    #[cfg(target_os = "windows")]
    #[expect(dead_code, reason = "closed via OwnedHandle::drop in AgentTenant::drop")]
    pub job_handle: std::os::windows::io::OwnedHandle,

    /// Process handle for the agent's root process.
    ///
    /// Used to `WaitForSingleObject` when reaping the agent (Plan 74-04).
    /// Closing this handle releases the kernel's reference to the process
    /// object; the process itself is already dead (killed by `job_handle` close).
    ///
    /// `#[expect(dead_code)]` because clippy's dead_code analysis does not
    /// consider implicit destruction (Drop) as a "read" of the field.
    #[cfg(target_os = "windows")]
    pub process_handle: std::os::windows::io::OwnedHandle,
}

#[cfg(target_os = "windows")]
impl Drop for AgentTenant {
    fn drop(&mut self) {
        // Step 1: `job_handle` and `process_handle` drop automatically via
        // `OwnedHandle::drop` — the compiler guarantees this after the
        // explicit code below runs. `job_handle` close fires KILL_ON_JOB_CLOSE.

        // Step 2: Delete the AppContainer profile to prevent HKCU registry
        // accumulation. This is best-effort: a failure does not abort cleanup
        // (the handles are still closed; the agent is still terminated).
        //
        // Callers MUST have already called `AgentRegistry::remove` on
        // `DaemonState::agent_registry` before this Drop runs. See `mod.rs`
        // for the correct remove-then-drop ordering.
        if let Err(e) = delete_app_container_profile(&self.profile_name) {
            tracing::warn!(
                tenant_id = %self.tenant_id,
                profile_name = %self.profile_name,
                error = %e,
                "Failed to delete AppContainer profile on agent reap (best-effort; \
                 daemon remains operational)"
            );
        }

        tracing::info!(
            tenant_id = %self.tenant_id,
            package_sid = %self.package_sid,
            "AgentTenant reaped: job handle closed (KILL_ON_JOB_CLOSE fired), \
             process handle released, AppContainer profile cleanup attempted"
        );
    }
}

/// Deletes the AppContainer profile registered under `profile_name`.
///
/// Calls `windows_sys::Win32::Security::Isolation::DeleteAppContainerProfile`
/// directly. Returns `Ok(())` on success (HRESULT == 0) or an error string
/// on failure. Callers (specifically `AgentTenant::Drop`) treat any error as
/// a warning and continue — this is a best-effort cleanup.
///
/// # Safety
///
/// `profile_name` is converted to a null-terminated UTF-16 string and passed
/// to the Win32 API. The string must be the same moniker that was used in the
/// original `CreateAppContainerProfile` call.
#[cfg(target_os = "windows")]
pub(crate) fn delete_app_container_profile(profile_name: &str) -> Result<(), String> {
    use windows_sys::Win32::Security::Isolation::DeleteAppContainerProfile;

    let name_wide: Vec<u16> = profile_name
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();

    // SAFETY: `name_wide` is a valid null-terminated UTF-16 string built from
    // the profile moniker that was used to create the AppContainer. The API
    // does not retain a pointer to `name_wide` after the call returns.
    let hr = unsafe { DeleteAppContainerProfile(name_wide.as_ptr()) };

    if hr == 0 {
        Ok(())
    } else {
        Err(format!(
            "DeleteAppContainerProfile({profile_name:?}) failed (HRESULT=0x{:08X})",
            hr as u32
        ))
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::delete_app_container_profile;
    use super::AgentTenant;
    use std::os::windows::io::{FromRawHandle, OwnedHandle};

    /// Verify that `delete_app_container_profile` does not panic, regardless of
    /// whether the profile exists.
    ///
    /// `DeleteAppContainerProfile` is idempotent on Windows: calling it for a
    /// profile name that was never created (or has already been deleted) returns
    /// `S_OK` (HRESULT 0) — it does NOT return an error for non-existent profiles.
    /// This test verifies the no-panic contract for both the success and no-op paths.
    ///
    /// The `AgentTenant::Drop` graceful-failure path (the `tracing::warn!` on
    /// non-zero HRESULT) protects against future Windows versions or
    /// error-prone profile names returning actual failure codes; it is not
    /// exercised by this test because the API is effectively always-success.
    #[test]
    fn delete_profile_does_not_panic() {
        // A profile name that was never registered.
        // On Windows 10/11, DeleteAppContainerProfile returns S_OK (idempotent).
        let result = delete_app_container_profile("nono.test.profile.nonexistent.74-03");
        // Must not panic — result is either Ok (idempotent success) or Err (handled gracefully).
        match result {
            Ok(()) => {
                // Expected: DeleteAppContainerProfile is idempotent for non-existent profiles.
            }
            Err(e) => {
                // Also acceptable: some OS versions return a failure code. The function
                // must return Err (not panic) and the caller (Drop) logs a warning.
                let _ = e; // Non-fatal: just verifying no panic occurred.
            }
        }
    }

    /// Verify that `AgentTenant` fields are accessible and that dropping an
    /// `AgentTenant` with a non-existent profile name does not panic.
    ///
    /// We use `GetCurrentProcess()` duplicated via `DuplicateHandle` to obtain
    /// real OS handles that are valid to close. This exercises the RAII lifecycle
    /// without needing a real AppContainer child process.
    ///
    /// The `DeleteAppContainerProfile` call in Drop will fail for the fake profile
    /// name — this is expected and must produce a `tracing::warn!`, not a panic.
    #[test]
    fn agent_tenant_drop_does_not_panic_on_fake_profile() {
        use windows_sys::Win32::Foundation::{DuplicateHandle, BOOL, DUPLICATE_SAME_ACCESS};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        // Duplicate the current process handle twice to get two valid, closeable handles.
        let current = unsafe { GetCurrentProcess() };

        let mut job_raw = std::ptr::null_mut();
        let ok: BOOL = unsafe {
            DuplicateHandle(
                current,
                current,
                current,
                &mut job_raw,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            )
        };
        assert_ne!(ok, 0, "DuplicateHandle for job_handle must succeed");

        let mut proc_raw = std::ptr::null_mut();
        let ok: BOOL = unsafe {
            DuplicateHandle(
                current,
                current,
                current,
                &mut proc_raw,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            )
        };
        assert_ne!(ok, 0, "DuplicateHandle for process_handle must succeed");

        // SAFETY: both raw handles are valid duplicates of the current process handle.
        let job_handle = unsafe { OwnedHandle::from_raw_handle(job_raw) };
        let proc_handle = unsafe { OwnedHandle::from_raw_handle(proc_raw) };

        // Construct an AgentTenant with fake metadata. caps uses the default empty set.
        let tenant = AgentTenant {
            tenant_id: "test-tenant-drop-74-03".to_string(),
            package_sid: "S-1-15-2-fake-test-74-03".to_string(),
            profile_name: "nono.test.fake-profile.drop.74-03".to_string(),
            engine_profile: "test-engine".to_string(),
            caps: nono::CapabilitySet::new(),
            job_handle,
            process_handle: proc_handle,
        };

        // Read all fields (satisfies clippy dead_code analysis for pub fields).
        assert_eq!(tenant.tenant_id, "test-tenant-drop-74-03");
        assert_eq!(tenant.package_sid, "S-1-15-2-fake-test-74-03");
        assert_eq!(tenant.profile_name, "nono.test.fake-profile.drop.74-03");
        // caps field is accessed (even though CapabilitySet has no simple eq)
        let _ = &tenant.caps;

        // Drop here — triggers AgentTenant::Drop:
        // - closes job_handle and process_handle (OwnedHandle::drop)
        // - calls DeleteAppContainerProfile("nono.test.fake-profile.drop.74-03")
        //   → fails with non-zero HRESULT → logs tracing::warn! → no panic
        drop(tenant);
        // If we reached here without panicking, the contract is satisfied.
    }
}
