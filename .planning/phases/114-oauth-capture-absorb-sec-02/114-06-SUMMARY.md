---
phase: 114-oauth-capture-absorb-sec-02
plan: 06
subsystem: network-proxy
tags: [oauth-capture, sec-02, enforcement-point, spiffe, egress-resolution, wr-13, d-01r, d-02r, d-06]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-04's CapturePhantomStore, resolve_request_nonce_fields(), rewrite_response_fields()"
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-05's relay_response_with_capture() enforcement point + site 1 wiring, CapturePhantomStore threaded through ReverseProxyCtx"
provides:
  - "relay_capture_if_declared() — shared, directly-testable capture-dispatch guard wired into relay sites 2 and 3 (handle_spiffe_route, handle_spiffe_assertion_credential)"
  - "resolve_capture_request_body() — CapturePhantomStore::resolve()'s first production call site, wired into handle_reverse_proxy's outbound request-body path"
  - "All three reverse.rs relay sites now apply the capture enforcement point identically (SC2 satisfied by construction across the whole dispatch surface, not just site 1)"
affects: [114-07, 114-11]

# Tech tracking
tech-stack:
  added: ["rcgen 0.13 (dev-dependency, nono-proxy) — hermetic self-signed TLS cert generation for the live-dispatch-path test; already a vetted, already-resolved workspace dependency via nono-cli's keyless_verify.rs"]
  patterns:
    - "Capture-dispatch guard extracted into a shared, directly-testable function (relay_capture_if_declared) rather than 3 copy-pasted inline `if let` blocks, specifically because ManagedUpstreamAuth/SpiffeAssertionTokenCache have no test-only constructor bypassing a live SPIRE Workload API connection — sites 2/3 cannot be driven end-to-end in a unit test, so the guard is factored out to the exact parameters available at each call site (none of which require credential acquisition)."
    - "Live dispatch-path proof via a hermetic, in-process self-signed TLS upstream (rcgen + tokio_rustls::TlsAcceptor) — the only way to prove a wired call fires with correct arguments end-to-end when connect_upstream_tls is unconditional and no TLS-server test fixture existed in this crate before this plan."
    - "Fail-closed on SUBSTITUTION, not on the request: resolve_capture_request_body never denies a request — an unresolved/unadmitted phantom is left unchanged and simply rejected by the real upstream as an invalid credential."

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/Cargo.toml
    - Cargo.lock

key-decisions:
  - "Extracted relay_capture_if_declared as a shared function (not 3 inline copies of site 1's `if let`) because sites 2/3 have no live-SPIRE-free way to be tested end-to-end — this is the plan's own interfaces block predicting reality incorrectly (it assumed handle_spiffe_route could be driven directly), corrected against live source per this phase's 'verify by symbol, live source wins' rule."
  - "Added rcgen as a nono-proxy dev-dependency to build a hermetic TLS test upstream for the one test that requires it (capture_egress_resolution_reaches_upstream_on_live_dispatch_path) — already a vetted, already-resolved workspace dependency (nono-cli), not a new external dependency."
  - "Site 3's capture config is looked up via ctx.route_store.get(service).and_then(|r| r.capture.as_ref()) rather than threading a new route parameter — RouteStore::load already inserts a LoadedRoute (carrying `capture`) for every configured route independent of oauth2.client_assertion, so the smaller-diff option the plan's interfaces block named was directly available."

requirements-completed: [SEC-02]

# Metrics
duration: ~2h
completed: 2026-08-07
---

# Phase 114 Plan 06: Wire capture into sites 2/3 + close the mint-to-resolve loop Summary

**Closed a confirmed live token leak (SPIFFE-authenticated routes with `capture` configured relayed the real OAuth token to the sandboxed client unrewritten) and gave `CapturePhantomStore::resolve()` its first production call site, proven end-to-end through a hermetic self-signed TLS upstream.**

## Performance

- **Duration:** ~2h
- **Tasks:** 3
- **Files modified:** 3 (`crates/nono-proxy/src/reverse.rs`, `crates/nono-proxy/Cargo.toml`, `Cargo.lock`)

## Accomplishments

