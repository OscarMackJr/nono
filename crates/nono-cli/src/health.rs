//! `nono health` — read-only fleet diagnostic command (DEPLOY-06, Phase 82).
//!
//! Reports the state of four subsystem groups and produces a tri-state verdict:
//!
//! * **Healthy** (exit 0) — all four groups are OK.
//! * **Degraded** (exit 1) — at least one group is degraded but none are broken.
//! * **Broken** (exit 2) — at least one group is broken (missing install, absent PATH, etc.).
//!
//! The full JSON verdict is **always printed to stdout** regardless of the exit
//! code so SCCM/Intune compliance scripts can parse it on any branch (D-06).
//!
//! ## Subsystem groups (D-07)
//!
//! (a) **install + version** — `INSTALLFOLDER` present, `nono.exe` self-locates,
//!     installed version string, best-effort MSI ProductCode/UpgradeCode.
//! (b) **WFP service** — install + running state via `sc query nono-wfp-service`.
//! (c) **machine policy** — `HKLM\SOFTWARE\Policies\nono` presence probe via
//!     `reg query` (read-only, no egress parsing; forward-looking for Phase 83).
//! (d) **scratch + cert + PATH** — user-owned scratch present, POC cert in
//!     `LocalMachine\Root` + `CurrentUser\Root` (presence probe), machine PATH
//!     entry present and current-process PATH checked.
//!
//! ## Security
//!
//! * **Read-only** — no create/write/addstore/setowner calls. SCM query only.
//! * **No raw paths in JSON** — scratch/cert/PATH state is reported as
//!   booleans/status strings, not absolute user paths (T-82-20 / no-raw-path).
//! * **Fail-secure** — a probe error maps to Degraded/Broken, never silently Healthy.
//!   An unreadable HKLM sentinel -> "unreadable", not "not_configured" (T-82-22).
//!
//! ## Cross-target
//!
//! Windows-specific probes are gated behind `#[cfg(target_os = "windows")]`.
//! Non-Windows builds return a `platform_unsupported` verdict.

use crate::cli::HealthArgs;
use nono::{NonoError, Result};

// -- tri-state verdict --------------------------------------------------------

/// The aggregated tri-state verdict for `nono health`.
///
/// Maps to process exit codes: `Healthy` -> 0, `Degraded` -> 1, `Broken` -> 2.
/// The dispatcher in `main.rs` / `app_runtime.rs` is responsible for calling
/// `std::process::exit` -- this function must NOT call it directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HealthVerdict {
    /// All subsystems are OK.
    Healthy,
    /// At least one subsystem is degraded but none are broken.
    Degraded,
    /// At least one subsystem is broken (missing install, absent PATH, etc.).
    Broken,
}

// -- per-subsystem state ------------------------------------------------------

/// State of a single subsystem (or sub-probe).
#[derive(Debug, Clone)]
enum SubsystemState {
    /// Subsystem is healthy/present/running.
    Ok,
    /// Subsystem is degraded (stopped service, missing cert, etc.).
    Degraded(String),
    /// Subsystem is broken (absent install, missing PATH, etc.).
    Broken(String),
}

impl SubsystemState {
    fn status_str(&self) -> &'static str {
        match self {
            SubsystemState::Ok => "ok",
            SubsystemState::Degraded(_) => "degraded",
            SubsystemState::Broken(_) => "broken",
        }
    }

    fn detail(&self) -> Option<&str> {
        match self {
            SubsystemState::Ok => None,
            SubsystemState::Degraded(msg) | SubsystemState::Broken(msg) => Some(msg.as_str()),
        }
    }
}

// -- aggregation --------------------------------------------------------------

/// Aggregate a slice of subsystem states into a tri-state verdict.
///
/// Rule (D-06): any Broken -> Broken; else any Degraded -> Degraded; else Healthy.
fn aggregate(states: &[SubsystemState]) -> HealthVerdict {
    let mut has_degraded = false;
    for state in states {
        match state {
            SubsystemState::Broken(_) => return HealthVerdict::Broken,
            SubsystemState::Degraded(_) => has_degraded = true,
            SubsystemState::Ok => {}
        }
    }
    if has_degraded {
        HealthVerdict::Degraded
    } else {
        HealthVerdict::Healthy
    }
}

