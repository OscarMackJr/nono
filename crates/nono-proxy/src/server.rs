//! Proxy server: TCP listener, connection dispatch, and lifecycle.
//!
//! The server binds to `127.0.0.1:0` (OS-assigned port), accepts TCP
//! connections, reads the first HTTP line to determine the mode, and
//! dispatches to the appropriate handler.
//!
//! CONNECT method -> [`connect`] or [`external`] handler
//! Other methods  -> [`reverse`] handler (credential injection)

use crate::audit;
use crate::config::ProxyConfig;
use crate::connect;
use crate::credential::CredentialStore;
use crate::error::{ProxyError, Result};
use crate::external;
use crate::filter::ProxyFilter;
use crate::reverse;
use crate::route::{self, RouteStore};
use crate::token;
use rustls::ClientConfig;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{debug, info, warn};
use zeroize::Zeroizing;

/// Maximum total size of HTTP headers (64 KiB). Prevents OOM from
/// malicious clients sending unbounded header data.
const MAX_HEADER_SIZE: usize = 64 * 1024;

/// Handle returned when the proxy server starts.
///
/// Contains the assigned port, session token, and a shutdown channel.
/// Drop the handle or send to `shutdown_tx` to stop the proxy.
pub struct ProxyHandle {
    /// The actual port the proxy is listening on
    pub port: u16,
    /// Session token for client authentication
    pub token: Zeroizing<String>,
    /// Shared in-memory network audit log
    audit_log: audit::SharedAuditLog,
    /// Send `true` to trigger graceful shutdown
    shutdown_tx: watch::Sender<bool>,
    /// Route prefixes that have credentials actually loaded.
    /// Routes whose credentials were unavailable are excluded so we
    /// don't inject phantom tokens that shadow valid external credentials.
    loaded_routes: std::collections::HashSet<String>,
    /// Client-side proxy bypass entries appended after loopback defaults.
    /// Computed at startup from direct-connect bypasses and profile-declared
    /// `no_proxy` entries, excluding route upstreams.
    no_proxy_hosts: Vec<String>,
    /// When true, loopback must not appear in `NO_PROXY` because a managed
    /// credential route targets a loopback upstream host. This fork does not
    /// yet detect that condition (no loopback-credential-route mechanism has
    /// been absorbed) so `start()` always sets this `false`; the field
    /// exists so `env_vars()` carries the same semantics upstream does and
    /// so a future absorb can wire detection without another env_vars()
    /// rewrite.
    managed_loopback_upstream: bool,
    /// Canonical nono-owned bypass patterns for wrappers/SDKs that need to
    /// translate profile intent to non-env proxy surfaces such as Java
    /// `http.nonProxyHosts`. Emitted as `NONO_NO_PROXY`.
    canonical_no_proxy_hosts: Vec<String>,
    /// Startup diagnostics from credential loading (empty in this fork — no
    /// load_with_diagnostics integration yet).
    diagnostics: Vec<crate::diagnostic::ProxyDiagnostic>,
}

impl ProxyHandle {
    /// Startup diagnostics from credential loading.
    #[must_use]
    pub fn diagnostics(&self) -> &[crate::diagnostic::ProxyDiagnostic] {
        &self.diagnostics
    }

    /// Path to the TLS interception CA bundle, if TLS intercept is active.
    /// Returns `None` in this fork (TLS interception not implemented).
    #[must_use]
    pub fn intercept_ca_path(&self) -> Option<&std::path::Path> {
        None
    }

    /// Per-route summary rows for the proxy startup banner.
    ///
    /// Returns `(prefix, summary)` pairs. This is a simplified implementation
    /// that omits OAuth2/TLS intercept fields not present in this fork.
    #[must_use]
    pub fn route_diagnostics(&self, config: &crate::config::ProxyConfig) -> Vec<(String, String)> {
        let mut rows = Vec::with_capacity(config.routes.len());
        for route in &config.routes {
            let prefix = route.prefix.trim_matches('/').to_string();
            let cred_status = if self.loaded_routes.contains(&prefix) {
                "cred: ok"
            } else if route.credential_key.is_some() {
                "cred: missing"
            } else {
                "cred: none"
            };
            let summary = format!("→ {} | {}", route.upstream, cred_status);
            rows.push((prefix, summary));
        }
        rows
    }

    /// Signal the proxy to shut down gracefully.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Drain and return collected network audit events.
    #[must_use]
    pub fn drain_audit_events(&self) -> Vec<nono::undo::NetworkAuditEvent> {
        audit::drain_audit_events(&self.audit_log)
    }

    /// Environment variables to inject into the child process.
    ///
    /// The proxy URL includes `nono:<token>@` userinfo so that standard HTTP
    /// clients (curl, Python requests, etc.) automatically send
    /// `Proxy-Authorization: Basic ...` on every request. The raw token is
    /// also provided via `NONO_PROXY_TOKEN` for nono-aware clients that
    /// prefer Bearer auth.
    #[must_use]
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let proxy_url = format!("http://nono:{}@127.0.0.1:{}", &*self.token, self.port);

        // Build NO_PROXY: include loopback unless a managed credential route
        // targets a loopback upstream (those must traverse the proxy). Add
        // startup-filtered bypass entries via the push_no_proxy_entry
        // pipeline (D-07: replaces the static
        // `vec!["localhost", "127.0.0.1"]` seed — see
        // no_proxy_pipeline_preserves_localhost_and_loopback_bypass for the
        // regression proof that localhost/127.0.0.1 still reach NO_PROXY).
        let mut no_proxy_parts = Vec::new();
        let mut canonical_no_proxy_parts = Vec::new();
        push_no_proxy_entry(&mut no_proxy_parts, "localhost");
        push_no_proxy_entry(&mut no_proxy_parts, "127.0.0.1");
        push_canonical_no_proxy_entry(&mut canonical_no_proxy_parts, "localhost");
        push_canonical_no_proxy_entry(&mut canonical_no_proxy_parts, "127.0.0.1");
        if self.managed_loopback_upstream {
            no_proxy_parts.clear();
            canonical_no_proxy_parts.clear();
        }
        for host in &self.no_proxy_hosts {
            push_no_proxy_entry(&mut no_proxy_parts, host);
        }
        for host in &self.canonical_no_proxy_hosts {
            push_canonical_no_proxy_entry(&mut canonical_no_proxy_parts, host);
        }
        let no_proxy = no_proxy_parts.join(",");
        let nono_no_proxy = canonical_no_proxy_parts.join(",");

        let mut vars = vec![
            ("HTTP_PROXY".to_string(), proxy_url.clone()),
            ("HTTPS_PROXY".to_string(), proxy_url.clone()),
            ("NO_PROXY".to_string(), no_proxy.clone()),
            ("NONO_NO_PROXY".to_string(), nono_no_proxy),
            ("NONO_PROXY_TOKEN".to_string(), self.token.to_string()),
        ];

        // Lowercase variants for compatibility
        vars.push(("http_proxy".to_string(), proxy_url.clone()));
        vars.push(("https_proxy".to_string(), proxy_url));
        vars.push(("no_proxy".to_string(), no_proxy));

