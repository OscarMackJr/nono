# Phase 101: Verify-Gate Hardening + Azure Profile Confirmation - Research

**Researched:** 2026-07-02
**Domain:** Windows Authenticode verification (signtool + .NET X509Chain) and Azure Trusted Signing certificate-profile administration
**Confidence:** HIGH (signtool/PowerShell/.NET APIs, official docs) / MEDIUM (GitHub-hosted runner signtool path, community-corroborated) / user-confirmed pending (actual PublicTrust vs PublicTrustTest state of this account — SIGN-01 checkpoint)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**signtool /pa role vs Get-AuthenticodeSignature (fail-closed contract)**
- **D-01:** The helper runs **BOTH** `Get-AuthenticodeSignature` **AND** `signtool verify /pa /v`
  on every strict verify — a `Valid` verdict requires **both** to pass (AND gate). Two
  independent chain-build engines must agree. This is *additive corroboration*: the existing
  `Status -ne 'Valid'` fail-closed condition is unchanged; signtool is a second must-pass
  check layered on top, never a loosened/alternative pass path. Diff review must confirm the
  original condition is byte-for-byte preserved (SIGN-02 success criterion 3).

**CRL/OCSP revocation-check transient behavior**
- **D-02:** On a **classified transient** (chain-build / revocation endpoint unreachable —
  the documented AOC/EOC CA rotation risk), the helper does **bounded retry-with-backoff**
  (sensible default ~3 attempts with increasing backoff — planner/researcher to set exact
  counts/timeouts), then **fail-closed** if still unresolved. Retries only ever *re-attempt*
  a verify; they never pass a non-`Valid` signature. A transient must never be silently
  swallowed — after exhausted retries it fails closed with the transient classification in
  the diagnostic.

**Helper interface / three-caller strictness (incl. .sys WHQL carve-out)**
- **D-03:** One Verb-Noun function with an **explicit mode parameter**, e.g.
  `Assert-TrustedSignature -Path <file> -Mode Strict|Informational`.
  - `Strict` → full fail-closed AND-gate verify (loose `.exe` assets + MSI `.exe` payloads).
  - `Informational` → logs status **without gating**, preserving the existing
    `nono-wfp-driver.sys` WHQL/cross-sign carve-out (that driver returns `UnknownError` under
    Authenticode on CI and is a *separate signing regime* — gating it would break the release).
  - Mode is passed **explicitly by every call site**. **No extension-based auto-classification**
    — hiding the strict/informational (security-relevant) decision behind a filename check is a
    footgun per CLAUDE.md path-handling guidance.

**Failure diagnostics depth (root cause "documented, not guessed")**
- **D-04:** On **any non-`Valid`** result, the helper dumps a **full chain introspection** to
  CI logs: every cert chain element (Subject / Issuer / Thumbprint), each element's
  chain-status flags, the verbatim `signtool /v` output, and which CRL/OCSP URLs were probed
  plus their reachability. Noise lands **only on the failure path** (happy path stays quiet).
  This must make the `PublicTrustTest`-issuer vs missing-intermediate vs revocation-transient
  distinction settleable from the CI log alone — directly serving SIGN-01's documented-not-guessed
  mandate.

**SIGN-01 operator flow + phase sequencing**
- **D-05:** **Ship hardening first; operator profile-fix is a checkpoint.** The fork
  autonomously builds and lands the helper (SIGN-02), wires it into all three verify sites,
  and adds the diagnostics — independent of the Azure ops step. The operator's
  `az trustedsigning certificate-profile show` confirm/fix (SIGN-01) is a documented
  **checkpoint that must pass before SIGN-03 smoke can go green.** The fork does not block at
  phase start waiting on the Azure step; instead the hardened verify + diagnostics are what
  *let* the operator settle the root cause fast when they run the check.

### Claude's Discretion
- Exact retry count / backoff timings for D-02 (a small, documented default is fine).
- Precise PowerShell function name/signature and how dot-sourcing is wired at each of the
  three call sites, as long as D-01/D-03 semantics hold and no fail-closed condition is
  weakened.
- Whether to also add a small scripted `az` confirmation/record helper (an option the
  operator did not select but did not forbid) — optional nice-to-have, not required; the
  runbook step is sufficient for SIGN-01.

### Deferred Ideas (OUT OF SCOPE)
- Retiring the POC signing path + deleting `trusted-signing-smoke.yml` + fixing the stale
  `docs/cli/development/windows-signing-guide.mdx` → **Phase 107 (Close-Out) / CLOSE-01**,
  gated on clean-host UAT PASS. Not this phase.
- Scripted `az` profile-confirmation helper (`scripts/azure/confirm-profile-type.ps1`) —
  optional; operator selected the runbook checkpoint. Can be added later if the manual step
  proves error-prone.
- Azure VM IaC + the `trusted-signed-assertion` / `broker-spawn-on-clean-host` gates that
  *reuse* this helper → **Phase 103**.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SIGN-01 | Confirm Azure Trusted Signing certificate profile is type `PublicTrust` (not `PublicTrustTest`); document finding (profile type + issuer chain), fix if needed | §4 Azure Trusted Signing profile types (`az trustedsigning certificate-profile show` output shape, accepted `--profile-type` values, issuer-naming differentiator); §2/§D-04 chain-dump gives the operator the log evidence to make this call fast |
| SIGN-02 | Extract shared `scripts/verify-authenticode.ps1` helper (dot-sourced by 3 call sites), add `signtool verify /pa /v` deep check + chain build/repair + CRL/OCSP transient handling + distinct chain-build-failure vs untrusted-root classification, without loosening `Status -ne 'Valid'` | §1 signtool verify semantics (exit codes, `/pa` policy meaning, output parsing); §2 X509Chain introspection (ChainElements, X509ChainStatusFlags); §3 transient classification + bounded retry pattern; §5 dot-sourcing structure; Code Examples section gives ready-to-adapt PowerShell |
| SIGN-03 | Smoke workflow green on `windows-latest`: throwaway exe signs + verifies `Valid`, issuer chains to public `Microsoft ID Verified CS EOC/AOC CA NN` root | §4 issuer-naming differentiator (`…Enterprise ID Verified Policy AOC CA…` = PublicTrustTest tell vs `Microsoft ID Verified CS EOC/AOC CA NN` = PublicTrust); §Environment Availability confirms signtool.exe is pre-installed on `windows-latest` |
</phase_requirements>

## Summary

