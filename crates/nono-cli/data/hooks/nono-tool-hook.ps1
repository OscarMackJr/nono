$ErrorActionPreference = "Stop"

$inputJson = [Console]::In.ReadToEnd()
$nono = if ($env:NONO_EXE) { $env:NONO_EXE } else { "nono" }

# Capture the nono process's stdout and stderr on SEPARATE channels.
# stdout is the JSON decision contract Claude Code parses; it must never be
# polluted by nono's tracing/log output (which goes to stderr and is gated by
# NONO_LOG / RUST_LOG). We redirect native stderr to a temp file rather than
# merging it into stdout (which corrupted the JSON contract; finding R-A1).
$stderrFile = [System.IO.Path]::GetTempFileName()
try {
    # Native stderr (file descriptor 2) -> temp file. $stdout captures ONLY stdout.
    #
    # IMPORTANT: under $ErrorActionPreference = "Stop", a native command that
    # writes ANYTHING to stderr raises a terminating NativeCommandError even on
    # exit 0. We relax the preference to "Continue" ONLY around the native call
    # so that nono's diagnostic stderr does not abort the success path. We still
    # fail CLOSED below by explicitly inspecting $LASTEXITCODE, then restore Stop.
    $prevEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $stdout = $inputJson | & $nono claude-code-hook 2>$stderrFile
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $prevEAP

    if ($exitCode -eq 0) {
        # Success: emit ONLY the captured stdout JSON. Stderr (diagnostic logs)
        # is intentionally discarded so it can never corrupt the contract.
        if ($stdout -is [array]) {
            $stdout -join [Environment]::NewLine
        } else {
            $stdout
        }
        exit 0
    }

    # Non-zero exit: fail CLOSED. Surface the captured stderr (and any stdout)
    # in the reason for diagnosis, but keep our own stdout a clean deny JSON.
    $stderrText = if (Test-Path $stderrFile) { (Get-Content -Raw -ErrorAction SilentlyContinue $stderrFile) } else { "" }
    $detail = @($stderrText, ($stdout -join [Environment]::NewLine)) | Where-Object { $_ } | ForEach-Object { $_.Trim() } | Where-Object { $_ }
    $reason = "nono Claude Code hook handler failed closed (exit $exitCode): $($detail -join [Environment]::NewLine)"
} catch {
    $reason = "nono Claude Code hook handler failed closed: $($_.Exception.Message)"
} finally {
    if (Test-Path $stderrFile) {
        Remove-Item -Force -ErrorAction SilentlyContinue $stderrFile
    }
}

@{
    hookSpecificOutput = @{
        hookEventName = "PreToolUse"
        permissionDecision = "deny"
        permissionDecisionReason = $reason
    }
} | ConvertTo-Json -Depth 4 -Compress
exit 0
