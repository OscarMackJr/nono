---
quick_id: 260907-ech
slug: echild-isolation-and-macos-pty
date: 2026-09-07
status: complete
commits:
  - 6ccb3fdb
milestone: v3.7
covers_triage_items: [6, "item 7 Shell suite"]
security_relevant: true
---

# Quick Task 260907-ech — Summary

**Status:** complete. The two previously deferred items.

## Part A — macOS `nono shell` PTY defect (real product bug)

`setpgid(0,0)` made the post-fork child a process-group leader; `setsid()` then
had to fail with `EPERM` → `_exit(126)`. Every `nono shell` on macOS died there.
The comment asserting **"Both orderings are safe"** was the false premise and the
bug.

Fixed in three coordinated places — child-side gate, parent-side gate, and the
watchdog — because gating only the child left the parent free to win the race,
and gating both reopened the race the double-setpgid idiom existed to close.

That third change surfaced a **pre-existing latent hazard**: WR-04 guarded
`getpgid`'s `Err` case but never its wrong-*value* case, so a lost race could
already arm the watchdog on the **parent's** pgid and `kill(-parent_pgrp,
SIGKILL)` the supervisor's own group. Now requires `child_pgrp == child`, with a
bounded 20×5ms retry so the PTY path keeps timeout enforcement rather than
silently losing it.

## Part B — waitpid(-1) test isolation

`reap_any_terminated_child` loops `waitpid(-1)` and discards non-target statuses.
Rust runs unit tests as parallel threads of one process, so it ate other tests'
`Command` children → ECHILD. `git worktree add: Os { code: 10 }` **is** ECHILD,
which is why that failure looked unrelated.

Fixed by re-exec (not fork — `reap_reparented_orphans` takes a `Mutex` on entry,
and a fork from a multithreaded process can inherit it locked).

### The first attempt was wrong and the gate caught it

It isolated **2 of 4** class members. The Linux run then failed with
`spawn isolated test subprocess: ECHILD` — an unisolated sibling had reaped the
isolated test's own subprocess. **Partial isolation moves the failure rather than
removing it.**

Root cause of the miss: the helper was scoped `#[cfg(target_os = "linux")]` while
the file is `#[cfg(not(target_os = "windows"))]`, so two macOS-compiling members
were structurally unreachable from the fix. The cfg I chose defined the class
smaller than it is.

Fixed by mechanical enumeration (scan every `#[test]` body for the three
reaching functions, cross-check against the helper) rather than by eye, which is
how the first pass missed them.

### Class guard

`every_waitpid_minus_one_test_is_isolated_in_a_subprocess` fails if any test
reaching `waitpid(-1)` lacks isolation. It pins the **invariant**, not a
signature spelling — deliberately unlike the CR-01 scan that went dark for ~2.5
months when a signature changed.

## Verification

- `exec_strategy::tests::` on Linux via `cross`: **54 passed / 0 failed**, exit 0.
- **Isolation proven to engage**, not early-return: each isolated test appears
  twice in the log — launcher, then its subprocess reporting `1 passed; 1733
  filtered out`. Four subprocesses for four isolated tests.
- **Guard perturbation-proved:** removing isolation from `reconnect_survival`
  makes it FAIL naming `["reconnect_survival"]` (exit 101); reverted, passes.
- Both cross-target clippy gates `--all-targets` exit 0; fmt clean.

## Verification limits — stated, not implied by the green gates

- **Part A is not proven by any run here.** macOS-only, runtime PTY behaviour;
  only compilation is verified. Needs a real `nono shell` on macOS. Reasoning
  alone is exactly the standard that produced the original bug.
- `reconnect_survival` self-skips its body in the cross container
  (`PtyProxy::new`: `/.local/state/nono` permission denied). Its isolation
  engages; its supervisor-loop body does not execute there.

## Findings worth carrying

1. **A `cfg` can silently define a class smaller than it is.** Scoping the helper
   to Linux made half the perpetrator class unreachable, and nothing about
   reading either test would have revealed it.
2. **Partial fixes to a shared-resource class are worse than none** — they
   relocate the symptom to the code that was fixed, which then looks like a new
   bug in the fix.
3. **A truncating pipe can fake a verification.** The first "green" re-run was
   captured through `tail -35`, so the guard's own result was discarded and it
   nearly passed unexamined. Capture full output when the thing being checked is
   *presence*, not just the final tally.
