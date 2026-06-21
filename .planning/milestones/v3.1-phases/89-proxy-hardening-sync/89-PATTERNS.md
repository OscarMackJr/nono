# Phase 89: Proxy Hardening Sync - Pattern Map

**Mapped:** 2026-06-20
**Files analyzed:** 7 (1 real code modify, 5 test-additions to existing modules, 1 docs append)
**Analogs found:** 7 / 7 (all analogs are in-file or sibling — this is a same-crate reconciliation phase)

> **Phase shape note:** This is a test-driven RECONCILIATION phase, not a feature build. Every
> "analog" is an *existing test in the very module being touched* or an *existing fork code path
> that proves the equivalence*. The planner should copy the in-module test conventions verbatim
> (imports, `#[allow(clippy::unwrap_used)]`, `tokio::io::duplex` harness, full struct literals).
> Only ONE behavioral code change lands (D-07); everything else is a test + a ledger addendum.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/proxy_runtime.rs` (activation predicate, D-07) | config / launch-policy | request-response (launch-time decision) | same file, lines 90-116 (the predicate itself) + test at 385-443 | exact (in-file) |
| `crates/nono-cli/src/proxy_runtime.rs` (D-07 regression test) | test | transform (pure config→bool) | `resolve_effective_proxy_settings_preserves_with_endpoints` (385-443) | exact (in-file template) |
| `crates/nono-cli/src/proxy_runtime.rs` (D-01 external_proxy mapping test) | test | transform | `test_build_proxy_config_propagates_network_block_to_strict_filter` (446-458) | exact (in-file template) |
| `crates/nono-proxy/src/reverse.rs` (D-09 403+audit equivalence test) | test | request-response | `connect.rs` tests 310-337 (`duplex` + `drain_audit_events`) | role+flow match (sibling) |
| `crates/nono-proxy/src/connect.rs` (D-02 keep-open equivalence test) | test | request-response (CONNECT tunnel) | same file, `write_upstream_failure_*` tests 287-417 | exact (in-file) |
| `crates/nono-proxy/src/route.rs` (D-10 shadow-disproof test) | test | CRUD (HashMap lookup) | same file, `test_load_routes_without_credentials` / `test_is_route_upstream` (294-383) | exact (in-file) |
| `crates/nono-proxy/src/server.rs` (D-01 — likely NO test; equivalence at code path 534-559) | config (verify-present) | request-response (CONNECT) | code path itself; CLI-side mapping test covers it | n/a (verify-present, no edit expected) |
| `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` (D-11 addendum) | docs | event-driven (sync provenance) | Phase 87 CR-02 Addendum (807-828) + Phase 88 CR-01 Addendum (832-855) | exact (in-file template) |

## Pattern Assignments

### `crates/nono-cli/src/proxy_runtime.rs` — D-07 activation predicate fix (THE one code change)

**Analog:** the predicate itself, `proxy_runtime.rs:90-116` (both branches).

**Core pattern to modify — ACTIVE branch (lines 108-116):**
```rust
matches!(
    prepared.caps.network_mode(),
    nono::NetworkMode::ProxyOnly { .. }
) || !credentials.is_empty()
    || network_profile.is_some()
    || !allow_domain.is_empty()
    || upstream_proxy.is_some()
    // ADD: || !prepared.custom_credentials.is_empty()   // #1197 / D-07
```

**WARN branch (lines 91-95) — mirror per D-07 discretion (recommended YES):**
```rust
if !credentials.is_empty()
    || network_profile.is_some()
    || !allow_domain.is_empty()
    || upstream_proxy.is_some()
    // ADD: || !prepared.custom_credentials.is_empty()   // mirror for warn-and-ignore consistency
{
    warn!("--block-net is active; ignoring proxy configuration ...");
    ...
}
false
```

**Key facts (no new plumbing needed):**
- `prepared: &PreparedSandbox` is the fn param (line 61); `prepared.custom_credentials` is in scope.
- Field type: `custom_credentials: HashMap<String, profile::CustomCredentialDef>` (`sandbox_prepare.rs:74`) — `.is_empty()` is valid.
- It is ALREADY cloned into `ProxyLaunchOptions { custom_credentials: prepared.custom_credentials.clone(), .. }` at line 123 — only the `active` predicate misses it.
- The predicate edit itself is NOT cfg-gated (no `#[cfg(...)]`), so it compiles identically cross-target.

