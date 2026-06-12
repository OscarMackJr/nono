---
phase: 68-macos-resl-enforcement-fix
plan: "01"
subsystem: exec_strategy/supervisor_macos
tags:
  - macos
  - resource-limits
  - rlimit
  - setpgid
  - watchdog
  - async-signal-safety
dependency_graph:
  requires:
    - "Phase 65 gate-65-A A5 finding (RESL-MAC-01/02 defect)"
  provides:
    - "RESL-MAC-01: setpgid(0,0) makes child own-pgrp leader; watchdog kills correctly"
    - "RESL-MAC-02: real RLIMIT_NPROC enforcement via libc::setrlimit + baseline+N"
  affects:
    - "crates/nono-cli/src/exec_strategy.rs (supervised path)"
    - "crates/nono-cli/src/exec_strategy/supervisor_macos.rs (direct path)"
tech_stack:
  added: []
  patterns:
    - "Raw libc::setrlimit(RLIMIT_NPROC) bypassing nix (which lacks macOS support for this resource)"
    - "sysctl(KERN_PROC_UID) in parent before fork for baseline UID process count"
    - "setpgid(0,0) post-fork child arm for deterministic process group isolation"
    - "const MSG_*: &[u8] + libc::write + libc::_exit for async-signal-safe error reporting"
key_files:
  created: []
  modified:
    - "crates/nono-cli/src/exec_strategy/supervisor_macos.rs"
    - "crates/nono-cli/src/exec_strategy.rs"
    - "crates/nono-cli/tests/resl_nix_macos.rs"
decisions:
  - "D-01: baseline+N bounding — parent reads per-UID count pre-fork via sysctl(KERN_PROC_UID); RLIMIT_NPROC = count + N"
  - "D-02: raw libc::setrlimit(libc::RLIMIT_NPROC) in both Direct (install_pre_exec) and Supervised (ForkResult::Child arm) paths"
  - "D-03: UID-wide RLIMIT_NPROC semantics documented inline; inherently racy, accepted behavior"
  - "D-04: setpgid(0,0) in child arm immediately post-fork; tolerates failure per D-06"
  - "D-05: fix covers both PTY and non-PTY supervised paths (setsid in PTY path supersedes; non-PTY is the primary fix site)"
  - "D-06: WR-04 preserved — no PID fallback on getpgid failure; setpgid failure writes MSG and continues"
  - "D-07: fail-closed — parent sysctl failure returns NonoError; child setrlimit failure calls _exit(126)"
  - "D-09: bonus macos_memory_limit_kills_at_rlimit_as test added to resl_nix_macos.rs (secondary; no new req)"
  - "D-10: cross-target clippy PARTIAL/deferred-to-CI — Windows dev host cannot cross-compile"
metrics:
  duration: "~25 min (Tasks 1+2+3-automated)"
  completed: "2026-06-12 (Tasks 1+2+3-automated; Task 3 human gate pending)"
  tasks_total: 3
  tasks_completed: 2
  tasks_in_progress: 1
---

# Phase 68 Plan 01: macOS Resource-Limit Enforcement Fix Summary

**One-liner:** Real RLIMIT_NPROC enforcement via libc::setrlimit + setpgid(0,0) child process group isolation fixing the watchdog kill miss on macOS.

## Status

Tasks 1 and 2 are complete. Task 3 is in-progress — the automated steps (D-09 bonus test + Windows host suite + cross-target deferred status) are done; the human-verify gate (real macOS host UAT with `NONO_RESL_HOST_VALIDATED=1`) is awaiting the user.

## What Was Built

### Task 1: uid_process_count + baseline_uid_count + Direct-path RLIMIT_NPROC fix

**File:** `crates/nono-cli/src/exec_strategy/supervisor_macos.rs`

- Added `uid_process_count()` function: calls `sysctl(KERN_PROC_UID, getuid())` in the parent before fork, returns the per-UID process count as `u64`, returns `Err(NonoError::SandboxInit(...))` on sysctl failure (fail-closed per D-07).
- Added `baseline_uid_count: u64` field to `MacosResourceLimits` struct: precomputed in the parent, captured by `Copy` into the `pre_exec` closure. Added `baseline_uid_count()` accessor method.
- Updated `MacosResourceLimits::new()`: calls `uid_process_count()?` when `max_processes.is_some()`; propagates `Err` fail-closed. Zero sysctl when `max_processes.is_none()`.
- Fixed `install_pre_exec` (Direct path): replaced `tracing::warn!` RLIMIT_NPROC no-op with real `libc::setrlimit(RLIMIT_NPROC, baseline+N)`. Reads existing hard limit via `libc::getrlimit` first to avoid EPERM (Pitfall 3). Returns `Err(std::io::Error::last_os_error())` on failure (fail-closed).
- Documented D-03 UID-wide semantics inline.

### Task 2: Supervised-path fix — setpgid(0,0) + MSG_RLIMIT_NPROC_FAIL

**File:** `crates/nono-cli/src/exec_strategy.rs`

