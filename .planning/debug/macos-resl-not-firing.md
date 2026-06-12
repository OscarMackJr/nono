---
slug: macos-resl-not-firing
status: diagnosed
handoff: re-scoped to a planned phase (user decision 2026-06-12) — fix is multi-defect, deferred to /gsd:plan-phase 68
trigger: "Phase 68-01 fix (setpgid(0,0) + libc::setrlimit(RLIMIT_NPROC, baseline+N)) does not make macOS --timeout and --max-processes enforcement fire on a real host, despite compiling and running."
created: 2026-06-12
updated: 2026-06-12
phase: 68
requirements: [RESL-MAC-01, RESL-MAC-02]
---

# Debug Session: macOS RESL enforcement still not firing (Phase 68)

## ✅ DIAGNOSIS COMPLETE — handoff to re-plan (2026-06-12)

This session diagnosed why Phase 68's macOS RESL fix doesn't work on a real host. Two non-bugs were
cleared first (a stale/undeployed binary, then two macOS-only compile errors — see Evidence + the
Resolution sub-steps below), which got us to a real test of the actual fix. The real test exposed
that **the macOS supervised path has multiple foundational defects, broader than Phase 68's planned
2-bug scope.** Per user decision, the FIX is re-scoped to planned work (`/gsd:plan-phase 68`); this
session is the diagnostic input. The confirmed defect set:

- **D1 — `set_read_timeout` / SO_RCVTIMEO EINVAL on the AF_UNIX supervisor socket** (exec_strategy.rs:1381,
  socket.rs:194). Fires in the CORE RESL supervised path (NOT rollback-only — earlier "out of scope"
  call was falsified by probe P-B). Aborts/destabilizes supervised runs. Pre-dates Phase 68 (Phase 59 IPC).
  Fix direction: replace SO_RCVTIMEO with a poll/recv-deadline on macOS (or skip the read-timeout there).
- **D2 — `setrlimit(RLIMIT_AS, N)` fails in the child** (exec_strategy.rs:1003) for `--memory`. macOS
  rejects the low RLIMIT_AS; the child `_exit(126)`s. Memory enforcement broken at the syscall level.
  Pre-dates Phase 68. Needs its own root-cause (why setrlimit(RLIMIT_AS) EINVAL/EPERMs on macOS arm64).
- **D3 — `--timeout` watchdog + `--max-processes` non-enforcement** — Phase 68's original targets
  (setpgid + RLIMIT_NPROC). The Phase 68 code change may be correct but is unobservable behind D1/D2.
  Note: the deployed Phase 68 source (commits 1b2e2ad0/f94c1c1b/3583bacc + compile fixes 53501113/
  fa6c2dc6) is ON origin/main; re-plan should decide whether to keep, revise, or gate it.

OPEN DATA POINT for the re-plan: P-A (`sleep 3`, no flags) — did nono exit at ~3s (basic reaping OK)
or hang (reaping broken)? Not yet confirmed; cheap to capture on the host during planning research.

PROVENANCE (2026-06-12, blame): D1+D2+D3 are all FORK-authored macOS/Unix code on `origin/main`
(`OscarMackJr/nono`), NOT upstream and NOT Windows-specific. D1 set_read_timeout wiring = `e9032edd`
(Phase 59-02, oscarmackjr-twg); D2 RLIMIT_AS fail-closed child block = `28df5c50` (Phase 25-03,
oscarmackjr-twg); the `set_read_timeout` *method* alone is upstream (Luke Hinds `d46d6026`).
`upstream/main` (always-further/nono, local ref possibly stale) has 0 hits for `supervisor_ipc_read_timeout`
and the RLIMIT_AS child block. These are macOS resource-limit + supervisor-IPC features the fork built but
never ran on a real Mac (Windows dev host → Windows `cargo check` only). The re-plan fixes the fork's own
macOS code; there is no upstream "known-good" to diff against for these paths.

Cross-cutting lesson: the Phase 68 macOS code was authored on a Windows host and shipped through a
Windows-only `cargo check` that cannot compile Apple cfg arms — two compile errors + fmt debt slipped
through ([[feedback_clippy_cross_target]], 3rd+ recurrence). The re-plan MUST treat a real macOS
build+test (host or CI) as the load-bearing gate, not Windows `cargo check`.

## ⚠ Platform constraint (READ FIRST — shapes the whole investigation)

The bug **only reproduces on macOS** (Apple Silicon, `Oscars-MacBook-Pro`). The dev host running
this debug session is **Windows (win32, PowerShell)** and **cannot build or run the macOS code
paths** — all of `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` and the
`#[cfg(target_os = "macos")]` arms in `exec_strategy.rs` are cfg-gated out on Windows. Therefore:

