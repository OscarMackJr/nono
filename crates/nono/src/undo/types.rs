//! Core types for the undo/snapshot system
//!
//! Defines content hashes, file state, change tracking, and session metadata
//! used by the object store and snapshot manager.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

/// A SHA-256 content hash (32 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Create a ContentHash from raw bytes
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Return the first 2 hex characters (used for object store directory sharding)
    #[must_use]
    pub fn prefix(&self) -> String {
        format!("{:02x}", self.0[0])
    }

    /// Return the remaining hex characters after the prefix
    #[must_use]
    pub fn suffix(&self) -> String {
        let mut s = String::with_capacity(62);
        for byte in &self.0[1..] {
            s.push_str(&format!("{byte:02x}"));
        }
        s
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", self)
    }
}

impl FromStr for ContentHash {
    type Err = ContentHashParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.len() != 64 {
            return Err(ContentHashParseError::InvalidLength(s.len()));
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hex_str =
                std::str::from_utf8(chunk).map_err(|_| ContentHashParseError::InvalidHex)?;
            bytes[i] =
                u8::from_str_radix(hex_str, 16).map_err(|_| ContentHashParseError::InvalidHex)?;
        }
        Ok(Self(bytes))
    }
}

/// Error parsing a ContentHash from a hex string
#[derive(Debug, Clone)]
pub enum ContentHashParseError {
    /// Hex string was not 64 characters
    InvalidLength(usize),
    /// Hex string contained invalid characters
    InvalidHex,
}

impl fmt::Display for ContentHashParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(len) => {
                write!(f, "expected 64 hex characters, got {len}")
            }
            Self::InvalidHex => write!(f, "invalid hex character"),
        }
    }
}

impl std::error::Error for ContentHashParseError {}

impl Serialize for ContentHash {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// State of a single file at snapshot time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// SHA-256 hash of file content
    pub hash: ContentHash,
    /// File size in bytes
    pub size: u64,
    /// Last modification time (seconds since epoch)
    pub mtime: i64,
    /// File permissions (Unix mode bits)
    pub permissions: u32,
}

/// Type of change detected between snapshots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// File was created (not in previous snapshot)
    Created,
    /// File content was modified
    Modified,
    /// File was deleted (in previous snapshot but not current)
    Deleted,
    /// Only file permissions changed
    PermissionsChanged,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "+"),
            Self::Modified => write!(f, "~"),
            Self::Deleted => write!(f, "-"),
            Self::PermissionsChanged => write!(f, "p"),
        }
    }
}

/// A change detected between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Path to the changed file
    pub path: PathBuf,
    /// Type of change
    pub change_type: ChangeType,
    /// Size delta in bytes (positive = grew, negative = shrank)
    pub size_delta: Option<i64>,
    /// Hash before the change (None for Created)
    pub old_hash: Option<ContentHash>,
    /// Hash after the change (None for Deleted)
    pub new_hash: Option<ContentHash>,
}

/// Proxy mode used for network audit events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditMode {
    /// CONNECT tunnel request
    Connect,
    /// Reverse proxy request
    Reverse,
    /// External proxy passthrough request
    External,
}

/// Decision outcome for a network audit event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditDecision {
    /// Request was allowed
    Allow,
    /// Request was denied
    Deny,
}

/// Authentication mechanism used at the proxy boundary.
///
/// Structured context added by upstream `9300de9` (v0.51.0). Fork-side D-20
/// manual replay against the Phase 22-05a + Phase 23 REQ-AUD-05 audit envelope:
/// extending `NetworkAuditEvent` with optional structured-context fields is
/// additive — the integrity-protection / merkle / chain-head invariants on the
/// envelope are unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditAuthMechanism {
    /// `Proxy-Authorization` on CONNECT or reverse-proxy fallback auth
    ProxyAuthorization,
    /// Phantom token carried in an HTTP header
    PhantomHeader,
    /// Phantom token carried in the URL path
    PhantomPath,
    /// Phantom token carried in a query parameter
    PhantomQuery,
    /// SPIFFE JWT-SVID presented as a bearer token
    SpiffeJwtBearer,
    /// OAuth2 jwt-bearer assertion (RFC 7523) signed with a SPIFFE JWT-SVID
    SpiffeOAuthAssertion,
}

