---
phase: 75-supplementary-controls-secondary-engines
plan: "07"
subsystem: daemon
tags: [windows, daemon, appcontainer, capability-set, dacl, gap-closure, fail-secure]

# Dependency graph
requires:
  - phase: 74-persistent-multi-tenant-daemon
    provides: launch_agent suspended-spawn sequence (steps 6/6.5/7a/7b/8); AgentTenant; reap path; per-agent AppContainer package SID
  - phase: 75-supplementary-controls-secondary-engines
    plan: "06"
    provides: daemon_start type-50 raw-spawn fallback (needed so `daemon start` reaches RUNNING for the live UAT)

provides:
  - build_daemon_capability_set(profile, resolved_exe, workspace) in mod.rs (Windows-cfg-gated) — real CapabilitySet from named profile
  - per-tenant workspace under %USERPROFILE%\nono-agents\<16-hex-token> (created daemon-side, R-B3 user-owned)
  - local DaemonDaclGuard RAII guard in launch.rs (apply/revert_all/Drop) — no exec_strategy_windows import (module-independence invariant preserved)
  - new launch_agent step 6.6: package-SID DACL grants applied AFTER WFP (6.5) and BEFORE ResumeThread (8); fail-secure TerminateProcess on grant error
  - AgentTenant.dacl_guard field (declared before job_handle/process_handle so revocation precedes job close)
  - resolve_exe_path is now pub(crate); handle_launch resolves bare exe to absolute BEFORE building caps
  - 6 daemon unit tests (in the nono-agentd binary)

affects: [75-05 Wave 3 UAT (SC1 demote now testable against a live long-running agent), Phase 75 UAT SC3]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "DaemonDaclGuard mirrors AppliedDaclGrantsGuard SHAPE but is defined locally (launch.rs) — preserves the launch.rs lines 27-31 module-independence invariant (nono-agentd must not declare exec_strategy_windows)"
    - "DACL grant ordering Pitfall-3: grants AFTER WFP, BEFORE ResumeThread; agent stays SUSPENDED so step-7a registry insert before grants is safe"
    - "Resolve bare exe via resolve_exe_path/SearchPathW BEFORE build_daemon_capability_set — caps + OS spawn boundary use the same absolute path"
    - "path_is_owned_by_current_user gate before every grant; non-owned workspace is fail-secure Err; non-owned ancestor stops the traverse walk (lowbox bypass-traverse)"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/agent_daemon/mod.rs
    - crates/nono-cli/src/agent_daemon/control_loop.rs
    - crates/nono-cli/src/agent_daemon/launch.rs
    - crates/nono-cli/src/agent_daemon/reap.rs
    - crates/nono-cli/src/agent_daemon/accept_loop.rs

requirements: [SUPP-01, SUPP-02, SUPP-03]
gap_closure: true
---

# Phase 75 Plan 07: Capability-less Daemon Launch Fix (GAP-75-B) Summary

## What was built

GAP-75-B: daemon-launched agents were capability-less. `handle_launch` passed an empty
`CapabilitySet::new()` to `launch_agent`, and `launch_agent` never applied package-SID DACL
grants — so a confined agent had zero filesystem access beyond the AppContainer System32
default, killing real engines on startup. This plan wired the daemon launch path to match the
per-invocation `nono run` path:

- **Task 1** — `build_daemon_capability_set` (mod.rs) parses the embedded policy, resolves the
  named profile's `windows_interpreters` via `where`, and builds a real CapabilitySet:
  engine exe dir (Read), interpreter dirs (Read), %SystemRoot%+System32+SysWOW64 (Read,
  the Phase 58 CLR/PE-loader baseline), per-tenant workspace (ReadWrite). `handle_launch`
  derives a per-tenant workspace at `%USERPROFILE%\nono-agents\<16-hex-token>` and creates it
  before spawn. Commit `4fb9551e`.
- **Task 2** — local `DaemonDaclGuard` in launch.rs (apply/revert_all/Drop, LIFO revert),
  new step 6.6 applying package-SID grants (write+traverse on workspace, traverse on read-only
  dirs + owned ancestors) AFTER the WFP gate and BEFORE ResumeThread, with fail-secure
  TerminateProcess on grant error. `AgentTenant` gained a `dacl_guard` field declared before
  the handles so revocation runs before job close. Commit `062f86ad`.

## Deviations / live-UAT findings

Two issues surfaced during the live Win11 UAT (Task 3) and were fixed before approval:

1. **Runbook profile-name error (plan text, not code):** Task 3 used `--profile claude`, which
   does not exist — fail-secure refusal is correct behavior. The valid native-PE Claude engine
   profile is **`claude-code`** (`windows_low_il_broker: true`, no interpreters). Runbook
   corrected to `claude-code`.
2. **Bare-exe → empty-parent crash (real code bug, regression-tested):** `handle_launch` passed
   the bare command name (`claude`) to `build_daemon_capability_set`, where `"claude".parent()`
   is the empty path → `allow_path("")` failed "Path does not exist". `launch_agent` resolved the
   exe via `resolve_exe_path`/SearchPathW, but only *after* caps were built. Fix: made
   `resolve_exe_path` `pub(crate)` and resolved the exe to an absolute path in `handle_launch`
   BEFORE building caps, passing the resolved path to both `build_daemon_capability_set` and
   `launch_agent` (the latter's internal resolve fast-paths on an already-absolute existing path).
   New regression test `resolve_exe_path_bare_name_returns_absolute` covers the bare-name path the
   Task-1 test missed (it used an absolute tempdir fake_exe). Commit `30afd094`.

## Verification

**Test-binary correction:** the plan's verify commands used `cargo test ... --bin nono`, but
agent_daemon tests compile into the **`nono-agentd`** binary (agent_daemon is `#[path]`-included
there per the module-independence invariant). `--bin nono` silently matched 0 daemon tests. Re-run
against `--bin nono-agentd` — all 6 PASS:

- `daemon_caps_non_empty_for_known_profile` — ok
- `daemon_workspace_path_uses_userprofile` — ok
- `daemon_dacl_guard_applies_and_reverts_write_grant` — ok
- `daemon_dacl_guard_mid_loop_failure_reverts_already_applied` — ok (fail-secure: mid-loop revert)
- `daemon_dacl_guard_reap_revokes_traverse_paths` — ok (both write AND traverse revoked)
- `resolve_exe_path_bare_name_returns_absolute` — ok (new regression test)

- `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` → 0 warnings
- `cargo clippy --bin nono-agentd -- -D warnings -D clippy::unwrap_used` → 0 warnings
- `cargo fmt --all -- --check` → 0 diff
- `cargo build --release -p nono-cli` → clean
- Cross-target clippy (Linux/macOS): PARTIAL — all new code is `#[cfg(target_os = "windows")]`-gated;
  deferred to CI per CLAUDE.md cross-target-verify-checklist.

**Live UAT Task 3 (real Win11 host, PowerShell, operator-run 2026-06-16) — APPROVED:**

- daemon start: printed the 75-06 type-50 guard message and reached RUNNING (pid 11484) — GAP-75-A
  also live-verified.
- `nono agent launch --profile claude-code -- claude --version`: launched confined
  (tenant_id=3d229a2f…, sid=S-1-15-2-…, pid 27888); **no DLL-death** → CapabilitySet is real.
- per-tenant workspace created: `C:\Users\OMack\nono-agents\ecf1dfc40c4c4fe8`.
- **write-outside-workspace denied: `Test-Path C:\nono-outside-test.txt` → False** (SECURITY GATE PASS).
- `nono agent list` → `No agents running` → tenant reaped (DaemonDaclGuard dropped → grants revoked).
- daemon stop: clean.

(The confined child's version string did not echo to the console because daemon-launched agents run
detached — stdout is not relayed, unlike an attached `nono run`. Confinement + clean reap are the
load-bearing proof and both passed.)

## Success criteria

1. Daemon-launched claude.exe runs confined, reads its runtime, reaps cleanly (not DLL-killed) — ✅ (live)
2. Write outside per-tenant workspace denied — ✅ (live, Test-Path False)
3. Package-SID DACL grants revoked on reap — ✅ (live reap + unit test)
4. Grant failure before ResumeThread terminates the suspended process (fail-secure) — ✅ (unit test)
5. 3 DACL unit tests + caps tests + regression test PASS; clippy clean; DCO sign-off — ✅
6. DaemonDaclGuard defined locally (module-independence invariant) — ✅
7. SC1 (demote) now testable against a live long-running agent — ✅ (unblocked)

## Carry-forwards

- Phase 75 **Wave 3 plan 75-05** (broader host-gated UAT: SC3 Copilot end-to-end, SC5 nono-ts
  confinedRun, A1 two-agent per-agent WFP isolation) remains incomplete — out of scope for this
  `--gaps-only` run; still needs the operator on a real Win11 host.
- Plan-text fixes to fold back if these plans are ever re-run: profile name `claude-code` (not
  `claude`), and daemon-test verify command `--bin nono-agentd` (not `--bin nono`).
