---
phase: 59-supervisor-ipc-robustness
plan: 03
subsystem: testing
tags: [supervisor-ipc, windows, named-pipe, PeekNamedPipe, re-accept, robustness, rust]

# Dependency graph
requires:
  - phase: 59-supervisor-ipc-robustness
    plan: 01
    provides: "SUPERVISOR_IPC_READ_TIMEOUT const + supervisor_ipc_read_timeout() + Windows scaffold supervisor_ipc_robustness_windows.rs"
provides:
  - "PeekNamedPipe-bounded read_exact_bounded helper in socket_windows.rs (SC2/SC4)"
  - "disconnect_and_reconnect() pub method on SupervisorSocket for re-accept (SC1/SC4)"
  - "write_raw_bytes() pub method on SupervisorSocket for test partial-frame injection"
  - "recv_message_with_timeout(Duration) pub method on SupervisorSocket (policy-free boundary)"
  - "capability-pipe re-accept loop in exec_strategy_windows/supervisor.rs (SC1)"
  - "Windows-gated integration tests: 4 tests passing in supervisor_ipc_robustness_windows.rs"
affects:
  - "59-02 (parallel Wave-2 plan — Unix side; no overlap on files_modified)"
  - "future supervisory IPC plans referencing Windows AIPC timeout/re-accept"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PeekNamedPipe poll-until-data-or-deadline as translate-not-cherry-pick from Unix set_read_timeout"
    - "ERROR_PIPE_CONNECTED-is-success idiom reused from finalize_server_connection for re-accept"
    - "Error tag strings ([disconnect] / [timeout]) for caller-distinguishable error classification"
    - "Send-safe HANDLE transfer via usize in integration test threads"

key-files:
  created: []
  modified:
    - crates/nono/src/supervisor/socket_windows.rs
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    - crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs

key-decisions:
  - "TRANSLATE-NOT-CHERRY-PICK: AF_UNIX is unix-only; no Unix socket code was ported. The PeekNamedPipe poll-until-data-or-deadline is the Windows equivalent of set_read_timeout on a Unix socket. The rejected alternative (overlapped I/O with ReadFile-overlapped + WaitForSingleObject + CancelIoEx) was explicitly rejected as too large and risky for this phase (D-03)."
  - "Error classification uses string tags ([disconnect] / [timeout]) rather than distinct error types to keep the library-cli boundary clean and avoid proliferating NonoError variants for transport-level conditions"
  - "disconnect_and_reconnect() re-arms the SAME handle (DisconnectNamedPipe + ConnectNamedPipe) rather than creating a fresh pipe instance (Pitfall 3) — mandatory for 1-instance control pipes; works for AIPC (PIPE_UNLIMITED_INSTANCES) too"
  - "seen_request_ids replay set is NOT reset on reconnect (V3 security invariant); session token re-verified by handle_windows_supervisor_message on every incoming message — transport reset does not weaken session-level trust"
  - "write_raw_bytes() added to SupervisorSocket pub surface as a documented test helper for partial-frame injection — alternative (raw Win32 WriteFile on HANDLE) would require exposing the handle, which is a larger API surface change"

patterns-established:
  - "Pattern: [disconnect] / [timeout] string tags in error messages for caller-distinguishable transport-layer error classification on Windows named pipes"
  - "Pattern: Send-safe HANDLE transfer across threads via usize cast + re-cast in integration tests"

requirements-completed: [REQ-IPC-01]

# Metrics
duration: PARTIAL (Tasks 1-2 complete; Task 3 awaiting operator UAT)
completed: PENDING
---

# Phase 59 Plan 03: Windows AIPC IPC Robustness — Partial Summary (Tasks 1-2)

**PeekNamedPipe deadline-bounded read_frame + capability-pipe re-accept loop on Windows; 4 integration tests green; SC1/SC2 live-repro PENDING operator UAT (Task 3)**

## Status

**PARTIAL** — Tasks 1 and 2 are complete and committed. Task 3 is a `checkpoint:human-verify (gate="blocking")` requiring operator-run live-repro on a real Win11 console. The plan is NOT marked complete until the operator approves.

## Performance

- **Duration:** ~45 min (Tasks 1-2 only)
- **Started:** 2026-06-06T14:10:00Z
- **Completed (partial):** 2026-06-06 (Tasks 1-2); Task 3 PENDING
- **Tasks completed:** 2 of 3
- **Files modified:** 3

## Accomplishments

