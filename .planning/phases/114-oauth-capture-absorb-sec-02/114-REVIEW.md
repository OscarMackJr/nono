---
phase: 114-oauth-capture-absorb-sec-02
reviewed: 2026-08-07T00:00:00Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - crates/nono-proxy/src/capture.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono-proxy/src/route.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/audit.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/lib.rs
  - crates/nono-proxy/Cargo.toml
  - crates/nono-proxy/tests/spiffe_integration.rs
  - crates/nono/src/undo/types.rs
  - crates/nono/src/audit.rs
  - crates/nono-cli/src/profile/credential_provider.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/proxy_command.rs
  - crates/nono-cli/src/audit_integrity.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/data/nono-profile.schema.json
  - ../nono-py/src/proxy.rs
  - ../nono-py/src/policy.rs
  - ../nono-py/src/undo.rs
  - proj/ADR-114-oauth-capture-disposition.md
findings:
  critical: 3
  warning: 9
  info: 6
  total: 18
status: issues_found
---

# Phase 114: Code Review Report

**Reviewed:** 2026-08-07
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Summary

The enforcement point itself (`relay_response_with_capture`) is genuinely fail-closed on its
eight enumerated branches, and all three reverse-proxy relay sites *are* wired to it with the
guard placed before each site's `[0u8; 8192]` streaming loop — I traced each one and could not
find a branch that writes the original body or falls through to streaming. The site-1/2/3 claim
in ADR-114 holds.

What does not hold is everything around that core:

1. **A real token can reach the operator's log and the persisted audit ledger** through the
   chunked-decode error path, which interpolates raw response bytes into the deny reason
   (`CR-01`). This directly contradicts the phase's D-08 claim ("never written to disk") and
   `capture.rs`'s module doc ("never included in any Debug/Display/log/audit output").
2. **The capture path only terminates on upstream EOF.** The proxy never sends `Connection:
   close`, never honours `Content-Length`/chunked framing to stop reading, and forwards the
   client's own `Connection:` header verbatim. Against any spec-compliant HTTP/1.1 token endpoint
   (all of them keep the connection alive), every capture request stalls for 30 s and then returns
   502 (`CR-02`). All six relay tests and the "live end-to-end" test mask this by explicitly
   closing the upstream write half.
3. **The CONNECT arm's guard is narrower than the phase claims.** `capture_declared_for_upstream()`'s
   deliberate host-only widening — the phase's headline D-06 hardening — is *dead code* on the
   CONNECT path, which is gated by exact-`host:port` `is_route_upstream()`. Since the realistic
   token endpoint is `https`, CONNECT is the only bypass path that matters, and it is the one left
   with the narrower gate (`CR-03`). ADR-114's table row and the inline comment at
   `server.rs:1385-1395` both assert the opposite.

Secondary: the "always held in `Zeroizing`" claim is false at several points on the real data path;
the fail-closed backstop is exact-name and case-sensitive so `accessToken` walks straight through;
the phantom store never evicts; the capture path silently drops the per-request audit records the
non-capture paths emit; and the `nono-py` decoder rejects the two denial-category strings its own
encoder produces.

---

## Critical Issues

### CR-01: Real OAuth token can be written to the log and the persisted audit ledger via the chunked-decode error path

**File:** `crates/nono-proxy/src/reverse.rs:1565-1573` (producer), `:1820-1828` (deny call),
`crates/nono-proxy/src/audit.rs:189-197,235` (log + persist)

**Issue:** `decode_chunked_body()` builds its chunk-size parse error from the raw response bytes:

```rust
let size_hex = std::str::from_utf8(&body[pos..line_end])? .split(';').next().unwrap_or("").trim();
let size = usize::from_str_radix(size_hex, 16).map_err(|e| {
    ProxyError::HttpParse(format!("malformed chunk size '{size_hex}': {e}"))
})?;
```

`line_end` is the offset of the first `\r\n` in the body — nothing bounds it. If an upstream
declares `Transfer-Encoding: chunked` but sends an identity body (a real, observed class of broken
gateway/HTTP-1.0-intermediary behaviour, and trivially inducible by a hostile upstream), `size_hex`
is *the entire first line of the JSON body*, i.e. `{"access_token":"<REAL TOKEN>",...`.

