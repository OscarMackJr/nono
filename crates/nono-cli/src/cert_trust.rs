//! Reusable certificate-store import logic for nono fleet deployment (DEPLOY-05).
//!
//! Provides:
//! - [`import_machine_root`]: imports a root cert into the machine `Root` and
//!   `TrustedPublisher` stores (invoked by the MSI custom action via
//!   `nono setup --trust-root`).  This is the **single source of truth** for
//!   the store-name list; the list is never duplicated in a PowerShell
//!   here-string or WiX fragment.
//! - [`import_current_user_root`]: idempotently imports a root cert into the
//!   per-user `CurrentUser\Root` store.
//! - [`is_cert_present_current_user`]: presence probe by SHA-1 thumbprint;
//!   used by the idempotency guard in [`import_current_user_root`].
//!
//! On non-Windows targets every public function is a cfg-stub returning a
//! typed `NonoError::Unsupported` so the crate compiles for cross-target
//! Clippy (Linux + macOS) without polluting those builds.

use nono::{NonoError, Result};
use std::path::Path;

// ── machine store-name list (single source of truth, Blocker-2 / D-04) ──────
//
// `certutil -addstore -f Root <cert>`           → CryptoAPI / PowerShell TLS
// `certutil -addstore -f TrustedPublisher <cert>` → Authenticode broker self-trust gate
//
// Do NOT add stores here without also evaluating the MSI custom-action contract
// (Plan 82-01 ExeCommand "nono.exe setup --trust-root nono-poc-root.cer").
#[cfg(target_os = "windows")]
const MACHINE_STORES: &[&str] = &["Root", "TrustedPublisher"];

// ── Windows implementation ────────────────────────────────────────────────────

/// Import a root certificate into the machine `Root` **and** `TrustedPublisher`
/// certificate stores.
///
/// This is the single Rust source of truth for the store-name list
/// (`Root`, `TrustedPublisher`).  Every invocation iterates that list and runs
///
/// ```text
/// certutil -addstore -f <store> <cert_path>
/// ```
///
/// for each store.  Admin / SYSTEM context is required for machine stores;
/// a non-elevated call returns a typed `NonoError` (the caller is responsible
/// for deciding fatality — the MSI custom action uses `Return="ignore"`).
///
/// # Errors
///
/// Returns `NonoError::Setup` if any `certutil` invocation fails or exits
/// non-zero.  On non-Windows targets returns `NonoError::Unsupported`.
#[cfg(target_os = "windows")]
pub(crate) fn import_machine_root(cert_path: &Path) -> Result<()> {
    for store in MACHINE_STORES {
        run_certutil_addstore(store, cert_path, false)?;
    }
    Ok(())
}

/// Import a root certificate into the per-user `CurrentUser\Root` store.
///
/// Idempotent: if a certificate with the given thumbprint is already present
/// the function returns `Ok(())` without re-importing.  The thumbprint is
/// derived by running `certutil -dump <cert>` and extracting the SHA-1 hash;
/// on failure the idempotency probe is skipped and the import proceeds
/// (best-effort idempotency).
///
/// # Errors
///
/// Returns `NonoError::Setup` if `certutil` exits non-zero.
/// On non-Windows returns `NonoError::Unsupported`.
#[cfg(target_os = "windows")]
pub(crate) fn import_current_user_root(cert_path: &Path) -> Result<()> {
    // Best-effort thumbprint extraction for idempotency.
    if let Ok(thumbprint) = extract_thumbprint(cert_path) {
        match is_cert_present_current_user(&thumbprint) {
            Ok(true) => {
                tracing::debug!(
                    thumbprint,
                    "cert_trust: cert already in CurrentUser\\Root — skipping import (idempotent)"
                );
                return Ok(());
            }
            Ok(false) => {
                // Not present — proceed with import.
            }
            Err(e) => {
                // Probe failed; log and fall through to import anyway.
                tracing::warn!(
                    error = %e,
                    "cert_trust: thumbprint presence probe failed; proceeding with import"
                );
            }
        }
    }
    run_certutil_addstore("Root", cert_path, true)
}

