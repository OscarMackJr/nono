---
phase: 80-clean-host-install-uat
plan: "01"
subsystem: build-pipeline
tags:
  - windows
  - msi
  - static-crt
  - wix
  - ci
  - release
dependency_graph:
  requires:
    - "Phase 80 plan context (D-03 + D-04 decisions)"
  provides:
    - "Static CRT linkage for all Windows MSVC builds (CI + release + local)"
    - "Non-fatal nono-wfp-service install (Vital=no on ServiceInstall)"
    - "Contract assertion locking Vital=no + ErrorControl=ignore in validate-windows-msi-contract.ps1"
  affects:
    - ".cargo/config.toml (new)"
    - ".github/workflows/ci.yml"
    - ".github/workflows/release.yml"
    - "scripts/build-windows-msi.ps1"
    - "scripts/validate-windows-msi-contract.ps1"
tech_stack:
  added:
    - ".cargo/config.toml — target-scoped rustflags for local dev Windows MSVC builds"
  patterns:
    - "step-level RUSTFLAGS env override in GitHub Actions YAML (overrides global env for Windows steps only)"
    - "Vital=no on WiX v4 ServiceInstall (YesNoTypeUnion, PascalCase — non-fatal service install)"
    - "Assert-Equal contract lock for MSI non-fatal service attributes"
key_files:
  created:
    - ".cargo/config.toml"
  modified:
    - ".github/workflows/ci.yml"
    - ".github/workflows/release.yml"
    - "scripts/build-windows-msi.ps1"
    - "scripts/validate-windows-msi-contract.ps1"
decisions:
  - "D-03 static CRT wired via TWO paths: config.toml covers local dev (no RUSTFLAGS env), step-level RUSTFLAGS env override covers CI/release (where global RUSTFLAGS overrides config.toml)"
  - "D-04 non-fatal service uses Vital=no (PascalCase) on ServiceInstall per WiX v4 XSD — ServiceControl has no Vital attribute; ErrorControl=ignore added as belt-and-suspenders at SCM boot-time level"
  - "release.yml Build step split into non-Windows (if: runner.os != Windows) + Windows (if: runner.os == Windows) to isolate +crt-static to MSVC target only"
metrics:
  duration: "~15 minutes"
  completed: "2026-06-18"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 5
---

# Phase 80 Plan 01: Wire +crt-static + Vital=no Build Fix Summary

**One-liner:** Static CRT linkage (`+crt-static`) wired across all Windows MSVC build paths and `Vital="no"` added to WiX `ServiceInstall` to make nono-wfp-service start non-fatal, closing the Phase 67 INST-01 clean-host install failure.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Wire +crt-static across all Windows build paths (D-03) | `a517284b` | `.cargo/config.toml` (new), `ci.yml`, `release.yml` |
| 2 | Patch Vital=no + ErrorControl=ignore + contract assertions (D-04) | `cd856641` | `build-windows-msi.ps1`, `validate-windows-msi-contract.ps1` |

## What Was Built

### Task 1: Static CRT Linkage (D-03)

**Problem:** `RUSTFLAGS` env var and `.cargo/config.toml` rustflags are mutually exclusive in Cargo — `RUSTFLAGS` wins and silently drops config.toml entries. CI sets `RUSTFLAGS: -Dwarnings` globally; `config.toml` alone would never reach the CI compiler.

**Solution (two-pronged):**

1. **`.cargo/config.toml`** — new file with `[target.x86_64-pc-windows-msvc]` section only (NOT `[build]`). Covers local dev builds where `RUSTFLAGS` is not set. Contains:
   ```toml
   [target.x86_64-pc-windows-msvc]
   rustflags = ["-C", "target-feature=+crt-static"]
   ```

2. **`ci.yml`** — added step-level `env: RUSTFLAGS: -Dwarnings -C target-feature=+crt-static` to the windows-packaging "Build Windows release binaries" step. Overrides the global `RUSTFLAGS: -Dwarnings` for that step only. Linux/macOS steps inherit the global value unchanged.

3. **`release.yml`** — split "Build" step into:
   - Non-Windows: `if: matrix.target != 'aarch64-unknown-linux-gnu' && runner.os != 'Windows'` (unchanged RUSTFLAGS from global env — which is absent in release.yml)
   - "Build (Windows — static CRT)": `if: runner.os == 'Windows'` with `env: RUSTFLAGS: -Dwarnings -C target-feature=+crt-static`
   - "Build broker (Windows)": existing `if: runner.os == 'Windows'` step now has `env: RUSTFLAGS: -Dwarnings -C target-feature=+crt-static`

**D-03 isolation confirmed:** Linux/macOS legs in release.yml do not inherit `+crt-static`. The ci.yml global env block is unchanged.

