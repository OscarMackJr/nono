# Phase 114: OAuth Capture Absorb (SEC-02) - Research

**Researched:** 2026-08-06
**Domain:** Reverse-proxy response buffering + rewrite (fork-native), declarative OAuth provider
config, profile schema, dual-binding rebuild
**Confidence:** HIGH (all claims below are grep/`git show`-verified against the live tree at
research time; a small number of design-reference claims from upstream source are marked CITED)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01r: ADAPT — build a fork-native response buffer+rewrite on the reverse-proxy path.**
  **This SUPERSEDES a retracted D-01 (formally DECLINE).** The decline was chosen, then withdrawn
  by the operator mid-discussion ("Phase 114 needs to resolve SEC-02, let's revisit"). The
  retraction is recorded deliberately, not erased — the evidence that triggered it matters:
  *Discovered during discussion:* the fork's reverse proxy **already has plaintext visibility into
  upstream responses**. The sandboxed client speaks plain HTTP to the proxy; the proxy holds its
  own `tls_stream` to upstream (`reverse.rs`, verified live). So MITM was never required for token
  endpoints reachable as configured routes — the premise that `forward.rs`'s `ResponseRewrite` was
  the *only possible* enforcement point conflated **visibility** (which the fork has) with
  **buffering** (which it lacks, by deliberate design: responses stream in 8 KiB chunks across
  **three** relay sites, for SSE / MCP Streamable HTTP / A2A JSON-RPC).
  TLS interception (`149abde0`) is **NOT** absorbed; the fork's no-MITM decision stands.
- **D-02r: SC2 is satisfied BY CONSTRUCTION**, via a fork-native enforcement point, rather than by
  importing upstream's `forward.rs` hook. SC2's forbidden outcome — "a reduced-scope absorb that
  drops the rewrite hook without an equivalent enforcement point" — is avoided because an
  equivalent point is *built*, not omitted.
- **D-09: SC3 is LIVE, not vacuous.** The ADAPT ships real code, so the ADR-86 / ADR-111 boundary
  must be confirmed non-regressed — no policy or enforcement logic may land in the core `nono`
  crate. SC4's two cross-target clippy gates are mandatory (no PARTIAL→CI).
- **D-05: Tight response-buffer cap, FAIL CLOSED on exceed.** Bound sized for token responses
  (~256 KiB–1 MiB, planner may refine). If an OAuth-capture-declared route returns more than the
  cap, **DENY the response — never release it unrewritten.** Exceeding the cap means the proxy
  cannot guarantee the token was rewritten, and releasing it anyway is exactly the fail-open shape
  SC2 forbids. This also bounds the DoS surface that buffering introduces. Precedent: the fork's
  existing 16 MiB **request**-body cap (`reverse.rs`) — deliberately NOT reused here, being far
  larger than any token response needs.
- **D-06: Buffering is OPT-IN PER ROUTE, plus a D-03-style cross-path fail-closed guard.**
  Only routes explicitly declared as OAuth token endpoints are buffered; **streaming remains the
  default for everything else** — the fork streams deliberately (module doc, `reverse.rs`) and
  buffering SSE / MCP Streamable HTTP / A2A JSON-RPC would break them.
  A capture-declared route arriving via **CONNECT, forward-HTTP, or the external-proxy chain**
  must **fail closed at request time**, since those paths have no rewrite. This mirrors Phase 113's
  D-03 exactly and is a **fork-original test obligation** — upstream ships no equivalent test
  because upstream *has* interception on those paths. See also WR-13 in `112-REVIEW.md`: one
  request path of six ignoring an auth gate is the precise defect shape this guard prevents.
- **D-08: Real OAuth tokens are held IN-MEMORY, SESSION-SCOPED ONLY**, zeroized on drop
  (CLAUDE.md mandates `zeroize` for secrets). Upstream's `oauth_capture/persist.rs` disk
  persistence is **not** adopted: no secret at rest, no file-permission surface, no cross-session
  replay if the host is later compromised. **Accepted cost:** re-login after every proxy restart.
- **D-07: Port upstream's config/rewrite LOGIC as a design reference; REBUILD the plumbing.**
  Take the declarative provider config, token-field/phantom-mapping logic, and JWT handling — they
  encode real problem knowledge (which fields OAuth providers actually return) that would be
  wasteful to re-derive. But rebuild the enforcement plumbing against the fork's `reverse.rs` relay
  sites rather than porting `forward.rs`-shaped code. This is Phase 113's proven pattern for
  `reverse.rs` / `route.rs` / `credential.rs`; `git apply` will not work here either.
- **D-12: `profile/credential_provider.rs` is ported ADAPTED, minus absent-subsystem coupling.**
  Bring the declarative provider surface (token endpoint, response/request token fields, API hosts,
  credential-store detection) adapted to the fork's profile shape; drop anything reaching into
  `oauth_capture/` or `forward.rs`. SEC-02 is literally titled *"declarative* sandboxed OAuth
  capture" — dropping the declarative half would under-deliver the requirement.
- **D-13: The profile schema is a FIRST-CLASS TASK with a round-trip test.** Hand-port the needed
  `$defs`/properties into `crates/nono-cli/data/nono-profile.schema.json` (fork-only companion,
  `additionalProperties: false`, upstream never touches it) **and** add a test asserting a realistic
  capture-provider profile both deserializes AND passes `validate_against_schema()`.
  Do **not** port upstream's 265 schema lines wholesale — they describe the persistence and
  interception surfaces this fork is not building, and a schema advertising configuration nothing
  implements is itself a fail-open shape. Phase 110 hit the serde/schema mismatch; Phase 113 needed
  four hand edits.
- **D-14: `nono-py` binding fix is its OWN NAMED TASK, with both bindings rebuilt.**
  `RouteConfig` gains capture config; `../nono-py` constructs it with exhaustive struct literals, so
  it will break — the **fifth** consecutive occurrence (after `endpoint_policy`/`enable_h2`,
  `denied_hosts`/`no_proxy`, `require_auth`/`strict_connect_auth`, and Phase 113's `spiffe`, which
  broke **three** files where static inspection predicted one). Run `maturin build` in `../nono-py`
  and `napi build --platform --release` in `../nono-ts`, reporting real results — only building
  catches struct drift. **This forces `workflow.use_worktrees=false`** (the plan reaches a sibling
  repo via `..`).
- **D-10: File the scope limit as a carry-forward note in
  `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`**, updating the deferred
  SEC-02 row to **ADAPTED-with-scope-limit**. Name `149abde0` as a tracked divergence, mirroring
  how Phase 113 filed `b1ecbc02`. The ledger is the canonical work-list a future UPST planner
  actually reads. Rationale for not relying on a code comment alone: Phase 113 proved a fork's own
  "why is this missing" comment can be wrong and mislead for months.
- **D-11: SEC-02's `REQUIREMENTS.md` checkbox resolves as satisfied-with-scope-limit citing
  ADR-114** — not left open. Phase 112 deliberately left it unchecked pending this verdict; leaving
  it open again would misrepresent the milestone.

### Claude's Discretion

