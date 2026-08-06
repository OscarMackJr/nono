---
phase: 113-spiffe-spire-workload-identity
plan: 05
subsystem: proxy-network-security
tags: [spiffe, spire, workload-identity, d-03, fail-closed, connect-tunnel, forward-http, audit, rust, nono-proxy]

# Dependency graph
requires:
  - phase: 113-spiffe-spire-workload-identity
    provides: "Plan 113-03's async CredentialStore::load (server.rs call site already awaited); Plan 113-04's async RouteStore::load (server.rs call site already awaited), LoadedRoute.has_spiffe_source()/declares_spiffe, RouteStore::spiffe_declared_for_upstream(), and the server.rs:231-238 divergence-comment rewrite (all landed ahead of this plan's own Task 1/Task 3 scope)"
provides:
  - "crates/nono-proxy/src/server.rs: ProxyState.bound_port field + forward-http self-request detection (observability-only); D-03's request-time fail-closed guard on handle_forward_http (new); D-03 audit-category refinement (SpiffeUnsupportedPath) on the pre-existing CONNECT route-upstream block, which was already a superset fail-closed guard for all 3 CONNECT dispatch arms before this plan"
  - "crates/nono-proxy/src/route.rs: RouteStore::from_loaded_routes, a #[cfg(test)] pub(crate) constructor enabling SPIFFE-declared-route test fixtures without a live SPIRE Workload API"
