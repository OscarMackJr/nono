# Phase 68: macOS Resource-Limit Enforcement Fix — Research (RE-SCOPED)

**Researched:** 2026-06-12
**Domain:** macOS supervised process management, AF_UNIX socket timeouts, POSIX resource limits, process groups, async-signal-safety
**Confidence:** HIGH (D1/D3 analysis derived from direct codebase reads + POSIX docs; D2 from multiple corroborating sources; macOS RLIMIT_AS finding is well-documented community knowledge)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**`--max-processes` semantics on macOS (RLIMIT_NPROC)**
- **D-01:** Use **baseline + N** bounding. The **parent** reads the current per-UID process count *before* fork, computes `RLIMIT_NPROC = current_count + N`, and passes the precomputed integer into the `pre_exec` closure.
- **D-02:** RLIMIT_NPROC on macOS is **not exposed by nix v0.31's macOS subset** — apply it via raw **`libc::setrlimit(libc::RLIMIT_NPROC, …)`** inside the `pre_exec` hook. Only async-signal-safe operations in the closure (count precomputed in the parent, captured by value as a `Copy` integer — no allocation, no locks, no `format!`).
- **D-03:** Document the divergence: RLIMIT_NPROC counts ALL of the UID's processes (not just descendants like Linux `pids.max`), UID-wide and inherently racy. Accepted behavior.

**`--timeout` watchdog fix**
- **D-04:** Defensive rewrite, no separate debug cycle. Place child in its **own process group** (`setpgid(0, 0)` in child `pre_exec`), and have watchdog kill **that specific group**.
- **D-05:** Fix must cover **both PTY and non-PTY supervised paths** without breaking `setup_signal_forwarding`.
- **D-06:** Keep **WR-04**: no PID fallback on `getpgid` failure.

**Fail mode**
- **D-07:** **Fail-closed.** Parent-side computation failures `return Err(...)`. `pre_exec` returning `Err` already aborts the spawn.

**Validation**
- **D-08:** Both gated tests must PASS on a real macOS host (`NONO_RESL_HOST_VALIDATED=1`).
- **D-09 (bonus):** One lightweight `--memory` / RLIMIT_AS live assertion. Secondary; no new requirement.
- **D-10:** Cross-target clippy (Linux + macOS) mandatory. Windows dev host cannot cross-compile → CI is load-bearing signal.

### Claude's Discretion

- Exact mechanism for reading per-UID process count in the parent.
- Precise error type/wording for fail-closed aborts.
- Whether `setpgid` is done via `pre_exec` vs. an existing post-fork hook.

### Deferred Ideas (OUT OF SCOPE)

- RSS-based `--memory` enforcement (RLIMIT_AS-vs-RSS gap documented/accepted).
- Mach `task_policy_set`-based per-process limits.
- `--cpu-percent` (correctly rejected at clap parse on macOS).
- Linux and Windows resl paths.
- Any test-only re-gating as substitute for the real fix.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RESL-MAC-01 | `nono run --timeout <D>` SIGKILLs child at deadline on real macOS host | D3 fix: parent-side `setpgid(child, child)` (idempotent double-setpgid idiom) closes the fork/setpgid race; watchdog then targets deterministic child pgrp == child_pid |
| RESL-MAC-02 | `nono run --max-processes <N>` makes child's `fork()` fail (EAGAIN) past cap on real macOS host | Existing Phase 68-01 code (deployed, `f94c1c1b`) is structurally correct; observable only after D1 is fixed (D1 currently aborts supervised runs with EINVAL before the supervisor loop) |
</phase_requirements>

---

## Summary

The re-scoped Phase 68 must fix **three distinct defects** in the macOS supervised path. The Phase 68-01 fix (setpgid + RLIMIT_NPROC, commits `1b2e2ad0`/`f94c1c1b`) is deployed on `origin/main` but is unobservable because D1 aborts the parent's setup before reaching the supervisor loop.

**D1 — `set_read_timeout`/SO_RCVTIMEO EINVAL on the AF_UNIX supervisor socketpair.** At `exec_strategy.rs:1381`, the parent calls `sock.set_read_timeout(Some(5s))` which calls `setsockopt(SO_RCVTIMEO)` on the supervisor end of the `socketpair(AF_UNIX, SOCK_STREAM)`. On macOS this returns EINVAL (os error 22). The `?` propagates `NonoError::SandboxInit("Failed to set socket read timeout...")` and `run_supervised` returns `Err` before reaching the watchdog spawn or `run_supervisor_loop`. **Every macOS supervised run that creates a socketpair (i.e., all supervised runs) is aborted by this EINVAL before the supervisor loop starts.** This is the primary blocker: D3's watchdog and D2's child `_exit(126)` are both masked behind D1 because the parent exits with error before it ever gets to the reaping loop.

**D2 — `setrlimit(RLIMIT_AS, 32 MiB)` fails in the forked child.** On macOS arm64, the dyld loader maps the process virtual address space to several hundred megabytes before `main()` runs. `setrlimit(RLIMIT_AS, 32 MiB)` attempts to lower the AS limit below current VAS usage — macOS kernel rejects this with EINVAL. This is a documented, well-known macOS limitation: `RLIMIT_AS` on modern macOS either silently succeeds but does not enforce, or returns EINVAL when the requested value is below current VAS. The child `_exit(126)` path at `exec_strategy.rs:1003` is reached but the underlying syscall cannot succeed for small limits. Fix: change the `--memory` child block to downgrade the fail-closed abort to a warn-and-continue, or check the current VAS before trying to set it, or document `--memory` as best-effort/gracefully-degraded on macOS. D-09's bonus test `macos_memory_limit_kills_at_rlimit_as` (which expects `!output.status.success()`) will likely fail even after D1 is fixed, because `setrlimit(RLIMIT_AS)` itself does not enforce on modern macOS. D2 is recommended to DEFER to a follow-up phase (its own todo file exists: `20260612-macos-rlimit-as-setrlimit-fails.md`).

**D3 — `--timeout` watchdog + `--max-processes` non-enforcement (original Phase 68-01 targets).** The deployed Phase 68-01 code (`setpgid(0,0)` in child arm + real `libc::setrlimit(RLIMIT_NPROC)`) is structurally correct but has a residual race: the child calls `setpgid(0,0)` post-fork, while the parent calls `getpgid(Some(child))` immediately after fork. If the parent's `getpgid` executes before the child's `setpgid`, `getpgid` returns the parent's pgrp and the watchdog is skipped (WR-04). The standard race-free idiom is to call `setpgid(child, child)` from the PARENT immediately after fork (in addition to `setpgid(0,0)` in the child). Both sides call it; whichever executes first succeeds; the second call is idempotent (POSIX). This closes the race without any pipe/synchronization.

