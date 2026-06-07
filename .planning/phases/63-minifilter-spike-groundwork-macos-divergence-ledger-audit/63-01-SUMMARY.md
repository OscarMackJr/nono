---
phase: 63-minifilter-spike-groundwork-macos-divergence-ledger-audit
plan: "01"
subsystem: kernel-driver-spike
tags: [wdk, minifilter, fltmgr, design-doc, pre-code-gate, bsod-avoidance]
dependency_graph:
  requires: []
  provides: [drivers/nono-fltmgr-scaffold, drivers/nono-fltmgr/DESIGN.md]
  affects: [63-02-PLAN, DRV-03]
tech_stack:
  added: [C/WDK-minifilter, MSBuild-Driver-project, INF-FSFilter-ActivityMonitor]
  patterns: [ring-buffer+worker-thread IPC (specified), WDK-nullFilter-scaffold]
key_files:
  created:
    - drivers/nono-fltmgr/nono-fltmgr.c
    - drivers/nono-fltmgr/nono-fltmgr.vcxproj
    - drivers/nono-fltmgr/nono-fltmgr.vcxproj.filters
    - drivers/nono-fltmgr/nono-fltmgr.inf
    - drivers/nono-fltmgr/DESIGN.md
    - docs/architecture/minifilter-spike-design-pointer.md
  modified: []
decisions:
  - "D-09 path correction applied: .planning/adr/ does not exist; pointer stub placed in docs/architecture/ consistent with 6 existing ADR siblings"
  - "Altitude placeholder 370020 in FSFilter Activity Monitor band (360000-389999); Phase 64 picks final non-colliding number after fltmc filters enumeration"
  - "DESIGN.md authored as a hard pre-code gate (D-10) using docs/architecture/broker-trust-anchor.md house-style header block"
metrics:
  duration_minutes: 5
  tasks_completed: 2
  tasks_total: 2
  files_created: 6
  files_modified: 0
  completed_date: "2026-06-07"
---

# Phase 63 Plan 01: WDK Scaffold + DESIGN.md Pre-Code Gate Summary

**One-liner:** WDK minifilter skeleton (DriverEntry + empty callbacks, nullFilter structure) with BSOD-avoidance DESIGN.md gate specifying ring-buffer+worker-thread IPC, NonPagedPoolNx, finite FltSendMessage timeout, and altitude band constraints.

## What Was Built

### Task 1: drivers/nono-fltmgr/ WDK scaffold

Four scaffold files created under the new `drivers/nono-fltmgr/` directory (not a Cargo workspace member):

**`nono-fltmgr.c`** — skeleton DriverEntry mirroring Microsoft's `nullFilter` sample. Registers `FltRegisterFilter` + `FltStartFiltering` + `NonoFltUnload`. EMPTY operation-callbacks array (`{ IRP_MJ_OPERATION_END }`). No file I/O, no FltCreateCommunicationPort, no pre-create callback body — all Phase 64.

**`nono-fltmgr.vcxproj`** — WDK MSBuild project with `ConfigurationType=Driver` and `DriverType=WDM` (minifilters are WDM-class FS filters, not KMDF). Includes Spectre mitigation flags and `fltMgr.lib` linkage. WDK `.props`/`.targets` imported. Mirrors nullFilter.vcxproj structure.

**`nono-fltmgr.vcxproj.filters`** — solution-explorer grouping (Source Files / Driver Install).

**`nono-fltmgr.inf`** — `Class = "ActivityMonitor"` with standard ActivityMonitor ClassGuid. `ServiceBinary = %12%\nono-fltmgr.sys`. `Dependencies = "FltMgr"`. `ServiceType = 2` (SERVICE_FILE_SYSTEM_DRIVER). `StartType = 3` (SERVICE_DEMAND_START, D-06 boot-loop safeguard). `LoadOrderGroup = "FSFilter Activity Monitor"`. Altitude placeholder `370020` in FSFilter Activity Monitor band (360000-389999) with explicit comment that Phase 64 must not use AV range 320000-329998.

### Task 2: DESIGN.md + docs/architecture/ pointer stub

**`drivers/nono-fltmgr/DESIGN.md`** — hard pre-code gate (D-10) using `docs/architecture/broker-trust-anchor.md` house-style header block (`**Status:**` / `**Date:**` / `**Phase:**` / `**Requirement:**` / `**Related ADR:**` / `## Context`). Contains:

