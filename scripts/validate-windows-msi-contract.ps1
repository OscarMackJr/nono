param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    # Phase 41 (REQ-CI-02): -BrokerPath is mandatory because scripts/build-windows-msi.ps1
    # made it mandatory in Phase 31 Plan 04 (2026-05-09). Without this, the MSI validator
    # fails with "Cannot process command because of one or more missing mandatory parameters: BrokerPath".
    [Parameter(Mandatory = $true)]
    [string]$BrokerPath,

    [string]$ServiceBinaryPath = "",

    # Quick task 260522-c9c: -DriverBinaryPath threads the pre-signed WFP
    # kernel driver path through to build-windows-msi.ps1. CI must pass this
    # whenever it also passes -ServiceBinaryPath: build-windows-msi.ps1's
    # scope-coherence guard throws if machine scope gets one without the
    # other.
    [string]$DriverBinaryPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-WixDocumentForScope {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Scope,

        [Parameter(Mandatory = $true)]
        [string]$Binary,

        [Parameter(Mandatory = $true)]
        [string]$BrokerBinary,

        [string]$ServiceBinary = "",

        # Quick task 260522-c9c: pre-signed WFP driver path threaded through to
        # build-windows-msi.ps1 -DriverBinaryPath. Machine scope MUST receive
        # this whenever it also receives ServiceBinary; build-windows-msi.ps1
        # enforces this and the caller below maps the parameter accordingly.
        [string]$DriverBinary = ""
    )

    $repoRoot = Split-Path -Parent $PSScriptRoot
    $tempDirName = "temp-msi-contract-" + $Scope
    $tempDir = Join-Path $repoRoot $tempDirName

    if (Test-Path -LiteralPath $tempDir) {
        Remove-Item -Recurse -Force -LiteralPath $tempDir
    }

    try {
        $buildArgs = @{
            VersionTag  = "v0.0.0-preview"
            BinaryPath  = $Binary
            BrokerPath  = $BrokerBinary    # unconditional; BrokerPath is mandatory in build-windows-msi.ps1
            Scope       = $Scope
            OutputDir   = $tempDirName
            EmitOnly    = $true
        }
        if ($ServiceBinary -ne "") {
            $buildArgs["ServiceBinaryPath"] = $ServiceBinary
        }
        if ($DriverBinary -ne "") {
            $buildArgs["DriverBinaryPath"] = $DriverBinary
        }
        & (Join-Path $PSScriptRoot "build-windows-msi.ps1") @buildArgs

        $wxsPath = Join-Path $tempDir ("nono-" + $Scope + ".wxs")
        if (-not (Test-Path -LiteralPath $wxsPath)) {
            throw "Expected WiX source was not generated for scope '$Scope'."
        }

        return [xml](Get-Content -LiteralPath $wxsPath -Raw)
    }
    finally {
        if (Test-Path -LiteralPath $tempDir) {
            Remove-Item -Recurse -Force -LiteralPath $tempDir
        }
    }
}

function Get-FirstNodeByLocalName {
    param(
        [Parameter(Mandatory = $true)]
        [xml]$Document,

        [Parameter(Mandatory = $true)]
        [string]$LocalName
    )

    $nodes = $Document.SelectNodes(("//*[local-name()='" + $LocalName + "']"))
    if ($null -eq $nodes -or $nodes.Count -eq 0) {
        throw "Missing <$LocalName> node in generated WiX document."
    }

    return $nodes[0]
}

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

$binaryFullPath = (Resolve-Path -LiteralPath $BinaryPath).Path

if (-not (Test-Path -LiteralPath $BrokerPath)) {
    throw "BrokerPath does not exist: $BrokerPath"
}
$brokerFullPath = (Resolve-Path -LiteralPath $BrokerPath).Path

