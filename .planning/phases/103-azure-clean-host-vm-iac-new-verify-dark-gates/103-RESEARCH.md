# Phase 103: Azure Clean-Host VM IaC + New Verify-Dark Gates - Research

**Researched:** 2026-07-03
**Domain:** Azure Bicep IaC (author-only) + PowerShell verify-dark gate harness extension
**Confidence:** HIGH (gate-contract mechanics and az/bicep tooling were live-verified on the dev host; issuer-naming and az-network reachability findings are HIGH-confidence live evidence, not training-data guesses)

## Summary

This phase is pure **authoring + local validation** — no live VM, no live deploy, no live gate
PASS. Two independent deliverables:

1. **`scripts/azure/clean-vm/main.bicep`** (+ deploy/teardown scripts): a self-contained Bicep
   module that creates its own VNet/subnet/NSG/PublicIP/NIC/VM (resource group `RG_Nono`
   currently contains only the `ArtifactNono` code-signing account — no reusable networking
   exists), targeting a Gen2 + Trusted-Launch `MicrosoftWindowsDesktop:windows-11` VM with the
   SKU resolved live via `az vm image list-skus`. **Live-verified on this dev host:** all
   `win11-24h2-*`/`win11-25h2-*` SKUs under that publisher/offer report `hyperVGeneration: V2`
   and `SecurityType: TrustedLaunchAndConfidentialVmSupported` — Gen2 + Trusted Launch is the
   *default* shape for this image family, not something that needs a special `-g2` SKU suffix.
   The correct local validation is `az bicep build` (pure compile+lint, zero Azure calls) — this
   **works on the dev host**, but only after a manual workaround (below) because `az bicep
   install` itself fails from corporate TLS interception.

2. **Two new `scripts/gates/*.ps1` files** (`trusted-signed-assertion.ps1`,
   `broker-spawn-on-clean-host.ps1`) that plug into the existing `verify-dark.ps1` harness with
   **zero harness code changes** — the harness auto-discovers any `*.ps1` under `scripts/gates/`
   by filename (confirmed by reading `verify-dark.ps1`: `Get-ChildItem ... -Filter "*.ps1"`, no
   hardcoded list). Both gates must return `SKIP_HOST_UNAVAILABLE` on this dev host today.

**Primary recommendation:** Author the Bicep module as a fully self-contained template (own
VNet/NSG/PIP, no dependency on pre-existing RG_Nono resources), validate it locally with `az
bicep build -f main.bicep` (after installing bicep via the manual-download workaround — `az
bicep install` itself is blocked on this host). Author both gates by cloning the exact two-function
contract from `scripts/gates/clean-host-install.ps1` (closest analog: elevation check + dirty-host
detection + artifact-staged check, all returning a SKIP reason string). Do **not** assert on a
literal issuer substring in `trusted-signed-assertion.ps1` — Phase 101's live evidence
(`101-SIGN03-SMOKE-VERDICT.md`) proved that heuristic wrong on a genuinely-valid signature; assert
only `Get-AuthenticodeSignature.Status -eq 'Valid'` via the shared helper (which already does the
dual-engine AND-gate) and treat issuer-string capture as **informational detail**, not a pass/fail
condition, until Phase 104/106 produce a real signed artifact to calibrate against.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| VM/network provisioning (Bicep) | Azure Resource Manager (IaC) | — | ARM/Bicep is the only tier that can declare Gen2+Trusted-Launch+NSG state; no app-tier code involved |
| SKU resolution | Operator/deploy-script (`az` CLI, pre-deploy) | Bicep param | Bicep receives the SKU as a parameter; the *live query* that resolves it is a shell-script concern, not template logic (Bicep has no `az vm image list-skus` equivalent at compile time) |
| Gate discovery/dispatch | `verify-dark.ps1` harness (PowerShell) | — | Fixed contract; new gates are pure data files to this tier, requiring zero harness changes |
| Signature assertion | `verify-authenticode.ps1` shared helper (PowerShell, dot-sourced) | New gate file | The gate is a thin orchestration wrapper; the actual dual-engine (GAS + signtool) logic lives in the Phase 101 helper and must not be duplicated |
| Broker spawn proof | `nono-cli` (installed binary under test) | New gate file (orchestrates install/run/uninstall) | The gate does not implement broker-spawn logic — it drives the already-built `nono run --profile claude-code` command and observes exit/behavior |
| Precondition/host-availability detection | New gate file (`Test-Precondition`) | — | Each gate owns its own clean/dirty-host and artifact-staged detection; the harness only consumes the return value |

## Standard Stack

### Core

| Tool | Version (live-confirmed) | Purpose | Why Standard |
|------|---------------------------|---------|---------------|
| Azure CLI (`az`) | 2.87.0 [VERIFIED: `az --version` on dev host] | Deploy/teardown scripts, live SKU resolution, RG operations | Repo is explicitly `az`-native (CLAUDE.md v3.5 constraint: "no Terraform/Pulumi") |
| Bicep CLI | 0.44.1 [VERIFIED: live-installed and run on dev host via manual workaround, see Pitfall 1] | Compile/lint `main.bicep`, `az bicep build` | Native ARM authoring format for `az`; no separate toolchain needed once installed |
| PowerShell (`pwsh`) | 7.6.3 [VERIFIED: `$PSVersionTable` on dev host] | Gate scripts, `verify-dark.ps1` harness | Matches every existing gate in `scripts/gates/` |

### Supporting

