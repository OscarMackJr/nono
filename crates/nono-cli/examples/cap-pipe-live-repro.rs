//! Multi-process live-repro helper for Phase 59-03 SC1 (re-accept) and SC2
//! (bounded read / slow child).
//!
//! Drives the production `nono::supervisor::socket::SupervisorSocket` server API
//! (`bind`, `recv_message_with_timeout`, `disconnect_and_reconnect`) across two
//! real OS processes over a 1-instance named pipe — the same transport shape as
//! the production capability pipe.
//!
//! Unlike the in-process integration tests (which use `SupervisorSocket::pair()`),
//! this helper exercises the real `bind()` → `ConnectNamedPipe` → client-connect
//! path with two separate processes and a real Windows named pipe. It uses a
//! normal pipe and a normal child (no WRITE_RESTRICTED token, no broker) so it
//! runs in any context.
//!
//! ## Modes
//!
//! - **Parent** (default): parse `--scenario <sc1|sc2|both>` (default `both`)
//!   and `--timeout-secs <N>` (default 2, or `NONO_SUPERVISOR_IPC_READ_TIMEOUT`
//!   env var). Runs the requested scenario(s) and prints `OVERALL: PASS` or
//!   `OVERALL: FAIL`.
//!
//! - **Child** (`--child <pipe_name> <sc1|sc2>`): connects to the pipe and
//!   behaves as specified by the scenario:
//!   - **sc2**: sends a 4-byte length prefix claiming 64 bytes, then stalls
//!     forever (partial-frame / slow child — exercises the bounded read timeout).
//!   - **sc1**: connects (conn1), drops the socket (triggers disconnect),
//!     waits 400 ms, reconnects (conn2), waits 300 ms, exits
//!     (exercises the re-accept loop).
//!
//! ## Platform
//!
//! Windows-only. The crate compiles on other platforms as an empty stub so the
//! `examples/` directory stays build-clean on all targets.

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("cap-pipe-live-repro: Windows-only; stub on other platforms.");
}

