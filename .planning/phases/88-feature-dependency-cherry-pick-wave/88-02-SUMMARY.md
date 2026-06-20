---
phase: 88-feature-dependency-cherry-pick-wave
plan: 02
subsystem: state-paths, config, audit, rollback
tags: [xdg, state-dirs, cherry-pick, windows, localappdata, audit-ledger, rollback, migration]

# Dependency graph
requires:
  - phase: 88-feature-dependency-cherry-pick-wave plan 01
    provides: Profile/mod.rs with user_profile_draft_dir and set_vars groundwork

provides:
  - state_paths.rs as single source of truth for all runtime state paths
  - XDG state dir migration (audit, rollback, sessions) from ~/.nono/ to ~/.local/state/nono/
  - D-01 config/mod.rs user_state_dir() delegates to state_paths::user_state_dir()
  - D-02 Windows %LOCALAPPDATA%\nono arm in state_paths::user_state_dir()
  - D-03 fail-secure migration: maybe_migrate_legacy_audit_ledger() errors propagate as Err
  - ProtectedRoots::from_defaults() normalized via resolve_path() for UNC path compatibility

affects: [88-03, 88-04, 88-05, 88-06, audit, rollback, session management, provisioner coexistence]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-01 delegation: config/mod.rs thin-delegate pattern for state paths"
    - "D-02 Windows arm: #[cfg(target_os=\"windows\")] reading %LOCALAPPDATA%"
    - "D-03 fail-secure migration: ? propagation, never .ok() or .unwrap_or_default()"
    - "PARTIAL→CI: #[cfg_attr(target_os = \"windows\", allow(dead_code))] for XDG-only functions"
    - "UNC path normalization: resolve_path() applied to state roots for component comparison"

key-files:
  created:
    - crates/nono-cli/src/state_paths.rs
  modified:
    - crates/nono-cli/src/config/mod.rs
    - crates/nono-cli/src/audit_session.rs
    - crates/nono-cli/src/rollback_session.rs
    - crates/nono-cli/src/session.rs
    - crates/nono-cli/src/protected_paths.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/profile_save_runtime.rs
    - crates/nono-cli/src/wiring.rs
    - crates/nono-cli/src/rollback_commands.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/cli.rs
    - tests/integration/test_bypass_protection.sh
    - .planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md

key-decisions:
  - "D-01: config/mod.rs user_state_dir() is a thin delegate; no inline dirs:: chain remains"
  - "D-02: Windows state root reads %LOCALAPPDATA%, validated non-empty — fail-secure, no default fallback"
  - "D-03: migration errors abort with Err, never swallowed — state split is an integrity failure"
  - "ProtectedRoots::from_defaults() applies resolve_path() to strip UNC prefix from try_canonicalize output"
  - "profile_save_runtime.rs dead functions (suggested_run_profile_name, profile_name_from_command) removed per CLAUDE.md no-dead-code policy"
  - "wiring.rs upstream functions requiring WiringRecord/ReversalFailure types kept in DU hold; not imported"
  - "XDG-path test functions gated #[cfg(not(target_os = \"windows\"))] / #[cfg(unix)] — PARTIAL→CI"

patterns-established:
  - "state_paths module: single import point for all runtime state dir resolution"
  - "PARTIAL→CI annotation in 88-PARTIAL-CI.md for every cfg-gated non-Windows code path"

requirements-completed: [FEAT-02]

# Metrics
duration: 185min
completed: 2026-06-20
---

# Phase 88 Plan 02: XDG State Dirs Cherry-Pick (FEAT-02) Summary

**Cherry-picked upstream commits e8293b36 + 8e0d94f9 to create state_paths.rs as the single source of truth for all runtime state paths, migrating audit/rollback/session state from ~/.nono/ to XDG ~/.local/state/nono/ (Unix) or %LOCALAPPDATA%\nono (Windows), with D-01/D-02/D-03 fork reconciliations applied.**

## Performance

- **Duration:** ~185 min
- **Started:** 2026-06-20T12:00:00Z (approx)
- **Completed:** 2026-06-20T15:48:16Z
- **Tasks:** 3/3 completed
- **Files modified:** 20 (across 3 commits)

## Accomplishments

