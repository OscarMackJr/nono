---
phase: 56-fine-grained-network-filtering
plan: "04"
subsystem: network-policy-query
tags: [query_ext, why_runtime, parse_host_input, domain_endpoints, endpoint_rules, SC3]
dependency_graph:
  requires:
    - AllowDomainEntry + DomainEndpointState (56-01)
    - proxy_runtime AllowDomainEntry wiring (56-02)
    - profile_cmd display + schema (56-03)
  provides:
    - parse_host_input fn in query_ext.rs
    - normalize_path fn in query_ext.rs
    - path_matches_endpoint_rules fn in query_ext.rs
    - query_network 5th param (domain_endpoints) in query_ext.rs
    - QueryResult::Allowed endpoint_rules field
    - QueryResult::Denied endpoint_rules field
    - resolve_domain_endpoints fn in why_runtime.rs
    - WhyContext.domain_endpoints field
    - Full Phase 56 SC3 user-visible diagnostic face
  affects:
    - nono why --host (URL parsing + endpoint rules display)
tech_stack:
  added: []
  patterns:
    - "url::Url::parse + host_str() for URL-to-domain extraction with lowercase normalization"
    - "urlencoding::decode_binary + split('/') for path normalization (mirrors proxy)"
    - "globset::Glob::new + compile_matcher() for diagnostic endpoint rule matching"
    - "filter_map on WithEndpoints for domain_endpoints extraction from profile"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/query_ext.rs
    - crates/nono-cli/src/why_runtime.rs
decisions:
  - "Use EndpointRuleState directly (not a separate EndpointRuleDisplay) — matches upstream 75b2265 exact shape"
  - "endpoint_rules added to both Allowed and Denied variants — Denied arm shows rules for endpoint_restricted results"
  - "parse_host_input uses to_lowercase() for domain — case-insensitive host matching"
  - "normalize_path strips query strings before glob matching — mirrors proxy behavior"
  - "resolve_domain_endpoints is a separate helper (not modifying resolve_allowed_domains) — follows upstream 0ced085 exact pattern"
  - "Cross-target Linux clippy: PARTIAL (ring build fails on Windows host, deferred to CI)"
metrics:
  duration_minutes: 25
  completed_date: "2026-06-05"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 2
---

# Phase 56 Plan 04: SC3 query_ext + why_runtime Endpoint Rules Summary

**One-liner:** SC3 complete — `nono why --host` parses URLs, matches endpoint rules via globset, and displays allowed endpoint scoping rules via parse_host_input + path_matches_endpoint_rules + query_network 5-arg signature.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | query_ext.rs — parse_host_input + path_matches_endpoint_rules + query_network extension + endpoint_rules field | e4af06be | query_ext.rs |
| 2 | why_runtime.rs extension + full verification sweep | bf1d687c | why_runtime.rs |

## What Was Built

### Task 1: query_ext.rs Extensions

**New functions:**
- `parse_host_input(input: &str) -> (String, Option<String>)` — extracts `(domain, Option<path>)` from a bare hostname or URL. Domain lowercased. Path is `None` for root `/` or empty.
- `normalize_path(path: &str) -> String` — strips query strings, URL-decodes via `urlencoding::decode_binary`, collapses empty segments. Mirrors proxy `config.rs::normalize_path` behavior for diagnostic display.
- `path_matches_endpoint_rules(path: &str, rules: &[EndpointRuleState]) -> bool` — returns `true` if rules is empty (no scoping) or any rule's path glob matches the normalized path via `globset::Glob::new`.

**QueryResult extensions:**
- `QueryResult::Allowed` gains `endpoint_rules: Option<Vec<EndpointRuleState>>` with `#[serde(skip_serializing_if = "Option::is_none")]` — populated when host has scoped entries.
- `QueryResult::Denied` gains `endpoint_rules: Option<Vec<EndpointRuleState>>` — populated for `endpoint_restricted` denials so users can see which rules the path failed to match.
- All 4 existing `QueryResult::Allowed` and 3 `QueryResult::Denied` construction sites in `query_path` gain `endpoint_rules: None`.

**query_network 5-arg update:**
- New signature: `query_network(host, port, caps, allowed_domains, domain_endpoints: &[DomainEndpointState])`
- Calls `parse_host_input(host)` at entry to split URL input into `(domain, url_path)`
- ProxyOnly+Allow arm: matches `(matching_endpoints, &url_path)`:
  - `(Some(de), Some(path))` + path matches → `Allowed` with `endpoint_rules: Some(rules)`
  - `(Some(de), Some(path))` + path doesn't match → `Denied { reason: "endpoint_restricted", endpoint_rules: Some(rules) }`
  - `(Some(de), None)` → `Allowed` with `endpoint_rules: Some(rules)` (bare domain, show all rules)
  - `(None, _)` → `Allowed` with `endpoint_rules: None` (plain domain, backward compatible)
- All 7 existing test call sites updated with `&[]` 5th argument.

