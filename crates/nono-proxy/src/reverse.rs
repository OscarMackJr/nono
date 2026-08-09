//! Reverse proxy handler (Mode 2 — Credential Injection).
//!
//! Routes requests by path prefix to upstream APIs, injecting credentials
//! from the keystore. The agent uses `http://localhost:PORT/openai/v1/chat`
//! and the proxy rewrites to `https://api.openai.com/v1/chat` with the
//! real credential injected.
//!
//! Supports multiple injection modes:
//! - `header`: Inject into HTTP header (e.g., `Authorization: Bearer ...`)
//! - `url_path`: Replace pattern in URL path (e.g., Telegram `/bot{}/`)
//! - `query_param`: Add/replace query parameter (e.g., `?api_key=...`)
//! - `basic_auth`: HTTP Basic Authentication
//!
//! Streaming responses (SSE, MCP Streamable HTTP, A2A JSON-RPC) are
//! forwarded without buffering.

use crate::audit;
use crate::auth::UpstreamAuthMaterial;
use crate::config::EndpointPolicyOutcome;
use crate::config::InjectMode;
use crate::credential::{CredentialStore, LoadedCredential, SpiffeAssertionRoute};
use crate::error::{ProxyError, Result};
use crate::filter::ProxyFilter;
use crate::route::{LoadedRoute, RouteStore};
use crate::token;
use serde_json::Value;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::{debug, warn};
use zeroize::Zeroizing;

/// Maximum request body size (16 MiB). Prevents DoS from malicious Content-Length.
const MAX_REQUEST_BODY: usize = 16 * 1024 * 1024;

/// Timeout for upstream TCP connect.
const UPSTREAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Handle a non-CONNECT HTTP request (reverse proxy mode).
///
/// Reads the full HTTP request from the client, matches path prefix to
/// a configured route, injects credentials, and forwards to the upstream.
/// Shared context passed from the server to the reverse proxy handler.
pub struct ReverseProxyCtx<'a> {
    /// Route store for upstream URL, L7 filtering, and per-route TLS
    pub route_store: &'a RouteStore,
    /// Credential store for service lookups (optional injection)
    pub credential_store: &'a CredentialStore,
    /// Session token for authentication
    pub session_token: &'a Zeroizing<String>,
    /// Host filter for upstream validation
    pub filter: &'a ProxyFilter,
    /// Shared TLS connector (direct-TLS forwarding path)
    pub tls_connector: &'a TlsConnector,
    /// Default TLS client configuration (for `UpstreamPool` per-route client identity).
    /// Upstream cdeeb5b9 (#983): pool infrastructure absorbed; direct-TLS path retained.
    pub default_tls_config: &'a Arc<rustls::ClientConfig>,
    /// Upstream HTTP connection pool (HTTP/1.1 keep-alive + optional HTTP/2 multiplexing).
    /// Upstream cdeeb5b9 (#983): pool infrastructure absorbed; direct-TLS path retained.
    pub upstream_pool: &'a crate::pool::UpstreamPool,
    /// Shared network audit sink for session metadata capture
    pub audit_log: Option<&'a audit::SharedAuditLog>,
    /// Shared OAuth-capture phantom store (SEC-02, D-01r) — the single
    /// instance every capture-declared relay site (this plan's site 1, and
    /// Plan 114-06's sites 2/3) mints/resolves phantoms against.
    pub capture_store: &'a crate::capture::CapturePhantomStore,
    /// When `false`, the session-token / phantom-token auth checks below are
    /// skipped entirely — every request is forwarded unauthenticated.
    /// Set by the standalone `nono proxy --no-auth` command; the sandboxed
    /// `run`/`shell`/`wrap` path and the default standalone invocation leave
    /// this `true`, matching this fork's pre-existing (always-authenticated)
    /// behavior byte-for-byte.
    /// Phase 112 SEC-07 (adapted from upstream `2663e990`, #1261).
    pub require_auth: bool,
}

/// Handle a non-CONNECT HTTP request (reverse proxy mode).
///
/// `buffered_body` contains any bytes the BufReader read ahead beyond the
/// headers. These are prepended to the body read from the stream to prevent
/// data loss.
///
/// ## Phantom Token Pattern
///
/// The client (SDK) sends the session token as its "API key". The proxy:
/// 1. Extracts the service from the path (e.g., `/openai/v1/chat` → `openai`)
/// 2. Looks up which header that service uses (e.g., `Authorization` or `x-api-key`)
/// 3. Validates the phantom token from that header
/// 4. Replaces it with the real credential from keyring
pub async fn handle_reverse_proxy(
    first_line: &str,
    stream: &mut TcpStream,
    remaining_header: &[u8],
    ctx: &ReverseProxyCtx<'_>,
    buffered_body: &[u8],
) -> Result<()> {
    // Parse method, path, and HTTP version
    let (method, path, version) = parse_request_line(first_line)?;
    debug!("Reverse proxy: {} {}", method, path);

    // Extract service prefix from path (e.g., "/openai/v1/chat" -> ("openai", "/v1/chat"))
    let (service, upstream_path) = parse_service_prefix(&path)?;

    // Look up route for service (required — provides upstream URL and L7 filtering)
    let route = ctx
        .route_store
        .get(&service)
        .ok_or_else(|| ProxyError::UnknownService {
            prefix: service.clone(),
        })?;

    // L7 endpoint filtering: check method+path against rules before any
    // credential operations. Denied endpoints get 403 immediately.
    // This check runs regardless of whether a credential is configured.
    if !route.endpoint_rules.is_allowed(&method, &upstream_path) {
        let reason = format!(
            "endpoint denied: {} {} on service '{}'",
            method, upstream_path, service
        );
        warn!("{}", reason);
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::Reverse,
            nono::undo::NetworkAuditDenialCategory::EndpointPolicy,
            &audit::EventContext {
                route_id: Some(&service),
                ..Default::default()
            },
            &service,
            0,
            &reason,
        );
        send_error(stream, 403, "Forbidden").await?;
        return Ok(());
    }

    // Structured L7 endpoint policy: evaluate explicit deny/approve/allow rules.
    // evaluate() is idempotent on legacy routes (endpoint_rules only) because
    // CompiledEndpointPolicy::compile() wraps legacy rules as allow entries with
    // a deny-default when rules are non-empty, mirroring endpoint_rules semantics.
    // When an explicit endpoint_policy is configured, this enforces deny and approve
    // rules that the legacy endpoint_rules path would silently ignore.
    //
    // Approve fails closed: no approval backend is wired yet. A route that matches
    // an approve rule (or default: approve) returns 403 until an approval backend
    // is implemented. This preserves Fail-Secure — an operator-configurable control
    // must never silently degrade to a pass-through.
    //
    // IMPORTANT: match is exhaustive (no wildcard arm) so the compiler forces
    // handling of every current and future EndpointPolicyOutcome variant.
    match route.endpoint_policy.evaluate(&method, &upstream_path) {
        EndpointPolicyOutcome::Allow { .. } => {
            // Explicit allow — fall through to credential lookup below.
        }
        EndpointPolicyOutcome::Deny { reason, rule_label } => {
            let deny_reason = format!(
                "endpoint_policy denied: {} {} on service '{}' (rule: {})",
                method, upstream_path, service, rule_label,
            );
            if let Some(r) = reason {
                warn!("{} — reason: {}", deny_reason, r);
            } else {
                warn!("{}", deny_reason);
            }
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::EndpointPolicy,
                &audit::EventContext {
                    route_id: Some(&service),
                    ..Default::default()
                },
                &service,
                0,
                &deny_reason,
            );
            send_error(stream, 403, "Forbidden").await?;
            return Ok(());
        }
        EndpointPolicyOutcome::Approve { rule_label, .. } => {
            // No approval backend is implemented — fail closed rather than
            // forwarding the request with the real upstream credential.
            let deny_reason = format!(
                "endpoint_policy requires approval but no approval backend is configured \
                 — failing closed: {} {} on service '{}' (rule: {})",
                method, upstream_path, service, rule_label,
            );
            warn!("{}", deny_reason);
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::EndpointPolicy,
                &audit::EventContext {
                    route_id: Some(&service),
                    ..Default::default()
                },
                &service,
                0,
                &deny_reason,
            );
            send_error(stream, 403, "Forbidden").await?;
            return Ok(());
        }
    }

    // SPIFFE dispatch (Phase 113, Plan 113-06). Two independent SPIFFE-backed
    // auth sources exist and are mutually exclusive per route:
    //   1. `route.spiffe` — direct JWT-SVID bearer injection (D-04, live
    //      `ManagedUpstreamAuth` connected at `RouteStore::load` time).
    //   2. `route.oauth2.client_assertion` — RFC 7523 jwt-bearer exchange
    //      (Plan 113-03, `CredentialStore.spiffe_assertion_routes`).
    // Both handlers are rewrites against this fork's OWN upstream-connect
    // primitives (`parse_upstream_url`/`connect_upstream_tls`, below) — NOT
    // upstream's absent `crate::forward` module, whose spec/strategy/
    // request-forwarding types this fork does not host (D-01). Checked
    // before the static_cred/no-credential branches below so a
    // SPIFFE-declared route never falls through to keystore-based lookup.
    if route.has_spiffe_source() {
        return handle_spiffe_route(
            first_line,
            stream,
            remaining_header,
            ctx,
            buffered_body,
            &service,
            &upstream_path,
            route,
        )
        .await;
    }
    if let Some(spiffe_assertion) = ctx.credential_store.get_spiffe_assertion(&service) {
        return handle_spiffe_assertion_credential(
            first_line,
            stream,
            remaining_header,
            ctx,
            buffered_body,
            &service,
            &upstream_path,
            spiffe_assertion,
        )
        .await;
    }

    // Look up credential for service (optional — not all routes inject credentials)
    let cred = ctx.credential_store.get(&service);
    let aws_route = ctx.credential_store.get_aws(&service);

    // Authenticate the request. Every reverse proxy request must prove
    // possession of the session token, regardless of whether a credential
    // is configured — this is the localhost auth boundary.
    //
    // Phase 112 SEC-07 (adapted from upstream `2663e990`, #1261): skipped
    // entirely when `ctx.require_auth` is false (standalone
    // `nono proxy --no-auth`). The sandboxed run/shell/wrap path and the
    // default standalone invocation always pass `require_auth: true`.
    if ctx.require_auth {
        if let Some(cred) = cred {
            // Credential route: validate phantom token from the service's auth
            // header (mode-dependent: header, url_path, query_param, basic_auth).
            if let Err(e) = validate_phantom_token_for_mode(
                &cred.inject_mode,
                remaining_header,
                &upstream_path,
                &cred.header_name,
                cred.path_pattern.as_deref(),
                cred.query_param_name.as_deref(),
                ctx.session_token,
            ) {
                audit::log_denied(
                    ctx.audit_log,
                    audit::ProxyMode::Reverse,
                    nono::undo::NetworkAuditDenialCategory::AuthenticationFailed,
                    &audit::EventContext {
                        route_id: Some(&service),
                        auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
                        managed_credential_active: Some(true),
                        ..Default::default()
                    },
                    &service,
                    0,
                    &e.to_string(),
                );
                send_error(stream, 401, "Unauthorized").await?;
                return Ok(());
            }
        } else {
            // No-credential route (L7 filtering only): validate session token
            // via Proxy-Authorization header. This is the same auth path that
            // CONNECT tunnels use — the token arrives via HTTPS_PROXY userinfo.
            if let Err(e) = token::validate_proxy_auth(remaining_header, ctx.session_token) {
                audit::log_denied(
                    ctx.audit_log,
                    audit::ProxyMode::Reverse,
                    nono::undo::NetworkAuditDenialCategory::AuthenticationFailed,
                    &audit::EventContext {
                        route_id: Some(&service),
                        auth_mechanism: Some(
                            nono::undo::NetworkAuditAuthMechanism::ProxyAuthorization,
                        ),
                        auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
                        managed_credential_active: Some(false),
                        ..Default::default()
                    },
                    &service,
                    0,
                    &e.to_string(),
                );
                send_error(stream, 407, "Proxy Authentication Required").await?;
                return Ok(());
            }
        }
    }

    // AWS SigV4 signing is not yet implemented. Return 501 so the caller
    // knows the route exists but is not functional. This branch will be
    // replaced with real SigV4 signing in a follow-up. (D-15 fork adaptation:
    // upstream's 501 is in tls_intercept/handle.rs which the fork does not
    // have; this is the equivalent guard on the non-TLS proxy path.)
    if aws_route.is_some() {
        send_error(stream, 501, "Not Implemented").await?;
        return Ok(());
    }

    // Transform the path based on injection mode (url_path and query_param modes).
    // When no credential is configured, the path is forwarded unchanged.
    let transformed_path = if let Some(cred) = cred {
        transform_path_for_mode(
            &cred.inject_mode,
            &upstream_path,
            cred.path_pattern.as_deref(),
            cred.path_replacement.as_deref(),
            cred.query_param_name.as_deref(),
            &cred.raw_credential,
        )?
    } else {
        upstream_path.clone()
    };

    // Parse upstream URL with potentially transformed path.
    // Upstream URL comes from the route, not the credential.
    let upstream_url = format!(
        "{}{}",
        route.upstream.trim_end_matches('/'),
        transformed_path
    );
    debug!("Forwarding to upstream: {} {}", method, upstream_url);

    let (upstream_host, upstream_port, upstream_path_full) = parse_upstream_url(&upstream_url)?;

    // DNS resolve + host check via the filter
    let check = ctx.filter.check_host(&upstream_host, upstream_port).await?;
    if !check.result.is_allowed() {
        let reason = check.result.reason();
        warn!("Upstream host denied by filter: {}", reason);
        send_error(stream, 403, "Forbidden").await?;
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::Reverse,
            nono::undo::NetworkAuditDenialCategory::HostDenied,
            &audit::EventContext {
                route_id: Some(&service),
                ..Default::default()
            },
            &service,
            0,
            &reason,
        );
        return Ok(());
    }

    // Collect remaining request headers (excluding Host, Content-Length,
    // and Proxy-Authorization which is proxy-hop-only).
    // When a credential is present, also strip the credential's auth header
    // (it contains the phantom token, not a real credential).
    // When no credential is present, pass all other headers through —
    // the caller may have a real Authorization header for the upstream.
    let strip_header = cred.map(|c| c.header_name.as_str()).unwrap_or("");
    let filtered_headers = filter_headers(remaining_header, strip_header);
    let content_length = extract_content_length(remaining_header);

    // Read request body if present, with size limit.
    // `buffered_body` may contain bytes the BufReader read ahead beyond
    // headers; we prepend those to avoid data loss.
    let body = if let Some(len) = content_length {
        if len > MAX_REQUEST_BODY {
            send_error(stream, 413, "Payload Too Large").await?;
            return Ok(());
        }
        let mut buf = Vec::with_capacity(len);
        let pre = buffered_body.len().min(len);
        buf.extend_from_slice(&buffered_body[..pre]);
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

    // OAuth capture egress resolution (SEC-02, D-02r, T-114-19b): closes
    // the mint-to-resolve loop. A phantom previously minted by
    // `relay_response_with_capture` and handed to the sandboxed client may
    // be presented back in a later request body (e.g. a `code_verifier` or
    // similar nonce field) — this route's own name (`service`) is the
    // admission scope, mirroring `relay_response_with_capture`'s own
    // `admitted_consumers: HashSet::from([route_id.to_string()])`.
    // `resolve_capture_request_body` never errors and never denies the
    // request: an unresolved/unadmitted phantom is left unchanged in the
    // forwarded body and is simply rejected by the real upstream as an
    // invalid credential (fail-closed on SUBSTITUTION only, not on the
    // request as a whole — see `resolve_capture_request_body`'s doc
    // comment). Shadows `body` so every downstream use (the
    // `Content-Length` header below and `tls_stream.write_all(&body)`)
    // automatically sees the resolved bytes with zero further changes.
    let body =
        resolve_capture_request_body(&body, route.capture.as_ref(), ctx.capture_store, &service);

    // Connect to upstream over TLS using pre-resolved addresses.
    // Use the per-route TLS connector (with custom CA) if configured,
    // otherwise fall back to the shared default connector.
    let connector = route.tls_connector.as_ref().unwrap_or(ctx.tls_connector);
    let upstream_result = connect_upstream_tls(
        &upstream_host,
        upstream_port,
        &check.resolved_addrs,
        connector,
    )
    .await;
    let mut tls_stream = match upstream_result {
        Ok(s) => s,
        Err(e) => {
            warn!("Upstream connection failed: {}", e);
            send_error(stream, 502, "Bad Gateway").await?;
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::UpstreamConnectFailed,
                &audit::EventContext {
                    route_id: Some(&service),
                    ..Default::default()
                },
                &service,
                0,
                &e.to_string(),
            );
            return Ok(());
        }
    };

    // Build the upstream request into a Zeroizing buffer since it may contain
    // credential values. This ensures credentials are zeroed from heap memory
    // when the buffer is dropped.
    let mut request = Zeroizing::new(format!(
        "{} {} {}\r\nHost: {}\r\n",
        method, upstream_path_full, version, upstream_host
    ));

    // Inject credential based on mode (only if credential is configured)
    if let Some(cred) = cred {
        inject_credential_for_mode(cred, &mut request);
    }

    // Forward filtered headers. The credential's auth header was already
    // stripped by filter_headers() when a credential is present, so no
    // additional skipping is needed here.
    for (name, value) in &filtered_headers {
        request.push_str(&format!("{}: {}\r\n", name, value));
    }

    // CR-02 (114-REVIEW.md): a capture-declared route's response is
    // BUFFERED, not streamed, so the proxy must know where the response
    // ends. `read_capped_response`'s framing-based termination is the
    // primary mechanism; asking the upstream to close afterwards is a
    // belt-and-braces backstop that also stops a keep-alive connection
    // being pinned for the lifetime of the buffered read. Capture routes
    // only — every other route keeps the upstream's default connection
    // reuse, unchanged (D-06).
    if route.capture.is_some() {
        request.push_str("Connection: close\r\n");
    }

    // Content-Length for body
    if !body.is_empty() {
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");

    tls_stream.write_all(request.as_bytes()).await?;
    if !body.is_empty() {
        tls_stream.write_all(&body).await?;
    }
    tls_stream.flush().await?;

    // OAuth capture (SEC-02, D-01r): a route with `capture` configured is
    // buffered and rewritten by `relay_response_with_capture` instead of
    // being streamed by the unbuffered loop below. This `if let` is the
    // ONLY fork in control flow between the two paths and it
    // unconditionally `return`s — a capture-declared route can never fall
    // through into the unbuffered streaming loop (D-06: streaming remains
    // the default for every OTHER route, unmodified). See this plan's
    // SUMMARY.md for the control-flow re-read confirming this.
    if let Some(capture) = &route.capture {
        return relay_response_with_capture(
            &mut tls_stream,
            stream,
            capture,
            ctx.capture_store,
            &service,
            ctx.audit_log,
        )
        .await;
    }

    // Stream the response back to the client without buffering.
    // This handles SSE (text/event-stream), chunked transfer, and regular responses.
    let status_code = relay_response_streaming(&mut tls_stream, stream).await?;

    audit::log_reverse_proxy(
        ctx.audit_log,
        &service,
        &method,
        &upstream_path,
        status_code,
    );
    Ok(())
}

