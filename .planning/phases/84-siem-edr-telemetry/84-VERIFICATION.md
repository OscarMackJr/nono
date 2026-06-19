---
phase: 84-siem-edr-telemetry
verified: 2026-06-19T00:00:00Z
status: gaps_found
score: 3/5 must-haves verified
overrides_applied: 0
gaps:
  - truth: "SC-1/SC-5 dark-factory gate (telemetry-event-emit.ps1) can reach PASS verdict on a path-deny event"
    status: failed
    reason: "Gate hard-codes 'Host' in RequiredJsonFields but SecurityEvent.host is Option<String> with #[serde(skip_serializing_if = \"Option::is_none\")] — None for path_deny so the field is absent from JSON; gate loop returns FAIL on missing 'Host' before SC-3/SC-5 execute"
    artifacts:
      - path: "scripts/gates/telemetry-event-emit.ps1"
        issue: "Line 67: $script:RequiredJsonFields = @('EventType','AgentPid','PathHash','Host','SessionId','ChainHead') — 'Host' unconditionally required"
      - path: "crates/nono-cli/src/telemetry/event.rs"
        issue: "Lines 234-235: pub host: Option<String> with #[serde(skip_serializing_if = \"Option::is_none\")] — None for path_deny, so 'Host' key is absent from JSON body"
    missing:
      - "Make RequiredJsonFields event-type-conditional: universal fields are EventType/AgentPid/SessionId/ChainHead; 'PathHash' only for path_deny; 'Host' only for network_deny"
      - "Or: drop skip_serializing_if on host/path_hash so all six SC-1 columns are always literally present as null"

  - truth: "TELEM-04 'config read from machine policy' — admin TelemetryEnabled=0 opt-out has runtime effect"
    status: failed
    reason: "init_tracing(&cli, None) is hardcoded in main.rs (line 165) with an explicit comment acknowledging the None. MachineEgressPolicy (which now contains TelemetryConfig) is read later in app_runtime but the TelemetryConfig is never plumbed back into the already-constructed SecurityEventLayer. The 'if !inner.config.enabled { return; }' guard in on_event is structurally dead — config.enabled is always true because TelemetryConfig::default() is always used. Three D-12 knobs (enabled, channel, min_severity) are all inert."
    artifacts:
      - path: "crates/nono-cli/src/main.rs"
        issue: "Line 165: init_tracing(&cli, None) — explicit None, TelemetryConfig::default() always used"
      - path: "crates/nono-cli/src/cli_bootstrap.rs"
        issue: "Line 120: let config = telemetry_config.unwrap_or_default() — always unwraps to default because main.rs never passes Some(...)"
      - path: "crates/nono-cli/src/telemetry/mod.rs"
        issue: "Line 234: if !inner.config.enabled { return; } — dead branch; config.enabled is always true"
      - path: "crates/nono-cli/src/bin/nono-agentd.rs"
        issue: "No SecurityEventLayer registration at all; daemon path has zero telemetry wiring (daemon security events never emitted)"
    missing:
      - "Read MachineEgressPolicy (or at minimum its telemetry sub-section) before or at tracing init and pass Some(policy.telemetry) into init_tracing"
      - "If policy read must occur later in app_runtime, expose a one-time setter on SecurityEventLayer that swaps config behind the existing Mutex, then call it after the policy read"
      - "Wire the daemon path (nono-agentd.rs) with SecurityEventLayer registration symmetrically"

human_verification:
  - test: "Trigger a path-deny event on a fully provisioned host (MSI installed, nono source registered, nono on PATH) and run verify-dark.ps1 --gate telemetry-event-emit"
    expected: "Gate reaches PASS; requires CR-01 fix first (Host field conditional logic)"
    why_human: "Requires Windows host with MSI install and registry source registration; cannot execute in CI or static analysis"
  - test: "Set HKLM\\SOFTWARE\\Policies\\nono\\Telemetry\\TelemetryEnabled=0 (DWORD), run a nono confinement command triggering a denial, and inspect whether a security event appears in the Application log"
    expected: "No event should appear (admin opt-out honored); currently events still emit because WR-01 makes config.enabled permanently true"
    why_human: "Requires Windows registry write access and live event log inspection; verifies WR-01 is fixed end-to-end"
---

# Phase 84: SIEM/EDR Telemetry Verification Report

**Phase Goal:** Every blocked or denied action (path-deny, network-deny, label-violation, hook fail-closed) is emitted as a structured security event to the Windows Application Event Log with named EventData fields, HMAC-chained within the session for tamper-evidence, and scrubbed of secrets and full paths — readable by Splunk and Microsoft Sentinel without custom parsers

