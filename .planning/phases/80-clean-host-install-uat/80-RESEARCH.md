# Phase 80: Clean-Host Install UAT - Research

**Researched:** 2026-06-17
**Domain:** Windows MSI packaging (static CRT, WiX non-fatal service) + Phase 76 gate contract
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** SKIP-on-dirty + operator-provided fresh VM. `Test-Precondition` detects whether the current host is clean; if not, returns `SKIP_HOST_UNAVAILABLE`. PASS is only achievable on a fresh Win11 VM/snapshot.
- **D-02:** "Clean" = no prior nono only. Skip if `nono.exe` exists under `C:\Program Files\nono\` OR a `nono-wfp-service` / `nono-agentd` service is registered. Does NOT require VC++ runtime to be absent.
- **D-03:** Static CRT (`+crt-static`). Build all four Windows binaries (nono, nono-shell-broker, nono-wfp-service, nono-agentd) with `target-feature=+crt-static`. Windows-MSVC-only change; must not affect Linux/macOS linkage.
- **D-04:** Non-fatal `nono-wfp-service` start (`vital="no"` posture on the relevant `ServiceInstall`/`ServiceControl`). Both D-03 and D-04 ship: belt-and-suspenders.
- **D-05:** Install-level only. Gate asserts install exit 0 + `nono --version` runs. Does NOT exercise the broker/supervised path. Untrusted-POC-cert broker failure is a known deferred limitation (DIST-SIGN-01).
- **D-06:** PASS requires ALL of: `msiexec /i` exits 0, `nono --version` from a NEW PowerShell session, nono-wfp-service start is non-fatal (service state reported in `detail`; stopped/failed is NOT a FAIL), then uninstall (`msiexec /x`) for repeatability.
- **D-07:** `-MsiPath` param, default `dist\windows\nono-machine.msi`. Operator stages that MSI on the fresh VM. Unsigned local build is acceptable.

### Claude's Discretion

- Exact `detail` JSON fields beyond the verdict contract.
- Precise WiX attribute(s) that make the service non-fatal and exact `.cargo/config.toml` stanza for `+crt-static`.
- Whether to add an optional dumpbin/`link /dump /imports` assertion.

### Deferred Ideas (OUT OF SCOPE)

- Publicly-trusted code signing (DIST-SIGN-01) — enterprise milestone.
- Windows Sandbox auto-run of the clean-host gate.
- Optional `nono setup --trust-broker` helper / POC-cert import UX.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INST-01 | The machine MSI installs and runs on a clean Win11 host with no manual steps, verified by an unattended clean-host harness. | Static CRT eliminates the VC++ 1603/0xC0000135 failure; `vital="no"` eliminates the service-start rollback; the gate script provides the unattended verification harness. |
</phase_requirements>

---

## Summary

Phase 80 has two independent work items that must both ship. The first is a **build fix**: the four Windows binaries built for the machine MSI currently link the MSVC CRT dynamically, which causes `0xC0000135` STATUS_DLL_NOT_FOUND and a `1603` MSI rollback on any host that does not have `vc_redist.x64.exe` installed. The fix is to add a `[target.x86_64-pc-windows-msvc]` stanza to `.cargo/config.toml` that sets `rustflags = ["-C", "target-feature=+crt-static"]` — scoped to the Windows MSVC target so it does not affect the Linux or macOS toolchains. Separately, the `ServiceControl Start="install"` in the generated WiX makes the first service-start a fatal MSI operation; making it non-fatal requires adding `vital="no"` to the `ServiceControl` element (the correct WiX v4 attribute) and, for belt-and-suspenders, setting `ErrorControl="ignore"` on `ServiceInstall`.

The second work item is a **new gate file**: `scripts/gates/clean-host-install.ps1`, following the `Test-Precondition` / `Invoke-Gate` contract defined in Phase 76, that orchestrates an unattended `msiexec /i ... /quiet` + fresh-session `nono --version` + `msiexec /x ... /quiet` on a clean host and returns a typed verdict object. On the dev (dirty) host it returns `SKIP_HOST_UNAVAILABLE`; on a fresh VM it returns `PASS`.

**Primary recommendation:** Create `.cargo/config.toml` with a `[target.x86_64-pc-windows-msvc]` rustflags block (not a global `[build]` block, and not `RUSTFLAGS` in the build script), update `scripts/build-windows-msi.ps1`'s service here-string to add `vital="no"` on `ServiceControl` and `ErrorControl="ignore"` on `ServiceInstall`, and write `scripts/gates/clean-host-install.ps1` following the `wfp-egress-isolation.ps1` reference implementation.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Static CRT linkage | Build config (.cargo/config.toml, target-scoped) | CI pipeline (release.yml must not override with a conflicting RUSTFLAGS) | Cargo consumes rustflags at build time; per-target scope is the safe mechanism |
| Non-fatal service start | MSI definition (scripts/build-windows-msi.ps1 here-string) | validate-windows-msi-contract.ps1 (add assertion) | The .wxs is generated from build-windows-msi.ps1 — never edit the .wxs directly |
| Clean-host detection | Gate script Test-Precondition | — | Filesystem/registry probes are pure PowerShell; no native tool needed |
| msiexec orchestration | Gate script Invoke-Gate | — | Shells out to msiexec; maps exit code to verdict |
| `nono --version` PATH proof | Gate script Invoke-Gate | — | Launched in a new pwsh session to prove PATH propagation |
| Verdict persistence | verify-dark.ps1 runner (Persist-Verdict) | — | Gate must NOT call Persist-Verdict — only the runner owns the canonical file |

---

## Standard Stack

### Core

| Tool/Library | Version | Purpose | Why Standard |
|---|---|---|---|
| Cargo target-scoped rustflags | N/A | Static CRT linkage on Windows MSVC | The only isolation mechanism that scopes to a single target triple without polluting other targets |
| WiX v4 `vital` attribute | v4 schema (WiX 7 on CI) | Make ServiceControl non-fatal | WiX v4 replaced the WiX v3 `ServiceControl/@Vital` with `vital` (lowercase); the project uses `wix build ... -acceptEula wix7` |
| `ServiceInstall ErrorControl` | SCM attribute | Tell SCM a start failure is non-fatal | Complementary to `vital="no"` — SCM won't pop an error dialog |
| `ProcessStartInfo.ArgumentList` | .NET | msiexec / pwsh invocation in gate | Used in copilot-e2e.ps1; handles space-in-path quoting correctly |

### Supporting

| Tool/Library | Version | Purpose | When to Use |
|---|---|---|---|
| `dumpbin /imports` | MSVC toolchain | Verify no `vcruntime140.dll` import in built binaries | Optional detail field in the gate verdict; confirms static CRT proof independently of clean-host success |
| `Get-Service` | PowerShell builtin | Detect registered nono services in Test-Precondition | Sole mechanism for the service-exists check in D-02 |
| `New-Object System.Security.Principal.WindowsPrincipal` | .NET | Elevation check (if gate requires admin for msiexec /i perMachine) | Machine-scope MSI install requires admin; Test-Precondition should return SKIP_HOST_UNAVAILABLE (not fail) if not elevated |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `.cargo/config.toml` `[target.x86_64-pc-windows-msvc]` | `RUSTFLAGS` env var in build script or CI | `RUSTFLAGS` in the build script only affects that script's cargo invocation; setting it globally in CI pollutes non-Windows targets and conflicts with the existing `RUSTFLAGS: -Dwarnings` CI env. Per-target config.toml is the safe, reviewed approach. |
| `.cargo/config.toml` `[target.x86_64-pc-windows-msvc]` | `[build] rustflags` in config.toml | `[build]` rustflags applies to ALL targets; `+crt-static` on Linux/macOS-musl targets changes the ABI. Must be target-scoped. |
| `vital="no"` on ServiceControl | Removing `Start="install"` from ServiceControl | Removing `Start` means the service is never started at install time, which is a behavior change. `vital="no"` preserves the start attempt and records the outcome without rolling back. |
| `vital="no"` on ServiceControl | WiX `<CustomAction>` with `Return="ignore"` wrapping service start | Overly complex; WiX already models this with `vital`. |

---

## Package Legitimacy Audit

> No external packages are introduced by this phase. The gate script is pure PowerShell using built-in cmdlets and the existing `msiexec` system tool. No npm/PyPI/crates packages are added.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
Dev host (dirty)                    Fresh VM (clean)
┌──────────────────┐               ┌──────────────────────────────────┐
│ pwsh verify-dark │               │ pwsh verify-dark                 │
│  --gate clean-   │               │  --gate clean-host-install       │
│  host-install    │               │  [-MsiPath dist\windows\nono-    │
│                  │               │   machine.msi]                   │
│ ┌─────────────┐  │               │                                  │
│ │Test-         │  │               │ ┌───────────────────────────┐    │
│ │Precondition │  │               │ │ Test-Precondition          │    │
│ │             │  │               │ │  - nono.exe under PF\nono? │    │
│ │ nono.exe    │  │               │ │  - svc nono-wfp-service    │    │
│ │ found →     │  │               │ │    registered?             │    │
│ │ returns     │  │               │ │  - svc nono-agentd         │    │
│ │ reason str  │  │               │ │    registered?             │    │
│ └──────┬──────┘  │               │ │  → $null (all clear)       │    │
│        │         │               │ └───────────┬───────────────┘    │
│        ▼         │               │             │                     │
│ Runner emits     │               │             ▼                     │
│ SKIP_HOST_       │               │ ┌───────────────────────────┐    │
│ UNAVAILABLE      │               │ │ Invoke-Gate               │    │
│ exit 3           │               │ │                            │    │
└──────────────────┘               │ │ 1. msiexec /i <msi>       │    │
                                   │ │    /quiet → exit 0?        │    │
                                   │ │                            │    │
                                   │ │ 2. pwsh -NoProfile         │    │
                                   │ │    -Command nono --version  │    │
                                   │ │    → capture output        │    │
                                   │ │                            │    │
                                   │ │ 3. Get-Service             │    │
                                   │ │    nono-wfp-service →      │    │
                                   │ │    record state in detail  │    │
                                   │ │                            │    │
                                   │ │ 4. msiexec /x <msi>       │    │
                                   │ │    /quiet → cleanup        │    │
                                   │ │                            │    │
                                   │ │ → verdict PASS/FAIL object │    │
                                   │ └───────────────────────────┘    │
                                   │                                  │
                                   │ Runner persists → emits → exit 0 │
                                   └──────────────────────────────────┘
```

