//! SIEM/EDR telemetry layer for nono (Phase 84).
//!
// Plan 01 is schema-only — SecurityEventLayer is constructed and tested but
// not yet registered in init_tracing() (that wiring is Plan 02 Task 1).
// The dead_code allows are intentional and tracked: remove when Plan 02
// calls SecurityEventLayer::new() from cli_bootstrap::init_tracing.
#![allow(dead_code)]
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
//!   └─ windows::emit_security_event() ── ETW + Application Log (stub, Plan 02)
//! ```
//!
//! # Domain separator independence (D-06)
//!
//! This module uses **different** domain separators from `audit_integrity.rs`
//! to keep the telemetry HMAC chain independent from the unkeyed SHA-256
//! audit ledger.  The separators below must NEVER be changed to match the
//! `nono.audit.*` prefix.
//!
//! # HMAC placeholder (Plan 01)
//!
//! The `hmac` crate (0.13.0, RustCrypto) is the intended HMAC engine but is
//! not yet added to `Cargo.toml` — that happens in Plan 02 after the operator
//! checkpoint (Task 1 approved, D-MSRV recorded for Plan 02).  This plan
//! implements `advance_chain` with a `sha2`-based placeholder that **will be
//! replaced** in Plan 02.  The placeholder is marked with a TODO comment below.

pub mod event;
pub mod syslog;
pub mod windows;

pub use event::{SecurityEvent, SecurityEventType, classify_path, path_hash_for};

use nono::TelemetryConfig;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use zeroize::{Zeroize, Zeroizing};

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

/// Advance the HMAC chain by appending a new event.
///
/// **Plan 01 placeholder:** Uses `sha2::Sha256` as a stand-in for
/// `Hmac<Sha256>` because the `hmac` crate is not yet in `Cargo.toml`
/// (added in Plan 02 after the operator checkpoint).  The construction below
/// preserves the domain-separator discipline and key-mixing intent of D-05/D-06.
///
/// # TODO(84-02)
///
/// Replace this sha2-placeholder with `Hmac<Sha256>` after operator checkpoint:
/// ```ignore
/// use hmac::{Hmac, Mac};
/// type HmacSha256 = Hmac<Sha256>;
/// let mut mac = HmacSha256::new_from_slice(&chain.key)
///     .expect("HMAC accepts any key length");
/// mac.update(TELEMETRY_CHAIN_DOMAIN);
/// mac.update(&chain.head);
/// mac.update(TELEMETRY_EVENT_DOMAIN);
/// mac.update(event_bytes);
/// chain.head = mac.finalize().into_bytes().into();
/// ```
pub(crate) fn advance_chain(chain: &mut ChainState, event_bytes: &[u8]) {
    // TODO(84-02): replace sha2-placeholder with Hmac<Sha256> after operator checkpoint.
    // sha2 placeholder: SHA-256(key || TELEMETRY_CHAIN_DOMAIN || prev_head ||
    //                          TELEMETRY_EVENT_DOMAIN || event_bytes)
    // This maintains domain separation and key mixing; just not keyed-MAC-secure
    // until the hmac crate is available.
    let mut hasher = Sha256::new();
    hasher.update(chain.key.as_ref());
    hasher.update(TELEMETRY_CHAIN_DOMAIN);
    hasher.update(chain.head);
    hasher.update(TELEMETRY_EVENT_DOMAIN);
    hasher.update(event_bytes);
    let result = hasher.finalize();
    chain.head.copy_from_slice(&result);
    chain.sequence = chain.sequence.saturating_add(1);
}

/// Convert a chain head to a lowercase hex string.
#[must_use]
pub(crate) fn chain_head_hex(head: &[u8; 32]) -> String {
    head.iter().map(|b| format!("{b:02x}")).collect()
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
pub struct SecurityEventLayer {
    inner: Mutex<SecurityEventLayerInner>,
}

impl SecurityEventLayer {
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
            inner: Mutex::new(SecurityEventLayerInner {
                chain,
                session_id,
                session_salt: salt_bytes,
                config,
            }),
        }
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

        // Emit to Windows sinks (stub in Plan 01; real emitter added in Plan 02).
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
            TELEMETRY_EVENT_DOMAIN,
            AUDIT_EVENT_DOMAIN,
            "telemetry EVENT domain must differ from audit EVENT domain (D-06)"
        );
        assert_ne!(
            TELEMETRY_CHAIN_DOMAIN,
            AUDIT_CHAIN_DOMAIN,
            "telemetry CHAIN domain must differ from audit CHAIN domain (D-06)"
        );
    }

    #[test]
    fn telemetry_event_domain_value() {
        assert_eq!(
            TELEMETRY_EVENT_DOMAIN,
            b"nono.telemetry.event.alpha\n",
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
            inner.session_salt,
            [0u8; 32],
            "session salt must be non-zero (OsRng-seeded)"
        );
        // Genesis head is all-zero.
        assert_eq!(
            inner.chain.head,
            [0u8; 32],
            "genesis chain head must be [0u8;32]"
        );
        assert_eq!(inner.chain.sequence, 0, "genesis sequence must be 0");
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
}