/// Outcome of proxy-side authentication or phantom-token validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditAuthOutcome {
    /// Validation succeeded
    Succeeded,
    /// Validation failed
    Failed,
}

/// Injection mode used when the proxy supplies an upstream credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditInjectionMode {
    Header,
    UrlPath,
    QueryParam,
    BasicAuth,
    OAuth2,
    /// SPIFFE JWT-SVID injected as a bearer token
    SpiffeJwt,
}

/// Structured category for denied proxy events.
///
/// D-09 (Phase 115-02, DRAIN-03/NEW-01): `InterceptHandshakeFailed` was
/// REMOVED (not reserved) from this enum. It had zero production
/// constructors anywhere in the workspace and can never gain one: the fork
/// declines TLS interception by standing decision (ADR-113 D-01;
/// `ProxyHandle::intercept_ca_path()` always returns `None`). A variant
/// advertising an enforcement point the fork structurally does not have is
/// the same over-claiming shape BOUND-01/BOUND-02 exist to close.
/// **Precondition discharged, not assumed:** 115-RESEARCH.md Q1 confirmed
/// zero persisted-ledger risk — no ledger, fixture, or golden anywhere in
/// this repo contains the string `intercept_handshake_failed`, and the
/// decode path (`serde_json::from_str::<AuditEventRecord>`,
/// `crates/nono/src/audit.rs`) has no `#[serde(other)]` tolerance, so no
/// historic data could ever have round-tripped this variant. No
/// unknown-variant escape route was needed as a result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditDenialCategory {
    AuthenticationFailed,
    EndpointPolicy,
    ManagedCredentialUnavailable,
    HostDenied,
    UpstreamConnectFailed,
    ConnectBypassesL7,
    ExternalProxyRejected,
    /// Route declared SPIFFE-authenticated but arrived on a proxy path with
    /// no SPIFFE implementation (D-03 fail-closed guard; the guard itself
    /// lands in Plan 113-05 — this variant is added ahead of need so that
    /// plan does not require a second edit to this enum).
    SpiffeUnsupportedPath,
    /// Route declared OAuth-capture (response buffer-and-rewrite) but
    /// arrived on a proxy path with no rewrite implementation — CONNECT,
    /// forward-HTTP, or the external-proxy chain (D-06 fail-closed guard;
    /// the guard itself lands in Plan 114-07 — this variant is added ahead
    /// of need so that plan does not require a second edit to this enum).
    CaptureUnsupportedPath,
    /// Route declared OAuth-capture and arrived on `relay_response_with_capture`
    /// (the buffer-and-rewrite enforcement point itself, Plan 114-05), but
    /// the response was denied there: buffer cap exceeded (D-05),
    /// `Content-Encoding` present (Pitfall 2 / `3c59c62e`), a malformed or
    /// unparseable body, or an unconfigured token-shaped field surviving
    /// rewrite (the `reject_unrewritten_token_fields` fail-closed backstop,
    /// D-07). Distinct from `CaptureUnsupportedPath`, which denies a
    /// capture-declared route for arriving on the WRONG path (no rewrite
    /// implementation at all); this variant denies on the RIGHT path
    /// because the buffer/parse/rewrite step itself failed closed.
    CaptureBufferOrRewriteFailed,
}

