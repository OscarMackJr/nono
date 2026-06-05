#[cfg(not(target_os = "windows"))]
use crate::launch_runtime::select_threading_context;
use crate::launch_runtime::LaunchPlan;
use crate::profile::AllowDomainEntry;
use crate::proxy_runtime::start_proxy_runtime;
use crate::sandbox_state::{DomainEndpointState, EndpointRuleState};
use crate::supervised_runtime::{execute_supervised_runtime, SupervisedRuntimeContext};
use crate::{config, exec_strategy, output, sandbox_state, session, DETACHED_SESSION_ID_ENV};
#[cfg(unix)]
use crate::hook_runtime;
#[cfg(windows)]
use crate::hook_runtime_windows;
use nono::{CapabilitySet, NonoError, Result, Sandbox};
use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::time::Duration;
use tracing::{error, info};
use zeroize::Zeroize;

fn apply_pre_fork_sandbox(
    strategy: exec_strategy::ExecStrategy,
    caps: &CapabilitySet,
    silent: bool,
) -> Result<()> {
    if matches!(strategy, exec_strategy::ExecStrategy::Direct) {
        output::print_applying_sandbox(silent);

        #[cfg(target_os = "linux")]
        {
            let detected = Sandbox::detect_abi()?;
            info!("Direct mode: detected {}", detected);
            Sandbox::apply_with_abi(caps, &detected)?;
        }

        #[cfg(not(target_os = "linux"))]
        {
            Sandbox::apply(caps)?;
        }
    }
    Ok(())
}

fn cleanup_capability_state_file(cap_file_path: &std::path::Path) {
    if cap_file_path.exists() {
        let _ = std::fs::remove_file(cap_file_path);
    }
}

fn next_capability_state_file_path() -> std::path::PathBuf {
    use rand::RngExt;

    let mut rng = rand::rng();
    let bytes: [u8; 8] = rng.random();
    let suffix = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    std::env::temp_dir().join(format!(".nono-{suffix}.json"))
}

/// Compute the canonical path and SHA-256 identity of the launched executable.
///
/// Delegates to `crate::exec_identity::compute` which owns the implementation;
/// this thin wrapper exists so `execution_runtime` tests can call the same
/// name-shape that upstream b5f0a3ab uses in its test module, and so the
/// audit-identity call at the `execute_sandboxed` launch point is clearly named.
fn compute_executable_identity(
    resolved_program: &std::path::Path,
) -> crate::Result<nono::undo::ExecutableIdentity> {
    crate::exec_identity::compute(resolved_program)
}

pub(crate) fn execution_start_dir(
    workdir: &std::path::Path,
    _caps: &CapabilitySet,
) -> Result<std::path::PathBuf> {
    let workdir_canonical =
        workdir
            .canonicalize()
            .map_err(|e| NonoError::PathCanonicalization {
                path: workdir.to_path_buf(),
                source: e,
            })?;

    #[cfg(target_os = "windows")]
    {
        Ok(workdir_canonical)
    }

    #[cfg(not(target_os = "windows"))]
    if _caps.path_covered(&workdir_canonical) {
        Ok(workdir_canonical)
    } else {
        Ok(std::path::PathBuf::from("/"))
    }
}

fn recommended_builtin_profile(program: &Path) -> Option<&'static str> {
    let name = program.file_name()?.to_str()?;
    match name {
        "claude" => Some("claude-code"),
        "codex" => Some("codex"),
        "opencode" => Some("opencode"),
        "openclaw" => Some("openclaw"),
        "swival" => Some("swival"),
        _ => None,
    }
}

