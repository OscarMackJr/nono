# ADR-114: OAuth Capture (SEC-02) — Absorb Disposition

**Status:** Accepted
**Phase:** 114 — OAuth Capture Absorb (SEC-02)
**Date:** 2026-08-07
**Authors:** Phase 114 execution

---

## Context

`112-OAUTH-CAPTURE-DISPOSITION.md` (Phase 112) captured reality-check evidence for upstream's
declarative sandboxed OAuth capture (`9b692e07` / SEC-02a, `3c59c62e` / SEC-02b, `d033c631` /
SEC-02c) and carved SEC-02 out to its own phase, deferring the adopt/adapt/decline call rather
than making it. Its §1c finding was specific and, at the time, correct as far as it went:
`forward.rs`'s `ResponseRewrite` hook — the mechanism that buffers a token-endpoint response and
rewrites real token fields to sandbox-visible phantoms before releasing it to the sandboxed
client — is absent from the fork, because `forward.rs` itself was never absorbed (its sole
creating commit is `149abde0`, "feat(proxy): add tls interception for l7-bearing connect
routes", outside every prior sync window). §1c went further and stated: "No equivalent
enforcement point exists elsewhere in the fork" — the fork's `reverse.rs`/`server.rs` response
paths "have no response-body rewrite hook of any kind." That framing reads, and was read at
Phase 114's planning start, as though `forward.rs` were the *only possible* place such a hook
could ever live.

**The retracted decline is part of this record, not erased from it.** The first decision locked
for this phase was a formal **DECLINE**: SEC-02 would not be resolved as a working capability,
on the reasoning that `112-OAUTH-CAPTURE-DISPOSITION.md` §1c had already established the only
enforcement point requires a subsystem (`forward.rs`, and transitively TLS interception) this
fork declines to build. The operator **retracted** that decline mid-discussion — "Phase 114
needs to resolve SEC-02, let's revisit" — and the retraction was correct, for a reason worth
recording precisely rather than glossing: the DECLINE's reasoning conflated response
**visibility** with response **buffering**. The fork's reverse proxy (`crates/nono-proxy/src/reverse.rs`)
already has plaintext visibility into upstream responses for every route reachable as a
configured reverse-proxy route — the sandboxed client speaks plain HTTP to the proxy, and the
proxy holds its own `tls_stream` to upstream. TLS interception (MITM) was never required to *see*
the response body for those routes. What the fork genuinely lacked was **buffering**: `reverse.rs`
relays responses in `[0u8; 8192]` chunks, by deliberate design, so that SSE / MCP Streamable HTTP
/ A2A JSON-RPC responses stream correctly. A rewrite hook needs the whole body in hand before it
can inspect and replace token fields; streaming forecloses that, independent of TLS interception
entirely. Once this distinction was surfaced, the real question stopped being "can we get MITM"
and became "can we build a fork-native buffer-and-rewrite point on the path that already sees
plaintext" — which is what this phase built.

**`112-OAUTH-CAPTURE-DISPOSITION.md`'s security conclusion is not overturned by this — it is
resolved differently.** Its conclusion that "a reduced-scope absorb without an equivalent
enforcement point would ship an unsafe half-feature" remains exactly correct; this phase does not
ship a reduced-scope absorb, it builds the equivalent enforcement point that document did not
know was buildable. A future reader who reads only `112-OAUTH-CAPTURE-DISPOSITION.md` and stops
there will re-derive the retracted DECLINE's wrong premise — that document's §1c framing is
corrected here explicitly so that does not happen again: **the real blocker to SEC-02 was
buffering, not visibility, and `forward.rs` was never the only possible enforcement point — it
was the only enforcement point upstream's own shape happened to use.**

---

## Decision

**D-01r: ADAPT — a fork-native response buffer-and-rewrite enforcement point on the reverse-proxy
path, superseding the retracted DECLINE.** `relay_response_with_capture()`
(`crates/nono-proxy/src/reverse.rs`) is that point: fail-closed at every non-success branch (read-cap
exceeded, incomplete headers, `Content-Encoding` present, chunked-decode failure, unparseable JSON,
rewrite error, unrewritten-token-field backstop trip, re-serialization failure — eight branches,
each `deny()` → `send_error(502)` → `return Ok(())`), with the single success path writing only the
rewritten body and a `Content-Length` recomputed from the rewritten bytes. TLS interception
(`149abde0`) is **not** absorbed; see the OD section below for that boundary recorded as permanent.

D-02r (SC2 satisfied by construction, not by importing upstream's `forward.rs` hook) and D-09 (the
ADR-86/ADR-111 policy-free-library boundary must be confirmed non-regressed, since this ADAPT ships
real code) are both confirmed below.

### D-06's positive proof — the four request-arrival paths, each with a confirmed capture disposition

This fork's proxy has exactly four ways a client request can arrive at an upstream: the
reverse-proxy dispatch path (three internal relay sites), a CONNECT tunnel, plain-HTTP
forward-proxy, and the external-proxy-chain arm nested inside CONNECT dispatch. Each is
enumerated below with its confirmed disposition, re-verified live against the current source
during this plan (not inherited from a prior plan's line numbers):

| Path | Dispatcher | Capture disposition |
|---|---|---|
| Reverse-proxy — site 1 (static-credential path) | `reverse.rs::handle_reverse_proxy` (`:486`) | **Enforced.** `if let Some(capture) = &route.capture { return relay_response_with_capture(...).await; }` — unconditional `return` before the unbuffered `[0u8; 8192]` loop. Proven by Plan 114-05's Finding 1 (control-flow audit) and its behavior tests (`capture_rewrites_configured_fields`, `capture_fails_closed_on_unrewritten_token_field`, `capture_denies_content_encoded_response`, `capture_buffer_cap_exceeded_denies_response`). |
| Reverse-proxy — site 2 (`handle_spiffe_route`, direct JWT-SVID bearer) | `reverse.rs::handle_spiffe_route` (`:791`) | **Enforced.** `if let Some(result) = relay_capture_if_declared(...).await { return result; }` immediately before the site's own `[0u8; 8192]` loop. Proven by Plan 114-06's `capture_rewrites_via_spiffe_route_site`. |
| Reverse-proxy — site 3 (`handle_spiffe_assertion_credential`, RFC 7523 jwt-bearer exchange) | `reverse.rs::handle_spiffe_assertion_credential` (`:1078`) | **Enforced.** Same shared-guard shape as site 2, immediately before its own `[0u8; 8192]` loop. Proven by Plan 114-06's `capture_rewrites_via_spiffe_assertion_site`. |
| CONNECT tunnel (bypass-route, non-bypass, and external-proxy-chain arms) | `server.rs::handle_connection`'s CONNECT dispatch (`:1343-1433`) | **Denied — fail-closed, no new deny surface.** The pre-existing, unconditional `is_route_upstream()` block (Phase 113's D-03 guard) already blocks CONNECT to every route upstream, and runs *before* `use_external` is even computed (`:1436`) — so the external-proxy-chain arm is unreachable for a capture-declared upstream by construction, not by a separate check. This plan's `114-07` only refines which `denial_category` the resulting 403 audit event carries (`CaptureUnsupportedPath` when `capture_declared_for_upstream()` matches, `:1396-1401`), mirroring the precedent `spiffe_declared_for_upstream` already established. Verified directly against the current `server.rs` this session — not merely cited from ADR-113 — because the plan's own interfaces block required confirming the external-proxy-chain claim live rather than inheriting it. |
| Plain-HTTP forward-proxy | `server.rs::handle_forward_http` (`:1096`) | **Denied — the phase's one genuinely new guard.** `if state.route_store.capture_declared_for_upstream(&host_port) { ... 403 ... }`, unconditional and explicitly **not** gated on `state.config.require_auth`, mirroring Phase 113's D-03 SPIFFE guard where that property was load-bearing (a guard that only fired when auth was enabled would fail open for `--no-auth` standalone proxies). Proven by Plan 114-07's `forward_http_denies_capture_declared_route_upstream` and, critically, by `forward_http_denies_capture_declared_route_upstream_on_different_port` — the **host-only** widening (deliberately broader than the SPIFFE sibling's exact `host:port` match, closing the same bypass class upstream's own `3c59c62e` hardening commit addressed) proven non-vacuous by an executed RED/GREEN: narrowing `capture_declared_for_upstream()` to `host_port_matches` broke only the different-port test, confirming the test discriminates the property it claims to. |

No request-arrival path is left silently unauthenticated for capture-declared routes.

### The sites-2/3 near-miss — recorded honestly, not smoothed over

The table above reads as though all three reverse-proxy relay sites were always covered together.
They were not, briefly, and that gap is part of this phase's record. Plan 114-05 wired only site
1 and its own SUMMARY named the gap explicitly as "Finding 3": dispatch at `reverse.rs:222` routes
on `has_spiffe_source()` and returns to `handle_spiffe_route`/`handle_spiffe_assertion_credential`
**before** the capture branch at `:486` is ever reached, so a route configured with both `spiffe`
and `capture` would have relayed the real OAuth token to the sandboxed client unrewritten — the
exact WR-13 defect shape (`112-REVIEW.md`: one request path of several ignoring a gate the others
have) applied to this phase's own new surface. Plan 114-05's executor Self-Check reported PASSED
for the plan's own (site-1-only) scope; the gap was caught by the orchestrator's hand control-flow
audit, not by that Self-Check, and closed one plan later by Plan 114-06 before any artifact — this
ADR included — was permitted to cite SC2 as satisfied across the full dispatch surface. This
mirrors Phase 112's own lesson (4 Critical fail-open defects that all 8 executor self-checks
reported PASSED on) at smaller scale, inside a single phase this time.

