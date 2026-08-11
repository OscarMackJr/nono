//! `SecurityEventLayer::emit_attestation_event` — the D-27/AUD-04
//! `LayerAttestationDowngraded` audit-event channel (Phase 117 CINT-02).
//!
//! # Why this lives here, not in `telemetry/mod.rs` (Plan 17 NR3-05 follow-up)
//!
//! `nono-cli` has no `[lib]` target — only `[[bin]]` targets (`nono` via
//! `src/main.rs`, `nono-agentd` via `src/bin/nono-agentd.rs`) — so shared
//! source files like `telemetry/mod.rs` are `#[path]`-included wholesale
//! into BOTH binaries as two structurally independent compilations of the
//! same text. `emit_attestation_event` used to be defined directly inside
//! `telemetry/mod.rs`'s `impl SecurityEventLayer` block and was called from
//! two real production sites: `exec_strategy_windows::attestation`'s callers
//! (this module tree, `nono`-only) and `agent_daemon::launch`'s step 6.7
//! gate (`nono-agentd`-only).
//!
//! Phase 117 Plan 17 deleted the daemon's call site — `daemon_attest_and_decide`
//! never actually constructed `DaemonAttestationDecision::ProceedDowngraded`
//! (dead code that looked like a supported state, NR3-05), so the variant
//! and its ~60-line audit-emission arm were removed, and the daemon's
//! decision shape is now deliberately two-state (`Proceed`/`Abort`). That
//! left `emit_attestation_event` with ZERO non-test callers inside
//! `nono-agentd`'s own `#[path]`-copy of `telemetry/mod.rs` — a
//! `-D warnings`-fatal `dead_code` lint on that one binary's build.
//!
//! `#[cfg(target_os = "windows")]` (the gate the method carried before this
//! move) doesn't help — both binaries target Windows. A Cargo feature
//! doesn't help — features aren't resolved per-`[[bin]]` target within one
//! package/build. `#[expect(dead_code)]` doesn't help either — it was
//! verified empirically (117-17-SUMMARY.md) to just flip which binary's
//! build fails, since the SAME attributed item is compiled twice with
//! different real call graphs. And `#[allow(dead_code)]` is the one thing
//! CLAUDE.md forbids as a lazy fix.
//!
//! The actual fix: move the method to where its one remaining real caller
//! lives. `exec_strategy_windows/` is a module tree `nono-agentd.rs` never
//! `#[path]`-includes (confirmed: it includes only `agent_daemon/mod.rs`,
//! `telemetry/mod.rs`, and `agent_daemon/telemetry_init.rs`), so this file
//! compiles into the `nono` binary only — matching `emit_attestation_event`'s
//! true, singular production caller
//! (`exec_strategy_windows/launch.rs`'s `apply_startup_attestation_gate`,
//! which calls `security_layer.emit_attestation_event(&downgraded_refs)`
//! unchanged; this move required no edit to that call site).
//!
//! A second `impl SecurityEventLayer` block in a different file is ordinary
//! Rust (inherent impls may split across files within one crate). Plan 17
//! originally widened `SecurityEventLayerInner`'s `chain`/`session_id`/
//! `config` fields to `pub(crate)` so this block could read them directly —
//! but that gave every module in the binary unmediated mutable access to
//! the tamper-evident HMAC chain, with no accessor discipline and no test
//! guarding the boundary (WR-09). Phase 117 Plan 25 reverted those fields to
//! private and added `SecurityEventLayer::advance_and_snapshot`, the sole
//! cross-module chain-advancement accessor; `emit_attestation_event` below
//! calls it instead of touching `inner`'s fields directly.

use crate::telemetry::event::{SecurityEvent, SecurityEventType};
use crate::telemetry::windows::emit_security_event;
use crate::telemetry::SecurityEventLayer;
use std::time::{SystemTime, UNIX_EPOCH};

