#Requires -Version 5.1
# Phase 104 (REL-01) local root-availability pre-check.
#
# Fast, free, local way to check whether Microsoft's live Trusted Root Program
# CTL now contains the root this account's current Azure Trusted Signing
# issuing hierarchy chains through (Microsoft Enterprise ID Root CA 2021),
# BEFORE burning an outward-facing smoke-workflow CI dispatch to find out the
# hard way. 104-RESEARCH.md Primary Investigation step 5 confirmed, via a live
# `certutil -generateSSTFromWU` pull this session, that this root is currently
# ABSENT from the public CTL — the empirical, external cause of SIGN-03's
# UnknownError/UntrustedRoot failure. This script is read-only and
# informational only: its PRESENT/ABSENT result is never a pass/fail input to
# Assert-TrustedSignature or any other gate — only the operator's own
# poll-until-green judgment in Plan 104-03 consumes it.
#
# NEVER add a TLS-bypass / insecure flag to the certutil invocation below —
# fail-secure per CLAUDE.md; this script only ever reads the live, standard
# Windows Update AuthRoot CTL endpoint, the same one every Windows host uses.
#
# Usage: pwsh -File scripts/azure/check-trusted-signing-root.ps1 [-Thumbprint <40-hex-char-sha1>]
# Exit 0 = PRESENT (root found — safe to proceed to a real smoke re-dispatch)
# Exit 1 = ABSENT  (root not found — still blocked, do not burn a CI dispatch yet)
# Exit 2 = harness-internal error (could not fetch/parse the CTL, OR the caller
#          passed a malformed thumbprint / unexpected extra arguments) — distinct
#          from a confirmed-absent result; never conflate the two.

param(
    [string]$Thumbprint = '991D364E97882715B80ED978F53E1F35DC2F07C2',  # Microsoft Enterprise ID Root CA 2021 (SHA-1)
    # Capture anything the caller passes beyond -Thumbprint so a stray copy-paste
    # (e.g. the tail of an instruction line: `... .ps1 Exit 1 = still`) is caught
    # and rejected LOUDLY, never silently degraded into a checked-the-wrong-thing
    # ABSENT result that looks identical to a real one.
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$UnexpectedArgs
)

$ErrorActionPreference = 'Stop'

# --- Input validation (fail loudly, never a misleading ABSENT) -------------------
if ($UnexpectedArgs -and $UnexpectedArgs.Count -gt 0) {
    [Console]::Error.WriteLine("ERROR: unexpected argument(s): '$($UnexpectedArgs -join ' ')'. Usage: pwsh -File scripts/azure/check-trusted-signing-root.ps1 [-Thumbprint <40-hex-char-sha1>]. Refusing to run — a stray argument must not silently produce a misleading ABSENT result.")
    exit 2
}

# Normalize common thumbprint formats (spaces/colons) then require exactly 40 hex
# chars — a SHA-1 certificate thumbprint. Garbage like 'Exit' fails here instead of
# being searched-for-and-not-found (which would masquerade as a real ABSENT).
$Thumbprint = ($Thumbprint -replace '[\s:]', '').ToUpperInvariant()
if ($Thumbprint -notmatch '^[0-9A-F]{40}$') {
    [Console]::Error.WriteLine("ERROR: -Thumbprint must be exactly 40 hexadecimal characters (a SHA-1 certificate thumbprint). Got: '$Thumbprint'. Refusing to run — an invalid thumbprint must not silently produce a misleading ABSENT result.")
    exit 2
}

$sstPath = $null

try {
    $tempDir = [System.IO.Path]::GetTempPath()
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $sstPath = Join-Path $tempDir "nono-trusted-signing-root-check-$timestamp.sst"

    $certutilPath = (Get-Command certutil.exe -ErrorAction SilentlyContinue).Source
    if (-not $certutilPath) {
        [Console]::Error.WriteLine("ERROR: certutil.exe not found on PATH.")
        exit 2
    }

    Write-Host "Fetching the live Microsoft Trusted Root Program CTL via certutil -generateSSTFromWU (no TLS bypass, standard Windows Update AuthRoot endpoint)..."

    $stdoutFile = New-TemporaryFile
    $stderrFile = New-TemporaryFile
    try {
        # Start-Process -PassThru -Wait (not a bare `&` + $LASTEXITCODE call) —
        # mirrors Invoke-SignToolVerify's convention in verify-authenticode.ps1
        # (101-RESEARCH.md Pitfall 3: bare $LASTEXITCODE is unsafe once any
        # other command runs between the invocation and the check).
        $proc = Start-Process -FilePath $certutilPath `
            -ArgumentList @('-generateSSTFromWU', "`"$sstPath`"") `
            -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput $stdoutFile.FullName `
            -RedirectStandardError $stderrFile.FullName

        if ($proc.ExitCode -ne 0) {
            $stderrText = Get-Content -Raw -LiteralPath $stderrFile.FullName -ErrorAction SilentlyContinue
            [Console]::Error.WriteLine("ERROR: certutil -generateSSTFromWU failed with exit code $($proc.ExitCode). This is a harness-internal failure (could not fetch the CTL at all) — distinct from a confirmed-absent root. Stderr: $stderrText")
            exit 2
        }
    }
    finally {
        Remove-Item -LiteralPath $stdoutFile.FullName -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $stderrFile.FullName -Force -ErrorAction SilentlyContinue
    }

    if (-not (Test-Path -LiteralPath $sstPath -PathType Leaf)) {
        [Console]::Error.WriteLine("ERROR: certutil reported success but the .sst file was not created at $sstPath.")
        exit 2
    }

    $collection = [System.Security.Cryptography.X509Certificates.X509Certificate2Collection]::new()
    $collection.Import($sstPath)

    $certCount = $collection.Count
    $found = $false
    foreach ($cert in $collection) {
        if ($cert.Thumbprint -eq $Thumbprint) {
            $found = $true
            break
        }
    }

    if ($found) {
        Write-Host "PRESENT: thumbprint $Thumbprint (Microsoft Enterprise ID Root CA 2021) found in the live Trusted Root Program CTL ($certCount certificates checked). Safe to proceed to a real smoke re-dispatch."
        exit 0
    }
    else {
        Write-Host "ABSENT: thumbprint $Thumbprint (Microsoft Enterprise ID Root CA 2021) NOT found in the live Trusted Root Program CTL ($certCount certificates checked). Still blocked — do not burn a CI dispatch yet."
        exit 1
    }
}
catch {
    [Console]::Error.WriteLine("ERROR: $_")
    exit 2
}
finally {
    if ($sstPath -and (Test-Path -LiteralPath $sstPath -PathType Leaf)) {
        Remove-Item -LiteralPath $sstPath -Force -ErrorAction SilentlyContinue
    }
}
