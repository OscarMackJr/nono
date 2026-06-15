---
phase: 74-persistent-multi-tenant-daemon
plan: "07"
subsystem: infra
tags: [windows, named-pipe, appcontainer, tokio, sddl, daemon, multi-tenant]

requires:
  - phase: 74-persistent-multi-tenant-daemon/74-04
    provides: accept_loop.rs (capability pipe accept loop), launch.rs (AppContainer spawn)
  - phase: 74-persistent-multi-tenant-daemon/74-05
    provides: agent_cli.rs operator verbs (daemon_start/status/stop/agent_launch/agent_list stubs)

provides:
  - control_loop.rs: tokio async named-pipe server on \\.\pipe\nono-agentd-control
    with Medium-IL SDDL DACL (T-74-07-01), per-connection tokio::spawn, biased select!
  - is_known_profile() embedded-policy validator in agent_daemon/mod.rs (T-74-07-03)
  - launch_agent pub(crate) re-export in launch.rs for cross-module access
  - daemon_start dev-layout: DETACHED_PROCESS + CREATE_NEW_PROCESS_GROUP background spawn
  - daemon_status: control-pipe probe (List request) with sc query fallback
  - daemon_stop: control-pipe Shutdown request; double notify_one() for both concurrent loops
  - nono-agentd.rs: tokio::join!(accept_loop, control_loop) concurrent execution
  - 74-HUMAN-UAT.md SC1 section: removed stub fallback, documented real end-to-end flow

affects:
  - 74-HUMAN-UAT (SC1 gate: nono daemon start → agent launch → list → stop)
  - future plans that extend control protocol (new ControlRequest variants)

tech-stack:
  added: []
  patterns:
    - "Dual-loop daemon shutdown: notify_one() x2 when multiple loops share one Arc<Notify>"
    - "Control pipe SDDL for operator-only (Medium+ IL): D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)"
    - "Dev-layout background daemon: DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP + mem::forget(child)"
    - "Control pipe probe for status/stop: send List/Shutdown over \\.\pipe\\nono-agentd-control"
    - "is_known_profile via include_str! embedded policy JSON — self-contained in agent_daemon/mod.rs"

key-files:
  created:
    - crates/nono-cli/src/agent_daemon/control_loop.rs
  modified:
    - crates/nono-cli/src/agent_daemon/mod.rs
    - crates/nono-cli/src/agent_daemon/launch.rs
    - crates/nono-cli/src/bin/nono-agentd.rs
    - crates/nono-cli/src/agent_cli.rs
    - .planning/phases/74-persistent-multi-tenant-daemon/74-HUMAN-UAT.md

key-decisions:
  - "Double notify_one() for dual-loop shutdown: Notify::notify_one() wakes exactly ONE waiter; with two concurrent loops both waiting on the same Arc<Notify>, two calls are required"
  - "SDDL Medium-IL mandatory label for control pipe: D:P(...)S:(ML;;NW;;;ME) admits only interactive-user (Medium+) and denies AppContainer (Low IL) — T-74-07-01"
  - "is_known_profile() self-contained in agent_daemon/mod.rs via include_str! to avoid crate::profile dependency (nono-agentd cannot access crate::profile)"
  - "dev-layout daemon_stop: send Shutdown over control pipe rather than sc stop (no SCM registration needed)"
  - "launch_agent pub(crate) re-export in launch.rs: control_loop.rs calls into launch.rs across module boundary"

patterns-established:
  - "Dual-loop Notify shutdown: call notify_one() N times (N = number of concurrent loops) not once"
  - "Control pipe SDDL constant in control_loop.rs, asserted in test to contain 'ME' and not 'LW'"
  - "Dev-layout background daemon: spawn with creation_flags, std::mem::forget(child) to detach"

requirements-completed: [DMON-01, DMON-02, DMON-03]

duration: ~90min (across two sessions)
completed: 2026-06-15
---

# Phase 74 Plan 07: SC1 Control-Plane Wiring Summary

**tokio async control-pipe server on \\.\pipe\nono-agentd-control (Medium-IL SDDL) wired concurrently with the capability-pipe accept loop, enabling `nono daemon start/status/stop` and `nono agent launch/list` to work end-to-end without SCM registration**

## Performance

- **Duration:** ~90 min (across two sessions)
- **Started:** 2026-06-14T00:00:00Z
- **Completed:** 2026-06-15T00:00:00Z
- **Tasks:** 2 (+ 1 deviation fix)
- **Files modified:** 6

