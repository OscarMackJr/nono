---
phase: 89-proxy-hardening-sync
reviewed: 2026-06-20T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-proxy/src/connect.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/route.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 89: Code Review Report

**Reviewed:** 2026-06-20
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

The Phase 89 diff against `2cafdc56` is overwhelmingly test code. The only
production change is a single disjunct added to the proxy-activation predicate in
`proxy_runtime.rs`:

```rust
|| !prepared.custom_credentials.is_empty() // #1197 / D-07
```

This disjunct was added to **both** arms of the `active` computation: the
`NetworkMode::Blocked` warn-and-suppress arm (lines 91-108) and the non-blocked
activation arm (lines 110-118). I traced the fail-secure invariant end-to-end:

- Both `args.block_net` and `profile.network.block` route through
  `capability_ext.rs:1021/1176` → `caps.set_network_blocked(true)` →
  `NetworkMode::Blocked`.
- Therefore any block request lands in the `Blocked` arm, where the new disjunct
  only triggers the override **warning** and `active` is hard-coded to `false`.
- The new disjunct can only flip `active` to `true` in the non-blocked arm,
  where activating the credential-injecting proxy is the intended (and more
  secure) behavior — it ensures custom credentials are actually mediated by the
  proxy rather than silently dropped.

**The production change is correct and preserves fail-secure.** The two
regression tests (`proxy_activates_with_custom_credentials_only`,
`block_net_overrides_custom_credentials_activation`) correctly pin both arms.

No BLOCKER-class defects found. All findings below are test-quality and
robustness concerns in the newly added test code (per CLAUDE.md, `.unwrap()` /
`.expect()` are explicitly permitted in test modules, so those are not flagged).

## Warnings

### WR-01: `connect_keeps_open_on_missing_proxy_auth` can pass vacuously and is host-port-dependent

**File:** `crates/nono-proxy/src/connect.rs:419-516`
**Issue:** The test's three assertions are all *negative* ("NOT 407", "NOT
`InvalidToken`", "NOT `AuthenticationFailed`"). The handler under test
(`handle_connect`) never emits a 407 or `InvalidToken` on **any** code path —
auth failure is logged at `debug` and execution always continues
(`connect.rs:46-48`). The assertions are therefore satisfied even if the request
never reaches the auth check at all (e.g. if `parse_connect_target` failed, or if
the filter denied the host first). The test does not positively prove that the
keep-open path was exercised. Additionally, the test depends on the host
refusing a connection to `127.0.0.1:1` quickly; on a host where port 1 behaves
unusually (or a 30s `UPSTREAM_CONNECT_TIMEOUT` is hit), the test slows or its
intended path changes. It also relies on `read_to_end` draining cleanly after
`drop(server_stream)`, which is timing-sensitive relative to
`copy_bidirectional` startup.
**Fix:** Make the equivalence positive. Either (a) supply a CONNECT request that
*does* carry a well-formed-but-wrong `Proxy-Authorization` header and assert the
handler still proceeds (proving leniency, not just absence of a feature), or (b)
add a positive assertion that the handler reached the upstream-connect stage —
e.g. assert the 502 status line is present (`response.starts_with("HTTP/1.1 502")`)
so the test fails loudly if the request short-circuits before the auth/filter
stages:
```rust
assert!(
    response.starts_with("HTTP/1.1 502"),
    "expected upstream-connect failure path to be reached; got: {:?}",
    response
);
```

### WR-02: `denied_endpoint_returns_403_and_audit` asserts only on `events[0]`, not on category exclusivity

