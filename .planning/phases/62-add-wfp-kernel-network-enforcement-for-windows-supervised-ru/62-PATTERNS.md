# Phase 62: Add WFP Kernel Network Enforcement for Windows Supervised Runs - Pattern Map

**Mapped:** 2026-06-02
**Files analyzed:** 4 modified files + 1 existing test file
**Analogs found:** 4 / 4 (all modified files have direct in-file analogs — no new files are created)

---

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/exec_strategy_windows/network.rs` | service | request-response + CRUD | Existing `install_wfp_network_backend_with_runner` + `start_windows_wfp_service_with_runner` in same file | exact (in-file extension) |
| `crates/nono-cli/src/bin/nono-wfp-service.rs` | config | request-response | Existing `PIPE_SDDL` constant + `CreateNamedPipeW` block in same file (lines 55, 573-619) | exact (in-file constant change) |
| `dist/windows/nono-machine.wxs` | config | N/A | Existing `ServiceInstall`/`ServiceControl` block in same file (lines 81-97) | exact (in-file attribute change) |
| `crates/nono-cli/tests/wfp_port_integration.rs` | test | request-response | Existing mock-runner unit tests in `network.rs` lines 1691-2004 | role-match |

---

## Pattern Assignments

### `crates/nono-cli/src/exec_strategy_windows/network.rs` — D-03 auto-start hook

**Role:** service / request-response
**Analog:** Same file — `install_wfp_network_backend_with_runner` (lines 1560-1649) and `start_windows_wfp_service_with_runner` (lines 1315-1392)

**Injectable-runner function signature pattern** (lines 1560-1570):
```rust
pub(super) fn install_wfp_network_backend_with_runner<P, R>(
    policy: &nono::WindowsNetworkPolicy,
    config: &ExecConfig<'_>,
    probe_config: &WfpProbeConfig,
    probe_fn: P,
    run_probe: R,
) -> Result<Option<NetworkEnforcementGuard>>
where
    P: Fn(&WfpProbeConfig) -> Result<WfpProbeStatus>,
    R: Fn(&WfpProbeConfig, &WfpRuntimeActivationRequest) -> Result<WfpRuntimeProbeOutput>,
