---
phase: 68
phase_name: "macos-resl-enforcement-fix"
project: "nono - Windows Parity & Quality"
generated: "2026-06-12"
counts:
  decisions: 5
  lessons: 6
  patterns: 5
  surprises: 4
missing_artifacts:
  - "68-UAT.md (no separate UAT doc — UAT was conducted live on Oscars-MacBook-Pro and recorded in 68-02-SUMMARY.md + the debug session)"
---

# Phase 68 Learnings: macOS Resource-Limit Enforcement Fix

> A narrow planned 2-bug fix (setpgid + RLIMIT_NPROC) turned into a five-layer, host-iterated
> debug arc. Both requirements (RESL-MAC-01 `--timeout`, RESL-MAC-02 `--max-processes`) were
> ultimately satisfied on a real macOS host (5/5 gated tests), but most of the effort went into
> defects the original plan never anticipated. These learnings are dominated by *process* lessons
> about validating cfg-gated cross-platform code from the wrong host.

## Decisions

### D1 — Platform-gate SO_RCVTIMEO to Linux only
`set_read_timeout` on the supervisor AF_UNIX socket was changed from `#[cfg(unix)]` to
`#[cfg(target_os = "linux")]`. macOS rejects `SO_RCVTIMEO` on AF_UNIX with EINVAL, which was
aborting every supervised run before enforcement could happen. The Phase 59 slowloris protection
is preserved on Linux; macOS already polls (200ms) so dropping the read-timeout there is safe.

**Rationale:** Restore macOS supervised runs without weakening the Linux slowloris guard.
**Source:** 68-02-SUMMARY.md, 68-02-PLAN.md (T2)

### D2 — `--memory`/RLIMIT_AS is best-effort on macOS (warn, don't abort)
`setrlimit(RLIMIT_AS)` returns EINVAL on macOS arm64 because dyld pre-maps several hundred MiB of
virtual address space before `main()`, so a low `--memory` cap can't be set. The fail-closed
`_exit(126)` was downgraded to a `const MSG_*` warn-and-continue, and the D-09 bonus test was
flipped to assert clean exit. Deeper enforcement is deferred (carry-forward todo).

**Rationale:** A documented kernel limitation shouldn't hard-abort the run; `--memory` is not a
gating requirement (RESL-MAC reqs are timeout + max-processes).
**Source:** 68-02-SUMMARY.md, debug `macos-resl-not-firing.md` (D2)

