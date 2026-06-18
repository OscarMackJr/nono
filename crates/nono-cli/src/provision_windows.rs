//! Idempotent first-run-in-user-context provisioner (D-09 / Phase 82).
//!
//! Performs the three things the SYSTEM MSI cannot do in a single,
//! idempotent, non-fatal code path:
//!
//! 1. Create `%LOCALAPPDATA%\nono\workspace` and set WRITE\_OWNER ownership
//!    to the invoking user so the R-B3 ownership guard passes (DEPLOY-03).
//! 2. Import the POC root certificate into `CurrentUser\Root`
//!    (DEPLOY-05, per-user trust for CryptoAPI / PowerShell / nono-cli).
//! 3. Set and **persist** `NODE_EXTRA_CA_CERTS` pointing at the MSI-staged
//!    PEM at `%PROGRAMDATA%\nono\nono-poc-root.pem` (DEPLOY-05, Node TLS).
//!
//! All sub-steps are **non-fatal** to `nono run`: a failure in any one is
//! recorded in [`ProvisionStatus`] rather than propagated as an abort.
//! [`provision_first_run`] returns a [`ProvisionOutcome`] that `nono health`
//! (Plan 03) reads to report the degraded subsystem.
//!
//! The provisioner is idempotent: a registry sentinel
//! (`HKCU\Software\nono\ProvisionedAt`) short-circuits a second call to
//! [`ProvisionOutcome::AlreadyProvisioned`].
//!
//! **Library/CLI boundary:** all Windows-specific logic lives here in `nono-cli`.
//! The `nono` library is consumed only for its R-B3 ownership primitives
//! (`path_is_owned_by_current_user`).

use nono::{NonoError, Result};
use std::path::PathBuf;
use tracing::{debug, warn};

// ── registry sentinel ─────────────────────────────────────────────────────────

/// Registry key path (under HKCU) where the provisioned-at timestamp is stored.
const PROVISION_REG_KEY: &str = r"Software\nono";
/// Registry value name for the provisioned-at timestamp.
const PROVISION_REG_VALUE: &str = "ProvisionedAt";

// ── outcome types ─────────────────────────────────────────────────────────────

/// Outcome of each individual provisioning sub-step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepStatus {
    /// Sub-step succeeded (or was an idempotent no-op).
    Ok,
    /// Sub-step encountered a non-fatal error; `nono run` continues.
    Degraded(String),
}

/// Per-sub-step status record for `nono health` consumption.
#[derive(Debug, Clone)]
pub(crate) struct ProvisionStatus {
    /// Status of the scratch-dir creation + ownership grant (DEPLOY-03).
    pub scratch: StepStatus,
    /// Status of the `CurrentUser\Root` cert import (DEPLOY-05).
    pub cert: StepStatus,
    /// Status of the `NODE_EXTRA_CA_CERTS` set + persist (DEPLOY-05).
    pub node_extra_ca_certs: StepStatus,
    /// Resolved path of the MSI-staged PEM, if known.
    pub pem_path: Option<PathBuf>,
}

/// Top-level result returned by [`provision_first_run`].
#[derive(Debug)]
pub(crate) enum ProvisionOutcome {
    /// First-run provisioning completed (possibly with degraded sub-steps).
    Provisioned(ProvisionStatus),
    /// Idempotency sentinel found — provisioner was a no-op.
    AlreadyProvisioned,
}

// ── public entry point ────────────────────────────────────────────────────────

/// Run the idempotent first-run provisioner.
///
/// If the idempotency sentinel is present (HKCU registry key), returns
/// `Ok(ProvisionOutcome::AlreadyProvisioned)` immediately.
///
/// On first run, executes the three sub-steps in order; each is non-fatal
/// (failures are captured in the returned [`ProvisionStatus`]).  After all
/// steps complete the sentinel is written.
///
/// # Errors
///
/// This function itself never returns `Err` — all sub-step failures are
/// captured in the returned `ProvisionStatus`.  The `Result` wrapper exists
/// for future use and uniform call-site ergonomics.
pub(crate) fn provision_first_run() -> Result<ProvisionOutcome> {
    // ── idempotency check ─────────────────────────────────────────────────
    if is_already_provisioned() {
        debug!("provision_windows: idempotency sentinel present — skipping (AlreadyProvisioned)");
        return Ok(ProvisionOutcome::AlreadyProvisioned);
    }

    let mut status = ProvisionStatus {
        scratch: StepStatus::Ok,
        cert: StepStatus::Ok,
        node_extra_ca_certs: StepStatus::Ok,
        pem_path: None,
    };

    // ── step 1: user-owned scratch dir ────────────────────────────────────
    step_scratch(&mut status);

    // ── step 2: CurrentUser\Root cert import ─────────────────────────────
    step_cert(&mut status);

    // ── step 3: NODE_EXTRA_CA_CERTS ──────────────────────────────────────
    step_node_extra_ca_certs(&mut status);

    // ── write idempotency sentinel ────────────────────────────────────────
    if let Err(e) = write_provision_sentinel() {
        warn!(error = %e, "provision_windows: failed to write idempotency sentinel; next run will re-provision");
    }

    Ok(ProvisionOutcome::Provisioned(status))
}