---

## D-09/SC3: Re-confirming the ADR-86 Boundary

`crates/nono/src/undo/types.rs` gained, for this phase: two `NetworkAuditDenialCategory` variants
(`CaptureUnsupportedPath`, `CaptureBufferOrRewriteFailed`), one new struct (`CaptureAuditContext`
— `route_id: String`, `rewritten_fields: Vec<String>`, `phantom_ids: Vec<String>`), and one new
field (`NetworkAuditEvent.capture_context: Option<CaptureAuditContext>`). Grep-backed, not
prose-backed:

```
$ grep -n "if \|match " crates/nono/src/undo/types.rs
65:        if s.len() != 64 {
90:        match self {
148:        match self {
287:    #[serde(default, skip_serializing_if = "Option::is_none")]
...
470:        match self {
```

Every `if`/`match` hit in the file is pre-existing (`ContentHash` length validation at `:65`,
`Display`/enum-mapping impls at `:90`/`:148`/`:470`) or a `#[serde(...)]` attribute — none falls
inside or near `CaptureAuditContext` (`:320-336`) or the two new enum variants (`:263`, `:274`).
`CaptureAuditContext`'s own doc comment states the invariant directly: "Pure data — records what
happened ..., applies no enforcement or policy evaluation. Structurally cannot carry a raw token:
every field is an identifier ..., never secret bytes." Confirmed by reading the struct: three
fields, all `String`/`Vec<String>`, no methods beyond `#[derive]`.