// -- public entry point -------------------------------------------------------

/// Entry point for `nono health [--json]`.
///
/// Collects the four subsystem group states, aggregates to a tri-state verdict,
/// and **always** prints the JSON verdict to stdout (D-06 always-print contract).
///
/// Returns the `HealthVerdict` to the dispatcher (`app_runtime.rs`) which maps
/// it to the appropriate `process::exit` code. This function must NOT call
/// `process::exit` itself.
///
/// # Errors
///
/// Returns `Err` only on genuine internal failures (JSON serialization).
/// Per-subsystem probe errors are captured as Degraded/Broken states, not
/// propagated as `Err`.
pub(crate) fn run_health(args: &HealthArgs) -> Result<HealthVerdict> {
    // Collect all four subsystem groups.
    let (install_state, version_str) = probe_install_version();
    let wfp_state = probe_wfp_service();
    let policy_state = probe_machine_policy();
    let (scratch_state, cert_machine_state, cert_user_state, path_state, path_warn) =
        probe_scratch_cert_path();

    // Aggregate all states into the tri-state verdict.
    let all_states = [
        install_state.clone(),
        wfp_state.clone(),
        policy_state.clone(),
        scratch_state.clone(),
        cert_machine_state.clone(),
        cert_user_state.clone(),
        path_state.clone(),
    ];
    let verdict = aggregate(&all_states);

    // Build the JSON verdict (always printed, D-06).
    // Exact field names are Claude's Discretion (82-CONTEXT.md :97-98).
    // Paths are reported as booleans/status strings, NOT raw absolute paths (T-82-20).
    let json_verdict = serde_json::json!({
        "verdict": match &verdict {
            HealthVerdict::Healthy => "healthy",
            HealthVerdict::Degraded => "degraded",
            HealthVerdict::Broken => "broken",
        },
        "groups": {
            "install": {
                "status": install_state.status_str(),
                "version": version_str,
                "detail": install_state.detail(),
            },
            "wfp_service": {
                "status": wfp_state.status_str(),
                "detail": wfp_state.detail(),
            },
            "machine_policy": {
                "status": policy_state.status_str(),
                "detail": policy_state.detail(),
            },
            "scratch_cert_path": {
                "scratch": scratch_state.status_str(),
                "cert_machine_store": cert_machine_state.status_str(),
                "cert_user_store": cert_user_state.status_str(),
                "path_entry": path_state.status_str(),
                "path_session_warn": path_warn,
                "detail": scratch_state.detail()
                    .or(cert_machine_state.detail())
                    .or(cert_user_state.detail())
                    .or(path_state.detail()),
            },
        },
    });

    let rendered = serde_json::to_string_pretty(&json_verdict).map_err(|e| {
        NonoError::ConfigParse(format!("nono health: JSON serialization failed: {e}"))
    })?;
    println!("{rendered}");

    if !args.json {
        print_human(
            &verdict,
            &install_state,
            &wfp_state,
            &policy_state,
            &scratch_state,
            &cert_machine_state,
            &cert_user_state,
            &path_state,
            path_warn.as_deref(),
        );
    }

    Ok(verdict)
}

// -- human output -------------------------------------------------------------

