# Phase 103: Azure Clean-Host VM IaC + New Verify-Dark Gates - Pattern Map

**Mapped:** 2026-07-03
**Files analyzed:** 5 (main.bicep, deploy.ps1, teardown.ps1, trusted-signed-assertion.ps1, broker-spawn-on-clean-host.ps1)
**Analogs found:** 3 / 5 (strong role-match); 2 / 5 no in-repo analog (documented below with the authoritative substitute source)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `scripts/azure/clean-vm/main.bicep` | config (IaC template) | batch (declarative resource provisioning) | *(none in-repo)* | no analog — see "No Analog Found" |
| `scripts/azure/clean-vm/deploy.ps1` | utility (CLI wrapper script) | request-response (shells out to `az`) | `scripts/sign-windows-artifacts.ps1` (top-of-file param/strict-mode convention only — NOT a gate) | partial (structural convention match, not domain match) |
| `scripts/azure/clean-vm/teardown.ps1` | utility (CLI wrapper script) | request-response (shells out to `az`) | `scripts/sign-windows-artifacts.ps1` (same as above) | partial |
| `scripts/gates/trusted-signed-assertion.ps1` | test/gate (verdict-emitting) | event-driven (precondition→invoke→verdict) | `scripts/gates/clean-host-install.ps1` (contract shape) + `scripts/verify-authenticode.ps1` (the actual assertion logic, dot-sourced not re-implemented) | exact (contract) / exact (helper reuse) |
| `scripts/gates/broker-spawn-on-clean-host.ps1` | test/gate (verdict-emitting) | event-driven (self-contained install→run→uninstall) | `scripts/gates/clean-host-install.ps1` (elevation/dirty-host precondition, MSI install/uninstall cycle) + `scripts/gates/deploy-silent-install.ps1` (self-contained multi-step Invoke-Gate with honest-partial detail aggregation) | exact |

## Pattern Assignments

### `scripts/azure/clean-vm/main.bicep` (config, batch)

**No Analog Found** — no `.bicep` file, no `scripts/azure/` directory, and no ARM/Bicep IaC of
any kind exists anywhere in this repo today (confirmed via repo-wide search: zero `.bicep`
files, zero `az deployment`/`az group`/`az bicep` references in any `.ps1`). This phase is the
first IaC authoring in the repo.

**Use `.planning/phases/103-azure-clean-host-vm-iac-new-verify-dark-gates/103-RESEARCH.md` as the
sole source of truth**, specifically:
- The "Minimal Bicep shape for the VM resource" code block (RESEARCH.md lines ~420-453) — the VM
  resource with `securityProfile.securityType = 'TrustedLaunch'` +
  `uefiSettings.secureBootEnabled/vTpmEnabled = true`, and `vmImageSku`/`operatorIpCidr` as
  **required parameters with no default** (D-01/Anti-Patterns section — never hardcode a SKU
  default).
- The "Recommended Project Structure" block (RESEARCH.md lines ~149-159) for the 3-file layout.
- Anti-Patterns section: never a `0.0.0.0/0` NSG rule; SKU must have no default value.
- Validation command (from 103-VALIDATION.md 103-SC1a): `az bicep build -f
  scripts/azure/clean-vm/main.bicep --stdout` must exit 0 — this is a pure local
  compile+lint, no live Azure calls, and is the only automated check available for this file
  this phase.
- Security domain table (RESEARCH.md "Security Domain" section) for the ASVS V1/V4/V9/V14
  controls the template must embody (ephemeral lifecycle, single-operator-IP NSG scope, no TLS
  bypass in the deploy tooling, no default-value config).

### `scripts/azure/clean-vm/deploy.ps1` / `teardown.ps1` (utility, request-response)

**No direct role/domain analog** (no existing `az`-wrapper script in this repo — confirmed via
grep for `az deployment|az group|az bicep|ResourceGroup` across all `*.ps1`: zero matches).
These are the first Azure CLI wrapper scripts.