/// Handle a request to a route with a direct SPIFFE JWT-SVID auth source
/// (`route.spiffe`, D-01/D-04). Mirrors `handle_reverse_proxy`'s static_cred
/// flow step-by-step (auth gate -> acquire credential -> build/parse/filter/
/// connect upstream -> inject -> forward -> audit), substituting credential
/// acquisition with `route.managed_auth.acquire()` (a freshly fetched
/// JWT-SVID, injected as a bearer token) and audit emission with
/// `audit::log_l7_request` (SC2/D-08 — carries `spiffe_context`, unlike the
/// ctx-less `log_reverse_proxy`).
///
/// This is a REWRITE against this fork's own `parse_upstream_url()` /
/// `connect_upstream_tls()` upstream-connect primitives, not a port of
/// upstream's diffed body — the absent `crate::forward` module's spec/
/// strategy/request-forwarding types do not exist in this fork (D-01).
///
/// Fails closed (503 + `ManagedCredentialUnavailable`) if credential
/// acquisition fails, or if the `has_spiffe_source()` invariant that gated
/// dispatch to this function is somehow violated — never forwards
/// unauthenticated (T-113-17).
#[allow(clippy::too_many_arguments)]
async fn handle_spiffe_route(
    first_line: &str,
    stream: &mut TcpStream,
    remaining_header: &[u8],
    ctx: &ReverseProxyCtx<'_>,
    buffered_body: &[u8],
    service: &str,
    upstream_path: &str,
    route: &LoadedRoute,
) -> Result<()> {
    let (method, _path, version) = parse_request_line(first_line)?;

    // Auth gate — IDENTICAL call to the existing no-credential branch's
    // session-token check (validate_proxy_auth via Proxy-Authorization).
    // SPIFFE routes have no phantom-token-in-client-header concept: the real
    // credential is fetched fresh per request, never supplied by the client,
    // so there is nothing for the client to prove possession of except the
    // session token itself (T-113-15: no bypass path — this is the SAME
    // code the static_cred/no-credential branches call, not a duplicate).
    if ctx.require_auth {
        if let Err(e) = token::validate_proxy_auth(remaining_header, ctx.session_token) {
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::AuthenticationFailed,
                &audit::EventContext {
                    route_id: Some(service),
                    auth_mechanism: Some(nono::undo::NetworkAuditAuthMechanism::ProxyAuthorization),
                    auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
                    managed_credential_active: Some(false),
                    ..Default::default()
                },
                service,
                0,
                &e.to_string(),
            );
            send_error(stream, 407, "Proxy Authentication Required").await?;
            return Ok(());
        }
    }

    // Acquire a fresh JWT-SVID / bearer token. `has_spiffe_source()` (the
    // caller's dispatch condition) guarantees `managed_auth` is `Some` for
    // any route reaching this function per `RouteStore::load`'s invariant —
    // but CLAUDE.md forbids `.unwrap()`/`.expect()`, so a violated invariant
    // fails closed exactly like a genuine acquisition failure rather than
    // panicking.
    let Some(managed_auth) = route.managed_auth.as_ref() else {
        warn!(
            "handle_spiffe_route dispatched for service '{}' but managed_auth is None \
             (has_spiffe_source() invariant violated) — failing closed",
            service
        );
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::Reverse,
            nono::undo::NetworkAuditDenialCategory::ManagedCredentialUnavailable,
            &audit::EventContext {
                route_id: Some(service),
                managed_credential_active: Some(false),
                ..Default::default()
            },
            service,
            0,
            "managed_auth invariant violated",
        );
        send_error(stream, 503, "Service Unavailable").await?;
        return Ok(());
    };

    // No path transformation for SPIFFE bearer-token injection (unlike the
    // url_path/query_param static-credential modes) — the token is always
    // header-injected.
    let upstream_url = format!("{}{}", route.upstream.trim_end_matches('/'), upstream_path);
    debug!(
        "Forwarding to upstream (SPIFFE): {} {}",
        method, upstream_url
    );

    let (upstream_host, upstream_port, upstream_path_full) = parse_upstream_url(&upstream_url)?;

    // DNS resolve + host check via the filter — identical to the static_cred
    // path, and — D-17/DRAIN-06 — deliberately ordered BEFORE the credential
    // mint below (managed_auth's own acquire call). The host-check's inputs
    // (`upstream_url` from `route.upstream` + `upstream_path`, then
    // `parse_upstream_url`) do not depend on the acquired credential
    // material, so a deny_domain-blocked upstream is now rejected before any
    // live SPIRE Workload API fetch is attempted.
    //
    // This check-first shape is shared by every `HostDenied` site in this file:
    // the static-credential path, and — since the Phase 115 gap closure that
    // followed this hoist — `handle_spiffe_assertion_credential`'s RFC 7523
    // path too. (It was NOT true of the assertion path when this comment was
    // first written: the hoist landed on this function only, leaving the
    // sibling minting before its own check. Re-grep `check_host` / `HostDenied`
    // for the current sibling locations; do not trust line numbers.) Both
    // SPIFFE dispatch paths' ordering is now pinned structurally by
    // `spiffe_integration.rs::d17_spiffe_dispatch_host_check_precedes_mint_structurally`,
    // which enumerates the SPIFFE dispatch functions from this source rather
    // than naming one.
    let check = ctx.filter.check_host(&upstream_host, upstream_port).await?;
    if !check.result.is_allowed() {
        let reason = check.result.reason();
        warn!("Upstream host denied by filter: {}", reason);
        send_error(stream, 403, "Forbidden").await?;
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::Reverse,
            nono::undo::NetworkAuditDenialCategory::HostDenied,
            &audit::EventContext {
                route_id: Some(service),
                ..Default::default()
            },
            service,
            0,
            &reason,
        );
        return Ok(());
    }

    let material = match managed_auth.acquire().await {
        Ok(m) => m,
        Err(e) => {
            // Fail closed: the SPIRE agent may have gone away after startup
            // (T-113-17) — never forward the request unauthenticated.
            warn!(
                "SPIFFE credential acquisition failed for service '{}': {}",
                service, e
            );
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::ManagedCredentialUnavailable,
                &audit::EventContext {
                    route_id: Some(service),
                    auth_mechanism: Some(managed_auth.audit_mechanism()),
                    managed_credential_active: Some(false),
                    ..Default::default()
                },
                service,
                0,
                &e.to_string(),
            );
            send_error(stream, 503, "Service Unavailable").await?;
            return Ok(());
        }
    };

    let UpstreamAuthMaterial::BearerToken {
        header,
        token: svid_token,
        credential_format,
        ..
    } = &material;

    // Strip the client's own copy of the injection header (if any) — the
    // real bearer token is injected fresh below and must never be shadowed
    // or duplicated by a client-supplied value.
    let filtered_headers = filter_headers(remaining_header, header);
    let content_length = extract_content_length(remaining_header);

    // Read request body if present, with size limit — identical to the
    // static_cred path's body-read sequence.
    let body = if let Some(len) = content_length {
        if len > MAX_REQUEST_BODY {
            send_error(stream, 413, "Payload Too Large").await?;
            return Ok(());
        }
        let mut buf = Vec::with_capacity(len);
        let pre = buffered_body.len().min(len);
        buf.extend_from_slice(&buffered_body[..pre]);
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

    // Connect to upstream over TLS — reuses the per-route TLS connector
    // (custom CA) if configured, otherwise the shared default connector.
    let connector = route.tls_connector.as_ref().unwrap_or(ctx.tls_connector);
    let upstream_result = connect_upstream_tls(
        &upstream_host,
        upstream_port,
        &check.resolved_addrs,
        connector,
    )
    .await;
    let mut tls_stream = match upstream_result {
        Ok(s) => s,
        Err(e) => {
            warn!("Upstream connection failed: {}", e);
            send_error(stream, 502, "Bad Gateway").await?;
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::UpstreamConnectFailed,
                &audit::EventContext {
                    route_id: Some(service),
                    ..Default::default()
                },
                service,
                0,
                &e.to_string(),
            );
            return Ok(());
        }
    };

    // Build the upstream request into a Zeroizing buffer since it contains
    // the injected credential. This ensures the token is zeroed from heap
    // memory when the buffer is dropped.
    let mut request = Zeroizing::new(format!(
        "{} {} {}\r\nHost: {}\r\n",
        method, upstream_path_full, version, upstream_host
    ));

    // Inject the freshly-fetched bearer token using the resolved credential
    // format (mirrors inject_credential_for_mode's Header/BasicAuth arm).
    request.push_str(&format!(
        "{}: {}\r\n",
        header,
        credential_format.replace("{}", svid_token.as_str())
    ));

    for (name, value) in &filtered_headers {
        request.push_str(&format!("{}: {}\r\n", name, value));
    }

    // CR-02: see site 1's identical comment — a buffered (capture-declared)
    // response asks the upstream to close afterwards as a backstop to
    // `read_capped_response`'s framing-based termination.
    if route.capture.is_some() {
        request.push_str("Connection: close\r\n");
    }

    if !body.is_empty() {
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");

    tls_stream.write_all(request.as_bytes()).await?;
    if !body.is_empty() {
        tls_stream.write_all(&body).await?;
    }
    tls_stream.flush().await?;

    // OAuth capture (SEC-02, D-01r, T-114-18): this route's SPIFFE-bearer
    // auth source must not determine whether its RESPONSE is buffered and
    // rewritten — the same enforcement point site 1 applies. Unconditional
    // `return` on `Some` so a capture-declared route can never fall through
    // into the unbuffered streaming loop below (D-06 for every other
    // route). See `relay_capture_if_declared`'s doc comment for why this is
    // a call to a shared function rather than inline duplication of site
    // 1's `if let`.
    if let Some(result) = relay_capture_if_declared(
        &mut tls_stream,
        stream,
        route.capture.as_ref(),
        ctx.capture_store,
        service,
        ctx.audit_log,
    )
    .await
    {
        return result;
    }

    // Stream the response back to the client without buffering.
    let mut response_buf = [0u8; 8192];
    let mut status_code: u16 = 502;
    let mut first_chunk = true;

    loop {
        let n = match tls_stream.read(&mut response_buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                debug!("Upstream read error: {}", e);
                break;
            }
        };

        if first_chunk {
            status_code = parse_response_status(&response_buf[..n]);
            first_chunk = false;
        }

        stream.write_all(&response_buf[..n]).await?;
        stream.flush().await?;
    }

    // SC2/D-08: log_l7_request (NOT log_reverse_proxy — the ctx-less function
    // kept untouched per Landmine L3), carrying structured spiffe_context.
    // The raw token never appears in this record (T-113-16) — only identity
    // metadata (`spiffe_audit_context()` reads workload_spiffe_id/trust_domain,
    // never the token field).
    audit::log_l7_request(
        ctx.audit_log,
        audit::ProxyMode::Reverse,
        &audit::EventContext {
            route_id: Some(service),
            auth_mechanism: Some(managed_auth.audit_mechanism()),
            auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Succeeded),
            managed_credential_active: Some(true),
            injection_mode: managed_auth.audit_injection_mode(),
            spiffe_context: Some(material.spiffe_audit_context()),
            ..Default::default()
        },
        &audit::L7RequestInfo {
            host: &upstream_host,
            port: upstream_port,
            method: &method,
            path: upstream_path,
            status: status_code,
        },
    );

    Ok(())
}

/// Handle a request to a route authenticated via the RFC 7523 jwt-bearer
/// OAuth2 `client_assertion` flow (D-01/OD-1,
/// `credential_store.spiffe_assertion_routes`, Plan 113-03).
///
/// Shares `handle_spiffe_route`'s security-relevant control-flow ORDER —
/// session-token auth gate, then the filter host-check and its `HostDenied`
/// deny branch, and only then any credential acquisition (D-17/DRAIN-06; the
/// two paths were brought back into agreement on this after the Phase 115
/// hoist was initially applied to `handle_spiffe_route` alone, and the
/// ordering is now pinned for BOTH by
/// `spiffe_integration.rs::d17_spiffe_dispatch_host_check_precedes_mint_structurally`).
///
/// It deliberately DIFFERS from `handle_spiffe_route` elsewhere: credential
/// acquisition is `spiffe_assertion.cache.get_or_refresh()` (an exchanged
/// OAuth2 access token, not a raw JWT-SVID); the SPIFFE audit context is built
/// inline rather than via `UpstreamAuthMaterial::spiffe_audit_context()`
/// because the assertion-cache path has no
/// `ManagedUpstreamAuth`/`UpstreamAuthMaterial` in its chain at all (Plan
/// 113-03's design: it goes straight from `SpiffeAssertionTokenCache` to an
/// injected header); it takes no `route: &LoadedRoute` (dispatching off
/// `SpiffeAssertionRoute`, so `capture` is looked up via `ctx.route_store`);
/// and it supports no per-route TLS connector.
///
/// Fails closed (503 + `ManagedCredentialUnavailable`) on exchange failure —
/// never forwards unauthenticated (T-113-17).
#[allow(clippy::too_many_arguments)]
async fn handle_spiffe_assertion_credential(
    first_line: &str,
    stream: &mut TcpStream,
    remaining_header: &[u8],
    ctx: &ReverseProxyCtx<'_>,
    buffered_body: &[u8],
    service: &str,
    upstream_path: &str,
    spiffe_assertion: &SpiffeAssertionRoute,
) -> Result<()> {
    let (method, _path, version) = parse_request_line(first_line)?;

    // Auth gate — IDENTICAL call to handle_spiffe_route's / the
    // no-credential branch's session-token check.
    if ctx.require_auth {
        if let Err(e) = token::validate_proxy_auth(remaining_header, ctx.session_token) {
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::AuthenticationFailed,
                &audit::EventContext {
                    route_id: Some(service),
                    auth_mechanism: Some(nono::undo::NetworkAuditAuthMechanism::ProxyAuthorization),
                    auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
                    managed_credential_active: Some(false),
                    ..Default::default()
                },
                service,
                0,
                &e.to_string(),
            );
            send_error(stream, 407, "Proxy Authentication Required").await?;
            return Ok(());
        }
    }

    // No path transformation — the exchanged access token is always
    // header-injected (Bearer), mirroring the OAuth2 injection mode.
    let upstream_url = format!(
        "{}{}",
        spiffe_assertion.upstream.trim_end_matches('/'),
        upstream_path
    );
    debug!(
        "Forwarding to upstream (SPIFFE OAuth2 assertion): {} {}",
        method, upstream_url
    );

    let (upstream_host, upstream_port, upstream_path_full) = parse_upstream_url(&upstream_url)?;

    // DNS resolve + host check via the filter — D-17/DRAIN-06, second SPIFFE
    // dispatch path. Deliberately ordered BEFORE the token exchange below, for
    // the same reason and by the same mechanism as `handle_spiffe_route`: this
    // path's exchange (SpiffeAssertionTokenCache::get_or_refresh ->
    // exchange_jwt_assertion -> SpiffeJwtSource::fetch_token) mints a JWT-SVID
    // from the SPIRE Workload API and presents it to the IdP token endpoint, so
    // a deny_domain-blocked upstream must be rejected before it, not after.
    // The check's inputs (`spiffe_assertion.upstream` + `upstream_path`, then
    // parse_upstream_url) do not depend on the exchanged access token, so the
    // ordering is free — no deny that fired before still fires, only earlier.
    let check = ctx.filter.check_host(&upstream_host, upstream_port).await?;
    if !check.result.is_allowed() {
        let reason = check.result.reason();
        warn!("Upstream host denied by filter: {}", reason);
        send_error(stream, 403, "Forbidden").await?;
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::Reverse,
            nono::undo::NetworkAuditDenialCategory::HostDenied,
            &audit::EventContext {
                route_id: Some(service),
                ..Default::default()
            },
            service,
            0,
            &reason,
        );
        return Ok(());
    }

    // Acquire (or refresh) the exchanged OAuth2 access token. Fails closed
    // on an SVID-fetch failure (workload identity gone, T-113-10/T-113-17) —
    // `get_or_refresh()` only falls back to a stale token for a
    // network/IdP-side exchange failure, never for a lost workload identity.
    let access_token = match spiffe_assertion.cache.get_or_refresh().await {
        Ok(t) => t,
        Err(e) => {
            warn!(
                "SPIFFE OAuth2 assertion exchange failed for service '{}': {}",
                service, e
            );
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::ManagedCredentialUnavailable,
                &audit::EventContext {
                    route_id: Some(service),
                    auth_mechanism: Some(
                        nono::undo::NetworkAuditAuthMechanism::SpiffeOAuthAssertion,
                    ),
                    managed_credential_active: Some(false),
                    ..Default::default()
                },
                service,
                0,
                &e.to_string(),
            );
            send_error(stream, 503, "Service Unavailable").await?;
            return Ok(());
        }
    };

    // Strip the client's own copy of the injection header — the exchanged
    // access token is injected fresh below.
    let filtered_headers = filter_headers(remaining_header, &spiffe_assertion.inject_header);
    let content_length = extract_content_length(remaining_header);

    let body = if let Some(len) = content_length {
        if len > MAX_REQUEST_BODY {
            send_error(stream, 413, "Payload Too Large").await?;
            return Ok(());
        }
        let mut buf = Vec::with_capacity(len);
        let pre = buffered_body.len().min(len);
        buf.extend_from_slice(&buffered_body[..pre]);
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

    // SpiffeAssertionRoute carries no per-route TLS connector (no `tls_ca`
    // support for this flow yet) — always the shared default connector.
    let upstream_result = connect_upstream_tls(
        &upstream_host,
        upstream_port,
        &check.resolved_addrs,
        ctx.tls_connector,
    )
    .await;
    let mut tls_stream = match upstream_result {
        Ok(s) => s,
        Err(e) => {
            warn!("Upstream connection failed: {}", e);
            send_error(stream, 502, "Bad Gateway").await?;
            audit::log_denied(
                ctx.audit_log,
                audit::ProxyMode::Reverse,
                nono::undo::NetworkAuditDenialCategory::UpstreamConnectFailed,
                &audit::EventContext {
                    route_id: Some(service),
                    ..Default::default()
                },
                service,
                0,
                &e.to_string(),
            );
            return Ok(());
        }
    };

    // OAuth capture (SEC-02, D-01r, T-114-18): this route's RFC 7523
    // jwt-bearer assertion auth source must not determine whether its
    // RESPONSE is buffered and rewritten — the same enforcement point sites
    // 1 and 2 apply. `handle_spiffe_assertion_credential` does not take a
    // `route: &LoadedRoute` parameter (it dispatches off
    // `SpiffeAssertionRoute`, a `credential.rs` type, not a route), so the
    // route's `capture` config is looked up here via `ctx.route_store`,
    // which is already cheaply in scope and already holds an entry for
    // this service (`RouteStore::load` inserts a `LoadedRoute` — carrying
    // `capture`, independent of `oauth2.client_assertion` — for every
    // configured route, not only `spiffe`-declared ones). Resolved BEFORE
    // the request is built (CR-02) so the request can carry `Connection:
    // close` when the response is going to be buffered.
    let route_capture = ctx
        .route_store
        .get(service)
        .and_then(|r| r.capture.as_ref());

    let mut request = Zeroizing::new(format!(
        "{} {} {}\r\nHost: {}\r\n",
        method, upstream_path_full, version, upstream_host
    ));

    request.push_str(&format!(
        "{}: Bearer {}\r\n",
        spiffe_assertion.inject_header,
        access_token.as_str()
    ));

    for (name, value) in &filtered_headers {
        request.push_str(&format!("{}: {}\r\n", name, value));
    }

    // CR-02: see site 1's identical comment.
    if route_capture.is_some() {
        request.push_str("Connection: close\r\n");
    }

    if !body.is_empty() {
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");

    tls_stream.write_all(request.as_bytes()).await?;
    if !body.is_empty() {
        tls_stream.write_all(&body).await?;
    }
    tls_stream.flush().await?;

    // Unconditional `return` on `Some` so a capture-declared route can
    // never fall through into the unbuffered streaming loop below (D-06 for
    // every other route). `route_capture` is resolved above, before the
    // request build — see its comment.
    if let Some(result) = relay_capture_if_declared(
        &mut tls_stream,
        stream,
        route_capture,
        ctx.capture_store,
        service,
        ctx.audit_log,
    )
    .await
    {
        return result;
    }

    let mut response_buf = [0u8; 8192];
    let mut status_code: u16 = 502;
    let mut first_chunk = true;

    loop {
        let n = match tls_stream.read(&mut response_buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                debug!("Upstream read error: {}", e);
                break;
            }
        };

        if first_chunk {
            status_code = parse_response_status(&response_buf[..n]);
            first_chunk = false;
        }

        stream.write_all(&response_buf[..n]).await?;
        stream.flush().await?;
    }

    // The assertion-cache path has no ManagedUpstreamAuth/UpstreamAuthMaterial
    // chain (Plan 113-03) — build the SpiffeAuditContext inline rather than
    // via UpstreamAuthMaterial::spiffe_audit_context(), which is specific to
    // the BearerToken variant handle_spiffe_route's chain produces. The
    // exchanged access token never appears in this record (T-113-16); nor
    // does the raw JWT-SVID the exchange consumed.
    let workload_spiffe_id = spiffe_assertion.cache.workload_spiffe_id.clone();
    let trust_domain = crate::auth::extract_trust_domain(&workload_spiffe_id);

    audit::log_l7_request(
        ctx.audit_log,
        audit::ProxyMode::Reverse,
        &audit::EventContext {
            route_id: Some(service),
            auth_mechanism: Some(nono::undo::NetworkAuditAuthMechanism::SpiffeOAuthAssertion),
            auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Succeeded),
            managed_credential_active: Some(true),
            injection_mode: Some(nono::undo::NetworkAuditInjectionMode::OAuth2),
            spiffe_context: Some(nono::undo::SpiffeAuditContext {
                workload_spiffe_id,
                trust_domain,
                svid_type: "jwt".to_string(),
                source: "spire-workload-api".to_string(),
                upstream_spiffe_id: None,
                delegation: None,
            }),
            ..Default::default()
        },
        &audit::L7RequestInfo {
            host: &upstream_host,
            port: upstream_port,
            method: &method,
            path: upstream_path,
            status: status_code,
        },
    );

    Ok(())
}

