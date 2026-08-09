//! SPIFFE/SPIRE proxy integration tests.
//!
//! Fail-closed tests run everywhere (no SPIRE needed).
//! Live tests run only when `SPIRE_AGENT_SOCKET` is set — `.github/workflows/spire.yml`
//! sets it in CI.
//!
//! D-07 (113-CONTEXT.md, HARD ACCEPTANCE CRITERION): every test that skips for
//! lack of a live SPIRE agent emits a `SKIP[<module_path>]: <reason>` marker to
//! stderr before returning, rather than a silent/bare ad-hoc "skip" message
//! or a bare early `return` with no marker at all. This is the mechanism that
//! prevents a skipped test from being mistaken for a passing one — see
//! `crates/nono-cli/tests/socket_access_run.rs`'s `python3_available()` guard
//! for the uncounted anti-pattern this deliberately replaces.
#![allow(clippy::unwrap_used)]

use nono_proxy::config::{
    ExternalProxyConfig, InjectMode, ProxyConfig, RouteConfig, SpiffeAuthConfig,
};
use nono_proxy::server;
use nono_proxy::spiffe::SpiffeJwtSource;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Builds a minimal SPIFFE-declared `RouteConfig` pointed at `socket`/`upstream`.
///
/// Fork adaptation: this fork's `RouteConfig` does not carry upstream's
/// `proxy`, `tls_client_cert`, or `tls_client_key` fields (no forward-proxy
/// chaining or mTLS-to-upstream support has been absorbed for routes), and
/// carries an additional `endpoint_policy` field upstream's diffed literal
/// does not — both confirmed by direct read of
/// `crates/nono-proxy/src/config.rs::RouteConfig` this session.
fn make_jwt_route(socket: &str, upstream: &str) -> RouteConfig {
    RouteConfig {
        prefix: "testapi".to_string(),
        upstream: upstream.to_string(),
        credential_key: None,
        inject_mode: InjectMode::Header,
        inject_header: "Authorization".to_string(),
        credential_format: None,
        path_pattern: None,
        path_replacement: None,
        query_param_name: None,
        env_var: None,
        endpoint_rules: vec![],
        tls_ca: None,
        oauth2: None,
        aws_auth: None,
        spiffe: Some(SpiffeAuthConfig::Jwt {
            workload_api_socket: socket.to_string(),
            audience: vec!["test-audience".to_string()],
            inject_header: "Authorization".to_string(),
            credential_format: None,
            svid_hint: None,
        }),
        capture: None,
        endpoint_policy: None,
    }
}

/// Returns the SPIRE agent socket path, or `None` if `SPIRE_AGENT_SOCKET` is unset.
fn live_socket() -> Option<String> {
    std::env::var("SPIRE_AGENT_SOCKET").ok()
}

fn live_audience() -> Vec<String> {
    let td = std::env::var("SPIRE_TRUST_DOMAIN").unwrap_or_else(|_| "test.nono".to_string());
    vec![format!("spiffe://{td}")]
}

fn live_expected_spiffe_id() -> String {
    std::env::var("SPIRE_WORKLOAD_SPIFFE_ID")
        .unwrap_or_else(|_| "spiffe://test.nono/nono-proxy".to_string())
}

// ─── Fail-closed tests (no SPIRE needed) ────────────────────────────────────

#[tokio::test]
async fn test_spiffe_jwt_fails_closed_on_missing_socket() {
    let route = make_jwt_route(
        "/tmp/nono-test-nonexistent-spire-socket.sock",
        "http://127.0.0.1:1",
    );
    let result = server::start(ProxyConfig {
        routes: vec![route],
        ..ProxyConfig::default()
    })
    .await;
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.contains("SPIFFE") || err.contains("spiffe") || err.contains("socket"),
        "unexpected error: {}",
        err
    );
}