That string then flows into `deny(e.to_string())` at `:1824`, which does:
- `warn!("OAuth capture: denying response for route '{}': {}", route_id, reason)` — plaintext token
  in the tracing log / log file;
- `audit::log_denied(..., &reason)` → `info!(target: "nono_proxy::audit", reason = reason, ...)`
  **and** `NetworkAuditEvent { reason: Some(reason.to_string()), .. }`, which is pushed into the
  session audit log and serialised into `SessionMetadata.network_events` **on disk**.

This falsifies three explicit records: D-08 ("real tokens are held in-memory, session-scoped only —
never written to disk"), `capture.rs`'s module doc ("never included in any Debug/Display/log/audit
output"), and ADR-114 Consequence 2 ("no secret at rest"). If the session audit directory is
readable by the sandboxed child (it lives under the nono state dir), this is not merely a hygiene
failure — it is a direct route for the confined agent to read the real token back out, defeating
the phase's entire security property.

Note that the `capture_store_holds_only_in_memory` test cannot catch this: it string-scans
`capture.rs` only, and the write happens in `reverse.rs`/`audit.rs`.

**Fix:** Never interpolate response bytes into an error that reaches a log or an audit record.

```rust
let size = usize::from_str_radix(size_hex, 16).map_err(|_| {
    ProxyError::HttpParse(format!(
        "malformed chunk size at byte offset {pos} (len {} bytes); value redacted",
        line_end - pos
    ))
})?;
```

Additionally, harden the enforcement point so no upstream-derived string can ever reach `deny()`:
map every `Err` from steps 4/5 to a fixed, content-free reason string and log the detailed error at
`debug!` only (or not at all).

---

### CR-02: Capture relay only terminates on upstream EOF — every capture request against a keep-alive upstream stalls 30 s and then 502s

**File:** `crates/nono-proxy/src/reverse.rs:1662-1694` (`read_capped_response`), `:1777` (call site),
`:1239-1264` (`filter_headers` does not strip `Connection`), `:467-494` (request build, no
`Connection: close`)

**Issue:** `read_capped_response` loops until `read()` returns `Ok(0)`, i.e. until the upstream
closes the TLS connection. Nothing else terminates the read: `Content-Length` is not consulted, the
chunked terminator (`0\r\n\r\n`) is not detected, and the request the proxy sends upstream contains
no `Connection: close`. Worse, `filter_headers()` strips only `Host`, `Content-Length`,
`Proxy-Authorization` and the credential header — the client's own `Connection:` header (a
hop-by-hop header for the client↔proxy hop that must not be forwarded at all) is passed through, so
a normal `Connection: keep-alive` client actively asks the upstream to hold the connection open.

Every real OAuth token endpoint (Okta, Auth0, Google, Entra, GitHub) is HTTP/1.1 with persistent
connections. Consequence: the capture path buffers the full response, then blocks until
`CAPTURE_READ_TIMEOUT` (30 s) elapses, then takes the timeout branch and denies with 502. **The
feature does not work against any spec-compliant provider**, and every attempt pins a task,
a TLS connection and up to `cap` bytes for 30 s — an amplification lever a sandboxed agent can
trivially drive in a loop.

The test suite cannot see this: `run_relay_with_capture` (`:2215`) and
`run_relay_capture_if_declared` (`:2450`) both `drop(upstream_client)` to force EOF, and the
"live end-to-end" `spawn_hermetic_tls_upstream` (`:2828`) calls `tls.shutdown()` after writing. The
`non_capture_route_still_streams_unbuffered` control does the same. The property "the relay
terminates" is never exercised against a persistent connection.

**Fix:** Two changes, both needed:

1. Terminate the read on HTTP framing, not EOF. After the header block is available, honour
   `Content-Length` (read exactly `body_start + len` bytes) or, when `Transfer-Encoding: chunked`,
   stop at the `0\r\n\r\n` terminator — still bounded by `cap` and `read_timeout`, still fail-closed
   when neither framing is present (deny rather than block).
