# Architecture Research

**Domain:** CI/CD release-signing pipeline hardening + ephemeral cloud-VM UAT infrastructure (GitHub Actions + Azure Trusted Signing + Azure VM IaC + PowerShell dark-factory gate harness)
**Researched:** 2026-07-02
**Confidence:** HIGH (all integration points cited against the actual `release.yml`, `trusted-signing-smoke.yml`, `scripts/verify-dark.ps1`, and existing `scripts/gates/*.ps1` in this repo; MEDIUM on Azure Bicep/VM specifics, which is new-to-this-repo territory not yet grounded in a live Azure resource)

> **Note:** this file previously held v3.2 (Signed Policy Overrides) research. That milestone
> shipped and archived; this is the v3.5 (Trusted Signing Go-Live) replacement.

## Standard Architecture

### System Overview

```
┌──────────────────────────────── GitHub (existing, MODIFIED) ─────────────────────────────────┐
│  trusted-signing-smoke.yml (workflow_dispatch, windows-latest)                                │
│    azure/login (OIDC) → csc.exe compiles throwaway exe → trusted-signing-action signs          │
│    → [MODIFIED] Verify-Authenticode shared helper (Get-AuthenticodeSignature +                │
│       signtool /pa /v fallback + chain-build/revocation diagnostics)                           │
│    → upload-artifact (signed exe, for offline inspection)                                      │
│                                                                                                  │
│  release.yml (push tag v*.*.*  |  workflow_dispatch)                                            │
│   job:build (5-leg matrix, environment:Development)                                             │
│     … cargo build … Package …                                                                   │
│     azure/login (OIDC) → Sign Windows binaries (pre-package, line ~167)                         │
│     → Package (Windows MSIs, line ~186) → Sign Windows MSIs (line ~241)                         │
│     → [MODIFIED] Verify Authenticode signatures (~line 259, fail-closed D-13)                   │
│     → Verify MSI payload signatures (admin-extract, ~line 281)                                  │
│     → zip / upload-artifact                                                                     │
│   job:release (needs build) → GitHub Release + checksums                                        │
│   job:publish-crates (needs release) → crates.io (nono, nono-proxy, nono-cli)                   │
│   job:update-homebrew-core (needs release)                                                      │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
                                          │  published artifacts (MSI, exe, checksums)
                                          ▼
┌────────────────────── Azure (NEW) ──────────────────────┐   ┌────── Operator (manual, keeps) ──┐
│  clean-vm/ IaC module (Bicep + deploy script)            │   │  crates.io publish (already CI'd) │
│    - fresh Win11 VM, no POC cert, no VC++ redist          │   │  PyPI: cd nono-py; maturin publish│
│    - NSG allowing RDP from operator IP only               │   │  npm:  cd nono-ts; npm publish     │
│    - tags: ephemeral=true, purpose=nono-clean-host-uat     │   └───────────────────────────────────┘
│    - teardown script (destroy after UAT)                  │
└──────────────────────────┬────────────────────────────────┘
                            │ RDP (operator session)
                            ▼
┌───────────────────── scripts/verify-dark.ps1 harness (existing, gate-discovery UNCHANGED) ─────┐
│  Gate auto-discovery scans scripts/gates/*.ps1 — NEW gate files just need to exist there:        │
│    - clean-host-install.ps1        [REUSED unmodified — MSI-clean-install]                       │
│    - trusted-signed-assertion.ps1  [NEW — Issuer-chain assertion on shipped artifacts]           │
│    - broker-spawn-on-clean-host.ps1[NEW — nono run --profile claude-code spawns, no cert import] │
│  Test-Precondition → Invoke-Gate → {gate,verdict,reason,detail,timestamp} → persist → emit → exit│
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Status this milestone |
|-----------|----------------|------------------------|
| `release.yml` "Verify Authenticode signatures" step (~line 259) | Fail-closed gate: `Get-AuthenticodeSignature -ne 'Valid'` aborts before upload | **MODIFIED** — add `signtool /pa /v` fallback + diagnostics, preserve fail-closed semantics |
| `release.yml` "Verify MSI payload signatures" step (~line 281-321) | Admin-extract, per-payload-exe signature check | **MODIFIED** — same hardening applied to the extracted-payload loop |
| `trusted-signing-smoke.yml` "Verify the embedded signature" step (~line 62-73) | Non-destructive proof the D-02 signing chain is live, publishes nothing | **MODIFIED** first (cheapest place to prove the fix), **DELETED** at close-out |
| New shared verify helper (e.g. `scripts/verify-authenticode.ps1`) | One PowerShell function/script implementing the hardened check, called from both workflows AND the new `trusted-signed-assertion` gate | **NEW** — avoids drift between 3 call sites |
| Azure clean-VM IaC module (`scripts/azure/clean-vm/`) | Stand up/tear down a fresh Win11 VM with no POC cert / no VC++ | **NEW** |
| `scripts/gates/clean-host-install.ps1` | MSI install proof on a clean host (INST-01) | **REUSED unmodified** — was previously always `SKIP_HOST_UNAVAILABLE` on the dev host; now runs for-real on the Azure VM |
| `scripts/gates/trusted-signed-assertion.ps1` | Assert Issuer chains to `Microsoft ID Verified CS` root for `nono.exe`, broker, and both MSIs | **NEW** |
| `scripts/gates/broker-spawn-on-clean-host.ps1` | `nono run --profile claude-code -- <cmd>` spawns the broker with zero manual cert-trust step | **NEW** |
| `scripts/verify-dark.ps1` | Gate auto-discovery, `Test-Precondition`→`Invoke-Gate` contract, verdict persistence/emit, exit-code mapping | **UNCHANGED** — new gates plug in purely by filename convention (D-04 auto-discovery, confirmed lines 133-145) |
| `RELEASE-RUNBOOK.md` (currently under `.planning/milestones/v3.3-phases/97-.../`) | Operator step-by-step for the push sequence | **MODIFIED** — Step 3 push sequence flips from "prepare-only, not executed" to actually executed; add VM-UAT + close-out steps |
| GitHub repo secrets `WINDOWS_SIGNING_CERT` / `_PASSWORD` (POC self-signed cert, legacy pre-Trusted-Signing) | Old fallback signing path, no longer referenced by current `release.yml` (confirmed: no `WINDOWS_SIGNING_CERT` reference anywhere in the file as read) | **RETIRE** (delete the GitHub secrets) at close-out — code already doesn't use them; this is pure secret hygiene |
| `docs/cli/development/windows-signing-guide.mdx` (~lines 244-251) | Developer-facing signing docs | **MODIFIED** at close-out — still describes the retired PFX flow |

## Recommended Project Structure

```
.github/workflows/
├── release.yml                          # MODIFIED: harden ~line 259 + ~line 281 verify steps
└── trusted-signing-smoke.yml             # MODIFIED then DELETED (close-out)