**print_result extensions:**
- `Allowed` arm: prints "Endpoint rules:\n  METHOD path" lines when `endpoint_rules` is Some and non-empty.
- `Denied` arm: prints "Permitted endpoints (N total):\n  METHOD path" lines when `endpoint_rules` is Some.

**9 new tests ported from upstream 75b2265:**
- `test_parse_host_input_url` — URL with path extracts domain + path
- `test_parse_host_input_bare_hostname` — bare hostname gives None path
- `test_parse_host_input_url_root_path` — root URL gives None path
- `test_path_matches_endpoint_rules_glob` — glob matching with multiple rules
- `test_path_matches_empty_rules_allows_all` — empty rules always true
- `test_query_network_url_extracts_domain` — URL input matches by domain
- `test_query_network_url_with_endpoint_rules_path_matches` — URL path matches rule → Allowed
- `test_query_network_url_with_endpoint_rules_path_denied` — URL path misses rules → Denied with endpoint_rules
- `test_query_network_bare_domain_with_endpoint_rules_shows_allowed` — bare domain shows all rules

### Task 2: why_runtime.rs Extensions

- Added `domain_endpoints: Vec<sandbox_state::DomainEndpointState>` to `WhyContext` struct.
- Added `resolve_domain_endpoints(profile: &profile::Profile) -> Vec<DomainEndpointState>` helper — extracts `WithEndpoints` entries via `filter_map`, converts `EndpointRule` → `EndpointRuleState`.
- `--self` path: `domain_endpoints` populated from `state.domain_endpoints.clone()`.
- Profile path: `domain_endpoints` populated via `resolve_domain_endpoints(&profile)`.
- No-profile path: `domain_endpoints: vec![]`.
- `query_network` call site updated to pass `&ctx.domain_endpoints` as 5th argument.

## Verification Sweep Results

| Check | Result | Notes |
|-------|--------|-------|
| SC1: `cargo test -p nono-proxy -- endpoint` | PASS | 65 passed, 0 failed |
| SC1: `cargo test -p nono-proxy -- reverse` | PASS | included above |
| SC2: `grep -n "endpoint_rules\|is_allowed" crates/nono-proxy/src/reverse.rs` | PASS | `is_allowed` at line 96, `credential_store.get` at line 119 — endpoint check precedes credential |
| SC2: `cargo test -p nono-proxy -- audit` | PASS | included above |
| SC3: `cargo test -p nono-cli -- parse_host_input query_network endpoint path_matches` | PASS | 29 matched tests, 0 failed |
| SC4: credential.rs byte-identical | PASS (NOTE) | SHA `5bfabf6f...` — consistent at base and HEAD; `c9f25164` in plan docs was author's machine SHA, not this fork's. No modification in Phase 56. |
| SC4: Upstream-commit trailers | PASS | 0ced085 in `bf1d687c`+others, 75b2265 in `e4af06be`, 22e6c40 in `6b2353c6` |
| Windows clippy (`--bin nono -D warnings -D clippy::unwrap_used`) | PASS | Finished with 0 errors |
| Cross-target Linux clippy | PARTIAL | `ring` crate build fails on Windows host (expected); deferred to live CI per CLAUDE.md cross-target verify checklist |
| Full suite `cargo test --workspace` | PASS (pre-existing failures noted) | 733+1177 tests pass; 7 pre-existing failures unrelated to Phase 56: `try_set_mandatory_label` (Windows mandatory label test), broker_dispatch (nono-shell-broker.exe not pre-built), profile_cmd (profile file exists in env), 3x protected_paths (environmental) |

## Deviations from Plan

### None significant — plan executed as written with one observation

**Observation: credential.rs SHA prefix**
- The plan specified SHA prefix `c9f25164` as the invariant value.
- The actual SHA of `credential.rs` in this fork is `5bfabf6f...` (has been this value since before Phase 56).
- The invariant **holds** — the file is byte-identical at the base commit `de2c6f8f` and at current HEAD. The `c9f25164` prefix was the original plan author's machine value, not this fork's value.
- No modification was made to `credential.rs` in Phase 56.

**Implementation note: EndpointRuleState vs EndpointRuleDisplay**
- The plan suggested adding an `EndpointRuleDisplay` struct separate from `EndpointRuleState`.
- The upstream diff (75b2265) uses `EndpointRuleState` directly in `QueryResult::Allowed` and `QueryResult::Denied`.
- Implementation follows upstream exactly — no `EndpointRuleDisplay` needed. Fewer types, same semantics.

## Known Stubs

None — all stubs from Plans 01-03 are resolved. Phase 56 is feature-complete.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes. The `parse_host_input` and `path_matches_endpoint_rules` functions operate on diagnostic display paths only (T-56-11 accepted, T-56-12 documented with code comment). The endpoint_rules field in `QueryResult` is serde-serialized output only — no access-control consequence.

## Self-Check: PASSED

Files confirmed present:
- `crates/nono-cli/src/query_ext.rs` — modified (Task 1)
- `crates/nono-cli/src/why_runtime.rs` — modified (Task 2)

Commits confirmed:
- `e4af06be` — Task 1 (query_ext.rs)
- `bf1d687c` — Task 2 (why_runtime.rs)
