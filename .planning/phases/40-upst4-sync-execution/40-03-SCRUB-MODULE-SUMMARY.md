---
phase: 40-upst4-sync-execution
plan: 03
slug: scrub-module
subsystem: nono-core, nono-cli
tags: [upst4, c6, scrub, audit, foundation, wave-0]

dependency_graph:
  requires: []
  provides:
    - nono::scrub (ScrubPolicy, ScrubPolicyDiff, scrub_argv_with_policy, scrub_header_with_policy, scrub_value_with_policy)
    - audit event emission scrubbed by default via AuditRecorder::new_with_policy
  affects:
    - crates/nono/src/scrub.rs
    - crates/nono/src/lib.rs
    - crates/nono-cli/src/audit_integrity.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/rollback_runtime.rs
    - crates/nono-cli/src/supervised_runtime.rs
    - crates/nono-cli/src/output.rs

tech_stack:
  added:
    - nono::scrub module (ScrubPolicy, ScrubPolicyDiff, cross-platform, no Windows-specific rules)
  patterns:
    - ScrubPolicy as redaction configuration threaded through ExecutionFlags → SupervisedRuntimeContext → SupervisorConfig → RollbackExitContext
    - AuditRecorder::new_with_policy(session_dir, redaction_policy) for argv scrubbing at event emit

key_files:
  created:
    - crates/nono/src/scrub.rs
    - crates/nono-cli/src/audit_ledger.rs
  modified:
    - crates/nono/src/lib.rs
    - crates/nono-cli/src/audit_integrity.rs
    - crates/nono-cli/src/command_runtime.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/output.rs
    - crates/nono-cli/src/rollback_runtime.rs
    - crates/nono-cli/src/supervised_runtime.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/config/user.rs
    - docs/cli/features/audit.mdx
    - docs/cli/internals/security-model.mdx
    - docs/cli/usage/flags.mdx

decisions:
  - "Kept fork's audit_attestation.rs implementation; upstream's rewrite references sign_statement_bundle and write_audit_attestation which don't exist in fork — kept fork's sign_session_attestation path"
  - "Accepted audit_ledger.rs as-is from upstream with #[cfg(unix)] gate in main.rs (uses nix crate, Unix-only)"
  - "Removed 2 upstream output.rs tests (allow_unix_socket, UnixSocketMode) — API not yet in fork; documented in Known Stubs"
  - "exec_strategy_windows/mod.rs: +4 lines to wire redaction_policy into RollbackExitContext — necessary fork-adaptation (struct gained new required field)"
  - "exec_strategy.rs: compute redaction_policy before finalize_supervised_exit call (audit_signer not pre-computed by execution_runtime.rs in fork)"

metrics:
  duration: "~3 hours (including complex 14-file conflict resolution for 6472011)"
  completed: "2026-05-14T00:44:18Z"
  tasks_completed: 4
  files_changed: 18
---

# Phase 40 Plan 03: Scrub Module Summary

Wave-0 foundation plan: cherry-picked upstream v0.53.0 Cluster C6 (2 commits) adding `nono::scrub` module for secret scrubbing of command arguments, HTTP headers, and URL query parameters before audit event persistence.

## What Was Built

- New `crates/nono/src/scrub.rs`: cross-platform secret-scrubbing module with `ScrubPolicy`, `ScrubPolicyDiff`, `scrub_argv_with_policy`, `scrub_header_with_policy`, `scrub_value_with_policy`
- `ScrubPolicy` threaded from `ExecutionFlags` → `LaunchPlan` → `SupervisedRuntimeContext` → `SupervisorConfig.redaction_policy` → `RollbackExitContext.redaction_policy`
- `AuditRecorder::new_with_policy` wired for argv scrubbing at session-started event emit
- `load_configured_redaction_policy()` reads user config for custom redaction settings
- `audit_ledger.rs` accepted from upstream (Unix-only, gated in main.rs)
- 8 scrub unit tests ported from upstream (D-40-E4 compliance)

## Commits

