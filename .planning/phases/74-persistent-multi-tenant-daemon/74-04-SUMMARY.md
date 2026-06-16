---
phase: 74-persistent-multi-tenant-daemon
plan: 04
subsystem: daemon
tags: [windows, appcontainer, named-pipe, ipc, job-object, tokio, impersonation, raii]

# Dependency graph
requires:
  - phase: 74-02
    provides: "AgentRegistry (insert/remove), build_capability_pipe_sddl (pub), authenticate_pipe_client, nono::supervisor re-exports"
  - phase: 74-03
    provides: "DaemonState, AgentTenant RAII, nono-agentd.rs binary skeleton with shutdown plumbing"
provides:
  - "accept_loop.rs — DMON-02 isolation boundary: multi-tenant named pipe accept loop with ImpersonateNamedPipeClient auth and registry SID check"
  - "launch.rs — DMON-01/DMON-03 launch orchestration: fresh AppContainer profile+SID+Job per agent, registry insert, reap task"
  - "nono-agentd.rs wired: Wave 1 placeholder replaced with real run_accept_loop call in both run_service and run_foreground_mode"
affects:
  - 74-05
  - 74-06

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "HANDLE Send-safety via usize cast: cast HANDLE (*mut c_void) to usize before tokio::spawn boundary, cast back inside spawn_blocking — avoids unsafe impl Send on custom wrapper while preserving raw handle semantics"
    - "Inlined Win32 job/AppContainer creation in agent_daemon/launch.rs (no exec_strategy_windows dependency): nono-agentd is a separate Cargo binary; its crate:: root is the binary itself, so exec_strategy_windows (declared in main.rs) is unreachable. Solution: inline CreateJobObjectW + PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES directly."
    - "Locking order: agent_registry first, tenants second — documented in DaemonState module doc; enforced in launch_agent (insert) and reap task (remove)"
    - "Fail-secure cleanup_failed_agent: removes registry entry and tenant on every error path; never silently degrades on partial failure"
    - "build_capability_pipe_sddl(None, None) for base Low-IL accept-loop SDDL (called before tenant SID is known); post-auth SID registry check is the primary isolation gate"

key-files:
  created:
    - crates/nono-cli/src/agent_daemon/accept_loop.rs
    - crates/nono-cli/src/agent_daemon/launch.rs
  modified:
    - crates/nono-cli/src/bin/nono-agentd.rs
    - crates/nono-cli/src/agent_daemon/mod.rs
    - crates/nono-cli/src/agent_daemon/reap.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono/src/supervisor/socket_windows.rs
    - crates/nono/src/supervisor/mod.rs

key-decisions:
  - "A6 resolution (broker trust gate): inlined Win32 API calls in agent_daemon/launch.rs rather than calling exec_strategy_windows::launch functions — the daemon binary cannot reach main.rs module tree"
  - "Reap mechanism: tokio::spawn(spawn_blocking(WaitForSingleObject(INFINITE))) with DuplicateHandle for the reap copy, usize cast for Send-safety"
  - "Authorization gate: kernel-vouched SID from authenticate_pipe_client is the SOLE authz signal; wire-frame session_id is a routing hint only (SC5; T-74-04-01)"
  - "Base pipe SDDL is Low-IL (build_capability_pipe_sddl(None, None)); post-auth registry check provides the tenant isolation — no per-tenant SDDL on the pipe instance itself"
  - "build_capability_pipe_sddl promoted from pub(crate) to pub fn to allow cross-crate call from nono-cli"

patterns-established:
  - "Binary-crate isolation: nono-agentd.rs is a separate Cargo binary; do not assume main.rs module tree is visible from agent_daemon/ — inline Win32 APIs or factor into nono crate"
  - "HANDLE Send-safety: usize cast is the canonical pattern; do NOT add unsafe impl Send on HANDLE newtypes"

requirements-completed:
  - DMON-01
  - DMON-02
  - DMON-03

# Metrics
duration: 90min
completed: 2026-06-15
---

# Phase 74 Plan 04: Active Daemon Sub-Modules — accept_loop + launch Summary

**Multi-tenant named-pipe accept loop with ImpersonateNamedPipeClient SID auth (DMON-02) and per-agent AppContainer+Job launch orchestration with KILL_ON_JOB_CLOSE reap (DMON-01/03)**

