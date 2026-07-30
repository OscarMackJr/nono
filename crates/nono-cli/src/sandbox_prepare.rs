use crate::capability_ext::{self, CapabilitySetExt};
use crate::cli::SandboxArgs;
#[cfg(target_os = "linux")]
use crate::config;
use crate::credential_runtime::load_env_credentials;
use crate::network_policy;
use crate::profile;
use crate::profile::WorkdirAccess;
use crate::{output, policy, protected_paths, sandbox_state};
use colored::Colorize;
use nono::{AccessMode, CapabilitySet, FsCapability, NonoError, Result, Sandbox};
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use tracing::info;

fn print_allow_domain_port_warnings(entries: &[String], context: &str, silent: bool) {
    if silent {
        return;
    }

    for warning in network_policy::collect_allow_domain_port_warnings(entries, context) {
        output::print_warning(&warning);
    }
}

fn collect_missing_cli_requested_paths(args: &SandboxArgs) -> Vec<String> {
    let mut missing = Vec::new();

    for path in &args.allow {
        if !path.exists() {
            missing.push(format!("--allow {}", path.display()));
        }
    }
    for path in &args.read {
        if !path.exists() {
            missing.push(format!("--read {}", path.display()));
        }
    }
    for path in &args.write {
        if !path.exists() {
            missing.push(format!("--write {}", path.display()));
        }
    }
    for path in &args.allow_file {
        if !path.exists() && !capability_ext::retains_missing_exact_file_grants() {
            missing.push(format!("--allow-file {}", path.display()));
        }
    }
    for path in &args.read_file {
        if !path.exists() && !capability_ext::retains_missing_exact_file_grants() {
            missing.push(format!("--read-file {}", path.display()));
        }
    }
    for path in &args.write_file {
        if !path.exists() && !capability_ext::retains_missing_exact_file_grants() {
            missing.push(format!("--write-file {}", path.display()));
        }
    }

    missing
}

/// Result of sandbox preparation.
pub(crate) struct PreparedSandbox {
    pub(crate) caps: CapabilitySet,
    pub(crate) secrets: Vec<nono::LoadedSecret>,
    pub(crate) rollback_exclude_patterns: Vec<String>,
    pub(crate) rollback_exclude_globs: Vec<String>,
    pub(crate) network_profile: Option<String>,
    pub(crate) allow_domain: Vec<crate::profile::AllowDomainEntry>,
    /// Raw `deny_domain` entries from the loaded profile (manifest path has
    /// none). CLI `--deny-domain` is merged in later via
    /// `proxy_runtime::resolve_effective_proxy_settings`.
    pub(crate) deny_domain: Vec<String>,
    pub(crate) credentials: Vec<String>,
    pub(crate) custom_credentials: HashMap<String, profile::CustomCredentialDef>,
    pub(crate) upstream_proxy: Option<String>,
    pub(crate) upstream_bypass: Vec<String>,
    pub(crate) listen_ports: Vec<u16>,
    pub(crate) capability_elevation: bool,
    #[cfg(target_os = "linux")]
    pub(crate) wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy,
    #[cfg(target_os = "linux")]
    pub(crate) af_unix_mediation: crate::profile::LinuxAfUnixMediation,
    pub(crate) allow_launch_services_active: bool,
    pub(crate) open_url_origins: Vec<String>,
    pub(crate) open_url_allow_localhost: bool,
    pub(crate) bypass_protection_paths: Vec<PathBuf>,
    pub(crate) ignored_denial_paths: Vec<PathBuf>,
    /// cc21229f (C3): non-filesystem sandbox operations suppressed from
    /// the diagnostic footer. Sandbox enforcement is unchanged; this only
    /// controls reporting. Required by Plan 70-03 (C2 prerequisite).
    pub(crate) suppressed_system_service_operations: Vec<String>,
    /// Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`):
    /// allow-list of environment variable names from the loaded profile's
    /// `environment.allow_vars` block. See [`crate::profile_runtime::PreparedProfile::allowed_env_vars`]
    /// for semantics.
    pub(crate) allowed_env_vars: Option<Vec<String>>,
    /// Plan 34-08a Task 4 (D-20 replay of v0.52.0 `3657c935`): operator-
    /// controlled deny-list of environment variable names from the loaded
    /// profile's `environment.deny_vars` block.
    pub(crate) denied_env_vars: Option<Vec<String>>,
    /// Expanded `environment.set_vars` (key, expanded-value), `None` if absent.
    pub(crate) set_vars: Option<Vec<(String, String)>>,
    /// True when the profile's `network.block` is set. The CLI `--block-net`
    /// flag is read directly from `SandboxArgs` at proxy-launch time, so only
    /// the profile's contribution needs to be carried through.
    /// Fork deviation from 72bcfd66: renamed network_block_requested → profile_network_block
    pub(crate) profile_network_block: bool,
    /// Plan 18.1-03 G-06: the loaded profile (if any) is preserved past
    /// profile destructuring so its `capabilities.aipc` widening can be
    /// resolved at Windows supervisor construction time via
    /// `Profile::resolve_aipc_allowlist`. `None` when the run has no
    /// `--profile` argument; widening falls back to hard-coded defaults.
    pub(crate) loaded_profile: Option<profile::Profile>,
    /// Phase 58: session lifecycle hooks carried forward from the loaded profile
    /// for dispatch in `execution_runtime::execute_sandboxed`. Cross-platform;
    /// runtime execution is platform-gated in `hook_runtime.rs` (Unix) /
    /// `hook_runtime_windows.rs` (Windows).
    pub(crate) session_hooks: profile::SessionHooks,
    /// True when the profile or CLI requested HTTP/2 for upstream proxy connections.
    ///
    /// Set from `--allow-http2` OR profile `network.allow_http2`. Carried into
    /// `ProxyLaunchOptions.enable_h2` via `prepare_proxy_launch_options`.
    /// Upstream cdeeb5b9 (#983): absorbed.
    pub(crate) allow_http2_requested: bool,
}

