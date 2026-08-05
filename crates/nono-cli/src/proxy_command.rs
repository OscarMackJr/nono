//! Standalone `nono proxy` command.
//!
//! Runs the network-filtering / credential-injection proxy as a foreground
//! server with no sandboxed child. Prints the connection details (bound
//! port, session token or `--no-auth` notice, route diagnostics) and then
//! blocks until Ctrl-C, periodically draining the in-memory network audit
//! buffer (nothing else consumes it on this path — only the sandboxed
//! rollback path drains it, and this command performs no rollback audit
//! recording).
//!
//! Phase 112 SEC-07 (adapted from upstream `2663e990`, #1261): upstream's
//! `proxy_command.rs` constructs several credential/domain/endpoint/TLS/
//! upstream "Intent" struct values from `launch_runtime.rs` — none of those
//! types exist in this fork (confirmed via live grep against
//! `launch_runtime.rs`, 0 hits; see `112-07-SUMMARY.md` for the exact
//! upstream type names checked). This fork's post-ADR-98 network-launch
//! shape is `ProxyLaunchOptions` / `proxy_runtime::build_proxy_config_from_flags`
//! / `nono_proxy::server::start`, the same path `nono run`'s proxy activation
//! uses — reused here directly rather than reimplemented. Upstream's TLS-
//! interception config-apply call is dropped entirely: this fork has no
//! TLS-interception module (0 grep hits), so there is nothing to call.

use crate::cli::ProxyArgs;
use crate::launch_runtime::ProxyLaunchOptions;
use crate::profile::{self, AllowDomainEntry};
use crate::proxy_runtime;
use nono::{NonoError, Result};
use std::collections::HashMap;
use std::time::Duration;

/// Entry point for `nono proxy`. Dispatched from `app_runtime::dispatch_command`.
pub(crate) fn run_proxy(args: ProxyArgs, silent: bool) -> Result<()> {
    // Fail-secure guard (T-112-15): an open (no-token) proxy must never be
    // reachable from other hosts. Checked before any launch-option
    // construction or server startup work.
    if args.no_auth && !args.listen.is_loopback() {
        return Err(NonoError::ConfigParse(format!(
            "--no-auth requires a loopback --listen address (got {}); refusing to start an \
             open proxy reachable from other hosts",
            args.listen
        )));
    }

    let launch_options = build_launch_options(&args)?;
    let mut proxy_config = proxy_runtime::build_proxy_config_from_flags(&launch_options)?;
    // Fields not modeled on `ProxyLaunchOptions` (the sandboxed `run` path
    // always binds loopback with a fixed connection ceiling and the
    // sandboxed-run default auth posture) — set directly on the resolved
    // `ProxyConfig` for the standalone path only.
    proxy_config.bind_addr = args.listen;
    proxy_config.bind_port = args.port;
    proxy_config.max_connections = args.max_connections;
    proxy_config.require_auth = !args.no_auth;
    proxy_config.strict_connect_auth = !args.no_auth;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| NonoError::SandboxInit(format!("Failed to start proxy runtime: {e}")))?;

    rt.block_on(run_until_shutdown(args, proxy_config, silent))
}