### D3 — Keep the child's `setpgid(0,0)`; the parent double-setpgid is belt-and-suspenders only
The research-recommended POSIX "double-setpgid" (parent also calls `setpgid(child,child)`) is
effectively a no-op here — the parent always loses the fork/exec race (EACCES if the child
exec'd, ESRCH if it exited). The watchdog targeting is carried entirely by the child's own
`setpgid(0,0)`. The parent call is harmless and left in as documentation/safety, not removed.

**Rationale:** The child setpgid is reliable; the parent variant can't win the race but does no
harm. Removing it is a future cleanup, not a correctness fix.
**Source:** 68-02-SUMMARY.md, debug evidence (host `WARN parent setpgid ... EACCES`)

### D4 — Two-call `proc_listpids(PROC_UID_ONLY)` for an accurate per-UID baseline
`uid_process_count()` was changed from a single NULL-buffer size query to the two-call pattern
(size query → real-buffer call → count non-zero pids). The NULL-buffer call over-reports on macOS
(~819 vs a real ~476), which set `RLIMIT_NPROC = baseline+N` far too loose to ever fire.

**Rationale:** An accurate baseline is required for `--max-processes` to produce a tight,
enforceable cap. macOS *does* enforce RLIMIT_NPROC — the bug was nono's count, not the platform.
**Source:** 68-02-SUMMARY.md, debug host probe (ulimit -u 824 → 474)

### D5 — The real-macOS-host build+test is the load-bearing gate, not Windows `cargo check`
Cross-target clippy stays PARTIAL/deferred-to-CI; the phase's decisive acceptance is the
`NONO_RESL_HOST_VALIDATED=1` gated suite on Apple hardware. Windows `cargo check` is explicitly
NOT accepted as proof for macOS cfg arms.

**Rationale:** Windows can't compile Apple cfg-gated code at all; treating it as a gate let two
compile errors and fmt debt ship earlier in this very phase.
**Source:** 68-VALIDATION.md, 68-02-PLAN.md, CLAUDE.md cross-target rule

---

## Lessons

### Windows `cargo check` is blind to macOS cfg-gated code — three defects slipped through
The 68-01 macOS code "passed" Windows `cargo check` yet shipped: (1) a missing `use nix::libc;`,
(2) `libc::kinfo_proc` which doesn't exist for Apple, and (3) rustfmt debt. None compile/exist on
Windows, so the Windows gate saw nothing. This is the 3rd+ recurrence of the cross-target gap.

**Context:** `mod supervisor_macos;` is not cfg-gated, but every `libc::` use inside is
`#[cfg(target_os = "macos")]` — so Windows compiles the module to nothing.
**Source:** debug `macos-resl-not-firing.md`, 68-02-SUMMARY.md

### "The fix doesn't work" was, first, "the fix was never deployed"
The first host UAT "failure" was a stale binary: the fix commits were committed locally but never
pushed, so the Mac's `git pull` + `cargo build` compiled pre-fix code. Tells: the host printed a
warning string the fix had deleted, and the test tree had 4 tests instead of 5.

**Context:** Always confirm the binary under test actually contains the change (grep a
deleted/added string; check the test count) before concluding the change is wrong.
**Source:** debug evidence (git divergence; "absent from nix" warning on host)

### A test-harness stdin/prompt stall masqueraded as "enforcement not firing" for the whole saga
The gated tests didn't hang on enforcement — `run_bounded` inherited stdin, so nono blocked on its
interactive `Share <cwd>? [y/N]` prompt and never spawned the child. The passing
`macos_no_warnings` test only passed because `.output()` sets stdin to `/dev/null`.

**Context:** When a "feature not working" symptom is a *hang*, suspect the harness/IO setup
(stdin, pipes, prompts), not only the feature. A manual `--allow-cwd` run worked all along.
**Source:** debug breakthrough (manual SIGKILL-137 run vs hanging test)

### macOS `setrlimit` is selectively unreliable: RLIMIT_AS broken, RLIMIT_NPROC fine
RLIMIT_AS can't be set below dyld's pre-mapped VAS (EINVAL) → best-effort. RLIMIT_NPROC *is*
enforced (default 2784) — the failure there was nono's loose baseline, not the platform. Don't
generalize "macOS setrlimit is broken" from one resource to another.

**Context:** Verify each rlimit's macOS behavior independently before declaring a platform wall.
**Source:** 68-02-SUMMARY.md (D2 vs RESL-MAC-02)

### `proc_listpids` NULL-buffer size query over-reports on macOS
The documented "pass NULL to get the size" idiom returns an upper bound, not the UID-filtered
count (~819 vs ~476). You must do the second call with a real buffer and count the returned pids.

**Context:** Don't treat a libproc size-query as the answer; it's only a buffer-sizing hint.
**Source:** debug host probe, supervisor_macos.rs comment

### A background `&` fork that EAGAINs does not change bash's exit code
The original `--max-processes` test asserted on bash's exit code, but `wait` returns 0 for the
jobs that *did* start, so failed background forks are invisible to `$?`. The test couldn't observe
enforcement even when it worked. Fixed by counting started jobs (`jobs -rp | wc -l`).

**Context:** To detect fork-limit enforcement deterministically, count successful spawns, not the
shell's exit status.
**Source:** 68-02-SUMMARY.md (RESL-MAC-02 test fix)

---

## Patterns

### Manual `--allow-cwd` + `ps`/`time` host probe to discriminate hang causes
Running the exact failing command manually (with the cwd prompt removed via `--allow-cwd`),
watching PGIDs and child death timing with a `ps` loop, isolated "watchdog works" from "harness
hangs" and "child in own group but kill misses".

**When to use:** Any time an automated test *hangs* on a real host and you can't tell whether the
feature, the reaping, or the harness is at fault.
**Source:** debug session probes (P-A, timeout probe, max-processes probe)

### Read the *applied* limit from inside the sandbox (`ulimit -u`)
Instead of inferring whether a resource limit took effect, run the sandboxed child as
`bash -c 'ulimit -u'` and compare the applied value to reality. This instantly revealed the
824-vs-481 baseline overcount.

**When to use:** Diagnosing whether a `setrlimit`-based limit is applied and at what value.
**Source:** debug host probe

### `Stdio::null()` on test-harness child spawns to neutralize interactive prompts
Mirror `Command::output()`'s behavior (`.stdin(Stdio::null())`) when a CLI under test may prompt;
the prompt EOFs and auto-resolves instead of blocking on inherited stdin.

**When to use:** Integration tests that `.spawn()` a CLI which can prompt interactively.
**Source:** run_bounded fix (b8822a55)

### Two-call libproc pattern (size query → buffer → count)
`proc_listpids(type, info, NULL, 0)` for an upper-bound size, allocate, call again with the
buffer, then count non-zero entries. Robust against zero-padding and the NULL-query over-report.

**When to use:** Any macOS libproc enumeration where you need an accurate count, not a buffer hint.
**Source:** supervisor_macos.rs uid_process_count()

### Verify a deployed fix by grepping for a string the fix added/removed
Before trusting a host result, assert the binary contains the change: grep a string the fix
deleted (should be 0) or added (should be ≥1), and check artifact counts (e.g. test count).

**When to use:** Remote/host validation of a change pushed from a different machine.
**Source:** debug deployment-confound resolution

---

## Surprises

### The `--timeout` watchdog (the headline target) was working the whole time
RESL-MAC-01 was effectively already satisfied via the child's `setpgid(0,0)` — a manual run
SIGKILLed the child at the deadline (exit 137) early in the debug arc. The original plan's premise
(a pgrp race needing a parent-side fix) was largely wrong; the parent double-setpgid never wins.

**Impact:** Most of the phase's effort went to defects *other* than the planned one (D1/D2, the
harness bug, the baseline). The plan's central fix (parent setpgid) was a no-op.
**Source:** 68-02-SUMMARY.md, debug timeout probe

