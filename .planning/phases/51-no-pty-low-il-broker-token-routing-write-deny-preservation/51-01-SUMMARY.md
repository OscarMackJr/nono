---
phase: 51
plan: "01"
subsystem: exec_strategy_windows
tags: [windows, token-arm, low-il, broker, profile, tdd]
dependency_graph:
  requires: []
  provides:
    - WindowsTokenArm::BrokerLaunchNoPty variant
    - select_windows_token_arm 5-param signature
    - ExecConfig.prefers_low_il_broker field
    - Profile.windows_low_il_broker field
  affects:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/data/nono-profile.schema.json
tech_stack:
  added: []
  patterns:
    - TDD RED→GREEN on pure-function cascade test (pty_token_gate_tests)
    - Profile field threading: policy.json → ProfileDef → Profile → ExecConfig
    - Cascade arm insertion: ordered AFTER has_pty, BEFORE has_session_sid
key_files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/network.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/data/nono-profile.schema.json
decisions:
  - "D-01: cross-platform Profile field — windows_low_il_broker: bool with #[serde(default)]; ProfileDeserialize updated atomically with Profile struct to satisfy deny_unknown_fields"
  - "D-02: WriteRestricted NOT removed — BrokerLaunchNoPty arm inserted BEFORE WriteRestricted, with WriteRestricted still reachable when prefers_low_il_broker=false"
  - "D-03: Only claude-code built-in profile gets windows_low_il_broker: true in v2.7 policy.json"
  - "D-06: Distinct variant BrokerLaunchNoPty (not reusing BrokerLaunch) so PTY-path tests keep asserting BrokerLaunch, proving Phase 31 PTY path is untouched"
metrics:
  duration: "1222s (~20 minutes)"
  completed: "2026-05-26"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 8
requirements_closed: [REQ-WSRH-02, REQ-WSRH-05]
---

# Phase 51 Plan 01: Broker No-PTY Opt-In Field + Cascade Extension Summary

Profile-gated opt-in field `windows_low_il_broker` threaded from `policy.json` through `ProfileDeserialize`/`Profile` to `ExecConfig`, plus new `WindowsTokenArm::BrokerLaunchNoPty` variant inserted into the `select_windows_token_arm` cascade as the third arm (after PTY, before WriteRestricted).

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add Profile.windows_low_il_broker field + policy.json + schema | 31696f81 | profile/mod.rs, policy.rs, policy.json, nono-profile.schema.json |
| 2 | Add ExecConfig.prefers_low_il_broker + BrokerLaunchNoPty + cascade + unit tests | cafcfa29 | launch.rs, mod.rs, network.rs, execution_runtime.rs |

## What Was Built

**Task 1** adds the profile field chain:
- `Profile.windows_low_il_broker: bool` with `#[serde(default)]`; doc comment calls out Windows-only semantics + Linux/macOS no-op
- Matching field in `ProfileDeserialize` (MUST be atomic with Profile — deny_unknown_fields constraint T-51A-03)
- `From<ProfileDeserialize>` mapping includes the field
- `merge_profiles` uses OR semantics: `base.windows_low_il_broker || child.windows_low_il_broker` (same as `interactive`)
- `ProfileDef.windows_low_il_broker` added to `policy.rs` so `to_raw_profile` can forward the field from built-in profiles
- `"windows_low_il_broker": true` added to `claude-code` profile block in `policy.json` (exactly 1 occurrence, D-03)
- Schema property added to `nono-profile.schema.json` after `unsafe_macos_seatbelt_rules`
- 8 TDD unit tests in `windows_low_il_broker_tests` module covering deserialization, defaults, merge semantics, and built-in profile correctness

**Task 2** adds the cascade extension:
- `WindowsTokenArm::BrokerLaunchNoPty` variant (Phase 51 D-06) inserted after `BrokerLaunch` with doc comment explaining the distinct-variant rationale
- `select_windows_token_arm` extended to 5 parameters; new third branch: `prefers_low_il_broker && has_session_sid → BrokerLaunchNoPty`
- Cascade ordering: Null (detached) → BrokerLaunch (PTY) → BrokerLaunchNoPty (no-PTY + opt-in + session-SID) → WriteRestricted (session-SID, no opt-in) → LowIlPrimary → Null
- `BrokerLaunchNoPty` token arm in spawn_windows_child: `_restricted_holder = None; _low_integrity_holder = None; null_mut()` (identical to BrokerLaunch — broker handles Low-IL construction internally)
- Call site updated to pass `config.prefers_low_il_broker` as 5th argument
- `ExecConfig.prefers_low_il_broker: bool` added to `exec_strategy_windows/mod.rs`
- `execution_runtime.rs` Windows ExecConfig literal gains `prefers_low_il_broker: loaded_profile.as_ref().map_or(false, |p| p.windows_low_il_broker)`
- All 7 existing `pty_token_gate_tests` updated with `false` as 5th argument; new test `pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty` asserts `BrokerLaunchNoPty` for `(false, false, true, false, true)`
- ExecConfig literals in `launch.rs` test helper and 2 in `network.rs` tests fixed with `prefers_low_il_broker: false`

## Test Results