**Build verification:** `cargo build -p nono-cli` succeeded on the Windows dev host — config.toml has no syntax errors.

**dumpbin check:** dumpbin is not available outside a VS Developer Command Prompt on the dev host; this optional check was not run. The structural config correctness (confirmed by build success + grep gates) is the primary evidence per D-06.

### Task 2: Non-Fatal Service Install (D-04)

**Problem:** With `Vital` absent (defaults to `yes`) on `ServiceInstall`, any service start failure during MSI installation triggers a full rollback (exit 1603). `ErrorControl="normal"` affects only SCM boot-time behavior — it has ZERO effect on install-time rollback.

**Solution:**

1. **`scripts/build-windows-msi.ps1`** — in the `$serviceComponentXml` here-string:
   - Changed `ErrorControl="normal"` → `ErrorControl="ignore"` (belt-and-suspenders at SCM boot-time level — prevents error dialogs and last-known-good restart attempts on failed service start; does NOT affect install-time rollback)
   - Added `Vital="no"` (PascalCase, `YesNoTypeUnion`) on `ServiceInstall` — the WiX v4 XSD-correct attribute that makes service install/start failure non-fatal to the MSI install. "Failure to install the service will be ignored."
   - `ServiceControl` is UNCHANGED — it has no `Vital` attribute in the WiX v4 XSD (only `Id`, `Name`, `Start`, `Stop`, `Remove`, `Wait`)
   - `Start="install"` on `ServiceControl` is preserved — the intent is "try to start, but don't roll back on failure"

2. **`scripts/validate-windows-msi-contract.ps1`** — added two `Assert-Equal` calls after the existing `$machineServiceControl.Wait` assertion:
   - `Assert-Equal -Actual $machineServiceInstall.ErrorControl -Expected "ignore"`
   - `Assert-Equal -Actual $machineServiceInstall.Vital -Expected "no"`

   Both check `$machineServiceInstall` (ServiceInstall node). Neither checks `$machineServiceControl` (ServiceControl has no Vital attribute). These lock the non-fatal service contract so any future refactor that reverts them is caught immediately.

## Verification Gates (all passed)

All 11 structural grep gates from the plan verification section:

| Gate | Expected | Result |
|------|----------|--------|
| `grep -c "target.x86_64-pc-windows-msvc" .cargo/config.toml` | 1 | **1** |
| `grep -c "crt-static" .cargo/config.toml` | 1 | **1** |
| `grep -c "crt-static" .github/workflows/ci.yml` | ≥1 | **1** |
| `grep -c "crt-static" .github/workflows/release.yml` | ≥2 | **2** |
| `grep -c 'Vital="no"' scripts/build-windows-msi.ps1` | 1 | **1** |
| `grep -c 'ErrorControl="ignore"' scripts/build-windows-msi.ps1` | 1 | **1** |
| `grep -c 'ErrorControl="normal"' scripts/build-windows-msi.ps1` | 0 | **0** |
| `grep -c 'vital' scripts/build-windows-msi.ps1` | 0 | **0** |
| `grep -c 'machineServiceInstall.Vital' scripts/validate-windows-msi-contract.ps1` | 1 | **1** |
| `grep -c 'machineServiceInstall.ErrorControl' scripts/validate-windows-msi-contract.ps1` | 1 | **1** |
| `grep -c 'machineServiceControl.vital' scripts/validate-windows-msi-contract.ps1` | 0 | **0** |

`dist/windows/nono-machine.wxs` NOT modified (`git diff dist/windows/nono-machine.wxs` — empty).

`cargo build -p nono-cli` — succeeded on Windows dev host.

## Deviations from Plan

None — plan executed exactly as written.

The PATTERNS.md comment "Do NOT change ErrorControl" was inconsistent with the PLAN task 2 action and the RESEARCH.md code examples/assertions. The PLAN (and RESEARCH.md A3 RESOLVED section) were authoritative: both `ErrorControl="ignore"` and `Vital="no"` were applied as specified.

## Known Stubs

None. All five files are complete implementations with no placeholder content.

## Threat Flags

None. The changes are:
- YAML CI/release config (step-level env var, hardcoded string in version-controlled YAML)
- `.cargo/config.toml` (target-scoped build config, dev convenience)
- PowerShell build script (here-string attribute change, version-controlled)
- PowerShell contract validator (add assertions)

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries were introduced.

## Self-Check: PASSED

Files exist:
- `.cargo/config.toml` — FOUND
- `.github/workflows/ci.yml` (modified) — FOUND
- `.github/workflows/release.yml` (modified) — FOUND
- `scripts/build-windows-msi.ps1` (modified) — FOUND
- `scripts/validate-windows-msi-contract.ps1` (modified) — FOUND

Commits exist:
- `a517284b` — Task 1 commit — FOUND
- `cd856641` — Task 2 commit — FOUND