fn finalize_prepared_sandbox(
    prepared: PreparedSandbox,
    blocked_grants: &[(PathBuf, Option<String>)],
    args: &SandboxArgs,
    silent: bool,
) -> Result<PreparedSandbox> {
    output::print_skipped_requested_paths(&collect_missing_cli_requested_paths(args), silent);
    // Compute proxy_pending before print_capabilities so the display shows
    // yellow "proxy" when AllowAll caps but a proxy will start, or when
    // Blocked caps but proxy flags override it (strict_filter mode).
    // Per upstream 72bcfd66 (#1225) + d457ecc3 (#1263): delegate to
    // has_proxy_intent() so custom_credentials are also counted.
    let proxy_intent = has_proxy_intent(args, &prepared);
    let block_wins = args.block_net || (prepared.profile_network_block && !proxy_intent);
    let proxy_pending = !block_wins && !args.allow_net && proxy_intent;
    output::print_capabilities(
        &prepared.caps,
        blocked_grants,
        args.verbose,
        silent,
        proxy_pending,
    );

    if let Some(ref profile_name) = args.profile {
        crate::pack_update_hint::show_pack_update_hints(profile_name, silent);
    }

    #[cfg(target_os = "linux")]
    output::print_abi_info(silent);
    #[cfg(target_os = "linux")]
    output::print_landlock_scope_policy(&prepared.caps, args.verbose, silent);

    if !Sandbox::is_supported() {
        return Err(NonoError::SandboxInit(Sandbox::support_info().details));
    }

    info!("{}", Sandbox::support_info().details);

    Ok(prepared)
}

/// Returns true if any CLI flag or profile field requires the proxy to run.
fn has_proxy_intent(args: &SandboxArgs, prepared: &PreparedSandbox) -> bool {
    args.has_proxy_flags()
        || !prepared.credentials.is_empty()
        || !prepared.custom_credentials.is_empty()
        || prepared.network_profile.is_some()
        || !prepared.allow_domain.is_empty()
        || !prepared.deny_domain.is_empty()
        || prepared.upstream_proxy.is_some()
}

pub(crate) fn validate_external_proxy_bypass(
    args: &SandboxArgs,
    prepared: &PreparedSandbox,
) -> Result<()> {
    let has_bypass = !args.external_proxy_bypass.is_empty() || !prepared.upstream_bypass.is_empty();
    let has_external_proxy = args.external_proxy.is_some() || prepared.upstream_proxy.is_some();

    if has_bypass && !has_external_proxy {
        return Err(NonoError::ConfigParse(
            "--upstream-bypass requires --upstream-proxy \
             (or upstream_proxy in profile network config)"
                .to_string(),
        ));
    }
    Ok(())
}

/// Validate that `--block-net` is not combined with flags that imply proxy
/// mode, and that `--allow-endpoint` always has a matching credential.
///
/// These combinations are logically contradictory: `--block-net` prevents all
/// outbound traffic, so proxy-mode flags would be silently ignored.
pub(crate) fn validate_block_net_conflicts(
    args: &SandboxArgs,
    prepared: &PreparedSandbox,
) -> Result<()> {
    let block_net = args.block_net || prepared.profile_network_block;

    if block_net {
        // Credential injection requires the proxy to be reachable.
        let has_credentials = !args.proxy_credential.is_empty() || !prepared.credentials.is_empty();
        if has_credentials {
            return Err(NonoError::ConfigParse(
                "--block-net and --credential are contradictory: \
                 credential injection requires the proxy to be reachable"
                    .to_string(),
            ));
        }

        // A network profile configures proxy-mode filtering.
        let has_network_profile =
            args.network_profile.is_some() || prepared.network_profile.is_some();
        if has_network_profile {
            return Err(NonoError::ConfigParse(
                "--block-net and --network-profile are contradictory: \
                 a network profile requires proxy mode"
                    .to_string(),
            ));
        }

        // --allow-domain implies proxy-filtered mode.
        let has_allow_domain = !args.allow_proxy.is_empty() || !prepared.allow_domain.is_empty();
        if has_allow_domain {
            return Err(NonoError::ConfigParse(
                "--block-net and --allow-domain are contradictory: \
                 domain filtering requires proxy mode"
                    .to_string(),
            ));
        }
    }

    // --allow-endpoint without any credential is a no-op (and almost certainly
    // a user error: the service name doesn't match any loaded credential).
    if !args.allow_endpoint.is_empty() {
        let has_credentials = !args.proxy_credential.is_empty()
            || !prepared.credentials.is_empty()
            || !prepared.custom_credentials.is_empty();
        if !has_credentials {
            return Err(NonoError::ConfigParse(
                "--allow-endpoint requires at least one --credential \
                 (no credential loaded for the named service)"
                    .to_string(),
            ));
        }
    }

    // --proxy-port without any proxy-triggering flag is almost certainly a
    // mistake: the port would be set but the proxy would never start.
    if args.proxy_port.is_some() && !has_proxy_intent(args, prepared) {
        return Err(NonoError::ConfigParse(
            "--proxy-port has no effect without a proxy-mode flag \
             (e.g. --credential, --network-profile, --allow-domain)"
                .to_string(),
        ));
    }

    Ok(())
}

