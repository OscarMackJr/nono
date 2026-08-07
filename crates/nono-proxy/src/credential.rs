//! Credential loading and management for reverse proxy mode.
//!
//! Loads API credentials from the system keystore or 1Password at proxy startup.
//! Credentials are stored in `Zeroizing<String>` and injected into
//! requests via headers, URL paths, query parameters, or Basic Auth.
//! The sandboxed agent never sees the real credentials.
//!
//! Route-level configuration (upstream URL, L7 endpoint rules, custom TLS CA)
//! is handled by [`crate::route::RouteStore`], which loads independently of
//! credentials. This module handles only credential-specific concerns.

use crate::config::{ClientAssertionConfig, InjectMode, RouteConfig};
use crate::diagnostic::ProxyDiagnostic;
use crate::error::{ProxyError, Result};
use base64::Engine;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio_rustls::TlsConnector;
use tracing::{debug, warn};
use zeroize::Zeroizing;

/// Result of loading credentials at proxy startup.
#[derive(Debug)]
pub struct CredentialLoadOutcome {
    /// Loaded store; may omit routes whose credentials were unavailable.
    pub store: CredentialStore,
    /// Per-route warnings for missing or unavailable credentials.
    pub diagnostics: Vec<ProxyDiagnostic>,
}

impl CredentialLoadOutcome {
    /// Extract the store, discarding diagnostics.
    #[must_use]
    pub fn into_store(self) -> CredentialStore {
        self.store
    }
}

/// A loaded credential ready for injection.
///
/// Contains only credential-specific fields (injection mode, header name/value,
/// raw secret). Route-level configuration (upstream URL, L7 endpoint rules,
/// custom TLS CA) is stored in [`crate::route::LoadedRoute`].
pub struct LoadedCredential {
    /// Injection mode
    pub inject_mode: InjectMode,
    /// Raw credential value from keystore (for modes that need it directly)
    pub raw_credential: Zeroizing<String>,

    // --- Header mode ---
    /// Header name to inject (e.g., "Authorization")
    pub header_name: String,
    /// Formatted header value (e.g., "Bearer sk-...")
    pub header_value: Zeroizing<String>,

    // --- URL path mode ---
    /// Pattern to match in incoming path (with {} placeholder)
    pub path_pattern: Option<String>,
    /// Pattern for outgoing path (with {} placeholder)
    pub path_replacement: Option<String>,

    // --- Query param mode ---
    /// Query parameter name
    pub query_param_name: Option<String>,
}

/// Custom Debug impl that redacts secret values to prevent accidental leakage
/// in logs, panic messages, or debug output.
impl std::fmt::Debug for LoadedCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedCredential")
            .field("inject_mode", &self.inject_mode)
            .field("raw_credential", &"[REDACTED]")
            .field("header_name", &self.header_name)
            .field("header_value", &"[REDACTED]")
            .field("path_pattern", &self.path_pattern)
            .field("path_replacement", &self.path_replacement)
            .field("query_param_name", &self.query_param_name)
            .finish()
    }
}

/// A route authenticated via the RFC 7523 jwt-bearer OAuth2 `client_assertion`
/// flow: a SPIFFE JWT-SVID is exchanged for an access token at `oauth2.token_url`,
/// and that access token (not the raw SVID) is injected into upstream requests.
///
/// OD-1 scope note: this is the narrow SPIFFE-scoped slice of the jwt-bearer
/// flow only. The general (non-SPIFFE) `client_credentials` route-wiring
/// layer that upstream's diff builds alongside this one — a parallel map,
/// route struct, and accessor for plain client_id/client_secret exchange —
/// is explicitly NOT built here: it traces to unabsorbed `b1ecbc02` and is
/// out of this plan's scope. See OD-1 in `113-03-PLAN.md` for the full
/// disposition.
#[derive(Debug)]
pub struct SpiffeAssertionRoute {
    /// Cached, auto-refreshing OAuth2 access token, backed by a JWT-SVID
    /// assertion fetched from the SPIRE Workload API.
    pub cache: crate::oauth2::SpiffeAssertionTokenCache,
    /// Upstream URL to forward to, mirroring [`RouteConfig::upstream`].
    pub upstream: String,
    /// HTTP header name the exchanged access token is injected under,
    /// mirroring [`RouteConfig::inject_header`].
    pub inject_header: String,
}

