---
phase: 114-oauth-capture-absorb-sec-02
plan: 03
subsystem: network-proxy
tags: [oauth-capture, sec-02, route-config, host-only-matching, fail-closed]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-02's RouteConfig.capture: Option<CaptureConfig> declarative field in crates/nono-proxy/src/config.rs"
provides:
  - "LoadedRoute.declares_capture bool field + has_capture_source() accessor"
  - "RouteStore::capture_declared_for_upstream(host_port) predicate, HOST-ONLY matching (D-06 deliberate divergence from spiffe_declared_for_upstream's exact host:port matching)"
  - "host_only_matches() private helper mirroring host_port_matches()'s wildcard semantics on the host portion only, port ignored"
affects: [114-05, 114-06, 114-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "declares_X / has_X_source() / X_declared_for_upstream() triad pattern (established by declares_spiffe in Phase 113) reused for a second predicate, this time pure config-derived (no live external dependency) rather than requiring a live connect"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/server.rs

key-decisions:
  - "capture_declared_for_upstream() uses HOST-ONLY matching, deliberately wider than spiffe_declared_for_upstream()'s exact host:port matching (D-06) — a capture-declared host reachable on ANY port is caught, closing the exact bypass class upstream's own 3c59c62e hardening commit found and fixed. Documented directly in the function's doc comment so a future reader does not narrow it to match the SPIFFE precedent."
  - "host_only_matches() is scoped pub(crate) and explicitly documented as single-purpose (only for capture_declared_for_upstream) so it is not mistakenly reused where port-exact matching is required."

patterns-established:
  - "Sixth consecutive occurrence of this phase-cluster's core lesson: a plan's <interfaces> block under-predicted the real LoadedRoute struct-literal break surface. Plan 114-02's own SUMMARY named 11 test literals fixed in route.rs, but did not (and could not, since declares_capture did not exist yet) anticipate that adding declares_capture would break 2 MORE LoadedRoute { .. } literals in reverse.rs and server.rs test helpers — found only via cargo build --workspace --all-targets, never by grep alone."

requirements-completed: [SEC-02]

# Metrics
duration: ~25min
completed: 2026-08-06
---

# Phase 114 Plan 03: capture_declared_for_upstream Host-Only Matching Predicate Summary

**Added `LoadedRoute.declares_capture`/`has_capture_source()`/`RouteStore::capture_declared_for_upstream()` to `route.rs`, deliberately using host-only (port-ignoring) matching via a new `host_only_matches()` helper — the documented D-06 divergence from the SPIFFE sibling's exact `host:port` matching, proven by tests showing the same host on a different port still counts as capture-declared.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 1 planned (TDD task), completed
- **Files modified:** 3

## Accomplishments
- `LoadedRoute.declares_capture: bool` field, its `Debug` impl line, and `has_capture_source()` accessor exist in `route.rs`, mirroring the `declares_spiffe`/`has_spiffe_source()` pattern exactly, with a doc comment explaining capture has zero live external dependency (pure `route.capture.is_some()` config check at load time).
- `RouteStore::load()` computes `declares_capture` alongside the existing `declares_spiffe` computation, no live connect involved.
- `host_only_matches(pattern, target) -> bool` (private helper) strips the port from both sides before comparing the host portion, applying the same `*.` wildcard-prefix semantics as `host_port_matches()`.
- `RouteStore::capture_declared_for_upstream(host_port) -> bool` mirrors `spiffe_declared_for_upstream()`'s body exactly, substituting `has_capture_source()`/`host_only_matches()` for `has_spiffe_source()`/`host_port_matches()`. Doc comment cites RESEARCH.md Pitfall 4 and upstream's `3c59c62e` hardening lesson, explicitly warning a future reader not to narrow it to match the SPIFFE precedent.
- 6 new tests added: 4 for the raw `host_only_matches()` matcher (same-host-different-port widening, wildcard-host-ignores-port, different-host-no-match, wildcard-does-not-match-apex) and 2 store-level integration tests (`capture_declared_for_upstream` true across a different port for a capture-declared route, false for a non-capture route even on an exact host:port match) — satisfying every behavior case the plan's `<behavior>` block listed.
- `cargo test -p nono-sandbox-proxy --lib` passes 251/251 (245 baseline + 6 new).
- `cargo build --workspace --all-targets` exits 0; `cargo fmt --all -- --check` clean; `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used` clean (no `target_os` cfg gates in the touched files, so the cross-target clippy gate from CLAUDE.md does not apply to this plan).

## Task Commits

Each task was committed atomically:

1. **Task 1: declares_capture field + has_capture_source() + host_only_matches() + capture_declared_for_upstream()** - `96a2e89d` (feat)

## Files Created/Modified
- `crates/nono-proxy/src/route.rs` - `LoadedRoute.declares_capture` field, `Debug` impl line, `has_capture_source()`, `RouteStore::load()` computation, `host_only_matches()` private helper, `RouteStore::capture_declared_for_upstream()`, 6 new tests, plus a fix to `test_loaded_route_debug`'s pre-existing `LoadedRoute { .. }` literal (added the missing `declares_capture: false` field and asserted `has_capture_source()`/the `declares_capture` debug-output substring)
- `crates/nono-proxy/src/reverse.rs` - `declares_capture: false,` added to 1 test-only `LoadedRoute { .. }` literal (`spiffe_route_denies_missing_session_token_before_credential_acquisition`'s helper)
- `crates/nono-proxy/src/server.rs` - `declares_capture: false,` added to 1 test-only `LoadedRoute { .. }` literal (`spiffe_declared_route_store()` test helper)

## Decisions Made
- **Adopted the pre-existing partial work as correct, then completed it.** A prior interrupted executor had already added `LoadedRoute.declares_capture`, `has_capture_source()`, the `Debug` impl line, the `RouteStore::load()` computation, and `RouteStore::capture_declared_for_upstream()` (with its D-06 doc-comment rationale already written and matching the plan's intent). I independently re-read `114-03-PLAN.md`, `114-CONTEXT.md`, and the live `git diff` before proceeding, and found the partial work correct and matching the plan's `<interfaces>` block exactly — including the D-06 doc-comment citing RESEARCH.md Pitfall 4 and `3c59c62e`. I adopted it as-is rather than rewriting it, and completed the missing piece: `host_only_matches()` did not exist, which was the sole reason `cargo build -p nono-sandbox-proxy` failed with E0425 before this session. All of Task 1 (adopted partial work + the missing helper + all 6 tests) is committed as one atomic commit, per the plan's single-task structure.
- **`host_only_matches()` kept `pub(crate)`, not `pub`,** matching `host_port_matches()`'s existing visibility and scoping it to this crate's internal dispatch logic only.
- **Test route construction used a `capture_route_config()` helper** rather than repeating the full 20-field `RouteConfig` literal inline for every capture test, reducing duplication versus the plan's suggestion of writing tests "mirroring the existing spiffe-related tests' shape" — the existing spiffe tests do use full inline literals, but capture's tests needed the identical literal in 2 places (positive and would-be additional cases), so a helper was used instead. `CaptureConfig` has no `Default` derive, so the helper constructs it explicitly (`response_fields: vec![], request_nonce_fields: vec![], max_response_bytes: None`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `host_only_matches()` did not exist, causing the non-compiling state described in the plan handoff**
- **Found during:** Task 1, reading the plan handoff and verifying `git diff crates/nono-proxy/src/route.rs`
- **Issue:** A prior interrupted executor's partial work referenced `host_only_matches` in `capture_declared_for_upstream()`'s body but never defined the function — `cargo build -p nono-sandbox-proxy` failed with E0425 (cannot find function `host_only_matches`).
- **Fix:** Added `host_only_matches(pattern: &str, target: &str) -> bool`, modeled on `host_port_matches()` but stripping the port from both sides before comparing the host portion (and before applying the `*.` wildcard check). Documented as single-purpose (capture-only) so it is not confused with `host_port_matches()`'s exact-port semantics.
- **Files modified:** `crates/nono-proxy/src/route.rs`
- **Verification:** `cargo build -p nono-sandbox-proxy --lib` exits 0; new `test_host_only_matches_*` tests pass.
- **Committed in:** `96a2e89d`

**2. [Rule 3 - Blocking] `test_loaded_route_debug`'s pre-existing `LoadedRoute { .. }` literal was missing the new `declares_capture` field**
- **Found during:** Task 1, running `cargo test -p nono-sandbox-proxy --lib route::` before any new tests were added
- **Issue:** E0063 missing field `declares_capture` — this literal predates this plan and was not part of the interrupted executor's partial edit, since the field addition to the struct definition breaks every exhaustive `LoadedRoute { .. }` literal in the crate, not just the ones the interrupted work touched.
- **Fix:** Added `declares_capture: false,` to the literal, plus assertions on `has_capture_source()` returning `false` and the `Debug` output containing `"declares_capture"`.
- **Files modified:** `crates/nono-proxy/src/route.rs`
- **Verification:** `cargo test -p nono-sandbox-proxy --lib route::test_loaded_route_debug` passes.
- **Committed in:** `96a2e89d`

**3. [Rule 3 - Blocking] 2 more `LoadedRoute { .. }` test-helper literals in `reverse.rs` and `server.rs` needed `declares_capture: false,`**
- **Found during:** Task 1, `cargo test -p nono-sandbox-proxy --lib route:: -- --nocapture` initially failed to even compile the test binary (the full-crate test build touches every module, not just `route.rs`)
- **Issue:** `reverse.rs`'s `spiffe_route_denies_missing_session_token_before_credential_acquisition` test helper and `server.rs`'s `spiffe_declared_route_store()` test helper both construct `route::LoadedRoute { .. }` exhaustively (both predate this plan, both build `declares_spiffe: true` test routes for Phase 113's SPIFFE work) — neither was named in this plan's `files_modified` frontmatter (which listed only `crates/nono-proxy/src/route.rs`) or Plan 114-02's SUMMARY (which only enumerated `RouteConfig` literal breaks, a different struct than `LoadedRoute`).
- **Fix:** Added `declares_capture: false,` immediately after each literal's existing `declares_spiffe: true,` line.
- **Files modified:** `crates/nono-proxy/src/reverse.rs`, `crates/nono-proxy/src/server.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0; `cargo test -p nono-sandbox-proxy --lib` passes 251/251 with zero regressions.
- **Committed in:** `96a2e89d`

---

**Total deviations:** 3 auto-fixed (all Rule 3 - blocking; 1 completed a prior interrupted executor's partial work per the plan handoff's explicit instruction to "finish the missing pieces and commit the whole of Task 1 as one commit", 2 are the same class of `struct`-literal compiler-found break this phase's Plan 114-02 already documented as a recurring pattern, this time on `LoadedRoute` instead of `RouteConfig`)
**Impact on plan:** No scope creep. Every fix is either the plan's own prescribed pattern (mirror `spiffe_declared_for_upstream`'s triad) or a mechanical compile-break closure the plan's own "compiler is ground truth" framing anticipated. This phase-cluster's memory note (`feedback_disposition_confidence_needs_symbol_level.md`) is reinforced again: the fix surface for a new `LoadedRoute` field was 2 sites wider than the interrupted work or the plan's `files_modified` predicted.

## Issues Encountered
None beyond the deviations documented above. The uncommitted partial work left by the prior interrupted executor was independently verified against the plan and found correct in every respect except the missing `host_only_matches()` function, exactly as the handoff instructions anticipated.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
`has_capture_source()` and `RouteStore::capture_declared_for_upstream()` are in place, tested, and documented for Plan 114-05/114-06 (the buffer-and-rewrite enforcement point, which consults `has_capture_source()` to decide whether to buffer a response) and Plan 114-07 (the D-06 cross-path fail-closed guard, which consults `capture_declared_for_upstream()` to deny a capture-declared route arriving via CONNECT/forward-HTTP/external-proxy paths that have no rewrite). No blockers.

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-06*

## Self-Check: PASSED

All modified files verified present on disk (`crates/nono-proxy/src/route.rs`,
`crates/nono-proxy/src/reverse.rs`, `crates/nono-proxy/src/server.rs`, this
SUMMARY.md); commit hash `96a2e89d` verified present in `git log --oneline`.
`cargo test -p nono-sandbox-proxy --lib` re-verified at 251 passed, 0 failed
before writing this section.
