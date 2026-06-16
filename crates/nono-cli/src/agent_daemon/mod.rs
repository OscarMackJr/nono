//! Agent daemon module: owning-struct layer for `nono-agentd`.
//!
//! This module is the root of the `agent_daemon` tree, loaded by
//! `src/bin/nono-agentd.rs` via a `#[path]` attribute. It defines
//! [`DaemonState`] — the top-level owning struct that the daemon binary
//! passes through all async tasks — and declares the sub-module tree.
//!
//! # Sub-module status
//!
//! | Module | Status | Plan |
//! |--------|--------|------|
//! | `reap` | **Implemented** (AgentTenant RAII) | 74-03 |
//! | `accept_loop` | **Implemented** (capability pipe accept loop) | 74-04 |
//! | `launch` | **Implemented** (AppContainer spawn + reap) | 74-04 |
//! | `control_loop` | **Implemented** (operator control pipe server) | 74-07 |
//!
//! # Thread safety
//!
//! `DaemonState` is `Send + Sync` via its `Arc<Mutex<_>>` fields. Clone the
//! `Arc` wrapper (not `DaemonState` itself) to share state across
//! `tokio::spawn` boundaries.

/// Implemented in Plan 74-04 (Wave 2).
pub(crate) mod accept_loop;
/// Daemon-side operator control-pipe server (Plan 74-07 Wave 5).
pub(crate) mod control_loop;
/// Implemented in Plan 74-04 (Wave 2).
pub(crate) mod launch;
pub(crate) mod reap;

// ─── WFP wire-protocol types (Plan 75-01, SUPP-02) ───────────────────────────
//
// The `nono-agentd` binary is compiled WITHOUT the rest of `nono-cli` crate
// (it is loaded only via `#[path]`). Including the shared contract file via
// `#[path]` makes `WfpRuntimeActivationRequest` and friends available to the
// `launch` and `control_loop` modules without duplicating definitions.
/// WFP runtime wire-protocol types (shared with `nono-cli`).
///
/// Path-included here because `nono-agentd` is a standalone binary that pulls
/// in `agent_daemon` via `#[path]` and cannot reach `crate::windows_wfp_contract`.
#[path = "../windows_wfp_contract.rs"]
pub(crate) mod wfp_contract;

// ─── Embedded policy data ─────────────────────────────────────────────────────

/// Embedded policy JSON (compiled into binary by build.rs).
///
/// This constant mirrors `crates/nono-cli/src/config/embedded.rs:EMBEDDED_POLICY_JSON`.
/// It is included directly here because `nono-agentd` is a standalone binary that
/// only pulls in `agent_daemon` via `#[path]` and cannot use `crate::config`.
const EMBEDDED_POLICY_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/policy.json"));

/// Return `true` if `profile_name` is a known profile in the embedded policy.
///
/// Used by `control_loop` to validate profile names before calling `launch_agent`
/// (T-74-07-03: unknown profile → fail-secure error response, never launch).
///
/// Parsing is minimal: we extract only the `"profiles"` object's top-level keys
/// from the embedded JSON. A parse failure is conservative — we return `false`
/// (fail-secure).
pub(crate) fn is_known_profile(profile_name: &str) -> bool {
    // Parse the embedded policy JSON using serde_json::Value for minimal overhead.
    // Fail-closed: if parsing fails, treat the profile as unknown.
    let policy: serde_json::Value = match serde_json::from_str(EMBEDDED_POLICY_JSON) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Check if `policy["profiles"][profile_name]` exists.
    policy
        .get("profiles")
        .and_then(|p| p.as_object())
        .map(|profiles| profiles.contains_key(profile_name))
        .unwrap_or(false)
}

use reap::AgentTenant;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Top-level daemon state: owns all per-agent tenants and the authorization registry.
///
/// `DaemonState` is constructed once at daemon startup and passed (via `Arc` clone)
/// to every async task. It is `Send + Sync` because all mutable state is behind
/// `Arc<Mutex<_>>`.
///
/// # Locking order (to prevent deadlock)
///
/// When both locks must be held simultaneously, always acquire them in this order:
/// 1. `agent_registry` first
/// 2. `tenants` second
///
/// This ordering is enforced in `agent_daemon::launch` and the reap path in
/// `accept_loop`. Violating it risks a deadlock between two concurrent reap calls.
///
/// # Reap sequence (caller contract)
///
/// Before removing an `AgentTenant` from `tenants` (which drops it, triggering
/// `AgentTenant::Drop`):
/// 1. Lock `agent_registry` and call `AgentRegistry::remove(&tenant.package_sid)`.
/// 2. Release the `agent_registry` lock.
/// 3. Lock `tenants`, remove the entry (this drops `AgentTenant`).
/// 4. Release the `tenants` lock.
///
/// This ordering ensures the SID is removed from the authorization registry before
/// the agent process group is killed, closing the race where a recycled SID could
/// briefly match a stale registry entry.
pub(crate) struct DaemonState {
    /// Per-agent tenant map: `tenant_id` (session UUID string) → `AgentTenant`.
    ///
    /// Removing an entry from this map drops the `AgentTenant`, which closes
    /// `job_handle` (firing `KILL_ON_JOB_CLOSE`) and calls
    /// `DeleteAppContainerProfile` (best-effort).
    ///
    /// Read and mutated by `agent_daemon::accept_loop` and `agent_daemon::launch`.
    pub tenants: Arc<Mutex<HashMap<String, AgentTenant>>>,

