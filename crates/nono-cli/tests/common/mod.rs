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
#[must_use]
pub fn is_ident_boundary(next: Option<char>) -> bool {
    !matches!(next, Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '!')
}