- The debugger **cannot reproduce locally**. Investigation = static code analysis (read the macOS
  code paths) → form hypotheses → produce **specific host-side commands for the USER to run on the
  Mac** → user pastes output → refine. This is a human-in-the-loop, checkpoint-heavy session.
- Do NOT propose "run the test locally to confirm." Propose Mac commands and ask the user to run them.
- Any code fix will likewise need the user to `cargo build -p nono-cli` + re-run the gated tests on
  the Mac to verify.

## Symptoms (prefilled from Phase 68 execute-phase UAT)

- **Expected:** `nono run --timeout 5s -- sleep 60` SIGKILLs the child at ~5s (exit non-zero,
  elapsed 3–10s). `nono run --max-processes 5 -- bash -c "for i in $(seq 1 20); do sleep 5 & done; wait"`
  causes `fork()` to fail with EAGAIN past the cap (child exits non-zero).
- **Actual:** Both hang until the test harness's bounded kill fires:
  - `macos_timeout_kills_at_deadline` → FAILED: "nono did not exit within 12s" (watchdog never SIGKILLs).
  - `macos_max_processes_blocks_on_rlimit_nproc` → FAILED: "nono did not exit within 20s" (no EAGAIN).
- **Error messages:** None from the RESL path itself — it just doesn't enforce. (Test panic is the
  harness's bounded-timeout guard, not a nono error.)
- **Timeline:** REQ-RESL-NIX-03 macOS enforcement was never host-validated (Phase 37 host-blocked).
  Phase 65 gate-65-A first confirmed it broken on a real host (the "A5" finding). Phase 68-01 attempted
  a fix (this session debugs why that fix is a runtime no-op).
- **Reproduction (on the Mac):**
  ```
  NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos -- --nocapture
  ```
  (Tests at `crates/nono-cli/tests/resl_nix_macos.rs`. Each spawns the release/debug `nono` binary
  via `run_bounded`. `cargo build -p nono-cli` SUCCEEDS on macOS — the fix IS in the binary.)

## What the Phase 68-01 fix changed (already applied, commits 1b2e2ad0, f94c1c1b, 3583bacc)

- `supervisor_macos.rs`: added `uid_process_count()` (parent-side `sysctl(KERN_PROC_UID)`),
  `MacosResourceLimits::baseline_uid_count`, and replaced the `tracing::warn!` RLIMIT_NPROC no-op in
  `install_pre_exec` (**Direct path**) with real `libc::setrlimit(RLIMIT_NPROC, baseline+N)`.
