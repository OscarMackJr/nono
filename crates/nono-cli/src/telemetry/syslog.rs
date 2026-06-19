//! RFC 5424 syslog stub for non-Windows security-event emission (Phase 84).
//!
//! This module is a compile-time stub.  RFC 5424 syslog emission is deferred
//! to TELEM-FU-01; this cycle uses Windows Event Log + ETW only.

// TODO(TELEM-FU-01): implement RFC 5424 syslog emission for Linux/macOS
// to enable SIEM ingestion on non-Windows platforms without WEF.

/// Emit a security event to the local syslog daemon (stub — always no-op).
///
/// Full RFC 5424 emission is deferred to TELEM-FU-01.
#[cfg(unix)]
#[allow(dead_code)]
pub fn emit_syslog_event(_event: &super::event::SecurityEvent) {
    // TODO(TELEM-FU-01): RFC 5424 syslog emission
}
