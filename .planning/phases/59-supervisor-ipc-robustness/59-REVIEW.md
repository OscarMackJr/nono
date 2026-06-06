---
phase: 59-supervisor-ipc-robustness
reviewed: 2026-06-06T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - crates/nono/src/supervisor/socket_windows.rs
  - crates/nono/src/sandbox/windows.rs
  - crates/nono/src/lib.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/timeouts.rs
  - crates/nono-cli/tests/supervisor_ipc_robustness_unix.rs
  - crates/nono-cli/tests/supervisor_ipc_robustness_windows.rs
  - crates/nono-cli/examples/cap-pipe-live-repro.rs
  - crates/nono-cli/examples/aipc-cap-child.rs
findings:
  critical: 0
  warning: 5
  info: 4
  total: 9
status: issues_found
---

# Phase 59: Code Review Report

**Reviewed:** 2026-06-06
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed the Phase 59 supervisor-IPC-robustness changes plus the AppContainer
cap-pipe reachability fix. The security-critical SDDL/DACL surface
(`build_capability_pipe_sddl`, `validate_package_sid_for_sddl`,
`grant_sid_read_on_path`, the package-SID rendezvous read-grant ordering) is
well-constructed: validate-before-embed is enforced, no null/world/AU fallback
exists on any error path, every fallible step is fail-closed, all `unsafe` FFI
carries `// SAFETY:` comments, and arithmetic uses `checked_`/`saturating_`.
The grant-before-blocking-ConnectNamedPipe ordering is correct and the
revert-via-Drop (file deletion destroys the leaf ACE) is sound.

No BLOCKER-class defects were found. The findings below are robustness and
correctness concerns. The two most important (WR-01 and WR-02) concern the
bounded-read defense: a slow-trickle variant can defeat the deadline, and the
Unix keep-alive classification matches error substrings that the actual
`WouldBlock` error Display does not contain. Both remain fail-SECURE (no
capability is granted on a stalled/timed-out frame), so they degrade
availability/keep-alive intent rather than confidentiality — hence WARNING, not
BLOCKER. The Unix arms are `cfg(unix)` and not compiled/tested on this Windows
host; WR-01/WR-02/WR-05 are flagged from reading and must be confirmed in
live CI.

## Warnings

### WR-01: Bounded read can be defeated by a slow byte-trickle (deadline only checked when `avail == 0`)

**File:** `crates/nono/src/supervisor/socket_windows.rs:564-654`
**Issue:** In `read_exact_bounded`, the deadline is only evaluated inside the
`if avail == 0` branch (line 609). When `PeekNamedPipe` reports `avail > 0`,
the function reads the available bytes and loops back WITHOUT re-checking the
deadline. A malicious child can therefore send one byte just before each
`POLL_INTERVAL` (10 ms) tick: `avail` is non-zero on every poll, the deadline
check is never reached, and the read proceeds well past the configured timeout
(`MAX_MESSAGE_SIZE` = 64 KiB ÷ 1 byte/10 ms ≈ 10.9 minutes for a max frame).
This is a slowloris variant that the SC2 bounded-read defense was intended to
prevent. It remains fail-secure (no capability is granted until a complete,
valid frame is parsed), so the impact is supervisor-thread liveness, not an
over-grant.
**Fix:** Check the deadline at the top of every loop iteration, regardless of
`avail`:
```rust
while filled < buf.len() {
    if Instant::now() >= deadline {
        return Err(NonoError::SandboxInit(format!(
            "[timeout] Supervisor IPC read deadline exceeded after {filled} bytes \
             (needed {}); in-flight partial frame discarded (fail-closed)",
            buf.len()
        )));
    }
    // ... existing PeekNamedPipe probe ...
}
```

### WR-02: macOS keep-alive predicate matches substrings absent from the `WouldBlock` error Display

**File:** `crates/nono-cli/src/exec_strategy.rs:2391-2406`
**Issue:** The SC2 keep-alive classifier matches the recv error string against
`"timed out"`, `"WouldBlock"`, and `"would block"`. The Unix `read_frame`
(`crates/nono/src/supervisor/socket.rs:218-235`) wraps the underlying io error
with `format!("Failed to read ... : {e}")`, where `{e}` is the `io::Error`
*Display*. A `SO_RCVTIMEO` timeout surfaces as `ErrorKind::WouldBlock`, whose
Display is the libc strerror text ("Resource temporarily unavailable" for
EAGAIN), NOT the kind name "WouldBlock" nor "timed out"/"would block". The
match therefore evaluates false, the timeout is misclassified as a disconnect,
and the loop demotes `sock_fd` (or breaks when no PTY) instead of treating it
as keep-alive — defeating the stated SC2 "keep supervision alive on a partial
frame" goal. Still fail-secure (no capability granted). The integration test
`bounded_read_timeout` (supervisor_ipc_robustness_unix.rs:128-132) only asserts
`result.is_err()`, so it does not catch this misclassification.
**Fix:** Classify on the `io::ErrorKind`, not on Display substrings. Expose the
kind from `read_frame` (e.g. return a typed error or preserve
`e.kind() == WouldBlock || e.kind() == TimedOut`) and match on that:
```rust
// in socket.rs read_frame, preserve the kind in the error or tag it [timeout]
// then in exec_strategy.rs:
let is_timeout = e.to_string().contains("[timeout]");
```
Tagging the timeout with `[timeout]` (mirroring the Windows arm) is the most
consistent fix and lets the same predicate serve both platforms.

### WR-03: Linux supervised loop never distinguishes a read timeout from a disconnect