$serviceBinaryFullPath = ""
if ($ServiceBinaryPath -ne "") {
    if (-not (Test-Path -LiteralPath $ServiceBinaryPath)) {
        throw "Service binary not found at '$ServiceBinaryPath'."
    }
    $serviceBinaryFullPath = (Resolve-Path -LiteralPath $ServiceBinaryPath).Path
}

# Quick task 260522-c9c: resolve the checked-in pre-signed WFP driver path.
# Same fail-closed pattern as the service binary above.
$driverBinaryFullPath = ""
if ($DriverBinaryPath -ne "") {
    if (-not (Test-Path -LiteralPath $DriverBinaryPath)) {
        throw "Driver binary not found at '$DriverBinaryPath'."
    }
    $driverBinaryFullPath = (Resolve-Path -LiteralPath $DriverBinaryPath).Path
}

$machineDoc = Get-WixDocumentForScope -Scope "machine" -Binary $binaryFullPath -BrokerBinary $brokerFullPath -ServiceBinary $serviceBinaryFullPath -DriverBinary $driverBinaryFullPath
$userDoc = Get-WixDocumentForScope -Scope "user" -Binary $binaryFullPath -BrokerBinary $brokerFullPath

$machinePackage = Get-FirstNodeByLocalName -Document $machineDoc -LocalName "Package"
$userPackage = Get-FirstNodeByLocalName -Document $userDoc -LocalName "Package"
$machineMajorUpgrade = Get-FirstNodeByLocalName -Document $machineDoc -LocalName "MajorUpgrade"
$userMajorUpgrade = Get-FirstNodeByLocalName -Document $userDoc -LocalName "MajorUpgrade"

Assert-Equal -Actual $machinePackage.Scope -Expected "perMachine" -Message "Machine MSI scope mismatch"
Assert-Equal -Actual $userPackage.Scope -Expected "perUser" -Message "User MSI scope mismatch"
Assert-True -Condition ($machinePackage.UpgradeCode -ne $userPackage.UpgradeCode) -Message "Machine and user MSI must use different upgrade codes"
Assert-True -Condition (-not [string]::IsNullOrWhiteSpace($machinePackage.UpgradeCode)) -Message "Machine MSI upgrade code must be present"
Assert-True -Condition (-not [string]::IsNullOrWhiteSpace($userPackage.UpgradeCode)) -Message "User MSI upgrade code must be present"
Assert-True -Condition (-not [string]::IsNullOrWhiteSpace($machineMajorUpgrade.DowngradeErrorMessage)) -Message "Machine MSI must declare MajorUpgrade downgrade messaging"
Assert-True -Condition (-not [string]::IsNullOrWhiteSpace($userMajorUpgrade.DowngradeErrorMessage)) -Message "User MSI must declare MajorUpgrade downgrade messaging"

$machineDirectoryXml = $machineDoc.OuterXml
$userDirectoryXml = $userDoc.OuterXml

Assert-True -Condition $machineDirectoryXml.Contains('ProgramFiles64Folder') -Message "Machine MSI must target ProgramFiles64Folder"
Assert-True -Condition $userDirectoryXml.Contains('LocalAppDataFolder') -Message "User MSI must target LocalAppDataFolder"

$machineNoRepair = $machineDoc.SelectSingleNode("//*[local-name()='Property' and @Id='ARPNOREPAIR']")
$userNoRepair = $userDoc.SelectSingleNode("//*[local-name()='Property' and @Id='ARPNOREPAIR']")
$machineNoModify = $machineDoc.SelectSingleNode("//*[local-name()='Property' and @Id='ARPNOMODIFY']")
$userNoModify = $userDoc.SelectSingleNode("//*[local-name()='Property' and @Id='ARPNOMODIFY']")

if ($null -eq $machineNoRepair -or $null -eq $userNoRepair) {
    throw "Both MSI scopes must disable ARP repair in the current release contract."
}
if ($null -eq $machineNoModify -or $null -eq $userNoModify) {
    throw "Both MSI scopes must disable ARP modify in the current release contract."
}

