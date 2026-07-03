#Requires -Version 5.1
# Phase 104 (REL-01) Pitfall 104-A regression guard.
#
# Fast, local, static check: asserts release.yml's `publish-crates` job is
# neutralized (job-level `if: false` within its bounded line range) AND that no
# OTHER enabled job in the file references the pre-Phase-102-rename crate
# selectors `-p nono` / `-p nono-proxy` / `-p nono-cli`. Phase 102 renamed the
# actual [package] names to nono-sandbox / nono-sandbox-proxy / nono-sandbox-cli
# (grep-confirmed against the current tree in 104-RESEARCH.md); the stale
# selectors inside the neutralized `publish-crates` job body are expected and
# tolerated (Phase 105/PUB-02 rewrites them) — this guard only fails on a
# reintroduction OUTSIDE that bounded, neutralized range.
#
# Usage: pwsh -File scripts/verify-release-yml-publish-selectors.ps1
# Exit 0 = PASS, exit 1 = FAIL (regression detected or guard missing), exit 2 =
# harness error (file not found, unexpected shape).

$ErrorActionPreference = 'Stop'

$releaseYmlPath = Join-Path $PSScriptRoot '..\.github\workflows\release.yml'

try {
    if (-not (Test-Path -LiteralPath $releaseYmlPath -PathType Leaf)) {
        [Console]::Error.WriteLine("ERROR: release.yml not found at $releaseYmlPath")
        exit 2
    }

    $lines = Get-Content -LiteralPath $releaseYmlPath

    # Locate the `publish-crates:` job header (2-space indent, top-level job key).
    $publishCratesIdx = -1
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^  publish-crates:\s*$') {
            $publishCratesIdx = $i
            break
        }
    }
    if ($publishCratesIdx -lt 0) {
        [Console]::Error.WriteLine("FAIL: could not find the 'publish-crates:' job header in release.yml — guard cannot verify neutralization.")
        exit 1
    }

    # Locate the next 2-space-indented top-level job key after publish-crates
    # (expected to be `update-homebrew-core:`, but detected generically).
    $nextJobIdx = $lines.Count
    for ($i = $publishCratesIdx + 1; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^  [A-Za-z0-9_-]+:\s*$') {
            $nextJobIdx = $i
            break
        }
    }

    # Bounded range covering the publish-crates job body (inclusive of header,
    # exclusive of the next job's header).
    $rangeStart = $publishCratesIdx
    $rangeEnd = $nextJobIdx - 1

    # 1. Assert the neutralization guard (`if: false`) is present within range.
    $hasIfFalse = $false
    for ($i = $rangeStart; $i -le $rangeEnd; $i++) {
        if ($lines[$i] -match '^\s*if:\s*false\s*$') {
            $hasIfFalse = $true
            break
        }
    }
    if (-not $hasIfFalse) {
        [Console]::Error.WriteLine("FAIL: 'publish-crates' job (lines $($rangeStart + 1)-$($rangeEnd + 1)) does not contain a job-level 'if: false' guard. Pitfall 104-A is NOT neutralized.")
        exit 1
    }

    # 2. Scan every OTHER line (outside the bounded, neutralized range),
    # skipping comment lines, for the exact stale selector substrings.
    $staleSubstrings = @(
        'cargo publish -p nono --allow-dirty',
        'cargo publish -p nono-proxy --allow-dirty',
        'cargo publish -p nono-cli --allow-dirty'
    )

    $violations = @()
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($i -ge $rangeStart -and $i -le $rangeEnd) {
            continue  # inside the neutralized publish-crates job — tolerated
        }
        $trimmed = $lines[$i].Trim()
        if ($trimmed.StartsWith('#')) {
            continue  # comment line — not an executable reference
        }
        foreach ($needle in $staleSubstrings) {
            if ($lines[$i].Contains($needle)) {
                $violations += [pscustomobject]@{
                    LineNumber = $i + 1
                    Text       = $lines[$i]
                    Needle     = $needle
                }
            }
        }
    }

    if ($violations.Count -gt 0) {
        [Console]::Error.WriteLine("FAIL: found $($violations.Count) stale publish-selector reference(s) OUTSIDE the neutralized publish-crates job:")
        foreach ($v in $violations) {
            [Console]::Error.WriteLine("  Line $($v.LineNumber): $($v.Text.Trim())  (matched: $($v.Needle))")
        }
        exit 1
    }

    Write-Host "PASS: 'publish-crates' job (lines $($rangeStart + 1)-$($rangeEnd + 1)) is neutralized via job-level 'if: false'; no enabled job references the stale -p nono / -p nono-proxy / -p nono-cli selectors."
    exit 0
}
catch {
    [Console]::Error.WriteLine("ERROR: $_")
    exit 2
}
