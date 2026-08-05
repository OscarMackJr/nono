---
phase: 112-security-residual-sync
plan: 06
subsystem: infra
tags: [upstream-sync, security-hardening, seccomp, supervisor, ptrace, orphan-reaping, cross-target-verify]

# Dependency graph
requires:
  - phase: 112-security-residual-sync
    provides: "112-02's SEC-03 changes to exec_strategy.rs (ExecConfig.proc_comm_notify field, extended 3-arg linux_child_requires_dumpable predicate) — this plan's gate reuses that extended signature, not the 2-arg one cited in the plan's own <interfaces> section"
  - phase: 112-security-residual-sync
    provides: "112-04's changes to tests/socket_access_run.rs (9840a16f-tightened af_unix_mediation_pathname_allows_connect_to_listed_socket test) — this plan appends a new test after it in the same file"
provides:
  - "PR_SET_CHILD_SUBREAPER pre-fork parent-side setup in execute_supervised, gated on linux_child_requires_dumpable()"
  - "reap_reparented_orphans(child) free function, ported verbatim from upstream ac5ccd70"
  - "reap_reparented_orphans() wired into the Linux-only run_supervisor_loop, between handle_pty_suspension and the primary waitpid"
  - "New fork-based unit test proving the reap loop's non-primary-vs-primary pid discrimination"
  - "New integration regression test (af_unix_mediation_pathname_allows_orphaned_child_tcp_connect) for the double-forked-orphan TCP connect scenario"
affects: [112-close-out, future-tool-sandbox-absorb-v3.7]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Symbol-level re-verification of a plan's <interfaces> section against the live tree before implementing: the plan's own PR_SET_DUMPABLE call-site example was already stale (112-02 extended linux_child_requires_dumpable from 2 to 3 args on 2026-08-05, in the same phase, before this plan ran) — re-grepped the live call site rather than trusting the plan's cited code snippet verbatim"
    - "Genuine RED/GREEN TDD verified live via cross test: a temporary always-None stub of the new function was compiled and run under `cross test --target x86_64-unknown-linux-gnu` to confirm the new unit test fails for the right reason, before restoring the real implementation and re-running to confirm GREEN"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/tests/socket_access_run.rs

key-decisions:
  - "Disposition re-verified as ADAPT (matching 112-DISPOSITION-TABLE.md's SEC-06 row): upstream's gate predicate `tool_sandbox_runtime.is_some() || seccomp_policy.child_requires_dumpable()` references two symbols absent from this fork (no tool_sandbox_runtime — standing v3.7 divergence per D-01/SEC-09; no SeccompPolicy struct — unabsorbed fa21a004/8a4237f2, #1283). Reused the fork's existing linux_child_requires_dumpable() predicate instead, dropping the tool_sandbox_runtime disjunct entirely."
  - "Amendment to the plan's own <interfaces> section: the plan cited a 2-arg linux_child_requires_dumpable(capability_elevation, network_notify) signature and a 2-arg call-site snippet for the new PR_SET_CHILD_SUBREAPER gate. Live grep of the fork tree found the function already takes 3 args (capability_elevation, network_notify, proc_comm_notify) as of 112-02's same-day SEC-03 absorb (which landed proc_comm_notify wiring before this plan executed). Used the current 3-arg signature and its existing call-site argument expression verbatim (config.capability_elevation, config.seccomp_proxy_fallback || config.af_unix_mediation.is_pathname(), config.proc_comm_notify) for the new gate, so both PR_SET_DUMPABLE and PR_SET_CHILD_SUBREAPER agree on when subreaping/dumpable-preservation is required. This is a plan-interfaces staleness finding, not a disposition change — the ADAPT call itself was correct."
  - "Task 1's tdd=\"true\" test authored as a genuine fork-based unit test (reap_reparented_orphans_reaps_non_primary_and_returns_status_for_primary), not the plan's fallback 'predicate-reuse' framing: this file already has an established fork()-based test pattern (3 existing tests use unsafe { fork() } / ForkResult), and reap_reparented_orphans is a pure, directly-callable free function whose core loop logic (reap non-tracked pid and continue vs. return Some for the tracked pid) can be exercised without any real PR_SET_CHILD_SUBREAPER/reparenting machinery, since waitpid(-1, WNOHANG) reaps any direct child of the calling process regardless of ancestry depth."
  - "Task 2's integration test ported upstream ac5ccd70's own regression test verbatim (double-fork orphan + loopback TCP connect), not a fork-authored alternative: live symbol/schema check confirmed linux.af_unix_mediation and network.block in the JSON profile match this fork's profile schema exactly, so no field-name adaptation was needed beyond the JSON body already matching."

patterns-established:
  - "When cross-referencing a same-phase Wave 3 plan's <interfaces> code snippets against a symbol already touched by an earlier same-day plan in the same wave-set, re-verify by grep/Read rather than trusting the plan's cited line numbers or call-site arity — same-phase plans can land structural changes to shared files between when CONTEXT/interfaces were authored and when a later plan executes."

requirements-completed: [SEC-06]

# Metrics
duration: ~2h30min
completed: 2026-08-05
---

