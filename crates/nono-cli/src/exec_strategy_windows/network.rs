use super::*;

impl Drop for NetworkEnforcementGuard {
    fn drop(&mut self) {
        match self {
            NetworkEnforcementGuard::FirewallRules {
                staged_dir,
                inbound_rule,
                outbound_rule,
                ..
            } => {
                let _ = delete_firewall_rule(inbound_rule);
                let _ = delete_firewall_rule(outbound_rule);
                cleanup_network_enforcement_staging(staged_dir);
            }
            NetworkEnforcementGuard::WfpServiceManaged {
                policy,
                probe_config,
                target_program,
                inbound_rule,
                outbound_rule,
            } => {
                let _ = cleanup_wfp_service_managed_enforcement_with_runner(
                    policy,
                    probe_config,
                    target_program,
                    inbound_rule,
                    outbound_rule,
                    run_wfp_runtime_probe_with_request,
                );
            }
        }
    }
}

pub(super) fn run_netsh_firewall(args: &[&str]) -> Result<String> {
    let output = Command::new(super::system32_exe("netsh"))
        .args(args)
        .output()
        .map_err(NonoError::CommandExecution)?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(classify_netsh_firewall_failure(
            args,
            &format!("{stdout}{stderr}"),
        ))
    }
}

pub(super) fn classify_netsh_firewall_failure(args: &[&str], output: &str) -> NonoError {
    let detail = if output.contains("requires elevation") || output.contains("Access is denied") {
        "Windows blocked-network enforcement currently uses temporary Windows Firewall rules and requires an elevated administrator session on this machine. The long-term Windows backend target is WFP.".to_string()
    } else if output.trim().is_empty() {
        "Windows Firewall did not return diagnostic output. The current blocked-network backend uses temporary Windows Firewall rules; the long-term backend target is WFP.".to_string()
    } else {
        format!(
            "{} (current backend: Windows Firewall rules; preferred backend: WFP)",
            output.trim()
        )
    };
    NonoError::SandboxInit(format!(
        "Failed to apply Windows blocked-network rule (args: {}): {}",
        args.join(" "),
        detail
    ))
}

pub(super) fn delete_firewall_rule(name: &str) -> Result<()> {
    let rule_name = format!("name={name}");
    let _ = run_netsh_firewall(&["advfirewall", "firewall", "delete", "rule", &rule_name]);
    Ok(())
}

pub(super) fn unique_windows_firewall_rule_suffix() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn stage_program_for_blocked_network_launch(
    program: &Path,
) -> Result<(PathBuf, PathBuf)> {
    let file_name = program.file_name().ok_or_else(|| {
        NonoError::SandboxInit(format!(
            "Failed to stage Windows blocked-network executable copy for {}",
            program.display()
        ))
    })?;
    let staged_dir = std::env::temp_dir()
        .join("nono-net-block")
        .join(unique_windows_firewall_rule_suffix());
    std::fs::create_dir_all(&staged_dir).map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to prepare Windows blocked-network staging directory {}: {}",
            staged_dir.display(),
            e
        ))
    })?;
    let staged_program = staged_dir.join(file_name);
    std::fs::copy(program, &staged_program).map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to stage Windows blocked-network executable copy {} -> {}: {}",
            program.display(),
            staged_program.display(),
            e
        ))
    })?;
    // Also stage sibling files the program is likely to load at startup
    // (side-by-side DLLs, runtime manifests, .NET configs). Without these,
    // programs that ship with bundled dependencies would fail to start from
    // the temp staging directory. Extension-gated so we don't blindly copy
    // unrelated data when the program lives in e.g. `C:\Program Files\...`.
    if let Some(parent) = program.parent() {
        copy_program_siblings(parent, file_name, &staged_dir)?;
    }
    Ok((staged_program, staged_dir))
}

/// Copy sibling files from `source_dir` into `staged_dir`, filtering to
/// extensions commonly needed for Windows program startup. Skips the program
/// binary itself (already copied) and anything that isn't a regular file.
/// Fail-secure: any copy error aborts the whole stage.
fn copy_program_siblings(
    source_dir: &Path,
    program_file_name: &std::ffi::OsStr,
    staged_dir: &Path,
) -> Result<()> {
    const ALLOWED_EXTENSIONS: &[&str] = &["dll", "pdb", "manifest", "config", "xml"];
    let entries = std::fs::read_dir(source_dir).map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to enumerate program directory {} for sibling copy: {}",
            source_dir.display(),
            e
        ))
    })?;
    for entry in entries.flatten() {
        let entry_name = entry.file_name();
        if entry_name == program_file_name {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let allowed = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .is_some_and(|ext| ALLOWED_EXTENSIONS.iter().any(|allowed| *allowed == ext));
        if !allowed {
            continue;
        }
        let dest = staged_dir.join(&entry_name);
        std::fs::copy(&path, &dest).map_err(|e| {
            NonoError::SandboxInit(format!(
                "Failed to stage program sibling {} -> {}: {}",
                path.display(),
                dest.display(),
                e
            ))
        })?;
    }
    Ok(())
}

pub(super) fn cleanup_network_enforcement_staging(staged_dir: &Path) {
    let _ = std::fs::remove_dir_all(staged_dir);
}

pub(super) fn cleanup_stale_network_enforcement_artifacts() {
    let staging_root = std::env::temp_dir().join("nono-net-block");
    if let Ok(entries) = std::fs::read_dir(&staging_root) {
        for entry in entries.flatten() {
            if let Some(suffix) = entry.file_name().to_str().map(|s| s.to_string()) {
                let inbound_rule = format!("nono-win-block-in-{suffix}");
                let outbound_rule = format!("nono-win-block-out-{suffix}");
                let _ = delete_firewall_rule(&inbound_rule);
                let _ = delete_firewall_rule(&outbound_rule);
            }
            cleanup_network_enforcement_staging(&entry.path());
        }
    }
}

pub(super) fn current_wfp_probe_config() -> Result<WfpProbeConfig> {
    let current_exe = std::env::current_exe().map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to resolve current executable for Windows WFP backend probing: {e}"
        ))
    })?;
    let exe_dir = current_exe.parent().ok_or_else(|| {
        NonoError::SandboxInit(format!(
            "Failed to resolve executable directory for Windows WFP backend probing: {}",
            current_exe.display()
        ))
    })?;

    Ok(WfpProbeConfig {
        platform_service: WINDOWS_WFP_PLATFORM_SERVICE,
        backend_service: WINDOWS_WFP_BACKEND_SERVICE,
        backend_driver: WINDOWS_WFP_BACKEND_DRIVER,
        backend_binary_path: exe_dir.join(WINDOWS_WFP_BACKEND_BINARY),
        backend_driver_binary_path: exe_dir.join(WINDOWS_WFP_BACKEND_DRIVER_BINARY),
        backend_service_args: WINDOWS_WFP_BACKEND_SERVICE_ARGS,
    })
}

pub(super) fn format_wfp_service_command(config: &WfpProbeConfig) -> String {
    format!(
        "\"{}\" {}",
        config.backend_binary_path.display(),
        config.backend_service_args.join(" ")
    )
}

pub(super) fn run_sc_query(service: &str) -> Result<String> {
    let output = Command::new(super::system32_exe("sc"))
        .args(["query", service])
        .output()
        .map_err(NonoError::CommandExecution)?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub(super) fn run_sc_command(args: &[String]) -> Result<String> {
    let output = Command::new(super::system32_exe("sc"))
        .args(args)
        .output()
        .map_err(NonoError::CommandExecution)?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub(super) fn sc_create_conflict_is_registered(output: &str) -> bool {
    let normalized = output.to_ascii_uppercase();
    normalized.contains("FAILED 1073")
        || normalized.contains("ALREADY EXISTS")
        || normalized.contains("MARKED FOR DELETION")
}

pub(super) fn build_wfp_service_create_args(config: &WfpProbeConfig) -> Vec<String> {
    vec![
        "create".to_string(),
        config.backend_service.to_string(),
        "binPath=".to_string(),
        format_wfp_service_command(config),
        "start=".to_string(),
        "auto".to_string(),
        "type=".to_string(),
        "own".to_string(),
        "DisplayName=".to_string(),
        "nono WFP Service".to_string(),
    ]
}

pub(super) fn build_wfp_service_description_args(config: &WfpProbeConfig) -> Vec<String> {
    vec![
        "description".to_string(),
        config.backend_service.to_string(),
        "nono Windows Filtering Platform service. Enforces kernel-level network blocking for supervised nono runs via WFP ALE_AUTH filters. Start this service before issuing `nono run` with network.block:true.".to_string(),
    ]
}

pub(super) fn build_wfp_driver_create_args(config: &WfpProbeConfig) -> Vec<String> {
    vec![
        "create".to_string(),
        config.backend_driver.to_string(),
        "binPath=".to_string(),
        // Embed literal quotes around the image path so the SCM stores a
        // quoted ImagePath entry. Without them, any whitespace in the path
        // (e.g. `C:\Program Files\...`) would be interpreted as argument
        // separators at service-start time and the service would fail to
        // locate its image. See `format_wfp_service_command` above for the
        // matching treatment on the service-binary side.
        format!("\"{}\"", config.backend_driver_binary_path.display()),
        "type=".to_string(),
        "kernel".to_string(),
        "start=".to_string(),
        "demand".to_string(),
        "DisplayName=".to_string(),
        "nono WFP Driver".to_string(),
    ]
}

pub(super) fn build_wfp_driver_description_args(config: &WfpProbeConfig) -> Vec<String> {
    vec![
        "description".to_string(),
        config.backend_driver.to_string(),
        "Placeholder kernel-driver registration for the future nono Windows WFP backend. Registration is supported; enforcement is not implemented yet.".to_string(),
    ]
}

pub(super) fn build_wfp_service_start_args(config: &WfpProbeConfig) -> Vec<String> {
    vec!["start".to_string(), config.backend_service.to_string()]
}

pub(super) fn build_wfp_driver_start_args(config: &WfpProbeConfig) -> Vec<String> {
    vec!["start".to_string(), config.backend_driver.to_string()]
}

pub(super) fn build_wfp_service_stop_args(config: &WfpProbeConfig) -> Vec<String> {
    vec!["stop".to_string(), config.backend_service.to_string()]
}

pub(super) fn build_wfp_service_delete_args(config: &WfpProbeConfig) -> Vec<String> {
    vec!["delete".to_string(), config.backend_service.to_string()]
}

pub(super) fn build_wfp_driver_stop_args(config: &WfpProbeConfig) -> Vec<String> {
    vec!["stop".to_string(), config.backend_driver.to_string()]
}

pub(super) fn build_wfp_driver_delete_args(config: &WfpProbeConfig) -> Vec<String> {
    vec!["delete".to_string(), config.backend_driver.to_string()]
}

pub(super) fn parse_windows_service_state(output: &str) -> WindowsServiceState {
    let normalized = output.to_ascii_uppercase();
    if normalized.contains("FAILED 1060") || normalized.contains("DOES NOT EXIST") {
        WindowsServiceState::Missing
    } else if normalized.contains("STATE") && normalized.contains("RUNNING") {
        WindowsServiceState::Running
    } else if normalized.contains("STATE")
        && (normalized.contains("STOPPED") || normalized.contains("STOP_PENDING"))
    {
        WindowsServiceState::Stopped
    } else {
        WindowsServiceState::Unknown
    }
}

/// Determine the WFP readiness status from the current system state.
///
/// Returns [`WfpProbeStatus::Ready`] when the Base Filtering Engine platform service
/// (`BFE`) AND the nono-wfp-service backend service are both Running.  Per D-05
/// (service-only model) the kernel driver `nono-wfp-driver` is **out of scope** and
/// is NOT required for `Ready`; its absence does not gate enforcement.
///
/// All other fail-secure prerequisites remain unchanged: a missing/stopped BFE or a
/// missing/stopped backend service short-circuit before `Ready`.
pub(super) fn build_wfp_probe_status(
    backend_binary_exists: bool,
    platform_service: WindowsServiceState,
    backend_service: WindowsServiceState,
) -> WfpProbeStatus {
    if !backend_binary_exists {
        return WfpProbeStatus::BackendBinaryMissing;
    }

    match platform_service {
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return WfpProbeStatus::PlatformServiceMissing;
        }
        WindowsServiceState::Stopped => return WfpProbeStatus::PlatformServiceStopped,
        WindowsServiceState::Running => {}
    }

    match backend_service {
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return WfpProbeStatus::BackendServiceMissing;
        }
        WindowsServiceState::Stopped => return WfpProbeStatus::BackendServiceStopped,
        WindowsServiceState::Running => {}
    }

    WfpProbeStatus::Ready
}

