# scripts/gates/wfp-egress-isolation.ps1
#
# Phase 79 Plan 01 — wfp-egress-isolation gate (WFP-01)
#
# CONTRACT (mirrors scripts/gates/harness-self-check.ps1, the reference contract for
# phases 77-80): this gate exports exactly two functions dot-sourced by
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
#
# WHAT THIS PROVES (satisfies WFP-01):
#   Two concurrent confined agents with distinct AppContainer package SIDs receive independent
#   WFP enforcement in one unattended run:
#     Agent A (nono-ts-wfp-test-open, network.block:false)  -> egress SUCCEEDS (exit 0)
#     Agent B (nono-ts-wfp-test-blocked, network.block:true) -> egress DENIED  (exit non-zero)
#
# SKIP vs FAIL classification:
#   SKIP_HOST_UNAVAILABLE: nono-wfp-service pipe absent / Connect timeout (T-79-01 precondition)
#                          nono-wfp-service went down mid-gate (detected via agent B stderr)
#                          curl.exe not available
#   FAIL: per-SID WFP filter produced the wrong result (A denied, or B allowed)
#   throw: harness-internal (nono not on PATH, cannot spawn at all)
#
# Pitfall guard (79-RESEARCH Pitfall 1 — false PASS):
#   When nono-wfp-service is absent, nono run --profile nono-ts-wfp-test-blocked exits
#   non-zero (WFP filter install fails -> process terminated). This looks like a PASS
#   (agent B denied) but is vacuous. Mitigation: Test-Precondition probes the pipe, and
#   Invoke-Gate inspects agent B stderr for WFP-unreachable diagnostics -> SKIP.

# ---------------------------------------------------------------------------
# Gate configuration
# ---------------------------------------------------------------------------

$script:GateTimeoutSeconds = 60
$script:AgentProfile_Open    = 'nono-ts-wfp-test-open'
$script:AgentProfile_Blocked = 'nono-ts-wfp-test-blocked'

# ---------------------------------------------------------------------------
# Local assertion helper (throw-on-failure per harness-self-check.ps1:42-54).
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
    # NOTE (per copilot-e2e.ps1 lines 94-95): nono absence is NOT a SKIP — it is a
    # harness-internal error (throw inside Invoke-Gate). Do not check nono here.

    # 1. Probe \\.\pipe\nono-wfp-control (2 s timeout).
    #    Service absent or pipe unavailable -> SKIP_HOST_UNAVAILABLE (T-79-01).
    $pipe = $null
    try {
        $pipe = [System.IO.Pipes.NamedPipeClientStream]::new(
            '.',
            'nono-wfp-control',
            [System.IO.Pipes.PipeDirection]::InOut)
        $pipe.Connect(2000)
        $pipe.Close()
    } catch {
        if ($pipe) { try { $pipe.Dispose() } catch { } }
        return 'nono-wfp-service is not running (pipe \\.\pipe\nono-wfp-control absent or did not accept in 2 s) — install and start nono-wfp-service then re-run'
    } finally {
        if ($pipe) { try { $pipe.Dispose() } catch { } }
    }

    # 2. Verify curl.exe is reachable (agent A and B launch curl.exe).
    $curlPath = 'C:\Windows\System32\curl.exe'
    if (-not (Test-Path -LiteralPath $curlPath)) {
        if (-not (Get-Command curl.exe -ErrorAction SilentlyContinue)) {
            return 'curl.exe not found at C:\Windows\System32\curl.exe or on PATH — required for WFP-01 egress probe'
        }
    }

    return $null
}

