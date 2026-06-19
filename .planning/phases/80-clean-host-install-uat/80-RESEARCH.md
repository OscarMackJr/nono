# Phase 80: Clean-Host Install UAT - Research

**Researched:** 2026-06-17
**Updated (open-question resolution):** 2026-06-18
**Domain:** Windows MSI packaging (static CRT, WiX non-fatal service) + Phase 76 gate contract
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** SKIP-on-dirty + operator-provided fresh VM. `Test-Precondition` detects whether the current host is clean; if not, returns `SKIP_HOST_UNAVAILABLE`. PASS is only achievable on a fresh Win11 VM/snapshot.
- **D-02:** "Clean" = no prior nono only. Skip if `nono.exe` exists under `C:\Program Files\nono\` OR a `nono-wfp-service` / `nono-agentd` service is registered. Does NOT require VC++ runtime to be absent.
- **D-03:** Static CRT (`+crt-static`). Build all four Windows binaries (nono, nono-shell-broker, nono-wfp-service, nono-agentd) with `target-feature=+crt-static`. Windows-MSVC-only change; must not affect Linux/macOS linkage.
- **D-04:** Non-fatal `nono-wfp-service` start (`vital="no"` posture on the relevant `ServiceInstall`/`ServiceControl`). Both D-03 and D-04 ship: belt-and-suspenders. **Note: D-04's "vital='no' posture" is intent-level wording. The technically-correct WiX mechanism is `Vital="no"` (PascalCase) on `ServiceInstall` — see A3 RESOLVED below. There is no `vital` attribute on `ServiceControl`.**
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
| INST-01 | The machine MSI installs and runs on a clean Win11 host with no manual steps, verified by an unattended clean-host harness. | Static CRT eliminates the VC++ 1603/0xC0000135 failure; `Vital="no"` on `ServiceInstall` eliminates the service-start rollback; the gate script provides the unattended verification harness. |
</phase_requirements>

---

## Summary

Phase 80 has two independent work items that must both ship. The first is a **build fix**: the four Windows binaries built for the machine MSI currently link the MSVC CRT dynamically, which causes `0xC0000135` STATUS_DLL_NOT_FOUND and a `1603` MSI rollback on any host that does not have `vc_redist.x64.exe` installed. The fix is to add `+crt-static` to the Windows MSVC build. **IMPORTANT (A2 resolved):** a `[target.x86_64-pc-windows-msvc] rustflags` entry in `.cargo/config.toml` is SILENTLY DROPPED by Cargo when the `RUSTFLAGS` environment variable is also set — they are mutually exclusive and `RUSTFLAGS` wins. Since CI sets `RUSTFLAGS: -Dwarnings` globally, the correct wiring is to append `-C target-feature=+crt-static` to the `RUSTFLAGS` env var used on Windows CI builds (or to set it only in `release.yml`'s Windows build step via `RUSTFLAGS: -Dwarnings -C target-feature=+crt-static`), not via `.cargo/config.toml` alone.

Separately, the `ServiceControl Start="install"` in the generated WiX makes the first service-start a fatal MSI operation. **IMPORTANT (A3 resolved):** the correct WiX v4 mechanism is `Vital="no"` (PascalCase) on `ServiceInstall` — **not** on `ServiceControl`. The WiX v4 XSD (`ServiceControl.xsd` in wixtoolset/wix) defines only five attributes on `ServiceControl` (`Id`, `Name`, `Start`, `Stop`, `Remove`, `Wait`) with no `Vital`/`vital` at all. `Vital` (PascalCase, `YesNoTypeUnion`) lives on `ServiceInstall` and is explicitly documented: "When set to 'yes' or left unspecified the overall install will fail if this service fails to install. A value of 'no' indicates failure to install the service will be ignored." Additionally, `ErrorControl` on `ServiceInstall` is a boot-time SCM setting (how SCM handles a start failure at boot), NOT an install-time rollback control — it has no effect on whether MSI rolls back.

The second work item is a **new gate file**: `scripts/gates/clean-host-install.ps1`, following the `Test-Precondition` / `Invoke-Gate` contract defined in Phase 76, that orchestrates an unattended `msiexec /i ... /quiet` + fresh-session `nono --version` + `msiexec /x ... /quiet` on a clean host and returns a typed verdict object. On the dev (dirty) host it returns `SKIP_HOST_UNAVAILABLE`; on a fresh VM it returns `PASS`.

**Primary recommendation:** Set `RUSTFLAGS: -Dwarnings -C target-feature=+crt-static` in `release.yml`'s Windows build step (and in `ci.yml`'s `windows-packaging` job). Update `scripts/build-windows-msi.ps1`'s service here-string to add `Vital="no"` on `ServiceInstall` (drop the previously-assumed `vital="no"` on `ServiceControl` — that attribute does not exist on that element). Write `scripts/gates/clean-host-install.ps1` following the `wfp-egress-isolation.ps1` reference implementation.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Static CRT linkage | CI pipeline (RUSTFLAGS env var in Windows build steps) | Build config (.cargo/config.toml CANNOT be used alone — env RUSTFLAGS overrides it) | Cargo rustflags sources are mutually exclusive; env var wins; +crt-static must ride in the same env var as -Dwarnings |
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
| `RUSTFLAGS` env var append (`-C target-feature=+crt-static`) | Cargo built-in | Static CRT linkage on Windows MSVC CI and release builds | The only safe wiring: Cargo's four rustflags sources are mutually exclusive; env RUSTFLAGS overrides config.toml; must be appended to the existing `-Dwarnings` value |
| WiX v4 `Vital="no"` on `ServiceInstall` | v4 schema (WiX 7 on CI) | Make service-install failure non-fatal to MSI | `Vital` (PascalCase, `YesNoTypeUnion`) is the correct attribute on `ServiceInstall`; `ServiceControl` has no `Vital`/`vital` attribute in any WiX version |
| `ServiceInstall ErrorControl="ignore"` | SCM attribute | Tell SCM a start failure at boot is non-fatal | Belt-and-suspenders with `Vital="no"` — SCM won't pop an error dialog or attempt last-known-good restart; has no effect on install-time MSI rollback |
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
| Appending `+crt-static` to CI `RUSTFLAGS` env var | `.cargo/config.toml` `[target.x86_64-pc-windows-msvc]` alone | Config.toml rustflags are SILENTLY DROPPED when `RUSTFLAGS` env var is set — they are mutually exclusive per Cargo docs. Config.toml alone is WRONG when CI sets RUSTFLAGS. |
| Appending `+crt-static` to CI `RUSTFLAGS` env var | `[build] rustflags` in config.toml | `[build]` rustflags applies to ALL targets AND is also overridden by env RUSTFLAGS. Double-wrong. |
| `Vital="no"` on `ServiceInstall` | Removing `Start="install"` from ServiceControl | Removing `Start` means the service is never started at install time, which is a behavior change. `Vital="no"` preserves the start attempt and records the outcome without rolling back. |
| `Vital="no"` on `ServiceInstall` | `Wait="no"` on `ServiceControl` | `Wait="no"` means MSI doesn't wait for the service start to complete — avoids the timeout path but is not the documented "non-fatal" mechanism. `Vital="no"` is the correct semantic. |
| `Vital="no"` on `ServiceInstall` | WiX `<CustomAction>` with `Return="ignore"` wrapping service start | Overly complex; WiX already models this with `Vital` on ServiceInstall. |

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
  build-windows-msi.ps1            # MODIFIED: Vital="no" on ServiceInstall + ErrorControl="ignore"
  validate-windows-msi-contract.ps1 # MODIFIED: add Vital="no" assertion
  gates/
    harness-self-check.ps1         # Phase 76
    copilot-e2e.ps1                # Phase 77
    wfp-egress-isolation.ps1       # Phase 79
    clean-host-install.ps1         # Phase 80 (NEW)
.github/workflows/
  ci.yml                           # MODIFIED: windows-packaging job RUSTFLAGS appended
  release.yml                      # MODIFIED: Windows build step RUSTFLAGS appended
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Static CRT on Windows | Custom build.rs that sets link flags | Append `-C target-feature=+crt-static` to `RUSTFLAGS` env var in Windows CI/release build steps | Cargo natively supports rustflags; env var is the only source that survives when other env vars are already set |
| Non-fatal WiX service | Custom action that wraps service start | `Vital="no"` on `ServiceInstall` (PascalCase; this is the correct element and attribute) | WiX v4 models this natively on ServiceInstall; ServiceControl has no vitality attribute |
| msiexec async wait | Sleep loops | `Start-Process -Wait` or `msiexec` called synchronously (it is inherently synchronous when not passed `/norestart /passive` — see pitfall section) | Correct idiom is documented |
| PATH propagation proof | Modifying `$env:PATH` in the current session | `pwsh -NoProfile -Command "nono --version"` in a new child process | Only a new process sees the MSI's `Environment` PATH change (system PATH is read at process startup) |

**Key insight:** WiX v4's `Vital="no"` attribute on `ServiceInstall` is specifically designed for "install/start this service but don't roll back the MSI if it fails" — this is the canonical mechanism, not a workaround. `ServiceControl` has no vitality attribute in the WiX v4 schema.

---

## Key Research Findings (the four open questions)

### Q1: Exact `+crt-static` Wiring

**Finding:** No `.cargo/config.toml` exists in the repo today. [VERIFIED: repo scan — no `.cargo/config.toml` or `.cargo/config` found]

**RESOLVED (A2):** See Open Questions (RESOLVED) section below for the full ruling. The correct mechanism is NOT `.cargo/config.toml` alone. Cargo's four rustflags sources are mutually exclusive — `RUSTFLAGS` env var wins over config-file entries. Since CI sets `RUSTFLAGS: -Dwarnings` globally (ci.yml line 14), a `[target.x86_64-pc-windows-msvc] rustflags` entry in `.cargo/config.toml` would be silently dropped on ALL CI builds.

**Correct mechanism for CI and release.yml:** Amend the `RUSTFLAGS` env var in each Windows build step:

```yaml
# In ci.yml windows-packaging job and release.yml Windows build step:
env:
  RUSTFLAGS: -Dwarnings -C target-feature=+crt-static
