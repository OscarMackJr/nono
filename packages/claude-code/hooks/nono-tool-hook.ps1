$ErrorActionPreference = "Stop"

$inputJson = [Console]::In.ReadToEnd()
$nono = if ($env:NONO_EXE) { $env:NONO_EXE } else { "nono" }

try {
    $output = $inputJson | & $nono claude-code-hook 2>&1
    if ($LASTEXITCODE -eq 0) {
        $output
        exit 0
    }

    $reason = "nono Claude Code hook handler failed closed: $($output -join [Environment]::NewLine)"
} catch {
    $reason = "nono Claude Code hook handler failed closed: $($_.Exception.Message)"
}

@{
    hookSpecificOutput = @{
        hookEventName = "PreToolUse"
        permissionDecision = "deny"
        permissionDecisionReason = $reason
    }
} | ConvertTo-Json -Depth 4 -Compress
exit 0
