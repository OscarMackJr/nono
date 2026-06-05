---
phase: 56-fine-grained-network-filtering
verified: 2026-06-05T00:00:00Z
status: human_needed
score: 4/4 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run nono with a profile containing a WithEndpoints allow_domain entry and attempt a request on a denied path"
    expected: "The proxy returns 403 for the disallowed path; the allowed path passes. Confirms enforcement is live in the proxy execution path."
    why_human: "Requires a running proxy + sandboxed child to exercise the enforcement path end-to-end. Unit tests confirm wiring but cannot substitute for a live proxy run."
  - test: "Run `nono why --host https://api.example.com/v1` against a profile with scoped endpoint rules"
    expected: "Output shows 'Endpoint rules:' section listing method+path entries for the domain"
    why_human: "The nono why display requires a running nono binary invocation and a profile file; cannot be verified from static grep alone."
  - test: "Confirm WR-02 (method blindness in nono why display) is acceptable as a non-blocking follow-up"
    expected: "Team acknowledges that path_matches_endpoint_rules ignores rule method in the nono why display (diagnostic only; enforcement in proxy uses method check). If this is unacceptable, promote WR-02 to a gap before shipping."
    why_human: "This is a policy/usability judgment — the display approximates enforcement but diverges on method matching. The REVIEW.md flags it as advisory; only a human can decide whether the divergence is acceptable pre-ship."
  - test: "Confirm WR-03 (profile+CLI same-host merge race) is acceptable as a non-blocking follow-up"
    expected: "Team acknowledges that when a domain appears as both Plain (from profile) and WithEndpoints (from CLI --allow-domain), both entries reach partition_allow_domain independently without merging. On this fork the CONNECT block in server.rs makes the state fail-secure, but the logic is fragile."
    why_human: "Requires an operator to review the security tradeoff and confirm the fail-secure CONNECT-block invariant is sufficient protection until WR-03 is addressed."
---

# Phase 56: Fine-grained Network Filtering Verification Report

