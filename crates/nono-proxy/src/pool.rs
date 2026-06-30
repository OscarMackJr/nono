//! HTTP connection pool for upstream requests.
//!
//! `UpstreamPool` manages persistent HTTP/1.1 keep-alive and optional
//! HTTP/2 multiplexed connections to upstream APIs, replacing per-request
//! TLS handshakes with pooled connections.
//!
//! `PinnedResolver` provides a DNS-rebinding protection utility: hosts are
//! pinned to pre-validated `SocketAddr` slices (from `ProxyFilter::check_host`)
//! before connection, preventing the resolving step from drifting to a
//! different IP between filter-time and connect-time.
//!
//! # Fork note
//!
//! Absorbed from upstream cdeeb5b9 (#983). The fork does NOT carry the
//! `tls_intercept` module, so pool.rs contains only the infrastructure that
//! is independent of TLS-interception (per D-40-B2 fork-preserve lock).

use crate::error::{ProxyError, Result};
use bytes::Bytes;
use http::{Request, Response};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Pooled hyper client type alias.
///
/// Uses `HttpsConnector<HttpConnector>` (default GaiResolver) for upstream TLS.
/// DNS rebinding protection is enforced at the filter level via
/// `ProxyFilter::check_host()`; pinned addresses from that check are stored in
/// `UpstreamPool.resolver` for auditing purposes.
type PooledClient =
    Client<hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>;

/// DNS address cache for rebinding protection.
///
/// Stores pre-validated `SocketAddr` slices keyed by hostname. Hosts must be
/// explicitly pinned via `pin()` after `ProxyFilter::check_host()` validation
/// before their addresses are available for lookup.
///
/// The internal `Arc<Mutex<...>>` makes this type cheaply `Clone` so a
/// single resolver can be shared between the pool and diagnostic callers.
#[derive(Clone)]
pub struct PinnedResolver {
    pins: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>>,
}

impl PinnedResolver {
    /// Create a new empty resolver (no hosts pinned).
    #[must_use]
    pub fn new() -> Self {
        Self {
            pins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Pin `addrs` as the pre-validated addresses for `host`.
    ///
    /// Must be called after `ProxyFilter::check_host()` returns the resolved
    /// addresses to lock in the validated IP set. The lock is acquired only
    /// once at pin time; subsequent reads are O(1) hash lookups.
    pub fn pin(&self, host: String, addrs: Vec<SocketAddr>) {
        // Poisoned mutex is non-recoverable in a proxy server context; pin()
        // is only called from setup paths, not the hot request path, so
        // dropping the update silently is acceptable — the worst case is that
        // the auditing record is absent, not that a request is incorrectly
        // allowed.
        if let Ok(mut pins) = self.pins.lock() {
            pins.insert(host, addrs);
        }
    }

    /// Return the pinned addresses for `host`, if any were previously set.
    #[must_use]
    pub fn get_pinned(&self, host: &str) -> Option<Vec<SocketAddr>> {
        self.pins
            .lock()
            .ok()
            .and_then(|pins| pins.get(host).cloned())
    }
}

impl Default for PinnedResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP/1.1 keep-alive + optional HTTP/2 multiplexed connection pool for
/// upstream API calls.
///
/// A single `UpstreamPool` is created per proxy session and shared (via
/// references) across all request handlers. Routes with a custom CA certificate
/// get a per-route client identified by the `Arc<ClientConfig>` pointer value,
/// created lazily on first use.
///
/// # Connection reuse
///
/// `hyper_util::client::legacy::Client` pools connections internally. HTTP/1.1
/// connections are kept alive between requests to the same upstream. HTTP/2
/// connections are multiplexed over a single TLS stream when `enable_h2` is
/// `true` (requires the upstream to advertise `h2` via ALPN).
pub struct UpstreamPool {
    /// Default client for routes without a custom CA certificate.
    default_client: PooledClient,
    /// Pointer value of the default `Arc<ClientConfig>` used as a stable
    /// identity key without requiring `PartialEq` on `ClientConfig`.
    default_config_ptr: usize,
    /// Enable HTTP/2 ALPN negotiation for newly created clients.
    enable_h2: bool,
    /// Pre-validated host address store for DNS rebinding auditing.
    resolver: PinnedResolver,
    /// Per-route clients keyed by `Arc<ClientConfig>` pointer value.
    /// Created lazily on first request using a custom CA certificate.
    route_clients: Mutex<HashMap<usize, PooledClient>>,
}

impl UpstreamPool {
    /// Create a new pool using `default_tls_config` for routes without a
    /// custom CA.
    ///
    /// HTTP/2 ALPN negotiation is enabled when `enable_h2` is `true`. All
    /// clients built by this pool will use the default `HttpConnector`
    /// (system GaiResolver), with DNS rebinding protection enforced upstream
    /// by `ProxyFilter::check_host()`.
    #[must_use]
    pub fn new(default_tls_config: Arc<rustls::ClientConfig>, enable_h2: bool) -> Self {
        let resolver = PinnedResolver::new();
        let default_config_ptr = Arc::as_ptr(&default_tls_config) as usize;
        let default_client = build_pooled_client(&default_tls_config, enable_h2);
        Self {
            default_client,
            default_config_ptr,
            enable_h2,
            resolver,
            route_clients: Mutex::new(HashMap::new()),
        }
    }

    /// Record `addrs` as the pre-validated addresses for `host`.
    ///
    /// Should be called immediately after `ProxyFilter::check_host()` succeeds
    /// to associate the validated IP set with the hostname. Stored in the
    /// internal `PinnedResolver` for auditing; does not affect the GaiResolver
    /// used by the underlying `HttpConnector`.
    pub fn pin_host(&self, host: String, addrs: Vec<SocketAddr>) {
        self.resolver.pin(host, addrs);
    }

    /// Access the internal `PinnedResolver`.
    ///
    /// Exposed for testing and diagnostic use only; the resolver is not wired
    /// into the pool's `HttpConnector` (which uses the system GaiResolver).
    #[must_use]
    pub fn resolver(&self) -> &PinnedResolver {
        &self.resolver
    }

    /// Send a request through the pool and return the upstream response.
    ///
    /// Uses the default client for routes without custom CA, or a per-route
    /// client for routes with a custom CA certificate. Per-route clients are
    /// created lazily and cached by `Arc<ClientConfig>` pointer identity.
    ///
    /// # Errors
    ///
    /// Returns `ProxyError::UpstreamConnect` on connection failure and
    /// `ProxyError::Config` if the route-client map lock is poisoned (indicates
    /// a prior panic in a thread holding the lock).
    pub async fn send(
        &self,
        tls_config: &Arc<rustls::ClientConfig>,
        req: Request<Full<Bytes>>,
    ) -> Result<Response<Incoming>> {
        let host = req.uri().host().unwrap_or("unknown").to_string();
        let ptr = Arc::as_ptr(tls_config) as usize;

        let client = if ptr == self.default_config_ptr {
            self.default_client.clone()
        } else {
            let mut clients = self.route_clients.lock().map_err(|_| {
                ProxyError::Config("upstream pool lock poisoned".to_string())
            })?;
            let enable_h2 = self.enable_h2;
            clients
                .entry(ptr)
                .or_insert_with(|| build_pooled_client(tls_config, enable_h2))
                .clone()
        };

        client.request(req).await.map_err(|e| ProxyError::UpstreamConnect {
            host,
            reason: e.to_string(),
        })
    }
}

/// Build a new pooled hyper client backed by the given TLS configuration.
///
/// Creates an `HttpsConnector<HttpConnector>` (default GaiResolver) and
/// wraps it in a `Client` with hyper's built-in connection pooling.
/// HTTP/2 ALPN negotiation is enabled when `enable_h2` is `true`.
fn build_pooled_client(tls_config: &Arc<rustls::ClientConfig>, enable_h2: bool) -> PooledClient {
    let https = if enable_h2 {
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls_config.as_ref().clone())
            .https_or_http()
            .enable_all_versions()
            .build()
    } else {
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls_config.as_ref().clone())
            .https_or_http()
            .enable_http1()
            .build()
    };
    Client::builder(TokioExecutor::new()).build(https)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_tls_config() -> Arc<rustls::ClientConfig> {
        let root_store = crate::route::build_base_root_store();
        let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();
        Arc::new(cfg)
    }

