---
phase: 113-spiffe-spire-workload-identity
reviewed: 2026-08-06T00:00:00Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - crates/nono-proxy/src/spiffe.rs
  - crates/nono-proxy/src/auth.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/route.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/oauth2.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/audit.rs
  - crates/nono-proxy/src/lib.rs
  - crates/nono-proxy/Cargo.toml
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/proxy_command.rs
  - crates/nono-cli/src/audit_integrity.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono/src/undo/types.rs
  - crates/nono/src/audit.rs
  - crates/nono-proxy/tests/spiffe_integration.rs
  - crates/nono-cli/tests/spiffe_run.rs
  - .github/workflows/spire.yml
  - scripts/spire-test.sh
  - testdata/spire/server.conf
  - testdata/spire/agent.conf
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 113: Code Review Report

**Reviewed:** 2026-08-06T00:00:00Z
**Depth:** standard
**Files Reviewed:** 25
**Status:** issues_found

## Summary

Phase 113 adds SPIFFE/SPIRE JWT-SVID workload identity as an upstream credential
source for `nono-proxy` reverse-proxy routes, plus an RFC 7523 jwt-bearer OAuth2
exchange variant. I reviewed this adversarially assuming the same class of
fail-open defect that shipped in Phase 112 (a silently dropped deny-list, a
predicate-less trust policy) was present here too, and specifically hunted for
it in the D-03 fail-closed guard, socket isolation, and token handling.

**I did not find a Critical/fail-open defect.** Specifically, and contrary to
the base hypothesis:

- The D-03 guard (`server.rs`'s CONNECT-arm `is_route_upstream` check and the
  forward-HTTP `spiffe_declared_for_upstream` check) is genuinely unconditional,
  fires before `use_external` is computed, and is not gated on `require_auth`.
  I traced host:port normalization end-to-end (`route::extract_host_port`,
  the CONNECT-arm authority parser, `parse_non_connect_target`) including the
  IPv6-bracket case: `url::Url::host_str()` returns IPv6 hosts *with* brackets
  (confirmed by reading the vendored `url-2.5.8` source), so all three
  normalization sites produce the same bracketed `[::1]:443`-style string —
  there is no bracket-stripping mismatch that would let a SPIFFE-declared
  route's upstream slip past the guard.
- `enforce_spiffe_socket_isolation()` hard-errors (`NonoError::ConfigParse`,
  not a warning) on a conflicting `unix_socket` grant, adds an explicit
  Seatbelt deny rule on macOS, and relies on Landlock's structural
  allow-list-only design on Linux. The Windows branch is a documented,
  tracked (T-113-05) residual risk rather than a silent claim of protection —
  see WR-03 below for why it still needs review attention.
- Token/credential handling is clean: `SpiffeAuditContext`/`NetworkAuditEvent`
  carry only `workload_spiffe_id`/`trust_domain`/`svid_type`, never a raw
  token; every buffer that concatenates a fetched token into an HTTP request
  is `Zeroizing`; `Debug` impls on `SpiffeJwtSource`, `LoadedCredential`,
  `OAuth2Config`, `SpiffeAssertionTokenCache` all redact secrets.
  `check_nbf`/expiry both validate against the SVID's own claims with no
  permissive clock-skew widening.
- No cache-key confusion: each route gets its own freshly-constructed
  `SpiffeJwtSource`/`SpiffeAssertionTokenCache` instance in the `RouteStore`/
  `CredentialStore` load loops (both are per-prefix `HashMap` entries built
  inside the same `for route in routes` loop) — there is no shared
  cache/connection that one route's request could pull another route's token
  from.
- The profile JSON schema (`nono-profile.schema.json`) and the Rust
  `SpiffeAuthConfig`/`ClientAssertionConfig`/`OAuth2Config`/`RouteConfig.spiffe`
  shapes agree field-for-field, including `additionalProperties: false` /
  `#[serde(deny_unknown_fields)]` on both sides and matching `required` arrays.
- `RouteStore::load` / `CredentialStore::load` becoming `async` is scoped
  correctly: the only `.await` inside each per-route branch is the SPIRE
  connect/exchange itself, non-SPIFFE profiles never touch the Workload API,
  and both `server.rs::start()` call sites are themselves `async fn` (no
  `block_on` bridging was introduced to paper over the new `async`).

