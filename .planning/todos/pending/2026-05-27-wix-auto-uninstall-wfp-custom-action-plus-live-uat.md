---
created: 2026-05-27T12:40:17.628Z
title: WiX auto-uninstall WFP custom action (Fix #2b) + live-UAT
area: tooling
files:
  - scripts/build-windows-msi.ps1
  - crates/nono-cli/src/setup.rs
  - crates/nono-cli/src/bin/nono-wfp-service.rs
  - .planning/debug/resolved/wfp-service-stop-uninstall.md
---

## Problem

Follow-up from debug session `wfp-service-stop-uninstall` (resolved 2026-05-27,
commits `0cbeb3be` Fix #1, `b852826b` Fix #2a). Two carry-forward items:

1. **Fix #2b — WiX auto-uninstall custom action (NOT yet done).** The kernel driver
   service `nono-wfp-driver` is registered post-install via `sc create type=kernel`
   and has no WiX representation, so a clean MSI uninstall leaves it behind. Fix #2a
   added the manual removal command `nono setup --uninstall-wfp`, but there is no
   automatic removal during MSI uninstall. Authoring a WiX custom action to invoke
   that command on uninstall was deliberately deferred: it runs at elevation during
   the uninstall sequence and could NOT be validated from the Claude Code non-elevated
   git-bash/MSYS shell — authoring it blind risks regressing uninstall worse than the
   current state.

2. **Live elevated UAT for Fix #1 + Fix #2a (NOT yet run).** All fixes were verified
   only by `cargo build` + 4 unit tests + production clippy. The behavioral fixes were
   never exercised live because that needs an elevated Windows session the agent shell
   lacks. The installed `C:\Program Files\nono` binary is still the pre-fix MSI, so the
   registered service must be the REBUILT `nono-wfp-service.exe` for the test to mean
   anything (rebuild + reinstall MSI, or re-register the service against the dev-layout
   binary).

## Solution

**Live-UAT (do this first — it gates whether 2b is even needed as described):** from an
elevated PowerShell, with the rebuilt `nono-wfp-service.exe` registered as the service:
- `sc.exe stop nono-wfp-service` → expect success (was the fast-fail STOP error).
- `nono setup --uninstall-wfp` → expect both `nono-wfp-service` + `nono-wfp-driver`
  stopped + deleted.
- MSI uninstall from Apps & Features → expect no leftover user-mode service/binary.
- Confirm fail-secure: after `sc stop`, no `nono` WFP filters/sublayer remain
  (`netsh wfp show filters` or equivalent).

**Fix #2b (WiX custom action), once the above passes:** add a deferred, elevation-time
custom action in `scripts/build-windows-msi.ps1` (and the generated .wxs) that runs on
uninstall (e.g. `Custom Action` of type "run nono.exe setup --uninstall-wfp" sequenced in
`InstallExecuteSequence` on REMOVE, before `RemoveFiles`, scheduled deferred + no-impersonate
for elevation, with a benign return so a failure cannot block uninstall). Validate the full
install → `nono setup --install-wfp-driver` → MSI-uninstall cycle on a real elevated Win11
box and confirm the driver service is gone afterward. Reference the resolved debug session
for the full root-cause + the Fix #2a removal primitive it should call.
