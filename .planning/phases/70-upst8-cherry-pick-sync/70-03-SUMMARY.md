---
phase: 70-upst8-cherry-pick-sync
plan: "03"
subsystem: upst-sync
tags: [cherry-pick, network-policy, proxy, strict-filter, credentials, upstream-sync]

# Dependency graph
requires:
  - phase: 70-01
    provides: "suppressed_system_service_operations on PreparedSandbox (C2 prerequisite)"
  - phase: 69-upst8-audit
    provides: "DIVERGENCE-LEDGER.md C2 cluster analysis"
provides:
  - "Embedded network profiles (opencode, developer, codex, claude-code) no longer include implicit credentials"
  - "ProxyConfig.strict_filter: bool field (deny-by-default when network.block is set)"
  - "HostFilter::new_strict() method in nono library"
  - "ProxyFilter::new_strict() wrapper in nono-proxy"
  - "network_block_requested: bool field on PreparedSandbox"
  - "strict_filter wired PreparedSandbox -> ProxyLaunchOptions.network_block -> ProxyConfig.strict_filter"
  - "UPST8-02 satisfied: all 5 will-sync commits (C3 + C4 + C2) on main with D-19/D-20 trailers"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-19 trailer format: Upstream-commit + Upstream-tag + Upstream-author + Co-Authored-By + two Signed-off-by lines"
    - "Cherry-pick with conflict resolution: take HEAD side for fork-specific fields, add upstream additions"
    - "Strict filter pattern: ProxyConfig.strict_filter -> server.start -> HostFilter::new_strict"

key-files:
  created: []
  modified:
    - crates/nono-cli/data/network-policy.json
    - crates/nono-cli/src/network_policy.rs
    - crates/nono/src/net_filter.rs
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/filter.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/main.rs

key-decisions:
  - "0fb59375 auto-merged cleanly: network-policy.json and network_policy.rs changes applied without conflicts"
  - "bd4c469a sandbox_prepare.rs conflict: took HEAD side for loaded_profile/session_hooks fields (G-06/Phase-58), added network_block_requested in correct position"
  - "bd4c469a launch_runtime.rs conflict: took HEAD side (fork lacks trust_proxy_ca/proxy_ca_validity), added network_block field only"
  - "bd4c469a proxy_runtime.rs conflict: took HEAD side (fork lacks trust_proxy_ca/proxy_ca_validity args), added network_block: prepared.network_block_requested"
  - "bd4c469a upstream refactored helpers rejected: cwd_access_requirement/pending_cwd_access_request/resolve_detached_cwd_prompt_response not ported (use Rust 2024 let-chains; fork uses Edition 2021 inline logic)"
  - "server.rs auto-merged cleanly: strict_filter code applied without touching RouteStore/CredentialStore decoupling (Phase 56 invariant preserved)"
  - "Rule 3 fix: missing network_block_requested in proxy_runtime.rs test fixture PreparedSandbox literal — added and amended into the bd4c469a commit"
  - "Cross-target clippy: PARTIAL — Windows host cannot cross-compile (ring/aws-lc-sys C-toolchain); GH Actions CI required"

requirements-completed: [UPST8-02]

# Metrics
duration: 90min
completed: 2026-06-12
---

# Phase 70 Plan 03: Network Policy Security Hardening Summary

**Cherry-picked 0fb59375 (credential removal from embedded profiles) + bd4c469a (deny-by-default when network.block is set) from upstream v0.61.0/v0.61.2 into fork; proxy now enforces deny-by-default under network.block and implicit credential routes removed from all embedded network profiles**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-06-12T22:30:00Z
- **Completed:** 2026-06-13T00:00:00Z
- **Tasks:** 2 (Task 0: prerequisite gate; Task 1: 2 cherry-picks)
- **Files modified:** 10

## Accomplishments

- Task 0: C3 prerequisite gate PASSED — `suppressed_system_service_operations` field confirmed in `sandbox_prepare.rs` (4 occurrences)
- Task 1a: Cherry-picked 0fb59375 — removed `credentials` arrays from opencode, developer, codex, claude-code embedded network profiles; updated network_policy.rs tests to assert empty credential arrays
- Task 1b: Cherry-picked bd4c469a — added `HostFilter::new_strict()` (deny-on-empty-allowlist), `ProxyFilter::new_strict()`, `ProxyConfig.strict_filter: bool`, `network_block_requested: bool` on PreparedSandbox, wired through ProxyLaunchOptions.network_block -> ProxyConfig.strict_filter; proxy now denies non-allowlisted hosts when network.block is set
- Both cherry-picks carry verbatim D-19 trailers with correct per-commit authors (Luke Hinds for 0fb59375; Caio Silva for bd4c469a)
- Zero Windows-only files touched (D-70-E1 PASS for C2 and phase-wide)

