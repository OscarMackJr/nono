---
phase: 71-engine-agnostic-launch-productionization
plan: "01"
subsystem: profile
tags: [engine-profiles, windows, policy, schema, profile-contract]
dependency_graph:
  requires: []
  provides: [windows_interpreters-field, aider-profile, langchain-python-profile]
  affects: [plan-03-coverage-gate, plan-04-launch-path]
tech_stack:
  added: []
  patterns: [serde-default-field, policy-json-profile-extension, exhaustive-struct-literal]
key_files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/src/profile/builtin.rs
decisions:
  - "windows_interpreters uses Vec<String> + #[serde(default)] identical to skipdirs/packs/command_args — cross-platform deserialize, Windows-only runtime"
  - "merge_profiles uses dedup_append semantics for windows_interpreters (union, not override) consistent with other collection fields"
  - "ProfileDeserialize also carries windows_interpreters due to deny_unknown_fields — required for round-trip correctness"
  - "aider and langchain-python profiles share identical shape: groups [python_runtime, git_config, unlink_protection], broker:true, interpreters:[python.exe]"
metrics:
  duration_minutes: 8
  completed_date: "2026-06-14"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 5
---

# Phase 71 Plan 01: Engine-Profile Contract — Summary

## One-liner

Added `windows_interpreters: Vec<String>` field to the Profile contract and shipped two built-in engine profiles (`aider`, `langchain-python`) in policy.json, establishing the D-01/D-02/D-03/ENG-03 engine-profile interface consumed by the coverage gate in Plans 03/04.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add windows_interpreters field to Profile struct + schema | `4639d302` | profile/mod.rs, nono-profile.schema.json, policy.rs |
| 2 | Add aider + langchain-python engine profiles to policy.json | `f7995041` | data/policy.json |
| 3 | Profile-load test assertions for both engine profiles | `ffd76b0c` | profile/builtin.rs, profile/mod.rs |

## What Was Built

### Task 1 — `windows_interpreters` field

Added `pub windows_interpreters: Vec<String>` with `#[serde(default)]` to the `Profile` struct (mod.rs), adjacent to `windows_low_il_broker`. The field:
- Deserializes on all platforms; runtime use is Windows-only (same convention as `windows_low_il_broker`)
- Empty = no interpreter coverage required (safe default via `Vec::default()`)
- Wired into `ProfileDeserialize` (required by `#[serde(deny_unknown_fields)]`), `From<ProfileDeserialize> for Profile`, and `merge_profiles` (dedup_append union semantics)
- Added to `ProfileDef` in `policy.rs` and forwarded from `to_raw_profile()`
- Mirrored as optional `array`-of-`string` property in `nono-profile.schema.json`

### Task 2 — `aider` and `langchain-python` profiles

Added two new entries to the `"profiles"` object in `policy.json`:
- Both extend `"default"` (enforced by `test_embedded_profiles_extend_default`)
- Both declare: `groups: [python_runtime, git_config, unlink_protection]`, `signal_mode: isolated`, `filesystem: {}`, `network.block: false`, `workdir.access: readwrite`, `windows_low_il_broker: true`, `windows_interpreters: ["python.exe"]`
- `python-dev` untouched (no broker flag added — anti-pattern per plan)

### Task 3 — Test assertions

Added `test_get_builtin_aider` and `test_get_builtin_langchain_python` to `profile/builtin.rs`, each asserting: `meta.name`, `workdir.access == ReadWrite`, presence of `python_runtime`/`git_config`/`unlink_protection` groups, `windows_low_il_broker == true`, and `windows_interpreters == ["python.exe"]`.

Also fixed `base_profile()` and `child_profile()` test helpers in `profile/mod.rs` to include `windows_interpreters: Vec::new()` (required by Rust's exhaustive struct-literal completeness check — Rule 3 auto-fix).

## Verification

- `cargo build -p nono-cli` — clean (no warnings, no errors)
- `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` — clean
- `cargo test -p nono-cli -- test_get_builtin_aider test_get_builtin_langchain_python test_embedded_profiles_extend_default` — 3/3 PASS
- Python JSON validation: `assert 'aider' in p and 'langchain-python' in p; assert p['aider']['windows_low_il_broker'] is True ...` — ok

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Exhaustive struct-literal completeness in test helpers**
- **Found during:** Task 3 (test compilation)
- **Issue:** `base_profile()` and `child_profile()` in `profile/mod.rs` are exhaustive struct literals; adding `windows_interpreters` to `Profile` caused E0063 compile errors in these test helpers
- **Fix:** Added `windows_interpreters: Vec::new()` to both test helper initializers
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Commit:** `ffd76b0c`

**2. [Rule 3 - Blocking] Exhaustive struct-literal completeness in policy.rs and merge_profiles**
- **Found during:** Task 1 (build verification)
- **Issue:** `to_raw_profile()` in `policy.rs` and `merge_profiles()` in `profile/mod.rs` are exhaustive struct literals; adding `windows_interpreters` to `Profile` caused E0063 compile errors
- **Fix:** Added `windows_interpreters` forwarding in `to_raw_profile()` and `dedup_append` merge in `merge_profiles()`; also added field to `ProfileDef` struct in `policy.rs`
- **Files modified:** `crates/nono-cli/src/policy.rs`, `crates/nono-cli/src/profile/mod.rs`
- **Commit:** `4639d302`

## Known Stubs

None. The `windows_interpreters` field is declared and deserializes correctly. The runtime gate (consume field at launch) is deferred to Plans 03/04 by design — this plan delivers only the contract half. The profiles ship with real data, not placeholders.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond what the plan's threat model describes. The new schema property is data-only (no new code paths). The two new profiles declare `filesystem: {}` (no baked path), which is structurally narrower than existing profiles.

## Self-Check: PASSED

Files created/modified exist:
- `crates/nono-cli/src/profile/mod.rs` — FOUND
- `crates/nono-cli/src/policy.rs` — FOUND
- `crates/nono-cli/data/nono-profile.schema.json` — FOUND
- `crates/nono-cli/data/policy.json` — FOUND
- `crates/nono-cli/src/profile/builtin.rs` — FOUND

Commits verified:
- `4639d302` — FOUND (feat: windows_interpreters field)
- `f7995041` — FOUND (feat: aider + langchain-python profiles)
- `ffd76b0c` — FOUND (test: profile-load assertions)
