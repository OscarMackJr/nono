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
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{debug, info, warn};
use url::Url;
use zeroize::Zeroizing;

/// Maximum forward-proxy request body size (16 MiB), mirroring
/// `reverse::MAX_REQUEST_BODY`. Prevents DoS from a malicious Content-Length
/// on the plain-HTTP forward path (#1335).
const MAX_FORWARD_BODY: usize = 16 * 1024 * 1024;

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

/// Parse host and port from a non-CONNECT request line's absolute-form
/// target (e.g. `GET http://host:port/path HTTP/1.1`).
///
/// Ported from upstream `726ac1f1` (#1335). Not previously present in this
/// fork's `server.rs` (verified absent before this plan — no prior caller
/// needed it, since absolute-form requests fell through to a flat 400).
/// Used by `handle_forward_http` to resolve the target for both the
/// host-filter check and audit records.
fn parse_non_connect_target(line: &str) -> Result<(String, u16)> {
    let mut parts = line.split_whitespace();
    let _method = parts.next();
    let url = parts
        .next()
        .ok_or_else(|| ProxyError::HttpParse(format!("malformed request line: {}", line)))?;
    let parsed = Url::parse(url)
        .map_err(|e| ProxyError::HttpParse(format!("invalid URL in request: {}: {}", url, e)))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| ProxyError::HttpParse(format!("no host in URL: {}", url)))?
        .to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);
    Ok((host, port))
}

/// Request-target form of a non-CONNECT proxy request line, used to
/// discriminate forward-proxy (absolute-form) requests from reverse-proxy
/// (origin-form) ones.
///
/// A forward-proxy client (one honoring `HTTP_PROXY`) sends the *absolute*
/// URL in the request line: `GET http://example.com/path HTTP/1.1`. A
/// reverse-proxy client sends *origin-form*: `GET /service/path HTTP/1.1`.
/// Discriminating on the target's form (not the method) is what lets nono
/// act as a drop-in `HTTP_PROXY` target without disturbing the existing
/// origin-form reverse-proxy routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestTargetForm {
    /// Absolute-form `http://…` — plain HTTP forward proxy.
    AbsoluteHttp,
    /// Absolute-form `https://…` — must be tunneled via CONNECT, not forwarded.
    AbsoluteHttps,
    /// Origin-form (`/path`) or anything else — reverse-proxy / inline path.
    Origin,
}

/// Classify the request-target of a non-CONNECT request line.
///
/// Only the scheme prefix of the request target is inspected. The check is
/// ASCII-case-insensitive per RFC 3986 (schemes are case-insensitive) and
/// deliberately conservative: anything that is not an `http://` or `https://`
/// absolute URL is treated as origin-form so existing reverse-proxy and
/// inline flows are completely unaffected.
fn classify_request_target(line: &str) -> RequestTargetForm {
    // Request line: METHOD SP request-target SP HTTP-version
    let Some(target) = line.split_whitespace().nth(1) else {
        return RequestTargetForm::Origin;
    };
    // Case-insensitive scheme match without allocating a lowercase copy of
    // the whole (possibly long) URL.
    let lower_starts_with = |s: &str, prefix: &str| {
        s.len() >= prefix.len()
            && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
    };
    if lower_starts_with(target, "http://") {
        RequestTargetForm::AbsoluteHttp
    } else if lower_starts_with(target, "https://") {
        RequestTargetForm::AbsoluteHttps
    } else {
        RequestTargetForm::Origin
    }
}

/// Rewrite an absolute-form request line into origin-form for forwarding.
///
/// `GET http://host/p?q HTTP/1.1` -> `GET /p?q HTTP/1.1`
/// `GET http://host HTTP/1.1`     -> `GET /`      (empty path becomes `/`)
///
/// The method and HTTP-version tokens are preserved verbatim. Returns the
/// rewritten first line (with trailing CRLF) so it can be prepended to the
/// forwarded header block.
fn rewrite_absolute_to_origin_form(line: &str) -> Result<String> {
    let mut parts = line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| ProxyError::HttpParse(format!("malformed request line: {}", line)))?;
    let target = parts
        .next()
        .ok_or_else(|| ProxyError::HttpParse(format!("malformed request line: {}", line)))?;
    // Preserve the version if present; default to HTTP/1.1 otherwise.
    let version = parts.next().unwrap_or("HTTP/1.1");

    let parsed = Url::parse(target)
        .map_err(|e| ProxyError::HttpParse(format!("invalid URL in request: {}: {}", target, e)))?;

    let mut origin = parsed.path().to_string();
    if origin.is_empty() {
        origin.push('/');
    }
    if let Some(query) = parsed.query() {
        origin.push('?');
        origin.push_str(query);
    }

    // Defence in depth: the method/version tokens come from the client's
    // request line and are echoed into the forwarded request line, which is
    // itself a protocol-formatting boundary. `Url` already rejects control
    // characters in the target, but strip CR/LF from the surrounding tokens
    // so a crafted method/version can never split the forwarded request.
    let sanitise = |s: &str| s.replace(['\r', '\n'], "");
    Ok(format!(
        "{} {} {}\r\n",
        sanitise(method),
        origin,
        sanitise(version)
    ))
}

