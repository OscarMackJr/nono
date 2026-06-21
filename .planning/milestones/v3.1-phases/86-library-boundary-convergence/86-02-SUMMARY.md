---
phase: 86-library-boundary-convergence
plan: "02"
subsystem: diagnostic
tags:
  - upstream-sync
  - cherry-pick
  - diagnostics
  - ffi
  - library-boundary
dependency_graph:
  requires:
    - "86-01 (BND-01 — refactor/query.rs unification)"
  provides:
    - "BND-02 — structured-diagnostics model in core crate + FFI"
    - "diagnostic/ 6-file module directory in nono core"
    - "NonoDiagnosticCode repr-C enum + 3 FFI extern-C fns"
    - "ProxyDiagnostic surface in nono-proxy"
    - "DiagnosticFormatter UX in nono-cli/src/diagnostic/"
  affects:
    - "86-03 (BND-03 — Windows-diag + FFI type unification)"
    - "nono-py + nono-ts FFI consumers (NonoDiagnosticCode now stable ABI)"
tech_stack:
  added:
    - "crates/nono/src/diagnostic/ (6-file module directory)"
    - "crates/nono-proxy/src/diagnostic.rs (ProxyDiagnostic)"
    - "bindings/c/src/diagnostic.rs (3 extern-C diagnostic fns)"
  patterns:
    - "cherry-pick -x with DCO sign-off amend"
    - "cfg-gate cfg(not(target_os = windows)) for Unix-only formatter module"
    - "nested if-let for Rust 2021 let-chain compat"
key_files:
  created:
    - crates/nono/src/diagnostic/codes.rs
    - crates/nono/src/diagnostic/detail.rs
    - crates/nono/src/diagnostic/mod.rs
    - crates/nono/src/diagnostic/observation.rs
    - crates/nono/src/diagnostic/records.rs
    - crates/nono/src/diagnostic/report.rs
    - crates/nono-proxy/src/diagnostic.rs
    - bindings/c/src/diagnostic.rs
    - crates/nono-cli/src/diagnostic/formatter.rs
  modified:
    - crates/nono/src/error.rs
    - crates/nono/src/lib.rs
    - crates/nono-cli/src/diagnostic/mod.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/query_ext.rs
    - crates/nono-cli/src/sandbox_log.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-proxy/src/lib.rs
    - bindings/c/src/lib.rs
    - bindings/c/src/types.rs
    - bindings/c/Cargo.toml
    - .gitignore
decisions:
  - "cfg-gate formatter module on cfg(not(target_os = windows)) — matches existing fork pattern for Unix-only exec_strategy"
  - "Restore HEAD versions of credential.rs + server.rs (upstream rewrites reference tls_intercept + RouteConfig.aws_auth not in fork) then add minimal diagnostic surface"
  - "Nested if-let blocks for all Rust 2024 let-chains (workspace is 2021 edition)"
  - "Add exhaustive NonoError arms for 9 fork-specific variants not in upstream's diagnostic_code() match"
  - "7f319b9e conflict resolved by keeping HEAD (format_command_for_learn block already updated by a6aa5995 restructure)"
  - "BND-02 cross-target partial: supervisor_linux.rs cannot be verified on Windows dev host — deferred to live CI"
metrics:
  duration_minutes: 120
  completed: "2026-06-19"
  tasks_completed: 4
  files_changed: 25
---

# Phase 86 Plan 02: BND-02 Structured Diagnostics to Core + FFI Summary

**One-liner:** Cherry-picked 4 upstream commits to move DiagnosticFormatter UX to CLI, expand nono core's diagnostic module from 1 file to 6, add NonoError::{diagnostic_code,remediation} methods, expose 3 FFI extern-C diagnostic functions, and bridge Windows denial paths without touching exec_strategy_windows/.

## Tasks Completed

