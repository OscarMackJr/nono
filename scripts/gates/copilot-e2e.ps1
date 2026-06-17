# scripts/gates/copilot-e2e.ps1
#
# Phase 77 Plan 03 — copilot-e2e gate (CPLT-03)
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
# WHAT THIS PROVES (replaces the interactive v2.12 Phase 75 SC3 UAT):
#   The standalone @github/copilot Node CLI completes a real one-shot task end-to-end under
#   AppContainer confinement (`nono run --profile copilot-cli`) with ZERO STATUS_ACCESS_DENIED
#   and ZERO Node module-resolution crash. It exercises the CPLT-01 runtime ancestor-RA guard
#   and DEPENDS on the CPLT-02 one-time-admin grant (`nono setup --grant-ancestors`) having
#   been run on this host.
#
# D-07 (SKIP, not FAIL): Copilot not installed / not authenticated / offline is a host
#   precondition gap, surfaced as SKIP_HOST_UNAVAILABLE — NOT a confinement FAIL.
# D-08 (FAIL, not SKIP): a STATUS_ACCESS_DENIED or a Node module-resolution crash during the
#   confined run is a real confinement FAIL — and is asserted BEFORE any auth/skip heuristic so
#   a real denial is NEVER masked as a SKIP (threat T-77-03b).
#
# OQ-3 (settle empirically on the live host): the exact Copilot one-shot flag and the
#   %APPDATA%\npm shim-coverage --allow set are host-dependent. The defaults below
#   (`-p` one-shot, %APPDATA%\npm allow) are the researched starting point (77-RESEARCH
#   Pitfall 2 / Pitfall 4); adjust on-host if the launch is refused at the executable-coverage
#   gate or Copilot's one-shot flag differs.

# ---------------------------------------------------------------------------
# Gate configuration (host-tunable — OQ-3)
# ---------------------------------------------------------------------------

# A trivial, deterministic, read-only one-shot prompt. The assertion is on CONFINEMENT
# behavior, not on Copilot's answer quality (77-RESEARCH Pitfall 4).
$script:CopilotPrompt = 'list the files in the current directory'

# The non-interactive one-shot flag for @github/copilot. `-p` / `--prompt` is the
# researched programmatic form; confirm on-host per OQ-3 if Copilot's CLI differs.
$script:CopilotOneShotFlag = '-p'

# Hard timeout so an interactive REPL (or a hung confined child) cannot stall the
# unattended harness (Pitfall 4 / threat T-77-03c).
$script:GateTimeoutSeconds = 120

# ---------------------------------------------------------------------------
# Local assertion helper (throw-on-failure per harness-self-check.ps1:42-54).
# NOTE: a throw is a HARNESS-INTERNAL error (exit 4) — used ONLY for "the gate itself
# could not run" (e.g. nono not on PATH, could not spawn the process). A CONFINEMENT
# violation is NOT a throw — it is a returned verdict='FAIL' (D-08).
# ---------------------------------------------------------------------------

function Assert-True {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$Condition,

        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

# ---------------------------------------------------------------------------
# Gate contract
# ---------------------------------------------------------------------------

function Test-Precondition {
    # D-07: detect ONLY host-precondition gaps (missing copilot / no auth / offline) and
    # return a reason string -> SKIP_HOST_UNAVAILABLE. Never conflate a missing or
    # unauthenticated Copilot with a confinement FAIL (that split is Invoke-Gate's job).
    # Return $null when every precondition is met.

    if (-not (Get-Command copilot -ErrorAction SilentlyContinue)) {
        return 'copilot CLI not installed (run: npm install -g @github/copilot)'
    }

    # nono itself is NOT a host precondition — its absence is a harness/env error, not a
    # host-unavailable SKIP. That is asserted (throw -> exit 4) inside Invoke-Gate.

    # Network reachability probe (cheap; SKIP if offline — D-07).
    $online = $false
    try {
        $req = [System.Net.WebRequest]::Create('https://api.github.com')
        $req.Method = 'HEAD'
        $req.Timeout = 5000
        $resp = $req.GetResponse()
        $online = $true
        $resp.Close()
    }
    catch {
        # A 401/403 still proves reachability (we got an HTTP response, not a transport error).
        if ($_.Exception.Response) { $online = $true }
    }
    if (-not $online) {
        return 'GitHub network unreachable (api.github.com) — Copilot auth/usage requires network'
    }

    # Auth probe (best-effort; SKIP if clearly not authenticated — D-07). The exact
    # authed-status command is host-empirical (OQ-3); prefer `gh auth status` when the
    # GitHub CLI is present. If we cannot positively DISPROVE auth, fall through to $null
    # and let Invoke-Gate's auth-marker heuristic emit a SKIP (never a FAIL) if needed.
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        $null = (& gh auth status 2>&1)
        if ($LASTEXITCODE -ne 0) {
            return 'GitHub CLI not authenticated (run: gh auth login) — Copilot requires GitHub auth'
        }
    }

    return $null
}