pub(super) fn probe_wfp_backend_status_with_config(
    config: &WfpProbeConfig,
) -> Result<WfpProbeStatus> {
    if windows_wfp_test_force_ready() {
        return Ok(build_wfp_probe_status(
            config.backend_binary_path.exists(),
            WindowsServiceState::Running,
            WindowsServiceState::Running,
        ));
    }

    if !config.backend_binary_path.exists() {
        return Ok(WfpProbeStatus::BackendBinaryMissing);
    }

    let platform_output = run_sc_query(config.platform_service)?;
    let platform_state = parse_windows_service_state(&platform_output);
    let backend_service_state = parse_windows_service_state(&run_sc_query(config.backend_service)?);

    Ok(build_wfp_probe_status(
        true,
        platform_state,
        backend_service_state,
    ))
}

pub(super) fn describe_wfp_runtime_activation_failure(
    policy: &nono::WindowsNetworkPolicy,
    config: &WfpProbeConfig,
    status: WfpProbeStatus,
) -> String {
    let runtime_target = describe_windows_network_runtime_target(policy);
    let reason = match status {
        WfpProbeStatus::Ready => format!(
            "the service `{}` and driver `{}` are present, but the runtime activation exchange did not install an enforceable network-policy state",
            config.backend_service, config.backend_driver
        ),
        WfpProbeStatus::BackendBinaryMissing => format!(
            "the WFP service binary `{}` is missing from this build output. Run `cargo build -p nono-cli --bins` first",
            config.backend_binary_path.display()
        ),
        WfpProbeStatus::PlatformServiceMissing => format!(
            "the Windows Base Filtering Engine service `{}` is missing on this machine",
            config.platform_service
        ),
        WfpProbeStatus::PlatformServiceStopped => format!(
            "the Windows Base Filtering Engine service `{}` is not running. Start it before retrying WFP activation",
            config.platform_service
        ),
        WfpProbeStatus::BackendServiceMissing => format!(
            "the WFP service `{}` is not registered. Run `nono setup --install-wfp-service` first",
            config.backend_service
        ),
        WfpProbeStatus::BackendServiceStopped => format!(
            "the WFP service `{}` is registered but not running and could not be started automatically. To start it, run in an elevated (Administrator) terminal: `nono setup --start-wfp-service`",
            config.backend_service
        ),
        WfpProbeStatus::BackendDriverBinaryMissing => format!(
            "the WFP driver binary `{}` is missing from this build output. Run `cargo build -p nono-cli --bins` first",
            config.backend_driver_binary_path.display()
        ),
        WfpProbeStatus::BackendDriverMissing => format!(
            "the WFP driver `{}` is not registered. Run `nono setup --install-wfp-driver` first",
            config.backend_driver
        ),
        WfpProbeStatus::BackendDriverStopped => format!(
            "the WFP driver `{}` is registered but not running. Run `nono setup --start-wfp-driver` first",
            config.backend_driver
        ),
    };

    format!(
        "Windows WFP runtime activation is required for {} but {} ({}). This request remains fail-closed until WFP activation is implemented.",
        runtime_target,
        reason,
        policy.backend_summary()
    )
}

pub(super) fn describe_windows_network_runtime_target(
    policy: &nono::WindowsNetworkPolicy,
) -> String {
    let base = match &policy.mode {
        nono::WindowsNetworkPolicyMode::AllowAll => "allow-all Windows network access".to_string(),
        nono::WindowsNetworkPolicyMode::Blocked => "blocked Windows network access".to_string(),
        nono::WindowsNetworkPolicyMode::ProxyOnly { port, bind_ports } => format!(
            "Windows proxy-only network access via localhost:{} with bind ports {:?}",
            port, bind_ports
        ),
    };

    let mut restrictions = Vec::new();
    if !policy.tcp_connect_ports.is_empty() {
        restrictions.push(format!("connect ports {:?}", policy.tcp_connect_ports));
    }
    if !policy.tcp_bind_ports.is_empty() {
        restrictions.push(format!("bind ports {:?}", policy.tcp_bind_ports));
    }
    if !policy.localhost_ports.is_empty() {
        restrictions.push(format!("localhost ports {:?}", policy.localhost_ports));
    }

    if restrictions.is_empty() {
        base
    } else {
        format!("{} with {}", base, restrictions.join(", "))
    }
}

pub(super) fn build_wfp_runtime_activation_request(
    policy: &nono::WindowsNetworkPolicy,
) -> WfpRuntimeActivationRequest {
    let network_mode = match &policy.mode {
        nono::WindowsNetworkPolicyMode::AllowAll => "allow-all",
        nono::WindowsNetworkPolicyMode::Blocked => "blocked",
        nono::WindowsNetworkPolicyMode::ProxyOnly { .. } => "proxy-only",
    };
    let mut tcp_bind_ports = policy.tcp_bind_ports.clone();
    let mut localhost_ports = policy.localhost_ports.clone();
    if let nono::WindowsNetworkPolicyMode::ProxyOnly { port, bind_ports } = &policy.mode {
        tcp_bind_ports.extend(bind_ports.iter().copied());
        tcp_bind_ports.sort_unstable();
        tcp_bind_ports.dedup();
        localhost_ports.push(*port);
        localhost_ports.sort_unstable();
        localhost_ports.dedup();
    }

    WfpRuntimeActivationRequest {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        request_kind: match &policy.mode {
            nono::WindowsNetworkPolicyMode::Blocked => "activate_blocked_mode",
            nono::WindowsNetworkPolicyMode::AllowAll => "activate_allow_all_mode",
            nono::WindowsNetworkPolicyMode::ProxyOnly { .. } => "activate_proxy_mode",
        }
        .to_string(),
        network_mode: network_mode.to_string(),
        preferred_backend: policy.preferred_backend.label().to_string(),
        active_backend: policy.active_backend.label().to_string(),
        runtime_target: describe_windows_network_runtime_target(policy),
        tcp_connect_ports: policy.tcp_connect_ports.clone(),
        tcp_bind_ports,
        localhost_ports,
        target_program_path: None,
        outbound_rule_name: None,
        inbound_rule_name: None,
        session_sid: None,
    }
}

pub(super) fn build_wfp_target_activation_request(
    policy: &nono::WindowsNetworkPolicy,
    target_program: &Path,
    outbound_rule: &str,
    inbound_rule: &str,
    session_sid: Option<&str>,
) -> WfpRuntimeActivationRequest {
    let mut request = build_wfp_runtime_activation_request(policy);
    request.target_program_path = Some(target_program.display().to_string());
    request.outbound_rule_name = Some(outbound_rule.to_string());
    request.inbound_rule_name = Some(inbound_rule.to_string());
    request.session_sid = session_sid.map(str::to_string);
    request
}

pub(super) fn build_wfp_runtime_cleanup_request(
    policy: &nono::WindowsNetworkPolicy,
    target_program: &Path,
    inbound_rule: &str,
    outbound_rule: &str,
) -> WfpRuntimeActivationRequest {
    let mut request = build_wfp_runtime_activation_request(policy);
    request.request_kind = "deactivate_policy_mode".to_string();
    request.target_program_path = Some(target_program.display().to_string());
    request.outbound_rule_name = Some(outbound_rule.to_string());
    request.inbound_rule_name = Some(inbound_rule.to_string());
    request.runtime_target = format!(
        "{} for {}",
        describe_windows_network_runtime_target(policy),
        target_program.display()
    );
    request
}

pub(super) fn cleanup_wfp_service_managed_enforcement_with_runner<R>(
    policy: &nono::WindowsNetworkPolicy,
    probe_config: &WfpProbeConfig,
    target_program: &Path,
    inbound_rule: &str,
    outbound_rule: &str,
    run_probe: R,
) -> Result<()>
where
    R: Fn(&WfpProbeConfig, &WfpRuntimeActivationRequest) -> Result<WfpRuntimeProbeOutput>,
{
    let request =
        build_wfp_runtime_cleanup_request(policy, target_program, inbound_rule, outbound_rule);
    let output = run_probe(probe_config, &request)?;
    match parse_wfp_runtime_probe_status(&output)? {
        WfpRuntimeActivationProbeStatus::CleanupSucceeded => Ok(()),
        WfpRuntimeActivationProbeStatus::Ready
        | WfpRuntimeActivationProbeStatus::AcceptedButNotEnforced
        | WfpRuntimeActivationProbeStatus::EnforcedPendingCleanup
        | WfpRuntimeActivationProbeStatus::FilteringProbeSucceeded
        | WfpRuntimeActivationProbeStatus::NotImplemented => Err(NonoError::SandboxInit(format!(
            "Windows WFP cleanup returned an unexpected network-policy state: {:?}",
            output.response
        ))),
    }
}