- `cargo test -p nono-cli pty_token_gate_tests`: **8 tests pass** (7 existing + 1 new)
- `cargo test -p nono-cli windows_low_il_broker_tests`: **8 tests pass** (all new)
- `cargo test -p nono-cli profile`: **350 tests pass** (1 pre-existing failure excluded — see Known Issues)
- `cargo build -p nono-cli`: **clean build**, no errors

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ProfileDef in policy.rs also required the new field**

- **Found during:** Task 1 GREEN phase — build revealed `missing field windows_low_il_broker` in `policy.rs:154`
- **Issue:** `ProfileDef` struct (used for built-in policy.json deserialization) has an exhaustive struct literal in `to_raw_profile()`. Adding the field to `Profile` without also adding it to `ProfileDef` caused a compile error.
- **Fix:** Added `pub windows_low_il_broker: bool` to `ProfileDef` + forwarded it in `to_raw_profile()`. This is the correct path — built-in profiles declare the field in `policy.json`, `ProfileDef` deserializes it, `to_raw_profile` converts it to `Profile`.
- **Files modified:** `crates/nono-cli/src/policy.rs`
- **Commit:** 31696f81

**2. [Rule 1 - Bug] ExecConfig literals in launch.rs and network.rs also needed the new field**

- **Found during:** Task 2 GREEN phase — build revealed `missing field prefers_low_il_broker` at 3 sites
- **Issue:** Three test-helper ExecConfig literals in `launch.rs:2807` and `network.rs:1619,1675` (all in test modules) used exhaustive struct syntax. Adding `prefers_low_il_broker` to `ExecConfig` required updating them.
- **Fix:** Added `prefers_low_il_broker: false` to each test-helper literal (tests don't exercise the broker path; false is correct).
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/launch.rs`, `crates/nono-cli/src/exec_strategy_windows/network.rs`
- **Commit:** cafcfa29

## Known Issues (Pre-existing, Out of Scope)

The following test failures existed before this plan and are NOT caused by this plan's changes. Verified by confirming none of the failing tests reference any field or function introduced in this plan:

1. `protected_paths::tests::blocks_parent_directory_capability` — panics at `expect_err("blocked")` because `validate_caps_against_protected_roots` returns `Ok` in the test environment. Unrelated to profile fields or token cascade.
2. `protected_paths::tests::blocks_child_directory_capability` — same root cause.
3. `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root` — same root cause.
4. `exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object` — requires a running Windows WFP/broker service context; environmental.
5. `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` — panics because `%APPDATA%\nono\profiles\my-agent.json` already exists in the test environment (leaked test fixture from a prior run). Not caused by `windows_low_il_broker` addition.

All 5 failures are pre-existing and fall outside this plan's scope. Logged for deferred-items awareness; not blocking.

## Cross-Target Clippy Status

`execution_runtime.rs` contains `#[cfg(target_os = "linux")]` blocks and falls within the cross-target clippy scope per CLAUDE.md. My addition to that file is solely in the `#[cfg(target_os = "windows")]` block, but the checklist requires live-CI verification.

Cross-target clippy SKIPPED on Windows dev host due to missing C cross-compiler (`x86_64-linux-gnu-gcc` not found for `ring`/`aws-lc-sys`). The live GH Actions Linux Clippy and macOS Clippy lanes on the head SHA are the decisive signals per `.planning/templates/cross-target-verify-checklist.md`. REQ-WSRH-02 and REQ-WSRH-05 marked PARTIAL pending CI confirmation.

## Known Stubs

None. All wiring is real:
- `windows_low_il_broker` field flows from `policy.json` → `ProfileDef` → `Profile` → `ExecConfig.prefers_low_il_broker`
- `select_windows_token_arm` with 5 params is called from `spawn_windows_child` with `config.prefers_low_il_broker`
- `BrokerLaunchNoPty` token arm produces the correct null-token for the broker spawn path

The spawn wiring that uses `BrokerLaunchNoPty` at the `match arm` level in `spawn_windows_child` is intentionally deferred to Plan 51-03 (the plan's stated scope). The `BrokerLaunchNoPty` match arm currently returns null h_token (identical to `BrokerLaunch`); Plan 51-03 adds the anonymous-pipe dispatch.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond those enumerated in the plan's `<threat_model>`.

- T-51A-01 (cascade ordering EoP): mitigated — WriteRestricted arm remains 4th; `pty_none_with_session_sid_selects_write_restricted` (prefers_low_il_broker=false) structurally proves WriteRestricted reachability.
- T-51A-02 (field widens to non-claude-code profiles): mitigated — only claude-code profile has `"windows_low_il_broker": true` in policy.json; 1 occurrence confirmed.
- T-51A-03 (ProfileDeserialize missing field crash): mitigated — both Profile and ProfileDeserialize updated atomically; rustc struct-literal completeness check catches future mismatches.
- T-51A-04 (Phase 31 PTY path regression): mitigated — `pty_some_no_detach_selects_broker_launch` still asserts `BrokerLaunch`; `BrokerLaunchNoPty` arm is only reached when `has_pty=false`.

## Self-Check: PASSED

| Item | Result |
|------|--------|
| SUMMARY.md exists | FOUND |
| Commit 31696f81 (Task 1) | FOUND |
| Commit cafcfa29 (Task 2) | FOUND |
| profile/mod.rs exists | FOUND |
| launch.rs exists | FOUND |
| policy.json exists | FOUND |