**Phase Goal:** Fine-grained network filtering — `allow_domain` accepts URL path scoping and fine-grained HTTP method+path restrictions enforced in the proxy path, with TLS-intercept endpoint rules evaluated before credential selection, and `nono why --host` awareness of the new scoping preserved.
**Verified:** 2026-06-05T00:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `allow_domain` accepts structured endpoint objects with path+method alongside plain hostname strings, backward-compatible | VERIFIED | `AllowDomainEntry` enum with `#[serde(untagged)]` at `profile/mod.rs:31`; 14 deserialization tests pass including mixed-array compat |
| 2 | Profile-defined endpoint rules survive end-to-end to the proxy's `RouteConfig` (CR-01 fix present) | VERIFIED | `PreparedSandbox.allow_domain: Vec<AllowDomainEntry>` at `sandbox_prepare.rs:72`; `resolve_effective_proxy_settings` clones directly at `proxy_runtime.rs:155`; regression test `resolve_effective_proxy_settings_preserves_with_endpoints` present |
| 3 | TLS-intercept endpoint rules are evaluated before credential selection | VERIFIED | `nono-proxy/src/reverse.rs:96` — `endpoint_rules.is_allowed()` check at line 96, `credential_store.get()` at line 119. Pre-existing proxy enforcement; Phase 56 wires CLI layer to populate `endpoint_rules` in `RouteConfig` via `partition_allow_domain`. |
| 4 | `nono why --host` surfaces path/method scoping rules (SC3) | VERIFIED | `parse_host_input` + `query_network` (5-arg) + `print_result` Endpoint rules display in `query_ext.rs`; `WhyContext.domain_endpoints` populated and passed in `why_runtime.rs:197`; 9 new tests pass |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/profile/mod.rs` | AllowDomainEntry enum, merge_allow_domain fn, Vec<AllowDomainEntry> field | VERIFIED | Lines 31, 1619, 3195 — all present with tests |
| `crates/nono-cli/src/network_policy.rs` | partition_allow_domain fn, is_loopback_domain (IpAddr semantics) | VERIFIED | Lines 363, 344 — WR-01 fix (IpAddr parsing) present at line 348 |
| `crates/nono-cli/src/sandbox_state.rs` | DomainEndpointState, EndpointRuleState, domain_endpoints field, 4-arg from_caps | VERIFIED | Lines 48, 59, 41, 89 — all present with serialization tests |
| `crates/nono-cli/Cargo.toml` | globset + urlencoding deps | VERIFIED | Both deps present |
| `crates/nono-cli/src/proxy_runtime.rs` | parse_allow_domain_arg (http(s):// guard), build_proxy_config_from_flags with partition_allow_domain + C5 rider | VERIFIED | Lines 36-57 (CR-02 guard), 209-218 (C5 rider) — present |
| `crates/nono-cli/src/execution_runtime.rs` | domain_endpoints derivation + from_caps 4th arg | VERIFIED | Lines 164-197 — derivation + call site present |
| `crates/nono-cli/src/query_ext.rs` | parse_host_input, path_matches_endpoint_rules, query_network 5-arg, endpoint_rules field | VERIFIED | Lines 372, 407, 245, 57, 72 — all present with 9 new tests |
| `crates/nono-cli/src/why_runtime.rs` | WhyContext.domain_endpoints, resolve_domain_endpoints, query_network 5-arg call | VERIFIED | Lines 11, 49, 192-198 — all present |
| `crates/nono-cli/data/nono-profile.schema.json` | AllowDomainWithEndpoints $defs + oneOf allow_domain items | VERIFIED | Lines 672, 469 — present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `profile/mod.rs` | `nono-proxy/src/config.rs` | `use nono_proxy::config::EndpointRule` in AllowDomainEntry::WithEndpoints | VERIFIED | Import present; EndpointRule is the proxy type |
| `sandbox_prepare.rs` | `proxy_runtime.rs` | `PreparedSandbox.allow_domain: Vec<AllowDomainEntry>` cloned directly (no lossy flatten) | VERIFIED | CR-01 fix confirmed at proxy_runtime.rs:155 |
| `proxy_runtime.rs` | `network_policy.rs` | `partition_allow_domain` call replaces `expand_proxy_allow` | VERIFIED | proxy_runtime.rs:210 |
| `proxy_runtime.rs` | `nono-proxy/src/config.rs` | `endpoint_routes` added to `resolved.routes` via C5 rider | VERIFIED | proxy_runtime.rs:218 |
| `execution_runtime.rs` | `sandbox_state.rs` | `from_caps` 4th arg `domain_endpoints` | VERIFIED | execution_runtime.rs:197 |
| `why_runtime.rs` | `query_ext.rs` | `query_network` call passes `&ctx.domain_endpoints` as 5th arg | VERIFIED | why_runtime.rs:197 |
| `nono-proxy/src/reverse.rs` | credential selection | endpoint check (line 96) precedes `credential_store.get` (line 119) | VERIFIED | Ordering confirmed in codebase; unchanged by Phase 56 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `proxy_runtime.rs::build_proxy_config_from_flags` | `endpoint_routes: Vec<RouteConfig>` | `partition_allow_domain(&net_policy, &proxy.allow_domain)` | Yes — constructed from operator-supplied AllowDomainEntry::WithEndpoints entries | FLOWING |
| `execution_runtime.rs::write_capability_state_file` | `domain_endpoints: Vec<DomainEndpointState>` | filter_map over `flags.proxy.allow_domain` at line 166 | Yes — extracts real WithEndpoints entries | FLOWING |
| `query_ext.rs::query_network` | `endpoint_rules: Option<Vec<EndpointRuleState>>` | `domain_endpoints.iter().find(|de| de.domain.eq_ignore_ascii_case(&domain))` | Yes — matched from caller-provided slice | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| parse_allow_domain_arg: 7 cases (plain, URL+path, root URL, host:port, host:port:443, unparseable, no-path URL) | `cargo test -p nono-cli -- parse_allow_domain` | 7 passed, 0 failed | PASS |
| partition_allow_domain, AllowDomainEntry deserialization, merge_allow_domain, DomainEndpointState (full unit test suite) | `cargo test -p nono-cli` | 1190 passed, 4 known pre-existing failures (unrelated to Phase 56) | PASS |
| nono-proxy endpoint enforcement, reverse handler, audit tests | `cargo test -p nono-proxy` | 161 passed, 0 failed | PASS |
| Windows clippy -D warnings -D clippy::unwrap_used | `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` | 0 errors, 0 warnings | PASS |
| Cross-target Linux/macOS clippy | Deferred — ring crate fails to build on Windows host | PARTIAL — deferred to CI per CLAUDE.md cross-target verify checklist |

### Probe Execution

No probe scripts declared for Phase 56. Step 7c: SKIPPED (no probe-*.sh files in phase directory).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| REQ-NET-01 | 56-01, 56-02, 56-03, 56-04 | `--allow-domain` URL path+method restrictions in proxy, TLS-intercept ordering, nono why awareness, diff-inspect | SATISFIED | AllowDomainEntry + partition_allow_domain + C5 rider + endpoint check before credential at reverse.rs:96 + parse_host_input + query_network SC3 display |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| No TBD/FIXME/XXX debt markers found in any phase-modified file | — | — | — | — |
| `query_ext.rs::path_matches_endpoint_rules` | 407 | Method-blind glob matching — documented in code comment ("diagnostic only, not access control") | Warning (WR-02 advisory from REVIEW.md) | Display over-reports "allowed" for method-scoped rules; enforcement in proxy uses method check. Non-blocking per code review disposition. |
| `proxy_runtime.rs::resolve_effective_proxy_settings` | 155 | Profile and CLI allow_domain concatenated without merge — same host can appear as both Plain and WithEndpoints | Warning (WR-03 advisory from REVIEW.md) | Contradictory state resolved fail-secure by CONNECT block in server.rs, but fragile. Non-blocking per code review disposition. |

### Human Verification Required

1. **Live Proxy Enforcement**

   **Test:** Run `nono run --allow-domain https://api.example.com/v1/repos/** -- <http-client-tool>` and attempt requests to `/v1/repos/x` (allowed) and `/v2/admin` (denied)
   **Expected:** `/v1/repos/x` succeeds; `/v2/admin` receives HTTP 403 from the proxy. The audit log shows an EndpointPolicy denial for the blocked path.
   **Why human:** Requires a running proxy + sandboxed child + real HTTP request. Static analysis confirms the wiring is correct but cannot substitute for live execution.

