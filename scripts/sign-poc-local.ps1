<#
.SYNOPSIS
    Local, self-signed Authenticode signing for a controlled internal nono POC.

.DESCRIPTION
    One command that makes a locally-built nono install pass the Windows broker-spawn
    trust gate (verify_broker_authenticode, D-32-12) on a small set of trusted machines:

        self-signed code-signing cert  ->  sign EXEs  ->  rebuild MSIs  ->  sign MSIs
                                                                        ->  export public .cer

    nono's broker gate uses full-chain WinVerifyTrust and requires nono.exe AND
    nono-shell-broker.exe to be Authenticode-signed by the SAME cert that chains to a
    trusted root. An unsigned Program Files install fail-closes with
    "Self-trust-anchor unavailable; refusing to spawn broker."

    *** POC-ONLY. NOT a production / external-distribution signing path. ***
    A self-signed cert is trusted only after its public certificate is imported into each
    target machine's LocalMachine\Root + LocalMachine\TrustedPublisher stores. Doing so
    BROADENS that machine's code-trust surface to anything signed by this cert, so it is
    appropriate ONLY for machines you control (here: a 5-person desktop-support team with
    admin rights). It is fully reversible: remove the cert from the two stores and uninstall.
    For real distribution use a commercial CA code-signing cert or Azure Trusted Signing
    (see docs/cli/development/windows-signing-guide.mdx) and the CI release.yml path.

    DigiCert is NOT required as a certificate authority. The only DigiCert dependency is the
    FREE public RFC-3161 timestamp server (-TimestampUrl), which keeps signatures valid after
    the cert expires; no account or purchase is involved.

    CRITICAL ORDERING (enforced by this script): the MSI embeds COPIES of the EXEs in its cab,
    and the broker gate checks the INSTALLED nono.exe's signature, not the MSI's. So the EXEs
    must be signed BEFORE the MSI is (re)built. Signing the MSI alone leaves unsigned EXEs
    inside it and the broker still fails.

.PARAMETER VersionTag
    Release tag used for MSI file names (must match the build that produced the binaries).
    Default: v0.57.3

.PARAMETER Scope
    Which MSI(s) to (re)build and sign: machine, user, or both. Default: both.

.PARAMETER CertSubject
    Subject for a freshly-created self-signed cert. Ignored when -Thumbprint is supplied.
    Default: "CN=nono POC Signing"

.PARAMETER TimestampUrl
    RFC-3161 timestamp server. Default: http://timestamp.digicert.com (free, no account).

.PARAMETER Thumbprint
    Reuse an existing code-signing cert in Cert:\CurrentUser\My by thumbprint instead of
    creating a new self-signed cert. Throws if the thumbprint is not found.

.PARAMETER SkipMsiRebuild
    Do NOT rebuild the MSIs. Sign the EXEs and any already-built MSIs in -OutputDir only.
    Use for re-signing/re-timestamping. NOTE: the MSI embeds whatever EXEs were present at
    its LAST build, so skipping the rebuild only yields a fully-signed install if those EXEs
    were already signed before that build.

.PARAMETER OutputDir
    Where build-windows-msi.ps1 emits the MSIs and where the public .cer is written.
    Default: dist/windows (relative to the repo root).

.EXAMPLE
    pwsh -File scripts/sign-poc-local.ps1
    # Create a self-signed cert, sign EXEs, rebuild + sign both MSIs, export the public cer.

.EXAMPLE
    pwsh -File scripts/sign-poc-local.ps1 -Scope machine -Thumbprint A1B2C3...
    # Reuse an existing cert; only the machine-scope MSI.