- `exec_strategy.rs` `ForkResult::Child` arm (**Supervised path**, CR-01 region): added `setpgid(0,0)`
  (so the watchdog's `kill(-child_pgrp, SIGKILL)` targets only the child group) and replaced the
  `MSG_RLIMIT_NPROC_UNAVAILABLE` no-op with real `libc::setrlimit(RLIMIT_NPROC)` + `_exit(126)` on fail.
- Watchdog (unchanged, parent arm `exec_strategy.rs:~1409`): `match getpgid(Some(child))` →
  `supervisor_macos::spawn_macos_timeout_watchdog(deadline, child_pgrp)` which sleeps to the deadline
  then `kill(-child_pgrp, SIGKILL)` (WR-04: no PID fallback on getpgid Err).

## Leading hypotheses to test (static-analysis candidates)

1. **Wrong exec strategy / path not reached.** Default `nono run` selects Supervised (`select_exec_strategy`).
   Does the macOS supervised run actually go through the `ForkResult::Child` arm that was modified, or a
   different launch path (PTY setup, posix_spawn, `std::process::Command`) that bypasses both the modified
   child arm AND `install_pre_exec`? If neither modified site executes, the fix is a no-op by construction.
   → First probe: `NONO_LOG=debug ./target/debug/nono run --timeout 5s --read=/bin --read=/usr --read=/private -- sleep 60`
   on the Mac. Look for: which strategy is logged, whether `spawn_macos_timeout_watchdog` is reached,
   whether getpgid succeeds, whether the setpgid/setrlimit child-arm log lines appear.
2. **Watchdog thread spawns but the kill misses / the parent never reaps.** `kill(-child_pgrp, ...)`
   could target the wrong group (if setpgid ran in the child but the parent captured getpgid BEFORE the
   child's setpgid took effect — a fork race), or the supervisor's `wait()` blocks elsewhere so even a
   killed child doesn't let nono exit. → Check whether `sleep 60` actually dies at ~5s (`ps`/Activity
   Monitor during the run) even if nono itself doesn't exit — that distinguishes "kill missed" from
   "child died but supervisor hangs".
3. **RLIMIT_NPROC applied but ineffective.** macOS RLIMIT_NPROC is UID-wide; if the baseline read is
   wrong (e.g. baseline already ≥ hard limit, or the cap logic clamps target to the existing soft so it
   never tightens), setrlimit "succeeds" but doesn't constrain. → Probe: a tiny `nono run --max-processes 1`
   and observe; add temporary debug logging of the computed rlim_cur/rlim_max.
4. **Timeout-only runs may not create the watchdog at all** if `timeout_deadline` is None on the path
   taken, or if the watchdog is gated behind a `supervisor_sock`/IPC condition that isn't met for static
   `--read` runs.

## Current Focus

hypothesis: H-REAP (post-deploy, real signal). With the fix actually running (binary e9fbedc7),
  all THREE enforcement tests hang — including `macos_memory_limit_kills_at_rlimit_as` whose
  RLIMIT_AS path PREDATES Phase 68. Since python3's allocate-then-exit child finishes in <1s
  regardless of RLIMIT_AS, the 10s hang means the macOS supervisor does not detect/reap the
  child's exit. Dominant hypothesis: the macOS run_supervisor_loop fails to terminate when the
  child exits, masking every enforcement mechanism. Open sub-question: did Phase 68's added
  `setpgid(0,0)` REGRESS reaping (old-code probe showed nono/bash exiting ~t=3s; new code hangs),
  or is reaping independently broken and Phase 68's fix simply unobservable behind it?
test: Two host probes on the DEPLOYED binary (e9fbedc7), both with `--allow-cwd` to remove the
  cwd-prompt confound and `NONO_LOG=debug` + a `ps` watcher:
  (P-A) PURE REAP, no enforcement: `nono run --allow-cwd --read=/bin --read=/usr --read=/private
    -- sleep 3`. Does nono exit at ~3s (reaping OK) or hang past 3s (reaping broken)?
  (P-B) MEMORY w/ marker: `nono run --allow-cwd --memory 32m --read=... --read=/usr/lib --
    python3 -c "x=bytearray(256*1024*1024); print('ALLOCATED', flush=True)"`. Does "ALLOCATED"
    print (RLIMIT_AS NOT enforced) or python die first (enforced)? Does nono linger after python
    exits (reaping bug)?
expecting: P-A is the discriminator. sleep 3 hanging => pure supervisor-reap defect (fix that
  first; it gates everything). sleep 3 exiting cleanly at 3s => reaping is fine and the bug is in
  each enforcement mechanism (watchdog kill targeting + setrlimit efficacy) — pivot back to H2'.
  P-B tells us whether RLIMIT_AS fires and whether nono reaps a fast-exiting child.
next_action: CHECKPOINT — hand P-A and P-B to the user to run on Oscars-MacBook-Pro and paste
  the logs + `ps` + `time` output back.

## Root Cause

The Phase 68 fix was never deployed to the macOS test host. The fix commits exist only on the
Windows dev host's local `main` (17 commits ahead of `origin/main`, unpushed). The Mac's
`git pull origin main` fetched `848ce71d` (which lacks the fix) and `cargo build` compiled
pre-fix source, so the UAT exercised the old no-op behavior. This is a deployment/sync confound,
not a defect in the setpgid/setrlimit fix. The fix's correctness remains UNVERIFIED pending a
re-test against the actually-deployed code.

## Second defect (exposed once the fix actually compiled on the Mac): macOS-only compile error

- timestamp: 2026-06-12 (Mac `cargo build` after deploying the fix)
  symptom: "cannot find module or crate libc" on macOS.
  root cause: Phase 68 Task 1 added bare `libc::` calls to `supervisor_macos.rs`, but that module
    had no `libc` in scope. `nono-cli` has NO direct `libc` dependency — it reaches libc via nix's
    re-export. `exec_strategy.rs` works because of `use nix::libc;` (line 25); `supervisor_macos.rs`
    had no such import (its pre-existing code used fully-qualified `nix::libc::rlim_t`). The new bare
    `libc::` failed to resolve → E0433. **Windows `cargo check` could not catch it** (the
    `mod supervisor_macos;` is not cfg-gated, but every bare `libc::` use is `#[cfg(target_os="macos")]`)
    — the THIRD recurrence of the cross-target compile gap ([[feedback_clippy_cross_target]]).
  fix: `#[cfg(target_os = "macos")] use nix::libc;` in supervisor_macos.rs (commit 53501113, pushed).
  note: This is WHY the first push still couldn't be tested — the fix never compiled on the Mac.
    Reviewed both files' new macOS code statically after the fix; no other unresolved `libc::`,
    all unsafe extern calls are inside `unsafe` blocks, `macos_baseline_uid_count` is cfg-gated, and
    the `baseline_uid_count()` accessor exists. Residual risk: cannot fully compile macOS from the
    Windows host — if `cargo build` surfaces a further macOS-only error, fix iteratively.

## Resolution

1. ✅ DONE — pushed local `main` → `origin/main` (848ce71d..63dfd9a5, 18 commits, safety-checked clean). 2026-06-12.
1b. ✅ DONE — fixed macOS compile error #1 (missing `use nix::libc;`), pushed `53501113`. 2026-06-12.
1c. ✅ DONE — fixed macOS compile error #2 (`libc::kinfo_proc` absent for Apple in libc 0.2.x):
    rewrote `uid_process_count()` to use `proc_listpids(PROC_UID_ONLY, uid, NULL, 0)` (byte count /
    size_of::<c_int>()); `PROC_UID_ONLY=4` defined locally. Audited ALL libc symbols the new macOS
    code uses against the cached libc-0.2.186 apple module — every other symbol (RLIMIT_NPROC,
    RLIM_INFINITY, rlim_t, setrlimit, getrlimit, rlimit, setpgid, getuid, proc_listpids, write,
    _exit, STDERR_FILENO, c_void, c_int, size_t) is present. Also applied pending rustfmt to the
    Phase 68 macOS blocks. Pushed `fa6c2dc6`. resl_nix_async_signal_safety stays 5/5.
    NOTE: these two compile errors are the SAME cross-target gap root cause — the Phase 68 macOS
    code was authored on a Windows host and never compiled for Apple before deploy
    ([[feedback_clippy_cross_target]], 3rd+ recurrence). After fa6c2dc6 the full libc-symbol audit
    is done, so further macOS compile errors are unlikely (but cannot be 100% ruled out without a
    real macOS/CI compile).
2. PENDING — Mac: `git pull origin main` → **verify fix landed** (`grep -rc "absent from nix" crates/` MUST be 0; D-09 test present; `cargo test` should now show 5 tests not 4) → `cargo build -p nono-cli` → re-run `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos`.
3. PENDING — Evaluate the FIRST real signal. PASS → phase 68 verifiable. FAIL → re-open `/gsd:debug continue macos-resl-not-firing` with the H2'(a/b/c) host probes (now meaningful — the new setpgid/setrlimit code will actually be running).

## ~~Out of scope~~ — CORRECTED 2026-06-12: the set_read_timeout EINVAL IS in scope

EARLIER ASSUMPTION (FALSIFIED): "the `set_read_timeout` EINVAL is a separate rollback-only Phase 59
bug; RESL tests bypass it." Host probe P-B disproved this — the EINVAL fires in the core RESL
supervised path (`exec_strategy.rs:1381`, `supervisor_sock` Some for every supervised run with an IPC
socket; macOS does not accept SO_RCVTIMEO on the AF_UNIX socketpair, at least once the child end has
closed). The todo `.planning/todos/pending/20260612-macos-supervisor-ipc-rcvtimeo-einval.md` is the
SAME root defect and is now a blocker for Phase 68, not a side issue. Fix direction unchanged: replace
SO_RCVTIMEO with a poll/recv-deadline on macOS (or skip the read-timeout on macOS).

## Evidence

- timestamp: 2026-06-12 (static analysis pass 1)
  checked: launch_runtime.rs::select_exec_strategy (lines 543-558)
  found: Unconditionally returns ExecStrategy::Supervised. All five inputs are
    `let _ =`-discarded. So a default `nono run --timeout`/`--max-processes` ALWAYS
    takes the Supervised path.
  implication: H1's premise "wrong strategy" is false at the strategy-selection level.
    The Supervised raw-fork path is the one that runs.

- timestamp: 2026-06-12 (static analysis pass 1)
  checked: exec_strategy.rs supervised macOS path: setpgid (966-987), setrlimit
    NPROC/AS in ForkResult::Child (989-1064), watchdog spawn in ForkResult::Parent
    (1484-1507). Gating: setrlimit on `macos_resource_limits.is_some()` (=
    resource_limits non-empty), watchdog on `timeout_deadline.is_some()`.
  found: ALL three modified sites are structurally ON the default supervised path.
    `timeout` flows RunArgs -> ResourceLimits.timeout -> timeout_deadline (877-879);
    `max_processes` flows into MacosResourceLimits and the child arm. The modified
    code is reached for `--timeout`/`--max-processes` runs.
  implication: H1 (modified sites bypassed / structural no-op) is REFUTED by static
    reading. The fix code IS on-path. The no-op is a RUNTIME failure of the
    mechanism, not a path-selection miss. Pivot to H2 (watchdog kill misses / wrong
    pgrp) and H3 (rlimit ineffective).

- timestamp: 2026-06-12 (static analysis pass 1)
  checked: supervised_runtime.rs:343-434, 472-496 — execute_supervised is ALWAYS
    called with `Some(&supervisor_cfg)`. exec_strategy.rs:626-627: on macOS
    `needs_child_ipc = supervisor.is_some()` (NO extra static-grant gating, unlike
    Linux 619-624). So `socket_pair = Some(SupervisorSocket::pair())` is created for
    EVERY macOS supervised run, including the static-`--read` RESL tests.
  found: The debug file's "Out of scope" claim — "RESL tests use only static --read
    grants (no IPC socket) so they bypass [the set_read_timeout EINVAL]" — is FALSE
    on macOS. The socketpair IS created and `sock.set_read_timeout(...)?` at line 1378
    DOES run for these tests.
  implication: IMPORTANT NUANCE. However, line 1378 uses `?` — if it errored, the
    function returns Err BEFORE the watchdog spawn (1484) and supervisor loop (1550),
    giving nono a FAST non-zero exit. The observed symptom is a 12s/20s HANG (the
    test's `try_wait` loop keeps seeing nono alive). A fast Err-exit cannot produce
    that hang. Therefore EITHER set_read_timeout does NOT error for socketpair on
    macOS (the filed EINVAL is on the named-socket bind path, socket.rs:194, not the
    socketpair path), OR it errors and something else hangs. The hang symptom is
    only consistent with: nono reaches run_supervisor_loop and blocks there because
    the watchdog never SIGKILLs the child. => set_read_timeout is NOT the cause of the
    hang (consistent with marking it out of scope), but the analysis CONFIRMS nono
    proceeds past 1378 into the supervisor loop.

- timestamp: 2026-06-12 (static analysis pass 1)
  checked: supervised_runtime.rs::should_allocate_pty (125-131) +
    create_session_runtime_state (184). For a non-interactive, non-detached
    `nono run -- sleep 60` (exactly the RESL test invocation), should_allocate_pty
    returns FALSE.
  found: pty_pair = None for the RESL tests. Therefore in the child arm,
    setup_child_pty/setsid is NOT called (line 1091-1093 skipped); the child's process
    group is established SOLELY by `setpgid(0,0)` at line 974. In the parent,
    pty_proxy = None, so run_supervisor_loop runs with `pty = None`.
  implication: The watchdog's target pgrp depends entirely on the child's
    `setpgid(0,0)` (974) winning the race against the parent's `getpgid(Some(child))`
    (1493). There is NO parent-side setpgid and NO synchronization barrier between
    fork() and the parent's getpgid read — classic POSIX fork/setpgid race. This is
    the prime suspect for H2.

- timestamp: 2026-06-12 (static analysis pass 1)
  checked: run_supervisor_loop (macOS variant, 2589-2828). Child-kill on a deadline
    happens ONLY via `startup_deadline` (2776-2788), which is `startup_timeout`, NOT
    `--timeout`. The `--timeout` kill is delivered exclusively by the detached
    watchdog thread `spawn_macos_timeout_watchdog` (supervisor_macos.rs:316) doing
    `kill(-child_pgrp, SIGKILL)`. For `sleep 60`, the child never writes/closes the
    supervisor socket, so the loop just polls (200ms) + waitpid(WNOHANG) forever until
    something kills the child.
  found: If the watchdog's `kill(-child_pgrp, SIGKILL)` targets a wrong/empty pgrp
    (ESRCH), the child is never killed, the supervisor loop spins indefinitely, and
    nono hangs until the test's bounded kill — EXACTLY the observed symptom.
  implication: Root-cause candidate confirmed by mechanism. Need host confirmation of
    WHICH way the kill misses (wrong pgrp vs ESRCH) before fixing. The robust fix
    (independent of the race outcome) is to ALSO call `setpgid(child, child)` in the
    PARENT immediately after fork (idempotent double-setpgid idiom) so the parent both
    closes the race AND learns the deterministic pgrp without getpgid — OR capture the
    pgrp deterministically as `child` itself once setpgid is guaranteed.

- timestamp: 2026-06-12 (static analysis pass 1)
  checked: The PASSING test `macos_no_warnings_on_resource_flags` runs a supervised
    `nono run --memory 4g --max-processes 1000 --timeout 60s --read=... -- echo hi`
    via blocking `.output()` and is NOT in the failing set.
  found: A supervised macOS run with the socketpair created DOES complete for a
    short-lived child (`echo hi`). So the supervisor-loop reaping + socketpair teardown
    work for a child that exits on its own. The failure is specific to children that
    stay alive long enough to require external termination (`sleep 60`) or that fork
    a tree under a low cap (`bash ... wait`).
  implication: Rules out "supervised macOS path is universally broken." Narrows to the
    long-lived-child termination/enforcement path.

- timestamp: 2026-06-12 (HOST PROBE 1+2 on Oscars-MacBook-Pro — DECISIVE)
  checked: /tmp/nono-maxproc.log runtime stderr + `ps` watcher PGID columns.
  found (SMOKING GUN): the --max-processes run printed the EXACT string
    `nono: --max-processes is unavailable on macOS (RLIMIT_NPROC absent from nix v0.31's
    macOS subset); continuing without enforcement`. That literal string was DELETED from
    the source by Phase 68 (commit f94c1c1b) — `grep -rc "absent from nix" crates/` = 0 in
    the current tree. A binary that prints it was built from PRE-Phase-68 source.
    Corroboration: (1) host `ps` shows the child sleep/bash with PGID == the PARENT nono's
    PGID (16006/16165), i.e. NO `setpgid(0,0)` ran — exactly the old code; (2) the FIRST
    UAT listed `running 4 tests`, missing the Phase 68 D-09 test `macos_memory_limit_
    kills_at_rlimit_as` (added in 3583bacc) — so the Mac's test tree is pre-Phase-68 too.
  implication: The Mac compiled and ran code that does NOT contain the fix. Every host
    probe describes the OLD no-op behavior. The static-analysis hypotheses H2'(a/b/c)
    about setpgid races / watchdog targeting are MOOT for this data — that new code never
    executed on the host.

- timestamp: 2026-06-12 (git deployment-state check — ROOT CAUSE)
  checked: `git rev-list --left-right --count origin/main...HEAD` = `0 17`;
    `git merge-base --is-ancestor 1b2e2ad0 848ce71d` = false; the Mac's earlier
    `git pull origin main` advanced to `848ce71d`.
  found: The Phase 68 fix commits (1b2e2ad0, f94c1c1b, 3583bacc + tracking) live ONLY on
    the Windows dev host's local `main`, which is 17 commits AHEAD of `origin/main` and was
    never pushed. `origin/main` = 848ce71d does NOT contain the fix. The Mac's
    `git pull origin main` + `cargo build` therefore compiled pre-fix source.
  implication: ROOT CAUSE. The "fix doesn't work" signal is a STALE-BINARY / undeployed-fix
    confound, NOT a defect in the Phase 68 code. The fix's real correctness is UNVERIFIED —
    it must be deployed (push origin/main → Mac re-pull + rebuild) and re-tested before any
    conclusion about whether setpgid + setrlimit actually work. Push-safety verified: no
    build_notes/ or .gsd/ paths in the 17 unpushed commits (public-repo gate honored).

- timestamp: 2026-06-12 (static analysis pass 1)
  checked: `--max-processes` repro semantics. `bash -c "for i in $(seq 1 20); do sleep
    5 & done; wait"` — with NO enforcement all 20 `sleep 5 &` start, `wait` returns at
    ~5s, bash exits 0. With enforcement, fork EAGAINs past the cap, the failed `&`
    jobs error, `wait` returns at ~5s, bash exits non-zero. EITHER WAY bash exits at
    ~5s. The child arm RLIMIT_NPROC setrlimit clamps soft to the existing hard limit
    (exec_strategy.rs:1041-1046) so it cannot EPERM/_exit(126) spuriously.
  found: The observed 20s HANG for `--max-processes` is NOT explained by an ineffective
    RLIMIT_NPROC (H3) — weak enforcement would still let bash exit at ~5s, not hang.
    The hang means nono is not detecting/reaping bash's exit, i.e. the supervisor loop
    keeps `waitpid(bash, WNOHANG) == StillAlive` (or the child tree is not exiting).
    This is a termination/reaping bug, plausibly SHARED with the timeout repro's
    "child never dies / nono never exits" mechanism.
  implication: Two failing tests likely share a common reaping/termination defect in
    the macOS supervised long-lived-child path, distinct from RLIMIT_NPROC efficacy.
    Host data needed to confirm whether bash actually exits at ~5s while nono hangs
    (=> reaping bug) or bash stays alive (=> enforcement + something keeps it running).

- timestamp: 2026-06-12 (FIRST REAL SIGNAL — gated tests vs the DEPLOYED fix, binary e9fbedc7)
  checked: `NONO_RESL_HOST_VALIDATED=1 cargo test --test resl_nix_macos` after the compile
    fixes (53501113 + fa6c2dc6). The new code is confirmed running: `running 5 tests` (the
    Phase 68 D-09 test is present) and the build compiled the actual fix.
  found: 2 pass / 3 FAIL. `macos_cpu_percent_rejected` + `macos_no_warnings_on_resource_flags`
    pass. ALL THREE enforcement tests HANG to their bounds: timeout (12s), max-processes (20s),
    AND **`macos_memory_limit_kills_at_rlimit_as` (10s)**. The memory test is the key new datum:
    RLIMIT_AS enforcement is PRE-Phase-68 (the `--memory`/RLIMIT_AS child-arm block existed
    before this work), yet it ALSO hangs. Its child `python3 -c "x=bytearray(256MB)"` allocates-
    then-exits in well under 1s REGARDLESS of whether RLIMIT_AS fires, so a 10s hang cannot be a
    "limit too weak" result — it can only mean nono does not detect/reap the child's exit.
  implication: The defect is BROADER than Phase 68's setpgid/NPROC changes — it sits in the
    macOS supervised path shared by all three (RLIMIT_AS predates Phase 68). Dominant hypothesis
    H-REAP: the macOS run_supervisor_loop fails to terminate/reap a child that exits on its own
    (or stays alive), so nono hangs and every enforcement mechanism is masked. The passing
    `macos_no_warnings` test (echo hi via `.output()`) shows SOME supervised child does get
    reaped — so the bug is conditional (child kind / how the harness waits). NOTE: this means
    Phase 68's specific fix may be CORRECT but unobservable until the reaping defect is fixed —
    OR Phase 68's `setpgid(0,0)` itself regressed reaping (the prior OLD-code probe showed
    nono/bash exiting by ~t=3s; the NEW code hangs). Must isolate with a no-enforcement reap
    probe (`sleep 3`, no flags) and a python-with-marker probe on the DEPLOYED binary.

- timestamp: 2026-06-12 (HOST PROBES P-A + P-B on the DEPLOYED binary — MULTI-DEFECT)
  checked: /tmp/nono-sleep3.log (P-A: `nono run --allow-cwd --read=... -- sleep 3`, no enforcement
    flags) and /tmp/nono-mem.log (P-B: `--memory 32m -- python3 -c "bytearray(256MB); print(...)"`).
  found:
    * P-B printed TWO errors:
      (1) `nono: setrlimit(RLIMIT_AS) failed in pre-exec child; aborting` — the SUPERVISED child arm
          RLIMIT_AS block (exec_strategy.rs:1003 `setrlimit(Resource::RLIMIT_AS, limit, limit).is_err()`
          → MSG_RLIMIT_AS_FAIL + _exit(126)) FIRES: setrlimit(RLIMIT_AS, 32 MiB) FAILS on macOS, so the
          child aborts before exec. RLIMIT_AS enforcement is itself broken at the syscall level (this
          path PREDATES Phase 68).
      (2) `nono: Sandbox initialization failed: Failed to set socket read timeout: Invalid argument
          (os error 22)` — the PARENT's `sock.set_read_timeout(...)?` at exec_strategy.rs:1381 EINVALs.
          This is the SAME `set_read_timeout` EINVAL I had filed as a separate "Phase 59 IPC / rollback-
          only" bug and marked OUT OF SCOPE. **That was WRONG** — it fires in the core RESL supervised
          path (socketpair, no rollback). set_read_timeout runs whenever `supervisor_sock` is Some
          (1380), i.e. for every supervised run with an IPC socket.
    * P-A (`sleep 3`, no flags) printed NEITHER error — only "Applying sandbox..." then quiet. AMBIGUOUS:
      need to know if nono EXITED at ~3s (basic reaping OK) or HUNG (reaping broken). [ps watcher / `time`
      output still needed from the user.]
  implication: The macOS supervised path has MULTIPLE foundational defects, broader than Phase 68's
    original 2-bug scope (max-processes no-op + timeout wrong-pgrp):
      D1. `set_read_timeout`/SO_RCVTIMEO EINVAL on the AF_UNIX supervisor socket — breaks/aborts
          supervised runs (in-scope, not the rollback-only confound I assumed).
      D2. `setrlimit(RLIMIT_AS, N)` fails in the child for `--memory` — memory enforcement broken.
      D3. the original `--timeout` watchdog + `--max-processes` non-enforcement (Phase 68 targets).
    D1+D2 PREDATE Phase 68. Phase 68's setpgid/NPROC fix sits on top of an already-broken macOS
    supervised foundation. The behavior is state-dependent (echo-hi passes, memory aborts, sleep
    quiet-hangs) — likely a timing/peer-state interaction (e.g. set_read_timeout EINVAL only once the
    child end has closed early). This exceeds a single-defect debug fix.

- timestamp: 2026-06-12 (68-02 host UAT on Oscars-MacBook-Pro, binary 1be54bec — D1/D2 FIXED, D3 NOT)
  checked: the D1/D2 sanity runs + the full `NONO_RESL_HOST_VALIDATED=1` gated suite.
  found:
    * **D1 FIXED** ✅ — `nono run --memory 4g/32m -- echo hi` now prints `hi` and exits; the
      `Failed to set socket read timeout` abort is GONE (platform-gate to Linux worked).
    * **D2 FIXED** ✅ — same runs print `setrlimit(RLIMIT_AS) not enforced on macOS (best-effort);
      continuing` then `hi`; the `_exit(126)` abort is GONE (downgrade worked).
    * **D3 NOT FIXED** ❌ — gated suite still 2 pass / 3 FAIL: `macos_timeout_kills_at_deadline`
      (12s hang), `macos_max_processes_blocks_on_rlimit_nproc` (20s hang), `macos_memory_limit_kills_at_rlimit_as`
      (10s hang). SMOKING GUN line: `WARN parent setpgid(18774, 18774) failed (ESRCH: No such process);
      watchdog will still attempt getpgid`.
  implication: The D3 **double-setpgid is INEFFECTIVE**. The parent's `setpgid(child, child)` (placed
    ~line 1487, after signal-forwarding setup) loses the race: by then the child has already exited
    (ESRCH, seen for instant `echo hi`) or exec'd (EACCES, expected for `sleep`). POSIX forbids the
    parent setpgid'ing a child that has execve'd. So the only effective pgrp set is the child's OWN
    `setpgid(0,0)` — i.e. D3's behavior is UNCHANGED from pre-68-02, which is why timeout/max-proc still
    hang identically. The non-PTY wait loop (`wait_for_child_with_startup_timeout`, exec_strategy.rs
    2086-2105) polls `waitpid(WNOHANG)` + 200ms sleep and returns on child exit — reaping WORKS (P-A
    `sleep 3` exits at 3s). So the hangs = the CHILDREN don't die: `sleep 60` needs the `--timeout`
    watchdog `kill(-child_pgrp, SIGKILL)` which MISSES; max-proc bash + memory python3 also fail to
    exit/get-killed. The real open question is now NARROW: why does the watchdog `kill(-child_pgrp)`
    miss when the child IS its own pgrp leader (via its own setpgid(0,0))? Candidates: (a) the watchdog
    thread never fires / getpgid(child) returns a stale/parent pgrp; (b) the child STOPS via SIGTTIN/
    SIGTTOU (own pgrp ≠ terminal foreground group) and WNOHANG-without-WUNTRACED reports it as StillAlive
    forever; (c) the kill targets the wrong group. A correct D3 fix likely needs a fork→exec SYNC BARRIER
    (pipe: child blocks until the parent has setpgid'd it) OR a different kill strategy — NOT the
    naive double-setpgid. This is a redesign, not a one-liner.

- timestamp: 2026-06-12 (FOCUSED TIMEOUT PROBE — BREAKTHROUGH: enforcement WORKS; tests had a harness bug)
  checked: `NONO_LOG=debug nono run --allow-cwd --timeout 5s --read=... -- sleep 60` on the host.
  found: It WORKED. Output: `WARN parent setpgid(19742,19742) failed (EACCES)` then `[nono] Session
    stopped.` + `Command killed by SIGKILL (exit code 137)`. exit 137 = 128+SIGKILL → the `--timeout`
    watchdog fired at 5s, `kill(-child_pgrp, SIGKILL)` hit `sleep 60`, and nono exited. The parent
    setpgid EACCES is HARMLESS — the child's own `setpgid(0,0)` already made it its own pgrp leader, so
    `getpgid(child)` returned the right group and the kill landed.
  implication: ROOT CAUSE of the GATED-TEST hangs identified, and it is NOT an enforcement bug. The
    `--timeout` watchdog (D3) WORKS on the deployed binary. The tests hang because `run_bounded`
    (resl_nix_macos.rs) spawns nono WITHOUT `--allow-cwd` and WITHOUT a stdin redirect → nono blocks on
    the interactive `Share <cwd>? [y/N]` prompt reading from the harness's inherited stdin (never
    answered under `cargo test`), so the CHILD IS NEVER SPAWNED and the run stalls to the bound. The
    passing `macos_no_warnings_on_resource_flags` test uses `.output()` (stdin=/dev/null → prompt EOF →
    auto-resolve → child runs), which is why it passed while the `run_bounded` tests hung. This also
    retro-explains the entire saga's "non-firing" signal on a real host. FIX: add `.stdin(Stdio::null())`
    to `run_bounded` (mirrors `.output()`); enforcement is then actually exercised. NOTE: D3's
    parent-side double-setpgid is effectively a no-op (always EACCES/ESRCH) but ALSO harmless — the child
    setpgid(0,0) carries it. Whether to keep or remove the parent setpgid is a cleanup question, not a
    correctness one.

## Eliminated

- hypothesis: H1 — the modified ForkResult::Child arm / setrlimit / setpgid /
    watchdog sites are not on the path a default `nono run --timeout`/`--max-processes`
    with static `--read` grants takes (structural no-op).
  evidence: select_exec_strategy always returns Supervised; the macOS supervised
    raw-fork path contains all three modified sites; gating conditions
    (resource_limits non-empty, timeout_deadline Some) are satisfied by the test
    invocations; timeout/max_processes flow into the right fields. The fix code IS
    executed at runtime. (See Evidence entries 1-2.)
  timestamp: 2026-06-12