| Commit | Hash | Upstream | Subject |
|--------|------|----------|---------|
| 1/2 | `96886ae9` | `6472011e` | feat(core): scrub command arguments for secrets |
| 2/2 | `7831c47f` | `78114e6` | refactor(scrub): optimize and simplify scrubbing logic |

Both carry verbatim D-19 6-line trailers (lowercase 'a' in Upstream-author). D-40-C4 compliant.

## D-40-C2 Close Gate Results

| Gate | Check | Result |
|------|-------|--------|
| 1 | `cargo test --workspace` | PASS (1011 nono-cli + 689 nono tests) |
| 2 | `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` | PASS |
| 3 | Linux cross-target clippy | SKIPPED — `x86_64-linux-gnu-gcc` unavailable on Windows host |
| 4 | macOS cross-target clippy | SKIPPED — cross-compiler unavailable on Windows host |
| 5 | `cargo fmt --check` | PASS |
| 6 | Phase 15 detached-console smoke | SKIPPED — manual integration test |
| 7 | `cargo test -p nono -- scrub` | PASS (8/8 scrub unit tests) |
| 8 | learn_windows_integration | SKIPPED — admin-level test |

**Gates 1 + 2 + 5 (hard gates): ALL PASS. D-40-C3 freeze condition NOT triggered.**

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `exec_strategy_windows/mod.rs` requires `redaction_policy` field**
- **Found during:** Task 2 (post-cherry-pick build verification)
- **Issue:** `RollbackExitContext` struct gained `redaction_policy: &'a nono::ScrubPolicy` as a required field; the Windows `execute_supervised` callsite in `exec_strategy_windows/mod.rs` did not include it
- **Fix:** Added 4 lines: `let win_default_redaction_policy = nono::ScrubPolicy::secure_default();` + `redaction_policy: &win_default_redaction_policy`. Uses `secure_default()` only — no Windows-specific scrub behavior added (D-40-E6 satisfied)
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/mod.rs`
- **Commit:** `96886ae9`
- **D-40-E1 impact:** +4 lines in Windows file on commit 1 (necessary fork-adaptation, not upstream code)

**2. [Rule 1 - Bug] `Cow<str>::as_ref()` type ambiguity from `typed_path` crate**
- **Found during:** Task 3 (cargo test build)
- **Issue:** The `typed_path` crate adds an `impl<T> AsRef<Utf8Path<T>> for Cow<'_, str>` which creates ambiguity when calling `.as_ref()` on a `Cow<str>` — the compiler cannot resolve between `AsRef<str>` and `AsRef<Utf8Path<T>>`
- **Fix:** Added `as &str` type coercions at 3 call sites in `scrub.rs`
- **Files modified:** `crates/nono/src/scrub.rs`
- **Commit:** `7831c47f` (amend)

**3. [Rule 1 - Bug] Test compilation failures from upstream API divergence**
- **Found during:** Task 3 (cargo test)
- **Issues:**
  - `verifier_round_trips_all_current_audit_event_payload_variants` test: missing `NetworkAuditMode`, `NetworkAuditDecision` imports; `record_capability_decision` takes 2 args in fork vs 1 in upstream; `CapabilityRequest` has 4 additional required fields in fork (`session_token`, `kind`, `target`, `access_mask`)
  - `unix_socket_mode_badges_are_fixed_width_and_distinct` and `print_capabilities_with_unix_socket_does_not_panic`: reference `allow_unix_socket`, `allow_unix_socket_dir`, `format_unix_socket_mode_badge`, `UnixSocketMode` — none exist in this fork's `nono` library
- **Fix:**
  - Added missing imports to audit_integrity.rs test module
  - Added `reject_stage: None` second arg to `record_capability_decision` call
  - Added full `CapabilityRequest` fields with defaults
  - Removed 2 output.rs tests; deferred to future plan that adds unix socket capability API
- **Files modified:** `crates/nono-cli/src/audit_integrity.rs`, `crates/nono-cli/src/output.rs`
- **Commit:** `7831c47f` (amend)