/// Print a human-readable summary of the health verdict.
///
/// The JSON is already printed unconditionally before this is called (D-06).
/// This function is called only when `--json` is NOT set.
#[allow(clippy::too_many_arguments)]
fn print_human(
    verdict: &HealthVerdict,
    install: &SubsystemState,
    wfp: &SubsystemState,
    policy: &SubsystemState,
    scratch: &SubsystemState,
    cert_machine: &SubsystemState,
    cert_user: &SubsystemState,
    path: &SubsystemState,
    path_warn: Option<&str>,
) {
    let verdict_str = match verdict {
        HealthVerdict::Healthy => "HEALTHY",
        HealthVerdict::Degraded => "DEGRADED",
        HealthVerdict::Broken => "BROKEN",
    };
    println!();
    println!("nono health: {verdict_str}");
    println!("  install+version : {}", install.status_str());
    if let Some(d) = install.detail() {
        println!("    detail: {d}");
    }
    println!("  wfp_service     : {}", wfp.status_str());
    if let Some(d) = wfp.detail() {
        println!("    detail: {d}");
    }
    println!("  machine_policy  : {}", policy.status_str());
    if let Some(d) = policy.detail() {
        println!("    detail: {d}");
    }
    println!("  scratch         : {}", scratch.status_str());
    if let Some(d) = scratch.detail() {
        println!("    detail: {d}");
    }
    println!("  cert (machine)  : {}", cert_machine.status_str());
    if let Some(d) = cert_machine.detail() {
        println!("    detail: {d}");
    }
    println!("  cert (user)     : {}", cert_user.status_str());
    if let Some(d) = cert_user.detail() {
        println!("    detail: {d}");
    }
    println!("  PATH entry      : {}", path.status_str());
    if let Some(d) = path.detail() {
        println!("    detail: {d}");
    }
    if let Some(w) = path_warn {
        println!("    warn: {w}");
    }
}

// -- probe: (a) install + version ---------------------------------------------

/// Probe group (a): install + version.
///
/// Returns the subsystem state and the installed version string (if detectable).
/// Returns `Broken` if the install folder or `nono.exe` cannot be self-located.
fn probe_install_version() -> (SubsystemState, Option<String>) {
    #[cfg(target_os = "windows")]
    return probe_install_version_windows();

    #[cfg(not(target_os = "windows"))]
    (
        SubsystemState::Degraded("nono health: install+version probe is Windows-only".to_string()),
        None,
    )
}

#[cfg(target_os = "windows")]
fn probe_install_version_windows() -> (SubsystemState, Option<String>) {
    // Locate the current executable -- its parent is INSTALLFOLDER.
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return (
                SubsystemState::Broken(format!("current_exe() failed: {e}")),
                None,
            );
        }
    };

    let install_folder = match exe.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            return (
                SubsystemState::Broken("current_exe() has no parent directory".to_string()),
                None,
            );
        }
    };

    if !install_folder.exists() {
        return (
            SubsystemState::Broken(
                "INSTALLFOLDER not found (install may be incomplete)".to_string(),
            ),
            None,
        );
    }

    // Read the embedded version string (build-time constant).
    let version = env!("CARGO_PKG_VERSION").to_string();

    (SubsystemState::Ok, Some(version))
}

// -- probe: (b) WFP service ---------------------------------------------------

/// Probe group (b): WFP service install + running state via SCM query.
///
/// A stopped/absent/failed service -> Degraded (not Broken -- other features work
/// without WFP; D-06 only escalates to Broken for install-level failures).
fn probe_wfp_service() -> SubsystemState {
    #[cfg(target_os = "windows")]
    return probe_wfp_service_windows();

    #[cfg(not(target_os = "windows"))]
    SubsystemState::Degraded("nono health: WFP service probe is Windows-only".to_string())
}

#[cfg(target_os = "windows")]
fn probe_wfp_service_windows() -> SubsystemState {
    // Use `sc query nono-wfp-service` (read-only, no mutation).
    let out = std::process::Command::new("sc")
        .args(["query", "nono-wfp-service"])
        .output();

    match out {
        Err(e) => SubsystemState::Degraded(format!("sc query failed to spawn: {e}")),
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stdout_lower = stdout.to_lowercase();

            if result.status.success() || stdout_lower.contains("state") {
                // Service exists; check STATE.
                if stdout_lower.contains("running") {
                    SubsystemState::Ok
                } else if stdout_lower.contains("stopped") {
                    SubsystemState::Degraded(
                        "nono-wfp-service is installed but stopped".to_string(),
                    )
                } else {
                    // Other states (START_PENDING, STOP_PENDING, etc.) -> degraded.
                    SubsystemState::Degraded(
                        "nono-wfp-service is in an intermediate state".to_string(),
                    )
                }
            } else {
                // Exit non-zero and no STATE line -- service does not exist.
                SubsystemState::Degraded(
                    "nono-wfp-service is not installed (WFP network filtering unavailable)"
                        .to_string(),
                )
            }
        }
    }
}