2. Strip hop-by-hop headers (`Connection`, `Keep-Alive`, `TE`, `Upgrade`, `Trailer`,
   `Transfer-Encoding`) in `filter_headers()`, and emit `Connection: close` on capture-declared
   routes' upstream requests as a belt-and-braces backstop:

```rust
if route.capture.is_some() {
    request.push_str("Connection: close\r\n");
}
```

Add a regression test whose fake upstream writes the response and then **stays open**, asserting the
client receives the rewritten body promptly rather than a 502 after the timeout.

---

### CR-03: CONNECT dispatch never applies the host-only capture guard — a capture-declared host on any other port tunnels through unrewritten

**File:** `crates/nono-proxy/src/server.rs:1362-1431`; `crates/nono-proxy/src/route.rs:371-380,469-487`

**Issue:** The CONNECT deny is gated by:

```rust
if state.route_store.is_route_upstream(&host_port) {   // :1378 — exact host:port
    ...
    let is_capture_route = state.route_store.capture_declared_for_upstream(&host_port); // :1396
```

`is_route_upstream` uses `host_port_matches` (exact port). `capture_declared_for_upstream` — the
host-only predicate this phase deliberately built to be *broader*, with `host_only_matches` and a
"Do NOT 'fix' this to match the SPIFFE precedent's exact-port behavior — narrowing it re-opens the
exact bypass class `3c59c62e` closed" comment on it — is evaluated **inside** that narrower gate.
It can therefore only ever be true where the exact match already fired. Its widening is dead on this
path; it only relabels the audit `denial_category`.

Consequence: `CONNECT capture-host.example.com:8443` (or any port other than the one configured on
the route) is not blocked. It falls through to `connect::handle_connect`, gets a raw TLS tunnel,
and the sandboxed agent completes the token exchange itself and receives the **real** OAuth token
with no rewrite. Because a real token endpoint is `https`, CONNECT is the only bypass path that
actually matters here — the plain-HTTP forward guard (`:1096`), which *does* apply the host-only
predicate, cannot reach an `https` upstream at all.

This also makes two records inaccurate, which is itself a finding for a project twice burned by
inaccurate records:
- `server.rs:1386-1388`: "`is_route_upstream` above already blocks CONNECT to every route upstream
  unconditionally" — false for any port other than the configured one.
- ADR-114's D-06 table, CONNECT row: "the external-proxy-chain arm is unreachable for a
  capture-declared upstream by construction" and "**Denied — fail-closed, no new deny surface**" —
  both overstate what the code does.

**Fix:** Widen the outer gate so the host-only predicate is actually load-bearing:

```rust
let is_spiffe_route = state.route_store.spiffe_declared_for_upstream(&host_port);
let is_capture_route = state.route_store.capture_declared_for_upstream(&host_port);
if state.route_store.is_route_upstream(&host_port) || is_spiffe_route || is_capture_route {
    // ... existing denial, now reachable for a capture host on a non-configured port
}
```

Add a test mirroring `forward_http_denies_capture_declared_route_upstream_on_different_port` for the
CONNECT path (declare the route at `:9443`, `CONNECT capture-upstream.invalid:443`, assert 403 +
`CaptureUnsupportedPath`), and correct the ADR row and the inline comment.

---

## Warnings

### WR-01: Real tokens exist in multiple non-zeroized heap copies despite the "always held in `Zeroizing`" claim

**File:** `crates/nono-proxy/src/reverse.rs:1667-1685,1830,1835`; `crates/nono-proxy/src/capture.rs:192-198,320-327`

**Issue:** `capture.rs`'s module doc states "Real token bytes are always held in
`zeroize::Zeroizing`". On the actual data path they are not:

- `read_capped_response` reads into `let mut buf = [0u8; 8192];` — a plain stack buffer holding
  token bytes, never zeroed (`:1668`).
- `raw` is `Zeroizing<Vec<u8>>` but is grown with `extend_from_slice` from zero capacity. Every
  reallocation leaves the previous (token-bearing) allocation in freed heap, unzeroed; `Zeroizing`
  only wipes the final buffer (`:1667,1685`).
- `raw_body.to_vec()` (`:1830`) and `decode_chunked_body`'s output are plain `Vec<u8>` copies of
  the token-bearing body.
