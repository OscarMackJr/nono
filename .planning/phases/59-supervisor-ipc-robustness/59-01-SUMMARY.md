---
phase: 59-supervisor-ipc-robustness
plan: 01
subsystem: testing
tags: [supervisor-ipc, timeouts, integration-testing, rust, windows, unix]

# Dependency graph
requires: []
provides:
  - "SUPERVISOR_IPC_READ_TIMEOUT const (5s, D-01) + supervisor_ipc_read_timeout() accessor in timeouts.rs"
  - "Four in-crate unit tests covering default/override/clamp/invalid-fallback for the new timeout knob"
  - "Wave-0 Unix integration test scaffold (supervisor_ipc_robustness_unix.rs) with labeled 59-02 insertion points"
  - "Wave-0 Windows integration test scaffold (supervisor_ipc_robustness_windows.rs) with labeled 59-03 insertion points"
affects:
  - 59-02 (Unix keep-alive + read-timeout wiring — OWNED by supervisor_ipc_robustness_unix.rs)
  - 59-03 (Windows PeekNamedPipe bounded-read + re-accept — OWNED by supervisor_ipc_robustness_windows.rs)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SUPERVISOR_IPC_READ_TIMEOUT / supervisor_ipc_read_timeout() mirrors DETACH_STARTUP_TIMEOUT / detach_startup_timeout() pair (env_duration_secs + MAX_TIMEOUT clamp)"
    - "In-crate #[cfg(test)] mod tests in timeouts.rs for bin-only crate (integration tests in tests/ cannot reach private crate items)"
    - "EnvVarGuard::set_all + lock_env() for parallel-safe env var mutation in tests"
    - "File-level #![cfg(unix)] / #![cfg(target_os = 'windows')] for empty-binary-on-other-platform integration test gating"

key-files:
  created:
    - crates/nono-cli/tests/supervisor_ipc_robustness_unix.rs
    - crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs
  modified:
    - crates/nono-cli/src/timeouts.rs

key-decisions:
  - "SUPERVISOR_IPC_READ_TIMEOUT is NOT cfg-gated: Windows and Unix both read the same 5s default; env_duration_secs is non-cfg'd (D-02)"
  - "#[allow(dead_code)] on const + accessor: callers are wired in Wave-2 plans 59-02 and 59-03; CLAUDE.md dead_code avoidance acknowledged — tests DO use the items but #[cfg(test)] usage does not satisfy the lint in the non-test binary target"
  - "Integration test scaffolds are split per platform (unix.rs / windows.rs) not combined: gives 59-02 and 59-03 exclusive files_modified ownership, enabling Wave-2 parallel execution with zero overlap"
  - "Unix scaffold uses SupervisorSocket::pair() as the real nono lib surface linkage test; Windows scaffold uses bind_aipc_pipe (PipeDirection::Read) matching the existing aipc_handle_brokering_integration.rs pattern"

patterns-established:
  - "Pattern: bin-only crate timeout tests go in the module's own #[cfg(test)] block, NOT in tests/ directory"
  - "Pattern: per-platform test scaffolds carry file-level cfg gate + empty_binary contract comment"

requirements-completed: [REQ-IPC-01]

# Metrics
duration: 11min
completed: 2026-06-06
---

# Phase 59 Plan 01: Supervisor IPC Robustness - Wave-0 Substrate Summary

**SUPERVISOR_IPC_READ_TIMEOUT const (5s) + env-override accessor with 4 parse/clamp/default in-crate tests, plus two per-platform empty-scaffold integration test files giving 59-02/59-03 exclusive file ownership in Wave 2**

## Performance

- **Duration:** 11 min
- **Started:** 2026-06-06T13:24:13Z
- **Completed:** 2026-06-06T13:35:22Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Added `SUPERVISOR_IPC_READ_TIMEOUT` (5s, matching upstream d1851c9) and `supervisor_ipc_read_timeout()` accessor to `timeouts.rs`, un-cfg'd, delegating to the shared `env_duration_secs` helper with `MAX_TIMEOUT` (3600s) clamp (D-01 / T-59-02 threat mitigated)
- Created four in-crate unit tests in `timeouts::tests` covering default (env absent → 5s), env override (`=1` → 1s), clamp (`=99999` → MAX_TIMEOUT), and invalid-fallback (`=abc` → 5s); all use `EnvVarGuard` + `lock_env()` for safe parallel env mutation
- Created `supervisor_ipc_robustness_unix.rs` with `#![cfg(unix)]` gate, a real `scaffold_links_nono_lib` test calling `SupervisorSocket::pair()`, and labeled TODO insertion points for 59-02 SC1/SC2
- Created `supervisor_ipc_robustness_windows.rs` with `#![cfg(target_os = "windows")]` gate, a real `scaffold_links_nono_lib` test calling `bind_aipc_pipe`, and labeled TODO placeholders for 59-03 SC4 bounded_read/re_accept

## Task Commits

Each task was committed atomically:

1. **Task 1: SUPERVISOR_IPC_READ_TIMEOUT const + accessor + in-crate unit tests** - `deba18ae` (feat)
2. **Task 2: Wave-0 per-platform integration test scaffolds** - `13c01b3d` (feat)

## Files Created/Modified

