---
phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
plan: 12
subsystem: infra
tags: [windows, appcontainer, wfp, lowbox, security-capabilities, package-sid, broker, sandbox]

# Dependency graph
requires:
  - phase: 62-10
    provides: "broker --session-sid plumbing, create_low_integrity_primary_token_with_sid, AppliedDaclGrantsGuard, single-source session_sid flow (now superseded)"
  - phase: 60
    provides: "grant_sid_write_on_path / AppliedDaclGrantsGuard mechanism (F-60-UAT-04), Low-IL broker arm"
provides:
  - "Per-run AppContainer (lowbox) confined child: nono::derive_app_container_sid + SECURITY_CAPABILITIES spawn — starts cleanly (no 0xC0000142)"
  - "Package-SID-scoped WFP filter via the unchanged ALE_USER_ID SD path (fed the S-1-15-2-* package SID)"
  - "AppliedDaclGrantsGuard retargeted to the package SID for confined AppContainer writes"
  - "Falsified 62-10 WRITE_RESTRICTED broker path removed (no dead code)"
affects: [62-04, windows-uat, wfp-enforcement, lowil-broker]

# Tech tracking
tech-stack:
  added: ["windows-sys Win32_Security_Isolation feature (DeriveAppContainerSidFromAppContainerName)"]
  patterns:
    - "Per-run AppContainer lowbox spawn: name -> DeriveAppContainerSid -> SECURITY_CAPABILITIES (2-attr STARTUPINFOEX) -> CreateProcessW CREATE_SUSPENDED -> label Low-IL -> Resume"
    - "Single-source per-run name -> deterministic package SID on BOTH broker token and WFP request"
    - "OwnedAppContainerSid RAII (FreeSid, distinct from OwnedSid/LocalFree)"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono/Cargo.toml
    - crates/nono/src/lib.rs
    - crates/nono-shell-broker/src/main.rs
    - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/network.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/execution_runtime.rs

key-decisions:
  - "Spawn shape: CreateProcessW + SECURITY_CAPABILITIES (debug D2 shape), NOT CreateProcessAsUserW+token"
  - "Low-IL label: applied EXPLICITLY to the suspended child token (CREATE_SUSPENDED -> label -> Resume) for NO_WRITE_UP defence-in-depth"
  - "session_sid retained as synthetic SID for the legacy WriteRestricted arm; NEW package_sid field carries the package SID to WFP+DACL (the two arms are mutually exclusive)"
  - "WFP service binary unchanged: reuse ALE_USER_ID SD path fed the package SID (ALE_PACKAGE_ID deferred fallback)"

patterns-established:
  - "AppContainer lowbox spawn from a Medium-IL broker via STARTUPINFOEX 2-attr list (HANDLE_LIST + SECURITY_CAPABILITIES)"
  - "Deterministic package-SID single-source: one nono.session.<uuid> name -> same S-1-15-2-* SID both sides"

requirements-completed: [REQ-WFP-01]

# Metrics
duration: ~75min
completed: 2026-06-03
---

# Phase 62 Plan 12: Per-run AppContainer confined-child redesign Summary

**Replaced the falsified 62-10 WRITE_RESTRICTED broker token (which crashed every confined child at startup with 0xC0000142) with a per-run AppContainer (lowbox) spawn: `DeriveAppContainerSidFromAppContainerName` -> `SECURITY_CAPABILITIES{0 caps}` -> `CreateProcessW`, WFP-scoped by the package SID via the unchanged ALE_USER_ID path, DACL guard retargeted to the package SID.**

## Performance

- **Duration:** ~75 min
- **Started:** 2026-06-02 (session)
- **Completed:** 2026-06-03
- **Tasks:** 5
- **Files modified:** 9

## Accomplishments
- The confined `--block-net` child is now spawned as a per-run AppContainer (lowbox) with an EMPTY capability set — strictly more confined than the prior Low-IL token, and it starts cleanly (private `AppContainerNamedObjects` namespace) instead of dying at DLL init.
- The WFP filter is scoped by the per-run **package SID** (`S-1-15-2-*`, a real OS SID class) via the UNCHANGED 62-07/62-08 `ALE_USER_ID` security-descriptor path — the WFP service binary is untouched.
- Single-source invariant preserved: ONE per-run name `nono.session.<uuid>` deterministically derives the SAME package SID on BOTH the broker `SECURITY_CAPABILITIES` and the WFP request (no SID bytes marshaled across argv).
- The falsified 62-10 `create_low_integrity_primary_token_with_sid` path and its tests were removed (no dead code).

## Task Commits