## Performance

- **Duration:** ~90 min (across two sessions; compacted between)
- **Started:** 2026-06-14T~20:00:00Z
- **Completed:** 2026-06-15T~02:00:00Z
- **Tasks:** 2
- **Files modified:** 9 (4 prerequisite plumbing + 5 primary)

## Accomplishments

- Implemented `accept_loop.rs`: `create_daemon_capability_pipe_instance` (calls `build_capability_pipe_sddl(None, None)` for base Low-IL SDDL), `run_accept_loop` (biased `tokio::select!` shutdown/connect loop, `tokio::spawn` per connection), `handle_one_connection` (authenticate_pipe_client → registry SID check → query-only capability frame serving); 2 tests cover the STRIDE T-74-04-01 and T-74-04-05 threats
- Implemented `launch.rs`: `launch_agent` (fresh `CreateAppContainerProfile` + `CreateJobObjectW` + `KILL_ON_JOB_CLOSE` + `CREATE_SUSPENDED` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` spawn); `agent_registry` insert BEFORE `tenants` insert (fail-secure locking order); reap task via `DuplicateHandle` + `usize` Send-safety cast + `WaitForSingleObject(INFINITE)` + `spawn_blocking`; `cleanup_failed_agent` on every error path; 3 tests
- Wired `nono-agentd.rs`: `run_service` and `run_foreground_mode` both replaced Wave 1 `todo!()` placeholders with real `agent_daemon::accept_loop::run_accept_loop(daemon_state, shutdown).await`
- All 10 agentd unit tests pass; `cargo build -p nono-cli` and `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` are clean

## Task Commits

Prerequisite commits (prerequisite plumbing for cross-crate and cross-module visibility):

- **Prerequisite A: expose exec_strategy_windows visibility** - `dcf0aad4` (feat)
- **Prerequisite B: expose build_capability_pipe_sddl as pub** - `6277637e` (feat)

Primary task commits:

1. **Task 1: accept loop (accept_loop.rs + mod.rs + reap.rs)** - `ca492378` (feat)
2. **Task 2: launch orchestration + nono-agentd wiring** - `2cefe134` (feat)

**Plan metadata:** (pending — written at end of this SUMMARY creation)

## Files Created/Modified

- `crates/nono-cli/src/agent_daemon/accept_loop.rs` — Multi-tenant pipe accept loop; `create_daemon_capability_pipe_instance`, `run_accept_loop`, `handle_one_connection`; 2 tests
- `crates/nono-cli/src/agent_daemon/launch.rs` — Per-agent launch orchestration; `launch_agent`, `create_agent_job`, `spawn_appcontainer_process_suspended`, `cleanup_failed_agent`, reap task; 3 tests
- `crates/nono-cli/src/bin/nono-agentd.rs` — Wired real `run_accept_loop` in `run_service` and `run_foreground_mode`; removed Wave 1 `todo!()` placeholders
- `crates/nono-cli/src/agent_daemon/mod.rs` — Removed `#[expect(dead_code)]` from `tenants`/`agent_registry` fields; updated doc comments
- `crates/nono-cli/src/agent_daemon/reap.rs` — Removed `#[expect(dead_code)]` from `caps`/`process_handle` fields; updated doc comments
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `create_process_containment` + `apply_process_handle_to_containment`: `pub(super)` → `pub(crate)`
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `ProcessContainment`: private → `pub(crate)`
- `crates/nono/src/supervisor/socket_windows.rs` — `build_capability_pipe_sddl`: `fn` → `pub fn`
- `crates/nono/src/supervisor/mod.rs` — Added `build_capability_pipe_sddl` to Windows re-export block

## Decisions Made

**D1: A6 resolution — inlined Win32 job/process APIs rather than calling exec_strategy_windows functions**

The plan indicated `launch.rs` should call `create_process_containment` + `apply_process_handle_to_containment` from `exec_strategy_windows`. However, `nono-agentd.rs` is a separate Cargo binary whose `crate::` root is itself, not `main.rs`. `exec_strategy_windows` is declared only in `main.rs`'s module tree and is unreachable from `agent_daemon/launch.rs` in the agentd binary context. Attempted `#[path = "../exec_strategy_windows/mod.rs"] mod exec_strategy_windows` in `nono-agentd.rs` failed because `exec_strategy_windows` pulls in dozens of modules (`profile`, `pty_proxy`, `rollback_runtime`, etc.) not in scope for the agentd binary. Decision: inline `CreateJobObjectW` + `SetInformationJobObject(KILL_ON_JOB_CLOSE)` + `CreateProcessW` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` + `AssignProcessToJobObject` directly in `agent_daemon/launch.rs`, matching the pattern in `exec_strategy_windows/launch.rs`'s existing `spawn_appcontainer_child` tests.

**D2: Reap mechanism — spawn_blocking + WaitForSingleObject(INFINITE)**

Used `tokio::task::spawn_blocking` with `WaitForSingleObject(reap_handle, INFINITE)` as the reap strategy. The process handle for the reap task is a `DuplicateHandle` copy (the original `process_handle` stays in `AgentTenant`). HANDLE Send-safety was achieved by casting to `usize` before the `spawn_blocking` boundary and casting back inside.

**D3: Authorization gate — SID-only, never session_id**

`handle_one_connection` calls `authenticate_pipe_client` to get the kernel-vouched AppContainer SID, then looks up the tenant by SID in `daemon_state.tenants`. The wire-frame `session_id` field is NOT used for authorization — it is a routing hint only. This directly mitigates STRIDE threat T-74-04-01.

**D4: Base pipe SDDL**

Called `build_capability_pipe_sddl(None, None)` (both `session_sid` and `package_sid` are `None`) to get the base Low-IL SDDL for the pipe instance. This SDDL admits all Low-IL processes in the same session. The post-connection `authenticate_pipe_client` + registry SID check provides the tenant isolation boundary — no per-tenant SDDL on the pipe instance itself (consistent with ADR-74 Decision D-04).

**D5: build_capability_pipe_sddl visibility**

Promoted from `fn` (private) to `pub fn` in `socket_windows.rs` and added to the `#[cfg(target_os = "windows")] pub use socket::{...}` block in `nono/src/supervisor/mod.rs`. This enables `nono::supervisor::build_capability_pipe_sddl` to be called from `nono-cli`'s `accept_loop.rs`. The function is safe, does not mutate state, and is appropriate to expose at the library's public supervisor API surface.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] exec_strategy_windows unreachable from nono-agentd binary**
- **Found during:** Task 2 (launch.rs implementation)
- **Issue:** `crate::exec_strategy_windows` unresolved in `agent_daemon/launch.rs`. The plan called for using `create_process_containment` + `apply_process_handle_to_containment` from `exec_strategy_windows`, but `exec_strategy_windows` is only in `main.rs`'s module tree, not in `nono-agentd.rs`'s binary crate root.
- **Fix:** Inlined all job creation and AppContainer process spawning in `agent_daemon/launch.rs` using raw Win32 APIs directly, matching the exact pattern in `exec_strategy_windows/launch.rs`. Made `ProcessContainment` and `create_process_containment`/`apply_process_handle_to_containment` `pub(crate)` (prerequisite commit `dcf0aad4`) as a partial step — the actual launch.rs did not end up calling these because the binary-crate isolation issue persisted.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** `cargo build -p nono-cli --bin nono-agentd` clean; 10 tests pass
- **Committed in:** `2cefe134`

