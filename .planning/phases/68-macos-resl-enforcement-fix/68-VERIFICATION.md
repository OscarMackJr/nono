---
phase: 68-macos-resl-enforcement-fix
verified: 2026-06-12T20:00:00Z
status: passed
score: 4/4
overrides_applied: 1
overrides:
  - must_have: "cross-target clippy verified on Linux and macOS targets"
    reason: "Windows dev host cannot cross-compile Apple or Linux cfg targets (ring/aws-lc-sys C toolchain absent). PARTIAL/deferred-to-CI is the project-standard disposition per .planning/templates/cross-target-verify-checklist.md and CLAUDE.md. The load-bearing gate is the real macOS host build (828a332c compiled cleanly on Oscars-MacBook-Pro with 0 errors, 0 warnings), which is the only environment that exercises the Apple cfg arms. GH Actions lanes carry the remaining signal."
    accepted_by: "oscarmackjr-twg"
    accepted_at: "2026-06-12T18:16:52Z"
---

# Phase 68: macOS Resource-Limit Enforcement Fix — Verification Report

**Phase Goal:** `nono run --timeout` and `--max-processes` deliver REAL enforcement on a real macOS host (REQ-RESL-NIX-03 defect, the Phase 65 gate-65-A "A5" finding). SC1: `--timeout <D>` SIGKILLs the child at the deadline. SC2: `--max-processes <N>` makes fork() EAGAIN past the cap via setrlimit(RLIMIT_NPROC). SC3: `macos_timeout_kills_at_deadline` + `macos_max_processes_blocks_on_rlimit_nproc` PASS with NONO_RESL_HOST_VALIDATED=1. SC4: cross-target clippy verified + macOS CI green.
**Verified:** 2026-06-12T20:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `--timeout <D>` SIGKILLs the child at the deadline on a real macOS host | VERIFIED | `macos_timeout_kills_at_deadline` PASS (SIGKILL exit 137 at ~5s). Confirmed by `setpgid(0,0)` in child arm (68-01, `f94c1c1b`), parent-side `setpgid(child, child)` belt-and-suspenders (68-02, `924b4d60`), and watchdog at `exec_strategy.rs:1524` (`match getpgid( ...spawn_macos_timeout_watchdog`). RESL-MAC-01. |
| 2 | `--max-processes <N>` makes fork() EAGAIN past the cap on a real macOS host | VERIFIED | `macos_max_processes_blocks_on_rlimit_nproc` PASS (ulimit -u ≈ 474, was 824). Root cause (loose single-NULL-buffer `proc_listpids` baseline) fixed by two-call pattern in `supervisor_macos.rs::uid_process_count()` (`828a332c`). RESL-MAC-02. |
| 3 | Both gated tests (`macos_timeout_kills_at_deadline` + `macos_max_processes_blocks_on_rlimit_nproc`) PASS with `NONO_RESL_HOST_VALIDATED=1` on a real macOS host | VERIFIED | Host UAT on Oscars-MacBook-Pro at head `828a332c`: 5 passed; 0 failed. Full output documented in 68-02-SUMMARY.md and 68-VALIDATION.md (nyquist_compliant: true, t4_host_uat: pass). |
| 4 | Cross-target clippy verified + macOS CI green | PASSED (override) | Windows dev host cannot cross-compile Apple/Linux targets. macOS host build at `828a332c` compiled with 0 errors, 0 warnings — this IS the load-bearing gate per project standard. GH Actions carries remaining CI signal. Override accepted per CLAUDE.md cross-target-verify-checklist.md. |

**Score:** 4/4 truths verified (1 via override)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/exec_strategy.rs` | D3 parent-setpgid + D1 platform-gated set_read_timeout + D2 RLIMIT_AS warn-and-continue | VERIFIED | `setpgid(child, child)` at line 1494; `#[cfg(target_os = "linux")]` on `set_read_timeout` block at line 1387; `MSG_RLIMIT_AS_WARN` at line 1006 with warn-and-continue (no `_exit(126)`). 13 `const MSG_*` lines (>= 11 required). Zero `let _ = setrlimit` (WR-02). `match getpgid(` preserved at line 1524 (WR-04). |
| `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` | Two-call `proc_listpids` `uid_process_count()` for accurate per-UID baseline + D2 Direct-path RLIMIT_AS best-effort | VERIFIED | `uid_process_count()` uses two-call `proc_listpids(PROC_UID_ONLY)` pattern (size-query then real-buffer call). `install_pre_exec` RLIMIT_AS block changed to `is_err()` check with silent continue (no propagating `?`). Documented D-03 UID-wide semantics inline. |
| `crates/nono-cli/tests/resl_nix_macos.rs` | `run_bounded` with `Stdio::null()` stdin; D-09 assertion flipped to `output.status.success()`; `macos_max_processes_blocks_on_rlimit_nproc` uses jobs-rp count detection | VERIFIED | Line 39: `stdin(Stdio::null())`. Line 310: `assert!(output.status.success(), ...)`. `macos_max_processes_blocks_on_rlimit_nproc` at line 329 spawns 50 `sleep`s and asserts `!output.status.success()` via `jobs -rp` count gate. All 5 tests PASS with `NONO_RESL_HOST_VALIDATED=1`. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `exec_strategy.rs` ForkResult::Parent arm (~line 1491) | `supervisor_macos::spawn_macos_timeout_watchdog(deadline, child_pgrp)` | `#[cfg(target_os = "macos")] setpgid(child, child)` before `match getpgid(` | VERIFIED | `setpgid(child, child)` at line 1494 closes the fork/exec race; `match getpgid(Some(child))` at line 1524 reads the now-reliable pgrp and passes it to the watchdog. |
| `exec_strategy.rs` `#[cfg(target_os = "linux")]` set_read_timeout block (~line 1387) | Linux-only SO_RCVTIMEO path | `#[cfg(target_os = "linux")]` replaces former `#[cfg(unix)]` | VERIFIED | Pattern `cfg\(target_os = .linux.\)` present at line 1387 with comment explaining macOS AF_UNIX SO_RCVTIMEO kernel limitation. |
| `exec_strategy.rs` RLIMIT_AS child block (~line 1005) | warn-and-continue (no `_exit(126)`) | `MSG_RLIMIT_AS_WARN` + `libc::write` inside CR-01 region | VERIFIED | `MSG_RLIMIT_AS_WARN` const at line 1006; `libc::write` at line 1011; no `libc::_exit(126)` on macOS RLIMIT_AS path. RLIMIT_NPROC path retains fail-closed `_exit(126)`. |
| `supervisor_macos.rs::uid_process_count()` | `MacosResourceLimits::new()` baseline_uid_count | two-call `proc_listpids(PROC_UID_ONLY)` | VERIFIED | `uid_process_count()?` called at line 195 in `MacosResourceLimits::new()` when `max_processes.is_some()`. Two-call pattern returns accurate count (~476 vs over-reported ~819). |

