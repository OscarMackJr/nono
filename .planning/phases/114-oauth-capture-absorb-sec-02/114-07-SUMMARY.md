---
phase: 114-oauth-capture-absorb-sec-02
plan: 07
subsystem: network-proxy
tags: [oauth-capture, sec-02, fail-closed, cross-path-guard, d-06, wr-13]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-01's NetworkAuditDenialCategory::CaptureUnsupportedPath"
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-03's RouteStore::capture_declared_for_upstream() host-only predicate"
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plans 114-05/114-06's reverse-proxy enforcement point (the path this guard redirects toward)"
provides:
  - "handle_forward_http denies any request whose upstream matches a capture-declared route (the phase's one genuinely new guard)"
  - "CONNECT dispatch emits denial_category=CaptureUnsupportedPath for capture-route bypass attempts, distinguishable from a plain route-upstream denial"
affects: [114-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-path fail-closed guard: a capability implemented on ONE request path must be explicitly denied on every OTHER path that lacks it, rather than left implicitly unreachable. Mirrors Phase 113's D-03 SPIFFE guard."
    - "Guard is unconditional — deliberately NOT gated on `state.config.require_auth`. 'This route only makes sense through the reverse-proxy path' does not depend on whether session-token auth happens to be enabled."
    - "RED/GREEN verification of a security predicate's WIDTH, not just its presence: temporarily narrowing the matcher must break exactly the test that covers the widening, and nothing else."

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/server.rs
---

# Plan 114-07 Summary — the D-06 cross-path fail-closed guard

## What was built

Plans 114-05 and 114-06 built the only response buffer-and-rewrite enforcement point and wired it
into all three `reverse.rs` relay sites. This plan closes the remaining hole: an agent reaching a
capture-declared token endpoint by a **different request path**, where no rewrite exists at all.
That is the WR-13 defect shape verbatim (`112-REVIEW.md`) — one request path of several ignoring a
gate the others have — and it is a **fork-original** test obligation, since upstream ships no
equivalent (upstream has TLS interception on those paths; this fork declines MITM, D-01r).

The two paths are deliberately **not** symmetric, and the plan was right about the asymmetry:

**1. `handle_forward_http` — the one genuinely new guard** (`crates/nono-proxy/src/server.rs:1096`).
Denies with 403 when `state.route_store.capture_declared_for_upstream(&host_port)` matches, emitting
`denial_category: CaptureUnsupportedPath`. It is **unconditional** — explicitly not gated on
`state.config.require_auth`, documented in-source, mirroring Phase 113's D-03 SPIFFE guard where
that property was called out as load-bearing. A guard that only fired when auth was enabled would
be a fail-open for `--no-auth` standalone proxies.

**2. CONNECT — no new deny logic, audit refinement only** (`server.rs:1378-1401`). The plan's claim
that `is_route_upstream()` already blocks CONNECT unconditionally was **verified at source rather
than inherited**: `if state.route_store.is_route_upstream(&host_port) {` at `:1378` gates the block
with no additional condition, and the capture check added inside it only selects which
`denial_category`/`denial_reason` the resulting audit event carries. No second deny was added.

## Commits

| Commit | Content |
|---|---|
| `0bd472cb` | D-06 cross-path fail-closed guard for capture-declared routes (+ CONNECT audit refinement) |
| `5131556f` | `cargo fmt` fix on the new negative-control test |
| `7bc814d2` | Prove D-06 host-only widening at the guard layer |

## The host-only widening — and why the first two commits weren't enough

D-06's load-bearing property is that `capture_declared_for_upstream()` matches **HOST-ONLY**,
deliberately diverging from the SPIFFE sibling's exact `host:port` matching. The widening is the
security property: a capture-declared host reachable on a *different* port is still caught. That is
the bypass class upstream's own `3c59c62e` hardening commit closed.

The plan's first two commits shipped three guard tests — but **all three declare and request the
same port** (`capture-upstream.invalid:9443`). So at the guard layer the widening was untested. It
was covered one layer down by `route.rs::test_capture_declared_for_upstream_true_across_different_port`,
but that proves the *predicate*, not that the *guard consults it*: a guard narrowed to `host:port`
would still pass the route.rs test while being fail-open at `handle_forward_http`.

`7bc814d2` adds `forward_http_denies_capture_declared_route_upstream_on_different_port` — route
declares `:9443`, request reaches the same host on `:443`, still denied 403 with
`CaptureUnsupportedPath`, `require_auth: false`.

### RED/GREEN proof that the new test is not vacuous

Not asserted by inspection — executed:

1. Temporarily replaced `host_only_matches(hp, &normalised)` with `host_port_matches(hp, &normalised)`
   inside `capture_declared_for_upstream()` (i.e. introduced the exact fail-open narrowing D-06
   forbids).
2. `cargo test -p nono-sandbox-proxy --lib forward_http_denies_capture_declared_route_upstream`
   → **1 passed, 1 FAILED**. Only the new different-port test broke; the same-port test still
   passed. That is precisely the discrimination required — it proves the new test catches a
   regression that no existing test would.
3. Reverted. `git diff --stat crates/nono-proxy/src/route.rs` empty; both tests green again.

## Verification

| Gate | Result |
|---|---|
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | exit 0 |
| `cargo test -p nono-sandbox-proxy --lib` | 296 passed, 0 failed (292 baseline + 4 new) |

Tests added by this plan:

- `forward_http_denies_capture_declared_route_upstream` — the guard fires, `require_auth: false`
- `forward_http_denies_capture_declared_route_upstream_on_different_port` — **D-06 host-only widening**
- `forward_http_non_capture_route_still_forwards` — negative control: the guard checks
  `declares_capture` specifically, not merely "some route exists at this upstream"
- `connect_denies_capture_declared_route_upstream` — regression proof the pre-existing
  `is_route_upstream()` block still holds, with the refined `denial_category`

## Deviations from plan

1. **The plan's test set omitted the host-only widening case at the guard layer.** Added as a third
   commit after the orchestrator's review flagged it. The plan named the widening as load-bearing in
   prose but its acceptance criteria did not require a test that would fail without it.
2. **The CONNECT "no new deny logic" claim was verified rather than assumed.** The plan instructed
   stopping and reporting if `is_route_upstream()` turned out not to block unconditionally. It does;
   confirmed at `server.rs:1378`.

## Provenance note

This plan's executor was interrupted by an operator stop after landing `0bd472cb` and `5131556f`
with a clean tree. The orchestrator verified the landed work, identified the missing widening test,
added it with the RED/GREEN proof above, and wrote this SUMMARY.

## Self-Check: PASSED

Both cross-paths are now fail-closed against capture-declared upstreams, and the host-only property
that makes the guard wider-than-SPIFFE is proven at the guard layer by a test demonstrated to fail
when the property is removed.