scripts/
├── verify-authenticode.ps1               # NEW: shared hardened-verify function/script
│                                          #   (Get-AuthenticodeSignature + signtool /pa /v
│                                          #    fallback + UnknownError-vs-UntrustedRoot
│                                          #    diagnostic classification). Dot-sourced by
│                                          #   release.yml steps, trusted-signing-smoke.yml,
│                                          #   AND scripts/gates/trusted-signed-assertion.ps1.
├── azure/
│   └── clean-vm/
│       ├── main.bicep                    # NEW: Win11 VM + NSG (RDP from operator IP only)
│       │                                  #   + no-cert/no-VC++ baseline image
│       ├── deploy-clean-vm.ps1            # NEW: az deployment wrapper, prints RDP connection
│       │                                  #   info, tags resource group ephemeral=true
│       └── teardown-clean-vm.ps1          # NEW: az group delete, confirms before destroying
└── gates/
    ├── clean-host-install.ps1            # REUSED unmodified — now actually exercised (not SKIP)
    ├── trusted-signed-assertion.ps1      # NEW gate: Issuer-chain check via verify-authenticode.ps1
    └── broker-spawn-on-clean-host.ps1    # NEW gate: nono run --profile claude-code spawn proof

.planning/todos/
├── pending/20260611-poc-cert-broker-clean-host.md   # drained by broker-spawn-on-clean-host PASS
└── pending/20260611-msi-vcredist-prereq.md          # drained by clean-host-install PASS on the VM