- `crates/nono-cli/src/timeouts.rs` - Added `SUPERVISOR_IPC_READ_TIMEOUT` const (5s), `supervisor_ipc_read_timeout()` accessor, and `#[cfg(test)] mod tests` with 4 parse/clamp/default unit tests
- `crates/nono-cli/tests/supervisor_ipc_robustness_unix.rs` - New: `#![cfg(unix)]` scaffold, `scaffold_links_nono_lib()` using `SupervisorSocket::pair()`, TODO placeholders for 59-02
- `crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs` - New: `#![cfg(target_os = "windows")]` scaffold, `scaffold_links_nono_lib()` using `bind_aipc_pipe`, TODO placeholders for 59-03

## Decisions Made

- **Un-cfg'd timeout const**: `SUPERVISOR_IPC_READ_TIMEOUT` is not `#[cfg(unix)]` because Windows reads the same 5s default for the named-pipe bounded-read path (D-02). `env_duration_secs` is already non-cfg'd — no platform split needed.
- **`#[allow(dead_code)]` with comment**: the const and accessor have no non-test callers yet (Wave-2 plans wire them). Tests DO use them, but `#[cfg(test)]` usage doesn't satisfy the lint in the non-test binary target. Attribute is scoped to the two items only, with comments identifying 59-02 and 59-03 as the caller plans.
- **File-split over per-test cfg**: using file-level cfg gates (`#![cfg(unix)]` vs `#![cfg(target_os = "windows")]`) rather than per-test `#[cfg]` gives 59-02 and 59-03 exclusive file ownership with no shared-file write in Wave 2.
- **`PipeDirection::Read` in Windows scaffold**: `Duplex` variant does not exist; `Read` matches the `aipc_handle_brokering_integration.rs` precedent and is sufficient to prove the library surface links.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed disallowed `std::env::set_var`/`remove_var` in test code**
- **Found during:** Task 1 (timeouts.rs unit tests — clippy all-targets run)
- **Issue:** Direct `std::env::set_var` and `std::env::remove_var` calls in test code trigger `clippy::disallowed_methods` (-D warnings in CI). The workspace has an allow-list managed by `clippy.toml`/`Cargo.toml` that forbids direct env mutation outside the `test_env` wrappers.
- **Fix:** Replaced all direct env var mutation with `EnvVarGuard::set_all(...)` + `_guard.remove(...)` from `crate::test_env`. For the "absent" default test: set a sentinel value via `set_all` then immediately `remove()`; Drop restores the original automatically.
- **Files modified:** `crates/nono-cli/src/timeouts.rs`
- **Verification:** `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` exits 0 with no new errors.
- **Committed in:** `deba18ae` (Task 1 commit)

**2. [Rule 1 - Bug] Fixed `PipeDirection::Duplex` variant not found**
- **Found during:** Task 2 (Windows scaffold first compile attempt)
- **Issue:** Used non-existent `PipeDirection::Duplex`; enum has `Read`, `Write`, `ReadWrite` variants.
- **Fix:** Changed to `PipeDirection::Read` (sufficient for the sanity link test; matches aipc integration test precedent).
- **Files modified:** `crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs`
- **Verification:** `cargo test -p nono-cli --test supervisor_ipc_robustness_windows` exits 0, `scaffold_links_nono_lib` PASS.
- **Committed in:** `13c01b3d` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — compile-time bugs caught immediately during verification)
**Impact on plan:** Both fixes necessary for correctness. No scope creep. Plan executed as designed.

## Issues Encountered

- Pre-existing `make clippy` failures in `nono-proxy` (3 `disallowed_methods`) and `nono-cli` bin test target (`oauth2_cred_builder` dead_code + `field_reassign_with_default` in `profile/mod.rs` + `unwrap_err()` in `offline_verify_extended_trust_bundle`) are NOT introduced by this plan — confirmed by targeted `cargo clippy -p nono-cli --bin nono` which is green for our changes. The workspace-wide `-D warnings` gate blocks on these pre-existing issues; out of scope per deviation scope-boundary rule.

## Cross-Target Clippy Note

CLAUDE.md MUST/NEVER rule: the new `supervisor_ipc_robustness_unix.rs` contains `#![cfg(unix)]` code. Cross-target Linux/macOS clippy verification is **PARTIAL / deferred to live CI** — Windows-host `cargo check` does not exercise these branches. The `scaffold_links_nono_lib` test on the Unix side calls `SupervisorSocket::pair()` which is Unix-only; compile error on Windows is prevented by the file-level `#![cfg(unix)]` gate.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **59-02 (Unix)**: `supervisor_ipc_robustness_unix.rs` scaffold is in place with labeled insertion points for `reconnect_survival` (SC1) and `bounded_read_timeout` (SC2). `supervisor_ipc_read_timeout()` is callable from `exec_strategy.rs` as `crate::timeouts::supervisor_ipc_read_timeout()`.
- **59-03 (Windows)**: `supervisor_ipc_robustness_windows.rs` scaffold is in place with labeled insertion points for `bounded_read` (SC4) and `re_accept` (SC4). Same accessor callable from `exec_strategy_windows/supervisor.rs`.
- Both Wave-2 plans have exclusive file ownership → zero `files_modified` overlap → safe parallel execution.

## Known Stubs

None — this plan creates infrastructure (const, accessor, and test scaffolds), not UI or data-wiring components.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The new env var `NONO_SUPERVISOR_IPC_READ_TIMEOUT` is parsed by `env_duration_secs` which already clamps to `MAX_TIMEOUT` and falls back to the safe default — T-59-02 mitigated as specified.

---
*Phase: 59-supervisor-ipc-robustness*
*Completed: 2026-06-06*
