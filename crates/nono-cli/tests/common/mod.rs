//! Shared test utilities for `nono-cli` integration tests.
//!
//! Include from any integration test with:
//! ```rust
//! #[path = "common/mod.rs"]
//! mod common;
//! ```
//!
//! See individual sub-modules for documentation.

pub mod test_env;

/// Make the current user the NTFS owner of `path`, so ownership-gated code
/// paths behave the same whether or not the test session is elevated.
///
/// # Why this is duplicated from `src/test_ownership_windows.rs`
///
/// That module is the canonical copy, but it is declared `#[cfg(test)] mod` in
/// `main.rs` — i.e. it lives in the *unit-test* configuration of a **binary**
/// crate. `nono-cli` has no lib target, so integration tests under `tests/`
/// compile as separate crates that link the binary's public API (there is
/// none) and cannot reach it. The duplication is structural, not laziness:
/// there is no shared compilation unit to hoist it into. Keep the two in sync;
/// if they ever diverge, the canonical semantics are the `src/` copy's.
///
/// # Why it is needed
///
/// The elevated `windows-latest` runner owns freshly created objects as
/// `BUILTIN\Administrators`, not as the runner user. `nono`'s R-B3 gate calls
/// `nono::path_has_write_owner` -> `path_is_owned_by_current_user`, which is a
/// token-USER-SID equality test, so on CI it reports "no WRITE_OWNER" for a
/// directory the test just created and `nono run` refuses to start. Taking
/// ownership first is the remedy the R-B3 error message itself recommends.
///
/// `icacls /setowner` needs no privilege when the caller can already take
/// ownership (the owner itself, or a member of the owning group — exactly the
/// elevated-runner case). Failure is a LOUD panic naming the principal and the
/// OS error: a fixture whose precondition silently did not hold would make
/// every downstream assertion meaningless.
///
/// Per-target dead-code justification: see `is_ident_boundary` below — only
/// `env_vars.rs` calls this, and each `tests/<name>.rs` is its own compilation
/// unit.
#[cfg(windows)]
#[allow(dead_code)]
pub fn take_ownership_for_current_user(path: &std::path::Path) {
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

/// Phase 117 Plan 31 (WR-13 gap closure): the single identifier-trailing-
/// boundary predicate shared by both source-scanning symbol matchers in
/// `crates/nono-cli/tests/` —
/// `layer_registry_meta_test.rs::contains_fn_exact` and
/// `layer_registry_selfcheck.rs::content_defines_symbol`. Plan 117-24 wrote
/// this exact check inline in `contains_fn_exact` only; `content_defines_symbol`
/// shipped with a prefix check but no suffix check, so a prefix-preserving
/// rename (`create_restricted_token_with_sid` ->
/// `create_restricted_token_with_sid_v2`) still satisfied it. Extracting one
/// definition both call means a future round cannot harden one matcher and
/// forget the other.
///
/// `next` is the character immediately after a candidate `needle` match (or
/// `None` at end-of-string). Returns `true` when that position is a valid
/// identifier boundary: end-of-string, or any character that is not an ASCII
/// alphanumeric, `_`, or `!` (the `!` exclusion rules out a macro-invocation
/// false positive, e.g. `fn foo!` is not a real function definition).
///
/// Per-target dead-code justification (same shape as `test_env.rs`'s
/// `EnvVarGuard`/`lock_env` above it): each `tests/<name>.rs` file is a
/// SEPARATE compilation unit, and dead-code analysis runs per-unit. Only
/// `layer_registry_meta_test.rs` and `layer_registry_selfcheck.rs` call this
/// function; the other `tests/*.rs` files that also declare `mod common;`
/// (`env_vars.rs`, `auto_pull_e2e_linux.rs`) use unrelated parts of this
/// module and never reference `is_ident_boundary`, so it appears dead in
/// those units. Structural, not lazy — see `CLAUDE.md`'s dead-code rule.
#[allow(dead_code)]
#[must_use]
pub fn is_ident_boundary(next: Option<char>) -> bool {
    !matches!(next, Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '!')
}