---

### Data-Flow Trace (Level 4)

Not applicable — this phase modifies Unix cfg-gated enforcement paths and test infrastructure, not components that render dynamic data. The relevant data flow (per-UID process count → RLIMIT_NPROC cap) is verified structurally in the key links above and confirmed by the real-host UAT result (RLIMIT_NPROC=~481 vs the previous ~824 that never fired).

---

### Behavioral Spot-Checks

The decisive behavioral checks require a real macOS host (CI runners hang on the enforcement tests, which is the stated reason for the env-gate). The host UAT is the load-bearing behavioral check.

| Behavior | Evidence | Status |
|----------|----------|--------|
| `--timeout 5s` SIGKILLs at ~5s on macOS | `macos_timeout_kills_at_deadline` PASS on Oscars-MacBook-Pro, head 828a332c | PASS (host-verified) |
| `--max-processes 5` causes EAGAIN past cap (ulimit -u ≈ 474) | `macos_max_processes_blocks_on_rlimit_nproc` PASS on Oscars-MacBook-Pro, head 828a332c | PASS (host-verified) |
| `--memory 32m` no longer aborts supervised run (`_exit(126)` gone) | `macos_memory_limit_kills_at_rlimit_as` PASS (asserts `success()`, not `!success()`) on host | PASS (host-verified) |
| Windows dev-host source-scan invariants (CR-01, WR-02, WR-04) | `resl_nix_async_signal_safety` 5/5 green (verifiable on Windows): 13 `const MSG_*` >= 11, zero `let _ = setrlimit` in exec_strategy.rs, `match getpgid(` present, zero `unwrap_or(child)` | PASS (dev-host verified) |

---

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` probes declared for this phase. The load-bearing gate is the host UAT documented in 68-VALIDATION.md (t4_host_uat: pass, nyquist_compliant: true, approved 2026-06-12).

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RESL-MAC-01 | 68-01-PLAN.md, 68-02-PLAN.md | `nono run --timeout <D>` SIGKILLs the child at deadline on a real macOS host | SATISFIED | `macos_timeout_kills_at_deadline` PASS host UAT; watchdog wiring confirmed in codebase (setpgid + match getpgid + spawn_macos_timeout_watchdog) |
| RESL-MAC-02 | 68-01-PLAN.md, 68-02-PLAN.md | `nono run --max-processes <N>` makes fork() EAGAIN past cap via setrlimit(RLIMIT_NPROC) | SATISFIED | `macos_max_processes_blocks_on_rlimit_nproc` PASS host UAT; two-call `uid_process_count()` tight baseline (~474) confirmed in `supervisor_macos.rs` |

---

### Anti-Patterns Found

No `TBD`, `FIXME`, or `XXX` markers found in any of the three modified files (`exec_strategy.rs`, `supervisor_macos.rs`, `resl_nix_macos.rs`). No unresolved debt markers.

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| — | None found | — | — |

The `let _ = setrlimit(...)` pattern in `supervisor_macos.rs` is intentional (D2 Direct-path best-effort, documented, inside a file the WR-02 source-scan test does not cover) and is not a stub — it is explicitly the designed best-effort behaviour for a documented macOS arm64 kernel limitation.

---

### Human Verification Required

None. All load-bearing behaviours were verified on a real macOS host (Oscars-MacBook-Pro, head `828a332c`, 2026-06-12). No outstanding items require further human testing for this phase.

---

### Gaps Summary

No gaps. All four Success Criteria are satisfied:

1. SC1 (`--timeout` SIGKILLs at deadline): VERIFIED by codebase and host UAT.
2. SC2 (`--max-processes` EAGAIN past cap): VERIFIED by codebase and host UAT.
3. SC3 (both gated tests PASS): VERIFIED by host UAT 5/5 at head `828a332c`.
4. SC4 (cross-target clippy + macOS CI green): PASSED (override) — macOS host build is the load-bearing gate, satisfied; Windows cross-compile PARTIAL/deferred-to-CI per project standard.

The phase closed a multi-layer defect (D1 SO_RCVTIMEO, D2 RLIMIT_AS, D3 watchdog pgrp race, test-harness cwd-prompt hang, loose `proc_listpids` baseline) that collectively prevented enforcement from firing. Each layer is fixed, wired, and confirmed by the real-host UAT.

---

_Verified: 2026-06-12T20:00:00Z_
_Verifier: Claude (gsd-verifier)_
