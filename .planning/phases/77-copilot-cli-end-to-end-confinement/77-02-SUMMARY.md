---
phase: 77-copilot-cli-end-to-end-confinement
plan: 02
subsystem: sandbox
tags: [windows, appcontainer, dacl, one-time-admin, setup, all-application-packages, cplt-02]

# Dependency graph
requires:
  - phase: 77-01
    provides: grant_sid_read_attributes_on_path primitive (consumed by grant_ancestors_for_path in setup.rs)
provides:
  - nono setup --grant-ancestors --profile <p> command (generic, D-06)
  - grant_sid_read_attributes_on_path in nono library (PACKAGE_SID_READ_ATTRS_MASK=0x80)
  - grant_ancestors_for_path idempotent helper in setup.rs
  - ALL_APPLICATION_PACKAGES_SID constant (S-1-15-2-1, hardcoded, never per-run)
affects:
  - 77-03 (scripted gate — the admin grant is the precondition; operator runs once before gate)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "GetAce/EqualSid absence check before grant (idempotency guard — no duplicate ACE stacking)"
    - "is_admin_process() gate returning NonoError::Setup elevation error (fail-closed admin gate)"
    - "Short-circuit dispatch in SetupRunner::run() for grant_ancestors (parallel to uninstall_wfp)"
    - "ALL_APPLICATION_PACKAGES_SID hardcoded (never derived from per-run package SID — D-05 RESOLUTION)"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono/src/lib.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/setup.rs

key-decisions:
  - "PACKAGE_SID_READ_ATTRS_MASK = FILE_READ_ATTRIBUTES (0x80) only — not FILE_GENERIC_READ (D-09 minimal)"
  - "Grantee is always the well-known SID S-1-15-2-1 — never derived from app_container_name or per-run UUID (D-05 RESOLUTION)"
  - "grant_sid_read_attributes_on_path added to THIS worktree's nono library (parallel agent; 77-01 in separate worktree not yet merged)"
  - "Idempotency via GetAce/EqualSid loop (not via revoke+re-grant) — preserves MERGE semantics and is non-destructive"
  - "Profile name is validated (non-empty) and logged but does NOT affect the grantee (D-06 engine-agnostic)"

patterns-established:
  - "New idempotent admin setup action follows: is_admin_process() gate + loop over fixed ancestor list + grant_ancestors_for_path()"
  - "grant_ancestors_for_path: GetAce/EqualSid absence check before edit_dacl_for_sid (pattern for any idempotent DACL grant)"

requirements-completed: [CPLT-02]

# Metrics
duration: 55min
completed: 2026-06-17
---

# Phase 77 Plan 02: CPLT-02 Admin Setup Step Summary

