# scripts/gates/trusted-signed-assertion.ps1
#
# Phase 103 - trusted-signed-assertion gate (CHOST-02)
#
# CONTRACT (mirrors scripts/gates/clean-host-install.ps1, the reference contract for this
# harness): this gate exports exactly two functions dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object - it MUST NOT call exit and MUST NOT call
# Persist-Verdict. Only the runner owns exit-code mapping (PASS=0 / FAIL=2 /
# SKIP_HOST_UNAVAILABLE=3 / harness-internal=4) and the persist-before-emit (WR-04).
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE - exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
#
# WHAT THIS PROVES (satisfies CHOST-02): a staged release artifact carries a genuinely
# trusted Authenticode signature. This gate dot-sources scripts/verify-authenticode.ps1 and
# reuses its already-hardened dual-engine (Get-AuthenticodeSignature + `signtool verify /pa
# /v`) Assert-TrustedSignature helper rather than re-implementing any part of that logic.
#
# CRITICAL - issuer-substring matching was evaluated and REJECTED as a pass/fail gating
# mechanism. Per live evidence in
# .planning/phases/101-verify-gate-hardening-azure-profile-confirmation/101-SIGN03-SMOKE-VERDICT.md
# Finding B, a genuinely-valid PublicTrust signature can chain through an issuer such as
# "Microsoft Enterprise ID Verified Policy AOC CA 02" (or CA 01) - an issuer that superficially
# resembles a non-production/test issuer string. Gating on any issuer substring (e.g.
# "Microsoft ID Verified CS") would therefore REJECT a legitimate signature. The ONLY
# pass/fail condition in this gate is `Get-AuthenticodeSignature.Status -eq 'Valid'`, asserted
# via `Assert-TrustedSignature -Mode Strict`. The issuer string is captured into `detail` as
# informational-only (via a second, direct Get-AuthenticodeSignature call - the
# Assert-TrustedSignature success object does not expose `.Issuer`) and is NEVER used as an
# `if` branch condition. A future maintainer must not resurrect issuer-string matching as a
# gate condition.
#
# WR-01: No stray pipeline output from Invoke-Gate. All results are assigned to named
#        variables. No bare object is written to the pipeline.
# WR-04: Only the runner persists the verdict file and owns the emit-before-persist order.

# ---------------------------------------------------------------------------
# Gate configuration (mirrors release.yml's artifact_staging / $binary convention,
# .github/workflows/release.yml lines 264-266)
# ---------------------------------------------------------------------------

# Operator stages a signed release build (Phase 104) or clean-host UAT payload (Phase 106)
# at this path before running the gate. Override by setting $StagedArtifactPath before
# dot-sourcing. The runner dot-sources this file without parameters, so the default is
# load-bearing. This literal path does not exist on the dev host today - that is the
# intended, honest SKIP condition (CHOST-02).
param(
    [string]$StagedArtifactPath = (Join-Path (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)) 'artifact_staging\nono.exe')
)
$script:StagedArtifactPath = $StagedArtifactPath

# Reuse the shared, already-hardened dual-engine helper - never reimplement any part of its
# Get-AuthenticodeSignature + signtool verify /pa /v logic.
. (Join-Path (Split-Path -Parent $PSScriptRoot) 'verify-authenticode.ps1')

# ---------------------------------------------------------------------------
# Gate contract
# ---------------------------------------------------------------------------

function Test-Precondition {
    # Return $null when all preconditions met; return a reason string -> SKIP_HOST_UNAVAILABLE.
    # NOTE: Test-Precondition MUST NOT throw; a throw here causes a harness-internal error (exit 4).

    # Signature verification is not a privileged operation - no elevation check needed here
    # (unlike clean-host-install.ps1's machine-scope MSI install). The sole precondition is
    # that a staged artifact actually exists to verify.
    if (-not (Test-Path -LiteralPath $script:StagedArtifactPath)) {
        return "trusted-signed-assertion gate requires a staged signed artifact at $($script:StagedArtifactPath) — stage a release build (Phase 104) or clean-host UAT payload (Phase 106) before running this gate"
    }

    return $null  # All clear - Invoke-Gate will run.
}

function Invoke-Gate {
    # Trusted-signature proof (CHOST-02).
    # Returns exactly one verdict object (gate never calls exit or writes verdict files).

    # Native tools write progress to stderr; do not promote to terminating errors.
    $ErrorActionPreference = 'Continue'

    $stamp = { Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ' }

    try {
        # Assert-TrustedSignature -Mode Strict throws on any non-Valid GAS status or a
        # failed `signtool verify /pa /v` (verify-authenticode.ps1:341-350) - never Mode
        # Informational, which would discard the result instead of gating on it.
        $result = Assert-TrustedSignature -Path $script:StagedArtifactPath -Mode 'Strict'

        # The Assert-TrustedSignature success object does NOT expose `.Issuer` - a second,
        # independent Get-AuthenticodeSignature call is required to read it. This value is
        # informational-only and is NEVER used as a pass/fail branch condition (see the
        # CRITICAL header note above and 101-SIGN03-SMOKE-VERDICT.md Finding B).
        $secondGasCheck = Get-AuthenticodeSignature -LiteralPath $script:StagedArtifactPath
        $signerCert = $secondGasCheck.SignerCertificate
        $issuerInformational = ''
        if ($signerCert) {
            $issuerInformational = $signerCert.Issuer
        }

        return [ordered]@{
            gate      = 'trusted-signed-assertion'
            verdict   = 'PASS'
            reason    = "Authenticode Valid (dual-engine) for $($script:StagedArtifactPath)"
            detail    = [ordered]@{
                gasStatus            = $result.GasStatus
                signtoolExit         = $result.SignToolExitCode
                # Informational only - never a gate condition. See CRITICAL header note.
                issuerInformational  = $issuerInformational
            }
            timestamp = & $stamp
        }
    }
    catch {
        # Assert-TrustedSignature THROWS on non-Valid (verify-authenticode.ps1:341-350) —
        # this MUST be caught here and translated to a FAIL verdict. An uncaught throw
        # inside Invoke-Gate is classified HARNESS_ERROR/exit 4 by verify-dark.ps1's own
        # try/catch, which would misclassify a legitimate signature failure as a harness
        # bug, not a FAIL. Do NOT re-throw.
        return [ordered]@{
            gate      = 'trusted-signed-assertion'
            verdict   = 'FAIL'
            reason    = "Authenticode assertion failed: $_"
            detail    = [ordered]@{}
            timestamp = & $stamp
        }
    }
}
