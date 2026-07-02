# Phase 101 Plan 01 (SIGN-02): imperative, framework-free test harness for the shared
# scripts/verify-authenticode.ps1 helper (Assert-TrustedSignature + supporting functions).
#
# Convention: mirrors scripts/tests/test_windows_attach.ps1 ($ErrorActionPreference = "Stop",
# Write-Host/Write-Error per-case assertions, explicit exit code) — no Pester, per this repo's
# deliberately Pester-free PowerShell testing convention (101-RESEARCH.md Validation Architecture).
#
# Usage:
#   pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All
#   pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case knownGood
#
# TASK 1 (RED) NOTE: at this commit, scripts/verify-authenticode.ps1 does not yet exist.
# The dot-source below is expected to fail with a path-not-found error BEFORE any case
# runs — proving this harness is syntactically complete and genuinely blocked only on the
# missing implementation, not a bug in the test itself. Task 2 (GREEN) makes this pass.

param(
    [ValidateSet('knownGood', 'whqlInformational', 'transientRetry', 'untrustedRootNoRetry', 'All')]
    [string]$Case = 'All'
)

$ErrorActionPreference = "Stop"

# Dot-source the helper under test. During the RED commit this file does not exist yet,
# so this line throws a path-not-found error immediately — before any case function is
# even defined, let alone invoked (RED state, per <done> criteria).
try {
    . (Join-Path $PSScriptRoot "..\verify-authenticode.ps1")
}
catch {
    [Console]::Error.WriteLine("FAIL: could not dot-source verify-authenticode.ps1: $_")
    exit 1
}

Write-Host "--- test_verify_authenticode: Assert-TrustedSignature cases ---"

function Assert-True {
    param(
        [Parameter(Mandatory)][bool]$Condition,
        [Parameter(Mandatory)][string]$Message
    )
    if (-not $Condition) {
        throw $Message
    }
}