A workspace-wide check confirms no capture-related policy logic landed anywhere else in the core
library:

```
$ grep -rln "capture" crates/nono/src/
crates/nono/src/audit.rs        (two `capture_context: None,` test-fixture initializers only)
crates/nono/src/keystore.rs     (pre-existing, unrelated: "credential capture path" prose, `op read` stdout capture)
crates/nono/src/supervisor/aipc_sdk.rs  (pre-existing, unrelated: "capture inference" doc comment on a raw-pointer note)
crates/nono/src/undo/{merkle,mod,snapshot}.rs  (pre-existing, unrelated: "filesystem state capture[d]" prose)
crates/nono/src/undo/types.rs   (the additions enumerated above)
```

No enforcement, buffering, rewrite, or policy logic of any kind landed in `crates/nono/src/` — all
of it (`relay_response_with_capture`, `relay_capture_if_declared`, `resolve_capture_request_body`,
`CapturePhantomStore`, the response-framing helpers) lives in `crates/nono-proxy/src/`. **D-09/SC3
confirmed: the ADR-86 boundary holds, with the same class of small, honestly-recorded
vocabulary-only widening ADR-113 recorded for SPIFFE** (the core library's audit vocabulary now
names capture-shaped identifiers even though it enforces nothing).

---

## OD: The TLS-Interception Boundary Against `149abde0` — A Named, Permanent Decision