I did find three real, bounded-impact **Warnings** and three **Info** items,
detailed below — none of them create a fail-open bypass of the SPIFFE
enforcement itself.

## Warnings

### WR-01: Direct `route.spiffe` (JWT) routes are invisible to `CredentialStore`, breaking env-var injection and the startup banner

**File:** `crates/nono-proxy/src/credential.rs:193-356` (the `CredentialStore::load` loop), consumed by `crates/nono-proxy/src/server.rs:97-112` (`route_diagnostics`) and `crates/nono-proxy/src/server.rs:195-228` (`credential_env_vars`)

**Issue:** `CredentialStore::load`'s per-route dispatch is:

```rust
if let Some(ref key) = route.credential_key { ... credentials.insert(...); continue; }
else if route.aws_auth.is_some() { aws_routes.insert(...); }
else if let Some(ref oauth2) = route.oauth2 { /* only handles oauth2.client_assertion.SpiffeJwt */ }
```

There is no branch for `route.spiffe.is_some()` (the direct JWT-SVID bearer
path added by this phase, dispatched via `RouteStore`/`LoadedRoute.managed_auth`
instead). So a route configured with only `spiffe: Some(SpiffeAuthConfig::Jwt{..})`
never appears in `credentials`, `aws_routes`, or `spiffe_assertion_routes`, and
therefore never appears in `CredentialStore::loaded_prefixes()`.

`server.rs::start()` builds `ProxyHandle.loaded_routes` directly from
`credential_store.loaded_prefixes()` (`server.rs:584`), so for a direct-SPIFFE
route:

1. `credential_env_vars()` (`server.rs:213`, `if !self.loaded_routes.contains(prefix) { continue; }`)
   never emits the phantom API-key env var (`{PREFIX}_API_KEY` /
   `route.env_var`) — only the `{PREFIX}_BASE_URL` var is set. Any SDK that
   requires a non-empty API key at client-construction time (many official
   SDKs do) will fail before ever making a request, even though the route is
   fully functional end-to-end for a raw HTTP client like curl.
