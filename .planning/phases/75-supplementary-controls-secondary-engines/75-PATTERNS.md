# Phase 75: Supplementary Controls + Secondary Engines - Pattern Map

**Mapped:** 2026-06-15
**Files analyzed:** 9 new/modified files across two repos
**Analogs found:** 9 / 9

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/data/policy.json` | config | transform | `aider` / `langchain-python` profiles + `node-dev` profile (same file, lines 884-954) | exact — same schema, native-PE shape mirrors `node-dev` not `aider` (no `windows_interpreters`) |
| `crates/nono-cli/src/agent_daemon/control_loop.rs` | service | event-driven | same file, `Launch`/`List`/`Shutdown` variants + `handle_launch`/`handle_list` (lines 315-570) | exact — adding `Demote` as 4th variant in existing `ControlRequest` enum |
| `crates/nono-cli/src/agent_daemon/launch.rs` | service | event-driven | same file, `launch_agent` step sequence + `cleanup_failed_agent` (lines 98-325) | exact — SUPP-02 WFP-add hooks between steps 6 and 8; D-05 gate is new fail-secure point |
| `crates/nono-cli/src/agent_daemon/reap.rs` | service | event-driven | same file, `AgentTenant::Drop` impl (lines 108-138) | exact — WFP-remove call fires from the reap task before `tenants.remove()`, keeping Drop clean |
| `crates/nono-cli/src/bin/nono-wfp-service.rs` | service | request-response | same file, `install_wfp_policy_filters` / `remove_wfp_policy_filters` / `sid_to_security_descriptor` (lines 1328-1620) | exact — daemon reuses existing wire protocol; no new service code needed |
| `crates/nono-cli/src/agent_cli.rs` | controller | request-response | same file, `agent_launch` / `agent_list` functions (lines 573-646) | exact — `agent_demote` is a 3rd `AgentCommands` dispatch branch |
| `crates/nono-cli/src/cli.rs` | config | transform | same file, `AgentCommands` enum + `AgentLaunchArgs` struct (lines 3208-3225) | exact — `Demote` variant mirrors `List` (single string arg) |
| `../nono-ts/src/windows_confined_run.rs` (NEW) | utility | request-response | `../nono-py/src/windows_confined_run.rs` (full file, lines 1-496) | exact — direct port; only surface differences are napi vs pyo3 error types |
| `../nono-ts/src/lib.rs` + `../nono-ts/Cargo.toml` | utility | transform | `../nono-ts/src/lib.rs` existing `JsCapabilitySet` / `apply` pattern (lines 1-376) + `../nono-py/src/windows_confined_run.rs` napi struct shapes | exact — `JsExecResult` and `confined_run`/`confine` napi exports follow existing `#[napi(object)]` / `#[napi]` pattern |

---

## Pattern Assignments

### `crates/nono-cli/data/policy.json` — `"copilot-cli"` engine profile (config, transform)

**Analog:** `crates/nono-cli/data/policy.json`, `"aider"` profile (lines 884-901) + `"node-dev"` profile (lines 938-954)

**Key insight from research:** Copilot CLI is a NATIVE PE (`copilot.exe`), NOT a node.exe-wrapped npm script. Profile shape follows `node-dev` (bare-exe, no `windows_interpreters`) combined with the `windows_low_il_broker: true` flag from `aider`. No `windows_interpreters` field.

**Imports pattern (aider profile, lines 884-901):**
```json
"aider": {
  "extends": "default",
  "meta": {
    "name": "aider",
    "version": "1.0.0",
    "description": "Aider AI pair-programming engine (Python entry point)",
    "author": "nono-project"
  },
  "security": {
    "groups": ["python_runtime", "git_config", "unlink_protection"],
    "signal_mode": "isolated"
  },
  "filesystem": {},
  "network": { "block": false },
  "workdir": { "access": "readwrite" },
  "windows_low_il_broker": true,
  "windows_interpreters": ["python.exe"]
}
```

**Copilot-cli shape (node-dev bare-exe analog, lines 938-954):**
```json
"node-dev": {
  "extends": "default",
  "meta": {
    "name": "node-dev",
    "version": "1.0.0",
    "description": "Node.js SDK development profile with nvm, fnm, pnpm, and npm support",
    "author": "nono-project"
  },
  "security": {
    "groups": ["node_runtime"],
    "signal_mode": "isolated"
  },
  "filesystem": {},
  "network": { "block": false, "network_profile": "developer" },
  "workdir": { "access": "readwrite" },
  "interactive": false
}
```