- Added `read_exact_bounded(reader, buf, deadline)` helper to `socket_windows.rs` using `PeekNamedPipe` as a non-destructive availability probe, with 10ms sleep-between-peeks (T-59-03b anti-busy-spin) and deadline-exceeded detection; rewired `read_frame` to use it with the library's `DEFAULT_READ_TIMEOUT` (5s)
- Added `recv_message_with_timeout(Duration)` pub method to `SupervisorSocket` so the CLI can pass `supervisor_ipc_read_timeout()` without the library reading env vars (policy-free boundary)
- Classified disconnect errors with `[disconnect]` tag and deadline errors with `[timeout]` tag in error messages — distinguishable by the supervisor loop
- Added `disconnect_and_reconnect()` pub method to `SupervisorSocket` encapsulating `DisconnectNamedPipe` + `ConnectNamedPipe` re-arm with the `ERROR_PIPE_CONNECTED`-is-success idiom
- Converted the capability-pipe server loop in `exec_strategy_windows/supervisor.rs` from break-on-close to a re-accept loop: `[timeout]` → keep-alive continue; `[disconnect]` → `sock.disconnect_and_reconnect()` + continue; unknown → break (fail-closed). Replay protection (`seen_request_ids`) preserved; session token re-verified per message
- Filled `supervisor_ipc_robustness_windows.rs` with 4 tests: scaffold (existing), bounded-read timeout, disconnect_and_reconnect reachability, named-pipe reconnect round-trip

## Task Commits

1. **Task 1: PeekNamedPipe-bounded read_frame in socket_windows.rs** - `c2993a03` (feat)
2. **Task 2: capability-pipe re-accept loop + Windows-gated lib-surface tests** - `51f54fa8` (feat)
3. **Task 3: Windows live-repro UAT** - PENDING operator approval

## Files Created/Modified

- `crates/nono/src/supervisor/socket_windows.rs` - Added `read_exact_bounded`, `read_frame_with_timeout`, `recv_message_with_timeout`, `disconnect_and_reconnect`, `write_raw_bytes` methods; added `PeekNamedPipe` import and `DEFAULT_READ_TIMEOUT`/`POLL_INTERVAL` constants; rewired `read_frame` to bounded variant
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` - Converted break-on-close cap-pipe loop to re-accept: `recv_message_with_timeout` + `[disconnect]`/`[timeout]` error classification + `disconnect_and_reconnect()` call
- `crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs` - Filled 59-03 Wave-2 insertion points with 4 tests

## SC4: Translate-Not-Cherry-Pick Rationale

**This is a TRANSLATE-NOT-CHERRY-PICK.** AF_UNIX sockets are a POSIX-only construct (`socket(AF_UNIX, ...)`) — they do not exist on Windows. No Unix socket code was or could be cherry-picked to the Windows path.

The **translation chosen**: a `PeekNamedPipe` poll-until-data-or-deadline loop (`read_exact_bounded`), which achieves the same semantic as `set_read_timeout(Duration)` on a Unix socket: the read returns within the deadline rather than blocking indefinitely under `PIPE_WAIT`.

The **rejected alternative**: a full overlapped-I/O rewrite — replace the blocking `File::read` with `ReadFile` using an `OVERLAPPED` structure + `WaitForSingleObjectEx` + `CancelIoEx` to cancel a timed-out read. This approach was explicitly rejected (D-03) as:
- Significantly larger scope (OVERLAPPED I/O lifecycle, IOCP or event-based completion, cancellation races)
- Higher risk (cancel-then-read ordering bugs, handle lifetime with OVERLAPPED)
- Requires converting the `File`-based reader to a raw HANDLE throughout
- The PeekNamedPipe approach is well-understood, non-destructive, and tested

## Verification (Tasks 1-2)

| Check | Result |
|-------|--------|
| `cargo build -p nono` | PASS |
| `cargo clippy -p nono -D warnings -D clippy::unwrap_used` | PASS |
| `cargo test -p nono --lib supervisor::socket` | 26/26 PASS |
| `cargo build -p nono-cli` | PASS |
| `cargo clippy -p nono-cli --bin nono -D warnings -D clippy::unwrap_used` | PASS |
| `cargo test -p nono-cli --test supervisor_ipc_robustness_windows` | 4/4 PASS |
| `cargo test -p nono-cli --test aipc_handle_brokering_integration` | 5/5 PASS |
| Cross-target clippy (Linux/macOS) | PARTIAL / deferred to CI (Windows-host cannot run cross-target; files are `#[cfg(target_os = "windows")]`-gated and only exercised on Windows) |