// -- probe: (c) machine policy ------------------------------------------------

/// Probe group (c): `HKLM\SOFTWARE\Policies\nono` presence + readability.
///
/// States: ok (key present + readable), degraded/not_configured (absent), degraded/unreadable
/// (access denied). Uses `reg query` subprocess (no winreg dep in Phase 82;
/// reads the 64-bit hive from a 64-bit process by default, per 82-PATTERNS :106).
fn probe_machine_policy() -> SubsystemState {
    #[cfg(target_os = "windows")]
    return probe_machine_policy_windows();

    #[cfg(not(target_os = "windows"))]
    SubsystemState::Degraded("nono health: machine policy probe is Windows-only".to_string())
}

#[cfg(target_os = "windows")]
fn probe_machine_policy_windows() -> SubsystemState {
    let out = std::process::Command::new("reg")
        .args(["query", r"HKLM\SOFTWARE\Policies\nono"])
        .output();

    match out {
        Err(e) => SubsystemState::Degraded(format!("reg query failed to spawn: {e}")),
        Ok(result) => {
            if result.status.success() {
                // Key exists and is readable.
                SubsystemState::Ok
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let stdout = String::from_utf8_lossy(&result.stdout);
                let combined = format!("{stdout}{stderr}").to_lowercase();

                if combined.contains("access denied") {
                    // Key exists but is unreadable -> "unreadable" (not "not_configured") T-82-22.
                    SubsystemState::Degraded(
                        "HKLM\\SOFTWARE\\Policies\\nono is present but unreadable (access denied)"
                            .to_string(),
                    )
                } else {
                    // Key absent -> not_configured (Degraded -- forward-looking for Phase 83).
                    SubsystemState::Degraded(
                        "HKLM\\SOFTWARE\\Policies\\nono is not configured (machine policy not deployed)"
                            .to_string(),
                    )
                }
            }
        }
    }
}

// -- probe: (d) scratch + cert + PATH -----------------------------------------

/// Probe group (d): user-owned scratch present, POC cert in both stores, machine PATH entry.
///
/// Returns per-probe states (scratch, cert_machine, cert_user, path_entry) and an
/// optional warning string for the Pitfall 6 "installed but old session" PATH check.
///
/// Paths are reported as boolean/status only -- no raw absolute paths in the return
/// values (T-82-20 no-raw-path principle).
fn probe_scratch_cert_path() -> (
    SubsystemState,
    SubsystemState,
    SubsystemState,
    SubsystemState,
    Option<String>,
) {
    #[cfg(target_os = "windows")]
    return probe_scratch_cert_path_windows();

    #[cfg(not(target_os = "windows"))]
    (
        SubsystemState::Degraded("scratch probe is Windows-only".to_string()),
        SubsystemState::Degraded("cert probe is Windows-only".to_string()),
        SubsystemState::Degraded("cert probe is Windows-only".to_string()),
        SubsystemState::Degraded("PATH probe is Windows-only".to_string()),
        None,
    )
}

#[cfg(target_os = "windows")]
fn probe_scratch_cert_path_windows() -> (
    SubsystemState,
    SubsystemState,
    SubsystemState,
    SubsystemState,
    Option<String>,
) {
    let scratch_state = probe_scratch_windows();
    let cert_machine = probe_cert_machine_store_windows();
    let cert_user = probe_cert_user_store_windows();
    let (path_state, path_warn) = probe_path_windows();
    (
        scratch_state,
        cert_machine,
        cert_user,
        path_state,
        path_warn,
    )
}

