//! Windows supervisor-IPC robustness integration tests (Phase 59, Wave 0 scaffold).
//!
//! # Cross-platform compile contract
//!
//! This file carries a file-level `#![cfg(target_os = "windows")]` gate. On
//! Linux and macOS, Cargo compiles it to an **empty test binary** (zero tests,
//! always passes). On Windows the real tests run against the `nono` LIBRARY's
//! public surface — specifically the AIPC named-pipe functions in
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
//! This file is OWNED BY 59-03 (Wave 2). Plan 59-01 creates the scaffold;
//! 59-03 fills in the `TODO` placeholders with real test logic.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use nono::supervisor::socket;
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
// TODO placeholders for 59-03 (Wave 2)
// ---------------------------------------------------------------------------

// TODO(59-03, SC4-bounded_read): bounded_read via PeekNamedPipe deadline
//   Test that `nono::supervisor::socket::recv_message()` (which drives the
//   private `read_exact_bounded` helper) returns an error within the configured
//   deadline when the peer sends only the 4-byte length prefix (no payload).
//
//   Implementation pattern:
//   1. `bind_aipc_pipe()` → server_handle; open client via `CreateFileW`.
//   2. Wrap server end in a `SupervisorWindowsSocket` (or use the pub
//      `recv_message` surface directly if exposed).
//   3. Write only 4 length-prefix bytes from client side.
//   4. Call `recv_message()` on server with a short deadline (e.g. 200ms).
//   5. Assert it returns Err within < 1s (bounded, not hung).
//
//   Note: `read_exact_bounded` + `PeekNamedPipe` live inside the library
//   (`socket_windows.rs`) and are driven via the pub `recv_message` surface.
//   The test does NOT call `read_exact_bounded` directly.

// TODO(59-03, SC4-re_accept): transient disconnect → re-accept
//   Test that a transient client disconnect does not permanently disable the
//   AIPC capability pipe. The SC1 bug (`supervisor.rs:561-600` hard-break-on-
//   error) is the regression surface; the fix is a re-accept loop via
//   `DisconnectNamedPipe` + `ConnectNamedPipe` (ERROR_PIPE_CONNECTED idiom).
//
//   Because the re-accept loop lives in `exec_strategy_windows/supervisor.rs`
//   (private CLI code), the integration-test angle is:
//   - Confirm the library's `disconnect_and_reconnect_pipe` helper (if 59-03
//     adds one to the pub surface) handles the ERROR_PIPE_CONNECTED success
//     case correctly.
//   - OR: document the live Win11 repro in 59-03-SUMMARY.md and provide a
//     manual test script (named-pipe timing is not deterministic in CI).
//
//   Security invariant to preserve: `seen_request_ids` replay protection
//   (supervisor.rs:560) must NOT be reset on reconnect; session SID/token
//   must be re-verified on each new connection.
