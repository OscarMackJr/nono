use crate::cli::{RunArgs, SandboxArgs, ShellArgs, WrapArgs};
use crate::exec_strategy;
use crate::execution_runtime::execute_sandboxed;
use crate::launch_runtime::{
    load_configured_detach_sequence, load_configured_redaction_policy, prepare_run_launch_plan,
    resolve_requested_workdir, ExecutionFlags, LaunchPlan, SessionLaunchOptions,
};
use crate::output;
use crate::sandbox_prepare::{
    prepare_sandbox, print_allow_launch_services_warning, validate_external_proxy_bypass,
};
use crate::theme;
#[cfg(target_os = "windows")]
use nono::Sandbox;
use nono::{NonoError, Result};
use std::ffi::OsString;
use std::path::PathBuf;
use tracing::warn;

/// Check whether the loaded profile specifies a `binary` field that should be
/// honoured. Only user-authored profiles (user overrides or file-path based)
/// are allowed to set the target binary. Pack/registry and built-in profiles
/// are not trusted to dictate which binary runs.
fn resolve_profile_binary(
    profile_name: &str,
    loaded: &crate::profile::Profile,
    silent: bool,
) -> Option<String> {
    let binary = loaded.binary.as_ref()?;

    let is_user_profile = crate::profile::is_user_override(profile_name)
        || crate::profile::is_file_path_ref(profile_name);

    if !is_user_profile {
        if !silent {
            warn!(
                "Profile '{profile_name}' specifies binary '{binary}' but is not a user profile; ignoring",
            );
        }
        return None;
    }
    Some(binary.clone())
}

/// Resolve the program to execute: if the profile specifies a `binary` field
/// (and is a user profile), use it. If the CLI also provides a trailing
/// command, warn that the profile binary takes precedence.
fn resolve_program_from_profile_or_cli(
    cli_command: &[String],
    loaded_profile: Option<(&str, &crate::profile::Profile)>,
    silent: bool,
) -> Result<(OsString, Vec<OsString>)> {
    let profile_binary =
        loaded_profile.and_then(|(name, prof)| resolve_profile_binary(name, prof, silent));

    if let Some(binary) = profile_binary {
        if !cli_command.is_empty() && !silent {
            crate::output::print_warning(&format!(
                "Profile specifies binary '{}'; ignoring trailing command '{}'",
                binary,
                cli_command.join(" ")
            ));
        }
        let program = OsString::from(&binary);
        Ok((program, Vec::new()))
    } else if !cli_command.is_empty() {
        let mut iter = cli_command.iter();
        let program = OsString::from(iter.next().ok_or(NonoError::NoCommand)?);
        let cmd_args: Vec<OsString> = iter.map(OsString::from).collect();
        Ok((program, cmd_args))
    } else {
        Err(NonoError::NoCommand)
    }
}