        // Node.js 20.6+ needs an explicit hint to use HTTPS_PROXY for built-in
        // fetch(). Without it, Node-based clients can bypass the proxy and hit
        // the sandboxed network directly.
        // NODE_USE_ENV_PROXY tells Node's built-in fetch() to read HTTPS_PROXY
        // from the environment.
        // Harmless to non-Node runtimes — they ignore unknown env vars.
        vars.push(("NODE_USE_ENV_PROXY".to_string(), "1".to_string()));

        vars
    }

    /// Environment variables for reverse proxy credential routes.
    ///
    /// Returns two types of env vars per route:
    /// 1. SDK base URL overrides (e.g., `OPENAI_BASE_URL=http://127.0.0.1:PORT/openai`)
    /// 2. SDK API key vars set to the session token (e.g., `OPENAI_API_KEY=<token>`)
    ///
    /// The SDK sends the session token as its "API key" (phantom token pattern).
    /// The proxy validates this token and swaps it for the real credential.
    #[must_use]
    pub fn credential_env_vars(&self, config: &ProxyConfig) -> Vec<(String, String)> {
        let mut vars = Vec::new();
        for route in &config.routes {
            // Strip any leading or trailing '/' from the prefix — prefix should
            // be a bare service name (e.g., "anthropic"), not a URL path.
            // Defensively handle both forms to prevent malformed env var names
            // and double-slashed URLs.
            let prefix = route.prefix.trim_matches('/');

            // Base URL override (e.g., OPENAI_BASE_URL)
            let base_url_name = format!("{}_BASE_URL", prefix.to_uppercase());
            let url = format!("http://127.0.0.1:{}/{}", self.port, prefix);
            vars.push((base_url_name, url));

            // Only inject phantom token env vars for routes whose credentials
            // were actually loaded. If a credential was unavailable (e.g.,
            // GITHUB_TOKEN env var not set), injecting a phantom token would
            // shadow valid credentials from other sources (keyring, gh auth).
            if !self.loaded_routes.contains(prefix) {
                continue;
            }

            // API key set to session token (phantom token pattern).
            // Use explicit env_var if set (required for URI manager refs), otherwise
            // fall back to uppercasing the credential_key (e.g., "openai_api_key" -> "OPENAI_API_KEY").
            if let Some(ref env_var) = route.env_var {
                vars.push((env_var.clone(), self.token.to_string()));
            } else if let Some(ref cred_key) = route.credential_key {
                let api_key_name = cred_key.to_uppercase();
                vars.push((api_key_name, self.token.to_string()));
            }
        }
        vars
    }
}

// ============================================================================
// no_proxy (#1415, D-06/D-07) — push-based NO_PROXY/NONO_NO_PROXY pipeline,
// route-conflict guards, and startup validation. Ported from upstream
// 1619275c and adapted to this fork's simpler `ProxyHandle`/`RouteStore`
// shape (no TLS intercept, no SPIFFE, no async RouteStore::load).
// ============================================================================

/// Append `entry` to `entries` (normalised, deduped) for the client-facing
/// `NO_PROXY`/`no_proxy` env vars.
fn push_no_proxy_entry(entries: &mut Vec<String>, entry: &str) {
    let env_entry = crate::config::normalise_no_proxy_env_entry(entry);
    let normalised = crate::config::normalise_no_proxy_host_pattern(&env_entry);
    if !entries
        .iter()
        .any(|existing| crate::config::normalise_no_proxy_host_pattern(existing) == normalised)
    {
        entries.push(env_entry);
    }
}

/// Append `entry` to `entries` (normalised, deduped) for the nono-owned
/// `NONO_NO_PROXY` env var consumed by wrappers/SDKs that need a canonical
/// (non-CSV, non-`.`-prefixed) bypass pattern representation.
fn push_canonical_no_proxy_entry(entries: &mut Vec<String>, entry: &str) {
    let canonical = canonical_no_proxy_entry(entry);
    let normalised = crate::config::normalise_no_proxy_host_pattern(&canonical);
    if !entries
        .iter()
        .any(|existing| crate::config::normalise_no_proxy_host_pattern(existing) == normalised)
    {
        entries.push(canonical);
    }
}

fn canonical_no_proxy_entry(entry: &str) -> String {
    let host = crate::config::strip_no_proxy_port(entry);
    let normalised = host.trim().to_ascii_lowercase();
    if let Some(suffix) = normalised.strip_prefix("*.") {
        format!("*.{suffix}")
    } else {
        crate::config::normalise_no_proxy_env_entry(&normalised)
    }
}

/// Validate a smart-derived (direct-connect-port) NO_PROXY candidate before
/// it is emitted. Smart entries are inferred from the sandbox's own
/// direct-connect grants, not operator-authored, but must still satisfy the
/// same grammar as a profile `no_proxy` entry — a wildcard allowlist host
/// (e.g. `*.example.com` in `allowed_hosts`) cannot be turned into a NO_PROXY
/// bypass without silently broadening it to a bare-domain suffix match.
fn smart_no_proxy_entry(host: &str) -> Option<String> {
    let entry = crate::config::strip_no_proxy_port(host);
    if entry.trim().to_ascii_lowercase().starts_with("*.") {
        debug!(
            "Skipping smart no_proxy entry {:?}: wildcard allowlist entries cannot be emitted without broadening to a bare-domain NO_PROXY bypass",
            host
        );
        return None;
    }
    if crate::config::validate_no_proxy_entry(&entry).is_ok() {
        Some(crate::config::normalise_no_proxy_env_entry(&entry))
    } else {
        debug!(
            "Skipping smart no_proxy entry {:?}: unsafe or ambiguous NO_PROXY bypass semantics",
            host
        );
        None
    }
}

/// Merge smart-derived and profile-declared no_proxy entries into the final
/// `NO_PROXY` bypass list, dropping any entry that overlaps a route upstream
/// (route traffic must stay on the proxy path for L7 filtering / credential
/// injection — a route entry here would be a bypass of that protection).
fn merge_no_proxy_hosts(
    smart_no_proxy_hosts: &[String],
    profile_no_proxy: &[String],
    route_hosts: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut merged = Vec::new();
    for entry in smart_no_proxy_hosts {
        if no_proxy_entry_matches_any_route(entry, route_hosts) {
            debug!(
                "Skipping smart no_proxy entry {:?}: it matches a proxy route upstream",
                entry
            );
            continue;
        }
        push_no_proxy_entry(&mut merged, entry);
    }

    for entry in profile_no_proxy {
        if no_proxy_entry_matches_any_route(entry, route_hosts) {
            debug!(
                "Skipping no_proxy entry {:?}: it matches a proxy route upstream",
                entry
            );
            continue;
        }
        push_no_proxy_entry(&mut merged, entry);
    }

    merged
}

/// Canonical-form counterpart to `merge_no_proxy_hosts`, feeding
/// `NONO_NO_PROXY`.
fn merge_canonical_no_proxy_hosts(
    smart_no_proxy_hosts: &[String],
    profile_no_proxy: &[String],
    route_hosts: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut merged = Vec::new();
    for entry in smart_no_proxy_hosts {
        if no_proxy_entry_matches_any_route(entry, route_hosts) {
            continue;
        }
        push_canonical_no_proxy_entry(&mut merged, entry);
    }

    for entry in profile_no_proxy {
        if no_proxy_entry_matches_any_route(entry, route_hosts) {
            continue;
        }
        push_canonical_no_proxy_entry(&mut merged, entry);
    }

    merged
}