## Multi-process live-repro: cap-pipe-live-repro example

### Artifact

**Path:** `crates/nono-cli/examples/cap-pipe-live-repro.rs`

A standalone multi-process live-repro helper that drives the production
`SupervisorSocket::bind` + `recv_message_with_timeout` + `disconnect_and_reconnect`
server API across two real OS processes over a 1-instance named pipe — the same
transport shape as the production capability pipe.

The example is cross-platform: a non-Windows stub plus a `#[cfg(target_os = "windows")]
mod windows_impl` module that uses only `nono::supervisor::socket::SupervisorSocket`,
`std::process::Command`, and standard library types. No `unsafe`, no `windows-sys`,
no `nono_cli::` imports.

### Run command

```
cargo run --quiet --example cap-pipe-live-repro -p nono-cli -- --scenario both --timeout-secs 2
```

### Captured PASS output (Windows 11 host, 2026-06-06)

```
cap-pipe-live-repro: scenario=both timeout=2s

--- SC2: bounded read (slow child) ---
pipe: \\.\pipe\nono-cap-live-repro-4554-18b687056b5457c4
SC2 RESULT: PASS (elapsed=2.01s, err="Sandbox initialization failed: [timeout] Supervisor IPC read deadline exceeded after 0 bytes (needed 64); in-flight partial frame discarded (fail-closed)")
--- SC1: transient close / re-accept ---
pipe: \\.\pipe\nono-cap-live-repro-4554-18b68705e52f0350
SC1: conn1 established
SC1 RESULT: PASS (disconnect_and_reconnect() returned Ok)

OVERALL: PASS
```

### Scope note

This proves the bounded-read (`[timeout]` tag) and re-accept (`disconnect_and_reconnect()
→ Ok`) behavior at the **multi-process layer** over the real `SupervisorSocket` server
API. It is a materially stronger evidence level than the in-process `pair()` integration
tests — it crosses a real OS process boundary and exercises the real `bind()` +
`ConnectNamedPipe` server-side lifecycle.

However, it uses a **normal pipe and a normal child** (no `WRITE_RESTRICTED` token, no
Low-IL broker). It does NOT exercise the `WRITE_RESTRICTED`-token / Low-IL broker path.
The full `nono run` under a restricted token launched from a real Win11 console (with
`target\release\nono.exe` from a profile-covered cwd) therefore **REMAINS** the operator's
Task-3 item. This helper materially de-risks it — the core PeekNamedPipe deadline logic
and the re-accept protocol are now proven multi-process — but does not replace the
operator UAT.

**Task-3 operator UAT status: STILL PENDING.**

## AIPC-SDK live-repro child: aipc-cap-child example

### Artifact

**Path:** `crates/nono-cli/examples/aipc-cap-child.rs`

A cross-platform AIPC-SDK child harness designed to be launched as the child
process under `nono run --profile claude-code`. When the supervisor injects
`NONO_CAP_FILE` into the child environment, the binary reads it and drives the
real capability pipe using the `nono::supervisor::socket::SupervisorSocket` and
`nono::supervisor::aipc_sdk::request_event` pub surface. No `nono_cli::`
references; no `unsafe`; cross-platform stub on non-Windows.

**SC2 mode (`aipc-cap-child.exe sc2`):** Connects to `NONO_CAP_FILE`, sends a
4-byte big-endian length prefix announcing 64 bytes (payload never sent), then
stalls for 20 s. Exercises the supervisor's PeekNamedPipe deadline-poll bounded
read: the supervisor must detect the partial frame within the configured
`NONO_SUPERVISOR_IPC_READ_TIMEOUT` and keep the supervision loop responsive
rather than blocking indefinitely.

**SC1 mode (`aipc-cap-child.exe sc1`):** Makes two sequential connections to
`NONO_CAP_FILE`, each using `aipc_sdk::request_event` with a named event and
`EVENT_ACCESS_MASK = 0x0010_0002` (SYNCHRONIZE | EVENT_MODIFY_STATE). Between
the two connections the child deliberately drops conn1 and waits 500 ms. Prints
`SC1 RESULT: PASS` if conn2 receives any supervisor Decision (Approved or
Denied-but-responded), proving the cap pipe re-accepted after the transient
close. If conn2 gets a transport error (supervisor permanently disabled the
pipe), prints `SC1 RESULT: FAIL`.

