param(
    [Parameter(Mandatory = $true)]
    [string]$CertBase64,

    [Parameter(Mandatory = $true)]
    [string]$CertPassword,

    [Parameter(Mandatory = $true)]
    [string[]]$ArtifactPaths,

    # RFC 3161 timestamp server endpoint.
    # http://timestamp.digicert.com supports both the legacy Authenticode timestamp
    # protocol and RFC 3161 when addressed via /tr. Using /tr + /td sha256 here is
    # the current Microsoft-recommended practice for SHA-256 signed binaries.
    # D-12 (locked): DigiCert, SHA-256 only. D-14 (locked): signtool.exe only.
    [string]$TimestampUrl = "http://timestamp.digicert.com"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

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
        # Directory layout: ...\bin\<version>\<arch>\signtool.exe → version is the grandparent.
        $verName = $_.Directory.Parent.Name
        try { [version]$verName } catch { [version]"0.0.0.0" }
    } -Descending | Select-Object -First 1

    if ($null -eq $chosen) {
        throw "signtool.exe not found on PATH or under the Windows SDK (Windows Kits\10\bin). The Windows SDK must be installed on the runner."
    }
    return $chosen.FullName
}

function Import-SigningCertificate {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PfxPath,

        [Parameter(Mandatory = $true)]
        [string]$Password
    )

    $securePassword = ConvertTo-SecureString $Password -AsPlainText -Force
    $cert = Import-PfxCertificate `
        -FilePath $PfxPath `
        -CertStoreLocation Cert:\CurrentUser\My `
        -Password $securePassword
    return $cert.Thumbprint
}

function Remove-SigningCertificate {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Thumbprint
    )

    $certPath = "Cert:\CurrentUser\My\$Thumbprint"
    if (Test-Path -LiteralPath $certPath) {
        Remove-Item -LiteralPath $certPath -Force -ErrorAction SilentlyContinue
    }
}

function Add-TrustForVerify {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Thumbprint
    )

    # A self-signed POC cert (D-53-02) does NOT chain to a trusted root, so
    # `signtool verify /pa` fails with "A certificate chain processed, but
    # terminated in a root certificate which is not trusted". Import the cert's
    # PUBLIC portion into the LocalMachine Root store so the chain validates.
    #
    # MUST be LocalMachine, not CurrentUser: adding to the CurrentUser Root store
    # raises the interactive protected-root consent dialog ("You are about to
    # install a certificate from a certification authority ... install?"), which
    # HANGS on a headless CI runner (observed: a 3h stall on the Sign step).
    # Adding to LocalMachine Root as an administrator (GitHub-hosted Windows
    # runners are admin) is non-interactive — no consent dialog. For a real
    # CA-issued cert this is a harmless no-op (already chains to a trusted root).
    $cerPath = Join-Path ([System.IO.Path]::GetTempPath()) ("nono-verify-trust-" + $Thumbprint + ".cer")
    try {
        Export-Certificate -Cert "Cert:\CurrentUser\My\$Thumbprint" -FilePath $cerPath | Out-Null
        Import-Certificate -FilePath $cerPath -CertStoreLocation Cert:\LocalMachine\Root | Out-Null
        Write-Host "Trusted signing cert in LocalMachine\Root for verification."
    }
    finally {
        Remove-Item -LiteralPath $cerPath -Force -ErrorAction SilentlyContinue
    }
}

function Remove-TrustForVerify {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Thumbprint
    )

    # Remove from the LocalMachine Root store via the X509Store API (admin,
    # non-interactive). `Remove-Item Cert:\...\Root\...` raises a UI prompt that
    # fails/hangs non-interactively; the X509Store API removes without a prompt.
    try {
        $store = [System.Security.Cryptography.X509Certificates.X509Store]::new('Root', 'LocalMachine')
        $store.Open('ReadWrite')
        foreach ($c in @($store.Certificates | Where-Object { $_.Thumbprint -eq $Thumbprint })) {
            $store.Remove($c)
        }
        $store.Close()
    }
    catch {
        Write-Host "Warning: could not remove trust cert from LocalMachine\Root: $($_.Exception.Message)"
    }
}

function Invoke-SigntoolSign {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SigntoolPath,

        [Parameter(Mandatory = $true)]
        [string]$ArtifactPath,

        [Parameter(Mandatory = $true)]
        [string]$Thumbprint,

        [Parameter(Mandatory = $true)]
        [string]$TimestampUrl
    )

    if (-not (Test-Path -LiteralPath $ArtifactPath)) {
        throw "Artifact not found: $ArtifactPath"
    }

    # /fd sha256   - file digest algorithm (SHA-256)
    # /sha1        - select the certificate by thumbprint
    # /tr          - RFC 3161 timestamp server URL (/td sha256 sets the timestamp digest)
    # /td sha256   - timestamp digest algorithm (SHA-256, required with /tr)
    #
    # D-12: DigiCert timestamping, SHA-256 only.
    # D-14: signtool.exe is the only signing primitive.
    & $SigntoolPath sign /fd sha256 /sha1 $Thumbprint /tr $TimestampUrl /td sha256 $ArtifactPath
    if ($LASTEXITCODE -ne 0) {
        throw "signtool sign failed for '$ArtifactPath' (exit $LASTEXITCODE)."
    }
}

function Invoke-SigntoolVerify {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SigntoolPath,

        [Parameter(Mandatory = $true)]
        [string]$ArtifactPath
    )

    # /pa  - use the default authentication policy
    # /tw  - warn if no timestamp is present (makes timestamp absence a failure mode
    #        rather than a silent success; D-12 requires timestamp-aware verification)
    & $SigntoolPath verify /pa /tw $ArtifactPath
    if ($LASTEXITCODE -ne 0) {
        throw "signtool verify failed for '$ArtifactPath' — signature is not valid Authenticode or is missing a timestamp (exit $LASTEXITCODE)."
    }
    Write-Host "Signature verified: $ArtifactPath"
}

# Decode certificate and write to temp PFX file
$certBytes = [System.Convert]::FromBase64String($CertBase64)
$tempFile = [System.IO.Path]::GetTempFileName()
$pfxPath = [System.IO.Path]::ChangeExtension($tempFile, ".pfx")
[System.IO.File]::Move($tempFile, $pfxPath)

try {
    [System.IO.File]::WriteAllBytes($pfxPath, $certBytes)

    $signtoolPath = Find-Signtool
    Write-Host "Using signtool: $signtoolPath"
    $thumbprint = Import-SigningCertificate -PfxPath $pfxPath -Password $CertPassword
    Add-TrustForVerify -Thumbprint $thumbprint

    try {
        # Sign all artifacts
        foreach ($path in $ArtifactPaths) {
            Invoke-SigntoolSign `
                -SigntoolPath $signtoolPath `
                -ArtifactPath $path `
                -Thumbprint $thumbprint `
                -TimestampUrl $TimestampUrl
        }

        # Verify all artifacts. Failure here aborts; D-13 requires that CI never
        # proceeds to artifact upload if signing or verification fails.
        foreach ($path in $ArtifactPaths) {
            Invoke-SigntoolVerify -SigntoolPath $signtoolPath -ArtifactPath $path
        }
    }
    finally {
        Remove-SigningCertificate -Thumbprint $thumbprint
        Remove-TrustForVerify -Thumbprint $thumbprint
    }
}
finally {
    Remove-Item $pfxPath -Force -ErrorAction SilentlyContinue
}

Write-Host "All artifacts signed and verified."
