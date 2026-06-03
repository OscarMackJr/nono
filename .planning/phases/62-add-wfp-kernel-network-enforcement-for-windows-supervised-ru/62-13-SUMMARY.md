---
phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
plan: 13
subsystem: infra
tags: [windows, appcontainer, wfp, lowbox, dacl, security-capabilities, file-traverse, network-enforcement]

# Dependency graph
requires:
  - phase: 62-12
    provides: derive_app_container_sid / package_sid_to_string / OwnedAppContainerSid + the AppContainer (lowbox) broker spawn (SECURITY_CAPABILITIES) + ExecConfig.package_sid + AppliedDaclGrantsGuard retargeted to the package SID
  - phase: 62 (spike)
    provides: crates/nono-cli/examples/spike_wfp_appcontainer.rs — the elevated proof that a registered AppContainer profile makes the lowbox child START and WFP kernel-blocks it via ALE_USER_ID(packageSid)
provides:
  - nono::create_app_container_profile + AppContainerProfile RAII guard (register CreateAppContainerProfile, Drop = DeleteAppContainerProfile, tolerate ALREADY_EXISTS, FreeSid)
  - nono::grant_sid_traverse_on_path (FILE_TRAVERSE|FILE_LIST_DIRECTORY = 0x21 traverse-only grant) via a parameterized edit_dacl_for_sid mask
  - broker registers the per-run AppContainer profile (fail-closed) BEFORE the SECURITY_CAPABILITIES spawn and holds the guard until the child exits
  - nono-cli AppliedAncestorTraverseGuard — grants the package SID traverse on user-owned cwd ancestors, reverted on Drop
affects: [62-04, windows-appcontainer-wfp-validated, claude.exe-read-grant-model]

# Tech tracking
tech-stack:
  added: []  # no new deps — windows-sys 0.59 Win32_Security_Isolation already exposes Create/DeleteAppContainerProfile
  patterns:
    - "AppContainer PROFILE registration (not just SID derivation) is the precondition for an AppContainer CreateProcessW spawn"
    - "Per-mask DACL grant via a parameterized edit_dacl_for_sid (writable 0x1301BF vs traverse-only 0x21)"
    - "Cwd-ancestor RAII traverse guard mirroring AppliedDaclGrantsGuard (snapshot/apply/revert/Drop, owner-gated, fail-closed)"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono/src/lib.rs
    - crates/nono-shell-broker/src/main.rs
    - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs

key-decisions:
  - "Tolerate BOTH already-exists HRESULTs: 0x800700B7 (ERROR_ALREADY_EXISTS, the LIVE Win11 build-26200 return) AND 0x80070050 (ERROR_FILE_EXISTS, the spike's value). The spike's single 0x80070050 constant is INSUFFICIENT on this host."
  - "Keep ALE_USER_ID(packageSid) WFP scoping unchanged — the spike proved it blocks AppContainer connections; do NOT switch to ALE_PACKAGE_ID."
  - "Ancestor traverse mask is narrow (FILE_TRAVERSE | FILE_LIST_DIRECTORY only, NOT FILE_GENERIC_READ, no write/delete) and never inheritable — pass-through, not read/enumerate (threat T-62-33 accept-minimal)."
  - "Profile guard held via a _-prefixed binding (NOT bare _) so it lives to the end of broker run() — past WaitForSingleObject — so DeleteAppContainerProfile runs on child exit."

patterns-established:
  - "Pattern 1: AppContainer spawn requires a REGISTERED profile; a derived-only package SID yields CreateProcessW ERROR_FILE_NOT_FOUND."
  - "Pattern 2: Parameterized edit_dacl_for_sid mask lets one DACL core serve both the writable (0x1301BF) and traverse-only (0x21) grant shapes."

requirements-completed: [REQ-WFP-01]

# Metrics
duration: 24min
completed: 2026-06-03
---

# Phase 62 Plan 13: AppContainer Profile-Registration Spawn Fix Summary

**Per-run AppContainer PROFILE is now REGISTERED (CreateAppContainerProfile RAII) before the broker's SECURITY_CAPABILITIES spawn — the spike-validated fix for the 62-12 ERROR_FILE_NOT_FOUND lowbox-spawn bug — plus a traverse-only package-SID grant on user-owned cwd ancestors so the confined child can reach a profile-deep cwd; WFP ALE_USER_ID(packageSid) scoping kept unchanged.**

## Performance

- **Duration:** ~24 min
- **Started:** 2026-06-03T03:22:37Z
- **Completed:** 2026-06-03T03:46:47Z
- **Tasks:** 4
- **Files modified:** 5

## Accomplishments