- Created `crates/nono-cli/src/state_paths.rs` with `user_state_dir()`, `audit_root()`, `rollback_root()`, `sessions_dir()`, `protected_state_roots()`, and `maybe_migrate_legacy_audit_ledger()` — all runtime state paths now originate from one module.
- Migrated all direct `nono_home_dir()` state-path constructions in `audit_session.rs` and `rollback_session.rs` to delegate through `state_paths::audit_root()` / `state_paths::rollback_root()`.
- Applied all three fork reconciliation constraints: D-01 (config/mod.rs thin delegate), D-02 (Windows %LOCALAPPDATA% arm with fail-secure validation), D-03 (migration errors propagate as Err via `?`).
- Fixed `ProtectedRoots::from_defaults()` UNC path mismatch: `try_canonicalize` returns `\\?\C:\...` while `validate_requested_path_against_protected_roots` uses `resolve_path()` (strips UNC prefix); now both sides normalize via `resolve_path()`.
- Cleared all Windows CI failures: stale NONO_TEST_HOME tests replaced with platform-appropriate LOCALAPPDATA/XDG tests; XDG-only test functions gated `#[cfg(not(target_os = "windows"))]` / `#[cfg(unix)]`.

## Task Commits

1. **Task 1: Cherry-pick e8293b36 (XDG state_paths.rs creation) with D-01/D-02/D-03** - `0a09ff41` (feat)
2. **Task 2: Cherry-pick 8e0d94f9 (XDG config path consistency)** - `74c5ac23` (fix)
3. **Task 3: make ci gate — Windows CI fixes for XDG cherry-picks** - `de553185` (fix)

## Files Created/Modified

- `crates/nono-cli/src/state_paths.rs` — NEW: single source of truth for runtime state paths (user_state_dir, audit_root, rollback_root, sessions_dir, migration, legacy fallbacks)
- `crates/nono-cli/src/config/mod.rs` — D-01 delegation: user_state_dir() now calls state_paths::user_state_dir().ok(); XDG doc comment adopted; stale test replaced
- `crates/nono-cli/src/audit_session.rs` — Callsite migrated: audit_root() delegates to state_paths::audit_root(); NONO_TEST_HOME tests removed
- `crates/nono-cli/src/rollback_session.rs` — Callsite migrated: rollback_root() delegates to state_paths::rollback_root(); platform split removed
- `crates/nono-cli/src/session.rs` — sessions_dir() delegates to state_paths::sessions_dir(); platform tests replaced
- `crates/nono-cli/src/protected_paths.rs` — from_defaults() applies resolve_path() to XDG state roots; paths_equal() annotated #[allow(dead_code)]
- `crates/nono-cli/src/profile/mod.rs` — user_profile_draft_dir() wired; expand_vars XDG tests gated #[cfg(unix)]
- `crates/nono-cli/src/profile_save_runtime.rs` — Dead functions removed (suggested_run_profile_name, profile_name_from_command)
- `crates/nono-cli/src/wiring.rs` — Upstream wiring module comment update applied; WiringRecord-dependent functions kept in DU hold
- `tests/integration/test_bypass_protection.sh` — XDG-aware ${XDG_CONFIG_HOME:-$HOME/.config} form adopted from upstream

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed ProtectedRoots::from_defaults() UNC path mismatch**
- **Found during:** Task 3 (make ci gate)
- **Issue:** `capability_ext::test_from_args_rejects_protected_state_subtree` failed because `protected_state_roots()` uses `try_canonicalize` (returns `\\?\C:\...` UNC paths) while `validate_requested_path_against_protected_roots` uses `resolve_path` (strips UNC prefix). Path component comparison then failed (`\\?\C:` != `C:`).
- **Fix:** Applied `resolve_path()` normalization to each root in `ProtectedRoots::from_defaults()` before storing, matching the comparison side.
- **Files modified:** `crates/nono-cli/src/protected_paths.rs`
- **Commit:** `de553185`