**Target copilot-cli profile (combine above, omit `windows_interpreters`, add broker flag):**
```json
"copilot-cli": {
  "extends": "default",
  "meta": {
    "name": "copilot-cli",
    "version": "1.0.0",
    "description": "GitHub Copilot CLI (copilot.exe native PE engine)",
    "author": "nono-project"
  },
  "security": {
    "groups": [],
    "signal_mode": "isolated"
  },
  "filesystem": {},
  "network": { "block": false },
  "workdir": { "access": "readwrite" },
  "windows_low_il_broker": true
}
```
Note: no `"windows_interpreters"` field. If SC3 UAT shows `copilot.exe` spawning `node.exe` as a grandchild (Pitfall 5), add `"windows_interpreters": ["node.exe"]` at that point. `"groups": []` is a placeholder — confirm whether `node_runtime` group coverage helps or not by tracing the actual install paths on the Win11 test host via `where copilot`.

---

### `crates/nono-cli/src/agent_daemon/control_loop.rs` — `Demote` variant + `handle_demote` (service, event-driven)

**Analog:** same file, `ControlRequest` enum (lines 313-325) + `handle_launch` / `handle_list` dispatch pattern (lines 418-611)

**Imports pattern (lines 57-68):**
```rust
use super::super::DaemonState;
use nono::NonoError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
// ... (existing imports in windows_impl)
```
For `handle_demote`, add:
```rust
use windows_sys::Win32::Security::{
    CreateWellKnownSid, WinLowLabelSid, TOKEN_ADJUST_DEFAULT, TOKEN_QUERY,
    TOKEN_MANDATORY_LABEL, SE_GROUP_INTEGRITY,
};
use windows_sys::Win32::System::Threading::{OpenProcessToken, SetTokenInformation, TokenIntegrityLevel};
use windows_sys::Win32::System::Sid::GetLengthSid;
use windows_sys::Win32::Foundation::SECURITY_MAX_SID_SIZE;
```

**ControlRequest enum extension (lines 313-325):**
```rust
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
    // ADD:
    /// `{"action":"demote","tenant_id":"<hex>"}` — post-hoc IL-drop on a running agent.
    ///
    /// SUPP-01: incident-response lever ONLY. Not a standalone confinement boundary.
    /// Demote is one-way; it does not reap the agent (D-03).
    Demote {
        tenant_id: String,
    },
}
```

**Dispatch pattern to copy from `handle_launch` (lines 418-441):**
```rust
let response = match request {
    ControlRequest::Launch { profile: profile_name, cmd } => {
        handle_launch(&state, &profile_name, cmd).await
    }
    ControlRequest::List => handle_list(&state),
    ControlRequest::Shutdown => { /* ... existing ... */ }
    // ADD:
    ControlRequest::Demote { tenant_id } => {
        handle_demote(&state, &tenant_id)  // synchronous: no await needed
    }
};
```

**handle_demote function pattern (modeled on handle_list, lines 576-611):**
```rust
/// Dispatch a `demote` request — post-hoc IL-drop on an already-born-confined agent.
///
/// # Security: SUPP-01 leak limits (spike-002, documented here per D-01)
///
/// 1. Handles opened BEFORE the IL-drop continue to function at Medium IL.
/// 2. Already-started child processes are NOT retroactively affected.
/// 3. The IL-drop may sever legitimate handles (agent may crash/malfunction).
/// 4. Outbound network is NOT automatically blocked; SUPP-02 WFP filter is
///    removed as part of demote (D-03 planning call: YES, cut egress too).
/// 5. Demote is one-way: no API to raise IL back to Medium from outside.
///
/// Does NOT reap/kill the agent (D-03).
fn handle_demote(state: &Arc<DaemonState>, tenant_id: &str) -> String {
    use std::os::windows::io::AsRawHandle;

    // Lock, clone the raw HANDLE, then release the lock (Pitfall 1 mitigation:
    // do NOT hold the lock during Win32 IL-drop calls).
    let (process_raw, package_sid) = {
        let tenants = state.tenants.lock().unwrap_or_else(|p| p.into_inner());
        match tenants.get(tenant_id) {
            None => return format!(
                "error: tenant_id '{tenant_id}' not found — \
                 run `nono agent list` for current tenant IDs"
            ),
            Some(t) => {
                let raw = t.process_handle.as_raw_handle() as HANDLE;
                (raw, t.package_sid.clone())
            }
        }
    };
    // Note: raw HANDLE from OwnedHandle is valid; we do NOT close it here —
    // AgentTenant still owns the handle.  We DuplicateHandle to get our own
    // copy (mirrors duplicate_process_handle_for_reap in launch.rs).
    // ... (DuplicateHandle → demote_tenant_il → CloseHandle dup)
    // ... then optionally: wfp_filter_remove(package_sid)
    todo!("implement demote")
}
```

