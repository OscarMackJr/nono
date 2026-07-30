---
phase: 109-proxy-network-absorb
plan: 04
subsystem: proxy-network
tags: [http-proxy, forward-proxy, connect, ssrf, host-filter, audit, adr-108]

# Dependency graph
requires:
  - phase: 109-proxy-network-absorb
    plan: 02
    provides: "no_proxy_hosts pipeline rewrite in server.rs (managed_loopback_upstream, canonical_no_proxy_hosts, metadata-literal deny short-circuit) — this plan extends the same server.rs, on top of that rewrite"
provides:
  - "classify_request_target/RequestTargetForm — dispatches non-CONNECT requests by request-target form (absolute-form http/https vs origin-form), inserted BEFORE the reverse-proxy route_store branch so absolute URLs never reach parse_service_prefix (#1334)"
  - "handle_forward_http — plain-HTTP forward-proxy serving path: Proxy-Authorization gate (407), check_host trust boundary (403 + HostDenied audit), origin-form rewrite + hop-by-hop header strip, direct connect to check_host's DNS-rebinding-safe resolved addresses, response streaming with L7 audit (status + managed_credential_active=false)"
  - "absolute-form https:// rejected with explicit 400 + CONNECT guidance (not a confusing 502 or silent cleartext downgrade)"
  - "reverse::extract_content_length / reverse::connect_to_resolved / reverse::parse_response_status widened to pub(crate) so the forward-http path reuses the reverse-proxy path's own primitives instead of duplicating them"
  - "audit::log_l7_request + audit::L7RequestInfo — new audit emitter recording target/port/method/path/status together with managed_credential_active, used to prove the forward path's transparent-pass-through-never-injects-credential contract"
affects: [112, 113]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "New serving paths (forward-http) reuse existing reverse-proxy connect/parse primitives via pub(crate) widening rather than introducing a parallel forward.rs abstraction module upstream has but this fork does not"
    - "Audit emitters that need to positively assert a security property (managed_credential_active=false) get a dedicated emitter rather than overloading log_allowed/log_reverse_proxy, whose shapes don't carry that combination of fields"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/server.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/audit.rs

key-decisions:
  - "Upstream 726ac1f1's forward.rs/AuditCtx/UpstreamScheme/UpstreamSpec/UpstreamStrategy abstraction does not exist in this fork (verified absent via grep before implementing — the same D-11 class of miss the plan's own execution_notes flagged for this milestone's <interfaces> blocks three times already). handle_forward_http is implemented directly against this fork's existing primitives instead: reverse.rs's extract_content_length/connect_to_resolved/parse_response_status (widened to pub(crate) and reused, not duplicated), and a new audit::log_l7_request emitter. Also absent and added in this plan: parse_non_connect_target (host:port parsing from an absolute-form request line) did not exist in the fork prior to this plan — no prior caller needed it, since absolute-form requests previously fell through to a flat 400."
  - "External-proxy (enterprise-proxy) chaining is explicitly OUT OF SCOPE for handle_forward_http. Upstream's forward_request chains through an external proxy via the missing forward.rs abstraction; this fork's only external-proxy-chaining code (external::handle_external_proxy) is CONNECT-tunnel-shaped (parses a bare host:port authority, not a GET with headers+body) and is not reusable here without inventing a second abstraction — an architectural decision outside this plan's scope. handle_forward_http connects only directly to check_host's resolved addresses. This does not weaken the plan's required security contract: the trust boundary (check_host) and its DNS-rebinding-safe resolved-address connect are both fully implemented; none of the plan's 9 named tests exercise external-proxy chaining."
  - "audit::log_l7_request takes an audit::L7RequestInfo<'_> struct (host/port/method/path/status) rather than 5 discrete parameters, to stay under clippy::too_many_arguments (max 7; a straightforward 8-parameter port failed -D warnings)."

patterns-established:
  - "When absorbing an upstream feature whose implementation depends on a module/abstraction this fork never absorbed, verify absence first (grep, not assumption), then re-target the port onto the fork's nearest equivalent primitives rather than either inventing the missing abstraction wholesale (Rule 4 territory) or blocking on it — document the scope reduction explicitly."

requirements-completed: [NET-03]

# Metrics
duration: ~70min
completed: 2026-07-29
---