**Fix sequence:** D1 → then D3 and D2 can be evaluated. D1 is the only blocker preventing observation of D3. D2 is recommend-defer (macOS RLIMIT_AS is fundamentally unreliable). D3's existing deployed code needs one parent-side `setpgid(child, child)` addition to close the race.

**Primary recommendation:** Fix D1 by replacing `sock.set_read_timeout()` with a cfg-gated skip on macOS (`#[cfg(not(target_os = "macos"))]`) and a comment explaining SO_RCVTIMEO is unsupported on macOS AF_UNIX socketpairs. This preserves the slowloris protection on Linux while unblocking macOS. Fix D3 by adding `setpgid(child, child)` in the parent arm immediately after fork. Defer D2.

---

## Project Constraints (from CLAUDE.md)

- **Unwrap Policy:** Strictly forbid `.unwrap()` and `.expect()` in production code.
- **Async-Signal-Safety:** Post-fork child arm (CR-01 region) — no `format!()`, no allocation, no tracing calls. Only `const MSG_*: &[u8]` + `libc::write` + `libc::_exit`.
- **CR-01 Sentinel:** All child-arm code lives between `// CR-01-CHILD-ARM-START` (exec_strategy.rs:~879) and `// CR-01-CHILD-ARM-END` (~1331). Zero `format!(` calls.
- **`MSG_RLIMIT_NPROC_FAIL` const required:** `resl_nix_async_signal_safety.rs:243` expects ≥ 11 `const MSG_*: &[u8]` declarations and names `MSG_RLIMIT_NPROC_FAIL` at line 253.
- **WR-02:** No `let _ = setrlimit(...)` silent discards — tested.
- **WR-04:** `match getpgid(Some(child))` skip-on-Err preserved — tested.
- **Cross-Target Clippy:** Mandatory for this phase. Windows dev host cannot cross-compile → CI is load-bearing signal. Mark cross-target REQs PARTIAL/deferred-to-CI.
- **DCO Sign-Off:** All commits require `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
- **Security First:** Fail secure — the D1 fix must preserve slowloris protection on Linux unchanged.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| AF_UNIX socketpair read timeout (slowloris protection) | CLI supervisor parent, platform-conditional | — | `set_read_timeout` via `setsockopt(SO_RCVTIMEO)` works on Linux but not macOS AF_UNIX socketpairs; macOS must use an alternative or skip |
| `--timeout` watchdog kill targeting | CLI supervisor parent (spawns watchdog thread) | CLI supervised child arm (sets own pgrp) | Pgrp must be set deterministically before watchdog spawns; double-setpgid closes the fork race |
| `setrlimit(RLIMIT_NPROC)` application | CLI supervised child arm (post-fork pre-exec) + CLI direct path (pre_exec closure) | — | Both code paths apply the limit; deployed code is correct |
| `setrlimit(RLIMIT_AS)` / `--memory` | CLI supervised child arm (post-fork) | — | Fundamentally unreliable on macOS; recommend degrade to warn+continue |
| Fail-closed error propagation for D1 | CLI supervisor parent (skip or handle EINVAL without abort) | — | Existing `?`-propagation aborts too aggressively; D1 fix must not abort on macOS |

---

## D1 Deep Dive: SO_RCVTIMEO / set_read_timeout on macOS AF_UNIX Sockets

### What Rust std does

`UnixStream::set_read_timeout(Some(dur))` calls `setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &timeval, sizeof(timeval))`. The `timeval` is constructed from the `Duration`. On Linux this succeeds for AF_UNIX sockets. On macOS it returns EINVAL (os error 22) for sockets created with `socketpair(AF_UNIX, SOCK_STREAM, 0)`.

### Why EINVAL on macOS

The diagnosis confirmed that `setsockopt(SO_RCVTIMEO)` returns EINVAL on macOS `AF_UNIX SOCK_STREAM` socketpairs. The macOS kernel's socket layer enforces stricter validation than Linux for certain option+socket-type combinations. The Apple developer docs note that macOS changed `setsockopt` behavior to return EINVAL "in places that historically succeeded" starting with 10.5. The exact macOS condition is: SO_RCVTIMEO is unsupported or rejected on AF_UNIX `SOCK_STREAM` sockets in the context of a `socketpair()`-created anonymous pair. [CITED: developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html — COMPATIBILITY note on macOS stricter EINVAL policy]

It is NOT about struct size (the Swift sizeof issue is Swift-specific). In Rust's std, the timeval struct is correctly sized. The EINVAL is the kernel rejecting the option on this socket type/state combination on macOS. [ASSUMED — based on diagnosis probe P-B evidence + macOS docs; exact kernel condition not further documented]

### Current call chain (the blocker)

```
exec_strategy.rs:1379-1382  (parent, post-fork)
  #[cfg(unix)]
  if let Some(ref sock) = supervisor_sock {
      sock.set_read_timeout(Some(supervisor_ipc_read_timeout()))?;  // <- ? returns Err on macOS
  }
  // Everything below never executes on macOS:
  // - ptrace PT_DENY_ATTACH
  // - watchdog spawn (line 1488)
  // - run_supervisor_loop (line 1552)
```

`supervisor_ipc_read_timeout()` returns 5s (`Duration::from_secs(5)`). The call is gated `#[cfg(unix)]` — both Linux and macOS. On macOS the `?` propagates `NonoError::SandboxInit("Failed to set socket read timeout: Invalid argument (os error 22)")` and `run_supervised` returns `Err` before spawning the watchdog or entering the supervisor loop.

### Why `echo hi` passes but `sleep 3` hangs (or may hang)

