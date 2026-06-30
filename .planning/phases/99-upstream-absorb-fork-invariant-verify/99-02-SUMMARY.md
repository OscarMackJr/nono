---
phase: "99"
plan: "02"
subsystem: network
tags: [upstream-absorb, NetworkIntent, proxy, deviation-tests, ADR-98]
dependency_graph:
  requires: [99-01]
  provides: [NetworkIntent-enum, D-08-deviation-tests]
  affects: [nono-cli, nono-proxy]
tech_stack:
  added: [NetworkIntent enum]
  patterns: [upstream-replay, deviation-preservation, fail-secure-network-block]
key_files:
  created: []
  modified:
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/capability_ext.rs
    - crates/nono-cli/src/output.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/supervised_runtime.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/main.rs
decisions:
  - D-NetworkIntent-placement: NetworkIntent placed in launch_runtime.rs (not sandbox_prepare.rs as plan stated) to avoid circular dependency; matches upstream placement
  - D-ProxyLaunchOptions-Debug: Added #[derive(Debug)] to ProxyLaunchOptions required by NetworkIntent derive(Debug)
  - D-ProxyOpts-CfgGated: proxy_opts binding in supervised_runtime.rs gated #[cfg(not(windows))] to avoid dead-code warning on Windows
metrics:
  duration: "~90 min (continuation session)"
  completed: "2026-06-30T14:32:46Z"
  tasks_completed: 2
  files_modified: 9
---

# Phase 99 Plan 02: D-08 Deviation Tests + 72bcfd66 NetworkIntent Replay Summary

D-08 proof-tests for two ADR-98 fork deviations; then manual replay of upstream commit 72bcfd66 introducing `NetworkIntent` enum and removing `ProxyOnly {port:0}` placeholders across 9 CLI files, preserving all fork-specific extensions.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | D-08 deviation tests | 19935363 | profile/mod.rs, proxy_runtime.rs |
| 2 | Manual replay 72bcfd66 (NetworkIntent) | 5423f51f | 9 CLI files |

## What Was Built

### Task 1: D-08 Deviation Tests (19935363)

Added `d08_deviation_tests` module to `profile/mod.rs`:
- `test_wsl2_proxy_policy_deviation_preserved`: proves `Wsl2ProxyPolicy::Error/InsecureProxy` enum variants, `SecurityConfig.wsl2_proxy_policy` field, and JSON round-trip survive the upstream replay (ADR-98 deviation 1).

Added D-08 test inside `proxy_runtime::tests`:
- `test_compiled_endpoint_policy_compat_deviation_preserved`: proves `CompiledEndpointPolicy::compile() → evaluate()` chain accessible from proxy_runtime context post-72bcfd66 (ADR-98 deviation 2).

### Task 2: Manual Replay of Upstream 72bcfd66 (5423f51f)

**NetworkIntent enum** added to `launch_runtime.rs`:
```
pub(crate) enum NetworkIntent {
    Unrestricted,          // --allow-net or no network flags
    BlockAll,              // --block-net wins
    ProxyFiltered(Box<ProxyLaunchOptions>),  // proxy-activating features present
}
```

