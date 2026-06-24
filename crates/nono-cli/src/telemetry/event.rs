//! Security event schema for SIEM/EDR telemetry (Phase 84).
//!
//! Defines the [`SecurityEvent`] struct and supporting types used by
//! [`super::SecurityEventLayer`] to emit structured security events to Windows
//! telemetry sinks (ETW + Application Event Log) and, in a future cycle, to
//! RFC 5424 syslog.
//!
//! # Field contract (D-10 / D-11)
//!
//! - `path_hash` — salted SHA-256 of the canonical path (D-08), **never** the
//!   raw path string.
//! - `host` — cleartext (SC-1 exception: analysts need the denied domain).
//! - All other free-text fields (reason, label, hook_name) are run through
//!   [`nono::scrub_value`] before they reach this struct.
//!
//! # EventID map (SC-1, locked in ROADMAP)
//!
//! | EventID | Variant                      |
//! |---------|------------------------------|
//! | 10001   | `PathDeny`                   |
//! | 10002   | `NetworkDeny`                |
//! | 10003   | `LabelViolation`             |
//! | 10004   | `HookFailClosed`             |
//! | 10005   | `TelemetryDegraded`          |
//! | 10006   | `PolicyOverridePresented`    |
//! | 10007   | `PolicyOverrideVerified`     |
//! | 10008   | `PolicyOverrideRejected`     |
//! | 10009   | `PolicyOverrideExpired`      |
//! | 10010   | `PolicyOverrideRevoked`      |

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

// ── EventID constants (locked by ROADMAP SC-1) ────────────────────────────────

/// EventID for a file-system path-deny event.
pub const EVENT_ID_PATH_DENY: u32 = 10001;
/// EventID for a network-egress-deny event.
pub const EVENT_ID_NETWORK_DENY: u32 = 10002;
/// EventID for a mandatory-integrity label violation.
pub const EVENT_ID_LABEL_VIOLATION: u32 = 10003;
/// EventID for a hook fail-closed event.
pub const EVENT_ID_HOOK_FAIL_CLOSED: u32 = 10004;
/// EventID for a telemetry-degraded self-describing event (D-14).
pub const EVENT_ID_TELEMETRY_DEGRADED: u32 = 10005;
/// EventID for a policy-override token presented (lifecycle start).
pub const EVENT_ID_POLICY_OVERRIDE_PRESENTED: u32 = 10006;
/// EventID for a policy-override token verified and applied.
pub const EVENT_ID_POLICY_OVERRIDE_VERIFIED: u32 = 10007;
/// EventID for a policy-override token rejected (any `OverrideErrorKind`).
pub const EVENT_ID_POLICY_OVERRIDE_REJECTED: u32 = 10008;
/// EventID for a policy-override token rejected because it was expired.
pub const EVENT_ID_POLICY_OVERRIDE_EXPIRED: u32 = 10009;
/// EventID for a policy-override token rejected due to replay (jti already consumed).
pub const EVENT_ID_POLICY_OVERRIDE_REVOKED: u32 = 10010;

// ── SecurityEventType ─────────────────────────────────────────────────────────

/// The type of security event being emitted.
///
/// Mapped to distinct EventIDs 10001–10005 by [`event_id_for`] (SC-1, locked
/// in ROADMAP).  The serde `rename_all = "snake_case"` representation is used
/// in the JSON payload written to the Application Event Log (D-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    /// File-system path access denied by the sandbox (EventID 10001).
    PathDeny,
    /// Outbound network connection denied by the proxy or WFP filter (EventID 10002).
    NetworkDeny,
    /// Mandatory-integrity label violation (EventID 10003).
    LabelViolation,
    /// Pre-tool-use hook returned a non-zero exit code → fail-closed (EventID 10004).
    HookFailClosed,
    /// Telemetry sub-system degraded to defaults (D-14 self-describing event,
    /// EventID 10005).
    TelemetryDegraded,
    // ── Phase 92: policy-override lifecycle events (EventIDs 10006–10010) ─────
    /// Signed override token presented for verification (EventID 10006).
    PolicyOverridePresented,
    /// Signed override token verified and applied (EventID 10007).
    PolicyOverrideVerified,
    /// Signed override token rejected (EventID 10008 — maps from OverrideErrorKind).
    PolicyOverrideRejected,
    /// Signed override token rejected as expired (EventID 10009).
    PolicyOverrideExpired,
    /// Signed override token rejected as already-consumed/revoked (EventID 10010).
    PolicyOverrideRevoked,
}

