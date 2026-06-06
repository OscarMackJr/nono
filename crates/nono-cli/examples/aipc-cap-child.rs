//! AIPC-SDK child harness for Phase 59-03 live `nono run` cap-pipe repro.
//!
//! When launched as the child process under `nono run --profile claude-code`,
//! this binary drives the real capability pipe (rendezvous path injected by the
//! supervisor as `NONO_CAP_FILE`) to exercise:
//!
//! - **SC2** (`sc2` mode): partial-frame / bounded-read. Connects to the cap
//!   pipe, sends a 4-byte big-endian length prefix announcing 64 bytes (payload
//!   never sent), then stalls for 20 s. Proves the supervisor's bounded
//!   `PeekNamedPipe` deadline keeps the supervision loop responsive — no
//!   indefinite hang.
//!
//! - **SC1** (`sc1` mode): transient close → re-accept + expansion survives.
//!   Makes two connections (conn1 then conn2) separated by a deliberate drop +
//!   500 ms pause. On each connection sends a real AIPC `request_event` via the
//!   SDK and reports whether the supervisor re-accepted the second connection
//!   (capability pipe not permanently disabled).
//!
//! - **`--selftest <sc1|sc2>`**: local self-test mode that proves the child-side
//!   connect / write / reconnect mechanics WITHOUT requiring `nono run`. A
//!   background thread acts as a minimal supervisor mimic (bind-side). Does NOT
//!   exercise the SDK request path or the WRITE_RESTRICTED / Low-IL broker token.
//!
//! ## Cross-platform stub
//!
//! Named-pipe AIPC is Windows-only. On other platforms the binary compiles as a
//! no-op stub so `cargo build --examples` stays clean everywhere.
//!
//! ## Operator live-repro commands (Win11 console, dev-layout)
//!
//! ```text
//! cargo build --release --example aipc-cap-child -p nono-cli
//! # SC2 (partial frame / bounded read):
//! target\release\nono.exe run --profile claude-code --allow-cwd -- ^
//!     target\release\examples\aipc-cap-child.exe sc2
//! # SC1 (reconnect / re-accept):
//! target\release\nono.exe run --profile claude-code --allow-cwd -- ^
//!     target\release\examples\aipc-cap-child.exe sc1
//! ```
//!
//! Run from a profile-covered cwd (e.g. `%USERPROFILE%\.claude`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

// -----------------------------------------------------------------------
// Non-Windows stub
// -----------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("aipc-cap-child: named-pipe AIPC is Windows-only; stub on other platforms.");
}

