# Phase 84: SIEM/EDR Telemetry - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-18
**Phase:** 84-siem-edr-telemetry
**Areas discussed:** Event Log surface & named fields, HMAC chain (key & reuse), Redaction & field schema, Telemetry config & fail behavior

---

## Event Log Surface & Named Fields

| Option | Description | Selected |
|--------|-------------|----------|
| Dual-emit: ETW (named) + Application (anchor) | tracing-etw self-describing named fields (logman/SC-5) + classic Application-log entry on the Phase-82 nono source carrying EventID + payload (SC-1). No manifest, no wevtutil. | ✓ |
| Application-only, JSON-packed Data | Single ReportEventW; fields packed as one structured payload; SIEM-side JSON extraction. | |
| Ship a compiled manifest in MSI | nono.man → mc.exe/rc.exe → wevtutil im for native named EventData + custom channel. | |

**User's choice:** Dual-emit.
**Notes:** Resolves the SC-1 (named columns, no custom parser) vs SC-5 (ETW/logman) vs PITFALLS-10 (named cols need manifest) tension without an MSI manifest. → CONTEXT D-01.

### Follow-up: Application-log payload format

| Option | Description | Selected |
|--------|-------------|----------|
| One JSON object per event | Single insertion string = compact JSON; spath/parse_json extract columns. | ✓ |
| Multiple insertion strings (one per field) | Positional Data/Data_1/... in XmlWinEventLog; order-fragile. | |
| key=value lines | Newline-separated pairs; needs SIEM field extraction. | |

**User's choice:** One JSON object per event. → CONTEXT D-02.

### Follow-up: NULL RegisterEventSourceW behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Log NonoError to stderr, continue | TelemetryUnavailable to stderr (no silent drop), run continues. | ✓ |
| Fall back to ETW only | Rely on ETW provider alone; note degraded once. | |
| Fail the run | Abort if security event sink unavailable. | |

**User's choice:** Log NonoError to stderr, continue. → CONTEXT D-03.

---

## HMAC Tamper-Evidence Chain

| Option | Description | Selected |
|--------|-------------|----------|
| Ephemeral random per session | 32-byte OsRng key, zeroized, never persisted; honest in-session/WEF scope. | ✓ |
| Derived from session context | KDF from session id + machine secret; introduces key-management story (edges toward SEED-005). | |
| You decide | Defer to research. | |

**User's choice:** Ephemeral random per session. → CONTEXT D-05.

### Follow-up: Relationship to existing audit_integrity.rs chain

| Option | Description | Selected |
|--------|-------------|----------|
| New independent keyed chain in telemetry/ | Fresh HMAC-SHA256 chain in SecurityEventLayer; borrow domain-separator discipline, not code. | ✓ |
| Extend audit_integrity to keyed HMAC | Unify both subsystems on one chain; risks regressing audit-verify fixtures. | |
| You decide | Defer to research. | |

**User's choice:** New independent keyed chain. → CONTEXT D-06.

---

## Redaction & Field Schema

| Option | Description | Selected |
|--------|-------------|----------|
| Per-session salted SHA-256 | SHA-256(session_salt ‖ canonical_path); intra-session correlation, cross-session rainbow-resistant. | ✓ |
| Unsalted SHA-256 | Stable fleet-wide correlation but rainbow-table-able. | |
| Keyed HMAC path tag | HMAC(session_key, path); strongest, zero cross-session correlation. | |

**User's choice:** Per-session salted SHA-256 (PathHash). → CONTEXT D-08.

### Follow-up: Category taxonomy

| Option | Description | Selected |
|--------|-------------|----------|
| Sensitivity-tiered set | workspace_file, system_path, credential_path, user_home, temp, other. | ✓ |
| Minimal (workspace vs non-workspace) | Just two tags. | |
| You decide | Defer to research. | |

**User's choice:** Sensitivity-tiered set. → CONTEXT D-09.

### Follow-up: Cleartext vs hashed fields

| Option | Description | Selected |
|--------|-------------|----------|
| Host cleartext; path hashed; scrub all values | Host kept (SC-1 column); full URLs/paths never emitted; nono::scrub on free text. | ✓ |
| Hash Host too | Maximally conservative; cripples network-deny triage. | |
| You decide | Defer to planning. | |

**User's choice:** Host cleartext; path hashed; scrub all values. → CONTEXT D-10.

---

## Telemetry Config & Fail Behavior

| Option | Description | Selected |
|--------|-------------|----------|
| enabled + channel + min_severity, default ON | Three knobs in MachineEgressPolicy; absent key → enabled→Application. | ✓ |
| enabled only, default ON | Single bool; always Application, all severities. | |
| enabled + channel + min_severity, default OFF | Same knobs but emit nothing until admin enables (would fail clean-host gate). | |

**User's choice:** enabled + channel + min_severity, default ON. → CONTEXT D-12/D-13.

### Follow-up: Malformed telemetry config behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Degrade telemetry, log loudly, run continues | Fall back to safe defaults; TelemetryConfigInvalid to stderr + self-describing degraded event; confinement unaffected. | ✓ |
| Abort like egress (fail-secure) | Treat identically to malformed egress; couples compliance config to run availability. | |
| You decide | Defer to planning. | |

**User's choice:** Degrade telemetry, log loudly, run continues. → CONTEXT D-14.

---

## Claude's Discretion

- Exact `ChainHead` construction + genesis seeding (D-05/D-06 principles hold).
- `SecurityEvent` enum/struct shape and canonical byte serialization for HMAC + PathHash.
- Whether ETW and Application-log writers share one serialization pass.
- `NonoError` variant shapes (`TelemetryUnavailable`, `TelemetryConfigInvalid`).
- `PathHash` truncation length; exact OsRng/key-size within "32-byte ephemeral, zeroized."
- Final category-enum membership beyond the D-09 tiers (source from existing path classification).

## Deferred Ideas

- Compiled custom Event Log channel manifest (`nono/Security`) — future "SIEM schema hardening" milestone (PITFALLS-10 defer).
- RFC 5424 Syslog emission (TELEM-FU-01).
- Real-time direct SIEM API push (TELEM-FU-02).
- Cross-session / cryptographic-local tamper anchoring (SEED-005 / ATTEST-02).
- Unifying telemetry chain with audit_integrity.rs (rejected D-06).
