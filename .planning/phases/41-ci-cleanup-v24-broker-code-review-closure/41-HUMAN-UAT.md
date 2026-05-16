---
status: partial
phase: 41-ci-cleanup-v24-broker-code-review-closure
source: [41-VERIFICATION.md]
started: 2026-05-16T19:30:00Z
updated: 2026-05-16T19:30:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. windows-build CI lane no longer fails at PowerShell parameter binding (post-Plan-41-08)
expected: ci-logs/windows-build.log contains NO "Cannot process command because of one or more missing mandatory parameters: BrokerPath" line; the build suite progresses past "validate windows msi contract" label; cargo build -p nono-shell-broker step appears and succeeds; Test-Path guard passes silently
result: [pending]

### 2. All 7 GitHub Actions CI lanes green on Phase 41 close SHA (post-Plan-41-08 head)
expected: Linux Clippy + macOS Clippy + Windows Build + Windows Integration + Windows Regression + Windows Security + Windows Packaging all PASS on the same head SHA
result: [pending]

### 3. env_vars parallel flake fix on real Windows host
expected: cargo test -p nono-cli --test env_vars windows_run_redirects_profile_state_vars_into_writable_allowlist run 10x in parallel — 0 failures across 10 runs
result: [pending]

### 4. Block-net probe tests on elevated Windows runner with NONO_CI_HAS_WFP=true
expected: windows_run_block_net_blocks_probe_connection + windows_run_block_net_blocks_probe_through_cmd_host both PASS with "connect failed" or "exit code 42" markers in stderr
result: [pending]

### 5. Cross-binding nono-py / nono-ts D-10 FFI remap audit
expected: No integer-mapping of -1 (ErrPathNotFound) as broker-discovery-failure in downstream bindings — or follow-up todo filed for lockstep
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
