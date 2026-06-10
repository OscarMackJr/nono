//! Windows supervisor-IPC robustness integration tests (Phase 59, Wave 2).
//!
//! # Cross-platform compile contract
//!
//! This file carries a file-level `#![cfg(target_os = "windows")]` gate. On
//! Linux and macOS, Cargo compiles it to an **empty test binary** (zero tests,
//! always passes). On Windows the real tests run against the `nono` LIBRARY's
//! public surface — specifically the supervisor socket functions in
//! `nono::supervisor::socket`.
//!
//! IMPORTANT: this file may ONLY reach the `nono` library's pub surface
//! (`use nono::supervisor::...`). It must NEVER reference `nono_cli::*`,
//! `nono_cli::timeouts::*`, `run_supervisor_loop`, `read_frame`, or
//! `read_exact_bounded` — all of those symbols are private to the `nono-cli`
//! bin target and are unreachable from an integration test (`nono-cli`
//! declares no `[lib]` target).
//!
//! # Ownership
//!
//! This file is OWNED BY 59-03 (Wave 2). Plan 59-01 created the scaffold;
//! 59-03 fills in the real test logic.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use nono::supervisor::socket;
use nono::supervisor::socket::SupervisorSocket;
use nono::supervisor::PipeDirection;

/// Wave-0 sanity test: confirm that the AIPC named-pipe bind function is
/// reachable from the integration-test binary and that a pipe server can be
/// created without error.
///
/// This proves the `nono` LIBRARY's Windows pub surface links correctly from
/// the `nono-cli` integration-test crate — the only surface reachable here.
#[test]
fn scaffold_links_nono_lib() {
    // Use a unique pipe name scoped to this test process to avoid collisions.
    let pipe_name = format!(
        r"\\.\pipe\nono-ipc-robustness-scaffold-{}",
        std::process::id()
    );

    // `bind_aipc_pipe` is the library-level function that creates the named
    // pipe with the Phase-11 Low-IL SDDL — the same function exercised by
    // `aipc_handle_brokering_integration.rs`. Creating the server end proves
    // the Windows AIPC pub surface links from the integration test crate.
    let server_handle = socket::bind_aipc_pipe(&pipe_name, PipeDirection::Read)
        .expect("bind_aipc_pipe must succeed for the scaffold sanity test");

    // Close the handle. SAFETY: server_handle is a live pipe handle returned
    // by bind_aipc_pipe; CloseHandle cleans up the kernel object.
    unsafe {
        windows_sys::Win32::Foundation::CloseHandle(server_handle);
    }
}

// ---------------------------------------------------------------------------
// Task 2 (59-03, SC2 / SC4): bounded_read via PeekNamedPipe deadline
// ---------------------------------------------------------------------------
//
// Test that `nono::supervisor::socket::SupervisorSocket::recv_message_with_timeout()`
// (which drives the private `read_exact_bounded` helper) returns an error
// within the configured deadline when the peer sends only the 4-byte length
// prefix (no payload).
//
// REACHABILITY: the test uses `SupervisorSocket::pair()` (the anonymous-pipe
// pair constructor) to obtain a `SupervisorSocket` on the server side. It
// then writes a partial frame (4-byte length prefix only) via the child
// side's `write_raw_bytes()` pub method, and calls
// `recv_message_with_timeout(~1s)` on the server side, asserting bounded
// return rather than indefinite blocking.
//
// `PeekNamedPipe` works on anonymous pipes (they are kernel pipe objects) as
// well as named pipes, so the `read_exact_bounded` poll loop is exercised
// correctly.
#[test]
fn bounded_read_timeout_via_recv_message() {
    use std::time::{Duration, Instant};

    // Create a connected anonymous-pipe pair. The server end (supervisor) is
    // where we call recv_message_with_timeout; the child end is where we send
    // only a partial frame (length prefix, no payload).
    let (mut server, mut child) =
        SupervisorSocket::pair().expect("SupervisorSocket::pair must succeed");

    // The partial frame: a 4-byte big-endian length prefix claiming 64 bytes
    // of payload, followed by silence (child never sends the payload).
    let partial_len: u32 = 64;
    let partial_frame_header = partial_len.to_be_bytes();

    // Write only the 4-byte length prefix from the child side using the
    // `write_raw_bytes` pub helper (bypasses normal message framing).
    child
        .write_raw_bytes(&partial_frame_header)
        .expect("write partial frame header must succeed");

    // Drive recv_message_with_timeout with a 1-second deadline so the CI run
    // is fast. The bounded read MUST return an Err (timeout) within ~ the
    // deadline rather than blocking indefinitely.
    let short_timeout = Duration::from_secs(1);
    let t0 = Instant::now();
    let result = server.recv_message_with_timeout(short_timeout);
    let elapsed = t0.elapsed();

    // The result MUST be an error (partial frame → timeout)
    assert!(
        result.is_err(),
        "recv_message_with_timeout with a partial frame must return Err, got Ok"
    );
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("[timeout]"),
        "Error must be tagged [timeout], got: {err_str}"
    );

    // The elapsed time should be within the timeout (+1000ms slack for CI jitter).
    // This is the key property: the read returned rather than blocking indefinitely.
    // Note: CI timing caveat — the deterministic sub-1s timeout proof is in the
    // operator live-repro (Task 3 / D-05). CI asserts bounded return within ~2s.
    let upper_bound = short_timeout + Duration::from_millis(1000);
    assert!(
        elapsed <= upper_bound,
        "recv_message_with_timeout must return within ~{:?}, actually took {:?}",
        upper_bound,
        elapsed,
    );
}