**Verified:** 2026-06-19
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (mapped from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | Application Event Log entry with EventID 10001-10005 and named EventData fields (EventType, AgentPid, PathHash, Host, SessionId, ChainHead) after a sandbox denial | PARTIAL | Schema correct, emitter wired — but gate cannot prove it (CR-01 blocks PASS) |
| SC-2 | ChainHead present in each event; ADR records tamper boundary = WEF; SEED-005 deferral documented | VERIFIED | HMAC chain implemented in telemetry/mod.rs; docs/adr/telemetry-tamper-evidence.md exists and is substantive |
| SC-3 | No raw file path, full URL, or credential value in event body | VERIFIED | path_hash_for() confirmed; skip_serializing_if on host/path_hash prevents raw path in JSON; scrub_value applied to host field in on_event |
| SC-4 | Emitter is tracing::Layer in nono-cli/src/telemetry/; reads channel/level config from machine policy | FAILED | Layer is in nono-cli (VERIFIED). Config reading from machine policy is NOT wired — init_tracing is always called with None; TelemetryConfig::default() is permanently used |
| SC-5 | verify-dark.ps1 --gate telemetry-event-emit emits PASS verdict | FAILED | Gate structurally cannot PASS: RequiredJsonFields includes 'Host' but path_deny events emit Host=None which is omitted from JSON by skip_serializing_if; gate returns FAIL on field-check before ETW assertion runs |

**Score:** 2/5 truths fully verified (SC-2 and SC-3); SC-1 has partial implementation evidence but no passing gate; SC-4 and SC-5 have BLOCKER defects

### Deferred Items

None — all gaps are in-scope for Phase 84 and must be fixed before closing.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/telemetry/mod.rs` | SecurityEventLayer (tracing::Layer) + ChainState | VERIFIED | Exists, substantive, registered in init_tracing |
| `crates/nono-cli/src/telemetry/event.rs` | SecurityEvent schema + SecurityEventType + PathCategory | VERIFIED | Exists, substantive; all 5 EventID mappings correct; serde PascalCase confirmed |
| `crates/nono-cli/src/telemetry/windows.rs` | ReportEventW + ETW dual-emit | VERIFIED | Exists; RegisterEventSourceW/ReportEventW wired; ETW via tracing::warn!(target: "nono_security") |
| `crates/nono-cli/src/telemetry/syslog.rs` | cfg(unix) stub with TODO(TELEM-FU-01) | VERIFIED | Exists per code review; TELEM-FU-01 comment present |
| `scripts/gates/telemetry-event-emit.ps1` | Dark-factory gate asserting SC-1/SC-3/SC-5 | STUB | File exists and has correct two-function structure, but gate is structurally broken on its happy-path event type (CR-01) |
| `docs/adr/telemetry-tamper-evidence.md` | Tamper-evidence ADR (TELEM-02) | VERIFIED | Exists; documents WEF tamper boundary, SEED-005 deferral, in-session HMAC scope honestly |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| main.rs | init_tracing | SecurityEventLayer construction | PARTIAL | Layer constructed and registered; but always with TelemetryConfig::default() — policy config never flows in |
| init_tracing | MachineEgressPolicy.telemetry | None parameter | NOT_WIRED | main.rs line 165: hardcoded None; policy read in app_runtime never back-wires config |
| exec_strategy.rs | SecurityEventLayer | tracing::warn!(target:"nono_security::path_deny") | WIRED | Line 1795 confirmed; path field forwarded to visitor |
| nono-proxy/audit.rs | SecurityEventLayer | tracing::warn!(target:"nono_security::network_deny") | WIRED | Line 204 confirmed |
| hooks.rs | SecurityEventLayer | tracing::warn!(target:"nono_security::hook_fail_closed") | WIRED | Lines 115/120 confirmed |
| nono-agentd.rs | SecurityEventLayer | (no path) | NOT_WIRED | Daemon binary has no init_tracing call and no SecurityEventLayer registration; daemon denials not telemetered |
| gate RequiredJsonFields | SecurityEvent.host | skip_serializing_if = "Option::is_none" | BROKEN | Gate requires 'Host' always; schema omits it for path_deny events; contradiction is a structural FAIL |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| SecurityEventLayer.on_event | security_event | advance_chain + path_hash_for + scrub_value + visitor fields | Yes — real denial event data from tracing fields | FLOWING |
| SecurityEventLayer config.enabled | TelemetryConfig.enabled | TelemetryConfig::default() hardwired | No — always true regardless of HKLM policy | HOLLOW — policy config disconnected |
| gate RequiredJsonFields['Host'] | parsed.Host from Application log JSON | SecurityEvent.host (Option<None> for path_deny) | No — field absent from JSON, gate returns FAIL | DISCONNECTED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| SecurityEvent serializes without 'Host' for path_deny | Static code analysis: serde skip_serializing_if on host field | host=None for path_deny; JSON omits key | VERIFIED — field correctly absent |
| Gate field loop on absent 'Host' | Static code analysis: lines 245-264 of gate | `if ($null -eq $fieldValue) { return FAIL }` fires on absent Host | FAIL — structurally confirmed by reading gate + schema |
| init_tracing receives policy config | Static code analysis: main.rs line 165 | `init_tracing(&cli, None)` — policy never flows in | FAIL — WR-01 confirmed |
| Daemon registers SecurityEventLayer | Static code analysis: nono-agentd.rs | No init_tracing or SecurityEventLayer call found | FAIL — daemon path untelemetered |

### Probe Execution

No probe scripts declared in plans. The gate `scripts/gates/telemetry-event-emit.ps1` is the phase verification artifact. Live execution requires MSI install and elevated Windows host — cannot execute in this static verification pass. The 84-04 SUMMARY documents a FAIL result on the dev host (exit 2, not exit 4), attributed to "stale MSI binary." Independent analysis shows the gate would fail even on a perfectly provisioned host because of the CR-01 schema/gate mismatch, separate from any binary staleness issue.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| TELEM-01 | 84-01 through 84-04 | Blocked/denied actions emitted as structured security events to Application-tier Windows Event Log with distinct EventIDs + named EventData fields | PARTIAL | Implementation exists and emits correctly; gate cannot prove it due to CR-01; REQUIREMENTS.md marks Complete |
| TELEM-02 | 84-01, 84-02 | In-session HMAC-SHA256 chain (ChainHead field); tamper boundary documented as WEF; ADR recorded | SATISFIED | HMAC chain is real (Hmac<Sha256>, Zeroizing key); ADR at docs/adr/telemetry-tamper-evidence.md is complete and honest |
| TELEM-03 | 84-01, 84-02 | Redacts secrets/tokens/paths — no credential or raw path leaks into log fields | SATISFIED | path_hash_for confirmed; scrub_value applied to host; skip_serializing_if prevents accidental raw path emission; SC-3 gate logic correct |
| TELEM-04 | 84-01, 84-02 | Emitter is tracing::Layer in nono-cli; config read from machine policy | PARTIALLY SATISFIED | Layer boundary is correct (telemetry/ in nono-cli, not in nono lib). Config read from machine policy is NOT wired — TelemetryConfig::default() is permanently used; admin opt-out has no effect |

REQUIREMENTS.md marks all four TELEM requirements as Complete. This is inaccurate: TELEM-04 ("config read from machine policy") is not wired end-to-end, and TELEM-01's SC-5 gate deliverable cannot pass.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono-cli/src/main.rs` | 163-165 | `init_tracing(&cli, None)` with comment acknowledging the disconnect | BLOCKER | Admin TelemetryEnabled=0 opt-out has no effect; D-12 knobs (enabled, channel, min_severity) all inert at runtime |
| `scripts/gates/telemetry-event-emit.ps1` | 67 | `'Host'` in `$script:RequiredJsonFields` unconditionally | BLOCKER | Gate cannot PASS on a path_deny event, which is the only event it triggers; SC-5 assertion never executes |
| `crates/nono-cli/src/telemetry/mod.rs` | 234-236 | `if !inner.config.enabled { return; }` | BLOCKER (dead code) | Guard exists but is structurally unreachable because config.enabled is always true |
| `crates/nono-cli/src/bin/nono-agentd.rs` | entire | No init_tracing or SecurityEventLayer registration | WARNING | Daemon-launched confined agents produce zero telemetry; TELEM-04 contract says "nono-cli emitter"; daemon is a separate binary not covered |
| `crates/nono-cli/Cargo.toml` | eventlog dep | `eventlog = "0.4"` declared but no `eventlog::` call sites | WARNING | Unused supply-chain dependency (WR-04); windows.rs uses windows-sys directly |
| `crates/nono-cli/src/telemetry/windows.rs` | 189-200 | `path_hash = ?event.path_hash` and `host = ?event.host` | INFO | Debug format of Option renders as `Some("...")` / `None` in ETW output, inconsistent with clean JSON body |
| `crates/nono-cli/src/telemetry/mod.rs` | 113-133 | `HmacSha256::new_from_slice(&[0u8;32])` in InvalidLength degrade | WARNING | Latent: structurally unreachable today but chosen fallback (all-zero key) would make chain forgeable; no TelemetryDegraded event emitted on this path |

## Human Verification Required

### 1. Full PASS on provisioned host (after CR-01 fix)

**Test:** Apply the CR-01 fix (conditional RequiredJsonFields by EventType), rebuild gate, MSI-install fresh nono binary, run `pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit` from an elevated shell
**Expected:** Verdict PASS with sc1Pass=true, sc3Pass=true, sc5Pass=true; event body excerpt shows EventType/AgentPid/PathHash/SessionId/ChainHead present; no raw path string
**Why human:** Requires Windows host with MSI registration of nono Application Event Log source; ETW provider only appears after nono process has registered it; cannot be verified statically

### 2. Admin TelemetryEnabled=0 opt-out (after WR-01 fix)

**Test:** Set `HKLM\SOFTWARE\Policies\nono\Telemetry\TelemetryEnabled = 0` (DWORD 0), run `nono run --profile claude-code -- cmd /c type C:\Windows\System32\config\SAM`, then `Get-WinEvent -FilterHashtable @{LogName='Application';ProviderName='nono';StartTime=(Get-Date).AddMinutes(-1)}`
**Expected:** Zero events returned — telemetry suppressed by admin policy
**Why human:** Requires live Windows environment with registry write access and Application Event Log query; verifies the WR-01 fix is wired end-to-end through the policy read path

## Gaps Summary

**Two structural blockers prevent phase goal achievement:**

**BLOCKER 1 — CR-01: Gate cannot prove SC-1 (its own primary deliverable)**

The phase's SC-5 / SC-1 verification artifact (`telemetry-event-emit.ps1`) is designed to trigger a path-deny event (EventID 10001, `host=None`) and assert that six named fields are present. The gate's `$script:RequiredJsonFields` includes `'Host'` unconditionally. The Rust schema emits `Host` only for network-deny events (the field is `Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]`). For path-deny events — the only type the gate triggers — `host` is `None`, the JSON key is absent, and the gate's field-check loop returns `FAIL: required field 'Host' is missing`. This is not a binary-staleness artifact (as 84-04-SUMMARY claims); it is a structural schema/gate mismatch that survives any rebuild. Fix: make `RequiredJsonFields` event-type-conditional, or drop `skip_serializing_if` on `host`/`path_hash` so all six columns are always present as explicit JSON null.

**BLOCKER 2 — WR-01: TELEM-04 "config read from machine policy" is wired decoratively**

`init_tracing` accepts an `Option<TelemetryConfig>` second argument intended to carry the HKLM-read policy. Every call site passes `None` (main.rs line 165, acknowledged in the comment). `SecurityEventLayer` is always constructed from `TelemetryConfig::default()`. The three D-12 knobs — `enabled`, `channel`, `min_severity` — are stored in `SecurityEventLayerInner.config` but never updated after construction. An admin setting `TelemetryEnabled=0` to suppress telemetry fleet-wide has no runtime effect. The daemon path (nono-agentd.rs) has no telemetry registration at all. Fix: read `MachineEgressPolicy` before or at tracing init and pass `Some(policy.telemetry)` into `init_tracing`; or provide a one-time setter callable after the policy read in app_runtime.

These two blockers mean: the phase cannot demonstrate its own SC-5 PASS requirement on any host, and the SC-4 contract ("config read from machine policy") is not structurally true. TELEM-01 and TELEM-04 in REQUIREMENTS.md should not be marked Complete until both gaps are closed.

**Non-blocking warnings (should be addressed but do not block SC verification):**
- WR-02: `min_severity` filtering not implemented — severity config is parsed but never consulted in `on_event`
- WR-03: `channel` config ignored — always writes to hardcoded Application source regardless of HKLM setting
- WR-04: `eventlog = "0.4"` dependency is unused (windows-sys used directly instead)
- WR-05: HMAC degrade path uses zeroed key rather than regenerating from OsRng or emitting TelemetryDegraded
- WR-06: `access` and `port` fields forwarded by denial call sites are silently dropped by the visitor

---

_Verified: 2026-06-19_
_Verifier: Claude (gsd-verifier)_