2. `route_diagnostics()` (`server.rs:101-107`) reports `"cred: none"` for a
   direct-SPIFFE route (since `loaded_routes` doesn't contain it and
   `route.credential_key.is_none()`), even though the route has a live,
   working managed credential. This misrepresents route status on the
   operator-facing startup banner (`proxy_runtime.rs`'s "Proxy routes:" log
   and `proxy_command.rs::print_connection_info`'s route table).

This is not a security bypass (no credential is skipped or weakened — the
route still fetches and injects a fresh SVID on every request regardless of
what env var is or isn't set), but it is a real, provable functional/UX
regression specific to this phase's new `route.spiffe` field, and it is
currently untested (no test constructs a `route.spiffe`-only route and
inspects `loaded_prefixes()`/`credential_env_vars()`/`route_diagnostics()`).

**Fix:** Add a branch to `CredentialStore::load` (or to `loaded_prefixes()`'s
caller in `server.rs`) that also considers `route.declares_spiffe` (exposed via
`RouteStore`) when computing `loaded_routes`, and update `route_diagnostics()`
to report a distinct status (e.g. `"cred: spiffe"`) instead of falling through
to `"cred: none"`. Simplest fix: pass `route_store` into
`route_diagnostics()`/the `loaded_routes` computation in `server.rs::start()`
and OR in `route_store.get(prefix).is_some_and(|r| r.has_spiffe_source())`.

### WR-02: `handle_spiffe_assertion_credential`'s successful-forward path has zero test coverage

**File:** `crates/nono-proxy/src/reverse.rs:831-1080`

**Issue:** Confirmed by search — no test anywhere in the tree (unit tests in
`credential.rs`, `spiffe_integration.rs`, or `spiffe_run.rs`) exercises
`handle_spiffe_assertion_credential`'s success path (auth gate passes, token
exchange succeeds, request forwarded, response streamed, audit logged). The
only tests touching the RFC 7523 assertion flow are `credential.rs`'s
load-time tests (`test_load_spiffe_assertion_route_unreachable_socket_skips_route_not_startup`,
`test_get_spiffe_assertion_none_when_unconfigured`) — both about
*construction*, not *forwarding*. `spiffe_run.rs` (the one live end-to-end
binary test file) only exercises the direct `route.spiffe` JWT path
(`spiffe_jwt_credential_injected_end_to_end`), not the `oauth2.client_assertion`
path. This means the entire request/response streaming, header-filtering, and
audit-emission logic unique to `handle_spiffe_assertion_credential` (~250
lines) has never actually been exercised end-to-end, even in CI with a live
SPIRE agent + IdP.

**Fix:** Add a live-gated integration test (mirroring
`spiffe_jwt_credential_injected_end_to_end`) that stands up a minimal mock
OAuth2 token endpoint plus a mock upstream, configures a route with
`oauth2.client_assertion.SpiffeJwt`, and asserts the upstream receives the
exchanged bearer token and the audit log records `SpiffeOAuthAssertion` +
`spiffe_context`.

### WR-03: Windows SPIRE-socket isolation ships unverified, not just undocumented

**File:** `crates/nono-cli/src/proxy_runtime.rs:502-568` (`enforce_spiffe_socket_isolation`)

**Issue:** The function's own doc comment is honest that the Windows branch
("logged as a residual-risk no-op... not independently verified this phase")
is weaker than the Linux/macOS branches, and this is tracked as T-113-05. I'm
restating it here because the practical consequence is understated by
"residual risk": on Windows, the entire security premise of this feature —
"the sandboxed child can never self-issue SVIDs by reaching the agent socket
directly" (the function's own doc comment) — currently rests on an assumption
about AppContainer's default-deny behavior for unlisted Unix-domain-socket
paths that has not been tested. If that assumption is wrong, a Windows
sandboxed child can reach the SPIRE Workload API socket directly and mint its
own SVIDs, defeating the entire point of routing credential acquisition
through the (unsandboxed) proxy. This is exactly the class of "assumed sound,
never verified" gap the review brief asked to hunt for specifically on this
`#[cfg(not(target_os = "macos"))]` branch.

**Fix:** Not a code fix for this phase (already correctly deferred/tracked as
T-113-05), but this should block any operator-facing claim that Windows SPIFFE
deployments are hardened until a verification pass confirms AppContainer
actually denies the socket path by default (or an explicit deny mechanism,
analogous to the macOS Seatbelt rule, is added).

## Info

### IN-01: `SpiffeJwtSource::fetch_token` uses unchecked arithmetic for the expiry check

**File:** `crates/nono-proxy/src/spiffe.rs:71-84`

**Issue:** `let remaining = exp_ts - now_ts;` uses plain `i64` subtraction on
two independently-derived timestamps. CLAUDE.md requires `checked_`/
`saturating_`/`overflowing_` arithmetic for "security-critical math," and a
JWT-SVID expiry check is exactly that class of computation. In practice this
cannot overflow with realistic Unix timestamps, so this is style/policy
compliance rather than an exploitable bug.

**Fix:** `let remaining = exp_ts.saturating_sub(now_ts);` — costs nothing and
brings the file into explicit compliance with the documented arithmetic rule.

### IN-02: `TokenCache::new`'s doc comment is now stale after this phase's async migration

**File:** `crates/nono-proxy/src/oauth2.rs:96-133`

**Issue:** The doc comment states "Called during `CredentialStore::load()`
which is synchronous. We bridge into async via
`tokio::runtime::Handle::current().block_on()`." `CredentialStore::load` was
made `async` by this phase (D-04). `TokenCache::new` (the plain, non-SPIFFE
`client_credentials` flow from an earlier, never-completed phase — confirmed
via `git log -S "oauth2_routes"` that this path was never actually wired into
this fork's `CredentialStore::load`, so this is pre-existing dead
infrastructure, not a regression introduced here) is not called from
production code at all, so the doc's factual claim about its caller's
async-ness is simply wrong now, in a way a future contributor re-wiring this
code could be misled by.

**Fix:** Update the doc comment to drop the now-false "which is synchronous"
claim, or note explicitly that `TokenCache`/`TokenCache::new` are currently
unused dead code pending the (declined-for-this-phase, OD-1) general
`client_credentials` route-wiring layer.

### IN-03: `nono-proxy` itself doesn't enforce `spiffe` vs `oauth2.client_assertion` mutual exclusion

**File:** `crates/nono-proxy/src/route.rs:130-222` / `crates/nono-proxy/src/credential.rs:185-356`

**Issue:** `nono-cli`'s profile validation (`profile/mod.rs:1150-1160`)
correctly rejects a custom credential that sets both `spiffe` and `auth`
(oauth2). But `nono-proxy`'s own `RouteConfig`/`RouteStore::load`/
`CredentialStore::load` place no such constraint on each other — both are
driven independently from the same `route.spiffe` / `route.oauth2` fields. A
caller that constructs a `RouteConfig` directly (bypassing `nono-cli`'s
profile layer — e.g. an external embedder of the `nono-proxy` crate) could set
both, causing `RouteStore::load` to open a live Workload API connection *and*
`CredentialStore::load` to perform an initial OAuth2 token exchange, only for
`reverse.rs::handle_reverse_proxy`'s dispatch (`route.has_spiffe_source()`
checked first) to silently prefer the direct-SPIFFE path and never use the
exchanged OAuth2 token. Not a security bypass (the direct-SPIFFE path is
itself fully authenticated), just wasted connections/exchanges and a
config-time invariant that only one of the two layers actually enforces.