```

This can be set as a step-level `env:` override (only on Windows steps) rather than the global `env:` block, so Linux/macOS steps keep `RUSTFLAGS: -Dwarnings` without `+crt-static`.

**Alternative:** A `.cargo/config.toml` with `[target.x86_64-pc-windows-msvc] rustflags` CAN be added as a developer-experience convenience for local `cargo build` on Windows (where `RUSTFLAGS` is not set by the developer's shell), but it MUST NOT be the sole mechanism relied on for CI correctness.

**Why this is correct:**
- `[target.x86_64-pc-windows-msvc]` scopes exclusively to the Windows MSVC target. Linux/macOS targets are completely unaffected. [VERIFIED: Cargo docs, doc.rust-lang.org/cargo/reference/config.html]
- Cargo's precedence: `CARGO_ENCODED_RUSTFLAGS` > `RUSTFLAGS` env var > `target.<triple>.rustflags` config > `build.rustflags` config — these are mutually exclusive, not cumulative. [VERIFIED: Cargo docs]
- The CI `RUSTFLAGS: -Dwarnings` at the global `env:` level wins over any config.toml entry, silently dropping `+crt-static` from all CI builds if only config.toml is used.
- The `[build] rustflags` alternative would apply to ALL targets including Linux/macOS, changing the ABI on those platforms. This must NOT be used.

**Which binaries are affected:**
All four binaries built from the `nono-cli` crate when targeting `x86_64-pc-windows-msvc`:
- `nono.exe` (the main CLI, `[[bin]] name = "nono"`)
- `nono-agentd.exe` (`[[bin]] name = "nono-agentd"`, `src/bin/nono-agentd.rs`)
- `nono-wfp-service.exe` (auto-discovered from `src/bin/nono-wfp-service.rs` — no explicit `[[bin]]` entry needed)
- `nono-shell-broker.exe` (separate crate `crates/nono-shell-broker`, its Windows build step also needs the amended RUSTFLAGS)

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

**Cross-target clippy impact:** The `+crt-static` flag applies only when compiling for `x86_64-pc-windows-msvc`. The CI Clippy jobs run on `ubuntu-latest` and `macos-latest` (targeting Linux/macOS), so their `RUSTFLAGS: -Dwarnings` (no `+crt-static`) is unchanged. No cross-target clippy concern. The CI yaml edits contain no cfg-gated Rust code, so the cross-target clippy CLAUDE.md requirement is not triggered. [VERIFIED: CI configuration]

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

**What goes wrong today:** With `Wait="yes"` and `Vital` not set (defaults to `yes`) on `ServiceInstall`, a service start timeout/failure causes the WiX installer to roll back the entire product (exit 1603). The `ErrorControl="normal"` on `ServiceInstall` tells SCM how to handle a start failure at BOOT TIME — it does NOT affect install-time MSI rollback in any way.

**RESOLVED (A3):** See Open Questions (RESOLVED) section below for the full ruling.

**The fix — single change, on the correct element:**

Change `ServiceInstall` to add `Vital="no"`:
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
    Arguments="--service-mode"
    Vital="no" />
```

