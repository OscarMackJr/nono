#Requires -Version 7.0
<#
.SYNOPSIS
    Read-only Azure preflight for the Phase 63 minifilter UAT (Task 1 of plan 63-02).

.DESCRIPTION
    Verifies — WITHOUT creating any resource — that the signed-in Azure CLI context
    has everything needed to provision the disposable test-signing VM that compiles
    drivers/nono-fltmgr/ to nono-fltmgr.sys (SC1/SC2 / DRV-03 partial).

    Checks performed:
      1. az CLI present + a logged-in account context.
      2. Microsoft.Compute / Microsoft.Network resource providers registered.
      3. RBAC: the signed-in identity holds a VM-deploy-capable role
         (Owner / Contributor / Virtual Machine Contributor [+ Network Contributor]).
         NOTE: the Entra "Application Administrator" directory role does NOT grant this.
      4. Compute quota: each candidate VM family has >= needed vCPUs free,
         AND "Total Regional vCPUs" has headroom, in the target region.
      5. Azure Policy: flags assignments that may force Trusted Launch / Secure Boot /
         vTPM or restrict locations/SKUs — these can DENY the Standard-security-type VM
         the plan requires (Secure Boot OFF is mandatory for bcdedit /set testsigning on).
      6. Image: the Gen2 Win11 Pro marketplace image resolves and is HyperV gen V2.
      7. Per-size availability/restrictions in the region.

    Exit code 0 if at least one candidate family is fully GO; 1 otherwise.

.PARAMETER Location
    Azure region to check. Default: eastus.

.PARAMETER NeededVcpus
    vCPUs the VM needs. Default: 4 (Standard_D4*/B4ms are 4 vCPU / 16 GB).

.PARAMETER ImageUrn
    Gen2 Win11 Pro image URN. Default: MicrosoftWindowsDesktop:windows-11:win11-24h2-pro:latest

.EXAMPLE
    pwsh ./63-preflight-azure.ps1
    pwsh ./63-preflight-azure.ps1 -Location westus2
#>
[CmdletBinding()]
param(
    [string]$Location = "eastus",
    [int]$NeededVcpus = 4,
    [string]$ImageUrn = "MicrosoftWindowsDesktop:windows-11:win11-24h2-pro:latest"
)

$ErrorActionPreference = "Continue"

# size -> quota "name.value" (the family bucket Azure enforces; matches the QuotaExceeded error)
$Candidates = [ordered]@{
    "Standard_D4s_v5"  = "standardDSv5Family"
    "Standard_D4as_v5" = "standardDASv5Family"
    "Standard_D4s_v4"  = "standardDSv4Family"
    "Standard_B4ms"    = "standardBSFamily"
}

$script:fail = 0
$script:locBlocked = @()   # families that have quota but are LOCATION-restricted in $Location
function Pass($m) { Write-Host "  [PASS] $m" -ForegroundColor Green }
function Warn($m) { Write-Host "  [WARN] $m" -ForegroundColor Yellow }
function Fail($m) { Write-Host "  [FAIL] $m" -ForegroundColor Red; $script:fail++ }
function Head($m) { Write-Host "`n=== $m ===" -ForegroundColor Cyan }

# Invoke az, return parsed JSON ($null on failure). Never throws.
# NOTE: param is $CmdArgs, NOT $Args — $Args collides with the automatic variable
# and would splat empty, running `az` with no subcommand.
function Invoke-Az {
    param([string[]]$CmdArgs)
    try {
        $raw = & az @CmdArgs -o json 2>$null
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($raw)) { return $null }
        return $raw | ConvertFrom-Json
    } catch { return $null }
}

Write-Host "nono Phase 63 — Azure VM provisioning preflight" -ForegroundColor White
Write-Host "Region: $Location | Needed vCPUs: $NeededVcpus | Image: $ImageUrn"
Write-Host "(read-only — creates nothing)"