pub(super) fn describe_wfp_probe_status_for_setup(
    config: &WfpProbeConfig,
    status: WfpProbeStatus,
) -> String {
    let service_command = format_wfp_service_command(config);
    match status {
        WfpProbeStatus::Ready => format!(
            "WFP backend components are present (service binary: {}, driver binary: {}, service: {}, driver: {}), and live network-policy activation now depends on the service-host runtime transport. Expected service command: {}.",
            config.backend_binary_path.display(),
            config.backend_driver_binary_path.display(),
            config.backend_service,
            config.backend_driver,
            service_command
        ),
        WfpProbeStatus::BackendBinaryMissing => format!(
            "Expected WFP backend service binary is missing: {}. Expected service: {}. Expected driver: {}. Expected driver binary: {}. Expected registration/start command: {}.",
            config.backend_binary_path.display(),
            config.backend_service,
            config.backend_driver,
            config.backend_driver_binary_path.display(),
            service_command
        ),
        WfpProbeStatus::PlatformServiceMissing => format!(
            "Base Filtering Engine service ({}) is missing or could not be queried.",
            config.platform_service
        ),
        WfpProbeStatus::PlatformServiceStopped => format!(
            "Base Filtering Engine service ({}) is not running.",
            config.platform_service
        ),
        WfpProbeStatus::BackendServiceMissing => format!(
            "WFP backend service is missing: {}. Register it to launch {} with: {}.",
            config.backend_service,
            config.backend_service,
            service_command
        ),
        WfpProbeStatus::BackendServiceStopped => format!(
            "WFP backend service is installed but not running: {}. Its expected startup command remains: {}.",
            config.backend_service,
            service_command
        ),
        WfpProbeStatus::BackendDriverBinaryMissing => format!(
            "WFP backend driver binary is missing: {}. Expected driver registration name: {}.",
            config.backend_driver_binary_path.display(),
            config.backend_driver
        ),
        WfpProbeStatus::BackendDriverMissing => format!(
            "WFP backend driver is not registered: {}. Expected driver binary: {}.",
            config.backend_driver,
            config.backend_driver_binary_path.display()
        ),
        WfpProbeStatus::BackendDriverStopped => format!(
            "WFP backend driver is installed but not running: {}. Expected driver binary: {}.",
            config.backend_driver,
            config.backend_driver_binary_path.display()
        ),
    }
}

pub(super) fn describe_wfp_service_status_for_setup(
    config: &WfpProbeConfig,
    status: WfpProbeStatus,
) -> (&'static str, String) {
    let service_command = format_wfp_service_command(config);
    match status {
        WfpProbeStatus::Ready => (
            "ready",
            format!(
                "WFP backend service {} is present and running. Expected startup command: {}.",
                config.backend_service, service_command
            ),
        ),
        WfpProbeStatus::BackendBinaryMissing => (
            "missing binary",
            format!(
                "WFP backend service binary is missing: {}. Expected service registration name: {}. Expected startup command: {}.",
                config.backend_binary_path.display(),
                config.backend_service,
                service_command
            ),
        ),
        WfpProbeStatus::PlatformServiceMissing => (
            "blocked by bfe",
            format!(
                "WFP backend service readiness is blocked because the Base Filtering Engine service ({}) is missing or could not be queried.",
                config.platform_service
            ),
        ),
        WfpProbeStatus::PlatformServiceStopped => (
            "blocked by bfe",
            format!(
                "WFP backend service readiness is blocked because the Base Filtering Engine service ({}) is not running.",
                config.platform_service
            ),
        ),
        WfpProbeStatus::BackendServiceMissing => (
            "not registered",
            format!(
                "WFP backend service is not registered: {}. Register it to launch {} with: {}.",
                config.backend_service, config.backend_service, service_command
            ),
        ),
        WfpProbeStatus::BackendServiceStopped => (
            "stopped",
            format!(
                "WFP backend service is registered but not running: {}. Its expected startup command remains: {}.",
                config.backend_service, service_command
            ),
        ),
        WfpProbeStatus::BackendDriverBinaryMissing
        | WfpProbeStatus::BackendDriverMissing
        | WfpProbeStatus::BackendDriverStopped => (
            "ready",
            format!(
                "WFP backend service {} is present and running. Expected startup command: {}.",
                config.backend_service, service_command
            ),
        ),
    }
}

pub(super) fn describe_wfp_driver_status_for_setup(
    config: &WfpProbeConfig,
    status: WfpProbeStatus,
) -> (&'static str, String) {
    match status {
        WfpProbeStatus::Ready => (
            "ready",
            format!(
                "WFP backend driver {} is present and running from binary {}.",
                config.backend_driver,
                config.backend_driver_binary_path.display()
            ),
        ),
        WfpProbeStatus::BackendBinaryMissing => (
            "blocked by service",
            format!(
                "WFP backend driver readiness is blocked until the service binary {} is available.",
                config.backend_binary_path.display()
            ),
        ),
        WfpProbeStatus::PlatformServiceMissing => (
            "blocked by bfe",
            format!(
                "WFP backend driver readiness is blocked because the Base Filtering Engine service ({}) is missing or could not be queried.",
                config.platform_service
            ),
        ),
        WfpProbeStatus::PlatformServiceStopped => (
            "blocked by bfe",
            format!(
                "WFP backend driver readiness is blocked because the Base Filtering Engine service ({}) is not running.",
                config.platform_service
            ),
        ),
        WfpProbeStatus::BackendServiceMissing => (
            "blocked by service",
            format!(
                "WFP backend driver readiness is blocked until the service {} is registered.",
                config.backend_service
            ),
        ),
        WfpProbeStatus::BackendServiceStopped => (
            "blocked by service",
            format!(
                "WFP backend driver readiness is blocked until the service {} is running.",
                config.backend_service
            ),
        ),
        WfpProbeStatus::BackendDriverBinaryMissing => (
            "missing binary",
            format!(
                "WFP backend driver binary is missing: {}. Expected driver registration name: {}.",
                config.backend_driver_binary_path.display(),
                config.backend_driver
            ),
        ),
        WfpProbeStatus::BackendDriverMissing => (
            "not registered",
            format!(
                "WFP backend driver is not registered: {}. Expected driver binary: {}.",
                config.backend_driver,
                config.backend_driver_binary_path.display()
            ),
        ),
        WfpProbeStatus::BackendDriverStopped => (
            "stopped",
            format!(
                "WFP backend driver is registered but not running: {}. Expected driver binary: {}.",
                config.backend_driver,
                config.backend_driver_binary_path.display()
            ),
        ),
    }
}

pub(super) fn describe_wfp_next_action_for_setup(
    config: &WfpProbeConfig,
    status: WfpProbeStatus,
) -> Option<String> {
    match status {
        WfpProbeStatus::Ready => Some(
            "Next action: Windows WFP components are present and the service can activate kernel WFP filters; runtime activation will proceed."
                .to_string(),
        ),
        WfpProbeStatus::BackendBinaryMissing => Some(format!(
            "Next action: build the Windows backend artifacts first with `cargo build -p nono-cli --bins` so `{}` exists.",
            config.backend_binary_path.display()
        )),
        WfpProbeStatus::PlatformServiceMissing => Some(format!(
            "Next action: verify that the Windows Base Filtering Engine service `{}` is available on this machine.",
            config.platform_service
        )),
        WfpProbeStatus::PlatformServiceStopped => Some(format!(
            "Next action: start the Windows Base Filtering Engine service `{}` before retrying WFP setup or activation.",
            config.platform_service
        )),
        WfpProbeStatus::BackendServiceMissing => Some(
            "Next action: run `nono setup --install-wfp-service`.".to_string(),
        ),
        WfpProbeStatus::BackendServiceStopped => Some(
            "Next action: run `nono setup --start-wfp-service`.".to_string(),
        ),
        WfpProbeStatus::BackendDriverBinaryMissing => Some(format!(
            "Next action: build the Windows backend artifacts first with `cargo build -p nono-cli --bins` so `{}` exists.",
            config.backend_driver_binary_path.display()
        )),
        WfpProbeStatus::BackendDriverMissing => Some(
            "Next action: run `nono setup --install-wfp-driver`.".to_string(),
        ),
        WfpProbeStatus::BackendDriverStopped => Some(
            "Next action: run `nono setup --start-wfp-driver`.".to_string(),
        ),
    }
}

pub(super) fn run_wfp_runtime_request(
    request: &WfpRuntimeActivationRequest,
) -> Result<WfpRuntimeActivationResponse> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| {
            NonoError::Setup(format!("Failed to build tokio runtime for WFP IPC: {}", e))
        })?;

    rt.block_on(async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut client = tokio::net::windows::named_pipe::ClientOptions::new()
            .open(r"\\.\pipe\nono-wfp-control")
            .map_err(|e| {
                NonoError::Setup(format!(
                    "Failed to connect to nono-wfp-service: {}. Is it running?",
                    e
                ))
            })?;

        let request_json = serde_json::to_vec(request)
            .map_err(|e| NonoError::Setup(format!("Failed to serialize WFP request: {}", e)))?;

        client
            .write_all(&request_json)
            .await
            .map_err(|e| NonoError::Setup(format!("Failed to write to nono-wfp-service: {}", e)))?;

        let mut buffer = vec![0u8; 64 * 1024];
        let n = client.read(&mut buffer).await.map_err(|e| {
            NonoError::Setup(format!("Failed to read from nono-wfp-service: {}", e))
        })?;

        if n == 0 {
            return Err(NonoError::Setup(
                "WFP service closed connection unexpectedly".to_string(),
            ));
        }

        let response: WfpRuntimeActivationResponse = serde_json::from_slice(&buffer[..n])
            .map_err(|e| NonoError::Setup(format!("Failed to parse WFP response: {}", e)))?;

        Ok(response)
    })
}

pub(super) fn run_wfp_runtime_probe_with_request(
    _config: &WfpProbeConfig,
    request: &WfpRuntimeActivationRequest,
) -> Result<WfpRuntimeProbeOutput> {
    let response = run_wfp_runtime_request(request)?;
    Ok(WfpRuntimeProbeOutput {
        status_code: Some(0),
        response,
        stderr: String::new(),
    })
}