Mirroring ADR-113's OD-1 shape: `149abde0` ("feat(proxy): add tls interception for l7-bearing
connect routes") — the commit that creates `forward.rs` and brings TLS interception into
upstream's proxy — is **not** absorbed by this phase, and was never on this phase's table to
absorb. The fork's no-MITM stance (`ProxyHandle::intercept_ca_path()` returns `None`
unconditionally; `112-07` dropped `apply_tls_intercept_config`; ADR-113's own D-01 re-affirmed it
for SPIFFE) is a **standing decision, reaffirmed by this phase, not reopened by it.**

This creates SEC-02's scope limit, stated precisely: **capture works only for OAuth token
endpoints reachable as configured reverse-proxy routes.** Agent-initiated flows to arbitrary hosts
via CONNECT remain out of reach of capture — those requests never enter `reverse.rs` at all, and
the CONNECT/forward-HTTP guards above deny them outright rather than rewriting their responses.
TLS interception is the **named condition** under which this scope limit could be lifted: only
with a decrypting man-in-the-middle on the CONNECT path would the proxy gain the plaintext
visibility into an arbitrary agent-initiated response that `reverse.rs` already has for configured
routes. This fork declines that condition by standing decision, so the scope limit stands as a
permanent characteristic of this disposition, not a TODO awaiting a later plan.

A future planner who eventually absorbs `149abde0` (bringing TLS interception into this fork for
some other reason) must find this ADR and understand that lifting SEC-02's scope limit would then
become possible, but is not automatic — it would require deliberately wiring
`relay_response_with_capture()` (or an equivalent) into whatever new intercepted-TLS response path
that absorb creates, mirroring how this phase wired it into all three `reverse.rs` sites rather
than assuming coverage. The corresponding ledger carry-forward note is filed in
`108-DIVERGENCE-LEDGER.md` (see Consequences).

---

## Consequences

1. **SHIPPED — the mint-to-resolve loop is closed end-to-end, with a real production call chain,
   not merely a minting-side stub.** `CapturePhantomStore::resolve()` (Plan 114-04) has a real
   production call site: `resolve_capture_request_body()` (`reverse.rs:2007`) — wired into
   `handle_reverse_proxy`'s outbound request-body path (Plan 114-06 Task 3) — calls
   `capture::resolve_request_nonce_fields()`, which calls `CapturePhantomStore::resolve()`, to
   resolve a previously-minted, admitted phantom presented in a `request_nonce_fields`-configured
   JSON request-body field back to the real captured token before the request is forwarded
   upstream. Fail-closed on substitution, not on the request: an unknown or non-admitted phantom is
   left unchanged and simply rejected by the real upstream as an invalid credential — the resolver
   never denies the request itself. Proven by two independent unit tests
   (`capture_egress_resolves_admitted_phantom_in_request_body`,
   `capture_egress_never_substitutes_real_token_for_unadmitted_consumer`) and one live end-to-end
   test, `capture_egress_resolution_reaches_upstream_on_live_dispatch_path`, which drives a real
   request through `handle_reverse_proxy` against a hermetic in-process self-signed TLS upstream
   and asserts the REAL token — not the phantom — is what upstream actually receives.

