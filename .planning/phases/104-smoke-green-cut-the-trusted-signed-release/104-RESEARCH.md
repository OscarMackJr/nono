# Phase 104: Smoke Green + Cut the Trusted-Signed Release - Research

**Researched:** 2026-07-03
**Domain:** Windows Authenticode / Azure Trusted Signing chain-of-trust diagnostics; GitHub Actions release-workflow mechanics
**Confidence:** HIGH (the central blocking question was resolved empirically, on-machine, in this session — not inferred from CI logs alone)

## Summary

The central research question — **does `signtool verify /pa` pass where `Get-AuthenticodeSignature` (GAS) returns `UnknownError`?** — is now answered definitively: **NO. Both engines fail identically.** This was proven by downloading the actual CI-signed smoke binary (`gh run download 28636133664 -n trusted-signing-smoke-signed`) and running both verification engines against it on this dev host. Both report the exact same underlying cause: `A certificate chain processed, but terminated in a root certificate which is not trusted by the trust provider` (`UntrustedRoot`).

Going one level deeper: a manual `X509Chain` build with `RevocationMode=NoCheck` (bypassing revocation-reachability noise entirely) shows the full 4-element chain and pins the failure precisely on the **root** element: `CN=Microsoft Enterprise ID Root CA 2021` is `UntrustedRoot`. A **fresh, live pull of the actual Windows Update root-certificate list** (`certutil -generateSSTFromWU`, 554 certs, run in this session) confirms this root is **not present** in the current, real-time Microsoft Trusted Root Program CTL that ships to every Windows machine (dev host included) via the standard AuthRoot auto-update mechanism. This is not a `windows-latest`-runner quirk, not a revocation/CRL/OCSP reachability problem, and not a profile-type defect (SIGN-01 already confirmed `PublicTrust`) — it is a **genuine, currently-live gap in Microsoft's root-certificate distribution** for this specific Trusted Signing PKI hierarchy (`Enterprise ID Root CA 2021` → `... Verified Policy Signing PCA 2021` → `... Verified Policy AOC CA 02` → leaf). The leaf's `NotBefore` (2026-07-01) is 2 days old, consistent with a very recent CA-hierarchy rotation whose root has not yet propagated to the public CTL.

A second, independent finding resolves Phase 101's Finding C (the "D-04 diagnostic-flush gap"): GitHub Actions' `pwsh`/`powershell` shell **prepends `$ErrorActionPreference = 'stop'` to every script** (confirmed via the official `actions/runner` ADR 0277, and reproduced locally). `Assert-TrustedSignature`'s Strict-mode failure path calls `Write-Error` (a normally non-terminating cmdlet) *before* `Write-ChainDiagnostic` and `throw` — under CI's forced `Stop` preference, that `Write-Error` call itself becomes terminating and aborts the script **before** `Write-ChainDiagnostic` (the chain dump, signtool `/pa` output, revocation-URL probe) ever runs. This is why the CI log showed only the retry line and a bare error, never the diagnostic dump. Reproduced locally: identical `Write-Error` call, with `$ErrorActionPreference='Stop'` set, skips the following `Write-Host` line every time; with the ambient default (`Continue`) it does not.

**Primary recommendation:** Phase 104 is genuinely hard-blocked by an external, Microsoft-side root-certificate propagation gap that no code change in this repo can fix — the correct response is NOT to weaken the gate, but to (1) ship two small, additive legibility/diagnostic fixes to `verify-authenticode.ps1` (fix the D-04 flush gap; add a `NoCheck`-mode corroborating chain build so future failures are correctly classified as "genuine untrusted root, do not retry" rather than misdiagnosed as "transient"), (2) correct stale issuer-heuristic wording in `REQUIREMENTS.md`/`ROADMAP.md` that Finding B already disproved, (3) fix a **separate, newly-discovered, concrete blocker to SC2** — `release.yml`'s `publish-crates` job still references the pre-rename package names (`-p nono`, `-p nono-proxy`, `-p nono-cli`), which no longer exist in the workspace (Phase 102 renamed them to `nono-sandbox`/`nono-sandbox-proxy`/`nono-sandbox-cli`) — and would fail immediately and turn the Release workflow red on the very tag push this phase exists to cut, and (4) design the operator checkpoint for SC1 as a **recurring poll-until-green** loop (with a cheap local pre-check via `certutil -generateSSTFromWU`, far cheaper than a full CI dispatch), not a single retry.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Authenticode chain-of-trust validation (GAS + signtool) | OS / CryptoAPI (Windows runner or dev host) | CI workflow (invokes it) | Chain building and root-trust decisions are OS-level (CAPI2/X509Chain), not application logic; the CI script only orchestrates and gates on the result. |
| Root-certificate distribution (AuthRoot CTL) | External — Microsoft Trusted Root Program | Windows Update client | Entirely outside this repo's control; the repo can only detect and wait for it, never force it. |
| Trusted Signing (sign operation) | Azure control plane (`azure/trusted-signing-action`, OIDC) | CI workflow (Windows runner) | Signing itself already works (SIGN-01/SIGN-02 proved this); this phase's blocker is purely on the verify side. |
| CI verify-gate legibility (D-04 flush, chain classification) | CI / Backend (PowerShell helper script) | — | Pure script logic; no OS or Azure dependency to fix the flush-order bug. |
| Release artifact assembly + GitHub Release creation | CI workflow (`release.yml` `build`/`release` jobs) | GitHub API (softprops/action-gh-release) | Standard CD pipeline responsibility, already built and green apart from the signing verify blocker. |
| crates.io / Homebrew publish (`publish-crates`, `update-homebrew-core` jobs) | External registry / CI workflow | Phase 105 (PUB-02) | Out of REL-01's scope, but auto-fires on the same tag push and is currently broken (stale package names) — must be neutralized so it does not falsely redden Phase 104's SC2. |
| Tag push / operator authorization | Operator (human) | Git / GitHub | Outward-facing, irreversible-ish action; explicitly gated per this fork's prepare-only/push-operator-gated posture and REQUIREMENTS.md's "operator-in-loop" annotations. |

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REL-01 | First publicly-trusted-signed release cut at `0.66.1` (Gate 2) — tag push runs `Release` workflow to green, all top-level `.exe` + both MSIs Trusted-Signed and pass the hardened fail-closed verify; published artifacts show Verified publisher | This research (a) resolves the *precondition* (SIGN-03 root cause, empirically nailed down — see Primary Investigation), (b) identifies a **second, previously-unknown blocker** to "workflow runs to green" (the stale `publish-crates` job `-p nono`/`-p nono-proxy`/`-p nono-cli` selectors, which no longer resolve post-Phase-102-rename), (c) supplies the exact, minimal, safe code diffs for the two legible-diagnostics fixes to `verify-authenticode.ps1`, and (d) documents the correct, disproven-heuristic-free gate condition for "Verified publisher" (Status=Valid + non-test-cert signer, never an issuer-substring match). |
</phase_requirements>