**9 files changed:**
- `launch_runtime.rs`: NetworkIntent enum + impl; `ProxyLaunchOptions.network_block` → `strict_filter`; `ExecutionFlags.proxy` → `network`; `ProxyLaunchOptions` gets `#[derive(Debug)]`
- `proxy_runtime.rs`: `prepare_proxy_launch_options` return type `Result<ProxyLaunchOptions>` → `Result<NetworkIntent>`; `start_proxy_runtime` accepts `&NetworkIntent`; `build_proxy_config_from_flags` uses `strict_filter`; all tests updated
- `sandbox_prepare.rs`: `PreparedSandbox.network_block_requested` → `profile_network_block`; proxy_pending computed before `print_capabilities`
- `capability_ext.rs`: removed `ProxyOnly {port:0}` placeholder branches (profile + CLI paths)
- `output.rs`: `print_capabilities` gains `proxy_pending: bool`; NetworkMode display updated
- `execution_runtime.rs`: `proxy` → `network`; `allow_domain` accessed via `network.proxy_options()`
- `supervised_runtime.rs`: struct field `proxy: &'a ProxyLaunchOptions` → `network: &'a NetworkIntent`; proxy field access wrapped with `proxy_opts.map(...).unwrap_or(...)`
- `profile/mod.rs`: `NetworkConfig::has_proxy_flags()` removed + tests removed
- `main.rs`: `network_block_requested` → `profile_network_block` (2 sites); unused `merge_dedup_ports` re-export removed

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] NetworkIntent placed in launch_runtime.rs (not sandbox_prepare.rs)**
- **Found during:** Task 2 planning
- **Issue:** Plan said put `NetworkIntent` in `sandbox_prepare.rs` but that creates a circular dependency: `sandbox_prepare.rs` imports `launch_runtime::ProxyLaunchOptions`, and `launch_runtime.rs` imports `sandbox_prepare::PreparedSandbox`. Placing `NetworkIntent` in `sandbox_prepare.rs` would require `launch_runtime.rs` to import from `sandbox_prepare.rs` creating the cycle.
- **Fix:** Placed `NetworkIntent` in `launch_runtime.rs`, matching upstream's actual placement.
- **Files modified:** `launch_runtime.rs`, `proxy_runtime.rs` (import updated)

**2. [Rule 1 - Bug] Doc comments on function parameters**
- **Found during:** Task 2 build
- **Issue:** `///` doc comments used on function parameters in `output.rs` — Rust does not allow doc comments on function parameters.
- **Fix:** Changed `///` to `//` on the `proxy_pending` parameter docs.
- **Files modified:** `output.rs`

**3. [Rule 2 - Missing] ProxyLaunchOptions missing Debug derive**
- **Found during:** Task 2 build
- **Issue:** `NetworkIntent` derives `Debug` but contains `Box<ProxyLaunchOptions>` which lacked `#[derive(Debug)]`.
- **Fix:** Added `Debug` to `ProxyLaunchOptions`'s derive list.
- **Files modified:** `launch_runtime.rs`

**4. [Rule 1 - Bug] proxy_opts unused on Windows**
- **Found during:** Task 2 build
- **Issue:** `let proxy_opts` binding in `supervised_runtime.rs` was unconditional but only used in `#[cfg(not(target_os = "windows"))]` code, triggering unused-variable warning (treated as error under `-D warnings`).
- **Fix:** Moved binding inside `#[cfg(not(target_os = "windows"))]` block; split `ProxyLaunchOptions` import to be cfg-gated as well.
- **Files modified:** `supervised_runtime.rs`

**5. [Implementation deviation] block_net_overrides test updated for new detection logic**
- **Found during:** Task 2 test update
- **Issue:** Old test detected block via `caps.set_network_mode_mut(NetworkMode::Blocked)`. New logic checks `args.block_net` directly (72bcfd66 change).
- **Fix:** Updated test to set `args.block_net = true` via `SandboxArgs { block_net: true, ..default() }` instead of mutating caps.
- **Files modified:** `proxy_runtime.rs`

## Test Results

- `cargo build --workspace --all-targets`: PASS (clean, 0 errors, 0 warnings)
- `cargo test -p nono-cli --bin nono`: 1370 passed, 11 failed (all 11 are documented pre-existing baseline failures — `config::tests::*` env-lock PoisonError, `profile_cmd::test_init_allowed_*`, `protected_paths::tests::*`)
- `cargo test -p nono-proxy --lib`: 176 passed, 0 failed
- D-08 deviation tests: all 2 pass (`test_wsl2_proxy_policy_deviation_preserved`, `test_compiled_endpoint_policy_compat_deviation_preserved`)
- Updated proxy_runtime tests: all pass (strict_filter propagation, block_net override, proxy_activates_with_custom_credentials, resolve_effective_proxy)

## Known Stubs

None — all proxy/network code paths are fully wired.

## Threat Flags

None — no new network endpoints or trust boundaries introduced. NetworkIntent is an internal routing enum with no new external surface.

## Self-Check: PASSED

- 99-02-SUMMARY.md: created
- Task 1 commit 19935363: verified (`git log --oneline | grep 19935363`)
- Task 2 commit 5423f51f: verified (`git log --oneline | grep 5423f51f`)
- Build: PASS
- D-08 deviation tests: PASS
