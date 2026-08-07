use crate::cli::SandboxArgs;
use crate::launch_runtime::{NetworkIntent, ProxyLaunchOptions};
use crate::network_policy;
use crate::profile::AllowDomainEntry;
use crate::sandbox_prepare::{validate_external_proxy_bypass, PreparedSandbox};
use nono::{CapabilitySet, NonoError, Result, UnixSocketOp};
use std::path::{Path, PathBuf};
use tracing::debug;
use tracing::info;
use tracing::warn;

pub(crate) struct ActiveProxyRuntime {
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) handle: Option<nono_proxy::server::ProxyHandle>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EffectiveProxySettings {
    pub(crate) network_profile: Option<String>,
    pub(crate) allow_domain: Vec<AllowDomainEntry>,
    /// Raw `deny_domain` entries (profile + `--deny-domain`), not yet
    /// group-expanded. Expansion happens in `build_proxy_config_from_flags`
    /// via `network_policy::expand_proxy_deny`.
    pub(crate) deny_domain: Vec<String>,
    /// Raw `no_proxy` entries from `profile.network.no_proxy`. Profile-only
    /// (no `--no-proxy` CLI flag). D-06's overlap validators
    /// (`validate_proxy_launch_no_proxy_conflicts`,
    /// `validate_expanded_proxy_no_proxy_conflicts`) run against these in
    /// `build_proxy_config_from_flags`, before and after `allow_domain`
    /// group-name expansion respectively.
    pub(crate) no_proxy: Vec<String>,
    pub(crate) credentials: Vec<String>,
}

/// Parse a `--allow-domain` CLI argument that may be a bare hostname or a URL with path.
///
/// - Plain hostname (e.g. `api.openai.com`) → `AllowDomainEntry::Plain`
/// - `host:port` (e.g. `api.openai.com:8080`) → `AllowDomainEntry::Plain` (port is stripped
///   by downstream filtering; no URL parsing attempted, prevents the scheme-confusion
///   where `url::Url::parse("host:port")` treats the host as the URL scheme)
/// - URL with non-root path (e.g. `https://api.github.com/repos/my-org/**`) →
///   `AllowDomainEntry::WithEndpoints` with a single wildcard-method rule
/// - URL with root or empty path → `AllowDomainEntry::Plain` (no endpoint restriction)
/// - Unparseable URL or no scheme → `AllowDomainEntry::Plain` (fallback)
///
/// Upstream-commit: 75b2265 (adapted: guard URL parse behind explicit http/https scheme check
/// to avoid mangling `host:port` entries — CR-02 fix)
/// Phase 112 SEC-07: made `pub(crate)` (was private) so `proxy_command.rs`
/// can reuse it verbatim when constructing `ProxyLaunchOptions` from
/// `ProxyArgs` — matches upstream `2663e990`'s own `pub(crate)` promotion.
pub(crate) fn parse_allow_domain_arg(input: &str) -> AllowDomainEntry {
    let looks_like_url = input.starts_with("http://") || input.starts_with("https://");
    if looks_like_url {
        if let Ok(parsed) = url::Url::parse(input) {
            if let Some(host) = parsed.host_str() {
                let domain = host.to_string();
                let path = parsed.path();
                return if path.is_empty() || path == "/" {
                    AllowDomainEntry::Plain(domain)
                } else {
                    AllowDomainEntry::WithEndpoints {
                        domain,
                        endpoints: vec![nono_proxy::config::EndpointRule {
                            method: "*".to_string(),
                            path: path.to_string(),
                        }],
                    }
                };
            }
        }
    }
    AllowDomainEntry::Plain(input.to_string())
}

/// Parse a `--allow-endpoint SERVICE:METHOD:PATH` argument into a
/// `(service_prefix, EndpointRule)` pair, or an error if the argument is
/// malformed.
///
/// Format: `SERVICE:METHOD:PATH` where:
/// - `SERVICE` is the credential/route prefix (e.g., `openai`, `github`)
/// - `METHOD` is an HTTP method or `*` (e.g., `GET`, `POST`, `*`)
/// - `PATH` is an exact or glob path pattern starting with `/` (e.g., `/v1/chat/completions`,
///   `/repos/*/issues`)
///
/// Upstream 46bcfbb9 (#1127): absorbed; returns `Result` for fail-fast error reporting.
///
/// Phase 112 SEC-07: made `pub(crate)` (was private) so `proxy_command.rs`
/// can reuse it — matches upstream `2663e990`'s own `pub(crate)` promotion.
pub(crate) fn parse_allow_endpoint_arg(
    entry: &str,
) -> nono::Result<(String, nono_proxy::config::EndpointRule)> {
    let err = || {
        nono::NonoError::ConfigParse(format!(
            "--allow-endpoint '{}': expected format SERVICE:METHOD:PATH \
             (e.g., 'github:GET:/repos/*/issues')",
            entry
        ))
    };
    let (service, rest) = entry.split_once(':').ok_or_else(err)?;
    let (method, path) = rest.split_once(':').ok_or_else(err)?;
    if service.is_empty() || method.is_empty() || path.is_empty() {
        return Err(err());
    }
    if !path.starts_with('/') {
        return Err(nono::NonoError::ConfigParse(format!(
            "--allow-endpoint '{}': path pattern must start with '/' \
             (e.g., '/repos/*/issues', not 'repos/*/issues')",
            entry
        )));
    }
    Ok((
        service.to_string(),
        nono_proxy::config::EndpointRule {
            method: method.to_string(),
            path: path.to_string(),
        },
    ))
}

