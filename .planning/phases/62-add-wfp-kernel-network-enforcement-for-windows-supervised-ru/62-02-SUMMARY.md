---
phase: 62
plan: "02"
subsystem: windows-wfp-service
tags: [windows, wfp, msi, wix, pipe-sddl, security]
dependency_graph:
  requires: []
  provides:
    - PIPE_SDDL with IU ACE (non-elevated supervised nono runs can connect to WFP control pipe)
    - nono-machine.wxs ServiceInstall Start=auto (SCM boot-starts service on every boot)
  affects:
    - crates/nono-cli/src/bin/nono-wfp-service.rs
    - dist/windows/nono-machine.wxs
tech_stack:
  added: []
  patterns:
    - TDD (RED → GREEN): unit test for PIPE_SDDL IU ACE constant
    - WiX v4 ServiceInstall Start attribute change
key_files:
  created: []
  modified:
    - crates/nono-cli/src/bin/nono-wfp-service.rs
    - dist/windows/nono-machine.wxs
decisions:
  - "Used IU (Interactive Users) not WD (World/Everyone) for SDDL ACE — avoids granting non-interactive service accounts; nono.exe runs at Medium IL which holds IU enabled"
  - "ServiceConfig (restart policy) deferred — util:ServiceConfig requires WixToolset.Util.wixext build dependency not currently in project; attribute name mismatch (ResetPeriodInDays not ResetPeriodInSeconds) confirmed from XSD"
  - "TDD approach: added failing test first (RED), then updated constant (GREEN), verified with cargo test"
metrics:
  duration: "~25 minutes"
  completed: "2026-06-02"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Phase 62 Plan 02: PIPE_SDDL IU ACE + MSI Boot-Start Summary

**One-liner:** Named-pipe SDDL gains `(A;;GRGW;;;IU)` ACE for non-elevated callers + WiX MSI flips `Start="demand"` to `Start="auto"` so the SCM boots the WFP service without manual intervention.

## What Was Built

### Task 1: Add IU ACE to PIPE_SDDL + regression test (TDD)

Updated `PIPE_SDDL` constant at line 55 of `crates/nono-cli/src/bin/nono-wfp-service.rs` from:
```
"D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;OW)"
```
to:
```
"D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)(A;;GRGW;;;OW)"
```

The new `(A;;GRGW;;;IU)` ACE grants generic read+write to Interactive Users. This covers `nono.exe` running at Medium IL (which holds the Interactive Users SID enabled), enabling non-elevated supervised runs to connect to `\\.\pipe\nono-wfp-control`.

Security invariants preserved:
- `(A;;GA;;;SY)` — SYSTEM retains full access (server side)
- `(A;;GA;;;BA)` — Administrators retain full access
- `(A;;GRGW;;;OW)` — Owner retains read+write
- Low-IL sandbox children cannot bypass WFP enforcement via this pipe because `nono.exe` (the pipe client) itself runs at Medium IL; Low-IL processes do not call `nono run` directly

Added `test_wfp_pipe_sddl_includes_interactive_users` in the existing `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests` block, gated by `#[cfg(target_os = "windows")]`, asserting `PIPE_SDDL.contains("IU") || PIPE_SDDL.contains("BU")`.

### Task 2: Flip ServiceInstall Start=demand to Start=auto in nono-machine.wxs

Changed line 87 of `dist/windows/nono-machine.wxs`:
```xml
Start="demand"  →  Start="auto"
```

This is the primary D-01 fix. The SCM now boot-starts `nono-wfp-service` as SYSTEM on every boot after the machine MSI is installed. Without this change, D-03's runtime auto-start fallback (Plan 62-01) is the only path for a non-elevated supervised run to get the service running, and that path requires elevation on the first start after install.

Unchanged as required:
- `ServiceControl Start="install"` — WiX continues to start the service at MSI install time (immediate availability, no reboot required post-install)
- `ServiceControl Stop="both"` — service is stopped on uninstall/upgrade
- `ServiceControl Remove="uninstall"` — service registration is removed on uninstall (clean-uninstall invariant preserved)
- `nono-user.wxs` — not modified (user MSI has no `ServiceInstall` element)

## Verification Results

