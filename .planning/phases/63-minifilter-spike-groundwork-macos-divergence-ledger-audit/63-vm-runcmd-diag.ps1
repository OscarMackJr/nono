# Diagnostic: is the build stuck? Run via the MANAGED run-command (runs concurrently
# with the blocked legacy `invoke`). ASCII-only.
$ErrorActionPreference = "Continue"

Write-Output "=== Build-related processes (CPU + how long running) ==="
Get-Process cmd,conhost,msbuild,cl,link,tracker,vctip,VsDevCmd,powershell -ErrorAction SilentlyContinue |
    Select-Object Name, Id, CPU, @{n='RunMin';e={ if ($_.StartTime) { [math]::Round(((Get-Date)-$_.StartTime).TotalMinutes,1) } }} |
    Format-Table -AutoSize | Out-String | Write-Output

Write-Output "=== msbuild.log (is it still growing?) ==="
$log = "C:\nono-fltmgr\msbuild.log"
if (Test-Path $log) {
    $li = Get-Item $log
    Write-Output ("size={0} bytes  lastWrite={1}  staleSec={2:N0}" -f $li.Length, $li.LastWriteTime, ((Get-Date)-$li.LastWriteTime).TotalSeconds)
    Write-Output "--- tail 25 ---"
    Get-Content $log -Tail 25 -ErrorAction SilentlyContinue | Out-String | Write-Output
} else {
    Write-Output "no msbuild.log yet (build never reached the redirect, or cmd is hung before it)"
}

Write-Output "=== .sys produced yet? ==="
Write-Output ("nono-fltmgr.sys exists: " + (Test-Path "C:\nono-fltmgr\x64\Release\nono-fltmgr.sys"))

Write-Output "=== Mounted volumes (EWDK should be a CD-ROM volume) ==="
Get-Volume -ErrorAction SilentlyContinue | Where-Object { $_.DriveType -eq 'CD-ROM' } |
    Select-Object DriveLetter, FileSystemLabel, @{n='SizeGB';e={[math]::Round($_.Size/1GB,1)}} |
    Format-Table -AutoSize | Out-String | Write-Output