# Phase 112 Plan 06: SEC-06 Seccomp Supervisor-Ancestry Orphan Reaping (Adapted) Summary

**Adapted absorb of upstream ac5ccd70's PR_SET_CHILD_SUBREAPER + reap_reparented_orphans() fix — keeps daemonizing descendants in the supervisor's ptrace-ancestry for seccomp-notify mediation under Yama ptrace_scope=1, gated on the fork's existing linux_child_requires_dumpable() predicate instead of upstream's absent tool_sandbox_runtime/SeccompPolicy symbols.**

## Performance

- **Duration:** ~2h30min (includes symbol-level disposition re-verification, live RED/GREEN TDD cycle via `cross test`, and both mandatory `--all-targets` cross-target clippy gates)
- **Started:** 2026-08-05 (approx.)
- **Completed:** 2026-08-05 (approx.)
- **Tasks:** 2/2 completed
- **Files modified:** 2

## Accomplishments
- Landed `PR_SET_CHILD_SUBREAPER` pre-fork parent-side setup in `execute_supervised`, gated on the fork's existing `linux_child_requires_dumpable()` predicate (same 3-argument call as the existing `PR_SET_DUMPABLE` site, so both gates always agree on when subreaping/dumpable-preservation is required) — closes an availability/mediation-bypass-adjacent gap where a descendant that backgrounds/daemonizes and reparents to pid 1 under Yama `ptrace_scope=1` would have its allowed network/AF_UNIX/openat traffic wrongly denied with EPERM once the supervisor's ancestry-gated `/proc/<pid>/mem` read started failing.
- Ported `reap_reparented_orphans(child) -> Option<WaitStatus>` verbatim from upstream `ac5ccd70`, placed between the two cfg-split `run_supervisor_loop` definitions (matching upstream's own placement) and wired ONLY into the `#[cfg(target_os = "linux")]` arm, between `handle_pty_suspension` and the primary `waitpid(child, WNOHANG)` call — confirmed via direct read of both cfg-gated definitions that the `#[cfg(not(target_os = "linux"))]` arm is untouched.
- Added a genuine fork-based unit test (`reap_reparented_orphans_reaps_non_primary_and_returns_status_for_primary`) proving the reap loop drains a non-tracked orphan (logged, looped past) without mistaking it for the tracked primary child, and returns `Some(status)` only for the primary child's own exit — TDD-verified live: a temporary always-`None` stub was compiled and run via `cross test --target x86_64-unknown-linux-gnu`, confirmed **RED** ("expected primary child's exit status, got None"), then the real implementation was restored and confirmed **GREEN** together with the pre-existing, unmodified `test_linux_child_requires_dumpable_only_for_seccomp_driven_features`.
- Ported upstream's own integration regression test (`af_unix_mediation_pathname_allows_orphaned_child_tcp_connect`) verbatim into `tests/socket_access_run.rs`: a double-forked grandchild (`fork` → `setsid` → `fork`) reparents to pid 1 before an allowed loopback TCP connect; asserts the connect succeeds and that the supervisor's diagnostic stderr contains no "Failed to read sockaddr" ancestry-loss fingerprint.
- Both mandatory cross-target clippy gates ran GREEN with `--all-targets` (linux-gnu via `cross clippy`, exit 0, 42s; apple-darwin via `cargo-zigbuild clippy`, exit 0, 16s — both fast because the underlying build was already warm from the preceding `cross test` runs in this same session). `cargo fmt --all --check` clean. Windows-host `cargo check --workspace --all-targets` clean throughout.

## Task Commits

Each task was committed atomically:

1. **Task 1: PR_SET_CHILD_SUBREAPER pre-fork setup + reap_reparented_orphans()** - `9aaba29e` (feat)
2. **Task 2: Wave-0 orphan-reap integration test + cross-target verification** - `c0b5fab4` (test)

_No separate plan-metadata-only commit; this SUMMARY's own commit serves as the final metadata commit, per the sequential single-repo execution mode for this project._

## Files Created/Modified
- `crates/nono-cli/src/exec_strategy.rs` - `PR_SET_CHILD_SUBREAPER` pre-fork gate in `execute_supervised` (parent side, before `fork()`); `reap_reparented_orphans(child)` ported verbatim; call site wired into the Linux `run_supervisor_loop` only; new fork-based unit test
- `crates/nono-cli/tests/socket_access_run.rs` - `yama_ptrace_scope()` helper + `af_unix_mediation_pathname_allows_orphaned_child_tcp_connect` integration test, ported verbatim from upstream `ac5ccd70`

## Decisions Made
See `key-decisions` in frontmatter for the full list. Summary: implemented SEC-06 as the disposition table's **ADAPT** call (T-112-13/T-112-14 threat mitigations), reusing the fork's existing `linux_child_requires_dumpable()` predicate in place of upstream's absent `tool_sandbox_runtime`/`SeccompPolicy` symbols. Additionally corrected a staleness in the plan's own `<interfaces>` section (2-arg vs. the live 3-arg predicate signature) discovered via live symbol re-verification before implementing, per this plan's explicit disposition-verification warning.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug-adjacent staleness, resolved via live re-verification before implementing] Plan's `<interfaces>` cited a stale 2-arg `linux_child_requires_dumpable` signature**
- **Found during:** Task 1's mandatory symbol-level re-verification (grepping `linux_child_requires_dumpable(` and reading the live `:1594-1598` call site before writing any code), per this plan's own `<disposition_verification_warning>`.
- **Issue:** The plan's `<interfaces>` section (confirmed live "2026-08-05, this planning session") cited `linux_child_requires_dumpable(config.capability_elevation, config.seccomp_proxy_fallback || config.af_unix_mediation.is_pathname())` — a 2-argument call — as the exact gate to reuse for the new `PR_SET_CHILD_SUBREAPER` block. Live grep of the current fork tree found the function signature already extended to 3 arguments (`capability_elevation`, `network_notify`, `proc_comm_notify`) with a matching 3-argument call site, landed by `112-02`'s SEC-03 absorb (commit `85c88cd8`) earlier the same day, before this plan (`112-06`) executed. The plan text itself anticipated line-number drift from `112-02` ("Line numbers have shifted — plan 112-02 already edited this file. Re-locate by symbol, not by line number.") but its cited code snippet's arity had also drifted, which the disposition-verification warning's own guidance ("verify at SYMBOL level") was written to catch.
- **Fix:** Used the live, current 3-argument `linux_child_requires_dumpable(config.capability_elevation, config.seccomp_proxy_fallback || config.af_unix_mediation.is_pathname(), config.proc_comm_notify)` call for the new `PR_SET_CHILD_SUBREAPER` gate — identical to the existing `PR_SET_DUMPABLE` site's call, satisfying the plan's own must_haves truth that "both call sites must agree on when subreaping/dumpable-preservation is required."
- **Files modified:** `crates/nono-cli/src/exec_strategy.rs`
- **Verification:** Both cross-target clippy gates GREEN (would have caught an arity mismatch as a compile error); `cross test` confirms the pre-existing 3-arg predicate test passes unmodified.
- **Committed in:** `9aaba29e`

---

**Total deviations:** 1 auto-fixed (plan-interfaces staleness, caught before any code was written — not a runtime bug)
**Impact on plan:** No change to WHAT was delivered — every `must_haves` truth in the plan frontmatter is satisfied, including the literal requirement that both dumpable/subreaper gates share identical arguments. The deviation is a plan-documentation staleness finding (same-phase sibling plan landed a same-day signature change), not a security or correctness gap.

## Issues Encountered

**python3 not available in the `cross`-rs Linux container image used for local verification.** All 3 tests in `tests/socket_access_run.rs` that depend on `python3` (the 2 pre-existing `af_unix_mediation_pathname_*` tests plus the new orphan-reap test added by this plan) short-circuit on their existing `if !python3_available() { eprintln!("skipping..."); return; }` guard and report `ok` trivially (confirmed via `--nocapture`: all 3 print "skipping: python3 not available"). This is a pre-existing environment limitation of the whole test file — not a regression introduced by this plan — and is within this phase's D-06 verification-depth bound ("cross test + both cross-target clippy gates on the affected modules; no live-kernel UAT checkpoint"). The new test's structural correctness (compiles, matches the fork's profile schema, wired to the real CLI/`run_nono` pipeline, ported verbatim from upstream's own regression test for this exact fix) is confirmed; full behavioral exercise (proving the orphan's TCP connect is actually authorized post-fix) requires a python3-equipped Linux host or a CI lane where python3 is present. Flagging for whoever verifies this phase's close-out or runs live CI on the head SHA.

## User Setup Required

None - no external service configuration required. No new dependencies (`Cargo.toml` unchanged).

## Next Phase Readiness

- **SEC-06 is functionally complete** per this plan's adapted implementation; `REQUIREMENTS.md`'s SEC-06 checkbox should be marked complete by this plan.
- No blockers for phase 112 close-out. This was the last Wave 3 plan touching `exec_strategy.rs`/`tests/socket_access_run.rs` per the phase's dependency graph (`depends_on: ["112-02", "112-04"]`); both upstream dependencies were confirmed landed and read before implementing.
- **Flag for the phase verifier / next live-CI run:** the python3-dependent test skip noted under "Issues Encountered" means `af_unix_mediation_pathname_allows_orphaned_child_tcp_connect`'s actual behavioral assertions (foreground/orphan TCP connect success, absence of the "Failed to read sockaddr" fingerprint) have not been exercised end-to-end in this local verification pass. Recommend confirming on a live GH Actions Linux lane (which does have python3) before treating SEC-06's regression coverage as fully proven, consistent with this same limitation already existing for the file's 2 pre-existing python3-dependent tests.

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*

## Self-Check: PASSED

Created file `112-06-SUMMARY.md` confirmed present on disk; task commits `9aaba29e`
(feat, Task 1), `c0b5fab4` (test, Task 2), and `63193109` (this summary) confirmed
present in `git log`. Both mandatory cross-target clippy gates (`--all-targets` form)
and `cargo fmt --all --check` re-confirmed GREEN before this summary was written.
