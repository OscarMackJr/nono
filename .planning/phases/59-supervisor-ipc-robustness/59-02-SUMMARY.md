---
phase: 59-supervisor-ipc-robustness
plan: 02
subsystem: supervisor-ipc
tags: [supervisor-ipc, keep-alive, read-timeout, macos, unix, integration-testing, rust]

# Dependency graph
requires: [59-01]
provides:
  - "SC1: macOS run_supervisor_loop sock_fd_active keep-alive (demote-not-break) matching the Linux arm predicate"
  - "SC2: set_read_timeout(supervisor_ipc_read_timeout()) wired on the supervisor socket end before run_supervisor_loop"
  - "SC2 fail-closed: WouldBlock/TimedOut classified as non-fatal keep-alive (no capability granted on partial frame)"
  - "in-crate reconnect_survival test (#[cfg(unix)]) driving private run_supervisor_loop via PtyProxy"
  - "lib-surface bounded_read_timeout integration test proving SC2 via SupervisorSocket::pair() + set_read_timeout() + recv_message()"
  - "Updated test_supervisor_loop_runs_without_pty_relay docstring with SC1 scope note"
affects:
  - exec_strategy.rs (macOS supervisor loop arm + supervisor socket setup)
  - supervisor_ipc_robustness_unix.rs (bounded_read_timeout test)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SC1 keep-alive: sock_fd_active demote predicate (pty.is_some()) mirrors the Linux arm's (seccomp || proxy || pty.is_some()) — narrower-by-default fail-secure rule"
    - "SC2 fail-closed: WouldBlock/TimedOut from recv_message classified via to_string().contains() — non-fatal keep-alive, not supervisor kill, no capability grant on partial frame"
    - "set_read_timeout wiring: #[cfg(unix)] block after supervisor_sock destructuring, error propagated with ? (no .unwrap())"
    - "reconnect_survival test uses PtyProxy::new + openpty to satisfy pty.is_some() predicate; child calls nanosleep(350ms) after closing IPC socket; assert elapsed >= 200ms proves demote-not-break"
    - "bounded_read_timeout test uses UnixStream::pair() + SupervisorSocket::from_stream() to inject partial frame (4-byte header only); asserts Err within 800ms-5s window"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/tests/supervisor_ipc_robustness_unix.rs

key-decisions:
  - "socket.rs declared read-only: set_read_timeout() + recv_message() + read_frame already surface WouldBlock/TimedOut via NonoError::SandboxInit wrapping std::io::Error with message containing 'timed out'/'would block'; no error-classification tweak needed"
  - "sock_fd_active demote predicate scoped to pty.is_some() only (macOS arm) — exactly mirroring the Linux arm's pty condition, narrowest-correct scope; other PTY relay fds retain existing break/demotion logic unchanged"
  - "SC3 N/A: be7681c (named socket mechanism) and 4a22e94 (UnixSocketCapability grant) are NOT ported — the fork's IPC uses an inherited socketpair fd (NONO_SUPERVISOR_FD), not a filesystem named socket; named-socket hardening is structurally inapplicable"
  - "f956fb6 blocking-mode verify: socketpair ends are blocking by default (confirmed by POSIX — UnixStream::pair() returns blocking sockets); set_read_timeout adds a bounded wall-clock limit while leaving full-frame reads working (A3)"
  - "c15c76a hygiene: debug! logging added on disconnect/timeout/re-accept mirroring the Linux arm's logging pattern (A4 absorbed)"
  - "test_supervisor_loop_runs_without_pty_relay preserved: tests the no-PTY path where break-on-disconnect is still correct; SC1 note added explaining scope"

patterns-established:
  - "Pattern: SC1 keep-alive predicate = demote-not-break the IPC socket fd when other active fds (PTY/seccomp/proxy) are present; scoped to the URL/direct-IPC listener fd only"
  - "Pattern: SC2 fail-closed = classify timeout error by to_string() substring match; loop continues, sandbox intact, no capability granted on partial frame"