#[cfg(target_os = "windows")]
fn main() -> std::process::ExitCode {
    windows_impl::run()
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use nono::supervisor::socket::SupervisorSocket;
    use std::env;
    use std::path::Path;
    use std::process::{Command, ExitCode};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    // -----------------------------------------------------------------------
    // Entry point
    // -----------------------------------------------------------------------

    pub(super) fn run() -> ExitCode {
        let args: Vec<String> = env::args().collect();

        // Child mode: `--child <pipe_name> <scenario>`
        if let Some(pos) = args.iter().position(|a| a == "--child") {
            let pipe_name = args
                .get(pos + 1)
                .expect("--child requires a <pipe_name> argument");
            let scenario = args
                .get(pos + 2)
                .expect("--child requires a <scenario> argument (sc1 or sc2)");
            return run_child(pipe_name, scenario);
        }

        run_parent(&args)
    }

    // -----------------------------------------------------------------------
    // Parent
    // -----------------------------------------------------------------------

    fn run_parent(args: &[String]) -> ExitCode {
        // --scenario <sc1|sc2|both>  (default: both)
        let scenario = args
            .iter()
            .position(|a| a == "--scenario")
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| "both".to_string());

        // --timeout-secs <N>  or NONO_SUPERVISOR_IPC_READ_TIMEOUT env var, default 2
        let timeout_secs: u64 = args
            .iter()
            .position(|a| a == "--timeout-secs")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                env::var("NONO_SUPERVISOR_IPC_READ_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2)
            });
        let timeout = Duration::from_secs(timeout_secs);

        println!("cap-pipe-live-repro: scenario={scenario} timeout={timeout_secs}s");
        println!();

        let sc2_pass = if scenario == "sc2" || scenario == "both" {
            let name = unique_pipe_name();
            run_sc2_parent(&name, timeout)
        } else {
            true // not requested; counts as pass for OVERALL
        };

        let sc1_pass = if scenario == "sc1" || scenario == "both" {
            let name = unique_pipe_name();
            run_sc1_parent(&name, timeout)
        } else {
            true
        };

        println!();
        if sc2_pass && sc1_pass {
            println!("OVERALL: PASS");
            ExitCode::SUCCESS
        } else {
            println!("OVERALL: FAIL");
            ExitCode::FAILURE
        }
    }

    // -----------------------------------------------------------------------
    // SC2: bounded read / slow child
    // -----------------------------------------------------------------------
    // Parent: spawn child → bind (blocks until child connects) →
    //   recv_message_with_timeout → PASS iff err contains "[timeout]" and
    //   elapsed <= timeout + 1500ms slack.
    //
    // Child sc2: connect → write 4-byte length prefix (64 BE) → stall 30s.

    fn run_sc2_parent(pipe_name: &str, timeout: Duration) -> bool {
        println!("--- SC2: bounded read (slow child) ---");
        println!("pipe: {pipe_name}");

        let current_exe = env::current_exe().expect("current_exe");

        // Spawn child BEFORE bind() so the pipe exists before the child tries
        // to connect. The child's connect-with-retry handles the startup race.
        let mut child = Command::new(&current_exe)
            .args(["--child", pipe_name, "sc2"])
            .spawn()
            .expect("spawn child process for SC2");

        // bind() creates the named pipe and blocks on ConnectNamedPipe.
        let mut server = match SupervisorSocket::bind(Path::new(pipe_name)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SC2: bind() failed: {e}");
                let _ = child.kill();
                let _ = child.wait();
                println!("SC2 RESULT: FAIL (bind error: {e})");
                return false;
            }
        };

        // Allow a slack of 1500 ms on top of the nominal timeout.
        let slack = Duration::from_millis(1500);
        let deadline_guard = timeout.checked_add(slack).unwrap_or(timeout + slack);

        let t0 = Instant::now();
        let res = server.recv_message_with_timeout(timeout);
        let elapsed = t0.elapsed();

        // Kill/wait the child so it doesn't linger (it may be sleeping for 30s).
        let _ = child.kill();
        let _ = child.wait();

        let pass = match &res {
            Err(e) => {
                let msg = e.to_string();
                let tagged_timeout = msg.contains("[timeout]");
                let bounded = elapsed <= deadline_guard;
                if tagged_timeout && bounded {
                    println!(
                        "SC2 RESULT: PASS (elapsed={elapsed:.2?}, err=\"{msg}\")"
                    );
                    true
                } else {
                    println!(
                        "SC2 RESULT: FAIL (tagged_timeout={tagged_timeout}, \
                         bounded={bounded}, elapsed={elapsed:.2?}, err=\"{msg}\")"
                    );
                    false
                }
            }
            Ok(_) => {
                println!(
                    "SC2 RESULT: FAIL (expected timeout error but recv_message_with_timeout \
                     succeeded — child sent a complete valid frame unexpectedly)"
                );
                false
            }
        };
        pass
    }

    // -----------------------------------------------------------------------
    // SC1: transient close → re-accept
    // -----------------------------------------------------------------------
    // Parent: spawn child → bind (conn1) → disconnect_and_reconnect() (blocks
    //   until child's conn2 arrives) → PASS iff Ok.
    //
    // Child sc1: connect (conn1) → drop socket → sleep 400ms → reconnect
    //   (conn2) → sleep 300ms → exit.

    fn run_sc1_parent(pipe_name: &str, timeout: Duration) -> bool {
        println!("--- SC1: transient close / re-accept ---");
        println!("pipe: {pipe_name}");
        let _ = timeout; // SC1 does not use a read timeout; only for API consistency

        let current_exe = env::current_exe().expect("current_exe");

        // Spawn child BEFORE bind().
        let mut child = Command::new(&current_exe)
            .args(["--child", pipe_name, "sc1"])
            .spawn()
            .expect("spawn child process for SC1");

        // bind() creates pipe + waits for conn1.
        let mut server = match SupervisorSocket::bind(Path::new(pipe_name)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SC1: bind() failed: {e}");
                let _ = child.kill();
                let _ = child.wait();
                println!("SC1 RESULT: FAIL (bind error: {e})");
                return false;
            }
        };
        println!("SC1: conn1 established");

        // disconnect_and_reconnect(): DisconnectNamedPipe + ConnectNamedPipe.
        // Blocks until the child reconnects (conn2).
        let res = server.disconnect_and_reconnect();

        let _ = child.kill();
        let _ = child.wait();

        match res {
            Ok(()) => {
                println!("SC1 RESULT: PASS (disconnect_and_reconnect() returned Ok)");
                true
            }
            Err(e) => {
                println!("SC1 RESULT: FAIL (disconnect_and_reconnect() error: {e})");
                false
            }
        }
    }

    // -----------------------------------------------------------------------
    // Child dispatch
    // -----------------------------------------------------------------------

    fn run_child(pipe_name: &str, scenario: &str) -> ExitCode {
        match scenario {
            "sc1" => run_child_sc1(pipe_name),
            "sc2" => run_child_sc2(pipe_name),
            other => {
                eprintln!("cap-pipe-live-repro child: unknown scenario {other:?}");
                ExitCode::FAILURE
            }
        }
    }

    fn run_child_sc2(pipe_name: &str) -> ExitCode {
        // Connect to the parent's pipe (with retry to handle startup race).
        let mut sock = match connect_with_retry(pipe_name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("child sc2: connect failed: {e}");
                return ExitCode::FAILURE;
            }
        };

        // Write a 4-byte big-endian length prefix claiming 64 bytes of payload,
        // then stall forever. This creates the partial-frame condition that the
        // parent's bounded read must detect and time out on.
        if let Err(e) = sock.write_raw_bytes(&64u32.to_be_bytes()) {
            eprintln!("child sc2: write_raw_bytes failed: {e}");
            return ExitCode::FAILURE;
        }

        // Stall — never send the payload. The parent will kill us after its timeout.
        std::thread::sleep(Duration::from_secs(30));
        ExitCode::SUCCESS
    }

    fn run_child_sc1(pipe_name: &str) -> ExitCode {
        // conn1: connect, then explicitly drop the socket to trigger a disconnect.
        {
            let _sock = match connect_with_retry(pipe_name) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("child sc1 conn1: connect failed: {e}");
                    return ExitCode::FAILURE;
                }
            };
            // _sock is dropped here → pipe client-side closed → conn1 severed.
        }

        // Brief pause to let the server side observe the disconnect before
        // the child rushes in with conn2.
        std::thread::sleep(Duration::from_millis(400));

        // conn2: reconnect. The parent's disconnect_and_reconnect() is blocking on
        // ConnectNamedPipe, waiting for this.
        let _sock2 = match connect_with_retry(pipe_name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("child sc1 conn2: reconnect failed: {e}");
                return ExitCode::FAILURE;
            }
        };

        // Hold conn2 briefly so disconnect_and_reconnect() completes on the
        // parent side before the child exits.
        std::thread::sleep(Duration::from_millis(300));
        ExitCode::SUCCESS
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Attempt to connect to the named pipe up to 50 times with 100 ms sleeps.
    ///
    /// This handles the startup race between child spawn and the parent's
    /// `bind()` call (the pipe is created during `bind()`, before
    /// `ConnectNamedPipe` blocks, so the child may arrive before the pipe
    /// exists).
    fn connect_with_retry(pipe_name: &str) -> nono::Result<SupervisorSocket> {
        let mut last_err: Option<nono::NonoError> = None;
        for attempt in 0..50u32 {
            match SupervisorSocket::connect(Path::new(pipe_name)) {
                Ok(sock) => return Ok(sock),
                Err(e) => {
                    last_err = Some(e);
                    if attempt < 49 {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
        Err(last_err.expect("at least one attempt was made"))
    }

    /// Build a unique pipe name using PID + epoch nanoseconds (mirrors pipe-repro.rs).
    fn unique_pipe_name() -> String {
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!(r"\\.\pipe\nono-cap-live-repro-{pid:x}-{nanos:x}")
    }
} // mod windows_impl
