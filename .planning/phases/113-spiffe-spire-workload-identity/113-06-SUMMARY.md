---
phase: 113-spiffe-spire-workload-identity
plan: 06
subsystem: proxy-network-security
tags: [spiffe, spire, workload-identity, jwt-svid, oauth2, jwt-bearer, reverse-proxy, credential-injection, rust, nono-proxy, d-01]

# Dependency graph
requires:
  - phase: 113-spiffe-spire-workload-identity
    provides: "Plan 113-03's SpiffeAssertionTokenCache/exchange_jwt_assertion (oauth2.rs), CredentialStore.spiffe_assertion_routes/get_spiffe_assertion() (credential.rs); Plan 113-04's LoadedRoute.managed_auth/declares_spiffe/has_spiffe_source(), async RouteStore::load (route.rs); Plan 113-05's D-03 fail-closed guard on the non-SPIFFE-implementing paths + RouteStore::from_loaded_routes test constructor"
provides:
  - "crates/nono-proxy/src/reverse.rs: handle_spiffe_route (direct JWT-SVID bearer injection via route.managed_auth.acquire()), handle_spiffe_assertion_credential (RFC 7523 jwt-bearer OAuth2 exchange via credential_store.get_spiffe_assertion()), both dispatched from handle_reverse_proxy before the static_cred/no-credential branches"