**File:** `crates/nono-proxy/src/reverse.rs:1357-1382`
**Issue:** The test asserts `events[0].decision == Deny` and
`events[0].denial_category == Some(EndpointPolicy)`. If the handler ever emits a
preceding audit event (e.g. an allow/observe entry) before the endpoint-deny
entry, `events[0]` would be the wrong record and the assertion would fail for the
right reason — but if it emits a *trailing* unexpected event, the test would
silently pass while masking a regression (e.g. a credential operation that should
not have run after a 403). The phase's own rationale states the 403 is sent
"BEFORE any credential operation"; the test does not assert that no
credential/allow event was emitted.
**Fix:** Assert the full event-stream shape, not just the first element:
```rust
assert_eq!(events.len(), 1, "exactly one audit event expected (deny, no cred op); got: {:?}", events);
assert!(
    events.iter().all(|e| e.decision == NetworkAuditDecision::Deny),
    "no allow/observe event must precede or follow the endpoint deny"
);
```

### WR-03: Duplicated async test harness across `connect.rs` and `reverse.rs` (drift risk)

**File:** `crates/nono-proxy/src/reverse.rs:1252-1259` (and `connect.rs:281-285`)
**Issue:** `reverse.rs` re-declares a `read_to_string<R: AsyncRead + Unpin>` helper
that is byte-for-byte the same as the one in `connect.rs:281-285`, and the
loopback `TcpListener` + `tokio::join!(connect, accept)` handshake is copy-pasted
across the two new tests (and the comment in `reverse.rs:1248` explicitly notes
"copied from connect.rs tests"). Copy-pasted test scaffolding tends to diverge:
a fix to draining/EOF semantics in one copy will not propagate to the other,
which is exactly the failure mode WR-01 is exposed to.
**Fix:** Hoist the shared loopback-pair + `read_to_string` helpers into a small
`#[cfg(test)] mod test_support` (or a `tests/common`-style internal module) and
have both call sites use it. Low priority but reduces future maintenance drift.

## Info

### IN-01: `proxy_activates_with_custom_credentials_only` comment asserts an implementation detail that should be pinned

**File:** `crates/nono-cli/src/proxy_runtime.rs:499-500`
**Issue:** The doc-comment states "`CapabilitySet::new()` defaults to
`NetworkMode::Open` ... so only the custom_credentials disjunct can trigger
active=true here." This is load-bearing for the test's validity — if a future
change makes `CapabilitySet::new()` default to `ProxyOnly`, the test would pass
for the wrong reason (the `ProxyOnly` disjunct, not custom_credentials). The
assumption is documented but not enforced.
**Fix:** Add a one-line guard so the assumption fails loudly if it ever changes:
```rust
assert!(
    matches!(prepared.caps.network_mode(), nono::NetworkMode::Open),
    "test premise: only the custom_credentials disjunct may drive activation"
);
```

### IN-02: Large `PreparedSandbox` struct literal duplicated across three tests

**File:** `crates/nono-cli/src/proxy_runtime.rs:406-435, 526-555, 595-624`
**Issue:** The ~30-field `PreparedSandbox { .. }` literal is fully repeated in
three tests with only `caps`, `custom_credentials`, and
`network_block_requested` varying. Any new field added to `PreparedSandbox` must
be edited in all three places (and any of the other call sites in the codebase),
a known churn point. This is a maintainability smell, not a defect.
**Fix:** Introduce a `#[cfg(test)]` builder/`Default`-style helper
(`fn test_prepared(caps, custom_creds, block) -> PreparedSandbox`) and have the
three tests call it.

### IN-03: `route.rs` shadow-disproof test's final assertion is informational, not a guard

**File:** `crates/nono-proxy/src/route.rs:679` (`is_route_upstream` assertion)
**Issue:** The closing `assert!(store.is_route_upstream("api.openai.com:443"))`
plus its preceding comment ("does not imply any shadowing in route dispatch")
documents that `is_route_upstream` is host-keyed while *dispatch* is prefix-keyed.
The disjoint-key disproof (the real point of the test) is fully carried by the
earlier `store.get("openai")` / `store.get("_ep_api.openai.com")` assertions; the
trailing line adds little disproof value and could mislead a future reader into
thinking host-keyed lookup is part of the selection path. Harmless as written.
**Fix:** None required. Optionally add a comment clarifying that
`is_route_upstream` is used only for upstream-allowlist gating, not route
selection, to forestall misreading.

---

_Reviewed: 2026-06-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