`ServiceControl` remains UNCHANGED — do NOT add any `vital` or `Vital` attribute to it (that attribute does not exist on `ServiceControl` in WiX v4 or any WiX version).

The `ErrorControl="ignore"` change (from `normal`) is belt-and-suspenders at the SCM level: it tells SCM not to display an error dialog and not to attempt last-known-good restart if the service fails to start at system boot. This has NO effect on install-time MSI rollback — that is entirely governed by `Vital="no"` on `ServiceInstall`.

**Important:** `Start="install"` on `ServiceControl` should remain. The intent is "try to start the service, but don't roll back if it fails." `Vital="no"` on `ServiceInstall` is the correct mechanism to preserve the start attempt while eliminating the rollback.

**Impact on `validate-windows-msi-contract.ps1`:** The existing contract validation script (`scripts/validate-windows-msi-contract.ps1`) does NOT currently assert `ErrorControl` or `Vital`. A new assertion `Assert-Equal -Actual $machineServiceInstall.Vital -Expected "no"` should be added to the service block (to lock the non-fatal contract going forward). The change to `ErrorControl="ignore"` also deserves a corresponding assertion.

**Location in `build-windows-msi.ps1`:** The `$serviceComponentXml` here-string is assembled at lines 226-247. Both attributes to change are in the `<ServiceInstall>` tag in that block. The `.wxs` file is generated at line 404 via `Write-Utf8NoBomCompat`; edit only the PowerShell source.

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