- Added `macos_baseline_uid_count: u64` extraction from `MacosResourceLimits` before fork (parent side, via new accessor).
- Added `setpgid(0,0)` block in `ForkResult::Child` arm (macOS, before rlimits block): child becomes own process-group leader so timeout watchdog `kill(-child_pgrp, SIGKILL)` targets only the agent tree. Failure tolerated (writes `MSG_SETPGID_FAIL` + continues) per D-06/RESEARCH Q1; WR-04 skip-on-Err is the safety net.
- Replaced `MSG_RLIMIT_NPROC_UNAVAILABLE` no-op with real `MSG_RLIMIT_NPROC_FAIL` enforcement: `libc::getrlimit` reads hard limit, `saturating_add` computes `baseline+N`, `libc::setrlimit(RLIMIT_NPROC)` applies the limit. On failure: writes `MSG_RLIMIT_NPROC_FAIL` + `libc::_exit(126)` (fail-closed per D-07).
- All CR-01 invariants preserved: zero `format!()` in child arm, all error messages are `const MSG_*: &[u8]` byte string literals. WR-04 `match getpgid()` block unchanged. WR-02 no silent setrlimit discards.

### Task 3 Automated Steps: D-09 bonus test + Windows suite + cross-target deferred status

**File:** `crates/nono-cli/tests/resl_nix_macos.rs`

- Added `macos_memory_limit_kills_at_rlimit_as()` test: env-gated on `NONO_RESL_HOST_VALIDATED=1`; skips gracefully if `python3` unavailable; asserts `!output.status.success()` after attempting 256 MB `bytearray` allocation under `--memory 32m`. Secondary per D-09 (no new requirement).

**Windows host suite:** 1211 passed, 4 pre-existing env-specific failures (profile_cmd init + 3 protected_paths — documented in project memory).

**Cross-target clippy:** PARTIAL/deferred-to-CI. Windows dev host cannot run `--target x86_64-unknown-linux-gnu` or `--target x86_64-apple-darwin` (ring/aws-lc-sys C toolchain missing). GH Actions Linux + macOS Clippy lanes on head SHA are the decisive signal per `.planning/templates/cross-target-verify-checklist.md`.

## Task 3 Human Gate (Pending)

**Status: AWAITING USER — checkpoint:human-verify (blocking)**

The user must run the gated tests on `Oscars-MacBook-Pro` with `NONO_RESL_HOST_VALIDATED=1`. See checkpoint message below.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| Task 1 | `1b2e2ad0` | feat(68-01): add uid_process_count + baseline_uid_count + Direct-path RLIMIT_NPROC fix |
| Task 2 | `f94c1c1b` | feat(68-01): setpgid(0,0) in child arm + MSG_RLIMIT_NPROC_FAIL real enforcement |
| Task 3 (automated) | `3583bacc` | test(68-01): add D-09 bonus macos_memory_limit_kills_at_rlimit_as test |

## Automated Verification Results (Windows Dev Host)

| Check | Result |
|-------|--------|
| `cargo test -p nono-cli --test resl_nix_async_signal_safety` (5 tests) | ✅ all green |
| `cr_01_no_format_macro_in_post_fork_child_branch` | ✅ pass |
| `cr_01_and_wr_02_const_msg_byte_strings_present` (>= 11 consts) | ✅ pass |
| `wr_04_no_pid_fallback_on_getpgid_failure` | ✅ pass |
| `wr_02_no_silent_setrlimit_discards` | ✅ pass |
| `cr_02_direct_mode_timeout_emits_warn_macro` | ✅ pass |
| `cargo test -p nono-cli` (full suite) | ✅ 1211 pass (4 pre-existing) |
| `grep MSG_RLIMIT_NPROC_FAIL exec_strategy.rs >= 1` | ✅ 3 |
| `grep RLIMIT_NPROC_UNAVAILABLE exec_strategy.rs == 0` | ✅ 0 |
| `grep uid_process_count supervisor_macos.rs >= 1` | ✅ 1 |
| `grep setpgid exec_strategy.rs >= 1` | ✅ 5 |
| `grep "match getpgid(" exec_strategy.rs >= 1` | ✅ 1 |
| `grep "unwrap_or(child)" exec_strategy.rs == 0` | ✅ 0 |

## Deviations from Plan

None — plan executed exactly as written. All decisions (D-01 through D-10) followed.

## Known Stubs

None — no stubs or placeholders in the implementation.

## Threat Flags

None — all changes are within the threat model defined in the plan. No new security-relevant surfaces introduced beyond the intended RLIMIT_NPROC and setpgid enforcement.

## Self-Check

### Created files exist:
- `68-01-SUMMARY.md`: FOUND (this file)
- `68-VALIDATION.md` (updated): FOUND

### Commits exist:
- `1b2e2ad0`: Task 1 — feat(68-01)
- `f94c1c1b`: Task 2 — feat(68-01)
- `3583bacc`: Task 3 automated — test(68-01)

## Self-Check: PASSED

All code committed. SUMMARY written. VALIDATION updated. Task 3 human gate pending.
