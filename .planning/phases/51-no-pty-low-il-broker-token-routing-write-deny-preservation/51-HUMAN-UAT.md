---
status: complete
phase: 51-no-pty-low-il-broker-token-routing-write-deny-preservation
source: [51-VERIFICATION.md]
started: "2026-05-26T18:29:01Z"
updated: "2026-05-26T19:30:00Z"
---

## Current Test

[testing complete — SC-4 closed via Phase 52 repro A (live PASS, operator-attested 2026-05-26)]

## Tests

### 1. ROADMAP SC-4 — positive supervised-spawn exit-0 on a real Windows host

expected: A supervised launch through the no-PTY Low-IL broker path
(`windows_low_il_broker: true`, e.g. `claude-code`) reaches the child process
and exits cleanly (exit 0) on a Windows host whose launch-path policy covers the
target executable — confirming the `0xC0000142` / STATUS_DLL_INIT_FAILED
regression is gone in live use (not just at the unit/integration layer).

result: pass

note: Closed via Phase 52 HUMAN-UAT. Repro A (`nono run --profile claude-code --
cmd /c "echo hi"`, run from the profile-covered cwd `C:\Users\OMack\.claude`)
printed `hi` and exited 0 on Windows 11 build 26200 with the Phase 51 `nono 0.57.0`
BrokerLaunchNoPty binary — operator-attested PASS 2026-05-26. This empirically
corrects the earlier pessimistic claim that the cmd/echo form "cannot exit 0 on
the dev host": that claim conflated the **cwd-coverage gate** with the distinct
**Phase 27 launch-path gate** (which fires only for cmd shapes resolving `C:\`,
e.g. `cmd /c cd`). `cmd /c echo hi` does not require `C:\` and passes from a
covered cwd. Repro B (`claude --version`, 234 MB self-contained `claude.exe`)
also exited 0 — stronger confirmation the `0xC0000142` / STATUS_DLL_INIT_FAILED
regression is gone in live end-to-end use. See
`52-HUMAN-UAT.md` § "Phase 51 SC-4 Closure Note" (REQ-WSRH-04 satisfied).

## Summary

total: 1
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

(none — SC-4 closed via Phase 52 repro A)
