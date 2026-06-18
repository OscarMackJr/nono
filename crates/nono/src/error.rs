//! Error types for the nono library

use std::path::PathBuf;
use thiserror::Error;

/// LOCKED hint string for the `cgroup_v2` `UnsupportedKernelFeature` variant.
///
/// Keep in sync with all cgroup-v2-detecting call sites in
/// `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`. The boot-flag
/// hint MUST remain stable for REQ-RESL-NIX-01 acceptance #5 — FFI
/// consumers grep this exact substring from `nono_last_error()` Display
/// output. CLAUDE.md § Coding Standards "lazy use of dead code" forbids
/// dropping or renaming this const without auditing every grep contract.
///
/// Phase 44 WR-02 P37 (REQ-REVIEW-FU-01 D-44-A4): promoted from a
/// test-mod-local `LOCKED_HINT` to module-level `pub const` after the
/// duplicated literal accreted across 6 supervisor_linux.rs call sites.
/// The promotion deduplicates the literal and gives library + CLI a
/// single source of truth.
pub const CGROUP_V2_HINT: &str =
    "cgroup v2 required; boot with systemd.unified_cgroup_hierarchy=1 or cgroup_no_v1=all";

/// Errors that can occur in the nono library
#[derive(Error, Debug)]
pub enum NonoError {
    // Path errors
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Expected a directory but got a file: {0}")]
    ExpectedDirectory(PathBuf),

    #[error("Expected a file but got a directory: {0}")]
    ExpectedFile(PathBuf),

    #[error("Failed to canonicalize path {path}: {source}")]
    PathCanonicalization {
        path: PathBuf,
        source: std::io::Error,
    },

    // Capability errors
    #[error("No filesystem capabilities specified")]
    NoCapabilities,

    #[error("No command specified")]
    NoCommand,

    #[error("CWD access requires --allow-cwd in silent mode")]
    CwdPromptRequired,

