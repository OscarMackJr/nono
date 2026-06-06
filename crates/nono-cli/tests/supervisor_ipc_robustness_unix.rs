//! Unix supervisor-IPC robustness integration tests (Phase 59, Wave 0 scaffold).
//!
//! # Cross-platform compile contract
//!
//! This file carries a file-level `#![cfg(unix)]` gate. On Windows, Cargo
//! compiles it to an **empty test binary** (zero tests, always passes). On
//! Linux and macOS the real tests run against the `nono` LIBRARY's public
//! surface — specifically `nono::supervisor::socket::SupervisorSocket`.
//!
//! IMPORTANT: this file may ONLY reach the `nono` library's pub surface
//! (`use nono::supervisor::...`). It must NEVER reference `nono_cli::*`,
//! `nono_cli::timeouts::*`, `run_supervisor_loop`, or `read_frame` — all of
//! those symbols are private to the `nono-cli` bin target and are unreachable
//! from an integration test (`nono-cli` declares no `[lib]` target).
//!
//! # Ownership
//!
//! This file is OWNED BY 59-02 (Wave 2). Plan 59-01 creates the scaffold;
//! 59-02 fills in the `TODO` placeholders with real test logic.

#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use nono::supervisor::socket::SupervisorSocket;

/// Wave-0 sanity test: confirm that `SupervisorSocket::pair()` is reachable
/// from the integration-test binary and that both socket ends are produced
/// without error.
///
/// This proves the `nono` LIBRARY pub surface links correctly from the
/// `nono-cli` integration-test crate — the only surface reachable here.
#[test]
fn scaffold_links_nono_lib() {
    let (supervisor_end, child_end) = SupervisorSocket::pair()
        .expect("SupervisorSocket::pair() must succeed on Unix");

    // Assert both ends are usable by obtaining their raw fds.
    // A valid raw fd is >= 0 (a non-negative i32).
    use std::os::unix::io::AsRawFd;
    assert!(
        supervisor_end.as_raw_fd() >= 0,
        "supervisor socket fd must be valid"
    );
    assert!(
        child_end.as_raw_fd() >= 0,
        "child socket fd must be valid"
    );
}

// ---------------------------------------------------------------------------
// TODO placeholders for 59-02 (Wave 2)
// ---------------------------------------------------------------------------

// TODO(59-02, SC1): reconnect_survival
//   Test that the supervisor's poll loop survives a child that closes the IPC
//   socket and reconnects. The macOS arm's hard-break-on-POLLHUP bug (SC1) is
//   the regression surface; the fix is the `sock_fd_active` keep-alive from
//   the Linux arm. Drive via the private `run_supervisor_loop` from an
//   in-crate `#[cfg(test)]` test in `exec_strategy.rs` (NOT here, because
//   `run_supervisor_loop` is private to the bin).
//
//   Integration-test angle (here, 59-02 can add a test using pub surface):
//   Use `SupervisorSocket::pair()` + `set_read_timeout()` + `recv_message()`
//   to simulate a child that closes early and show the supervisor end
//   receives a timeout/disconnect rather than hanging forever.

// TODO(59-02, SC2): bounded_read_timeout
//   Test that `SupervisorSocket::recv_message()` honours the read timeout
//   configured via `set_read_timeout()` and returns an error within the
//   configured deadline when the peer holds a partial frame (slowloris).
//
//   Implementation pattern:
//   1. `SupervisorSocket::pair()` → (supervisor_sock, child_sock).
//   2. `supervisor_sock.set_read_timeout(Some(Duration::from_millis(200)))`.
//   3. In a thread, write only the 4-byte length prefix on child_sock (no
//      payload) to trigger the bounded-read path.
//   4. `supervisor_sock.recv_message()` must return Err within ~200ms.
//   5. Assert elapsed time < 1s (bounded, not hung).
//
//   Set `NONO_SUPERVISOR_IPC_READ_TIMEOUT` via `EnvVarGuard` to a low value
//   and use `supervisor_ipc_read_timeout()` in the in-crate test that wires
//   `set_read_timeout` in `exec_strategy.rs` (59-02 Task 1). The integration
//   test here drives the lib surface directly.