/// Credential store for all configured routes.
///
/// # Credential-match policy (D-20 replay of upstream f77e0e3)
///
/// Upstream `f77e0e3` ("absolute match / 2 matches = deny / no match =
/// passthrough w no creds") defines selection semantics when **multiple
/// credential routes share the same upstream host** (e.g. two GitHub
/// orgs with different tokens injected at TLS-intercept time based on
/// the inner-request path). Per D-40-B2, the fork does NOT host the
/// `tls_intercept` module that owns that selection algorithm. Instead,
/// the fork's reverse proxy is **path-prefix routed**: each request's
/// service prefix is parsed from the path (see
/// [`crate::reverse::parse_service_prefix`]) and looked up directly in
/// [`CredentialStore::get`]. The three f77e0e3 cases collapse as
/// follows in the fork's architecture:
///
/// 1. **Absolute match** — one and only one credential per service
///    prefix is structurally enforced: `credentials` is a
///    `HashMap<String, LoadedCredential>` keyed by prefix, so each
///    prefix maps to at most one credential by construction. There is
///    no path through which two `LoadedCredential` values can be
///    selected for one request.
///
/// 2. **2-match-deny** — structurally impossible in this store. The
///    upstream "ambiguous selection" case only arises when multiple
///    routes share an upstream host AND the inner-request path can
///    match more than one route's `endpoint_rules`. Both preconditions
///    require the TLS-intercept multi-route dispatch surface that the
///    fork does not have.
///
/// 3. **No match → passthrough with no credentials** — already the
///    fork's behavior: `get(&prefix)` returns `None` when no
///    credential is configured for the service prefix, and the
///    downstream reverse-proxy code path forwards the request without
///    injecting any credential header / URL transform. This is
///    semantically Option A (uniform no-creds passthrough) from the
///    Plan 40-06 D-40-B2 Windows-fallback decision: see the disposition
///    commit body for the explicit Windows-side analysis (no
///    Windows-specific credential fallback exists in the fork; Windows
///    credential injection is a transitive consequence of
///    `nono::keystore::load_secret_by_ref` using the `keyring v3`
///    crate, which is cross-platform).
#[derive(Debug)]
pub struct CredentialStore {
    /// Map from route prefix to loaded credential.
    ///
    /// Single-credential-per-prefix is a structural invariant of
    /// `HashMap`; this guarantees the f77e0e3 "absolute match" case
    /// without runtime checks.
    credentials: HashMap<String, LoadedCredential>,
    /// Map from route prefix to AWS SigV4 route (placeholder until full
    /// SigV4 signing is implemented; value is () because no runtime state
    /// is needed yet).
    aws_routes: HashMap<String, ()>,
    /// Map from route prefix to a SPIFFE-JWT-SVID-backed OAuth2 jwt-bearer
    /// assertion route (RFC 7523, NET-02/OD-1). See [`SpiffeAssertionRoute`].
    spiffe_assertion_routes: HashMap<String, SpiffeAssertionRoute>,
    /// Route prefixes that declare a direct `route.spiffe` (JWT-SVID bearer)
    /// auth source (WR-01).
    ///
    /// This is a sibling of `spiffe_assertion_routes` for the OTHER SPIFFE
    /// flow added by this phase: direct JWT-SVID injection, dispatched
    /// entirely via `RouteStore`/`LoadedRoute.managed_auth` (`route.rs`'s
    /// D-04 branch), never through this store. `CredentialStore::load`
    /// never connects to the SPIRE Workload API or exchanges anything for
    /// these routes — it only records that the route's config *declares*
    /// the auth source, mirroring the "declared vs connected" split
    /// `route.rs::LoadedRoute::declares_spiffe`'s doc comment already
    /// documents for that store. Before this field existed, a route
    /// configured with only `spiffe: Some(..)` was invisible to
    /// `loaded_prefixes()`, which made `credential_env_vars()` skip its
    /// phantom API-key env var and `route_diagnostics()` misreport
    /// `"cred: none"` for a route with a live, working managed credential.
    declared_spiffe_routes: HashSet<String>,
}

