---
phase: 71
plan: "03"
subsystem: windows-sandbox-library
tags: [windows, sandbox, coverage-gate, interpreter-coverage, write-owner, fail-secure, library]
dependency_graph:
  requires: []
  provides:
    - validate_launch_paths(interpreters: &[PathBuf]) — single coverage chokepoint now checks interpreter set
    - path_has_write_owner(path) -> Result<bool> — R-B3 WRITE_OWNER predicate for pre-launch gate
  affects:
    - crates/nono-cli/src/exec_strategy_windows/mod.rs (call site updated; Plan 04 threads real interpreters)
tech_stack:
  added: []
  patterns:
    - "TDD RED/GREEN: failing tests committed before implementation; GREEN in single feat commit"
    - "Interpreter coverage loop: normalize_candidate_path + covers_path (component-wise, not string starts_with)"
    - "path_has_write_owner delegates to path_is_owned_by_current_user (260522-wn0 v2 proxy — GetEffectiveRightsFromAclW dropped due to UAC-filtered-token false positives)"
    - "try_set_mandatory_label refactored to call path_has_write_owner: single source of truth"
key_files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono/src/sandbox/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
decisions:
  - "path_has_write_owner delegates to path_is_owned_by_current_user per debug 260522-wn0 v2: GetEffectiveRightsFromAclW was explicitly removed because it walks the full (unfiltered) token groups, yielding false positives for local admins under the UAC-filtered token that SetNamedSecurityInfoW(LABEL_*) actually runs under. Owner-SID-equality is the correct proxy on standard user accounts."
  - "Interpreter coverage uses normalize_candidate_path + covers_path (component-wise case-insensitive path comparison), not string starts_with — guards CLAUDE.md footgun #1"
  - "Empty interpreters slice reproduces pre-extension behavior (backward-compatible call sites pass &[] until Plan 04 threads the resolved set)"
metrics:
  duration: "367 seconds (~6 minutes)"
  completed_date: "2026-06-14"
  tasks: 2
  files_modified: 3
---

# Phase 71 Plan 03: Library Fail-Secure Coverage Gate + R-B3 WRITE_OWNER Predicate Summary

**One-liner:** Extended `validate_launch_paths` to coverage-check declared interpreters with named D-07 diagnostics, and extracted `path_has_write_owner` as the R-B3 WRITE_OWNER predicate with a `#[must_use]` fail-closed Result.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| RED | Failing tests for Task 1 + Task 2 | 7f4b895e | windows.rs (+101 lines of tests) |
| GREEN | Implement both primitives + update call sites | 6dc64b3c | windows.rs, mod.rs, exec_strategy_windows/mod.rs |

## What Was Built

### Task 1: validate_launch_paths interpreter coverage (D-07/ENG-02)

