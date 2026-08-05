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
fn build_launch_options(args: &ProxyArgs) -> Result<ProxyLaunchOptions> {
    let mut network_profile = args.network_profile.clone();
    let mut allow_domain: Vec<AllowDomainEntry> = Vec::new();
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
    credentials.extend(args.proxy_credential.iter().cloned());

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
        deny_domain: Vec::new(),
        no_proxy: Vec::new(),
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
        strict_filter: false,
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
