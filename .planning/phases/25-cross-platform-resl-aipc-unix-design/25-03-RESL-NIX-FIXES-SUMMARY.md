---
phase: 25-cross-platform-resl-aipc-unix-design
plan: 03
subsystem: sandbox
tags: [linux, macos, exec-strategy, async-signal-safety, fail-closed, code-review-gap-closure]

requires:
  - phase: 25-cross-platform-resl-aipc-unix-design
    plan: 01
    provides: execute_supervised post-fork child branch + macOS setrlimit + watchdog scaffolding (the surface this plan hardens)

provides:
  - "Async-signal-safe error reporting in the post-fork child branch of execute_supervised — every error path uses const &[u8] + libc::write + libc::_exit; zero heap allocation"
  - "User-visible warning on --timeout in Direct strategy mode (warn! to tracing + eprintln! to stderr); fail-secure UX consistent with project principle"
  - "Fail-closed setrlimit handlers on macOS supervised child branch — RLIMIT_AS / RLIMIT_NPROC failures abort the child instead of silently dropping enforcement"
  - "Safe getpgid handling in macOS timeout watchdog spawn — match arm replaces the unwrap_or(child) PID fallback; on Err the watchdog is skipped and a warning logged, eliminating the PID-reuse wrong-pgrp kill risk"
  - "Static-analysis regression suite: crates/nono-cli/tests/resl_nix_async_signal_safety.rs with 5 source-text assertions enforcing the four properties cross-platform (runs on Windows CI as well)"

affects: [exec_strategy, supervised_runtime, launch_runtime]

tech-stack:
  added: []
  patterns:
    - "Static-byte-string error reporting in post-fork child: `const MSG_X: &[u8] = b\"...\\n\"; libc::write(STDERR_FILENO, MSG_X.as_ptr().cast(), MSG_X.len()); libc::_exit(126);` — extends the existing chdir handler pattern to all 9 child-branch error sites"
    - "Source-text static-analysis tests for structural code properties (e.g., 'no format!() in lexical region X'): scan source via env!(\"CARGO_MANIFEST_DIR\"), strip comments, count macro invocations, assert"
    - "Fail-closed pre-exec child resource setup: setrlimit failure -> static diagnostic + _exit(126), matching the Linux cgroup placement failure pattern"
    - "Match-on-Result instead of unwrap_or fallback for PID-sensitive operations: `match getpgid(...) { Ok(pgrp) => spawn_watchdog, Err(e) => { warn!(...); None } }`"

key-files:
  created:
    - crates/nono-cli/tests/resl_nix_async_signal_safety.rs
  modified:
    - crates/nono-cli/src/exec_strategy.rs

key-decisions:
  - "CR-01 trade-off accepted (T-25-03-03): static error messages drop the dynamic errno detail (e.g. the inner error display) in exchange for async-signal-safety. The reduced diagnostic resolution is acceptable; the alternative — heap allocation in async-signal-unsafe context — risks allocator-mutex deadlock under fork."
  - "CR-02 dual-emission: warn! for the structured tracing pipeline AND eprintln! for unconditional stderr. Fail-secure principle requires the user be told even when no log filter is configured."
  - "WR-04 watchdog skip on getpgid failure: returning None (no watchdog) is preferable to firing kill against a possibly-reused PID. The trade-off is documented inline; the timeout will not fire, but a wrong-target kill cannot occur."
  - "Static-analysis regression test rather than runtime dynamic test: a runtime test of fork-while-allocator-locked is platform-specific, inherently flaky, and provides no signal during code review. The static check fires immediately and runs on every host (including the Windows worktree where this plan was implemented)."
  - "TDD RED gate first: a single test commit (115b548d) introduces all five regression assertions. Each subsequent task GREEN commit turns one (or more) assertion green; the full suite is green after task 4."

requirements-completed: [REQ-RESL-NIX-01, REQ-RESL-NIX-02, REQ-RESL-NIX-03]
addresses: [CR-01, CR-02, WR-02, WR-04]

metrics:
  duration_minutes: 17
  tasks_total: 4
  commits: 5
  files_created: 1
  files_modified: 1
  completed_utc: 2026-05-10T20:19:33Z
---

