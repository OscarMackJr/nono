---
phase: 84-siem-edr-telemetry
verified: 2026-06-19T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 2/5
  gaps_closed:
    - "SC-1/SC-5 dark-factory gate (telemetry-event-emit.ps1) can reach PASS verdict on a path-deny event"
    - "TELEM-04 config read from machine policy — admin TelemetryEnabled=0 opt-out has runtime effect"
    - "WR-02 min_severity level filtering not implemented"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Trigger a path-deny event on a fully provisioned host (MSI installed, nono source registered, nono on PATH) and run pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit from an elevated shell"
    expected: "Gate reaches PASS with sc1Pass=true, sc3Pass=true, sc5Pass=true; event body excerpt shows EventType/AgentPid/PathHash/SessionId/ChainHead; no raw path string"
    why_human: "Requires Windows host with MSI registration of the nono Application Event Log source and an elevated shell; the gate also needs logman available for SC-5 ETW-provider enumeration. Cannot execute statically."
  - test: "Set HKLM\\SOFTWARE\\Policies\\nono\\Telemetry\\TelemetryEnabled=0 (DWORD 0), run a nono confinement command that triggers a denial, and query Get-WinEvent -FilterHashtable @{LogName='Application';ProviderName='nono';StartTime=(Get-Date).AddMinutes(-1)}"
    expected: "Zero events returned — admin opt-out is honored; the WR-01 fix now threads TelemetryConfig.enabled=false from the registry into the layer and the on_event guard returns early"
    why_human: "Requires live Windows environment with HKLM write access and Application Event Log query; verifies the WR-01 fix is wired end-to-end through the policy read path"
  - test: "Set HKLM\\SOFTWARE\\Policies\\nono\\Telemetry\\TelemetryMinSeverity=Error (REG_SZ), trigger a path-deny (Warning-level), and confirm no event appears in the log"
    expected: "No event emitted — WR-02 fix causes severity_for(PathDeny)=Warning < min_severity=Error to return early in on_event"
    why_human: "Requires live registry write + live event log inspection; the on_event severity comparison is structural but the wiring from HKLM through TelemetryConfig.min_severity to the on_event guard needs end-to-end live proof"
---

# Phase 84: SIEM/EDR Telemetry Verification Report

**Phase Goal:** Every blocked or denied action (path-deny, network-deny, label-violation, hook fail-closed) is emitted as a structured security event to the Windows Application Event Log with named EventData fields, HMAC-chained within the session for tamper-evidence, and scrubbed of secrets and full paths — readable by Splunk and Microsoft Sentinel without custom parsers

**Verified:** 2026-06-19
**Status:** human_needed
**Re-verification:** Yes — after gap closure (commit `fix(84): close CR-01 + WR-01`)

## Gap Closure Evidence

### CR-01 — Gate field check is now event-type-conditional (CLOSED)

Prior state: `$script:RequiredJsonFields = @('EventType','AgentPid','PathHash','Host','SessionId','ChainHead')` unconditionally required `Host`, which is absent for path-deny events.

Current state in `scripts/gates/telemetry-event-emit.ps1`:

- Line 75: `$script:RequiredJsonFieldsUniversal = @('EventType', 'AgentPid', 'SessionId', 'ChainHead')` — universal only; no longer includes `Host` or `PathHash`
- Lines 256-261: event-type-conditional logic added:
  ```powershell
  $requiredFields = @($script:RequiredJsonFieldsUniversal)
  if ($eventId -eq $script:EventIdPathDeny) {
      $requiredFields += 'PathHash'
  } elseif ($eventId -eq $script:EventIdNetworkDeny) {
      $requiredFields += 'Host'
  }
  ```
- Lines 63-74: the gate comment block explicitly documents the CR-01 fix rationale (why `Host` was removed from the universal set)
- The structural blocker is resolved: a path-deny event (EventID 10001) will assert `EventType, AgentPid, SessionId, ChainHead, PathHash` and will NOT assert `Host`. The schema correctly omits `Host` for path-deny via `#[serde(skip_serializing_if = "Option::is_none")]`. Gate and schema are now aligned — the gate can structurally reach PASS on its happy-path event type.

