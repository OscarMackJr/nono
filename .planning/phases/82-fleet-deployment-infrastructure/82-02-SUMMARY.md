---
phase: 82-fleet-deployment-infrastructure
plan: 02
subsystem: infra
tags: [windows, cert-trust, provisioner, fleet-deploy, first-run, node-tls]

# Dependency graph
requires:
  - phase: 82-01
    provides: MSI layout (INSTALLFOLDER cert staging, %PROGRAMDATA%\nono\nono-poc-root.pem path confirmed)
provides:
  - cert_trust.rs: reusable cert-store import module (Root+TrustedPublisher machine; CurrentUser\Root per-user) — single store-list source of truth
  - provision_windows.rs: idempotent D-09 first-run provisioner (scratch + cert + NODE_EXTRA_CA_CERTS)
  - nono setup --trust-root <cer> verb: MSI CA entry point for machine cert import
  - command_runtime.rs hook: non-fatal first-run provisioner call before exec_strategy builds child
affects: [82-03-health-command, 82-04-dark-gate, 83-policy-reader]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "certutil subprocess shim for cert-store import (no winreg/Windows-cert-store API needed)"
    - "HKCU reg sentinel idempotency via reg.exe subprocess (defers winreg to Phase 83)"
    - "Platform-stub pattern for cross-target clippy: #[cfg(not(target_os = windows))] stubs returning UnsupportedPlatform"
    - "Non-fatal provisioner: all sub-steps captured in ProvisionStatus, run never aborted"

key-files:
  created:
    - crates/nono-cli/src/cert_trust.rs
    - crates/nono-cli/src/provision_windows.rs
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/setup.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/command_runtime.rs

key-decisions:
  - "Cert thumbprint probe via certutil -user -store Root output parse (no Windows CryptoAPI FFI needed for probe)"
  - "HKCU registry sentinel written via reg.exe subprocess; winreg crate deferred to Phase 83 policy reader"
  - "set_owner_to_current_user uses whoami+icacls subprocess pattern from dacl_guard.rs — mirrors existing R-B3 idiom"
  - "NODE_EXTRA_CA_CERTS set both in-process (std::env::set_var) AND persisted via setx user scope"
  - "provisioner is cfg(target_os = windows) only in main.rs module declaration; cert_trust.rs has non-Windows stubs for cross-target clippy"

patterns-established:
  - "Non-fatal provisioning: sub-steps return StepStatus::Degraded(msg) to ProvisionStatus, never propagate Err"
  - "Single source of truth for machine cert store names: MACHINE_STORES const in cert_trust.rs"
  - "MSI-CA-only verb pattern: --trust-root flag documented as MSI-internal, not a general user affordance"

requirements-completed: [DEPLOY-03, DEPLOY-05]

# Metrics
duration: 45min
completed: 2026-06-18
---

# Phase 82 Plan 02: First-Run Provisioner + Cert Trust Summary

**D-09 unifying first-run provisioner: user-owned scratch + per-user cert import + NODE_EXTRA_CA_CERTS persistence, with a single Rust store-name source of truth for the MSI CA cert verb.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-18T~19:10Z
- **Completed:** 2026-06-18T~19:55Z
- **Tasks:** 3/3
- **Files modified:** 6 (2 new, 4 modified)

## Accomplishments

### Task 1: cert_trust.rs + nono setup --trust-root verb
- Created `crates/nono-cli/src/cert_trust.rs` (267 lines) with:
  - `import_machine_root(cert_path)`: iterates `MACHINE_STORES = ["Root", "TrustedPublisher"]` (single source of truth), shells `certutil -addstore -f <store> <cert>` for each
  - `import_current_user_root(cert_path)`: idempotent (extracts thumbprint via `certutil -dump`, probes presence via `certutil -user -store Root`), then `certutil -user -addstore -f Root <cert>`
  - `is_cert_present_current_user(thumbprint)`: presence probe returning `Ok(true/false)`
  - Non-Windows stubs returning `NonoError::UnsupportedPlatform` for cross-target clippy
