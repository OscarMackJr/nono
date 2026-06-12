# Phase 68: macOS Resource-Limit Enforcement Fix — Research

**Researched:** 2026-06-12
**Domain:** macOS supervisor process management, POSIX resource limits, process groups, async-signal-safety
**Confidence:** HIGH (all key claims verified against libc 0.2.186 docs.rs and nix 0.31.3 docs.rs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**`--max-processes` semantics on macOS (RLIMIT_NPROC)**
- **D-01:** Use **baseline + N** bounding. The **parent** reads the current per-UID process count *before* fork, computes `RLIMIT_NPROC = current_count + N`, and passes the precomputed integer into the `pre_exec` closure. This matches the Linux `pids.max` intent (the agent gets N *additional* processes).
- **D-02:** RLIMIT_NPROC on macOS is **not exposed by nix v0.31's macOS subset** — apply it via raw **`libc::setrlimit(libc::RLIMIT_NPROC, …)`** inside the `pre_exec` hook. Only async-signal-safe operations in the closure (the count is precomputed in the parent and captured by value as a `Copy` integer — no allocation, no locks, no `format!`). Follow the existing async-signal-safety posture documented for `RLIMIT_AS` in `supervisor_macos.rs`.
- **D-03:** Document the divergence: RLIMIT_NPROC counts ALL of the UID's processes (not just descendants like Linux `pids.max`), so the bound is UID-wide and inherently racy. Accepted behavior.

**`--timeout` watchdog fix**
- **D-04:** Defensive rewrite, no separate debug cycle. Root cause is assumed to be a shared/incorrect process group. Place child in its **own process group** (`setpgid(0, 0)` in child `pre_exec`), and have watchdog kill **that specific group**.
- **D-05:** Fix must cover **both PTY and non-PTY supervised paths** without breaking `setup_signal_forwarding`.
- **D-06:** Keep **WR-04**: no PID fallback on `getpgid` failure. Make the child's own group *exist deterministically* so `getpgid` is reliable.

**Fail mode**
- **D-07:** **Fail-closed.** Parent-side computation failures `return Err(...)`. `pre_exec` returning `Err` already aborts the spawn. Replaces the current silent-warn behavior.

**Validation**
- **D-08:** Both gated tests (`macos_timeout_kills_at_deadline`, `macos_max_processes_blocks_on_rlimit_nproc`) must PASS on a real macOS host (`NONO_RESL_HOST_VALIDATED=1`). Tests stay env-gated off CI.
- **D-09 (bonus):** Add one lightweight `--memory` / RLIMIT_AS live assertion to real-host UAT. Secondary; no new requirement.
- **D-10:** Cross-target clippy (Linux + macOS) mandatory per `.planning/templates/cross-target-verify-checklist.md`. Windows dev host cannot cross-compile → CI is the load-bearing signal.

### Claude's Discretion

- Exact mechanism for reading per-UID process count in the parent (sysctl / proc_listpids / counting) — researcher/planner picks most robust async-safe-friendly approach; runs in parent before fork, yields a single integer.
- Precise error type/wording for fail-closed aborts (use `NonoError` variants consistent with existing resl error surface).
- Whether `setpgid` is done via `pre_exec` vs. an existing post-fork hook — whichever integrates cleanly with current supervised fork flow.

### Deferred Ideas (OUT OF SCOPE)

- RSS-based `--memory` enforcement (RLIMIT_AS-vs-RSS gap is documented/accepted).
- Mach `task_policy_set`-based per-process limits.
- `--cpu-percent` (correctly rejected at clap parse on macOS).
- Linux and Windows resl paths.
- Any test-only re-gating as a substitute for the real fix.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RESL-MAC-01 | `nono run --timeout <D>` SIGKILLs the child at the deadline on a real macOS host — supervisor wall-clock watchdog fires | Research identifies root cause (wrong/shared process group) and exact fix (`setpgid(0,0)` in child pre_exec + watchdog targets child-specific pgrp) |
| RESL-MAC-02 | `nono run --max-processes <N>` makes child's `fork()` fail (EAGAIN) past cap on real macOS host | Research confirms `libc::RLIMIT_NPROC = 7` is defined for both `x86_64-apple-darwin` and `aarch64-apple-darwin` in libc 0.2.186; confirms baseline+N strategy via sysctl `KERN_PROC_UID`; confirms async-signal-safe raw `libc::setrlimit` pattern |

</phase_requirements>

---

## Summary

Phase 68 fixes two distinct supervisor bugs in `crates/nono-cli/src/exec_strategy/` that prevent macOS resource-limit enforcement from firing on real hardware. Both bugs were confirmed on `Oscars-MacBook-Pro` (Apple Silicon, macOS 15+) during Phase 65 gate-65-A UAT.

**Bug 1 — `--max-processes` silent no-op.** In two places: `supervisor_macos.rs::install_pre_exec` (used by the Direct path via `execute_direct`) and the raw-fork child arm in `exec_strategy.rs` (used by `execute_supervised`). Both replace the `tracing::warn!` / libc `write` warning with a real `libc::setrlimit(libc::RLIMIT_NPROC, …)` call. `libc::RLIMIT_NPROC` is confirmed available as value `7` for both `x86_64-apple-darwin` and `aarch64-apple-darwin` in libc 0.2.186. The nix v0.31 absence of `Resource::RLIMIT_NPROC` on macOS is the reason for using raw `libc::` — the fix does not require nix. The baseline per-UID process count is read in the parent (before fork) via `libc::sysctl` with `[CTL_KERN, KERN_PROC, KERN_PROC_UID, uid]` key, which yields a list of `kinfo_proc` structures whose length is the count. `sysctl` is not listed in POSIX async-signal-safe functions, which is why the count must be computed in the parent and captured by `Copy` into the closure/child arm — not inside the `pre_exec` or post-fork arm.

**Bug 2 — `--timeout` watchdog kills the wrong process group.** `execute_supervised` uses `nix::unistd::fork()` (raw fork, not `std::process::Command`). After fork, the child inherits the parent's process group. `getpgid(child)` therefore returns the parent's pgid, and `kill(-pgrp, SIGKILL)` kills the parent's process group — including nono itself — or fails silently if the pgrp is shared with other system processes. Fix: in the child arm of `execute_supervised` (inside the CR-01 child-arm block, which is async-signal-safe), call `libc::setpgid(0, 0)` immediately after fork to make the child its own process-group leader. The watchdog already receives `child_pgrp` from `getpgid(Some(child))` (WR-04 pattern) — after the fix, this pgrp is the child's own dedicated group, and `kill(-child_pgrp, SIGKILL)` correctly targets only the agent tree.

**Primary recommendation:** Fix both bugs in a single plan — they share the same `supervisor_macos.rs` and `exec_strategy.rs` surfaces. Three changes are required: (1) add `libc::setpgid(0,0)` in the child arm of `execute_supervised` before the sandbox apply, (2) replace the RLIMIT_NPROC warn/write in both the `install_pre_exec` closure and the `execute_supervised` child arm with `libc::setrlimit(libc::RLIMIT_NPROC, …)` fail-closed, (3) add a parent-side `uid_process_count()` helper that calls `sysctl(KERN_PROC_UID)` before fork to compute the baseline.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Per-UID process count (baseline) | CLI supervisor (parent, before fork) | — | Must happen before fork; `sysctl` allocates, so cannot run in child arm |
| `setrlimit(RLIMIT_NPROC)` application | CLI supervised child arm (post-fork pre-exec) | CLI direct path (pre_exec closure) | Must be in forked child before execve; two separate code paths share the same intent |
| Process group isolation (`setpgid`) | CLI supervised child arm (post-fork, async-signal-safe) | — | Must happen immediately after fork, before any exec, only on the supervised path |
| Timeout watchdog kill | CLI supervisor parent thread (spawned after fork) | — | Watchdog targets child pgrp established by child arm |
| Fail-closed error propagation | CLI supervisor parent (baseline count failure → Err before fork; setrlimit failure → _exit(126) in child) | — | Consistent with project fail-secure principle |

---

## Standard Stack

### Core (already in deps — no new packages)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `libc` | 0.2.186 (workspace) | `RLIMIT_NPROC`, `setrlimit`, `setpgid`, `sysctl`, `KERN_PROC`, `CTL_KERN`, `getuid` raw FFI | Already a workspace dep; provides all needed macOS syscall bindings |
| `nix` | 0.31.3 (workspace) | `nix::unistd::getpgid`, `nix::sys::signal::kill` (already used in watchdog) | Already used in watchdog + supervisor; `setpgid` is in `nix::unistd` for macOS |

### No New Packages Required

All required functionality is available via `libc` 0.2.186 and `nix` 0.31.3 — both already in the workspace. No new Cargo dependencies are introduced.

**Version verification (VERIFIED via docs.rs):**
- `libc::RLIMIT_NPROC` = `7` on `x86_64-apple-darwin` and `aarch64-apple-darwin` in 0.2.186 [VERIFIED: docs.rs/libc/0.2.186]
- `libc::setrlimit(resource: c_int, rlim: *const rlimit) -> c_int` available for both Apple Darwin targets [VERIFIED: docs.rs/libc/0.2.186]
- `libc::rlim_t` = `u64` on `aarch64-apple-darwin` [VERIFIED: docs.rs/libc/0.2.186]
- `nix::unistd::setpgid(pid: Pid, pgid: Pid) -> Result<()>` available for `x86_64-apple-darwin` and `aarch64-apple-darwin` in nix 0.31.3 [VERIFIED: docs.rs/nix/0.31.3]
- `nix::unistd::getpgid(pid: Option<Pid>) -> Result<Pid>` available for both Apple Darwin targets in nix 0.31.3 [VERIFIED: docs.rs/nix/0.31.3]

---

## Package Legitimacy Audit

> No new external packages are introduced in this phase — all required functionality comes from `libc` and `nix` which are pre-existing workspace dependencies. This section is N/A.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
nono (supervisor, parent)
  │
  ├─ [PARENT, before fork]
  │    └── uid_process_count() via sysctl(KERN_PROC_UID, getuid())
  │         → baseline: u64   ← captured by Copy into child arm / pre_exec
  │
  ├─ fork() ──────────────────────────────────────────────────────────┐
  │                                                             [CHILD arm]
  │  [PARENT continues]                                   setpgid(0,0)  ← NEW: child becomes own pgrp leader
  │  ├─ getpgid(child) → child_pgrp                      setrlimit(RLIMIT_NPROC, baseline+N)  ← NEW: fail-closed
  │  ├─ spawn watchdog thread:                            Sandbox::apply()
  │  │    sleep(deadline)                                 execve(...)
  │  │    kill(-child_pgrp, SIGKILL)  ← targets CHILD's
  │  │         own group (not parent's)                   ─────────────┘
  │  └─ waitpid(child) → returns after SIGKILL
  │       nono exits
  │
  └─ [Direct path: std::process::Command.pre_exec]
       MacosResourceLimits::install_pre_exec:
         setrlimit(RLIMIT_AS, memory_bytes)
         setrlimit(RLIMIT_NPROC, baseline+N)  ← NEW: replaces warn!
```

### Recommended Project Structure (no new files required)

The fix modifies existing files only:

```
crates/nono-cli/src/exec_strategy/
├── supervisor_macos.rs   # MODIFY: install_pre_exec (RLIMIT_NPROC replace warn)
│                         #         MacosResourceLimits::new() takes baseline u64
│                         #         uid_process_count() helper fn (parent-side sysctl)
└── (mod.rs is exec_strategy.rs)

crates/nono-cli/src/exec_strategy.rs
│   # MODIFY: execute_supervised child arm:
│   #   - setpgid(0,0) immediately after fork (before sandbox apply)
│   #   - RLIMIT_NPROC block: replace warn!-write with libc::setrlimit + _exit(126) on err
│   # MODIFY: execute_supervised parent arm:
│   #   - call uid_process_count() before fork; pass baseline into MacosResourceLimits::new
│   # MODIFY: MacosResourceLimits struct: add baseline_uid_count: u64 field

crates/nono-cli/tests/resl_nix_macos.rs
│   # NO CHANGE to existing tests (they stay env-gated)
│   # OPTIONAL: add D-09 bonus --memory live assertion under NONO_RESL_HOST_VALIDATED

crates/nono-cli/tests/resl_nix_async_signal_safety.rs
│   # VERIFY: MSG_RLIMIT_NPROC_FAIL const is present (already expected by the test at line 253)
```

### Pattern 1: Raw `libc::setrlimit` in Async-Signal-Safe Context

**What:** Call `libc::setrlimit` with `libc::RLIMIT_NPROC` directly (bypassing nix, which lacks macOS support) inside the post-fork child arm. The hard-limit is the minimum of `baseline + N` and the existing system hard limit from `getrlimit`.

**When to use:** In the `pre_exec` closure (Direct path) and the post-fork child arm (Supervised path) where nix's `Resource::RLIMIT_NPROC` is unavailable on macOS. The pattern exactly mirrors the existing `RLIMIT_AS` handling.

**Example (post-fork child arm in `execute_supervised`):**
```rust
// Source: existing RLIMIT_AS pattern at exec_strategy.rs ~line 940-965 (extended for RLIMIT_NPROC)
// SAFETY: setrlimit is async-signal-safe per POSIX; no allocation.
#[cfg(target_os = "macos")]
if let Some(n) = resource_limits.max_processes {
    // baseline_uid_count was computed in the parent before fork (sysctl; not async-signal-safe).
    // Captured by Copy — no allocation, no locks inside this arm.
    let limit_val: libc::rlim_t = macos_baseline_uid_count.saturating_add(u64::from(n));
    // Cap at existing hard limit to avoid EPERM when raising above hard limit.
    let mut existing: libc::rlimit = unsafe { std::mem::zeroed() };
    let _ = unsafe { libc::getrlimit(libc::RLIMIT_NPROC, &mut existing) };
    let hard = if existing.rlim_max == libc::RLIM_INFINITY { limit_val } else { existing.rlim_max };
    let soft = limit_val.min(hard);
    let rl = libc::rlimit { rlim_cur: soft, rlim_max: hard };
    if unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &rl) } != 0 {
        const MSG_RLIMIT_NPROC_FAIL: &[u8] =
            b"nono: setrlimit(RLIMIT_NPROC) failed in pre-exec child; aborting\n";
        unsafe {
            libc::write(libc::STDERR_FILENO, MSG_RLIMIT_NPROC_FAIL.as_ptr().cast(), MSG_RLIMIT_NPROC_FAIL.len());
            libc::_exit(126);
        }
    }
}
```

### Pattern 2: `setpgid(0, 0)` for Deterministic Child Process Group

**What:** Immediately after fork in the child arm (before anything else), call `libc::setpgid(0, 0)` to make the child its own process-group leader with `pgid == child_pid`. The watchdog's existing `getpgid(child)` call in the parent will then reliably return `child_pgrp == child_pid`, and `kill(-child_pgrp, SIGKILL)` targets only the agent tree.

**When to use:** In the `execute_supervised` post-fork child arm, early in the CR-01 child arm block. `setpgid` is not in POSIX async-signal-safe list BUT it is safe in post-fork contexts (it manipulates process group membership, a kernel attribute, not a userspace lock). The existing code already calls `libc::chdir`, `libc::write`, `libc::_exit`, etc. in this context. `setpgid(0,0)` is a standard post-fork operation used by all Unix shells.

**Interaction with `setup_signal_forwarding`:** `setup_signal_forwarding` stores the child PID in `CHILD_PID` and installs handlers that `kill(child_raw, sig)` — these forward to the child's PID, not its process group. Setting `setpgid(0,0)` in the child does NOT affect this: the parent still holds the child PID and signal forwarding still delivers to the child's PID directly. The child's process group is separate from signal forwarding (which is PID-based, not pgrp-based in the handler). PTY and non-PTY paths are both safe.

**Example:**
```rust
// Source: POSIX setpgid(2) — place child in own process group immediately after fork.
// In execute_supervised ForkResult::Child arm, BEFORE sandbox apply:
#[cfg(target_os = "macos")]
{
    // Make the child its own process-group leader so the timeout watchdog's
    // kill(-child_pgrp, SIGKILL) targets only the agent tree, not the parent's group.
    // setpgid(0,0) = setpgid(getpid(), getpid()). Safe in post-fork context.
    // SAFETY: setpgid is a direct kernel call; no userspace lock interaction.
    unsafe { libc::setpgid(0, 0) };
    // Non-fatal if it fails: watchdog's getpgid will still return the parent's group,
    // the D-06/WR-04 skip logic fires, and the watchdog is skipped rather than targeting
    // a wrong group. This is the conservative choice but the planner may choose to
    // fail-closed here too (consistent with D-07).
}
```

### Pattern 3: Parent-Side UID Process Count via `sysctl(KERN_PROC_UID)`

**What:** Before fork, in the parent, use `libc::sysctl` with MIB `[CTL_KERN, KERN_PROC, KERN_PROC_UID, uid]` to get the number of `kinfo_proc` structures for the current UID — this is the per-UID process count.

**Why sysctl over alternatives:**
- `getrlimit(RLIMIT_NPROC)` returns the *limit*, not the *current usage*. Useless for baseline.
- `proc_listpids` with `PROC_UID_ONLY` type: clean API but requires `libproc.h` / `libc::proc_listpids` — not exposed in libc 0.2.186 on all platforms; sysctl is more portable and directly maps to libc constants.
- `sysctl(KERN_PROC_UID)` with `libc::KERN_PROC` / `libc::KERN_PROC_UID` / `libc::CTL_KERN`: all exposed in libc 0.2.186 for Apple Darwin. Returns `kinfo_proc` structures; the count is `returned_bytes / sizeof(kinfo_proc)`. This is the standard macOS pattern used by `ps`, `top`, and other utilities.

**Async-signal-safety:** sysctl is NOT async-signal-safe (allocates kernel-side). This is why the count MUST be computed in the parent before fork and captured as a plain `u64` integer. Zero allocation in the child.

**Fail-closed:** If `sysctl` returns an error (EPERM, ENOMEM), the function returns `Err(NonoError::SandboxInit(...))` and the run is aborted before fork. Consistent with D-07.

**Example:**
```rust
// Source: [CITED: developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/sysctl.3.html]
// Runs in PARENT before fork. NOT async-signal-safe.
#[cfg(target_os = "macos")]
fn uid_process_count() -> Result<u64> {
    use std::mem;
    let uid = unsafe { libc::getuid() };
    let mut mib: [libc::c_int; 4] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_UID, uid as libc::c_int];
    let mut len: libc::size_t = 0;
    // First call: get required buffer size.
    let ret = unsafe { libc::sysctl(mib.as_mut_ptr(), 4, std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) };
    if ret != 0 {
        return Err(NonoError::SandboxInit(format!(
            "sysctl(KERN_PROC_UID) size query failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    let count = len.checked_div(mem::size_of::<libc::kinfo_proc>()).unwrap_or(0);
    Ok(count as u64)
}
```

### Anti-Patterns to Avoid

- **Calling `sysctl` in post-fork child arm:** Not async-signal-safe; may deadlock if parent held kernel lock at fork. Baseline count MUST be in parent.
- **Using `nix::sys::resource::Resource::RLIMIT_NPROC` on macOS:** nix 0.31 does not expose this variant on macOS. Must use `libc::RLIMIT_NPROC` directly.
- **Using `setpgid` without understanding PTY session:** In a PTY-supervised child, the slave fd setup calls `setsid()` (via `setup_child_pty`) which implicitly creates a new session AND process group. On the PTY path, `setpgid(0,0)` before `setup_child_pty` may be superseded — but it is harmless and the watchdog's `getpgid` call happens in the parent after the child's `setsid()`, so the returned pgrp will still be deterministic (== child_pid for setsid, or the explicit group from setpgid(0,0) if called before setsid). Verify ordering.
- **Putting `format!()` inside the CR-01 child arm:** Forbidden per existing CR-01 regression test. All error messages must be `const MSG_*: &[u8] = b"...";` static byte strings.
- **Raising RLIMIT_NPROC above the hard limit:** Always read the existing hard limit via `getrlimit` first and cap `soft` at `min(baseline+N, hard)`. Attempting to set `rlim_max` above the existing hard limit returns EPERM unless root.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| macOS process-group isolation | Custom bookkeeping of PIDs in agent tree | `setpgid(0,0)` post-fork + `kill(-pgrp, SIGKILL)` | Standard POSIX; the OS tracks all descendants |
| UID process count | Parsing `/proc/` or running `ps` | `libc::sysctl(KERN_PROC_UID)` | ps is not available in async-safe context; sysctl is the authoritative kernel interface |
| Applying `RLIMIT_NPROC` | Writing a Mach task_policy wrapper | `libc::setrlimit(libc::RLIMIT_NPROC, …)` | Direct POSIX syscall; no FFI complexity; already used for RLIMIT_AS in same file |

**Key insight:** The nix crate's macOS Resource enum gap is a nix design choice, not a kernel limitation. `RLIMIT_NPROC = 7` is a standard macOS BSD syscall available via `libc` directly. No wrapper crate needed.

---

## Common Pitfalls

### Pitfall 1: Two Code Paths Apply Limits — Both Must Be Fixed

**What goes wrong:** A fix only applied in `supervisor_macos.rs::install_pre_exec` does not fix the `execute_supervised` raw-fork path, and vice versa. The two paths are independent.

**Why it happens:** `execute_supervised` uses raw `nix::unistd::fork()` — it does NOT go through `std::process::Command::pre_exec`. The `install_pre_exec` method registers a `Command::pre_exec` closure used only by `execute_direct` (which calls `cmd.exec()`). The `execute_supervised` function has its own `ForkResult::Child` arm in `exec_strategy.rs` at ~line 878, with its own RLIMIT_NPROC handling (currently the no-op write at ~line 967-986).

**How to avoid:** Check both sites:
- `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` → `install_pre_exec` closure (Direct path)
- `crates/nono-cli/src/exec_strategy.rs` → `ForkResult::Child` arm around line 967 (Supervised path)

**Warning signs:** Test passes on Direct strategy but `--strategy supervised` (the default) still fails.

### Pitfall 2: `setpgid` in Child vs. PTY `setsid` Ordering

**What goes wrong:** On the PTY supervised path, `setup_child_pty` calls `setsid()` inside a `libc::setsid()` call (in `crate::pty_proxy::setup_child_pty`). `setsid()` creates a new session AND a new process group where the child is the group leader. If `setpgid(0,0)` is called BEFORE `setup_child_pty`, the subsequent `setsid()` will still create a new group (good). If called AFTER, there is no effect (already a group leader from setsid). Either order is safe, but MUST be before the watchdog's `getpgid(child)` call in the parent.

**Why it happens:** The watchdog's `getpgid(Some(child))` runs in the parent after the child is spawned (line ~1416), so even if `setup_child_pty` runs in the child and calls `setsid()`, the parent calling `getpgid` afterward will see the child's group as `child_pid` (the result of `setsid`). On the non-PTY path, `setpgid(0,0)` is the only mechanism, so it must be present.

**How to avoid:** Place `setpgid(0,0)` early in the child arm (before `pty_slave_fd` setup), and separately verify the non-PTY path has no `setsid` that would be bypassed.

**Warning signs:** Watchdog fires but targets wrong group on non-PTY path only.

### Pitfall 3: RLIMIT_NPROC Hard Limit — EPERM on Raise

**What goes wrong:** Calling `setrlimit(RLIMIT_NPROC)` with `rlim_max` set to `baseline + N` fails with EPERM if `baseline + N` exceeds the current system hard limit. An unprivileged process cannot raise the hard limit.

**Why it happens:** macOS default hard limit for RLIMIT_NPROC is typically 2666 (derived from `kern.maxproc / 2`). On a developer machine with 400 processes running under the UID, `baseline + 5 = 405` is well within the hard limit. But the code must read `getrlimit(RLIMIT_NPROC)` first and cap `rlim_max` at `min(computed, existing_hard)`.

**How to avoid:** Pattern 3 code example above shows the cap. Always: read existing limits → compute new soft → cap at existing hard → pass `{rlim_cur: soft, rlim_max: min(soft, hard)}`.

**Warning signs:** `setrlimit` returns `EPERM` on some hosts, RLIMIT_NPROC enforcement silently skipped (if not fail-closed).

### Pitfall 4: RLIMIT_NPROC Is UID-Wide, Not Process-Tree-Wide

**What goes wrong:** Test `macos_max_processes_blocks_on_rlimit_nproc` runs on a host under high user-process load and the baseline + 5 limit is too tight — unrelated GUI apps' fork() calls also hit the limit, causing EAGAIN before the bash test loop even starts.

**Why it happens:** RLIMIT_NPROC on macOS counts ALL processes owned by the UID (not just descendants of the sandboxed child). A developer's desktop with 350 processes + baseline + 5 = 355 may be very close to a system limit.

**How to avoid:** D-03 accepts this as documented behavior. The test uses `--max-processes 5` with `run_bounded(20s)`. A developer machine idling at 300 processes with a hard limit of 2666 has 2366 free slots — `baseline + 5` = ~305, which is well within the hard limit. The test is designed for this: 5 additional processes beyond an already-running system is the intent.

**Warning signs:** Test fails on a very busy host even with the fix applied.

### Pitfall 5: CR-01 Regression — No `format!()` in Child Arm

**What goes wrong:** Adding `format!()`, `String::from`, `Vec::push`, or any heap-allocating call inside the `ForkResult::Child` arm (between CR-01-CHILD-ARM-START and CR-01-CHILD-ARM-END sentinels) triggers the CR-01 regression test.

**Why it happens:** `exec_strategy.rs` has an automated test `cr_01_no_format_macro_in_post_fork_child_branch` that scans the source text for `format!(` between the two sentinels. The test also expects `MSG_RLIMIT_NPROC_FAIL` to be a named `const MSG_*: &[u8]` (asserted at `cr_01_and_wr_02_const_msg_byte_strings_present`, line 243, which checks for at least 11 such constants including `MSG_RLIMIT_NPROC_FAIL`).

**How to avoid:** Use `const MSG_RLIMIT_NPROC_FAIL: &[u8] = b"...\n";` declared immediately before the unsafe block. The existing test already names `MSG_RLIMIT_NPROC_FAIL` in its assertion message (line 253) — this const must exist in `exec_strategy.rs` for the count assertion to hold.

**Warning signs:** Build passes but `cargo test -p nono-cli --test resl_nix_async_signal_safety` fails with `found format!(`.

### Pitfall 6: `setup_signal_forwarding` Forwards to PID, Not PGRP — No Conflict

**What goes wrong:** Confusion between `kill(-pgrp, sig)` (kills whole process group) and the signal forwarding handler which does `kill(child_raw, sig)` (kills the specific PID). Someone might worry that `setpgid(0,0)` breaks signal forwarding.

**Why it doesn't:** `setup_signal_forwarding` stores `CHILD_PID` (the child's PID, not its pgid) and the handler does `libc::kill(child_raw, sig)`. `child_raw` is the child PID, which is unchanged by `setpgid`. The forwarding delivers to the child PID directly. The child's process group (`pgid`) is only relevant to the watchdog's `kill(-pgrp, SIGKILL)` and to the PTY controlling-terminal logic. No conflict.

---

## Code Examples

### Reading UID Process Count (Parent Side, Before Fork)

```rust
// Source: [CITED: developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/sysctl.3.html]
// POSIX sysctl MIB: [CTL_KERN, KERN_PROC, KERN_PROC_UID, uid] returns kinfo_proc array.
// NOT async-signal-safe — call in PARENT before fork() only.
#[cfg(target_os = "macos")]
fn uid_process_count() -> nono::Result<u64> {
    let uid = unsafe { libc::getuid() };
    let mut mib = [
        libc::CTL_KERN,
        libc::KERN_PROC,
        libc::KERN_PROC_UID,
        uid as libc::c_int,
    ];
    let mut len: libc::size_t = 0;
    let ret = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            4,
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret != 0 {
        return Err(nono::NonoError::SandboxInit(format!(
            "sysctl(KERN_PROC_UID) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    let count = len
        .checked_div(std::mem::size_of::<libc::kinfo_proc>())
        .unwrap_or(0);
    Ok(count as u64)
}
```

### Applying RLIMIT_NPROC (Post-Fork Child Arm, Async-Signal-Safe)

```rust
// Source: libc 0.2.186 [VERIFIED: docs.rs/libc/0.2.186/aarch64-apple-darwin]
// Called in execute_supervised ForkResult::Child arm — async-signal-safe path.
// baseline_uid_count is a u64 captured by Copy from the parent.
#[cfg(target_os = "macos")]
if let Some(n) = resource_limits.max_processes {
    let target_soft: libc::rlim_t = (baseline_uid_count).saturating_add(u64::from(n));
    // Read existing hard limit — getrlimit is async-signal-safe per POSIX.
    let mut existing: libc::rlimit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    let got = unsafe { libc::getrlimit(libc::RLIMIT_NPROC, &mut existing) };
    let hard_limit = if got == 0 && existing.rlim_max != libc::RLIM_INFINITY {
        existing.rlim_max
    } else {
        target_soft
    };
    let rl = libc::rlimit {
        rlim_cur: target_soft.min(hard_limit),
        rlim_max: target_soft.min(hard_limit),
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &rl) } != 0 {
        const MSG_RLIMIT_NPROC_FAIL: &[u8] =
            b"nono: setrlimit(RLIMIT_NPROC) failed in supervised child; aborting\n";
        // SAFETY: write and _exit are async-signal-safe.
        unsafe {
            libc::write(
                libc::STDERR_FILENO,
                MSG_RLIMIT_NPROC_FAIL.as_ptr().cast::<libc::c_void>(),
                MSG_RLIMIT_NPROC_FAIL.len(),
            );
            libc::_exit(126);
        }
    }
}
```

### setpgid(0,0) in Child Arm

```rust
// Source: [ASSUMED — POSIX setpgid(2); standard post-fork process-group isolation]
// Place child in own process group immediately after fork (before sandbox apply).
// This makes getpgid(child) in the parent return child_pid deterministically.
#[cfg(target_os = "macos")]
{
    // setpgid(0,0) = setpgid(getpid(), getpid()): child becomes its own pgrp leader.
    // SAFETY: setpgid is a direct kernel call; safe in post-fork context.
    let _ = unsafe { libc::setpgid(0, 0) };
    // Note: failure is tolerated here — if setpgid fails, the watchdog's getpgid
    // will still return a group, but it may be the parent's; WR-04 skip logic protects.
    // The planner may tighten this to fail-closed per D-07 (discussed in Open Questions).
}
```

### `install_pre_exec` Fix (Direct Path)

```rust
// Source: existing supervisor_macos.rs pattern for RLIMIT_AS (extended)
// Pre-computed baseline_uid_count is a new field on MacosResourceLimits, captured
// by value (Copy) from the parent. NO allocation inside the closure.
unsafe {
    cmd.pre_exec(move || -> std::io::Result<()> {
        #[cfg(target_os = "macos")]
        {
            use nix::sys::resource::{setrlimit, Resource};
            if let Some(bytes) = memory_bytes {
                let limit: nix::libc::rlim_t = bytes;
                setrlimit(Resource::RLIMIT_AS, limit, limit)
                    .map_err(std::io::Error::from)?;
            }
            if let Some(n) = max_processes {
                // baseline_uid_count is a u64 Copy-captured field; no allocation.
                let target: libc::rlim_t = baseline_uid_count.saturating_add(u64::from(n));
                let mut existing: libc::rlimit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
                let got = unsafe { libc::getrlimit(libc::RLIMIT_NPROC, &mut existing) };
                let hard = if got == 0 && existing.rlim_max != libc::RLIM_INFINITY {
                    existing.rlim_max
                } else { target };
                let rl = libc::rlimit {
                    rlim_cur: target.min(hard),
                    rlim_max: target.min(hard),
                };
                if unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &rl) } != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
        }
        Ok(())
    });
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| nix `Resource::RLIMIT_NPROC` for macOS | Not exposed; use `libc::RLIMIT_NPROC` directly | nix design decision, pre-existing | Must bypass nix for this specific constant on macOS |
| `tracing::warn!` + silent pass for RLIMIT_NPROC on macOS | `libc::setrlimit` + fail-closed `_exit(126)` | This phase | Actually enforces the limit |
| Child inherits parent's process group | `setpgid(0,0)` in child post-fork | This phase | Makes watchdog kill target deterministic |
| Watchdog targets `getpgid(child)` which may be parent's pgrp | Watchdog targets child's own pgrp (set by setpgid) | This phase | Watchdog actually kills the agent tree |

**Note on RLIMIT_NPROC macOS semantics (D-03 required documentation):**
macOS `RLIMIT_NPROC` is UID-wide, unlike Linux `pids.max` which is descendant-tree-scoped. The baseline+N strategy compensates: the child's effective new-process budget is N beyond the already-running UID processes. A developer machine running 300 user processes with `--max-processes 5` gets `RLIMIT_NPROC = 305`; the 306th fork for any UID process fails EAGAIN.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `setpgid(0,0)` is the root cause of the timeout watchdog failure — child inherits parent's process group and `kill(-pgrp)` misses | Architecture Patterns, Pitfall 2 | If the root cause is different (e.g., watchdog thread not scheduled before child exits), additional investigation needed. D-04 authorizes defensive rewrite without a separate debug cycle. |
| A2 | `setpgid` failure in child arm should be tolerated (log and continue) rather than fail-closed | Code Examples — setpgid | If fail-closed is required for D-07 consistency, the child arm should `_exit(126)` on setpgid failure. Planner should decide (see Open Questions). |
| A3 | `libc::KERN_PROC_UID`, `libc::CTL_KERN`, `libc::KERN_PROC`, `libc::kinfo_proc` are all exposed for aarch64-apple-darwin in libc 0.2.186 | Pattern 3, Code Examples | If any of these constants/types are missing from libc on Apple Silicon, alternative approach (proc_listpids or getrlimit soft-limit comparison) needed. Verified `RLIMIT_NPROC`, `setrlimit`, `rlim_t` — sysctl constants assumed from training knowledge. |

**If this table is empty for A3:** The planner should verify `libc::CTL_KERN`, `libc::KERN_PROC`, `libc::KERN_PROC_UID`, `libc::kinfo_proc` compile under `#[cfg(target_os = "macos")]` before implementing the sysctl approach.

---

## Open Questions

1. **Should `setpgid(0,0)` failure be fail-closed (D-07) or tolerated?**
   - What we know: D-07 says "abort the run with a clear error" when a limit cannot be applied. `setpgid` is not strictly a "limit" (it's process group isolation for the watchdog). WR-04 already provides a skip path if `getpgid` fails in the parent.
   - What's unclear: Whether failing to isolate the process group is a security-impacting event (it is a functional bug but not a sandbox escape; the Seatbelt sandbox still applies).
   - Recommendation: Tolerate `setpgid` failure with a `const MSG_SETPGID_FAIL: &[u8]` write to stderr (async-signal-safe), then continue. The watchdog's WR-04 skip logic ensures no wrong-group kill occurs. This is consistent with the defensive-rewrite spirit of D-04 without over-applying D-07 to a non-limit operation.

2. **Does `libc::CTL_KERN`, `libc::KERN_PROC`, `libc::KERN_PROC_UID`, `libc::kinfo_proc` all compile for `aarch64-apple-darwin` in libc 0.2.186?**
   - What we know: `libc::RLIMIT_NPROC`, `libc::setrlimit`, `libc::rlim_t` are confirmed. The sysctl constants are standard macOS BSD kernel interfaces.
   - What's unclear: Whether libc 0.2.186 exposes all sysctl MIB constants for Apple Silicon specifically (docs.rs shows x86_64-apple-darwin easily; aarch64-apple-darwin sometimes has gaps).
   - Recommendation: Planner adds a quick `#[cfg(target_os = "macos")] let _ = libc::CTL_KERN;` smoke check in the implementation task and fails early if it doesn't compile on the CI macOS lane.

3. **D-09 bonus — where should the lightweight `--memory` RLIMIT_AS assertion land?**
   - What we know: `run_bounded` is already used by both gated tests. An RLIMIT_AS assertion would look like: `nono run --memory 32m -- sh -c 'python3 -c "x=[0]*10**9" 2>&1'; assert non-zero exit`.
   - Recommendation: Add as a third `#[test] fn macos_memory_limit_kills_at_rlimit_as()` gated on `host_enforcement_validated()` in `resl_nix_macos.rs`. Use a memory-heavy operation that exceeds 32m. Keep `run_bounded` timeout at 10s.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Real macOS host (`NONO_RESL_HOST_VALIDATED=1`) | RESL-MAC-01, RESL-MAC-02 host gate | ✓ (Oscars-MacBook-Pro, Apple Silicon) | macOS 15+ (aarch64) | None — CI runners cannot validate |
| Cross-target clippy Linux (x86_64-unknown-linux-gnu) | D-10 | ✗ (Windows dev host, ring/aws-lc-sys C-toolchain missing) | — | CI macOS/Linux Clippy lanes (load-bearing signal) |
| Cross-target clippy macOS (x86_64-apple-darwin) | D-10 | ✗ (Windows dev host) | — | CI macOS Clippy lane |

**Missing dependencies with no fallback:**
- Real macOS host UAT (gated; must be done on `Oscars-MacBook-Pro` with `NONO_RESL_HOST_VALIDATED=1`)

**Missing dependencies with fallback:**
- Cross-target clippy: CI is the load-bearing signal per `.planning/templates/cross-target-verify-checklist.md`; mark cross-target REQs PARTIAL/deferred-to-CI.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner + `cargo test` |
| Config file | None (standard Cargo) |
| Quick run command | `cargo test -p nono-cli --test resl_nix_macos` |
| Full suite command | `cargo test -p nono-cli` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RESL-MAC-01 | `--timeout 5s` SIGKILLs `sleep 60` at ~5s on real macOS host | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos macos_timeout_kills_at_deadline` | ✅ |
| RESL-MAC-02 | `--max-processes 5` causes EAGAIN on 6th fork on real macOS host | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos macos_max_processes_blocks_on_rlimit_nproc` | ✅ |
| RESL-MAC-01+02 (CI) | Build/clippy stays green; gated tests skip cleanly | build + clippy | `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | ✅ |
| CR-01 (async-signal-safety) | No `format!()` in child arm; `MSG_RLIMIT_NPROC_FAIL` const present | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✅ |
| WR-04 (no PID fallback) | watchdog uses `match getpgid` with skip-on-Err | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety wr_04_no_pid_fallback_on_getpgid_failure` | ✅ |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli --test resl_nix_async_signal_safety` (fast, source-scan tests; ~5s)
- **Per wave merge:** `cargo test -p nono-cli` (full suite including skip-path for gated tests; ~60s on Windows host)
- **Phase gate:** Full suite green + real macOS host UAT with `NONO_RESL_HOST_VALIDATED=1` before `/gsd:verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements. No new test files needed unless D-09 bonus `--memory` assertion is added (in which case: add one test function to existing `resl_nix_macos.rs`, gated on `host_enforcement_validated()`).

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes | Process group isolation prevents watchdog from targeting wrong process group; fail-closed RLIMIT_NPROC ensures limits are never silently dropped |
| V5 Input Validation | yes | `baseline_uid_count.saturating_add(u64::from(n))` — checked arithmetic; hard-limit cap prevents EPERM; `sysctl` return checked before use |
| V6 Cryptography | no | — |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Agent-spawned processes escape `--max-processes` cap | Elevation of privilege | `setrlimit(RLIMIT_NPROC)` applied before `exec` in child arm; fail-closed on `setrlimit` error |
| Watchdog kills supervisor's process group instead of agent tree | Elevation of privilege / Denial of service | `setpgid(0,0)` in child arm isolates agent tree; WR-04 skip-on-getpgid-failure preserved |
| Baseline count race: new UID processes spawn between sysctl read and `setrlimit` apply | Tampering | Documented/accepted per D-03; baseline+N is inherently racy on macOS due to UID-wide scope; acceptable tradeoff per project decision |
| RLIMIT_NPROC above hard limit causes EPERM → silent degradation | Tampering | Read existing hard limit via `getrlimit` before `setrlimit`; cap at min(soft, hard); fail-closed on `setrlimit` error |

---

## Sources

### Primary (HIGH confidence)
- `docs.rs/libc/0.2.186/aarch64-apple-darwin/libc/constant.RLIMIT_NPROC.html` — `RLIMIT_NPROC = 7` for aarch64-apple-darwin confirmed [VERIFIED: docs.rs/libc/0.2.186]
- `docs.rs/libc/0.2.186/x86_64-apple-darwin/libc/constant.RLIMIT_NPROC.html` — `RLIMIT_NPROC = 7` for x86_64-apple-darwin confirmed [VERIFIED: docs.rs/libc/0.2.186]
- `docs.rs/libc/0.2.186/aarch64-apple-darwin/libc/type.rlim_t.html` — `rlim_t = u64` on Apple Darwin [VERIFIED: docs.rs/libc/0.2.186]
- `docs.rs/libc/0.2.186/aarch64-apple-darwin/libc/fn.setrlimit.html` — `setrlimit(resource: c_int, rlim: *const rlimit) -> c_int` [VERIFIED: docs.rs/libc/0.2.186]
- `docs.rs/nix/0.31.3/x86_64-apple-darwin/nix/unistd/fn.setpgid.html` — `setpgid(pid: Pid, pgid: Pid) -> Result<()>` available for Apple Darwin targets [VERIFIED: docs.rs/nix/0.31.3]
- `docs.rs/nix/0.31.3/x86_64-apple-darwin/nix/unistd/fn.getpgid.html` — available for Apple Darwin targets [VERIFIED: docs.rs/nix/0.31.3]
- `developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/getrlimit.2.html` — RLIMIT_NPROC = "maximum simultaneous processes for this user id" (UID-wide) [CITED: Apple developer docs]
- `developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/sysctl.3.html` — `sysctl([CTL_KERN, KERN_PROC, KERN_PROC_UID, uid], ...)` returns `kinfo_proc` array [CITED: Apple developer docs]

### Secondary (MEDIUM confidence)
- Codebase inspection of `crates/nono-cli/src/exec_strategy.rs` (ForkResult::Child arm ~line 878-1252) — two RLIMIT_NPROC no-op sites confirmed, CR-01 arm structure confirmed [VERIFIED: codebase read]
- Codebase inspection of `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` — `install_pre_exec` RLIMIT_NPROC warn path and `spawn_macos_timeout_watchdog` pgrp kill logic confirmed [VERIFIED: codebase read]
- Codebase inspection of `crates/nono-cli/tests/resl_nix_async_signal_safety.rs` — CR-01 format!() scan, `MSG_RLIMIT_NPROC_FAIL` expected by assertion at line 253 [VERIFIED: codebase read]
- Codebase inspection of `crates/nono-cli/tests/resl_nix_macos.rs` — test structure, `run_bounded` harness, `host_enforcement_validated()` gate confirmed [VERIFIED: codebase read]

### Tertiary (LOW confidence / ASSUMED)
- sysctl constants `libc::CTL_KERN`, `libc::KERN_PROC`, `libc::KERN_PROC_UID`, `libc::kinfo_proc` available for `aarch64-apple-darwin` in libc 0.2.186 [ASSUMED — standard macOS BSD interface; verify at compile time]
- `setpgid(0,0)` is the root cause of the watchdog miss (not a scheduling issue) [ASSUMED — D-04 defensive rewrite; no separate debug cycle authorized]

---

## Project Constraints (from CLAUDE.md)

- **Unwrap Policy:** Strictly forbid `.unwrap()` and `.expect()` in production code. Use `?` or match-based error handling.
- **Async-Signal-Safety:** Post-fork child arm must use only async-signal-safe calls. No `format!()`, `String`, `Vec`, or allocator calls. Named `const MSG_*: &[u8]` + `libc::write` + `libc::_exit` required.
- **CR-01 Sentinel:** All code inside `ForkResult::Child` arm must stay between `// CR-01-CHILD-ARM-START` and `// CR-01-CHILD-ARM-END` sentinels. No `format!(` calls.
- **`MSG_RLIMIT_NPROC_FAIL`:** The async-signal-safety test at `resl_nix_async_signal_safety.rs:243` expects at least 11 `const MSG_*: &[u8]` declarations and names `MSG_RLIMIT_NPROC_FAIL` in the assertion message. This const must exist in `exec_strategy.rs`.
- **Checked Arithmetic:** Use `saturating_add` / `checked_div` for all numeric operations involving process counts.
- **Cross-Target Clippy:** Mandatory for this phase (touches cfg-gated Unix code). Windows dev host cannot cross-compile → CI is load-bearing signal. Mark cross-target REQs PARTIAL/deferred-to-CI.
- **DCO Sign-Off:** All commits require `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
- **GSD Workflow:** Changes must go through `/gsd:execute-phase 68`.
- **No `#[allow(dead_code)]`:** If a new field is added (`baseline_uid_count`) it must be used in both the Direct and Supervised paths.
- **Security First:** Fail secure — parent-side baseline computation failure aborts the run; child-side setrlimit failure calls `_exit(126)`.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — libc 0.2.186 and nix 0.31.3 constants verified against docs.rs
- Architecture: HIGH — code surfaces read directly; two distinct paths (Direct/Supervised) confirmed; process group behavior confirmed via POSIX docs
- Pitfalls: HIGH — CR-01 regression test structure read directly; PITfalls 1/3/5/6 derive from codebase evidence; Pitfall 2 and 4 from documented macOS RLIMIT_NPROC semantics
- sysctl constants: MEDIUM — standard BSD interface confirmed in docs; specific libc 0.2.186 aarch64-apple-darwin compilation unverified (A3)
- Root cause of watchdog bug: MEDIUM — consistent with all observed evidence; D-04 permits defensive rewrite without a debug cycle

**Research date:** 2026-06-12
**Valid until:** 2026-07-12 (stable APIs; libc 0.2.x is stable)
