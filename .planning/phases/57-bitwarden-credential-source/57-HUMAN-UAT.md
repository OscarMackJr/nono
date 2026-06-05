---
status: partial
phase: 57-bitwarden-credential-source
source: [57-VERIFICATION.md]
started: 2026-06-05T21:00:00Z
updated: 2026-06-05T21:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Live bw CLI path (item backend)
expected: Run `nono run` with a real `BW_SESSION` and a `bw://item/<id>/password=MY_VAR` credential against an actual unlocked Bitwarden vault. The sandboxed child process receives the secret as the `MY_VAR` environment variable; no raw secret appears in `ps` output, `/proc/<pid>/cmdline`, or any tracing log line. `BW_SESSION` itself is absent from argv.
result: [pending]

### 2. Live bws CLI path (Secrets Manager backend)
expected: Run `nono run` with a real `BWS_ACCESS_TOKEN` and a `bw://secret/<uuid>=MY_VAR` credential against a real Bitwarden Secrets Manager project. The sandboxed child receives the secret's `.value` field as `MY_VAR`; no raw token appears in `ps` output or tracing logs.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
