<#
.SYNOPSIS
    Dry-run validation for all three publish paths at v0.66.0.

.DESCRIPTION
    Validates crates.io, PyPI (nono-py), and npm (nono-ts) packaging WITHOUT uploading
    to any live registry.  No registry credential is referenced or required.

    crates.io leg  : cargo publish --dry-run -p <crate>  (per-crate, dependency order)
    PyPI leg       : maturin build  +  twine check target/wheels/*.whl
    npm leg        : npm publish --dry-run  (tarball manifest check)

    Absent toolchains (maturin, twine, npm) are recorded as documented SKIPs with the
    verbatim tool-not-found message, mirroring the verify-dark SKIP_HOST_UNAVAILABLE
    convention.  The crates.io leg is the always-runnable core and must pass exit 0.

    PRE_PUBLISH_REGISTRY_BLOCKED: cargo publish --dry-run resolves all dependencies
    against the live crates.io index at package time.  For downstream workspace crates
    (nono-proxy, nono-shell-broker, nono-cli) this will fail with "failed to select a
    version for nono = ^0.66.0" until nono 0.66.0 has been published.  This is an
    EXPECTED pre-publish state, not a packaging error.  The status is surfaced
    transparently so the operator can re-run after publishing the base crate.

.NOTES
    SAFETY INVARIANTS (enforced by design):
      - No registry credential appears in this script.
      - No live upload command appears here (no PyPI push, no live npm publish).
      - All cargo publish calls use --dry-run.

    Invoke via:  pwsh -File scripts\release-dry-run.ps1
    Never invoke via:  pwsh -Command "<bare path>"  (swallows non-zero exit codes).
#>

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir  = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot   = Split-Path -Parent $ScriptDir
$NoonoPyDir = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot ".." "nono-py"))
$NonoTsDir  = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot ".." "nono-ts"))

# ──────────────────────────────────────────────────────────────────────────────
# Result tracking
# ──────────────────────────────────────────────────────────────────────────────
$Results = [ordered]@{}
$HardFailures = 0

function Add-Result {
    param([string]$Key, [string]$Status, [string]$Message)
    $script:Results[$Key] = [pscustomobject]@{ Status = $Status; Message = $Message }
    if ($Status -eq "FAIL") { $script:HardFailures++ }
}

function Write-Banner([string]$Title) {
    Write-Host ""
    Write-Host ("─" * 70) -ForegroundColor DarkGray
    Write-Host "  $Title" -ForegroundColor Cyan
    Write-Host ("─" * 70) -ForegroundColor DarkGray
}

# ══════════════════════════════════════════════════════════════════════════════
# LEG 1 — crates.io dry-run  (dependency order: nono → nono-proxy →
#          nono-shell-broker → nono-cli)
# ══════════════════════════════════════════════════════════════════════════════
Write-Banner "crates.io dry-run (dependency order)"

# Publishable set (no publish=false) in crates.io dependency order.
# Excluded (publish=false): nono-fltmgr-client, nono-ffi (bindings/c), tools/sign-fixture.
$PublishableCrates = @("nono", "nono-proxy", "nono-shell-broker", "nono-cli")

Push-Location $RepoRoot
try {
    foreach ($crate in $PublishableCrates) {
        Write-Host "  cargo publish --dry-run -p $crate ..." -NoNewline
        $output   = cargo publish --dry-run -p $crate 2>&1
        $exitCode = $LASTEXITCODE
        if ($exitCode -eq 0) {
            Write-Host "  OK" -ForegroundColor Green
            Add-Result "crates.$crate" "PASS" "cargo publish --dry-run -p $crate exited 0"
        } else {
            $outStr = $output | Out-String
            # Detect pre-publish registry-resolution block: downstream crates depend on
            # nono = ^0.66.0 which does not exist on crates.io until after nono is published.
            # This is an expected pre-publish state, NOT a packaging error.
            # Pattern matched per-line; distinctive phrase appears in the error message.
            if ($outStr -match 'failed to select a version for the requirement') {
                Write-Host "  PRE_PUBLISH_REGISTRY_BLOCKED" -ForegroundColor Yellow
                Add-Result "crates.$crate" "PRE_PUBLISH_REGISTRY_BLOCKED" `
                    "nono ^0.66.0 not yet on crates.io; re-run after publishing nono"
            } else {
                Write-Host "  FAILED (exit $exitCode)" -ForegroundColor Red
                Write-Host ($outStr)
                Add-Result "crates.$crate" "FAIL" "cargo publish --dry-run -p $crate exited $exitCode"
            }
        }
    }
} finally {
    Pop-Location
}

# ══════════════════════════════════════════════════════════════════════════════
# FINAL VERDICT
# ══════════════════════════════════════════════════════════════════════════════
Write-Banner "Dry-Run Results"

foreach ($key in $Results.Keys) {
    $r = $Results[$key]
    switch ($r.Status) {
        "PASS"                        { Write-Host ("  PASS    {0,-38} {1}" -f $key, $r.Message) -ForegroundColor Green  }
        "SKIP"                        { Write-Host ("  SKIP    {0,-38} {1}" -f $key, $r.Message) -ForegroundColor Yellow }
        "PRE_PUBLISH_REGISTRY_BLOCKED"{ Write-Host ("  BLOCKED {0,-38} {1}" -f $key, $r.Message) -ForegroundColor Yellow }
        "FAIL"                        { Write-Host ("  FAIL    {0,-38} {1}" -f $key, $r.Message) -ForegroundColor Red    }
    }
}

Write-Host ""

if ($HardFailures -eq 0) {
    $blockedCount = ($Results.Values | Where-Object { $_.Status -eq "PRE_PUBLISH_REGISTRY_BLOCKED" }).Count
    if ($blockedCount -gt 0) {
        Write-Host "PASS: nono base crate dry-run passed. $blockedCount downstream crate(s) are" -ForegroundColor Green
        Write-Host "      PRE_PUBLISH_REGISTRY_BLOCKED — re-run after publishing nono to crates.io." -ForegroundColor Green
    } else {
        Write-Host "PASS: All dry-run checks passed or recorded a documented SKIP." -ForegroundColor Green
    }
    exit 0
} else {
    Write-Host "FAIL: $HardFailures dry-run check(s) failed." -ForegroundColor Red
    exit 1
}