`validate_launch_paths` now takes `interpreters: &[PathBuf]` as a fourth parameter. After the existing program-coverage check (and before the current_dir check), it loops over each interpreter:
- Normalizes via `normalize_candidate_path` (canon + `normalize_windows_path` to strip `\\?\` prefix)
- Checks `policy.covers_path(&normalized, AccessMode::Read)` — component-wise, case-insensitive (`windows_paths_start_with_case_insensitive`), NEVER string `starts_with`
- On miss: returns `NonoError::UnsupportedPlatform` naming (a) the uncovered interpreter path, (b) the wrapper program (`program.display()`), and (c) the concrete `--allow <interpreter parent dir>` fix; states nono will not launch a partially-confined engine

The existing program-coverage `Err` was also enriched with the `--allow` fix hint (D-07 requires the fix hint on both coverage failures).

`Sandbox::validate_windows_launch_paths` wrapper (mod.rs) updated to thread the new parameter. The one existing CLI call site in `exec_strategy_windows/mod.rs:331` now passes `&[]` (Plan 04 will thread the real resolved interpreter set).

Three existing tests updated to pass `&[]` for the new parameter (backward-compatible behavior preserved).

### Task 2: path_has_write_owner helper (D-08/ENG-02/R-B3)

`pub fn path_has_write_owner(path: &Path) -> Result<bool>` added with `#[must_use]`. This is the R-B3 pre-launch predicate: Plan 04's CLI pre-gate will call it to refuse a non-relabelable workspace before spawn (named diagnostic instead of opaque `SetNamedSecurityInfoW(LABEL_*)` failure).

Implementation delegates to `path_is_owned_by_current_user` (the established proxy from debug 260522-wn0 v2). This is fail-closed: `Err` propagates, never silently `Ok(true)`.

`try_set_mandatory_label` refactored to call `path_has_write_owner` (instead of the direct `path_is_owned_by_current_user` call) so the WRITE_OWNER decision has a single source of truth for both apply-time and pre-launch use.

## Verification Results

```
cargo test -p nono -- validate_launch_paths path_has_write_owner path_is_owned_by_current_user
test result: ok. 9 passed; 0 failed; 0 ignored
```

Tests:
- `validate_launch_paths_refuses_uncovered_interpreter` — Err names interpreter, wrapper, `--allow`, "partially-confined"
- `validate_launch_paths_accepts_covered_program_and_interpreter` — Ok when both covered
- `validate_launch_paths_empty_interpreter_slice_is_unchanged` — empty slice = unchanged behavior
- `path_has_write_owner_returns_true_for_userprofile_tempdir` — Ok(true) for user-created tempdir
- `path_is_owned_by_current_user_returns_true_for_tempfile` — pre-existing, still passes
- `path_is_owned_by_current_user_returns_false_for_system_windows_dir` — pre-existing, still passes
- 3 pre-existing `validate_launch_paths` tests updated and passing

```
cargo build -p nono   # clean
cargo build -p nono-cli  # clean (call site update verified)
cargo clippy -p nono -- -D warnings -D clippy::unwrap_used  # clean
```

## Threat Mitigations Applied

| Threat | Status |
|--------|--------|
| T-71-05: covered wrapper spawns uncovered interpreter (partial confinement) | MITIGATED — interpreter loop in validate_launch_paths; test green |
| T-71-06: non-relabelable workspace → opaque failure → operator disables confinement | MITIGATED — path_has_write_owner available for Plan 04 pre-gate |
| T-71-07: string starts_with path comparison | MITIGATED — covers_path (component-wise) used for all interpreter checks |
| T-71-SC: no package installs | N/A — library-only edits |

## Deviations from Plan

### Decision: path_has_write_owner implementation via delegation

**Rule 2 — Correctness decision (preserved from debug 260522-wn0):**

- **Found during:** Task 2 implementation research
- **Issue:** The plan offers `GetEffectiveRightsFromAclW` as the implementation approach, but this API was explicitly removed from the codebase in debug 260522-wn0 because it walks the **full** (unfiltered) token's group memberships, yielding false positives for local admins under the UAC-filtered token that `SetNamedSecurityInfoW(LABEL_*)` actually runs under.
- **Fix:** `path_has_write_owner` delegates to `path_is_owned_by_current_user` — the same proxy the apply-time `try_set_mandatory_label` branch already uses. The plan explicitly permits this: "reuse the established RAII guard + fail-closed pattern from `path_is_owned_by_current_user`" and "(b) gate on `path_is_owned_by_current_user` (weaker but exists)".
- **Impact:** On standard (non-admin) user accounts, WRITE_OWNER is present iff the user is the NTFS owner, so the proxy is correct for the 260522-wn0-diagnosed R-B3 failure mode. Local admin edge case: identical to existing behavior in `try_set_mandatory_label`.
- **Files modified:** windows.rs only

## Known Stubs

- `exec_strategy_windows/mod.rs:331` — interpreter slice passed as `&[]` (Plan 04 wires the real resolved interpreter set from the profile's `windows_interpreters` field).

## Threat Flags

None — changes are Windows-only (`#[cfg(target_os = "windows")]`) library primitives. No new network endpoints, auth paths, or cross-platform trust boundaries introduced.

## Self-Check: PASSED

- `validate_launch_paths` with 4-arg signature: present in windows.rs line 2108
- `path_has_write_owner` with `#[must_use]`: present in windows.rs line 1222
- `try_set_mandatory_label` calls `path_has_write_owner`: verified at line 1154
- `Sandbox::validate_windows_launch_paths` wrapper updated: present in mod.rs line 842
- CLI call site updated: verified in exec_strategy_windows/mod.rs line 331
- 9 tests green
- Commits: 7f4b895e (RED tests), 6dc64b3c (GREEN implementation)