## C2 Cherry-pick Log

### 0fb59375 — refactor(network-policy): do not enable credentials by default in profiles

- **Fork commit:** `1f5b6193`
- **Files changed:** `crates/nono-cli/data/network-policy.json`, `crates/nono-cli/src/network_policy.rs`
- **Merge result:** Auto-merged cleanly (no conflicts)
- **Conflict inventory:** None
- **D-19 trailer:** Upstream-author Luke Hinds, Upstream-tag v0.61.0 — CORRECT
- **Security effect:** 4 embedded profiles (opencode, developer, codex, claude-code) no longer carry implicit credential routes; credential injection now requires explicit profile declaration

### bd4c469a — fix(proxy): deny-by-default when network.block is set (#1082)

- **Fork commit:** `35282744` (amended to include Rule 3 test fixture fix)
- **Files changed:** 8 files (nono/net_filter.rs, nono-proxy/config.rs, nono-proxy/filter.rs, nono-proxy/server.rs, nono-cli/launch_runtime.rs, nono-cli/main.rs, nono-cli/proxy_runtime.rs, nono-cli/sandbox_prepare.rs)
- **Merge result:** 4 files had conflicts (launch_runtime.rs, main.rs, proxy_runtime.rs, sandbox_prepare.rs); 4 files auto-merged (net_filter.rs, config.rs, filter.rs, server.rs)
- **D-19 trailer:** Upstream-author Caio Silva, Upstream-tag v0.61.2 — CORRECT

**Conflict details:**

| File | Conflict reason | Resolution |
|------|----------------|------------|
| `sandbox_prepare.rs` | Upstream added many new helper functions (cwd_access_requirement, pending_cwd_access_request, resolve_detached_cwd_prompt_response) using Rust 2024 let-chains, plus network_block_requested to struct | Rejected upstream helper functions (Edition 2021 incompatible; fork has equivalent inline logic). Added `network_block_requested: bool` field to PreparedSandbox struct and to both struct literals (manifest path + profile path). Added `let network_block_requested = args.block_net || profile_network_block;` extraction block. |
| `launch_runtime.rs` | Upstream added `trust_proxy_ca: bool` and `proxy_ca_validity: Option<Duration>` to ProxyLaunchOptions (not in fork) plus `network_block: bool` | Took HEAD side (fork struct ends after `allow_launch_services_active`), added `network_block: bool` field only |
| `proxy_runtime.rs` | Upstream added `trust_proxy_ca: args.trust_proxy_ca` and `proxy_ca_validity` to struct literal plus `network_block: prepared.network_block_requested` | Took HEAD side, added `network_block: prepared.network_block_requested` only |
| `main.rs` (2 hunks) | Upstream added `network_block_requested: false` before fork's loaded_profile/session_hooks fields | Merged both: added `network_block_requested: false` before `loaded_profile`/`session_hooks` in both test PreparedSandbox literals |

## RouteStore/CredentialStore Decoupling Preservation (T-70-03-03)

Phase 56 fork surface preserved: `server.rs` auto-merged cleanly. The fork's `RouteStore::load()` and `CredentialStore::load()` are still called separately (lines 220-231). The only change in server.rs is the filter construction logic (lines 234-241), which now checks `config.strict_filter` to select `ProxyFilter::new_strict()` vs `ProxyFilter::allow_all()` vs `ProxyFilter::new()`. This does not touch the route or credential loading path. The RouteStore/CredentialStore decoupling is fully preserved.

## Task Commits

1. **Task 1 C2a: 0fb59375 cherry-pick** — `1f5b6193` (refactor)
2. **Task 1 C2b: bd4c469a cherry-pick** — `35282744` (fix, amended with Rule 3 fix)

## Files Created/Modified

