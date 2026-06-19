---
phase: 84-siem-edr-telemetry
reviewed: 2026-06-19T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - crates/nono/src/error.rs
  - crates/nono/src/machine_policy.rs
  - crates/nono/src/lib.rs
  - crates/nono-cli/src/telemetry/event.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/src/telemetry/windows.rs
  - crates/nono-cli/src/telemetry/syslog.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/hooks.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/agent_daemon/mod.rs
  - crates/nono-proxy/src/audit.rs
  - crates/nono-cli/Cargo.toml
  - bindings/c/src/lib.rs
  - scripts/gates/telemetry-event-emit.ps1
findings:
  critical: 1
  warning: 6
  info: 5
  total: 12
status: issues_found
---

# Phase 84: Code Review Report

**Reviewed:** 2026-06-19
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Phase 84 adds a `SecurityEventLayer` (`tracing::Layer`) that intercepts `nono_security::*`
denial events, hashes paths (D-08), maintains an in-session HMAC-SHA256 chain (D-05/D-06),
and dual-emits to the Windows Application Event Log + ETW (D-01). The redaction/hashing
boundary is sound — no raw path or secret reaches `ReportEventW`; paths are hashed, host is
the only cleartext field by explicit D-10 exception, and the `// SAFETY:` discipline on the
FFI calls in `windows.rs` is correct. The `unwrap_used` policy is respected in non-test code
(`advance_chain` uses a `match`-based degrade, `build_event_payload` uses `unwrap_or_else`).
The FFI `map_error` exhaustiveness debt flagged in 84-04-SUMMARY has been resolved.

However the review surfaced one **BLOCKER**: the SC-5 dark-factory gate is structurally
guaranteed to FAIL on the very event it triggers, because the gate demands the `Host` field
be present while the schema omits `Host` for path-deny events. It also surfaced a cluster of
**WARNING**-level defects where the telemetry config plumbing is decorative: the
admin-controlled `enabled`, `min_severity`, and `channel` knobs (TELEM-04 / D-12 / D-14) are
never threaded from the HKLM policy into the layer, so an admin opt-OUT has no effect and
`min_severity` filtering does not exist. These do not weaken confinement (telemetry is
compliance, not enforcement) but they break the stated SC-4 contract and silently emit when
policy says not to.

## Critical Issues

### CR-01: Dark-factory gate requires `Host` field that the schema omits for path-deny events — gate FAILs the event it triggers

**File:** `scripts/gates/telemetry-event-emit.ps1:67,245-264` (with `crates/nono-cli/src/telemetry/event.rs:232-235`)

**Issue:**
The gate's `$script:RequiredJsonFields` includes `'Host'`, and the SC-1 loop fails the gate
if any required field is `$null`:

```powershell
foreach ($field in $script:RequiredJsonFields) {        # includes 'Host'
    $fieldValue = $parsed.$field
    if ($null -eq $fieldValue) { return FAIL "required field '$field' is missing" }
}
```

