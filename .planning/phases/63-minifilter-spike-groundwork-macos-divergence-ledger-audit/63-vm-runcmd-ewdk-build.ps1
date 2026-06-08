# Runs ON the Azure VM via: az vm run-command invoke --scripts @<this file>
# SC2 part 2 of 2: mount the EWDK ISO, pull the nono-fltmgr scaffold from the public repo,
# build it to nono-fltmgr.sys, and report the result. Re-runnable (does not re-download the ISO).
# ASCII-ONLY: the run-command @file transport corrupts non-ASCII chars (no em-dashes / smart quotes).
$ErrorActionPreference = "Continue"

$ewdkIso = "C:\ewdk\ewdk.iso"
$src     = "C:\nono-fltmgr"
$log     = "$src\msbuild.log"
$repo    = "https://raw.githubusercontent.com/OscarMackJr/nono/main/drivers/nono-fltmgr"
$files   = "nono-fltmgr.c","nono-fltmgr.vcxproj","nono-fltmgr.vcxproj.filters","nono-fltmgr.inf"

if (-not (Test-Path $ewdkIso)) { Write-Output "ERROR: $ewdkIso not found. Run the download script first."; return }

# 1. Fetch the scaffold from the public repo (the exact committed Plan 63-01 files).
New-Item -ItemType Directory -Force -Path $src | Out-Null
Write-Output "=== Fetching scaffold from repo ==="
foreach ($f in $files) {
    try { Invoke-WebRequest -Uri "$repo/$f" -OutFile (Join-Path $src $f) -UseBasicParsing -ErrorAction Stop; Write-Output ("  got " + $f) }
    catch { Write-Output ("  FAILED " + $f + " : " + $_.Exception.Message) }
}
Get-ChildItem $src -File | Where-Object { $_.Extension -ne ".log" } | Format-Table Name,Length -AutoSize | Out-String | Write-Output

# 2. Mount the EWDK ISO.
Write-Output "=== Mounting EWDK ISO ==="
$img = Mount-DiskImage -ImagePath $ewdkIso -PassThru -ErrorAction SilentlyContinue
Start-Sleep -Seconds 4
$vol = $img | Get-Volume
$drive = $vol.DriveLetter
if (-not $drive) { Write-Output "ERROR: could not determine EWDK mount drive letter."; return }
Write-Output ("EWDK mounted at " + $drive + ":")
if (-not (Test-Path ("${drive}:\LaunchBuildEnv.cmd"))) {
    Write-Output ("ERROR: " + $drive + ":\LaunchBuildEnv.cmd not found. ISO may be wrong/corrupt.")
    Get-ChildItem ("${drive}:\") -ErrorAction SilentlyContinue | Select-Object Name | Out-String | Write-Output
    Dismount-DiskImage -ImagePath $ewdkIso | Out-Null
    return
}

# 3. Build inside the EWDK environment. LaunchBuildEnv.cmd <command> runs the command in the
#    configured env and exits (non-interactive form).
$proj = "$src\nono-fltmgr.vcxproj"
Write-Output "=== Building (msbuild Release/x64) ==="
$buildLine = "`"${drive}:\LaunchBuildEnv.cmd`" msbuild `"$proj`" /p:Configuration=Release /p:Platform=x64 /v:minimal /nologo"
Write-Output ("cmd: " + $buildLine)
cmd.exe /c $buildLine *> $log
$buildExit = $LASTEXITCODE
Write-Output ("=== msbuild exit code: {0} ===" -f $buildExit)

# 4. Report.
Write-Output "=== msbuild output (tail 30) ==="
if (Test-Path $log) { Get-Content $log -Tail 30 | Out-String | Write-Output } else { Write-Output "(no log produced)" }

$sys = "$src\x64\Release\nono-fltmgr.sys"
Write-Output "=== .sys check ==="
if (Test-Path $sys) {
    Get-Item $sys | Format-List Name,Length,LastWriteTime | Out-String | Write-Output
    Write-Output ("SC2 PASS: nono-fltmgr.sys produced ({0:N0} bytes), msbuild exit {1}" -f (Get-Item $sys).Length, $buildExit)
} else {
    Write-Output "SC2 FAIL: nono-fltmgr.sys NOT found. Output tree under x64:"
    Get-ChildItem -Recurse ("$src\x64") -ErrorAction SilentlyContinue | Select-Object FullName | Out-String | Write-Output
}

# 5. Toolchain version (evidence) + dismount.
Write-Output "=== Toolchain ==="
& cmd.exe /c ("`"${drive}:\LaunchBuildEnv.cmd`" msbuild -version -nologo") 2>$null | Select-Object -Last 3 | Out-String | Write-Output
Dismount-DiskImage -ImagePath $ewdkIso | Out-Null
Write-Output "EWDK dismounted. The .sys is a throwaway VM-local artifact (do NOT copy it back / commit it)."