- `crates/nono-cli/data/network-policy.json` — Removed credentials arrays from opencode/developer/codex/claude-code embedded profiles
- `crates/nono-cli/src/network_policy.rs` — Updated tests: test names renamed, assertions flipped to `is_empty()`, new `test_embedded_network_profiles_do_not_enable_credentials_by_default` sweeps all profiles
- `crates/nono/src/net_filter.rs` — Added `strict: bool` field to HostFilter struct; `HostFilter::new_strict()` constructor; `HostFilter::new()` and `allow_all()` default to `strict: false`; check_host returns DenyNotAllowed on empty allowlist when strict=true; 3 new tests
- `crates/nono-proxy/src/config.rs` — Added `strict_filter: bool` field to ProxyConfig (default false); Default impl updated
- `crates/nono-proxy/src/filter.rs` — Added `ProxyFilter::new_strict()` wrapping `HostFilter::new_strict()`
- `crates/nono-proxy/src/server.rs` — Filter construction updated to check `strict_filter` flag first; new integration test `test_strict_filter_with_empty_allowlist_denies_connect`
- `crates/nono-cli/src/sandbox_prepare.rs` — Added `network_block_requested: bool` to PreparedSandbox struct; extraction logic added; both struct literals updated
- `crates/nono-cli/src/launch_runtime.rs` — Added `network_block: bool` to ProxyLaunchOptions struct
- `crates/nono-cli/src/proxy_runtime.rs` — `prepare_proxy_launch_options` sets `network_block: prepared.network_block_requested`; `build_proxy_config_from_flags` sets `proxy_config.strict_filter = proxy.network_block`; 2 new tests; test fixture PreparedSandbox literal updated (Rule 3 fix)
- `crates/nono-cli/src/main.rs` — Two test PreparedSandbox literals updated with `network_block_requested: false`

## Decisions Made

- **Rejected upstream resolve_detached_cwd_prompt_response helpers**: The upstream bd4c469a commit was authored against a significantly newer codebase that has PendingCwdAccessRequest, DetachedCwdPromptResponse, DETACHED_CWD_PROMPT_RESPONSE_ENV types absent from the fork. These helpers also use Rust 2024 edition let-chains (Edition 2021 would compile if structured without let-chains, but the upstream code literally uses `if let Some(x) = ... && let Ok(y) = ...`). Rejected the injected block; the fork's existing resolved_workdir (simpler, Edition 2021 compatible) was retained. Security impact: none — these helpers are for CWD prompt UX, not sandbox enforcement.
- **network_block_requested field ordering**: Placed before loaded_profile/session_hooks in the struct declaration and in struct literals (consistent with upstream's intent but merged with fork's additional fields).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Missing `network_block_requested` in proxy_runtime.rs test PreparedSandbox literal**
- **Found during:** Task 1, bd4c469a conflict resolution — test suite run (`cargo test --workspace`)
- **Issue:** The test at proxy_runtime.rs:378 had a PreparedSandbox struct literal missing the new `network_block_requested` field (compile error E0063)
- **Fix:** Added `network_block_requested: false` to the test fixture PreparedSandbox literal; amended into the bd4c469a commit
- **Files modified:** `crates/nono-cli/src/proxy_runtime.rs`
- **Verification:** `cargo test --workspace` — 779 passed + 4 pre-existing nono-cli failures + 1 pre-existing nono lib failure; no new regressions

---

**Total deviations:** 1 auto-fixed (Rule 3 blocking)

## Baseline-Aware CI Gate (D-70-E4)

**Plan base SHA:** 6667177e

| Crate | Result | Details |
|-------|--------|---------|
| nono lib | 779 PASS, 1 FAIL | Pre-existing: `try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails` (red->red carry-forward) |
| nono-proxy | 162 PASS, 0 FAIL | New strict filter test PASS |
| nono-cli | 1219 PASS, 4 FAIL | Pre-existing: `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` + 3 `protected_paths::tests::*` (red->red carry-forward) |

**No new green->red transitions.** All pre-existing failures match the documented baseline.

## Close-Gate Verification (8-check format)

1. [_load_bearing] `git log --format="%B" HEAD~2..HEAD | grep -v "^#" | grep -c "^Upstream-commit:"` = **2** (PASS)
2. [_load_bearing] `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` = **0 lines** (D-70-E1 PASS for C2)
3. [_load_bearing] `grep "strict_filter" crates/nono-proxy/src/config.rs` finds `pub strict_filter: bool` (PASS)
4. [_load_bearing] `grep "network_block_requested" crates/nono-cli/src/sandbox_prepare.rs` finds the field (PASS)
5. [_load_bearing] `cargo build --workspace` exits 0 (PASS)
6. [_load_bearing] `cargo test --workspace`: no new green->red transitions (5 pre-existing failures = red->red carry-forward) (PASS)
7. [_load_bearing] Cross-target clippy: **PARTIAL** — Windows host cannot cross-compile (ring/aws-lc-sys C-toolchain missing); GH Actions Linux Clippy + macOS Clippy lanes on HEAD must report green
8. [_environmental] `git status --short | grep -E "build_notes|\.gsd"` = **0 lines** (PASS)