- `serde_json::from_slice::<Value>` (`:1835`) allocates a plain `String` per token; the rewrite at
  `capture.rs:198` (`*real_str = phantom_value`) drops that `String` without zeroing it.
- Egress: `capture.rs:323-326` does `String::from_utf8(real.to_vec())` (two plain copies) and
  writes the result into a plain `Value`, then `serde_json::to_vec` produces a plain `Vec<u8>`
  which `handle_reverse_proxy` writes to the wire and drops unzeroed.

CLAUDE.md mandates `zeroize` for sensitive data in memory. The gap is exposure via core dump, swap,
or heap-grooming, not an immediate leak — but the *documentation asserting the opposite* is what
makes this a real defect: a future reader will trust the module doc.

**Fix:** Pre-allocate `raw` with `Vec::with_capacity(cap)` so it never reallocates; zeroize the read
scratch buffer (`buf.zeroize()` after the loop, or read directly into `raw`'s spare capacity);
and either soften the module doc to describe what is actually guaranteed (store entries only) or
route the parse/rewrite through a zeroizing wrapper. At minimum, correct the doc so it stops
asserting a property the code does not have.

### WR-02: The fail-closed backstop is exact-name and case-sensitive — `accessToken` walks straight through

**File:** `crates/nono-proxy/src/capture.rs:158-160,217-256`

**Issue:** `is_sensitive_token_field` is `matches!(field, "access_token" | "refresh_token" |
"id_token")`. `reject_unrewritten_token_fields` is documented and cited (ADR-114, D-07) as "the last
line of defense against provider-config drift". It misses, silently:

- case/format variants: `accessToken`, `AccessToken`, `Access_Token`, `ACCESS_TOKEN` (the camelCase
  form is extremely common in real provider payloads);
- other token names: `token`, `session_token`, `authentication_token`, `client_secret`, `api_key`;
- non-string values: `{"access_token": {"value": "REAL"}}` — the key matches but `val` is not
  `Value::String`, so no error is raised and the recursion descends into a sub-object whose keys
  don't match, releasing the real token;
- literal-dot keys: `{"data.access_token": "REAL"}` is unreachable by `value_at_path_mut` (so
  never rewritten) and its key does not match the sensitive set (so never rejected).

Arrays *are* handled fail-closed (a token under `tokens[0].access_token` can never be in
`configured_paths`, so it always denies) — that part is correct.

**Fix:** Match case-insensitively on a normalised key (strip `_`/`-`, lowercase) against a broader
name set, and treat a sensitive-named key holding a **non-string, non-null** value as a denial
rather than a skip:

```rust
fn is_sensitive_token_field(field: &str) -> bool {
    let norm: String = field.chars().filter(|c| *c != '_' && *c != '-')
        .flat_map(char::to_lowercase).collect();
    matches!(norm.as_str(),
        "accesstoken" | "refreshtoken" | "idtoken" | "token" | "sessiontoken" | "clientsecret")
}
```

If the narrow set is a deliberate upstream-parity choice, say so in the doc comment and stop
describing it as a general drift backstop.

### WR-03: Both capture guards are bypassable by trailing-dot / non-canonical host spellings

**File:** `crates/nono-proxy/src/route.rs:371-380,469-487`; `crates/nono-proxy/src/server.rs:1060,1366-1378`

**Issue:** Guard matching is byte equality on the lowercased host. DNS treats `api.example.com.`
(fully-qualified, trailing root dot) as identical to `api.example.com`, and `url::Url` preserves the
trailing dot in `host_str()`. So:

- forward-HTTP: `GET http://api.example.com./token` → `host_port = "api.example.com.:80"` →
  `capture_declared_for_upstream` false → guard bypassed;
- CONNECT: `CONNECT api.example.com.:443` → `is_route_upstream` false → tunnel established.

Precondition for an actual leak is that `ProxyFilter` permits the host —
`net_filter.rs::check_host` is also exact/suffix match, so it denies the dotted form when an
allowlist is configured, but allows it in the default `allowed_hosts: []` + `strict_filter: false`
allow-all configuration (`config.rs:39-45`). Under allow-all the request proceeds and the real token
is relayed unrewritten.

**Fix:** Normalise both sides before comparison, in `extract_host_port` and at each guard call site:

```rust
fn canonical_host(h: &str) -> String {
    h.trim_end_matches('.').to_ascii_lowercase()
}
```

Apply it in `route.rs::extract_host_port`, `host_only_matches`, `host_port_matches`, and to the
`host` produced by `parse_non_connect_target` / the CONNECT authority parse. Consider also
normalising in `net_filter::check_host` so the allowlist cannot be evaded the same way.

### WR-04: `CapturePhantomStore` never evicts — unbounded, agent-driven growth of real-token entries

**File:** `crates/nono-proxy/src/capture.rs:51-105`

**Issue:** `mint()` inserts into the `HashMap` and nothing ever removes an entry: no TTL, no
capacity bound, no removal on successful `resolve`. A sandboxed agent that loops token requests
through a capture route forces one entry (plus the full real token) per configured
`response_fields` entry per request, held for the entire proxy session. That is an
attacker-controlled memory-growth channel *and* it maximises the window during which real secrets
sit in the proxy's address space, which is precisely what D-08's "session-scoped" framing is meant
to bound.

**Fix:** Bound the store: a per-entry TTL (OAuth tokens have `expires_in`), a max entry count with
oldest-first eviction (`Zeroizing` wipes on removal), and — for the nonce-style use case —
single-use semantics (`resolve` removes the entry) which would also close the replay window. Any of
these is better than unbounded; the TTL + cap combination is the minimum.

### WR-05: Two routes sharing an upstream host where only one declares `capture` — no validation, no guard

**File:** `crates/nono-proxy/src/route.rs:180-263`; `crates/nono-cli/src/network_policy.rs:205-296`

**Issue:** Nothing at load time rejects (or warns about) a profile with e.g.
`oauth → https://provider.example` **with** capture and `providerapi → https://provider.example`
**without** capture. The sandboxed agent simply requests `/providerapi/oauth/token` instead of
`/oauth/token`: `handle_reverse_proxy` resolves the second route, `route.capture` is `None`, and the
response streams through unrewritten with the real token. The cross-path guard
(`capture_declared_for_upstream`) only covers CONNECT/forward-HTTP — the reverse-proxy path has no
equivalent host-level check.

This is the same "one path of several ignores a gate the others have" shape (WR-13 / the sites-2/3
near-miss) applied to route configuration instead of dispatch.

**Fix:** At `RouteStore::load()`, fail closed when two routes resolve to the same upstream host and
their `capture` declarations differ:

```rust
// after building `loaded`
for (prefix, route) in &loaded {
    if route.capture.is_none() {
        if let Some(hp) = &route.upstream_host_port {
            if loaded.values().any(|o| o.declares_capture
                && o.upstream_host_port.as_deref().map(host_only) == Some(host_only(hp))) {
                return Err(ProxyError::Config(format!(
                    "route '{prefix}' shares an upstream host with a capture-declared route but \
                     does not declare capture; this would relay real tokens unrewritten"
                )));
            }
        }
    }
}
```

### WR-06: `nono-py` denial-category decoder rejects the two strings its own encoder emits

**File:** `../nono-py/src/undo.rs:587-622`; `../nono-py/src/proxy.rs:98-103`

**Issue:** `proxy.rs` encodes `CaptureUnsupportedPath → "capture_unsupported_path"` and
`CaptureBufferOrRewriteFailed → "capture_buffer_or_rewrite_failed"` (correctly, since the encoder's
`match` is exhaustive and the compiler forced it). The dict → `NetworkAuditEvent` decoder in
`undo.rs` was **not** updated: its `match` has arms through `"spiffe_unsupported_path"` and then
falls to `other => Err(PyValueError::new_err(...))`. Any Python round-trip of an audit event
carrying either new category now raises `ValueError: invalid denial_category: ...` — a hard failure,
not a lossy one. The compiler could not catch it because the decoder matches on `&str`, not on the
enum. ADR-114 Consequence 3 records the `capture_context: None` residual but misses this.

**Fix:** Add the two arms next to `"spiffe_unsupported_path"`:

```rust
"capture_unsupported_path" =>
    Ok(nono::undo::NetworkAuditDenialCategory::CaptureUnsupportedPath),
"capture_buffer_or_rewrite_failed" =>
    Ok(nono::undo::NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed),
```

and add a round-trip test that iterates every encoder output string through the decoder, so the
next enum addition fails a test rather than shipping.

### WR-07: The capture path drops the per-request audit records every other relay path emits

**File:** `crates/nono-proxy/src/reverse.rs:504-514,791-802,1078-1089,1905-1920`

**Issue:** All three sites `return` the capture relay's result, so the audit call that would
otherwise follow never runs:

- site 1 skips `audit::log_reverse_proxy(service, method, upstream_path, status_code)` (`:520`);
- sites 2/3 skip `audit::log_l7_request(...)` including `auth_mechanism`,
  `auth_outcome: Succeeded`, `managed_credential_active`, `injection_mode` and — for site 2 —
  `spiffe_context` (`:833`, `:1123`).

What replaces them is `log_allowed(audit_log, Reverse, &ctx, route_id, 0, "")` (`:1905`), which
records `target = route_id`, `port = Some(0)`, `method = Some("")`, `status = None`. So exactly the
routes that handle OAuth tokens produce the *least* useful audit record in the system, and the
SPIFFE identity of a capture-declared SPIFFE route is no longer recorded at all.

**Fix:** Have `relay_response_with_capture` return the parsed `status_code` (it already computes it
at `:1796`) and let each call site emit its normal audit event with the capture context attached,
rather than substituting a degraded event inside the relay:

```rust
let status = relay_response_with_capture(...).await?;
audit::log_l7_request(ctx.audit_log, ..., &EventContext {
    spiffe_context: Some(material.spiffe_audit_context()),
    capture_context: Some(capture_ctx),
    ..
}, &L7RequestInfo { host: &upstream_host, port: upstream_port, method: &method,
                    path: upstream_path, status });
```

### WR-08: `request_nonce_fields` is accepted by profile validation but is inert on SPIFFE-dispatched routes and for header-presented phantoms

**File:** `crates/nono-proxy/src/reverse.rs:427-428` (only wiring), `:549-855`, `:872-1152`;
`crates/nono-cli/src/profile/credential_provider.rs:55-57`

**Issue:** `resolve_capture_request_body` is called only in `handle_reverse_proxy`. A route with
`spiffe` (or `oauth2.client_assertion`) plus `capture.request_nonce_fields` passes profile
validation and loads fine, but dispatch returns at `:222`/`:235` long before the resolution call —
the configured field silently does nothing. Likewise a phantom presented as
`Authorization: Bearer <phantom>` is never resolved. ADR-114 Consequence 2 names both as scope
limits, which is honest, but the operator-facing surface gives no signal: `validate_capture_config`
happily accepts nonce fields on a spiffe-declared credential. "Never silently degrade" (CLAUDE.md)
means an operator-configurable control that cannot fire should be rejected, not accepted.

**Fix:** In `validate_custom_credential`, reject `capture.request_nonce_fields` being non-empty when
`spiffe` is set or when `auth.client_assertion` is set, with an error naming the scope limit; or
wire `resolve_capture_request_body` into sites 2/3's request-building code (both already have the
`body` in hand at `:700-717` / `:983-1000`) so the config means what it says.