- Ported the spike-proven `CreateAppContainerProfile` / `DeleteAppContainerProfile` sequence into the nono library as `create_app_container_profile` + the `AppContainerProfile` RAII guard (tolerates ALREADY_EXISTS, frees the fresh-create SID via `FreeSid`, Drop deletes the profile best-effort, fail-closed on any other HRESULT).
- Added `grant_sid_traverse_on_path` (traverse-only `FILE_TRAVERSE | FILE_LIST_DIRECTORY` = 0x21) by parameterizing the shared `edit_dacl_for_sid`/`build_explicit_access` to take an explicit access mask (was a hardcoded `SESSION_SID_WRITE_MASK`).
- Broker now REGISTERS the per-run AppContainer profile (fail-closed `?`) before deriving the SID / building `SECURITY_CAPABILITIES`, and holds the guard until the child exits — THE spawn fix the 62-12 Derive-only path was missing.
- nono-cli `AppliedAncestorTraverseGuard` grants the package SID traverse on user-owned cwd ancestors (stops at the first non-owned ancestor), reverted on Drop, fail-closed.
- windows-sys 0.59 `Win32_Security_Isolation` already exposes `Create/DeleteAppContainerProfile` — no new dependency and no `userenv` extern shim needed (confirmed reachable in the registry source).

## Task Commits

Each task was committed atomically (all with DCO sign-off `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`):

1. **Task 1: nono lib — AppContainer profile RAII guard + ancestor FILE_TRAVERSE grant** - `a250701f` (feat)
2. **Task 2: Broker — register the AppContainer profile before the spawn** - `e3a22895` (feat)
3. **Task 3: nono-cli — grant the package SID FILE_TRAVERSE on user-owned cwd ancestors** - `b5550717` (feat)
4. **Task 4: Workspace build + tests + clippy** - `66bd0b6b` (chore — deferred-items re-confirmation)

## Files Created/Modified

- `crates/nono/src/sandbox/windows.rs` - `AppContainerProfile` RAII guard + `create_app_container_profile` (the registration FFI) + `grant_sid_traverse_on_path` + `PACKAGE_SID_TRAVERSE_MASK`; parameterized `edit_dacl_for_sid`/`build_explicit_access` with an explicit access mask; 4 new tests (profile round-trip, profile empty-name fail-closed, traverse grant/revoke round-trip, traverse invalid-SID fail-closed).
- `crates/nono/src/lib.rs` - export `create_app_container_profile`, `grant_sid_traverse_on_path`, `AppContainerProfile`.
- `crates/nono-shell-broker/src/main.rs` - register the per-run profile (fail-closed) before deriving the SID; hold the guard until the child exits.
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` - `AppliedAncestorTraverseGuard` (snapshot/apply/revert/Drop, owner-gated, fail-closed) + 2 tests.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - apply the ancestor-traverse guard on the AppContainer arm + new `PreparedWindowsLaunch._applied_ancestor_traverse` field (correct reverse-drop order).

## Decisions Made

- **Tolerate two already-exists HRESULTs.** The reference spike tolerated only `0x80070050` (`ERROR_FILE_EXISTS`). On the live Win11 build-26200 host, `CreateAppContainerProfile` on an existing profile returns `0x800700B7` (`ERROR_ALREADY_EXISTS`) instead — discovered when the round-trip test's second registration failed. The lib now tolerates BOTH (fail-safe: an already-registered profile with a deterministic SID is exactly the reuse case); any other HRESULT still propagates fail-closed. This is a real correction over the spike (deviation Rule 1, below).
- **Keep ALE_USER_ID(packageSid) unchanged** — the spike proved it blocks an AppContainer connection (curl "Could not resolve host"); no `ALE_PACKAGE_ID` switch, the WFP service binary was untouched.
- **Narrow ancestor mask, never inheritable** — ancestors get traverse-only pass-through (0x21), not read/list/write, and `NO_INHERITANCE` so the grant does not propagate onto the ancestor's other children.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Tolerated the live ERROR_ALREADY_EXISTS HRESULT (0x800700B7), not only the spike's 0x80070050**
- **Found during:** Task 1 (profile round-trip test)
- **Issue:** The plan/spike specified tolerating `HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS)` as `0x80070050`. That constant is actually `HRESULT_FROM_WIN32(ERROR_FILE_EXISTS)`. The real `ERROR_ALREADY_EXISTS` is `0xB7` → `0x800700B7`, which is what the live host returns on a re-create. Tolerating only `0x80070050` made the ALREADY_EXISTS path fail-closed incorrectly (the round-trip test caught it: `second create ... failed (HRESULT=0x800700B7)`).
- **Fix:** Tolerate BOTH `0x800700B7` (primary) and `0x80070050` (the spike's value, retained for cross-build safety); every other HRESULT still propagates `NonoError::SandboxInit`.
- **Files modified:** `crates/nono/src/sandbox/windows.rs`
- **Verification:** `create_app_container_profile_round_trips` test passes (create → ALREADY_EXISTS tolerated → Drop deletes → re-create fresh).
- **Committed in:** `a250701f` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug).
**Impact on plan:** The fix was essential for the ALREADY_EXISTS-tolerance correctness contract (a crashed prior run leaving a registered profile must be REUSED, not fail-closed). No scope creep; everything else followed the plan exactly.

## Issues Encountered

- **Test landed in the wrong nested test sub-module.** `windows.rs` has four `#[cfg(test)]` sub-modules with EXPLICIT (not `super::*`) imports. Initial tests for the new symbols failed to compile ("not found in this scope"). Resolved by adding `grant_sid_traverse_on_path` to `dacl_grant_tests`' import and moving the profile tests into `app_container_tests` (with `create_app_container_profile` added to its import). No production impact.