2. **NOT SHIPPED, named explicitly, not silently dropped:**
   - **Header-based egress resolution.** An `Authorization: Bearer <phantom>` presented directly
     (rather than embedded in a JSON request-body field named in `request_nonce_fields`) is not
     resolved by this phase — only the JSON-body path is wired.
   - **Egress resolution on SPIFFE-dispatched routes.** `resolve_capture_request_body()` is wired
     only into `handle_reverse_proxy`'s own dispatch path (site 1, the static-credential /
     no-credential route flow) — it is not threaded into `handle_spiffe_route`'s or
     `handle_spiffe_assertion_credential`'s own outbound request-building code. A route reached
     exclusively via SPIFFE-based request auth therefore has response-side capture (Task 1/2 above,
     fully wired at all three sites) but no egress-resolution path yet. This is an explicit scope
     limit recorded in Plan 114-06's own SUMMARY, not an oversight discovered here.
   - **Disk persistence of real tokens** (D-08; upstream's `oauth_capture/persist.rs`). Real tokens
     are held in-memory, session-scoped only, zeroized on drop — no secret at rest, at the accepted
     cost of re-login after every proxy restart.
   - **Login-flow helper commands and upstream's `credential_store`/`helpers`/provider-registry
     indirection** (D-12 discretion) — the declarative provider surface was ported adapted; the
     surrounding CLI helper-command machinery was not.

3. **One known residual in the `../nono-py` binding, recorded here rather than silently patched.**
   `../nono-py/src/undo.rs`'s dict → `NetworkAuditEvent` struct literal sets
   `capture_context: None,` — there is no dict encoder for the rich `CaptureAuditContext` payload,
   so it is lost on a dict → `NetworkAuditEvent` round-trip through the Python binding. This exactly
   mirrors the pre-existing `spiffe_context: None,` residual ADR-113 recorded for the identical
   reason (no encoder exists for that payload either). This is a binding-layer gap, not a
   confinement gap: no Rust-side enforcement or audit-writing behavior is affected, only Python
   callers reconstructing a `NetworkAuditEvent` from a dict lose the capture detail. See Plan
   114-10's SUMMARY for the full break-surface record (4 compiler errors across 3 files — one more
   file and one more error class than the plan predicted, consistent with every prior occurrence of
   this pattern in Phases 113-114).

4. **`108-DIVERGENCE-LEDGER.md` receives a "SEC-02 Carry-Forward Note (Phase 114, D-10)"** naming
   `149abde0` as the tracked divergence behind SEC-02's scope limit — see that document for the full
   note; cross-referenced here, not duplicated.

5. **`REQUIREMENTS.md`'s SEC-02 checkbox resolves as satisfied-with-scope-limit**, citing this ADR
   by path, per D-11 — not left open.

6. **`112-OAUTH-CAPTURE-DISPOSITION.md`'s §1c framing is corrected by this ADR's Context section**,
   not superseded silently: its file-presence evidence and its "a reduced-scope absorb without an
   equivalent enforcement point would ship an unsafe half-feature" security conclusion both remain
   correct and are the reason this phase built a real enforcement point rather than shipping a
   half-feature. What was wrong was the implicit claim that no equivalent point could exist — this
   ADR is the record a future reader should consult instead of re-deriving that wrong premise from
   `112-OAUTH-CAPTURE-DISPOSITION.md` alone.

---

## References

- `.planning/phases/112-security-residual-sync/112-OAUTH-CAPTURE-DISPOSITION.md` — the reality-check
  evidence this ADR builds on and whose §1c framing it corrects (not overturns).
- `.planning/phases/112-security-residual-sync/112-REVIEW.md` — WR-13, the defect shape D-06's
  cross-path guard (and the sites-2/3 near-miss) both instantiate.
- `proj/ADR-113-spiffe-disposition.md` — the closest structural precedent: its OD-1 section is the
  shape for naming a declined prerequisite as a permanent boundary; its D-01 positive-proof section
  is the shape for a grep/test-backed enumeration of every request path; its D-08/SC3 section is the
  shape for the ADR-86 boundary re-confirmation above.
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary D-09/SC3 tests
  against.
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-RESEARCH.md`,
  `114-CONTEXT.md`, `114-PATTERNS.md` — the phase's governing research and locked decisions
  (D-01r through D-14).
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-05-SUMMARY.md` — the enforcement point and
  site-1 wiring, including Finding 3 (the sites-2/3 near-miss).
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-06-SUMMARY.md` — sites 2/3 wired, the
  mint-to-resolve loop closed with a live end-to-end test.
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-07-SUMMARY.md` — the D-06 cross-path
  fail-closed guard, including the host-only-widening RED/GREEN proof.
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-10-SUMMARY.md` — the real D-14 sibling-
  binding break surface, including the `capture_context` residual.
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — the SEC-02 Carry-Forward
  Note (Phase 114, D-10) this ADR's Consequences section cross-references.
