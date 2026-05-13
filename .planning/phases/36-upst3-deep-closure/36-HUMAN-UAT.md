---
status: partial
phase: 36-upst3-deep-closure
source: [36-VERIFICATION.md]
started: 2026-05-13T00:00:00Z
updated: 2026-05-13T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. PTY quiet-period real-world feel
expected: Process exits cleanly through `nono run -- <short-cmd>` on Linux or macOS; the 100ms PTY drain timeout is not perceptible and no output is truncated.
result: [pending]

### 2. Docs MDX render
expected: `docs/cli/features/profile-authoring.mdx` and the embedded `crates/nono-cli/data/profile-authoring-guide.md` render correctly in the docs site or VS Code MDX preview; all code blocks, field tables, and `bypass_protection` examples are intact post-migration.
result: [pending]

### 3. Linux/macOS host execution (Landlock + Seatbelt)
expected: On a Linux host (Landlock) and macOS host (Seatbelt), `nono run --profile claude-code -- echo "sandbox test"` applies the sandbox without capability regression. Verifies the CR-01 fix (commands.allow/deny wired into `from_profile()`) and the `bypass_protection` rename takes effect end-to-end.
result: [pending]

### 4. Detached-console smoke gate (Windows interactive)
expected: On Windows, `nono run` invoked from a detached-console scenario (or wrapped via cmd /c start) does not panic; `should_offer_profile_save()` and `compute_executable_identity()` behave correctly with a real console handle.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
