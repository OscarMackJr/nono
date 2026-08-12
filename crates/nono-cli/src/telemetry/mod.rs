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
        // Phase 117: a downgraded confinement claim is Warning-level — it is
        // an honesty-preserving degrade (D-14), not itself an active denial,
        // but must never be filtered below the default Warning threshold.
        SecurityEventType::LayerAttestationDowngraded => TelemetrySeverity::Warning,
    }
}

// ── SecurityEventLayerInner ───────────────────────────────────────────────────

// Phase 117 Plan 25 (WR-09): `chain`/`session_id`/`config` are private again.
// Plan 17 (NR3-05 follow-up) had widened them to `pub(crate)` so
// `exec_strategy_windows::attestation_downgrade_event`'s `impl
// SecurityEventLayer` block — moved there specifically so `nono-agentd`'s
// independent #[path]-copy of this file never compiles
// `emit_attestation_event`, which had zero non-test callers in the daemon
// binary after that plan deleted its only call site
// (`DaemonAttestationDecision::ProceedDowngraded`, now removed) — could read
// them directly. That gave every module in the binary unmediated mutable
// access to the tamper-evident HMAC chain, with no accessor discipline and
// no test guarding the boundary. `advance_and_snapshot` below is the sole
// cross-module accessor: it locks once, always routes the mutation through
// `advance_chain`, and returns a snapshot of the fields callers need instead
// of exposing the fields themselves.
pub(crate) struct SecurityEventLayerInner {
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

