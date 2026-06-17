param(
    # Gate name to run. Must match the base name of a file in scripts/gates/<name>.ps1.
    # Gates are auto-discovered (D-04) — no hardcoded ValidateSet; unknown names are
    # harness-internal errors (exit 1), never a FAIL verdict (D-05).
    [string]$Gate,

    # Run all discovered gates in sequence (Phase 81 formalizes the {gates:[...],overall}
    # rollup; the per-gate dispatch loop is left reusable here without the aggregator logic).
    [switch]$All
)

$ErrorActionPreference = "Stop"
# Cargo and other native tools write normal progress output to stderr.
# Keep that from being promoted into terminating PowerShell errors while we tee logs.
# (Mirrors scripts/windows-test-harness.ps1:7-10; gates that shell out may locally
# set $ErrorActionPreference = 'Continue' inside Invoke-Gate per PATTERNS.md reasoning.)
$PSNativeCommandUseErrorActionPreference = $false

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Get-IsoTimestamp {
    # ISO-ish timestamp matching the CONTEXT.md D-01 example.
    # Source: scripts/test-windows-shell-write-deny.ps1:60
    return (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}

function Build-Verdict {
    param(
        [string]$GateName,
        [string]$Verdict,
        [string]$Reason,
        $Detail = @{}
    )
    # Locked key order per D-01: gate, verdict, reason, detail, timestamp
    return [ordered]@{
        gate      = $GateName
        verdict   = $Verdict
        reason    = $Reason
        detail    = $Detail
        timestamp = Get-IsoTimestamp
    }
}

function Emit-Verdict {
    param($VerdictObj)
    # -Depth 6 (NOT default 2!) so nested objects in detail don't serialize as
    # "System.Object[]". -Compress matches bash printf no-pretty-print.
    # [Console]::Out.Write + explicit LF to avoid PowerShell CRLF on Windows.
    # Source: scripts/check-upstream-drift.ps1:237-242
    $json = ($VerdictObj | ConvertTo-Json -Depth 6 -Compress)
    [Console]::Out.Write($json + "`n")
    return $json
}

function Persist-Verdict {
    param(
        [string]$GateName,
        [string]$Json
    )
    # Resolve repo root one level above $PSScriptRoot (scripts/).
    # Source: scripts/validate-windows-msi-contract.ps1:44
    $repoRoot   = Split-Path -Parent $PSScriptRoot
    $verdictDir = Join-Path $repoRoot ".nono-runtime\verdicts"
    # Source: scripts/windows-test-harness.ps1:12
    New-Item -ItemType Directory -Force -Path $verdictDir | Out-Null
    $verdictFile = Join-Path $verdictDir "$GateName.json"
    Set-Content -Path $verdictFile -Value $Json -Encoding UTF8 -NoNewline
}

# ---------------------------------------------------------------------------
# Gate auto-discovery (D-04)
# ---------------------------------------------------------------------------

$gatesDir = Join-Path $PSScriptRoot "gates"

if (-not (Test-Path -LiteralPath $gatesDir)) {
    [Console]::Error.WriteLine("[verify-dark] harness-internal error: gates directory not found: $gatesDir")
    exit 1
}

$gateFiles = Get-ChildItem -Path $gatesDir -Filter "*.ps1" -File | Sort-Object Name
$discoveredGates = @{}
foreach ($f in $gateFiles) {
    $name = [System.IO.Path]::GetFileNameWithoutExtension($f.Name)
    $discoveredGates[$name] = $f.FullName
}

# ---------------------------------------------------------------------------
# Determine run mode
# ---------------------------------------------------------------------------

# Treat "no -Gate and no -All" the same as -All (Phase 81 formalizes --all rollup;
# per-gate dispatch loop is the reusable primitive).
if (-not $Gate -and -not $All) {
    $All = $true
}

# ---------------------------------------------------------------------------
# Single-gate dispatch (D-05, D-06, D-07, D-08)
# ---------------------------------------------------------------------------

function Invoke-SingleGate {
    param([string]$GateName)

    # Resolve gate file — unknown gate is harness-internal error (D-05), not FAIL.
    if (-not $discoveredGates.ContainsKey($GateName)) {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: unknown gate '$GateName'. Discovered gates: $($discoveredGates.Keys -join ', ')")
        exit 1
    }
    $gateFile = $discoveredGates[$GateName]

    # Dot-source the gate file so Test-Precondition and Invoke-Gate enter scope (D-05).
    . $gateFile

    # --- Precondition check FIRST (D-06) ---
    $preconditionReason = Test-Precondition

    if ($null -ne $preconditionReason -and $preconditionReason -ne '') {
        # Host unavailable — emit SKIP_HOST_UNAVAILABLE and exit 3 (D-02, D-06).
        # Never enter Invoke-Gate.
        $verdictObj = Build-Verdict -GateName $GateName `
                                    -Verdict  'SKIP_HOST_UNAVAILABLE' `
                                    -Reason   $preconditionReason `
                                    -Detail   @{}
        $json = Emit-Verdict -VerdictObj $verdictObj
        Persist-Verdict -GateName $GateName -Json $json
        exit 3
    }

    # --- Gate body (D-07: crash -> harness-internal error, never PASS) ---
    try {
        $verdictObj = Invoke-Gate
    }
    catch {
        # Unhandled exception inside Invoke-Gate is a harness-internal error.
        # Print to stderr, DO NOT catch-and-return PASS, DO NOT swallow (D-07).
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate threw for '$GateName': $_")
        exit 4
    }

    if ($null -eq $verdictObj) {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate returned null for '$GateName'")
        exit 4
    }

    # Ensure the verdict object has the required gate field stamped (in case the gate
    # omitted it or used a different casing — the runner owns the final shape).
    $verdictObj['gate']      = $GateName
    $verdictObj['timestamp'] = Get-IsoTimestamp

    # Emit once (D-01) and persist (D-08).
    $json = Emit-Verdict -VerdictObj $verdictObj
    Persist-Verdict -GateName $GateName -Json $json

    # Exit mapping (D-02): PASS=0, FAIL=2, SKIP=3, harness-internal=1/4+
    $verdictStr = $verdictObj['verdict']
    if ($verdictStr -eq 'PASS') {
        exit 0
    } elseif ($verdictStr -eq 'FAIL') {
        exit 2
    } elseif ($verdictStr -eq 'SKIP_HOST_UNAVAILABLE') {
        exit 3
    } else {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: unexpected verdict value '$verdictStr' from gate '$GateName'")
        exit 4
    }
}

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------

if ($Gate) {
    # Single-gate run — full verdict emit + exit (function calls exit directly).
    Invoke-SingleGate -GateName $Gate
} else {
    # All-run: loop each discovered gate (Phase 81 adds the {gates:[...],overall} rollup).
    # Per-gate results are persisted to .nono-runtime/verdicts/<gate>.json as each runs.
    # This loop is intentionally minimal — Phase 81 owns the aggregator/rollup logic (D-03).
    if ($discoveredGates.Count -eq 0) {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: no gate files found in $gatesDir")
        exit 1
    }

    $anyFail = $false
    foreach ($gateName in ($discoveredGates.Keys | Sort-Object)) {
        # Re-source for each gate run in all-mode to avoid function-name collisions
        # between gates (each dot-source overwrites Test-Precondition/Invoke-Gate).
        $gateFile = $discoveredGates[$gateName]
        . $gateFile

        $preconditionReason = Test-Precondition
        if ($null -ne $preconditionReason -and $preconditionReason -ne '') {
            $verdictObj = Build-Verdict -GateName $gateName `
                                        -Verdict  'SKIP_HOST_UNAVAILABLE' `
                                        -Reason   $preconditionReason `
                                        -Detail   @{}
            $json = Emit-Verdict -VerdictObj $verdictObj
            Persist-Verdict -GateName $gateName -Json $json
            continue
        }

        try {
            $verdictObj = Invoke-Gate
        }
        catch {
            [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate threw for '$gateName': $_")
            $anyFail = $true
            continue
        }

        if ($null -eq $verdictObj) {
            [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate returned null for '$gateName'")
            $anyFail = $true
            continue
        }

        $verdictObj['gate']      = $gateName
        $verdictObj['timestamp'] = Get-IsoTimestamp
        $json = Emit-Verdict -VerdictObj $verdictObj
        Persist-Verdict -GateName $gateName -Json $json

        if ($verdictObj['verdict'] -eq 'FAIL') {
            $anyFail = $true
        }
    }

    # Minimal all-run exit: 0 if no FAILs, 2 if any FAIL.
    # Phase 81 replaces this with the {gates:[...],overall}/PASS_WITH_SKIPS rollup (D-03).
    if ($anyFail) {
        exit 2
    } else {
        exit 0
    }
}