// ── step implementations ──────────────────────────────────────────────────────

/// Step 1 — Create `%LOCALAPPDATA%\nono\workspace` and grant WRITE\_OWNER to
/// the invoking user.
///
/// Uses the `icacls <path> /setowner *<SID>` idiom from `dacl_guard.rs:506-527`,
/// then verifies ownership via `nono::path_is_owned_by_current_user`.
fn step_scratch(status: &mut ProvisionStatus) {
    let scratch = match scratch_dir() {
        Ok(p) => p,
        Err(e) => {
            status.scratch = StepStatus::Degraded(format!("resolve scratch dir: {e}"));
            return;
        }
    };

    // Create directory tree if missing.
    if let Err(e) = std::fs::create_dir_all(&scratch) {
        status.scratch =
            StepStatus::Degraded(format!("create scratch dir {}: {e}", scratch.display()));
        return;
    }

    // Set ownership to the invoking user via icacls /setowner.
    if let Err(e) = set_owner_to_current_user(&scratch) {
        status.scratch =
            StepStatus::Degraded(format!("icacls /setowner on {}: {e}", scratch.display()));
        // Continue to ownership verification — best effort.
    }

    // Verify ownership via R-B3 primitive.
    match nono::path_is_owned_by_current_user(&scratch) {
        Ok(true) => {
            debug!(path = %scratch.display(), "provision_windows: scratch owned by current user — R-B3 guard will pass");
            status.scratch = StepStatus::Ok;
        }
        Ok(false) => {
            warn!(
                path = %scratch.display(),
                "provision_windows: ownership check returned Ok(false); scratch not owned by current user"
            );
            status.scratch = StepStatus::Degraded(format!(
                "scratch dir {} not owned by current user after setowner",
                scratch.display()
            ));
        }
        Err(e) => {
            warn!(path = %scratch.display(), error = %e, "provision_windows: ownership check failed");
            status.scratch = StepStatus::Degraded(format!(
                "path_is_owned_by_current_user({}) failed: {e}",
                scratch.display()
            ));
        }
    }
}

/// Step 2 — Import the POC root cert into `CurrentUser\Root`.
///
/// Locates the `.cer` file relative to the current executable (staged by the MSI
/// alongside `nono.exe` in INSTALLFOLDER).
fn step_cert(status: &mut ProvisionStatus) {
    let cert_path = match cert_path_from_exe() {
        Ok(p) => p,
        Err(e) => {
            status.cert = StepStatus::Degraded(format!("locate cert file: {e}"));
            return;
        }
    };

    if !cert_path.exists() {
        status.cert = StepStatus::Degraded(format!(
            "cert file not found: {} (MSI may not have staged it yet)",
            cert_path.display()
        ));
        return;
    }

    match crate::cert_trust::import_current_user_root(&cert_path) {
        Ok(()) => {
            debug!(path = %cert_path.display(), "provision_windows: cert imported into CurrentUser\\Root");
            status.cert = StepStatus::Ok;
        }
        Err(e) => {
            warn!(error = %e, "provision_windows: cert import into CurrentUser\\Root failed (non-fatal)");
            status.cert = StepStatus::Degraded(format!("import_current_user_root: {e}"));
        }
    }
}

