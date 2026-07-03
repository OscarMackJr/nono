# scripts/verify-authenticode.ps1
#
# Phase 101 Plan 01 (SIGN-02): a single, dot-sourceable Authenticode verify helper,
# shared by all three CI verify sites (release.yml Site 1 + Site 2, and
# trusted-signing-smoke.yml) plus Phase 103's non-CI reuse.
#
# This file is a PURE FUNCTION LIBRARY: no top-level executable statements, no
# `$env:GITHUB_*` / `$env:CI` reads anywhere in any function body (Phase 103 reuses this
# helper outside CI). Every parameter (path, mode) is passed explicitly by the caller —
# per D-03, the helper never infers Strict/Informational from a file extension.
#
# D-01: Assert-TrustedSignature's Strict branch is a dual-engine AND-gate —
#       Get-AuthenticodeSignature AND `signtool verify /pa /v` must both pass.
# D-02: a classified transient (RevocationStatusUnknown / OfflineRevocation /
#       PartialChain, with UntrustedRoot NOT set) retries up to 3 attempts with
#       backoff, then still fails closed if unresolved. Retry never upgrades a fail
#       to a pass. A genuine UntrustedRoot never retries.
# D-03: -Mode is Mandatory with ValidateSet('Strict','Informational') and no default —
#       a call site that omits -Mode throws (fail-closed), never silently picks a mode.
# D-04: on any non-Valid result, dump a full chain-introspection diagnostic to logs —
#       ONLY on the failure path; the happy path stays quiet (a single Write-Host line).

# -----------------------------------------------------------------------------
# Find-Signtool
# -----------------------------------------------------------------------------
# Reused verbatim from scripts/sign-windows-artifacts.ps1:22-58 — live, already-proven
# repo code for locating signtool.exe under the Windows SDK on windows-latest, where
# it is not on PATH by default. Strictly preferred over re-deriving the probe from
# scratch (101-PATTERNS.md).
function Find-Signtool {
    # Prefer signtool.exe if it is already on PATH.
    $onPath = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($null -ne $onPath) {
        return $onPath.Source
    }

    # GitHub-hosted Windows runners ship the Windows SDK but do NOT put signtool.exe
    # on PATH — it lives under Windows Kits\10\bin\<sdk-version>\<arch>\signtool.exe.
    # Search the SDK install roots, prefer the x64 build and the highest SDK version.
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "${env:ProgramFiles}\Windows Kits\10\bin"
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }

    $candidates = foreach ($root in $roots) {
        Get-ChildItem -Path $root -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' }
    }
    # Fall back to any arch if no x64 build was found.
    if ($null -eq $candidates -or @($candidates).Count -eq 0) {
        $candidates = foreach ($root in $roots) {
            Get-ChildItem -Path $root -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue
        }
    }

    $chosen = @($candidates) | Sort-Object {
        # Directory layout: ...\bin\<version>\<arch>\signtool.exe -> version is the grandparent.
        $verName = $_.Directory.Parent.Name
        try { [version]$verName } catch { [version]"0.0.0.0" }
    } -Descending | Select-Object -First 1

    if ($null -eq $chosen) {
        throw "signtool.exe not found on PATH or under the Windows SDK (Windows Kits\10\bin). The Windows SDK must be installed on the runner."
    }
    return $chosen.FullName
}