impl NetworkAuditDenialCategory {
    /// Every variant of this enum, in declaration order.
    ///
    /// D-11 (Phase 115-02, DRAIN-02): self-enumerating so the DRAIN-02
    /// round-trip test (Plan 115-05, `../nono-py`) does not need its own
    /// hand-written variant list — a hand-maintained list drifting from its
    /// source of truth is exactly the class DRAIN-02/NEW-06 exist to close.
    ///
    /// **D-11 dependency choice, decided here:** the zero-new-dependency
    /// fallback (`const ALL` + `assert_all_variants_covered` below), not
    /// `strum::EnumIter`. `strum`/`strum_macros` are completely absent from
    /// this workspace today (zero hits in the root `Cargo.lock` and every
    /// workspace member `Cargo.toml` — verified, 115-RESEARCH.md Q3), so
    /// adding it is a genuinely new dependency, not a feature-flag on
    /// something already resolved. `thiserror` (`crates/nono/Cargo.toml`,
    /// `thiserror.workspace = true`) is direct, load-bearing precedent that
    /// a derive-only macro dependency in this core crate does not cross
    /// ADR-86's policy-free-library boundary — ADR-86 constrains what the
    /// library *decides* (security policy), not what it *derives* — so
    /// `strum` would not have been contentious either. But the fallback
    /// achieves the identical "fails to compile on a missing variant"
    /// guarantee with zero new dependency surface and no
    /// package-legitimacy review needed: strictly cheaper on pure
    /// dependency-surface grounds with equal structural strength,
    /// consistent with this phase's governing "make it unrepresentable at
    /// the lowest cost" rule.
    // Used only by this module's own `#[cfg(test)]` guard test today
    // (`all_denial_categories_present_and_guard_covers_every_entry`) — the
    // non-test `lib` target has no other production consumer yet, which
    // rustc's per-target dead_code analysis flags. Precedent for this exact
    // targeted allow: `crates/nono-cli/src/policy.rs`'s
    // `expand_egress_preset_tokens`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: &'static [NetworkAuditDenialCategory] = &[
        NetworkAuditDenialCategory::AuthenticationFailed,
        NetworkAuditDenialCategory::EndpointPolicy,
        NetworkAuditDenialCategory::ManagedCredentialUnavailable,
        NetworkAuditDenialCategory::HostDenied,
        NetworkAuditDenialCategory::UpstreamConnectFailed,
        NetworkAuditDenialCategory::ConnectBypassesL7,
        NetworkAuditDenialCategory::ExternalProxyRejected,
        NetworkAuditDenialCategory::SpiffeUnsupportedPath,
        NetworkAuditDenialCategory::CaptureUnsupportedPath,
        NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed,
    ];
}

// IMPORTANT: match is exhaustive (no wildcard arm) so the compiler forces
// handling of every current and future NetworkAuditDenialCategory variant —
// mirrors `reverse.rs`'s `EndpointPolicyOutcome` "exhaustive... forces the
// compiler" guard style. Adding a variant to the enum above without adding
// a corresponding arm here fails to compile with `error[E0004]:
// non-exhaustive patterns: NetworkAuditDenialCategory::<Variant> not
// covered`. This is `ALL`'s own compile-time drift guard: forgetting to
// keep this match (and `ALL`, alongside it) in sync with the enum is caught
// at build time, not left for a runtime test to discover.
//
// Counterexample (do not uncomment — kept for reviewers, live-verified
// during Task 3 of 115-02-PLAN.md and reverted after confirming the
// failure): adding `NewVariant,` to `NetworkAuditDenialCategory` above
// without adding `NetworkAuditDenialCategory::NewVariant => {}` to the
// match below fails `cargo build -p nono-sandbox` with the E0004 error
// shown above.
#[cfg_attr(not(test), allow(dead_code))]
fn assert_all_variants_covered(category: &NetworkAuditDenialCategory) {
    match category {
        NetworkAuditDenialCategory::AuthenticationFailed
        | NetworkAuditDenialCategory::EndpointPolicy
        | NetworkAuditDenialCategory::ManagedCredentialUnavailable
        | NetworkAuditDenialCategory::HostDenied
        | NetworkAuditDenialCategory::UpstreamConnectFailed
        | NetworkAuditDenialCategory::ConnectBypassesL7
        | NetworkAuditDenialCategory::ExternalProxyRejected
        | NetworkAuditDenialCategory::SpiffeUnsupportedPath
        | NetworkAuditDenialCategory::CaptureUnsupportedPath
        | NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed => {}
    }
}