**IL-drop Win32 path (from RESEARCH.md — spike-002 proven pattern):**
```rust
// windows-cfg-gated helper; uses windows-sys 0.59 (no new imports at workspace level)
fn demote_tenant_il(process_handle: HANDLE) -> nono::Result<()> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: TOKEN_ADJUST_DEFAULT | TOKEN_QUERY is sufficient for
    // SetTokenInformation(TokenIntegrityLevel) on same-user processes.
    let ok = unsafe {
        OpenProcessToken(process_handle, TOKEN_ADJUST_DEFAULT | TOKEN_QUERY, &mut token)
    };
    if ok == 0 {
        return Err(NonoError::SandboxInit(format!(
            "demote_tenant_il: OpenProcessToken failed: GLE={}",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        )));
    }
    // RAII: close token handle on all exit paths.
    struct TokenGuard(HANDLE);
    impl Drop for TokenGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CloseHandle(self.0) };
            }
        }
    }
    let _token_guard = TokenGuard(token);

    let mut low_label: TOKEN_MANDATORY_LABEL = unsafe { std::mem::zeroed() };
    let mut low_sid = [0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut sid_size = SECURITY_MAX_SID_SIZE;
    // SAFETY: WinLowLabelSid (9) is the documented constant for Low Integrity.
    unsafe {
        CreateWellKnownSid(WinLowLabelSid, std::ptr::null_mut(),
            low_sid.as_mut_ptr().cast(), &mut sid_size)
    };
    low_label.Label.Sid = low_sid.as_mut_ptr().cast();
    low_label.Label.Attributes = SE_GROUP_INTEGRITY;

    // SAFETY: low_label is a valid TOKEN_MANDATORY_LABEL pointing to a valid SID.
    let size = std::mem::size_of::<TOKEN_MANDATORY_LABEL>() as u32
        + unsafe { GetLengthSid(low_label.Label.Sid) };
    let ok = unsafe {
        SetTokenInformation(
            token,
            TokenIntegrityLevel,
            &low_label as *const TOKEN_MANDATORY_LABEL as *mut _,
            size,
        )
    };
    if ok == 0 {
        return Err(NonoError::SandboxInit(format!(
            "demote_tenant_il: SetTokenInformation(TokenIntegrityLevel) failed: GLE={}",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        )));
    }
    Ok(())
}
```

**Error handling pattern (from handle_launch, lines 561-569):**
```rust
Err(e) => {
    tracing::warn!(
        error = %e,
        tenant_id = %tenant_id,
        "handle_demote: IL-drop failed"
    );
    format!("error: demote failed for tenant '{tenant_id}': {e}")
}
```

**Test pattern (from existing control_loop.rs tests, lines 654-793):**
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    // ...
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn demote_returns_err_for_unknown_tenant_cross_platform() {
        let state = empty_state();
        let result = handle_demote_testable(&state, "nonexistent-tenant-id");
        assert!(result.contains("error:"), "must return error for unknown tenant");
        assert!(result.contains("not found"), "must name the missing tenant");
    }
}
```

---

### `crates/nono-cli/src/agent_daemon/launch.rs` — WFP filter add at launch (service, event-driven)

**Analog:** same file, `launch_agent` steps 6-8 (lines 177-262) + `cleanup_failed_agent` (lines 513-526)

**Hook point — BETWEEN steps 6 and 7 (lines 190-213 in launch.rs):**
```rust
// Step 6: Assign the process to the job (fail-secure: terminate on failure).
if let Err(e) = assign_process_to_agent_job(job_guard.0, process_handle_raw) {
    // ... existing cleanup ...
    return Err(...);
}

// >>> INSERT SUPP-02 HERE (before step 7a registry insert, before ResumeThread) <<<
// Step 6.5: Add per-agent WFP filter (D-04 daemon→service coupling, D-05 fail-secure).
//
// Pitfall 3: MUST happen BEFORE ResumeThread. If the WFP service is absent AND
// the profile needs network scoping, terminate the suspended process and return Err.
if profile_needs_network_scoping(&engine_profile) {
    match wfp_filter_add(&package_sid, &tenant_id) {
        Ok(()) => {
            tracing::info!(tenant_id = %tenant_id, package_sid = %package_sid,
                "launch_agent: per-agent WFP filter installed (SUPP-02)");
        }
        Err(e) => {
            // D-05: refuse to launch if WFP scope cannot be enforced.
            // SAFETY: handles are valid; terminate the suspended process.
            unsafe { TerminateProcess(process_handle_raw, 1) };
            unsafe { CloseHandle(process_handle_raw) };
            unsafe { CloseHandle(thread_handle_raw) };
            // job_guard drops here → closes job handle.
            return Err(NonoError::SandboxInit(format!(
                "launch_agent: WFP network scope required by profile \
                 '{engine_profile}' but nono-wfp-service is not reachable: {e}\n\
                 Install and start nono-wfp-service before launching this profile."
            )));
        }
    }
}

