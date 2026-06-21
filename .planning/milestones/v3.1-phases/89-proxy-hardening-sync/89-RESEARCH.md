# Phase 89: Proxy Hardening Sync - Research

**Researched:** 2026-06-20
**Domain:** Upstream-sync reconciliation of a 7-commit proxy-hardening cluster (Cluster F) against the fork's structurally divergent CONNECT-block + reverse-proxy L7 proxy (no `tls_intercept/` module)
**Confidence:** HIGH (all findings grounded in the fork's actual current source, read line-by-line this session; dispositions LOCKED in Phase 85 ledger)

## Summary

This is a **test-driven reconciliation** phase, not a feature build. The defining method (D-01)
is: for each upstream Cluster F intent, **default to no-op** — write a behavioral-equivalence note
plus a fork test proving the fork already delivers the intent; port the *behavioral intent* (never
upstream's `RouteSelection`/`TlsInterceptIntent` code shape) into the fork's CONNECT/reverse path
**only if a test exposes a real gap**.

I read every cited fork file this session. The findings are unambiguous:

- **#1197 / D-07 is the ONE clearly-real fix.** The activation gate in `proxy_runtime.rs:90-116`
  checks `credentials`, `network_profile`, `allow_domain`, `upstream_proxy` — but **not**
  `prepared.custom_credentials`. `custom_credentials` is already plumbed end-to-end (it is read at
  line 123 into `ProxyLaunchOptions`); only the activation predicate misses it. This is a one-line
  fix in two places (active branch + `--block-net` warn branch). [VERIFIED: fork source read]
- **#1077 / D-09, #1048-#1091 / D-01, #1151 / D-02 are all already-satisfied equivalences.** The
  fork returns `403 Forbidden` + `audit::log_denied` on endpoint default-deny (`reverse.rs:96-115`),
  honors `external_proxy` in the CONNECT path (`server.rs:534-561`), and its CONNECT auth is lenient
  by design — `connect.rs:46-48` logs the failure at debug and **continues** (keeps the connection
  open) rather than dropping it, which *is* #1151's "keep connection open for reactive proxy auth"
  behavior. [VERIFIED: fork source read]
- **#1132 / D-10 (allow_domain shadowing) cannot reproduce on the fork's model.** The fork dispatches
  reverse-proxy requests by **exact path prefix** (`parse_service_prefix` → exact-key `HashMap`
  lookup in `RouteStore`), and gives allow_domain endpoint routes a disjoint key namespace
  (`_ep_{domain}`, `network_policy.rs:389`) vs credential routes (the service name). There is no
  "longest-prefix-wins" or upstream-host route-selection where one route shadows another. Upstream's
  bug is intrinsic to its `select_route`/`RouteSelection` host-ordering abstraction — which the fork
  does not have. Expect: equivalence note + a disproof test, no fix. [VERIFIED: fork source read]
- **#1192 / D-05 and #1199 / D-04 are won't-sync** — `76b7b695` lives entirely in the absent
  `tls_intercept/handle.rs`; `bd4b6b7f` is an organizational intent/activation refactor carrying a
  `TlsInterceptIntent` struct the fork cannot back. Ledger-addendum only. [VERIFIED: Phase 85 ledger]

**Primary recommendation:** Land exactly ONE behavioral code change (#1197/D-07 activation predicate
+ regression test). Everything else is a behavioral-equivalence test + a Phase-85 ledger addendum
(D-11) recording equivalence / won't-sync. Do **not** import `RouteSelection`, `TlsInterceptIntent`,
or upstream's intent/activation separation.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Proxy activation gate (#1197) | CLI (`nono-cli/proxy_runtime.rs`) | — | Activation is a launch-time policy decision; lives in the CLI runtime, not the proxy crate |
| CONNECT dispatch + external_proxy honoring (#1048/#1091) | Proxy (`nono-proxy/server.rs`) | — | CONNECT-block model is a proxy-server responsibility |
| CONNECT lenient auth / keep-open (#1151) | Proxy (`nono-proxy/connect.rs`) | — | Tunnel auth is a connect-handler concern |
| Endpoint default-deny 403+audit (#1077) | Proxy (`nono-proxy/reverse.rs`) | — | L7 method+path filtering is the reverse handler's job |
| allow_domain route partition + RouteStore keying (#1132) | CLI (`network_policy.rs` partition) | Proxy (`route.rs` RouteStore) | Partition happens CLI-side; lookup happens proxy-side; both use exact-key, no host-selection |
| AWS SigV4 activation vs signing (#1197/D-08) | Proxy (`credential.rs`, `reverse.rs` 501 stub) | CLI (resolve_credentials) | Activation reaches the route; signing stays a 501 stub (deferred) |
| Won't-sync recording (D-04, D-05, D-11) | Docs (`85-DIVERGENCE-LEDGER.md` addendum) | — | Sync provenance, not code |

## Standard Stack

No new packages. This phase touches **only** existing fork crates and their existing test
infrastructure. The relevant existing dependencies (all already in `Cargo.lock`):

| Library | Purpose | Notes |
|---------|---------|-------|
| `tokio` (v1) | async runtime + `#[tokio::test]` + `tokio::io::duplex` test harness | Used by existing `connect.rs`/`reverse.rs` tests |
| `tokio-rustls` / `rustls` / `webpki-roots` / `rustls-native-certs` | TLS connectors in `route.rs` | Untouched this phase |
| `zeroize` | `Zeroizing<String>` for session token / secrets | Untouched |
| `url` | host:port extraction in `route.rs` | Untouched |

**Installation:** none. No `cargo add`. No Package Legitimacy Audit required (no external packages
installed this phase).

## Package Legitimacy Audit

**N/A — this phase installs no external packages.** All work is reconciliation against existing
fork code plus test additions using already-vendored dependencies. No `cargo add`, no `Cargo.toml`
dependency-line changes anticipated. If the planner discovers a need for a new dependency (it should
not), run the Package Legitimacy Gate before adding it.

## Architecture Patterns

### System Architecture Diagram (fork's actual proxy model — the reconciliation surface)

```
                    sandboxed child process
                            │
                            │ HTTP/CONNECT to localhost proxy
                            ▼
              ┌──────────────────────────────────────┐
              │ server.rs :: handle_connection        │
              │ (reads first line + headers)          │
              └───────────────┬──────────────────────┘
                              │ dispatch by method
            ┌─────────────────┴───────────────────────────┐
            │ first_line starts_with "CONNECT "            │ else (non-CONNECT)
            ▼                                              ▼
  ┌──────────────────────────┐              ┌──────────────────────────────┐
  │ CONNECT path (server.rs   │              │ reverse proxy (reverse.rs)    │
  │  445-597)                 │              │  handle_reverse_proxy          │
  │                           │              │                               │
  │ 1. is_route_upstream? →   │              │ parse_service_prefix(path)    │
  │    403 + audit            │              │   "/openai/v1/.." → "openai"  │
  │    (ConnectBypassesL7)    │              │ RouteStore.get(prefix) exact  │
  │    [#1132/#1077 surface]  │              │   HashMap lookup [#1132]      │
  │ 2. external_proxy set &   │              │ endpoint_rules.is_allowed?    │
  │    not bypassed? →        │              │   NO → 403 Forbidden + audit  │
  │    external::handle_      │              │   (EndpointPolicy) [#1077]    │
  │    external_proxy [#1048] │              │ credential? validate token    │
  │ 3. bypass? strict auth +  │              │   → 401 / 407 on fail         │
  │    connect::handle_connect│              │ aws_route? → 501 [D-08 stub]  │
  │ 4. else connect::         │              │ else forward + inject cred    │
  │    handle_connect         │              └──────────────────────────────┘
  │    (LENIENT auth: missing │
  │     Proxy-Auth = debug    │
  │     log + CONTINUE, keeps  │
  │     connection open [#1151]│
  └──────────────────────────┘

  Activation decision (BEFORE any of the above) lives CLI-side:
  proxy_runtime.rs :: prepare_proxy_launch_options → `active` bool [#1197/D-07]
```

### Pattern 1: Test-driven equivalence proof (THIS phase's defining method, D-01)
**What:** For each Cluster F intent, write (a) a behavioral-equivalence note and (b) a fork test
asserting the fork already delivers the intent. Port intent into the fork only if the test fails.
**When to use:** Every Cluster F commit except #1197 (which is a known real gap).
**Example shape:** see Code Examples below (D-02 keep-open test, D-09 403+audit test).

### Pattern 2: Ledger-addendum recording (D-11 — mirror Phase 87 CR-02 / Phase 88 CR-01)
**What:** All equivalence and won't-sync findings land as a Phase-85 ledger addendum so future
syncs expect the Cluster F divergences and never blind-cherry-pick. Any fix that *does* land on
fork-divergent lines is recorded as a deliberate fork-divergence with its regression test.
**Where:** `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md`, appended **after**
the Phase 88 CR-01 Addendum (currently ending ~line 856). Use the existing `| Field | Value |`
table + `**Future sync note:**` format verbatim (see Code Examples).

### Anti-Patterns to Avoid
- **Importing upstream abstractions:** Do NOT add `RouteSelection`, `select_route`,
  `TlsInterceptIntent`, `DomainFilterIntent`, or the intent/activation separation. They don't map to
  the fork's `EffectiveProxySettings` / exact-prefix `RouteStore` model and add divergence cost with
  zero behavioral payoff (LOCKED: D-04, specifics §). No-dead-code standard forbids a stub
  `TlsInterceptIntent`.
- **Blind `git cherry-pick -x`:** The Cluster F disposition is `split` — the tls_intercept/ hunks
  won't apply and the shared-surface hunks must be reconciled by behavioral test, not applied raw.
- **Restructuring working route code** to match an abstraction the fork doesn't use (D-10).
- **Upgrading the AWS 501 to a diagnostic** — considered and explicitly NOT taken this phase (D-08;
  deferred §).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 403/audit on denied request | A new denial path | Existing `audit::log_denied` + `NetworkAuditDenialCategory::{EndpointPolicy, ConnectBypassesL7, AuthenticationFailed}` | Already wired in `reverse.rs` and `server.rs`; reuse |
| Proxy test harness | A network socket | `tokio::io::duplex(1024)` + `read_to_string` helper | Established in `connect.rs` tests (lines 277-417) |
| Route shadowing logic | upstream `RouteSelection` enum | Existing exact-prefix `RouteStore` HashMap | Fork model already avoids the shadow class |
| customCredentials plumbing | New config path | Existing `prepared.custom_credentials` → `resolve_credentials(.., custom_credentials)` | Already threaded; only the activation gate misses it |

**Key insight:** The fork already does almost everything Cluster F adds upstream — but
*differently* (CONNECT-block instead of tls_intercept, exact-prefix instead of host-selection,
lenient-auth-continue instead of explicit keep-open handshake). The phase's value is *proving* that
with tests and *recording* it in the ledger, not writing new code.

## Per-Item Findings (the code-grounded core)

### D-07 / #1197 — customCredentials activation gate (THE real fix) — PORT
**File:** `crates/nono-cli/src/proxy_runtime.rs:90-116` (`prepare_proxy_launch_options`).

The activation predicate appears twice. Current code:

```rust
let active = if matches!(prepared.caps.network_mode(), nono::NetworkMode::Blocked) {
    if !credentials.is_empty()
        || network_profile.is_some()
        || !allow_domain.is_empty()
        || upstream_proxy.is_some()          // <-- WARN branch (lines 91-95): missing custom_credentials
    { warn!("--block-net is active; ignoring proxy configuration ..."); ... }
    false
} else {
    matches!(prepared.caps.network_mode(), nono::NetworkMode::ProxyOnly { .. })
        || !credentials.is_empty()
        || network_profile.is_some()
        || !allow_domain.is_empty()
        || upstream_proxy.is_some()          // <-- ACTIVE branch (lines 109-116): missing custom_credentials
};
```

**Exact predicate to add (both branches):** `|| !prepared.custom_credentials.is_empty()`.
Note `prepared.custom_credentials` is in scope (the function takes `prepared: &PreparedSandbox`);
it is already cloned into `ProxyLaunchOptions { custom_credentials: prepared.custom_credentials.clone(), .. }`
at line 123. The ONLY missing link is the `active` predicate. [VERIFIED: fork source read]

**`--block-net` override branch (Claude's-discretion item):** Recommendation — **YES, add it to the
warn branch too**, for consistency with the warn-and-ignore semantics. The warn branch's job is to
emit the "ignoring proxy configuration that would re-enable network" warning whenever the user
supplied proxy-enabling config under `--block-net`. A customCredentials-only config is exactly such
a case; omitting it would silently swallow the user's intent without the courtesy warning. The
branch still returns `active = false` regardless (block-net wins), so adding the predicate only
affects *whether the warning fires*, never security posture. [Recommendation grounded in the
warn-and-ignore semantics; planner confirms.]

**Test (regression, the one new assertion that proves a real fix):** construct a `PreparedSandbox`
with a non-empty `custom_credentials` HashMap and otherwise-empty proxy config, `network_mode` NOT
Blocked, call `prepare_proxy_launch_options`, assert the returned `ProxyLaunchOptions.active == true`.
A second test asserting `active == false` under `--block-net` (network_mode Blocked) documents the
override. The existing test `resolve_effective_proxy_settings_preserves_with_endpoints`
(`proxy_runtime.rs:385-443`) is the template for constructing a full `PreparedSandbox` literal.

> **Cross-target subtlety (planner MUST heed):** the `PreparedSandbox` struct literal in any new
> test must include the two `#[cfg(target_os = "linux")]` fields `wsl2_proxy_policy` and
> `af_unix_mediation` (see existing test at `proxy_runtime.rs:417-420`). They drop out on Windows
> but are required to compile on Linux CI. Omitting them = green on Windows host, red on Linux CI —
> the exact cross-target blind-spot class from memory `feedback_clippy_cross_target`.

### D-10 / #1132 — allow_domain shadowing credential catch-all — VERIFY (expect: cannot reproduce)
**Files:** `crates/nono-proxy/src/route.rs` (RouteStore), `crates/nono-proxy/src/reverse.rs:82-91`
(`parse_service_prefix` + `RouteStore.get`), `crates/nono-cli/src/network_policy.rs:365-419`
(`partition_allow_domain`).

The fork **cannot** reproduce the upstream shadow scenario. Three structural facts:
1. `RouteStore` is `HashMap<String, LoadedRoute>` keyed by the **exact normalized prefix**
   (`route.rs:62-124`). Lookup is `get(&service)` — exact key, no longest-prefix/host-selection.
2. Reverse-proxy dispatch is by **path prefix**, not upstream host: `parse_service_prefix("/openai/v1/..")`
   → `"openai"` (`reverse.rs:396-404`), then exact `RouteStore.get("openai")`.
3. allow_domain endpoint routes get a **disjoint key namespace**: `partition_allow_domain` assigns
   prefix `format!("_ep_{}", domain)` (`network_policy.rs:389`), while credential routes use the
   service name as prefix (`resolve_credentials`, `network_policy.rs:223`/`263`). An `_ep_api.openai.com`
   endpoint route and an `openai` credential route are different HashMap keys — they cannot collide.

Upstream's #1132 bug is intrinsic to its `select_route`/`RouteSelection<'a>` abstraction (added in
`b0b2c743`, ledger line 336), which selects routes by **upstream host** with ordering precedence —
so an allow_domain endpoint route matching the same host as a credential "catch-all" could win and
shadow it. The fork has no such host-keyed selection and no "catch-all" route concept; there is no
fallback/wildcard route in `RouteStore`. [VERIFIED: fork source read]

**Disproof test scenario (the assertion to write):** Load a `RouteStore` with (a) a credential
route `prefix="openai"`, `upstream="https://api.openai.com"`, and (b) an allow_domain endpoint route
`prefix="_ep_api.openai.com"`, `upstream="https://api.openai.com"` (same upstream host — the
shadow trigger upstream). Then assert:
- `store.get("openai")` returns the **credential** route (its endpoint_rules / credential intact).
- `store.get("_ep_api.openai.com")` returns the **endpoint** route.
- Neither lookup returns the other — same-upstream-host does not cause one to shadow the other,
  because dispatch is by prefix key, not host.
Optionally add a request-level test: a request `GET /openai/v1/models` resolves to the credential
route regardless of the presence of the `_ep_api.openai.com` route. If this passes (it will),
record equivalence + skip the `RouteSelection` refactor (D-10). If — against expectation — it
reproduces, fix the fork's route resolution **directly** (do not import `RouteSelection`).

### D-09 / #1077 — 403 + audit on denied non-CONNECT — VERIFY (expect: already present)
**File:** `crates/nono-proxy/src/reverse.rs:96-115`.

The fork **already** does exactly this. On endpoint-rule default-deny:
```rust
if !route.endpoint_rules.is_allowed(&method, &upstream_path) {
    warn!("{}", reason);
    audit::log_denied(ctx.audit_log, audit::ProxyMode::Reverse,
        &audit::EventContext {
            route_id: Some(&service),
            denial_category: Some(nono::undo::NetworkAuditDenialCategory::EndpointPolicy),
            ..Default::default()
        }, &service, 0, &reason);
    send_error(stream, 403, "Forbidden").await?;
    return Ok(());
}
```
This is checked **before any credential operation** (comment lines 93-95), matching #1077's intent.
[VERIFIED: fork source read]

**Test assertion:** drive `handle_reverse_proxy` (via the `tokio::io::duplex` harness, as in
`connect.rs` tests) with a route whose `endpoint_rules` deny the request's method+path; assert the
client receives `HTTP/1.1 403`, and `audit::drain_audit_events` yields one event with
`decision == Deny` and `denial_category == Some(EndpointPolicy)`. Record equivalence + skip
cherry-pick. (The `connect.rs` test module at lines 276-417 shows the exact `duplex`/`drain_audit_events`
pattern to copy.)

### D-01 / #1048-#1091 — respect external_proxy in CONNECT — VERIFY (expect: already present)
**File:** `crates/nono-proxy/src/server.rs:534-597`.

The fork **already** honors `external_proxy` in the CONNECT path. When `state.config.external_proxy`
is `Some` and the host is not bypassed (`bypass_matcher`), it routes via
`external::handle_external_proxy(..)` (line 561-571). The CLI maps `--upstream-proxy` →
`ProxyConfig.external_proxy` in `build_proxy_config_from_flags` (`proxy_runtime.rs:222-228`). The
fork uses `external_proxy`; upstream uses `upstream_proxy` — same intent, different field name.
[VERIFIED: fork source read]

**Test assertion:** unit-test that `build_proxy_config_from_flags` with
`ProxyLaunchOptions { upstream_proxy: Some("http://corp:3128".into()), .. }` produces a `ProxyConfig`
with `external_proxy == Some(ExternalProxyConfig { address: "http://corp:3128", .. })`. (Pattern
mirrors the existing `test_build_proxy_config_propagates_network_block_to_strict_filter` at
`proxy_runtime.rs:446-458`.) Record equivalence + skip.

### D-02 / #1151 — keep connection open for reactive proxy auth on CONNECT — VERIFY (expect: moot)
**File:** `crates/nono-proxy/src/connect.rs:43-48`.

The fork's CONNECT auth is **lenient by design** and already keeps the connection open:
```rust
// Non-fatal for CONNECT: Node.js undici doesn't send Proxy-Authorization
// from URL userinfo for CONNECT requests.
if let Err(e) = validate_proxy_auth(remaining_header, session_token) {
    debug!("CONNECT auth skipped: {}", e);   // logs and CONTINUES — does NOT drop/return
}
```
Missing/invalid `Proxy-Authorization` on a CONNECT does **not** drop the connection — it logs at
debug and proceeds to host-filter + tunnel. This is precisely #1151's "keep connection open for
reactive proxy auth" behavior, achieved via the fork's undici-compat lenient auth rather than
upstream's 407-retry handshake. [VERIFIED: fork source read]

> Caveat (planner note): the lenient path is the **direct** CONNECT (`connect::handle_connect`). The
> *bypass* arm of an external-proxy config (`server.rs:572-586`) deliberately enforces **strict**
> `token::validate_proxy_auth` before falling through to `handle_connect` (comment lines 574-577).
> The keep-open equivalence applies to the lenient direct path; the strict bypass path is a separate
> deliberate fork decision. The D-02 test should target the direct CONNECT path.

**Keep-open test scenario:** drive `handle_connect` via `tokio::io::duplex` with a CONNECT line and
**no** `Proxy-Authorization` header, to a filter-allowed host. Assert the handler does NOT return an
auth error / does NOT send 407 — i.e. it proceeds (the upstream-connect will fail in a unit test
with no real upstream, but the assertion is "did not reject on missing auth"; can assert the debug
"CONNECT auth skipped" path is taken, or that the failure is an `UpstreamConnect`/`HostDenied`, never
an auth rejection). If the fork ever *dropped* on missing auth, this test would catch it. Since it
keeps open, record equivalence + skip #1151. Same test-driven bar as D-01 (no special-casing).

### D-04 / #1199 (bd4b6b7f) — intent/activation refactor — WON'T-SYNC (ledger addendum only)
Organizational refactor across 5 CLI runtime files (`command_runtime.rs`, `execution_runtime.rs`,
`launch_runtime.rs`, `proxy_runtime.rs`, `supervised_runtime.rs`), adding `pub(crate)` intent structs
incl. `TlsInterceptIntent` (ledger lines 349-354). Conflicts with the fork's `EffectiveProxySettings`
model and carries a struct the fork cannot back (no `tls_intercept/`). No-dead-code standard forbids
a stub. **No fork code change.** Record as won't-sync (architecture divergence) in the ledger addendum.
[LOCKED: D-04; Phase 85 ledger]

### D-05 / #1192 (76b7b695) — forward_inner_request refactor — WON'T-SYNC (ledger addendum only)
Lives **entirely** inside `tls_intercept/handle.rs` (ledger lines 347-348), which is absent from the
fork. **No fork code change.** Record as won't-apply in the ledger addendum. [LOCKED: D-05]

### D-08 / AWS 501 stub — UNCHANGED this phase
#1197 is about *activation*, not *signing*. Keep the honest 501 at `reverse.rs:189` (`aws_route.is_some()
→ send_error(.., 501, "Not Implemented")`) and the registration-only AWS path at `credential.rs:219-225`
(registers the prefix so `get_aws()` returns true → 501). Real SigV4 is a dedicated future phase.
The 501 stays a **bare status line** this phase (diagnostic upgrade considered, not taken). [LOCKED: D-08]

## Runtime State Inventory

> This is a sync/reconciliation phase, not a rename/migration. No stored data, live-service config,
> OS-registered state, secrets, or build artifacts carry strings that change. Inventory N/A.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — proxy is stateless per-process; no datastore keys change | none |
| Live service config | None — no external service config touched | none |
| OS-registered state | None | none |
| Secrets/env vars | None — `custom_credentials` plumbing already exists; no key renames | none |
| Build artifacts | None — no package/manifest renames | none |

## Common Pitfalls

### Pitfall 1: Adding upstream's abstractions to "match" the cluster
**What goes wrong:** Importing `RouteSelection` / `TlsInterceptIntent` / `select_route` to make the
cherry-picks apply cleanly.
**Why it happens:** Blind cherry-pick reflex; the diffs reference these types.
**How to avoid:** Default-to-no-op (D-01). Prove the fork delivers the intent with a test; record in
ledger. These abstractions are LOCKED out (D-04, specifics §). No-dead-code standard blocks a stub.
**Warning signs:** Any new `enum RouteSelection`, `struct TlsInterceptIntent`, or `fn select_route`
in a diff.

### Pitfall 2: New `PreparedSandbox` test literal that compiles on Windows but not Linux
**What goes wrong:** The D-07 activation test constructs a `PreparedSandbox` and omits the
`#[cfg(target_os = "linux")]` fields (`wsl2_proxy_policy`, `af_unix_mediation`). Green on the Windows
dev-host, **red on Linux CI** (E0063 missing field).
**Why it happens:** Windows host doesn't compile the linux-gated fields, so their absence is invisible
locally. This is the exact class in memory `feedback_clippy_cross_target`.
**How to avoid:** Copy the field set from the existing test at `proxy_runtime.rs:404-433`, including
the two `#[cfg(target_os = "linux")]` lines (417-420). Verify per CLAUDE.md cross-target checklist;
mark PARTIAL→CI if the Linux/macOS cross-toolchain isn't installed on the dev host.
**Warning signs:** A `PreparedSandbox { .. }` literal in a new test without `#[cfg(target_os = "linux")]`.

### Pitfall 3: Testing the wrong CONNECT auth path for D-02
**What goes wrong:** Asserting keep-open against the external-proxy **bypass** arm
(`server.rs:572-586`), which deliberately enforces strict auth.
**How to avoid:** Target the **direct** `connect::handle_connect` lenient path for the D-02 keep-open
test. The bypass-arm strictness is a separate deliberate fork decision, not a regression.

### Pitfall 4: Recording the ledger addendum in the wrong place / wrong format
**What goes wrong:** Inventing a new format or putting findings in the phase dir instead of the
canonical ledger.
**How to avoid:** Append after the Phase 88 CR-01 Addendum in `85-DIVERGENCE-LEDGER.md` (~line 856),
reusing the exact `| Field | Value |` table + `**Future sync note:**` shape (see Code Examples).

## Code Examples

### D-07 activation predicate fix (the one code change)
```rust
// crates/nono-cli/src/proxy_runtime.rs  — ACTIVE branch (currently ~109-116)
matches!(prepared.caps.network_mode(), nono::NetworkMode::ProxyOnly { .. })
    || !credentials.is_empty()
    || network_profile.is_some()
    || !allow_domain.is_empty()
    || upstream_proxy.is_some()
    || !prepared.custom_credentials.is_empty()   // <-- #1197 / D-07 fix

// WARN branch (currently ~91-95) — add the same disjunct (recommended, Claude's-discretion):
if !credentials.is_empty()
    || network_profile.is_some()
    || !allow_domain.is_empty()
    || upstream_proxy.is_some()
    || !prepared.custom_credentials.is_empty()   // <-- mirror for warn-and-ignore consistency
{ warn!("--block-net is active; ignoring proxy configuration ..."); ... }
```

### Test harness pattern (reuse — from connect.rs tests, lines 276-337)
```rust
use tokio::io::{duplex, AsyncReadExt};
use nono::undo::{NetworkAuditDecision, NetworkAuditDenialCategory, NetworkAuditMode};

async fn read_to_string<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> String {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.unwrap();
    String::from_utf8(buf).unwrap()
}

#[tokio::test]
async fn denied_endpoint_returns_403_and_audit() {
    let (mut server, client) = duplex(1024);
    let log = audit::new_audit_log();
    // ... build ReverseProxyCtx with a route whose endpoint_rules deny GET /forbidden ...
    // drive handle_reverse_proxy, then:
    let response = read_to_string(client).await;
    assert!(response.starts_with("HTTP/1.1 403"));
    let events = audit::drain_audit_events(&log);
    assert_eq!(events[0].decision, NetworkAuditDecision::Deny);
    assert_eq!(events[0].denial_category, Some(NetworkAuditDenialCategory::EndpointPolicy));
}
```

### Ledger addendum format (D-11 — mirror exactly; append after line 856 of 85-DIVERGENCE-LEDGER.md)
```markdown
## Phase 89 Cluster F Reconciliation Addendum

**Added:** 2026-06-20 (Phase 89 execution — Cluster F proxy hardening reconciliation)

### Equivalence findings (no fork change — fork already delivers the intent)

| Commit (issue) | Intent | Fork delivers via | Equivalence test |
|----------------|--------|-------------------|------------------|
| a5d623fd (#1077) | 403 + audit on denied non-CONNECT | reverse.rs:96-115 endpoint default-deny | denied_endpoint_returns_403_and_audit |
| b5f8db5c (#1048/#1091) | respect upstream_proxy in CONNECT | server.rs:534-561 external_proxy honoring | <test name> |
| 7c9abd3b (#1151) | keep connection open for reactive auth | connect.rs:46-48 lenient undici-compat auth | <test name> |
| b0b2c743 (#1132) | allow_domain not shadow credential catch-all | exact-prefix RouteStore + _ep_ key namespace | <disproof test name> |

### Won't-sync findings (architecture divergence — no fork change, ever)

| Commit (issue) | Reason | Classification |
|----------------|--------|----------------|
| 76b7b695 (#1192) | forward_inner_request lives entirely in absent tls_intercept/ | won't-apply |
| bd4b6b7f (#1199) | intent/activation refactor + TlsInterceptIntent fork cannot back | won't-sync (arch divergence) |

### Deliberate fork-divergence landed (if any)

| Field | Value |
|-------|-------|
| File | crates/nono-cli/src/proxy_runtime.rs |
| Fork lines | activation predicate (~95 warn branch, ~116 active branch) |
| Upstream reference commit | 724bb207 (#1197) |
| Fork behavior after Phase 89 | activation predicate includes `!prepared.custom_credentials.is_empty()` |
| Reason | #1197: proxy did not start when only customCredentials was set |
| Classification | Behavioral fix (intent ported; upstream's intent/activation refactor NOT ported) |
| Commit | <sha> |

**Future sync note:** When upstream b0b2c743/bd4b6b7f/76b7b695 reappear, expect conflicts on
route.rs / the CLI runtime intent types / tls_intercept. The fork's exact-prefix RouteStore and
EffectiveProxySettings model are the deliberate divergence — preserve them; do not import
RouteSelection or TlsInterceptIntent. Regression tests <names> guard against reversion.
```

## State of the Art

| Old Approach (upstream) | Fork Approach | Impact |
|--------------------------|---------------|--------|
| `tls_intercept/` module terminating TLS | CONNECT-block + reverse-proxy L7 (raw TLS pipe blocked to route upstreams) | Upstream's tls_intercept hardening (#1192, parts of #1048/#1151) is structurally N/A |
| Host-keyed `RouteSelection`/`select_route` with ordering | Exact path-prefix `HashMap` `RouteStore` + `_ep_` key namespace | #1132 shadow class cannot arise |
| `upstream_proxy` field + 407-retry reactive-auth handshake | `external_proxy` field + lenient undici-compat continue-on-missing-auth | #1048/#1091/#1151 intents delivered differently |
| intent/activation separation (TlsInterceptIntent et al.) | `EffectiveProxySettings` + single `active` bool | bd4b6b7f won't-sync |

**Deprecated/outdated for this fork:** upstream's `tls_intercept/` surface, `RouteSelection`,
`select_route`, `TlsInterceptIntent`, `DomainFilterIntent` — none exist in the fork and none are to
be introduced (LOCKED).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Adding the `custom_credentials` disjunct to the `--block-net` warn branch is desirable for consistency | D-07 / Claude's Discretion | Low — affects only whether a warning fires; `active` stays `false` under block-net regardless. Planner confirms against warn-and-ignore semantics (D-07 discretion item). |
| A2 | The D-10 disproof test will pass (fork avoids the shadow) | D-10 | Low — if it unexpectedly reproduces, D-10's fallback is "fix fork route resolution directly" (already planned-for in CONTEXT). The test is the gate either way. |

*All other claims are [VERIFIED: fork source read] or [LOCKED: Phase 85 ledger / 89-CONTEXT].*

## Open Questions

1. **Exact wording/structure of each equivalence test** (Claude's Discretion per CONTEXT).
   - What we know: harness = `tokio::io::duplex` + `drain_audit_events` (connect.rs precedent);
     config-level tests use `build_proxy_config_from_flags` + `ProxyLaunchOptions::default()`.
   - Recommendation: planner authors per-item test names; sketches above are sufficient to write them.
2. **ProxyDiagnostic surface usage** — only if a fix adds a NEW denial path (CONTEXT discretion).
   - What we know: #1197/D-07 adds an *activation*, not a denial. D-09/D-01/D-02 are equivalence
     (no new denial). So no new ProxyDiagnostic wiring is mandated this phase.
   - Recommendation: skip ProxyDiagnostic unless an unexpected D-10 fix introduces a denial.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (Windows host) | build + native tests | ✓ (per project) | 1.82+ | — |
| `cargo test -p nono-proxy` / `-p nono-cli` | proxy + activation tests | ✓ runs on Windows host | — | — |
| Linux/macOS cross-toolchain (cross clippy/test) | CLAUDE.md cross-target verify of cfg-gated edits | ✗ (host has rustup std, no cross C compiler per memory) | — | PARTIAL→CI per cross-target checklist |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** cross-toolchain → PARTIAL→CI deferral for any edit touching
cfg-gated lines (the D-07 test's `#[cfg(target_os = "linux")]` `PreparedSandbox` fields are the only
cfg surface this phase touches; the predicate edit itself is not cfg-gated).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) + `tokio` (`#[tokio::test]`, `tokio::io::duplex`) |
| Config file | none (Cargo convention; tests inline in `#[cfg(test)] mod tests`) |
| Quick run command | `cargo test -p nono-cli proxy_runtime` and `cargo test -p nono-proxy` |
| Full suite command | `make test` (or `cargo test --workspace --all-targets`) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PROXY-02 (#1197/D-07) | proxy activates when only customCredentials set | unit | `cargo test -p nono-cli proxy_runtime::tests` | ✅ module exists (add test) |
| PROXY-02 (#1048/#1091/D-01) | external_proxy honored in CONNECT / mapped from upstream_proxy | unit | `cargo test -p nono-cli build_proxy_config` | ✅ module exists (add test) |
| PROXY-02 (#1151/D-02) | CONNECT keeps connection open on missing auth | unit | `cargo test -p nono-proxy connect` | ✅ connect.rs tests exist (add test) |
| PROXY-01 (#1077/D-09) | denied non-CONNECT → 403 + audit | unit | `cargo test -p nono-proxy reverse` | ✅ (add test) |
| PROXY-01 (#1132/D-10) | allow_domain endpoint route does not shadow credential route | unit | `cargo test -p nono-proxy route` | ✅ route.rs tests exist (add test) |

### Sampling Rate
- **Per task commit:** `cargo test -p nono-proxy` + `cargo test -p nono-cli proxy_runtime`
- **Per wave merge:** `make test` (full workspace)
- **Phase gate:** full suite green + cross-target clippy (PARTIAL→CI if cross-toolchain absent)
  before `/gsd:verify-work`

### Wave 0 Gaps
- None — every touched module already has a `#[cfg(test)] mod tests` with the needed harness
  (`tokio::io::duplex`, `drain_audit_events`, `build_proxy_config_from_flags`, `RouteStore::load`).
  No new test file or framework install required. The phase only adds test functions to existing
  modules.

## Security Domain

> `security_enforcement` not explicitly false → enabled. This is a security-critical crate.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Session-token validation (`token::validate_proxy_auth`); lenient on CONNECT (undici compat) is a deliberate, documented relaxation — do not tighten without test |
| V4 Access Control | yes | L7 endpoint default-deny (`endpoint_rules.is_allowed`) + CONNECT-block to route upstreams |
| V5 Input Validation | yes | CONNECT line parse, header size limit (`MAX_HEADER_SIZE`), CRLF sanitisation in `send_response` |
| V6 Cryptography | no (this phase) | TLS connectors untouched; AWS SigV4 stays a 501 stub |
| V7 Logging | yes | `audit::log_denied` / `log_allowed` on every decision — preserve audit coverage on any touched path |

### Known Threat Patterns for the proxy

| Pattern | STRIDE | Standard Mitigation (already in fork) |
|---------|--------|---------------------------------------|
| CONNECT tunnel bypasses L7 filtering | Elevation of Privilege | `is_route_upstream` → 403 + `ConnectBypassesL7` audit (server.rs:489-530) |
| DNS rebinding TOCTOU on CONNECT | Tampering | connect to pre-resolved IPs, not re-resolved host (connect.rs:66-69) |
| HTTP response splitting via reason phrase | Tampering | CRLF sanitisation in `send_response` (connect.rs:182-191) |
| Header OOM | Denial of Service | `MAX_HEADER_SIZE` → 431 (server.rs:466-471) |
| Bypassed-host auth downgrade | Spoofing | strict `validate_proxy_auth` on external-proxy bypass arm before lenient handler (server.rs:572-577) |

**Phase-specific security note:** The D-07 fix only *activates* the proxy — it adds protection, never
relaxes it. The D-02 lenient-auth equivalence is an existing deliberate relaxation (undici compat);
do not "fix" it into strictness, and keep the strict bypass-arm carve-out intact. Any new denial
path (only plausible under an unexpected D-10 fix) must call `audit::log_denied` (V7).

## Sources

### Primary (HIGH confidence)
- Fork source, read line-by-line this session:
  - `crates/nono-cli/src/proxy_runtime.rs` (activation gate 80-133, build_proxy_config 175-234,
    test harness 385-473)
  - `crates/nono-proxy/src/server.rs:445-615` (handle_connection, CONNECT dispatch, external_proxy)
  - `crates/nono-proxy/src/connect.rs` (full — lenient auth 43-48, test harness 223-418)
  - `crates/nono-proxy/src/reverse.rs:80-191` + `parse_service_prefix` 396-404 (403+audit, AWS 501)
  - `crates/nono-proxy/src/route.rs` (full — RouteStore exact-key model, tests)
  - `crates/nono-cli/src/network_policy.rs:182-419` (resolve_credentials, partition_allow_domain)
  - `crates/nono-cli/src/sandbox_prepare.rs` (custom_credentials plumbing into PreparedSandbox)
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` §Cluster F (306-374),
  Phase 87 CR-02 Addendum (807-828), Phase 88 CR-01 Addendum (832-856) — disposition + addendum format
- `.planning/phases/89-proxy-hardening-sync/89-CONTEXT.md` — LOCKED decisions D-01..D-11
- `CLAUDE.md` §Coding Standards — unwrap policy, cross-target clippy MUST/NEVER, DCO, security

### Secondary (MEDIUM confidence)
- Memory `feedback_clippy_cross_target` — cfg-gated cross-target blind-spot class (informs Pitfall 2)
- Memory `project_v31_opened` — Phase 87/88 addendum precedent context

### Tertiary (LOW confidence)
- None. No WebSearch used — this phase is entirely grounded in fork source + locked planning artifacts.

## Project Constraints (from CLAUDE.md)

- **Unwrap policy:** no `.unwrap()`/`.expect()` in non-test code (`clippy::unwrap_used` -D). Test
  modules use `#[allow(clippy::unwrap_used)]` (existing pattern in every touched test module).
- **Cross-target clippy MUST/NEVER:** the proxy is not cfg-gated Unix code (verified: zero
  `cfg(target_os)`/`cfg(unix)` in `nono-proxy/src`), but the D-07 test's `PreparedSandbox` literal
  touches two `#[cfg(target_os = "linux")]` fields → verify via cross-target clippy, PARTIAL→CI if
  cross-toolchain absent (per `.planning/templates/cross-target-verify-checklist.md`).
- **DCO sign-off** on every commit (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`).
- **Security non-negotiable / fail-secure:** any touched decision path must preserve `audit::log_denied`
  coverage and never relax a denial without a test.
- **No dead code:** forbids a stub `TlsInterceptIntent` (reinforces D-04 won't-sync).
- **GSD workflow:** edits flow through the phase execution path, not ad-hoc.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; all existing deps verified in tree
- Architecture: HIGH — fork model read line-by-line; CONNECT-block + exact-prefix RouteStore confirmed
- Pitfalls: HIGH — cross-target `PreparedSandbox` field gap and lenient-vs-strict CONNECT auth paths
  both confirmed in source
- Per-item dispositions: HIGH — D-07 gap confirmed at exact lines; D-09/D-01/D-02/D-10 equivalences
  confirmed; D-04/D-05 won't-sync locked in Phase 85 ledger

**Research date:** 2026-06-20
**Valid until:** 2026-07-20 (stable — fork source + locked planning artifacts; only invalidated by
intervening edits to the named proxy files)
