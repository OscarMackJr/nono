# Phase 113: SPIFFE/SPIRE Workload Identity - Pattern Map

**Mapped:** 2026-08-06
**Files analyzed:** 20 (new + modified, per 113-CONTEXT.md/113-RESEARCH.md's Symbol-Level Disposition Table)
**Analogs found:** 20 / 20 (all files have a concrete fork analog; three require build-from-scratch
per the RESEARCH "Decision Conflicts" section rather than a literal port — flagged below)

**IMPORTANT — this document is complementary to 113-RESEARCH.md's Symbol-Level Disposition Table,
not a replacement for it.** The disposition table (ADOPT/ADAPT/DROP verdict + confidence) is
authoritative for *whether/how much* to absorb each file. This document answers the separate
question: *which existing fork file should the planner open side-by-side while writing the new
code, and what exact lines does it copy the shape from.*

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono-proxy/src/spiffe.rs` (new) | service (external-auth-source client) | request-response (fetch-on-demand, cached) | `crates/nono-proxy/src/oauth2.rs` (`TokenCache`) | exact — same shape: wrap external identity source, cache, `Zeroizing<String>` secret out |
| `crates/nono-proxy/src/auth.rs` (new) | model (enum-dispatch "mechanism" type) | n/a (pure dispatch) | `crates/nono-proxy/src/config.rs` (`InjectMode` enum) | role-match — small serde enum + match-dispatch idiom |
| `crates/nono-proxy/src/reverse.rs` (`handle_spiffe_route`, `handle_spiffe_assertion_credential`) | controller (request handler) | request-response, credential-injection | `crates/nono-proxy/src/reverse.rs::handle_reverse_proxy` (the `static_cred` un-diffed portion, same file) | exact — same function, extend in place; DO NOT port upstream's `forward::forward_request` call shape |
| `crates/nono-proxy/src/credential.rs` (`CredentialStore.spiffe_assertion_routes`, `get_spiffe_assertion()`) | model/store | CRUD (load-once, read-many) | `crates/nono-proxy/src/credential.rs` (`aws_routes: HashMap<String,()>` + `get_aws()`) | role-match — same file's existing sibling placeholder-map pattern, not upstream's absent `oauth2_routes`/`OAuth2Route` |
| `crates/nono-proxy/src/route.rs` (`LoadedRoute.managed_auth`, `has_spiffe_source()`) | model | CRUD | `crates/nono-proxy/src/route.rs::LoadedRoute` (existing `tls_connector: Option<...>` field + `Debug` impl) | exact — same struct, additive field |
| `crates/nono-proxy/src/route.rs` (`RouteStore::load` → async) | service | request-response (startup) | `crates/nono-proxy/src/route.rs::RouteStore::load` (same fn, sync today) | exact — signature change only |
| `crates/nono-proxy/src/server.rs` (`bound_port`, comment rewrite, call-site `.await`) | controller/service (startup + dispatch) | request-response | `crates/nono-proxy/src/server.rs::start()` (same fn) | exact |
| `crates/nono-proxy/src/audit.rs` (`EventContext.spiffe_context`) | model (observability) | event-driven | `crates/nono-proxy/src/audit.rs::EventContext` (same struct) | exact — additive `Option` field under existing `Default` derive |
| `crates/nono-proxy/src/config.rs` (`RouteConfig.spiffe`, `SpiffeAuthConfig`, `ClientAssertionConfig`, `OAuth2Config.client_assertion`/`extra_params`) | model (schema types) | CRUD | `crates/nono-proxy/src/config.rs::OAuth2Config`/`RouteConfig` (same file) | exact |
| `crates/nono-proxy/src/server.rs`/`connect.rs`/`external.rs` (D-03 fail-closed guard, NEW code, no upstream equivalent) | middleware (auth gate) | request-response | `crates/nono-proxy/src/server.rs::handle_forward_http`'s WR-13 `require_auth` gate (112-REVIEW.md fix) | exact — same *shape* of guard (route-level boolean → 403/503 + audit before forward), new orthogonal condition |
| `crates/nono/src/undo/types.rs` (`SpiffeAuditContext`, `SpiffeDelegationContext`, enum variants) | model (audit vocabulary) | event-driven | `crates/nono/src/undo/types.rs` (existing `NetworkAuditEvent`/`NetworkAuditAuthMechanism`) | exact — pure additive data types |
| `crates/nono/src/audit.rs` (`spiffe_context: None,` fixtures) | test fixture | n/a | same file, existing fixture literals | exact |
| `crates/nono-cli/src/audit_integrity.rs`, `audit_ledger.rs`, `exec_strategy/supervisor_linux.rs` (`spiffe_context: None,`) | test fixture | n/a | same files, existing fixture literals | exact |
| `crates/nono-cli/src/network_policy.rs` (`RouteConfig.spiffe` thread-through) | service | transform | same file, existing `oauth2`/`aws_auth` thread-through lines | exact |
| `crates/nono-cli/src/profile/mod.rs` (`CustomCredentialDef.spiffe`, `validate_custom_credential()` extension) | model + validation | CRUD | `crates/nono-cli/src/profile/mod.rs::CustomCredentialDef`/`validate_custom_credential` (same file, existing `aws_auth` mutual-exclusion branch) | exact |
| `crates/nono-cli/data/nono-profile.schema.json` (`spiffe` property, `$defs/SpiffeAuthConfig`, `$defs/ClientAssertionConfig`, `$defs/OAuth2Config` fix) | config (JSON Schema) | n/a | same file, existing `aws_auth`/`$defs/AwsAuthConfig`/`$defs/OAuth2Config` blocks | exact |
| `crates/nono-cli/src/proxy_runtime.rs` (`enforce_spiffe_socket_isolation()`) | service (sandbox policy) | request-response (startup) | same file, existing `add_platform_rule`/`unix_socket_capabilities` call sites | role-match |
| `crates/nono-proxy/tests/spiffe_integration.rs` (new; new `nono-proxy/tests/` dir) | test | integration | `crates/nono-cli/tests/wfp_port_integration.rs`-style env-var-gated integration test (see below) | role-match — `nono-proxy` currently has **no `tests/` directory at all**; this is the first |
| `crates/nono-cli/tests/spiffe_run.rs` (new) | test | integration (e2e binary) | `crates/nono-cli/tests/socket_access_run.rs` | exact for scaffolding shape; **anti-pattern** for skip reporting (see Landmines) |
| `.github/workflows/spire.yml` (new) | config (CI) | n/a | `.github/workflows/ci.yml` | role-match — copy runner/permissions/checkout shape, not job content |
| `../nono-py/src/proxy.rs:206-218` (`RustRouteConfig` literal) | model (FFI wrapper) | CRUD | same file, same literal (exhaustive struct, 4th consecutive break site) | exact |

## Pattern Assignments

### 1. `crates/nono-proxy/src/spiffe.rs` (new) — analog `crates/nono-proxy/src/oauth2.rs`

**Analog:** `crates/nono-proxy/src/oauth2.rs` — this is the fork's only existing "wrap an external
identity/token source, cache it, hand back a `Zeroizing<String>` secret" module, and its shape is
what `SpiffeJwtSource` should mirror (constructor that fails closed via `ProxyError::Config`,
`Zeroizing` everywhere a secret crosses a function boundary, redacted custom `Debug`).

**Module doc-comment convention** (`oauth2.rs:1-17`):
```rust
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
```

**Imports pattern** (`oauth2.rs:19-27`):
```rust
use crate::error::{ProxyError, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_rustls::TlsConnector;
use tracing::{debug, warn};
use zeroize::Zeroizing;
```
For `spiffe.rs`, swap the raw-HTTP/TLS imports for `spiffe_workload::{JwtSource, SpiffeId}` — the
rest of the import shape (crate-relative `error::{ProxyError, Result}`, `zeroize::Zeroizing`,
`tracing::{debug, warn}`) carries over unchanged.

**Secret-wrapping / redacted-Debug idiom** (`oauth2.rs:46-90`, the exact idiom `SpiffeJwtSource`
must reuse for its cached token and any config struct with a secret field):
```rust
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

pub struct TokenCache {
    token: Arc<RwLock<CachedToken>>,
    config: OAuth2ExchangeConfig,
    tls_connector: TlsConnector,
}

struct CachedToken {
    access_token: Zeroizing<String>,
    expires_at: Instant,
}
```

**Fail-closed constructor / error-mapping idiom** (`oauth2.rs:96-141`, the exact `ProxyError::Config`
mapping `SpiffeJwtSource::connect()` should reuse for an unreachable Workload API socket):
```rust
/// Create a new cache and perform the **initial** token exchange.
/// ...
/// # Errors
///
/// Returns [`ProxyError::Config`] if no tokio runtime context is
/// available (caller invoked outside `Runtime::block_on` /
/// `Runtime::enter` scope). Returns [`ProxyError::OAuth2Exchange`] if
/// the initial exchange fails (DNS, TCP, TLS, non-200, malformed JSON).
/// The calling code skips the route so the proxy can still start for
/// other routes.
pub fn new(config: OAuth2ExchangeConfig, tls_connector: TlsConnector) -> Result<Self> {
    let runtime_handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ProxyError::Config(format!(
            // ... contributor-friendly diagnostic, not the raw panic message ...
        ))
    })?;
    // ...
}
```
`spiffe.rs::SpiffeJwtSource::connect` should map `spiffe_workload::JwtSource` connect failures to
`ProxyError::Config` the same way — this is also the literal grep target for D-03/D-04's
fail-closed-at-startup claim ("returns `ProxyError::Config` on failure").

**Error-variant to add:** none needed beyond the existing `ProxyError::Config`/`ProxyError::Credential`
(see `crates/nono-proxy/src/error.rs:33-34`, excerpted in full below under Shared Patterns) — do
not add a new `SpiffeError` variant family; the fork's `ProxyError` enum is the single error type
for this crate and every other new-module absorb in this fork (`oauth2.rs`, `config.rs`) reuses it.

**Test-module convention:** `oauth2.rs` has no `#[cfg(test)] mod tests` visible in the excerpted
range but the crate-wide convention (seen in `route.rs`, `reverse.rs`, `server.rs`) is a trailing
`#[cfg(test)] mod tests { #![allow(clippy::unwrap_used)] use super::*; ... }` block in the same
file — mirror that placement for `spiffe.rs`'s unit tests (`check_nbf`, etc.) rather than a
separate test file (only `nono-proxy/tests/` integration tests are separate files).