/// SPIFFE delegation-chain context recovered from a JWT-SVID's `act` claim.
///
/// Pure data — records what the token asserted, applies no policy. See
/// `crate::undo::SpiffeAuditContext` for the parent audit record this nests
/// inside.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiffeDelegationContext {
    /// SPIFFE ID of the workload that authorized this delegation (the `act.sub` claim)
    pub authorized_by: String,
    /// SPIFFE ID of the workload the token was issued on behalf of, when present
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,
    /// Number of nested `act` claims observed (delegation-chain depth)
    pub chain_depth: u32,
}

/// SPIFFE audit context attached to a network audit event when a request
/// used SPIFFE/SPIRE workload-identity auth. Threaded into
/// `NetworkAuditEvent::spiffe_context`.
///
/// Pure data — records what happened (workload identity, trust domain,
/// delegation chain), applies no enforcement or policy evaluation. See
/// ADR-86 / D-08 (113-CONTEXT.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiffeAuditContext {
    /// SPIFFE ID of the workload that presented the credential
    pub workload_spiffe_id: String,
    /// Trust domain portion of `workload_spiffe_id`
    pub trust_domain: String,
    /// SVID type (currently always "jwt")
    pub svid_type: String,
    /// Where the credential was obtained (for example, "spire-workload-api")
    pub source: String,
    /// SPIFFE ID of the upstream service the credential was presented to, when known
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_spiffe_id: Option<String>,
    /// Delegation-chain context recovered from the token's `act` claim, when present
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<SpiffeDelegationContext>,
}

/// OAuth-capture audit context attached to a network audit event when a
/// response-buffer-and-rewrite route rewrote captured token fields to
/// phantoms. Threaded into `NetworkAuditEvent::capture_context`.
///
/// Pure data — records what happened (which route, which fields were
/// rewritten, which phantom IDs were minted), applies no enforcement or
/// policy evaluation. Structurally cannot carry a raw token: every field is
/// an identifier (route name, dot-path, opaque phantom ID), never secret
/// bytes. See ADR-86 / D-08 / D-09 (114-CONTEXT.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureAuditContext {
    /// Configured custom-credential/route name that declared capture
    pub route_id: String,
    /// Dot-paths in the response body that were rewritten to phantoms
    pub rewritten_fields: Vec<String>,
    /// Opaque phantom identifiers minted for this response, one per
    /// rewritten field, same order/length as `rewritten_fields`
    pub phantom_ids: Vec<String>,
}

/// A single network audit event captured by the proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAuditEvent {
    /// Event timestamp in Unix milliseconds
    pub timestamp_unix_ms: u64,
    /// Proxy mode handling the request
    pub mode: NetworkAuditMode,
    /// Allow or deny decision
    pub decision: NetworkAuditDecision,
    /// Stable configured route identifier when the request was associated
    /// with a proxy route (for example, `openai`); None for opaque traffic
    /// with no route identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    /// Authentication mechanism used at the proxy boundary, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_mechanism: Option<NetworkAuditAuthMechanism>,
    /// Outcome of proxy-side authentication or phantom-token validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_outcome: Option<NetworkAuditAuthOutcome>,
    /// Whether a proxy-managed upstream credential was active for the route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_credential_active: Option<bool>,
    /// Proxy-side injection mode when a managed upstream credential was active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injection_mode: Option<NetworkAuditInjectionMode>,
    /// Structured denial category when the request was denied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denial_category: Option<NetworkAuditDenialCategory>,
    /// SPIFFE audit context, when the request used SPIFFE/SPIRE workload-identity auth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spiffe_context: Option<SpiffeAuditContext>,
    /// OAuth-capture audit context, when the request was served by a
    /// response-buffer-and-rewrite capture route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_context: Option<CaptureAuditContext>,
    /// Hostname or logical service target (for reverse proxy events)
    pub target: String,
    /// Port when available (CONNECT/external), otherwise None
    pub port: Option<u16>,
    /// HTTP method when available
    pub method: Option<String>,
    /// Request path for reverse proxy events
    pub path: Option<String>,
    /// Upstream response status for reverse proxy events
    pub status: Option<u16>,
    /// Denial reason, if denied
    pub reason: Option<String>,
}