impl CredentialStore {
    /// Load credentials for all configured routes from the system keystore.
    ///
    /// Routes without a `credential_key` are skipped (no credential injection).
    /// Routes whose credential is not found (e.g. unset env var) are skipped
    /// with a warning — this allows profiles to declare optional credentials
    /// without failing when they are unavailable.
    ///
    /// Returns an error only for hard failures (config parse errors,
    /// non-UTF-8 values). Missing or inaccessible credentials are logged
    /// as warnings and the route is skipped.
    ///
    /// # Async (D-04)
    ///
    /// Async because a route with `oauth2.client_assertion` (SPIFFE jwt-bearer,
    /// NET-02/OD-1) requires an awaited `SpiffeJwtSource::connect()` to the
    /// SPIRE Workload API and an awaited initial `SpiffeAssertionTokenCache::new()`
    /// token exchange. That connect sits inside this per-route branch only — a
    /// profile with no `client_assertion` routes never touches the Workload API
    /// and gains no live-SPIRE startup dependency.
    pub async fn load(routes: &[RouteConfig]) -> Result<Self> {
        let mut credentials = HashMap::new();
        let mut aws_routes = HashMap::new();
        let mut spiffe_assertion_routes = HashMap::new();
        let mut declared_spiffe_routes = HashSet::new();
        // Lazily built: only routes with `oauth2.client_assertion` need a TLS
        // connector, so proxies with none of those never pay this cost.
        let mut assertion_tls_connector: Option<TlsConnector> = None;

        for route in routes {
            // Normalize prefix: strip leading/trailing slashes so it matches
            // the bare service name returned by parse_service_prefix() in
            // the reverse proxy path (e.g., "/anthropic" -> "anthropic").
            let normalized_prefix = route.prefix.trim_matches('/').to_string();

            // WR-01: record every route that declares a direct `route.spiffe`
            // (JWT-SVID bearer) auth source, independent of whichever branch
            // below (or none) also applies to this route. This does NOT
            // connect to the SPIRE Workload API or exchange anything — the
            // live SVID fetch happens entirely in `RouteStore`/
            // `LoadedRoute.managed_auth` (`route.rs`'s D-04 branch). Only the
            // prefix string is recorded here, never a token or SVID.
            if route.spiffe.is_some() {
                declared_spiffe_routes.insert(normalized_prefix.clone());
            }

            if let Some(ref key) = route.credential_key {
                debug!(
                    "Loading credential for route prefix: {} (mode: {:?})",
                    normalized_prefix, route.inject_mode
                );

                let secret = match nono::keystore::load_secret_by_ref(KEYRING_SERVICE, key) {
                    Ok(s) => s,
                    Err(nono::NonoError::SecretNotFound(msg)) => {
                        let redacted = redact_credential_ref(key);
                        let hint = build_credential_miss_hint(key);
                        warn!(
                            "Credential '{}' not available for route '{}': {}.{} \
                             Managed-credential requests on this route will be denied.",
                            redacted, normalized_prefix, msg, hint
                        );
                        continue;
                    }
                    Err(nono::NonoError::KeystoreAccess(msg)) => {
                        let redacted = redact_credential_ref(key);
                        warn!(
                            "Credential '{}' not available for route '{}': {}. \
                             Managed-credential requests on this route will be denied until the credential is available. \
                             Set NONO_KEYRING_TIMEOUT_SECS=N (default 120) to wait longer for keychain unlock; 0 disables the timeout.",
                            redacted, normalized_prefix, msg
                        );
                        continue;
                    }
                    Err(e) => return Err(ProxyError::Credential(e.to_string())),
                };

                let effective_format = crate::config::resolved_credential_format(
                    route.inject_header.as_str(),
                    route.credential_format.as_deref(),
                );

                let header_value = match route.inject_mode {
                    InjectMode::Header => Zeroizing::new(effective_format.replace("{}", &secret)),
                    InjectMode::BasicAuth => {
                        // Base64 encode the credential for Basic auth
                        let encoded =
                            base64::engine::general_purpose::STANDARD.encode(secret.as_bytes());
                        Zeroizing::new(format!("Basic {}", encoded))
                    }
                    // For url_path and query_param, header_value is not used
                    InjectMode::UrlPath | InjectMode::QueryParam => Zeroizing::new(String::new()),
                };

                credentials.insert(
                    normalized_prefix.clone(),
                    LoadedCredential {
                        inject_mode: route.inject_mode.clone(),
                        raw_credential: secret,
                        header_name: route.inject_header.clone(),
                        header_value,
                        path_pattern: route.path_pattern.clone(),
                        path_replacement: route.path_replacement.clone(),
                        query_param_name: route.query_param_name.clone(),
                    },
                );
                continue;
            } else if route.aws_auth.is_some() {
                // AWS SigV4 path — no credentials to load yet. Register the
                // prefix so get_aws() returns true and the proxy can return
                // 501 Not Implemented. The () value is a placeholder; the
                // real AwsRoute struct will replace it when SigV4 signing is
                // implemented.
                aws_routes.insert(normalized_prefix.clone(), ());
            } else if let Some(ref oauth2) = route.oauth2 {
                // SPIFFE OAuth2 jwt-bearer client_assertion path (RFC 7523,
                // NET-02/OD-1). Plain client_credentials (`oauth2.client_id`/
                // `client_secret`, no `client_assertion`) is intentionally
                // left untouched here — that flow's general route-wiring
                // layer (a parallel map/struct/accessor for non-SPIFFE
                // exchange) traces to unabsorbed b1ecbc02 and is declined
                // per OD-1; see SpiffeAssertionRoute's doc comment.
                if let Some(ClientAssertionConfig::SpiffeJwt {
                    workload_api_socket,
                    audience,
                    svid_hint,
                }) = &oauth2.client_assertion
                {
                    debug!(
                        "Loading SPIFFE OAuth2 jwt-bearer assertion route for prefix: {}",
                        normalized_prefix
                    );

                    // A single connect/exchange failure must skip only THIS
                    // route, not abort the whole CredentialStore::load — the
                    // fork's existing per-route-skip idiom (see the
                    // credential_key branch above), not upstream's
                    // whole-proxy-abort for this specific sub-case.
                    let jwt_source = match crate::spiffe::SpiffeJwtSource::connect(
                        workload_api_socket,
                        audience.clone(),
                        "Authorization".to_string(),
                        None,
                        svid_hint.as_deref(),
                    )
                    .await
                    {
                        Ok(source) => source,
                        Err(e) => {
                            warn!(
                                "SPIFFE Workload API unreachable for OAuth2 jwt-bearer route '{}': {}. \
                                 Requests on this route will be denied until the SPIRE agent is reachable.",
                                normalized_prefix, e
                            );
                            continue;
                        }
                    };

                    let connector = match &assertion_tls_connector {
                        Some(c) => c.clone(),
                        None => {
                            let c = build_default_tls_connector()?;
                            assertion_tls_connector = Some(c.clone());
                            c
                        }
                    };

                    let extra_params: Vec<(String, String)> = oauth2
                        .extra_params
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();

                    match crate::oauth2::SpiffeAssertionTokenCache::new(
                        oauth2.token_url.clone(),
                        Arc::new(jwt_source),
                        audience.clone(),
                        extra_params,
                        connector,
                    )
                    .await
                    {
                        Ok(cache) => {
                            spiffe_assertion_routes.insert(
                                normalized_prefix.clone(),
                                SpiffeAssertionRoute {
                                    cache,
                                    upstream: route.upstream.clone(),
                                    inject_header: route.inject_header.clone(),
                                },
                            );
                        }
                        Err(e) => {
                            warn!(
                                "SPIFFE OAuth2 jwt-bearer initial token exchange failed for route '{}': {}. \
                                 Requests on this route will be denied until the IdP is reachable.",
                                normalized_prefix, e
                            );
                            continue;
                        }
                    }
                }
            }
        }

        Ok(Self {
            credentials,
            aws_routes,
            spiffe_assertion_routes,
            declared_spiffe_routes,
        })
    }