pub(crate) fn run_sandbox(run_args: RunArgs, silent: bool) -> Result<()> {
    let command = run_args.command.clone();
    // Phase 37 D-12: capture --no-auto-pull for the dry-run path so the
    // dry-run preview behaves identically to a real run with respect to
    // profile resolution (i.e. dry-run must NOT trigger auto-pull either).
    let resolve_ctx = crate::profile::ResolveContext {
        no_auto_pull: run_args.profile_resolver.no_auto_pull,
    };

    // Phase 41 (REQ-CI-02): wire the --dangerous-force-wfp-ready flag to the
    // Windows WFP test-force-ready runtime setter. Previously the flag was
    // parsed by clap but never forwarded (the wiring was absent). The setter
    // checks for NONO_TEST_HARNESS at runtime so production builds are guarded.
    #[cfg(target_os = "windows")]
    if run_args.sandbox.dangerous_force_wfp_ready {
        exec_strategy::set_windows_wfp_test_force_ready(true);
    }

    // Load profile once and reuse for binary resolution and command_args.
    // Phase 37 D-12: resolve through `resolve_ctx` so `--no-auto-pull` is honored
    // on the real run path too. Using the context-free `load_profile` here made
    // this early resolution auto-pull a registry profile regardless of the flag
    // (the flag was only consulted on the dry-run branch), so a missing pack hit
    // the network instead of failing closed with the D-11 footer.
    let loaded_profile = match run_args.sandbox.profile.as_ref() {
        Some(name) => Some((
            name.clone(),
            crate::profile::load_profile_with_context(name, &resolve_ctx)?,
        )),
        None => None,
    };

    // Resolve the program: profile `binary` takes precedence over CLI trailing command.
    let (program, mut cmd_args) = resolve_program_from_profile_or_cli(
        &command,
        loaded_profile.as_ref().map(|(n, p)| (n.as_str(), p)),
        silent,
    )?;

    // Append profile command_args if applicable
    if let Some((_, ref loaded)) = loaded_profile {
        if !loaded.command_args.is_empty() {
            let all_packs_installed = loaded.packs.iter().all(|pack_ref| {
                let parts: Vec<&str> = pack_ref.splitn(2, '/').collect();
                if parts.len() != 2 {
                    return false;
                }
                crate::package::package_install_dir(parts[0], parts[1])
                    .map(|dir| dir.exists())
                    .unwrap_or(false)
            });

            if all_packs_installed || loaded.packs.is_empty() {
                let workdir = run_args
                    .sandbox
                    .workdir
                    .clone()
                    .or_else(|| std::env::current_dir().ok())
                    .unwrap_or_else(|| PathBuf::from("."));
                for arg in &loaded.command_args {
                    let expanded = crate::profile::expand_vars(arg, &workdir)?;
                    cmd_args.push(OsString::from(expanded));
                }
            }
        }
    }

    let args = run_args.sandbox.clone();

    if args.dry_run {
        let prepared =
            crate::sandbox_prepare::prepare_sandbox_with_context(&args, silent, &resolve_ctx)?;
        validate_external_proxy_bypass(&args, &prepared)?;
        if !prepared.secrets.is_empty() && !silent {
            eprintln!(
                "  Would inject {} credential(s) as environment variables",
                prepared.secrets.len()
            );
        }
        let redaction_policy = load_configured_redaction_policy()?;
        output::print_dry_run(&program, &cmd_args, &redaction_policy, silent);
        return Ok(());
    }

    // D-09 Phase 82: first-run provisioner (scratch + cert + NODE_EXTRA_CA_CERTS).
    // Non-fatal: errors and degraded sub-steps are recorded for `nono health`;
    // the run continues regardless.  Not invoked on dry_run (no host mutation).
    // The call is cfg-gated to Windows — Linux/macOS compile clean without it.
    #[cfg(target_os = "windows")]
    {
        match crate::provision_windows::provision_first_run() {
            Ok(crate::provision_windows::ProvisionOutcome::AlreadyProvisioned) => {
                tracing::debug!("provision_windows: already provisioned — skipping");
            }
            Ok(crate::provision_windows::ProvisionOutcome::Provisioned(ref status)) => {
                use crate::provision_windows::StepStatus;
                let any_degraded = matches!(status.scratch, StepStatus::Degraded(_))
                    || matches!(status.cert, StepStatus::Degraded(_))
                    || matches!(status.node_extra_ca_certs, StepStatus::Degraded(_));
                if any_degraded && !silent {
                    tracing::warn!(
                        "provision_windows: one or more provisioning sub-steps degraded; \
                         run `nono health` for details"
                    );
                }
            }
            Err(e) => {
                if !silent {
                    tracing::warn!(error = %e, "provision_windows: provisioner returned error (non-fatal); continuing run");
                }
            }
        }
    }

    let launch_plan = prepare_run_launch_plan(run_args, program, cmd_args, silent)?;
    execute_sandboxed(launch_plan)
}