/// Prepare the network intent for the sandbox run.
///
/// Returns a `NetworkIntent` describing the resolved network mode:
/// - `BlockAll` when `--block-net` or profile `network.block` wins over any proxy config.
/// - `ProxyFiltered(opts)` when proxy-activating features are configured.
/// - `Unrestricted` when `--allow-net` or no network flags are given.
///
/// Upstream 72bcfd66 (#1225): changed return type from ProxyLaunchOptions to NetworkIntent;
/// removed ProxyOnly { port: 0 } placeholder detection (no longer set in capability_ext.rs);
/// replaced network_block_requested with profile_network_block + direct args.block_net check.
pub(crate) fn prepare_proxy_launch_options(
    args: &SandboxArgs,
    prepared: &PreparedSandbox,
    silent: bool,
) -> Result<NetworkIntent> {
    validate_external_proxy_bypass(args, prepared)?;

    let effective_proxy = resolve_effective_proxy_settings(args, prepared);
    let network_profile = effective_proxy.network_profile;
    let allow_domain = effective_proxy.allow_domain;
    let deny_domain = effective_proxy.deny_domain;
    let no_proxy = effective_proxy.no_proxy;
    let credentials = effective_proxy.credentials;
    let allow_bind_ports = merge_dedup_ports(&prepared.listen_ports, &args.allow_bind);

    let upstream_proxy = if args.allow_net {
        None
    } else {
        args.external_proxy
            .clone()
            .or_else(|| prepared.upstream_proxy.clone())
    };

    let upstream_bypass = if args.allow_net {
        Vec::new()
    } else if args.external_proxy.is_some() {
        args.external_proxy_bypass.clone()
    } else {
        let mut bypass = prepared.upstream_bypass.clone();
        bypass.extend(args.external_proxy_bypass.clone());
        bypass
    };

    // Determine whether any proxy-activating configuration is present.
    //
    // ADR-108 Consequence (c): `deny_domain` is DELIBERATELY excluded from
    // this OR-chain — unlike `allow_domain`, which IS a term. Upstream's
    // `deny_domain` activates the proxy on its own ("allow everything
    // except these domains"); this fork rejects that trigger. A
    // deny+allow_domain configuration still activates via the
    // `!allow_domain.is_empty()` term above. This is safe only because
    // `sandbox_prepare::validate_deny_domain_requires_allow_domain` (D-04)
    // runs upstream of this function on BOTH `nono run` call sites
    // (command_runtime.rs's dry-run branch and launch_runtime.rs's real
    // launch path) and rejects every deny-only state before it can reach
    // `would_activate` — so a bare `deny_domain` can never appear here
    // without a paired `allow_domain`.
    let would_activate = !credentials.is_empty()
        || network_profile.is_some()
        || !allow_domain.is_empty()
        || upstream_proxy.is_some()
        || !prepared.custom_credentials.is_empty(); // #1197 / D-07

    // --block-net always wins; profile network.block yields to any proxy config.
    // Per upstream 72bcfd66 (#1225): block_wins replaces the old NetworkMode::Blocked check.
    let block_wins = args.block_net || (prepared.profile_network_block && !would_activate);

    if block_wins {
        if would_activate {
            warn!(
                "--block-net is active; ignoring proxy configuration \
                 that would re-enable network access"
            );
            if !silent {
                eprintln!(
                    "  [nono] Warning: --block-net overrides proxy/credential settings. \
                     Network remains fully blocked."
                );
            }
        }
        return Ok(NetworkIntent::BlockAll);
    }

    if args.allow_net {
        return Ok(NetworkIntent::Unrestricted);
    }

    // Profile network.block + proxy flags → strict mode: deny unlisted hosts.
    let strict_filter = prepared.profile_network_block;

    let active = would_activate;

    // Parse --allow-endpoint SERVICE:METHOD:PATH args into typed (prefix, EndpointRule) pairs.
    // Fails fast on the first malformed argument (service missing, path missing leading '/').
    let endpoint_restrictions = args
        .allow_endpoint
        .iter()
        .map(|s| parse_allow_endpoint_arg(s))
        .collect::<nono::Result<Vec<_>>>()?;

    Ok(NetworkIntent::ProxyFiltered(Box::new(ProxyLaunchOptions {
        active,
        network_profile,
        allow_domain,
        deny_domain,
        no_proxy,
        credentials,
        custom_credentials: prepared.custom_credentials.clone(),
        upstream_proxy,
        upstream_bypass,
        allow_bind_ports,
        proxy_port: args.proxy_port,
        open_url_origins: prepared.open_url_origins.clone(),
        open_url_allow_localhost: prepared.open_url_allow_localhost,
        allow_launch_services_active: prepared.allow_launch_services_active,
        strict_filter,
        enable_h2: prepared.allow_http2_requested || args.allow_http2,
        endpoint_restrictions,
    })))
}

pub(crate) fn resolve_effective_proxy_settings(
    args: &SandboxArgs,
    prepared: &PreparedSandbox,
) -> EffectiveProxySettings {
    if args.allow_net {
        return EffectiveProxySettings {
            network_profile: None,
            allow_domain: Vec::new(),
            deny_domain: Vec::new(),
            no_proxy: Vec::new(),
            credentials: Vec::new(),
        };
    }

    let network_profile = args
        .network_profile
        .clone()
        .or_else(|| prepared.network_profile.clone());
    // Clone the structured entries from PreparedSandbox (already Vec<AllowDomainEntry> —
    // endpoint rules from profile WithEndpoints entries are preserved end-to-end).
    // CLI --allow-domain args are parsed and appended; they use parse_allow_domain_arg
    // because they arrive as raw strings from the command line.
    let mut allow_domain: Vec<AllowDomainEntry> = prepared.allow_domain.clone();
    allow_domain.extend(args.allow_proxy.iter().map(|s| parse_allow_domain_arg(s)));
    // deny_domain mirrors allow_domain's merge (profile entries + CLI flag),
    // but stays a flat Vec<String> — group-name expansion happens later in
    // build_proxy_config_from_flags via network_policy::expand_proxy_deny.
    let mut deny_domain = prepared.deny_domain.clone();
    deny_domain.extend(args.deny_proxy.iter().cloned());
    // no_proxy is profile-only — there is no `--no-proxy` CLI flag, so this
    // is a straight clone (no merge step).
    let no_proxy = prepared.no_proxy.clone();
    let mut credentials = prepared.credentials.clone();
    credentials.extend(args.proxy_credential.clone());

    EffectiveProxySettings {
        network_profile,
        allow_domain,
        deny_domain,
        no_proxy,
        credentials,
    }
}

pub(crate) fn merge_dedup_ports(a: &[u16], b: &[u16]) -> Vec<u16> {
    let mut ports = a.to_vec();
    ports.extend_from_slice(b);
    ports.sort_unstable();
    ports.dedup();
    ports
}

/// D-06 (#1415) / T-109-08 sibling: reject a `no_proxy` entry that overlaps a
/// LITERAL (pre-group-expansion) `allow_domain` entry on `ProxyLaunchOptions`.
///
/// This is the CLI-flag-layer counterpart to `profile::validate_profile_no_proxy`
/// — it runs on the fully-merged (profile + CLI) `ProxyLaunchOptions`, catching
/// overlaps that only appear after `--allow-domain`/profile merge. It does NOT
/// see allow_domain group names expanded to their member hosts — that is
/// `validate_expanded_proxy_no_proxy_conflicts`'s job, run later once
/// `network_policy::partition_allow_domain` has expanded them.
///
/// Uses `nono_proxy::config::no_proxy_entry_overlaps_host_pattern` (Plan
/// 109-02's proxy-crate validator) so this check's overlap semantics stay
/// identical to the proxy-crate's own runtime overlap check.
fn validate_proxy_launch_no_proxy_conflicts(proxy: &ProxyLaunchOptions) -> Result<()> {
    for no_proxy_entry in &proxy.no_proxy {
        for allow in &proxy.allow_domain {
            let host = allow.domain();
            if nono_proxy::config::no_proxy_entry_overlaps_host_pattern(no_proxy_entry, host) {
                return Err(NonoError::ConfigParse(format!(
                    "no_proxy entry '{no_proxy_entry}' overlaps allow_domain entry '{host}' — \
                     a no_proxy bypass cannot cover a host explicitly allowed through the proxy \
                     (D-06)."
                )));
            }
        }
    }
    Ok(())
}

