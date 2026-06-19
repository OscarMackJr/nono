# Phase 80: Clean-Host Install UAT - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 4 (1 new gate, 1 new config, 2 modifications)
**Analogs found:** 4 / 4

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `scripts/gates/clean-host-install.ps1` | gate/utility | request-response (msiexec orchestration) | `scripts/gates/wfp-egress-isolation.ps1` | exact — same contract, SKIP_HOST_UNAVAILABLE precondition, elevation check; `copilot-e2e.ps1` for Start-Process idiom |
| `.cargo/config.toml` | config | N/A | none (greenfield — no `.cargo/config.toml` exists in repo) | no analog |
| `scripts/build-windows-msi.ps1` | config/script | batch (MSI generation) | self — the existing `$serviceComponentXml` here-string at lines 226-246 | exact — modify in-place |
| `scripts/validate-windows-msi-contract.ps1` | test/utility | request-response (XML assertion) | self — the existing `Assert-Equal` service block at lines 197-215 | exact — extend in-place |

---

## Pattern Assignments

### `scripts/gates/clean-host-install.ps1` (gate, request-response)

**Primary analog:** `scripts/gates/wfp-egress-isolation.ps1`
**Secondary analog:** `scripts/gates/copilot-e2e.ps1` (Start-Process -Wait -PassThru idiom)
**Tertiary analog:** `scripts/gates/harness-self-check.ps1` (minimal contract reference)

---

#### File header comment block pattern
Source: `scripts/gates/wfp-egress-isolation.ps1` lines 1-16 and `scripts/gates/harness-self-check.ps1` lines 1-19

```powershell
# scripts/gates/clean-host-install.ps1
#
# Phase 80 — clean-host-install gate (INST-01)
#
# CONTRACT (mirrors scripts/gates/harness-self-check.ps1, the reference contract for
# phases 77-81): this gate exports exactly two functions dot-sourced by
# scripts/verify-dark.ps1. The gate RETURNS its verdict object — it MUST NOT call exit.
# Only the runner owns exit-code mapping (PASS=0 / FAIL=2 / SKIP_HOST_UNAVAILABLE=3 /
# harness-internal error=4) and the persist-before-emit (WR-04). Do NOT duplicate
# persist/exit logic here.
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE — exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
```

---

#### Gate configuration block (script-scope defaults — MsiPath)
Source: `scripts/gates/wfp-egress-isolation.ps1` lines 49-55 (configuration block pattern); `scripts/gates/copilot-e2e.ps1` lines 39-57

```powershell
# ---------------------------------------------------------------------------
# Gate configuration (D-07: -MsiPath defaults to repo-relative machine MSI)
# ---------------------------------------------------------------------------

# Operator stages this MSI on the fresh VM before running the gate.
# Override by setting $MsiPath before dot-sourcing, or by the script param below.
# The runner dot-sources this file without parameters, so the default is load-bearing.
param(
    [string]$MsiPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist\windows\nono-machine.msi')
)
$script:MsiPath = $MsiPath
```

---

#### Local assertion helper pattern
Source: `scripts/gates/wfp-egress-isolation.ps1` lines 63-73

```powershell
# ---------------------------------------------------------------------------
# Local assertion helper (throw-on-failure per harness-self-check.ps1).
# A throw = harness-internal error (exit 4). Use ONLY for "gate cannot run at all".
# ---------------------------------------------------------------------------

function Assert-True {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$Condition,

        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if (-not $Condition) { throw $Message }
}
```

---

#### Test-Precondition — elevation check pattern
Source: `scripts/gates/wfp-egress-isolation.ps1` lines 106-116 (exact idiom to copy)

```powershell
function Test-Precondition {
    # Elevation check: machine MSI requires admin.
    $identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return 'clean-host-install gate requires elevation (machine MSI install needs admin) - re-run from an elevated shell'
    }

    # ... additional checks below ...
    return $null
}
```

Note: `wfp-egress-isolation.ps1` uses `$identity` + `$principal` as two separate variables (lines 112-115). Copy that exact two-line form — not the single-expression form shown in RESEARCH.md — to stay consistent with the established codebase style.

---

