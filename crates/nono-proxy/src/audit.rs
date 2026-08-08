//! Audit logging for proxy requests.
//!
//! Logs all proxy requests with structured fields via `tracing`.
//! Sensitive data (authorization headers, tokens, request bodies)
//! is never included in audit logs.

use nono::undo::{
    CaptureAuditContext, NetworkAuditAuthMechanism, NetworkAuditAuthOutcome, NetworkAuditDecision,
    NetworkAuditDenialCategory, NetworkAuditEvent, NetworkAuditInjectionMode, NetworkAuditMode,
    SpiffeAuditContext,
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
    // `denial_category` deliberately removed (D-06, Phase 115-02, DRAIN-03/NEW-01):
    // it lived here as an `Option<_>` on a `#[derive(Default)]` struct, which let a
    // new `log_denied` call site silently default to uncategorised. It is now
    // `log_denied`'s own required 3rd positional parameter below, so omitting a
    // category is a compile error (E0061), not a silent default. `log_allowed`
    // never read this field (confirmed by inspection), so its removal has no
    // effect on that path.
    pub spiffe_context: Option<SpiffeAuditContext>,
    pub capture_context: Option<CaptureAuditContext>,
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
            spiffe_context: ctx.spiffe_context.clone(),
            capture_context: ctx.capture_context.clone(),
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
///
/// `category` is a required, non-`Option` argument (D-06): a new call site
/// that omits it fails to compile rather than silently emitting an
/// uncategorised denial. Bite-proof, live-verified (see Task 1 of
/// `115-02-PLAN.md`): temporarily removing this argument from a call site
/// produces `error[E0061]: this function takes 6 arguments but 5 arguments
/// were supplied`. Counterexample (do not uncomment — kept for reviewers):
/// ```text
/// // log_denied(audit_log, ProxyMode::Connect, &EventContext::default(), host, port, reason);
/// // ^ missing `category` argument — E0061 missing-argument compile error.
/// ```
pub fn log_denied(
    audit_log: Option<&SharedAuditLog>,
    mode: ProxyMode,
    category: NetworkAuditDenialCategory,
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
            denial_category: Some(category),
            spiffe_context: ctx.spiffe_context.clone(),
            capture_context: ctx.capture_context.clone(),
            target: host.to_string(),
            port: Some(port),
            method: None,
            path: None,
            status: None,
            reason: Some(reason.to_string()),
        },
    );
}

/// Target details for `log_l7_request`, grouped into a struct to keep the
/// function under clippy's `too_many_arguments` threshold.
#[derive(Debug, Clone, Copy)]
pub struct L7RequestInfo<'a> {
    pub host: &'a str,
    pub port: u16,
    pub method: &'a str,
    pub path: &'a str,
    pub status: u16,
}

/// Log an L7 (application-layer) proxy request result: a request that was
/// forwarded to an upstream and produced an observed response status.
///
/// Distinct from `log_allowed` (no `status`/`path`) and `log_reverse_proxy`
/// (no `managed_credential_active`, and `route_id`/`target` are both the
/// service name rather than the actual upstream host). Added for the
/// forward-proxy path (`server::handle_forward_http`, #1335), which needs to
/// audit `target = host`, `status`, `method`, `path`, and
/// `managed_credential_active = Some(false)` together — the forward path is
/// a transparent pass-through, so its audit record must positively assert
/// no managed credential was active, not merely omit the field.
pub fn log_l7_request(
    audit_log: Option<&SharedAuditLog>,
    mode: ProxyMode,
    ctx: &EventContext<'_>,
    info: &L7RequestInfo<'_>,
) {
    info!(
        target: "nono_proxy::audit",
        mode = %mode,
        host = info.host,
        port = info.port,
        method = info.method,
        path = info.path,
        status = info.status,
        decision = "allow",
        "proxy L7 request forwarded"
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
            spiffe_context: ctx.spiffe_context.clone(),
            capture_context: ctx.capture_context.clone(),
            target: info.host.to_string(),
            port: Some(info.port),
            method: Some(info.method.to_string()),
            path: Some(info.path.to_string()),
            status: Some(info.status),
            reason: None,
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
            spiffe_context: None,
            capture_context: None,
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
            NetworkAuditDenialCategory::HostDenied,
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