**Closest structural analog:** `scripts/sign-windows-artifacts.ps1` (top-of-file conventions
only — it wraps `signtool.exe`, not `az`, but is the closest existing example of "a
standalone PowerShell script that wraps an external CLI tool with mandatory params and no
default-guessing," which is exactly deploy.ps1/teardown.ps1's shape).

**Top-of-file convention to copy** (`scripts/sign-windows-artifacts.ps1` lines 1-19):
```powershell
param(
    [Parameter(Mandatory = $true)]
    [string]$CertBase64,
    ...
    # Comment explaining WHY a param has no default / is required — same discipline
    # RESEARCH.md D-01 demands for vmImageSku (no default, forces live az resolution).
    [string]$TimestampUrl = "http://timestamp.digicert.com"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
```

**Required behavior not present in any existing repo file (must be authored fresh from
RESEARCH.md, not copied):**
- `deploy.ps1` must resolve the Windows 11 SKU **live** via `az vm image list-skus` (never a
  hardcoded default) and the operator's public IP via **`curl`, never `az rest`** (Pitfall 2:
  `az rest`'s Python HTTP client fails on this network's corporate TLS interception for non-Azure
  hosts; `curl`, backed by Windows SChannel, succeeds). See RESEARCH.md Pitfall 1/2 and the
  "Don't Hand-Roll" table entry ("Live operator public-IP lookup").
- Both scripts must **never** use `-k`/`--insecure`/cert-bypass flags to route around the TLS
  errors in Pitfall 1/2 — fail closed on any `SSLCertVerificationError`, per RESEARCH.md's
  Security Domain "Known Threat Patterns" table.
- `teardown.ps1` must document (in a header comment, matching the threat-model-comment style
  seen in `deploy-silent-install.ps1` lines 36-42) the ephemeral create→use→teardown lifecycle
  and that it targets a dedicated ephemeral RG (open question in RESEARCH.md — confirm resolution
  in CONTEXT.md/plan before hardcoding an RG name).
- Neither script is run live this phase (author + `az bicep build`/code-review only per
  103-VALIDATION.md) — do not gate their correctness on a live `az deployment group create` call.

### `scripts/gates/trusted-signed-assertion.ps1` (test/gate, event-driven)

**Analog:** `scripts/gates/clean-host-install.ps1` (two-function contract shape) +
`scripts/verify-authenticode.ps1` (the dot-sourced assertion helper — reuse, do not
reimplement).

**Header/contract-comment pattern to copy verbatim in style** (`clean-host-install.ps1` lines 1-27):
```powershell
# scripts/gates/clean-host-install.ps1
#
# Phase 80 - clean-host-install gate (INST-01)
#
# CONTRACT (mirrors scripts/gates/harness-self-check.ps1, the reference contract for
# phases 77-81): this gate exports exactly two functions dot-sourced by
# scripts/verify-dark.ps1. The gate RETURNS its verdict object - it MUST NOT call exit and
# MUST NOT call Persist-Verdict. Only the runner owns exit-code mapping (PASS=0 / FAIL=2 /
# SKIP_HOST_UNAVAILABLE=3 / harness-internal=4) and the persist-before-emit (WR-04).
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE - exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
```

**Precondition pattern — elevation check, exact two-line form to reuse**
(`clean-host-install.ps1` lines 72-76, itself sourced from `wfp-egress-isolation.ps1:112-115`):
```powershell
$identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    return 'clean-host-install gate requires elevation (machine MSI install needs admin) - re-run from an elevated shell'
}
```
Note: `trusted-signed-assertion.ps1` likely does NOT need the elevation check (signature
verification is not a privileged operation) — but it DOES need its own
`Test-Precondition` returning a reason string when the artifact-to-verify is not staged (mirror
`clean-host-install.ps1` lines 91-95's `-LiteralPath` MSI-staged check, substituting the signed
artifact path).

