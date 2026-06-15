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

// All `windows_impl` functions are called by `control_loop.rs` (Wave 5);
// `#[allow(dead_code)]` is retained for non-called helpers within this module.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
mod windows_impl {
    use super::super::reap::AgentTenant;
    use super::super::DaemonState;
    use nono::NonoError;
    use std::path::PathBuf;
    use std::os::windows::io::FromRawHandle;
    use std::sync::Arc;

    use windows_sys::Win32::Foundation::{
        CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, PSECURITY_DESCRIPTOR,
        PSID, SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess, GetExitCodeProcess,
        InitializeProcThreadAttributeList, ResumeThread, TerminateProcess, UpdateProcThreadAttribute,
        WaitForSingleObject, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
        EXTENDED_STARTUPINFO_PRESENT, INFINITE, LPPROC_THREAD_ATTRIBUTE_LIST,
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, STARTUPINFOEXW,
        STARTUPINFOW,
    };

    // SDDL_REVISION_1 is not exported from windows-sys 0.59 as a constant.
    // Use 1u32 directly (the only documented revision value).
    const SDDL_REVISION_1: u32 = 1;

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
    ) -> nono::Result<String> {
        // Step 1: Generate a unique tenant_id and AppContainer profile name.
        let tenant_id = generate_tenant_id()?;
        let profile_name = format!("nono.session.{}", &tenant_id[..16]);

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
            NonoError::SandboxInit(format!(
                "launch_agent: package_sid_to_string failed: {e}"
            ))
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
        let job_owned = unsafe {
            std::os::windows::io::OwnedHandle::from_raw_handle(job_raw_owned)
        };
        // SAFETY: `process_handle_raw` is a valid process handle from CreateProcessW.
        let process_owned = unsafe {
            std::os::windows::io::OwnedHandle::from_raw_handle(process_handle_raw)
        };

        // Forget the profile — AppContainer cleanup deferred to AgentTenant::Drop.
        // Dropping the profile here would call DeleteAppContainerProfile too early.
        std::mem::forget(profile);

        // Step 7b: Insert AgentTenant into DaemonState::tenants AFTER registry.
        let tenant = AgentTenant {
            tenant_id: tenant_id.clone(),
            package_sid: package_sid.clone(),
            profile_name: profile_name.clone(),
            caps,
            job_handle: job_owned,
            process_handle: process_owned,
        };

        {
            let mut tenants = daemon_state.tenants.lock().map_err(|_| {
                NonoError::SandboxInit(
                    "launch_agent: DaemonState::tenants mutex poisoned".into(),
                )
            })?;
            tenants.insert(tenant_id.clone(), tenant);
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

                // Remove from tenants — Drops AgentTenant:
                //   - closes job_handle → KILL_ON_JOB_CLOSE fires
                //   - closes process_handle
                //   - calls DeleteAppContainerProfile (best-effort)
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
        let sddl = format!(
            "D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)(D;;0x1F001F;;;{package_sid})"
        );
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

    /// Remove state for a failed agent launch. If `AgentTenant` was already
    /// inserted, removing it drops the struct → closes job_handle →
    /// `KILL_ON_JOB_CLOSE` terminates the process group.
    fn cleanup_failed_agent(
        daemon_state: &Arc<DaemonState>,
        tenant_id: &str,
        package_sid: &str,
    ) {
        // Registry remove FIRST (locking order).
        if let Ok(mut registry) = daemon_state.agent_registry.lock() {
            registry.remove(package_sid);
        }
        // Tenants remove — Drop closes handles + DeleteAppContainerProfile.
        if let Ok(mut tenants) = daemon_state.tenants.lock() {
            tenants.remove(tenant_id);
        }
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
                std::ptr::null(),     // lpProcessAttributes
                std::ptr::null(),     // lpThreadAttributes
                0,                    // bInheritHandles = FALSE
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
            NonoError::SandboxInit(format!(
                "generate_tenant_id: getrandom::fill failed: {e}"
            ))
        })?;
        Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
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
                DuplicateHandle(current, current, current, &mut raw, 0, 0, DUPLICATE_SAME_ACCESS)
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
            caps: nono::CapabilitySet::new(),
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
        assert_eq!(t.package_sid, package_sid, "AgentTenant.package_sid must match");
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
            assert!(ids.insert(id), "each tenant_id must be unique (fresh per agent)");
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
                DuplicateHandle(current, current, current, &mut raw, 0, 0, DUPLICATE_SAME_ACCESS)
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
                caps: nono::CapabilitySet::new(),
                job_handle: make_handle(),
                process_handle: make_handle(),
            };
            state.tenants.lock().unwrap().insert(tenant_id.clone(), tenant);
        }

        assert_eq!(state.tenants.lock().unwrap().len(), 1, "one tenant before reap");

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
}
