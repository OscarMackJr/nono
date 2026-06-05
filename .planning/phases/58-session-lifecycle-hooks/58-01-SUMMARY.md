---
phase: 58-session-lifecycle-hooks
plan: "01"
subsystem: profile-pipeline
tags:
  - session-hooks
  - profile
  - data-model
  - schema
dependency_graph:
  requires: []
  provides:
    - "profile::SessionHook"
    - "profile::SessionHooks"
    - "Profile.session_hooks"
    - "PreparedSandbox.session_hooks"
    - "ExecutionFlags.session_hooks"
    - "nono-profile.schema.json SessionHooks/$defs"
  affects:
    - "crates/nono-cli/src/profile/mod.rs"
    - "crates/nono-cli/src/policy.rs"
    - "crates/nono-cli/src/sandbox_prepare.rs"
    - "crates/nono-cli/src/launch_runtime.rs"
    - "crates/nono-cli/data/nono-profile.schema.json"
tech_stack:
  added: []
  patterns:
    - "4-location lockstep (Profile, ProfileDeserialize, From<ProfileDeserialize>, merge_profiles)"
    - "Option-semantics merge (child wins per slot, not OR)"
    - "Pre-move clone before struct literal"
key_files:
  created: []
  modified:
    - "crates/nono-cli/src/profile/mod.rs"
    - "crates/nono-cli/src/policy.rs"
    - "crates/nono-cli/src/sandbox_prepare.rs"
    - "crates/nono-cli/src/launch_runtime.rs"
    - "crates/nono-cli/data/nono-profile.schema.json"
    - "crates/nono-cli/src/proxy_runtime.rs"
    - "crates/nono-cli/src/main.rs"
decisions:
  - "D-PLAN58-01-A: Used Option::or semantics (not OR semantics) for merge_profiles per upstream daa55c8 — hooks are nullable slots, not boolean flags"
  - "D-PLAN58-01-B: Extracted profile_session_hooks before loaded_profile move in sandbox_prepare.rs to avoid partial-move compiler error; mirrors profile_secrets extraction pattern"
  - "D-PLAN58-01-C: Added test_session_hooks_rejects_unknown_top_level_field as 5th test (beyond plan's 4) to cover SessionHooks-level deny_unknown_fields, not just SessionHook-level"
metrics:
  duration: "~45 minutes"
  completed: "2026-06-05"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 7
  lines_added: 364
---

# Phase 58 Plan 01: SessionHooks Type System and Profile Pipeline Summary

Add `SessionHook`/`SessionHooks` Rust types and thread them through the full 4-location profile pipeline (Profile, ProfileDeserialize, From, merge_profiles), policy.rs ProfileDef, PreparedSandbox, ExecutionFlags, and JSON schema — providing Plans 02 and 03 with complete type infrastructure for hook dispatch.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add SessionHook + SessionHooks types and 4-location lockstep | `72ee31db` | profile/mod.rs |
| 2 | Thread through policy.rs, sandbox_prepare.rs, launch_runtime.rs, schema | `678cb97a` | policy.rs, sandbox_prepare.rs, launch_runtime.rs, nono-profile.schema.json, proxy_runtime.rs, main.rs |

## What Was Built

### Task 1: profile/mod.rs

Added `SessionHook` and `SessionHooks` structs (placed after `HooksConfig`/`HookConfig`, before the ProfileDeserialize section):

- `SessionHook`: `pub script: PathBuf`, `pub timeout_secs: Option<u64>`, `#[serde(deny_unknown_fields)]` — required `script` field, no Default (required field)
- `SessionHooks`: `pub before: Option<SessionHook>`, `pub after: Option<SessionHook>`, `Default`, `#[serde(deny_unknown_fields)]` — defaults to no hooks

Threaded `session_hooks: SessionHooks` at all 4 lockstep locations:
1. `Profile` struct — `#[serde(default)] pub session_hooks: SessionHooks`
2. `ProfileDeserialize` struct — `#[serde(default)] session_hooks: SessionHooks`
3. `From<ProfileDeserialize>` body — `session_hooks: raw.session_hooks`
4. `merge_profiles` — `session_hooks: SessionHooks { before: child.session_hooks.before.or(base.session_hooks.before), after: child.session_hooks.after.or(base.session_hooks.after) }`

Fixed 2 test-helper Profile struct literals (`base_profile()` + `child_profile()`) that became non-exhaustive after adding the field.

Added `session_hooks_tests` module with 5 tests: basic deserialize, SessionHook-level unknown-field rejection, SessionHooks-level unknown-field rejection, child-overrides-per-slot, child-inherits-absent.

### Task 2: policy.rs, sandbox_prepare.rs, launch_runtime.rs, schema

**policy.rs:**
- `ProfileDef.session_hooks: profile::SessionHooks` with `#[serde(default)]`
- `to_raw_profile()` forwards `session_hooks: self.session_hooks.clone()`
- 3 new tests: `test_schema_has_session_hooks_property`, `test_schema_has_session_hooks_defs`, `test_to_raw_profile_includes_session_hooks`

**sandbox_prepare.rs:**
- `PreparedSandbox.session_hooks: profile::SessionHooks`
- Two construction sites:
  - Manifest path: `session_hooks: profile::SessionHooks::default()`
  - Profile path: `session_hooks: profile_session_hooks` (pre-extracted before `loaded_profile` moves)

**launch_runtime.rs:**
- `ExecutionFlags.session_hooks: crate::profile::SessionHooks`
- `ExecutionFlags::defaults()`: `session_hooks: crate::profile::SessionHooks::default()`
- `prepare_run_launch_plan` struct literal: `session_hooks: prepared.session_hooks`

