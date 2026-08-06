//! OAuth2 `client_credentials` token exchange and caching.
//!
//! Provides [`TokenCache`] — a thread-safe cache that holds an OAuth2 access
//! token and refreshes it on demand before expiry. Designed for the reverse
//! proxy credential injection flow where the agent never sees the real
//! client_id/client_secret.
//!
//! ## Design
//!
//! - **No background tasks**: Token validity is checked on each use via
//!   [`TokenCache::get_or_refresh()`]. If the cached token is about to expire
//!   (within 30 seconds), a synchronous refresh is attempted.
//! - **Graceful degradation**: If a refresh attempt fails but a stale token
//!   exists, the stale token is returned with a warning log. This avoids
//!   transient auth-server outages from cascading into request failures.
//! - **TLS via rustls**: Uses the same `webpki-roots` + `tokio-rustls` stack
//!   as the rest of the proxy. No additional HTTP client dependencies.

use crate::error::{ProxyError, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_rustls::TlsConnector;
use tracing::{debug, warn};
use zeroize::Zeroizing;

/// Buffer subtracted from `expires_in` to refresh before the token actually
/// expires. Avoids edge cases where a token expires between check and use.
const EXPIRY_BUFFER_SECS: u64 = 30;

/// Default TTL when the token endpoint omits `expires_in`.
const DEFAULT_EXPIRES_IN_SECS: u64 = 3600;

/// Timeout for the TCP connect + TLS handshake + HTTP exchange.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum response body size from the token endpoint (64 KiB).
const MAX_TOKEN_RESPONSE: usize = 64 * 1024;

// ────────────────────────────────────────────────────────────────────────────
// Public types
// ────────────────────────────────────────────────────────────────────────────

/// Resolved OAuth2 credentials ready for token exchange.
///
/// All secret fields use [`Zeroizing`] so they are zeroed on drop.
pub struct OAuth2ExchangeConfig {
    pub token_url: String,
    pub client_id: Zeroizing<String>,
    pub client_secret: Zeroizing<String>,
    pub scope: String,
}

/// Custom Debug that redacts secrets.
impl std::fmt::Debug for OAuth2ExchangeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuth2ExchangeConfig")
            .field("token_url", &self.token_url)
            .field("client_id", &"[REDACTED]")
            .field("client_secret", &"[REDACTED]")
            .field("scope", &self.scope)
            .finish()
    }
}

/// Thread-safe OAuth2 access-token cache with on-demand refresh.
pub struct TokenCache {
    token: Arc<RwLock<CachedToken>>,
    config: OAuth2ExchangeConfig,
    tls_connector: TlsConnector,
}

impl std::fmt::Debug for TokenCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenCache")
            .field("config", &self.config)
            .finish()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Internal types
// ────────────────────────────────────────────────────────────────────────────

struct CachedToken {
    access_token: Zeroizing<String>,
    expires_at: Instant,
}

// ────────────────────────────────────────────────────────────────────────────
// TokenCache implementation
// ────────────────────────────────────────────────────────────────────────────

