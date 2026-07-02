# Phase 101: Verify-Gate Hardening + Azure Profile Confirmation - Pattern Map

**Mapped:** 2026-07-02
**Files analyzed:** 4 (2 new, 2 modified)
**Analogs found:** 4 / 4 (all strong; two files have multiple partial analogs worth combining)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `scripts/verify-authenticode.ps1` | utility (dot-sourceable pwsh function library) | transform (file path in -> classified verdict object out) | `scripts/sign-windows-artifacts.ps1` (signtool discovery + invoke/verify) + `scripts/verify-dark.ps1` (fail-closed classification/dispatch idiom, native-command footgun handling) | exact (signtool discovery), role-match (classification/dispatch structure) |
| `scripts/tests/test_verify_authenticode.ps1` | test | batch (sequential imperative cases, explicit exit code) | `scripts/tests/test_windows_attach.ps1` / `test_windows_detach.ps1` | exact |
| `.github/workflows/release.yml` (2 verify sites, ~L259-274, ~L281-321) | config / CI pipeline step | request-response (native-tool invoke -> exit-code gate) | itself (pre-refactor inline logic) — the two existing `run:` blocks | exact (this IS the logic being extracted) |
| `.github/workflows/trusted-signing-smoke.yml` (verify site, ~L62-73) | config / CI pipeline step | request-response | itself (pre-refactor inline logic); secondary analog `release.yml`'s Site 1 (near-duplicate) | exact |

## Pattern Assignments

### `scripts/verify-authenticode.ps1` (utility, transform)

**Primary analog:** `scripts/sign-windows-artifacts.ps1`
**Secondary analog:** `scripts/verify-dark.ps1` (fail-closed dispatch/classification idiom + documented native-command footgun)
**Tertiary analog:** `scripts/verify-trust-root-cached.ps1` (small standalone verify script, `$LASTEXITCODE` idiom)

**signtool discovery — copy near-verbatim** (`scripts/sign-windows-artifacts.ps1:22-58`):
```powershell
function Find-Signtool {
    # Prefer signtool.exe if it is already on PATH.
    $onPath = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($null -ne $onPath) {
        return $onPath.Source
    }

    # GitHub-hosted Windows runners ship the Windows SDK but do NOT put signtool.exe
    # on PATH — it lives under Windows Kits\10\bin\<sdk-version>\<arch>\signtool.exe.
    # Search the SDK install roots, prefer the x64 build and the highest SDK version.
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "${env:ProgramFiles}\Windows Kits\10\bin"
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }

    $candidates = foreach ($root in $roots) {
        Get-ChildItem -Path $root -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' }
    }
    # Fall back to any arch if no x64 build was found.
    if ($null -eq $candidates -or @($candidates).Count -eq 0) {
        $candidates = foreach ($root in $roots) {
            Get-ChildItem -Path $root -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue
        }
    }

    $chosen = @($candidates) | Sort-Object {
        # Directory layout: ...\bin\<version>\<arch>\signtool.exe -> version is the grandparent.
        $verName = $_.Directory.Parent.Name
        try { [version]$verName } catch { [version]"0.0.0.0" }
    } -Descending | Select-Object -First 1

    if ($null -eq $chosen) {
        throw "signtool.exe not found on PATH or under the Windows SDK (Windows Kits\10\bin). The Windows SDK must be installed on the runner."
    }
    return $chosen.FullName
}
```
This is a strictly better analog than RESEARCH.md's speculative `Find-SignTool` (Pattern 2) — it is live, already-exercised repo code (used by the signing side, `scripts/sign-windows-artifacts.ps1`, itself dot-sourced-equivalent via direct invocation from `release.yml`'s "Sign" steps historically). **Reuse this function verbatim** (rename to `Find-SignTool` or keep `Find-Signtool` — either is fine, just be consistent) rather than re-deriving the SDK-path probe from scratch.