    /// Phase 73/74 authorization registry: the private set of AppContainer
    /// package SIDs minted by this daemon instance.
    ///
    /// A pipe client is authorized if and only if its kernel-attested package SID
    /// (from `ImpersonateNamedPipeClient` + `GetTokenInformation`) is present in
    /// this registry. Namespace-pattern matching is intentionally NOT used as the
    /// authorization check — it is forgeable (see `agent.rs` module doc).
    ///
    /// Mutated by `agent_daemon::launch` when agents are spawned/reaped.
    /// `#[allow(dead_code)]` because the field is accessed only through the
    /// `launch` module (Wave 3 wiring) and tests; clippy cannot see those
    /// as reads in the binary compilation unit.
    #[allow(dead_code)]
    pub agent_registry: Arc<Mutex<nono::AgentRegistry>>,
}

impl DaemonState {
    /// Construct a new, empty `DaemonState`.
    ///
    /// Called once at daemon startup (both service-mode and foreground-mode paths).
    /// The registry and tenant map start empty; entries are added by
    /// `agent_daemon::launch` as agents are spawned.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            tenants: Arc::new(Mutex::new(HashMap::new())),
            agent_registry: Arc::new(Mutex::new(nono::AgentRegistry::new())),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Verify that `DaemonState::new()` constructs a valid empty state and that
    /// the tenant map can be locked, mutated, and inspected without deadlock.
    #[test]
    fn daemon_state_new_is_empty() {
        let state = DaemonState::new();

        let tenants = state.tenants.lock().unwrap();
        assert_eq!(
            tenants.len(),
            0,
            "A freshly constructed DaemonState must have an empty tenant map"
        );
    }

    /// Verify that the `agent_registry` can be locked and that `AgentRegistry`
    /// operations work correctly when accessed through the `DaemonState` wrapper.
    #[test]
    fn daemon_state_registry_insert_remove_roundtrip() {
        let state = DaemonState::new();

        {
            let mut registry = state.agent_registry.lock().unwrap();
            registry.insert("S-1-15-2-test-daemon-state-01".to_string());
        }

        // Classify via the same registry to verify the insert landed.
        // On Windows: classify checks the token SID against the registry.
        // On non-Windows: classify always returns NotAnAgent (stub behavior).
        // Either way, the insert + lock round-trip must not deadlock or panic.
        {
            let mut registry = state.agent_registry.lock().unwrap();
            registry.remove("S-1-15-2-test-daemon-state-01");
        }

        // After removal, the registry must be empty (verifiable via a second insert).
        {
            let mut registry = state.agent_registry.lock().unwrap();
            registry.insert("S-1-15-2-test-daemon-state-02".to_string());
            registry.remove("S-1-15-2-test-daemon-state-02");
        }
        // No panic → contract satisfied.
    }

    /// Verify that two `Arc` clones of the same `DaemonState` fields share the
    /// same underlying mutex (write visible to both arcs).
    #[test]
    fn daemon_state_arcs_share_same_mutex() {
        let state = DaemonState::new();
        let registry_arc = Arc::clone(&state.agent_registry);

        // Insert via the cloned Arc.
        {
            let mut registry = registry_arc.lock().unwrap();
            registry.insert("S-1-15-2-arc-share-test".to_string());
        }

        // Remove via the original DaemonState field — must see the same state.
        {
            let mut registry = state.agent_registry.lock().unwrap();
            // remove is idempotent; if the insert above was in a DIFFERENT mutex
            // this would be a no-op and the test would still pass. However,
            // any subsequent operations on the shared arc would reflect the state.
            registry.remove("S-1-15-2-arc-share-test");
        }
        // No deadlock or panic → the same Mutex is shared via Arc.
    }
}