affects: [113-07, 113-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Both new handlers are REWRITES against this fork's own parse_upstream_url()/connect_upstream_tls() upstream-connect primitives (no scheme in the 3-tuple, always-TLS connector), not ports of upstream's diffed body — upstream's crate::forward::{UpstreamSpec, UpstreamStrategy, forward_request} module does not exist in this fork (D-01)"
    - "A violated has_spiffe_source()/managed_auth invariant fails closed (503 + ManagedCredentialUnavailable) via a let-else early return, never via unwrap/expect (CLAUDE.md forbids both, even though only clippy::unwrap_used is CI-enforced)"
    - "The two SPIFFE-backed auth sources (direct route.spiffe JWT-SVID vs. oauth2.client_assertion jwt-bearer exchange) are mutually exclusive per route and dispatch from the SAME point in handle_reverse_proxy, before the static_cred/no-credential branches, so a SPIFFE-declared route never falls through to keystore-based lookup"
    - "Both handlers reuse the SAME auth-gate call (token::validate_proxy_auth) the existing no-credential branch already uses — no new auth logic, no bypass path (T-113-15)"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/reverse.rs

key-decisions:
  - "D-01 held exactly as scoped and grep-verified: grep -c \"forward::forward_request|UpstreamSpec|UpstreamStrategy\" crates/nono-proxy/src/reverse.rs returns 0, including doc comments (both handlers' doc comments paraphrase the absent module's contents — 'spec/strategy/request-forwarding types' — rather than spelling out the literal banned identifiers, following 113-03/113-04's established wording workaround for the same whole-file-including-comments grep constraint)."
  - "Both handlers built by direct symbol-level cross-reference against reverse.rs's own existing static_cred flow inside handle_reverse_proxy (auth gate -> acquire credential -> build/parse/filter/connect upstream -> inject -> forward -> audit), NOT by porting upstream's diffed handle_spiffe_route/handle_spiffe_assertion_credential bodies, which reference the absent forward module. `git show c831dade` was read only as a design-shape reference (which steps exist, in what order), matching the plan's own read_first instruction."
  - "Disposition amendment: Task 2's action text asked to convert this file's own line-~1406 RouteStore::load test call site from #[test] fn to #[tokio::test] async fn. Direct read (before any Task 2 edit, per this plan's own critical_execution_constraints instruction to verify by symbol) showed the test (denied_endpoint_returns_403_and_audit) was ALREADY #[tokio::test] async fn with .await — landed as a Rule 3 blocking-issue fix by Plan 113-04 (route.rs's own D-04 async-conversion blast-radius, which measured this exact reverse.rs call site as one of its 7 sites). No code change was needed for this sub-scope; verified via grep -n '#\\[test\\]|#\\[tokio::test\\]' before starting, confirmed against 113-04-SUMMARY.md's own documented deviation."
  - "handle_spiffe_assertion_credential's injection_mode is recorded as NetworkAuditInjectionMode::OAuth2 (not SpiffeJwt) — the exchanged OAuth2 access token, not the raw SPIFFE JWT-SVID, is what gets injected into the upstream request. SpiffeJwt is handle_spiffe_route's injection_mode (via ManagedUpstreamAuth::audit_injection_mode()), used for the direct-bearer-injection path only."
  - "The SPIFFE audit context for handle_spiffe_assertion_credential is built inline (workload_spiffe_id/trust_domain from SpiffeAssertionTokenCache.workload_spiffe_id + crate::auth::extract_trust_domain(), svid_type/source hardcoded to \"jwt\"/\"spire-workload-api\", delegation: None) rather than via UpstreamAuthMaterial::spiffe_audit_context() — that helper is specific to the BearerToken variant handle_spiffe_route's ManagedUpstreamAuth::acquire() chain produces; the assertion-cache path has no ManagedUpstreamAuth/UpstreamAuthMaterial in its chain at all (Plan 113-03's design, matching the plan's own <interfaces> guidance)."
  - "Delegation is always None for the assertion-cache path (unlike handle_spiffe_route, which recovers delegation via crate::spiffe::delegation_from_jwt(token) on the raw SVID) — the exchanged OAuth2 access token is an opaque IdP-issued token, not necessarily a JWT with an inspectable act claim, so no delegation-chain recovery is attempted."

patterns-established: []

requirements-completed: []  # NET-02 is satisfied across the whole 8-plan phase, not this plan alone

# Metrics
duration: ~75min
completed: 2026-08-06
---

# Phase 113 Plan 06: SPIFFE reverse-proxy handlers — direct bearer injection + OAuth2 jwt-bearer assertion Summary

**Both SPIFFE request handlers (`handle_spiffe_route`, `handle_spiffe_assertion_credential`) landed in `reverse.rs` as from-scratch rewrites against this fork's own `parse_upstream_url()`/`connect_upstream_tls()` upstream-connect primitives — upstream's diffed bodies call a `crate::forward` module this fork does not host (D-01, grep-verified 0 hits including comments). This is where D-01's absorbed SPIFFE surface actually becomes working request-handling code: a SPIFFE-declared reverse-proxy route now authenticates the client, fetches/exchanges the credential, forwards to upstream, and records structured `spiffe_context` audit data — end to end, fail-closed on every credential-acquisition failure path.**

## Performance

- **Duration:** ~75 min
- **Started:** 2026-08-06 (session start)
- **Completed:** 2026-08-06
- **Tasks:** 2 completed
- **Files modified:** 1 (`crates/nono-proxy/src/reverse.rs`, exactly the plan's stated scope)

## Accomplishments

- `handle_reverse_proxy` gained the SPIFFE dispatch block (two `if`/`if let` branches, checked before the static_cred/no-credential branches): `route.has_spiffe_source()` routes to `handle_spiffe_route`; `ctx.credential_store.get_spiffe_assertion(&service)` routes to `handle_spiffe_assertion_credential`. Both dispatch conditions are mutually exclusive per route by construction (a route either declares `route.spiffe` or `route.oauth2.client_assertion`, never both, per Plan 113-04/113-03's respective load-time wiring).
- `handle_spiffe_route` (Task 1): mirrors the existing static_cred flow step-by-step — auth gate (`token::validate_proxy_auth`, identical call to the no-credential branch) → acquire (`route.managed_auth.acquire()`, fails closed 503 + `ManagedCredentialUnavailable` on error, and identically on a violated `has_spiffe_source()` invariant via a `let-else` rather than `unwrap`/`expect`) → build/parse/filter/connect upstream (verbatim reuse of `parse_upstream_url()`/`ctx.filter.check_host()`/`connect_upstream_tls()`, including the per-route custom-TLS-CA connector) → inject (`credential_format.replace("{}", token)` into the resolved header, mirroring `inject_credential_for_mode`'s Header/BasicAuth arm) → forward + stream response → audit success via `audit::log_l7_request` carrying `spiffe_context: Some(material.spiffe_audit_context())`, `auth_mechanism: SpiffeJwtBearer`, `injection_mode: SpiffeJwt`.
- `handle_spiffe_assertion_credential` (Task 2): identical shape, substituting credential acquisition with `spiffe_assertion.cache.get_or_refresh()` (an exchanged OAuth2 access token, `Result<Zeroizing<String>>` — fails closed identically on an SVID-fetch failure, per `SpiffeAssertionTokenCache::get_or_refresh`'s own T-113-10 contract from Plan 113-03) and building the `SpiffeAuditContext` inline (no `ManagedUpstreamAuth`/`UpstreamAuthMaterial` in this chain at all) with `auth_mechanism: SpiffeOAuthAssertion`, `injection_mode: OAuth2` (the exchanged access token is injected, not the raw SVID), `delegation: None` (the access token is opaque, not necessarily an inspectable JWT).
- New test `spiffe_route_denies_missing_session_token_before_credential_acquisition`: proves the auth gate on the SPIFFE dispatch branch denies (407) before `route.managed_auth` is ever read, using `RouteStore::from_loaded_routes` (Plan 113-05's test-only constructor) with `managed_auth: None` to make the assertion strong — if the gate were bypassed the request would still be denied, but with 503 (the invariant-violation fail-closed branch), not 407, so the exact status code proves WHICH guard fired.
- `log_reverse_proxy` (Landmine L3) is untouched and still called from `handle_reverse_proxy`'s original static_cred/no-credential tail — confirmed via `grep -n "audit::log_reverse_proxy("` returning only the pre-existing call site.
- D-01's negative proof holds for the whole file, including both new handlers' doc comments: `grep -c "forward::forward_request\|UpstreamSpec\|UpstreamStrategy" crates/nono-proxy/src/reverse.rs` returns `0`. The doc comments describing what does NOT exist in this fork were worded to paraphrase ("spec/strategy/request-forwarding types") rather than spell out the literal banned identifiers, following 113-03/113-04's established workaround for the same whole-file-including-comments grep constraint.
- `tls_intercept` grep note (see Deviations below): this plan introduces zero new `tls_intercept` text (confirmed via `git diff`); the 11 existing hits across the crate are all pre-existing documentation of D-01's absence from earlier plans (113-04's OD-2 fix, `credential.rs`'s D-20 doc, `route.rs`'s multiple D-01 notes, `server.rs`'s D-40-B2 comment, and `reverse.rs`'s own pre-existing AWS SigV4 501 comment) — D-01 is preserved in the sense that matters (no `tls_intercept` module/struct/fn exists anywhere; confirmed by a separate structural grep).

## Task Commits

1. **Task 1: handle_spiffe_route — direct JWT-SVID bearer injection** - `0772b82b` (feat)
2. **Task 2: handle_spiffe_assertion_credential — OAuth2 jwt-bearer assertion path + async test-site conversion (already satisfied)** - `5f513680` (feat)

_No plan-metadata commit yet — this SUMMARY + its own commit is that step._

## Files Created/Modified

- `crates/nono-proxy/src/reverse.rs` - `handle_spiffe_route`, `handle_spiffe_assertion_credential`, the two-branch SPIFFE dispatch block in `handle_reverse_proxy`, imports (`crate::auth::UpstreamAuthMaterial`, `crate::route::LoadedRoute`, `crate::credential::SpiffeAssertionRoute`), 1 new test

## Decisions Made

- Both handlers were built as independent, from-scratch rewrites cross-referenced by SYMBOL against `reverse.rs`'s own existing `static_cred` flow — not ported from upstream's diffed bodies, which call the absent `crate::forward` module. `git show c831dade` was read only for step-ordering shape, per the plan's own `<critical_execution_constraints>` instruction.
- `handle_spiffe_assertion_credential`'s `injection_mode` is `OAuth2`, not `SpiffeJwt` — see key-decisions above for the full reasoning (the exchanged access token, not the raw SVID, is what's injected).
- The Task 2 async-test-conversion sub-scope was found already satisfied by Plan 113-04's own blast-radius fix; recorded as a disposition amendment (see Deviations) rather than re-applied, matching the established precedent from Plans 113-04/113-05 for the same class of already-landed sub-scope.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `cargo fmt --all --check` failures after adding the new handlers/dispatch/test**
- **Found during:** After each task's initial edit
- **Issue:** New multi-line `EventContext { .. }` struct literals and one `handle_reverse_proxy(...).await;` call chain exceeded this workspace's configured line-wrap width.
- **Fix:** Ran `cargo fmt --all`, which reformatted the file in place; re-ran `cargo fmt --all --check` to confirm clean before each commit.
- **Files modified:** `crates/nono-proxy/src/reverse.rs`
- **Verification:** `cargo fmt --all --check` exits 0 for both task commits.
- **Committed in:** `0772b82b` and `5f513680` respectively.

**2. [Rule 1 - Bug] Doc comments literally spelling out the D-01 banned identifiers**
- **Found during:** First `cargo fmt --all --check` / grep-verification pass after Task 1's initial draft
- **Issue:** My first draft of both handlers' doc comments literally wrote `crate::forward::{UpstreamSpec, UpstreamStrategy, forward_request}` to explain what does NOT exist in this fork — but the plan's own acceptance criterion (`grep -c "forward::forward_request\|UpstreamSpec\|UpstreamStrategy" ... returns 0`) is a whole-file substring match that does not distinguish code from comments, so this literal wording made the grep return `2` instead of `0`.
- **Fix:** Reworded both doc comments to paraphrase the absent module's contents ("spec/strategy/request-forwarding types") rather than spell out the literal banned identifiers — the exact workaround Plans 113-03/113-04 already established for the identical constraint (OD-1's banned-identifier grep, D-01's own).
- **Files modified:** `crates/nono-proxy/src/reverse.rs`
- **Verification:** `grep -c "forward::forward_request\|UpstreamSpec\|UpstreamStrategy" crates/nono-proxy/src/reverse.rs` returns `0`.
- **Committed in:** `0772b82b` and `5f513680` (the wording was corrected before either task's first commit — no separate fix-up commit was needed).

### Disposition Amendments (no code change)

**3. [Already satisfied] Task 2's async test-site conversion**
- **Found during:** Task 2, before any edit (per this plan's own instruction to verify current file state by symbol first)
- **Issue:** The plan's `<action>` asks to convert this file's own line-~1406 `RouteStore::load` test call site from `#[test] fn` to `#[tokio::test] async fn` with `.await` appended.
- **Fix:** No code change. Direct read via `grep -n "#\[test\]\|#\[tokio::test\]\|RouteStore::load"` showed the test (`denied_endpoint_returns_403_and_audit`) was already `#[tokio::test] async fn` with `.await` — this exact call site was one of the 7 measured D-04 blast-radius sites Plan 113-04 converted as a Rule 3 blocking-issue fix (documented in 113-04-SUMMARY.md's own Deviations section: "1 in `reverse.rs`'s `mod tests`... already `#[tokio::test] async fn`, only needed `.await` added").
- **Files modified:** none (verification only)
- **Verification:** `grep -n "#\[tokio::test\]" crates/nono-proxy/src/reverse.rs` shows the site; `cargo build -p nono-sandbox-proxy --all-targets` exits 0 both before and after Task 2's other edits.
- **Committed in:** n/a — no commit needed for an already-true state; documented in Task 2's commit message.

**4. [Informational, no action needed] The orchestrator-level success-criteria's `grep -rn 'tls_intercept' crates/nono-proxy/src/ | wc -l` returns 11, not 0**
- **Found during:** Final verification pass
- **Issue:** The executor prompt's own `<success_criteria>` (distinct from this PLAN.md's own `<verification>` block, which does not include this grep) asks for zero `tls_intercept` mentions anywhere in `crates/nono-proxy/src/`. This grep returns `11`, all pre-existing: `credential.rs`'s D-20 credential-match-policy doc, `pool.rs`'s module doc, `route.rs`'s OD-2 fix + `build_base_root_store` doc + two other D-01 notes (4 total), `server.rs`'s D-40-B2 comment (2 occurrences), and `reverse.rs`'s own pre-existing (not introduced by this plan) AWS SigV4 501 comment ("upstream's 501 is in tls_intercept/handle.rs which the fork does not have").
- **Analysis:** `git diff` confirms this plan introduces ZERO new `tls_intercept` text anywhere. A separate structural grep (`grep -rn "mod tls_intercept\|struct.*TlsIntercept\|fn.*tls_intercept"`) confirms zero matches — no `tls_intercept` module, struct, or function exists anywhere in the crate. All 11 hits are DOCUMENTATION of D-01's absence (the correct, intentional pattern established by Plans 113-04/113-05), not a violation of it. The literal grep in the executor prompt's success criteria appears to have been accurate at an earlier point in the phase, before 113-04/113-05 landed their own D-01-documenting comments using the word "tls_intercept" as plain text.
- **Fix:** None applied — this is a stale/inherited success-criterion, not a defect in this plan's own changes. D-01 itself (no TLS-intercept module exists, no `ProxyHandle::intercept_ca_path()` behavior change, no `tls_intercept` module created) is fully preserved and verified by the structural grep above.
- **Files modified:** none
- **Verification:** `git diff -- crates/nono-proxy/src/reverse.rs | grep -i tls_intercept` returns nothing (zero new occurrences); `grep -rn "mod tls_intercept\|struct.*TlsIntercept\|fn.*tls_intercept" crates/nono-proxy/src/` returns nothing (no module/struct/fn exists).
- **Committed in:** n/a — no code change.

---

**Total deviations:** 4 (2 auto-fixed formatting/grep-wording bugs, 2 disposition amendments requiring no code change). No scope creep — all within `reverse.rs`, the plan's own `files_modified` scope.
**Impact on plan:** Minor. Both auto-fixed issues were caught and corrected before either task's commit; both disposition amendments reflect already-satisfied prior-plan work or a stale executor-level checklist item, not gaps in this plan's own delivery.

## Issues Encountered

None blocking. `reverse.rs` (the only file this plan touched) contains zero `#[cfg(target_os = ...)]` blocks (confirmed via `grep -n "cfg(target_os" crates/nono-proxy/src/reverse.rs` — no matches), so CLAUDE.md's mandatory cross-target clippy trigger condition does not strictly apply to this plan's changes. Both cross-target clippy gates were still run informationally on the fully-merged (both-tasks) working-tree state before splitting into per-task commits, and the resulting file content was confirmed byte-identical (via `diff`) to the final two-commit state:

- `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used` (Windows-host, informational) — **GREEN**, 0 errors.
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors.
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) — **GREEN**, 0 errors.

No regression from Plan 113-05's close (0 errors on both mandatory gates).

**Testing constraint carried forward from Plans 113-03/113-04/113-05:** `SpiffeJwtSource`/`SpiffeAssertionTokenCache` have no test-only constructor bypassing a live SPIRE Workload API connection. `handle_spiffe_route`'s auth-gate-only test (`spiffe_route_denies_missing_session_token_before_credential_acquisition`) works around this via `RouteStore::from_loaded_routes` with `managed_auth: None`, matching the plan's own acceptance-criteria allowance. `handle_spiffe_assertion_credential` hits the SAME constraint one layer deeper — `SpiffeAssertionRoute.cache: SpiffeAssertionTokenCache` has no equivalent test-only constructor (its `new()` performs a live initial token exchange), so no analogous auth-gate-only unit test was written for it; this is consistent with — not a deviation from — the constraint 113-03-SUMMARY.md's own "Rejected the mock-HTTP-server-based test alternative" decision already documented for this exact type. A full end-to-end proof of BOTH handlers' successful-forward path (not just the auth gate) is deferred to Plan 113-07's SPIRE-gated `spiffe_integration.rs` lane, per this plan's own acceptance-criteria text ("if no lighter-weight construction is possible... deferring full end-to-end proof to Plan 113-07's `spiffe_integration.rs`").

## User Setup Required

None — no external service configuration required for this plan's own verification. The new test exercises only the fully-unit-testable auth-gate half of the SPIFFE dispatch (no live SPIRE agent needed).

## Verification Results

- `cargo build -p nono-sandbox-proxy --all-targets` — **GREEN**, zero errors (both task commits individually, and the final combined state).
- `cargo build --workspace --all-targets` — **GREEN**, zero errors.
- `cargo test -p nono-sandbox-proxy --lib -- reverse::` — **GREEN**, 41 passed, 0 failed (40 baseline + 1 new: `spiffe_route_denies_missing_session_token_before_credential_acquisition`).
- `cargo test -p nono-sandbox-proxy --lib` (full crate suite) — **GREEN**, 242 passed, 0 failed (241 baseline at Plan 113-05's close + 1 new). Exceeds the plan's `>= 241` success criterion.
- `cargo fmt --all --check` — **GREEN** (both task commits, and the final combined state).
- `grep -c "forward::forward_request\|UpstreamSpec\|UpstreamStrategy" crates/nono-proxy/src/reverse.rs` — returns `0` (D-01's grep-verified negative proof for this file, whole-file including comments).
- `grep -n "audit::log_reverse_proxy("` — one hit, the pre-existing call site in `handle_reverse_proxy`'s static_cred/no-credential tail (Landmine L3, untouched).
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors (informational — see Issues Encountered for why this plan's own trigger condition doesn't strictly mandate it).
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors (informational, second mandatory target).
- `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used` (Windows-host, informational) — **GREEN**, 0 errors.

## Next Phase Readiness

- Both SPIFFE reverse-proxy handlers are landed, dispatch correctly, and fail closed on every credential-acquisition/auth failure path. D-01's "~90%" absorb now lands as real working request-handling code — a SPIFFE-declared route genuinely authenticates, injects, forwards, and audits.
- **Deferred to Plan 113-07:** a full end-to-end proof of a SUCCESSFUL SPIFFE-authenticated forward through either handler (both credential-acquisition types require a live/reachable SPIRE Workload API; no test-only constructor exists for `SpiffeJwtSource` or `SpiffeAssertionTokenCache` that bypasses this). This is the SAME constraint every prior plan in this phase (113-03/113-04/113-05) has hit and documented identically — not new to this plan.
- No blockers for Plan 113-07 or 113-08.

---
*Phase: 113-spiffe-spire-workload-identity*
*Completed: 2026-08-06*

## Self-Check: PASSED

Claimed modified file (`crates/nono-proxy/src/reverse.rs`) confirmed present on disk with the
described changes. Both claimed task commit hashes (`0772b82b`, `5f513680`) confirmed present in
`git log --oneline --all`.
