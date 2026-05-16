---
status: partial
phase: 41-ci-cleanup-v24-broker-code-review-closure
source: [41-VERIFICATION.md]
started: 2026-05-16T19:30:00Z
updated: 2026-05-16T21:50:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Linux/macOS CI lanes flip RED → GREEN after Plan 41-09 (NEW)
expected: GitHub Actions Linux Test, Linux Clippy, macOS Clippy lanes on the SHA carrying 47d55905 (or its successor) all PASS. No occurrence of "function `validate_env_var_patterns` is never used", "field `interactive_shell` is never read", "fields `register_wfp_service`", "methods `register_phase_index`", "associated function `set_all` is never used", or "using `map_err` over `inspect_err`" in lane logs.
result: [pending]

### 2. windows-build CI lane no longer fails at PowerShell parameter binding (Plan 41-08)
expected: ci-logs/windows-build.log contains NO "Cannot process command because of one or more missing mandatory parameters: BrokerPath" line; build suite progresses past "validate windows msi contract" label; cargo build -p nono-shell-broker step appears and succeeds.
result: [pending]

### 3. All 8 GH Actions CI lanes green on Phase 41 close SHA (post-41-09 head)
expected: Linux Clippy + Linux Test + macOS Clippy + Windows Build + Windows Integration + Windows Regression + Windows Security + Windows Packaging all PASS on the same head commit.
result: [pending]

### 4. env_vars parallel flake fix on real Windows host (Plan 41-05) — 10x runs
expected: cargo test -p nono-cli --test env_vars windows_run_redirects_profile_state_vars_into_writable_allowlist run 10x in parallel — 0 failures across 10 runs.
result: [pending]

### 5. Block-net probe tests on elevated Windows runner with NONO_CI_HAS_WFP=true
expected: windows_run_block_net_blocks_probe_connection + windows_run_block_net_blocks_probe_through_cmd_host both PASS with "connect failed" or "exit code 42" markers in stderr.
result: [pending]

### 6. Cross-binding nono-py / nono-ts D-10 FFI remap audit
expected: No integer-mapping of -1 (ErrPathNotFound) as broker-discovery-failure in downstream bindings — or follow-up todo filed for lockstep.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