# ----------------------------------------------------------------------------
Head "1. Azure CLI + login context"
if (-not (Get-Command az -ErrorAction SilentlyContinue)) {
    Fail "az CLI not found on PATH. Install from https://aka.ms/installazurecli"
    exit 1
}
$acct = Invoke-Az @("account", "show")
if ($null -eq $acct) {
    Fail "Not logged in. Run: az login"
    exit 1
}
Pass "Logged in as: $($acct.user.name)"
Pass "Subscription: $($acct.name) ($($acct.id))"
Pass "Tenant: $($acct.tenantId)"
$subId = $acct.id

# ----------------------------------------------------------------------------
Head "2. Resource providers registered"
foreach ($ns in @("Microsoft.Compute", "Microsoft.Network")) {
    $rp = Invoke-Az @("provider", "show", "--namespace", $ns)
    if ($null -eq $rp) { Warn "Could not query provider $ns (insufficient rights?)"; continue }
    if ($rp.registrationState -eq "Registered") { Pass "$ns = Registered" }
    else { Fail "$ns = $($rp.registrationState). Register: az provider register --namespace $ns" }
}

# ----------------------------------------------------------------------------
Head "2b. Standard security-type feature (opt out of Trusted Launch)"
Write-Host "  Gen2 VMs default to Trusted Launch; --security-type Standard needs a subscription feature flag." -ForegroundColor DarkGray
$featOk = $false
foreach ($fn in @("UseStandardSecurityType", "StandardSecurityTypeAsFirstClassEnum")) {
    $f = Invoke-Az @("feature", "show", "--namespace", "Microsoft.Compute", "--name", $fn)
    if ($null -eq $f) { continue }
    if ($f.properties.state -eq "Registered") { Pass "Microsoft.Compute/$fn = Registered"; $featOk = $true }
    else { Warn "Microsoft.Compute/$fn = $($f.properties.state)" }
}
if (-not $featOk) {
    Fail "Standard security type not enabled (Trusted Launch is forced). Owner can self-serve — register, wait, re-register provider:"
    Write-Host "    az feature register --namespace Microsoft.Compute --name UseStandardSecurityType" -ForegroundColor DarkGray
    Write-Host "    az feature show --namespace Microsoft.Compute --name UseStandardSecurityType --query properties.state -o tsv   # wait for 'Registered'" -ForegroundColor DarkGray
    Write-Host "    az provider register --namespace Microsoft.Compute" -ForegroundColor DarkGray
}

# ----------------------------------------------------------------------------
Head "3. RBAC (VM-deploy capability)"
Write-Host "  (Entra 'Application Administrator' is a directory role and does NOT grant VM deploy rights.)" -ForegroundColor DarkGray
$assignee = $acct.user.name
$oid = (& az ad signed-in-user show --query id -o tsv 2>$null)
if (-not [string]::IsNullOrWhiteSpace($oid)) { $assignee = $oid }
$roles = Invoke-Az @("role", "assignment", "list", "--assignee", $assignee, "--all",
                     "--query", "[].{role:roleDefinitionName, scope:scope}")
if ($null -eq $roles) {
    Warn "Could not list role assignments (need Microsoft.Authorization/roleAssignments/read). Verify RBAC manually."
} else {
    $capable = $roles | Where-Object { $_.role -in @("Owner", "Contributor", "Virtual Machine Contributor") }
    if ($capable) {
        $capable | ForEach-Object { Pass "$($_.role)  @ $($_.scope)" }
        if (($capable.role -contains "Virtual Machine Contributor") -and
            ($capable.role -notcontains "Owner") -and ($capable.role -notcontains "Contributor")) {
            if ($roles.role -notcontains "Network Contributor") {
                Warn "Have VM Contributor but no Network Contributor — auto-creating VNet/NSG/public IP may be denied."
            }
        }
    } else {
        Fail "No Owner/Contributor/Virtual Machine Contributor found. Request Contributor on a subscription or resource group."
        Write-Host "    Roles present: $([string]::Join(', ', ($roles.role | Select-Object -Unique)))" -ForegroundColor DarkGray
    }
}