## Accomplishments

- Implemented `control_loop.rs` with Medium-IL SDDL DACL, per-connection `tokio::spawn`, biased `select!` on shutdown signal, and wire framing matching `socket_windows.rs`
- Wired `is_known_profile()` validation (T-74-07-03: unknown profile → fail-secure error, never launch) and `ControlRequest::Launch/List/Shutdown` dispatch
- Fixed `agent_cli.rs` `daemon_start` (DETACHED_PROCESS background spawn), `daemon_status` (control-pipe probe), and `daemon_stop` (Shutdown request over pipe)
- Fixed dual-loop shutdown: `notify_one()` called twice at every shutdown origin (SCM Stop, Ctrl-C handler, Shutdown request) so both `run_accept_loop` and `run_control_loop` receive the signal
- Dev-validated SC1 end-to-end on Win11 build 26200: two concurrent notepad.exe agents in distinct AppContainers (distinct SIDs), `daemon stop` exits cleanly, `daemon status` shows NOT RUNNING

## Task Commits

1. **Task 1: control_loop.rs** - `00214758` (feat)
2. **Task 2: dev-layout daemon start/status/stop** - `9b995e80` (feat)
3. **Deviation fix: double notify_one() for dual-loop shutdown** - `2ee68b71` (fix)
4. **Task 2D: UAT update, remove stub caveat** - `90d9371d` (docs)

## Files Created/Modified

- `crates/nono-cli/src/agent_daemon/control_loop.rs` — NEW: tokio named-pipe server for \\.\pipe\nono-agentd-control; CONTROL_PIPE_SDDL (Medium IL); ControlRequest enum (Launch/List/Shutdown); handle_control_connection; write_framed_response; handle_list_testable test helper
- `crates/nono-cli/src/agent_daemon/mod.rs` — Added `pub(crate) mod control_loop;`, EMBEDDED_POLICY_JSON constant, `is_known_profile()` function
- `crates/nono-cli/src/agent_daemon/launch.rs` — Added `pub(crate) use windows_impl::launch_agent;` re-export
- `crates/nono-cli/src/bin/nono-agentd.rs` — Changed both run_service and run_foreground_mode to `tokio::join!(run_accept_loop, run_control_loop)`; doubled notify_one() in SCM Stop handler and Ctrl-C handler
- `crates/nono-cli/src/agent_cli.rs` — Rewrote daemon_start (dev-layout background spawn), daemon_status (control-pipe probe + sc query fallback), daemon_stop (Shutdown request + sc stop fallback)
- `.planning/phases/74-persistent-multi-tenant-daemon/74-HUMAN-UAT.md` — SC1 section rewritten: removed stub caveat; added dev-validation evidence; added SC1 Step 5 (daemon stop); fixed Common Failure Modes

## Decisions Made

1. **Double notify_one() for dual-loop shutdown:** `tokio::Notify::notify_one()` stores exactly one permit and wakes exactly one waiter. With `tokio::join!(accept_loop, control_loop)` both loops park on the same `Arc<Notify>`. A single `notify_one()` only woke one loop, leaving the process running after `daemon stop`. Fix: call `notify_one()` twice at every shutdown origin (SCM Stop, Ctrl-C, Shutdown wire request).

2. **SDDL Medium-IL mandatory label for control pipe:** `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)` — SACL `S:(ML;;NW;;;ME)` sets Medium IL as the minimum integrity level. AppContainer processes (Low IL) are structurally denied without an explicit ACE check. The DACL `OW` ACE is object-owner rather than a specific user SID, keeping the SDDL robust across user accounts.

3. **is_known_profile() self-contained:** The `nono-agentd` binary pulls in `agent_daemon` via `#[path = "../agent_daemon/mod.rs"]` and cannot access `crate::profile`. Solution: embed the policy JSON via `include_str!(concat!(env!("OUT_DIR"), "/policy.json"))` directly in `mod.rs`, matching the pattern in `config/embedded.rs`.

4. **Dev-layout daemon stop via control pipe:** Rather than requiring SCM registration for `daemon stop`, the Shutdown ControlRequest allows stopping a dev-layout background daemon via the same pipe the daemon is serving. Falls back to `sc stop nono-agentd` if the control pipe probe fails (SCM-managed daemon).

