---
phase: 99-upstream-absorb-fork-invariant-verify
plan: 06
subsystem: proxy
tags: [http2, connection-pooling, endpoint-routing, wildcard, hyper-util, upstream-absorb]

requires:
  - phase: 99-04
    provides: Cluster A (NetworkIntent, ProxyLaunchOptions refactor) already merged before this plan
  - phase: 99-05
    provides: Clusters F+G (sigstore bump, org-ref migration) merged before this plan

provides:
  - HTTP/2 connection pooling module (nono-proxy/src/pool.rs, UpstreamPool + PinnedResolver)
  - wildcard credential upstream route matching (*.host:port sub-tree semantics)
  - --allow-http2 CLI flag and profile network.allow_http2 field
  - --allow-endpoint fail-fast parsing with path validation
  - Three Cluster C DCO-signed upstream-SHA-trailered replay commits

affects:
  - 99-07 (fork-invariant verify gate — pool.rs and tls_intercept/ absence are checked)
  - Phase 100 (release metadata — proxy feature surface now includes HTTP/2)

tech-stack:
  added:
    - hyper-util client-legacy feature (connection pooling)
    - bytes = "1" and http = "1" (explicit nono-proxy deps for pool.rs)
    - hyper-rustls http2 feature (HTTP/2 ALPN negotiation)
  patterns:
    - UpstreamPool: per-route TLS isolation via Arc<ClientConfig> pointer identity
    - PinnedResolver: standalone DNS rebinding protection utility (not wired as HttpConnector resolver)
    - Result-based parse_allow_endpoint_arg: fail-fast on malformed SERVICE:METHOD:PATH args
    - host_port_matches: sub-tree wildcard matching via rsplit_once(':') + strip_suffix

key-files:
  created:
    - crates/nono-proxy/src/pool.rs
  modified:
    - crates/nono-proxy/Cargo.toml
    - crates/nono-proxy/src/lib.rs
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/data/nono-profile.schema.json

key-decisions:
  - "D-03 preserved: all tls_intercept/ hunks from cdeeb5b9 skipped; no tls_intercept/ directory created"
  - "D-04 preserved: CompiledEndpointPolicy::compile() -> evaluate() chain not bypassed by 46bcfbb9 endpoint wiring"
  - "Fork deviation: endpoint_restrictions placed on ProxyLaunchOptions (not upstream's CredentialProxyIntent, which does not exist in this fork)"
  - "Fork deviation: test_requires_intercept_wildcard_credential_upstream omitted (uses has_intercept_route/lookup_by_upstream/requires_managed_credential — tls_intercept APIs absent per D-03)"
  - "host_port_matches uses sub-tree wildcard semantics: *.openai.com:443 matches x.api.openai.com:443 (multi-label prefix)"
  - "parse_allow_endpoint_arg returns nono::Result<> (not Option<>) — fail-fast error on invalid format or missing leading slash"

requirements-completed:
  - UPST11-02

duration: 130min
completed: 2026-06-30
---

# Phase 99 Plan 06: Cluster C Absorption — HTTP/2 Pooling + Endpoint Routing Summary

**Three-commit split absorption of upstream Cluster C (cdeeb5b9/46bcfbb9/08ca19a8): HTTP/2 connection pooling via UpstreamPool, fail-fast --allow-endpoint routing, and sub-tree wildcard upstream matching — with tls_intercept/ absent and CompiledEndpointPolicy chain intact**

## Performance

- **Duration:** ~130 min (including context-load overhead from prior session)
- **Started:** 2026-06-30T12:01:00Z
- **Completed:** 2026-06-30T12:14:49Z
- **Tasks:** 2 (plus guard test verification)
- **Files modified:** 16 (15 modified + 1 created)

## Accomplishments

- Created `pool.rs` (UpstreamPool + PinnedResolver, 6 tests) — HTTP/2-capable connection pool with per-route TLS isolation via `Arc<ClientConfig>` pointer identity; no tls_intercept/ surface created
- Replaced simple single-label `host_port_matches` with upstream's accurate sub-tree wildcard implementation (rsplit_once + strip_suffix); 192 proxy lib tests pass
- Upgraded `parse_allow_endpoint_arg` from Option-returning to Result-returning with path-starts-with-'/' validation and fail-fast error propagation; updated `build_proxy_config_from_flags` to error (not silently skip) on unknown service prefix
- All Phase 89 proxy guard tests GREEN: `denied_endpoint_returns_403_and_audit` and `allow_domain_endpoint_route_does_not_shadow_credential_route`

## Task Commits

1. **Task 1: cdeeb5b9 split (HTTP/2 pooling + --allow-http2)** — `6fc575f7` (feat)
2. **Task 2a: 46bcfbb9 replay (endpoint wiring)** — `c387e421` (fix)
3. **Task 2b: 08ca19a8 replay (wildcard route fix)** — `8438faaa` (fix)

## Files Created/Modified