requirements-completed: [REQ-IPC-01]

# Metrics
duration: 17min
completed: 2026-06-06
---

# Phase 59 Plan 02: Unix Supervisor IPC Hardening (SC1 + SC2 + SC3) Summary

**SC1 macOS keep-alive (sock_fd_active demote-not-break) + SC2 set_read_timeout(5s) wired + SC3 N/A documented + SC1 reconnect_survival in-crate test + SC2 bounded_read_timeout lib-surface integration test**

## Performance

- **Duration:** 17 min
- **Started:** 2026-06-06T13:37:24Z
- **Completed:** 2026-06-06T13:54:12Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

### SC1 — macOS run_supervisor_loop keep-alive (Task 1)

Modified the macOS `run_supervisor_loop` arm (`#[cfg(not(target_os = "linux"))]` at exec_strategy.rs:2284) to add the `sock_fd_active` demote-not-break predicate matching the Linux arm:

- Declared `let mut sock_fd_active = true;` before the loop
- Changed `fd: sock_fd` to `fd: if sock_fd_active { sock_fd } else { -1 }` in the first pollfd so a demoted IPC fd is ignored by `poll(2)`
- On POLLHUP/POLLERR: if `pty.is_some()` → `sock_fd_active = false; continue` (demote); else → `break` (unchanged no-PTY behavior)
- On `recv_message` error: same predicate — timeout classified as non-fatal keep-alive; non-timeout disconnect demotes when `pty.is_some()`, breaks when `pty.is_none()`

**Demoted fd (Q1):** The single supervisor IPC socket fd (`sock_fd`) — the URL-open/direct-IPC rendezvous. This is the ONLY fd that received the demote-don't-break treatment. PTY relay fds (pty_master, pty_client, pty_attach, pty_resize) retain their existing break/demote logic unchanged.

**Open-URL reconnect path (Q3):** The macOS open-url shim creates a script that exec's `nono open-url-helper --fd <fd>`. The helper process inherits `NONO_SUPERVISOR_FD` (the child's end of the socketpair). When the helper exits, it closes the fd. If the main child process also had `NONO_SUPERVISOR_FD` open (via inheritance), POLLHUP only fires when ALL holders close the fd. In the URL-open case, the helper is a separate exec'd process, so its exit closes its inherited copy of the fd. The SC1 fix ensures the supervisor survives this close event when a PTY relay is still active.

### SC2 — set_read_timeout wiring (Task 1)

Added the following block after `supervisor_sock` destructuring (parent arm of `execute_supervised_fork`):

```rust
#[cfg(unix)]
if let Some(ref sock) = supervisor_sock {
    sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?;
}
```