**2. [Rule 1 - Bug] Wrong LocalFree import path**
- **Found during:** Task 1 (accept_loop.rs SecurityDescriptorGuard::drop)
- **Issue:** `windows_sys::Win32::Security::LocalFree` does not exist; correct path is `windows_sys::Win32::Foundation::LocalFree`
- **Fix:** Corrected import inside the `SecurityDescriptorGuard` drop implementation
- **Files modified:** `crates/nono-cli/src/agent_daemon/accept_loop.rs`
- **Committed in:** `ca492378`

**3. [Rule 1 - Bug] build_capability_pipe_sddl was private, could not be re-exported**
- **Found during:** Task 1 (accept_loop.rs calling build_capability_pipe_sddl across crate boundary)
- **Issue:** `build_capability_pipe_sddl` was `fn` (private). Attempting `pub(crate)` failed: "is only public within the crate, cannot be re-exported outside".
- **Fix:** Changed to `pub fn` and added to `supervisor/mod.rs` re-export block
- **Files modified:** `crates/nono/src/supervisor/socket_windows.rs`, `crates/nono/src/supervisor/mod.rs`
- **Committed in:** `6277637e`

**4. [Rule 1 - Bug] HANDLE not Send for tokio::spawn**
- **Found during:** Task 2 (reap task needing process handle across spawn boundary)
- **Issue:** `HANDLE` (`*mut c_void`) is not `Send`; passing it in a `tokio::spawn` async block fails to compile.
- **Fix:** Cast to `usize` before the `spawn_blocking` call boundary (`let reap_handle_usize = reap_handle_raw as usize;`), cast back inside the closure. `usize` is Send. No `unsafe impl Send` needed.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Committed in:** `2cefe134`