Assert-Equal -Actual $machineNoRepair.Value -Expected "1" -Message "Machine MSI ARPNOREPAIR mismatch"
Assert-Equal -Actual $userNoRepair.Value -Expected "1" -Message "User MSI ARPNOREPAIR mismatch"
Assert-Equal -Actual $machineNoModify.Value -Expected "1" -Message "Machine MSI ARPNOMODIFY mismatch"
Assert-Equal -Actual $userNoModify.Value -Expected "1" -Message "User MSI ARPNOMODIFY mismatch"

# Service and Event Log element assertions (machine MSI only)
if ($serviceBinaryFullPath -ne "") {
    $machineServiceInstall = Get-FirstNodeByLocalName -Document $machineDoc -LocalName "ServiceInstall"
    Assert-Equal -Actual $machineServiceInstall.Name -Expected "nono-wfp-service" `
        -Message "Machine MSI ServiceInstall Name mismatch"
    Assert-Equal -Actual $machineServiceInstall.Start -Expected "auto" `
        -Message "Machine MSI ServiceInstall Start mismatch (expected auto/boot-start for out-of-box WFP enforcement)"
    Assert-Equal -Actual $machineServiceInstall.Type -Expected "ownProcess" `
        -Message "Machine MSI ServiceInstall Type mismatch"
    Assert-Equal -Actual $machineServiceInstall.Account -Expected "LocalSystem" `
        -Message "Machine MSI ServiceInstall Account mismatch"

    $machineServiceControl = Get-FirstNodeByLocalName -Document $machineDoc -LocalName "ServiceControl"
    Assert-Equal -Actual $machineServiceControl.Name -Expected "nono-wfp-service" `
        -Message "Machine MSI ServiceControl Name mismatch"
    Assert-Equal -Actual $machineServiceControl.Stop -Expected "both" `
        -Message "Machine MSI ServiceControl Stop mismatch"
    Assert-Equal -Actual $machineServiceControl.Remove -Expected "uninstall" `
        -Message "Machine MSI ServiceControl Remove mismatch"
    Assert-Equal -Actual $machineServiceControl.Wait -Expected "yes" `
        -Message "Machine MSI ServiceControl Wait mismatch"
    Assert-Equal -Actual $machineServiceInstall.ErrorControl -Expected "ignore" `
        -Message "Machine MSI ServiceInstall ErrorControl mismatch (must be ignore so SCM start failure is non-fatal per D-04)"
    Assert-Equal -Actual $machineServiceInstall.Vital -Expected "no" `
        -Message "Machine MSI ServiceInstall Vital mismatch (must be no so a service start failure does not roll back the install per D-04 — Vital is on ServiceInstall, not ServiceControl)"

    # User MSI must contain no service elements (D-02)
    $userServiceInstalls = $userDoc.SelectNodes("//*[local-name()='ServiceInstall']")
    Assert-True -Condition ($null -eq $userServiceInstalls -or $userServiceInstalls.Count -eq 0) `
        -Message "User MSI must not contain ServiceInstall elements"
    $userServiceControls = $userDoc.SelectNodes("//*[local-name()='ServiceControl']")
    Assert-True -Condition ($null -eq $userServiceControls -or $userServiceControls.Count -eq 0) `
        -Message "User MSI must not contain ServiceControl elements"

    # Event Log source registration must exist in machine MSI (D-07).
    # The source is registered under the classic Application log via registry keys.
    $eventLogKey = "SYSTEM\CurrentControlSet\Services\EventLog\Application\nono-wfp-service"
    $machineRegistryKeys = $machineDoc.SelectNodes("//*[local-name()='RegistryKey']")
    $machineEventLogKey = $null
    foreach ($node in $machineRegistryKeys) {
        if ($node.Key -eq $eventLogKey) {
            $machineEventLogKey = $node
            break
        }
    }
    Assert-True -Condition ($null -ne $machineEventLogKey) `
        -Message "Machine MSI must register the classic Application Event Log source for nono-wfp-service"

    # EventMessageFile value must be present so Event Viewer can format entries.
    $eventMessageFileNode = $machineEventLogKey.SelectSingleNode(
        "*[local-name()='RegistryValue' and @Name='EventMessageFile']"
    )
    Assert-True -Condition ($null -ne $eventMessageFileNode) `
        -Message "Machine MSI Event Log source must include EventMessageFile registry value"

    # TypesSupported value must be present.
    $typesSupportedNode = $machineEventLogKey.SelectSingleNode(
        "*[local-name()='RegistryValue' and @Name='TypesSupported']"
    )
    Assert-True -Condition ($null -ne $typesSupportedNode) `
        -Message "Machine MSI Event Log source must include TypesSupported registry value"

    # User MSI must not carry any EventLog registry keys.
    $userRegistryKeys = $userDoc.SelectNodes("//*[local-name()='RegistryKey']")
    $userEventLogKey = $null
    foreach ($node in $userRegistryKeys) {
        if ($null -ne $node.Key -and $node.Key.Contains("EventLog")) {
            $userEventLogKey = $node
            break
        }
    }
    Assert-True -Condition ($null -eq $userEventLogKey) `
        -Message "User MSI must not register any EventLog registry keys"
}