/// Parse an HTTP request line into (method, path, version).
fn parse_request_line(line: &str) -> Result<(String, String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(ProxyError::HttpParse(format!(
            "malformed request line: {}",
            line
        )));
    }
    Ok((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}

/// Extract service prefix from path.
///
/// "/openai/v1/chat/completions" -> ("openai", "/v1/chat/completions")
/// "/anthropic/v1/messages" -> ("anthropic", "/v1/messages")
fn parse_service_prefix(path: &str) -> Result<(String, String)> {
    let trimmed = path.strip_prefix('/').unwrap_or(path);
    if let Some((prefix, rest)) = trimmed.split_once('/') {
        Ok((prefix.to_string(), format!("/{}", rest)))
    } else {
        // No sub-path, just the prefix
        Ok((trimmed.to_string(), "/".to_string()))
    }
}

/// Validate the phantom token from the service's auth header.
///
/// The SDK sends the session token as its "API key" in the standard auth header
/// for that service (e.g., `Authorization: Bearer <token>` for OpenAI,
/// `x-api-key: <token>` for Anthropic). We validate the token matches the
/// session token before swapping in the real credential.
fn validate_phantom_token(
    header_bytes: &[u8],
    header_name: &str,
    session_token: &Zeroizing<String>,
) -> Result<()> {
    let header_str = std::str::from_utf8(header_bytes).map_err(|_| ProxyError::InvalidToken)?;
    let header_name_lower = header_name.to_lowercase();

    for line in header_str.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with(&format!("{}:", header_name_lower)) {
            let value = line.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");

            // Handle "Bearer <token>" format (strip "Bearer " prefix if present)
            // Use case-insensitive check, then slice original value by length
            let value_lower = value.to_lowercase();
            let token_value = if value_lower.starts_with("bearer ") {
                // "bearer ".len() == 7
                value[7..].trim()
            } else {
                value
            };

            if token::constant_time_eq(token_value.as_bytes(), session_token.as_bytes()) {
                return Ok(());
            }
            warn!("Invalid phantom token in {} header", header_name);
            return Err(ProxyError::InvalidToken);
        }
    }

    warn!(
        "Missing {} header for phantom token validation",
        header_name
    );
    Err(ProxyError::InvalidToken)
}

/// Hop-by-hop header names (RFC 9110 §7.6.1) that apply to a single
/// transport hop and MUST NOT be forwarded across a proxy.
///
/// CR-02 (114-REVIEW.md): `Connection` in particular was being forwarded
/// verbatim, so a normal client's `Connection: keep-alive` actively asked
/// the UPSTREAM to hold the connection open — on the capture path, whose
/// buffered read then had nothing to terminate on. Each of these describes
/// the client<->proxy hop, never the proxy<->upstream hop.
const HOP_BY_HOP_HEADER_PREFIXES: &[&str] = &[
    "connection:",
    "keep-alive:",
    "proxy-connection:",
    "te:",
    "trailer:",
    "upgrade:",
];

/// Filter headers, removing hop-by-hop and proxy-internal headers.
///
/// Always strips:
/// - `Host` (rewritten to upstream)
/// - `Content-Length` (re-added after body is read)
/// - `Proxy-Authorization` (hop-by-hop, contains session token)
/// - every [`HOP_BY_HOP_HEADER_PREFIXES`] header (CR-02)
///
/// When `cred_header` is non-empty, also strips that header (it contains
/// the phantom token that must not be forwarded alongside the real credential).
/// When `cred_header` is empty (no-credential route), all other headers
/// including `Authorization` are passed through to the upstream.
fn filter_headers(header_bytes: &[u8], cred_header: &str) -> Vec<(String, String)> {
    let header_str = std::str::from_utf8(header_bytes).unwrap_or("");
    let cred_header_lower = if cred_header.is_empty() {
        String::new()
    } else {
        format!("{}:", cred_header.to_lowercase())
    };
    let mut headers = Vec::new();

    for line in header_str.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("host:")
            || lower.starts_with("content-length:")
            || lower.starts_with("proxy-authorization:")
            || HOP_BY_HOP_HEADER_PREFIXES
                .iter()
                .any(|prefix| lower.starts_with(prefix))
            || (!cred_header_lower.is_empty() && lower.starts_with(&cred_header_lower))
            || line.trim().is_empty()
        {
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    headers
}

/// Extract Content-Length value from raw headers.
///
/// `pub(crate)`: reused by the forward-proxy path (`server::handle_forward_http`,
/// #1335) so both paths parse Content-Length identically instead of
/// duplicating this logic.
pub(crate) fn extract_content_length(header_bytes: &[u8]) -> Option<usize> {
    let header_str = std::str::from_utf8(header_bytes).ok()?;
    for line in header_str.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            let value = line.split_once(':')?.1.trim();
            return value.parse().ok();
        }
    }
    None
}

/// Parse an upstream URL into (host, port, path).
fn parse_upstream_url(url_str: &str) -> Result<(String, u16, String)> {
    let parsed = url::Url::parse(url_str)
        .map_err(|e| ProxyError::HttpParse(format!("invalid upstream URL '{}': {}", url_str, e)))?;

    let scheme = parsed.scheme();
    if scheme != "https" && scheme != "http" {
        return Err(ProxyError::HttpParse(format!(
            "unsupported URL scheme: {}",
            url_str
        )));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| ProxyError::HttpParse(format!("missing host in URL: {}", url_str)))?
        .to_string();

    let default_port = if scheme == "https" { 443 } else { 80 };
    let port = parsed.port().unwrap_or(default_port);

    let path = parsed.path().to_string();
    let path = if path.is_empty() {
        "/".to_string()
    } else {
        path
    };

    // Include query string if present
    let path_with_query = if let Some(query) = parsed.query() {
        format!("{}?{}", path, query)
    } else {
        path
    };

    Ok((host, port, path_with_query))
}

/// Connect to an upstream host over TLS using pre-resolved addresses.
///
/// Uses the pre-resolved `SocketAddr`s from the filter check to prevent
/// DNS rebinding TOCTOU. Falls back to hostname resolution only if no
/// pre-resolved addresses are available.
///
/// The `TlsConnector` is shared across all connections (created once at
/// server startup with the system root certificate store).
async fn connect_upstream_tls(
    host: &str,
    port: u16,
    resolved_addrs: &[SocketAddr],
    connector: &TlsConnector,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let tcp = if resolved_addrs.is_empty() {
        // Fallback: no pre-resolved addresses (shouldn't happen in practice)
        let addr = format!("{}:{}", host, port);
        match tokio::time::timeout(UPSTREAM_CONNECT_TIMEOUT, TcpStream::connect(&addr)).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                return Err(ProxyError::UpstreamConnect {
                    host: host.to_string(),
                    reason: e.to_string(),
                });
            }
            Err(_) => {
                return Err(ProxyError::UpstreamConnect {
                    host: host.to_string(),
                    reason: "connection timed out".to_string(),
                });
            }
        }
    } else {
        connect_to_resolved(resolved_addrs, host).await?
    };

    let server_name = rustls::pki_types::ServerName::try_from(host.to_string()).map_err(|_| {
        ProxyError::UpstreamConnect {
            host: host.to_string(),
            reason: "invalid server name for TLS".to_string(),
        }
    })?;

    let tls_stream =
        connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| ProxyError::UpstreamConnect {
                host: host.to_string(),
                reason: format!("TLS handshake failed: {}", e),
            })?;

    Ok(tls_stream)
}

/// Connect to one of the pre-resolved socket addresses with timeout.
///
/// `pub(crate)`: reused by the forward-proxy path (`server::handle_forward_http`,
/// #1335) as its plain-TCP (no-TLS) upstream connect — the DNS-rebinding-safe
/// connect-to-resolved-addresses mechanism is identical for both paths.
pub(crate) async fn connect_to_resolved(addrs: &[SocketAddr], host: &str) -> Result<TcpStream> {
    let mut last_err = None;
    for addr in addrs {
        match tokio::time::timeout(UPSTREAM_CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(e)) => {
                debug!("Connect to {} failed: {}", addr, e);
                last_err = Some(e.to_string());
            }
            Err(_) => {
                debug!("Connect to {} timed out", addr);
                last_err = Some("connection timed out".to_string());
            }
        }
    }
    Err(ProxyError::UpstreamConnect {
        host: host.to_string(),
        reason: last_err.unwrap_or_else(|| "no addresses to connect to".to_string()),
    })
}

/// Parse HTTP status code from the first response chunk.
///
/// Looks for the "HTTP/x.y NNN" pattern in the first line. Returns 502
/// if the response doesn't contain a valid status line (upstream sent
/// garbage or incomplete data).
/// `pub(crate)`: reused by the forward-proxy path (`server::handle_forward_http`,
/// #1335) to parse the upstream status code for its audit event, identically
/// to how the reverse-proxy path parses it here.
pub(crate) fn parse_response_status(data: &[u8]) -> u16 {
    // Find the end of the first line (or use full data if no newline)
    let line_end = data
        .iter()
        .position(|&b| b == b'\r' || b == b'\n')
        .unwrap_or(data.len());
    let first_line = &data[..line_end.min(64)];

    if let Ok(line) = std::str::from_utf8(first_line) {
        // Split on whitespace: ["HTTP/1.1", "200", "OK"]
        let mut parts = line.split_whitespace();
        if let Some(version) = parts.next() {
            if version.starts_with("HTTP/") {
                if let Some(code_str) = parts.next() {
                    if code_str.len() == 3 {
                        return code_str.parse().unwrap_or(502);
                    }
                }
            }
        }
    }
    502
}

/// Send an HTTP error response.
async fn send_error(stream: &mut TcpStream, status: u16, reason: &str) -> Result<()> {
    let body = format!("{{\"error\":\"{}\"}}", reason);
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        status,
        reason,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Relay an upstream response to the client byte-for-byte, unbuffered —
/// the pre-existing behavior for every route WITHOUT `capture` configured
/// (SSE, MCP Streamable HTTP, A2A JSON-RPC depend on this, D-06).
/// Extracted verbatim from site 1's inline loop (zero behavior change) so
/// it is independently testable: now that `relay_response_with_capture`
/// exists as a second dispatch path, proving a `capture: None` route
/// still forwards bytes as-is (no buffering, no rewrite processing) is a
/// meaningful regression test for D-06's "streaming stays the default"
/// property. Sites 2/3 (`handle_spiffe_route`,
/// `handle_spiffe_assertion_credential`) keep their own separate inline
/// copies of this same loop shape — out of this plan's scope (Plan
/// 114-06 wires capture into those) — so this extraction deliberately
/// does not touch them.
///
/// Returns the parsed HTTP status code (502 if the response never
/// produced a valid status line — identical to the pre-extraction
/// behavior).
async fn relay_response_streaming<R: AsyncRead + Unpin>(
    tls_stream: &mut R,
    stream: &mut TcpStream,
) -> Result<u16> {
    let mut response_buf = [0u8; 8192];
    let mut status_code: u16 = 502;
    let mut first_chunk = true;

    loop {
        let n = match tls_stream.read(&mut response_buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                debug!("Upstream read error: {}", e);
                break;
            }
        };

        // Parse status from first chunk. The HTTP status line format is:
        // "HTTP/1.1 200 OK\r\n..." — we need the 3-digit code after the
        // first space. We scan up to 32 bytes (enough for any valid status line).
        if first_chunk {
            status_code = parse_response_status(&response_buf[..n]);
            first_chunk = false;
        }

        stream.write_all(&response_buf[..n]).await?;
        stream.flush().await?;
    }

    Ok(status_code)
}

// ============================================================================
// OAuth capture: response buffer-and-rewrite (SEC-02, D-01r)
// ============================================================================
//
// This is the fork-native enforcement point D-01r builds to replace the
// retracted decline (114-CONTEXT.md): the reverse proxy already has
// plaintext visibility into upstream responses (its own `tls_stream`), it
// previously lacked only BUFFERING (responses stream in 8 KiB chunks by
// deliberate design, D-06, for SSE / MCP Streamable HTTP / A2A JSON-RPC).
// Every function below is used ONLY on capture-declared routes — the
// unbuffered streaming loops elsewhere in this file are completely
// unmodified and remain the default.
//
// Fail-closed contract (non-negotiable, restated at each call site below):
// buffer cap exceeded -> deny; Content-Encoding present (any status code,
// Pitfall 2 / `3c59c62e`) -> deny; malformed/unparseable body -> deny; an
// unconfigured token-shaped field surviving rewrite -> deny. No branch
// releases a partially-processed or unrewritten body.

/// Per-read timeout while buffering an upstream response for OAuth-capture
/// rewrite (D-05). Bounds the time dimension of the DoS surface buffering
/// introduces (`read_capped_response`'s `cap` parameter bounds the memory
/// dimension). Mirrors upstream's `UPSTREAM_REWRITE_READ_TIMEOUT` (design
/// reference, D-07 — the constant is ported, not the surrounding
/// `ResponseRewrite` abstraction, which this fork does not have).
pub const CAPTURE_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Locate the `\r\n\r\n` header/body boundary in a buffered HTTP response.
/// Returns the byte offset where the FOUR-byte `\r\n\r\n` marker itself
/// begins (the body starts at `offset + 4`), or `None` if no complete
/// header block is present yet.
///
/// `pub`, not private: with Task 3's `relay_response_with_capture` landing
/// in this same plan's next commit, this is transiently true dead code
/// (only `#[cfg(test)]` call sites exist between this commit and the
/// next). `cargo clippy --lib` (no `--tests`) does not compile the `test`
/// cfg, so a private/`pub(crate)` item with zero non-test callers is
/// flagged `dead_code` under `-D warnings` — verified empirically:
/// `pub(crate)` alone does NOT suppress it, only full `pub` does (rustc
/// exempts externally-reachable API surface from this lint). Mirrors Plan
/// 114-03's `capture_declared_for_upstream` and Plan 114-04's
/// `rewrite_response_fields`/etc., the same not-yet-wired-in situation.
pub fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|window| window == b"\r\n\r\n")
}

/// Decode an HTTP/1.1 chunked-transfer-encoded body into its reassembled
/// bytes. Never panics on malformed input (non-hex chunk-size, missing
/// terminating CRLF, truncated chunk data, chunk-size overflow) — always
/// returns `Err` instead, per this enforcement point's fail-closed
/// contract. Ported from the upstream design reference cited in this
/// plan's `<interfaces>` section (D-07), including its `checked_add` use
/// for the chunk-size-overflow guard (CLAUDE.md's checked-arithmetic
/// mandate for security-critical math).
///
/// SECRET-HYGIENE INVARIANT (CR-01, 114-REVIEW.md): every `Err` this
/// function returns is built from FIXED text plus byte offsets/lengths
/// only — never from the response bytes themselves. An upstream that
/// declares `Transfer-Encoding: chunked` and then sends an identity JSON
/// body makes the "chunk-size line" literally the first line of that body
/// (i.e. `{"access_token":"<REAL TOKEN>",...`); interpolating it here put
/// a real OAuth token into `warn!` and into the on-disk audit ledger via
/// `relay_response_with_capture`'s deny path. Offsets and lengths are safe
/// (they carry no response content); the bytes are not, and are NEVER
/// truncated-and-included either — a truncated token prefix is still a
/// secret leak.
pub fn decode_chunked_body(body: &[u8]) -> Result<Vec<u8>> {
    let mut pos = 0usize;
    let mut out = Vec::new();
    loop {
        let Some(line_end_rel) = body
            .get(pos..)
            .and_then(|rest| rest.windows(2).position(|w| w == b"\r\n"))
        else {
            return Err(ProxyError::HttpParse(
                "malformed chunked response: missing chunk-size line terminator".to_string(),
            ));
        };
        let line_end = pos + line_end_rel;
        let line_len = line_end.saturating_sub(pos);
        let size_hex = std::str::from_utf8(&body[pos..line_end])
            .map_err(|_| {
                ProxyError::HttpParse(format!(
                    "malformed chunked response: chunk-size line at byte offset {pos} \
                     ({line_len} bytes) is not valid UTF-8; value redacted"
                ))
            })?
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let size = usize::from_str_radix(size_hex, 16).map_err(|_| {
            ProxyError::HttpParse(format!(
                "malformed chunked response: chunk-size line at byte offset {pos} \
                 ({line_len} bytes) is not a valid hex chunk size; value redacted"
            ))
        })?;
        pos = line_end + 2;
        if size == 0 {
            break;
        }
        let end = pos.checked_add(size).ok_or_else(|| {
            ProxyError::HttpParse("malformed chunked response: chunk size overflow".to_string())
        })?;
        let end_plus_crlf = end.checked_add(2).ok_or_else(|| {
            ProxyError::HttpParse("malformed chunked response: chunk size overflow".to_string())
        })?;
        if end_plus_crlf > body.len() || &body[end..end_plus_crlf] != b"\r\n" {
            return Err(ProxyError::HttpParse(
                "malformed chunked response: missing terminating CRLF after chunk data".to_string(),
            ));
        }
        out.extend_from_slice(&body[pos..end]);
        pos = end_plus_crlf;
    }
    Ok(out)
}

