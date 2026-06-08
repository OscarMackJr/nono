# Runs ON the Azure VM via: az vm run-command invoke --scripts @<this file>
# SC2 part 2 of 2: mount EWDK, pull the nono-fltmgr scaffold from the public repo, build to .sys.
# Uses SetupBuildEnv.cmd (non-interactive env setup) inside a wrapper, with a HARD timeout so it
# can never hang on an interactive shell. Re-runnable. ASCII-ONLY.
$ErrorActionPreference = "Continue"

$ewdkIso = "C:\ewdk\ewdk.iso"
$src     = "C:\nono-fltmgr"
$log     = "$src\msbuild.log"
$err     = "$src\msbuild.err"
$wrapper = "$src\dobuild.cmd"
$repo    = "https://raw.githubusercontent.com/OscarMackJr/nono/main/drivers/nono-fltmgr"
$files   = "nono-fltmgr.c","nono-fltmgr.vcxproj","nono-fltmgr.vcxproj.filters","nono-fltmgr.inf"
$proj    = "$src\nono-fltmgr.vcxproj"

if (-not (Test-Path $ewdkIso)) { Write-Output "ERROR: $ewdkIso not found. Run the download script first."; return }

# 0. Clear any leftover hung build processes from the previous attempt.
Get-Process msbuild,cl,link,tracker -ErrorAction SilentlyContinue | ForEach-Object { try { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue } catch {} }

# 1. Fetch the scaffold from the public repo.
New-Item -ItemType Directory -Force -Path $src | Out-Null
Write-Output "=== Fetching scaffold ==="
foreach ($f in $files) {
    try { Invoke-WebRequest -Uri "$repo/$f" -OutFile (Join-Path $src $f) -UseBasicParsing -ErrorAction Stop; Write-Output ("  got " + $f) }
    catch { Write-Output ("  FAILED " + $f + " : " + $_.Exception.Message) }
}

# 2. Mount the EWDK ISO (dismount first in case the prior run left it mounted).
try { Dismount-DiskImage -ImagePath $ewdkIso -ErrorAction SilentlyContinue | Out-Null } catch {}
$img = Mount-DiskImage -ImagePath $ewdkIso -PassThru -ErrorAction SilentlyContinue
Start-Sleep -Seconds 4
$drive = ($img | Get-Volume).DriveLetter
if (-not $drive) { Write-Output "ERROR: could not determine EWDK mount drive letter."; return }
Write-Output ("EWDK mounted at " + $drive + ":")
Write-Output "=== EWDK root contents ==="
Get-ChildItem ("${drive}:\") -ErrorAction SilentlyContinue | Select-Object Name | Out-String | Write-Output

# 3. Find the non-interactive env-setup script.
$setup = Get-ChildItem ("${drive}:\") -Recurse -Filter "SetupBuildEnv.cmd" -ErrorAction SilentlyContinue | Select-Object -First 1
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
Write-Output "=== Building (max 420s) ==="
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
    Write-Output ("SC2 PASS: nono-fltmgr.sys produced ({0:N0} bytes)" -f (Get-Item $sys).Length)
} else {
    Write-Output "SC2 FAIL: nono-fltmgr.sys NOT found. Output tree under x64:"
    Get-ChildItem -Recurse ("$src\x64") -ErrorAction SilentlyContinue | Select-Object FullName | Out-String | Write-Output
}

# 7. Dismount.
Dismount-DiskImage -ImagePath $ewdkIso | Out-Null
Write-Output "EWDK dismounted. (.sys is throwaway VM-local; do NOT copy back / commit.)"