.NOTES
    Running against the default -OutputDir dist/windows regenerates the tracked
    dist/windows/*.wxs reference snapshots as a side effect of build-windows-msi.ps1.
    Revert with `git checkout -- dist/windows/nono-machine.wxs dist/windows/nono-user.wxs`
    if you do not want that churn (or pass a throwaway -OutputDir).
#>
[CmdletBinding()]
param(
    [string]$VersionTag = "v0.57.4",

    [ValidateSet("machine", "user", "both")]
    [string]$Scope = "both",

    [string]$CertSubject = "CN=nono POC Signing",

    [string]$TimestampUrl = "http://timestamp.digicert.com",

    [string]$Thumbprint = "",

    [switch]$SkipMsiRebuild,

    [string]$OutputDir = "dist/windows"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

# --- Locate signtool.exe (newest Windows Kits\10\bin\<ver>\x64) ------------------------------
function Find-SignTool {
    $kitsRoots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "$env:ProgramFiles\Windows Kits\10\bin"
    )
    $found = @()
    foreach ($root in $kitsRoots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($dir in Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue) {
            $candidate = Join-Path $dir.FullName "x64\signtool.exe"
            if (Test-Path -LiteralPath $candidate) {
                $ver = [version]"0.0.0.0"
                [void][version]::TryParse($dir.Name, [ref]$ver)
                $found += [pscustomobject]@{ Version = $ver; Path = $candidate }
            }
        }
    }
    if ($found.Count -eq 0) {
        throw "signtool.exe not found under any 'Windows Kits\10\bin\<ver>\x64'. Install the Windows 10/11 SDK (App Certification Kit / Signing Tools) and rerun."
    }
    return ($found | Sort-Object Version -Descending | Select-Object -First 1).Path
}

$signtool = Find-SignTool
Write-Host "signtool: $signtool"

# --- Resolve / create the signing certificate ------------------------------------------------
if ($Thumbprint -ne "") {
    $normalized = ($Thumbprint -replace '[^0-9A-Fa-f]', '').ToUpperInvariant()
    $cert = Get-Item -Path ("Cert:\CurrentUser\My\{0}" -f $normalized) -ErrorAction SilentlyContinue
    if ($null -eq $cert) {
        throw "No code-signing certificate with thumbprint '$Thumbprint' in Cert:\CurrentUser\My. Omit -Thumbprint to create a new self-signed cert."
    }
    Write-Host "Using existing certificate: $($cert.Subject) [$($cert.Thumbprint)]"
}
else {
    $cert = New-SelfSignedCertificate `
        -Subject $CertSubject `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -Type CodeSigningCert `
        -KeyUsage DigitalSignature `
        -HashAlgorithm SHA256 `
        -NotAfter (Get-Date).AddYears(2)
    Write-Host "Created self-signed certificate: $($cert.Subject) [$($cert.Thumbprint)] (expires $($cert.NotAfter.ToString('yyyy-MM-dd')))"
}
$thumb = $cert.Thumbprint

# --- Sign helper (fail-closed) ---------------------------------------------------------------
function Invoke-SignTool {
    param(
        [Parameter(Mandatory = $true)][string]$File
    )
    if (-not (Test-Path -LiteralPath $File)) {
        throw "Cannot sign — file does not exist: $File"
    }
    & $signtool sign /fd sha256 /sha1 $thumb /tr $TimestampUrl /td sha256 $File
    if ($LASTEXITCODE -ne 0) {
        throw "signtool failed to sign '$File' (exit $LASTEXITCODE)."
    }
    Write-Host "  signed: $File"
}

# --- Resolve binaries + scope ----------------------------------------------------------------
$nonoExe        = Join-Path $repoRoot "target\release\nono.exe"
$brokerExe      = Join-Path $repoRoot "target\release\nono-shell-broker.exe"
$wfpServiceExe  = Join-Path $repoRoot "target\release\nono-wfp-service.exe"
$driverSys      = Join-Path $repoRoot "crates\nono-cli\data\windows\nono-wfp-driver.sys"
$buildScript    = Join-Path $PSScriptRoot "build-windows-msi.ps1"
$outputFullPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDir))

if (-not (Test-Path -LiteralPath $nonoExe))   { throw "Missing $nonoExe. Build first: cargo build --release -p nono-cli." }
if (-not (Test-Path -LiteralPath $brokerExe)) { throw "Missing $brokerExe. Build first: cargo build --release -p nono-shell-broker." }

$scopes = switch ($Scope) {
    "machine" { @("machine") }
    "user"    { @("user") }
    "both"    { @("machine", "user") }
}

# WFP service is bundled only by the machine MSI. Sign it only when a machine MSI is in scope
# AND the binary exists (a machine MSI without it produces a plain, WFP-less install).
$signWfpService = ($scopes -contains "machine") -and (Test-Path -LiteralPath $wfpServiceExe)

# --- Step 1: sign the EXEs FIRST (before any MSI is built) -----------------------------------
Write-Host ""
Write-Host "[1/4] Signing executables..."
$signedArtifacts = New-Object System.Collections.Generic.List[string]
Invoke-SignTool -File $nonoExe;   $signedArtifacts.Add($nonoExe)
Invoke-SignTool -File $brokerExe; $signedArtifacts.Add($brokerExe)
if ($signWfpService) {
    Invoke-SignTool -File $wfpServiceExe; $signedArtifacts.Add($wfpServiceExe)
}

# --- Step 2: rebuild the MSIs so the cab embeds the now-signed EXEs ---------------------------
Write-Host ""
if ($SkipMsiRebuild) {
    Write-Host "[2/4] Skipping MSI rebuild (-SkipMsiRebuild). The MSI(s) embed whatever EXEs"
    Write-Host "      were present at their last build — sign those EXEs and rebuild if unsure."
}
else {
    Write-Host "[2/4] Rebuilding MSI(s) off the signed EXEs via build-windows-msi.ps1..."
    if (-not (Test-Path -LiteralPath $buildScript)) { throw "Missing build script: $buildScript" }
    foreach ($s in $scopes) {
        if ($s -eq "machine") {
            $buildArgs = @{
                VersionTag = $VersionTag
                BinaryPath = $nonoExe
                BrokerPath = $brokerExe
                Scope      = "machine"
                OutputDir  = $OutputDir
            }
            # Machine scope requires BOTH service + driver, or neither (build script enforces).
            if ($signWfpService) {
                if (-not (Test-Path -LiteralPath $driverSys)) {
                    throw "WFP service exists but driver is missing: $driverSys (machine MSI needs both or neither)."
                }
                $buildArgs.ServiceBinaryPath = $wfpServiceExe
                $buildArgs.DriverBinaryPath  = $driverSys
            }
            & $buildScript @buildArgs
        }
        else {
            & $buildScript -VersionTag $VersionTag -BinaryPath $nonoExe -BrokerPath $brokerExe -Scope user -OutputDir $OutputDir
        }
    }
}

# --- Step 3: sign the MSI(s) -----------------------------------------------------------------
Write-Host ""
Write-Host "[3/4] Signing MSI(s)..."
foreach ($s in $scopes) {
    $msiPath = Join-Path $outputFullPath ("nono-{0}-x86_64-pc-windows-msvc-{1}.msi" -f $VersionTag, $s)
    if (-not (Test-Path -LiteralPath $msiPath)) {
        throw "Expected MSI not found: $msiPath. Rebuild without -SkipMsiRebuild, or check -VersionTag/-Scope/-OutputDir."
    }
    Invoke-SignTool -File $msiPath
    $signedArtifacts.Add($msiPath)
}

# --- Step 4: export the PUBLIC certificate for distribution (no private key) ------------------
Write-Host ""
Write-Host "[4/4] Exporting public certificate..."
New-Item -ItemType Directory -Force -Path $outputFullPath | Out-Null
$cerPath = Join-Path $outputFullPath "nono-poc-signing.cer"
Export-Certificate -Cert $cert -FilePath $cerPath -Type CERT | Out-Null
Write-Host "  public cert: $cerPath (safe to distribute — contains NO private key)"

# --- Signature status report (non-fatal) -----------------------------------------------------
# On THIS build machine the self-signed cert is in CurrentUser\My but not in a trusted root,
# so Get-AuthenticodeSignature will report a non-Valid status (e.g. UnknownError) until the
# .cer is imported into Root + TrustedPublisher. That is EXPECTED pre-import; we do NOT gate
# on `signtool verify /pa` here because it would false-fail on the build machine.
Write-Host ""
Write-Host "Signature status (informational; non-Valid is expected until the cert is trusted):"
foreach ($a in $signedArtifacts) {
    $sig = Get-AuthenticodeSignature -FilePath $a
    Write-Host ("  {0,-14} {1}" -f $sig.Status, $a)
}

# --- Next steps ------------------------------------------------------------------------------
$machineMsi = Join-Path $outputFullPath ("nono-{0}-x86_64-pc-windows-msvc-machine.msi" -f $VersionTag)
Write-Host ""
Write-Host "================================================================================"
Write-Host " Done. POC signing complete (thumbprint $thumb)."
Write-Host "================================================================================"
Write-Host ""
Write-Host " The PRIVATE key stays in this machine's Cert:\CurrentUser\My — NEVER export or"
Write-Host " share the .pfx. Distribute ONLY the public .cer + the signed MSI."
Write-Host ""
Write-Host " On EACH target machine (admin, one-time):"
Write-Host ""
Write-Host "   Import-Certificate -FilePath nono-poc-signing.cer -CertStoreLocation Cert:\LocalMachine\Root"
Write-Host "   Import-Certificate -FilePath nono-poc-signing.cer -CertStoreLocation Cert:\LocalMachine\TrustedPublisher"
Write-Host "   msiexec /i `"$(Split-Path -Leaf $machineMsi)`""
Write-Host "   (Get-AuthenticodeSignature 'C:\Program Files\nono\nono.exe').Status   # -> Valid"
Write-Host "   nono run --profile claude-code --allow-cwd -- claude --version"
Write-Host ""
Write-Host " To undo on a machine: remove the cert from LocalMachine\Root + \TrustedPublisher"
Write-Host " and uninstall the MSI. This is a POC trust path, not an external-release path."
Write-Host ""
