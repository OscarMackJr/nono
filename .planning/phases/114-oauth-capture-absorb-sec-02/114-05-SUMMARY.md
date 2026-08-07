---
phase: 114-oauth-capture-absorb-sec-02
plan: 05
subsystem: network-proxy
tags: [oauth-capture, sec-02, enforcement-point, buffer-and-rewrite, fail-closed, d-01r, d-02r, d-05]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-01's CaptureAuditContext + NetworkAuditDenialCategory, EventContext.capture_context"
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-02's CaptureConfig / CaptureResponseField declarative types and RouteConfig.capture"
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-04's CapturePhantomStore, rewrite_response_fields(), reject_unrewritten_token_fields()"
provides:
  - "relay_response_with_capture() — the fork-native OAuth-capture response buffer-and-rewrite enforcement point (D-01r/D-02r), crates/nono-proxy/src/reverse.rs"
  - "Relay site 1 (handle_reverse_proxy, static-credential path) wired to the enforcement point"
  - "CapturePhantomStore threaded through ProxyState + ReverseProxyCtx as ONE shared session-scoped instance"
  - "NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed — right-path fail-closed denial, distinct from CaptureUnsupportedPath's wrong-path denial"
  - "read_capped_response(), find_header_end(), decode_chunked_body(), parse_capture_response_headers(), relay_response_streaming() — extracted HTTP response framing helpers"
