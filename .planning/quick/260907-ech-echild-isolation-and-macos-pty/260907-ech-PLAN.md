---
quick_id: 260907-ech
slug: echild-isolation-and-macos-pty
date: 2026-09-07
description: Fix the ECHILD test-isolation race and the macOS setsid/setpgid PTY defect
milestone: v3.7
phase_context: CI run 34068950597 triage — the two deferred items
security_relevant: true
---

# Quick Task 260907-ech

Two items previously deferred with diagnosis, now implemented.

## Part A — macOS `nono shell` PTY defect (real product bug)

`exec_strategy.rs:1166` (macOS-only) made the post-fork child a process-group
leader via `setpgid(0,0)`; `pty_proxy.rs` then called `setsid()`, which POSIX
requires to fail with `EPERM` for a group leader → `child_setup_pty_fatal` →
`_exit(126)`. Every `nono shell` on macOS died there. The parent's
`setpgid … ESRCH` was downstream noise — the child was already gone.

The comment at `:1156-1159` asserted **"Both orderings are safe"**. That premise
is false and was the bug.

### Fix — three parts, because two were not enough

1. **Child side** — `setpgid(0,0)` now runs only when `pty_slave_fd.is_none()`.
   `setsid()` creates a new session AND a new process group (`pgid == pid`), so
   on the PTY path the call is not merely redundant, it is harmful.
2. **Parent side** — `setpgid(child, child)` (`:1917`, the other half of Phase
   68-02's double-setpgid idiom) is gated the same way. Gating only the child
   would have left a live race: the parent can win and make the child a group
   leader before the child ever reaches `setsid()`.
3. **Watchdog** — required, because gating (2) reopens the race the idiom
   existed to close. `getpgid` can return `Ok(parent_pgrp)`, and arming on that
   makes the deadline fire `kill(-parent_pgrp, SIGKILL)` — SIGKILLing the
   supervisor's own process group.

   This is a **pre-existing latent hazard**, not one this change created: WR-04
   guarded the `Err` case but never the wrong-*value* case, so losing the
   double-setpgid race on both halves already exposed it. The watchdog now
   accepts only `child_pgrp == child`, with a **bounded retry** so the PTY path
   still gets a watchdog once `setsid()` lands, instead of silently losing
   timeout enforcement.

### Blast radius — checked, not assumed

The only consumer of the child's process group is the macOS timeout watchdog.
`setup_signal_forwarding` targets `CHILD_PID` (the pid, not the group), so signal
forwarding is unaffected. `setup_child_pty` runs at `:1311`, separated from
`fork()` only by fast syscalls (cgroup placement, `setrlimit`), so the child
becomes a session leader within microseconds — the retry bound is generous.

## Part B — ECHILD test isolation

`reap_any_terminated_child` loops on `waitpid(-1, WNOHANG)` and **discards** the
status of any child that is not its target; `reap_reparented_orphans` falls
through to it when the owned-pid set is empty. Correct in production — a
supervisor *should* reap inherited orphans — but Rust runs unit tests as parallel
threads of **one** process, so a test reaching that code steals other tests'
`Command` children and their `wait()` returns ECHILD.

Three CI failures, one cause: `git worktree add: Os { code: 10 }` (that **is**
ECHILD), `waitpid() failed: ECHILD`, and "got None".

### Why the obvious fixes fail

- **A mutex is too narrow.** `waitpid(-1)` can steal from any of 1700+ tests, so
  the lock would have to be taken by every test that spawns a process.
  `owned_children`'s existing mutex guards the *registry*, not the reaping.
- **Isolating one test is insufficient.** There are **two** perpetrators —
  `reap_reparented_orphans_…` calls `waitpid(-1)` directly, and
  `test_supervisor_loop_proxy_only_v4_no_deadlock` reaches it through
  `run_supervisor_loop`. They are perpetrator *and* victim of each other, and
  neither protects the third-party victim.
- **Fork-then-run is unsafe here.** It would scope the wait correctly, but
  `reap_reparented_orphans` takes `owned_children`'s `Mutex` on entry, and a
  child forked from a multithreaded process can inherit that mutex locked by a
  thread that does not exist in the child → deadlock.

### Fix

`run_isolated_in_subprocess` re-execs the test binary to run **one** test alone
in a fresh process (env-var marker + `--exact` + `--test-threads=1`). A fresh
process has no inherited lock state, and every `waitpid(-1)` caller is confined
to a process running a single test — structural, so a test added later cannot
reintroduce the race. Launchers only `spawn` + targeted-`wait`, never
`waitpid(-1)`.

`dynamic_tokens`' test is a **pure victim** (plain `Command`) and needs no
change — confirmed by reading it, not assumed.

## Verification

- Both cross-target clippy gates at `--all-targets`.
- `cross test` on Linux for the three previously-failing tests, run **together**
  so the concurrency that produced the race is present.
- Part A cannot be executed here: macOS-only, and the failure is a runtime PTY
  behaviour. Compile-verified via `cargo-zigbuild --all-targets`; runtime proof
  requires macOS/CI and is stated as such rather than implied.
