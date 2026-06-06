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
// 59-02 (Wave 2): SC2 bounded_read_timeout integration test
// ---------------------------------------------------------------------------

/// SC2 (Phase 59-02): Verify that `SupervisorSocket::recv_message()` honours
/// the read timeout set via `set_read_timeout()` and returns a bounded error
/// when the peer holds a partial frame (slowloris-style stall).
///
/// This test drives only the `nono` LIBRARY's public surface:
/// `SupervisorSocket::pair()`, `set_read_timeout()`, and `recv_message()`.
/// It never references `nono_cli::*` (which is bin-only and unreachable here).
///
/// # Protocol
///
/// The child side writes only the 4-byte length prefix (payload length = 100)
/// but never sends the payload bytes. The supervisor side calls
/// `recv_message()`, which internally calls `read_exact` for the 4-byte
/// header (succeeds) and then `read_exact` for the payload (stalls).
/// The 1-second read timeout fires, and `recv_message()` returns an error.
///
/// # Acceptance criteria
///
/// - `recv_message()` returns `Err` (not `Ok`).
/// - The elapsed time is ≥ 800ms (the timeout actually fired) and < 5s
///   (bounded — did not block indefinitely).
/// - No capability is granted on a partial frame (fail-closed: the error is
///   surfaced to the caller, not silently swallowed).
#[test]
fn bounded_read_timeout() {
    use std::thread;
    use std::time::{Duration, Instant};

    // We use UnixStream::pair() for the raw socket pair so that the child end
    // can inject a partial frame (header only, no payload) using std::io::Write.
    // SupervisorSocket::send_message() would send a complete frame, which is
    // not what we want for the slowloris simulation.
    //
    // The supervisor end is wrapped in SupervisorSocket::from_stream() so we can
    // call set_read_timeout() and recv_message() via the library's public API.
    let (supervisor_raw, child_raw) = std::os::unix::net::UnixStream::pair()
        .expect("UnixStream::pair() must succeed on Unix");
    let mut supervisor_sock = SupervisorSocket::from_stream(supervisor_raw);

    // Wire a 1-second read timeout on the supervisor end (SC2).
    // This matches the production wiring in exec_strategy.rs (set_read_timeout
    // called with supervisor_ipc_read_timeout() — 5s by default, overridable).
    // We use 1s here to keep CI fast while still Nyquist-sampling above the
    // 200ms poll tick (must wait > 200ms for the timeout to deterministically fire).
    supervisor_sock
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set_read_timeout must succeed on Unix");

    // Spawn a thread that acts as the child: write a 4-byte length prefix
    // announcing a 100-byte payload, then stall (never send the payload).
    // This simulates a slowloris partial-frame attack on the supervisor.
    let child_handle = thread::spawn(move || {
        use std::io::Write;
        // Write only the length prefix (u32 big-endian = 100) via the raw stream.
        // SupervisorSocket::send_message() would send a complete frame; we want
        // to inject a partial frame (header only, no payload) to trigger the
        // bounded-read path in recv_message() → read_exact(payload).
        let len_bytes: [u8; 4] = 100u32.to_be_bytes();
        let mut child = child_raw;
        // Write the 4-byte length prefix. The supervisor reads this successfully
        // and then waits for 100 payload bytes that never arrive.
        let _ = child.write_all(&len_bytes);
        // Stall: hold the socket open so the supervisor's payload read_exact
        // blocks waiting for 100 bytes that never arrive. Hold for 3s so the
        // 1s timeout fires first.
        thread::sleep(Duration::from_secs(3));
        // child drops here, closing the socket.
        drop(child);
    });

    let t0 = Instant::now();
    let result = supervisor_sock.recv_message();
    let elapsed = t0.elapsed();

    // SC2 acceptance: recv_message must return an error (the timeout fired).
    assert!(
        result.is_err(),
        "recv_message() should return an error after the read timeout fires, got Ok"
    );

    // The error must have fired within a bounded window.
    // Lower bound: 800ms (timeout is 1s; allow 200ms jitter for slow CI).
    // Upper bound: 5s (must not have blocked indefinitely).
    assert!(
        elapsed >= Duration::from_millis(800),
        "recv_message() returned too quickly ({}ms); read timeout may not have fired",
        elapsed.as_millis()
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "recv_message() blocked too long ({}ms); likely blocked indefinitely",
        elapsed.as_millis()
    );

    // Fail-closed invariant: the error is surfaced to the caller. No capability
    // was granted on a partial frame (this is enforced architecturally: the
    // supervisor's recv_message caller checks the Result before acting on any
    // message content).

    child_handle.join().expect("child thread should not panic");
}
