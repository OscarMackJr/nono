---
phase: 63-minifilter-spike-groundwork-macos-divergence-ledger-audit
verified: 2026-06-08T20:08:55Z
status: passed
score: 9/9 must-haves verified
overrides_applied: 0
---

# Phase 63: Minifilter Spike Groundwork + macOS Divergence Ledger Audit — Verification Report

**Phase Goal:** The minifilter spike has a working build environment and a written design doc before any driver code runs, and the macOS upstream commit inventory is complete so cherry-picks can begin in Phase 64.
**Verified:** 2026-06-08T20:08:55Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | drivers/nono-fltmgr/ WDK scaffold (.c/.vcxproj/.filters/.inf) exists and is committed | VERIFIED | `git ls-files drivers/` shows 5 files; commits `cadf4509` + `74812b18` in log |
| 2 | Skeleton .c registers empty callbacks array and does NO file I/O | VERIFIED | `grep FltRegisterFilter`, `IRP_MJ_OPERATION_END` pass; no `ZwCreateFile`/`NtCreateFile`/`FltCreateCommunicationPort` present |
| 3 | INF declares SERVICE_DEMAND_START (StartType=3) + FSFilter Activity Monitor altitude band placeholder (D-06) | VERIFIED | `StartType = 3`, `"FSFilter Activity Monitor"`, altitude `370020` (in 360000-389999, not AV range 320000-329998) |
| 4 | DESIGN.md exists as the hard pre-code gate (D-10) specifying all required BSOD-avoidance elements | VERIFIED | All 9 D-10 assertions pass: NonPagedPoolNx, FltSendMessage, STATUS_TIMEOUT, ring-buffer, worker-thread, IRQL, ZwCreateFile prohibition, 360000, 389999, 329998 |
| 5 | docs/architecture/minifilter-spike-design-pointer.md cross-links to canonical DESIGN.md | VERIFIED | File exists; contains literal path `drivers/nono-fltmgr/DESIGN.md` |
| 6 | 63-SC1-vm-state.md records: TESTSIGNING on, Secure Boot off, HVCI off, and msbuild-exit-0 + nono-fltmgr.sys compile proof (DRV-03 partial) | VERIFIED | `testsigning Yes` in bcdedit; `Secure Boot: Off`; `HVCI: Off` (VBS=0); `Build succeeded. 0 Error(s)`; `5,120 nono-fltmgr.sys` dir listing. VM: Standard_D4s_v4, `--security-type Standard`, eastus |
| 7 | 63-altitude-request.md records fsfcomm@microsoft.com + altitude band + send-date + pending status; DESIGN.md altitude slot filled | VERIFIED | `fsfcomm@microsoft.com`, `360000`, `320000-329998` avoidance, `send-date: 2026-06-07`, `pending`; DESIGN.md contains `PENDING — request sent 2026-06-07` |
| 8 | 63-DIVERGENCE-LEDGER.md audits upstream v0.57.0..v0.61.2 scoped to macOS surface with macos-only column, no windows-touch (MACOS-01) | VERIFIED | `range: v0.57.0..v0.61.2`, `macos-only` column present, `windows-touch` absent, drift-tool sha `0834aa664fbaf4c5e41af5debece292992211559` |
| 9 | All three P1 commits (8f84d454, 362ada22, 8f1b0b74) dispositioned will-sync; Headline explicitly supersedes Phase 54 C14 (D-13) | VERIFIED | All three SHAs present with `will-sync` disposition in C14 cluster row; Headline contains `SUPERSESSION OF PHASE 54 C14 (D-13)` |