// ---------------------------------------------------------------------------
// Task 2 (59-03, SC1 / SC4): transient disconnect → disconnect_and_reconnect
// ---------------------------------------------------------------------------
//
// Test that `SupervisorSocket::disconnect_and_reconnect()` can be called
// without panicking and returns a `Result`. This exercises the pub API
// surface that the supervisor loop uses to re-arm the capability pipe after
// a transient child close.
//
// On an anonymous-pipe pair, `DisconnectNamedPipe` is expected to fail (not a
// named-pipe server handle) — the test asserts no panic and a Result return
// (either Ok or Err is acceptable). The full named-pipe round-trip is in
// `capability_pipe_reconnect_named_pipe` below.
//
// Security invariant (V3): `disconnect_and_reconnect()` resets transport only.
// The caller's responsibility for preserving `seen_request_ids` and
// re-verifying the session SID/token on every incoming message (both of which
// happen in `handle_windows_supervisor_message`) is documented in
// `socket_windows.rs` and the SUMMARY.
#[test]
fn disconnect_and_reconnect_method_is_reachable() {
    // The disconnect_and_reconnect() method is on SupervisorSocket.
    // For anonymous pairs, DisconnectNamedPipe returns an error (expected).
    // The important property: no panic, returns Result.
    let (mut server, _child) =
        SupervisorSocket::pair().expect("SupervisorSocket::pair must succeed");

    // Calling disconnect_and_reconnect on an anonymous pipe is expected to
    // return Err (anonymous pipes are not named-pipe server handles).
    // Either Ok or Err is acceptable — the key property is no panic.
    let _result = server.disconnect_and_reconnect();
    // result may be Ok or Err — both acceptable for this API-surface test.
}