    /// Create an empty credential store (no credential injection).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            credentials: HashMap::new(),
            aws_routes: HashMap::new(),
            spiffe_assertion_routes: HashMap::new(),
            declared_spiffe_routes: HashSet::new(),
        }
    }

    /// Get a credential for a route prefix, if configured.
    ///
    /// Returns `None` for a no-match lookup; callers MUST treat that as
    /// **passthrough with no credentials injected** (per the D-20 replay
    /// of upstream f77e0e3 "no match = passthrough w no creds"). This
    /// fail-secure default applies uniformly across Linux / macOS /
    /// Windows — the fork has no Windows-specific credential fallback
    /// path that could silently inject a credential on a no-prefix-match
    /// (Windows credential injection is a transitive property of
    /// `nono::keystore::load_secret_by_ref` invoked from
    /// [`CredentialStore::load`], not a separate "if Windows, try
    /// Credential Manager again" branch). See the `CredentialStore`
    /// struct-level doc for the full policy analysis.
    #[must_use]
    pub fn get(&self, prefix: &str) -> Option<&LoadedCredential> {
        self.credentials.get(prefix)
    }

    /// Returns `Some(())` if an AWS SigV4 route is configured for the given
    /// prefix, `None` otherwise. The `Option<&()>` return mirrors `get_oauth2`
    /// so call sites can use `.is_some()` uniformly. The value will become
    /// `Option<&AwsRoute>` when SigV4 signing is implemented.
    #[must_use]
    pub fn get_aws(&self, prefix: &str) -> Option<&()> {
        self.aws_routes.get(prefix)
    }

    /// Returns the SPIFFE OAuth2 jwt-bearer assertion route configured for
    /// the given prefix, if any (RFC 7523, NET-02/OD-1).
    ///
    /// OD-1 scope note: there is deliberately no `get_oauth2()` accessor for
    /// the general (non-SPIFFE) `client_credentials` case — see
    /// [`SpiffeAssertionRoute`]'s doc comment.
    #[must_use]
    pub fn get_spiffe_assertion(&self, prefix: &str) -> Option<&SpiffeAssertionRoute> {
        self.spiffe_assertion_routes.get(prefix)
    }

    /// Check if any credentials (static, AWS, SPIFFE OAuth2 assertion, or
    /// declared direct-SPIFFE) are loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.credentials.is_empty()
            && self.aws_routes.is_empty()
            && self.spiffe_assertion_routes.is_empty()
            && self.declared_spiffe_routes.is_empty()
    }

    /// Number of loaded credentials (static + AWS + SPIFFE OAuth2 assertion +
    /// declared direct-SPIFFE).
    #[must_use]
    pub fn len(&self) -> usize {
        self.credentials.len()
            + self.aws_routes.len()
            + self.spiffe_assertion_routes.len()
            + self.declared_spiffe_routes.len()
    }

    /// Returns the set of route prefixes that have loaded credentials
    /// (static keystore, AWS routes, SPIFFE OAuth2 assertion routes, and
    /// routes that declare a direct `route.spiffe` auth source — WR-01).
    #[must_use]
    pub fn loaded_prefixes(&self) -> std::collections::HashSet<String> {
        self.credentials
            .keys()
            .chain(self.aws_routes.keys())
            .chain(self.spiffe_assertion_routes.keys())
            .chain(self.declared_spiffe_routes.iter())
            .cloned()
            .collect()
    }

    /// Returns the set of route prefixes that declare a direct
    /// `route.spiffe` (JWT-SVID bearer) auth source (WR-01). See
    /// `declared_spiffe_routes`'s doc comment for why this is a distinct,
    /// non-connecting subset of `loaded_prefixes()`.
    #[must_use]
    pub fn declared_spiffe_prefixes(&self) -> &HashSet<String> {
        &self.declared_spiffe_routes
    }

    /// Insert a pre-built credential directly into the store for testing.
    ///
    /// Not available in production builds. Use this in tests to set up
    /// credential stores without going through the keystore loading path.
    #[cfg(test)]
    pub fn insert_for_test(&mut self, prefix: String, cred: LoadedCredential) {
        self.credentials.insert(prefix, cred);
    }
}