// -----------------------------------------------------------------------
// Windows implementation
// -----------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn main() -> std::process::ExitCode {
    windows_impl::run()
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use nono::supervisor::aipc_sdk;
    use nono::supervisor::socket::SupervisorSocket;
    use std::env;
    use std::path::Path;
    use std::process::ExitCode;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    // Access mask for `request_event` calls: SYNCHRONIZE | EVENT_MODIFY_STATE.
    // nono::supervisor::policy::EVENT_DEFAULT_MASK is pub but lives in the nono
    // crate behind #[cfg(target_os = "windows")]; use the literal so the import
    // stays tidy. The supervisor validates this against its own allowlist.
    const EVENT_ACCESS_MASK: u32 = 0x0010_0002; // SYNCHRONIZE | EVENT_MODIFY_STATE

    // -----------------------------------------------------------------------
    // Entry point
    // -----------------------------------------------------------------------

    pub(super) fn run() -> ExitCode {
        let args: Vec<String> = env::args().collect();

        // --selftest <sc1|sc2>
        if let Some(pos) = args.iter().position(|a| a == "--selftest") {
            let sc = args
                .get(pos + 1)
                .map(String::as_str)
                .unwrap_or_else(|| panic!("--selftest requires a <sc1|sc2> argument"));
            return run_selftest(sc);
        }

        // First non-flag arg, or --mode <m>
        let mode = args
            .iter()
            .position(|a| a == "--mode")
            .and_then(|i| args.get(i + 1).cloned())
            .or_else(|| {
                // First positional arg that doesn't start with '-'
                args.iter()
                    .skip(1)
                    .find(|a| !a.starts_with('-'))
                    .cloned()
            });

        match mode.as_deref() {
            Some("sc1") => run_sc1(),
            Some("sc2") => run_sc2(),
            other => {
                eprintln!(
                    "aipc-cap-child: unknown mode {:?}. Use: sc1 | sc2 | --selftest <sc1|sc2>",
                    other.unwrap_or("<none>")
                );
                ExitCode::FAILURE
            }
        }
    }

    // -----------------------------------------------------------------------
    // SC2 — partial frame / bounded read (real nono run child)
    // -----------------------------------------------------------------------

    fn run_sc2() -> ExitCode {
        let cap_file = match env::var("NONO_CAP_FILE") {
            Ok(v) => v,
            Err(_) => {
                eprintln!(
                    "aipc-cap-child sc2 must be launched as a `nono run` child; \
                     NONO_CAP_FILE not set"
                );
                return ExitCode::FAILURE;
            }
        };

        let mut sock = match SupervisorSocket::connect(Path::new(&cap_file)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("sc2: connect failed: {e}");
                return ExitCode::FAILURE;
            }
        };

        // Send a 4-byte big-endian length prefix claiming 64 bytes, but never
        // send the payload. This creates the partial-frame condition that the
        // supervisor's PeekNamedPipe deadline poll must detect and bound.
        if let Err(e) = sock.write_raw_bytes(&64u32.to_be_bytes()) {
            eprintln!("sc2: write_raw_bytes (partial frame prefix) failed: {e}");
            return ExitCode::FAILURE;
        }

        println!(
            "sc2: partial frame (4-byte prefix) sent on cap pipe; stalling 20s. \
             Supervisor read_frame should bound at the configured timeout and keep \
             the supervision loop responsive (no indefinite hang)."
        );

        // Stall — never send the payload.
        thread::sleep(Duration::from_secs(20));
        ExitCode::SUCCESS
    }

    // -----------------------------------------------------------------------
    // SC1 — transient close → re-accept + expansion survives (real nono run child)
    // -----------------------------------------------------------------------

    fn run_sc1() -> ExitCode {
        let cap_file = match env::var("NONO_CAP_FILE") {
            Ok(v) => v,
            Err(_) => {
                eprintln!(
                    "aipc-cap-child sc1 must be launched as a `nono run` child; \
                     NONO_CAP_FILE not set"
                );
                return ExitCode::FAILURE;
            }
        };

        // --- conn1 ---
        let conn1_decision = {
            let mut sock1 = match SupervisorSocket::connect(Path::new(&cap_file)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("sc1 conn1: connect failed: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let result = aipc_sdk::request_event(
                &mut sock1,
                "nono-aipc-cap-child-sc1-probe-1",
                EVENT_ACCESS_MASK,
                Some("sc1 conn1 probe"),
            );
            classify_request_event_result("conn1", result)
            // sock1 dropped here → conn1 severed (transient close)
        };

        println!("sc1 conn1: {conn1_decision}");

        // Brief pause so the supervisor can observe the disconnect
        thread::sleep(Duration::from_millis(500));

        // --- conn2 ---
        let mut sock2 = match SupervisorSocket::connect(Path::new(&cap_file)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "SC1 RESULT: FAIL (conn2 connect failed: {e} — \
                     cap pipe did not re-accept after the transient close)"
                );
                return ExitCode::FAILURE;
            }
        };
        let conn2_decision = classify_request_event_result(
            "conn2",
            aipc_sdk::request_event(
                &mut sock2,
                "nono-aipc-cap-child-sc1-probe-2",
                EVENT_ACCESS_MASK,
                Some("sc1 conn2 probe after reconnect"),
            ),
        );
        println!("sc1 conn2: {conn2_decision}");

        // PASS iff conn2 received a Decision (Approved or Denied-but-responded)
        // — that proves the cap pipe RE-ACCEPTED after the transient close.
        let pass = !conn2_decision.starts_with("TRANSPORT ERROR");
        if pass {
            if conn2_decision.contains("GRANTED") {
                println!("expansion survives reconnect: CONFIRMED");
            }
            println!("SC1 RESULT: PASS");
            ExitCode::SUCCESS
        } else {
            println!("SC1 RESULT: FAIL (transport error on conn2 — cap pipe did not re-accept)");
            ExitCode::FAILURE
        }
    }

    /// Classify an `aipc_sdk::request_event` result into a human-readable string.
    ///
    /// Returns a string starting with:
    /// - `"GRANTED"` — supervisor approved and returned a handle
    /// - `"DENIED-but-responded"` — supervisor returned a Denied decision (channel alive)
    /// - `"TRANSPORT ERROR: ..."` — connection-level failure (not a supervisor decision)
    fn classify_request_event_result(label: &str, result: nono::Result<u64>) -> String {
        match result {
            Ok(_handle) => {
                // We received a handle. In an example we intentionally leak it
                // (we have no DuplicateHandle / CloseHandle import) — acceptable
                // for a test harness. Note the leak so it's obvious.
                // NOTE: leaking the HANDLE is intentional in this example harness;
                // the OS reclaims it when the child process exits.
                format!("{label}: GRANTED (handle received; intentionally not closed in example)")
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("supervisor denied") {
                    format!("{label}: DENIED-but-responded (channel alive): {msg}")
                } else {
                    format!("{label}: TRANSPORT ERROR: {msg}")
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // --selftest <sc1|sc2>
    //
    // Proves child-side connect / write / reconnect mechanics over a local
    // named pipe. No `nono run`, no WRITE_RESTRICTED token, no SDK dispatcher.
    // The supervisor mimic runs in a background thread.
    // -----------------------------------------------------------------------

    fn run_selftest(sc: &str) -> ExitCode {
        match sc {
            "sc1" => run_selftest_sc1(),
            "sc2" => run_selftest_sc2(),
            other => {
                eprintln!("--selftest: unknown scenario {other:?}; use sc1 or sc2");
                ExitCode::FAILURE
            }
        }
    }

    // -- Selftest SC2: mimic binds, child sends partial frame, mimic asserts [timeout] --

    fn run_selftest_sc2() -> ExitCode {
        let pipe_name = selftest_pipe_name();

        // #[allow(clippy::disallowed_methods)] — set_var is safe in a single-
        // threaded selftest context; the only side effect is setting NONO_CAP_FILE
        // for this process, which is exactly what we need here.
        #[allow(clippy::disallowed_methods)]
        unsafe {
            env::set_var("NONO_CAP_FILE", &pipe_name);
        }

        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        let mimic_pipe = pipe_name.clone();

        // Background mimic: bind, wait for the client to connect, call
        // recv_message_with_timeout(2s), assert the error contains [timeout].
        let mimic = thread::spawn(move || {
            let mut server = match SupervisorSocket::bind(Path::new(&mimic_pipe)) {
                Ok(s) => s,
                Err(e) => {
                    tx.send(Err(format!("mimic SC2: bind failed: {e}"))).ok();
                    return;
                }
            };
            let t0 = Instant::now();
            // Use a 2s timeout so the selftest completes quickly.
            match server.recv_message_with_timeout(Duration::from_secs(2)) {
                Err(e) => {
                    let msg = e.to_string();
                    let elapsed = t0.elapsed();
                    if msg.contains("[timeout]") && elapsed <= Duration::from_secs(4) {
                        tx.send(Ok(())).ok();
                    } else {
                        tx.send(Err(format!(
                            "mimic SC2: expected [timeout] within 4s, got \"{msg}\" \
                             after {elapsed:.2?}"
                        )))
                        .ok();
                    }
                }
                Ok(_) => {
                    tx.send(Err(
                        "mimic SC2: recv_message_with_timeout succeeded unexpectedly".to_string(),
                    ))
                    .ok();
                }
            }
        });

        // Child side: connect (with retry), send 4-byte partial prefix, stall briefly.
        let mut sock = match connect_with_retry(&pipe_name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("selftest SC2 child: connect failed: {e}");
                // Mimic will block; give it a moment then collect its result.
                let _ = mimic.join();
                return ExitCode::FAILURE;
            }
        };

        if let Err(e) = sock.write_raw_bytes(&64u32.to_be_bytes()) {
            eprintln!("selftest SC2 child: write_raw_bytes failed: {e}");
            let _ = mimic.join();
            return ExitCode::FAILURE;
        }

        // Stall long enough for the mimic's 2s timeout to fire.
        thread::sleep(Duration::from_millis(2500));
        drop(sock);

        mimic.join().ok();

        match rx.recv() {
            Ok(Ok(())) => {
                println!("SELFTEST sc2 RESULT: PASS");
                ExitCode::SUCCESS
            }
            Ok(Err(msg)) => {
                eprintln!("SELFTEST sc2 RESULT: FAIL ({msg})");
                ExitCode::FAILURE
            }
            Err(_) => {
                eprintln!("SELFTEST sc2 RESULT: FAIL (mimic thread channel dropped)");
                ExitCode::FAILURE
            }
        }
    }

    // -- Selftest SC1: mimic binds, child connects (conn1) + drops + reconnects (conn2) --

    fn run_selftest_sc1() -> ExitCode {
        let pipe_name = selftest_pipe_name();

        // #[allow(clippy::disallowed_methods)] — set_var is safe in a single-
        // threaded selftest context before the mimic thread is spawned.
        #[allow(clippy::disallowed_methods)]
        unsafe {
            env::set_var("NONO_CAP_FILE", &pipe_name);
        }

        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        let mimic_pipe = pipe_name.clone();

        // Background mimic: bind (accepts conn1), then disconnect_and_reconnect()
        // (accepts conn2). Both Ok → PASS.
        let mimic = thread::spawn(move || {
            let mut server = match SupervisorSocket::bind(Path::new(&mimic_pipe)) {
                Ok(s) => s,
                Err(e) => {
                    tx.send(Err(format!("mimic SC1: bind (conn1) failed: {e}"))).ok();
                    return;
                }
            };
            // conn1 is established. Now re-arm for conn2.
            match server.disconnect_and_reconnect() {
                Ok(()) => {
                    tx.send(Ok(())).ok();
                }
                Err(e) => {
                    tx.send(Err(format!("mimic SC1: disconnect_and_reconnect failed: {e}"))).ok();
                }
            }
        });

        // Child conn1: connect, then drop (trigger disconnect).
        {
            let _sock1 = match connect_with_retry(&pipe_name) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("selftest SC1 conn1: connect failed: {e}");
                    let _ = mimic.join();
                    return ExitCode::FAILURE;
                }
            };
            // _sock1 dropped here → conn1 severed.
        }

        // Brief pause so the mimic observes the disconnect.
        thread::sleep(Duration::from_millis(300));

        // Child conn2: reconnect (mimic is blocking on disconnect_and_reconnect →
        // ConnectNamedPipe, waiting for this).
        let _sock2 = match connect_with_retry(&pipe_name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("selftest SC1 conn2: reconnect failed: {e}");
                let _ = mimic.join();
                return ExitCode::FAILURE;
            }
        };

        // Hold conn2 briefly so mimic's ConnectNamedPipe returns before the
        // child drops the socket.
        thread::sleep(Duration::from_millis(300));
        drop(_sock2);

        mimic.join().ok();

        match rx.recv() {
            Ok(Ok(())) => {
                println!("SELFTEST sc1 RESULT: PASS");
                ExitCode::SUCCESS
            }
            Ok(Err(msg)) => {
                eprintln!("SELFTEST sc1 RESULT: FAIL ({msg})");
                ExitCode::FAILURE
            }
            Err(_) => {
                eprintln!("SELFTEST sc1 RESULT: FAIL (mimic thread channel dropped)");
                ExitCode::FAILURE
            }
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Connect to the named pipe with up to 50 retries / 100 ms apart to handle
    /// the bind-before-connect startup race (the pipe is created during `bind()`,
    /// which may not have run yet when the child first calls `connect()`).
    fn connect_with_retry(pipe_name: &str) -> nono::Result<SupervisorSocket> {
        let mut last_err: Option<nono::NonoError> = None;
        for attempt in 0..50u32 {
            match SupervisorSocket::connect(Path::new(pipe_name)) {
                Ok(sock) => return Ok(sock),
                Err(e) => {
                    last_err = Some(e);
                    if attempt < 49 {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
        Err(last_err.expect("at least one connect attempt was made"))
    }

    /// Build a unique pipe name for selftest using PID + epoch nanoseconds.
    fn selftest_pipe_name() -> String {
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!(r"\\.\pipe\nono-aipc-cap-child-selftest-{pid:x}-{nanos:x}")
    }
} // mod windows_impl
