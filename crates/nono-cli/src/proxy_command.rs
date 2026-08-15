//! Standalone `nono proxy` command.
//!
//! Runs the network-filtering / credential-injection proxy as a foreground
//! server with no sandboxed child. Prints the connection details (bound
//! port, session token or `--no-auth` notice, route diagnostics) and then
//! blocks until Ctrl-C, periodically draining the in-memory network audit
//! buffer.
//!
//! WR-12: drained events are emitted to the `nono_security` tracing target
//! and, with `--audit-log PATH`, appended as newline-delimited JSON. This
//! mode has no OS sandbox behind it — the proxy IS the enforcement boundary —
//! so its allow/deny decisions, authentication failures and credential-route
//! usage are the whole audit trail. They used to be drained and dropped
//! because nothing else consumed them on this path, which left the one mode
//! with no kernel enforcement with zero auditability.
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

    // WR-05: binding beyond loopback must be a deliberate choice, not a
    // consequence of typing an address. The proxy speaks plain HTTP: the
    // session token crosses the network in every `Proxy-Authorization`
    // header, credential-injection routes mint real upstream secrets for
    // whoever presents it, and there is no TLS, no per-source-IP restriction
    // and no failed-auth lockout behind it.
    if !args.listen.is_loopback() && !args.allow_remote {
        return Err(NonoError::ConfigParse(format!(
            "--listen {} is not a loopback address. The proxy speaks plain HTTP with no \
             TLS: the session token crosses the network in every Proxy-Authorization \
             header, and any client presenting it can drive credential-injection routes \
             and obtain real upstream secrets. Pass --allow-remote to accept that \
             exposure deliberately, or bind a loopback address.",
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

    // WR-12: the in-memory network audit buffer used to be drained and
    // dropped on the floor. `nono proxy` is by construction the *only*
    // enforcement boundary in its mode — there is no OS sandbox behind it —
    // so allow/deny decisions, authentication failures and credential-route
    // usage are the entire audit trail for this mode, and discarding them
    // left the one mode with no kernel enforcement with zero auditability.
    // Draining is still required (left undrained the buffer fills to its cap
    // and logs a WARN on every subsequent request), but the events now go
    // somewhere.
    let mut audit_sink = AuditSink::open(args.audit_log.as_deref())?;

    let mut audit_drain = tokio::time::interval(Duration::from_secs(30));
    audit_drain.tick().await; // first tick fires immediately; nothing to drain yet.

    // WR-09: bind the Ctrl-C future ONCE. Constructing
    // `tokio::signal::ctrl_c()` inside the `select!` created a fresh listener
    // on every loop iteration, so a SIGINT delivered after the previous
    // future was dropped and before the new one registered was not observed —
    // and because tokio has already replaced the default SIGINT disposition,
    // the process did not die either, leaving the user to press Ctrl-C again.
    // The window reopened on every 30 s tick. Pinning the future keeps a
    // single registration alive for the whole loop. It is only polled until
    // it resolves, at which point we break.
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            signal = &mut ctrl_c => {
                if let Err(e) = signal {
                    tracing::warn!("failed to install Ctrl-C handler: {e}");
                }
                break;
            }
            _ = audit_drain.tick() => {
                audit_sink.emit(handle.drain_audit_events());
            }
        }
    }

    // Drain once more on the way out: the events produced since the last tick
    // are the ones immediately preceding shutdown, which is exactly when a
    // denial is most worth having.
    audit_sink.emit(handle.drain_audit_events());

    handle.shutdown();
    Ok(())
}

/// Destination for drained network audit events.
///
/// Always emits to the `nono_security` tracing target (so `-v`, `--log-file`
/// and the Windows ETW layer all see them), and additionally appends
/// newline-delimited JSON when `--audit-log PATH` was given.
struct AuditSink {
    file: Option<std::fs::File>,
    path: Option<std::path::PathBuf>,
}

impl AuditSink {
    /// Open the sink, failing before the proxy accepts traffic if the
    /// requested audit log cannot be appended to.
    ///
    /// Fail-closed: an operator who asked for an audit log must not get a
    /// running proxy that silently is not writing one.
    fn open(path: Option<&std::path::Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self {
                file: None,
                path: None,
            });
        };

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| {
                NonoError::ConfigParse(format!(
                    "cannot open --audit-log {} for appending: {e}",
                    path.display()
                ))
            })?;

        Ok(Self {
            file: Some(file),
            path: Some(path.to_path_buf()),
        })
    }

    fn emit(&mut self, events: Vec<nono::undo::NetworkAuditEvent>) {
        use std::io::Write;

        for event in &events {
            tracing::info!(
                target: "nono_security",
                timestamp_unix_ms = event.timestamp_unix_ms,
                mode = ?event.mode,
                decision = ?event.decision,
                target_host = %event.target,
                port = ?event.port,
                method = ?event.method,
                path = ?event.path,
                status = ?event.status,
                route_id = ?event.route_id,
                auth_mechanism = ?event.auth_mechanism,
                auth_outcome = ?event.auth_outcome,
                denial_category = ?event.denial_category,
                managed_credential_active = ?event.managed_credential_active,
                injection_mode = ?event.injection_mode,
                reason = ?event.reason,
                "proxy network audit event"
            );
        }

        let (Some(file), Some(path)) = (self.file.as_mut(), self.path.as_ref()) else {
            return;
        };

        for event in &events {
            // A serialization failure here would mean a malformed event, not
            // a malformed log — report it and keep the remaining events
            // rather than losing the whole batch.
            let line = match serde_json::to_string(event) {
                Ok(line) => line,
                Err(e) => {
                    tracing::error!("failed to serialize a network audit event: {e}");
                    continue;
                }
            };
            if let Err(e) = writeln!(file, "{line}") {
                tracing::error!("failed to append to --audit-log {}: {e}", path.display());
                return;
            }
        }

        if let Err(e) = file.flush() {
            tracing::error!("failed to flush --audit-log {}: {e}", path.display());
        }
    }
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
            *handle.token, args.listen, handle.port
        );
        eprintln!("  [nono] NONO_PROXY_TOKEN={}", *handle.token);
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

    // WR-05: a non-loopback bind is opt-in (--allow-remote), but the operator
    // still needs the exposure spelled out at the point of use, and needs to
    // know which credential routes are now mintable by anyone holding the
    // token that just crossed the network in cleartext.
    if !config.bind_addr.is_loopback() {
        eprintln!(
            "  [nono] WARNING: bound to a non-loopback address. This proxy speaks plain \
             HTTP with no TLS — the session token above traverses the network in every \
             Proxy-Authorization header, and there is no per-source-IP restriction and \
             no failed-auth lockout."
        );
        let credential_routes: Vec<&str> = config
            .routes
            .iter()
            .filter(|route| route.credential_key.is_some())
            .map(|route| route.prefix.as_str())
            .collect();
        if !credential_routes.is_empty() {
            eprintln!(
                "  [nono] WARNING: {} credential-injection route(s) are reachable from the \
                 network (/{}). Anyone who obtains the session token can mint real \
                 upstream credentials through them.",
                credential_routes.len(),
                credential_routes.join(", /")
            );
        }
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
            allow_remote: false,
            audit_log: None,
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

    /// WR-05: a non-loopback `--listen` exposes the session token and every
    /// credential-injection route over plaintext HTTP. It now requires an
    /// explicit `--allow-remote` opt-in.
    #[test]
    fn non_loopback_listen_requires_allow_remote() {
        let mut args = base_args();
        args.listen = std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);

        let err = run_proxy(args, true).expect_err("a non-loopback bind must be opt-in");
        let msg = err.to_string();
        assert!(
            msg.contains("--allow-remote"),
            "error must point at the opt-in flag: {msg}"
        );
    }

    /// WR-05: `--no-auth` stays refused for non-loopback binds even with
    /// `--allow-remote`. The two guards are independent — `--allow-remote`
    /// accepts plaintext exposure of an authenticated proxy, not an open one.
    #[test]
    fn allow_remote_does_not_unlock_no_auth_off_loopback() {
        let mut args = base_args();
        args.listen = std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
        args.allow_remote = true;
        args.no_auth = true;

        let err = run_proxy(args, true).expect_err("--no-auth off loopback must stay refused");
        let msg = err.to_string();
        assert!(
            msg.contains("--no-auth"),
            "error must name --no-auth: {msg}"
        );
    }

    /// WR-12: an operator who asked for an audit log must never get a
    /// running proxy that silently is not writing one. The sink is opened
    /// before the listener binds.
    #[test]
    fn audit_sink_open_fails_closed_on_unwritable_path() {
        let dir = tempfile::tempdir().unwrap();
        // A path whose parent does not exist cannot be created.
        let unwritable = dir.path().join("no-such-dir").join("audit.jsonl");
        assert!(AuditSink::open(Some(&unwritable)).is_err());
    }

    /// WR-12: drained events must reach the audit log as newline-delimited
    /// JSON instead of being dropped on the floor.
    #[test]
    fn audit_sink_appends_events_as_json_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");
        let mut sink = AuditSink::open(Some(&log_path)).expect("sink must open");

        let event = nono::undo::NetworkAuditEvent {
            timestamp_unix_ms: 1,
            mode: nono::undo::NetworkAuditMode::Connect,
            decision: nono::undo::NetworkAuditDecision::Deny,
            route_id: None,
            auth_mechanism: None,
            auth_outcome: None,
            managed_credential_active: None,
            injection_mode: None,
            denial_category: Some(nono::undo::NetworkAuditDenialCategory::HostDenied),
            spiffe_context: None,
            capture_context: None,
            target: "blocked.example".to_string(),
            port: Some(443),
            method: None,
            path: None,
            status: None,
            reason: Some("host not in allowlist".to_string()),
        };
        sink.emit(vec![event]);

        let written = std::fs::read_to_string(&log_path).expect("audit log must exist");
        let mut lines = written.lines();
        let first = lines.next().expect("one event must have been appended");
        assert!(lines.next().is_none(), "exactly one line expected");

        let parsed: serde_json::Value = serde_json::from_str(first).expect("each line is JSON");
        assert_eq!(parsed["target"], "blocked.example");
        assert_eq!(parsed["reason"], "host not in allowlist");
    }

    /// Without `--audit-log` the sink is still constructible and emitting is
    /// a no-op on disk (events still reach the `nono_security` target).
    #[test]
    fn audit_sink_without_path_is_a_tracing_only_sink() {
        let mut sink = AuditSink::open(None).expect("sink must open");
        sink.emit(Vec::new());
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