pub(crate) fn execute_sandboxed(plan: LaunchPlan) -> Result<()> {
    let LaunchPlan {
        program,
        cmd_args,
        mut caps,
        loaded_secrets,
        // Plan 18.1-03 G-06: kept as owned local binding so downstream
        // `SupervisedRuntimeContext.loaded_profile: Option<&Profile>`
        // reference lives for the entire execute_sandboxed scope (which is
        // also the process lifetime — execute_supervised_runtime does not
        // return until the child exits, and the caller std::process::exit's).
        loaded_profile,
        flags,
    } = plan;
    let rollback = &flags.rollback;
    let trust = &flags.trust;
    let proxy = &flags.proxy;
    let session = &flags.session;

    if let Some(blocked) =
        config::check_blocked_command(&program, caps.allowed_commands(), caps.blocked_commands())?
    {
        return Err(NonoError::BlockedCommand {
            command: blocked,
            reason: "This command is blocked by default due to destructive potential. 
                     Use --allow-command to override if you understand the risks."
                .to_string(),
        });
    }

    let command: Vec<String> = std::iter::once(program.to_string_lossy().into_owned())
        .chain(
            cmd_args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned()),
        )
        .collect();

    if command.is_empty() {
        return Err(NonoError::NoCommand);
    }

    let resolved_program = exec_strategy::resolve_program(&command[0])?;
    let known_builtin_profile = recommended_builtin_profile(&resolved_program);
    let recommended_profile = if flags.session.profile_name.is_none() {
        known_builtin_profile
    } else {
        None
    };

    let recommended_program_name = resolved_program
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&command[0]);

    if let Some(profile) = recommended_profile {
        output::print_profile_hint(recommended_program_name, profile, flags.silent);
    }
    // Derive plain domain strings and endpoint state from Vec<AllowDomainEntry>.
    // domain_endpoints captures WithEndpoints entries for the capability state file;
    // plain_allowed_domains is the flat domain list for the allowed_domains field.
    let domain_endpoints: Vec<DomainEndpointState> = flags
        .proxy
        .allow_domain
        .iter()
        .filter_map(|entry| {
            if let AllowDomainEntry::WithEndpoints { domain, endpoints } = entry {
                Some(DomainEndpointState {
                    domain: domain.clone(),
                    endpoints: endpoints
                        .iter()
                        .map(|e| EndpointRuleState {
                            method: e.method.clone(),
                            path: e.path.clone(),
                        })
                        .collect(),
                })
            } else {
                None
            }
        })
        .collect();
    let plain_allowed_domains: Vec<String> = flags
        .proxy
        .allow_domain
        .iter()
        .map(|e| e.domain().to_string())
        .collect();
    let cap_file = write_capability_state_file(
        &caps,
        &flags.bypass_protection_paths,
        &plain_allowed_domains,
        &domain_endpoints,
        flags.silent,
    );
    let cap_file_path = cap_file.as_ref().cloned().unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        return std::path::PathBuf::from("NUL");
        #[cfg(not(target_os = "windows"))]
        return std::path::PathBuf::from("/dev/null");
    });

    for secret in &loaded_secrets {
        if exec_strategy::is_dangerous_env_var(&secret.env_var) {
            return Err(NonoError::ConfigParse(format!(
                "secret mapping targets dangerous environment variable: {}",
                secret.env_var
            )));
        }
    }

    let strategy = flags.strategy;

    if matches!(strategy, exec_strategy::ExecStrategy::Supervised) {
        output::print_supervised_info(flags.silent, rollback.requested, proxy.active);
    }

    // Fail-secure guard (D-02): if the user's intent is proxy-only mode
    // (caps were set to ProxyOnly by a profile or credential path at
    // capability_ext.rs:535) but `proxy.active` is false (no network profile,
    // no credentials, no upstream proxy — see proxy_runtime.rs lines 52-78),
    // the sandboxed process would have no proxy to route traffic through.
    // Fail before `start_proxy_runtime` and before any WFP/sandbox activation.
    if matches!(caps.network_mode(), nono::NetworkMode::ProxyOnly { .. }) && !proxy.active {
        return Err(NonoError::SandboxInit(
            "Cannot use proxy-only mode without a network profile or credential configuration."
                .to_string(),
        ));
    }

    let active_proxy = start_proxy_runtime(proxy, &mut caps)?;
    let proxy_env_vars = active_proxy.env_vars;
    let proxy_handle = active_proxy.handle;

    let current_dir = execution_start_dir(&flags.workdir, &caps)?;

    // AUD-03 SHA-256 portion (upstream 02ee0bd1): capture the canonical
    // path + SHA-256 of the executable BEFORE sandbox apply so the audit
    // trail commits to exactly the bytes the supervisor handed off. Only
    // computed for Supervised strategy (Direct/Monitor record nothing).
    // Hash failure is non-fatal for the launch path: `crate::exec_identity::compute`
    // bubbles `NonoError::CommandExecution`; we propagate that so the user
    // sees a concrete diagnostic rather than running with no audit identity.
    let executable_identity = if matches!(strategy, exec_strategy::ExecStrategy::Supervised) {
        Some(compute_executable_identity(&resolved_program)?)
    } else {
        None
    };

    apply_pre_fork_sandbox(strategy, &caps, flags.silent)?;

    // Session id shared across before- and after-hook so paired setup/teardown
    // scripts see the same NONO_SESSION_ID. Only allocated when at least one
    // hook is configured.
    let hook_session_id: Option<String> =
        (flags.session_hooks.before.is_some() || flags.session_hooks.after.is_some()).then(|| {
            std::env::var(DETACHED_SESSION_ID_ENV)
                .ok()
                .filter(|id| !id.is_empty())
                .unwrap_or_else(session::generate_session_id)
        });

    // ---- Before-hook execution ----
    // FORK DIVERGENCE (D-01/D-03): ? propagates Err (session aborts).
    // Upstream daa55c8 warns and continues (fail-open); fork propagates Err (fail-closed).
    let mut hook_env_vars_owned: Vec<(String, String)> = if let Some((before, session_id)) =
        flags.session_hooks.before.as_ref().zip(hook_session_id.as_deref())
    {
        #[cfg(unix)]
        {
            // FORK DIVERGENCE (D-01/D-03): ? propagates Err, not warn-and-continue
            hook_runtime::execute_before_hook(before, session_id, &current_dir)?
        }
        #[cfg(windows)]
        {
            // FORK DIVERGENCE (D-01/D-03): ? propagates Err, not warn-and-continue
            hook_runtime_windows::execute_before_hook(before, session_id, &current_dir)?
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (before, session_id);
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Generate per-session runtime capability expansion credentials BEFORE
    // building `env_vars` so the owned strings outlive the `ExecConfig`.
    //
    // `NONO_SESSION_TOKEN`: 32 random bytes, hex-encoded (64 chars). This is
    // the secret the sandboxed child must echo back on every
    // `RequestCapability` message; the supervisor validates it in constant
    // time before invoking any approval backend.
    //
    // `NONO_SUPERVISOR_PIPE`: rendezvous file path where the supervisor
    // capability pipe server binds. Unique per session id to avoid collisions.
    //
    // Never log `windows_session_token`.
    #[cfg(target_os = "windows")]
    let windows_session_token: String = {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| {
            NonoError::SandboxInit(format!("Failed to generate session token: {e}"))
        })?;
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    #[cfg(target_os = "windows")]
    let windows_cap_pipe_path: std::path::PathBuf =
        std::env::temp_dir().join(format!("nono-cap-{}.pipe", flags.session.session_id));
    #[cfg(target_os = "windows")]
    let windows_cap_pipe_path_str: String = windows_cap_pipe_path.to_string_lossy().into_owned();

    let mut env_vars: Vec<(&str, &str)> = loaded_secrets
        .iter()
        .map(|secret| (secret.env_var.as_str(), secret.value.as_str()))
        .collect();
    for (key, value) in &proxy_env_vars {
        env_vars.push((key.as_str(), value.as_str()));
    }
    #[cfg(target_os = "windows")]
    {
        env_vars.push(("NONO_SESSION_TOKEN", windows_session_token.as_str()));
        env_vars.push(("NONO_SUPERVISOR_PIPE", windows_cap_pipe_path_str.as_str()));
    }

    // Hook env vars have lowest priority: prepend so secrets and proxy override.
    for (key, value) in hook_env_vars_owned.iter().rev() {
        env_vars.insert(0, (key.as_str(), value.as_str()));
    }
    // Note: hook_env_vars_owned values are zeroized after config is dropped below,
    // once the &str borrows in env_vars are no longer live (CLAUDE.md §Memory,
    // T-58-02-08). Inline zeroize here would conflict with the outstanding &str
    // references inserted into env_vars.

    #[cfg(not(target_os = "windows"))]
    let threading = select_threading_context(
        !loaded_secrets.is_empty(),
        proxy.active,
        trust.scan_performed,
        trust.interception_active,
    );

    #[cfg(not(target_os = "windows"))]
    info!(
        "Executing with strategy: {:?}, threading: {:?}",
        strategy, threading
    );

    #[cfg(target_os = "windows")]
    info!("Executing with strategy: {:?}", strategy);

    #[cfg(target_os = "linux")]
    let seccomp_proxy_fallback = {
        let needs_proxy = matches!(caps.network_mode(), nono::NetworkMode::ProxyOnly { .. });
        if needs_proxy && nono::is_wsl2() {
            let needs_seccomp_fallback = !Sandbox::detect_abi()
                .ok()
                .is_some_and(|abi| abi.has_network());
            if needs_seccomp_fallback {
                match flags.wsl2_proxy_policy {
                    crate::profile::Wsl2ProxyPolicy::Error => {
                        return Err(NonoError::SandboxInit(
                            "WSL2: proxy-only network mode cannot be kernel-enforced. 
                             seccomp user notification returns EBUSY on WSL2 and Landlock V4 
                             (per-port TCP filtering) is not available on this kernel.


                             The sandboxed process would be able to bypass the credential proxy 
                             and open arbitrary outbound connections.


                             To allow degraded execution (credential proxy without network lockdown), 
                             set wsl2_proxy_policy: \"insecure_proxy\" in your profile's security config.


                             See: https://nono.sh/docs/cli/internals/wsl2"
                                .to_string(),
                        ));
                    }
                    crate::profile::Wsl2ProxyPolicy::InsecureProxy => {
                        eprintln!(
                            "  [nono] WARNING: WSL2 insecure proxy mode — credential proxy active 
                             but network is NOT kernel-enforced. The sandboxed process can bypass 
                             the proxy and open arbitrary outbound connections."
                        );
                    }
                }
            }
            false
        } else if needs_proxy {
            !Sandbox::detect_abi()
                .ok()
                .is_some_and(|abi| abi.has_network())
        } else {
            false
        }
    };

    #[cfg(target_os = "linux")]
    if flags.af_unix_mediation.is_pathname() && nono::sandbox::is_wsl2() {
        return Err(NonoError::SandboxInit(
            "WSL2: linux.af_unix_mediation = \"pathname\" requires seccomp user notification, \
             but WSL2 reports EBUSY for seccomp notify listeners. Disable AF_UNIX mediation or \
             run on native Linux."
                .to_string(),
        ));
    }

    // Pre-compute ignored denial paths from the loaded profile's
    // `filesystem.suppress_save_prompt`. Used to annotate denied paths with
    // `[save skipped]` in the diagnostic footer (UX gate, not security gate).
    // Failure is non-fatal: empty list = no annotations (conservative default).
    #[cfg(not(target_os = "windows"))]
    let ignored_denial_paths: Vec<std::path::PathBuf> = loaded_profile
        .as_ref()
        .map(|p| {
            p.filesystem
                .suppress_save_prompt
                .iter()
                .map(|raw| {
                    crate::profile_save_runtime::canonicalize_suppress_path(raw)
                })
                .collect()
        })
        .unwrap_or_default();

    #[cfg(not(target_os = "windows"))]
    let config = exec_strategy::ExecConfig {
        command: &command,
        resolved_program: &resolved_program,
        caps: &caps,
        env_vars,
        cap_file: cap_file.as_deref(),
        current_dir: &current_dir,
        no_diagnostics: flags.no_diagnostics || flags.silent,
        threading,
        protected_paths: &trust.protected_paths,
        profile_save_base: flags
            .session
            .profile_name
            .as_deref()
            .or(recommended_profile),
        startup_timeout: flags
            .startup_timeout_secs
            .filter(|&secs| secs > 0)
            .map(|secs| exec_strategy::StartupTimeoutConfig {
                timeout: Duration::from_secs(secs),
                program: recommended_program_name,
                recommended_profile: known_builtin_profile,
            }),
        capability_elevation: flags.capability_elevation,
        #[cfg(target_os = "linux")]
        seccomp_proxy_fallback,
        #[cfg(target_os = "linux")]
        af_unix_mediation: flags.af_unix_mediation,
        allowed_env_vars: flags.allowed_env_vars,
        denied_env_vars: flags.denied_env_vars,
        ignored_denial_paths: &ignored_denial_paths,
    };
    // Plan 62-12 (F-62-UAT-05 redesign): the SINGLE per-run identifier for the
    // WFP-enforced broker-no-PTY arm is the AppContainer moniker
    // `nono.session.<uuid>`. From it we deterministically derive the package SID
    // (`S-1-15-2-*`) — the value the WFP filter keys on AND the value the DACL
    // guard grants to the AppContainer child. The name flows to the broker via
    // `--app-container-name`, where it derives the SAME package SID for the
    // lowbox `SECURITY_CAPABILITIES`. Never two ids for the AppContainer arm.
    //
    // `session_sid` (the synthetic `S-1-5-117-*`) is RETAINED for the legacy,
    // non-broker `WriteRestricted` arm only (the package SID is NOT a restricting
    // SID). The two arms are mutually exclusive (BrokerLaunchNoPty XOR
    // WriteRestricted), so the synthetic SID and package SID never coexist on a
    // single child.
    //
    // FAIL-CLOSED: if the package SID cannot be derived, the run aborts here
    // rather than launching a child whose WFP filter would match nothing.
    #[cfg(target_os = "windows")]
    let windows_app_container_name = exec_strategy::generate_app_container_name();
    #[cfg(target_os = "windows")]
    let windows_package_sid = {
        let psid = nono::derive_app_container_sid(&windows_app_container_name)?;
        nono::package_sid_to_string(&psid)?
    };
    #[cfg(target_os = "windows")]
    let config = exec_strategy::ExecConfig {
        command: &command,
        resolved_program: &resolved_program,
        caps: &caps,
        env_vars,
        cap_file: cap_file.as_deref(),
        current_dir: &current_dir,
        // Synthetic SID for the legacy WriteRestricted arm (mutually exclusive
        // with the AppContainer arm). The WFP request + DACL guard read the
        // package SID below, NOT this field.
        session_sid: Some(exec_strategy::generate_session_sid()),
        app_container_name: Some(windows_app_container_name),
        package_sid: Some(windows_package_sid),
        interactive_shell: flags.interactive_shell,
        session_token: Some(windows_session_token.clone()),
        cap_pipe_rendezvous_path: Some(windows_cap_pipe_path.clone()),
        allowed_env_vars: flags.allowed_env_vars.clone(),
        denied_env_vars: flags.denied_env_vars.clone(),
        // Phase 51 D-02: source from profile.windows_low_il_broker.
        // loaded_profile is in scope (LaunchPlan destructure at line ~114).
        // is_some_and(...) is fail-safe: no profile → false → WriteRestricted
        // (existing non-PTY supervised behavior preserved, no security downgrade).
        prefers_low_il_broker: loaded_profile
            .as_ref()
            .is_some_and(|p| p.windows_low_il_broker),
    };

    // Resource limits are now kernel-enforced on Linux (cgroup v2) and macOS
    // (setrlimit). The old "not enforced on linux/macos" warnings were removed
    // in Phase 25 Plan 01. Enforcement is wired inside execute_direct and
    // execute_supervised_runtime via apply_resource_limits_unix.

    match strategy {
        exec_strategy::ExecStrategy::Direct => {
            #[cfg(target_os = "windows")]
            {
                let exit_code = exec_strategy::execute_direct(
                    &config,
                    &flags.resource_limits,
                    Some(flags.session.session_id.as_str()),
                )?;
                cleanup_capability_state_file(&cap_file_path);
                drop(config);
                // Zeroize hook env-var values after injection; values may be credentials (CLAUDE.md §Memory).
                // config is dropped above so &str borrows into hook_env_vars_owned are no longer live.
                for (_, value) in &mut hook_env_vars_owned {
                    value.zeroize();
                }
                drop(loaded_secrets);
                std::process::exit(exit_code);
            }
            #[cfg(not(target_os = "windows"))]
            {
                exec_strategy::execute_direct(
                    &config,
                    // Phase 25-01: kernel-level resource enforcement (Linux cgroup v2, macOS setrlimit).
                    #[cfg(any(target_os = "linux", target_os = "macos"))]
                    &flags.resource_limits,
                    #[cfg(any(target_os = "linux", target_os = "macos"))]
                    flags.session.session_id.as_str(),
                )?;
                unreachable!("execute_direct only returns on error");
            }
        }
        exec_strategy::ExecStrategy::Supervised => {
            let exit_code = execute_supervised_runtime(SupervisedRuntimeContext {
                config: &config,
                caps: &caps,
                command: &command,
                capability_elevation: flags.capability_elevation,
                session,
                rollback,
                trust,
                proxy,
                proxy_handle: proxy_handle.as_ref(),
                executable_identity: executable_identity.as_ref(),
                // audit_signer is created inside execute_supervised_runtime from
                // rollback.audit_sign_key (fork pattern: signer lifetime is local to
                // execute_supervised_runtime, not pre-computed by execution_runtime).
                audit_signer: None,
                redaction_policy: &flags.redaction_policy,
                silent: flags.silent,
                resource_limits: &flags.resource_limits,
                // Plan 18.1-03 G-06: pass a reference to the owned
                // `loaded_profile` binding so `execute_supervised_runtime`
                // can call `Profile::resolve_aipc_allowlist()` at Windows
                // `SupervisorConfig` construction time.
                loaded_profile: loaded_profile.as_ref(),
            })?;

            // ---- After-hook execution ----
            // FORK DIVERGENCE (D-04): ? propagates Err so CI sees non-zero exit.
            // Upstream daa55c8 warns and swallows (fail-open); fork propagates Err (fail-closed).
            if let Some((after, session_id)) =
                flags.session_hooks.after.as_ref().zip(hook_session_id.as_deref())
            {
                #[cfg(unix)]
                // FORK DIVERGENCE (D-01/D-03): ? propagates Err, not warn-and-continue
                hook_runtime::execute_after_hook(after, session_id, &current_dir, exit_code)?;
                #[cfg(windows)]
                // FORK DIVERGENCE (D-01/D-03): ? propagates Err, not warn-and-continue
                hook_runtime_windows::execute_after_hook(after, session_id, &current_dir, exit_code)?;
                #[cfg(not(any(unix, windows)))]
                {
                    let _ = (after, session_id, exit_code);
                }
            }

            cleanup_capability_state_file(&cap_file_path);
            drop(config);
            // Zeroize hook env-var values after injection; values may be credentials (CLAUDE.md §Memory).
            // config is dropped above so &str borrows into hook_env_vars_owned are no longer live.
            for (_, value) in &mut hook_env_vars_owned {
                value.zeroize();
            }
            drop(loaded_secrets);
            std::process::exit(exit_code);
        }
    }
}

fn write_capability_state_file(
    caps: &CapabilitySet,
    bypass_protection_paths: &[std::path::PathBuf],
    allowed_domains: &[String],
    domain_endpoints: &[DomainEndpointState],
    silent: bool,
) -> Option<std::path::PathBuf> {
    let state = sandbox_state::SandboxState::from_caps(
        caps,
        bypass_protection_paths,
        allowed_domains,
        domain_endpoints,
    );

    for _ in 0..8 {
        let cap_file = next_capability_state_file_path();
        match state.write_to_file(&cap_file) {
            Ok(()) => return Some(cap_file),
            Err(NonoError::ConfigWrite { source, .. })
                if source.kind() == std::io::ErrorKind::AlreadyExists =>
            {
                continue;
            }
            Err(e) => {
                error!(
                    "Failed to write capability state file: {}. 
                     Sandboxed processes will not be able to query their own capabilities using 'nono why --self'.",
                    e
                );
                if !silent {
                    eprintln!(
                        "  WARNING: Capability state file could not be written.
  
                         The sandbox is active, but 'nono why --self' will not work inside this sandbox."
                    );
                }
                return None;
            }
        }
    }

    error!(
        "Failed to allocate a unique capability state file after repeated collisions. 
         Sandboxed processes will not be able to query their own capabilities using 'nono why --self'."
    );
    if !silent {
        eprintln!(
            "  WARNING: Capability state file could not be written.
  
             The sandbox is active, but 'nono why --self' will not work inside this sandbox."
        );
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{compute_executable_identity, recommended_builtin_profile};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::Path;

    /// D-03 behavioral test: before-hook returning Err must propagate to the caller.
    ///
    /// This test verifies the fail-closed ? propagation at the hook dispatch level
    /// (execute_before_hook). Full execute_sandboxed integration requires a
    /// complete LaunchPlan which is not feasible in a unit test; however, the
    /// dispatch code in execute_sandboxed uses `?` which is the same propagation
    /// mechanism tested here. This test provides the behavioral guarantee that a
    /// before-hook returning Err causes the caller to see Err (D-03).
    ///
    /// On Unix: calls execute_before_hook with a script that exits 1 → asserts Err.
    /// On Windows: the stub returns Ok; the full test fires on Unix CI.
    #[test]
    #[cfg(unix)]
    fn test_execute_sandboxed_before_hook_err_aborts_session() {
        use crate::hook_runtime;
        use crate::profile;
        use tempfile::TempDir;
        use std::os::unix::fs::PermissionsExt;

        let (_lock, _env, _home) = {
            let lock = match crate::test_env::ENV_LOCK.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            let home = TempDir::new().unwrap();
            let home_str = home.path().to_str().unwrap();
            let env = crate::test_env::EnvVarGuard::set_all(&[("HOME", home_str)]);
            (lock, env, home)
        };

        let dir = TempDir::new().unwrap();
        let script = dir.path().join("fail.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let hook = profile::SessionHook {
            script: script.clone(),
            timeout_secs: Some(5),
        };

        // D-03: before-hook Err propagates to the caller (session aborts).
        // The execute_sandboxed dispatch uses ? — this test verifies the Err is
        // returned by the hook, confirming the ? will propagate it.
        let result = hook_runtime::execute_before_hook(&hook, "d03-test", Path::new("/tmp"));
        assert!(
            result.is_err(),
            "D-03 fail-closed: before-hook exit 1 must return Err (session aborts)"
        );
    }

    #[test]
    fn recommended_builtin_profile_matches_known_agent_commands() {
        assert_eq!(
            recommended_builtin_profile(Path::new("/usr/local/bin/claude")),
            Some("claude-code")
        );
        assert_eq!(
            recommended_builtin_profile(Path::new("/usr/local/bin/codex")),
            Some("codex")
        );
    }

    #[test]
    fn recommended_builtin_profile_ignores_unknown_commands() {
        assert_eq!(recommended_builtin_profile(Path::new("/usr/bin/env")), None);
    }

    #[test]
    fn compute_executable_identity_hashes_canonical_binary_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("tool");
        fs::write(&binary, b"#!/bin/sh\necho hello\n").expect("write binary");

        let identity = compute_executable_identity(&binary).expect("compute identity");
        let expected = Sha256::digest(b"#!/bin/sh\necho hello\n");

        assert_eq!(
            identity.resolved_path,
            binary.canonicalize().expect("canonical")
        );
        assert_eq!(identity.sha256.as_bytes(), &<[u8; 32]>::from(expected));
    }
}