- Added `--trust-root <PATH>` flag to `SetupArgs` in `cli.rs` (MSI-internal verb)
- Added dispatch arm in `setup.rs` `run()`: short-circuits to `cert_trust::import_machine_root` when `--trust-root` is Some
- Added `mod cert_trust;` to `main.rs`
- Unit tests: `is_cert_present_current_user` with absent thumbprint returns Ok(false) on Windows / UnsupportedPlatform on non-Windows (no panic)

### Task 2: provision_windows.rs (D-09 unifying provisioner)
- Created `crates/nono-cli/src/provision_windows.rs` (497 lines) with:
  - `provision_first_run() -> Result<ProvisionOutcome>`: 3-step idempotent provisioner
  - `ProvisionStatus` struct: per-step `StepStatus` (Ok/Degraded) for nono health consumption
  - Step 1 (scratch): `create_dir_all(%LOCALAPPDATA%\nono\workspace)` + `icacls /setowner` + `nono::path_is_owned_by_current_user` verify
  - Step 2 (cert): `cert_trust::import_current_user_root` against cert adjacent to `current_exe()`
  - Step 3 (NODE_EXTRA_CA_CERTS): sets in-process via `std::env::set_var` + persists via `setx NODE_EXTRA_CA_CERTS <pem>` (user scope)
  - Idempotency: HKCU `\Software\nono\ProvisionedAt` registry sentinel via `reg query`/`reg add` subprocess
- Added `#[cfg(target_os = "windows")] mod provision_windows;` to `main.rs`
- Unit tests: idempotency round-trip, scratch path under LOCALAPPDATA, missing LOCALAPPDATA error

### Task 3: command_runtime.rs hook
- Inserted `#[cfg(target_os = "windows")]` provisioner call in `run_sandbox()` after dry-run guard, before `prepare_run_launch_plan`
- NON-FATAL: `Err` and degraded outcomes log `tracing::warn!` (respecting `silent`) and run continues
- Not called on `args.dry_run` branch (no host mutation on dry-run)

## Verification Results

```
cargo test -p nono-cli -- cert_trust provision
  cert_trust::tests::test_cert_trust_api_compiles ... ok
  cert_trust::tests::test_is_cert_present_absent_thumbprint_no_panic ... ok
  provision_windows::tests::test_scratch_dir_under_localappdata ... ok
  provision_windows::tests::test_scratch_dir_fails_without_localappdata ... ok
  provision_windows::tests::test_provision_idempotent_second_call ... ok
  test result: ok. 5 passed; 0 failed

cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used
  Finished (no errors, no warnings)

cargo build -p nono-cli
  Finished (no errors, no warnings)
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed missing `trust_root` field in SetupRunner test initializer**
- **Found during:** Task 1 test run
- **Issue:** A test in setup.rs constructs `SetupRunner` directly (struct literal); missing the new `trust_root` field caused E0063
- **Fix:** Added `#[cfg(target_os = "windows")] trust_root: None` to the test initializer
- **Files modified:** `crates/nono-cli/src/setup.rs`
- **Commit:** a5288f07

**2. [Rule 1 - Bug] Fixed clippy::collapsible_if in command_runtime.rs**
- **Found during:** Task 3 clippy check
- **Issue:** Nested `if any_degraded { if !silent {` triggered `-D clippy::collapsible_if`
- **Fix:** Extracted `let any_degraded = ...` and collapsed to `if any_degraded && !silent`
- **Files modified:** `crates/nono-cli/src/command_runtime.rs`
- **Commit:** 53decff4

