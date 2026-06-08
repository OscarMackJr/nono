# Runs ON the Azure VM via: az vm run-command invoke --scripts @<this file>
# SC1 state read (non-destructive): test-signing, Secure Boot, HVCI/Device Guard, OS.
$ErrorActionPreference = "Continue"

Write-Output "=== TESTSIGNING / current boot entry (bcdedit) ==="
bcdedit

Write-Output ""
Write-Output "=== Secure Boot state ==="
try {
    $sb = Confirm-SecureBootUEFI
    Write-Output ("Secure Boot enabled: " + $sb)
} catch {
    Write-Output "Secure Boot: NOT enabled / not supported (EXPECTED for Standard security type)"
}

Write-Output ""
Write-Output "=== HVCI / Device Guard (Memory Integrity) ==="
$dg = Get-CimInstance -ClassName Win32_DeviceGuard -Namespace root\Microsoft\Windows\DeviceGuard -ErrorAction SilentlyContinue
if ($dg) {
    Write-Output ("VBS status (0=Off, 1=Configured-not-running, 2=Running): " + $dg.VirtualizationBasedSecurityStatus)
    $running = @($dg.SecurityServicesRunning)
    if ($running.Count -eq 0) {
        Write-Output "SecurityServicesRunning: (none) -> HVCI OFF"
    } else {
        Write-Output ("SecurityServicesRunning (1=CredGuard, 2=HVCI/Memory-Integrity): " + ($running -join ","))
    }
} else {
    Write-Output "Win32_DeviceGuard: not present -> HVCI OFF"
}

Write-Output ""
Write-Output "=== OS / build ==="
$os = Get-CimInstance Win32_OperatingSystem
Write-Output ($os.Caption + "  |  Version " + $os.Version + "  |  Build " + $os.BuildNumber)
