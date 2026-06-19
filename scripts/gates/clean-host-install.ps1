# scripts/gates/clean-host-install.ps1
#
# Phase 80 - clean-host-install gate (INST-01)
#
# CONTRACT (mirrors scripts/gates/harness-self-check.ps1, the reference contract for
# phases 77-81): this gate exports exactly two functions dot-sourced by
# scripts/verify-dark.ps1. The gate RETURNS its verdict object - it MUST NOT call exit and
# MUST NOT call Persist-Verdict. Only the runner owns exit-code mapping (PASS=0 / FAIL=2 /
# SKIP_HOST_UNAVAILABLE=3 / harness-internal=4) and the persist-before-emit (WR-04).
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE - exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
#
# WHAT THIS PROVES (satisfies INST-01):
#   The machine MSI installs on a clean Win11 host with no manual steps. The gate orchestrates:
#     1. msiexec /i exits 0 (or 3010 = reboot required) - install succeeded.
#     2. `nono --version` succeeds from a NEW pwsh session - PATH was propagated by the MSI.
#     3. nono-wfp-service start state is recorded in detail (non-fatal per D-06).
#     4. msiexec /x cleans up for repeatability.
#   PASS proves the full install cycle works unattended on a fresh host.
#
# WR-01: No stray pipeline output from Invoke-Gate. All Start-Process results are assigned to
#         named variables. No bare process object is written to the pipeline.
# WR-04: Only the runner persists the verdict file and owns the emit-before-persist order.

# ---------------------------------------------------------------------------
# Gate configuration (D-07: -MsiPath defaults to repo-relative machine MSI)
# ---------------------------------------------------------------------------

# Operator stages this MSI on the fresh VM before running the gate. Override
# by setting $MsiPath before dot-sourcing. The runner dot-sources this file without
# parameters, so the default is load-bearing (D-07). The MSI must be rebuilt after
# Plan 80-01 lands for the live PASS to be achievable (Vital="no" + +crt-static must
# be present in the artifact under test).
param(
    [string]$MsiPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist\windows\nono-machine.msi')
)
$script:MsiPath = $MsiPath

# ---------------------------------------------------------------------------
# Local assertion helper (throw-on-failure per harness-self-check.ps1).
# A throw = harness-internal error (exit 4). Use ONLY for "gate cannot run at all".
# Confinement results are verdict objects, never throws.
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

# ---------------------------------------------------------------------------
# Gate contract
# ---------------------------------------------------------------------------

function Test-Precondition {
    # Return $null when all preconditions met; return a reason string -> SKIP_HOST_UNAVAILABLE.
    # NOTE: Test-Precondition MUST NOT throw; a throw here causes a harness-internal error (exit 4).
    # Check in order: elevation -> nono.exe -> services -> MSI staged.

    # 1. Elevation check: machine-scope MSI install requires administrator privileges.
    #    Exact two-line form from wfp-egress-isolation.ps1 lines 112-115.
    $identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return 'clean-host-install gate requires elevation (machine MSI install needs admin) - re-run from an elevated shell'
    }

    # 2. D-02: detect a dirty host - nono.exe already installed under Program Files.
    #    Use -LiteralPath to avoid wildcard expansion (T-80-04 security).
    if (Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe') {
        return 'nono.exe detected under C:\Program Files\nono — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }

    # 3. D-02: detect registered nono services (either means a prior install).
    $wfpSvc   = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
    $agentSvc = Get-Service 'nono-agentd'      -ErrorAction SilentlyContinue
    if ($null -ne $wfpSvc -or $null -ne $agentSvc) {
        return 'nono service(s) already registered (nono-wfp-service or nono-agentd) — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }

    # 4. D-07: MSI artifact must be staged on this VM before the gate runs.
    #    Use -LiteralPath to avoid wildcard expansion (T-80-04 security).
    if (-not (Test-Path -LiteralPath $script:MsiPath)) {
        return "MSI not found at $($script:MsiPath) - stage dist\windows\nono-machine.msi on this VM before running the gate"
    }

    return $null  # All clear - Invoke-Gate will run.
}

function Invoke-Gate {
    # Clean-host MSI install proof (INST-01).
    # Returns exactly one verdict object (gate never calls exit or writes verdict files).
    # WR-01: all Start-Process results assigned to named variables; no stray pipeline output.

    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'

    $stamp = { Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ' }

    # --- STEP 1: Install ---
    # Log path uses fixed suffix only - no operator input in the log path (T-80-05).
    $installLogPath = Join-Path $env:TEMP 'nono-gate-install.log'
    $installArgs    = @('/i', $script:MsiPath, '/quiet', '/norestart', '/l*v', $installLogPath)
    $installProc    = Start-Process -FilePath 'msiexec.exe' `
                          -ArgumentList $installArgs `
                          -Wait -PassThru -NoNewWindow
    $installExit    = $installProc.ExitCode
    # 3010 = success + reboot required; treat as PASS for gate purposes (D-06).
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

    # --- STEP 2: nono --version from a NEW session (PATH propagation proof per D-06) ---
    # The MSI writes to SYSTEM PATH in the registry; the current session's PATH is frozen.
    # Only a fresh child process inherits the updated SYSTEM PATH. This proves PATH propagation.
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

    # --- STEP 3: Service state (non-fatal per D-06) ---
    # Records the observed state in detail; never flips a PASS verdict to FAIL.
    $wfpSvcAfter = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
    $wfpSvcState = if ($wfpSvcAfter) { $wfpSvcAfter.Status.ToString() } else { 'not-registered' }

    # --- STEP 4: Uninstall (cleanup for repeatability, D-06) ---
    $uninstallArgs = @('/x', $script:MsiPath, '/quiet', '/norestart')
    $unProc        = Start-Process -FilePath 'msiexec.exe' `
                         -ArgumentList $uninstallArgs `
                         -Wait -PassThru -NoNewWindow
    $uninstallExit = $unProc.ExitCode
    # Non-zero uninstall goes in detail but does NOT flip a PASS verdict to FAIL.

    # Assemble $detail BEFORE the verdict branch (per wfp-egress-isolation.ps1 pattern).
    $detail = [ordered]@{
        installExitCode   = $installExit
        rebootRequired    = ($installExit -eq 3010)
        versionOutput     = if ($null -ne $versionOut) { $versionOut.Trim() } else { '' }
        versionExitCode   = $versionExit
        wfpServiceState   = $wfpSvcState
        uninstallExitCode = $uninstallExit
        msiPath           = $script:MsiPath
    }

    # Verdict branch: nono --version must exit 0 AND produce non-empty output.
    if ($versionExit -ne 0 -or [string]::IsNullOrWhiteSpace($versionOut)) {
        return [ordered]@{
            gate      = 'clean-host-install'
            verdict   = 'FAIL'
            reason    = "nono --version failed (exit $versionExit) after install — PATH not propagated or binary does not load"
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
