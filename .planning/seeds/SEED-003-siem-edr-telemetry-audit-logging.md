---
id: SEED-003
status: dormant
planted: 2026-06-08
planted_during: v2.10 (Phase 63)
trigger_when: milestone scope includes SIEM/EDR integration, Windows Event Log emission, Syslog, audit telemetry, or security event forwarding
scope: medium
priority: P2
---

# SEED-003: SIEM/EDR Integration & Tamper-Evident Audit Logging

## Why This Matters

A CISO needs to know when a containment breach was *attempted*, even if nono successfully blocked it. Today, blocked actions die in local stderr. If a prompt injection commands an agent to sweep the local subnet or read host files and nono stops it, that event must become a **structured security signal**, not a lost log line.

The solution: a telemetry pipeline that converts blocked actions into structured events pushed to the **Windows Event Log (Application/Security channels)** and/or **Syslog**, so existing EDR/SIEM platforms (Splunk, Microsoft Sentinel) can flag anomalous agent behavior immediately.

This is the **P2 ("Compliance")** priority of the enterprise horizon — satisfies corporate data governance + security monitoring (Windows Event Forwarding).

## When to Surface

**Trigger:** when a milestone targets SIEM/EDR integration, Windows Event Log emission, Syslog output, audit telemetry, or Windows Event Forwarding.

This seed will surface during `/gsd:new-milestone` when the milestone scope matches.

## Scope Estimate

**Medium — mostly net-new.** Work:
- A structured security-event schema for blocked/denied actions (path-deny, network-deny, label-violation, hook fail-closed).
- Emit to Windows Event Log (custom Application channel; consider Security channel which needs auth) and/or Syslog.
- Tamper-evidence (the "tamper-proof" claim) — append-only / signed event chain; cross-link to [[SEED-005]] ZT-Infra ledger for the immutable-audit angle.
- Wire the supervisor's existing deny-diagnostic path (`DiagnosticFormatter`) into the emitter.

## Breadcrumbs

- `crates/nono/src/diagnostic.rs` — `DiagnosticFormatter` (human-readable denial explanations; the structured-event source).
- `crates/nono-cli/src/hooks.rs` + Phase 58 session lifecycle hooks (`windows_hook_interpreter_spawn_gotchas`) — fail-closed events worth emitting.
- Phase 66 (WR-02 EDR HUMAN-UAT) — adjacent EDR-runner UAT in the current v2.10 milestone; this seed is the *telemetry-emission* counterpart.
- `tracing`/`tracing-subscriber` (existing logging stack) — likely the emission integration point.

## Notes

Captured 2026-06-08 (CISO/CTO horizon). Distinct from Phase 66 (which is EDR *UAT*, not Event-Log emission). Sibling seeds: [[SEED-001]], [[SEED-002]], [[SEED-004]], [[SEED-005]].