#### Test-Precondition — D-02 dirty-host detection pattern
Source: `scripts/gates/wfp-egress-isolation.ps1` lines 119-144 (pattern: try-connect + return reason string)
Adapted for: file-system + service checks (not pipe checks)

```powershell
    # D-02: SKIP if host has a prior nono installation.
    if (Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe') {
        return 'nono.exe detected under C:\Program Files\nono — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }
    $wfpSvc   = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
    $agentSvc = Get-Service 'nono-agentd'      -ErrorAction SilentlyContinue
    if ($null -ne $wfpSvc -or $null -ne $agentSvc) {
        return 'nono service(s) already registered (nono-wfp-service or nono-agentd) — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }

    # D-07: MSI artifact must be staged on this VM.
    if (-not (Test-Path -LiteralPath $script:MsiPath)) {
        return "MSI not found at $($script:MsiPath) - stage dist\windows\nono-machine.msi on this VM before running the gate"
    }

    return $null  # All clear.
```

---

#### Invoke-Gate — ErrorActionPreference pattern
Source: `scripts/gates/wfp-egress-isolation.ps1` line 152; `scripts/gates/copilot-e2e.ps1` line 168

```powershell
function Invoke-Gate {
    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'
    # ... remainder of gate body
}
```

---

#### Invoke-Gate — Start-Process -Wait -PassThru msiexec pattern
Source: `scripts/gates/copilot-e2e.ps1` lines 259-295 (ProcessStartInfo form for complex args); RESEARCH.md Q4 documents the simpler `Start-Process` form for msiexec which has simple arg lists

```powershell
    # --- 1. Install (D-06 step 1) ---
    $installLogPath = Join-Path $env:TEMP 'nono-gate-install.log'
    $installArgs    = @('/i', $script:MsiPath, '/quiet', '/norestart', '/l*v', $installLogPath)
    $installProc    = Start-Process -FilePath 'msiexec.exe' `
                          -ArgumentList $installArgs `
                          -Wait -PassThru -NoNewWindow
    $installExit    = $installProc.ExitCode
    # 3010 = success + reboot required; treat as PASS.
    $installOk = ($installExit -eq 0 -or $installExit -eq 3010)
```

**Why `Start-Process -Wait -PassThru` not `& msiexec`:** msiexec is a GUI application; `& msiexec` may return the launcher exit before the installer finishes. `$proc.ExitCode` is the authoritative value (pitfall documented in RESEARCH.md Pitfall 3). This is the same idiom as `Invoke-LoggedCargo` in `scripts/windows-test-harness.ps1`.

---

#### Invoke-Gate — fresh-session PATH propagation pattern
Source: RESEARCH.md Q4 (no existing codebase analog for this exact pattern; copilot-e2e.ps1 uses a different process model)

```powershell
    # --- 2. nono --version from a NEW session (PATH propagation proof, D-06 step 2) ---
    $versionStdout = Join-Path $env:TEMP 'nono-gate-version.stdout.tmp'
    $versionStderr = Join-Path $env:TEMP 'nono-gate-version.stderr.tmp'
    $versionProc   = Start-Process -FilePath 'pwsh.exe' `
                         -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', 'nono --version') `
                         -Wait -PassThru -NoNewWindow `
                         -RedirectStandardOutput $versionStdout `
                         -RedirectStandardError  $versionStderr
    $versionExit = $versionProc.ExitCode
    $versionOut  = (Get-Content $versionStdout -Raw -ErrorAction SilentlyContinue) +
                   (Get-Content $versionStderr -Raw -ErrorAction SilentlyContinue)
    Remove-Item $versionStdout, $versionStderr -Force -ErrorAction SilentlyContinue