**Generic, idempotent, non-destructive `nono setup --grant-ancestors` command that grants the well-known ALL APPLICATION PACKAGES SID (`S-1-15-2-1`) `FILE_READ_ATTRIBUTES` on `C:\` and `C:\Users` — the system ancestors the runtime guard cannot ACL.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-06-17T13:50:00Z
- **Completed:** 2026-06-17T14:40:28Z
- **Tasks:** 2 (Task 1: cli flags + parse tests; Task 2: TDD runtime)
- **Files modified:** 4

## Accomplishments

- Added `PACKAGE_SID_READ_ATTRS_MASK` const (FILE_READ_ATTRIBUTES=0x80, the minimal D-09 grant) and `grant_sid_read_attributes_on_path` public fn to `nono/src/sandbox/windows.rs`, re-exported from `lib.rs` in alphabetical position (also added in this worktree since the 77-01 primitive is in a parallel worktree not yet merged)
- Added generic `--grant-ancestors` and `--profile` flags to `SetupArgs` in `cli.rs` with 3 parse tests (with profile, without profile, profile-only rejected) — all GREEN
- Implemented `grant_ancestors_for_path()` idempotent helper in `setup.rs` using GetAce/EqualSid absence check before calling `grant_sid_read_attributes_on_path`; no duplicate ACE stacking (D-09 non-destructive)
- Added `ALL_APPLICATION_PACKAGES_SID = "S-1-15-2-1"` constant — hardcoded well-known SID, never derived from a per-run package SID (D-05 RESOLUTION)
- Added `grant_ancestors: bool` and `profile: Option<String>` fields to `SetupRunner` (cfg(windows)-gated) and wired through `new()`
- Added short-circuit dispatch in `run()` for `grant_ancestors` flag (parallel to `uninstall_wfp` early-return)
- Implemented `grant_ancestors_for_profile()`: admin gate via `is_admin_process()` (fail-closed NonoError::Setup); profile name validated (non-empty) + logged; loops over `["C:\\", "C:\\Users"]` calling `grant_ancestors_for_path` per ancestor
- Two Windows-gated TDD tests: `grant_ancestors_idempotent` (exactly one ACE after two applies) and `grant_ancestors_non_destructive` (pre-existing ACE unchanged) — both PASS

## Task Commits

Each task was committed atomically (Task 2 has TDD RED + GREEN commits):

1. **Task 1 + Task 2 RED: add failing tests for grant-ancestors runtime and cli flags** - `8210775d`
2. **Task 2 GREEN: implement grant-ancestors runtime (CPLT-02)** - `3090af14`

## Files Created/Modified

- `crates/nono/src/sandbox/windows.rs` - Added `PACKAGE_SID_READ_ATTRS_MASK` const (FILE_READ_ATTRIBUTES=0x80) and `grant_sid_read_attributes_on_path` pub fn with NO_INHERITANCE; doc comment explains D-09 minimal grant rationale and use-cases (CPLT-01 runtime + CPLT-02 admin)
- `crates/nono/src/lib.rs` - Re-exported `grant_sid_read_attributes_on_path` in alphabetical position (between `grant_sid_read_on_path` and `grant_sid_traverse_on_path` cluster)
- `crates/nono-cli/src/cli.rs` - Added `grant_ancestors: bool` and `profile: Option<String>` to `SetupArgs` with doc comments + 3 clap parse tests
- `crates/nono-cli/src/setup.rs` - Added `ALL_APPLICATION_PACKAGES_SID` const, `grant_ancestors_for_path()` idempotent helper, new `SetupRunner` fields, short-circuit dispatch, `grant_ancestors_for_profile()` implementation, and 2 TDD tests

## Decisions Made

- **PACKAGE_SID_READ_ATTRS_MASK = FILE_READ_ATTRIBUTES (0x80):** The minimal grant per D-09 — attribute-read only, NOT FILE_GENERIC_READ (which includes FILE_READ_DATA, FILE_READ_EA, SYNCHRONIZE). Matches the const-fold idiom from PACKAGE_SID_READ_MASK / PACKAGE_SID_TRAVERSE_MASK.
- **Grantee = hardcoded S-1-15-2-1 (D-05 RESOLUTION):** The well-known ALL APPLICATION PACKAGES group SID covers every AppContainer token on the host without re-granting per-run SIDs. The per-run derivation (`derive_app_container_sid`, windows.rs:747) is explicitly NOT used here — this is the locked operator decision.
- **grant_sid_read_attributes_on_path added to this worktree:** The 77-01 primitive exists in a parallel worktree not yet merged to the branch base. Adding it here (same implementation) ensures this plan's code compiles and tests pass; the orchestrator merge will reconcile via the two identical definitions.
- **Idempotency via GetAce/EqualSid loop (not revoke+re-grant):** REVOKE_ACCESS removes ALL ACEs for a trustee regardless of mask — using it for idempotency would remove a pre-existing ACE (violating D-09). The GetAce absence check is the correct pattern.
- **Profile name is validated but does not affect grantee:** The grantee is always the well-known SID regardless of `--profile`. The profile name is logged to give the operator traceability of which engine's setup they're running.

## Deviations from Plan

**[Rule 3 - Blocking] Added grant_sid_read_attributes_on_path to this worktree's nono library**

- **Found during:** Task 2 GREEN — `cargo build --bin nono` failed with E0425 `cannot find function grant_sid_read_attributes_on_path in crate nono`
- **Issue:** The function from 77-01 is in a parallel worktree that has not merged to the milestone branch yet. This worktree was branched from the pre-77-01 base commit.
- **Fix:** Added the identical implementation (`PACKAGE_SID_READ_ATTRS_MASK` + `grant_sid_read_attributes_on_path` + re-export in lib.rs) to this worktree's nono library. The implementation is bit-for-bit identical to the 77-01 version per the 77-PATTERNS.md specification.
- **Files modified:** `crates/nono/src/sandbox/windows.rs`, `crates/nono/src/lib.rs`
- **Commit:** `3090af14`
- **Impact:** When the orchestrator merges both worktrees, git will see the same lines added at the same location — the merge will be clean (no conflict). The SUMMARY.md from 77-01 documents the exact same addition.

## Known Stubs

None — no hardcoded empty values, placeholder text, or unwired data sources introduced.

## TDD Gate Compliance

- Task 2 RED gate: `test(77-02): add failing tests for grant-ancestors runtime and cli flags (RED)` — `8210775d` — compile errors E0425 (ALL_APPLICATION_PACKAGES_SID and grant_ancestors_for_path not found) confirmed
- Task 2 GREEN gate: `feat(77-02): implement grant-ancestors runtime (CPLT-02, GREEN)` — `3090af14` — all 5 new tests pass (3 cli parse + 2 idempotency/non-destructive)

RED/GREEN gate commits present in correct order.

## Cross-Target Clippy: PARTIAL

Same limitation as 77-01: `cargo clippy --workspace --target x86_64-unknown-linux-gnu` fails with `error: failed to run custom build command for ring v0.17.14` because `x86_64-linux-gnu-gcc` is not installed on the Windows dev host. All new Windows-only symbols are `#[cfg(target_os = "windows")]`-gated in both `windows.rs` (module-level) and `setup.rs` (per-field/per-fn). Deferred to CI per `.planning/templates/cross-target-verify-checklist.md`. CPLT-02 cross-target verification marked **PARTIAL**.