**Existing signtool verify invocation for interface shape** (`scripts/sign-windows-artifacts.ps1:180-189`):
```powershell
# ... (Invoke-SigntoolVerify, header comment at line 183-184)
    # /pa - policy: Default Authenticode Verification Policy
    # /tw - warn if the signature is valid but has an expiring/expired timestamp
    #        rather than a silent success; D-12 requires timestamp-aware verification)
    & $SigntoolPath verify /pa /tw $ArtifactPath
    if ($LASTEXITCODE -ne 0) {
        throw "signtool verify failed for '$ArtifactPath' — signature is not valid Authenticode or is missing a timestamp (exit $LASTEXITCODE)."
    }
    Write-Host "Signature verified: $ArtifactPath"
```
Note: this existing call uses `& ... ; if ($LASTEXITCODE -ne 0)`, NOT `Start-Process -PassThru`. RESEARCH.md Pitfall 3 flags `$LASTEXITCODE`-after-`&` as unreliable when interleaved with other cmdlets — but this repo already runs it safely today because nothing runs between the `&` and the check. For the new helper's dual-engine AND-gate (D-01), prefer the more robust `Start-Process -PassThru -Wait` capture (see release.yml's own admin-extract step below) since the new helper interleaves a `Get-AuthenticodeSignature` call and diagnostic logic around the signtool invocation, which is exactly the interleaving scenario the pitfall warns about.

**Reliable native exit-code capture — copy this idiom, already live in this repo** (`.github/workflows/release.yml:293-297`, inside "Verify MSI payload signatures (Windows)"):
```powershell
$proc = Start-Process msiexec -ArgumentList @("/a", "`"$msi`"", "/qn", "TARGETDIR=`"$extractDir`"", "/l*v", "`"$log`"") -Wait -PassThru
if ($proc.ExitCode -ne 0) {
    Write-Error "Administrative extract failed for $msi (exit $($proc.ExitCode)). See $log."
    exit 1
}
```
This proves `Start-Process -PassThru -Wait` + `.ExitCode` is an established, already-shipping pattern in this exact workflow file (not just a research suggestion) — use the same shape for `Invoke-SignToolVerify`, adding `-RedirectStandardOutput`/`-RedirectStandardError` to temp files for the D-04 verbatim-output dump.

**Fail-closed dispatch / classification idiom to model the helper's internal structure on** (`scripts/verify-dark.ps1:1-17`):
```powershell
$ErrorActionPreference = "Stop"
# Cargo and other native tools write normal progress output to stderr.
# Keep that from being promoted into terminating PowerShell errors while we tee logs.
# (Mirrors scripts/windows-test-harness.ps1:7-10; gates that shell out may locally
# set $ErrorActionPreference = 'Continue' inside Invoke-Gate per PATTERNS.md reasoning.)
$PSNativeCommandUseErrorActionPreference = $false
```
And the harness-internal-error-vs-FAIL separation idiom (`scripts/verify-dark.ps1:82-92`, `234-249`) — not copied verbatim into the new helper (different domain), but the **discipline** it embodies — never conflate "could not determine" with either PASS or a specific FAIL classification — is exactly D-04's ask (transient chain-build failure vs genuine `UntrustedRoot` vs harness bug must stay three distinguishable outcomes, never collapsed).

**`$LASTEXITCODE` / `Write-Error` + `$ErrorActionPreference='Stop'` interaction footgun, already documented once in this repo** (`scripts/verify-trust-root-cached.ps1:14-17, 27-31, 63-67`):
```powershell
# F-03-05 mitigation: `$ErrorActionPreference = 'Stop'` does NOT trap
# native-command failures. After every `& nono ...` invocation we explicitly
# check `$LASTEXITCODE` and `throw` on non-zero — that's the only mechanism
# that propagates a failed `nono setup` to the script's exit code.
...
# Use [Console]::Error.WriteLine + explicit exit 2 to avoid Write-Error +
# $ErrorActionPreference='Stop' terminating with the generic ExitCode 1
# before the param-validation early-exit (exit 2) can fire.
[Console]::Error.WriteLine("ERROR: candidate path does not exist or is not a file: $Candidate")
exit 2
```
Apply the same `[Console]::Error.WriteLine` + explicit `throw`/`exit` discipline inside `Assert-TrustedSignature` rather than bare `Write-Error` when you need a guaranteed, non-swallowed failure signal — this repo has already hit and documented the `Write-Error` + `$ErrorActionPreference='Stop'` interaction bug twice (here and in `verify-dark.ps1`).

**No existing X509Chain / X509ChainStatusFlags code in this repo** — this is new territory; use RESEARCH.md Patterns 4-5 (`Get-ChainClassification`, `Invoke-VerifyWithTransientRetry`) directly, since research already grounded them in official .NET BCL docs and there is no closer in-repo analog to draw from.

