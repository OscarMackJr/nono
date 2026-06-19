# Phase 84: SIEM/EDR Telemetry - Pattern Map

**Mapped:** 2026-06-18
**Files analyzed:** 9 new/modified files
**Analogs found:** 9 / 9

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/telemetry/mod.rs` | middleware/layer | event-driven | `crates/nono-cli/src/audit_integrity.rs` | role-match (chain discipline) |
| `crates/nono-cli/src/telemetry/event.rs` | model | event-driven | `crates/nono/src/diagnostic.rs` (DenialRecord/DenialReason) | role-match (security event schema) |
| `crates/nono-cli/src/telemetry/windows.rs` | utility | event-driven | `crates/nono-cli/src/bin/nono-wfp-service.rs` §`write_event_log` | exact (same RegisterEventSourceW/ReportEventW/DeregisterEventSource pattern) |
| `crates/nono-cli/src/telemetry/syslog.rs` | utility | event-driven | N/A — stub only this cycle | none (see No Analog Found) |
| `crates/nono-cli/src/cli_bootstrap.rs` (MODIFIED) | config | request-response | self (existing `init_tracing`) | exact (extend existing registry chain) |
| `crates/nono/src/machine_policy.rs` (MODIFIED) | model/config | request-response | self (existing `MachineEgressPolicy`) | exact (extend existing struct) |
| `crates/nono-proxy/src/audit.rs` (MODIFIED) | middleware | event-driven | self (existing `log_denied`) | exact (add dual-emit tracing call) |
| `crates/nono-cli/src/hooks.rs` (MODIFIED) | middleware | event-driven | `crates/nono-cli/src/hooks.rs` (existing fail-closed path) | exact (add tracing call at existing site) |
| `scripts/gates/telemetry-event-emit.ps1` | config/test | event-driven | `scripts/gates/egress-policy-deny.ps1` | exact (same two-function gate contract) |

---

## Pattern Assignments

### `crates/nono-cli/src/telemetry/mod.rs` (layer, event-driven)

**Analog:** `crates/nono-cli/src/audit_integrity.rs`

**Imports pattern** (audit_integrity.rs lines 1-8):
```rust
use nono::supervisor::{AuditEntry, UrlOpenRequest};
use nono::undo::{AuditIntegritySummary, ContentHash, NetworkAuditEvent};
use nono::{NonoError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
```

For the telemetry layer, replace sha2 direct use with hmac + sha2, add tracing_subscriber Layer import:
```rust
// telemetry/mod.rs imports (new)
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use zeroize::Zeroizing;
use nono::{NonoError, Result};
```

**Chain discipline pattern** (audit_integrity.rs lines 21-28 — domain separator convention to copy):
```rust
// Borrow the DOMAIN SEPARATOR discipline but with a DIFFERENT prefix (D-06 independent chain).
// audit_integrity.rs uses:
const EVENT_DOMAIN: &[u8] = b"nono.audit.event.alpha\n";
const CHAIN_DOMAIN: &[u8] = b"nono.audit.chain.alpha\n";

// telemetry/mod.rs MUST use DIFFERENT separators to keep chains independent:
const TELEMETRY_EVENT_DOMAIN: &[u8] = b"nono.telemetry.event.alpha\n";
const TELEMETRY_CHAIN_DOMAIN: &[u8] = b"nono.telemetry.chain.alpha\n";
// Key held in Zeroizing<[u8; 32]> — generated from OsRng at session start (D-05).
```

**Core chain pattern** (audit_integrity.rs lines 263-286 — `append_event` body, adapt for HMAC):
```rust
// audit_integrity.rs unkeyed SHA-256 chain (reference discipline only — D-06):
fn append_event(&mut self, event: AuditEventPayload) -> Result<()> {
    let event_bytes = serde_json::to_vec(&event)
        .map_err(|e| NonoError::Snapshot(format!("Failed to serialize audit event: {e}")))?;
    let leaf_hash = hash_event(&event_bytes);
    let chain_hash = hash_chain(self.previous_chain.as_ref(), &leaf_hash);
    // ...
    self.next_sequence = self.next_sequence.saturating_add(1);
    self.previous_chain = Some(chain_hash);
    Ok(())
}

// Keyed HMAC version for telemetry (D-05/D-06 — NEW independent chain):
// ChainHead = HMAC-SHA256(session_key, prev_head_bytes || canonical_event_bytes)
// Genesis: prev_head = [0u8; 32] (or session_id bytes, per planner discretion)
```

**tracing::Layer integration site** (cli_bootstrap.rs lines 87-119 — where to insert):
```rust
// cli_bootstrap.rs init_tracing (lines 87-119) — current pattern:
pub(crate) fn init_tracing(cli: &Cli) {
    match cli.log_file.as_deref() {
        Some(path) => { /* ... */ }
        None => {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_filter(cli))
                .with_target(false)
                .with_writer(std::io::stderr)
                .init();  // <-- REPLACE .init() with .with(security_layer).init()
        }
    }
}

