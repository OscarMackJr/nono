# Phase 74: Persistent Multi-Tenant Daemon - Pattern Map

**Mapped:** 2026-06-14
**Files analyzed:** 9 new/modified files
**Analogs found:** 9 / 9

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/bin/nono-agentd.rs` | service binary | event-driven (SCM dispatch + accept loop) | `crates/nono-cli/src/bin/nono-wfp-service.rs` | exact (same crate, same SCM dispatch shape) |
| `crates/nono-cli/src/agent_daemon/mod.rs` | service module root / state | CRUD + event-driven | `crates/nono-cli/src/exec_strategy_windows/mod.rs` | role-match |
| `crates/nono-cli/src/agent_daemon/accept_loop.rs` | middleware (pipe auth) | request-response | `crates/nono/src/supervisor/socket_windows.rs` (`bind_impl`, `finalize_server_connection`) | role-match (same pipe primitives, adds impersonation) |
| `crates/nono-cli/src/agent_daemon/launch.rs` | service / orchestration | CRUD | `crates/nono-cli/src/exec_strategy_windows/launch.rs` (`create_process_containment`, `apply_process_handle_to_containment`) | exact |
| `crates/nono-cli/src/agent_daemon/reap.rs` | utility / RAII lifecycle | event-driven | `crates/nono-cli/src/exec_strategy_windows/launch.rs` (`ProcessContainment::Drop`) + `nono-wfp-service.rs` (`WfpEngine::Drop`) | role-match |
| `crates/nono-cli/src/agent_cli.rs` | controller (CLI verbs) | request-response | `crates/nono-cli/src/classify_runtime.rs` + `crates/nono-cli/src/cli.rs` (`Classify` variant) | role-match |
| `crates/nono/src/supervisor/socket_windows.rs` (modify) | middleware | request-response | self (`ImpersonateLoggedOnUser`/`RevertToSelf` test at lines 2349-2445; `build_capability_pipe_sddl` at lines 1610-1663) | exact (extending existing file) |
| `crates/nono/src/agent.rs` (modify, promote AgentRegistry) | service (state store) | CRUD | self (lines 78-152; `Arc<Mutex<AgentRegistry>>` usage in `classify_runtime.rs` lines 66-68) | exact |
| `crates/nono-cli/tests/daemon_handle_baseline.rs` | test (integration, host-gated) | batch | `crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs` | role-match |
| `proj/ADR-74-privilege-model.md` | config / decision record | N/A | `proj/DESIGN-supervisor.md` (narrative) | partial (different format; ADR is SC4 gate) |

---

## Pattern Assignments

---

### `crates/nono-cli/src/bin/nono-agentd.rs` (service binary, event-driven)

**Analog:** `crates/nono-cli/src/bin/nono-wfp-service.rs`

**Key delta from analog:** USER privilege (not SYSTEM), `SERVICE_USER_OWN_PROCESS` (not `SERVICE_WIN32_OWN_PROCESS`), NO WFP calls, multi-tenant accept loop in place of the WFP engine loop.

**Non-Windows stub pattern** (lines 18-22 of analog):
```rust
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("nono-agentd is Windows-only");
    std::process::exit(1);
}
```

**Module gate pattern** (lines 24-28 of analog):
```rust
#[cfg(target_os = "windows")]
#[path = "../agent_daemon/mod.rs"]     // daemon logic in a sibling module
mod agent_daemon;

#[cfg(target_os = "windows")]
mod windows_impl { /* ... */ }
```

**`define_windows_service!` / SCM dispatch pattern** (lines 582-651 of analog):
```rust
use windows_service::{
    define_windows_service,
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const SERVICE_NAME: &str = "nono-agentd";
const SERVICE_MODE_ARG: &str = "--service-mode";

define_windows_service!(ffi_service_main, service_main);

fn service_main(arguments: Vec<OsString>) {
    if let Err(e) = run_service(arguments) {
        eprintln!("Service failed: {}", e);
    }
}

fn run_service(_arguments: Vec<OsString>) -> windows_service::Result<()> {
    let shutdown = Arc::new(Notify::new());
    let shutdown_handler = Arc::clone(&shutdown);
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                shutdown_handler.notify_one();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,  // daemon: USER_OWN_PROCESS variant
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| windows_service::Error::Winapi(std::io::Error::other(format!("{e}"))))?;
    rt.block_on(async {
        // daemon: call agent_daemon::run_accept_loop(shutdown).await
    });
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;
    Ok(())
}
```

**SCM dispatch / foreground fallback pattern** (`run_service_mode` at line 806; `run()` dispatch at lines 1758-1791 of analog):
```rust
fn run_service_mode() -> ExitCode {
    match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Failed to start service dispatcher: {}", e);
            ExitCode::from(3)    // non-zero = non-fatal; binary stays up in foreground
        }
    }
}