/// Header facts relevant to OAuth-capture buffering, parsed from the raw
/// header block of a buffered upstream response.
pub struct CaptureResponseHeaders {
    /// `Transfer-Encoding: chunked` was present.
    pub chunked: bool,
    /// A non-empty `Content-Encoding` header was present. Detection is
    /// deliberately UNCONDITIONAL of status code — this struct has no
    /// status field to gate on — closing the exact narrower-gate defect
    /// class upstream's own `3c59c62e` fixed (Pitfall 2, 114-CONTEXT.md).
    pub content_encoded: bool,
}

/// Scan a raw HTTP header block (the bytes before the `\r\n\r\n` boundary,
/// NOT including the status line's own significance) for
/// `Transfer-Encoding: chunked` and a non-empty `Content-Encoding` header,
/// matched case-insensitively per HTTP header-name semantics. Pure,
/// side-effect-free, and independently unit-tested so the
/// content-encoding-is-unconditional property is provable without needing
/// the full `relay_response_with_capture` dispatch (this function never
/// looks at a status code at all).
pub fn parse_capture_response_headers(header_block: &[u8]) -> CaptureResponseHeaders {
    let header_str = std::str::from_utf8(header_block).unwrap_or("");
    let mut chunked = false;
    let mut content_encoded = false;
    for line in header_str.lines() {
        let lower = line.to_lowercase();
        if let Some(value) = lower.strip_prefix("transfer-encoding:") {
            if value.trim().contains("chunked") {
                chunked = true;
            }
        } else if let Some(value) = lower.strip_prefix("content-encoding:") {
            if !value.trim().is_empty() {
                content_encoded = true;
            }
        }
    }
    CaptureResponseHeaders {
        chunked,
        content_encoded,
    }
}

/// Scan a chunked-transfer-encoded body prefix and report whether the
/// whole body is present, so `read_capped_response` can stop reading at
/// the terminal (zero-size) chunk instead of waiting for EOF (CR-02,
/// 114-REVIEW.md).
///
/// Returns `true` when the body is COMPLETE **or MALFORMED**, `false` only
/// when more bytes are genuinely needed. Treating malformed framing as
/// "stop reading" is deliberate and fail-closed: `decode_chunked_body`
/// will then produce the deny, whereas continuing to read would block on a
/// keep-alive connection that is never going to send the terminator.
///
/// Chunk-framing semantics mirror `decode_chunked_body` exactly (including
/// its `checked_add` overflow guards and its "a zero-size chunk header
/// ends the body" break condition) so the two can never disagree about
/// where the body ends. Never interpolates response bytes anywhere — it
/// returns a bool.
fn chunked_body_complete(body: &[u8]) -> bool {
    let mut pos = 0usize;
    loop {
        let Some(line_end_rel) = body
            .get(pos..)
            .and_then(|rest| rest.windows(2).position(|w| w == b"\r\n"))
        else {
            // No complete chunk-size line yet — need more bytes.
            return false;
        };
        let line_end = pos + line_end_rel;
        let Ok(line) = std::str::from_utf8(&body[pos..line_end]) else {
            return true; // malformed -> stop; step 4 denies
        };
        let size_hex = line.split(';').next().unwrap_or("").trim();
        let Ok(size) = usize::from_str_radix(size_hex, 16) else {
            return true; // malformed -> stop; step 4 denies
        };
        pos = line_end + 2;
        if size == 0 {
            return true; // terminal chunk reached — body is complete
        }
        let (Some(end), Some(end_plus_crlf)) = (
            pos.checked_add(size),
            pos.checked_add(size).and_then(|e| e.checked_add(2)),
        ) else {
            return true; // overflow -> malformed -> stop; step 4 denies
        };
        if end_plus_crlf > body.len() {
            return false; // chunk data still arriving
        }
        if body.get(end..end_plus_crlf) != Some(b"\r\n".as_slice()) {
            return true; // malformed -> stop; step 4 denies
        }
        pos = end_plus_crlf;
    }
}

/// Decide whether a buffered response prefix is a COMPLETE HTTP message
/// according to its own framing, so the capture path can stop reading
/// without waiting for the upstream to close the connection (CR-02,
/// 114-REVIEW.md).
///
/// Precedence follows RFC 9112 §6.3: `Transfer-Encoding: chunked` wins
/// over `Content-Length` when both are present. When NEITHER framing is
/// present the response is connection-close-delimited by definition, so
/// this returns `false` and the caller falls back to reading until EOF.
///
/// Returns a bool — no response bytes are ever surfaced (CR-01).
fn response_framing_complete(raw: &[u8]) -> bool {
    let Some(marker_pos) = find_header_end(raw) else {
        return false; // header block still arriving
    };
    let header_block = &raw[..marker_pos];
    let body = &raw[marker_pos + 4..];

    if parse_capture_response_headers(header_block).chunked {
        return chunked_body_complete(body);
    }
    if let Some(len) = extract_content_length(header_block) {
        return body.len() >= len;
    }
    false
}

/// Read an upstream response into memory, enforcing `cap` as a hard
/// ceiling on bytes held (D-05) and `read_timeout` as a hard ceiling on
/// time spent per read (bounds the DoS surface buffering introduces).
///
/// TERMINATION (CR-02, 114-REVIEW.md): stops on the response's OWN HTTP
/// framing — `Transfer-Encoding: chunked`'s terminal zero-size chunk, or
/// `Content-Length` bytes of body — and falls back to reading until EOF
/// ONLY when the response carries neither (i.e. is genuinely
/// connection-close-delimited).
///
/// This previously terminated on EOF alone. Every real OAuth token
/// endpoint (Okta, Auth0, Google, Entra, GitHub) is HTTP/1.1 with
/// persistent connections and does not close after a response, so every
/// capture request against a real provider blocked for the full
/// `CAPTURE_READ_TIMEOUT` and then returned 502 — the feature did not work
/// against any spec-compliant provider, and each attempt pinned a task, a
/// TLS connection and up to `cap` bytes for 30 s (an amplification lever a
/// sandboxed agent could drive in a loop). Every relay test masked this by
/// explicitly closing the upstream write half; see
/// `capture_relay_terminates_without_upstream_eof` for the regression
/// test that deliberately does not.
///
/// Still fail-closed in every direction: `cap` and `read_timeout` are
/// unchanged, and malformed chunk framing stops the read so the caller's
/// decode step denies rather than the read blocking forever.
///
/// Does NOT parse headers or write anything to the client — it only
/// bounds the read. The caller is responsible for converting an `Err`
/// into an actual denial response written to the client stream; this
/// function never writes partial data anywhere.
///
/// Generic over `AsyncRead + Unpin` rather than a concrete
/// `tokio_rustls::client::TlsStream<TcpStream>`: no TLS-server test
/// fixture exists anywhere in this crate, and building one solely for
/// this plan's 4 required behavior tests would be new, heavy machinery
/// unrelated to the security property under test. Genericity lets those
/// tests drive an in-memory `tokio::io::duplex()` pair instead; the real
/// call site's `TlsStream<TcpStream>` satisfies `AsyncRead + Unpin`
/// unchanged, so this is a pure testability improvement with zero
/// behavior difference at the production call site. `read_timeout` is
/// likewise a parameter (not `CAPTURE_READ_TIMEOUT` used inline) purely so
/// the timeout branch itself is unit-testable without a real 30-second
/// wait; the production call site always passes `CAPTURE_READ_TIMEOUT`.
pub async fn read_capped_response<R: AsyncRead + Unpin>(
    reader: &mut R,
    cap: usize,
    read_timeout: Duration,
) -> Result<Zeroizing<Vec<u8>>> {
    let mut raw: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    let mut buf = [0u8; 8192];
    loop {
        // Framing-based termination is checked BEFORE each read, so a
        // response delivered in a single read never issues a second,
        // blocking read (CR-02).
        if response_framing_complete(&raw) {
            break;
        }
        let n = match tokio::time::timeout(read_timeout, reader.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                debug!("Upstream read error during OAuth capture buffering: {}", e);
                return Err(ProxyError::HttpParse(format!(
                    "upstream read error during OAuth capture buffering: {e}"
                )));
            }
            Err(_) => {
                return Err(ProxyError::HttpParse(
                    "timed out reading response for OAuth capture rewrite".to_string(),
                ));
            }
        };
        raw.extend_from_slice(&buf[..n]);
        if raw.len() > cap {
            return Err(ProxyError::HttpParse(format!(
                "response exceeded {cap}-byte OAuth capture buffer cap; denying rather than \
                 releasing an unrewritten body"
            )));
        }
    }
    Ok(raw)
}

/// Read a dot-separated JSON path's string value out of an ALREADY
/// REWRITTEN body, for audit purposes only — recording which phantom
/// value was minted at a rewritten path. Never called on unrewritten
/// data (the only caller is `relay_response_with_capture`, after
/// `rewrite_response_fields` has already replaced the real token with a
/// phantom at every path in `rewritten_paths`), so this can never surface
/// a real secret to the audit log. Mirrors `capture.rs`'s private
/// `value_at_path_mut` traversal shape but read-only and duplicated here
/// rather than exposed from `capture.rs`, since this is audit-only
/// plumbing, not security logic.
fn read_capture_body_path<'a>(body: &'a Value, path: &str) -> Option<&'a str> {
    let mut current = body;
    for part in path.split('.') {
        if part.is_empty() {
            return None;
        }
        current = current.as_object()?.get(part)?;
    }
    current.as_str()
}

/// The fork-native OAuth-capture response buffer-and-rewrite enforcement
/// point (SEC-02, D-01r/D-02r). Called ONLY for a route with `capture`
/// configured, in place of the unbuffered streaming loop the rest of this
/// file uses. Buffers the ENTIRE upstream response (bounded by
/// `read_capped_response`'s cap + timeout), and only ever releases it to
/// `stream` after every configured token field has been rewritten to a
/// sandbox-visible phantom and the fail-closed backstop
/// (`reject_unrewritten_token_fields`) has confirmed no unconfigured
/// token-shaped field survived.
///
/// FAIL-CLOSED CONTRACT (this is the property ADR-114 cites as proof SC2
/// is satisfied by construction): every branch below that cannot
/// guarantee a safe, fully-rewritten body denies the response with a 502
/// written to `stream` and returns `Ok(())` — it NEVER releases a
/// partially-processed, unrewritten, or unparseable body, and it never
/// panics. There is no path from this function back into the unbuffered
/// streaming loop in `handle_reverse_proxy` (see the call site's control-
/// flow comment and this plan's SUMMARY.md for the explicit re-read that
/// confirms this).
///
/// Generic over `AsyncRead + Unpin` for the same testability reason
/// `read_capped_response` is (see its doc comment) — the real call site's
/// `tokio_rustls::client::TlsStream<TcpStream>` satisfies the bound
/// unchanged.
pub async fn relay_response_with_capture<R: AsyncRead + Unpin>(
    tls_stream: &mut R,
    stream: &mut TcpStream,
    capture: &crate::config::CaptureConfig,
    capture_store: &crate::capture::CapturePhantomStore,
    route_id: &str,
    audit_log: Option<&audit::SharedAuditLog>,
) -> Result<()> {
    let cap = capture
        .max_response_bytes
        .unwrap_or(crate::config::DEFAULT_CAPTURE_MAX_RESPONSE_BYTES);

    // SECRET-HYGIENE INVARIANT (CR-01, 114-REVIEW.md): `reason` is
    // `&'static str`, NOT `String`. This is a STRUCTURAL guarantee, not a
    // convention — it is impossible to interpolate an upstream-derived
    // value (a `format!` result, an `e.to_string()`, a body fragment) into
    // a deny reason, because such a value cannot coerce to `&'static str`.
    // The reason reaches BOTH `warn!` (the operator's log file) AND
    // `audit::log_denied` -> `NetworkAuditEvent.reason` -> the session
    // metadata PERSISTED ON DISK. Before this was tightened, an upstream
    // that declared `Transfer-Encoding: chunked` and sent an identity JSON
    // body wrote a real OAuth token into both (CR-01), falsifying D-08
    // ("never written to disk"), `capture.rs`'s module doc, and ADR-114.
    // Detailed, PROVABLY CONTENT-FREE diagnostics go to `debug!` at each
    // call site below; response-derived bytes go nowhere at all.
    //
    // Do NOT relax this to `String` or `impl Into<String>`.
    let deny = move |reason: &'static str| async move {
        warn!(
            "OAuth capture: denying response for route '{}': {}",
            route_id, reason
        );
        audit::log_denied(
            audit_log,
            audit::ProxyMode::Reverse,
            nono::undo::NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed,
            &audit::EventContext {
                route_id: Some(route_id),
                ..Default::default()
            },
            route_id,
            0,
            reason,
        );
    };

    // Step 1: bounded read (D-05). Cap-exceeded, timeout, and upstream
    // read errors are ALL denied here — never a partial or unrewritten
    // body released.
    let raw = match read_capped_response(tls_stream, cap, CAPTURE_READ_TIMEOUT).await {
        Ok(raw) => raw,
        Err(e) => {
            // Safe to log: every error `read_capped_response` produces is
            // built from fixed text plus an `io::Error`/cap value — never
            // from response bytes.
            debug!("OAuth capture: buffered read failed for route '{route_id}': {e}");
            deny("upstream response could not be buffered within the OAuth capture cap or timeout")
                .await;
            send_error(stream, 502, "Bad Gateway").await?;
            return Ok(());
        }
    };

    // Step 2: header/body split. No complete header block -> deny.
    let Some(marker_pos) = find_header_end(&raw) else {
        deny("no complete header block in buffered response").await;
        send_error(stream, 502, "Bad Gateway").await?;
        return Ok(());
    };
    let body_start = marker_pos + 4;
    let header_block = &raw[..marker_pos];
    let raw_body = &raw[body_start..];

    let status_code = parse_response_status(&raw);
    let status_line_end = header_block
        .iter()
        .position(|&b| b == b'\r')
        .unwrap_or(header_block.len());
    let status_line =
        std::str::from_utf8(&header_block[..status_line_end]).unwrap_or("HTTP/1.1 502 Bad Gateway");

    // Step 3: Content-Encoding is denied UNCONDITIONALLY of status code
    // (Pitfall 2 / `3c59c62e`) — a compressed body cannot be
    // field-rewritten reliably, so it must never be forwarded from a
    // capture-declared route, 2xx or not.
    let header_facts = parse_capture_response_headers(header_block);
    if header_facts.content_encoded {
        // `status_code` is a parsed `u16`, not response bytes — safe to log.
        debug!(
            "OAuth capture: Content-Encoding present on route '{route_id}' (status {status_code})"
        );
        deny(
            "Content-Encoding present on capture-declared route; a compressed body cannot be \
             field-rewritten reliably",
        )
        .await;
        send_error(stream, 502, "Bad Gateway").await?;
        return Ok(());
    }

    // Step 4: chunked decode, if present.
    let body_bytes: Vec<u8> = if header_facts.chunked {
        match decode_chunked_body(raw_body) {
            Ok(decoded) => decoded,
            Err(e) => {
                // Safe to log: `decode_chunked_body` guarantees every `Err`
                // it returns carries offsets/lengths only, never response
                // bytes (see its SECRET-HYGIENE INVARIANT).
                debug!("OAuth capture: chunked decode failed for route '{route_id}': {e}");
                deny("chunked response body could not be decoded").await;
                send_error(stream, 502, "Bad Gateway").await?;
                return Ok(());
            }
        }
    } else {
        raw_body.to_vec()
    };

    // Step 5: parse as JSON. An unparseable body on a capture-declared
    // route cannot be safety-inspected, so it must not be forwarded.
    //
    // CR-01: the `serde_json::Error` is deliberately DISCARDED, not logged
    // even at `debug!` — serde_json error messages can echo fragments of
    // the offending input, and the offending input here is a response body
    // that may hold a real OAuth token.
    let mut body: Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(_) => {
            deny("response body on a capture-declared route is not parseable JSON").await;
            send_error(stream, 502, "Bad Gateway").await?;
            return Ok(());
        }
    };

    // Step 6: rewrite configured fields to phantoms. `admitted_consumers`
    // is the route's own name — the minimal viable admission scope for
    // this phase (live cross-consumer egress resolution is out of scope
    // per this phase's ADR / scope-limit note, 114-CONTEXT.md D-10).
    let admitted_consumers: HashSet<String> = HashSet::from([route_id.to_string()]);
    let rewritten_paths = match crate::capture::rewrite_response_fields(
        &mut body,
        &capture.response_fields,
        capture_store,
        admitted_consumers,
    ) {
        Ok(paths) => paths,
        Err(e) => {
            // Safe to log: `rewrite_response_fields` only errors on RNG
            // failure or phantom construction — never on body content.
            debug!("OAuth capture: field rewrite failed for route '{route_id}': {e}");
            deny("configured capture field rewrite failed").await;
            send_error(stream, 502, "Bad Gateway").await?;
            return Ok(());
        }
    };

    // Step 7: fail-closed backstop (D-07) — an unconfigured token-shaped
    // field surviving rewrite must never be forwarded (provider-config
    // drift is exactly the leak this backstop exists to close).
    if let Err(e) = crate::capture::reject_unrewritten_token_fields(&body, &rewritten_paths) {
        // Safe to log: `reject_unrewritten_token_fields` redacts the
        // enclosing JSON path (CR-01) — the enclosing object KEYS are
        // response-derived and a hostile upstream can put a real token in
        // one. It reports the matched field name (drawn from a fixed set)
        // and a depth only.
        debug!("OAuth capture: fail-closed backstop tripped for route '{route_id}': {e}");
        deny(
            "an unconfigured token-shaped field survived rewrite on a capture-declared route; \
             declare it in this route's capture.response_fields",
        )
        .await;
        send_error(stream, 502, "Bad Gateway").await?;
        return Ok(());
    }

    // Step 8: re-serialize and reassemble. Content-Length is recomputed
    // from the ACTUAL rewritten body, never the original.
    let rewritten_body = match serde_json::to_vec(&body) {
        Ok(bytes) => bytes,
        Err(e) => {
            // Safe to log: a serialization failure on an already-parsed,
            // already-rewritten `Value` carries no response bytes.
            debug!("OAuth capture: re-serialization failed for route '{route_id}': {e}");
            deny("failed to re-serialize the rewritten response body").await;
            send_error(stream, 502, "Bad Gateway").await?;
            return Ok(());
        }
    };

    let mut response = format!("{status_line}\r\n");
    for (name, value) in filter_headers(header_block, "") {
        let lower = name.to_lowercase();
        if lower == "transfer-encoding" {
            continue;
        }
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str(&format!("Content-Length: {}\r\n\r\n", rewritten_body.len()));

    stream.write_all(response.as_bytes()).await?;
    stream.write_all(&rewritten_body).await?;
    stream.flush().await?;

    // Step 9: audit — reads back the (already-rewritten, phantom) values
    // at each rewritten path for the audit context. Never touches
    // unrewritten data, so it can never surface a real token.
    let phantom_ids: Vec<String> = rewritten_paths
        .iter()
        .filter_map(|path| read_capture_body_path(&body, path))
        .map(str::to_string)
        .collect();
    audit::log_allowed(
        audit_log,
        audit::ProxyMode::Reverse,
        &audit::EventContext {
            route_id: Some(route_id),
            capture_context: Some(nono::undo::CaptureAuditContext {
                route_id: route_id.to_string(),
                rewritten_fields: rewritten_paths.clone(),
                phantom_ids,
            }),
            ..Default::default()
        },
        route_id,
        0,
        "",
    );

    Ok(())
}