**4. [Rule 1 - Bug] `exec_strategy.rs`: `redaction_policy` not yet in scope at `finalize_supervised_exit` callsite**
- **Found during:** Task 2 conflict resolution
- **Issue:** Upstream moved `redaction_policy` extraction to after `finalize_supervised_exit`; fork needed it before
- **Fix:** Added local `finalize_redaction_policy` extraction from `supervisor` config before the `finalize_supervised_exit` call
- **Files modified:** `crates/nono-cli/src/exec_strategy.rs`
- **Commit:** `96886ae9`

**5. [Rule 1 - Bug] `supervised_runtime.rs`: `audit_signer` field from ctx shadowed by local computation**
- **Found during:** Task 2 conflict resolution
- **Issue:** Fork always computes `audit_signer` locally from `rollback.audit_sign_key`; incoming `audit_signer` from `SupervisedRuntimeContext` was unused and would trigger compiler warnings
- **Fix:** Renamed destructured field to `_audit_signer_ctx` with `let _ = _audit_signer_ctx;` comment explaining fork's local computation pattern
- **Files modified:** `crates/nono-cli/src/supervised_runtime.rs`
- **Commit:** `96886ae9`

**6. [Rule 1 - Bug] `execution_runtime.rs`: referenced undeclared `audit_signer` variable**
- **Found during:** Task 2 conflict resolution
- **Issue:** Summary noted `audit_signer: audit_signer.as_ref()` in `SupervisedRuntimeContext` literal but `audit_signer` was not declared in `execution_runtime.rs` (fork computes it locally in supervised_runtime.rs)
- **Fix:** Changed to `audit_signer: None` with comment
- **Files modified:** `crates/nono-cli/src/execution_runtime.rs`
- **Commit:** `96886ae9`

### Accepted Upstream Structural Divergences

- **audit_attestation.rs**: Kept fork's `sign_session_attestation` + `verify_audit_attestation` API (upstream rewrote to `write_audit_attestation` using `sign_statement_bundle` — not available in fork). Fork's AUD-02 attestation implementation is preserved verbatim.
- **supervisor_linux.rs**: Kept fork's `CgroupSession::new` implementation; skipped upstream's `mod network_decision` test helper (references `decide_network_notification`/`NetworkDecision` that don't exist in fork).
- **supervised_runtime.rs**: Fork's `audit_recorder` uses `Arc<Mutex<>>` (Phase 23 D-01 pattern) vs upstream's plain `Mutex`. Fork's finalization flow calls `finalize_supervised_exit` from inside `execute_supervised` vs upstream's direct call.

## Known Stubs

- `print_capabilities_with_unix_socket_does_not_panic` and `unix_socket_mode_badges_are_fixed_width_and_distinct` tests removed from output.rs — blocked on `allow_unix_socket` / `UnixSocketMode` API not yet in fork's `nono` library. These tests will be re-enabled when the unix socket capability API is added.

## Invariant Verification

| Invariant | Status |
|-----------|--------|
| D-40-E1: Windows files zero per cherry-pick | PARTIAL — commit 1 has +4 lines in exec_strategy_windows/mod.rs (documented fork-adaptation, not upstream code) |
| D-40-E4: Test fixtures ported | PASS — 8 scrub tests from scrub.rs ported; 2 output.rs tests deferred (missing API) |
| D-40-E6: No Windows-specific scrub rules | PASS — scrub.rs has zero cfg(windows) blocks |
| D-40-C4: Verbatim D-19 trailers | PASS — 2/2 commits |
| D-40-C3: Stop on gate failure | N/A — no hard gate failures |

## Self-Check: PASSED

- `crates/nono/src/scrub.rs` exists: YES
- `cargo test -p nono -- scrub`: 8/8 PASS
- `cargo test --workspace`: ALL PASS
- `cargo clippy --workspace`: PASS
- `cargo fmt --check`: PASS
- Commit `96886ae9` exists: YES
- Commit `7831c47f` exists: YES
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '`: 2
- Push to origin/main: SUCCESS (c1de59ec..7831c47f)