pub(super) fn parse_wfp_runtime_probe_status(
    output: &WfpRuntimeProbeOutput,
) -> Result<WfpRuntimeActivationProbeStatus> {
    if output.response.status == "ready" {
        return Ok(WfpRuntimeActivationProbeStatus::Ready);
    }
    if output.response.status == "accepted-but-not-enforced" {
        return Ok(WfpRuntimeActivationProbeStatus::AcceptedButNotEnforced);
    }
    if output.response.status == "enforced-pending-cleanup" {
        return Ok(WfpRuntimeActivationProbeStatus::EnforcedPendingCleanup);
    }
    if output.response.status == "cleanup-succeeded" {
        return Ok(WfpRuntimeActivationProbeStatus::CleanupSucceeded);
    }
    if output.response.status == "filtering-probe-succeeded" {
        return Ok(WfpRuntimeActivationProbeStatus::FilteringProbeSucceeded);
    }
    if output.response.status == "not-implemented" {
        return Ok(WfpRuntimeActivationProbeStatus::NotImplemented);
    }
    if output.response.status == "invalid-request" {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows WFP service rejected the runtime activation request: {}",
            output.response.details
        )));
    }
    if output.response.status == "protocol-mismatch" {
        return Err(NonoError::SandboxInit(format!(
            "Windows WFP activation protocol mismatch: {}",
            output.response.details
        )));
    }
    if output.response.status == "prerequisites-missing" {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows WFP activation prerequisites are missing: {}",
            output.response.details
        )));
    }
    if output.response.status == "filtering-probe-failed" {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows WFP service could not install its network-policy filtering probe: {}",
            output.response.details
        )));
    }
    if output.response.status == "cleanup-failed" {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows WFP service could not clean up target-attached network-policy enforcement: {}",
            output.response.details
        )));
    }

    Err(NonoError::SandboxInit(format!(
        "Windows WFP runtime probe returned unexpected response (status: {:?}, response: {:?}, stderr: {:?})",
        output.status_code, output.response, output.stderr
    )))
}

pub(super) fn describe_wfp_runtime_probe_failure(
    config: &WfpProbeConfig,
    output: &WfpRuntimeProbeOutput,
) -> String {
    format!(
        "the WFP service probe `{}` {} reported an unexpected runtime activation state (status: {:?}, response: {:?}, stderr: {:?})",
        config.backend_binary_path.display(),
        WINDOWS_WFP_RUNTIME_PROBE_ARG,
        output.status_code,
        output.response,
        output.stderr
    )
}

pub(super) fn install_windows_wfp_service_with_runner<Q, R>(
    config: &WfpProbeConfig,
    query_service: Q,
    run_service_command: R,
) -> Result<WindowsWfpInstallReport>
where
    Q: Fn(&str) -> Result<String>,
    R: Fn(&[String]) -> Result<String>,
{
    if !config.backend_binary_path.exists() {
        return Err(NonoError::Setup(format!(
            "Cannot register Windows WFP service because the backend binary is missing: {}. Build nono-wfp-service first.",
            config.backend_binary_path.display()
        )));
    }

    let platform_state = parse_windows_service_state(&query_service(config.platform_service)?);
    match platform_state {
        WindowsServiceState::Running => {}
        WindowsServiceState::Stopped => {
            return Err(NonoError::Setup(format!(
                "Cannot register Windows WFP service because the Base Filtering Engine service ({}) is not running.",
                config.platform_service
            )));
        }
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return Err(NonoError::Setup(format!(
                "Cannot register Windows WFP service because the Base Filtering Engine service ({}) is missing or could not be queried.",
                config.platform_service
            )));
        }
    }

    let service_command = format_wfp_service_command(config);
    let service_state = parse_windows_service_state(&query_service(config.backend_service)?);
    if service_state != WindowsServiceState::Missing {
        return Ok(WindowsWfpInstallReport {
            status_label: "already installed",
            details: format!(
                "Windows WFP service {} is already registered. Expected startup command: {}. The service host is used for blocked-mode activation, but unsupported states still fail closed until full backend parity is implemented.",
                config.backend_service, service_command
            ),
        });
    }

    if let Err(err) = run_service_command(&build_wfp_service_create_args(config)) {
        if let Ok(state) = query_service(config.backend_service) {
            let registered_state = parse_windows_service_state(&state);
            if registered_state != WindowsServiceState::Missing
                && sc_create_conflict_is_registered(&err.to_string())
            {
                return Ok(WindowsWfpInstallReport {
                    status_label: "already installed",
                    details: format!(
                        "Windows WFP service {} is already registered. Expected startup command: {}. The service host is used for blocked-mode activation, but unsupported states still fail closed until full backend parity is implemented.",
                        config.backend_service, service_command
                    ),
                });
            }
        }
        return Err(err);
    }
    run_service_command(&build_wfp_service_description_args(config))?;

    let registered_state = parse_windows_service_state(&query_service(config.backend_service)?);
    if registered_state == WindowsServiceState::Missing {
        return Err(NonoError::Setup(format!(
            "Windows WFP service registration did not persist for {}. Expected startup command: {}.",
            config.backend_service, service_command
        )));
    }

    Ok(WindowsWfpInstallReport {
        status_label: "installed",
        details: format!(
            "Registered Windows WFP service {} with startup command: {}. Service startup is not attempted automatically because explicit lifecycle control is still required before live WFP activation.",
            config.backend_service, service_command
        ),
    })
}

pub(crate) fn install_windows_wfp_service() -> Result<WindowsWfpInstallReport> {
    let config = current_wfp_probe_config()?;
    install_windows_wfp_service_with_runner(&config, run_sc_query, run_sc_command)
}

pub(super) fn install_windows_wfp_driver_with_runner<Q, R>(
    config: &WfpProbeConfig,
    query_service: Q,
    run_service_command: R,
) -> Result<WindowsWfpDriverInstallReport>
where
    Q: Fn(&str) -> Result<String>,
    R: Fn(&[String]) -> Result<String>,
{
    if !config.backend_driver_binary_path.exists() {
        return Err(NonoError::Setup(format!(
            "Cannot register Windows WFP driver because the driver binary is missing: {}. Build nono-cli so the placeholder driver artifact is staged first.",
            config.backend_driver_binary_path.display()
        )));
    }

    let platform_state = parse_windows_service_state(&query_service(config.platform_service)?);
    match platform_state {
        WindowsServiceState::Running => {}
        WindowsServiceState::Stopped => {
            return Err(NonoError::Setup(format!(
                "Cannot register Windows WFP driver because the Base Filtering Engine service ({}) is not running.",
                config.platform_service
            )));
        }
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return Err(NonoError::Setup(format!(
                "Cannot register Windows WFP driver because the Base Filtering Engine service ({}) is missing or could not be queried.",
                config.platform_service
            )));
        }
    }

    let driver_state = parse_windows_service_state(&query_service(config.backend_driver)?);
    if driver_state != WindowsServiceState::Missing {
        return Ok(WindowsWfpDriverInstallReport {
            status_label: "already installed",
            details: format!(
                "Windows WFP driver {} is already registered. Expected driver binary path: {}. Driver startup is not attempted automatically.",
                config.backend_driver,
                config.backend_driver_binary_path.display()
            ),
        });
    }

    if let Err(err) = run_service_command(&build_wfp_driver_create_args(config)) {
        if let Ok(state) = query_service(config.backend_driver) {
            let registered_state = parse_windows_service_state(&state);
            if registered_state != WindowsServiceState::Missing
                && sc_create_conflict_is_registered(&err.to_string())
            {
                return Ok(WindowsWfpDriverInstallReport {
                    status_label: "already installed",
                    details: format!(
                        "Windows WFP driver {} is already registered. Expected driver binary path: {}. Driver startup is not attempted automatically.",
                        config.backend_driver,
                        config.backend_driver_binary_path.display()
                    ),
                });
            }
        }
        return Err(err);
    }
    run_service_command(&build_wfp_driver_description_args(config))?;

    let registered_state = parse_windows_service_state(&query_service(config.backend_driver)?);
    if registered_state == WindowsServiceState::Missing {
        return Err(NonoError::Setup(format!(
            "Windows WFP driver registration did not persist for {}. Expected driver binary path: {}.",
            config.backend_driver,
            config.backend_driver_binary_path.display()
        )));
    }

    Ok(WindowsWfpDriverInstallReport {
        status_label: "installed",
        details: format!(
            "Registered Windows WFP driver {} with binary path {}. Driver startup is not attempted automatically because this branch still does not ship a working WFP driver.",
            config.backend_driver,
            config.backend_driver_binary_path.display()
        ),
    })
}

pub(crate) fn install_windows_wfp_driver() -> Result<WindowsWfpDriverInstallReport> {
    let config = current_wfp_probe_config()?;
    install_windows_wfp_driver_with_runner(&config, run_sc_query, run_sc_command)
}

pub(super) fn start_windows_wfp_driver_with_runner<Q, R>(
    config: &WfpProbeConfig,
    query_service: Q,
    run_service_command: R,
) -> Result<WindowsWfpDriverStartReport>
where
    Q: Fn(&str) -> Result<String>,
    R: Fn(&[String]) -> Result<String>,
{
    if !config.backend_driver_binary_path.exists() {
        return Err(NonoError::Setup(format!(
            "Cannot start Windows WFP driver because the driver binary is missing: {}. Build nono-cli so the placeholder driver artifact is staged first.",
            config.backend_driver_binary_path.display()
        )));
    }

    let platform_state = parse_windows_service_state(&query_service(config.platform_service)?);
    match platform_state {
        WindowsServiceState::Running => {}
        WindowsServiceState::Stopped => {
            return Err(NonoError::Setup(format!(
                "Cannot start Windows WFP driver because the Base Filtering Engine service ({}) is not running.",
                config.platform_service
            )));
        }
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return Err(NonoError::Setup(format!(
                "Cannot start Windows WFP driver because the Base Filtering Engine service ({}) is missing or could not be queried.",
                config.platform_service
            )));
        }
    }

    let driver_state = parse_windows_service_state(&query_service(config.backend_driver)?);
    match driver_state {
        WindowsServiceState::Running => {
            return Ok(WindowsWfpDriverStartReport {
                status_label: "already running",
                details: format!(
                    "Windows WFP driver {} is already running from binary {}. Network enforcement is still not active until the real WFP backend is implemented.",
                    config.backend_driver,
                    config.backend_driver_binary_path.display()
                ),
            });
        }
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return Err(NonoError::Setup(format!(
                "Cannot start Windows WFP driver because it is not registered: {}. Run `nono setup --install-wfp-driver` first.",
                config.backend_driver
            )));
        }
        WindowsServiceState::Stopped => {}
    }

    let start_output = run_service_command(&build_wfp_driver_start_args(config))?;
    let updated_state = parse_windows_service_state(&query_service(config.backend_driver)?);
    if updated_state == WindowsServiceState::Running {
        return Ok(WindowsWfpDriverStartReport {
            status_label: "running",
            details: format!(
                "Windows WFP driver {} is running from binary {}. The placeholder driver still does not provide network enforcement yet.",
                config.backend_driver,
                config.backend_driver_binary_path.display()
            ),
        });
    }

    Err(NonoError::Setup(format!(
        "Windows WFP driver {} did not reach RUNNING after an explicit start attempt. Driver binary: {}. Current host output: {}. This is expected while the placeholder driver still fails closed.",
        config.backend_driver,
        config.backend_driver_binary_path.display(),
        start_output.trim()
    )))
}

pub(crate) fn start_windows_wfp_driver() -> Result<WindowsWfpDriverStartReport> {
    let config = current_wfp_probe_config()?;
    start_windows_wfp_driver_with_runner(&config, run_sc_query, run_sc_command)
}

