---
status: partial
phase: 37-linux-resl-backends-pkgs-auto-pull
source: [37-VERIFICATION.md]
started: 2026-05-20T03:48:00Z
updated: 2026-05-20T03:48:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Push umbrella PR branch with Phase 37 commits and confirm `Phase 37 Linux RESL` workflow runs green on `ubuntu-24.04`
expected: Both jobs (`resl-nix` and `pkgs-auto-pull`) report `conclusion=success`; the `Wait for user session and verify cpu controller delegated` step prints `OK: cpu controller delegated`; integration tests `linux_memory_limit_oom_kills_child`, `linux_cpu_percent_throttles_yes_loop`, `linux_max_processes_5_fork_bomb_contained`, `linux_max_processes_blocks_eleventh_fork`, `auto_pull_happy_path_mock`, `auto_pull_unknown_name_fails_closed`, `auto_pull_no_auto_pull_flag_falls_back_to_profile_not_found`, `auto_pull_signature_failure_aborts`, `auto_pull_rejects_non_policy_pack_type` all pass; CPU-throttle test's measured average falls within `[15, 40]`% band.
result: [pending]

### 2. Confirm REQ-RESL-NIX-02 CPU throttling actually fires on cgroup v2 host (not silently skipped)
expected: `linux_cpu_percent_throttles_yes_loop` runs (not SKIPs due to `require_cpu_controller!` macro), samples top 5 times, asserts average %CPU in [15, 40]
result: [pending]

### 3. Confirm REQ-PKGS-04 acceptance #1 (happy path) e2e on Linux runner with CI-signed fixture
expected: `auto_pull_happy_path_mock` runs (not SKIPs due to missing NONO_FIXTURE_PACK_DIR), exits 0, asserts `req_count > 0`
result: [pending]

### 4. Confirm Plan 37-02 `nono pull --no-auto-pull foo` rejection at clap-parse time
expected: Smoke test `nono pull --no-auto-pull foo` exits with `unexpected argument '--no-auto-pull' found`
result: [pending]

### 5. Confirm doc-flag check script (`check-cli-doc-flags.sh`) passes on `--no-auto-pull`
expected: Script exits 0 (or non-zero only for pre-existing `--dangerous-force-wfp-ready` drift per Phase 37-02 deferred-items)
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