// Step 7a: Insert package SID into AgentRegistry FIRST.
// ...existing code...
```

**wfp_filter_add helper (new, reuses windows_wfp_contract.rs + existing pipe protocol):**
```rust
/// Send a WFP activation request to the elevated nono-wfp-service for a per-agent
/// AppContainer package SID (SUPP-02). Reuses WfpRuntimeActivationRequest verbatim.
///
/// # Fail-secure
///
/// Any error (service absent, pipe error, NACK response) returns Err.
/// Caller MUST terminate the suspended process before returning Err (D-05).
fn wfp_filter_add(package_sid: &str, tenant_id: &str) -> nono::Result<()> {
    use crate::windows_wfp_contract::{WfpRuntimeActivationRequest, WFP_RUNTIME_PROTOCOL_VERSION};

    let req = WfpRuntimeActivationRequest {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        request_kind: "activate".to_string(),
        network_mode: "blocked".to_string(),
        preferred_backend: "wfp".to_string(),
        active_backend: "wfp".to_string(),
        runtime_target: String::new(),
        tcp_connect_ports: vec![443, 80],  // from profile; "blocked" = deny-all
        tcp_bind_ports: vec![],
        localhost_ports: vec![],
        target_program_path: None,
        session_sid: Some(package_sid.to_string()),  // per-agent E4 SID
        outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
        inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
    };
    send_wfp_service_request(&req)
}
```

**Fail-secure cleanup pattern (from cleanup_failed_agent, lines 513-526):**
```rust
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
```

---

### `crates/nono-cli/src/agent_daemon/reap.rs` — WFP filter remove at reap (service, event-driven)

**Analog:** same file, `AgentTenant::Drop` impl (lines 108-138)

**Preferred pattern: fire WFP deactivation from the reap TASK, not from Drop (Pitfall 2 mitigation).**

The reap task sequence in `launch.rs` lines 310-322 is the hook point:
```rust
// Remove from registry FIRST (locking order: registry → tenants).
if let Ok(mut registry) = reap_daemon_state.agent_registry.lock() {
    registry.remove(&reap_package_sid);
}

// >>> INSERT SUPP-02 DEACTIVATION HERE (before tenants.remove → Drop) <<<
// Send WFP deactivation synchronously before dropping the AgentTenant.
// Using blocking std::fs pipe write (Pitfall 2: do NOT use tokio pipe in Drop).
if let Err(e) = wfp_filter_remove(&reap_package_sid, &reap_tenant_id) {
    tracing::warn!(
        tenant_id = %reap_tenant_id,
        error = %e,
        "launch_agent reap: WFP filter removal failed (best-effort; \
         service startup sweep will reclaim stale filters)"
    );
    // Non-fatal: the WFP service's startup sweep handles orphaned filters (Pitfall 6).
}