---

### `crates/nono-cli/src/proxy_runtime.rs` — D-07 regression test (the one new behavioral assertion)

**Analog:** `resolve_effective_proxy_settings_preserves_with_endpoints` (`proxy_runtime.rs:385-443`).
This is the canonical full-`PreparedSandbox`-literal template. **Copy its field set verbatim.**

**CRITICAL cross-target trap (Pitfall 2 / `feedback_clippy_cross_target`):** the literal MUST
include the two `#[cfg(target_os = "linux")]` fields, or it goes green on Windows host and RED on
Linux CI (E0063 missing field). Copy lines 417-420 exactly:
```rust
        let prepared = PreparedSandbox {
            caps: CapabilitySet::new(),
            secrets: Vec::new(),
            rollback_exclude_patterns: Vec::new(),
            rollback_exclude_globs: Vec::new(),
            network_profile: None,
            allow_domain: vec![/* empty for D-07 */],
            credentials: Vec::new(),
            custom_credentials: /* NON-EMPTY HashMap with one CustomCredentialDef for D-07 */,
            upstream_proxy: None,
            upstream_bypass: Vec::new(),
            listen_ports: Vec::new(),
            capability_elevation: false,
            #[cfg(target_os = "linux")]
            wsl2_proxy_policy: crate::profile::Wsl2ProxyPolicy::default(),
            #[cfg(target_os = "linux")]
            af_unix_mediation: crate::profile::LinuxAfUnixMediation::default(),
            allow_launch_services_active: false,
            open_url_origins: Vec::new(),
            open_url_allow_localhost: false,
            bypass_protection_paths: Vec::new(),
            ignored_denial_paths: Vec::new(),
            suppressed_system_service_operations: Vec::new(),
            allowed_env_vars: None,
            denied_env_vars: None,
            set_vars: None,
            network_block_requested: false,   // FALSE → network_mode NOT Blocked → active branch
            loaded_profile: None,
            session_hooks: crate::profile::SessionHooks::default(),
        };
```

**Assertion shape:**
```rust
let opts = prepare_proxy_launch_options(&SandboxArgs::default(), &prepared, true).unwrap();
assert!(opts.active, "proxy must activate when only custom_credentials is set (#1197/D-07)");
```
A second test with `network_block_requested: true` (Blocked mode) asserts `active == false`
(documents the `--block-net` override; the warn fires but `active` stays false).

> Note: `caps.network_mode()` defaults from a fresh `CapabilitySet::new()` — confirm it is NOT
> `Blocked` and NOT `ProxyOnly` so the test actually exercises the `custom_credentials` disjunct
> (not a different short-circuit). If `CapabilitySet::new()` defaults to `Blocked`, set network
> mode explicitly to a non-blocked, non-proxy-only mode.

---

### `crates/nono-cli/src/proxy_runtime.rs` — D-01 external_proxy mapping test (config-level, verify-present)

**Analog:** `test_build_proxy_config_propagates_network_block_to_strict_filter` (`proxy_runtime.rs:446-458`).

**Pattern (copy the `ProxyLaunchOptions { .., ..::default() }` + `build_proxy_config_from_flags` shape):**
```rust
#[test]
fn test_build_proxy_config_maps_upstream_proxy_to_external_proxy() {
    let proxy = ProxyLaunchOptions {
        active: true,
        upstream_proxy: Some("http://corp:3128".into()),
        ..ProxyLaunchOptions::default()
    };
    let config = build_proxy_config_from_flags(&proxy).expect("build_proxy_config_from_flags");
    let ext = config.external_proxy.expect("external_proxy must be set");
    assert_eq!(ext.address, "http://corp:3128");
}
```
**Code being verified:** `build_proxy_config_from_flags` lines 222-228 already maps
`proxy.upstream_proxy` → `ProxyConfig.external_proxy` (`ExternalProxyConfig { address, auth: None, bypass_hosts }`). Record equivalence + skip cherry-pick (D-01).

