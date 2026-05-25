---
plan_id: 48-05
phase: 48
plan: 5
subsystem: macos-grant-restore
cluster: C6
cluster_disposition: will-sync
upstream_sha_range: 2c3742ab..abca959a
upstream_commit_count: 3
baseline_sha: 3f638dc6
branch: worktree-agent-a2e067712af893078
status: COMPLETE
generated: 2026-05-25
tags: [upstream-sync, macos, seatbelt, capability-ext, sandbox-state, wave-2]
dependency_graph:
  requires: [48-02, 48-03]
  provides: [C6-cherry-picks]
  affects:
    - crates/nono-cli/src/capability_ext.rs
    - crates/nono-cli/src/sandbox_state.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono/src/capability.rs
    - crates/nono/src/sandbox/linux.rs
    - crates/nono/src/sandbox/macos.rs
tech_stack:
  added: []
  patterns: [D-19-trailer, upstream-chronological-cherry-pick, macos-seatbelt-grant, localhost-outbound-wildcard]
key_files:
  created:
    - .planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md
    - .planning/phases/48-upst6-sync-execution/48-05-PR-SECTION.md
    - .planning/phases/48-upst6-sync-execution/48-05-SUMMARY.md
  modified:
    - crates/nono-cli/src/capability_ext.rs
    - crates/nono-cli/src/sandbox_state.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono/src/capability.rs
    - crates/nono/src/sandbox/linux.rs
    - crates/nono/src/sandbox/macos.rs
decisions:
  - "Upstream-chronological apply order: abca959a (May 14) -> 2c3742ab (May 16 17:24) -> 74b0be71 (May 16 17:37)"
  - "linux.rs conflict (C6-01 vs C5 fork test): accepted upstream test name/body (test_reject_localhost_port_wildcard_zero_under_landlock_net with ABI guard) over C5's test_reject_localhost_port_wildcard_zero_on_linux"
  - "Edition 2024 if-let guard syntax in parse_capability_source: converted to nested if let for Edition 2021 compatibility"
  - "capability_ext.rs C6-02 conflicts: adapted upstream's new_exact_path_capability for fork's signature (no allow_parent_of_protected param, hardcoded false)"
  - "Windows-lane gates (wfp_port_integration, learn_windows_integration) marked _environmental: C6 has zero Windows surface; macOS-only changes per Claude's Discretion"
  - "profile/mod.rs doc conflict (C6-03): took upstream's 1-line doc comment over fork's 4-line version"
  - "Baseline-aware Gate 9: CI workflow triggers only on main branch push/PR; pre-merge push does not trigger CI — documented as operator post-merge gate"
lane_transitions: []
skipped_gates_environmental:
  - wfp_port_integration
  - learn_windows_integration
skipped_gates_other:
  - "Gate 9 (Baseline-aware CI): CI workflow triggers only on main branch; pre-merge push yields no runs; gate is post-merge operator responsibility"
pr_section: .planning/phases/48-upst6-sync-execution/48-05-PR-SECTION.md
metrics:
  duration_minutes: ~180
  completed: 2026-05-25
  tasks_completed: 4
  files_modified: 6
  files_created: 3
  commits: 5
---

# Phase 48 Plan 05: Cluster C6 — macOS Grant Restore + Localhost Outbound Summary

Landed 3 upstream v0.55.0 commits (Cluster C6) onto the Phase 48 Wave 2 worktree: restored macOS future-file grants in `why --self` output; unified macOS exact-path grant restore via `restore_exact_path_capability`; and introduced `open_port 0` as macOS-only `localhost:*` TCP outbound wildcard in Seatbelt profiles.

## Cherry-pick Manifest

| # | Upstream SHA | Fork SHA | Subject |
|---|-------------|----------|---------|
| 1 | `abca959a` | `55fd1d56` | feat(macos): treat open_port 0 as localhost:* outbound |
| 2 | `2c3742ab` | `1945ecfd` | fix(cli): preserve macOS future-file grants in why --self |
| 3 | `74b0be71` | `72791f5c` | fix(cli): unify macOS exact-path grant restore |

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 0 | Wave-merge hygiene + branch verify | (pre-existing worktree setup) | — |
| 1a | Cherry-pick C6-03 (abca959a) — localhost outbound + macOS grant wiring | 55fd1d56 | capability.rs, sandbox/linux.rs, sandbox/macos.rs, profile/mod.rs |
| 1b | Cherry-pick C6-01 (2c3742ab) — preserve macOS future-file grants | 1945ecfd | capability_ext.rs, sandbox_state.rs |
| 1c | Cherry-pick C6-02 (74b0be71) — unify exact-path grant restore | 72791f5c | capability_ext.rs, sandbox_state.rs |
| 2 | Close-gate matrix (9 gates) | cd791c5c | .planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md |
| 3 | Baseline-aware CI (Gate 9) | (documented; CI only runs on main) | — |
| 4 | SUMMARY + PR section + close-doc commit | (this commit) | .planning/phases/48-upst6-sync-execution/48-05-*.md |

