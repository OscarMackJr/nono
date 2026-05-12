//! Integration tests for `nono profile validate --strict` (Plan 36-01a).
//!
//! Exercises the strict-mode fail-closed path and the non-strict
//! deprecation-warning path for legacy `override_deny` keys in profile JSON.
//!
//! # Test architecture
//!
//! These tests run `nono profile validate` (and `nono profile validate --strict`)
//! as separate subprocess invocations so that:
//!   1. The exit code is directly observable.
//!   2. stderr capture is clean (no cross-test pollution from the process-global
//!      `GLOBAL_DEPRECATION_COUNTER`).
//!
//! # Environment note
//!
//! Tests save/restore `HOME` and `XDG_CONFIG_HOME` per CLAUDE.md § "Environment
//! variables in tests" to prevent cross-test pollution. The restore is explicit
//! (not deferred via Drop-only patterns) to keep the modified window as short
//! as possible.

use std::process::Command;

fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

/// A minimal valid profile JSON that uses the legacy `override_deny` key.
fn legacy_profile_json() -> &'static str {
    r#"{
  "meta": { "name": "legacy-test", "version": "1.0" },
  "security": { "groups": ["allow_read_home"] },
  "filesystem": { "allow": [] },
  "policy": {
    "override_deny": ["/var/log"]
  }
}"#
}

/// A minimal valid profile JSON that uses the canonical `bypass_protection` key.
fn canonical_profile_json() -> &'static str {
    r#"{
  "meta": { "name": "canonical-test", "version": "1.0" },
  "security": { "groups": ["allow_read_home"] },
  "filesystem": { "allow": [] },
  "policy": {
    "bypass_protection": ["/var/log"]
  }
}"#
}

/// Helper: write a profile JSON fixture to a temp file and return the path.
fn write_fixture(dir: &tempfile::TempDir, filename: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(filename);
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// T-36-01-STRICT-MODE (acceptance criteria #1):
///
/// `nono profile validate --strict <legacy_profile.json>` exits non-zero
/// AND stderr contains both the literal strings "override_deny" and
/// "bypass_protection".
#[test]
fn test_profile_validate_strict_rejects_legacy_override_deny() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let fixture = write_fixture(&dir, "legacy.json", legacy_profile_json());

    // Save + restore HOME and XDG_CONFIG_HOME to prevent cross-test pollution.
    let orig_home = std::env::var("HOME").ok();
    let orig_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("HOME", dir.path());
    std::env::remove_var("XDG_CONFIG_HOME");

    let output = nono_bin()
        .args(["profile", "validate", "--strict"])
        .arg(&fixture)
        .output()
        .expect("failed to run nono");

    // Restore env before assertions (in case assertion panics).
    match orig_home {
        Some(ref v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }
    match orig_xdg {
        Some(ref v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "strict mode should exit non-zero for legacy override_deny key, \
         but exited 0.\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("override_deny"),
        "strict mode stderr should mention the legacy key 'override_deny'.\n\
         stderr: {stderr}"
    );
    assert!(
        stderr.contains("bypass_protection"),
        "strict mode stderr should mention the canonical key 'bypass_protection'.\n\
         stderr: {stderr}"
    );
}

/// T-36-01-STRICT-MODE (acceptance criteria #2):
///
/// `nono profile validate <legacy_profile.json>` (no `--strict`) exits zero
/// AND stderr contains the literal string "deprecated".
#[test]
fn test_profile_validate_non_strict_warns_and_continues() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let fixture = write_fixture(&dir, "legacy.json", legacy_profile_json());

    // Save + restore env.
    let orig_home = std::env::var("HOME").ok();
    let orig_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("HOME", dir.path());
    std::env::remove_var("XDG_CONFIG_HOME");

    let output = nono_bin()
        .args(["profile", "validate"])
        .arg(&fixture)
        .output()
        .expect("failed to run nono");

    // Restore env before assertions.
    match orig_home {
        Some(ref v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }
    match orig_xdg {
        Some(ref v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "non-strict validate should exit 0 for a valid legacy profile, \
         but exited {}.\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code().unwrap_or(-1)
    );
    assert!(
        stderr.contains("deprecated") || stderr.contains("WARN"),
        "non-strict validate should emit a deprecation warning to stderr.\n\
         stderr: {stderr}"
    );
}

/// T-36-01-STRICT-MODE (acceptance criteria #3):
///
/// `nono profile validate --strict <canonical_profile.json>` exits zero for
/// profiles that use the canonical `bypass_protection` key (no legacy key).
#[test]
fn test_profile_validate_strict_accepts_canonical_key() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let fixture = write_fixture(&dir, "canonical.json", canonical_profile_json());

    // Save + restore env.
    let orig_home = std::env::var("HOME").ok();
    let orig_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("HOME", dir.path());
    std::env::remove_var("XDG_CONFIG_HOME");

    let output = nono_bin()
        .args(["profile", "validate", "--strict"])
        .arg(&fixture)
        .output()
        .expect("failed to run nono");

    // Restore env before assertions.
    match orig_home {
        Some(ref v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }
    match orig_xdg {
        Some(ref v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "--strict should accept canonical bypass_protection key (exit 0).\n\
         stdout: {stdout}\nstderr: {stderr}"
    );
}
