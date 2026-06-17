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

# NOTE: emit (ConvertTo-Json -Depth 6 -Compress + [Console]::Out.Write($json + "`n"))
# is now inlined at each call site so the emit happens AFTER Persist-Verdict succeeds
# (WR-04: the persisted file-of-record must exist before the stdout line the consumer
# reads). The -Depth 6 / -Compress / [Console]::Out.Write idiom is preserved verbatim;
# Source: scripts/check-upstream-drift.ps1:237-242.

function Persist-Verdict {
    param(
        [string]$GateName,
        [string]$Json
    )
    # WR-04: Persistence must run BEFORE the verdict is emitted to stdout, and a
    # write failure must be classified as a harness-internal error by the caller —
    # never a bare terminating error mid-exit-mapping. This function therefore
    # CATCHES its own failure and returns $false (caller maps that to exit 4),
    # rather than letting $ErrorActionPreference="Stop" abort the script after
    # emit but before the verdict->exit mapping.
    # Returns $true on success, $false on failure.
    try {
        # Resolve repo root one level above $PSScriptRoot (scripts/).
        # Source: scripts/validate-windows-msi-contract.ps1:44
        $repoRoot   = Split-Path -Parent $PSScriptRoot
        $verdictDir = Join-Path $repoRoot ".nono-runtime\verdicts"
        # Source: scripts/windows-test-harness.ps1:12
        New-Item -ItemType Directory -Force -Path $verdictDir | Out-Null
        $verdictFile = Join-Path $verdictDir "$GateName.json"
        Set-Content -Path $verdictFile -Value $Json -Encoding UTF8 -NoNewline
        return $true
    }
    catch {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: failed to persist verdict for '$GateName': $_")
        return $false
    }
}

# ---------------------------------------------------------------------------
# Verdict normalization + classification (shared by single-gate AND all-run)
# ---------------------------------------------------------------------------
# IN-02 / CR-01 / CR-02 / WR-01: a single helper owns the verdict->classification
# contract so the two dispatch paths cannot drift. It (a) normalizes a stray-array
# return (WR-01: any uncaptured pipeline output in Invoke-Gate appends to the return,
# producing an Object[]), (b) type-checks that the result is a dictionary, and
# (c) validates the verdict string against the known set.
#
# Returns one of: 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' | 'HARNESS_ERROR'.
# 'HARNESS_ERROR' covers null, wrong-type, and unknown/garbage verdict values — the
# caller maps it to a reserved exit (4), NEVER to FAIL (2) and NEVER to a silent PASS (0).

function Normalize-VerdictObject {
    param(
        $VerdictObj,
        [string]$GateName
    )
    # WR-01: collapse a stray-array return to its last element (the returned dict),
    # then require a dictionary. Anything else is a harness-internal error ($null).
    if ($VerdictObj -is [array]) {
        $VerdictObj = $VerdictObj[-1]
    }
    if ($null -eq $VerdictObj -or -not ($VerdictObj -is [System.Collections.IDictionary])) {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate did not return a single verdict object for '$GateName'")
        return $null
    }
    return $VerdictObj
}

