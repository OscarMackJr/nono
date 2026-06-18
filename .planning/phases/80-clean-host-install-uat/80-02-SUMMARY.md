---
phase: 80-clean-host-install-uat
plan: "02"
subsystem: gates
tags: [dark-factory, gate, msiexec, powershell, inst-01]
dependency_graph:
  requires: ["80-01", "76-02"]
  provides: ["scripts/gates/clean-host-install.ps1", "INST-01 structural gate"]
  affects: [".nono-runtime/verdicts/clean-host-install.json"]
tech_stack:
  added: []
  patterns: ["Phase 76 Test-Precondition/Invoke-Gate gate contract", "Start-Process -Wait -PassThru exit-code idiom", "fresh-session PATH propagation proof"]
key_files:
  created:
    - scripts/gates/clean-host-install.ps1
  modified: []
decisions:
  - "D-01: SKIP_HOST_UNAVAILABLE on dirty dev host is the structurally verifiable outcome; PASS requires operator-provided clean Win11 VM"
  - "D-02 dirty-host detection: nono.exe under Program Files OR nono-wfp-service/nono-agentd service registered triggers SKIP"
  - "All Start-Process invocations use -Wait -PassThru and read .ExitCode (never $LASTEXITCODE after bare & call)"
  - "Fresh pwsh child process used for nono --version to prove MSI PATH propagation (current session PATH is frozen)"
  - "Service state probe is non-fatal per D-06 (recorded in detail, never flips PASS to FAIL)"
  - "Gate never calls exit or Persist-Verdict (runner contract)"
metrics:
  duration: "25 minutes"
  completed_date: "2026-06-18"
  tasks_completed: 1
  tasks_pending_checkpoint: 1
  files_created: 1
  files_modified: 0
---

# Phase 80 Plan 02: Clean-Host Install Gate Summary

**One-liner:** Dark-factory gate `clean-host-install.ps1` implementing Phase 76 Test-Precondition/Invoke-Gate contract with D-02 dirty-host SKIP detection and four-step PASS criteria (msiexec install + fresh-session nono --version + non-fatal service probe + uninstall cleanup).

## Status: CHECKPOINT PENDING (Task 2 — human-verify)

Task 1 is complete and committed. Task 2 is a `checkpoint:human-verify` gate awaiting operator verification on the dirty dev host (and ultimately on a clean Win11 VM for the live PASS).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create scripts/gates/clean-host-install.ps1 | `2b969737` | scripts/gates/clean-host-install.ps1 (188 lines, created) |

## Tasks Pending

| Task | Type | Status |
|------|------|--------|
| 2 | checkpoint:human-verify | Awaiting operator verification |

## What Was Built

`scripts/gates/clean-host-install.ps1` — an unattended dark-factory gate that implements the Phase 76 two-function contract for the INST-01 requirement (machine MSI installs clean on a fresh Win11 host).

### Gate Structure

**`Test-Precondition`** (SKIP_HOST_UNAVAILABLE detection — D-02):
1. Elevation check (machine MSI needs admin; exact two-line WindowsIdentity form from wfp-egress-isolation.ps1)
2. `Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe'` — dirty host if found
3. `Get-Service nono-wfp-service/nono-agentd` — dirty host if either registered
4. `Test-Path -LiteralPath $script:MsiPath` — SKIP if MSI not staged

**`Invoke-Gate`** (four-step PASS criteria per D-06):
1. `msiexec /i <MSI> /quiet /norestart /l*v <log>` — exit 0 or 3010 = installOk
2. Fresh `pwsh -NoProfile -NonInteractive -Command 'nono --version'` — PATH propagation proof
3. `Get-Service nono-wfp-service` — service state recorded in detail (non-fatal)
4. `msiexec /x <MSI> /quiet /norestart` — cleanup for repeatability

### Contract Compliance

