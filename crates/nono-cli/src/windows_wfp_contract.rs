use serde::{Deserialize, Serialize};

/// Wire-protocol version for the `nono-wfp-service` control pipe.
///
/// **Bump this whenever a field is added to [`WfpRuntimeActivationRequest`] or
/// [`WfpRuntimeActivationResponse`].** Serde silently ignores unknown fields, so
/// a service built before a field was added deserializes the request, drops the
/// field, and enforces a *weaker* policy than the caller asked for — while
/// reporting success. That is a fail-open, and the version check is the only
/// thing standing between a stale service binary and silent under-enforcement.
///
/// History:
/// - `1` — initial contract.
/// - `2` — Phase 110-06 added `localhost_port_ranges` to the request but did not
///   bump, so a July service silently dropped port ranges and installed zero
///   filters for an `allow-all` + range policy while returning
///   `enforced-pending-cleanup`. Bumped retroactively, together with
///   `installed_filter_count` on the response.
pub const WFP_RUNTIME_PROTOCOL_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WfpRuntimeActivationRequest {
    pub protocol_version: u32,
    pub request_kind: String,
    pub network_mode: String,
    pub preferred_backend: String,
    pub active_backend: String,
    pub runtime_target: String,
    pub tcp_connect_ports: Vec<u16>,
    pub tcp_bind_ports: Vec<u16>,
    pub localhost_ports: Vec<u16>,
    /// Loopback-only port ranges (inclusive, `start <= end`), expressed
    /// natively via WFP's `FWP_MATCH_RANGE` condition — one filter object per
    /// range entry, never unrolled into per-port entries.
    pub localhost_port_ranges: Vec<(u16, u16)>,
    pub target_program_path: Option<String>,
    pub session_sid: Option<String>,
    pub outbound_rule_name: Option<String>,
    pub inbound_rule_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WfpRuntimeActivationResponse {
    pub protocol_version: u32,
    pub status: String,
    pub details: String,
    /// Number of WFP filter objects the service actually installed (activation)
    /// or removed (cleanup). `None` on responses that touch no filters.
    ///
    /// The caller uses this to close the gap the version bump cannot: a service
    /// at the right protocol version can still install fewer filters than the
    /// request implies. Any activation that gets past the "no enforcement
    /// needed" early-return must yield at least one filter, so a reported `0`
    /// means the policy was not enforced and the caller must fail closed rather
    /// than trust the `enforced-pending-cleanup` status.
    #[serde(default)]
    pub installed_filter_count: Option<u32>,
}