// Remove from tenants — Drops AgentTenant:
//   - closes job_handle → KILL_ON_JOB_CLOSE fires
//   - closes process_handle
//   - calls DeleteAppContainerProfile (best-effort)
if let Ok(mut tenants) = reap_daemon_state.tenants.lock() {
    tenants.remove(&reap_tenant_id);
}
```

**wfp_filter_remove helper (new, mirrors wfp_filter_add):**
```rust
fn wfp_filter_remove(package_sid: &str, tenant_id: &str) -> nono::Result<()> {
    use crate::windows_wfp_contract::{WfpRuntimeActivationRequest, WFP_RUNTIME_PROTOCOL_VERSION};
    let req = WfpRuntimeActivationRequest {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        request_kind: "deactivate".to_string(),
        network_mode: "blocked".to_string(),
        preferred_backend: "wfp".to_string(),
        active_backend: "wfp".to_string(),
        runtime_target: String::new(),
        tcp_connect_ports: vec![],
        tcp_bind_ports: vec![],
        localhost_ports: vec![],
        target_program_path: None,
        session_sid: Some(package_sid.to_string()),
        outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
        inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
    };
    send_wfp_service_request(&req)
}
```

**AgentTenant::Drop pattern (lines 108-138) — Drop stays focused on handle cleanup only:**
```rust
#[cfg(target_os = "windows")]
impl Drop for AgentTenant {
    fn drop(&mut self) {
        // Step 1: job_handle and process_handle drop automatically via OwnedHandle::drop.
        // Step 2: Delete the AppContainer profile (best-effort).
        if let Err(e) = delete_app_container_profile(&self.profile_name) {
            tracing::warn!(
                tenant_id = %self.tenant_id,
                profile_name = %self.profile_name,
                error = %e,
                "Failed to delete AppContainer profile on agent reap (best-effort)"
            );
        }
        tracing::info!(
            tenant_id = %self.tenant_id,
            package_sid = %self.package_sid,
            "AgentTenant reaped: job handle closed, AppContainer profile cleanup attempted"
        );
        // NOTE: WFP filter deactivation is handled in the reap TASK (not here)
        // to avoid blocking pipe I/O inside Drop. See launch.rs reap task.
    }
}
```

---

### `crates/nono-cli/src/bin/nono-wfp-service.rs` — per-agent (package-SID) filter add/remove path

**No new code needed in this file.** The existing `install_wfp_policy_filters` and `remove_wfp_policy_filters` already handle the `session_sid` case. The daemon sends `session_sid = package_sid` and the service routes to the correct SID-keyed filter path.

**Existing pattern the daemon will invoke (lines 1548-1619):**
```rust
// install_wfp_policy_filters branches on session_sid:
let (app_id, sd) = if let Some(sid_str) = &request.session_sid {
    let sd = sid_to_security_descriptor(sid_str)?;  // validates + builds D:(A;;CC;;;<sid>)
    (None, Some(sd))
} else {
    let app_id = get_app_id_blob(target_program)?;
    (Some(app_id), None)
};
// ... adds FWPM_CONDITION_ALE_USER_ID filter keyed to the SD ...
```

**sid_to_security_descriptor pattern (lines 1328-1365) — validation + SDDL construction:**
```rust
fn sid_to_security_descriptor(sid_str: &str) -> Result<WfpSecurityDescriptor, String> {
    // Phase 1: validate the SID string via ConvertStringSidToSidW.
    let sid_wide = to_utf16_null(std::ffi::OsStr::new(sid_str));
    let mut sid = null_mut();
    let status = unsafe { ConvertStringSidToSidW(sid_wide.as_ptr(), &mut sid) };
    if status == 0 {
        return Err(format_windows_error(unsafe { GetLastError() },
            &format!("invalid SID string: {}", sid_str)));
    }
    unsafe { LocalFree(sid as _) };

    // Phase 2: convert SDDL "D:(A;;CC;;;<sid>)" to a security descriptor.
    let sddl = format!("D:(A;;CC;;;{sid_str})");
    let sddl_wide = to_utf16_null(std::ffi::OsStr::new(&sddl));
    let mut sd = null_mut();
    let status = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_wide.as_ptr(), SDDL_REVISION_1, &mut sd, null_mut())
    };
    if status == 0 {
        return Err(format_windows_error(unsafe { GetLastError() },
            &format!("failed to convert SDDL '{}' to security descriptor", sddl)));
    }
    Ok(WfpSecurityDescriptor(sd))
}
```

**WfpRuntimeActivationRequest wire shape (windows_wfp_contract.rs, lines 1-27):**
```rust
pub const WFP_RUNTIME_PROTOCOL_VERSION: u32 = 1;