impl TokenCache {
    /// Create a new cache and perform the **initial** token exchange.
    ///
    /// Called during [`CredentialStore::load()`](crate::credential::CredentialStore::load)
    /// which is synchronous. We bridge into async via
    /// [`tokio::runtime::Handle::current().block_on()`].
    ///
    /// # Runtime requirements
    ///
    /// MN-01-M (Phase 22 review): this constructor MUST be invoked from a
    /// thread that has an active tokio runtime in its TLS slot but is NOT
    /// itself executing inside a tokio runtime worker. Specifically:
    ///
    /// - The inner [`Handle::current`] previously panicked if no runtime
    ///   context was available; we now detect that case via
    ///   [`Handle::try_current`] and return a structured
    ///   [`ProxyError::Config`] error instead — callers can degrade
    ///   gracefully rather than crash.
    /// - The inner [`Handle::block_on`] will still deadlock or panic if
    ///   invoked from inside an async task on a single-threaded scheduler.
    ///   Do NOT call this from inside `async fn` bodies on a current-thread
    ///   runtime; call from synchronous startup code or from a
    ///   [`tokio::task::spawn_blocking`] task.
    ///
    /// In short: call this from synchronous startup code that runs inside
    /// a `Runtime::block_on(async { ... })` or `Runtime::enter()` guard,
    /// not from inside an `async fn` itself. The proxy's startup flow
    /// (`CredentialStore::load` → `TokenCache::new`) satisfies this.
    ///
    /// # Errors
    ///
    /// Returns [`ProxyError::Config`] if no tokio runtime context is
    /// available (caller invoked outside `Runtime::block_on` /
    /// `Runtime::enter` scope). Returns [`ProxyError::OAuth2Exchange`] if
    /// the initial exchange fails (DNS, TCP, TLS, non-200, malformed JSON).
    /// The calling code skips the route so the proxy can still start for
    /// other routes.
    pub fn new(config: OAuth2ExchangeConfig, tls_connector: TlsConnector) -> Result<Self> {
        // Convert the previously-implicit panic on missing runtime context
        // into a structured error. The original `Handle::current()` panic
        // message ("there is no reactor running") is opaque; this surfaces
        // a contributor-friendly diagnostic that points at the runtime
        // contract documented above.
        let runtime_handle = tokio::runtime::Handle::try_current().map_err(|e| {
            ProxyError::Config(format!(
                "TokenCache::new requires an active tokio runtime context \
                 (call from Runtime::block_on/enter or tokio::task::spawn_blocking): {e}"
            ))
        })?;
        let (access_token, expires_in) =
            runtime_handle.block_on(exchange_token(&config, &tls_connector))?;

        let expires_at = Instant::now() + expires_in;
        debug!(
            "OAuth2 initial token acquired, expires in {}s",
            expires_in.as_secs()
        );

        Ok(Self {
            token: Arc::new(RwLock::new(CachedToken {
                access_token,
                expires_at,
            })),
            config,
            tls_connector,
        })
    }

