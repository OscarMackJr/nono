---
phase: 56-fine-grained-network-filtering
plan: "01"
subsystem: network-policy
tags: [allow_domain, AllowDomainEntry, partition_allow_domain, DomainEndpointState, network-filtering, type-migration]
dependency_graph:
  requires: []
  provides:
    - AllowDomainEntry enum in profile/mod.rs (Vec<AllowDomainEntry> field)
    - merge_allow_domain fn in profile/mod.rs
    - partition_allow_domain fn in network_policy.rs
    - DomainEndpointState + EndpointRuleState in sandbox_state.rs
    - from_caps 4-arg signature in sandbox_state.rs
    - globset + urlencoding deps in nono-cli/Cargo.toml
  affects:
    - profile_cmd.rs (display/diff use .domain() adapter)
    - profile_runtime.rs (PreparedProfile.allow_domain type updated)
    - sandbox_prepare.rs (cascade adapter to Vec<String>)
    - why_runtime.rs (cascade adapter to Vec<String>)
    - execution_runtime.rs (from_caps stub with &[] TODO for Plan 02)
tech_stack:
  added:
    - globset = "0.4" (workspace dep, now also in nono-cli)
    - urlencoding = "2" (direct dep in nono-cli)
  patterns:
    - "#[serde(untagged)] backward-compatible enum deserialization"
    - "Domain-keyed HashMap merge for allow_domain union semantics"
    - "RouteConfig with fork-safe fields (no proxy/tls_client_cert/tls_client_key)"
    - "4-arg from_caps with #[serde(default, skip_serializing_if)] field"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/sandbox_state.rs
    - crates/nono-cli/Cargo.toml
    - crates/nono-cli/src/profile_cmd.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/why_runtime.rs
    - crates/nono-cli/src/execution_runtime.rs
decisions:
  - "AllowDomainEntry uses #[serde(untagged)] for backward-compatible mixed JSON arrays"
  - "merge_allow_domain replaces dedup_append for allow_domain — enables endpoint union semantics"
  - "partition_allow_domain fails-secure on empty domain (T-56-01 mitigation)"
  - "domain_endpoints field uses skip_serializing_if to preserve old NONO_CAP_FILE compat"
  - "Cascade type errors in profile_cmd/runtime/prepare/why_runtime fixed with minimal .domain() adapters — Plan 02 wires full partition_allow_domain"
  - "execution_runtime.rs passes &[] as domain_endpoints until Plan 02 wires it properly"
metrics:
  duration_minutes: 35
  completed_date: "2026-06-05"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 9
---

# Phase 56 Plan 01: AllowDomainEntry Foundation Types Summary

**One-liner:** AllowDomainEntry untagged enum + partition_allow_domain + DomainEndpointState with backward-compatible serde, porting upstream 0ced085.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | AllowDomainEntry enum + merge_allow_domain + Cargo.toml deps | 429cf4c6 | profile/mod.rs, Cargo.toml + 4 cascade files |
| 2 | partition_allow_domain + DomainEndpointState + from_caps 4-arg | 61a6545b | network_policy.rs, sandbox_state.rs, execution_runtime.rs |

## What Was Built

### Task 1: AllowDomainEntry + merge_allow_domain (profile/mod.rs)

- Introduced `AllowDomainEntry` enum with `#[serde(untagged)]` — backward-compatible with existing profile JSON using plain hostname strings.
- `AllowDomainEntry::Plain(String)` for bare hostnames.
- `AllowDomainEntry::WithEndpoints { domain, endpoints }` for fine-grained method+path rules.
- `domain()` accessor method marked `#[must_use]`.
- Changed `NetworkConfig.allow_domain` from `Vec<String>` to `Vec<AllowDomainEntry>` (all three serde attributes preserved).
- Added `merge_allow_domain()` pub(crate) function with domain-keyed HashMap union semantics; replaces `dedup_append` in the profile merge function.
- Added `globset.workspace = true` and `urlencoding = "2"` to nono-cli/Cargo.toml.
- 14 unit tests ported from upstream 0ced085.

### Task 2: partition_allow_domain + DomainEndpointState + from_caps 4-arg