### Pitfall 1: `[build] rustflags` instead of `[target.X] rustflags` — or relying on config.toml when CI sets RUSTFLAGS

**What goes wrong:** (a) `[build] rustflags = ["-C", "target-feature=+crt-static"]` in `.cargo/config.toml` applies to ALL targets. On Linux, `+crt-static` changes glibc linkage behavior. On macOS, it may trigger warnings. (b) Even a correct `[target.x86_64-pc-windows-msvc] rustflags` entry in config.toml is SILENTLY DROPPED when the `RUSTFLAGS` environment variable is set — and CI sets `RUSTFLAGS: -Dwarnings` globally. The `+crt-static` flag would never reach the linker on CI.

**Why it happens:** Developers assume Cargo merges rustflags from multiple sources. It does NOT — the four sources (`CARGO_ENCODED_RUSTFLAGS`, `RUSTFLAGS`, `target.<triple>.rustflags`, `build.rustflags`) are mutually exclusive. The first one set wins; the rest are ignored.

**How to avoid:** Set `RUSTFLAGS: -Dwarnings -C target-feature=+crt-static` as a step-level env override in the Windows CI and release.yml build steps. Do NOT rely on config.toml as the sole mechanism when CI sets RUSTFLAGS.

**Warning signs:** `dumpbin /imports target\release\nono.exe | Select-String vcruntime` returns output on a CI-built binary (static CRT not applied). A clean-host install still gets 0xC0000135.

---

### Pitfall 2: Adding `vital="no"` to `ServiceControl` (WRONG ELEMENT — attribute does not exist there)

**What goes wrong:** Adding `vital="no"` (or `Vital="no"`) to `<ServiceControl>` produces a WiX schema validation error ("unexpected attribute 'vital'") because `ServiceControl` has no such attribute in the WiX v4 XSD. [VERIFIED: wixtoolset/wix ServiceControl.xsd — attributes are `Id`, `Name`, `Start`, `Stop`, `Remove`, `Wait` only.]

**Why it happens:** The WiX documentation migration from v3 to v4 is incomplete in some community articles. The `Vital` attribute belongs to `ServiceInstall`, not `ServiceControl`. Some older community guides incorrectly attributed it to `ServiceControl`.

**How to avoid:** Add `Vital="no"` to `<ServiceInstall>`, not `<ServiceControl>`. The PascalCase `Vital` is required — the WiX v4 schema uses PascalCase for attribute names on this element.

**Note on CONTEXT.md D-04 wording:** D-04 describes the goal as "vital='no' posture on the relevant ServiceInstall/ServiceControl." The correct implementation is `Vital="no"` (PascalCase) on `ServiceInstall` only. D-04's lowercase `vital` and the mention of `ServiceControl` are intent-level approximations; the technically-correct WiX v4 mechanism differs.

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

