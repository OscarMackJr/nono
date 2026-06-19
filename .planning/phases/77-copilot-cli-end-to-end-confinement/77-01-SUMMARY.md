---
phase: 77-copilot-cli-end-to-end-confinement
plan: 01
subsystem: sandbox
tags: [windows, appcontainer, dacl, node-esm, copilot-cli, file-read-attributes]

# Dependency graph
requires:
  - phase: 76-self-verifying-harness-foundation
    provides: harness foundation (scripts/verify-dark.ps1 runner contract)
provides:
  - grant_sid_read_attributes_on_path (FILE_READ_ATTRIBUTES DACL primitive in nono library)
  - AppliedAncestorReadAttributesGuard (ownership-gated ancestor-RA RAII guard in nono-cli)
  - copilot-cli profile with node.exe interpreter coverage
affects:
  - 77-02 (admin grant — builds on the RA primitive and guard added here)
  - 77-03 (scripted gate — uses the profile and launch path wired here)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PACKAGE_SID_READ_ATTRS_MASK const-fold idiom (FILE_READ_ATTRIBUTES=0x80, minimal grant per D-09)"
    - "AppliedAncestorReadAttributesGuard: ownership-gated ancestor-walk RAII with Ok(false)=>break D-04 structural split"
    - "PreparedWindowsLaunch drop-order discipline: AFTER _applied_ancestor_traverse, BEFORE _network_enforcement"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono/src/lib.rs
    - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/src/profile/builtin.rs

key-decisions:
  - "PACKAGE_SID_READ_ATTRS_MASK = FILE_READ_ATTRIBUTES (0x80) only — not FILE_GENERIC_READ (D-09 minimal grant)"
  - "Walk target for RA guard is config.resolved_program (target binary), NOT current_dir — load-bearing distinction from traverse guard"
  - "Ok(false) => break arm in RA guard is the D-04 structural split: runtime guard provably never touches C:\\, C:\\Users"
  - "Kept AppliedAncestorReadAttributesGuard separate from AppliedAncestorTraverseGuard (different walk target, different right, different error surface)"
  - "copilot-cli profile supersedes D-06 native-PE assumption: target is standalone @github/copilot Node CLI, requires node.exe coverage"

patterns-established:
  - "New Windows DACL grant primitive follows: mask const (const-fold idiom) + one-liner pub fn calling edit_dacl_for_sid + re-export in lib.rs alphabetical position"
  - "New ancestor RAII guard follows: clone AppliedAncestorTraverseGuard shape, parameterize walk target, swap single grant call, update doc comment"

requirements-completed: [CPLT-01]

# Metrics
duration: 55min
completed: 2026-06-17
---

# Phase 77 Plan 01: CPLT-01 Runtime Fix Summary

**`FILE_READ_ATTRIBUTES` DACL primitive + ownership-gated ancestor-RA RAII guard wired into Windows AppContainer launch, plus `node.exe` interpreter coverage added to copilot-cli profile (D-01/D-02 reconciliation)**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-06-17T12:50:00Z
- **Completed:** 2026-06-17T13:47:29Z
- **Tasks:** 3 (2 TDD, 1 auto)
- **Files modified:** 6

## Accomplishments

- Added `grant_sid_read_attributes_on_path` library primitive (FILE_READ_ATTRIBUTES=0x80, the minimal DACL grant per D-09) with full doc comment and 3 Windows-gated unit tests (round-trip, original-DACL-preserved, bad-SID-fail-closed) — all 3 pass on Windows dev host
- Added `AppliedAncestorReadAttributesGuard` RAII guard in `dacl_guard.rs` that walks the confined target binary's resolution chain, grants RA on user-owned ancestors, stops at the first non-owned ancestor (D-04 structural split), and reverts LIFO on Drop — wired into `PreparedWindowsLaunch` in the correct drop-order position (after `_applied_ancestor_traverse`, before `_network_enforcement`); 2 Windows-gated tests pass
- Updated `copilot-cli` profile in `policy.json` with `"windows_interpreters": ["node.exe"]` and a corrected `meta.description` that supersedes the stale D-06 native-PE assumption; inverted `copilot_cli_profile_is_native_pe` to `copilot_cli_profile_declares_node_interpreter` asserting `== vec!["node.exe"]` — both profile tests pass

