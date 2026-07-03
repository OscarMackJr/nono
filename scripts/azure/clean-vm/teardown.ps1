param(
    # Must match deploy.ps1's default so a plain `.\teardown.ps1` tears down exactly what
    # `.\deploy.ps1` stood up. NEVER point this at the live `RG_Nono`, which holds the
    # permanent `ArtifactNono` code-signing account.
    [string]$ResourceGroupName = 'RG_Nono_CleanHost',

    [switch]$Force
)

# scripts/azure/clean-vm/teardown.ps1
#
# Phase 103 (CHOST-01) - the ONLY supported cleanup path for the Phase 103 clean-host VM IaC.
#
# NOT INVOKED LIVE in Phase 103 - authored + syntax-checked only this phase
# (`[System.Management.Automation.Language.Parser]::ParseFile`, no execution). Phase 106 runs it
# for real, once per UAT session, immediately after the clean-host gates have run.
#
# Ephemeral lifecycle (T-103-02): a VM stood up by deploy.ps1 must never be left running past a
# single UAT session. This script blocks until `az group delete` actually completes (it never
# queues the deletion asynchronously), so the script's own successful exit is itself the proof
# that the ephemeral resource group - and everything in it - is genuinely gone.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $Force) {
    $confirmation = Read-Host "This will PERMANENTLY DELETE resource group '$ResourceGroupName' and everything in it. Type the resource group name to confirm"
    if ($confirmation -ne $ResourceGroupName) {
        throw "Confirmation did not match '$ResourceGroupName' - aborting teardown. Re-run with -Force to skip this prompt."
    }
}

Write-Host "Deleting resource group $ResourceGroupName (blocking until deletion completes) ..."
az group delete --name $ResourceGroupName --yes

Write-Host "Teardown complete: $ResourceGroupName is gone. This is the only supported cleanup path for the Phase 103 clean-host VM IaC."
