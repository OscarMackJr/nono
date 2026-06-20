---
phase: 89-proxy-hardening-sync
plan: 01
subsystem: proxy
tags: [proxy, credentials, activation-predicate, regression-test, security, customCredentials]

# Dependency graph
requires:
  - phase: 88-feat-deps-wave
    provides: proxy_runtime.rs baseline including custom_credentials plumbing in ProxyLaunchOptions
provides:
  - D-07 fix: proxy now activates when only customCredentials is configured (#1197)
  - D-07 regression tests: proxy_activates_with_custom_credentials_only, block_net_overrides_custom_credentials_activation
  - D-01 equivalence test: build_proxy_config_maps_upstream_proxy_to_external_proxy (no cherry-pick needed)
affects: [89-04-ledger-addendum, phase-89-completion]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Activation predicate disjunction: add new condition to both ACTIVE and WARN branches simultaneously"
    - "Cross-target PreparedSandbox literal: always include #[cfg(target_os = linux)] wsl2_proxy_policy and af_unix_mediation fields"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/proxy_runtime.rs

key-decisions:
  - "D-07 fix: mirrored !prepared.custom_credentials.is_empty() in BOTH the ACTIVE and WARN branches (warn-and-ignore consistency recommended by D-07 discretion)"
  - "D-01 disposition: cherry-pick of upstream #1048/#1091 skipped; equivalence confirmed via test — fork already delivers the intent via external_proxy field"
  - "Cross-target clippy: PARTIAL→CI — Rust std targets installed but C cross-compilers (x86_64-linux-gnu-gcc, cc for darwin) absent on Windows host; activation predicate change is NOT cfg-gated so functional behavior is identical cross-target; test linux-cfg fields follow established template"

patterns-established:
  - "Proxy activation predicate: any new proxy-enabling field must be added to BOTH the ACTIVE branch (~line 109-116) AND the WARN branch (~line 91-95) in prepare_proxy_launch_options"

requirements-completed: [PROXY-02]

# Metrics
duration: 28min
completed: 2026-06-20
---

# Phase 89 Plan 01: Proxy Hardening Sync Summary

**Activation-gate fix (#1197/D-07): proxy now starts when only `customCredentials` is configured, plus D-01 external_proxy equivalence test, with 3 regression tests in proxy_runtime.rs**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-06-20T19:13:53Z
- **Completed:** 2026-06-20T19:42:00Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- Fixed the D-07 activation gap: `prepare_proxy_launch_options` now includes `!prepared.custom_credentials.is_empty()` in both the ACTIVE branch and the WARN branch so a customCredentials-only config correctly starts the proxy (previously the predicate checked `credentials`, `network_profile`, `allow_domain`, and `upstream_proxy` but NOT `custom_credentials`, leaving injected credentials unprotected)
- Added two D-07 regression tests: `proxy_activates_with_custom_credentials_only` (asserts `active=true`) and `block_net_overrides_custom_credentials_activation` (asserts `active=false` under Blocked mode), both using a full `PreparedSandbox` literal with cross-target `#[cfg(target_os = "linux")]` fields
- Added one D-01 equivalence test: `build_proxy_config_maps_upstream_proxy_to_external_proxy` confirms the fork's `external_proxy` field delivers upstream #1048/#1091's `upstream_proxy` intent — cherry-pick confirmed unnecessary

## Task Commits

Each task was committed atomically:

1. **Task 1: Add custom_credentials disjunct to the activation predicate (both branches)** - `0c08e5d2` (fix)
2. **Task 2: D-07 regression tests** - `73bd03a6` (test)
3. **Task 3: D-01 external_proxy mapping equivalence test** - `751c6cab` (test)

## Files Created/Modified

- `crates/nono-cli/src/proxy_runtime.rs` - Activation predicate fix (2 disjunct additions) + 3 new tests (proxy_activates_with_custom_credentials_only, block_net_overrides_custom_credentials_activation, build_proxy_config_maps_upstream_proxy_to_external_proxy)

## Decisions Made

- Mirrored `!prepared.custom_credentials.is_empty()` in BOTH the ACTIVE and WARN branches per D-07 discretion (recommended YES for warn-and-ignore consistency). Under `--block-net`, the warn fires but `active` stays false regardless.
- D-01 disposition confirmed: fork already maps `upstream_proxy` to `ProxyConfig.external_proxy` at `build_proxy_config_from_flags` lines 222-228. No cherry-pick of upstream #1048/#1091 required.
- Cross-target clippy deferred to CI (PARTIAL→CI): the production predicate change carries no `#[cfg]` guards so it compiles identically cross-target. The test `PreparedSandbox` literals follow the established cross-target pattern (linux-cfg fields included). Windows host cross-compilers absent.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Cross-Target Verify Status

**PARTIAL→CI** — Both `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` Rust std targets are installed; however, the C cross-compilers (`x86_64-linux-gnu-gcc`, `cc`) required by `aws-lc-sys` build script are absent on this Windows host. `cargo clippy --workspace --target x86_64-unknown-linux-gnu` exits with a build-script cc-rs error before clippy can run.

The activation predicate change (`!prepared.custom_credentials.is_empty()`) is NOT inside any `#[cfg(...)]` block — it compiles identically on all platforms. The test `PreparedSandbox` literals include the required `#[cfg(target_os = "linux")]` fields (`wsl2_proxy_policy`, `af_unix_mediation`) following the established template from `resolve_effective_proxy_settings_preserves_with_endpoints` (line 417-420). Full cross-target clippy will run in CI.

## Test Names (for 89-04 ledger addendum)

- `proxy_runtime::tests::proxy_activates_with_custom_credentials_only` — D-07 fix guard (active=true)
- `proxy_runtime::tests::block_net_overrides_custom_credentials_activation` — D-07 block-net override guard (active=false)
- `proxy_runtime::tests::build_proxy_config_maps_upstream_proxy_to_external_proxy` — D-01 equivalence (external_proxy mapping)

## D-07 Fix Commit

**Commit SHA:** `0c08e5d2`
**Upstream reference:** 724bb207 (#1197)
**Classification:** Behavioral fix (fail-secure: adds activation, never removes a denial)

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The activation predicate edit only ADDS a disjunct that starts the proxy when previously it would have been skipped — consistent with the threat model (T-89-01 mitigated: credentials now protected via proxy injection). No new threat flags.

## Self-Check: PASSED

- `crates/nono-cli/src/proxy_runtime.rs`: FOUND (modified, not deleted)
- Commit `0c08e5d2`: Task 1 fix commit
- Commit `73bd03a6`: Task 2 test commit
- Commit `751c6cab`: Task 3 test commit
- `grep -c '!prepared.custom_credentials.is_empty()' proxy_runtime.rs` = 2 (both branches)
- `cargo test -p nono-cli -- proxy_runtime`: 13 passed, 0 failed

## Next Phase Readiness

- D-07 fix and regression tests complete; test names captured for 89-04 ledger addendum
- All 3 plan-level success criteria met
- Ready for 89-02 (nono-proxy tests: D-09 reverse proxy 403+audit, D-02 CONNECT keep-open)

---
*Phase: 89-proxy-hardening-sync*
*Plan: 01*
*Completed: 2026-06-20*