```

**Why a new pwsh process:** the current PowerShell session's `$env:PATH` is frozen at session start; the MSI's `<Environment>` change writes to the SYSTEM PATH registry key. Only a fresh child process starts with the updated PATH (pitfall documented in RESEARCH.md Pitfall 4).

---

#### Invoke-Gate — verdict object shape (PASS/FAIL/SKIP)
Source: `scripts/gates/wfp-egress-isolation.ps1` lines 175-200 and 250-277 (exact `[ordered]@{}` key order)

```powershell
    # FAIL — install failed
    return [ordered]@{
        gate      = 'clean-host-install'
        verdict   = 'FAIL'
        reason    = "msiexec install failed (exit $installExit) — MSI rolled back; check $installLogPath"
        detail    = [ordered]@{ installExitCode = $installExit; installLog = $installLogPath }
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }

    # PASS
    return [ordered]@{
        gate      = 'clean-host-install'
        verdict   = 'PASS'
        reason    = 'machine MSI installed, nono --version succeeded in a new session, uninstalled cleanly'
        detail    = $detail
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }
```

Key order `gate; verdict; reason; detail; timestamp` is locked by the runner contract (D-01 of Phase 76). All three existing gates follow this order exactly.

---

#### Invoke-Gate — detail object pattern
Source: `scripts/gates/wfp-egress-isolation.ps1` lines 235-245 (assembled `$detail` before the verdict branch)

```powershell
    $detail = [ordered]@{
        installExitCode   = $installExit
        rebootRequired    = ($installExit -eq 3010)
        versionOutput     = $versionOut.Trim()
        versionExitCode   = $versionExit
        wfpServiceState   = $wfpSvcState       # 'Running'/'Stopped'/'not-registered'
        uninstallExitCode = $uninstallExit
        msiPath           = $script:MsiPath
    }
```

---

#### Invoke-Gate — WFP service state probe (non-fatal per D-06)
Source pattern: `scripts/gates/wfp-egress-isolation.ps1` lines 119-128 (Get-Service with ErrorAction SilentlyContinue)

```powershell
    # --- 3. Service state (non-fatal: D-06 records state in detail, never flips PASS to FAIL) ---
    $wfpSvcAfter = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
    $wfpSvcState = if ($wfpSvcAfter) { $wfpSvcAfter.Status.ToString() } else { 'not-registered' }
```

---

#### Invoke-Gate — msiexec uninstall (cleanup for repeatability, D-06 step 4)
Source: same `Start-Process -Wait -PassThru` pattern as the install step

```powershell
    # --- 4. Uninstall (repeatability, D-06) ---
    $uninstallArgs = @('/x', $script:MsiPath, '/quiet', '/norestart')
    $unProc        = Start-Process -FilePath 'msiexec.exe' `
                         -ArgumentList $uninstallArgs `
                         -Wait -PassThru -NoNewWindow
    $uninstallExit = $unProc.ExitCode
    # Non-zero uninstall is recorded in detail but does NOT flip a PASS verdict to FAIL.
```

---

### `.cargo/config.toml` (config, N/A — greenfield)

**Analog:** none. No `.cargo/config.toml` exists in the repo today (confirmed by RESEARCH.md Q1 scan).

**Pattern source:** Cargo documentation (per-target rustflags is the canonical safe scoping mechanism). RESEARCH.md Q1 provides the exact stanza to use verbatim.

```toml
# Applies to Windows MSVC target ONLY. Does NOT affect Linux or macOS builds.
# Eliminates vcruntime140.dll dependency (INST-01 / Phase 80 D-03).
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

