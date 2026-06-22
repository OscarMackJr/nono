// The dead_code allow from Plan 01 is intentionally removed here:
// SecurityEventLayer is now registered in init_tracing() (Plan 02).
//! SIEM/EDR telemetry layer for nono (Phase 84).
//!
//! This module implements [`SecurityEventLayer`], a [`tracing_subscriber::Layer`]
//! that intercepts `tracing` events on the `nono_security::*` target and emits
//! structured, secret-scrubbed, HMAC-chained [`event::SecurityEvent`] records
//! to Windows telemetry sinks (ETW + Application Event Log).
//!
//! # Architecture
//!
//! ```text
//! nono_security::path_deny   ──┐
//! nono_security::network_deny ─┤  tracing events
//! nono_security::hook_fail_closed ─┘
//!         │
//!         ▼
//! SecurityEventLayer::on_event()
//!   └─ advance_chain() ── HMAC-SHA256 chain (D-05)
//!   └─ scrub_value()   ── redact free-text (D-10)
//!   └─ path_hash_for() ── hash path (D-08)
//!   └─ windows::emit_security_event() ── ETW + Application Log (Plan 02)
//! ```
//!
//! # Domain separator independence (D-06)
//!
//! This module uses **different** domain separators from `audit_integrity.rs`
//! to keep the telemetry HMAC chain independent from the unkeyed SHA-256
//! audit ledger.  The separators below must NEVER be changed to match the
//! `nono.audit.*` prefix.

pub mod event;
pub mod syslog;
pub mod windows;

pub use event::{classify_path, path_hash_for, SecurityEvent, SecurityEventType};

/// Global `SecurityEventLayer` instance, set once by `init_tracing` /
/// `init_daemon_telemetry` (Phase 92 Plan 03 / OQ-1 resolution).
///
/// Used by `execute_sandboxed` to call `emit_override_event` (AUD-04 gate).
/// `SecurityEventLayer` is cheaply cloneable (wraps `Arc<Mutex<...>>` inner),
/// so this `OnceLock` stores one clone while the tracing registry takes another;
/// both clones share the same underlying chain state.
///
/// `OnceLock::set` silently fails if already set (daemon double-init guard
/// pattern mirrors `telemetry_init.rs::INIT`).
pub(crate) static SECURITY_LAYER: std::sync::OnceLock<SecurityEventLayer> =
    std::sync::OnceLock::new();

use hmac::{Hmac, Mac};
use nono::TelemetryConfig;
use sha2::Sha256;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use zeroize::{Zeroize, Zeroizing};

type HmacSha256 = Hmac<Sha256>;

// ── Domain separator constants (D-06) ────────────────────────────────────────

/// HMAC domain separator for individual event content hashing.
///
/// Distinct from `audit_integrity.rs` `EVENT_DOMAIN = b"nono.audit.event.alpha\n"`.
/// Must remain different per D-06 to keep the two chains independent.
pub(crate) const TELEMETRY_EVENT_DOMAIN: &[u8] = b"nono.telemetry.event.alpha\n";

/// HMAC domain separator for the running chain head update.
///
/// Distinct from `audit_integrity.rs` `CHAIN_DOMAIN = b"nono.audit.chain.alpha\n"`.
/// Must remain different per D-06 to keep the two chains independent.
pub(crate) const TELEMETRY_CHAIN_DOMAIN: &[u8] = b"nono.telemetry.chain.alpha\n";

// ── ChainState ────────────────────────────────────────────────────────────────

/// Mutable state for the per-session HMAC chain (D-05 / TELEM-02).
///
/// - `key` is an ephemeral 32-byte random key generated at `SecurityEventLayer`
///   construction time.  It is held in [`Zeroizing`] and explicitly zeroed in
///   [`Drop`] (belt-and-suspenders).
/// - `head` is the running chain head (genesis = `[0u8; 32]`).
/// - `sequence` is a monotonically increasing event counter, incremented with
///   [`u64::saturating_add`] per CLAUDE.md § Arithmetic.
pub(crate) struct ChainState {
    /// Ephemeral per-session HMAC key — zeroized on drop (D-05).
    pub(crate) key: Zeroizing<[u8; 32]>,
    /// Current chain head (updated by [`advance_chain`]).
    pub(crate) head: [u8; 32],
    /// Monotonically increasing event sequence number.
    pub(crate) sequence: u64,
}