pub(crate) fn run_shell(args: ShellArgs, silent: bool) -> Result<()> {
    #[cfg(target_os = "windows")]
    let shell_path = args.shell.unwrap_or_else(|| {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        let pwsh = std::path::PathBuf::from(&system_root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if pwsh.exists() {
            pwsh
        } else {
            std::path::PathBuf::from(&system_root)
                .join("System32")
                .join("cmd.exe")
        }
    });
    #[cfg(not(target_os = "windows"))]
    let shell_path = args
        .shell
        .or_else(|| {
            std::env::var("SHELL")
                .ok()
                .filter(|shell| !shell.is_empty())
                .map(std::path::PathBuf::from)
        })
        .unwrap_or_else(|| std::path::PathBuf::from("/bin/sh"));

    if args.sandbox.dry_run {
        let prepared = prepare_sandbox(&args.sandbox, silent)?;
        if !prepared.secrets.is_empty() && !silent {
            eprintln!(
                "  Would inject {} credential(s) as environment variables",
                prepared.secrets.len()
            );
        }
        let redaction_policy = load_configured_redaction_policy()?;
        output::print_dry_run(shell_path.as_os_str(), &[], &redaction_policy, silent);
        return Ok(());
    }

    let prepared = prepare_sandbox(&args.sandbox, silent)?;

    #[cfg(target_os = "windows")]
    Sandbox::validate_windows_preview_entry_point(
        nono::WindowsPreviewEntryPoint::Shell,
        &prepared.caps,
        &resolve_requested_workdir(args.sandbox.workdir.as_ref()),
        nono::WindowsPreviewContext {
            has_deny_override_policy: !prepared.bypass_protection_paths.is_empty(),
        },
    )?;

    if prepared.allow_launch_services_active {
        print_allow_launch_services_warning(silent);
    }

    if !silent {
        eprintln!("{}", {
            let theme = theme::current();
            theme::fg("Exit the shell with Ctrl-D or 'exit'.", theme.subtext)
        });
        eprintln!();
    }

    execute_sandboxed(LaunchPlan {
        program: shell_path.into_os_string(),
        cmd_args: vec![],
        caps: prepared.caps,
        loaded_secrets: prepared.secrets,
        // Plan 18.1-03 G-06: `nono shell` accepts a profile; carry it forward.
        loaded_profile: prepared.loaded_profile,
        flags: ExecutionFlags {
            workdir: resolve_requested_workdir(args.sandbox.workdir.as_ref()),
            no_diagnostics: true,
            interactive_shell: true,
            capability_elevation: prepared.capability_elevation,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: prepared.wsl2_proxy_policy,
            #[cfg(target_os = "linux")]
            af_unix_mediation: prepared.af_unix_mediation,
            bypass_protection_paths: prepared.bypass_protection_paths,
            ignored_denial_paths: prepared.ignored_denial_paths,
            suppressed_system_service_operations: prepared.suppressed_system_service_operations,
            allowed_env_vars: prepared.allowed_env_vars,
            denied_env_vars: prepared.denied_env_vars,
            set_vars: prepared.set_vars,
            startup_timeout_secs: args.startup_timeout_secs,
            redaction_policy: load_configured_redaction_policy()?,
            session: SessionLaunchOptions {
                session_name: args.name,
                detach_sequence: load_configured_detach_sequence()?,
                interactive_pty: true,
                ..SessionLaunchOptions::default()
            },
            ..ExecutionFlags::defaults(silent)?
        },
    })
}

pub(crate) fn run_wrap(wrap_args: WrapArgs, silent: bool) -> Result<()> {
    // Phase 37 D-12: capture `--no-auto-pull` BEFORE `wrap_args.sandbox` and
    // `wrap_args.profile_resolver` are consumed by the conversion below.
    let resolve_ctx = crate::profile::ResolveContext {
        no_auto_pull: wrap_args.profile_resolver.no_auto_pull,
    };
    let args: SandboxArgs = wrap_args.sandbox.into();
    let command = wrap_args.command;
    let no_diagnostics = wrap_args.no_diagnostics;

    if command.is_empty() {
        return Err(NonoError::NoCommand);
    }

    let mut command_iter = command.into_iter();
    let program = OsString::from(command_iter.next().ok_or(NonoError::NoCommand)?);
    let cmd_args: Vec<OsString> = command_iter.map(OsString::from).collect();

    if args.dry_run {
        let prepared =
            crate::sandbox_prepare::prepare_sandbox_with_context(&args, silent, &resolve_ctx)?;
        if !prepared.secrets.is_empty() && !silent {
            eprintln!(
                "  Would inject {} credential(s) as environment variables",
                prepared.secrets.len()
            );
        }
        let redaction_policy = load_configured_redaction_policy()?;
        output::print_dry_run(&program, &cmd_args, &redaction_policy, silent);
        return Ok(());
    }

    let prepared =
        crate::sandbox_prepare::prepare_sandbox_with_context(&args, silent, &resolve_ctx)?;

    #[cfg(target_os = "windows")]
    Sandbox::validate_windows_preview_entry_point(
        nono::WindowsPreviewEntryPoint::Wrap,
        &prepared.caps,
        &resolve_requested_workdir(args.workdir.as_ref()),
        nono::WindowsPreviewContext {
            has_deny_override_policy: !prepared.bypass_protection_paths.is_empty(),
        },
    )?;

    if prepared.upstream_proxy.is_some()
        || matches!(
            prepared.caps.network_mode(),
            nono::NetworkMode::ProxyOnly { .. }
        )
    {
        return Err(NonoError::ConfigParse(
            "nono wrap does not support proxy mode (activated by profile network settings). \
             Use `nono run` instead."
                .to_string(),
        ));
    }

    #[cfg(target_os = "linux")]
    if prepared.af_unix_mediation.is_pathname() {
        return Err(NonoError::ConfigParse(
            "nono wrap does not support linux.af_unix_mediation = \"pathname\" because direct \
             exec cannot run the seccomp supervisor. Use `nono run` instead."
                .to_string(),
        ));
    }

    if prepared.allow_launch_services_active {
        print_allow_launch_services_warning(silent);
    }

    execute_sandboxed(LaunchPlan {
        program,
        cmd_args,
        caps: prepared.caps,
        loaded_secrets: prepared.secrets,
        // Plan 18.1-03 G-06: `nono wrap` is Direct-strategy only (no
        // supervisor, no capability pipe). The field is carried for
        // struct-literal completeness; the Direct path never consults it.
        loaded_profile: prepared.loaded_profile,
        flags: ExecutionFlags {
            strategy: exec_strategy::ExecStrategy::Direct,
            workdir: resolve_requested_workdir(args.workdir.as_ref()),
            no_diagnostics,
            bypass_protection_paths: prepared.bypass_protection_paths,
            ignored_denial_paths: prepared.ignored_denial_paths,
            suppressed_system_service_operations: prepared.suppressed_system_service_operations,
            allowed_env_vars: prepared.allowed_env_vars,
            denied_env_vars: prepared.denied_env_vars,
            set_vars: prepared.set_vars,
            ..ExecutionFlags::defaults(silent)?
        },
    })
}