**Critical constraints to carry forward to the plan:**
- Use `[target.x86_64-pc-windows-msvc]` — NOT `[build]` (which is global and would change Linux/macOS ABI; see RESEARCH.md Pitfall 1).
- **A2 RESOLVED:** the `.cargo/config.toml` `[target.<triple>].rustflags` is SILENTLY DROPPED whenever a `RUSTFLAGS` env var is set (Cargo's four flag sources are mutually exclusive, first-match-wins; env beats config). config.toml alone is INSUFFICIENT. Wire `+crt-static` via BOTH the config.toml stanza (covers local + release.yml where no `RUSTFLAGS` env is set) AND an appended `RUSTFLAGS: -Dwarnings -C target-feature=+crt-static` in the Windows-gated compile steps of `ci.yml`/`release.yml`. See RESEARCH.md § Open Questions (RESOLVED) A2.
- **A3 RESOLVED:** add `Vital="no"` (PascalCase) on **`ServiceInstall`**, not `vital` on `ServiceControl` (which has no such attribute). See the `build-windows-msi.ps1` section above and RESEARCH.md A3.

---

### `scripts/build-windows-msi.ps1` (modification — ServiceInstall/ServiceControl here-string)

**Analog:** self — the existing `$serviceComponentXml` here-string at lines 226-246.

**Current state** (lines 226-246, read directly from source):

```powershell
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
            ErrorControl="normal"
            Arguments="--service-mode" />
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

**Target state** (one attribute change, D-04 — A3 RESOLVED):

```powershell
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
            ErrorControl="normal"
            Vital="no"
            Arguments="--service-mode" />
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

**Change summary:**
1. `ServiceInstall` — add `Vital="no"` (PascalCase). Per the WiX v4 XSD, `Vital="no"` on `ServiceInstall` means "failure to install the service will be ignored" — i.e. the overall install does NOT roll back on a service start/install failure. This is the verified, schema-valid mechanism.

**Do NOT change `ErrorControl`** for the rollback goal — `ErrorControl` is the SCM boot-time error level and has ZERO effect on install-time MSI rollback (the earlier `ErrorControl="ignore"` idea was a red herring; leave it `normal`).

**Do NOT add `vital`/`Vital` to `ServiceControl`** — the WiX v4 `ServiceControl` element has NO `Vital` attribute (only `Id`, `Name`, `Start`, `Stop`, `Remove`, `Wait`). Leave `ServiceControl` unchanged.

**Edit the PowerShell source only — never edit `dist/windows/nono-machine.wxs` directly** (it is regenerated at line ~404 of `build-windows-msi.ps1` via `Write-Utf8NoBomCompat`; any direct edit is overwritten on the next build).

**Verify after editing (A3 RESOLVED — casing confirmed):** Attribute NAMES in this WiX v4 markup are PascalCase (`ErrorControl`, `Wait`, `Start`), so `Vital="no"` is correct casing. After editing, run `.\scripts\build-windows-msi.ps1 ... -EmitOnly` and confirm the generated `.wxs` contains `Vital="no"` on the ServiceInstall and that `wix build` accepts it.

> CONTEXT.md D-04's "`vital='no'` posture on the relevant `ServiceInstall`/`ServiceControl`" is **intent-level** wording. The technically-correct WiX v4 mechanism is `Vital="no"` (PascalCase) on **`ServiceInstall`** specifically. See 80-RESEARCH.md § Open Questions (RESOLVED) A3.

---

### `scripts/validate-windows-msi-contract.ps1` (modification — add Vital="no" assertion)

**Analog:** self — the existing `Assert-Equal` service block at lines 197-215.

**Existing service block** (lines 197-215, read directly from source):

```powershell
    $machineServiceInstall = Get-FirstNodeByLocalName -Document $machineDoc -LocalName "ServiceInstall"
    Assert-Equal -Actual $machineServiceInstall.Name -Expected "nono-wfp-service" `
        -Message "Machine MSI ServiceInstall Name mismatch"
    Assert-Equal -Actual $machineServiceInstall.Start -Expected "auto" `
        -Message "Machine MSI ServiceInstall Start mismatch (expected auto/boot-start for out-of-box WFP enforcement)"
    Assert-Equal -Actual $machineServiceInstall.Type -Expected "ownProcess" `
        -Message "Machine MSI ServiceInstall Type mismatch"
    Assert-Equal -Actual $machineServiceInstall.Account -Expected "LocalSystem" `
        -Message "Machine MSI ServiceInstall Account mismatch"

    $machineServiceControl = Get-FirstNodeByLocalName -Document $machineDoc -LocalName "ServiceControl"
    Assert-Equal -Actual $machineServiceControl.Name -Expected "nono-wfp-service" `
        -Message "Machine MSI ServiceControl Name mismatch"
    Assert-Equal -Actual $machineServiceControl.Stop -Expected "both" `
        -Message "Machine MSI ServiceControl Stop mismatch"
    Assert-Equal -Actual $machineServiceControl.Remove -Expected "uninstall" `
        -Message "Machine MSI ServiceControl Remove mismatch"
    Assert-Equal -Actual $machineServiceControl.Wait -Expected "yes" `
        -Message "Machine MSI ServiceControl Wait mismatch"
```

**New assertion to add** (append after line 215, before the user MSI block):

```powershell
    Assert-Equal -Actual $machineServiceInstall.Vital -Expected "no" `
        -Message "Machine MSI ServiceInstall Vital mismatch (must be no so a service install/start failure does not roll back the install)"
```

> Assert `Vital` on **`$machineServiceInstall`** (PascalCase), NOT `vital` on `$machineServiceControl` — `ServiceControl` has no such attribute. Do NOT add an `ErrorControl` assertion for the rollback goal (it is a boot-time SCM setting unrelated to install rollback).

**Assert-Equal signature** (lines 100-115 of validate-windows-msi-contract.ps1 — copy for reference):

```powershell
function Assert-Equal {
    param(
        [Parameter(Mandatory = $true)]
        $Actual,
        [Parameter(Mandatory = $true)]
        $Expected,
        [Parameter(Mandatory = $true)]
        [string]$Message
    )
    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}
```

---

## Shared Patterns

### Gate contract (all gate files)
**Source:** `scripts/gates/harness-self-check.ps1` lines 1-19 (contract comment) + lines 60-126 (two-function body)
**Apply to:** `scripts/gates/clean-host-install.ps1`

Rules that MUST be followed (enforced by the runner):
- Gate MUST NOT call `exit` — only the runner owns exit-code mapping.
- Gate MUST NOT call `Persist-Verdict` — only the runner writes `.nono-runtime/verdicts/`.
- `Test-Precondition` returns `$null` or a reason string (no throw from precondition).
- `Invoke-Gate` returns exactly one `[ordered]@{gate;verdict;reason;detail;timestamp}` object.
- A `throw` inside `Invoke-Gate` = harness-internal error (exit 4), never a silent PASS.
- No bare pipeline output from `Invoke-Gate` (WR-01 — stray output breaks the runner's `Normalize-VerdictObject`; capture all `Start-Process` return values to variables).

### Verdict timestamp format
**Source:** `scripts/gates/harness-self-check.ps1` line 78; `scripts/gates/wfp-egress-isolation.ps1` line 246

```powershell
timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
```

### ErrorActionPreference inside Invoke-Gate
**Source:** `scripts/gates/wfp-egress-isolation.ps1` line 152; `scripts/gates/copilot-e2e.ps1` line 168

```powershell
$ErrorActionPreference = 'Continue'
```

Set at the top of `Invoke-Gate` (not globally) so native tool stderr does not promote to a terminating error, while the runner's outer `Stop` setting is preserved.

### Assert-Equal / Assert-True idiom
**Source:** `scripts/gates/harness-self-check.ps1` lines 25-54; `scripts/validate-windows-msi-contract.ps1` lines 100-129

Throw-on-failure. Used for "gate cannot run" conditions inside `Invoke-Gate`, and for contract assertions in `validate-windows-msi-contract.ps1`. Never use for confinement verdict decisions (those are `return [ordered]@{verdict='FAIL'}`).

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `.cargo/config.toml` | config | N/A | No `.cargo/config.toml` or `.cargo/config` exists in the repo today (greenfield; use RESEARCH.md Q1 pattern verbatim) |

---

## Metadata

**Analog search scope:** `scripts/gates/`, `scripts/build-windows-msi.ps1`, `scripts/validate-windows-msi-contract.ps1`, `.cargo/`
**Files read:** harness-self-check.ps1 (127 lines), wfp-egress-isolation.ps1 (279 lines), copilot-e2e.ps1 (423 lines), build-windows-msi.ps1 lines 220-280, validate-windows-msi-contract.ps1 lines 98-291 (targeted via Grep)
**Pattern extraction date:** 2026-06-17

**Assumptions RESOLVED during planning (see RESEARCH.md § Open Questions (RESOLVED)):**
- **A2 — RESOLVED/FALSIFIED:** Cargo's four rustflags sources are mutually exclusive (first-match-wins); a set `RUSTFLAGS` env var fully overrides config-file `target.<triple>.rustflags`. So `+crt-static` must be wired via BOTH config.toml (local/release, no env) AND `RUSTFLAGS` in the Windows-gated CI/release steps. config.toml alone would be silently dropped on CI.
- **A3 — RESOLVED/FALSIFIED:** the correct WiX v4 mechanism is `Vital="no"` (PascalCase) on `ServiceInstall` (per the official WiX v4 XSD). `ServiceControl` has no `Vital` attribute; lowercase `vital` is invalid. Confirm via `build-windows-msi.ps1 ... -EmitOnly` + `wix build`.
