//! Windows security-event emitter stub (Phase 84 Plan 01).
//!
// Plan 01 stub — called from SecurityEventLayer::on_event (mod.rs) but
// the binary never reaches on_event yet (no tracing::Layer registered until
// Plan 02 wires it into init_tracing).  The #[allow(dead_code)] is tracking
// debt removed when Plan 02 registers the layer.
#![allow(dead_code)]
//!
//! Plan 01 delivers the schema and chain contracts only.  The real
//! `RegisterEventSourceW` + `ReportEventW` + ETW emission is implemented in
//! Plan 02, after the operator checkpoint approves `eventlog` 0.4.0 and
//! `tracing-etw` 0.2.3 (Task 1 approved — Plan 02 adds them to Cargo.toml).

// TODO(84-02): real RegisterEventSourceW + ReportEventW + ETW emit
// Pattern ref: crates/nono-cli/src/bin/nono-wfp-service.rs §write_event_log

/// Emit a security event to the Windows Application Event Log and ETW provider
/// (stub — always no-op in Plan 01; implemented in Plan 02).
///
/// # Non-fatal contract (D-03)
///
/// If the Application source is unavailable (dev/test or broken install),
/// the real implementation surfaces `NonoError::TelemetryUnavailable` to
/// stderr and continues the confined run.  This stub returns silently to
/// maintain the same non-fatal interface.
#[cfg(target_os = "windows")]
pub fn emit_security_event(_event: &super::event::SecurityEvent) {
    // TODO(84-02): real RegisterEventSourceW + ReportEventW + ETW emit
}

/// Non-Windows stub: no-op.
#[cfg(not(target_os = "windows"))]
pub fn emit_security_event(_event: &super::event::SecurityEvent) {}

// ── Tests (RED phase — Plan 84-02) ────────────────────────────────────────────
//
// These tests verify the REAL Plan-02 behavior:
//   1. EVENT_LOG_SOURCE constant = "nono" (does not exist in the stub → RED)
//   2. All five EVENT_ID_* constants are present and correctly mapped.
//   3. emit_security_event is non-fatal (no panic) on any platform.
//
// The tests will FAIL (compile error) until the Plan-02 implementation adds
// the constants and the real emit function.

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::telemetry::event::{SecurityEvent, SecurityEventType};

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
    /// On non-Windows the stub is a no-op; on Windows the real impl falls back
    /// to stderr if the source is not registered.  Either way: no panic.
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