/// Summary of append-only integrity metadata for an audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIntegritySummary {
    /// Hash algorithm used for event leaves and chain/root derivation
    pub hash_algorithm: String,
    /// Number of audit events written for the session
    pub event_count: u64,
    /// Hash-chain head over the append-only audit log
    pub chain_head: ContentHash,
    /// Merkle root over ordered audit event leaves
    pub merkle_root: ContentHash,
}

/// Identity of the executable binary launched for a session.
///
/// Cross-platform SHA-256 hash + canonical path of the binary executed by the
/// supervisor. Captured before sandbox apply so the hash commits to exactly
/// the bytes that ran. AUD-03 SHA-256 portion (Plan 22-05a Task 4); the
/// Windows Authenticode addition lands in Plan 22-05b as a SIBLING field on
/// the audit envelope (see RESEARCH Contradiction #2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableIdentity {
    /// Canonical path to the executable file hashed by the supervisor.
    pub resolved_path: PathBuf,
    /// SHA-256 digest of the executable file contents.
    pub sha256: ContentHash,
}

/// Signed attestation metadata for an audit session (AUD-02).
///
/// Plan 22-05a Task 7 (upstream `6ecade2e`): when `--audit-sign-key` is set,
/// the supervisor signs the audit-integrity Merkle root + chain head +
/// session ID and writes a Sigstore bundle to
/// `<session_dir>/audit-attestation.bundle`. This summary records the
/// metadata needed to verify the bundle later (`key_id`, base64 DER public
/// key, predicate type, bundle filename).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditAttestationSummary {
    /// Predicate type embedded in the DSSE/in-toto statement.
    pub predicate_type: String,
    /// Signer key identifier derived from the public key.
    pub key_id: String,
    /// DER-encoded public key as base64, used for standalone keyed verification.
    pub public_key: String,
    /// Filename of the bundle written into the session directory.
    pub bundle_filename: String,
}

/// Rollback availability status for a session.
///
/// Recorded in [`SessionMetadata`] so that `nono rollback list/show/restore`
/// can surface exactly why rollback is or is not available without re-examining
/// snapshot files.
///
/// Older `session.json` payloads that lack this field deserialize as `Available`
/// (backward-compatible default) so existing sessions continue to behave as before.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RollbackStatus {
    /// Baseline snapshot captured successfully; rollback is available.
    #[default]
    Available,
    /// Rollback was explicitly disabled (`--no-rollback`) or skipped (no write paths).
    Skipped,
    /// Baseline snapshot capture was attempted but failed; execution continued with a
    /// warning and rollback is NOT available for this session.
    FailedWarningOnly {
        /// Human-readable reason the capture failed.
        reason: String,
    },
}

impl RollbackStatus {
    /// Returns `true` if rollback snapshots are available for this session.
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Returns a human-readable label for display in `nono rollback list/show`.
    #[must_use]
    pub fn display_label(&self) -> &str {
        match self {
            Self::Available => "rollback-capable",
            Self::Skipped => "audit-only",
            Self::FailedWarningOnly { .. } => "audit-only (capture failed)",
        }
    }
}

