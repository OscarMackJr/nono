# Phase 68: macOS Resource-Limit Enforcement Fix - Context

**Gathered:** 2026-06-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Make `nono run --timeout <D>` and `nono run --max-processes <N>` deliver **real,
host-validated enforcement on a real macOS host**. These are nono's own
cross-platform supervisor/`setrlimit` mechanisms; their non-firing on macOS
(surfaced as the Phase 65 gate-65-A "A5" finding, REQ-RESL-NIX-03 defect) is a
nono bug to fix — NOT a test-gate adjustment and NOT an OS limitation.

Two distinct bugs are in scope:

1. **`--max-processes` is a silent no-op.** `supervisor_macos.rs` (the
   `install_pre_exec` closure) never calls `setrlimit(RLIMIT_NPROC, …)` — it
   only logs a `tracing::warn!` ("RLIMIT_NPROC unavailable in nix v0.31's macOS
   subset"). Must actually apply the limit.
2. **`--timeout` watchdog fires but the kill misses.** Default strategy is
   Supervised, so `spawn_macos_timeout_watchdog` IS reached and does
   `kill(-getpgid(child), SIGKILL)`. On a real host nono never exited → the
   group-kill is targeting the wrong / a shared process group (the child is not
   reliably its own group leader), or the supervisor `wait()` doesn't return
   after the kill.

**Out of scope:** `--cpu-percent` (correctly rejected at clap parse on macOS);
RSS-based memory enforcement (RLIMIT_AS-vs-RSS gap is documented/accepted);
Linux and Windows resl paths; any test-only re-gating as a substitute for the
real fix.
</domain>

<decisions>
## Implementation Decisions

### `--max-processes` semantics on macOS (RLIMIT_NPROC)
- **D-01:** Use **baseline + N** bounding. The **parent** reads the current
  per-UID process count *before* fork, computes `RLIMIT_NPROC = current_count + N`,
  and passes the precomputed integer into the `pre_exec` closure. This matches
  the Linux `pids.max` intent (the agent gets N *additional* processes) and makes
  `macos_max_processes_blocks_on_rlimit_nproc` (`--max-processes 5` → the 6th+
  fork in the agent tree fails EAGAIN) meaningful rather than passing for the
  wrong reason.
- **D-02:** RLIMIT_NPROC on macOS is **not exposed by nix v0.31's macOS subset** —
  apply it via raw **`libc::setrlimit(libc::RLIMIT_NPROC, …)`** inside the
  `pre_exec` hook. Only async-signal-safe operations in the closure (the count is
  precomputed in the parent and captured by value as a `Copy` integer — no
  allocation, no locks, no `format!`). Follow the existing async-signal-safety
  posture already documented for `RLIMIT_AS` in `supervisor_macos.rs`.
- **D-03:** **Document the divergence** in the macOS module + user docs:
  RLIMIT_NPROC counts ALL of the UID's processes (not just descendants like Linux
  `pids.max`), so the bound is UID-wide and inherently racy — unrelated processes
  the user spawns between the parent's count and the child's `exec` shrink the
  agent's effective budget. This is accepted behavior, not a bug.

### `--timeout` watchdog fix
- **D-04:** **Defensive rewrite, no separate `/gsd:debug` cycle.** Root cause is
  assumed to be a shared/incorrect process group. Place the child in its **own
  process group** (`setpgid(0, 0)` in the child `pre_exec`, equivalently a clean
  session/group), and have the watchdog kill **that specific group**. Then
  confirm the supervisor `wait()` returns and nono exits promptly after the kill.
- **D-05:** Fix must cover **both the PTY and non-PTY supervised paths** (verify
  signal-forwarding / process-group setup is consistent across both — do not
  break the existing `setup_signal_forwarding` behavior).
- **D-06:** Keep the existing **WR-04 safety**: no PID fallback on `getpgid`
  failure (skip rather than risk a wrong-pgrp kill under PID reuse). The fix is to
  make the child's own group *exist deterministically*, so `getpgid`/the known
  child pgrp is reliable rather than racy.