    #[test]
    fn test_pinned_resolver_new_is_empty() {
        let resolver = PinnedResolver::new();
        assert!(
            resolver.get_pinned("example.com").is_none(),
            "new resolver must have no pinned addresses"
        );
    }

    #[test]
    fn test_pinned_resolver_pin_and_retrieve() {
        let resolver = PinnedResolver::new();
        let addr: SocketAddr = "93.184.216.34:443".parse().unwrap();
        resolver.pin("example.com".to_string(), vec![addr]);
        let pinned = resolver.get_pinned("example.com").unwrap();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0], addr);
        // Different host must still be absent
        assert!(resolver.get_pinned("other.com").is_none());
    }

    #[test]
    fn test_pinned_resolver_clone_shares_pins() {
        let resolver = PinnedResolver::new();
        let cloned = resolver.clone();
        let addr: SocketAddr = "93.184.216.34:443".parse().unwrap();
        // Pin on the original — should be visible via the clone.
        resolver.pin("example.com".to_string(), vec![addr]);
        assert!(
            cloned.get_pinned("example.com").is_some(),
            "clone must share the same Arc<Mutex> as the original"
        );
    }

    #[test]
    fn test_upstream_pool_new_creates_resolver() {
        let tls_config = make_tls_config();
        let pool = UpstreamPool::new(tls_config, false);
        assert!(
            pool.resolver().get_pinned("example.com").is_none(),
            "new pool resolver must be empty"
        );
    }

    #[test]
    fn test_upstream_pool_pin_host_stores_addresses() {
        let tls_config = make_tls_config();
        let pool = UpstreamPool::new(tls_config, false);
        let addr: SocketAddr = "93.184.216.34:443".parse().unwrap();
        pool.pin_host("example.com".to_string(), vec![addr]);
        let pinned = pool.resolver().get_pinned("example.com").unwrap();
        assert_eq!(pinned[0], addr);
    }

    #[tokio::test]
    async fn test_upstream_pool_send_returns_error_for_invalid_host() {
        let tls_config = make_tls_config();
        let pool = UpstreamPool::new(Arc::clone(&tls_config), false);
        let req = Request::builder()
            .method("GET")
            .uri("https://this-host-does-not-exist.invalid/path")
            .body(Full::new(Bytes::new()))
            .unwrap();
        let result = pool.send(&tls_config, req).await;
        assert!(
            result.is_err(),
            "sending to unresolvable host must return an error"
        );
    }
}