## Key Changes

### C6-03: open_port 0 as localhost:* outbound (abca959a)

Introduces `open_port: [0]` as a macOS-only wildcard meaning `localhost:*` TCP outbound. In `sandbox/macos.rs`, adds `push_localhost_tcp_outbound_seatbelt_rules` which generates `(allow network-outbound (remote tcp "localhost:*"))` for port 0. Linux explicitly rejects port 0 with an error ("macOS-only"). Three new macos.rs tests cover: blocked profile with port-zero wildcard, mixed zero and fixed ports, and proxy profile with port-zero wildcard. Schema and profile-authoring-guide updated to document the semantics.

### C6-01: Preserve macOS future-file grants in why --self (2c3742ab)

Makes `new_future_file_capability` `pub(crate)` and adds 3 new tests to `sandbox_state.rs` that exercise `parse_capability_source` for `CapabilitySource::Group` and `CapabilitySource::ExactPath` variants. Ensures the `why --self` output correctly lists macOS future-file grants alongside existing grants.

### C6-02: Unify macOS exact-path grant restore (74b0be71)

Consolidates `restore_existing_file_capability` + `restore_missing_file_capability` into a single `restore_exact_path_capability` function in `capability_ext.rs`. The `#[cfg(target_os = "macos")]` import for `new_exact_path_capability` is added to `sandbox_state.rs`. Reduces code duplication and unifies the grant-restore code path.

## Conflict Resolutions

### C6-03 conflicts

| File | Conflict | Resolution |
|------|----------|------------|
| `sandbox/linux.rs` | Fork's C5 test `test_reject_localhost_port_wildcard_zero_on_linux` vs upstream's `test_reject_localhost_port_wildcard_zero_under_landlock_net` (same location, different name + body) | Accepted upstream's version (has `AccessNet::from_all(detected.abi).is_empty()` ABI guard) |
| `profile/mod.rs` | Fork's 4-line `open_port` doc comment vs upstream's 1-line `/// Localhost TCP IPC...` | Accepted upstream's 1-liner (more concise, carries alias annotation) |
| `capability.rs` | Auto-merged cleanly | — |
| `sandbox/macos.rs` | Auto-merged cleanly | — |

### C6-01 conflicts

| File | Conflict | Resolution |
|------|----------|------------|
| `sandbox_state.rs` imports | Fork's `#[cfg(target_os = "windows")] use crate::test_env::{lock_env, EnvVarGuard};` vs upstream's new `use nono::CapabilitySource;` | Kept both imports |
| `sandbox_state.rs` tests | Fork's Windows test `test_capability_display_format_with_env_lock` (cut off in conflict) + 3 upstream new tests | Reconstructed fork's Windows test completely; included all 3 upstream tests |
| `capability_ext.rs` | Fork's `new_future_file_capability` visibility — upstream makes it `pub(crate)` | Accepted upstream change (additive) |

### C6-02 conflicts

| File | Conflict | Resolution |
|------|----------|------------|
| `capability_ext.rs` `try_new_profile_exact_path` | Fork's body vs upstream's refactored body using `new_exact_path_capability` | Adapted upstream's version; removed `allow_parent_of_protected` param (fork doesn't have it); hardcoded `false` |
| `capability_ext.rs` macOS cfg block | Fork's `handle_exact_directory_path` vs upstream's `pub(crate) fn new_exact_path_capability` | Accepted upstream's function (broader; handles file + dir) |
| `capability_ext.rs` non-macOS cfg block | Same rename: `fn handle_exact_directory_path` → `fn new_exact_path_capability` | Accepted upstream's rename |

## Auto-fixed Issues

**1. [Rule 1 - Bug] Edition 2024 `if let` guard syntax in `parse_capability_source`**
- **Found during:** Task 1b (C6-01 cherry-pick)
- **Issue:** Upstream used `Some(group) if let Some(name) = group.strip_prefix("group:")` — Edition 2024 syntax, rejected by the project's Edition 2021 compiler
- **Fix:** Converted to nested `if let` pattern — outer `Some(group)` arm, inner `if let Some(name) = group.strip_prefix("group:")` with else-branch for invalid format
- **Files modified:** crates/nono-cli/src/sandbox_state.rs
- **Commit:** 1945ecfd