impl SecurityEventLayer {
    /// Emit a `LayerAttestationDowngraded` event directly into the HMAC chain
    /// (Phase 117 CINT-02 / D-27's structured-telemetry channel).
    ///
    /// Bypasses the tracing-intercept path (`on_event`) for the same reason
    /// `emit_override_event` does — this event carries a `downgraded_layers`
    /// field, not `path`/`host`. Called once per attestation decision, i.e.
    /// once per spawn — this event is **never deduplicated**, unlike
    /// `output::print_attestation_downgrade_banner`'s per-session dedup: the
    /// HMAC-chained audit record intentionally keeps a complete history of
    /// every downgraded spawn (the operator-forensics channel), while the
    /// banner is the noise-sensitive human channel.
    ///
    /// `downgraded_layers` carries specific `LayerId` `Debug`-format names
    /// (e.g. `"AppContainerProfile,DaclPackageSidGrant"`) — see
    /// [`SecurityEvent::downgraded_layers`]'s doc comment for the D-28
    /// justification of why this channel, unlike the banner, is allowed to
    /// name specific layers.
    ///
    /// # Returns
    ///
    /// `Ok(chain_head_hex)` when the event was committed to the chain.
    /// `Err(&'static str)` if the mutex is poisoned.
    ///
    /// # AUD-04 contract
    ///
    /// Callers MUST treat `Err` as non-fatal for the *launch* decision — an
    /// audit-record failure must never itself block a downgraded session
    /// from proceeding, since blocking here would trade an honesty gap for
    /// an availability regression. Callers MUST still surface the downgrade
    /// to the operator through the banner even if this call fails, per the
    /// `#[must_use]` contract below — silently discarding the `Err` is the
    /// one thing never permitted.
    ///
    /// # Telemetry-disabled behaviour (D-14 degrade-not-abort)
    ///
    /// When telemetry is configured disabled, the HMAC chain still advances
    /// (for sequence correctness and audit ordering) but no ETW/AppLog emit
    /// occurs — mirrors `emit_override_event`'s policy.
    #[must_use = "AUD-04: Err means the audit record was not committed — callers MUST \
                  surface the downgrade to the operator through the banner even if this \
                  call fails, never silently proceed"]
    pub fn emit_attestation_event(
        &self,
        downgraded_layers: &[&str],
    ) -> Result<String, &'static str> {
        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let downgraded_layers_value = downgraded_layers.join(",");

        // Build canonical event bytes for chain advancement AND emit the
        // Event Log record, both under the sole cross-module chain-advance
        // accessor's ONE lock acquisition (WR-09 accessor discipline; WR-21
        // point 1 atomicity — see `advance_and_emit`'s doc comment). The
        // emit closure runs while the chain mutex is still held, matching
        // `emit_override_event`'s existing single-critical-section shape.
        self.advance_and_emit(
            |session_id| {
                format!(
                    "{event_type:?}|{downgraded}|{session_id}|{ts}",
                    event_type = SecurityEventType::LayerAttestationDowngraded,
                    downgraded = downgraded_layers_value,
                    ts = timestamp_unix_ms,
                )
                .into_bytes()
            },
            |session_id, chain_head, enabled| {
                if enabled {
                    let security_event = SecurityEvent {
                        event_type: SecurityEventType::LayerAttestationDowngraded,
                        agent_pid: std::process::id(),
                        path_hash: None,
                        path_category: None,
                        host: None,
                        session_id: session_id.to_string(),
                        chain_head: chain_head.to_string(),
                        timestamp_unix_ms,
                        downgraded_layers: Some(downgraded_layers_value.clone()),
                    };
                    emit_security_event(&security_event);
                }
                chain_head.to_string()
            },
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nono::{TelemetryConfig, TelemetrySeverity};

    /// Behavior test: chain advances by exactly one call, regardless of
    /// `config.enabled` (mirrors `emit_override_event_advances_chain_by_one`).
    #[test]
    fn emit_attestation_event_advances_chain_by_one_enabled() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig {
                enabled: true,
                channel: "Application".to_string(),
                min_severity: TelemetrySeverity::Warning,
            },
            "test-attestation-session".to_string(),
        );
        assert_eq!(layer.chain_sequence(), 0, "genesis must be 0");
        let result = layer.emit_attestation_event(&["AppContainerProfile"]);
        assert!(result.is_ok(), "emit_attestation_event must succeed");
        assert_eq!(
            layer.chain_sequence(),
            1,
            "chain must advance by exactly 1 (enabled=true)"
        );
    }

    #[test]
    fn emit_attestation_event_advances_chain_by_one_disabled() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig {
                enabled: false,
                channel: "Application".to_string(),
                min_severity: TelemetrySeverity::Warning,
            },
            "test-attestation-session-disabled".to_string(),
        );
        assert_eq!(layer.chain_sequence(), 0, "genesis must be 0");
        let result = layer.emit_attestation_event(&["WfpEgressFilters"]);
        assert!(
            result.is_ok(),
            "emit_attestation_event must succeed even when telemetry is disabled (D-14)"
        );
        assert_eq!(
            layer.chain_sequence(),
            1,
            "chain must still advance by exactly 1 (enabled=false, D-14 degrade-not-abort)"
        );
    }

    #[test]
    fn emit_attestation_event_err_on_poisoned_mutex() {
        use std::sync::Arc;

        let layer = Arc::new(SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-attestation-poison".to_string(),
        ));

        let layer_clone = Arc::clone(&layer);
        let _ = std::panic::catch_unwind(move || {
            let _guard = layer_clone.inner.lock().unwrap();
            panic!("intentionally poison the mutex");
        });

        let result = layer.emit_attestation_event(&["RestrictedToken"]);
        assert!(
            result.is_err(),
            "poisoned mutex must return Err (AUD-04 fail-closed)"
        );
    }
}