## Task Commits

Each task was committed atomically (TDD tasks have RED + GREEN commits):

1. **Task 1 RED: Add failing tests for grant_sid_read_attributes_on_path** - `ca898fc8`
2. **Task 1 GREEN: Add grant_sid_read_attributes_on_path DACL primitive** - `162ee75b`
3. **Task 2 RED: Add failing tests for AppliedAncestorReadAttributesGuard** - `f7aaf4c1`
4. **Task 2 GREEN: Add AppliedAncestorReadAttributesGuard and wire into Windows launch** - `8931f56f`
5. **Task 3: Add node.exe interpreter coverage to copilot-cli profile** - `ce86faf6`

## Files Created/Modified

- `crates/nono/src/sandbox/windows.rs` - Added `PACKAGE_SID_READ_ATTRS_MASK` const (0x80) and `grant_sid_read_attributes_on_path` public fn + 3 unit tests in `dacl_grant_tests`
- `crates/nono/src/lib.rs` - Re-exported `grant_sid_read_attributes_on_path` in alphabetical position between `grant_sid_read_on_path` and `grant_sid_traverse_on_path`
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` - Added `AppliedAncestorReadAttributesGuard` struct + `snapshot_and_apply` + `revert_all` + `Drop` + 2 unit tests; added `grant_sid_read_attributes_on_path` to nono import cluster
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - Added `_applied_ancestor_read_attrs` field to `PreparedWindowsLaunch` and apply site in `prepare_live_windows_launch` (gated on `config.package_sid`, walking `config.resolved_program`)
- `crates/nono-cli/data/policy.json` - Added `"windows_interpreters": ["node.exe"]` to `copilot-cli` profile; replaced stale native-PE `meta.description`
- `crates/nono-cli/src/profile/builtin.rs` - Renamed + inverted `copilot_cli_profile_is_native_pe` to `copilot_cli_profile_declares_node_interpreter`; updated `copilot_cli_profile_present` doc + assert messages

## Decisions Made

- **Mask = FILE_READ_ATTRIBUTES (0x80) only:** No wider bits added. D-09 mandates attribute-read only (not FILE_GENERIC_READ which includes FILE_READ_DATA, FILE_READ_EA, SYNCHRONIZE). The const-fold idiom from `PACKAGE_SID_TRAVERSE_MASK` was followed exactly.
- **Walk target = resolved_program (not cwd):** The traverse guard walks `current_dir` (so the AppContainer child can set its cwd). The RA guard must walk the *binary's resolution chain* — the Node-ESM `realpathSync` call walks ancestor dirs of each module path starting from the binary. Using cwd would miss the binary's ancestor chain.
- **Guards kept separate:** Merging traverse + RA into one guard would require multi-right per-ancestor tracking, complicating revert logic with no robustness benefit. Separate guards for separate semantics is the established pattern.
- **`Ok(false) => break` is verbatim:** This is the D-04 structural proof. The runtime guard never calls grant on non-owned ancestors. The break is intentional and load-bearing — do not change to `continue`.

## Deviations from Plan

None — plan executed exactly as written. All three tasks matched their analogs precisely (mask const, pub fn wrapper, re-export, guard clone, mod.rs wiring, policy.json edit, builtin.rs invert).

## Issues Encountered

**Cross-target clippy: PARTIAL.** `cargo clippy --workspace --target x86_64-unknown-linux-gnu` fails with `error: failed to run custom build command for ring v0.17.14` because `x86_64-linux-gnu-gcc` is not installed on the Windows dev host (C toolchain missing for ring/aws-lc-sys cross-compile). The Linux and macOS targets themselves ARE installed (`rustup target list --installed`). All new Windows-only symbols are properly gated via `#[cfg(target_os = "windows")] pub mod windows` (nono) and `#[cfg(target_os = "windows")] #[path = "exec_strategy_windows/mod.rs"] mod exec_strategy` (nono-cli), so non-Windows builds see no dead code. Deferred to CI per `.planning/templates/cross-target-verify-checklist.md`. CPLT-01 cross-target verification marked **PARTIAL**.