// Modified pattern (add SecurityEventLayer as an additional Layer):
// tracing_subscriber::registry()
//     .with(fmt_layer)
//     .with(security_event_layer)
//     .init();
// SecurityEventLayer is constructed from MachineEgressPolicy telemetry config (D-12).
```

**Sequence number pattern** (audit_integrity.rs lines 282-283):
```rust
// saturating_add is the project-wide safe arithmetic convention (CLAUDE.md):
self.next_sequence = self.next_sequence.saturating_add(1);
```

---

### `crates/nono-cli/src/telemetry/event.rs` (model, event-driven)

**Analog:** `crates/nono/src/diagnostic.rs` (DenialRecord / DenialReason shape)

**DenialRecord and DenialReason** (diagnostic.rs lines 47-72):
```rust
// crates/nono/src/diagnostic.rs — DenialReason and DenialRecord (source types)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenialReason {
    PolicyBlocked,
    InsufficientAccess,
    UserDenied,
    RateLimited,
    BackendError,
    UnixSocketDenied,
}

#[derive(Debug, Clone)]
pub struct DenialRecord {
    pub path: PathBuf,
    pub access: AccessMode,
    pub reason: DenialReason,
}
```

`SecurityEvent` is SEPARATE from these types (Pitfall 11 — never alias DiagnosticOutput). Pattern to follow for the schema struct:
```rust
// telemetry/event.rs — SecurityEvent schema (NEW, distinct from DenialRecord)
#[derive(Debug, Clone, Serialize)]
pub struct SecurityEvent {
    pub event_type: SecurityEventType,   // maps to EventID 10001-10005
    pub agent_pid: u32,
    pub path_hash: Option<String>,        // SHA-256(session_salt || canonical_path)[0..N], NOT raw path
    pub path_category: Option<PathCategory>, // D-09 sensitivity tier
    pub host: Option<String>,             // cleartext (SC-1 — analyst needs this)
    pub session_id: String,
    pub chain_head: String,               // hex of HMAC chain head (D-05)
    pub timestamp_unix_ms: u64,
}

// D-09 sensitivity tiers:
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathCategory {
    WorkspaceFile,
    SystemPath,
    CredentialPath,    // .ssh/.aws/keystore paths
    UserHome,
    Temp,
    Other,
}

// D-11 EventID map (locked in ROADMAP):
// 10001 = path_deny
// 10002 = network_deny
// 10003 = label_violation
// 10004 = hook_fail_closed
// 10005 = telemetry_degraded (self-describing D-14 event)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    PathDeny,
    NetworkDeny,
    LabelViolation,
    HookFailClosed,
    TelemetryDegraded,
}
```

**Scrub integration pattern** (scrub.rs lines 176-193):
```rust
// crates/nono/src/scrub.rs — scrub_value_with_policy (use this for any free-text field)
#[must_use]
pub fn scrub_value(s: &str) -> Cow<'_, str> {
    scrub_value_with_policy(s, &ScrubPolicy::secure_default())
}