| Tool | Purpose | When to Use |
|------|---------|-------------|
| `az vm image list-skus -p MicrosoftWindowsDesktop -f windows-11 -l <region>` | Live SKU enumeration | Deploy script only — never hardcode a SKU string in `main.bicep`'s default; pass it as a required/no-default param so a stale hardcode cannot silently ship |
| `az vm image show --urn <pub>:<offer>:<sku>:latest` | Confirms `hyperVGeneration`/`SecurityType` for a chosen SKU before deploy | Optional extra verification step in the deploy script's preflight |
| `az deployment group create --template-file` | Actual (future, Phase 106) live deploy | NOT this phase — author + `what-if` only |
| `az deployment group what-if` | Optional deeper pre-deploy diff/validation | Requires a live Azure call (safe — does not create resources) but needs a target resource group and a concrete parameter file; treat as optional/stretch, `az bicep build` alone satisfies "author + lint" |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Bicep | ARM JSON directly | Bicep transpiles to identical ARM JSON (confirmed: `az bicep build` emits `$schema: .../deploymentTemplate.json#`) but is far more readable; no reason to hand-write JSON |
| Bicep | Terraform/Pulumi | Explicitly out of scope per CLAUDE.md v3.5 constraint ("repo is `az`-native... no Terraform/Pulumi") |
| `az vm image list-skus` live resolution | Hardcoded SKU string | Explicitly forbidden by CHOST-01/ROADMAP wording — SKUs deprecate/rotate (e.g. `win11-23h2-*` are already superseded by `win11-24h2-*`/`win11-25h2-*` in the live list) |