But `SecurityEvent.host` is declared with:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub host: Option<String>,
```

The gate seeds its event via `Invoke-TriggerDenial`, which runs a **path-deny** (reading
`C:\Windows\System32\config\SAM` under the `claude-code` profile → EventID 10001). For a
path-deny event, `host = None`, so `Host` is **omitted entirely** from the JSON body. The gate
therefore parses the event, finds `$parsed.Host -eq $null`, and returns
`SC-1 FAILED: required field 'Host' is missing` — on a correctly-formed event.

This is not a host-provisioning artifact (the 84-04-SUMMARY attributes the observed FAIL to a
stale MSI binary); even on a perfectly provisioned host with a fresh path-deny event in the
Application log, this gate can never reach PASS for a path-deny, which is the only event type
the gate triggers. The phase ships a gate that cannot pass its own happy path. Symmetrically,
a network-deny event omits `PathHash`/`PathCategory`, so a network-triggered run would fail the
`PathHash` assertion.

**Fix:** Make `Host` (and `PathHash`) conditionally-required based on the event's `EventType`,
or emit the absent fields as explicit JSON `null` so they are always present. Minimal gate-side
fix:

```powershell
# Only EventType, AgentPid, SessionId, ChainHead are universal.
$script:RequiredJsonFields = @('EventType', 'AgentPid', 'SessionId', 'ChainHead')
# PathHash required only for path_deny; Host required only for network_deny:
if ($parsed.EventType -eq 'path_deny') {
    if ($null -eq $parsed.PathHash) { return FAIL "path_deny event missing PathHash" }
}
if ($parsed.EventType -eq 'network_deny') {
    if ($null -eq $parsed.Host) { return FAIL "network_deny event missing Host" }
}
```

Alternatively (schema side), drop `skip_serializing_if` on `host`/`path_hash` so SC-1's
"six named EventData columns" are always literally present as the ROADMAP/SC-1 wording implies,
serializing absent values as `null`.

## Warnings

### WR-01: Admin telemetry opt-OUT (`TelemetryEnabled=0`) has no effect — config never reaches the layer

**File:** `crates/nono-cli/src/main.rs:160-165`, `crates/nono-cli/src/cli_bootstrap.rs:107-121`

**Issue:**
`init_tracing(&cli, None)` is called with `None`, so `SecurityEventLayer::new` always receives
`TelemetryConfig::default()` (`enabled=true`). The `MachineEgressPolicy.telemetry` section read
by `parse_telemetry_config()` (machine_policy.rs) is never plumbed into `init_tracing`. The
daemon path (`agent_daemon/mod.rs`) has no telemetry wiring at all. Consequence: an admin who
sets `HKLM\...\nono\Telemetry\TelemetryEnabled = 0` to opt OUT (the explicit D-13 escape hatch,
"admins opt out or redirect") still gets telemetry emitted on every denial. This directly
violates the TELEM-04 / D-12 / SC-4 contract that the layer's enable/channel/severity come from
the single machine-policy read. The `on_event` guard `if !inner.config.enabled` exists but is
dead because `enabled` is hard-wired `true` at every call site.

**Fix:** Read `MachineEgressPolicy` (or at least its telemetry sub-section) before/at tracing
init and pass `Some(policy.telemetry)` into `init_tracing`. If the policy read must happen later
in `app_runtime`, either move tracing init after the read, or expose a one-time setter that
swaps the layer's `config` (behind the existing `Mutex`) once policy is known. Wire the daemon
path symmetrically.

### WR-02: `min_severity` is never consulted — severity filtering does not exist

**File:** `crates/nono-cli/src/telemetry/mod.rs:234-303`

**Issue:**
`TelemetryConfig.min_severity` (and `TelemetrySeverity`) is parsed from the registry and stored
on `SecurityEventLayerInner.config`, but `on_event` only checks `config.enabled`. No code maps a
`SecurityEventType` to a severity or compares it against `min_severity`, so setting
`TelemetryMinSeverity=Error` does not suppress lower-severity events. The field is decorative
config — an admin tuning fleet noise (the stated D-12/D-13 purpose) gets no behavior change.

**Fix:** In `on_event`, derive the event's severity (or map `SecurityEventType` → severity) and
`return` early when it is below `inner.config.min_severity`. Add a test that
`min_severity=Error` suppresses a `PathDeny` (Warning-level) event.

### WR-03: `channel` config is ignored — always writes to the hardcoded `EVENT_LOG_SOURCE = "nono"` Application source

**File:** `crates/nono-cli/src/telemetry/windows.rs:35,100-154`; `crates/nono-cli/src/telemetry/mod.rs:302`

**Issue:**
`TelemetryConfig.channel` (default `"Application"`, D-12 third knob, intended to let admins
"redirect") is never read by the emit path. `write_security_event_log` unconditionally registers
the `"nono"` source. So `TelemetryChannel=Security` (exercised in the round-trip test
`policy_serde_round_trip_with_telemetry`) has no runtime effect. Combined with WR-01/WR-02, all
three D-12 knobs are inert. At minimum the dead knob should be documented as deferred, not
presented as wired.

**Fix:** Either consume `channel` in the emit path (select the source/channel name) or
explicitly mark `channel` as a reserved/deferred field in the struct doc and in the phase
summary so it is not mistaken for a live control.

### WR-04: `eventlog` crate is a dependency but is never used

**File:** `crates/nono-cli/Cargo.toml:162-164`

**Issue:**
`eventlog = "0.4"` was added to the Windows target dependencies (and 23 packages locked), but
`windows.rs` calls `RegisterEventSourceW`/`ReportEventW` directly via `windows-sys`. A
workspace-wide grep finds no `eventlog::` usage. This is an unused dependency: extra supply-chain
surface (a newly-vetted crate) and build cost with zero call sites, which contradicts the
project's lean-deps posture (MEMORY: "lean deps winreg+tracing-etw+eventlog+hmac").

**Fix:** Remove `eventlog` from `Cargo.toml` and `Cargo.lock`, or actually route the Application-
log write through it (and then drop the hand-rolled FFI). Do not ship both.

### WR-05: HMAC chain head is reset to a zeroed key on `InvalidLength` degrade, silently breaking the tamper chain

**File:** `crates/nono-cli/src/telemetry/mod.rs:113-142`

**Issue:**
The documented-as-unreachable `InvalidLength` arm degrades to a **fixed all-zero key**
(`HmacSha256::new_from_slice(&[0u8;32])`). If that path were ever reached, the chain would
continue producing `ChainHead` values keyed by a publicly-known constant key — i.e. an attacker
could forge a continuation of the chain. The degrade also occurs without emitting the D-14
self-describing `TelemetryDegraded` security event the design promises for "no silent drop"
gaps. The branch is structurally unreachable today (key is always 32 bytes), so this is a
latent/robustness defect rather than an active vuln, but the chosen fallback (predictable key)
is the wrong fail-mode for a tamper-evidence primitive.

**Fix:** On `InvalidLength`, do not continue the keyed chain with a constant key. Prefer to skip
emitting the chained event (or emit a `TelemetryDegraded` event recording the gap) rather than
advancing the chain under a known key. At minimum, regenerate a fresh `OsRng` key rather than
zeroing, and surface the degrade as the auditable `TelemetryDegraded` event per D-14.

### WR-06: Forwarded `access` / `port` denial fields are silently dropped by the visitor (incomplete event data)

**File:** `crates/nono-cli/src/telemetry/mod.rs:309-333` (visitor) vs. `exec_strategy.rs:1794-1800`, `audit.rs:203-209`

**Issue:**
The call sites deliberately forward `access = %denial.access` (path-deny) and `port = port`
(network-deny), but `SecurityEventVisitor` only captures the `path` and `host` fields and
ignores everything else (`_ => {}`). So the access mode and destination port never reach the
emitted `SecurityEvent`. This is not a leak, but it is dead wiring: the emitted event is missing
analyst-relevant signal the denial sources took care to provide, and the `port` is arguably part
of "the denied destination an analyst needs" (D-10 rationale for keeping host cleartext). The
divergence between what call sites send and what the schema records is a maintenance trap.

**Fix:** Either extend `SecurityEvent` + the visitor to capture `port` (cleartext, alongside
host) and a scrubbed `access`, or remove the unused fields from the `tracing::warn!` call sites
and document that only `path`/`host` are consumed.

## Info

### IN-01: `path_hash_for` hashes a non-canonical display string, weakening the D-08 "same path → same hash" guarantee

**File:** `crates/nono-cli/src/telemetry/event.rs:188-199`; `exec_strategy.rs:1796`

**Issue:** The doc for `path_hash_for` says it takes "a canonicalized `Path`," but the path-deny
call site passes `denial.path.display()` (the raw `DenialRecord` path, not canonicalized), which
the layer re-parses with `Path::new`. Two textually-different spellings of the same file (e.g.
trailing slash, `.`-segments, case on Windows) hash to different values, so the
analyst-correlation property (D-08) holds only as far as the display string is stable.

**Fix:** Canonicalize (or at least `Path::components`-normalize) before hashing where feasible,
or soften the doc comment to state it hashes the path string as-presented.

### IN-02: `path_hash` and `host` are emitted to ETW with Debug formatting, embedding quotes/`Some(...)`

**File:** `crates/nono-cli/src/telemetry/windows.rs:189-200`

**Issue:** The ETW `tracing::warn!` uses `path_hash = ?event.path_hash` and `host = ?event.host`
(Debug of `Option<String>`), which renders as `Some("abcd...")` / `None` rather than the bare
value. SIEM consumers parsing the ETW field will see `Some("...")`. Cosmetic, but inconsistent
with the clean JSON Application-log body and may complicate ETW-side parsing.

**Fix:** Unwrap before formatting (e.g. `path_hash = event.path_hash.as_deref().unwrap_or("")`)
to emit the bare hash string on the ETW surface.

### IN-03: `chain_head_hex`/`path_hash_for` use `format!("{b:02x}")` in a hot per-event loop

**File:** `crates/nono-cli/src/telemetry/mod.rs:146-148`; `event.rs:195-198`

**Issue:** Allocating a `String` per byte via `format!` is idiomatic but wasteful; a single
`write!` into a pre-sized `String` or a hex helper would avoid 32–64 small allocations per
event. Out of scope for v1 (performance), noted for cleanup only.

**Fix:** Optional: use a shared `to_hex(&[u8]) -> String` helper.

### IN-04: `EventLogLevel::Information` is dead (`#[allow(dead_code)]`) — CLAUDE.md discourages `allow(dead_code)`

