//! Audit logging for proxy requests.
//!
//! Logs all proxy requests with structured fields via `tracing`.
//! Sensitive data (authorization headers, tokens, request bodies)
//! is never included in audit logs.

use nono::undo::{
    NetworkAuditAuthMechanism, NetworkAuditAuthOutcome, NetworkAuditDecision,
    NetworkAuditDenialCategory, NetworkAuditEvent, NetworkAuditInjectionMode, NetworkAuditMode,
};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Maximum number of in-memory network audit events kept per proxy session.
const MAX_AUDIT_EVENTS: usize = 4096;

/// Shared in-memory sink for network audit events.
pub type SharedAuditLog = Arc<Mutex<Vec<NetworkAuditEvent>>>;

/// Proxy mode for audit logging.
#[derive(Debug, Clone, Copy)]
pub enum ProxyMode {
    /// CONNECT tunnel (host filtering only)
    Connect,
    /// Reverse proxy (credential injection)
    Reverse,
    /// External proxy passthrough (enterprise)
    External,
}

/// Optional structured audit context attached to a proxy event.
///
/// Composed into network-event rows by the `log_allowed` / `log_denied` /
/// `log_l7_request` emitters. Per upstream `9300de9` (v0.51.0) — fork-side D-20
/// manual replay against the Phase 22-05a / Phase 23 REQ-AUD-05 audit envelope.
/// All fields are optional; default-constructed context contributes no
/// additional ledger entries and preserves prior-Phase emission shape.
#[derive(Debug, Clone, Default)]
pub struct EventContext<'a> {
    pub route_id: Option<&'a str>,
    pub auth_mechanism: Option<NetworkAuditAuthMechanism>,
    pub auth_outcome: Option<NetworkAuditAuthOutcome>,
    pub managed_credential_active: Option<bool>,
    pub injection_mode: Option<NetworkAuditInjectionMode>,
    pub denial_category: Option<NetworkAuditDenialCategory>,
}

impl std::fmt::Display for ProxyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyMode::Connect => write!(f, "connect"),
            ProxyMode::Reverse => write!(f, "reverse"),
            ProxyMode::External => write!(f, "external"),
        }
    }
}

/// Create a shared in-memory audit log.
#[must_use]
pub fn new_audit_log() -> SharedAuditLog {
    Arc::new(Mutex::new(Vec::new()))
}

/// Drain all network audit events collected so far.
#[must_use]
pub fn drain_audit_events(audit_log: &SharedAuditLog) -> Vec<NetworkAuditEvent> {
    match audit_log.lock() {
        Ok(mut events) => events.drain(..).collect(),
        Err(e) => {
            warn!(
                "Network audit log mutex poisoned while draining events: {}",
                e
            );
            Vec::new()
        }
    }
}

fn now_unix_millis() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let millis = duration.as_millis();
            if millis > u128::from(u64::MAX) {
                warn!("System clock millis exceeded u64::MAX; clamping audit timestamp");
                u64::MAX
            } else {
                millis as u64
            }
        }
        Err(e) => {
            warn!(
                "System clock before UNIX_EPOCH while generating audit timestamp: {}",
                e
            );
            0
        }
    }
}

fn map_mode(mode: ProxyMode) -> NetworkAuditMode {
    match mode {
        ProxyMode::Connect => NetworkAuditMode::Connect,
        ProxyMode::Reverse => NetworkAuditMode::Reverse,
        ProxyMode::External => NetworkAuditMode::External,
    }
}

fn push_event(audit_log: Option<&SharedAuditLog>, event: NetworkAuditEvent) {
    let Some(audit_log) = audit_log else {
        return;
    };

    match audit_log.lock() {
        Ok(mut events) => {
            if events.len() < MAX_AUDIT_EVENTS {
                events.push(event);
            } else {
                warn!(
                    "Network audit buffer full ({} events); dropping event",
                    MAX_AUDIT_EVENTS
                );
            }
        }
        Err(e) => {
            warn!(
                "Network audit log mutex poisoned while recording event: {}",
                e
            );
        }
    }
}

/// Log an allowed proxy request.
pub fn log_allowed(
    audit_log: Option<&SharedAuditLog>,
    mode: ProxyMode,
    ctx: &EventContext<'_>,
    host: &str,
    port: u16,
    method: &str,
) {
    info!(
        target: "nono_proxy::audit",
        mode = %mode,
        host = host,
        port = port,
        method = method,
        decision = "allow",
        "proxy request allowed"
    );

    push_event(
        audit_log,
        NetworkAuditEvent {
            timestamp_unix_ms: now_unix_millis(),
            mode: map_mode(mode),
            decision: NetworkAuditDecision::Allow,
            route_id: ctx.route_id.map(str::to_string),
            auth_mechanism: ctx.auth_mechanism.clone(),
            auth_outcome: ctx.auth_outcome.clone(),
            managed_credential_active: ctx.managed_credential_active,
            injection_mode: ctx.injection_mode.clone(),
            denial_category: None,
            target: host.to_string(),
            port: Some(port),
            method: Some(method.to_string()),
            path: None,
            status: None,
            reason: None,
        },
    );
}

