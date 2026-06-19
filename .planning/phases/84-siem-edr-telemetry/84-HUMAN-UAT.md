---
status: partial
phase: 84-siem-edr-telemetry
source: [84-VERIFICATION.md]
started: 2026-06-19T03:30:00.000Z
updated: 2026-06-19T03:30:00.000Z
---

## Current Test

[awaiting human testing on a provisioned Windows host]

## Tests

### 1. Full gate PASS on a provisioned host (SC-1 + SC-3 + SC-5)
expected: On an elevated shell with the nono MSI installed (Application Event Log source `nono` registered) and a fresh `nono.exe` on PATH, running `pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit` triggers a path-deny, finds EventID 10001 in the Application log with named fields (EventType, AgentPid, PathHash, SessionId, ChainHead), confirms no raw path strings (SC-3), and finds the `nono` ETW provider via `logman` (SC-5) — overall verdict PASS (exit 0).
result: [pending]
note: Invocation rule (durable) — use `pwsh -File scripts/verify-dark.ps1 ...`, NEVER `pwsh -Command "<bare path>"` (swallows exit N→1). Dev host cannot run this (needs admin + MSI-registered source + logman).

### 2. Admin opt-out end-to-end (WR-01 live proof)
expected: With `HKLM\SOFTWARE\Policies\nono` telemetry `TelemetryEnabled=0` (REG_DWORD 0) set, a confined `nono run` that hits a denied path emits NO nono security event to the Application log (the `inner.config.enabled` guard suppresses emission). Setting it back to 1 (or removing it) restores emission.
result: [pending]
why_human: Requires HKLM write access + live Application Event Log query; proves the policy-read → init_tracing → on_event path is wired end-to-end.

### 3. min_severity filtering (WR-02 live proof)
expected: With telemetry `MinSeverity=Error` in HKLM, a path-deny (Warning-severity) event is suppressed (no Application-log entry); with `MinSeverity=Warning` (default) or lower, the same denial emits. Confirms the HKLM → `TelemetryConfig.min_severity` → `severity_for()` guard pipeline is live.
result: [pending]
why_human: Requires HKLM write + live Event Log inspection.

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps

(none — structural verification passed 5/5; these are live-host confirmations of already-wired behavior, host-gated per the milestone Dark Factory policy)

## Follow-ups (not Phase 84 blockers)

- **Daemon-side telemetry emission**: `nono-agentd` / `agent_daemon/launch.rs` launch AppContainer agents via raw `CreateProcessW` and do not register a `SecurityEventLayer` or emit `nono_security::*` events. Daemon-side security-event emission is a distinct future emission domain (no Phase 84 success criterion names it). Carry forward to a future compliance/telemetry phase: call `init_tracing` from the daemon binary and wire daemon-level denial/launch events to `nono_security::*` targets.
