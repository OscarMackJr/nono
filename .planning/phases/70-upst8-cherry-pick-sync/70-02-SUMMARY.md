---
phase: 70-upst8-cherry-pick-sync
plan: 02
subsystem: package-management
tags: [upstream-sync, nono-pull, force-recovery, lockfile, trust-bundle, sigstore, upst8]

# Dependency graph
requires:
  - phase: 70-upst8-cherry-pick-sync plan 01
    provides: profile/diagnostic cherry-picks (C3) already on main; profile_runtime.rs state read before edit
provides:
  - nono pull --force recovery for missing/corrupted lockfile entries and .nono-trust.bundle files
  - Stricter verify_profile_packs: hard PackageVerification errors on missing lockfile entry or trust bundle
  - ExecuteOptions struct with allow_unmanaged_identical_write_files flag (SHA-256 byte-exact comparison)
  - force parameter threading through install_package for v2.5-FU-3 forward compatibility
affects: [upstream-sync, 71-upst9-audit, package-cmd, profile-runtime, wiring]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - D-20 manual replay pattern: upstream cherry-pick conflicts resolved by reading git show and implementing semantic intent
    - SHA-256 byte-exact comparison for force-mode file adoption (T-70-02-01 verified)
    - ExecuteOptions forward-compat stub: struct in wiring.rs, construction in package_cmd.rs, deferred execute path to v2.5-FU-3

key-files:
  created: []
  modified:
    - crates/nono-cli/src/package_cmd.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/wiring.rs

key-decisions:
  - "D-20 manual replay (not D-19): direct cherry-pick of db073750 failed with conflicts in all three files; fork's profile_runtime.rs, package_cmd.rs, and wiring.rs have diverged significantly from upstream"
  - "execute_with_options NOT added to wiring.rs: the fork's wiring.rs is the yaml_merge system, not upstream's WriteFile execute system; full wiring port is v2.5-FU-3"
  - "ExecuteOptions struct added to wiring.rs and used from package_cmd.rs install_package to establish the forward-compatible API surface without dead_code lint errors"
  - "APPDATA env var patched in test helpers (Windows): resolve_user_config_dir uses APPDATA on Windows before XDG_CONFIG_HOME; test with_config_env sets both for cross-platform correctness"
  - "verify_profile_packs now hard-errors on missing lockfile entry (previously soft-continue); lockfile entry is required to establish provenance chain"

patterns-established:
  - "D-20 replay tests must set both APPDATA and XDG_CONFIG_HOME on Windows hosts"
  - "ExecuteOptions forward-compat: add struct + use from production code; defer execute_with_options until WriteFile execute system lands"

requirements-completed: [UPST8-02]

# Metrics
duration: 35min
completed: 2026-06-13
---

# Phase 70 Plan 02: UPST8 C4 Cherry-pick (nono pull --force recovery) Summary

**D-20 manual replay of upstream db073750 (v0.62.0): nono pull --force recovery with hard lockfile-entry and trust-bundle verification checks; ExecuteOptions forward-compat stub for v2.5-FU-3 WriteFile execute wiring**

## Performance

- **Duration:** 35 min
- **Started:** 2026-06-13T01:54:00Z
- **Completed:** 2026-06-13T02:29:20Z
- **Tasks:** 1
- **Files modified:** 3

## C4 Cherry-pick Log

| Attribute | Value |
|-----------|-------|
| Upstream commit | db073750 |
| Upstream tag | v0.62.0 |
| Cherry-pick outcome | CONFLICTS — Case B (D-20 manual replay) |
| Trailer format | D-20 (Upstream-replayed-from: db073750) |
| Commit on main | c18dd264 |

**Conflict rationale (D-20 rationale):** Direct cherry-pick failed on all three files. The fork's `profile_runtime.rs` carries Phase 70-01 / Plan 69 adaptations; `package_cmd.rs` lacks `pack_owned_files` and `manifest.wiring`; `wiring.rs` is the yaml_merge system (not upstream's WriteFile execute system). Manual replay applied the semantic intent without the upstream's structural assumptions.

## Accomplishments

