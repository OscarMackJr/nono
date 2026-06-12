# Phase 68: macOS Resource-Limit Enforcement Fix - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-12
**Phase:** 68-macos-resl-enforcement-fix
**Areas discussed:** max-processes semantics, timeout watchdog fix, --memory validation scope, fail-closed vs best-effort

---

## max-processes semantics (RLIMIT_NPROC, UID-wide on macOS)

| Option | Description | Selected |
|--------|-------------|----------|
| baseline + N | Parent reads current per-UID proc count before fork, sets RLIMIT_NPROC = count + N, passes precomputed int into async-signal-safe pre_exec. Matches Linux pids.max intent; makes the test meaningful. Document UID-wide + racy. | ✓ |
| literal absolute cap N | setrlimit(RLIMIT_NPROC, N) verbatim. With hundreds of UID procs alive, N=5 means agent can't fork at all; test passes for wrong reason. | |
| baseline + N, clamp/headroom | baseline+N with safety headroom + hard-limit clamp; more robust under load, looser enforcement. | |

**User's choice:** baseline + N (recommended)
**Notes:** Parent-side count → captured-by-value integer into the pre_exec closure. RLIMIT_NPROC applied via raw libc (nix v0.31 doesn't expose it on macOS). Divergence (UID-wide, racy) documented, not treated as a bug.

---

## timeout watchdog fix (non-firing kill — wrong/shared process group)

| Option | Description | Selected |
|--------|-------------|----------|
| Defensive rewrite + host-verify | Put child in own process group (setpgid(0,0)/setsid), watchdog kills that group, confirm supervisor wait() returns. Covers PTY + non-PTY. Skip separate debug cycle. | ✓ |
| Full /gsd:debug session first | Dedicated debug investigation on the real Mac before any fix. Higher confidence, more overhead. | |
| Diagnostic step inside research | No separate debug command; researcher adds on-host instrumentation to confirm cause before the setpgid fix. | |

**User's choice:** Defensive rewrite + host-verify (recommended)
**Notes:** Root cause assumed = shared/incorrect pgrp. Preserve WR-04 (no PID fallback) by making the child's own group deterministic. Must not break existing signal forwarding.

---

## --memory / RLIMIT_AS validation scope

| Option | Description | Selected |
|--------|-------------|----------|
| Add as bonus assertion, no new req | Add one cheap --memory/RLIMIT_AS live check during the host UAT to catch a third silent gap; keep secondary, no roadmap-scope expansion. | ✓ |
| Strictly out of scope | Keep phase exactly to --timeout + --max-processes per roadmap. | |

**User's choice:** Add as bonus assertion, no new req (recommended)
**Notes:** RESL-MAC-01/02 remain the only gating requirements; the --memory check is informational only.

---

## fail-closed vs best-effort

| Option | Description | Selected |
|--------|-------------|----------|
| Fail-closed: refuse to launch | Abort with a clear error if baseline can't be computed or setrlimit fails, rather than launch unbounded. Matches "Fail Secure". Current silent-warn is the bug. | ✓ |
| Best-effort + warning | Launch even if the limit didn't apply, with a prominent warning. More permissive; risks the same false enforcement. | |

**User's choice:** Fail-closed: refuse to launch (recommended)
**Notes:** pre_exec Err already aborts spawn; parent-side computation failures must also return Err. Consistent with the security-tool posture.

---

## Claude's Discretion

- Exact per-UID process-count mechanism in the parent (sysctl KERN_PROC / proc_listpids / counting) — any robust approach that runs before fork and yields one integer.
- Error type/wording for fail-closed aborts (use existing NonoError resl variants).
- setpgid via pre_exec vs an existing post-fork hook — whichever integrates cleanly.

## Deferred Ideas

- RSS-based --memory enforcement (RLIMIT_AS-vs-RSS gap documented/accepted; polling is racy) — not planned.
- Mach task_policy_set per-process limits — possible future true-equivalent; not pursued.
- Reviewed-but-not-folded todos: `msi-vcredist-prereq` + `poc-cert-broker-clean-host` belong to Phase 67 (Clean-Host Windows), not macOS.