/// Shared capture-dispatch guard used by relay sites 2 and 3
/// (`handle_spiffe_route` / `handle_spiffe_assertion_credential`, Plan
/// 114-06, T-114-18): when `capture` is `Some`, forwards to
/// `relay_response_with_capture` and returns its result — the caller MUST
/// `return` this immediately, mirroring site 1's own unconditional `return`
/// in `handle_reverse_proxy` so a capture-declared route can never fall
/// through into that site's own unbuffered streaming relay loop (D-06).
/// Returns `None` when `capture` is `None`, telling the caller to proceed
/// to its own streaming loop unchanged.
///
/// Extracted into its own function — rather than 3 independent copies of
/// site 1's inline `if let` — specifically so sites 2/3's wiring is
/// directly testable. `ManagedUpstreamAuth`/`SpiffeAssertionTokenCache`
/// have no test-only constructor that bypasses a live SPIRE Workload API
/// connection (Plans 113-03/04/05's documented constraint — see this
/// file's `spiffe_route_denies_missing_session_token_before_credential_acquisition`
/// test's own doc comment for the same constraint applied to auth-gate
/// testing), so `handle_spiffe_route`/`handle_spiffe_assertion_credential`
/// cannot be driven end-to-end in a unit test. This function takes only
/// the parameters available at the exact point each site calls it —
/// none of which require credential acquisition or an upstream
/// connection — so the literal call each site makes in production is
/// independently, directly testable (see
/// `capture_rewrites_via_spiffe_route_site` and
/// `capture_rewrites_via_spiffe_assertion_site`).
async fn relay_capture_if_declared<R: AsyncRead + Unpin>(
    tls_stream: &mut R,
    stream: &mut TcpStream,
    capture: Option<&crate::config::CaptureConfig>,
    capture_store: &crate::capture::CapturePhantomStore,
    route_id: &str,
    audit_log: Option<&audit::SharedAuditLog>,
) -> Option<Result<()>> {
    let capture = capture?;
    Some(
        relay_response_with_capture(
            tls_stream,
            stream,
            capture,
            capture_store,
            route_id,
            audit_log,
        )
        .await,
    )
}