## Threat Surface Scan

All changes are within the `#[cfg(target_os = "windows")]` trust boundary. No new network endpoints, no new auth paths, no new schema changes. The `grant_ancestors_for_profile` function edits the DACL of `C:\` and `C:\Users` — this is the highest-severity surface in the phase and is addressed by the threat model (T-77-02 through T-77-02d):

- T-77-02 (EoP/InfoDisc): Mask is FILE_READ_ATTRIBUTES only (0x80); principal is S-1-15-2-1 (well-known group, not user/Everyone); paths are exactly C:\ and C:\Users — asserted by acceptance criteria
- T-77-02b (Tampering): GetAce/EqualSid absence check + MERGE mode; no ACE removal; `grant_ancestors_idempotent` + `grant_ancestors_non_destructive` tests both PASS
- T-77-02c (Spoofing): Hardcoded S-1-15-2-1; no derivation from per-run SID — verified by code review (no call to derive_app_container_sid / package_sid_to_string in grant_ancestors_for_profile)
- T-77-02d (fail-open): is_admin_process() gate + DaclApplyFailed fail-closed on access-denied

No new threat surface beyond the modeled CPLT-02 DACL-edit surface.

## Self-Check: PASSED

- FOUND: `ALL_APPLICATION_PACKAGES_SID` in `setup.rs` (value "S-1-15-2-1")
- FOUND: `grant_ancestors_for_path` in `setup.rs`
- FOUND: `grant_ancestors_for_profile` in `setup.rs`
- FOUND: `PACKAGE_SID_READ_ATTRS_MASK` in `windows.rs`
- FOUND: `grant_sid_read_attributes_on_path` in `windows.rs`
- FOUND: re-export in `lib.rs`
- FOUND: `grant_ancestors: bool` in `SetupArgs` (cli.rs)
- FOUND: `profile: Option<String>` in `SetupArgs` (cli.rs)
- FOUND: `requires = "grant_ancestors"` on profile field (cli.rs)
- FOUND: 3 clap parse tests in cli.rs
- FOUND: `grant_ancestors_idempotent` test in setup.rs
- FOUND: `grant_ancestors_non_destructive` test in setup.rs
- OK: no call to `derive_app_container_sid` in `grant_ancestors_for_profile`
- All tests pass: 13/13 (5 new + 8 pre-existing)
- Both task commits found: `8210775d` (RED), `3090af14` (GREEN)

---
*Phase: 77-copilot-cli-end-to-end-confinement*
*Completed: 2026-06-17*