// ─── D-17/DRAIN-06 structural ordering regression ───────────────────────────
//
// PROPERTY: in EVERY production dispatch function in `reverse.rs` that can
// trigger a credential mint, the filter host-check and its `HostDenied` deny
// branch must run BEFORE that mint — so a `deny_domain`-blocked upstream is
// denied without ever reaching the SPIRE Workload API or an IdP token
// endpoint.
//
// WHY THIS IS A CLASS TEST, NOT AN INSTANCE TEST. The first version of this
// test (Phase 115 Plan 04) scanned `handle_spiffe_route`'s body only. Phase
// 115's verification found that `reverse.rs` documents TWO independent,
// mutually exclusive SPIFFE-backed auth sources per route — `route.spiffe`
// (direct JWT-SVID bearer injection) and `route.oauth2.client_assertion`
// (RFC 7523 jwt-bearer exchange) — and the hoist had landed on the first
// only. The one-function scan was structurally incapable of observing that.
// So this test now DISCOVERS its own subjects from the shipped source rather
// than naming them: it enumerates every top-level `async fn` in `reverse.rs`'s
// production section, and applies the ordering assertion to each one whose
// body reaches any known mint primitive (`MINT_MARKERS`). A third dispatch
// path added later is therefore covered automatically, with no edit here.
//
// TWO DELIBERATE MUST-ACKNOWLEDGE FAILURE MODES (not brittleness):
//   1. A minting dispatch fn with no host-check at all fails loudly rather
//      than being skipped — "no check to order" must never read as "ordered".
//   2. `KNOWN_MINTING_DISPATCH_FNS` is asserted to be a SUBSET of what
//      discovery found. If a mint primitive is renamed or replaced (so a known
//      path stops matching `MINT_MARKERS`), discovery silently loses that
//      path — this assertion converts that silent coverage loss into a failure.
//
// RESIDUAL, stated plainly: discovery is scoped to `reverse.rs`. A SPIFFE
// dispatch added in another proxy path file would not be enumerated here.
// That residual is bounded by D-03/ADR-113: the other proxy paths (CONNECT
// tunnel, forward HTTP, external-proxy chain) fail closed on a SPIFFE-declared
// route rather than implementing their own dispatch, which is itself covered
// by `d03_connect_and_forward_http_deny_spiffe_declared_route_upstream_end_to_end`
// below. Discovery is also blind to a mint reached only through a helper fn
// this file calls (the marker set matches call syntax, not a call graph).
//
// TEST-MECHANISM DECISION (115-CONTEXT.md D-17 / 115-VALIDATION.md "Open
// Design Decision — DRAIN-06 test mechanism", locked option (c) —
// structural): this asserts the property by control-flow position in the
// shipped source, not by driving a live request through `server::start`.
// Both alternatives were rejected explicitly by that decision:
//   - a test double / spy on `SpiffeJwtSource::connect` (option a) needs a
//     new production test-seam on security-relevant code;
//   - an end-to-end integration test pointing `workload_api_socket` at a
//     path (option b) cannot actually observe REQUEST-time ordering here:
//     `RouteStore::load()` (`route.rs`) performs its OWN live connect to
//     `workload_api_socket` at proxy STARTUP, before the filter is even
//     built (`server.rs::start`, route load precedes `with_denied_hosts`)
//     — an unreachable socket fails the WHOLE proxy at startup
//     (see `test_spiffe_jwt_fails_closed_on_missing_socket` above), so a
//     `server::start`-based test can never reach the SPIFFE dispatch
//     functions' request handling at all without a socket that a live SPIRE
//     agent already answers. That is exactly the "needs no SPIRE agent" bar
//     option (c) was chosen to clear.

/// Call-syntax markers for "this line reaches a credential mint".
///
/// `.get_or_refresh(` is `SpiffeAssertionTokenCache::get_or_refresh` ->
/// `exchange_jwt_assertion` -> `SpiffeJwtSource::fetch_token` (a SPIRE
/// Workload API JWT-SVID mint AND an outbound POST of that SVID to the IdP
/// token endpoint). `managed_auth.acquire(` is the direct JWT-SVID path.
/// The remaining two are the lower-level primitives, listed so a dispatch
/// function that calls them directly is also caught.
const MINT_MARKERS: &[&str] = &[
    "managed_auth.acquire(",
    ".get_or_refresh(",
    ".fetch_token(",
    "exchange_jwt_assertion(",
];