**Score:** 9/9 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `drivers/nono-fltmgr/nono-fltmgr.c` | Skeleton DriverEntry + FltRegisterFilter + empty callbacks | VERIFIED | Contains FltRegisterFilter, FltStartFiltering, FltUnregisterFilter, IRP_MJ_OPERATION_END; no forbidden kernel I/O APIs |
| `drivers/nono-fltmgr/nono-fltmgr.vcxproj` | WDK MSBuild project (ConfigurationType=Driver, DriverType=WDM) | VERIFIED | Both `Driver` and `WDM` present; SignMode=Off (compile-proof mode) |
| `drivers/nono-fltmgr/nono-fltmgr.vcxproj.filters` | Solution-explorer grouping | VERIFIED | File tracked in git |
| `drivers/nono-fltmgr/nono-fltmgr.inf` | FSFilter Activity Monitor INF, StartType=3, altitude placeholder | VERIFIED | StartType=3, `"FSFilter Activity Monitor"`, altitude 370020 |
| `drivers/nono-fltmgr/DESIGN.md` | Pre-code BSOD-avoidance gate with all D-10 elements | VERIFIED | All 9 grep assertions pass |
| `docs/architecture/minifilter-spike-design-pointer.md` | ADR-home pointer stub | VERIFIED | Exists; cross-links to `drivers/nono-fltmgr/DESIGN.md` |
| `.planning/phases/63-.../63-SC1-vm-state.md` | VM state + compile-proof evidence | VERIFIED | TESTSIGNING, Secure Boot Off, HVCI Off, Build succeeded, nono-fltmgr.sys listing |
| `.planning/phases/63-.../63-altitude-request.md` | Altitude-request record + send-date + pending | VERIFIED | fsfcomm@microsoft.com, 360000-389999, send-date 2026-06-07, pending |
| `.planning/phases/63-.../63-DIVERGENCE-LEDGER.md` | Complete macOS-scoped upstream audit v0.57.0..v0.61.2 | VERIFIED | 19 clusters, macos-only column, C14 supersession, diff-inspect notes |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `drivers/nono-fltmgr/nono-fltmgr.inf` | `nono-fltmgr.sys` | ServiceBinary %12%\nono-fltmgr.sys | VERIFIED | INF contains `nono-fltmgr.sys` reference |
| `docs/architecture/minifilter-spike-design-pointer.md` | `drivers/nono-fltmgr/DESIGN.md` | Explicit cross-link | VERIFIED | Literal path `drivers/nono-fltmgr/DESIGN.md` in stub |
| `63-DIVERGENCE-LEDGER.md` | Phase 54 cluster C14 | Headline supersession | VERIFIED | `SUPERSESSION OF PHASE 54 C14 (D-13)` in Headline |
| `63-DIVERGENCE-LEDGER.md` | Three P1 commits | will-sync disposition rows | VERIFIED | 8f84d454, 362ada22, 8f1b0b74 all in C14 cluster with will-sync |
| `63-altitude-request.md` | DESIGN.md altitude slot | pending status in both | VERIFIED | Both files record send-date 2026-06-07 + pending |

---

### Data-Flow Trace (Level 4)

Not applicable — phase produces only documentation artifacts, WDK source scaffolding (no dynamic data rendering), and planning documents. No components render dynamic data.

---

### Behavioral Spot-Checks

Step 7b: SKIPPED — no runnable entry points. Phase 63 produces:
- WDK C source (requires WDK toolchain to build; SC2 compile proof was human-driven UAT on Azure VM — treated as authoritative per verification instructions)
- Planning documents (markdown)

No CLI commands, API endpoints, or Rust code were introduced.

---

### Probe Execution

No probes defined for this phase. Phase 63 is a documentation + WDK scaffolding phase with no scripted test-runner artifacts.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DRV-03 (partial) | 63-01, 63-02 | Reproducible driver build + test-signing pipeline documented/proven | SATISFIED | WDK scaffold source committed (63-01); SC2 compile proof (msbuild exit 0 producing nono-fltmgr.sys on EWDK) captured in 63-SC1-vm-state.md (63-02); DESIGN.md pre-code gate authored |
| MACOS-01 | 63-03 | DIVERGENCE-LEDGER.md audits upstream v0.57.0..v0.61.2 macOS surface with per-commit dispositions | SATISFIED | 63-DIVERGENCE-LEDGER.md: 63 commits, 19 clusters, macos-only column, all P1 commits will-sync, C14 supersession |