pub(super) fn run() -> ExitCode {
    match std::env::args().skip(1).next().as_deref() {
        Some(SERVICE_MODE_ARG) => run_service_mode(),
        Some("--foreground") => run_foreground_mode(),  // daemon-specific addition
        Some("--help") | Some("-h") => { print_help(); ExitCode::SUCCESS }
        // ... other modes ...
        None | Some(_) => {
            // non-fatal: fall through to foreground if not in SCM context
            run_foreground_mode()
        }
    }
}
```

**Event Log pattern** (`write_event_log` at lines 161-204; `log_sweep_event` at lines 211-216 of analog):
```rust
const EVENT_LOG_SOURCE: &str = "nono-agentd";
const EVENT_ID_AGENT_LAUNCHED: u32 = 2001;
const EVENT_ID_AGENT_REAPED: u32   = 2002;
const EVENT_ID_AUTH_DENIED: u32    = 2003;

#[cfg(target_os = "windows")]
fn write_event_log(level: EventLogLevel, event_id: u32, body: &str) {
    use windows_sys::Win32::System::EventLog::{
        DeregisterEventSource, RegisterEventSourceW, ReportEventW,
        EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE,
    };
    let source_wide: Vec<u16> = EVENT_LOG_SOURCE.encode_utf16().chain(std::iter::once(0u16)).collect();
    // SAFETY: source_wide is a valid null-terminated UTF-16 string.
    let handle = unsafe { RegisterEventSourceW(std::ptr::null(), source_wide.as_ptr()) };
    if handle.is_null() {
        eprintln!("{}", build_event_log_message(level, event_id, body));
        return;
    }
    // ... ReportEventW + DeregisterEventSource ...
}
```

**`[[bin]]` Cargo.toml entry** (copy the pattern from the single existing `[[bin]]` at line 29 of `crates/nono-cli/Cargo.toml`):
```toml
[[bin]]
name = "nono"
path = "src/main.rs"