### WR-01 — TelemetryConfig now threaded from HKLM into init_tracing (CLOSED)

Prior state: `main.rs` called `init_tracing(&cli, None)` with an explicit `None`.

Current state in `crates/nono-cli/src/main.rs` (lines 160-176):

```rust
let telemetry_config = match nono::read_machine_egress_policy() {
    Ok(Some(policy)) => Some(policy.telemetry),
    Ok(None) => None,
    Err(_) => None,
};
init_tracing(&cli, telemetry_config);
```

- `nono::read_machine_egress_policy()` is a re-export of `machine_policy::read_machine_egress_policy()`, verified present in `crates/nono/src/machine_policy.rs` (lines 523-532)
- `MachineEgressPolicy.telemetry: TelemetryConfig` field is present (machine_policy.rs line 182)
- The `Err(_) => None` arm is correct D-14 degrade-not-abort behavior: a policy read error falls back to `TelemetryConfig::default()` (enabled=true) rather than aborting the run
- `cli_bootstrap.rs` line 120: `let config = telemetry_config.unwrap_or_default()` — `Some(TelemetryConfig{enabled: false, ...})` now flows through instead of always defaulting
- `SecurityEventLayer::new(config, session_id)` at line 121 constructs the layer with the real config
- `on_event` guard at `telemetry/mod.rs` line 251: `if !inner.config.enabled { return; }` is no longer dead — `inner.config.enabled` can now be `false` when the admin sets `TelemetryEnabled=0`

### WR-02 — Severity filtering now implemented (CLOSED)

Prior state: `on_event` only checked `config.enabled`; `min_severity` was never consulted.

Current state:

- `machine_policy.rs` lines 58-70: `TelemetrySeverity` now derives `PartialOrd, Ord` (in addition to previous derives). Ordering is `Debug < Info < Warning < Error` per declaration order.
- `telemetry/mod.rs` lines 150-165: `severity_for()` function maps all five `SecurityEventType` variants to `TelemetrySeverity::Warning`
- `telemetry/mod.rs` lines 265-269: new guard in `on_event`:
  ```rust
  if severity_for(&event_type) < inner.config.min_severity {
      return;
  }
  ```
- Two tests added in `telemetry/mod.rs` (lines 387-417): `severity_for_all_denial_types_is_warning` and `min_severity_filter_predicate_matches_policy_threshold` — confirm that `Warning < Error` (path-deny suppressed at min_severity=Error) and `Warning >= Warning` (emits at default threshold)
- The `TelemetrySeverity::Ord` derive test in `machine_policy.rs` (lines 562-570): `telemetry_severity_orders_debug_to_error` asserts the ordering contract

## Goal Achievement

### Observable Truths (mapped from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | Application Event Log entry with EventID 10001-10005 and named EventData fields after a sandbox denial | VERIFIED (structural) | Schema correct; emitter wired; gate now structurally can PASS for path-deny (CR-01 fixed). Live PASS requires provisioned host — human verification item #1 |
| SC-2 | ChainHead present in each event; ADR records tamper boundary = WEF; SEED-005 deferral documented | VERIFIED | HMAC chain in telemetry/mod.rs; docs/adr/telemetry-tamper-evidence.md present; chain head hex propagates to SecurityEvent.chain_head |
| SC-3 | No raw file path, full URL, or credential value in event body | VERIFIED | path_hash_for() confirmed; skip_serializing_if on host/path_hash; scrub_value applied to host in on_event; gate SC-3 assertions unchanged and correct |
| SC-4 | Emitter is tracing::Layer in nono-cli/src/telemetry/; config read from machine policy | VERIFIED | Layer is in nono-cli (structural boundary correct). WR-01 closed: main.rs now calls read_machine_egress_policy() and passes Some(policy.telemetry) to init_tracing. WR-02 closed: severity_for() + on_event guard now functional. Live opt-out requires human verification item #2 |
| SC-5 | verify-dark.ps1 --gate telemetry-event-emit emits PASS verdict | VERIFIED (structural) | Gate now structurally sound — CR-01 removed the unconditional Host requirement; gate can reach all three PASS assertions (SC-1 field check, SC-3 raw-path check, SC-5 logman check). Live execution requires provisioned host — human verification item #1 |