# Phase 25 Plan 03: RESL-NIX-FIXES Summary

Async-signal-safe error reporting + fail-secure UX hardening in `crates/nono-cli/src/exec_strategy.rs` — closing four code-review gaps (CR-01, CR-02, WR-02, WR-04) selected from `25-REVIEW.md`.

## What Shipped

| Finding | Disposition | Implementation |
|---------|-------------|----------------|
| **CR-01** post-fork `format!()` allocator-deadlock risk | Mitigated | 9 `format!()` invocations in the `Ok(ForkResult::Child)` arm replaced with named `const MSG_*: &[u8]` static byte strings; each writes via `libc::write(STDERR_FILENO, ...)` + `_exit(126)` (or no-exit for the one non-fatal seccomp-fail warning). |
| **CR-02** silent `--timeout` non-enforcement in Direct mode | Mitigated | `execute_direct` now emits both `warn!(...)` (structured) and `eprintln!(...)` (unconditional stderr) when `resource_limits.timeout.is_some()`, naming the limitation and recommending `--strategy supervised`. Block is `#[cfg(any(target_os = "linux", target_os = "macos"))]`-guarded so Windows builds are unaffected. |
| **WR-02** macOS `let _ = setrlimit(...)` silent enforcement loss | Mitigated | Both `let _ = setrlimit(...)` discards in the macOS supervised child branch converted to fail-closed: `if setrlimit(...).is_err() { write(MSG_RLIMIT_*_FAIL); _exit(126); }`. Pattern matches the Linux cgroup placement failure handler. |
| **WR-04** macOS watchdog `getpgid(...).unwrap_or(child)` PID-reuse kill risk | Mitigated | `unwrap_or(child)` replaced with `match getpgid(Some(child)) { Ok(pgrp) => Some(spawn_watchdog), Err(e) => { warn!("…no PID fallback…"); None } }`. The watchdog is skipped on Err — no kill-to-wrong-pgrp possible. |

## Commits

| Hash | Type | Subject |
|------|------|---------|
| `115b548d` | test | add static-analysis regressions for CR-01/CR-02/WR-02/WR-04 (TDD RED gate) |
| `45ef4f3f` | fix  | replace format!() with const &[u8] in post-fork child branch (CR-01) |
| `a069d6b3` | feat | warn loudly when --timeout is set in Direct strategy mode (CR-02) |
| `28df5c50` | fix  | fail-closed setrlimit in macOS supervised child branch (WR-02) |
| `abeda8e7` | fix  | replace getpgid PID fallback with safe match in macOS watchdog (WR-04) |

## Verification

All success-criteria checks from the plan pass:

```text
=== CR-01: format!() in lexical child branch (after stripping comments) ===
0      (test cr_01_no_format_macro_in_post_fork_child_branch ... ok)

=== const MSG_*: &[u8] declarations in exec_strategy.rs ===
11     (9 from CR-01 + 2 from WR-02; matches `cr_01_and_wr_02_const_msg_byte_strings_present`)

=== CR-02: warn!(...) invocation containing 'timeout'+'not enforced' ===
present (test cr_02_direct_mode_timeout_emits_warn_macro ... ok)
=== CR-02: eprintln!(...) containing '--strategy supervised' ===
present (same test)

=== WR-02: silent `let _ = setrlimit(...)` discards ===
0      (test wr_02_no_silent_setrlimit_discards ... ok)

=== WR-04: `unwrap_or(child)` PID fallback ===
0      (test wr_04_no_pid_fallback_on_getpgid_failure ... ok)
=== WR-04: `match getpgid(...)` arms ===
1      (same test)

cargo build --workspace                                                       ✅
cargo clippy --workspace -- -D warnings -D clippy::unwrap_used                ✅
cargo clippy --package nono-cli --tests -- -D warnings -D clippy::unwrap_used ✅
cargo fmt --check --all                                                       ✅
cargo test --package nono-cli --bin nono                                      ✅ (856 passed)
cargo test --package nono-cli --test resl_nix_async_signal_safety             ✅ (5 passed)
```

## Tasks Executed

### Task 1 (TDD: RED → GREEN) — CR-01: replace format!() in post-fork child branch