---

### 2. `crates/nono-proxy/src/auth.rs` (new) — analog `crates/nono-proxy/src/config.rs::InjectMode`

**Analog:** `crates/nono-proxy/src/config.rs::InjectMode` — the fork's only existing small,
serde-tagged, match-dispatched "mechanism" enum. `ManagedUpstreamAuth` (from `auth.rs`) should
follow the same shape: `#[derive(Debug, Clone, ...)]`, `#[serde(rename_all = "snake_case")]` where
applicable, and a match-dispatch consumer in the handler file (`reverse.rs`), not inline `if`
chains.

**Enum definition pattern** (`config.rs:10-23`):
```rust
/// Credential injection mode determining how credentials are inserted into requests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectMode {
    /// Inject credential into an HTTP header (default)
    #[default]
    Header,
    /// Replace a pattern in the URL path with the credential
    UrlPath,
    /// Add or replace a query parameter with the credential
    QueryParam,
    /// Use HTTP Basic Authentication (credential format: "username:password")
    BasicAuth,
}
```

**Match-dispatch consumer pattern** (`reverse.rs:1034-1041`, the exact shape `reverse.rs`'s new
SPIFFE dispatch branch should mirror when it matches on `ManagedUpstreamAuth`):
```rust
match cred.inject_mode {
    InjectMode::Header | InjectMode::BasicAuth => {
        // Inject credential header
        request.push_str(&format!(
            "{}: {}\r\n",
            cred.header_name,
            cred.header_value.as_str()
        ));
    }
    // ...
}
```
Three call sites in `reverse.rs` (`:784`, `:897`, `:1034`) all match on `cred.inject_mode` this
way — this is the idiom to extend for `ManagedUpstreamAuth::SpiffeJwt`, not a new `if let`/`match`
style.