```

The D-03 hook adds a third injectable runner parameter `start_service_fn: S` following this exact pattern. The public `install_wfp_network_backend` wrapper (lines 1651-1663) then passes the real `start_windows_wfp_service_with_runner` for production, and tests pass mocks.

**D-03 insertion point** (lines 1588-1646):
The probe result is obtained at line 1588:
```rust
let status = probe_fn(probe_config).map_err(|err| {
    NonoError::SandboxInit(format!(
        "Failed to probe Windows WFP backend status ({}): {}",
        policy.backend_summary(),
        err
    ))
})?;
if status == WfpProbeStatus::Ready {
    // ... IPC activation ... (lines 1589-1642)
}
// D-03 hook: intercept BackendServiceStopped HERE before line 1644
Err(NonoError::UnsupportedPlatform(
    describe_wfp_runtime_activation_failure(policy, probe_config, status),
))
```

The new logic inserts a `match status { WfpProbeStatus::BackendServiceStopped => { ... }, _ => {} }` block immediately before line 1644. The `BackendServiceStopped` arm calls `start_service_fn`, then re-probes, then either falls through to the `Ready` IPC path (via a retry restructure) or returns the updated fail-closed error.

**start_windows_wfp_service_with_runner return contract** (lines 1315-1392):
```rust
pub(super) fn start_windows_wfp_service_with_runner<Q, R>(
    config: &WfpProbeConfig,
    query_service: Q,
    run_service_command: R,
) -> Result<WindowsWfpStartReport>
where
    Q: Fn(&str) -> Result<String>,
    R: Fn(&[String]) -> Result<String>,
{
    // Returns:
    //   Ok(WindowsWfpStartReport { status_label: "already running", .. })  -- already running
    //   Ok(WindowsWfpStartReport { status_label: "running", .. })           -- just started OK
    //   Err(NonoError::Setup(...))                                           -- failed to start
```

The `Ok` variants both mean the service is running. The D-03 hook calls this, then on `Ok` re-probes with `probe_fn` to get a fresh `WfpProbeStatus::Ready` before proceeding to IPC activation.

**Error handling pattern for fail-closed** (lines 1644-1646):
```rust
Err(NonoError::UnsupportedPlatform(
    describe_wfp_runtime_activation_failure(policy, probe_config, status),
))
```

The `describe_wfp_runtime_activation_failure` function (lines 417-468) has per-variant arms. The `BackendServiceStopped` arm (lines 444-447) currently reads:
```rust
WfpProbeStatus::BackendServiceStopped => format!(
    "the WFP service `{}` is registered but not running. Run `nono setup --start-wfp-service` first",
    config.backend_service
),
```

After D-03, this arm is only reached when the auto-start was also attempted and failed. The updated message must distinguish "auto-start failed" from "service not registered". Use an `auto_start_attempted: bool` parameter or a new `WfpProbeStatus::BackendServiceStoppedStartFailed` variant to thread this cleanly. The message wording must name the exact elevated remediation command: `nono setup --start-wfp-service`.

**build_wfp_service_create_args "demand" string** (lines 253-266):
```rust
pub(super) fn build_wfp_service_create_args(config: &WfpProbeConfig) -> Vec<String> {
    vec![
        "create".to_string(),
        config.backend_service.to_string(),
        "binPath=".to_string(),
        format_wfp_service_command(config),
        "start=".to_string(),
        "demand".to_string(),    // <-- change to "auto" per research open question recommendation
        ...
    ]
}
```

Research recommends also changing this to `"auto"` to prevent a manual `nono setup --register-wfp-service` from reverting the MSI's `start=auto`. The planner must decide.

**build_wfp_service_description_args stale string** (lines 268-274) — must be updated:
```rust
pub(super) fn build_wfp_service_description_args(config: &WfpProbeConfig) -> Vec<String> {
    vec![
        "description".to_string(),
        config.backend_service.to_string(),
        "Placeholder service host for the future nono Windows WFP backend. Registration is supported; runtime still fails closed until enforcement is implemented.".to_string(),
    ]
}
```

Replace the placeholder description with an accurate production description.

---

### `crates/nono-cli/src/bin/nono-wfp-service.rs` — PIPE_SDDL constant (Pitfall 5 fix)

**Role:** config / request-response
**Analog:** Same file — existing `PIPE_SDDL` constant (line 55) and its use in the `CreateNamedPipeW` loop (lines 573-619)

**Current PIPE_SDDL constant** (line 55):
```rust
const PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;OW)";
```

This grants: SYSTEM (SY) = full access, Administrators (BA) = full access, Owner (OW) = read+write. Standard (non-elevated) users cannot connect, blocking D-03's non-elevated supervisor.

**Required change** (one-line constant update):
```rust
const PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)(A;;GRGW;;;OW)";
```

Adding `(A;;GRGW;;;IU)` grants Interactive Users generic read+write. This covers `nono.exe` running at Medium IL (which holds the Interactive Users SID enabled) without opening the pipe to non-interactive service accounts or anonymous callers.

**SDDL-to-SD conversion pattern** (lines 573-619) — no change needed, only the constant changes:
```rust
let sd = {
    let mut sd = null_mut();
    let sddl_wide: Vec<u16> = PIPE_SDDL.encode_utf16().chain(Some(0)).collect();
    let status = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_wide.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            null_mut(),
        )
    };
    if status == 0 {
        return Err(format!(
            "failed to convert SDDL to security descriptor: {}",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        ));
    }
    sd
};
let sa = SECURITY_ATTRIBUTES {
    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
    lpSecurityDescriptor: sd,
    bInheritHandle: 0,
};
// ... CreateNamedPipeW(&sa, ...) ...
```

The SDDL is converted once per accept-loop iteration (to allow dynamic updates if the constant changes). Changing `PIPE_SDDL` at line 55 is the only required edit; the machinery that converts it to a security descriptor and passes it to `CreateNamedPipeW` is unchanged.

**SDDL test pattern** — add a `#[cfg(target_os = "windows")]` compile-time assertion or unit test in the `#[cfg(test)]` block at the bottom of the file:
```rust
#[test]
fn pipe_sddl_includes_interactive_users() {
    assert!(
        PIPE_SDDL.contains("IU") || PIPE_SDDL.contains("BU"),
        "PIPE_SDDL must grant access to Interactive Users or Built-in Users \
         so non-elevated supervised nono runs can reach the WFP service pipe"
    );
}
```

---

### `dist/windows/nono-machine.wxs` — ServiceInstall start=auto (D-01)

**Role:** config / N/A
**Analog:** Same file — existing `ServiceInstall`/`ServiceControl` block (lines 81-97)

**Current block** (lines 81-97):
```xml
<Component Id="cmpWfpServiceExe" Guid="*">
  <File Id="filWfpServiceExe"
        Source="...\nono-wfp-service.exe" KeyPath="yes" />
  <ServiceInstall
      Id="svcWfpService"
      Name="nono-wfp-service"
      DisplayName="nono WFP Service"
      Description="nono Windows Filtering Platform backend service"
      Type="ownProcess"
      Start="demand"            <!-- CHANGE: demand -> auto -->
      Account="LocalSystem"
      ErrorControl="normal"
      Arguments="--service-mode" />
  <ServiceControl
      Id="svcCtrlWfpService"
      Name="nono-wfp-service"
      Start="install"           <!-- keep: starts service on first install -->
      Stop="both"               <!-- keep: stops on uninstall and upgrade -->
      Remove="uninstall"        <!-- keep: removes on uninstall -->
      Wait="yes" />
</Component>
```