# Phase 109 Plan 04: HTTP_PROXY plain-HTTP forward-proxying absorb Summary

**Absorbed upstream `726ac1f1` (#1335) — a new plain-HTTP forward-proxy serving path in `crates/nono-proxy/src/server.rs`, dispatched before the reverse-proxy route to prevent absolute-form URLs being misparsed as service prefixes (#1334), enforcing the same `check_host` trust boundary the CONNECT tunnel uses and never injecting managed credentials.**

## Performance

- **Duration:** ~70 min
- **Completed:** 2026-07-29
- **Tasks:** 2/2 completed
- **Files modified:** 3 (`server.rs`, `reverse.rs`, `audit.rs`) + this SUMMARY

## Accomplishments

- `RequestTargetForm`/`classify_request_target` discriminate non-CONNECT request lines by request-target form (absolute-form `http://`, absolute-form `https://`, or origin-form), inserted into `handle_connection`'s dispatch chain BEFORE the `!state.route_store.is_empty()` reverse-proxy branch — the exact #1334 ordering fix, so an absolute-form URL can never be misread as a reverse-proxy service prefix.
- `handle_forward_http` implements the full 3-step forward-proxy trust boundary: `Proxy-Authorization` gate (407 + `AuthenticationFailed` audit on failure), `filter.check_host` (403 + `HostDenied` audit on deny — the SAME trust boundary the CONNECT tunnel uses, no new bypass path), then request-line rewrite to origin-form + hop-by-hop header stripping (`Proxy-Connection`/`Proxy-Authorization` only; `Host` and all other headers pass through verbatim) and a direct connect to the DNS-rebinding-safe resolved addresses `check_host` returned (never re-resolving the hostname).
- Absolute-form `https://` is rejected with an explicit `400` and the exact upstream guidance text (`"https forward-proxying is not supported; use CONNECT for https"`) rather than a confusing `502` or a silent cleartext downgrade.
- The forward path is proven transparent: `forward_http_does_not_inject_managed_credential` configures a route with an unresolvable credential key and shows the forward-proxied request still succeeds with the client's own `Authorization` header intact and `managed_credential_active = Some(false)` in the audit event — the forward path never consults routes or credentials.
- All 9 upstream-named tests ported and passing, including the three explicitly called out as the security contract: `forward_http_denied_host_returns_403_and_audits`, `forward_http_does_not_inject_managed_credential`, `forward_https_absolute_form_is_rejected_with_connect_guidance`.
- `reverse::extract_content_length`, `reverse::connect_to_resolved`, and `reverse::parse_response_status` widened from private to `pub(crate)` so `handle_forward_http` reuses the reverse-proxy path's own primitives instead of duplicating ~60 lines of body-reading/connect/status-parsing logic.

## Task Commits

Each task was committed atomically:

1. **Task 1: classify_request_target + rewrite/strip helpers + handle_forward_http** - `2b855f94` (feat)
2. **Task 2: Wire into handle_connection dispatch + integration tests** - `9fd65803` (feat)

_No plan-metadata-only commit yet; this SUMMARY + STATE/ROADMAP updates land in the orchestrator's final commit (STATE.md/ROADMAP.md/REQUIREMENTS.md are hand-tracked for this milestone per the execution prompt and are out of scope for this executor)._

## Files Created/Modified

- `crates/nono-proxy/src/server.rs` — `parse_non_connect_target`, `RequestTargetForm`, `classify_request_target`, `rewrite_absolute_to_origin_form`, `strip_proxy_headers`, `handle_forward_http`, `MAX_FORWARD_BODY`; two new dispatch branches in `handle_connection`; 9 new tests (`classify_request_target_detects_absolute_and_origin_forms`, `rewrite_absolute_to_origin_form_produces_origin_line`, `strip_proxy_headers_removes_proxy_hop_by_hop_only`, `spawn_echo_origin` test fixture, `forward_http_allowed_host_is_forwarded`, `forward_http_does_not_inject_managed_credential`, `forward_http_denied_host_returns_403_and_audits`, `forward_https_absolute_form_is_rejected_with_connect_guidance`, `forward_http_missing_proxy_auth_is_rejected_407`, `origin_form_request_still_routes_to_reverse_proxy`).
- `crates/nono-proxy/src/reverse.rs` — `extract_content_length`, `connect_to_resolved`, `parse_response_status` widened `fn` -> `pub(crate) fn` (doc comments added noting the forward-http reuse; no behavior change).
- `crates/nono-proxy/src/audit.rs` — new `L7RequestInfo<'a>` struct and `log_l7_request` emitter (records `target`/`port`/`method`/`path`/`status` together with `managed_credential_active`, which no existing emitter does).

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. Implemented `handle_forward_http` against this fork's own `reverse.rs` primitives (widened to `pub(crate)`) instead of porting upstream's separate `forward.rs`/`UpstreamSpec`/`UpstreamStrategy`/`AuditCtx` abstraction, which does not exist in this fork.
2. External-proxy chaining is out of scope for the forward-http path specifically — deferred as a documented gap, not silently dropped; the plan's required security contract (host-filter trust boundary, DNS-rebinding-safe connect) is fully implemented and none of the 9 required tests exercise it.
3. `audit::log_l7_request` takes a struct parameter to satisfy `clippy::too_many_arguments`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `parse_non_connect_target` did not exist in the fork**
- **Found during:** Task 1, while implementing `handle_forward_http`'s auth-gate and host-parse steps
- **Issue:** The plan's `<interfaces>` block implies this function is pre-existing fork context (it appears as unmodified diff-hunk context in the upstream commit). `grep -rn "parse_non_connect_target"` returned zero hits in the fork prior to this plan — no prior caller needed it, since absolute-form requests previously fell straight through to a flat `HTTP/1.1 400 Bad Request` in the final `else` arm.
- **Fix:** Ported the function verbatim from upstream `726ac1f1`'s copy of `server.rs` (confirmed via `git show 726ac1f1:crates/nono-proxy/src/server.rs`).
- **Files modified:** `crates/nono-proxy/src/server.rs`
- **Verification:** Used by `handle_forward_http`'s auth-gate audit fallback and host-filter step; exercised by all 6 integration tests.
- **Commit:** `2b855f94`

**2. [Rule 3 - Blocking] No `forward.rs`/`UpstreamSpec`/`UpstreamStrategy`/`AuditCtx` module exists in this fork**
- **Found during:** Task 1, before writing `handle_forward_http`'s upstream-connect step
- **Issue:** Upstream's `handle_forward_http` imports `crate::forward::{self, AuditCtx, UpstreamScheme, UpstreamSpec, UpstreamStrategy}` and calls `forward::forward_request(...)`. `ls crates/nono-proxy/src/` confirms no `forward.rs` file exists in this fork. Inventing that abstraction wholesale would be a Rule 4 (architectural) decision outside this plan's scope.
- **Fix:** Implemented the connect/forward/audit steps directly in `handle_forward_http` using this fork's existing `reverse.rs` primitives (`connect_to_resolved`, `extract_content_length`, `parse_response_status`, widened to `pub(crate)`) plus a new `audit::log_l7_request` emitter. External-proxy chaining (the one piece of upstream's `forward_request` this doesn't replicate) is explicitly out of scope — see key-decisions.
- **Files modified:** `crates/nono-proxy/src/server.rs`, `crates/nono-proxy/src/reverse.rs`, `crates/nono-proxy/src/audit.rs`
- **Verification:** `cargo test -p nono-sandbox-proxy` — 218 passed, 0 failed, including all 9 named tests.
- **Commit:** `2b855f94`

**3. [Rule 1 - Bug] `origin_form_request_still_routes_to_reverse_proxy` asserted upstream's 503 behavior, which this fork does not exhibit**
- **Found during:** Task 2, `cargo test -p nono-sandbox-proxy` after wiring the dispatch
- **Issue:** The ported test asserted `HTTP/1.1 503` for a route whose `credential_key` fails to resolve. This fork's `CredentialStore` resolves credentials once at startup and silently excludes routes whose credential could not load (`ProxyHandle.loaded_routes`, already documented in this file from an earlier absorb) rather than returning a request-time 503 — so the route falls back to `handle_reverse_proxy`'s no-credential branch (session-token-only L7 auth via `Proxy-Authorization`), which the test's request omits, yielding `407` instead.
- **Fix:** Changed the status assertion to `407` and strengthened the audit assertion to check `route_id.as_deref() == Some("openai")` in addition to `mode == Reverse` — this positively proves the request reached `handle_reverse_proxy` specifically (only the reverse handler's audit calls ever set `route_id`; `handle_forward_http` never does), which is the actual regression this test guards (not the specific status code, which is incidental to upstream's different `CredentialStore` design).
- **Files modified:** `crates/nono-proxy/src/server.rs`
- **Verification:** `cargo test -p nono-sandbox-proxy` — 218 passed, 0 failed.
- **Commit:** `9fd65803`

---

**Total deviations:** 3 auto-fixed (2 blocking/missing-fork-machinery, 1 bug/stale-test-vs-fork-credential-store-behavior)
**Impact on plan:** All three were necessary to compile and correctly reflect this fork's actual architecture. No security scope was reduced: the host-filter trust boundary, credential non-injection, and denied-host audit contract — the plan's three named security-contract tests — are all implemented and passing exactly as specified. The one scope reduction (external-proxy chaining) is documented and does not affect any required behavior test.

## Issues Encountered

Same D-11-class miss (plan `<interfaces>` describing upstream-only symbols as fork-present) that 109-CONTEXT.md's `<specifics>` section already documented three times for this milestone — this plan is a fourth and fifth instance (`parse_non_connect_target`, the `forward.rs` module family), both handled per Rule 3 (port/adapt, don't block) rather than requiring a stop, since neither involved an architectural decision about this plan's own required behavior (only about how much of upstream's optional external-proxy-chaining machinery to replicate).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The forward-http serving path is complete, tested, and wired into `handle_connection`'s dispatch ahead of the reverse-proxy branch. `crates/nono-proxy/src/server.rs`'s `handle_forward_http`, `classify_request_target`, `rewrite_absolute_to_origin_form`, and `strip_proxy_headers` are all `pub`-visibility-appropriate (module-private, matching the rest of `server.rs`'s helper functions) and available for reuse by future proxy work.
- `reverse::extract_content_length`/`connect_to_resolved`/`parse_response_status` are now `pub(crate)` and available to any future proxy-crate module that needs the same primitives (e.g. Phase 112's async `load_with_diagnostics` refactor, which touches `server.rs` per 109-CONTEXT.md D-03).
- External-proxy chaining for the forward-http path remains a known, documented gap — a future plan wanting that capability will need to either extend `external::handle_external_proxy` to support a GET-with-body shape, or introduce a small shared abstraction (Rule 4 territory, not attempted here).
- No blockers.

## Known Stubs

None. `handle_forward_http` is fully wired end-to-end (auth gate, host filter, request forwarding, response streaming, audit) — the only intentionally-unimplemented piece (external-proxy chaining) is documented above as a scope decision, not a stub in the plan's own required behavior.

## Threat Flags

None beyond what the plan's own `<threat_model>` already names (T-109-10 through T-109-13, T-109-SC). This plan introduces a new serving path (forward-proxy) exactly as scoped by the plan's threat register; no additional network endpoints, auth paths, or schema changes at trust boundaries beyond what T-109-10..13 already cover. The `audit::log_l7_request`/`L7RequestInfo` addition is an audit-emission mechanism, not new attack surface.

## Self-Check: PASSED

- FOUND: `crates/nono-proxy/src/server.rs` (classify_request_target, handle_forward_http, dispatch branches present)
- FOUND: `crates/nono-proxy/src/reverse.rs` (extract_content_length/connect_to_resolved/parse_response_status now pub(crate))
- FOUND: `crates/nono-proxy/src/audit.rs` (log_l7_request, L7RequestInfo present)
- FOUND commit: `2b855f94` (Task 1)
- FOUND commit: `9fd65803` (Task 2)
- `cargo test -p nono-sandbox-proxy`: 218 passed, 0 failed (209 baseline + 9 new)
- `cargo test -p nono-sandbox-cli --bin nono`: 1411 passed, 11 failed (matches documented baseline exactly)
- `cargo build --workspace --all-targets`: exit 0
- `cargo clippy -p nono-sandbox-proxy --all-targets -- -D warnings -D clippy::unwrap_used`: clean
- `cargo fmt --all -- --check`: clean

---
*Phase: 109-proxy-network-absorb*
*Completed: 2026-07-29*
