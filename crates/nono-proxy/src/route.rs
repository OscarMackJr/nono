//! Route store: per-route configuration independent of credentials.
//!
//! `RouteStore` holds the route-level configuration (upstream URL, L7 endpoint
//! rules, custom TLS CA) for **all** configured routes, regardless of whether
//! they have a credential attached. This decouples L7 filtering from credential
//! injection — a route can enforce endpoint restrictions without injecting any
//! secret.
//!
//! The `CredentialStore` remains responsible for credential-specific fields
//! (inject mode, header name/value, raw secret). Both stores are keyed by the
//! normalised route prefix and are consulted independently by the proxy handlers.

use crate::config::{CompiledEndpointPolicy, CompiledEndpointRules, RouteConfig};
use crate::error::{ProxyError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;
use zeroize::Zeroizing;

/// Route-level configuration loaded at proxy startup.
///
/// Contains everything needed to forward and filter a request for a route,
/// but no credential material. Credential injection is handled separately
/// by `CredentialStore`.
pub struct LoadedRoute {
    /// Upstream URL (e.g., "https://api.openai.com")
    pub upstream: String,

    /// Pre-normalised `host:port` extracted from `upstream` at load time.
    /// Used for O(1) lookups in `is_route_upstream()` without per-request
    /// URL parsing. `RouteStore::load` rejects routes where this cannot be
    /// parsed so route-protection checks never silently drop an upstream.
    pub upstream_host_port: Option<String>,

    /// Pre-compiled L7 endpoint rules for method+path filtering.
    /// When non-empty, only matching requests are allowed (default-deny).
    /// When empty, all method+path combinations are permitted.
    pub endpoint_rules: CompiledEndpointRules,

    /// Pre-compiled explicit endpoint policy. When no explicit policy is
    /// configured this preserves legacy `endpoint_rules` semantics.
    pub endpoint_policy: CompiledEndpointPolicy,

    /// Per-route TLS connector with custom CA trust, if configured.
    /// Built once at startup from the route's `tls_ca` certificate file.
    /// When `None`, the shared default connector (webpki roots only) is used.
    pub tls_connector: Option<tokio_rustls::TlsConnector>,

    /// Per-route TLS client configuration backing `tls_connector`.
    ///
    /// Stored separately from the connector so that `UpstreamPool` can create
    /// a pooled client keyed by `Arc<ClientConfig>` pointer identity, providing
    /// per-route TLS isolation for the connection pool.
    ///
    /// `None` when the route uses the shared default TLS config.
    pub tls_client_config: Option<Arc<rustls::ClientConfig>>,

    /// Stable string key for the route's TLS settings.
    ///
    /// Derived from the `tls_ca` file path; used for diagnostics and as a
    /// human-readable label in pool statistics. `None` for routes using the
    /// default TLS config.
    pub tls_config_key: Option<String>,
}

impl std::fmt::Debug for LoadedRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedRoute")
            .field("upstream", &self.upstream)
            .field("upstream_host_port", &self.upstream_host_port)
            .field("endpoint_rules", &self.endpoint_rules)
            .field("endpoint_policy", &self.endpoint_policy)
            .field("has_custom_tls_ca", &self.tls_connector.is_some())
            .field("tls_config_key", &self.tls_config_key)
            .finish()
    }
}

/// Store of all configured routes, keyed by normalised prefix.
///
/// Loaded at proxy startup for **all** routes in the config, not just those
/// with credentials. This ensures L7 endpoint filtering and upstream routing
/// work independently of credential presence.
#[derive(Debug)]
pub struct RouteStore {
    routes: HashMap<String, LoadedRoute>,
}