**RESEARCH's own quoted target shape** (113-RESEARCH.md, "Code Examples" section, ADOPT verbatim):
```rust
// Source: c831dade, crates/nono-proxy/src/auth.rs (new file, ADOPT verbatim)
pub enum ManagedUpstreamAuth {
    /// JWT-SVID fetched from the SPIRE Workload API and injected as a bearer token.
    SpiffeJwt(Arc<crate::spiffe::SpiffeJwtSource>),
}
```
Preserve the upstream doc-comment convention "*Adding a new auth mechanism: add a variant here and
a corresponding handler branch in reverse.rs*" — it correctly describes the SPIFFE-only scope this
phase keeps (do not build the general `client_credentials` variant per the operator's D-01/scope
decision).

---

### 3. `handle_spiffe_route` rewrite target — analog: the un-diffed `static_cred` portion of
`reverse.rs::handle_reverse_proxy` (same file)

**This is the single most valuable excerpt in the document, per the orchestrator's scope notes.**
RESEARCH's Decision Conflicts section is explicit: `handle_spiffe_route` as upstream diffed it
calls `crate::forward::{UpstreamSpec, UpstreamStrategy, AuditCtx, forward_request}` — a module
that **does not exist in this fork at all**. The correct pattern is not upstream's diff; it is the
fork's own existing `handle_reverse_proxy`, which already does every step SPIFFE needs (auth gate →
credential lookup → path transform → `parse_upstream_url` → filter check → `connect_upstream_tls` →
inject → forward → stream response → audit) using the fork's own primitives.

**Full flow to copy, `reverse.rs:203-467`** (the exact sequence `handle_spiffe_route` should
follow, substituting `cred`/static-credential steps for `route.managed_auth`/SPIFFE steps):

```rust
// 1. Auth gate (reverse.rs:207-275) — session-token / phantom-token check,
//    gated on ctx.require_auth (Phase 112 SEC-07). SPIFFE routes reuse
//    validate_reverse_local_auth() here per D-01's evidence text.
if ctx.require_auth {
    if let Some(cred) = cred {
        if let Err(e) = validate_phantom_token_for_mode(/* ... */) {
            audit::log_denied(/* ... */);
            send_error(stream, 401, "Unauthorized").await?;
            return Ok(());
        }
    } else {
        if let Err(e) = token::validate_proxy_auth(remaining_header, ctx.session_token) {
            audit::log_denied(/* ... */);
            send_error(stream, 407, "Proxy Authentication Required").await?;
            return Ok(());
        }
    }
}

// 2. Path transform (reverse.rs:289-300) — SPIFFE routes have no
//    url_path/query_param transform, so this collapses to the else-branch
//    (upstream_path.clone()) for handle_spiffe_route specifically.

// 3. Build + parse upstream URL — the load-bearing fork-only primitives:
let upstream_url = format!("{}{}", route.upstream.trim_end_matches('/'), transformed_path);
let (upstream_host, upstream_port, upstream_path_full) = parse_upstream_url(&upstream_url)?;
//   fn parse_upstream_url(url_str: &str) -> Result<(String, u16, String)>  (reverse.rs:599)
//   — 3-tuple, NO scheme in the return type. This is what forward_request/
//   UpstreamSpec would have done upstream; here it's this one call.

// 4. Filter/DNS-rebinding check (reverse.rs:313-332)
let check = ctx.filter.check_host(&upstream_host, upstream_port).await?;
if !check.result.is_allowed() { /* 403 + audit::log_denied */ }

// 5. Connect (reverse.rs:366-398) — always-TLS, no scheme branch:
let connector = route.tls_connector.as_ref().unwrap_or(ctx.tls_connector);
let upstream_result = connect_upstream_tls(&upstream_host, upstream_port, &check.resolved_addrs, connector).await;
//   async fn connect_upstream_tls(...) -> Result<TlsStream<TcpStream>>  (reverse.rs:644)
//   — always negotiates TLS; there is no HTTP-vs-HTTPS scheme branch in this
//   fork's upstream connector at all (unlike upstream's UpstreamStrategy).

// 6. Build Zeroizing request buffer + inject + forward + stream response
//    (reverse.rs:400-458) — identical mechanics for a SPIFFE bearer token:
let mut request = Zeroizing::new(format!("{} {} {}\r\nHost: {}\r\n", method, upstream_path_full, version, upstream_host));
// inject_credential_for_mode(cred, &mut request);  ->  becomes the SPIFFE
// bearer-header injection using SpiffeJwtSource::fetch_token()'s Zeroizing<String>

// 7. Audit (reverse.rs:460-466) — KEEP calling audit::log_reverse_proxy here
//    (see Landmine 3 below — do NOT delete it per upstream's diff).
audit::log_reverse_proxy(ctx.audit_log, &service, &method, &upstream_path, status_code);
```