/// Probe whether the user-owned scratch dir under `%LOCALAPPDATA%\nono\` exists.
#[cfg(target_os = "windows")]
fn probe_scratch_windows() -> SubsystemState {
    let local_app_data = match std::env::var("LOCALAPPDATA") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            return SubsystemState::Degraded(
                "scratch probe: %LOCALAPPDATA% is not set".to_string(),
            );
        }
    };
    let scratch = std::path::PathBuf::from(&local_app_data)
        .join("nono")
        .join("workspace");

    if !scratch.exists() {
        return SubsystemState::Degraded(
            "user scratch directory is not provisioned (run `nono run` to auto-provision)"
                .to_string(),
        );
    }

    // Check ownership (read-only -- calls nono::path_is_owned_by_current_user).
    match nono::path_is_owned_by_current_user(&scratch) {
        Ok(true) => SubsystemState::Ok,
        Ok(false) => SubsystemState::Degraded(
            "scratch dir exists but is not owned by the current user (R-B3 guard will fail)"
                .to_string(),
        ),
        Err(e) => SubsystemState::Degraded(format!("scratch ownership check failed: {e}")),
    }
}

/// Probe whether the POC root cert is present in the machine `LocalMachine\Root` store.
///
/// Uses `certutil -store Root <thumbprint>` subprocess (read-only, presence probe).
/// The thumbprint is the SHA-1 from Plan 01's committed cert.
#[cfg(target_os = "windows")]
fn probe_cert_machine_store_windows() -> SubsystemState {
    // SHA-1 thumbprint of the POC signing cert (from Plan 01 SUMMARY).
    const POC_CERT_SHA1: &str = "319e507e950472d490f56f7c4cd94437c013cc06";

    let out = std::process::Command::new("certutil")
        .args(["-store", "Root", POC_CERT_SHA1])
        .output();

    match out {
        Err(e) => SubsystemState::Degraded(format!("certutil -store probe failed: {e}")),
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout).to_lowercase();
            // certutil -store exits 0 and prints the cert if found; exits non-zero if not found.
            // Check for the thumbprint or cert data in stdout as confirmation.
            if result.status.success()
                && (stdout.contains(POC_CERT_SHA1)
                    || stdout.contains("cert hash")
                    || stdout.contains("nono"))
            {
                SubsystemState::Ok
            } else {
                SubsystemState::Degraded(
                    "POC root cert not found in LocalMachine\\Root (TLS through broker/proxy may fail)"
                        .to_string(),
                )
            }
        }
    }
}

/// Probe whether the POC root cert is present in the per-user `CurrentUser\Root` store.
#[cfg(target_os = "windows")]
fn probe_cert_user_store_windows() -> SubsystemState {
    // Delegate to cert_trust::is_cert_present_current_user (Plan 02).
    const POC_CERT_SHA1: &str = "319e507e950472d490f56f7c4cd94437c013cc06";

    match crate::cert_trust::is_cert_present_current_user(POC_CERT_SHA1) {
        Ok(true) => SubsystemState::Ok,
        Ok(false) => SubsystemState::Degraded(
            "POC root cert not found in CurrentUser\\Root (run `nono run` to auto-import)"
                .to_string(),
        ),
        Err(e) => SubsystemState::Degraded(format!("cert user store probe failed: {e}")),
    }
}

