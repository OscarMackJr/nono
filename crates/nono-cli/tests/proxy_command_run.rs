//! Wave-0 dispatch and fail-secure tests for the standalone `nono proxy`
//! command (Phase 112 SEC-07, adapted from upstream `2663e990`, #1261).
//!
//! Both tests are fast and never start a real proxy server: `--help` exits
//! via clap before any command body runs, and the `--no-auth` + non-loopback
//! `--listen` combination is rejected by the fail-secure guard (T-112-15)
//! before `proxy_command::run_proxy` does any launch-option construction or
//! server startup work.

use std::process::Command;

fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

#[test]
fn test_proxy_help_exits_zero_and_documents_usage() {
    let output = nono_bin()
        .args(["proxy", "--help"])
        .output()
        .expect("failed to run nono");

    assert!(
        output.status.success(),
        "expected exit 0, got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("proxy"),
        "expected 'proxy' in --help output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("nono proxy [flags]"),
        "expected the documented usage line 'nono proxy [flags]' in --help output, got:\n{stdout}"
    );
}

#[test]
fn test_proxy_no_auth_with_non_loopback_listen_is_rejected() {
    let output = nono_bin()
        .args(["proxy", "--no-auth", "--listen", "8.8.8.8"])
        .output()
        .expect("failed to run nono");

    assert!(
        !output.status.success(),
        "expected non-zero exit for --no-auth + non-loopback --listen, got: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires a loopback --listen address"),
        "expected the fail-secure guard message in stderr, got:\n{stderr}"
    );
}