impl Drop for ChainState {
    fn drop(&mut self) {
        // Belt-and-suspenders: Zeroizing<T> zeroes on Drop automatically,
        // but an explicit call here makes the contract auditable.
        self.key.zeroize();
        self.head.zeroize();
    }
}

// ── advance_chain ─────────────────────────────────────────────────────────────

/// Advance the per-session HMAC-SHA256 chain by appending a new event (D-05/D-06).
///
/// The new chain head is:
/// ```text
/// HMAC-SHA256(session_key,
///     TELEMETRY_CHAIN_DOMAIN || prev_head || TELEMETRY_EVENT_DOMAIN || event_bytes)
/// ```
///
/// The domain separators ensure the telemetry chain is independent from
/// `audit_integrity.rs` (D-06) and from any chain that does not use both
/// domain-prefix constants.
///
/// # Error handling (D-14 / `clippy::unwrap_used` prohibition)
///
/// `new_from_slice` returns `InvalidLength` only when the key is empty, which
/// cannot happen because `ChainState.key` is always a 32-byte array filled by
/// `OsRng`.  However, we use a `match`-based fallback to a zeroed key (D-14
/// degrade-not-abort) rather than `.expect()` or `.unwrap()`, which are
/// forbidden in production code by CLAUDE.md § Unwrap Policy.
pub(crate) fn advance_chain(chain: &mut ChainState, event_bytes: &[u8]) {
    use hmac::KeyInit as _;
    let mut mac = match HmacSha256::new_from_slice(chain.key.as_ref()) {
        Ok(m) => m,
        Err(e) => {
            // InvalidLength only if key is empty — structurally impossible for
            // our 32-byte OsRng key, but we handle it gracefully per D-14.
            eprintln!("nono: telemetry: HMAC key length error ({e}), degrading to zeroed key");
            // SAFETY: a 32-byte all-zero slice always satisfies HMAC-SHA256's
            // key constraint (any non-empty key is valid).
            match HmacSha256::new_from_slice(&[0u8; 32]) {
                Ok(m) => m,
                Err(_) => {
                    // 32-byte zeroed slice cannot fail — this branch is unreachable
                    // but we must handle it without panic.
                    return;
                }
            }
        }
    };
    mac.update(TELEMETRY_CHAIN_DOMAIN);
    mac.update(&chain.head);
    mac.update(TELEMETRY_EVENT_DOMAIN);
    mac.update(event_bytes);
    let result = mac.finalize().into_bytes();
    chain.head.copy_from_slice(&result);
    chain.sequence = chain.sequence.saturating_add(1);
}

/// Convert a chain head to a lowercase hex string.
#[must_use]
pub(crate) fn chain_head_hex(head: &[u8; 32]) -> String {
    head.iter().map(|b| format!("{b:02x}")).collect()
}

/// Map a [`SecurityEventType`] to its telemetry severity (WR-02 / TELEM-04).
///
/// All current denial events are `Warning`-level. This is the single point to
/// raise an event's severity to `Error` should a future event type warrant it.
/// The layer suppresses any event whose severity is below the policy's
/// `min_severity` (see `on_event`).
fn severity_for(t: &SecurityEventType) -> nono::TelemetrySeverity {
    use nono::TelemetrySeverity;
    match t {
        SecurityEventType::PathDeny
        | SecurityEventType::NetworkDeny
        | SecurityEventType::LabelViolation
        | SecurityEventType::HookFailClosed
        | SecurityEventType::TelemetryDegraded => TelemetrySeverity::Warning,
        // Phase 92: override lifecycle events are Warning-level (authorization events).
        SecurityEventType::PolicyOverridePresented
        | SecurityEventType::PolicyOverrideVerified
        | SecurityEventType::PolicyOverrideRejected
        | SecurityEventType::PolicyOverrideExpired
        | SecurityEventType::PolicyOverrideRevoked => TelemetrySeverity::Warning,
    }
}