# ----------------------------------------------------------------------------
Head "4. Compute quota ($Location)"
$usage = Invoke-Az @("vm", "list-usage", "-l", $Location)
$goodFamilies = @()
if ($null -eq $usage) {
    Fail "Could not read quota for $Location."
} else {
    $regional = $usage | Where-Object { $_.name.value -eq "cores" } | Select-Object -First 1
    $regFree = if ($regional) { [int]$regional.limit - [int]$regional.currentValue } else { 0 }
    if ($regional) {
        if ($regFree -ge $NeededVcpus) { Pass "Total Regional vCPUs: $($regional.currentValue)/$($regional.limit) used ($regFree free)" }
        else { Fail "Total Regional vCPUs: only $regFree free (need $NeededVcpus). Request a regional vCPU increase." }
    } else { Warn "Could not find Total Regional vCPUs entry." }

    foreach ($size in $Candidates.Keys) {
        $fam = $Candidates[$size]
        $u = $usage | Where-Object { $_.name.value -eq $fam } | Select-Object -First 1
        if ($null -eq $u) { Warn "$size ($fam): no quota entry (not offered here?)"; continue }
        $free = [int]$u.limit - [int]$u.currentValue
        if ($free -ge $NeededVcpus -and $regFree -ge $NeededVcpus) {
            Pass "$size ($fam): $($u.currentValue)/$($u.limit) used ($free free) -> GO"
            $goodFamilies += $size
        } elseif ([int]$u.limit -eq 0) {
            # Informational only: a zero-quota family you don't need must NOT fail the whole run.
            Warn "$size ($fam): limit 0 (no quota — fine if another family is GO)."
        } else {
            Warn "$size ($fam): only $free free (need $NeededVcpus — fine if another family is GO)."
        }
    }
    # Only a genuine blocker if NO candidate family has quota.
    if ($goodFamilies.Count -eq 0) {
        Fail "No candidate family has quota in $Location. Request >= $NeededVcpus vCPUs for a general-purpose family (or try another region with -Location)."
    }
}

# ----------------------------------------------------------------------------
Head "5. Azure Policy (Trusted Launch / Secure Boot blockers)"
Write-Host "  The plan REQUIRES --security-type Standard with Secure Boot OFF. A deny policy that" -ForegroundColor DarkGray
Write-Host "  mandates Trusted Launch / Secure Boot / vTPM will block provisioning." -ForegroundColor DarkGray
$pol = Invoke-Az @("policy", "assignment", "list",
                   "--query", "[].{name:displayName, id:name, enforce:enforcementMode}")
if ($null -eq $pol) {
    Warn "Could not list policy assignments. Confirm no Trusted-Launch mandate applies to your target RG."
} else {
    $kw = "secure boot|secureboot|trusted launch|trustedlaunch|vtpm|allowed location|allowed sku|allowed virtual machine"
    $flagged = $pol | Where-Object { ($_.name -match $kw) -or ($_.id -match $kw) }
    if ($flagged) {
        $flagged | ForEach-Object { Warn "Review policy: '$($_.name)' (enforcement=$($_.enforce))" }
        Write-Host "    -> If any DENY Standard security type, request a policy exemption on the target resource group." -ForegroundColor DarkGray
    } else {
        Pass "No obviously-blocking policy assignment found ($($pol.Count) assignment(s) scanned)."
        Write-Host "    (Keyword scan only — still confirm with your tenant admin if unsure.)" -ForegroundColor DarkGray
    }
}

# ----------------------------------------------------------------------------
Head "6. Gen2 Win11 image"
$img = Invoke-Az @("vm", "image", "show", "--urn", $ImageUrn)
if ($null -eq $img) {
    Warn "Could not resolve image '$ImageUrn'. Marketplace deploy may be disabled, or try: az vm image list --publisher MicrosoftWindowsDesktop --offer windows-11 --all -o table"
} else {
    if ($img.hyperVGeneration -eq "V2") { Pass "Image resolves, HyperV generation = V2 (Gen2)" }
    else { Warn "Image resolves but HyperV generation = $($img.hyperVGeneration) (expected V2/Gen2)" }
}

