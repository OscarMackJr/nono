---
phase: 113-spiffe-spire-workload-identity
plan: 04
subsystem: proxy-network-security
tags: [spiffe, spire, workload-identity, jwt-svid, route-store, async, rust, nono-proxy, od-1, od-2, d-04]

# Dependency graph
requires:
  - phase: 113-spiffe-spire-workload-identity
    provides: "Plan 113-01's spiffe.rs (SpiffeJwtSource::connect) + auth.rs (ManagedUpstreamAuth::SpiffeJwt); Plan 113-02's config.rs RouteConfig.spiffe/SpiffeAuthConfig::Jwt; Plan 113-03's CredentialStore::load already async (a DIFFERENT function on a DIFFERENT call site) and its established per-route-skip vs whole-load-fail distinction"
provides:
  - "crates/nono-proxy/src/route.rs: LoadedRoute.managed_auth (Option<Arc<ManagedUpstreamAuth>>), LoadedRoute.declares_spiffe, LoadedRoute::has_spiffe_source(), RouteStore::spiffe_declared_for_upstream(host_port), async RouteStore::load() that connects SpiffeJwtSource inside the per-route spiffe branch and fails the WHOLE call closed on an unreachable Workload API socket"
  - "The corrected OD-2 attribution comment in route.rs's mod tests, naming b1ecbc02 as the real ancestor of the declined-this-phase general OAuth2 route-wiring machinery"