    /// Return a valid access token, refreshing if needed.
    ///
    /// If the cached token is still valid (expires > 30 s from now), returns
    /// the cached value without any network call.
    ///
    /// If expired, attempts one exchange. On failure, returns the **stale**
    /// token with a warning — better to try a possibly-expired token than to
    /// fail the request outright.
    pub async fn get_or_refresh(&self) -> Zeroizing<String> {
        // Fast path — token still valid.
        {
            let guard = self.token.read().await;
            if Instant::now() + Duration::from_secs(EXPIRY_BUFFER_SECS) < guard.expires_at {
                return guard.access_token.clone();
            }
        }

        // Slow path — need to refresh.
        let mut guard = self.token.write().await;

        // Double-check after acquiring write lock (another task may have refreshed).
        if Instant::now() + Duration::from_secs(EXPIRY_BUFFER_SECS) < guard.expires_at {
            return guard.access_token.clone();
        }

        match exchange_token(&self.config, &self.tls_connector).await {
            Ok((new_token, expires_in)) => {
                debug!(
                    "OAuth2 token refreshed, expires in {}s",
                    expires_in.as_secs()
                );
                guard.access_token = new_token;
                guard.expires_at = Instant::now() + expires_in;
                guard.access_token.clone()
            }
            Err(e) => {
                warn!("OAuth2 token refresh failed, returning stale token: {}", e);
                guard.access_token.clone()
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Token exchange (HTTP POST)
// ────────────────────────────────────────────────────────────────────────────

/// Perform a single `client_credentials` token exchange against the token
/// endpoint described in `config`.
///
/// Returns `(access_token, expires_in_duration)`.
async fn exchange_token(
    config: &OAuth2ExchangeConfig,
    tls_connector: &TlsConnector,
) -> Result<(Zeroizing<String>, Duration)> {
    let parsed = url::Url::parse(&config.token_url).map_err(|e| {
        ProxyError::OAuth2Exchange(format!("invalid token_url '{}': {}", config.token_url, e))
    })?;

    let is_https = validate_token_url_scheme(&parsed, &config.token_url)?;

    let host = parsed
        .host_str()
        .ok_or_else(|| {
            ProxyError::OAuth2Exchange(format!("missing host in token_url '{}'", config.token_url))
        })?
        .to_string();

    let default_port: u16 = if is_https { 443 } else { 80 };
    let port = parsed.port().unwrap_or(default_port);
    let path = if parsed.path().is_empty() {
        "/"
    } else {
        parsed.path()
    };
    let path_with_query = match parsed.query() {
        Some(q) => format!("{}?{}", path, q),
        None => path.to_string(),
    };

    // ── Build form body ──────────────────────────────────────────────────
    let body = build_token_request_body(&config.client_id, &config.client_secret, &config.scope);

    // ── Build HTTP/1.1 request ───────────────────────────────────────────
    let request = Zeroizing::new(format!(
        "POST {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/x-www-form-urlencoded\r\n\
         Content-Length: {}\r\n\
         Accept: application/json\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        path_with_query,
        host,
        body.len(),
        body.as_str()
    ));

    // ── TCP + optional TLS + HTTP exchange ───────────────────────────────
    let response_bytes =
        connect_and_send(&host, port, is_https, request.as_bytes(), tls_connector).await?;

    // ── Parse HTTP response ──────────────────────────────────────────────
    let response_str = String::from_utf8(response_bytes).map_err(|_| {
        ProxyError::OAuth2Exchange("token endpoint returned non-UTF-8 response".to_string())
    })?;

    // Split headers from body
    let body_start = response_str
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .or_else(|| response_str.find("\n\n").map(|i| i + 2))
        .ok_or_else(|| {
            ProxyError::OAuth2Exchange(
                "malformed HTTP response: no header/body separator".to_string(),
            )
        })?;

    // Check status code
    let status_line = response_str.lines().next().unwrap_or("");
    let status_code = parse_status_code(status_line);
    if !(200..300).contains(&status_code) {
        let body_preview: String = response_str[body_start..].chars().take(200).collect();
        return Err(ProxyError::OAuth2Exchange(format!(
            "token endpoint returned HTTP {}: {}",
            status_code, body_preview
        )));
    }

    let json_body = &response_str[body_start..];
    parse_token_response(json_body)
}

/// Validate a token endpoint URL's scheme.
///
/// HTTPS is always allowed. HTTP is only permitted for loopback addresses
/// (testing / local mock servers) — sending a client_secret or a JWT-SVID
/// assertion over plaintext HTTP to a non-loopback host would expose the
/// credential on the network. Any other scheme is rejected.
///
/// Returns `Ok(true)` for `https`, `Ok(false)` for loopback `http`.
fn validate_token_url_scheme(parsed: &url::Url, token_url: &str) -> Result<bool> {
    match parsed.scheme() {
        "https" => Ok(true),
        "http" => {
            let is_loopback = parsed.host_str().is_some_and(|h| {
                h == "localhost"
                    || h.parse::<std::net::IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback())
            });
            if is_loopback {
                Ok(false)
            } else {
                Err(ProxyError::OAuth2Exchange(format!(
                    "token_url '{}' must use https:// (http:// is only permitted for loopback addresses)",
                    token_url
                )))
            }
        }
        other => Err(ProxyError::OAuth2Exchange(format!(
            "unsupported scheme '{}' in token_url",
            other
        ))),
    }
}

/// Connect to `host:port` (TLS if `is_https`), send `request`, and read the
/// full response body. Shared by [`exchange_token`] (`client_credentials`)
/// and [`exchange_jwt_assertion`] (RFC 7523 `jwt-bearer`) so the TCP/TLS
/// connection setup is defined exactly once.
async fn connect_and_send(
    host: &str,
    port: u16,
    is_https: bool,
    request: &[u8],
    tls_connector: &TlsConnector,
) -> Result<Vec<u8>> {
    let addr = format!("{}:{}", host, port);

    tokio::time::timeout(EXCHANGE_TIMEOUT, async {
        let tcp = TcpStream::connect(&addr)
            .await
            .map_err(|e| ProxyError::OAuth2Exchange(format!("TCP connect to {}: {}", addr, e)))?;

        if is_https {
            let server_name =
                rustls::pki_types::ServerName::try_from(host.to_string()).map_err(|_| {
                    ProxyError::OAuth2Exchange(format!("invalid TLS server name: {}", host))
                })?;

            let mut tls = tls_connector.connect(server_name, tcp).await.map_err(|e| {
                ProxyError::OAuth2Exchange(format!("TLS handshake with {}: {}", host, e))
            })?;

            tls.write_all(request)
                .await
                .map_err(|e| ProxyError::OAuth2Exchange(format!("write to {}: {}", host, e)))?;
            tls.flush()
                .await
                .map_err(|e| ProxyError::OAuth2Exchange(format!("flush to {}: {}", host, e)))?;

            read_http_response(&mut tls).await
        } else {
            let mut tcp = tcp;
            tcp.write_all(request)
                .await
                .map_err(|e| ProxyError::OAuth2Exchange(format!("write to {}: {}", host, e)))?;
            tcp.flush()
                .await
                .map_err(|e| ProxyError::OAuth2Exchange(format!("flush to {}: {}", host, e)))?;

            read_http_response(&mut tcp).await
        }
    })
    .await
    .map_err(|_| ProxyError::OAuth2Exchange(format!("token exchange with {} timed out", addr)))?
}

/// Read a full HTTP response from a stream up to [`MAX_TOKEN_RESPONSE`] bytes.
async fn read_http_response<S: tokio::io::AsyncRead + Unpin>(stream: &mut S) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream
            .read(&mut tmp)
            .await
            .map_err(|e| ProxyError::OAuth2Exchange(format!("read response: {}", e)))?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > MAX_TOKEN_RESPONSE {
            return Err(ProxyError::OAuth2Exchange(format!(
                "token response exceeds {} bytes",
                MAX_TOKEN_RESPONSE
            )));
        }
    }
    Ok(buf)
}

/// Parse the HTTP status code from the status line.
fn parse_status_code(line: &str) -> u16 {
    // "HTTP/1.1 200 OK" -> "200"
    let mut parts = line.split_whitespace();
    parts.nth(1).and_then(|code| code.parse().ok()).unwrap_or(0)
}

// ────────────────────────────────────────────────────────────────────────────
// Request / response helpers (pub(crate) for testing)
// ────────────────────────────────────────────────────────────────────────────

/// Build the `application/x-www-form-urlencoded` body for the token request.
///
/// The `scope` parameter is omitted when empty.
fn build_token_request_body(
    client_id: &str,
    client_secret: &str,
    scope: &str,
) -> Zeroizing<String> {
    let mut body = Zeroizing::new(format!(
        "grant_type=client_credentials&client_id={}&client_secret={}",
        urlencoding::encode(client_id),
        urlencoding::encode(client_secret),
    ));
    if !scope.is_empty() {
        body.push_str(&format!("&scope={}", urlencoding::encode(scope)));
    }
    body
}

/// Parse a standard OAuth2 token response JSON.
///
/// Expects `{"access_token": "...", "expires_in": 3600, ...}`.
/// - `access_token` is required.
/// - `expires_in` defaults to [`DEFAULT_EXPIRES_IN_SECS`] if missing.
fn parse_token_response(json: &str) -> Result<(Zeroizing<String>, Duration)> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
        ProxyError::OAuth2Exchange(format!("invalid JSON from token endpoint: {}", e))
    })?;

    let access_token = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProxyError::OAuth2Exchange("token response missing 'access_token' field".to_string())
        })?;

    let expires_in_secs = value
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_EXPIRES_IN_SECS);

    Ok((
        Zeroizing::new(access_token.to_string()),
        Duration::from_secs(expires_in_secs),
    ))
}