### Fail mode
- **D-07:** **Fail-closed.** When a limit cannot be applied — baseline UID count
  can't be computed (parent side) or `setrlimit` returns an error (child
  `pre_exec` side) — **abort the run with a clear error** instead of launching an
  unbounded child. This replaces the current silent-warn behavior, which is
  precisely the defect this phase fixes. (`pre_exec` returning `Err` already
  aborts the spawn; parent-side computation failures must `return Err(...)` too.)
  Consistent with the project "Fail Secure" core principle.

### Validation
- **D-08:** Both gated tests — `macos_timeout_kills_at_deadline` and
  `macos_max_processes_blocks_on_rlimit_nproc` — must **PASS on a real macOS host
  (`NONO_RESL_HOST_VALIDATED=1`)**. The host is available (`Oscars-MacBook-Pro`,
  Apple Silicon — the same host that surfaced A5). Tests stay env-gated OFF the CI
  runner (they hang there); CI keeps only the build/clippy/skip-path green.
- **D-09:** (bonus, no new requirement) Add **one lightweight `--memory` /
  RLIMIT_AS live assertion** to the real-host validation while the host UAT is
  running, to catch any third silent gap. Keep it secondary — do NOT expand the
  roadmap scope or add a requirement; RESL-MAC-01/02 remain the only gating reqs.
- **D-10:** The fix touches cfg-gated Unix code → **cross-target clippy (Linux +
  macOS)** is mandatory per `.planning/templates/cross-target-verify-checklist.md`,
  and the **macOS CI build leg must stay green**. Windows dev host cannot
  cross-compile (ring/aws-lc-sys C-toolchain) → CI is the load-bearing signal;
  mark the cross-target REQ PARTIAL/deferred-to-CI if the cross-toolchain is
  unavailable on the dev host.

### Claude's Discretion
- Exact mechanism for reading the per-UID process count in the parent (e.g.
  `sysctl` KERN_PROC / `proc_listpids` / counting) — researcher/planner picks the
  most robust async-safe-friendly approach; the only constraint is it runs in the
  **parent before fork** and yields a single integer.
- Precise error type/wording for the fail-closed aborts (use `NonoError`
  variants consistent with the existing resl error surface).
- Whether `setpgid` is done via `pre_exec` vs. an existing post-fork hook —
  whichever integrates cleanly with the current supervised fork flow.