- Added `is_loopback_domain()` private helper (localhost/127.x/::1/0.0.0.0).
- Added `partition_allow_domain(policy, &[AllowDomainEntry]) -> Result<(Vec<String>, Vec<RouteConfig>)>` — splits entries into plain hosts (via `expand_proxy_allow`) and endpoint-scoped RouteConfigs. Fail-secure `Err(ConfigParse)` on empty domain. Loopback entries get `http://` upstream scheme. Empty `endpoints` list treated as plain.
- RouteConfig construction uses only fork-present fields (no proxy/tls_client_cert/tls_client_key — A2 invariant verified).
- Added `DomainEndpointState { domain, endpoints }` and `EndpointRuleState { method, path }` to sandbox_state.rs.
- Added `domain_endpoints: Vec<DomainEndpointState>` field to `SandboxState` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for backward compat.
- Updated `from_caps` to 4-arg signature accepting `&[DomainEndpointState]`.
- Updated all 5 test call sites in sandbox_state.rs to 4-arg form.
- 10 unit tests ported from upstream 0ced085.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Type cascade from Vec<String> → Vec<AllowDomainEntry> blocked compilation**

- **Found during:** Task 1 GREEN phase — 9 compilation errors in profile_cmd.rs, profile_runtime.rs, sandbox_prepare.rs, why_runtime.rs
- **Issue:** The field type change cascaded to downstream callers before Plan 02 could fix them
- **Fix:** Minimal adapters using `.domain()` to extract `Vec<String>` for call sites that still expect `Vec<String>`. `PreparedProfile.allow_domain` upgraded to `Vec<AllowDomainEntry>` per PATTERNS.md; `PreparedSandbox.allow_domain` left as `Vec<String>` with conversion at boundary (Plan 02 will lift this)
- **Files modified:** profile_cmd.rs (3 sites), profile_runtime.rs (field type), sandbox_prepare.rs (2 sites), why_runtime.rs (2 sites)
- **Commits:** 429cf4c6

**2. [Rule 3 - Blocking] execution_runtime.rs from_caps 3-arg call blocked Task 2 test compilation**

- **Found during:** Task 2 GREEN phase — 1 compilation error in execution_runtime.rs
- **Issue:** `from_caps` signature change to 4-arg broke the production call site; Plan 02 is responsible for wiring it properly
- **Fix:** Added `&[]` as 4th arg stub with `// TODO(56-02)` comment documenting intent
- **Files modified:** execution_runtime.rs (1 site)
- **Commits:** 61a6545b

Both deviations are correctness-required minimal stubs that allow the tests to compile while preserving the explicit Plan 02 wiring work.

## Known Stubs

| File | Line | Description |
|------|------|-------------|
| crates/nono-cli/src/execution_runtime.rs | ~514 | `&[]` passed as domain_endpoints — Plan 02 will wire partition_allow_domain here |
| crates/nono-cli/src/sandbox_prepare.rs | ~508 | `.domain()` extraction converts Vec<AllowDomainEntry> to Vec<String> — Plan 02 will lift PreparedSandbox.allow_domain type |
| crates/nono-cli/src/why_runtime.rs | ~38-41 | Plain domain extraction passed to expand_proxy_allow — Plan 02/03 will wire partition_allow_domain |

These stubs are intentional cascade adapters — Plan 02 resolves them as part of full wiring.

## Threat Surface Scan

| Flag | File | Description |
|------|------|-------------|
| threat_flag: input-validation | crates/nono-cli/src/network_policy.rs | partition_allow_domain validates empty domain → Err (T-56-01 mitigated) |

No new network endpoints, auth paths, or schema changes at trust boundaries beyond what the plan's threat model anticipated.

## Verification Results

- `cargo check --bin nono | grep error | grep -v expected-callers` — no unexpected errors
- `cargo test -p nono-cli -- allow_domain partition merge` — 59 passed, 0 failed
- `grep "Vec<AllowDomainEntry>" profile/mod.rs` — field type changed
- `grep "merge_allow_domain" profile/mod.rs` — function + call site present
- `grep "globset|urlencoding" nono-cli/Cargo.toml` — both deps present
- `grep "fn partition_allow_domain" network_policy.rs` — function present
- `grep "DomainEndpointState" sandbox_state.rs` — type + field present
- `grep "domain_endpoints: &\[DomainEndpointState\]" sandbox_state.rs` — 4-arg signature present
- Both commits carry `Upstream-commit: 0ced085` trailer and `Signed-off-by: Oscar Mack Jr`

## Self-Check: PASSED