/// Return the Windows Application Event Log EventID for a [`SecurityEventType`].
///
/// EventIDs 10001–10005 are **locked by ROADMAP SC-1** and must not change
/// without a ROADMAP update.
#[must_use]
pub fn event_id_for(t: &SecurityEventType) -> u32 {
    match t {
        SecurityEventType::PathDeny => EVENT_ID_PATH_DENY,
        SecurityEventType::NetworkDeny => EVENT_ID_NETWORK_DENY,
        SecurityEventType::LabelViolation => EVENT_ID_LABEL_VIOLATION,
        SecurityEventType::HookFailClosed => EVENT_ID_HOOK_FAIL_CLOSED,
        SecurityEventType::TelemetryDegraded => EVENT_ID_TELEMETRY_DEGRADED,
        // Phase 92 additions:
        SecurityEventType::PolicyOverridePresented => EVENT_ID_POLICY_OVERRIDE_PRESENTED,
        SecurityEventType::PolicyOverrideVerified => EVENT_ID_POLICY_OVERRIDE_VERIFIED,
        SecurityEventType::PolicyOverrideRejected => EVENT_ID_POLICY_OVERRIDE_REJECTED,
        SecurityEventType::PolicyOverrideExpired => EVENT_ID_POLICY_OVERRIDE_EXPIRED,
        SecurityEventType::PolicyOverrideRevoked => EVENT_ID_POLICY_OVERRIDE_REVOKED,
    }
}

// ── PathCategory ──────────────────────────────────────────────────────────────

/// Sensitivity tier for the path involved in a security event (D-09).
///
/// Replaces the raw path in the event payload.  Analysts see "a credential
/// path was accessed" without the literal `C:\Users\alice\.ssh\id_ed25519`.
/// The serde `rename_all = "snake_case"` representation matches the
/// Application Event Log JSON payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathCategory {
    /// Source/project files in the agent's working area.
    WorkspaceFile,
    /// Operating-system paths (Windows/system32, /etc, /usr, …).
    SystemPath,
    /// High-sensitivity credential paths (`.ssh`, `.aws`, keystore directories).
    CredentialPath,
    /// Files under the user's home directory (outside other categories).
    UserHome,
    /// Temporary directories (`%TEMP%`, `/tmp`, …).
    Temp,
    /// Paths that do not match any other tier.
    Other,
}

/// Classify a canonical path into a [`PathCategory`] sensitivity tier (D-09).
///
/// Uses **path component comparison** (not string operations) per CLAUDE.md
/// §Path Handling to avoid the `/homeevil` footgun.
///
/// # Priority order (highest wins)
///
/// 1. Any component matches `.ssh`, `.aws`, or `keystore` → `CredentialPath`
///    (checked first across ALL components before lower-priority categories)
/// 2. Any component matches `temp` or `tmp` → `Temp`
/// 3. Any component matches `windows`, `system32`, `system64`, `etc`, `usr`,
///    `bin`, `lib`, `sysroot` → `SystemPath`
/// 4. Path starts with the current user's home directory → `UserHome`
/// 5. Fallback → `WorkspaceFile`
///
/// The multi-pass approach ensures that a path like `/var/lib/keystore/token`
/// is classified as `CredentialPath` even though `lib` appears first.
#[must_use]
pub fn classify_path(path: &Path) -> PathCategory {
    // Pass 1: Credential paths — absolute highest priority.
    // A `keystore` directory under `/var/lib/` must still be CredentialPath.
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        let lower = s.to_lowercase();
        if lower == ".ssh" || lower == ".aws" || lower == "keystore" {
            return PathCategory::CredentialPath;
        }
    }

    // Pass 2: Temp directories — higher priority than SystemPath (system /tmp).
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        let lower = s.to_lowercase();
        if lower == "temp" || lower == "tmp" {
            return PathCategory::Temp;
        }
    }

    // Pass 3: System paths.
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        let lower = s.to_lowercase();
        if matches!(
            lower.as_str(),
            "windows" | "system32" | "system64" | "etc" | "usr" | "bin" | "lib" | "sysroot"
        ) {
            return PathCategory::SystemPath;
        }
    }

    // Pass 4: Home directory check — use dirs::home_dir() for portability.
    if let Some(home) = dirs::home_dir() {
        if path.starts_with(&home) {
            return PathCategory::UserHome;
        }
    }

    PathCategory::WorkspaceFile
}

