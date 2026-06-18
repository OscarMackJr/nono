# scripts/gates/wfp-egress-isolation.ps1
#
# Phase 79 Plan 01 - WFP-01 per-SID egress isolation gate (DAEMON-PATH structural proof)
#
# CONTRACT (mirrors scripts/gates/harness-self-check.ps1, the reference contract for
# phases 77-81): this gate exports exactly two functions, dot-sourced by
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
# WHAT THIS PROVES (satisfies WFP-01):
#   Per-SID WFP egress isolation is actually installed in the Windows kernel. Two confined
#   agents are launched THROUGH the multi-tenant daemon (nono-agentd) - the code path that
#   installs per-package-SID WFP filters (agent_daemon/launch.rs::wfp_filter_add ->
#   nono-wfp-service). Each launch returns the agent's AppContainer package SID. The gate then
#   inspects live kernel WFP state (netsh wfp show filters):
#     Agent B (nono-ts-wfp-test-blocked, network.block:true)  -> a per-SID FWP_ACTION_BLOCK
#                                                                  filter conditioned on
#                                                                  FWPM_CONDITION_ALE_USER_ID =
#                                                                  B's package SID MUST exist.
#     Agent A (nono-ts-wfp-test-open, network.block:false)     -> NO nono block filter for A's
#                                                                  package SID.
#   PASS proves enforcement is per-SID isolated (B filtered, A not) in one unattended run.
#
# WHY DAEMON-PATH (not direct `nono run`):
#   A direct `nono run` confined agent enforces network-block via the zero-capability
#   AppContainer (the lowbox child is created with SECURITY_CAPABILITIES{ CapabilityCount: 0 },
#   so it has no network capability at all and cannot egress to ANY target) - NOT via a WFP
#   filter. Direct-run agents therefore install no WFP filter, and "allowed vs blocked" is
#   unobservable through an egress probe. Per-SID WFP egress isolation is a daemon (multi-tenant)
#   feature. See 79-01-SUMMARY.md for the full empirical record (OQ-1 was falsified at runtime).
#
# WHY THE BUSY-LOOP KEEP-ALIVE:
#   A zero-capability AppContainer agent cannot run ping/curl (no network) and self-exits
#   instantly; the daemon then reaps it and removes the WFP filter before the (slow) `netsh wfp`
#   dump can observe it. The agent command is a pure CPU busy-loop (`for /L ... do @rem`,
#   no network/console) so the agent - and thus its per-SID WFP filter - stays alive across the
#   snapshot window, then exits on its own. The gate uses a baseline delta (new SID vs baseline)
#   so any pre-existing leaked filters do not affect the verdict.

# ---------------------------------------------------------------------------
# Gate configuration
# ---------------------------------------------------------------------------

# CPU busy-loop: keeps the confined agent alive ~15-30s with no network/console dependency.
$script:KeepAliveCmd   = 'for /L %i in (1,1,60000000) do @rem'
$script:ProfileBlocked = 'nono-ts-wfp-test-blocked'
$script:ProfileOpen    = 'nono-ts-wfp-test-open'
$script:WfpDumpPath    = (Join-Path $env:TEMP 'nono-wfp-gate-filters.xml')

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

# Return the set of AppContainer package SIDs (S-1-15-2-*) that currently have a nono
# FWP_ACTION_BLOCK filter conditioned on FWPM_CONDITION_ALE_USER_ID. Requires admin
# (netsh wfp show filters). Returns @() if the dump is unavailable or empty.
function Get-NonoBlockSids {
    & netsh wfp show filters file=$script:WfpDumpPath 2>&1 | Out-Null
    if (-not (Test-Path -LiteralPath $script:WfpDumpPath)) { return @() }

    [xml]$doc = Get-Content -LiteralPath $script:WfpDumpPath -Raw
    $sids = @{}
    foreach ($f in $doc.wfpdiag.filters.item) {
        if ($f.displayData.name -notmatch 'nono') { continue }
        if ($f.action.type -ne 'FWP_ACTION_BLOCK') { continue }
        $cond = $f.filterCondition.item | Where-Object { $_.fieldKey -eq 'FWPM_CONDITION_ALE_USER_ID' }
        $sd = $cond.conditionValue.sd
        if ($sd -match '(S-1-15-2-[\d-]+)') { $sids[$Matches[1]] = $true }
    }
    return @($sids.Keys)
}