### Pitfall 5: `Vital="no"` not adding an assertion to `validate-windows-msi-contract.ps1`

**What goes wrong:** The `Vital="no"` attribute is added to `build-windows-msi.ps1` but not asserted in `validate-windows-msi-contract.ps1`. A future refactor silently removes it; the contract validation still passes; the next release ships a fatal-service MSI again.

**How to avoid:** Add `Assert-Equal -Actual $machineServiceInstall.Vital -Expected "no" -Message "Machine MSI ServiceInstall Vital mismatch (must be non-fatal)"` to the service block in `validate-windows-msi-contract.ps1`.

---

### Pitfall 6: WR-01 — stray pipeline output from Invoke-Gate

**What goes wrong:** `Invoke-Gate` accidentally emits pipeline output (e.g. from a `Start-Process` that returns a Process object not captured to a variable). The runner's `Normalize-VerdictObject` collapses a stray-array to its last element, so unintended pipeline output prepended before the verdict dict causes `$VerdictObj[-1]` to be the correct dict — but this is fragile.

**How to avoid:** Pipe or redirect any process-launching side effects to `| Out-Null` or `$null = ...`. Assign `Start-Process ... -PassThru` to a variable. The pattern from `copilot-e2e.ps1` is the correct reference.

---

## Code Examples

### `+crt-static` via CI RUSTFLAGS (correct wiring)

```yaml
# Source: Cargo Book (doc.rust-lang.org/cargo/reference/config.html) — rustflags precedence
# Apply to: Windows MSVC CI build steps ONLY. Do NOT set in the global env: block.
# Eliminates vcruntime140.dll dependency (INST-01 / Phase 80 D-03).

# In ci.yml — windows-packaging job, under the cargo build step:
- name: Build (Windows)
  env:
    RUSTFLAGS: -Dwarnings -C target-feature=+crt-static
  run: cargo build --release -p nono-cli -p nono-shell-broker

# In release.yml — Windows build step, same pattern.
```

### WiX `Vital="no"` in `scripts/build-windows-msi.ps1`

