//! Daemon-side telemetry init helper (DRAIN-04 D-02).
//!
//! Provides [`init_daemon_telemetry`] — a minimal, standalone tracing-subscriber
//! registration that wires [`crate::telemetry::SecurityEventLayer`] into the
//! daemon process.
//!
//! # Design (D-02)
//!
//! This helper is intentionally **separate** from `cli_bootstrap::init_tracing`.
//! The daemon binary (`nono-agentd`) has no `Cli` type, no `EnvFilter` verbosity
//! flags, and no file-log arm.  This function mirrors only the registry
//! composition from `init_registry` (security_layer + optional ETW arm on Windows),
//! keeping the daemon binary's tracing stack minimal and independent.
//!
//! # Double-init guard (Pitfall 1)
//!
//! `run_service_mode()` may fall through to `run_foreground_mode()` when the SCM
//! is unavailable.  Both code paths call this helper.  A [`std::sync::OnceLock`]
//! guard ensures the global subscriber is set at most once per process — the second
//! call is a no-op (using `try_init()` instead of `init()`).

use crate::telemetry::SecurityEventLayer;
use nono::TelemetryConfig;
use tracing_subscriber::prelude::*;

/// Initialize the daemon's tracing subscriber with a [`SecurityEventLayer`] (D-02).
///
/// Must be called once at daemon startup in **both** `run_service` and
/// `run_foreground_mode`, after `resolve_machine_egress_policy` returns the
/// `telemetry_config`.  The [`OnceLock`] guard makes the second call a no-op so
/// the foreground-fallback path does not panic on a "subscriber already set" error.
///
/// # Arguments
///
/// - `config` — telemetry configuration from the SOLE HKLM read (D-03); controls
///   `enabled` opt-out and `min_severity` threshold.
/// - `session_id` — opaque per-session identifier (e.g. derived from process ID)
///   for correlating events in the HMAC chain and OS log.
///
/// # Platform
///
/// On Windows, attempts to add a `tracing-etw` "nono" provider layer; continues
/// without ETW on build failure (D-03 non-fatal).  On non-Windows targets the
/// function is a no-op so the source file compiles cross-platform for clippy.
pub(crate) fn init_daemon_telemetry(config: TelemetryConfig, session_id: String) {
    // OnceLock guard: only the first call reaches the subscriber registration.
    // Subsequent calls (e.g. via foreground-fallback path) are silently dropped.
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    let already_init = INIT.set(()).is_err();
    if already_init {
        return;
    }

    let security_layer = SecurityEventLayer::new(config, session_id);

    #[cfg(not(target_os = "windows"))]
    {
        // Non-Windows: registry + security_layer only (no ETW).
        let _ = tracing_subscriber::registry()
            .with(security_layer)
            .try_init();
    }

    #[cfg(target_os = "windows")]
    {
        let base = tracing_subscriber::registry().with(security_layer);

        // Attempt to add the tracing-etw "nono" provider (D-03 non-fatal).
        // If ETW layer construction fails, continue without it — never abort.
        match tracing_etw::LayerBuilder::new("nono").build() {
            Ok(etw) => {
                let _ = base.with(etw).try_init();
            }
            Err(e) => {
                eprintln!(
                    "nono-agentd: telemetry: ETW layer init failed ({e}); \
                     continuing without ETW (D-03 non-fatal)"
                );
                let _ = base.try_init();
            }
        }
    }
}