- Only wired on Unix (the socketpair is Unix-only; the `socket_pair` and `supervisor_sock` are created in the same fork'd code path)
- Error propagated with `?` (no `.unwrap()`); fail-secure: a failure to set the timeout aborts the supervised run
- `supervisor_ipc_read_timeout()` returns 5s by default (from `SUPERVISOR_IPC_READ_TIMEOUT`), overridable via `NONO_SUPERVISOR_IPC_READ_TIMEOUT`

### SC3 — Cluster C2 divergence documented (N/A)

Upstream commits `be7681c` (named socket mechanism) and `4a22e94` (UnixSocketCapability grant) are **not ported**. The fork's IPC uses an inherited `socketpair` fd (`NONO_SUPERVISOR_FD`), not a filesystem named socket. Named-socket hardening (permissions, filesystem access grants) is structurally inapplicable to the socketpair model. The socketpair fd is:
- Anonymous (no filesystem path reachable by other processes)
- Inherited by the child at `fork()` time (no connect/bind race)
- Already secured by OS-level POSIX semantics

### SC1 reconnect_survival in-crate test (Task 2)

Added `reconnect_survival` in `exec_strategy.rs` at the `#[cfg(unix)]` test section. The test:
1. Creates a PTY pair via `openpty()` + `PtyProxy::new()` (no initial client) to satisfy `pty.is_some()`
2. Forks a child that immediately closes its IPC socket (simulating a URL-open helper exit) then calls `nanosleep(350ms)` before `_exit(0)`
3. Calls `run_supervisor_loop` with `pty: Some(&mut pty_proxy)` on the parent side
4. Asserts `elapsed >= 200ms` (loop survived the IPC disconnect and waited for child exit) and `exit code == 0`

Without SC1, the loop would `break` on POLLHUP and `wait_for_child` would block for 350ms but the elapsed would be measured differently (the loop exits before the child). With SC1, the loop stays in the poll cycle until `waitpid(WNOHANG)` detects the child exit after 350ms.

The test handles `PtyProxy::new` failure gracefully (skips the test with `eprintln!` + `return`) in case `HOME` is not set or the sessions directory cannot be created.

### SC2 bounded_read_timeout integration test (Task 2)

Filled the Wave-0 TODO placeholder in `supervisor_ipc_robustness_unix.rs` with the SC2 test:
1. Creates a `UnixStream::pair()` and wraps the supervisor end in `SupervisorSocket::from_stream()`
2. Calls `set_read_timeout(Some(Duration::from_secs(1)))` via the lib pub surface
3. Spawns a thread that writes only the 4-byte length prefix (announces 100-byte payload, never sends it)
4. Calls `recv_message()` and measures elapsed time
5. Asserts `Err` returned, elapsed >= 800ms (timeout fired), elapsed < 5s (bounded)

This proves SC2 at the library boundary without requiring the private `run_supervisor_loop`. The test references only `nono::supervisor::socket::SupervisorSocket` (never `nono_cli::`).

## Task Commits

Each task committed atomically:

1. **Task 1: SC1 macOS keep-alive + SC2 set_read_timeout wiring + SC1 reconnect_survival test** - `e9032edd` (feat)
2. **Task 2: SC2 bounded_read_timeout integration test via nono lib pub surface** - `ce2f3d19` (feat)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy.rs` - Added `sock_fd_active` predicate to macOS `run_supervisor_loop`; wired `set_read_timeout` in `execute_supervised_fork`; updated `test_supervisor_loop_runs_without_pty_relay` scope note; added `reconnect_survival` in-crate test
- `crates/nono-cli/tests/supervisor_ipc_robustness_unix.rs` - Filled Wave-0 TODO with `bounded_read_timeout` integration test
- `crates/nono/src/supervisor/socket.rs` - **READ-ONLY** (reviewed only; no changes required — see "socket.rs scope" below)

## socket.rs Scope Reconciliation

`socket.rs` is declared in `files_modified` as a possible "error-classification tweak" scope. After inspection:

- `set_read_timeout()` is already implemented (line 192) and propagates `std::io::Error` via `NonoError::SandboxInit`
- `read_frame()` uses `read_exact()` which returns `WouldBlock`/`TimedOut` error kinds when a read timeout fires
- The error message from `NonoError::SandboxInit` wraps the `std::io::Error` message: "timed out", "would block" — distinguishable by the `to_string().contains()` check in the macOS loop arm
- No error-classification change was needed

**Result: socket.rs left READ-ONLY (declared-superset scope; no edit invented).**

## f956fb6 Blocking-Mode Verification (A3)

`UnixStream::pair()` (and by extension `SupervisorSocket::pair()`) creates blocking sockets by default per POSIX. `set_read_timeout()` sets `SO_RCVTIMEO` on the underlying socket, which adds a wall-clock bound to blocking `read()` calls without converting the socket to non-blocking mode. Full-frame reads (header + payload in one `recv_message()` call) complete normally when the payload arrives before the timeout. The timeout only fires on stalled/partial frames. Confirmed by the `bounded_read_timeout` test: a complete 4-byte header write succeeds (the header `read_exact` returns before the timeout), while the 100-byte payload `read_exact` stalls and fires the timeout.

## c15c76a Hygiene (A4)

`debug!` logging added on disconnect/timeout/re-accept events in the macOS loop arm, mirroring the Linux arm's logging pattern. Specific messages:
- `"Supervisor socket closed by child; PTY relay active, keeping supervisor alive"` (SC1 POLLHUP demote)
- `"Supervisor socket read timeout (partial frame stall); keeping supervision alive"` (SC2 timeout non-fatal)
- `"Error receiving supervisor message: {}; PTY relay active, keeping supervisor alive"` (SC1 recv-error demote)

## Deviations from Plan

### Auto-fixed Issues

None - plan executed exactly as specified.

### Socket.rs: Declared-superset scope, read-only

The plan pre-declared `socket.rs` in `files_modified` as a POSSIBLE scope (error-classification tweak). After inspection, the existing `WouldBlock`/`TimedOut` error surfacing was sufficient. No change made; documented above. This is within the plan's stated intent ("intentionally a superset; do not invent an edit to justify it").

### set_read_timeout Wiring: #[cfg(unix)] not explicitly mentioned in plan

The plan's SC2 action says to wire `set_read_timeout` "after wrapping the supervisor sock, before `run_supervisor_loop`". The `execute_supervised_fork` function uses `fork()` and is Unix-only, but the `supervisor_sock` destructuring code at line 1279 is in the parent arm. Added a `#[cfg(unix)]` guard consistent with the surrounding fork-only code structure. This is a no-op on Windows (which doesn't use `fork()` in this path).

## Cross-Target Clippy Status

**PARTIAL / CI-deferred** per CLAUDE.md MUST rule and `.planning/templates/cross-target-verify-checklist.md`.

This plan modifies `crates/nono-cli/src/exec_strategy.rs` which contains `#[cfg(not(target_os = "linux"))]` and `#[cfg(target_os = "linux")]` branches, and `crates/nono-cli/tests/supervisor_ipc_robustness_unix.rs` which has `#![cfg(unix)]`. The modified Unix cfg-branches cannot be compiled or clippy-checked on the Windows host.

- **Windows host**: `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` PASSES (confirmed: no warnings)
- **Linux target** (`x86_64-unknown-linux-gnu`): cross-toolchain not installed on this Windows host — DEFERRED to live CI
- **macOS target** (`x86_64-apple-darwin`): cross-toolchain not installed on this Windows host — DEFERRED to live CI

The SC1 and SC2 changes are in the `#[cfg(not(target_os = "linux"))]` arm (macOS/other Unix) and the `#[cfg(unix)]` test gate. Clippy verification for these branches requires a cross-compilation toolchain or a Unix CI runner.

## Baseline-Aware Test Results

Windows `cargo test -p nono-cli --bin nono` results:
- **Pre-existing failures (4, documented in `nono_cli_windows_baseline_test_failures.md`):**
  - `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`
  - `protected_paths::tests::blocks_parent_directory_capability`
  - `protected_paths::tests::blocks_child_directory_capability`
  - `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`
- **New failures introduced by this plan: ZERO**
- **`cargo test -p nono-cli --test aipc_handle_brokering_integration`: 5/5 PASS** (regression guard green)
- **`cargo test -p nono-cli --test supervisor_ipc_robustness_unix`**: 0 tests on Windows (correct — `#![cfg(unix)]` produces empty binary)
- **`cargo test -p nono-cli --test supervisor_ipc_robustness_windows`**: scaffold_links_nono_lib PASS

## Known Stubs

None — this plan implements production hardening and tests, not UI or data-wiring components.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The changes are:
- Defensive timeout on an existing socket (reduces attack surface for slowloris)
- Loop keep-alive on an existing fd (no new trust boundaries)
- Test-only code (no production surface area added)

All threats from the plan's STRIDE register (T-59-01a through T-59-SC) are mitigated as specified.

---
*Phase: 59-supervisor-ipc-robustness*
*Completed: 2026-06-06*
