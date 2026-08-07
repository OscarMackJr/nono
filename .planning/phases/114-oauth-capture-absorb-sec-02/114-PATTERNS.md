# Phase 114: OAuth Capture Absorb (SEC-02) - Pattern Map

**Mapped:** 2026-08-06
**Files analyzed:** 11 (new + modified)
**Analogs found:** 11 / 11

All line numbers below were re-verified live against the working tree during this pass
(not trusted from RESEARCH.md alone). Where a research-cited line number had drifted, the
corrected number is used and the drift is noted.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono-proxy/src/capture.rs` (NEW) | service (phantom store) | transform / event-driven | `crates/nono-proxy/src/spiffe.rs` | role-match, **inverse direction** (see note below) |
| `crates/nono-proxy/src/reverse.rs` (buffer-and-rewrite helper + 3 call sites) | service (relay/enforcement) | streaming → buffered transform | its own 3 existing `[0u8; 8192]` relay loops (self-analog) | exact (same file, same function shape, 3 sites) |
| `crates/nono-proxy/src/server.rs` (D-06 guard in `handle_forward_http`) | middleware (request-time guard) | request-response | `spiffe_declared_for_upstream()` call site in `handle_forward_http` | exact |
| `crates/nono-proxy/src/config.rs` (`CaptureConfig` + friends) | config/model | CRUD (declarative config) | `RouteConfig.spiffe` field + `OAuth2Config` struct | exact |
| `crates/nono-proxy/src/route.rs` (`declares_capture`, `capture_declared_for_upstream()`) | model/service | CRUD (loaded-config predicate) | `declares_spiffe` / `has_spiffe_source()` / `spiffe_declared_for_upstream()` | exact |
| `crates/nono-proxy/src/audit.rs` (`capture_context` field) | model (audit context) | event-driven | `EventContext.spiffe_context` | exact |
| `crates/nono/src/undo/types.rs` (`CaptureAuditContext` + `NetworkAuditDenialCategory` variant) | model (pure data) | transform | `SpiffeAuditContext` / `SpiffeUnsupportedPath` variant | exact |
| `crates/nono-cli/src/profile/mod.rs` (`CustomCredentialDef.capture` + validation) | model/validator | CRUD (profile parse+validate) | `CustomCredentialDef.spiffe` + `validate_custom_credential()`'s SPIFFE arm | role-match (mutual-exclusion shape likely does NOT apply verbatim — see Open Design Question) |
| `crates/nono-cli/src/profile/mod.rs` (schema round-trip test) | test | request-response (schema validation) | `test_schema_validates_spiffe_custom_credential` | exact |
| `crates/nono-cli/data/nono-profile.schema.json` (`CaptureConfig` `$defs`) | config (schema) | CRUD | `$defs.SpiffeAuthConfig` + `RouteConfig.properties.spiffe` | exact |
| `../nono-py/src/proxy.rs`, `../nono-py/src/policy.rs` (RouteConfig struct-literal fix) | binding | CRUD (struct construction) | the existing `spiffe: None,` lines in both files | exact |
| `proj/ADR-114-oauth-capture-disposition.md` | doc (ADR) | — | `proj/ADR-113-spiffe-disposition.md` | exact (structure) |
| `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` (SEC-02 row + carry-forward note) | doc (ledger) | — | "SPIFFE Carry-Forward Note (Phase 113, D-02)" section | exact (structure) |

---

## Pattern Assignments

### `crates/nono-proxy/src/capture.rs` (NEW module — phantom store)

**Analog:** `crates/nono-proxy/src/spiffe.rs` (168 lines, full file read) — **structural** analog
only (one small standalone module, `#[cfg(test)] mod tests` at the bottom, `zeroize`d secret
material, `crate::error::{ProxyError, Result}` imports). **Direction mismatch, stated explicitly:**
`spiffe.rs`'s `SpiffeJwtSource::fetch_token()` runs the *outbound* direction — the proxy fetches a
real credential and injects it into a request going *to* upstream. Capture's `capture.rs` runs the
*inbound* direction — the proxy must mint a **new** phantom for a real token it just received
*from* upstream and store a `phantom → real` reverse mapping (a `HashMap`-shaped store, not a
single-fetch client). Do not copy `spiffe.rs`'s single-value `fetch_token()` shape; copy only its
module hygiene (imports, zeroize discipline, doc comment on why secrets never hit disk, bottom-of-file
`#[cfg(test)] mod tests` with `#[allow(clippy::unwrap_used)]`).

**Imports pattern** (`spiffe.rs` lines 1-9, verified live):
```rust
//! SPIFFE/SPIRE Workload API credential sources.
//! SVID private key material is never written to disk or logged.

use crate::error::{ProxyError, Result};
use base64::Engine as _;
use spiffe_workload::{JwtSource, SpiffeId};
use std::sync::Arc;
use tracing::{debug, warn};
use zeroize::Zeroizing;
```
`capture.rs`'s equivalent header should read: no `spiffe_workload` dep (not needed), but keep
`base64::Engine as _` (for JWT-shaped phantom construction, per D-07/RESEARCH Code-Examples), add
`serde_json` (dot-path rewrite/JWT payload), `getrandom` (phantom generation), and
`std::collections::HashMap` (the phantom→real store) alongside `zeroize::Zeroizing`.