/// Dispatch functions that are KNOWN to mint. Asserted to be a subset of what
/// discovery finds — see must-acknowledge failure mode 2 above. This list
/// exists to detect coverage LOSS; it is not what drives the assertions.
const KNOWN_MINTING_DISPATCH_FNS: &[&str] =
    &["handle_spiffe_route", "handle_spiffe_assertion_credential"];

/// Drops whole-line `//` and `///` comments.
///
/// Without this, prose in a doc comment that names a marker (e.g. a comment
/// explaining the ordering, positioned above the code it describes) would be
/// found instead of the real call site — a false positive that actually bit
/// Plan 115-04 and forced a comment reword. Stripping comments removes that
/// coupling, so ordering comments may name the real call syntax freely.
fn strip_line_comments(body: &str) -> String {
    body.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn d17_spiffe_dispatch_host_check_precedes_mint_structurally() {
    let full = include_str!("../src/reverse.rs");

    // Scan production code only — `reverse.rs`'s `#[cfg(test)] mod tests`
    // legitimately contains mint-marker text in fixtures and assertions.
    let production = match full.find("\n#[cfg(test)]") {
        Some(i) => &full[..i],
        None => full,
    };

    // Enumerate top-level `async fn` items. Top-level items in this rustfmt'd
    // file start at column 0 and close with their brace alone on an unindented
    // line ("\n}\n") — nested block closes are always indented — so this bounds
    // each body without a full brace-depth parser.
    let mut fn_starts: Vec<(String, usize)> = Vec::new();
    let mut offset = 0usize;
    for line in production.split_inclusive('\n') {
        if line.starts_with("async fn ")
            || line.starts_with("pub async fn ")
            || line.starts_with("pub(crate) async fn ")
        {
            let name = line
                .split("async fn ")
                .nth(1)
                .and_then(|rest| rest.split(['(', '<', ' ']).next())
                .unwrap_or("<unparsed>")
                .to_string();
            fn_starts.push((name, offset));
        }
        offset += line.len();
    }
    assert!(
        !fn_starts.is_empty(),
        "no top-level `async fn` found in reverse.rs — the discovery scan is broken \
         (file shape changed?), so this test would vacuously pass; fix the scan"
    );

    let mut minting_fns: Vec<String> = Vec::new();

    for (name, start) in &fn_starts {
        let body_end = production[*start..]
            .find("\n}\n")
            .map(|rel| start + rel)
            .unwrap_or(production.len());
        let body = strip_line_comments(&production[*start..body_end]);

        let Some((mint_pos, mint_marker)) = MINT_MARKERS
            .iter()
            .filter_map(|m| body.find(m).map(|p| (p, *m)))
            .min()
        else {
            continue; // this function performs no credential mint
        };
        minting_fns.push(name.clone());

        let check_host_pos = body.find("ctx.filter.check_host(").unwrap_or_else(|| {
            panic!(
                "D-17/DRAIN-06: `{name}` reaches a credential mint (`{mint_marker}`) but contains \
                 no `ctx.filter.check_host(` at all. A minting dispatch path with no host-check \
                 cannot be 'correctly ordered' — it is unguarded. Add the check-then-deny block \
                 above the mint, mirroring `handle_spiffe_route`."
            )
        });
        let host_denied_pos = body
            .find("NetworkAuditDenialCategory::HostDenied")
            .unwrap_or_else(|| {
                panic!(
                    "D-17/DRAIN-06: `{name}` reaches a credential mint (`{mint_marker}`) and has a \
                     host-check, but no `NetworkAuditDenialCategory::HostDenied` deny-emission \
                     branch — a check whose deny path is missing or uncategorised does not deny."
                )
            });

        assert!(
            check_host_pos < mint_pos,
            "D-17/DRAIN-06 regression in `{name}`: ctx.filter.check_host(...) (byte \
             {check_host_pos}) must textually precede the credential mint `{mint_marker}` (byte \
             {mint_pos}) — a deny_domain-blocked SPIFFE route must never trigger a SPIRE Workload \
             API fetch (or an IdP token-endpoint POST of a minted SVID) before being denied."
        );
        assert!(
            host_denied_pos < mint_pos,
            "D-17/DRAIN-06 regression in `{name}`: the HostDenied deny-emission branch (byte \
             {host_denied_pos}) must textually precede the credential mint `{mint_marker}` (byte \
             {mint_pos}) — the whole deny path, not just the check call, must resolve before any \
             mint is attempted."
        );
    }

    for known in KNOWN_MINTING_DISPATCH_FNS {
        assert!(
            minting_fns.iter().any(|f| f == known),
            "D-17/DRAIN-06 coverage loss: `{known}` is a known minting SPIFFE dispatch function, \
             but discovery did not classify it as minting. Either it was renamed/removed, or its \
             mint primitive changed and is no longer in MINT_MARKERS ({MINT_MARKERS:?}). Until \
             this is reconciled, that path's ordering is UNGUARDED. Discovered minting fns: \
             {minting_fns:?}"
        );
    }
    assert!(
        minting_fns.len() >= KNOWN_MINTING_DISPATCH_FNS.len(),
        "discovered fewer minting dispatch fns ({minting_fns:?}) than the known set \
         ({KNOWN_MINTING_DISPATCH_FNS:?})"
    );
}

// ─── Live tests (require SPIRE_AGENT_SOCKET) ────────────────────────────────

#[tokio::test]
async fn test_spiffe_jwt_live_fetch() {
    let Some(socket) = live_socket() else {
        eprintln!("SKIP[{}]: SPIRE_AGENT_SOCKET not set", module_path!());
        return;
    };
    let audience = live_audience();
    let expected_id = live_expected_spiffe_id();

    let src = SpiffeJwtSource::connect(
        &socket,
        audience.clone(),
        "Authorization".to_string(),
        None,
        None,
    )
    .await
    .expect("should connect to live SPIRE agent");

    let (token, spiffe_id) = src
        .fetch_token(&audience)
        .await
        .expect("should fetch JWT-SVID from live agent");

    let parts: Vec<&str> = token.as_str().split('.').collect();
    assert_eq!(parts.len(), 3, "JWT-SVID should have three parts");
    assert!(!parts[0].is_empty() && !parts[1].is_empty() && !parts[2].is_empty());

    assert_eq!(
        spiffe_id, expected_id,
        "workload SPIFFE ID should match the registered entry"
    );
}

#[tokio::test]
async fn test_spiffe_jwt_live_delegation_none_on_plain_svid() {
    let Some(socket) = live_socket() else {
        eprintln!("SKIP[{}]: SPIRE_AGENT_SOCKET not set", module_path!());
        return;
    };
    let audience = live_audience();

    let src = SpiffeJwtSource::connect(
        &socket,
        audience.clone(),
        "Authorization".to_string(),
        None,
        None,
    )
    .await
    .expect("should connect to live SPIRE agent");

    let (token, _) = src
        .fetch_token(&audience)
        .await
        .expect("should fetch JWT-SVID");

    // A plain SPIRE JWT-SVID has no `act` claim, so delegation must be None.
    let delegation = nono_proxy::spiffe::delegation_from_jwt(token.as_str());
    assert!(
        delegation.is_none(),
        "plain SPIRE JWT-SVID should not have delegation context"
    );
}

#[tokio::test]
async fn test_spiffe_jwt_live_proxy_startup() {
    let Some(socket) = live_socket() else {
        eprintln!("SKIP[{}]: SPIRE_AGENT_SOCKET not set", module_path!());
        return;
    };
    let audience = live_audience();

    let route = RouteConfig {
        spiffe: Some(SpiffeAuthConfig::Jwt {
            workload_api_socket: socket.clone(),
            audience,
            inject_header: "Authorization".to_string(),
            credential_format: None,
            svid_hint: None,
        }),
        ..make_jwt_route(&socket, "http://127.0.0.1:1")
    };
    let result = server::start(ProxyConfig {
        routes: vec![route],
        ..ProxyConfig::default()
    })
    .await;
    assert!(
        result.is_ok(),
        "proxy should start with a live SPIRE agent: {:?}",
        result.err()
    );
}

// ─── Fork-original test (D-03, no upstream equivalent) ──────────────────────
//
// Upstream has a PARALLEL `tls_intercept/handle.rs::handle_spiffe_intercept_request`
// implementation for TLS-intercepted streams; this fork has none (D-01, ADR-113).
// D-03 (113-CONTEXT.md) requires that a request for a SPIFFE-declared route that
// arrives on a proxy path with no SPIFFE implementation (CONNECT tunnel, forward
// HTTP, external-proxy chain) MUST fail closed at request time. Plan 113-05
// landed this guard and unit-tested it directly inside `server.rs`'s own
// `mod tests` using `RouteStore::from_loaded_routes` — a `#[cfg(test)]
// pub(crate)` constructor that bypasses a live SPIRE Workload API connect.
//
// That constructor is crate-internal: a `#[cfg(test)] pub(crate)` item is
// compiled into the unit-test binary only, and is ABSENT from the plain `rlib`
// Cargo links against this external integration-test binary. So unlike the
// unit-level proof, this end-to-end proof genuinely needs a live, reachable
// SPIRE Workload API to build a `declares_spiffe: true` route through the only
// public entry point (`nono_proxy::server::start`) — hence this test is
// `SPIRE_AGENT_SOCKET`-gated, with its own `SKIP[...]` marker, exactly like the
// three live tests above (executor's documented choice per this plan's own
// `<action>` text, which explicitly allows this fallback).
#[tokio::test]
async fn d03_connect_and_forward_http_deny_spiffe_declared_route_upstream_end_to_end() {
    let Some(socket) = live_socket() else {
        eprintln!("SKIP[{}]: SPIRE_AGENT_SOCKET not set", module_path!());
        return;
    };
    let audience = live_audience();

    let upstream_host = "spiffe-e2e-upstream.invalid";
    let upstream_port = 9443u16;
    let upstream = format!("https://{upstream_host}:{upstream_port}");

    let route = RouteConfig {
        spiffe: Some(SpiffeAuthConfig::Jwt {
            workload_api_socket: socket,
            audience,
            inject_header: "Authorization".to_string(),
            credential_format: None,
            svid_hint: None,
        }),
        ..make_jwt_route("unused", &upstream)
    };

    let handle = server::start(ProxyConfig {
        routes: vec![route],
        require_auth: false,
        external_proxy: Some(ExternalProxyConfig {
            address: "127.0.0.1:1".to_string(),
            auth: None,
            bypass_hosts: vec![],
        }),
        ..ProxyConfig::default()
    })
    .await
    .expect("proxy should start with a live SPIRE agent and a declares_spiffe route");

    // CONNECT arm: must be denied, never tunneled — even with require_auth=false
    // and an external_proxy configured, proving the guard preempts all 3 CONNECT
    // dispatch arms (external-proxy chain, bypass-route direct, non-bypass), per
    // Plan 113-05's unit-level proof of the same property.
    {
        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        let request = format!(
            "CONNECT {upstream_host}:{upstream_port} HTTP/1.1\r\nHost: {upstream_host}:{upstream_port}\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            response_str.starts_with("HTTP/1.1 403"),
            "CONNECT to a SPIFFE-declared route upstream must be denied end-to-end, got: {}",
            response_str
        );
    }

    // forward-HTTP arm: same guard, same denial — `handle_forward_http` has no
    // SPIFFE implementation of its own (the WR-13 defect shape D-03 targets).
    {
        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .unwrap();
        let request = format!(
            "GET http://{upstream_host}:{upstream_port}/v1/models HTTP/1.1\r\nHost: {upstream_host}:{upstream_port}\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            response_str.starts_with("HTTP/1.1 403"),
            "forward-HTTP to a SPIFFE-declared route upstream must be denied end-to-end, got: {}",
            response_str
        );
    }

    let events = handle.drain_audit_events();
    let spiffe_denies = events
        .iter()
        .filter(|e| {
            e.decision == nono::undo::NetworkAuditDecision::Deny
                && e.denial_category
                    == Some(nono::undo::NetworkAuditDenialCategory::SpiffeUnsupportedPath)
        })
        .count();
    assert!(
        spiffe_denies >= 2,
        "expected at least 2 Deny+SpiffeUnsupportedPath audit events (CONNECT + forward-HTTP), got: {:?}",
        events
    );

    handle.shutdown();
}