    /// Poison this layer's internal chain mutex (test-only helper, Phase 117
    /// Plan 33 / WR-21 point 2).
    ///
    /// Deliberately panics while holding `inner`'s lock, then swallows the
    /// panic via `catch_unwind` so the poisoning itself never propagates to
    /// the caller. After this returns, every lock-taking method on this
    /// layer (or any of its clones — they share the same `Arc<Mutex<...>>`)
    /// returns `Err("mutex poisoned")` (AUD-04 fail-closed).
    ///
    /// # Test accessor
    ///
    /// This is the ONLY cross-module access point for poisoning
    /// `SecurityEventLayer`'s mutex from a test in a different module (e.g.
    /// `exec_strategy_windows::attestation_downgrade_event`'s
    /// `emit_attestation_event_err_on_poisoned_mutex`). Before this plan,
    /// that test reached `inner` directly via a `pub(crate)` field; `inner`
    /// is private again as of this plan, so this named, documented method
    /// is now the sole substitute (WR-21 point 2 — narrows `inner`'s
    /// visibility to what production code actually needs).
    ///
    /// # `clippy::unwrap_used` justification
    ///
    /// This function is `#[cfg(test)]` but lives in the production `impl
    /// SecurityEventLayer` block above, NOT inside `mod tests` — so it is
    /// not covered by this file's `mod tests`-scoped `#[allow(clippy::
    /// unwrap_used)]` (see the bottom of this file). `cargo clippy
    /// --all-targets` compiles `#[cfg(test)]` items outside `mod tests`
    /// too, so this function carries its OWN scoped allow below, matching
    /// the identical precedent in `attestation_downgrade_event.rs`'s
    /// own test-only poisoning code.
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    pub(crate) fn poison_for_test(&self) {
        let this = self.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _guard = this.inner.lock().unwrap();
            panic!("intentionally poison the mutex (poison_for_test)");
        }));
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
                downgraded_layers: None,
            };
            windows::emit_security_event(&security_event);
        }

        Ok(chain_head)
    }

    /// Advance the HMAC chain by exactly one event and emit its record,
    /// both under ONE lock acquisition (Phase 117 Plan 33 / WR-21 point 1).
    ///
    /// This is the sole cross-module accessor for the tamper-evident chain:
    /// every module outside `telemetry::mod` that needs to advance the chain
    /// (currently `exec_strategy_windows::attestation_downgrade_event`'s
    /// `emit_attestation_event`) MUST go through this method instead of
    /// reaching into `SecurityEventLayerInner`'s fields directly — those
    /// fields are private again as of Plan 25.
    ///
    /// Replaces the former `advance_and_snapshot` (Plan 25), which locked
    /// once, advanced the chain, and returned a `(session_id, chain_head,
    /// enabled)` snapshot — dropping the lock BEFORE the caller built and
    /// emitted its `SecurityEvent`. That left a window between chain
    /// advancement and the Event Log write where a concurrent emitter
    /// (`emit_override_event` or `on_event`, both of which hold the lock
    /// across their own emit) could advance the chain and emit in between,
    /// producing chain-advance order A,B but Event Log write order B,A. Each
    /// record still carries its own correct `chain_head` (not a forgery
    /// risk), but the two sibling emitters no longer shared the same
    /// ordering guarantee — an undocumented divergence in a tamper-evidence
    /// mechanism. `advance_and_emit` closes that window: the `MutexGuard`
    /// stays in scope across the `emit` closure call, so no other emitter
    /// can interleave between chain-advance and Event Log write.
    ///
    /// Locks `inner` exactly once. `build_event_bytes` receives the current
    /// `session_id` (so the caller never needs a separate lock just to read
    /// it before building the bytes to chain) and its return value is
    /// passed to [`advance_chain`]. `emit` then receives
    /// `(&session_id, &chain_head_hex, telemetry_enabled)` and runs to
    /// completion — including any Event Log emit it performs — WHILE THE
    /// LOCK IS STILL HELD, mirroring `emit_override_event`'s existing
    /// single-critical-section shape.
    ///
    /// # Deadlock analysis
    ///
    /// `emit` MUST NOT re-enter any path that takes `self.inner`'s lock
    /// (or any clone's — all clones share the same `Arc<Mutex<...>>`).
    /// `std::sync::Mutex` is NOT reentrant, so a re-entry deadlocks the
    /// supervisor before the child is ever resumed.
    ///
    /// WR-30 (Phase 117-41) corrected this section. It previously claimed
    /// `emit_security_event` "touches no `SecurityEventLayer` state, so it
    /// cannot re-enter this mutex." **That premise is false.**
    /// `emit_security_event` ends in a `tracing::warn!`, which is dispatched
    /// through the subscriber stack to
    /// [`SecurityEventLayer::on_event`] — a `SecurityEventLayer` method that
    /// takes exactly this mutex (`self.inner.lock()`).
    ///
    /// What actually prevents the deadlock is a STRING MISMATCH:
    ///
    /// - `on_event` early-returns unless
    ///   `event.metadata().target().starts_with("nono_security::")` — note
    ///   the trailing double colon.
    /// - `emit_security_event`'s `tracing::warn!` uses the BARE target
    ///   `"nono_security"`, which does not satisfy that prefix test.
    ///
    /// So `on_event` returns before reaching its `lock()` call, and the
    /// re-entry never happens. This is incidental, not designed: aligning
    /// the emit target with this module's own documented `nono_security::*`
    /// convention — the obvious "cleanup" for a future reader, and one the
    /// `telemetry/windows.rs` module docs actively invite — would make
    /// `on_event` match, take the lock, and deadlock.
    ///
    /// `emit_security_event_target_must_not_match_on_event_prefix` pins the
    /// real invariant mechanically so that edit fails the build instead.
    ///
    /// # Returns
    ///
    /// `Ok(R)` — the `emit` closure's own return value — on success.
    /// `Err("mutex poisoned")` if the lock is poisoned (AUD-04 fail-closed —
    /// callers must treat this per their own event's contract). `emit` is
    /// never invoked when the lock is poisoned.
    // This method's only caller is `exec_strategy_windows::
    // attestation_downgrade_event::emit_attestation_event`, a file compiled
    // only into the `nono` binary (not `nono-agentd` — see that module's
    // doc comment) and exercised by that file's own unit tests. Same
    // multi-binary compilation artifact as `emit_override_event` above
    // (`nono-cli` has no `[lib]` target, so `telemetry/mod.rs` is
    // `#[path]`-included wholesale into both binaries): the method IS used
    // in production (`nono` binary) but has zero reachable callers within
    // `nono-agentd`'s independent compilation unit. Not actual dead code.
    #[allow(dead_code)]
    pub(crate) fn advance_and_emit<R>(
        &self,
        build_event_bytes: impl FnOnce(&str) -> Vec<u8>,
        emit: impl FnOnce(&str, &str, bool) -> R,
    ) -> Result<R, &'static str> {
        let mut inner = self.inner.lock().map_err(|_| "mutex poisoned")?;
        let event_bytes = build_event_bytes(&inner.session_id);
        advance_chain(&mut inner.chain, &event_bytes);
        let chain_head = chain_head_hex(&inner.chain.head);
        // The MutexGuard (`inner`) stays alive across this call — `emit`
        // runs, including any Event Log write it performs, WHILE the lock
        // is still held. See "Deadlock analysis" above: safety rests on the
        // emit target being the BARE `"nono_security"`, which fails
        // `on_event`'s `"nono_security::"` prefix test — NOT on `emit`
        // touching no layer state (it does reach `on_event`).
        Ok(emit(&inner.session_id, &chain_head, inner.config.enabled))
    }

    // `emit_attestation_event` (Phase 117 CINT-02 / D-27's structured-telemetry
    // channel for `LayerAttestationDowngraded` events) moved to
    // `exec_strategy_windows::attestation_downgrade_event` in Plan 17
    // (NR3-05 follow-up). It used to live here and be shared, via this
    // file's #[path]-duplication, into BOTH the `nono` and `nono-agentd`
    // binaries — but after Plan 17 deleted the daemon's only call site
    // (`DaemonAttestationDecision::ProceedDowngraded`, now removed; the
    // daemon is a deliberately two-state Proceed/Abort design), it became
    // genuinely dead code within `nono-agentd`'s own compilation unit, a
    // `-D warnings`-fatal `dead_code` lint CLAUDE.md's `#[allow(dead_code)]`
    // ban correctly refuses to paper over. Its one remaining real caller is
    // `exec_strategy_windows::attestation`'s callers
    // (`exec_strategy_windows/launch.rs`'s `apply_startup_attestation_gate`)
    // — a module tree `nono-agentd.rs` never `#[path]`-includes — so moving
    // the method there (as a second `impl SecurityEventLayer` block) makes
    // its compiled-into-one-binary-only reality match its real usage,
    // instead of silencing the mismatch with an attribute. It reaches the
    // chain exclusively through `advance_and_emit` above (Plan 33; formerly
    // `advance_and_snapshot`, Plan 25).
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
            downgraded_layers: None,
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
        let layer =
            SecurityEventLayer::new(TelemetryConfig::default(), "test-chain-seq".to_string());
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

    /// `poison_for_test` (Phase 117 Plan 33 / WR-21 point 2) must leave the
    /// layer's mutex poisoned exactly like the manual `catch_unwind` dance
    /// above — this is the accessor that replaced direct `.inner.lock()`
    /// field access from `exec_strategy_windows::attestation_downgrade_event`.
    /// Exercised here (module-locally) so the method has a real caller in
    /// BOTH `nono` and `nono-agentd`'s independent `#[path]`-copies of this
    /// file's `mod tests` — its cross-module caller in
    /// `attestation_downgrade_event.rs` only compiles into the `nono` binary.
    #[test]
    fn poison_for_test_poisons_mutex_for_emit_override_event() {
        let layer =
            SecurityEventLayer::new(TelemetryConfig::default(), "test-poison-helper".to_string());

        layer.poison_for_test();

        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideRejected,
            "jti",
            "kms_key_id",
            None,
        );
        assert!(
            result.is_err(),
            "poison_for_test must leave the mutex poisoned (AUD-04 fail-closed)"
        );
    }

    /// AUD-01 idempotent ordering: two calls advance chain_sequence from 0 to 2.
    #[test]
    fn emit_override_event_two_calls_advance_by_two() {
        let layer =
            SecurityEventLayer::new(TelemetryConfig::default(), "test-two-calls".to_string());
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
        let layer =
            SecurityEventLayer::new(TelemetryConfig::default(), "test-no-zt-hash".to_string());
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

    // `emit_attestation_event`'s tests moved to
    // `exec_strategy_windows::attestation_downgrade_event::tests` alongside
    // the method itself (Plan 17 NR3-05 follow-up — see the comment on this
    // impl block's now-empty tail above).

    /// Perturbation-proof-carrying regression test (Phase 117 Plan 33 / WR-21
    /// point 1): `advance_and_emit` must hold `inner`'s lock across the WHOLE
    /// build+advance+emit sequence, not release it before `emit` runs.
    ///
    /// Two threads call `advance_and_emit` concurrently. Thread A's `emit`
    /// closure sleeps for 150ms after signalling it has started (simulating
    /// a slow Event Log write). Thread B starts ~30ms later and its
    /// `build_event_bytes` closure — which runs INSIDE the lock, immediately
    /// after acquisition — records whether A's emit had already finished by
    /// the time B got the lock. If the lock is genuinely held across A's
    /// full sequence, B cannot acquire it until A's 150ms sleep completes,
    /// so B always observes `a_emit_finished == true`. This test was proven
    /// to actually catch a regression: see this plan's SUMMARY.md
    /// "Perturbation Proofs" section for the observed failure when
    /// `advance_and_emit` was temporarily changed to drop the lock before
    /// calling `emit` (the pre-fix `advance_and_snapshot` shape).
    #[test]
    fn advance_and_emit_holds_lock_across_full_build_advance_emit_sequence() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Barrier};
        use std::thread;
        use std::time::Duration;

        let layer = SecurityEventLayer::new(
            TelemetryConfig::default(),
            "concurrency-proof-session".to_string(),
        );

        let a_emit_finished = Arc::new(AtomicBool::new(false));
        let b_observed_a_finished_before_its_own_advance = Arc::new(AtomicBool::new(false));
        let start_barrier = Arc::new(Barrier::new(2));

        let layer_a = layer.clone();
        let a_emit_finished_writer = Arc::clone(&a_emit_finished);
        let barrier_a = Arc::clone(&start_barrier);
        let handle_a = thread::spawn(move || {
            barrier_a.wait();
            let _ = layer_a.advance_and_emit(
                |_session_id| b"event-a".to_vec(),
                |_session_id, _chain_head, _enabled| {
                    // Simulate a slow Event Log write, still holding the lock.
                    thread::sleep(Duration::from_millis(150));
                    a_emit_finished_writer.store(true, Ordering::SeqCst);
                },
            );
        });

        let layer_b = layer.clone();
        let a_emit_finished_reader = Arc::clone(&a_emit_finished);
        let observed = Arc::clone(&b_observed_a_finished_before_its_own_advance);
        let barrier_b = Arc::clone(&start_barrier);
        let handle_b = thread::spawn(move || {
            barrier_b.wait();
            // Give thread A a head start into its critical section before B
            // attempts to acquire the same lock.
            thread::sleep(Duration::from_millis(30));
            let _ = layer_b.advance_and_emit(
                |_session_id| {
                    // Runs INSIDE the lock, immediately after acquisition. If
                    // the fix holds, B cannot reach this point until A's
                    // 150ms emit-sleep (and lock release) has completed.
                    if a_emit_finished_reader.load(Ordering::SeqCst) {
                        observed.store(true, Ordering::SeqCst);
                    }
                    b"event-b".to_vec()
                },
                |_session_id, _chain_head, _enabled| {},
            );
        });

        handle_a.join().unwrap();
        handle_b.join().unwrap();

        assert!(
            b_observed_a_finished_before_its_own_advance.load(Ordering::SeqCst),
            "thread B's build_event_bytes (which runs under the lock) must not \
             be reachable until thread A's full build+advance+emit sequence has \
             completed — this is the atomicity advance_and_emit must guarantee \
             (WR-21 point 1)"
        );
    }

    /// WR-30 (Phase 117-41): pin the invariant `advance_and_emit`'s safety
    /// ACTUALLY rests on.
    ///
    /// `advance_and_emit` runs its `emit` closure while holding
    /// `self.inner`'s lock. That closure reaches
    /// `telemetry::windows::emit_security_event`, which ends in a
    /// `tracing::warn!` — dispatched through the subscriber to
    /// `SecurityEventLayer::on_event`, which takes THAT SAME mutex.
    /// `std::sync::Mutex` is not reentrant, so the only thing preventing a
    /// supervisor deadlock is that `on_event` early-returns on
    /// `!target.starts_with("nono_security::")` while the emit uses the bare
    /// `"nono_security"`.
    ///
    /// That is incidental, and actively fragile: `telemetry/windows.rs`'s own
    /// module doc and `emit_security_event`'s doc comment both describe the
    /// target as `"nono_security::*"`, so "aligning" the literal with the
    /// documented convention looks like a tidy-up and is in fact a deadlock.
    /// This test makes that edit fail the build.
    ///
    /// Source-text based on purpose: the deadlock cannot be exercised at
    /// runtime without hanging the test process.
    #[test]
    fn emit_security_event_target_must_not_match_on_event_prefix() {
        const WINDOWS_SRC: &str = include_str!("windows.rs");

        // Every `target: "..."` literal in the emitter's source.
        let mut targets = Vec::new();
        let mut rest = WINDOWS_SRC;
        while let Some(pos) = rest.find("target: \"") {
            let after = &rest[pos + "target: \"".len()..];
            let Some(end) = after.find('"') else { break };
            targets.push(&after[..end]);
            rest = &after[end + 1..];
        }

        assert!(
            !targets.is_empty(),
            "no `target: \"...\"` literal found in telemetry/windows.rs — this test has gone \
             blind by construction; if the emit moved, move this gate with it"
        );

        for target in &targets {
            assert!(
                !target.starts_with("nono_security::"),
                "DEADLOCK: telemetry/windows.rs emits on target {target:?}, which satisfies \
                 SecurityEventLayer::on_event's `nono_security::` prefix test. on_event takes \
                 the same mutex advance_and_emit holds across its emit closure, and \
                 std::sync::Mutex is not reentrant — this hangs the supervisor before the \
                 child is resumed. The target must stay the BARE \"nono_security\" (WR-30)."
            );
        }

        assert!(
            targets.contains(&"nono_security"),
            "expected the bare \"nono_security\" emit target to still be present; found {targets:?}"
        );
    }

    /// WR-30 companion: the prefix literal `on_event` tests against must keep
    /// its trailing `::`. Dropping it would make the bare `"nono_security"`
    /// emit target match, producing the same deadlock from the other side —
    /// the sibling half of the invariant above.
    #[test]
    fn on_event_prefix_test_must_keep_its_trailing_colons() {
        const MOD_SRC: &str = include_str!("mod.rs");

        // Anchor on the FULL statement form, which occurs only in `on_event`'s
        // production body. A bare `contains("starts_with(\"nono_security::\")")`
        // also matches this test's own doc comment and assertion message, so it
        // passes even after `on_event` is broken — caught by perturbation while
        // writing this test. (Truncating at the first `#[cfg(test)]` does not
        // work either: the first occurrence is inside a doc comment ~300 lines
        // above `on_event`.)
        const PRODUCTION_GUARD: &str =
            "if !event.metadata().target().starts_with(\"nono_security::\") {";

        assert!(
            MOD_SRC.contains(PRODUCTION_GUARD),
            "SecurityEventLayer::on_event must gate on the `nono_security::` prefix INCLUDING \
             the trailing double colon; without it the bare \"nono_security\" emit target \
             matches and advance_and_emit deadlocks (WR-30)"
        );
    }
}