/// Strip hop-by-hop proxy headers (`Proxy-Connection`, `Proxy-Authorization`)
/// from a raw header block before forwarding upstream.
///
/// These headers are meaningful only on the client<->proxy hop and must never
/// be forwarded: `Proxy-Authorization` carries the session token, and
/// `Proxy-Connection` is a non-standard hop-by-hop hint. Other headers
/// (including `Host`) are preserved verbatim so the forwarded request matches
/// what the client sent.
fn strip_proxy_headers(header_bytes: &[u8]) -> Vec<u8> {
    let header_str = match std::str::from_utf8(header_bytes) {
        Ok(s) => s,
        // Non-UTF-8 headers: forward unchanged rather than corrupt the block.
        // The upstream will reject malformed headers itself.
        Err(_) => return header_bytes.to_vec(),
    };
    let mut out = Vec::with_capacity(header_bytes.len());
    for line in header_str.split_inclusive("\r\n") {
        let name = line.split(':').next().unwrap_or("").trim();
        if name.eq_ignore_ascii_case("proxy-connection")
            || name.eq_ignore_ascii_case("proxy-authorization")
        {
            continue;
        }
        out.extend_from_slice(line.as_bytes());
    }
    out
}

/// Handle an absolute-form `http://` forward-proxy request.
///
/// This is the plain-HTTP counterpart to the CONNECT tunnel: a client that
/// honors `HTTP_PROXY` sends `GET http://host/path HTTP/1.1` for cleartext
/// HTTP, and nono forwards it after applying the same trust boundary the
/// tunnel path uses (session-token auth + host filter).
///
/// Steps:
/// 1. Enforce `Proxy-Authorization` (same session-token gate as CONNECT /
///    reverse). On failure: 407 + audit denial, matching the reverse path's
///    no-credential branch.
/// 2. Parse host+port from the absolute URL and run the host filter. On deny:
///    403 + `HostDenied` audit event, mirroring the no-routes inline `else`.
/// 3. Rewrite the request line to origin-form and strip hop-by-hop proxy
///    headers, then forward directly to the DNS-rebinding-safe resolved
///    addresses `check_host` already returned.
///
/// SAFETY / SECURITY: this path intentionally does NOT restrict itself to
/// loopback-only upstreams. Upstream nono (`726ac1f1`) documents this as not
/// calling a `validate_http_upstream_target` loopback-only guard; this fork's
/// `reverse.rs` has no function by that name (the reverse-proxy path here
/// accepts any route-configured upstream without a loopback restriction), but
/// the underlying rationale still applies and is preserved: a general forward
/// proxy exists precisely to reach arbitrary ALLOWED `http://` hosts on
/// behalf of the agent, unlike a reverse-proxy route (operator-configured,
/// where an accidental non-local plain-HTTP upstream would be a
/// credential-leak footgun). Here the host filter (`check_host`) is the
/// sufficient and authoritative trust boundary: it applies the allowlist and
/// the cloud-metadata / link-local SSRF guards, and returns the exact
/// resolved addresses this function then connects to. Do NOT add a
/// loopback-only restriction to this path — that would defeat its purpose.
async fn handle_forward_http(
    first_line: &str,
    stream: &mut tokio::net::TcpStream,
    header_bytes: &[u8],
    buffered: &[u8],
    state: &ProxyState,
) -> Result<()> {
    // 1. Proxy-Authorization gate — identical to the reverse-proxy
    //    no-credential branch: 407 on missing/invalid auth, with an audit
    //    denial recording the authentication failure. No auth bypass.
    if let Err(e) = token::validate_proxy_auth(header_bytes, &state.session_token) {
        // Parse the target host for the audit record where possible; fall
        // back to a placeholder so a malformed line still audits.
        let (host, port) =
            parse_non_connect_target(first_line).unwrap_or_else(|_| ("unknown".to_string(), 0));
        audit::log_denied(
            Some(&state.audit_log),
            audit::ProxyMode::Reverse,
            &audit::EventContext {
                auth_mechanism: Some(nono::undo::NetworkAuditAuthMechanism::ProxyAuthorization),
                auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
                denial_category: Some(nono::undo::NetworkAuditDenialCategory::AuthenticationFailed),
                ..audit::EventContext::default()
            },
            &host,
            port,
            &e.to_string(),
        );
        let response = "HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"nono\"\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    // 2. Parse host+port and run the host filter (DNS resolution + SSRF guard).
    let (host, port) = parse_non_connect_target(first_line)?;
    let check = state.filter.check_host(&host, port).await?;
    if !check.result.is_allowed() {
        let reason = check.result.reason();
        audit::log_denied(
            Some(&state.audit_log),
            audit::ProxyMode::Reverse,
            &audit::EventContext {
                denial_category: Some(nono::undo::NetworkAuditDenialCategory::HostDenied),
                ..audit::EventContext::default()
            },
            &host,
            port,
            &reason,
        );
        let sanitised = reason.replace(['\r', '\n'], " ");
        let response = format!("HTTP/1.1 403 Forbidden: {}\r\n\r\n", sanitised);
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    // 3. Build the origin-form request: rewritten request line + proxy-header
    //    -stripped header block, then read the body honoring Content-Length.
    //    `filtered_headers` already retains any client-sent Content-Length
    //    verbatim (strip_proxy_headers only removes Proxy-Connection and
    //    Proxy-Authorization), so no Content-Length is re-added here.
    let origin_line = rewrite_absolute_to_origin_form(first_line)?;
    let inbound_path = origin_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let method = first_line
        .split_whitespace()
        .next()
        .unwrap_or("GET")
        .to_string();

    let filtered_headers = strip_proxy_headers(header_bytes);

    let content_length = reverse::extract_content_length(header_bytes);
    let body = if let Some(len) = content_length {
        if len > MAX_FORWARD_BODY {
            let response = "HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n";
            stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }
        let mut buf = Vec::with_capacity(len);
        let pre = buffered.len().min(len);
        buf.extend_from_slice(&buffered[..pre]);
        let remaining = len - pre;
        if remaining > 0 {
            let mut rest = vec![0u8; remaining];
            stream.read_exact(&mut rest).await?;
            buf.extend_from_slice(&rest);
        }
        buf
    } else {
        Vec::new()
    };

    let mut request_bytes =
        Vec::with_capacity(origin_line.len() + filtered_headers.len() + body.len() + 2);
    request_bytes.extend_from_slice(origin_line.as_bytes());
    request_bytes.extend_from_slice(&filtered_headers);
    request_bytes.extend_from_slice(b"\r\n");
    if !body.is_empty() {
        request_bytes.extend_from_slice(&body);
    }

    // 4. Connect directly to the resolved addresses (DNS-rebinding-safe) and
    //    forward. Fork scope note: upstream `726ac1f1` additionally chains
    //    through an external/enterprise proxy via a `forward.rs`/
    //    `UpstreamSpec`/`UpstreamStrategy` abstraction this fork does not
    //    have (verified absent — the same class of miss the plan's
    //    execution_notes flagged for `<interfaces>` blocks in this
    //    milestone). External-proxy chaining for this specific forward-http
    //    path is out of scope here; the security-critical part — connecting
    //    only to the DNS-rebinding-safe resolved addresses `check_host`
    //    returned, never re-resolving the hostname — is fully implemented.
    let mut upstream = match reverse::connect_to_resolved(&check.resolved_addrs, &host).await {
        Ok(s) => s,
        Err(e) => {
            warn!("forward-http upstream connection failed: {}", e);
            audit::log_denied(
                Some(&state.audit_log),
                audit::ProxyMode::Reverse,
                &audit::EventContext {
                    denial_category: Some(
                        nono::undo::NetworkAuditDenialCategory::UpstreamConnectFailed,
                    ),
                    ..audit::EventContext::default()
                },
                &host,
                port,
                &e.to_string(),
            );
            let response = "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n";
            stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }
    };

    upstream.write_all(&request_bytes).await?;
    upstream.flush().await?;

    // Stream the response back to the client without buffering, tracking the
    // upstream status code for the audit record (mirrors the reverse-proxy
    // streaming loop in reverse.rs).
    let mut response_buf = [0u8; 8192];
    let mut status_code: u16 = 502;
    let mut first_chunk = true;

    loop {
        let n = match upstream.read(&mut response_buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                debug!("forward-http upstream read error: {}", e);
                break;
            }
        };

        if first_chunk {
            status_code = reverse::parse_response_status(&response_buf[..n]);
            first_chunk = false;
        }

        stream.write_all(&response_buf[..n]).await?;
        stream.flush().await?;
    }

    // The forward path is transparent pass-through: no managed credential is
    // ever consulted or injected, so the audit record positively asserts
    // managed_credential_active = false (not merely omitted) — this is the
    // T-109-12 mitigation the plan's threat model requires.
    audit::log_l7_request(
        Some(&state.audit_log),
        audit::ProxyMode::Reverse,
        &audit::EventContext {
            auth_mechanism: Some(nono::undo::NetworkAuditAuthMechanism::ProxyAuthorization),
            auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Succeeded),
            managed_credential_active: Some(false),
            ..audit::EventContext::default()
        },
        &audit::L7RequestInfo {
            host: &host,
            port,
            method: &method,
            path: &inbound_path,
            status: status_code,
        },
    );

    Ok(())
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
            let ext_ctx = external::ExternalProxyCtx {
                filter: &state.filter,
                session_token: &state.session_token,
                audit_log: Some(&state.audit_log),
                require_auth: state.config.require_auth,
            };
            external::handle_external_proxy(
                first_line,
                &mut stream,
                &header_bytes,
                &ext_ctx,
                ext_config,
            )
            .await
        } else if state.config.external_proxy.is_some() {
            // Bypass route: enforce strict session token validation before
            // routing direct. Without this, bypassed hosts would inherit
            // connect::handle_connect()'s lenient auth (which tolerates
            // missing Proxy-Authorization for Node.js undici compat).
            // Phase 112 SEC-07: skipped entirely when require_auth is false
            // (standalone `nono proxy --no-auth`).
            if state.config.require_auth {
                token::validate_proxy_auth(&header_bytes, &state.session_token)?;
            }
            connect::handle_connect(
                first_line,
                &mut stream,
                &state.filter,
                &state.session_token,
                &header_bytes,
                Some(&state.audit_log),
                state.config.require_auth && state.config.strict_connect_auth,
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
                state.config.require_auth && state.config.strict_connect_auth,
            )
            .await
        }
    } else if classify_request_target(first_line) == RequestTargetForm::AbsoluteHttp {
        // Absolute-form `http://…` request from an HTTP_PROXY-honoring client.
        // Forward it as a plain-HTTP forward proxy (see handle_forward_http).
        // This branch is checked BEFORE the origin-form reverse-proxy path so
        // that absolute-form URLs never reach parse_service_prefix (which
        // would misread the scheme as a service name — see issue #1334).
        handle_forward_http(first_line, &mut stream, &header_bytes, &buffered, state).await
    } else if classify_request_target(first_line) == RequestTargetForm::AbsoluteHttps {
        // Absolute-form `https://…` cannot be forwarded as cleartext: the
        // proxy would have to originate TLS to the upstream on the client's
        // behalf, which no standard HTTP_PROXY client expects. Such clients
        // use CONNECT for HTTPS. Reject with explicit guidance rather than a
        // confusing 502.
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: 62\r\n\r\nhttps forward-proxying is not supported; use CONNECT for https";
        stream.write_all(response.as_bytes()).await?;
        Ok(())
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
            require_auth: state.config.require_auth,
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

    // ========================================================================
    // Forward-proxy (absolute-form http://) tests — issue #1334 (#1335)
    // ========================================================================

    #[test]
    fn classify_request_target_detects_absolute_and_origin_forms() {
        assert_eq!(
            classify_request_target("GET http://example.com/path HTTP/1.1"),
            RequestTargetForm::AbsoluteHttp
        );
        // Scheme match is case-insensitive.
        assert_eq!(
            classify_request_target("GET HTTP://example.com/ HTTP/1.1"),
            RequestTargetForm::AbsoluteHttp
        );
        assert_eq!(
            classify_request_target("CONNECT https://example.com/ HTTP/1.1"),
            RequestTargetForm::AbsoluteHttps
        );
        assert_eq!(
            classify_request_target("GET /openai/v1/chat HTTP/1.1"),
            RequestTargetForm::Origin
        );
        // Malformed / empty lines are treated as origin-form (unaffected).
        assert_eq!(classify_request_target("GET"), RequestTargetForm::Origin);
        assert_eq!(classify_request_target(""), RequestTargetForm::Origin);
    }

    #[test]
    fn rewrite_absolute_to_origin_form_produces_origin_line() {
        assert_eq!(
            rewrite_absolute_to_origin_form("GET http://host.example/p/q?a=1 HTTP/1.1").unwrap(),
            "GET /p/q?a=1 HTTP/1.1\r\n"
        );
        // Bare authority with no path becomes "/".
        assert_eq!(
            rewrite_absolute_to_origin_form("GET http://host.example HTTP/1.1").unwrap(),
            "GET / HTTP/1.1\r\n"
        );
        // Method and version are preserved verbatim.
        assert_eq!(
            rewrite_absolute_to_origin_form("POST http://host.example:8080/x HTTP/1.0").unwrap(),
            "POST /x HTTP/1.0\r\n"
        );
    }

    #[test]
    fn strip_proxy_headers_removes_proxy_hop_by_hop_only() {
        let headers = b"Host: example.com\r\nProxy-Connection: keep-alive\r\nProxy-Authorization: Basic abc\r\nAccept: */*\r\n";
        let stripped = strip_proxy_headers(headers);
        let s = String::from_utf8(stripped).unwrap();
        assert!(s.contains("Host: example.com"));
        assert!(s.contains("Accept: */*"));
        assert!(
            !s.to_lowercase().contains("proxy-connection"),
            "Proxy-Connection must be stripped, got: {s:?}"
        );
        assert!(
            !s.to_lowercase().contains("proxy-authorization"),
            "Proxy-Authorization must be stripped, got: {s:?}"
        );
    }

    /// Spawn a one-shot local HTTP/1.1 origin server that echoes the received
    /// request line back in the body and returns 200. Returns its address and
    /// a receiver that yields the raw request bytes it saw.
    async fn spawn_echo_origin() -> (std::net::SocketAddr, tokio::sync::oneshot::Receiver<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let received = String::from_utf8_lossy(&buf[..n]).to_string();
                let body = "ok";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(response.as_bytes()).await;
                let _ = sock.flush().await;
                let _ = tx.send(received);
            }
        });
        (addr, rx)
    }

    /// Absolute-form http:// to an ALLOWED host is forwarded and returns the
    /// upstream status. Also asserts the upstream saw an origin-form request
    /// line with the proxy headers stripped.
    #[tokio::test]
    async fn forward_http_allowed_host_is_forwarded() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let (origin_addr, origin_rx) = spawn_echo_origin().await;

        let config = ProxyConfig {
            allowed_hosts: vec!["127.0.0.1".to_string()],
            ..ProxyConfig::default()
        };
        let handle = start(config).await.unwrap();
        let token = handle.token.to_string();

        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        let creds = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(format!("nono:{}", token))
        };
        let request = format!(
            "GET http://127.0.0.1:{}/hello?x=1 HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nProxy-Connection: keep-alive\r\nProxy-Authorization: Basic {}\r\nAccept: */*\r\n\r\n",
            origin_addr.port(),
            origin_addr.port(),
            creds
        );
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            response_str.starts_with("HTTP/1.1 200"),
            "expected upstream 200 status, got: {}",
            response_str
        );

        // The upstream must have seen an origin-form request line with the
        // proxy headers stripped.
        let received = origin_rx.await.unwrap();
        assert!(
            received.starts_with("GET /hello?x=1 HTTP/1.1"),
            "upstream should see origin-form request line, got: {received:?}"
        );
        assert!(
            !received.to_lowercase().contains("proxy-connection"),
            "Proxy-Connection must not reach upstream: {received:?}"
        );
        assert!(
            !received.to_lowercase().contains("proxy-authorization"),
            "Proxy-Authorization must not reach upstream: {received:?}"
        );
        assert!(
            received.contains("Accept: */*"),
            "non-proxy headers must be preserved: {received:?}"
        );

        // An L7 audit event should have been recorded for the allowed request.
        let events = handle.drain_audit_events();
        assert!(
            events
                .iter()
                .any(|e| e.decision == nono::undo::NetworkAuditDecision::Allow
                    && e.target == "127.0.0.1"
                    && e.status == Some(200)),
            "expected an allow L7 audit event, got: {events:?}"
        );

        handle.shutdown();
    }

    /// The forward path is a TRANSPARENT proxy: it must never inject a managed
    /// credential, even when credential-injecting routes are configured.
    /// Credential injection is reserved for the reverse path (HTTPS or
    /// http-loopback upstreams).
    ///
    /// The configured route carries an UNRESOLVABLE credential key. If the
    /// forward path ever attempted injection it would fail credential
    /// resolution and return 503 (the reverse path's missing-credential
    /// behavior) — so a 200 with the client's own Authorization header intact
    /// proves the forward path bypasses routing/injection entirely. We also
    /// assert the audit event reports `managed_credential_active = false`.
    #[tokio::test]
    async fn forward_http_does_not_inject_managed_credential() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let (origin_addr, origin_rx) = spawn_echo_origin().await;

        // A credential-injecting route whose key cannot resolve. The forward
        // path must ignore it entirely rather than fail resolving it.
        let config = ProxyConfig {
            allowed_hosts: vec!["127.0.0.1".to_string()],
            routes: vec![crate::config::RouteConfig {
                prefix: "svc".to_string(),
                upstream: "https://api.example.com".to_string(),
                credential_key: Some("env://NONO_TEST_TOTALLY_MISSING".to_string()),
                inject_mode: Default::default(),
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                endpoint_policy: None,
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
            }],
            ..ProxyConfig::default()
        };
        let handle = start(config).await.unwrap();
        let token = handle.token.to_string();

        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        let creds = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(format!("nono:{}", token))
        };
        // The client sends its own Authorization header. A transparent proxy
        // forwards it unchanged.
        let request = format!(
            "GET http://127.0.0.1:{}/data HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nProxy-Authorization: Basic {}\r\nAuthorization: Bearer client-token\r\n\r\n",
            origin_addr.port(),
            origin_addr.port(),
            creds
        );
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        // 200 (not 503) proves no credential resolution/injection was attempted.
        assert!(
            response_str.starts_with("HTTP/1.1 200"),
            "expected upstream 200 (no injection attempt), got: {}",
            response_str
        );

        let received = origin_rx.await.unwrap();
        // The client's own credential must survive verbatim, unmodified.
        assert!(
            received.contains("Authorization: Bearer client-token"),
            "client Authorization must be forwarded verbatim: {received:?}"
        );

        // The audit event must record that no managed credential was active.
        let events = handle.drain_audit_events();
        let l7 = events
            .iter()
            .find(|e| e.status == Some(200))
            .expect("expected an L7 audit event for the forwarded request");
        assert_eq!(
            l7.managed_credential_active,
            Some(false),
            "forward path must audit managed_credential_active=false: {l7:?}"
        );

        handle.shutdown();
    }

    /// Absolute-form http:// to a DENIED host returns 403 with an audit denial.
    #[tokio::test]
    async fn forward_http_denied_host_returns_403_and_audits() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        // Allowlist a different host so 127.0.0.1 is denied.
        let config = ProxyConfig {
            allowed_hosts: vec!["example.com".to_string()],
            ..ProxyConfig::default()
        };
        let handle = start(config).await.unwrap();
        let token = handle.token.to_string();

        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        let creds = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(format!("nono:{}", token))
        };
        let request = format!(
            "GET http://denied.example.org/secret HTTP/1.1\r\nHost: denied.example.org\r\nProxy-Authorization: Basic {}\r\n\r\n",
            creds
        );
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            response_str.starts_with("HTTP/1.1 403"),
            "expected 403 for denied host, got: {}",
            response_str
        );

        let events = handle.drain_audit_events();
        assert!(
            events
                .iter()
                .any(|e| e.decision == nono::undo::NetworkAuditDecision::Deny
                    && e.target == "denied.example.org"
                    && e.denial_category
                        == Some(nono::undo::NetworkAuditDenialCategory::HostDenied)),
            "expected a HostDenied audit event, got: {events:?}"
        );

        handle.shutdown();
    }

    /// Absolute-form https:// is rejected with guidance to use CONNECT.
    #[tokio::test]
    async fn forward_https_absolute_form_is_rejected_with_connect_guidance() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let config = ProxyConfig {
            allowed_hosts: vec!["example.com".to_string()],
            ..ProxyConfig::default()
        };
        let handle = start(config).await.unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        let request = b"GET https://example.com/ HTTP/1.1\r\nHost: example.com\r\n\r\n";
        stream.write_all(request).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            response_str.starts_with("HTTP/1.1 400"),
            "expected 400 for absolute-form https, got: {}",
            response_str
        );
        assert!(
            response_str.to_lowercase().contains("connect"),
            "response should direct the client to use CONNECT, got: {}",
            response_str
        );

        handle.shutdown();
    }

    /// Missing/invalid Proxy-Authorization with the strict filter active is
    /// rejected with 407 (matching the reverse-proxy no-credential branch).
    #[tokio::test]
    async fn forward_http_missing_proxy_auth_is_rejected_407() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let config = ProxyConfig {
            allowed_hosts: vec!["127.0.0.1".to_string()],
            ..ProxyConfig::default()
        };
        let handle = start(config).await.unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        // No Proxy-Authorization header at all.
        let request = b"GET http://127.0.0.1:9/hello HTTP/1.1\r\nHost: 127.0.0.1:9\r\n\r\n";
        stream.write_all(request).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            response_str.starts_with("HTTP/1.1 407"),
            "expected 407 for missing proxy auth, got: {}",
            response_str
        );

        let events = handle.drain_audit_events();
        assert!(
            events
                .iter()
                .any(|e| e.decision == nono::undo::NetworkAuditDecision::Deny
                    && e.denial_category
                        == Some(nono::undo::NetworkAuditDenialCategory::AuthenticationFailed)),
            "expected an AuthenticationFailed audit event, got: {events:?}"
        );

        handle.shutdown();
    }

    /// Regression guard: origin-form requests with routes configured still
    /// route to the reverse proxy (unaffected by the forward-proxy dispatch).
    ///
    /// Fork adaptation of upstream's assertion: upstream's `CredentialStore`
    /// returns a request-time 503 when a route's `credential_key` fails to
    /// resolve. This fork's `CredentialStore` resolves credentials once at
    /// startup and silently excludes routes whose credential could not be
    /// loaded (`ProxyHandle.loaded_routes`, documented in this file) rather
    /// than surfacing a 503 per-request — so the route falls back to
    /// `handle_reverse_proxy`'s no-credential branch (session-token-only L7
    /// filtering via `Proxy-Authorization`), which this request omits,
    /// yielding 407. The regression this test actually guards — that an
    /// origin-form request reaches the REVERSE handler, not the new
    /// forward-http path — is proven positively by the audit event's
    /// `route_id = Some("openai")`: only `handle_reverse_proxy`'s audit calls
    /// ever set `route_id`; `handle_forward_http` never does.
    #[tokio::test]
    async fn origin_form_request_still_routes_to_reverse_proxy() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let config = ProxyConfig {
            routes: vec![crate::config::RouteConfig {
                prefix: "openai".to_string(),
                upstream: "https://api.openai.com".to_string(),
                credential_key: Some("env://NONO_TEST_TOTALLY_MISSING".to_string()),
                inject_mode: Default::default(),
                inject_header: "Authorization".to_string(),
                credential_format: Some("Bearer {}".to_string()),
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                endpoint_policy: None,
                tls_ca: None,
                oauth2: None,
                aws_auth: None,
            }],
            ..Default::default()
        };
        let handle = start(config).await.unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        let request = b"GET /openai/v1/models HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer whatever\r\n\r\n";
        stream.write_all(request).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        // Any structured HTTP answer (not a dropped socket / raw forward
        // failure) proves *a* handler answered; the audit event below proves
        // it was specifically the reverse handler.
        assert!(
            response_str.starts_with("HTTP/1.1 407"),
            "origin-form request must reach the reverse proxy (407 for this fork's \
             unresolved-credential-falls-back-to-no-credential-L7-auth behavior), \
             got: {}",
            response_str
        );

        let events = handle.drain_audit_events();
        assert!(
            events
                .iter()
                .any(|e| e.mode == nono::undo::NetworkAuditMode::Reverse
                    && e.route_id.as_deref() == Some("openai")),
            "expected a Reverse-mode audit event with route_id=openai — proves the \
             reverse handler, not handle_forward_http, handled this request \
             (handle_forward_http never sets route_id), got: {events:?}"
        );

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