### WR-09: The D-08 "in-memory only" evidence is a source-text scan that cannot see the actual persistence surface

**File:** `crates/nono-proxy/src/capture.rs:409-421`

**Issue:** `capture_store_holds_only_in_memory` reads `capture.rs` as text and asserts the
production half contains none of `"std::fs::"`, `"File::create"`, `".write("`. It is presented (and
cited in ADR-114) as an "automated (not prose) structural assertion" that real tokens never touch
disk. It cannot establish that: the persistence surface is `reverse.rs` → `audit::log_denied` →
`NetworkAuditEvent.reason` → session metadata on disk, which is exactly the CR-01 leak. The scan
also misses `tokio::fs`, `OpenOptions`, `write_all`, `std::io::Write`, and anything reached via a
callee.

**Fix:** Replace (or supplement) it with a behavioural test that drives the enforcement point with a
known real token through each of the eight deny branches and asserts the token string appears in
neither the bytes written to the client **nor** the captured audit events' `reason` field (the
crate already has an in-memory `SharedAuditLog` fixture pattern for this — see
`forward_http_denies_capture_declared_route_upstream`). That test would have caught CR-01.

---

## Info

### IN-01: `phantom_ids` can silently desynchronise from `rewritten_fields`

**File:** `crates/nono-proxy/src/reverse.rs:1900-1904`