/// Metadata for an undo session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Unique session identifier
    pub session_id: String,
    /// Session start time (ISO 8601)
    pub started: String,
    /// Session end time (ISO 8601), None if still running
    pub ended: Option<String>,
    /// Command that was executed
    pub command: Vec<String>,
    /// Canonical executable identity hashed by the supervisor before launch.
    /// AUD-03 SHA-256 portion (upstream 02ee0bd1).
    #[serde(default)]
    pub executable_identity: Option<ExecutableIdentity>,
    /// Paths being tracked for changes
    pub tracked_paths: Vec<PathBuf>,
    /// Number of snapshots taken
    pub snapshot_count: u32,
    /// Child process exit code
    pub exit_code: Option<i32>,
    /// Merkle roots from each snapshot (chain of state commitments)
    pub merkle_roots: Vec<ContentHash>,
    /// Network events captured by the proxy during this session
    #[serde(default)]
    pub network_events: Vec<NetworkAuditEvent>,
    /// Number of audit events captured for this session
    #[serde(default)]
    pub audit_event_count: u64,
    /// Optional integrity summary for the append-only audit log
    #[serde(default)]
    pub audit_integrity: Option<AuditIntegritySummary>,
    /// Optional DSSE/in-toto attestation summary for the audit-integrity
    /// commitments (`audit-attestation.bundle` written into the session
    /// directory). AUD-02 (upstream `6ecade2e`).
    #[serde(default)]
    pub audit_attestation: Option<AuditAttestationSummary>,
    /// Whether rollback snapshots were captured for this session.
    ///
    /// Defaults to `Available` when deserializing older payloads that lack this
    /// field, preserving backward compatibility.
    #[serde(default)]
    pub rollback_status: RollbackStatus,
}

