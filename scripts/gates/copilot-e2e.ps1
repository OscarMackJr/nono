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

    # Network reachability probe (SKIP if offline — D-07). A raw TCP-443 connect is the
    # reliable reachability signal: it is fast on a connected host and does not depend on an
    # HTTP status (api.github.com returns 403 to an unauthenticated HEAD anyway, which proves
    # only reachability, not auth). The connect timeout is generous (10s) because a slow TLS
    # path on a marginally-connected host must not be misread as offline (OQ-3, settled on-host).
    $online = $false
    $tcp = $null
    try {
        $tcp = [System.Net.Sockets.TcpClient]::new()
        $iar = $tcp.BeginConnect('api.github.com', 443, $null, $null)
        if ($iar.AsyncWaitHandle.WaitOne(10000, $false) -and $tcp.Connected) {
            $online = $true
            $tcp.EndConnect($iar)
        }
    }
    catch {
        $online = $false
    }
    finally {
        if ($tcp) { $tcp.Close() }
    }
    if (-not $online) {
        return 'GitHub network unreachable (api.github.com:443) — Copilot auth/usage requires network'
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

    # --- Executable / shim coverage (77-RESEARCH Pitfall 2 / OQ-3, settled on-host) ---
    # The `copilot` launch target varies by install method: an npm global install is a
    # %APPDATA%\npm shim loading %APPDATA%\npm\node_modules; a WinGet install is a native
    # launcher under %LOCALAPPDATA%\Microsoft\WinGet\Packages\GitHub.Copilot_*. nono's
    # fail-secure executable-coverage gate (R-B3) refuses to launch any path the filesystem
    # policy does not cover. Resolve the ACTUAL copilot command and cover its directory so the
    # launch is not refused — this is install-method-agnostic (the plan's "resolve the package
    # entry" guidance). The copilot-cli profile already covers the node.exe interpreter (CPLT-01).
    $allowArgs = @()
    $copilotCmd = Get-Command copilot -ErrorAction SilentlyContinue
    if ($copilotCmd -and $copilotCmd.Source) {
        # Collect the command source dir AND, if it is a symlink/shim (e.g. the WinGet
        # `...\WinGet\Links\copilot.exe` that points at the real binary under
        # `...\WinGet\Packages\GitHub.Copilot_*`), the FINAL resolved target's dir. nono
        # canonicalizes the launch target and the exe-coverage gate checks the RESOLVED path,
        # so the package dir — not just the shim dir — must be covered.
        $coverDirs = [System.Collections.Generic.List[string]]::new()
        $coverDirs.Add((Split-Path -Parent $copilotCmd.Source))
        try {
            $item = Get-Item -LiteralPath $copilotCmd.Source -ErrorAction Stop
            $target = $item.ResolveLinkTarget($true)   # follows symlink chains; $null if not a link
            if ($target -and $target.FullName) {
                $coverDirs.Add((Split-Path -Parent $target.FullName))
            }
        }
        catch { }
        foreach ($d in ($coverDirs | Select-Object -Unique)) {
            if ($d -and (Test-Path -LiteralPath $d)) {
                $allowArgs += @('--allow', $d)
            }
        }
    }
    # Cover the Node interpreter's directory. The copilot-cli profile declares
    # `windows_interpreters: ["node.exe"]` by NAME, but nono's fail-secure interpreter gate
    # additionally requires the filesystem policy to cover the directory the interpreter lives
    # in (it refuses to launch a partially-confined engine where the wrapper would spawn an
    # uncovered interpreter). The WinGet copilot.exe launcher spawns the system node at
    # `C:\Program Files\nodejs\node.exe`; resolve node and cover its dir.
    $nodeCmd = Get-Command node -ErrorAction SilentlyContinue
    if ($nodeCmd -and $nodeCmd.Source) {
        $nodeDir = Split-Path -Parent $nodeCmd.Source
        if ($nodeDir -and (Test-Path -LiteralPath $nodeDir)) {
            $allowArgs += @('--allow', $nodeDir)
        }
    }

    # Also cover the npm global dir when present (covers the npm-install shape; harmless otherwise).
    $npmDir = if ($env:APPDATA) { Join-Path $env:APPDATA 'npm' } else { $null }
    if ($npmDir -and (Test-Path -LiteralPath $npmDir)) {
        $allowArgs += @('--allow', $npmDir)
    }

    # --- Dedicated covered workspace (cwd-coverage / AppContainer execution-dir gate) ---
    # nono refuses a live run whose execution directory is outside the supported allowlist.
    # `--workspace <DIR>` sets the confined child's CWD AND the writable grant (single source
    # of truth). Use a stable dir under %USERPROFILE% (drive-root workspaces fail the
    # AppContainer label/grant; the user profile is the proven location).
    $workspace = Join-Path $env:USERPROFILE 'nono-copilot-e2e-gate'
    if (-not (Test-Path -LiteralPath $workspace)) {
        New-Item -ItemType Directory -Path $workspace -Force | Out-Null
    }
    # R-B3: nono applies a mandatory integrity label (NO_WRITE_UP) to the workspace, which
    # requires the current user to OWN the workspace AND hold WRITE_OWNER (0x80000) on it.
    # WRITE_OWNER is NOT implicit for an owner, and a dir created from an ELEVATED session is
    # owned by BUILTIN\Administrators (not the user) — both break R-B3. Set the current user as
    # the explicit owner (via SID) and grant Full Control (includes WRITE_OWNER). On a normal
    # non-elevated session the user already owns dirs it creates, so /setowner is a harmless
    # no-op; this keeps the gate robust whether or not it is run elevated.
    $mySid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    & icacls $workspace /setowner "*$mySid" /Q 2>&1 | Out-Null
    & icacls $workspace /grant "*$($mySid):(OI)(CI)F" /Q 2>&1 | Out-Null
    Set-Content -Path (Join-Path $workspace 'README.txt') `
                -Value 'nono copilot-e2e gate workspace' -Encoding UTF8 -ErrorAction SilentlyContinue

    # --- Build the confined one-shot invocation (the CPLT-03 key-link) ---
    #   nono run --profile copilot-cli --workspace <ws> [--allow <exe/interp dirs>] -- copilot -p "<prompt>"
    $nonoArgs = @('run', '--profile', 'copilot-cli', '--workspace', $workspace) + $allowArgs + `
                @('--', 'copilot', $script:CopilotOneShotFlag, $script:CopilotPrompt)

    $timedOut = $false
    $exitCode = $null
    $output = ''

    # Use ProcessStartInfo.ArgumentList — each element is escaped individually, so paths
    # with spaces (e.g. "C:\Program Files\nodejs") are passed as ONE argument. (Start-Process
    # -ArgumentList with an array does naive space-joining and would split such paths.)
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $nono.Source
    foreach ($a in $nonoArgs) { [void]$psi.ArgumentList.Add([string]$a) }
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $proc = [System.Diagnostics.Process]::new()
    $proc.StartInfo = $psi
    try {
        [void]$proc.Start()
        # Drain both streams concurrently (async) so a full pipe buffer cannot deadlock.
        $outTask = $proc.StandardOutput.ReadToEndAsync()
        $errTask = $proc.StandardError.ReadToEndAsync()

        if (-not $proc.WaitForExit($script:GateTimeoutSeconds * 1000)) {
            $timedOut = $true
            try { $proc.Kill($true) } catch { }
            try { [void]$proc.WaitForExit(5000) } catch { }
        }
        else {
            $exitCode = $proc.ExitCode
        }

        $out = ''
        $err = ''
        try { $out = $outTask.GetAwaiter().GetResult() } catch { }
        try { $err = $errTask.GetAwaiter().GetResult() } catch { }
        $output = (@($out, $err) -join "`n").Trim()
    }
    finally {
        $proc.Dispose()
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

    # (2) CONFINEMENT FAIL — Node module-resolution crash (the v2.12 SC3 / CPLT failure mode).
    #     Includes the signature ancestor-RA denial: realpathSync/lstat on an ancestor (often
    #     the drive root `C:\`) returns EPERM because FILE_READ_ATTRIBUTES is not granted on it
    #     — exactly what the CPLT-01 runtime guard + CPLT-02 one-time-admin grant exist to fix.
    if ($output -match 'Cannot find module' -or `
        $output -match 'ERR_MODULE_NOT_FOUND' -or `
        $output -match 'ENOENT.*node_modules' -or `
        $output -match "(?i)EPERM.*lstat" -or `
        $output -match '(?i)realpathSync' -or `
        $output -match '(?i)Failed to load package index') {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'FAIL'
            reason    = 'Node module-resolution crash under confinement (realpathSync/lstat ancestor walk denied — run the CPLT-02 one-time-admin grant: nono setup --grant-ancestors --profile copilot-cli)'
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

    # (4) FAIL — nono itself could not launch the confined Copilot run (the run never
    #     happened): a missing/uninstalled `copilot-cli` profile (stale or unbuilt nono),
    #     an executable-coverage refusal, or any nono-level usage/config error. This is the
    #     anti-false-PASS guard (threat T-77-03): a nono diagnostic is NOT a Copilot
    #     suggestion and MUST NOT be reported as a PASS. The provisioned-host contract
    #     requires `make build` so the freshly-built nono embeds the copilot-cli profile.
    if ($output -match '(?i)profile not found' -or `
        $output -match '(?i)not covered' -or `
        $output -match '(?i)executable .*(not covered|coverage)' -or `
        $output -match '(?i)^nono: .*error' -or `
        $output -match '(?i)\berror:\s') {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'FAIL'
            reason    = 'nono could not launch the confined Copilot run (profile/coverage/config error) — confined run did not occur; build+install nono so the copilot-cli profile is embedded'
            detail    = $detail
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # (5) SKIP — auth gap that slipped past Test-Precondition. Only reachable when NO
    #     confinement violation and NO nono-launch error was detected above, so a real
    #     denial or launch failure is never masked as a SKIP (D-07 / T-77-03b).
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

    # (6) FAIL — the confined wrapper exited non-zero (the one-shot did not complete
    #     cleanly) even though no specific marker above matched. Surfaced rather than
    #     masked: a clean PASS requires a zero exit.
    if ($null -ne $exitCode -and $exitCode -ne 0) {
        return [ordered]@{
            gate      = 'copilot-e2e'
            verdict   = 'FAIL'
            reason    = "confined Copilot run exited non-zero (exit $exitCode) — one-shot task did not complete cleanly"
            detail    = $detail
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }

    # (7) FAIL — preconditions were met and no denial occurred, but Copilot produced no
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

    # (8) PASS — nono launched the confined run, the wrapper exited 0, and a real, non-empty
    #     Copilot suggestion was printed with zero STATUS_ACCESS_DENIED and zero Node
    #     module-resolution crash (CPLT-03 / D-08).
    return [ordered]@{
        gate      = 'copilot-e2e'
        verdict   = 'PASS'
        reason    = 'confined Copilot one-shot completed end-to-end: clean exit, non-empty suggestion, no STATUS_ACCESS_DENIED, no module-resolution crash'
        detail    = $detail
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }
}