---

**Total deviations:** 4 auto-fixed (2 blocking blockers, 2 bugs)
**Impact on plan:** All auto-fixes necessary for correctness. The binary-crate isolation deviation is the most architecturally significant — it establishes the "inline Win32 APIs in agent_daemon, do not attempt to reach main.rs module tree" pattern for future agentd plans.

## Cross-Target Status

All files changed in this plan are Windows-only (`#[cfg(target_os = "windows")]`-gated or under `exec_strategy_windows/`):
- `crates/nono-cli/src/agent_daemon/accept_loop.rs` — `#[cfg(target_os = "windows")]` mod gate
- `crates/nono-cli/src/agent_daemon/launch.rs` — `#[cfg(target_os = "windows")]` mod gate
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — under `exec_strategy_windows/` (Windows-only)
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — under `exec_strategy_windows/` (Windows-only)
- `crates/nono/src/supervisor/socket_windows.rs` — `_windows` suffix, no Linux/macOS counterpart

Per `.planning/templates/cross-target-verify-checklist.md` § Scope: "Does NOT apply to pure Windows-only files." Cross-target clippy verification is **NOT REQUIRED** for this plan. Windows-host `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` is the decisive signal and passed clean.

## Issues Encountered

**Binary crate module isolation** — The biggest discovery of this plan. `nono-agentd.rs` is a separate Cargo `[[bin]]` target in `nono-cli`. When the binary declares `#[path = "../agent_daemon/mod.rs"] mod agent_daemon;`, all of `agent_daemon/`'s code runs with `crate::` pointing at the agentd binary — NOT at `main.rs`. This means `crate::exec_strategy_windows` (declared in `main.rs`'s `use` tree) is invisible from `agent_daemon/launch.rs` when compiled by the agentd binary. Future plans for Wave 3 (`74-05`, `74-06`) should factor any shared Windows-spawn logic into the `nono` crate's `supervisor` module or into a `pub(crate)` utility in a file that the agentd binary can reach via its own `#[path]` declaration without pulling in `main.rs`'s full dependency graph.

## User Setup Required

None — no external service configuration required. Changes are internal daemon implementation.

## Next Phase Readiness

- Wave 2 complete: accept loop + launch orchestration both implemented; nono-agentd wired and compiling
- `DaemonState.tenants` and `DaemonState.agent_registry` both live and correctly managed with KILL_ON_JOB_CLOSE reap
- Wave 3 (`74-05`) can call `agent_daemon::launch::launch_agent` from the accept loop's session-init path (the `#[allow(dead_code)]` on `windows_impl` module is the handoff signal; Wave 3 removes it when wiring the call)
- Blocker: `launch_agent` in Wave 3 will need to supply a real `nono::CapabilitySet` from the client's initial capability-request frame (currently the test uses `CapabilitySet::new()`); the Wave 3 plan should specify how the initial caps are sourced

---
*Phase: 74-persistent-multi-tenant-daemon*
*Completed: 2026-06-15*

## Self-Check: PASSED

Files verified to exist:
- `crates/nono-cli/src/agent_daemon/accept_loop.rs` — FOUND
- `crates/nono-cli/src/agent_daemon/launch.rs` — FOUND
- `crates/nono-cli/src/bin/nono-agentd.rs` — FOUND

Commits verified:
- `dcf0aad4` — FOUND (git log)
- `6277637e` — FOUND (git log)
- `ca492378` — FOUND (git log)
- `2cefe134` — FOUND (git log)