/// A snapshot manifest capturing filesystem state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Snapshot sequence number (0 = baseline)
    pub number: u32,
    /// Timestamp when snapshot was taken (ISO 8601)
    pub timestamp: String,
    /// Parent snapshot number (None for baseline)
    pub parent: Option<u32>,
    /// Map of file paths to their state at snapshot time
    pub files: HashMap<PathBuf, FileState>,
    /// Merkle root over all file hashes (cryptographic state commitment)
    pub merkle_root: ContentHash,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-09/D-11 (Phase 115-02, DRAIN-02/DRAIN-03): `ALL` must list exactly
    /// the 10 variants remaining after `InterceptHandshakeFailed`'s removal,
    /// and every entry must pass the exhaustive-match guard (which itself
    /// fails to compile, not just fails this assertion, if it has drifted
    /// from the enum — see `assert_all_variants_covered`'s own doc comment).
    #[test]
    fn all_denial_categories_present_and_guard_covers_every_entry() {
        assert_eq!(
            NetworkAuditDenialCategory::ALL.len(),
            10,
            "NetworkAuditDenialCategory::ALL must list every variant \
             (10, post-D-09 removal of InterceptHandshakeFailed) — update \
             both ALL and assert_all_variants_covered together when a \
             variant is added or removed"
        );
        for category in NetworkAuditDenialCategory::ALL {
            assert_all_variants_covered(category);
        }
    }

    #[test]
    fn content_hash_hex_roundtrip() {
        let bytes = [
            0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
            0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01,
            0x23, 0x45, 0x67, 0x89,
        ];
        let hash = ContentHash::from_bytes(bytes);
        let hex = hash.to_string();
        let parsed: ContentHash = hex.parse().expect("should parse");
        assert_eq!(hash, parsed);
    }

    #[test]
    fn content_hash_prefix_suffix() {
        let bytes = [
            0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
            0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01,
            0x23, 0x45, 0x67, 0x89,
        ];
        let hash = ContentHash::from_bytes(bytes);
        assert_eq!(hash.prefix(), "ab");
        assert!(hash.suffix().starts_with("cdef"));
        assert_eq!(hash.prefix().len() + hash.suffix().len(), 64);
    }

    #[test]
    fn content_hash_invalid_length() {
        let result = "abc".parse::<ContentHash>();
        assert!(result.is_err());
    }

    #[test]
    fn content_hash_invalid_hex() {
        let result = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
            .parse::<ContentHash>();
        assert!(result.is_err());
    }

    #[test]
    fn content_hash_serde_roundtrip() {
        let bytes = [42u8; 32];
        let hash = ContentHash::from_bytes(bytes);
        let json = serde_json::to_string(&hash).expect("should serialize");
        let parsed: ContentHash = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(hash, parsed);
    }

    #[test]
    fn change_type_display() {
        assert_eq!(ChangeType::Created.to_string(), "+");
        assert_eq!(ChangeType::Modified.to_string(), "~");
        assert_eq!(ChangeType::Deleted.to_string(), "-");
        assert_eq!(ChangeType::PermissionsChanged.to_string(), "p");
    }

    #[test]
    fn snapshot_manifest_serde_roundtrip() {
        let manifest = SnapshotManifest {
            number: 0,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            parent: None,
            files: HashMap::new(),
            merkle_root: ContentHash::from_bytes([0u8; 32]),
        };
        let json = serde_json::to_string(&manifest).expect("should serialize");
        let parsed: SnapshotManifest = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed.number, 0);
        assert!(parsed.parent.is_none());
        assert!(parsed.files.is_empty());
    }

    #[test]
    fn rollback_status_available_is_available() {
        assert!(RollbackStatus::Available.is_available());
        assert!(!RollbackStatus::Skipped.is_available());
        assert!(!RollbackStatus::FailedWarningOnly {
            reason: "disk full".to_string()
        }
        .is_available());
    }

    #[test]
    fn rollback_status_display_labels() {
        assert_eq!(
            RollbackStatus::Available.display_label(),
            "rollback-capable"
        );
        assert_eq!(RollbackStatus::Skipped.display_label(), "audit-only");
        assert_eq!(
            RollbackStatus::FailedWarningOnly {
                reason: "error".to_string()
            }
            .display_label(),
            "audit-only (capture failed)"
        );
    }

    #[test]
    fn rollback_status_serde_roundtrip() {
        let status = RollbackStatus::FailedWarningOnly {
            reason: "locked file".to_string(),
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let parsed: RollbackStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, status);
    }

    #[test]
    fn session_metadata_defaults_rollback_status_for_legacy_json() {
        // Older session.json payloads lack rollback_status; must deserialize as Available.
        let legacy = serde_json::json!({
            "session_id": "legacy-no-status",
            "started": "2025-01-01T00:00:00Z",
            "ended": null,
            "command": ["test"],
            "tracked_paths": [],
            "snapshot_count": 1,
            "exit_code": 0,
            "merkle_roots": [],
        });
        let meta: SessionMetadata = serde_json::from_value(legacy).expect("deserialize");
        // Default must be Available for backward compatibility
        assert_eq!(meta.rollback_status, RollbackStatus::Available);
        assert!(meta.rollback_status.is_available());
    }

    #[test]
    fn session_metadata_rollback_status_skipped_roundtrip() {
        let meta = SessionMetadata {
            session_id: "test-skip".to_string(),
            started: "2025-01-01T00:00:00Z".to_string(),
            ended: None,
            command: vec!["test".to_string()],
            executable_identity: None,
            tracked_paths: vec![],
            snapshot_count: 0,
            exit_code: None,
            merkle_roots: vec![],
            network_events: vec![],
            audit_event_count: 0,
            audit_integrity: None,
            audit_attestation: None,
            rollback_status: RollbackStatus::Skipped,
        };
        let json = serde_json::to_string(&meta).expect("serialize");
        let parsed: SessionMetadata = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.rollback_status, RollbackStatus::Skipped);
        assert!(!parsed.rollback_status.is_available());
    }

    #[test]
    fn session_metadata_rollback_status_failed_warning_roundtrip() {
        let reason = "disk full during baseline capture".to_string();
        let meta = SessionMetadata {
            session_id: "test-fail".to_string(),
            started: "2025-01-01T00:00:00Z".to_string(),
            ended: None,
            command: vec!["test".to_string()],
            executable_identity: None,
            tracked_paths: vec![],
            snapshot_count: 0,
            exit_code: None,
            merkle_roots: vec![],
            network_events: vec![],
            audit_event_count: 0,
            audit_integrity: None,
            audit_attestation: None,
            rollback_status: RollbackStatus::FailedWarningOnly {
                reason: reason.clone(),
            },
        };
        let json = serde_json::to_string(&meta).expect("serialize");
        let parsed: SessionMetadata = serde_json::from_str(&json).expect("deserialize");
        assert!(!parsed.rollback_status.is_available());
        assert_eq!(
            parsed.rollback_status.display_label(),
            "audit-only (capture failed)"
        );
    }
}