**nono-profile.schema.json:**
- `"session_hooks"` property referencing `#/$defs/SessionHooks`
- `"SessionHooks"` $def: `additionalProperties: false`, `before`/`after` as `$ref/SessionHook`
- `"SessionHook"` $def: `additionalProperties: false`, `required: ["script"]`, `script` (string) + `timeout_secs` (integer, minimum 1)

**Fixture fixes (Rule 3 auto-fix):** Added `session_hooks: crate::profile::SessionHooks::default()` to 3 test-fixture `PreparedSandbox` literals in `proxy_runtime.rs` (1) and `main.rs` (2).

## Test Results

All 8 new tests pass:
- `profile::session_hooks_tests::test_session_hooks_basic_deserialize` — PASS
- `profile::session_hooks_tests::test_session_hooks_rejects_unknown_field` — PASS
- `profile::session_hooks_tests::test_session_hooks_rejects_unknown_top_level_field` — PASS
- `profile::session_hooks_tests::test_merge_profiles_session_hooks_child_overrides_per_field` — PASS
- `profile::session_hooks_tests::test_merge_profiles_session_hooks_child_inherits_when_absent` — PASS
- `policy::tests::test_schema_has_session_hooks_property` — PASS
- `policy::tests::test_schema_has_session_hooks_defs` — PASS
- `policy::tests::test_to_raw_profile_includes_session_hooks` — PASS

Full test suite: 1198 passed, 4 failed (all 4 are pre-existing baseline failures — `profile_cmd init` + 3 `protected_paths`; no regressions).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 3 additional PreparedSandbox struct literals missing session_hooks field**

- **Found during:** Task 2 compilation
- **Issue:** `cargo test -p nono-cli` revealed 3 more PreparedSandbox struct literal construction sites in test fixtures (proxy_runtime.rs:383, main.rs:260, main.rs:313) that failed with E0063 missing-field errors
- **Fix:** Added `session_hooks: crate::profile::SessionHooks::default()` to each test fixture
- **Files modified:** `crates/nono-cli/src/proxy_runtime.rs`, `crates/nono-cli/src/main.rs`
- **Commit:** `678cb97a`

**2. [Rule 2 - Critical] Added 5th test for SessionHooks-level deny_unknown_fields**

- **Found during:** Task 1 implementation review
- **Issue:** Plan specified 4 tests but only covered SessionHook-level unknown-field rejection, not SessionHooks-level (the `befor` typo case). Both levels have `#[serde(deny_unknown_fields)]`.
- **Fix:** Added `test_session_hooks_rejects_unknown_top_level_field` — verifies `befor` (typo) at the SessionHooks level causes a serde error
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Commit:** `72ee31db`

**3. [Rule 1 - Bug] sandbox_prepare.rs profile_session_hooks pre-move extraction**

- **Found during:** Task 2 implementation
- **Issue:** Initial implementation attempted `session_hooks: loaded_profile_ref.as_ref()...` but the variable was `loaded_profile` which is moved into the same struct literal later
- **Fix:** Extracted `let profile_session_hooks = loaded_profile.as_ref().map(|p| p.session_hooks.clone()).unwrap_or_default()` BEFORE the `finalize_prepared_sandbox(PreparedSandbox { ..., loaded_profile, ... })` move; mirrors the existing `profile_secrets` extraction pattern at lines 507-510
- **Files modified:** `crates/nono-cli/src/sandbox_prepare.rs`
- **Commit:** `678cb97a`

## Verification

1. `cargo test -p nono-cli -- session_hooks` — 8/8 PASS
2. `cargo build -p nono-cli` — CLEAN (no E0063 missing-field errors)
3. `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` — CLEAN
4. `grep -c "SessionHooks" crates/nono-cli/src/profile/mod.rs` — 14 occurrences (struct def, Profile field, ProfileDeserialize field, merge_profiles, default, tests)
5. `grep -c "session_hooks" crates/nono-cli/data/nono-profile.schema.json` — 1 occurrence (property key)
6. Cross-target clippy: PARTIAL/deferred-to-CI per CLAUDE.md MUST rule — dev host is Windows; `profile/mod.rs` has no cfg-gated Unix code itself, but Plans 02/03 will add cfg-gated files (`hook_runtime.rs`, `hook_runtime_windows.rs`) so the cross-target gate is deferred.

## Known Stubs

None. All fields are live data-model wiring. No placeholder values or TODO connections in this plan.

## Threat Flags

No new security-relevant surface introduced in this plan. The `SessionHooks`/`SessionHook` types are data model only — no runtime execution, no new network endpoints, no new file access. Mitigations T-58-01-01 (`deny_unknown_fields`) and T-58-01-02 (schema type validation) are implemented as specified.

## Self-Check: PASSED

- `crates/nono-cli/src/profile/mod.rs` — exists, contains `pub struct SessionHooks`
- `crates/nono-cli/src/policy.rs` — exists, contains `session_hooks: self.session_hooks.clone()`
- `crates/nono-cli/src/sandbox_prepare.rs` — exists, contains `session_hooks: profile::SessionHooks`
- `crates/nono-cli/src/launch_runtime.rs` — exists, contains `session_hooks: crate::profile::SessionHooks`
- `crates/nono-cli/data/nono-profile.schema.json` — exists, contains `"session_hooks"`
- Commit `72ee31db` — verified in git log
- Commit `678cb97a` — verified in git log