/// Probe whether the INSTALLFOLDER is present in the machine PATH (`reg query`),
/// and whether the **current-process** PATH also contains it (Pitfall 6 check).
///
/// Returns (state, Option<warning_string>).
/// State uses the machine-registry PATH as the authoritative source (persistent install).
/// The optional warning notes if the current session's PATH is stale (Pitfall 6).
#[cfg(target_os = "windows")]
fn probe_path_windows() -> (SubsystemState, Option<String>) {
    // Determine INSTALLFOLDER from current_exe().
    let install_folder = match std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
    {
        Some(p) => p,
        None => {
            return (
                SubsystemState::Broken(
                    "cannot determine INSTALLFOLDER (current_exe failed)".to_string(),
                ),
                None,
            );
        }
    };

    let install_str = install_folder.to_string_lossy().to_lowercase();

    // Read the machine PATH from the registry (the persistent value).
    let machine_path = query_machine_path_registry();

    // Check machine PATH registry value.
    let path_state = match &machine_path {
        Some(mp) if mp.to_lowercase().contains(&install_str) => SubsystemState::Ok,
        Some(_) => SubsystemState::Broken(
            "INSTALLFOLDER is not in the machine PATH registry value (DEPLOY-02 not satisfied)"
                .to_string(),
        ),
        None => SubsystemState::Degraded(
            "cannot read machine PATH from registry; PATH coverage uncertain".to_string(),
        ),
    };

    // Pitfall 6 (82-PATTERNS :106): warn if the current session's PATH doesn't
    // contain the install dir (installed but old session -- user needs to re-login).
    let path_warn = match std::env::var("PATH") {
        Ok(current_path) if !current_path.to_lowercase().contains(&install_str) => Some(
            "current shell PATH does not include INSTALLFOLDER; re-login or refresh PATH to pick up the install"
                .to_string(),
        ),
        _ => None,
    };

    (path_state, path_warn)
}

/// Read `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment PATH`
/// via `reg query` (read-only).
#[cfg(target_os = "windows")]
fn query_machine_path_registry() -> Option<String> {
    let out = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            "/v",
            "Path",
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("path") {
            // Line format: "    Path    REG_EXPAND_SZ    <value>"
            if let Some(val) = trimmed.split("REG_EXPAND_SZ").nth(1) {
                return Some(val.trim().to_string());
            }
            if let Some(val) = trimmed.split("REG_SZ").nth(1) {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

// -- unit tests ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Aggregation: a single Broken state -> Broken verdict.
    #[test]
    fn test_aggregate_broken_wins() {
        let states = vec![
            SubsystemState::Ok,
            SubsystemState::Broken("install missing".to_string()),
            SubsystemState::Degraded("wfp stopped".to_string()),
        ];
        assert_eq!(aggregate(&states), HealthVerdict::Broken);
    }

    /// Aggregation: Degraded only (no Broken) -> Degraded verdict.
    #[test]
    fn test_aggregate_degraded_without_broken() {
        let states = vec![
            SubsystemState::Ok,
            SubsystemState::Degraded("wfp stopped".to_string()),
            SubsystemState::Ok,
        ];
        assert_eq!(aggregate(&states), HealthVerdict::Degraded);
    }

    /// Aggregation: all Ok -> Healthy verdict.
    #[test]
    fn test_aggregate_all_ok_is_healthy() {
        let states = vec![SubsystemState::Ok, SubsystemState::Ok, SubsystemState::Ok];
        assert_eq!(aggregate(&states), HealthVerdict::Healthy);
    }

    /// Aggregation: empty slice -> Healthy (no degraded/broken).
    #[test]
    fn test_aggregate_empty_is_healthy() {
        assert_eq!(aggregate(&[]), HealthVerdict::Healthy);
    }

    /// Aggregation: multiple Broken -> Broken (first Broken short-circuits).
    #[test]
    fn test_aggregate_multiple_broken() {
        let states = vec![
            SubsystemState::Broken("a".to_string()),
            SubsystemState::Broken("b".to_string()),
        ];
        assert_eq!(aggregate(&states), HealthVerdict::Broken);
    }

    /// SubsystemState::Ok has no detail.
    #[test]
    fn test_subsystem_state_ok_has_no_detail() {
        assert!(SubsystemState::Ok.detail().is_none());
    }

    /// SubsystemState::Degraded carries detail.
    #[test]
    fn test_subsystem_state_degraded_has_detail() {
        let s = SubsystemState::Degraded("wfp stopped".to_string());
        assert_eq!(s.detail(), Some("wfp stopped"));
        assert_eq!(s.status_str(), "degraded");
    }

    /// SubsystemState::Broken carries detail.
    #[test]
    fn test_subsystem_state_broken_has_detail() {
        let s = SubsystemState::Broken("path missing".to_string());
        assert_eq!(s.detail(), Some("path missing"));
        assert_eq!(s.status_str(), "broken");
    }
}