## User Constraints

No phase-level `104-CONTEXT.md` exists (no `/gsd:discuss-phase 104` run yet). In its absence, the following milestone-level constraints from `.planning/REQUIREMENTS.md` and `.planning/ROADMAP.md` function as locked decisions and MUST be honored identically to a CONTEXT.md:

### Locked Decisions (from REQUIREMENTS.md architecture invariants + ROADMAP Phase 104 section)

- **Never loosen the fail-closed verify.** `Get-AuthenticodeSignature -ne 'Valid'` (and the paired `signtool` exit-code check) must remain byte-for-byte unweakened. Any new logic is additive corroboration/diagnosis only — this is diff-reviewable per SIGN-02's own success criterion and remains true for every change this research recommends.
- **Verify-gate debugging is CI-side** (a clean cloud host); the operator's corporate host is explicitly called out as NOT a valid clean-host stand-in for *behavioral* UAT (Azure VM is that path, Phase 106) — but is explicitly valid, and used in this research, for *offline binary re-verification* (the smoke workflow's own `if: always()` artifact upload exists precisely to enable this).
- **Do not cut a release until smoke is green** — Phase 104 depends on Phase 101, hard dependency, explicitly stated in ROADMAP.
- **SC1 [Operator-in-loop]:** operator re-runs the hardened smoke workflow immediately before tag push and confirms GREEN (FIC-subject/profile canary re-check, Pitfall 9 `AADSTS700213` recurrence risk).
- **SC2 [Operator-in-loop]:** operator pushes tag `v0.66.1`; `Release` workflow runs to green; all top-level `.exe` (`nono.exe`, `nono-shell-broker.exe`, `nono-wfp-service.exe`) + both MSIs pass the hardened fail-closed verify.
- **SC3:** published artifacts show Verified publisher — issuer chains to a public Microsoft ID Verified CS root, not `CN=nono Test Signing`. **Note (Finding B, corroborated by this research's own live evidence):** the literal wording "Issuer = `Microsoft ID Verified CS` root" is the same disproven issuer-naming heuristic from `101-SIGN03-SMOKE-VERDICT.md` Finding B — a genuine `PublicTrust` signature chains through `Microsoft Enterprise ID Verified Policy AOC CA 02` / `Microsoft Enterprise ID Root CA 2021`, NOT a `Microsoft ID Verified CS...` string. The gate must be implemented as `Status -eq 'Valid'` + reject known-test-cert CN (`CN=nono Test Signing`), with the issuer captured informationally only — exactly the pattern Phase 103's `scripts/gates/trusted-signed-assertion.ps1` already implements correctly. Do not encode the issuer-substring check as a pass/fail condition anywhere in Phase 104's plan.
- **Repo stays PUBLIC** — no `build_notes/`/`.gsd/` staged before any push (release-readiness gate already structurally enforces this).
- **All commits DCO-signed.**
- **Secret-retirement ordering (CLOSE-01) is NOT this phase's concern** — POC secrets stay until Phase 106 (clean-host UAT) passes; Phase 104 must not touch `WINDOWS_SIGNING_CERT`/`WINDOWS_SIGNING_CERT_PASSWORD` or remove `trusted-signing-smoke.yml`.

### Claude's Discretion

- The exact shape of the two `verify-authenticode.ps1` legibility fixes (D-04 flush order; adding a `NoCheck`-mode corroborating classification) — this research proposes a specific, minimal diff below; the planner may refine wording/structure but should not change the underlying approach given it's now empirically justified.
- How to neutralize the stale `publish-crates` job for this tag push (disable vs. no-op vs. explicit `if: false` guard) — this research recommends `if: false` with a dated comment; alternatives are viable as long as the mechanism doesn't preempt Phase 105's dependency-order/index-polling design.
- The exact operator-facing wording/runbook for the "poll until smoke is green" loop.

### Deferred Ideas (OUT OF SCOPE for Phase 104)

- Live multi-registry publish (crates.io/PyPI/npm) — PUB-02, Phase 105.
- Fixing/rewriting the `publish-crates` job's actual publish logic (dependency order, index-visibility polling) — Phase 105's job. This research recommends only *neutralizing* the job for Phase 104's tag push, not rewriting its publish semantics.
- Clean-host UAT (Azure VM, broker spawn, MSI install) — CHOST-03, Phase 106.
- POC secret retirement / `trusted-signing-smoke.yml` removal — CLOSE-01, Phase 107.
- Re-litigating SIGN-01 (profile type) — already confirmed `PublicTrust`, do not re-open.
- Terraform/Pulumi, private VM, SmartScreen-reputation gating — explicitly out of scope per REQUIREMENTS.md's Out-of-Scope table.

## PRIMARY INVESTIGATION — Empirical Resolution of SIGN-03's Root Cause

### Method

1. Downloaded the actual CI-signed artifact from the authoritative failed run: `gh run download 28636133664 -R OscarMackJr/nono -n trusted-signing-smoke-signed -D <tmp>` — succeeded, 19,216-byte `nono-smoke.exe`, signer `CN=TWGGLOBAL.onmicrosoft.com`, issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 02`, cert validity window `2026-07-01 02:24:55` → `2026-07-04 02:24:55` (a 3-day short-lived Trusted Signing cert, consistent with the PublicTrust default lifetime).
2. Ran `Get-AuthenticodeSignature` on this dev host:
   ```
   Status        : UnknownError
   StatusMessage : A certificate chain processed, but terminated in a root certificate
                   which is not trusted by the trust provider
   ```
   [VERIFIED: local reproduction, this session] — this is new information beyond the CI log (CI log never printed `StatusMessage`), and it reproduces on a machine that is NOT `windows-latest` and NOT network-constrained in the way `windows-latest` might be — ruling out "runner-specific" as the sole explanation.
3. Ran `signtool verify /pa /v` (`C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe`) on the same file:
   ```
   Signing Certificate Chain:
       Issued to: Microsoft Enterprise ID Root CA 2021       (self-signed root)
           Issued to: Microsoft Enterprise ID Verified Policy Signing PCA 2021
               Issued to: Microsoft Enterprise ID Verified Policy AOC CA 02
                   Issued to: TWGGLOBAL.onmicrosoft.com        (leaf)
   Number of files successfully Verified: 0
   Number of errors: 1
   SignTool Error: A certificate chain processed, but terminated in a root
       certificate which is not trusted by the trust provider.
   EXITCODE=1
   ```
   [VERIFIED: local reproduction, this session] — **this directly disproves the hypothesis that `signtool /pa` walks a more lenient path than GAS.** Both engines fail on the exact same cause. The `101-SIGN03-SMOKE-VERDICT.md` Finding C's open question ("does signtool `/pa` pass where GAS returns UnknownError?") is answered: **no, it does not** for this specific failure mode (untrusted root). It might still behave differently for *other* failure modes (e.g. a genuinely revoked cert), but for THIS blocker, the two engines agree.
4. Built the chain manually via `X509Chain`, per the investigation's instructions, in two modes:
   - `RevocationMode=Online, RevocationFlag=EntireChain` (this is what `Get-ChainClassification` in `verify-authenticode.ps1` already hardcodes): `Built=False`, only **3** chain elements returned (leaf, AOC CA 02, Signing PCA 2021 — **the root is not even included**), and every element carries `RevocationStatusUnknown` + `OfflineRevocation`. **No `UntrustedRoot` flag appears in Online mode at all.**
   - `RevocationMode=NoCheck, RevocationFlag=EntireChain`: `Built=False`, all **4** elements returned including the root, and the root element alone carries `UntrustedRoot` — `A certificate chain processed, but terminated in a root certificate which is not trusted by the trust provider.`
   [VERIFIED: local reproduction, this session]
5. Fetched the **live, current** Microsoft root-certificate CTL directly: `certutil -generateSSTFromWU roots.sst` (554 certificates). Searched for `Microsoft Enterprise ID Root CA 2021` (thumbprint `991D364E97882715B80ED978F53E1F35DC2F07C2`, read from the signer cert's chain) — **absent**. Only `Microsoft Identity Verification Root Certificate Authority 2020` (a *different*, older, already-locally-trusted root — the one that successfully validated the RFC3161 *timestamp* signature on the same binary) is present.
   [VERIFIED: local reproduction, this session — freshly pulled, not a stale local cache]
6. Confirmed via the official `ctldl.windowsupdate.com` mechanism docs [CITED: learn.microsoft.com — "How to Update Trusted Root Certificates in Windows"] that the CTL updates on an approximately twice-monthly cadence, and that this is the standard, universal distribution path for all Windows hosts (not a corporate-network-specific mechanism) — network reachability to the endpoint was independently confirmed (HTTP 200) so this is not a corporate-proxy/firewall confound either.

### Conclusion / Resolution Class

**This is resolution class (b) from `101-SIGN03-SMOKE-VERDICT.md`'s hand-off list: a genuine external Microsoft-side root-certificate propagation gap — NOT fixable by any code change in this repository, and NOT limited to `windows-latest`.** It reproduces identically on a corporate dev host with a freshly-pulled CTL. The Trusted Signing PublicTrust profile's *current* active issuing hierarchy (`... AOC CA 02` ← `... Signing PCA 2021` ← `Microsoft Enterprise ID Root CA 2021`) chains through a root that Microsoft has not yet published to the publicly-distributed Trusted Root Program CTL, as of 2026-07-03. This is consistent with (and sharpens) `101-SIGN01-FINDING.md`'s note about "the March-2026 AOC/EOC CA rotation" and the leaf's very recent `NotBefore` (2026-07-01) — Microsoft appears to have rotated to a new issuing hierarchy whose root has not finished propagating.

**Practical implication for planning:** every artifact signed by this Trusted Signing account/profile — the smoke throwaway *and the real release binaries* — will show the identical `UnknownError`/"Unknown publisher" symptom until Microsoft's root-store propagation catches up. This is not something Phase 104 can force to resolve in a single session; the phase must be planned around a **poll-until-green** operator loop, not a one-shot retry.

**Actionable, low-cost monitoring recommendation:** since the failure reproduces identically and losslessly on a local Windows host, the operator does NOT need to burn a full CI dispatch to check for propagation. A cheap local pre-check — pull the current CTL (`certutil -generateSSTFromWU roots.sst`) and grep for thumbprint `991D364E97882715B80ED978F53E1F35DC2F07C2` (or, more robustly, re-run `signtool verify /pa /v` against the already-downloaded `nono-smoke.exe` artifact) — costs seconds and can be run daily until it starts succeeding, at which point a real smoke dispatch confirms it end-to-end on the actual `windows-latest` runner before the tag push.

### Root cause of Finding C (D-04 diagnostic-flush gap) — also resolved

[CITED: github.com/actions/runner, ADR 0277 "Run action shell options"] — "For `pwsh` and `powershell` builtins, [the runner will] prepend `$ErrorActionPreference = 'stop'` to script contents." This is confirmed, official GitHub Actions runner behavior for every `shell: pwsh` `run:` step in both `release.yml` and `trusted-signing-smoke.yml`.

[VERIFIED: local reproduction, this session] — a minimal repro script confirms: with the ambient (default) `$ErrorActionPreference`, a `Write-Error` call is non-terminating and the following `Write-Host` line executes normally. With `$ErrorActionPreference = 'Stop'` explicitly set (matching what GitHub Actions injects), the identical `Write-Error` call becomes a **terminating** error — the following line does **not** execute, and (absent a `try/catch`) the whole script aborts at that point.

`Assert-TrustedSignature`'s Strict-mode failure path (`scripts/verify-authenticode.ps1:341-350`) is:
```powershell
if ($gas.Status -ne 'Valid') {
    Write-Error "Authenticode verification failed for $Path with status $($gas.Status)."
    Write-ChainDiagnostic -Path $Path -Gas $gas -SignToolResult $signToolResult -Classification $attemptResult.Classification
    throw "Assert-TrustedSignature: GAS status not Valid ($($gas.Status)) for $Path"
}
```
Under CI's injected `$ErrorActionPreference = 'Stop'`, the `Write-Error` call on line 342 terminates the script **immediately** — `Write-ChainDiagnostic` (the full chain dump, signtool `/pa` StdOut/StdErr, revocation-URL reachability) and the subsequent `throw` never execute. This is precisely what the live CI log showed: the retry-loop's `Write-Host` line printed fine (it never calls `Write-Error`), but the rich diagnostic dump never appeared.

Corroborating evidence already exists in the repo: `scripts/gates/trusted-signed-assertion.ps1` (Phase 103) *already* defensively sets `$ErrorActionPreference = 'Continue'` at the top of its `Invoke-Gate` function, immediately before calling `Assert-TrustedSignature` — independent evidence that this exact class of problem was anticipated (or hit) once already in this codebase, just not fixed at the source (`verify-authenticode.ps1` itself, or the two raw `release.yml`/`trusted-signing-smoke.yml` call sites that dot-source the helper directly with no such guard).

**Minimal, safe fix (does not touch the fail-closed gate condition):** in `Write-Error` calls within `Assert-TrustedSignature`'s Strict-mode failure branches, either (a) add `-ErrorAction Continue` explicitly to each `Write-Error` call so it is non-terminating regardless of the caller's ambient preference, or (b) reorder so `Write-ChainDiagnostic` runs *before* `Write-Error`/`throw`. Option (a) is preferred — it is a one-line-per-call-site change, is textually obvious in review, and doesn't depend on call ordering being preserved correctly by future edits.

## Standard Stack

No new external packages are introduced by this phase — it is a diagnostics/CI-hardening + release-mechanics phase using tools already present in the toolchain.

### Core (already present, no install needed)
| Tool | Version (confirmed this session) | Purpose | Why Standard |
|------|-----------------------------------|---------|---------------|
| `signtool.exe` | `10.0.26100.0` (Windows SDK, this host) [VERIFIED: local] | Independent Authenticode verify engine | Already used by `Invoke-SignToolVerify`; Microsoft's canonical CLI verify tool. |
| PowerShell `X509Chain`/`X509Certificate2` (.NET) | Built into PowerShell 7 / Windows PowerShell | Manual chain-build + flag introspection | Already used by `Get-ChainClassification`; the only first-class API for chain-status flags (no CLI equivalent gives structured flags). |
| `certutil.exe` | Built into Windows | Fetch live root CTL (`-generateSSTFromWU`) | Canonical, built-in way to pull the actual current Microsoft root list without a browser/manual download; used in this research to prove the propagation gap. |
| `gh` CLI | 2.91.0 [VERIFIED: local, `gh --version`] | Download CI artifacts, dispatch/inspect workflow runs | Already the project's standard GitHub automation tool (per auto-memory: "gh CLI available — use it directly"). |

### Package Legitimacy Audit

**Not applicable.** This phase installs no new external packages (no npm/pip/cargo/crates additions). All tools used (`signtool`, `certutil`, PowerShell built-ins, `gh`) are already present in the CI runner image and/or this repo's toolchain. Skip the slopcheck/registry-verification gate.

## Architecture Patterns

### System Architecture Diagram — Verify-Gate Data Flow (current + proposed fixes)

```
                    ┌─────────────────────────────────────────┐
                    │  Azure Trusted Signing (PublicTrust)     │
                    │  Issuing hierarchy: Enterprise ID Root   │
                    │  CA 2021 → ...PCA 2021 → ...AOC CA 02    │
                    └───────────────┬───────────────────────────┘
                                    │ signs binary (WORKS — SIGN-01/02 proven)
                                    ▼
                    ┌─────────────────────────────────────────┐
                    │  CI runner (windows-latest) OR dev host  │
                    │  signed artifact on disk                 │
                    └───────────────┬───────────────────────────┘
                                    │
              ┌─────────────────────┴─────────────────────────┐
              ▼                                                 ▼
  Get-AuthenticodeSignature                       signtool verify /pa /v
  (CryptoAPI2 WinVerifyTrust)                     (independent CryptoAPI client)
              │                                                 │
              │   both walk the SAME OS trust-anchor store       │
              │   (LocalMachine\Root, populated by AuthRoot CTL) │
              └─────────────────────┬─────────────────────────┘
                                    ▼
                    ┌─────────────────────────────────────────┐
                    │  Windows Update AuthRoot CTL             │
                    │  (ctldl.windowsupdate.com, ~biweekly)    │
                    │  MISSING: "Enterprise ID Root CA 2021"   │
                    │  → BOTH engines report UntrustedRoot      │
                    │    (surfaces as GAS Status=UnknownError)  │
                    └─────────────────────┬─────────────────────┘
                                          │ fail-closed (correct)
                                          ▼
                    Assert-TrustedSignature throws
                    ├── [FIX 1] Write-Error -ErrorAction Continue
                    │            (so Write-ChainDiagnostic actually runs
                    │             before the terminating throw, under
                    │             CI's forced $ErrorActionPreference='Stop')
                    └── [FIX 2] add NoCheck-mode chain build alongside
                                 the existing Online-mode build, so the
                                 diagnostic correctly reports "genuine
                                 UntrustedRoot" instead of misclassifying
                                 as "transient, retrying" (Online mode
                                 masks UntrustedRoot behind
                                 RevocationStatusUnknown/OfflineRevocation
                                 flags on a 3-element truncated chain)
```

### Pattern 1: Dual-mode chain classification (NoCheck as ground truth, Online as the operational check)

**What:** Keep the existing `Online`-mode `Get-ChainClassification` as the actual pass/fail input (matches production reality — CI should still care about revocation reachability in the general case), but ALSO build a `NoCheck`-mode chain purely for diagnostic classification purposes when the Online build fails. If the `NoCheck` chain shows `UntrustedRoot` on the root element, classify and log the failure as "genuine untrusted root (root cert not yet in local/CI trust store)" rather than "transient revocation" — this stops the retry loop from burning 3 attempts (14s of backoff) on a condition that can never resolve via retry, and gives the operator/CI log an unambiguous, correctly-labeled diagnosis.

**When to use:** Only in the failure/diagnostic path (`Write-ChainDiagnostic`), never as a new pass condition — this is strictly additive legibility, not a gate change.

**Example (illustrative diff shape, not literal replacement code):**
```powershell
# Source: this research, verified locally 2026-07-03 against the real CI-signed artifact
function Get-ChainClassification {
    param(
        [Parameter(Mandatory)][System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
    )

    # Existing Online-mode build (unchanged — remains the operational check)
    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
    $chain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain
    $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(15)
    $built = $chain.Build($Certificate)

    $allFlags = 0
    foreach ($status in $chain.ChainStatus) { $allFlags = $allFlags -bor [int]$status.Status }
    $flags = [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]$allFlags

    # NEW: NoCheck-mode corroborating build — diagnostic-only, never gates.
    # Reveals whether Online mode's transient-looking flags are masking a genuine
    # UntrustedRoot that Online mode can't even reach (research finding: Online mode
    # truncates the chain to 3 elements and omits the root entirely when revocation
    # can't be checked, so UntrustedRoot never surfaces there).
    $noCheckChain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    $noCheckChain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
    $noCheckChain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain
    $noCheckBuilt = $noCheckChain.Build($Certificate)
    $noCheckUntrustedRoot = $false
    foreach ($el in $noCheckChain.ChainElements) {
        foreach ($s in $el.ChainElementStatus) {
            if ($s.Status -band [System.Security.Cryptography.X509Certificates.X509ChainStatusFlags]::UntrustedRoot) {
                $noCheckUntrustedRoot = $true
            }
        }
    }

    $classification = Get-FlagClassification -Flags $flags -Built $built
    # A NoCheck-confirmed UntrustedRoot always overrides an Online-mode "transient" read —
    # this is real evidence a retry cannot fix.
    if ($noCheckUntrustedRoot) {
        $classification.IsTransient = $false
        $classification.IsUntrustedRoot = $true
    }
    $classification | Add-Member -MemberType NoteProperty -Name Chain -Value $chain -Force
    $classification | Add-Member -MemberType NoteProperty -Name NoCheckChain -Value $noCheckChain -Force
    return $classification
}
```

### Pattern 2: `Write-Error -ErrorAction Continue` at every terminating-risk call site inside a helper meant to run under CI's forced `Stop` preference

**What:** Any `Write-Error` call inside a function that must guarantee subsequent lines execute (diagnostics, cleanup) needs an explicit `-ErrorAction Continue`, because the function cannot assume its caller's ambient `$ErrorActionPreference` — CI callers force it to `Stop`.
**When to use:** Any shared PowerShell helper dot-sourced into a `shell: pwsh` GitHub Actions step.
**Example:**
```powershell
# Source: this research (D-04 flush-gap fix), confirmed against actions/runner ADR 0277
# and local repro (Write-Error under $ErrorActionPreference='Stop' terminates before
# the next statement runs; explicit -ErrorAction Continue prevents that regardless of caller).
if ($gas.Status -ne 'Valid') {
    Write-Error -ErrorAction Continue "Authenticode verification failed for $Path with status $($gas.Status)."
    Write-ChainDiagnostic -Path $Path -Gas $gas -SignToolResult $signToolResult -Classification $attemptResult.Classification
    throw "Assert-TrustedSignature: GAS status not Valid ($($gas.Status)) for $Path"
}
```

### Pattern 3: Neutralize a downstream job that references pre-rename identities without touching its future (Phase 105) logic

**What:** `release.yml`'s `publish-crates` job (`needs: release`, runs automatically on any non-alpha/beta/rc tag push) still runs `cargo publish -p nono`, `-p nono-proxy`, `-p nono-cli` — package names that no longer exist post-Phase-102 rename (`crates/nono/Cargo.toml` `[package] name = "nono-sandbox"`, confirmed this session via direct `grep`). This job WILL fail immediately (`error: package ID specification 'nono' matched no packages`) the moment the operator pushes the `v0.66.1` tag, which would make the overall `Release` workflow run show red/partial-failure — directly conflicting with SC2's "the `Release` workflow runs to green." `update-homebrew-core` is unaffected (it only references a GitHub archive download URL, not crate identities) and needs no change.
**When to use:** Any time a phase's tag-push exercises a workflow that has jobs explicitly deferred to a *later* phase's scope (here, Phase 105/PUB-02 owns rewriting `publish-crates`'s actual publish logic).
**Example (recommended minimal guard, NOT a rewrite of publish logic):**
```yaml
  publish-crates:
    name: Publish to crates.io
    needs: release
    runs-on: ubuntu-latest
    # NEUTRALIZED for the Phase 104 (REL-01) release cut: this job's -p selectors
    # (nono / nono-proxy / nono-cli) reference the PRE-rename package identities.
    # Phase 102 renamed the actual [package] names to nono-sandbox / nono-sandbox-proxy
    # / nono-sandbox-cli; Phase 105 (PUB-02) rewrites this job with the correct names +
    # dependency-order + index-visibility polling. Re-enable only in Phase 105.
    if: false
    ...
```

### Anti-Patterns to Avoid

- **Weakening `Status -ne 'Valid'` to also accept `UnknownError`:** explicitly forbidden (SIGN-02 invariant, REQUIREMENTS.md Out-of-Scope table). Never do this even though this research proves the failure is externally-caused — "externally caused" does not mean "safe to accept"; a genuinely revoked or compromised cert could also produce `UnknownError` in other circumstances.
- **Resurrecting the issuer-substring heuristic** (`Enterprise ID Verified Policy` = bad / `Microsoft ID Verified CS` = good) as a pass/fail condition anywhere — Finding B already disproved it empirically, and this research's own live evidence reconfirms a real `PublicTrust` signature chains through exactly that "bad-looking" issuer string.
- **Fixing `publish-crates`'s package names as part of Phase 104** — that is Phase 105's job (dependency-order + index-visibility polling design). Phase 104 should only *neutralize*, not *rewrite*, this job.
- **Treating the D-04 flush-gap fix as sufficient to make SIGN-03 pass** — it only makes the *next* failure legible; it does not and cannot fix the underlying untrusted-root condition, which is external and time-bound.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Authenticode chain validation | A custom ASN.1/chain walker | `System.Security.Cryptography.X509Certificates.X509Chain` (already used) | .NET's chain engine already wraps the correct CAPI2 primitives; a hand-rolled parser would be a security-critical reimplementation with no upside. |
| Detecting "is this root actually trusted right now" | Scraping the Microsoft Trusted Root Program docs pages for a cert list | `certutil -generateSSTFromWU` (live, authoritative, official Windows tool) | Docs pages lag; the actual runtime trust decision only depends on the live CTL, which this tool fetches directly from the source. |
| Retrying transient CI failures | A bespoke sleep/retry wrapper around the whole verify step | The existing `Invoke-VerifyWithTransientRetry` (already correctly bounded, backoff, never-upgrades-a-fail) | Already built and tested in Phase 101; only its input classification (Online-mode-only) needs the NoCheck corroboration, not the retry mechanism itself. |

**Key insight:** every primitive needed to *diagnose* this blocker already exists in the OS or in this repo's own Phase 101 work — the gap was in how the existing primitives were sequenced (Write-Error before Write-ChainDiagnostic; Online-mode-only classification), not in missing tooling.

## Common Pitfalls

(See also `.planning/research/PITFALLS.md` Pitfalls 2, 3, 9, 10 — all still fully applicable and reconfirmed by this session's evidence. New pitfall specific to Phase 104 below.)

### Pitfall 104-A: `publish-crates` job will fail on tag push due to the Phase 102 rename, silently conflicting with SC2

**What goes wrong:** The operator pushes `v0.66.1`, expecting only the signing/verify/GitHub-Release path to matter for "green." The `publish-crates` job auto-fires (needs: release, no manual trigger required) and fails immediately because `-p nono` no longer resolves to any workspace package.
**Why it happens:** Phase 102 renamed `[package] name` but `release.yml`'s publish-crates job was explicitly, deliberately left unchanged (documented in-file: "Phase 105 (PUB-02) rewrites these publish targets... Deliberately left unchanged here") — a correct scoping decision at the time, but one that becomes a live landmine the moment Phase 104 actually pushes a tag.
**How to avoid:** Neutralize the job (see Pattern 3 above) before the tag push, as part of Phase 104's own prep work — do not defer this discovery to the operator mid-push.
**Warning signs:** Any release-readiness dry-run or gate that doesn't specifically simulate the full tag-push job graph (build → release → publish-crates/update-homebrew-core) will miss this, because `cargo publish -p nono` was never actually run in any of the existing dry-run scripts against the CURRENT (post-rename) tree — the existing `scripts/release-dry-run.ps1` referenced in `RELEASE-RUNBOOK.md` predates the rename and may itself need re-verification of exactly which `-p` names it exercises.

### Pitfall 104-B: Treating "smoke went green once" as durable

**What goes wrong:** Because the root cause is an external, time-bound propagation gap, a green smoke run today does not certify the *next* CI-signed artifact will also be trusted — if the account's issuing CA rotates again (as it apparently just did, `AOC CA 01` → `AOC CA 02`, between the June-30 and July-3 runs), the same class of failure could recur on a *future* tag or re-run.
**Why it happens:** Azure Trusted Signing's PublicTrust profiles are actively rotating their issuing hierarchy on a cadence outside this repo's control; a pass today is evidence for today, not a permanent guarantee.
**How to avoid:** SC1 already requires the operator to re-run smoke immediately before every tag push, not rely on a stale prior green — this research reconfirms that requirement is load-bearing, not belt-and-suspenders.
**Warning signs:** A future release attempted without re-running smoke first, on the assumption "we proved this works in Phase 104."

## Code Examples

Already included inline under Architecture Patterns above (Pattern 1, Pattern 2, Pattern 3) — these are the three concrete, minimal diffs this phase's plan should implement, each independently verifiable and each strictly additive/non-weakening to the existing fail-closed contract.

## State of the Art

| Old Approach | Current (verified) Approach | When Changed | Impact |
|--------------|------------------------------|---------------|--------|
| Issuer-substring pattern matching (`Microsoft ID Verified CS` = good, `Enterprise ID Verified Policy` = bad) as an implicit pass/fail signal in requirements/roadmap wording | `Status -eq 'Valid'` + reject known-test-cert CN only; issuer captured informationally, never gated | Disproven 2026-07-02/03 (Finding B), reconfirmed this session | REQUIREMENTS.md/ROADMAP.md wording for SIGN-03/REL-01 SC3 is now known-stale and should be corrected in this phase's docs updates (not just left as historical color) so a future reader doesn't reintroduce the heuristic. |
| Single-engine (GAS-only) verify | Dual-engine (GAS + `signtool /pa`) AND-gate | Phase 101 (SIGN-02) | Already shipped; this research confirms both engines currently agree (both fail identically on the untrusted-root case) — the dual-engine gate is not (yet) providing extra signal for THIS specific failure mode, but remains valuable for other failure classes (e.g. a corrupted signature that only one engine catches). |
| Online-only chain classification | Online (operational) + NoCheck (diagnostic corroboration) — proposed this session | Not yet shipped — Phase 104 to implement | Prevents future retries from being wasted on a case that can never resolve via retry, and gives correct root-cause attribution in logs. |

**Deprecated/outdated:** The `101-RESEARCH.md` issuer-naming heuristic (see Assumptions Log A1) — do not resurrect.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Microsoft will eventually publish `Microsoft Enterprise ID Root CA 2021` to the public Trusted Root Program CTL, resolving this automatically without any Azure-side account/profile change | Primary Investigation, Conclusion | If this root is intentionally NOT destined for the standard public CTL (e.g., a private/enterprise-only hierarchy was mistakenly issued against instead of the public one), the propagation-wait strategy would never resolve and the real fix would be an Azure-side profile/account correction (re-opening SIGN-01) — the operator should escalate to Azure/Microsoft support if smoke has not gone green after a reasonable multi-day polling window, rather than waiting indefinitely. |
| A2 | Every top-level `.exe` and both MSIs in the real release will exhibit the identical untrusted-root symptom as the smoke throwaway (same account/profile → same issuing hierarchy) | Primary Investigation, "Practical implication for planning" | Low risk — this follows directly from all artifacts being signed via the same `certificate-profile-name: ${{ vars.TRUSTED_SIGNING_PROFILE }}` in `release.yml`, verified by inspection this session; but if Azure round-robins between multiple issuing CAs even within one profile, some artifacts could pass while others fail, which would be a confusing, non-deterministic partial-green outcome the plan should be prepared to re-diagnose rather than assume is a fluke. |
| A3 | `existing scripts/release-dry-run.ps1` and the release-readiness gate do not already catch the stale `publish-crates -p nono` selector problem | Common Pitfalls, Pitfall 104-A | If a dry-run script does actually simulate `cargo publish -p nono` (unlikely given it predates the rename, but not independently confirmed by this research beyond reading `RELEASE-RUNBOOK.md`), the actual failure might already be known/flagged elsewhere; the planner should grep `scripts/release-dry-run.ps1` directly before assuming this is entirely novel. |

## Open Questions (RESOLVED)

**RESOLVED (2026-07-03, planning):** OQ1 → addressed by the poll-until-green operator loop in Plan 104-03 (local `certutil` pre-check before each CI dispatch) + the escalation guidance below. OQ2 → MOOT: `scripts/release-dry-run.ps1` was already reconciled to the renamed names (`nono-sandbox`/`nono-sandbox-proxy`/`nono-sandbox-cli`) during Phase 103's rename-consumer cleanup (commit `44a6ca2a`), so it no longer references the stale `-p nono` selectors; the `if: false` neutralization of the `publish-crates` job (Plan 104-02) is the primary mitigation and does not depend on the dry-run script.

1. **How long will the root-propagation gap persist?** *(RESOLVED: poll-until-green loop, Plan 104-03.)*
   - What we know: the current CTL (fetched live this session) lacks the root; Microsoft's public CTL updates on an approximately twice-monthly cadence per official docs; the leaf cert's `NotBefore` is only 2 days old (very recent CA rotation).
   - What's unclear: no official Microsoft changelog entry was found (via WebFetch of the Trusted Root Program release-notes page) confirming this specific root is scheduled for an upcoming release, or already deprecated/replaced by yet another rotation.
   - Recommendation: plan for a poll-until-green operator loop (daily local pre-check via `certutil -generateSSTFromWU`, escalating to a full smoke dispatch once the local pre-check succeeds); if no progress after ~1-2 weeks, recommend the operator open an Azure support ticket referencing the specific thumbprint and issuing CA, since this blocks a paying customer's entire go-live.

2. **Does `scripts/release-dry-run.ps1` (referenced in the Phase 97 runbook) already exercise the broken `-p nono` selector, and would it have caught this?**
   - What we know: the runbook references it as passing pre-Phase-102; Phase 102 was completed after that runbook was written.
   - What's unclear: whether it was re-run against the post-rename tree, and whether it simulates the actual `cargo publish -p nono` command or only a `--dry-run` variant that might behave differently.
   - Recommendation: the plan should include a task to `grep`/re-run this script (or explicitly confirm it's superseded/irrelevant) before relying on Pattern 3's `if: false` guard as the sole mitigation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|--------------|-----------|---------|----------|
| `gh` CLI | Downloading CI artifacts for offline re-verification | ✓ | 2.91.0 (authenticated, `repo`+`workflow` scopes) | — |
| `signtool.exe` | Independent Authenticode verify | ✓ | 10.0.26100.0 (Windows SDK, this host) | — |
| PowerShell (`pwsh`/Windows PowerShell) `X509Chain` APIs | Manual chain-build diagnostics | ✓ | Built into .NET / PowerShell, this host | — |
| `certutil.exe` | Live root-CTL fetch | ✓ | Built into Windows | — |
| Azure Trusted Signing (live account `ArtifactNono`) | The sign operation itself | ✓ (SIGN-01/02 already proved this works) | `PublicTrust` profile `NonoCertProfile` | — |
| Microsoft Trusted Root Program CTL containing `Microsoft Enterprise ID Root CA 2021` | SIGN-03/REL-01 SC1/SC3 | ✗ (confirmed absent, live pull, this session) | — | **No code-level fallback exists** — this is the phase's genuine external blocker; the only "fallback" is the poll-until-green operator loop described above. |

**Missing dependencies with no fallback:**
- The Microsoft-side root-certificate propagation for the current Trusted Signing issuing hierarchy. This blocks SC1 (smoke green) and therefore SC2/SC3 (release cut) until it resolves externally.

**Missing dependencies with fallback:**
- None — the CTL propagation gap has no code-side fallback; only operator patience/polling and (if prolonged) an Azure support escalation.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Pester-free, imperative PowerShell test harness (repo convention — `scripts/tests/test_*.ps1`, `$ErrorActionPreference='Stop'`, explicit exit codes, no framework dependency) |
| Config file | none — convention-based, no config file needed |
| Quick run command | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All` |
| Full suite command | `make ci` (clippy + fmt + tests) plus the PowerShell harness above (not currently wired into `make ci` — a Rust-only target; the PowerShell harness is invoked standalone) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|--------------|
| REL-01 (D-04 flush fix) | `Write-ChainDiagnostic` output appears before any terminating throw, even under `$ErrorActionPreference='Stop'` | unit (imperative harness, new case) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case diagnosticFlushUnderStop` (new case — does not exist yet) | ❌ Wave 0 — add a new `Test-DiagnosticFlushUnderStop` case that sets `$ErrorActionPreference='Stop'` in a child scope/process, calls the failure path, and asserts the diagnostic Write-Host output actually appears in captured output before the throw propagates. |
| REL-01 (NoCheck-mode classification) | A synthetic/mocked untrusted-root chain is classified as `IsUntrustedRoot=$true, IsTransient=$false` even when the Online-mode read alone would look transient | unit (imperative harness, new case) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case noCheckOverridesTransient` (new case) | ❌ Wave 0 — existing `Test-TransientRetry`/`Test-UntrustedRootNoRetry` cases (lines 192, 226) are the closest analogues to extend/mirror. |
| REL-01 (publish-crates neutralization) | The tag-push job graph does not fail on `publish-crates`'s stale `-p` selectors | integration / manual-only (requires an actual or dry-run GitHub Actions dispatch — cannot be a local unit test) | Manual: `workflow_dispatch` a dry run of `release.yml` on a test tag, or `act`/local YAML lint confirming the `if: false` guard is syntactically present and `needs:`/job graph still resolves | ❌ Wave 0 — no existing automated check of this job's selectors against the live Cargo.toml package names; recommend a lightweight grep-based gate script (`-p nono` string absent from any *enabled* job in `release.yml`) as a cheap regression guard, since a full workflow dispatch is expensive/outward-facing. |
| REL-01 (SC1/SC2/SC3 overall) | Smoke green, tag-push green, Verified publisher | manual-only (operator-in-loop, outward-facing) | N/A — explicitly operator-gated per REQUIREMENTS.md/ROADMAP.md annotations | N/A |

### Sampling Rate

- **Per task commit:** `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All` (fast, no network, all mocked cases) after any `verify-authenticode.ps1` edit.
- **Per wave merge:** `make ci` (Rust side unaffected by this phase, but still the standard gate) + the full PowerShell harness + the new grep-based `publish-crates` selector guard.
- **Phase gate:** both automated gates green; SC1/SC2/SC3 require explicit operator confirmation (outward-facing, cannot be automated in CI without literally cutting the release).

### Wave 0 Gaps

- [ ] `scripts/tests/test_verify_authenticode.ps1` — add `Test-DiagnosticFlushUnderStop` case (D-04 flush-order regression guard)
- [ ] `scripts/tests/test_verify_authenticode.ps1` — add `Test-NoCheckOverridesTransient` case (NoCheck-mode classification regression guard)
- [ ] A lightweight, fast, local grep-based gate (new small script or an inline check in an existing gate) asserting `release.yml` has no *enabled* job referencing `-p nono`/`-p nono-proxy`/`-p nono-cli` — cheap regression guard against re-introducing Pitfall 104-A
- [ ] Confirm whether `scripts/release-dry-run.ps1` needs re-running/updating post-rename (Open Question 2) before relying on it as evidence the tag-push job graph is otherwise clean

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | No | N/A — no user-facing auth surface in this phase |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A |
| V5 Input Validation | No | N/A — no new untrusted input parsing |
| V6 Cryptography / Code Signing | **Yes** | Authenticode chain-of-trust validation via OS-native `X509Chain`/`WinVerifyTrust` (never hand-rolled); fail-closed on any non-`Valid` status; never accept a chain that can't be built to a trusted root, regardless of cause. |
| V14 Configuration | Yes (secondarily) | CI secrets (`AZURE_CLIENT_ID`/`AZURE_TENANT_ID`/`AZURE_SUBSCRIPTION_ID`, `CARGO_REGISTRY_TOKEN`, `HOMEBREW_CORE_TOKEN`) are GitHub-managed secrets, unchanged by this phase; the `publish-crates` neutralization (Pattern 3) must not accidentally expose or misuse `CARGO_REGISTRY_TOKEN` (an `if: false` guard is the safest form — it prevents the job body, including any secret reference, from executing at all). |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Signature-verification bypass via a loosened fail-closed check | Tampering / Spoofing | Never weaken `Status -ne 'Valid'`; this research explicitly reconfirms and does not touch that condition in any proposed diff. |
| Accepting a chain that resolves to an untrusted root because "it's probably fine" | Tampering | Fail closed unconditionally; the poll-until-green operator loop is the only sanctioned response to this specific external blocker — never a code-side acceptance of `UnknownError`. |
| Silent partial-publish exposing a half-signed/half-published release as if complete | Repudiation / Information Disclosure (of pipeline state) | Neutralizing `publish-crates` via `if: false` (not a silent no-op that still consumes secrets) ensures no ambiguous partial-publish state; Phase 105 handles publish with its own explicit dependency-order + index-polling correctness guarantees. |

## Sources

### Primary (HIGH confidence — direct local empirical verification, this session)
- `gh run download 28636133664 -R OscarMackJr/nono -n trusted-signing-smoke-signed` — the actual CI-signed artifact, downloaded and inspected directly.
- `Get-AuthenticodeSignature` + `signtool verify /pa /v` (Windows SDK 10.0.26100.0) run locally against that artifact.
- Manual `X509Chain` builds (`RevocationMode=Online` and `=NoCheck`) run locally against the same artifact's signer certificate.
- `certutil -generateSSTFromWU roots.sst` — live pull of the current Microsoft Windows Update root CTL (554 certs), searched for the specific root thumbprint.
- Local PowerShell repro of `Write-Error` behavior under default vs. `$ErrorActionPreference='Stop'`.
- Direct `grep` of `crates/nono/Cargo.toml`, `crates/nono-proxy/Cargo.toml`, `crates/nono-cli/Cargo.toml` `[package] name` fields (confirms the Phase 102 rename is live in the tree) and of `.github/workflows/release.yml` (confirms the `publish-crates` job's stale `-p` selectors).

### Secondary (MEDIUM confidence — official docs, WebFetch/WebSearch verified)
- [actions/runner ADR 0277 "Run action shell options"](https://github.com/actions/runner/blob/main/docs/adrs/0277-run-action-shell-options.md) — confirms GitHub Actions prepends `$ErrorActionPreference = 'stop'` to `pwsh`/`powershell` `run:` steps. [CITED]
- [How to Update Trusted Root Certificates in Windows](https://woshub.com/updating-trusted-root-certificates-in-windows-10/) — confirms the `ctldl.windowsupdate.com` CTL mechanism and its approximate biweekly update cadence. [CITED]
- [Microsoft Trusted Root Certificate Program — Release Notes index](https://learn.microsoft.com/en-us/security/trusted-root/release-notes) — confirms the program's monthly/twice-monthly cadence structure; did not surface a specific dated entry for this root (content returned was truncated/older than 2026). [CITED, incomplete]
- [Root certificate trust problem after creating an Azure Trusted Signing certificate profile - Microsoft Q&A](https://learn.microsoft.com/en-au/answers/questions/2140998/root-certificate-trust-problem-after-creating-an-a) — corroborates that this exact class of symptom ("chain terminated in an untrusted root," "Publisher: Unknown") is a known, recurring community-reported issue for Azure Trusted Signing, with Microsoft's own guidance being "ensure Windows is fully updated" (i.e., wait for root propagation) — consistent with this research's conclusion. [CITED]
- [Azure Trusted Signing CA root certificate not trusted - Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/2238216/azure-trusted-signing-ca-root-certificate-not-trus) — same class of community report. [CITED]

### Tertiary (LOW confidence — not used as load-bearing evidence)
- General WebSearch summaries characterizing GitHub Actions pwsh error handling (superseded by the direct ADR fetch above, kept only as corroboration).

## Metadata

**Confidence breakdown:**
- Primary investigation (SIGN-03 root cause, signtool-vs-GAS, D-04 flush gap): HIGH — resolved via direct, reproducible, local empirical testing against the real CI-signed artifact, not inference from logs.
- `publish-crates` stale-selector finding: HIGH — directly confirmed via `grep` against the current tree; deterministic failure mode (cargo package-ID resolution), not speculative.
- Timeline for Microsoft's root-propagation fix (Open Question 1): LOW — genuinely external and unknowable from this session; flagged honestly as an assumption (A1) requiring operator judgment/possible escalation.
- Architecture/gate-diff proposals: HIGH — all three proposed diffs (D-04 flush, NoCheck corroboration, publish-crates neutralization) are minimal, additive, and directly justified by the empirical findings above; none touch the fail-closed contract.

**Research date:** 2026-07-03
**Valid until:** The root-cause finding (untrusted-root, external propagation gap) should be re-verified at the start of Phase 104 execution if more than ~3-4 days elapse before planning converts to execution (cheap re-check: re-run `certutil -generateSSTFromWU` + `signtool verify /pa /v` against the already-downloaded `nono-smoke.exe` artifact) — Microsoft's CTL could update at any time and change the phase's actual blocking status.