# ADD:
[[bin]]
name = "nono-agentd"
path = "src/bin/nono-agentd.rs"
```

---

### `crates/nono-cli/src/agent_daemon/accept_loop.rs` (middleware, request-response)

**Primary analog:** `crates/nono/src/supervisor/socket_windows.rs` — specifically:
- `build_capability_pipe_sddl` (lines 1610-1663): per-SID SDDL construction
- `bind_low_integrity_with_session_and_package_sid` / `bind_impl` (lines 260-351): blocking `ConnectNamedPipe` + rendezvous pattern
- `bind_aipc_pipe` (lines 1099-1133): `PIPE_UNLIMITED_INSTANCES` + Low-IL SDDL
- Test block (lines 2346-2445): `ImpersonateLoggedOnUser` / `RevertToSelf` usage pattern

**PIPE_UNLIMITED_INSTANCES pattern** (lines 1109-1125 of analog):
```rust
let handle: HANDLE = unsafe {
    CreateNamedPipeW(
        wide_name.as_ptr(),
        PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
        PIPE_UNLIMITED_INSTANCES,   // <-- critical for multi-client
        MAX_MESSAGE_SIZE as u32,
        MAX_MESSAGE_SIZE as u32,
        0,
        &sa,
    )
};
```

**Per-tenant SDDL construction** — call the existing function in `socket_windows.rs` (lines 1610-1663):
```rust
// Pass package_sid=Some(tenant_pkg_sid) to embed the tenant's AppContainer ACE.
// session_sid=None because daemon pipe instances are not WRITE_RESTRICTED arm.
let sddl = build_capability_pipe_sddl(None, Some(tenant_pkg_sid))?;
// Result: "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019F;;;S-1-15-2-...)S:(ML;;NW;;;LW)"
```

**`ImpersonateNamedPipeClient` call sequence** — new Win32 calls; analog is the `ImpersonateLoggedOnUser`/`RevertToSelf` test block (lines 2349-2445 of analog); production shape from RESEARCH.md §Unspiked Mechanism 2:
```rust
#[cfg(target_os = "windows")]
fn authenticate_pipe_client(pipe_handle: HANDLE) -> Result<String> {
    use windows_sys::Win32::Security::{
        GetTokenInformation, ImpersonateNamedPipeClient, OpenThreadToken,
        RevertToSelf, TokenAppContainerSid, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThread;

    // SAFETY: pipe_handle is a valid connected named pipe handle.
    let ok = unsafe { ImpersonateNamedPipeClient(pipe_handle) };
    if ok == 0 {
        return Err(NonoError::SandboxInit(format!(
            "ImpersonateNamedPipeClient failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    // ImpersonationGuard MUST call RevertToSelf on Drop (see Pitfall 3 in RESEARCH).
    // Never return Err from this scope without calling RevertToSelf first.
    let mut token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 0, &mut token)
    };
    if ok == 0 {
        unsafe { RevertToSelf() };
        return Err(NonoError::SandboxInit(format!(
            "OpenThreadToken after impersonation failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    // Delegate SID extraction to the existing library function where possible,
    // or replicate the GetTokenInformation(TokenAppContainerSid) pattern from
    // `read_process_appcontainer_sid` in crates/nono/src/agent.rs (lines 178-340).
    let pkg_sid_result = extract_appcontainer_sid_from_token(token);
    // ALWAYS revert before returning (even on error path).
    unsafe { RevertToSelf() };
    unsafe { windows_sys::Win32::Foundation::CloseHandle(token) };
    pkg_sid_result
}
```

**`RevertToSelf` RAII guard** — analog from `WfpTransaction::drop` at lines 1043-1053 in `nono-wfp-service.rs`:
```rust
struct ImpersonationGuard;
impl Drop for ImpersonationGuard {
    fn drop(&mut self) {
        unsafe { RevertToSelf() };
    }
}
```

**Registry membership check** (after impersonation, uses `AgentRegistry` from `crates/nono/src/agent.rs`):
```rust
// After authenticate_pipe_client returns the kernel-vouched pkg_sid:
let tenant = {
    let state = daemon_state.tenants.lock()
        .map_err(|_| NonoError::SandboxInit("DaemonState mutex poisoned".into()))?;
    state.get(&client_sid).cloned()  // fail-secure: None → close instance
};
match tenant {
    None => {
        // SID not in registry → deny (Pitfall 1 in RESEARCH.md)
        // Pipe instance is dropped here, closing the connection.
        return Ok(());
    }
    Some(tenant_state) => serve_frames(pipe, &tenant_state).await?,
}
```

**tokio named-pipe server accept loop** — analog from `run_named_pipe_server` in `nono-wfp-service.rs` (lines 653-803 of analog):
```rust
async fn run_accept_loop(daemon_state: Arc<DaemonState>, shutdown: Arc<Notify>) {
    let shutdown_signal = shutdown.notified();
    tokio::pin!(shutdown_signal);
    loop {
        // Create a new pipe instance for the next connection.
        let pipe_handle = create_daemon_capability_pipe_instance()?;
        let mut server = unsafe {
            tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle(
                pipe_handle as *mut _,
            )
        }?;
        tokio::select! {
            biased;
            _ = &mut shutdown_signal => { break; }
            connect_result = server.connect() => { connect_result?; }
        }
        let state = Arc::clone(&daemon_state);
        tokio::spawn(async move {
            if let Err(e) = handle_one_connection(pipe_handle, state).await {
                tracing::warn!(error = %e, "Capability pipe connection error");
            }
        });
    }
}
```

---

### `crates/nono-cli/src/agent_daemon/reap.rs` — `AgentTenant` struct + `Drop` (utility, event-driven)

**Primary analog:** `ProcessContainment::Drop` in `crates/nono-cli/src/exec_strategy_windows/launch.rs` (lines 24-34) + `WfpEngine::Drop` in `nono-wfp-service.rs` (lines 938-949)

**RAII handle-owning struct pattern** (from `launch.rs` lines 24-34 + `nono-wfp-service.rs` lines 935-949):
```rust
#[cfg(target_os = "windows")]
pub(crate) struct AgentTenant {
    pub tenant_id: String,
    pub package_sid: String,
    pub profile_name: String,    // for DeleteAppContainerProfile in Drop
    pub caps: nono::CapabilitySet,
    // OwnedHandle closes handles on Drop via std::os::windows::io::OwnedHandle
    pub job_handle: std::os::windows::io::OwnedHandle,
    pub process_handle: std::os::windows::io::OwnedHandle,
}

#[cfg(target_os = "windows")]
impl Drop for AgentTenant {
    fn drop(&mut self) {
        // job_handle and process_handle: OwnedHandle closes them automatically.
        // KILL_ON_JOB_CLOSE means job_handle.drop() kills the agent process group.

        // Delete the AppContainer profile to avoid HKCU registry accumulation
        // (Pitfall 4 in RESEARCH.md: DeleteAppContainerProfile in Drop is mandatory).
        if let Err(e) = delete_app_container_profile(&self.profile_name) {
            tracing::warn!(
                tenant_id = %self.tenant_id,
                error = %e,
                "Failed to delete AppContainer profile on agent reap"
            );
        }
        // AgentRegistry removal is the caller's responsibility before drop.
    }
}
```

**`OwnedHandle` import** — already used in `socket_windows.rs` line 17 and re-exported via `exec_strategy_windows/mod.rs` line 613:
```rust
use std::os::windows::io::{FromRawHandle, OwnedHandle};
// OR the nono crate re-export:
pub(crate) use nono::OwnedHandle;
```

**`ProcessContainment::Drop` pattern** (lines 24-34 in `launch.rs`) — the analog the `AgentTenant::Drop` generalizes:
```rust
impl Drop for ProcessContainment {
    fn drop(&mut self) {
        if !self.job.is_null() {
            unsafe {
                // SAFETY: `self.job` was returned by CreateJobObjectW and is
                // owned by this struct. Closing the handle releases the job.
                CloseHandle(self.job);
            }
        }
    }
}
```

**`DaemonState` registry struct** (from RESEARCH.md §Deterministic Reap; analog is `AgentRegistry::new()` pattern in `crates/nono/src/agent.rs` lines 84-95, wrapped as in `classify_runtime.rs` lines 66-68):
```rust
pub(crate) struct DaemonState {
    /// Per-agent tenant map: tenant_id (session_id) → AgentTenant.
    /// AgentTenant::Drop reaps all handles when removed from this map.
    pub tenants: Arc<Mutex<HashMap<String, AgentTenant>>>,
    /// Phase 73 registry: the authoritative SID set for pipe auth.
    /// Promoted from per-run to persistent in Phase 74.
    pub agent_registry: Arc<Mutex<nono::AgentRegistry>>,
}
```

---

### `crates/nono-cli/src/agent_daemon/launch.rs` (service / orchestration, CRUD)

**Analog:** `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `create_process_containment` (lines 273-348) + `apply_process_handle_to_containment` (lines 375-389) + `build_job_security_attributes` (lines 78-131)

The daemon calls the Phase 71 broker-arm launch path (already in `exec_strategy_windows/launch.rs`) N times — one per agent. The orchestration layer:

**`create_process_containment` pattern** (lines 273-348 of analog):
```rust
pub(super) fn create_process_containment(
    session_id: Option<&str>,
    package_sid: Option<&str>,
) -> Result<ProcessContainment> {
    let name_u16 = session_id.map(|id| {
        let name = format!(r"Local\nono-session-{}", id);
        to_u16_null_terminated(&name)
    });
    // Build SDDL: OW=full, LW=deny, package_sid=deny
    let (_sd_guard, sa) = build_job_security_attributes(package_sid)?;
    let job = unsafe {
        CreateJobObjectW(
            &sa,
            name_u16.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null()),
        )
    };
    // ... set KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION ...
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
    // SetInformationJobObject ...
    Ok(ProcessContainment { job })
}
```

**Job security descriptor pattern** (lines 78-131 of analog) — copy the SDDL string with per-agent package-SID deny ACE:
```rust
fn build_job_security_attributes(package_sid: Option<&str>) -> Result<(OwnedJobSD, SECURITY_ATTRIBUTES)> {
    let sddl = match package_sid {
        None => "D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)".to_string(),
        Some(sid) => format!("D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)(D;;0x1F001F;;;{sid})"),
    };
    // ... ConvertStringSecurityDescriptorToSecurityDescriptorW ...
}
```

**`assign_failure_message` + `apply_process_handle_to_containment` pattern** (lines 350-389 of analog):
```rust
pub(super) fn apply_process_handle_to_containment(
    containment: &ProcessContainment,
    process: HANDLE,
) -> Result<()> {
    let ok = unsafe { AssignProcessToJobObject(containment.job, process) };
    if ok == 0 {
        let gle = unsafe { GetLastError() };
        // fail-secure: terminate the suspended process before returning Err
        return Err(NonoError::SandboxInit(assign_failure_message(gle)));
    }
    Ok(())
}
```

**`AgentRegistry::insert` call** — immediately after the spawn succeeds, before the process is resumed (analog: `classify_runtime.rs` lines 66-68 show `Arc<Mutex<AgentRegistry>>` usage):
```rust
// After broker spawn returns pkg_sid + process_handle + job_handle:
{
    let mut registry = daemon_state.agent_registry.lock()
        .map_err(|_| NonoError::SandboxInit("AgentRegistry poisoned".into()))?;
    registry.insert(pkg_sid.clone());
}
// Insert into tenants map atomically after registry.
{
    let mut tenants = daemon_state.tenants.lock()
        .map_err(|_| NonoError::SandboxInit("DaemonState tenants poisoned".into()))?;
    tenants.insert(tenant_id.clone(), AgentTenant { ... });
}
```

---

### `crates/nono-cli/src/agent_cli.rs` (controller / CLI verbs, request-response)

**Analog:** `crates/nono-cli/src/classify_runtime.rs` (lines 1-177) — a single-function runtime module dispatched from `app_runtime.rs` — PLUS `crates/nono-cli/src/cli.rs` `Classify` variant (lines 745-765)

**Runtime module pattern** (from `classify_runtime.rs` lines 55-92):
```rust
// File: crates/nono-cli/src/agent_cli.rs
use crate::cli::{DaemonArgs, DaemonCommands, AgentArgs, AgentCommands};
use nono::Result;

pub(crate) fn run_daemon(args: DaemonArgs) -> Result<()> {
    match args.command {
        DaemonCommands::Start => run_daemon_start(),
        DaemonCommands::Stop  => run_daemon_stop(),
        DaemonCommands::Status => run_daemon_status(),
    }
}

pub(crate) fn run_agent(args: AgentArgs) -> Result<()> {
    match args.command {
        AgentCommands::Launch(launch_args) => run_agent_launch(launch_args),
        AgentCommands::List => run_agent_list(),
    }
}
```

**clap subcommand pattern** (from `cli.rs` lines 745-765 — `Classify` variant and `ClassifyArgs`):
```rust
// Add to Commands enum in cli.rs:
/// Manage the nono agent daemon (persistent multi-tenant service)
#[command(subcommand_help_heading = "COMMANDS", disable_help_subcommand = true)]
#[command(help_template = "...")]
#[command(after_help = "\x1b[1mEXAMPLES\x1b[0m
  nono daemon start                    # Start the daemon (SCM or foreground)
  nono daemon stop                     # Stop a running daemon
  nono daemon status                   # Show daemon status
")]
Daemon(DaemonArgs),

/// Launch and inspect confined agents via the daemon
#[command(subcommand_help_heading = "COMMANDS", disable_help_subcommand = true)]
Agent(AgentArgs),
```

**clap args struct pattern** (from `cli.rs` — pattern used for `Rollback(RollbackArgs)` / `Audit(AuditArgs)` subcommands with nested `Subcommand` enums):
```rust
#[derive(clap::Args, Debug)]
pub(crate) struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum DaemonCommands {
    Start,
    Stop,
    Status,
}

#[derive(clap::Args, Debug)]
pub(crate) struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum AgentCommands {
    Launch(AgentLaunchArgs),
    List,
}

#[derive(clap::Args, Debug)]
pub(crate) struct AgentLaunchArgs {
    #[arg(long)]
    pub profile: String,
    #[arg(last = true)]
    pub cmd: Vec<String>,
}
```

**`app_runtime.rs` dispatch pattern** (from `app_runtime.rs` lines 60-68 — `Classify` dispatch):
```rust
// Add to dispatch_command match in app_runtime.rs:
Commands::Daemon(args) => run_command_with_update(update_handle, silent, || {
    agent_cli::run_daemon(args)
}),
Commands::Agent(args) => run_command_with_update(update_handle, silent, || {
    agent_cli::run_agent(args)
}),
```

**Error handling pattern** (from `classify_runtime.rs` lines 55-91 — `NonoError::SandboxInit` wrapping):
```rust
fn run_daemon_start() -> Result<()> {
    // Connect to daemon control pipe and send Start request.
    // Fail-secure: any pipe error → Err(NonoError::SandboxInit(...))
    let pipe_path = daemon_control_pipe_path();
    // ... connect + send ...
    Ok(())
}
```

---

### `crates/nono/src/supervisor/socket_windows.rs` (modify — add `authenticate_pipe_client`)

**This is the primary analog for itself.** The new function is a peer of the existing:
- `build_capability_pipe_sddl` (lines 1610-1663)
- `validate_package_sid_for_sddl` (lines 1399-1431)
- `current_logon_sid` (lines 1455-1543) — `GetTokenInformation(TokenGroups)` pattern reused for `TokenAppContainerSid`

**`GetTokenInformation` two-pass (probe then fill) pattern** (from `current_logon_sid` lines 1473-1507):
```rust
// Probe required buffer size:
let mut needed: u32 = 0;
let _ = unsafe {
    GetTokenInformation(token_handle, TokenGroups, std::ptr::null_mut(), 0, &mut needed)
};
if needed == 0 {
    return Err(NonoError::SandboxInit("GetTokenInformation probe returned 0".into()));
}
// Allocate and fill:
let mut buf = vec![0u8; needed as usize];
let ok = unsafe {
    GetTokenInformation(token_handle, TokenGroups, buf.as_mut_ptr() as *mut _, needed, &mut needed)
};
if ok == 0 {
    return Err(NonoError::SandboxInit(format!(
        "GetTokenInformation failed: {}", std::io::Error::last_os_error()
    )));
}
```

**`ConvertSidToStringSidW` pattern** (from `current_logon_sid` lines ~1526-1544; also `read_process_appcontainer_sid` in `agent.rs`):
```rust
let mut sid_str_ptr: *mut u16 = std::ptr::null_mut();
let ok = unsafe { ConvertSidToStringSidW(entry.Sid, &mut sid_str_ptr) };
if ok == 0 { return Err(...); }
let sid_str = unsafe { /* read null-terminated UTF-16 */ };
unsafe { LocalFree(sid_str_ptr as _) };
```

**New imports needed** (additions to the existing `use windows_sys::Win32::Security` block at lines 31-34 of `socket_windows.rs`):
```rust
use windows_sys::Win32::Security::{
    // EXISTING:
    GetTokenInformation, TokenGroups, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    SID_AND_ATTRIBUTES, TOKEN_GROUPS, TOKEN_QUERY,
    // ADD (for authenticate_pipe_client):
    ImpersonateNamedPipeClient,
    OpenThreadToken,
    TokenAppContainerSid,   // verify exact name in windows-sys 0.59; fallback: use numeric 56u32
    RevertToSelf,           // already in test scope at line 2349; add to production scope
};
use windows_sys::Win32::System::Threading::{
    // EXISTING:
    OpenProcessToken,
    GetCurrentProcess, GetCurrentProcessId, GetProcessId,
    // ADD:
    GetCurrentThread,
};
```

---

### `crates/nono/src/agent.rs` (modify — promote `AgentRegistry` to persistent, add `remove`)

**This is the primary analog for itself** (lines 78-152).

**`AgentRegistry` struct + methods** (lines 78-152 of `agent.rs` — copy and extend):
```rust
pub struct AgentRegistry {
    minted_sids: HashSet<String>,
}

impl AgentRegistry {
    pub fn insert(&mut self, package_sid_str: String) {
        self.minted_sids.insert(package_sid_str);
    }

    // ADD for Phase 74 (daemon removes on reap):
    pub fn remove(&mut self, package_sid_str: &str) {
        self.minted_sids.remove(package_sid_str);
    }

    // EXISTING:
    pub fn classify(&self, pid: u32) -> AgentClassification { ... }
}
```

**`Arc<Mutex<AgentRegistry>>` wrapping pattern** (from `classify_runtime.rs` lines 66-68 — already the pattern):
```rust
let registry = Arc::new(Mutex::new(nono::AgentRegistry::new()));
```

In the daemon `DaemonState`, this becomes a persistent field initialized once at daemon startup.

---

### `crates/nono-cli/tests/daemon_handle_baseline.rs` (test, integration / batch, host-gated)

**Analog:** `crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs` (lines 1-56) + `crates/nono/src/supervisor/socket_windows.rs` test block (lines 2346-2445)

**File-level platform gate pattern** (from `supervisor_ipc_robustness_windows.rs` lines 1-6, 23-24):
```rust
//! Daemon handle-count-baseline integration tests (Phase 74, Wave 0).
//!
//! Gate: NONO_DAEMON_INTEGRATION_TESTS=1 AND real Win11 host with dev-layout nono.exe.
//! On non-Windows: #![cfg] compiles to empty binary — always passes.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]
```

**Env-var gate pattern** (from `NONO_DAEMON_INTEGRATION_TESTS=1` convention; analog: `supervisor_ipc_robustness_windows.rs` uses direct `#[cfg(target_os)]` only — this file adds an env gate):
```rust
fn daemon_integration_tests_enabled() -> bool {
    std::env::var("NONO_DAEMON_INTEGRATION_TESTS").as_deref() == Ok("1")
}

macro_rules! require_integration {
    () => {
        if !daemon_integration_tests_enabled() {
            eprintln!("SKIP: set NONO_DAEMON_INTEGRATION_TESTS=1 to run");
            return;
        }
    };
}
```

**100-agent handle-baseline test structure** (from RESEARCH.md §100-agent test structure):
```rust
#[test]
fn n_agents_over_time_returns_to_baseline_handle_count() {
    require_integration!();

    let baseline = get_process_handle_count();

    for _ in 0..100 {
        let tenant = spawn_minimal_agent();   // mints AppContainer, job, process
        wait_for_agent_exit(&tenant);
        drop(tenant);  // triggers AgentTenant::Drop
    }

    let post = get_process_handle_count();
    assert!(
        post <= baseline + 5,
        "handle count did not return to baseline: before={baseline} after={post}"
    );
}

fn get_process_handle_count() -> u32 {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
    let mut count: u32 = 0;
    unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) };
    count
}
```

**In-process impersonation cross-tenant denial test** — analog from test block at lines 2346-2445 in `socket_windows.rs` (`ImpersonateLoggedOnUser` + `RevertToSelf` pattern):
```rust
#[test]
fn cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance() {
    require_integration!();
    use windows_sys::Win32::Security::{ImpersonateLoggedOnUser, RevertToSelf};

    // Establish tenant A pipe instance with tenant A's pkg_sid in the SDDL.
    let tenant_a_sid = "S-1-15-2-<a-sid-here>";
    let pipe_handle = create_tenant_capability_pipe_instance(tenant_a_sid)
        .expect("create tenant A pipe instance");

    // Impersonate tenant B's token (a DIFFERENT AppContainer token).
    let tenant_b_token = build_appcontainer_token_for_sid("S-1-15-2-<b-sid-here>");
    let ok = unsafe { ImpersonateLoggedOnUser(tenant_b_token) };
    assert_ne!(ok, 0);

    // Attempt to connect as tenant B to tenant A's pipe — must fail with access denied.
    let connect_result = connect_to_pipe(pipe_handle_name);
    let revert_ok = unsafe { RevertToSelf() };
    assert_ne!(revert_ok, 0, "RevertToSelf must succeed");

    assert!(
        connect_result.is_err(),
        "Tenant B must NOT be able to connect to tenant A's pipe instance"
    );
}
```

---

## Shared Patterns

### Error Handling
**Source:** `crates/nono/src/error.rs` — `NonoError` enum; used throughout all files above
**Apply to:** all service, middleware, utility, and test files

```rust
// All errors propagate via `?` and wrap in NonoError::SandboxInit for Win32 failures:
return Err(NonoError::SandboxInit(format!(
    "{context}: {}",
    std::io::Error::last_os_error()
)));
```

### Fail-Secure Default
**Source:** `crates/nono/src/agent.rs` lines 129-133 (`NotAnAgent` default) + `crates/nono-cli/src/classify_runtime.rs` lines 71-84
**Apply to:** `accept_loop.rs` auth path, `agent_cli.rs` dispatch, all `authenticate_pipe_client` error paths

```rust
// Any error → deny (close pipe instance, return NotAnAgent)
// Never silently fall back to a permissive state.
_ => AgentClassification::NotAnAgent,
```

### `OwnedHandle` RAII Pattern
**Source:** `crates/nono-cli/src/exec_strategy_windows/mod.rs` line 613 (`pub(crate) use nono::OwnedHandle`) + `crates/nono/src/supervisor/socket_windows.rs` line 17
**Apply to:** `AgentTenant` struct fields in `reap.rs`, any new Win32 handle allocations in `accept_loop.rs`

```rust
use std::os::windows::io::{FromRawHandle, OwnedHandle};
// OR the nono crate re-export:
pub(crate) use nono::OwnedHandle;
// Usage: let owned = unsafe { OwnedHandle::from_raw_handle(raw_handle) };
```

### Cross-Target cfg Gate (MANDATORY)
**Source:** `crates/nono-cli/src/bin/nono-wfp-service.rs` lines 18-22 (non-Windows stub)
**Apply to:** `nono-agentd.rs` (binary stub), any new `#[cfg(target_os = "windows")]` blocks in `socket_windows.rs` and `agent.rs`

```rust
// Top of every Windows-specific file or module:
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("nono-agentd is Windows-only");
    std::process::exit(1);
}
// All Windows-only items must be inside #[cfg(target_os = "windows")] blocks.
// MUST verify via cargo clippy --target x86_64-unknown-linux-gnu after any change.
```

### `Arc<Mutex<_>>` Thread-Safety Wrapping
**Source:** `crates/nono-cli/src/classify_runtime.rs` lines 66-68; doc comment in `crates/nono/src/agent.rs` lines 70-72
**Apply to:** `DaemonState` fields in `mod.rs`, all shared state passed across `tokio::spawn` boundaries

```rust
let registry = Arc::new(Mutex::new(nono::AgentRegistry::new()));
// In DaemonState:
pub agent_registry: Arc<Mutex<nono::AgentRegistry>>,
pub tenants: Arc<Mutex<HashMap<String, AgentTenant>>>,
```

### tracing Structured Logging
**Source:** Used throughout `nono-wfp-service.rs` (`tracing::warn!`, `eprintln!` for Event Log fallback)
**Apply to:** `accept_loop.rs`, `reap.rs`, `launch.rs` — all daemon operational paths

```rust
tracing::info!(tenant_id = %tenant_id, pkg_sid = %pkg_sid, "Agent launched");
tracing::warn!(tenant_id = %tenant_id, error = %e, "Failed to delete AppContainer profile");
```

### `validate_package_sid_for_sddl` Before Any SDDL Embedding
**Source:** `crates/nono/src/supervisor/socket_windows.rs` lines 1399-1431 + lines 1617-1623 (call site)
**Apply to:** `accept_loop.rs` anywhere a pkg_sid is embedded in a new pipe SDDL

```rust
validate_package_sid_for_sddl(pkg_sid)?;  // must precede build_capability_pipe_sddl call
let sddl = build_capability_pipe_sddl(None, Some(pkg_sid))?;
```

---

## No Analog Found

All Phase 74 files have close in-tree analogs. No files require falling back to RESEARCH.md patterns exclusively; every pattern has a direct in-tree code example to copy from.

The only net-new Win32 call surfaces — `ImpersonateNamedPipeClient`, `OpenThreadToken`, `GetCurrentThread`, `DeleteAppContainerProfile`, `GetProcessHandleCount` — have no production-code analog, but the test-scope `ImpersonateLoggedOnUser`/`RevertToSelf` block at lines 2349-2445 of `socket_windows.rs` provides the structural pattern, and RESEARCH.md §Unspiked Mechanism 2 documents the full call sequence.

| File / Function | Role | Data Flow | Reason No Production Analog |
|-----------------|------|-----------|------------------------------|
| `authenticate_pipe_client` (new fn in `socket_windows.rs`) | middleware | request-response | `ImpersonateNamedPipeClient` is not yet called in any production code path; only in test scope (line 2349 analog). Use the test-scope pattern + RESEARCH.md call sequence. |
| `proj/ADR-74-privilege-model.md` | config / decision record | N/A | No existing ADR files follow a strict format; use `proj/DESIGN-supervisor.md` for narrative structure, but content is original per SC4 ordering gate. |

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/bin/`, `crates/nono-cli/src/`, `crates/nono-cli/tests/`, `crates/nono/src/supervisor/`, `crates/nono/src/`
**Files scanned:** 12 source files read; 6 grep passes
**Pattern extraction date:** 2026-06-14
