---
phase: 56-fine-grained-network-filtering
plan: "02"
subsystem: network-proxy-wiring
tags: [AllowDomainEntry, proxy_runtime, partition_allow_domain, C5-rider, domain_endpoints, type-cascade]
dependency_graph:
  requires:
    - AllowDomainEntry enum (56-01)
    - partition_allow_domain fn (56-01)
    - DomainEndpointState + from_caps 4-arg (56-01)
  provides:
    - parse_allow_domain_arg fn in proxy_runtime.rs
    - EffectiveProxySettings.allow_domain: Vec<AllowDomainEntry>
    - ProxyLaunchOptions.allow_domain: Vec<AllowDomainEntry>
    - build_proxy_config_from_flags with partition_allow_domain + C5 rider
    - write_capability_state_file with domain_endpoints 4th arg (removes Plan 01 stub)
    - main.rs test fixtures updated to AllowDomainEntry::Plain(...)
  affects:
    - query_ext.rs (Plans 03/04)
    - why_runtime.rs (Plans 03/04)
    - profile_cmd.rs (Plans 03/04)
tech_stack:
  added: []
  patterns:
    - "url::Url::parse(input).host_str() URL-to-domain extraction"
    - "C5 rider: endpoint route upstreams pushed to plain_hosts for TLS-intercept TCP"
    - "Iterator::filter_map for WithEndpoints extraction into DomainEndpointState"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/main.rs
decisions:
  - "PreparedSandbox.allow_domain remains Vec<String>; resolve_effective_proxy_settings converts via parse_allow_domain_arg — avoids a broader type cascade into the manifest path"
  - "parse_allow_domain_arg: URL-with-non-root-path → WithEndpoints (single wildcard method rule); root or no path → Plain; parse failure → Plain (safe fallback)"
  - "domain_endpoints derivation lives at the call site rather than inside write_capability_state_file — keeps function focused on state serialization, not type conversion"
  - "sandbox_prepare.rs had no new changes needed — Plan 01 deviation already applied domain() extraction adapters"
metrics:
  duration_minutes: 25
  completed_date: "2026-06-05"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 4
---

# Phase 56 Plan 02: Proxy Runtime AllowDomainEntry Wiring Summary

**One-liner:** Wire AllowDomainEntry through proxy_runtime (parse_allow_domain_arg + C5 rider) and complete the cascade in execution_runtime + main.rs, replacing the Plan 01 stubs.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | proxy_runtime + launch_runtime AllowDomainEntry wiring + C5 rider | 6b2353c6 | proxy_runtime.rs, launch_runtime.rs |
| 2 | execution_runtime + main.rs AllowDomainEntry cascade completion | 4c13b90b | execution_runtime.rs, main.rs |

## What Was Built

### Task 1: proxy_runtime.rs + launch_runtime.rs

**proxy_runtime.rs:**
- Added `parse_allow_domain_arg(input: &str) -> AllowDomainEntry` (upstream 75b2265):
  - URL with non-root path → `WithEndpoints { domain, endpoints: [{method:"*", path}] }`
  - URL with root/empty path → `Plain(domain)`
  - URL parse failure → `Plain(input)` (safe fallback)
- Changed `EffectiveProxySettings.allow_domain` from `Vec<String>` to `Vec<AllowDomainEntry>`
- Updated `resolve_effective_proxy_settings`: converts `PreparedSandbox.allow_domain: Vec<String>` via `parse_allow_domain_arg`, then extends with CLI `--allow-domain` args via same parser
- Replaced `expand_proxy_allow` call with `partition_allow_domain` in `build_proxy_config_from_flags`
- Applied C5 rider (upstream 22e6c40): after `partition_allow_domain`, strips `https://`/`http://` prefixes from endpoint route upstreams and pushes them to `plain_hosts` so the proxy filter allowlist allows upstream TCP connections for TLS-intercept routes
- Added 5 unit tests for `parse_allow_domain_arg` (plain hostname, URL+path, root URL, no-path URL, unparseable input)