#[must_use]
pub fn scrub_value_with_policy<'a>(s: &'a str, policy: &ScrubPolicy) -> Cow<'a, str> {
    let header_scrubbed = scrub_header_line(s, policy);
    let url_scrubbed = scrub_url_userinfo(header_scrubbed.as_ref() as &str);
    let query_scrubbed = scrub_query_params(url_scrubbed.as_ref() as &str, policy);
    // ...
}
// Apply scrub_value() to every free-text SecurityEvent field before constructing
// the event (D-10). Host stays cleartext (SC-1), but reason/label strings go
// through scrub_value_with_policy.
```

**NetworkAuditEvent shape** (nono-proxy/audit.rs lines 7-47 — shows what fields the proxy already has):
```rust
// proxy/audit.rs — fields available at log_denied() call site:
pub struct NetworkAuditEvent {
    pub timestamp_unix_ms: u64,
    pub mode: NetworkAuditMode,
    pub decision: NetworkAuditDecision,
    pub target: String,   // host — goes into SecurityEvent.host cleartext (SC-1)
    pub port: Option<u16>,
    pub reason: Option<String>,  // goes through scrub_value before SecurityEvent
    // ... (method/path/status/auth fields)
}
```

---

### `crates/nono-cli/src/telemetry/windows.rs` (utility, event-driven, cfg(windows))

**Analog:** `crates/nono-cli/src/bin/nono-wfp-service.rs` lines 59-215

**Event source constant pattern** (nono-wfp-service.rs lines 59-73):
```rust
// nono-wfp-service.rs (analog):
const EVENT_LOG_SOURCE: &str = SERVICE_NAME;  // = "nono-wfp-service"

const EVENT_ID_SWEEP_COMPLETE: u32 = 1001;
const EVENT_ID_SWEEP_REMOVED: u32 = 1002;
const EVENT_ID_SWEEP_SKIPPED: u32 = 1003;
const EVENT_ID_SWEEP_FAILED: u32 = 1004;

// telemetry/windows.rs (new) — mirror this pattern with the Phase 84 EventIDs:
const EVENT_LOG_SOURCE: &str = "nono";  // Phase-82-registered Application source
const EVENT_ID_PATH_DENY: u32       = 10001;
const EVENT_ID_NETWORK_DENY: u32    = 10002;
const EVENT_ID_LABEL_VIOLATION: u32 = 10003;
const EVENT_ID_HOOK_FAIL_CLOSED: u32 = 10004;
const EVENT_ID_TELEMETRY_DEGRADED: u32 = 10005;
```

**RegisterEventSourceW/ReportEventW/DeregisterEventSource pattern** (nono-wfp-service.rs lines 161-205 — exact clone shape):
```rust
// nono-wfp-service.rs lines 161-205 — proven pattern; copy verbatim for telemetry emitter:
#[cfg(target_os = "windows")]
fn write_event_log(level: EventLogLevel, event_id: u32, body: &str) {
    use windows_sys::Win32::System::EventLog::{
        DeregisterEventSource, RegisterEventSourceW, ReportEventW, EVENTLOG_INFORMATION_TYPE,
        EVENTLOG_WARNING_TYPE,
    };

    let source_wide: Vec<u16> = EVENT_LOG_SOURCE
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();
    // SAFETY: source_wide is a valid null-terminated UTF-16 string.
    // The handle is closed via DeregisterEventSource before the function returns.
    let handle = unsafe { RegisterEventSourceW(std::ptr::null(), source_wide.as_ptr()) };
    if handle.is_null() {
        // Source not registered (development or test environment). Fall back to stderr.
        eprintln!("{}", build_event_log_message(level, event_id, body));
        return;  // D-03: loud non-fatal — emit to stderr, do NOT silently drop
    }
    // ...
    let body_wide: Vec<u16> = body.encode_utf16().chain(std::iter::once(0u16)).collect();
    let strings: [*const u16; 1] = [body_wide.as_ptr()];

    // SAFETY: handle is valid; strings contains exactly one pointer to a
    // null-terminated UTF-16 string; user-data pointer is null.
    unsafe {
        let _ = ReportEventW(handle, event_type, 0, event_id,
            std::ptr::null_mut(), 1, 0, strings.as_ptr(), std::ptr::null_mut());
        let _ = DeregisterEventSource(handle);
    }
}
```

**NULL handle = non-fatal stderr fallback** (nono-wfp-service.rs lines 175-178):
```rust
// Critical: handle.is_null() is the D-03 trigger — NOT a panic, NOT a silent drop:
if handle.is_null() {
    eprintln!("{}", build_event_log_message(level, event_id, body));
    return;
}
```

**D-02 JSON payload pattern** — the single insertion string is a compact JSON object:
```rust
// D-02: body = serde_json::to_string(&SecurityEventJsonPayload { ... }).unwrap_or_else(...)
// Fields: EventType, AgentPid, PathHash, Host, SessionId, ChainHead (SC-1 minimum)
// scrub all free-text fields through scrub_value before serializing.
```

**EventLogLevel analog** (nono-wfp-service.rs lines 86-91):
```rust
// nono-wfp-service.rs — copy this shape:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventLogLevel {
    Information,
    Warning,
}
// Map SecurityEventType to EventLogLevel:
// PathDeny/NetworkDeny/LabelViolation/HookFailClosed -> Warning
// TelemetryDegraded -> Warning
```

**cfg-gate pattern** (nono-wfp-service.rs lines 161, 210-215):
```rust
// cfg-gate the real emit; always emit to stderr fallback (cross-platform logging):
#[allow(unused_variables)]
fn emit_security_event(event: &SecurityEvent) {
    // Always log to tracing::warn! for the ETW layer (tracing-etw) to pick up.
    // The Event Log write is Windows-only:
    #[cfg(target_os = "windows")]
    write_event_log(EventLogLevel::Warning, event_id_for(&event.event_type), &payload_json);
}
```

---

### `crates/nono-cli/src/telemetry/syslog.rs` (utility, event-driven, cfg(unix))

**No codebase analog.** This is a stub this cycle (CONTEXT.md `<deferred>` section). Research notes that RFC 5424 syslog emission is TELEM-FU-01, out of scope. The file should exist as a `cfg(unix)` stub with a `// TODO(TELEM-FU-01): RFC 5424 syslog emission` comment.