/// Request-side mirror of [`relay_response_with_capture`]'s response
/// rewrite (SEC-02, D-02r): resolves any previously-minted phantom found
/// at a `capture.request_nonce_fields` dot-path in `body` back to its real
/// captured value via [`crate::capture::resolve_request_nonce_fields`],
/// before the request is forwarded to upstream. This is the mint-to-resolve
/// loop's production call site — before this function was wired into
/// `handle_reverse_proxy`, `CapturePhantomStore::resolve()` had no caller
/// outside `capture.rs`'s own tests, so a phantom handed to the sandboxed
/// client could never actually be redeemed (T-114-19b, plan-checker
/// blocker).
///
/// Returns `body` UNCHANGED (byte-for-byte) when `capture` is `None`, when
/// `capture.request_nonce_fields` is empty, or when `body` is not valid
/// JSON. This is the safe structural default: a capture-declared route
/// with no configured nonce fields is unaffected, and a malformed body is
/// forwarded as-is — matching the fork's existing behavior for every other
/// route, never a panic, never a dropped request. Also returns `body`
/// unchanged (skipping a needless re-serialize round-trip that could
/// reformat whitespace/field-order) when
/// `resolve_request_nonce_fields` resolves nothing.
///
/// # Fails closed on SUBSTITUTION, not on the request
///
/// An unresolved or unadmitted phantom is left UNCHANGED in the forwarded
/// body — never replaced by a real token for the wrong consumer. The
/// request still reaches upstream carrying the unresolved phantom, which
/// upstream rejects as an invalid credential — a functional no-op, not a
/// security gap. The phantom is shape-only (`alg: none` for JWT-shaped
/// phantoms) and was already exposed to this same consumer in the
/// response that minted it, so forwarding it leaks nothing new. See
/// `capture_egress_never_substitutes_real_token_for_unadmitted_consumer`
/// — do not read this as "the request is denied."
///
/// No call in this function's path ever logs, audits, or otherwise
/// records the real resolved value.
fn resolve_capture_request_body(
    body: &[u8],
    capture: Option<&crate::config::CaptureConfig>,
    capture_store: &crate::capture::CapturePhantomStore,
    consumer: &str,
) -> Vec<u8> {
    let Some(capture) = capture else {
        return body.to_vec();
    };
    if capture.request_nonce_fields.is_empty() {
        return body.to_vec();
    }
    let Ok(mut value) = serde_json::from_slice::<Value>(body) else {
        return body.to_vec();
    };
    let resolved_count = crate::capture::resolve_request_nonce_fields(
        &mut value,
        &capture.request_nonce_fields,
        capture_store,
        consumer,
    );
    if resolved_count == 0 {
        return body.to_vec();
    }
    serde_json::to_vec(&value).unwrap_or_else(|_| body.to_vec())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod capture_relay_tests {
    use super::*;

    #[test]
    fn capture_find_header_end_locates_boundary() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}";
        let pos = find_header_end(raw).expect("boundary must be found");
        assert_eq!(&raw[pos..pos + 4], b"\r\n\r\n");
    }

    #[test]
    fn capture_find_header_end_none_when_headers_incomplete() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n";
        assert!(find_header_end(raw).is_none());
    }

    #[test]
    fn capture_decode_chunked_body_reassembles_two_chunks() {
        let chunked = b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
        let decoded = decode_chunked_body(chunked).expect("well-formed chunked body must decode");
        assert_eq!(decoded, b"hello world");
    }

    #[test]
    fn capture_decode_chunked_body_fails_closed_on_non_hex_chunk_size() {
        let chunked = b"zz\r\nhello\r\n0\r\n\r\n";
        assert!(decode_chunked_body(chunked).is_err());
    }

    #[test]
    fn capture_decode_chunked_body_fails_closed_on_missing_terminating_crlf() {
        // Chunk announces 5 bytes but the data + terminator are truncated —
        // must error, never panic on out-of-bounds indexing.
        let chunked = b"5\r\nhel";
        assert!(decode_chunked_body(chunked).is_err());
    }

    #[test]
    fn capture_decode_chunked_body_fails_closed_on_chunk_size_overflow() {
        let chunked = b"ffffffffffffffff\r\nhello\r\n0\r\n\r\n";
        assert!(decode_chunked_body(chunked).is_err());
    }

    #[test]
    fn capture_parse_response_headers_detects_chunked() {
        let headers = parse_capture_response_headers(b"Transfer-Encoding: chunked\r\n");
        assert!(headers.chunked);
        assert!(!headers.content_encoded);
    }

    /// Proves detection is UNCONDITIONAL of status code (Pitfall 2 /
    /// `3c59c62e`): this function has no status parameter at all, so a
    /// caller cannot narrow the check to only 2xx responses the way
    /// upstream's own pre-`3c59c62e` code did.
    #[test]
    fn capture_parse_response_headers_detects_content_encoding_regardless_of_status() {
        let headers_2xx = parse_capture_response_headers(b"Content-Encoding: gzip\r\n");
        assert!(headers_2xx.content_encoded);
        let headers_non_2xx_shaped_headers =
            parse_capture_response_headers(b"content-encoding: br\r\n");
        assert!(headers_non_2xx_shaped_headers.content_encoded);
    }

    #[test]
    fn capture_parse_response_headers_ignores_empty_content_encoding() {
        let headers = parse_capture_response_headers(b"Content-Encoding: \r\n");
        assert!(!headers.content_encoded);
    }

    #[tokio::test]
    async fn capture_read_capped_response_denies_when_cap_exceeded_with_no_content_length() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let cap = 16usize;
        let body = b"this response body is definitely longer than sixteen bytes and carries \
                      no Content-Length header at all";
        let write_task = tokio::spawn(async move {
            client.write_all(body).await.unwrap();
            // Close the write half to simulate connection-close-delimited framing.
            drop(client);
        });
        let result = read_capped_response(&mut server, cap, Duration::from_secs(5)).await;
        assert!(
            result.is_err(),
            "response exceeding the cap with no Content-Length must be denied, not released"
        );
        write_task.await.unwrap();
    }

    #[tokio::test]
    async fn capture_read_capped_response_succeeds_within_cap() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let write_task = tokio::spawn(async move {
            client.write_all(b"short body").await.unwrap();
            drop(client);
        });
        let result = read_capped_response(&mut server, 4096, Duration::from_secs(5)).await;
        assert_eq!(
            result.expect("within-cap read must succeed").as_slice(),
            b"short body"
        );
        write_task.await.unwrap();
    }

    #[tokio::test]
    async fn capture_read_capped_response_denies_on_timeout() {
        // Never write anything and never close — read_capped_response must
        // deny via the timeout branch rather than hang forever.
        let (client, mut server) = tokio::io::duplex(64);
        let result = read_capped_response(&mut server, 4096, Duration::from_millis(20)).await;
        assert!(
            result.is_err(),
            "a stalled upstream must be denied via timeout, not hung on forever"
        );
        drop(client);
    }

    // ========================================================================
    // relay_response_with_capture end-to-end behavior tests (Task 3). These
    // are the plan's 4 required VALIDATION.md behaviors, driven through the
    // real dispatch function with an in-memory `tokio::io::duplex()` pair
    // standing in for the upstream `tls_stream` (see `read_capped_response`'s
    // doc comment for why: no TLS-server test fixture exists anywhere in
    // this crate) and a real loopback `TcpStream` pair for the client-facing
    // `stream` (mirroring the pattern this file's pre-existing
    // `handle_reverse_proxy` tests already use for that side).
    // ========================================================================

    use tokio::net::TcpListener;

    fn capture_config_with_field(path: &str) -> crate::config::CaptureConfig {
        capture_config_with_field_and_cap(path, None)
    }

    fn capture_config_with_field_and_cap(
        path: &str,
        max_response_bytes: Option<usize>,
    ) -> crate::config::CaptureConfig {
        crate::config::CaptureConfig {
            response_fields: vec![crate::config::CaptureResponseField {
                path: path.to_string(),
                kind: crate::config::CaptureResponseFieldKind::Opaque,
            }],
            request_nonce_fields: vec![],
            max_response_bytes,
        }
    }

    /// A real loopback `TcpStream` pair standing in for the client-facing
    /// `stream` parameter — `relay_response_with_capture` requires a
    /// concrete `&mut TcpStream` there (only the upstream side is
    /// genericized), so this mirrors the exact pattern this file's
    /// pre-existing `handle_reverse_proxy` tests already use.
    async fn client_stream_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (connect_result, accept_result) =
            tokio::join!(TcpStream::connect(addr), listener.accept());
        (connect_result.unwrap(), accept_result.unwrap().0)
    }

    /// Drive `relay_response_with_capture` with `upstream_response_bytes`
    /// as the raw bytes an upstream would have sent, returning everything
    /// written to the client-facing side. Shuts down the server-side
    /// stream's write half after the relay function returns so the
    /// client's `read_to_end` observes EOF instead of hanging.
    async fn run_relay_with_capture(
        upstream_response_bytes: &'static [u8],
        capture: &crate::config::CaptureConfig,
        store: &crate::capture::CapturePhantomStore,
        route_id: &str,
    ) -> (Result<()>, Vec<u8>) {
        run_relay_with_capture_audited(upstream_response_bytes, capture, store, route_id, None)
            .await
    }

    /// `run_relay_with_capture` with an optional in-memory audit log, so a
    /// test can assert on what the relay actually WROTE to the audit
    /// ledger — not just on what it wrote to the client (CR-01,
    /// 114-REVIEW.md: the leak was in the audit record, which every
    /// pre-existing relay test passed `None` for and therefore could not
    /// see).
    async fn run_relay_with_capture_audited(
        upstream_response_bytes: &'static [u8],
        capture: &crate::config::CaptureConfig,
        store: &crate::capture::CapturePhantomStore,
        route_id: &str,
        audit_log: Option<&audit::SharedAuditLog>,
    ) -> (Result<()>, Vec<u8>) {
        let (mut upstream_client, mut upstream_server) = tokio::io::duplex(1 << 20);
        let (mut client_stream, mut server_stream) = client_stream_pair().await;

        let upstream_write = async move {
            upstream_client
                .write_all(upstream_response_bytes)
                .await
                .unwrap();
            drop(upstream_client);
        };

        let relay = async {
            let result = relay_response_with_capture(
                &mut upstream_server,
                &mut server_stream,
                capture,
                store,
                route_id,
                audit_log,
            )
            .await;
            // Close the write half so the client's read_to_end below sees
            // EOF instead of hanging (server_stream itself stays alive
            // until this async block's scope ends, so it never
            // auto-closes on its own).
            let _ = server_stream.shutdown().await;
            result
        };

        let read_client = async {
            let mut buf = Vec::new();
            client_stream.read_to_end(&mut buf).await.unwrap();
            buf
        };

        let (relay_result, received, ()) = tokio::join!(relay, read_client, upstream_write);
        (relay_result, received)
    }

    #[tokio::test]
    async fn capture_rewrites_configured_fields() {
        let capture = capture_config_with_field("access_token");
        let store = crate::capture::CapturePhantomStore::new();
        let body: &'static [u8] =
            br#"{"access_token":"real-secret-token-value","unrelated":"leave-me-alone"}"#;
        let upstream_response: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).unwrap()
            )
            .into_bytes()
            .into_boxed_slice(),
        );

        let (relay_result, received) =
            run_relay_with_capture(upstream_response, &capture, &store, "testroute").await;
        relay_result.expect("relay must succeed for a well-formed, fully-configured response");

        let received_str = String::from_utf8(received).expect("response must be valid UTF-8");
        assert!(
            received_str.starts_with("HTTP/1.1 200"),
            "expected 200 OK to be forwarded; got: {received_str}"
        );
        assert!(
            !received_str.contains("real-secret-token-value"),
            "the real token must never reach the client; got: {received_str}"
        );
        assert!(
            received_str.contains("leave-me-alone"),
            "an unrelated field must be forwarded unchanged; got: {received_str}"
        );
        assert!(
            received_str.to_lowercase().contains("content-length:"),
            "Content-Length must be present and recomputed for the rewritten body; got: {received_str}"
        );
    }

    #[tokio::test]
    async fn capture_fails_closed_on_unrewritten_token_field() {
        // No response_fields configured at all — access_token is an
        // unconfigured token-shaped field and must be denied, never
        // forwarded, by the reject_unrewritten_token_fields backstop.
        let capture = crate::config::CaptureConfig {
            response_fields: vec![],
            request_nonce_fields: vec![],
            max_response_bytes: None,
        };
        let store = crate::capture::CapturePhantomStore::new();
        let body: &'static [u8] = br#"{"access_token":"leaked-real-token"}"#;
        let upstream_response: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).unwrap()
            )
            .into_bytes()
            .into_boxed_slice(),
        );

        let (relay_result, received) =
            run_relay_with_capture(upstream_response, &capture, &store, "testroute").await;
        relay_result.expect("relay must return Ok(()) even on a fail-closed deny");

        let received_str = String::from_utf8(received).expect("response must be valid UTF-8");
        assert!(
            received_str.starts_with("HTTP/1.1 502"),
            "an unconfigured token-shaped field must be denied (502); got: {received_str}"
        );
        assert!(
            !received_str.contains("leaked-real-token"),
            "the real token must never reach the client even in the deny response; got: {received_str}"
        );
    }

    #[tokio::test]
    async fn capture_denies_content_encoded_response() {
        let capture = capture_config_with_field("access_token");
        let store = crate::capture::CapturePhantomStore::new();

        // Proven for BOTH a 2xx and a non-2xx status — unconditional
        // rejection (Pitfall 2 / 3c59c62e), not narrowed to 2xx only.
        for status_line in ["HTTP/1.1 200 OK", "HTTP/1.1 401 Unauthorized"] {
            let upstream_response: &'static [u8] = Box::leak(
                format!(
                    "{status_line}\r\nContent-Encoding: gzip\r\nContent-Length: 5\r\n\r\nabcde"
                )
                .into_bytes()
                .into_boxed_slice(),
            );

            let (relay_result, received) =
                run_relay_with_capture(upstream_response, &capture, &store, "testroute").await;
            relay_result.expect("relay must return Ok(()) even on a fail-closed deny");

            let received_str = String::from_utf8(received).expect("response must be valid UTF-8");
            assert!(
                received_str.starts_with("HTTP/1.1 502"),
                "Content-Encoding must be denied unconditionally of status (status line was \
                 '{status_line}'); got: {received_str}"
            );
        }
    }

    #[tokio::test]
    async fn capture_buffer_cap_exceeded_denies_response() {
        let capture = capture_config_with_field_and_cap("access_token", Some(16));
        let store = crate::capture::CapturePhantomStore::new();
        // No Content-Length header at all — proves the cap is enforced
        // against actual bytes read, not a Content-Length pre-check.
        let upstream_response: &'static [u8] = b"HTTP/1.1 200 OK\r\n\r\n{\"access_token\":\"this body is definitely longer than the sixteen byte cap\"}";

        let (relay_result, received) =
            run_relay_with_capture(upstream_response, &capture, &store, "testroute").await;
        relay_result.expect("relay must return Ok(()) even on a fail-closed deny");

        let received_str = String::from_utf8(received).expect("response must be valid UTF-8");
        assert!(
            received_str.starts_with("HTTP/1.1 502"),
            "a response exceeding the buffer cap must be denied, never released unrewritten; \
             got: {received_str}"
        );
    }

    /// CR-02 regression proof (114-REVIEW.md): the capture relay must
    /// terminate on the response's OWN HTTP framing, NOT on upstream EOF.
    ///
    /// This is the one test in this module that deliberately does NOT close
    /// the upstream write half. Every pre-existing relay test
    /// (`run_relay_with_capture`, `run_relay_capture_if_declared`,
    /// `spawn_hermetic_tls_upstream`, `non_capture_route_still_streams_unbuffered`)
    /// forces EOF with `drop(upstream_client)` / `tls.shutdown()`, so none
    /// of them could fail on the original defect: `read_capped_response`
    /// looped until `Ok(0)` and nothing else terminated it. Every real
    /// OAuth token endpoint (Okta, Auth0, Google, Entra, GitHub) is
    /// HTTP/1.1 with persistent connections, so against a real provider
    /// every capture request blocked for the full `CAPTURE_READ_TIMEOUT`
    /// (30 s) and then returned 502 — the feature did not work at all.
    ///
    /// Both framings are proven: `Content-Length` and chunked's terminal
    /// `0\r\n\r\n`. The whole relay is wrapped in a 5-second timeout, far
    /// below `CAPTURE_READ_TIMEOUT`, so the test fails (rather than hangs)
    /// if framing-based termination regresses.
    #[tokio::test]
    async fn capture_relay_terminates_without_upstream_eof() {
        let json = r#"{"access_token":"real-secret-token-value","unrelated":"keep-me"}"#;
        let content_length_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            json.len(),
            json
        );
        let chunked_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Transfer-Encoding: chunked\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
            json.len(),
            json
        );

        for (label, upstream_response) in [
            ("content-length", content_length_response),
            ("chunked", chunked_response),
        ] {
            let capture = capture_config_with_field("access_token");
            let store = crate::capture::CapturePhantomStore::new();

            let (mut upstream_client, mut upstream_server) = tokio::io::duplex(1 << 20);
            let (mut client_stream, mut server_stream) = client_stream_pair().await;

            upstream_client
                .write_all(upstream_response.as_bytes())
                .await
                .unwrap();
            // DELIBERATELY NOT dropped/shutdown: `upstream_client` stays
            // alive for the whole test, exactly like a keep-alive HTTP/1.1
            // token endpoint that does not close after responding. This is
            // the entire point of the test.

            let relay = async {
                let result = relay_response_with_capture(
                    &mut upstream_server,
                    &mut server_stream,
                    &capture,
                    &store,
                    "testroute",
                    None,
                )
                .await;
                let _ = server_stream.shutdown().await;
                result
            };
            let read_client = async {
                let mut buf = Vec::new();
                client_stream.read_to_end(&mut buf).await.unwrap();
                buf
            };

            let (relay_result, received) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::join!(relay, read_client)
            })
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "[{label}] the capture relay must terminate on HTTP framing, not on upstream \
                     EOF — it blocked with the upstream connection still open, which is what \
                     every real keep-alive token endpoint does"
                )
            });

            relay_result.expect("relay must succeed for a well-formed, fully-configured response");
            let received_str = String::from_utf8(received).expect("response must be valid UTF-8");
            assert!(
                received_str.starts_with("HTTP/1.1 200"),
                "[{label}] expected the rewritten 200 to be relayed promptly; got: {received_str}"
            );
            assert!(
                !received_str.contains("real-secret-token-value"),
                "[{label}] the real token must never reach the client; got: {received_str}"
            );
            assert!(
                received_str.contains("keep-me"),
                "[{label}] unrelated fields must survive the rewrite; got: {received_str}"
            );

            // Keep the upstream write half alive to the very end of the
            // iteration so nothing can accidentally supply the EOF this
            // test proves is unnecessary.
            drop(upstream_client);
        }
    }

    /// CR-01 regression proof (114-REVIEW.md): NO deny branch of the
    /// enforcement point may put upstream-response-derived bytes into the
    /// operator's log or the persisted audit ledger.
    ///
    /// The original defect: `decode_chunked_body` interpolated the raw
    /// "chunk-size line" into its error, and `relay_response_with_capture`
    /// forwarded that error verbatim into `deny()` -> `warn!` AND
    /// `audit::log_denied` -> `NetworkAuditEvent.reason` -> session
    /// metadata ON DISK. An upstream that declares `Transfer-Encoding:
    /// chunked` but sends an identity JSON body makes the chunk-size line
    /// literally `{"access_token":"<REAL TOKEN>"...`, so a real OAuth token
    /// was written to disk — falsifying D-08, `capture.rs`'s module doc,
    /// and ADR-114 Consequence 2. Because the session audit directory is
    /// readable by the sandboxed child, that is a route for the confined
    /// agent to read the real token back out.
    ///
    /// This drives three distinct deny branches (chunked decode, JSON
    /// parse, fail-closed backstop), each with the token placed exactly
    /// where that branch used to echo it, and asserts the token appears in
    /// NEITHER the bytes sent to the client NOR any audit event's `reason`.
    /// `capture_store_holds_only_in_memory` could not catch this: it
    /// string-scans `capture.rs`, and the write happens in `reverse.rs`.
    #[tokio::test]
    async fn capture_deny_reason_never_contains_response_derived_bytes() {
        const REAL_TOKEN: &str = "REAL-OAUTH-TOKEN-ce7b1a2d-MUST-NEVER-BE-LOGGED";

        // (a) chunked declared, identity body sent -> the whole first line
        //     of the JSON body becomes the "chunk-size line".
        let chunked_lie: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
                 {{\"access_token\":\"{REAL_TOKEN}\"}}\r\n"
            )
            .into_bytes()
            .into_boxed_slice(),
        );
        // (b) unparseable body -> the serde_json error path.
        let unparseable: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n\
                 not-json {{\"access_token\":\"{REAL_TOKEN}\"}}"
            )
            .into_bytes()
            .into_boxed_slice(),
        );
        // (c) the token embedded in an enclosing object KEY, with an
        //     unconfigured token-shaped field inside it -> the fail-closed
        //     backstop's JSON-path echo.
        let token_in_key: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n\
                 {{\"{REAL_TOKEN}\":{{\"access_token\":\"nested\"}}}}"
            )
            .into_bytes()
            .into_boxed_slice(),
        );

        let no_fields = crate::config::CaptureConfig {
            response_fields: vec![],
            request_nonce_fields: vec![],
            max_response_bytes: None,
        };
        let with_field = capture_config_with_field("access_token");

        // The expected reason fragment per case is asserted too: it proves
        // each case really reached the deny branch it was constructed for,
        // rather than short-circuiting on some earlier branch and passing
        // the "no token in the reason" assertion vacuously.
        for (label, response, capture, expected_reason) in [
            (
                "chunked-size-line",
                chunked_lie,
                &with_field,
                "chunked response body could not be decoded",
            ),
            (
                "unparseable-json",
                unparseable,
                &with_field,
                "is not parseable JSON",
            ),
            (
                "token-in-json-key",
                token_in_key,
                &no_fields,
                "unconfigured token-shaped field survived rewrite",
            ),
        ] {
            let store = crate::capture::CapturePhantomStore::new();
            let audit_log: audit::SharedAuditLog =
                std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

            let (relay_result, received) = run_relay_with_capture_audited(
                response,
                capture,
                &store,
                "testroute",
                Some(&audit_log),
            )
            .await;
            relay_result.expect("relay must return Ok(()) even on a fail-closed deny");

            let received_str = String::from_utf8_lossy(&received).to_string();
            assert!(
                received_str.starts_with("HTTP/1.1 502"),
                "[{label}] this case must take a deny branch (502); got: {received_str}"
            );
            assert!(
                !received_str.contains(REAL_TOKEN),
                "[{label}] the real token must never reach the client; got: {received_str}"
            );

            let events = audit::drain_audit_events(&audit_log);
            assert!(
                !events.is_empty(),
                "[{label}] the deny must have produced an audit event to assert on"
            );
            let mut saw_expected_branch = false;
            for event in &events {
                let reason = event.reason.clone().unwrap_or_default();
                assert!(
                    !reason.contains(REAL_TOKEN),
                    "[{label}] the real token must never reach the persisted audit ledger's \
                     `reason` field; got: {reason}"
                );
                if reason.contains(expected_reason) {
                    saw_expected_branch = true;
                }
            }
            assert!(
                saw_expected_branch,
                "[{label}] expected the deny to come from the branch containing \
                 '{expected_reason}', so this case is not passing vacuously; got: {events:?}"
            );
        }
    }

    /// D-06 regression proof: `relay_response_streaming` (the unbuffered
    /// path used when `route.capture` is `None`) forwards bytes verbatim —
    /// no rewrite/buffering processing — proving streaming truly stays the
    /// default now that `relay_response_with_capture` exists as a second
    /// dispatch path.
    #[tokio::test]
    async fn non_capture_route_still_streams_unbuffered() {
        let (mut upstream_client, mut upstream_server) = tokio::io::duplex(1 << 16);
        let (mut client_stream, mut server_stream) = client_stream_pair().await;

        let body = br#"{"access_token":"this field is NOT rewritten on the streaming path"}"#;
        let upstream_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );

        let upstream_write = async move {
            upstream_client
                .write_all(upstream_response.as_bytes())
                .await
                .unwrap();
            drop(upstream_client);
        };

        let relay = async {
            let status = relay_response_streaming(&mut upstream_server, &mut server_stream)
                .await
                .unwrap();
            let _ = server_stream.shutdown().await;
            status
        };

        let read_client = async {
            let mut buf = Vec::new();
            client_stream.read_to_end(&mut buf).await.unwrap();
            buf
        };

        let (status, received, ()) = tokio::join!(relay, read_client, upstream_write);
        assert_eq!(status, 200);
        let received_str = String::from_utf8(received).unwrap();
        // Verbatim passthrough: the real "token" value survives unchanged
        // on the streaming path, unlike the capture path's rewrite.
        assert!(
            received_str.contains("this field is NOT rewritten on the streaming path"),
            "streaming path must forward bytes verbatim, not rewrite them; got: {received_str}"
        );
    }

    // ========================================================================
    // Plan 114-06 — relay sites 2/3 (`handle_spiffe_route` /
    // `handle_spiffe_assertion_credential`) capture-dispatch wiring (T-114-18,
    // WR-13). Both handlers call `relay_capture_if_declared` at the exact
    // point these tests drive — see that function's doc comment for why a
    // full end-to-end drive of either handler is impossible without a live
    // SPIRE Workload API connection.
    // ========================================================================

    /// Drive `relay_capture_if_declared` exactly as `handle_spiffe_route` and
    /// `handle_spiffe_assertion_credential` each call it at their own call
    /// site — proving the function each site actually invokes rewrites the
    /// response when `capture` is configured, independent of
    /// `relay_response_with_capture`'s own already-proven behavior (114-05)
    /// and independent of site 1's `capture_rewrites_configured_fields`.
    async fn run_relay_capture_if_declared(
        upstream_response_bytes: &'static [u8],
        capture: Option<&crate::config::CaptureConfig>,
        store: &crate::capture::CapturePhantomStore,
        route_id: &str,
    ) -> (Option<Result<()>>, Vec<u8>) {
        let (mut upstream_client, mut upstream_server) = tokio::io::duplex(1 << 20);
        let (mut client_stream, mut server_stream) = client_stream_pair().await;

        let upstream_write = async move {
            upstream_client
                .write_all(upstream_response_bytes)
                .await
                .unwrap();
            drop(upstream_client);
        };

        let relay = async {
            let result = relay_capture_if_declared(
                &mut upstream_server,
                &mut server_stream,
                capture,
                store,
                route_id,
                None,
            )
            .await;
            let _ = server_stream.shutdown().await;
            result
        };

        let read_client = async {
            let mut buf = Vec::new();
            client_stream.read_to_end(&mut buf).await.unwrap();
            buf
        };

        let (relay_result, received, ()) = tokio::join!(relay, read_client, upstream_write);
        (relay_result, received)
    }

    /// Site 2 (`handle_spiffe_route`) proof: a route reachable via the
    /// direct JWT-SVID bearer dispatch path that ALSO declares `capture`
    /// gets its response buffered and rewritten — the real OAuth token
    /// never reaches the sandboxed client, even though request-side auth
    /// for this route is SPIFFE, not the static-credential path site 1
    /// covers.
    #[tokio::test]
    async fn capture_rewrites_via_spiffe_route_site() {
        let capture = capture_config_with_field("access_token");
        let store = crate::capture::CapturePhantomStore::new();
        let body: &'static [u8] =
            br#"{"access_token":"real-secret-token-value-site2","unrelated":"leave-me-alone"}"#;
        let upstream_response: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).unwrap()
            )
            .into_bytes()
            .into_boxed_slice(),
        );

        let (relay_result, received) = run_relay_capture_if_declared(
            upstream_response,
            Some(&capture),
            &store,
            "spiffe-route-svc",
        )
        .await;
        let relay_result = relay_result.expect(
            "relay_capture_if_declared must return Some(..) when capture is configured — this \
             is exactly what handle_spiffe_route's own call site does with \
             route.capture.as_ref()",
        );
        relay_result.expect("relay must succeed for a well-formed, fully-configured response");

        let received_str = String::from_utf8(received).expect("response must be valid UTF-8");
        assert!(
            received_str.starts_with("HTTP/1.1 200"),
            "expected 200 OK to be forwarded; got: {received_str}"
        );
        assert!(
            !received_str.contains("real-secret-token-value-site2"),
            "the real token must never reach the client via the SPIFFE-bearer dispatch path \
             (site 2); got: {received_str}"
        );
        assert!(
            received_str.contains("leave-me-alone"),
            "an unrelated field must be forwarded unchanged; got: {received_str}"
        );
    }

    /// Site 3 (`handle_spiffe_assertion_credential`) proof: a route
    /// reachable via the RFC 7523 jwt-bearer assertion-exchange dispatch
    /// path that ALSO declares `capture` gets its response buffered and
    /// rewritten — independent of sites 1 and 2's tests (different test
    /// function, different route_id, different token value).
    #[tokio::test]
    async fn capture_rewrites_via_spiffe_assertion_site() {
        let capture = capture_config_with_field("access_token");
        let store = crate::capture::CapturePhantomStore::new();
        let body: &'static [u8] =
            br#"{"access_token":"real-secret-token-value-site3","unrelated":"leave-me-alone"}"#;
        let upstream_response: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).unwrap()
            )
            .into_bytes()
            .into_boxed_slice(),
        );

        let (relay_result, received) = run_relay_capture_if_declared(
            upstream_response,
            Some(&capture),
            &store,
            "spiffe-assertion-svc",
        )
        .await;
        let relay_result = relay_result.expect(
            "relay_capture_if_declared must return Some(..) when capture is configured — this \
             is exactly what handle_spiffe_assertion_credential's own call site does with \
             ctx.route_store.get(service).and_then(|r| r.capture.as_ref())",
        );
        relay_result.expect("relay must succeed for a well-formed, fully-configured response");

        let received_str = String::from_utf8(received).expect("response must be valid UTF-8");
        assert!(
            received_str.starts_with("HTTP/1.1 200"),
            "expected 200 OK to be forwarded; got: {received_str}"
        );
        assert!(
            !received_str.contains("real-secret-token-value-site3"),
            "the real token must never reach the client via the SPIFFE-assertion dispatch path \
             (site 3); got: {received_str}"
        );
        assert!(
            received_str.contains("leave-me-alone"),
            "an unrelated field must be forwarded unchanged; got: {received_str}"
        );
    }

    /// Regression guard: `relay_capture_if_declared` returns `None` when the
    /// route/service has no capture config — proving the caller (either
    /// site 2 or site 3) correctly falls through to its own unmodified
    /// streaming loop rather than being silently swallowed by this shared
    /// helper. Mirrors `non_capture_route_still_streams_unbuffered`'s D-06
    /// regression intent at this function's own level.
    #[tokio::test]
    async fn relay_capture_if_declared_returns_none_when_capture_absent() {
        let store = crate::capture::CapturePhantomStore::new();
        let (relay_result, _received) =
            run_relay_capture_if_declared(b"unused", None, &store, "no-capture-svc").await;
        assert!(
            relay_result.is_none(),
            "relay_capture_if_declared must return None when capture is None, so the caller \
             falls through to its own streaming loop unchanged (D-06)"
        );
    }
}

