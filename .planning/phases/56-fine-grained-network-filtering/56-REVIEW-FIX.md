---
phase: 56-fine-grained-network-filtering
fixed_at: 2026-06-05T00:00:00Z
review_path: .planning/phases/56-fine-grained-network-filtering/56-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 3
skipped: 0
status: partial
---

# Phase 56: Code Review Fix Report

**Fixed at:** 2026-06-05
**Source review:** .planning/phases/56-fine-grained-network-filtering/56-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-01, CR-02, WR-01, WR-02, WR-03)
- Fixed: 3 (CR-01, CR-02, WR-01)
- Skipped: 2 (WR-02, WR-03 — out of scope for this run; only CR-01/CR-02/WR-01 were requested)

## Fixed Issues

### CR-01: Profile-defined `allow_domain` endpoint rules are silently dropped (fail-open)

**Files modified:** `crates/nono-cli/src/sandbox_prepare.rs`, `crates/nono-cli/src/proxy_runtime.rs`, `crates/nono-cli/src/main.rs`
**Commit:** `05cd7580`
**Applied fix:**
- Changed `PreparedSandbox.allow_domain` from `Vec<String>` to `Vec<crate::profile::AllowDomainEntry>`.
- On the manifest path in `sandbox_prepare.rs`: the manifest's `Vec<String>` `allow_domains` is now converted to `Vec<AllowDomainEntry::Plain(...)>` entries before assignment.
- On the profile path in `sandbox_prepare.rs`: assignment at line 508 changed from `.iter().map(|e| e.domain().to_string()).collect()` to simply `profile_allow_domain` (direct move, no flattening).
- In `proxy_runtime.rs` `resolve_effective_proxy_settings`: replaced the re-parsing loop (`prepared.allow_domain.iter().map(|s| parse_allow_domain_arg(s))`) with a direct clone (`prepared.allow_domain.clone()`). CLI `--allow-domain` args are still parsed via `parse_allow_domain_arg`.
- Fixed two test fixtures in `main.rs` that constructed `PreparedSandbox` with literal `Vec<String>`.
- Added test `resolve_effective_proxy_settings_preserves_with_endpoints` in `proxy_runtime.rs` asserting a two-rule `WithEndpoints` entry survives end-to-end.

### CR-02: `host:port` allow-domain entries are mangled into bogus endpoint routes

**Files modified:** `crates/nono-cli/src/proxy_runtime.rs`
**Commit:** `05cd7580` (included in CR-01 commit — changes to `parse_allow_domain_arg` were staged together)
**Applied fix:**
- Added a `looks_like_url` guard in `parse_allow_domain_arg`: URL parsing is now only attempted when the input starts with `http://` or `https://`.
- All other inputs (bare hostnames, `host:port`) fall through to `AllowDomainEntry::Plain(input.to_string())` without any `url::Url::parse` call.
- For real URL inputs the behavior is unchanged: non-root path produces `WithEndpoints`, root/empty path produces `Plain`.
- Added tests `parse_allow_domain_host_port_yields_plain` and `parse_allow_domain_host_port_443_yields_plain`.

### WR-01: `is_loopback_domain` uses a string prefix that misclassifies real domains

**Files modified:** `crates/nono-cli/src/network_policy.rs`
**Commit:** `c4931750`
**Applied fix:**
- Replaced the `domain.starts_with("127.")` (and companion `"0.0.0.0"` / `"::1"`) string comparisons with parsed IP semantics using `std::net::IpAddr`.
- New logic: exact equality for `"localhost"`; for anything else, attempt `domain.parse::<std::net::IpAddr>()` and test `ip.is_loopback() || ip.is_unspecified()`. `is_loopback()` covers the full `127.0.0.0/8` block without misclassifying `127.example.com`.
- Added eight tests covering correct/incorrect classification: `127.example.com` (public, NOT loopback), `127.0.0.1`, `127.1.2.3`, `127.255.255.255`, `localhost`, `::1`, `0.0.0.0`, and an integration test `partition_allow_domain_127_example_com_uses_https_scheme` confirming `127.example.com` routes get `https://` upstream.

## Skipped Issues

### WR-02: `nono why` endpoint display can diverge from proxy enforcement

**File:** `crates/nono-cli/src/query_ext.rs:336-373`
**Reason:** Out of scope for this fix run — not listed in the `<fixes_to_apply>` block. Requires either reusing `CompiledEndpointRules` or adding a `--method` flag to `nono why`.
**Original issue:** `path_matches_endpoint_rules` ignores rule `method`, so `nono why` over-reports "allowed" for method-scoped rules.

### WR-03: Profile + CLI `allow_domain` for the same host not merged at runtime

**File:** `crates/nono-cli/src/proxy_runtime.rs:144-149`, `crates/nono-cli/src/network_policy.rs:358-405`
**Reason:** Out of scope for this fix run — not listed in the `<fixes_to_apply>` block. Requires running the concatenated runtime list through `merge_allow_domain` before `partition_allow_domain`.
**Original issue:** Same host can end up simultaneously in `plain_hosts` and an endpoint route, creating contradictory state.

---

_Fixed: 2026-06-05_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