5. **notepad.exe as SC1 proof agent:** Most CLI processes (ping, timeout, cmd) exit immediately under AppContainer due to blocked console/network access. notepad.exe (GUI) survives because it doesn't require those resources, making it a reliable stand-in for long-running agent processes in dev-layout tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Double notify_one() for dual-loop shutdown**
- **Found during:** Win11 end-to-end validation (daemon_stop test)
- **Issue:** After `nono daemon stop` sent the Shutdown request, the daemon printed "nono-agentd: shutdown initiated." but remained RUNNING 4 seconds later. Root cause: `tokio::Notify::notify_one()` wakes exactly ONE waiter; with both `run_accept_loop` and `run_control_loop` parked on the same `Arc<Notify>`, only one loop received the shutdown signal and broke; the other kept running.
- **Fix:** Changed every shutdown notification site to call `notify_one()` twice: the Shutdown ControlRequest handler in `control_loop.rs`, the SCM `ServiceControl::Stop` handler in `nono-agentd.rs`, and the Ctrl-C handler in `nono-agentd.rs`.
- **Files modified:** `crates/nono-cli/src/agent_daemon/control_loop.rs`, `crates/nono-cli/src/bin/nono-agentd.rs`
- **Commit:** `2ee68b71`

---

**Total deviations:** 1 auto-fixed (Rule 1 — behavioral bug in shutdown path)
**Impact on plan:** Essential correctness fix; no scope creep. The dual-loop architecture introduced in this plan required a corresponding dual-notify at every shutdown origin.

## Issues Encountered

- `launch_agent` function was in `windows_impl` (private module) in `launch.rs`; `control_loop.rs` couldn't call it directly. Fixed by adding `pub(crate) use windows_impl::launch_agent;` re-export.
- `child.id()` returns `u32` (not `Option<u32>`) — `unwrap_or` usage was incorrect. Fixed by removing intermediate variable.
- `notepad.exe` (bare name) fails in detached daemon process (no system PATH). UAT commands must use `C:\Windows\System32\notepad.exe` full path.

## Known Stubs

None. The control pipe is fully wired. The SC1 "Known stub caveat" from 74-05-SUMMARY has been removed.

> Note: `nono agent list` displays the AppContainer profile moniker
> (`nono.session.<uuid>`) rather than the user-facing profile name (`aider`). This is
> cosmetic — the moniker IS the per-agent AppContainer profile name. A future plan could
> track the original profile name separately in `AgentTenant` and surface it in the list
> output.

## Threat Surface Scan

No new network endpoints or auth paths introduced. Control pipe is operator-local (SDDL
Medium IL mandatory label; IPC only, no network socket). No schema changes at trust
boundaries.

T-74-07-01 (Medium-IL SDDL gate) satisfied — verified by SDDL constant in `control_loop.rs`
and asserted in unit test (`CONTROL_PIPE_SDDL.contains("ME")` + `!CONTROL_PIPE_SDDL.contains("LW")`).

T-74-07-03 (unknown profile fail-secure) satisfied — `is_known_profile()` returns `false`
on parse error (fail-closed) and `false` for any name not in the `"profiles"` object.

SC4 (no escape hatch) satisfied — `ControlRequest` has no variant that mutates an existing
`AgentTenant`'s `CapabilitySet`. The `Launch` variant creates a NEW agent; the existing tenants'
capabilities are immutable.

## Cross-Target Clippy Verification

Windows-host `cargo build --bin nono-agentd --bin nono --release` PASS (final build 2026-06-15).

Cross-target (Linux/macOS) clippy: PARTIAL — `control_loop.rs` and `agent_cli.rs` daemon paths
are behind `#[cfg(target_os = "windows")]`; Linux/macOS cross-toolchain not installed on this
host. Deferred to CI per `.planning/templates/cross-target-verify-checklist.md`.
The non-Windows stub `fn main()` in `nono-agentd.rs` compiles on all targets; control logic is
gated; no new cfg-ungated code introduced.

## Next Phase Readiness

SC1 (two concurrent confined agents) is end-to-end verified in dev-layout. The Phase 74 human
gate (74-06-HUMAN-UAT.md SC1 through SC5 checklist) is unblocked — SC1 no longer has a stub
fallback. Remaining SC2/SC3/SC4/SC5 checks are integration test and SCM-install paths not
requiring further code changes.

For Phase 74 completion: run the full UAT runbook (74-HUMAN-UAT.md) on Win11 with SCM
registration to confirm SC4 (USER_OWN_PROCESS type) and the SC2/SC3 integration tests.

---
*Phase: 74-persistent-multi-tenant-daemon*
*Completed: 2026-06-15*