**Issue:** `CaptureAuditContext.phantom_ids` is documented (`undo/types.rs:333-335`) as "one per
rewritten field, same order/length as `rewritten_fields`", but is built with `filter_map`, so a path
that fails read-back yields a shorter vector and every subsequent index misaligns with no signal.

**Fix:** Use `map(|p| read_capture_body_path(&body, p).unwrap_or_default().to_string())` so the
lengths are structurally equal, or make the field `Vec<Option<String>>`.

### IN-02: Non-UTF-8 status line silently downgrades the status while still emitting the rewritten 200 body

**File:** `crates/nono-proxy/src/reverse.rs:1801-1802`

**Issue:** `std::str::from_utf8(...).unwrap_or("HTTP/1.1 502 Bad Gateway")` emits a 502 status line
followed by the successfully rewritten body and a 200-shaped `Content-Length`. On a path whose
contract is "deny or release, never something in between", falling back to a mismatched status is
inconsistent.

**Fix:** Treat a non-UTF-8 status line as a deny branch (it is already a malformed response).

### IN-03: Several internals are `pub` solely to dodge the `dead_code` lint, widening the crate's public API

**File:** `crates/nono-proxy/src/reverse.rs:1530-1541,1552,1615,1662,1741`;
`crates/nono-proxy/src/capture.rs:296-306`

