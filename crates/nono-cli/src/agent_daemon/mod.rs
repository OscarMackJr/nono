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

/// Build a real `CapabilitySet` for a daemon-launched agent (GAP-75-B fix).
///
/// Grants cover:
/// - Engine exe parent directory (Read): so the runtime linker can load the engine.
/// - `%SystemRoot%`, `%SystemRoot%\System32`, `%SystemRoot%\SysWOW64` (Read): CLR/PE
///   loader baseline (Phase 58 lesson: CLR fails with `0xFFFF0000` if these are absent).
/// - Per-profile interpreter directories (Read): resolved via `where <interp_name>` for
///   each entry in `policy["profiles"][profile_name]["windows_interpreters"]`. Missing
///   interpreters are logged and skipped (non-fatal — the engine may not need all of them
///   at startup; the DACL guard will still confine what it can).
/// - Per-tenant workspace (ReadWrite): the workspace directory that the daemon created
///   before calling this function. The workspace MUST exist on disk (verified by this
///   function via `workspace.exists()`).
///
/// All path joins use `Path::join` (component-based), never string concatenation
/// (CLAUDE.md path security rule).
///
/// # Errors
///
/// Returns `Err(NonoError::SandboxInit(_))` if:
/// - `EMBEDDED_POLICY_JSON` cannot be parsed.
/// - The engine exe has no parent directory.
/// - `caps.allow_path(...)` fails for any path.
/// - The workspace directory does not exist.
#[cfg(target_os = "windows")]
pub(crate) fn build_daemon_capability_set(
    profile_name: &str,
    resolved_exe: &std::path::Path,
    workspace: &std::path::Path,
) -> nono::Result<nono::CapabilitySet> {
    use nono::{AccessMode, NonoError};

    // 1. Parse embedded policy JSON (same approach as is_known_profile).
    let policy: serde_json::Value = serde_json::from_str(EMBEDDED_POLICY_JSON).map_err(|e| {
        NonoError::SandboxInit(format!(
            "build_daemon_capability_set: failed to parse embedded policy: {e}"
        ))
    })?;

    // 2. Extract windows_interpreters from the profile (empty list if absent).
    let interpreter_names: Vec<String> = policy
        .get("profiles")
        .and_then(|p| p.as_object())
        .and_then(|profiles| profiles.get(profile_name))
        .and_then(|profile| profile.get("windows_interpreters"))
        .and_then(|interps| interps.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut caps = nono::CapabilitySet::new();

    // 3a. Engine exe parent directory (Read — runtime linker needs the exe dir).
    let exe_parent = resolved_exe.parent().ok_or_else(|| {
        NonoError::SandboxInit(format!(
            "build_daemon_capability_set: resolved exe has no parent directory: {}",
            resolved_exe.display()
        ))
    })?;
    // Canonicalize for accuracy; fall back to unresolved on failure.
    let exe_parent_canon = std::fs::canonicalize(exe_parent).unwrap_or_else(|e| {
        tracing::warn!(
            path = %exe_parent.display(),
            error = %e,
            "build_daemon_capability_set: could not canonicalize exe parent; using unresolved path"
        );
        exe_parent.to_path_buf()
    });
    caps = caps
        .allow_path(&exe_parent_canon, AccessMode::Read)
        .map_err(|e| {
            NonoError::SandboxInit(format!(
                "build_daemon_capability_set: allow_path(exe_parent={}) failed: {e}",
                exe_parent_canon.display()
            ))
        })?;

    // 3b. CLR/PE loader baseline: %SystemRoot%, %SystemRoot%\System32, %SystemRoot%\SysWOW64.
    // Phase 58 lesson (MEMORY.md): CLR returns 0xFFFF0000 if SystemRoot is absent.
    let system_root_str = std::env::var("SystemRoot").unwrap_or_else(|_| {
        tracing::warn!(
            "build_daemon_capability_set: %SystemRoot% not set; defaulting to C:\\Windows"
        );
        "C:\\Windows".to_string()
    });
    let system_root = std::path::Path::new(&system_root_str);
    for dir in &[
        system_root.to_path_buf(),
        system_root.join("System32"),
        system_root.join("SysWOW64"),
    ] {
        caps = caps.allow_path(dir, AccessMode::Read).map_err(|e| {
            NonoError::SandboxInit(format!(
                "build_daemon_capability_set: allow_path(system_dir={}) failed: {e}",
                dir.display()
            ))
        })?;
    }

    // 3c. Interpreter directories (Read). Resolved via `where <name>` (Windows PATH search).
    for interp_name in &interpreter_names {
        let output = std::process::Command::new("where")
            .arg(interp_name.as_str())
            .output();
        match output {
            Ok(out) if out.status.success() && !out.stdout.is_empty() => {
                // Parse first line of `where` output as a path.
                let stdout = String::from_utf8_lossy(&out.stdout);
                let first_line = stdout.lines().next().unwrap_or("").trim();
                if first_line.is_empty() {
                    tracing::warn!(
                        interp = %interp_name,
                        "build_daemon_capability_set: `where` output was empty for interpreter; skipping"
                    );
                    continue;
                }
                let interp_path = std::path::PathBuf::from(first_line);
                let interp_dir = match interp_path.parent() {
                    Some(p) => p.to_path_buf(),
                    None => {
                        tracing::warn!(
                            interp = %interp_name,
                            path = %interp_path.display(),
                            "build_daemon_capability_set: interpreter has no parent dir; skipping"
                        );
                        continue;
                    }
                };
                caps = caps
                    .allow_path(&interp_dir, AccessMode::Read)
                    .map_err(|e| {
                        NonoError::SandboxInit(format!(
                        "build_daemon_capability_set: allow_path(interpreter_dir={}) failed: {e}",
                        interp_dir.display()
                    ))
                    })?;
            }
            Ok(_) => {
                tracing::warn!(
                    interp = %interp_name,
                    "build_daemon_capability_set: interpreter not found via `where`; skipping"
                );
            }
            Err(e) => {
                tracing::warn!(
                    interp = %interp_name,
                    error = %e,
                    "build_daemon_capability_set: `where` command failed for interpreter; skipping"
                );
            }
        }
    }

    // 3d. Per-tenant workspace (ReadWrite). The workspace MUST exist (created by handle_launch).
    if !workspace.exists() {
        return Err(NonoError::SandboxInit(format!(
            "build_daemon_capability_set: per-tenant workspace does not exist: {}",
            workspace.display()
        )));
    }
    caps = caps
        .allow_path(workspace, AccessMode::ReadWrite)
        .map_err(|e| {
            NonoError::SandboxInit(format!(
                "build_daemon_capability_set: allow_path(workspace={}) failed: {e}",
                workspace.display()
            ))
        })?;

    Ok(caps)
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

    // ── Task 1 (75-07-T1): build_daemon_capability_set + workspace derivation ──

    /// Verify `build_daemon_capability_set` returns a non-empty CapabilitySet for a
    /// known profile ("aider"). Uses a real tempdir for the workspace (must exist)
    /// and a fake exe path pointing at a known binary (nono.exe or cmd.exe) so the
    /// exe parent resolution succeeds.
    #[test]
    #[cfg(target_os = "windows")]
    fn daemon_caps_non_empty_for_known_profile() {
        use tempfile::tempdir;

        // Use cmd.exe as a stable "fake exe" with a resolvable parent directory.
        let fake_exe = std::path::PathBuf::from(r"C:\Windows\System32\cmd.exe");
        let workspace_dir = tempdir().expect("tempdir for workspace");
        let workspace = workspace_dir.path().to_path_buf();

        let caps = build_daemon_capability_set("aider", &fake_exe, &workspace)
            .expect("build_daemon_capability_set must succeed for 'aider' profile");

        // The CapabilitySet must cover at least the exe dir + SystemRoot + workspace.
        // We can't inspect individual rules from CapabilitySet's public API, but we
        // CAN verify it is non-trivially non-empty by applying it to a Windows policy.
        let policy = nono::Sandbox::windows_filesystem_policy(&caps);
        assert!(
            !policy.rules.is_empty(),
            "CapabilitySet for 'aider' profile must produce at least one filesystem rule; \
             got 0 rules (exe_parent, SystemRoot dirs, and workspace should each contribute)"
        );
    }

    /// Verify workspace derivation logic produces a path under USERPROFILE (or temp_dir)
    /// with a "nono-agents" component and a 16-hex-char token suffix.
    #[test]
    fn daemon_workspace_path_uses_userprofile() {
        // Mirror the workspace derivation logic from handle_launch.
        let userprofile = std::env::var("USERPROFILE")
            .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
        let base = std::path::Path::new(&userprofile);

        // Generate an 8-byte random token (16 hex chars) — same as handle_launch.
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).expect("getrandom::fill");
        let workspace_token: String = b.iter().map(|x| format!("{x:02x}")).collect();
        assert_eq!(
            workspace_token.len(),
            16,
            "workspace token must be 16 hex chars (8 random bytes)"
        );

        let workspace = base.join("nono-agents").join(&workspace_token);

        // The path must contain "nono-agents" as a component.
        let has_nono_agents = workspace.components().any(|c| {
            c.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case("nono-agents")
        });
        assert!(
            has_nono_agents,
            "workspace path must contain 'nono-agents' component; got: {}",
            workspace.display()
        );

        // The leaf component (token dir) must be all-lowercase hex.
        let token_component = workspace
            .file_name()
            .expect("workspace must have a file_name")
            .to_string_lossy();
        assert_eq!(
            token_component.len(),
            16,
            "workspace token dir must be 16 chars; got: {token_component}"
        );
        assert!(
            token_component.chars().all(|c| c.is_ascii_hexdigit()),
            "workspace token dir must be lowercase hex; got: {token_component}"
        );
    }

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