// ── D-01 integration-style tests ─────────────────────────────────────────────
//
// These tests prove that an in-process `nono_security::network_deny` tracing
// event, emitted with the exact field shape from `nono_proxy::audit`, actually
// reaches `SecurityEventLayer::on_event` and advances the HMAC chain
// (sequence goes 0 → 1).  They are inline `#[cfg(test)]` here so they can
// reference `crate::telemetry::SecurityEventLayer` and its `pub(crate)`
// `chain_sequence()` accessor — which would not be reachable from an external
// integration test that cannot see these #[path]-included internals.
//
// The tests drive the layer via `tracing::subscriber::with_default` so the
// real tracing dispatch path is exercised (not a direct on_event call), matching
// the D-01 requirement that the test "drives a synthesized
// `nono_security::network_deny` event … and asserts the event is actually
// processed (HMAC chain advances)."

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nono::TelemetrySeverity;
    use std::sync::{Arc, Mutex};
    use tracing::Subscriber;
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::Layer;

    // ── Test-bridge layer (D-01 / A2 workaround) ─────────────────────────────
    //
    // `Arc<SecurityEventLayer>` does not implement `Layer<S>` in
    // tracing-subscriber 0.3.23 (the impl was not in `impl_layer_for_ptr!`
    // for `Arc` in that release).  We bridge the gap with a thin wrapper that:
    //  1. Owns the `SecurityEventLayer` and delegates `on_event` to it.
    //  2. Exposes a shared `Arc<Mutex<u64>>` so callers can read the chain
    //     sequence after the `with_default` closure returns (when the subscriber
    //     has already consumed the layer).
    struct SpyLayer {
        inner: SecurityEventLayer,
        /// Mirror counter updated by `on_event` from `inner.chain_sequence()`.
        /// Read by the test AFTER `with_default` returns.
        sequence_mirror: Arc<Mutex<u64>>,
    }

    impl SpyLayer {
        fn new(config: TelemetryConfig, session_id: &str) -> (Self, Arc<Mutex<u64>>) {
            let sequence_mirror = Arc::new(Mutex::new(0u64));
            let this = Self {
                inner: SecurityEventLayer::new(config, session_id.into()),
                sequence_mirror: Arc::clone(&sequence_mirror),
            };
            (this, sequence_mirror)
        }
    }

    impl<S: Subscriber> Layer<S> for SpyLayer {
        fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
            self.inner.on_event(event, ctx);
            // Mirror the real sequence into the shared counter.
            if let Ok(mut guard) = self.sequence_mirror.lock() {
                *guard = self.inner.chain_sequence();
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────

    /// D-01 (chain-advance test): a `nono_security::network_deny` event emitted
    /// through the SecurityEventLayer advances the HMAC chain from 0 to 1.
    ///
    /// Uses `tracing::subscriber::with_default` to exercise the real dispatch
    /// path through `SpyLayer → SecurityEventLayer::on_event → advance_chain`.
    #[test]
    fn d01_network_deny_advances_chain_sequence_to_one() {
        let config = TelemetryConfig {
            enabled: true,
            channel: "Application".to_string(),
            min_severity: TelemetrySeverity::Warning,
        };
        let (spy, sequence_mirror) = SpyLayer::new(config, "test-session");

        // Genesis: mirror starts at 0.
        assert_eq!(
            *sequence_mirror.lock().unwrap(),
            0,
            "genesis chain sequence must be 0"
        );

        let subscriber = tracing_subscriber::registry().with(spy);
        tracing::subscriber::with_default(subscriber, || {
            // Emit the exact shape from nono_proxy::audit::log_network_deny
            // (crates/nono-proxy/src/audit.rs:203-209).
            tracing::warn!(
                target: "nono_security::network_deny",
                host = "blocked.example.com",
                port = 443u16,
                agent_pid = std::process::id(),
                "network deny"
            );
        });

        // D-01 assertion: advance_chain was called exactly once (sequence 0 → 1).
        assert_eq!(
            *sequence_mirror.lock().unwrap(),
            1,
            "chain sequence must advance to 1 after one nono_security::network_deny event; \
             if it stays 0, on_event was not reached (registration or target-matching failure)"
        );
    }

    /// Opt-out test (T-90-03): a layer built with `enabled=false` TelemetryConfig
    /// does NOT advance the chain when the same event is emitted (sequence stays 0).
    #[test]
    fn opt_out_disabled_layer_does_not_advance_chain() {
        let config = TelemetryConfig {
            enabled: false,
            channel: "Application".to_string(),
            min_severity: TelemetrySeverity::Warning,
        };
        let (spy, sequence_mirror) = SpyLayer::new(config, "test-opt-out");

        assert_eq!(
            *sequence_mirror.lock().unwrap(),
            0,
            "genesis sequence must be 0"
        );

        let subscriber = tracing_subscriber::registry().with(spy);
        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!(
                target: "nono_security::network_deny",
                host = "blocked.example.com",
                port = 443u16,
                agent_pid = std::process::id(),
                "network deny"
            );
        });

        // Opt-out: the chain must NOT advance when config.enabled == false.
        assert_eq!(
            *sequence_mirror.lock().unwrap(),
            0,
            "opt-out: chain sequence must stay 0 when telemetry is disabled (T-90-03); \
             if it advanced, the config.enabled guard in on_event is broken"
        );
    }
}