### Recommended Project Structure

```
scripts/
  verify-dark.ps1                  # runner (Phase 76, unchanged)
  build-windows-msi.ps1            # MODIFIED: vital="no" + ErrorControl="ignore"
  validate-windows-msi-contract.ps1 # MODIFIED: add vital="no" assertion
  gates/
    harness-self-check.ps1         # Phase 76
    copilot-e2e.ps1                # Phase 77
    wfp-egress-isolation.ps1       # Phase 79
    clean-host-install.ps1         # Phase 80 (NEW)
.cargo/
  config.toml                      # NEW: [target.x86_64-pc-windows-msvc] rustflags
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Static CRT on Windows | Custom build.rs that sets link flags | `.cargo/config.toml` `[target.x86_64-pc-windows-msvc] rustflags` | Cargo natively supports per-target rustflags; a build.rs approach re-fires on every rebuild and is harder to audit |
| Non-fatal WiX service | Custom action that wraps service start | `vital="no"` on `ServiceControl` + `ErrorControl="ignore"` on `ServiceInstall` | WiX v4 models this natively; custom actions add complexity and can themselves fail |
| msiexec async wait | Sleep loops | `Start-Process -Wait` or `msiexec` called synchronously (it is inherently synchronous when not passed `/norestart /passive` — see pitfall section) | Correct idiom is documented |
| PATH propagation proof | Modifying `$env:PATH` in the current session | `pwsh -NoProfile -Command "nono --version"` in a new child process | Only a new process sees the MSI's `Environment` PATH change (system PATH is read at process startup) |

**Key insight:** WiX v4's `vital="no"` attribute is specifically designed for "start this service but don't roll back if it fails" — this is the canonical mechanism, not a workaround.

---

## Key Research Findings (the four open questions)

### Q1: Exact `+crt-static` Wiring

**Finding:** No `.cargo/config.toml` exists in the repo today. [VERIFIED: repo scan — no `.cargo/config.toml` or `.cargo/config` found]

**Correct mechanism:** Create `.cargo/config.toml` at the workspace root with:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

**Why this is correct:**
- `[target.x86_64-pc-windows-msvc]` scopes exclusively to the Windows MSVC target. Linux (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`) and macOS (`x86_64-apple-darwin`, `aarch64-apple-darwin`) targets are completely unaffected. [ASSUMED — based on Cargo documentation semantics]
- The existing CI `RUSTFLAGS: -Dwarnings` is set as an environment variable in `ci.yml:14`. Cargo merges environment-variable `RUSTFLAGS` and config-file `rustflags` on the SAME target; on the Windows MSVC target the effective flags will be `-Dwarnings -C target-feature=+crt-static`. They do not conflict. [ASSUMED — based on Cargo flag merging semantics]
- The `[build] rustflags` alternative would apply to ALL targets including Linux/macOS, changing the ABI on those platforms. This must NOT be used.
- A `RUSTFLAGS` env var in `scripts/build-windows-msi.ps1` would only affect that script's invocations, not the release.yml build invocations. The `.cargo/config.toml` approach applies uniformly to all Windows MSVC builds including release.yml.