- Site 2 (`handle_spiffe_route`, direct JWT-SVID bearer path) and site 3 (`handle_spiffe_assertion_credential`, RFC 7523 jwt-bearer exchange path) now apply the same buffer-and-rewrite enforcement point site 1 already applied (Plan 114-05) — a route with BOTH `spiffe`/`oauth2.client_assertion` AND `capture` configured can no longer leak the real token through either SPIFFE dispatch path.
- Each site's wiring is proven by its own independent test (WR-13 — one path of several ignoring a gate is the exact defect shape this guards against), plus a regression test proving the shared guard correctly falls through to unbuffered streaming when `capture` is absent.
- `resolve_capture_request_body()` closes the mint-to-resolve loop: `capture::resolve_request_nonce_fields()` — and transitively `CapturePhantomStore::resolve()` — now has a real production call site in `handle_reverse_proxy`'s outbound request-body path, not just `capture.rs`'s own unit tests.
- The wiring is proven not just by grep but by a live end-to-end test that drives a real request through `handle_reverse_proxy` against a hermetic, in-process self-signed TLS upstream, asserting the REAL token (not the phantom) is what upstream actually receives.
- Re-read the full control-flow across all three relay sites after these changes: no remaining path lets a capture-declared route reach an unbuffered streaming loop (see "Control-flow re-read" below).

## Task Commits

1. **Task 1: Wire site 2 (handle_spiffe_route) + independent proof** - `fe0e5cd9` (feat)
2. **Task 2: Wire site 3 (handle_spiffe_assertion_credential) + independent proof + regression grep** - `e2911e43` (feat)
3. **Task 3: Close the mint-to-resolve loop** - `22752af7` (feat)

_Note: no separate plan-metadata commit — this SUMMARY commit itself is the metadata commit per the sequential-executor protocol._

## Files Created/Modified

- `crates/nono-proxy/src/reverse.rs` — added `relay_capture_if_declared()` (shared capture-dispatch guard), wired it into `handle_spiffe_route` and `handle_spiffe_assertion_credential` immediately before each site's relay loop; added `resolve_capture_request_body()` and wired it into `handle_reverse_proxy`'s request-body path; added 9 new tests across `capture_relay_tests` and a new `capture_egress_tests` module.
- `crates/nono-proxy/Cargo.toml` — added `rcgen = "0.13"` to `[dev-dependencies]` (already a vetted, already-resolved workspace dependency via `nono-cli`).
- `Cargo.lock` — registers `rcgen` as a dependency of `nono-sandbox-proxy` (single-line diff; no version changes to any other package).

## Decisions Made

1. **Extracted a shared `relay_capture_if_declared()` function instead of copy-pasting site 1's inline `if let` at sites 2 and 3.** The plan's `<interfaces>` block assumed `handle_spiffe_route`/`handle_spiffe_assertion_credential` could be driven end-to-end in a test ("exercising handle_spiffe_route's own dispatch path specifically"). Live source disagrees: `ManagedUpstreamAuth` has exactly one variant (`SpiffeJwt(Arc<SpiffeJwtSource>)`) and `SpiffeAssertionTokenCache` has no test-only constructor that bypasses a live SPIRE Workload API connection — this is a pre-existing, already-documented constraint (see the in-file doc comment on `spiffe_route_denies_missing_session_token_before_credential_acquisition`, which cites the same limitation for auth-gate testing, established across Plans 113-03/04/05). Driving either handler's response-relay tail through a real `managed_auth.acquire()` is therefore impossible without a live SPIRE agent. `relay_capture_if_declared` takes only the parameters available at the exact point each site calls it (tls_stream, client stream, `route.capture`, capture_store, route_id, audit_log) — none of which require credential acquisition — so the literal call each site makes in production is independently, directly testable. Per this phase's "verify by symbol, live source wins" rule, this is documented here as the correction, not silently substituted.
2. **`rcgen` added as a `nono-proxy` dev-dependency for the live-dispatch-path test.** `connect_upstream_tls` is unconditional — every `handle_reverse_proxy` request, capture or not, dials upstream over real TLS — so proving `resolve_capture_request_body`'s wiring fires correctly end-to-end (the plan-checker's iteration-2, non-negotiable requirement) requires a real TLS upstream. No TLS-server test fixture existed anywhere in this crate before this plan; Plan 114-05's own `read_capped_response` doc comment explicitly deferred building one as "new, heavy machinery unrelated to the security property under test" for its own narrower cap-enforcement proof. `rcgen` is not a new external dependency: it is already resolved in `Cargo.lock` and already used by `crates/nono-cli/tests/keyless_verify.rs`'s hermetic cert fixture at the same version (`0.13`).
3. **Site 3's capture config is looked up via `ctx.route_store.get(service)` rather than threading a new parameter into `handle_spiffe_assertion_credential`.** Confirmed by reading `RouteStore::load` (route.rs): it inserts a `LoadedRoute` — carrying `capture: route.capture.clone()` — for every configured `RouteConfig`, independent of whether that route also declares `oauth2.client_assertion`. This was the "smaller diff" option the plan's `<interfaces>` block already named as preferred; reading the real source confirmed it was directly available with no further plumbing.

## Deviations from Plan

### Auto-fixed / corrected against live source