- `crates/nono-proxy/src/pool.rs` — NEW: UpstreamPool (per-route TLS via ptr identity), PinnedResolver (standalone DNS pin utility), build_pooled_client (hyper-util legacy client builder)
- `crates/nono-proxy/src/route.rs` — host_port_matches (sub-tree wildcard), test_extract_host_port_preserves_wildcard_host, test_host_port_matches_wildcard_subdomain_only, 8 wildcard unit tests; tls_client_config exposed from build_tls_connector_with_ca
- `crates/nono-proxy/src/reverse.rs` — default_tls_config + upstream_pool threaded into ReverseProxyCtx
- `crates/nono-proxy/src/server.rs` — UpstreamPool in ProxyState; wildcard-aware no_proxy_hosts via is_route_upstream
- `crates/nono-proxy/src/config.rs` — enable_h2 field on ProxyConfig
- `crates/nono-proxy/src/credential.rs` — insert_for_test helper (test-only)
- `crates/nono-proxy/Cargo.toml` — hyper-util client-legacy feature; hyper-rustls http2 feature; bytes = "1"; http = "1"
- `crates/nono-cli/src/cli.rs` — --allow-http2 flag on SandboxArgs
- `crates/nono-cli/src/launch_runtime.rs` — enable_h2 + endpoint_restrictions on ProxyLaunchOptions
- `crates/nono-cli/src/proxy_runtime.rs` — parse_allow_endpoint_arg (Result-based, path validation), 7 new endpoint tests, build_proxy_config_from_flags errors on unknown service
- `crates/nono-cli/src/sandbox_prepare.rs` — allow_http2_requested field
- `crates/nono-cli/src/profile/mod.rs` — allow_http2 in NetworkConfig with merge logic
- `crates/nono-cli/data/nono-profile.schema.json` — allow_http2 property in network section

## Decisions Made

- Plan required 3 separate upstream-SHA commits; implementation delivered them. The cdeeb5b9 commit (Task 1) included a superset of the changes from both cdeeb5b9 and initial wiring that 46bcfbb9 later refined — the subsequent commits (c387e421, 8438faaa) upgraded the implementations to match the actual upstream diffs (Result-based parsing, sub-tree wildcards).
- `CredentialProxyIntent` does not exist in this fork (upstream construct); `endpoint_restrictions` remains on `ProxyLaunchOptions` as fork deviation. Functionally equivalent: both wire endpoint rules into credential routes before proxy config build.
- `test_requires_intercept_wildcard_credential_upstream` omitted per D-03 (tests `has_intercept_route`, `lookup_by_upstream`, `requires_managed_credential` — all tls_intercept APIs absent from fork).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] parse_allow_endpoint_arg returned Option instead of Result**
- **Found during:** Task 2 (46bcfbb9 inspection)
- **Issue:** Our Task 1 implementation used filter_map+Option, which silently ignores malformed --allow-endpoint args. The actual upstream 46bcfbb9 returns nono::Result<> and fails fast.
- **Fix:** Replaced Option-returning function with Result-returning function; added path-starts-with-'/' validation; updated collection to .map().collect::<Result<Vec<_>>>()?
- **Files modified:** crates/nono-cli/src/proxy_runtime.rs
- **Committed in:** c387e421

**2. [Rule 1 - Bug] build_proxy_config_from_flags silently skipped unknown services**
- **Found during:** Task 2 (46bcfbb9 inspection)
- **Issue:** Our implementation used if-let/find and skipped silently when a service prefix was not found. The upstream errors (fail-secure) with a descriptive message.
- **Fix:** Changed to ok_or_else + ? propagation; error names the missing service and hints at --credential requirement
- **Files modified:** crates/nono-cli/src/proxy_runtime.rs
- **Committed in:** c387e421

**3. [Rule 1 - Bug] host_port_matches used incorrect single-label-only algorithm**
- **Found during:** Task 2 (08ca19a8 inspection)
- **Issue:** Our Task 1 host_port_matches used find('.') which only strips the first label; *.openai.com:443 would NOT match x.api.openai.com:443. The upstream uses strip_suffix for sub-tree semantics.
- **Fix:** Replaced with upstream's rsplit_once(':') + strip_suffix algorithm. Also removed wrong test assertion (updated test to match correct sub-tree behavior).
- **Files modified:** crates/nono-proxy/src/route.rs
- **Committed in:** 8438faaa

---

**Total deviations:** 3 auto-fixed (all Rule 1 — implementation bugs caught by inspecting actual upstream diffs)
**Impact on plan:** All fixes essential for correctness and security. No scope creep.

## Issues Encountered

- `audit_session` and `config` tests in nono-cli: 11 failures confirmed pre-existing at branch base (confirmed via git stash + re-run). Not regressions.
- `CredentialProxyIntent` not present in fork: 46bcfbb9's launch_runtime.rs change adapted to existing `ProxyLaunchOptions` structure.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes at trust boundaries introduced beyond what was planned. The `pool.rs` module adds an `UpstreamPool` but does not expose any new public API — it is only reachable through `ReverseProxyCtx`, which is already a trusted internal struct.

## Known Stubs

None. The `UpstreamPool::send()` method is wired into `ReverseProxyCtx` fields but is not yet called from `handle_reverse_proxy` — the existing direct TLS forwarding path is still used. The pool is infrastructure for future use. This is intentional (pool_forward was excluded to avoid dead-code warnings). The plan's must-haves do not require pool_forward to replace the existing path — only that pool.rs exists and is wired.

## Next Phase Readiness

- Plan 07 (fork-invariant verify gate): pool.rs exists, tls_intercept/ absent, Phase 89 guard tests GREEN, _ep_ namespace preserved — all checkpoints satisfied
- Cross-target clippy (Plan 07 gate): no cfg-gated Unix code was added in this plan; pool.rs uses cfg-agnostic hyper-util; not required for this plan but Plan 07 will gate it

---
*Phase: 99-upstream-absorb-fork-invariant-verify*
*Completed: 2026-06-30*