| # | Cherry-Pick SHA | Upstream Title | Commit |
|---|----------------|----------------|--------|
| 1 | 4ad8ba92 | refactor(diagnostic): move diagnostic UX out of core nono crate (#1155) | `5c050e94` |
| 2 | f867aba2 | fix: report actual blocked operation instead of readable target path in sandbox denial diagnostics (#1150) | `8a3ef904` |
| 3 | a6aa5995 | feat(diagnostics): expose structured diagnostics for library and FFI clients (#1171) | `f65e153e` |
| 4 | 7f319b9e | fix(diagnostic): replace deprecated nono learn with nono run (#1170) | `d6a81355` |
| fmt | — | style(86-02): apply cargo fmt across cherry-pick series | `eba1edbc` |

## Plan Assertions Verified

| Assertion | Status |
|-----------|--------|
| `crates/nono/src/diagnostic/` 6-file module dir exists | PASS |
| `crates/nono/src/diagnostic.rs` (single file) DELETED | PASS |
| `NonoError` has `diagnostic_code()` and `remediation()` methods in error.rs | PASS |
| `bindings/c/src/diagnostic.rs` with 3 pub extern C fns | PASS |
| `NonoDiagnosticCode` repr-C enum in bindings/c/src/types.rs | PASS |
| `crates/nono-proxy/src/diagnostic.rs` (ProxyDiagnostic) | PASS |
| `crates/nono-cli/src/diagnostic/formatter.rs` (DiagnosticFormatter) | PASS |
| `cargo clippy --workspace --all-targets` exits 0 | PASS |
| `exec_strategy_windows/` unchanged (D-02 preserve-and-bridge) | PASS — 0 diff lines |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Windows build dead code for formatter module**
- **Found during:** Task 1 (4ad8ba92) clippy gate
- **Issue:** All formatter types showed as dead code on Windows builds because exec_strategy.rs and profile_save_runtime.rs are both cfg(not(target_os = windows))-gated, so the formatter module was unreachable on Windows
- **Fix:** Added `#[cfg(not(target_os = "windows"))]` to both `mod formatter` and `pub use formatter::{...}` in `crates/nono-cli/src/diagnostic/mod.rs`
- **Files modified:** `crates/nono-cli/src/diagnostic/mod.rs`
- **Commits:** `5c050e94`

**2. [Rule 1 - Bug] Upstream credential.rs + server.rs incompatible with fork**
- **Found during:** Task 3 (a6aa5995) conflict resolution
- **Issue:** Upstream's credential.rs references `RouteConfig.proxy` + `RouteConfig.aws_auth` not in fork; upstream's server.rs references `crate::tls_intercept` module not in fork
- **Fix:** Restored HEAD versions via `git checkout HEAD -- ...` then surgically added only `CredentialLoadOutcome` struct and minimal `ProxyHandle` diagnostic surface (`diagnostics` field, `diagnostics()`, `intercept_ca_path()`, `route_diagnostics()`)
- **Files modified:** `crates/nono-proxy/src/credential.rs`, `crates/nono-proxy/src/server.rs`
- **Commits:** `f65e153e`

**3. [Rule 1 - Bug] `ActionRequired` struct variant pattern mismatch**
- **Found during:** Task 3 (a6aa5995) — upstream's `diagnostic_code()` used `Self::ActionRequired(_)` (tuple pattern) but fork has struct variant `ActionRequired { expected, actual, resolve_via }`
- **Fix:** Changed to `Self::ActionRequired { .. }`
- **Files modified:** `crates/nono/src/error.rs`
- **Commits:** `f65e153e`

**4. [Rule 2 - Missing coverage] Fork-specific NonoError variants not covered in diagnostic_code()**
- **Found during:** Task 3 (a6aa5995) — non-exhaustive match for 9 fork-specific variants: `NotSupportedOnPlatform`, `UnsupportedKernelFeature`, `BrokerNotFound`, `LabelApplyFailed`, `DaclApplyFailed`, `PolicyLoadFailed`, `PartialRestore`, `TelemetryUnavailable`, `TelemetryConfigInvalid`
- **Fix:** Added match arms for each variant with semantically appropriate diagnostic codes (`UnsupportedPlatformFeature`, `ConfigurationError`, `Other`)
- **Files modified:** `crates/nono/src/error.rs`
- **Commits:** `f65e153e`

**5. [Rule 1 - Bug] Rust 2024 let-chain syntax in 2021 workspace**
- **Found during:** Tasks 3 and 4 — 4 instances in formatter.rs + 1 in exec_strategy.rs using `if let x = y && let z = w` syntax only valid in Rust 2024
- **Fix:** Converted all 5 instances to nested `if let` blocks
- **Files modified:** `crates/nono-cli/src/diagnostic/formatter.rs`, `crates/nono-cli/src/exec_strategy.rs`
- **Commits:** `f65e153e`

**6. [Rule 1 - Bug] 7f319b9e conflict in wrong function**
- **Found during:** Task 4 — upstream diff applied to `format_network_denial_from_diagnostic` but our fork's a6aa5995 restructuring already applied the "nono learn → nono run" text update in `format_follow_up_from_diagnostics` and test assertions (lines 3198, 3229, 3285 already had "Add permissions: nono run --allow")
- **Fix:** Kept HEAD for the conflict block (the function that received the conflict has its own `self.suggested_flag_for_hint` call; the "nono learn" string was already gone)
- **Files modified:** `crates/nono-cli/src/diagnostic/formatter.rs`
- **Commits:** `d6a81355`

**7. [Rule 1 - Bug] cargo fmt drift across cherry-pick series**
- **Found during:** Theme B gate (`cargo fmt --all -- --check`)
- **Issue:** 9 files had import ordering + assert! multi-line formatting drift
- **Fix:** `cargo fmt --all` pass, committed as separate style commit
- **Files modified:** 9 files across nono, nono-cli, nono-proxy
- **Commits:** `eba1edbc`

## Cross-Target Verification Status

**BND-02 PARTIAL** — `supervisor_linux.rs` contains `#[cfg(target_os = "linux")]`-gated code modified during Task 2 (f867aba2: `path.clone()` → `canonicalized.clone()` in 4 DenialRecord constructions). Cross-target clippy cannot be verified on this Windows dev host. Status: **DEFERRED to live CI** per `.planning/templates/cross-target-verify-checklist.md`.

All other modified files (error.rs, lib.rs, diagnostic/ module, FFI bindings, proxy) are platform-agnostic and verified clean via `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`.

## Test Results

- `cargo test -p nono`: 775 passed, 1 failed (`try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails` — **pre-existing env-specific failure documented in MEMORY**)
- `cargo test -p nono-cli`: 1315 passed, 4 failed (profile_cmd init + 3 protected_paths — **all pre-existing env-specific failures documented in MEMORY**)
- `cargo test -p nono-proxy`: 163 passed, 0 failed

## Known Stubs

`ProxyHandle::intercept_ca_path()` returns `None` unconditionally — this is a fork-compatible stub since the fork does not implement TLS interception. The return type matches upstream's signature for API compatibility. Tracked for resolution if/when the fork adopts TLS intercept from upstream.

`ProxyHandle::route_diagnostics()` returns simplified route rows — omits OAuth2/TLS intercept fields not in fork. API-compatible with upstream signature.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. Diagnostic functions are read-only (query last error state + generate JSON reports). FFI functions operate on process-local thread-local state only.

## Self-Check: PASSED

- `crates/nono/src/diagnostic/` directory: FOUND (6 files: codes.rs, detail.rs, mod.rs, observation.rs, records.rs, report.rs)
- `bindings/c/src/diagnostic.rs`: FOUND
- `crates/nono-proxy/src/diagnostic.rs`: FOUND
- `crates/nono-cli/src/diagnostic/formatter.rs`: FOUND
- Cherry-pick commits `5c050e94`, `8a3ef904`, `f65e153e`, `d6a81355`, `eba1edbc`: FOUND in git log
- D-02 exec_strategy_windows/ diff: 0 lines changed
- `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`: EXIT 0
- `cargo fmt --all -- --check`: EXIT 0