### Folded Todos
- **`macos-resl-enforcement-broken`** (`.planning/todos/pending/20260611-macos-resl-enforcement-broken.md`)
  — this todo IS the source of this phase (Phase 65 gate-65-A A5 finding). Folded:
  its acceptance criteria (both gated tests pass on a real macOS host with
  `NONO_RESL_HOST_VALIDATED=1`) are this phase's success criteria. Remove/close
  the todo at phase completion.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/REQUIREMENTS.md` § macOS Resource Limits (RESL-MAC) — RESL-MAC-01 (timeout) + RESL-MAC-02 (max-processes); the gating acceptance criteria.
- `.planning/ROADMAP.md` § Phase 68 — goal, 4 success criteria, host gate.

### The defect / prior validation
- `.planning/todos/pending/20260611-macos-resl-enforcement-broken.md` — the A5 finding, problem statement, fix hints, acceptance.
- `.planning/phases/65-minifilter-adr-macos-live-re-validation/65-HUMAN-UAT.md` — gate-65-A live UAT where A5 was discovered (A1–A4 Seatbelt re-validated clean; A5 = these two failures).

### Implementation surfaces (existing code to modify)
- `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` — `MacosResourceLimits` (`install_pre_exec` — the no-op RLIMIT_NPROC `warn!` to replace; RLIMIT_AS precedent for async-signal-safe `setrlimit`) + `spawn_macos_timeout_watchdog` (the `kill(-pgrp, SIGKILL)` watchdog).
- `crates/nono-cli/src/exec_strategy.rs` — `run_supervised` (the macOS watchdog spawn site ~line 1407, `getpgid(child)` WR-04 logic; `apply_resource_limits_unix` dispatch ~line 90; process-group / signal-forwarding setup).
- `crates/nono-cli/tests/resl_nix_macos.rs` — the two gated tests + `host_enforcement_validated()` gate + `run_bounded` harness; where the bonus `--memory` assertion lands.

### Cross-platform precedent (do NOT modify; reference for parity)
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — Linux `cgroup` `pids.max` + `spawn_linux_timeout_watchdog` (`cgroup.kill`) — the enforcement-intent the macOS path must approximate.

### Process & verification gates
- `.planning/templates/cross-target-verify-checklist.md` — mandatory cross-target clippy protocol (D-10).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `MacosResourceLimits::install_pre_exec` — already installs an async-signal-safe
  `pre_exec` closure for `RLIMIT_AS`; extend the same closure for RLIMIT_NPROC
  (raw libc) + the child `setpgid`. The SAFETY doc + `Copy`-capture discipline are
  already established here.
- `spawn_macos_timeout_watchdog(deadline, child_pgrp)` — watchdog plumbing exists
  and is wired into `run_supervised`; the bug is the *target group*, not the
  watchdog thread/deadline logic.
- `run_bounded` test harness — already bounds the gated tests so a non-firing fix
  fails fast (~12s/20s) rather than hanging; reuse for the bonus `--memory` check.

### Established Patterns
- Default strategy = **Supervised** (`ExecStrategy::default() == Supervised`,
  asserted in `exec_strategy.rs` tests) → `nono run --timeout`/`--max-processes`
  reaches `run_supervised`; no strategy-selection change needed.
- WR-04: never PID-fallback on `getpgid` failure (avoid wrong-pgrp kill under PID
  reuse) — preserve this; make the child's own group deterministic instead.
- Fail-secure on resl config failure is the house style (mirrors Linux
  `UnsupportedKernelFeature` fail-closed) → D-07 aligns.

### Integration Points
- Parent-side per-UID count → captured-by-value integer → child `pre_exec`
  `libc::setrlimit(RLIMIT_NPROC, count + N)`.
- Child `setpgid(0,0)` in `pre_exec` → watchdog `kill(-child_pgrp, SIGKILL)` now
  targets exactly the agent tree → supervisor `wait()` returns → nono exits.
- `setpgid` must not break `setup_signal_forwarding` (verify forwarding still
  reaches the child's new group on both PTY and non-PTY paths).

</code_context>

<specifics>
## Specific Ideas

- The live failures (the bar to clear): on `Oscars-MacBook-Pro` with
  `NONO_RESL_HOST_VALIDATED=1`, `--timeout 5s` did NOT SIGKILL `sleep 60` at ~5s
  (nono didn't exit within 12s), and `--max-processes 5` did NOT make `fork()`
  fail (nono didn't exit within 20s). Both must now PASS.
- `--max-processes 5` semantics under baseline+N: the agent tree gets 5 procs
  beyond the parent's baseline; the test's `for i in 1..20; do sleep 5 & done`
  loop must hit EAGAIN.

</specifics>

<deferred>
## Deferred Ideas

- **RSS-based `--memory` enforcement** — RLIMIT_AS bounds virtual address space,
  not resident set; the gap is documented/accepted (REQ-RESL-NIX-03). Polling-based
  RSS enforcement has racy bypass windows — out of scope, not planned.
- **Mach `task_policy_set`-based per-process limits** — the macOS module notes a
  possible future true-equivalent path; not pursued here.

### Reviewed Todos (not folded)
- **`msi-vcredist-prereq`** — belongs to Phase 67 (Clean-Host Windows Install), not macOS.
- **`poc-cert-broker-clean-host`** — belongs to Phase 67 (interim broker-trust path), not macOS.

</deferred>

---

*Phase: 68-macos-resl-enforcement-fix*
*Context gathered: 2026-06-12*