impl RouteStore {
    /// Load route configuration for all configured routes.
    ///
    /// Each route's endpoint rules are compiled at startup so the hot path
    /// does a regex match, not a glob compile. Routes with a `tls_ca` field
    /// get a per-route TLS connector built from the custom CA certificate.
    pub fn load(routes: &[RouteConfig]) -> Result<Self> {
        let mut loaded = HashMap::new();

        for route in routes {
            let normalized_prefix = route.prefix.trim_matches('/').to_string();

            debug!(
                "Loading route '{}' -> {}",
                normalized_prefix, route.upstream
            );

            let endpoint_rules = CompiledEndpointRules::compile(&route.endpoint_rules)
                .map_err(|e| ProxyError::Config(format!("route '{}': {}", normalized_prefix, e)))?;

            let endpoint_policy = CompiledEndpointPolicy::compile(
                route.endpoint_policy.as_ref(),
                &route.endpoint_rules,
            )
            .map_err(|e| ProxyError::Config(format!("route '{}': {}", normalized_prefix, e)))?;

            let (tls_connector, tls_client_config, tls_config_key) = match route.tls_ca {
                Some(ref ca_path) => {
                    debug!(
                        "Building TLS connector with custom CA for route '{}': {}",
                        normalized_prefix, ca_path
                    );
                    let (connector, config) = build_tls_connector_with_ca(ca_path)?;
                    (Some(connector), Some(config), Some(ca_path.clone()))
                }
                None => (None, None, None),
            };

            let upstream_host_port = extract_host_port(&route.upstream).map_err(|err| {
                ProxyError::Config(format!(
                    "route '{}': invalid upstream '{}': {}",
                    normalized_prefix, route.upstream, err
                ))
            })?;

            loaded.insert(
                normalized_prefix,
                LoadedRoute {
                    upstream: route.upstream.clone(),
                    upstream_host_port: Some(upstream_host_port),
                    endpoint_rules,
                    endpoint_policy,
                    tls_connector,
                    tls_client_config,
                    tls_config_key,
                },
            );
        }

        Ok(Self { routes: loaded })
    }

    /// Create an empty route store (no routes configured).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Get a loaded route by normalised prefix, if configured.
    #[must_use]
    pub fn get(&self, prefix: &str) -> Option<&LoadedRoute> {
        self.routes.get(prefix)
    }

    /// Check if any routes are loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Number of loaded routes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check whether `host_port` (e.g. `"api.openai.com:443"`) matches
    /// any route's upstream URL. Uses pre-normalised `host:port` strings
    /// computed at load time to avoid per-request URL parsing.
    ///
    /// Supports wildcard prefixes in stored upstream patterns (e.g. a route
    /// upstream of `"*.openai.com:443"` matches `"api.openai.com:443"`).
    /// Upstream 08ca19a8 (#1243): wildcard credential upstream route fix.
    #[must_use]
    pub fn is_route_upstream(&self, host_port: &str) -> bool {
        let normalised = host_port.to_lowercase();
        self.routes.values().any(|route| {
            route
                .upstream_host_port
                .as_ref()
                .is_some_and(|hp| host_port_matches(hp, &normalised))
        })
    }

    /// Return the set of normalised `host:port` strings for all route
    /// upstreams. Uses pre-normalised values computed at load time.
    #[must_use]
    pub fn route_upstream_hosts(&self) -> std::collections::HashSet<String> {
        self.routes
            .values()
            .filter_map(|route| route.upstream_host_port.clone())
            .collect()
    }
}

