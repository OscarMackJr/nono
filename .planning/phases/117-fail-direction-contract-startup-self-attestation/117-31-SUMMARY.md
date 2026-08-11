---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 31
subsystem: testing
tags: [rust, integration-tests, source-scanning, symbol-resolution, wr-13]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "Plan 117-24's content_defines_symbol / contains_fn_exact source-scanning symbol matchers (layer_registry_selfcheck.rs, layer_registry_meta_test.rs)"
provides:
  - "is_ident_boundary — the single shared identifier-trailing-boundary predicate in tests/common/mod.rs, used by both source-scanning symbol matchers"
  - "content_defines_symbol now rejects prefix-preserving renames (trailing-boundary check) and resolves Type::method citations against the correct impl block"
affects: [117-gap-closure-round-4, any-future-plan-touching-layer_registry_selfcheck-or-layer_registry_meta_test]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Shared boundary predicate extracted to tests/common/mod.rs so a hardening rule applied to one source-scanning matcher cannot be silently absent from the other"
    - "Type::method citation resolution via nearest-preceding-impl-block backward line scan, reusing the same word-boundary predicate for the impl name check"

key-files:
  created: []
  modified:
    - crates/nono-cli/tests/common/mod.rs
    - crates/nono-cli/tests/layer_registry_meta_test.rs
    - crates/nono-cli/tests/layer_registry_selfcheck.rs

key-decisions:
  - "is_ident_boundary carries #[allow(dead_code)] with a structural per-compilation-unit justification, matching the existing house precedent on test_env.rs's EnvVarGuard/lock_env (each tests/*.rs file is a separate compilation unit; env_vars.rs and auto_pull_e2e_linux.rs also declare mod common; but never call this function, so it appears dead in those units)."
  - "Type qualifier resolution takes only the second rsplit(\"::\") segment as ty — does not support a longer mod::Type::method qualifier chain. Documented as a known, currently-unneeded limitation rather than silently mis-resolving."
  - "A rejected impl-mismatch match does not fail the whole search — scanning continues to a possible later, correct match in the same file (same pattern as the existing prefix/doc-comment rejection)."

patterns-established:
  - "Perturbation-proof-then-restore discipline for hardening checks: temporarily neutralize the new gate, confirm the specific regression test fails, restore, re-verify green — executed live for both new checks in this plan, not just asserted in prose."

requirements-completed: [CINT-01, CINT-03]

# Metrics
duration: ~15min
completed: 2026-08-11
---

# Phase 117 Plan 31: Shared Identifier-Boundary Predicate + Type::method Resolution Summary

**Extracted `is_ident_boundary` to `tests/common/mod.rs` so both source-scanning symbol matchers share one trailing-boundary rule, and hardened `content_defines_symbol` to reject prefix-preserving renames and resolve `Type::method` citations against the correct `impl` block — closing WR-13.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-08-11T12:20:00-04:00 (approx, first Read)
- **Completed:** 2026-08-11T12:45:00-04:00
- **Tasks:** 2/2 completed
- **Files modified:** 3 (`tests/common/mod.rs`, `tests/layer_registry_meta_test.rs`, `tests/layer_registry_selfcheck.rs`)

## Accomplishments

- `is_ident_boundary(next: Option<char>) -> bool` now lives in `crates/nono-cli/tests/common/mod.rs`, the single definition of "is the character after a needle match a valid identifier boundary" both matchers use.
- `contains_fn_exact` (`layer_registry_meta_test.rs`) delegates to `common::is_ident_boundary` instead of its own inline copy — pure extraction, byte-for-byte identical behavior, all 9 pre-existing tests pass unchanged.
- `content_defines_symbol` (`layer_registry_selfcheck.rs`) gained the trailing-boundary check it never had, closing the false positive where `fn create_restricted_token_with_sid_v2` satisfied a search for `create_restricted_token_with_sid`.
- `content_defines_symbol` also gained `Type::method` qualifier resolution: a citation like `WfpNetworkBackend::install` now resolves only against the nearest preceding `impl WfpNetworkBackend` block, so a same-named `install` method in `impl FirewallRulesNetworkBackend` no longer satisfies it.
- 3 new tests added; all 9 tests in `layer_registry_selfcheck.rs` pass (6 pre-existing + 3 new).
- Enumerated every source-scanning symbol matcher under `crates/nono-cli/tests/` (see below) — confirmed exactly two exist, and both now route through the one shared predicate.

