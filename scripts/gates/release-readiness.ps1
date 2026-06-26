# scripts/gates/release-readiness.ps1
#
# Phase 97 Plan 04 — release-readiness gate
#
# CONTRACT: exports exactly two functions dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object — it MUST NOT call exit.
#
# Test-Precondition -> $null (run on any Win11 host; static-consistency check,
#                      no external host dependency)
# Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                      verdict = PASS | FAIL
#
# Design rules:
#   - Policy violations (wrong version, private path staged) return FAIL verdict.
#   - Infrastructure failures (command not found, JSON parse error) throw, so the
#     harness classifies them as harness-internal errors (exit 4) — not FAIL (exit 2)
#     and never PASS (exit 0).
#   - This gate NEVER calls exit. The runner (verify-dark.ps1) owns exit-code mapping.
#   - Keep fast: no network, no build, no compilation.
#
# Invocation (single-gate):
#   pwsh -File scripts/verify-dark.ps1 -Gate release-readiness
#     exit 0 = PASS, exit 2 = FAIL, exit 3 = SKIP, exit 4 = harness error

# ---------------------------------------------------------------------------
# Gate contract — Test-Precondition
# ---------------------------------------------------------------------------

function Test-Precondition {
    # Static consistency check. Runs on any Win11 dev host — cargo and git are
    # dev-host invariants; no optional external service is required.
    # Return $null to indicate the gate is always eligible to run.
    return $null
}

# ---------------------------------------------------------------------------
# Gate contract — Invoke-Gate
# ---------------------------------------------------------------------------