**File:** `crates/nono-cli/src/exec_strategy.rs:2609-2618`
**Issue:** `set_read_timeout(...)` is wired for all of `cfg(unix)` (line 1293),
so the Linux supervised loop now also receives `WouldBlock` errors on a partial
frame. Unlike the macOS arm (which at least attempts a timeout/keep-alive
classification), the Linux arm treats every `recv_message` error identically:
demote `sock_fd` (if seccomp/proxy/pty active) or `break`. A slowloris partial
frame thus permanently demotes the capability/IPC channel for the rest of the
session even though the child is still alive. Fail-secure, but the IPC channel
is silently disabled and the behavior is inconsistent with the macOS arm.
**Fix:** Apply the same kind-based timeout classification as WR-02 to the Linux
arm: on a `WouldBlock`/`TimedOut` recv error, log and `continue` (keep-alive)
rather than demoting/breaking.

### WR-04: `seen_request_ids` is mutated before the session-token check, enabling request-ID poisoning / capability denial

**File:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:2074-2091`
**Issue:** The dispatcher inserts a new `request_id` into `seen_request_ids`
(line 2091) BEFORE validating `session_token` (line 2095). A peer that reaches
the pipe but does not hold a valid token can submit a request carrying a
request_id; it is recorded in the replay set, then denied for the bad token.
If the legitimate child later issues a request with that same request_id, it is
rejected as a replay ("Duplicate request_id rejected"). Reaching the pipe is
DACL-gated to the AppContainer/WriteRestricted principal, and request IDs are
child-generated (collision requires guessing), so exploitability is low — but
the ordering inverts the intended invariant (only authenticated requests should
consume the replay namespace).
**Fix:** Move the constant-time token check (lines 2095-2114) ABOVE the
`seen_request_ids.contains/insert` block so an unauthenticated request never
mutates the replay set. Keep the duplicate check after authentication.

### WR-05: Persisted rendezvous file + package-SID ACE survive an abnormal supervisor exit

**File:** `crates/nono/src/supervisor/socket_windows.rs:329-340, 1137-1153`
**Issue:** The package-SID READ grant is applied directly to the rendezvous
file's DACL, and the documented cleanup is "the rendezvous file is deleted in
`SupervisorSocket::Drop`". If the supervisor process is killed (or panics
across the FFI boundary) before `Drop` runs, the rendezvous file persists on
disk WITH the per-run package-SID allow-ACE. The package SID is per-run and
unique, so the residual exposure is small, but stale per-run rendezvous files
accumulate under `%TEMP%`/the rendezvous dir and the cleanup is best-effort
only. (This mirrors the existing `disconnect_on_drop` best-effort pattern.)
**Fix:** Document the residual-file behavior in the rendezvous helper, and
consider a startup sweep that removes stale `nono` rendezvous files older than
a session bound (the unique-nonce naming already prevents reuse collisions).
Not blocking, but should be tracked.

## Info

### IN-01: `read_exact_bounded` classifies ALL non-disconnect Peek/read GLEs as `[disconnect]`

**File:** `crates/nono/src/supervisor/socket_windows.rs:601-604, 638-642`
**Issue:** Any GLE that is not `ERROR_BROKEN_PIPE`/`ERROR_PIPE_NOT_CONNECTED`/
`ERROR_NO_DATA` is folded into the `[disconnect]` class with the message
"unexpected error". This is fail-secure (the loop re-accepts or terminates,
never grants), but a genuinely transient or programming error (e.g. an invalid
handle) is indistinguishable from a peer close in the logs and triggers a
needless re-accept attempt.
**Fix:** Consider a distinct `[error]` tag for non-disconnect GLEs so the
supervision loop can fail-closed (terminate) rather than re-accept on a
non-recoverable error, and so logs disambiguate the two.

### IN-02: `SUPERVISOR_IPC_READ_TIMEOUT` / `supervisor_ipc_read_timeout` carry `#[allow(dead_code)]`

**File:** `crates/nono-cli/src/timeouts.rs:83-97`
**Issue:** Both the const and the function are annotated `#[allow(dead_code)]`,
which CLAUDE.md (§ "Lazy use of dead code") asks to avoid. They are in fact
used (exec_strategy.rs:1293, supervisor.rs:590), so on at least one target the
allow is unnecessary; on a target where neither arm compiles them they would be
genuinely dead. The allow masks which case is true per target.
**Fix:** Replace the blanket `#[allow(dead_code)]` with target-scoped
`#[cfg(...)]` (or remove the allow if both are always reachable under
`cfg(any(unix, windows))`), so an actually-unused symbol surfaces as a warning.

### IN-03: Duplicated `unique_pipe_name`/`selftest_pipe_name` and `connect_with_retry` across the two examples

**File:** `crates/nono-cli/examples/cap-pipe-live-repro.rs:339-363`,
`crates/nono-cli/examples/aipc-cap-child.rs:430-454`
**Issue:** `connect_with_retry` (50× / 100 ms) and the PID+nanos pipe-name
builder are near-identical copies in both example harnesses. Minor duplication;
example code, so low priority.
**Fix:** Optional — factor the shared helper into a small `examples/common`
module if these grow.

### IN-04: `disconnect_and_reconnect` doc references a non-resolvable doctest import path

**File:** `crates/nono/src/supervisor/socket_windows.rs:419-428`
**Issue:** The `no_run` doc example for `recv_message_with_timeout` imports
`nono::supervisor::socket::SupervisorSocket` and constructs it via
`unimplemented!()`. This is fine as a `no_run` snippet, but the module is
`socket_windows`; confirm the re-export path used in the doc matches the public
path on all targets so the doctest does not silently skip or fail on non-Windows
doc builds.
**Fix:** Verify the doc path against `lib.rs` re-exports; adjust if the canonical
public path differs.

---

_Reviewed: 2026-06-06_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