/// The keyring service name used by nono for all credentials.
/// Uses the same constant as `nono::keystore::DEFAULT_SERVICE` to ensure consistency.
const KEYRING_SERVICE: &str = nono::keystore::DEFAULT_SERVICE;

/// Build a default TLS connector (webpki roots + OS trust store) for the
/// RFC 7523 jwt-bearer token exchange against a route's `oauth2.token_url`.
///
/// Reuses [`crate::route::build_base_root_store`] — the same root store
/// construction `server.rs`'s shared connector and `route.rs`'s per-route
/// custom-CA connector both use — rather than inventing a second root-store
/// idiom in this file.
fn build_default_tls_connector() -> Result<TlsConnector> {
    let root_store = crate::route::build_base_root_store();
    let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| ProxyError::Config(format!("TLS config error: {}", e)))?
    .with_root_certificates(root_store)
    .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(tls_config)))
}

/// Redact a credential reference for safe display in warnings.
///
/// Delegates to the appropriate URI-specific redaction helper so that
/// secrets (account names, file paths, field names) are never echoed raw.
fn redact_credential_ref(key: &str) -> String {
    if nono::keystore::is_op_uri(key) {
        nono::keystore::redact_op_uri(key)
    } else if nono::keystore::is_apple_password_uri(key) {
        nono::keystore::redact_apple_password_uri(key)
    } else if nono::keystore::is_keyring_uri(key) {
        nono::keystore::redact_keyring_uri(key)
    } else if nono::keystore::is_bw_uri(key) {
        nono::keystore::redact_bw_uri(key)
    } else if nono::keystore::is_file_uri(key) {
        nono::keystore::redact_file_uri(key)
    } else {
        key.to_string()
    }
}

