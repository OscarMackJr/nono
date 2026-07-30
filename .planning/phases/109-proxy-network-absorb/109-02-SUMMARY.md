---
phase: 109-proxy-network-absorb
plan: 02
subsystem: proxy-network
tags: [no-proxy, ssrf, host-filter, proxy-config, adr-108, fail-closed, route-store]

# Dependency graph
requires:
  - phase: 109-proxy-network-absorb
    plan: 01
    provides: "deny_domain HostFilter/ProxyFilter/ProxyConfig deny mechanism (denied_hosts, with_denied_hosts) this plan's filter.rs metadata check and config.rs no_proxy validators build alongside"
provides:
  - "ProxyConfig.no_proxy field + the three D-06-named validators (validate_no_proxy_entry, no_proxy_entry_overlaps_host_pattern, bare_single_label_suffix_overlaps_host) — the proxy-crate half of #1415"
  - "RouteStore::load fail-closed on unparseable/unsupported route upstreams (extract_host_port: Option<String> -> Result<String, String>)"
  - "ProxyFilter metadata-literal deny short-circuit closing an AWS IMDSv6 (fd00:ec2::254) gap in HostFilter's link-local check"
  - "server.rs push_no_proxy_entry/push_canonical_no_proxy_entry pipeline (managed_loopback_upstream, canonical_no_proxy_hosts, NONO_NO_PROXY) replacing the static no_proxy_parts vec, with validate_no_proxy_config/validate_no_proxy_route_conflicts wired fail-closed into start()"
affects: [109-03, 109-04, 109-05, 112, 113]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "no_proxy validators live in nono-proxy (library-adjacent, policy-carrying) because the crate owns proxy env generation and route protection; CLI profile validation (109-03) calls the same functions for parse-time UX"
    - "Smart-derived (direct-connect-port) NO_PROXY candidates run through the same validate_no_proxy_entry grammar as operator-declared entries, not a separate ad-hoc check"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/filter.rs
    - crates/nono-proxy/src/server.rs

key-decisions:
  - "managed_loopback_upstream is added to ProxyHandle (matching upstream's shape and env_vars() semantics) but start() always sets it false: this fork has not absorbed the loopback-credential-route detection mechanism that would compute it (that logic lives in upstream code this fork's ProxyHandle never had, and 109-CONTEXT.md's D-06/D-07 scope is the no_proxy bypass surface, not loopback-credential-route detection). The field exists so a future absorb can wire real detection without another env_vars() rewrite, and so the D-07 'must not regress' test for the true=>clears-loopback case has something real to assert against."
  - "Task 1's filter.rs change (ProxyFilter metadata-literal deny short-circuit) is in scope even though it isn't named in the plan's must_haves artifacts, because it is part of the same upstream commit (1619275c) and closes a real SSRF gap: fork's HostFilter link-local check does not cover fd00::/8 (AWS IMDSv6 unique-local address), so a no_proxy entry rejected at validation time for overlapping that metadata IP could otherwise still reach the proxy filter through a different code path. D-06 requires validation-time and proxy-time 'always denied' semantics to agree."
  - "smart_no_proxy_entry ported verbatim from upstream tightens smart-derived (direct-connect-port) NO_PROXY entries to reject bare multi-label domains (e.g. github.com) as ambiguous — an intentional #1415 change, not a fork-local choice. The pre-existing test test_no_proxy_includes_hosts_with_matching_connect_port asserted the old bare-domain-included behavior; updated to use an IP literal so it still proves the underlying 'matching connect port bypasses the proxy' mechanism."

patterns-established:
  - "D-06: no_proxy is a declared bypass surface — every entry point that builds a NO_PROXY value (smart-derived or profile-declared) runs through validate_no_proxy_entry before being emitted, and validate_no_proxy_config/validate_no_proxy_route_conflicts run before ProxyHandle construction so an invalid or overlapping entry fails proxy startup instead of degrading silently."

requirements-completed: [NET-03]

# Metrics
duration: ~55min
completed: 2026-07-29
---

# Phase 109 Plan 02: no_proxy config surface + D-06/D-07 pipeline absorb Summary

