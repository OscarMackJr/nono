---
status: partial
phase: 35-upst3-closure-quick-wins
source: [35-VERIFICATION.md]
started: 2026-05-12T00:00:00Z
updated: 2026-05-12T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Windows env_filter_tests execution
expected: `cargo test -p nono-cli --lib exec_strategy_windows::launch::env_filter_tests` on a Windows host reports `test result: ok. 4 passed; 0 failed` — `test_windows_empty_allow_denies_all_env_vars`, `test_windows_deny_strips_matching_env_vars`, `test_windows_allow_passes_only_matching_env_vars`, `test_windows_nono_injected_credentials_bypass_both` all pass.
result: [pending]

### 2. Linux first-run profiles-dir UX
expected: Triggering first-run `nono run` on a clean Linux install (after removing `~/.config/nono/profiles/`) completes startup without a "No such file or directory" error; the profiles directory is created on disk before the Landlock ruleset applies.
result: [pending]

### 3. JSON Debug-leak integration tests
expected: `cargo test -p nono-cli --test profile_cli test_policy_show_json_no_rust_debug_syntax` and `cargo test -p nono-cli --test profile_cli test_policy_diff_json_no_rust_debug_syntax` both pass with exit 0; JSON output for built-in profiles contains no `Some(...)` or `None` Rust-Debug-format strings for security fields.
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