## Threat Flags

None — no new security surface beyond the plan's `<threat_model>` (T-62-30..33). The ancestor traverse grant is the accept-minimal T-62-33 surface (traverse-only, per-run package SID, user-owned ancestors only, reverted on Drop).

## Verification Results

- `cargo build -p nono -p nono-shell-broker -p nono-cli` (Windows host) — exit 0.
- `cargo build -p nono-cli --example spike_wfp_appcontainer` — exit 0; spike UNTOUCHED (`git status --short` clean for that file).
- `cargo clippy -p nono -p nono-shell-broker -p nono-cli -- -D warnings -D clippy::unwrap_used` — clean (no warnings; no dead code).
- Tests: nono lib 730 passed; broker 23 passed; `dacl_guard` 5 passed (incl. 2 new ancestor-traverse); `app_container_tests` 5 passed (incl. 2 new profile tests).
- grep invariants: `CreateAppContainerProfile` defined in the nono lib (`windows.rs`) + called in the broker (`main.rs`); WFP service still uses `FWPM_CONDITION_ALE_USER_ID` (no `ALE_PACKAGE_ID` switch — unchanged).

### Cross-target clippy (PARTIAL — deferred to CI per CLAUDE.md)

All 62-13 changes are `cfg(windows)`-only (`windows.rs` cfg-gated fns, broker, `exec_strategy_windows/*`). Per CLAUDE.md § Coding Standards, the cross-target Unix clippy (`--target x86_64-unknown-linux-gnu` / `--target x86_64-apple-darwin`) cannot run from this Windows host and is deferred to live CI. Windows-host clippy is green.

### Pre-existing out-of-scope test failures (NOT 62-13)

`cargo test -p nono-cli --bin nono` shows 4 failures in `protected_paths::tests::*` (3) and `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` (1). Re-verified FAILING at the pristine pre-62-13 baseline `e290d6bf` (detached worktree, single-threaded — so not a parallelism flake). None of those files were touched by 62-13. Logged + re-confirmed in `deferred-items.md`; out of scope per SCOPE BOUNDARY.

## Pending live UAT (operator, elevated Win11) — the unanswered questions

The live SC1 cannot run from this host (needs elevation + a fresh signed MSI). The orchestrator must rebuild `nono.exe` + broker + a version-bumped signed MSI (v0.57.12) and the operator runs SC1 from `%USERPROFILE%\.claude`:
`nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org`

1. **PRIMARY (the prior failure):** the confined child STARTS — no `ERROR_FILE_NOT_FOUND`, no `0xC0000142` — because the profile is now registered. curl actually runs.
2. **SC1:** curl outbound is BLOCKED (no external IP; "Could not resolve host"/timeout), exit != 0 — confirming the registered-profile path matches the existing ALE_USER_ID(packageSid) filter end-to-end (the spike proved this in isolation; this is the in-nono confirmation).
3. **BYPASS-TRAVERSE QUESTION (still open):** does the lowbox retain bypass-traverse (`SeChangeNotifyPrivilege`)? The ancestor-traverse guard grants only USER-OWNED ancestors; `C:\Users` and `C:\` are SYSTEM/TrustedInstaller-owned and CANNOT be granted (no WRITE_DAC) — so they are SKIPPED. If CreateProcessW STILL fails FILE_NOT_FOUND, the lowbox lacks bypass-traverse AND a non-owned ancestor blocks reaching the cwd → the fix is a profile-accessible-cwd strategy (next follow-up), NOT more ancestor grants. The operator should compare runs WITH vs WITHOUT the ancestor grants to record whether they were even needed (answers the bypass-traverse unknown for the project record).

## Deferred (explicitly NOT in this plan)

- **Full read-grant model for arbitrary tools (e.g. claude.exe).** The AppContainer child is a different principal with zero inherent access to the user profile; reading the tool's exe dir, DLLs, config, node_modules etc. needs package-SID READ grants on every read path. This plan only makes the confined `--block-net` child START + be WFP-blocked end-to-end (the curl SC1 target). The broad read-grant surface remains a follow-up.

## Next Phase Readiness

- Code-complete + build/clippy/test green on the Windows host. Ready for the orchestrator to rebuild + sign a v0.57.12 MSI and run the live elevated 62-04 SC1 UAT.
- The AppContainer spawn fix (profile registration) is the validated unblock for the long-standing F-62-UAT-05 / 0xC0000142 → ERROR_FILE_NOT_FOUND chain.

## Self-Check: PASSED

- Files verified present: `62-13-SUMMARY.md`, `crates/nono/src/sandbox/windows.rs`, `crates/nono-shell-broker/src/main.rs`, `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs`.
- Commits verified in git log: `a250701f`, `e3a22895`, `b5550717`, `66bd0b6b`.

---
*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Completed: 2026-06-03*
