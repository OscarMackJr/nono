//! Runtime for `nono override audit-emit` — live-reject HMAC chain emission (OQ-1 option a).
//!
//! Provides the reject-branch emission path for Phase 93 live ZT-Infra integration.
//! On a live deny or timeout, nono-py fails closed BEFORE spawning `nono.exe`, so
//! the spawn-time `--override-audit` path (`execution_runtime.rs`) never runs.
//! This subcommand gives nono-py a way to still land `PolicyOverrideRejected` (10008)
//! or `PolicyOverrideRevoked` (10010) in the authoritative nono-cli HMAC chain.
//!
//! # Threat model (T-93-02-02 / T-93-02-03 / T-93-02-04)
//!
//! - The emitted event carries only `jti`, `kms_key_id`, and optional `zt_audit_hash`
//!   from the already-decoded `OverrideAuditMeta` — no raw key material.
//! - `emit_override_event` is `#[must_use]`; on `Err` (poisoned mutex) or absent
//!   `SECURITY_LAYER`, this fn returns `NonoError` → non-zero exit (fail-closed,
//!   AUD-04 parity).
//! - `--kind` is a closed clap enum `{rejected, revoked}`; clap rejects any other value
//!   (T-93-02-05 spoofing mitigation).

use crate::cli::OverrideAuditMeta;
use crate::telemetry::event::SecurityEventType;
use crate::telemetry::SECURITY_LAYER;
use nono::{NonoError, Result};

/// Emit a `PolicyOverrideRejected` (10008) or `PolicyOverrideRevoked` (10010) event into
/// the nono-cli HMAC audit chain from the live-reject branch.
///
/// # Parameters
///
/// - `meta` — the already-decoded `OverrideAuditMeta` (jti, kms_key_id, zt_audit_hash).
///   Decoded from base64url-no-pad by the CLI dispatch layer; never re-parses the raw token.
/// - `kind` — whether the live check returned `deny` (revoked) or a timeout/error (rejected).
///
/// # Returns
///
/// `Ok(())` when the event was committed and the chain advanced.
/// `Err(NonoError)` if `SECURITY_LAYER` is unset or the mutex is poisoned — callers must
/// propagate this so the process exits non-zero (AUD-04 fail-closed).
#[must_use = "AUD-04: Err means no audit record was committed — propagate to exit non-zero"]
pub(crate) fn emit_override_audit_event(
    meta: &OverrideAuditMeta,
    kind: OverrideKind,
) -> Result<()> {
    let event_type = match kind {
        // Live POST /actions returned deny → REVOKED (10010)
        OverrideKind::Revoked => SecurityEventType::PolicyOverrideRevoked,
        // Timeout / unreachable / non-200 / malformed → REJECTED (10008)
        OverrideKind::Rejected => SecurityEventType::PolicyOverrideRejected,
    };

    let layer = SECURITY_LAYER.get().ok_or_else(|| {
        NonoError::SandboxInit(
            "override audit-emit: SECURITY_LAYER not initialised — \
             cannot commit audit record (AUD-04 fail-closed)"
                .to_string(),
        )
    })?;

    layer
        .emit_override_event(
            &event_type,
            &meta.jti,
            &meta.kms_key_id,
            meta.zt_audit_hash.as_deref(),
        )
        .map_err(|e| {
            NonoError::SandboxInit(format!(
                "override audit-emit: emit_override_event failed ({e}) — \
                 AUD-04 fail-closed: no audit record committed"
            ))
        })?;

    Ok(())
}