**Which binaries are affected:**
All four binaries built from the `nono-cli` crate when targeting `x86_64-pc-windows-msvc`:
- `nono.exe` (the main CLI, `[[bin]] name = "nono"`)
- `nono-agentd.exe` (`[[bin]] name = "nono-agentd"`, `src/bin/nono-agentd.rs`)
- `nono-wfp-service.exe` (auto-discovered from `src/bin/nono-wfp-service.rs`)
- `nono-shell-broker.exe` (separate crate `crates/nono-shell-broker`, built separately in CI and release.yml)

The `.cargo/config.toml` stanza applies workspace-wide, so `nono-shell-broker` also picks it up when built for `x86_64-pc-windows-msvc`. This is correct — the broker is also shipped in the MSI.

**Interaction with `panic = "abort"` in `[profile.release]`:** The workspace `Cargo.toml` already sets `panic = "abort"` in `[profile.release]`. This is compatible with `+crt-static`; no conflict. [ASSUMED]

**Static-CRT proof:** After building with `+crt-static`, the absence of `vcruntime140.dll` in the PE import table can be verified on the dev host via:
```powershell
# Using dumpbin (MSVC toolchain — available in VS Developer Command Prompt):
dumpbin /imports target\release\nono.exe | Select-String vcruntime
# Expected: no output (empty = static CRT confirmed)

# Using link /dump (same tool, different name):
link /dump /imports target\release\nono.exe | Select-String vcruntime
```
This can be added as an optional `detail` field in the gate verdict, or as a pre-build validation step. [ASSUMED — toolchain availability on clean VM not confirmed]

**Cross-target clippy impact:** The `+crt-static` flag applies only when compiling for `x86_64-pc-windows-msvc`. The CI Clippy jobs run on `ubuntu-latest` and `macos-latest` (targeting Linux/macOS), so the `.cargo/config.toml` stanza is invisible to them. No cross-target clippy concern. The `scripts/build-windows-msi.ps1` and `.cargo/config.toml` edits contain no cfg-gated Rust code, so the cross-target clippy CLAUDE.md requirement is not triggered. [VERIFIED: CI configuration]

---

### Q2: Exact WiX Non-Fatal Service Mechanism

**Current state (from `scripts/build-windows-msi.ps1` lines 226-246):**

```xml
<ServiceInstall
    Id="svcWfpService"
    Name="nono-wfp-service"
    DisplayName="nono WFP Service"
    Description="nono Windows Filtering Platform backend service"
    Type="ownProcess"
    Start="auto"
    Account="LocalSystem"
    ErrorControl="normal"
    Arguments="--service-mode" />
<ServiceControl
    Id="svcCtrlWfpService"
    Name="nono-wfp-service"
    Start="install"
    Stop="both"
    Remove="uninstall"
    Wait="yes" />
```

**What goes wrong today:** With `Wait="yes"` and no `vital` attribute, a service start timeout/failure causes the WiX installer to roll back the entire product (exit 1603). The `ErrorControl="normal"` on `ServiceInstall` tells SCM to log the error but still try the service — this does NOT make the MSI non-fatal.

**The fix:**

WiX v4 (the schema this project uses: `xmlns="http://wixtoolset.org/schemas/v4/wxs"`) uses `vital` (lowercase) on `ServiceControl`. [ASSUMED — WiX v4 schema attribute names; verified against WiX project documentation patterns in the codebase]

Change 1 — `ServiceControl`: add `vital="no"` so a start failure does not roll back the MSI:
```xml
<ServiceControl
    Id="svcCtrlWfpService"
    Name="nono-wfp-service"
    Start="install"
    Stop="both"
    Remove="uninstall"
    Wait="yes"
    vital="no" />
```

Change 2 — `ServiceInstall`: change `ErrorControl="normal"` to `ErrorControl="ignore"` so SCM does not pop an error dialog or block boot on start failure (belt-and-suspenders with D-04):
```xml
<ServiceInstall
    Id="svcWfpService"
    Name="nono-wfp-service"
    DisplayName="nono WFP Service"
    Description="nono Windows Filtering Platform backend service"
    Type="ownProcess"
    Start="auto"
    Account="LocalSystem"
    ErrorControl="ignore"
    Arguments="--service-mode" />
```

**Important:** `Start="install"` on `ServiceControl` should remain. The intent is "try to start the service, but don't roll back if it fails." Removing `Start="install"` would mean the service is never started at install time (different behavior). `vital="no"` is the correct mechanism to preserve the attempt while eliminating the rollback.