affects: [113-05, 113-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-04's eager-connect-at-load design: the SPIFFE connect await sits strictly inside the per-route `Some(SpiffeAuthConfig::Jwt {..})` match arm, so a profile with zero spiffe routes never touches the Workload API and RouteStore::load's async signature costs nothing for non-SPIFFE deployments"
    - "declares_spiffe kept as a field independent of managed_auth.is_some() specifically to make has_spiffe_source() unit-testable without a live SPIRE Workload API connection (SpiffeJwtSource has no test-only constructor) — same rationale 113-03 used for CredentialStore's own SPIFFE testability gap"
    - "OD-1 scope-boundary doc comments worded to avoid literally spelling out the banned identifier substrings (lookup_all_by_upstream, has_intercept_route, requires_managed_credential), since the acceptance grep matches the whole file including comments — mirrors 113-03's established wording workaround for the same constraint"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/server.rs

key-decisions:
  - "D-04 adopted exactly as scoped: RouteStore::load is now async, connecting SpiffeJwtSource only inside the per-route spiffe branch, and fails the WHOLE load() call closed (not a per-route skip) on an unreachable Workload API socket — the opposite fail-closed shape from credential.rs's oauth2.client_assertion per-route-skip (Plan 113-03), and intentionally so per this plan's own <interfaces> rationale (route.spiffe is upstream's PRIMARY, upstream-diffed design; the oauth2.client_assertion per-route-skip is this fork's OWN idiom with no upstream precedent to diverge from)."
  - "OD-1 held exactly as scoped: grep -c 'lookup_all_by_upstream\\|has_intercept_route\\|requires_managed_credential' crates/nono-proxy/src/route.rs returns 0 after Task 2's comment correction — no general-purpose machinery of that shape was added, only has_spiffe_source()/spiffe_declared_for_upstream()."
  - "OD-2 held exactly as scoped: the misattributed 'belongs to the upstream tls_intercept module' comment is rewritten to name b1ecbc02 (git describe v0.38.0-3-gb1ecbc02) as the real ancestor, confirmed via git log --all -S\"handle_oauth2_credential\" -- crates/nono-proxy/src/reverse.rs on the upstream remote."
  - "Disposition amendment (Rule 3 - blocking): reverse.rs's one test call site (denied_endpoint_returns_403_and_audit, already #[tokio::test] async fn) and server.rs's one production call site (inside pub async fn start, same function 113-03 already awaited CredentialStore::load in) both required a one-line .await addition to compile once RouteStore::load became async — the plan's own files_modified scope names only route.rs, but making load() async without updating these same-crate call sites would fail cargo build -p nono-sandbox-proxy --all-targets, this plan's own stated acceptance criterion. Matches 113-03's identical precedent for CredentialStore::load's server.rs call site."
  - "Disposition amendment (Rule 2 - documentation correctness, opportunistic): server.rs's own divergence-note comment ('...no TLS intercept, no SPIFFE, no async RouteStore::load') was already flagged as doubly-stale by 113-CONTEXT.md's D-04 text and by 113-03-SUMMARY.md's own watch-item. Since this plan was already forced to touch server.rs for the .await compile fix immediately adjacent to that comment's subject, it was rewritten in the same commit rather than left for a later plan to rediscover — matching 113-PATTERNS.md's suggested replacement wording (L4)."
  - "The plan's acceptance criterion 'spiffe_declared_for_upstream returns true for a loaded SPIFFE route's exact upstream host:port' could not be exercised directly: SpiffeJwtSource::connect requires a live (or at minimum reachable-socket) SPIRE Workload API and has no test-only constructor bypassing that connection — identical constraint 113-03 hit and documented for exchange_jwt_assertion. Covered instead: (a) the false-case for a route with no spiffe field (both for an unrelated host:port AND the route's own upstream host:port, proving the SPIFFE condition genuinely gates the result, not just the host:port match), and (b) the true-connect-path's fail-closed half via the new unreachable-socket whole-load test. The live-true-case is exercised by 113-CONTEXT's D-06 SPIRE-gated integration lane (SPIRE_AGENT_SOCKET), not a unit test in this file."

patterns-established: []

requirements-completed: []  # NET-02 is satisfied across the whole 8-plan phase, not this plan alone

# Metrics
duration: ~40min
completed: 2026-08-06
---

# Phase 113 Plan 04: RouteStore async load() + LoadedRoute SPIFFE surface + OD-2 comment fix Summary

**`RouteStore::load` is now async and connects a `SpiffeJwtSource` inside the per-route SPIFFE branch, failing the whole call closed on an unreachable Workload API socket (D-04); `LoadedRoute.managed_auth`/`has_spiffe_source()`/`RouteStore::spiffe_declared_for_upstream()` give `reverse.rs` (Plan 113-06) and `server.rs` (Plan 113-05) the queryable SPIFFE-status surface both need, without either re-deriving the logic. `route.rs`'s misattributed `tls_intercept` comment is corrected to name the real ancestor, `b1ecbc02` (OD-2). As a direct consequence of this plan's own dead_code-consuming wiring, the phase's inherited cross-target clippy dead_code count drops from 6 to 0 on both mandatory targets.**

## Performance

- **Duration:** ~40 min
- **Started:** 2026-08-06 (session start)
- **Completed:** 2026-08-06
- **Tasks:** 2 completed
- **Files modified:** 3 (1 in the plan's stated `files_modified` scope + 2 blocking-issue/documentation fixes, see Deviations)

## Accomplishments

- `LoadedRoute` gained `managed_auth: Option<Arc<crate::auth::ManagedUpstreamAuth>>` and `declares_spiffe: bool`, plus `#[must_use] fn has_spiffe_source(&self) -> bool`. `Debug` impl updated to include `declares_spiffe`.
- `RouteStore::load` converted from `pub fn(routes: &[RouteConfig]) -> Result<Self>` to `pub async fn`. The new per-route match arm connects `SpiffeJwtSource::connect(..).await` only when `route.spiffe` is `Some(SpiffeAuthConfig::Jwt {..})`, `?`-propagating on failure so the WHOLE `load()` call aborts — never a per-route skip, matching this plan's threat register (T-113-11) and the D-04/D-03 fail-closed contract described in the plan's own `<interfaces>` block.
- `RouteStore::spiffe_declared_for_upstream(host_port: &str) -> bool` added, mirroring `is_route_upstream`'s exact `host_port_matches` logic with an additional `has_spiffe_source()` requirement.
- All 7 measured D-04 blast-radius call sites converted and `.await`ed: 6 in `route.rs`'s `mod tests` (`test_load_routes_without_credentials`, `test_load_routes_normalises_prefix`, `test_is_route_upstream`, `test_route_upstream_hosts`, `test_load_routes_rejects_malformed_or_unsupported_upstreams`, `allow_domain_endpoint_route_does_not_shadow_credential_route` — all converted to `#[tokio::test] async fn`), 1 in `reverse.rs`'s `mod tests` (`denied_endpoint_returns_403_and_audit`, already `#[tokio::test] async fn`, only needed `.await` added), and the 1 production site in `server.rs` (`pub async fn start`, already async). Zero hits in `nono-cli`/`bindings/c`/`../nono-py`/`../nono-ts`, matching 113-RESEARCH.md's measured blast radius exactly.
- 3 new tests: `test_load_spiffe_route_unreachable_socket_fails_whole_load` (D-04/D-03 fail-closed proof, using the same `/tmp/nono-test-nonexistent-spire-agent.sock` convention as `spiffe.rs`'s own fail-closed test), `test_load_routes_without_spiffe_has_no_spiffe_source` (zero-behavior-change proof for non-SPIFFE routes), `test_spiffe_declared_for_upstream_false_for_non_spiffe_route` (false-case proof for both the route's own upstream and an unrelated one — see Deviations for why the true-case isn't unit-testable here).
- OD-2's misattributed comment corrected: rewritten to name `b1ecbc02` (`git describe` = `v0.38.0-3-gb1ecbc02`) as the real ancestor per `git log --all -S"handle_oauth2_credential" -- crates/nono-proxy/src/reverse.rs` on the `upstream` remote, explicitly notes this phase's OD-1 decline of that machinery, and corrects the stale "not present" claim for `NetworkAuditAuthMechanism`/`NetworkAuditInjectionMode` (now partially present since Plan 113-01).
- OD-1 held: `grep -c "lookup_all_by_upstream\|has_intercept_route\|requires_managed_credential" crates/nono-proxy/src/route.rs` returns `0` — no general-purpose machinery of that shape exists in the file (the pre-existing "Fork divergence" comment previously mentioned these 3 identifiers as plain-text names of what was NOT ported; Task 2's rewrite preserves that documentation intent using paraphrased wording that avoids the literal banned substrings, per the plan's own acceptance criterion applying to the whole file including comments).
- **Direct consequence for the phase's watched cross-target clippy dead_code count:** at Plan 113-03's close, 6 `dead_code` errors remained on `auth.rs`/`spiffe.rs` symbols (`UpstreamAuthMaterial`, `spiffe_audit_context`, `ManagedUpstreamAuth`, `acquire`/`audit_mechanism`/`audit_injection_mode`, `extract_trust_domain`, `delegation_from_jwt`) — all belonging to the direct-JWT-injection flow this plan wires up. After this plan, **both mandatory cross-target clippy gates are fully GREEN with 0 errors**: `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset). This is the phase's inherited dead_code watch-item fully resolved — no `#[allow(dead_code)]` was added anywhere (confirmed via grep across all touched files).

## Task Commits

1. **Task 1: LoadedRoute.managed_auth/declares_spiffe + async RouteStore::load + spiffe_declared_for_upstream** - `e8db2cc1` (feat)
2. **Task 2: OD-2 — correct the misattributed tls_intercept comment** - `4d7d3b18` (docs)

_No plan-metadata commit yet — this SUMMARY + its own commit is that step._

## Files Created/Modified

- `crates/nono-proxy/src/route.rs` - `LoadedRoute.managed_auth`/`declares_spiffe`/`has_spiffe_source()`, async `RouteStore::load()` with the SPIFFE-connect branch, `RouteStore::spiffe_declared_for_upstream()`, 6 test call sites converted to `#[tokio::test] async fn`, 3 new tests, the OD-2 comment correction (Task 2)
- `crates/nono-proxy/src/reverse.rs` - one-line `.await` fix on the pre-existing `#[tokio::test]`'s `RouteStore::load` call site (Rule 3, blocking-issue fix — see Deviations; no other logic touched)
- `crates/nono-proxy/src/server.rs` - one-line `.await` fix on the production `RouteStore::load(&config.routes)?` call site (Rule 3, blocking-issue fix), plus the divergence-note comment rewrite (Rule 2, documentation correctness — see Deviations)

## Decisions Made

- D-04/OD-1/OD-2 all held exactly as scoped in the plan's `must_haves`; see key-decisions above for the exact grep-verified proof for each.
- The plan's `files_modified` scope names only `route.rs`, but `reverse.rs` and `server.rs` both required a one-line `.await` fix to keep `cargo build -p nono-sandbox-proxy --all-targets` (this plan's own acceptance criterion) green once `RouteStore::load` became async — identical shape and identical justification to Plan 113-03's `server.rs` fix for `CredentialStore::load` (a different function, different call site, same crate-boundary reasoning).
- `server.rs`'s stale divergence-note comment (flagged as a known follow-up by both 113-CONTEXT.md's D-04 text and 113-03-SUMMARY.md's watch-item) was corrected opportunistically in the same commit as the `.await` fix, since this plan was already touching the exact line block for a mechanical reason.
- The `spiffe_declared_for_upstream` "true" acceptance criterion is not unit-testable without a live/reachable SPIRE Workload API (no test-only `SpiffeJwtSource` constructor exists) — covered instead via the false-case (both matching and non-matching host:port) plus the unreachable-socket whole-load-fail test, which together prove the SPIFFE condition is a genuine gate on the result rather than a host:port-only match. The live-true path is exercised by the phase's D-06 SPIRE-gated integration lane, not this file's unit tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `reverse.rs` and `server.rs`'s pre-existing `RouteStore::load` call sites required `.await` to compile**
- **Found during:** Task 1, immediately after converting `RouteStore::load` to `pub async fn`
- **Issue:** `server.rs:557` (`RouteStore::load(&config.routes)?`, inside `pub async fn start`) and `reverse.rs`'s `denied_endpoint_returns_403_and_audit` test (`RouteStore::load(&routes).unwrap()`, already `#[tokio::test] async fn`) both call the now-async fn synchronously; `cargo build -p nono-sandbox-proxy --all-targets` fails without the fix.
- **Fix:** Added `.await` at both call sites (`RouteStore::load(&config.routes).await?` and `RouteStore::load(&routes).await.unwrap()`). Both enclosing functions were already async — no other change needed.
- **Files modified:** `crates/nono-proxy/src/server.rs`, `crates/nono-proxy/src/reverse.rs`
- **Verification:** `cargo build -p nono-sandbox-proxy --all-targets` exits 0; `cargo build --workspace --all-targets` exits 0; `cargo test -p nono-sandbox-proxy --lib` — 238 passed, 0 failed.
- **Committed in:** `e8db2cc1` (Task 1 commit)

**2. [Rule 2 - Documentation correctness] `server.rs`'s stale "no SPIFFE, no async RouteStore::load" divergence-note comment**
- **Found during:** Task 1, while adding the `.await` fix to the same file
- **Issue:** The comment at `server.rs` (immediately above the no_proxy pipeline section) asserted the fork's `RouteStore` shape has "no TLS intercept, no SPIFFE, no async `RouteStore::load`" — two of those three clauses are now false as a direct result of this plan's own change, and 113-CONTEXT.md's D-04 text explicitly states this note "MUST be rewritten, not left stale."
- **Fix:** Rewrote to state the fork still has no TLS intercept (D-01/ADR-113) but now has SPIFFE-route support and async `RouteStore::load` (Phase 113, D-04), explicitly noting it overturns the note's prior "no SPIFFE, no async RouteStore::load" clauses — following 113-PATTERNS.md's suggested replacement wording.
- **Files modified:** `crates/nono-proxy/src/server.rs`
- **Verification:** Comment-only change; `cargo build -p nono-sandbox-proxy --all-targets` exits 0 (confirms no accidental code change alongside).
- **Committed in:** `e8db2cc1` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking/build-scope covering 2 call sites, 1 documentation-correctness/opportunistic). Both mechanical or directly consequential of this plan's own D-04 change; no scope creep beyond files already touched for a compile-correctness reason.
**Impact on plan:** Minor. `server.rs`'s comment fix went slightly beyond the plan's literal `files_modified: [route.rs]` scope but was strictly adjacent to a change already required for the plan's own acceptance criteria to pass, and directly resolves a staleness 113-CONTEXT.md flagged as this plan's own responsibility.

## Issues Encountered

None blocking. `route.rs`, `reverse.rs`, and `server.rs` (the three files this plan touched) contain zero `#[cfg(target_os = ...)]` blocks (confirmed via `grep -n "cfg(target_os" crates/nono-proxy/src/route.rs crates/nono-proxy/src/server.rs crates/nono-proxy/src/reverse.rs` — no matches), so CLAUDE.md's mandatory cross-target clippy trigger condition ("files containing `#[cfg(target_os = ...)]` blocks") does not strictly apply to this plan's changes, consistent with Plan 113-03's identical finding and this plan's own `<verification>` block (which calls only for `cargo build`/`cargo test`/`cargo fmt --all --check`).

Both cross-target clippy gates were still run informationally, given this plan directly resolves the phase's own watched dead_code item from 113-01/113-02/113-03:

- `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used` (Windows-host, informational) — **GREEN**, 0 errors (down from 6 informational errors at Plan 113-03's close).
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors (down from the 6 `dead_code` errors inherited from Plan 113-02/113-03's close).
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) — **GREEN**, 0 errors (identical result on the second mandatory target).

**This closes the phase's cross-target clippy dead_code watch-item** first flagged in 113-01-SUMMARY.md (10 errors), tracked down to 6 in 113-02/113-03, and now 0 after this plan wires `ManagedUpstreamAuth`/`SpiffeJwtSource` into `route.rs`'s `RouteStore::load` as a real, load-bearing consumer.

## User Setup Required

None — no external service configuration required. The new `test_load_spiffe_route_unreachable_socket_fails_whole_load` test exercises the fail-closed unreachable-socket path without a live SPIRE agent (matching `spiffe.rs`'s and `credential.rs`'s own precedent tests); it takes ~10s per run because `SpiffeJwtSource::connect`'s `initial_sync_timeout` is 10s.

## Verification Results

- `cargo build -p nono-sandbox-proxy --all-targets` — **GREEN**, zero errors.
- `cargo build --workspace --all-targets` — **GREEN**, zero errors.
- `cargo test -p nono-sandbox-proxy --lib -- route::` — **GREEN**, 28 passed, 0 failed (25 baseline `#[test]` fns + 3 new tests added by this plan = 28; the 6 converted-to-async sites are pre-existing tests, not new ones).
- `cargo test -p nono-sandbox-proxy --lib` (full crate suite) — **GREEN**, 238 passed, 0 failed (235 baseline at Plan 113-03's close + 3 new: `test_load_spiffe_route_unreachable_socket_fails_whole_load`, `test_load_routes_without_spiffe_has_no_spiffe_source`, `test_spiffe_declared_for_upstream_false_for_non_spiffe_route`). Exceeds the `>= 235` success criterion.
- `cargo fmt --all --check` — **GREEN**.
- `grep -c "belong to the upstream tls_intercept module" crates/nono-proxy/src/route.rs` — returns `0`.
- `grep -c "b1ecbc02" crates/nono-proxy/src/route.rs` — returns `4`.
- `grep -c "lookup_all_by_upstream\|has_intercept_route\|requires_managed_credential" crates/nono-proxy/src/route.rs` — returns `0` (OD-1 scope boundary, whole-file including comments).
- `grep -rn "allow(dead_code)" crates/nono-proxy/src/route.rs crates/nono-proxy/src/auth.rs crates/nono-proxy/src/spiffe.rs crates/nono-proxy/src/server.rs crates/nono-proxy/src/reverse.rs` — returns nothing (no dead_code masking added).
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors (see Issues Encountered — closes the phase's inherited dead_code watch-item).
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors (identical, second mandatory target).

## Next Phase Readiness

- `LoadedRoute.managed_auth`/`has_spiffe_source()` and `RouteStore::spiffe_declared_for_upstream()` are landed and stable for Plan 113-06 (`reverse.rs`'s credential-injection dispatch, calling `.acquire()` on `managed_auth`) and Plan 113-05 (`server.rs`'s D-03 request-time fail-closed guard on non-reverse-proxy paths) to build against.
- **No outstanding dead_code watch-item remains from Plans 113-01/113-02/113-03** — both mandatory cross-target clippy gates are fully GREEN at 0 errors as of this plan's close. Any NEW dead_code findings in a later plan's cross-target run are a genuine regression signal, not an inherited/expected state.
- `server.rs`'s divergence-note comment is no longer stale — Plan 113-05 (which also touches `server.rs` per its own scope) does not need to revisit this specific comment block.
- No blockers for Plan 113-05 or 113-06.

---
*Phase: 113-spiffe-spire-workload-identity*
*Completed: 2026-08-06*

## Self-Check: PASSED

All 3 claimed modified files (`route.rs`, `reverse.rs`, `server.rs`) confirmed present on disk
with the described changes. Both claimed task commit hashes (`e8db2cc1`, `4d7d3b18`) confirmed
present in `git log --oneline --all`.