// ── PathHash ─────────────────────────────────────────────────────────────────

/// Compute a per-session salted path hash (D-08 / SC-3).
///
/// `PathHash = hex(SHA-256(session_salt || canonical_path_bytes)[0..16])`
///
/// Properties:
/// - **Deterministic within a session**: the same path hashes the same way, so
///   analysts can correlate repeated denials on one file.
/// - **Cross-session opaque**: the per-session salt prevents rainbow tables.
/// - **Raw path never appears** in the returned string (SC-3 gate).
///
/// # Arguments
///
/// - `salt` — the 32-byte per-session salt (the same entropy used for the
///   HMAC chain key; both derived at `SecurityEventLayer` construction time).
/// - `canonical_path` — a canonicalized [`Path`].
///
/// # Returns
///
/// A 32-character lowercase hex string (16 bytes = 128 bits of hash output).
#[must_use]
pub fn path_hash_for(salt: &[u8], canonical_path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(canonical_path.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    // Take the first 16 bytes → 32 hex chars (SC-3 truncated hash).
    let truncated = &digest[..16];
    truncated
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

// ── SecurityEvent ─────────────────────────────────────────────────────────────

/// A structured security event emitted by [`super::SecurityEventLayer`].
///
/// This struct is the canonical payload for both the ETW trace and the
/// Application Event Log JSON body (D-01 / D-02).  Field names use
/// `PascalCase` via `serde(rename_all = "PascalCase")` to match the SC-1
/// named EventData columns that Splunk `spath` and Sentinel `parse_json`
/// extract without a custom parser.
///
/// # Security properties
///
/// - `path_hash`: salted SHA-256, **never** the raw path (D-08 / SC-3).
/// - `path_category`: sensitivity tier label, **never** the raw path (D-09).
/// - `host`: cleartext (D-10 exception — SC-1 requires parseable denied domain).
/// - All free-text fields passed to construction are scrubbed via
///   [`nono::scrub_value`] before being stored here.
/// - `chain_head`: hex of the current HMAC chain head (D-05 / TELEM-02).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SecurityEvent {
    /// Type of security event (maps to EventID via [`event_id_for`]).
    pub event_type: SecurityEventType,
    /// PID of the confined agent process.
    pub agent_pid: u32,
    /// Salted SHA-256 of the canonical path (D-08).  `None` for network events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_hash: Option<String>,
    /// Sensitivity tier of the path (D-09).  `None` for network events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_category: Option<PathCategory>,
    /// Cleartext denied destination host/domain (D-10 / SC-1).
    /// `None` for path-deny and label events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Per-session opaque identifier (correlates events within one run).
    pub session_id: String,
    /// Hex of the current HMAC chain head after this event (D-05 / TELEM-02).
    pub chain_head: String,
    /// Unix timestamp in milliseconds (UTC).
    pub timestamp_unix_ms: u64,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── event_id_for ──────────────────────────────────────────────────────────

    #[test]
    fn event_id_for_maps_all_five_types() {
        assert_eq!(event_id_for(&SecurityEventType::PathDeny), 10001);
        assert_eq!(event_id_for(&SecurityEventType::NetworkDeny), 10002);
        assert_eq!(event_id_for(&SecurityEventType::LabelViolation), 10003);
        assert_eq!(event_id_for(&SecurityEventType::HookFailClosed), 10004);
        assert_eq!(event_id_for(&SecurityEventType::TelemetryDegraded), 10005);
    }

    // ── Phase 92: override EventID mapping (AUD-03) ───────────────────────────

    #[test]
    fn override_event_ids_are_10006_through_10010() {
        assert_eq!(
            event_id_for(&SecurityEventType::PolicyOverridePresented),
            10006
        );
        assert_eq!(
            event_id_for(&SecurityEventType::PolicyOverrideVerified),
            10007
        );
        assert_eq!(
            event_id_for(&SecurityEventType::PolicyOverrideRejected),
            10008
        );
        assert_eq!(
            event_id_for(&SecurityEventType::PolicyOverrideExpired),
            10009
        );
        assert_eq!(
            event_id_for(&SecurityEventType::PolicyOverrideRevoked),
            10010
        );
    }

    #[test]
    fn override_event_type_serde_roundtrip() {
        let t = SecurityEventType::PolicyOverrideVerified;
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            json.contains("policy_override_verified"),
            "serde rename_all=snake_case must produce policy_override_verified; got: {json}"
        );
        let restored: SecurityEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(
            t, restored,
            "round-trip must recover PolicyOverrideVerified"
        );
    }

    // ── classify_path ─────────────────────────────────────────────────────────

    #[test]
    fn classify_ssh_path_is_credential() {
        let path = std::path::Path::new("/home/user/.ssh/id_ed25519");
        assert_eq!(classify_path(path), PathCategory::CredentialPath);
    }

    #[test]
    fn classify_aws_path_is_credential() {
        // Build from components so `.aws` is a distinct path component on every
        // platform (a `\`-separated literal is a single component on Unix, which
        // makes the classifier — correctly — never see `.aws`).
        let path: std::path::PathBuf = ["/", "home", "alice", ".aws", "credentials"]
            .iter()
            .collect();
        assert_eq!(classify_path(&path), PathCategory::CredentialPath);
    }

    #[test]
    fn classify_keystore_path_is_credential() {
        let path = std::path::Path::new("/var/lib/keystore/tokens.db");
        assert_eq!(classify_path(path), PathCategory::CredentialPath);
    }

    #[test]
    fn classify_system32_is_system_path() {
        // Component-built path so `system32` is a distinct component on Unix CI
        // too (a `\`-separated literal would be a single opaque component there).
        let path: std::path::PathBuf = ["/", "Windows", "system32", "ntdll.dll"].iter().collect();
        assert_eq!(classify_path(&path), PathCategory::SystemPath);
    }

    #[test]
    fn classify_etc_is_system_path() {
        let path = std::path::Path::new("/etc/hosts");
        assert_eq!(classify_path(path), PathCategory::SystemPath);
    }

    #[test]
    fn classify_temp_is_temp() {
        // Component-built path so `Temp` is a distinct component on Unix CI too
        // (a `\`-separated literal would be a single opaque component there).
        let path: std::path::PathBuf =
            ["/", "Users", "alice", "AppData", "Local", "Temp", "foo.tmp"]
                .iter()
                .collect();
        assert_eq!(classify_path(&path), PathCategory::Temp);
    }

    #[test]
    fn classify_tmp_is_temp() {
        let path = std::path::Path::new("/tmp/build-artifact.tar.gz");
        assert_eq!(classify_path(path), PathCategory::Temp);
    }

    #[test]
    fn classify_project_file_is_workspace() {
        // A path that doesn't hit any higher-priority category.
        let path = std::path::Path::new(r"C:\projects\nono\src\main.rs");
        // Doesn't contain .ssh/.aws/keystore/system32/windows/etc/tmp
        // and doesn't start with home dir on most test hosts.
        // May be WorkspaceFile or UserHome depending on home dir; just confirm it's not Temp/Credential.
        let cat = classify_path(path);
        assert_ne!(cat, PathCategory::CredentialPath);
        assert_ne!(cat, PathCategory::SystemPath);
        assert_ne!(cat, PathCategory::Temp);
    }

    // ── path_hash_for ─────────────────────────────────────────────────────────

    #[test]
    fn path_hash_for_is_deterministic() {
        let salt = [0xABu8; 32];
        let path = std::path::Path::new(r"C:\projects\nono\secret.txt");
        let h1 = path_hash_for(&salt, path);
        let h2 = path_hash_for(&salt, path);
        assert_eq!(
            h1, h2,
            "path_hash_for must be deterministic for same inputs"
        );
    }

    #[test]
    fn path_hash_for_does_not_contain_raw_path() {
        let salt = [0x11u8; 32];
        let path = std::path::Path::new(r"C:\Users\alice\secret\file.txt");
        let hash = path_hash_for(&salt, path);
        // SC-3 baseline: the hash output must NOT contain the raw path string.
        assert!(
            !hash.contains("alice"),
            "path hash must not contain raw path; hash={hash}"
        );
        assert!(
            !hash.contains("secret"),
            "path hash must not contain raw path component; hash={hash}"
        );
        // The hash is a 32-char lowercase hex string.
        assert_eq!(hash.len(), 32, "hash must be 32 hex chars (16 bytes)");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash must be hex; got: {hash}"
        );
    }

    #[test]
    fn path_hash_differs_for_different_paths() {
        let salt = [0x22u8; 32];
        let p1 = std::path::Path::new(r"C:\foo\bar.txt");
        let p2 = std::path::Path::new(r"C:\foo\baz.txt");
        assert_ne!(
            path_hash_for(&salt, p1),
            path_hash_for(&salt, p2),
            "different paths must produce different hashes"
        );
    }

    #[test]
    fn path_hash_differs_for_different_salts() {
        let path = std::path::Path::new(r"C:\same\path.txt");
        let h1 = path_hash_for(&[0x00u8; 32], path);
        let h2 = path_hash_for(&[0xFFu8; 32], path);
        assert_ne!(
            h1, h2,
            "different salts must produce different hashes for the same path"
        );
    }

    // ── SecurityEvent serialization ───────────────────────────────────────────

    #[test]
    fn security_event_serializes_with_pascal_case_sc1_fields() {
        let event = SecurityEvent {
            event_type: SecurityEventType::PathDeny,
            agent_pid: 1234,
            path_hash: Some("aabbccdd00112233".to_string()),
            path_category: Some(PathCategory::WorkspaceFile),
            host: None,
            session_id: "sess-abc".to_string(),
            chain_head: "00112233445566778899aabbccddeeff".to_string(),
            timestamp_unix_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&event).unwrap();
        // SC-1 named fields must appear in PascalCase.
        assert!(json.contains("\"EventType\""), "missing EventType: {json}");
        assert!(json.contains("\"AgentPid\""), "missing AgentPid: {json}");
        assert!(json.contains("\"PathHash\""), "missing PathHash: {json}");
        assert!(json.contains("\"SessionId\""), "missing SessionId: {json}");
        assert!(json.contains("\"ChainHead\""), "missing ChainHead: {json}");
        // Host is None/omitted in this test.
        // The path_hash value must not be the raw path.
        assert!(
            !json.contains("secret") && !json.contains("Users"),
            "raw path component leaked into JSON: {json}"
        );
    }

    #[test]
    fn security_event_type_serde_round_trip() {
        for t in [
            SecurityEventType::PathDeny,
            SecurityEventType::NetworkDeny,
            SecurityEventType::LabelViolation,
            SecurityEventType::HookFailClosed,
            SecurityEventType::TelemetryDegraded,
        ] {
            let json = serde_json::to_string(&t).unwrap();
            let restored: SecurityEventType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, restored, "serde round-trip failed for {t:?}");
        }
    }
}