| Requirement | Status |
|-------------|--------|
| No `exit` calls | Confirmed — `grep -n "^\s*exit\b"` returns empty |
| No `Persist-Verdict` calls | Confirmed — only appears in header comment (rule doc) |
| `Test-Precondition` exported | Confirmed — dot-source test passes |
| `Invoke-Gate` exported | Confirmed — dot-source test passes |
| `LiteralPath` for file checks | Confirmed — 4 occurrences (T-80-04) |
| `Start-Process` -Wait -PassThru (>=3) | Confirmed — 5 occurrences |
| No `$LASTEXITCODE` usage | Confirmed — 0 occurrences |
| Verdict key order: gate;verdict;reason;detail;timestamp | Confirmed — all return objects follow this order |

### Structural Verification (dirty dev host)

```
pwsh -NoProfile -Command "scripts/verify-dark.ps1 -Gate clean-host-install"
```

**Result:** Exit 3, verdict:
```json
{
  "gate": "clean-host-install",
  "verdict": "SKIP_HOST_UNAVAILABLE",
  "reason": "nono.exe detected under C:\\Program Files\\nono - host is not clean; ...",
  "detail": {},
  "timestamp": "2026-06-18T08:49:59.291Z"
}
```

Verdict file written to `.nono-runtime/verdicts/clean-host-install.json`.

## Deviations from Plan

None — plan executed exactly as written. The gate file was constructed following the primary analog (wfp-egress-isolation.ps1) and the patterns from 80-PATTERNS.md verbatim.

## Known Stubs

None. The gate is complete. Live PASS requires:
1. A clean Win11 VM (no prior nono install)
2. MSI rebuilt after Plan 80-01 (Vital="no" + +crt-static fixes must be in the artifact)
These are structural prerequisites, not stubs in the gate code itself.

## Threat Flags

No new threat surface introduced. Gate file is pure PowerShell; no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. All STRIDE mitigations from the plan's threat_model are implemented:
- T-80-04: All file checks use `Test-Path -LiteralPath` (4 occurrences)
- T-80-05: Log path is `Join-Path $env:TEMP 'nono-gate-install.log'` (fixed suffix, no operator input)
- T-80-07: All `Start-Process -PassThru` results assigned to named variables (no stray pipeline output)

## Checkpoint Pending (Task 2)

**Type:** checkpoint:human-verify  
**Gate:** blocking  

**What to verify:**

Step 1 — Non-elevated SKIP path (from a non-elevated PowerShell):
```powershell
pwsh -NoProfile -Command "scripts/verify-dark.ps1 -Gate clean-host-install"
```
Expected: exit 3, verdict `SKIP_HOST_UNAVAILABLE`, reason mentions "requires elevation"

Step 2 — Elevated SKIP path (from an elevated PowerShell):
```powershell
pwsh -NoProfile -Command "scripts/verify-dark.ps1 -Gate clean-host-install"
```
Expected: exit 3, verdict `SKIP_HOST_UNAVAILABLE`, reason mentions nono.exe or services detected

Step 3 — Verdict file check:
```powershell
Get-Content .nono-runtime\verdicts\clean-host-install.json | ConvertFrom-Json
```
Expected: `verdict` field = "SKIP_HOST_UNAVAILABLE"

Step 4 — Structural checks (Git Bash):
```bash
grep -c "function Test-Precondition" scripts/gates/clean-host-install.ps1  # → 1
grep -c "function Invoke-Gate" scripts/gates/clean-host-install.ps1        # → 1
grep -n "^\s*exit\b" scripts/gates/clean-host-install.ps1                  # → empty
```

**Live PASS (requires operator-provided clean Win11 VM + rebuilt MSI):**
- Rebuild MSI after Plan 80-01 lands (Vital="no" + +crt-static must be in artifact)
- Stage `dist\windows\nono-machine.msi` on fresh VM
- Run `pwsh scripts/verify-dark.ps1 -Gate clean-host-install` elevated
- Expected: exit 0, verdict PASS, detail.versionOutput contains nono version string

## Self-Check: PASSED

- `scripts/gates/clean-host-install.ps1`: FOUND
- Commit `2b969737`: FOUND in git log
- `verify-dark.ps1 -Gate clean-host-install` exits 3 (SKIP_HOST_UNAVAILABLE): CONFIRMED
- `.nono-runtime/verdicts/clean-host-install.json` written: CONFIRMED