**launch_runtime.rs:**
- Changed `ProxyLaunchOptions.allow_domain` from `Vec<String>` to `Vec<crate::profile::AllowDomainEntry>`

**profile_runtime.rs:** Already updated in Plan 01 deviation — no change needed.

### Task 2: execution_runtime.rs + main.rs

**execution_runtime.rs:**
- Added imports for `AllowDomainEntry`, `DomainEndpointState`, `EndpointRuleState`
- At `write_capability_state_file` call site: derived `domain_endpoints: Vec<DomainEndpointState>` by filtering `flags.proxy.allow_domain` for `WithEndpoints` entries and mapping to `DomainEndpointState`; derived `plain_allowed_domains: Vec<String>` via `.domain().to_string()`
- Updated `write_capability_state_file` signature to add `domain_endpoints: &[DomainEndpointState]` parameter
- Removed `TODO(56-02)` stub — now passes real `domain_endpoints` to `from_caps` 4th arg

**main.rs:**
- Updated `EffectiveProxySettings` test assertion (line ~348) to use `AllowDomainEntry::Plain(...)` instead of bare `String`

**sandbox_prepare.rs:** No changes needed — Plan 01 deviation already applied `.domain().to_string()` extraction at both `print_allow_domain_port_warnings` call sites and the `PreparedSandbox` construction.

## Deviations from Plan

### None — plan executed exactly as written

The only "deviation" is that `sandbox_prepare.rs` required no changes: Plan 01's deviation (Rule 3 — blocking type cascade) had already applied the `.domain()` extraction adapters at all three call sites. Plan 02 correctly identified this in the context note and `read_first` instructions. No rework needed.

## Verification Results

- `cargo check --bin nono 2>&1 | grep "^error"` — zero errors (profile_cmd.rs, query_ext.rs, why_runtime.rs all compile clean too; Plan 03/04 scope)
- `cargo test -p nono-cli -- parse_allow_domain allow_domain partition` — all 26 matching tests pass
- `grep "parse_allow_domain_arg" crates/nono-cli/src/proxy_runtime.rs` — function present
- `grep "partition_allow_domain" crates/nono-cli/src/proxy_runtime.rs` — replacement call present
- `grep "plain_hosts.push" crates/nono-cli/src/proxy_runtime.rs` — C5 rider present
- `grep "Vec<crate::profile::AllowDomainEntry>" crates/nono-cli/src/profile_runtime.rs` — type updated (Plan 01)
- `grep "Vec<crate::profile::AllowDomainEntry>" crates/nono-cli/src/launch_runtime.rs` — type updated
- `grep "domain_endpoints" crates/nono-cli/src/execution_runtime.rs` — derivation + 4th arg present
- `grep "AllowDomainEntry::Plain" crates/nono-cli/src/main.rs` — fixture updated
- `credential.rs` not modified in this session (D-04 invariant: SHA unchanged from base)

## Known Stubs

None — all Plan 01 stubs in this plan's scope are resolved:
- `execution_runtime.rs TODO(56-02)` — removed; `domain_endpoints` now derived and passed
- `sandbox_prepare.rs .domain() extraction` — already wired in Plan 01, confirmed correct

Remaining stubs (out of Plan 02 scope, tracked for Plans 03/04):
- `why_runtime.rs` — plain domain extraction passed to `expand_proxy_allow` (Plan 03/04)
- `query_ext.rs` — `query_network` still uses `&[String]` for domain_endpoints (Plan 03/04)
- `profile_cmd.rs` — display rendering still uses `.domain()` adapter (Plan 03/04)

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes. The C5 rider (`plain_hosts.push`) operates on operator-supplied endpoint route upstreams only — no new attack surface beyond what the operator explicitly configured (T-56-06 accepted per plan threat register).

## Self-Check: PASSED