// ============================================================================
// Plan 114-06 Task 3 — mint-to-resolve loop closure
// (`resolve_capture_request_body`, wired into `handle_reverse_proxy`'s real
// outbound request-body path). SEC-02, D-02r, T-114-19b/T-114-19c.
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod capture_egress_tests {
    use super::*;
    use std::collections::HashSet;
    use zeroize::Zeroizing;

    fn capture_with_nonce_field(path: &str) -> crate::config::CaptureConfig {
        crate::config::CaptureConfig {
            response_fields: vec![],
            request_nonce_fields: vec![path.to_string()],
            max_response_bytes: None,
        }
    }

    /// (i) A minted phantom, admitted for `consumer`, resolves to the real
    /// captured token in the request body — the mint-to-resolve loop's
    /// core behavior, exercised via the same pure function
    /// `handle_reverse_proxy` calls in production.
    #[test]
    fn capture_egress_resolves_admitted_phantom_in_request_body() {
        let store = crate::capture::CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("testservice".to_string());
        let phantom = store
            .mint(Zeroizing::new(b"real-nonce-value".to_vec()), admitted)
            .unwrap();
        let capture = capture_with_nonce_field("code_verifier");
        let body = serde_json::json!({ "code_verifier": phantom, "unrelated": "leave-me-alone" })
            .to_string()
            .into_bytes();

        let resolved = resolve_capture_request_body(&body, Some(&capture), &store, "testservice");
        let resolved_value: Value = serde_json::from_slice(&resolved).unwrap();
        assert_eq!(
            resolved_value["code_verifier"],
            serde_json::json!("real-nonce-value"),
            "an admitted phantom must resolve to the real captured token in the outbound \
             request body"
        );
        assert_eq!(
            resolved_value["unrelated"],
            serde_json::json!("leave-me-alone")
        );
    }

    /// (ii) A phantom minted for a DIFFERENT consumer, or an unknown value,
    /// is left UNCHANGED — never substituted with a real token for an
    /// unauthorized/unknown presenter. This is fail-closed on
    /// SUBSTITUTION, not on the request: the request is still forwarded
    /// (see this function's doc comment); upstream rejects the
    /// still-phantom value as an invalid credential.
    #[test]
    fn capture_egress_never_substitutes_real_token_for_unadmitted_consumer() {
        let store = crate::capture::CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("testservice".to_string());
        let phantom = store
            .mint(Zeroizing::new(b"real-nonce-value".to_vec()), admitted)
            .unwrap();
        let capture = capture_with_nonce_field("code_verifier");

        // Case A: consumer not admitted for this phantom.
        let body_a = serde_json::json!({ "code_verifier": phantom.clone() })
            .to_string()
            .into_bytes();
        let resolved_a =
            resolve_capture_request_body(&body_a, Some(&capture), &store, "not-admitted");
        let resolved_value_a: Value = serde_json::from_slice(&resolved_a).unwrap();
        assert_eq!(
            resolved_value_a["code_verifier"],
            serde_json::json!(phantom),
            "an unadmitted consumer must never receive the real token — the phantom must be \
             forwarded unchanged"
        );

        // Case B: value is not a known phantom at all.
        let body_b = serde_json::json!({ "code_verifier": "not-a-known-phantom" })
            .to_string()
            .into_bytes();
        let resolved_b =
            resolve_capture_request_body(&body_b, Some(&capture), &store, "testservice");
        let resolved_value_b: Value = serde_json::from_slice(&resolved_b).unwrap();
        assert_eq!(
            resolved_value_b["code_verifier"],
            serde_json::json!("not-a-known-phantom"),
            "an unknown value must never be substituted — forwarded unchanged"
        );
    }

    /// Structural default: `capture: None`, empty `request_nonce_fields`,
    /// and a non-JSON body are all forwarded byte-for-byte unchanged — no
    /// route without configured nonce fields is ever affected, and a
    /// malformed body is never dropped or panicked on.
    #[test]
    fn resolve_capture_request_body_forwards_unchanged_when_not_applicable() {
        let store = crate::capture::CapturePhantomStore::new();
        let capture_no_fields = crate::config::CaptureConfig {
            response_fields: vec![],
            request_nonce_fields: vec![],
            max_response_bytes: None,
        };
        let json_body = br#"{"code_verifier":"whatever"}"#.to_vec();
        assert_eq!(
            resolve_capture_request_body(&json_body, None, &store, "testservice"),
            json_body,
            "capture: None must forward the body unchanged"
        );
        assert_eq!(
            resolve_capture_request_body(
                &json_body,
                Some(&capture_no_fields),
                &store,
                "testservice"
            ),
            json_body,
            "empty request_nonce_fields must forward the body unchanged"
        );
        let non_json_body = b"not json at all".to_vec();
        let capture_with_fields = capture_with_nonce_field("code_verifier");
        assert_eq!(
            resolve_capture_request_body(
                &non_json_body,
                Some(&capture_with_fields),
                &store,
                "testservice"
            ),
            non_json_body,
            "a non-JSON body must be forwarded unchanged, never dropped or panicked on"
        );
    }

    // ========================================================================
    // capture_egress_resolution_reaches_upstream_on_live_dispatch_path — the
    // end-to-end wiring proof. Drives a real request through
    // `handle_reverse_proxy` against a real (self-signed, hermetic) TLS
    // upstream and asserts the bytes upstream actually receives contain the
    // REAL token, not the phantom. The two tests above prove
    // `resolve_capture_request_body` computes correctly in isolation; this
    // is the only test in this file that proves the wired call fires with
    // the correct `route.capture` / `ctx.capture_store` / `consumer`
    // arguments on a live request path — a grep proves only that the call
    // was written.
    //
    // `connect_upstream_tls` is unconditional (every reverse-proxy request,
    // capture or not, dials upstream over real TLS), and no TLS-server test
    // fixture existed anywhere in this crate before this plan
    // (`read_capped_response`'s own doc comment, Plan 114-05, explicitly
    // deferred building one as "new, heavy machinery unrelated to the
    // security property under test" for ITS narrower cap-enforcement
    // proof). This plan's Task 3 is a different, wider claim — that the
    // wiring fires correctly on a live dispatch path — which cannot be
    // proven without one. `rcgen` (already a vetted, already-resolved
    // workspace dependency via `nono-cli`) mints a hermetic self-signed
    // cert entirely in-process; no network access, no filesystem
    // certificate fixtures.
    // ========================================================================

    use tokio::net::TcpListener;

    /// Spin up a real, hermetic TLS "upstream" on loopback: accepts exactly
    /// one connection, reads a full HTTP/1.1 request (headers + a
    /// Content-Length-bounded body), replies with a minimal 200 OK JSON
    /// response, and returns the raw bytes it received (for the test to
    /// assert against) via the returned `JoinHandle`.
    async fn spawn_hermetic_tls_upstream() -> (
        u16,
        rustls::pki_types::CertificateDer<'static>,
        tokio::task::JoinHandle<Vec<u8>>,
    ) {
        let key_pair = rcgen::KeyPair::generate().expect("rcgen key generation");
        let params = rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()])
            .expect("rcgen params for IP SAN 127.0.0.1");
        let cert = params
            .self_signed(&key_pair)
            .expect("rcgen self-signed cert");
        let cert_der = cert.der().clone();
        let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der()),
        );
        let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der)
        .expect("hermetic self-signed server config");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        let handle = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut tls = acceptor.accept(tcp).await.unwrap();

            // Read until the headers are complete AND the Content-Length-
            // declared body has fully arrived (or there is no body at all).
            let mut buf = Vec::new();
            let mut tmp = [0u8; 4096];
            loop {
                if let Some(marker_pos) = find_header_end(&buf) {
                    let content_length = extract_content_length(&buf[..marker_pos]).unwrap_or(0);
                    if buf.len() >= marker_pos + 4 + content_length {
                        break;
                    }
                }
                let n = tls.read(&mut tmp).await.unwrap();
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }

            let response_body = b"{\"ok\":true}";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                std::str::from_utf8(response_body).unwrap()
            );
            let _ = tls.write_all(response.as_bytes()).await;
            let _ = tls.flush().await;
            let _ = tls.shutdown().await;

            buf
        });

        (port, cert_der, handle)
    }

    /// The end-to-end wiring proof (plan-checker warning, iteration 2):
    /// a real request carrying a previously-minted, admitted phantom in its
    /// JSON body flows through `handle_reverse_proxy` to a real (hermetic)
    /// TLS upstream. Asserts the bytes upstream actually received contain
    /// the REAL token and do NOT contain the phantom string — proving
    /// `resolve_capture_request_body` fires with the correct
    /// `route.capture` / `ctx.capture_store` / `consumer` arguments on this
    /// live dispatch path, not merely that the call was written.
    #[tokio::test]
    async fn capture_egress_resolution_reaches_upstream_on_live_dispatch_path() {
        let (port, cert_der, upstream_handle) = spawn_hermetic_tls_upstream().await;

        // Client-side TLS trust: trust ONLY this hermetic self-signed cert.
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(cert_der).unwrap();
        let client_tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();
        let client_tls_config_arc = Arc::new(client_tls_config);
        let test_tls_connector =
            tokio_rustls::TlsConnector::from(Arc::clone(&client_tls_config_arc));
        let upstream_pool =
            crate::pool::UpstreamPool::new(Arc::clone(&client_tls_config_arc), false);

        // Seed a phantom admitted for this route's own name ("liveservice"),
        // mirroring `relay_response_with_capture`'s own admission scope
        // (`admitted_consumers: HashSet::from([route_id.to_string()])`).
        let capture_store = crate::capture::CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("liveservice".to_string());
        let phantom = capture_store
            .mint(Zeroizing::new(b"real-nonce-value-e2e".to_vec()), admitted)
            .unwrap();
        assert!(
            !phantom.contains("real-nonce-value-e2e"),
            "sanity: the minted phantom must not itself contain the real value"
        );

        let capture = crate::config::CaptureConfig {
            response_fields: vec![],
            request_nonce_fields: vec!["code_verifier".to_string()],
            max_response_bytes: None,
        };

        let loaded_route = LoadedRoute {
            upstream: format!("https://127.0.0.1:{port}"),
            upstream_host_port: Some(format!("127.0.0.1:{port}")),
            endpoint_rules: crate::config::CompiledEndpointRules::compile(&[]).unwrap(),
            endpoint_policy: crate::config::CompiledEndpointPolicy::compile(None, &[]).unwrap(),
            tls_connector: None,
            tls_client_config: None,
            tls_config_key: None,
            managed_auth: None,
            declares_spiffe: false,
            declares_capture: true,
            capture: Some(capture),
        };
        let mut routes = std::collections::HashMap::new();
        routes.insert("liveservice".to_string(), loaded_route);
        let route_store = RouteStore::from_loaded_routes(routes);

        let credential_store = CredentialStore::empty();
        let session_token = Zeroizing::new("unused-session-token".to_string());
        let filter = ProxyFilter::allow_all();

        let ctx = ReverseProxyCtx {
            route_store: &route_store,
            credential_store: &credential_store,
            session_token: &session_token,
            filter: &filter,
            tls_connector: &test_tls_connector,
            default_tls_config: &client_tls_config_arc,
            upstream_pool: &upstream_pool,
            audit_log: None,
            capture_store: &capture_store,
            // Skip the session-token/phantom-token auth gate entirely — this
            // test's focus is request-body egress resolution, not the
            // (already independently tested) auth boundary.
            require_auth: false,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (connect_result, accept_result) =
            tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept());
        let mut server_stream = accept_result.unwrap().0;
        let mut client_conn = connect_result.unwrap();

        let request_body = serde_json::json!({ "code_verifier": phantom.clone() })
            .to_string()
            .into_bytes();
        let first_line = "POST /liveservice/token HTTP/1.1";
        let remaining_header = format!(
            "Host: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            request_body.len()
        );

        let result = handle_reverse_proxy(
            first_line,
            &mut server_stream,
            remaining_header.as_bytes(),
            &ctx,
            &request_body,
        )
        .await;
        result.expect("handle_reverse_proxy must succeed for a well-formed live request");

        drop(server_stream);
        let mut client_response_buf = Vec::new();
        client_conn
            .read_to_end(&mut client_response_buf)
            .await
            .unwrap();

        let upstream_received = upstream_handle
            .await
            .expect("hermetic upstream task must not panic");
        let upstream_received_str =
            String::from_utf8(upstream_received).expect("upstream must receive valid UTF-8");

        assert!(
            upstream_received_str.contains("real-nonce-value-e2e"),
            "the REAL token must reach upstream once the admitted phantom is resolved on the \
             live dispatch path; upstream received: {upstream_received_str}"
        );
        assert!(
            !upstream_received_str.contains(&phantom),
            "the phantom must NOT reach upstream unresolved — resolve_capture_request_body must \
             have substituted it; upstream received: {upstream_received_str}"
        );
    }
}

// ============================================================================
// Injection mode helpers
// ============================================================================

/// Validate phantom token based on injection mode.
///
/// Different modes extract the phantom token from different locations:
/// - `Header`/`BasicAuth`: From the auth header (Authorization, x-api-key, etc.)
/// - `UrlPath`: From the URL path pattern (e.g., `/bot<token>/getMe`)
/// - `QueryParam`: From the query parameter (e.g., `?api_key=<token>`)
fn validate_phantom_token_for_mode(
    mode: &InjectMode,
    header_bytes: &[u8],
    path: &str,
    header_name: &str,
    path_pattern: Option<&str>,
    query_param_name: Option<&str>,
    session_token: &Zeroizing<String>,
) -> Result<()> {
    match mode {
        InjectMode::Header | InjectMode::BasicAuth => {
            // Validate from header (existing behavior)
            validate_phantom_token(header_bytes, header_name, session_token)
        }
        InjectMode::UrlPath => {
            // Validate from URL path
            let pattern = path_pattern.ok_or_else(|| {
                ProxyError::HttpParse("url_path mode requires path_pattern".to_string())
            })?;
            validate_phantom_token_in_path(path, pattern, session_token)
        }
        InjectMode::QueryParam => {
            // Validate from query parameter
            let param_name = query_param_name.ok_or_else(|| {
                ProxyError::HttpParse("query_param mode requires query_param_name".to_string())
            })?;
            validate_phantom_token_in_query(path, param_name, session_token)
        }
    }
}

/// Validate phantom token embedded in URL path.
///
/// Extracts the token from the path using the pattern (e.g., `/bot{}/` matches
/// `/bot<token>/getMe` and extracts `<token>`).
fn validate_phantom_token_in_path(
    path: &str,
    pattern: &str,
    session_token: &Zeroizing<String>,
) -> Result<()> {
    // Split pattern on {} to get prefix and suffix
    let parts: Vec<&str> = pattern.split("{}").collect();
    if parts.len() != 2 {
        return Err(ProxyError::HttpParse(format!(
            "invalid path_pattern '{}': must contain exactly one {{}}",
            pattern
        )));
    }
    let (prefix, suffix) = (parts[0], parts[1]);

    // Find the token in the path
    if let Some(start) = path.find(prefix) {
        let after_prefix = start + prefix.len();

        // Handle empty suffix case (token extends to end of path or next '/' or '?')
        let end_offset = if suffix.is_empty() {
            path[after_prefix..]
                .find(['/', '?'])
                .unwrap_or(path[after_prefix..].len())
        } else {
            match path[after_prefix..].find(suffix) {
                Some(offset) => offset,
                None => {
                    warn!("Missing phantom token in URL path (pattern: {})", pattern);
                    return Err(ProxyError::InvalidToken);
                }
            }
        };

        let token = &path[after_prefix..after_prefix + end_offset];
        if token::constant_time_eq(token.as_bytes(), session_token.as_bytes()) {
            return Ok(());
        }
        warn!("Invalid phantom token in URL path");
        return Err(ProxyError::InvalidToken);
    }

    warn!("Missing phantom token in URL path (pattern: {})", pattern);
    Err(ProxyError::InvalidToken)
}

/// Validate phantom token in query parameter.
fn validate_phantom_token_in_query(
    path: &str,
    param_name: &str,
    session_token: &Zeroizing<String>,
) -> Result<()> {
    // Parse query string from path
    if let Some(query_start) = path.find('?') {
        let query = &path[query_start + 1..];
        for pair in query.split('&') {
            if let Some((name, value)) = pair.split_once('=') {
                if name == param_name {
                    // URL-decode the value
                    let decoded = urlencoding::decode(value).unwrap_or_else(|_| value.into());
                    if token::constant_time_eq(decoded.as_bytes(), session_token.as_bytes()) {
                        return Ok(());
                    }
                    warn!("Invalid phantom token in query parameter '{}'", param_name);
                    return Err(ProxyError::InvalidToken);
                }
            }
        }
    }

    warn!("Missing phantom token in query parameter '{}'", param_name);
    Err(ProxyError::InvalidToken)
}

/// Transform URL path based on injection mode.
///
/// - `UrlPath`: Replace phantom token with real credential in path
/// - `QueryParam`: Add/replace query parameter with real credential
/// - `Header`/`BasicAuth`: No path transformation needed
fn transform_path_for_mode(
    mode: &InjectMode,
    path: &str,
    path_pattern: Option<&str>,
    path_replacement: Option<&str>,
    query_param_name: Option<&str>,
    credential: &Zeroizing<String>,
) -> Result<String> {
    match mode {
        InjectMode::Header | InjectMode::BasicAuth => {
            // No path transformation needed
            Ok(path.to_string())
        }
        InjectMode::UrlPath => {
            let pattern = path_pattern.ok_or_else(|| {
                ProxyError::HttpParse("url_path mode requires path_pattern".to_string())
            })?;
            let replacement = path_replacement.unwrap_or(pattern);
            transform_url_path(path, pattern, replacement, credential)
        }
        InjectMode::QueryParam => {
            let param_name = query_param_name.ok_or_else(|| {
                ProxyError::HttpParse("query_param mode requires query_param_name".to_string())
            })?;
            transform_query_param(path, param_name, credential)
        }
    }
}

/// Transform URL path by replacing phantom token pattern with real credential.
///
/// Example: `/bot<phantom>/getMe` with pattern `/bot{}/` becomes `/bot<real>/getMe`
fn transform_url_path(
    path: &str,
    pattern: &str,
    replacement: &str,
    credential: &Zeroizing<String>,
) -> Result<String> {
    // Split pattern on {} to get prefix and suffix
    let parts: Vec<&str> = pattern.split("{}").collect();
    if parts.len() != 2 {
        return Err(ProxyError::HttpParse(format!(
            "invalid path_pattern '{}': must contain exactly one {{}}",
            pattern
        )));
    }
    let (pattern_prefix, pattern_suffix) = (parts[0], parts[1]);

    // Split replacement on {}
    let repl_parts: Vec<&str> = replacement.split("{}").collect();
    if repl_parts.len() != 2 {
        return Err(ProxyError::HttpParse(format!(
            "invalid path_replacement '{}': must contain exactly one {{}}",
            replacement
        )));
    }
    let (repl_prefix, repl_suffix) = (repl_parts[0], repl_parts[1]);

    // Find and replace the token in the path
    if let Some(start) = path.find(pattern_prefix) {
        let after_prefix = start + pattern_prefix.len();

        // Handle empty suffix case (token extends to end of path or next '/' or '?')
        let end_offset = if pattern_suffix.is_empty() {
            // Find the next path segment delimiter or end of path
            path[after_prefix..]
                .find(['/', '?'])
                .unwrap_or(path[after_prefix..].len())
        } else {
            // Find the suffix in the remaining path
            match path[after_prefix..].find(pattern_suffix) {
                Some(offset) => offset,
                None => {
                    return Err(ProxyError::HttpParse(format!(
                        "path '{}' does not match pattern '{}'",
                        path, pattern
                    )));
                }
            }
        };

        let before = &path[..start];
        let after = &path[after_prefix + end_offset + pattern_suffix.len()..];
        return Ok(format!(
            "{}{}{}{}{}",
            before,
            repl_prefix,
            credential.as_str(),
            repl_suffix,
            after
        ));
    }

    Err(ProxyError::HttpParse(format!(
        "path '{}' does not match pattern '{}'",
        path, pattern
    )))
}

/// Transform query string by adding or replacing a parameter with the credential.
fn transform_query_param(
    path: &str,
    param_name: &str,
    credential: &Zeroizing<String>,
) -> Result<String> {
    let encoded_value = urlencoding::encode(credential.as_str());

    if let Some(query_start) = path.find('?') {
        let base_path = &path[..query_start];
        let query = &path[query_start + 1..];

        // Check if parameter already exists
        let mut found = false;
        let new_query: Vec<String> = query
            .split('&')
            .map(|pair| {
                if let Some((name, _)) = pair.split_once('=') {
                    if name == param_name {
                        found = true;
                        return format!("{}={}", param_name, encoded_value);
                    }
                }
                pair.to_string()
            })
            .collect();

        if found {
            Ok(format!("{}?{}", base_path, new_query.join("&")))
        } else {
            // Append the parameter
            Ok(format!(
                "{}?{}&{}={}",
                base_path, query, param_name, encoded_value
            ))
        }
    } else {
        // No query string, add one
        Ok(format!("{}?{}={}", path, param_name, encoded_value))
    }
}