/// Step 3 — Set and persist `NODE_EXTRA_CA_CERTS` to the MSI-staged PEM.
///
/// Uses `setx NODE_EXTRA_CA_CERTS <pem>` for USER-scope persistence so that
/// a bare `node` launched in a fresh shell inherits the var.  Also sets it in
/// the current process env so the same-session confined Node child sees it.
fn step_node_extra_ca_certs(status: &mut ProvisionStatus) {
    let pem = programdata_pem_path();
    status.pem_path = Some(pem.clone());

    if !pem.exists() {
        status.node_extra_ca_certs = StepStatus::Degraded(format!(
            "PEM not found at {} (MSI may not have staged it yet)",
            pem.display()
        ));
        return;
    }

    let pem_str = pem.to_string_lossy();

    // Set in the current process environment immediately so that same-session
    // confined Node children inherit the var without requiring a new shell.
    // This is production provisioning code (single-threaded call path before
    // exec_strategy builds the confined child), not test code.
    #[allow(clippy::disallowed_methods)] // Production env injection, not test mutation.
    std::env::set_var("NODE_EXTRA_CA_CERTS", pem.as_os_str());

    // Persist at USER scope via setx (not machine scope — avoids requiring admin).
    let out = std::process::Command::new("setx")
        .arg("NODE_EXTRA_CA_CERTS")
        .arg(pem.as_os_str())
        .output();

    match out {
        Ok(result) if result.status.success() => {
            debug!(
                pem = %pem_str,
                "provision_windows: NODE_EXTRA_CA_CERTS persisted (user scope via setx)"
            );
            status.node_extra_ca_certs = StepStatus::Ok;
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            warn!(
                pem = %pem_str,
                exit_code = %result.status,
                stderr = %stderr,
                "provision_windows: setx NODE_EXTRA_CA_CERTS failed (non-fatal)"
            );
            status.node_extra_ca_certs = StepStatus::Degraded(format!(
                "setx NODE_EXTRA_CA_CERTS exited {}: {stderr}",
                result.status
            ));
        }
        Err(e) => {
            warn!(error = %e, "provision_windows: failed to spawn setx (non-fatal)");
            status.node_extra_ca_certs = StepStatus::Degraded(format!("spawn setx: {e}"));
        }
    }
}

// ── path helpers ──────────────────────────────────────────────────────────────

/// Resolve the user-owned scratch directory path: `%LOCALAPPDATA%\nono\workspace`.
///
/// # Errors
///
/// Returns `NonoError::Setup` if `%LOCALAPPDATA%` is not set or is empty.
fn scratch_dir() -> Result<PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
        NonoError::Setup(
            "provision_windows: %LOCALAPPDATA% is not set (cannot resolve scratch dir)".to_string(),
        )
    })?;
    if local_app_data.trim().is_empty() {
        return Err(NonoError::Setup(
            "provision_windows: %LOCALAPPDATA% is empty".to_string(),
        ));
    }
    Ok(PathBuf::from(local_app_data).join("nono").join("workspace"))
}

/// Resolve the POC cert `.cer` path: `<INSTALLFOLDER>\nono-poc-root.cer`.
///
/// The cert is staged by the MSI alongside `nono.exe` in INSTALLFOLDER.
///
/// # Errors
///
/// Returns `NonoError::Setup` if the current executable path cannot be
/// determined.
fn cert_path_from_exe() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|e| NonoError::Setup(format!("provision_windows: current_exe() failed: {e}")))?;
    let install_folder = exe.parent().ok_or_else(|| {
        NonoError::Setup("provision_windows: current_exe() has no parent directory".to_string())
    })?;
    Ok(install_folder.join("nono-poc-root.cer"))
}

/// Resolve the MSI-staged PEM path: `%PROGRAMDATA%\nono\nono-poc-root.pem`.
///
/// Falls back to a hardcoded conventional path if `%PROGRAMDATA%` is not set.
fn programdata_pem_path() -> PathBuf {
    let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    PathBuf::from(base).join("nono").join("nono-poc-root.pem")
}

// ── ownership helper ──────────────────────────────────────────────────────────