# Parse the AppContainer package SID from a `nono agent launch` response
# (the daemon prints "  sid=S-1-15-2-..."). Returns $null if not present.
function Get-LaunchSid {
    param([string]$Text)
    if ($Text -match 'sid=(S-1-15-2[^\s]+)') { return $Matches[1] }
    return $null
}

# ---------------------------------------------------------------------------
# Gate contract
# ---------------------------------------------------------------------------

function Test-Precondition {
    # Return $null when all preconditions met; return a reason string -> SKIP_HOST_UNAVAILABLE.
    # NOTE: nono absence is NOT a SKIP - it is a harness-internal error (Assert-True throw inside
    # Invoke-Gate). Do not check nono on PATH here.

    # 1. Admin required: `netsh wfp show filters` needs elevation.
    $identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return 'WFP-01 gate requires elevation (netsh wfp show filters needs admin) - re-run from an elevated shell'
    }

    # 2. nono-wfp-service control pipe (installs the per-SID WFP filters on the daemon path).
    $svcPipe = $null
    try {
        $svcPipe = [System.IO.Pipes.NamedPipeClientStream]::new(
            '.', 'nono-wfp-control', [System.IO.Pipes.PipeDirection]::InOut)
        $svcPipe.Connect(2000)
        $svcPipe.Close()
    } catch {
        return 'nono-wfp-service is not running (pipe \\.\pipe\nono-wfp-control absent or did not accept in 2 s) - install and start nono-wfp-service then re-run'
    } finally {
        if ($svcPipe) { try { $svcPipe.Dispose() } catch { } }
    }

    # 3. nono-agentd control pipe (the multi-tenant daemon that launches agents on the WFP path).
    $daemonPipe = $null
    try {
        $daemonPipe = [System.IO.Pipes.NamedPipeClientStream]::new(
            '.', 'nono-agentd-control', [System.IO.Pipes.PipeDirection]::InOut)
        $daemonPipe.Connect(2000)
        $daemonPipe.Close()
    } catch {
        return 'nono-agentd is not running (pipe \\.\pipe\nono-agentd-control absent) - start the daemon in the user (non-elevated) context with `nono daemon start` then re-run'
    } finally {
        if ($daemonPipe) { try { $daemonPipe.Dispose() } catch { } }
    }

    return $null
}