/// Merge `--profile` network settings with explicit `ProxyArgs` flags into a
/// `ProxyLaunchOptions`, following the same profile-extends-CLI merge
/// direction as `proxy_runtime::resolve_effective_proxy_settings` (the
/// `nono run` equivalent). `active: true` unconditionally — unlike `nono
/// run`, where proxy activation is inferred from whether any proxy-shaped
/// flag was passed, `nono proxy` is an explicit standalone invocation.
///
/// Every security-relevant field of the profile's `network` block is
/// honoured here: dropping `deny_domain`, `no_proxy`, or `block` would make
/// `nono proxy --profile X` strictly weaker than `nono run --profile X` for
/// the same profile, which is a silent downgrade rather than a documented
/// difference.
///
/// # Errors
///
/// Returns `NonoError::ConfigParse` when the effective config is deny-only
/// (D-04/D-05 parity with `nono run`), when `--upstream-bypass` is given
/// without an upstream proxy, or when an `--allow-endpoint` argument is
/// malformed.
fn build_launch_options(args: &ProxyArgs) -> Result<ProxyLaunchOptions> {
    let mut network_profile = args.network_profile.clone();
    let mut allow_domain: Vec<AllowDomainEntry> = Vec::new();
    let mut deny_domain: Vec<String> = Vec::new();
    let mut no_proxy: Vec<String> = Vec::new();
    let mut profile_network_block = false;
    let mut credentials: Vec<String> = Vec::new();
    let mut custom_credentials: HashMap<String, profile::CustomCredentialDef> = HashMap::new();
    let mut upstream_proxy = args.external_proxy.clone();
    let mut upstream_bypass = args.external_proxy_bypass.clone();
    let mut enable_h2 = args.allow_http2;

    if let Some(ref name) = args.profile {
        let loaded = profile::load_profile(name)?;
        let net = loaded.network;
        if network_profile.is_none() {
            network_profile = net.resolved_network_profile().map(ToString::to_string);
        }
        allow_domain.extend(net.allow_domain.clone());
        // CR-01 fix: the profile's deny layer, `no_proxy` bypass list, and
        // `network.block` were previously dropped on the floor here, turning
        // a deny-scoped or fully-blocked profile into an allow-all
        // forwarding proxy. All three now flow through exactly as they do on
        // the `nono run` path (`proxy_runtime::resolve_effective_proxy_settings`
        // + `prepare_proxy_launch_options`).
        deny_domain.extend(net.deny_domain.clone());
        no_proxy.extend(net.no_proxy.clone());
        profile_network_block = net.block;
        credentials.extend(net.resolved_credentials().iter().cloned());
        custom_credentials.extend(net.custom_credentials.clone());
        if upstream_proxy.is_none() {
            upstream_proxy.clone_from(&net.upstream_proxy);
        }
        if upstream_bypass.is_empty() {
            upstream_bypass.clone_from(&net.upstream_bypass);
        }
        enable_h2 = enable_h2 || net.allow_http2;
    }

    allow_domain.extend(
        args.allow_proxy
            .iter()
            .map(|s| proxy_runtime::parse_allow_domain_arg(s)),
    );
    // deny_domain mirrors allow_domain's merge direction (profile entries
    // first, then the CLI flag), matching `nono run`'s
    // `prepare_proxy_launch_options`.
    deny_domain.extend(args.deny_proxy.iter().cloned());
    credentials.extend(args.proxy_credential.iter().cloned());

    // D-04/D-05 parity with `nono run`'s
    // `sandbox_prepare::validate_deny_domain_requires_allow_domain`: a
    // deny-only effective config is a hard error at parse time, never a
    // silent fallback to default-allow. A guard on only one of the two
    // commands is a bypass, not a guard.
    if !deny_domain.is_empty() && allow_domain.is_empty() {
        return Err(NonoError::ConfigParse(
            "deny_domain / --deny-domain requires at least one allow_domain / \
             --allow-domain entry. This fork deliberately diverges from upstream \
             nono here: upstream's deny_domain silently applies default-allow \
             semantics (\"allow everything except these domains\"), which would \
             leave every host you did not think to name reachable. Add at least \
             one --allow-domain (or profile allow_domain) entry."
                .to_string(),
        ));
    }

    // Fails fast on the first malformed --allow-endpoint argument (service
    // missing, path missing leading '/') — same fail-closed behavior as
    // `nono run`'s equivalent flag.
    let endpoint_restrictions = args
        .allow_endpoint
        .iter()
        .map(|s| proxy_runtime::parse_allow_endpoint_arg(s))
        .collect::<Result<Vec<_>>>()?;

    // Mirrors `sandbox_prepare::validate_external_proxy_bypass` (the `nono
    // run` equivalent guard): a bypass list with no upstream proxy to bypass
    // is a contradiction, not a no-op.
    if !upstream_bypass.is_empty() && upstream_proxy.is_none() {
        return Err(NonoError::ConfigParse(
            "--upstream-bypass requires --upstream-proxy (or upstream_proxy in profile \
             network config)"
                .to_string(),
        ));
    }

    Ok(ProxyLaunchOptions {
        active: true,
        network_profile,
        allow_domain,
        deny_domain,
        no_proxy,
        credentials,
        custom_credentials,
        upstream_proxy,
        upstream_bypass,
        allow_bind_ports: Vec::new(),
        // `proxy_port` would overwrite `ProxyConfig.bind_port` inside
        // `build_proxy_config_from_flags`; the standalone path sets
        // `bind_port` directly from `args.port` after that call instead, so
        // this stays `None` to avoid a redundant/confusing double-source.
        proxy_port: None,
        open_url_origins: Vec::new(),
        open_url_allow_localhost: false,
        allow_launch_services_active: false,
        // Profile `network.block` means "deny any host not explicitly
        // allowed" once a proxy is active; forcing this to `false`
        // downgraded a blocked profile to an allow-all forwarding proxy.
        strict_filter: profile_network_block,
        enable_h2,
        endpoint_restrictions,
    })
}