**Required change:** `Start="demand"` → `Start="auto"` at line 87. This is the primary D-01 fix. The SCM will auto-start the service at every boot after machine MSI installation.

**Do NOT change `ServiceControl Start="install"`** — that causes WiX to start the service immediately during install (independent of boot-start). It should remain so the service is available right after MSI installation without a reboot.

**Optional ServiceConfig for restart policy** (planner decision):
```xml
<ServiceConfig
    ServiceName="nono-wfp-service"
    OnInstall="yes"
    FirstFailureActionType="restart"
    SecondFailureActionType="restart"
    ThirdFailureActionType="none"
    ResetPeriodInSeconds="60"
    RestartServiceDelayInSeconds="5" />
```

Note: WiX v4 `ServiceConfig` syntax is marked ASSUMED in RESEARCH.md — verify against WiX v4 docs at `wixtoolset.org/docs/reference/schema/wxs/serviceconfig/` before using.

**Clean-uninstall invariant** — the existing `ServiceControl Stop="both" Remove="uninstall"` (lines 91-97) already handles stopping and removing the service during MSI uninstall. The `CaUninstallWfpServices` custom action (lines 25-34 of the file) provides defense-in-depth. Changing `Start="demand"` to `Start="auto"` does not affect the uninstall path.

---

### `crates/nono-cli/tests/wfp_port_integration.rs` — new D-03 unit tests

**Role:** test / request-response
**Analog:** `crates/nono-cli/src/exec_strategy_windows/network.rs` lines 1836-2004 (mock-runner unit tests for `install_wfp_network_backend_with_runner`)

**Mock-runner test structure** (lines 1836-1891 of network.rs) — copy this pattern for D-03 tests:
```rust
#[test]
fn install_wfp_network_backend_returns_guard_on_enforced_pending_cleanup() {
    let policy = make_blocked_policy();
    let caps = nono::CapabilitySet::new();
    let command = vec!["agent.exe".to_string()];
    let resolved_program = std::path::PathBuf::from(r"C:\tools\agent.exe");
    let current_dir = std::path::PathBuf::from(r"C:\workspace");
    let config = ExecConfig {
        command: &command,
        resolved_program: &resolved_program,
        caps: &caps,
        env_vars: vec![],
        cap_file: None,
        current_dir: &current_dir,
        session_sid: Some("S-1-5-117-123456789-1234-5678-9012".to_string()),
        interactive_shell: false,
        session_token: None,
        cap_pipe_rendezvous_path: None,
        allowed_env_vars: None,
        denied_env_vars: None,
        prefers_low_il_broker: false,
    };
    let probe_config = make_test_probe_config();

    let mock_probe =
        |_config: &WfpProbeConfig| -> Result<WfpProbeStatus> { Ok(WfpProbeStatus::Ready) };
    let mock_runner = |_config: &WfpProbeConfig,
                       _request: &WfpRuntimeActivationRequest|
     -> Result<WfpRuntimeProbeOutput> { ... };

    let result = install_wfp_network_backend_with_runner(
        &policy, &config, &probe_config, mock_probe, mock_runner,
        // NEW: add mock_start_service parameter
    );
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
}
```

**Helper fixtures** (lines 1696-1728 of network.rs) — reuse in new tests:
```rust
fn make_blocked_policy() -> nono::WindowsNetworkPolicy { ... }
fn make_test_probe_config() -> WfpProbeConfig { ... }
fn sc_missing_output() -> String { ... }
fn sc_running_output() -> String { ... }
fn sc_stopped_output() -> String { ... }
```

**Three new tests required** (all mock-based, no elevation needed):

1. `test_wfp_autostart_on_stopped` — probe returns `BackendServiceStopped`, `start_service_fn` returns `Ok(...)`, re-probe returns `Ready`, IPC mock returns `enforced-pending-cleanup` → result is `Ok(Some(WfpServiceManaged { .. }))`.

2. `test_wfp_autostart_fail_remediation_message` — probe returns `BackendServiceStopped`, `start_service_fn` returns `Err(NonoError::Setup(...))` → result is `Err(NonoError::UnsupportedPlatform(msg))` where `msg` contains both `"nono-wfp-service"` and `"nono setup --start-wfp-service"`.