**2. [Rule 1 - Bug] Replaced stale NONO_TEST_HOME tests with platform-appropriate tests**
- **Found during:** Task 3 (make ci gate)
- **Issue:** Three tests (`user_state_dir_honors_nono_test_home`, `sessions_dir_returns_path_under_nono_test_home`, `sessions_dir_maps_envvar_validation_to_config_parse`) tested the old NONO_TEST_HOME behavior that no longer exists after D-01 delegation and state_paths restructuring.
- **Fix:** Replaced with `user_state_dir_uses_localappdata_on_windows` (#[cfg(target_os="windows")]), `sessions_dir_uses_localappdata_on_windows` (#[cfg(target_os="windows")]), and `sessions_dir_uses_xdg_state_home` (#[cfg(not(target_os="windows"))]).
- **Files modified:** `crates/nono-cli/src/config/mod.rs`, `crates/nono-cli/src/session.rs`
- **Commit:** `de553185`

**3. [Rule 2 - Missing functionality] Removed dead upstream functions from profile_save_runtime.rs**
- **Found during:** Task 2 (cherry-pick 8e0d94f9)
- **Issue:** Upstream added `suggested_run_profile_name()` and `profile_name_from_command()` in `profile_save_runtime.rs`, but they are not callable in the fork without changing the `offer_save_run_profile` signature. Per CLAUDE.md no-dead-code policy, these would produce lint failures.
- **Fix:** Removed both functions from the fork's version. Will be absorbed when the calling function's signature is updated in a future phase.
- **Files modified:** `crates/nono-cli/src/profile_save_runtime.rs`
- **Commit:** `74c5ac23`

**4. [Rule 1 - Bug] Fixed needless_return clippy lint in state_paths D-02 Windows arm**
- **Found during:** Task 3 (make ci gate)
- **Issue:** Clippy flagged `return Ok(PathBuf::from(local_app_data).join("nono"))` inside the `#[cfg(target_os = "windows")]` cfg block. The `return` keyword is needless when the cfg block makes it the last expression.
- **Fix:** Removed the `return` keyword; expression form `Ok(PathBuf::from(local_app_data).join("nono"))` used.
- **Files modified:** `crates/nono-cli/src/state_paths.rs`
- **Commit:** `de553185`

### PARTIAL→CI Deferrals

Six cfg-gated code paths cannot be verified on Windows host (see `88-PARTIAL-CI.md`):
- `state_paths.rs`: `resolve_xdg_state_base`, `AUDIT_LEDGER_FILENAME`, `maybe_migrate_legacy_audit_ledger` (Linux/macOS only)
- `audit_session.rs`: new `state_paths::audit_root()` delegation (file has `#[cfg(unix)]` blocks)
- `protected_paths.rs`: `resolve_path` normalization (file has platform-specific `#[cfg]` blocks)
- `profile/mod.rs`: XDG config expansion tests gated `#[cfg(unix)]`
- `session.rs`: XDG session dir test gated `#[cfg(not(target_os = "windows"))]`
- `config/mod.rs`: XDG config dir fallback test gated `#[cfg(not(target_os = "windows"))]`

### Out-of-Scope Items

`wiring.rs` `$NONO_CONFIG`/`$NONO_PACKAGES` variable expansion from upstream 8e0d94f9 references `WiringContext`/`expand_vars` types not yet in this fork. The upstream tests for this expansion were not imported. Tracked in `88-PARTIAL-CI.md` forward-compat note.

## Known Stubs

None — all state path functions are fully wired. Legacy fallback (`~/.nono/` read-only fallback) is intentional per FEAT-02 design (T-88-09 accepted).

## Threat Surface Scan

No new network endpoints, auth paths, or trust boundaries introduced. All changes are filesystem path resolution. Threat model per plan:
- T-88-05 (LOCALAPPDATA validation): mitigated — empty/missing env var returns Err, no default fallback
- T-88-06 (XDG_STATE_HOME absolute path validation): mitigated — upstream validate_env_path check preserved in cherry-pick
- T-88-07 (NONO_CONFIG absolute path validation): mitigated — adopted from upstream 8e0d94f9
- T-88-08 (D-03 migration fail-secure): mitigated — `?` propagation verified; no `.ok()` or `.unwrap_or_default()` at migration callsite
- T-88-09 (dual-read legacy fallback): accepted — read-only for existing data; no new writes to ~/.nono/

## Self-Check: PASSED

- `crates/nono-cli/src/state_paths.rs`: FOUND
- Commits 0a09ff41, 74c5ac23, de553185: FOUND (git log verified above)
- `88-PARTIAL-CI.md` Plan 88-02 section: FOUND
- Zero `nono_home_dir.*join.*audit` or `nono_home_dir.*join.*rollback` hits in source