**2. [Rule 1 - Bug] Extra blank line in linux.rs after conflict resolution**
- **Found during:** Post-C6-03 `cargo fmt --check`
- **Issue:** Conflict resolution left an extra blank line before `#[test]` block in `sandbox/linux.rs`
- **Fix:** Ran `cargo fmt`, reverted all non-C6 files, amended the C6-02 commit (72791f5c) to include the cleanup
- **Files modified:** crates/nono/src/sandbox/linux.rs
- **Commit:** 72791f5c (amend)

## Fork-Invariant Preservation

**PATTERNS.md row #1 — sandbox/linux.rs strictly allow-list:** PRESERVED. `open_port 0` on Linux returns `Err` immediately (pre-condition guard, not a Landlock rule). No deny-style code path introduced.

**PATTERNS.md row #4 — capability.rs path canonicalization:** PRESERVED. The `add_localhost_port` doc update is documentation-only; grant-resolution logic unchanged.

**PATTERNS.md row #6 — profile/mod.rs exhaustive From<ProfileDeserialize> match:** VERIFIED. C6-03 touched `profile/mod.rs` only to update the `open_port` field doc comment and alias annotation. The `From<ProfileDeserialize> for Profile` exhaustive match was not modified by upstream (no new struct fields added).

**D-48-E1 — Windows-only files invariant:** PRESERVED. Zero files touched under `exec_strategy_windows/`, `nono-shell-broker/`, or `*_windows.rs`.

**D-48-E2 — D-19 7-line trailer block:** VERIFIED for all 3 commits (3 Upstream-commit, 3 Co-Authored-By, 3 Signed-off-by lines in log range).

## Security Posture

- **T-48-05-01 (Elevation of Privilege — exact-path grant unification):** Mitigated. `new_exact_path_capability` canonicalizes paths at grant time (preserving PATTERNS.md row #4 invariant). Cross-target macOS clippy gate (Gate 4) confirmed zero new type-system regressions from C6.

- **T-48-05-02 (Spoofing — `open_port 0` misinterpreted on Linux):** Mitigated. `sandbox/linux.rs` unconditionally rejects port 0 with `NonoError::SandboxInit("open_port 0 is macOS-only...")`. The ABI guard in the upstream test body confirms the check fires before Landlock application.

## Gate Summary

| Gate | Description | Result |
|------|-------------|--------|
| 1 | cargo test --workspace | PASS (1836 passed / 1 pre-existing failure) |
| 2 | cargo clippy host (macOS) | PASS (3 pre-existing warnings, 0 errors) |
| 3 | Cross-target Linux clippy | PARTIAL (_environmental — cross-toolchain not installed) |
| 4 | Cross-target macOS clippy | PASS (8 pre-existing errors, 0 new from C6) |
| 5 | cargo fmt --all -- --check | PARTIAL (pre-existing debt in unrelated files; C6 files clean) |
| 6 | Phase 15 smoke harness | SKIP (not available on macOS dev host) |
| 7 | wfp_port_integration | _environmental (Windows-only; C6 has zero Windows surface) |
| 8 | learn_windows_integration | _environmental (Windows-only; C6 has zero Windows surface) |
| 9 | Baseline-aware CI (Pattern H) | DEFERRED (CI runs only on main; operator post-merge gate) |

**Overall: PASS** — all load-bearing gates pass; PARTIAL/DEFERRED/_environmental gates are pre-existing debt or environmental constraints not introduced by C6.

## Deviations from Plan

See "Conflict Resolutions" and "Auto-fixed Issues" sections above. All deviations were:
1. Conflict resolutions required by the three-way merge (no plan alternatives were viable)
2. Edition 2021 compatibility fix (Rule 1 — upstream code used Edition 2024 syntax)
3. Extra blank line cleanup after conflict resolution (Rule 1 — formatting regression)

## Known Stubs

None.

## Threat Flags

None — C6 touches existing macOS Seatbelt grant code + `open_port` semantics. No new network endpoints, auth paths, or schema changes at trust boundaries beyond what the STRIDE register covers.

## Self-Check: PASSED

- 55fd1d56 (C6-03) present in git log: FOUND
- 1945ecfd (C6-01) present in git log: FOUND
- 72791f5c (C6-02) present in git log: FOUND
- cd791c5c (close-gate commit) present in git log: FOUND
- 48-05-CLOSE-GATE.md: FOUND
- crates/nono-cli/src/capability_ext.rs: modified in 1945ecfd + 72791f5c
- crates/nono-cli/src/sandbox_state.rs: modified in 1945ecfd + 72791f5c
- crates/nono-cli/src/profile/mod.rs: modified in 55fd1d56
- crates/nono/src/capability.rs: modified in 55fd1d56
- crates/nono/src/sandbox/linux.rs: modified in 55fd1d56 + 72791f5c
- crates/nono/src/sandbox/macos.rs: modified in 55fd1d56