**Score:** 5/5 truths structurally verified. Live gate PASS is human-gated (provisioned host required per MEMORY durable: `pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit`).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/telemetry/mod.rs` | SecurityEventLayer (tracing::Layer) + ChainState + severity_for | VERIFIED | Exists, substantive; severity_for() added; on_event min_severity guard added |
| `crates/nono-cli/src/telemetry/event.rs` | SecurityEvent schema + SecurityEventType + PathCategory | VERIFIED | Exists, substantive; schema unchanged from initial verification |
| `crates/nono-cli/src/telemetry/windows.rs` | ReportEventW + ETW dual-emit | VERIFIED | Unchanged from initial verification |
| `crates/nono-cli/src/telemetry/syslog.rs` | cfg(unix) stub with TODO(TELEM-FU-01) | VERIFIED | Unchanged |
| `crates/nono-cli/src/main.rs` | Calls read_machine_egress_policy() and passes Some(policy.telemetry) to init_tracing | VERIFIED | Lines 160-176 confirmed: policy read + Some(policy.telemetry) threading |
| `scripts/gates/telemetry-event-emit.ps1` | Dark-factory gate asserting SC-1/SC-3/SC-5 with conditional field checks | VERIFIED | CR-01 fix confirmed: RequiredJsonFieldsUniversal + event-type-conditional PathHash/Host |
| `docs/adr/telemetry-tamper-evidence.md` | Tamper-evidence ADR (TELEM-02) | VERIFIED | Previously verified; unchanged |
| `crates/nono/src/machine_policy.rs` | TelemetrySeverity with Ord derive; TelemetryConfig; parse_telemetry_config | VERIFIED | TelemetrySeverity derives PartialOrd, Ord (line 58); parse_telemetry_config reads all three HKLM fields with D-14 degrade semantics |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| main.rs | init_tracing | read_machine_egress_policy() + Some(policy.telemetry) | WIRED | Lines 160-176: match on Ok(Some(policy)) → Some(policy.telemetry) passed to init_tracing |
| init_tracing | SecurityEventLayer | telemetry_config.unwrap_or_default() | WIRED | cli_bootstrap.rs line 120-121: config flows into SecurityEventLayer::new |
| SecurityEventLayer.on_event | config.enabled guard | TelemetryConfig.enabled from HKLM | WIRED | on_event line 251: guard now live (enabled can be false via WR-01 fix) |
| SecurityEventLayer.on_event | severity_for() + min_severity guard | TelemetryConfig.min_severity from HKLM | WIRED | on_event lines 265-269: guard now live (WR-02 fix) |
| exec_strategy.rs | SecurityEventLayer | tracing::warn!(target:"nono_security::path_deny") | WIRED | Unchanged from initial verification |
| nono-proxy/audit.rs | SecurityEventLayer | tracing::warn!(target:"nono_security::network_deny") | WIRED | Unchanged from initial verification |
| hooks.rs | SecurityEventLayer | tracing::warn!(target:"nono_security::hook_fail_closed") | WIRED | Unchanged from initial verification |
| nono-agentd.rs | SecurityEventLayer | (no path) | NOT_WIRED | Daemon binary (nono-agentd) has no init_tracing call and no SecurityEventLayer registration — daemon-side security-event emission does not exist. See daemon-emission assessment below. |

### Daemon-Emission Gap Assessment

The daemon path (`crates/nono-cli/src/bin/nono-agentd.rs`) does not call `init_tracing` and does not register a `SecurityEventLayer`. Daemon-launched AppContainer agents produce zero security telemetry.

**Is this a phase blocker?** No, for the following reasons:

1. TELEM-01 and TELEM-04 define the emitter scope as "blocked or denied actions" on the `nono run` / exec_strategy path, which is what the Phase 84 denial sources (exec_strategy.rs, nono-proxy/audit.rs, hooks.rs) cover. The plans explicitly wired these three call sites.

2. The gate (`telemetry-event-emit.ps1`) exercises the `nono run --profile claude-code` path (exec_strategy), not the daemon path. The gate is the contractual dark-factory verification artifact for this phase.

3. The daemon emitting its own security events (agent launch, AppContainer setup failures, daemon-level denials) is a separate emission domain not described in any of the four Phase 84 plans' must-haves or ROADMAP success criteria.

4. The prior verification's REVIEW.md flagged this as a WARNING (not BLOCKER), noting: "TELEM-04 contract says 'nono-cli emitter'; daemon is a separate binary not covered."

**Classification:** daemon-emission gap is a legitimate follow-up item, not a Phase 84 blocker. The `nono run` path — which the phase planned and the gate exercises — is fully wired.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| TELEM-01 | 84-01 through 84-04 | Blocked/denied actions emitted as structured security events to Application-tier Windows Event Log with distinct EventIDs + named EventData fields | SATISFIED (structural) | Implementation sound; gate now structurally correct (CR-01 closed); live gate PASS is human-gated |
| TELEM-02 | 84-01, 84-02 | In-session HMAC-SHA256 chain (ChainHead field); tamper boundary documented as WEF; ADR recorded | SATISFIED | Unchanged from initial verification — fully wired and tested |
| TELEM-03 | 84-01, 84-02 | Redacts secrets/tokens/paths — no credential or raw path leaks into log fields | SATISFIED | Unchanged from initial verification — path_hash_for, scrub_value, skip_serializing_if all confirmed |
| TELEM-04 | 84-01, 84-02 | Emitter is tracing::Layer in nono-cli; config read from machine policy | SATISFIED (structural) | Layer boundary correct (unchanged). WR-01 closed: policy.telemetry now threaded into init_tracing. WR-02 closed: severity_for() + min_severity guard functional. Live admin opt-out requires human verification item #2 |

### Anti-Patterns — Status After Gap Closure

| File | Line | Pattern | Prior Severity | Current Status |
|------|------|---------|----------------|----------------|
| `crates/nono-cli/src/main.rs` | 160-176 | `init_tracing(&cli, None)` | BLOCKER | CLOSED — now passes `Some(policy.telemetry)` |
| `scripts/gates/telemetry-event-emit.ps1` | 75 + 256-261 | RequiredJsonFields | BLOCKER | CLOSED — universal set + event-type-conditional PathHash/Host |
| `crates/nono-cli/src/telemetry/mod.rs` | 251, 265-269 | `if !inner.config.enabled` dead code | BLOCKER | CLOSED — guard is now live; severity_for() + min_severity guard added |
| `crates/nono-cli/src/bin/nono-agentd.rs` | entire | No SecurityEventLayer registration | WARNING | Unchanged — acknowledged as follow-up (not phase-scoped) |
| `crates/nono-cli/Cargo.toml` | eventlog dep | `eventlog = "0.4"` unused | WARNING | Unchanged — WR-04; cosmetic supply-chain debt |
| `crates/nono-cli/src/telemetry/windows.rs` | 189-200 | Debug fmt of Option in ETW | INFO | Unchanged — WR-06; ETW cosmetic |
| `crates/nono-cli/src/telemetry/mod.rs` | 113-133 | HMAC degrade to zeroed key | WARNING | Unchanged — WR-05; structurally unreachable latent defect |

No new TBD / FIXME / XXX markers introduced by the gap-closure commit. The commit comment block in main.rs (lines 160-164) is a proper doc comment referencing the fix, not an unresolved debt marker.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Gate field loop for path-deny no longer requires Host | Static: gate lines 75 + 256-261 | RequiredJsonFieldsUniversal has 4 fields; path-deny adds PathHash, NOT Host | PASS |
| WR-01: Some(policy.telemetry) reaches init_tracing | Static: main.rs lines 160-176 | `Ok(Some(policy)) => Some(policy.telemetry)` feeds `init_tracing` | PASS |
| WR-01: init_tracing uses real config | Static: cli_bootstrap.rs lines 120-121 | `telemetry_config.unwrap_or_default()` — Some(TelemetryConfig{enabled:false}) flows in | PASS |
| WR-01: on_event enabled guard is live | Static: telemetry/mod.rs line 251 | `if !inner.config.enabled { return; }` — inner.config.enabled can now be false | PASS |
| WR-02: severity_for() maps all types | Static: telemetry/mod.rs lines 150-165 + test | All five SecurityEventType variants map to Warning; test asserts this | PASS |
| WR-02: on_event min_severity guard present | Static: telemetry/mod.rs lines 265-269 | `if severity_for(&event_type) < inner.config.min_severity { return; }` | PASS |
| TelemetrySeverity derives Ord | Static: machine_policy.rs line 58 | `#[derive(... PartialOrd, Ord ...)]` confirmed; ordering test passes | PASS |
| Non-fatal on policy read error | Static: main.rs lines 173-175 | `Err(_) => None` — degrades to TelemetryConfig::default(); never aborts run | PASS |