### "out of scope" turned out to be in-scope
The `set_read_timeout` EINVAL was initially filed as a separate, rollback-only Phase 59 bug and
explicitly marked out of scope. A host probe (P-B) disproved that — it fires in the core RESL
supervised path and was a real blocker (D1).

**Impact:** A confidently-scoped-out confound was actually a load-bearing defect; re-checking the
"out of scope" call cost (and saved) a cycle.
**Source:** debug `macos-resl-not-firing.md` (CORRECTED Out-of-scope section)

### Estimated 2-bug, ~3-edit fix; actual five-layer, multi-cycle host debug
The re-planned 68-02 was scoped as ~3 small edits (D1/D2/D3) with HIGH research confidence. Reality
added two more layers (test-harness stdin hang, proc_listpids overcount) only discoverable on the
host, plus a deployment confound and two compile errors.

**Impact:** Many more Mac round-trips than planned; the "one focused probe" gate at each step kept
each round productive rather than blind.
**Source:** 68-02-PLAN.md (planned) vs 68-02-SUMMARY.md (actual)

### `proc_listpids(PROC_UID_ONLY, NULL, 0)` returned ~1.7× the real count
The size-query over-report wasn't a small slack (a few extra) — it was 819 vs 476, nearly double,
enough to render `--max-processes 5` (cap 824) completely inert.

**Impact:** The magnitude is what made the limit silently ineffective rather than merely slightly
loose; a small over-report would have masked the bug less.
**Source:** debug host probe numbers