function Invoke-Gate {
    # WFP-01 daemon-path per-SID isolation proof.
    # NEVER calls exit. NEVER calls Persist-Verdict. Returns exactly one verdict object.

    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'

    # --- Harness-internal precheck: nono must be on PATH ---
    $nono = Get-Command nono -ErrorAction SilentlyContinue
    Assert-True -Condition ($null -ne $nono) `
                -Message   'harness-internal: nono is not on PATH (build/install nono before running this gate)'
    $nonoExe = $nono.Source

    # Detect the elevated-daemon misconfiguration (workspace ownership fails when the daemon runs
    # elevated) so it reads as SKIP, not a spurious FAIL.
    $daemonElevatedSignals = @('workspace not owned by current user', 'DaemonDaclGuard')
    # Detect the WFP-service-unreachable fail-secure refusal so it reads as SKIP, not FAIL.
    $wfpDownSignals = @('nono-wfp-service', 'not reachable', 'WFP network scope required', 'rejected the request')

    $baseline = @(Get-NonoBlockSids)

    # --- Agent B: blocked profile (network.block=true). Daemon MUST install a per-SID WFP filter. ---
    $respB = & $nonoExe agent launch --profile $script:ProfileBlocked -- cmd /c $script:KeepAliveCmd 2>&1 | Out-String
    $sidB = Get-LaunchSid $respB
    if (-not $sidB) {
        $elevated = $false
        foreach ($s in $daemonElevatedSignals) { if ($respB -match [regex]::Escape($s)) { $elevated = $true; break } }
        if ($elevated) {
            return [ordered]@{
                gate      = 'wfp-egress-isolation'
                verdict   = 'SKIP_HOST_UNAVAILABLE'
                reason    = 'nono-agentd is running elevated (workspace ownership check fails) - restart the daemon in the non-elevated user context, then re-run'
                detail    = [ordered]@{ blockedProfile = $script:ProfileBlocked; blockedLaunchOutput = $respB.Trim() }
                timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
            }
        }
        $wfpDown = $false
        foreach ($s in $wfpDownSignals) { if ($respB -match [regex]::Escape($s)) { $wfpDown = $true; break } }
        if ($wfpDown) {
            return [ordered]@{
                gate      = 'wfp-egress-isolation'
                verdict   = 'SKIP_HOST_UNAVAILABLE'
                reason    = 'daemon refused the blocked-agent launch because nono-wfp-service is unreachable (fail-secure) - start nono-wfp-service then re-run'
                detail    = [ordered]@{ blockedProfile = $script:ProfileBlocked; blockedLaunchOutput = $respB.Trim() }
                timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
            }
        }
        return [ordered]@{
            gate      = 'wfp-egress-isolation'
            verdict   = 'FAIL'
            reason    = 'blocked agent failed to launch through the daemon (no package SID in response)'
            detail    = [ordered]@{ blockedProfile = $script:ProfileBlocked; blockedLaunchOutput = $respB.Trim() }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    Start-Sleep -Milliseconds 800
    $afterB = @(Get-NonoBlockSids)
    $blockedHasFilter = ($afterB -contains $sidB) -and (-not ($baseline -contains $sidB))

    # --- Agent A: open profile (network.block=false). Daemon MUST install NO WFP filter. ---
    $respA = & $nonoExe agent launch --profile $script:ProfileOpen -- cmd /c $script:KeepAliveCmd 2>&1 | Out-String
    $sidA = Get-LaunchSid $respA
    if (-not $sidA) {
        $elevatedA = $false
        foreach ($s in $daemonElevatedSignals) { if ($respA -match [regex]::Escape($s)) { $elevatedA = $true; break } }
        if ($elevatedA) {
            return [ordered]@{
                gate      = 'wfp-egress-isolation'
                verdict   = 'SKIP_HOST_UNAVAILABLE'
                reason    = 'nono-agentd is running elevated (workspace ownership check fails) - restart the daemon in the non-elevated user context, then re-run'
                detail    = [ordered]@{ openProfile = $script:ProfileOpen; openLaunchOutput = $respA.Trim() }
                timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
            }
        }
        return [ordered]@{
            gate      = 'wfp-egress-isolation'
            verdict   = 'FAIL'
            reason    = 'allowed agent failed to launch through the daemon (no package SID in response)'
            detail    = [ordered]@{ openProfile = $script:ProfileOpen; openLaunchOutput = $respA.Trim() }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    Start-Sleep -Milliseconds 800
    $afterA = @(Get-NonoBlockSids)
    $allowedHasFilter = ($afterA -contains $sidA)

    $detail = [ordered]@{
        blockedProfile        = $script:ProfileBlocked
        blockedSid            = $sidB
        blockedHasFilter      = $blockedHasFilter
        openProfile           = $script:ProfileOpen
        openSid               = $sidA
        openHasFilter         = $allowedHasFilter
        baselineBlockSidCount = $baseline.Count
        afterBlockedSidCount  = $afterB.Count
        afterOpenSidCount     = $afterA.Count
    }
    $stamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')

    # --- Verdict logic ---
    # PASS: blocked agent has a per-SID WFP block filter AND allowed agent has none.
    if ($blockedHasFilter -and -not $allowedHasFilter) {
        return [ordered]@{
            gate      = 'wfp-egress-isolation'
            verdict   = 'PASS'
            reason    = 'per-SID WFP egress isolation proven: blocked agent received a per-package-SID WFP block filter; allowed agent received none'
            detail    = $detail
            timestamp = $stamp
        }
    }

    if (-not $blockedHasFilter) {
        return [ordered]@{
            gate      = 'wfp-egress-isolation'
            verdict   = 'FAIL'
            reason    = 'blocked agent did NOT receive a per-SID WFP block filter for its package SID (expected one) - check nono-wfp-service WFP installation'
            detail    = $detail
            timestamp = $stamp
        }
    }

    # blockedHasFilter is true but allowedHasFilter is also true.
    return [ordered]@{
        gate      = 'wfp-egress-isolation'
        verdict   = 'FAIL'
        reason    = 'allowed agent unexpectedly received a per-SID WFP block filter (should have none) - per-SID isolation breach'
        detail    = $detail
        timestamp = $stamp
    }
}