Pre-existing test failures (6 total, none new): `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, 3 `protected_paths::tests::*` (documented in MEMORY `nono_cli_windows_baseline_test_failures`), plus 2 `exec_strategy::launch::*` failures that are also pre-existing environment-specific failures unrelated to this plan's files.

## TDD Gate Compliance

- Task 1 RED gate: `test(77-01): add failing tests for grant_sid_read_attributes_on_path (RED)` — `ca898fc8` — compile error E0432 confirmed (3 unresolved imports)
- Task 1 GREEN gate: `feat(77-01): add grant_sid_read_attributes_on_path DACL primitive (GREEN)` — `162ee75b` — 3/3 tests pass
- Task 2 RED gate: `test(77-01): add failing tests for AppliedAncestorReadAttributesGuard (RED)` — `f7aaf4c1` — compile error E0433 confirmed (2 cannot find type)
- Task 2 GREEN gate: `feat(77-01): add AppliedAncestorReadAttributesGuard and wire into Windows launch (GREEN)` — `8931f56f` — 2/2 tests pass

All 4 TDD gate commits present and in correct RED→GREEN order.

## Threat Surface Scan

All changes are within the existing `#[cfg(target_os = "windows")]` trust boundary. No new network endpoints, no new auth paths, no new schema changes. The new DACL grant (`grant_sid_read_attributes_on_path`) adds to the existing DACL edit surface — addressed by T-77-01 in the plan's threat model (mask is exactly 0x80, no broader bits; ownership gate enforced by `path_is_owned_by_current_user`; fail-closed at every step). No new threat surface found beyond what was already modeled.

## Known Stubs

None — no hardcoded empty values, placeholder text, or unwired data sources introduced.

## Next Phase Readiness

- **77-02 (admin grant):** The `grant_sid_read_attributes_on_path` primitive is ready for `nono setup --grant-ancestors` to consume for the `C:\` / `C:\Users` durable admin grant (well-known SID `S-1-15-2-1`)
- **77-03 (scripted gate):** The `copilot-cli` profile has `node.exe` coverage; the RA guard is live on the AppContainer arm — the gate can now invoke `nono run --profile copilot-cli -- copilot ...` and assert no `STATUS_ACCESS_DENIED` from ancestor lstat

## Self-Check: PASSED

All files exist, all commits found, all key symbols present:
- FOUND: `grant_sid_read_attributes_on_path` in `windows.rs`
- FOUND: `PACKAGE_SID_READ_ATTRS_MASK` in `windows.rs`
- FOUND: re-export in `lib.rs`
- FOUND: `AppliedAncestorReadAttributesGuard` in `dacl_guard.rs`
- FOUND: `_applied_ancestor_read_attrs` in `mod.rs`
- FOUND: `"node.exe"` in `policy.json`
- FOUND: `copilot_cli_profile_declares_node_interpreter` in `builtin.rs`
- OK: stale `copilot_cli_profile_is_native_pe` test removed
- All 5 task commits found: `ca898fc8`, `162ee75b`, `f7aaf4c1`, `8931f56f`, `ce86faf6`
- SUMMARY.md exists at `.planning/phases/77-copilot-cli-end-to-end-confinement/77-01-SUMMARY.md`

---
*Phase: 77-copilot-cli-end-to-end-confinement*
*Completed: 2026-06-17*