# Quick task 260522-c9c: WFP kernel driver component assertions (machine MSI only).
# The driver is a flat data file (no <ServiceInstall>) at INSTALLFOLDER\nono-wfp-driver.sys.
# Without it, the runtime probe in exec_strategy_windows::network fails with
# BackendDriverBinaryMissing before any sandbox can be applied.
if ($driverBinaryFullPath -ne "") {
    # Machine MSI must have a Component with Id=cmpWfpDriverSys whose <File>
    # child has Name="nono-wfp-driver.sys" (the sibling-of-nono.exe name the
    # runtime probe checks for).
    $machineDriverFiles = $machineDoc.SelectNodes("//*[local-name()='File' and @Name='nono-wfp-driver.sys']")
    Assert-True -Condition ($null -ne $machineDriverFiles -and $machineDriverFiles.Count -ge 1) `
        -Message "Machine MSI must contain a <File Name='nono-wfp-driver.sys' /> element"

    # The driver MUST NOT receive a <ServiceInstall> — WiX's element only models
    # user-mode services and cannot represent SERVICE_KERNEL_DRIVER. The CLI
    # command `nono setup install-wfp-driver` performs the kernel registration
    # post-install instead.
    $machineComponents = $machineDoc.SelectNodes("//*[local-name()='Component' and @Id='cmpWfpDriverSys']")
    Assert-True -Condition ($null -ne $machineComponents -and $machineComponents.Count -ge 1) `
        -Message "Machine MSI must contain a Component with Id='cmpWfpDriverSys'"
    $driverServiceInstalls = $machineComponents[0].SelectNodes("*[local-name()='ServiceInstall']")
    Assert-True -Condition ($null -eq $driverServiceInstalls -or $driverServiceInstalls.Count -eq 0) `
        -Message "cmpWfpDriverSys must not contain ServiceInstall (kernel driver registration is post-install via the CLI)"

    # User MSI must not carry the driver component.
    $userDriverFiles = $userDoc.SelectNodes("//*[local-name()='File' and @Name='nono-wfp-driver.sys']")
    Assert-True -Condition ($null -eq $userDriverFiles -or $userDriverFiles.Count -eq 0) `
        -Message "User MSI must not contain nono-wfp-driver.sys file element"
}

# ─── Phase 82 Plan 01: machine-only element assertions ─────────────────────────
# These assertions are unconditional for machine scope (not gated on WFP service
# binary) because the ProgramData root, sentinel key, cert files, cert CA, and
# nono CLI Event Log source are always emitted for all machine-scope MSI builds.