3. `test_wfp_non_stopped_status_unchanged` — probe returns `BackendServiceMissing` (not `BackendServiceStopped`) → `start_service_fn` is never called, result is `Err(NonoError::UnsupportedPlatform(...))` with the `BackendServiceMissing` message (not the `BackendServiceStopped` auto-start message).

**Test placement:** New D-03 unit tests belong in the existing `#[cfg(test)]` block in `network.rs` (lines 1691-2004), not in `wfp_port_integration.rs`. The `wfp_port_integration.rs` file is for admin-gated, real-service integration tests. The SDDL constant test belongs in `nono-wfp-service.rs` (or alongside the `PIPE_SDDL` constant). The research's "Wave 0 Gaps" table lists these as the test deliverables.

---

## Shared Patterns

### Windows-only gating
**Source:** `crates/nono-cli/src/exec_strategy_windows/network.rs` (entire file is within `exec_strategy_windows/`)
**Apply to:** All changes in this phase

All new and modified code is Windows-only. No `#[cfg(target_os = "windows")]` attribute is needed on individual items inside `exec_strategy_windows/` because the module itself is cfg-gated. In `nono-wfp-service.rs` the pattern is explicit:
```rust
#[cfg(target_os = "windows")]
mod windows_impl { ... }
```

New test assertions that reference Windows-only constants (e.g., `PIPE_SDDL`) should be inside `#[cfg(target_os = "windows")]` blocks.

### Injectable-runner pattern for testability
**Source:** `crates/nono-cli/src/exec_strategy_windows/network.rs` lines 1560-1663
**Apply to:** D-03 hook function signature extension

The project uses a consistent pattern of `_with_runner` suffixed functions that accept `Fn` trait-bounded closure parameters in place of real system calls. The public wrapper without `_with_runner` passes the real production functions. All new start-attempt logic must follow this pattern — `start_service_fn: S where S: Fn(&WfpProbeConfig, &[String]) -> Result<String>` (matching `run_sc_command`'s signature) or a typed wrapper matching `start_windows_wfp_service_with_runner`'s own signature.

### Fail-secure error returns
**Source:** `crates/nono-cli/src/exec_strategy_windows/network.rs` lines 1644-1646
**Apply to:** Every code path in D-03 that does not result in confirmed `WfpProbeStatus::Ready`

Pattern: any non-Ready outcome returns `Err(NonoError::UnsupportedPlatform(...))` with a formatted message. Never return `Ok(None)` when `network.block:true` and the service is unavailable. The existing `describe_wfp_runtime_activation_failure` is the single source of truth for all per-status failure messages — extend it rather than creating inline strings.

### Elevation detection — attempt-then-handle, not pre-check
**Source:** `crates/nono-cli/src/exec_strategy_windows/mod.rs` lines 524-547 (`is_admin_process()`)
**Apply to:** D-03 start attempt

`is_admin_process()` is `pub(crate)` and available in `network.rs`. However, RESEARCH.md explicitly says NOT to call it before the start attempt (TOCTOU risk, duplicate elevation check). The correct pattern for D-03 is: call `start_service_fn`, and if it returns `Err`, include the remediation command in the error message. `is_admin_process()` may be called in tests to gate expected behavior, but not in production control flow.

### `#[must_use]` on Result-returning helpers
**Source:** CLAUDE.md Coding Standards
**Apply to:** Any new `pub(super)` or `pub(crate)` helper extracted for D-03

If the planner extracts a new helper (e.g., `ensure_wfp_service_running_with_runner`), it must carry `#[must_use]`.

### Test module pattern
**Source:** `crates/nono-cli/src/exec_strategy_windows/network.rs` lines 1691-1695
**Apply to:** New unit tests for D-03

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    ...
}
```

The `#[allow(clippy::unwrap_used)]` annotation is the project-standard exception for test modules. Do not add `#[allow(dead_code)]` — all new test helpers must be actively called.

---

## No Analog Found

No files in this phase lack an analog. All changes are modifications to existing files with strong in-file analogs.

---

## WiX `nono-user.wxs` — confirmed out of scope

`dist/windows/nono-user.wxs` has no `ServiceInstall` element (confirmed from RESEARCH.md live-source verification). The user-scope MSI cannot register SCM services. Do NOT add service components to the user MSI.

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/exec_strategy_windows/`, `crates/nono-cli/src/bin/`, `crates/nono-cli/src/setup.rs`, `crates/nono-cli/tests/`, `dist/windows/`
**Files scanned:** 6 (network.rs, nono-wfp-service.rs, mod.rs, setup.rs, wfp_port_integration.rs, nono-machine.wxs)
**Pattern extraction date:** 2026-06-02