**Interface skeleton to build around (Mode Strict|Informational AND-gate)** — RESEARCH.md's Pattern 1 `Assert-TrustedSignature` skeleton (context lines 279-313) is sound and should be used as the starting scaffold; graft the real `Find-Signtool` (above) and `Start-Process -PassThru` capture (above) into it in place of the research's placeholder `Invoke-SignToolVerify`.

---

### `scripts/tests/test_verify_authenticode.ps1` (test, batch)

**Analog:** `scripts/tests/test_windows_attach.ps1` (also `test_windows_detach.ps1`, identical convention)

**Header + preamble pattern** (`scripts/tests/test_windows_attach.ps1:1-8`):
```powershell
$ErrorActionPreference = "Stop"

$NonoBin = Join-Path $PSScriptRoot "..\..\target\debug\nono.exe"
if (-not (Test-Path $NonoBin)) {
    Write-Error "nono binary not found at $NonoBin. Please build the project first."
}

Write-Host "--- Task 2: Create Attach Integration Test ---"
```
Adapt: no `nono.exe` dependency needed for this test; instead dot-source the helper under test:
```powershell
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "..\verify-authenticode.ps1")
Write-Host "--- test_verify_authenticode: Assert-TrustedSignature cases ---"
```

**Assertion + cleanup convention** (`scripts/tests/test_windows_attach.ps1:26-37, 43-62`) — sequential `if (...) { Write-Error/Write-Warning } else { Write-Host "Verified..." }` blocks, no test framework, explicit `Write-Host "<Name> test script completed."` at the end, and a `finally`-style cleanup section (temp files removed via `Remove-Item -Force`). Mirror this exact shape for the 4 unit-level cases named in RESEARCH.md's Wave-0 Gaps (`knownGood`, `whqlInformational`, `transientRetry`, `untrustedRootNoRetry`): one case per block, `Write-Host` on pass, `Write-Error` (or `throw`, given this is a real assertion script rather than an integration smoke test) on failure, and an explicit exit code at the very end — following `scripts/verify-trust-root-cached.ps1`'s `exit 0` / `exit 1` / `exit 2` discipline (see excerpt above) rather than leaving the exit code to fall through implicitly.

**Mocking the transient/untrusted-root cases:** since there is no existing X509Chain-mocking precedent in this repo, construct a `[pscustomobject]` shaped like `Get-ChainClassification`'s return (`Built`, `Flags`, `IsUntrustedRoot`, `IsTransient`) directly in the test rather than driving a real network condition — this keeps the test hermetic, consistent with RESEARCH.md's Validation Architecture guidance ("mocked … inputs, not live network conditions").

---

### `.github/workflows/release.yml` — Site 1 (~L259-274) and Site 2 (~L281-321)

**Site 1 — current inline logic to extract, preserving the fail-closed condition byte-for-byte** (`release.yml:259-274`):
```yaml
      - name: Verify Authenticode signatures (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          $binary = Join-Path $PWD "target\\${{ matrix.target }}\\release\\${{ matrix.artifact }}"
          $broker = Join-Path $PWD "target\\${{ matrix.target }}\\release\\nono-shell-broker.exe"
          $machineMsi = Join-Path $PWD "artifact_staging\\nono-${{ env.RELEASE_TAG }}-x86_64-pc-windows-msvc-machine.msi"
          $userMsi = Join-Path $PWD "artifact_staging\\nono-${{ env.RELEASE_TAG }}-x86_64-pc-windows-msvc-user.msi"
          foreach ($artifact in @($binary, $broker, $machineMsi, $userMsi)) {
            $sig = Get-AuthenticodeSignature -FilePath $artifact
            if ($sig.Status -ne "Valid") {
              Write-Error "Authenticode verification failed for $artifact with status $($sig.Status)."
              exit 1
            }
            Write-Host "Authenticode OK: $artifact"
          }
```
**Refactor target:** dot-source the helper, then call `Assert-TrustedSignature -Path $artifact -Mode Strict` inside the same `foreach` loop, in place of the inline `Get-AuthenticodeSignature`/`if`/`Write-Error`/`exit 1` block. **The `Status -ne 'Valid'` condition must land unchanged inside the helper** (D-01 success criterion 3) — diff the extracted line against this excerpt at review time.