// ── SecurityEventLayerInner ───────────────────────────────────────────────────

struct SecurityEventLayerInner {
    chain: ChainState,
    session_id: String,
    session_salt: [u8; 32],
    config: TelemetryConfig,
}

// ── SecurityEventLayer ────────────────────────────────────────────────────────

/// A [`tracing_subscriber::Layer`] that intercepts `nono_security::*` events
/// and emits structured, secret-scrubbed, HMAC-chained security events to
/// Windows telemetry sinks.
///
/// # Construction
///
/// Use [`SecurityEventLayer::new`] which generates an ephemeral 32-byte key
/// and salt from the OS random source.
///
/// # Thread safety
///
/// The mutable chain state is wrapped in a [`Mutex`] so the layer can be
/// registered as a global subscriber across multiple threads.
///
/// # Cloneability (Phase 92 Plan 03)
///
/// `SecurityEventLayer` is cheaply cloneable — `clone()` clones the
/// `Arc<Mutex<...>>` inner, so all clones share the SAME mutable chain state.
/// This allows `SECURITY_LAYER` (an `OnceLock<SecurityEventLayer>`) to store
/// one clone while the tracing registry takes another, with both advancing the
/// same HMAC chain. This pattern is safe because the `Mutex` serialises all
/// concurrent access across all clones.
#[derive(Clone)]
pub struct SecurityEventLayer {
    inner: std::sync::Arc<Mutex<SecurityEventLayerInner>>,
}

impl SecurityEventLayer {
    /// Return the current HMAC chain sequence number.
    ///
    /// The genesis value is `0`.  Each call to [`advance_chain`] increments this
    /// by one (saturating).  Used by the D-01 non-host-gated integration test to
    /// assert that an in-process `nono_security::network_deny` event actually
    /// reached `on_event` and advanced the chain (DRAIN-04).
    ///
    /// Returns `0` if the internal mutex is poisoned (fail-silent, never panics).
    ///
    /// # Test accessor
    ///
    /// This method is intentionally `#[cfg(test)]` — it exists solely to expose
    /// chain state to inline integration tests.  It is not called in production
    /// code paths.  This avoids a `dead_code` lint (CLAUDE.md: avoid
    /// `#[allow(dead_code)]`) while keeping the accessor available to all
    /// `#[cfg(test)]` modules in the same crate compilation unit.
    #[cfg(test)]
    pub(crate) fn chain_sequence(&self) -> u64 {
        match self.inner.lock() {
            Ok(guard) => guard.chain.sequence,
            Err(_) => 0, // Mutex poisoned — return genesis value, never panic
        }
    }

    /// Construct a new `SecurityEventLayer` with a freshly generated ephemeral
    /// key and session salt.
    ///
    /// The key and salt are generated from the OS random source via the
    /// `rand` crate [`rand::RngCore`] trait so the layer compiles on all
    /// platforms (not just Windows).
    ///
    /// # Arguments
    ///
    /// - `config` — the telemetry configuration from `MachineEgressPolicy`
    ///   (D-12); controls `enabled`, `channel`, and `min_severity`.
    /// - `session_id` — an opaque per-session identifier (e.g. a UUID or
    ///   the daemon's pipe name) for correlating events within one run.
    #[must_use]
    pub fn new(config: TelemetryConfig, session_id: String) -> Self {
        use rand::RngExt as _;
        let mut rng = rand::rng();

        // Generate independent 32-byte key and salt.
        let mut key_bytes = [0u8; 32];
        let mut salt_bytes = [0u8; 32];
        rng.fill(&mut key_bytes[..]);
        rng.fill(&mut salt_bytes[..]);

        let chain = ChainState {
            key: Zeroizing::new(key_bytes),
            head: [0u8; 32], // genesis IV
            sequence: 0,
        };

        Self {
            inner: std::sync::Arc::new(Mutex::new(SecurityEventLayerInner {
                chain,
                session_id,
                session_salt: salt_bytes,
                config,
            })),
        }
    }

