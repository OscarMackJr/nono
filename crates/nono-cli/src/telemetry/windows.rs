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
