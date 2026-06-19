# Phase 84: SIEM/EDR Telemetry - Context

**Gathered:** 2026-06-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Emit every blocked/denied confinement action (path-deny, network-deny, label-violation, hook fail-closed) as a **structured, secret-scrubbed, in-session-HMAC-chained** security event to Windows telemetry sinks, readable by Splunk (`XmlWinEventLog`) and Microsoft Sentinel without custom parsers. The emitter is a `tracing::Layer` (`SecurityEventLayer`) in `nono-cli/src/telemetry/`, registered in `init_tracing()`, with its enable/channel/level config read from the **same single `MachineEgressPolicy` HKLM read** that Phase 83 already performs.

Requirements covered: TELEM-01, TELEM-02, TELEM-03, TELEM-04.

**In scope:** new `nono-cli/src/telemetry/` module (SecurityEventLayer + SecurityEvent schema + Windows emitter); dual emission (ETW TraceLogging + classic Application Event Log anchor); structured EventData with named fields + distinct EventIDs (10001-10005); in-session HMAC-SHA256 tamper-evidence chain (`ChainHead`); secret/token redaction + path hashing + category tagging; telemetry config section in `MachineEgressPolicy`; tamper-evidence ADR; `verify-dark.ps1 --gate telemetry-event-emit`.

**Out of scope (other phases / deferred):** RFC 5424 Syslog emission (TELEM-FU-01); real-time direct SIEM API push (TELEM-FU-02); cross-session / cryptographic-local tamper anchoring (SEED-005 / ATTEST-02); any change to the `nono` library's `DiagnosticFormatter` (library/CLI boundary — telemetry is CLI-only); a compiled custom Event Log channel manifest (PITFALLS-10 defer; revisit in a future "SIEM schema hardening" milestone).

</domain>

<decisions>
## Implementation Decisions

