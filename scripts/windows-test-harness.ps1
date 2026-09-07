param(
    [ValidateSet("build", "smoke", "integration", "security", "regression", "all")]
    [string]$Suite = "all",
    [string]$LogDir = "ci-logs"
)

$ErrorActionPreference = "Stop"
# Cargo and other native tools write normal progress output to stderr.
# Keep that from being promoted into terminating PowerShell errors while we tee logs.
$PSNativeCommandUseErrorActionPreference = $false

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# ── R-B3 precondition: own the workspace before running any live `nono run` ──
#
# nono's R-B3 gate refuses to start when the current user lacks WRITE_OWNER on
# the workspace (== the child CWD, D-06), because applying a mandatory integrity
# label needs it. The gate is CORRECT; the problem is the environment. GitHub's
# windows-latest runner is ELEVATED, so the checked-out tree is owned by
# BUILTIN\Administrators rather than the runner user, and
# `nono::path_is_owned_by_current_user` is a token-USER-SID equality test that
# says "not yours". Every live supervised run then dies with:
#
#   R-B3: the current user lacks WRITE_OWNER (0x00080000) on the workspace:
#   \\?\D:\a\nono\nono\crates\nono-cli
#
# This is fixed HERE, once, rather than in the ~60 tests in env_vars.rs that run
# `nono` from the process CWD. Patching those individually is both large and
# wrong: several (windows_run_honors_workdir, the relative-path allowlist tests)
# depend on the inherited CWD by design, so forcing each into a tempdir would
# break the thing they exist to test. The workspace is not a fixture any test
# created -- it is the checkout -- so ownership belongs at the environment layer.
# Placing it in the harness rather than in ci.yml means all three jobs that call
# this script (smoke, integration, regression) are covered by one edit, and any
# suite added later inherits it.
#
# `icacls /setowner` needs no privilege when the caller can already take
# ownership -- the owner itself, or a member of the owning group, which is
# exactly the elevated-runner case. Non-recursive on purpose: R-B3 inspects the
# workspace DIRECTORY, not its contents, so /T would cost minutes over the whole
# tree for no benefit.
#
# Fails LOUD. A silent no-op here would surface later as an R-B3 failure that
# looks like a product defect rather than a setup gap.
$repoRootForOwn = (Resolve-Path -LiteralPath $PSScriptRoot\..).Path
$currentPrincipal = (whoami).Trim()
if ([string]::IsNullOrWhiteSpace($currentPrincipal)) {
    throw "R-B3 precondition: `whoami` returned nothing; cannot take workspace ownership."
}
foreach ($ownTarget in @($repoRootForOwn, (Join-Path $repoRootForOwn "crates\nono-cli"))) {
    if (-not (Test-Path -LiteralPath $ownTarget)) { continue }
    & icacls $ownTarget /setowner "$currentPrincipal" /Q | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "R-B3 precondition: icacls /setowner '$ownTarget' -> '$currentPrincipal' failed with exit $LASTEXITCODE."
    }
    Write-Host "R-B3 precondition: $currentPrincipal now owns $ownTarget"
}