// ────────────────────────────────────────────────────────────────────────────
// SPIFFE JWT-bearer client assertion (RFC 7523) token cache
// ────────────────────────────────────────────────────────────────────────────

/// Like [`TokenCache`] but exchanges a SPIFFE JWT-SVID as the OAuth2
/// `client_assertion` (RFC 7523 `urn:ietf:params:oauth:grant-type:jwt-bearer`
/// grant) instead of a `client_id`/`client_secret` pair.
///
/// OD-1 scope note: this is the narrow SPIFFE-scoped slice of the jwt-bearer
/// flow only. The general (non-SPIFFE) `client_credentials` route-wiring
/// layer (`CredentialStore.oauth2_routes`/`OAuth2Route`/`get_oauth2()`) that
/// upstream's `credential.rs` hunk assumes as pre-existing context traces to
/// unabsorbed `b1ecbc02` and is explicitly NOT built here.
pub struct SpiffeAssertionTokenCache {
    token: Arc<RwLock<CachedToken>>,
    jwt_source: Arc<crate::spiffe::SpiffeJwtSource>,
    token_url: String,
    assertion_audience: Vec<String>,
    extra_params: Vec<(String, String)>,
    tls_connector: TlsConnector,
    pub workload_spiffe_id: String,
}

/// Custom Debug that redacts nothing secret (there is nothing secret in this
/// struct's non-token fields) but avoids deriving Debug on `jwt_source`
/// (`SpiffeJwtSource` already has its own redacted `Debug`) and keeps the
/// output stable, mirroring [`TokenCache`]'s redaction idiom.
impl std::fmt::Debug for SpiffeAssertionTokenCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiffeAssertionTokenCache")
            .field("token_url", &self.token_url)
            .finish()
    }
}