---

### `crates/nono-cli/src/cli_bootstrap.rs` (MODIFIED — init_tracing extension)

**Analog:** self — `crates/nono-cli/src/cli_bootstrap.rs` lines 87-119

**Current pattern** (lines 87-119 — the exact code to extend):
```rust
pub(crate) fn init_tracing(cli: &Cli) {
    match cli.log_file.as_deref() {
        Some(path) => match SharedFileMakeWriter::new(path) {
            Ok(writer) => {
                tracing_subscriber::fmt()
                    .with_env_filter(tracing_filter(cli))
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(writer)
                    .init();  // <-- extend here
            }
            Err(err) => {
                // ... fallback
                tracing_subscriber::fmt()
                    // ...
                    .init();  // <-- extend here too
            }
        },
        None => {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_filter(cli))
                .with_target(false)
                .with_writer(std::io::stderr)
                .init();  // <-- extend here
        }
    }
}
```

**Extension target:** Replace every `.init()` call with `.with(security_layer).init()`. The `SecurityEventLayer` is constructed once before the match, receiving the telemetry config from `MachineEgressPolicy` (D-12). The layer must be `Clone` or wrapped in `Arc` if shared across the three arms. Pattern from existing SharedFileMakeWriter (lines 195-238) shows how the project wraps shareable state in `Arc<Mutex<T>>`.

**MachineEgressPolicy read pattern** — read happens before `init_tracing` in main/app_runtime; pass config in:
```rust
// init_tracing signature change:
pub(crate) fn init_tracing(cli: &Cli, telemetry_config: Option<TelemetryConfig>) { ... }
// TelemetryConfig extracted from MachineEgressPolicy.telemetry (D-12).
// When MachineEgressPolicy is absent (Ok(None)), TelemetryConfig uses default ON (D-13).
```

---

### `crates/nono/src/machine_policy.rs` (MODIFIED — telemetry sub-section)

**Analog:** self — current `MachineEgressPolicy` at lines 63-150

**Current struct** (machine_policy.rs lines 62-83):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MachineEgressPolicy {
    #[serde(default)]
    pub allowed_suffixes: Vec<String>,

    #[serde(default)]
    pub allowed_hosts: Vec<String>,

    #[serde(default)]
    pub preset_tokens: Vec<String>,
}
```

**D-12 extension pattern** — add telemetry sub-section as a nested struct:
```rust
// Add to MachineEgressPolicy:
#[serde(default)]
pub telemetry: TelemetryConfig,

