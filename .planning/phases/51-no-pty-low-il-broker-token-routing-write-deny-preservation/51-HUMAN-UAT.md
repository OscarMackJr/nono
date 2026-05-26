---
status: partial
phase: 51-no-pty-low-il-broker-token-routing-write-deny-preservation
source: [51-VERIFICATION.md]
started: "2026-05-26T18:29:01Z"
updated: "2026-05-26T18:29:01Z"
---

## Current Test

[awaiting human testing — deferred to Phase 52 heavy-runtime HUMAN-UAT per user approval]

## Tests

### 1. ROADMAP SC-4 — positive supervised-spawn exit-0 on a real Windows host

expected: A supervised launch through the no-PTY Low-IL broker path
(`windows_low_il_broker: true`, e.g. `claude-code`) reaches the child process
and exits cleanly (exit 0) on a Windows host whose launch-path policy covers the
target executable — confirming the `0xC0000142` / STATUS_DLL_INIT_FAILED
regression is gone in live use (not just at the unit/integration layer).

result: [pending — deferred to Phase 52]

note: The literal ROADMAP smoke `nono run --profile claude-code -- cmd /c "echo hi"`
cannot exit 0 on the dev host because `cmd.exe` hits the pre-existing Phase 27
launch-path policy gate ("Windows filesystem policy does not cover the absolute
path argument required for launch: C:\\"; see `crates/nono-cli/tests/audit_attestation.rs:118`),
a fail-secure refusal that fires BEFORE any process spawn — so the `0xC0000142`
regression is confirmed absent (the run never reaches DLL init). Positive
end-to-end validation with a real heavy runtime (`claude.exe`) is the explicit
charter of Phase 52 (REQ-WSRH-04 / REQ-WSRH-06). User approved this deferral at
the Phase 51 execute-phase checkpoint (2026-05-26).

## Summary

total: 1
passed: 0
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