```powershell
# Source: wixtoolset/wix ServiceInstall.xsd (github.com/wixtoolset/wix)
# Vital (PascalCase, YesNoTypeUnion) is on ServiceInstall, NOT ServiceControl.
# ServiceControl has no Vital/vital attribute.
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
            Arguments="--service-mode"
            Vital="no" />
        <ServiceControl
            Id="svcCtrlWfpService"
            Name="nono-wfp-service"
            Start="install"
            Stop="both"
            Remove="uninstall"
            Wait="yes" />
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
| Dynamic CRT (default MSVC) | Static CRT (`+crt-static` in CI RUSTFLAGS) | Phase 80 | Eliminates VC++ redist dependency; no 0xC0000135 on clean hosts |
| Fatal service start (Vital defaults to yes on ServiceInstall) | `Vital="no"` on `ServiceInstall` | Phase 80 | Service failure never rolls back the MSI |
| Human UAT for clean-host install | `scripts/gates/clean-host-install.ps1` | Phase 80 | Unattended; machine-readable PASS/FAIL/SKIP verdict |

**Deprecated/outdated:**
- Bundling `vc_redist.x64.exe` as a chained installer: rejected in favor of `+crt-static` (D-03). A chained installer adds payload size, requires a separate binary download at build time, and still fails if the redist install itself fails.
- `WFP-service-as-fatal-for-install`: the current `Vital`-absent (defaults to yes) behavior is the defect being fixed.
- **Falsified assumption:** `vital="no"` (lowercase) on `ServiceControl` — this attribute does not exist on `ServiceControl` in any WiX version. The correct attribute is `Vital="no"` (PascalCase) on `ServiceInstall`.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong | Status |
|---|-------|---------|---------------|--------|
| A1 | `[target.x86_64-pc-windows-msvc] rustflags` in `.cargo/config.toml` does not affect Linux/macOS targets | Q1: `+crt-static` Wiring | Low: this is the documented Cargo behavior; any regression would appear immediately as unexpected CI flags | VERIFIED (Cargo docs) |
| A2 | CI `RUSTFLAGS: -Dwarnings` (env var) merges additively with `.cargo/config.toml` `rustflags` (no conflict) | Q1: `+crt-static` Wiring | RESOLVED — this claim is **FALSE**. Cargo's sources are mutually exclusive; env RUSTFLAGS wins and drops config.toml entries entirely. See A2 RESOLVED below. | RESOLVED — FALSIFIED |
| A3 | WiX v4 schema uses `vital="no"` (lowercase) on `ServiceControl` | Q2: WiX non-fatal service | RESOLVED — this claim is **FALSE** on two counts: (1) the attribute belongs on `ServiceInstall`, not `ServiceControl`; (2) the correct casing is `Vital` (PascalCase). See A3 RESOLVED below. | RESOLVED — FALSIFIED |
| A4 | `+crt-static` is compatible with `panic = "abort"` in `[profile.release]` | Q1: `+crt-static` Wiring | Low: both are independent linker/codegen flags; no known conflict | [ASSUMED] |
| A5 | `msiexec.exe` invoked via `Start-Process -Wait` is fully synchronous (installer completes before the process object's ExitCode is readable) | Q4: msiexec orchestration | Low: this is the documented behavior; the alternative `& msiexec` may be async in some contexts | [ASSUMED] |
| A6 | `dumpbin /imports` is available on the clean VM to provide the optional static-CRT import proof | Q1: `+crt-static` Wiring (optional dumpbin check) | Low: it is a MSVC toolchain tool and would not be on a clean Win11 host; the clean-host install success is sufficient evidence per D-06 | [ASSUMED] |

---

## Open Questions (RESOLVED)

> All three open questions from the original research are resolved below with authoritative sources.

---

### A2 RESOLVED: Cargo rustflags merge order

**Original question:** Does CI `RUSTFLAGS: -Dwarnings` (env var) merge additively with `.cargo/config.toml` `[target.x86_64-pc-windows-msvc] rustflags`?

**RESOLVED: The original assumption was WRONG — they do NOT merge.**

The Cargo Book states verbatim:

> "There are four mutually exclusive sources of extra flags. They are checked in order, with the first one being used:
> 1. `CARGO_ENCODED_RUSTFLAGS` environment variable.
> 2. `RUSTFLAGS` environment variable.
> 3. All matching `target.<triple>.rustflags` and `target.<cfg>.rustflags` config entries joined together.
> 4. `build.rustflags` config value."

Source: [VERIFIED: https://doc.rust-lang.org/cargo/reference/config.html]

**Concrete implication:** CI sets `RUSTFLAGS: -Dwarnings` at the global `env:` level in `ci.yml` (line 14). This means `RUSTFLAGS` env var is present on every job, including the Windows packaging job. Any `[target.x86_64-pc-windows-msvc] rustflags` entry in `.cargo/config.toml` is source 3 — it is NEVER consulted when `RUSTFLAGS` is set (source 2 wins). The `+crt-static` flag would be silently dropped on all CI builds.

**Correct wiring:** Add `-C target-feature=+crt-static` to the `RUSTFLAGS` value on each Windows build step in `ci.yml` and `release.yml` as a step-level `env:` override:
```yaml
env:
  RUSTFLAGS: -Dwarnings -C target-feature=+crt-static
```
This overrides the global `RUSTFLAGS: -Dwarnings` for that step only. Linux/macOS steps inherit the global value unchanged.

**Can `.cargo/config.toml` still be added?** Yes, as a dev convenience for local Windows builds where `RUSTFLAGS` is not set in the developer's shell. But it must NOT be the sole mechanism — it is invisible to CI.

**Impact on plan:** The plan must modify `ci.yml` and `release.yml`, not (only) create `.cargo/config.toml`. The "Recommended Project Structure" now includes CI yaml files as modified artifacts.

---

### A3 RESOLVED: WiX v4 non-fatal service mechanism

**Original question:** Does WiX v4 `ServiceControl` have a `Vital`/`vital` attribute? What is the correct attribute, element, and casing?

**RESOLVED: The original recommendation was WRONG on both the element and the casing.**

**Authoritative source:** WiX v4 XSD schemas from the official wixtoolset/wix repository on GitHub.

**`ServiceControl` — confirmed attribute set** [VERIFIED: github.com/wixtoolset/wix — `src/xsd/wix/ServiceControl.xsd`]:
```
Id, Name, Start (InstallUninstallType), Stop (InstallUninstallType),
Remove (InstallUninstallType), Wait (YesNoTypeUnion)
```
There is NO `Vital` or `vital` attribute on `ServiceControl` in WiX v4. There never was in any WiX version.

**`ServiceInstall` — the correct element** [VERIFIED: github.com/wixtoolset/wix — `src/xsd/wix/ServiceInstall.xsd`]:
```xml
<attribute name="Vital" type="YesNoTypeUnion">
    <annotation>
        <documentation>When set to 'yes' or left unspecified the overall install will
        fail if this service fails to install. A value of 'no' indicates failure to
        install the service will be ignored.</documentation>
    </annotation>
