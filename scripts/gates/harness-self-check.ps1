# scripts/gates/harness-self-check.ps1
#
# Phase 76 Plan 02 — harness-self-check gate
#
# CONTRACT (D-05): exports exactly two functions dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object — it MUST NOT call exit (D-02, PATTERNS exit/return
# convention). Only the runner owns exit-code mapping.
#
# Test-Precondition -> $null (preconditions met) | "reason string" (SKIP_HOST_UNAVAILABLE)
# Invoke-Gate       -> verdict object (PASS / FAIL / SKIP_HOST_UNAVAILABLE)
#
# D-11: harness-self-check always returns $null from Test-Precondition — it runs on any
#       Win11 host and is the proof that the framework itself is wired (ROADMAP SC5).
#       Its Invoke-Gate trivially exercises: emit + persist + JSON round-trip.
#
# Reference contract for phases 77-80: copy this two-function shape.
# Assertion-helper idiom: scripts/validate-windows-msi-contract.ps1:100-129 (throw on failure).
# ISO timestamp: scripts/test-windows-shell-write-deny.ps1:60.
# JSON house style: scripts/check-upstream-drift.ps1:224-243 ([ordered]@{}, ConvertTo-Json -Depth N -Compress).

# ---------------------------------------------------------------------------
# Local assertion helpers (throw-on-failure per Assert-Equal/Assert-True idiom)
# ---------------------------------------------------------------------------

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
# Gate contract (D-05)
# ---------------------------------------------------------------------------

function Test-Precondition {
    # D-11: always return $null — the self-check gate runs on any Win11 host.
    # No host probing. Test-Precondition returns $null -> Invoke-Gate runs.
    return $null
}

function Invoke-Gate {
    # D-11: trivially verify framework wiring — emit + persist + JSON round-trip.
    # Returns a PASS verdict on success; a failing assertion throws (surfaces to
    # the runner as a harness-internal error per D-07). NEVER returns PASS on a
    # broken framework. NEVER calls exit (D-02).

    # Build candidate verdict ([ordered]@{} locks key order per D-01)
    $candidate = [ordered]@{
        gate      = 'harness-self-check'
        verdict   = 'PASS'
        reason    = 'framework functional'
        detail    = @{}
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }

    # --- Assertion (a): serialize via ConvertTo-Json -Depth 5 yields a non-empty string ---
    $json = ($candidate | ConvertTo-Json -Depth 5 -Compress)
    Assert-True -Condition ($null -ne $json -and $json.Length -gt 0) `
                -Message   'Assertion (a) failed: ConvertTo-Json produced empty or null string'

    # --- Assertion (b): ConvertFrom-Json round-trips gate and verdict fields ---
    $roundTripped = ($json | ConvertFrom-Json)
    Assert-Equal -Actual   $roundTripped.gate `
                 -Expected 'harness-self-check' `
                 -Message  'Assertion (b) failed: round-tripped gate field mismatch'
    Assert-Equal -Actual   $roundTripped.verdict `
                 -Expected 'PASS' `
                 -Message  'Assertion (b) failed: round-tripped verdict field mismatch'

    # --- Assertion (c): persistence file round-trip ---
    # WR-04 / IN-01: prove the write+read-back round-trip against a TEMP file rather than
    # the canonical .nono-runtime/verdicts/<gate>.json path. The runner (Persist-Verdict)
    # is the single owner of the canonical verdict file; duplicating that write here created
    # two independent path-resolution chains that could silently diverge. Using a temp file
    # keeps this assertion's intent (Set-Content/Get-Content round-trip works) without owning
    # the real verdict path.
    $tempFile = New-TemporaryFile
    try {
        # Write the JSON to the temp file and read it back.
        Set-Content -Path $tempFile.FullName -Value $json -Encoding UTF8 -NoNewline
        Assert-True -Condition (Test-Path -LiteralPath $tempFile.FullName) `
                    -Message   'Assertion (c) failed: round-trip temp file not found'

        $persisted = Get-Content -Path $tempFile.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
        Assert-Equal -Actual   $persisted.gate `
                     -Expected 'harness-self-check' `
                     -Message  'Assertion (c) failed: round-tripped gate field mismatch'
        Assert-Equal -Actual   $persisted.verdict `
                     -Expected 'PASS' `
                     -Message  'Assertion (c) failed: round-tripped verdict field mismatch'
    }
    finally {
        Remove-Item -LiteralPath $tempFile.FullName -Force -ErrorAction SilentlyContinue
    }

    # All assertions passed — return the candidate verdict.
    # The runner stamps gate + timestamp after receiving this object (Invoke-SingleGate lines
    # 150-151); returning them here is redundant but clarifies intent and satisfies the
    # "gate returns a verdict object with gate/verdict fields" contract.
    return $candidate
}