// New nested type (library-layer, policy-free — only carries raw config values):
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// D-13: default true when absent (security telemetry on by default).
    #[serde(default = "default_telemetry_enabled")]
    pub enabled: bool,
    /// REG_SZ — default "Application" (D-01.2).
    #[serde(default = "default_telemetry_channel")]
    pub channel: String,
    /// Minimum severity to emit (informational/warning/error). Default = warning.
    #[serde(default = "default_telemetry_min_severity")]
    pub min_severity: TelemetrySeverity,
}

fn default_telemetry_enabled() -> bool { true }
fn default_telemetry_channel() -> String { "Application".to_string() }
fn default_telemetry_min_severity() -> TelemetrySeverity { TelemetrySeverity::Warning }

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_telemetry_enabled(),
            channel: default_telemetry_channel(),
            min_severity: default_telemetry_min_severity(),
        }
    }
}
```

**Windows reader extension** (machine_policy.rs lines 217-226 — parse_policy function):
```rust
// Current parse_policy:
pub(super) fn parse_policy(key: &RegKey) -> std::result::Result<MachineEgressPolicy, String> {
    let allowed_suffixes = read_list_subkey(key, "AllowedSuffixes")?;
    let allowed_hosts = read_list_subkey(key, "AllowedHosts")?;
    let preset_tokens = read_preset_subkey(key, "PresetTokens")?;
    Ok(MachineEgressPolicy { allowed_suffixes, allowed_hosts, preset_tokens })
}

// D-12 extension: add telemetry reads.
// Per D-14: malformed telemetry config DEGRADES (not abort) — contrast with D-07 egress abort.
// Map None (absent) → TelemetryConfig::default() (D-13 default ON).
// Map malformed REG values → TelemetryConfig::default() + log reason to stderr.
```

**is_unconfigured helper** (machine_policy.rs lines 129-133 — extend to not count telemetry):
```rust
// is_unconfigured must remain based on egress content only:
pub fn is_unconfigured(&self) -> bool {
    self.allowed_suffixes.is_empty()
        && self.allowed_hosts.is_empty()
        && self.preset_tokens.is_empty()
    // telemetry config does NOT gate enforcement (CR-02 still holds)
}
```

---

### `crates/nono-proxy/src/audit.rs` (MODIFIED — dual-emit at log_denied)

**Analog:** self — `crates/nono-proxy/src/audit.rs` lines 175-214

**Current log_denied** (lines 176-214):
```rust
pub fn log_denied(
    audit_log: Option<&SharedAuditLog>,
    mode: ProxyMode,
    ctx: &EventContext<'_>,
    host: &str,
    port: u16,
    reason: &str,
) {
    info!(
        target: "nono_proxy::audit",
        mode = %mode,
        host = host,
        port = port,
        decision = "deny",
        reason = reason,
        "proxy request denied"
    );
    // ...
}
```

**D-10 dual-emit extension** — add a second `tracing::warn!` with `nono_security::network_deny` target:
```rust
// Add AFTER the existing info! call (explicit dual-emit per ARCHITECTURE.md preferred option 1):
tracing::warn!(
    target: "nono_security::network_deny",
    host = host,                // cleartext (SC-1 — D-10 exception)
    port = port,
    session_id = %ctx.route_id.unwrap_or(""),
    "network deny"
    // NO url/path/query params — D-10 forbids full URLs
);
// The SecurityEventLayer picks up the nono_security::* target.
// reason goes through scrub_value() before being attached to the SecurityEvent
// inside the Layer impl (not at this call site, since it lives in nono-proxy).
```

---

### `crates/nono-cli/src/hooks.rs` (MODIFIED — emit at fail-closed path)

**Analog:** `crates/nono-cli/src/hooks.rs` (fail-closed code path — location to identify during planning)

**Pattern to inject** (modeled on nono_proxy::audit dual-emit):
```rust
// At the hooks.rs fail-closed path, add:
tracing::warn!(
    target: "nono_security::hook_fail_closed",
    hook_name = %hook_name,
    exit_code = exit_code,
    "hook fail-closed"
);
// No raw paths or tokens in the event fields (D-10).
// scrub_value applied inside SecurityEventLayer before emit.
```

---

### `scripts/gates/telemetry-event-emit.ps1` (new gate, config/test)

**Analog:** `scripts/gates/egress-policy-deny.ps1` lines 1-72

**Two-function gate contract** (egress-policy-deny.ps1 lines 1-42 — mandatory contract):
```powershell
# scripts/gates/egress-policy-deny.ps1 — gate contract comments (lines 1-42):
# this gate exports exactly two functions, dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object — it MUST NOT call exit and MUST NOT call Persist-Verdict.
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
```

**Gate configuration block pattern** (egress-policy-deny.ps1 lines 48-55):
```powershell
$script:PolicyKeyPath = 'SOFTWARE\Policies\nono'
# telemetry-event-emit.ps1 equivalent:
$script:EventLogSource = 'nono'
$script:EventIdPathDeny = 10001
$script:EventIdNetworkDeny = 10002
$script:TelemetryGateName = 'telemetry-event-emit'
```

**Assert-True helper** (egress-policy-deny.ps1 lines 62-72 — copy verbatim):
```powershell
function Assert-True {
    param(
        [Parameter(Mandatory = $true)] [bool]$Condition,
        [Parameter(Mandatory = $true)] [string]$Message
    )
    if (-not $Condition) { throw $Message }
}
```

**Gate proof obligations** (D-04 — both surfaces must be asserted):
```powershell
# Invoke-Gate must assert:
# (a) Application log entry under 'nono' source with EventID 10001-10005,
#     named JSON fields, and NO raw path strings in the event message
# (b) ETW provider emission detectable via `logman query providers | Select-String 'nono'`
#
# Use Get-WinEvent to query the Application log:
$events = Get-WinEvent -FilterHashtable @{
    LogName = 'Application'; ProviderName = 'nono'; Id = 10001
} -ErrorAction SilentlyContinue
# Assert event body does not contain raw path strings (SC-3):
$body = ($events | Select-Object -First 1 -ExpandProperty Message)
Assert-True (-not ($body -match 'C:\\Users\\[^"]+')) "Raw path found in event body"
```

**INVOCATION RULE** (from egress-policy-deny.ps1 line 43 — durable from MEMORY):
```
pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit
NEVER: pwsh -Command "<bare path>" (swallows exit N -> 1)
```

---

## Shared Patterns

### Windows cfg-gating (CLAUDE.md cross-target MUST/NEVER)
**Source:** `crates/nono-cli/src/bin/nono-wfp-service.rs` lines 161, 231
**Apply to:** `telemetry/windows.rs`, any Windows-only import in `telemetry/mod.rs`
```rust
// All Win32 Event Log and ETW imports must be behind cfg(target_os = "windows"):
#[cfg(target_os = "windows")]
fn write_security_event_log(event_id: u32, body: &str) { ... }