This phase is almost entirely **in-box Windows/.NET tooling** — no new external packages are
installed. `signtool.exe` ships with the Windows SDK and is pre-installed on GitHub's
`windows-latest` hosted runner image [MEDIUM: community-corroborated, not an official
GitHub table]; `Get-AuthenticodeSignature` is a built-in PowerShell cmdlet
(`Microsoft.PowerShell.Security` module); `System.Security.Cryptography.X509Certificates`
(`X509Chain`, `X509ChainPolicy`, `X509ChainStatusFlags`) is part of the .NET base class
library available to any `pwsh` step with no additional install. `az trustedsigning
certificate-profile show` requires the `trustedsigning` Azure CLI extension, which
auto-installs on first use — this runs on the **operator's machine**, not in CI.

The technical crux is that `Get-AuthenticodeSignature`'s `Status` property is a **coarse
7-value enum** (`Valid`, `UnknownError`, `NotSigned`, `HashMismatch`, `NotTrusted`,
`NotSupportedFileFormat`, `Incompatible`) [VERIFIED: learn.microsoft.com]. Critically,
`UnknownError` ("the file has an invalid signature") is a **different, more generic** value
than `NotTrusted` ("signed by a publisher not trusted on the system") — the observed
`Status: UnknownError` on the smoke run is *not* the specific "untrusted root" status,
which is independent evidence supporting the "chain couldn't be built" (missing intermediate
/ revocation-check failure) hypothesis over "genuine untrusted root." `signtool verify /pa`
uses a completely separate chain-building code path (WinVerifyTrust with the Default
Authentication Verification Policy, vs GAS's internal chain build) — running both and
requiring both to report success is real corroboration from two independently-implemented
verifiers, not redundant noise. `X509Chain.Build()` is the tool for turning a bare
`UnknownError`/failure into an itemized, per-element diagnosis: each `X509ChainElement` has
its own `ChainElementStatus` array of `X509ChainStatus{Status, StatusInformation}` pairs,
and `X509ChainStatusFlags` has dedicated bits for `PartialChain` (chain couldn't reach the
root), `RevocationStatusUnknown`/`OfflineRevocation` (CRL/OCSP unreachable — the transient
case), and `UntrustedRoot` (the genuine failure) — these three are structurally
distinguishable from each other via bitwise flag inspection, which is exactly what D-04
needs.

**Primary recommendation:** Build `scripts/verify-authenticode.ps1` as a single
`Assert-TrustedSignature -Path <file> -Mode Strict|Informational` function with **no
GitHub-Actions-specific dependencies** (plain params, plain `Write-Host`/`Write-Error`,
returns/throws — never reads `$env:GITHUB_*`), dot-sourced identically by all three call
sites. Internally: (1) run `Get-AuthenticodeSignature`; (2) locate `signtool.exe` via a
`Get-ChildItem`-based Windows-SDK-path probe and run `signtool verify /pa /v`, capturing
stdout/stderr/exit code via `Start-Process -PassThru -RedirectStandardOutput/-RedirectStandardError -Wait`
(pipeline `&` output-capture is unreliable for reliably distinguishing exit code from
warnings — see Pitfall 3); (3) if `Mode -eq 'Strict'`, AND-gate on both being success —
`GAS.Status -eq 'Valid'` AND `signtool` exit code `0`; (4) if the result is anything but a
clean pass, build an `X509Chain` from the signer cert, classify the failure via
`X509ChainStatusFlags` bitwise tests, and if the classification is transient
(`RevocationStatusUnknown`/`OfflineRevocation`/`PartialChain` with no `UntrustedRoot` bit
set) retry the whole verify up to 3 times with linear backoff (2s/4s/8s) before failing
closed with the classification embedded in the error; (5) dump the full chain-element table
+ verbatim signtool output only on the failure path.

## Architectural Responsibility Map

This phase has no browser/frontend/API/database tiers — it is CI-pipeline tooling. The
table below maps each capability to the tier that structurally owns it in this repo's
existing architecture (CI workflow YAML vs. a shared PowerShell script library vs. the
operator's Azure control-plane session), which is the closest analog to "tier ownership"
for a build/release phase.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Authenticode dual-engine verify (GAS + signtool AND-gate) | Shared PowerShell helper (`scripts/verify-authenticode.ps1`) | — | Must be callable identically from 3 workflow sites + Phase 103's non-CI gate; logic does not belong duplicated in YAML `run:` blocks (that's the exact problem SIGN-02 fixes) |
| Chain-build failure classification + retry (D-02/D-04) | Shared PowerShell helper | — | Pure function of a `X509Certificate2`/file path; no CI-environment dependency, so it can't live in workflow YAML if Phase 103 must reuse it outside CI |
| Call-site wiring (`Mode Strict\|Informational`, which files, which step) | GitHub Actions workflow YAML (`release.yml` × 2, `trusted-signing-smoke.yml` × 1) | — | Deciding *which* files are strict vs informational (the `.sys` WHQL carve-out) is call-site policy, explicitly NOT inferred by the helper (D-03 no-auto-classify) |
| Azure Trusted Signing profile-type confirmation/fix (SIGN-01) | Operator / Azure control plane (`az trustedsigning certificate-profile show`, portal) | CI (consumes `TRUSTED_SIGNING_PROFILE` var as an opaque string) | Certificate-profile type is an Azure resource property outside any code the repo controls; CI only ever references the profile name by variable, never inspects its type |
| CI-clean-log evidence for the operator to act on (D-04 dump) | Shared PowerShell helper (emits) | GitHub Actions workflow (surfaces via CI logs) | The helper produces the diagnostic text; the CI log is merely the transport that lets the operator read it without re-running locally |

## Standard Stack

### Core

| Tool/API | Version | Purpose | Why Standard |
|----------|---------|---------|---------------|
| `signtool.exe` (Windows SDK) | Ships with Windows SDK; GitHub `windows-latest` image has a recent SDK preinstalled (path varies, e.g. `10.0.22621.0`) [MEDIUM: community-corroborated] | Deep Authenticode chain verification via `/pa /v` | The canonical Microsoft tool for Authenticode verify; unlike `Get-AuthenticodeSignature`, defaults can be forced to the Default Authentication Verification Policy (not the driver policy) [VERIFIED: learn.microsoft.com/windows/win32/seccrypto/signtool] |
| `Get-AuthenticodeSignature` (`Microsoft.PowerShell.Security`) | In-box with any Windows PowerShell / pwsh | First-pass, already-wired verify (existing 3 call sites) | Already the repo's existing fail-closed check; kept as one half of the AND-gate per D-01, never removed |
| `System.Security.Cryptography.X509Certificates.X509Chain` (.NET BCL) | In-box (.NET 8/9 runtime bundled with PowerShell 7) | Full chain enumeration + `X509ChainStatusFlags` classification for D-04 diagnostics and D-02 transient detection | The only supported, non-hand-rolled way to enumerate cert-chain elements and per-element status flags [VERIFIED: learn.microsoft.com/dotnet/api/…x509chain, …x509chainstatusflags] |
| `az trustedsigning certificate-profile show` (Azure CLI `trustedsigning` extension) | Extension in Preview, auto-installs on Azure CLI ≥ 2.57.0 [VERIFIED: learn.microsoft.com/cli/azure/trustedsigning] | SIGN-01 operator confirmation of `profileType` | The only supported way to read the deployed profile's type outside the portal UI |

### Supporting

| Tool/API | Version | Purpose | When to Use |
|----------|---------|---------|-------------|
| `X509Certificate2.Extensions` (AIA OID `1.3.6.1.5.5.7.1.1`, CDP OID `2.5.29.31`) | .NET BCL | Read CRL Distribution Point / Authority Information Access URLs from a chain element's raw extension for the D-04 "which CRL/OCSP URLs were probed" requirement | There is no first-class typed `.AiaUrls`/`.CdpUrls` property on `X509Certificate2` — extract via `X509Extension.Format(true)`-parsed text or the documented `CryptGetObjectUrl` P/Invoke pattern; see Code Examples |
| `Invoke-WebRequest` / raw TCP probe | pwsh built-in | Confirm CRL/OCSP endpoint reachability (part of D-04's "reachability" dump) | A lightweight HEAD/GET against each extracted CDP/AIA URL with a short timeout — informational only, does not gate |
| `vswhere.exe` (bundled on GitHub Windows images) | Any | Alternate signtool-location strategy if the direct SDK-path probe fails | Fallback only — `vswhere` locates Visual Studio installs, not the SDK directly, so prefer the direct `Windows Kits\10\bin` probe first |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `signtool verify /pa /v` | `Add-SignTool` community GitHub Action (`kamaranl/add-signtool-action`) to PATH-inject signtool | Adds a third-party Action dependency for a one-line `Get-ChildItem` probe the repo can inline; rejected — no new external dependency needed for something this small |
| Manual `X509Chain` bit-flag classification | `certutil -verify` output text-scraping | `certutil` output format is locale-dependent and less structured than `X509ChainStatusFlags`; `X509Chain` is the typed, stable API |
| Bounded retry inside the helper | GitHub Actions step-level `retry` via a marketplace action | Step-level retry would re-run the *entire* `run:` block (including unrelated setup), and can't distinguish "transient chain" vs "genuine fail" — retry logic must be inside the verify function per D-02, not at the workflow-step level |

**Installation:** None required — every Core/Supporting tool listed is either bundled with
the Windows SDK (`signtool.exe`), part of the .NET BCL / PowerShell 7 (`X509Chain`,
`Get-AuthenticodeSignature`), or an Azure CLI extension the operator runs locally
(auto-installs on first `az trustedsigning …` invocation). No `npm install` / `pip install`
/ `cargo add` is needed for this phase.

**Version verification:** N/A (no versioned package dependency added to `Cargo.toml` or any
lockfile this phase). `signtool.exe`'s exact build number is runner-image-dependent and is
resolved dynamically at runtime by the helper (see Code Examples), not pinned.

## Package Legitimacy Audit

**Not applicable to this phase.** SIGN-02 adds a PowerShell script (`scripts/verify-authenticode.ps1`)
that calls only in-box Windows/.NET/PowerShell APIs (`signtool.exe`, `Get-AuthenticodeSignature`,
`System.Security.Cryptography.X509Certificates`). No `npm`/`pip`/`cargo` package is added to
any manifest or lockfile, and no third-party GitHub Action is introduced (the existing
`azure/login@v2` and `azure/trusted-signing-action@v0` actions are unchanged carryovers from
prior phases, out of this phase's scope). The Package Legitimacy Gate (slopcheck, registry
verification) is therefore skipped — there is nothing to audit.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────┐
                    │      GitHub Actions: release.yml / smoke     │
                    │                                               │
   Sign step  ──────┤  azure/trusted-signing-action@v0 signs file  │
                    │                    │                          │
                    │                    ▼                          │
   Verify step ─────┤  . scripts/verify-authenticode.ps1           │
                    │  Assert-TrustedSignature -Path $f -Mode X    │
                    └───────────────┬───────────────────────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────────────┐
                    │  Get-AuthenticodeSignature $f          │──► Status: Valid|UnknownError|...
                    └───────────────┬─────────────────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────────────┐
                    │  signtool verify /pa /v $f              │──► exit 0|1|2 + stdout/stderr
                    └───────────────┬─────────────────────────┘
                                    │
                        Mode==Strict? AND-gate both results
                                    │
                    ┌───────────────┴───────────────────────┐
                    │ both pass?                              │
              YES → │ quiet success, return                   │
              NO  → │ classify via X509Chain.Build(cert)       │
                    │   → PartialChain/RevocationStatusUnknown │
                    │     /OfflineRevocation, no UntrustedRoot │
                    │        = TRANSIENT → retry (≤3, backoff) │
                    │   → UntrustedRoot / retries exhausted     │
                    │        = FAIL-CLOSED, dump full chain +   │
                    │          signtool /v output + CDP/AIA URL │
                    │          reachability, throw/exit 1       │
                    └───────────────────────────────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────────────┐
                    │  Operator reads CI log → runs           │
                    │  az trustedsigning certificate-profile  │
                    │  show (SIGN-01 checkpoint) if the        │
                    │  classification points at profile type   │
                    └───────────────────────────────────────┘
```

### Recommended Project Structure

```
scripts/
├── verify-authenticode.ps1     # NEW — dot-sourceable, no GHA env-var dependency
│   ├── function Assert-TrustedSignature { param([string]$Path, [ValidateSet('Strict','Informational')][string]$Mode) ... }
│   ├── function Invoke-SignToolVerify { ... }        # locates signtool, runs /pa /v, captures exit+output
│   ├── function Get-ChainClassification { ... }      # X509Chain.Build + flag bitwise classification
│   ├── function Get-CertificateRevocationUrls { ... } # AIA/CDP extraction + reachability probe
│   └── function Write-ChainDiagnostic { ... }         # D-04 full-chain dump (failure path only)
.github/workflows/
├── release.yml                 # Site 1 (~L259) + Site 2 (~L281) now `. ..\scripts\verify-authenticode.ps1`
└── trusted-signing-smoke.yml   # Smoke site (~L62) now `. ..\scripts\verify-authenticode.ps1`
```

### Pattern 1: Dot-sourceable, CI-agnostic helper (D-03/D-05, Phase-103-reuse requirement)

**What:** A single `.ps1` file with only `function` definitions and **no top-level executable
statements** that reference `$env:GITHUB_*`, `$env:CI`, or any GitHub-Actions-only context.
Every parameter (path, mode) is passed explicitly by the caller.

**When to use:** Any script three-plus call sites need identically, especially when a
future phase (103) needs to reuse the same logic outside CI.

**Example:**
```powershell
# scripts/verify-authenticode.ps1
# Source pattern: PowerShell dot-sourcing (built-in language feature, not a package)
# https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_scripts

function Assert-TrustedSignature {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][ValidateSet('Strict', 'Informational')][string]$Mode
    )
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Assert-TrustedSignature: file not found: $Path"
    }
    $gas = Get-AuthenticodeSignature -LiteralPath $Path
    $signtoolResult = Invoke-SignToolVerify -Path $Path

    $gasOk      = $gas.Status -eq 'Valid'
    $signtoolOk = $signtoolResult.ExitCode -eq 0

    if ($Mode -eq 'Informational') {
        Write-Host "Authenticode (informational): $Path GAS=$($gas.Status) signtool_exit=$($signtoolResult.ExitCode)"
        return [pscustomobject]@{ Path = $Path; GasStatus = $gas.Status; SignToolExitCode = $signtoolResult.ExitCode; Passed = $null }
    }

    # Mode -eq 'Strict': AND-gate — D-01. The ORIGINAL condition is preserved verbatim below
    # (do not touch this line during refactor — SIGN-02 success criterion 3 diff-checks it).
    if ($gas.Status -ne 'Valid') {
        Write-Error "Authenticode verification failed for $Path with status $($gas.Status)."
        Invoke-ChainDiagnostic -Path $Path -Gas $gas -SignToolResult $signtoolResult
        throw "Assert-TrustedSignature: GAS status not Valid ($($gas.Status)) for $Path"
    }
    if (-not $signtoolOk) {
        Write-Error "signtool verify /pa /v failed for $Path (exit $($signtoolResult.ExitCode))."
        Invoke-ChainDiagnostic -Path $Path -Gas $gas -SignToolResult $signtoolResult
        throw "Assert-TrustedSignature: signtool verify failed (exit $($signtoolResult.ExitCode)) for $Path"
    }
    Write-Host "Authenticode OK (dual-engine): $Path"
    return [pscustomobject]@{ Path = $Path; GasStatus = $gas.Status; SignToolExitCode = 0; Passed = $true }
}
```

Dot-sourced identically at every call site:
```powershell
# In release.yml / trusted-signing-smoke.yml, inside a `shell: pwsh` step:
. "$PWD\scripts\verify-authenticode.ps1"
Assert-TrustedSignature -Path $binary -Mode Strict
```

### Pattern 2: signtool discovery on windows-latest

**What:** `signtool.exe` is not on `PATH` by default on GitHub's `windows-latest` runner; it
must be located under the Windows SDK's `bin` tree and the highest-versioned copy selected.

**When to use:** Any CI step that shells out to `signtool` rather than relying solely on
`Get-AuthenticodeSignature`.

**Example:**
```powershell
# Source: community pattern corroborated across multiple guides (MEDIUM confidence —
# no single canonical Microsoft/GitHub doc pins this exact path, but it is stable across
# recent windows-latest images: C:\Program Files (x86)\Windows Kits\10\bin\<ver>\<arch>\signtool.exe)
function Find-SignTool {
    $candidates = Get-ChildItem -Path "C:\Program Files (x86)\Windows Kits\10\bin" `
        -Recurse -Filter "signtool.exe" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' }
    if (-not $candidates) {
        throw "Find-SignTool: no signtool.exe found under Windows Kits 10 bin tree."
    }
    # Highest SDK version wins (path segment sorts lexically for same-width version strings)
    return ($candidates | Sort-Object FullName -Descending | Select-Object -First 1).FullName
}
```

### Pattern 3: Reliable exit-code + stdout/stderr capture for signtool in pwsh

**What:** `& signtool.exe verify /pa /v $Path` inside `pwsh` can have its exit code masked by
`$PSNativeCommandUseErrorActionPreference` / `$ErrorActionPreference = 'Stop'` (a documented
repo convention — see `scripts/verify-dark.ps1:17`, which explicitly disables this for native
commands). Use `Start-Process -PassThru -Wait` with redirected streams for a value that
cannot be silently swallowed.

**Example:**
```powershell
function Invoke-SignToolVerify {
    param([Parameter(Mandatory)][string]$Path)
    $signtool = Find-SignTool
    $stdout = New-TemporaryFile
    $stderr = New-TemporaryFile
    try {
        $proc = Start-Process -FilePath $signtool `
            -ArgumentList @('verify', '/pa', '/v', "`"$Path`"") `
            -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        [pscustomobject]@{
            ExitCode = $proc.ExitCode                     # 0=success, 1=fail, 2=warnings (official)
            StdOut   = Get-Content -Raw $stdout
            StdErr   = Get-Content -Raw $stderr
        }
    } finally {
        Remove-Item $stdout, $stderr -Force -ErrorAction SilentlyContinue
    }
}
```

### Pattern 4: Chain classification for D-02/D-04 (transient vs genuine)

**What:** Build an `X509Chain` from the file's signer certificate, force an online revocation
check, and bitwise-test `ChainStatus` flags to decide TRANSIENT vs FAIL-CLOSED vs re-verify.

**Example:**
```powershell
function Get-ChainClassification {
    param([Parameter(Mandatory)][System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate)

    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    $chain.ChainPolicy.RevocationMode  = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
    $chain.ChainPolicy.RevocationFlag  = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain
    $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(15)
    $built = $chain.Build($Certificate)

    $allFlags = 0
    foreach ($status in $chain.ChainStatus) { $allFlags = $allFlags -bor $status.Status.value__ }

    $flags = [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]$allFlags
    $untrusted = ($flags -band [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::UntrustedRoot) -ne 0
    $transientBits = [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags](
        [int][System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::RevocationStatusUnknown -bor
        [int][System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::OfflineRevocation -bor
        [int][System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::PartialChain
    )
    $isTransientOnly = (($flags -band $transientBits) -ne 0) -and (-not $untrusted) -and $built -eq $false

    [pscustomobject]@{
        Built            = $built
        Flags            = $flags
        Chain            = $chain
        IsUntrustedRoot  = $untrusted
        IsTransient      = $isTransientOnly
    }
}
```

### Pattern 5: Bounded retry-with-backoff (D-02) — always ends fail-closed

```powershell
function Invoke-VerifyWithTransientRetry {
    param([Parameter(Mandatory)][scriptblock]$VerifyBlock, [int]$MaxAttempts = 3)
    $backoffSeconds = @(2, 4, 8)
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        $result = & $VerifyBlock
        if ($result.Passed) { return $result }
        if (-not $result.IsTransient) { return $result }   # not transient -> fail-closed immediately, no more retries
        if ($attempt -lt $MaxAttempts) {
            Write-Host "Transient chain/revocation failure (attempt $attempt/$MaxAttempts) — retrying in $($backoffSeconds[$attempt-1])s"
            Start-Sleep -Seconds $backoffSeconds[$attempt-1]
        }
    }
    # Exhausted retries on a transient classification — STILL fail closed (D-02: never pass a non-Valid signature)
    return $result
}
```

### Anti-Patterns to Avoid
- **Setting `X509RevocationMode.NoCheck` to make transient failures go away:** disables
  revocation checking entirely — this is a security regression masquerading as a fix, and is
  exactly the kind of "loosened pass path" D-01/anti-goals forbid. Retry is the only sanctioned
  response to a transient; never turn off the check.
- **Classifying strict-vs-informational by file extension** (`.sys` → informational,
  everything else → strict): explicitly forbidden by D-03. Every call site must pass `-Mode`
  explicitly.
- **Reading `$env:GITHUB_*` inside the helper function body:** breaks the Phase 103 reuse
  requirement (the helper must be CI-agnostic); pass everything as parameters instead.
- **Relying on PowerShell `&` pipe output to get signtool's exit code:** `$LASTEXITCODE` after
  `&` can be clobbered by subsequent commands or masked by `$ErrorActionPreference`; use
  `Start-Process -PassThru` (Pattern 3) for a value that can't be silently lost.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parsing an Authenticode PKCS#7 blob to find the signer cert / issuer chain | A manual ASN.1/DER parser | `Get-AuthenticodeSignature`'s `.SignerCertificate` property, or `[X509Certificate2]::CreateFromSignedFile` / `X509Chain` | Windows already exposes a fully-parsed `X509Certificate2` object; hand-parsing PKCS#7 is exactly the kind of deceptively-complex, security-critical parsing this project's CLAUDE.md path/crypto guidance warns against |
| Determining "is this cert chain trustworthy" | Custom trust-anchor comparison logic | `X509Chain.Build()` + `X509ChainStatusFlags` | Building/validating a cert chain against the OS trust store, including revocation, name constraints, and policy constraints, is a solved problem in the BCL; a hand-rolled version would silently miss edge cases (name constraints, weak signature detection, CTL checks) that `X509Chain` already covers |
| Extracting CRL/AIA URLs from certificate extensions | Manual byte-offset scanning of extension OIDs | `X509Extension.Format(true)` (human-readable) or the documented `CryptGetObjectUrl` P/Invoke pattern (sysadmins.lv) | AIA/CDP extensions are ASN.1-encoded `GeneralNames`; Windows CryptoAPI already has a purpose-built extraction function — reinventing this is unnecessary parsing risk for a diagnostics-only feature |
| Retry/backoff scheduling | A generic retry library/module | The small inline loop in Pattern 5 | 3 attempts with fixed linear backoff is simple enough that pulling in a dependency (and the associated legitimacy-audit overhead) is not justified for one call site |

**Key insight:** Every primitive this phase needs — chain building, status flags, signature
parsing — already exists in the .NET BCL that ships with the PowerShell runtime GitHub's
Windows runners use. The entire implementation surface is glue code around
`Get-AuthenticodeSignature`, `signtool.exe`, and `X509Chain`; there is no algorithmic
component that should be reinvented.

## Common Pitfalls

### Pitfall 1: signtool defaults to the driver-signing policy, not the Authenticode policy
**What goes wrong:** Running bare `signtool verify MyFile.exe` (no `/pa`) applies the Windows
**Driver** Verification Policy, which can spuriously fail on a legitimately Authenticode-signed
user-mode `.exe`/`.msi`.
**Why it happens:** `signtool verify`'s default policy predates broad Authenticode-only usage;
`/pa` selects the Default Authentication Verification Policy explicitly.
**How to avoid:** Always pass `/pa` (this phase's D-01 requirement already mandates it).
**Warning signs:** `signtool verify` failing on a file that `Get-AuthenticodeSignature`
reports `Valid` for — check whether `/pa` is present in the invocation before treating this
as a real corroboration failure. [VERIFIED: learn.microsoft.com/windows/win32/seccrypto/using-signtool-to-verify-a-file-signature]

### Pitfall 2: Confusing `UnknownError` with `NotTrusted`
**What goes wrong:** Treating the observed `Status: UnknownError` as proof of "genuine
untrusted root," which would misdirect the SIGN-01 investigation toward the wrong fix.
**Why it happens:** Both sound like trust failures, but `SignatureStatus` explicitly
separates them: `UnknownError` (value 1) = "the file has an invalid signature" (a broad
catch-all, which is what chain-build failures surface as), `NotTrusted` (value 4) =
"signed by a publisher not trusted on the system" (the actually-specific untrusted-root
case). [VERIFIED: learn.microsoft.com/dotnet/api/system.management.automation.signaturestatus]
**How to avoid:** D-04's full chain dump exists precisely to replace this ambiguous single
enum value with the itemized `X509ChainStatusFlags` breakdown.
**Warning signs:** Any diagnostic write-up that says "UnknownError means untrusted" without
citing the actual `X509ChainStatusFlags` bits found.

### Pitfall 3: `$LASTEXITCODE` after `&` is not a reliable signtool exit-code source in this repo's pwsh convention
**What goes wrong:** Using `& signtool verify ...; $LASTEXITCODE` can read a stale or
clobbered value if `$ErrorActionPreference = 'Stop'` intervenes, or if any cmdlet runs
between the native call and the read.
**Why it happens:** This repo already has a documented workaround for exactly this class of
issue (`$PSNativeCommandUseErrorActionPreference = $false` in `scripts/verify-dark.ps1:17`,
citing `scripts/windows-test-harness.ps1:7-10`) — native-command output/exit-code handling in
pwsh is a known repo-wide footgun, not new to this phase.
**How to avoid:** Use `Start-Process -PassThru -Wait` and read `.ExitCode` directly (Pattern 3)
— it does not depend on `$LASTEXITCODE` timing at all.
**Warning signs:** An exit code of `0` when the printed signtool text clearly shows a failure
message, or vice versa.

### Pitfall 4: Retrying a genuinely-untrusted signature
**What goes wrong:** A naive "retry on any failure" implementation would re-attempt verify on
a real `UntrustedRoot`/bad-signature case, wasting CI time and potentially masking the real
failure behind transient-looking log noise.
**Why it happens:** Conflating "any non-Valid result" with "transient" instead of gating
retry strictly on the classification (`PartialChain`/`RevocationStatusUnknown`/`OfflineRevocation`
with `UntrustedRoot` explicitly absent).
**How to avoid:** Pattern 5's retry loop checks `IsTransient` (which is `false` whenever
`UntrustedRoot` is set) before ever looping — an untrusted-root result returns immediately
without retrying.
**Warning signs:** CI logs showing 3 retry attempts for a case that should have failed on
attempt 1 (i.e., a case with `UntrustedRoot` set).

### Pitfall 5: signtool location varies by runner image / SDK version
**What goes wrong:** Hardcoding a specific SDK version path (e.g.
`...\bin\10.0.22621.0\x64\signtool.exe`) breaks silently whenever GitHub bumps the
`windows-latest` image's bundled SDK.
**Why it happens:** GitHub periodically updates the Windows SDK version on hosted runner
images without notice tied to this repo.
**How to avoid:** Discover the highest-versioned `signtool.exe` dynamically at runtime
(Pattern 2) rather than pinning a path string.
**Warning signs:** A previously-green verify step suddenly failing with "signtool.exe not
found" after an unrelated GitHub Actions runner-image update.

### Pitfall 6: Azure CLI `trustedsigning` extension is Preview — output shape may shift
**What goes wrong:** Automating parsing of `az trustedsigning certificate-profile show`
JSON output (e.g., for an optional scripted confirmation helper) could break if Microsoft
changes field names while the extension is in Preview.
**Why it happens:** The command group is explicitly marked "in preview and under
development" [VERIFIED: learn.microsoft.com/cli/azure/trustedsigning/certificate-profile].
**How to avoid:** SIGN-01 is operator-run and human-read per D-05 — this is a soft
risk, not a hard blocker, but any optional scripted helper (Claude's Discretion item) should
tolerate an unexpected/missing `profileType` key gracefully rather than crashing.
**Warning signs:** `az trustedsigning certificate-profile show --query profileType` returning
`null` unexpectedly after an Azure CLI extension update.

## Code Examples

### Locating signtool + running the deep verify (combines Patterns 2 & 3)
```powershell
# Source: Microsoft SignTool docs (exit codes, /pa semantics) +
# community-corroborated Windows-Kits path convention
# https://learn.microsoft.com/windows/win32/seccrypto/signtool
$signtool = Find-SignTool
$result   = Invoke-SignToolVerify -Path $binary
if ($result.ExitCode -ne 0) {
    Write-Host "signtool verify /pa /v FAILED (exit $($result.ExitCode)):"
    Write-Host $result.StdOut
    Write-Host $result.StdErr
}
```

### Enumerating every chain element for the D-04 dump
```powershell
# Source: https://learn.microsoft.com/dotnet/api/system.security.cryptography.x509certificates.x509chainpolicy
#         (official example adapted to Write-Host / CI log output)
foreach ($element in $chain.ChainElements) {
    Write-Host "Subject:   $($element.Certificate.Subject)"
    Write-Host "Issuer:    $($element.Certificate.Issuer)"
    Write-Host "Thumbprint:$($element.Certificate.Thumbprint)"
    foreach ($status in $element.ChainElementStatus) {
        Write-Host "  ChainStatus: $($status.Status) — $($status.StatusInformation)"
    }
}
```

### Extracting a raw distinguishing name check for PublicTrust vs PublicTrustTest (SIGN-01/SIGN-03 evidence)
```powershell
# Not an API call — a simple string check on the already-parsed Issuer. This is the exact
# check the D-04 dump should surface for the operator (per CONTEXT.md §Specific Ideas):
$issuer = $gas.SignerCertificate.Issuer
if ($issuer -match 'Enterprise ID Verified Policy') {
    Write-Warning "Issuer '$issuer' matches the PublicTrustTest naming pattern — profile is likely NOT PublicTrust."
} elseif ($issuer -match 'Microsoft ID Verified CS.*(EOC|AOC) CA') {
    Write-Host "Issuer '$issuer' matches the expected public PublicTrust naming."
} else {
    Write-Warning "Issuer '$issuer' matches neither known pattern — do not guess, escalate to SIGN-01 operator check."
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Single `Get-AuthenticodeSignature` check duplicated in 3 workflow `run:` blocks, `Status -ne 'Valid'` fail-closed | Shared `scripts/verify-authenticode.ps1` helper, dual-engine AND-gate (GAS + signtool `/pa /v`), classified retry, full-chain diagnostics on failure | This phase (101) | Same fail-closed guarantee, but now corroborated by a second independent chain-builder and self-diagnosing on failure instead of a bare status string |
| Self-signed/base64 PFX signing (`WINDOWS_SIGNING_CERT`) | Azure Trusted Signing (keyless, OIDC, `azure/trusted-signing-action@v0`) | Phase ~53 (D-02 remediation, quick-task 260603-i31) | Already in place before this phase; this phase only hardens the *verify* side of an already-migrated signing side |

**Deprecated/outdated:**
- Nothing in this phase deprecates prior code; SIGN-02 explicitly requires the original
  `Status -ne 'Valid'` condition to be preserved byte-for-byte inside the new helper (D-01
  success criterion 3), not replaced.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `signtool.exe` is pre-installed on GitHub's `windows-latest` hosted runner image under `C:\Program Files (x86)\Windows Kits\10\bin\<ver>\x64\signtool.exe` | Standard Stack, Pattern 2, Environment Availability | If absent/moved, `Find-SignTool` throws and the whole verify step fails closed (safe failure mode, but blocks CI until a fallback install step is added) — low risk since fail-closed is the intended behavior anyway |
| A2 | The PublicTrustTest issuer naming reliably contains the substring `Enterprise ID Verified Policy` and PublicTrust reliably contains `Microsoft ID Verified CS` + `EOC`/`AOC CA` | Code Examples, §4 | If Microsoft changes CA naming conventions, the pattern-match diagnostic gives a false negative/positive — mitigated because SIGN-01 is operator-verified against the actual `az trustedsigning certificate-profile show` output, not solely the string match |
| A3 | A 3-attempt / 2s-4s-8s linear backoff is a "sensible default" per D-02's discretion clause | Pattern 5 | If Azure's actual CRL/OCSP transient window is longer, retries could still exhaust before the endpoint recovers — acceptable per D-02 since exhausting retries still fails closed (no security risk, only a possible false-negative CI run needing a manual re-run) |
| A4 | `X509RevocationMode.Online` + a 15s `UrlRetrievalTimeout` is an appropriate chain-build policy for this diagnostic (vs. the OS default used implicitly by `Get-AuthenticodeSignature`) | Pattern 4 | If set too short, legitimate but slow CRL fetches could be misclassified as `RevocationStatusUnknown`/transient more often than necessary — safe direction (triggers retry, not a false pass) |

**If this table is empty:** N/A — see rows above; all four assumptions err toward the
fail-closed direction if wrong (worst case is spurious retries or a diagnostic false
positive, never a loosened pass).

## Open Questions (RESOLVED during planning)

> All three questions are operationally resolved by the Phase 101 plans (no execution risk):
> **Q1** → Plan 101-03 SIGN-01 operator checkpoint (`autonomous: false`, gates SIGN-03 smoke).
> **Q2** → Plan 101-01 reuses the live `Find-Signtool` dynamic highest-version probe from
> `scripts/sign-windows-artifacts.ps1:22-58` (per PATTERNS.md), making the SDK-path question moot.
> **Q3** → resolved to implementer discretion (default `Invoke-WebRequest -Method Head`); it is a
> diagnostics-only, failure-path probe with no gating effect.

1. **Is the account's current certificate profile actually `PublicTrustTest`?**
   - What we know: the observed issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 01`
     matches the naming pattern this research (and the go-live cookbook) associates with
     `PublicTrustTest`, not `PublicTrust`.
   - What's unclear: this has not been confirmed by actually running
     `az trustedsigning certificate-profile show` against the live account — that is
     precisely SIGN-01's job and is explicitly an operator checkpoint (D-05), not something
     this research or the autonomous build can resolve.
   - Recommendation: the planner should treat SIGN-01 as a `checkpoint:human-verify` task
     gating SIGN-03 (smoke-green), per D-05 — ship the SIGN-02 hardening independently first.

2. **Will GitHub bump the `windows-latest` image's bundled Windows SDK during this phase's
   execution window in a way that changes the signtool path shape?**
   - What we know: the path pattern (`Windows Kits\10\bin\<ver>\<arch>\signtool.exe`) has been
     stable across recent SDK versions.
   - What's unclear: no official GitHub `runner-images` changelog was consulted in this
     research pass to confirm the *current* pinned SDK version on `windows-latest` at the time
     of implementation.
   - Recommendation: implement `Find-SignTool` as a dynamic highest-version probe (Pattern 2),
     never a hardcoded path — this makes the question moot regardless of the answer.

3. **Should the CDP/AIA reachability probe (D-04) use a raw TCP/HTTP HEAD request or shell out
   to `certutil -URL` / `certutil -verify`?**
   - What we know: `certutil` has built-in URL-fetch diagnostic modes; a raw
     `Invoke-WebRequest -Method Head -TimeoutSec N` is simpler and avoids another external
     process.
   - What's unclear: which produces more actionable log output for distinguishing "DNS/TLS
     unreachable" from "endpoint reachable but returned an error" — not resource-constrained
     to pin down definitively in research; both are viable, non-hand-rolled options.
   - Recommendation: leave this as planner/implementer discretion (it's diagnostics-only,
     does not affect the fail-closed contract either way) — default to `Invoke-WebRequest`
     for simplicity unless the implementer finds `certutil` output more useful during
     manual testing.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `signtool.exe` (Windows SDK) | SIGN-02 (`Invoke-SignToolVerify`) | ✓ (on `windows-latest` GitHub-hosted runner) [MEDIUM confidence — community-corroborated, not fetched from an official GitHub runner-images manifest in this research pass] | Runner-dependent (e.g. `10.0.22621.0`), resolved dynamically | None with no fallback — if genuinely absent, the helper should fail closed (throw), which is the correct behavior for a security verify gate, not a soft-skip |
| `pwsh` (PowerShell 7) | All three call sites, already in use | ✓ (`shell: pwsh` already used in all 3 existing steps) | Whatever `windows-latest` bundles (PowerShell 7.x) | — |
| `.NET BCL` (`X509Chain` et al.) | D-02/D-04 chain classification | ✓ (bundled with the PowerShell 7 runtime) | — | — |
| `az` CLI + `trustedsigning` extension | SIGN-01 (operator-run, NOT CI) | Operator-machine dependent, not probed in this research session (out of scope — happens on the operator's own `az`-logged-in machine per the cookbook) | Extension auto-installs on Azure CLI ≥ 2.57.0 | The go-live cookbook's portal-UI fallback ("if the verb differs in your CLI version, do it in the portal") already documents a manual fallback |

**Missing dependencies with no fallback:**
- None identified as blocking — `signtool.exe` absence would correctly fail closed rather
  than block planning; this is the intended security posture, not a gap to route around.

**Missing dependencies with fallback:**
- `az trustedsigning` CLI extension → portal UI (already documented in the go-live cookbook,
  §2 "if the verb differs in your CLI version, do it in the portal").

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None detected — this repo has no Pester (`*.Tests.ps1`) scaffolding; existing PowerShell validation is ad-hoc, imperative scripts under `scripts/tests/` (e.g. `test_windows_attach.ps1`) and `scripts/gates/` (machine-verdict style, `scripts/verify-dark.ps1` dispatcher) |
| Config file | none — see Wave 0 |
| Quick run command | `pwsh -File scripts/tests/test_verify_authenticode.ps1` (proposed, mirroring the existing `scripts/tests/test_windows_*.ps1` naming convention) |
| Full suite command | The 3 live call sites themselves are the true integration test — `gh workflow run trusted-signing-smoke.yml` (SIGN-03's actual acceptance gate) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|---------------------|--------------|
| SIGN-02 | `Assert-TrustedSignature -Mode Strict` on a known-good signed file returns `Passed=$true`, quiet log | unit (ad-hoc pwsh script) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case knownGood` | ❌ Wave 0 |
| SIGN-02 | Original `Status -ne 'Valid'` fail-closed condition byte-for-byte unchanged (diff check) | static/manual code review | `git diff` review of the extracted lines vs. `release.yml`/`trusted-signing-smoke.yml` pre-refactor | N/A — code-review gate, not a runtime test |
| SIGN-02 | `Mode Informational` on the `.sys` WHQL driver logs status without throwing/gating | unit (ad-hoc pwsh script) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case whqlInformational` | ❌ Wave 0 |
| SIGN-02 | Transient classification (simulate `RevocationStatusUnknown`) triggers retry, still fails closed after 3 attempts if unresolved | unit (ad-hoc pwsh script, mocked chain result) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case transientRetry` | ❌ Wave 0 |
| SIGN-01 | `az trustedsigning certificate-profile show` returns `profileType == PublicTrust` | manual (operator checkpoint) | `az trustedsigning certificate-profile show -g <rg> --account-name <acct> -n <profile> --query profileType -o tsv` | N/A — operator-run, not automatable in CI |
| SIGN-03 | `trusted-signing-smoke.yml` run is green; embedded signature `Valid`, issuer matches `Microsoft ID Verified CS.*(EOC\|AOC) CA` | smoke (live CI) | `gh workflow run trusted-signing-smoke.yml` then `gh run watch` | ✓ (workflow already exists, gated on SIGN-01+SIGN-02 landing first) |

### Sampling Rate
- **Per task commit:** Run the ad-hoc `scripts/tests/test_verify_authenticode.ps1` cases
  locally on Windows (or via a scratch `windows-latest` `workflow_dispatch` run) before
  merging any helper change.
- **Per wave merge:** Full `trusted-signing-smoke.yml` dispatch (SIGN-03's actual gate).
- **Phase gate:** `trusted-signing-smoke.yml` green + SIGN-01 operator checkpoint confirmed
  before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `scripts/tests/test_verify_authenticode.ps1` — new ad-hoc test script exercising the
      4 unit-level cases above (knownGood, whqlInformational, transientRetry, untrustedRootNoRetry);
      no Pester needed, follow the existing `scripts/tests/test_windows_attach.ps1` imperative
      convention (`$ErrorActionPreference = "Stop"`, `Write-Host`/`Write-Error` assertions,
      explicit exit code).
- [ ] No shared fixtures needed — a locally self-signed throwaway test certificate (or
      reuse of `scripts/sign-poc-local.ps1`'s existing POC cert) is sufficient for the
      knownGood/whqlInformational cases; the transient/untrusted-root cases should be
      tested via **mocked** `Get-ChainClassification` inputs (constructed
      `X509ChainStatusFlags` values), not live network conditions, per the standard
      "don't depend on real revocation infrastructure being in a bad state" testing
      principle.
- [ ] Framework install: none — this repo's convention is deliberately Pester-free for
      PowerShell; do not introduce Pester in this phase unless a separate decision is made
      (out of scope here; note only).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | No | N/A — no authentication surface changes in this phase |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A |
| V5 Input Validation | Yes | File paths passed to `Assert-TrustedSignature -Path` must be validated with `Test-Path -LiteralPath` (not `-Path`, to avoid wildcard-expansion path-injection) before use — consistent with CLAUDE.md's "path component comparison, not string operations" guidance |
| V6 Cryptography | Yes | Certificate-chain validation MUST use `X509Chain`/`signtool` (BCL/OS-provided cryptographic primitives), never hand-rolled signature/chain parsing — see Don't Hand-Roll table |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|------------------------|
| Loosening the fail-closed verify to accept `UnknownError` (silently trusting an unverifiable signature) | Spoofing / Tampering | D-01/D-02 explicitly forbid this; the AND-gate + retry design only ever *adds* corroboration or *re-attempts* — the code review gate (SIGN-02 success criterion 3) diff-checks that the original condition is untouched |
| Revocation check disabled to "fix" flaky CI (`X509RevocationMode.NoCheck`) | Tampering (a revoked cert would silently pass) | Never set `NoCheck`; use bounded retry (Pattern 5) instead — a transient revocation-check failure retries the *check*, it never bypasses it |
| Path-injection via an untrusted `-Path` argument (e.g., a crafted filename with wildcard chars reaching `Get-ChildItem`/`Test-Path`) | Tampering | Use `-LiteralPath` everywhere in the helper, not `-Path`, and validate the path exists before any signtool/GAS call — all current call sites pass compile-time-known, repo-controlled paths (build artifacts), so external input is not actually reachable here, but the helper should still default to the safer `-LiteralPath` form for defense in depth |
| Diagnostic log injection (a malicious CN/Subject string containing log-forging control characters, printed verbatim in the D-04 chain dump) | Tampering (log integrity) | Low risk — the printed Subject/Issuer strings originate from the Trusted Signing service's own issued certificates, not attacker-controlled input in this CI-signing context; no additional sanitization needed beyond what `Write-Host`/GitHub Actions log rendering already does |

## Sources

### Primary (HIGH confidence)
- [SignTool - Win32 apps | Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool) — verify command options (`/pa`, `/v`, `/kp`, `/r`), exit codes (0/1/2)
- [Use SignTool to Verify a File Signature - Win32 apps | Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/seccrypto/using-signtool-to-verify-a-file-signature) — `/pa` vs default driver policy explanation
- [SignatureStatus Enum (System.Management.Automation) | Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/api/system.management.automation.signaturestatus) — all 7 `Get-AuthenticodeSignature` status values with descriptions
- [Get-AuthenticodeSignature (Microsoft.PowerShell.Security) | Microsoft Learn](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature) — cmdlet syntax, `SignerCertificate` output shape
- [X509ChainStatusFlags Enum | Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/api/system.security.cryptography.x509certificates.x509chainstatusflags) — full flag list incl. `PartialChain`, `RevocationStatusUnknown`, `OfflineRevocation`, `UntrustedRoot`
- [X509ChainPolicy Class | Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/api/system.security.cryptography.x509certificates.x509chainpolicy) — `RevocationMode`, `RevocationFlag`, `UrlRetrievalTimeout`, official `X509Chain.Build()` + `ChainElements` example
- [az trustedsigning certificate-profile | Microsoft Learn](https://learn.microsoft.com/en-us/cli/azure/trustedsigning/certificate-profile) — full `--profile-type` accepted values (`PrivateTrust, PrivateTrustCIPolicy, PublicTrust, PublicTrustTest, VBSEnclave`), `show`/`create` command shapes; command group flagged Preview
- [Artifact Signing FAQ | Microsoft Learn](https://learn.microsoft.com/en-us/azure/artifact-signing/faq) — Public Trust / Public Trust Test / VBS enclave profile constraints (no custom OU/CN), common error codes

### Secondary (MEDIUM confidence)
- Multiple community guides (Federico Terzi's blog, dev.to "Missing Guide to Windows Code Signing," `kamaranl/add-signtool-action` README) corroborating `signtool.exe`'s pre-installed path on GitHub's `windows-latest` hosted runner under `C:\Program Files (x86)\Windows Kits\10\bin\<ver>\<arch>\signtool.exe`
- sysadmins.lv "How to programmatically extract CDP, AIA and OCSP URLs from a digital certificate" — the `CryptGetObjectUrl` P/Invoke pattern for AIA/CDP extraction (no first-class typed .NET API exists)

### Tertiary (LOW confidence)
- None retained — all WebSearch findings used above were cross-checked against an official Microsoft Learn page before inclusion.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every tool (signtool, GAS, X509Chain, az trustedsigning) is confirmed via official Microsoft Learn documentation fetched in this session.
- Architecture: HIGH for the PowerShell patterns (dot-sourcing, Start-Process capture, X509Chain classification) — all built on documented, stable APIs; MEDIUM for the exact signtool runner-image path (community-corroborated, not an official GitHub runner-images citation).
- Pitfalls: HIGH — sourced directly from official docs (SignatureStatus enum semantics, `/pa` policy behavior) plus this repo's own prior art (`scripts/verify-dark.ps1`'s documented native-command exit-code footgun).
- SIGN-01 root cause: MEDIUM — the PublicTrustTest-vs-PublicTrust hypothesis is well-supported by naming-convention evidence and the accepted `--profile-type` enum, but genuinely **unconfirmed** until the operator runs `az trustedsigning certificate-profile show` (this is correctly modeled as a D-05 checkpoint, not resolved by research).

**Research date:** 2026-07-02
**Valid until:** 30 days (stable Microsoft-owned APIs; the one fast-moving element is the Azure CLI `trustedsigning` extension, still in Preview, and the `windows-latest` runner image's bundled SDK version — both already designed around via dynamic discovery / operator-run confirmation rather than pinned assumptions)