**Site 2 — `.sys` informational carve-out to preserve exactly** (`release.yml:298-320`):
```yaml
            # Verify only .exe payloads strictly — these are the binaries REQ-RLS-01 /
            # UAT-B care about (nono.exe, broker, wfp-service). The kernel driver
            # nono-wfp-driver.sys is the checked-in WHQL/cross-signed copy governed by a
            # SEPARATE signing regime; its cross-cert chain returns UnknownError under
            # Get-AuthenticodeSignature on the CI runner (it is not signed with the POC
            # Authenticode cert), so it is logged informationally rather than gated here.
            $exePayload = Get-ChildItem -LiteralPath $extractDir -Recurse -File | Where-Object { $_.Extension -eq '.exe' }
            ...
            foreach ($f in $exePayload) {
              $sig = Get-AuthenticodeSignature -FilePath $f.FullName
              if ($sig.Status -ne "Valid") {
                Write-Error "MSI payload signature invalid: $($f.FullName) status $($sig.Status) — the MSI shipped an unsigned binary (signing-order regression)."
                exit 1
              }
              Write-Host "MSI payload Authenticode OK: $($f.Name) [$name]"
            }
            foreach ($d in (Get-ChildItem -LiteralPath $extractDir -Recurse -File | Where-Object { $_.Extension -eq '.sys' })) {
              $dsig = Get-AuthenticodeSignature -FilePath $d.FullName
              Write-Host "MSI payload driver signature (informational, WHQL regime): $($d.Name) status=$($dsig.Status) [$name]"
            }
```
**Refactor target:** the `.exe` foreach becomes `Assert-TrustedSignature -Path $f.FullName -Mode Strict`; the `.sys` foreach becomes `Assert-TrustedSignature -Path $d.FullName -Mode Informational`. This is the file-extension-based grouping already present in the codebase — note per D-03 the **call site** (this YAML) is exactly where extension-based grouping is allowed to live (it decides which files get which literal `-Mode` argument); the helper itself must never re-derive strict/informational from `$Path`'s extension internally.

**How to dot-source a repo-relative script from inside a `shell: pwsh` step (concrete in-repo precedent for the dot-sourcing mechanics themselves)** — `scripts/verify-dark.ps1:180-181` and `278-280` dot-source sibling `.ps1` files at runtime:
```powershell
# Dot-source the gate file so Test-Precondition and Invoke-Gate enter scope (D-05).
. $gateFile
```
and (all-run loop):
```powershell
# Re-source for each gate run in all-mode to avoid function-name collisions
# between gates (each dot-source overwrites Test-Precondition/Invoke-Gate).
$gateFile = $discoveredGates[$gateName]
. $gateFile
```
Adapt for the workflow YAML `run:` blocks (repo root is `$PWD` in a `runs-on: windows-latest` step, matching the `Join-Path $PWD ...` convention already used throughout these same steps):
```powershell
. (Join-Path $PWD "scripts\verify-authenticode.ps1")
Assert-TrustedSignature -Path $artifact -Mode Strict
```

---

### `.github/workflows/trusted-signing-smoke.yml` — verify site (~L62-73)

**Current inline logic to extract** (`trusted-signing-smoke.yml:62-73`):
```yaml
      - name: Verify the embedded signature is OUR Trusted Signing cert
        shell: pwsh
        run: |
          $sig = Get-AuthenticodeSignature smoke\nono-smoke.exe
          Write-Host "Status: $($sig.Status)"
          Write-Host "Signer: $($sig.SignerCertificate.Subject)"
          Write-Host "Issuer: $($sig.SignerCertificate.Issuer)"
          if ($sig.Status -ne 'Valid') {
            Write-Error "Smoke test FAILED: Authenticode status is $($sig.Status) (expected Valid)."
            exit 1
          }
          Write-Host "OK: Trusted Signing smoke test PASSED — the D-02 signing path is live."
```
**Refactor target:** dot-source + `Assert-TrustedSignature -Path smoke\nono-smoke.exe -Mode Strict`. The pre-existing `Write-Host "Signer: ..."` / `"Issuer: ..."` lines are exactly the manual precursor to D-04's structured chain dump — the helper's happy-path output can stay this quiet (single-line signer/issuer echo), reserving the full chain dump for the failure branch only, per D-04 ("noise lands only on the failure path"). This file is the near-duplicate of `release.yml` Site 1 (same fail-closed one-liner, no MSI/broker loop) referenced in CONTEXT.md's Reusable Assets note.

## Shared Patterns

