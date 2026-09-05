//! Policy-free vocabulary for the Nono↔Popeye Session Credential Contract v0.1.
//!
//! Reference by path only:
//! `fiskroad/contracts/NONO_POPEYE_SESSION_CREDENTIAL_CONTRACT_v0.1.md` (external, canonical —
//! never quoted in full or copied into this repo). This module contains **no network
//! implementation** and calls **no real issuance endpoint** — it is a compiling, tested shape
//! for a future implementation to build against, once the joint review named in the contract's
//! header ratifies D-1 through D-5. It must never import from `nono-cli` or `nono-proxy`.
//!
//! # Why `SessionCredentialKey` never derives or implements `Serialize`
//!
//! Serde's derive macro would serialize the wrapped secret string verbatim — exactly the leak
//! REQ-CRED-04 forbids ("The bearer key is unreadable from the contained workspace for the
//! whole session"). Omitting the `Serialize` impl entirely turns that mistake into a **compile
//! error** on [`SessionCredential`] (which embeds a [`SessionCredentialKey`]) rather than a
//! runtime leak discovered only when someone happens to serialize the wrong value. For the
//! same reason, [`SessionCredential`] itself also does not derive `Serialize` — the only
//! serializable view of a live credential is [`WorkspaceVisibleSessionCredential`], produced by
//! [`SessionCredential::workspace_view`], whose shape structurally excludes the secret (it has
//! no `key` field of any type, not even a redacted one).
//!
//! # Recorded, unresolved conflicts
//!
//! See the companion design note for the full discussion (this module does not re-explain
//! them): `../../../.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-DESIGN-session-credential-consumer.md`.

use thiserror::Error;
use zeroize::Zeroizing;

use serde::{Deserialize, Serialize};

/// The contract's fixed `principal_type` value (contract §2.1: `"principal_type": "agent"`,
/// a constant for this contract).
pub const AGENT_PRINCIPAL_TYPE: &str = "agent";

/// A session-credential issuance request, mirroring contract §2.1's request body field-for-field.
///
/// This struct carries no secret — every field is either an identifier, a classification value,
/// or a plain integer — so it derives `Serialize`/`Deserialize` freely, unlike
/// [`SessionCredential`] below.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCredentialRequest {
    /// Contract §2.1: constant `"agent"` for this contract. See [`AGENT_PRINCIPAL_TYPE`].
    pub principal_type: String,
    /// Contract §2.1: stable agent identifier.
    pub agent_id: String,
    /// Contract §2.1: the user or application principal this agent acts on behalf of.
    pub on_behalf_of: String,
    /// Contract §2.1: one of the program's vocabulary purposes.
    pub purpose: String,
    /// Contract §2.1: canonical lowercase UUID tenant identifier.
    pub tenant_id: String,
    /// Contract §2.1: requested time-to-live, in seconds. The issuer clamps this to its own
    /// configured maximum (contract §4.1); this scaffold does not implement clamping.
    pub requested_ttl_seconds: u64,
    /// Contract §2.1: UUID minted by nono at governed launch.
    pub session_id: String,
    /// Contract §2.1: opaque managed-device reference — shape decided by D-1 (undecided here).
    pub device_ref: String,
}

/// A newtype wrapper around the bearer secret returned by popeye's issuance response
/// (contract §2.3's `key` field). Never serializable, never `Display`-able, and its `Debug`
/// output is hand-written to redact the wrapped value — see the module doc for why.
#[derive(Clone)]
pub struct SessionCredentialKey(Zeroizing<String>);

impl SessionCredentialKey {
    /// Wrap a secret value.
    pub fn new(secret: impl Into<String>) -> Self {
        Self(Zeroizing::new(secret.into()))
    }

    /// Expose the wrapped secret as a string slice.
    ///
    /// This accessor exists only for a future real proxy-injection call site (the mechanism
    /// described in the design note's §2, mirroring
    /// `crates/nono-proxy/src/credential.rs`'s `LoadedCredential` injection shape) and must
    /// never be called from workspace-visible code.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

/// Hand-written `Debug` impl that redacts the wrapped secret, mirroring
/// `crates/nono-proxy/src/credential.rs`'s `LoadedCredential` `Debug` impl shape — a
/// `debug_tuple` call with a literal `"[REDACTED]"` field, never a derived `Debug`.
impl std::fmt::Debug for SessionCredentialKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SessionCredentialKey")
            .field(&"[REDACTED]")
            .finish()
    }
}