/// Check whether a certificate identified by its SHA-1 thumbprint is already
/// present in the `CurrentUser\Root` store.
///
/// Returns `Ok(true)` if present, `Ok(false)` if absent.
///
/// # Errors
///
/// Returns `NonoError::Setup` if the `certutil` invocation fails to run.
/// On non-Windows returns `NonoError::Unsupported`.
#[cfg(target_os = "windows")]
#[must_use = "caller must check whether cert is already present before importing"]
pub(crate) fn is_cert_present_current_user(thumbprint: &str) -> Result<bool> {
    // `certutil -user -store Root` dumps the current-user Root store.
    // We search the output for the thumbprint (case-insensitive).
    let output = std::process::Command::new("certutil")
        .args(["-user", "-store", "Root"])
        .output()
        .map_err(|e| {
            NonoError::Setup(format!(
                "cert_trust: failed to spawn certutil for presence probe: {e}"
            ))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let normalised_thumbprint = thumbprint.replace(" ", "").to_ascii_uppercase();
    let found = stdout.lines().any(|line| {
        line.replace(" ", "")
            .to_ascii_uppercase()
            .contains(&normalised_thumbprint)
    });
    Ok(found)
}

// ── private helpers (Windows) ─────────────────────────────────────────────────

/// Run `certutil [-user] -addstore -f <store> <cert_path>`.
///
/// `user_scope=true` prepends the `-user` flag (per-user store import).
#[cfg(target_os = "windows")]
fn run_certutil_addstore(store: &str, cert_path: &Path, user_scope: bool) -> Result<()> {
    let mut cmd = std::process::Command::new("certutil");
    if user_scope {
        cmd.arg("-user");
    }
    cmd.arg("-addstore").arg("-f").arg(store).arg(cert_path);

    let output = cmd.output().map_err(|e| {
        NonoError::Setup(format!(
            "cert_trust: failed to spawn certutil -addstore -f {store}: {e}"
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(NonoError::Setup(format!(
            "cert_trust: certutil -addstore -f {store} exited {}: {stdout} {stderr}",
            output.status
        )));
    }
    Ok(())
}

/// Extract the SHA-1 thumbprint from a certificate file using `certutil -dump`.
///
/// Returns the first non-empty thumbprint found in the dump output.
/// On any failure returns `Err` (callers use this for idempotency — a failure
/// is not fatal; import proceeds anyway).
#[cfg(target_os = "windows")]
fn extract_thumbprint(cert_path: &Path) -> Result<String> {
    let output = std::process::Command::new("certutil")
        .args(["-dump"])
        .arg(cert_path)
        .output()
        .map_err(|e| {
            NonoError::Setup(format!("cert_trust: certutil -dump failed to spawn: {e}"))
        })?;

    if !output.status.success() {
        return Err(NonoError::Setup(format!(
            "cert_trust: certutil -dump exited {}",
            output.status
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // `certutil -dump` prints lines like:
    //   Cert Hash(sha1): XX XX XX XX ...
    for line in stdout.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("cert hash(sha1)") || lower.contains("sha1 hash") {
            if let Some(colon_pos) = line.find(':') {
                let raw = &line[colon_pos + 1..];
                let cleaned: String = raw.chars().filter(|c| c.is_ascii_hexdigit()).collect();
                if !cleaned.is_empty() {
                    return Ok(cleaned.to_ascii_uppercase());
                }
            }
        }
    }

    Err(NonoError::Setup(
        "cert_trust: could not extract SHA-1 thumbprint from certutil -dump output".to_string(),
    ))
}

// ── non-Windows stubs ─────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
pub(crate) fn import_machine_root(_cert_path: &Path) -> Result<()> {
    Err(NonoError::UnsupportedPlatform(
        "cert_trust::import_machine_root is Windows-only".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn import_current_user_root(_cert_path: &Path) -> Result<()> {
    Err(NonoError::UnsupportedPlatform(
        "cert_trust::import_current_user_root is Windows-only".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
#[must_use = "caller must check whether cert is already present before importing"]
pub(crate) fn is_cert_present_current_user(_thumbprint: &str) -> Result<bool> {
    Err(NonoError::UnsupportedPlatform(
        "cert_trust::is_cert_present_current_user is Windows-only".to_string(),
    ))
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// An absent/random thumbprint must return `Ok(false)` on Windows.
    /// On non-Windows platforms the function returns `Err(Unsupported)` —
    /// that is the correct cross-platform behaviour; we assert it does not
    /// panic (no `.unwrap()` anywhere in the call chain).
    #[test]
    fn test_is_cert_present_absent_thumbprint_no_panic() {
        // A thumbprint that will never be in any cert store.
        let result = is_cert_present_current_user("DEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF");
        // Must not panic.  On Windows it should be Ok(false); on non-Windows Err(Unsupported).
        match result {
            Ok(present) => {
                // Windows: the cert must not be there.
                assert!(
                    !present,
                    "randomly generated thumbprint should not be present"
                );
            }
            Err(NonoError::UnsupportedPlatform(_)) => {
                // Non-Windows stub — expected.
            }
            Err(other) => {
                panic!("unexpected error from is_cert_present_current_user: {other}");
            }
        }
    }

    /// Verify that the public API surface compiles on all platforms (cross-target Clippy).
    #[test]
    fn test_cert_trust_api_compiles() {
        let _: fn(&Path) -> Result<()> = import_machine_root;
        let _: fn(&Path) -> Result<()> = import_current_user_root;
        let _: fn(&str) -> Result<bool> = is_cert_present_current_user;
    }
}