/// Stop (best-effort) then delete a single Windows service by name.
///
/// Idempotent: a `Missing` service is reported as "not installed" and skipped.
/// A running service is stopped first so the delete completes immediately and
/// the service-host binary is unlocked; a stop failure does NOT abort the delete
/// because `sc delete` can still mark a running service for deletion.
fn remove_single_windows_service<Q, R>(
    service_name: &str,
    stop_args: &[String],
    delete_args: &[String],
    query_service: &Q,
    run_service_command: &R,
) -> Result<&'static str>
where
    Q: Fn(&str) -> Result<String>,
    R: Fn(&[String]) -> Result<String>,
{
    let state = parse_windows_service_state(&query_service(service_name)?);
    if state == WindowsServiceState::Missing {
        return Ok("not installed");
    }
    if state == WindowsServiceState::Running {
        // Best-effort: ignore the stop result and proceed to delete regardless.
        let _ = run_service_command(stop_args);
    }
    run_service_command(delete_args)?;
    let after = parse_windows_service_state(&query_service(service_name)?);
    match after {
        WindowsServiceState::Missing => Ok("removed"),
        // `sc delete` on a service that still has open handles marks it for
        // deletion; it disappears once those handles close (often immediately,
        // otherwise on reboot).
        _ => Ok("removed (pending handle close)"),
    }
}

/// Stop and delete the nono user-mode WFP service AND the kernel driver service.
///
/// This is the removal counterpart to `install_windows_wfp_service` /
/// `install_windows_wfp_driver`. The machine MSI already removes the user-mode
/// service on uninstall via WiX `ServiceControl`, but the kernel driver service
/// (`nono-wfp-driver`, registered post-install via `sc create type=kernel`) has
/// no WiX representation, so a clean MSI uninstall leaves it behind. This command
/// removes both.
///
/// Before stopping/deleting the services, the persistent WFP objects introduced
/// by 62-09 are purged FAIL-OPEN: `run_purge` receives the path to the backend
/// binary and must invoke it with `--purge-wfp-objects`. A purge failure is
/// recorded in `report.details` but NEVER aborts or returns an error — uninstall
/// must always complete regardless of purge outcome. The MSI custom action
/// (`CaUninstallWfpServices`) has `Return=ignore`, so even a non-zero exit from
/// the whole setup chain is non-blocking. This is the deliberate inverse of the
/// run-time fail-CLOSED stance: at uninstall the service is going away and
/// best-effort cleanup is correct (T-62-24).
///
/// Safety: only the two well-known nono-owned service names from `WfpProbeConfig`
/// are ever touched, so this can never delete an unrelated service. Idempotent —
/// services that are not installed are reported and skipped.
pub(super) fn uninstall_windows_wfp_with_runner<Q, R, P>(
    config: &WfpProbeConfig,
    query_service: Q,
    run_service_command: R,
    run_purge: P,
) -> Result<WindowsWfpUninstallReport>
where
    Q: Fn(&str) -> Result<String>,
    R: Fn(&[String]) -> Result<String>,
    P: Fn(&std::path::Path) -> Result<String>,
{
    // Run the WFP object purge FIRST (before stopping/deleting the service),
    // while the binary is still present (MSI custom action runs Before=RemoveFiles).
    // FAIL-OPEN: a purge error is recorded but must NOT abort uninstall.
    let purge_note = match run_purge(&config.backend_binary_path) {
        Ok(_) => "wfp objects purged",
        Err(e) => {
            tracing::warn!(
                "uninstall: wfp object purge failed (best-effort, uninstall continues): {}",
                e
            );
            "wfp object purge skipped (best-effort)"
        }
    };

    let service_outcome = remove_single_windows_service(
        config.backend_service,
        &build_wfp_service_stop_args(config),
        &build_wfp_service_delete_args(config),
        &query_service,
        &run_service_command,
    )?;
    let driver_outcome = remove_single_windows_service(
        config.backend_driver,
        &build_wfp_driver_stop_args(config),
        &build_wfp_driver_delete_args(config),
        &query_service,
        &run_service_command,
    )?;

    let status_label = if service_outcome == "not installed" && driver_outcome == "not installed" {
        "nothing to remove"
    } else {
        "removed"
    };
    Ok(WindowsWfpUninstallReport {
        status_label,
        details: format!(
            "purge: {}; user-mode service {}: {}; kernel driver {}: {}",
            purge_note,
            config.backend_service,
            service_outcome,
            config.backend_driver,
            driver_outcome
        ),
    })
}

/// Run the backend binary with `--purge-wfp-objects` to delete all nono WFP
/// filters and the nono sublayer before the service is stopped+deleted.
///
/// Non-zero exit from the binary is NOT treated as an error — the uninstall
/// path is fail-open (T-62-24). Spawn failure is propagated so the caller
/// can record it, but the caller must not abort uninstall on Err either.
fn run_backend_purge(binary: &std::path::Path) -> Result<String> {
    if !binary.exists() {
        return Ok("backend binary absent; nothing to purge".into());
    }
    let out = std::process::Command::new(binary)
        .arg("--purge-wfp-objects")
        .output()
        .map_err(NonoError::CommandExecution)?;
    // Non-zero exit is NOT an error here (fail-open): surface stdout/stderr
    // for the uninstall report so the outcome is auditable.
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

pub(crate) fn uninstall_windows_wfp() -> Result<WindowsWfpUninstallReport> {
    let config = current_wfp_probe_config()?;
    uninstall_windows_wfp_with_runner(&config, run_sc_query, run_sc_command, run_backend_purge)
}

pub(super) fn start_windows_wfp_service_with_runner<Q, R>(
    config: &WfpProbeConfig,
    query_service: Q,
    run_service_command: R,
) -> Result<WindowsWfpStartReport>
where
    Q: Fn(&str) -> Result<String>,
    R: Fn(&[String]) -> Result<String>,
{
    if !config.backend_binary_path.exists() {
        return Err(NonoError::Setup(format!(
            "Cannot start Windows WFP service because the backend binary is missing: {}. Build nono-wfp-service first.",
            config.backend_binary_path.display()
        )));
    }

    let platform_state = parse_windows_service_state(&query_service(config.platform_service)?);
    match platform_state {
        WindowsServiceState::Running => {}
        WindowsServiceState::Stopped => {
            return Err(NonoError::Setup(format!(
                "Cannot start Windows WFP service because the Base Filtering Engine service ({}) is not running.",
                config.platform_service
            )));
        }
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return Err(NonoError::Setup(format!(
                "Cannot start Windows WFP service because the Base Filtering Engine service ({}) is missing or could not be queried.",
                config.platform_service
            )));
        }
    }

    let service_command = format_wfp_service_command(config);
    let service_state = parse_windows_service_state(&query_service(config.backend_service)?);
    match service_state {
        WindowsServiceState::Running => {
            return Ok(WindowsWfpStartReport {
                status_label: "already running",
                details: format!(
                    "Windows WFP service {} is already running. Its registered startup command is {}. Network enforcement is still not active until the real WFP backend is implemented.",
                    config.backend_service, service_command
                ),
            });
        }
        WindowsServiceState::Missing | WindowsServiceState::Unknown => {
            return Err(NonoError::Setup(format!(
                "Cannot start Windows WFP service because it is not registered: {}. Run `nono setup --install-wfp-service` first.",
                config.backend_service
            )));
        }
        WindowsServiceState::Stopped => {}
    }

    let start_output = run_service_command(&build_wfp_service_start_args(config))?;
    let updated_state = parse_windows_service_state(&query_service(config.backend_service)?);
    if updated_state == WindowsServiceState::Running {
        return Ok(WindowsWfpStartReport {
            status_label: "running",
            details: format!(
                "Windows WFP service {} is running with startup command {}. The service is ready to activate WFP filters on request.",
                config.backend_service, service_command
            ),
        });
    }

    Err(NonoError::Setup(format!(
        "Windows WFP service {} did not reach RUNNING after an explicit start attempt. Startup command: {}. Current host output: {}.",
        config.backend_service,
        service_command,
        start_output.trim()
    )))
}

pub(crate) fn start_windows_wfp_service() -> Result<WindowsWfpStartReport> {
    let config = current_wfp_probe_config()?;
    start_windows_wfp_service_with_runner(&config, run_sc_query, run_sc_command)
}

pub(crate) fn probe_windows_wfp_readiness() -> WindowsWfpReadinessReport {
    let Ok(config) = current_wfp_probe_config() else {
        return WindowsWfpReadinessReport {
            status_label: "probe failed",
            details: "Failed to resolve expected WFP backend component paths from the current executable layout.".to_string(),
            next_action: None,
            service_status_label: "probe failed",
            service_details: "Failed to resolve expected WFP backend service component paths from the current executable layout.".to_string(),
            driver_status_label: "probe failed",
            driver_details: "Failed to resolve expected WFP backend driver component paths from the current executable layout.".to_string(),
        };
    };

    match probe_wfp_backend_status_with_config(&config) {
        Ok(status) => {
            let (service_status_label, service_details) =
                describe_wfp_service_status_for_setup(&config, status);
            let (driver_status_label, driver_details) =
                describe_wfp_driver_status_for_setup(&config, status);
            WindowsWfpReadinessReport {
                status_label: match status {
                    WfpProbeStatus::Ready => "ready",
                    WfpProbeStatus::BackendBinaryMissing => "missing binary",
                    WfpProbeStatus::PlatformServiceMissing => "missing bfe",
                    WfpProbeStatus::PlatformServiceStopped => "bfe stopped",
                    WfpProbeStatus::BackendServiceMissing => "missing service",
                    WfpProbeStatus::BackendServiceStopped => "service stopped",
                    WfpProbeStatus::BackendDriverBinaryMissing => "missing driver binary",
                    WfpProbeStatus::BackendDriverMissing => "driver not registered",
                    WfpProbeStatus::BackendDriverStopped => "driver stopped",
                },
                details: describe_wfp_probe_status_for_setup(&config, status),
                next_action: describe_wfp_next_action_for_setup(&config, status),
                service_status_label,
                service_details,
                driver_status_label,
                driver_details,
            }
        }
        Err(err) => WindowsWfpReadinessReport {
            status_label: "probe failed",
            details: format!("Failed to probe Windows WFP readiness: {err}"),
            next_action: None,
            service_status_label: "probe failed",
            service_details: format!("Failed to probe Windows WFP service readiness: {err}"),
            driver_status_label: "probe failed",
            driver_details: format!("Failed to probe Windows WFP driver readiness: {err}"),
        },
    }
}