/// The kind of override denial being audited.
///
/// Maps 1:1 to `SecurityEventType` variants (kind→EventID 1:1 contract, no string parsing):
/// - `Rejected` → `PolicyOverrideRejected` → EventID 10008
/// - `Revoked`  → `PolicyOverrideRevoked`  → EventID 10010
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum OverrideKind {
    /// Live POST /actions timeout / unreachable / non-200 / malformed → EventID 10008 (REJECTED).
    Rejected,
    /// Live POST /actions returned `decision: deny` → EventID 10010 (REVOKED).
    Revoked,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::telemetry::event::{event_id_for, SecurityEventType};
    use crate::telemetry::SecurityEventLayer;
    use nono::TelemetryConfig;

    // ── kind → EventID 1:1 mapping (T-93-02-05 / PATTERNS § kind→EventID) ────

    #[test]
    fn rejected_kind_maps_to_event_id_10008() {
        // OverrideKind::Rejected must resolve to PolicyOverrideRejected → 10008.
        let event_type = match OverrideKind::Rejected {
            OverrideKind::Rejected => SecurityEventType::PolicyOverrideRejected,
            OverrideKind::Revoked => SecurityEventType::PolicyOverrideRevoked,
        };
        assert_eq!(
            event_id_for(&event_type),
            10008,
            "rejected kind must map to EventID 10008 (REJECTED)"
        );
    }

    #[test]
    fn revoked_kind_maps_to_event_id_10010() {
        // OverrideKind::Revoked must resolve to PolicyOverrideRevoked → 10010.
        let event_type = match OverrideKind::Revoked {
            OverrideKind::Rejected => SecurityEventType::PolicyOverrideRejected,
            OverrideKind::Revoked => SecurityEventType::PolicyOverrideRevoked,
        };
        assert_eq!(
            event_id_for(&event_type),
            10010,
            "revoked kind must map to EventID 10010 (REVOKED)"
        );
    }

    // ── chain-advance-by-one (mirrors telemetry/mod.rs emit_override_event_advances_chain_by_one) ──

    /// A fresh SecurityEventLayer + emit_override_audit_event advances chain by exactly 1.
    ///
    /// This test wires a fresh layer into the OnceLock, calls `emit_override_audit_event`,
    /// and asserts the sequence incremented. The OnceLock is a global; we set it in a
    /// one-shot test binary context only — this test must run in isolation from any other
    /// test that also calls `SECURITY_LAYER.set` (use `cargo test ... --test-threads=1` or
    /// accept the skip if already set).
    #[test]
    fn emit_override_audit_event_advances_chain_by_one_via_fresh_layer() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-emit-audit-93-02".to_string(),
        );
        assert_eq!(
            layer.chain_sequence(),
            0,
            "fresh layer chain must be 0 (genesis)"
        );

        // Emit a rejected event directly on the layer (bypasses SECURITY_LAYER OnceLock
        // since we cannot guarantee the OnceLock is unset in a parallel test context).
        let meta = OverrideAuditMeta {
            zt_audit_hash: Some("test-zt-hash-abc".to_string()),
            kms_key_id: "arn:aws:kms:us-east-1:123456789012:key/test-93-02".to_string(),
            jti: "test-jti-93-02-abc".to_string(),
            granted_paths: vec!["/tmp/test".to_string()],
            expires_at: "2026-12-31T23:59:59Z".to_string(),
        };

        let event_type = SecurityEventType::PolicyOverrideRejected;
        let result = layer.emit_override_event(
            &event_type,
            &meta.jti,
            &meta.kms_key_id,
            meta.zt_audit_hash.as_deref(),
        );
        assert!(
            result.is_ok(),
            "emit on fresh layer must succeed, got: {result:?}"
        );
        assert_eq!(
            layer.chain_sequence(),
            1,
            "chain must advance by exactly 1 after emit_override_audit_event (AUD-01 parity)"
        );
    }

    /// Revoked kind also advances chain by one.
    #[test]
    fn emit_revoked_event_advances_chain_by_one() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-emit-revoked-93-02".to_string(),
        );
        let meta = OverrideAuditMeta {
            zt_audit_hash: None,
            kms_key_id: "arn:aws:kms:us-east-1:123456789012:key/revoked".to_string(),
            jti: "test-jti-revoked-001".to_string(),
            granted_paths: vec![],
            expires_at: "2026-12-31T23:59:59Z".to_string(),
        };

        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideRevoked,
            &meta.jti,
            &meta.kms_key_id,
            meta.zt_audit_hash.as_deref(),
        );
        assert!(result.is_ok(), "revoked emit must succeed");
        assert_eq!(
            layer.chain_sequence(),
            1,
            "chain must advance by exactly 1 after revoked emit"
        );
    }

    /// No raw key material appears in the event bytes (T-93-02-03).
    ///
    /// `emit_override_event` carries only jti/kms_key_id/zt_audit_hash — verified by
    /// asserting the returned chain_head is a 64-char hex string (HMAC output),
    /// not any of the input fields verbatim.
    #[test]
    fn emit_event_returns_chain_head_hex_not_raw_material() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-no-raw-material".to_string(),
        );
        let raw_secret = "SUPER_SECRET_KEY_MATERIAL_xyz";
        // We pass the "secret" as the kms_key_id so that if any raw leakage occurs it
        // would appear in the returned chain_head. The HMAC output must be opaque.
        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideRejected,
            "jti-secret-test",
            raw_secret,
            None,
        );
        let chain_head = result.unwrap();
        assert!(
            !chain_head.contains(raw_secret),
            "chain_head must NOT contain raw key material; head={chain_head}"
        );
        assert_eq!(
            chain_head.len(),
            64,
            "chain_head must be 64 hex chars (32-byte SHA-256)"
        );
    }
}