**Do NOT** construct an `UpstreamSpec`/`UpstreamStrategy::Direct` or call `forward::forward_request`
— rewrite against `parse_upstream_url()` + `connect_upstream_tls()` as shown. `validate_reverse_local_auth`
(D-01's cited "local session-token validation") is the one piece of upstream's diff that DOES port
cleanly standalone — it only calls `token::validate_proxy_auth`/`validate_phantom_token_for_mode`,
both confirmed present at the exact call sites excerpted above.

---

### 4. D-03's fail-closed guard — request-entry paths and their existing auth-gate excerpts

**Pattern to mirror:** `crates/nono-cli`'s `validate_block_net_conflicts`-style fail-closed-guard
idiom (a startup/request-time hard check that returns an error rather than degrading) combined with
Phase 112's `require_auth`/`strict_connect_auth` per-path boolean gate, composed with (not
duplicating) the new D-03 route-level check.

Every request-entry path in `crates/nono-proxy/src/` that can reach a route, with its existing
auth-gate excerpt (D-03's new `route.has_spiffe_source()` guard must be added at each, after route
resolution, before forwarding):

| Symbol | File:Line | Existing auth-gate excerpt |
|---|---|---|
| `handle_reverse_proxy` | `reverse.rs:215` | `if ctx.require_auth { ... }` (full excerpt in Pattern 3 above) — **HAS its own SPIFFE dispatch after this** (item 3), so D-03 does not apply here; included for completeness. |
| `handle_forward_http` | `server.rs:941-980` | See exact WR-13 excerpt below — the fixed shape D-03's new guard should copy verbatim in structure. |
| `handle_connect` (via `server.rs` dispatch, both arms) | `server.rs:1296-1316` | See excerpt below — dispatch passes `state.config.require_auth && state.config.strict_connect_auth` into `connect::handle_connect`. |
| `handle_external_proxy` | `external.rs:112-126` | See excerpt below. |
| bypass-route direct dispatch | `server.rs:1286-1295` | See excerpt below — a THIRD, distinct auth call site inline in the dispatcher itself, not inside a handler fn. |

**`handle_forward_http`'s WR-13 fixed shape** (`server.rs:948-966`, the exact precedent D-03 must
mirror — this is 112-REVIEW.md's WR-13 finding, already fixed once):
```rust
// 1. Proxy-Authorization gate — identical to the reverse-proxy
//    no-credential branch: 407 on missing/invalid auth, with an audit
//    denial recording the authentication failure. No auth bypass.
//
//    WR-13: gated on `require_auth`, matching every other request path in
//    `handle_connection` (external chain, external bypass, both CONNECT
//    arms, reverse proxy) ...
if state.config.require_auth {
    if let Err(e) = token::validate_proxy_auth(header_bytes, &state.session_token) {
        let (host, port) = parse_non_connect_target(first_line).unwrap_or_else(|_| ("unknown".to_string(), 0));
        audit::log_denied(
            Some(&state.audit_log),
            audit::ProxyMode::Reverse,
            &audit::EventContext {
                auth_mechanism: Some(nono::undo::NetworkAuditAuthMechanism::ProxyAuthorization),
                auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
                denial_category: Some(nono::undo::NetworkAuditDenialCategory::AuthenticationFailed),
                ..Default::default()
            },
            // ...
        );
        // ... 407 ...
    }
}
```

**`external.rs::handle_external_proxy`'s gate** (`external.rs:87-126`):
```rust
/// Shared context for [`handle_external_proxy`].
pub struct ExternalProxyCtx<'a> {
    pub filter: &'a ProxyFilter,
    pub session_token: &'a Zeroizing<String>,
    pub audit_log: Option<&'a audit::SharedAuditLog>,
    pub require_auth: bool,
}

pub async fn handle_external_proxy(/* ... */) -> Result<()> {
    let (host, port) = parse_connect_target(first_line)?;
    if ctx.require_auth {
        validate_proxy_auth(remaining_header, ctx.session_token)?;
    }
    // ... cloud-metadata deny check ...
}
```

**`server.rs` dispatcher's inline CONNECT auth calls** (`server.rs:1271-1317`, three distinct call
shapes in one dispatcher — this is where D-03's `route.has_spiffe_source()` check needs to be
threaded through for the CONNECT/bypass/external-chain arms, since none of these three arms know
about routes at all today):
```rust
if let Some(ext_config) = use_external {
    let ext_ctx = external::ExternalProxyCtx { /* ..., require_auth: state.config.require_auth */ };
    external::handle_external_proxy(first_line, &mut stream, &header_bytes, &ext_ctx, ext_config).await
} else if state.config.external_proxy.is_some() {
    // Bypass route: enforce strict session token validation before routing direct.
    if state.config.require_auth {
        token::validate_proxy_auth(&header_bytes, &state.session_token)?;
    }
    connect::handle_connect(
        first_line, &mut stream, &state.filter, &state.session_token, &header_bytes,
        Some(&state.audit_log),
        state.config.require_auth && state.config.strict_connect_auth,
    ).await
} else {
    connect::handle_connect(/* same shape, non-bypass */).await
}
```