RELEASE-RUNBOOK.md (or successor at repo root)         # MODIFIED: prepare-only → executed, +VM UAT
docs/cli/development/windows-signing-guide.mdx         # MODIFIED at close-out (gitignored-but-tracked,
                                                        #   needs `git add -f`)
```

### Structure Rationale

- **`scripts/verify-authenticode.ps1` as a shared helper, not copy-pasted 3x:** `release.yml` verifies signatures in two places (loose-binary loop ~line 259-274, MSI-admin-extract-payload loop ~line 281-321) and `trusted-signing-smoke.yml` verifies once (~line 62-73). All three currently do an inline `Get-AuthenticodeSignature -ne 'Valid'` check with slightly different loop shapes. Introducing a single dot-sourced function with a fixed `{status, issuer, chainBuildOk, revocationOk, diagnosis}` return shape means the `UnknownError`-diagnosis logic is written once and the new `trusted-signed-assertion.ps1` verify-dark gate can reuse the exact same classification on a downloaded release artifact — no drift between "does CI think it's signed" and "does the operator's dark gate think it's signed."
- **`scripts/azure/clean-vm/` mirrors no existing pattern (new territory)** — there is no existing `scripts/azure/` directory in this repo; Bicep is the natural IaC choice given the workflows already use `az`/OIDC (azure/login) elsewhere, keeping the tooling surface Azure-CLI-centric rather than introducing Terraform.
- **New gates live in `scripts/gates/` alongside existing ones, not a separate directory** — `verify-dark.ps1`'s auto-discovery (D-04, `Get-ChildItem -Path $gatesDir -Filter "*.ps1"`) is directory-scoped and name-agnostic; no harness code changes are needed to add a gate, only a new file matching the `Test-Precondition`/`Invoke-Gate` contract (confirmed against `harness-self-check.ps1`/`clean-host-install.ps1`/`release-readiness.ps1`, all following the identical two-function shape).
- **One gate per concern, matching the existing 10-gate convention** — `clean-host-install.ps1`, `release-readiness.ps1`, `wfp-egress-isolation.ps1`, `override-01/02.ps1`, `telemetry-event-emit.ps1`, `egress-policy-deny.ps1`, `copilot-e2e.ps1`, `deploy-silent-install.ps1` are each single-purpose. `trusted-signed-assertion` and `broker-spawn-on-clean-host` follow that shape rather than being bolted onto `clean-host-install.ps1` as extra steps — this also lets each gate SKIP/PASS/FAIL independently and gives 3 distinct verdict JSON files instead of one overloaded one, which matches the cookbook's Gate 3 checklist (3 distinct assertions: signature Valid+Issuer, install succeeds, broker spawns).

## Architectural Patterns

### Pattern 1: Fail-closed verify with graduated fallback (not "or")

**What:** The existing `release.yml` verify step is a hard boolean gate (`Status -ne 'Valid'` → `exit 1`, D-13). Hardening must ADD a `signtool verify /pa /v` fallback *without* weakening that boolean — i.e., the fallback is a second, independent attempt to reach `Valid`, and the overall step is still fail-closed if BOTH checks disagree-with-Valid. Do not turn this into "pass if either check says Valid" without also asserting the two checks agree on WHY the other failed (chain-build vs genuine untrusted-root) — that distinction is the entire point of the hardening (cookbook §5 field note: `UnknownError` ≠ `UntrustedRoot`).
**When to use:** Any place `release.yml`/`trusted-signing-smoke.yml` currently does `Get-AuthenticodeSignature -ne 'Valid'` → `exit 1` (3 call sites: ~259, ~281-321 loop, smoke ~69-72).
**Trade-offs:** More CI runtime (chain-build + revocation checks take longer, especially with a CRL/OCSP round-trip); worth it because a silent regression back to "sign succeeds but verify never confirms it" reproduces the exact `UnknownError` blocker this milestone exists to close.

**Example (shape, not literal code):**
```powershell
function Test-TrustedSignedArtifact {
    param([string]$Path)
    $psSig = Get-AuthenticodeSignature -FilePath $Path
    if ($psSig.Status -eq 'Valid') {
        return @{ valid = $true; method = 'Get-AuthenticodeSignature'; issuer = $psSig.SignerCertificate.Issuer }
    }
    # Fallback: signtool's /pa (default Authenticode policy) chain-builds independently
    # of PowerShell's WinVerifyTrust wrapper and surfaces distinct exit codes/verbose text
    # for "chain could not be built" vs "explicit distrust".
    $stError = Join-Path $env:TEMP "signtool-verify-$(Split-Path -Leaf $Path).log"
    & signtool.exe verify /pa /v $Path *> $stError
    $signtoolOk = ($LASTEXITCODE -eq 0)
    $diagnosis = if ($signtoolOk) { 'chain-build-or-revocation-transient (resolved by signtool)' }
                 elseif ($psSig.Status -eq 'UnknownError') { 'chain-build-or-revocation-failure (unresolved)' }
                 else { 'genuine-untrusted-or-invalid' }
    return @{ valid = $signtoolOk; method = 'signtool /pa /v (fallback)'; diagnosis = $diagnosis; log = $stError }
}
```

### Pattern 2: Dark-factory gate is the single UAT contract — Azure VM is just a host, not a new harness

**What:** `verify-dark.ps1` already generalizes "run this assertion, emit a typed verdict, persist before emit, map to a reserved exit code" across every prior host-gated UAT (clean-host install, WFP egress, override live-check, telemetry emission). The Azure VM does NOT need its own bespoke verification framework — it needs (a) the IaC to exist, (b) the operator to RDP in, stage artifacts, and run `pwsh -File scripts/verify-dark.ps1 -Gate <name>` (or `-All`) exactly as documented for every prior host-gated gate, and (c) the resulting `.nono-runtime/verdicts/*.json` to be the same machine-readable proof format already used to close every prior todo.
**When to use:** Any new "prove it on a real host" requirement in this codebase, not just this milestone.
**Trade-offs:** None — this is a straight reuse. The only new work is writing 2 new gate bodies + 1 IaC module; zero harness changes required.

### Pattern 3: Version-family / release-readiness style static gate for the VM's clean-host precondition

**What:** `Test-Precondition` in `clean-host-install.ps1` already fails-open-to-SKIP (not FAIL) when the host is dirty (elevation missing, `nono.exe` already under Program Files, services already registered, MSI not staged) — see lines 65-98. The two new gates should follow this exact shape: `trusted-signed-assertion.ps1`'s precondition is "artifact files exist at the expected staged paths"; `broker-spawn-on-clean-host.ps1`'s precondition is "nono is currently installed" (i.e., it should be run AFTER `clean-host-install` in the same `-All` sweep, or accept a pre-staged binary path parameter) plus elevation if it needs to install first itself.
**When to use:** Designing the two new gates.
**Trade-offs:** Decide explicitly whether `broker-spawn-on-clean-host` assumes a prior successful `clean-host-install` run (cheaper, but creates an inter-gate ordering dependency the aggregator doesn't enforce — `-All` runs gates in `Sort-Object Name` alphabetical order, so `broker-spawn-on-clean-host` < `clean-host-install` < `trusted-signed-assertion` alphabetically, meaning **broker-spawn would run BEFORE install** in an unmodified `-All` sweep) or is self-contained (installs, tests spawn, uninstalls — more code, but order-independent and matches how `clean-host-install.ps1` itself is already fully self-contained). **Recommend self-contained** given the alphabetical-ordering gotcha just identified — do not rely on gate execution order.

## Data Flow

### Verify-gate hardening data flow (CI, autonomous)

```
azure/trusted-signing-action (sign step)
    ↓ (signed .exe / .msi in files-folder)
[NEW] Test-TrustedSignedArtifact (shared helper)
    ↓
  Get-AuthenticodeSignature  →  Valid? ──yes──→ PASS, upload proceeds
       │ no (incl. UnknownError)
       ▼
  signtool verify /pa /v      →  exit 0? ──yes──→ PASS (chain-build/revocation transient resolved), upload proceeds, log which check saved it
       │ no
       ▼
  classify: UnknownError-without-signtool-fix → "chain-build/revocation failure, needs investigation"
            explicit distrust/InvalidSignature → "genuine untrusted/invalid, STOP"
    ↓
  exit 1 (fail-closed preserved either way — D-13 unchanged)
```

### VM clean-host UAT data flow (operator-in-loop)

```
[NEW] scripts/azure/clean-vm/deploy-clean-vm.ps1  →  fresh Win11 VM (Azure)
    ↓ (RDP, operator)
Operator downloads published nono-v0.66.1-*.msi + nono.exe from GitHub Release
    ↓ (stage at expected paths)
pwsh -File scripts/verify-dark.ps1 -All
    ↓ (gate auto-discovery, alphabetical)
  broker-spawn-on-clean-host  [self-contained: install → nono run --profile claude-code → assert broker spawns → uninstall]
  clean-host-install          [REUSED: msiexec /i → nono --version → uninstall]
  ... (existing gates SKIP_HOST_UNAVAILABLE where not applicable to this VM, e.g. override-01/02) ...
  trusted-signed-assertion    [NEW: Issuer-chain check on the staged exe/msi]
    ↓
  {gates:[...], overall: PASS | PASS_WITH_SKIPS | FAIL | HARNESS_ERROR}
    ↓ persisted to .nono-runtime/verdicts/_aggregate.json (WR-04: before stdout emit)
Operator inspects verdict → if all 3 new-relevant gates PASS → Gate 3 satisfied → close-out
```

### Key Data Flows

1. **UnknownError resolution proof chain:** smoke workflow (cheap, no publish) → hardened verify passes on GitHub's `windows-latest` runner → confidence the SAME hardened verify will pass in `release.yml`'s build job → tag push → real release. This ordering (smoke before tag) is already mandated by the cookbook ("If this fails, STOP and fix it here — do not cut a release until smoke is green") and must be preserved as the phase-1→phase-2 gate.
2. **Verdict-file-of-record before stdout (WR-04):** every new gate MUST persist its JSON to `.nono-runtime/verdicts/<gate>.json` before writing to stdout — this is enforced by the harness itself (`Persist-Verdict` called before `[Console]::Out.Write`), not by the gate author, so no new discipline is required, just conformance to the `[ordered]@{gate;verdict;reason;detail;timestamp}` return contract.
3. **Registry publish ordering (unchanged, reused):** `publish-crates` job already encodes `nono` → (30s sleep) → `nono-proxy` → (30s sleep) → `nono-cli` dependency order; PyPI/npm remain manual operator steps per `RELEASE-RUNBOOK.md` Steps 5-6 — this milestone does not change that mechanism, only flips it from documented-but-unexecuted to executed.

## Scaling Considerations

Not applicable in the traditional sense (this is release infrastructure, not a live service). The relevant "scale" axis is **frequency of re-run**:

| Concern | Single go-live (this milestone) | Ongoing (future releases) | Fleet-wide (many concurrent VMs) |
|---------|----------------------------------|----------------------------|-----------------------------------|
| Azure VM cost | One VM, RDP session, teardown after UAT | Same pattern reused per release if VM is destroyed each time — consider a reusable-but-resettable VM (snapshot + revert) if UAT cadence increases, to save the ~10-15 min VM provisioning time per release | Not needed; this is a manual gate, not a fleet |
| Smoke workflow | Deleted at close-out per its own header | N/A — no longer exists | N/A |
| verify-dark gate count | +2 new gates (12 total) | Harness already scales to N gates via auto-discovery; no redesign needed | N/A |
| Trusted Signing quota/rate limits | Not researched — MEDIUM confidence gap; Azure Trusted Signing has account-level signing-operation quotas that could matter if release cadence increases sharply | Flag for future research if release frequency grows | N/A |

### Scaling Priorities

1. **First real friction point:** repeated VM provisioning cost/time if clean-host UAT becomes a per-release gate rather than a one-time go-live proof. Mitigate by scripting VM snapshot+revert (`teardown-clean-vm.ps1` could instead be `reset-clean-vm.ps1` that restores from a "never touched by nono" snapshot) — defer until UAT cadence is known.
2. **Second friction point:** if `signtool verify /pa /v` revocation checks (CRL/OCSP round-trips) add meaningful CI wall-clock time across 1 leg × N releases, consider caching a positive verify result keyed by artifact hash for a short TTL — not needed at go-live-once scale.

## Anti-Patterns

### Anti-Pattern 1: Weakening the fail-closed verify to "unblock the release"

**What people do:** Under go-live time pressure, change `Status -ne 'Valid'` to something permissive (e.g., accept `UnknownError` as passing, or wrap the check in `-ErrorAction SilentlyContinue` and ignore the result) to get past the `UnknownError` blocker.
**Why it's wrong:** This is exactly D-13's threat model — it silently ships a possibly-untrusted binary. `UnknownError` genuinely can mean "the AOC intermediate is absent and Windows will show 'Windows protected your PC'" — the whole point of this milestone is to resolve WHY it's UnknownError (chain-build/revocation vs profile-type), not to stop checking.
**Do this instead:** Add the `signtool /pa /v` fallback as a second independent attempt to reach a definitive `Valid`; if both checks disagree with Valid, still fail-closed and surface the diagnosis (chain-build vs untrusted) so the next debugging step is obvious from the CI log.

### Anti-Pattern 2: Testing "clean host" behavior on the operator's corporate dev host

**What people do:** Run the new `broker-spawn-on-clean-host` / `trusted-signed-assertion` gates on the same Windows box used for development, reasoning "it's basically clean now that the POC cert path is gone."
**Why it's wrong:** Per `PROJECT.md`'s explicit milestone context, the corporate dev host previously had the POC cert imported, has VC++ preinstalled, and sits behind corporate proxy/EDR/managed-trust-store — any of which can mask the exact chain-build/revocation failure being debugged (a managed trust store might already carry the missing AOC intermediate that GitHub's `windows-latest` runner lacks). This is precisely why the milestone specifies an Azure VM.
**Do this instead:** Only trust a PASS from a freshly-provisioned VM (or GitHub's `windows-latest` runner for the CI-side smoke/verify, which is also genuinely clean per-run) as evidence Gate 3 (cookbook) is satisfied. The existing `clean-host-install.ps1` precondition check (lines 78-89: detects `nono.exe` already under Program Files or nono services already registered) already encodes this discipline — reuse it, don't bypass it.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Azure Trusted Signing | `azure/login@v2` (OIDC, `id-token: write`) + `azure/trusted-signing-action@v0`, already wired in both workflows | Federated credential subject MUST be exactly `repo:OscarMackJr/nono:environment:Development` (cookbook §0, verified against `release.yml:29-32` and `trusted-signing-smoke.yml:20-21`) — do not touch this; the bug is downstream in verify, not in the sign/auth steps, per the cookbook's own field note. |
| Azure VM (new) | `az deployment group create` with a Bicep template + `az vm run-command` or plain RDP for the operator session | No existing az/Bicep tooling in this repo (`scripts/azure/` does not yet exist) — this is genuinely new infrastructure, not an extension of an existing pattern. Confirm subscription quota for a Win11 Azure Marketplace image before scripting. |
| crates.io / PyPI / npm | Unchanged from v3.4 prepare-only state — `publish-crates` job (CI-automated) + `RELEASE-RUNBOOK.md` Steps 5-6 (operator manual, `maturin publish` / `npm publish`) | This milestone flips these from "documented, not run" to "actually run" — no code changes to the mechanism itself. |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `release.yml` verify step ↔ new shared `verify-authenticode.ps1` | PowerShell dot-source or direct script invocation from the workflow YAML `run:` block | Keep the fail-closed contract at the CALL SITE (the workflow step still does `if (-not $result.valid) { exit 1 }`), not buried inside the helper, so `-D13` remains auditable by reading the workflow YAML alone. |
| `trusted-signing-smoke.yml` ↔ same shared helper | Same as above | Smoke is the FIRST place this helper gets exercised live (cheapest, non-destructive) — prove it here before it gates the real release. |
| New verify-dark gates ↔ `scripts/verify-dark.ps1` harness | File-presence convention only (`scripts/gates/<name>.ps1`, two exported functions) | Zero harness code changes; confirmed via `Get-ChildItem -Path $gatesDir -Filter "*.ps1"` auto-discovery (verify-dark.ps1 lines 140-145) and the `-All` alphabetical-order gotcha noted in Pattern 3 above. |
| New verify-dark gates ↔ shared `verify-authenticode.ps1` | `trusted-signed-assertion.ps1`'s `Invoke-Gate` dot-sources/calls the same helper the CI workflows use | Ensures "CI thinks it's signed" and "operator's dark gate thinks it's signed" can never silently diverge. |
| Azure VM IaC ↔ verify-dark gates | Purely operational (operator RDPs in, stages artifacts, invokes `pwsh -File scripts/verify-dark.ps1`) — no programmatic coupling | The VM is infrastructure that makes the gates' `Test-Precondition` pass (elevation available, host genuinely clean); it is not itself part of the harness contract. |
| `RELEASE-RUNBOOK.md` ↔ push sequence | Documentation, manually followed by the operator | Needs a new "Step 7 — Azure VM Clean-Host UAT" and "Step 8 — Close-out" appended, mirroring cookbook §7-§8, once Steps 1-6 stop being hypothetical. |

## Suggested Phase / Build Order (feeds gsd-roadmapper)

Ordering respects three hard dependencies: (1) you cannot prove smoke-green without the hardened verify existing, (2) you cannot cut a release before smoke is green (cookbook: "If this fails, STOP... do not cut a release until smoke is green"), (3) you cannot run clean-host UAT before a real trusted-signed release exists to download.

| # | Phase (suggested) | Depends on | Operator checkpoint? | Produces |
|---|--------------------|------------|------------------------|----------|
| 1 | **CI verify-gate hardening** — shared `verify-authenticode.ps1` helper + wire into `release.yml` (~259, ~281-321) and `trusted-signing-smoke.yml` (~62-73); diagnostics for UnknownError-vs-untrusted-root | none (autonomous, code-only) | No | Modified `release.yml`, `trusted-signing-smoke.yml`, new `scripts/verify-authenticode.ps1` |
| 2 | **Azure VM clean-host IaC** — Bicep module + deploy/teardown scripts | none (parallelizable with phase 1) | Partial — Azure subscription access/cost approval likely needed before first real deploy | New `scripts/azure/clean-vm/` |
| 3 | **New verify-dark gates** — `trusted-signed-assertion.ps1`, `broker-spawn-on-clean-host.ps1` (script authoring; will `SKIP_HOST_UNAVAILABLE` on the dev host, same as `clean-host-install.ps1` does today) | Phase 1 (reuses its helper) | No | New gate files under `scripts/gates/` |
| 4 | **Operator: Azure Public Trust profile config** — confirm/create Public Trust certificate profile, correct `TRUSTED_SIGNING_PROFILE`, verify FIC subject (already correct per cookbook §0) | Phase 1 (need the hardened workflow to test against) | **YES — operator-only, Azure portal/CLI RBAC-admin action** | Corrected Azure Trusted Signing config |
| 5 | **Smoke green** — run hardened `trusted-signing-smoke.yml`, confirm `Status: Valid` + Issuer chains to `Microsoft ID Verified CS` | Phases 1 + 4 | Operator triggers the run; may need to iterate with dev on diagnostics output | Proof the D-02 chain is genuinely live |
| 6 | **Cut the release** — operator tags `v0.66.1`, pushes; `release.yml` build job runs hardened verify, fail-closed green | Phase 5 (smoke MUST be green first, per cookbook) | **YES — operator executes `git push origin v0.66.1`** (this milestone's tag push is intentional, unlike v3.1-v3.4) | Trusted-signed GitHub Release with MSI/exe/checksums |
| 7 | **Multi-registry live publish (FUT-01)** — `publish-crates` job auto-runs on release; PyPI (`maturin publish`) + npm (`npm publish`) are operator-manual per `RELEASE-RUNBOOK.md` Steps 5-6 | Phase 6 | **YES — PyPI/npm are manual operator commands** | Live packages on crates.io, PyPI, npm |
| 8 | **Azure VM clean-host UAT (FUT-03 drain)** — deploy VM (phase 2 IaC), operator RDPs in, stages the phase-6/7 published artifacts, runs phase-3 gates (+ reused `clean-host-install.ps1`) via `verify-dark.ps1 -All` | Phases 2, 3, 6 (needs real published artifacts) | **YES — operator RDP session, stages files, runs the harness** | `.nono-runtime/verdicts/*.json` PASS evidence; drains both `poc-cert-broker-clean-host` and `msi-vcredist-prereq` todos |
| 9 | **Close-out** — retire POC secrets (`WINDOWS_SIGNING_CERT`/`_PASSWORD` in GitHub repo settings), `git rm .github/workflows/trusted-signing-smoke.yml`, fix `docs/cli/development/windows-signing-guide.mdx` (needs `git add -f`, gitignored-but-tracked), move both todos `pending/` → `resolved/`, update `STATE.md`/`PROJECT.md` | Phase 8 PASS | **YES — operator deletes GitHub secrets** (code-only parts can be autonomous) | Clean, closed-out repository state; DIST-SIGN-01 arc closed |

**Phases that can run in parallel:** 1 and 2 (independent code/infra work); phase 3 can start as soon as phase 1's helper shape is stable, even before phase 2's VM exists (gates author against `SKIP_HOST_UNAVAILABLE` the same way `clean-host-install.ps1` already does).

**Hard sequential dependency chain:** 1 → 4 → 5 → 6 → 7/8 → 9 (this is the actual go-live critical path; 2 and 3 feed into 8 but don't block 4-7).

**Operator-in-loop checkpoints, summarized:** 4 (Azure profile config — cannot be automated, requires Azure RBAC-admin), 6 (the tag push itself — deliberate, unlike every prior prepare-only milestone), 7 (PyPI/npm publish commands — no CI automation exists for these two registries per current `RELEASE-RUNBOOK.md`), 8 (physically RDP into the VM and run the harness), 9 (delete GitHub repo secrets — a console action, not a code change). Phases 1, 2 (authoring), and 3 (authoring) are autonomous engineering work with no operator checkpoint.

## Sources

- `.github/workflows/release.yml` (this repo, read in full — line numbers cited throughout are from this read)
- `.github/workflows/trusted-signing-smoke.yml` (this repo, read in full)
- `scripts/verify-dark.ps1` (this repo, read in full — gate-discovery mechanics at lines 133-145, `-All` alphabetical ordering at line 276)
- `scripts/gates/clean-host-install.ps1`, `scripts/gates/release-readiness.ps1` (this repo, read in full — reference for the `Test-Precondition`/`Invoke-Gate` contract shape new gates must follow)
- `.planning/quick/260630-trusted-signing-golive/AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md` (this repo — authoritative operator runbook; Gates 1/2/3 + close-out §8 directly inform the phase build order)
- `.planning/todos/pending/20260611-poc-cert-broker-clean-host.md`, `.planning/todos/pending/20260611-msi-vcredist-prereq.md` (this repo — acceptance criteria for what "drained" means)
- `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md` (this repo — current operator push sequence, Steps 1-6, to be extended)
- `.planning/PROJECT.md` (this repo — v3.5 milestone scope, key decisions, "Full go-live EXECUTE posture")
- No live Azure documentation lookup was performed for Bicep VM specifics (WebFetch/WebSearch not used in this pass — all findings above are grounded in the actual repo files). **Gap flagged below.**

---
*Architecture research for: nono v3.5 Trusted Signing Go-Live + First Distributed Release*
*Researched: 2026-07-02*

## Confidence Gaps (flag for phase-specific research)

- **MEDIUM/LOW:** Azure Bicep syntax for a Windows 11 Marketplace VM image + NSG RDP-restriction is not verified against current Azure docs in this pass (no WebFetch/WebSearch performed — this research prioritized grounding every claim in the actual repo files per the quality gate). The phase that authors `scripts/azure/clean-vm/main.bicep` should do a fresh Context7/official-docs pass on `Microsoft.Compute/virtualMachines` + `Microsoft.Network/networkSecurityGroups` Bicep resource schemas before writing the template.
- **LOW:** Whether `signtool.exe` is present by default on GitHub's `windows-latest` runner (it ships with the Windows SDK, which is typically preinstalled on `windows-latest`, but this was not explicitly re-verified against the current runner image manifest in this pass) — the phase implementing Pattern 1 should confirm `Get-Command signtool` resolves before relying on it, and add an explicit SDK-install fallback step if not.
- **LOW:** Azure Trusted Signing account-level rate/quota limits were not researched — flagged only as a scaling consideration, not a go-live blocker (this is a one-time go-live, not a high-frequency release cadence).
