# ADR: Telemetry Tamper-Evidence Scope (Phase 84)

**Status:** Accepted
**Date:** 2026-06-19
**Phase:** 84 (SIEM/EDR Telemetry)
**Requirements:** TELEM-02, D-07

---

## Context

nono Phase 84 ships HMAC-SHA256 chained security events to the Windows Application
Event Log. Every blocked/denied confinement action (path-deny, network-deny,
hook fail-closed) is emitted as a structured `SecurityEvent` with a running
`ChainHead` field computed as `HMAC-SHA256(session_key, prev_head || event_bytes)`.
This provides ordering and continuity proof within a single nono session.

This ADR records the exact scope of this tamper-evidence guarantee and what it does
NOT cover, satisfying D-07 (ADR required before Phase 84 closes).

---

## Decision

### 1. Tamper boundary = Windows Event Forwarding

The tamper-evidence claim for Phase 84 is: once security events are forwarded by a
Windows Event Forwarding (WEF) subscription or SIEM agent (Splunk Universal
Forwarder, Microsoft Sentinel MMA/AMA) to an external SIEM collector, the forwarded
copy is outside the reach of a locally-compromised host. A local attacker who gains
admin access after events have been forwarded cannot retroactively alter or delete the
SIEM copy.

This is the correct and honest claim. The Application Event Log itself is NOT
tamper-evident in isolation — it is a forwarding source, not a tamper-proof store.

### 2. In-session HMAC chain only

The HMAC key is a per-session `OsRng`-generated ephemeral secret (32 bytes, held in
`Zeroizing<[u8;32]>`). It is never persisted to disk, registry, or the Event Log
payload. The chain proves:

- **Intra-session event ordering**: events within one `nono` session form a verifiable
  sequence. A gap in `ChainHead` values indicates a missed or dropped event.
- **Intra-session continuity**: the SIEM can detect that events are sequential and
  unmodified within the session window.

On session end, the key is zeroized (via `Zeroizing<T>::drop()`). No cross-session
continuity is provided: each new `nono` invocation starts a fresh chain with a new
ephemeral key. A gap between sessions is expected and is NOT a security signal.

### 3. What is NOT guaranteed

The following are explicitly NOT guaranteed by the Phase 84 implementation:

**(a) Local admin log-clear:** A local administrator can clear the Application Event
Log at any time using `wevtutil cl Application` (or via Event Viewer). This removes
events before they have been forwarded. Events cleared this way produce no forensic
trace in the Application Log itself. Mitigation: configure WEF subscriptions to
forward events with minimal latency.

**(b) Local admin process substitution:** A local admin with write access to the
`nono.exe` binary path could replace nono with a modified binary that produces events
with a fresh HMAC key but arbitrary `EventType`/`PathHash` fields. The SIEM would see
a new key (a fresh session) and a gap but cannot distinguish intentional injection
from a normal session restart without an external integrity anchor (e.g. signed binary
attestation).

**(c) Cross-session chain continuity:** The HMAC key is zeroized at session end.
There is no cross-session chain head that would allow a SIEM to detect whether events
from two adjacent `nono` sessions are contiguous.

**(d) Cryptographic-local tamper anchoring:** TPM PCR-sealing of the HMAC key,
remote attestation of the key material, or HSM-backed key management are out of scope
for Phase 84. See SEED-005 (future milestone) for cryptographic-local anchoring work.

---

## Consequences

- **Fleet deployments SHOULD configure WEF subscriptions** to forward the Application
  log to a SIEM collector to achieve effective tamper-evidence. Without WEF, the
  HMAC chain provides intra-session ordering only and a local admin can clear the log
  undetected.

- **Documentation MUST use accurate language.** Do NOT claim "cryptographically
  tamper-proof audit log." Correct claim:
  > "Structured, HMAC-sequenced security events with intra-session ordering proof,
  > tamper-evident via Windows Event Forwarding to an external SIEM."

- **SEED-005** is the correct milestone for cryptographic-local anchoring (TPM
  PCR-sealed key or remote key management). Phase 84 explicitly defers that work.

- **The HMAC chain is an analyst aid, not an enforcement control.** Telemetry failure
  (e.g., Application log source unavailable, HMAC key generation error) degrades
  loudly to stderr and does NOT block confined agent execution (D-03/D-14). Confinement
  enforcement is unaffected by telemetry availability.

---

## References

- TELEM-02: HMAC tamper-evidence chain requirement
- D-05: Ephemeral per-session HMAC key (OsRng, zeroize on drop)
- D-06: Independent keyed chain in `telemetry/`, separate from `audit_integrity.rs`
- D-07: ADR required (this document)
- SEED-005: Future cryptographic-local tamper anchoring (cross-session continuity,
  TPM PCR sealing, remote attestation)
- Pitfall 12: Local HMAC key in HKLM is deletable by local admin — the reason the
  tamper boundary is WEF/external SIEM rather than a local key store
- `crates/nono-cli/src/telemetry/mod.rs`: `ChainState`, `advance_chain`,
  `SecurityEventLayer`
- `crates/nono-cli/src/telemetry/windows.rs`: `write_security_event_log`,
  `emit_security_event`