**Dot-source + throw-to-verdict translation pattern (the load-bearing pattern for this file)** —
copy this shape exactly, from RESEARCH.md Pattern 2 (which itself cites live-read
`verify-authenticode.ps1:272-359`):
```powershell
. (Join-Path (Split-Path -Parent $PSScriptRoot) 'verify-authenticode.ps1')

function Invoke-Gate {
    $ErrorActionPreference = 'Continue'
    try {
        $result = Assert-TrustedSignature -Path $script:StagedArtifactPath -Mode 'Strict'
        return [ordered]@{
            gate = 'trusted-signed-assertion'; verdict = 'PASS'
            reason = "Authenticode Valid (dual-engine) for $($script:StagedArtifactPath)"
            detail = [ordered]@{ gasStatus = $result.GasStatus; signtoolExit = $result.SignToolExitCode }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    } catch {
        # Assert-TrustedSignature THROWS on non-Valid (verify-authenticode.ps1:341-350) —
        # this MUST be caught here and translated to a FAIL verdict. An uncaught throw
        # inside Invoke-Gate is classified HARNESS_ERROR/exit 4 by verify-dark.ps1's own
        # try/catch (verify-dark.ps1 lines 213-221), which would misclassify a legitimate
        # signature failure as a harness bug, not a FAIL.
        return [ordered]@{
            gate = 'trusted-signed-assertion'; verdict = 'FAIL'
            reason = "Authenticode assertion failed: $_"
            detail = [ordered]@{}
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }
}
```

**CRITICAL — do not gate on issuer substring** (RESEARCH.md Pitfall 3, direct citation of
`.planning/phases/101-verify-gate-hardening-azure-profile-confirmation/101-SIGN03-SMOKE-VERDICT.md`
Finding B): only `Get-AuthenticodeSignature.Status -eq 'Valid'` (via `Assert-TrustedSignature
-Mode Strict`) is a pass/fail condition. Issuer string must be captured into `detail` as
informational-only, via a **second, direct** `Get-AuthenticodeSignature -LiteralPath $path` call
(the `Assert-TrustedSignature` returned object does NOT expose `.Issuer` — see RESEARCH.md
"Important gap" note under Pattern 2). A header comment must explicitly flag that issuer-substring
matching was evaluated and rejected, citing `101-SIGN03-SMOKE-VERDICT.md`, mirroring the
threat-model-comment style of `deploy-silent-install.ps1` lines 36-42.

**Verdict-object shape / key order** — must match `verify-dark.ps1`'s own `Build-Verdict`
locked key order exactly (`verify-dark.ps1` lines 29-44):
```powershell
[ordered]@{
    gate      = $GateName
    verdict   = $Verdict
    reason    = $Reason
    detail    = $Detail
    timestamp = Get-IsoTimestamp
}
```

### `scripts/gates/broker-spawn-on-clean-host.ps1` (test/gate, event-driven, self-contained)

**Analog:** `scripts/gates/clean-host-install.ps1` (precondition + install/uninstall cycle) AND
`scripts/gates/deploy-silent-install.ps1` (self-contained multi-step `Invoke-Gate` with honest
partial-legs aggregation — the closer structural match since this gate also does more than one
proof step).