pub(super) fn select_network_backend(
    policy: &nono::WindowsNetworkPolicy,
) -> Result<Option<Box<dyn WindowsNetworkBackend>>> {
    if matches!(&policy.mode, nono::WindowsNetworkPolicyMode::AllowAll) && policy.has_port_rules() {
        return Ok(Some(Box::new(WfpNetworkBackend)));
    }

    match (&policy.mode, policy.active_backend) {
        (nono::WindowsNetworkPolicyMode::AllowAll, nono::WindowsNetworkBackendKind::None) => {
            Ok(None)
        }
        (
            nono::WindowsNetworkPolicyMode::Blocked,
            nono::WindowsNetworkBackendKind::FirewallRules,
        ) => Ok(Some(Box::new(FirewallRulesNetworkBackend))),
        (
            nono::WindowsNetworkPolicyMode::Blocked,
            nono::WindowsNetworkBackendKind::Wfp,
        ) => Ok(Some(Box::new(WfpNetworkBackend))),
        (nono::WindowsNetworkPolicyMode::Blocked, nono::WindowsNetworkBackendKind::None)
            if policy.preferred_backend == nono::WindowsNetworkBackendKind::Wfp =>
        {
            Ok(Some(Box::new(WfpNetworkBackend)))
        }
        (nono::WindowsNetworkPolicyMode::ProxyOnly { .. }, _)
            if policy.preferred_backend == nono::WindowsNetworkBackendKind::Wfp =>
        {
            Ok(Some(Box::new(WfpNetworkBackend)))
        }
        (_, active_backend) => Err(NonoError::UnsupportedPlatform(format!(
            "Windows network enforcement does not have an applicable active backend for this policy ({}, active backend: {}).",
            policy.backend_summary(),
            active_backend.label()
        ))),
    }
}

impl WindowsNetworkBackend for FirewallRulesNetworkBackend {
    fn label(&self) -> &'static str {
        "windows-firewall-rules"
    }

    fn install(
        &self,
        policy: &nono::WindowsNetworkPolicy,
        config: &ExecConfig<'_>,
        _session_id: Option<&str>,
    ) -> Result<Option<NetworkEnforcementGuard>> {
        let _ = Sandbox::windows_network_launch_support(policy, config.resolved_program);

        let (staged_program, staged_dir) =
            stage_program_for_blocked_network_launch(config.resolved_program)?;
        let suffix = unique_windows_firewall_rule_suffix();
        let inbound_rule = format!("nono-win-block-in-{suffix}");
        let outbound_rule = format!("nono-win-block-out-{suffix}");
        let program_arg = format!("program={}", staged_program.display());

        if let Err(err) = run_netsh_firewall(&[
            "advfirewall",
            "firewall",
            "add",
            "rule",
            &format!("name={outbound_rule}"),
            "dir=out",
            "action=block",
            &program_arg,
            "enable=yes",
            "profile=any",
        ]) {
            cleanup_network_enforcement_staging(&staged_dir);
            return Err(err);
        }

        if let Err(err) = run_netsh_firewall(&[
            "advfirewall",
            "firewall",
            "add",
            "rule",
            &format!("name={inbound_rule}"),
            "dir=in",
            "action=block",
            &program_arg,
            "enable=yes",
            "profile=any",
        ]) {
            let _ = delete_firewall_rule(&outbound_rule);
            cleanup_network_enforcement_staging(&staged_dir);
            return Err(err);
        }

        Ok(Some(NetworkEnforcementGuard::FirewallRules {
            staged_program,
            staged_dir,
            inbound_rule,
            outbound_rule,
        }))
    }
}

impl WindowsNetworkBackend for WfpNetworkBackend {
    fn label(&self) -> &'static str {
        "windows-filtering-platform"
    }

    fn install(
        &self,
        policy: &nono::WindowsNetworkPolicy,
        config: &ExecConfig<'_>,
        _session_id: Option<&str>,
    ) -> Result<Option<NetworkEnforcementGuard>> {
        let probe_config = current_wfp_probe_config()?;
        install_wfp_network_backend(policy, config, &probe_config)
    }
}

pub(super) fn install_wfp_network_backend_with_runner<P, R, S>(
    policy: &nono::WindowsNetworkPolicy,
    config: &ExecConfig<'_>,
    probe_config: &WfpProbeConfig,
    probe_fn: P,
    run_probe: R,
    start_service_fn: S,
) -> Result<Option<NetworkEnforcementGuard>>
where
    P: Fn(&WfpProbeConfig) -> Result<WfpProbeStatus>,
    R: Fn(&WfpProbeConfig, &WfpRuntimeActivationRequest) -> Result<WfpRuntimeProbeOutput>,
    S: Fn(&WfpProbeConfig) -> Result<WindowsWfpStartReport>,
{
    if matches!(&policy.mode, nono::WindowsNetworkPolicyMode::AllowAll) && !policy.has_port_rules()
    {
        return Ok(None);
    }

    match &policy.mode {
        nono::WindowsNetworkPolicyMode::AllowAll
        | nono::WindowsNetworkPolicyMode::Blocked
        | nono::WindowsNetworkPolicyMode::ProxyOnly { .. } => {
            let _ = Sandbox::windows_network_launch_support(policy, config.resolved_program);
            let initial_status = probe_fn(probe_config).map_err(|err| {
                NonoError::SandboxInit(format!(
                    "Failed to probe Windows WFP backend status ({}): {}",
                    policy.backend_summary(),
                    err
                ))
            })?;

            // D-03: if the service is stopped, attempt to start it before failing closed.
            // This provides defense-in-depth for dev layouts, user-scope installs, and
            // post-crash scenarios where the boot-start service was never started or was stopped.
            let status = if initial_status == WfpProbeStatus::BackendServiceStopped {
                match start_service_fn(probe_config) {
                    Ok(_) => {
                        // Service started; re-probe to get fresh status for the IPC path.
                        probe_fn(probe_config).map_err(|err| {
                            NonoError::SandboxInit(format!(
                                "Failed to re-probe Windows WFP backend status after auto-start ({}): {}",
                                policy.backend_summary(),
                                err
                            ))
                        })?
                    }
                    Err(_) => {
                        // Start attempt failed (e.g. not elevated). Fail closed with an
                        // actionable remediation message. Never return Ok(None) here.
                        return Err(NonoError::UnsupportedPlatform(format!(
                            "Windows WFP runtime activation is required for {} but the WFP service \
                             `{}` is not running and could not be started automatically \
                             (elevation is required). To start it, run this command once in an \
                             elevated (Administrator) terminal: `nono setup --start-wfp-service` \
                             ({}). This request remains fail-closed.",
                            describe_windows_network_runtime_target(policy),
                            probe_config.backend_service,
                            policy.backend_summary()
                        )));
                    }
                }
            } else {
                initial_status
            };

            if status == WfpProbeStatus::Ready {
                let suffix = unique_windows_firewall_rule_suffix();
                let outbound_rule = format!("nono-wfp-block-out-{suffix}");
                let inbound_rule = format!("nono-wfp-block-in-{suffix}");
                // Plan 62-12: feed the WFP request the per-run PACKAGE SID
                // (`S-1-15-2-*`, derived from the AppContainer name) so the
                // UNCHANGED ALE_USER_ID security-descriptor path scopes the
                // filter to the AppContainer child. The package SID is a normal
                // access-check participant, so the CC-probe matches AND the child
                // starts (debug D1 step 4 / D4). `package_sid` is single-sourced
                // with the broker's `--app-container-name` (same derivation).
                let request = build_wfp_target_activation_request(
                    policy,
                    config.resolved_program,
                    &outbound_rule,
                    &inbound_rule,
                    config.package_sid.as_deref(),
                );
                let probe_output = run_probe(probe_config, &request)?;
                return match parse_wfp_runtime_probe_status(&probe_output)? {
                    WfpRuntimeActivationProbeStatus::Ready => Err(NonoError::UnsupportedPlatform(
                        format!(
                            "Windows WFP service returned 'ready' in response to an activation request for {}, which is an unexpected protocol state that violates the WFP IPC contract ({}). This request remains fail-closed.",
                            describe_windows_network_runtime_target(policy),
                            policy.backend_summary()
                        ),
                    )),
                    WfpRuntimeActivationProbeStatus::NotImplemented => Err(
                        NonoError::UnsupportedPlatform(format!(
                            "Windows WFP runtime activation is required for {} but {} ({}). This request remains fail-closed until WFP activation is implemented.",
                            describe_windows_network_runtime_target(policy),
                            describe_wfp_runtime_probe_failure(probe_config, &probe_output),
                            policy.backend_summary()
                        )),
                    ),
                    WfpRuntimeActivationProbeStatus::AcceptedButNotEnforced => Err(
                        NonoError::UnsupportedPlatform(format!(
                            "Windows WFP network-policy activation was accepted by the service host but no filtering primitive was installed yet: {}. This request remains fail-closed.",
                            probe_output.response.details
                        )),
                    ),
                    WfpRuntimeActivationProbeStatus::EnforcedPendingCleanup => Ok(Some(
                        NetworkEnforcementGuard::WfpServiceManaged {
                            policy: Box::new(policy.clone()),
                            probe_config: probe_config.clone(),
                            target_program: config.resolved_program.to_path_buf(),
                            inbound_rule,
                            outbound_rule,
                        },
                    )),
                    WfpRuntimeActivationProbeStatus::CleanupSucceeded => Err(
                        NonoError::SandboxInit(
                            "Windows WFP activation returned cleanup success during install; this is an unexpected protocol state.".to_string(),
                        ),
                    ),
                    WfpRuntimeActivationProbeStatus::FilteringProbeSucceeded => Err(
                        NonoError::UnsupportedPlatform(format!(
                            "Windows WFP network-policy activation successfully exercised a service-owned filtering primitive, but it is not attached to the target process yet: {}. This request remains fail-closed.",
                            probe_output.response.details
                        )),
                    ),
                };
            }
            Err(NonoError::UnsupportedPlatform(
                describe_wfp_runtime_activation_failure(policy, probe_config, status),
            ))
        }
    }
}

pub(super) fn install_wfp_network_backend(
    policy: &nono::WindowsNetworkPolicy,
    config: &ExecConfig<'_>,
    probe_config: &WfpProbeConfig,
) -> Result<Option<NetworkEnforcementGuard>> {
    install_wfp_network_backend_with_runner(
        policy,
        config,
        probe_config,
        probe_wfp_backend_status_with_config,
        run_wfp_runtime_probe_with_request,
        |cfg| start_windows_wfp_service_with_runner(cfg, run_sc_query, run_sc_command),
    )
}