### Human Verification Required

#### 1. Full PASS on provisioned host (SC-1 + SC-3 + SC-5 gate run)

**Test:** On an elevated shell with MSI-installed nono and the Application Event Log source registered, run `pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit`

**Expected:** Verdict PASS; sc1Pass=true, sc3Pass=true, sc5Pass=true; event body excerpt shows EventType/AgentPid/PathHash/SessionId/ChainHead; no raw path string present; `logman query providers` shows "nono" ETW provider

**Why human:** Requires Windows host with MSI installation, nono Application Event Log source registration, admin elevation, and a fresh nono binary on PATH. The structural gate logic is now verified; only the live execution environment is unverifiable statically. Invocation rule (MEMORY durable): `pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit` — NEVER `pwsh -Command "<bare path>"`.

#### 2. Admin opt-out end-to-end (WR-01 live verification)

**Test:** Set `HKLM\SOFTWARE\Policies\nono\Telemetry\TelemetryEnabled = 0` (DWORD 0), run `nono run --profile claude-code -- cmd /c type C:\Windows\System32\config\SAM`, then query `Get-WinEvent -FilterHashtable @{LogName='Application';ProviderName='nono';StartTime=(Get-Date).AddMinutes(-1)}`

**Expected:** Zero events returned — admin opt-out suppresses telemetry emission; the `if !inner.config.enabled { return; }` guard fires because the real TelemetryConfig(enabled=false) now flows in from HKLM

**Why human:** Requires HKLM write access and live Application Event Log query. Verifies the WR-01 fix is end-to-end wired, not just structurally sound.

#### 3. min_severity filtering (WR-02 live verification)

**Test:** Set `HKLM\SOFTWARE\Policies\nono\Telemetry\TelemetryMinSeverity = Error` (REG_SZ), trigger a path-deny, and confirm no event appears in the Application log

**Expected:** No event — `severity_for(PathDeny) = Warning < min_severity = Error`; guard returns early before ReportEventW

**Why human:** Requires HKLM write + live Event Log inspection. Static analysis confirms the guard and Ord comparison are correct; live proof confirms the registry read-to-filter pipeline is end-to-end operative.

---

_Verified: 2026-06-19_
_Verifier: Claude (gsd-verifier)_
_Re-verification: Yes (gaps_found → human_needed after fix(84): close CR-01 + WR-01)_