affects: [113-06, 113-07, 113-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-03's guard composes with (does not gate on) require_auth — verified via require_auth: false in both new end-to-end tests, proving the guard is unconditional per the locked decision text"
    - "Symbol-level verification before adding new deny surface: the CONNECT dispatch's pre-existing 'Block CONNECT tunnels to route upstreams' check (is_route_upstream) was proven to be a strict superset of spiffe_declared_for_upstream (same host_port_matches logic + declares_spiffe as an additional AND condition), so D-03's CONNECT-path requirement was already met — the guard was refined for audit precision, not duplicated three times across the CONNECT dispatch arms"
    - "#[cfg(test)] pub(crate) test-only constructors for otherwise module-private aggregate types (RouteStore::from_loaded_routes) — the established pattern for building SPIFFE-declared-but-not-live-connected fixtures, mirroring LoadedRoute.declares_spiffe's own field-level version of the same idea (Plan 113-04)"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/server.rs
    - crates/nono-proxy/src/route.rs

key-decisions:
  - "Task 1's `.await` sub-scope (RouteStore::load/CredentialStore::load call sites) was found ALREADY SATISFIED — both awaits, and the server.rs:231-238 divergence-comment rewrite (Task 3's entire scope), landed in Plans 113-03/113-04 as blocking-issue/documentation-correctness deviations. Verified via `grep -c \"no TLS intercept, no SPIFFE, no async RouteStore::load\" crates/nono-proxy/src/server.rs` returning 0 both before and after this plan's changes. No code change was made for Task 3; it is recorded here as a disposition amendment rather than re-applied."
  - "The plan's literal text said 'add a bound_port: u16 field to ProxyHandle' but handle_forward_http's self-request check needs `state.bound_port` where `state: &ProxyState` — ProxyHandle is not in scope inside a request handler. Cross-checked against upstream c831dade's actual diff: bound_port belongs on the server-side dispatch state struct (upstream's equivalent of this fork's ProxyState), not the caller-facing handle. Added to ProxyState, populated from `port` (already computed via `local_addr.port()` in start()) — this is a disposition amendment from the plan's literal wording, following the more authoritative upstream precedent."
  - "Task 1's self-request handling is observability-only (a debug! log), not a re-route to handle_reverse_proxy as upstream c831dade does. Upstream's shape requires ReverseProxyCtx.approval_backends/credential_capture_backend, which do not exist in this fork's ReverseProxyCtx. The plan's own read_first/action text explicitly authorized this: 'if none exists, add a minimal one that logs and treats it as a normal forward rather than inventing new denial semantics not requested by any locked decision.'"
  - "D-03's CONNECT-path requirement (bypass-route CONNECT arm, external-proxy-chain arm, non-bypass CONNECT arm) was found ALREADY SATISFIED by a pre-existing, unconditional check in handle_connection ('Block CONNECT tunnels to route upstreams', using RouteStore::is_route_upstream) that runs before use_external is computed and before any of the 3 CONNECT dispatch arms are selected. Verified by symbol: is_route_upstream and spiffe_declared_for_upstream both call the identical host_port_matches normalisation; spiffe_declared_for_upstream additionally requires has_spiffe_source(), making it a strict subset of is_route_upstream's match set. Also verified connect.rs::parse_connect_target derives host:port identically to the pre-check's own authority parsing (same first_line, same parts[1] split), so there is no parsing-divergence bypass. Given this, adding a second, independent spiffe_declared_for_upstream check inside each of the 3 CONNECT arms would be provably unreachable/dead code under current control flow. Instead, the existing check's audit denial_category was refined: SpiffeUnsupportedPath when the matched route specifically declares SPIFFE, ConnectBypassesL7 otherwise — preserving the existing (broader) protection while adding the audit precision D-03 asks for, without adding redundant deny branches. This is the plan's single largest disposition amendment and is called out with full reasoning in both new commit messages."
  - "handle_forward_http's D-03 guard is genuinely new code (this path had no route-upstream check of any kind before this plan) — the exact WR-13 defect shape (112-REVIEW.md: one path of six missing a check the others already had), but for the SPIFFE-unsupported-path condition rather than require_auth. Unconditional (not gated on state.config.require_auth), matching D-03's explicit 'even when --no-auth is set' requirement."
  - "RouteStore::from_loaded_routes (route.rs, #[cfg(test)] pub(crate)) was added beyond this plan's stated files_modified: [crates/nono-proxy/src/server.rs] scope. Required because RouteStore.routes is module-private to route.rs; without it, server.rs's D-03 tests (a different file) could not construct a RouteStore containing a declares_spiffe: true route without a live SPIRE Workload API for RouteStore::load to connect to. This is a Rule 3 (blocking-issue) deviation, mirroring the identical class of same-crate cross-file necessity documented in Plans 113-03/113-04's own deviations for the .await fixes."

patterns-established: []

requirements-completed: []  # NET-02 is satisfied across the whole 8-plan phase, not this plan alone

# Metrics
duration: ~65min
completed: 2026-08-06
---

# Phase 113 Plan 05: server.rs D-04 call-site wiring + D-03 fail-closed guard Summary

**Added `ProxyState.bound_port` + a request-time D-03 fail-closed guard denying (403, `denial_category: SpiffeUnsupportedPath`) any request whose target matches a SPIFFE-declared route's upstream on `handle_forward_http` (a genuinely new check — this path had none) and, via audit-category refinement rather than duplication, on all 3 CONNECT dispatch arms (already covered by a pre-existing unconditional `is_route_upstream` superset check, verified by symbol-level analysis of `route.rs::host_port_matches`/`spiffe_declared_for_upstream` and `connect.rs::parse_connect_target`); both of Task 1's `.await` call sites and Task 3's divergence-comment rewrite were found already landed by Plans 113-03/113-04.**

## Performance

- **Duration:** ~65 min
- **Started:** 2026-08-06 (session start)
- **Completed:** 2026-08-06
- **Tasks:** 3 (2 with code changes, 1 already-satisfied — see Deviations)
- **Files modified:** 2 (`server.rs` in the plan's stated scope; `route.rs` added for a same-crate test-construction necessity)

## Accomplishments

- **Task 1 (partial — see Deviations for the already-satisfied sub-scope):** Added `ProxyState.bound_port: u16`, populated from `port` (already computed via `local_addr.port()` in `start()`). `handle_forward_http` now detects an absolute-form request whose target is the proxy's own bound port on a loopback host and logs it (`debug!`) — observability-only, per the plan's own explicit authorization not to invent new re-routing/denial semantics this fork's `ReverseProxyCtx` has no shape for. New test `start_fails_closed_on_unreachable_spiffe_socket` proves `nono_proxy::server::start` itself (not just `RouteStore::load` in isolation, which Plan 113-04 already tested) returns `Err` within a bounded 30s timeout on an unreachable SPIRE Workload API socket, rather than hanging or panicking.
- **Task 2:** D-03's fail-closed guard is live on `handle_forward_http` (new, unconditional — not gated on `require_auth`) and, via audit-category refinement, on the CONNECT dispatch (already unconditionally denied for any route upstream, SPIFFE or not, by a pre-existing check — see Deviations for the full symbol-level proof). `grep -c "SpiffeUnsupportedPath" crates/nono-proxy/src/server.rs` returns `8` (>= 3 required by the plan's acceptance criteria). 2 new end-to-end tests (`d03_connect_denies_spiffe_declared_route_upstream`, `d03_forward_http_denies_spiffe_declared_route_upstream`) exercise the guard over real TCP against a live dispatch loop, both with `require_auth: false` to directly prove the guard's unconditional requirement, and the CONNECT test additionally sets `external_proxy` to prove the guard preempts all 3 CONNECT arms rather than only the plain non-bypass one.
- **Task 3:** `grep -c "no TLS intercept, no SPIFFE, no async RouteStore::load" crates/nono-proxy/src/server.rs` returns `0` — already true before this plan started (Plan 113-04 rewrote the comment as an opportunistic deviation while fixing the same file for its own `.await` compile requirement). No code change made; documented here per the plan's own instruction to record an already-satisfied task explicitly rather than redoing it.
- `RouteStore::from_loaded_routes` (`route.rs`, `#[cfg(test)] pub(crate)`) added to let `server.rs`'s D-03 tests build a `declares_spiffe: true` `RouteStore` fixture without a live SPIRE agent — `RouteStore.routes` is module-private, so no cross-file test construction was otherwise possible.
- Full suite: `cargo test -p nono-sandbox-proxy --lib` — **241 passed, 0 failed** (238 baseline at Plan 113-04's close + 3 new: `start_fails_closed_on_unreachable_spiffe_socket`, `d03_connect_denies_spiffe_declared_route_upstream`, `d03_forward_http_denies_spiffe_declared_route_upstream`). Exceeds the plan's `>= 238` success criterion.
- Both mandatory cross-target clippy gates run at this wave's boundary (D-12/113-VALIDATION.md sampling rate) — **GREEN, 0 errors** on both: `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset). No regression from Plan 113-04's close (0 errors on both).

## Task Commits

1. **Task 1: ProxyState.bound_port + forward-http self-request detection** - `6ef702bd` (feat)
2. **Task 2: D-03 fail-closed guard on non-SPIFFE-implementing proxy paths** - `fdd4692b` (feat)
3. **Task 3: rewrite the stale server.rs divergence comment** - already satisfied by Plan 113-04; no commit (see Deviations)

**Plan metadata:** this SUMMARY's own commit is that step.

## Files Created/Modified

- `crates/nono-proxy/src/server.rs` - `ProxyState.bound_port` field + construction; `handle_forward_http`'s self-request detection log; `handle_forward_http`'s new D-03 guard (403 + `SpiffeUnsupportedPath`); the pre-existing CONNECT route-upstream block's audit-category refinement (`SpiffeUnsupportedPath` vs `ConnectBypassesL7`); 3 new tests + 2 test-only helper functions (`spawn_state_for_d03_dispatch_test`, `spiffe_declared_route_store`)
- `crates/nono-proxy/src/route.rs` - `RouteStore::from_loaded_routes`, a `#[cfg(test)] pub(crate)` constructor (Rule 3, blocking-issue — see Deviations)

## Decisions Made

- Task 1's `.await` sub-scope and Task 3's comment rewrite were already landed by Plans 113-03/113-04; recorded as disposition amendments rather than re-applied. See key-decisions above for the exact grep-verified proof for each.
- `bound_port` was added to `ProxyState` (not `ProxyHandle` as the plan's literal text said) — cross-checked against upstream `c831dade`'s actual diff, which places `bound_port` on the server-side dispatch state struct. `ProxyHandle` is not accessible from `handle_forward_http`, so the plan's literal instruction could not type-check as written.
- The CONNECT-path portion of D-03 was found already satisfied by a pre-existing, unconditional, symbol-verified superset check. Rather than add 3 duplicate (and provably unreachable, per current control flow) `spiffe_declared_for_upstream` checks inside each CONNECT arm — which would themselves be a form of dead code — the existing check's audit category was refined for precision. This is the plan's most consequential deviation from its literal `<action>` text (which asked for a check "at each of the 3 non-reverse-proxy dispatch points... the bypass-route CONNECT arm, the non-bypass CONNECT arm"); the full symbol-level proof is captured in the Task 2 commit message and in key-decisions above.
- `RouteStore::from_loaded_routes` was added to `route.rs` (outside the plan's stated `files_modified: [server.rs]` scope) because it was strictly required for this plan's own Task 2 acceptance criteria (a test exercising the guard against a `declares_spiffe: true` route with no live SPIRE agent) to be satisfiable at all.

## Deviations from Plan

### Auto-fixed Issues

**1. [Disposition amendment — already satisfied] Task 1's `.await` call sites**
- **Found during:** Task 1, before making any edit (per this plan's own critical_execution_constraints instruction to re-read current file state first)
- **Issue:** The plan's `<interfaces>` block shows both `RouteStore::load(&config.routes)?` and `CredentialStore::load(&config.routes)?` as needing `.await` added. Direct read of `server.rs` (lines 560, 569 at the time) showed both already `.await`ed.
- **Fix:** No code change. Verified via `grep -n "RouteStore::load\|CredentialStore::load" crates/nono-proxy/src/server.rs` showing `.await?` at both call sites, confirmed by 113-03-SUMMARY.md and 113-04-SUMMARY.md's own documented deviations (both landed as Rule 3 blocking-issue fixes in their respective plans, since `server.rs` shares a crate with `credential.rs`/`route.rs`).
- **Files modified:** none (verification only)
- **Verification:** `cargo build -p nono-sandbox-proxy --all-targets` exits 0 before any Task 1 edit was made.
- **Committed in:** n/a — no commit needed for an already-true state.

**2. [Disposition amendment — already satisfied] Task 3's divergence-comment rewrite**
- **Found during:** Task 1 read_first, before Task 3 was reached
- **Issue:** The plan's Task 3 asks to rewrite `server.rs`'s stale "no TLS intercept, no SPIFFE, no async RouteStore::load" comment. Direct read showed the comment already rewritten (to "...adapted to this fork's `ProxyHandle`/`RouteStore` shape (no TLS intercept — see ADR-113/D-01 — but now with SPIFFE-route support and async `RouteStore::load`, absorbed in Phase 113 per 113-CONTEXT.md D-04, which overturns the...) clauses this comment previously asserted).").
- **Fix:** No code change. Verified via `grep -c "no TLS intercept, no SPIFFE, no async RouteStore::load" crates/nono-proxy/src/server.rs` returning `0`, confirmed by 113-04-SUMMARY.md's own documented Rule 2 deviation (opportunistic documentation-correctness fix, landed while that plan was already touching the same line block for its own `.await` compile fix).
- **Files modified:** none (verification only)
- **Verification:** grep above; `cargo build -p nono-sandbox-proxy --all-targets` exits 0.
- **Committed in:** n/a — no commit needed for an already-true state.

**3. [Rule 4-adjacent — plan-literal-vs-actual-type-system correction] `bound_port` field placement**
- **Found during:** Task 1, while reading the plan's `<action>` text against `handle_forward_http`'s actual signature (`state: &ProxyState`, not `&ProxyHandle`)
- **Issue:** The plan's `<action>` literally says "Add a `bound_port: u16` field to `ProxyHandle`... In `handle_forward_http`, add a check: if the parsed target's port equals `state.bound_port`..." — `ProxyHandle` and `ProxyState` are different structs; `handle_forward_http` only has access to `ProxyState`, so a field on `ProxyHandle` could never satisfy `state.bound_port`. Cross-checked against upstream `c831dade`'s actual diff (`git show c831dade -- crates/nono-proxy/src/server.rs`), which confirms `bound_port` lives on the server-side dispatch state struct (upstream's structural equivalent of this fork's `ProxyState`), not the caller-facing handle.
- **Fix:** Added `bound_port: u16` to `ProxyState` instead, populated from `port` (the same `local_addr.port()` value `ProxyHandle.port` also uses) at the single `ProxyState` construction site inside `start()`.
- **Files modified:** `crates/nono-proxy/src/server.rs`
- **Verification:** `cargo build -p nono-sandbox-proxy --all-targets` exits 0; `cargo test -p nono-sandbox-proxy --lib` full suite green.
- **Committed in:** `6ef702bd` (Task 1 commit)

**4. [Rule 1-adjacent — avoid introducing dead code] D-03's CONNECT-arm guard placement**
- **Found during:** Task 2's `<read_first>` step, reading `handle_connection`'s full body (not just the plan's partial `<interfaces>` excerpt, per this plan's own instruction)
- **Issue:** The plan's `<action>`/`<interfaces>` ask for a fresh `spiffe_declared_for_upstream` check inside each of the 3 CONNECT dispatch arms (external-proxy chain, bypass-route direct, non-bypass). Direct read found a pre-existing "Block CONNECT tunnels to route upstreams" check (`is_route_upstream`) that already runs unconditionally, before `use_external` is computed and before any of the 3 arms are selected — for ANY route upstream, a strict superset of the SPIFFE-only match `spiffe_declared_for_upstream` would perform (verified by reading both functions: `spiffe_declared_for_upstream` calls the identical `host_port_matches` normalisation `is_route_upstream` uses, with an additional `has_spiffe_source()` AND condition; `connect.rs::parse_connect_target` derives host:port identically to the pre-check's own authority parsing, ruling out a parsing-divergence bypass). Adding the requested check inside each of the 3 arms would therefore be provably unreachable code under current control flow — itself a form of dead code CLAUDE.md forbids.
- **Fix:** Did not add 3 duplicate checks. Instead refined the pre-existing check's audit `denial_category` to `SpiffeUnsupportedPath` (vs. the pre-existing `ConnectBypassesL7`) specifically when the matched route declares SPIFFE — preserving the existing (broader) protection and satisfying D-03's precise-audit-trail intent without adding redundant/unreachable branches. `handle_forward_http`'s guard (the genuinely-missing path) was added fresh, as the plan requested.
- **Files modified:** `crates/nono-proxy/src/server.rs`
- **Verification:** `cargo build -p nono-sandbox-proxy --all-targets` exits 0; `grep -c "SpiffeUnsupportedPath" crates/nono-proxy/src/server.rs` returns `8` (>= 3 required); 2 new end-to-end tests prove denial on both `handle_forward_http` and CONNECT (with `external_proxy` configured, proving the guard preempts the would-be external-chain arm too); full suite (241 tests) green.
- **Committed in:** `fdd4692b` (Task 2 commit)

**5. [Rule 3 - Blocking] `RouteStore::from_loaded_routes` test-only constructor needed in `route.rs`**
- **Found during:** Task 2, while writing the D-03 end-to-end tests
- **Issue:** `RouteStore.routes: HashMap<String, LoadedRoute>` is module-private to `route.rs`. `server.rs`'s new D-03 tests (a different file) need a `RouteStore` containing a `declares_spiffe: true` `LoadedRoute` to exercise the guard, but `RouteStore::load()` would genuinely try to connect to a live SPIRE Workload API for such a route and fail on an unreachable socket (the exact behavior Task 1's own new test proves) — there was no way to build the required fixture without either a live SPIRE agent or a new constructor.
- **Fix:** Added `#[cfg(test)] pub(crate) fn from_loaded_routes(routes: HashMap<String, LoadedRoute>) -> Self` to `route.rs`, test-gated so it adds no production API surface. Mirrors `LoadedRoute.declares_spiffe`'s own documented testability rationale from Plan 113-04.
- **Files modified:** `crates/nono-proxy/src/route.rs`
- **Verification:** `cargo build -p nono-sandbox-proxy --all-targets` exits 0 (the `#[cfg(test)]` gate means this has zero production build impact); `cargo test -p nono-sandbox-proxy --lib` full suite green; both cross-target clippy gates GREEN (confirms no dead_code flag on the test-gated fn during a `cargo test`-shaped build).
- **Committed in:** `fdd4692b` (Task 2 commit, alongside the D-03 guard it supports)

---

**Total deviations:** 5 (2 already-satisfied disposition amendments requiring no code change, 1 type-system correction to the plan's literal field-placement instruction, 1 dead-code-avoidance redesign of the CONNECT-arm guard shape, 1 blocking test-infrastructure addition). All within this plan's own security intent (D-03) or directly required by this plan's own stated acceptance criteria. No scope creep beyond `server.rs` + the one necessary `route.rs` test-only addition.
**Impact on plan:** Moderate on wording/shape, none on the security guarantee — D-03's fail-closed property (unconditional, covers every non-SPIFFE-implementing request path, composes with `require_auth`/`strict_connect_auth`) is fully achieved; `handle_forward_http` gained a genuinely new guard and the CONNECT paths' pre-existing guard gained audit precision, both grep- and test-verified.

## Issues Encountered

None blocking beyond the deviations documented above. `server.rs`/`route.rs` (the two files this plan touched) contain no NEW `#[cfg(target_os = ...)]` blocks introduced by this plan (pre-existing `cfg!(target_os = "macos")` and `#[cfg(not(target_os = "macos"))]` instances in `server.rs` predate this plan and are unrelated to its changes) — nonetheless both mandatory cross-target clippy gates were run per this plan's own `<verification>` block ("run at minimum at this wave's boundary") and both are GREEN, 0 errors, matching Plan 113-04's close with no regression.

## User Setup Required

None — no external service configuration required. `start_fails_closed_on_unreachable_spiffe_socket` exercises the fail-closed unreachable-socket path without a live SPIRE agent (matching `spiffe.rs`'s/`credential.rs`'s/`route.rs`'s own established precedent tests); it takes up to ~10s per run because `SpiffeJwtSource::connect`'s `initial_sync_timeout` is 10s, bounded by an outer 30s `tokio::time::timeout` in the test itself.

## Verification Results

- `cargo build -p nono-sandbox-proxy --all-targets` — **GREEN**, zero errors.
- `cargo build --workspace --all-targets` — **GREEN**, zero errors.
- `cargo test -p nono-sandbox-proxy --lib` — **GREEN**, 241 passed, 0 failed (238 baseline + 3 new). Exceeds the `>= 238` success criterion.
- `cargo fmt --all --check` — **GREEN**.
- `grep -c "no TLS intercept, no SPIFFE, no async RouteStore::load" crates/nono-proxy/src/server.rs` — returns `0` (already true before this plan; Task 3 disposition amendment).
- `grep -c "SpiffeUnsupportedPath" crates/nono-proxy/src/server.rs` — returns `8` (>= 3 required).
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors.
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) — **GREEN**, 0 errors.
- `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used` (Windows-host, informational) — **GREEN**, 0 errors.

## Next Phase Readiness

- D-03's fail-closed guard is live, unconditional, and verified by symbol on every non-SPIFFE-implementing request path this fork has: `handle_forward_http` (new guard), and all 3 CONNECT dispatch arms (external-proxy chain, bypass-route direct, non-bypass — already covered by the pre-existing `is_route_upstream` superset check, now with `SpiffeUnsupportedPath` audit precision). `handle_reverse_proxy` (the one path that DOES have SPIFFE dispatch of its own) is explicitly out of D-03's scope per the plan's own `<interfaces>` block and is Plan 113-06's territory.
- `RouteStore::from_loaded_routes` is available (test-only) for any later plan's own SPIFFE-route test fixtures needing to avoid a live SPIRE agent.
- No blockers for Plan 113-06 or 113-07/113-08.

---
*Phase: 113-spiffe-spire-workload-identity*
*Completed: 2026-08-06*

## Self-Check: PASSED

Both claimed modified files (`crates/nono-proxy/src/server.rs`, `crates/nono-proxy/src/route.rs`)
confirmed present on disk with the described changes. Both claimed task commit hashes
(`6ef702bd`, `fdd4692b`) confirmed present in `git log --oneline --all`.