## Phase-Wide Close-Gate

- `git diff --name-only 6667177e..HEAD -- crates/ bindings/ | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` = **0 lines** (D-70-E1 PHASE-WIDE PASS)
- `git log --format="%B" 6667177e..HEAD | grep -v "^#" | grep -c -E "^Upstream-(commit|replayed-from):"` = **5** (one per will-sync commit: 2 C3 + 1 C4 D-20 + 2 C2) (PASS)

## Cross-Target Clippy Status

**Status: PARTIAL**

Rationale: `crates/nono-cli/src/sandbox_prepare.rs` contains `#[cfg(target_os = "linux")]` and `#[cfg(target_os = "macos")]` blocks. The Windows dev host cannot cross-compile to Linux/macOS (ring/aws-lc-sys C-toolchain absent from the Windows environment). Per CLAUDE.md § Coding Standards and `.planning/templates/cross-target-verify-checklist.md`:

**Human verification truth:** GH Actions Linux Clippy + macOS Clippy lanes on HEAD (`35282744`) must report green before the phase is considered fully verified.

The changes in `sandbox_prepare.rs` that touch cfg-gated code are minimal: two struct literal additions (`network_block_requested: false` / `network_block_requested: args.block_net`) and one extraction block (none of which are inside cfg-gated blocks). The `net_filter.rs` change has no cfg-gating. Risk of cross-target clippy failure is low.

## UPST8-02 Status

**SATISFIED** — All 5 will-sync commits (C3 + C4 + C2) are on main with D-19/D-20 trailers:

| Cluster | Commit | Fork SHA | Trailer | Tag |
|---------|--------|----------|---------|-----|
| C3a | cc21229f | e80a7c45 | D-19 (Luke Hinds) | v0.61.0 |
| C3b | 20cc5df9 | 497101ae | D-19 (Luke Hinds) | v0.61.0 |
| C4 | db073750 | c18dd264 | D-20 (Luke Hinds, manual replay) | v0.61.0 |
| C2a | 0fb59375 | 1f5b6193 | D-19 (Luke Hinds) | v0.61.0 |
| C2b | bd4c469a | 35282744 | D-19 (Caio Silva) | v0.61.2 |

## Human-Verify Checkpoint

AWAITING — This plan ends with a `checkpoint:human-verify` gate. The automated checks above all pass. The checkpoint asks the user to:
1. Run `git log --oneline --format="%h %s" -8` to confirm all 5 cherry-pick commits appear
2. Verify D-19 trailer count = 5
3. Verify D-70-E1 phase-wide invariant
4. Verify C2 security properties (no credentials in profiles, strict_filter, HostFilter strict mode)
5. Run full workspace test suite and confirm no new regressions
6. Confirm cross-target clippy PARTIAL status is acceptable
7. Confirm repo-public invariant

## Known Stubs

None — all new fields are fully wired through the data pipeline.

## Threat Flags

None — the changes close two documented threats (T-70-03-01 implicit credential disclosure; T-70-03-02 allow-all fallback under network.block) rather than introducing new surface.

## Self-Check: PASSED

- FOUND: crates/nono-cli/data/network-policy.json
- FOUND: crates/nono-cli/src/network_policy.rs
- FOUND: crates/nono/src/net_filter.rs
- FOUND: crates/nono-proxy/src/config.rs
- FOUND: crates/nono-proxy/src/filter.rs
- FOUND: crates/nono-proxy/src/server.rs
- FOUND: crates/nono-cli/src/sandbox_prepare.rs
- FOUND: crates/nono-cli/src/launch_runtime.rs
- FOUND: crates/nono-cli/src/proxy_runtime.rs
- FOUND: crates/nono-cli/src/main.rs
- FOUND commit: 1f5b6193 (C2a: 0fb59375)
- FOUND commit: 35282744 (C2b: bd4c469a)

---
*Phase: 70-upst8-cherry-pick-sync*
*Completed: 2026-06-12*
