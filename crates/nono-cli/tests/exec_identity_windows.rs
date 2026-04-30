//! Plan 22-05b Task 5 — Windows Authenticode + SHA-256 fallback regression
//! suite (REQ-AUD-03 acceptance #2 + #3).
//!
//! These tests run only on Windows hosts. On non-Windows hosts the entire
//! file compiles to nothing via the top-of-file `#![cfg(target_os = "windows")]`
//! attribute (documented-skip per phase posture per VALIDATION 22-05-T3).
//!
//! ## Coverage relationship to unit tests (Phase 28 update)
//!
//! Phase 28 (v2.3) lit up the Authenticode chain walker by enabling the
//! `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip`
//! features on `windows-sys`. The deferred substring-match test
//! `authenticode_signed_records_subject` previously lived in this file
//! deferred behind a v2.2 ignore message; per Phase 28 28-CONTEXT.md
//! PATH-4 it has been MOVED to live inline alongside the other unit
//! tests in `crates/nono-cli/src/exec_identity_windows.rs::tests`, where
//! it has direct access to `query_authenticode_status` without needing
//! a lib+bin refactor of `nono-cli` (the bin-only crate cannot re-expose
//! internal modules to integration test targets without major
//! restructuring).
//!
//! What remains in THIS file: subprocess-boundary regressions that probe
//! the linkage of the Authenticode subsystem from outside the bin process
//! tree:
//!
//! 1. `nono_binary_loads_without_unresolved_authenticode_symbols` — the
//!    cheapest end-to-end probe; a `nono --version` invocation that would
//!    fail at link time if any windows-sys feature flag was dropped.
//! 2. `nono_prune_help_still_functions_post_authenticode_addition` —
//!    cross-references the prune_alias_deprecation suite and probes that
//!    the new Authenticode features did not break the `nono prune --help`
//!    surface.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

// Integration-test-side: exercise the subprocess surface only. The
// in-bin unit tests at `crates/nono-cli/src/exec_identity_windows.rs::tests`
// already cover the direct-API shape (Unsigned + missing-path + RAII
// close-guard implicit). This file adds high-level subprocess regressions
// that will become end-to-end once the audit-show pipeline emits the
// `AuthenticodeStatus` sibling field.

use std::process::Command;

fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

// `authenticode_signed_records_subject` was MOVED to
// `crates/nono-cli/src/exec_identity_windows.rs::tests` in Phase 28 Plan
// 28-01 per 28-CONTEXT.md PATH-4. The relocated test exercises the same
// substring assertion against the same fixture binary directly via
// `query_authenticode_status`, sidestepping the bin-only-crate visibility
// constraint that previously kept the integration-target version
// inactive. REQ-AUDC-02 acceptance #1 closes via the relocated unit test.

/// Plan 22-05b Task 5 acceptance: the prune-alias regression test runs
/// at the integration boundary. This test verifies the Authenticode
/// query subsystem is at least *callable* via the binary's diagnostic
/// surfaces (which today means: the binary loads without resolving any
/// missing symbols). A linkage failure would surface as a non-zero
/// exit code on a benign command (`--version`).
#[test]
fn nono_binary_loads_without_unresolved_authenticode_symbols() {
    // If `Win32_Security_Cryptography` / `Win32_Security_WinTrust`
    // feature flags were dropped from Cargo.toml, the binary would
    // fail to link or fail to load on first invocation. `nono --version`
    // is the cheapest end-to-end probe.
    let out = nono_bin()
        .arg("--version")
        .output()
        .expect("failed to invoke nono.exe");
    assert!(
        out.status.success(),
        "nono --version must exit cleanly; got status {:?}, stderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("nono"),
        "expected 'nono' version banner, got: {stdout}"
    );
}

/// Plan 22-05b Task 5 — verifies the `nono prune` CLI surface still
/// works post-Authenticode-feature-flag-additions. Any link-time or
/// runtime failure introduced by the new windows-sys features would
/// surface here. Cross-references the prune_alias_deprecation suite.
#[test]
fn nono_prune_help_still_functions_post_authenticode_addition() {
    let out = nono_bin()
        .arg("prune")
        .arg("--help")
        .output()
        .expect("failed to invoke nono prune --help");
    assert!(
        out.status.success(),
        "nono prune --help must exit cleanly; got status {:?}, stderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.to_lowercase().contains("deprecat"),
        "expected DEPRECATED note carried over from Task 3, got:\n{combined}"
    );
}