**Zeroize discipline to mirror** (`spiffe.rs:65-93`, `fetch_token`):
```rust
pub async fn fetch_token(&self, audience: &[String]) -> Result<(Zeroizing<String>, String)> {
    ...
    Ok((Zeroizing::new(svid.token().to_string()), spiffe_id))
}
```
Capture's store must hold real tokens the same way — `Zeroizing<Vec<u8>>` or `Zeroizing<String>`,
never a bare `String`/`Vec<u8>` (CLAUDE.md, D-08). The nearest WRONG analog to avoid is
`reverse.rs`'s `body` request-buffer at `reverse.rs:392-401`, which is deliberately `Vec<u8>`
because it is *not* secret (see Anti-Patterns / Pitfall 5 in RESEARCH.md — confirmed still true,
`body` variable at that range is a plain `Vec<u8>`).

**Test module shape to mirror** (`spiffe.rs:153-168`, full block, verified live):
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use spiffe_workload::JwtSource;

    #[tokio::test]
    async fn test_jwt_source_fails_closed_on_missing_socket() {
        let endpoint = "unix:/tmp/nono-test-nonexistent-spire-agent.sock";
        let result = JwtSource::builder()
            .endpoint(endpoint)
            .initial_sync_timeout(std::time::Duration::from_secs(1))
            .build()
            .await;
        assert!(result.is_err(), "should fail when socket does not exist");
    }
}
```
Note the "fails closed" framing in the test name — `capture.rs`'s own tests should follow the same
naming convention (`capture_buffer_cap_exceeded_denies_response`, etc., per RESEARCH.md's
Phase-Requirements → Test-Map table).

---

### `crates/nono-proxy/src/reverse.rs` — buffer-and-rewrite helper + 3 call sites

**Analog:** the file's own three existing relay loops (self-analog, byte-for-byte structurally
identical, verified live at lines **474-498**, **764-785**, **1019-1040** — matches RESEARCH.md's
cited line numbers exactly).

**Relay loop pattern, site 1 of 3** (`reverse.rs:472-498`, verified live, full text):
```rust
    // Stream the response back to the client without buffering.
    // This handles SSE (text/event-stream), chunked transfer, and regular responses.
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
```
Sites 2 (`reverse.rs:763-785`) and 3 (`reverse.rs:1019-1040`) are the same shape with only the
surrounding function/variable names differing (`handle_spiffe_route` / `handle_spiffe_assertion_credential`
call sites, per RESEARCH.md's architecture map). All three must call the new shared helper when
`route`/`cred` declares capture — do not hand-duplicate a fourth copy.

**Request-body cap idiom to mirror at a tighter bound** (`reverse.rs:36`, `384-401`, verified live):
```rust
const MAX_REQUEST_BODY: usize = 16 * 1024 * 1024;
...
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
```
D-05's response cap must mirror the shape (check length before allocating, bound actual bytes
read, fail closed — `send_error(..., 413, ...)` equivalent must become a deny, never a
"forward partial" fallback) at a value in the 256 KiB-1 MiB range instead of 16 MiB, and must be
enforced against **actual bytes read**, not just a declared `Content-Length` header (an upstream
lying about `Content-Length` or omitting it via chunked transfer must not bypass the cap — this
mirrors the pattern above, which already does both: header pre-check AND still bounds the read
loop).

**Status-line parse helper to reuse as-is** (`reverse.rs:1337-1359`, verified live, full function):
```rust
pub(crate) fn parse_response_status(data: &[u8]) -> u16 {
    let line_end = data
        .iter()
        .position(|&b| b == b'\r' || b == b'\n')
        .unwrap_or(data.len());
    let first_line = &data[..line_end.min(64)];

    if let Ok(line) = std::str::from_utf8(first_line) {
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
```

**Header/body split helper to reuse** (`reverse.rs:1199-1208`, verified live, full function):
```rust
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
```
Both `parse_response_status` and `extract_content_length` are `pub(crate)` already — the new
buffer-and-rewrite helper can call them directly, no need to duplicate header parsing.

**Error-response idiom** (`reverse.rs:1362-1374`, verified live, full function):
```rust
async fn send_error(stream: &mut TcpStream, status: u16, reason: &str) -> Result<()> {
    let body = format!("{{\"error\":\"{}\"}}", reason);
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        status, reason, body.len(), body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
```
This is a private (non-`pub(crate)`) helper — the new buffer-and-rewrite path can either call it
directly (same module) or return a `Result<...>` that the 3 call sites turn into a `send_error`
call on any fail-closed branch (cap exceeded, content-encoded, unrewritten token field found).

**Upstream design-reference only (NOT live in this fork — CITED, do not treat as copy-paste-able):**
the header/body split + `Content-Encoding` rejection shape from upstream's `forward.rs` (git show
`9b692e07:crates/nono-proxy/src/forward.rs` lines ~373-425) and the JSON dot-path rewrite +
fail-closed unrewritten-token-field scan from `oauth_capture/rewrite.rs` (full file, CITED) are
real portable *logic* per D-07, but depend on absent `forward.rs`/`oauth_capture/` types — port the
algorithm shape only, rebuild the types against this fork's `CaptureConfig`. RESEARCH.md's
"Architecture Patterns" § Pattern 2 and Pattern 3 already contain the exact upstream snippets;
not re-quoted here to avoid duplication — read RESEARCH.md directly for those two blocks.

---

### `crates/nono-proxy/src/server.rs` — D-06 cross-path fail-closed guard

**Analog:** the existing SPIFFE guard inside `handle_forward_http`, verified live at
`server.rs:1054-1077` (RESEARCH.md cited 1043-1077; the check itself starts at line 1054/1055,
1043-1053 is the preceding doc comment — both ranges land in the same block, no drift of
substance).

**Guard pattern, exact block to mirror** (`server.rs:1043-1077`, verified live, full text incl.
doc comment):
```rust
    // D-03 (113-CONTEXT.md): this forward-HTTP path has no SPIFFE
    // implementation of its own — the fork carries no `tls_intercept`
    // module (D-01), so there is nowhere for a JWT-SVID to be fetched or
    // injected on this code path. A request whose absolute-form target
    // matches a SPIFFE-declared route's upstream must never be silently
    // forwarded unauthenticated here, since that would bypass the route's
    // only auth mechanism entirely. Unconditional — NOT gated on
    // `state.config.require_auth` (mirroring the WR-13 guard's structural
    // shape above but for an orthogonal condition): a SPIFFE-declared
    // route's auth requirement does not depend on whether session-token
    // auth happens to be enabled for this proxy instance.
    let host_port = format!("{}:{}", host, port);
    if state.route_store.spiffe_declared_for_upstream(&host_port) {
        warn!(
            "Blocked forward-HTTP request to SPIFFE-declared route upstream {} — this path has \
             no SPIFFE implementation (D-01); use the reverse-proxy path instead",
            host_port
        );
        audit::log_denied(
            Some(&state.audit_log),
            audit::ProxyMode::Reverse,
            &audit::EventContext {
                denial_category: Some(
                    nono::undo::NetworkAuditDenialCategory::SpiffeUnsupportedPath,
                ),
                ..audit::EventContext::default()
            },
            &host,
            port,
            "SPIFFE-declared route upstream: forward-HTTP path has no SPIFFE implementation",
        );
        let response = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }
```
D-06's new guard is this exact shape with `capture_declared_for_upstream` /
`NetworkAuditDenialCategory::CaptureUnsupportedPath` (new variant, name TBD by planner)
substituted — same unconditional placement (not gated on `require_auth`), same 403 response body.

**CONNECT dispatch — confirm NO new guard needed there** (`server.rs:1320-1352`, verified live):
```rust
        if !state.route_store.is_empty() {
            if let Some(authority) = first_line.split_whitespace().nth(1) {
                let host_port = if authority.starts_with('[') {
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
                    let is_spiffe_route =
                        state.route_store.spiffe_declared_for_upstream(&host_port);
                    let denial_category = if is_spiffe_route {
                        nono::undo::NetworkAuditDenialCategory::SpiffeUnsupportedPath
                    } else {
                        nono::undo::NetworkAuditDenialCategory::ConnectBypassesL7
                    };
                    ...
```
`is_route_upstream()` already blocks CONNECT to **every** route upstream unconditionally before
any SPIFFE/capture-specific refinement — confirmed live, matches RESEARCH.md's claim exactly. The
planner's task here is only to add an `is_capture_route` branch to this `if/else` (mirroring
`is_spiffe_route`) so the audit `denial_category` distinguishes a capture-route CONNECT-bypass from
a plain one — this is an audit-fidelity refinement, not new deny surface. A regression test proving
this (`connect_denies_capture_declared_route_upstream`) still belongs in Wave 0 per RESEARCH.md's
test map, even though no new *code* branch is strictly required to make it pass.

---

### `crates/nono-proxy/src/config.rs` — `CaptureConfig` declarative surface

**Analog:** `RouteConfig.spiffe` field + `OAuth2Config` struct shape (both in the same file).

**Field-and-doc-comment pattern to mirror** (`config.rs:619-622`, verified live):
```rust
    /// SPIFFE/SPIRE workload identity auth. Mutually exclusive with
    /// `credential_key`, `oauth2`, and `aws_auth`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spiffe: Option<SpiffeAuthConfig>,
```
A `capture: Option<CaptureConfig>` field follows this exact `#[serde(default, skip_serializing_if
= "Option::is_none")]` idiom. Whether it needs the same "mutually exclusive with X, Y, Z" framing
is the open design question RESEARCH.md flags (capture is response-direction, the other four
fields are request-direction) — do not assume mutual exclusion without resolving that question
first (see "Open Design Questions" below).

`OAuth2Config` (struct header at `config.rs:1083`, not fully re-read this pass — RESEARCH.md
confirms full struct body was read; reuse its `#[serde(default)]`-per-optional-field convention as
the shape for `CaptureConfig`'s own sub-fields, e.g. `CaptureTokenEndpointConfig`,
`CaptureResponseFieldConfig`).

---

### `crates/nono-proxy/src/route.rs` — `declares_capture` / `capture_declared_for_upstream()`

**Analog:** `LoadedRoute.declares_spiffe` + `has_spiffe_source()` + `RouteStore::spiffe_declared_for_upstream()`.

**Field + accessor pattern** (`route.rs:70-90`, verified live, full text):
```rust
    /// Whether this route declared a `spiffe` auth source at load time,
    /// independent of whether the live connect succeeded (a failed connect
    /// aborts the whole `RouteStore::load` call per D-04, so by the time any
    /// `RouteStore` exists this always equals `managed_auth.is_some()`).
    /// Kept as a separate field so `has_spiffe_source()` is unit-testable
    /// without constructing a real `SpiffeJwtSource` (which has no test-only
    /// constructor bypassing a live Workload API connection).
    pub declares_spiffe: bool,
}

impl LoadedRoute {
    /// Whether this route requires a live SPIFFE/SPIRE-issued credential.
    /// Cheap, always-correct, and answerable without a live SPIRE agent —
    /// the D-01 proof surface Plan 113-05's D-03 guard and this phase's ADR
    /// both depend on.
    #[must_use]
    pub fn has_spiffe_source(&self) -> bool {
        self.declares_spiffe
    }
}
```
`declares_capture: bool` + `has_capture_source(&self) -> bool` follow this exact shape — note the
doc-comment rationale (testable without live external dependency) applies even more directly to
capture, since capture has no live external dependency at all (it is pure config-driven, unlike
SPIFFE's SPIRE-agent-connect dependency).

**Predicate-over-routes pattern** (`route.rs:293-309`, verified live, full function):
```rust
    /// Check whether `host_port` (e.g. `"api.openai.com:443"`) matches a
    /// loaded route that ALSO declares a SPIFFE auth source
    /// (`has_spiffe_source()`). Mirrors `is_route_upstream`'s exact matching
    /// logic, additionally requiring the SPIFFE condition — this is the
    /// cheap, unit-testable proof surface D-01's ADR obligation and Plan
    /// 113-05's D-03 request-time guard both need.
    #[must_use]
    pub fn spiffe_declared_for_upstream(&self, host_port: &str) -> bool {
        let normalised = host_port.to_lowercase();
        self.routes.values().any(|route| {
            route.has_spiffe_source()
                && route
                    .upstream_host_port
                    .as_ref()
                    .is_some_and(|hp| host_port_matches(hp, &normalised))
        })
    }
```
`capture_declared_for_upstream()` is this exact shape with `has_capture_source()` substituted.
**Open design question, flagged not resolved (RESEARCH.md Pitfall 4, still applicable):** this
uses exact `host:port` matching via `host_port_matches()` (`route.rs:354`, wildcard-aware but still
port-specific once a concrete port is given). Upstream's own `3c59c62e` hardened its equivalent
`host_policy()` check from exact-port to host-only-any-port matching after finding a capture-host
reachable on an unconfigured port could slip past. Whether `capture_declared_for_upstream()` should
diverge from the SPIFFE precedent and match host-only is a genuine open call for ADR-114 to record
explicitly, not silently inherit.

**`is_route_upstream()` for comparison (already-unconditional CONNECT block)** (`route.rs:273-281`,
verified live):
```rust
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
```

---

### `crates/nono-proxy/src/audit.rs` — `capture_context` field

**Analog:** `EventContext.spiffe_context`.

**Struct + derive pattern** (`audit.rs:33-49`, verified live, full text):
```rust
/// Optional structured audit context attached to a proxy event.
///
/// Composed into network-event rows by the `log_allowed` / `log_denied` /
/// `log_l7_request` emitters. Per upstream `9300de9` (v0.51.0) — fork-side D-20
/// manual replay against the Phase 22-05a / Phase 23 REQ-AUD-05 audit envelope.
/// All fields are optional; default-constructed context contributes no
/// additional ledger entries and preserves prior-Phase emission shape.
#[derive(Debug, Clone, Default)]
pub struct EventContext<'a> {
    pub route_id: Option<&'a str>,
    pub auth_mechanism: Option<NetworkAuditAuthMechanism>,
    pub auth_outcome: Option<NetworkAuditAuthOutcome>,
    pub managed_credential_active: Option<bool>,
    pub injection_mode: Option<NetworkAuditInjectionMode>,
    pub denial_category: Option<NetworkAuditDenialCategory>,
    pub spiffe_context: Option<SpiffeAuditContext>,
}
```
`#[derive(Default)]` confirmed live — adding `pub capture_context: Option<CaptureAuditContext>,`
is safely additive, no other construction site needs updating except the (multiple)
`spiffe_context: ctx.spiffe_context.clone(),`-shaped propagation lines already present at
`audit.rs:167,226,289,330` — a `capture_context: ctx.capture_context.clone(),` line must be added
alongside each, not just the struct definition.

---

### `crates/nono/src/undo/types.rs` — `CaptureAuditContext` (pure data) + denial-category variant

**Analog:** `SpiffeAuditContext` / `SpiffeDelegationContext` / `NetworkAuditDenialCategory::SpiffeUnsupportedPath`.

**Denial-category enum, exact block to extend** (`types.rs:241-258`, verified live, full text):
```rust
/// Structured category for denied proxy events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditDenialCategory {
    AuthenticationFailed,
    EndpointPolicy,
    ManagedCredentialUnavailable,
    HostDenied,
    InterceptHandshakeFailed,
    UpstreamConnectFailed,
    ConnectBypassesL7,
    ExternalProxyRejected,
    /// Route declared SPIFFE-authenticated but arrived on a proxy path with
    /// no SPIFFE implementation (D-03 fail-closed guard; the guard itself
    /// lands in Plan 113-05 — this variant is added ahead of need so that
    /// plan does not require a second edit to this enum).
    SpiffeUnsupportedPath,
}
```
Add a `CaptureUnsupportedPath` (or planner-chosen name) variant here, following the same
doc-comment convention (explain WHY the category exists, cite the guard it serves).

**Pure-data audit-context struct, exact block to mirror** (`types.rs:276-299`, verified live, full
text):
```rust
/// SPIFFE audit context attached to a network audit event when a request
/// used SPIFFE/SPIRE workload-identity auth. Threaded into
/// `NetworkAuditEvent::spiffe_context`.
///
/// Pure data — records what happened (workload identity, trust domain,
/// delegation chain), applies no enforcement or policy evaluation. See
/// ADR-86 / D-08 (113-CONTEXT.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiffeAuditContext {
    pub workload_spiffe_id: String,
    pub trust_domain: String,
    pub svid_type: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_spiffe_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<SpiffeDelegationContext>,
}
```
`CaptureAuditContext` must follow the same "pure data, no branching logic" posture (ADR-86/D-09) —
CONTEXT.md's hard constraint that it "must never contain a raw token" maps directly onto this
struct never having a `real_token`/`Zeroizing<...>` field; it should carry only identifiers (e.g.
which route/provider, which response field was rewritten, phantom ID) — never token bytes,
mirroring how `SpiffeAuditContext` carries only IDs and never the JWT itself.

**Verification note for SC3/D-09 (`grep -rn "if \|match " crates/nono/src/undo/types.rs`):**
`SpiffeAuditContext`/`SpiffeDelegationContext` contain zero `if`/`match` — pure struct/derive only.
`CaptureAuditContext` must land the same way; any branching logic (e.g. "is this field
token-shaped?") belongs in `nono-proxy::capture`, never in this core-crate file.

---

### `crates/nono-cli/src/profile/mod.rs` — `CustomCredentialDef.capture` + validation

**Analog:** `CustomCredentialDef.spiffe` field + `validate_custom_credential()`'s SPIFFE
mutual-exclusion arm.

**Struct field pattern** (`mod.rs:1032-1035`, verified live):
```rust
    /// SPIFFE/SPIRE Workload API auth. Mutually exclusive with
    /// `credential_key`, `auth`, and `aws_auth`.
    #[serde(default)]
    pub spiffe: Option<nono_proxy::config::SpiffeAuthConfig>,
```
Note: unlike `config.rs`'s `RouteConfig.spiffe` (which uses
`#[serde(default, skip_serializing_if = "Option::is_none")]`), this CLI-side copy uses only
`#[serde(default)]` — both are valid live patterns in the same codebase; match whichever sibling
field (`spiffe` here) your new `capture` field sits nearest to, for diff-minimalism.

**Mutual-exclusion validation, exact block to extend or NOT extend** (`mod.rs:1131-1173`, verified
live, full function body up to the "at least one" check):
```rust
fn validate_custom_credential(name: &str, cred: &CustomCredentialDef) -> Result<()> {
    // Mutual exclusion: aws_auth is incompatible with credential_key and auth.
    if cred.aws_auth.is_some() && (cred.credential_key.is_some() || cred.auth.is_some()) {
        return Err(NonoError::ProfileParse(format!(
            "custom credential '{}' has 'aws_auth' set together with 'credential_key' or 'auth'; \
             aws_auth is mutually exclusive with both — remove the other auth field",
            name
        )));
    }

    // PROF-03 (Phase 22): Mutual exclusion of credential_key and auth.
    if cred.credential_key.is_some() && cred.auth.is_some() {
        return Err(NonoError::ProfileParse(format!(
            "custom credential '{}' has both 'credential_key' and 'auth' set; \
             these are mutually exclusive — use one or the other",
            name
        )));
    }

    // NET-02 (Phase 113): spiffe is mutually exclusive with every other auth
    // mechanism on the same custom credential.
    if cred.spiffe.is_some()
        && (cred.credential_key.is_some() || cred.auth.is_some() || cred.aws_auth.is_some())
    {
        return Err(NonoError::ProfileParse(format!(
            "custom credential '{}' has 'spiffe' set together with 'credential_key', 'auth' \
             (oauth2), or 'aws_auth'; spiffe is mutually exclusive with all other auth fields",
            name
        )));
    }

    // At least one of credential_key, auth, aws_auth, or spiffe must be set.
    if cred.credential_key.is_none()
        && cred.auth.is_none()
        && cred.aws_auth.is_none()
        && cred.spiffe.is_none()
    {
        return Err(NonoError::ProfileParse(format!(
            "custom credential '{}' must have either 'credential_key', 'auth', 'aws_auth', or \
             'spiffe' set",
            name
        )));
    }
```
**Open design question (RESEARCH.md, not resolved here, flagged for the planner):** capture is
response-direction (what the route returns), the other four fields are request-direction (what
credential the proxy injects outbound). A route may plausibly need BOTH a `credential_key`
(different, already-authenticated endpoint) and a `capture` config (its token endpoint), or capture
may need to stand alone with no other auth field (the whole point being the client has no prior
credential yet). Do not add `capture` to the existing mutual-exclusion `if` block by reflexive
analogy — decide explicitly (and record the decision + reasoning in ADR-114) whether capture
composes with the other four or needs its own "at least one of" rule.

---

### `crates/nono-cli/src/profile/mod.rs` — schema round-trip test

**Analog:** `test_schema_validates_spiffe_custom_credential`, verified live at **lines 7396-7415**
(RESEARCH.md cited 7396-7434, which included the *next* test too — corrected range here is the
single test only).

**Exact template to copy** (`mod.rs:7396-7415`, verified live, full test):
```rust
    #[test]
    fn test_schema_validates_spiffe_custom_credential() {
        let json = r#"{
            "meta": { "name": "spiffe-test" },
            "network": {
                "custom_credentials": {
                    "inventory": {
                        "upstream": "https://inventory.internal.example",
                        "spiffe": {
                            "type": "jwt",
                            "workload_api_socket": "/run/spire/agent.sock",
                            "audience": ["x"]
                        }
                    }
                }
            }
        }"#;
        validate_against_schema(json)
            .expect("custom credential with a valid spiffe block should pass schema validation");
    }
```

**Note — TWO `validate_against_schema` helpers exist in this file, not one:**
- `mod.rs:7249` — private to the `mod tests` block that contains the test above (verified: the
  test at 7397 is inside a module whose local helper starts at 7249).
- `mod.rs:9465-9482` — a second, explicitly-labelled duplicate: `/// Local copy of the `mod tests`
  schema-validation helper (that one is private to its own module)`. Verified live, full function:
```rust
    fn validate_against_schema(json_str: &str) -> std::result::Result<(), String> {
        let schema_str = crate::config::embedded::embedded_profile_schema();
        let schema: serde_json::Value =
            serde_json::from_str(schema_str).expect("schema is valid JSON");
        let instance: serde_json::Value =
            serde_json::from_str(json_str).expect("instance is valid JSON");
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let errors: Vec<_> = validator.iter_errors(&instance).collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors
                .iter()
                .map(|e| format!("{} at {}", e, e.instance_path()))
                .collect::<Vec<_>>()
                .join("; "))
        }
    }
```
Whichever `mod tests` block the planner puts the new capture schema test in determines which
`validate_against_schema` copy it resolves against — both are functionally identical, but the
planner should place the new test in the SAME `mod tests` block as
`test_schema_validates_spiffe_custom_credential` (the one whose helper is at line 7249) unless
there's a reason to use the other, to keep capture's tests co-located with its closest sibling.

---

### `crates/nono-cli/data/nono-profile.schema.json` — `CaptureConfig` `$defs`

**Analog:** `$defs.SpiffeAuthConfig` + the `spiffe` property block on `CustomCredentialDef`'s
schema entry.

**Property-block pattern to mirror** (schema file, lines 645-651, verified live):
```json
        "spiffe": {
          "oneOf": [
            { "$ref": "#/$defs/SpiffeAuthConfig" },
            { "type": "null" }
          ],
          "description": "SPIFFE/SPIRE Workload API auth (NET-02, Phase 113). When present, the proxy fetches a JWT-SVID from the local SPIRE Workload API and injects it as a bearer token. Mutually exclusive with `credential_key`, `auth`, and `aws_auth`."
        },
```

**`$defs` entry pattern to mirror** (schema file, lines 754-758+, verified live, opening):
```json
    "SpiffeAuthConfig": {
      "type": "object",
      "description": "NET-02 (Phase 113) SPIFFE/SPIRE Workload API auth for a credential route. The proxy fetches a JWT-SVID from the local SPIRE Workload API (via a Unix domain socket) and injects it as a bearer token (or custom header). The sandboxed child never sees the socket directly — see `enforce_spiffe_socket_isolation()` in nono-cli.",
      "additionalProperties": false,
      "required": ["type", "workload_api_socket", "audience"],
      "properties": {
```
`additionalProperties: false` confirmed as the enforced posture (D-13's hard requirement). A
`CaptureConfig` `$defs` entry should follow this exact shape: `type: object`,
`additionalProperties: false`, a `required` array naming only the fields the fork actually
implements (D-13 explicitly warns against porting upstream's full 265-line schema — "a schema
advertising configuration nothing implements is itself a fail-open shape").

---

### `../nono-py/src/proxy.rs` and `../nono-py/src/policy.rs` — `RouteConfig` struct-literal fix

**Analog:** the existing `spiffe: None,` line already present in both exhaustive struct literals
(the Phase 113 fix site).

**`proxy.rs`, exact literal to extend** (`../nono-py/src/proxy.rs:211-232`, verified live):
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
                spiffe: None,
                endpoint_rules: endpoint_rules
                    .into_iter()
                    .map(|(method, path)| RustEndpointRule { method, path })
                    .collect(),
                tls_ca,
                endpoint_policy: None,
            },
        }
```

**`policy.rs`, exact literal to extend** (`../nono-py/src/policy.rs:741-762`, verified live):
```rust
impl From<PolicyRouteConfig> for RustRouteConfig {
    fn from(route: PolicyRouteConfig) -> Self {
        Self {
            prefix: route.prefix,
            upstream: route.upstream,
            credential_key: route.credential_key,
            inject_mode: route.inject_mode.into(),
            inject_header: route.inject_header,
            credential_format: Some(route.credential_format),
            path_pattern: route.path_pattern,
            path_replacement: route.path_replacement,
            query_param_name: route.query_param_name,
            env_var: route.env_var,
            oauth2: None,
            aws_auth: None,
            spiffe: None,
            endpoint_rules: route.endpoint_rules.into_iter().map(Into::into).collect(),
            tls_ca: route.tls_ca,
            endpoint_policy: None,
        }
    }
}
```
Both sites need a new `capture: None,` line (or the planner-chosen field name) added directly
below `spiffe: None,` — this is the "fifth consecutive occurrence" D-14 names. Confirmed live:
`nono-ts` has **zero** `nono_proxy` references anywhere under `../nono-ts/src` (grep returns no
files) — matches RESEARCH.md's claim that `nono-ts` is structurally immune to this class of break;
still run its build per D-14 to report a real (expected-green) result, not to fix anything.

---

### `proj/ADR-114-oauth-capture-disposition.md` (new doc)

**Analog:** `proj/ADR-113-spiffe-disposition.md` (structure — 8 top-level `##` sections verified
live: `Context`, `Decision` with `D-01's positive proof` subsection, `D-05: Dependency Review`,
`D-08/SC3: Re-confirming the ADR-86 Boundary`, `OD-1: The SPIFFE-Only Scope Boundary...` — the
"named permanent decision" shape — `D-07: Loud-Skip Reality`, `Consequences`, `References`).

ADR-114 should mirror at minimum:
- A `Context` section that states the retracted-decline history (D-01r) as part of the record —
  CONTEXT.md's `<specifics>` block explicitly requires this: "A future reader who finds only the
  ADAPT will otherwise re-derive the same wrong premise from `112-OAUTH-CAPTURE-DISPOSITION.md`."
- A `Decision` section with a positive-proof subsection analogous to ADR-113's "D-01's positive
  proof — no fork route type is left silently unauthenticated" (`ADR-113-spiffe-disposition.md:46`),
  here proving D-06's cross-path guard covers all four arrival paths (reverse-proxy/handled;
  CONNECT/already-blocked; forward-HTTP/newly-guarded; external-proxy-chain — confirm this fourth
  path explicitly, it is not covered by RESEARCH.md's architecture diagram in explicit detail).
- An `OD-1`-shaped section naming `149abde0` (TLS interception) as the tracked, permanently-declined
  boundary — mirrors ADR-113's `OD-1: The SPIFFE-Only Scope Boundary Against b1ecbc02`
  (`ADR-113-spiffe-disposition.md:219`).
- A `D-05: Dependency Review`-shaped section is likely NOT needed verbatim (this phase adds zero new
  crates per RESEARCH.md's Package Legitimacy Audit) — but the SC3/ADR-86 boundary re-confirmation
  section (ADR-113's `D-08/SC3`, `ADR-113-spiffe-disposition.md:184`) IS directly applicable per
  this phase's own D-09.

---

### `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — SEC-02 carry-forward note

**Analog:** "SPIFFE Carry-Forward Note (Phase 113, D-02)", verified live at
`108-DIVERGENCE-LEDGER.md:659-691+` (full section read).

**Structure to mirror** (opening of the analog section, verified live):
```markdown
### SPIFFE Carry-Forward Note (Phase 113, D-02)

**Filed:** 2026-08-06 (Phase 113, Plan 113-08 close-out; premise re-confirmed live via direct
`grep`/`git log -S` reads during Plans 113-01 through 113-08).

Upstream `c831dade...` (#1272, "feat(proxy): add SPIFFE/SPIRE workload identity auth for
upstream routes") includes two `tls_intercept/` hunks — ...

**This fork does not currently have a `tls_intercept/` module for that handler to land on.**
...

**Obligation for whichever future plan introduces a `tls_intercept/` module:** ...
```
The Phase 114 note should be titled e.g. `### SEC-02 Carry-Forward Note (Phase 114, D-10)`, name
`149abde0` explicitly (mirroring how this section names `c831dade`), state the "obligation for
whichever future plan introduces TLS interception" in the same forward-looking form, and land as a
**new subsection**, not a rewrite of the existing SEC-02 rows at `108-DIVERGENCE-LEDGER.md:1800-1802`
(those three table rows should be updated in place from "deferred -> Phase 114" to
"ADAPTED-with-scope-limit — see SEC-02 Carry-Forward Note", per D-10/D-11).

---

## Shared Patterns

### Fail-closed, never-partial-forward
**Source:** `reverse.rs:388-390` (`send_error(stream, 413, "Payload Too Large").await?; return
Ok(());` — an early `return`, no fallthrough to the streaming loop below it) and the D-06 guard's
`return Ok(());` after writing a 403 (`server.rs:1076`).
**Apply to:** every fail-closed branch in the new buffer-and-rewrite helper (cap exceeded,
`Content-Encoding` present, unrewritten token-shaped field found) — every one must be an explicit
early-return deny, never a fallthrough to the unbuffered streaming path. RESEARCH.md's
Anti-Patterns section states this as a named risk ("Chunked-vs-buffered ambiguity").

### Zeroize for secret buffers, plain `Vec<u8>` for non-secret buffers
**Source:** `reverse.rs:33,443,735,993` (`Zeroizing<String>`/`Zeroizing<Vec<u8>>` for
credential/request-build buffers) vs. `reverse.rs:392-401`'s `body: Vec<u8>` (deliberately
unzeroized — request bodies, not credentials).
**Apply to:** the new response accumulator in `reverse.rs` (Zeroizing — holds a real OAuth token)
and `capture.rs`'s phantom store's `real` field (Zeroizing) vs. its `phantom` key (plain String,
not secret).

### `#[must_use]` + doc-comment-with-rationale on predicate accessors
**Source:** `route.rs:81-90`'s `has_spiffe_source()`, `route.rs:293-309`'s
`spiffe_declared_for_upstream()` — both `#[must_use]`, both have a doc comment explaining WHY the
predicate is cheap/safe/unit-testable, not just WHAT it returns.
**Apply to:** `has_capture_source()` and `capture_declared_for_upstream()`.

### Audit context is pure data, never branching logic, in the core `nono` crate
**Source:** `crates/nono/src/undo/types.rs`'s `SpiffeAuditContext`/`SpiffeDelegationContext` —
confirmed zero `if`/`match` in either struct definition.
**Apply to:** `CaptureAuditContext` — enforced by D-09/SC3's mandatory ADR-86 boundary check
(`grep -rn "if \|match " crates/nono/src/undo/types.rs`, per RESEARCH.md's Validation Architecture
table).

### `#[serde(default, skip_serializing_if = "Option::is_none")]` for optional declarative config
**Source:** `config.rs:621-622` (`RouteConfig.spiffe`).
**Apply to:** `RouteConfig.capture` in `config.rs`. Note `mod.rs:1034` (CLI-side
`CustomCredentialDef.spiffe`) uses only `#[serde(default)]` without the `skip_serializing_if` — the
two sibling structs are NOT byte-identical in their serde attributes; match whichever struct you're
extending, not a single canonical form.

---

## No Analog Found

None — every file in the CONTEXT.md/RESEARCH.md scope has a direct, verified analog in the current
tree (this phase is explicitly designed by CONTEXT.md's D-07/D-12 to mirror Phase 113's file-set
almost 1:1).

---

## Open Design Questions Surfaced During Pattern Mapping (not resolved here — planner/ADR-114 must decide)

1. **Mutual exclusion:** does `capture` on `CustomCredentialDef`/`RouteConfig` need the same
   mutual-exclusion treatment as `spiffe`/`aws_auth`/`auth` (RESEARCH.md Assumption A3)? The
   analog's mutual-exclusion `if` block (`mod.rs:1150-1160`) should NOT be extended by reflexive
   copy — confirm the semantic question first.
2. **Host-match granularity:** should `capture_declared_for_upstream()` use exact `host:port`
   matching (mirroring `spiffe_declared_for_upstream()`'s live shape at `route.rs:300-309`) or
   host-only-any-port matching (mirroring upstream's own `3c59c62e` hardening lesson for the
   analogous `host_policy()` check)? RESEARCH.md Pitfall 4 flags this as genuinely unresolved.
3. **`validate_against_schema` duplication:** two copies of this helper exist in
   `crates/nono-cli/src/profile/mod.rs` (lines 7249 and 9465-9482) — not a capture-specific issue,
   but the planner should note which `mod tests` block the new capture schema test lands in.

## Metadata

**Analog search scope:** `crates/nono-proxy/src/` (spiffe.rs, reverse.rs, server.rs, route.rs,
config.rs, audit.rs, token.rs), `crates/nono/src/undo/types.rs`, `crates/nono-cli/src/profile/mod.rs`,
`crates/nono-cli/data/nono-profile.schema.json`, `../nono-py/src/{proxy,policy}.rs`,
`../nono-ts/src/` (negative-match confirmation), `proj/ADR-113-spiffe-disposition.md`,
`.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`.
**Files scanned/read this pass:** 13 (all listed above; all excerpts re-verified live, not
trusted from RESEARCH.md's line numbers alone — one drift corrected: the schema round-trip test
template range was 7396-7415, not 7396-7434 as RESEARCH.md cited, because that range included a
second, different test).
**Pattern extraction date:** 2026-08-06