function Resolve-VerdictClass {
    param(
        $VerdictObj,
        [string]$GateName
    )
    # CR-02: validate against the known set; unknown/garbage/empty -> HARNESS_ERROR.
    $verdictStr = $VerdictObj['verdict']
    switch ($verdictStr) {
        'PASS'                  { return 'PASS' }
        'FAIL'                  { return 'FAIL' }
        'SKIP_HOST_UNAVAILABLE' { return 'SKIP_HOST_UNAVAILABLE' }
        default {
            [Console]::Error.WriteLine("[verify-dark] harness-internal error: unexpected verdict value '$verdictStr' from gate '$GateName'")
            return 'HARNESS_ERROR'
        }
    }
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

# WR-03: -Gate and -All are mutually exclusive. Silently ignoring -All when -Gate
# is also supplied is a correctness hazard for a verdict harness whose output drives
# automated gating (the operator may believe all gates ran). Fail as a harness-internal
# error, never a FAIL/PASS verdict.
if ($Gate -and $All) {
    [Console]::Error.WriteLine("[verify-dark] harness-internal error: -Gate and -All are mutually exclusive")
    exit 1
}

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
    # WR-02: a thrown Test-Precondition is a harness-internal error (exit 4), never a
    # FAIL/PASS verdict. With $ErrorActionPreference="Stop" an uncaught throw here would
    # otherwise terminate the script with exit 1 and no diagnostic verdict.
    try {
        $preconditionReason = Test-Precondition
    }
    catch {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: Test-Precondition threw for '$GateName': $_")
        exit 4
    }

    if ($null -ne $preconditionReason -and $preconditionReason -ne '') {
        # Host unavailable — emit SKIP_HOST_UNAVAILABLE and exit 3 (D-02, D-06).
        # Never enter Invoke-Gate.
        $verdictObj = Build-Verdict -GateName $GateName `
                                    -Verdict  'SKIP_HOST_UNAVAILABLE' `
                                    -Reason   $preconditionReason `
                                    -Detail   @{}
        # WR-04: persist BEFORE emit so the file-of-record exists before the consumer
        # sees the stdout line; a persist failure is a harness-internal error (exit 4).
        $json = ($verdictObj | ConvertTo-Json -Depth 6 -Compress)
        if (-not (Persist-Verdict -GateName $GateName -Json $json)) {
            exit 4
        }
        [Console]::Out.Write($json + "`n")
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

    # WR-01: normalize a stray-array return and require a dictionary (shared helper).
    $verdictObj = Normalize-VerdictObject -VerdictObj $verdictObj -GateName $GateName
    if ($null -eq $verdictObj) {
        exit 4
    }

    # Ensure the verdict object has the required gate field stamped (in case the gate
    # omitted it or used a different casing — the runner owns the final shape).
    $verdictObj['gate']      = $GateName
    $verdictObj['timestamp'] = Get-IsoTimestamp

    # Exit mapping (D-02) via shared classifier: PASS=0, FAIL=2, SKIP=3, else=4.
    $verdictClass = Resolve-VerdictClass -VerdictObj $verdictObj -GateName $GateName

    # WR-04: persist BEFORE emit; a persist failure is a harness-internal error (exit 4).
    $json = ($verdictObj | ConvertTo-Json -Depth 6 -Compress)
    if (-not (Persist-Verdict -GateName $GateName -Json $json)) {
        exit 4
    }
    [Console]::Out.Write($json + "`n")

    switch ($verdictClass) {
        'PASS'                  { exit 0 }
        'FAIL'                  { exit 2 }
        'SKIP_HOST_UNAVAILABLE' { exit 3 }
        default                 { exit 4 }   # HARNESS_ERROR (diagnostic already printed)
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

    # CR-01: track harness-internal errors SEPARATELY from gate FAILs. A thrown
    # Invoke-Gate / Test-Precondition, a null/array/non-dict return, or an unknown
    # verdict value is a harness-internal error and MUST map to a reserved exit (4),
    # NEVER to exit 2 (FAIL) and NEVER contribute to a silent exit 0 (PASS).
    $anyFail      = $false
    $harnessError = $false
    foreach ($gateName in ($discoveredGates.Keys | Sort-Object)) {
        # Re-source for each gate run in all-mode to avoid function-name collisions
        # between gates (each dot-source overwrites Test-Precondition/Invoke-Gate).
        $gateFile = $discoveredGates[$gateName]
        . $gateFile

        # WR-02: a thrown Test-Precondition skips THIS gate as a harness-internal error
        # and continues the sweep — it must NOT abort the entire all-run loop and drop
        # coverage of every subsequent gate.
        try {
            $preconditionReason = Test-Precondition
        }
        catch {
            [Console]::Error.WriteLine("[verify-dark] harness-internal error: Test-Precondition threw for '$gateName': $_")
            $harnessError = $true
            continue
        }

        if ($null -ne $preconditionReason -and $preconditionReason -ne '') {
            $verdictObj = Build-Verdict -GateName $gateName `
                                        -Verdict  'SKIP_HOST_UNAVAILABLE' `
                                        -Reason   $preconditionReason `
                                        -Detail   @{}
            # WR-04: persist before emit; a persist failure is a harness-internal error.
            $json = ($verdictObj | ConvertTo-Json -Depth 6 -Compress)
            if (-not (Persist-Verdict -GateName $gateName -Json $json)) {
                $harnessError = $true
                continue
            }
            [Console]::Out.Write($json + "`n")
            continue
        }

        try {
            $verdictObj = Invoke-Gate
        }
        catch {
            [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate threw for '$gateName': $_")
            $harnessError = $true
            continue
        }

        # WR-01: normalize a stray-array return and require a dictionary.
        $verdictObj = Normalize-VerdictObject -VerdictObj $verdictObj -GateName $gateName
        if ($null -eq $verdictObj) {
            $harnessError = $true
            continue
        }

        $verdictObj['gate']      = $gateName
        $verdictObj['timestamp'] = Get-IsoTimestamp

        # CR-02: validate the verdict against the known set via the shared classifier;
        # an unknown/garbage value is a harness-internal error, NOT a silent non-FAIL PASS.
        $verdictClass = Resolve-VerdictClass -VerdictObj $verdictObj -GateName $gateName

        # WR-04: persist before emit; a persist failure is a harness-internal error.
        $json = ($verdictObj | ConvertTo-Json -Depth 6 -Compress)
        if (-not (Persist-Verdict -GateName $gateName -Json $json)) {
            $harnessError = $true
            continue
        }
        [Console]::Out.Write($json + "`n")

        switch ($verdictClass) {
            'FAIL'          { $anyFail = $true }
            'HARNESS_ERROR' { $harnessError = $true }
            # 'PASS' / 'SKIP_HOST_UNAVAILABLE' contribute nothing to the failure state.
        }
    }

    # Minimal all-run exit (Phase 81 owns the {gates:[...],overall}/PASS_WITH_SKIPS
    # rollup — D-03). Precedence: harness-internal error (4) > gate FAIL (2) > PASS (0).
    # CR-01/CR-02: a harness-internal error NEVER reads as FAIL or as a silent PASS.
    if ($harnessError) {
        exit 4
    } elseif ($anyFail) {
        exit 2
    } else {
        exit 0
    }
}