pub(super) fn prepare_network_enforcement(
    config: &ExecConfig<'_>,
    session_id: Option<&str>,
) -> Result<Option<NetworkEnforcementGuard>> {
    let policy = Sandbox::windows_network_policy(config.caps);
    if !policy.is_fully_supported() {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows network enforcement does not support this capability set yet ({}, {}).",
            policy.unsupported_messages().join(", "),
            policy.backend_summary()
        )));
    }

    let Some(backend) = select_network_backend(&policy)? else {
        return Ok(None);
    };

    tracing::debug!(
        "Windows network enforcement selecting backend {} ({})",
        backend.label(),
        policy.backend_summary()
    );

    backend.install(&policy, config, session_id)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_blocked_policy() -> nono::WindowsNetworkPolicy {
        nono::WindowsNetworkPolicy {
            mode: nono::WindowsNetworkPolicyMode::Blocked,
            tcp_connect_ports: vec![],
            tcp_bind_ports: vec![],
            localhost_ports: vec![],
            unsupported: vec![],
            preferred_backend: nono::WindowsNetworkBackendKind::Wfp,
            active_backend: nono::WindowsNetworkBackendKind::Wfp,
        }
    }

    fn make_test_probe_config() -> WfpProbeConfig {
        WfpProbeConfig {
            platform_service: WINDOWS_WFP_PLATFORM_SERVICE,
            backend_service: WINDOWS_WFP_BACKEND_SERVICE,
            backend_driver: WINDOWS_WFP_BACKEND_DRIVER,
            backend_binary_path: std::path::PathBuf::from(r"C:\tools\nono-wfp-service.exe"),
            backend_driver_binary_path: std::path::PathBuf::from(r"C:\tools\nono-wfp-driver.sys"),
            backend_service_args: WINDOWS_WFP_BACKEND_SERVICE_ARGS,
        }
    }

    // `sc query` output fragments that `parse_windows_service_state` recognizes.
    fn sc_missing_output() -> String {
        "[SC] EnumQueryServicesStatus:OpenService FAILED 1060: The specified service does not exist as an installed service.".to_string()
    }
    fn sc_running_output() -> String {
        "SERVICE_NAME: x\n        STATE              : 4  RUNNING".to_string()
    }
    fn sc_stopped_output() -> String {
        "SERVICE_NAME: x\n        STATE              : 1  STOPPED".to_string()
    }

    #[test]
    fn uninstall_stops_and_deletes_both_running_services() {
        let config = make_test_probe_config();
        let deleted: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
        let calls: std::cell::RefCell<Vec<Vec<String>>> = std::cell::RefCell::new(Vec::new());

        let query = |service: &str| -> Result<String> {
            if deleted.borrow().iter().any(|s| s == service) {
                Ok(sc_missing_output())
            } else {
                Ok(sc_running_output())
            }
        };
        let run = |args: &[String]| -> Result<String> {
            calls.borrow_mut().push(args.to_vec());
            if args.first().map(String::as_str) == Some("delete") {
                if let Some(name) = args.get(1) {
                    deleted.borrow_mut().push(name.clone());
                }
            }
            Ok(String::new())
        };
        let purge = |_p: &std::path::Path| -> Result<String> { Ok("purged".into()) };

        let report = uninstall_windows_wfp_with_runner(&config, query, run, purge).unwrap();
        assert_eq!(report.status_label, "removed");
        assert!(
            report.details.contains("wfp objects purged"),
            "purge note must appear in report details"
        );

        let calls = calls.into_inner();
        // Each running service is stopped THEN deleted.
        assert!(calls.contains(&build_wfp_service_stop_args(&config)));
        assert!(calls.contains(&build_wfp_service_delete_args(&config)));
        assert!(calls.contains(&build_wfp_driver_stop_args(&config)));
        assert!(calls.contains(&build_wfp_driver_delete_args(&config)));
    }

    #[test]
    fn uninstall_is_noop_when_nothing_installed() {
        let config = make_test_probe_config();
        let calls: std::cell::RefCell<Vec<Vec<String>>> = std::cell::RefCell::new(Vec::new());
        let query = |_service: &str| -> Result<String> { Ok(sc_missing_output()) };
        let run = |args: &[String]| -> Result<String> {
            calls.borrow_mut().push(args.to_vec());
            Ok(String::new())
        };
        let purge = |_p: &std::path::Path| -> Result<String> { Ok("purged".into()) };

        let report = uninstall_windows_wfp_with_runner(&config, query, run, purge).unwrap();
        assert_eq!(report.status_label, "nothing to remove");
        assert!(
            calls.into_inner().is_empty(),
            "no sc stop/delete should run when both services are missing"
        );
    }

    #[test]
    fn uninstall_deletes_stopped_service_without_stopping_it() {
        let config = make_test_probe_config();
        let deleted: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
        let calls: std::cell::RefCell<Vec<Vec<String>>> = std::cell::RefCell::new(Vec::new());

        let query = |service: &str| -> Result<String> {
            if deleted.borrow().iter().any(|s| s == service) {
                Ok(sc_missing_output())
            } else {
                Ok(sc_stopped_output())
            }
        };
        let run = |args: &[String]| -> Result<String> {
            calls.borrow_mut().push(args.to_vec());
            if args.first().map(String::as_str) == Some("delete") {
                if let Some(name) = args.get(1) {
                    deleted.borrow_mut().push(name.clone());
                }
            }
            Ok(String::new())
        };
        let purge = |_p: &std::path::Path| -> Result<String> { Ok("purged".into()) };

        let report = uninstall_windows_wfp_with_runner(&config, query, run, purge).unwrap();
        assert_eq!(report.status_label, "removed");
        let calls = calls.into_inner();
        assert!(
            !calls
                .iter()
                .any(|c| c.first().map(String::as_str) == Some("stop")),
            "an already-stopped service must not be stopped again"
        );
        assert!(calls.contains(&build_wfp_service_delete_args(&config)));
        assert!(calls.contains(&build_wfp_driver_delete_args(&config)));
    }

    #[test]
    fn uninstall_propagates_delete_failure() {
        let config = make_test_probe_config();
        let query = |_service: &str| -> Result<String> { Ok(sc_running_output()) };
        let run = |args: &[String]| -> Result<String> {
            if args.first().map(String::as_str) == Some("delete") {
                return Err(nono::NonoError::Setup("access is denied".to_string()));
            }
            Ok(String::new())
        };
        let purge = |_p: &std::path::Path| -> Result<String> { Ok("purged".into()) };

        let result = uninstall_windows_wfp_with_runner(&config, query, run, purge);
        assert!(
            result.is_err(),
            "a failing sc delete must surface as an error, not a silent success"
        );
    }

    /// Task 2 (62-11): a purge Err must NOT fail the uninstall call (fail-open
    /// contract, T-62-24). The purge outcome is recorded in report.details as
    /// "wfp object purge skipped (best-effort)" so the outcome is auditable.
    #[test]
    fn uninstall_purge_failure_is_fail_open() {
        let config = make_test_probe_config();
        let query = |_service: &str| -> Result<String> { Ok(sc_missing_output()) };
        let run = |_args: &[String]| -> Result<String> { Ok(String::new()) };
        // Simulate a purge error (e.g. BFE stopped, binary not reachable).
        let purge = |_p: &std::path::Path| -> Result<String> {
            Err(nono::NonoError::Setup("boom".to_string()))
        };

        let result = uninstall_windows_wfp_with_runner(&config, query, run, purge);
        assert!(
            result.is_ok(),
            "a failing purge must NOT abort uninstall (fail-open)"
        );
        let report = result.unwrap();
        assert!(
            report
                .details
                .contains("wfp object purge skipped (best-effort)"),
            "report details must record the skipped-purge note; got: {}",
            report.details
        );
    }

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
            // Plan 62-12: session_sid is the legacy synthetic SID; package_sid is
            // the per-run AppContainer package SID fed to WFP + DACL.
            session_sid: Some("S-1-5-117-123456789-1234-5678-9012".to_string()),
            app_container_name: Some("nono.session.deadbeefcafebabe0123456789abcdef".to_string()),
            package_sid: Some("S-1-15-2-1-2-3-4-5-6-7".to_string()),
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
         -> Result<WfpRuntimeProbeOutput> {
            Ok(WfpRuntimeProbeOutput {
                status_code: Some(0),
                response: WfpRuntimeActivationResponse {
                    protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
                    status: "enforced-pending-cleanup".to_string(),
                    details: "WFP filters installed".to_string(),
                },
                stderr: String::new(),
            })
        };

        let mock_start = |_cfg: &WfpProbeConfig| -> Result<WindowsWfpStartReport> {
            panic!("start_service_fn must not be called when probe returns Ready")
        };

        let result = install_wfp_network_backend_with_runner(
            &policy,
            &config,
            &probe_config,
            mock_probe,
            mock_runner,
            mock_start,
        );
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let guard = result.unwrap();
        assert!(guard.is_some(), "Expected Some(NetworkEnforcementGuard)");
        match guard.unwrap() {
            NetworkEnforcementGuard::WfpServiceManaged { .. } => {}
            other => panic!("Expected WfpServiceManaged, got: {:?}", other),
        }
    }

    #[test]
    fn install_wfp_network_backend_returns_error_on_prerequisites_missing() {
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
            // Plan 62-12: session_sid is the legacy synthetic SID; package_sid is
            // the per-run AppContainer package SID fed to WFP + DACL.
            session_sid: Some("S-1-5-117-123456789-1234-5678-9012".to_string()),
            app_container_name: Some("nono.session.deadbeefcafebabe0123456789abcdef".to_string()),
            package_sid: Some("S-1-15-2-1-2-3-4-5-6-7".to_string()),
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
         -> Result<WfpRuntimeProbeOutput> {
            Ok(WfpRuntimeProbeOutput {
                status_code: Some(0),
                response: WfpRuntimeActivationResponse {
                    protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
                    status: "prerequisites-missing".to_string(),
                    details: "Service not available".to_string(),
                },
                stderr: String::new(),
            })
        };

        let mock_start = |_cfg: &WfpProbeConfig| -> Result<WindowsWfpStartReport> {
            panic!("start_service_fn must not be called when probe returns Ready")
        };

        let result = install_wfp_network_backend_with_runner(
            &policy,
            &config,
            &probe_config,
            mock_probe,
            mock_runner,
            mock_start,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, NonoError::UnsupportedPlatform(_)),
            "Expected UnsupportedPlatform, got: {:?}",
            err
        );
    }

    #[test]
    fn build_wfp_target_activation_request_populates_session_sid() {
        let policy = make_blocked_policy();

        // Plan 62-12: the value carried by `session_sid` is now the per-run
        // PACKAGE SID (`S-1-15-2-*`, derived from the AppContainer name), fed to
        // the UNCHANGED ALE_USER_ID security-descriptor path. The field is reused
        // verbatim — only the value's SID class changed (S-1-5-117 → S-1-15-2).
        let request = build_wfp_target_activation_request(
            &policy,
            std::path::Path::new(r"C:\tools\agent.exe"),
            "nono-wfp-block-out-abc123",
            "nono-wfp-block-in-abc123",
            Some("S-1-15-2-1111111111-2222222222-3333333333-4444444444-5555555555-6666666666-7777777777"),
        );

        assert_eq!(
            request.session_sid.as_deref(),
            Some("S-1-15-2-1111111111-2222222222-3333333333-4444444444-5555555555-6666666666-7777777777")
        );
        assert_eq!(
            request.outbound_rule_name.as_deref(),
            Some("nono-wfp-block-out-abc123")
        );
        assert_eq!(
            request.inbound_rule_name.as_deref(),
            Some("nono-wfp-block-in-abc123")
        );
        assert_eq!(request.network_mode, "blocked");
        assert_eq!(request.request_kind, "activate_blocked_mode");
        assert_eq!(
            request.target_program_path.as_deref(),
            Some(r"C:\tools\agent.exe")
        );
    }

    #[test]
    fn build_wfp_target_activation_request_leaves_session_sid_none_for_appid_fallback() {
        let policy = make_blocked_policy();

        let request = build_wfp_target_activation_request(
            &policy,
            std::path::Path::new(r"C:\tools\agent.exe"),
            "nono-wfp-block-out-abc123",
            "nono-wfp-block-in-abc123",
            None,
        );

        assert!(request.session_sid.is_none());
        assert_eq!(
            request.outbound_rule_name.as_deref(),
            Some("nono-wfp-block-out-abc123")
        );
        assert_eq!(
            request.inbound_rule_name.as_deref(),
            Some("nono-wfp-block-in-abc123")
        );
    }

    // D-03 unit tests: verify auto-start hook branches without elevation.

    fn make_exec_config<'a>(
        command: &'a Vec<String>,
        resolved_program: &'a std::path::PathBuf,
        current_dir: &'a std::path::PathBuf,
        caps: &'a nono::CapabilitySet,
    ) -> ExecConfig<'a> {
        ExecConfig {
            command,
            resolved_program,
            caps,
            env_vars: vec![],
            cap_file: None,
            current_dir,
            // Plan 62-12: session_sid is the legacy synthetic SID; package_sid is
            // the per-run AppContainer package SID fed to WFP + DACL.
            session_sid: Some("S-1-5-117-123456789-1234-5678-9012".to_string()),
            app_container_name: Some("nono.session.deadbeefcafebabe0123456789abcdef".to_string()),
            package_sid: Some("S-1-15-2-1-2-3-4-5-6-7".to_string()),
            interactive_shell: false,
            session_token: None,
            cap_pipe_rendezvous_path: None,
            allowed_env_vars: None,
            denied_env_vars: None,
            prefers_low_il_broker: false,
        }
    }

    /// D-03: probe returns Stopped → start_service_fn succeeds → re-probe returns Ready
    /// → IPC mock returns EnforcedPendingCleanup → result is Ok(Some(WfpServiceManaged)).
    #[test]
    fn test_wfp_autostart_on_stopped() {
        let policy = make_blocked_policy();
        let caps = nono::CapabilitySet::new();
        let command = vec!["agent.exe".to_string()];
        let resolved_program = std::path::PathBuf::from(r"C:\tools\agent.exe");
        let current_dir = std::path::PathBuf::from(r"C:\workspace");
        let config = make_exec_config(&command, &resolved_program, &current_dir, &caps);
        let probe_config = make_test_probe_config();

        // First call: BackendServiceStopped; second call (after auto-start): Ready.
        let call_count = std::cell::Cell::new(0u32);
        let mock_probe = |_cfg: &WfpProbeConfig| -> Result<WfpProbeStatus> {
            let n = call_count.get();
            call_count.set(n + 1);
            if n == 0 {
                Ok(WfpProbeStatus::BackendServiceStopped)
            } else {
                Ok(WfpProbeStatus::Ready)
            }
        };

        let mock_start = |_cfg: &WfpProbeConfig| -> Result<WindowsWfpStartReport> {
            Ok(WindowsWfpStartReport {
                status_label: "running",
                details: "service started by mock".to_string(),
            })
        };

        let mock_runner = |_cfg: &WfpProbeConfig,
                           _req: &WfpRuntimeActivationRequest|
         -> Result<WfpRuntimeProbeOutput> {
            Ok(WfpRuntimeProbeOutput {
                status_code: Some(0),
                response: WfpRuntimeActivationResponse {
                    protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
                    status: "enforced-pending-cleanup".to_string(),
                    details: "WFP filters installed".to_string(),
                },
                stderr: String::new(),
            })
        };

        let result = install_wfp_network_backend_with_runner(
            &policy,
            &config,
            &probe_config,
            mock_probe,
            mock_runner,
            mock_start,
        );
        assert!(
            result.is_ok(),
            "Expected Ok after auto-start, got: {:?}",
            result
        );
        let guard = result.unwrap();
        assert!(guard.is_some(), "Expected Some(NetworkEnforcementGuard)");
        match guard.unwrap() {
            NetworkEnforcementGuard::WfpServiceManaged { .. } => {}
            other => panic!("Expected WfpServiceManaged, got: {:?}", other),
        }
    }

    /// D-03: probe returns Stopped → start_service_fn returns Err → result is
    /// Err(UnsupportedPlatform) with message containing service name, "could not be started
    /// automatically", "elevated", and "nono setup --start-wfp-service".
    #[test]
    fn test_wfp_autostart_fail_remediation_message() {
        let policy = make_blocked_policy();
        let caps = nono::CapabilitySet::new();
        let command = vec!["agent.exe".to_string()];
        let resolved_program = std::path::PathBuf::from(r"C:\tools\agent.exe");
        let current_dir = std::path::PathBuf::from(r"C:\workspace");
        let config = make_exec_config(&command, &resolved_program, &current_dir, &caps);
        let probe_config = make_test_probe_config();

        let mock_probe = |_cfg: &WfpProbeConfig| -> Result<WfpProbeStatus> {
            Ok(WfpProbeStatus::BackendServiceStopped)
        };

        let mock_start = |_cfg: &WfpProbeConfig| -> Result<WindowsWfpStartReport> {
            Err(nono::NonoError::Setup("elevation required".to_string()))
        };

        let mock_runner = |_cfg: &WfpProbeConfig,
                           _req: &WfpRuntimeActivationRequest|
         -> Result<WfpRuntimeProbeOutput> {
            panic!("run_probe should never be called when start_service_fn fails")
        };

        let result = install_wfp_network_backend_with_runner(
            &policy,
            &config,
            &probe_config,
            mock_probe,
            mock_runner,
            mock_start,
        );
        assert!(result.is_err(), "Expected Err, got: {:?}", result);
        let err = result.unwrap_err();
        match &err {
            nono::NonoError::UnsupportedPlatform(msg) => {
                assert!(
                    msg.contains("nono-wfp-service"),
                    "Expected service name in message, got: {msg}"
                );
                assert!(
                    msg.contains("nono setup --start-wfp-service"),
                    "Expected remediation command in message, got: {msg}"
                );
                assert!(
                    msg.contains("elevated"),
                    "Expected 'elevated' in message, got: {msg}"
                );
            }
            other => panic!("Expected UnsupportedPlatform, got: {:?}", other),
        }
    }

    /// D-03: probe returns BackendServiceMissing (not BackendServiceStopped) → start_service_fn
    /// is never called → result is Err(UnsupportedPlatform) with the BackendServiceMissing message
    /// (does NOT contain "could not be started automatically").
    #[test]
    fn test_wfp_non_stopped_status_unchanged() {
        let policy = make_blocked_policy();
        let caps = nono::CapabilitySet::new();
        let command = vec!["agent.exe".to_string()];
        let resolved_program = std::path::PathBuf::from(r"C:\tools\agent.exe");
        let current_dir = std::path::PathBuf::from(r"C:\workspace");
        let config = make_exec_config(&command, &resolved_program, &current_dir, &caps);
        let probe_config = make_test_probe_config();

        let mock_probe = |_cfg: &WfpProbeConfig| -> Result<WfpProbeStatus> {
            Ok(WfpProbeStatus::BackendServiceMissing)
        };

        // This closure must never be called; panic if it is.
        let mock_start = |_cfg: &WfpProbeConfig| -> Result<WindowsWfpStartReport> {
            panic!("start_service_fn must NOT be called for BackendServiceMissing")
        };

        let mock_runner = |_cfg: &WfpProbeConfig,
                           _req: &WfpRuntimeActivationRequest|
         -> Result<WfpRuntimeProbeOutput> {
            panic!("run_probe should never be called when service is missing")
        };

        let result = install_wfp_network_backend_with_runner(
            &policy,
            &config,
            &probe_config,
            mock_probe,
            mock_runner,
            mock_start,
        );
        assert!(result.is_err(), "Expected Err, got: {:?}", result);
        let err = result.unwrap_err();
        match &err {
            nono::NonoError::UnsupportedPlatform(msg) => {
                assert!(
                    !msg.contains("could not be started automatically"),
                    "BackendServiceMissing must NOT route through auto-start path, got: {msg}"
                );
            }
            other => panic!("Expected UnsupportedPlatform, got: {:?}", other),
        }
    }

    /// 62-06: build_wfp_probe_status returns Ready when BFE and backend service are
    /// both Running — no kernel driver required (F-62-UAT-01 fix, D-05 service-only).
    #[test]
    fn test_wfp_ready_without_kernel_driver() {
        let status = build_wfp_probe_status(
            true,                         // backend_binary_exists
            WindowsServiceState::Running, // platform_service (BFE)
            WindowsServiceState::Running, // backend_service (nono-wfp-service)
        );
        assert_eq!(
            status,
            WfpProbeStatus::Ready,
            "BFE Running + service Running must yield Ready with no driver requirement"
        );
    }

    /// 62-06: fail-secure regression — BFE stopped still short-circuits before Ready.
    #[test]
    fn test_wfp_platform_service_stopped_is_fail_closed() {
        let status = build_wfp_probe_status(
            true,
            WindowsServiceState::Stopped, // BFE stopped
            WindowsServiceState::Running,
        );
        assert_eq!(status, WfpProbeStatus::PlatformServiceStopped);
    }

    /// 62-06: fail-secure regression — backend service missing still short-circuits.
    #[test]
    fn test_wfp_backend_service_missing_is_fail_closed() {
        let status = build_wfp_probe_status(
            true,
            WindowsServiceState::Running,
            WindowsServiceState::Missing, // backend service missing
        );
        assert_eq!(status, WfpProbeStatus::BackendServiceMissing);
    }

    /// 62-06: fail-secure regression — backend service stopped still short-circuits.
    #[test]
    fn test_wfp_backend_service_stopped_is_fail_closed() {
        let status = build_wfp_probe_status(
            true,
            WindowsServiceState::Running,
            WindowsServiceState::Stopped, // backend service stopped
        );
        assert_eq!(status, WfpProbeStatus::BackendServiceStopped);
    }

    /// 62-06: fail-secure regression — binary missing still short-circuits.
    #[test]
    fn test_wfp_backend_binary_missing_is_fail_closed() {
        let status = build_wfp_probe_status(
            false, // binary missing
            WindowsServiceState::Running,
            WindowsServiceState::Running,
        );
        assert_eq!(status, WfpProbeStatus::BackendBinaryMissing);
    }
}
