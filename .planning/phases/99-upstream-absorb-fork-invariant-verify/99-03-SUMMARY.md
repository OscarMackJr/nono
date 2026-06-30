---
phase: "99"
plan: "03"
subsystem: network
tags: [upstream-absorb, validate-block-net, contradictory-flags, proxy-guard-tests, D-08]
dependency_graph:
  requires: [99-02]
  provides: [validate_block_net_conflicts, has_proxy_intent, d457ecc3-replay]
  affects: [nono-cli]
tech_stack:
  added: []
  patterns: [upstream-replay, fail-secure-validation, early-error-detection]
key_files:
  created: []
  modified:
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/command_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
decisions:
  - D-has_proxy_intent-placement: has_proxy_intent() placed as private fn in sandbox_prepare.rs, consistent with validate_block_net_conflicts; both functions share file scope and avoid import cycles
  - D-empty_prepared-adaptation: empty_prepared() test helper adapted to fork struct (omits upstream-only fields profile_display_name, command_policies, credential_capture, tls_intercept, allow_gpu_active, allow_http2_requested — not yet absorbed)
  - D-linux-seccomp-host-limit: linux.rs seccomp guard tests (test_proxy_only_with_landlock_v4_returns_no_fallback etc.) return 0 tests on Windows host (cfg-gated); confirmed exits 0; cross-target gate in Plan 07 is the definitive Linux verification
metrics:
  duration: "~20 min"
  completed: "2026-06-30"
  tasks_completed: 2
  files_modified: 3
---

# Phase 99 Plan 03: d457ecc3 (#1263) Replay + Guard Test Verification Summary

Manual replay of upstream commit d457ecc3 (#1263 "error early on contradictory network flag combinations") as companion to 72bcfd66 (Plan 02). Adds validate_block_net_conflicts() called before sandbox/proxy setup on both the dry-run and main launch paths; catches --block-net combined with proxy/credential/network-profile/allow-domain flags. All Phase 89 proxy guard tests and D-08 deviation tests verified GREEN.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Manual replay d457ecc3 (#1263 validate_block_net_conflicts) | 92bd42fe | sandbox_prepare.rs, command_runtime.rs, launch_runtime.rs |
| 2 | Phase 89 proxy guard tests + D-08 deviation tests (verification only) | — | (no file changes) |

## What Was Built

### Task 1: d457ecc3 Replay (92bd42fe)

**sandbox_prepare.rs:**
- Refactored `finalize_prepared_sandbox` to call new `has_proxy_intent(args, &prepared)` instead of inline computation — also fixes gap where `custom_credentials` from a profile were not counted toward proxy intent for display output
- Added private `fn has_proxy_intent(args: &SandboxArgs, prepared: &PreparedSandbox) -> bool` — checks `has_proxy_flags()`, `credentials`, `custom_credentials`, `network_profile`, `allow_domain`, `upstream_proxy`
- Added `pub(crate) fn validate_block_net_conflicts(args: &SandboxArgs, prepared: &PreparedSandbox) -> Result<()>` — validates: (a) `--block-net` + credentials contradictory, (b) `--block-net` + `--network-profile` contradictory, (c) `--block-net` + `--allow-domain` contradictory, (d) `--allow-endpoint` without any credential is a no-op/error, (e) `--proxy-port` without any proxy-triggering flag is a no-op/error. Checks both CLI flags AND profile-sourced values (profile_network_block, prepared.credentials, etc.)
- Added 9 new tests + `empty_prepared()` helper in the test module (adapted to fork struct fields)

**command_runtime.rs:**
- Added `validate_block_net_conflicts` to imports from `crate::sandbox_prepare`
- Added `validate_block_net_conflicts(&args, &prepared)?;` call on dry-run path (before `validate_external_proxy_bypass`)

**launch_runtime.rs:**
- Added `validate_block_net_conflicts` to imports (expanded import block)
- Added `validate_block_net_conflicts(&args, &prepared)?;` call on main launch path (before `validate_rollback_destination`)

### Task 2: Guard Test Verification (verification only, no commit)

All guard tests run and confirmed GREEN:

| Test | Result |
|------|--------|
| `reverse::tests::denied_endpoint_returns_403_and_audit` | ok |
| `route::tests::allow_domain_endpoint_route_does_not_shadow_credential_route` | ok |
| `cargo test -p nono-proxy --lib` (176 tests) | 176 passed, 0 failed |
| `profile::d08_deviation_tests::test_wsl2_proxy_policy_deviation_preserved` | ok |
| `proxy_runtime::tests::test_compiled_endpoint_policy_compat_deviation_preserved` | ok |
| linux.rs seccomp guard tests | 0 tests on Windows host (cfg-gated linux); exits 0 (Plan 07 cross-target gate is definitive) |
| `cargo test -p nono-cli --bin nono` (1390 total) | 1379 passed, 11 failed (all 11 are documented pre-existing baseline failures identical to 99-02) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] empty_prepared() adapted for fork struct**
- **Found during:** Task 1 test implementation
- **Issue:** Upstream diff's `empty_prepared()` test helper includes fields that don't exist in the fork: `profile_display_name`, `command_policies`, `credential_capture`, `tls_intercept`, `allow_gpu_active`, `allow_http2_requested`. These upstream fields are from clusters not yet absorbed.
- **Fix:** Wrote `empty_prepared()` with only the fork's actual PreparedSandbox fields (matched against the struct definition at lines 66-119).
- **Files modified:** `sandbox_prepare.rs`
- **Commit:** 92bd42fe