    /// Emit a PolicyOverride lifecycle event directly into the HMAC chain.
    ///
    /// Bypasses the tracing-intercept path (`on_event`) because override events
    /// carry `zt_audit_hash`/`kms_key_id` fields, not `path`/`host`. The direct
    /// method is accessible from `execute_sandboxed` where the layer instance
    /// is in scope via `SECURITY_LAYER.get()`.
    ///
    /// # Returns
    ///
    /// `Ok(chain_head_hex)` when the event was emitted and the chain advanced.
    /// `Err(&'static str)` if the mutex is poisoned or the event cannot be
    /// committed.
    ///
    /// # AUD-04 contract
    ///
    /// Callers MUST treat `Err` as FATAL and abort before spawning the sandboxed
    /// child. A poisoned mutex means telemetry is in an unrecoverable state — the
    /// override path must not proceed without a committed audit record (D-02
    /// bilateral gate; AUD-04 fail-closed).
    ///
    /// # Telemetry-disabled behaviour (D-14 degrade-not-abort)
    ///
    /// When `inner.config.enabled` is `false`, the HMAC chain still advances
    /// (for sequence correctness and audit ordering) but no ETW/AppLog emit
    /// occurs. This is a policy choice, not an AUD-04 failure — the function
    /// returns `Ok`.
    // This method is called from execution_runtime.rs (compiled only for the `nono`
    // binary, not for `nono-agentd`). The `dead_code` lint fires for nono-agentd
    // because that binary does not reach execution_runtime.rs. The method IS used
    // in production (nono binary + unit tests) — this is a multi-binary compilation
    // artifact, not actual dead code (per CLAUDE.md rule: tests use it).
    #[allow(dead_code)]
    #[must_use = "AUD-04: Err means the audit record was not committed — callers MUST \
                  return Err before spawning (never silently proceed)"]
    pub fn emit_override_event(
        &self,
        event_type: &SecurityEventType,
        jti: &str,
        kms_key_id: &str,
        zt_audit_hash: Option<&str>,
    ) -> Result<String, &'static str> {
        let mut inner = self.inner.lock().map_err(|_| "mutex poisoned")?;

        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Build canonical event bytes for chain advancement (same format as on_event).
        // Fields: event_type | jti | kms_key_id | zt_audit_hash | session_id | ts
        let pre_chain_bytes = format!(
            "{event_type:?}|{jti}|{kms_key_id}|{zt}|{session_id}|{ts}",
            zt = zt_audit_hash.unwrap_or(""),
            session_id = inner.session_id,
            ts = timestamp_unix_ms,
        );

        advance_chain(&mut inner.chain, pre_chain_bytes.as_bytes());
        let chain_head = chain_head_hex(&inner.chain.head);

        if inner.config.enabled {
            let security_event = SecurityEvent {
                event_type: event_type.clone(),
                agent_pid: std::process::id(),
                // Override events carry no path — kms_key_id/jti are in
                // the chain bytes above. path_hash / path_category / host
                // are None (AUD-03 redaction: raw secrets never in log fields).
                path_hash: None,
                path_category: None,
                host: None,
                session_id: inner.session_id.clone(),
                chain_head: chain_head.clone(),
                timestamp_unix_ms,
            };
            windows::emit_security_event(&security_event);
        }

        Ok(chain_head)
    }
}