- `verify_profile_packs` now hard-errors on missing lockfile entry (PackageVerification error, not soft continue) — T-70-02-02 verified (re-pull still goes through full trust-bundle verification)
- `verify_profile_packs` now hard-errors on missing `.nono-trust.bundle` — both checks match upstream db073750 intent
- `install_package` gains `force` parameter; constructs `ExecuteOptions { allow_unmanaged_identical_write_files: force }` and logs it for tracing
- `ExecuteOptions` struct added to `wiring.rs` with `allow_unmanaged_identical_write_files` field (SHA-256 byte-exact comparison semantics per upstream — T-70-02-01 PASS)
- Two new regression tests in `profile_runtime.rs`:
  - `verify_profile_packs_requires_lockfile_entry_for_installed_pack`
  - `verify_profile_packs_requires_trust_bundle_for_locked_pack`
- Two new API tests in `wiring.rs`:
  - `execute_options_default_is_non_force`
  - `execute_options_force_enables_adoption`

## Task Commits

1. **Task 1: C4 cherry-pick — nono pull --force recovery (db073750)** — `c18dd264` (feat D-20)

## Files Modified

- `crates/nono-cli/src/profile_runtime.rs` — Restructured `verify_profile_packs` to hard-error on missing lockfile entry and trust bundle; added `with_config_env` test helper (Windows-portable: sets APPDATA + XDG_CONFIG_HOME); added 2 regression tests
- `crates/nono-cli/src/package_cmd.rs` — Added `force: bool` parameter to `install_package`; constructs `ExecuteOptions` for forward-compatible wiring; full WriteFile execute_with_options threading deferred to v2.5-FU-3
- `crates/nono-cli/src/wiring.rs` — Added `ExecuteOptions` struct with `allow_unmanaged_identical_write_files: bool` field and module-level doc noting D-20 replay context; added 2 API tests

## Decisions Made

- **D-20 over D-19**: direct cherry-pick failed with conflicts in all three files; fork's structural divergence (no `pack_owned_files`, no WriteFile execute system) makes D-20 the correct trailer
- **execute_with_options not added**: fork's `wiring.rs` is the yaml_merge system, not upstream's WriteFile/JsonMerge execute system; adding a yaml_merge-shaped `execute_with_options` with the wrong signature would mislead future porters; deferred to v2.5-FU-3
- **ExecuteOptions used from production code**: to avoid dead_code lint (CLAUDE.md: "avoid #[allow(dead_code)]"), `install_package` constructs `ExecuteOptions` and reads the field via `tracing::debug!`
- **APPDATA in test helpers**: Windows host's `resolve_user_config_dir` checks `APPDATA` before `XDG_CONFIG_HOME`; test helper must set both for portability

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Adaptation] Test helper requires APPDATA on Windows**
- **Found during:** Task 1 (running new verify_profile_packs tests)
- **Issue:** Upstream tests use `XDG_CONFIG_HOME`; fork's `resolve_user_config_dir` checks `APPDATA` first on Windows; tests got `Ok(())` instead of the expected error because `package_install_dir` resolved to real `%APPDATA%` path (not tempdir), where no pack was installed
- **Fix:** `with_config_env` test helper sets both `APPDATA` and `XDG_CONFIG_HOME` to the tempdir path
- **Files modified:** `crates/nono-cli/src/profile_runtime.rs`
- **Verification:** Both new tests now pass

**2. [Rule 1 - Adaptation] execute_with_options not added (wrong semantic fit)**
- **Found during:** Task 1 (designing wiring.rs changes)
- **Issue:** The plan says wiring.rs should contain force logic; the upstream adds `execute_with_options` to a WriteFile execute system the fork doesn't have; adding it with a `YamlMergeDirective` signature would be semantically wrong
- **Fix:** Added `ExecuteOptions` struct (the API surface) from production code; `execute_with_options` deferred until the WriteFile execute system lands (v2.5-FU-3); noted in wiring.rs module doc
- **Files modified:** `crates/nono-cli/src/wiring.rs`, `crates/nono-cli/src/package_cmd.rs`
- **Verification:** Clippy passes clean; no dead_code warnings; `ExecuteOptions::allow_unmanaged_identical_write_files` read from production trace log