/// Validate `config.no_proxy` before any `ProxyHandle` is constructed:
/// every entry must be a well-formed host pattern (D-06
/// `validate_no_proxy_entry`) and must not overlap `allowed_hosts` (a
/// no_proxy entry overlapping an allowed host would let a client silently
/// skip the proxy filter for traffic the operator explicitly allowlisted).
#[must_use = "no_proxy proxy config validation result must be handled"]
fn validate_no_proxy_config(config: &ProxyConfig) -> Result<()> {
    for entry in &config.no_proxy {
        crate::config::validate_no_proxy_entry(entry).map_err(|err| match err {
            ProxyError::Config(message) => {
                ProxyError::Config(format!("invalid no_proxy entry '{entry}': {message}"))
            }
            other => other,
        })?;
    }
    validate_no_proxy_allowed_host_conflicts(&config.no_proxy, &config.allowed_hosts)
}

/// Reject any `no_proxy` entry that overlaps a configured route upstream:
/// route traffic must go through the proxy for L7 filtering and/or
/// credential injection, so bypassing it is never valid regardless of what
/// the operator intended.
#[must_use = "no_proxy route conflict validation result must be handled"]
fn validate_no_proxy_route_conflicts(
    no_proxy: &[String],
    route_hosts: &std::collections::HashSet<String>,
) -> Result<()> {
    for no_proxy_entry in no_proxy {
        for route_host in route_hosts {
            if no_proxy_entry_matches_route(no_proxy_entry, route_host) {
                return Err(ProxyError::Config(format!(
                    "no_proxy entry '{no_proxy_entry}' conflicts with route upstream '{route_host}': configured route traffic must go through the proxy, not bypass it"
                )));
            }
        }
    }
    Ok(())
}

/// Reject any `no_proxy` entry that overlaps an `allowed_hosts` entry (D-06):
/// without this guard a `no_proxy` value could silently defeat the
/// allow_domain filter, which is exactly the security regression D-06 exists
/// to prevent.
#[must_use = "no_proxy allowed_host conflict validation result must be handled"]
fn validate_no_proxy_allowed_host_conflicts(
    no_proxy: &[String],
    allowed_hosts: &[String],
) -> Result<()> {
    for no_proxy_entry in no_proxy {
        for allowed_host in allowed_hosts {
            if crate::config::no_proxy_entry_overlaps_host_pattern(no_proxy_entry, allowed_host) {
                return Err(ProxyError::Config(format!(
                    "no_proxy entry '{no_proxy_entry}' conflicts with allowed_host '{allowed_host}': proxy-allowed traffic must go through the proxy filter, not bypass it"
                )));
            }
        }
    }
    Ok(())
}

fn no_proxy_entry_matches_any_route(
    entry: &str,
    route_hosts: &std::collections::HashSet<String>,
) -> bool {
    route_hosts
        .iter()
        .any(|route_host| no_proxy_entry_matches_route(entry, route_host))
}

fn no_proxy_entry_matches_route(entry: &str, route_host_port: &str) -> bool {
    let Some(route_host) = route_host_from_host_port(route_host_port) else {
        return true;
    };
    crate::config::no_proxy_entry_overlaps_host_pattern(entry, route_host)
}

/// Extract the host portion from a pre-normalised `host:port` string
/// (`route_store.route_upstream_hosts()` output). Returns `None` on a
/// malformed `host:port` — callers treat that as "matches everything" (fail
/// closed: an unparseable route host must not be silently excluded from
/// route-conflict protection).
fn route_host_from_host_port(route_host_port: &str) -> Option<&str> {
    if let Some(rest) = route_host_port.strip_prefix('[') {
        let end = rest.find(']')?;
        let host_end = end.checked_add(2)?;
        let port = route_host_port[host_end..].strip_prefix(':')?;
        if port.parse::<u16>().is_err() {
            return None;
        }
        return Some(&route_host_port[..host_end]);
    }

    let (host, port) = route_host_port.rsplit_once(':')?;
    if host.is_empty() || host.contains(':') || port.parse::<u16>().is_err() {
        return None;
    }
    Some(host)
}

/// Shared state for the proxy server.
struct ProxyState {
    filter: ProxyFilter,
    session_token: Zeroizing<String>,
    /// Route-level configuration (upstream, L7 filtering, custom TLS CA) for all routes.
    route_store: RouteStore,
    /// Credential-specific configuration (inject mode, headers, secrets) for routes with credentials.
    credential_store: CredentialStore,
    config: ProxyConfig,
    /// Shared TLS connector for upstream connections (reverse proxy mode).
    /// Created once at startup to avoid rebuilding the root cert store per request.
    tls_connector: tokio_rustls::TlsConnector,
    /// Default TLS client configuration backing `tls_connector`.
    /// Stored separately so `UpstreamPool` can create per-route clients keyed
    /// by `Arc<ClientConfig>` pointer identity.
    /// Upstream cdeeb5b9 (#983): pool infrastructure absorbed.
    default_tls_config: Arc<rustls::ClientConfig>,
    /// HTTP connection pool for upstream requests (HTTP/1.1 keep-alive + optional HTTP/2).
    /// Upstream cdeeb5b9 (#983): pool infrastructure absorbed; direct-TLS path retained.
    upstream_pool: crate::pool::UpstreamPool,
    /// Active connection count for connection limiting.
    active_connections: AtomicUsize,
    /// Shared network audit log for this proxy session.
    audit_log: audit::SharedAuditLog,
    /// Matcher for hosts that bypass the external proxy and route direct.
    /// Built once at startup from `ExternalProxyConfig.bypass_hosts`.
    bypass_matcher: external::BypassMatcher,
}

