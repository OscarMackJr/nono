---
phase: 63-minifilter-spike-groundwork-macos-divergence-ledger-audit
plan: "02"
subsystem: kernel-driver-spike
tags: [wdk, ewdk, azure-vm, test-signing, compile-proof, altitude-request, bastion]
dependency_graph:
  requires: [drivers/nono-fltmgr-scaffold, drivers/nono-fltmgr/DESIGN.md]
  provides: [63-SC1-vm-state, 63-altitude-request, nono-fltmgr.sys-compile-proof]
  affects: [DRV-03, Phase-64]
tech_stack:
  added: [Azure-Standard-security-VM, EWDK-26H1, EWDK-VS-BuildTools-18.3.0, az-vm-run-command, Azure-Bastion]
  patterns: [headless-run-command-capture, EWDK-ISO-mount-build, compile-proof-defect-surfacing]
key_files:
  created:
    - .planning/phases/63-minifilter-spike-groundwork-macos-divergence-ledger-audit/63-SC1-vm-state.md
    - .planning/phases/63-minifilter-spike-groundwork-macos-divergence-ledger-audit/63-altitude-request.md
  modified:
    - drivers/nono-fltmgr/DESIGN.md
    - drivers/nono-fltmgr/nono-fltmgr.vcxproj
decisions:
  - "Task 2 (altitude email) executed BEFORE Task 1 (VM compile) to start the ~30-business-day Microsoft altitude clock early; email sent 2026-06-07 to fsfcomm@microsoft.com, recorded pending"
  - "VM = Standard_D4s_v4 (DSv5/DASv5 had zero quota in eastus); --security-type Standard required registering Microsoft.Compute/UseStandardSecurityType (Gen2 defaults to Trusted Launch); Owner self-serve, no Azure admin"
  - "SC1 captured headless via az vm run-command CLI equivalents (corporate egress blocks RDP/3389); SC2 built in Azure Bastion (443) after the legacy run-command channel wedged on an interactive LaunchBuildEnv hang"
  - "Toolchain = EWDK 26H1 ISO (VS BuildTools 18.3.0) mounted on the VM, not an installed VS+WDK"
  - "INF stamping + test-signing dropped from the SC2 compile-proof build (Phase 64 packaging concerns); SignMode=Off; .sys is throwaway (5120 bytes), not committed (Pitfall 6)"
metrics:
  tasks_completed: 2
  tasks_total: 2
  files_created: 2
  files_modified: 2
  completed_date: "2026-06-08"
---

# Phase 63 Plan 02: Azure Test-Signing VM + Scaffold Compile Proof + Altitude Request

**One-liner:** Stood up a disposable Azure Standard-security-type VM (Secure Boot off, HVCI off, TESTSIGNING on), proved the Plan 63-01 `drivers/nono-fltmgr/` scaffold compiles to `nono-fltmgr.sys` on the EWDK (SC2 / DRV-03 partial) after fixing five real scaffold defects the compile-proof surfaced, and kicked off the Microsoft altitude request (sent 2026-06-07).

## What Was Built

**Task 1 — Azure VM + SC1 + SC2 (human-verify checkpoint):**
- Provisioned `nono-fltmgr-vm` (eastus, `Standard_D4s_v4`, Win11 24H2 Gen2, `--security-type Standard --enable-secure-boot false`). Required registering the `Microsoft.Compute/UseStandardSecurityType` feature flag (Gen2 defaults to Trusted Launch).
- **SC1** (headless via `az vm run-command`): `bcdedit /set testsigning on` succeeded with no Secure-Boot-policy error (Pitfall A cleared); captured `testsigning Yes`, `Secure Boot: Off`, `HVCI/VBS: 0 (Off)`, Win11 Pro build 26100.
- **SC2** (Azure Bastion desktop + EWDK 26H1 ISO): `msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64` → `Build succeeded. 0 Warning(s) 0 Error(s)`, producing `x64\Release\nono-fltmgr.sys` (5,120 bytes). Evidence in `63-SC1-vm-state.md`.

**Task 2 — Microsoft altitude request (human-action checkpoint):**
- Verified the channel is still `fsfcomm@microsoft.com` (subject "Filter altitude request"; no web form).
- Drafted the request (maintainer Oscar Mack Jr, driver purpose, FSFilter Activity Monitor band 360000-389999, explicit AV-range 320000-329998 avoidance) into `63-altitude-request.md`.
- USER sent the email 2026-06-07; recorded send-date + `pending` in `63-altitude-request.md` and the `DESIGN.md` request-status slot.

## Scaffold Defects Surfaced + Fixed by the Compile-Proof (the spike's purpose)

The SC2 compile-proof found and corrected five real defects in the Plan 63-01 `nono-fltmgr.vcxproj`:
1. MSB4019 — bogus `$(WDKContentRoot)\build\$(Platform)\WindowsDriver.props/.targets` imports removed (WDK logic comes from `PlatformToolset=WindowsKernelModeDriver10.0`). (`a250779b`)
2. Structural — driver config moved into a `Configuration` group after `Cpp.Default.props`. (`a250779b`)
3. MSB4025 — illegal `--` inside an XML comment. (`b1977e07`)
4. stampinf exit 87 — placeholder-INF stamping dropped from the compile-proof build (Phase 64 packaging). (`229658d0`)
5. signtool `/fd` — WDK auto test-sign rejected by the newer EWDK signtool (deleted the unsigned `.sys`); `SignMode=Off`, real signing is Phase 64. (`57fc6eb7`)

## Deviations

- **Execution order:** Task 2 ran before Task 1 (altitude clock is long-lead; Task 2 only depends on `DESIGN.md`).
- **SC1 method:** `az vm run-command` CLI equivalents instead of msinfo32 GUI (RDP/3389 blocked by corporate egress). Same facts.
- **SC2 channel:** Azure Bastion (443) after the legacy `az vm run-command` slot wedged; full VM stop/start + Bastion to recover. The EWDK auto-INF/auto-sign steps were scoped out of the compile-proof.
- The corrected `.vcxproj` is committed; the `.sys` is a throwaway VM-local artifact (not committed).

## Self-Check: PASSED

- [x] Standard-security-type VM (Secure Boot off, HVCI off, TESTSIGNING on) provisioned + captured (SC1)
- [x] `nono-fltmgr.sys` compiled from the scaffold with 0 MSBuild errors on the EWDK (SC2 / DRV-03 partial)
- [x] Altitude request drafted, sent 2026-06-07, recorded `pending` in `63-altitude-request.md` + `DESIGN.md`
- [x] No `.sys` binary committed to the repo
- [x] `63-SC1-vm-state.md` contains TESTSIGNING + Secure-Boot-off + HVCI-off + `nono-fltmgr.sys` evidence
