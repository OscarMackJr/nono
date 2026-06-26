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
# LEG 2 — PyPI dry-run  (maturin build + twine check)
# ══════════════════════════════════════════════════════════════════════════════
Write-Banner "PyPI dry-run (maturin build + twine check)"

$maturin = Get-Command maturin -ErrorAction SilentlyContinue
if ($null -eq $maturin) {
    $sig = "maturin: command not found"
    Write-Host "  SKIP [SKIP_HOST_UNAVAILABLE]: $sig" -ForegroundColor Yellow
    Add-Result "pypi.maturin_build" "SKIP" $sig
    Add-Result "pypi.twine_check"   "SKIP" "twine check skipped: maturin absent"
} else {
    Push-Location $NoonoPyDir
    try {
        Write-Host "  maturin build ..." -NoNewline
        $mbOut  = maturin build 2>&1
        $mbExit = $LASTEXITCODE
        if ($mbExit -eq 0) {
            Write-Host "  OK" -ForegroundColor Green
            Add-Result "pypi.maturin_build" "PASS" "maturin build exited 0"

            # Locate twine (CLI or python -m twine fallback)
            $twineAvail = $false
            $twineCmd   = $null
            if (Get-Command twine -ErrorAction SilentlyContinue) {
                $twineAvail = $true
                $twineCmd   = @("twine")
            } else {
                $pyTwineCheck = python -m twine --version 2>&1 | Out-String
                if ($pyTwineCheck -match "twine") {
                    $twineAvail = $true
                    $twineCmd   = @("python", "-m", "twine")
                }
            }

            if (-not $twineAvail) {
                $sig = "twine: command not found (tried: twine CLI; python -m twine)"
                Write-Host "  SKIP [SKIP_HOST_UNAVAILABLE]: $sig" -ForegroundColor Yellow
                Add-Result "pypi.twine_check" "SKIP" $sig
            } else {
                $wheels = Get-ChildItem -Path "target/wheels" -Filter "*.whl" -ErrorAction SilentlyContinue
                if ($null -eq $wheels -or $wheels.Count -eq 0) {
                    Add-Result "pypi.twine_check" "FAIL" "No .whl files found under target/wheels after maturin build"
                } else {
                    Write-Host "  twine check target/wheels/*.whl ..." -NoNewline
                    $tcOut  = & $twineCmd[0] ($twineCmd[1..99] + @("check") + $wheels.FullName) 2>&1
                    $tcExit = $LASTEXITCODE
                    if ($tcExit -eq 0) {
                        Write-Host "  OK" -ForegroundColor Green
                        Add-Result "pypi.twine_check" "PASS" "twine check exited 0"
                    } else {
                        Write-Host "  FAILED (exit $tcExit)" -ForegroundColor Red
                        Write-Host ($tcOut | Out-String)
                        Add-Result "pypi.twine_check" "FAIL" "twine check exited $tcExit"
                    }
                }
            }
        } else {
            Write-Host "  FAILED (exit $mbExit)" -ForegroundColor Red
            Write-Host ($mbOut | Out-String)
            Add-Result "pypi.maturin_build" "FAIL" "maturin build exited $mbExit"
            Add-Result "pypi.twine_check"   "SKIP" "twine check skipped: maturin build failed"
        }
    } finally {
        Pop-Location
    }
}

# ══════════════════════════════════════════════════════════════════════════════
# LEG 3 — npm dry-run  (npm publish --dry-run)
# ══════════════════════════════════════════════════════════════════════════════
Write-Banner "npm dry-run (npm publish --dry-run)"

$npmExe = Get-Command npm -ErrorAction SilentlyContinue
if ($null -eq $npmExe) {
    $sig = "npm: command not found"
    Write-Host "  SKIP [SKIP_HOST_UNAVAILABLE]: $sig" -ForegroundColor Yellow
    Add-Result "npm.dry_run" "SKIP" $sig
} else {
    Push-Location $NonoTsDir
    try {
        Write-Host "  npm publish --dry-run ..." -NoNewline
        $npmOut  = npm publish --dry-run 2>&1
        $npmExit = $LASTEXITCODE
        if ($npmExit -eq 0) {
            $manifest    = $npmOut | Out-String
            $hasIndexJs  = $manifest -match "index\.js"
            $hasIndexDts = $manifest -match "index\.d\.ts"
            if ($hasIndexJs -and $hasIndexDts) {
                Write-Host "  OK (index.js + index.d.ts present)" -ForegroundColor Green
                Add-Result "npm.dry_run" "PASS" "npm publish --dry-run exited 0; index.js + index.d.ts in manifest"
            } else {
                $missing = @()
                if (-not $hasIndexJs)  { $missing += "index.js" }
                if (-not $hasIndexDts) { $missing += "index.d.ts" }
                $msg = "tarball manifest missing: $($missing -join ', ')"
                Write-Host "  FAIL: $msg" -ForegroundColor Red
                Write-Host $manifest
                Add-Result "npm.dry_run" "FAIL" $msg
            }
        } else {
            Write-Host "  FAILED (exit $npmExit)" -ForegroundColor Red
            Write-Host ($npmOut | Out-String)
            Add-Result "npm.dry_run" "FAIL" "npm publish --dry-run exited $npmExit"
        }
    } finally {
        Pop-Location
    }
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

# Overall verdict:
#   PASS  = no hard failures; PRE_PUBLISH_REGISTRY_BLOCKED and SKIPs are not failures
#   FAIL  = at least one hard FAIL (packaging error, unexpected compilation failure, etc.)
if ($HardFailures -eq 0) {
    $blockedCount = ($Results.Values | Where-Object { $_.Status -eq "PRE_PUBLISH_REGISTRY_BLOCKED" }).Count
    $skipCount    = ($Results.Values | Where-Object { $_.Status -eq "SKIP" }).Count
    if ($blockedCount -gt 0 -or $skipCount -gt 0) {
        Write-Host "PASS: Hard failures: 0. Blocked: $blockedCount (pre-publish). Skipped: $skipCount (toolchain absent)." -ForegroundColor Green
    } else {
        Write-Host "PASS: All dry-run checks passed." -ForegroundColor Green
    }
    exit 0
} else {
    Write-Host "FAIL: $HardFailures dry-run check(s) failed." -ForegroundColor Red
    exit 1
}