/// Start the proxy server.
///
/// Binds to `config.bind_addr:config.bind_port` (port 0 = OS-assigned),
/// generates a session token, and begins accepting connections.
///
/// Returns a `ProxyHandle` with the assigned port and session token.
/// The server runs until the handle is dropped or `shutdown()` is called.
pub async fn start(config: ProxyConfig) -> Result<ProxyHandle> {
    // D-06: validate config.no_proxy (grammar + allowed_hosts overlap)
    // before doing any other startup work — an invalid or bypass-widening
    // no_proxy list must fail the whole proxy, not just be silently dropped
    // or partially applied.
    validate_no_proxy_config(&config)?;

    // Generate session token
    let session_token = token::generate_session_token()?;

    // Bind listener
    let bind_addr = SocketAddr::new(config.bind_addr, config.bind_port);
    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|e| ProxyError::Bind {
            addr: bind_addr.to_string(),
            source: e,
        })?;

    let local_addr = listener.local_addr().map_err(|e| ProxyError::Bind {
        addr: bind_addr.to_string(),
        source: e,
    })?;
    let port = local_addr.port();

    info!("Proxy server listening on {}", local_addr);

    // Load route-level configuration (upstream, L7 filtering, custom TLS CA)
    // for ALL routes, regardless of credential presence. This happens before
    // the no_proxy/route-conflict check below so a route/no_proxy conflict
    // fails configuration instead of being silently omitted.
    let route_store = if config.routes.is_empty() {
        RouteStore::empty()
    } else {
        RouteStore::load(&config.routes)?
    };
    let route_hosts = route_store.route_upstream_hosts();
    validate_no_proxy_route_conflicts(&config.no_proxy, &route_hosts)?;

    // Load credentials for reverse proxy routes (only routes with credential_key)
    let credential_store = if config.routes.is_empty() {
        CredentialStore::empty()
    } else {
        CredentialStore::load(&config.routes)?
    };
    let loaded_routes = credential_store.loaded_prefixes();

    // Build filter. Strict mode treats an empty allowlist as deny-all.
    // `with_denied_hosts` layers caller-supplied deny entries (deny_domain,
    // ADR-108) on top — deny is always evaluated before the allowlist and
    // never activates the proxy or widens it on its own.
    let filter = if config.strict_filter {
        ProxyFilter::new_strict(&config.allowed_hosts)
    } else if config.allowed_hosts.is_empty() {
        ProxyFilter::allow_all()
    } else {
        ProxyFilter::new(&config.allowed_hosts)
    }
    .with_denied_hosts(&config.denied_hosts);

    // Build shared TLS connector (root cert store is expensive to construct).
    // Use the ring provider explicitly to avoid ambiguity when multiple
    // crypto providers are in the dependency tree.
    //
    // Combines webpki roots with the OS trust store via the shared
    // `route::build_base_root_store()` helper. This replays the security
    // intent of upstream 8ddb143 (TLS trust on corporate networks with
    // MITM inspection) WITHOUT pulling in the tls_intercept module
    // (per D-40-B2 fork-preserve lock; fork has no tls_intercept).
    let root_store = route::build_base_root_store();
    let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| ProxyError::Config(format!("TLS config error: {}", e)))?
    .with_root_certificates(root_store)
    .with_no_client_auth();
    // Store the Arc<ClientConfig> separately so UpstreamPool can key per-route
    // clients by pointer identity without re-building the root cert store.
    let tls_config_arc: Arc<ClientConfig> = Arc::new(tls_config);
    let tls_connector = tokio_rustls::TlsConnector::from(Arc::clone(&tls_config_arc));
    let upstream_pool =
        crate::pool::UpstreamPool::new(Arc::clone(&tls_config_arc), config.enable_h2);

    // Build bypass matcher from external proxy config (once, not per-request)
    let bypass_matcher = config
        .external_proxy
        .as_ref()
        .map(|ext| external::BypassMatcher::new(&ext.bypass_hosts))
        .unwrap_or_else(|| external::BypassMatcher::new(&[]));

    // Shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let audit_log = audit::new_audit_log();

    // Compute smart NO_PROXY hosts: allowed_hosts that can be reached via
    // direct TCP connections (i.e. their port is in direct_connect_ports).
    // Hosts without a direct TCP grant MUST go through the proxy —
    // adding them to NO_PROXY would cause clients to attempt direct
    // connections that the sandbox (Landlock / Seatbelt) denies.
    //
    // Route upstreams are always excluded so their traffic goes through
    // the proxy for L7 path filtering and/or credential injection.
    //
    // On macOS this MUST be empty regardless: Seatbelt's ProxyOnly mode
    // blocks ALL direct outbound. See #580.
    let smart_no_proxy_hosts: Vec<String> = if cfg!(target_os = "macos") {
        Vec::new()
    } else {
        config
            .allowed_hosts
            .iter()
            .filter_map(|host| {
                let normalised = {
                    let h = host.to_lowercase();
                    if h.starts_with('[') {
                        // IPv6 literal: "[::1]:443" has port, "[::1]" needs default
                        if h.contains("]:") {
                            h
                        } else {
                            format!("{}:443", h)
                        }
                    } else if h.contains(':') {
                        h
                    } else {
                        format!("{}:443", h)
                    }
                };
                // Use is_route_upstream (wildcard-aware) so that wildcard route upstreams
                // like "*.api.example.com:443" correctly exclude matching allowed_hosts.
                // Upstream 08ca19a8 (#1243): updated from HashSet::contains exact match.
                if route_store.is_route_upstream(&normalised) {
                    return None;
                }
                // Only bypass the proxy if the sandbox grants direct
                // TCP on this host's port (via --allow-connect-port).
                let port = normalised
                    .rsplit_once(':')
                    .and_then(|(_, p)| p.parse::<u16>().ok())
                    .unwrap_or(443);
                if config.direct_connect_ports.contains(&port) {
                    smart_no_proxy_entry(host)
                } else {
                    None
                }
            })
            .collect()
    };

    // No credential-loopback-route detection is absorbed in this fork yet
    // (see the ProxyHandle.managed_loopback_upstream doc comment) — always
    // false here.
    let managed_loopback_upstream = false;

    let no_proxy_hosts =
        merge_no_proxy_hosts(&smart_no_proxy_hosts, &config.no_proxy, &route_hosts);
    let canonical_no_proxy_hosts =
        merge_canonical_no_proxy_hosts(&smart_no_proxy_hosts, &config.no_proxy, &route_hosts);

    if !no_proxy_hosts.is_empty() {
        debug!("NO_PROXY bypass hosts: {:?}", no_proxy_hosts);
    }

    let state = Arc::new(ProxyState {
        filter,
        session_token: session_token.clone(),
        route_store,
        credential_store,
        config,
        tls_connector,
        default_tls_config: tls_config_arc,
        upstream_pool,
        active_connections: AtomicUsize::new(0),
        audit_log: Arc::clone(&audit_log),
        bypass_matcher,
    });

    // Spawn accept loop as a task within the current runtime.
    // The caller MUST ensure this runtime is being driven (e.g., via
    // a dedicated thread calling block_on or a multi-thread runtime).
    tokio::spawn(accept_loop(listener, state, shutdown_rx));

    Ok(ProxyHandle {
        port,
        token: session_token,
        audit_log,
        shutdown_tx,
        loaded_routes,
        no_proxy_hosts,
        managed_loopback_upstream,
        canonical_no_proxy_hosts,
        diagnostics: vec![],
    })
}

/// Accept loop: listen for connections until shutdown.
async fn accept_loop(
    listener: TcpListener,
    state: Arc<ProxyState>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        // Connection limit enforcement
                        let max = state.config.max_connections;
                        if max > 0 {
                            let current = state.active_connections.load(Ordering::Relaxed);
                            if current >= max {
                                warn!("Connection limit reached ({}/{}), rejecting {}", current, max, addr);
                                // Drop the stream (connection refused)
                                drop(stream);
                                continue;
                            }
                        }
                        state.active_connections.fetch_add(1, Ordering::Relaxed);

                        debug!("Accepted connection from {}", addr);
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, &state).await {
                                debug!("Connection handler error: {}", e);
                            }
                            state.active_connections.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) => {
                        warn!("Accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Proxy server shutting down");
                    return;
                }
            }
        }
    }
}

