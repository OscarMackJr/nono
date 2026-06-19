# TODO: machine MSI must handle the VC++ x64 runtime prerequisite

**Captured:** 2026-06-11 (Phase 66 WR-02 EDR UAT, clean-host install)
**Severity:** medium — public release fails to install on a clean Windows host
**Source:** `.planning/phases/66-wr-02-edr-human-uat/66-HUMAN-UAT.md` (findings)
**Resolves phase:** 80 — Clean-Host Install UAT (v2.13; INST-01) — origin Phase 67 (v2.11; DIST-01/DIST-02), UAT carried forward
**Resolves phase (v3.0):** 82 — Fleet Deployment Infrastructure (DEPLOY-01 silent install / correct exit codes; DEPLOY-06 non-fatal atomic service install)

## Problem
On a clean Windows 11 host (no VC++ runtime), the v0.62.2 **machine** MSI fails `1603`:
both `nono.exe` (`0xC0000135` STATUS_DLL_NOT_FOUND) and `nono-wfp-service.exe` can't load,
so the MSI's `ServiceControl` start of `nono-wfp-service` times out (SCM event 7009) and the
**entire install rolls back**. Installing `vc_redist.x64.exe` first resolves both.

## Fix options
- Bundle the VC++ x64 redistributable merge module / launch a chained redist install in the MSI, OR
- Build the Rust binaries with the **static CRT** (`+crt-static`) so no redist is needed, OR
- At minimum: declare the prerequisite + make the `nono-wfp-service` start **non-fatal** to the
  install (a service-start failure should not roll back the whole product).

## Acceptance
v0.62.2-equivalent machine MSI installs cleanly on a fresh Win11 host with no manual redist step.