**2. [Implementation deviation] linux.rs seccomp guard tests host-gated on Windows**
- **Found during:** Task 2 execution
- **Issue:** `test_proxy_only_with_landlock_v4_returns_no_fallback`, `test_seccomp_network_fallback_mode_proxy_only`, `test_seccomp_network_fallback_mode_proxy_only_with_bind` are `#[cfg(target_os = "linux")]` — they compile and run only on Linux. On the Windows host, `cargo test -p nono --lib sandbox::linux::tests::*` finds 0 tests and exits 0.
- **Impact:** Not a regression — these tests were not broken by Plan 03 (Cluster A is CLI-only; linux.rs was not touched). Definitive verification is the cross-target `cross clippy` + `cargo test --target x86_64-unknown-linux-gnu` gate in Plan 07.
- **Action:** Documented as host-limitation deviation; no fix needed.

## Test Results

- `cargo build --workspace --all-targets`: PASS (clean, 0 errors, 0 warnings)
- `cargo test -p nono-proxy --lib`: 176 passed, 0 failed
- `cargo test -p nono-cli --bin nono` new sandbox_prepare tests: 9 passed, 0 failed
- `cargo test -p nono-cli --bin nono` full suite: 1379 passed, 11 failed (11 pre-existing baseline failures, identical to 99-02)
- D-08 deviation tests: both pass
- Phase 89 proxy guard tests: all pass (including `denied_endpoint_returns_403_and_audit` — primary CompiledEndpointPolicy chain signal)

## Security Verification

- T-99-05: validate_block_net_conflicts() call ordering confirmed: call site in launch_runtime.rs is BEFORE `validate_rollback_destination` and all sandbox/proxy setup. Call site in command_runtime.rs dry-run path is BEFORE `validate_external_proxy_bypass`. MITIGATED.
- T-99-06: CompiledEndpointPolicy evaluate() bypass — `denied_endpoint_returns_403_and_audit` passes. D-08 `test_compiled_endpoint_policy_compat_deviation_preserved` passes. MITIGATED.
- T-99-07: linux.rs seccomp/AF_UNIX invariants — Cluster A is CLI-only; linux.rs was not touched. Cross-target gate in Plan 07 is definitive confirmation. DEFERRED to Plan 07.
- T-99-SC: No new package installs. d457ecc3 adds only function code; Cargo.toml unchanged. ACCEPTED.

## Known Stubs

None — all validation logic paths are fully implemented. validate_block_net_conflicts() is a complete early-exit validator with no placeholder returns.

## Threat Flags

None — no new network endpoints or trust boundaries introduced. validate_block_net_conflicts() is a pure error-on-contradiction validator; it adds no new capability grant or bypass surface.

## Self-Check: PASSED

- 99-03-SUMMARY.md: created at `.planning/phases/99-upstream-absorb-fork-invariant-verify/99-03-SUMMARY.md`
- Task 1 commit 92bd42fe: verified (`git log --oneline -1` shows fix(network) commit)
- cherry-pick trailer present: `git show HEAD | grep "(cherry picked from commit d457ecc3"` returns 1 match
- DCO sign-off present: `git show HEAD | grep "Signed-off-by: Oscar Mack Jr"` returns 1 match
- validate_block_net_conflicts in sandbox_prepare.rs: 10 matches (definition + tests)
- validate_block_net_conflicts in launch_runtime.rs: 2 matches (import + call)
- validate_block_net_conflicts in command_runtime.rs: 2 matches (import + call)
- Build: PASS (0 errors, 0 warnings)
- proxy guard tests: PASS
- D-08 deviation tests: PASS