### Fail-closed `Status -ne 'Valid'` gate (unweakened, D-01)
**Source:** `.github/workflows/release.yml:269`, `:311`, `.github/workflows/trusted-signing-smoke.yml:69`
**Apply to:** `scripts/verify-authenticode.ps1`'s `Assert-TrustedSignature` Strict-mode branch — this exact condition must appear verbatim inside the helper; success criterion 3 (SIGN-02) is a diff-check against these three lines.

### `.sys` / WHQL informational carve-out (D-03)
**Source:** `.github/workflows/release.yml:298-303, 317-320`
**Apply to:** call sites only (Site 2's `.sys` foreach loop) — pass `-Mode Informational` explicitly; never infer from `.Extension` inside the helper.

### Reliable native-command exit-code capture
**Source:** `.github/workflows/release.yml:293-297` (`Start-Process -PassThru -Wait` + `.ExitCode`, already shipping); documented footgun in `scripts/verify-trust-root-cached.ps1:14-17` and `scripts/verify-dark.ps1:15` (`$PSNativeCommandUseErrorActionPreference = $false`).
**Apply to:** `Invoke-SignToolVerify` inside the new helper — do not rely on bare `& signtool ...; $LASTEXITCODE` (the pattern `sign-windows-artifacts.ps1:184-186` uses safely today only because nothing intervenes between call and check; the new helper's dual-engine interleaving breaks that safety).

### signtool SDK-path discovery
**Source:** `scripts/sign-windows-artifacts.ps1:22-58` (`Find-Signtool`)
**Apply to:** `scripts/verify-authenticode.ps1`'s signtool-location helper — reuse verbatim rather than re-deriving from RESEARCH.md's speculative version (both are equivalent in shape; the in-repo one is already proven on this project's CI).

### Repo-relative dot-sourcing at a `shell: pwsh` step
**Source:** `scripts/verify-dark.ps1:181, 280` (`. $gateFile`); workflow-side `Join-Path $PWD "..."` convention throughout `release.yml`'s existing verify steps (e.g. `release.yml:263-266`).
**Apply to:** all three workflow call sites — `. (Join-Path $PWD "scripts\verify-authenticode.ps1")` as the first line of each `run:` block, before calling `Assert-TrustedSignature`.

### Imperative, framework-free pwsh test convention
**Source:** `scripts/tests/test_windows_attach.ps1`, `scripts/tests/test_windows_detach.ps1`
**Apply to:** `scripts/tests/test_verify_authenticode.ps1` — no Pester; `$ErrorActionPreference = "Stop"` header, `Write-Host`/`Write-Error` per-case assertions, explicit exit code at the end (per `scripts/verify-trust-root-cached.ps1`'s `exit 0`/`1`/`2` discipline, since this new test script is closer to that file's "explicit pass/fail exit code" contract than to the attach/detach tests' softer `Write-Warning`-on-not-yet-implemented style).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| X509Chain / X509ChainStatusFlags classification logic (`Get-ChainClassification` inside the new helper) | utility (internal function) | transform | No existing PowerShell code in this repo touches `System.Security.Cryptography.X509Certificates.X509Chain` — this is genuinely new territory; use RESEARCH.md Pattern 4 (chain build + bitwise flag classification) and Pattern 5 (bounded retry) directly, both already grounded in official .NET BCL docs with no closer in-repo precedent to prefer over them. |
| CDP/AIA revocation-URL reachability probe (D-04) | utility (internal function) | transform | No existing script in this repo extracts CRL/AIA URLs from a certificate or probes endpoint reachability; RESEARCH.md's Open Question 3 leaves the exact probe method (`Invoke-WebRequest` vs `certutil`) as implementer discretion — default to `Invoke-WebRequest -Method Head` per research's own recommendation, no repo precedent to override it. |

## Metadata

**Analog search scope:** `scripts/`, `scripts/tests/`, `scripts/gates/`, `.github/workflows/release.yml`, `.github/workflows/trusted-signing-smoke.yml`
**Files scanned:** `scripts/verify-dark.ps1`, `scripts/tests/test_windows_attach.ps1`, `scripts/tests/test_windows_detach.ps1`, `scripts/verify-trust-root-cached.ps1`, `scripts/sign-windows-artifacts.ps1`, `scripts/windows-test-harness.ps1`, `.github/workflows/release.yml` (full verify-and-adjacent-steps region), `.github/workflows/trusted-signing-smoke.yml` (full file)
**Pattern extraction date:** 2026-07-02