---

### `crates/nono-proxy/src/reverse.rs` — D-09 403+audit equivalence test

**Analog (test harness):** `connect.rs` tests 276-337 — the `tokio::io::duplex` + `read_to_string`
+ `audit::drain_audit_events` pattern. **Copy these imports and the helper into reverse.rs's test
module** (reverse.rs already has a `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests` at 945-947;
its existing tests are pure-fn unit tests, so the async harness imports are NEW to this module).

**Harness imports to add (from connect.rs:276-277):**
```rust
use nono::undo::{NetworkAuditDecision, NetworkAuditDenialCategory, NetworkAuditMode};
use tokio::io::{duplex, AsyncReadExt};

async fn read_to_string<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> String {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.unwrap();
    String::from_utf8(buf).unwrap()
}
```

**Code path being verified (`reverse.rs:96-116`):** endpoint default-deny already does
`audit::log_denied(.., EndpointPolicy, ..)` then `send_error(stream, 403, "Forbidden")` BEFORE any
credential op. Test must construct a `ReverseProxyCtx<'_>` (struct at `reverse.rs:43-56`: needs
`route_store`, `credential_store`, `session_token`, `filter`, `tls_connector`, `audit_log`) with a
route whose `endpoint_rules` deny the request method+path, drive `handle_reverse_proxy`, assert:
```rust
let response = read_to_string(client).await;
assert!(response.starts_with("HTTP/1.1 403"));
let events = audit::drain_audit_events(&log);
assert_eq!(events[0].decision, NetworkAuditDecision::Deny);
assert_eq!(events[0].denial_category, Some(NetworkAuditDenialCategory::EndpointPolicy));
```
Audit-event field names confirmed against `connect.rs:328-336` (`mode`, `decision`,
`denial_category`, `target`, `port`, `reason`).

> Harness caveat: `handle_reverse_proxy` takes `stream: &mut TcpStream` (real `tokio::net::TcpStream`),
> NOT a generic `AsyncRead+Write`, whereas the connect.rs `write_upstream_failure` tests drive a
> `duplex` half directly. The planner must confirm whether `handle_reverse_proxy` is duplex-drivable
> as-is; if it requires a real `TcpStream`, prefer a `tokio::net::TcpListener::bind("127.0.0.1:0")`
> loopback pair (the standard substitute) rather than refactoring the handler signature. The
> equivalence assertion (403 + EndpointPolicy audit) is unchanged either way.

---

### `crates/nono-proxy/src/connect.rs` — D-02 keep-open equivalence test

**Analog:** the existing `#[tokio::test]` block in the SAME module, `connect.rs:287-417`
(`write_upstream_failure_*` family). Module header at 223-225 already has
`#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { use super::*; }` and the `duplex` +
`read_to_string` helper at 277-285 — REUSE them, no new imports.

**Code being verified (`connect.rs:43-48`):** lenient undici-compat auth — missing/invalid
`Proxy-Authorization` on CONNECT logs at `debug!` and CONTINUES (no return, no 407, connection
kept open). This IS #1151's keep-open behavior.

**Test scenario:** drive `handle_connect` with a CONNECT line and NO `Proxy-Authorization` header
to a filter-allowed host. Assert the handler does NOT reject on missing auth — the failure (if any
in a unit context with no real upstream) must be `UpstreamConnect`/`HostDenied`, NEVER an auth
rejection / 407. Target the **direct** `connect::handle_connect` path, NOT the external-proxy
bypass arm (`server.rs:572-586`) which deliberately enforces strict auth first (Pitfall 3).

