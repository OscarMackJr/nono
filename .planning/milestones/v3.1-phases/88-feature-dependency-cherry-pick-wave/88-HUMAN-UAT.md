---
status: partial
phase: 88-feature-dependency-cherry-pick-wave
source: [88-VERIFICATION.md]
started: 2026-06-20T22:30:00Z
updated: 2026-06-20T22:30:00Z
---

## Current Test

[awaiting human testing — all three items require a Linux/macOS host or GH Actions CI; not runnable on the Windows dev host]

## Tests

### 1. PTY ctrl-z suspend/resume no longer hangs on Linux/macOS
expected: Pressing ctrl-z in a supervised PTY session suspends the child process group and the nono supervisor does not hang; fg resumes cleanly.
why_human: DEPS-01 PTY functions (`signal_pty_foreground_group`, `handle_pty_suspension`) use `nix::` symbols gated behind `#[cfg(not(target_os = "windows"))]`; cannot run on the Windows dev host. Requires Linux/macOS CI or a live terminal test.
result: [pending]

### 2. XDG state migration fires on first run when legacy ~/.nono exists on Linux/macOS
expected: Running nono for the first time after the upgrade moves `~/.nono/audit/` to `~/.local/state/nono/audit/`, and subsequent runs use the new XDG location exclusively.
why_human: `maybe_migrate_legacy_audit_ledger()` and the `cfg(not(target_os = "windows"))` branches in `state_paths.rs` are PARTIAL→CI — unverifiable on the Windows dev host.
result: [pending]

### 3. Hook subprocess on Linux/macOS inherits parent environment after env_clear removal (e54cf9cb)
expected: A hook script can read environment variables from the nono parent process (e.g. HOME, PATH), confirming `env_clear()` is absent from the Unix hook path. Windows hook must still clear env (`env_clear` retained in `hook_runtime_windows.rs`).
why_human: The `hook_runtime.rs` env_clear removal is in a `#[cfg(unix)]` exec path; only GH Actions Linux/macOS CI or a live Unix test can confirm the intended env-inheritance behavior.
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