2. **nono why --host URL Output**

   **Test:** With a profile containing `allow_domain: [{domain:"api.example.com", endpoints:[{method:"GET",path:"/v1/**"}]}]`, run `nono why --profile <profile> --host https://api.example.com/v1/test`
   **Expected:** Output includes "Endpoint rules:" section listing `GET /v1/**`. Running `nono why --host https://api.example.com/v2/other` shows denied with "endpoint_restricted" reason.
   **Why human:** Requires a real nono binary invocation with a profile file. The code paths are verified but the formatted terminal output cannot be confirmed statically.

3. **WR-02 Method-blind display policy decision**

   **Test:** Review whether `path_matches_endpoint_rules` ignoring rule method in `nono why` output is acceptable for Phase 56 scope.
   **Expected:** Team confirms WR-02 is a known follow-up (advisory, non-blocking) OR escalates it to a gap requiring a fix before this phase is accepted.
   **Why human:** Security usability judgment — `nono why` is the primary operator reasoning tool. The code comment documents the divergence, but only a human can decide if it is acceptable pre-ship.

4. **WR-03 Same-host merge race policy decision**

   **Test:** Review whether the profile+CLI same-host Plain/WithEndpoints contradictory state resolved by CONNECT block is an acceptable pre-ship state.
   **Expected:** Team confirms WR-03 is a known follow-up OR escalates it to a gap.
   **Why human:** Requires architectural judgment about whether the CONNECT-block fail-secure invariant is robust enough before the merge fix lands.

### Gaps Summary

No automated gaps found. All 4 ROADMAP success criteria are verified as present in the codebase:

- SC1 (path/method restriction enforced by proxy): `partition_allow_domain` feeds `WithEndpoints` entries as `RouteConfig` with `endpoint_rules`; pre-existing `CompiledEndpointRules::is_allowed` in `nono-proxy` enforces them. CR-01 (profile endpoint rules were silently dropped) and CR-02 (host:port mangling) were fixed before verification.
- SC2 (TLS-intercept endpoint rules before credential selection): `reverse.rs:96` has the check before `reverse.rs:119` credential lookup; ordering is structural and unchanged.
- SC3 (nono why --host surfacing): `parse_host_input` + `query_network` 5-arg + `print_result` endpoint_rules display wired end-to-end.
- SC4 (Phase 34 C11 fork-preserve): `credential.rs` not modified; Upstream-commit trailers (0ced085, 75b2265, 22e6c40) present on all relevant commits; proxy-absent tls_client_cert/tls_client_key fields excluded from fork's RouteConfig construction.

Two open advisory items from the code review (WR-02 method-blind display, WR-03 same-host merge) are non-blocking but require human sign-off before the phase can be marked fully accepted.

---

_Verified: 2026-06-05T00:00:00Z_
_Verifier: Claude (gsd-verifier)_