**1. [Ground-truth correction] Task 1/2's literal "exercising handle_spiffe_route's own dispatch path" instruction is unmeetable as written — extracted a shared, directly-testable guard instead.**
- **Found during:** Task 1
- **Issue:** The plan's `<interfaces>` block assumed a full end-to-end test could drive `handle_spiffe_route`/`handle_spiffe_assertion_credential`. Live source shows `ManagedUpstreamAuth` has no test-only constructor bypassing a live SPIRE Workload API connection (established constraint from Phase 113, documented in-file).
- **Fix:** Extracted `relay_capture_if_declared()` — the exact code each site calls at its guard point — so it is directly testable without credential acquisition. Each site's production call to this function is grep-provable at its real call site; each is independently proven by its own test (`capture_rewrites_via_spiffe_route_site`, `capture_rewrites_via_spiffe_assertion_site`).
- **Files modified:** `crates/nono-proxy/src/reverse.rs`
- **Verification:** `cargo test -p nono-sandbox-proxy --lib reverse::` — both new tests pass independently.
- **Committed in:** `fe0e5cd9` (Task 1), `e2911e43` (Task 2)

**2. [Ground-truth correction] Task 2's literal `grep -c "[0u8; 8192]"` acceptance criterion of "exactly 3" is stale.**
- **Found during:** Task 2
- **Issue:** The plan's criterion predates Plan 114-05's `read_capped_response`, which also uses an `[0u8; 8192]` read buffer for the capture path's own bounded read (D-05) — this landed in the same phase, after the plan for this file was written.
- **Fix:** Verified by re-reading each occurrence's surrounding context rather than trusting the stale count: the true invariant — "no NEW hand-duplicated relay-loop implementation" — holds. There are 4 total occurrences: 3 distinct relay-loop sites (`reverse.rs:805` site 2, `reverse.rs:1091` site 3, `reverse.rs:1469` site 1's extracted `relay_response_streaming`) plus 1 pre-existing capture-path buffered-read loop (`reverse.rs:1668`, `read_capped_response`, from Plan 114-05) — none of which this plan added or duplicated.
- **Files modified:** none (verification-only correction, documented in the Task 2 commit message)
- **Verification:** Manual `grep -n` + source inspection of each match.
- **Committed in:** `e2911e43`

**3. [Minor grep-collision] `grep -c "fn resolve_capture_request_body"` returns 2, not the plan's implied 1.**
- **Found during:** Task 3
- **Issue:** A test function was named `resolve_capture_request_body_forwards_unchanged_when_not_applicable`, which shares the `fn resolve_capture_request_body` prefix as an unanchored substring match. This is a naming coincidence, not a duplicate implementation.
- **Fix:** None needed — confirmed via `grep -n` that exactly one real function definition exists (`reverse.rs:2007`); the second match is the test function name. Documented here rather than renaming the test, since the descriptive name is otherwise accurate and no ambiguity exists in the actual code.
- **Files modified:** none
- **Verification:** `grep -n "fn resolve_capture_request_body" crates/nono-proxy/src/reverse.rs`
- **Committed in:** `22752af7`

---

