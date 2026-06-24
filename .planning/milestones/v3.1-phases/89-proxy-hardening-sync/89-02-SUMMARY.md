---
phase: 89-proxy-hardening-sync
plan: "02"
subsystem: nono-proxy
tags: [proxy, equivalence-test, audit, connect, reverse-proxy, D-02, D-09]
dependency_graph:
  requires: []
  provides:
    - "D-02 keep-open equivalence test (connect_keeps_open_on_missing_proxy_auth)"
    - "D-09 403+audit equivalence test (denied_endpoint_returns_403_and_audit)"
  affects:
    - "crates/nono-proxy/src/connect.rs"
    - "crates/nono-proxy/src/reverse.rs"
    - ".planning/phases/89-proxy-hardening-sync/89-02-SUMMARY.md"
tech_stack:
  added: []
  patterns:
    - "tokio::net::TcpListener loopback pair as TcpStream substitute for handler tests"
    - "audit::drain_audit_events assertion pattern for network audit equivalence"
key_files:
  created: []
  modified:
    - "crates/nono-proxy/src/connect.rs"
    - "crates/nono-proxy/src/reverse.rs"
decisions:
  - "D-02 equivalence: CONNECT missing auth is lenient by design (undici compat); cherry-pick 7c9abd3b skipped"
  - "D-09 equivalence: endpoint default-deny returns 403+EndpointPolicy audit before any credential op; cherry-pick a5d623fd skipped"
  - "Loopback TcpListener used for both tests because handle_connect and handle_reverse_proxy require &mut TcpStream, not a generic AsyncWrite"
  - "NetworkAuditMode removed from reverse.rs import (not needed by D-09 assertions); no forced let _ usage"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-20T19:26:17Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 2
---

# Phase 89 Plan 02: Proxy Fork Equivalence Tests (D-02 + D-09) Summary