impl SpiffeAssertionTokenCache {
    /// Create a new cache and perform the **initial** jwt-bearer token
    /// exchange. Unlike [`TokenCache::new`], this is a plain `async fn` —
    /// callers already run inside an async context (`CredentialStore::load`
    /// is itself async under D-04), so no `Handle::block_on` bridging is
    /// needed here.
    pub async fn new(
        token_url: String,
        jwt_source: Arc<crate::spiffe::SpiffeJwtSource>,
        assertion_audience: Vec<String>,
        extra_params: Vec<(String, String)>,
        tls_connector: TlsConnector,
    ) -> Result<Self> {
        let (access_token, expires_in, workload_spiffe_id) = exchange_jwt_assertion(
            &token_url,
            &jwt_source,
            &assertion_audience,
            &extra_params,
            &tls_connector,
        )
        .await?;

        debug!(
            "OAuth2 jwt-bearer initial token acquired for {}, expires in {}s",
            workload_spiffe_id,
            expires_in.as_secs()
        );

        Ok(Self {
            token: Arc::new(RwLock::new(CachedToken {
                access_token,
                expires_at: Instant::now() + expires_in,
            })),
            jwt_source,
            token_url,
            assertion_audience,
            extra_params,
            tls_connector,
            workload_spiffe_id,
        })
    }

    /// Return a valid access token, refreshing if needed.
    ///
    /// Unlike [`TokenCache::get_or_refresh`], this can fail: if the SVID
    /// fetch itself fails (workload identity is gone — SPIRE agent down,
    /// socket unreachable, SVID expired with no refresh possible), the
    /// error is propagated rather than masked by a stale-token fallback
    /// (T-113-10). Only a failure of the *token exchange* step (network or
    /// IdP-side, the SVID fetch having succeeded) falls back to a stale
    /// token, matching [`TokenCache::get_or_refresh`]'s existing
    /// graceful-degradation behavior for that failure class.
    pub async fn get_or_refresh(&self) -> Result<Zeroizing<String>> {
        {
            let guard = self.token.read().await;
            if Instant::now() + Duration::from_secs(EXPIRY_BUFFER_SECS) < guard.expires_at {
                return Ok(guard.access_token.clone());
            }
        }

        let mut guard = self.token.write().await;
        // Double-check after acquiring write lock (another task may have refreshed).
        if Instant::now() + Duration::from_secs(EXPIRY_BUFFER_SECS) < guard.expires_at {
            return Ok(guard.access_token.clone());
        }

        match exchange_jwt_assertion(
            &self.token_url,
            &self.jwt_source,
            &self.assertion_audience,
            &self.extra_params,
            &self.tls_connector,
        )
        .await
        {
            Ok((new_token, expires_in, _)) => {
                debug!(
                    "OAuth2 jwt-bearer token refreshed, expires in {}s",
                    expires_in.as_secs()
                );
                guard.access_token = new_token;
                guard.expires_at = Instant::now() + expires_in;
                Ok(guard.access_token.clone())
            }
            // SVID fetch failed - workload identity is gone; fail the
            // request (do not serve stale) per T-113-10.
            Err(e @ ProxyError::Credential(_)) => Err(e),
            // Token exchange failed (network/auth) - use stale token if
            // available, matching TokenCache::get_or_refresh's existing
            // graceful-degradation idiom for the SAME class of failure.
            Err(e) => {
                warn!(
                    "OAuth2 jwt-bearer token refresh failed, returning stale token: {}",
                    e
                );
                Ok(guard.access_token.clone())
            }
        }
    }
}