### Event Log Surface & Named Fields (TELEM-01, SC-1, SC-5)
- **D-01: Dual-emit.** Emit each security event via **two** surfaces from the one `SecurityEventLayer`:
  1. **ETW TraceLogging** (`tracing-etw`) — self-describing **named fields**, no manifest, detectable via `logman` (satisfies SC-5 + the "named columns without custom parser" intent of SC-1).
  2. **Classic Application Event Log** entry on the **Phase-82-registered `nono` source** (`SYSTEM\CurrentControlSet\Services\EventLog\Application\nono`, `EventMessageFile=nono.exe`, `TypesSupported=7`) — the human-visible anchor carrying the distinct EventID (satisfies SC-1's "entry in the Windows Application Event Log under the nono source"). No `wevtutil im`, no custom channel manifest this cycle.
- **D-02: Application-log payload = one JSON object per event.** The single `ReportEventW` insertion string is a compact JSON object (`{EventType, AgentPid, PathHash, Host, SessionId, ChainHead, ...}`). Splunk `spath` / Sentinel `parse_json` extract columns reliably; self-contained and evolvable without a manifest.
- **D-03: NULL `RegisterEventSourceW` → loud, non-fatal.** If the Application source is unavailable (dev/test or broken install), surface `NonoError::TelemetryUnavailable` to **stderr** (no silent drop — PITFALLS-10) and continue the confined run. Telemetry is compliance, not a confinement control, so a failed sink must NOT block the agent.
- **D-04:** The `telemetry-event-emit` dark-factory gate asserts BOTH surfaces: (a) Application-log entry under the `nono` source with the correct EventID + named JSON fields and **no raw path strings**; (b) ETW provider emission detectable via `logman`.

### HMAC Tamper-Evidence Chain (TELEM-02, SC-2)
- **D-05: Ephemeral per-session HMAC key.** Generate a random key at session start (e.g. 32 bytes from `OsRng`), hold in memory, **zeroize on drop**, never persist. The chain proves intra-session ordering/continuity; honest with the documented scope — the real tamper boundary is **Windows Event Forwarding** (external copy out of local attacker reach). Cross-session / cryptographic-local anchoring is explicitly deferred to SEED-005 (recorded in the ADR).
- **D-06: New independent keyed chain in `telemetry/`.** Build a fresh HMAC-SHA256 chain inside the `SecurityEventLayer`, **separate from `audit_integrity.rs`** (which is an *unkeyed* SHA-256 NDJSON ledger with a different lifetime/trust model). Borrow the **domain-separator discipline** (e.g. `nono.telemetry.chain.alpha`) but not the code — avoids regressing the existing audit-verify fixtures.
- **D-07: ADR required.** The phase ships a tamper-evidence ADR recording: (a) tamper boundary = Windows Event Forwarding; (b) in-session HMAC only; (c) cross-session/crypto-local anchoring deferred to SEED-005.

### Redaction & Field Schema (TELEM-03, SC-1, SC-3)
- **D-08: `PathHash` = per-session salted SHA-256.** `PathHash = SHA-256(session_salt || canonical_path)` (truncated). Same path → same hash **within** a session (analysts correlate repeated denials on one file); hashes differ across sessions → resists precomputed rainbow tables. The salt is the ephemeral per-session value (may reuse the D-05 session entropy).
- **D-09: Sensitivity-tiered category enum** replaces the raw path: `workspace_file`, `system_path`, `credential_path` (e.g. `.ssh`/`.aws`/keystore paths), `user_home`, `temp`, `other`. Gives analysts signal ("a credential path was touched") without the literal path. (Planner: derive from existing nono path-classification code where possible.)
- **D-10: Field treatment.** **Host/domain stays cleartext** (SC-1 names `Host` as a parseable column; it's the denied destination an analyst needs). **Full URLs and file paths are NEVER emitted** — only `PathHash` + category. Every free-text field is run through **`nono::scrub`** (`ScrubPolicy` / `scrub_value`) to strip tokens/secrets before emit.
- **D-11: Named EventData fields (SC-1 minimum):** `EventType, AgentPid, PathHash, Host, SessionId, ChainHead`. Distinct EventIDs **10001-10005** (locked by ROADMAP) map to the event types.

### Telemetry Config in Machine Policy (TELEM-04, SC-4)
- **D-12: Three knobs, same single read.** Extend `MachineEgressPolicy` (Phase 83, core lib type) with a telemetry section: `telemetry_enabled` (bool), `telemetry_channel` (REG_SZ, default `"Application"`), `telemetry_min_severity` (level). Read from the **same single HKLM deserialization** Phase 83 performs — **no second registry read** (the Phase 83 deferred-note carry-forward).
- **D-13: Default ON when key absent.** When the HKLM policy key is ABSENT, default to **enabled → Application log** (security telemetry on by default; admins opt OUT or redirect). A default-OFF would make a clean-host MSI install emit nothing and FAIL the SC-1/SC-5 gate.
- **D-14: Malformed telemetry config DEGRADES, does not abort.** Unlike egress policy (which aborts fail-secure per Phase 83 D-07), a malformed telemetry sub-section falls back to safe defaults (enabled → Application), emits `NonoError::TelemetryConfigInvalid` to **stderr** PLUS a self-describing **"telemetry degraded" security event** so the gap is itself auditable (no silent drop). Confinement enforcement is unaffected. Rationale: a typo in a telemetry REG value must not brick confined runs fleet-wide.

### Claude's Discretion
- Exact `ChainHead` construction (recommended: running `HMAC(key, prev_head ‖ canonical_event_bytes)`; genesis seeding from session id or a fixed IV) — D-05/D-06 principles hold regardless.
- Precise `SecurityEvent` enum/struct shape and the canonical byte serialization fed to the HMAC and to `PathHash`.
- Whether the ETW provider and the Application-log writer share a single serialization pass or format independently from the same `SecurityEvent`.
- Exact `NonoError` variant shapes (`TelemetryUnavailable`, `TelemetryConfigInvalid`).
- Truncation length for `PathHash`; exact `OsRng`/key-size choice within "32-byte ephemeral, zeroized."
- Final category-enum membership beyond the D-09 tiers, sourced from existing path classification.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 84 section: goal + 5 success criteria (the prescriptive contract; EventIDs 10001-10005 + named-field list are locked here).
- `.planning/REQUIREMENTS.md` §TELEM-01..04 (+ §TELEM-FU-01/02, §ATTEST-02 deferrals, SEED-003 source).

### Milestone research (HIGH confidence, telemetry-specific)
- `.planning/research/ARCHITECTURE.md` — "Feature 3: Structured Security-Event Telemetry": `nono-cli/src/telemetry/` layout (`mod.rs`/`event.rs`/`windows.rs`), the `nono_security::*` target-prefix tracing::Layer approach, DenialRecord wiring after child exit, EventID map.
- `.planning/research/PITFALLS.md` — Pitfall 10 (custom-channel manifest requires admin + `wevtutil im` → use the Application source instead; treat NULL `RegisterEventSourceW` as a logged error not a silent drop) and Pitfall 11 (secret/path leakage into Event Log → hash paths, scrub values).
- `.planning/research/STACK.md` — locked dep delta: `tracing-etw` 0.2.3, `eventlog` 0.4.0, `hmac` 0.13.0; **no `windows-sys` version bump**.

### Existing code to wire into / reuse
- `crates/nono-cli/src/cli_bootstrap.rs` §`init_tracing` (~L87) — where `SecurityEventLayer` registers (TELEM-04).
- `crates/nono-cli/src/bin/nono-wfp-service.rs` §`write_event_log` (~L162) — the proven `RegisterEventSourceW`/`ReportEventW` pattern + existing EventID constants (1001-1004) to mirror; uses `EVENT_LOG_SOURCE`.
- `crates/nono/src/scrub.rs` — `ScrubPolicy`, `scrub_value`, `scrub_value_with_policy`, `scrub_argv` (TELEM-03 redaction engine; reuse, don't rebuild).
- `crates/nono-cli/src/audit_integrity.rs` — existing **unkeyed** SHA-256 chain (leaf/chain/Merkle, domain separators `nono.audit.*.alpha`). Reference for chain discipline ONLY; the telemetry HMAC chain is independent (D-06).
- `crates/nono/src/diagnostic.rs` — `DiagnosticFormatter` + `DenialRecord` (the path-deny source; telemetry wiring goes in the CLI caller AFTER child exit, NOT inside the formatter — boundary preserved).
- `crates/nono-proxy/src/audit.rs` — network-deny event source.
- The `MachineEgressPolicy` type (core lib, Phase 83 D-05) — extend with the telemetry config section (D-12).

### Deployment / install footprint (Phase 82 outputs)
- `scripts/build-windows-msi.ps1` ~L317-335 — the `cmpNonoCliEventLogSource` component that pre-registered the `nono` Application source "to de-risk Phase 84." (The `.wxs` is GENERATED from here-strings — edit the script, not the `.wxs`.)

### Dark Factory gate
- `scripts/verify-dark.ps1` + `scripts/gates/` — runner + two-function gate contract; Phase 84 adds `telemetry-event-emit` (clone an existing gate e.g. `scripts/gates/egress-policy-deny.ps1`).
- `Skill("spike-findings-nono")` — engine-agnostic confinement patterns + Windows landmines.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `nono::scrub` (ScrubPolicy / scrub_value / scrub_argv): existing secret/token redaction — the TELEM-03 engine.
- `nono-wfp-service.rs::write_event_log`: working `RegisterEventSourceW`+`ReportEventW`+`DeregisterEventSource` with the stderr fallback already implemented; the Application-log anchor (D-01.2) clones this shape with the `nono` source + JSON payload.
- Phase-82 pre-registered `nono` Application Event Log source — SC-1's "no prior `wevtutil im`" precondition is already satisfiable.
- `audit_integrity.rs` chain machinery (domain-separated SHA-256, `hash_chain`) — pattern reference for the new keyed chain.
- `init_tracing()` already builds the `tracing-subscriber` registry — `SecurityEventLayer` is an additional `Layer` on it.

### Established Patterns
- Library is policy/UX-free; the CLI owns output. Telemetry (output + policy-driven) lives entirely in `nono-cli` (TELEM-04, CLAUDE.md boundary).
- Windows-cfg-gating: keep ETW + Event Log emitters behind `#[cfg(target_os = "windows")]` with non-Windows stubs so cross-target clippy (Linux/macOS) compiles (CLAUDE.md cross-target MUST/NEVER rule).
- Fail-secure on confinement; **fail-loud-but-continue** on telemetry availability (D-03/D-14) — telemetry is compliance, not enforcement, but security events are never silently dropped.

### Integration Points
- `init_tracing()` → register `SecurityEventLayer` (reads telemetry config from `MachineEgressPolicy`).
- Denial sources → tracing events on `nono_security::*` target → `SecurityEventLayer` → (a) ETW provider, (b) Application-log JSON anchor. Sources: `DiagnosticFormatter`/`DenialRecord` (path-deny), `nono-proxy/audit.rs` (network-deny), label-violation + hook fail-closed paths.
- Single HKLM read (Phase 83 daemon/CLI startup) now also yields the telemetry config section (D-12).

</code_context>

<specifics>
## Specific Ideas

- Named EventData minimum (SC-1): `EventType, AgentPid, PathHash, Host, SessionId, ChainHead`. EventIDs 10001-10005 (locked).
- SC-3 acceptance: a blocked-action event for `C:\Users\alice\secret.txt` contains a hashed `PathHash` + category `workspace_file` (or `credential_path` if under `.ssh`/`.aws`), NOT the literal path — the gate greps the event body for raw path strings and must find none.
- SC-5: `telemetry-event-emit` gate proves emission on BOTH surfaces — Application log (EventID + named JSON fields, clean-host) AND ETW via `logman`.
- Tamper boundary statement for the ADR: "Windows Event Forwarding to an external SIEM; in-session HMAC only; cross-session/crypto-local anchoring deferred to SEED-005."

</specifics>

<deferred>
## Deferred Ideas

- **Compiled custom Event Log channel manifest** (`nono.man` → `mc.exe`/`rc.exe` → `wevtutil im` in MSI) for true native named EventData + a dedicated `nono/Security` channel — deferred per PITFALLS-10; revisit in a future "SIEM schema hardening" milestone. Dual-emit (D-01) achieves the named-field goal without it this cycle.
- **RFC 5424 Syslog emission** (TELEM-FU-01) — non-Windows / network SIEM ingestion. Out of scope (Windows Event Log only this cycle); the `telemetry/syslog.rs` slot sketched in research stays a stub.
- **Real-time direct SIEM API push** (TELEM-FU-02) — this cycle relies on Windows Event Forwarding / agent collection.
- **Cross-session / cryptographic-local tamper anchoring** (SEED-005 / ATTEST-02) — extends TELEM-02 beyond the WEF boundary; the ADR explicitly scopes it out.
- **Unifying the telemetry HMAC chain with `audit_integrity.rs`** — rejected (D-06) to avoid coupling two subsystems with different trust models/lifetimes; revisit only if a single audit chain becomes a hard requirement.

### Reviewed Todos (not folded)
None — no open todos matched Phase 84 scope (the v3.0 pending todos were Phase 82/83 deployment/cert concerns).

</deferred>

---

*Phase: 84-siem-edr-telemetry*
*Context gathered: 2026-06-18*