**One-liner:** CONNECT lenient-auth keep-open (D-02) and reverse-proxy 403+EndpointPolicy audit (D-09) proven with fork unit tests; both upstream cherry-picks skipped as equivalences.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | D-02 keep-open equivalence test (CONNECT missing auth, #1151) | `05cdd0d9` | `crates/nono-proxy/src/connect.rs` |
| 2 | D-09 403+audit equivalence test (denied non-CONNECT, #1077) | `20bc305f` | `crates/nono-proxy/src/reverse.rs` |

## Test Function Names (for 89-04 Ledger Addendum)

| Test fn | File | Issue | Disposition |
|---------|------|-------|-------------|
| `connect_keeps_open_on_missing_proxy_auth` | `crates/nono-proxy/src/connect.rs` | #1151 / D-02 | Equivalence — cherry-pick 7c9abd3b skipped |
| `denied_endpoint_returns_403_and_audit` | `crates/nono-proxy/src/reverse.rs` | #1077 / D-09 | Equivalence — cherry-pick a5d623fd skipped |

## What Was Built

### Task 1: D-02 Keep-Open Equivalence (connect.rs)

Added `connect_keeps_open_on_missing_proxy_auth` to the existing `connect::tests` module.

**Test design:** Uses `tokio::net::TcpListener::bind("127.0.0.1:0")` loopback pair to obtain a real `TcpStream` (required by `handle_connect`). Drives `handle_connect` with a CONNECT line targeting `127.0.0.1:1` and no `Proxy-Authorization` header. The lenient auth block at `connect.rs:46-48` logs at debug and continues.

**Assertions:**
1. Response written to client does NOT start with `HTTP/1.1 407`
2. Error returned (if any) is NOT `ProxyError::InvalidToken`
3. No `NetworkAuditDenialCategory::AuthenticationFailed` audit event emitted

**Result:** Test passes. The handler proceeds past auth, attempts upstream connect (port 1 is refused), returns `UpstreamConnect`. No 407, no auth rejection. This IS the #1151 keep-open intent.

### Task 2: D-09 403+Audit Equivalence (reverse.rs)

Added `denied_endpoint_returns_403_and_audit` to the existing `reverse::tests` module. Also added the async harness imports (`NetworkAuditDecision`, `NetworkAuditDenialCategory` from `nono::undo`, `AsyncReadExt`) and `read_to_string` helper (copied from `connect.rs:280-285`).

**Test design:** Uses `TcpListener` loopback pair (same rationale: `handle_reverse_proxy` requires `&mut TcpStream`). Constructs `ReverseProxyCtx` with:
- `RouteStore` containing one route (`testservice` → `https://example.invalid`) allowing only `GET /v1/models`
- `CredentialStore::empty()` (no credentials)
- `ProxyFilter::allow_all()`
- Minimal `TlsConnector` (built from `route::build_base_root_store()`; never exercised — endpoint deny fires first)

Request: `GET /testservice/forbidden HTTP/1.1` — `/forbidden` is not in the allowed endpoint rules → default-deny.

**Assertions:**
1. Response starts with `HTTP/1.1 403`
2. `audit::drain_audit_events` yields at least one event with `decision == NetworkAuditDecision::Deny`
3. Same event has `denial_category == Some(NetworkAuditDenialCategory::EndpointPolicy)`

**Result:** Test passes. The `reverse.rs:96-116` endpoint default-deny path fires before any credential or auth operation. This IS the #1077 intent.

## Verification

```
cargo test -p nono-proxy connect_keeps_open_on_missing_proxy_auth   → ok (1 test)
cargo test -p nono-proxy denied_endpoint_returns_403_and_audit      → ok (1 test)
cargo test -p nono-proxy                                             → ok (172 tests, 0 failed)
cargo build -p nono-proxy                                           → exit 0
cargo clippy -p nono-proxy -- -D warnings -D clippy::unwrap_used    → 0 warnings
```

## Real Gap Analysis

Neither test reproduced a real gap:
- **D-02:** Handler correctly continues on missing auth (lenient by design). No upstream behavior change needed.
- **D-09:** Handler correctly returns 403 + EndpointPolicy audit before any credential op. No upstream behavior change needed.

Both are pure equivalence findings. No non-test proxy code was modified.

## Deviations from Plan

None — plan executed exactly as written.

**Implementation notes (not deviations):**
- The plan suggested driving `handle_connect` "via `tokio::io::duplex`" but `handle_connect` takes `&mut TcpStream`. Used `TcpListener::bind("127.0.0.1:0")` loopback pair per the plan's own HARNESS CAVEAT (89-PATTERNS.md line 179-184 and 89-PLAN.md Task 2 action). This is the expected approach.
- `NetworkAuditMode` was included in the plan's suggested import line for D-09 but is not needed by the actual assertions. Omitted to keep imports minimal and avoid dead-code appearance. Clippy passes.

## Cross-Target Verification

The proxy crate (`nono-proxy`) has ZERO `#[cfg(target_os = ...)]` gates (confirmed in 89-RESEARCH.md §Environment Availability). No cfg-gated lines were touched by this plan. Standard `cargo clippy -p nono-proxy` on the Windows dev-host is sufficient — no cross-target deferral required for this plan's changes.

## Known Stubs

None — no stubs introduced. Both tests are pure behavioral assertions on existing production code paths.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced. Tests only read existing behavior.

## Self-Check: PASSED

| Item | Status |
|------|--------|
| `crates/nono-proxy/src/connect.rs` exists | FOUND |
| `crates/nono-proxy/src/reverse.rs` exists | FOUND |
| `89-02-SUMMARY.md` exists | FOUND |
| Commit `05cdd0d9` exists | FOUND |
| Commit `20bc305f` exists | FOUND |
| fn `connect_keeps_open_on_missing_proxy_auth` in connect.rs | FOUND |
| fn `denied_endpoint_returns_403_and_audit` in reverse.rs | FOUND |