/// A full session credential, mirroring contract §2.3's issuance response field-for-field.
///
/// Deliberately does **not** derive or implement `Serialize` — it carries the secret-bearing
/// `key` field, and the module doc above explains why no `Serialize` impl exists for any type
/// that embeds [`SessionCredentialKey`]. The only serializable view is
/// [`WorkspaceVisibleSessionCredential`], produced by [`Self::workspace_view`].
#[derive(Debug, Clone)]
pub struct SessionCredential {
    /// Contract §2.3: the bearer credential itself, opaque to nono. Never serialize this
    /// struct or expose this field outside the proxy's injection path.
    pub key: SessionCredentialKey,
    /// Contract §2.3: non-secret key identifier for logs/receipts.
    pub key_ref: String,
    /// Contract §2.3: RFC 3339 UTC expiry timestamp.
    pub expires_at: String,
    /// Contract §2.3: echoed session identifier.
    pub session_id: String,
    /// Contract §2.3: informational budget id this key draws on.
    pub budget_scope: String,
}

impl SessionCredential {
    /// Produce the subset of this credential a contained workspace or config surface may see.
    ///
    /// Clones `key_ref`, `expires_at`, `session_id`, and `budget_scope` only — never touches
    /// `key`. The returned type has no field capable of carrying the secret at all.
    pub fn workspace_view(&self) -> WorkspaceVisibleSessionCredential {
        WorkspaceVisibleSessionCredential {
            key_ref: self.key_ref.clone(),
            expires_at: self.expires_at.clone(),
            session_id: self.session_id.clone(),
            budget_scope: self.budget_scope.clone(),
        }
    }
}

/// The subset of a [`SessionCredential`] a contained workspace or config surface may see.
///
/// This is the shape a contained workspace or config surface may see; it structurally cannot
/// carry the secret because the field does not exist on the type — not because of a redaction
/// pass, but because no `key` field of any type is declared here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceVisibleSessionCredential {
    /// Non-secret key identifier for logs/receipts (contract §2.3).
    pub key_ref: String,
    /// RFC 3339 UTC expiry timestamp (contract §2.3).
    pub expires_at: String,
    /// Echoed session identifier (contract §2.3).
    pub session_id: String,
    /// Informational budget id this key draws on (contract §2.3).
    pub budget_scope: String,
}

/// Errors from session-credential issuance.
///
/// A separate, dedicated `thiserror`-derived enum — **not** a new [`crate::NonoError`] variant
/// — specifically so this speculative, unratified-contract error shape never has to be threaded
/// through `NonoError`'s exhaustive `diagnostic_code()`/`remediation()` match arms.
#[derive(Error, Debug)]
pub enum IssueError {
    /// REQ-CRED-05's variant: issuance failed, and governed launch must fail closed with no
    /// fallback credential of any kind.
    #[error("session credential issuance failed; governed launch fails closed: {reason}")]
    FailClosed {
        /// Human-readable description of why issuance failed.
        reason: String,
    },
    /// REQ-CRED-06's variant: the gateway refused an expired key, or the issuer itself refused
    /// the request — a distinct condition from a fail-closed launch abort.
    #[error("session credential refused or expired: {reason}")]
    RefusedOrExpired {
        /// Human-readable description of the refusal or expiry.
        reason: String,
    },
}

/// A source of session credentials — the consumer-side (nono) half of the contract's issuance
/// flow. A real implementation would call popeye's issuance endpoint; this scaffold defines
/// only the shape.
pub trait SessionCredentialIssuer {
    /// Request a session credential for `req`. A real implementor calls popeye's issuance
    /// endpoint (contract §2); this scaffold does not implement any HTTP client.
    fn request_session_credential(
        &self,
        req: &SessionCredentialRequest,
    ) -> Result<SessionCredential, IssueError>;

    /// Placeholder for the client's own outbound authentication to the issuance endpoint
    /// (contract §7 D-1 — issuer identity, undecided).
    ///
    /// This default is never overridden or called by any code in this crate; a real
    /// HTTP-backed implementor fills it in only after the joint review ratifies D-1.
    fn outbound_auth_placeholder(&self) -> ! {
        todo!("contract §7 D-1 — issuer identity, undecided")
    }
}

/// Request a session credential and return the issuer's result unchanged.
///
/// This is the single call site REQ-CRED-05 needs: no retry, no default credential, no fallback
/// branch of any kind. It is not wired into `exec_strategy_windows/`, `agent_daemon/`, or any
/// real launch path by this scaffold.
pub fn issue_session_credential_or_fail_closed(
    issuer: &dyn SessionCredentialIssuer,
    req: &SessionCredentialRequest,
) -> Result<SessionCredential, IssueError> {
    issuer.request_session_credential(req)
}