/// Handle a single client connection.
///
/// Reads the first HTTP line to determine the proxy mode:
/// - CONNECT method -> tunnel (Mode 1 or 3)
/// - Other methods  -> reverse proxy (Mode 2)
async fn handle_connection(mut stream: tokio::net::TcpStream, state: &ProxyState) -> Result<()> {
    // Read the first line and headers through a BufReader.
    // We keep the BufReader alive until we've consumed the full header
    // to prevent data loss (BufReader may read ahead into the body).
    let mut buf_reader = BufReader::new(&mut stream);
    let mut first_line = String::new();
    buf_reader.read_line(&mut first_line).await?;

    if first_line.is_empty() {
        return Ok(()); // Client disconnected
    }

    // Read remaining headers (up to empty line), with size limit to prevent OOM.
    let mut header_bytes = Vec::new();
    loop {
        let mut line = String::new();
        let n = buf_reader.read_line(&mut line).await?;
        if n == 0 || line.trim().is_empty() {
            break;
        }
        header_bytes.extend_from_slice(line.as_bytes());
        if header_bytes.len() > MAX_HEADER_SIZE {
            drop(buf_reader);
            let response = "HTTP/1.1 431 Request Header Fields Too Large\r\n\r\n";
            stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }
    }

    // Extract any data buffered beyond headers before dropping BufReader.
    // BufReader may have read ahead into the request body. We capture
    // those bytes and pass them to the reverse proxy handler so no body
    // data is lost. For CONNECT requests this is always empty (no body).
    let buffered = buf_reader.buffer().to_vec();
    drop(buf_reader);

    let first_line = first_line.trim_end();

    // Dispatch by method
    if first_line.starts_with("CONNECT ") {
        // Block CONNECT tunnels to route upstreams. These must go
        // through the reverse proxy path so L7 path filtering and
        // credential injection are enforced. A CONNECT tunnel would
        // bypass both (raw TLS pipe, proxy never sees HTTP method/path).
        if !state.route_store.is_empty() {
            if let Some(authority) = first_line.split_whitespace().nth(1) {
                // Normalise authority to host:port. Handle IPv6 brackets:
                // "[::1]:443" already has port, "[::1]" needs default, "host:443" has port.
                let host_port = if authority.starts_with('[') {
                    // IPv6 literal
                    if authority.contains("]:") {
                        authority.to_lowercase()
                    } else {
                        format!("{}:443", authority.to_lowercase())
                    }
                } else if authority.contains(':') {
                    authority.to_lowercase()
                } else {
                    format!("{}:443", authority.to_lowercase())
                };
                if state.route_store.is_route_upstream(&host_port) {
                    let (host, port) = host_port
                        .rsplit_once(':')
                        .map(|(h, p)| (h, p.parse::<u16>().unwrap_or(443)))
                        .unwrap_or((&host_port, 443));
                    warn!(
                        "Blocked CONNECT to route upstream {} — use reverse proxy path instead",
                        authority
                    );
                    audit::log_denied(
                        Some(&state.audit_log),
                        audit::ProxyMode::Connect,
                        &audit::EventContext {
                            denial_category: Some(
                                nono::undo::NetworkAuditDenialCategory::ConnectBypassesL7,
                            ),
                            ..Default::default()
                        },
                        host,
                        port,
                        "route upstream: CONNECT bypasses L7 filtering",
                    );
                    let response = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
                    stream.write_all(response.as_bytes()).await?;
                    return Ok(());
                }
            }
        }

        // Check if external proxy is configured and host is not bypassed
        let use_external = if let Some(ref ext_config) = state.config.external_proxy {
            if state.bypass_matcher.is_empty() {
                Some(ext_config)
            } else {
                // Parse host from CONNECT line to check bypass
                let host = first_line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|authority| {
                        authority
                            .rsplit_once(':')
                            .map(|(h, _)| h)
                            .or(Some(authority))
                    })
                    .unwrap_or("");
                if state.bypass_matcher.matches(host) {
                    debug!("Bypassing external proxy for {}", host);
                    None
                } else {
                    Some(ext_config)
                }
            }
        } else {
            None
        };

        if let Some(ext_config) = use_external {
            external::handle_external_proxy(
                first_line,
                &mut stream,
                &header_bytes,
                &state.filter,
                &state.session_token,
                ext_config,
                Some(&state.audit_log),
            )
            .await
        } else if state.config.external_proxy.is_some() {
            // Bypass route: enforce strict session token validation before
            // routing direct. Without this, bypassed hosts would inherit
            // connect::handle_connect()'s lenient auth (which tolerates
            // missing Proxy-Authorization for Node.js undici compat).
            token::validate_proxy_auth(&header_bytes, &state.session_token)?;
            connect::handle_connect(
                first_line,
                &mut stream,
                &state.filter,
                &state.session_token,
                &header_bytes,
                Some(&state.audit_log),
            )
            .await
        } else {
            connect::handle_connect(
                first_line,
                &mut stream,
                &state.filter,
                &state.session_token,
                &header_bytes,
                Some(&state.audit_log),
            )
            .await
        }
    } else if !state.route_store.is_empty() {
        // Non-CONNECT request with routes configured -> reverse proxy
        let ctx = reverse::ReverseProxyCtx {
            route_store: &state.route_store,
            credential_store: &state.credential_store,
            session_token: &state.session_token,
            filter: &state.filter,
            tls_connector: &state.tls_connector,
            default_tls_config: &state.default_tls_config,
            upstream_pool: &state.upstream_pool,
            audit_log: Some(&state.audit_log),
        };
        reverse::handle_reverse_proxy(first_line, &mut stream, &header_bytes, &ctx, &buffered).await
    } else {
        // No routes configured, reject non-CONNECT requests
        let response = "HTTP/1.1 400 Bad Request\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_starts_and_binds() {
        let config = ProxyConfig::default();
        let handle = start(config).await.unwrap();

        // Port should be non-zero (OS-assigned)
        assert!(handle.port > 0);
        // Token should be 64 hex chars
        assert_eq!(handle.token.len(), 64);

        // Shutdown
        handle.shutdown();
    }

    // Fork divergence: test_intercept_lifecycle_end_to_end,
    // test_intercept_skipped_for_purely_declarative_routes,
    // test_intercept_setup_failure_degrades_without_aborting_proxy,
    // test_route_diagnostics_summarises_each_route omitted.
    // These tests require ProxyConfig::intercept_ca_dir, ProxyHandle::intercept_ca_path(),
    // ProxyHandle::route_diagnostics(), and RouteConfig fields
    // (proxy, tls_client_cert, tls_client_key) not present in this fork.
    // Deferred to a future plan porting the intercept surface.

    #[tokio::test]
    async fn test_proxy_env_vars() {
        let config = ProxyConfig::default();
        let handle = start(config).await.unwrap();

        let vars = handle.env_vars();
        let http_proxy = vars.iter().find(|(k, _)| k == "HTTP_PROXY");
        assert!(http_proxy.is_some());
        assert!(http_proxy.unwrap().1.starts_with("http://nono:"));

        let token_var = vars.iter().find(|(k, _)| k == "NONO_PROXY_TOKEN");
        assert!(token_var.is_some());
        assert_eq!(token_var.unwrap().1.len(), 64);

        let node_proxy_flag = vars.iter().find(|(k, _)| k == "NODE_USE_ENV_PROXY");
        assert!(
            node_proxy_flag.is_some(),
            "proxy env must set NODE_USE_ENV_PROXY for Node 20.6+ (undici 5.22+) built-in fetch()"
        );
        assert_eq!(
            node_proxy_flag.unwrap().1,
            "1",
            "NODE_USE_ENV_PROXY must be '1'"
        );

        handle.shutdown();
    }

    #[tokio::test]
    async fn test_proxy_credential_env_vars() {
        let config = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "openai".to_string(),
                upstream: "https://api.openai.com".to_string(),
                credential_key: None,
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };
        let handle = start(config.clone()).await.unwrap();

        let vars = handle.credential_env_vars(&config);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].0, "OPENAI_BASE_URL");
        assert!(vars[0].1.contains("/openai"));

        handle.shutdown();
    }

    #[test]
    fn test_proxy_credential_env_vars_fallback_to_uppercase_key() {
        // When env_var is None and credential_key is set, the env var name
        // should be derived from uppercasing credential_key. This is the
        // backward-compatible path for keyring-backed credentials.
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: ["openai".to_string()].into_iter().collect(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };
        let config = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "openai".to_string(),
                upstream: "https://api.openai.com".to_string(),
                credential_key: Some("openai_api_key".to_string()),
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None, // No explicit env_var — should fall back to uppercase
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };

        let vars = handle.credential_env_vars(&config);
        assert_eq!(vars.len(), 2); // BASE_URL + API_KEY

        // Should derive OPENAI_API_KEY from uppercasing "openai_api_key"
        let api_key_var = vars.iter().find(|(k, _)| k == "OPENAI_API_KEY");
        assert!(
            api_key_var.is_some(),
            "Should derive env var name from credential_key.to_uppercase()"
        );

        let (_, val) = api_key_var.expect("OPENAI_API_KEY should exist");
        assert_eq!(val, "test_token");
    }

    #[test]
    fn test_proxy_credential_env_vars_with_explicit_env_var() {
        // When env_var is set on a route, it should be used instead of
        // deriving from credential_key. This is essential for URI manager
        // credential refs (e.g., op://, apple-password://)
        // where uppercasing produces nonsensical env var names.
        //
        // We construct a ProxyHandle directly to test env var generation
        // without starting a real proxy (which would try to load credentials).
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: ["openai".to_string()].into_iter().collect(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };
        let config = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "openai".to_string(),
                upstream: "https://api.openai.com".to_string(),
                credential_key: Some("op://Development/OpenAI/credential".to_string()),
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: Some("OPENAI_API_KEY".to_string()),
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };

        let vars = handle.credential_env_vars(&config);
        assert_eq!(vars.len(), 2); // BASE_URL + API_KEY

        let api_key_var = vars.iter().find(|(k, _)| k == "OPENAI_API_KEY");
        assert!(
            api_key_var.is_some(),
            "Should use explicit env_var name, not derive from credential_key"
        );

        // Verify the value is the phantom token, not the real credential
        let (_, val) = api_key_var.expect("OPENAI_API_KEY var should exist");
        assert_eq!(val, "test_token");

        // Verify no nonsensical OP:// env var was generated
        let bad_var = vars.iter().find(|(k, _)| k.starts_with("OP://"));
        assert!(
            bad_var.is_none(),
            "Should not generate env var from op:// URI uppercase"
        );
    }

    #[test]
    fn test_proxy_credential_env_vars_skips_unloaded_routes() {
        // When a credential is unavailable (e.g., GITHUB_TOKEN not set),
        // the route should NOT inject a phantom token env var. Otherwise
        // the phantom token shadows valid credentials from other sources
        // like the system keyring. See: #234
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            // Only "openai" was loaded; "github" credential was unavailable
            loaded_routes: ["openai".to_string()].into_iter().collect(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };
        let config = ProxyConfig {
            routes: vec![
                crate::config::RouteConfig {
                    prefix: "openai".to_string(),
                    upstream: "https://api.openai.com".to_string(),
                    credential_key: Some("openai_api_key".to_string()),
                    inject_mode: crate::config::InjectMode::Header,
                    inject_header: "Authorization".to_string(),
                    credential_format: Some("Bearer {}".to_string()),
                    path_pattern: None,
                    path_replacement: None,
                    query_param_name: None,
                    env_var: None,
                    endpoint_rules: vec![],
                    tls_ca: None,
                    oauth2: None,
                    aws_auth: None,
                    endpoint_policy: None,
                },
                crate::config::RouteConfig {
                    prefix: "github".to_string(),
                    upstream: "https://api.github.com".to_string(),
                    credential_key: Some("env://GITHUB_TOKEN".to_string()),
                    inject_mode: crate::config::InjectMode::Header,
                    inject_header: "Authorization".to_string(),
                    credential_format: Some("token {}".to_string()),
                    path_pattern: None,
                    path_replacement: None,
                    query_param_name: None,
                    env_var: Some("GITHUB_TOKEN".to_string()),
                    endpoint_rules: vec![],
                    tls_ca: None,
                    oauth2: None,
                    aws_auth: None,
                    endpoint_policy: None,
                },
            ],
            ..Default::default()
        };

        let vars = handle.credential_env_vars(&config);

        // openai should have BASE_URL + API_KEY (credential loaded)
        let openai_base = vars.iter().find(|(k, _)| k == "OPENAI_BASE_URL");
        assert!(openai_base.is_some(), "loaded route should have BASE_URL");
        let openai_key = vars.iter().find(|(k, _)| k == "OPENAI_API_KEY");
        assert!(openai_key.is_some(), "loaded route should have API key");

        // github should have BASE_URL (always set for declared routes) but
        // must NOT have GITHUB_TOKEN (credential was not loaded)
        let github_base = vars.iter().find(|(k, _)| k == "GITHUB_BASE_URL");
        assert!(
            github_base.is_some(),
            "declared route should still have BASE_URL"
        );
        let github_token = vars.iter().find(|(k, _)| k == "GITHUB_TOKEN");
        assert!(
            github_token.is_none(),
            "unloaded route must not inject phantom GITHUB_TOKEN"
        );
    }

    #[test]
    fn test_proxy_credential_env_vars_strips_slashes() {
        // When prefix includes leading/trailing slashes, the env var name
        // must not contain slashes and the URL must not double-slash.
        // Regression test for user-reported bug where "/anthropic" produced
        // "/ANTHROPIC_BASE_URL=http://127.0.0.1:PORT//anthropic".
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 58406,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: std::collections::HashSet::new(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };

        // Test leading slash
        let config = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "/anthropic".to_string(),
                upstream: "https://api.anthropic.com".to_string(),
                credential_key: None,
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };

        let vars = handle.credential_env_vars(&config);
        assert_eq!(vars.len(), 1);
        assert_eq!(
            vars[0].0, "ANTHROPIC_BASE_URL",
            "env var name must not have leading slash"
        );
        assert_eq!(
            vars[0].1, "http://127.0.0.1:58406/anthropic",
            "URL must not have double slash"
        );

        // Test trailing slash
        let config = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "openai/".to_string(),
                upstream: "https://api.openai.com".to_string(),
                credential_key: None,
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };

        let vars = handle.credential_env_vars(&config);
        assert_eq!(
            vars[0].0, "OPENAI_BASE_URL",
            "env var name must not have trailing slash"
        );
        assert_eq!(
            vars[0].1, "http://127.0.0.1:58406/openai",
            "URL must not have trailing slash in path"
        );
    }

    #[test]
    fn test_anthropic_credential_phantom_token_regression() {
        // Regression test for issue #624: the built-in anthropic credential
        // entry had no env_var or credential_key, so ANTHROPIC_API_KEY was
        // never set to the phantom token. Only ANTHROPIC_BASE_URL was injected,
        // leaving the sandbox to send the host's real key directly.
        //
        // Fork adaptation: removed intercept_ca_path (not in fork's ProxyHandle),
        // proxy, tls_client_cert, tls_client_key (not in fork's RouteConfig).
        //
        // Pre-fix state: route in loaded_routes but no env_var / credential_key
        // => ANTHROPIC_API_KEY must NOT appear (demonstrates the bug).
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle_no_env_var = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("phantom".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx: shutdown_tx.clone(),
            loaded_routes: ["anthropic".to_string()].into_iter().collect(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };
        let config_no_env_var = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "anthropic".to_string(),
                upstream: "https://api.anthropic.com".to_string(),
                credential_key: None,
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "x-api-key".to_string(),
                credential_format: Some("{}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };
        let vars_no_env_var = handle_no_env_var.credential_env_vars(&config_no_env_var);
        assert!(
            vars_no_env_var
                .iter()
                .all(|(k, _)| k != "ANTHROPIC_API_KEY"),
            "pre-fix: ANTHROPIC_API_KEY must not be set when neither env_var nor credential_key is defined (bug reproduced)"
        );

        // Post-fix state: route has env_var = "ANTHROPIC_API_KEY"
        // => ANTHROPIC_API_KEY must be set to the phantom token.
        let (shutdown_tx2, _) = tokio::sync::watch::channel(false);
        let handle_fixed = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("phantom".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx: shutdown_tx2,
            loaded_routes: ["anthropic".to_string()].into_iter().collect(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };
        let config_fixed = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "anthropic".to_string(),
                upstream: "https://api.anthropic.com".to_string(),
                credential_key: Some("ANTHROPIC_API_KEY".to_string()),
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "x-api-key".to_string(),
                credential_format: Some("{}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: Some("ANTHROPIC_API_KEY".to_string()),
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };
        let vars_fixed = handle_fixed.credential_env_vars(&config_fixed);
        let api_key_var = vars_fixed.iter().find(|(k, _)| k == "ANTHROPIC_API_KEY");
        assert!(
            api_key_var.is_some(),
            "post-fix: ANTHROPIC_API_KEY must be set to the phantom token"
        );
        assert_eq!(api_key_var.unwrap().1, "phantom");
    }

    #[test]
    fn test_no_proxy_excludes_credential_upstreams() {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: std::collections::HashSet::new(),
            no_proxy_hosts: vec![
                "nats.internal:4222".to_string(),
                "opencode.internal:4096".to_string(),
            ],
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };

        let vars = handle.env_vars();
        let no_proxy = vars.iter().find(|(k, _)| k == "NO_PROXY").unwrap();
        assert!(
            no_proxy.1.contains("nats.internal"),
            "non-credential host should be in NO_PROXY"
        );
        assert!(
            no_proxy.1.contains("opencode.internal"),
            "non-credential host should be in NO_PROXY"
        );
        assert!(
            no_proxy.1.contains("localhost"),
            "localhost should always be in NO_PROXY"
        );
    }

    #[test]
    fn test_no_proxy_empty_when_no_non_credential_hosts() {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: std::collections::HashSet::new(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };

        let vars = handle.env_vars();
        let no_proxy = vars.iter().find(|(k, _)| k == "NO_PROXY").unwrap();
        assert_eq!(
            no_proxy.1, "localhost,127.0.0.1",
            "NO_PROXY should only contain loopback when no bypass hosts"
        );
    }

    // ========================================================================
    // #1415 (D-06/D-07) — push_no_proxy_entry pipeline regression coverage
    // ========================================================================

    /// D-07 regression proof: `#1415` replaces the fork's static
    /// `vec!["localhost", "127.0.0.1"]` NO_PROXY seed with the
    /// `push_no_proxy_entry` pipeline. This test proves that replacement did
    /// not silently drop the fork's pre-existing loopback-bypass behavior —
    /// with no profile-declared `no_proxy` entries and
    /// `managed_loopback_upstream: false`, both `localhost` and `127.0.0.1`
    /// must still appear in the child's `NO_PROXY` env var.
    #[test]
    fn no_proxy_pipeline_preserves_localhost_and_loopback_bypass() {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: std::collections::HashSet::new(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };

        let vars = handle.env_vars();
        let no_proxy = vars.iter().find(|(k, _)| k == "NO_PROXY").unwrap();
        let entries: Vec<&str> = no_proxy.1.split(',').collect();
        assert!(
            entries.contains(&"localhost"),
            "localhost must survive the push_no_proxy_entry pipeline replacement (D-07): {}",
            no_proxy.1
        );
        assert!(
            entries.contains(&"127.0.0.1"),
            "127.0.0.1 must survive the push_no_proxy_entry pipeline replacement (D-07): {}",
            no_proxy.1
        );

        // lowercase `no_proxy` variant must carry the same value.
        let no_proxy_lower = vars.iter().find(|(k, _)| k == "no_proxy").unwrap();
        assert_eq!(no_proxy_lower.1, no_proxy.1);
    }

    /// Existing behavior must not regress: when a managed credential route
    /// targets a loopback upstream, loopback must be ABSENT from `NO_PROXY`
    /// so that traffic still traverses the proxy (for L7 filtering /
    /// credential injection) instead of connecting directly.
    #[test]
    fn no_proxy_pipeline_clears_loopback_when_managed_loopback_upstream() {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: std::collections::HashSet::new(),
            no_proxy_hosts: Vec::new(),
            managed_loopback_upstream: true,
            canonical_no_proxy_hosts: Vec::new(),
            diagnostics: vec![],
        };

        let vars = handle.env_vars();
        let no_proxy = vars.iter().find(|(k, _)| k == "NO_PROXY").unwrap();
        assert!(
            no_proxy.1.is_empty(),
            "NO_PROXY must be empty when managed_loopback_upstream is true: {}",
            no_proxy.1
        );
    }

    /// A no_proxy entry that survives the D-06 validators (Task 1) is
    /// appended to `NO_PROXY` via `push_no_proxy_entry`, and `NONO_NO_PROXY`
    /// (the canonical form) is present alongside `NO_PROXY` in the child env.
    #[test]
    fn no_proxy_pipeline_appends_validated_entry_and_emits_canonical_var() {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = ProxyHandle {
            port: 12345,
            token: Zeroizing::new("test_token".to_string()),
            audit_log: audit::new_audit_log(),
            shutdown_tx,
            loaded_routes: std::collections::HashSet::new(),
            no_proxy_hosts: vec!["redis".to_string()],
            managed_loopback_upstream: false,
            canonical_no_proxy_hosts: vec!["redis".to_string()],
            diagnostics: vec![],
        };

        let vars = handle.env_vars();
        let no_proxy = vars.iter().find(|(k, _)| k == "NO_PROXY").unwrap();
        assert!(
            no_proxy.1.split(',').any(|h| h == "redis"),
            "validated no_proxy entry must be appended to NO_PROXY via push_no_proxy_entry: {}",
            no_proxy.1
        );

        let nono_no_proxy = vars
            .iter()
            .find(|(k, _)| k == "NONO_NO_PROXY")
            .expect("NONO_NO_PROXY must be present alongside NO_PROXY");
        assert!(
            nono_no_proxy.1.split(',').any(|h| h == "redis"),
            "canonical no_proxy var must also carry the validated entry: {}",
            nono_no_proxy.1
        );
    }

    /// D-06 fail-closed proof: `start()` must reject a config whose
    /// `no_proxy` entry does not satisfy `validate_no_proxy_entry`'s
    /// grammar, before any listener is bound or `ProxyHandle` constructed.
    #[tokio::test]
    async fn start_rejects_no_proxy_entry_with_invalid_grammar() {
        let config = ProxyConfig {
            no_proxy: vec!["evil.com/path".to_string()],
            ..Default::default()
        };
        match start(config).await {
            Err(ProxyError::Config(_)) => {}
            Err(other) => panic!("expected ProxyError::Config, got: {other:?}"),
            Ok(handle) => {
                handle.shutdown();
                panic!("start() must reject a malformed no_proxy entry, not succeed");
            }
        }
    }

    /// D-06 fail-closed proof: `start()` must reject a `no_proxy` entry that
    /// overlaps `allowed_hosts` — without this guard the entry would
    /// silently defeat the allow_domain filter for the overlapping host.
    #[tokio::test]
    async fn start_rejects_no_proxy_entry_overlapping_allowed_host() {
        let config = ProxyConfig {
            allowed_hosts: vec!["api.example.com".to_string()],
            no_proxy: vec!["*.example.com".to_string()],
            ..Default::default()
        };
        match start(config).await {
            Err(ProxyError::Config(_)) => {}
            Err(other) => panic!("expected ProxyError::Config, got: {other:?}"),
            Ok(handle) => {
                handle.shutdown();
                panic!("start() must reject a no_proxy entry that overlaps an allowed_hosts entry");
            }
        }
    }

    /// D-06 fail-closed proof: `start()` must reject a `no_proxy` entry that
    /// overlaps a configured route upstream — route traffic must go through
    /// the proxy for L7 filtering / credential injection, so a bypass here
    /// is never valid.
    #[tokio::test]
    async fn start_rejects_no_proxy_entry_conflicting_with_route_upstream() {
        let config = ProxyConfig {
            no_proxy: vec!["*.openai.com".to_string()],
            routes: vec![crate::config::RouteConfig {
                prefix: "openai".to_string(),
                upstream: "https://api.openai.com".to_string(),
                credential_key: Some("openai".to_string()),
                inject_mode: crate::config::InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
                endpoint_policy: None,
            }],
            ..Default::default()
        };
        match start(config).await {
            Err(ProxyError::Config(_)) => {}
            Err(other) => panic!("expected ProxyError::Config, got: {other:?}"),
            Ok(handle) => {
                handle.shutdown();
                panic!("start() must reject a no_proxy entry that conflicts with a route upstream");
            }
        }
    }

    #[tokio::test]
    async fn test_no_proxy_empty_without_direct_connect_ports() {
        // When direct_connect_ports is empty (no --allow-connect-port),
        // allowed_hosts should NOT appear in NO_PROXY because the sandbox
        // blocks direct TCP and clients would fail to connect. See #760.
        let config = ProxyConfig {
            allowed_hosts: vec!["github.com".to_string()],
            ..Default::default()
        };
        let handle = start(config).await.unwrap();

        let vars = handle.env_vars();
        let no_proxy = vars.iter().find(|(k, _)| k == "NO_PROXY").unwrap();
        assert_eq!(
            no_proxy.1, "localhost,127.0.0.1",
            "allowed_hosts must not appear in NO_PROXY without direct_connect_ports"
        );

        handle.shutdown();
    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn test_no_proxy_includes_hosts_with_matching_connect_port() {
        // When direct_connect_ports includes port 443, allowed_hosts on
        // that port SHOULD appear in NO_PROXY (direct TCP is permitted).
        // macOS always returns empty NO_PROXY (Seatbelt blocks all direct outbound).
        //
        // #1415 (D-06) tightened smart-derived NO_PROXY entries to go
        // through the same `validate_no_proxy_entry` grammar as
        // operator-declared entries: a bare multi-label domain like
        // "github.com" is now filtered out as ambiguous (common HTTP
        // clients treat bare NO_PROXY domains as suffix matches), so this
        // test uses an IP literal — which the grammar accepts unambiguously
        // — to prove the "matching connect port bypasses the proxy"
        // mechanism still works post-absorb. See
        // `test_smart_no_proxy_entry_filters_ambiguous_bare_domains` for the
        // direct grammar-tightening coverage.
        let config = ProxyConfig {
            allowed_hosts: vec![
                "203.0.113.5".to_string(),
                "server.internal:4222".to_string(),
            ],
            direct_connect_ports: vec![443],
            ..Default::default()
        };
        let handle = start(config).await.unwrap();

        let vars = handle.env_vars();
        let no_proxy = vars.iter().find(|(k, _)| k == "NO_PROXY").unwrap();
        assert!(
            no_proxy.1.contains("203.0.113.5"),
            "IP-literal host on port 443 should be in NO_PROXY when 443 is in direct_connect_ports"
        );
        assert!(
            !no_proxy.1.contains("server.internal"),
            "host on port 4222 should NOT be in NO_PROXY when only 443 is allowed"
        );

        handle.shutdown();
    }

    /// #1415 (D-06): smart-derived NO_PROXY candidates go through the same
    /// `validate_no_proxy_entry` grammar as operator-declared entries. Bare
    /// multi-label domains are filtered as ambiguous; single-label aliases,
    /// IP literals, and bracketed IPv6 literals survive; wildcard-prefixed
    /// entries are always filtered (a smart entry cannot safely widen to a
    /// bare-domain suffix bypass).
    #[test]
    fn test_smart_no_proxy_entry_filters_ambiguous_bare_domains() {
        assert_eq!(smart_no_proxy_entry("github.com"), None);
        assert_eq!(smart_no_proxy_entry("api.github.com:443"), None);
        assert_eq!(smart_no_proxy_entry("redis"), Some("redis".to_string()));
        assert_eq!(
            smart_no_proxy_entry("127.0.0.1"),
            Some("127.0.0.1".to_string())
        );
        assert_eq!(smart_no_proxy_entry("[::1]:443"), Some("::1".to_string()));
        assert_eq!(smart_no_proxy_entry("*.internal.example:443"), None);
    }

    /// Regression test: when `strict_filter` is true and `allowed_hosts` is
    /// empty, the proxy must deny CONNECT instead of falling back to allow-all.
    #[tokio::test]
    async fn test_strict_filter_with_empty_allowlist_denies_connect() {
        use tokio::io::AsyncReadExt;
        use tokio::net::TcpStream;

        let config = ProxyConfig {
            strict_filter: true,
            allowed_hosts: Vec::new(),
            ..ProxyConfig::default()
        };
        let handle = start(config).await.unwrap();
        let addr = format!("127.0.0.1:{}", handle.port);

        let mut stream = TcpStream::connect(&addr).await.unwrap();
        let request = b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n";
        tokio::io::AsyncWriteExt::write_all(&mut stream, request)
            .await
            .unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            response_str.starts_with("HTTP/1.1 403"),
            "strict filter with empty allowlist must deny CONNECT, got: {}",
            response_str
        );

        let events = handle.drain_audit_events();
        assert!(
            events
                .iter()
                .any(|e| e.decision == nono::undo::NetworkAuditDecision::Deny
                    && e.target == "example.com"),
            "expected a Deny audit event for example.com, got: {:?}",
            events
        );

        handle.shutdown();
    }
}