impl<S: Subscriber> Layer<S> for SecurityEventLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Only intercept events on the nono_security::* target prefix.
        if !event.metadata().target().starts_with("nono_security::") {
            return;
        }

        // Guard: telemetry disabled by policy → skip.
        // We check inside the lock to avoid a TOCTOU on config.enabled.
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return, // Mutex poisoned — fail-silent (D-03 non-fatal)
        };

        if !inner.config.enabled {
            return;
        }

        // Determine SecurityEventType from the tracing target suffix.
        let event_type = match event.metadata().target() {
            t if t.ends_with("path_deny") => SecurityEventType::PathDeny,
            t if t.ends_with("network_deny") => SecurityEventType::NetworkDeny,
            t if t.ends_with("label_violation") => SecurityEventType::LabelViolation,
            t if t.ends_with("hook_fail_closed") => SecurityEventType::HookFailClosed,
            t if t.ends_with("telemetry_degraded") => SecurityEventType::TelemetryDegraded,
            _ => return, // Unknown sub-target — skip.
        };

        // WR-02 / TELEM-04 level filtering: emit only when the event's severity
        // meets the policy's min_severity threshold (Debug < Info < Warning < Error).
        // Checked inside the same lock as `enabled` to avoid a config TOCTOU.
        if severity_for(&event_type) < inner.config.min_severity {
            return;
        }

        // Extract structured fields via a field visitor.
        let mut visitor = SecurityEventVisitor::default();
        event.record(&mut visitor);

        // Hash path if present (D-08); scrub host (D-10 — cleartext by exception).
        let path_hash = visitor
            .path
            .as_deref()
            .map(std::path::Path::new)
            .map(|p| path_hash_for(&inner.session_salt, p));
        let path_category = visitor
            .path
            .as_deref()
            .map(std::path::Path::new)
            .map(classify_path);

        // Scrub free-text fields (D-10 / Pitfall 11).
        // host stays cleartext (SC-1) — analyst needs it.
        // reason/label go through scrub_value.
        let host = visitor.host.map(|h| nono::scrub_value(&h).into_owned());

        // Timestamp.
        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Build the event bytes for the chain (canonical JSON of the payload
        // fields before ChainHead is known).
        let pre_chain_bytes = format!(
            "{event_type:?}|{agent_pid}|{path_hash_str}|{host_str}|{session_id}|{ts}",
            event_type = event_type,
            agent_pid = std::process::id(),
            path_hash_str = path_hash.as_deref().unwrap_or(""),
            host_str = host.as_deref().unwrap_or(""),
            session_id = inner.session_id,
            ts = timestamp_unix_ms,
        );

        advance_chain(&mut inner.chain, pre_chain_bytes.as_bytes());
        let chain_head = chain_head_hex(&inner.chain.head);

        let security_event = SecurityEvent {
            event_type,
            agent_pid: std::process::id(),
            path_hash,
            path_category,
            host,
            session_id: inner.session_id.clone(),
            chain_head,
            timestamp_unix_ms,
        };

        // Emit to Windows Application log + ETW (dual-emit, D-01).
        windows::emit_security_event(&security_event);
    }
}

// ── Field visitor ─────────────────────────────────────────────────────────────

/// Visitor that extracts structured fields from a tracing `Event`.
#[derive(Default)]
struct SecurityEventVisitor {
    /// Raw path string from a `path = …` field (will be hashed — D-08).
    path: Option<String>,
    /// Host/domain from a `host = …` field (cleartext — D-10 / SC-1).
    host: Option<String>,
}