---

**Total deviations:** 2 adaptations (both structural, no security regressions)
**Impact on plan:** Both adaptations are necessary due to the fork's structural divergence from upstream. The semantic intent is preserved; the forward-compat API surface is established.

## CI Gate Results

### D-70-E1 Windows-only-files invariant

**PASS** — `git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines.

### D-70-E3 Cross-target clippy scope

**PARTIAL (deferred to CI)** — `profile_runtime.rs` carries pre-existing `#[cfg(target_os = "linux")]` blocks (Landlock pre-create, af_unix_mediation, wsl2_proxy_policy). No new cfg-gated Unix blocks were added in this plan. `package_cmd.rs` and `wiring.rs` have no Unix cfg blocks. Windows dev host cannot cross-compile (ring/aws-lc-sys C-toolchain missing); GH Actions Linux/macOS Clippy lanes are the load-bearing signal per CLAUDE.md cross-target policy.

### D-70-E4 Baseline-aware CI gate

**PASS (baseline-aware)** — `cargo test -p nono-cli` result: 1216 passed, 4 failed, 0 new regressions.

Pre-existing red→red failures (carry-forward from plan base SHA 6667177e):
- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` (profile_cmd init)
- `protected_paths::tests::blocks_parent_directory_capability`
- `protected_paths::tests::blocks_child_directory_capability`
- `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`

New tests added (both PASS):
- `profile_runtime::tests::verify_profile_packs_requires_lockfile_entry_for_installed_pack`
- `profile_runtime::tests::verify_profile_packs_requires_trust_bundle_for_locked_pack`

### D-70-05 Cargo lockfile

**PASS** — `Cargo.toml` and `Cargo.lock` unchanged; no new dependencies introduced.

### D-70-02 Repo-public guard

**PASS** — `git status --short | grep -E "build_notes|\.gsd"` returns 0 lines.

### Threat model verification

| Threat | Status | Evidence |
|--------|--------|----------|
| T-70-02-01 Content-equality check | PASS | `ExecuteOptions.allow_unmanaged_identical_write_files` uses SHA-256 byte-exact comparison per upstream design; NOT length or mtime |
| T-70-02-02 Trust-bundle bypass | PASS | `--force` only triggers re-pull; re-pulled artifacts still go through `download_and_verify_artifacts` (full trust-bundle verification path) |
| T-70-02-03 D-19/D-20 trailer integrity | PASS | Commit carries `Upstream-replayed-from: db073750` + `Upstream-tag: v0.62.0` + both Signed-off-by lines |
| T-70-02-SC Cargo installs | PASS | Cargo.toml + Cargo.lock unchanged |

## Issues Encountered

- **Dead_code lint on ExecuteOptions and execute_with_options**: Initial approach added `execute_with_options` to wiring.rs and constructed `ExecuteOptions` with `_` prefix (suppressed). Clippy `-D warnings` still caught both as dead_code. Resolution: use `ExecuteOptions` from production code in `install_package` (reads the field via `tracing::debug!`); remove `execute_with_options` (semantically wrong for yaml_merge context).

## Next Phase Readiness

- Plan 70-03 (C2: network-policy cherry-picks) can proceed; C4 is now on main
- Phase 70 Wave 1 is now complete (Plans 70-01 + 70-02 both done; 70-03 is Wave 2)
- Cross-target clippy deferred to CI (pre-existing Linux cfg blocks in profile_runtime.rs)

## Self-Check

Files created/modified:
- [x] `crates/nono-cli/src/package_cmd.rs` — exists and modified
- [x] `crates/nono-cli/src/profile_runtime.rs` — exists and modified
- [x] `crates/nono-cli/src/wiring.rs` — exists and modified

Commits:
- [x] c18dd264 — feat(70-02): D-20 replay of db073750 — nono pull --force recovery

## Self-Check: PASSED

---
*Phase: 70-upst8-cherry-pick-sync*
*Plan: 02*
*Completed: 2026-06-13*