/// Validate that `deny_domain`/`--deny-domain` is never used without at
/// least one `allow_domain`/`--allow-domain` entry (D-04/D-05).
///
/// Mirrors `validate_block_net_conflicts`'s shape exactly — a CLI-side,
/// fail-closed guard called from both `nono run` entry points
/// (`command_runtime.rs`'s dry-run branch and `launch_runtime.rs`'s real
/// launch path). **A guard on only one of the two call sites is a bypass,
/// not a guard** (D-04).
///
/// Per ADR-108 / CONTEXT.md D-05, a deny-only configuration is a HARD ERROR
/// at parse time — never a silent fallback to strict/allow-all. Upstream's
/// `deny_domain` auto-activates the proxy with default-allow semantics
/// ("allow everything except these domains"); this fork deliberately
/// rejects that trigger. A user who reaches this error almost certainly
/// expected upstream's behavior and needs to be told explicitly that this
/// fork requires at least one `allow_domain` entry (or `--block-net` if the
/// actual intent was "deny everything").
pub(crate) fn validate_deny_domain_requires_allow_domain(
    args: &SandboxArgs,
    prepared: &PreparedSandbox,
) -> Result<()> {
    let has_deny = !args.deny_proxy.is_empty() || !prepared.deny_domain.is_empty();
    let has_allow = !args.allow_proxy.is_empty() || !prepared.allow_domain.is_empty();

    if has_deny && !has_allow {
        return Err(NonoError::ConfigParse(
            "deny_domain / --deny-domain requires at least one allow_domain / \
             --allow-domain entry. This fork deliberately diverges from upstream \
             nono here: upstream's deny_domain silently auto-activates the proxy \
             with default-allow semantics (\"allow everything except these \
             domains\"), which would leave every host you did not think to \
             name reachable. Add at least one --allow-domain (or profile \
             allow_domain) entry, or use --block-net instead if the intent \
             was to deny all outbound network access."
                .to_string(),
        ));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn maybe_enable_macos_launch_services(
    caps: &mut CapabilitySet,
    cli_requested: bool,
    profile_allowed: bool,
    open_url_origins: &[String],
    open_url_allow_localhost: bool,
) -> Result<bool> {
    if !cli_requested {
        return Ok(false);
    }

    if !profile_allowed {
        return Err(NonoError::ConfigParse(
            "--allow-launch-services requires a profile that opts into allow_launch_services"
                .to_string(),
        ));
    }

    if open_url_origins.is_empty() && !open_url_allow_localhost {
        return Err(NonoError::ConfigParse(
            "--allow-launch-services requires the selected profile to configure open_urls"
                .to_string(),
        ));
    }

    caps.add_platform_rule("(allow lsopen)")?;
    tracing::debug!(
        "--allow-launch-services enabled: allowing direct LaunchServices opens on macOS"
    );
    Ok(true)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn maybe_enable_macos_launch_services(
    _caps: &mut CapabilitySet,
    cli_requested: bool,
    _profile_allowed: bool,
    _open_url_origins: &[String],
    _open_url_allow_localhost: bool,
) -> Result<bool> {
    if cli_requested {
        return Err(NonoError::ConfigParse(
            "--allow-launch-services is only supported on macOS".to_string(),
        ));
    }
    Ok(false)
}

pub(crate) fn print_allow_launch_services_warning(silent: bool) {
    if silent {
        return;
    }

    eprintln!(
        "  {}",
        "WARNING: --allow-launch-services permits the sandboxed process to ask macOS \
         LaunchServices to open URLs, files, or apps."
            .yellow()
    );
    eprintln!("  Use this only for temporary login/setup flows, then exit and rerun without it.");
    eprintln!("  Prefer using it from a trusted directory, not inside an untrusted project.");
}

/// Resolve the working directory used for sandbox preparation.
///
/// Prefers an explicit `--workspace` (D-06: single source of truth — the
/// workspace IS the child CWD; takes priority over `--workdir`), then
/// `--workdir`, then the process's current directory, falling back to `.` as a
/// last resort. The final `unwrap_or_else` fallback is the pre-existing
/// production default (non-security-critical, not a security-config load
/// failure), so the `clippy::unwrap_used` rule does not apply to it
/// (PATTERNS.md annotation).
fn resolved_workdir(args: &SandboxArgs) -> PathBuf {
    args.workspace
        .clone()
        .or_else(|| args.workdir.clone())
        .or_else(|| {
            // $PWD preserves the symlink path; current_dir() resolves it. Prefer
            // $PWD so the CWD capability covers the symlink form (e.g. /tmp rather
            // than /private/tmp) when `nono run` is invoked without --workdir.
            std::env::var("PWD").ok().map(PathBuf::from)
        })
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Phase 37 D-12: legacy entry point — thin wrapper that supplies the default
/// [`crate::profile::ResolveContext`]. Sites outside `nono run` / `nono wrap`
/// (e.g. `nono shell`, dry-run paths, internal tooling) call this and inherit
/// pre-Phase-37 behavior (auto-pull enabled). The `nono run` and `nono wrap`
/// handlers MUST use [`prepare_sandbox_with_context`] so the `--no-auto-pull`
/// flag and `NONO_NO_AUTO_PULL=1` env var are honored.
pub(crate) fn prepare_sandbox(args: &SandboxArgs, silent: bool) -> Result<PreparedSandbox> {
    prepare_sandbox_with_context(args, silent, &crate::profile::ResolveContext::default())
}

pub(crate) fn prepare_sandbox_with_context(
    args: &SandboxArgs,
    silent: bool,
    resolve_ctx: &crate::profile::ResolveContext,
) -> Result<PreparedSandbox> {
    sandbox_state::cleanup_stale_state_files();

    let workdir = resolved_workdir(args);

    if let Some(ref config_path) = args.config {
        let json = std::fs::read_to_string(config_path).map_err(|e| {
            NonoError::ConfigParse(format!(
                "failed to read manifest file '{}': {e}",
                config_path.display()
            ))
        })?;
        let mut manifest = nono::manifest::CapabilityManifest::from_json(&json)?;
        manifest.validate()?;

        if let Some(ref mut fs) = manifest.filesystem {
            for grant in &mut fs.grants {
                let expanded = profile::expand_vars(grant.path.as_str(), &workdir)?;
                grant.path = expanded
                    .to_string_lossy()
                    .parse()
                    .map_err(|e| NonoError::ConfigParse(format!("invalid path: {e}")))?;
            }
            for deny in &mut fs.deny {
                let expanded = profile::expand_vars(deny.path.as_str(), &workdir)?;
                deny.path = expanded
                    .to_string_lossy()
                    .parse()
                    .map_err(|e| NonoError::ConfigParse(format!("invalid path: {e}")))?;
            }
        }

        let caps = CapabilitySet::try_from(&manifest)?;
        let protected_roots = protected_paths::ProtectedRoots::from_defaults()?;
        protected_paths::validate_caps_against_protected_roots(&caps, protected_roots.as_paths())?;

        let (rollback_exclude_patterns, rollback_exclude_globs) =
            if let Some(ref rb) = manifest.rollback {
                (rb.exclude_patterns.clone(), rb.exclude_globs.clone())
            } else {
                (Vec::new(), Vec::new())
            };

        let allow_domain_strings: Vec<String> = manifest
            .network
            .as_ref()
            .map(|network| network.allow_domains.clone())
            .unwrap_or_default();
        print_allow_domain_port_warnings(&allow_domain_strings, "manifest allow_domain", silent);
        let allow_domain: Vec<crate::profile::AllowDomainEntry> = allow_domain_strings
            .into_iter()
            .map(crate::profile::AllowDomainEntry::Plain)
            .collect();
        let credentials = manifest
            .credentials
            .iter()
            .map(|credential| credential.name.as_str().to_string())
            .collect();

        return finalize_prepared_sandbox(
            PreparedSandbox {
                caps,
                secrets: Vec::new(),
                rollback_exclude_patterns,
                rollback_exclude_globs,
                network_profile: None,
                allow_domain,
                // Manifest schema has no deny_domain key today — manifest
                // path always has an empty deny list.
                deny_domain: Vec::new(),
                credentials,
                custom_credentials: HashMap::new(),
                upstream_proxy: None,
                upstream_bypass: Vec::new(),
                listen_ports: Vec::new(),
                capability_elevation: false,
                #[cfg(target_os = "linux")]
                wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::default(),
                #[cfg(target_os = "linux")]
                af_unix_mediation: crate::profile::LinuxAfUnixMediation::default(),
                allow_launch_services_active: false,
                open_url_origins: Vec::new(),
                open_url_allow_localhost: false,
                bypass_protection_paths: Vec::new(),
                ignored_denial_paths: Vec::new(),
                suppressed_system_service_operations: Vec::new(),
                // Plan 34-08a Task 3 (D-20 replay of `1b412a7`): manifest path
                // has no loaded Profile and no env-filter block.
                allowed_env_vars: None,
                // Plan 34-08a Task 4 (D-20 replay of v0.52.0 `3657c935`):
                // deny_vars also unset on the manifest path.
                denied_env_vars: None,
                set_vars: None,
                // Manifest path has no profile, so profile_network_block is false.
                // The CLI --block-net flag is read directly at proxy-launch time.
                profile_network_block: false,
                // Plan 18.1-03 G-06: manifest path has no loaded Profile —
                // AIPC widening defaults to hard-coded supervisor allowlist.
                loaded_profile: None,
                // Phase 58: manifest path has no loaded Profile — no session
                // hooks configured.
                session_hooks: profile::SessionHooks::default(),
                // Upstream cdeeb5b9 (#983): manifest path has no profile —
                // HTTP/2 defaults off.
                allow_http2_requested: false,
            },
            &[],
            args,
            silent,
        );
    }

    let prepared_profile =
        crate::profile_runtime::prepare_profile_with_context(args, silent, &workdir, resolve_ctx)?;
    let crate::profile_runtime::PreparedProfile {
        loaded_profile,
        capability_elevation,
        #[cfg(target_os = "linux")]
        wsl2_proxy_policy,
        #[cfg(target_os = "linux")]
        af_unix_mediation,
        workdir_access: profile_workdir_access,
        rollback_exclude_patterns: profile_rollback_patterns,
        rollback_exclude_globs: profile_rollback_globs,
        network_profile: profile_network_profile,
        allow_domain: profile_allow_domain,
        deny_domain: profile_deny_domain,
        credentials: profile_credentials,
        custom_credentials: profile_custom_credentials,
        upstream_proxy: profile_upstream_proxy,
        upstream_bypass: profile_upstream_bypass,
        listen_ports: profile_listen_ports,
        open_url_origins,
        open_url_allow_localhost,
        allow_launch_services: profile_allow_launch_services,
        bypass_protection_paths,
        ignored_denial_paths,
        suppressed_system_service_operations,
        allowed_env_vars: profile_allowed_env_vars,
        denied_env_vars: profile_denied_env_vars,
        set_vars: profile_set_vars,
    } = prepared_profile;

    // OAUTH-03 (Plan 22-04): warn when allow_domain entries include `:port`
    // suffixes — nono now ignores ports in allow-domain rules and only applies
    // hostname filtering through the proxy. Cherry-picked from upstream
    // 005579a9 + 60ad1eb3 (DRY helper). The deprecation-warning call from
    // upstream's loaded_profile.as_ref() block is intentionally omitted —
    // fork has no `command_blocking_deprecation` flow at this site
    // (deprecation warnings emit at profile-load time instead).
    let profile_allow_domain_strings: Vec<String> = profile_allow_domain
        .iter()
        .map(|e| e.domain().to_string())
        .collect();
    print_allow_domain_port_warnings(
        &profile_allow_domain_strings,
        "profile allow_domain",
        silent,
    );
    print_allow_domain_port_warnings(&args.allow_proxy, "--allow-domain", silent);
    print_allow_domain_port_warnings(&profile_deny_domain, "profile deny_domain", silent);
    print_allow_domain_port_warnings(&args.deny_proxy, "--deny-domain", silent);

    #[cfg(target_os = "linux")]
    if args.profile.as_deref() == Some("claude-code") {
        let home = config::validated_home()?;
        let home_path = std::path::Path::new(&home);

        let precreate = |path: &std::path::Path, is_dir: bool| {
            let result = if is_dir {
                std::fs::create_dir_all(path)
            } else {
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .mode(0o600)
                    .open(path)
                    .map(|_| ())
            };
            if let Err(e) = result {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    // CR-04 fix (REVIEW.md): fully-qualified `tracing::warn!`
                    // for consistency with line 377 below. The bare `warn!`
                    // macro is not in scope (this file only imports
                    // `tracing::info`), so the previous form would fail to
                    // compile on Linux (block is `#[cfg(target_os = "linux")]`,
                    // invisible from Windows-host clippy).
                    tracing::warn!("Failed to pre-create {}: {}", path.display(), e);
                }
            }
        };

        precreate(&home_path.join(".claude.json.lock"), false);
        precreate(&home_path.join(".cache/claude-cli-nodejs"), true);
    }

    let prepared = if let Some(ref profile) = loaded_profile {
        CapabilitySet::from_profile(profile, &workdir, args)?
    } else {
        CapabilitySet::from_args(args)?
    };
    let mut caps = prepared.caps;
    let needs_unlink_overrides = prepared.needs_unlink_overrides;
    // Resolved policy denies (groups + profile add_deny_access). Used to
    // re-run validate_deny_overlaps after CWD/pack grants are added below,
    // because Landlock cannot enforce a deny that lives under a later allow.
    let prepared_deny_paths = prepared.deny_paths;
    // User grants silently blocked by deny groups (macOS); folded into the
    // capability summary instead of emitting one warning per path.
    let blocked_grants = prepared.blocked_grants;

    // PROF-01 (Phase 22): apply raw Seatbelt rules from the profile (macOS only).
    // On Linux/Windows the field deserializes but is intentionally ignored —
    // there is no equivalent escape hatch on those platforms (REQ-PROF-01).
    #[cfg(target_os = "macos")]
    if let Some(ref profile) = loaded_profile {
        if !profile.unsafe_macos_seatbelt_rules.is_empty() {
            tracing::warn!(
                "Profile uses {} raw Seatbelt rule(s) via unsafe_macos_seatbelt_rules — review carefully",
                profile.unsafe_macos_seatbelt_rules.len()
            );
            for rule in &profile.unsafe_macos_seatbelt_rules {
                caps.add_platform_rule(rule).map_err(|e| {
                    NonoError::ConfigParse(format!(
                        "unsafe_macos_seatbelt_rules: invalid rule {rule:?}: {e}"
                    ))
                })?;
            }
        }
    }

    let allow_launch_services_active = maybe_enable_macos_launch_services(
        &mut caps,
        args.allow_launch_services,
        profile_allow_launch_services,
        &open_url_origins,
        open_url_allow_localhost,
    )?;

    // D-06 workspace grant: when --workspace is explicitly declared, it IS the
    // writable engine working directory — grant read+write unconditionally (no
    // prompt). This is the "single source of truth" for both child CWD and the
    // writable grant (T-71-09). The grant is expressed with the canonicalized
    // absolute path (footgun #1: component-wise, never string starts_with).
    if let Some(ref ws) = args.workspace {
        let ws_canonical = ws
            .canonicalize()
            .map_err(|e| NonoError::PathCanonicalization {
                path: ws.clone(),
                source: e,
            })?;
        if !caps.path_covered_with_access(&ws_canonical, AccessMode::ReadWrite) {
            info!(
                "Auto-granting workspace read+write access (--workspace): {}",
                ws_canonical.display()
            );
            let cap = FsCapability::new_dir(ws_canonical, AccessMode::ReadWrite)?;
            caps.add_fs(cap);
        }
    }

    let cwd_access = if let Some(ref access) = profile_workdir_access {
        match access {
            WorkdirAccess::Read => Some(AccessMode::Read),
            WorkdirAccess::Write => Some(AccessMode::Write),
            WorkdirAccess::ReadWrite => Some(AccessMode::ReadWrite),
            WorkdirAccess::None => None,
        }
    } else {
        Some(AccessMode::Read)
    };

    if let Some(access) = cwd_access {
        let cwd_canonical =
            workdir
                .canonicalize()
                .map_err(|e| NonoError::PathCanonicalization {
                    path: workdir.clone(),
                    source: e,
                })?;

        if !caps.path_covered_with_access(&cwd_canonical, access) {
            if args.allow_cwd {
                info!("Auto-including CWD with {} access (--allow-cwd)", access);
                let cap = FsCapability::new_dir(cwd_canonical.clone(), access)?;
                caps.add_fs(cap);
                #[cfg(target_os = "macos")]
                {
                    // When CWD is reached via a symlink (e.g. /tmp -> /private/tmp),
                    // the canonical path differs. Emit the symlink form as a second
                    // capability so Seatbelt allows traversal via the symlink path too.
                    if workdir != cwd_canonical {
                        let symlink_cap = FsCapability::new_dir(workdir.clone(), access)?;
                        caps.add_fs(symlink_cap);
                    }
                }
            } else if silent {
                return Err(NonoError::CwdPromptRequired);
            } else {
                let confirmed = output::prompt_cwd_sharing(&cwd_canonical, &access)?;
                if confirmed {
                    let cap = FsCapability::new_dir(cwd_canonical.clone(), access)?;
                    caps.add_fs(cap);
                    #[cfg(target_os = "macos")]
                    {
                        // Symlink-form CWD grant (see --allow-cwd branch above); only
                        // emitted when the user confirmed sharing, so a declined prompt
                        // grants nothing (the upstream call-site had no decline branch).
                        if workdir != cwd_canonical {
                            let symlink_cap = FsCapability::new_dir(workdir.clone(), access)?;
                            caps.add_fs(symlink_cap);
                        }
                    }
                } else {
                    info!("User declined CWD sharing. Continuing without automatic CWD access.");
                }
            }
            caps.deduplicate();
        }
    }

    // Re-validate against the full deny set (groups + profile add_deny_access)
    // now that CWD, pack dirs, and any GPU/launch-services grants have been
    // added on top of the caps produced by from_profile/from_args. The initial
    // validation inside finalize_caps did not see those later grants, so a
    // profile deny that lands under e.g. --allow-cwd would otherwise be a
    // silent no-op on Linux (Landlock cannot deny under an allow).
    policy::validate_deny_overlaps(&prepared_deny_paths, &caps)?;
    let protected_roots = protected_paths::ProtectedRoots::from_defaults()?;
    protected_paths::validate_caps_against_protected_roots(&caps, protected_roots.as_paths())?;

    if needs_unlink_overrides {
        policy::apply_unlink_overrides(&mut caps);
    }

    if !caps.has_fs() && caps.is_network_blocked() {
        return Err(NonoError::NoCapabilities);
    }

    // Capture the profile's `network.block` intent before `loaded_profile`
    // is consumed below.
    let profile_network_block = loaded_profile
        .as_ref()
        .map(|p| p.network.block)
        .unwrap_or(false);
    // Note: the CLI `args.block_net` is now read directly at proxy-launch time
    // via prepare_proxy_launch_options; only the profile's block contribution
    // is carried in PreparedSandbox.profile_network_block.

    // Plan 18.1-03 G-06: clone env_credentials.mappings out by reference so
    // `loaded_profile` stays owned and can be preserved in `PreparedSandbox`
    // for downstream AIPC-allowlist resolution at Windows supervisor
    // construction time. The previous shape MOVED `loaded_profile` out here;
    // G-06 wiring needs the profile to survive past this point.
    let profile_secrets = loaded_profile
        .as_ref()
        .map(|profile| profile.env_credentials.mappings.clone())
        .unwrap_or_default();
    let loaded_secrets = load_env_credentials(args, &profile_secrets, silent)?;
    // Phase 58: extract session_hooks before `loaded_profile` is moved into
    // the PreparedSandbox struct literal below. Follows the profile_secrets
    // extraction pattern above (clone before move).
    let profile_session_hooks = loaded_profile
        .as_ref()
        .map(|p| p.session_hooks.clone())
        .unwrap_or_default();
    // Upstream cdeeb5b9 (#983): carry profile's allow_http2 intent into
    // PreparedSandbox so prepare_proxy_launch_options can merge it with
    // the CLI --allow-http2 flag.
    let profile_allow_http2 = loaded_profile
        .as_ref()
        .map(|p| p.network.allow_http2)
        .unwrap_or(false);
    let allow_http2_requested = args.allow_http2 || profile_allow_http2;

    finalize_prepared_sandbox(
        PreparedSandbox {
            caps,
            secrets: loaded_secrets,
            rollback_exclude_patterns: profile_rollback_patterns,
            rollback_exclude_globs: profile_rollback_globs,
            network_profile: profile_network_profile,
            allow_domain: profile_allow_domain,
            deny_domain: profile_deny_domain,
            credentials: profile_credentials,
            custom_credentials: profile_custom_credentials,
            upstream_proxy: profile_upstream_proxy,
            upstream_bypass: profile_upstream_bypass,
            listen_ports: profile_listen_ports,
            capability_elevation,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy,
            #[cfg(target_os = "linux")]
            af_unix_mediation,
            allow_launch_services_active,
            open_url_origins,
            open_url_allow_localhost,
            bypass_protection_paths,
            ignored_denial_paths,
            suppressed_system_service_operations,
            // Plan 34-08a Task 3 (D-20 replay of `1b412a7`): forward the
            // env-filter allow-list from PreparedProfile to PreparedSandbox.
            allowed_env_vars: profile_allowed_env_vars,
            // Plan 34-08a Task 4 (D-20 replay of v0.52.0 `3657c935`):
            // forward the env-filter deny-list.
            denied_env_vars: profile_denied_env_vars,
            set_vars: profile_set_vars,
            profile_network_block,
            // Plan 18.1-03 G-06: preserve the loaded profile so
            // `Profile::resolve_aipc_allowlist` can be consulted at the
            // Windows supervisor construction site.
            loaded_profile,
            // Phase 58: carry session_hooks from the loaded profile into
            // PreparedSandbox. Follows the allowed_env_vars pattern above.
            session_hooks: profile_session_hooks,
            // Upstream cdeeb5b9 (#983): carry HTTP/2 intent.
            allow_http2_requested,
        },
        &blocked_grants,
        args,
        silent,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(target_os = "macos")]
    #[test]
    fn missing_exact_file_cli_grants_are_not_reported_as_skipped() {
        let dir = tempdir().expect("tmpdir");
        let args = SandboxArgs {
            allow_file: vec![dir.path().join("future.lock")],
            ..SandboxArgs::default()
        };

        assert!(
            collect_missing_cli_requested_paths(&args).is_empty(),
            "macOS exact-file grants should not be reported as skipped when the file is absent"
        );
    }

    #[test]
    fn missing_directory_cli_grants_are_reported_as_skipped() {
        let dir = tempdir().expect("tmpdir");
        let args = SandboxArgs {
            allow: vec![dir.path().join("future-dir")],
            ..SandboxArgs::default()
        };

        assert_eq!(
            collect_missing_cli_requested_paths(&args),
            vec![format!(
                "--allow {}",
                dir.path().join("future-dir").display()
            )]
        );
    }

    fn empty_prepared() -> PreparedSandbox {
        PreparedSandbox {
            caps: CapabilitySet::default(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: None,
            allow_domain: Vec::new(),
            deny_domain: Vec::new(),
            credentials: Vec::new(),
            custom_credentials: std::collections::HashMap::new(),
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: profile::Wsl2ProxyPolicy::default(),
            #[cfg(target_os = "linux")]
            af_unix_mediation: profile::LinuxAfUnixMediation::default(),
            allow_launch_services_active: false,
            open_url_origins: Vec::new(),
            open_url_allow_localhost: false,
            bypass_protection_paths: Vec::new(),
            ignored_denial_paths: Vec::new(),
            suppressed_system_service_operations: Vec::new(),
            allowed_env_vars: None,
            denied_env_vars: None,
            set_vars: None,
            profile_network_block: false,
            loaded_profile: None,
            session_hooks: profile::SessionHooks::default(),
            allow_http2_requested: false,
        }
    }

    #[test]
    fn block_net_with_credential_errors() {
        let args = SandboxArgs {
            block_net: true,
            proxy_credential: vec!["openai".to_string()],
            ..Default::default()
        };
        let prepared = empty_prepared();
        let err = validate_block_net_conflicts(&args, &prepared)
            .expect_err("expected error for --block-net + --credential");
        assert!(
            err.to_string().contains("--block-net") && err.to_string().contains("--credential"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn block_net_with_network_profile_errors() {
        let args = SandboxArgs {
            block_net: true,
            network_profile: Some("strict".to_string()),
            ..Default::default()
        };
        let prepared = empty_prepared();
        let err = validate_block_net_conflicts(&args, &prepared)
            .expect_err("expected error for --block-net + --network-profile");
        assert!(
            err.to_string().contains("--block-net")
                && err.to_string().contains("--network-profile"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn block_net_with_allow_domain_errors() {
        let args = SandboxArgs {
            block_net: true,
            allow_proxy: vec!["example.com".to_string()],
            ..Default::default()
        };
        let prepared = empty_prepared();
        let err = validate_block_net_conflicts(&args, &prepared)
            .expect_err("expected error for --block-net + --allow-domain");
        assert!(
            err.to_string().contains("--block-net") && err.to_string().contains("--allow-domain"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn block_net_alone_is_valid() {
        let args = SandboxArgs {
            block_net: true,
            ..Default::default()
        };
        let prepared = empty_prepared();
        assert!(validate_block_net_conflicts(&args, &prepared).is_ok());
    }

    #[test]
    fn profile_network_block_with_credential_from_profile_errors() {
        let args = SandboxArgs::default();
        let mut prepared = empty_prepared();
        prepared.profile_network_block = true;
        prepared.credentials = vec!["github".to_string()];
        let err = validate_block_net_conflicts(&args, &prepared)
            .expect_err("expected error for profile network block + profile credential");
        assert!(
            err.to_string().contains("--block-net") && err.to_string().contains("--credential"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn allow_endpoint_without_credential_errors() {
        let args = SandboxArgs {
            allow_endpoint: vec!["openai:GET:/v1/chat/completions".to_string()],
            ..Default::default()
        };
        let prepared = empty_prepared();
        let err = validate_block_net_conflicts(&args, &prepared)
            .expect_err("expected error for --allow-endpoint without --credential");
        assert!(
            err.to_string().contains("--allow-endpoint")
                && err.to_string().contains("--credential"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn allow_endpoint_with_credential_is_valid() {
        let args = SandboxArgs {
            allow_endpoint: vec!["openai:GET:/v1/chat/completions".to_string()],
            proxy_credential: vec!["openai".to_string()],
            ..Default::default()
        };
        let prepared = empty_prepared();
        assert!(validate_block_net_conflicts(&args, &prepared).is_ok());
    }

    #[test]
    fn proxy_port_without_proxy_intent_errors() {
        let args = SandboxArgs {
            proxy_port: Some(8080),
            ..Default::default()
        };
        let prepared = empty_prepared();
        let err = validate_block_net_conflicts(&args, &prepared)
            .expect_err("expected error for --proxy-port without proxy mode");
        assert!(
            err.to_string().contains("--proxy-port"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn proxy_port_with_credential_is_valid() {
        let args = SandboxArgs {
            proxy_port: Some(8080),
            proxy_credential: vec!["openai".to_string()],
            ..Default::default()
        };
        let prepared = empty_prepared();
        assert!(validate_block_net_conflicts(&args, &prepared).is_ok());
    }

    // ========================================================================
    // D-04/D-05: validate_deny_domain_requires_allow_domain
    // ========================================================================

    #[test]
    fn deny_domain_without_allow_domain_errors() {
        let args = SandboxArgs {
            deny_proxy: vec!["evil.com".to_string()],
            ..Default::default()
        };
        let prepared = empty_prepared();
        let err = validate_deny_domain_requires_allow_domain(&args, &prepared)
            .expect_err("deny-only (no allow_domain) must be a hard error (D-05)");
        let msg = err.to_string();
        assert!(
            msg.contains("deny_domain") || msg.contains("--deny-domain"),
            "error should name deny_domain/--deny-domain: {msg}"
        );
        assert!(
            msg.contains("diverges from upstream") || msg.contains("upstream"),
            "error must explain the fork's divergence from upstream (D-05): {msg}"
        );
    }

    #[test]
    fn deny_domain_with_allow_domain_is_valid() {
        let args = SandboxArgs {
            deny_proxy: vec!["evil.com".to_string()],
            allow_proxy: vec!["good.com".to_string()],
            ..Default::default()
        };
        let prepared = empty_prepared();
        assert!(validate_deny_domain_requires_allow_domain(&args, &prepared).is_ok());
    }

    #[test]
    fn deny_domain_from_profile_without_allow_domain_errors() {
        let args = SandboxArgs::default();
        let mut prepared = empty_prepared();
        prepared.deny_domain = vec!["evil.com".to_string()];
        let err = validate_deny_domain_requires_allow_domain(&args, &prepared)
            .expect_err("profile deny_domain with no allow_domain must be a hard error (D-05)");
        assert!(err.to_string().contains("upstream"));
    }

    #[test]
    fn deny_domain_from_profile_with_allow_domain_from_profile_is_valid() {
        let args = SandboxArgs::default();
        let mut prepared = empty_prepared();
        prepared.deny_domain = vec!["evil.com".to_string()];
        prepared.allow_domain = vec![crate::profile::AllowDomainEntry::Plain(
            "good.com".to_string(),
        )];
        assert!(validate_deny_domain_requires_allow_domain(&args, &prepared).is_ok());
    }

    #[test]
    fn no_deny_domain_is_always_valid() {
        let args = SandboxArgs::default();
        let prepared = empty_prepared();
        assert!(validate_deny_domain_requires_allow_domain(&args, &prepared).is_ok());
    }
}