/// Build a hint for the credential-not-found warning that probes other
/// credential sources for the same name.
///
/// Targets the most common confusion pattern in the wild: a route shipped
/// with `credential_key: env://X` while the user stored their secret in
/// the system keyring (or vice versa). When we detect the secret in a
/// *different* source, we name it explicitly so the user can fix the
/// route's URI in one edit.
///
/// The probe is deliberately scoped: we only check the obvious "you put
/// it in the wrong place" cases (env↔keyring), not URI-managed sources
/// like `op://` or `apple-password://` whose lookups have side effects.
fn build_credential_miss_hint(key: &str) -> String {
    // Case 1: `env://X` failed → the env var isn't set. Check whether a
    // bare-name keyring entry exists; if so, suggest dropping the prefix.
    if let Some(var) = key.strip_prefix("env://") {
        if nono::keystore::load_secret_by_ref(KEYRING_SERVICE, var).is_ok() {
            return format!(
                " Tip: a keyring entry exists for '{}'. Change credential_key to bare \
                 '{}' (no env:// prefix) to use the keyring, or set the env var.",
                var, var
            );
        }
        return format!(
            " Looked for env var '{}' (not set). To add to the macOS keychain: \
             security add-generic-password -s \"nono\" -a \"{}\" -w  — and set credential_key \
             to bare '{}' (no env:// prefix).",
            var, var, var
        );
    }

    // Case 2: bare key (default keyring) failed → check whether the env
    // var of the same name is set; if so, suggest the env:// URI.
    if !key.contains("://") {
        if std::env::var_os(key).is_some() {
            return format!(
                " Tip: env var '{}' is set on the host. Change credential_key to \
                 'env://{}' to use it, or add a keyring entry for '{}'.",
                key, key, key
            );
        }
        if cfg!(target_os = "macos") {
            return format!(
                " To add it to the macOS keychain: security add-generic-password \
                 -s \"nono\" -a \"{}\" -w",
                key
            );
        }
    }

    // URI-managed sources (op://, apple-password://, file://, keyring://)
    // — no automatic cross-probe; the URI scheme is itself an explicit
    // statement of where to look, so we trust the user's intent.
    String::new()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
// The env-guard below mutates process env via set_var/remove_var with a symmetric
// Drop restore (the CLAUDE.md-endorsed save/restore test pattern); the
// disallowed_methods ban targets non-test misuse, so scope an allow to the tests.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_credential_store() {
        let store = CredentialStore::empty();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.get("openai").is_none());
    }

    #[test]
    fn test_loaded_credential_debug_redacts_secrets() {
        // Security: Debug output must NEVER contain real secret values.
        // This prevents accidental leakage in logs, panic messages, or
        // tracing output at debug level.
        let cred = LoadedCredential {
            inject_mode: InjectMode::Header,
            raw_credential: Zeroizing::new("sk-secret-12345".to_string()),
            header_name: "Authorization".to_string(),
            header_value: Zeroizing::new("Bearer sk-secret-12345".to_string()),
            path_pattern: None,
            path_replacement: None,
            query_param_name: None,
        };

        let debug_output = format!("{:?}", cred);

        // Must contain REDACTED markers
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output should contain [REDACTED], got: {}",
            debug_output
        );
        // Must NOT contain the actual secret
        assert!(
            !debug_output.contains("sk-secret-12345"),
            "Debug output must not contain the real secret"
        );
        assert!(
            !debug_output.contains("Bearer sk-secret"),
            "Debug output must not contain the formatted secret"
        );
        // Non-secret fields should still be visible
        assert!(debug_output.contains("Authorization"));
    }

    #[tokio::test]
    async fn test_load_no_credential_routes() {
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "/test".to_string(),
            upstream: "https://example.com".to_string(),
            credential_key: None,
            inject_mode: InjectMode::Header,
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
            capture: None,
            endpoint_policy: None,
        }];
        let store = CredentialStore::load(&routes).await;
        assert!(store.is_ok());
        let store = store.unwrap_or_else(|_| CredentialStore::empty());
        assert!(store.is_empty());
    }

    // Minimal env-var save/restore helper for tests in this module.
    // Fork: nono-proxy does not depend on nono-cli's test_env module.
    // CLAUDE.md: env vars modified in tests must be saved and restored;
    // tests are run in parallel within the same process.
    struct TestEnvGuard {
        key: &'static str,
        prior: Option<String>,
    }
    impl TestEnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prior = std::env::var(key).ok();
            // SAFETY: test-only; no threads spawned between set and restore.
            #[allow(unsafe_code)]
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, prior }
        }
    }
    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            // SAFETY: symmetric restore in Drop; matches the set_var above.
            #[allow(unsafe_code)]
            unsafe {
                match &self.prior {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_load_non_authorization_header_explicit_bearer_format() {
        // Test Case B: explicit 'Bearer {}' on custom inject header → honored exactly.
        // Fork adaptation: uses inline env guard (no nono-cli test_env dependency);
        // RouteConfig uses fork struct (no proxy/tls_client_cert/tls_client_key fields).
        let _guard = TestEnvGuard::set("NONO_PROXY_TEST_LITELLM_TOKEN", "sk-litellm-test");
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "litellm".to_string(),
            upstream: "https://litellm".to_string(),
            credential_key: Some("env://NONO_PROXY_TEST_LITELLM_TOKEN".to_string()),
            inject_mode: InjectMode::Header,
            inject_header: "x-litellm-api-key".to_string(),
            credential_format: Some("Bearer {}".to_string()),
            path_pattern: None,
            path_replacement: None,
            query_param_name: None,
            env_var: None,
            endpoint_rules: vec![],
            tls_ca: None,
            oauth2: None,
            aws_auth: None,
            capture: None,
            endpoint_policy: None,
        }];
        // Fork: CredentialStore::load takes only routes (no TLS connector arg)
        let store = CredentialStore::load(&routes)
            .await
            .expect("credential load");
        let cred = store.get("litellm").expect("route should be loaded");
        assert_eq!(cred.header_name, "x-litellm-api-key");
        assert_eq!(cred.header_value.as_str(), "Bearer sk-litellm-test");
    }

    #[tokio::test]
    async fn test_load_non_authorization_header_omitted_format_injects_bare_secret() {
        // Test Case C: credential_format omitted on non-Authorization header → bare secret.
        // Fork adaptation: uses inline env guard; RouteConfig uses fork struct.
        let _guard = TestEnvGuard::set("NONO_PROXY_TEST_API_KEY", "secret-key");
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "api".to_string(),
            upstream: "https://api.example.com".to_string(),
            credential_key: Some("env://NONO_PROXY_TEST_API_KEY".to_string()),
            inject_mode: InjectMode::Header,
            inject_header: "x-api-key".to_string(),
            credential_format: None,
            path_pattern: None,
            path_replacement: None,
            query_param_name: None,
            env_var: None,
            endpoint_rules: vec![],
            tls_ca: None,
            oauth2: None,
            aws_auth: None,
            capture: None,
            endpoint_policy: None,
        }];
        // Fork: CredentialStore::load takes only routes (no TLS connector arg)
        let store = CredentialStore::load(&routes)
            .await
            .expect("credential load");
        let cred = store.get("api").expect("route should be loaded");
        assert_eq!(cred.header_value.as_str(), "secret-key");
    }

    // ── SPIFFE OAuth2 jwt-bearer assertion routes (NET-02/OD-1) ────────────

    /// A route with `oauth2.client_assertion` pointing at an unreachable
    /// Workload API socket path is skipped (continue + warn), not a whole-
    /// startup abort — `CredentialStore::load` still returns `Ok` with any
    /// other routes loaded, and the failed route is absent from
    /// `get_spiffe_assertion()`.
    #[tokio::test]
    async fn test_load_spiffe_assertion_route_unreachable_socket_skips_route_not_startup() {
        let routes = vec![
            RouteConfig {
                spiffe: None,
                prefix: "inventory".to_string(),
                upstream: "https://inventory.internal.example".to_string(),
                credential_key: None,
                inject_mode: InjectMode::Header,
                inject_header: "Authorization".to_string(),
                credential_format: None,
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: vec![],
                tls_ca: None,
                oauth2: Some(crate::config::OAuth2Config {
                    token_url: "https://idp.internal.example/oauth/token".to_string(),
                    client_id: String::new(),
                    client_secret: String::new(),
                    scope: String::new(),
                    client_assertion: Some(ClientAssertionConfig::SpiffeJwt {
                        workload_api_socket:
                            "/tmp/nono-test-nonexistent-spire-agent-credential.sock".to_string(),
                        audience: vec!["inventory.internal.example".to_string()],
                        svid_hint: None,
                    }),
                    extra_params: std::collections::HashMap::new(),
                }),
                aws_auth: None,
                capture: None,
                endpoint_policy: None,
            },
            // A second, unrelated route must still load successfully.
            RouteConfig {
                spiffe: None,
                prefix: "api".to_string(),
                upstream: "https://api.example.com".to_string(),
                credential_key: None,
                inject_mode: InjectMode::Header,
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
                capture: None,
                endpoint_policy: None,
            },
        ];

        let store = CredentialStore::load(&routes)
            .await
            .expect("load must return Ok even when one route's SPIFFE socket is unreachable");

        assert!(
            store.get_spiffe_assertion("inventory").is_none(),
            "the unreachable-socket route must not be registered"
        );
    }

    #[test]
    fn test_get_spiffe_assertion_none_when_unconfigured() {
        let store = CredentialStore::empty();
        assert!(store.get_spiffe_assertion("anything").is_none());
    }

    // ── WR-01: direct `route.spiffe` (JWT) routes must be visible to
    //    `CredentialStore::loaded_prefixes()` ─────────────────────────────

    /// A route configured with ONLY `route.spiffe` (no `credential_key`,
    /// `aws_auth`, or `oauth2`) — the direct JWT-SVID bearer flow dispatched
    /// via `RouteStore`/`LoadedRoute.managed_auth` (`route.rs`'s D-04
    /// branch) — must still register its prefix in `loaded_prefixes()` and
    /// `declared_spiffe_prefixes()`. Before the WR-01 fix, `CredentialStore
    /// ::load`'s per-route dispatch had no branch for `route.spiffe.is_some
    /// ()`, so this prefix was invisible to `loaded_prefixes()`, which made
    /// `server.rs::credential_env_vars()` skip the phantom API-key env var
    /// and `server.rs::route_diagnostics()` misreport `"cred: none"` for a
    /// route that actually has a live, working managed credential.
    ///
    /// `CredentialStore::load` only ever inspects `route.spiffe.is_some()`
    /// for this — it never connects to the SPIRE Workload API (that connect
    /// lives entirely in `RouteStore::load`, a separate call site), so this
    /// test needs no live SPIRE agent despite the socket path below being
    /// unreachable.
    #[tokio::test]
    async fn test_load_route_spiffe_only_is_visible_in_loaded_prefixes() {
        let routes = vec![RouteConfig {
            spiffe: Some(crate::config::SpiffeAuthConfig::Jwt {
                workload_api_socket: "/tmp/nono-test-nonexistent-spire-agent-wr01.sock".to_string(),
                audience: vec!["inventory.internal.example".to_string()],
                inject_header: "Authorization".to_string(),
                credential_format: None,
                svid_hint: None,
            }),
            prefix: "inventory".to_string(),
            upstream: "https://inventory.internal.example".to_string(),
            credential_key: None,
            inject_mode: InjectMode::Header,
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
            capture: None,
            endpoint_policy: None,
        }];

        let store = CredentialStore::load(&routes).await.expect(
            "declaring route.spiffe alone must not fail CredentialStore::load — \
             no live Workload API connect happens in this store for the direct \
             JWT bearer flow",
        );

        assert!(
            store.loaded_prefixes().contains("inventory"),
            "WR-01: a route.spiffe-only route must appear in loaded_prefixes()"
        );
        assert!(
            store.declared_spiffe_prefixes().contains("inventory"),
            "the prefix must also be visible via the dedicated declared_spiffe_prefixes() accessor"
        );
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);

        // Security: only the prefix string is ever recorded for this route —
        // no static credential, AWS route, or SPIFFE OAuth2 assertion route
        // is synthesized, and no real SVID/token is stored anywhere in this
        // struct.
        assert!(
            store.get("inventory").is_none(),
            "no static LoadedCredential must be synthesized for a route.spiffe route"
        );
        assert!(store.get_aws("inventory").is_none());
        assert!(
            store.get_spiffe_assertion("inventory").is_none(),
            "route.spiffe is the direct JWT bearer flow, not the RFC 7523 \
             oauth2.client_assertion flow — the two must not be conflated"
        );
    }

    /// A route with no `spiffe`, `credential_key`, `aws_auth`, or `oauth2`
    /// field must NOT appear in `declared_spiffe_prefixes()` — the new
    /// WR-01 branch must not accidentally widen to match unrelated routes.
    #[tokio::test]
    async fn test_load_route_without_spiffe_absent_from_declared_spiffe_prefixes() {
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "plain".to_string(),
            upstream: "https://plain.example.com".to_string(),
            credential_key: None,
            inject_mode: InjectMode::Header,
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
            capture: None,
            endpoint_policy: None,
        }];

        let store = CredentialStore::load(&routes)
            .await
            .expect("credential load");

        assert!(!store.declared_spiffe_prefixes().contains("plain"));
        assert!(!store.loaded_prefixes().contains("plain"));
        assert!(store.is_empty());
    }
}