---

### `crates/nono-proxy/src/route.rs` — D-10 allow_domain shadow-disproof test

**Analog:** `test_load_routes_without_credentials` (`route.rs:294-335`) and `test_is_route_upstream`
(`route.rs:361-383`) — both show the full `RouteConfig { .. }` literal (15 fields) loaded via
`RouteStore::load(&routes).unwrap()`. **Copy the RouteConfig literal field set verbatim** (prefix,
upstream, credential_key, inject_mode: `Default::default()`, inject_header, credential_format,
path_pattern, path_replacement, query_param_name, env_var, endpoint_rules, tls_ca, oauth2, aws_auth).
Module header: `route.rs:280-284` (`use super::*; use crate::config::EndpointRule;`).

**Disproof assertion (the test to write):** load a `RouteStore` with TWO routes sharing the same
upstream host but DIFFERENT prefix keys —
```rust
// (a) credential route, key "openai"
RouteConfig { prefix: "openai".into(), upstream: "https://api.openai.com".into(), .. }
// (b) allow_domain endpoint route, key "_ep_api.openai.com"
RouteConfig { prefix: "_ep_api.openai.com".into(), upstream: "https://api.openai.com".into(),
              endpoint_rules: vec![EndpointRule { method: "GET".into(), path: "/v1/models".into() }], .. }
```
Then assert exact-key lookups are disjoint (no shadow):
```rust
assert!(store.get("openai").is_some());                 // credential route intact
assert!(store.get("_ep_api.openai.com").is_some());     // endpoint route intact
// same-upstream-host does NOT make one shadow the other — dispatch is by prefix key, not host.
```
**Structural facts confirmed in source:** `RouteStore` is `HashMap<String, LoadedRoute>` keyed by
`route.prefix.trim_matches('/')` (`route.rs:62-76`); `get(&prefix)` is an exact `HashMap.get`
(`route.rs:122-124`); there is no longest-prefix / host-ordering selection. Expect PASS → record
equivalence + skip the `RouteSelection` refactor. Do NOT import upstream's `RouteSelection`.

---

### `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` — D-11 addendum

**Analog:** Phase 87 CR-02 Addendum (lines 807-828) and Phase 88 CR-01 Addendum (lines 832-855).
**Append AFTER line 855** (current EOF — the Phase 88 addendum is the last section). Use a `---`
separator then the new `## Phase 89 Cluster F Reconciliation Addendum` section.

**Format (mirror exactly):**
- An `**Added:** 2026-06-20 (...)` line.
- Equivalence-findings table: `| Commit (issue) | Intent | Fork delivers via | Equivalence test |`
  with rows for a5d623fd/#1077 (D-09), b5f8db5c/#1048/#1091 (D-01), 7c9abd3b/#1151 (D-02),
  b0b2c743/#1132 (D-10) — fill the test-name column with the actual test fn names written above.
- Won't-sync table: `| Commit (issue) | Reason | Classification |` with 76b7b695/#1192 (won't-apply,
  D-05) and bd4b6b7f/#1199 (won't-sync arch divergence, D-04).
- Deliberate fork-divergence `| Field | Value |` table for the D-07 landed change — copy the exact
  field rows from the Phase 88 CR-01 Addendum (`| File |`, `| Fork lines |`,
  `| Upstream reference commit |` = 724bb207 (#1197), `| Fork behavior after Phase 89 |`,
  `| Reason |`, `| Classification |` = Behavioral fix, `| Commit |` = `<sha>`).
- A closing `**Future sync note:**` paragraph naming the regression tests as reversion guards
  (mirrors the Phase 87/88 closing-paragraph convention at lines 822-828 / 847-855).

The full target template is reproduced in `89-RESEARCH.md` "Code Examples" (lines 430-468) — use it.

## Shared Patterns

### Test module convention (all proxy/CLI test additions)
**Source:** every touched module — `connect.rs:223-225`, `reverse.rs:945-947`, `route.rs:280-282`,
`proxy_runtime.rs` test mod.
**Apply to:** all new tests.
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]   // tests may use .unwrap() — this attr is REQUIRED (clippy -D unwrap_used)
mod tests {
    use super::*;
    // ...
}
```
`.unwrap()`/`.expect()` are allowed ONLY inside test modules carrying this attr (CLAUDE.md unwrap policy).

### Async test harness (CONNECT/reverse handler tests)
**Source:** `connect.rs:276-285`.
**Apply to:** D-02 (connect.rs), D-09 (reverse.rs).
```rust
use tokio::io::{duplex, AsyncReadExt};
async fn read_to_string<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> String { /* read_to_end → String::from_utf8 */ }