- STRIDE threat register (T-63-01 through T-63-05): recursive I/O BSOD, infinite FltSendMessage hang, IRQL violation, altitude collision, spike .sys repo leakage
- Ring-buffer + worker-thread IPC architecture diagram and 6 binding design rules
- Altitude configuration table (band 360000-389999, AV range 320000-329998 avoidance, placeholder 370020, Microsoft assignment status: **PENDING** with slot for Plan 63-02 to fill)
- Phase 63/64 scope boundary table
- V5 input validation note for Phase 64 (static layout assertion on IPC struct)

All D-10 grep assertions pass: `NonPagedPoolNx`, `FltSendMessage`, `STATUS_TIMEOUT`, `ring-buffer`, `worker-thread`, `KeGetCurrentIrql`/`IRQL`, `ZwCreateFile`, `360000`, `389999`, `329998`.

**`docs/architecture/minifilter-spike-design-pointer.md`** — ADR-home pointer stub (path correction: `.planning/adr/` does not exist; `docs/architecture/` is the real ADR home per PATTERNS.md). Cross-links to `drivers/nono-fltmgr/DESIGN.md` as the canonical pre-code gate. Uses the same house-style header block as the 6 existing ADR siblings.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| Task 1: WDK scaffold | `cadf4509` | feat(63-01): add drivers/nono-fltmgr/ WDK scaffold (.c, .vcxproj, .inf) |
| Task 2: DESIGN.md + pointer | `74812b18` | docs(63-01): add DESIGN.md pre-code gate + docs/architecture/ pointer stub |

## Verification Results

All automated checks pass:

- `nono-fltmgr.c` contains `FltRegisterFilter`, `FltStartFiltering`, `FltUnregisterFilter`, `IRP_MJ_OPERATION_END`; no `ZwCreateFile`/`NtCreateFile`/`FltCreateCommunicationPort`
- `nono-fltmgr.vcxproj` contains `Driver` (ConfigurationType) and `WDM` (DriverType)
- `nono-fltmgr.inf` contains `StartType = 3`, `"FSFilter Activity Monitor"`, altitude 370020 (in band 360000-389999, not in AV range 320000-329998)
- `DESIGN.md` passes all D-10 content assertions (9/9)
- `docs/architecture/minifilter-spike-design-pointer.md` contains literal path `drivers/nono-fltmgr/DESIGN.md`
- No `.sys` binary committed; `drivers/` is not a Cargo workspace member; `crates/nono-cli/data/windows/nono-wfp-driver.sys` untouched
- No `Cargo.toml`/`Cargo.lock` modifications

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Forbidden-call strings in comments matched automated grep check**
- **Found during:** Task 1 verification
- **Issue:** The plan's automated verify uses `! grep -Eq 'ZwCreateFile|NtCreateFile|FltCreateCommunicationPort'` against `nono-fltmgr.c`. The initial draft included comment text like `// No ZwCreateFile / NtCreateFile in this file` which matched the pattern, causing the check to fail even though no actual calls exist.
- **Fix:** Replaced comment text with equivalent prose that does not contain the forbidden API names (e.g., "No kernel file-open APIs").
- **Files modified:** `drivers/nono-fltmgr/nono-fltmgr.c`
- **Commit:** included in `cadf4509`

### Path Correction (D-09 — noted in PATTERNS.md)

**2. [Plan-specified path correction] `.planning/adr/` → `docs/architecture/`**
- The plan notes (CONTEXT, PATTERNS.md) that `.planning/adr/` does not exist and the pointer stub should live in `docs/architecture/` consistent with the 6 existing ADRs.
- Applied as directed: pointer stub placed at `docs/architecture/minifilter-spike-design-pointer.md`.
- This is a plan-documented correction, not a deviation.

## Known Stubs

None that affect the plan's goal. The altitude value `370020` in `nono-fltmgr.inf` is an intentional PLACEHOLDER documented in both the INF and DESIGN.md; it is Phase 64's responsibility to select the final non-colliding number after `fltmc filters` enumeration on the test VM. The Microsoft altitude-request status slot in DESIGN.md is intentionally `PENDING` — Plan 63-02 fills in the send date.

## Threat Flags

No new security-relevant surface introduced beyond the plan's `<threat_model>`. The `drivers/nono-fltmgr/` directory is:
- Source-only (no `.sys` binary)
- Not a Cargo workspace member
- Not installed or registered at this phase
- Not connected to the MSI or `nono-wfp-driver.sys` placeholder

## Self-Check: PASSED

All 7 files exist on disk. Both task commits (`cadf4509`, `74812b18`) confirmed in git log.
