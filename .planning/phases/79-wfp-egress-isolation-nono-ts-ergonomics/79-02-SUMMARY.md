# 79-02 SUMMARY — nono-ts confinedRun default-broker-arm ergonomics

**Status:** COMPLETE (checkpoint SC4 PASS)
**Requirement:** TSRG-01
**Repos:** code/tests in sibling C:\Users\OMack\nono-ts (branch `44-broker-ffi-lockstep`); this SUMMARY in C:\Users\OMack\Nono

## What was delivered

1. **D-03 + D-04 wiring** in `nono-ts/src/windows_confined_run.rs` (commit `e84e4d0`, nono-ts repo):
   - `resolve_exe_dir(exe) -> napi::Result<Option<String>>` helper (PATH-walk like find_nono_exe; best-effort `Ok(None)` on failure, never Err).
   - D-03: `let profile = profile.or_else(|| Some("nono-ts-default".to_string()));` is the **first statement** in `confined_run` (line 160), before the validation guard (Pitfall 5). So a no-profile/no-allow caller now reaches the Low-IL broker arm (`windows_low_il_broker:true` via nono-ts-default) instead of the WriteRestricted arm.
   - D-04: auto-covers ONLY the resolved target exe's parent dir (no cwd, no ancestors), appended to a shadowed `allow` before `build_nono_run_args` (unchanged).
   - 3 unit tests added; existing `test_confined_run_requires_profile_or_allow` retargeted to `test_confined_run_default_profile_not_required` (the old InvalidArg path is now unreachable). `cargo test` (nono-ts) → 7 passed.

2. **SC4 napi integration test** `nono-ts/tests/test_confined_run_default.js` + `package.json` `"test"` rewire (commit `aa90938`, nono-ts repo): platform-skip on non-win32; `confinedRun('node.exe', ['-e','process.exit(0)'], undefined, undefined, ws, 30)` with `ws` under `os.homedir()`; assert exitCode===0.

## Live-host checkpoint (Task 3, SC4) — PASS

- `npm run build` (napi build --platform --release) regenerated `nono.win32-x64-msvc.node` with D-03/D-04. (index.d.ts/index.js unchanged — confinedRun signature didn't change.)
- `NONO_EXE` pointed at the fresh `C:\Users\OMack\Nono\target\release\nono.exe` (embeds `nono-ts-default` from Plan 79-01 `507ff683`).
- `npm test` → **PASS**: `confinedRun default-broker-arm path succeeded (exit 0) — exitCode=0`; npm test exit 0. node.exe ran cleanly inside the Low-IL broker arm (no 0xC0000142). SC2/SC3/SC4 satisfied; TSRG-01 met.

## Deviation / host note

The test workspace under `%USERPROFILE%` was pre-owned to the current user via `icacls /setowner` before `npm test`, because this dev session runs elevated and an elevated-created workspace would be BUILTIN\Administrators-owned → broker-arm mandatory-label (R-B3) failure (same elevation footgun seen on the daemon path in Plan 79-01). In a normal non-elevated dev session the workspace is user-owned automatically. See memory `feedback_windows_mandatory_label_write_owner` / `wfp_confined_egress_and_daemon_gate`.

## Cross-target clippy

PARTIAL — `windows_confined_run.rs` is `#![cfg(windows)]`; no non-Windows stub touched. Per `.planning/templates/cross-target-verify-checklist.md`, deferred to CI; nono-ts `cargo test` (Windows host) green.

## Commits

- nono-ts `e84e4d0` — D-03/D-04 + resolve_exe_dir + unit tests (src/windows_confined_run.rs)
- nono-ts `aa90938` — SC4 integration test + package.json test wiring
- (napi `.node` rebuilt locally; index.js/index.d.ts unchanged; incidental package-lock.json churn left uncommitted)