**Fix:** Either enforce the mutual exclusion inside `RouteStore::load`/
`CredentialStore::load` (return `ProxyError::Config` if both are set) so the
invariant holds for every caller of the crate, or document explicitly on
`RouteConfig::spiffe`/`RouteConfig::oauth2` that `nono-cli`'s profile layer is
the sole enforcement point and library consumers must replicate the check.

---

## Resolution (2026-08-06, operator-directed fix pass)

**WR-01: RESOLVED.** `CredentialStore` (`crates/nono-proxy/src/credential.rs`)
now tracks a `declared_spiffe_routes: HashSet<String>` field, populated in
`CredentialStore::load`'s per-route loop by an unconditional
`if route.spiffe.is_some() { declared_spiffe_routes.insert(normalized_prefix.clone()); }`
check that runs independently of the existing `credential_key`/`aws_auth`/
`oauth2` dispatch chain — a sibling of `spiffe_assertion_routes`'s existing
role for the RFC 7523 jwt-bearer flow. This performs no live Workload API
connect or token exchange (that stays entirely in `RouteStore::load`,
`route.rs`'s D-04 branch); it only records that the route's config
*declares* the auth source, so only a prefix string is ever added — never a
token or SVID.

`CredentialStore::loaded_prefixes()`, `is_empty()`, and `len()` were extended
to include this set, and a new `declared_spiffe_prefixes()` accessor was
added for callers that need to distinguish it from static/AWS/OAuth2-assertion
credentials. `server.rs::start()` now derives `ProxyHandle.spiffe_routes` from
`credential_store.declared_spiffe_prefixes()` (in addition to the existing
`loaded_routes` from `loaded_prefixes()`), and `route_diagnostics()` reports
`"cred: spiffe"` for prefixes in that set instead of falling through to the
misleading `"cred: none"` or conflating it with the static-credential
`"cred: ok"`. `credential_env_vars()` needed no direct change — it already
gates the phantom API-key env var on `loaded_routes.contains(prefix)`, which
now includes direct-SPIFFE routes automatically.

Added test coverage (no live SPIRE agent required, since `CredentialStore
::load` never connects for this flow): `credential::tests::
test_load_route_spiffe_only_is_visible_in_loaded_prefixes`,
`credential::tests::test_load_route_without_spiffe_absent_from_declared_spiffe_prefixes`,
and `server::tests::test_route_spiffe_visible_in_env_vars_and_diagnostics`
(the last covers both `credential_env_vars()` and `route_diagnostics()`
against a `ProxyHandle` constructed directly, mirroring this file's existing
test pattern).

Verified: `cargo test -p nono-sandbox-proxy --lib` — 245 passed (242 baseline
+ 3 new), 0 failed. Native `cargo clippy --workspace --all-targets -- -D
warnings -D clippy::unwrap_used` — 0 errors. `cargo fmt --all --check` —
clean. Both mandatory cross-target clippy gates (`cross` linux-gnu,
`cargo-zigbuild` apple-darwin) re-run — see the fix commit / plan artifacts
for the recorded result.

**WR-02, WR-03, IN-01, IN-02, IN-03: left OPEN by explicit operator decision**
(2026-08-06). Not modified in this pass. These remain tracked as named
residuals of Phase 113 pending a future fix pass or operator disposition.

---

_Reviewed: 2026-08-06T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_WR-01 fix applied: 2026-08-06 (operator-directed; WR-02/WR-03/Info items left OPEN)_
