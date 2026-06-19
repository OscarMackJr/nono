//! Windows security-event emitter — Phase 84 Plan 02.
//!
//! Implements the dual-emit backend for [`super::SecurityEventLayer`]:
//!
//! 1. **Application Event Log** via `RegisterEventSourceW`/`ReportEventW` (D-01.2).
//!    Writes a compact JSON insertion string (D-02) to the Phase-82-registered
//!    `nono` Application source at EventIDs 10001-10005.
//!
//! 2. **ETW TraceLogging** (D-01.1).  The `tracing::warn!(target: "nono_security::*", …)`
//!    call inside [`emit_security_event`] is automatically picked up by any
//!    `tracing-etw::LayerBuilder` registered in the `init_tracing()` subscriber
//!    stack (Plan 02 wires that layer).
//!
//! # Non-fatal contract (D-03)
//!
//! If `RegisterEventSourceW` returns NULL (source not registered — expected on
//! dev hosts before MSI install), the function emits the payload to `stderr`
//! and returns immediately.  It never panics and never blocks the confined run.
//!
//! # Safety
//!
//! All `unsafe` blocks in this file are Windows FFI calls.  Each is documented
//! with a `// SAFETY:` comment per CLAUDE.md.

use super::event::{event_id_for as schema_event_id_for, SecurityEvent, SecurityEventType};

// ── Event Log source name (Phase-82-registered Application source) ────────────

/// Classic Windows Application Event Log source name.
///
/// Registered by the Phase-82 machine MSI under
/// `SYSTEM\CurrentControlSet\Services\EventLog\Application\nono`.
/// Must match the source string used by the MSI `cmpNonoCliEventLogSource`
/// component in `scripts/build-windows-msi.ps1`.
pub(crate) const EVENT_LOG_SOURCE: &str = "nono";

// ── EventLogLevel ─────────────────────────────────────────────────────────────

/// Verbosity level for a Windows Event Log entry.
///
/// All security events currently use `Warning` level.  `Information` is
/// retained for future use (e.g., TelemetryDegraded self-reporting at a
/// lower severity) and for parity with the `nono-wfp-service` pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventLogLevel {
    #[allow(dead_code)]
    Information,
    Warning,
}

// ── event_id_for ─────────────────────────────────────────────────────────────

/// Return the EventID for a [`SecurityEventType`] (ROADMAP-locked, 10001-10005).
///
/// Delegates to [`super::event::event_id_for`] (the authoritative implementation)
/// and re-exported here for use by the tests below.
fn event_id_for(t: &SecurityEventType) -> u32 {
    schema_event_id_for(t)
}

// ── build_event_payload ───────────────────────────────────────────────────────

/// Serialize a [`SecurityEvent`] to a compact JSON insertion string (D-02).
///
/// This is the single string passed to `ReportEventW`.  It contains only the
/// already-scrubbed/hashed fields from [`SecurityEvent`]; raw paths and URLs
/// are structurally absent from the type (enforced by the schema in event.rs).
fn build_event_payload(event: &SecurityEvent) -> String {
    serde_json::to_string(event).unwrap_or_else(|e| {
        // Serialization failure must not silently drop the event body.
        // Emit a minimal JSON object recording the error (D-14 degrade-not-abort).
        format!("{{\"error\":\"serialize:{e}\"}}")
    })
}

// ── build_event_log_message ───────────────────────────────────────────────────

/// Build a fallback stderr message for the D-03 NULL-handle path.
fn build_event_log_message(level: EventLogLevel, event_id: u32, body: &str) -> String {
    let level_str = match level {
        EventLogLevel::Information => "INFO",
        EventLogLevel::Warning => "WARN",
    };
    format!(
        "[{level_str}] source={} event_id={} {}",
        EVENT_LOG_SOURCE, event_id, body
    )
}

// ── write_security_event_log ──────────────────────────────────────────────────