# -----------------------------------------------------------------------------
# Invoke-SignToolVerify
# -----------------------------------------------------------------------------
# Runs `signtool verify /pa /v` via Start-Process -PassThru -Wait (not bare `&` +
# $LASTEXITCODE) because Assert-TrustedSignature interleaves a Get-AuthenticodeSignature
# call and diagnostic logic around this invocation — exactly the interleaving scenario
# that makes bare $LASTEXITCODE unsafe (101-RESEARCH.md Pitfall 3).
function Invoke-SignToolVerify {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$SignToolPath,
        [Parameter(Mandatory)][string]$Path
    )

    $stdoutFile = New-TemporaryFile
    $stderrFile = New-TemporaryFile
    try {
        $proc = Start-Process -FilePath $SignToolPath `
            -ArgumentList @('verify', '/pa', '/v', "`"$Path`"") `
            -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput $stdoutFile.FullName `
            -RedirectStandardError $stderrFile.FullName

        return [pscustomobject]@{
            ExitCode = $proc.ExitCode
            StdOut   = (Get-Content -Raw -LiteralPath $stdoutFile.FullName -ErrorAction SilentlyContinue)
            StdErr   = (Get-Content -Raw -LiteralPath $stderrFile.FullName -ErrorAction SilentlyContinue)
        }
    }
    finally {
        Remove-Item -LiteralPath $stdoutFile.FullName -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $stderrFile.FullName -Force -ErrorAction SilentlyContinue
    }
}

# -----------------------------------------------------------------------------
# Get-FlagClassification
# -----------------------------------------------------------------------------
# PURE function (no chain building, no I/O) — classifies an already-computed
# X509ChainStatusFlags value. UntrustedRoot always dominates any transient-looking
# bit (Pitfall 4): never retry a case that also happens to show a transient flag
# alongside a genuine untrusted root.
function Get-FlagClassification {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]$Flags,
        [Parameter(Mandatory)][bool]$Built
    )

    $untrusted = ($Flags -band [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::UntrustedRoot) -ne 0
    $transientBits = [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags](
        [int][System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::RevocationStatusUnknown -bor
        [int][System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::OfflineRevocation -bor
        [int][System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::PartialChain
    )
    $isTransient = (($Flags -band $transientBits) -ne 0) -and (-not $untrusted) -and (-not $Built)

    return [pscustomobject]@{
        IsUntrustedRoot = $untrusted
        IsTransient     = $isTransient
        Flags           = $Flags
        Built           = $Built
    }
}

# -----------------------------------------------------------------------------
# Merge-NoCheckOverride
# -----------------------------------------------------------------------------
# PURE function (no chain building, no I/O). Strictly one-directional: when a NoCheck-mode
# corroborating chain build has confirmed a genuine UntrustedRoot on the root element, this
# forces the classification to IsUntrustedRoot=$true / IsTransient=$false — i.e. it can only
# ever move a classification TOWARD "do not retry, this is real," never toward "treat as
# passing." It never touches Passed/gasOk/signToolOk (those are computed independently in
# $verifyBlock, before this classification is even consulted). When $NoCheckUntrustedRoot is
# $false, $Classification is returned completely unchanged (identity pass-through).
function Merge-NoCheckOverride {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]$Classification,
        [Parameter(Mandatory)][bool]$NoCheckUntrustedRoot
    )

    if ($NoCheckUntrustedRoot) {
        $Classification.IsTransient = $false
        $Classification.IsUntrustedRoot = $true
    }
    return $Classification
}

# -----------------------------------------------------------------------------
# Get-ChainClassification
# -----------------------------------------------------------------------------
# Builds an X509Chain from the signer certificate with online revocation checking,
# ORs all resulting ChainStatus flags together, and delegates to Get-FlagClassification.
# Attaches the built $chain object (for Write-ChainDiagnostic's failure-path dump).
#
# Also builds a second, NoCheck-mode chain purely as diagnostic/classification
# corroboration (research Pattern 1): Online mode can truncate the chain to 3 elements
# and omit the root entirely when revocation can't be checked, masking a genuine
# UntrustedRoot behind transient-looking RevocationStatusUnknown/OfflineRevocation
# flags. NoCheck never calls out to a network endpoint, so it reliably surfaces the
# root element and its true UntrustedRoot status when one exists. This NoCheck build
# is strictly additive corroboration — it never becomes a new pass condition, and it
# can only make a classification MORE conservative (see Merge-NoCheckOverride).
function Get-ChainClassification {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
    )

    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
    $chain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain
    $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(15)
    $built = $chain.Build($Certificate)

    $allFlags = 0
    foreach ($status in $chain.ChainStatus) { $allFlags = $allFlags -bor [int]$status.Status }
    $flags = [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]$allFlags

    $noCheckChain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    $noCheckChain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
    $noCheckChain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain
    $null = $noCheckChain.Build($Certificate)
    $noCheckUntrustedRoot = $false
    foreach ($element in $noCheckChain.ChainElements) {
        foreach ($status in $element.ChainElementStatus) {
            if ($status.Status -band [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::UntrustedRoot) {
                $noCheckUntrustedRoot = $true
            }
        }
    }

    $classification = Get-FlagClassification -Flags $flags -Built $built
    $classification = Merge-NoCheckOverride -Classification $classification -NoCheckUntrustedRoot $noCheckUntrustedRoot
    $classification | Add-Member -MemberType NoteProperty -Name Chain -Value $chain -Force
    $classification | Add-Member -MemberType NoteProperty -Name NoCheckChain -Value $noCheckChain -Force
    return $classification
}

# -----------------------------------------------------------------------------
# Get-CertificateRevocationUrls
# -----------------------------------------------------------------------------
# Extracts AIA (1.3.6.1.5.5.7.1.1) / CDP (2.5.29.31) URLs via X509Extension.Format(true)
# text-parsing (no first-class typed API exists — see 101-RESEARCH.md Don't Hand-Roll),
# probes each with a short-timeout HEAD request. Informational only — never gates.
function Get-CertificateRevocationUrls {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
    )

    $urls = [System.Collections.Generic.List[string]]::new()
    foreach ($ext in $Certificate.Extensions) {
        if ($ext.Oid.Value -eq '2.5.29.31' -or $ext.Oid.Value -eq '1.3.6.1.5.5.7.1.1') {
            $text = $ext.Format($true)
            foreach ($m in [regex]::Matches($text, 'URL=(\S+)')) {
                $urls.Add($m.Groups[1].Value)
            }
        }
    }

    foreach ($url in $urls) {
        $reachable = $false
        try {
            Invoke-WebRequest -Uri $url -Method Head -TimeoutSec 5 -ErrorAction Stop | Out-Null
            $reachable = $true
        }
        catch {
            $reachable = $false
        }
        [pscustomobject]@{ Url = $url; Reachable = $reachable }
    }
}

# -----------------------------------------------------------------------------
# Write-ChainDiagnostic
# -----------------------------------------------------------------------------
# D-04's full-chain dump: invoked ONLY from failure branches, never on the happy path.
# Every chain element's Subject/Issuer/Thumbprint + ChainElementStatus flags, the
# verbatim signtool /v output, and revocation-URL reachability.
function Write-ChainDiagnostic {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Gas,
        [Parameter(Mandatory)]$SignToolResult,
        $Classification
    )

    Write-Host "=== Authenticode chain diagnostic for $Path ==="
    Write-Host "GAS Status: $($Gas.Status)"
    if ($Gas.SignerCertificate) {
        Write-Host "Signer:  $($Gas.SignerCertificate.Subject)"
        Write-Host "Issuer:  $($Gas.SignerCertificate.Issuer)"
    }

    if ($Classification -and $Classification.Chain) {
        Write-Host "--- Chain elements ---"
        foreach ($element in $Classification.Chain.ChainElements) {
            Write-Host "Subject:    $($element.Certificate.Subject)"
            Write-Host "Issuer:     $($element.Certificate.Issuer)"
            Write-Host "Thumbprint: $($element.Certificate.Thumbprint)"
            foreach ($status in $element.ChainElementStatus) {
                Write-Host "  ChainStatus: $($status.Status) - $($status.StatusInformation)"
            }
        }
    }

    Write-Host "--- signtool verify /pa /v output (exit $($SignToolResult.ExitCode)) ---"
    if ($SignToolResult.StdOut) { Write-Host $SignToolResult.StdOut }
    if ($SignToolResult.StdErr) { Write-Host $SignToolResult.StdErr }

    if ($Gas.SignerCertificate) {
        $revocationUrls = Get-CertificateRevocationUrls -Certificate $Gas.SignerCertificate
        foreach ($u in $revocationUrls) {
            Write-Host "Revocation URL: $($u.Url) reachable=$($u.Reachable)"
        }
    }
}

# -----------------------------------------------------------------------------
# Invoke-VerifyWithTransientRetry
# -----------------------------------------------------------------------------
# D-02's core invariant: an exhausted-retry transient never becomes Passed=$true.
# Retries only ever re-attempt the whole verify block; a genuine (non-transient)
# failure returns immediately on attempt 1 without retrying.
function Invoke-VerifyWithTransientRetry {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][scriptblock]$VerifyBlock,
        [int]$MaxAttempts = 3
    )

    $backoffSeconds = @(2, 4, 8)
    $result = $null
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        $result = & $VerifyBlock
        if ($result.Passed) { return $result }
        if (-not $result.IsTransient) { return $result }
        if ($attempt -lt $MaxAttempts) {
            Write-Host "Transient chain/revocation failure (attempt $attempt/$MaxAttempts) - retrying in $($backoffSeconds[$attempt - 1])s"
            Start-Sleep -Seconds $backoffSeconds[$attempt - 1]
        }
    }
    # Exhausted retries on a transient classification — STILL fail closed.
    return $result
}

# -----------------------------------------------------------------------------
# Assert-TrustedSignature — the sole public entry point.
# -----------------------------------------------------------------------------
function Assert-TrustedSignature {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][ValidateSet('Strict', 'Informational')][string]$Mode
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Assert-TrustedSignature: file not found: $Path"
    }

    $signtoolPath = Find-Signtool

    # Captures $Path/$signtoolPath from the enclosing scope. Runs both engines (D-01)
    # and, only if either is a non-pass, classifies via the chain-build.
    $verifyBlock = {
        $gas = Get-AuthenticodeSignature -LiteralPath $Path
        $signToolResult = Invoke-SignToolVerify -SignToolPath $signtoolPath -Path $Path

        $gasOk = ($gas.Status -eq 'Valid')
        $signToolOk = ($signToolResult.ExitCode -eq 0)

        $classification = $null
        if (-not $gasOk -or -not $signToolOk) {
            if ($gas.SignerCertificate) {
                $classification = Get-ChainClassification -Certificate $gas.SignerCertificate
            }
            else {
                # No signer certificate at all (e.g. NotSigned) — cannot build a chain;
                # treat as a genuine (non-transient) failure so it never retries.
                $classification = [pscustomobject]@{ IsUntrustedRoot = $true; IsTransient = $false; Flags = 0; Built = $false; Chain = $null }
            }
        }

        [pscustomobject]@{
            Gas            = $gas
            SignToolResult = $signToolResult
            Passed         = ($gasOk -and $signToolOk)
            IsTransient    = if ($classification) { $classification.IsTransient } else { $false }
            Classification = $classification
        }
    }

    if ($Mode -eq 'Informational') {
        # Informational NEVER retries — a cross-signed WHQL driver carve-out call site
        # predictably classifies as a chain-build failure and would otherwise eat the
        # full ~14s @(2,4,8) backoff on every CI run for a result that is discarded.
        # Branch on -Mode BEFORE any retry.
        $attemptResult = & $verifyBlock
        Write-Host "Authenticode (informational): $Path GAS=$($attemptResult.Gas.Status) signtool_exit=$($attemptResult.SignToolResult.ExitCode)"
        return [pscustomobject]@{
            Path             = $Path
            GasStatus        = $attemptResult.Gas.Status
            SignToolExitCode = $attemptResult.SignToolResult.ExitCode
            Passed           = $null
        }
    }

    # Mode -eq 'Strict': retry is Strict-only (D-02).
    $attemptResult = Invoke-VerifyWithTransientRetry -VerifyBlock $verifyBlock -MaxAttempts 3

    $gas = $attemptResult.Gas
    $signToolResult = $attemptResult.SignToolResult

    # The ORIGINAL condition is preserved verbatim below (byte-for-byte, same
    # operator/string as release.yml:269/:311 and trusted-signing-smoke.yml:69) — do
    # NOT touch this line in any future refactor; SIGN-02 success criterion 3
    # diff-checks it. This is the fail-closed gate: it is never weakened to accept
    # UnknownError, and it always guards a `throw`, never a mere log.
    if ($gas.Status -ne 'Valid') {
        Write-Error -ErrorAction Continue "Authenticode verification failed for $Path with status $($gas.Status)."
        Write-ChainDiagnostic -Path $Path -Gas $gas -SignToolResult $signToolResult -Classification $attemptResult.Classification
        throw "Assert-TrustedSignature: GAS status not Valid ($($gas.Status)) for $Path"
    }
    if ($signToolResult.ExitCode -ne 0) {
        Write-Error -ErrorAction Continue "signtool verify /pa /v failed for $Path (exit $($signToolResult.ExitCode))."
        Write-ChainDiagnostic -Path $Path -Gas $gas -SignToolResult $signToolResult -Classification $attemptResult.Classification
        throw "Assert-TrustedSignature: signtool verify failed (exit $($signToolResult.ExitCode)) for $Path"
    }

    Write-Host "Authenticode OK (dual-engine): $Path"
    return [pscustomobject]@{
        Path             = $Path
        GasStatus        = $gas.Status
        SignToolExitCode = 0
        Passed           = $true
    }
}