**DRV-03 scope note:** REQUIREMENTS.md traceability maps DRV-03 to "Phase 63 (partial groundwork) + Phase 64 (complete)". Phase 63 delivers the build-pipeline-documented-and-proven half; Phase 64 completes the full DRV-03 requirement (test-signing pipeline end-to-end). This is the intended partial coverage.

---

### Anti-Patterns Found

Scan run on all files created or modified by Phase 63: `drivers/nono-fltmgr/` (4 source files + DESIGN.md), `docs/architecture/minifilter-spike-design-pointer.md`, `63-SC1-vm-state.md`, `63-altitude-request.md`, `63-DIVERGENCE-LEDGER.md`.

**No TBD/FIXME/XXX debt markers found** in any phase-created file.

**Intentional placeholders (not blockers):**
- `nono-fltmgr.inf` altitude `370020` — documented as PLACEHOLDER in both the INF comment and DESIGN.md; Phase 64 replaces with official assigned altitude. Not a stub — the design contract explicitly states this is the placeholder phase.
- DESIGN.md altitude-request-status: `PENDING — request sent 2026-06-07` — this is the correctly filled status slot (Plan 63-02 responsibility), not a deferred stub.
- 63-SC1-vm-state.md "Exact image version (az exactVersion): *(pending `az vm show`)*" — the exact resolved image version string was not captured (the `latest` alias was recorded instead). This is an informational reproducibility gap: the image URN publisher:offer:sku is unambiguous (`MicrosoftWindowsDesktop:windows-11:win11-24h2-pro`), and the VM creation date (2026-06-07) bounds the resolution. The plan's automated verify check (`grep -q 'TESTSIGNING'`) passes; the security-relevant facts (TESTSIGNING, Secure Boot, HVCI, compile result) are all present. This is a WARNING-level note, not a BLOCKER.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `63-SC1-vm-state.md` | Provisioning table | Exact image version row says "*(pending `az vm show`)*" | INFO | Reproducibility: the `latest` alias is recorded but the resolved image version string (e.g. `22621.XXXXX`) was not captured before VM teardown. No security impact; Phase 64 will provision from fresh. |

---

### Human Verification Required

No human verification items. All observable truths are verified from committed codebase artifacts.

The SC1/SC2 human-driven UAT on the Azure VM is treated as authoritative per verification instructions. The committed evidence in `63-SC1-vm-state.md` provides the documented proof for DRV-03 (partial).

---

### Gaps Summary

No gaps. All 9 must-haves verified. The two required phase artifacts (DRV-03 partial + MACOS-01) are satisfied by committed evidence.

**DRV-03 (partial):** The WDK scaffold compiles-ready source exists in the repo (`drivers/nono-fltmgr/`), the DESIGN.md pre-code gate specifies the full BSOD-avoidance contract, and the SC2 compile proof demonstrates the scaffold produces a real `.sys` kernel binary on the EWDK toolchain. Five real scaffold defects were surfaced and fixed by the compile-proof (the spike's documented value). The altitude-request email was sent 2026-06-07 (pending reply ~30 business days).

**MACOS-01:** The divergence ledger covers 63 unique upstream commits in `v0.57.0..v0.61.2`, groups them into 19 clusters with dispositions (will-sync 12, split 3, won't-sync 4), provides a `macos-only` column, includes diff-inspect notes for every will-sync cluster covering the three required call sites (`generate_profile`, `sandbox_prepare`, `add_platform_rule`), and explicitly supersedes Phase 54's C14 won't-sync verdict. Phase 64 cherry-picks can begin from this ledger immediately.

---

_Verified: 2026-06-08T20:08:55Z_
_Verifier: Claude (gsd-verifier)_