**Precondition pattern to clone verbatim (elevation + dirty-host + artifact-staged)**
(`clean-host-install.ps1` lines 65-98, already reproduced above in RESEARCH.md Pattern 1 —
copy this shape exactly, substituting the MSI default path if a different artifact is used):
```powershell
function Test-Precondition {
    $identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return '... gate requires elevation ... re-run from an elevated shell'
    }
    if (Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe') {
        return 'nono.exe detected under C:\Program Files\nono — host is not clean; ...'
    }
    if (-not (Test-Path -LiteralPath $script:MsiPath)) {
        return "MSI not found at $($script:MsiPath) - stage it on this VM before running the gate"
    }
    return $null
}
```
On the current dev host this gate MUST SKIP via **both** the non-elevation branch AND the
dirty-host branch independently [VERIFIED live per RESEARCH.md: `nono.exe` present under
`C:\Program Files\nono\`, current pwsh session non-elevated]. Per RESEARCH.md Pitfall 4: do NOT
treat "it SKIPped" alone as proof the dirty-host branch logic is correct — a code-review pass
confirming the exact `-LiteralPath`/path string is still required since the elevation branch can
mask a bug in the dirty-host branch on this host.

**Self-contained install→run→uninstall shape to copy** (`clean-host-install.ps1` lines 100-188,
`Invoke-Gate`'s Start-Process idiom for msiexec + fresh-session `nono --version` check):
```powershell
$installArgs = @('/i', $script:MsiPath, '/quiet', '/norestart', '/l*v', $installLogPath)
$installProc = Start-Process -FilePath 'msiexec.exe' -ArgumentList $installArgs -Wait -PassThru -NoNewWindow
$installExit = $installProc.ExitCode
$installOk   = ($installExit -eq 0 -or $installExit -eq 3010)   # 3010 = success + reboot required
...
# fresh-session check (PATH propagation) via Start-Process pwsh.exe -NoProfile -NonInteractive -Command
...
# uninstall cleanup at the end for host repeatability
$uninstallArgs = @('/x', $script:MsiPath, '/quiet', '/norestart')
Start-Process -FilePath 'msiexec.exe' -ArgumentList $uninstallArgs -Wait -PassThru -NoNewWindow
```

**Broker-spawn-specific step to author fresh (no existing analog runs `nono run --profile
claude-code`):** after install + PATH-propagation proof, this gate must additionally invoke
`nono run --profile claude-code` (per RESEARCH.md Architectural Responsibility Map: "the gate
does not implement broker-spawn logic — it drives the already-built `nono run --profile
claude-code` command and observes exit/behavior") and record the observed spawn behavior in
`detail`, following the same `$stamp`/`$detail`-assembled-before-verdict-branch idiom used in both
analogs.

**Honest-partial detail aggregation pattern** (mirror `deploy-silent-install.ps1` lines 121-125,
487-535 — build `$detail[stepN_*]` incrementally, classify `$hardFails` vs `$partialLegs`
separately, only `$hardFails.Count -gt 0` flips the verdict to FAIL):
```powershell
$detail = [ordered]@{
    # Populated as legs execute.
}
...
$hardFails = [System.Collections.Generic.List[string]]::new()
...
if ($hardFails.Count -gt 0) {
    return [ordered]@{ gate = '...'; verdict = 'FAIL'; reason = "..."; detail = $detail; timestamp = & $stamp }
}
```

**CRITICAL anti-pattern (RESEARCH.md Anti-Patterns section, explicit):** this gate must NOT
depend on `clean-host-install.ps1` having already run in the same `-All` sweep — alphabetical
dispatch order (`broker-spawn-on-clean-host` sorts before `clean-host-install`, `b` < `c` per
`verify-dark.ps1` line 276 `$discoveredGates.Keys | Sort-Object`) means it must install/uninstall
its **own** copy of the MSI within its own `Invoke-Gate`, never assume prior-gate state.

## Shared Patterns

### Gate two-function contract (applies to both new gate files)
**Source:** `scripts/gates/clean-host-install.ps1` (reference contract), enforced by the runner
at `scripts/verify-dark.ps1` lines 170-250 (single-gate) and 260-354 (`-All` sweep).
**Apply to:** `trusted-signed-assertion.ps1`, `broker-spawn-on-clean-host.ps1`.
```powershell
function Test-Precondition {
    # Return $null (run Invoke-Gate) or a reason string (SKIP_HOST_UNAVAILABLE). MUST NOT throw
    # — a throw here is a harness-internal error (exit 4), not a SKIP/FAIL verdict.
}
function Invoke-Gate {
    # Returns exactly one [ordered]@{ gate; verdict; reason; detail; timestamp } object.
    # NEVER calls exit. NEVER calls Persist-Verdict. verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }.
    # A throw here IS legitimate for "gate cannot run at all" but is caught by the RUNNER as
    # HARNESS_ERROR — a caught, translated FAIL (per Pattern 2 above) must be RETURNED, not thrown.
}
```

### Zero-harness-changes discovery contract
**Source:** `scripts/verify-dark.ps1` lines 133-145 (`Get-ChildItem -Path $gatesDir -Filter
"*.ps1" -File | Sort-Object Name`).
**Apply to:** both new gate files — placing them under `scripts/gates/*.ps1` is sufficient for
auto-discovery; no harness edit is permitted or needed. Verify with `git diff --stat
scripts/verify-dark.ps1` staying empty (103-VALIDATION.md 103-REG).

### Verdict key order / timestamp format
**Source:** `scripts/verify-dark.ps1` lines 23-27 (`Get-IsoTimestamp`) and 29-44 (`Build-Verdict`
locked order: `gate, verdict, reason, detail, timestamp`).
**Apply to:** both new gate files' `Invoke-Gate` return objects — the runner re-stamps `gate` and
`timestamp` anyway (verify-dark.ps1 lines 231-232, 331-332), but the gate's own return should
still follow this order for readability/consistency with every existing gate file.

### Throw-vs-return discipline (never let a throw become a silent PASS or a false HARNESS_ERROR)
**Source:** `scripts/verify-authenticode.ps1` lines 341-350 (`Assert-TrustedSignature` throws on
non-Valid) + `scripts/verify-dark.ps1` lines 212-221 (`Invoke-Gate` throw → HARNESS_ERROR/exit 4).
**Apply to:** `trusted-signed-assertion.ps1` specifically — this is the one place in this phase
where a legitimate helper's `throw` must be deliberately caught and reclassified as a `FAIL`
verdict inside the gate, not allowed to propagate (see Pattern 2 excerpt above).

### External-CLI-wrapper param/strict-mode convention
**Source:** `scripts/sign-windows-artifacts.ps1` lines 1-19 (`param(...)` with
`[Parameter(Mandatory = $true)]` for anything that must never silently default, plus
`Set-StrictMode -Version Latest` / `$ErrorActionPreference = "Stop"` immediately after the param
block).
**Apply to:** `deploy.ps1` / `teardown.ps1` — `vmImageSku`-equivalent and `operatorIpCidr`-equivalent
inputs should be Mandatory params (no silent default), matching RESEARCH.md D-01's "SKU has no
default" requirement at the Bicep layer too.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `scripts/azure/clean-vm/main.bicep` | config (IaC) | batch | No `.bicep` file or `scripts/azure/` directory exists anywhere in the repo — this is the first IaC authoring. Use `103-RESEARCH.md`'s "Minimal Bicep shape for the VM resource" code block as the authoritative template; validate via `az bicep build -f main.bicep --stdout` (exit 0), not a repo-analog diff. |
| `scripts/azure/clean-vm/deploy.ps1` | utility | request-response | No existing `az`-CLI wrapper script in this repo (confirmed via grep: zero matches for `az deployment\|az group\|az bicep\|ResourceGroup` across all `*.ps1`). Only a structural (param/strict-mode) convention analog exists (`sign-windows-artifacts.ps1`); the `az`-specific logic (live SKU resolution, `curl`-not-`az rest` IP lookup, ephemeral-RG teardown wiring) must be authored fresh from `103-RESEARCH.md`'s Pitfall 1/2 and "Don't Hand-Roll" sections. |
| `scripts/azure/clean-vm/teardown.ps1` | utility | request-response | Same as `deploy.ps1` above. |

## Metadata

**Analog search scope:** `scripts/gates/*.ps1` (all 10 files enumerated), `scripts/verify-dark.ps1`,
`scripts/verify-authenticode.ps1`, `scripts/sign-windows-artifacts.ps1`; repo-wide search for any
`.bicep` file or any `az deployment`/`az group`/`az bicep`/`ResourceGroup` reference in any `.ps1`
(zero matches, confirming no IaC/Azure-wrapper precedent exists).
**Files scanned (full read):** `scripts/verify-dark.ps1` (386 lines), `scripts/gates/clean-host-install.ps1`
(188 lines), `scripts/gates/deploy-silent-install.ps1` (553 lines), `scripts/verify-authenticode.ps1`
(359 lines); partial read: `scripts/sign-windows-artifacts.ps1` (lines 1-40, top-of-file convention only).
**Pattern extraction date:** 2026-07-03
