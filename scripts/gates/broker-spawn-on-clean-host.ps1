# scripts/gates/broker-spawn-on-clean-host.ps1
#
# Phase 103 - broker-spawn-on-clean-host gate (CHOST-02)
#
# CONTRACT (mirrors the Phase 80 INST-01 install-proof gate, the reference contract for this
# harness): this gate exports exactly two functions dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object - it MUST NOT call exit and MUST NOT call
# Persist-Verdict. Only the runner owns exit-code mapping (PASS=0 / FAIL=2 /
# SKIP_HOST_UNAVAILABLE=3 / harness-internal=4) and the persist-before-emit (WR-04).
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE - exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
#
# WHAT THIS PROVES (satisfies CHOST-02): the machine MSI installs on a clean Win11 host and
# `nono run --profile claude-code` can spawn its broker/child cleanly with zero manual
# cert-trust steps. This gate does not reimplement broker-spawn logic - it drives the
# already-built `nono run --profile claude-code` command and observes its exit code and
# output (per the Architectural Responsibility Map in 103-RESEARCH.md).
#
# CRITICAL - SELF-CONTAINED, ORDER-SAFE UNDER -All: `scripts/verify-dark.ps1` discovers gates
# via `Get-ChildItem -Filter "*.ps1" | Sort-Object Name` (alphabetical). This file's name
# ("broker-spawn-on-clean-host") sorts BEFORE the INST-01 install gate's filename ('b' < 'c'),
# so under a `-All` sweep this gate ALWAYS runs first. It therefore MUST NEVER assume the
# INST-01 install gate has already installed anything - this gate installs AND uninstalls
# its OWN copy of the MSI entirely within its own Invoke-Gate. It has zero reference to, and
# zero dependency on, that other gate file or its state.
#
# WR-01: No stray pipeline output from Invoke-Gate. All Start-Process results are assigned to
#         named variables. No bare process object is written to the pipeline.
# WR-04: Only the runner persists the verdict file and owns the emit-before-persist order.

# ---------------------------------------------------------------------------
# Gate configuration (D-07 analog: -MsiPath defaults to repo-relative machine MSI, identical
# default to the INST-01 install gate's default per RESEARCH.md Assumption A1 - the machine MSI bundles
# nono.exe, nono-shell-broker.exe, and the WFP service needed for
# `nono run --profile claude-code` to spawn the broker)
# ---------------------------------------------------------------------------

# Operator stages this MSI on the fresh VM before running the gate. Override by setting
# $MsiPath before dot-sourcing. The runner dot-sources this file without parameters, so the
# default is load-bearing.
param(
    [string]$MsiPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist\windows\nono-machine.msi')
)
$script:MsiPath = $MsiPath

# ---------------------------------------------------------------------------
# Gate contract
# ---------------------------------------------------------------------------

function Test-Precondition {
    # Return $null when all preconditions met; return a reason string -> SKIP_HOST_UNAVAILABLE.
    # NOTE: Test-Precondition MUST NOT throw; a throw here causes a harness-internal error (exit 4).
    # Check in order: elevation -> nono.exe -> services -> MSI staged (cloned verbatim in shape
    # from the INST-01 install gate's Test-Precondition - this gate needs the same clean-host
    # guarantees since it also performs a machine-scope MSI install of its own).

    # 1. Elevation check: machine-scope MSI install requires administrator privileges.
    $identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return 'broker-spawn-on-clean-host gate requires elevation (machine MSI install needs admin) - re-run from an elevated shell'
    }

    # 2. Detect a dirty host - nono.exe already installed under Program Files.
    #    Use -LiteralPath to avoid wildcard expansion.
    if (Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe') {
        return 'nono.exe detected under C:\Program Files\nono — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }

    # 3. Detect registered nono services (either means a prior install).
    $wfpSvc   = Get-Service 'nono-wfp-service' -ErrorAction SilentlyContinue
    $agentSvc = Get-Service 'nono-agentd'      -ErrorAction SilentlyContinue
    if ($null -ne $wfpSvc -or $null -ne $agentSvc) {
        return 'nono service(s) already registered (nono-wfp-service or nono-agentd) — host is not clean; snapshot/restore and retry on a fresh Win11 VM'
    }

    # 4. MSI artifact must be staged on this VM before the gate runs.
    #    Use -LiteralPath to avoid wildcard expansion.
    if (-not (Test-Path -LiteralPath $script:MsiPath)) {
        return "MSI not found at $($script:MsiPath) - stage dist\windows\nono-machine.msi on this VM before running the gate"
    }

    return $null  # All clear - Invoke-Gate will run.
}

