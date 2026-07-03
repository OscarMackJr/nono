param(
    # Dedicated EPHEMERAL resource group for this VM fixture. NEVER the pre-existing `RG_Nono`,
    # which holds the live `ArtifactNono` code-signing account - `az group delete` against a
    # VM-only RG can never accidentally remove the signing account (103-RESEARCH.md Open
    # Question 1, RESOLVED: self-contained networking + dedicated RG_Nono_CleanHost).
    [string]$ResourceGroupName = 'RG_Nono_CleanHost',

    [string]$Location = 'eastus',

    [string]$VmName = 'nono-clean-host',

    [string]$VmSize = 'Standard_D4s_v5',

    [Parameter(Mandatory = $true)]
    [string]$AdminUsername,

    [Parameter(Mandatory = $true)]
    [securestring]$AdminPassword
)

# scripts/azure/clean-vm/deploy.ps1
#
# Phase 103 (CHOST-01) - deploy wrapper for the clean-host VM Bicep module.
#
# NOT INVOKED LIVE in Phase 103 - this script is authored + syntax-checked only this phase
# (`[System.Management.Automation.Language.Parser]::ParseFile`, no execution). Phase 106 runs it
# for real against a live Azure subscription for clean-host UAT.
#
# Ephemeral lifecycle (T-103-02): this script creates (idempotently) a dedicated, disposable
# resource group and deploys the VM fixture into it. `teardown.ps1` is the ONLY supported cleanup
# path - a VM stood up by this script must never be left running past a single UAT session. Never
# reuse this RG for anything persistent; never point `$ResourceGroupName` at the live `RG_Nono`.
#
# The Windows 11 image SKU and the operator's public IP are ALWAYS resolved LIVE (never
# hardcoded) immediately before deploy:
#   - SKU: `az vm image list-skus` (103-RESEARCH.md "Live-verified Windows 11 SKU resolution").
#   - Operator IP: `curl` against a public IP-echo endpoint - NEVER the Azure CLI's generic REST
#     passthrough subcommand (103-RESEARCH.md Pitfall 2: that subcommand's Python HTTP client
#     fails on this corporate network's TLS interception for any non-`management.azure.com` host;
#     `curl`'s Windows-SChannel-backed client succeeds). Any TLS-bypass/certificate-skip flag is
#     FORBIDDEN here - fail closed on any certificate error, matching every other live call in
#     this phase.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# --- Step 1: resolve the Windows 11 image SKU LIVE (never a hardcoded fallback) ---
# Matches ^win11-\d{2}h2-(ent|pro)$ to exclude -ltsc/-entn variants and any superseded naming
# (e.g. win11-23h2-* is already superseded by 24h2/25h2 in the live catalog - 103-RESEARCH.md
# "State of the Art"). Sorted descending so the newest matching SKU wins.
$skuListJson = az vm image list-skus --location $Location --publisher MicrosoftWindowsDesktop --offer windows-11 -o json
$skus = $skuListJson | ConvertFrom-Json
$candidateSkus = $skus.name | Where-Object { $_ -match '^win11-\d{2}h2-(ent|pro)$' } | Sort-Object -Descending
if ($candidateSkus.Count -eq 0) {
    throw "No Windows 11 SKU matched the expected ^win11-\d{2}h2-(ent|pro)$ pattern under MicrosoftWindowsDesktop/windows-11 in $Location - refusing to fall back to a hardcoded SKU."
}
$vmImageSku = $candidateSkus[0]
Write-Host "Resolved vmImageSku (live): $vmImageSku"

# --- Step 2: create the dedicated ephemeral resource group (idempotent) ---
az group create --name $ResourceGroupName --location $Location | Out-Null
Write-Host "Ephemeral resource group ready: $ResourceGroupName"

# --- Step 3: fetch the operator's public IP LIVE, as the LAST step before deploy ---
# (minimizes the IP-drift window documented in 103-RESEARCH.md Pitfall 2 - three separate
# providers/calls in the same research session returned three different IPs on this network).
# NEVER the Azure CLI's generic REST passthrough subcommand (fails on this corporate network's
# TLS interception for non-Azure hosts). NEVER pass a certificate-bypass flag to curl - fail
# closed on any certificate error.
$operatorIp = (curl.exe -s https://ifconfig.me).Trim()
if ([string]::IsNullOrWhiteSpace($operatorIp)) {
    throw "Failed to resolve the operator's public IP via curl - refusing to deploy with an empty NSG scope."
}
$operatorIpCidr = "$operatorIp/32"
Write-Host "Resolved operatorIpCidr (live, fetched immediately before deploy): $operatorIpCidr"

# --- Step 4: assemble the ARM parameters-file JSON and deploy ---
# AdminPassword is converted from [securestring] to plaintext only transiently, in memory, for
# this single `az deployment group create` call (T-103-05) - never logged, never persisted
# beyond the temp params file removed in the `finally` block below.
$plainAdminPassword = (New-Object System.Net.NetworkCredential('', $AdminPassword)).Password

$armParameters = [ordered]@{
    '$schema'       = 'https://schema.management.azure.com/schemas/2019-04-01/deploymentParameters.json#'
    contentVersion  = '1.0.0.0'
    parameters      = [ordered]@{
        location        = @{ value = $Location }
        vmImageSku      = @{ value = $vmImageSku }
        operatorIpCidr  = @{ value = $operatorIpCidr }
        vmName          = @{ value = $VmName }
        vmSize          = @{ value = $VmSize }
        adminUsername   = @{ value = $AdminUsername }
        adminPassword   = @{ value = $plainAdminPassword }
    }
}

$tempParamsFile = New-TemporaryFile
try {
    $armParameters | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $tempParamsFile.FullName -Encoding utf8

    $bicepPath = Join-Path $PSScriptRoot 'main.bicep'
    Write-Host "Deploying $bicepPath to resource group $ResourceGroupName ..."
    az deployment group create `
        --resource-group $ResourceGroupName `
        --template-file $bicepPath `
        --parameters "@$($tempParamsFile.FullName)"
}
finally {
    # Secret hygiene (T-103-05): never leave a plaintext password on disk longer than the single
    # `az` invocation above, regardless of success or failure.
    Remove-Item -LiteralPath $tempParamsFile.FullName -Force -ErrorAction SilentlyContinue
}

Write-Host "Deploy complete. Remember: teardown.ps1 is the ONLY supported cleanup path - never leave this VM running past the UAT session."