## Enumeration of source-scanning symbol matchers (round-3 discipline)

Searched `crates/nono-cli/tests/` for any function performing a `format!("fn {…}")` / `format!("impl {…}")` needle scan with boundary/prefix logic:

```
$ rg -n 'boundary_ok|is_definition_prefix|qualifier-keyword|prefix_ok' crates/nono-cli/tests
crates\nono-cli\tests\layer_registry_meta_test.rs   (contains_fn_exact)
crates\nono-cli\tests\layer_registry_selfcheck.rs   (content_defines_symbol)

$ rg -n 'format!\("fn \{|format!\("impl \{' crates/nono-cli/tests
crates\nono-cli\tests\layer_registry_meta_test.rs:284:    let needle = format!("fn {fn_name}");
crates\nono-cli\tests\layer_registry_meta_test.rs:353,399:  (re-derived local `expected_fn` strings, not separate matchers)
crates\nono-cli\tests\layer_registry_selfcheck.rs:266:    let needles = [format!("fn {method}"), format!("impl {method}")];
```

Result: exactly two matcher functions exist — `contains_fn_exact` and `content_defines_symbol`. No third matcher was found. Both now call `common::is_ident_boundary` for the trailing-boundary check (Task 1 extracted it from `contains_fn_exact`'s pre-existing inline check; Task 2 added it to `content_defines_symbol`, which previously had none).

## Task Commits

1. **Task 1: Extract the shared identifier-boundary predicate; apply it to `contains_fn_exact`** - `39a1bcf4` (refactor)
2. **Task 2: Add the trailing-boundary check and `Type::method` resolution to `content_defines_symbol`** - `c0846f95` (fix)

_No separate plan-metadata commit yet — this SUMMARY.md commit follows immediately below (orchestrator-owned STATE.md/ROADMAP.md are NOT touched by this executor per project override)._

## Files Created/Modified

- `crates/nono-cli/tests/common/mod.rs` - Adds `pub fn is_ident_boundary`, the shared trailing-boundary predicate, with a structural `#[allow(dead_code)]` justification matching house precedent.
- `crates/nono-cli/tests/layer_registry_meta_test.rs` - `contains_fn_exact` now delegates to `common::is_ident_boundary`; doc comment updated to note the shared extraction. Net diff after temp perturbation-test add/remove is a no-op beyond the Task 1 commit (confirmed via `git status`/`git diff` — clean after cleanup).
- `crates/nono-cli/tests/layer_registry_selfcheck.rs` - `content_defines_symbol` rewritten with trailing-boundary check + `Type::method`/`impl`-block resolution (`nearest_preceding_impl_names`, `line_contains_word` helpers added); 3 new tests added.

## Decisions Made

- **`#[allow(dead_code)]` on `is_ident_boundary`, not a workaround.** `cargo clippy --workspace --all-targets -D warnings` initially failed with `error: function is_ident_boundary is never used` against the `env_vars` test binary — each `tests/<name>.rs` file compiles as a *separate* crate, and `env_vars.rs`/`auto_pull_e2e_linux.rs` both declare `mod common;` but use unrelated parts of the module (`test_env`), never `is_ident_boundary`. This exact situation is already documented and handled in the codebase on `test_env.rs`'s `EnvVarGuard`/`lock_env` ("Per-target dead-code justification... Structural, not lazy"). Followed the same pattern rather than inventing a new one, and cited CLAUDE.md's dead-code rule in the comment to make the exception legible on review. [Rule 3 — Blocking: build failure under the full clippy gate]
- **`ty` qualifier resolution is limited to one `::` segment.** Per the plan's own instruction, taking only the second `rsplit("::")` segment does not support a `mod::Type::method` chain. Documented in the function doc comment as a known limitation rather than silently mis-resolving a longer chain — no current citation in `layer_registry.rs` needs it (confirmed by `symbol_citation_extraction_finds_the_eight_converted_citations` still passing, which floor-checks at least 8 symbol-form citations, all resolved correctly by both tests and the full `registry_call_sites_exist`/`spec_matches_registry` suite).

## Deviations from Plan

None beyond the `#[allow(dead_code)]` addition, which was necessary to satisfy the workspace clippy gate under CLAUDE.md's `-D warnings` policy and is documented above as Rule 3 (blocking issue, auto-fixed, following an established in-repo pattern rather than introducing a new one).

## Perturbation Proofs

**Proof 1 — trailing-boundary check is load-bearing (WR-13's headline regression).**

Temporarily replaced `let boundary_ok = common::is_ident_boundary(content[after..].chars().next());` with `let boundary_ok = true;` in `content_defines_symbol`, then ran:

```
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck -- content_defines_symbol
running 5 tests
test content_defines_symbol_accepts_a_qualified_type_method_citation ... ok
test content_defines_symbol_rejects_a_doc_comment_mention ... ok
test content_defines_symbol_accepts_a_real_definition ... ok
test content_defines_symbol_distinguishes_same_named_methods_in_different_impls ... ok
test content_defines_symbol_rejects_a_prefix_preserving_rename ... FAILED

thread 'content_defines_symbol_rejects_a_prefix_preserving_rename' panicked at
crates\nono-cli\tests\layer_registry_selfcheck.rs:513:5:
`fn create_restricted_token_with_sid_v2` (a prefix-preserving rename) must not satisfy
content_defines_symbol("create_restricted_token_with_sid") — WR-13

test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
```

Restored the real check; re-ran the same command — all 5 filtered tests pass again (confirmed as part of the full 9-test run below).

**Proof 2 — impl-block resolution is load-bearing.**

Temporarily replaced the `impl_ok` computation with `let impl_ok = true;` (ignoring `ty`), then ran:

```
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck -- content_defines_symbol
running 5 tests
test content_defines_symbol_accepts_a_real_definition ... ok
test content_defines_symbol_accepts_a_qualified_type_method_citation ... ok
test content_defines_symbol_rejects_a_doc_comment_mention ... ok
test content_defines_symbol_distinguishes_same_named_methods_in_different_impls ... FAILED
test content_defines_symbol_rejects_a_prefix_preserving_rename ... ok

thread 'content_defines_symbol_distinguishes_same_named_methods_in_different_impls' panicked at
crates\nono-cli\tests\layer_registry_selfcheck.rs:526:5:
a `fn install` defined only inside `impl FirewallRulesNetworkBackend` must not satisfy the
qualified citation `WfpNetworkBackend::install` — WR-13

test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
```

Restored the real check; re-ran — all 9 tests in the file pass (see full run below).

**Proof 3 (round-3-discipline (a), extra) — `contains_fn_exact` already rejects the same rename shape.**

Added a temporary test asserting `!contains_fn_exact(src, "create_restricted_token_with_sid")` against a `..._v2` definition, ran it standalone, confirmed `ok` (it was already correctly guarded by Plan 24/WR-10's boundary check, now delegated to `common::is_ident_boundary`), then removed the temporary test — net diff on `layer_registry_meta_test.rs` after this session is zero beyond the Task 1 commit (`git status` shows the file unmodified relative to `39a1bcf4`).

```
$ cargo test -p nono-sandbox-cli --test layer_registry_meta_test -- temp_contains_fn_exact_rejects_a_prefix_preserving_rename
running 1 test
test temp_contains_fn_exact_rejects_a_prefix_preserving_rename ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out
```

**Live-tree empirical re-check (D-16) — the review's cited `::run` scenario, re-derived, not confirmed.**

```
$ rg -n 'pub fn run\(|fn run_fails_when_app_container_forced_unavailable' crates/nono-shell-broker/src/main.rs
459:    pub fn run(args: BrokerArgs) -> NonoResult<i32> {
1902:        fn run_fails_when_app_container_forced_unavailable() {
```

Added a temporary test loading the real `nono-shell-broker/src/main.rs` from disk and asserting `content_defines_symbol(&src, "run")` is `true`:

```
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck -- temp_live_tree_check_run_symbol_still_resolves
running 1 test
test temp_live_tree_check_run_symbol_still_resolves ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out
```

Confirms the fix does not turn into a false negative for the legitimate bare-symbol citation (`"main.rs::run"` in `layer_registry.rs`'s `call_sites`) — the real `pub fn run` still satisfies both the prefix and the new suffix check. Temporary test removed before the Task 2 commit.

## Full Test Run (both files, after cleanup)

```
$ cargo test -p nono-sandbox-cli --test layer_registry_meta_test --test layer_registry_selfcheck
     Running tests\layer_registry_meta_test.rs
running 9 tests
test contains_fn_exact_rejects_bang_suffix ... ok
test contains_fn_exact_accepts_real_definition_after_rejecting_a_false_positive ... ok
test contains_fn_exact_rejects_doc_comment_mention ... ok
test pascal_to_snake_case_matches_expected_shapes ... ok
test host_gated_rows_are_loud ... ok
test manual_verification_section_excludes_the_registry_table ... ok
test security_assumptions_are_loud ... ok
test every_registry_row_has_a_test ... ok
test also_automated_entries_are_non_vacuous ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests\layer_registry_selfcheck.rs
running 9 tests
test content_defines_symbol_distinguishes_same_named_methods_in_different_impls ... ok
test content_defines_symbol_accepts_a_real_definition ... ok
test call_site_extraction_ignores_backtick_doc_comment_citations ... ok
test content_defines_symbol_rejects_a_prefix_preserving_rename ... ok
test content_defines_symbol_accepts_a_qualified_type_method_citation ... ok
test content_defines_symbol_rejects_a_doc_comment_mention ... ok
test symbol_citation_extraction_finds_the_eight_converted_citations ... ok
test spec_matches_registry ... ok
test registry_call_sites_exist ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Verification Gate Results

- `cargo build --workspace --all-targets` — clean (only the pre-existing, unrelated `nono-shell-broker` "missing a lib target" advisory warning).
- `cargo fmt --check` — clean, no output.
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean (exit 0) after adding the structural `#[allow(dead_code)]` documented above.
- `cargo test -p nono-sandbox-cli --workspace` — **1622 passed, 12 failed, 2 ignored.** All 12 failures match the documented baseline exactly:
  - 11 pre-existing baseline failures (`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`, `config::tests::nono_home_dir_rejects_non_absolute_override`, `config::tests::nono_home_dir_falls_through_when_unset`, `config::tests::nono_home_dir_returns_override_when_set`, `config::tests::test_validated_home_falls_back_to_userprofile`, `config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists`, `config::tests::user_state_dir_uses_localappdata_on_windows`, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, `protected_paths::tests::blocks_child_directory_capability`, `protected_paths::tests::blocks_parent_directory_capability`, `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`).
  - 1 host-blocked failure from Plan 117-30's intentional D-31 test: `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` (this shell has no `SeTakeOwnershipPrivilege`, per the prior-work note in this plan's brief).
  - No new regressions. This plan's own changed test binaries (`layer_registry_meta_test`, `layer_registry_selfcheck`) are 100% green (18/18).
- No cross-target clippy gate applies to this plan — see `117-31-PLAN.md`'s `<cross_target_rationale>`: the canonical D-35 gate commands do not pass `--all-targets`/`--tests`, so they never compile `tests/*.rs` files regardless of `cfg` gates, and this plan touches only `tests/*.rs` files.

## Issues Encountered

`cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` initially failed on `nono-sandbox-cli`'s `env_vars` test binary with `error: function is_ident_boundary is never used` (dead_code, denied by `-D warnings`). Root cause: each `tests/<name>.rs` file is a distinct compilation unit; `is_ident_boundary` is only called from `layer_registry_meta_test.rs` and `layer_registry_selfcheck.rs`, not from `env_vars.rs` or `auto_pull_e2e_linux.rs`, both of which also declare `mod common;`. Resolved by following the exact precedent already established and documented in this same file for `EnvVarGuard`/`lock_env` — a structural, per-compilation-unit `#[allow(dead_code)]` with a cited rationale, not a blanket suppression. Re-ran the full workspace clippy gate afterward; clean.

## Next Phase Readiness

WR-13 is closed: both discovery matchers now share one boundary rule (`common::is_ident_boundary`), and `content_defines_symbol` correctly disambiguates `Type::method` citations by resolving the enclosing `impl` block, with both new gates proven load-bearing via live perturbation. No further action needed on this item for gap-closure round 3; STATE.md/ROADMAP.md updates are owned by the orchestrator per this repo's project-specific override and are not touched by this executor.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*