**D-03's composed guard (new code, no existing literal analog — compose per this shape):**
```rust
// After route resolution, before forwarding, on each of the 3 paths above:
if let Some(route) = resolved_route {
    if route.has_spiffe_source() {
        // This request path has no SPIFFE implementation (D-01) — never
        // silently forward unauthenticated. Fail closed, mirroring the
        // WR-13/require_auth 407 shape above (403/503, not silent pass-through).
        audit::log_denied(/* denial_category: SpiffeUnsupportedPath or similar */);
        send_error(stream, 403, "Forbidden").await?;
        return Ok(());
    }
}
```
This composes with (does not replace) the `require_auth`/`strict_connect_auth` checks already
shown — those gate the SESSION-TOKEN question; D-03's guard gates the separate "does this route
need a credential this code path cannot supply" question. Note: `handle_connect`/`connect.rs`
itself takes no `require_auth` parameter directly — it's threaded in via the boolean at the
`server.rs` call sites above, so D-03's route lookup most naturally belongs in `server.rs`'s
dispatcher (all three non-reverse arms), not inside `connect.rs`/`external.rs` themselves, unless
those functions gain a `route_store: &RouteStore` parameter.

---

### 5. `crates/nono-cli/data/nono-profile.schema.json` — `aws_auth`/`$defs/AwsAuthConfig`/`$defs/OAuth2Config` verbatim

**`aws_auth` property block** (`nono-profile.schema.json:638-644`, the literal copy-pattern for the
new `spiffe` property):
```json
"aws_auth": {
  "oneOf": [
    { "$ref": "#/$defs/AwsAuthConfig" },
    { "type": "null" }
  ],
  "description": "AWS SigV4 signing configuration. When present, the proxy signs outbound requests with AWS credentials. Mutually exclusive with `credential_key` and `auth`."
},
```
New `spiffe` property should read: `"oneOf": [{"$ref": "#/$defs/SpiffeAuthConfig"}, {"type": "null"}]`
with a description naming the same mutual-exclusion set (`credential_key`, `auth`, `aws_auth`).