**RED commit** `115b548d` introduces `crates/nono-cli/tests/resl_nix_async_signal_safety.rs` with five source-text assertions covering all four findings. Verified failing on the pre-fix tree:

```text
running 5 tests
test wr_04_no_pid_fallback_on_getpgid_failure ... FAILED
test cr_01_no_format_macro_in_post_fork_child_branch ... FAILED
test wr_02_no_silent_setrlimit_discards ... FAILED
test cr_01_and_wr_02_const_msg_byte_strings_present ... FAILED
test cr_02_direct_mode_timeout_emits_warn_macro ... FAILED
```

**GREEN commit** `45ef4f3f` replaces all 9 child-branch `format!()` calls with named static byte strings:

| Site | Const | Source line range (post-fix) |
|------|-------|------------------------------|
| cgroup placement failure | `MSG_CGROUP` | ~862 |
| clear_close_on_exec on supervisor sock | `MSG_SOCK` | ~899 |
| Sandbox::apply (Linux #[cfg]) | `MSG_SANDBOX_LINUX` | ~933 |
| Sandbox::apply (non-Linux #[cfg]) | `MSG_SANDBOX_OTHER` | ~951 |
| send seccomp notify fd | `MSG_SECCOMP_SEND` | ~994 |
| seccomp-notify not available (non-fatal) | `MSG_SECCOMP_FAIL` | ~1011 |
| send proxy seccomp notify fd | `MSG_PROXY_SEND` | ~1054 |
| proxy seccomp filter unavailable | `MSG_PROXY_FAIL` | ~1071 |
| PR_SET_DUMPABLE(0) failure | `MSG_DUMPABLE` | ~1093 |

Each error binding renamed `_e` (the dynamic detail is dropped — see decision in trade-off table above). Each `unsafe` block has its own `// SAFETY:` comment noting `write`/`_exit` are async-signal-safe.

After this commit: `cr_01_no_format_macro_in_post_fork_child_branch` GREEN; the other 4 assertions remain RED (their findings ship in tasks 2/3/4).

### Task 2 — CR-02: --timeout warning in Direct mode

`a069d6b3`. Inserts a 16-line block immediately after `info!("Executing (direct): ...")` and before the `Command::new` setup:

```rust
#[cfg(any(target_os = "linux", target_os = "macos"))]
if resource_limits.timeout.is_some() {
    warn!("--timeout is not enforced in Direct strategy mode; \
           use --strategy supervised for wall-clock deadline enforcement");
    eprintln!("nono: warning: --timeout is not enforced in Direct strategy mode; \
               use --strategy supervised");
}
```

Dual emission per fail-secure UX principle (warn! for tracing pipeline; eprintln! reaches stderr unconditionally). After commit: `cr_02_direct_mode_timeout_emits_warn_macro` GREEN.

### Task 3 — WR-02: fail-closed setrlimit on macOS

`28df5c50`. Replaces both `let _ = setrlimit(...)` discards in the `#[cfg(target_os = "macos")]` block of the supervised child branch with fail-closed handlers:

- `MSG_RLIMIT_AS_FAIL`     — RLIMIT_AS failure -> `_exit(126)`
- `MSG_RLIMIT_NPROC_FAIL`  — RLIMIT_NPROC failure -> `_exit(126)`

Both consts inside the macOS `cfg` block; both `unsafe` blocks SAFETY-commented. After commit: `wr_02_no_silent_setrlimit_discards` GREEN; `cr_01_and_wr_02_const_msg_byte_strings_present` GREEN (now sees 11 consts).

### Task 4 — WR-04: safe getpgid match in macOS watchdog spawn

`abeda8e7`. Replaces `let child_pgrp = getpgid(Some(child)).unwrap_or(child);` with:

```rust
match getpgid(Some(child)) {
    Ok(child_pgrp) => {
        let fired = timeout_fired.clone();
        Some(supervisor_macos::spawn_macos_timeout_watchdog(
            deadline, child_pgrp, fired,
        ))
    }
    Err(e) => {
        warn!(
            "getpgid({}) failed ({}); skipping timeout watchdog — \
             no PID fallback to avoid wrong-pgrp kill under PID reuse",
            child.as_raw(), e,
        );
        None
    }
}
```

`.flatten()` on the surrounding `.map(|deadline| { ... })` collapses `None` into the `_timeout_watchdog` binding being `None`; the timeout simply does not fire. After commit: `wr_04_no_pid_fallback_on_getpgid_failure` GREEN; full 5-test regression suite GREEN.

## Deviations from Plan

**None.** All four tasks executed as specified; no Rule 1/2/3 auto-fixes triggered, no Rule 4 architectural questions surfaced. The plan's task actions were applied verbatim with the only adjustment being the conventional renaming of error bindings to `_e` once their dynamic display value was dropped (silences `unused_variables` without needing `#[allow]`).

The TDD task in the plan (Task 1) was executed strictly RED-then-GREEN: the test commit lands first, fails as expected against the pre-fix source, and only then is the implementation applied. The remaining three tasks (CR-02, WR-02, WR-04) reuse the same regression file's other assertions as their built-in done-criteria.

## Authentication Gates

None. The plan is purely Rust source modification; no host setup, credentials, or external services required.

## Threat Surface Scan

No new attack surface introduced. The plan strictly removes attack surface (eliminates async-signal-unsafe heap allocation in post-fork child; eliminates silent enforcement-loss paths; eliminates a wrong-target kill primitive). The threat register entries T-25-03-01 through T-25-03-05 are all addressed:

| Threat | Status |
|--------|--------|
| T-25-03-01 (allocator-mutex deadlock under fork via format!) | mitigated |
| T-25-03-02 (silent --timeout non-enforcement → user EoP via false safety assumption) | mitigated |
| T-25-03-03 (reduced diagnostic detail) | accepted, documented |
| T-25-03-04 (silent setrlimit failure → enforcement loss) | mitigated |
| T-25-03-05 (PID-reuse SIGKILL to wrong process group) | mitigated |

## Known Stubs

None.

## Self-Check: PASSED

**Files claimed created/modified — verified via git log + filesystem:**

```text
crates/nono-cli/tests/resl_nix_async_signal_safety.rs   FOUND (created in 115b548d)
crates/nono-cli/src/exec_strategy.rs                    FOUND (modified in 45ef4f3f, a069d6b3, 28df5c50, abeda8e7)
.planning/phases/25-cross-platform-resl-aipc-unix-design/25-03-RESL-NIX-FIXES-SUMMARY.md  WRITTEN (this file)
```

**Commit hashes — verified via `git log --oneline`:**

```text
115b548d  test(25-03): add static-analysis regressions for CR-01/CR-02/WR-02/WR-04                    FOUND
45ef4f3f  fix(25-03): replace format!() with const &[u8] in post-fork child branch (CR-01)            FOUND
a069d6b3  feat(25-03): warn loudly when --timeout is set in Direct strategy mode (CR-02)              FOUND
28df5c50  fix(25-03): fail-closed setrlimit in macOS supervised child branch (WR-02)                  FOUND
abeda8e7  fix(25-03): replace getpgid PID fallback with safe match in macOS watchdog (WR-04)          FOUND
```

**Build/lint/test gates — re-run as final verification:**

```text
cargo build --workspace                                                       PASS
cargo clippy --workspace -- -D warnings -D clippy::unwrap_used                PASS
cargo fmt --check --all                                                       PASS
cargo test --package nono-cli --bin nono                                      PASS (856 passed, 0 failed)
cargo test --package nono-cli --test resl_nix_async_signal_safety             PASS (5 passed, 0 failed)
```

## TDD Gate Compliance

The plan declared Task 1 as `tdd="true"`. Required gate sequence verified in git log:

| Gate | Commit | Status |
|------|--------|--------|
| RED   | `115b548d` test(25-03): add static-analysis regressions… | present |
| GREEN | `45ef4f3f` fix(25-03): replace format!() with const &[u8]… | present after RED |
| REFACTOR | n/a | optional, none required (the GREEN code is already in target shape) |

The remaining three tasks (CR-02, WR-02, WR-04) are not separately TDD-gated in the plan; they reuse assertions already introduced in the RED commit, satisfying the "test before code" principle implicitly. Each task's GREEN commit turns one (or more) of the originally-failing assertions green.