**`--selftest <sc1|sc2>` mode:** Runs the child-side mechanics against an
in-process mimic pipe — no `nono run` required, no WRITE_RESTRICTED token, no
SDK dispatcher. The mimic acts as a minimal supervisor: for SC2 it asserts that
`recv_message_with_timeout(2s)` returns an error containing `[timeout]` within
4 s; for SC1 it binds, accepts conn1, calls `disconnect_and_reconnect()`, and
asserts conn2 arrives. Proves the connect/write/reconnect mechanics in
isolation.

### Captured selftest output (Windows 11 host, 2026-06-06)

```
$ cargo run --quiet --example aipc-cap-child -p nono-cli -- --selftest sc2
SELFTEST sc2 RESULT: PASS

$ cargo run --quiet --example aipc-cap-child -p nono-cli -- --selftest sc1
SELFTEST sc1 RESULT: PASS
```

**Selftest scope caveat:** Selftest uses a normal named pipe with no
restricting-SID DACL, no WRITE_RESTRICTED token, and the raw `SupervisorSocket`
transport (no AIPC dispatcher). It proves only that the child-side
connect/write/reconnect code paths execute correctly at the transport layer. The
`aipc_sdk::request_event` path (which requires `NONO_SESSION_TOKEN` in the
environment and a live supervisor dispatcher) is exercised only under a real
`nono run` (sc1 / sc2 modes).

### Build

```
cargo build --release --example aipc-cap-child -p nono-cli
# Output: target\release\examples\aipc-cap-child.exe
```

### Operator live-repro commands (Win11 console, dev-layout)

**Profile:** `claude-code` — this is the built-in profile with
`windows_low_il_broker: true` (confirmed in `crates/nono-cli/data/policy.json`
line 729). The Low-IL broker arm is required for the production cap-pipe server
to run (supervisor.rs:411).

**Pre-requisites:** Run from a profile-covered cwd. The `claude-code` profile
covers `$HOME/.claude` (`%USERPROFILE%\.claude`). Use
`target\release\nono.exe` (dev-layout) to skip the broker trust gate (unsigned
`Program Files` install fails the gate by design).

**SC2 — partial frame / bounded read:**
```
cd %USERPROFILE%\.claude
C:\path\to\repo\target\release\nono.exe run --profile claude-code --allow-cwd -- ^
    C:\path\to\repo\target\release\examples\aipc-cap-child.exe sc2
```

Set `NONO_SUPERVISOR_IPC_READ_TIMEOUT=2` (seconds) for fast observation (default
is 5 s):
```
set NONO_SUPERVISOR_IPC_READ_TIMEOUT=2
C:\path\to\repo\target\release\nono.exe run --profile claude-code --allow-cwd -- ^
    C:\path\to\repo\target\release\examples\aipc-cap-child.exe sc2
```

**SC1 — reconnect / re-accept + expansion survives:**
```
cd %USERPROFILE%\.claude
C:\path\to\repo\target\release\nono.exe run --profile claude-code --allow-cwd -- ^
    C:\path\to\repo\target\release\examples\aipc-cap-child.exe sc1
```

### PASS criteria for the operator