**Impact on `validate-windows-msi-contract.ps1`:** The existing contract validation script (`scripts/validate-windows-msi-contract.ps1`) asserts `ServiceInstall.Start = "auto"` (line 201) and `ServiceControl.Stop = "both"` / `ServiceControl.Remove = "uninstall"` / `ServiceControl.Wait = "yes"` (lines 210-215). It does NOT currently assert `ErrorControl` or `vital`. A new assertion `Assert-Equal -Actual $machineServiceControl.vital -Expected "no"` should be added to the service block (to lock the non-fatal contract going forward). The change to `ErrorControl="ignore"` also deserves a corresponding assertion.

**Location in `build-windows-msi.ps1`:** The `$serviceComponentXml` here-string is assembled at lines 226-247. Both attributes to change are in that block. The `.wxs` file is generated at line 404 via `Write-Utf8NoBomCompat`; edit only the PowerShell source.

---

### Q3: Gate Contract Details

**From reading `scripts/verify-dark.ps1` (Phase 76, fully implemented):**

The gate contract for `scripts/gates/clean-host-install.ps1` is:

**Two-function export (dot-sourced by runner):**
```powershell
function Test-Precondition {
    # Returns: $null (preconditions met, run Invoke-Gate)
    #          "reason string" (SKIP_HOST_UNAVAILABLE — Invoke-Gate never runs)
    # Must NOT throw (a throw is caught by the runner as harness-internal error / exit 4)
}

function Invoke-Gate {
    # Returns: [ordered]@{ gate; verdict; reason; detail; timestamp }
    #          verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
    # Must NOT call exit — only the runner owns exit mapping
    # Must NOT call Persist-Verdict — only the runner writes .nono-runtime/verdicts/
    # A throw here = harness-internal error (exit 4), never a silent PASS
}
```

**Verdict object shape (D-01, key order locked):**
```powershell
[ordered]@{
    gate      = 'clean-host-install'
    verdict   = 'PASS'            # or 'FAIL' or 'SKIP_HOST_UNAVAILABLE'
    reason    = 'short human string'
    detail    = [ordered]@{ ... } # free-form debugging context
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}
```

**Exit mapping (owned by runner, not gate):** `PASS=0`, `FAIL=2`, `SKIP_HOST_UNAVAILABLE=3`, harness-internal=4.

**SKIP_HOST_UNAVAILABLE detection for D-02 ("clean" = no prior nono):**

`Test-Precondition` should probe two conditions (either triggers a SKIP, not a FAIL):

