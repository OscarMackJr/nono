//! Phase 25-01 integration tests — Linux resource-limit enforcement (REQ-RESL-NIX-01/02).
//!
//! Each test runs the `nono` binary with a specific resource-limit flag and asserts the
//! kernel-level enforcement is active. All tests are gated on cgroup v2 availability;
//! on a system without cgroup v2 delegation (e.g., CI with cgroup v1 or no systemd), each
//! test prints a skip message and returns without failing.
//!
//! Run individually:
//! ```sh
//! cargo test -p nono-cli --test resl_nix_linux
//! ```

#![cfg(target_os = "linux")]

use std::process::Command;
use std::time::Instant;

const NONO_BIN: &str = env!("CARGO_BIN_EXE_nono");

/// Returns `true` if the current process has a cgroup v2 delegation (single `0::/...` line).
/// Tests use this to skip gracefully on cgroup v1 / non-systemd CI hosts.
fn cgroup_v2_available() -> bool {
    let Ok(content) = std::fs::read_to_string("/proc/self/cgroup") else {
        return false;
    };
    let trimmed = content.trim();
    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() != 1 {
        return false;
    }
    if !lines[0].starts_with("0::/") {
        return false;
    }
    // Also confirm the cgroup directory is writable (delegation check).
    let cg_path_rel = lines[0].trim_start_matches("0::/");
    let cg_path = format!("/sys/fs/cgroup/{cg_path_rel}/cgroup.subtree_control");
    std::fs::metadata(&cg_path)
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
}

/// Macro to skip test with an explanatory message if cgroup v2 is not available.
macro_rules! require_cgroup_v2 {
    () => {
        if !cgroup_v2_available() {
            eprintln!(
                "SKIP: cgroup v2 delegation not available on this host (non-systemd or cgroup v1); \
                 test is only meaningful on Linux with systemd user slice delegation."
            );
            return;
        }
    };
}

/// REQ-RESL-NIX-01 criterion 1: `--memory 256m` OOM-kills a large allocation.
///
/// Spawns `bash -c 'tail -c 1G </dev/urandom'` which tries to read 1GiB into memory.
/// With a 256MiB memory.max limit, the cgroup OOM killer delivers SIGKILL (exit code 137)
/// before the allocation completes.
#[test]
fn linux_memory_limit_oom_kills_child() {
    require_cgroup_v2!();

    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--memory",
            "256m",
            "--allow-fs-read=/dev",
            "--allow-fs-read=/usr",
            "--allow-fs-read=/bin",
            "--allow-fs-read=/lib",
            "--allow-fs-read=/lib64",
            "--allow-fs-exec=/bin",
            "--allow-fs-exec=/usr",
            "--",
            "bash",
            "-c",
            // Allocate memory aggressively to trigger OOM kill
            "python3 -c \"import ctypes; buf = ctypes.create_string_buffer(1024*1024*1024)\" 2>&1 || \
             bash -c 'tail -c 1073741824 /dev/urandom > /dev/null' 2>&1",
        ])
        .output()
        .expect("failed to run nono binary");

    // Exit code 137 = 128 + 9 (SIGKILL) is the typical OOM kill exit code.
    // The child may also exit with 1 from bash if the subprocess was killed.
    // We accept any non-zero exit code as evidence the OOM limit triggered.
    assert!(
        !output.status.success(),
        "expected child to be killed by OOM, but it exited successfully. \
         Check that --memory 256m is actually enforced by cgroup v2 memory.max."
    );
}

/// REQ-RESL-NIX-01 criterion 3: `--max-processes 10` blocks the eleventh fork.
///
/// Spawns 20 background sleep processes; only 10 should succeed. The 11th+ fork
/// fails with an error containing "pids.max" or similar kernel diagnostic.
#[test]
fn linux_max_processes_blocks_eleventh_fork() {
    require_cgroup_v2!();

    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--max-processes",
            "10",
            "--allow-fs-read=/usr",
            "--allow-fs-read=/bin",
            "--allow-fs-read=/lib",
            "--allow-fs-read=/lib64",
            "--allow-fs-exec=/bin",
            "--allow-fs-exec=/usr",
            "--",
            "bash",
            "-c",
            // Try to spawn 20 background processes; the 11th+ should fail due to pids.max.
            "for i in $(seq 1 20); do sleep 60 & done; wait",
        ])
        .output()
        .expect("failed to run nono binary");

    // The child should exit non-zero because fork failures cause bash to exit with error.
    assert!(
        !output.status.success(),
        "expected child to fail with pids.max violation, but it exited successfully"
    );
}

