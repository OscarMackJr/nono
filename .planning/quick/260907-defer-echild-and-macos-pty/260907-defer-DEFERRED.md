---
quick_id: 260907-defer
slug: echild-and-macos-pty
date: 2026-09-07
status: deferred_with_diagnosis
milestone: v3.7
covers_triage_items: [6, "item 7 Shell suite"]
---

# Deferred: two items from the CI run `34068950597` triage

Both are **fully diagnosed and deliberately not fixed**. Each needs a platform
this dev host cannot provide, and each is the kind of change that a plausible
guess gets wrong — this triage produced three separate cases where a persuasive
written rationale rested on a false premise, so shipping an unverifiable fix here
would be repeating the pattern the session exists to correct.

---

## Item 6 — `Test (ubuntu-latest)`: three ECHILD failures

```
dynamic_tokens::tests::git_read_toplevel_returns_worktree_root_in_linked_worktree
  git worktree add: Os { code: 10, message: "No child processes" }
exec_strategy::tests::test_supervisor_loop_proxy_only_v4_no_deadlock
  supervisor loop: Sandbox initialization failed: waitpid() failed: ECHILD
exec_strategy::tests::reap_reparented_orphans_reaps_non_primary_and_returns_status_for_primary
  expected primary child's exit status, got None
```

### Mechanism (established, not hypothesised)

`reap_any_terminated_child` (`exec_strategy.rs:3294`) loops on
`waitpid(-1, WNOHANG)` and **discards the status of any child that is not its
target**. `reap_reparented_orphans` (`:3254`) falls through to it whenever the
owned-pid set is empty, and otherwise reaps every `/proc` direct child not in
that set. Both are correct in production — a supervisor *should* reap inherited
orphans.

Rust runs unit tests as **parallel threads of one process**. So any concurrently
running test's `std::process::Command` child is a direct child of that same
process, is not in `owned`, and gets reaped with its status thrown away. The
owning test's `wait()`/`output()` then returns `ECHILD`. `Os { code: 10 }` *is*
ECHILD — that is why the `git worktree` failure looks unrelated but is not.

### Why the obvious fixes do not work

- **A mutex is too narrow.** `waitpid(-1)` can steal from *any* of the 1,700+
  tests in the binary, so a lock would have to be taken by every test that
  spawns a process. `owned_children`'s existing `Mutex` guards the *registry*,
  not the reaping, so it does not help.
- **Isolating one test is insufficient.** There are at least two perpetrators:
  the reaper test calls `waitpid(-1)` directly, and
  `test_supervisor_loop_proxy_only_v4_no_deadlock` reaches the same path through
  `run_supervisor_loop`. They are perpetrator *and* victim of each other, and
  neither protects the third-party `dynamic_tokens` victim.
- **Changing the production reaper is wrong.** Reaping non-owned direct children
  is the intended supervisor behaviour.

### Proposed design

Run each `waitpid(-1)`-invoking test body inside a **forked child process**, so
its wait scope contains only its own descendants:

1. `fork()`; in the child perform the existing setup, reaper calls and
   assertions; write any failure message to stderr and `_exit(1)`, else
   `_exit(0)`.
2. In the parent, `waitpid(that_pid)` — a *targeted* wait, which steals nothing —
   and assert exit 0.

This is structural rather than lock-based, so it cannot be defeated by a test
added later. Applies to both `exec_strategy` tests; `dynamic_tokens` needs no
change once the perpetrators are contained.

### Verification required

`#[cfg(target_os = "linux")]`, so it cannot run on this Windows host at all.
Needs `cross test` (~20 min/iteration). The proof must be a **perturbation**:
confirm the isolated test still fails when the reaper is broken, otherwise the
isolation may simply have made it vacuous.

---

## Item 7 (Shell suite) — `nono shell` is broken on macOS

```
nono: setsid() failed while configuring child PTY    -> _exit(126)
WARN parent setpgid(34387, 34387) failed (ESRCH: No such process)
```

**This is a real product defect, not a test artifact.** Any macOS user running
`nono shell` hits it.

### Mechanism

- `exec_strategy.rs:1166-1174` — **macOS-only** — the post-fork child calls
  `setpgid(0, 0)`, making itself a process-group leader (Phase 68-02's
  "POSIX double-setpgid idiom", for watchdog signal targeting).
- `pty_proxy.rs:329` — `setup_child_pty` then calls `setsid()`, which POSIX
  requires to fail with `EPERM` **when the caller is already a process-group
  leader**.
- `setsid()` fails → `child_setup_pty_fatal` → `_exit(126)`, the exit code CI
  reported.
- The parent's `setpgid … ESRCH` is *downstream noise*: the child was already
  dead when the parent tried.

### The false premise

The comment at `:1156-1159` states:

> On the PTY supervised path, setup_child_pty will call setsid() which supersedes
> this (setsid creates a new session and process group). **Both orderings are
> safe**; setpgid(0,0) here ensures the non-PTY path is also covered (D-05).

"Both orderings are safe" is false. `setsid()` after `setpgid(0,0)` is exactly
the unsafe ordering. The `#[cfg(target_os = "macos")]` gate is why this shows on
the macOS Integration Tests job and not in the Linux ECHILD cluster.

### Proposed fix

`setsid()` already creates a new session **and** a new process group, so it fully
subsumes `setpgid(0,0)`. On the PTY path the `setpgid(0,0)` is redundant *and*
harmful. Gate it so it runs only on the non-PTY path (its stated purpose per
D-05), or perform `setsid()` first and drop the `setpgid` entirely on that arm.

### Verification required

macOS host or CI. The change is post-fork session/signal setup — async-signal
-safety constrained, and a wrong ordering silently breaks either job control or
the controlling terminal. Must be validated by actually running `nono shell`,
not by reasoning.

Note the watchdog interaction: D-04/D-06 rely on `getpgid(child)` returning
`child_pid` for `kill(-child_pgrp, SIGKILL)`. `setsid()` also makes the child a
group leader with pgid == pid, so the watchdog invariant is preserved — but that
must be confirmed on the real platform, not assumed, because assuming is what
produced the current bug.