// Non-Windows stub at module level:
#[cfg(not(target_os = "windows"))]
fn write_security_event_log(_event_id: u32, _body: &str) {}
```
Cross-target clippy MUST check: `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` must pass after any change to cfg-gated telemetry code.

### Error Handling — fail-loud-not-abort for telemetry
**Source:** `crates/nono-cli/src/bin/nono-wfp-service.rs` lines 175-178 (stderr fallback)
**Apply to:** `telemetry/windows.rs`, `telemetry/mod.rs` Layer impl
```rust
// D-03/D-14 pattern: telemetry sink failure = loud stderr, continue run.
// NEVER: panic, NEVER: return Err propagated to confinement path.
// Contrast with D-07 in machine_policy.rs (egress errors abort — different criticality).
if handle.is_null() {
    eprintln!("{}", build_event_log_message(level, event_id, body));
    return;   // continue, do not abort the agent
}
```

### Scrub before emit (Pitfall 11)
**Source:** `crates/nono/src/scrub.rs` lines 176-193 (`scrub_value`, `scrub_value_with_policy`)
**Apply to:** all `SecurityEvent` field construction in `telemetry/event.rs` and `telemetry/windows.rs`
```rust
// NEVER emit raw paths — always hash (D-08):
let path_hash = sha256_truncated(session_salt, canonical_path);
// NEVER emit full URLs — hostname only (D-10):
let host = event.host.as_deref().unwrap_or("");
// All free-text reason/label fields:
let scrubbed_reason = nono::scrub_value(reason).into_owned();
```

### Zeroize for session key (D-05)
**Source:** CLAUDE.md §Memory: "Use the `zeroize` crate for sensitive data (keys/passwords) in memory."
**Apply to:** `telemetry/mod.rs` ChainState session key field
```rust
use zeroize::Zeroizing;