function Invoke-LoggedCargo {
    param(
        [string]$LogFile,
        [string]$Label,
        [string[]]$CargoArgs
    )

    $logPath = Join-Path $LogDir $LogFile
    "==> $Label" | Tee-Object -FilePath $logPath -Append
    "`$ cargo $($CargoArgs -join ' ')" | Tee-Object -FilePath $logPath -Append
    $stdoutPath = Join-Path $LogDir ([System.Guid]::NewGuid().ToString() + ".stdout.tmp")
    $stderrPath = Join-Path $LogDir ([System.Guid]::NewGuid().ToString() + ".stderr.tmp")
    $process = Start-Process -FilePath "cargo" `
        -ArgumentList $CargoArgs `
        -NoNewWindow `
        -Wait `
        -PassThru `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath
    foreach ($capturePath in @($stdoutPath, $stderrPath)) {
        if (Test-Path $capturePath) {
            Get-Content $capturePath | Tee-Object -FilePath $logPath -Append
            Remove-Item -LiteralPath $capturePath -Force
        }
    }
    if ($process.ExitCode -ne 0) {
        throw "Cargo command failed for $Label with exit code $($process.ExitCode)"
    }
    "" | Tee-Object -FilePath $logPath -Append | Out-Null
}

function Invoke-TestList {
    param(
        [string]$LogFile,
        [object[]]$Tests
    )

    foreach ($test in $Tests) {
        $pkg = $test.Package
        $filter = $test.Filter
        Invoke-LoggedCargo -LogFile $LogFile -Label "$pkg::$filter" -CargoArgs @(
            "test",
            "-p",
            $pkg,
            $filter,
            "--",
            "--nocapture"
        )
    }
}

function Invoke-LoggedCommand {
    param(
        [string]$LogFile,
        [string]$Label,
        [scriptblock]$Command
    )

    $logPath = Join-Path $LogDir $LogFile
    "==> $Label" | Tee-Object -FilePath $logPath -Append
    $capturePath = Join-Path $LogDir ([System.Guid]::NewGuid().ToString() + ".tmp")
    & $Command *> $capturePath
    if (Test-Path $capturePath) {
        Get-Content $capturePath | Tee-Object -FilePath $logPath -Append
        Remove-Item -LiteralPath $capturePath -Force
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed for $Label with exit code $LASTEXITCODE"
    }
    "" | Tee-Object -FilePath $logPath -Append | Out-Null
}

$smokeTests = @(
    @{ Package = "nono-sandbox-cli"; Filter = "test_root_help_mentions_windows_restricted_execution_surface" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_setup_check_only_reports_live_profile_subset" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_setup_check_only_reports_unified_support_status" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_executes_basic_command" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_live_default_profile_executes_command" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_shell_help_reports_documented_limitation" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_wrap_help_reports_documented_limitation" }
)

$integrationTests = @(
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_redirects_profile_state_vars_into_writable_allowlist" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_honors_workdir" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_live_codex_profile_fails_intentionally_with_backend_reason" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_supervised_rollback_executes_command" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_smoke_validates_stdout_stderr_and_exit_code" }
)

$securityTests = @(
    @{ Package = "nono-sandbox"; Filter = "validate_preview_entry_point_rejects_shell" },
    @{ Package = "nono-sandbox"; Filter = "validate_preview_entry_point_rejects_wrap" },
    @{ Package = "nono-sandbox"; Filter = "validate_command_args_rejects_relative_parent_escape_outside_policy" },
    @{ Package = "nono-sandbox"; Filter = "validate_command_args_rejects_symlink_escape_inside_policy" },
    @{ Package = "nono-sandbox"; Filter = "validate_command_args_rejects_junction_escape_inside_policy" },
    @{ Package = "nono-sandbox-cli"; Filter = "test_handle_windows_supervisor_message_rejects_duplicate_request_ids" },
    @{ Package = "nono-sandbox-cli"; Filter = "test_handle_windows_supervisor_message_reports_open_url_limitation" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_open_url_helper_reports_documented_limitation" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_block_net_blocks_probe_connection" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_block_net_cleans_up_promoted_wfp_filters_after_exit" }
)

$regressionTests = @(
    @{ Package = "nono-sandbox"; Filter = "normalize_windows_path_strips_verbatim_prefix" },
    @{ Package = "nono-sandbox"; Filter = "normalize_windows_path_strips_unc_verbatim_prefix" },
    @{ Package = "nono-sandbox"; Filter = "windows_paths_start_with_case_insensitive_matches_drive_case" },
    @{ Package = "nono-sandbox"; Filter = "low_integrity_compatible_dir_matches_localappdata_temp_low" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_protected_path_check_handles_verbatim_prefix_and_case_insensitive_drive_letters" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_path_overlaps_filter_handles_verbatim_prefix_and_drive_case" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_prefers_managed_low_integrity_runtime_root_inside_allowlist" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_redirects_temp_vars_into_writable_allowlist" },
    @{ Package = "nono-sandbox-cli"; Filter = "windows_run_redirects_profile_state_vars_into_writable_allowlist" },
    @{ Package = "nono-sandbox-cli"; Filter = "config_with_valid_manifest_is_accepted" },
    @{ Package = "nono-sandbox-cli"; Filter = "test_show_format_manifest_round_trip" }
)

$suites = if ($Suite -eq "all") {
    @("build", "smoke", "integration", "security", "regression")
} else {
    @($Suite)
}

foreach ($activeSuite in $suites) {
    switch ($activeSuite) {
        "build" {
            Invoke-LoggedCargo -LogFile "windows-build.log" -Label "build workspace" -CargoArgs @(
                "build",
                "--workspace",
                "--verbose"
            )
            # Phase 41 Plan 08 (REQ-CI-02 gap closure): explicitly pre-build the broker so
            # `target\debug\nono-shell-broker.exe` is guaranteed to exist before the MSI validator
            # runs. Defense-in-depth against future workspace-build configuration changes that
            # might exclude the broker crate. Mirrors the windows-packaging CI lane pattern
            # established by Plan 41-03 (.github/workflows/ci.yml:334-338).
            Invoke-LoggedCargo -LogFile "windows-build.log" -Label "build nono-shell-broker" -CargoArgs @(
                "build",
                "-p",
                "nono-shell-broker"
            )
            Invoke-LoggedCommand -LogFile "windows-build.log" -Label "validate windows msi contract" -Command {
                # Phase 41 Plan 08 (REQ-CI-02 gap closure): validate-windows-msi-contract.ps1 made
                # `-BrokerPath` mandatory in Plan 41-03 (validator line 8). Without `-BrokerPath`,
                # PowerShell rejects with "Cannot process command because of one or more missing
                # mandatory parameters: BrokerPath" and the GH Actions windows-build job fails
                # every run. Pass the workspace's debug-built broker artifact and fail-secure if
                # it is missing (CLAUDE.md Fail Secure principle).
                $brokerPath = Join-Path $PWD "target\debug\nono-shell-broker.exe"
                if (-not (Test-Path -LiteralPath $brokerPath)) {
                    Write-Error "nono-shell-broker.exe missing at $brokerPath; MSI validator cannot proceed. The 'build nono-shell-broker' step above should produce this artifact - investigate cargo output."
                    throw "MSI validator pre-check failed: broker artifact not found at $brokerPath"
                }
                & (Join-Path $PWD "scripts\validate-windows-msi-contract.ps1") `
                    -BinaryPath (Join-Path $PWD "target\debug\nono.exe") `
                    -BrokerPath $brokerPath
            }
        }
        "smoke" {
            Invoke-TestList -LogFile "windows-smoke.log" -Tests $smokeTests
        }
        "integration" {
            Invoke-TestList -LogFile "windows-integration.log" -Tests $integrationTests
        }
        "security" {
            $wfpFilters = @(
                "windows_run_block_net_blocks_probe_connection",
                "windows_run_block_net_cleans_up_promoted_wfp_filters_after_exit"
            )
            $nonWfpTests = $securityTests | Where-Object { $_.Filter -notin $wfpFilters }
            $wfpTests = $securityTests | Where-Object { $_.Filter -in $wfpFilters }

            Invoke-TestList -LogFile "windows-security.log" -Tests $nonWfpTests

            if ($env:NONO_CI_HAS_WFP -eq 'true') {
                Invoke-TestList -LogFile "windows-security.log" -Tests $wfpTests
            } else {
                $msg = "SKIPPED: WFP tests require elevated runner (NONO_CI_HAS_WFP not set)"
                $msg | Tee-Object -FilePath (Join-Path $LogDir "windows-security.log") -Append
            }
        }
        "regression" {
            Invoke-TestList -LogFile "windows-regression.log" -Tests $regressionTests
        }
    }
}