**`$defs/AwsAuthConfig` verbatim** (`nono-profile.schema.json:732-759`, the shape to mirror for
`$defs/SpiffeAuthConfig`/`$defs/ClientAssertionConfig` — all-optional-fields, `additionalProperties: false`):
```json
"AwsAuthConfig": {
  "type": "object",
  "description": "AWS SigV4 signing configuration for a credential route. All fields are optional: an empty {} block uses the default credential chain (IAM role, environment variables, ~/.aws/credentials).",
  "additionalProperties": false,
  "properties": {
    "profile": {
      "oneOf": [{ "type": "string" }, { "type": "null" }],
      "description": "AWS profile name from ~/.aws/credentials or ~/.aws/config. When omitted, the default credential chain is used."
    },
    "region": {
      "oneOf": [{ "type": "string" }, { "type": "null" }],
      "description": "Explicit SigV4 signing region (e.g., \"us-east-1\"). When omitted, the region is resolved from the environment (AWS_REGION, AWS_DEFAULT_REGION, or instance metadata)."
    },
    "service": {
      "oneOf": [{ "type": "string" }, { "type": "null" }],
      "description": "Explicit SigV4 service name (e.g., \"execute-api\", \"s3\"). When omitted, the service is inferred from the upstream URL."
    }
  }
}
```
`$defs/SpiffeAuthConfig` needs `workload_api_socket`/`audience` marked REQUIRED (unlike
`AwsAuthConfig`'s all-optional shape) — copy the `additionalProperties: false` + per-property
`oneOf`/`description` idiom, not the all-optional posture.

**`$defs/OAuth2Config` verbatim, INCLUDING its `required` array** (`nono-profile.schema.json:707-731`
— this is the exact block RESEARCH flags as needing correction: `client_id`/`client_secret` are
wrongly `required` for the new `client_assertion` shape):
```json
"OAuth2Config": {
  "type": "object",
  "description": "PROF-03 (Phase 22) OAuth2 client_credentials configuration. Plan 22-01 lands the deserialize wiring + fail-closed validation; Plan 22-04 (OAUTH) implements the actual token-exchange client. token_url MUST be HTTPS (http only for loopback) per fail-secure principle (T-22-01-02 BLOCKING).",
  "additionalProperties": false,
  "required": ["token_url", "client_id", "client_secret"],
  "properties": {
    "token_url": {
      "type": "string",
      "description": "OAuth2 token endpoint URL. MUST be HTTPS (http allowed only for loopback hosts). Rejected fail-closed at profile-load time."
    },
    "client_id": {
      "type": "string",
      "description": "OAuth2 client ID. Plain value or credential reference. Must not be empty."
    },
    "client_secret": {
      "type": "string",
      "description": "OAuth2 client secret. May be a plain value, env://VAR, file:///path, op://… (1Password), apple-password://…, or keyring://service/account URI. Resolved through nono::keystore::load_secret at proxy startup."
    },
    "scope": {
      "type": "string",
      "description": "OAuth2 scopes (space-separated). Empty string = no scope parameter sent.",
      "default": ""
    }
  }
}
```
**Required fix (RESEARCH-flagged, not optional):** `"required": ["token_url", "client_id", "client_secret"]`
must change to `"required": ["token_url"]` once `client_assertion` is added as an alternative to
`client_id`/`client_secret` — the Rust struct already made both `#[serde(default)]` for exactly
this reason (config.rs's `OAuth2Config`, matching this schema, must be checked for the same
`#[serde(default)]` presence on `client_id`/`client_secret` before/while editing this block).
Also add `client_assertion` (`oneOf: [{"$ref": "#/$defs/ClientAssertionConfig"}, {"type": "null"}]`)
and `extra_params` (`{"type": "object", "additionalProperties": {"type": "string"}}`) properties.

**Rust-side mirror to check while editing** (`config.rs:1051-1064`, confirms the schema's
`required` array is currently in sync with a Rust struct that has NOT yet added the `#[serde(default)]`
relaxation SPIFFE needs):
```rust
pub struct OAuth2Config {
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scope: String,
}
```

---

### 6. Test analogs — `nono-proxy/tests/` (does not exist yet) and `nono-cli/tests/`

**`crates/nono-proxy/tests/spiffe_integration.rs` has no existing sibling** — `grep`/`ls` confirm
`crates/nono-proxy/` currently has **no `tests/` directory at all**; every existing `nono-proxy`
test lives in `#[cfg(test)] mod tests` blocks inside `src/*.rs` files (e.g. `route.rs:600-644`,
excerpted above). This will be the first `nono-proxy` integration-test file. There is no
crate-internal analog to copy scaffolding boilerplate from; use `nono-cli/tests/*.rs`'s general
`Command`/env-var conventions as the nearest cross-crate shape reference (below), and rely on
RESEARCH's confirmed finding that `spiffe_integration.rs`'s mock server is a plain
`std::net::TcpListener`, requiring no new test-helper crate dependency.

**`crates/nono-cli/tests/spiffe_run.rs`'s closest analog: `socket_access_run.rs`** — same
`Command::new(env!("CARGO_BIN_EXE_nono"))` scaffolding shape:
```rust
fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

fn setup_isolated_home(prefix: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp_root = std::env::current_dir().expect("cwd").join("target").join("test-artifacts");
    fs::create_dir_all(&temp_root).expect("create temp root");
    let tmp = tempfile::Builder::new().prefix(&format!("nono-{prefix}-it-")).tempdir_in(&temp_root).expect("tempdir");
    // ...
}

fn run_nono(args: &[&str], home: &Path, cwd: &Path) -> Output {
    nono_bin().args(args).env("HOME", home) /* ... */ .output().expect("failed to run nono")
}
```

**socket_access_run.rs's skip idiom — this is the D-07 ANTI-PATTERN to explicitly NOT repeat**
(`socket_access_run.rs:63-77`, the exact silent-skip shape D-07 exists to replace):
```rust
fn python3_available() -> bool {
    Command::new("python3")
        .args(["-c", "import socket"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
#[cfg(target_os = "linux")]
fn af_unix_mediation_pathname_blocks_connect_to_unlisted_socket() {
    if !python3_available() {
        eprintln!("skipping: python3 not available");
        return;
    }
    // ...
}
```
This `eprintln!("skipping: ...")` + bare `return;` is unaggregated, uncounted, and invisible to
`cargo test`'s summary line (reports `ok` even though nothing ran) — exactly the failure mode
D-06/D-07 name. `spiffe_run.rs`/`spiffe_integration.rs` must use the `SKIP[...]`-prefixed,
`--nocapture`-greppable marker the RESEARCH document proposes (see 113-RESEARCH.md "D-07 Loud-Skip-
Reporting Mechanism — Design Proposal") instead of bare `eprintln!("skipping: ...")`.

---

### 7. `.github/workflows/spire.yml` (new) — analog `.github/workflows/ci.yml`

**Header block to copy shape from** (`ci.yml:1-27`):
```yaml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -Dwarnings

jobs:
  changes:
    name: Classify Changes
    runs-on: ubuntu-latest
    outputs:
      run_code_jobs: ${{ steps.classify.outputs.run_code_jobs }}
      run_docs_checks: ${{ steps.classify.outputs.run_docs_checks }}
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6
        with:
          fetch-depth: 0
```
Note this fork's `ci.yml` pins `actions/checkout` at a DIFFERENT SHA (`de0fac2e...` / v6) than the
one RESEARCH flagged from upstream's diff (`9c091bb2...` / v7.0.0, Assumption A2) — use THIS
fork's currently-pinned `checkout` SHA for consistency across workflow files rather than trusting
upstream's diff SHA, unless a newer version is deliberately being adopted. `spire.yml` should be a
standalone `runs-on: ubuntu-latest` Linux-only lane (per D-06, no cross-platform SPIRE need),
`permissions: contents: read` matching `ci.yml`'s minimal-permissions posture.

---

### 8. `../nono-py/src/proxy.rs:206-218` — exhaustive `RustRouteConfig` literal, exact fix site

**Verbatim current state** (`../nono-py/src/proxy.rs:205-226`):
```rust
Self {
    inner: RustRouteConfig {
        prefix,
        upstream,
        credential_key,
        inject_mode: inject_mode.into(),
        inject_header,
        credential_format,
        path_pattern,
        path_replacement,
        query_param_name,
        env_var,
        oauth2: None,
        aws_auth: None,
        endpoint_rules: endpoint_rules
            .into_iter()
            .map(|(method, path)| RustEndpointRule { method, path })
            .collect(),
        tls_ca,
        endpoint_policy: None,
```
**Concrete one-line fix (D-09):** insert `spiffe: None,` immediately after `aws_auth: None,`
(line 218) once `RouteConfig` gains the `spiffe` field. This is the fourth consecutive occurrence
of this exact break pattern (after `endpoint_policy`/`enable_h2` in v3.4, `denied_hosts`/`no_proxy`
in 109-05, `require_auth`/`strict_connect_auth` in Phase 112) — confirmed still present, not yet
fixed to `..Default::default()` (that fix was explicitly deferred, see CONTEXT.md's Deferred Ideas).
`../nono-ts` needs no equivalent fix — confirmed zero `nono_proxy` references in that repo.

---

## Shared Patterns

### Error type: `ProxyError` (all new `nono-proxy` code)
**Source:** `crates/nono-proxy/src/error.rs:1-50`
**Apply to:** `spiffe.rs`, `auth.rs`, `credential.rs`, `route.rs`, `reverse.rs` additions
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Host denied by filter: {host}: {reason}")]
    HostDenied { host: String, reason: String },
    #[error("Credential loading error: {0}")]
    Credential(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("OAuth2 token exchange error: {0}")]
    OAuth2Exchange(String),
    // ...
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, ProxyError>;
```
No new `SpiffeError` variant family needed — reuse `ProxyError::Config` (connect/setup failures)
and `ProxyError::Credential` (fetch-token failures), matching `oauth2.rs`'s existing split between
`ProxyError::Config` (runtime-context failure) and `ProxyError::OAuth2Exchange` (exchange failure).

### Auth-gate composition: `require_auth`/`strict_connect_auth` (Phase 112)
**Source:** `crates/nono-proxy/src/config.rs:108-148`, `crates/nono-proxy/src/server.rs:361-380`
**Apply to:** every D-03 guard site (item 4 above) — D-03's new route-level check must be additive
to, never a replacement for, these existing session-token gates.
```rust
#[serde(default = "default_require_auth")]
pub require_auth: bool,
// ...
pub strict_connect_auth: bool,
// Default::default(): require_auth: true, strict_connect_auth: false,
fn default_require_auth() -> bool { /* true */ }
```

### Secret handling: `Zeroizing<String>` everywhere a token crosses a boundary
**Source:** `crates/nono-proxy/src/oauth2.rs:46-90` (excerpted above), `crates/nono-proxy/src/credential.rs:42-63`
**Apply to:** `spiffe.rs::SpiffeJwtSource::fetch_token`, any `SpiffeAssertionTokenCache` cached
token, `auth.rs::ManagedUpstreamAuth` variants that carry a fetched secret.

### `EventContext`/audit call idiom
**Source:** `crates/nono-proxy/src/audit.rs:40-47`, used at every `reverse.rs`/`server.rs`/`external.rs` denial site
```rust
pub struct EventContext<'a> {
    pub route_id: Option<&'a str>,
    pub auth_mechanism: Option<NetworkAuditAuthMechanism>,
    pub auth_outcome: Option<NetworkAuditAuthOutcome>,
    pub managed_credential_active: Option<bool>,
    pub injection_mode: Option<NetworkAuditInjectionMode>,
    pub denial_category: Option<NetworkAuditDenialCategory>,
    // + new: pub spiffe_context: Option<SpiffeAuditContext>,
}
```
Call shape (`reverse.rs:228-243`, repeated at every denial site in the crate):
```rust
audit::log_denied(
    ctx.audit_log,
    audit::ProxyMode::Reverse,
    &audit::EventContext {
        route_id: Some(&service),
        auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
        managed_credential_active: Some(true),
        denial_category: Some(nono::undo::NetworkAuditDenialCategory::AuthenticationFailed),
        ..Default::default()
    },
    &service,
    0,
    &e.to_string(),
);
```

---

## Landmines (research-verified; re-stated here so the planner cannot miss them)

### L1 — Let-chains pass `cargo build`, fail `cargo fmt --all --check`
Two confirmed instances land in this diff: `spiffe.rs::check_nbf`'s `if let Some(nbf) = ... &&
now_ts < nbf` and `proxy_runtime.rs`'s host-collection loop `if let Ok(parsed) = url::Url::parse(...)
&& let Some(host) = parsed.host_str()`. This workspace is edition 2021; let-chains require a later
edition. `rustc 1.95` silently accepts the syntax (`cargo build`/`cargo check` exit 0); only
`cargo fmt --all --check` (part of `make ci`) rejects it. **This is the identical failure Plan
112-02 already hit and fixed.** The fork's established fix pattern (`crates/nono-cli/src/hook_runtime.rs:360-370`):
```rust
// Nested `if let` (not an `if let ... && let ...` let-chain) — let-chains
// require Rust 2024 and this workspace is edition 2021. This code is
// `cfg(unix)`-only and is never compiled on the Windows dev host, so the
// edition violation escaped local checks (cross-target drift).
if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(&self.path) {
    if let Ok(metadata) = file.metadata() {
        // ...
    }
}
```
Rewrite both `spiffe.rs::check_nbf` and `proxy_runtime.rs`'s loop to this nested-`if let` shape
(with the same explanatory comment) before either file reaches `cargo fmt --all --check`.

### L2 — `.gitignore:16`'s bare `proj/` pattern silently drops the NEW ADR file
`.gitignore:16` contains a bare `proj/` ignore pattern. `git ls-files proj/` proves already-tracked
files are unaffected:
```
proj/ADR-100-ci-pipeline-reconcile.md
proj/ADR-108-deny-domain-posture.md
proj/ADR-111-resource-limits-boundary.md
proj/ADR-112-allow-vars-fail-closed-preserved.md
proj/ADR-74-privilege-model.md
proj/ADR-86-library-boundary-convergence.md
proj/ADR-87-cr02-audit-bypass.md
proj/ADR-98-network-intent-disposition.md
proj/DESIGN-engine-abstraction.md
proj/POC-zt-infra-e5-local-provisioner.md
```
All ten of these are tracked and visible to `git status` despite the ignore rule — proving the
pattern only bites brand-NEW files. `proj/ADR-113-spiffe-disposition.md` is new; it MUST be staged
with `git add -f proj/ADR-113-spiffe-disposition.md`, or a plain `git add`/`git commit` will
silently omit it (no error, no warning). `proj/ADR-111-resource-limits-boundary.md`'s header shape
(`# ADR-NNN: Title`, `**Status:**`/`**Phase:**`/`**Date:**`/`**Authors:**` frontmatter, `## Context`
first section) is the precedent CONTEXT.md cites for ADR-113's shape.

### L3 — `audit.rs`'s diff DELETES `log_reverse_proxy`, but the fork still CALLS it live
Upstream's `c831dade` diff deletes `log_reverse_proxy` (deprecated upstream since 0.46.0). But:
```
crates\nono-proxy\src\audit.rs:296:pub fn log_reverse_proxy(
crates\nono-proxy\src\reverse.rs:460:    audit::log_reverse_proxy(
```
The live call site, `reverse.rs:460-466` (also shown in Pattern 3 above):
```rust
audit::log_reverse_proxy(
    ctx.audit_log,
    &service,
    &method,
    &upstream_path,
    status_code,
);
```
**Do NOT delete `log_reverse_proxy`** when adapting `audit.rs` — only add the new
`spiffe_context: Option<SpiffeAuditContext>` field to `EventContext` and thread it through
`log_allowed`/`log_denied`/`log_l7_request`/`log_l7_policy_decision`/`log_credential_capture`.

### L4 — `server.rs:235`'s divergence comment becomes factually false under D-04
**Verbatim current text** (`server.rs:231-236`):
```rust
// ============================================================================
// no_proxy (#1415, D-06/D-07) — push-based NO_PROXY/NONO_NO_PROXY pipeline,
// route-conflict guards, and startup validation. Ported from upstream
// 1619275c and adapted to this fork's simpler `ProxyHandle`/`RouteStore`
// shape (no TLS intercept, no SPIFFE, no async RouteStore::load).
// ============================================================================
```
Under D-04 (adopt async `RouteStore::load`) and this phase's SPIFFE absorb, 2 of the 3
parenthetical clauses become false ("no SPIFFE", "no async `RouteStore::load`" — only "no TLS
intercept" remains true per D-01). **Replacement text must be written**, e.g.: *"...adapted to
this fork's `ProxyHandle`/`RouteStore` shape (no TLS intercept — see ADR-113/D-01 — but now with
SPIFFE-route support and async `RouteStore::load`, absorbed in Phase 113)."* Exact wording is the
planner's/executor's call; the constraint is only that it must stop asserting the two now-false
clauses.

### L5 — `route.rs:617-639`'s comment misattributes the gap to `tls_intercept`; real ancestor is `b1ecbc02`
**Verbatim current text** (`route.rs:614-639`):
```rust
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
```
This attribution is factually wrong — `git log --all -S"handle_oauth2_credential" -- crates/nono-proxy/src/reverse.rs`
on the `upstream` remote traces this machinery to `b1ecbc02` (`git describe` = `v0.38.0-3-gb1ecbc02`),
an unrelated, much older commit with zero relationship to `tls_intercept`. The operator has decided
this phase CORRECTS it while touching `route.rs` anyway (adding `managed_auth`/`has_spiffe_source()`)
— replace "belong to the upstream tls_intercept module not present in this fork" with an accurate
attribution to `b1ecbc02`'s general (declined-this-phase) OAuth2 route-wiring layer, so a future
absorb doesn't inherit the wrong mental model a second time.

### L6 — no `nono-proxy/tests/` directory exists yet
Confirmed via directory listing: `crates/nono-proxy/tests/*.rs` currently matches nothing.
`spiffe_integration.rs` will be the first file in a brand-new `crates/nono-proxy/tests/` directory
— there is no `Cargo.toml` `[[test]]` wiring to check/extend (Rust auto-discovers `tests/*.rs` as
integration test binaries), but the planner should confirm `nono-proxy`'s `Cargo.toml` `[dev-dependencies]`
carries whatever `spiffe_integration.rs` needs (e.g. `tokio` test features) — it is not implied by
`src/`'s existing dependency set alone.

---

## No Analog Found

None — every file in scope has at least a role-match analog in the fork. The three files RESEARCH
flags as "build a narrow SPIFFE-only slice from scratch" (`credential.rs`'s `spiffe_assertion_routes`,
`route.rs`'s `managed_auth`/`has_spiffe_source()`, `reverse.rs`'s `handle_spiffe_route` rewrite) are
NOT analog-less — they have exact same-file analogs (the sibling `aws_routes` placeholder map, the
existing `LoadedRoute` struct, and `handle_reverse_proxy`'s `static_cred` flow, respectively); what
they lack is upstream's own diff as a literal port target, per the Decision Conflicts section. This
distinction is called out per-file in Pattern Assignments above rather than listed here.

## Metadata

**Analog search scope:** `crates/nono-proxy/src/*.rs`, `crates/nono-cli/src/{profile/mod,network_policy,proxy_runtime,hook_runtime}.rs`, `crates/nono-cli/tests/*.rs`, `crates/nono/src/{undo/types,audit}.rs`, `crates/nono-cli/data/nono-profile.schema.json`, `.github/workflows/*.yml`, `../nono-py/src/proxy.rs`, `.gitignore`, `proj/*.md`
**Files scanned:** ~30 (direct reads/greps against the live tree this session; no upstream-remote reads beyond what 113-RESEARCH.md already captured)
**Pattern extraction date:** 2026-08-06
