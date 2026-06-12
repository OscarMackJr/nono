---
phase: 68-macos-resl-enforcement-fix
plan: "02"
subsystem: sandbox
tags: [macos, setpgid, rlimit, rlimit_nproc, proc_listpids, supervised-execution, watchdog, test-harness]
status: complete

# Dependency graph
requires:
  - phase: 68-01
    provides: "setpgid(0,0) in child arm + RLIMIT_NPROC enforcement (Direct+Supervised)"
provides:
  - "D1 SO_RCVTIMEO gated to Linux only (macOS AF_UNIX EINVAL fix)"
  - "D2 RLIMIT_AS abort downgraded to warn-and-continue on macOS arm64 (best-effort)"
  - "D3 timeout watchdog confirmed working on real host (child setpgid(0,0) carries it)"
  - "RESL-MAC-02: accurate two-call proc_listpids baseline (tight, enforceable RLIMIT_NPROC cap)"
  - "Test-harness stdin fix (run_bounded) + enforcement-detecting max-processes assertion"
affects: [macos-resl, supervised-run, timeout-watchdog, memory-limit, max-processes]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Platform-gated cfg(target_os = linux) for SO_RCVTIMEO (macOS AF_UNIX kernel limitation)"
    - "Best-effort RLIMIT_AS on macOS: warn-and-continue instead of fail-closed abort"
    - "Two-call proc_listpids(PROC_UID_ONLY) for an accurate per-UID process count"
    - "Test harness: Stdio::null() stdin so the interactive cwd prompt EOFs instead of blocking"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy/supervisor_macos.rs
    - crates/nono-cli/tests/resl_nix_macos.rs

key-decisions:
  - "D3: the parent-side setpgid(child,child) double-setpgid is ineffective (parent loses the fork/exec race → EACCES/ESRCH) but harmless; the child's own setpgid(0,0) reliably carries the watchdog. Watchdog enforcement confirmed working on the real host (SIGKILL 137 at the deadline). Kept as belt-and-suspenders."
  - "D1 SO_RCVTIMEO: Linux-only guard replaces unix-wide guard; macOS rejects SO_RCVTIMEO on AF_UNIX; supervisor loop already polls (200ms) so the read-timeout removal is safe on macOS."
  - "D2 RLIMIT_AS: best-effort on macOS (dyld pre-maps VAS; setrlimit EINVAL is a documented kernel limitation). Deeper enforcement deferred to todo 20260612-macos-rlimit-as-setrlimit-fails.md."
  - "RESL-MAC-02 root cause was a LOOSE BASELINE, not a macOS limitation: single NULL-buffer proc_listpids over-reports (~819 vs real ~476) → RLIMIT_NPROC=baseline+N=824 too loose to fire. Fixed with the two-call pattern (~474 cap)."
  - "The gated-test 'non-firing' signal across the whole saga was largely a TEST-HARNESS bug: run_bounded inherited stdin, so nono blocked on the interactive cwd-share [y/N] prompt and never spawned the child. Fixed with Stdio::null()."

requirements-completed:
  - RESL-MAC-01
  - RESL-MAC-02

# Metrics
duration: ~1 session (2026-06-12, multi-cycle host-iterated debug)
completed: 2026-06-12
---

# Phase 68 Plan 02: macOS Supervised-Path Enforcement Fix — COMPLETE

**`--timeout` and `--max-processes` enforcement now fire on a real macOS host (Oscars-MacBook-Pro): `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos` → 5 passed; 0 failed. RESL-MAC-01 + RESL-MAC-02 satisfied.**

## Status: COMPLETE — macOS host UAT 5/5 PASS (2026-06-12)

Final gated-suite result on Oscars-MacBook-Pro (head `828a332c`):
```
macos_cpu_percent_rejected_at_clap_parse ........ ok
macos_no_warnings_on_resource_flags ............. ok
macos_memory_limit_kills_at_rlimit_as ........... ok   (D-09, best-effort)
macos_timeout_kills_at_deadline ................. ok   ← RESL-MAC-01
macos_max_processes_blocks_on_rlimit_nproc ...... ok   ← RESL-MAC-02
test result: ok. 5 passed; 0 failed
```
Direct confirmations on host: timeout watchdog kills the child with SIGKILL (exit 137) at the deadline; `--max-processes 5` now applies a tight `RLIMIT_NPROC` (`ulimit -u` ≈ 474, was 824) and the fork storm EAGAINs.

---

## The full story (why this took several host cycles)

The original 68-01 narrow fix (setpgid + RLIMIT_NPROC) didn't work on a real host. A host-iterated debug session (`.planning/debug/macos-resl-not-firing.md`) peeled back several layers:

1. **Deployment confound** — the first "fails" were a stale/undeployed binary (fix commits were unpushed) and then two macOS-only compile errors (`use nix::libc` missing; `libc::kinfo_proc` absent for Apple → rewrote `uid_process_count` to `proc_listpids`). Windows `cargo check` can't compile Apple cfg arms, so these slipped through.
2. **D1 + D2 (real defects, predate Phase 68)** — supervised `--memory` runs aborted on `set_read_timeout` EINVAL (SO_RCVTIMEO unsupported on macOS AF_UNIX) and on `setrlimit(RLIMIT_AS)` EINVAL (macOS arm64). Fixed: D1 platform-gate to Linux; D2 warn-and-continue best-effort.
3. **D3 (the watchdog) actually WORKS** — a manual `--allow-cwd --timeout 5s` run SIGKILLs the child at the deadline (exit 137). The parent-side double-setpgid is ineffective (EACCES — parent loses the exec race) but harmless; the child's own `setpgid(0,0)` carries the watchdog targeting.
4. **The test-harness bug (the big masker)** — the gated tests hung not on enforcement but on nono's interactive `Share <cwd>? [y/N]` prompt: `run_bounded` inherited stdin so nono blocked reading the prompt and never spawned the child. Fixed with `Stdio::null()` (mirrors the passing `.output()` tests). → 4/5.
5. **RESL-MAC-02 (last piece)** — `--max-processes` ran but didn't block forks. Host probe: nono applied `RLIMIT_NPROC=824` for `--max-processes 5` while the real per-UID count was 476. The single NULL-buffer `proc_listpids` over-reports (~819) → cap far too loose. Fixed with the two-call `proc_listpids` pattern (accurate ~476 → tight ~481 cap) + a test that exits non-zero iff fewer than the requested processes start (bash's exit code can't see background-fork EAGAIN). → 5/5.

---

## Commits (this plan)

| # | What | Commit |
|---|------|--------|
| T1 | D3 parent-side setpgid(child,child) | `924b4d60` |
| T2 | D1 platform-gate set_read_timeout to Linux | `c3cf3855` |
| T3 | D2 RLIMIT_AS warn-and-continue + D-09 flip | `648c5856` |
| + | run_bounded stdin=null (test-harness cwd-prompt hang) | `b8822a55` |
| + | RESL-MAC-02: two-call proc_listpids baseline + enforcement-detecting test | `828a332c` |

(Plus deployment/compile fixes earlier in the phase: `1b2e2ad0`, `f94c1c1b`, `3583bacc`, `53501113`, `fa6c2dc6`.)

---

## Files Modified

- `crates/nono-cli/src/exec_strategy.rs` — D3 parent-setpgid; D1 `#[cfg(target_os = "linux")]` on the SO_RCVTIMEO block; D2 RLIMIT_AS child arm → `MSG_RLIMIT_AS_WARN` + continue (no `_exit(126)`).
- `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` — D2 Direct-path RLIMIT_AS best-effort; **two-call `proc_listpids` `uid_process_count()`** (accurate per-UID baseline).
- `crates/nono-cli/tests/resl_nix_macos.rs` — `run_bounded` `Stdio::null()` stdin; D-09 assertion flip; `macos_max_processes_blocks_on_rlimit_nproc` now spawns 50 and asserts non-zero exit iff forks were blocked.

---

## Verification

| Check | Result |
|-------|--------|
| **Real macOS host: `NONO_RESL_HOST_VALIDATED=1 cargo test --test resl_nix_macos`** | **5 passed / 0 failed** ✅ |
| RESL-MAC-01 (`macos_timeout_kills_at_deadline`) | PASS (SIGKILL at deadline) |
| RESL-MAC-02 (`macos_max_processes_blocks_on_rlimit_nproc`) | PASS (EAGAIN past cap; ulimit -u ≈ 474) |
| `cargo test -p nono-cli --test resl_nix_async_signal_safety` (Windows) | 5/5 (CR-01/WR-02/WR-04 preserved) |
| `cargo fmt -p nono-cli -- --check` | CLEAN |
| Cross-target clippy (Linux + macOS) | PARTIAL/deferred-to-CI (Windows host can't cross-compile) — macOS host build is the load-bearing gate, satisfied |

---

## Carry-forward / deferred

- **D2 (`--memory`/RLIMIT_AS) is best-effort on macOS**, not enforced — documented; deeper enforcement deferred to `.planning/todos/pending/20260612-macos-rlimit-as-setrlimit-fails.md`.
- **The parent-side double-setpgid is a no-op (always EACCES/ESRCH)** — harmless but could be removed in a future cleanup; left in as belt-and-suspenders.
- **Durable lesson:** the load-bearing gate for macOS cfg-gated code MUST be a real macOS build+test, not Windows `cargo check` (which let 2 compile errors + a fmt-debt slip through this phase). See `[[feedback_clippy_cross_target]]`.

---
*Phase: 68-macos-resl-enforcement-fix · Plan: 02 · Status: COMPLETE (host UAT 5/5, 2026-06-12)*