**Issue:** `find_header_end`, `decode_chunked_body`, `parse_capture_response_headers`,
`read_capped_response`, `relay_response_with_capture`, `CAPTURE_READ_TIMEOUT` and
`resolve_request_nonce_fields` are `pub` with doc comments explicitly stating the reason is that
`pub(crate)` would trip `dead_code` under `-D warnings`. These are now permanent semver-visible API
on a library crate, and the transient justification no longer applies (they all have production
callers as of 114-06).

**Fix:** Now that the wiring has landed, demote them to `pub(crate)` (or private) — the lint no
longer fires — and keep `#[cfg(test)]`-only helpers behind the test module.

### IN-04: JSON schema has no `maximum` for `max_response_bytes`

**File:** `crates/nono-cli/data/nono-profile.schema.json` (`CaptureConfig.max_response_bytes`)

**Issue:** The ceiling (`CAPTURE_MAX_RESPONSE_BYTES_CEILING`, 1 MiB) is enforced only in Rust
(`credential_provider.rs:67`). The schema, which the same file uses `minItems: 1` on for exactly
this defense-in-depth reason, allows any positive integer.

**Fix:** `{ "type": "integer", "minimum": 1, "maximum": 1048576 }`.

### IN-05: Duplicated JSON-path traversal

**File:** `crates/nono-proxy/src/reverse.rs:1706-1715` vs `crates/nono-proxy/src/capture.rs:144-153`

**Issue:** `read_capture_body_path` re-implements `value_at_path_mut`'s traversal read-only, with a
comment justifying the duplication. Two copies of path-traversal semantics will drift; the audit
readback and the rewrite must agree on what a path means.

**Fix:** Expose a `pub(crate) fn value_at_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value>`
in `capture.rs` and have `value_at_path_mut` and the audit readback share it.

### IN-06: `CAPTURE_READ_TIMEOUT` is a hard-coded 30 s with no per-route override

**File:** `crates/nono-proxy/src/reverse.rs:1523`

**Issue:** `max_response_bytes` is per-route configurable but the time dimension of the same DoS
surface is not, and 30 s is long enough to matter once CR-02 is fixed (it becomes the tail-latency
bound for a hung upstream).

**Fix:** Add an optional `max_response_read_secs` to `CaptureConfig` with the same
validated-ceiling treatment `max_response_bytes` gets, or reduce the constant to something closer to
a token endpoint's realistic response time (5-10 s).

---

_Reviewed: 2026-08-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