    // Sandbox errors
    #[error("Sandbox initialization failed: {0}")]
    SandboxInit(String),

    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),

    /// A feature is not supported on this specific platform.
    ///
    /// This is distinct from [`UnsupportedPlatform`] in that the platform itself
    /// is supported, but a specific feature within that platform is not available.
    /// For example, `--cpu-percent` is not supported on macOS because there is no
    /// per-process CPU-quota equivalent, but nono itself runs fine on macOS.
    ///
    /// The `feature` field contains a stable machine-readable identifier (e.g.
    /// `"cpu_percent_macos"`) that tests and callers can match on.
    #[error("Feature not supported on this platform: {feature}")]
    NotSupportedOnPlatform { feature: String },

    /// The host kernel does not support a feature nono requires.
    ///
    /// This is distinct from [`UnsupportedPlatform`] in that the platform itself
    /// is supported, and distinct from [`NotSupportedOnPlatform`] in that the
    /// feature exists on this OS but the kernel is misconfigured (e.g., Linux
    /// cgroup v1 instead of v2). The `hint` field carries an actionable
    /// remediation pointer (e.g., a boot-flag suggestion).
    ///
    /// Phase 37 D-05 / D-07: introduced for the `cgroup_v2` detection sites in
    /// `exec_strategy/supervisor_linux.rs` so cgroup-v1 hosts that pass
    /// `--memory` / `--cpu-percent` / `--max-processes` fail closed with a typed
    /// variant carrying the LOCKED `cgroup_no_v1=all` boot-flag hint per
    /// REQ-RESL-NIX-01 acceptance #3.
    #[error("Kernel feature not supported: {feature} ({hint})")]
    UnsupportedKernelFeature { feature: String, hint: String },

    #[error("Command '{command}' is blocked: {reason}")]
    BlockedCommand { command: String, reason: String },

    /// Broker binary (`nono-shell-broker.exe`) not found as sibling of the
    /// running `nono.exe`. Resolved via `std::env::current_exe()` parent +
    /// platform-specific filename (Phase 31 D-07).
    ///
    /// No env-var override surface (D-07): env-poisoning would let an attacker
    /// redirect the broker to a malicious binary.
    #[error("Broker binary not found: {path:?}")]
    BrokerNotFound { path: PathBuf },

    // Landlock errors (Linux only)
    #[cfg(target_os = "linux")]
    #[error("Landlock error: {0}")]
    Landlock(#[from] landlock::RulesetError),

    #[cfg(target_os = "linux")]
    #[error("Landlock path error: {0}")]
    LandlockPath(#[from] landlock::PathFdError),

    // Keystore errors
    #[error("Failed to access system keystore: {0}")]
    KeystoreAccess(String),

    #[error("Secret not found in keystore: {0}")]
    SecretNotFound(String),

    // Configuration errors (CLI-level but useful in library)
    #[error("Configuration parse error: {0}")]
    ConfigParse(String),

    #[error("Failed to write config to {path}: {source}")]
    ConfigWrite {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Profile read error at {path}: {source}")]
    ProfileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Profile parse error: {0}")]
    ProfileParse(String),

    #[error("Profile inheritance error: {0}")]
    ProfileInheritance(String),

    #[error("Home directory not found")]
    HomeNotFound,

    #[error("Setup error: {0}")]
    Setup(String),

    #[error("Learn mode error: {0}")]
    LearnError(String),

    #[error("Hook installation error: {0}")]
    HookInstall(String),

    #[error("Environment variable '{var}' validation failed: {reason}")]
    EnvVarValidation { var: String, reason: String },

    #[error("Capability state file validation failed: {reason}")]
    CapFileValidation { reason: String },

    #[error("Capability state file too large: {size} bytes (max: {max} bytes)")]
    CapFileTooLarge { size: u64, max: u64 },

    // Configuration read errors
    #[error("Failed to read config at {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    // Version tracking errors
    #[error("Version downgrade detected for {config}: {current} -> {attempted}")]
    VersionDowngrade {
        config: String,
        current: u64,
        attempted: u64,
    },

    // Command execution errors
    #[error("Command execution failed: {0}")]
    CommandExecution(#[source] std::io::Error),

    // Undo/snapshot errors
    #[error("Object store error: {0}")]
    ObjectStore(String),

    #[error("Snapshot error: {0}")]
    Snapshot(String),

    #[error("Hash integrity mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already has an active attached client")]
    AttachBusy,

    /// Failed to apply (or revert) a Windows mandatory integrity label on a path.
    ///
    /// Fail-closed: any `SetNamedSecurityInfoW` non-zero return surfaces here.
    /// The `hint` field carries a human-actionable diagnostic string (e.g.
    /// "Ensure the target file is writable by the current user and is on NTFS
    /// (not ReFS or a network share).") that callers can show to end users.
    #[error("Failed to apply integrity label to {path}: {hint} (HRESULT: 0x{hresult:08X})")]
    LabelApplyFailed {
        /// The exact path that failed.
        path: PathBuf,
        /// The Win32 HRESULT (or raw error code) returned by the OS.
        hresult: u32,
        /// Human-actionable hint for remediation.
        hint: String,
    },

    /// Failed to apply (or revoke) a Windows DACL allow-ACE granting a SID
    /// write-class rights on a path.
    ///
    /// Distinct from [`LabelApplyFailed`] (which mutates the *mandatory
    /// integrity label* / SACL): this variant covers the *discretionary*
    /// access-control list (DACL) edit performed on the `WriteRestricted`
    /// token arm so the synthetic per-session restricting SID can write to
    /// already-user-owned, already-grant-scoped paths. Fail-closed: any
    /// non-zero return from `GetNamedSecurityInfoW` / `SetEntriesInAclW` /
    /// `SetNamedSecurityInfoW` (or a SID-string parse failure) surfaces here.
    /// The `hint` field carries a human-actionable diagnostic string.
    #[error("Failed to apply DACL grant to {path}: {hint} (HRESULT: 0x{hresult:08X})")]
    DaclApplyFailed {
        /// The exact path that failed.
        path: PathBuf,
        /// The Win32 HRESULT (or raw error code) returned by the OS.
        hresult: u32,
        /// Human-actionable hint for remediation.
        hint: String,
    },

    /// Machine-level egress policy could not be read from the Windows registry.
    ///
    /// # Fail-secure contract (D-07)
    ///
    /// This variant is returned when the `HKLM\SOFTWARE\Policies\nono` key is
    /// **present but unreadable** (e.g. `ERROR_ACCESS_DENIED`) **or present but
    /// malformed** (wrong REG_* type, bad UTF-16, unparseable value).
    ///
    /// **Absent** is NOT an error — an absent key returns `Ok(None)` (fall-through
    /// to per-user config).  Only a present-but-broken key aborts here.
    ///
    /// Callers MUST propagate this with `?` and MUST NOT silently fall through to
    /// per-user configuration — that would be a fail-open vulnerability (Pitfall 3,
    /// CLAUDE.md footgun #2).
    #[error("Machine policy load failed: {reason}")]
    PolicyLoadFailed {
        /// Human-readable description of what failed (OS error, type mismatch, etc.).
        reason: String,
    },

    /// One or more files could not be restored (e.g. locked on Windows).
    ///
    /// Carries the list of successfully applied changes along with per-file
    /// failure details so callers can surface exactly which files are stuck
    /// without claiming full rollback success.
    #[error("Partial rollback: {applied} file(s) restored, {failed} file(s) failed: {summary}")]
    PartialRestore {
        /// Number of files successfully restored.
        applied: usize,
        /// Number of files that could not be restored.
        failed: usize,
        /// Human-readable summary of the first few failures.
        summary: String,
    },

    /// Operator intervention required before the operation can proceed.
    ///
    /// Carries structured fields so consumers (CLI, FFI, tests) can branch on
    /// the specific cause without parsing the `Display` string. Maps to C FFI
    /// `NonoErrorCode::ErrConfigParse` (-9) — no new code value is added.
    /// (Phase 36.5 D-36.5-B2 / D-36.5-B3.)
    ///
    /// # Field convention by callsite
    ///
    /// | Callsite                     | `expected`                | `actual`                            | `resolve_via`                                          |
    /// |------------------------------|---------------------------|-------------------------------------|--------------------------------------------------------|
    /// | Base-hash mismatch (promote) | 64-char lowercase hex     | 64-char lowercase hex               | `nono profile init --draft --refresh <name>`           |
    /// | Shadow-refusal (built-in)    | canonical profile path    | draft path                          | multi-line resolution text per D-36.5-D3               |
    /// | Shadow-refusal (pack-managed)| canonical profile path    | draft path                          | multi-line resolution text per D-36.5-D3               |
    /// | Package-status (yanked)      | canonical package ref     | `installed: <ver> (status: yanked)` | multi-line `yanked_message(...)` output                |
    ///
    /// **Security (V7 / T-36.5-07):** `resolve_via` MUST be constructed from
    /// constant strings + resource paths/names ONLY. Never embed an env-var
    /// value, a credential, or a registry URL. The
    /// `action_required_display_does_not_leak_env` test asserts this.
    ///
    /// **Fork divergence:** upstream `829c341a` uses a single-tuple shape
    /// `ActionRequired(String)`; the fork's struct shape (D-36.5-B2) is
    /// pattern-match-friendly and gives the C FFI consumer typed access via
    /// the Display string format.
    #[error("Action required: base-hash mismatch (expected: {expected}; actual: {actual}; resolve via: {resolve_via})")]
    ActionRequired {
        /// Expected resource state (hash, canonical path, or canonical ref).
        expected: String,
        /// Actual observed state.
        actual: String,
        /// Operator-actionable resolution instruction (multi-line for shadow / advisory).
        resolve_via: String,
    },

    // Trust/attestation errors
    #[error("Trust verification failed for {path}: {reason}")]
    TrustVerification { path: String, reason: String },

    #[error("Signing failed for {path}: {reason}")]
    TrustSigning { path: String, reason: String },

    #[error("Trust policy error: {0}")]
    TrustPolicy(String),

    #[error("Blocked by trust policy: {path} matches blocklist entry: {reason}")]
    BlocklistBlocked { path: String, reason: String },

    #[error("Instruction file denied: {path}: {reason}")]
    InstructionFileDenied { path: String, reason: String },

    #[error("Package install error: {0}")]
    PackageInstall(String),

    #[error("Package verification failed for {package}: {reason}")]
    PackageVerification { package: String, reason: String },

    #[error("Registry error: {0}")]
    RegistryError(String),

    // Network errors
    #[error("Per-port network filtering not supported on {platform}: {reason}")]
    NetworkFilterUnsupported { platform: String, reason: String },

    /// Operation cancelled by the user or a pre-condition check. The message
    /// is displayed to the user if non-empty; an empty string means "silent
    /// cancel" (the caller already printed a diagnostic).
    #[error("{0}")]
    Cancelled(String),

    // I/O errors
    #[error("I/O error: {0}")]
    Io(std::io::Error),

    /// A security-event telemetry sink is unavailable (e.g. `RegisterEventSourceW`
    /// returned a null handle, or the Application Event Log source is not registered).
    ///
    /// # Non-fatal contract (D-03)
    ///
    /// This error is surfaced to **stderr** via `warn!()` and does NOT propagate
    /// to the confinement path.  Telemetry is compliance, not an enforcement
    /// control — a failed sink must never block a confined run.  Callers MUST NOT
    /// use `?` to propagate this; use `eprintln!` / `tracing::warn!` and continue.
    #[error("Telemetry sink unavailable: {reason}")]
    TelemetryUnavailable {
        /// Human-readable description of why the sink is unavailable.
        reason: String,
    },

    /// A telemetry configuration value read from the Windows registry was
    /// malformed (wrong type, unparseable, or out of range).
    ///
    /// # Non-fatal contract (D-14)
    ///
    /// Unlike [`PolicyLoadFailed`] (which aborts the run), this error degrades
    /// gracefully to [`TelemetryConfig::default()`].  A typo in a telemetry
    /// REG value must not brick confined runs fleet-wide.  Callers surface this
    /// via `eprintln!` / `tracing::warn!` and continue with safe defaults.
    #[error("Telemetry config invalid: {reason}")]
    TelemetryConfigInvalid {
        /// Human-readable description of the malformed value.
        reason: String,
    },
}

/// Result type alias for nono operations
pub type Result<T> = std::result::Result<T, NonoError>;

impl NonoError {
    /// Map this error to a [`NonoDiagnosticCode`].
    #[must_use]
    pub fn diagnostic_code(&self) -> crate::diagnostic::NonoDiagnosticCode {
        use crate::diagnostic::NonoDiagnosticCode;
        match self {
            Self::CwdPromptRequired => NonoDiagnosticCode::CwdAccessRequired,
            Self::SecretNotFound(_) => NonoDiagnosticCode::CredentialNotFound,
            Self::KeystoreAccess(_) => NonoDiagnosticCode::CredentialUnavailable,
            Self::UnsupportedPlatform(_)
            | Self::NetworkFilterUnsupported { .. }
            | Self::NotSupportedOnPlatform { .. }
            | Self::UnsupportedKernelFeature { .. } => {
                NonoDiagnosticCode::UnsupportedPlatformFeature
            }
            Self::SandboxInit(_) | Self::BlockedCommand { .. } => {
                NonoDiagnosticCode::SandboxDeniedPath
            }
            Self::TrustVerification { .. }
            | Self::TrustSigning { .. }
            | Self::TrustPolicy(_)
            | Self::BlocklistBlocked { .. }
            | Self::InstructionFileDenied { .. }
            | Self::PackageVerification { .. } => NonoDiagnosticCode::TrustVerificationFailed,
            Self::Snapshot(msg) | Self::ObjectStore(msg) if msg.contains("budget exceeded") => {
                NonoDiagnosticCode::RollbackBudgetExceeded
            }
            Self::Cancelled(_) => NonoDiagnosticCode::Cancelled,
            Self::Io(_) | Self::CommandExecution(_) => NonoDiagnosticCode::IoError,
            Self::ConfigParse(_)
            | Self::ConfigWrite { .. }
            | Self::ConfigRead { .. }
            | Self::ProfileNotFound(_)
            | Self::ProfileRead { .. }
            | Self::ProfileParse(_)
            | Self::ProfileInheritance(_)
            | Self::HomeNotFound
            | Self::Setup(_)
            | Self::LearnError(_)
            | Self::HookInstall(_)
            | Self::EnvVarValidation { .. }
            | Self::CapFileValidation { .. }
            | Self::CapFileTooLarge { .. }
            | Self::VersionDowngrade { .. }
            | Self::PackageInstall(_)
            | Self::ActionRequired { .. }
            | Self::RegistryError(_)
            | Self::AttachBusy
            | Self::NoCapabilities
            | Self::NoCommand
            | Self::BrokerNotFound { .. }
            | Self::LabelApplyFailed { .. }
            | Self::DaclApplyFailed { .. }
            | Self::PolicyLoadFailed { .. }
            | Self::TelemetryUnavailable { .. }
            | Self::TelemetryConfigInvalid { .. } => NonoDiagnosticCode::ConfigurationError,
            Self::PathNotFound(_)
            | Self::ExpectedDirectory(_)
            | Self::ExpectedFile(_)
            | Self::PathCanonicalization { .. }
            | Self::HashMismatch { .. }
            | Self::SessionNotFound(_)
            | Self::ObjectStore(_)
            | Self::Snapshot(_)
            | Self::PartialRestore { .. } => NonoDiagnosticCode::Other,
            #[cfg(target_os = "linux")]
            Self::Landlock(_) | Self::LandlockPath(_) => NonoDiagnosticCode::SandboxDeniedPath,
        }
    }

    /// Remediation action when the library can suggest one without CLI context.
    #[must_use]
    pub fn remediation(&self) -> Option<crate::diagnostic::NonoRemediation> {
        use crate::diagnostic::NonoRemediation;
        match self {
            Self::CwdPromptRequired => Some(NonoRemediation::AllowCwd),
            Self::SecretNotFound(_) | Self::KeystoreAccess(_) => {
                Some(NonoRemediation::AuthenticateCredentialProvider {
                    provider: "keystore".to_string(),
                })
            }
            Self::Snapshot(msg) | Self::ObjectStore(msg) if msg.contains("budget exceeded") => {
                Some(NonoRemediation::AdjustRollbackBudget {
                    current_bytes: None,
                    limit_bytes: None,
                })
            }
            Self::Snapshot(msg) | Self::ObjectStore(msg)
                if msg.contains("--no-rollback") || msg.contains("disable rollback") =>
            {
                Some(NonoRemediation::DisableRollback)
            }
            Self::NetworkFilterUnsupported { .. } => Some(NonoRemediation::GrantNetwork),
            Self::ProfileNotFound(_)
            | Self::ProfileParse(_)
            | Self::NoCapabilities
            | Self::ConfigParse(_) => Some(NonoRemediation::CheckPolicy),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn action_required_display_format_base_hash() {
        let err = NonoError::ActionRequired {
            expected: "a".repeat(64),
            actual: "b".repeat(64),
            resolve_via: "nono profile init --draft --refresh myagent".into(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains(&"a".repeat(64)),
            "Display missing expected hash: {msg}"
        );
        assert!(
            msg.contains(&"b".repeat(64)),
            "Display missing actual hash: {msg}"
        );
        assert!(
            msg.contains("nono profile init --draft --refresh myagent"),
            "Display missing resolve_via: {msg}"
        );
        assert!(
            msg.contains("base-hash mismatch"),
            "Display missing 'base-hash mismatch' prefix: {msg}"
        );
    }

    #[test]
    fn action_required_display_does_not_leak_env() {
        let err = NonoError::ActionRequired {
            expected: "p1".into(),
            actual: "p2".into(),
            resolve_via: "do X".into(),
        };
        let msg = err.to_string();
        assert!(
            !msg.contains('$'),
            "Display must not contain '$' (env-var leak): {msg}"
        );
        assert!(
            !msg.contains("%APPDATA%"),
            "Display must not contain '%APPDATA%': {msg}"
        );
        assert!(
            !msg.contains("AKIA"),
            "Display must not contain 'AKIA' (credential prefix): {msg}"
        );
    }

    #[test]
    fn action_required_is_pattern_matchable() {
        let err = NonoError::ActionRequired {
            expected: "x".into(),
            actual: "y".into(),
            resolve_via: "z".into(),
        };
        assert!(
            matches!(err, NonoError::ActionRequired { .. }),
            "ActionRequired must be pattern-matchable via matches! macro"
        );
    }

    #[test]
    fn label_apply_failed_display_includes_path_hresult_and_hint() {
        let err = NonoError::LabelApplyFailed {
            path: PathBuf::from(r"C:\Users\test\.gitconfig"),
            hresult: 5,
            hint: "Ensure the target file is writable by the current user.".into(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains(r"C:\Users\test\.gitconfig"),
            "Display missing path: {msg}"
        );
        assert!(
            msg.contains("0x00000005"),
            "Display missing hex HRESULT: {msg}"
        );
        assert!(
            msg.contains("writable by the current user"),
            "Display missing hint: {msg}"
        );
    }

    #[test]
    fn label_apply_failed_is_propagatable_via_result_alias() {
        fn producer() -> Result<()> {
            Err(NonoError::LabelApplyFailed {
                path: PathBuf::from("/tmp/x"),
                hresult: 0xDEADBEEF,
                hint: "test".into(),
            })
        }
        let err = producer().expect_err("must error");
        assert!(matches!(err, NonoError::LabelApplyFailed { .. }));
    }
}

#[cfg(test)]
mod broker_not_found_tests {
    use super::NonoError;
    use std::path::PathBuf;

    /// Phase 31 D-07: BrokerNotFound display surfaces the resolved path so
    /// operators can see exactly which sibling lookup failed.
    #[test]
    fn broker_not_found_displays_path() {
        let err = NonoError::BrokerNotFound {
            path: PathBuf::from("/tmp/missing-broker.exe"),
        };
        let s = err.to_string();
        assert!(
            s.contains("missing-broker.exe"),
            "BrokerNotFound display should include the path; got: {s}"
        );
    }

    /// Phase 31 D-07: BrokerNotFound carries Debug derivation through
    /// `#[derive(Error, Debug)]` on NonoError. Smoke check that
    /// formatting the error via `{err:?}` does not panic.
    #[test]
    fn broker_not_found_is_debug() {
        let err = NonoError::BrokerNotFound {
            path: PathBuf::from("foo.exe"),
        };
        let _ = format!("{err:?}");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod unsupported_kernel_feature_tests {
    use super::NonoError;

    // Phase 44 WR-02 P37 (D-44-A4): refer to the now-promoted module-level
    // CGROUP_V2_HINT instead of duplicating the literal locally.
    const LOCKED_HINT: &str = super::CGROUP_V2_HINT;

    #[test]
    fn unsupported_kernel_feature_display_contains_cgroup_no_v1_hint() {
        let err = NonoError::UnsupportedKernelFeature {
            feature: "cgroup_v2".into(),
            hint: LOCKED_HINT.into(),
        };
        let s = err.to_string();
        assert!(
            s.starts_with("Kernel feature not supported:"),
            "Display must start with the Phase 37 D-05 prefix; got: {s}"
        );
        assert!(
            s.contains("cgroup_v2"),
            "Display must contain the feature id; got: {s}"
        );
        assert!(
            s.contains("cgroup_no_v1=all"),
            "Display must contain the LOCKED D-07 boot-flag hint substring; got: {s}"
        );
    }

    #[test]
    fn unsupported_kernel_feature_is_pattern_matchable() {
        let err = NonoError::UnsupportedKernelFeature {
            feature: "cgroup_v2".into(),
            hint: LOCKED_HINT.into(),
        };
        assert!(matches!(err, NonoError::UnsupportedKernelFeature { .. }));
    }

    #[test]
    fn unsupported_kernel_feature_is_debug() {
        let err = NonoError::UnsupportedKernelFeature {
            feature: "cgroup_v2".into(),
            hint: LOCKED_HINT.into(),
        };
        let _ = format!("{err:?}");
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::{NonoError, Result};
    use crate::diagnostic::{NonoDiagnosticCode, NonoRemediation};

    #[test]
    fn cwd_prompt_maps_to_structured_code_and_remediation() {
        let err = NonoError::CwdPromptRequired;
        assert_eq!(err.diagnostic_code(), NonoDiagnosticCode::CwdAccessRequired);
        assert_eq!(err.remediation(), Some(NonoRemediation::AllowCwd));
    }

    #[test]
    fn secret_not_found_maps_to_credential_not_found() {
        let err = NonoError::SecretNotFound("missing".to_string());
        assert_eq!(
            err.diagnostic_code(),
            NonoDiagnosticCode::CredentialNotFound
        );
        assert!(matches!(
            err.remediation(),
            Some(NonoRemediation::AuthenticateCredentialProvider { .. })
        ));
    }

    #[test]
    fn rollback_budget_error_maps_to_structured_code() -> Result<()> {
        let err = NonoError::Snapshot(
            "Rollback budget exceeded: 10 bytes tracked (limit: 5 bytes). \
             or disable rollback with --no-rollback."
                .to_string(),
        );
        assert_eq!(
            err.diagnostic_code(),
            NonoDiagnosticCode::RollbackBudgetExceeded
        );
        assert!(matches!(
            err.remediation(),
            Some(NonoRemediation::AdjustRollbackBudget { .. })
        ));
        Ok(())
    }
}