struct ChainState {
    key: Zeroizing<[u8; 32]>,  // ephemeral per-session HMAC key
    head: [u8; 32],             // current chain head
}
// key is generated from OsRng at SecurityEventLayer construction and zeroized on drop.
```

### Domain-separator convention (from audit_integrity.rs)
**Source:** `crates/nono-cli/src/audit_integrity.rs` lines 21-23
**Apply to:** `telemetry/mod.rs` HMAC chain (D-06 — different prefix, same discipline)
```rust
// audit_integrity.rs uses "nono.audit.*.alpha\n" — REFERENCE ONLY.
// telemetry/mod.rs uses DIFFERENT separators (D-06 independence):
const TELEMETRY_EVENT_DOMAIN: &[u8] = b"nono.telemetry.event.alpha\n";
const TELEMETRY_CHAIN_DOMAIN: &[u8] = b"nono.telemetry.chain.alpha\n";
```

### NonoError variants
**Source:** `crates/nono/src/error.rs` (existing enum) — add two variants:
```rust
// Per CONTEXT.md D-03/D-14 and Claude's Discretion:
// TelemetryUnavailable — RegisterEventSourceW returned NULL (non-fatal, stderr)
// TelemetryConfigInvalid — malformed telemetry REG value (non-fatal, degrade to default)
// These are CLI-layer conditions; they should be added to NonoError in the library
// IF the library returns them, OR defined locally in nono-cli if telemetry is entirely CLI.
// Per CLAUDE.md boundary: telemetry is CLI-only, so a local nono-cli error type is preferred.
```

### saturating arithmetic (CLAUDE.md)
**Source:** `crates/nono-cli/src/audit_integrity.rs` line 282
**Apply to:** sequence counters in `telemetry/mod.rs`
```rust
self.sequence = self.sequence.saturating_add(1);
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/nono-cli/src/telemetry/syslog.rs` | utility | event-driven | No event-driven syslog emitter exists in the codebase; RFC 5424 is deferred (TELEM-FU-01); file is a cfg(unix) stub this cycle |

---

## Metadata

**Analog search scope:** `crates/nono/src/`, `crates/nono-cli/src/`, `crates/nono-proxy/src/`, `scripts/gates/`
**Files read for pattern extraction:** 9 source files + 4 research documents
**Pattern extraction date:** 2026-06-18

---

## Key Invariants Extracted from Analogs

1. **write_event_log NULL-handle = stderr fallback, not panic.** `nono-wfp-service.rs:175-178` is the authoritative D-03 pattern. Copy exactly — including the `eprintln!` format call before `return`.

2. **Every HMAC/SHA-256 domain must be distinct.** `audit_integrity.rs:21-23` shows `b"nono.audit.event.alpha\n"`. The telemetry chain must use `b"nono.telemetry.event.alpha\n"` and `b"nono.telemetry.chain.alpha\n"` (D-06).

3. **MachineEgressPolicy.is_unconfigured() must not count telemetry config.** The CR-02 content-gate (machine_policy.rs:129-133) must remain egress-only; a telemetry-only HKLM write must not flip strict deny-all egress.

4. **telemetry degrade ≠ egress abort.** `machine_policy.rs:262-266` shows the egress abort path (`return Err(PolicyLoadFailed)`). Telemetry parse errors must take the opposite path: log to stderr, fall back to `TelemetryConfig::default()`, emit a 10005 self-describing event (D-14).

5. **log_denied dual-emit preserves the existing info! call.** The new `tracing::warn!(target: "nono_security::network_deny", ...)` in `audit.rs` is ADDITIVE — never replaces the existing `info!(target: "nono_proxy::audit", ...)` call.

6. **scrub_value before ANY SecurityEvent field.** `scrub.rs:176-193` is the enforcement engine. Apply to every field that could contain a token, URL query param, or header. Paths are hashed (D-08), never passed to scrub_value (hashing is the mitigation, not redaction).

7. **Gate contract is non-negotiable.** `egress-policy-deny.ps1:1-42` defines the two-function shape. `telemetry-event-emit.ps1` must export exactly `Test-Precondition` and `Invoke-Gate`; no direct `exit` calls; no direct `Persist-Verdict` calls.