function Invoke-Gate {
    # D-08: run a SINGLE confined ONE-SHOT Copilot task under nono, capture stdout+stderr,
    # and assert confinement behavior. Confinement violations are returned as verdict='FAIL'
    # (NOT thrown). A throw here is reserved for harness-internal failure (nono missing /
    # cannot spawn) -> exit 4, never a silent PASS.

    # Native tools (cargo/node) write progress to stderr; do not promote that to a
    # terminating error while we capture output (mirrors verify-dark.ps1:17 reasoning).
    $ErrorActionPreference = 'Continue'

    # --- Harness-internal precheck: nono must be runnable (else exit 4, not FAIL) ---
    $nono = Get-Command nono -ErrorAction SilentlyContinue
    Assert-True -Condition ($null -ne $nono) `
                -Message   'harness-internal: nono is not on PATH (build/install nono before running this gate)'

    # --- Shim coverage (77-RESEARCH Pitfall 2 / OQ-3) ---
    # On Windows the `copilot` command is a %APPDATA%\npm shim that loads the package entry
    # under %APPDATA%\npm\node_modules. The copilot-cli profile covers node.exe (CPLT-01),
    # but the shim dir + package dir may also need launch coverage. Add them as --allow paths
    # when present so the confined launch is not refused at the executable-coverage gate.
    $allowArgs = @()
    $npmDir = if ($env:APPDATA) { Join-Path $env:APPDATA 'npm' } else { $null }
    if ($npmDir -and (Test-Path -LiteralPath $npmDir)) {
        $allowArgs += @('--allow', $npmDir)
    }

    # --- Build the confined one-shot invocation (the CPLT-03 key-link) ---
    #   nono run --profile copilot-cli [--allow <npm shim dir>] -- copilot -p "<prompt>"
    $nonoArgs = @('run', '--profile', 'copilot-cli') + $allowArgs + `
                @('--', 'copilot', $script:CopilotOneShotFlag, $script:CopilotPrompt)

    $stdoutFile = New-TemporaryFile
    $stderrFile = New-TemporaryFile
    $timedOut = $false
    $exitCode = $null
    $output = ''

    try {
        $proc = Start-Process -FilePath $nono.Source `
                              -ArgumentList $nonoArgs `
                              -NoNewWindow -PassThru `
                              -RedirectStandardOutput $stdoutFile.FullName `
                              -RedirectStandardError  $stderrFile.FullName

        if (-not $proc.WaitForExit($script:GateTimeoutSeconds * 1000)) {
            $timedOut = $true
            try { $proc.Kill($true) } catch { }
            try { $proc.WaitForExit(5000) | Out-Null } catch { }
        }
        else {
            $exitCode = $proc.ExitCode
        }

        $out = (Get-Content -LiteralPath $stdoutFile.FullName -Raw -ErrorAction SilentlyContinue)
        $err = (Get-Content -LiteralPath $stderrFile.FullName -Raw -ErrorAction SilentlyContinue)
        $output = (@($out, $err) -join "`n").Trim()
    }
    finally {
        Remove-Item -LiteralPath $stdoutFile.FullName -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $stderrFile.FullName -Force -ErrorAction SilentlyContinue
    }

    $detail = [ordered]@{
        invocation = "nono $($nonoArgs -join ' ')"
        exitCode   = $exitCode
        timedOut   = $timedOut
        outputHead = if ($output.Length -gt 600) { $output.Substring(0, 600) } else { $output }
    }

    # === Assertions — ORDER IS LOAD-BEARING (T-77-03b: never mask a real denial as SKIP) ===

    # (1) CONFINEMENT FAIL — a real access denial. Checked FIRST so it can never be
    #     reclassified as an auth/skip below (D-08).
    if ($output -match 'STATUS_ACCESS_DENIED' -or $output -match 'Access is denied') {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'FAIL'
            reason    = 'confined Copilot run hit STATUS_ACCESS_DENIED — ancestor-RA chain incomplete'
            detail    = $detail
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # (2) CONFINEMENT FAIL — Node module-resolution crash (the v2.12 SC3 failure mode).
    if ($output -match 'Cannot find module' -or `
        $output -match 'ERR_MODULE_NOT_FOUND' -or `
        $output -match 'ENOENT.*node_modules') {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'FAIL'
            reason    = 'Node module-resolution crash under confinement (realpathSync/lstat ancestor walk denied)'
            detail    = $detail
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # (3) CONFINEMENT FAIL — the unattended run hung (interactive REPL or stuck child).
    if ($timedOut) {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'FAIL'
            reason    = "confined Copilot run exceeded ${script:GateTimeoutSeconds}s timeout (no one-shot exit — check OQ-3 one-shot flag)"
            detail    = $detail
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # (4) SKIP — auth gap that slipped past Test-Precondition. Only reachable when NO
    #     confinement violation was detected above, so a real denial is never masked (D-07).
    if ($output -match '(?i)not (logged|signed) in' -or `
        $output -match '(?i)please (sign|log) in' -or `
        $output -match '(?i)authentication (failed|required)' -or `
        $output -match '(?i)unauthorized') {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'SKIP_HOST_UNAVAILABLE'
            reason    = 'Copilot reported an authentication gap — not a confinement failure (D-07)'
            detail    = $detail
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # (5) FAIL — preconditions were met and no denial occurred, but Copilot produced no
    #     output: the confined task did not complete (no real suggestion was printed).
    if ([string]::IsNullOrWhiteSpace($output)) {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'FAIL'
            reason    = 'confined Copilot produced no output — task did not complete under confinement'
            detail    = $detail
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # (6) PASS — a real, non-empty confined Copilot suggestion with zero STATUS_ACCESS_DENIED
    #     and zero Node module-resolution crash (CPLT-03 / D-08).
    return [ordered]@{
        gate      = 'copilot-e2e'
        verdict   = 'PASS'
        reason    = 'confined Copilot one-shot completed end-to-end: non-empty suggestion, no STATUS_ACCESS_DENIED, no module-resolution crash'
        detail    = $detail
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }
}
