# Requirements: nono v3.5 — Trusted Signing Go-Live + First Distributed Release

**Defined:** 2026-07-02
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and, for v3.5, actually *distributable*: a publicly-trusted-signed release that runs out-of-the-box on a clean host, published live under fork-owned identities.

**Scope:** four pillars — (1) resolve the Azure Trusted Signing verify-gate `UnknownError` and harden the CI Authenticode verify; (2) cut the first publicly-trusted-signed `0.66.1` release; (3) publish `0.66.1` live to crates.io + PyPI + npm under **fork-owned `nono-sandbox` identities** (upstream owns the `nono*` names); (4) prove out-of-the-box clean-host behavior on a fresh Azure Win11 VM, draining both host-gated distribution todos, then close out the POC signing path. **Full go-live EXECUTE posture (operator-in-loop).**

> **Architecture invariants:**
> - **`UnknownError` is a disambiguable 3-cause taxonomy**, not a mystery: (a) the profile is `PublicTrustTest` not `PublicTrust` (Azure CLI has no plain "Test" type — the cookbook's "Test" = `PublicTrustTest`, whose issuer naming reproduces the exact 2026-06-30 symptom); (b) runner-side chain-build staleness from Azure's March-2026 AOC/EOC CA rotation; (c) CRL/OCSP revocation timeout. Probes: `az trustedsigning certificate-profile show` → confirm `profileType == PublicTrust`; `signtool verify /pa /v`; revocation-disabled diagnostic.
> - **Never loosen the fail-closed verify.** `release.yml`'s `Get-AuthenticodeSignature -ne 'Valid'` gate stays fail-closed — the fix is a robuster chain build + `signtool /pa` fallback + clearer diagnostics, NOT accepting `UnknownError`.
> - **Verify-gate debugging is CI-side** (GitHub `windows-latest`, a clean cloud host); only the behavioral install/broker-spawn/no-VC++ tests need the Azure VM. The operator's corporate host is NOT a valid clean host (POC cert previously imported + VC++ installed + corporate proxy/EDR/managed-trust-store confound the exact chain-build/revocation failure).
> - **Registry identities are upstream-owned.** crates.io `nono`/`nono-proxy`, PyPI `nono-py`, npm `nono-ts` belong to the upstream maintainer (lukehinds). The fork publishes under **`nono-sandbox`** family names it owns — the product/binary name `nono` (`[[bin]] name`), the `nono` lib name (`[lib] name`, so `use nono::` is unchanged internally), and the `OscarMackJr/nono` GitHub repo all stay as-is; only the `[package] name` registry identities change.
> - **Per-registry idempotency, not a uniform retry loop.** crates.io has no skip flag (dependency-order manual resume + index-visibility polling); PyPI uses `twine upload --skip-existing`; npm is atomic-per-version. A single uniform retry across all three is an anti-feature.
> - **Azure VM = Gen2 + Trusted Launch** (`--enable-secure-boot true --enable-vtpm true`); image `MicrosoftWindowsDesktop:windows-11`, SKU resolved live via `az vm image list-skus` (never hardcoded); ephemeral (create → UAT → teardown), never persistent. Do NOT pull the Trusted Signing cert as a PFX from Key Vault (unsupported; produces an untrusted chain that mimics this bug); no Terraform/Pulumi (repo is `az`-native).
> - **Secret-retirement ordering:** retire the POC signing secrets + delete the smoke workflow ONLY after clean-host UAT (Gate 3) passes — never merely after the Release workflow is green (Gate 2), so a recovery path survives if UAT reveals a problem.
> - **Repo stays PUBLIC** (minifilter altitude received; go-private retired) — no `build_notes/`/`.gsd/` staged before any push. Unlike prior prepare-only tags, v3.5's tag push is *intended* (go-live). All commits DCO-signed.

## v1 Requirements

### Trusted Signing Verify-Gate (SIGN)

- [x] **SIGN-01**: The Azure Trusted Signing certificate profile is confirmed to be type **`PublicTrust`** — the operator runs `az trustedsigning certificate-profile show` and verifies `profileType == PublicTrust`; if it is `PublicTrustTest` (the likely root cause — its issuer naming matches the observed `…Enterprise ID Verified Policy AOC CA…`), a `PublicTrust` profile is created and `TRUSTED_SIGNING_PROFILE` (+ the corrected FIC subject `repo:OscarMackJr/nono:environment:Development`) are updated. The finding is recorded (profile type + issuer chain) so the root cause is documented, not guessed. **CONFIRMED 2026-07-02** via Azure Portal (CLI extension unavailable — corporate-TLS SSL error): live account `ArtifactNono` / RG `RG_Nono` is already `PublicTrust`; no fix needed. The observed `UnknownError` is therefore NOT a profile-type issue — root cause remains open, routed to D-04 chain diagnostics on a future smoke run. See `101-SIGN01-FINDING.md`.
- [x] **SIGN-02**: The CI Authenticode verify is hardened into a single shared `scripts/verify-authenticode.ps1` helper, dot-sourced by both fail-closed verify sites in `release.yml` and the smoke workflow — it adds a `signtool verify /pa /v` deep check, builds/repairs the cert chain and handles CRL/OCSP-revocation transients, and reports chain-build failure distinctly from a genuine untrusted root — WITHOUT ever loosening the fail-closed `Status -ne 'Valid'` contract.
- [ ] **SIGN-03**: The **"Trusted Signing Smoke Test"** workflow runs GREEN on GitHub's clean `windows-latest` runner (Gate 1) — a throwaway exe signs and verifies `Valid` with an issuer chaining to a public `Microsoft ID Verified CS EOC/AOC CA NN` root (not `PublicTrustTest`, not the POC root) — proving the signing path is live end-to-end before any release is cut. **BLOCKED/DEFERRED 2026-07-02**: no OIDC federated credential, no `TRUSTED_SIGNING_ACCOUNT`/`_PROFILE`/`_ENDPOINT` GitHub variables, Plan 02's hardened workflow not yet pushed, and `gh` on this host resolves to `nolabs-ai/nono` not `OscarMackJr/nono` — a dispatch is guaranteed to fail before reaching the verify step, so none was attempted. Unblock preconditions + hand-off to Phase 104 recorded in `101-SIGN03-SMOKE-VERDICT.md`.

### Trusted-Signed Release Go-Live (REL)

- [ ] **REL-01**: The first publicly-trusted-signed release is cut at **`0.66.1`** (Gate 2) — a tag push runs the `Release` workflow to green with all top-level `.exe` (`nono.exe`, `nono-shell-broker.exe`, `nono-wfp-service.exe`) and both MSIs Trusted-Signed and passing the hardened fail-closed verify; the published `OscarMackJr/nono` GitHub Release artifacts show **Verified publisher** (Issuer = `Microsoft ID Verified CS` root, not `CN=nono Test Signing`).

### Fork-Owned Multi-Registry Publish (PUB — FUT-01)

- [ ] **PUB-01**: The published package identities are renamed to fork-owned **`nono-sandbox`** family names — crates.io `nono-sandbox` / `nono-sandbox-proxy` / `nono-sandbox-cli`, PyPI `nono-sandbox`, npm `@oscarmackjr/nono-ts` — via `[package] name` (crates) + binding manifest changes only; the `[[bin]] name = "nono"`, `[lib] name = "nono"`, and the `OscarMackJr/nono` repo are unchanged, internal path-dep names/pins are reconciled, each new name's availability is confirmed on its registry, and the workspace + both binding builds are green.
- [ ] **PUB-02**: `0.66.1` is published **LIVE** to all three registries under the fork-owned identities — crates.io in dependency order (`nono-sandbox` → `nono-sandbox-proxy` → `nono-sandbox-cli`) with index-visibility polling (not a fixed `sleep`), PyPI via maturin + `twine upload --skip-existing`, npm `@oscarmackjr/nono-ts` (all platform-specific packages present, avoiding the documented missing-platform-package failure) — each registry's own idempotency/resume semantics respected; `cargo install nono-sandbox-cli`, `pip install nono-sandbox`, and `npm i @oscarmackjr/nono-ts` all resolve post-publish.

### Clean-Host UAT — Azure VM (CHOST — FUT-03)

- [ ] **CHOST-01**: A reproducible fresh-Win11 clean-host is stood up from fork-owned IaC (`az` CLI / Bicep under `scripts/azure/`) — a Gen2 + Trusted-Launch (vTPM) `windows-11` VM (SKU resolved live, not hardcoded), never-trusted-POC-cert, no VC++ runtime, RDP-reachable — with a documented create → use → teardown lifecycle (ephemeral, never persistent).
- [ ] **CHOST-02**: Two new unattended `verify-dark.ps1` gates assert clean-host trust and plug into the existing gate-discovery harness (emitting `SKIP_HOST_UNAVAILABLE` when no clean host is present): a `trusted-signed-assertion` gate (Authenticode `Valid`, Issuer = `Microsoft ID Verified CS` root, reusing the SIGN-02 shared helper) and a **self-contained** `broker-spawn-on-clean-host` gate (install → `nono run --profile claude-code` spawns the broker with NO manual cert import → uninstall; ordered-safe under a `-All` sweep).
- [ ] **CHOST-03**: On the Azure VM, both host-gated distribution todos are drained to a genuine PASS (Gate 3) — `poc-cert-broker-clean-host` (the broker spawns out-of-box on the trusted-signed `0.66.1` release) via CHOST-02, and `msi-vcredist-prereq` (the machine MSI installs on fresh Win11 with no VC++ redist — no `1603`/rollback, `nono.exe` launches, no `0xC0000135`) via the existing `clean-host-install.ps1` gate — and both todos are moved `pending/` → `resolved/`.

### Close-Out (CLOSE)

- [ ] **CLOSE-01**: The POC signing path is retired **only after** clean-host UAT (Gate 3) passes — POC secrets (`WINDOWS_SIGNING_CERT` / `WINDOWS_SIGNING_CERT_PASSWORD`) deleted, `trusted-signing-smoke.yml` removed, the stale `docs/cli/development/windows-signing-guide.mdx` corrected to point at the go-live cookbook, and the runbook + STATE updated to reflect FUT-01/FUT-02/FUT-03 + DIST-SIGN-01 cleared.

## v2 / Future Requirements

Tracked, not in this roadmap.

- **FUT-04**: Native cross-target clippy run in a hosted CI matrix as the *enforcing* gate (complementary to the local toolchain).
- **FUT-05**: Next upstream-sync cadence past `nolabs-ai/nono` v0.66.0 (a future UPST cycle).
- **FUT-06**: SmartScreen reputation accrual for the new signing identity — time/volume-based, not gateable; monitor first-run "Windows protected your PC" behavior as it fades.
- **FUT-07**: Re-consolidate published package identity with upstream (co-owner grant) if/when the fork and upstream align — supersedes the `nono-sandbox` rename.

## Out of Scope

Explicit exclusions, documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Publishing under the upstream-owned `nono`/`nono-py`/`nono-ts` identities | Owned by the upstream maintainer (lukehinds); the fork publishes under fork-owned `nono-sandbox` names (PUB-01). |
| Pulling the Trusted Signing cert as a PFX from Key Vault | Microsoft-unsupported; produces an untrusted-root chain that mimics the exact `UnknownError` bug. |
| Terraform / Pulumi for the Azure VM | The repo is `az`-native; a second IaC toolchain is unjustified for one ephemeral VM. |
| A persistent Azure VM | The clean-host VM is ephemeral (create → UAT → teardown); a persistent VM re-contaminates the trust store and costs. |
| Loosening the fail-closed `-ne 'Valid'` verify to accept `UnknownError` | Security regression, not a fix — the gate stays fail-closed (SIGN-02). |
| Gating on SmartScreen reputation | Volume/time-based with no EV bypass (Artifact Signing issues no EV certs); documented as an accepted first-run cost (FUT-06). |
| Upstream sync past v0.66.0 | Out of this milestone's window (FUT-05). |
| Renaming the `nono` binary / product / GitHub repo | Only the registry `[package]` identities change; `[[bin]] name`, `[lib] name`, and `OscarMackJr/nono` stay as-is. |

## Traceability

Phase numbering continues from Phase 100 → Phase 101+.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SIGN-01 | Phase 101 | Complete |
| SIGN-02 | Phase 101 | Complete |
| SIGN-03 | Phase 101 | Blocked/Deferred |
| REL-01 | Phase 104 | Pending |
| PUB-01 | Phase 102 | Pending |
| PUB-02 | Phase 105 | Pending |
| CHOST-01 | Phase 103 | Pending |
| CHOST-02 | Phase 103 | Pending |
| CHOST-03 | Phase 106 | Pending |
| CLOSE-01 | Phase 107 | Pending |
</content>