**Absorbed the proxy-crate half of upstream `1619275c` (#1415) — a validated `no_proxy` bypass surface with three non-negotiable D-06 overlap/grammar validators, plus a push-based NO_PROXY/NONO_NO_PROXY pipeline in `server.rs` proven by a real test not to have dropped the fork's pre-existing `localhost`/`127.0.0.1` loopback bypass (D-07).**

## Performance

- **Duration:** ~55 min
- **Completed:** 2026-07-29
- **Tasks:** 2/2 completed
- **Files modified:** 4 (`config.rs`, `route.rs`, `filter.rs`, `server.rs`) + this SUMMARY

## Accomplishments

- `ProxyConfig.no_proxy: Vec<String>` field plus the full D-06 validator set ported from `1619275c`: `validate_no_proxy_entry` (rejects URL credentials, ports, paths, comma lists, catch-all `*`; accepts single-label aliases, IP literals, `*.`/`.`-prefixed suffixes), `no_proxy_entry_overlaps_host_pattern`, and the private `bare_single_label_suffix_overlaps_host` maintainer-feedback guard (a bare `"internal"` entry now correctly overlaps `metadata.google.internal`).
- `RouteStore::load` fails closed on an unparseable or unsupported route upstream: `extract_host_port` changed from `Option<String>` to `Result<String, String>`, so a malformed upstream can no longer be silently invisible to no_proxy/route-conflict checks.
- `ProxyFilter` gained a metadata-literal deny short-circuit (`proxy_metadata_filter_result`) that closes an AWS IMDSv6 (`fd00:ec2::254`) gap in the fork's `HostFilter` link-local check, keeping no_proxy-validation-time and proxy-time "always denied" semantics in agreement.
- `server.rs`'s static `vec!["localhost", "127.0.0.1"]` NO_PROXY seed is fully replaced by the `push_no_proxy_entry`/`push_canonical_no_proxy_entry` pipeline (`ProxyHandle.managed_loopback_upstream`, `canonical_no_proxy_hosts`, `NONO_NO_PROXY` env var), with `validate_no_proxy_config` and `validate_no_proxy_route_conflicts` wired fail-closed before any listener bind or `ProxyHandle` construction.
- D-07 regression proof landed as a real test: `no_proxy_pipeline_preserves_localhost_and_loopback_bypass` asserts both `localhost` and `127.0.0.1` still reach the child's `NO_PROXY` after the pipeline replacement.

## Task Commits

Each task was committed atomically:

1. **Task 1: no_proxy config surface + validators (D-06)** - `82b0b6a0` (feat)
2. **Task 2: Replace no_proxy_hosts pipeline in server.rs + D-07 regression test** - `fcbd8ab3` (feat)

_No plan-metadata-only commit yet; this SUMMARY + STATE/ROADMAP updates land in the orchestrator's final commit (STATE.md/ROADMAP.md/REQUIREMENTS.md are hand-tracked for this milestone per the execution prompt and are out of scope for this executor)._

## Files Created/Modified

- `crates/nono-proxy/src/config.rs` — `ProxyConfig.no_proxy` field; `validate_no_proxy_entry`, `no_proxy_entry_overlaps_host_pattern`, `bare_single_label_suffix_overlaps_host`, and their private helpers (`strip_no_proxy_port`, `normalise_no_proxy_host_pattern`, `normalise_no_proxy_env_entry`, `ALWAYS_DENIED_HOSTS`, `is_proxy_denied_metadata_ip`, `parse_host_ip_literal`); 6 new unit tests.
- `crates/nono-proxy/src/route.rs` — `extract_host_port` now returns `Result<String, String>`; `RouteStore::load` propagates the error via `ProxyError::Config`; new `test_load_routes_rejects_malformed_or_unsupported_upstreams`.
- `crates/nono-proxy/src/filter.rs` — `proxy_metadata_filter_result` short-circuit in both `check_host` and `check_host_with_ips`; 2 new tests for AWS IPv6 metadata literal/resolved-IP denial.
- `crates/nono-proxy/src/server.rs` — `ProxyHandle.managed_loopback_upstream`/`canonical_no_proxy_hosts` fields; `push_no_proxy_entry`/`push_canonical_no_proxy_entry`/`canonical_no_proxy_entry`/`smart_no_proxy_entry`/`merge_no_proxy_hosts`/`merge_canonical_no_proxy_hosts`/`validate_no_proxy_config`/`validate_no_proxy_route_conflicts`/`validate_no_proxy_allowed_host_conflicts`/`no_proxy_entry_matches_any_route`/`no_proxy_entry_matches_route`/`route_host_from_host_port` free functions; `start()` wiring; 9 new tests (D-07 regression + 3 fail-closed rejection paths + smart-entry grammar + pipeline behavior coverage); 1 pre-existing test updated for the intentional bare-domain-filtering behavior change.

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. `managed_loopback_upstream` added as inert scaffolding (always `false` in `start()`) rather than either omitting the field (which would break upstream-shape parity and the D-07 "must not regress" test's negative case) or inventing loopback-credential-route detection logic (out of scope — Rule 4 territory, not requested by this plan).
2. `filter.rs`'s metadata-literal deny check is in scope as part of the same upstream commit closing a real SSRF gap, even though it isn't named in the plan's `must_haves.artifacts`.
3. `smart_no_proxy_entry`'s bare-multi-label-domain filtering is an intentional upstream behavior change absorbed as-is; the one pre-existing fork test that encoded the old behavior was updated to use an IP literal instead of removing coverage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `let`-chains not available in this fork's Rust edition**
- **Found during:** Task 1, first `cargo build -p nono-sandbox-proxy`
- **Issue:** The upstream `strip_no_proxy_port` body used a `let`-chain (`if let Some(...) = ... && let Some(...) = ... && ...`), which requires Rust 2024; this fork is Edition 2021 (CLAUDE.md: Rust 1.82, Edition 2021).
- **Fix:** Rewrote as nested `if let` blocks with identical semantics.
- **Files modified:** `crates/nono-proxy/src/config.rs`
- **Verification:** `cargo build -p nono-sandbox-proxy --all-targets` exits 0; all `strip_no_proxy_port`-dependent tests pass.
- **Commit:** `82b0b6a0`

**2. [Rule 1 - Bug] Existing test encoded the old (now-superseded) smart-NO_PROXY bare-domain behavior**
- **Found during:** Task 2, `cargo test -p nono-sandbox-proxy`
- **Issue:** `test_no_proxy_includes_hosts_with_matching_connect_port` asserted that a bare domain (`github.com`) with a matching `direct_connect_ports` entry appears in `NO_PROXY`. Upstream's own `1619275c` (`smart_no_proxy_entry`, verified against upstream's `test_smart_no_proxy_entry_filters_ambiguous_bare_domains`) intentionally changed this: smart-derived candidates now go through `validate_no_proxy_entry`, which rejects bare multi-label domains as ambiguous. This is the absorbed commit's own behavior, not a fork-introduced regression.
- **Fix:** Updated the test to use an IP literal (`203.0.113.5`) for the positive case, preserving coverage of the "matching connect port bypasses the proxy" mechanism, and added `test_smart_no_proxy_entry_filters_ambiguous_bare_domains` (ported verbatim from upstream's test vectors) as direct grammar coverage.
- **Files modified:** `crates/nono-proxy/src/server.rs`
- **Verification:** `cargo test -p nono-sandbox-proxy` — 209 passed, 0 failed.
- **Commit:** `fcbd8ab3`

---

**Total deviations:** 2 auto-fixed (1 blocking/toolchain, 1 bug/stale-test-vs-intentional-upstream-behavior-change)
**Impact on plan:** Both fixes were necessary to compile and to correctly reflect the absorbed commit's own intended semantics. No scope creep — no functionality was added beyond what `1619275c`'s proxy-crate half specifies.

## Issues Encountered

The plan's `<interfaces>` block described `managed_loopback_upstream` as "EXISTING — unchanged" in the fork's current `ProxyHandle`. It is not present in the fork prior to this plan (confirmed via `grep -rn "managed_loopback_upstream"` returning no hits, and via `git log -p -S managed_loopback_upstream -- crates/nono-proxy/src/server.rs` showing it as pre-existing *upstream* context in the `1619275c` diff, not a fork field). This is a plan-authoring inaccuracy, not a code discrepancy to fix — handled per the `key-decisions` entry above (field added as inert scaffolding, always `false`).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The proxy-crate half of `#1415` (no_proxy mechanism + D-06 validators + D-07 regression proof) is complete and tested. Plan 109-03 can now build the CLI-crate half (`validate_profile_no_proxy`, `validate_proxy_launch_no_proxy_conflicts`, `validate_expanded_proxy_no_proxy_conflicts`, profile schema key, `--no-proxy` flag threading) on top of the `ProxyConfig.no_proxy` field and the `no_proxy_entry_overlaps_host_pattern`/`validate_no_proxy_entry` functions this plan exposed as `pub`.
- `RouteStore::load`'s new fail-closed `Result` signature is confirmed compatible with the rest of the workspace (`cargo build --workspace --all-targets` exits 0) — no `nono-cli` call site needed updating, since the fork never called `extract_host_port` outside `route.rs`.
- No blockers.

## Known Stubs

None. `no_proxy` config validation, route/allowed-host conflict rejection, and the NO_PROXY/NONO_NO_PROXY env pipeline are fully wired end-to-end within the proxy crate; no placeholder or empty-value stub was introduced. (The CLI-side profile/flag wiring is intentionally out of scope for this plan — see 109-03.)

## Threat Flags

None. The security-relevant surface introduced here (`ProxyConfig.no_proxy`, the three D-06 validators, the metadata-literal deny short-circuit, `validate_no_proxy_config`/`validate_no_proxy_route_conflicts`) is explicitly covered by the plan's own `<threat_model>` (T-109-05, T-109-06, T-109-07, T-109-SC). No new network endpoints, auth paths, or schema changes at trust boundaries beyond what the threat model already names.

## Self-Check: PASSED

- FOUND: `crates/nono-proxy/src/config.rs` (no_proxy field + 3 validators present)
- FOUND: `crates/nono-proxy/src/route.rs` (extract_host_port returns Result)
- FOUND: `crates/nono-proxy/src/filter.rs` (proxy_metadata_filter_result)
- FOUND: `crates/nono-proxy/src/server.rs` (push_no_proxy_entry pipeline, no static `no_proxy_parts = vec!["localhost"...` remaining)
- FOUND commit: `82b0b6a0` (Task 1)
- FOUND commit: `fcbd8ab3` (Task 2)
- `cargo test -p nono-sandbox-proxy`: 209 passed, 0 failed
- `cargo build --workspace --all-targets`: exit 0
- `cargo fmt --all -- --check`: clean

---
*Phase: 109-proxy-network-absorb*
*Completed: 2026-07-29*