#[tokio::test]
async fn name() {
    let (mut server, client) = duplex(1024);
    let log = audit::new_audit_log();
    // drive handler with `&mut server`, then read `client`, then `audit::drain_audit_events(&log)`
}
```

### Audit-denial reuse (any newly-touched denial path)
**Source:** `reverse.rs:96-116` (EndpointPolicy), `connect.rs` audit calls.
**Apply to:** only if an unexpected D-10 fix adds a denial (not expected).
Reuse `audit::log_denied(log, audit::ProxyMode::{Reverse|Connect}, &audit::EventContext { denial_category: Some(NetworkAuditDenialCategory::{EndpointPolicy|ConnectBypassesL7|AuthenticationFailed}), .. }, ..)`.
Do NOT build a new denial path; do NOT add ProxyDiagnostic unless a fix introduces a NEW denial (research §Open Q2: not mandated this phase).

### Cross-target `PreparedSandbox` literal (D-07 test ONLY)
**Source:** `proxy_runtime.rs:404-433`.
**Apply to:** any new test constructing a `PreparedSandbox`.
MUST include `#[cfg(target_os = "linux")] wsl2_proxy_policy` and `#[cfg(target_os = "linux")] af_unix_mediation` (lines 417-420). Verify per `.planning/templates/cross-target-verify-checklist.md`; mark PARTIAL→CI if cross-toolchain absent on the Windows dev-host (per `feedback_clippy_cross_target`).

### Ledger-addendum recording (won't-sync / fork-divergence)
**Source:** Phase 87 CR-02 Addendum (807-828), Phase 88 CR-01 Addendum (832-855).
**Apply to:** D-04, D-05, D-11 (all of them land here, not in the phase dir).
`| Field | Value |` table + closing `**Future sync note:**` paragraph naming the guard test.

## No Analog Found

None. Every file in scope is either modified in place with an in-file/sibling template, or appends
to an existing doc with a 2-precedent template. No file requires a RESEARCH.md fallback pattern.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | — |

## Anti-Patterns (LOCKED OUT — do not introduce; from CONTEXT D-04/D-10 + RESEARCH §Anti-Patterns)

- **No `RouteSelection` / `select_route`** — fork uses exact-prefix `RouteStore` HashMap (D-10).
- **No `TlsInterceptIntent` / intent-activation separation** — fork uses `EffectiveProxySettings` + single `active` bool; no-dead-code standard forbids a stub (D-04).
- **No blind `git cherry-pick -x`** — Cluster F disposition is `split`; reconcile by behavioral test.
- **No AWS 501→diagnostic upgrade** — considered and NOT taken this phase (D-08); 501 stays a bare status line at `reverse.rs:189` / `credential.rs:219`.
- **Do not target the external-proxy bypass arm for D-02** (`server.rs:572-586` is deliberately strict).

## Metadata

**Analog search scope:** `crates/nono-cli/src/proxy_runtime.rs`, `crates/nono-cli/src/sandbox_prepare.rs`, `crates/nono-proxy/src/{server,connect,reverse,route}.rs`, `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md`.
**Files scanned:** 7 source/doc files (read line-by-line for the cited ranges).
**Pattern extraction date:** 2026-06-20
**Ledger append point:** line 855 (EOF; after Phase 88 CR-01 Addendum).