**File:** `crates/nono-cli/src/telemetry/windows.rs:44-49`

**Issue:** All security events emit at `Warning`; the `Information` variant is never constructed
and is retained behind `#[allow(dead_code)]`. CLAUDE.md § "Lazy use of dead code" asks to remove
unused code or exercise it. This is the same pattern flagged historically in the codebase.

**Fix:** Remove the `Information` variant until a caller needs it (e.g. a future
`TelemetryDegraded`-at-info emit), or wire `TelemetryDegraded` to emit at `Information` now.

### IN-05: `syslog.rs` stub is a no-op behind `#[cfg(unix)]` + `#[allow(dead_code)]`

**File:** `crates/nono-cli/src/telemetry/syslog.rs:12-16`

**Issue:** Acceptable per the explicit TELEM-FU-01 deferral, but it means non-Windows hosts emit
**nothing** to any SIEM sink — the layer's `on_event` still runs and computes the HMAC chain and
calls `windows::emit_security_event`, which on non-Windows only does the `tracing::warn!` (no
real sink). Confirm downstream consumers understand Linux/macOS telemetry is log-file-only this
cycle (matches scope; flagged so it is not mistaken for a wiring bug).

**Fix:** None required this cycle; ensure the deferral is reflected wherever non-Windows
telemetry behavior is documented.

---

_Reviewed: 2026-06-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
