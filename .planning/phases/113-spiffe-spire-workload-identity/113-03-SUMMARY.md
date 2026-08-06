---
phase: 113-spiffe-spire-workload-identity
plan: 03
subsystem: proxy-network-security
tags: [spiffe, spire, workload-identity, oauth2, jwt-bearer, rfc7523, rust, nono-proxy]

# Dependency graph
requires:
  - phase: 113-spiffe-spire-workload-identity
    provides: "Plan 113-01's spiffe.rs (SpiffeJwtSource) + core-library SPIFFE audit vocabulary; Plan 113-02's config.rs schema types (RouteConfig.oauth2.client_assertion, ClientAssertionConfig::SpiffeJwt, OAuth2Config.extra_params) and proxy_runtime.rs socket isolation"
provides:
  - "crates/nono-proxy/src/oauth2.rs: SpiffeAssertionTokenCache, exchange_jwt_assertion(), shared connect_and_send()/validate_token_url_scheme() helpers (also used by the pre-existing exchange_token)"
  - "crates/nono-proxy/src/credential.rs: SpiffeAssertionRoute, CredentialStore.spiffe_assertion_routes, CredentialStore::get_spiffe_assertion(), async CredentialStore::load()"
affects: [113-04, 113-05, 113-06, 113-07, 113-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SPIFFE-scoped OAuth2 jwt-bearer slice built directly (spiffe_assertion_routes/get_spiffe_assertion) rather than the general client_credentials route-wiring layer upstream's diff assumes as pre-existing context (OD-1)"
    - "TCP/TLS connect-and-send logic extracted once (connect_and_send) and shared by both the pre-existing client_credentials exchange_token and the new exchange_jwt_assertion, instead of duplicating connection setup"
    - "TLS connector for the jwt-bearer exchange is built lazily inside CredentialStore::load (only when at least one client_assertion route is configured), reusing crate::route::build_base_root_store() rather than inventing a second root-store construction"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/oauth2.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/server.rs

key-decisions:
  - "OD-1 held exactly as scoped: grep -c 'oauth2_routes\\|struct OAuth2Route\\|fn get_oauth2' crates/nono-proxy/src/credential.rs returns 0 — the general (non-SPIFFE) client_credentials route-wiring layer from unabsorbed b1ecbc02 was not built. Doc comments describing the exclusion had to be worded to avoid literally spelling out the three banned identifiers, since the acceptance grep is a pure substring match over the whole file including comments."
  - "Disposition amendment: the plan's read_first text claims the per-route-skip idiom to mirror is 'load_oauth_keystore_ref-style' — no function of that name exists anywhere in the codebase (grep-verified, zero matches). The actual existing per-route-skip idiom in this file is the credential_key branch's own `warn!(...); continue;` pattern, which is what the new client_assertion branch mirrors instead."
  - "Disposition amendment: server.rs's one production CredentialStore::load call site was explicitly deferred to 'Plan 113-05' per the plan's own D-04 truth text, but server.rs is in the SAME crate as credential.rs — making CredentialStore::load async without also adding `.await` at that call site would fail `cargo build -p nono-sandbox-proxy --all-targets`, this plan's own stated acceptance criterion. Fixed with a one-line `.await` addition (Rule 3, blocking build issue) since the call site was already inside an async fn (`pub async fn start`); no other logic in server.rs touched. server.rs added to files_modified beyond the plan's stated scope for this reason."
  - "is_empty()/len()/loaded_prefixes() extended to include spiffe_assertion_routes alongside the existing credentials/aws_routes counts (Rule 1/2, minor) — these are pre-existing multi-kind aggregation accessors on CredentialStore (already spanning static-keystore + AWS); leaving the new SPIFFE-assertion kind out would silently under-report `len()`/`is_empty()` and omit spiffe_assertion_routes prefixes from `loaded_prefixes()`, a correctness gap fully within this plan's own file."
  - "Rejected the mock-HTTP-server-based test alternative in Task 1's acceptance criteria for exchange_jwt_assertion directly: the function calls jwt_source.fetch_token() BEFORE any URL work (matching upstream's exact ordering, verified via git show c831dade), and SpiffeJwtSource has no test-only constructor bypassing a live SPIRE Workload API connection. Tested validate_token_url_scheme() directly instead — the exact shared helper exchange_jwt_assertion delegates to for the described rejection behavior."

requirements-completed: []  # NET-02 is satisfied across the whole 8-plan phase, not this plan alone

# Metrics
duration: ~50min
completed: 2026-08-06
---

# Phase 113 Plan 03: OAuth2 jwt-bearer SPIFFE Assertion Flow Summary

**Built the narrow SPIFFE-scoped RFC 7523 jwt-bearer OAuth2 client_assertion flow (`SpiffeAssertionTokenCache` in `oauth2.rs`, `CredentialStore.spiffe_assertion_routes`/`get_spiffe_assertion()` in `credential.rs`), deliberately declining the general (non-SPIFFE) `client_credentials` route-wiring layer per OD-1 — `CredentialStore::load` is now async because building the cache requires an awaited SPIRE Workload API connect.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-08-06 (session start)
- **Completed:** 2026-08-06
- **Tasks:** 2 completed
- **Files modified:** 3 (2 in files_modified scope + 1 blocking-issue fix, see Deviations)

## Accomplishments

- `SpiffeAssertionTokenCache` (`new()`, `get_or_refresh()`) and `exchange_jwt_assertion()` landed in `oauth2.rs`, matching upstream `c831dade`'s shape exactly (verified via `git show c831dade -- crates/nono-proxy/src/oauth2.rs`) except: no `decode_chunked` (the fork's pre-existing `exchange_token` doesn't decode chunked responses either — not introduced here to stay consistent with existing behavior), and the TCP/TLS connect logic is factored into a single shared `connect_and_send()` helper used by BOTH `exchange_token` (pre-existing client_credentials) and the new `exchange_jwt_assertion`, rather than duplicating connection setup as upstream's own diff does with its own inline `send_and_read` closure.
- The HTTPS-or-loopback-HTTP scheme hardening (T-113-09) was applied identically to both exchange functions via one shared `validate_token_url_scheme()` helper — `exchange_token`'s own `"http" => false` arm is now the SAME hardened check, not a second copy.
- `get_or_refresh()` fails closed on `ProxyError::Credential` (SVID fetch failure — workload identity is gone, T-113-10) but falls back to a stale token on any other exchange failure (network/IdP), matching `TokenCache::get_or_refresh`'s existing graceful-degradation idiom for that failure class exactly.
- `CredentialStore.spiffe_assertion_routes`/`SpiffeAssertionRoute`/`get_spiffe_assertion()` landed in `credential.rs`; `CredentialStore::load` is now `pub async fn`. The new branch (`else if let Some(ref oauth2) = route.oauth2`) connects `SpiffeJwtSource`, builds the `SpiffeAssertionTokenCache`, and inserts on success — a connect failure OR an initial-exchange failure skips only that route (warn + continue), never the whole `load()` call, matching the file's existing `credential_key` branch's per-route-skip idiom.
- A lazily-built TLS connector (`build_default_tls_connector()`, reusing `crate::route::build_base_root_store()`) is constructed at most once per `load()` call, only when at least one `client_assertion` route is present — a proxy with zero SPIFFE-assertion routes pays zero extra construction cost.
- `is_empty()`/`len()`/`loaded_prefixes()` extended to include the new `spiffe_assertion_routes` map alongside the existing `credentials`/`aws_routes` counts.
- OD-1's scope boundary held and is grep-verifiable: `grep -c "oauth2_routes\|struct OAuth2Route\|fn get_oauth2" crates/nono-proxy/src/credential.rs` returns `0`. The general (non-SPIFFE) `client_credentials` route-wiring layer that traces to unabsorbed `b1ecbc02` was not built — doc comments describing the exclusion were deliberately worded to avoid spelling out the three banned identifiers as literal substrings, since the acceptance grep matches the whole file including comments.
- Cross-target-clippy inherited dead_code count (10 errors at Plan 113-02's close, all on unconsumed `spiffe.rs`/`auth.rs` symbols) dropped to 6 after this plan: `SpiffeJwtSource` and its private `check_nbf` helper are now real consumers via `credential.rs`, so they no longer appear in the dead_code set. The remaining 6 (`UpstreamAuthMaterial`, `spiffe_audit_context`, `ManagedUpstreamAuth`, `acquire`/`audit_mechanism`/`audit_injection_mode`, `extract_trust_domain`, `delegation_from_jwt`) are all `auth.rs`/`spiffe.rs` symbols this plan does not consume — they belong to the OTHER SPIFFE flow (`RouteConfig.spiffe` direct-JWT-injection, `route.rs`/`reverse.rs`), out of this plan's `oauth2.rs`/`credential.rs` scope, expected to resolve in later plans per 113-01's own watch-item note.

## Task Commits

1. **Task 1: oauth2.rs — SpiffeAssertionTokenCache + exchange_jwt_assertion** - `46d9fb4b` (feat)
2. **Task 2: credential.rs — SPIFFE-only assertion routes, CredentialStore::load -> async** - `77373182` (feat)

_No plan-metadata commit yet — this SUMMARY + its own commit is that step._

## Files Created/Modified

- `crates/nono-proxy/src/oauth2.rs` - `SpiffeAssertionTokenCache`, `exchange_jwt_assertion()`, shared `connect_and_send()`/`validate_token_url_scheme()` helpers (also refactored into by the pre-existing `exchange_token`), 4 new unit tests
- `crates/nono-proxy/src/credential.rs` - `SpiffeAssertionRoute`, `CredentialStore.spiffe_assertion_routes`, `get_spiffe_assertion()`, `build_default_tls_connector()`, `load()` converted to `pub async fn`, `is_empty()`/`len()`/`loaded_prefixes()` extended, 3 existing test call sites converted to `#[tokio::test] async fn`, 2 new tests
- `crates/nono-proxy/src/server.rs` - one-line `.await` fix on the production `CredentialStore::load` call site (Rule 3, blocking-issue fix — see Deviations; no other logic touched)

## Decisions Made

- OD-1 upheld literally: no `oauth2_routes`/`OAuth2Route`/`get_oauth2()` machinery was built. See key-decisions above for the exact wording adjustment needed to keep doc comments grep-clean.
- The plan's `load_oauth_keystore_ref`-style precedent reference does not exist in this codebase (grep-verified, zero matches anywhere) — the new client_assertion branch instead mirrors the file's actual existing per-route-skip idiom (the `credential_key` branch's `warn!(...); continue;` pattern).
- server.rs's one production `CredentialStore::load` call site was fixed in THIS plan rather than deferred to "Plan 113-05" as the plan's D-04 truth text states, because it is in the same crate and this plan's own acceptance criterion (`cargo build -p nono-sandbox-proxy --all-targets exits 0`) cannot pass otherwise. This is a minimal, mechanical `.await` addition — no other server.rs logic was touched, and the fn was already async.
- `is_empty()`/`len()`/`loaded_prefixes()` were extended to include the new route kind — these accessors already aggregate across `credentials` + `aws_routes`; omitting the new kind would be an internal-consistency gap within this same file, not a scope expansion into another plan's territory.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] server.rs's production `CredentialStore::load` call site required `.await` to compile**
- **Found during:** Task 2, immediately after converting `CredentialStore::load` to `pub async fn`
- **Issue:** `server.rs:566` (same crate as `credential.rs`) calls `CredentialStore::load(&config.routes)?` synchronously; making `load()` async without updating this call site fails `cargo build -p nono-sandbox-proxy --all-targets` — this plan's own stated acceptance criterion. The plan's D-04 truth text says this call site is "updated in Plan 113-05," but that is incompatible with same-crate compilation.
- **Fix:** Added `.await` at the existing call site (`CredentialStore::load(&config.routes).await?`). The enclosing function (`pub async fn start`) was already async — no other change needed.
- **Files modified:** `crates/nono-proxy/src/server.rs`
- **Verification:** `cargo build -p nono-sandbox-proxy --all-targets` exits 0; `cargo build --workspace --all-targets` exits 0.
- **Committed in:** `77373182` (Task 2 commit)

**2. [Rule 1 - Bug] `cargo fmt --all --check` failures after adding new code**
- **Found during:** After Task 1's `warn!` macro reformatting and Task 2's `.await.expect(...)` chains
- **Issue:** New multi-arg `warn!()` calls and `.await.expect(...)` chains exceeded this workspace's configured line-wrap width
- **Fix:** Ran `cargo fmt --all`, which reformatted both files in place; re-ran `cargo fmt --all --check` to confirm clean
- **Files modified:** `crates/nono-proxy/src/oauth2.rs`, `crates/nono-proxy/src/credential.rs`
- **Verification:** `cargo fmt --all --check` exits 0
- **Committed in:** `46d9fb4b` and `77373182` respectively

---

**Total deviations:** 2 auto-fixed (1 blocking/build-scope, 1 bug/fmt). Both mechanical, no scope creep beyond the single-line `.await` fix documented above.
**Impact on plan:** Minor. The server.rs fix was strictly necessary for this plan's own acceptance criteria to pass; nothing else in server.rs was touched or reviewed.

## Issues Encountered

None blocking. `oauth2.rs`/`credential.rs`/`server.rs` (the three files this plan touched) contain zero `#[cfg(target_os = ...)]` blocks (grep-verified: `grep -n "cfg(target_os" crates/nono-proxy/src/oauth2.rs crates/nono-proxy/src/credential.rs crates/nono-proxy/src/server.rs` returns nothing) — per CLAUDE.md's own trigger condition ("files containing `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks"), the mandatory cross-target clippy gate (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) does NOT apply to this plan's changes, and this plan's own `<verification>` block does not call for a clippy run either (only `cargo build`/`cargo test`/`cargo fmt --all --check`/grep), consistent with 113-01/113-02's precedent of deferring clippy to whichever plan runs the phase-level gate last.

A local (Windows-host) `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used` run was still done informationally: **6 errors**, all pre-existing `dead_code` on `auth.rs`/`spiffe.rs` symbols this plan does not consume (`UpstreamAuthMaterial`, `spiffe_audit_context`, `ManagedUpstreamAuth`, `acquire`/`audit_mechanism`/`audit_injection_mode`, `extract_trust_domain`, `delegation_from_jwt`) — down from the 10 inherited at Plan 113-02's close (`SpiffeJwtSource`/`check_nbf` are no longer in the dead_code set, now consumed via `credential.rs`). Zero new findings; zero `clippy::unwrap_used` violations.

## User Setup Required

None — no external service configuration required. The new `test_load_spiffe_assertion_route_unreachable_socket_skips_route_not_startup` test exercises the fail-closed unreachable-socket path without a live SPIRE agent (matching `spiffe.rs`'s own `test_jwt_source_fails_closed_on_missing_socket` precedent); it takes ~10s per run because `SpiffeJwtSource::connect`'s `initial_sync_timeout` is 10s.

## Verification Results

- `cargo build -p nono-sandbox-proxy --all-targets` — **GREEN**, zero errors (dead_code warnings only, not errors — plain `cargo build` has no `-D warnings`).
- `cargo build --workspace --all-targets` — **GREEN**, zero errors.
- `cargo test -p nono-sandbox-proxy --lib -- oauth2:: credential::` — **GREEN**, 22 passed, 0 failed.
- `cargo test -p nono-sandbox-proxy --lib` (full crate suite) — **GREEN**, 235 passed, 0 failed (229 baseline + 6 new: 4 in `oauth2::`, 2 in `credential::`).
- `cargo fmt --all --check` — **GREEN**.
- `grep -c "todo!" crates/nono-proxy/src/oauth2.rs` — returns `0`.
- `grep -rn "oauth2_routes\|struct OAuth2Route\|fn get_oauth2\b" crates/nono-proxy/src/credential.rs | wc -l` — returns `0` (OD-1 scope boundary).
- `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used` (Windows-host, informational, not the mandatory cross-target gate — see Issues Encountered for why) — **RED**, 6 pre-existing `dead_code` errors, zero new, down from the 10 inherited at Plan 113-02's close.

## Next Phase Readiness

- `CredentialStore::get_spiffe_assertion()` is the stable contract Plan 113-06 (`reverse.rs::handle_spiffe_assertion_credential`) dispatches against — landed and unit-tested.
- **Watch-item for whichever later plan runs the phase-level cross-target clippy gate:** the dead_code set is now 6 (`auth.rs`'s `UpstreamAuthMaterial`/`ManagedUpstreamAuth`/`extract_trust_domain` family + `spiffe.rs`'s `delegation_from_jwt`), down from 10. These belong to the OTHER SPIFFE flow (`RouteConfig.spiffe` direct-JWT-injection via `route.rs`/`reverse.rs`) and are expected to resolve once those plans wire real consumers.
- `RouteStore::load` (in `route.rs`) is still sync and does not yet consume `RouteConfig.spiffe` — untouched by this plan, confirmed by direct read; D-04's async conversion for `RouteStore::load` is a separate, not-yet-executed piece of work (per 113-CONTEXT's `RouteStore::load`/`route.rs` framing), not something this plan silently completed.
- `server.rs`'s divergence-note comment at line ~235 (`"...no async RouteStore::load"`) is now partially stale in the OTHER direction (`CredentialStore::load` IS now async) but was not rewritten here — out of this plan's `files_modified` scope; flagged here so whichever plan next touches that comment block treats it as a known follow-up, not a surprise.
- No blockers for Plan 113-04.

---
*Phase: 113-spiffe-spire-workload-identity*
*Completed: 2026-08-06*

## Self-Check: PASSED

Both claimed modified files (`oauth2.rs`, `credential.rs`) plus the additional `server.rs` fix
confirmed present on disk with the described changes. Both claimed task commit hashes
(`46d9fb4b`, `77373182`) confirmed present in `git log --oneline --all`.