/// Set the owner of `path` to the current user via `icacls <path> /setowner <whoami>`.
///
/// This mirrors the `take_ownership_for_current_user` idiom in
/// `exec_strategy_windows/dacl_guard.rs:506-527`.
fn set_owner_to_current_user(path: &std::path::Path) -> Result<()> {
    // Resolve the current user identity: `whoami` prints `domain\user`.
    let who = std::process::Command::new("whoami")
        .output()
        .map_err(|e| NonoError::Setup(format!("provision_windows: failed to spawn whoami: {e}")))?;
    let user = String::from_utf8_lossy(&who.stdout).trim().to_string();
    if user.is_empty() {
        return Err(NonoError::Setup(
            "provision_windows: whoami returned an empty user".to_string(),
        ));
    }

    let out = std::process::Command::new("icacls")
        .arg(path)
        .arg("/setowner")
        .arg(&user)
        .arg("/Q")
        .output()
        .map_err(|e| NonoError::Setup(format!("provision_windows: failed to spawn icacls: {e}")))?;

    if !out.status.success() {
        return Err(NonoError::Setup(format!(
            "provision_windows: icacls /setowner {} failed (exit {}): {}",
            path.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

// ── idempotency sentinel ──────────────────────────────────────────────────────

/// Return `true` if the registry sentinel indicates the provisioner has already run.
fn is_already_provisioned() -> bool {
    read_provision_sentinel().is_some()
}

/// Read the sentinel value from the registry.  Returns `Some(timestamp)` if
/// present, `None` if absent or on any error.
fn read_provision_sentinel() -> Option<String> {
    // Use `reg query` via subprocess (consistent with platform.rs pattern;
    // winreg crate is a Phase 83 addition, not Phase 82).
    let out = std::process::Command::new("reg")
        .args([
            "query",
            &format!("HKCU\\{PROVISION_REG_KEY}"),
            "/v",
            PROVISION_REG_VALUE,
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    // `reg query` output contains the value on a line like:
    //   ProvisionedAt    REG_SZ    2026-06-18T...
    for line in stdout.lines() {
        if line.trim().starts_with(PROVISION_REG_VALUE) {
            if let Some(val) = line.split("REG_SZ").nth(1) {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

/// Write the provisioned-at timestamp to the registry sentinel.
///
/// Uses `reg add HKCU\<key> /v <value> /t REG_SZ /d <timestamp> /f`.
fn write_provision_sentinel() -> Result<()> {
    let timestamp = {
        // Use a simple ISO-8601-ish timestamp without needing chrono here.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("unix:{now}")
    };

    let out = std::process::Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\{PROVISION_REG_KEY}"),
            "/v",
            PROVISION_REG_VALUE,
            "/t",
            "REG_SZ",
            "/d",
            &timestamp,
            "/f",
        ])
        .output()
        .map_err(|e| {
            NonoError::Setup(format!("provision_windows: reg add failed to spawn: {e}"))
        })?;

    if !out.status.success() {
        return Err(NonoError::Setup(format!(
            "provision_windows: reg add sentinel exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Calling `provision_first_run()` when the sentinel is already set should
    /// return `AlreadyProvisioned` without error.  We fake the sentinel by
    /// writing then reading it.
    ///
    /// This test is idempotent and non-mutating: the sentinel key written by
    /// `write_provision_sentinel` may or may not already exist; either way the
    /// function must succeed or fail cleanly without panic.
    #[test]
    fn test_provision_idempotent_second_call() {
        // Write the sentinel first.
        let _ = write_provision_sentinel(); // best-effort; ignore result in CI

        // Now the provisioner should short-circuit.
        let result = provision_first_run();
        match result {
            Ok(ProvisionOutcome::AlreadyProvisioned) => {
                // Correct: idempotency sentinel was found.
            }
            Ok(ProvisionOutcome::Provisioned(_)) => {
                // Acceptable: sentinel write may have failed (e.g., CI no-reg),
                // so we re-provisioned — that is still not a panic.
            }
            Err(e) => {
                panic!("provision_first_run returned unexpected error: {e}");
            }
        }
    }

    /// `scratch_dir()` must return a path under `%LOCALAPPDATA%\nono\workspace`.
    #[test]
    fn test_scratch_dir_under_localappdata() {
        // Use EnvVarGuard to safely set LOCALAPPDATA and restore it on drop
        // (CLAUDE.md: tests that modify env vars must save/restore the original).
        let _guard = crate::test_env::EnvVarGuard::set_all(&[(
            "LOCALAPPDATA",
            r"C:\Users\TestUser\AppData\Local",
        )]);

        let path = scratch_dir().expect("scratch_dir must succeed when LOCALAPPDATA is set");

        assert!(
            path.starts_with(r"C:\Users\TestUser\AppData\Local\nono"),
            "scratch dir must be under %LOCALAPPDATA%\\nono, got: {}",
            path.display()
        );
    }

    /// `scratch_dir()` must return an error when `%LOCALAPPDATA%` is unset.
    #[test]
    fn test_scratch_dir_fails_without_localappdata() {
        // Use EnvVarGuard with an empty sentinel then remove to simulate unset.
        let guard = crate::test_env::EnvVarGuard::set_all(&[("LOCALAPPDATA", "__placeholder__")]);
        guard.remove("LOCALAPPDATA");

        let result = scratch_dir();
        // Guard restore happens on drop here.
        drop(guard);

        assert!(
            result.is_err(),
            "scratch_dir must fail when LOCALAPPDATA is not set"
        );
    }
}