/// Inject credential into request based on mode.
///
/// For header/basic_auth modes, adds the credential header.
/// For url_path/query_param modes, the credential is already in the path.
fn inject_credential_for_mode(cred: &LoadedCredential, request: &mut Zeroizing<String>) {
    match cred.inject_mode {
        InjectMode::Header | InjectMode::BasicAuth => {
            // Inject credential header
            request.push_str(&format!(
                "{}: {}\r\n",
                cred.header_name,
                cred.header_value.as_str()
            ));
        }
        InjectMode::UrlPath | InjectMode::QueryParam => {
            // Credential is already injected into the URL path/query
            // No header injection needed
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_line() {
        let (method, path, version) = parse_request_line("POST /openai/v1/chat HTTP/1.1").unwrap();
        assert_eq!(method, "POST");
        assert_eq!(path, "/openai/v1/chat");
        assert_eq!(version, "HTTP/1.1");
    }

    #[test]
    fn test_parse_request_line_malformed() {
        assert!(parse_request_line("GET").is_err());
    }

    #[test]
    fn test_parse_service_prefix() {
        let (service, path) = parse_service_prefix("/openai/v1/chat/completions").unwrap();
        assert_eq!(service, "openai");
        assert_eq!(path, "/v1/chat/completions");
    }

    #[test]
    fn test_parse_service_prefix_no_subpath() {
        let (service, path) = parse_service_prefix("/anthropic").unwrap();
        assert_eq!(service, "anthropic");
        assert_eq!(path, "/");
    }

    #[test]
    fn test_validate_phantom_token_bearer_valid() {
        let token = Zeroizing::new("secret123".to_string());
        let header = b"Authorization: Bearer secret123\r\nContent-Type: application/json\r\n\r\n";
        assert!(validate_phantom_token(header, "Authorization", &token).is_ok());
    }

    #[test]
    fn test_validate_phantom_token_bearer_invalid() {
        let token = Zeroizing::new("secret123".to_string());
        let header = b"Authorization: Bearer wrong\r\n\r\n";
        assert!(validate_phantom_token(header, "Authorization", &token).is_err());
    }

    #[test]
    fn test_validate_phantom_token_x_api_key_valid() {
        let token = Zeroizing::new("secret123".to_string());
        let header = b"x-api-key: secret123\r\nContent-Type: application/json\r\n\r\n";
        assert!(validate_phantom_token(header, "x-api-key", &token).is_ok());
    }

    #[test]
    fn test_validate_phantom_token_x_goog_api_key_valid() {
        let token = Zeroizing::new("secret123".to_string());
        let header = b"x-goog-api-key: secret123\r\nContent-Type: application/json\r\n\r\n";
        assert!(validate_phantom_token(header, "x-goog-api-key", &token).is_ok());
    }

    #[test]
    fn test_validate_phantom_token_missing() {
        let token = Zeroizing::new("secret123".to_string());
        let header = b"Content-Type: application/json\r\n\r\n";
        assert!(validate_phantom_token(header, "Authorization", &token).is_err());
    }

    #[test]
    fn test_validate_phantom_token_case_insensitive_header() {
        let token = Zeroizing::new("secret123".to_string());
        let header = b"AUTHORIZATION: Bearer secret123\r\n\r\n";
        assert!(validate_phantom_token(header, "Authorization", &token).is_ok());
    }

    #[test]
    fn test_filter_headers_removes_host_auth() {
        let header = b"Host: localhost:8080\r\nAuthorization: Bearer old\r\nContent-Type: application/json\r\nAccept: */*\r\n\r\n";
        let filtered = filter_headers(header, "Authorization");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].0, "Content-Type");
        assert_eq!(filtered[1].0, "Accept");
    }

    #[test]
    fn test_filter_headers_removes_x_api_key() {
        let header = b"x-api-key: sk-old\r\nContent-Type: application/json\r\n\r\n";
        let filtered = filter_headers(header, "x-api-key");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "Content-Type");
    }

    #[test]
    fn test_filter_headers_removes_custom_header() {
        let header = b"PRIVATE-TOKEN: phantom123\r\nContent-Type: application/json\r\n\r\n";
        let filtered = filter_headers(header, "PRIVATE-TOKEN");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "Content-Type");
    }

    #[test]
    fn test_extract_content_length() {
        let header = b"Content-Type: application/json\r\nContent-Length: 42\r\n\r\n";
        assert_eq!(extract_content_length(header), Some(42));
    }

    #[test]
    fn test_extract_content_length_missing() {
        let header = b"Content-Type: application/json\r\n\r\n";
        assert_eq!(extract_content_length(header), None);
    }

    #[test]
    fn test_parse_upstream_url_https() {
        let (host, port, path) =
            parse_upstream_url("https://api.openai.com/v1/chat/completions").unwrap();
        assert_eq!(host, "api.openai.com");
        assert_eq!(port, 443);
        assert_eq!(path, "/v1/chat/completions");
    }

    #[test]
    fn test_parse_upstream_url_http_with_port() {
        let (host, port, path) = parse_upstream_url("http://localhost:8080/api").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
        assert_eq!(path, "/api");
    }

    #[test]
    fn test_parse_upstream_url_no_path() {
        let (host, port, path) = parse_upstream_url("https://api.anthropic.com").unwrap();
        assert_eq!(host, "api.anthropic.com");
        assert_eq!(port, 443);
        assert_eq!(path, "/");
    }

    #[test]
    fn test_parse_upstream_url_invalid_scheme() {
        assert!(parse_upstream_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_parse_response_status_200() {
        let data = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n";
        assert_eq!(parse_response_status(data), 200);
    }

    #[test]
    fn test_parse_response_status_404() {
        let data = b"HTTP/1.1 404 Not Found\r\n\r\n";
        assert_eq!(parse_response_status(data), 404);
    }

    #[test]
    fn test_parse_response_status_garbage() {
        let data = b"not an http response";
        assert_eq!(parse_response_status(data), 502);
    }

    #[test]
    fn test_parse_response_status_empty() {
        assert_eq!(parse_response_status(b""), 502);
    }

    #[test]
    fn test_parse_response_status_partial() {
        let data = b"HTTP/1.1 ";
        assert_eq!(parse_response_status(data), 502);
    }

    // ============================================================================
    // URL Path Injection Mode Tests
    // ============================================================================

    #[test]
    fn test_validate_phantom_token_in_path_valid() {
        let token = Zeroizing::new("session123".to_string());
        let path = "/bot/session123/getMe";
        let pattern = "/bot/{}/";
        assert!(validate_phantom_token_in_path(path, pattern, &token).is_ok());
    }

    #[test]
    fn test_validate_phantom_token_in_path_invalid() {
        let token = Zeroizing::new("session123".to_string());
        let path = "/bot/wrong_token/getMe";
        let pattern = "/bot/{}/";
        assert!(validate_phantom_token_in_path(path, pattern, &token).is_err());
    }

    #[test]
    fn test_validate_phantom_token_in_path_missing() {
        let token = Zeroizing::new("session123".to_string());
        let path = "/api/getMe";
        let pattern = "/bot/{}/";
        assert!(validate_phantom_token_in_path(path, pattern, &token).is_err());
    }

    #[test]
    fn test_transform_url_path_basic() {
        let credential = Zeroizing::new("real_token".to_string());
        let path = "/bot/phantom_token/getMe";
        let pattern = "/bot/{}/";
        let replacement = "/bot/{}/";
        let result = transform_url_path(path, pattern, replacement, &credential).unwrap();
        assert_eq!(result, "/bot/real_token/getMe");
    }

    #[test]
    fn test_transform_url_path_different_replacement() {
        let credential = Zeroizing::new("real_token".to_string());
        let path = "/api/v1/phantom_token/chat";
        let pattern = "/api/v1/{}/";
        let replacement = "/v2/bot/{}/";
        let result = transform_url_path(path, pattern, replacement, &credential).unwrap();
        assert_eq!(result, "/v2/bot/real_token/chat");
    }

    #[test]
    fn test_transform_url_path_no_trailing_slash() {
        let credential = Zeroizing::new("real_token".to_string());
        let path = "/bot/phantom_token";
        let pattern = "/bot/{}";
        let replacement = "/bot/{}";
        let result = transform_url_path(path, pattern, replacement, &credential).unwrap();
        assert_eq!(result, "/bot/real_token");
    }

    // ============================================================================
    // Query Param Injection Mode Tests
    // ============================================================================

    #[test]
    fn test_validate_phantom_token_in_query_valid() {
        let token = Zeroizing::new("session123".to_string());
        let path = "/api/data?api_key=session123&other=value";
        assert!(validate_phantom_token_in_query(path, "api_key", &token).is_ok());
    }

    #[test]
    fn test_validate_phantom_token_in_query_invalid() {
        let token = Zeroizing::new("session123".to_string());
        let path = "/api/data?api_key=wrong_token";
        assert!(validate_phantom_token_in_query(path, "api_key", &token).is_err());
    }

    #[test]
    fn test_validate_phantom_token_in_query_missing_param() {
        let token = Zeroizing::new("session123".to_string());
        let path = "/api/data?other=value";
        assert!(validate_phantom_token_in_query(path, "api_key", &token).is_err());
    }

    #[test]
    fn test_validate_phantom_token_in_query_no_query_string() {
        let token = Zeroizing::new("session123".to_string());
        let path = "/api/data";
        assert!(validate_phantom_token_in_query(path, "api_key", &token).is_err());
    }

    #[test]
    fn test_validate_phantom_token_in_query_url_encoded() {
        let token = Zeroizing::new("token with spaces".to_string());
        let path = "/api/data?api_key=token%20with%20spaces";
        assert!(validate_phantom_token_in_query(path, "api_key", &token).is_ok());
    }

    #[test]
    fn test_transform_query_param_add_to_no_query() {
        let credential = Zeroizing::new("real_key".to_string());
        let path = "/api/data";
        let result = transform_query_param(path, "api_key", &credential).unwrap();
        assert_eq!(result, "/api/data?api_key=real_key");
    }

    #[test]
    fn test_transform_query_param_add_to_existing_query() {
        let credential = Zeroizing::new("real_key".to_string());
        let path = "/api/data?other=value";
        let result = transform_query_param(path, "api_key", &credential).unwrap();
        assert_eq!(result, "/api/data?other=value&api_key=real_key");
    }

    #[test]
    fn test_transform_query_param_replace_existing() {
        let credential = Zeroizing::new("real_key".to_string());
        let path = "/api/data?api_key=phantom&other=value";
        let result = transform_query_param(path, "api_key", &credential).unwrap();
        assert_eq!(result, "/api/data?api_key=real_key&other=value");
    }

    #[test]
    fn test_transform_query_param_url_encodes_special_chars() {
        let credential = Zeroizing::new("key with spaces".to_string());
        let path = "/api/data";
        let result = transform_query_param(path, "api_key", &credential).unwrap();
        assert_eq!(result, "/api/data?api_key=key%20with%20spaces");
    }

    // ============================================================================
    // D-09 / #1077 — 403 + EndpointPolicy audit equivalence test
    // ============================================================================
    //
    // Async harness imports copied from connect.rs tests (lines 276-285).
    use nono::undo::{NetworkAuditDecision, NetworkAuditDenialCategory};
    use tokio::io::AsyncReadExt;

    /// Drain a reader to end-of-stream and return the bytes as a String.
    async fn read_to_string<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> String {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// D-09 / #1077 403+audit equivalence test.
    ///
    /// Drives `handle_reverse_proxy` with a route that allows only `GET /v1/models`
    /// via endpoint_rules (default-deny). The request asks for `GET /forbidden`, which
    /// the endpoint rules deny. Asserts:
    ///   1. The client receives `HTTP/1.1 403 Forbidden`.
    ///   2. `audit::drain_audit_events` yields one event with
    ///      `decision == NetworkAuditDecision::Deny` and
    ///      `denial_category == Some(NetworkAuditDenialCategory::EndpointPolicy)`.
    ///
    /// The 403 is sent BEFORE any credential operation (reverse.rs:96-116), which IS
    /// #1077's intent. Cherry-pick of a5d623fd skipped (equivalence confirmed); non-test
    /// code is unchanged.
    #[tokio::test]
    async fn denied_endpoint_returns_403_and_audit() {
        use crate::config::{EndpointRule, RouteConfig};
        use std::sync::Arc;
        use tokio::net::TcpListener;

        // Build a RouteStore with one route: service prefix "testservice", allowing
        // only GET /v1/models. Any other path is endpoint-denied.
        let routes = vec![RouteConfig {
            spiffe: None,
            prefix: "testservice".to_string(),
            upstream: "https://example.invalid".to_string(),
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
            capture: None,
            endpoint_policy: None,
        }];

        let route_store = RouteStore::load(&routes).await.unwrap();
        let credential_store = CredentialStore::empty();
        let session_token = Zeroizing::new("test-session-token".to_string());
        let filter = ProxyFilter::allow_all();

        // Build a minimal TLS connector. It is required by ReverseProxyCtx but will
        // NOT be exercised because the endpoint-deny path returns before any upstream
        // TLS connection is attempted.
        let root_store = crate::route::build_base_root_store();
        let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();
        let tls_config_arc = Arc::new(tls_config);
        let tls_connector = tokio_rustls::TlsConnector::from(Arc::clone(&tls_config_arc));
        let upstream_pool = crate::pool::UpstreamPool::new(Arc::clone(&tls_config_arc), false);

        let audit_log = audit::new_audit_log();
        let capture_store = crate::capture::CapturePhantomStore::new();

        let ctx = ReverseProxyCtx {
            route_store: &route_store,
            credential_store: &credential_store,
            session_token: &session_token,
            filter: &filter,
            tls_connector: &tls_connector,
            default_tls_config: &tls_config_arc,
            upstream_pool: &upstream_pool,
            audit_log: Some(&audit_log),
            capture_store: &capture_store,
            require_auth: true,
        };

        // Bind a loopback listener so we have a real TcpStream pair (handle_reverse_proxy
        // requires &mut TcpStream, not a generic AsyncWrite).
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (connect_result, accept_result) =
            tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept());
        let mut server_stream = accept_result.unwrap().0;
        let mut client_conn = connect_result.unwrap();

        // Request: GET /testservice/forbidden — endpoint rules deny /forbidden on this route.
        let first_line = "GET /testservice/forbidden HTTP/1.1";
        let remaining_header = b"Host: example.invalid\r\n\r\n";

        let _result =
            handle_reverse_proxy(first_line, &mut server_stream, remaining_header, &ctx, &[]).await;

        // Drop the server side so the client reader can drain to EOF.
        drop(server_stream);

        // Read what was written to the "client" side.
        let response = read_to_string(&mut client_conn).await;

        // ASSERTION 1 (core equivalence): 403 Forbidden returned.
        assert!(
            response.starts_with("HTTP/1.1 403"),
            "D-09: expected HTTP/1.1 403 response; got: {:?}",
            response
        );

        // ASSERTION 2: audit event carries Deny + EndpointPolicy.
        let events = audit::drain_audit_events(&audit_log);
        assert!(
            !events.is_empty(),
            "D-09: expected at least one audit event; got none"
        );
        let event = &events[0];
        assert_eq!(
            event.decision,
            NetworkAuditDecision::Deny,
            "D-09: expected Deny decision; got: {:?}",
            event.decision
        );
        assert_eq!(
            event.denial_category,
            Some(NetworkAuditDenialCategory::EndpointPolicy),
            "D-09: expected EndpointPolicy denial_category; got: {:?}",
            event.denial_category
        );
    }

    // ============================================================================
    // Plan 113-06 — SPIFFE dispatch: auth-gate denial before credential acquisition
    // ============================================================================
    //
    // `SpiffeJwtSource`/`SpiffeAssertionTokenCache` have no test-only constructor
    // bypassing a live SPIRE Workload API connection (documented constraint,
    // Plans 113-03/113-04/113-05), so a full end-to-end proof of a SUCCESSFUL
    // SPIFFE-authenticated forward is deferred to Plan 113-07's SPIRE-gated
    // `spiffe_integration.rs` lane. This test instead proves the narrower,
    // fully-unit-testable half of T-113-15/T-113-17: `handle_spiffe_route`'s
    // auth gate runs and denies BEFORE any `managed_auth` access is attempted.
    //
    // `RouteStore::from_loaded_routes` (Plan 113-05, `#[cfg(test)] pub(crate)`)
    // lets this test build a `declares_spiffe: true` route with `managed_auth:
    // None` — an invariant intentionally NOT upheld here. If the auth gate were
    // ever bypassed, the request would reach the `managed_auth: None` fail-closed
    // branch and still be denied, but with 503, not 407 — the exact status code
    // returned is what proves WHICH guard fired.

    /// A `LoadedRoute` with `declares_spiffe: true` and `managed_auth: None`
    /// dispatches to `handle_spiffe_route` via `has_spiffe_source()`. An
    /// invalid/missing session token is denied 407 by the auth gate — the same
    /// `validate_proxy_auth` call the no-credential branch already uses — before
    /// `route.managed_auth` is ever read.
    #[tokio::test]
    async fn spiffe_route_denies_missing_session_token_before_credential_acquisition() {
        use crate::config::{CompiledEndpointPolicy, CompiledEndpointRules};
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::net::TcpListener;

        let loaded_route = LoadedRoute {
            upstream: "https://example.invalid".to_string(),
            upstream_host_port: Some("example.invalid:443".to_string()),
            endpoint_rules: CompiledEndpointRules::compile(&[]).unwrap(),
            endpoint_policy: CompiledEndpointPolicy::compile(None, &[]).unwrap(),
            tls_connector: None,
            tls_client_config: None,
            tls_config_key: None,
            managed_auth: None,
            declares_spiffe: true,
            declares_capture: false,
            capture: None,
        };
        let mut routes = HashMap::new();
        routes.insert("spiffesvc".to_string(), loaded_route);
        let route_store = RouteStore::from_loaded_routes(routes);

        let credential_store = CredentialStore::empty();
        let session_token = Zeroizing::new("correct-session-token".to_string());
        let filter = ProxyFilter::allow_all();

        let root_store = crate::route::build_base_root_store();
        let tls_config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();
        let tls_config_arc = Arc::new(tls_config);
        let tls_connector = tokio_rustls::TlsConnector::from(Arc::clone(&tls_config_arc));
        let upstream_pool = crate::pool::UpstreamPool::new(Arc::clone(&tls_config_arc), false);

        let audit_log = audit::new_audit_log();
        let capture_store = crate::capture::CapturePhantomStore::new();

        let ctx = ReverseProxyCtx {
            route_store: &route_store,
            credential_store: &credential_store,
            session_token: &session_token,
            filter: &filter,
            tls_connector: &tls_connector,
            default_tls_config: &tls_config_arc,
            upstream_pool: &upstream_pool,
            audit_log: Some(&audit_log),
            capture_store: &capture_store,
            require_auth: true,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (connect_result, accept_result) =
            tokio::join!(tokio::net::TcpStream::connect(addr), listener.accept());
        let mut server_stream = accept_result.unwrap().0;
        let mut client_conn = connect_result.unwrap();

        // No Proxy-Authorization header at all -> validate_proxy_auth fails.
        let first_line = "GET /spiffesvc/v1/models HTTP/1.1";
        let remaining_header = b"Host: example.invalid\r\n\r\n";

        let _result =
            handle_reverse_proxy(first_line, &mut server_stream, remaining_header, &ctx, &[]).await;

        drop(server_stream);
        let response = read_to_string(&mut client_conn).await;

        assert!(
            response.starts_with("HTTP/1.1 407"),
            "expected 407 Proxy Authentication Required from the SPIFFE route's auth gate \
             (proves the gate ran before managed_auth was ever accessed — a bypass or an \
             invariant-violation fail-closed would both have produced a DIFFERENT status); \
             got: {:?}",
            response
        );

        let events = audit::drain_audit_events(&audit_log);
        assert!(
            !events.is_empty(),
            "expected at least one audit event; got none"
        );
        let event = &events[0];
        assert_eq!(
            event.decision,
            NetworkAuditDecision::Deny,
            "expected Deny decision; got: {:?}",
            event.decision
        );
        assert_eq!(
            event.denial_category,
            Some(NetworkAuditDenialCategory::AuthenticationFailed),
            "expected AuthenticationFailed denial_category (auth gate, not credential \
             acquisition); got: {:?}",
            event.denial_category
        );
    }
}