affects: [114-06, 114-07, 114-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fail-closed enforcement helper: every non-success branch calls deny() + send_error(502) + `return Ok(())`, so no branch can fall through to a body write. The single success path writes only the rewritten body with a recomputed Content-Length."
    - "Helper is generic over `AsyncRead + Unpin` rather than taking the concrete tokio_rustls::client::TlsStream, purely so the fail-closed branches are drivable from unit tests without a live TLS upstream — the real call site satisfies the bound unchanged."
    - "Capture branch placed as an unconditional early `return` immediately before the unbuffered streaming call, so the two paths are mutually exclusive by control-flow construction rather than by a flag checked in two places."

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono/src/undo/types.rs
---

# Plan 114-05 Summary — the OAuth-capture enforcement point

## What was built

`relay_response_with_capture()` (`crates/nono-proxy/src/reverse.rs:1671`) — the fork-native
response buffer-and-rewrite enforcement point that operator decision **D-01r** substituted for
upstream's `forward.rs` `ResponseRewrite` hook. It is wired into **relay site 1 only**
(`handle_reverse_proxy`'s static-credential path, `reverse.rs:486-496`). Sites 2 and 3 are
Plan 114-06's scope and are deliberately untouched here — see the open-gap finding below.

`CapturePhantomStore` is threaded through `ProxyState` and `ReverseProxyCtx` as **one** shared
instance, so all three relay sites will share a single session-scoped store once 114-06 lands.

A new denial category, `NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed`, was added to
`crates/nono/src/undo/types.rs` alongside 114-01's `CaptureUnsupportedPath`. The two are
deliberately distinct: `CaptureUnsupportedPath` denies a capture-declared route for arriving on
the **wrong path** (no rewrite implementation there at all — 114-07's guard);
`CaptureBufferOrRewriteFailed` denies on the **right path** because the buffer/parse/rewrite step
itself failed closed.

## Commits

| Commit | Content |
|---|---|
| `026f770b` | Thread `CapturePhantomStore` through `ProxyState` and `ReverseProxyCtx` |
| `fd1ff92d` | Carry the real `CaptureConfig` through `LoadedRoute`, not just a bool |
| `7882ab09` | HTTP response framing helpers for buffer-and-rewrite |
| `2f11440f` | Helper + site-1 wiring + behavior tests (committed as WIP by the orchestrator when the executor was interrupted; see Provenance) |

## THE CONTROL-FLOW AUDIT (this plan's central obligation)

The plan required an explicit re-read confirming no path from a capture-declared route reaches the
unbuffered 8 KiB streaming loop. **This audit was performed by the orchestrator, by hand, reading
the source — not by an executor self-check.** That distinction is deliberate: Phase 112 shipped 4
Critical fail-open defects that all 8 executor self-checks reported PASSED on, and the code-review
gate, not the executors, caught them.

### Finding 1 — Site 1: NO fall-through. Verified.

`reverse.rs:486-496` is the only fork in control flow between the buffered and unbuffered paths:

```rust
if let Some(capture) = &route.capture {
    return relay_response_with_capture(...).await;   // unconditional return
}
// Stream the response back to the client without buffering.
let status_code = relay_response_streaming(&mut tls_stream, stream).await?;
```

The `return` is unconditional and precedes the streaming call. There is no `break`, no fallthrough
arm, and no error path in the `if let` body that resumes below it. A capture-declared route
reaching site 1 cannot enter the unbuffered loop.

### Finding 2 — The helper is fail-closed at every branch. Verified line by line.

All eight non-success branches call `deny(...)` → `send_error(stream, 502, "Bad Gateway")` →
`return Ok(())`. None writes a body:

| # | Branch | Line |
|---|---|---|
| 1 | `read_capped_response` error — cap exceeded (D-05), read timeout, or upstream read failure | 1707-1714 |
| 2 | No complete header block in the buffered response | 1717-1721 |
| 3 | `Content-Encoding` present — **unconditional of status code** (Pitfall 2 / `3c59c62e`) | 1739-1747 |
| 4 | Chunked-decode failure | 1750-1758 |
| 5 | Body is not parseable JSON | 1765-1772 |
| 6 | `rewrite_response_fields()` error | 1779-1791 |
| 7 | `reject_unrewritten_token_fields()` backstop trips (D-07) | 1796-1800 |
| 8 | Re-serialization of the rewritten body fails | 1804-1811 |

The single success path (1813-1825) writes `rewritten_body` — never `raw_body` — with
`Content-Length` recomputed from the rewritten bytes and `Transfer-Encoding` stripped. Step 9's
audit reads back only already-rewritten (phantom) values via `read_capture_body_path`, so the
audit log cannot surface a real token.

One cosmetic observation, not a leak: if the upstream status line is invalid UTF-8, line 1732
falls back to the literal `"HTTP/1.1 502 Bad Gateway"` status line but still forwards the
rewritten body. The body is safe by that point (rewrite and backstop have both passed), so this is
a status-line cosmetic issue rather than a confinement issue. Noted for 114-11's review rather
than fixed here, since changing it is out of this plan's scope.

### Finding 3 — Sites 2 and 3 DO currently fall through. OPEN, in-flight, closed by 114-06.

**This is the honest counter-finding and it must not be read as "the phase is safe yet."**

Dispatch at `reverse.rs:222` routes on `has_spiffe_source()` and returns **before** the capture
branch at `:486` is ever reached:

```rust
if route.has_spiffe_source() {
    return handle_spiffe_route(...);          // site 2 — no capture branch
}
    ... return handle_spiffe_assertion_credential(...);   // site 3 — no capture branch
```

Both `handle_spiffe_route` (`:531`, streaming buf at `:766`) and
`handle_spiffe_assertion_credential` (`:833`, streaming buf at `:1021`) were confirmed by grep to
contain **zero** `capture` references, so each streams unbuffered.

**Consequence as of this commit:** a route configured with BOTH `spiffe` and `capture` reaches
site 2 or 3 and the real OAuth token is relayed to the sandboxed client unrewritten.

This is exactly the gap Plan 114-06 exists to close — its own objective names this scenario
verbatim — and 114-05's scope was explicitly site 1 only. But until 114-06 lands, **SC2 does not
hold for spiffe+capture routes**, and no artifact should claim otherwise. ADR-114 (114-11) must
only cite SC2 as satisfied by construction once all three sites are wired.

## Verification

All run by the orchestrator against the committed tree, not reported from an executor:

| Gate | Result |
|---|---|
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | exit 0 |
| `cargo test -p nono-sandbox-proxy --lib` | 285 passed, 0 failed (268 baseline + 17 new) |

Required behavior tests, all present and passing:

- `capture_rewrites_configured_fields` — the success path actually rewrites
- `capture_fails_closed_on_unrewritten_token_field` — backstop denies (branch 7)
- `capture_denies_content_encoded_response` — branch 3
- `capture_buffer_cap_exceeded_denies_response` — branch 1 / D-05
- `non_capture_route_still_streams_unbuffered` — **D-06 regression guard**: streaming remains the
  default for non-capture routes, so SSE / MCP Streamable HTTP / A2A JSON-RPC are unaffected

## Deviations from plan

1. **`LoadedRoute` carries the real `CaptureConfig`, not just the `declares_capture` bool**
   (`fd1ff92d`). Plan 114-03 added the bool as a cheap predicate; the enforcement point needs the
   actual config (field list, per-route `max_response_bytes`) at relay time. The bool is retained
   because 114-07's cross-path guard only needs the predicate.
2. **New denial category `CaptureBufferOrRewriteFailed`** added beyond 114-01's vocabulary. 114-01
   added `CaptureUnsupportedPath` ahead of need for the wrong-path case; the right-path
   fail-closed case had no distinct category and would otherwise have been indistinguishable in
   the audit trail.
3. **HTTP response framing was extracted into named helpers** (`read_capped_response`,
   `find_header_end`, `decode_chunked_body`, `parse_capture_response_headers`,
   `relay_response_streaming`) rather than inlined, so the fail-closed branches are unit-testable
   without a live TLS upstream.

## Provenance note

This plan's executor was interrupted twice by operator stops. Three commits (`026f770b`,
`fd1ff92d`, `7882ab09`) landed normally; the final chunk — the helper body, site-1 wiring, and the
behavior tests — was left uncommitted and was preserved by the orchestrator as WIP commit
`2f11440f` after confirming all gates were green. The control-flow audit above and this SUMMARY
were then completed by the orchestrator. No executor self-check is being relied on for this
plan's security claims.

## Self-Check: PASSED (with one open gap explicitly recorded)

Site 1 and the helper meet the plan's fail-closed contract in full. Sites 2 and 3 remain open by
design and are Plan 114-06's scope — recorded above as Finding 3 rather than deferred silently.