# (a) Cert-import CustomAction: deferred SYSTEM, non-fatal (T-82-01 / D-04)
$machineCustomActions = $machineDoc.SelectNodes("//*[local-name()='CustomAction']")
$certImportCa = $null
foreach ($ca in $machineCustomActions) {
    if ($null -ne $ca.ExeCommand -and $ca.ExeCommand.Contains("setup --trust-root")) {
        $certImportCa = $ca
        break
    }
}
Assert-True -Condition ($null -ne $certImportCa) `
    -Message "Machine MSI must contain a CustomAction with ExeCommand containing 'setup --trust-root'"
Assert-Equal -Actual $certImportCa.Execute -Expected "deferred" `
    -Message "Cert-import CustomAction must be Execute='deferred' (runs as LocalSystem per D-04)"
Assert-Equal -Actual $certImportCa.Impersonate -Expected "no" `
    -Message "Cert-import CustomAction must have Impersonate='no' (run as LocalSystem, not as installer user)"
Assert-Equal -Actual $certImportCa.Return -Expected "ignore" `
    -Message "Cert-import CustomAction must have Return='ignore' (non-fatal per D-04 — cert failure does not roll back install)"

# (b) HKLM\SOFTWARE\Policies\nono sentinel key
$machineRegistryKeysAll = $machineDoc.SelectNodes("//*[local-name()='RegistryKey']")
$sentinelKey = $null
foreach ($node in $machineRegistryKeysAll) {
    if ($null -ne $node.Key -and $node.Key -eq "SOFTWARE\Policies\nono") {
        $sentinelKey = $node
        break
    }
}
Assert-True -Condition ($null -ne $sentinelKey) `
    -Message "Machine MSI must create the HKLM\SOFTWARE\Policies\nono sentinel key (Phase 83 reader + nono health probe)"

# (c) ProgramData root directory: machine MSI must target CommonAppDataFolder (never LocalAppDataFolder in machine block)
Assert-True -Condition $machineDirectoryXml.Contains('CommonAppDataFolder') `
    -Message "Machine MSI must target CommonAppDataFolder for the %PROGRAMDATA%\nono\ root (Pitfall 4 / D-08)"
Assert-True -Condition (-not $machineDirectoryXml.Contains('LocalAppDataFolder')) `
    -Message "Machine MSI must NOT target LocalAppDataFolder (SYSTEM-context install writes to systemprofile, breaking R-B3; Pitfall 4)"

# (d) PEM File component (Blocker-1 guard): machine MSI must stage both DER .cer and PEM .pem cert files.
#     Node's NODE_EXTRA_CA_CERTS cannot read a DER .cer file (Pitfall 13).
$machineFiles = $machineDoc.SelectNodes("//*[local-name()='File']")
$pemFile = $null
$cerFile = $null
foreach ($f in $machineFiles) {
    if ($null -ne $f.Name) {
        if ($f.Name -like "*.pem") { $pemFile = $f }
        if ($f.Name -eq "nono-poc-root.cer") { $cerFile = $f }
    }
}
Assert-True -Condition ($null -ne $pemFile) `
    -Message "Machine MSI must contain a PEM File component (*.pem) for Node NODE_EXTRA_CA_CERTS trust (Pitfall 13 / D-05)"
Assert-True -Condition ($null -ne $cerFile) `
    -Message "Machine MSI must contain a DER .cer File component for certutil -addstore imports (Pitfall 13)"

# (e) nono CLI Event Log source: machine MSI must register EventLog\Application\nono
$nonoCliEventLogKey = $null
foreach ($node in $machineRegistryKeysAll) {
    if ($null -ne $node.Key -and $node.Key -eq "SYSTEM\CurrentControlSet\Services\EventLog\Application\nono") {
        $nonoCliEventLogKey = $node
        break
    }
}
Assert-True -Condition ($null -ne $nonoCliEventLogKey) `
    -Message "Machine MSI must register the nono CLI Application Event Log source (EventLog\Application\nono) for Phase 84"
