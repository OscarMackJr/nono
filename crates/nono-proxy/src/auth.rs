// Adding a new auth mechanism: add a variant here and a corresponding handler
// branch in reverse.rs (handle_spiffe_route / handle_spiffe_assertion_credential).
// NOTE: keep this comment as-is — it correctly documents the SPIFFE-only scope
// this phase keeps (OD-1: declining b1ecbc02's general client_credentials wiring).

use crate::error::{ProxyError, Result};
use std::sync::Arc;
use zeroize::Zeroizing;

pub enum UpstreamAuthMaterial {
    BearerToken {
        header: String,
        token: Zeroizing<String>,
        workload_spiffe_id: String,
        credential_format: String,
    },
}

impl UpstreamAuthMaterial {
    pub fn spiffe_audit_context(&self) -> nono::undo::SpiffeAuditContext {
        let UpstreamAuthMaterial::BearerToken {
            workload_spiffe_id,
            token,
            ..
        } = self;
        nono::undo::SpiffeAuditContext {
            trust_domain: extract_trust_domain(workload_spiffe_id),
            workload_spiffe_id: workload_spiffe_id.clone(),
            svid_type: "jwt".to_string(),
            source: "spire-workload-api".to_string(),
            upstream_spiffe_id: None,
            delegation: crate::spiffe::delegation_from_jwt(token.as_str()),
        }
    }
}

pub enum ManagedUpstreamAuth {
    SpiffeJwt(Arc<crate::spiffe::SpiffeJwtSource>),
}

impl ManagedUpstreamAuth {
    #[must_use = "dropping credential material without using it wastes an SVID fetch"]
    pub async fn acquire(&self) -> Result<UpstreamAuthMaterial> {
        match self {
            ManagedUpstreamAuth::SpiffeJwt(src) => {
                let (token, spiffe_id) = src
                    .fetch_token(&src.audience)
                    .await
                    .map_err(|e| ProxyError::Credential(e.to_string()))?;
                let fmt = crate::config::resolved_credential_format(
                    &src.inject_header,
                    src.credential_format.as_deref(),
                );
                Ok(UpstreamAuthMaterial::BearerToken {
                    header: src.inject_header.clone(),
                    token,
                    workload_spiffe_id: spiffe_id,
                    credential_format: fmt,
                })
            }
        }
    }

    pub fn audit_mechanism(&self) -> nono::undo::NetworkAuditAuthMechanism {
        match self {
            ManagedUpstreamAuth::SpiffeJwt(_) => {
                nono::undo::NetworkAuditAuthMechanism::SpiffeJwtBearer
            }
        }
    }

    pub fn audit_injection_mode(&self) -> Option<nono::undo::NetworkAuditInjectionMode> {
        match self {
            ManagedUpstreamAuth::SpiffeJwt(_) => {
                Some(nono::undo::NetworkAuditInjectionMode::SpiffeJwt)
            }
        }
    }
}

// `spiffe://prod.example/workload` -> `"prod.example"`. Returns "" and logs a warning
// for malformed IDs.
pub fn extract_trust_domain(spiffe_id: &str) -> String {
    match spiffe_id
        .strip_prefix("spiffe://")
        .and_then(|s| s.split('/').next())
    {
        Some(domain) => domain.to_string(),
        None => {
            tracing::warn!(
                "extract_trust_domain: malformed SPIFFE ID (missing spiffe:// prefix): audit trust_domain will be empty"
            );
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extract_trust_domain_valid() {
        assert_eq!(
            extract_trust_domain("spiffe://prod.example/workload"),
            "prod.example"
        );
    }
    #[test]
    fn extract_trust_domain_no_path() {
        assert_eq!(
            extract_trust_domain("spiffe://prod.example"),
            "prod.example"
        );
    }
    #[test]
    fn extract_trust_domain_invalid() {
        assert_eq!(extract_trust_domain("not-a-spiffe-id"), "");
    }
    #[test]
    fn extract_trust_domain_empty() {
        assert_eq!(extract_trust_domain(""), "");
    }
}