/// Start the proxy, print connection info, and block until Ctrl-C.
async fn run_until_shutdown(
    args: ProxyArgs,
    proxy_config: nono_proxy::config::ProxyConfig,
    silent: bool,
) -> Result<()> {
    let handle = nono_proxy::server::start(proxy_config.clone())
        .await
        .map_err(|e| NonoError::SandboxInit(format!("Failed to start proxy: {e}")))?;

    print_connection_info(&args, &handle, &proxy_config, silent);

    // Nothing else consumes the in-memory network audit buffer on this
    // standalone path — only the sandboxed rollback path drains it, and this
    // command performs no rollback audit recording. Left undrained, the
    // buffer fills to its cap and logs a WARN on every subsequent request.
    let mut audit_drain = tokio::time::interval(Duration::from_secs(30));
    audit_drain.tick().await; // first tick fires immediately; nothing to drain yet.
    loop {
        tokio::select! {
            signal = tokio::signal::ctrl_c() => {
                if let Err(e) = signal {
                    tracing::warn!("failed to install Ctrl-C handler: {e}");
                }
                break;
            }
            _ = audit_drain.tick() => {
                let _ = handle.drain_audit_events();
            }
        }
    }

    handle.shutdown();
    Ok(())
}

fn print_connection_info(
    args: &ProxyArgs,
    handle: &nono_proxy::server::ProxyHandle,
    config: &nono_proxy::config::ProxyConfig,
    silent: bool,
) {
    if silent {
        return;
    }

    eprintln!();
    eprintln!(
        "  [nono] proxy listening on {}:{}",
        args.listen, handle.port
    );
    if config.require_auth {
        eprintln!(
            "  [nono] HTTPS_PROXY=http://nono:{}@{}:{}",
            &*handle.token, args.listen, handle.port
        );
        eprintln!("  [nono] NONO_PROXY_TOKEN={}", &*handle.token);
    } else {
        eprintln!(
            "  [nono] WARNING: --no-auth active — every request on this address is accepted \
             without a session token"
        );
        eprintln!(
            "  [nono] HTTPS_PROXY=http://{}:{}",
            args.listen, handle.port
        );
    }

    // CR-01: an empty allowlist with no strict-filter selection means
    // `ProxyConfig.allowed_hosts` falls back to "allow all hosts (except the
    // deny list)". That is a legitimate standalone-proxy configuration, but
    // it is not a filtering boundary and must never be mistaken for one.
    if config.allowed_hosts.is_empty() && !config.strict_filter {
        eprintln!(
            "  [nono] WARNING: no allow-domain filter is active — this proxy forwards to \
             ANY host. Pass --allow-domain (or --profile with network.allow_domain) to \
             filter egress."
        );
    }

    let route_rows = handle.route_diagnostics(config);
    if !route_rows.is_empty() {
        eprintln!("  [nono] routes:");
        for (prefix, summary) in &route_rows {
            eprintln!("    /{prefix}  {summary}");
        }
    }

    let proxy_diagnostics = handle.diagnostics();
    if !proxy_diagnostics.is_empty() {
        crate::output::print_proxy_diagnostics(proxy_diagnostics);
    }

    eprintln!();
    eprintln!("  [nono] press Ctrl-C to stop");
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// A `ProxyArgs` with every field at its clap default, so each test can
    /// vary exactly the one input it is about.
    fn base_args() -> ProxyArgs {
        ProxyArgs {
            listen: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            port: 0,
            no_auth: false,
            max_connections: 256,
            profile: None,
            network_profile: None,
            allow_proxy: Vec::new(),
            deny_proxy: Vec::new(),
            external_proxy: None,
            external_proxy_bypass: Vec::new(),
            allow_http2: false,
            proxy_credential: Vec::new(),
            allow_endpoint: Vec::new(),
            verbose: 0,
        }
    }

    fn write_profile(dir: &std::path::Path, network_json: &str) -> String {
        let path = dir.join("proxy-cmd-test.json");
        std::fs::write(
            &path,
            format!(
                r#"{{
                    "meta": {{ "name": "proxy-cmd-test" }},
                    "network": {network_json}
                }}"#
            ),
        )
        .unwrap();
        path.to_str().unwrap().to_string()
    }

    /// CR-01 regression: `nono proxy --profile X` must carry the profile's
    /// `deny_domain`, `no_proxy`, and `network.block` into
    /// `ProxyLaunchOptions`. Before the fix these three were hardcoded to
    /// empty/false, so a deny-scoped or fully-blocked profile silently
    /// became an allow-all forwarding proxy.
    #[test]
    fn profile_deny_layer_no_proxy_and_block_reach_launch_options() {
        let dir = tempfile::tempdir().unwrap();
        let profile_path = write_profile(
            dir.path(),
            r#"{
                "block": true,
                "allow_domain": ["*.corp.com"],
                "deny_domain": ["secrets.corp.com"],
                "no_proxy": ["internal-only.example"]
            }"#,
        );

        let mut args = base_args();
        args.profile = Some(profile_path);

        let opts = build_launch_options(&args).expect("profile must load");

        assert!(
            opts.deny_domain.contains(&"secrets.corp.com".to_string()),
            "profile deny_domain must reach ProxyLaunchOptions: {:?}",
            opts.deny_domain
        );
        assert!(
            opts.no_proxy.contains(&"internal-only.example".to_string()),
            "profile no_proxy must reach ProxyLaunchOptions: {:?}",
            opts.no_proxy
        );
        assert!(
            opts.strict_filter,
            "profile network.block must set strict_filter on ProxyLaunchOptions"
        );
        assert!(
            !opts.allow_domain.is_empty(),
            "profile allow_domain must still be merged"
        );
    }

    /// CR-01: the `--deny-domain` flag exists on `nono proxy` and composes
    /// with the profile's deny entries rather than replacing them.
    #[test]
    fn cli_deny_domain_flag_composes_with_profile_deny_domain() {
        let dir = tempfile::tempdir().unwrap();
        let profile_path = write_profile(
            dir.path(),
            r#"{
                "allow_domain": ["*.corp.com"],
                "deny_domain": ["secrets.corp.com"]
            }"#,
        );

        let mut args = base_args();
        args.profile = Some(profile_path);
        args.deny_proxy = vec!["extra-evil.corp.com".to_string()];

        let opts = build_launch_options(&args).expect("profile must load");

        assert!(opts.deny_domain.contains(&"secrets.corp.com".to_string()));
        assert!(opts
            .deny_domain
            .contains(&"extra-evil.corp.com".to_string()));
    }

    /// CR-01 / D-04 parity: a deny-only effective config is a hard error on
    /// `nono proxy` exactly as it is on `nono run`. A guard on only one of
    /// the two commands is a bypass, not a guard.
    #[test]
    fn cli_deny_domain_without_allow_domain_is_rejected() {
        let mut args = base_args();
        args.deny_proxy = vec!["evil.com".to_string()];

        let err = build_launch_options(&args)
            .expect_err("deny-only config must be rejected at parse time");
        let msg = err.to_string();
        assert!(
            msg.contains("deny_domain") || msg.contains("--deny-domain"),
            "error must name deny_domain/--deny-domain: {msg}"
        );
    }

    /// CR-01 / D-04 parity: the same guard fires when the deny entries come
    /// from a profile rather than the CLI flag.
    #[test]
    fn profile_deny_domain_without_allow_domain_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let profile_path = write_profile(dir.path(), r#"{ "deny_domain": ["evil.com"] }"#);

        let mut args = base_args();
        args.profile = Some(profile_path);

        let err = build_launch_options(&args)
            .expect_err("profile deny-only config must be rejected at parse time");
        let msg = err.to_string();
        assert!(
            msg.contains("deny_domain") || msg.contains("--deny-domain"),
            "error must name deny_domain/--deny-domain: {msg}"
        );
    }

    /// A profile with no network deny layer and no `block` must not
    /// spuriously enable strict filtering (guards against over-correcting
    /// CR-01 into a behavior change for ordinary profiles).
    #[test]
    fn plain_profile_leaves_deny_layer_empty_and_strict_filter_off() {
        let dir = tempfile::tempdir().unwrap();
        let profile_path = write_profile(dir.path(), r#"{ "allow_domain": ["github.com"] }"#);

        let mut args = base_args();
        args.profile = Some(profile_path);

        let opts = build_launch_options(&args).expect("profile must load");
        assert!(opts.deny_domain.is_empty());
        assert!(opts.no_proxy.is_empty());
        assert!(!opts.strict_filter);
    }
}