function Invoke-Gate {
    # WFP-01 two-agent concurrent egress isolation proof.
    # NEVER calls exit. NEVER calls Persist-Verdict. Returns exactly one verdict object.

    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'

    # --- Harness-internal precheck: nono must be on PATH ---
    $nono = Get-Command nono -ErrorAction SilentlyContinue
    Assert-True -Condition ($null -ne $nono) `
                -Message   'harness-internal: nono is not on PATH (build/install nono before running this gate)'

    # --- Spin up a loopback TCP mock server on port 0 (OS-assigned) ---
    # Accepts AT LEAST 2 connections (Pitfall 2 — 79-RESEARCH.md: single-accept loop
    # causes the second agent's connection to be refused, not denied by WFP, producing
    # a false FAIL verdict for agent A or a vacuous PASS for agent B).
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = $listener.LocalEndpoint.Port

    # Background Task accepts up to 2 connections; each gets a minimal HTTP 200 response.
    # The mock runs in a .NET Task (not a PS job) so it shares the same process memory
    # as the gate and can be stopped cleanly when both agents have exited.
    $listenerTask = [System.Threading.Tasks.Task]::Run([Action]{
        for ($i = 0; $i -lt 2; $i++) {
            try {
                $client = $listener.AcceptTcpClient()
                $stream = $client.GetStream()
                $response = [System.Text.Encoding]::ASCII.GetBytes(
                    "HTTP/1.1 200 OK`r`nContent-Length: 2`r`nConnection: close`r`n`r`nOK")
                $stream.Write($response, 0, $response.Length)
                $stream.Flush()
                $stream.Close()
                $client.Close()
            } catch {
                # Accept or write failed (e.g. listener stopped before second connect) —
                # swallow so the Task completes cleanly and does not block Wait().
            }
        }
    })

    # --- Launch Agent A and Agent B concurrently via Start-Job ---
    # Each job launches: nono run --profile <profile> -- curl.exe -s --max-time 5 http://127.0.0.1:<port>/probe
    # The job returns LASTEXITCODE (int) and collects combined stdout+stderr via 2>&1.
    $nonoSrc = $nono.Source

    $jobA = Start-Job -ScriptBlock {
        param($nonoPath, $port, $profile)
        $out = & $nonoPath run --profile $profile -- curl.exe -s --max-time 5 "http://127.0.0.1:$port/probe" 2>&1
        return @{ ExitCode = $LASTEXITCODE; Output = ($out -join "`n") }
    } -ArgumentList $nonoSrc, $port, $script:AgentProfile_Open

    $jobB = Start-Job -ScriptBlock {
        param($nonoPath, $port, $profile)
        $out = & $nonoPath run --profile $profile -- curl.exe -s --max-time 5 "http://127.0.0.1:$port/probe" 2>&1
        return @{ ExitCode = $LASTEXITCODE; Output = ($out -join "`n") }
    } -ArgumentList $nonoSrc, $port, $script:AgentProfile_Blocked

    # --- Wait for both jobs ---
    $null = Wait-Job $jobA, $jobB -Timeout $script:GateTimeoutSeconds

    # Collect results
    $resultA = Receive-Job $jobA -ErrorAction SilentlyContinue
    $resultB = Receive-Job $jobB -ErrorAction SilentlyContinue
    Remove-Job $jobA, $jobB -Force -ErrorAction SilentlyContinue

    # Stop the listener (allow the Task to complete naturally)
    try { $listener.Stop() } catch { }
    try { $null = $listenerTask.Wait(5000) } catch { }

    # Unpack exit-codes and combined output
    $exitA = if ($resultA -and $null -ne $resultA.ExitCode) { [int]$resultA.ExitCode } else { -1 }
    $exitB = if ($resultB -and $null -ne $resultB.ExitCode) { [int]$resultB.ExitCode } else { -1 }
    $outA  = if ($resultA -and $resultA.Output) { $resultA.Output } else { '' }
    $outB  = if ($resultB -and $resultB.Output) { $resultB.Output } else { '' }

    # --- Pitfall 1 guard (79-RESEARCH.md): detect vacuous PASS caused by WFP service down ---
    # When nono-wfp-service is absent, prepare_network_enforcement fails early and nono exits
    # non-zero before curl even runs. That looks like "agent B denied" but is vacuous.
    # Inspect agent B output for WFP-unreachable diagnostic strings and re-classify as SKIP.
    $wfpDownSignals = @(
        'WFP network scope required',
        'nono-wfp-service is not reachable',
        'not running',
        'nono-wfp-control',
        'Failed to connect to WFP service'
    )
    $wfpServiceDown = $false
    foreach ($signal in $wfpDownSignals) {
        if ($outB -match [regex]::Escape($signal)) {
            $wfpServiceDown = $true
            break
        }
    }
    if ($wfpServiceDown) {
        return [ordered]@{
            gate      = 'wfp-egress-isolation'
            verdict   = 'SKIP_HOST_UNAVAILABLE'
            reason    = 'nono run --profile nono-ts-wfp-test-blocked stderr indicates WFP service unreachable — install/start nono-wfp-service'
            detail    = [ordered]@{
                agentAExitCode = $exitA
                agentBExitCode = $exitB
                mockPort       = $port
                agentAProfile  = $script:AgentProfile_Open
                agentBProfile  = $script:AgentProfile_Blocked
                agentBStderr   = $outB
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # --- Verdict logic ---
    # PASS: Agent A egress succeeds (exitCode=0) AND Agent B egress is denied (exitCode!=0).
    # FAIL-A: Agent A egress was denied (exitCode!=0) — profile or WFP state issue.
    # FAIL-B: Agent B egress succeeded (exitCode=0) — per-SID WFP filter did not fire.

    if ($exitA -eq 0 -and $exitB -ne 0) {
        return [ordered]@{
            gate      = 'wfp-egress-isolation'
            verdict   = 'PASS'
            reason    = 'agent A egress succeeded (exitCode=0) and agent B egress denied (exitCode!=0) — per-SID WFP isolation confirmed'
            detail    = [ordered]@{
                agentAExitCode = $exitA
                agentBExitCode = $exitB
                mockPort       = $port
                agentAProfile  = $script:AgentProfile_Open
                agentBProfile  = $script:AgentProfile_Blocked
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    if ($exitA -ne 0) {
        return [ordered]@{
            gate      = 'wfp-egress-isolation'
            verdict   = 'FAIL'
            reason    = 'agent A egress denied (should be allowed) — check nono-ts-wfp-test-open profile coverage and WFP state'
            detail    = [ordered]@{
                agentAExitCode = $exitA
                agentBExitCode = $exitB
                mockPort       = $port
                agentAProfile  = $script:AgentProfile_Open
                agentBProfile  = $script:AgentProfile_Blocked
                agentAOutput   = $outA
            }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # exitB -eq 0 (agent B egress succeeded when it should be denied)
    return [ordered]@{
        gate      = 'wfp-egress-isolation'
        verdict   = 'FAIL'
        reason    = 'agent B egress succeeded (should be WFP-denied) — per-SID WFP filter did not deny B'
        detail    = [ordered]@{
            agentAExitCode = $exitA
            agentBExitCode = $exitB
            mockPort       = $port
            agentAProfile  = $script:AgentProfile_Open
            agentBProfile  = $script:AgentProfile_Blocked
            agentBOutput   = $outB
        }
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }
}