$nonoCliEventMsgFile = $nonoCliEventLogKey.SelectSingleNode(
    "*[local-name()='RegistryValue' and @Name='EventMessageFile']"
)
Assert-True -Condition ($null -ne $nonoCliEventMsgFile) `
    -Message "nono CLI Event Log source must include EventMessageFile registry value"

# (f) User MSI must NOT contain any of the machine-only elements
$userXml = $userDoc.OuterXml

$userCertCas = $userDoc.SelectNodes("//*[local-name()='CustomAction']")
$userCertCa = $null
foreach ($ca in $userCertCas) {
    if ($null -ne $ca.ExeCommand -and $ca.ExeCommand.Contains("setup --trust-root")) {
        $userCertCa = $ca
        break
    }
}
Assert-True -Condition ($null -eq $userCertCa) `
    -Message "User MSI must NOT contain a 'setup --trust-root' CustomAction (machine-only element)"

$userRegistryKeysAll = $userDoc.SelectNodes("//*[local-name()='RegistryKey']")
$userSentinelKey = $null
foreach ($node in $userRegistryKeysAll) {
    if ($null -ne $node.Key -and $node.Key -eq "SOFTWARE\Policies\nono") {
        $userSentinelKey = $node
        break
    }
}
Assert-True -Condition ($null -eq $userSentinelKey) `
    -Message "User MSI must NOT contain the SOFTWARE\Policies\nono sentinel key (machine-only)"

Assert-True -Condition (-not $userXml.Contains('CommonAppDataFolder')) `
    -Message "User MSI must NOT contain CommonAppDataFolder (machine-only ProgramData component)"

$userFiles = $userDoc.SelectNodes("//*[local-name()='File']")
$userPemFile = $null
foreach ($f in $userFiles) {
    if ($null -ne $f.Name -and $f.Name -like "*.pem") { $userPemFile = $f; break }
}
Assert-True -Condition ($null -eq $userPemFile) `
    -Message "User MSI must NOT contain a PEM cert File component (machine-only element)"

$userNonoCliEventLogKey = $null
foreach ($node in $userRegistryKeysAll) {
    if ($null -ne $node.Key -and $node.Key -eq "SYSTEM\CurrentControlSet\Services\EventLog\Application\nono") {
        $userNonoCliEventLogKey = $node
        break
    }
}
Assert-True -Condition ($null -eq $userNonoCliEventLogKey) `
    -Message "User MSI must NOT register the nono CLI Event Log source (machine-only)"

# ─── Static-CRT flag assertion (.cargo/config.toml) ──────────────────────────
# Phase 82 Plan 01 D-01/D-02: verify the target-feature=+crt-static flag is in
# .cargo/config.toml under [target.x86_64-pc-windows-msvc]. The flag eliminates
# the vcruntime140.dll dependency and the 0xC0000135 clean-host 1603 rollback.
# NOTE: This stanza is silently dropped when the RUSTFLAGS env var is set (e.g.
# in CI with step-level RUSTFLAGS= override). CI/RUSTFLAGS-override caveat is
# documented in .cargo/config.toml. This assertion verifies the in-file flag only.
$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoConfigPath = Join-Path $repoRoot ".cargo\config.toml"
if (Test-Path -LiteralPath $cargoConfigPath) {
    $cargoConfigContent = Get-Content -LiteralPath $cargoConfigPath -Raw
    Assert-True -Condition ($cargoConfigContent -match 'crt-static') `
        -Message ".cargo/config.toml must contain target-feature=+crt-static for the windows-msvc target (D-01/D-02 root-cause fix for 0xC0000135 clean-host 1603 rollback)"
    Write-Host "Static-CRT flag verified present in .cargo/config.toml"
} else {
    throw ".cargo/config.toml not found at '$cargoConfigPath'. This file must carry the static-CRT rustflag for the windows-msvc target."
}

Write-Host "Validated Windows MSI contract for machine and user scopes."
