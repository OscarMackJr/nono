# Runs ON the Azure VM via: az vm run-command invoke --scripts @<this file>
# Phase 64 variant of 63-vm-runcmd-ewdk-build.ps1: builds the LOCAL copy of the
# nono-fltmgr scaffold already staged at C:\nono-fltmgr (does NOT fetch from the
# repo), so you can build Phase 64 changes WITHOUT pushing to main first.
# Mounts the EWDK ISO, sets env non-interactively via SetupBuildEnv.cmd inside a
# wrapper with a HARD 7-minute timeout (can never hang on an interactive shell).
# Re-runnable. ASCII-ONLY (the run-command @file transport corrupts non-ASCII).
$ErrorActionPreference = "Continue"

$ewdkIso = "C:\ewdk\ewdk.iso"
$src     = "C:\nono-fltmgr"
$log     = "$src\msbuild.log"
$err     = "$src\msbuild.err"
$wrapper = "$src\dobuild.cmd"
$proj    = "$src\nono-fltmgr.vcxproj"
$required = "nono-fltmgr.c","nono-fltmgr.vcxproj","nono-fltmgr.vcxproj.filters","nono-fltmgr.inf"

if (-not (Test-Path $ewdkIso)) { Write-Output "ERROR: $ewdkIso not found. Run 63-vm-runcmd-ewdk-download.ps1 first."; return }
if (-not (Test-Path $proj))    { Write-Output "ERROR: $proj not found. Copy drivers\nono-fltmgr\ to C:\nono-fltmgr on the VM first."; return }

# 0. Clear any leftover hung build processes from a previous attempt.
Get-Process msbuild,cl,link,tracker -ErrorAction SilentlyContinue | ForEach-Object { try { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue } catch {} }

# 1. Verify the LOCAL scaffold is present (no fetch). Report what is staged.
Write-Output "=== Local scaffold at $src ==="
foreach ($f in $required) {
    $p = Join-Path $src $f
    if (Test-Path $p) { Write-Output ("  present " + $f + " (" + (Get-Item $p).Length + " bytes)") }
    else              { Write-Output ("  MISSING " + $f + " -- copy it to " + $src) }
}
# Show the altitude actually staged in the INF (Phase 64 D-08 check).
$inf = Join-Path $src "nono-fltmgr.inf"
if (Test-Path $inf) { (Select-String -Path $inf -Pattern "Instance1.Altitude").Line | ForEach-Object { Write-Output ("  INF altitude: " + $_.Trim()) } }

# 2. Mount the EWDK ISO (dismount first in case a prior run left it mounted).
try { Dismount-DiskImage -ImagePath $ewdkIso -ErrorAction SilentlyContinue | Out-Null } catch {}
$img = Mount-DiskImage -ImagePath $ewdkIso -PassThru -ErrorAction SilentlyContinue
Start-Sleep -Seconds 4
$drive = ($img | Get-Volume).DriveLetter
if (-not $drive) { Write-Output "ERROR: could not determine EWDK mount drive letter."; return }
Write-Output ("EWDK mounted at " + $drive + ":")

# 3. Find the non-interactive env-setup script (known paths first; shallow fallback only).
$setup = $null
$known = @("${drive}:\BuildEnv\SetupBuildEnv.cmd", "${drive}:\SetupBuildEnv.cmd")
foreach ($k in $known) { if (Test-Path $k) { $setup = Get-Item $k; break } }
if (-not $setup) {
    $setup = Get-ChildItem ("${drive}:\") -Filter "SetupBuildEnv.cmd" -Depth 2 -ErrorAction SilentlyContinue | Select-Object -First 1
}
Write-Output ("SetupBuildEnv.cmd: " + $(if ($setup) { $setup.FullName } else { "NOT FOUND (will fall back to LaunchBuildEnv)" }))

# 4. Write a wrapper .cmd: set env (non-interactive), then msbuild. Never drops to an interactive shell.
if ($setup) {
@"
@echo off
call "$($setup.FullName)"
echo SETUP_EXIT=%errorlevel%
msbuild "$proj" /p:Configuration=Release /p:Platform=x64 /v:minimal /nologo
echo MSBUILD_EXIT=%errorlevel%
"@ | Set-Content -Path $wrapper -Encoding ASCII
} else {
@"
@echo off
call "${drive}:\LaunchBuildEnv.cmd" msbuild "$proj" /p:Configuration=Release /p:Platform=x64 /v:minimal /nologo
echo MSBUILD_EXIT=%errorlevel%
"@ | Set-Content -Path $wrapper -Encoding ASCII
}

# 5. Run with a HARD 7-minute timeout (cannot hang).
Write-Output "=== Building LOCAL copy (max 420s) ==="
Remove-Item $log,$err -ErrorAction SilentlyContinue
$p = Start-Process -FilePath "cmd.exe" -ArgumentList "/c","`"$wrapper`"" -RedirectStandardOutput $log -RedirectStandardError $err -PassThru -WindowStyle Hidden
if (-not $p.WaitForExit(420000)) {
    Write-Output "BUILD TIMED OUT after 420s -> killing process tree (likely an interactive prompt)."
    taskkill /T /F /PID $p.Id 2>$null | Out-Null
    Start-Sleep -Seconds 2
}
$buildExit = try { $p.ExitCode } catch { "killed" }
Write-Output ("=== build wrapper exit: {0} ===" -f $buildExit)

# 6. Report.
Write-Output "=== msbuild stdout (tail 30) ==="
if (Test-Path $log) { Get-Content $log -Tail 30 | Out-String | Write-Output } else { Write-Output "(no stdout log)" }
if ((Test-Path $err) -and (Get-Item $err).Length -gt 0) { Write-Output "=== stderr (tail 10) ==="; Get-Content $err -Tail 10 | Out-String | Write-Output }

$sys = "$src\x64\Release\nono-fltmgr.sys"
Write-Output "=== .sys check ==="
if (Test-Path $sys) {
    Get-Item $sys | Format-List Name,Length,LastWriteTime | Out-String | Write-Output
    Write-Output ("BUILD PASS: nono-fltmgr.sys produced ({0:N0} bytes)" -f (Get-Item $sys).Length)
} else {
    Write-Output "BUILD FAIL: nono-fltmgr.sys NOT found. Output tree under x64:"
    Get-ChildItem -Recurse ("$src\x64") -ErrorAction SilentlyContinue | Select-Object FullName | Out-String | Write-Output
}

# 7. Dismount.
Dismount-DiskImage -ImagePath $ewdkIso | Out-Null
Write-Output "EWDK dismounted. (.sys is throwaway VM-local; do NOT copy back / commit.)"