pub struct WfpRuntimeActivationRequest {
    pub protocol_version: u32,
    pub request_kind: String,   // "activate" | "deactivate"
    pub network_mode: String,   // "blocked" | "proxy-only" | "allow-all"
    pub preferred_backend: String,
    pub active_backend: String,
    pub runtime_target: String,
    pub tcp_connect_ports: Vec<u16>,
    pub tcp_bind_ports: Vec<u16>,
    pub localhost_ports: Vec<u16>,
    pub target_program_path: Option<String>,
    pub session_sid: Option<String>,        // per-agent AppContainer package SID
    pub outbound_rule_name: Option<String>, // "nono-agent-{tenant_id}"
    pub inbound_rule_name: Option<String>,  // "nono-agent-{tenant_id}-in"
}
```

---

### `crates/nono-cli/src/agent_cli.rs` — `agent_demote` function (controller, request-response)

**Analog:** same file, `agent_launch` (lines 573-608) and `agent_list` (lines 617-646)

**agent_launch pattern to copy (lines 573-608):**
```rust
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
```

**agent_demote follows the same shape:**
```rust
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
```

**run_agent dispatch extension (lines 556-560):**
```rust
pub(crate) fn run_agent(args: AgentArgs) -> Result<()> {
    match args.command {
        AgentCommands::Launch(launch_args) => agent_launch(launch_args),
        AgentCommands::List => agent_list(),
        // ADD:
        AgentCommands::Demote { tenant_id } => agent_demote(tenant_id),
    }
}
```

---

### `crates/nono-cli/src/cli.rs` — `AgentCommands::Demote` variant (config, transform)

**Analog:** same file, `AgentCommands` enum (lines 3208-3225)

**Current enum (lines 3208-3225):**
```rust
#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// Launch a confined agent through the daemon
    Launch(AgentLaunchArgs),
    /// List running confined agents (tenant IDs and package SIDs)
    List,
}
```

**Extension — add Demote variant:**
```rust
#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// Launch a confined agent through the daemon
    Launch(AgentLaunchArgs),
    /// List running confined agents (tenant IDs and package SIDs)
    List,
    // ADD:
    /// Apply a post-hoc IL-drop (supplementary incident-response lever) to a running agent.
    ///
    /// SUPP-01: demote is a further IL-drop + WFP-cut on an already-born-confined
    /// agent. It is NOT a standalone confinement boundary. Leak limits apply:
    /// handles opened before the drop continue at Medium IL; already-started
    /// children are not retroactively affected; the IL-drop may crash the agent.
    /// Use `nono agent list` to find tenant IDs.
    Demote {
        /// Tenant ID from `nono agent list` (32-char hex string)
        tenant_id: String,
    },
}
```

**Test pattern (from existing agent_list_parses test, lines 964-976):**
```rust
#[test]
fn agent_demote_parses() {
    let cli = Cli::parse_from(["nono", "agent", "demote", "abcdef1234567890abcdef1234567890"]);
    let Commands::Agent(AgentArgs {
        command: AgentCommands::Demote { ref tenant_id },
    }) = cli.command
    else {
        panic!("expected Commands::Agent(AgentCommands::Demote(...))");
    };
    assert_eq!(tenant_id, "abcdef1234567890abcdef1234567890");
}
```

---

### `../nono-ts/src/windows_confined_run.rs` (NEW utility, request-response)

**Analog:** `../nono-py/src/windows_confined_run.rs` (full file, lines 1-496) — direct port; all logic identical, only surface differences are napi error types vs pyo3 error types.

**Module-level cfg gate (mirrors nono-py line 11):**
```rust
#![cfg(windows)]
// No `use pyo3::...` — use napi::Error / Status instead.
use napi::Error;
use napi::Status;
use std::io::Read as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
```

**find_nono_exe pattern (lines 66-94 in nono-py — copy exactly, replace PyResult with napi::Result):**
```rust
fn find_nono_exe() -> napi::Result<PathBuf> {
    if let Some(val) = std::env::var_os("NONO_EXE") {
        let path = PathBuf::from(val);
        if path.is_file() {
            return Ok(path);
        }
        return Err(Error::new(Status::GenericFailure, format!(
            "NONO_EXE is set to '{}' but that path is not an existing file",
            path.display()
        )));
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("nono.exe");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(Error::new(Status::GenericFailure,
        "nono.exe not found. Set NONO_EXE to its path or add it to PATH."))
}
```

**JsExecResult struct (mirrors ExecResult from nono-py lines 33-40, using napi(object)):**
```rust
#[napi(object)]
pub struct JsExecResult {
    pub stdout: Vec<u8>,   // napi Buffer
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}
```

**confined_run signature (mirrors nono-py lines 175-215, adapted for napi):**
```rust
pub(crate) fn confined_run(
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,
    profile: Option<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    if profile.is_none() && allow.as_ref().map_or(true, |v| v.is_empty()) {
        return Err(Error::new(Status::InvalidArg,
            "confined_run: at least one of 'profile' or 'allow' must be provided"));
    }
    let nono_path = find_nono_exe()?;
    let mut cmd = Command::new(&nono_path);
    cmd.arg("run");
    build_nono_run_args(&mut cmd, profile.as_deref(), allow.as_deref(), cwd.as_deref());
    cmd.arg("--").arg(&exe).args(&args);
    if let Some(ref d) = cwd {
        cmd.current_dir(d);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    do_spawn_and_wait(cmd, timeout_secs)
}
```

**confine signature (mirrors nono-py lines 253-325, adapted for napi):**
```rust
pub(crate) fn confine(
    profile: Option<String>,
    allow: Option<Vec<String>>,
) -> napi::Result<()> {
    // FIRST: Born-confined guard (T-72-02-01 / T-72-02-04).
    if std::env::var("NONO_ALREADY_CONFINED").as_deref() == Ok("1") {
        return Ok(());
    }
    let allow_is_empty = allow.as_ref().map_or(true, |v| v.is_empty());
    if profile.is_none() && allow_is_empty {
        return Err(Error::new(Status::InvalidArg,
            "confine: at least one of 'profile' or 'allow' must be provided"));
    }
    let nono_path = find_nono_exe()?;
    let current_exe = std::env::current_exe()
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;
    let original_args: Vec<String> = std::env::args().skip(1).collect();
    let mut cmd = Command::new(&nono_path);
    cmd.arg("run");
    build_nono_run_args(&mut cmd, profile.as_deref(), allow.as_deref(), None);
    cmd.env("NONO_ALREADY_CONFINED", "1");
    cmd.arg("--").arg(&current_exe).args(&original_args);
    let mut child = cmd.spawn()
        .map_err(|e| Error::new(Status::GenericFailure,
            format!("confine: failed to spawn nono.exe: {}", e)))?;
    let status = child.wait()
        .map_err(|e| Error::new(Status::GenericFailure,
            format!("confine: failed to wait for nono.exe: {}", e)))?;
    let exit_code = status.code().unwrap_or(1);
    std::process::exit(exit_code);
}
```

**Test pattern (mirrors nono-py tests lines 428-495):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_nono_exe_from_env_var() {
        let test_exe = std::env::current_exe().expect("test exe must be locatable");
        let test_exe_str = test_exe.to_string_lossy().to_string();
        // Save and restore NONO_EXE (env-var test isolation per CLAUDE.md).
        let prev = std::env::var_os("NONO_EXE");
        unsafe { std::env::set_var("NONO_EXE", &test_exe_str) };
        let result = find_nono_exe();
        unsafe {
            match prev {
                Some(v) => std::env::set_var("NONO_EXE", v),
                None => std::env::remove_var("NONO_EXE"),
            }
        }
        let found = result.expect("NONO_EXE pointing at existing file should succeed");
        assert_eq!(found, test_exe);
    }

    #[test]
    fn test_find_nono_exe_not_found_returns_err() {
        let prev_nono_exe = std::env::var_os("NONO_EXE");
        let prev_path = std::env::var_os("PATH");
        unsafe {
            std::env::remove_var("NONO_EXE");
            std::env::set_var("PATH", "");
        }
        let result = find_nono_exe();
        unsafe {
            match prev_nono_exe {
                Some(v) => std::env::set_var("NONO_EXE", v),
                None => std::env::remove_var("NONO_EXE"),
            }
            match prev_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }
        assert!(result.is_err(), "find_nono_exe() must return Err when nono.exe is absent");
    }
}
```

---

### `../nono-ts/src/lib.rs` + `../nono-ts/Cargo.toml` (utility, transform)

**Analog:** `../nono-ts/src/lib.rs` existing `#[napi]` / `#[napi(object)]` export pattern (lines 31-375) + nono-py windows_confined_run.rs non-Windows stub pattern

**Existing napi export pattern to copy (lib.rs lines 110-202):**
```rust
#[napi]
impl JsCapabilitySet {
    #[napi(constructor)]
    pub fn new() -> Self { ... }

    #[napi]
    pub fn allow_path(&mut self, path: String, mode: AccessMode) -> Result<()> { ... }
    // ... etc
}

#[napi]
pub fn apply(caps: &JsCapabilitySet) -> Result<()> { ... }

#[napi(js_name = "isSupported")]
pub fn is_supported() -> bool { ... }
```

**cfg-gated exports to add to lib.rs (Windows + non-Windows stubs):**
```rust
// ---------------------------------------------------------------------------
// Windows confined execution (SUPP-03b)
// ---------------------------------------------------------------------------

// JsExecResult is always declared (needed for the non-Windows stub signatures).
#[napi(object)]
pub struct JsExecResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

#[cfg(target_os = "windows")]
mod windows_confined_run;

/// Run an executable in a confined child process (Shape A). Windows-only.
///
/// Spawns `nono.exe run --profile <profile> --allow <path>… -- <exe> <args>`.
#[napi(js_name = "confinedRun")]
#[cfg(target_os = "windows")]
pub fn confined_run(
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,
    profile: Option<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    windows_confined_run::confined_run(exe, args, allow, profile, cwd, timeout_secs)
}

#[napi(js_name = "confinedRun")]
#[cfg(not(target_os = "windows"))]
pub fn confined_run(
    _exe: String, _args: Vec<String>, _allow: Option<Vec<String>>,
    _profile: Option<String>, _cwd: Option<String>, _timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    Err(napi::Error::new(napi::Status::GenericFailure, "confinedRun is Windows-only"))
}

/// Make the current process born-confined via the nono.exe broker (Shape B). Windows-only.
#[napi]
#[cfg(target_os = "windows")]
pub fn confine(
    profile: Option<String>,
    allow: Option<Vec<String>>,
) -> napi::Result<()> {
    windows_confined_run::confine(profile, allow)
}

#[napi]
#[cfg(not(target_os = "windows"))]
pub fn confine(
    _profile: Option<String>,
    _allow: Option<Vec<String>>,
) -> napi::Result<()> {
    Err(napi::Error::new(napi::Status::GenericFailure, "confine is Windows-only"))
}
```

**Cargo.toml pin bump (single change):**
```toml
# ../nono-ts/Cargo.toml — BEFORE:
nono = { version = "0.33.0" }

# AFTER:
nono = { version = "0.62" }
```
All other pins unchanged: `napi = { version = "2", ... }`, `napi-derive = "2"`, `napi-build = "2"`.

---

## Shared Patterns

### Windows cfg-gating
**Source:** `crates/nono-cli/src/agent_daemon/control_loop.rs` lines 51-55 + `../nono-py/src/windows_confined_run.rs` line 11
**Apply to:** all new agent_daemon code, all nono-ts windows_confined_run.rs, nono-ts lib.rs exports
```rust
// Module-level gate (whole-module Windows-only):
#![cfg(windows)]   // in windows_confined_run.rs

// Re-export gate (selective, for lib.rs exports):
#[cfg(target_os = "windows")]
mod windows_confined_run;

// Function-level dual (for non-Windows stubs):
#[napi]
#[cfg(target_os = "windows")]
pub fn confined_run(...) -> napi::Result<JsExecResult> { windows_confined_run::confined_run(...) }

#[napi]
#[cfg(not(target_os = "windows"))]
pub fn confined_run(...) -> napi::Result<JsExecResult> {
    Err(napi::Error::new(napi::Status::GenericFailure, "confinedRun is Windows-only"))
}
```

### Error Handling
**Source:** `crates/nono-cli/src/agent_daemon/control_loop.rs` `handle_launch` error path (lines 561-569) + `agent_cli.rs` `agent_launch` error path (lines 592-597)
**Apply to:** all new daemon handlers + agent_cli demote function
```rust
// Daemon handler error path:
Err(e) => {
    tracing::warn!(error = %e, "handle_demote: failed");
    format!("error: {e}")
}

// Agent CLI error path:
Err(e) if is_pipe_not_found(&e) => Err(NonoError::SandboxInit(
    "nono-agentd is not running. Use `nono daemon start` first.".into(),
)),
Err(e) => Err(e),
```

### RAII Handle Guards
**Source:** `crates/nono-cli/src/agent_daemon/launch.rs` `JobGuard` + `AttrListGuard` + `SdGuard` patterns (lines 161-170, 374-385, 682-692)
**Apply to:** `demote_tenant_il` token handle guard
```rust
struct TokenGuard(HANDLE);
impl Drop for TokenGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: self.0 was opened by OpenProcessToken; sole owner.
            unsafe { CloseHandle(self.0) };
        }
    }
}
let _token_guard = TokenGuard(token);
```

### Fail-Secure Pattern
**Source:** `crates/nono-cli/src/agent_daemon/launch.rs` `assign_process_to_agent_job` failure path (lines 178-190) + D-05 from CONTEXT.md
**Apply to:** `wfp_filter_add` failure in `launch_agent`
```rust
// Any failure in the new gating step must mirror the job-assign failure path:
if let Err(e) = wfp_filter_add(...) {
    unsafe { TerminateProcess(process_handle_raw, 1) };
    unsafe { CloseHandle(process_handle_raw) };
    unsafe { CloseHandle(thread_handle_raw) };
    // job_guard drops → closes job → KILL_ON_JOB_CLOSE
    return Err(NonoError::SandboxInit(format!("...actionable error naming the missing service: {e}")));
}
```

### SAFETY Comment Convention
**Source:** Throughout `launch.rs` and `control_loop.rs`
**Apply to:** all new `unsafe` blocks
```rust
// SAFETY: <why the invariant holds — pointer validity, ownership, lifetime, API contract>.
```

### Test Module Pattern
**Source:** `crates/nono-cli/src/agent_daemon/control_loop.rs` lines 654-660 + `agent_cli.rs` lines 888-893
**Apply to:** all new test modules
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*; // or explicit imports
    // ...
}
```

---

## No Analog Found

None — all files have strong analogs in the existing codebase or sibling repo.

---

## Cross-Target Verification Note

Per CLAUDE.md MUST rule: any cfg-gated code in nono-ts (`windows_confined_run.rs` + `#[cfg(not(target_os = "windows"))]` stubs in `lib.rs`) requires:
```
cargo clippy --target x86_64-unknown-linux-gnu   # from ../nono-ts/ directory
cargo clippy --target x86_64-apple-darwin        # from ../nono-ts/ directory
```
This is the **nono-ts workspace**, NOT the nono workspace — the nono repo's cross-target check does NOT cover sibling repos. If cross-toolchain is absent on the Win11 host, mark PARTIAL and defer to CI per `.planning/templates/cross-target-verify-checklist.md`.

The daemon/WFP additions in `crates/nono-cli/src/agent_daemon/` are also cfg-gated Windows code; per CLAUDE.md these need cross-target clippy from the nono workspace. The existing non-Windows stub pattern in `control_loop.rs` (lines 51-52) shows the re-export guard: `#[cfg(target_os = "windows")] pub(crate) use windows_impl::run_control_loop;` — any new public exports from `handle_demote` need the same gate.

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/agent_daemon/`, `crates/nono-cli/src/`, `crates/nono-cli/src/bin/`, `crates/nono-cli/data/`, `../nono-py/src/`, `../nono-ts/src/`
**Files read:** `control_loop.rs`, `agent_cli.rs`, `cli.rs`, `launch.rs`, `reap.rs`, `nono-wfp-service.rs` (targeted sections), `windows_wfp_contract.rs`, `../nono-py/src/windows_confined_run.rs`, `../nono-ts/src/lib.rs`, `../nono-ts/Cargo.toml`, `policy.json` (targeted sections)
**Pattern extraction date:** 2026-06-15