- Exact buffer cap value within D-05's stated range, and whether it is configurable.
- The concrete phantom-token minting/resolution mechanism, and how it composes with the fork's
  existing phantom machinery (which runs the *outbound* direction — proxy injects real credentials
  into outbound requests; capture is the *inbound* direction).
- What the audit record carries — subject to the hard constraint that it must **never** contain a
  raw token (cf. Phase 113's `SpiffeAuditContext`, which carries only IDs).
- Whether `nono-cli` needs a helper command to drive a login flow.
- Plan/wave decomposition, and whether ADR-114 lands as its own plan or a wave of a larger one.

### Deferred Ideas (OUT OF SCOPE)

- **Disk persistence of real OAuth tokens** (upstream `oauth_capture/persist.rs`, bounded phantom
  retention). Deliberately not taken — secret-at-rest is a separate decision from the
  enforcement-point decision and deserves its own security review (file permissions, path
  validation, multi-user hosts). Revisit only with that review.
- **TLS interception / `149abde0`.** Would extend capture to agent-initiated flows to arbitrary
  hosts via CONNECT. The fork's no-MITM stance is a standing decision and was not reopened here.
  This is the named condition under which SEC-02's scope limit could be lifted.
- **Switching `nono-py` to `..Default::default()`** to end the recurring build break. Raised in
  Phase 113 and rejected there: it trades a loud compile error for silent field adoption on a
  security-relevant struct — arguably the worse failure mode. Still its own scoped change.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-------------------|
| SEC-02 | Declarative sandboxed OAuth capture (`9b692e07`), capture-boundary hardening (`3c59c62e`), and the stdin fixture test (`d033c631`) are absorbed or formally declined, with the guarantee that real OAuth tokens never reach the sandboxed client preserved as a hard precondition. | Priority 1 (exact 3 relay-site locations + shared-helper feasibility), Priority 2/D-07 (portable rewrite logic: JSON dot-path field rewrite, JWT-shaped phantom, fail-closed unrewritten-token rejection, Content-Encoding hardening), Priority 3 (`RouteConfig`/`CustomCredentialDef`/`OAuth2Config` declarative surface to extend), Priority 4/D-06 (exact guard precedent — `spiffe_declared_for_upstream`/`has_spiffe_source`, and which arrival paths already covered vs. genuinely new), Priority 5 (outbound-vs-inbound phantom direction mismatch, confirmed structurally), Priority 6/D-13/D-14 (schema `$defs` pattern + round-trip test template; both binding break sites + exact build commands), Priority 7 (Validation Architecture section below), Priority 8 (Pitfalls 1-5 + Security Domain threat table) |
</phase_requirements>

## Summary

Phase 114 builds a **fork-native** response buffer-and-rewrite enforcement point on the
reverse-proxy path (`crates/nono-proxy/src/reverse.rs`) so real OAuth tokens are replaced with
sandbox-visible phantoms before reaching the sandboxed client — this is D-01r/D-02r, already
locked. This research answers the *how*: it locates the exact three response-relay loops the
rewrite must cover, the exact shape of the request-arrival paths D-06's fail-closed guard must
gate, the exact declarative-config precedent to extend (`RouteConfig`/`CustomCredentialDef`), and
the exact portable logic in upstream's `oauth_capture/{jwt,rewrite,endpoint}.rs` (D-07/D-12) worth
lifting as design reference.

The three `[u8; 8192]` response-relay loops in `reverse.rs` are **structurally identical** —
same buffer size, same `first_chunk`/`parse_response_status` idiom, same unbuffered
`stream.write_all` — which makes a single shared buffer-and-rewrite helper genuinely feasible
(Priority 1, confirmed by direct read, not assumed). A **fourth**, structurally identical loop
exists in `server.rs::handle_forward_http` (`server.rs:1209-1230`) but is NOT a rewrite target —
that path is transparent forward-proxying to arbitrary hosts, exactly the traffic class D-06's
scope limit (CONNECT/forward-HTTP/external-proxy-chain) excludes from capture.

D-06's fail-closed guard has a **direct, already-live precedent** to extend, not invent: Phase
113's `spiffe_declared_for_upstream()` / `RouteStore::has_spiffe_source()` pair, and the two call
sites that use it (`server.rs:1341-1352` inside the CONNECT dispatch's pre-existing
`is_route_upstream` block, and `server.rs:1055-1077` as `handle_forward_http`'s own dedicated
guard). A parallel `capture_declared_for_upstream()` following the exact same shape closes D-06
with minimal new surface, and — critically — the CONNECT path does **not** need a new guard at
all: `is_route_upstream()` already blocks CONNECT to **every** route upstream unconditionally
(`server.rs:1320-1373`), before any SPIFFE- or capture-specific refinement. Only
`handle_forward_http` needs a genuinely new check, mirroring the SPIFFE precedent exactly.

Upstream's `oauth_capture/{jwt,rewrite,endpoint}.rs` contain real, portable problem-knowledge
(D-07): JSON dot-path field extraction and rewrite (`value_at_path_mut`), a fail-closed
"reject-unrewritten-token-shaped-field" heuristic (`reject_unrewritten_token_fields`, keyed on
`access_token`/`refresh_token`/`id_token`), JWT-shaped phantom construction
(`jwt_shaped_phantom`), and — critically — a **hardening lesson from `3c59c62e`**: an unconditional
(not 2xx-only) rejection of `Content-Encoding`-compressed responses, because the simple
header/body split this fork would build cannot safely decompress-then-recompress. This is a
concrete pitfall the fork-native helper must replicate from day one, not discover the hard way.

**Primary recommendation:** build one shared `rewrite_capture_response()`-shaped helper in
`reverse.rs` (buffer to a `Zeroizing<Vec<u8>>` capped per D-05, fail closed on cap exceed or
`Content-Encoding` presence, split header/body on `\r\n\r\n`, apply upstream's ported
dot-path-rewrite logic, recompute `Content-Length`, write once), call it from all three relay
sites when `route`/`cred` declares a capture config, and add one new `has_capture_source()`-style
guard analogous to `spiffe_declared_for_upstream()` for `handle_forward_http`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Response buffering + phantom rewrite | API / Backend (`nono-proxy` reverse-proxy handler) | — | This IS the enforcement point; must live where the proxy already holds plaintext response visibility (`reverse.rs`'s `tls_stream` to upstream) — confirmed live, D-01r |
| Declarative provider/route config (schema + parse) | API / Backend (`nono-cli` profile layer) + `nono-proxy` config types | — | Mirrors `RouteConfig`/`CustomCredentialDef`/`OAuth2Config` — config always lives in `nono-cli::profile` + `nono-proxy::config`, never the core `nono` library (ADR-86) |
| Phantom-token storage + resolution | API / Backend (`nono-proxy`, in-memory, session-scoped) | — | D-08: in-memory only, zeroized on drop; no Database/Storage tier — disk persistence explicitly declined |
| D-06 fail-closed cross-path guard | API / Backend (`nono-proxy::server`/`route`) | — | Request-time gate before any handler dispatch; same tier as the existing D-03 SPIFFE guard it extends |
| Audit record (capture context) | API / Backend (`nono-proxy::audit`) + core `nono` (data type only) | — | Mirrors `SpiffeAuditContext`: the *type* is pure data in the core `nono` crate (`crates/nono/src/undo/types.rs`), the *emission logic* stays in `nono-proxy` — ADR-86 boundary |
| Profile schema (`nono-profile.schema.json`) | API / Backend (`nono-cli` data, fork-only) | — | D-13: hand-ported `$defs`, never touched by upstream sync |
| Python/Node bindings (`RouteConfig` struct literal) | API / Backend (sibling repos `nono-py`/`nono-ts`) | — | D-14: mechanical struct-literal fixup, not new architecture |

## Standard Stack

No new external crates are required. All portable logic (Priority 2 below) depends only on crates
`nono-proxy` **already has**:

| Crate | Version (verified) | Purpose | Already used for |
|-------|---------------------|---------|-------------------|
| `base64` | `0.22` (`crates/nono-proxy/Cargo.toml:50`) | JWT-shaped phantom header/payload encoding (mirrors upstream's `jwt.rs`) | not yet used in `reverse.rs`; used elsewhere in the crate |
| `getrandom` | workspace `0.4` (`Cargo.toml:64`, `crates/nono-proxy/Cargo.toml:49`) | Phantom-token random-byte generation (mirrors `generate_phantom()`) | zeroize-adjacent RNG already a workspace dep |
| `url` | `2` (`crates/nono-proxy/Cargo.toml:52`) | Already used in `reverse.rs` for `parse_upstream_url`; also has `form_urlencoded` for request-nonce-field rewriting (mirrors `rewrite_form_request_body`) | `reverse.rs:1211-1246` |
| `serde_json` | workspace (already a `nono-proxy` dep, used throughout `config.rs`) | JSON dot-path traversal for response-field rewrite | ubiquitous |
| `zeroize` | workspace (already used in `reverse.rs` via `Zeroizing<String>`) | D-08: real tokens held `Zeroizing<Vec<u8>>` | `reverse.rs:33,443,735,993` (session token, request-build buffers) |

**Verification note:** `getrandom = "0.4"` is a workspace-level pin (`Cargo.toml:64`); confirm the
exact patch version with `cargo tree -p nono-proxy -i getrandom` at plan time if the executor needs
an exact SemVer string — not re-verified against the live registry in this pass since no new
dependency is being added (existing lockfile entry governs).

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled dot-path JSON traversal (`value_at_path_mut`, ~12 lines, upstream-proven) | `jsonpath`/`jsonpath-rust` crate | Adds a new dependency for a problem upstream already solved in 12 lines with zero deps; D-07 says port the *logic*, not add a dependency — reject |
| Custom base64 JWT-shape construction | `jsonwebtoken` crate (real signing) | The phantom JWT is deliberately `alg: none` (never verified, only shape-matched for SDKs that parse-but-don't-verify) — a real signing library is over-engineering for a token nobody is meant to cryptographically trust; upstream's own `jwt.rs` (19 lines) confirms this is the right scope |

## Package Legitimacy Audit

**No external packages are being added.** All logic in scope (Priority 2/6 below) is built from
crates already present in `crates/nono-proxy/Cargo.toml` (`base64`, `getrandom`, `url`,
`serde_json`, `zeroize`). The Package Legitimacy Gate protocol therefore does not apply to this
phase — there is nothing for `slopcheck`/`npm view`/`pip index versions` to check. If the planner's
discretion (buffer-cap configurability, helper-command mechanism) ends up requiring a new crate,
re-run the gate at that time.

## Architecture Patterns

### System Architecture Diagram

```
Sandboxed client (agent)
      │  plain HTTP (localhost:PORT)
      ▼
server.rs::handle_connection()
      │
      ├─ CONNECT ──────► is_route_upstream(host_port)? ──yes──► 403 (existing, unconditional
      │                        │                                  block for ALL route upstreams,
      │                        no                                 incl. future capture routes)
      │                        ▼
      │                  external-proxy-chain / bypass-route / plain CONNECT
      │                  (never reaches a route; D-06 needs no new check here)
      │
      ├─ absolute http:// ──► handle_forward_http()
      │                             │
      │                             ├─ require_auth gate (WR-13)
      │                             ├─ spiffe_declared_for_upstream() 403  ◄── D-03 precedent
      │                             ├─ [NEW] capture_declared_for_upstream() 403  ◄── D-06 (THIS PHASE)
      │                             └─ transparent 4th [u8;8192] relay loop (server.rs:1209-1230)
      │                                (NOT a rewrite target — arbitrary-host traffic, no route)
      │
      └─ origin-form (route configured) ──► reverse::handle_reverse_proxy()
                                                   │
                                                   ├─ L7 endpoint filter / endpoint_policy
                                                   ├─ route.has_spiffe_source()? ──► handle_spiffe_route()
                                                   ├─ credential_store.get_spiffe_assertion()? ──► handle_spiffe_assertion_credential()
                                                   ├─ [NEW] route/cred declares capture? ──► buffer-and-rewrite path (THIS PHASE)
                                                   │        │
                                                   │        ├─ read upstream response into Zeroizing<Vec<u8>>,
                                                   │        │  capped at D-05's bound — cap exceeded ⇒ DENY (502/403), never release
                                                   │        ├─ reject if Content-Encoding present (3c59c62e lesson) ⇒ DENY
                                                   │        ├─ split header/body on \r\n\r\n
                                                   │        ├─ rewrite configured response_fields to phantoms
                                                   │           (opaque or JWT-shaped), fail closed if an
                                                   │           unrewritten token-shaped field remains
                                                   │        ├─ recompute Content-Length, reassemble
                                                   │        └─ single stream.write_all (buffered, not chunked)
                                                   └─ else: existing 3-site unbuffered [u8;8192] relay
                                                        (static_cred handler / handle_spiffe_route /
                                                        handle_spiffe_assertion_credential — UNCHANGED
                                                        for non-capture routes, preserves SSE/MCP/A2A streaming)
      ▲
      │  real credential injected outbound (existing phantom machinery, OPPOSITE direction —
      │  proxy → upstream, not upstream → proxy)
      ▼
Upstream OAuth token endpoint (configured route, plaintext-visible via proxy's own tls_stream)
```

### Recommended Project Structure

No new files are structurally required — extend existing modules in place, matching Phase 113's
"reverse.rs / route.rs / credential.rs" pattern (113-CONTEXT.md, D-07 confirmation above):

```
crates/nono-proxy/src/
├── config.rs        # + CaptureConfig / CaptureTokenEndpointConfig / CaptureResponseFieldConfig
│                     #   (adapted from upstream's OAuthCaptureConfig/OAuthTokenEndpointConfig/
│                     #   OAuthTokenResponseFieldConfig, config.rs:264-320 in 9b692e07)
├── route.rs          # + LoadedRoute.declares_capture: bool (mirrors declares_spiffe, route.rs:78)
│                     # + RouteStore::capture_declared_for_upstream() (mirrors
│                     #   spiffe_declared_for_upstream, route.rs:300-309)
├── reverse.rs         # + shared buffer-and-rewrite helper; called from all 3 relay sites
├── capture.rs        # [NEW, small] phantom store: generate/store/resolve, JSON dot-path rewrite,
│                     #   JWT-shaped phantom construction — the D-07 portable logic, adapted
│                     #   (NOT a port of oauth_capture/mod.rs's persist-coupled shape)
├── server.rs         # + capture_declared_for_upstream() guard in handle_forward_http (mirrors
│                     #   the spiffe_declared_for_upstream block at server.rs:1043-1077)
└── audit.rs          # + capture_context: Option<CaptureAuditContext> on EventContext (Default-
                        #   derived, additive — mirrors spiffe_context, audit.rs:48)

crates/nono/src/undo/types.rs   # + CaptureAuditContext (pure data, mirrors SpiffeAuditContext,
                                  #   types.rs:283-299) + new NetworkAuditDenialCategory variant(s)
                                  #   (mirrors SpiffeUnsupportedPath, types.rs:253-257)

crates/nono-cli/src/profile/mod.rs   # + CustomCredentialDef.capture field, next mutual-exclusion
                                       #   arm in validate_custom_credential() (mod.rs:1131-1173)

crates/nono-cli/data/nono-profile.schema.json   # + CaptureConfig $defs entry (D-13)
```

### Pattern 1: Guard-then-dispatch, mirroring `has_spiffe_source()`

**What:** A route (or `CustomCredentialDef`) carries a boolean-derivable "declares capture"
predicate, computed once at `RouteStore::load()`/profile-parse time, never per-request.
**When to use:** Exactly the D-06 dispatch check and the D-06 cross-path guard.
**Example (existing code, the pattern to mirror verbatim):**
```rust
// crates/nono-proxy/src/route.rs:81-90 (verified live)
impl LoadedRoute {
    #[must_use]
    pub fn has_spiffe_source(&self) -> bool {
        self.declares_spiffe
    }
}

// crates/nono-proxy/src/route.rs:293-309 (verified live)
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
A `capture_declared_for_upstream()` following this exact shape (with `has_capture_source()`
substituted) is the minimal-diff way to satisfy D-06.

### Pattern 2: Response header/body split (upstream design reference, port the LOGIC not the file)

**What:** Locate `\r\n\r\n`, split status-line+headers from body, detect
`Transfer-Encoding: chunked` and `Content-Encoding`, decode chunked bodies, reject content-encoded
bodies unconditionally, recompute `Content-Length` after rewrite.
**When to use:** The shared buffer-and-rewrite helper this phase builds.
**Example (upstream `forward.rs`, CITED — design reference only, not portable code since
`ResponseRewrite`/`forward_request_with_response_rewrite` depend on the absent `forward.rs`
abstractions):**
```rust
// git show 9b692e07:crates/nono-proxy/src/forward.rs (lines ~373-425)
fn rewrite_http1_response(raw: &[u8], rewrite: ResponseRewrite<'_>) -> Result<Vec<u8>> {
    let Some(header_end) = find_header_end(raw) else { /* ... */ };
    let head = &raw[..header_end];
    let mut body = raw[header_end + 4..].to_vec();
    // ... parse status line, headers, detect chunked/content-encoded ...
    if chunked {
        body = decode_chunked_body(&body)?;
    }
    // 3c59c62e HARDENING: this fork must reject content_encoded for ALL
    // statuses, not just 200..300 (upstream's own follow-up commit widened
    // this after finding the 2xx-only guard was too narrow):
    if content_encoded {
        return Err(ProxyError::HttpParse(
            "cannot safely rewrite or inspect content-encoded response".to_string(),
        ));
    }
    let rewritten_body = rewrite(status, &headers, &body)?;
    // ... reassemble with recomputed Content-Length ...
}
```

### Pattern 3: JSON dot-path field rewrite + fail-closed unrewritten-token rejection (D-07 portable logic)

**What:** Walk a `serde_json::Value` by dot-separated path, replace configured token fields with
minted phantoms, then defensively scan the WHOLE response body for any *unconfigured*
`access_token`/`refresh_token`/`id_token`-named field still holding non-empty string content — and
fail closed if found.
**When to use:** The response-rewrite step, directly ported (this is real portable logic per D-07 —
pure functions over `&[u8]`/`serde_json::Value`, no dependency on `forward.rs` or `oauth_capture/`
internals).
**Example (upstream `oauth_capture/rewrite.rs`, CITED, adapt directly):**
```rust
// git show 9b692e07:crates/nono-proxy/src/oauth_capture/rewrite.rs (verified, full read)
fn value_at_path_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut current = root;
    for part in path.split('.') {
        if part.is_empty() { return None; }
        current = current.as_object_mut()?.get_mut(part)?;
    }
    Some(current)
}

fn is_sensitive_token_field(field: &str) -> bool {
    matches!(field, "access_token" | "refresh_token" | "id_token")
}
// reject_unrewritten_token_fields_inner() walks the whole tree recursively,
// erroring if a sensitive-named field outside `configured_paths` still holds
// non-empty string content — the fail-closed backstop for provider config drift.
```

### Anti-Patterns to Avoid
- **Reusing the outbound phantom machinery for inbound rewrite:** the fork's existing
  `validate_phantom_token`/`inject_credential_for_mode` (`reverse.rs:1118-1192`, `~450`) implement
  **proxy → upstream** injection (agent presents session-token-as-phantom, proxy substitutes the
  real credential going out). OAuth capture is **upstream → proxy → agent**: the proxy must mint a
  *new, per-real-token* phantom and store a reverse mapping. These are different data structures
  (`CredentialStore.get()` returns ONE credential per route prefix; capture needs a
  `HashMap<phantom, real>` store, mirroring upstream's `OAuthCaptureStore.phantoms`). Do not try to
  bolt capture onto `LoadedCredential`/`CredentialStore::get()` — build a sibling store.
- **Chunked-vs-buffered ambiguity:** never let a capture-declared route silently fall through to
  the existing unbuffered streaming loop on any error path — every `?`/early-return in the new
  buffer-and-rewrite helper must resolve to an explicit deny response, never a
  "forward what we have so far" fallback (that would define away D-05's whole purpose).
- **Trusting `Content-Length` alone for the cap:** D-05's cap must be enforced against the
  *actual bytes read* (mirroring `MAX_REQUEST_BODY`'s existing pattern at `reverse.rs:387-391`,
  which checks the declared `Content-Length` AND still bounds the read), not just an upfront header
  check — an upstream that lies about `Content-Length` or omits it (chunked) must not bypass the cap.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON dot-path traversal | A new path-expression parser/crate | The 12-line `value_at_path_mut` pattern (Pattern 3 above), ported from upstream | Upstream already solved this narrowly-scoped problem with zero dependencies; adding a `jsonpath` crate for single-segment dot-paths is over-engineering |
| Chunked-transfer decoding | Hand-write from scratch | Port upstream's `decode_chunked_body()` (`forward.rs`, CITED — not yet read in full this session; grep-locate before use) | HTTP/1.1 chunked decoding has well-known edge cases (trailer headers, chunk-extension syntax); upstream's implementation is already proven against `9b692e07`'s and `3c59c62e`'s own tests |
| Constant-time phantom comparison | Ad-hoc `==` | Reuse `token::constant_time_eq` (already in `crates/nono-proxy/src/token.rs`, used throughout `reverse.rs`) | Existing, tested, avoids a fresh timing-side-channel review |

**Key insight:** the portable 20% of upstream's OAuth-capture code (JSON rewrite logic, JWT
shaping, chunked decode, content-encoding rejection) is genuinely reusable low-level HTTP/JSON
plumbing with no coupling to the absent `oauth_capture`/`forward.rs`/`tls_intercept` subsystems.
The other 80% (persistence, the `ResponseRewrite` closure-injection abstraction, provider
lifecycle) is exactly what D-07/D-08/D-12 correctly identify as not portable.

## Common Pitfalls

### Pitfall 1: The three relay sites silently diverge over time
**What goes wrong:** A future edit fixes the rewrite bug in `handle_reverse_proxy`'s copy but
misses `handle_spiffe_route`'s or `handle_spiffe_assertion_credential`'s — exactly the WR-13 defect
shape (`112-REVIEW.md`), just at a different granularity.
**Why it happens:** The three loops are currently hand-duplicated (verified: `reverse.rs:474-498`,
`764-785`, `1019-1040` are byte-for-byte structurally identical except variable names).
**How to avoid:** Extract ONE shared function taking `&mut TlsStream`, `&mut TcpStream`, and a
`Option<&CaptureConfig>` (or similar), called from all three sites — do not duplicate the new
buffering logic a fourth time.
**Warning signs:** A grep for `[u8; 8192]` in `reverse.rs` returning more than one distinct
buffer-and-rewrite implementation after this phase lands.

### Pitfall 2: Content-Encoding bypass (a real upstream-discovered defect class)
**What goes wrong:** A gzip/br-encoded token response is buffered, the rewrite step tries to find
`access_token` in the compressed bytes, finds nothing, and the ORIGINAL (still-encoded, still
containing the real token once decoded client-side) body is released unrewritten.
**Why it happens:** Upstream's OWN commit `3c59c62e` had to widen a `(200..300).contains(&status)`
condition to unconditional after finding the narrower check missed non-2xx encoded bodies —
this is a proven, not hypothetical, defect class in exactly this code shape.
**How to avoid:** Reject (fail closed, deny the response) on ANY non-empty `Content-Encoding`
header, regardless of status code, from the first implementation — do not ship the narrower
2xx-only check and "harden later."
**Warning signs:** A test with `Content-Encoding: gzip` + a non-2xx status that still returns 200
success from the rewrite helper.

### Pitfall 3: `ProxyLaunchOptions`/`PreparedSandbox` do not match upstream's diff context
**What goes wrong:** Reading `git show 9b692e07 -- crates/nono-cli/src/{launch_runtime,
sandbox_prepare,main,policy}.rs` shows hunks that add `credential_providers`/`credential_routes`
fields *alongside* an EXISTING `credential_capture: HashMap<String, profile::CredentialCaptureEntry>`
field and an EXISTING `tls_intercept: Option<TlsInterceptConfig>` field. **Neither field exists in
the fork's actual `ProxyLaunchOptions`** (verified: `grep -n credential_capture
crates/nono-cli/src/*.rs` returns zero hits; `crates/nono-cli/src/launch_runtime.rs:100-140`'s full
struct body has no `tls_intercept`/`credential_capture` field). These upstream diff hunks are
therefore not just non-`git apply`-able (already known, D-07) — the diff CONTEXT LINES themselves
describe a struct shape the fork never had at all, from an earlier, separately-unabsorbed upstream
feature.
**Why it happens:** Upstream's `ProxyLaunchOptions` has evolved through commits this fork never
synced (the "credential_capture" CLI-command-driven credential-provider feature is older and
distinct from this phase's declarative OAuth-capture-via-response-rewrite feature — do not
conflate the two).
**How to avoid:** When planning the `nono-cli` plumbing task (threading capture config from
profile → `ProxyLaunchOptions` → `PreparedSandbox` → `build_proxy_config_from_flags`), read the
FORK's actual current struct bodies (`launch_runtime.rs:100-215`ish, `sandbox_prepare.rs`'s
`PreparedSandbox`) rather than trusting upstream's diff hunks as a field-by-field checklist. The
existing `custom_credentials: HashMap<String, profile::CustomCredentialDef>` threading (verified
live at `launch_runtime.rs:117`, `proxy_runtime.rs:227,382`) is the correct pattern to mirror.
**Warning signs:** A task description that says "add `credential_providers` next to
`credential_capture`" without first grepping for `credential_capture` in the live fork tree.

### Pitfall 4: Host:port exact-match vs. host-only-match for the D-06 guard
**What goes wrong:** Upstream's own `3c59c62e` hardened `OAuthCaptureStore::host_policy()` from
exact `host:port` matching to host-only matching (`host_policy_matches_capture_host_on_any_port`
test, CITED) after finding a capture-declared host reachable on a non-configured port would
otherwise slip past. The fork's existing `is_route_upstream`/`spiffe_declared_for_upstream`
precedent (which THIS phase's `capture_declared_for_upstream` is designed to mirror) uses EXACT
`host:port` matching (`route.rs:273-281`, `293-309`).
**Why it happens:** These are two different design choices upstream made for two different
purposes (credential-injection routing needs exact-port matching because the upstream URL is
precise; capture-detection for a fail-closed DENY arguably wants to be broader, since ANY port on
a capture host reaching a non-rewrite path is still a leak).
**How to avoid:** This is a genuine open design question for the planner/discuss-phase, not
something this research resolves — flag it explicitly rather than silently picking exact-match
"because that's what SPIFFE did." Document the choice made and why in ADR-114.
**Warning signs:** A test that only checks the exact configured port and never checks a
capture-declared host reached on a *different* port via `handle_forward_http`.

### Pitfall 5: Zeroizing the NEW buffer, not just the existing request-build buffers
**What goes wrong:** The existing `reverse.rs` request-build buffers already use
`Zeroizing<String>`/`Zeroizing<Vec<u8>>` (`reverse.rs:33,443,735,993`), but the existing
**response**-side code (`response_buf: [u8; 8192]`, the per-chunk stack array) does NOT
accumulate into a heap buffer at all today — it's write-through, chunk by chunk. The NEW
accumulator this phase introduces (to hold the FULL response for rewrite) is a genuinely new heap
allocation holding a real OAuth token in cleartext, and is easy to build as a plain `Vec<u8>` by
analogy with the existing (non-secret) `body` request-read buffer at `reverse.rs:392-401` (which
is NOT `Zeroizing`, because it holds request bodies, not credentials).
**Why it happens:** The nearest-by-analogy code pattern (`body` at `reverse.rs:392-401`) is
deliberately unzeroized because it's not secret material — copying that pattern for the response
accumulator would be a CLAUDE.md violation (mandatory `zeroize` for secrets) and contradicts D-08.
**How to avoid:** Wrap the new response accumulator in `Zeroizing<Vec<u8>>` from the first line of
code, never `Vec<u8>`.
**Warning signs:** A buffer-and-rewrite helper signature that takes/returns plain `Vec<u8>`.

## Code Examples

### D-06 guard precedent — `handle_forward_http`'s existing SPIFFE check (mirror exactly)
```rust
// crates/nono-proxy/src/server.rs:1043-1077 (verified live, full function context read)
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
            denial_category: Some(nono::undo::NetworkAuditDenialCategory::SpiffeUnsupportedPath),
            ..audit::EventContext::default()
        },
        &host, port,
        "SPIFFE-declared route upstream: forward-HTTP path has no SPIFFE implementation",
    );
    let response = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;
    return Ok(());
}
```
D-06's new check is the same shape with `capture_declared_for_upstream` /
`NetworkAuditDenialCategory::CaptureUnsupportedPath` (new variant, name TBD by planner) substituted.
Note this check is deliberately **unconditional** — not gated on `state.config.require_auth` — for
the same reason the SPIFFE check isn't: the property being enforced (this route only makes sense
through the reverse-proxy path) doesn't depend on whether session-token auth happens to be enabled.

### `CustomCredentialDef` mutual-exclusion pattern to extend (D-12's landing site)
```rust
// crates/nono-cli/src/profile/mod.rs:1150-1173 (verified live)
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
```
A `capture` field on `CustomCredentialDef` most likely does NOT need to be mutually exclusive with
`credential_key`/`auth`/`aws_auth`/`spiffe` in the same sense — capture is about the RESPONSE
direction (what the route returns to the client), while the other four are about the REQUEST
direction (what credential the proxy injects outbound). A route could plausibly have both a
`credential_key` (for a different, already-authenticated endpoint) and a `capture` config (for its
token endpoint) — or capture may need its OWN dedicated route with no other auth field at all,
since the whole point is the client has no prior credential yet. **This is a genuine open design
question for the planner** — do not assume mutual exclusion by analogy without checking whether it
makes semantic sense for capture specifically.

### Portable JWT-shaped phantom construction (D-07, verified via full-file read)
```rust
// git show 9b692e07:crates/nono-proxy/src/oauth_capture/jwt.rs — CITED, full file (19 lines)
pub(super) fn jwt_shaped_phantom(phantom: &str) -> Result<String> {
    let header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let payload = serde_json::json!({
        "iss": "nono", "sub": phantom, "aud": "nono",
        "iat": 0, "exp": 4_102_444_800_u64
    });
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&payload).map_err(...)?);
    Ok(format!("{header}.{payload}.{phantom}"))
}
```
Directly adaptable — uses only `base64`/`serde_json`, both already `nono-proxy` deps.

### Schema round-trip test template (D-13, exact template to copy)
```rust
// crates/nono-cli/src/profile/mod.rs:7396-7434 (verified live — the exact template to copy
// for a capture-config equivalent)
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
`validate_against_schema()` itself: `crates/nono-cli/src/profile/mod.rs:9465-9482` (loads
`crate::config::embedded::embedded_profile_schema()`, compiles via `jsonschema::validator_for`,
collects `iter_errors`).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Assumed forward.rs's `ResponseRewrite` hook is the *only* possible enforcement point (112-OAUTH-CAPTURE-DISPOSITION.md §1c's framing) | Fork already has plaintext response visibility via `reverse.rs`'s own `tls_stream` — buffering (not visibility) was the actual missing piece | D-01r, this phase's discussion (2026-08-06) | Unlocks an ADAPT disposition instead of a DECLINE; the retraction is itself part of the historical record ADR-114 must preserve (see CONTEXT.md D-01r) |
| Upstream's `content_encoded && (200..300).contains(&status)` rewrite-rejection gate | Unconditional `content_encoded` rejection (any status) | `3c59c62e` (2026-07-05, upstream) | The fork-native helper must ship with the WIDER (unconditional) gate from day one — Pitfall 2 above |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `getrandom = "0.4"` workspace pin resolves to a SemVer that supports the `getrandom::fill()` API upstream's `generate_phantom()` uses | Standard Stack | Low — if the API differs, `cargo build` fails immediately and loudly (compile error, not a silent behavior change); trivial to fix at implementation time |
| A2 | D-06's host:port exact-match vs. host-only-match choice (Pitfall 4) is left to the planner/ADR-114, not resolved here | Pitfall 4 | Medium — if the planner silently copies the SPIFFE exact-match pattern without considering upstream's own hardening lesson, a capture-declared host reachable on a non-configured port could bypass the fail-closed guard via `handle_forward_http` |
| A3 | A `capture` field on `CustomCredentialDef`/`RouteConfig` does NOT need the same mutual-exclusion treatment as `spiffe`/`aws_auth`/`auth` (Code Examples, mutual-exclusion pattern) | Code Examples | Medium — if wrong, the planner should add capture to the mutual-exclusion arm instead of allowing composition; this needs explicit resolution before implementation, not silent assumption either way |

**Note on the `[ASSUMED]` tagging convention:** the three items above are genuine open design
questions this research surfaces but does not resolve — they are not claims about external
package identity or unverified library behavior (this phase adds no new packages), so they are
tracked here as planning-discretion items rather than provenance-tagged factual claims.

## Open Questions

1. **Where does the phantom-token store live structurally — a new `capture.rs` module, or inside
   `credential.rs`/`route.rs`?**
   - What we know: the fork's existing pattern is one module per concern (`credential.rs` for
     outbound credential lookup, `route.rs` for route-level config, `oauth2.rs` for
     client_credentials token exchange). A new phantom store (mapping minted phantoms → real
     captured tokens, session-scoped, zeroized) doesn't cleanly fit any existing module's stated
     purpose.
   - What's unclear: whether Phase 113's precedent (adding `spiffe.rs` as a genuinely new module)
     argues for a new `capture.rs`, or whether the store is small enough to nest inside
     `reverse.rs` itself (which already owns the enforcement point).
   - Recommendation: new module (`capture.rs`), mirroring Phase 113's `spiffe.rs` precedent —
     keeps `reverse.rs` from growing past its current 2228 lines further than necessary, and gives
     the phantom-store's own unit tests (mint/resolve/admit) a natural home independent of the
     wire-protocol relay code.

2. **What is the exact D-05 buffer-cap value, and is it configurable?**
   - What we know: CONTEXT.md's discretion range is "~256 KiB–1 MiB"; the fork's existing
     `MAX_REQUEST_BODY` is 16 MiB (`reverse.rs:36`); upstream's own `MAX_REWRITE_RESPONSE_BYTES`
     is 16 MiB too (`forward.rs`, CITED) — but upstream's cap exists for a DIFFERENT reason
     (general safety net on a feature that also proxies non-token traffic through the same rewrite
     path in some configurations), not tuned specifically for token-response size.
   - What's unclear: whether any real-world OAuth token response (Anthropic/OpenAI/GitHub-shaped)
     ever legitimately exceeds ~256 KiB — this research did not find live data on that.
   - Recommendation: the planner should pick a concrete value (256 KiB is a defensible, generous
     default for a JSON object holding a handful of JWTs) and make it configurable via profile
     (mirrors the pattern of every other numeric limit in `config.rs`), so an operator hitting a
     legitimately larger provider response isn't forced to a code change.

3. **Does `handle_forward_http`'s existing 4th relay loop (`server.rs:1209-1230`) need ANY
   capture-awareness beyond the D-06 deny guard, or is deny-at-request-time fully sufficient?**
   - What we know: D-06 already specifies deny-at-request-time for this path; the loop itself is
     never reached for a capture-declared route once the guard fires.
   - What's unclear: nothing structurally — this is fully resolved by D-06 as locked. Listed here
     only so the planner doesn't accidentally ALSO try to add buffering logic to this 4th loop
     (which would be scope creep beyond D-06's stated boundary).
   - Recommendation: explicitly confirm in the plan that this loop remains untouched.

## Environment Availability

No new external tools, services, or runtimes are required for this phase — it is a pure Rust
code change within the existing workspace plus two sibling-repo binding rebuilds. Skipped per the
skip condition (code/config changes with build-tool dependencies already verified present in prior
phases — `maturin`/`napi` availability was confirmed working as recently as Phase 113,
`113-08-SUMMARY.md:154-155`).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) + `cross test` for Linux-gated paths |
| Config file | none — standard `#[cfg(test)] mod tests` per crate, matches every other `nono-proxy`/`nono-cli` module |
| Quick run command | `cargo test -p nono-proxy reverse::tests -- --nocapture` (or the equivalent new `capture::tests` module) |
| Full suite command | `make ci` (clippy + fmt + tests, per CLAUDE.md), plus both cross-target clippy gates (SC4) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|-------------|
| SEC-02 (SC2, D-05) | A capture-declared route's response exceeding the buffer cap is DENIED, never released unrewritten | unit | `cargo test -p nono-proxy capture_buffer_cap_exceeded_denies_response` | ❌ Wave 0 |
| SEC-02 (SC2, D-07/Pattern 3) | A configured `response_fields` path in a real token response is rewritten to a phantom; an UNCONFIGURED but token-shaped field left unrewritten fails closed | unit | `cargo test -p nono-proxy capture_rewrites_configured_fields` / `capture_fails_closed_on_unrewritten_token_field` | ❌ Wave 0 |
| SEC-02 (Pitfall 2, 3c59c62e lesson) | A `Content-Encoding`-present response on a capture-declared route is denied (any status code, not just 2xx) | unit | `cargo test -p nono-proxy capture_denies_content_encoded_response` | ❌ Wave 0 |
| SEC-02 (D-06, all 3 relay sites) | All three `reverse.rs` credential dispatch paths (static_cred, `handle_spiffe_route`, `handle_spiffe_assertion_credential`) apply the same rewrite when their route/cred declares capture | integration | `cargo test -p nono-proxy all_three_relay_sites_apply_capture_rewrite` (or 3 discrete tests, one per site — matches WR-13's "prove every site independently" lesson) | ❌ Wave 0 |
| SEC-02 (D-06, cross-path guard) | A capture-declared route's upstream reached via CONNECT is already blocked by the pre-existing `is_route_upstream` check (regression test only — no new guard needed there) | unit | `cargo test -p nono-proxy connect_denies_capture_declared_route_upstream` | ❌ Wave 0 |
| SEC-02 (D-06, cross-path guard, NEW) | A capture-declared route's upstream reached via `handle_forward_http` (absolute-form `http://`) is denied at request time | unit | `cargo test -p nono-proxy forward_http_denies_capture_declared_route_upstream` (mirrors `d03_forward_http_denies_spiffe_declared_route_upstream`, Plan 113-05) | ❌ Wave 0 |
| SEC-02 (D-08) | A resolved/stored real token is `Zeroizing`-wrapped and the phantom store never persists to disk | unit | `cargo test -p nono-proxy capture_store_holds_only_in_memory` (assert no `persist_path`-equivalent field exists / no file I/O in the store's `load`/`store_phantom` path) | ❌ Wave 0 |
| SEC-02 (SC3, ADR-86) | No policy/enforcement logic lands in `crates/nono/src/`; any new audit context type there is pure data | manual review + existing doctest pattern | `grep -rn "if \|match " crates/nono/src/undo/types.rs` (spot-check: new `CaptureAuditContext` has no branching logic) | N/A — review task, not a test file |
| SEC-02 (SC4) | Both cross-target clippy gates GREEN | manual gate | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` + `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | N/A — see `.planning/templates/cross-target-verify-checklist.md` |
| SEC-02 (D-13) | A realistic capture-provider profile deserializes AND passes `validate_against_schema()` | unit | `cargo test -p nono-cli test_schema_validates_capture_custom_credential` (mirrors `test_schema_validates_spiffe_custom_credential`, `mod.rs:7396`) | ❌ Wave 0 |
| SEC-02 (D-14) | `../nono-py` and `../nono-ts` rebuild green after `RouteConfig` gains a `capture` field | build gate (not `cargo test`) | `cd ../nono-py && uv run maturin develop` (or `maturin build`); `cd ../nono-ts && napi build --platform --release` | N/A — build commands, not test files |

### Sampling Rate
- **Per task commit:** `cargo test -p nono-proxy` (fast subset covering the module just touched)
- **Per wave merge:** `make ci` (full workspace clippy + fmt + tests)
- **Phase gate:** Full suite green + both cross-target clippy gates green + both binding rebuilds
  green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/nono-proxy/src/capture.rs` (or equivalent module) — new file, needs its own
      `#[cfg(test)] mod tests` for phantom mint/resolve/admit, mirroring
      `oauth_capture/mod.rs`'s test shape (CITED) adapted to the fork's config types
- [ ] Buffer-and-rewrite helper tests in `reverse.rs` — cap-exceeded, content-encoded-rejected,
      configured-field-rewritten, unconfigured-token-field-fails-closed (4 cases minimum)
- [ ] `server.rs` D-06 guard tests — mirror `d03_forward_http_denies_spiffe_declared_route_upstream`
      and `d03_connect_denies_spiffe_declared_route_upstream` (Plan 113-05) exactly, substituted for
      capture
- [ ] `crates/nono-cli/src/profile/mod.rs` schema round-trip test — mirror
      `test_schema_validates_spiffe_custom_credential` (`mod.rs:7396-7415`)
- Framework install: none — `cargo test` already fully configured; no new test-only dependency
  anticipated (D-07's portable logic uses only existing crates)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | yes | Existing `token::validate_proxy_auth`/phantom-token validation, unchanged by this phase — capture rides the same auth gate as every other reverse-proxy route (`ctx.require_auth` check, `reverse.rs:255`) |
| V3 Session Management | yes | D-08: captured real tokens are session-scoped, in-memory only, zeroized on drop and on proxy restart — no session persistence across restarts (accepted cost, explicitly recorded) |
| V4 Access Control | yes | D-06's fail-closed cross-path guard IS an access-control control — a capture-declared route reachable only through the reverse-proxy path, never CONNECT/forward-HTTP/external-proxy-chain |
| V5 Input Validation | yes | Response body must be valid JSON with expected structure to be safely rewritten; malformed/oversized/content-encoded input fails closed rather than being forwarded (Pitfalls 1, 2) |
| V6 Cryptography | partial | JWT-shaped phantom is deliberately `alg: none` (never a real signature) — not a cryptographic control, a shape-compatibility shim for SDKs that parse-but-don't-verify JWTs; the REAL secret material (`zeroize`-wrapped) is the actual protected asset, not the phantom's shape |

### Known Threat Patterns for this phase's stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| Real OAuth token forwarded unrewritten to the sandboxed client (SC2's named forbidden outcome) | Information Disclosure | Buffer-then-rewrite with fail-closed-on-any-error (never partial-forward); the whole point of this phase |
| Content-Encoding bypass of the rewrite scan (Pitfall 2, upstream-proven defect class) | Information Disclosure | Unconditional `Content-Encoding` rejection, any status code |
| Capture-declared route reached via a non-rewrite path (CONNECT/forward-HTTP/external-proxy-chain) | Information Disclosure / Elevation of Privilege (bypasses the ONLY enforcement point) | D-06's fail-closed cross-path guard — the SC2 "forbidden outcome" applies equally to a bypass path as to a dropped hook |
| Oversized response used as a buffering-DoS vector | Denial of Service | D-05's tight cap (256 KiB–1 MiB range), enforced against actual bytes read (Pitfall in Anti-Patterns above), not just a `Content-Length` header check |
| Real token lingering in heap memory after use (cold-boot/core-dump/swap exposure) | Information Disclosure | `Zeroizing<Vec<u8>>` for the response accumulator and the phantom store's `real` field, mirroring `StoredOAuthToken.real: Zeroizing<Vec<u8>>` (CITED, `oauth_capture/mod.rs`) |
| Phantom resolved for a non-admitted consumer (cross-provider token leak) | Elevation of Privilege | Mirror upstream's `admitted_consumers: HashSet<String>` per-phantom scoping (CITED, `oauth_capture/mod.rs`'s `NonceResolver::resolve` checks `admitted_consumers.contains(consumer)` before returning) |

## Sources

### Primary (HIGH confidence — direct read of the live fork tree this session)
- `crates/nono-proxy/src/reverse.rs` (full file, 2228 lines) — all 3 response-relay loops, the
  auth/route dispatch chain, `parse_response_status`/`extract_content_length`/`filter_headers`
- `crates/nono-proxy/src/server.rs` (`handle_forward_http` full function, `handle_connection` full
  function, D-03 SPIFFE guard comments and code) — all request-arrival-path dispatch
- `crates/nono-proxy/src/route.rs` (full `RouteStore`/`LoadedRoute` — `has_spiffe_source`,
  `is_route_upstream`, `spiffe_declared_for_upstream`)
- `crates/nono-proxy/src/config.rs` (`RouteConfig`, `OAuth2Config`, `SpiffeAuthConfig` — full
  struct bodies)
- `crates/nono-proxy/src/audit.rs` (`EventContext`, `ProxyMode`, log function signatures)
- `crates/nono/src/undo/types.rs` (`NetworkAuditDenialCategory`, `SpiffeAuditContext`,
  `SpiffeDelegationContext` — pure-data doc-comment pattern)
- `crates/nono-cli/src/profile/mod.rs` (`CustomCredentialDef` full struct,
  `validate_custom_credential` full function, `validate_against_schema` helper, SPIFFE schema
  round-trip tests at line 7396+)
- `crates/nono-cli/data/nono-profile.schema.json` (`$defs.CustomCredentialDef`,
  `$defs.SpiffeAuthConfig`, `additionalProperties: false` posture)
- `crates/nono-cli/src/launch_runtime.rs` (`ProxyLaunchOptions` full struct body — confirmed
  absence of `credential_capture`/`tls_intercept` fields)
- `crates/nono-cli/src/proxy_runtime.rs` (`build_proxy_config_from_flags` call site,
  `custom_credentials` threading)
- `../nono-py/src/proxy.rs` (`RouteConfig` exhaustive struct literal, `#[new]` constructor)
- `../nono-py/src/policy.rs` (`impl From<PolicyRouteConfig> for RustRouteConfig` — second
  exhaustive struct literal)
- `../nono-py/Cargo.toml`, `../nono-ts/Cargo.toml` (path-dependency confirmation)
- `../nono-ts/src/` (confirmed zero `nono_proxy` references — structurally immune, matches
  113-08-SUMMARY.md's finding)
- `.planning/phases/112-security-residual-sync/112-OAUTH-CAPTURE-DISPOSITION.md` (full read)
- `.planning/phases/112-security-residual-sync/112-REVIEW.md` (WR-13 full finding + resolution)
- `proj/ADR-113-spiffe-disposition.md` (D-01 positive-proof table, full read of Decision section)
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` (SEC-02 rows, SPIFFE
  Carry-Forward Note shape precedent for D-10)
- `.planning/templates/cross-target-verify-checklist.md` (full read — SC4's two gate commands)
- `git show 9b692e07` (full commit + `--stat`), `git show 9b692e07:crates/nono-cli/src/profile/credential_provider.rs`
  (full file), `git show 9b692e07:crates/nono-proxy/src/oauth_capture/{jwt,endpoint,rewrite,mod}.rs`
  (full files), `git show 9b692e07:crates/nono-proxy/src/config.rs` (`OAuthCaptureConfig` region),
  `git show 9b692e07:crates/nono-proxy/src/forward.rs` (header, `rewrite_http1_response`,
  `buffer_rewrite_response` regions) — all via local `upstream` remote, no fetch needed
- `git show 3c59c62e` (full diff) — hardening follow-up, `content_encoded` widening,
  `host_policy` any-port matching
- `git show d033c631` (full diff) — trivial test-fixture confirmation

### Secondary (MEDIUM confidence)
- None — this research relied entirely on direct source reads (Primary) and the phase's own
  locked CONTEXT.md/ROADMAP.md/prior-phase docs (also Primary, direct reads).

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, all existing crate versions confirmed via
  `Cargo.toml` grep
- Architecture (relay sites, dispatch paths, guard precedent): HIGH — every claim backed by a
  direct full-function read of the live tree, with file:line citations
- Portable-logic design reference (D-07): HIGH for what the code does (full-file reads of
  upstream source via local `git show`); this is CITED (design reference), not VERIFIED-as-portable
  until the planner/executor confirms it compiles against the fork's adapted types
- Pitfalls: HIGH — Pitfalls 1, 3, 5 are directly observed structural facts; Pitfall 2 is a
  upstream-proven defect class (their own commit fixed it); Pitfall 4 is a genuine open design
  tension between two verified-different upstream/fork precedents, correctly flagged as unresolved

**Research date:** 2026-08-06
**Valid until:** 30 days (stable, no external API surface; re-verify signatures if Phase 109's
`deny_domain` work or any other pending `nono-proxy` phase lands first and touches `reverse.rs`/
`server.rs`/`route.rs` — the phase depends on 112 and 113, both complete, but not on 109, which
remains pending per REQUIREMENTS.md traceability)