/// Log a denied proxy request.
pub fn log_denied(
    audit_log: Option<&SharedAuditLog>,
    mode: ProxyMode,
    ctx: &EventContext<'_>,
    host: &str,
    port: u16,
    reason: &str,
) {
    info!(
        target: "nono_proxy::audit",
        mode = %mode,
        host = host,
        port = port,
        decision = "deny",
        reason = reason,
        "proxy request denied"
    );

    // Telemetry: additive dual-emit alongside the existing info! call
    // (invariant 5 — never replace, only add). SecurityEventLayer routes
    // nono_security::* events to dual-emit (Application-log + ETW).
    // Host stays cleartext (D-10 exception: host is what the analyst needs
    // per SC-1). Full URLs, paths, and query params are NOT emitted here.
    // The `reason` string is NOT forwarded to avoid leaking sensitive detail;
    // scrubbing happens inside SecurityEventLayer if reason is passed as a
    // field. `ctx.route_id` provides session correlation without exposing
    // URL path components.
    tracing::warn!(
        target: "nono_security::network_deny",
        host = host,
        port = port,
        agent_pid = std::process::id(),
        "network deny"
    );

    push_event(
        audit_log,
        NetworkAuditEvent {
            timestamp_unix_ms: now_unix_millis(),
            mode: map_mode(mode),
            decision: NetworkAuditDecision::Deny,
            route_id: ctx.route_id.map(str::to_string),
            auth_mechanism: ctx.auth_mechanism.clone(),
            auth_outcome: ctx.auth_outcome.clone(),
            managed_credential_active: ctx.managed_credential_active,
            injection_mode: ctx.injection_mode.clone(),
            denial_category: ctx.denial_category.clone(),
            target: host.to_string(),
            port: Some(port),
            method: None,
            path: None,
            status: None,
            reason: Some(reason.to_string()),
        },
    );
}

/// Log a reverse proxy request with service info.
pub fn log_reverse_proxy(
    audit_log: Option<&SharedAuditLog>,
    service: &str,
    method: &str,
    path: &str,
    status: u16,
) {
    info!(
        target: "nono_proxy::audit",
        mode = "reverse",
        service = service,
        method = method,
        path = path,
        status = status,
        "reverse proxy response"
    );

    push_event(
        audit_log,
        NetworkAuditEvent {
            timestamp_unix_ms: now_unix_millis(),
            mode: NetworkAuditMode::Reverse,
            decision: NetworkAuditDecision::Allow,
            route_id: Some(service.to_string()),
            auth_mechanism: None,
            auth_outcome: None,
            managed_credential_active: None,
            injection_mode: None,
            denial_category: None,
            target: service.to_string(),
            port: None,
            method: Some(method.to_string()),
            path: Some(path.to_string()),
            status: Some(status),
            reason: None,
        },
    );
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn log_allowed_records_event() {
        let log = new_audit_log();

        log_allowed(
            Some(&log),
            ProxyMode::Connect,
            &EventContext::default(),
            "api.openai.com",
            443,
            "CONNECT",
        );

        let events = drain_audit_events(&log);
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.mode, NetworkAuditMode::Connect);
        assert_eq!(event.decision, NetworkAuditDecision::Allow);
        assert_eq!(event.route_id, None);
        assert_eq!(event.auth_mechanism, None);
        assert_eq!(event.target, "api.openai.com");
        assert_eq!(event.port, Some(443));
        assert_eq!(event.method.as_deref(), Some("CONNECT"));
        assert!(event.timestamp_unix_ms > 0);
    }

    #[test]
    fn log_denied_records_reason() {
        let log = new_audit_log();

        log_denied(
            Some(&log),
            ProxyMode::External,
            &EventContext::default(),
            "169.254.169.254",
            80,
            "blocked by metadata deny list",
        );

        let events = drain_audit_events(&log);
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.mode, NetworkAuditMode::External);
        assert_eq!(event.decision, NetworkAuditDecision::Deny);
        assert_eq!(event.route_id, None);
        assert_eq!(event.auth_mechanism, None);
        assert_eq!(
            event.reason.as_deref(),
            Some("blocked by metadata deny list")
        );
    }
}