**Installation (this phase does NOT run these against a live subscription — documented for the deploy script's own preflight):**
```bash
az bicep install           # SEE PITFALL 1: fails on this dev host due to corporate TLS interception
az extension list          # confirm no stale az extensions interfere
```

## Package Legitimacy Audit

Not applicable — this phase installs no npm/pip/cargo packages. All tooling (`az`, `bicep`,
`pwsh`) is pre-existing infrastructure tooling already present on the dev host and CI runners,
not a new project dependency. Package Legitimacy Gate is skipped.

## Architecture Patterns

### System Architecture Diagram

```
                     ┌─────────────────────────────────────────────┐
                     │  Operator workstation (dev host, this repo)   │
                     │                                               │
  az vm image        │  1. az vm image list-skus  ──► live SKU str   │
  list-skus  ────────┼─►                                             │
  (LIVE, read-only)  │  2. main.bicep (+ param file) ──► az bicep    │
                     │       build  ──► compiled ARM JSON (local,     │
                     │       no Azure call)                          │
                     │                                               │
                     │  3. deploy.ps1 (NOT run this phase) would:    │
                     │       az deployment group create              │
                     │         --resource-group RG_Nono               │
                     │         --template-file main.bicep              │
                     │         --parameters vmSku=<live> operatorIp=<> │
                     │       ──► [Gen2+TrustedLaunch VM + VNet/NSG]   │
                     │       (deferred to Phase 106)                  │
                     │                                               │
                     │  4. teardown.ps1 (NOT run this phase) would:  │
                     │       az group delete / az deployment group    │
                     │         delete-then-cleanup ──► ephemeral      │
                     │       (deferred to Phase 106)                  │
                     └─────────────────────────────────────────────┘

                     ┌─────────────────────────────────────────────┐
                     │  scripts/verify-dark.ps1 (existing harness)   │
                     │                                               │
  -All / -Gate ──────┼─► Get-ChildItem scripts/gates/*.ps1            │
                     │       (auto-discover, alphabetical sort)      │
                     │         │                                     │
                     │         ▼                                     │
                     │   dot-source gate file                        │
                     │         │                                     │
                     │         ▼                                     │
                     │   Test-Precondition()                         │
                     │     null ──► Invoke-Gate() ──► verdict object  │
                     │     "reason" ──► SKIP_HOST_UNAVAILABLE (never  │
                     │                   enters Invoke-Gate)          │
                     │         │                                     │
                     │         ▼                                     │
                     │   Persist-Verdict (BEFORE stdout emit)         │
                     │         │                                     │
                     │         ▼                                     │
                     │   exit 0/2/3/4 (PASS/FAIL/SKIP/HARNESS_ERROR)  │
                     └─────────────────────────────────────────────┘
                              ▲                        ▲
                              │                        │
             trusted-signed-assertion.ps1   broker-spawn-on-clean-host.ps1
             (dot-sources verify-authenticode.ps1,     (self-contained
              calls Assert-TrustedSignature)             install→run→
                                                          uninstall)
```

### Recommended Project Structure
```
scripts/azure/clean-vm/
├── main.bicep           # Gen2 + Trusted-Launch VM + VNet/subnet/NSG/PIP/NIC, all in one module
├── deploy.ps1           # az deployment group create wrapper; resolves SKU + operator IP live, passes as params
└── teardown.ps1         # az group/deployment cleanup; documents the ephemeral lifecycle explicitly

scripts/gates/
├── trusted-signed-assertion.ps1        # dot-sources ../verify-authenticode.ps1
└── broker-spawn-on-clean-host.ps1      # self-contained install→run→uninstall
```

### Pattern 1: Gate two-function contract (mirrors `clean-host-install.ps1`)
**What:** Every gate file exports exactly `Test-Precondition` (returns `$null` or a reason
string) and `Invoke-Gate` (returns an `[ordered]@{ gate; verdict; reason; detail; timestamp }`
object). The gate **never calls `exit`** and **never calls `Persist-Verdict`** — only the
runner (`verify-dark.ps1`) owns exit-code mapping and file persistence.
**When to use:** Both new gates, verbatim.
**Example (precondition pattern to clone from `clean-host-install.ps1`, live-read):**
```powershell
# Source: scripts/gates/clean-host-install.ps1:65-98 (existing repo code, closest analog)
function Test-Precondition {
    # 1. Elevation check (machine-scope MSI install needs admin)
    $identity  = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        return '... gate requires elevation ... re-run from an elevated shell'
    }
    # 2. Dirty-host detection (nono.exe already installed => not a clean host)
    if (Test-Path -LiteralPath 'C:\Program Files\nono\nono.exe') {
        return 'nono.exe detected under C:\Program Files\nono — host is not clean; ...'
    }
    # 3. Artifact-staged check
    if (-not (Test-Path -LiteralPath $script:MsiPath)) {
        return "MSI not found at $($script:MsiPath) - stage it on this VM before running the gate"
    }
    return $null
}
```
**Live-verified relevance:** on THIS dev host, `nono.exe` **is** installed under
`C:\Program Files\nono\nono.exe` [VERIFIED: `Test-Path` check run live] and the current pwsh
session is **not elevated** (`IsInRole(Administrator) = False`) [VERIFIED: live check]. Any gate
that clones this precondition shape will legitimately SKIP on this host via **two independent
signals** (non-elevation AND dirty-host) — a strong double-guarantee for `SC4`.

### Pattern 2: Reusing a throw-based helper inside a verdict-returning gate
**What:** `Assert-TrustedSignature` (Phase 101 helper) **throws** on a non-Valid signature —
this is correct for its original CI call sites (a thrown error fails the GitHub Actions step),
but the verify-dark gate contract requires `Invoke-Gate` to **return** a `FAIL` verdict object,
not let an exception propagate (an uncaught throw inside `Invoke-Gate` is classified by the
harness as `HARNESS_ERROR`/exit 4, per `verify-dark.ps1`'s own `try { $verdictObj = Invoke-Gate }
catch { ... harness-internal error ... }` block — this is the correct behavior for a genuine
harness bug, but a caught signature-invalid condition is a legitimate FAIL, not a harness bug).
**When to use:** `trusted-signed-assertion.ps1`'s `Invoke-Gate` MUST wrap its
`Assert-TrustedSignature` call in `try { ... } catch { return FAIL-verdict }`.
**Example:**
```powershell
# Source: scripts/verify-authenticode.ps1:272-359 (function signature, confirmed live-read)
. (Join-Path (Split-Path -Parent $PSScriptRoot) 'verify-authenticode.ps1')

function Invoke-Gate {
    $ErrorActionPreference = 'Continue'
    try {
        $result = Assert-TrustedSignature -Path $script:StagedArtifactPath -Mode 'Strict'
        # $result.Passed -eq $true only on success; a throw is caught below.
        return [ordered]@{
            gate = 'trusted-signed-assertion'; verdict = 'PASS'
            reason = "Authenticode Valid (dual-engine) for $($script:StagedArtifactPath)"
            detail = [ordered]@{ gasStatus = $result.GasStatus; signtoolExit = $result.SignToolExitCode }
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    } catch {
        # Assert-TrustedSignature throws on non-Valid — translate to a FAIL verdict, do NOT
        # let this propagate (that would be misclassified as HARNESS_ERROR by the runner).
        return [ordered]@{
            gate = 'trusted-signed-assertion'; verdict = 'FAIL'
            reason = "Authenticode assertion failed: $_"
            detail = [ordered]@{}
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }
}
```
**Important gap:** `Assert-TrustedSignature`'s **returned object** (`Path`/`GasStatus`/
`SignToolExitCode`/`Passed`) does **not** expose the signer's `Issuer` string — that field only
appears in the `Write-Host`/`Write-ChainDiagnostic` unstructured failure-path output, never in
returned data. If the gate also needs to assert/record issuer-chain info (CHOST-02 SC2 says
"issuer chaining to the `Microsoft ID Verified CS` root"), it must **independently** call
`Get-AuthenticodeSignature -LiteralPath $path` a second time itself to read
`.SignerCertificate.Issuer` — see Pitfall 3 for why this should be recorded as **informational
detail**, not a pass/fail gate condition.

### Anti-Patterns to Avoid
- **Hardcoding a Windows 11 SKU string in `main.bicep`'s default parameter value:** the live SKU
  list already shows generational churn (`win11-23h2-*` superseded by `24h2`/`25h2` within the
  same query) — a hardcoded default silently goes stale. Make the SKU a **required parameter
  with no default**, forcing the deploy script's live `az vm image list-skus` resolution to be
  the only path to a value.
- **Asserting a literal issuer substring as a pass/fail gate condition:** Phase 101's live
  evidence (see Pitfall 3) proved this produces false negatives on a genuinely valid signature.
- **`broker-spawn-on-clean-host.ps1` depending on `clean-host-install.ps1` having already run:**
  alphabetical `-All` dispatch order is `broker-spawn-on-clean-host` **before**
  `clean-host-install` (`b` < `c`) — the gate must install/uninstall its own copy of the MSI
  within its own `Invoke-Gate`, never assume a prior gate staged state.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Authenticode dual-engine verification (GAS + signtool) | A new signature-check function in the gate | `Assert-TrustedSignature` (Phase 101 shared helper, dot-sourced) | CHOST-02 SC2 explicitly requires reuse; re-implementing risks losing the D-02 transient-retry classification and re-introducing the exact bug class SIGN-02 fixed |
| Gate auto-discovery / dispatch / exit-code mapping | Any change to `verify-dark.ps1` | The existing `Get-ChildItem *.ps1` auto-discovery | CHOST-02 SC2 explicitly requires **zero harness code changes**; the harness is already generic |
| Live operator public-IP lookup for NSG scoping | A hand-rolled Azure REST call via `az rest` | `curl` against a public IP-echo endpoint (`ifconfig.me`, `api.ipify.org`), invoked from the deploy script, NOT from `az rest` | `az rest`'s Python HTTP client fails on this corporate network for domains outside `management.azure.com`/`login.microsoftonline.com` (see Pitfall 2) — `curl` (Windows SChannel-backed) succeeds where `az`'s Python `requests`+`certifi` client does not |

**Key insight:** Both "don't hand-roll" items in this phase are about **reuse discipline**, not
avoiding a third-party library — the phase's entire point is to slot two new files into existing,
already-hardened machinery (the verify-dark harness, the Phase 101 signature helper) without
touching either.

## Common Pitfalls

### Pitfall 1: `az bicep install` fails on this dev host (corporate TLS interception) — but there is a working manual fallback
**What goes wrong:** `az bicep install` (and `az bicep version` before any manual fix) fails with:
```
ERROR: Error while attempting to retrieve the latest Bicep version:
HTTPSConnectionPool(host='aka.ms', port=443): Max retries exceeded ...
SSLError(SSLCertVerificationError(1, '[SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed:
unable to get local issuer certificate ...'))
```
[VERIFIED: reproduced live on this dev host, 2026-07-03]
**Why it happens:** `az`'s bundled Python HTTP client (`requests`/`certifi`) does not trust the
corporate TLS-inspection root CA for `aka.ms` (Bicep's release-redirect host). `curl`, by
contrast, uses the Windows native SChannel trust store which already has that corporate root
installed, and **succeeds** against the same URL:
`curl -sL https://github.com/Azure/bicep/releases/latest/download/bicep-win-x64.exe` returned
HTTP 200 and a valid 111 MB binary [VERIFIED: live download + `bicep.exe --version` → `Bicep CLI
version 0.44.1 (28275db947)` succeeded on this dev host].
**How to avoid:** The deploy script's preflight (or a one-time manual setup step documented in
the phase's plan) should:
1. `curl -sL -o bicep.exe https://github.com/Azure/bicep/releases/latest/download/bicep-win-x64.exe`
   (retry on transient timeout — first attempt truncated at 67 MB/111 MB and produced a corrupt
   binary that failed with `Arithmetic overflow while reading bundle`; a second attempt with a
   longer timeout completed cleanly).
2. Place the binary at `%USERPROFILE%\.azure\bin\bicep.exe` (the path `az` probes for a
   manually-supplied Bicep binary — confirmed live: `az bicep version` succeeded immediately
   after copying the file there, with no `az config` changes needed).
3. `az bicep build -f main.bicep` then works with **zero further Azure/network calls** — it is a
   pure local compile+lint, confirmed live on a throwaway test `.bicep` file (emitted valid ARM
   JSON with `$schema`/`resources`/`parameters`, plus 3 expected linter warnings for unused
   test-file params).
**Warning signs:** Any `SSLCertVerificationError` mentioning `aka.ms` or a non-`management.azure.com`
/`login.microsoftonline.com` host from an `az` command is this same corporate-TLS-interception
class of failure (also previously seen with `az trustedsigning` extension install in Phase 101,
per project memory) — the fix pattern (download via `curl`, place the binary manually) applies
generally, not just to Bicep.

### Pitfall 2: Live operator-IP lookup returns a *different* IP per provider/call on this network
**What goes wrong:** Fetching the "current public IP" for NSG scoping is not a single stable
value on this corporate network. Three separate live calls returned **three different IPs**:
`curl https://ifconfig.me` → `170.85.72.192`; `curl https://api.ipify.org` → `209.246.110.194`;
a repeat of `ifconfig.me`-equivalent → `170.85.72.180` [VERIFIED: all three run live in the same
session, 2026-07-03]. Separately, `az rest --method get --url "https://api.ipify.org?format=json"`
**failed outright** with the same TLS-interception error as Pitfall 1 (confirming `az`'s own HTTP
client cannot be used for this lookup at all on this network), while `management.azure.com`-scoped
`az` calls (`az account show`, `az group show`, `az vm image list-skus`) all succeeded normally.
**Why it happens:** The corporate network appears to route outbound traffic through a NAT/proxy
pool with multiple egress IPs, and/or per-provider edge routing differs; `az`'s Python client is
also more narrowly cert-trusted than `curl`'s SChannel-backed client, only working for the
specific Azure-first-party hosts already covered by the existing `az login` corporate trust
config.
**How to avoid:** Do NOT assume a single deterministic IP-detection call is safe for a hard
allow-list NSG rule on this kind of network. Document in the deploy script/runbook that: (a) the
operator's effective public IP must be captured via `curl` (not `az rest`) **immediately before**
`az deployment group create` runs (minimizing the window for IP drift), (b) if repeated calls
disagree, the operator should manually confirm via `whatismyip`-style browser check before
committing the NSG rule, and (c) treat this as a real (not hypothetical) operational risk to
flag for the planner — a rule scoped to a since-changed IP silently locks the operator out of
RDP, which is an availability bug, not a security bug, but still worth a documented remediation
path (e.g. redeploy just the NSG rule, or widen briefly via portal).
**Warning signs:** RDP connection refused/timeout immediately after a fresh deploy — first
suspect is IP drift between "resolve IP" and "deploy," not a Bicep/NSG authoring bug.

### Pitfall 3: The roadmap's issuer-naming assertion (`Microsoft ID Verified CS` root) is DISPROVEN by Phase 101's live evidence — do not bake it into a pass/fail gate condition
**What goes wrong:** CHOST-02 (REQUIREMENTS.md) and ROADMAP Phase 103 SC2 both specify:
"...assert Authenticode `Valid` + issuer chaining to the `Microsoft ID Verified CS` root..." This
exact naming was ALSO the pass condition baked into Phase 101 Plan 04's SIGN-03 acceptance
criteria — and it was **falsified by a live, genuinely-valid signature**.
[CITED: `.planning/phases/101-verify-gate-hardening-azure-profile-confirmation/101-SIGN03-SMOKE-VERDICT.md`,
Finding (B), 2026-07-02/03] The authoritative live smoke run (`28636133664`, on an
operator-confirmed **`PublicTrust`**-profile signature) produced:
```
Signer: CN=TWGGLOBAL.onmicrosoft.com, ...
Issuer: CN=Microsoft Enterprise ID Verified Policy AOC CA 02, O=Microsoft Corporation, C=US
```
— the **exact** `Enterprise ID Verified Policy AOC CA` substring that the research/roadmap
labeled as the tell for the *untrusted* `PublicTrustTest` profile. Finding (B) states explicitly:
*"`PublicTrust` can and does chain through an `Enterprise ID Verified Policy AOC CA` issuer... Any
future gate must not resurrect this differentiator as a pass/fail condition."*
**Why it happens:** Azure Trusted Signing's actual PKI hierarchy issues intermediate certs under
the `Enterprise ID Verified Policy AOC/EOC CA` naming for BOTH PublicTrust and PublicTrustTest
profiles in practice — the naming convention referenced in early documentation/research
(`Microsoft ID Verified CS EOC/AOC CA NN`) does not match what the live tenant actually issues,
or the CS-branded root is a level higher in the chain than the leaf issuer that
`Get-AuthenticodeSignature.SignerCertificate.Issuer` reports (immediate parent, not full chain).
**How to avoid:** For this phase (author-only, no real signed artifact exists yet to test
against), `trusted-signed-assertion.ps1` should:
1. Assert `Get-AuthenticodeSignature.Status -eq 'Valid'` via `Assert-TrustedSignature -Mode
   Strict` (the hard PASS/FAIL condition) — this alone proves trust-chain validity end-to-end,
   which is the actual security property that matters.
2. **Capture** the issuer string (via a second, direct `Get-AuthenticodeSignature` call — see
   Pattern 2's gap note) into the verdict's `detail` field for operator visibility/audit —
   **informational only**, never a pass/fail branch.
3. Explicitly flag in the gate's header comment (mirroring the style of
   `deploy-silent-install.ps1`'s threat-model comments) that issuer-substring matching was
   evaluated and rejected as a gating mechanism, citing `101-SIGN03-SMOKE-VERDICT.md`, so a
   future maintainer does not "fix" this by re-adding the disproven check.
**Warning signs:** A gate that FAILs a real, humanly-confirmed-valid Trusted Signing artifact
because its issuer string doesn't match an expected substring is very likely resurrecting this
exact disproven heuristic.

### Pitfall 4: `nono.exe` under `C:\Program Files\nono\` on this dev host makes the SKIP path *doubly* guaranteed — do not treat that as evidence the gate logic is untested
**What goes wrong:** It would be easy to assume "the gate returned SKIP, therefore the
precondition logic is proven correct" — but on this host BOTH the elevation check (non-admin
session) AND the dirty-host check (`nono.exe` present) independently trigger SKIP. A gate that
has a bug in the dirty-host check specifically (e.g. wrong path, wrong `-LiteralPath` usage)
would still SKIP correctly here, purely via the elevation branch, masking the bug.
**Why it happens:** Precondition functions with multiple independent SKIP branches are
inherently harder to fully exercise on a single host state.
**How to avoid:** When authoring the plan/verification for this phase, note that "SC4 passes on
the dev host" is a necessary but not sufficient proof — the plan should also include a manual
code-review check (or a temporary `Assert-Equal` unit-style probe, mirroring
`harness-self-check.ps1`'s idiom) confirming each precondition branch's string/path exactly
matches the intended target, since the dev host's current state cannot exercise the
"elevated-but-dirty" or "non-elevated-but-clean" branches independently.
**Warning signs:** A dirty-host detection path with a typo'd or wrong-cased path (Windows paths
are case-insensitive but a `-LiteralPath` vs unquoted-wildcard difference is NOT forgiving) that
never gets caught because elevation already short-circuits.

## Code Examples

### Live-verified Windows 11 SKU resolution (Gen2 confirmed)
```bash
# Source: live `az` output on this dev host, 2026-07-03 [VERIFIED]
az vm image list-skus --location eastus --publisher MicrosoftWindowsDesktop --offer windows-11 -o table
# Location    Name
# ----------  --------------------
# eastus      win11-24h2-ent
# eastus      win11-24h2-ent-ltsc
# eastus      win11-24h2-entn
# eastus      win11-25h2-ent
# eastus      win11-25h2-entn
# eastus      win11-25h2-pro
# ... (win11-23h2-* also present but superseded — do not hardcode any of these)

az vm image show --location eastus --urn "MicrosoftWindowsDesktop:windows-11:win11-24h2-ent:latest"
# "hyperVGeneration": "V2"
# "features": [ { "name": "SecurityType", "value": "TrustedLaunchAndConfidentialVmSupported" }, ... ]
```
Confirms: any current SKU under this publisher/offer is already Gen2 + Trusted-Launch-capable —
`main.bicep`'s `securityProfile.securityType = 'TrustedLaunch'` +
`uefiSettings.secureBootEnabled/vTpmEnabled = true` will be accepted for whichever SKU the live
query resolves to; no SKU-specific gating logic is needed in the Bicep itself.

### `az bicep build` local validation (zero Azure calls, works after the Pitfall 1 workaround)
```bash
# Source: live test on this dev host, 2026-07-03 [VERIFIED]
az bicep build -f scripts/azure/clean-vm/main.bicep --stdout
# Emits: { "$schema": ".../deploymentTemplate.json#", "resources": [...], ... }
# Exit code 0 = syntactically valid Bicep, ready for `--stdout` inspection or `az deployment
# group validate`/`what-if` (both of which remain Phase 106 concerns, not authored/run here).
```

### Minimal Bicep shape for the VM resource (illustrative — planner should expand)
```bicep
// Illustrative shape — NOT copied from an existing repo file (no prior scripts/azure/ exists).
// D-01: SKU has NO default — forces the deploy script's live `az vm image list-skus` resolution.
@description('Windows 11 SKU, resolved LIVE via az vm image list-skus — never hardcode a default.')
param vmImageSku string

@description('Operator public IP (CIDR, e.g. 1.2.3.4/32) — fetched via curl, NOT az rest (Pitfall 2).')
param operatorIpCidr string

resource vm 'Microsoft.Compute/virtualMachines@2024-07-01' = {
  name: vmName
  location: location
  properties: {
    hardwareProfile: { vmSize: vmSize }
    storageProfile: {
      imageReference: {
        publisher: 'MicrosoftWindowsDesktop'
        offer: 'windows-11'
        sku: vmImageSku
        version: 'latest'
      }
    }
    securityProfile: {
      securityType: 'TrustedLaunch'
      uefiSettings: {
        secureBootEnabled: true
        vTpmEnabled: true
      }
    }
    // networkProfile referencing an NIC bound to an NSG-scoped subnet, defined in the same module
  }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| POC self-signed `CN=nono Test Signing` cert (untrusted on clean hosts) | Azure Trusted Signing, `PublicTrust` profile | Phase 101 (2026-07-02/03), still not fully green (SIGN-03 deferred to Phase 104) | The `trusted-signed-assertion` gate's real PASS is blocked on Phase 104 producing a real signed artifact — this phase only proves the gate SKIPs correctly today |
| Assumed issuer-naming tell (`Microsoft ID Verified CS` vs `Enterprise ID Verified Policy`) | Disproven — both profile types can chain through `Enterprise ID Verified Policy AOC CA` | 2026-07-02/03 (SIGN-03 live run) | Do not gate on issuer substring (Pitfall 3) |

**Deprecated/outdated:**
- `win11-23h2-*` SKUs: still enumerable but superseded by `24h2`/`25h2` in the same live query —
  confirms the "never hardcode" requirement is not theoretical.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The machine MSI (`dist\windows\nono-machine.msi` default path pattern, matching `clean-host-install.ps1`'s `$MsiPath` default) is the correct artifact for `broker-spawn-on-clean-host.ps1` to install (bundles `nono.exe` + `nono-shell-broker.exe` + WFP service, all needed for `nono run --profile claude-code` to spawn the broker) | Don't Hand-Roll / Recommended Project Structure | If the user MSI is actually the correct target (e.g. if the broker path doesn't need machine-scope elevation), the gate would wrongly require elevation and SKIP when it shouldn't; low risk since `clean-host-install.ps1` and `deploy-silent-install.ps1` both already use the machine MSI for their nono-install proofs, so this is a strong pattern match, not a guess |
| A2 | `az deployment group what-if` is optional/stretch for this phase, and `az bicep build` alone satisfies "author + lint" per the phase's stated AUTHOR+VALIDATE (not live-deploy) scope | Standard Stack / Architecture Patterns | Low risk — the phase objective explicitly says "Research the AUTHOR+VALIDATE path... NOT a live deploy," and `az bicep build` was live-verified to require zero Azure API calls, matching that scope precisely |
| A3 | The three-provider IP-drift observation (Pitfall 2) generalizes beyond this single session — i.e., it is a persistent property of this corporate network, not a one-off transient blip | Common Pitfalls (Pitfall 2) | Medium — if it was a one-off (e.g., a VPN reconnect mid-session), the planner may over-engineer IP-drift handling for a non-recurring issue; recommend the plan treat it as a documented risk with a cheap mitigation (fetch-IP-immediately-before-deploy) rather than heavy engineering, since the downside of being wrong either way is small (worst case: a redeploy of one NSG rule) |

**If this table is empty:** N/A — see entries above.

## Open Questions

1. **Which exact resource-group / networking scope should `main.bicep` target?**
   - What we know: `RG_Nono` exists live [VERIFIED: `az group show -n RG_Nono`] and currently
     contains only `ArtifactNono` (`Microsoft.CodeSigning/codeSigningAccounts`) — no VNet/subnet
     to attach to.
   - What's unclear: whether the operator wants the ephemeral VM's networking to live inside
     `RG_Nono` alongside the signing account, or in its own dedicated ephemeral resource group
     (cleaner teardown — `az group delete` on a VM-only RG is simpler/safer than deleting
     individual resources out of a RG that also holds the permanent signing account).
   - Recommendation: default `main.bicep` to accept a `resourceGroupName`/target-RG as a deploy-
     time concern (the Bicep module itself is RG-agnostic — deployed via `az deployment group
     create -g <rg>`), and have the plan surface this as a discuss-phase-worthy decision: reuse
     `RG_Nono` vs. a fresh ephemeral RG (e.g. `RG_Nono_CleanHost`) created and destroyed per
     lifecycle. A fresh ephemeral RG is lower-risk (a `az group delete` can never accidentally
     remove the signing account) and matches "ephemeral, never persistent" more literally.

2. **Does `az deployment group what-if` also suffer the Pitfall 1 corporate-TLS class of failure?**
   - What we know: `az account show`, `az group show`, `az vm image list-skus`, `az resource
     list` (all `management.azure.com`-scoped calls) succeeded live on this dev host with no TLS
     issue; only `aka.ms` (Bicep release) and `api.ipify.org` (via `az rest`, a generic non-Azure
     REST passthrough) failed.
   - What's unclear: whether `az deployment group what-if` — which also needs the Bicep CLI to
     transpile the template before submitting to `management.azure.com` — would succeed now that
     Bicep is manually installed at `~/.azure/bin/bicep.exe`, since it combines both the
     (now-fixed) Bicep-compile step and a (previously-fine) `management.azure.com` call.
   - Recommendation: not required for this phase's scope (author + lint only); if the planner
     wants extra confidence, a stretch task could attempt `az deployment group what-if` in dry-run
     against a throwaway/no-op parameter set, but this is optional and should not block the phase.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Azure CLI (`az`) | SKU resolution, deploy/teardown scripts (not run this phase) | Yes | 2.87.0 | — |
| Bicep CLI | `az bicep build` (local validation, run this phase) | Yes, after manual workaround | 0.44.1 | Manual `curl` download + place at `~/.azure/bin/bicep.exe` (Pitfall 1) — `az bicep install` itself fails |
| PowerShell (`pwsh`) | New gate files, `verify-dark.ps1` | Yes | 7.6.3 | — |
| Live Azure network reachability (`management.azure.com`) | `az account show`/`az vm image list-skus`/`az resource list` (all live-tested successfully) | Yes | — | — |
| Live Azure network reachability (`aka.ms`, generic public internet via `az`'s Python client) | `az bicep install`, `az rest` to arbitrary hosts | No (corporate TLS interception) | — | `curl` (Windows SChannel) succeeds for the same URLs — use `curl`, never `az`'s own HTTP client, for anything outside `management.azure.com`/`login.microsoftonline.com` |
| Administrator/elevated pwsh session | `broker-spawn-on-clean-host.ps1`'s install/uninstall (machine-scope MSI) | No (dev host session is non-elevated) [VERIFIED live] | — | Gate correctly returns `SKIP_HOST_UNAVAILABLE` via elevation check — this IS the expected/required behavior for SC4, not a blocker |
| `nono.exe` NOT present under `C:\Program Files\nono\` (clean-host precondition) | `broker-spawn-on-clean-host.ps1`'s dirty-host detection | No — `nono.exe` IS present [VERIFIED live] | — | Same as above — this independently also produces the required SKIP |

**Missing dependencies with no fallback:** None — every gap has a documented, live-verified fallback.

**Missing dependencies with fallback:**
- Bicep CLI auto-install path (`az bicep install`) — use manual `curl` download instead (Pitfall 1).
- Elevated session / clean host for the broker-spawn gate — both absences are the *correct*,
  expected dev-host state per SC4 (gate should SKIP, not a real capability gap to work around).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None (no pytest/jest equivalent) — PowerShell gate scripts + `az`/`bicep` CLI checks are the test surface |
| Config file | none — see Wave 0 |
| Quick run command | `pwsh -File scripts/verify-dark.ps1 -Gate trusted-signed-assertion` / `-Gate broker-spawn-on-clean-host` |
| Full suite command | `pwsh -File scripts/verify-dark.ps1 -All` (also exercises every pre-existing gate, all of which must remain unaffected — zero harness changes) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CHOST-01 (SC1) | `main.bicep` compiles/lints cleanly, Gen2+Trusted-Launch+live-SKU-param shape present | static/lint | `az bicep build -f scripts/azure/clean-vm/main.bicep --stdout` (exit 0) | ❌ Wave 0 — file does not exist yet |
| CHOST-01 (SC1) | Deploy/teardown scripts exist and document the ephemeral lifecycle (never persistent) | manual/code-review | n/a (doc/script review, no live invocation this phase) | ❌ Wave 0 |
| CHOST-02 (SC2) | `trusted-signed-assertion.ps1` exists, dot-sources `verify-authenticode.ps1`, plugs into harness with zero code changes | integration | `pwsh -File scripts/verify-dark.ps1 -Gate trusted-signed-assertion` → exit 3 (`SKIP_HOST_UNAVAILABLE`) on this dev host | ❌ Wave 0 |
| CHOST-02 (SC3) | `broker-spawn-on-clean-host.ps1` exists and is self-contained (not order-dependent on `-All`'s alphabetical dispatch) | integration + code-review | `pwsh -File scripts/verify-dark.ps1 -Gate broker-spawn-on-clean-host` → exit 3 (`SKIP_HOST_UNAVAILABLE`); code-review confirms no dependency on another gate having run first | ❌ Wave 0 |
| CHOST-02 (SC4) | Both new gates correctly SKIP on the dev host | integration | `pwsh -File scripts/verify-dark.ps1 -All` → both gates present in `gates[]` array with `verdict: SKIP_HOST_UNAVAILABLE`, `overall: PASS_WITH_SKIPS` (assuming no other gate FAILs) | ❌ Wave 0 |
| (regression) harness stays unmodified | `verify-dark.ps1` byte-for-byte unchanged | static | `git diff --stat scripts/verify-dark.ps1` → empty | N/A (pre-existing file) |

### Sampling Rate
- **Per task commit:** `pwsh -File scripts/verify-dark.ps1 -Gate <new-gate-name>` (single-gate,
  fast) + `az bicep build -f scripts/azure/clean-vm/main.bicep --stdout` (fast, local, no network
  round-trip beyond the one-time Bicep binary).
- **Per wave merge:** `pwsh -File scripts/verify-dark.ps1 -All` (full sweep, confirms no
  pre-existing gate regressed and both new gates SKIP correctly).
- **Phase gate:** `az bicep build` exit 0 + `-All` sweep `overall` is `PASS` or
  `PASS_WITH_SKIPS` (never `FAIL`/`HARNESS_ERROR`) + `git diff --stat scripts/verify-dark.ps1`
  empty, before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `scripts/azure/clean-vm/main.bicep` — does not exist yet (covers CHOST-01)
- [ ] `scripts/azure/clean-vm/deploy.ps1` / `teardown.ps1` — do not exist yet (covers CHOST-01)
- [ ] `scripts/gates/trusted-signed-assertion.ps1` — does not exist yet (covers CHOST-02 SC2)
- [ ] `scripts/gates/broker-spawn-on-clean-host.ps1` — does not exist yet (covers CHOST-02 SC3)
- [ ] Bicep CLI install on the dev host — currently absent; the manual-download workaround
  (Pitfall 1) must be run once before `az bicep build` can be used as a verification step; this is
  environment setup, not code, but should be called out explicitly in the plan's first task so it
  isn't silently assumed

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V1 Architecture | Yes | Ephemeral-by-design IaC (create→use→teardown), least-privilege NSG (deny-all except operator-IP-scoped RDP) — no persistent attack surface |
| V4 Access Control | Yes | NSG explicitly scoped to a single operator IP/CIDR for RDP (3389); no broad `0.0.0.0/0` rule ever authored |
| V6 Cryptography | Yes (indirectly) | Trusted-Launch's Secure Boot + vTPM are the relevant Azure-managed crypto/attestation controls for the VM itself; the `trusted-signed-assertion` gate reuses (never reimplements) the Phase 101 dual-engine Authenticode verify — no hand-rolled crypto |
| V9 Communication | Yes | All live `az`/`curl` calls in the deploy/teardown scripts must fail closed on TLS verification errors (never `-k`/`--insecure`/`az --debug`-suppress a cert error) — the Pitfall 1/2 workarounds are about *routing around* a broken client library, never about disabling certificate validation |
| V14 Configuration | Yes | SKU/region/operator-IP are all required parameters with no defaults (Don't-Hand-Roll table + Anti-Patterns) — no silently-stale hardcoded config |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Overly broad NSG rule (RDP open to `0.0.0.0/0`) | Elevation of Privilege / Information Disclosure | NSG source scoped to a single `/32` (or narrowly-scoped CIDR) operator IP, resolved fresh at deploy time, never a wildcard |
| Persistent VM re-contaminating the "clean host" property across runs | Tampering (of the test fixture itself) | Documented ephemeral create→use→teardown lifecycle; `main.bicep`/deploy script never used to stand up a long-lived VM (explicitly out of scope per CLAUDE.md v3.5 constraint) |
| Disabling TLS verification to work around the corporate-TLS Pitfall 1/2 failures | Tampering / Spoofing (MITM) | Never use `-k`/`--insecure`/cert-bypass flags to route around the SSL errors seen in this research — the correct fix is using `curl` (which already validates against the legitimately-installed corporate root) or the manual-binary-placement workaround, both of which preserve full certificate validation |
| A gate silently PASSing when it should SKIP (e.g., precondition logic bug) | Spoofing (false confidence in an unproven claim) | Mirror the existing `Test-Precondition`/`Invoke-Gate` throw-vs-return discipline exactly (Pattern 1/2); a thrown exception must never be swallowed into a false PASS |

## Sources

### Primary (HIGH confidence — live-verified on this dev host, 2026-07-03)
- `az --version` → azure-cli 2.87.0
- `az bicep install` (failed) / manual `curl` download + `~/.azure/bin/bicep.exe` placement
  (succeeded) → Bicep CLI 0.44.1
- `az bicep build -f <test.bicep> --stdout` → valid ARM JSON emitted, exit 0
- `az vm image list-skus --location eastus --publisher MicrosoftWindowsDesktop --offer windows-11`
  → live SKU list (`win11-24h2-*`, `win11-25h2-*`, superseded `win11-23h2-*`)
- `az vm image show --urn MicrosoftWindowsDesktop:windows-11:win11-24h2-ent:latest` →
  `hyperVGeneration: V2`, `SecurityType: TrustedLaunchAndConfidentialVmSupported`
- `az group show -n RG_Nono` / `az resource list -g RG_Nono` → RG exists, contains only
  `ArtifactNono` (code-signing account)
- `az account show` → subscription "TWG Architecture POCs", tenant confirmed
- `az rest --method get --url "https://api.ipify.org?format=json"` → failed (SSL cert error,
  same class as Bicep install)
- `curl https://ifconfig.me` / `curl https://api.ipify.org` (three separate calls) → three
  different IPs returned
- `pwsh -Command '$PSVersionTable.PSVersion'` → 7.6.3
- `pwsh` elevation check (`WindowsPrincipal.IsInRole(Administrator)`) → `False` on this session
- `Test-Path 'C:\Program Files\nono\nono.exe'` → `True` (nono already installed on this dev host)
- Full read of `scripts/verify-dark.ps1` (harness contract, auto-discovery, exit-code mapping)
- Full read of `scripts/gates/clean-host-install.ps1`, `deploy-silent-install.ps1`,
  `wfp-egress-isolation.ps1`, `telemetry-event-emit.ps1`, `harness-self-check.ps1`
- Full read of `scripts/verify-authenticode.ps1` (`Assert-TrustedSignature` and all helpers)
- `grep "claude-code" crates/nono-cli/data/policy.json` → confirms `claude-code` is a real
  built-in profile
- `.github/workflows/release.yml` grep for `artifact_staging`/MSI naming conventions

### Secondary (MEDIUM confidence — cited from prior phase artifacts in this repo)
- `.planning/phases/101-verify-gate-hardening-azure-profile-confirmation/101-SIGN03-SMOKE-VERDICT.md`
  — Finding (B) issuer-naming disproof (Pitfall 3), live smoke-run evidence from GitHub Actions
- `.planning/quick/260630-trusted-signing-golive/AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md` —
  Gate 3 clean-host UAT acceptance criteria, first-live-run field note (2026-06-30)
- `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` — CHOST-01/CHOST-02 locked wording, Phase
  103 success criteria and dependency on Phase 101

### Tertiary (LOW confidence)
- None — this research relied on live tool invocation and repo file reads rather than web search.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — `az`/Bicep versions and behavior directly observed on the dev host
- Architecture (gate contract): HIGH — full source read of the harness and 4 example gates
- Bicep VM shape (Gen2/Trusted-Launch/SKU): HIGH — live-queried against the actual subscription
- Issuer-naming tension: HIGH — sourced from this same repo's own prior-phase live evidence, not
  training-data assumption
- Operator-IP-drift finding: MEDIUM — observed live in one session (see Assumption A3); may not
  generalize to every future session on this network

**Research date:** 2026-07-03
**Valid until:** 30 days (Azure SKU catalog and corporate network/TLS posture can drift; re-verify
`az vm image list-skus` and the Bicep-install workaround before Phase 106's live deploy)