impl tracing::field::Visit for SecurityEventVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "path" => self.path = Some(value.to_string()),
            "host" => self.host = Some(value.to_string()),
            _ => {} // Ignore other fields.
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "path" => self.path = Some(format!("{value:?}")),
            "host" => self.host = Some(format!("{value:?}")),
            _ => {}
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nono::TelemetrySeverity;

    // ── Domain separator independence (D-06) ──────────────────────────────────

    #[test]
    fn telemetry_domains_differ_from_audit_domains() {
        const AUDIT_EVENT_DOMAIN: &[u8] = b"nono.audit.event.alpha\n";
        const AUDIT_CHAIN_DOMAIN: &[u8] = b"nono.audit.chain.alpha\n";
        assert_ne!(
            TELEMETRY_EVENT_DOMAIN, AUDIT_EVENT_DOMAIN,
            "telemetry EVENT domain must differ from audit EVENT domain (D-06)"
        );
        assert_ne!(
            TELEMETRY_CHAIN_DOMAIN, AUDIT_CHAIN_DOMAIN,
            "telemetry CHAIN domain must differ from audit CHAIN domain (D-06)"
        );
    }

    // ── Severity filtering (WR-02 / TELEM-04) ─────────────────────────────────

    #[test]
    fn severity_for_all_denial_types_is_warning() {
        for t in [
            SecurityEventType::PathDeny,
            SecurityEventType::NetworkDeny,
            SecurityEventType::LabelViolation,
            SecurityEventType::HookFailClosed,
            SecurityEventType::TelemetryDegraded,
        ] {
            assert_eq!(
                severity_for(&t),
                TelemetrySeverity::Warning,
                "denial event {t:?} must be Warning severity"
            );
        }
    }

    #[test]
    fn severity_for_override_lifecycle_events_is_warning() {
        for t in [
            SecurityEventType::PolicyOverridePresented,
            SecurityEventType::PolicyOverrideVerified,
            SecurityEventType::PolicyOverrideRejected,
            SecurityEventType::PolicyOverrideExpired,
            SecurityEventType::PolicyOverrideRevoked,
        ] {
            assert_eq!(
                severity_for(&t),
                TelemetrySeverity::Warning,
                "override lifecycle event {t:?} must be Warning severity"
            );
        }
    }

    #[test]
    fn min_severity_filter_predicate_matches_policy_threshold() {
        // The on_event guard is `severity_for(event) < min_severity → suppress`.
        // Warning-level events emit at Debug/Info/Warning thresholds and are
        // suppressed only at the Error threshold.
        let event_sev = severity_for(&SecurityEventType::PathDeny); // Warning
        assert!(event_sev >= TelemetrySeverity::Debug, "emits at Debug min");
        assert!(event_sev >= TelemetrySeverity::Info, "emits at Info min");
        assert!(
            event_sev >= TelemetrySeverity::Warning,
            "emits at Warning min (default)"
        );
        assert!(
            event_sev < TelemetrySeverity::Error,
            "Warning event is suppressed when min_severity=Error"
        );
    }

    #[test]
    fn telemetry_event_domain_value() {
        assert_eq!(
            TELEMETRY_EVENT_DOMAIN, b"nono.telemetry.event.alpha\n",
            "TELEMETRY_EVENT_DOMAIN must match the locked value"
        );
    }

    // ── SecurityEventLayer construction ───────────────────────────────────────

    #[test]
    fn new_produces_nonzero_key_and_salt() {
        let cfg = TelemetryConfig {
            enabled: true,
            channel: "Application".to_string(),
            min_severity: TelemetrySeverity::Warning,
        };
        let layer = SecurityEventLayer::new(cfg, "test-session".to_string());
        let inner = layer.inner.lock().unwrap();
        // Key must be non-zero (session-unique).
        assert_ne!(
            inner.chain.key.as_ref(),
            &[0u8; 32],
            "session key must be non-zero (OsRng-seeded)"
        );
        // Salt must be non-zero.
        assert_ne!(
            inner.session_salt, [0u8; 32],
            "session salt must be non-zero (OsRng-seeded)"
        );
        // Genesis head is all-zero.
        assert_eq!(
            inner.chain.head, [0u8; 32],
            "genesis chain head must be [0u8;32]"
        );
        assert_eq!(inner.chain.sequence, 0, "genesis sequence must be 0");
    }

    // ── chain_sequence accessor (DRAIN-04 D-01) ───────────────────────────────

    #[test]
    fn chain_sequence_genesis_is_zero() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-chain-seq".to_string(),
        );
        // Genesis sequence via the pub(crate) accessor (DRAIN-04 D-01 test hook).
        assert_eq!(
            layer.chain_sequence(),
            0,
            "chain_sequence() must return 0 at genesis (no events emitted)"
        );
    }

    // ── advance_chain ─────────────────────────────────────────────────────────

    #[test]
    fn advance_chain_changes_head_and_increments_sequence() {
        let mut chain = ChainState {
            key: Zeroizing::new([0xABu8; 32]),
            head: [0u8; 32],
            sequence: 0,
        };
        let initial_head = chain.head;
        advance_chain(&mut chain, b"event-1");
        assert_ne!(chain.head, initial_head, "head must change after advance");
        assert_eq!(chain.sequence, 1);
    }

    #[test]
    fn two_events_produce_different_chain_heads() {
        let mut chain = ChainState {
            key: Zeroizing::new([0x55u8; 32]),
            head: [0u8; 32],
            sequence: 0,
        };
        advance_chain(&mut chain, b"event-1");
        let head_after_1 = chain.head;
        advance_chain(&mut chain, b"event-2");
        assert_ne!(
            chain.head, head_after_1,
            "two events must produce different ChainHead values"
        );
    }

    #[test]
    fn advance_chain_uses_key_in_hash() {
        // Same event bytes but different keys must produce different heads.
        let event = b"same-event-bytes";

        let mut chain_a = ChainState {
            key: Zeroizing::new([0x11u8; 32]),
            head: [0u8; 32],
            sequence: 0,
        };
        advance_chain(&mut chain_a, event);

        let mut chain_b = ChainState {
            key: Zeroizing::new([0x22u8; 32]),
            head: [0u8; 32],
            sequence: 0,
        };
        advance_chain(&mut chain_b, event);

        assert_ne!(
            chain_a.head, chain_b.head,
            "different keys must produce different heads for the same event (key mixing)"
        );
    }

    #[test]
    fn chain_head_hex_is_64_chars() {
        let head = [0xABu8; 32];
        let hex = chain_head_hex(&head);
        assert_eq!(hex.len(), 64, "chain_head_hex must be 64 chars (32 bytes)");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "chain_head_hex must be hex: {hex}"
        );
    }

    // ── ChainState key zeroize (D-05) ─────────────────────────────────────────

    #[test]
    fn chain_state_key_is_zeroizing_type() {
        // Compile-time: Zeroizing<[u8;32]> must be the field type.
        // We verify this by constructing and dropping one.
        let chain = ChainState {
            key: Zeroizing::new([0xFFu8; 32]),
            head: [0u8; 32],
            sequence: 0,
        };
        // Confirm the key is non-zero before drop.
        assert_eq!(
            chain.key.as_ref(),
            &[0xFFu8; 32],
            "key must be 0xFF before drop"
        );
        drop(chain);
        // After drop, memory is zeroed (Zeroizing<T> guarantee — we trust the crate).
    }

    // ── RED-phase tests (Plan 84-02) ─────────────────────────────────────────────
    //
    // These tests exercise the REAL Plan-02 advance_chain behavior.
    // The sha2 placeholder produces a different output from the real Hmac<Sha256>
    // implementation for the same input.  The test below pins the HMAC-SHA256
    // output for a known key/head/event triple so it FAILS on the sha2 placeholder
    // and PASSES only after the real implementation is in place.

    /// Verify advance_chain produces the correct HMAC-SHA256 result for a known
    /// key, genesis head, and event bytes.
    ///
    /// This test pins the EXPECTED output of the real `Hmac<Sha256>` computation
    /// using TELEMETRY_CHAIN_DOMAIN + TELEMETRY_EVENT_DOMAIN domain separators.
    /// It fails with the sha2 placeholder (which computes SHA-256 without a keyed
    /// MAC) and passes only after the Plan-02 replacement.
    ///
    /// Expected value computed via:
    ///   key = [0x42u8; 32]
    ///   input fed to HMAC-SHA256 in order:
    ///     TELEMETRY_CHAIN_DOMAIN || prev_head([0u8;32]) ||
    ///     TELEMETRY_EVENT_DOMAIN || b"test-event"
    ///   The expected head is the 32-byte HMAC output.
    #[test]
    fn advance_chain_uses_hmac_not_sha2_placeholder() {
        use hmac::KeyInit as _;

        // Compute the expected HMAC-SHA256 output independently.
        let key = [0x42u8; 32];
        let prev_head = [0u8; 32];
        let event_bytes = b"test-event";
        let mut mac = HmacSha256::new_from_slice(&key).unwrap();
        mac.update(TELEMETRY_CHAIN_DOMAIN);
        mac.update(&prev_head);
        mac.update(TELEMETRY_EVENT_DOMAIN);
        mac.update(event_bytes);
        let expected: [u8; 32] = mac.finalize().into_bytes().into();

        // advance_chain must produce the same result.
        let mut chain = ChainState {
            key: Zeroizing::new(key),
            head: prev_head,
            sequence: 0,
        };
        advance_chain(&mut chain, event_bytes);
        assert_eq!(
            chain.head, expected,
            "advance_chain must use Hmac<Sha256> (TELEMETRY_CHAIN_DOMAIN || prev_head || \
             TELEMETRY_EVENT_DOMAIN || event_bytes); sha2 placeholder produces a different value"
        );
    }

    // ── emit_override_event (Phase 92 Plan 03 Task 2 — AUD-01 / AUD-04) ─────

    /// AUD-01: emit_override_event on a fresh layer advances chain_sequence by 1.
    #[test]
    fn emit_override_event_advances_chain_by_one() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-override-session".to_string(),
        );
        assert_eq!(layer.chain_sequence(), 0, "genesis must be 0");
        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideVerified,
            "test-jti-123",
            "arn:aws:kms:us-east-1:123456789012:key/test",
            Some("abc123deadbeef"),
        );
        assert!(
            result.is_ok(),
            "emit_override_event must succeed on fresh layer, got: {result:?}"
        );
        assert_eq!(
            layer.chain_sequence(),
            1,
            "chain must advance by exactly 1 after emit_override_event (AUD-01)"
        );
    }

    /// AUD-04 fail-closed: poisoned mutex returns Err, never silently Ok.
    #[test]
    fn emit_override_event_err_on_poisoned_mutex() {
        use std::sync::Arc;

        let layer = Arc::new(SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-poison".to_string(),
        ));

        // Poison the mutex by panicking while holding the lock.
        let layer_clone = Arc::clone(&layer);
        let _ = std::panic::catch_unwind(move || {
            let _guard = layer_clone.inner.lock().unwrap();
            panic!("intentionally poison the mutex");
        });

        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideRejected,
            "jti",
            "kms_key_id",
            None,
        );
        assert!(
            result.is_err(),
            "poisoned mutex must return Err (AUD-04 fail-closed)"
        );
    }

    /// AUD-01 idempotent ordering: two calls advance chain_sequence from 0 to 2.
    #[test]
    fn emit_override_event_two_calls_advance_by_two() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-two-calls".to_string(),
        );
        let r1 = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideVerified,
            "jti-1",
            "arn:kms:1",
            None,
        );
        let r2 = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideVerified,
            "jti-2",
            "arn:kms:2",
            Some("zt-hash"),
        );
        assert!(r1.is_ok(), "first emit must succeed");
        assert!(r2.is_ok(), "second emit must succeed");
        assert_eq!(
            layer.chain_sequence(),
            2,
            "chain must advance by 2 after two emit_override_event calls (AUD-01)"
        );
    }

    /// emit_override_event with zt_audit_hash=None returns Ok (CAF v0.1 tokens).
    #[test]
    fn emit_override_event_none_zt_audit_hash_ok() {
        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "test-no-zt-hash".to_string(),
        );
        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideVerified,
            "jti-no-zt",
            "arn:kms:no-zt",
            None, // zt_audit_hash absent (pre-ZT-Infra token)
        );
        assert!(
            result.is_ok(),
            "None zt_audit_hash must return Ok (CAF v0.1 pre-ZT tokens allowed)"
        );
    }
}