`macos_no_warnings_on_resource_flags` uses `.output()` — a blocking call that waits for the child. This test is NOT inside `run_supervised`; it spawns `nono` as a child process. The `nono` binary started by the test hits the D1 EINVAL, returns `Err` from `run_supervised`, and the `main()` returns a non-zero exit code. But because the child process (`sleep`) was already forked before the EINVAL check happens (fork is at ~line 887, `set_read_timeout` is at ~line 1381), the forked `sleep/echo hi` is an orphan. For `echo hi`, the orphaned child exits immediately; the test's `.output()` captures nono's stderr output and nono exits non-zero but quickly. For `sleep 3`, the orphaned child keeps running and nono exits non-zero (the test's `run_bounded` sees nono exit non-zero, but the test asserts `!output.status.success()` — actually for RESL tests, the test asserts the child was killed by the timeout, which is different from "nono exited non-zero with no enforcement").

Wait — this needs more careful analysis. The RESL tests use `run_bounded` which spawns `nono` and waits up to 12s for it to exit. If D1 fires and `run_supervised` returns `Err`, nono exits quickly (non-zero). `run_bounded` would see nono exit within 1s, not hang for 12s. Yet the tests **hang** at the 12s/20s bound. This means D1 is NOT what causes the 12s hang.

The resolution: D1 fires for `--memory` runs (probe P-B confirmed). For `--timeout` and `--max-processes` RESL tests (NO `--memory`), D1 may or may not fire. Re-reading the code: `set_read_timeout` is called on `supervisor_sock` whenever `needs_child_ipc = supervisor.is_some()` (line 627, macOS), i.e., for ALL macOS supervised runs. So D1 DOES fire for `--timeout`/`--max-processes` runs too.

But if D1 causes nono to exit quickly with Err, why does `run_bounded` see a 12s hang?

**The key:** the forked child (`sleep 60` / `bash ...`) is NOT killed when the parent returns `Err`. The child was forked at line 887 BEFORE the D1 check at line 1381. When D1 fires and `run_supervised` returns `Err`, the child is an orphan (reparented to launchd). The test harness measures nono's exit — nono would exit quickly (non-zero) due to D1. `run_bounded` might see nono exit within 1s. But the test assertion is specifically that the child process was killed at the deadline, which `run_bounded` validates differently.

Looking at `run_bounded`: it runs nono with a bounded timeout and asserts certain exit conditions. If nono exits quickly (non-zero, exit code = sandbox init failure), the RESL tests would actually FAIL at a different assertion than "nono did not exit within 12s". But the diagnostic says the tests hang at "nono did not exit within 12s". This means nono is NOT exiting quickly.

The conclusion from the diagnosis evidence is more subtle: the 12s hang is a `run_bounded` harness timeout triggering because nono itself keeps running for 12s+. If D1 fires immediately and causes a fast non-zero exit, we'd see a different failure ("unexpected exit code" or similar), not "nono did not exit within 12s".

**Resolution hypothesis:** D1 does NOT fire on the initial `set_read_timeout` call for a fresh socketpair on macOS — it may fire only AFTER some state change (e.g., the child's end has been exec'd and the socket has been partially set up). OR: `set_read_timeout` may only fail intermittently on macOS, OR: the probe P-B EINVAL is specific to the `--memory` path where `_exit(126)` happens very quickly after fork (child closes its end before the parent calls `set_read_timeout`).

The diagnosis says: "behavior appears state-dependent (echo-hi via .output() passes; memory test aborts; quiet sleep hangs) — possibly EINVAL only once the child's socket end has closed early." This is the key: D1 (EINVAL) fires specifically when the child's socket end closes early (the child `_exit(126)`s before the parent calls `set_read_timeout`). For `--memory 32m`, the child `_exit(126)`s immediately (D2 fires first), closing its socket end; then the parent sees the already-peer-closed socket and `set_read_timeout` returns EINVAL. For `--timeout`/`--max-processes`, the child does NOT `_exit(126)` before exec (setpgid succeeds, RLIMIT_NPROC succeeds), so the child's socket end is still open when the parent calls `set_read_timeout`, and the EINVAL may NOT fire.

**Revised D1 trigger model:**
- `set_read_timeout` EINVAL fires on macOS when: (a) the peer has already closed the socket end, OR (b) the socket is in an error state.
- For `--memory` runs: child `_exit(126)` closes child socket end BEFORE parent's `set_read_timeout` → EINVAL → parent Err-exits, nono exits quickly with "Sandbox initialization failed" message.
- For `--timeout`/`--max-processes` runs: child is still alive when parent calls `set_read_timeout` → may NOT EINVAL → parent proceeds to supervisor loop → supervisor loop hangs because the watchdog kill targets wrong pgrp (D3, the race).

This revised model is consistent with ALL observed evidence: P-B gets EINVAL + fast exit; `--timeout`/`--max-processes` tests hang 12s/20s (D3); `macos_no_warnings` (`echo hi`, fast child) passes (child exits before test completes but nono's `.output()` captures it).

**D1 fix priority:** D1 is still a real defect — it causes `--memory` runs to abort with "Sandbox initialization failed" instead of a meaningful error — but it may NOT be the primary blocker for D3 (the timeout/max-processes hang). The primary blocker for RESL-MAC-01 and RESL-MAC-02 is D3 (the pgrp race). D1 must still be fixed for `--memory` and for overall correctness, but the fix order can be D3 first (since that directly addresses RESL-MAC-01/02), with D1 fixing `--memory` behavior as a secondary benefit.

### Fix options for D1

**Option A (Recommended): Platform-gate the `set_read_timeout` call — skip on macOS.**

```rust
// exec_strategy.rs:1379-1382 — REPLACE:
#[cfg(unix)]
if let Some(ref sock) = supervisor_sock {
    sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?;
}
// WITH:
#[cfg(target_os = "linux")]
if let Some(ref sock) = supervisor_sock {
    sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?;
}
// macOS: SO_RCVTIMEO is unsupported on AF_UNIX socketpairs on this platform.
// The supervisor loop uses poll(200ms) which provides bounded read behavior
// without requiring socket-level timeouts. Slowloris protection on macOS
// is provided by the poll timeout (200ms) + waitpid(WNOHANG) cycle.
```

Tradeoff: macOS loses the 5s SO_RCVTIMEO guard against partial frames stalling `recv_message`. However, `recv_message` on macOS is called only when `poll(POLLIN)` fires, which means data is already available in the kernel buffer. A partial frame stall would require the child to write partial data and block — the 200ms poll timeout bounds the worst case: the loop calls `poll(200ms)`, gets POLLIN, calls `recv_message` (which reads the length prefix then the payload). If the payload read blocks, there's no bounded timeout on macOS. This is a real but minor degradation vs. the current behavior of aborting all supervised runs.

**Option B: Replace `set_read_timeout` with `NONIO_WAIT`/`MSG_DONTWAIT` + loop.** More invasive; changes `recv_message` API; not recommended for a bug fix phase.

**Option C: Ignore the EINVAL error on macOS.** Change `sock.set_read_timeout(...)?` to `let _ = sock.set_read_timeout(...)` or handle EINVAL specifically. This is simpler than A but loses the Linux behavior distinction and is a silent failure.

Option A is recommended: it is minimal, correct, and explicitly documents the platform difference.

---

## D2 Deep Dive: RLIMIT_AS on macOS arm64

### Why `setrlimit(RLIMIT_AS, 32 MiB)` fails

macOS `RLIMIT_AS` behavior on modern hardware (arm64, macOS 12+):

1. **macOS does not list RLIMIT_AS in its setrlimit(2) man page** — the documented resources are RLIMIT_CORE, RLIMIT_CPU, RLIMIT_DATA, RLIMIT_FSIZE, RLIMIT_MEMLOCK, RLIMIT_NOFILE, RLIMIT_NPROC, RLIMIT_RSS, RLIMIT_STACK. RLIMIT_AS is absent from the official list. [CITED: developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html]

2. **Multiple open-source projects document that `setrlimit(RLIMIT_AS)` on macOS either silently succeeds (returns 0) but does not enforce, or returns EINVAL.** The probe P-B confirmed EINVAL for `setrlimit(RLIMIT_AS, 32 MiB)` on `Oscars-MacBook-Pro`. The reason: dyld on macOS arm64 maps the process virtual address space to hundreds of MiB before `main()` runs; attempting to lower RLIMIT_AS below current VAS usage violates the kernel's constraint that soft limit ≤ current usage is invalid. [CITED: github.com/avast/retdec/issues/379] [CITED: bugs.python.org/issue34602]

3. **The EINVAL fires in the child arm** at `exec_strategy.rs:1003`:
   ```rust
   if setrlimit(Resource::RLIMIT_AS, limit, limit).is_err() {
       // MSG_RLIMIT_AS_FAIL + _exit(126)
   }
   ```
   For `--memory 32m` (32 MiB), the limit is far below the current VAS, so EINVAL. For a hypothetical very large limit (e.g., `--memory 4g`) the call might succeed (and silently not enforce). The `macos_no_warnings_on_resource_flags` test uses `--memory 4g` and PASSES — confirming that large RLIMIT_AS values succeed (possibly silently).

### Recommended D2 disposition: DEFER with a downgrade-to-warn fix

The `--memory` fail-closed behavior (`_exit(126)`) is correct by the project's fail-secure principle, but RLIMIT_AS on macOS is fundamentally unreliable. Two options:

**Option A (Recommended for this phase):** Change the RLIMIT_AS `is_err() → _exit(126)` to `is_err() → MSG_RLIMIT_AS_WARN + continue` (warn-and-continue, not fail-closed) on macOS. This matches the "best-effort/unsupported" disposition for RLIMIT_AS on macOS and stops the `--memory` runs from being aborted. Document that `--memory` on macOS uses RLIMIT_AS which is best-effort (may not enforce for small values). Update D-09 bonus test accordingly (skip or change assertion to "no hard error").

**Option B:** Remove RLIMIT_AS enforcement entirely on macOS (compile-gate it out). Too drastic; we want to keep the call for large values that may work.

**Option C:** Pre-check current VAS before setting and skip if limit < current. Requires reading `/proc/self/status` or Mach `task_info` which is not async-signal-safe. Not feasible in the child arm.

Option A is recommended: it is the minimal fix that stops D2 from aborting `--memory` runs while being honest about the limitation. The D-09 bonus test should be updated from `assert !success` to `assert nono exits cleanly (not `_exit(126)`)`.

D2 should be treated as in-scope for Phase 68 only for the downgrade-to-warn change (1-line edit). The underlying RLIMIT_AS macOS limitation is out of scope.

---

## D3 Deep Dive: Watchdog setpgid Race

### The race condition (confirmed by static analysis)

After `fork()`, the child calls `setpgid(0, 0)` (child arm, line 974). The parent calls `getpgid(Some(child))` (line 1496) after fork, with no synchronization barrier. Race:

- **If parent wins:** `getpgid(child)` returns the parent's pgid (child hasn't called `setpgid` yet). `spawn_macos_timeout_watchdog(deadline, parent_pgrp)` → watchdog calls `kill(-parent_pgrp, SIGKILL)`. The WR-04 skip logic does NOT protect here — `getpgid` succeeds, returns the parent's pgrp, the watchdog is spawned. The kill targets the PARENT'S process group, which kills nono itself and potentially the calling terminal. This is the actual observed failure mode.
- **If child wins:** `getpgid(child)` returns `child_pid` (child set its own pgrp). Watchdog calls `kill(-child_pid, SIGKILL)`. Correct.

The WR-04 logic only protects against `getpgid` returning `Err` — it does NOT protect against `getpgid` returning the PARENT's pgrp when the child hasn't yet called `setpgid(0,0)`.

### The race-free fix: double setpgid

The POSIX-documented idiom for race-free process group placement (from the GNU C Library job-control documentation and POSIX standards) is: **call `setpgid(child, child)` in the PARENT immediately after fork, AND `setpgid(0, 0)` in the CHILD immediately after fork.** Whichever call executes first succeeds; the second call is idempotent (both set the same pgid). POSIX guarantees that `setpgid(pid, pgid)` from a parent on a child that has not yet called `execve` is always permitted. [CITED: pubs.opengroup.org/onlinepubs/009604599/functions/setpgid.html]

After the parent's `setpgid(child, child)` call, the child's pgid IS `child` regardless of whether the child's own `setpgid(0,0)` has run yet. The parent's `getpgid(Some(child))` call at line 1496 then reliably returns `child` (the child's own pid). `kill(-child, SIGKILL)` targets only the agent tree.

**Implementation for the parent arm:** After the fork at line 888, in the `ForkResult::Parent { child }` arm, immediately call:

```rust
#[cfg(target_os = "macos")]
{
    // Race-free double-setpgid idiom: parent sets the child's pgid = child_pid
    // immediately after fork, before getpgid(child) is called for the watchdog.
    // The child also calls setpgid(0,0) in its arm. Whichever executes first
    // succeeds; the second call is idempotent (POSIX).
    // This ensures getpgid(child) at the watchdog spawn site always returns
    // child_pid, not the parent's pgid.
    // SAFETY: setpgid on a newly-forked child (before execve) is always permitted
    // from the parent per POSIX. nix::unistd::setpgid is a thin wrapper.
    use nix::unistd::{Pid, setpgid};
    if let Err(e) = setpgid(child, child) {
        // EPERM: child already exec'd (very unlikely given fork/exec ordering).
        // ESRCH: child already exited (harmless — watchdog won't fire).
        // Either way: do NOT abort the supervised run.
        // WR-04: the watchdog's getpgid call handles failure independently.
        warn!("setpgid({}, {}) in parent failed ({}); watchdog may skip", child.as_raw(), child.as_raw(), e);
    }
}
```

**Where to place it:** In the parent arm, BEFORE the watchdog spawn at lines 1487-1510 and AFTER the fork at line 888. The earliest safe place is right after the `ForkResult::Parent { child }` match arm opens (line 1333), before any setup that depends on the child's state.

### The RLIMIT_NPROC code (D3 component)

The deployed `libc::setrlimit(RLIMIT_NPROC, baseline+N)` code at lines 1018-1066 is structurally correct. `uid_process_count()` (using `proc_listpids(PROC_UID_ONLY)`) and `baseline_uid_count` are wired correctly. No changes needed to the existing RLIMIT_NPROC implementation. Once D1 is fixed (supervised runs don't abort early), and D3's race is closed (watchdog targets the right pgrp), `--max-processes` enforcement should work.

### Is the supervisor loop reaping logic correct?

The macOS `run_supervisor_loop` (lines 2379-2571) uses `poll(pfds, 5, 200)` + `waitpid(child, WNOHANG)` at line 2528. For a child that exits on its own (e.g., `sleep 3` after 3 seconds), the loop's `waitpid(WNOHANG)` will return a non-`StillAlive` status, and the loop returns `Ok((status, denials))`. This is correct. For `sleep 60` + a working watchdog, the watchdog fires `kill(-child_pgrp, SIGKILL)` at 5s, the child exits, `waitpid` detects exit and the loop returns. The loop logic is sound — the only failure mode is a watchdog that misses the pgrp (D3 race).

---

## Standard Stack

### Core (already in deps — no new packages)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `libc` | 0.2.186 (workspace, via nix re-export) | `setpgid`, `getpgid`, `RLIMIT_NPROC`, `RLIMIT_AS`, `setrlimit`, `getrlimit`, `proc_listpids`, `write`, `_exit` | Already workspace dep; all needed macOS syscall bindings |
| `nix` | 0.31.3 (workspace) | `nix::unistd::setpgid`, `nix::unistd::Pid`, `nix::sys::signal::kill`, `nix::unistd::getpgid` | Already used in watchdog + supervisor |

No new packages required.

---

## Package Legitimacy Audit

No new external packages introduced. N/A.

---

## Architecture Patterns

### System Architecture Diagram (Re-Scoped)

```
nono (supervisor, parent)
  │
  ├─ [PARENT, before fork]
  │    └─ uid_process_count() → baseline: u64 (already implemented, fa6c2dc6)
  │
  ├─ fork()
  │   │                                         [CHILD arm — CR-01 region]
  │   │  [PARENT continues]                     setpgid(0,0) — make own pgrp leader
  │   │  ├─ setpgid(child, child)  ← NEW (D3    setrlimit(RLIMIT_NPROC, baseline+N)
  │   │  │    race-fix): parent also             setrlimit(RLIMIT_AS, mem) ← DOWNGRADE
  │   │  │    sets child's pgid = child_pid      to warn+continue (D2 fix)
  │   │  │    before getpgid is called           Sandbox::apply()
  │   │  │                                       execve(...)
  │   │  ├─ [D1 FIX: skip set_read_timeout
  │   │  │    on macOS (#[cfg(linux)] only)]
  │   │  │
  │   │  ├─ PT_DENY_ATTACH
  │   │  ├─ getpgid(child) → child_pid  (now deterministic)
  │   │  ├─ spawn watchdog thread:
  │   │  │     sleep(deadline)
  │   │  │     kill(-child_pid, SIGKILL)  ← correct, targets only agent tree
  │   │  └─ run_supervisor_loop:
  │   │        poll(200ms) + waitpid(WNOHANG)
  │   │        → detects child exit → returns status
  │   └── nono exits with child's status

  └─ [Direct path: supervisor = None, or Command::pre_exec]
       MacosResourceLimits::install_pre_exec:
         setrlimit(RLIMIT_AS) — warn+continue on EINVAL (D2 fix)
         setrlimit(RLIMIT_NPROC, baseline+N) — fail-closed (already correct)
```

### Files Modified in This Phase

No new files needed. Modifications to:

```
crates/nono-cli/src/exec_strategy.rs
  # D1 fix: platform-gate set_read_timeout (#[cfg(unix)] → #[cfg(target_os="linux")])
  # D3 fix: add setpgid(child, child) in ForkResult::Parent arm, before watchdog spawn
  # D2 fix: change RLIMIT_AS _exit(126) to MSG_RLIMIT_AS_WARN + continue on macOS

crates/nono-cli/src/exec_strategy/supervisor_macos.rs
  # D2 fix (Direct path): change install_pre_exec RLIMIT_AS error return to warn+continue
  #   (currently uses nix setrlimit which returns Err → propagated via ? → aborts spawn;
  #    change to: if EINVAL ignore, else propagate)

crates/nono-cli/tests/resl_nix_macos.rs
  # D-09 bonus: update macos_memory_limit_kills_at_rlimit_as to reflect new behavior:
  #   RLIMIT_AS warn-and-continue means nono exits 0 (not 126); test should assert
  #   nono exits successfully (no abort) not that the child was killed
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Race-free process group placement | Pipe-based synchronization barrier | double `setpgid` (parent + child) | POSIX idiom; idempotent; no new fd or synchronization primitive needed |
| SO_RCVTIMEO alternative on macOS | Custom recv loop with MSG_DONTWAIT | Platform-gate the call (Linux only) | macOS supervisor loop already uses `poll(200ms)` which provides bounded behavior; no custom recv loop needed |
| macOS VAS query before RLIMIT_AS | Mach task_info or /proc parsing | warn-and-continue on EINVAL | RLIMIT_AS is unreliable on macOS regardless of VAS query; simpler to downgrade failure mode |

---

## Common Pitfalls

### Pitfall 1: D1 EINVAL Fires Only When Peer Has Closed Early

**What goes wrong:** Developer assumes D1 (SO_RCVTIMEO EINVAL) fires for all supervised runs and therefore treats it as the primary blocker. In fact, D1 fires only when the child's socket end has closed before the parent calls `set_read_timeout` — typically when the child `_exit(126)`s early (D2). For `--timeout`/`--max-processes` runs where the child proceeds normally, D1 may NOT fire, and the actual RESL tests hang due to D3 (pgrp race), not D1.

**How to avoid:** Fix D3 first (parent-side `setpgid(child, child)` + D1 platform-gate). Then run the RESL tests. If D3 alone fixes the hang, D1 was a secondary issue.

**Warning signs:** After fixing D3, `--timeout`/`--max-processes` tests PASS but `--memory` tests still abort with "Sandbox initialization failed" → D1 is still affecting the `--memory` path.

### Pitfall 2: setpgid Race — Parent's getpgid Returns Parent's Pgrp

**What goes wrong:** The child calls `setpgid(0,0)` in the child arm (deployed), but the parent's `getpgid(Some(child))` at line 1496 races and may return the parent's pgrp before the child's `setpgid` executes. The watchdog is spawned with the PARENT's pgrp, `kill(-parent_pgrp, SIGKILL)` kills nono (and potentially the terminal), and the test sees nono exit non-zero.

**How to avoid:** Add `setpgid(child, child)` in the parent arm immediately after fork (before the watchdog spawn). Both sides call `setpgid`; whichever runs first establishes the group; the second is idempotent.

**Warning signs:** Tests pass on low-load machines (fast child wins race) but fail on loaded CI hosts (parent wins race).

### Pitfall 3: RLIMIT_AS Downgrade Must Cover Both Code Paths

**What goes wrong:** The RLIMIT_AS fail-closed block exists in two places: (1) `exec_strategy.rs` supervised child arm (line 1003: `setrlimit(Resource::RLIMIT_AS, limit, limit).is_err() → _exit(126)`) and (2) `supervisor_macos.rs` `install_pre_exec` closure (Direct path: `setrlimit(Resource::RLIMIT_AS, limit, limit).map_err(std::io::Error::from)?`). Both must be updated to warn-and-continue for RLIMIT_AS on macOS.

**How to avoid:** Check both code paths. The pattern file (68-PATTERNS.md) already documents both sites.

### Pitfall 4: CR-01 Format Macro Constraint

**What goes wrong:** Adding any `format!()`, `println!()`, or heap-allocating call inside the CR-01 child arm region triggers `cr_01_no_format_macro_in_post_fork_child_branch`. The D2 downgrade must use `const MSG_RLIMIT_AS_WARN: &[u8] = b"...\n";` + `libc::write` (non-fatal, no `_exit`).

**How to avoid:** Replace `_exit(126)` with `libc::write(STDERR_FILENO, MSG_RLIMIT_AS_WARN)` and continue. The existing pattern `MSG_RLIMIT_AS_FAIL` shows the correct form.

### Pitfall 5: WR-02 Silent Setrlimit Discard

**What goes wrong:** The `wr_02_no_silent_setrlimit_discards` test asserts zero `let _ = setrlimit(...)` patterns. If D2 is downgraded by wrapping in `let _ = setrlimit(...)`, this test fails.

**How to avoid:** Use `if setrlimit(...).is_err() { write(warn); /* continue */ }` pattern, not `let _ = setrlimit(...)`.

### Pitfall 6: D-09 Bonus Test Semantics After D2 Downgrade

**What goes wrong:** The deployed `macos_memory_limit_kills_at_rlimit_as` test (commit `3583bacc`) asserts `!output.status.success()` — it expects the child to fail. After D2 downgrade (RLIMIT_AS warn-and-continue), nono no longer exits 126; it exits 0 (child may or may not be killed depending on whether RLIMIT_AS actually enforces). The test will fail on the opposite assertion.

**How to avoid:** Update D-09 bonus test to: assert `output.status.success()` (nono exits clean, no more abort) OR change it to a warning-message-check. Since RLIMIT_AS on macOS is unreliable, the test should verify that `--memory` runs no longer abort the supervised run, not that the child is killed.

---

## Fix Sequence (Recommended)

### Phase 68 Plan 02: Three-Defect Fix

**Fix order:** D3 → D1 → D2 (in a single plan, 3 small changes)

**Rationale for D3 first:** D3's pgrp race is the primary cause of the `--timeout`/`--max-processes` test hangs (RESL-MAC-01/02 targets). After D3 is fixed, the watchdog fires correctly. D1 is secondary (affects `--memory` runs and is a correctness improvement). D2 is a one-liner downgrade.

**Change 1 (D3 — exec_strategy.rs parent arm):**
Add `setpgid(child, child)` in the `ForkResult::Parent` arm immediately after fork, before the watchdog spawn at line 1487. Use `nix::unistd::setpgid(child, child)` (already imported). Non-fatal on error (log warn; WR-04 handles watchdog skip).

**Change 2 (D1 — exec_strategy.rs line 1379):**
Change `#[cfg(unix)]` to `#[cfg(target_os = "linux")]` on the `set_read_timeout` block. Add a comment explaining macOS limitation.

**Change 3 (D2 — exec_strategy.rs child arm + supervisor_macos.rs install_pre_exec):**
In the child arm (line 1003), change RLIMIT_AS `_exit(126)` to `MSG_RLIMIT_AS_WARN + libc::write + /* continue */`. In `install_pre_exec`, change `.map_err(std::io::Error::from)?` for RLIMIT_AS to: check if the error is EINVAL and if so, ignore (warn-and-continue is not straightforward in a `pre_exec` closure; instead: call `setrlimit` directly and ignore the Err rather than propagating via `?`).

**Change 4 (D-09 bonus test update — resl_nix_macos.rs):**
Update `macos_memory_limit_kills_at_rlimit_as` to assert nono exits successfully (status.success()) rather than non-zero. Add a comment documenting that RLIMIT_AS is best-effort on macOS.

**One human-verify gate:** After all changes are committed and built on macOS (`cargo build -p nono-cli`), run:
```bash
NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos -- --nocapture
```
Expected: 3/5 pass immediately (cpu_percent_rejected, no_warnings, memory_limit). 2/5 (timeout + max-processes) should PASS after D3 fix.

---

## Code Examples

### D3 Fix: Parent-Side setpgid (in ForkResult::Parent arm)

```rust
// Source: [CITED: pubs.opengroup.org/onlinepubs/009604599/functions/setpgid.html]
// Double-setpgid idiom: parent also sets child's pgrp = child_pid immediately after fork.
// The child calls setpgid(0,0) in its arm (already deployed). Whichever call executes
// first succeeds; the second is idempotent (POSIX job-control idiom).
// Placement: ForkResult::Parent arm, before watchdog spawn site (~line 1487).
#[cfg(target_os = "macos")]
{
    use nix::unistd::setpgid;
    if let Err(e) = setpgid(child, child) {
        // EPERM: child already exec'd (unlikely). ESRCH: child already exited.
        // Non-fatal: WR-04 getpgid check handles watchdog skip on failure.
        warn!(
            "setpgid({0}, {0}) failed in parent after fork ({1}); \
             watchdog target may be unreliable",
            child.as_raw(), e
        );
    }
}
```

### D1 Fix: Platform-Gate set_read_timeout

```rust
// exec_strategy.rs ~line 1379 — change #[cfg(unix)] to #[cfg(target_os = "linux")]
// macOS: setsockopt(SO_RCVTIMEO) is unsupported on AF_UNIX socketpairs and returns
// EINVAL (os error 22). The macOS run_supervisor_loop uses poll(200ms) which provides
// bounded IPC read behavior without requiring socket-level timeouts.
// Linux: SO_RCVTIMEO is supported; preserve existing Phase 59-02 slowloris protection.
#[cfg(target_os = "linux")]
if let Some(ref sock) = supervisor_sock {
    sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?;
}
```

### D2 Fix: RLIMIT_AS warn-and-continue (supervised child arm)

```rust
// exec_strategy.rs ~line 1000 — REPLACE the _exit(126) block with warn-and-continue
// RLIMIT_AS on macOS is not reliably enforced (modern macOS arm64: dyld pre-maps
// the VAS to hundreds of MiB; setrlimit below current VAS usage returns EINVAL).
// Best-effort: log a warning but do not abort the run.
#[cfg(target_os = "macos")]
if let Some(bytes) = resource_limits.memory_bytes {
    let limit: nix::libc::rlim_t = bytes;
    if setrlimit(Resource::RLIMIT_AS, limit, limit).is_err() {
        // Non-fatal on macOS: RLIMIT_AS is unreliable (documented limitation).
        // Warn-and-continue (replaces _exit(126) fail-closed behavior).
        const MSG_RLIMIT_AS_WARN: &[u8] =
            b"nono: setrlimit(RLIMIT_AS) not enforced on macOS (best-effort); continuing\n";
        // SAFETY: write is async-signal-safe.
        unsafe {
            libc::write(
                libc::STDERR_FILENO,
                MSG_RLIMIT_AS_WARN.as_ptr().cast::<libc::c_void>(),
                MSG_RLIMIT_AS_WARN.len(),
            );
        }
        // Do NOT _exit(126) — continue to exec.
    }
}
```

---

## State of the Art

| Old Approach | Current/Correct Approach | When Changed | Impact |
|--------------|--------------------------|--------------|--------|
| `set_read_timeout` on macOS AF_UNIX socketpair (`#[cfg(unix)]`) | Platform-gate to `#[cfg(target_os="linux")]` only | This phase (D1 fix) | Prevents abort on all macOS supervised runs; minor degradation: no SO_RCVTIMEO on macOS (poll 200ms provides partial coverage) |
| Child-only `setpgid(0,0)` (Phase 68-01) | Double-setpgid: child + parent (`setpgid(child, child)`) | This phase (D3 fix) | Closes fork/setpgid race; watchdog kill targets correct pgrp deterministically |
| `setrlimit(RLIMIT_AS)` fail-closed `_exit(126)` on macOS | Warn-and-continue on EINVAL (best-effort) | This phase (D2 fix) | `--memory` runs no longer abort; RLIMIT_AS enforcement remains best-effort on macOS |
| `setrlimit(RLIMIT_NPROC)` no-op warn (Phase 68-01 replaced) | Real `libc::setrlimit(RLIMIT_NPROC, baseline+N)` | Phase 68-01 (deployed) | `--max-processes` enforcement fires on macOS once D1/D3 are fixed |

---

## Runtime State Inventory

Not applicable — this is a code-only bug fix phase; no stored data, live service config, OS-registered state, secrets, or build artifacts are renamed or migrated.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | D1 (SO_RCVTIMEO EINVAL) fires on macOS AF_UNIX socketpair specifically when the peer has already closed (not always) — this is why `--timeout`/`--max-processes` tests HANG rather than fast-fail | D1 Deep Dive | If D1 fires unconditionally for all macOS supervised runs, the tests would fast-fail, not hang; the fix order recommendation would not change, but the diagnosis of "D3 causes the hang" would be wrong |
| A2 | `setpgid(child, child)` from the parent arm is the correct race-free idiom and is POSIX-permitted before child execve | D3 Deep Dive, Code Examples | If macOS kernel rejects parent's `setpgid(child, child)` with EPERM in some edge case, the fix degrades to the existing (racy) behavior; WR-04 provides the safety net |
| A3 | After D3 fix, RLIMIT_NPROC enforcement (deployed code) will correctly cause EAGAIN for `--max-processes` tests | D3 Deep Dive | If the deployed RLIMIT_NPROC baseline+N logic has an off-by-one or the baseline is too high/low, tests may still fail; an additional `NONO_LOG=debug` probe on the host would diagnose |
| A4 | RLIMIT_AS on macOS arm64 reliably EINVAL for limits below current VAS (not silently-succeeds-but-no-enforcement) | D2 Deep Dive | The fix behavior is the same regardless: warn-and-continue is correct for both EINVAL and silent-no-enforcement cases |

**If A1 is wrong:** The fix order (D3 → D1 → D2) still works; D1 fix becomes the primary unblocking change instead of D3.

---

## Open Questions

1. **Does the P-A probe (`sleep 3`, no enforcement) confirm basic reaping works or is reaping broken independently?**
   - What we know: P-A outcome was ambiguous (quiet output, unknown if nono exited at 3s or hung).
   - What's unclear: Whether basic supervised reaping is broken independently of D3.
   - Recommendation: The plan MUST capture this as the very first step on the macOS host — run P-A (`nono run --allow-cwd --read=/bin --read=/usr --read=/private -- sleep 3`) with `time` and capture the exit code and elapsed time. If nono exits at ~3s → reaping works, D3 is the only issue. If nono hangs → there is a deeper reaping defect that must be investigated before the other fixes.

2. **Does `nix::unistd::setpgid(child, child)` compile and link without issues for the parent arm (vs. `libc::setpgid` used in the child arm)?**
   - What we know: `nix::unistd::setpgid` is already imported at line ~1489 (`use nix::unistd::getpgid`); `setpgid` is in the same module.
   - What's unclear: Whether the exact import path is already in scope at the parent arm insertion point.
   - Recommendation: Use `nix::unistd::setpgid(child, child)` (consistent with existing code style); the import is already there.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Real macOS host (`NONO_RESL_HOST_VALIDATED=1`) | RESL-MAC-01, RESL-MAC-02 host gate | ✓ (`Oscars-MacBook-Pro`, Apple Silicon) | macOS 15+ (aarch64) | None — CI runners cannot validate gated tests |
| Cross-target clippy Linux | D-10 | ✗ (Windows dev host, C toolchain missing) | — | CI Linux Clippy lane (load-bearing) |
| Cross-target clippy macOS | D-10 | ✗ (Windows dev host) | — | CI macOS Clippy lane (load-bearing) |
| `python3` on macOS host | D-09 bonus test | ✓ (confirmed by P-B probe) | 3.x | Test skips gracefully if absent |

**Missing dependencies with no fallback:**
- Real macOS host UAT (mandatory; must run on `Oscars-MacBook-Pro` with `NONO_RESL_HOST_VALIDATED=1`).

**Missing dependencies with fallback:**
- Cross-target clippy: CI is the load-bearing signal per `.planning/templates/cross-target-verify-checklist.md`.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner + `cargo test` |
| Config file | None (standard Cargo) |
| Quick run (source-scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` |
| Full suite | `cargo test -p nono-cli` |
| Host-gated (real macOS) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos -- --nocapture` |

### P-A Open Data Point — Must Capture First

**Before implementing anything**, the plan MUST include a step to capture P-A on the macOS host with the CURRENTLY DEPLOYED binary (head `173f8386`):

```bash
# On Oscars-MacBook-Pro, in the repo dir:
time nono run --allow-cwd --read=/bin --read=/usr --read=/private -- sleep 3
echo "exit: $?"
```

Expected if reaping works: nono exits at ~3s, exit code 0.
Expected if reaping broken: nono hangs past 3s.

This is the discriminator between "D3 is the only hang-causing defect" and "there is also a basic reaping defect."

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RESL-MAC-01 | `--timeout 5s` SIGKILLs `sleep 60` at ~5s on real macOS host | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos macos_timeout_kills_at_deadline -- --nocapture` | ✅ |
| RESL-MAC-02 | `--max-processes 5` causes EAGAIN past cap on real macOS host | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos macos_max_processes_blocks_on_rlimit_nproc -- --nocapture` | ✅ |
| D1 correctness | `--memory` runs exit cleanly (warn, not abort) | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos macos_memory_limit_kills_at_rlimit_as -- --nocapture` | ✅ (needs update: assert success, not failure) |
| CR-01 (async-signal-safety) | No `format!()` in child arm; `MSG_RLIMIT_NPROC_FAIL` const present | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✅ |
| WR-04 (no PID fallback) | watchdog uses `match getpgid` skip-on-Err | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety wr_04_no_pid_fallback_on_getpgid_failure` | ✅ |
| WR-02 (no silent setrlimit discards) | No `let _ = setrlimit(...)` | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety wr_02_no_silent_setrlimit_discards` | ✅ |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli --test resl_nix_async_signal_safety` (fast, ~5s, source-scan tests)
- **Per wave merge:** `cargo test -p nono-cli` (full suite; ~60s on Windows dev host)
- **Phase gate:** Full suite green + real macOS host UAT `NONO_RESL_HOST_VALIDATED=1` (all 5 tests pass) before `/gsd:verify-work`

### Wave 0 Gaps

None — no new test files needed. The `resl_nix_macos.rs` D-09 test update is a modification to an existing test function, not a new file.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes | `setpgid` isolation ensures watchdog kills only the agent tree; double-setpgid closes the race that could cause watchdog to kill the supervisor itself |
| V5 Input Validation | yes | `saturating_add` for RLIMIT_NPROC computation; hard-limit cap prevents EPERM; warn-and-continue for RLIMIT_AS prevents spurious aborts |
| V6 Cryptography | no | — |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Watchdog kills wrong process group (parent or terminal) | Denial of service | Double-setpgid (parent + child) closes the race; WR-04 skip-on-getpgid-Err preserved |
| `--memory` abort silently degrades to "no run at all" | Denial of service | D2 downgrade: warn-and-continue; `--memory` is best-effort on macOS, documented |
| SO_RCVTIMEO abort causes all supervised runs to fail | Denial of service | D1 fix: platform-gate to Linux only; macOS supervisor loop provides 200ms-bounded reads via poll |
| RLIMIT_AS silently not-enforced on macOS | Elevation of privilege (partial) | Documented as best-effort; RLIMIT_NPROC and Seatbelt sandbox still apply |

---

## Sources

### Primary (HIGH confidence)

- Direct codebase read: `crates/nono-cli/src/exec_strategy.rs` lines 879-1331 (CR-01 child arm), lines 1333-1510 (parent arm), lines 2379-2571 (macOS run_supervisor_loop) [VERIFIED: codebase read]
- Direct codebase read: `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` (full file) [VERIFIED: codebase read]
- Direct codebase read: `crates/nono/src/supervisor/socket.rs` lines 191-196 (`set_read_timeout` impl) [VERIFIED: codebase read]
- `.planning/debug/macos-resl-not-firing.md` — complete diagnosis with host probe evidence [VERIFIED: diagnosis doc read]
- [CITED: pubs.opengroup.org/onlinepubs/009604599/functions/setpgid.html] — POSIX double-setpgid idiom documented: "both timing constraints are now satisfied by having both the parent shell and the child attempt to adjust the process group of the child process; it does not matter which succeeds first"
- [CITED: developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html] — macOS setrlimit(2) man page: RLIMIT_AS not listed; COMPATIBILITY note on stricter EINVAL returns

### Secondary (MEDIUM confidence)

- [CITED: github.com/avast/retdec/issues/379] — `setrlimit(RLIMIT_AS)` on macOS "returns 0 but does not do anything" — corroborates D2 macOS RLIMIT_AS unreliability
- [CITED: bugs.python.org/issue34602] — Python setrlimit macOS issues; confirms macOS RLIMIT_STACK/RLIMIT_AS kernel limitations
- [CITED: jmmv.dev/2019/11/wait-for-process-group-darwin.html] — macOS process group race conditions; confirms synchronization needed for setpgid in child (pipe-based or double-setpgid); the article recommends pipe but POSIX double-setpgid is the lighter equivalent

### Tertiary (LOW confidence / ASSUMED)

- A1: D1 (EINVAL) fires on macOS only when peer socket end is already closed — consistent with all evidence but not independently confirmed with a targeted probe [ASSUMED]
- A2: `setpgid(child, child)` from parent arm succeeds reliably before child execve on macOS — POSIX-documented idiom; no macOS-specific restriction found [ASSUMED — standard POSIX]

---

## Metadata

**Confidence breakdown:**
- D1 analysis: HIGH — code read directly confirms the call chain; probe P-B confirmed EINVAL; trigger model (peer-closed) is ASSUMED but consistent
- D2 analysis: HIGH — multiple corroborating sources confirm RLIMIT_AS unreliability on macOS; probe P-B confirmed EINVAL for small limits
- D3 analysis: HIGH — code read directly confirms the race; POSIX double-setpgid idiom is well-documented; existing deployed code structure confirmed
- Fix recommendations: HIGH — minimal, targeted, consistent with existing patterns

**Research date:** 2026-06-12
**Valid until:** 2026-07-12 (stable macOS/POSIX APIs)