**3. [Rule 1 - Bug] Fixed disallowed_methods for std::env::set_var**
- **Found during:** Task 2 clippy check
- **Issue:** `clippy.toml` globally disallows `std::env::set_var`; production env injection in provisioner was flagged
- **Fix:** Added `#[allow(clippy::disallowed_methods)]` with a clarifying comment ("Production env injection, not test mutation") following the `cli_bootstrap.rs` / `profile_runtime.rs` precedent
- **Files modified:** `crates/nono-cli/src/provision_windows.rs`
- **Commit:** 05a91ee9

**4. [Rule 1 - Bug] Fixed test env-var mutation to use EnvVarGuard**
- **Found during:** Task 2 clippy check
- **Issue:** Tests in provision_windows.rs used raw `std::env::set_var` / `remove_var` which violates CLAUDE.md (env var mutation in tests must use `EnvVarGuard`)
- **Fix:** Replaced raw calls with `crate::test_env::EnvVarGuard::set_all` + `.remove()`
- **Files modified:** `crates/nono-cli/src/provision_windows.rs`
- **Commit:** 05a91ee9

### Plan Reference Deviation

**The plan references `lib.rs` for module declarations, but nono-cli is a binary crate with `main.rs` as its root.** All `mod cert_trust;` and `mod provision_windows;` declarations were added to `main.rs` instead. This is architecturally correct — the plan's intent was to add module declarations to the crate root, which is `main.rs` for a binary crate.

## Known Stubs

None — all provisioner steps are fully wired. Sub-steps that fail at runtime (e.g., cert not yet staged, %LOCALAPPDATA% unusual path, setx unavailable) degrade gracefully and report via ProvisionStatus.

## Deferred Items

Pre-existing formatting drift in three files was touched by `cargo fmt` but NOT staged for this commit (out-of-scope per CLAUDE.md deviation scope boundary):
- `crates/nono-cli/src/agent_daemon/control_loop.rs` — line-wrapping formatting
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` — line-wrapping formatting
- `crates/nono-cli/tests/daemon_handle_baseline.rs` — formatting

**Cross-target clippy (PARTIAL):** This plan introduces `#[cfg(target_os = "windows")]`-gated code in shared files (`main.rs`, `command_runtime.rs`, `cli.rs`, `setup.rs`) and new modules (`cert_trust.rs` with non-Windows stubs, `provision_windows.rs` Windows-only). Per CLAUDE.md, cross-target verification via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` and `--target x86_64-apple-darwin` is required. The cross-toolchain is not installed on the Windows dev host — this criterion is marked **PARTIAL** and deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`.

## Threat Surface Scan

The new code introduces the following security-relevant surfaces, all of which are covered by the plan's `<threat_model>`:

| Flag | File | Description |
|------|------|-------------|
| threat_flag: trust-store-mutation | cert_trust.rs | Machine + per-user cert store imports via certutil subprocess |
| threat_flag: env-injection | provision_windows.rs | NODE_EXTRA_CA_CERTS set in-process + persisted via setx |
| threat_flag: filesystem-acl | provision_windows.rs | WRITE_OWNER scratch dir creation under %LOCALAPPDATA%\nono\ |

All three surfaces are enumerated in the plan's STRIDE register (T-82-10 through T-82-16) and the mitigations are implemented as specified:
- Scratch under %LOCALAPPDATA% (user-owned, not drive-root — T-82-11)
- Cert path resolved from `current_exe().parent()` (INSTALLFOLDER, Admins-only write — T-82-12)
- PEM path from %PROGRAMDATA%\nono\ (Admins-only write — T-82-13)
- All steps non-fatal (T-82-14)

## Self-Check: PASSED

Files confirmed present:
- crates/nono-cli/src/cert_trust.rs — FOUND
- crates/nono-cli/src/provision_windows.rs — FOUND

Commits confirmed:
- a5288f07 feat(82-02): implement cert_trust.rs + nono setup --trust-root verb — FOUND
- 05a91ee9 feat(82-02): implement idempotent first-run provisioner (provision_windows.rs) — FOUND
- 53decff4 feat(82-02): hook first-run provisioner into nono run path (non-fatal) — FOUND