1. `nono.exe` under `C:\Program Files\nono\`:
```powershell
$nonoInstalled = Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe'
```

2. Either `nono-wfp-service` or `nono-agentd` registered as a Windows service:
```powershell
$wfpSvc    = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
$agentSvc  = Get-Service 'nono-agentd' -ErrorAction SilentlyContinue
$svcExists = ($null -ne $wfpSvc) -or ($null -ne $agentSvc)
```

If either is true, return a reason string. If both are clean, return `$null`.

**Elevation check:** Machine-scope MSI (`perMachine`) requires administrator privileges for `msiexec /i`. `Test-Precondition` should also check elevation and return SKIP if not elevated (same pattern as `wfp-egress-isolation.ps1` lines 112-116):
```powershell
$principal = New-Object System.Security.Principal.WindowsPrincipal(
    [System.Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    return 'clean-host-install gate requires elevation (msiexec perMachine install needs admin) - re-run from an elevated shell'
}
```

**`-MsiPath` parameter threading (D-07):** The gate is dot-sourced by the runner, which does not pass parameters to the gate functions. The gate file should declare the MSI path as a script-scope default at the top:
```powershell
param(
    [string]$MsiPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist\windows\nono-machine.msi')
)
```
This is consistent with `wfp-egress-isolation.ps1`'s `$script:ProfileBlocked` / `$script:ProfileOpen` configuration block at the top of the file. The operator can override by dot-sourcing with a parameter or by setting `$MsiPath` before invoking.

However, the runner dot-sources the gate file as `. $gateFile` without parameters. Therefore `-MsiPath` must be threaded differently — either as a script-scope variable set before dot-sourcing (operator pattern), or more simply: the gate defaults to the well-known path, and the operator stages the MSI there. **Simplest conforming approach:** make `$MsiPath` a script-level variable with default, resolved relative to the repo root (the gate knows `$PSScriptRoot` = `scripts/gates/`, so repo root = `Split-Path -Parent $PSScriptRoot`). The runner cannot pass `-MsiPath` because it dot-sources without arguments. The operator who wants a non-default path must set `$MsiPath = '...'` in their shell before running `verify-dark.ps1`.

---

### Q4: msiexec Orchestration Idiom

**msiexec async behavior:** `msiexec.exe` on Windows returns immediately if called via `Start-Process` without `-Wait`, spawning a background instance. When invoked synchronously (via `& msiexec /i <msi> /quiet`), it IS synchronous — the blocking `&` call waits for completion. However, the preferred pattern for capturing exit code reliably (given `$ErrorActionPreference="Stop"` + `$PSNativeCommandUseErrorActionPreference=$false`) is `Start-Process -FilePath msiexec -ArgumentList ... -Wait -PassThru`, then read `$process.ExitCode`. This mirrors `Invoke-LoggedCargo` in `scripts/windows-test-harness.ps1`. [ASSUMED — msiexec behavior; widely documented]

**Recommended msiexec install call:**
```powershell
$installArgs = @('/i', $MsiPath, '/quiet', '/norestart', '/l*v', $logPath)
$proc = Start-Process -FilePath 'msiexec.exe' `
    -ArgumentList $installArgs `
    -Wait -PassThru -NoNewWindow
$installExit = $proc.ExitCode
```

**Exit code meanings:**
- `0` = success
- `3010` = success, reboot required (should be treated as PASS for this gate — the gate records it in `detail.rebootRequired = $true`)
- `1603` = fatal error (the old clean-host failure mode — should map to FAIL with the 1603 exit code in detail)
- Any other non-zero = FAIL

**`nono --version` from a NEW session (PATH propagation):**

The MSI's `<Environment Id="EnvPath" ... System="yes">` sets the SYSTEM `PATH`. The current PowerShell session will NOT see this change (environment is inherited at process start). A new child process reads the updated system PATH from the registry. The correct idiom:

```powershell
# Launch a fresh pwsh to see the installed PATH
$versionProc = Start-Process -FilePath 'pwsh.exe' `
    -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', 'nono --version') `
    -Wait -PassThru -NoNewWindow `
    -RedirectStandardOutput $stdoutTmp `
    -RedirectStandardError  $stderrTmp
$versionExit = $versionProc.ExitCode
$versionOut  = (Get-Content $stdoutTmp -Raw -ErrorAction SilentlyContinue) + `
               (Get-Content $stderrTmp -Raw -ErrorAction SilentlyContinue)
```

Alternatively, launch `cmd /c nono --version` — cmd also gets the refreshed PATH. The `pwsh -NoProfile` approach is cleaner and consistent with the project's PowerShell idiom.

**msiexec uninstall (cleanup for D-06 repeatability):**

```powershell
$uninstallArgs = @('/x', $MsiPath, '/quiet', '/norestart')
$unProc = Start-Process -FilePath 'msiexec.exe' `
    -ArgumentList $uninstallArgs `
    -Wait -PassThru -NoNewWindow
$uninstallExit = $unProc.ExitCode
```

Uninstall exit code: `0` = success; `3010` = success with reboot required. A non-zero uninstall exit should be recorded in `detail` but should NOT flip a PASS verdict to FAIL — the product was verified; the cleanup failure is an operational note.

**`$ErrorActionPreference` inside Invoke-Gate:** Following the pattern established in `wfp-egress-isolation.ps1` (line 152) and `copilot-e2e.ps1` (line 168), set `$ErrorActionPreference = 'Continue'` inside `Invoke-Gate` so that native-tool stderr (msiexec progress messages) does not promote to a terminating error. The runner's outer `$ErrorActionPreference = "Stop"` remains unchanged.

---

## Common Pitfalls

### Pitfall 1: `[build] rustflags` instead of `[target.X] rustflags`

**What goes wrong:** `[build] rustflags = ["-C", "target-feature=+crt-static"]` in `.cargo/config.toml` applies to ALL targets. On Linux, `+crt-static` changes glibc linkage behavior (produces a musl-like static binary that cannot use glibc). On macOS, it is not meaningful but may trigger warnings. The CI Clippy/Test jobs on Linux/macOS would be affected.

**Why it happens:** Developers assume "static CRT" is a Windows concept, so they put it in `[build]`. But Cargo's `[build]` section is truly global.

**How to avoid:** Always use `[target.x86_64-pc-windows-msvc]`. Confirm by running `cargo build -p nono-cli` on the Windows dev host and checking `dumpbin /imports`.

**Warning signs:** Clippy failures on Linux after the `.cargo/config.toml` change; unexpected linker flags in non-Windows CI output.

---

### Pitfall 2: WiX v3 vs WiX v4 `vital` attribute casing

**What goes wrong:** WiX v3 used `Vital="no"` (capital V) on `ServiceControl`. WiX v4 (`xmlns="http://wixtoolset.org/schemas/v4/wxs"`) uses `vital="no"` (lowercase v). The project uses WiX v7 (`wix build ... -acceptEula wix7`) which enforces the v4 schema. Using `Vital="no"` in v4 will produce a WiX compile error or silently be ignored. [ASSUMED — WiX v3/v4 schema migration; not verified via WiX docs]

**How to avoid:** Use `vital="no"` (lowercase). Verify by running the build-windows-msi.ps1 script after the change and confirming the `.wxs` emits `vital="no"`.

---

### Pitfall 3: msiexec returning before completion (async spawn)

**What goes wrong:** On some Windows configurations, `& msiexec.exe /i <msi> /quiet` returns exit 0 immediately while msiexec runs in the background. `$LASTEXITCODE` captures the launcher's exit, not the installer's.

**Why it happens:** msiexec.exe on Windows is a GUI application; when launched from some contexts it spawns a worker process and exits the launcher.

**How to avoid:** Use `Start-Process -FilePath msiexec.exe -ArgumentList $args -Wait -PassThru` and read `$process.ExitCode`. This ensures the installer runs to completion before the gate reads the exit code. Do NOT use `$LASTEXITCODE` directly after a bare `& msiexec` call.

**Warning signs:** Gate reports PASS immediately on a host that clearly did not complete the install; the `nono --version` step then fails.

---

### Pitfall 4: PATH propagation — reading from current session

**What goes wrong:** After `msiexec /i` completes, `& nono --version` in the SAME PowerShell session returns "nono: not recognized" because the system PATH was updated in the registry but the current process's environment block still has the old PATH.

**How to avoid:** Always launch a NEW process: `Start-Process pwsh -ArgumentList '-NoProfile', '-Command', 'nono --version' -Wait -PassThru`. The child process starts with the updated system PATH.

**Warning signs:** `nono --version` fails in the gate even though nono is clearly installed (visible in Programs and Features).

---

### Pitfall 5: `vital="no"` not adding an assertion to `validate-windows-msi-contract.ps1`

**What goes wrong:** The `vital="no"` attribute is added to `build-windows-msi.ps1` but not asserted in `validate-windows-msi-contract.ps1`. A future refactor silently removes it; the contract validation still passes; the next release ships a fatal-service MSI again.

**How to avoid:** Add `Assert-Equal -Actual $machineServiceControl.vital -Expected "no" -Message "Machine MSI ServiceControl vital mismatch (must be non-fatal)"` to the service block in `validate-windows-msi-contract.ps1`.

---

### Pitfall 6: WR-01 — stray pipeline output from Invoke-Gate

**What goes wrong:** `Invoke-Gate` accidentally emits pipeline output (e.g. from a `Start-Process` that returns a Process object not captured to a variable). The runner's `Normalize-VerdictObject` collapses a stray-array to its last element, so unintended pipeline output prepended before the verdict dict causes `$VerdictObj[-1]` to be the correct dict — but this is fragile.

**How to avoid:** Pipe or redirect any process-launching side effects to `| Out-Null` or `$null = ...`. Assign `Start-Process ... -PassThru` to a variable. The pattern from `copilot-e2e.ps1` is the correct reference.

---

## Code Examples

### `+crt-static` in `.cargo/config.toml`

```toml
# Source: Cargo documentation (per-target rustflags is the safe scoping mechanism)
# Apply to: Windows MSVC target only. Does NOT affect Linux/macOS builds.
# Eliminates vcruntime140.dll dependency (INST-01 / Phase 80 D-03).
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

### WiX `vital="no"` in `scripts/build-windows-msi.ps1`

```powershell
# Source: build-windows-msi.ps1 lines 226-247 (current state → patched state)
# Edit the PowerShell here-string; do NOT edit the generated .wxs directly.
$serviceComponentXml = @"
      <Component Id="cmpWfpServiceExe" Guid="*">
        <File Id="filWfpServiceExe" Source="$serviceBinaryFullPath" KeyPath="yes" />
        <ServiceInstall
            Id="svcWfpService"
            Name="nono-wfp-service"
            DisplayName="nono WFP Service"
            Description="nono Windows Filtering Platform backend service"
            Type="ownProcess"
            Start="auto"
            Account="LocalSystem"
            ErrorControl="ignore"
            Arguments="--service-mode" />
        <ServiceControl
            Id="svcCtrlWfpService"
            Name="nono-wfp-service"
            Start="install"
            Stop="both"
            Remove="uninstall"
            Wait="yes"
            vital="no" />
      </Component>
"@
```

### `Test-Precondition` skeleton for `clean-host-install.ps1`

```powershell
# Source: contract established by scripts/gates/harness-self-check.ps1 (Phase 76)
# and scripts/gates/wfp-egress-isolation.ps1 (Phase 79).
function Test-Precondition {
    # D-02: SKIP if host has a prior nono installation (nono.exe present or services registered).
    # D-01: Operator must run this on a deliberately fresh VM.

    # Elevation check: machine MSI requires admin.
    $principal = New-Object System.Security.Principal.WindowsPrincipal(
        [System.Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return 'clean-host-install gate requires elevation (machine MSI install needs admin) - re-run from an elevated shell'
    }

    # MSI artifact must exist on this host (operator responsibility, D-07).
    if (-not (Test-Path -LiteralPath $script:MsiPath)) {
        return "MSI not found at $($script:MsiPath) - stage dist\windows\nono-machine.msi on this VM before running the gate"
    }

    # D-02: detect a dirty host (prior nono install or registered services).
    $nonoExePath = 'C:\Program Files\nono\nono.exe'
    if (Test-Path -LiteralPath $nonoExePath) {
        return 'nono.exe detected under C:\Program Files\nono — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }
    $wfpSvc   = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
    $agentSvc = Get-Service 'nono-agentd'      -ErrorAction SilentlyContinue
    if ($null -ne $wfpSvc -or $null -ne $agentSvc) {
        return 'nono service(s) already registered (nono-wfp-service or nono-agentd) — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }

    return $null  # All clear — Invoke-Gate will run.
}
```

### `Invoke-Gate` skeleton for `clean-host-install.ps1`

```powershell
# Source: pattern from scripts/gates/copilot-e2e.ps1 (ProcessStartInfo/Start-Process idiom)
# and scripts/windows-test-harness.ps1 (Invoke-LoggedCargo exit-code check).
function Invoke-Gate {
    # Must NOT call exit. Must NOT call Persist-Verdict.
    # Returns exactly one [ordered]@{gate;verdict;reason;detail;timestamp}.

    $ErrorActionPreference = 'Continue'  # Native stderr must not throw (mirrors wfp-egress-isolation.ps1:152)

    $stamp = { Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ' }

    # --- 1. Install ---
    $installLogPath = Join-Path $env:TEMP 'nono-gate-install.log'
    $installArgs = @('/i', $script:MsiPath, '/quiet', '/norestart', "/l*v", $installLogPath)
    $installProc = Start-Process -FilePath 'msiexec.exe' `
        -ArgumentList $installArgs -Wait -PassThru -NoNewWindow
    $installExit = $installProc.ExitCode

    # 3010 = success with reboot required; treat as success for gate purposes.
    $installOk = ($installExit -eq 0 -or $installExit -eq 3010)

    if (-not $installOk) {
        return [ordered]@{
            gate      = 'clean-host-install'
            verdict   = 'FAIL'
            reason    = "msiexec install failed (exit $installExit) — MSI rolled back; check $installLogPath"
            detail    = [ordered]@{ installExitCode = $installExit; installLog = $installLogPath }
            timestamp = & $stamp
        }
    }

    # --- 2. nono --version from a NEW session (PATH propagation proof) ---
    $versionStdout = Join-Path $env:TEMP 'nono-gate-version.stdout.tmp'
    $versionStderr = Join-Path $env:TEMP 'nono-gate-version.stderr.tmp'
    $versionProc = Start-Process -FilePath 'pwsh.exe' `
        -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', 'nono --version') `
        -Wait -PassThru -NoNewWindow `
        -RedirectStandardOutput $versionStdout `
        -RedirectStandardError  $versionStderr
    $versionExit = $versionProc.ExitCode
    $versionOut  = (Get-Content $versionStdout -Raw -ErrorAction SilentlyContinue) +
                   (Get-Content $versionStderr -Raw -ErrorAction SilentlyContinue)
    Remove-Item $versionStdout, $versionStderr -Force -ErrorAction SilentlyContinue

    # --- 3. Service state (non-fatal; D-06 records state in detail) ---
    $wfpSvcAfter = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
    $wfpSvcState = if ($wfpSvcAfter) { $wfpSvcAfter.Status.ToString() } else { 'not-registered' }

    # --- 4. Uninstall (cleanup for repeatability, D-06) ---
    $uninstallArgs = @('/x', $script:MsiPath, '/quiet', '/norestart')
    $unProc = Start-Process -FilePath 'msiexec.exe' `
        -ArgumentList $uninstallArgs -Wait -PassThru -NoNewWindow
    $uninstallExit = $unProc.ExitCode

    $detail = [ordered]@{
        installExitCode   = $installExit
        rebootRequired    = ($installExit -eq 3010)
        versionOutput     = $versionOut.Trim()
        versionExitCode   = $versionExit
        wfpServiceState   = $wfpSvcState
        uninstallExitCode = $uninstallExit
        msiPath           = $script:MsiPath
    }

    if ($versionExit -ne 0 -or [string]::IsNullOrWhiteSpace($versionOut)) {
        return [ordered]@{
            gate      = 'clean-host-install'
            verdict   = 'FAIL'
            reason    = "`nono --version` failed (exit $versionExit) after install — PATH not propagated or binary does not load"
            detail    = $detail
            timestamp = & $stamp
        }
    }

    return [ordered]@{
        gate      = 'clean-host-install'
        verdict   = 'PASS'
        reason    = 'machine MSI installed, nono --version succeeded in a new session, uninstalled cleanly'
        detail    = $detail
        timestamp = & $stamp
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Dynamic CRT (default MSVC) | Static CRT (`+crt-static`) | Phase 80 | Eliminates VC++ redist dependency; no 0xC0000135 on clean hosts |
| Fatal service start (no `vital`) | `vital="no"` on ServiceControl | Phase 80 | Service failure never rolls back the MSI |
| Human UAT for clean-host install | `scripts/gates/clean-host-install.ps1` | Phase 80 | Unattended; machine-readable PASS/FAIL/SKIP verdict |

**Deprecated/outdated:**
- Bundling `vc_redist.x64.exe` as a chained installer: rejected in favor of `+crt-static` (D-03). A chained installer adds payload size, requires a separate binary download at build time, and still fails if the redist install itself fails.
- `WFP-service-as-fatal-for-install`: the current `vital`-absent behavior is the defect being fixed.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `[target.x86_64-pc-windows-msvc] rustflags` in `.cargo/config.toml` does not affect Linux/macOS targets | Q1: `+crt-static` Wiring | Low: this is the documented Cargo behavior; any regression would appear immediately as unexpected CI flags |
| A2 | CI `RUSTFLAGS: -Dwarnings` (env var) merges additively with `.cargo/config.toml` `rustflags` (no conflict) | Q1: `+crt-static` Wiring | Medium: if Cargo overrides rather than merges, the `+crt-static` flag would be dropped on CI; the static-CRT proof would then only exist on local builds |
| A3 | WiX v4 schema uses `vital="no"` (lowercase) on `ServiceControl` | Q2: WiX non-fatal service | Medium: if WiX v7's schema accepts `Vital="no"` (v3 casing), both would work, but only one would be schema-valid; test by running `wix build` after the change |
| A4 | `+crt-static` is compatible with `panic = "abort"` in `[profile.release]` | Q1: `+crt-static` Wiring | Low: both are independent linker/codegen flags; no known conflict |
| A5 | `msiexec.exe` invoked via `Start-Process -Wait` is fully synchronous (installer completes before the process object's ExitCode is readable) | Q4: msiexec orchestration | Low: this is the documented behavior; the alternative `& msiexec` may be async in some contexts |
| A6 | `dumpbin /imports` is available on the clean VM to provide the optional static-CRT import proof | Q1: `+crt-static` Wiring (optional dumpbin check) | Low: it is a MSVC toolchain tool and would not be on a clean Win11 host; the clean-host install success is sufficient evidence per D-06 |

**If this table is empty for any entry:** The planner should confirm A2 and A3 empirically during implementation before marking the plan complete.

---

## Open Questions

1. **Cargo rustflags merge order (A2 above)**
   - What we know: Cargo documentation says environment-variable `RUSTFLAGS` and config-file `rustflags` for the same target are merged; env-var takes precedence for conflicting flags.
   - What's unclear: The existing CI `RUSTFLAGS: -Dwarnings` is set globally. The config-file adds `-C target-feature=+crt-static` on Windows MSVC. These are non-conflicting flags, so merge should be additive — but should be confirmed by inspecting the cargo verbose output on a Windows CI run.
   - Recommendation: Add a `cargo build --verbose` step or check `cargo rustc -- --print cfg` to confirm effective rustflags on the first Windows CI run after the change.

2. **`vital="no"` attribute name in WiX v4/v7 (A3 above)**
   - What we know: WiX v3 used `Vital` (capitalized). WiX v4 schema changed attribute naming conventions. The project uses `xmlns="http://wixtoolset.org/schemas/v4/wxs"` and `wix build ... -acceptEula wix7`.
   - What's unclear: Whether WiX 7 enforces lowercase `vital` or accepts both casings.
   - Recommendation: After editing `build-windows-msi.ps1`, run `.\scripts\build-windows-msi.ps1 -VersionTag v0.0.0-test -BinaryPath ... -BrokerPath ... -EmitOnly` and verify the generated `.wxs` contains `vital="no"`, then attempt `wix build` to confirm no schema error.

3. **nono-wfp-service build command in CI (`windows-packaging` job)**
   - What we know: The `windows-packaging` CI job (ci.yml:334-338) builds only `nono-cli` and `nono-shell-broker` explicitly. `nono-wfp-service` is a `[[bin]]` auto-discovered within `nono-cli`; it is built as part of `cargo build --release -p nono-cli`.
   - What's unclear: Whether a separate `cargo build --release -p nono-cli --bin nono-wfp-service` is needed, or whether the workspace build already produces all bins.
   - Recommendation: Confirm by checking `target\release\nono-wfp-service.exe` existence after `cargo build --release -p nono-cli`. Auto-discovered bins are built unless `[[bin]] default = false` is set (it is not).

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| WiX v7 (`wix` CLI) | `build-windows-msi.ps1` | On CI; operator must install locally | 7.0.0 (pinned in release.yml) | Local build can use `EmitOnly` to validate the .wxs without WiX |
| `dumpbin` / `link /dump` | Optional static-CRT proof in gate `detail` | Available in VS Developer Prompt; NOT on clean VM | MSVC toolchain | Skip the dumpbin check on the clean VM; rely on install success as proof |
| `msiexec.exe` | Gate orchestration | Built-in to Windows 10/11 | System | None needed |
| `pwsh.exe` | Gate PATH propagation proof | Built-in to Windows 11 | 7.x | `powershell.exe` (Windows PowerShell 5.1) — acceptable fallback for `nono --version` |

**Missing dependencies with no fallback:** None — all required tools are system-provided on Windows 11.

---

## Validation Architecture

> `workflow.nyquist_validation` not explicitly set to false in config.json — section included.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in + PowerShell (gate is PS; build change is Rust) |
| Config file | None for gate PS; `Cargo.toml` for Rust |
| Quick run command | `.\scripts\validate-windows-msi-contract.ps1 -BinaryPath ... -BrokerPath ... -ServiceBinaryPath ...` |
| Full suite command | `pwsh scripts/verify-dark.ps1 --gate clean-host-install` (on a clean VM) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INST-01 | MSI installs on clean Win11, `nono --version` succeeds | integration/gate | `pwsh scripts/verify-dark.ps1 --gate clean-host-install` | ❌ Wave 0 (new gate file) |
| INST-01 | MSI contract includes `vital="no"` on ServiceControl | unit/contract | `pwsh scripts/validate-windows-msi-contract.ps1 -BinaryPath ... -BrokerPath ... -ServiceBinaryPath ... -DriverBinaryPath ...` | ✅ (existing; needs new assertion) |
| INST-01 | `+crt-static` does not affect Linux/macOS | compile | CI Clippy/Test on ubuntu-latest + macos-latest (no new test needed; existing CI proves no regression) | ✅ |

### Wave 0 Gaps

- [ ] `scripts/gates/clean-host-install.ps1` — covers INST-01 (new gate file, the main Phase 80 deliverable)
- [ ] New `Assert-Equal` for `vital="no"` in `scripts/validate-windows-msi-contract.ps1` — locks the D-04 contract
- [ ] `.cargo/config.toml` — covers D-03 (new file, static CRT)

---

## Security Domain

> CLAUDE.md `security_enforcement` not set to false — section included.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes (MSI path parameter) | `Test-Path -LiteralPath` (path component check, not string ops); `Resolve-Path` before use |
| V6 Cryptography | no | — |

### Known Threat Patterns for Windows MSI + PowerShell Gate Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| MSI path injection (operator-supplied `-MsiPath`) | Tampering | Use `Test-Path -LiteralPath` (literal, not wildcard); never interpolate `$MsiPath` into a shell string |
| Dirty-host detection bypass (nono installed to non-standard path) | Spoofing | D-02 is scoped to "has nono ever been installed via this MSI" — standard Program Files + service check is sufficient; the SKIP semantics mean a missed dirty host produces a potentially incorrect PASS (risk accepted by D-01 operator-VM model) |
| msiexec log file path injection | Tampering | Use `Join-Path $env:TEMP 'nono-gate-install.log'` (fixed suffix, no operator input in log path) |
| Static-CRT binary replacing dynamic-CRT binary at signtool level | Repudiation | Not a concern for this phase — signing is out of scope (D-05); the gate uses unsigned local builds |

---

## Sources

### Primary (HIGH confidence)

- `scripts/verify-dark.ps1` (read in full) — canonical gate contract, function signatures, exit mapping, verdict shape, WR-01/WR-04 rules
- `scripts/gates/harness-self-check.ps1` (read in full) — reference gate implementation; Test-Precondition/Invoke-Gate exact contract
- `scripts/gates/wfp-egress-isolation.ps1` (read in full) — reference gate with SKIP_HOST_UNAVAILABLE pattern, elevation check, ErrorActionPreference='Continue' inside Invoke-Gate
- `scripts/gates/copilot-e2e.ps1` (read in full) — reference gate with ProcessStartInfo process-launch idiom, detail fields, -MsiPath-equivalent config at top
- `scripts/build-windows-msi.ps1` (read in full) — current ServiceInstall/ServiceControl here-string (lines 226-246), where D-04 change lands
- `scripts/validate-windows-msi-contract.ps1` (read in full) — existing assertions; gap identified (no `vital` assertion)
- `.planning/phases/80-clean-host-install-uat/80-CONTEXT.md` — locked decisions D-01..D-07
- `.planning/phases/76-self-verifying-harness-foundation/76-CONTEXT.md` — gate contract D-01..D-11
- `Cargo.toml` (workspace) — confirmed no existing `[target.*]` rustflags
- `.github/workflows/ci.yml` — confirmed `RUSTFLAGS: -Dwarnings` env var; Windows packaging build commands

### Secondary (MEDIUM confidence)

- `.planning/todos/pending/20260611-msi-vcredist-prereq.md` — the clean-host failure scenario (1603, 0xC0000135, SCM 7009 timeout, rollback) confirmed from UAT findings
- `scripts/windows-test-harness.ps1` — Invoke-LoggedCargo idiom; Start-Process -Wait -PassThru -RedirectStandardOutput/Error pattern
- `.planning/phases/76-self-verifying-harness-foundation/76-PATTERNS.md` — gate contract shape, exit convention, dot-source semantics

### Tertiary (LOW confidence)

- WiX v4 `vital` attribute lowercase naming — inferred from WiX v3→v4 migration conventions; must be confirmed by running `wix build` after the change (A3 above)
- Cargo rustflags merge order (env var + config.toml, same target) — inferred from Cargo reference documentation; confirm empirically on first Windows CI run (A2 above)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all tools are system-provided; build mechanism is well-understood from the codebase
- Gate contract: HIGH — fully implemented in Phase 76; two reference gates read in full
- `+crt-static` wiring: MEDIUM — mechanism is correct; two assumptions (A2, A3) need empirical confirmation on first run
- WiX `vital="no"` wiring: MEDIUM — semantic intent is correct; attribute casing in WiX v4/v7 must be confirmed

**Research date:** 2026-06-17
**Valid until:** 2026-07-17 (stable; WiX and Cargo semantics do not change rapidly)
