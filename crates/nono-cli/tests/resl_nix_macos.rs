//! Phase 25-01 integration tests — macOS resource-limit enforcement (REQ-RESL-NIX-03).
//!
//! Tests the setrlimit-based enforcement for `--memory` and `--max-processes`, the
//! clap-time rejection of `--cpu-percent`, and the supervisor-side `--timeout` watchdog.
//!
//! Run individually:
//! ```sh
//! cargo test -p nono-cli --test resl_nix_macos
//! ```

#![cfg(target_os = "macos")]

use std::process::Command;
use std::time::Instant;

const NONO_BIN: &str = env!("CARGO_BIN_EXE_nono");

/// REQ-RESL-NIX-03 criterion 3: `--cpu-percent` is rejected at clap parse time on macOS.
///
/// nono must exit with a non-zero code (clap exits with 2 for parse errors), and the
/// stderr must reference the REQ-RESL-NIX-03 error context or `cpu_percent_macos`.
/// Crucially, no child process should be spawned (verified by absence of `echo hi` output).
#[test]
fn macos_cpu_percent_rejected_at_clap_parse() {
    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--cpu-percent",
            "50",
            "--",
            "echo",
            "this-should-not-appear",
        ])
        .output()
        .expect("failed to run nono binary");

    // Exit code must be non-zero (clap parse error = 2; nono may use a different code).
    assert!(
        !output.status.success(),
        "expected clap to reject --cpu-percent on macOS with a non-zero exit code, \
         but the command succeeded"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Stderr must contain a reference to REQ-RESL-NIX-03 or cpu_percent_macos.
    assert!(
        stderr.contains("REQ-RESL-NIX-03") || stderr.contains("cpu_percent_macos") || stderr.contains("not supported on macOS"),
        "expected stderr to mention 'REQ-RESL-NIX-03', 'cpu_percent_macos', or 'not supported on macOS'; \
         got stderr:\n{stderr}"
    );

    // Confirm no child was spawned (echo output must be absent).
    assert!(
        !stdout.contains("this-should-not-appear"),
        "echo child was spawned despite --cpu-percent rejection; stdout:\n{stdout}"
    );
    assert!(
        !stderr.contains("this-should-not-appear"),
        "echo output appeared in stderr despite --cpu-percent rejection; stderr:\n{stderr}"
    );
}

/// REQ-RESL-NIX-03 criterion 4: `--timeout 5s` kills the child at deadline.
///
/// The supervisor-side Instant deadline + SIGKILL watchdog fires at ~5s.
/// Wall time must be between 3s and 10s.
#[test]
fn macos_timeout_kills_at_deadline() {
    let start = Instant::now();
    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--timeout",
            "5s",
            // `--read=<dir>` grants read+execute recursively (the old split
            // `--allow-fs-read` / `--allow-fs-exec` flags were removed; on
            // macOS read paths receive `file-map-executable`, on Linux
            // `AccessMode::Read` maps to `ReadFile|ReadDir|Execute`). This
            // lets the sandboxed child exec /bin/* and load libs from /usr +
            // /private.
            "--read=/bin",
            "--read=/usr",
            "--read=/private",
            "--",
            "sleep",
            "60",
        ])
        .output()
        .expect("failed to run nono binary");
    let elapsed = start.elapsed();

    // Surface the child's exit status + captured output on every failure path.
    // Without this, a fast-exit (e.g. the sandbox refusing to launch `sleep`)
    // is indistinguishable from a watchdog bug — the bare timing assertion
    // hides the real cause. (Diagnostics added during the v2.10 Phase 65 macOS
    // CI rehab to root-cause a "took only 0.0s" failure on the runner.)
    let ctx = || {
        format!(
            "exit={:?} elapsed={:.3}s\nstdout:\n{}\nstderr:\n{}",
            output.status,
            elapsed.as_secs_f64(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
    };

    assert!(
        !output.status.success(),
        "expected child to be killed by timeout, but it exited successfully\n{}",
        ctx()
    );
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "expected timeout to kill within 10s, but took {:.1}s\n{}",
        elapsed.as_secs_f64(),
        ctx()
    );
    assert!(
        elapsed.as_secs_f64() >= 3.0,
        "expected timeout to take at least 3s, but took only {:.1}s\n{}",
        elapsed.as_secs_f64(),
        ctx()
    );
}

/// REQ-RESL-NIX-03 criterion 3 (supplementary): no "is not enforced on macos" in stderr.
///
/// With all valid resource-limit flags, stderr must NOT contain the old Phase 16
/// stub warning strings. `--cpu-percent` is intentionally omitted (it's rejected on macOS).
#[test]
fn macos_no_warnings_on_resource_flags() {
    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--memory",
            "4g", // generous to avoid accidental RLIMIT_AS failures
            "--max-processes",
            "1000",
            "--timeout",
            "60s",
            // `--read=<dir>` grants read+execute recursively (the old split
            // `--allow-fs-read` / `--allow-fs-exec` flags were removed; on
            // macOS read paths receive `file-map-executable`, on Linux
            // `AccessMode::Read` maps to `ReadFile|ReadDir|Execute`). This
            // lets the sandboxed child exec /bin/* and load libs from /usr +
            // /private.
            "--read=/bin",
            "--read=/usr",
            "--read=/private",
            "--",
            "echo",
            "hi",
        ])
        .output()
        .expect("failed to run nono binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("is not enforced on macos"),
        "found stale 'is not enforced on macos' warning in stderr. \
         The old Phase 16 stub warnings should have been removed in Phase 25-01.\n\
         stderr:\n{stderr}"
    );

    assert!(
        !stderr.contains("is not enforced on linux"),
        "found stale 'is not enforced on linux' warning in stderr on macOS.\n\
         stderr:\n{stderr}"
    );
}

/// REQ-RESL-NIX-03 criterion 2: `--max-processes 5` blocks the sixth fork on macOS.
///
/// RLIMIT_NPROC limits the total number of processes for the user. Spawning more
/// than `max_processes` should fail with EAGAIN from fork(). We set a very low limit
/// (5) and try to spawn 20 processes.
///
/// Note: RLIMIT_NPROC counts ALL processes for the UID on macOS (not just descendants).
/// This test may be sensitive to system load. A generous limit (5) is used to reduce
/// false negatives on lightly-loaded CI hosts.
#[test]
fn macos_max_processes_blocks_on_rlimit_nproc() {
    // Use a limit of 5 to account for the current process and nono supervisor
    // already consuming slots. The child (bash) + its subprocesses should hit the limit.
    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--max-processes",
            "5",
            // `--read=<dir>` grants read+execute recursively (the old split
            // `--allow-fs-read` / `--allow-fs-exec` flags were removed; on
            // macOS read paths receive `file-map-executable`, on Linux
            // `AccessMode::Read` maps to `ReadFile|ReadDir|Execute`). This
            // lets the sandboxed child exec /bin/* and load libs from /usr +
            // /private.
            "--read=/bin",
            "--read=/usr",
            "--read=/private",
            "--",
            "bash",
            "-c",
            "for i in $(seq 1 20); do sleep 60 & done; wait",
        ])
        .output()
        .expect("failed to run nono binary");

    // The child should exit non-zero due to fork failures (EAGAIN from RLIMIT_NPROC).
    // We don't assert a specific error message since EAGAIN presentation varies
    // (bash may not print it; the test just confirms enforcement kicked in).
    assert!(
        !output.status.success(),
        "expected child to fail with RLIMIT_NPROC violation (EAGAIN), but it exited successfully. \
         Check that --max-processes is enforced via setrlimit(RLIMIT_NPROC) on macOS."
    );
}
