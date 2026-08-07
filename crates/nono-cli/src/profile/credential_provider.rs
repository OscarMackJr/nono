//! Declarative OAuth-capture field-path validation (SEC-02, Phase 114, D-12).
//!
//! Ported/adapted from upstream's `profile/credential_provider.rs`
//! (`9b692e07`): the field-path validation shape (`validate_provider_field`/
//! `validate_provider_path`) is the portable part of that file — it encodes
//! real problem knowledge about what a safe dot-separated JSON field path
//! looks like, independent of any provider-registry plumbing.
//!
//! Deliberately NOT ported (D-12's "minus absent-subsystem coupling"):
//! - `CredentialProviderDef` and the separate top-level `credential_providers`
//!   / `credential_routes` indirection layer — this fork's `CustomCredentialDef`
//!   already IS a per-route credential declaration (the binding upstream's
//!   `CredentialRouteDef` exists to create), so there is no separate
//!   provider-registry to bind against. See D-07 ("rebuild the plumbing
//!   against the fork's actual per-route shape") in `114-CONTEXT.md`.
//! - `credential_store` (keychain/file/command-status credential detection)
//!   and `helpers` (status/login/logout lifecycle command hooks) — upstream
//!   subsystems this fork does not have; `114-CONTEXT.md` leaves "whether
//!   nono-cli needs a helper command to drive a login flow" open by explicit
//!   discretion grant, not by omission.
//! - `api_hosts` (provider-level API-origin allowlist) — subsumed by this
//!   fork's existing per-route `upstream` / `endpoint_rules` fields already
//!   on `CustomCredentialDef`; a second, parallel origin list would duplicate
//!   an authorization boundary this fork already enforces elsewhere.
//!
//! What *is* ported: strict field-path validation for
//! `CaptureConfig.response_fields` / `request_nonce_fields`, and the
//! `max_response_bytes` ceiling check (D-05, T-114-10).

use nono::{NonoError, Result};
use nono_proxy::config::{CaptureConfig, CAPTURE_MAX_RESPONSE_BYTES_CEILING};

/// Validate a profile-declared [`CaptureConfig`] before it is allowed to
/// reach the proxy's real `RouteConfig.capture` field.
///
/// `name` identifies the owning custom credential, for error messages. This
/// is a profile-load-time gate (T-114-10 / T-114-11): a malformed field path
/// or an over-large `max_response_bytes` value must be rejected here, never
/// discovered later at request time.
pub(super) fn validate_capture_config(name: &str, capture: &CaptureConfig) -> Result<()> {
    // A capture config declaring nothing to rewrite is meaningless and must
    // be rejected, not silently accepted as a no-op.
    if capture.response_fields.is_empty() {
        return Err(NonoError::ProfileParse(format!(
            "custom credential '{}' has 'capture' set but 'capture.response_fields' is empty; \
             a capture config declaring nothing to rewrite is not valid",
            name
        )));
    }

    for field in &capture.response_fields {
        validate_capture_field_path(name, &field.path)?;
    }

    for field in &capture.request_nonce_fields {
        validate_capture_field_path(name, field)?;
    }

    if let Some(max_bytes) = capture.max_response_bytes {
        if max_bytes == 0 {
            return Err(NonoError::ProfileParse(format!(
                "custom credential '{}' has 'capture.max_response_bytes' set to 0; \
                 a zero-byte cap cannot buffer any response",
                name
            )));
        }
        if max_bytes > CAPTURE_MAX_RESPONSE_BYTES_CEILING {
            return Err(NonoError::ProfileParse(format!(
                "custom credential '{}' has 'capture.max_response_bytes' ({}) exceeding the \
                 hard ceiling of {} bytes",
                name, max_bytes, CAPTURE_MAX_RESPONSE_BYTES_CEILING
            )));
        }
    }

    Ok(())
}

/// Validate a single dot-separated JSON field path used by
/// `capture.response_fields[].path` or `capture.request_nonce_fields[]`.
///
/// Ported from upstream's `validate_provider_field` shape (D-12), extended
/// with the leading/trailing/double-dot rejections this plan's behavior spec
/// requires — upstream's own shape only rejected empty/NUL-bearing values.
fn validate_capture_field_path(name: &str, path: &str) -> Result<()> {
    if path.is_empty()
        || path.starts_with('.')
        || path.ends_with('.')
        || path.contains("..")
        || path.contains('\0')
    {
        return Err(NonoError::ProfileParse(format!(
            "custom credential '{}' has an invalid capture field path '{}': must be non-empty, \
             must not start or end with '.', must not contain '..', and must not contain NUL",
            name, path
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nono_proxy::config::{CaptureResponseField, CaptureResponseFieldKind};

    fn valid_capture() -> CaptureConfig {
        CaptureConfig {
            response_fields: vec![CaptureResponseField {
                path: "access_token".to_string(),
                kind: CaptureResponseFieldKind::Opaque,
            }],
            request_nonce_fields: Vec::new(),
            max_response_bytes: None,
        }
    }

    #[test]
    fn valid_capture_config_passes() {
        assert!(validate_capture_config("myroute", &valid_capture()).is_ok());
    }

    #[test]
    fn empty_response_fields_rejected() {
        let mut capture = valid_capture();
        capture.response_fields.clear();
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn empty_field_path_rejected() {
        let mut capture = valid_capture();
        capture.response_fields[0].path = String::new();
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn leading_dot_field_path_rejected() {
        let mut capture = valid_capture();
        capture.response_fields[0].path = ".access_token".to_string();
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn trailing_dot_field_path_rejected() {
        let mut capture = valid_capture();
        capture.response_fields[0].path = "access_token.".to_string();
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn double_dot_field_path_rejected() {
        let mut capture = valid_capture();
        capture.response_fields[0].path = "data..token".to_string();
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn invalid_request_nonce_field_rejected() {
        let mut capture = valid_capture();
        capture.request_nonce_fields.push(".bad".to_string());
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn zero_max_response_bytes_rejected() {
        let mut capture = valid_capture();
        capture.max_response_bytes = Some(0);
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn max_response_bytes_above_ceiling_rejected() {
        let mut capture = valid_capture();
        capture.max_response_bytes = Some(CAPTURE_MAX_RESPONSE_BYTES_CEILING + 1);
        assert!(validate_capture_config("myroute", &capture).is_err());
    }

    #[test]
    fn max_response_bytes_at_ceiling_accepted() {
        let mut capture = valid_capture();
        capture.max_response_bytes = Some(CAPTURE_MAX_RESPONSE_BYTES_CEILING);
        assert!(validate_capture_config("myroute", &capture).is_ok());
    }

    #[test]
    fn max_response_bytes_none_accepted() {
        let capture = valid_capture();
        assert!(capture.max_response_bytes.is_none());
        assert!(validate_capture_config("myroute", &capture).is_ok());
    }
}