# ----------------------------------------------------------------------------
Head "7. Candidate size availability in $Location"
foreach ($size in $Candidates.Keys) {
    # No --size filter (it can over-filter to empty); match by exact name in JMESPath instead.
    $sku = Invoke-Az @("vm", "list-skus", "-l", $Location,
                       "--resource-type", "virtualMachines",
                       "--query", "[?name=='$size'] | [0]")
    if ($null -eq $sku) { Warn "${size}: not in $Location catalog (capacity/region, or query blocked)"; continue }
    $restr = @($sku.restrictions)
    if ($restr.Count -eq 0) { Pass "${size}: offered, no subscription restrictions"; continue }
    foreach ($r in $restr) {
        $locs  = ($r.restrictionInfo.locations -join ",")
        $zones = ($r.restrictionInfo.zones -join ",")
        if ($r.type -eq "Zone") {
            Warn "${size}: ZONE restriction ($($r.reasonCode); zones: $zones) — a non-zonal region deploy still works."
        } else {
            $script:locBlocked += $size
            Warn "${size}: LOCATION restriction ($($r.reasonCode); locations: $locs) — NOT deployable in $Location even with quota; use another region."
        }
    }
}

# ----------------------------------------------------------------------------
Head "VERDICT"
# Deployable = has quota AND not location-restricted (zone-restricted is fine for a non-zonal deploy).
$deployable = @($goodFamilies | Where-Object { $_ -notin $script:locBlocked })
if ($deployable.Count -gt 0 -and $script:fail -eq 0) {
    $pick = $deployable[0]
    Write-Host "GO — '$pick' has quota and no blocking checks failed." -ForegroundColor Green
    Write-Host "`nSuggested provisioning command (fill <rg>/<user>/<pass>):" -ForegroundColor White
    Write-Host @"
  az group create -n rg-nono-fltmgr-spike -l $Location
  az vm create ``
    --resource-group rg-nono-fltmgr-spike ``
    --name nono-fltmgr-vm ``
    --image $ImageUrn ``
    --size $pick ``
    --security-type Standard ``
    --enable-secure-boot false ``
    --admin-username <user> --admin-password <pass>
"@ -ForegroundColor Gray
    Write-Host "  Reminder: NOT TrustedLaunch; Secure Boot OFF; then on the VM: bcdedit /set testsigning on + reboot." -ForegroundColor DarkGray
    if ($script:locBlocked.Count -gt 0) {
        Write-Host "  (Note: $([string]::Join(', ', $script:locBlocked)) have quota but are location-restricted here; '$pick' was chosen because it is not.)" -ForegroundColor DarkGray
    }
    exit 0
} elseif ($goodFamilies.Count -gt 0 -and $deployable.Count -eq 0 -and $script:fail -eq 0) {
    Write-Host "NOT READY (region capacity) — families have quota ($([string]::Join(', ', $goodFamilies))) but ALL are LOCATION-restricted in $Location." -ForegroundColor Yellow
    Write-Host "Quota and permissions are fine — this is a regional SKU-availability block. Re-run against another region:" -ForegroundColor White
    Write-Host "  pwsh ./63-preflight-azure.ps1 -Location westus2" -ForegroundColor Gray
    Write-Host "  (also worth trying: eastus2, centralus, westus3)" -ForegroundColor DarkGray
    exit 1
} else {
    Write-Host "NOT READY — $($script:fail) blocking check(s) failed. Resolve the [FAIL] items above before provisioning." -ForegroundColor Red
    if ($goodFamilies.Count -gt 0) {
        Write-Host "Families with quota: $([string]::Join(', ', $goodFamilies))" -ForegroundColor DarkGray
    } else {
        Write-Host "No candidate family has quota in $Location. File the quota request (per 63-SC1-vm-state runbook) or try another region with -Location." -ForegroundColor DarkGray
    }
    exit 1
}