/// REQ-RESL-NIX-02 criterion 1: `--timeout 5s` kills the child at deadline.
///
/// The child sleeps 60s but must be killed within ~5s by the cgroup.kill watchdog.
/// We assert the wall time is between 3s and 10s (generous bounds for CI variance).
#[test]
fn linux_timeout_kills_at_deadline() {
    require_cgroup_v2!();

    let start = Instant::now();
    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--timeout",
            "5s",
            "--allow-fs-exec=/bin",
            "--allow-fs-exec=/usr",
            "--allow-fs-read=/bin",
            "--allow-fs-read=/usr",
            "--allow-fs-read=/lib",
            "--allow-fs-read=/lib64",
            "--",
            "sleep",
            "60",
        ])
        .output()
        .expect("failed to run nono binary");
    let elapsed = start.elapsed();

    assert!(
        !output.status.success(),
        "expected child to be killed by timeout, but it exited successfully"
    );
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "expected timeout to kill within 10s, but took {:.1}s",
        elapsed.as_secs_f64()
    );
    assert!(
        elapsed.as_secs_f64() >= 3.0,
        "expected timeout to take at least 3s, but took only {:.1}s (deadline not firing at right time)",
        elapsed.as_secs_f64()
    );
}

/// REQ-RESL-NIX-01 criterion 4: no "is not enforced on linux" warnings in stderr.
///
/// With all four resource-limit flags set, stderr must NOT contain the old Phase 16
/// "is not enforced on linux" warning strings. Presence would mean the old stub code
/// was not removed.
#[test]
fn linux_no_warnings_on_resource_flags() {
    // This test does NOT require cgroup v2 — it tests warning-string absence, which
    // should be true even on cgroup v1 hosts (the error is a different kind).
    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--memory",
            "4g", // generous limit to avoid accidental OOM in this test
            "--cpu-percent",
            "50",
            "--max-processes",
            "1000",
            "--timeout",
            "60s",
            "--allow-fs-exec=/bin",
            "--allow-fs-exec=/usr",
            "--allow-fs-read=/bin",
            "--allow-fs-read=/usr",
            "--allow-fs-read=/lib",
            "--allow-fs-read=/lib64",
            "--",
            "echo",
            "hi",
        ])
        .output()
        .expect("failed to run nono binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("is not enforced on linux"),
        "found stale 'is not enforced on linux' warning in stderr. \
         The old Phase 16 stub warnings should have been removed in Phase 25-01.\n\
         stderr:\n{stderr}"
    );

    assert!(
        !stderr.contains("is not enforced on macos"),
        "found stale 'is not enforced on macos' warning in stderr on Linux. \
         stderr:\n{stderr}"
    );
}

/// REQ-RESL-NIX-02 criterion 2: `--timeout` atomically kills grandchildren via cgroup.kill.
///
/// Spawns 10 background sleep processes. The timeout must kill ALL of them, not just
/// the direct child. Verified by confirming the parent's `nono run` exits within the
/// timeout window (if grandchildren weren't killed, `wait` would block indefinitely
/// after the parent receives SIGKILL but the grandchildren are still running).
#[test]
fn linux_timeout_atomic_kill_grandchildren() {
    require_cgroup_v2!();

    let start = Instant::now();
    let output = Command::new(NONO_BIN)
        .args([
            "run",
            "--timeout",
            "5s",
            "--allow-fs-exec=/bin",
            "--allow-fs-exec=/usr",
            "--allow-fs-read=/bin",
            "--allow-fs-read=/usr",
            "--allow-fs-read=/lib",
            "--allow-fs-read=/lib64",
            "--",
            "bash",
            "-c",
            // Spawn 10 grandchildren; `wait` would block forever if they survived the kill.
            "for i in $(seq 1 10); do sleep 60 & done; wait",
        ])
        .output()
        .expect("failed to run nono binary");
    let elapsed = start.elapsed();

    assert!(
        !output.status.success(),
        "expected child to be killed by timeout, but it exited successfully"
    );
    assert!(
        elapsed.as_secs_f64() < 12.0,
        "expected cgroup.kill to atomically kill grandchildren within 12s, took {:.1}s. \
         This may indicate grandchildren survived the kill and `wait` was not interrupted.",
        elapsed.as_secs_f64()
    );
}