/// D-06 (#1415) / T-109-09: reject a `no_proxy` entry that overlaps a host
/// reachable only AFTER `allow_domain` group-name expansion
/// (`network_policy::partition_allow_domain`).
///
/// `validate_proxy_launch_no_proxy_conflicts` only sees literal `allow_domain`
/// strings/group names as declared — a `no_proxy` entry naming a host that is
/// merely a MEMBER of an allowed group (not the group name itself) would
/// silently slip through that check. This validator runs against
/// `plain_hosts` — the already-expanded host list — closing that gap.
fn validate_expanded_proxy_no_proxy_conflicts(
    no_proxy: &[String],
    plain_hosts: &[String],
) -> Result<()> {
    for no_proxy_entry in no_proxy {
        for host in plain_hosts {
            if nono_proxy::config::no_proxy_entry_overlaps_host_pattern(no_proxy_entry, host) {
                return Err(NonoError::ConfigParse(format!(
                    "no_proxy entry '{no_proxy_entry}' overlaps allow_domain host '{host}' \
                     (reached via network-policy group-name expansion) — a no_proxy bypass \
                     cannot cover a host explicitly allowed through the proxy (D-06)."
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn build_proxy_config_from_flags(
    proxy: &ProxyLaunchOptions,
) -> Result<nono_proxy::config::ProxyConfig> {
    // D-06 (#1415): reject a no_proxy entry that overlaps a literal
    // (pre-group-expansion) allow_domain entry before doing any further
    // work. The group-expanded overlap check (`validate_expanded_proxy_no_proxy_conflicts`)
    // runs later, once `plain_hosts` is available.
    validate_proxy_launch_no_proxy_conflicts(proxy)?;

    let net_policy_json = crate::config::embedded::embedded_network_policy_json();
    let net_policy = network_policy::load_network_policy(net_policy_json)?;

    let mut resolved = if let Some(ref profile_name) = proxy.network_profile {
        network_policy::resolve_network_profile(&net_policy, profile_name)?
    } else {
        network_policy::ResolvedNetworkPolicy {
            hosts: Vec::new(),
            suffixes: Vec::new(),
            routes: Vec::new(),
            profile_credentials: Vec::new(),
        }
    };

    let mut all_credentials = resolved.profile_credentials.clone();
    for cred in &proxy.credentials {
        if !all_credentials.contains(cred) {
            all_credentials.push(cred.clone());
        }
    }

    let routes = network_policy::resolve_credentials(
        &net_policy,
        &all_credentials,
        &proxy.custom_credentials,
    )?;
    resolved.routes = routes;

    // Partition allow_domain entries into plain hosts and endpoint-scoped routes.
    // C5 rider (22e6c40): also push endpoint route upstreams into plain_hosts so
    // the proxy filter allowlist allows upstream TCP connections for TLS-intercept routes.
    let (mut plain_hosts, endpoint_routes) =
        network_policy::partition_allow_domain(&net_policy, &proxy.allow_domain)?;
    for route in &endpoint_routes {
        if let Some(hp) = route.upstream.strip_prefix("https://") {
            plain_hosts.push(hp.to_string());
        } else if let Some(hp) = route.upstream.strip_prefix("http://") {
            plain_hosts.push(hp.to_string());
        }
    }
    resolved.routes.extend(endpoint_routes);

    // D-06 (#1415) / T-109-09: reject a no_proxy entry that overlaps a host
    // reachable only after allow_domain group-name expansion. Must run AFTER
    // `plain_hosts` is fully assembled (including endpoint-route upstream
    // hosts pushed in above) — the pre-expansion check above only sees
    // literal allow_domain strings/group names, not their expanded members.
    validate_expanded_proxy_no_proxy_conflicts(&proxy.no_proxy, &plain_hosts)?;

    // Expand deny_domain entries (group-name expansion, else literal
    // hostname) the same way allow_domain is expanded above.
    let denied_hosts = network_policy::expand_proxy_deny(&net_policy, &proxy.deny_domain);

    let mut proxy_config =
        network_policy::build_proxy_config(&resolved, &plain_hosts, &denied_hosts);
    // OR, not overwrite: build_proxy_config already applied ADR-108
    // Consequence (b)'s deny-only strict-selection; proxy.strict_filter
    // (block-net / profile network.block) must widen, never narrow, that
    // decision.
    proxy_config.strict_filter = proxy.strict_filter || proxy_config.strict_filter;
    proxy_config.enable_h2 = proxy.enable_h2;
    // D-06: profile-declared no_proxy entries (already overlap-validated
    // above) flow into ProxyConfig.no_proxy, where server.rs's
    // push_no_proxy_entry pipeline (Plan 109-02) emits them into the
    // generated NO_PROXY/no_proxy env var and re-validates them (grammar +
    // allowed-host overlap) fail-closed before any ProxyHandle is
    // constructed. Without this assignment, a validated no_proxy entry
    // would never actually reach the running proxy.
    proxy_config.no_proxy = proxy.no_proxy.clone();

    // Apply per-service endpoint restrictions from `--allow-endpoint` args.
    // Runs before domain-endpoint routes are merged so the prefix lookup
    // only matches credential routes (never `_ep_*` entries).
    // Errors if a specified service prefix is not found in active routes —
    // this prevents silently no-op restrictions (fail-secure; upstream 46bcfbb9).
    for (service_prefix, rule) in &proxy.endpoint_restrictions {
        let route = proxy_config
            .routes
            .iter_mut()
            .find(|r| r.prefix.trim_matches('/') == service_prefix.as_str())
            .ok_or_else(|| {
                nono::NonoError::ConfigParse(format!(
                    "--allow-endpoint: service '{}' not found in active credentials; \
                     ensure --credential {} is also specified",
                    service_prefix, service_prefix
                ))
            })?;
        route.endpoint_rules.push(rule.clone());
    }

    if let Some(ref addr) = proxy.upstream_proxy {
        proxy_config.external_proxy = Some(nono_proxy::config::ExternalProxyConfig {
            address: addr.clone(),
            auth: None,
            bypass_hosts: proxy.upstream_bypass.clone(),
        });
    }

    if let Some(port) = proxy.proxy_port {
        proxy_config.bind_port = port;
    }

    Ok(proxy_config)
}

/// Canonicalize a path (resolving symlinks such as `/var` -> `/private/var`
/// on macOS), falling back to the original, unresolved path if it (or a
/// parent directory) does not yet exist. Mirrors the canonicalize-or-fall-back
/// idiom `policy.rs::add_deny_access_rules` uses for socket paths that may
/// not exist at policy-computation time.
fn try_canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Collect every SPIRE Workload API socket path a resolved route set
/// configures — both the direct `RouteConfig.spiffe` auth path and the
/// `oauth2.client_assertion` (RFC 7523 SPIFFE-JWT-bearer) alternative. Both
/// shapes cause the proxy to open a connection to the local SPIRE agent
/// socket on the sandboxed child's behalf, so both need isolation.
fn collect_spiffe_socket_paths(routes: &[nono_proxy::config::RouteConfig]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for route in routes {
        if let Some(nono_proxy::config::SpiffeAuthConfig::Jwt {
            ref workload_api_socket,
            ..
        }) = route.spiffe
        {
            paths.push(PathBuf::from(workload_api_socket));
        }
        if let Some(ref oauth2) = route.oauth2 {
            if let Some(nono_proxy::config::ClientAssertionConfig::SpiffeJwt {
                ref workload_api_socket,
                ..
            }) = oauth2.client_assertion
            {
                paths.push(PathBuf::from(workload_api_socket));
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Deny the sandboxed child direct access to every configured SPIRE Workload
/// API socket path (NET-02, Phase 113; T-113-04). The proxy — not the
/// sandboxed child — holds the workload identity; the entire point of the
/// SPIFFE feature is that the child can never self-issue SVIDs by reaching
/// the agent socket directly.
///
/// - **Linux:** deny-by-omission. Landlock is strictly allow-list-only (see
///   CLAUDE.md's Linux platform note), so simply never issuing a
///   `unix_socket` grant for this path is a structural, not incidental,
///   isolation guarantee.
/// - **macOS:** an explicit Seatbelt `(deny network-outbound (path ...))`
///   rule, mirroring `policy.rs::add_deny_access_rules`'s existing Unix-socket
///   deny pattern — `connect(2)` on an AF_UNIX socket is enforced by
///   Seatbelt as network-outbound, not as a file operation, so a file-only
///   deny would not stop it.
/// - **Windows:** logged as a residual-risk no-op, not silently assumed
///   protected — AppContainer's default-deny behavior for unlisted
///   Unix-domain-socket paths was not independently verified this phase
///   (113-RESEARCH.md's Security Domain note; T-113-05, accepted residual
///   risk, confirmed/closed in a later phase's final verification pass).
///
/// Hard-errors (`NonoError`, never `warn!`) if the profile's OWN
/// `unix_socket` capability grants already cover the exact socket path —
/// that is a real conflict the operator must resolve explicitly, not
/// something this function may silently paper over or downgrade to a log
/// line (CLAUDE.md "Fail Secure").
pub(crate) fn enforce_spiffe_socket_isolation(
    caps: &mut CapabilitySet,
    routes: &[nono_proxy::config::RouteConfig],
) -> Result<()> {
    let socket_paths = collect_spiffe_socket_paths(routes);
    if socket_paths.is_empty() {
        return Ok(());
    }

    for socket_path in &socket_paths {
        let canonical = try_canonicalize(socket_path);

        if caps.unix_socket_allowed(&canonical, UnixSocketOp::Connect) {
            return Err(NonoError::ConfigParse(format!(
                "profile grants direct unix_socket access to '{}', which is also configured as \
                 a SPIFFE Workload API socket; the sandboxed child must never reach the SPIRE \
                 agent socket directly. Remove the conflicting unix_socket grant, or remove the \
                 spiffe/client_assertion route that uses this socket.",
                canonical.display()
            )));
        }

        if cfg!(target_os = "macos") {
            let utf8_path = crate::policy::path_to_utf8(&canonical)?;
            let escaped = crate::policy::escape_seatbelt_path(utf8_path)?;
            caps.add_platform_rule(format!("(deny network-outbound (path \"{}\"))", escaped))?;
        } else {
            // Linux: deny-by-omission is structural (Landlock allow-list-only,
            // no unix_socket grant issued). Windows: residual risk, not a
            // proven deny — see the doc comment above.
            debug!(
                "SPIFFE Workload API socket '{}' isolated from the sandboxed child by \
                 deny-by-omission (no unix_socket grant issued; Windows AppContainer coverage \
                 not independently verified — see T-113-05)",
                canonical.display()
            );
        }
    }

    Ok(())
}

/// Start the proxy runtime based on the resolved `NetworkIntent`.
///
/// Upstream 72bcfd66 (#1225): signature changed from `&ProxyLaunchOptions` to `&NetworkIntent`;
/// callers now pass the intent returned by `prepare_proxy_launch_options` directly.
/// Non-proxy intents (BlockAll, Unrestricted) return an inactive runtime without starting
/// a proxy server; ProxyFiltered with active=false also returns inactive.
pub(crate) fn start_proxy_runtime(
    intent: &NetworkIntent,
    caps: &mut CapabilitySet,
) -> Result<ActiveProxyRuntime> {
    let proxy = match intent {
        NetworkIntent::ProxyFiltered(opts) if opts.active => opts.as_ref(),
        _ => {
            return Ok(ActiveProxyRuntime {
                env_vars: Vec::new(),
                handle: None,
            });
        }
    };

    let mut proxy_config = build_proxy_config_from_flags(proxy)?;
    proxy_config.direct_connect_ports = caps.tcp_connect_ports().to_vec();
    // NET-02 (Phase 113, T-113-04): isolate every configured SPIRE Workload
    // API socket from the sandboxed child BEFORE the proxy (which needs to
    // reach that same socket) starts. Fails the launch (fail-secure) rather
    // than starting a proxy the child could bypass by connecting to the
    // agent socket itself.
    enforce_spiffe_socket_isolation(caps, &proxy_config.routes)?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| NonoError::SandboxInit(format!("Failed to start proxy runtime: {}", e)))?;
    let handle = rt
        .block_on(async { nono_proxy::server::start(proxy_config.clone()).await })
        .map_err(|e| NonoError::SandboxInit(format!("Failed to start proxy: {}", e)))?;

    let port = handle.port;
    if proxy.allow_bind_ports.is_empty() {
        info!("Network proxy started on localhost:{}", port);
    } else {
        info!(
            "Network proxy started on localhost:{}, bind ports: {:?}",
            port, proxy.allow_bind_ports
        );
    }

    // Per-route diagnostic banner. Lifts credential resolution status —
    // including misses — to the user-visible info level so the silent
    // "WARN at debug" failure mode (issue #797) becomes immediately
    // discoverable.
    let route_rows = handle.route_diagnostics(&proxy_config);
    if !route_rows.is_empty() {
        info!("Proxy routes:");
        for (prefix, summary) in &route_rows {
            info!("  /{}  {}", prefix, summary);
        }
        if handle.intercept_ca_path().is_some() {
            info!(
                "TLS interception trust bundle: {}",
                handle
                    .intercept_ca_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );
        }
    }

    let proxy_diagnostics = handle.diagnostics();
    if !proxy_diagnostics.is_empty() {
        crate::output::print_proxy_diagnostics(proxy_diagnostics);
    }
    caps.set_network_mode_mut(nono::NetworkMode::ProxyOnly {
        port,
        bind_ports: proxy.allow_bind_ports.clone(),
    });

    let mut env_vars: Vec<(String, String)> = Vec::new();
    for (key, value) in handle.env_vars() {
        env_vars.push((key, value));
    }

    for (key, value) in handle.credential_env_vars(&proxy_config) {
        env_vars.push((key, value));
    }

    std::mem::forget(rt);

    Ok(ActiveProxyRuntime {
        env_vars,
        handle: Some(handle),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::AllowDomainEntry;

    /// Upstream-commit: 75b2265 — parse_allow_domain_arg tests
    #[test]
    fn parse_allow_domain_plain_hostname() {
        let entry = parse_allow_domain_arg("api.openai.com");
        assert_eq!(entry, AllowDomainEntry::Plain("api.openai.com".to_string()));
    }

    #[test]
    fn parse_allow_domain_url_with_path_produces_with_endpoints() {
        let entry = parse_allow_domain_arg("https://api.github.com/repos/my-org/**");
        assert_eq!(
            entry,
            AllowDomainEntry::WithEndpoints {
                domain: "api.github.com".to_string(),
                endpoints: vec![nono_proxy::config::EndpointRule {
                    method: "*".to_string(),
                    path: "/repos/my-org/**".to_string(),
                }],
            }
        );
    }

    #[test]
    fn parse_allow_domain_url_with_root_path_produces_plain() {
        let entry = parse_allow_domain_arg("https://api.github.com/");
        assert_eq!(entry, AllowDomainEntry::Plain("api.github.com".to_string()));
    }

    #[test]
    fn parse_allow_domain_url_no_path_produces_plain() {
        let entry = parse_allow_domain_arg("https://api.github.com");
        assert_eq!(entry, AllowDomainEntry::Plain("api.github.com".to_string()));
    }

    #[test]
    fn parse_allow_domain_unparseable_input_falls_back_to_plain() {
        let entry = parse_allow_domain_arg("not-a-url");
        assert_eq!(entry, AllowDomainEntry::Plain("not-a-url".to_string()));
    }

    // CR-02 regression: `host:port` must NOT be mangled into a WithEndpoints
    // (url::Url::parse treats the host as the URL scheme for `host:port` inputs).
    #[test]
    fn parse_allow_domain_host_port_yields_plain() {
        let entry = parse_allow_domain_arg("api.openai.com:8080");
        assert_eq!(
            entry,
            AllowDomainEntry::Plain("api.openai.com:8080".to_string()),
            "host:port must parse as Plain, not WithEndpoints"
        );
    }

    #[test]
    fn parse_allow_domain_host_port_443_yields_plain() {
        let entry = parse_allow_domain_arg("api.github.com:443");
        assert_eq!(
            entry,
            AllowDomainEntry::Plain("api.github.com:443".to_string()),
            "host:port must parse as Plain, not WithEndpoints"
        );
    }

    // CR-01 regression: resolve_effective_proxy_settings must preserve structured
    // WithEndpoints entries from PreparedSandbox (no round-trip through string parsing).
    #[test]
    fn resolve_effective_proxy_settings_preserves_with_endpoints() {
        use crate::cli::SandboxArgs;
        use nono::CapabilitySet;

        let endpoint_entry = AllowDomainEntry::WithEndpoints {
            domain: "api.github.com".to_string(),
            endpoints: vec![
                nono_proxy::config::EndpointRule {
                    method: "GET".to_string(),
                    path: "/repos/**".to_string(),
                },
                nono_proxy::config::EndpointRule {
                    method: "POST".to_string(),
                    path: "/issues/**".to_string(),
                },
            ],
        };

        let prepared = PreparedSandbox {
            caps: CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: None,
            allow_domain: vec![endpoint_entry.clone()],
            deny_domain: Vec::new(),
            no_proxy: Vec::new(),
            credentials: Vec::new(),
            custom_credentials: std::collections::HashMap::new(),
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::default(),
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::default(),
            #[cfg(target_os = "linux")]
            proc_comm_notify: false,
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
            session_hooks: crate::profile::SessionHooks::default(),
            allow_http2_requested: false,
        };

        let args = SandboxArgs::default();
        let effective = resolve_effective_proxy_settings(&args, &prepared);

        assert_eq!(effective.allow_domain.len(), 1);
        assert_eq!(
            effective.allow_domain[0], endpoint_entry,
            "WithEndpoints entry must survive end-to-end without being flattened to Plain"
        );
    }

    /// #1374 / ADR-108: deny_domain composes with allow_domain end-to-end
    /// through build_proxy_config_from_flags into ProxyConfig.denied_hosts,
    /// without clearing or replacing the allowlist.
    #[test]
    fn build_proxy_config_propagates_deny_domain_with_allow_domain() {
        let proxy = ProxyLaunchOptions {
            active: true,
            allow_domain: vec![AllowDomainEntry::Plain("good.com".to_string())],
            deny_domain: vec!["evil.com".to_string()],
            ..ProxyLaunchOptions::default()
        };
        let config = build_proxy_config_from_flags(&proxy).expect("build_proxy_config_from_flags");
        assert!(config.allowed_hosts.contains(&"good.com".to_string()));
        assert!(config.denied_hosts.contains(&"evil.com".to_string()));
        // Allowlist is non-empty, so the ADR-108 (b) deny-only strict-selection
        // must NOT force strict_filter on here.
        assert!(!config.strict_filter);
    }

    // ========================================================================
    // Plan 109-03 / D-06 (#1415): no_proxy CLI-flag-layer overlap validators
    // ========================================================================

    /// D-06: a no_proxy entry that matches a LITERAL allow_domain entry
    /// (pre-group-expansion) is rejected by validate_proxy_launch_no_proxy_conflicts,
    /// called at the very start of build_proxy_config_from_flags.
    #[test]
    fn build_proxy_config_rejects_literal_no_proxy_allow_domain_overlap() {
        let proxy = ProxyLaunchOptions {
            active: true,
            allow_domain: vec![AllowDomainEntry::Plain("good.com".to_string())],
            no_proxy: vec!["good.com".to_string()],
            ..ProxyLaunchOptions::default()
        };
        let err = build_proxy_config_from_flags(&proxy)
            .expect_err("literal no_proxy/allow_domain overlap must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("no_proxy") && msg.contains("good.com"),
            "error should name the overlapping no_proxy entry and host: {msg}"
        );
    }

    /// D-06 non-negotiable regression test (verbatim name, upstream 1619275c):
    /// a no_proxy entry that overlaps a host reachable ONLY through
    /// allow_domain group-name expansion (not the literal group name string
    /// itself) is still rejected. "llm_apis" is a real group in the embedded
    /// network policy that expands to (among others) api.openai.com — the
    /// literal string "llm_apis" never appears in no_proxy, so a
    /// pre-expansion-only check (validate_proxy_launch_no_proxy_conflicts)
    /// would silently miss this overlap. This proves
    /// validate_expanded_proxy_no_proxy_conflicts runs AFTER
    /// network_policy::partition_allow_domain's group expansion.
    #[test]
    fn test_build_proxy_config_rejects_group_expanded_no_proxy_overlap() {
        let proxy = ProxyLaunchOptions {
            active: true,
            allow_domain: vec![AllowDomainEntry::Plain("llm_apis".to_string())],
            no_proxy: vec!["api.openai.com".to_string()],
            ..ProxyLaunchOptions::default()
        };
        let err = build_proxy_config_from_flags(&proxy)
            .expect_err("no_proxy entry overlapping a group-expanded host must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("no_proxy") && msg.contains("api.openai.com"),
            "error should name the overlapping no_proxy entry and expanded host: {msg}"
        );
    }

    /// A non-overlapping no_proxy/allow_domain(-group) pair builds
    /// successfully and the no_proxy entries flow through to
    /// `ProxyConfig.no_proxy` (D-06: without this assignment, a validated
    /// no_proxy entry would never actually reach the running proxy).
    #[test]
    fn build_proxy_config_propagates_non_overlapping_no_proxy() {
        let proxy = ProxyLaunchOptions {
            active: true,
            allow_domain: vec![AllowDomainEntry::Plain("llm_apis".to_string())],
            no_proxy: vec!["internal-only".to_string()],
            ..ProxyLaunchOptions::default()
        };
        let config = build_proxy_config_from_flags(&proxy).expect("build_proxy_config_from_flags");
        assert_eq!(config.no_proxy, vec!["internal-only".to_string()]);
    }

    /// `strict_filter: true` must propagate to `ProxyConfig.strict_filter`.
    /// Upstream 72bcfd66: field renamed from network_block to strict_filter on ProxyLaunchOptions.
    #[test]
    fn test_build_proxy_config_propagates_strict_filter() {
        let proxy = ProxyLaunchOptions {
            active: true,
            strict_filter: true,
            ..ProxyLaunchOptions::default()
        };
        let config = build_proxy_config_from_flags(&proxy).expect("build_proxy_config_from_flags");
        assert!(
            config.strict_filter,
            "strict_filter: true on ProxyLaunchOptions must set strict_filter on ProxyConfig"
        );
    }

    #[test]
    fn test_build_proxy_config_strict_filter_off_when_no_block() {
        let proxy = ProxyLaunchOptions {
            active: true,
            strict_filter: false,
            ..ProxyLaunchOptions::default()
        };
        let config = build_proxy_config_from_flags(&proxy).expect("build_proxy_config_from_flags");
        assert!(
            !config.strict_filter,
            "strict_filter must default off when strict_filter is false on ProxyLaunchOptions"
        );
    }

    /// D-01 equivalence: build_proxy_config_from_flags maps upstream_proxy to
    /// ProxyConfig.external_proxy (#1048/#1091). Fork uses the field name `external_proxy`
    /// where upstream uses `upstream_proxy`; the mapping is already present at
    /// build_proxy_config_from_flags lines 222-228. This test documents equivalence and
    /// confirms the cherry-pick of #1048/#1091 is unnecessary (verify-present).
    #[test]
    fn build_proxy_config_maps_upstream_proxy_to_external_proxy() {
        let proxy = crate::launch_runtime::ProxyLaunchOptions {
            active: true,
            upstream_proxy: Some("http://corp:3128".into()),
            ..crate::launch_runtime::ProxyLaunchOptions::default()
        };
        let config = build_proxy_config_from_flags(&proxy).expect("build_proxy_config_from_flags");
        let ext = config
            .external_proxy
            .expect("external_proxy must be Some when upstream_proxy is set");
        assert_eq!(
            ext.address, "http://corp:3128",
            "upstream_proxy must map to external_proxy.address (#1048/#1091 / D-01)"
        );
    }

    /// D-07 regression: proxy activates when only customCredentials is configured (#1197).
    /// CapabilitySet::new() defaults to NetworkMode::Open (not Blocked/ProxyOnly) so only
    /// the custom_credentials disjunct can trigger active=true here.
    #[test]
    fn proxy_activates_with_custom_credentials_only() {
        use crate::cli::SandboxArgs;
        use nono::CapabilitySet;

        let mut custom_creds = std::collections::HashMap::new();
        custom_creds.insert(
            "my_api".to_string(),
            crate::profile::CustomCredentialDef {
                spiffe: None,
                capture: None,
                upstream: "https://api.example.com".to_string(),
                credential_key: Some("my_api_key".to_string()),
                auth: None,
                inject_mode: crate::profile::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                aws_auth: None,
            },
        );

        let prepared = crate::sandbox_prepare::PreparedSandbox {
            caps: CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: None,
            allow_domain: vec![],
            deny_domain: Vec::new(),
            no_proxy: Vec::new(),
            credentials: Vec::new(),
            custom_credentials: custom_creds,
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::default(),
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::default(),
            #[cfg(target_os = "linux")]
            proc_comm_notify: false,
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
            session_hooks: crate::profile::SessionHooks::default(),
            allow_http2_requested: false,
        };

        let args = SandboxArgs::default();
        let intent = prepare_proxy_launch_options(&args, &prepared, true)
            .expect("prepare_proxy_launch_options");
        assert!(
            intent.is_proxy_active(),
            "proxy must activate when only custom_credentials is set (#1197/D-07)"
        );
    }

    /// ADR-108 Consequence (c) / T-109-17: a config with BOTH deny_domain
    /// AND allow_domain non-empty produces `NetworkIntent::ProxyFiltered`
    /// with `active: true` — activation comes from the `!allow_domain.is_empty()`
    /// term in `would_activate`, never from `deny_domain`'s mere presence
    /// (which is deliberately excluded from that OR-chain).
    #[test]
    fn deny_domain_with_allow_domain_activates_proxy() {
        use crate::cli::SandboxArgs;
        use nono::CapabilitySet;

        let prepared = crate::sandbox_prepare::PreparedSandbox {
            caps: CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: None,
            allow_domain: vec![AllowDomainEntry::Plain("good.com".to_string())],
            deny_domain: vec!["evil.com".to_string()],
            no_proxy: Vec::new(),
            credentials: Vec::new(),
            custom_credentials: std::collections::HashMap::new(),
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::default(),
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::default(),
            #[cfg(target_os = "linux")]
            proc_comm_notify: false,
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
            session_hooks: crate::profile::SessionHooks::default(),
            allow_http2_requested: false,
        };

        let args = SandboxArgs {
            deny_proxy: vec!["extra-evil.com".to_string()],
            ..SandboxArgs::default()
        };
        let intent = prepare_proxy_launch_options(&args, &prepared, true)
            .expect("prepare_proxy_launch_options");
        assert!(
            intent.is_proxy_active(),
            "proxy must activate for a deny+allow_domain config, via the allow_domain term \
             (ADR-108 Consequence (c) / T-109-17)"
        );
        let opts = intent
            .proxy_options()
            .expect("intent must be ProxyFiltered");
        assert!(
            opts.deny_domain.contains(&"evil.com".to_string())
                && opts.deny_domain.contains(&"extra-evil.com".to_string()),
            "deny_domain must carry both profile and CLI entries through into ProxyLaunchOptions"
        );
    }

    /// ADR-108 Consequence (c) / D-04 ordering: a deny-only state (no
    /// allow_domain) must never reach `prepare_proxy_launch_options` at all
    /// — `validate_deny_domain_requires_allow_domain` rejects it first on
    /// both `nono run` call sites. This test documents that ordering
    /// directly: the validator call precedes the `prepare_proxy_launch_options`
    /// / proxy-preparation call in both `command_runtime.rs`'s dry-run
    /// branch and `launch_runtime.rs::prepare_run_launch_plan` (source-order
    /// verified by the acceptance-criteria grep in 109-01-PLAN.md), and here
    /// we assert the validator itself actually rejects the deny-only state
    /// that would otherwise reach this function.
    #[test]
    fn deny_only_state_is_rejected_before_reaching_prepare_proxy_launch_options() {
        use crate::cli::SandboxArgs;
        use crate::sandbox_prepare::validate_deny_domain_requires_allow_domain;

        let args = SandboxArgs {
            deny_proxy: vec!["evil.com".to_string()],
            ..SandboxArgs::default()
        };
        let prepared = crate::sandbox_prepare::PreparedSandbox {
            caps: nono::CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: None,
            allow_domain: Vec::new(),
            deny_domain: Vec::new(),
            no_proxy: Vec::new(),
            credentials: Vec::new(),
            custom_credentials: std::collections::HashMap::new(),
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::default(),
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::default(),
            #[cfg(target_os = "linux")]
            proc_comm_notify: false,
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
            session_hooks: crate::profile::SessionHooks::default(),
            allow_http2_requested: false,
        };

        // The guard rejects this state before either nono-run call site
        // would reach prepare_proxy_launch_options.
        assert!(
            validate_deny_domain_requires_allow_domain(&args, &prepared).is_err(),
            "deny-only state must be rejected by the D-04 guard before it can reach \
             prepare_proxy_launch_options / would_activate"
        );
    }

    /// D-08 deviation test: CompiledEndpointPolicy compatibility.
    ///
    /// After upstream commit 72bcfd66 (#1225 NetworkIntent) adoption,
    /// `CompiledEndpointPolicy::compile() → evaluate()` chain must remain intact
    /// in `proxy_runtime.rs` context — this is a fork-specific extension from Phase 95.
    ///
    /// The `denied_endpoint_returns_403_and_audit` integration test in
    /// `crates/nono-proxy/src/reverse.rs` covers the HTTP-level 403 response;
    /// this unit test documents the direct type-accessibility invariant from within
    /// proxy_runtime context.
    ///
    /// Fork deviation from 72bcfd66: CompiledEndpointPolicy chain preserved
    /// (Phase 95 / ADR-98 deviation 2)
    #[test]
    fn test_compiled_endpoint_policy_compat_deviation_preserved() {
        use nono_proxy::config::{
            CompiledEndpointPolicy, EndpointPolicyConfig, EndpointPolicyDecision,
            EndpointPolicyDefault, EndpointPolicyOutcome, EndpointPolicyRule,
        };

        // Build a policy with a deny rule to verify the compile → evaluate chain
        // produces the expected Deny outcome for a denied endpoint.
        let deny_rule = EndpointPolicyRule {
            method: "*".to_string(),
            path: "/restricted/**".to_string(),
            backend: None,
            reason: Some("Endpoint restricted by fork endpoint policy".to_string()),
            timeout_secs: None,
        };
        let policy = EndpointPolicyConfig {
            default: EndpointPolicyDefault {
                decision: EndpointPolicyDecision::Allow,
                backend: None,
                timeout_secs: None,
            },
            deny: vec![deny_rule],
            approve: Vec::new(),
            allow: Vec::new(),
        };

        let compiled = CompiledEndpointPolicy::compile(Some(&policy), &[])
            .expect("CompiledEndpointPolicy::compile must succeed (ADR-98 D-08)");

        let denied = compiled.evaluate("GET", "/restricted/secret");
        assert!(
            matches!(denied, EndpointPolicyOutcome::Deny { .. }),
            "CompiledEndpointPolicy::evaluate must produce Deny for a denied path \
             (CEP chain intact post-72bcfd66 replay)"
        );

        let allowed = compiled.evaluate("GET", "/public/resource");
        assert!(
            matches!(allowed, EndpointPolicyOutcome::Allow { .. }),
            "CompiledEndpointPolicy::evaluate must produce Allow for a non-denied path"
        );
    }

    /// D-07 regression: --block-net overrides customCredentials activation; active=false (#1197).
    /// Upstream 72bcfd66: block detection now via args.block_net (not NetworkMode::Blocked cap).
    #[test]
    fn block_net_overrides_custom_credentials_activation() {
        use crate::cli::SandboxArgs;
        use nono::CapabilitySet;

        let mut custom_creds = std::collections::HashMap::new();
        custom_creds.insert(
            "my_api".to_string(),
            crate::profile::CustomCredentialDef {
                spiffe: None,
                capture: None,
                upstream: "https://api.example.com".to_string(),
                credential_key: Some("my_api_key".to_string()),
                auth: None,
                inject_mode: crate::profile::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                aws_auth: None,
            },
        );

        let prepared = crate::sandbox_prepare::PreparedSandbox {
            caps: CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: None,
            allow_domain: vec![],
            deny_domain: Vec::new(),
            no_proxy: Vec::new(),
            credentials: Vec::new(),
            custom_credentials: custom_creds,
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::default(),
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::default(),
            #[cfg(target_os = "linux")]
            proc_comm_notify: false,
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
            session_hooks: crate::profile::SessionHooks::default(),
            allow_http2_requested: false,
        };

        // Upstream 72bcfd66: block_wins now driven by args.block_net (flag) not NetworkMode.
        let args = SandboxArgs {
            block_net: true,
            ..SandboxArgs::default()
        };
        let intent = prepare_proxy_launch_options(&args, &prepared, true)
            .expect("prepare_proxy_launch_options");
        assert!(
            !intent.is_proxy_active(),
            "--block-net must override custom_credentials activation; proxy must not be active (#1197/D-07)"
        );
    }

    // ── parse_allow_endpoint_arg tests (46bcfbb9) ────────────────────────────

    #[test]
    fn test_parse_allow_endpoint_arg_valid() {
        let (service, rule) =
            parse_allow_endpoint_arg("github:GET:/repos/*/issues").expect("should parse");
        assert_eq!(service, "github");
        assert_eq!(rule.method, "GET");
        assert_eq!(rule.path, "/repos/*/issues");
    }

    #[test]
    fn test_parse_allow_endpoint_arg_wildcard_method() {
        let (service, rule) =
            parse_allow_endpoint_arg("openai:*:/v1/chat/completions").expect("should parse");
        assert_eq!(service, "openai");
        assert_eq!(rule.method, "*");
        assert_eq!(rule.path, "/v1/chat/completions");
    }

    #[test]
    fn test_parse_allow_endpoint_arg_path_must_start_with_slash() {
        let result = parse_allow_endpoint_arg("github:GET:repos/no-leading-slash");
        assert!(result.is_err());
        let err = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            err.contains("must start with '/'"),
            "error should explain the leading slash requirement, got: {err}"
        );
    }

    #[test]
    fn test_parse_allow_endpoint_arg_missing_parts() {
        assert!(parse_allow_endpoint_arg("github:GET").is_err());
        assert!(parse_allow_endpoint_arg("github").is_err());
        assert!(parse_allow_endpoint_arg("").is_err());
    }

    fn ep(service: &str, method: &str, path: &str) -> (String, nono_proxy::config::EndpointRule) {
        (
            service.to_string(),
            nono_proxy::config::EndpointRule {
                method: method.to_string(),
                path: path.to_string(),
            },
        )
    }

    #[test]
    fn test_allow_endpoint_applied_to_credential_route() {
        let proxy = ProxyLaunchOptions {
            credentials: vec!["github".to_string()],
            endpoint_restrictions: vec![
                ep("github", "GET", "/repos/*/issues"),
                ep("github", "POST", "/repos/*/issues/*/comments"),
            ],
            ..ProxyLaunchOptions::default()
        };
        let config = build_proxy_config_from_flags(&proxy).expect("build");
        let github = config
            .routes
            .iter()
            .find(|r| r.prefix == "github")
            .expect("github route must exist");
        assert_eq!(github.endpoint_rules.len(), 2);
        assert_eq!(github.endpoint_rules[0].method, "GET");
        assert_eq!(github.endpoint_rules[0].path, "/repos/*/issues");
        assert_eq!(github.endpoint_rules[1].method, "POST");
        assert_eq!(github.endpoint_rules[1].path, "/repos/*/issues/*/comments");
    }

    #[test]
    fn test_allow_endpoint_does_not_affect_other_routes() {
        let proxy = ProxyLaunchOptions {
            credentials: vec!["github".to_string(), "openai".to_string()],
            endpoint_restrictions: vec![ep("github", "GET", "/repos/*/issues")],
            ..ProxyLaunchOptions::default()
        };
        let config = build_proxy_config_from_flags(&proxy).expect("build");
        let openai = config
            .routes
            .iter()
            .find(|r| r.prefix == "openai")
            .expect("openai route must exist");
        assert!(
            openai.endpoint_rules.is_empty(),
            "openai route should not have endpoint rules when only github was restricted"
        );
    }

    #[test]
    fn test_allow_endpoint_unknown_service_errors() {
        let proxy = ProxyLaunchOptions {
            credentials: vec!["github".to_string()],
            endpoint_restrictions: vec![ep("nonexistent", "GET", "/path")],
            ..ProxyLaunchOptions::default()
        };
        let result = build_proxy_config_from_flags(&proxy);
        assert!(result.is_err());
        let err = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            err.contains("nonexistent"),
            "error should name the unknown service, got: {}",
            err
        );
    }

    #[test]
    fn test_allow_endpoint_no_credential_errors() {
        let proxy = ProxyLaunchOptions {
            credentials: Vec::new(),
            endpoint_restrictions: vec![ep("github", "GET", "/repos")],
            ..ProxyLaunchOptions::default()
        };
        let result = build_proxy_config_from_flags(&proxy);
        assert!(
            result.is_err(),
            "--allow-endpoint for a service without --credential must error"
        );
    }

    // ========================================================================
    // NET-02 (Phase 113, T-113-04) — enforce_spiffe_socket_isolation
    // ========================================================================

    fn spiffe_route(socket: &str) -> nono_proxy::config::RouteConfig {
        nono_proxy::config::RouteConfig {
            prefix: "inventory".to_string(),
            upstream: "https://inventory.internal.example".to_string(),
            credential_key: None,
            inject_mode: nono_proxy::config::InjectMode::Header,
            inject_header: "Authorization".to_string(),
            credential_format: None,
            path_pattern: None,
            path_replacement: None,
            query_param_name: None,
            env_var: None,
            endpoint_rules: vec![],
            tls_ca: None,
            oauth2: None,
            aws_auth: None,
            spiffe: Some(nono_proxy::config::SpiffeAuthConfig::Jwt {
                workload_api_socket: socket.to_string(),
                audience: vec!["inventory.internal.example".to_string()],
                inject_header: "Authorization".to_string(),
                credential_format: None,
                svid_hint: None,
            }),
            capture: None,
            endpoint_policy: None,
        }
    }

    #[test]
    fn test_enforce_spiffe_socket_isolation_noop_when_no_spiffe_routes() {
        let mut caps = CapabilitySet::new();
        let routes: Vec<nono_proxy::config::RouteConfig> = Vec::new();
        assert!(enforce_spiffe_socket_isolation(&mut caps, &routes).is_ok());
        assert!(caps.unix_socket_capabilities().is_empty());
        assert!(caps.platform_rules().is_empty());
    }

    #[test]
    fn test_enforce_spiffe_socket_isolation_never_grants_the_socket() {
        let mut caps = CapabilitySet::new();
        let routes = vec![spiffe_route("/run/spire/sockets/agent.sock")];
        assert!(enforce_spiffe_socket_isolation(&mut caps, &routes).is_ok());
        // Deny-by-omission: no unix_socket capability was added for the
        // SPIRE agent socket, so the sandboxed child has no grant path to it.
        assert!(
            !caps.unix_socket_allowed(
                Path::new("/run/spire/sockets/agent.sock"),
                UnixSocketOp::Connect
            ),
            "the SPIRE agent socket must never be reachable through a granted capability"
        );
    }

    #[test]
    fn test_enforce_spiffe_socket_isolation_hard_errors_on_conflicting_grant() {
        let mut caps = CapabilitySet::new();
        // Simulate a profile that (incorrectly) also grants direct unix_socket
        // access to the same path used as a SPIRE Workload API socket. Use a
        // path that exists on every CI/dev host so `UnixSocketCapability::new_file`
        // (which requires the path to exist for Connect mode) succeeds.
        let existing_file = std::env::current_exe().expect("current_exe");
        let cap =
            nono::UnixSocketCapability::new_file(&existing_file, nono::UnixSocketMode::Connect)
                .expect("build unix socket capability");
        caps.add_unix_socket(cap);

        let socket_str = existing_file.to_string_lossy().into_owned();
        let routes = vec![spiffe_route(&socket_str)];

        let result = enforce_spiffe_socket_isolation(&mut caps, &routes);
        assert!(
            result.is_err(),
            "a conflicting explicit unix_socket grant must hard-error, not silently degrade"
        );
        let err = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            err.contains("conflict") || err.contains("unix_socket"),
            "error should explain the conflict, got: {}",
            err
        );
    }
}
