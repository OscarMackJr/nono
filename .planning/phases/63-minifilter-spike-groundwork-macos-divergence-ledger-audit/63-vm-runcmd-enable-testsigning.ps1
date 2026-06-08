# Runs ON the Azure VM via: az vm run-command invoke --scripts @<this file>
# Enables test-signing (Pitfall A check: must succeed with Secure Boot OFF), then reboots.
$ErrorActionPreference = "Continue"

Write-Output "=== bcdedit /set testsigning on ==="
$out = bcdedit /set testsigning on 2>&1
Write-Output $out
Write-Output ("ExitCode: " + $LASTEXITCODE)

if ($out -match "secure boot|Secure Boot|policy") {
    Write-Output "!!! Secure-Boot-policy error detected -> VM was provisioned as Trusted Launch. Reprovision as Standard."
} else {
    Write-Output "OK: test-signing set without a Secure-Boot-policy error (Pitfall A cleared)."
}

Write-Output "=== bcdedit after set (testsigning should now read 'Yes') ==="
bcdedit

Write-Output "=== Rebooting to make test-signing effective ==="
Start-Sleep -Seconds 3
Restart-Computer -Force