function Invoke-Gate {
    # Allow native command non-zero exits without throwing; $LASTEXITCODE checked
    # explicitly for each native call. Mirrors verify-dark PATTERNS.md guidance
    # ("gates that shell out may locally set $ErrorActionPreference = 'Continue'
    # inside Invoke-Gate").
    $savedEAP = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {

        # -----------------------------------------------------------------------
        # INFRASTRUCTURE SETUP
        # Failures here are broken preconditions → throw (harness-internal error).
        # -----------------------------------------------------------------------

        # Verify required commands are available before any native call.
        if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
            throw "'cargo' is not on PATH — cannot run cargo metadata"
        }
        if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
            throw "'git' is not on PATH — cannot check git staging state"
        }

        # Resolve repo root via git (unambiguous, independent of $PSScriptRoot
        # context when this function is called from the dot-source chain).
        $repoRootRaw = (git rev-parse --show-toplevel 2>$null) | Out-String
        if ($LASTEXITCODE -ne 0) {
            throw "git rev-parse --show-toplevel failed (exit $LASTEXITCODE) — not inside a git repository"
        }
        $repoRoot = $repoRootRaw.Trim()

        # -----------------------------------------------------------------------
        # ASSERTION TRACKING
        # -----------------------------------------------------------------------
        $failedChecks = [System.Collections.Generic.List[string]]::new()
        $detail       = [ordered]@{}

        $targetVersion       = '0.66.0'
        $upstreamHighest     = '0.65.1'
        $versionFamilyCrates = @(
            'nono',
            'nono-cli',
            'nono-proxy',
            'nono-shell-broker',
            'nono-fltmgr-client',
            'nono-ffi'
        )

        # -----------------------------------------------------------------------
        # ASSERTION (a): cargo metadata reports all version-family crates at 0.66.0
        # -----------------------------------------------------------------------
        Push-Location $repoRoot
        try {
            $metaLines = (cargo metadata --format-version 1 --no-deps 2>$null)
            $metaExit  = $LASTEXITCODE
        } finally {
            Pop-Location
        }
        if ($metaExit -ne 0) {
            throw "cargo metadata failed (exit $metaExit) — workspace may not be parseable"
        }
        $metaJson = ($metaLines | Out-String)
        $meta     = $null
        try {
            $meta = $metaJson | ConvertFrom-Json
        } catch {
            throw "cargo metadata output could not be parsed as JSON: $_"
        }
        if ($null -eq $meta -or $null -eq $meta.packages) {
            throw "cargo metadata returned no packages field — unexpected output shape"
        }

        $foundVersions = @{}
        foreach ($pkg in $meta.packages) {
            if ($versionFamilyCrates -contains $pkg.name) {
                $foundVersions[$pkg.name] = $pkg.version
            }
        }
        $versionMismatches = [System.Collections.Generic.List[string]]::new()
        foreach ($crate in $versionFamilyCrates) {
            if (-not $foundVersions.ContainsKey($crate)) {
                $versionMismatches.Add("${crate}: not found in cargo metadata output")
            } elseif ($foundVersions[$crate] -ne $targetVersion) {
                $versionMismatches.Add("${crate}: expected $targetVersion, found $($foundVersions[$crate])")
            }
        }
        if ($versionMismatches.Count -gt 0) {
            $failedChecks.Add('version-family')
            $detail['version_family'] = $versionMismatches.ToArray()
        } else {
            $detail['version_family'] = "$targetVersion confirmed for all $($versionFamilyCrates.Count) version-family crates"
        }

        # -----------------------------------------------------------------------
        # ASSERTION (b): no 0.62.2 version string in any tracked workspace Cargo.toml
        # -----------------------------------------------------------------------
        Push-Location $repoRoot
        try {
            $trackedTomls = (git ls-files -- '*.toml' 2>$null)
            $lsExit       = $LASTEXITCODE
        } finally {
            Pop-Location
        }
        if ($lsExit -ne 0) {
            throw "git ls-files failed (exit $lsExit)"
        }
        $staleRefs = [System.Collections.Generic.List[string]]::new()
        foreach ($relPath in ($trackedTomls | Where-Object { $_ -match '(^|/)Cargo\.toml$' })) {
            $absPath = [System.IO.Path]::Combine($repoRoot, $relPath.TrimStart('/'))
            if (Test-Path -LiteralPath $absPath) {
                $content = Get-Content -Path $absPath -Raw -Encoding UTF8
                if ($content -match '0\.62\.2') {
                    $staleRefs.Add($relPath)
                }
            }
        }
        if ($staleRefs.Count -gt 0) {
            $failedChecks.Add('stale-0.62.2')
            $detail['stale_062_2'] = $staleRefs.ToArray()
        } else {
            $detail['stale_062_2'] = 'no 0.62.2 version string found in tracked Cargo.toml files'
        }

        # -----------------------------------------------------------------------
        # ASSERTION (c): leapfrog — 0.66.0 is strictly greater than upstream 0.65.1
        # -----------------------------------------------------------------------
        $releaseVer  = [Version]$targetVersion
        $upstreamVer = [Version]$upstreamHighest
        if ($releaseVer -le $upstreamVer) {
            $failedChecks.Add('leapfrog')
            $detail['leapfrog'] = "FAIL: $targetVersion is not strictly greater than upstream highest $upstreamHighest"
        } else {
            $detail['leapfrog'] = "$targetVersion > $upstreamHighest (leapfrog confirmed)"
        }

        # -----------------------------------------------------------------------
        # ASSERTION (d): no build_notes/ or .gsd/ paths in staging area or tracked set
        # -----------------------------------------------------------------------
        Push-Location $repoRoot
        try {
            $stagedFiles    = (git diff --cached --name-only 2>$null)
            $diffExit       = $LASTEXITCODE
            $trackedPrivate = (git ls-files -- 'build_notes/' '.gsd/' 2>$null)
            $lsPrivExit     = $LASTEXITCODE
        } finally {
            Pop-Location
        }
        if ($diffExit -ne 0) {
            throw "git diff --cached --name-only failed (exit $diffExit)"
        }
        if ($lsPrivExit -ne 0) {
            throw "git ls-files (private path check) failed (exit $lsPrivExit)"
        }

        $privateViolations = [System.Collections.Generic.List[string]]::new()
        $allPaths = @($stagedFiles) + @($trackedPrivate)
        foreach ($p in $allPaths) {
            if ($null -ne $p -and $p -ne '' -and ($p -match '^build_notes/' -or $p -match '^\.gsd/')) {
                $privateViolations.Add($p)
            }
        }
        $uniqueViolations = @($privateViolations | Sort-Object -Unique)
        if ($uniqueViolations.Count -gt 0) {
            $failedChecks.Add('private-paths')
            $detail['private_paths'] = $uniqueViolations
        } else {
            $detail['private_paths'] = 'no build_notes/ or .gsd/ paths in staging area or tracked set'
        }

        # -----------------------------------------------------------------------
        # ASSERTION (e): Cargo.lock contains 0.66.0 workspace entries
        # -----------------------------------------------------------------------
        $lockFile = [System.IO.Path]::Combine($repoRoot, 'Cargo.lock')
        if (-not (Test-Path -LiteralPath $lockFile)) {
            throw "Cargo.lock not found at $lockFile — workspace is not materialized"
        }
        $lockContent = Get-Content -Path $lockFile -Raw -Encoding UTF8
        if ($lockContent -notmatch '0\.66\.0') {
            $failedChecks.Add('cargo-lock-version')
            $detail['cargo_lock'] = "FAIL: Cargo.lock does not contain any 0.66.0 entry"
        } else {
            $matchCount = ([regex]::Matches($lockContent, [regex]::Escape('0.66.0'))).Count
            $detail['cargo_lock'] = "0.66.0 found ($matchCount occurrence(s)) in Cargo.lock"
        }

        # -----------------------------------------------------------------------
        # VERDICT
        # -----------------------------------------------------------------------
        $timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')

        if ($failedChecks.Count -gt 0) {
            return [ordered]@{
                gate      = 'release-readiness'
                verdict   = 'FAIL'
                reason    = "release-readiness checks failed: $($failedChecks -join ', ')"
                detail    = $detail
                timestamp = $timestamp
            }
        }

        return [ordered]@{
            gate      = 'release-readiness'
            verdict   = 'PASS'
            reason    = 'all release-readiness checks passed (version-family, no-stale-0.62.2, leapfrog, no-private-paths, cargo-lock)'
            detail    = $detail
            timestamp = $timestamp
        }

    } finally {
        $ErrorActionPreference = $savedEAP
    }
}
