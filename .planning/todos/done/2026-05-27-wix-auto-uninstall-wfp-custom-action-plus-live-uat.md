---
created: 2026-05-27T12:40:17.628Z
updated: 2026-05-27T14:47:06.082Z
title: Elevated live-UAT for WFP stop/uninstall fixes (Fix #1/#2a/#2b)
area: tooling
files:
  - scripts/build-windows-msi.ps1
  - crates/nono-cli/src/setup.rs
  - crates/nono-cli/src/bin/nono-wfp-service.rs
  - .planning/debug/resolved/wfp-service-stop-uninstall.md
resolves_phase: 53
---

## Problem

Follow-up from debug session `wfp-service-stop-uninstall` (resolved 2026-05-27,
commits `0cbeb3be` Fix #1, `b852826b` Fix #2a, `59808e2d` Fix #2b).

> **STATUS UPDATE 2026-05-27:** Fix #2b (the WiX custom action) is now DRAFTED and
> COMPILE-VALIDATED (commit `59808e2d`; the machine MSI builds clean with WiX 7.0.0).
> The ONLY remaining work is the **elevated live-UAT** (item 2), which also validates
> #2b's runtime behavior. Item 1 below is retained for context.

1. **Fix #2b — WiX auto-uninstall custom action (DRAFTED + compile-validated, commit
   `59808e2d`; runtime behavior UNVALIDATED).** The kernel driver service
   `nono-wfp-driver` is registered post-install via `sc create type=kernel` and has no
   WiX representation, so a clean MSI uninstall leaves it behind. Fix #2a added the
   manual `nono setup --uninstall-wfp`; Fix #2b added a machine-scope, deferred,
   no-impersonate, `Return="ignore"` (fail-open) custom action `CaUninstallWfpServices`
   that runs `nono.exe setup --uninstall-wfp` `Before="RemoveFiles"`, conditioned
   `(REMOVE="ALL") AND NOT UPGRADINGPRODUCTCODE`. NOT yet validated at runtime: that the
   deferred type-34 CA actually launches nono.exe (cwd=INSTALLFOLDER, relative exe), that
   the condition fires on uninstall-not-upgrade, and that fail-open holds. Needs the same
   elevated live-uninstall test as item 2.

2. **Live elevated UAT for Fix #1 + Fix #2a + Fix #2b (NOT yet run).** All fixes were verified
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

**Fix #2b (WiX custom action) — ALREADY AUTHORED (commit `59808e2d`), needs runtime
validation:** the `CaUninstallWfpServices` action is in `scripts/build-windows-msi.ps1`
(machine scope). Validate the full cycle on a real elevated Win11 box: build + install the
machine MSI → `nono setup --install-wfp-driver` → uninstall from Apps & Features → confirm
BOTH `nono-wfp-service` AND `nono-wfp-driver` are gone and no install dir/binary remains. Also
confirm a MAJOR-UPGRADE install (not uninstall) does NOT tear down the services (the
`NOT UPGRADINGPRODUCTCODE` condition). If the deferred type-34 CA fails to launch nono.exe
(deferred property/cwd resolution is the known risk), switch to the immediate-CA +
CustomActionData pattern to pass the resolved `[INSTALLFOLDER]` path. Fail-open
(`Return="ignore"`) means a mistake degrades to "driver left behind" (today's behavior), never
a broken uninstall — verify that too.

## Disposition (Phase 53) — CLOSED
**Result:** PASS
**Date:** 2026-05-29
**Closed by:** Phase 53 UAT-C (Plan 53-04 Task 3)
**Requirement:** REQ-DRN-01 — elevated WFP stop/uninstall leaves nothing behind
**Decision:** D-53-07 — Todo 1 IS the REQ-DRN-01 HUMAN-UAT work
**Outcome:** Fix #1 (SERVICE_CONTROL_STOP), Fix #2a (nono setup --uninstall-wfp),
and Fix #2b (CaUninstallWfpServices WiX CA) all confirmed at runtime on Windows 11
build 26200 against the signed v0.57.4 machine MSI. Live evidence:
- `sc.exe stop nono-wfp-service` → accepted (STOPPABLE), no fast-fail (Fix #1 ✓).
- `nono setup --uninstall-wfp` → both `nono-wfp-service` + `nono-wfp-driver` removed;
  `sc query` on each → `1060 does not exist` (Fix #2a ✓).
- `netsh wfp show filters | sls nono` → empty, no residual filters.
- `msiexec /x …machine.msi` → no service, no driver, `Test-Path "C:\Program Files\nono"`
  = `False`, no filters (Fix #2b WiX CA ✓).
- Upgrade guard: double `msiexec /i` → service still `RUNNING`, not torn down
  (`NOT UPGRADINGPRODUCTCODE` ✓).
No CA-fallback (immediate-CA + CustomActionData) was needed.

**Note:** This UAT ran against a build whose MSI payload binaries were Authenticode
**unsigned** (signing-order defect found in the same checkpoint — see 53-04-SUMMARY.md
and REQ-RLS-01). REQ-DRN-01 is independent of that defect: WFP stop/uninstall behavior
does not depend on the binary's Authenticode signature, and all WFP fixes passed.