/// Write a security event to the Windows Application Event Log.
///
/// On non-Windows platforms this function is absent; the stub below is used.
///
/// # Non-fatal contract (D-03 / Pitfall 10)
///
/// If `RegisterEventSourceW` returns NULL, emit the formatted message to
/// `stderr` and return.  Never panics; never blocks the confined run.
#[cfg(target_os = "windows")]
fn write_security_event_log(level: EventLogLevel, event_id: u32, body: &str) {
    use windows_sys::Win32::System::EventLog::{
        DeregisterEventSource, RegisterEventSourceW, ReportEventW, EVENTLOG_INFORMATION_TYPE,
        EVENTLOG_WARNING_TYPE,
    };

    let source_wide: Vec<u16> = EVENT_LOG_SOURCE
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();

    // SAFETY: source_wide is a valid null-terminated UTF-16 string.
    // The handle is closed via DeregisterEventSource before this function returns,
    // so there is no handle leak even on the NULL fallback path (we return early).
    let handle = unsafe { RegisterEventSourceW(std::ptr::null(), source_wide.as_ptr()) };

    if handle.is_null() {
        // Source not registered (development environment or broken MSI install).
        // D-03: loud non-fatal — emit to stderr, do NOT silently drop the event.
        eprintln!(
            "nono: telemetry: RegisterEventSourceW returned NULL (source not registered) — {}",
            build_event_log_message(level, event_id, body)
        );
        return;
    }

    let event_type = match level {
        EventLogLevel::Information => EVENTLOG_INFORMATION_TYPE,
        EventLogLevel::Warning => EVENTLOG_WARNING_TYPE,
    };

    let body_wide: Vec<u16> = body.encode_utf16().chain(std::iter::once(0u16)).collect();
    let strings: [*const u16; 1] = [body_wide.as_ptr()];

    // SAFETY: handle is valid (non-null, returned by RegisterEventSourceW);
    // strings contains exactly one pointer to a null-terminated UTF-16 string;
    // nStrings is 1, matching the array length;
    // lpUserSid and lpRawData are null (no binary data appended).
    // DeregisterEventSource closes the handle unconditionally after the write.
    unsafe {
        let _ = ReportEventW(
            handle,
            event_type,
            0,
            event_id,
            std::ptr::null_mut(),
            1,
            0,
            strings.as_ptr(),
            std::ptr::null_mut(),
        );
        let _ = DeregisterEventSource(handle);
    }
}

// ── emit_security_event ───────────────────────────────────────────────────────

/// Emit a security event to the Windows Application Event Log AND the ETW
/// provider (dual-emit, D-01).
///
/// The ETW emission is implicit: the `tracing::warn!(target: "nono_security::*", …)`
/// call below is intercepted by the `tracing-etw::LayerBuilder`-based layer that
/// `init_tracing()` registers in the tracing subscriber stack.  No additional
/// ETW API calls are needed here.
///
/// On non-Windows platforms this function compiles as a no-op via the stub below.
///
/// # Non-fatal contract (D-03 / D-14)
///
/// If the Application-log write fails (source not registered), the error is
/// emitted to stderr and the function returns without affecting the confined run.
pub fn emit_security_event(event: &SecurityEvent) {
    let payload = build_event_payload(event);
    let event_id = event_id_for(&event.event_type);

    // Application Event Log write (D-01.2, cfg-gated).
    #[cfg(target_os = "windows")]
    write_security_event_log(EventLogLevel::Warning, event_id, &payload);

    // ETW TraceLogging emit (D-01.1).
    // The tracing-etw LayerBuilder registered in init_tracing() intercepts this
    // warn! call on the "nono_security::*" target prefix.  Field names are
    // SC-1-compliant named EventData columns that logman and SIEM agents parse
    // without a custom manifest.
    //
    // NOTE: on non-Windows this warn! is still emitted so the fmt layer can log
    // it to the log file / stderr.  The ETW layer simply does not exist on those
    // platforms.
    tracing::warn!(
        target: "nono_security",
        event_type = ?event.event_type,
        event_id = event_id,
        agent_pid = event.agent_pid,
        path_hash = ?event.path_hash,
        host = ?event.host,
        session_id = %event.session_id,
        chain_head = %event.chain_head,
        timestamp_unix_ms = event.timestamp_unix_ms,
        "security event"
    );

    // Suppress unused-variable warnings on non-Windows where the cfg-gated
    // write_security_event_log call above is absent.
    let _ = payload;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::telemetry::event::{
        SecurityEvent, SecurityEventType, EVENT_ID_HOOK_FAIL_CLOSED, EVENT_ID_LABEL_VIOLATION,
        EVENT_ID_NETWORK_DENY, EVENT_ID_PATH_DENY, EVENT_ID_TELEMETRY_DEGRADED,
    };

    /// Verify EVENT_LOG_SOURCE is "nono" (the Phase-82-registered Application source).
    #[test]
    fn event_log_source_is_nono() {
        assert_eq!(
            EVENT_LOG_SOURCE,
            "nono",
            "EVENT_LOG_SOURCE must be the Phase-82-registered source"
        );
    }

    /// Verify all five EventID constants match the ROADMAP-locked values.
    #[test]
    fn event_id_constants_are_correct() {
        assert_eq!(EVENT_ID_PATH_DENY, 10001);
        assert_eq!(EVENT_ID_NETWORK_DENY, 10002);
        assert_eq!(EVENT_ID_LABEL_VIOLATION, 10003);
        assert_eq!(EVENT_ID_HOOK_FAIL_CLOSED, 10004);
        assert_eq!(EVENT_ID_TELEMETRY_DEGRADED, 10005);
    }

    /// Verify emit_security_event does not panic on any platform.
    ///
    /// On non-Windows the Application-log write is skipped; on Windows it falls
    /// back to stderr if the source is not registered.  Either way: no panic.
    #[test]
    fn emit_security_event_is_non_fatal() {
        let event = SecurityEvent {
            event_type: SecurityEventType::PathDeny,
            agent_pid: 0,
            path_hash: None,
            path_category: None,
            host: None,
            session_id: "test-session".to_string(),
            chain_head: "0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            timestamp_unix_ms: 0,
        };
        // Must not panic.
        emit_security_event(&event);
    }
}