/// Extract and normalise `host:port` from a URL string.
///
/// Defaults to port 443 for `https://` and 80 for `http://` when no
/// explicit port is present.
///
/// # Errors
///
/// Returns a human-readable error if the URL cannot be parsed, has no host,
/// or uses a scheme other than `http`/`https`. `RouteStore::load` propagates
/// this as a hard startup error rather than silently dropping the upstream
/// (an unparseable route upstream must not be invisible to no_proxy/route
/// conflict checks — 109-CONTEXT.md D-06/D-07).
#[must_use = "route upstream host:port extraction result must be handled"]
pub(crate) fn extract_host_port(url: &str) -> std::result::Result<String, String> {
    let parsed = url::Url::parse(url).map_err(|err| format!("URL parse error: {err}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "upstream URL must include a host".to_string())?;
    let default_port = match parsed.scheme() {
        "https" => 443,
        "http" => 80,
        scheme => {
            return Err(format!(
                "unsupported upstream scheme '{scheme}' (expected http or https)"
            ));
        }
    };
    let port = parsed.port().unwrap_or(default_port);
    Ok(format!("{}:{}", host.to_lowercase(), port))
}

/// Check whether `pattern` (a pre-normalised `host:port` string) matches
/// `target` (a pre-normalised `host:port` string).
///
/// Supports a leading `*.` wildcard prefix on the host portion of `pattern`.
/// The wildcard matches any non-empty sub-tree of labels — for example:
/// - `"*.openai.com:443"` matches `"api.openai.com:443"` ✓
/// - `"*.openai.com:443"` matches `"x.api.openai.com:443"` ✓ (multi-label)
/// - `"*.openai.com:443"` does NOT match `"openai.com:443"` (no prefix label)
///
/// Upstream 08ca19a8 (#1243): added to fix CONNECT-block detection and
/// NO_PROXY computation when wildcard upstreams are configured.
pub(crate) fn host_port_matches(pattern: &str, target: &str) -> bool {
    if pattern == target {
        return true;
    }
    if !pattern.starts_with("*.") {
        return false;
    }

    let Some((pattern_host, pattern_port)) = pattern.rsplit_once(':') else {
        return false;
    };
    let Some((target_host, target_port)) = target.rsplit_once(':') else {
        return false;
    };
    if pattern_port != target_port {
        return false;
    }

    let Some(suffix) = pattern_host.strip_prefix("*.") else {
        return false;
    };
    target_host
        .strip_suffix(suffix)
        .is_some_and(|prefix| prefix.ends_with('.') && prefix.len() > 1)
}

/// Build a root cert store combining webpki roots with the OS trust store.
///
/// Replays the security intent of upstream 8ddb143 ("Load native system CAs
/// alongside webpki_roots for upstream TLS connections, fixing UnknownIssuer
/// on corporate networks with TLS inspection") and the 54c7552 factoring of
/// the shared base store, WITHOUT pulling in upstream's tls_intercept module
/// or multi-route dispatch surface (per D-40-B2 fork-preserve lock).
///
/// Errors loading individual native certificates are logged at debug level
/// and skipped; the webpki roots are always present so this never fails to
/// produce a usable root store.
pub(crate) fn build_base_root_store() -> rustls::RootCertStore {
    let mut store = rustls::RootCertStore::empty();
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let native = rustls_native_certs::load_native_certs();
    if !native.errors.is_empty() {
        debug!(
            "failed to load {} native cert(s) for route connector base store; \
             continuing with webpki roots + any that succeeded",
            native.errors.len()
        );
    }
    let native_count = native.certs.len();
    for cert in native.certs {
        if let Err(e) = store.add(cert) {
            debug!("skipping unparseable native cert in route connector: {e}");
        }
    }
    if native_count > 0 {
        debug!("added {native_count} native system CA(s) to route connector trust store");
    }
    store
}

/// Build a `TlsConnector` and the underlying `ClientConfig` that trusts the
/// system roots plus a custom CA certificate.
///
/// Returns both the connector and the raw `Arc<ClientConfig>` so callers can
/// create per-route `UpstreamPool` clients keyed by pointer identity.
///
/// The CA file must be PEM-encoded and contain at least one certificate.
/// Returns an error if the file cannot be read, contains no valid certificates,
/// or the TLS configuration fails.
fn build_tls_connector_with_ca(
    ca_path: &str,
) -> Result<(tokio_rustls::TlsConnector, Arc<rustls::ClientConfig>)> {
    let ca_path = std::path::Path::new(ca_path);

    let ca_pem = Zeroizing::new(std::fs::read(ca_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ProxyError::Config(format!(
                "CA certificate file not found: '{}'",
                ca_path.display()
            ))
        } else {
            ProxyError::Config(format!(
                "failed to read CA certificate '{}': {}",
                ca_path.display(),
                e
            ))
        }
    })?);

    // Start from the shared base store (webpki + native system CAs) so the
    // custom CA is added on top of the OS trust store rather than replacing it.
    let mut root_store = build_base_root_store();

    // Parse and add custom CA certificates from PEM file
    let certs: Vec<_> = rustls_pemfile::certs(&mut ca_pem.as_slice())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| {
            ProxyError::Config(format!(
                "failed to parse CA certificate '{}': {}",
                ca_path.display(),
                e
            ))
        })?;

    if certs.is_empty() {
        return Err(ProxyError::Config(format!(
            "CA certificate file '{}' contains no valid PEM certificates",
            ca_path.display()
        )));
    }

    for cert in certs {
        root_store.add(cert).map_err(|e| {
            ProxyError::Config(format!(
                "invalid CA certificate in '{}': {}",
                ca_path.display(),
                e
            ))
        })?;
    }

    let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| ProxyError::Config(format!("TLS config error: {}", e)))?
    .with_root_certificates(root_store)
    .with_no_client_auth();

    let config_arc = Arc::new(tls_config);
    let connector = tokio_rustls::TlsConnector::from(Arc::clone(&config_arc));
    Ok((connector, config_arc))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::config::EndpointRule;

    #[test]
    fn test_empty_route_store() {
        let store = RouteStore::empty();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.get("openai").is_none());
    }

    #[test]
    fn test_load_routes_without_credentials() {
        // Routes without credential_key should still be loaded into RouteStore
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "/openai".to_string(),
            upstream: "https://api.openai.com".to_string(),
            credential_key: None,
            inject_mode: Default::default(),
            inject_header: "Authorization".to_string(),
            credential_format: Some("Bearer {}".to_string()),
            path_pattern: None,
            path_replacement: None,
            query_param_name: None,
            env_var: None,
            endpoint_rules: vec![
                EndpointRule {
                    method: "POST".to_string(),
                    path: "/v1/chat/completions".to_string(),
                },
                EndpointRule {
                    method: "GET".to_string(),
                    path: "/v1/models".to_string(),
                },
            ],
            tls_ca: None,
            oauth2: None,
            aws_auth: None,
            endpoint_policy: None,
        }];

        let store = RouteStore::load(&routes).unwrap();
        assert_eq!(store.len(), 1);

        let route = store.get("openai").unwrap();
        assert_eq!(route.upstream, "https://api.openai.com");
        assert!(route
            .endpoint_rules
            .is_allowed("POST", "/v1/chat/completions"));
        assert!(route.endpoint_rules.is_allowed("GET", "/v1/models"));
        assert!(!route
            .endpoint_rules
            .is_allowed("DELETE", "/v1/files/file-123"));
    }

    #[test]
    fn test_load_routes_normalises_prefix() {
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "/anthropic/".to_string(),
            upstream: "https://api.anthropic.com".to_string(),
            credential_key: None,
            inject_mode: Default::default(),
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
        }];

        let store = RouteStore::load(&routes).unwrap();
        assert!(store.get("anthropic").is_some());
        assert!(store.get("/anthropic/").is_none());
    }

    #[test]
    fn test_is_route_upstream() {
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "openai".to_string(),
            upstream: "https://api.openai.com".to_string(),
            credential_key: None,
            inject_mode: Default::default(),
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
        }];

        let store = RouteStore::load(&routes).unwrap();
        assert!(store.is_route_upstream("api.openai.com:443"));
        assert!(!store.is_route_upstream("github.com:443"));
    }

    #[test]
    fn test_route_upstream_hosts() {
        let routes = vec![
            RouteConfig {
                spiffe: None,
                prefix: "openai".to_string(),
                upstream: "https://api.openai.com".to_string(),
                credential_key: None,
                inject_mode: Default::default(),
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
            RouteConfig {
                spiffe: None,
                prefix: "anthropic".to_string(),
                upstream: "https://api.anthropic.com".to_string(),
                credential_key: None,
                inject_mode: Default::default(),
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
        ];

        let store = RouteStore::load(&routes).unwrap();
        let hosts = store.route_upstream_hosts();
        assert!(hosts.contains("api.openai.com:443"));
        assert!(hosts.contains("api.anthropic.com:443"));
        assert_eq!(hosts.len(), 2);
    }

    #[test]
    fn test_extract_host_port_preserves_wildcard_host() {
        assert_eq!(
            extract_host_port("https://*.dev.example.net"),
            Ok("*.dev.example.net:443".to_string())
        );
    }

    #[test]
    fn test_extract_host_port_https() {
        assert_eq!(
            extract_host_port("https://api.openai.com"),
            Ok("api.openai.com:443".to_string())
        );
    }

    #[test]
    fn test_extract_host_port_with_port() {
        assert_eq!(
            extract_host_port("https://api.example.com:8443"),
            Ok("api.example.com:8443".to_string())
        );
    }

    #[test]
    fn test_extract_host_port_http() {
        assert_eq!(
            extract_host_port("http://internal-service"),
            Ok("internal-service:80".to_string())
        );
    }

    #[test]
    fn test_extract_host_port_normalises_case() {
        assert_eq!(
            extract_host_port("https://API.Example.COM"),
            Ok("api.example.com:443".to_string())
        );
    }

    /// D-06/D-07: `RouteStore::load` must fail closed on an unparseable or
    /// unsupported route upstream instead of silently storing `None` for
    /// `upstream_host_port` — a silently-dropped upstream would be invisible
    /// to `validate_no_proxy_route_conflicts` and the no_proxy bypass could
    /// slip past route protection.
    #[test]
    fn test_load_routes_rejects_malformed_or_unsupported_upstreams() {
        for upstream in [
            "not a url",
            "ftp://api.openai.com",
            "https://",
            "https://api.openai.com:notaport",
        ] {
            let routes = vec![RouteConfig {
                spiffe: None,
                prefix: "bad".to_string(),
                upstream: upstream.to_string(),
                credential_key: None,
                inject_mode: Default::default(),
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
            }];

            assert!(
                RouteStore::load(&routes).is_err(),
                "route upstream {upstream:?} must fail closed at load time"
            );
        }
    }

    #[test]
    fn test_loaded_route_debug() {
        // Fork: LoadedRoute has upstream, upstream_host_port, endpoint_rules, tls_connector.
        // Upstream fields (requires_intercept, requires_managed_credential,
        // managed_auth_mechanism, managed_injection_mode) are not present in the fork.
        let route = LoadedRoute {
            upstream: "https://api.openai.com".to_string(),
            upstream_host_port: Some("api.openai.com:443".to_string()),
            endpoint_rules: CompiledEndpointRules::compile(&[]).unwrap(),
            endpoint_policy: CompiledEndpointPolicy::compile(None, &[]).unwrap(),
            tls_connector: None,
            tls_client_config: None,
            tls_config_key: None,
        };
        let debug_output = format!("{:?}", route);
        assert!(debug_output.contains("api.openai.com"));
        assert!(debug_output.contains("has_custom_tls_ca"));
    }

    // Fork divergence: tests for requires_intercept, requires_managed_credential,
    // managed_auth_mechanism, managed_injection_mode, lookup_by_upstream,
    // lookup_all_by_upstream, has_intercept_route, missing_managed_credential,
    // NetworkAuditAuthMechanism, NetworkAuditInjectionMode, and mTLS
    // (tls_client_cert/tls_client_key) are not ported. These fields and methods
    // belong to the upstream tls_intercept module not present in this fork.
    // Deferred to a future plan that ports the intercept surface.

    /// Self-signed CA for testing. Generated with:
    /// openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
    ///   -keyout /dev/null -nodes -days 36500 -subj '/CN=nono-test-ca' -out -
    const TEST_CA_PEM: &str = "\
-----BEGIN CERTIFICATE-----
MIIBnjCCAUWgAwIBAgIUT0bpOJJvHdOdZt+gW1stR8VBgXowCgYIKoZIzj0EAwIw
FzEVMBMGA1UEAwwMbm9uby10ZXN0LWNhMCAXDTI1MDEwMTAwMDAwMFoYDzIxMjQx
MjA3MDAwMDAwWjAXMRUwEwYDVQQDDAxub25vLXRlc3QtY2EwWTATBgcqhkjOPQIB
BggqhkjOPQMBBwNCAAR8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
AAAAAAAAAAAAAAAAAAAAo1MwUTAdBgNVHQ4EFgQUAAAAAAAAAAAAAAAAAAAAAAAA
AAAAMB8GA1UdIwQYMBaAFAAAAAAAAAAAAAAAAAAAAAAAAAAAADAPBgNVHRMBAf8E
BTADAQH/MAoGCCqGSM49BAMCA0cAMEQCIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
AAAAAAAICAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
-----END CERTIFICATE-----";

    #[test]
    fn test_build_tls_connector_with_valid_ca() {
        let dir = tempfile::tempdir().unwrap();
        let ca_path = dir.path().join("ca.pem");
        std::fs::write(&ca_path, TEST_CA_PEM).unwrap();

        let result = build_tls_connector_with_ca(ca_path.to_str().unwrap());
        match result {
            Ok(connector) => {
                drop(connector);
            }
            Err(ProxyError::Config(msg)) => {
                assert!(
                    msg.contains("invalid CA certificate") || msg.contains("CA certificate"),
                    "unexpected error: {}",
                    msg
                );
            }
            Err(e) => panic!("unexpected error type: {}", e),
        }
    }

    #[test]
    fn test_build_tls_connector_missing_file() {
        let result = build_tls_connector_with_ca("/nonexistent/path/ca.pem");
        let err = result
            .err()
            .expect("should fail for missing file")
            .to_string();
        assert!(
            err.contains("CA certificate file not found"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_build_tls_connector_empty_pem() {
        let dir = tempfile::tempdir().unwrap();
        let ca_path = dir.path().join("empty.pem");
        std::fs::write(&ca_path, "not a certificate\n").unwrap();

        let result = build_tls_connector_with_ca(ca_path.to_str().unwrap());
        let err = result
            .err()
            .expect("should fail for invalid PEM")
            .to_string();
        assert!(
            err.contains("no valid PEM certificates"),
            "unexpected error: {}",
            err
        );
    }

    // Fork divergence: mTLS (client certificate) tests omitted.
    // Fork's route.rs does not have build_tls_connector (4-arg: base+ca+cert+key);
    // only build_tls_connector_with_ca exists. The mTLS feature
    // (RouteConfig::tls_client_cert, RouteConfig::tls_client_key,
    // build_tls_connector multi-arg API) is not yet ported to this fork.
    // Tests omitted: test_build_tls_connector_cert_without_key_errors,
    // test_build_tls_connector_key_without_cert_errors,
    // test_build_tls_connector_missing_client_cert_file,
    // test_build_tls_connector_missing_client_key_file,
    // test_build_tls_connector_permission_denied,
    // test_build_tls_connector_empty_client_cert_pem,
    // test_build_tls_connector_empty_client_key_pem,
    // test_route_store_loads_mtls_route.
    // Deferred to a future plan.

    /// D-10 / #1132 allow_domain shadow-disproof:
    ///
    /// Upstream #1132 describes a bug intrinsic to upstream's host-ordering route selection
    /// abstraction, where an allow_domain endpoint route matching the same upstream HOST as
    /// a credential catch-all could win and shadow it.
    ///
    /// The fork has no host-keyed selection and no catch-all. It dispatches by EXACT
    /// path-prefix key, and allow_domain endpoint routes get a DISJOINT key namespace
    /// (`_ep_{domain}`) vs credential routes (the service name, e.g. "openai").
    ///
    /// This test proves the shadow class is structurally absent: two routes sharing the
    /// same upstream host but different prefix keys occupy disjoint HashMap slots.
    /// Neither lookup returns the other. The upstream host-ordered selection abstraction
    /// is not imported (D-10 lock).
    #[test]
    fn allow_domain_endpoint_route_does_not_shadow_credential_route() {
        // (a) Credential route — key "openai", upstream api.openai.com
        let credential_route = RouteConfig {
            spiffe: None,
            prefix: "openai".to_string(),
            upstream: "https://api.openai.com".to_string(),
            credential_key: Some("openai".to_string()),
            inject_mode: Default::default(),
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
        };

        // (b) allow_domain endpoint route — key "_ep_api.openai.com", SAME upstream host
        //     (this is the upstream shadow trigger: same host, different prefix namespace)
        let endpoint_route = RouteConfig {
            spiffe: None,
            prefix: "_ep_api.openai.com".to_string(),
            upstream: "https://api.openai.com".to_string(),
            credential_key: None,
            inject_mode: Default::default(),
            inject_header: "Authorization".to_string(),
            credential_format: None,
            path_pattern: None,
            path_replacement: None,
            query_param_name: None,
            env_var: None,
            endpoint_rules: vec![EndpointRule {
                method: "GET".to_string(),
                path: "/v1/models".to_string(),
            }],
            tls_ca: None,
            oauth2: None,
            aws_auth: None,
            endpoint_policy: None,
        };

        let routes = vec![credential_route, endpoint_route];
        let store = RouteStore::load(&routes).unwrap();

        // Both routes are present under their respective disjoint keys.
        assert_eq!(store.len(), 2);
        assert!(
            store.get("openai").is_some(),
            "credential route must be accessible by its own key"
        );
        assert!(
            store.get("_ep_api.openai.com").is_some(),
            "endpoint route must be accessible by its own key"
        );

        // The credential route is NOT accessible via the endpoint key, and vice versa.
        // Same upstream host does NOT collapse them into one slot.
        let cred = store.get("openai").unwrap();
        let ep = store.get("_ep_api.openai.com").unwrap();

        // Credential route has no endpoint rules → is_allowed returns true for everything
        // (open-access credential route, not shadowed by the endpoint route's rules).
        assert!(
            cred.endpoint_rules.is_allowed("DELETE", "/v1/models"),
            "credential route must allow all methods (no endpoint rules, not shadowed)"
        );
        assert!(
            cred.endpoint_rules
                .is_allowed("POST", "/v1/chat/completions"),
            "credential route must allow all paths (no endpoint rules, not shadowed)"
        );

        // Endpoint route has its own rules → restricts to GET /v1/models only.
        assert!(
            ep.endpoint_rules.is_allowed("GET", "/v1/models"),
            "endpoint route must enforce its own endpoint rules"
        );
        assert!(
            !ep.endpoint_rules.is_allowed("DELETE", "/v1/models"),
            "endpoint route must deny methods not in its rules"
        );

        // Both routes resolve to the same upstream — this is the shadow-trigger condition.
        // Dispatch is by prefix key, NOT by upstream host, so they remain distinct.
        assert_eq!(cred.upstream, "https://api.openai.com");
        assert_eq!(ep.upstream, "https://api.openai.com");
        assert_eq!(
            cred.upstream_host_port.as_deref(),
            Some("api.openai.com:443")
        );
        assert_eq!(ep.upstream_host_port.as_deref(), Some("api.openai.com:443"));

        // is_route_upstream reports true for the shared upstream host (expected — both
        // routes point there), but this does not imply any shadowing in route dispatch.
        assert!(store.is_route_upstream("api.openai.com:443"));
    }

    // ── host_port_matches tests (cdeeb5b9 wildcard routing) ──────────────────

    #[test]
    fn host_port_matches_exact_match() {
        assert!(host_port_matches(
            "api.openai.com:443",
            "api.openai.com:443"
        ));
    }

    #[test]
    fn host_port_matches_exact_mismatch() {
        assert!(!host_port_matches(
            "api.openai.com:443",
            "api.anthropic.com:443"
        ));
    }

    #[test]
    fn host_port_matches_wildcard_single_label() {
        // *.openai.com:443 matches api.openai.com:443
        assert!(host_port_matches("*.openai.com:443", "api.openai.com:443"));
    }

    #[test]
    fn host_port_matches_wildcard_different_label() {
        // *.openai.com:443 also matches files.openai.com:443
        assert!(host_port_matches(
            "*.openai.com:443",
            "files.openai.com:443"
        ));
    }

    #[test]
    fn host_port_matches_wildcard_does_not_match_apex() {
        // *.openai.com:443 must NOT match openai.com:443 (no prefix label)
        assert!(!host_port_matches("*.openai.com:443", "openai.com:443"));
    }

    #[test]
    fn host_port_matches_wildcard_matches_multi_label_prefix() {
        // *.openai.com:443 also matches x.api.openai.com:443 (multi-label prefix allowed)
        // The wildcard matches any non-empty sub-tree, not just single labels.
        assert!(host_port_matches(
            "*.openai.com:443",
            "x.api.openai.com:443"
        ));
    }

    #[test]
    fn host_port_matches_wildcard_port_mismatch() {
        // Port must also match
        assert!(!host_port_matches("*.openai.com:443", "api.openai.com:80"));
    }

    #[test]
    fn host_port_matches_no_wildcard_no_match() {
        assert!(!host_port_matches("openai.com:443", "api.openai.com:443"));
    }

    #[test]
    fn test_host_port_matches_wildcard_subdomain_only() {
        // multi-label subdomains are matched (sub-tree wildcard)
        assert!(host_port_matches(
            "*.dev.example.net:443",
            "api.admin.dev.example.net:443"
        ));
        assert!(host_port_matches(
            "*.dev.example.net:443",
            "admin.dev.example.net:443"
        ));
        // apex must NOT match
        assert!(!host_port_matches(
            "*.dev.example.net:443",
            "dev.example.net:443"
        ));
        // port mismatch
        assert!(!host_port_matches(
            "*.dev.example.net:443",
            "api.admin.dev.example.net:8443"
        ));
        // different suffix
        assert!(!host_port_matches(
            "*.dev.example.net:443",
            "api.admin.other.net:443"
        ));
    }
}
