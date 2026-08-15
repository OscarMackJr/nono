//! Test-only Windows fixture-ownership normalisation, stated once.
//!
//! # Why this module exists
//!
//! `nono::path_is_owned_by_current_user` compares an object's NTFS **owner
//! SID** against `TOKEN_USER.User.Sid` with `EqualSid`. It is a SID-equality
//! test, not a privilege or effective-access test. That distinction decides
//! the behaviour of every ownership-gated code path in this crate:
//!
//! - `exec_strategy_windows::labels_guard` (mandatory-label apply gate),
//! - `exec_strategy_windows::dacl_guard` (writable-rule and ancestor walks),
//! - `hook_runtime_windows::validate_hook_script_windows` (step 4),
//! - the R-B3 `path_has_write_owner` workspace gate.
//!
//! In a **non-elevated** session (this project's dev host) a freshly created
//! tempdir is owned by the token's own user, so all of the above return
//! `Ok(true)` for free and ownership-dependent tests pass without doing
//! anything special.
//!
//! In an **elevated** session — which is what GitHub Actions'
//! `windows-latest` runner gives you, since it executes as `runneradmin` with
//! an elevated token — freshly created objects are owned by
//! `BUILTIN\Administrators`, a GROUP, not by the token user. `EqualSid` then
//! returns false, every ownership gate takes its fail-safe SKIP branch, and
//! the test's subject is never reached. The direction is counter-intuitive
//! and worth stating plainly: **elevation makes these guards apply FEWER
//! grants, not more**, because the guard is refusing to touch a path it does
//! not own.
//!
//! Calling [`take_ownership_for_current_user`] on a fixture normalises that
//! difference away, so an ownership-dependent test exercises the logic it
//! names on both kinds of session.
//!
//! # Why it lives here rather than in each test module
//!
//! The original copy of this helper was private to
//! `exec_strategy_windows::dacl_guard::tests`, so only that module could use
//! it. CI run 31888431118 turned that into a natural controlled experiment:
//! the five `dacl_guard` tests that called it PASSED on the elevated runner
//! and the two near-twin tests that did not both FAILED, with the helper call
//! the only material difference. Fifteen tests across four modules needed the
//! same normalisation; duplicating it fifteen times would have destroyed that
//! control and invited the divergence `cfg_test_regions` documents. There is
//! one implementation, and every ownership-dependent test routes through it.
//!
//! # What this helper is NOT
//!
//! It is not a way to make a test pass. Taking ownership changes only the
//! fixture's *precondition* — it does not touch the product logic under test,
//! and on a host that already owns the fixture it is a no-op. A test whose
//! subject is the NON-owned branch (`labels_guard::tests::
//! guard_skips_path_not_owned_by_current_user`, `non_owned_path_with_a_
//! foreign_label_is_exempt_not_a_coverage_gap`) must NOT call it — those
//! deliberately construct the opposite precondition.

use std::path::Path;

/// Make `path` owned by the CURRENT user so that
/// `nono::path_is_owned_by_current_user(path)` returns `Ok(true)`
/// deterministically, whether or not the test session is elevated.
///
/// Uses `icacls /setowner`, which needs no privilege when the caller can
/// already take ownership of the object (the owner itself, or a member of the
/// owning group — the elevated-runner case). Failure is a LOUD panic naming
/// the principal and the OS error: a fixture whose precondition silently did
/// not hold would make every downstream assertion meaningless.
///
/// Ownership assignment does not disturb the object's DACL or its
/// `SYSTEM_MANDATORY_LABEL_ACE`, so it is safe to call before or after
/// planting a label — though callers should prefer calling it immediately
/// after creating the fixture, since `SetNamedSecurityInfoW(LABEL_…)` itself
/// requires `WRITE_OWNER`.
pub fn take_ownership_for_current_user(path: &Path) {
    // `whoami` prints `domain\user`, which icacls /setowner accepts.
    let who = std::process::Command::new("whoami")
        .output()
        .expect("run whoami");
    let user = String::from_utf8_lossy(&who.stdout).trim().to_string();
    assert!(!user.is_empty(), "whoami returned an empty user");
    let out = std::process::Command::new("icacls")
        .arg(path)
        .arg("/setowner")
        .arg(&user)
        .arg("/Q")
        .output()
        .expect("run icacls /setowner");
    assert!(
        out.status.success(),
        "icacls /setowner {} -> {} failed: {}",
        path.display(),
        user,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// [`take_ownership_for_current_user`] applied to a hook script AND the
/// directory containing it.
///
/// `hook_runtime_windows::validate_hook_script_windows` gates on the SCRIPT's
/// owner (step 4) and separately inspects the PARENT directory's DACL for a
/// world-writable ACE (D-10). On an elevated runner both objects are created
/// owned by `BUILTIN\Administrators`, so the owner gate fires first and masks
/// the D-10 check entirely — the world-writable test never reaches the
/// assertion it exists to make.
pub fn take_ownership_of_script_and_parent(script: &Path) {
    if let Some(parent) = script.parent() {
        take_ownership_for_current_user(parent);
    }
    take_ownership_for_current_user(script);
}
