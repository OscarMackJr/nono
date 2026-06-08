# Runs ON the Azure VM via: az vm run-command invoke --scripts @<this file>
# SC2 part 1 of 2: download the self-contained EWDK ISO (Win11 26H1, VS BuildTools 18.3.0).
# ~15 GB; Azure egress to the Microsoft CDN is fast (usually a few minutes). Run ONCE.
$ErrorActionPreference = "Continue"

$dir = "C:\ewdk"
$dst = "$dir\ewdk.iso"
$url = "https://go.microsoft.com/fwlink/?LinkId=2362109"   # EWDK 26H1 ISO (follows redirect to the CDN)

New-Item -ItemType Directory -Force -Path $dir | Out-Null

if (Test-Path $dst) {
    $existing = (Get-Item $dst).Length
    Write-Output ("Existing ewdk.iso: {0:N2} GB" -f ($existing/1GB))
    if ($existing -gt 5GB) { Write-Output "Already downloaded (>5 GB). Skipping. Delete C:\ewdk\ewdk.iso to force re-download."; return }
    Write-Output "Existing file is too small — re-downloading."
    Remove-Item $dst -Force
}

Write-Output "Downloading EWDK ISO via curl (-L follows the fwlink redirect)..."
$sw = [System.Diagnostics.Stopwatch]::StartNew()
# curl.exe ships in Windows 11; robust for large files, follows redirects, retries.
curl.exe -L --retry 3 --retry-delay 5 --fail --silent --show-error -o $dst $url
$code = $LASTEXITCODE
$sw.Stop()

if (-not (Test-Path $dst)) { Write-Output "DOWNLOAD FAILED (curl exit $code) — no file produced."; return }
$len = (Get-Item $dst).Length
Write-Output ("curl exit: {0} | size: {1:N2} GB | elapsed: {2:N1} min" -f $code, ($len/1GB), $sw.Elapsed.TotalMinutes)
if ($len -lt 5GB) {
    Write-Output "WARNING: file < 5 GB — likely the license HTML page, not the ISO. First 200 bytes:"
    Get-Content $dst -TotalCount 5 -ErrorAction SilentlyContinue
    Write-Output "If this is HTML, the fwlink needs a manual accept; tell the orchestrator."
} else {
    Write-Output "OK: EWDK ISO downloaded. Proceed to the build script."
}