</attribute>
```
The default is `yes` (when omitted). Setting `Vital="no"` makes a service start failure non-fatal to the MSI install.

**Answers to the four sub-questions in the objective:**

1. `ServiceControl` has NO `Vital`/`vital` attribute. `ServiceInstall` has `Vital` (PascalCase, `YesNoTypeUnion`).

2. The correct attribute name is `Vital` (PascalCase), as defined in the WiX v4 XSD. WiX v4 uses PascalCase for attribute names on service elements.

3. `Wait="no"` on `ServiceControl` is NOT the canonical non-fatal mechanism. It means "MSI does not wait for the service start to complete" — which sidesteps the timeout path but does not document the intent. `Vital="no"` on `ServiceInstall` is the explicit, documented mechanism.

4. `ErrorControl` on `ServiceInstall` controls the **SCM's boot-time behavior** (how the Service Control Manager responds to a start failure during system boot — `ignore`/`normal`/`critical`). It has ZERO effect on install-time MSI rollback. Changing `ErrorControl="normal"` to `ErrorControl="ignore"` is belt-and-suspenders at the SCM level but is NOT the rollback-prevention mechanism.

**Exact minimal edit to lines 226-247 of `build-windows-msi.ps1`:**

Add `Vital="no"` to `<ServiceInstall>` (and optionally change `ErrorControl="normal"` to `ErrorControl="ignore"` for belt-and-suspenders). Do NOT touch `<ServiceControl>`.

```xml
<!-- BEFORE -->
<ServiceInstall
    ...
    ErrorControl="normal"
    Arguments="--service-mode" />
<ServiceControl
    ...
    Wait="yes" />

<!-- AFTER -->
<ServiceInstall
    ...
    ErrorControl="ignore"
    Arguments="--service-mode"
    Vital="no" />
<ServiceControl
    ...
    Wait="yes" />
    <!-- No vital/Vital here — that attribute does not exist on ServiceControl -->