# ---------------------------------------------------------------------------
# Case: knownGood
# ---------------------------------------------------------------------------
# Signs a throwaway exe with a locally self-signed code-signing cert, trusts that cert
# for chain-building purposes, and asserts Assert-TrustedSignature -Mode Strict returns
# Passed=$true with a quiet (no "ChainStatus:") happy path.
#
# ENVIRONMENT-CONSTRAINT DEVIATION (documented in 101-01-SUMMARY.md): the dev host running
# this test is NOT elevated. Cert:\LocalMachine\Root (the pattern in
# scripts/sign-windows-artifacts.ps1's Add-TrustForVerify) requires administrator
# privileges and would fail Access Denied here. This fixture instead imports the
# throwaway cert into Cert:\CurrentUser\Root — Get-AuthenticodeSignature and
# X509Chain.Build() both consult the CurrentUser Root store when building the trust
# chain for the current-user context, so this produces Status=Valid exactly as
# LocalMachine\Root would for the machine-wide context. This is a test-fixture-only,
# security-neutral change; it does not alter Assert-TrustedSignature's production
# behavior at all — only where the throwaway test cert is trusted.
function Test-KnownGood {
    $csc = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
    if (-not (Test-Path -LiteralPath $csc)) {
        throw "Test-KnownGood: csc.exe not found at $csc — cannot stage a signable fixture."
    }

    $workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("nono-verify-authenticode-good-" + [System.Guid]::NewGuid().ToString("N").Substring(0, 8))
    New-Item -ItemType Directory -Path $workDir -Force | Out-Null
    $exePath = Join-Path $workDir "knowngood.exe"
    $csPath = Join-Path $workDir "prog.cs"
    $thumbprint = $null

    try {
        'class P { static void Main() { System.Console.WriteLine("nono verify-authenticode knownGood fixture"); } }' |
            Set-Content -LiteralPath $csPath

        & $csc /nologo /out:$exePath $csPath | Out-Null
        if (-not (Test-Path -LiteralPath $exePath)) {
            throw "Test-KnownGood: csc.exe did not produce $exePath"
        }

        $cert = New-SelfSignedCertificate -Type CodeSigningCert `
            -Subject "CN=nono verify-authenticode test fixture" `
            -CertStoreLocation Cert:\CurrentUser\My `
            -NotAfter (Get-Date).AddDays(1)
        $thumbprint = $cert.Thumbprint

        $signtoolPath = Find-Signtool
        & $signtoolPath sign /fd sha256 /sha1 $thumbprint $exePath | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Test-KnownGood: signtool sign failed (exit $LASTEXITCODE)"
        }

        # DEVIATION: Cert:\CurrentUser\Root, not Cert:\LocalMachine\Root — see header note.
        # Use the raw X509Store API (not Export-Certificate + Import-Certificate) to add
        # the cert: the Import-Certificate cmdlet invokes the CryptUI "Security Warning"
        # trust-confirmation dialog for CurrentUser\Root, which HANGS a non-interactive
        # session (observed empirically) — exactly the class of footgun this repo already
        # documents for LocalMachine\Root in Add-TrustForVerify/Remove-TrustForVerify
        # (scripts/sign-windows-artifacts.ps1). X509Store.Add() writes the store directly,
        # bypassing the trust-UI layer entirely — no dialog, no hang.
        $rootStoreAdd = [System.Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
        $rootStoreAdd.Open('ReadWrite')
        $rootStoreAdd.Add($cert)
        $rootStoreAdd.Close()

        $infoOutput = $null
        $result = Assert-TrustedSignature -Path $exePath -Mode Strict -InformationVariable infoOutput

        Assert-True ($result.Passed -eq $true) "knownGood: Passed should be `$true, got $($result.Passed) (GasStatus=$($result.GasStatus))"

        $infoText = ($infoOutput | ForEach-Object { $_.MessageData.Message }) -join "`n"
        Assert-True ($infoText -notmatch 'ChainStatus:') "knownGood: happy path must stay quiet — found a ChainStatus: diagnostic line on a passing verify"
    }
    finally {
        if ($thumbprint) {
            # Remove-Item on Cert:\CurrentUser\Root raises an interactive UI consent
            # prompt ("The operation is on user root store and UI is not allowed" when
            # non-interactive) — mirrors the documented LocalMachine\Root footgun in
            # scripts/sign-windows-artifacts.ps1's Remove-TrustForVerify. Use the
            # X509Store API instead, which removes without a prompt.
            try {
                $rootStore = [System.Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
                $rootStore.Open('ReadWrite')
                foreach ($c in @($rootStore.Certificates | Where-Object { $_.Thumbprint -eq $thumbprint })) {
                    $rootStore.Remove($c)
                }
                $rootStore.Close()
            }
            catch {
                Write-Host "Warning: could not remove test fixture cert from CurrentUser\Root: $($_.Exception.Message)"
            }
            Remove-Item -LiteralPath "Cert:\CurrentUser\My\$thumbprint" -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# ---------------------------------------------------------------------------
# Case: whqlInformational
# ---------------------------------------------------------------------------
# An intentionally-unsigned file, verified in -Mode Informational: must never throw,
# must return Passed=$null, and must not pay the retry/backoff cost (single attempt).
function Test-WhqlInformational {
    $csc = "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
    if (-not (Test-Path -LiteralPath $csc)) {
        throw "Test-WhqlInformational: csc.exe not found at $csc — cannot stage a fixture."
    }

    $workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("nono-verify-authenticode-whql-" + [System.Guid]::NewGuid().ToString("N").Substring(0, 8))
    New-Item -ItemType Directory -Path $workDir -Force | Out-Null
    $exePath = Join-Path $workDir "unsigned.exe"
    $csPath = Join-Path $workDir "prog.cs"

    try {
        'class P { static void Main() { System.Console.WriteLine("nono verify-authenticode whqlInformational fixture"); } }' |
            Set-Content -LiteralPath $csPath

        & $csc /nologo /out:$exePath $csPath | Out-Null
        if (-not (Test-Path -LiteralPath $exePath)) {
            throw "Test-WhqlInformational: csc.exe did not produce $exePath"
        }

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $threw = $false
        $result = $null
        try {
            $result = Assert-TrustedSignature -Path $exePath -Mode Informational
        }
        catch {
            $threw = $true
        }
        $sw.Stop()

        Assert-True (-not $threw) "whqlInformational: Assert-TrustedSignature -Mode Informational must never throw, regardless of signature status"
        Assert-True ($null -eq $result.Passed) "whqlInformational: Passed must be `$null in Informational mode, got $($result.Passed)"
        Assert-True ($sw.Elapsed.TotalSeconds -lt 5) "whqlInformational: Informational mode must not enter the retry/backoff loop (took $($sw.Elapsed.TotalSeconds)s — the @(2,4,8) backoff alone exceeds 5s)"
    }
    finally {
        Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# ---------------------------------------------------------------------------
# Case: transientRetry (+ folded direct Get-FlagClassification sub-assertions)
# ---------------------------------------------------------------------------
# No real certs/files here — a mocked scriptblock simulates a classified transient on
# every attempt; asserts the retry loop exhausts MaxAttempts and still fails closed.
function Test-TransientRetry {
    # Direct unit tests of the pure classifier (folded in per PLAN.md discretion).
    $flagsType = [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]

    $c1 = Get-FlagClassification -Flags ($flagsType::RevocationStatusUnknown) -Built $false
    Assert-True ($c1.IsTransient -eq $true) "Get-FlagClassification: RevocationStatusUnknown should classify IsTransient=`$true"
    Assert-True ($c1.IsUntrustedRoot -eq $false) "Get-FlagClassification: RevocationStatusUnknown should classify IsUntrustedRoot=`$false"

    $c2 = Get-FlagClassification -Flags ($flagsType::UntrustedRoot) -Built $false
    Assert-True ($c2.IsUntrustedRoot -eq $true) "Get-FlagClassification: UntrustedRoot should classify IsUntrustedRoot=`$true"
    Assert-True ($c2.IsTransient -eq $false) "Get-FlagClassification: UntrustedRoot should classify IsTransient=`$false"

    $combined = [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags](
        [int]$flagsType::UntrustedRoot -bor [int]$flagsType::RevocationStatusUnknown
    )
    $c3 = Get-FlagClassification -Flags $combined -Built $false
    Assert-True ($c3.IsUntrustedRoot -eq $true) "Get-FlagClassification: UntrustedRoot must dominate a combined transient-looking flag set (Pitfall 4)"

    # Mock-counter retry test.
    $script:transientCallCount = 0
    $mock = {
        $script:transientCallCount++
        [pscustomobject]@{ Passed = $false; IsTransient = $true }
    }
    $result = Invoke-VerifyWithTransientRetry -VerifyBlock $mock -MaxAttempts 3
    Assert-True ($script:transientCallCount -eq 3) "transientRetry: expected exactly 3 attempts (retries exhausted), got $script:transientCallCount"
    Assert-True ($result.Passed -eq $false) "transientRetry: an exhausted-retry transient must still fail closed (Passed=`$false), got $($result.Passed)"
}

# ---------------------------------------------------------------------------
# Case: untrustedRootNoRetry
# ---------------------------------------------------------------------------
# A mocked scriptblock simulates a genuine untrusted root on every attempt; asserts no
# retry is ever attempted (distinct from the transient case).
function Test-UntrustedRootNoRetry {
    $script:untrustedCallCount = 0
    $mock = {
        $script:untrustedCallCount++
        [pscustomobject]@{ Passed = $false; IsTransient = $false }
    }
    $result = Invoke-VerifyWithTransientRetry -VerifyBlock $mock -MaxAttempts 3
    Assert-True ($script:untrustedCallCount -eq 1) "untrustedRootNoRetry: expected exactly 1 attempt (no retry for a genuine untrusted root), got $script:untrustedCallCount"
    Assert-True ($result.Passed -eq $false) "untrustedRootNoRetry: must fail closed (Passed=`$false), got $($result.Passed)"
}

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------
function Invoke-NamedCase {
    param(
        [Parameter(Mandatory)][string]$CaseName,
        [Parameter(Mandatory)][scriptblock]$Block
    )
    try {
        & $Block
        Write-Host "PASS: $CaseName"
    }
    catch {
        [Console]::Error.WriteLine("FAIL: ${CaseName}: $_")
        exit 1
    }
}

switch ($Case) {
    'knownGood' { Invoke-NamedCase -CaseName 'knownGood' -Block { Test-KnownGood } }
    'whqlInformational' { Invoke-NamedCase -CaseName 'whqlInformational' -Block { Test-WhqlInformational } }
    'transientRetry' { Invoke-NamedCase -CaseName 'transientRetry' -Block { Test-TransientRetry } }
    'untrustedRootNoRetry' { Invoke-NamedCase -CaseName 'untrustedRootNoRetry' -Block { Test-UntrustedRootNoRetry } }
    'All' {
        Invoke-NamedCase -CaseName 'knownGood' -Block { Test-KnownGood }
        Invoke-NamedCase -CaseName 'whqlInformational' -Block { Test-WhqlInformational }
        Invoke-NamedCase -CaseName 'transientRetry' -Block { Test-TransientRetry }
        Invoke-NamedCase -CaseName 'untrustedRootNoRetry' -Block { Test-UntrustedRootNoRetry }
    }
}

Write-Host "test_verify_authenticode: all cases PASSED"
exit 0