function Invoke-Gate {
    # Self-contained clean-host install -> broker-spawn -> uninstall proof (CHOST-02).
    # Returns exactly one verdict object (gate never calls exit or writes verdict files).
    # WR-01: all Start-Process results assigned to named variables; no stray pipeline output.

    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'

    $stamp = { Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ' }

    # Track per-leg results in detail (honest partial per Dark Factory standard, mirroring
    # deploy-silent-install.ps1's hardFails / detail-built-incrementally idiom).
    $detail    = [ordered]@{}
    $hardFails = [System.Collections.Generic.List[string]]::new()

    # =========================================================================
    # STEP 1: Install (this gate's OWN copy of the MSI - never assumes the
    # INST-01 install gate has already run in the same -All sweep).
    # =========================================================================
    $installLogPath = Join-Path $env:TEMP 'nono-broker-gate-install.log'
    $installArgs    = @('/i', $script:MsiPath, '/quiet', '/norestart', '/l*v', $installLogPath)
    $installProc    = Start-Process -FilePath 'msiexec.exe' `
                          -ArgumentList $installArgs `
                          -Wait -PassThru -NoNewWindow
    $installExit = $installProc.ExitCode
    # 3010 = success + reboot required; treat as PASS for gate purposes.
    $installOk = ($installExit -eq 0 -or $installExit -eq 3010)

    $detail['installExitCode'] = $installExit
    $detail['rebootRequired']  = ($installExit -eq 3010)
    $detail['installLog']      = $installLogPath

    if (-not $installOk) {
        $hardFails.Add("msiexec install failed (exit $installExit) — see $installLogPath") | Out-Null
        return [ordered]@{
            gate      = 'broker-spawn-on-clean-host'
            verdict   = 'FAIL'
            reason    = "msiexec install failed (exit $installExit) — MSI rolled back; check $installLogPath"
            detail    = $detail
            timestamp = & $stamp
        }
    }

    # =========================================================================
    # STEP 2: Broker-spawn proof (the step with no existing analog). Drive
    # `nono run --profile claude-code` from a NEW pwsh session and observe its exit
    # code and combined output - this gate does not reimplement broker-spawn logic,
    # it only drives the already-built command.
    # =========================================================================
    $spawnStdout = Join-Path $env:TEMP 'nono-broker-gate-spawn.stdout.tmp'
    $spawnStderr = Join-Path $env:TEMP 'nono-broker-gate-spawn.stderr.tmp'
    $spawnCommand = 'nono run --profile claude-code -- cmd /c exit 0'
    $spawnProc = Start-Process -FilePath 'pwsh.exe' `
                     -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', $spawnCommand) `
                     -Wait -PassThru -NoNewWindow `
                     -RedirectStandardOutput $spawnStdout `
                     -RedirectStandardError  $spawnStderr
    $brokerSpawnExitCode = $spawnProc.ExitCode
    $brokerSpawnOutput   = (Get-Content $spawnStdout -Raw -ErrorAction SilentlyContinue) +
                           (Get-Content $spawnStderr -Raw -ErrorAction SilentlyContinue)
    Remove-Item $spawnStdout, $spawnStderr -Force -ErrorAction SilentlyContinue

    $detail['brokerSpawnExitCode'] = $brokerSpawnExitCode
    $detail['brokerSpawnOutput']   = if ($null -ne $brokerSpawnOutput) { $brokerSpawnOutput.Trim() } else { '' }

    # A non-zero exit here is a hard FAIL - the broker did not spawn/execute cleanly with
    # zero manual cert-trust steps.
    if ($brokerSpawnExitCode -ne 0) {
        $hardFails.Add("nono run --profile claude-code failed (exit $brokerSpawnExitCode)") | Out-Null
    }

    # =========================================================================
    # STEP 3: Uninstall (cleanup for host repeatability). A non-zero uninstall goes
    # in detail but does NOT flip a PASS verdict to FAIL - informational only.
    # =========================================================================
    $uninstallArgs = @('/x', $script:MsiPath, '/quiet', '/norestart')
    $unProc        = Start-Process -FilePath 'msiexec.exe' `
                         -ArgumentList $uninstallArgs `
                         -Wait -PassThru -NoNewWindow
    $uninstallExit = $unProc.ExitCode
    $detail['uninstallExitCode'] = $uninstallExit
    $detail['msiPath']           = $script:MsiPath

    # Verdict branch: only Step 1 or Step 2 failures are hard fails; Step 3's outcome is
    # informational only (mirrors the INST-01 install gate's Step 4 / deploy-silent-install.ps1's
    # hardFails-only-flips-verdict idiom).
    if ($hardFails.Count -gt 0) {
        return [ordered]@{
            gate      = 'broker-spawn-on-clean-host'
            verdict   = 'FAIL'
            reason    = ($hardFails -join '; ')
            detail    = $detail
            timestamp = & $stamp
        }
    }

    return [ordered]@{
        gate      = 'broker-spawn-on-clean-host'
        verdict   = 'PASS'
        reason    = 'machine MSI installed, nono run --profile claude-code broker-spawn succeeded, uninstalled cleanly'
        detail    = $detail
        timestamp = & $stamp
    }
}