/// Perform a single RFC 7523 jwt-bearer token exchange, using a freshly
/// fetched SPIFFE JWT-SVID as the `client_assertion`.
///
/// Returns `(access_token, expires_in_duration, workload_spiffe_id)`.
async fn exchange_jwt_assertion(
    token_url: &str,
    jwt_source: &crate::spiffe::SpiffeJwtSource,
    audience: &[String],
    extra_params: &[(String, String)],
    tls_connector: &TlsConnector,
) -> Result<(Zeroizing<String>, Duration, String)> {
    // Fetches (and internally caches/refreshes) the JWT-SVID from the SPIRE
    // Workload API. A failure here is a `ProxyError::Credential` — this is
    // the "workload identity is gone" case `get_or_refresh` fails closed on.
    let (assertion, spiffe_id) = jwt_source.fetch_token(audience).await?;

    // ── Build form body (URL-encoded per T-113-08) ───────────────────────
    let mut body = Zeroizing::new(format!(
        "grant_type={}&client_assertion_type={}&assertion={}",
        urlencoding::encode("urn:ietf:params:oauth:grant-type:jwt-bearer"),
        urlencoding::encode("urn:ietf:params:oauth:client-assertion-type:jwt-bearer"),
        urlencoding::encode(assertion.as_str()),
    ));
    for (k, v) in extra_params {
        body.push_str(&format!(
            "&{}={}",
            urlencoding::encode(k),
            urlencoding::encode(v)
        ));
    }

    let parsed = url::Url::parse(token_url).map_err(|e| {
        ProxyError::OAuth2Exchange(format!("invalid token_url '{}': {}", token_url, e))
    })?;

    // Reuses the same HTTPS-or-loopback validation as exchange_token (T-113-09).
    let is_https = validate_token_url_scheme(&parsed, token_url)?;

    let host = parsed
        .host_str()
        .ok_or_else(|| {
            ProxyError::OAuth2Exchange(format!("missing host in token_url '{}'", token_url))
        })?
        .to_string();

    let default_port: u16 = if is_https { 443 } else { 80 };
    let port = parsed.port().unwrap_or(default_port);
    let path = if parsed.path().is_empty() {
        "/"
    } else {
        parsed.path()
    };
    let path_with_query = match parsed.query() {
        Some(q) => format!("{}?{}", path, q),
        None => path.to_string(),
    };

    let request = Zeroizing::new(format!(
        "POST {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/x-www-form-urlencoded\r\n\
         Content-Length: {}\r\n\
         Accept: application/json\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        path_with_query,
        host,
        body.len(),
        body.as_str()
    ));

    // Reuses the same connect + send + read chain as exchange_token.
    let response_bytes =
        connect_and_send(&host, port, is_https, request.as_bytes(), tls_connector).await?;

    let response_str = String::from_utf8(response_bytes).map_err(|_| {
        ProxyError::OAuth2Exchange("token endpoint returned non-UTF-8 response".to_string())
    })?;

    let body_start = response_str
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .or_else(|| response_str.find("\n\n").map(|i| i + 2))
        .ok_or_else(|| {
            ProxyError::OAuth2Exchange(
                "malformed HTTP response: no header/body separator".to_string(),
            )
        })?;

    let status_line = response_str.lines().next().unwrap_or("");
    let status_code = parse_status_code(status_line);
    if !(200..300).contains(&status_code) {
        let body_preview: String = response_str[body_start..].chars().take(200).collect();
        return Err(ProxyError::OAuth2Exchange(format!(
            "token endpoint returned HTTP {}: {}",
            status_code, body_preview
        )));
    }

    // Reuses the same JSON-parse helper as exchange_token; wraps its result
    // with spiffe_id as the 3rd tuple element.
    let json_body = &response_str[body_start..];
    let (access_token, expires_in) = parse_token_response(json_body)?;
    Ok((access_token, expires_in, spiffe_id))
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── parse_token_response ─────────────────────────────────────────────

    #[test]
    fn test_parse_token_response_success() {
        let json =
            r#"{"access_token":"eyJhbGciOiJSUzI1NiJ9","token_type":"Bearer","expires_in":3600}"#;
        let (token, expires) = parse_token_response(json).unwrap();
        assert_eq!(token.as_str(), "eyJhbGciOiJSUzI1NiJ9");
        assert_eq!(expires, Duration::from_secs(3600));
    }

    #[test]
    fn test_parse_token_response_missing_expires_defaults() {
        let json = r#"{"access_token":"tok_abc","token_type":"Bearer"}"#;
        let (token, expires) = parse_token_response(json).unwrap();
        assert_eq!(token.as_str(), "tok_abc");
        assert_eq!(expires, Duration::from_secs(DEFAULT_EXPIRES_IN_SECS));
    }

    #[test]
    fn test_parse_token_response_missing_access_token_errors() {
        let json = r#"{"token_type":"Bearer","expires_in":3600}"#;
        let err = parse_token_response(json).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("access_token"),
            "error should mention access_token: {}",
            msg
        );
    }

    #[test]
    fn test_parse_token_response_non_json_errors() {
        let err = parse_token_response("this is not json").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid JSON"),
            "error should mention invalid JSON: {}",
            msg
        );
    }

    // ── build_token_request_body ─────────────────────────────────────────

    #[test]
    fn test_build_token_request_body() {
        let body = build_token_request_body("my-client", "s3cret!", "read write");
        assert!(body.contains("grant_type=client_credentials"));
        assert!(body.contains("client_id=my-client"));
        assert!(body.contains("client_secret=s3cret%21"));
        assert!(body.contains("scope=read%20write"));
    }

    #[test]
    fn test_build_token_request_body_no_scope() {
        let body = build_token_request_body("cid", "csec", "");
        assert!(body.contains("grant_type=client_credentials"));
        assert!(body.contains("client_id=cid"));
        assert!(body.contains("client_secret=csec"));
        assert!(!body.contains("scope="), "empty scope should be omitted");
    }

    // ── parse_status_code ────────────────────────────────────────────────

    #[test]
    fn test_parse_status_code_200() {
        assert_eq!(parse_status_code("HTTP/1.1 200 OK"), 200);
    }

    #[test]
    fn test_parse_status_code_401() {
        assert_eq!(parse_status_code("HTTP/1.1 401 Unauthorized"), 401);
    }

    #[test]
    fn test_parse_status_code_garbage() {
        assert_eq!(parse_status_code("not http"), 0);
    }

    // ── TokenCache expiry logic ──────────────────────────────────────────

    #[tokio::test]
    async fn test_token_cache_returns_valid_token() {
        // Construct a cache with a token that expires far in the future.
        let cache = make_test_cache("valid_token", Duration::from_secs(3600));
        let token = cache.get_or_refresh().await;
        assert_eq!(token.as_str(), "valid_token");
    }

    #[tokio::test]
    async fn test_token_cache_detects_expiry() {
        // Token that "expired" 10 seconds ago. Because exchange_token will
        // fail (no real server), the stale token is returned.
        let cache = make_test_cache("stale_token", Duration::from_secs(0));
        // Manually set expires_at to the past.
        {
            let mut guard = cache.token.write().await;
            guard.expires_at = Instant::now() - Duration::from_secs(10);
        }
        let token = cache.get_or_refresh().await;
        // Should still get the stale token (graceful degradation).
        assert_eq!(token.as_str(), "stale_token");
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    /// Build a `TokenCache` with a pre-populated token for unit tests.
    /// The `exchange_token` config points to a non-routable address so any
    /// actual exchange attempt will fail (which is fine — we test cache logic).
    fn make_test_cache(token: &str, ttl: Duration) -> TokenCache {
        let config = OAuth2ExchangeConfig {
            token_url: "https://127.0.0.1:1/oauth/token".to_string(),
            client_id: Zeroizing::new("test-client".to_string()),
            client_secret: Zeroizing::new("test-secret".to_string()),
            scope: String::new(),
        };

        // Build a minimal TLS connector (never actually used in these tests).
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();
        let tls_connector = TlsConnector::from(Arc::new(tls_config));

        TokenCache {
            token: Arc::new(RwLock::new(CachedToken {
                access_token: Zeroizing::new(token.to_string()),
                expires_at: Instant::now() + ttl,
            })),
            config,
            tls_connector,
        }
    }

    // ── validate_token_url_scheme (shared by exchange_token and
    //    exchange_jwt_assertion — Task 1 acceptance criterion) ────────────
    //
    // exchange_jwt_assertion cannot be exercised directly in a unit test:
    // it fetches the JWT-SVID via `jwt_source.fetch_token()` BEFORE
    // validating the token_url scheme (matching upstream c831dade's exact
    // ordering), and `SpiffeJwtSource` has no test-only constructor that
    // bypasses a live SPIRE Workload API connection (see spiffe.rs's own
    // `test_jwt_source_fails_closed_on_missing_socket`). These tests
    // exercise `validate_token_url_scheme` directly instead — the exact
    // function both `exchange_token` and `exchange_jwt_assertion` delegate
    // to for this check, so the rejection behavior under test is identical.

    #[test]
    fn test_validate_token_url_scheme_accepts_https() {
        let url = url::Url::parse("https://auth.example.com/oauth/token").unwrap();
        let is_https = validate_token_url_scheme(&url, "https://auth.example.com/oauth/token")
            .expect("https should be accepted");
        assert!(is_https);
    }

    #[test]
    fn test_validate_token_url_scheme_accepts_loopback_http() {
        // Matches exchange_token's existing loopback check exactly (verbatim
        // from upstream c831dade): `localhost` and IPv4 loopback are covered.
        // IPv6 `[::1]` is NOT special-cased by that check (Url::host_str()
        // returns the bracketed form for IPv6 hosts, which does not parse as
        // an IpAddr) — an inherited limitation from upstream's own logic,
        // not something this plan's absorb introduces or is scoped to fix.
        for token_url in ["http://localhost:8080/token", "http://127.0.0.1:8080/token"] {
            let url = url::Url::parse(token_url).unwrap();
            let is_https = validate_token_url_scheme(&url, token_url).unwrap_or_else(|e| {
                panic!("loopback http should be accepted for {token_url}: {e}")
            });
            assert!(!is_https);
        }
    }

    #[test]
    fn test_validate_token_url_scheme_rejects_non_loopback_http() {
        let token_url = "http://auth.example.com/oauth/token";
        let url = url::Url::parse(token_url).unwrap();
        let err = validate_token_url_scheme(&url, token_url).unwrap_err();
        match err {
            ProxyError::OAuth2Exchange(msg) => {
                assert!(
                    msg.contains("https://"),
                    "error should explain https:// is required: {msg}"
                );
            }
            other => panic!("expected ProxyError::OAuth2Exchange, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_token_url_scheme_rejects_other_scheme() {
        let token_url = "ftp://auth.example.com/oauth/token";
        let url = url::Url::parse(token_url).unwrap();
        let err = validate_token_url_scheme(&url, token_url).unwrap_err();
        assert!(matches!(err, ProxyError::OAuth2Exchange(_)));
    }
}