1. **Task 1: AppContainer name + package-SID derivation** - `c1fe2572` (feat)
2. **Task 2: broker AppContainer spawn (SECURITY_CAPABILITIES)** - `cb341165` (feat)
3. **Task 3+4: thread name + package-SID WFP/DACL** - `066d1c74` (feat)
4. **Task 5: retire dead 62-10 WRITE_RESTRICTED code** - `c573586d` (refactor)

**Plan metadata:** (this commit)

## Files Created/Modified
- `crates/nono/src/sandbox/windows.rs` - `derive_app_container_sid` + `OwnedAppContainerSid` (FreeSid RAII), `package_sid_to_string`, `apply_low_il_label_to_token`; removed `create_low_integrity_primary_token_with_sid` + dead imports
- `crates/nono/Cargo.toml` - added `Win32_Security_Isolation` windows-sys feature
- `crates/nono/src/lib.rs` - re-exports updated (added Derive/string/label-token/OwnedAppContainerSid; dropped `_with_sid`)
- `crates/nono-shell-broker/src/main.rs` - `--app-container-name` replaces `--session-sid`; `SECURITY_CAPABILITIES` 2-attr spawn via `CreateProcessW` + `CREATE_SUSPENDED` + Low-IL label + `ResumeThread`; fail-closed
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` - `generate_app_container_name()` + uniqueness test
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - no-PTY broker_args push `--app-container-name`, fail-closed via `ok_or`
- `crates/nono-cli/src/exec_strategy_windows/network.rs` - WFP request fed `config.package_sid`; fixtures updated to package-SID values
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - `ExecConfig.app_container_name` + `package_sid` fields; DACL guard retargeted to `package_sid`
- `crates/nono-cli/src/execution_runtime.rs` - generate one per-run name, derive package SID (fail-closed), thread both

## Decisions Made

### Design decision 1 — spawn shape (resolved per debug D2)
**Chosen: `CreateProcessW` + `SECURITY_CAPABILITIES`** (the broker's Medium-IL token is the base; the lowbox child token is built by the OS at spawn), NOT `CreateProcessAsUserW` with a pre-built token. Rationale: this is the canonical AppContainer launch shape (every UWP/Store app and Chromium renderer), avoids the subtle interaction of mixing AsUserW+token with SECURITY_CAPABILITIES, and is exactly what debug D2 shows. The `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` stdio-gating attribute is preserved — the proc-thread attribute list grew from count 1 to count 2 (HANDLE_LIST at index 0 + SECURITY_CAPABILITIES). `bInheritHandles=1` retained. The legacy/PTY path keeps `CreateProcessAsUserW` + the plain Low-IL primary token unchanged.

### Design decision 2 — Low-IL label on the child (resolved: set explicitly)
**Chosen: apply `nono::apply_low_il_label_to_token` to the child explicitly.** The AppContainer child is spawned `CREATE_SUSPENDED`; the broker opens the child's primary token (`TOKEN_ADJUST_DEFAULT | TOKEN_QUERY`), applies the Low-IL mandatory label (`WinLowLabelSid`, `NO_WRITE_UP`), then `ResumeThread`. This gives explicit NO_WRITE_UP parity / defence-in-depth (D1 step 3) before any user code runs, rather than relying solely on AppContainer's own write-up isolation. FAIL-CLOSED: any failure (`OpenProcessToken` / label / `ResumeThread`) terminates the suspended child and propagates `Err` — an unlabeled or un-resumed child is never left running.

### windows-sys 0.59 symbol/feature resolution
Confirmed against the installed `windows-sys-0.59.0` source — NO new crate dependency and NO `extern "system"` shim needed:
- `DeriveAppContainerSidFromAppContainerName` → `Win32::Security::Isolation` (feature **`Win32_Security_Isolation`**, added to `crates/nono/Cargo.toml`; returns `HRESULT`).
- `SECURITY_CAPABILITIES` + `FreeSid` → `Win32::Security` (feature `Win32_Security`, already enabled in nono + broker). The derived package PSID is freed via **`FreeSid`** (NOT `LocalFree`) — `OwnedAppContainerSid` Drop uses `FreeSid`, distinct from `OwnedSid` (LocalFree).
- `ConvertSidToStringSidW` → `Win32::Security::Authorization` (feature `Win32_Security_Authorization`, already enabled in nono).
- `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` (= 131081) + `CreateProcessW` + `CREATE_SUSPENDED` + `ResumeThread` + `OpenProcessToken` + `TerminateProcess` → `Win32::System::Threading` / `Win32::Security` (already enabled in broker).

The broker reuses `nono::derive_app_container_sid` (one code path); only the `nono` crate gained the new feature flag.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `session_sid` semantics split to preserve the legacy WriteRestricted arm**
- **Found during:** Task 3 (threading the package SID)
- **Issue:** The plan's interface notes suggested feeding the package SID through the existing `config.session_sid` field. But `config.session_sid` is ALSO consumed by the legacy non-broker `WriteRestricted` arm (`create_restricted_token_with_sid`, launch.rs:1273), which requires a synthetic restricting SID — feeding it a package SID would re-introduce the exact `WRITE_RESTRICTED` startup failure for non-broker profiles.
- **Fix:** Added a SEPARATE `ExecConfig.package_sid` field carrying the package SID to the WFP request + DACL guard, while `session_sid` retains the synthetic SID for the (mutually-exclusive) WriteRestricted arm. Single-source is preserved: `package_sid` and the broker's `--app-container-name` both derive from the same per-run name.
- **Files modified:** mod.rs (ExecConfig), execution_runtime.rs, network.rs, dacl_guard apply site
- **Verification:** `cargo build`/`clippy` clean; WriteRestricted arm unchanged; network/dacl tests pass
- **Committed in:** `066d1c74`

---

**Total deviations:** 1 auto-fixed (1 blocking). **Impact:** Necessary for correctness — avoids breaking the non-broker WriteRestricted arm while honoring single-source for the AppContainer arm. No scope creep.

## Issues Encountered

- **Pre-existing, out-of-scope test failures (NOT introduced by 62-12):** 4 `nono-cli` tests fail on the pristine baseline (verified against pre-62-12 code at `e290d6bf`): `protected_paths::tests::{blocks_parent_directory_capability, blocks_child_directory_capability, requested_path_blocks_nonexistent_child_under_protected_root}` (HOME/path-coverage env-dependence) and `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` (a REAL `my-agent.json` exists in the operator's actual `%APPDATA%`). Logged to `deferred-items.md`. 1134 tests pass; the AppContainer/WFP/DACL/broker tests are all green.

## Cross-target clippy deferral

`crates/nono/src/sandbox/windows.rs`, `crates/nono-shell-broker/src/main.rs`, and `crates/nono-cli/src/exec_strategy_windows/*` are `cfg(windows)`-only. Windows-host `cargo clippy -p nono -p nono-shell-broker -p nono-cli -- -D warnings -D clippy::unwrap_used` is CLEAN. Cross-target Linux/macOS clippy is **PARTIAL / deferred-to-CI** per CLAUDE.md (the new code never compiles on Unix; this matches the WFP-62 phase pattern).

## D5 — Four live-UAT unknowns (DEFERRED to the follow-up elevated Win11 UAT)

The AppContainer lowbox spawn + WFP match require a live elevated Win11 host and CANNOT be validated here. Implemented per the plan + debug D1–D4; verified it BUILDS + unit tests pass. The four debug-D5 unknowns remain OPEN for the 62-04 HUMAN-UAT:

1. **(PRIMARY) Does the AppContainer child START under the broker spawn?** — `CreateProcessW` + `SECURITY_CAPABILITIES` + `CREATE_SUSPENDED` from the Medium-IL broker, anonymous-pipe stdio, no ConPTY. Expected YES (canonical lowbox launch). SC1 probe (from `%USERPROFILE%\.claude`): `nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org` MUST run curl (no 0xC0000142).
2. **Does `ALE_USER_ID` SD `D:(A;;CC;;;<packageSid>)` MATCH the AppContainer child's outbound connection?** — if curl starts but is NOT blocked, implement the `ALE_PACKAGE_ID` FWP_SID fallback (debug D1 step 4) in a follow-up. Not pre-built here.
3. **Do confined WRITES to cwd grant paths succeed with the package-SID DACL grant?** — `AppliedDaclGrantsGuard` now grants the package SID; confirm for the `S-1-15-2-*` SID class on the lowbox child.
4. **Does the Low-IL mandatory label survive / need re-applying after the lowbox build?** — we set it explicitly on the suspended child token before resume; confirm it is present on the running child.

## Next Phase Readiness
- Code-complete + build/test/clippy green. Ready for the orchestrator to rebuild `nono.exe` + broker + a version-bumped MSI (v0.57.10+) and run the 62-04 HUMAN-UAT (SC1–SC5) on a live elevated Win11 host to validate the four D5 unknowns.
- 62-11 (uninstall WFP purge) is independent and unaffected.

---
*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Completed: 2026-06-03*

## Self-Check: PASSED
- All 4 task commits present (c1fe2572, cb341165, 066d1c74, c573586d).
- SUMMARY file present.
- Workspace build + Windows-host clippy clean (nono, nono-shell-broker, nono-cli).
- Relevant unit tests green (app_container, broker argv, network, dacl_guard, restricted_token); 4 pre-existing out-of-scope failures documented.