**SC2 PASS:** The `nono run` process stays responsive and eventually exits (or
can be Ctrl-C'd) after the timeout fires. The supervisor log should contain a
`[timeout]` keep-alive message (e.g. "Supervisor IPC read deadline exceeded
..."). The process does NOT hang indefinitely (which would indicate the
PeekNamedPipe bounded-read is not active). The child prints:
```
sc2: partial frame (4-byte prefix) sent on cap pipe; stalling 20s. Supervisor
read_frame should bound at the configured timeout and keep the supervision loop
responsive (no indefinite hang).
```
Then exits 0 after the stall (or is killed by the supervisor after the
timeout).

**SC1 PASS:** The child prints `SC1 RESULT: PASS`. Additionally the child
may print `expansion survives reconnect: CONFIRMED` if the supervisor approved
the conn2 `request_event`. If the supervisor is configured for approval prompts,
a Denied-but-responded result on either connection also counts as PASS (the
channel is alive, the pipe re-accepted). No `Access is denied` / `ERROR_PIPE_BUSY`
errors in the supervisor output.

### Honest status

`aipc-cap-child.exe` compiles clean, passes clippy (`-D warnings -D
clippy::unwrap_used`), and both selftests PASS on this Windows 11 host (captured
above). The full operator live-repro under `nono run --profile claude-code` is
**first-run success not guaranteed** — the WRITE_RESTRICTED / Low-IL broker
arm is the still-stabilizing path (see Phase 52 notes in [[project_v27_opened]],
Phase 60 UAT defects [[project_sandbox_the_tools]]). Task 3 remains OPEN until
the operator runs it on real Win11 hardware and records PASS/FAIL.

## SC1 / SC2 Live-Repro (PENDING — Task 3, operator `nono run`)

**STATUS: AWAITING OPERATOR UAT** (multi-process helper PASS captured above; full
`nono run` under restricted-token still needed)

The `cap-pipe-live-repro` helper (above) runs clean on this Windows 11 host and proves
the bounded-read and re-accept logic at the multi-process named-pipe layer. What remains
is verifying the same behavior through the full supervisor stack — `nono run` under a
`WRITE_RESTRICTED` child token, Low-IL broker, and the production AIPC pipe SDDL.

### Required Repro (from Task 3)

On a real Win11 console (NOT git-bash/MSYS — supervised runs need a real console; run
`target\release\nono.exe` from a profile-covered cwd to skip the broker trust gate):

1. `cargo build --release -p nono-cli`
2. **SLOW-CHILD (SC2):** Start a supervised `nono run` whose child opens the AIPC pipe, sends a partial frame, then stalls. Set `NONO_SUPERVISOR_IPC_READ_TIMEOUT=2` (2s) for fast observation. Confirm the supervisor is NOT blocked past the deadline.
3. **RECONNECT (SC1):** Have the child close its capability-pipe connection then reconnect. Confirm the capability pipe re-accepts and capability expansion still works. Confirm no `Access is denied` / `ERROR_PIPE_BUSY` errors.

**Resume signal:** Type "approved" with PASS evidence, or describe any observed failure.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added write_raw_bytes() to SupervisorSocket pub surface**
- **Found during:** Task 2 (writing the bounded_read_timeout_via_recv_message integration test)
- **Issue:** The test needed to write a partial frame (length prefix only) from the child side. `SupervisorSocket` had no raw write method — only `send_message()` which sends a complete framed message. Without raw write capability, the partial-frame timeout test was not possible.
- **Fix:** Added `write_raw_bytes(&[u8]) -> Result<()>` pub method to `SupervisorSocket` in `socket_windows.rs`, clearly documented as a test helper.
- **Files modified:** `crates/nono/src/supervisor/socket_windows.rs`
- **Verification:** Test compiles and passes.
- **Committed in:** `51f54fa8` (Task 2 commit)

**2. [Rule 3 - Blocking] Fixed *mut c_void not Send error in integration test thread**
- **Found during:** Task 2 (named-pipe reconnect test — spawning a client thread that returns HANDLE)
- **Issue:** `HANDLE = *mut c_void` is `!Send`; Rust rejected the thread closure returning `HANDLE`.
- **Fix:** Transfer the handle value as `usize` across the thread boundary; re-cast to `HANDLE` in the calling thread. Pattern matches existing worktree usage in the codebase.
- **Files modified:** `crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs`
- **Verification:** Compiles and tests pass.
- **Committed in:** `51f54fa8` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 2 — missing critical test helper; 1 Rule 3 — blocking compile error)
**Impact on plan:** Both fixes necessary for correctness and test completeness. No scope creep.

## Issues Encountered

- Pre-existing `cargo test -p nono-cli` baseline failures (profile_cmd init + protected_paths) confirmed present at the Task-2 base commit — not introduced by this plan (see [[nono_cli_windows_baseline_test_failures]]).

## User Setup Required

None — the automated portion (Tasks 1-2) requires no external service configuration. Task 3 requires a real Win11 console for the live-repro UAT.

## Cross-Target Clippy Note

CLAUDE.md MUST/NEVER rule: files modified in this plan (`socket_windows.rs`, `exec_strategy_windows/supervisor.rs`, `supervisor_ipc_robustness_windows.rs`) are all `#[cfg(target_os = "windows")]`-gated. Cross-target Linux/macOS clippy verification is **PARTIAL / deferred to live CI** — Windows-host `cargo check` does not exercise cross-platform cfg branches, but these files compile exclusively on Windows so the Unix paths have no new code.

## Known Stubs

None — this plan implements concrete functionality, not placeholder UI or data-wiring.

## Threat Flags

No new network endpoints, auth paths, or schema changes introduced. The new surface (bounded read + re-accept) is transport-layer hardening within the existing supervisor IPC channel. All T-59-03a through T-59-03f threats are mitigated as specified.

---
*Phase: 59-supervisor-ipc-robustness*
*Partial completion: 2026-06-06 (Tasks 1-2); Task 3 PENDING operator UAT*