| Check | Result |
|-------|--------|
| `cargo test -p nono-cli --bin nono-wfp-service -- pipe_sddl` | PASS (1 passed) |
| PIPE_SDDL == `"D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)(A;;GRGW;;;OW)"` | PASS |
| `Select-String 'Start="auto"' dist/windows/nono-machine.wxs` at line 87 | PASS |
| `Select-String 'Start="install"' dist/windows/nono-machine.wxs` (ServiceControl preserved) | PASS |
| `Stop="both"` and `Remove="uninstall"` preserved | PASS |
| `nono-user.wxs` unmodified | PASS |
| `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | PASS |
| Pre-existing failures in `cargo test -p nono-cli` | 4 (pre-existing, unrelated) |
| Cross-target clippy (Linux/macOS) | PARTIAL — deferred to CI per CLAUDE.md |

## Deviations from Plan

### Auto-fixed Issues

None.

### Intentional Deviations

**1. [Research A1 - Assumption Incorrect] ServiceConfig deferred — util extension not in project scope**

- **Found during:** Task 2 implementation (WiX v4 docs verification step)
- **Issue:** RESEARCH.md marked WiX `ServiceConfig` syntax as ASSUMED (A1). Verification against the official WiX v4 XSD (`src/xsd/util/ServiceConfig.xsd` in wixtoolset/wix repo) revealed:
  1. `ServiceConfig` for failure actions is in the **WiX `util` extension** (`xmlns:util="http://wixtoolset.org/schemas/v4/wxs/util"`), not the base WiX schema
  2. The attribute is `ResetPeriodInDays` (not `ResetPeriodInSeconds` as assumed)
  3. Using `util:ServiceConfig` requires the `WixToolset.Util.wixext` build dependency, which is not currently referenced anywhere in the project (no `xmlns:util` in any `.wxs` file, no `--ext WixToolset.Util.wixext` in build scripts)
  4. The base WiX `ServiceConfig` element (`src/xsd/wix/ServiceConfig.xsd`) is for `MsiServiceConfig` table (MSI 5.0) which has a documented bug per WiX docs and does not support failure action types
- **Decision:** Per plan instructions: "If the WiX docs are unreachable, OMIT the ServiceConfig and note 'ServiceConfig deferred — WiX v4 syntax unverified'". In this case, the docs were reachable but the verified syntax requires build infrastructure changes (new WiX extension dependency) outside Plan 62-02 scope — an architectural change (Rule 4). ServiceConfig omitted.
- **Impact:** T-62-06 (DoS / crash-loop DoS threat) is unmitigated by ServiceConfig; the OS default behavior (no restart on failure) applies. The D-03 fallback in Plan 62-01 provides defense-in-depth for the running-service path.
- **Follow-up:** ServiceConfig with `util:ServiceConfig` can be added in a follow-up plan once `WixToolset.Util.wixext` is wired into the MSI build scripts and CI. The correct syntax is:
  ```xml
  <!-- Add to <Wix> element: xmlns:util="http://wixtoolset.org/schemas/v4/wxs/util" -->
  <!-- Child of ServiceInstall: -->
  <util:ServiceConfig
      FirstFailureActionType="restart"
      SecondFailureActionType="restart"
      ThirdFailureActionType="none"
      ResetPeriodInDays="1"
      RestartServiceDelayInSeconds="5" />
  ```

## Known Stubs

None. Both changes are complete one-line edits. No stubs or placeholder values remain.

## Threat Flags

None. The IU ACE addition is within the plan's threat model (T-62-04, T-62-05). The ServiceConfig deferral leaves T-62-06 unmitigated but is documented above.

## Commits

| Hash | Type | Description |
|------|------|-------------|
| `a3f0fcf7` | test | RED: add failing test for PIPE_SDDL IU ACE |
| `ab0b01d3` | feat | GREEN: add IU ACE to PIPE_SDDL — grant non-elevated supervised runs pipe access |
| `ce3b7954` | feat | flip ServiceInstall Start=demand to Start=auto in nono-machine.wxs (D-01) |

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/bin/nono-wfp-service.rs` (modified)
- FOUND: `dist/windows/nono-machine.wxs` (modified)
- FOUND: `.planning/phases/62-.../62-02-SUMMARY.md` (created)
- FOUND: commit `a3f0fcf7` (RED test)
- FOUND: commit `ab0b01d3` (GREEN PIPE_SDDL update)
- FOUND: commit `ce3b7954` (Task 2 WiX Start=auto)
- PIPE_SDDL verified: `"D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)(A;;GRGW;;;OW)"`
- `Start="auto"` and `Start="install"` both present at correct lines in nono-machine.wxs