```

**Note on CONTEXT.md D-04:** D-04 says "vital='no' posture on the relevant ServiceInstall/ServiceControl." The technically-correct WiX v4 mechanism uses `Vital="no"` (PascalCase) on `ServiceInstall` only. The executor must implement the mechanism that actually prevents rollback, with a code comment explaining the difference from D-04's intent-level wording.

---

### OQ-3 RESOLVED: nono-wfp-service build command

**Original question:** Is `nono-wfp-service` built by `cargo build --release -p nono-cli`?

**RESOLVED: Yes — it is auto-discovered and built automatically.**

`crates/nono-cli/Cargo.toml` has exactly two explicit `[[bin]]` entries:
- `[[bin]] name = "nono" path = "src/main.rs"`
- `[[bin]] name = "nono-agentd" path = "src/bin/nono-agentd.rs"`

`nono-wfp-service` has no explicit `[[bin]]` entry. However, `crates/nono-cli/src/bin/nono-wfp-service.rs` exists (confirmed: `ls crates/nono-cli/src/bin/` shows `nono-agentd.rs`, `nono-wfp-service.rs`, `test-connector.rs`, `windows-net-probe.rs`). Per Cargo's target auto-discovery rules, every `.rs` file directly under `src/bin/` is automatically treated as a binary target with `default-features = true` unless `autobins = false` is set in `[package]` (it is not). Therefore `cargo build --release -p nono-cli` builds ALL four binaries: `nono`, `nono-agentd`, `nono-wfp-service`, `test-connector`, and `windows-net-probe`. No separate `--bin nono-wfp-service` flag is needed.

Source: [VERIFIED: crates/nono-cli/Cargo.toml — no `autobins = false`; `src/bin/nono-wfp-service.rs` exists]

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
| Framework | Rust built-in + PowerShell (gate is PS; build change is CI yaml + Rust) |
| Config file | None for gate PS; `Cargo.toml` for Rust |
| Quick run command | `.\scripts\validate-windows-msi-contract.ps1 -BinaryPath ... -BrokerPath ... -ServiceBinaryPath ...` |
| Full suite command | `pwsh scripts/verify-dark.ps1 --gate clean-host-install` (on a clean VM) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INST-01 | MSI installs on clean Win11, `nono --version` succeeds | integration/gate | `pwsh scripts/verify-dark.ps1 --gate clean-host-install` | ❌ Wave 0 (new gate file) |
| INST-01 | MSI contract includes `Vital="no"` on ServiceInstall | unit/contract | `pwsh scripts/validate-windows-msi-contract.ps1 -BinaryPath ... -BrokerPath ... -ServiceBinaryPath ... -DriverBinaryPath ...` | ✅ (existing; needs new assertion for Vital) |
| INST-01 | `+crt-static` does not affect Linux/macOS | compile | CI Clippy/Test on ubuntu-latest + macos-latest (no new test needed; existing CI proves no regression) | ✅ |

### Wave 0 Gaps

- [ ] `scripts/gates/clean-host-install.ps1` — covers INST-01 (new gate file, the main Phase 80 deliverable)
- [ ] New `Assert-Equal` for `Vital="no"` on `ServiceInstall` in `scripts/validate-windows-msi-contract.ps1` — locks the D-04 contract
- [ ] CI yaml edit: append `-C target-feature=+crt-static` to `RUSTFLAGS` in `ci.yml` windows-packaging step and `release.yml` Windows build step — covers D-03

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
- `scripts/validate-windows-msi-contract.ps1` (read in full) — existing assertions; gap identified (no `Vital` assertion)
- `.planning/phases/80-clean-host-install-uat/80-CONTEXT.md` — locked decisions D-01..D-07
- `.planning/phases/76-self-verifying-harness-foundation/76-CONTEXT.md` — gate contract D-01..D-11
- `Cargo.toml` (workspace) — confirmed no existing `[target.*]` rustflags
- `crates/nono-cli/Cargo.toml` — confirmed `[[bin]]` entries and absence of `autobins = false`
- `crates/nono-cli/src/bin/` (directory listing) — confirmed `nono-wfp-service.rs` present (auto-discovered by Cargo)
- `.github/workflows/ci.yml` — confirmed `RUSTFLAGS: -Dwarnings` env var at global level (line 14); Windows packaging build commands
- **[VERIFIED]** Cargo Book, `build.rustflags` section: "There are four mutually exclusive sources of extra flags. They are checked in order, with the first one being used..." — https://doc.rust-lang.org/cargo/reference/config.html
- **[VERIFIED]** wixtoolset/wix `src/xsd/wix/ServiceControl.xsd` (fetched via GitHub API) — ServiceControl attribute set: `Id`, `Name`, `Start`, `Stop`, `Remove`, `Wait`. No `Vital` attribute.
- **[VERIFIED]** wixtoolset/wix `src/xsd/wix/ServiceInstall.xsd` (fetched via GitHub API) — `Vital` (PascalCase, `YesNoTypeUnion`) on `ServiceInstall`: "When set to 'yes' or left unspecified the overall install will fail if this service fails to install. A value of 'no' indicates failure to install the service will be ignored."

### Secondary (MEDIUM confidence)

- `.planning/todos/pending/20260611-msi-vcredist-prereq.md` — the clean-host failure scenario (1603, 0xC0000135, SCM 7009 timeout, rollback) confirmed from UAT findings
- `scripts/windows-test-harness.ps1` — Invoke-LoggedCargo idiom; Start-Process -Wait -PassThru -RedirectStandardOutput/Error pattern
- `.planning/phases/76-self-verifying-harness-foundation/76-PATTERNS.md` — gate contract shape, exit convention, dot-source semantics
- WiX v3.10.1 documentation (documentation.help) — confirmed ServiceControl attribute list (Id, Name, Start, Stop, Remove, Wait — no Vital), and ServiceInstall Vital attribute description

### Tertiary (LOW confidence)

- (none — A2 and A3 are now VERIFIED via authoritative sources)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all tools are system-provided; build mechanism verified from Cargo official docs and WiX official XSD
- Gate contract: HIGH — fully implemented in Phase 76; two reference gates read in full
- `+crt-static` wiring: HIGH — mechanism now VERIFIED: env RUSTFLAGS wins over config.toml; CI yaml must be amended
- WiX `Vital="no"` wiring: HIGH — attribute confirmed on ServiceInstall (not ServiceControl) via official XSD from wixtoolset/wix repo

**Research date:** 2026-06-17
**Open-question resolution date:** 2026-06-18
**Valid until:** 2026-07-18 (stable; WiX and Cargo semantics do not change rapidly)