// ---------------------------------------------------------------------------
// Task 2 (59-03, SC1 / SC4): named-pipe server disconnect + reconnect
// ---------------------------------------------------------------------------
//
// A more complete test of the re-accept path using a named pipe created via
// `bind_aipc_pipe` (raw HANDLE). The test:
//
// 1. Binds an AIPC named pipe (server handle, PIPE_UNLIMITED_INSTANCES).
// 2. Connects a first client via `CreateFileW`, then closes it.
// 3. Calls `DisconnectNamedPipe` + `ConnectNamedPipe` on the server handle to
//    re-arm (mirrors `disconnect_and_reconnect` internals).
// 4. Connects a second client, asserts the server re-accepted (not ERROR_PIPE_BUSY).
//
// This test validates the raw pipe re-accept idiom that
// `SupervisorSocket::disconnect_and_reconnect()` wraps. The full supervisor-
// level end-to-end (with session token re-verify + `seen_request_ids`
// preservation) requires a live operator repro (Task 3 / SC1 live-repro).
//
// Pattern mirrors `aipc_handle_brokering_integration.rs`: explicit `// SAFETY:`
// on every FFI call, `last_os_error()` in assert messages, close handles.
#[test]
fn capability_pipe_reconnect_named_pipe() {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, OPEN_EXISTING};
    use windows_sys::Win32::System::Pipes::{ConnectNamedPipe, DisconnectNamedPipe};

    let pipe_name = format!(r"\\.\pipe\nono-ipc-reconnect-{}-{}", std::process::id(), {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    });
    let wide_name: Vec<u16> = pipe_name.encode_utf16().chain(std::iter::once(0)).collect();

    // Step 1: Create the AIPC pipe server handle (PIPE_UNLIMITED_INSTANCES,
    // PIPE_WAIT, Low-IL SDDL).
    let server_handle: HANDLE = socket::bind_aipc_pipe(&pipe_name, PipeDirection::Read)
        .expect("bind_aipc_pipe must succeed");

    // Step 2: First client connects (CreateFileW → pipe goes to "connected" state).
    // SAFETY: `wide_name` is a valid null-terminated UTF-16 string. `CreateFileW`
    // returns an owned HANDLE on success, INVALID_HANDLE_VALUE on failure.
    let client1: HANDLE = unsafe {
        CreateFileW(
            wide_name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    assert!(
        client1 != INVALID_HANDLE_VALUE,
        "First client CreateFileW failed: {}",
        std::io::Error::last_os_error()
    );

    // Close the first client — simulates a transient child close (SC1 scenario).
    // SAFETY: `client1` is a live pipe handle returned by CreateFileW above.
    unsafe { CloseHandle(client1) };

    // Step 3: Disconnect + reconnect (re-arm the SAME server handle —
    // Pitfall 3 avoidance: do NOT create a new pipe instance for 1-instance
    // control pipes; AIPC uses PIPE_UNLIMITED_INSTANCES but the idiom is the same).
    //
    // SAFETY: `server_handle` is a live named-pipe server handle from
    // `bind_aipc_pipe`. `DisconnectNamedPipe` flushes and severs the connection;
    // the handle remains valid for the subsequent `ConnectNamedPipe` call.
    let dc = unsafe { DisconnectNamedPipe(server_handle) };
    assert!(
        dc != 0,
        "DisconnectNamedPipe must succeed after client close: {}",
        std::io::Error::last_os_error()
    );

    // Spawn the second client in a background thread so we can call
    // ConnectNamedPipe (blocking) on the server without deadlock.
    // Transfer HANDLE as usize (Send-safe integer), re-cast to HANDLE in thread.
    let wide2 = wide_name.clone();
    let client2_thread = std::thread::spawn(move || -> usize {
        // Give the server a moment to reach ConnectNamedPipe.
        std::thread::sleep(std::time::Duration::from_millis(50));
        // SAFETY: `wide2` is a valid null-terminated UTF-16 pipe name.
        let h: HANDLE = unsafe {
            CreateFileW(
                wide2.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        h as usize
    });

    // SAFETY: `server_handle` is live. `ConnectNamedPipe` blocks until a
    // client connects. `ERROR_PIPE_CONNECTED` (535) means a client raced in —
    // treat as success (the standard idiom from `finalize_server_connection`).
    let cn = unsafe { ConnectNamedPipe(server_handle, std::ptr::null_mut()) };
    if cn == 0 {
        let gle = std::io::Error::last_os_error();
        assert_eq!(
            gle.raw_os_error(),
            Some(windows_sys::Win32::Foundation::ERROR_PIPE_CONNECTED as i32),
            "ConnectNamedPipe after re-arm must succeed or return ERROR_PIPE_CONNECTED, got: {gle}"
        );
    }

    // Step 4: Second client arrived — confirm re-accept (not ERROR_PIPE_BUSY).
    let client2_usize = client2_thread
        .join()
        .expect("client2 thread must not panic");
    let client2: HANDLE = client2_usize as HANDLE;
    assert!(
        client2 != INVALID_HANDLE_VALUE,
        "Second client CreateFileW must succeed after re-accept: {}",
        std::io::Error::last_os_error()
    );

    // Cleanup. SAFETY: both handles are live.
    unsafe {
        CloseHandle(client2);
        CloseHandle(server_handle);
    }
}