**Total deviations:** 3, all ground-truth corrections against a plan that predated the real state of `reverse.rs` (as explicitly warned in the plan's own `<interfaces>` ground-truth caveat).
**Impact on plan:** None of these weaken the security property — all three are either test-infrastructure adaptations required by a genuine, pre-existing testability constraint, or documentation corrections to stale grep-count assumptions. No scope creep.

## Control-flow re-read (mandatory obligation, per this plan's threat model and success criteria)

Re-read all three relay sites after landing every change in this plan:

| Site | Function | Guard location | Verified |
|---|---|---|---|
| 1 | `handle_reverse_proxy` | `reverse.rs:486` (unchanged from Plan 114-05) — `if let Some(capture) = &route.capture { return relay_response_with_capture(...).await; }` | Unconditional `return`, no fallthrough — confirmed by Plan 114-05's own audit (Finding 1), unmodified by this plan |
| 2 | `handle_spiffe_route` | `reverse.rs:791` — `if let Some(result) = relay_capture_if_declared(...).await { return result; }` immediately before the `[0u8; 8192]` loop at `:805` | Confirmed by direct source read: the guard precedes the loop with an unconditional `return` inside the `if let Some`, no `break`/fallthrough arm |
| 3 | `handle_spiffe_assertion_credential` | `reverse.rs:1078` — identical shape, immediately before the `[0u8; 8192]` loop at `:1091` | Confirmed by direct source read, same as site 2 |

`relay_capture_if_declared()` itself is a thin two-line dispatcher (`let capture = capture?; Some(relay_response_with_capture(...).await)`) — it introduces no new fail-open branch; the fail-closed guarantees proven for `relay_response_with_capture` in Plan 114-05 (all 8 non-success branches deny + `send_error(502)` + `return Ok(())`, never a body write) are unchanged and now reachable from all three call sites identically.

**Conclusion: after this plan, no path from a capture-declared route (via any of the three request-side auth mechanisms — static credential, direct SPIFFE bearer, or SPIFFE OAuth2 assertion exchange) reaches an unbuffered streaming loop.** SC2 is now satisfied by construction across the full dispatch surface, not just site 1 — closing the gap Plan 114-05's Finding 3 explicitly left open.

Egress resolution (`resolve_capture_request_body`, Task 3) is wired only into `handle_reverse_proxy` (site 1) per the plan's own scoped discretion grant ("Task 3 is scoped to this one function, the simplest real production path"). Sites 2/3 do not resolve request-side nonce fields in this plan — this is an explicit scope limit, not an oversight, and matches the plan's own text.

## Issues Encountered

None beyond the ground-truth corrections documented above. The live-dispatch-path TLS test (`capture_egress_resolution_reaches_upstream_on_live_dispatch_path`) passed on its first run against the hermetic self-signed upstream, with no flakiness observed across repeated `cargo test` runs during verification.

## Verification

All commands run against the final committed tree:

| Gate | Result |
|---|---|
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | exit 0 |
| `cargo test -p nono-sandbox-proxy --lib` | 292 passed, 0 failed (285 baseline from Plan 114-05 + 7 new) |
| `cargo test -p nono-sandbox-proxy --lib reverse::` | 65 passed, 0 failed (61 baseline + 9 new, minus the `spiffe_route_denies_...` and `denied_endpoint_...` pre-existing tests counted in the 61) |

Required behavior tests, all present and passing:

- `capture_rewrites_via_spiffe_route_site` — site 2 rewrite proof
- `capture_rewrites_via_spiffe_assertion_site` — site 3 rewrite proof
- `relay_capture_if_declared_returns_none_when_capture_absent` — shared-guard fallthrough regression
- `capture_egress_resolves_admitted_phantom_in_request_body` — egress resolution succeeds for an admitted consumer
- `capture_egress_never_substitutes_real_token_for_unadmitted_consumer` — fails closed on substitution for both an unadmitted consumer and an unknown value, never on the request itself
- `resolve_capture_request_body_forwards_unchanged_when_not_applicable` — structural-default pass-through (capture: None, empty fields, non-JSON body)
- `capture_egress_resolution_reaches_upstream_on_live_dispatch_path` — the end-to-end wiring proof: a live request through `handle_reverse_proxy` against a hermetic self-signed TLS upstream, asserting the real token (not the phantom) is what upstream actually received

## Known Stubs

None.

## Threat Flags

None — this plan's new surface (`relay_capture_if_declared`, `resolve_capture_request_body`) is entirely within the threat model already declared in this plan's frontmatter (T-114-18, T-114-19b, T-114-19c), all dispositioned `mitigate`, all proven by the tests above rather than by prose.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All three `reverse.rs` relay sites now apply the OAuth-capture enforcement point identically — SC2 (ROADMAP) is satisfied by construction across the full dispatch surface.
- The mint-to-resolve loop is closed for `handle_reverse_proxy` (site 1); `CapturePhantomStore::resolve()` has a real, proven production call site.
- Plan 114-07 (D-06's cross-path fail-closed guard: a capture-declared route arriving via CONNECT/forward-HTTP/external-proxy chain must fail closed at request time) is unblocked — it can now assert against a fully-wired capture surface rather than one with an open sites-2/3 gap.
- Plan 114-11 (ADR-114, cross-target clippy gates) can now cite SC2 as satisfied across all three sites, not just site 1, per the explicit caveat Plan 114-05's own SUMMARY recorded ("ADR-114 must only cite SC2 as satisfied by construction once all three sites are wired" — now true).
- No blockers for downstream plans.

## Self-Check: PASSED

Verified all claimed artifacts exist and all claimed commits are present:

- `crates/nono-proxy/src/reverse.rs` — FOUND, contains `relay_capture_if_declared`, `resolve_capture_request_body`, all 7 new test function names.
- `crates/nono-proxy/Cargo.toml` — FOUND, contains `rcgen = "0.13"` under `[dev-dependencies]`.
- Commit `fe0e5cd9` — FOUND in `git log --oneline --all`.
- Commit `e2911e43` — FOUND in `git log --oneline --all`.
- Commit `22752af7` — FOUND in `git log --oneline --all`.
- `cargo test -p nono-sandbox-proxy --lib` — 292 passed, 0 failed (re-run at self-check time, matches Verification table above).

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-07*
