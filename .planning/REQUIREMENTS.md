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
- [ ] **SIGN-03**: The **"Trusted Signing Smoke Test"** workflow runs GREEN on GitHub's clean `windows-latest` runner (Gate 1) — a throwaway exe signs and verifies `Valid` with an issuer chaining to a public `Microsoft ID Verified CS EOC/AOC CA NN` root (not `PublicTrustTest`, not the POC root) — proving the signing path is live end-to-end before any release is cut. **FAILED 2026-07-03 (RED, diagnosed)**: operator authorized push + live dispatch; branch pushed, workflow dispatched twice. Run `28636000664` failed on a real wiring defect (missing `actions/checkout`, fixed in `83eefe11`). Run `28636133664` (authoritative) ran the real hardened verify path and still FAILED: `Status: UnknownError`, issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 02`, on a genuinely-signed `PublicTrust`-profile binary — the fail-closed gate correctly retried the transient case and then refused to pass (no loosening). This also **disproves** the research's issuer-naming heuristic (`Enterprise ID Verified Policy` is NOT a reliable `PublicTrustTest` tell — `PublicTrust` chains through it too). Root cause is a runner-side chain-build/revocation-validation issue on `windows-latest`, not a profile-type or gate defect. Full evidence + 3 findings + Phase 104 hand-off recorded in `101-SIGN03-SMOKE-VERDICT.md`.

### Trusted-Signed Release Go-Live (REL)

- [ ] **REL-01**: The first publicly-trusted-signed release is cut at **`0.66.1`** (Gate 2) — a tag push runs the `Release` workflow to green with all top-level `.exe` (`nono.exe`, `nono-shell-broker.exe`, `nono-wfp-service.exe`) and both MSIs Trusted-Signed and passing the hardened fail-closed verify; the published `OscarMackJr/nono` GitHub Release artifacts show **Verified publisher** — `Get-AuthenticodeSignature.Status -eq 'Valid'` plus a non-test signer (rejecting `CN=nono Test Signing`/`PublicTrustTest`), with the issuer captured informationally only, never gated on an issuer-substring match (per `101-SIGN03-SMOKE-VERDICT.md` Finding B, which disproved the prior issuer-naming heuristic: a genuine `PublicTrust` signature can chain through `Microsoft Enterprise ID Verified Policy AOC CA NN`).

### Fork-Owned Multi-Registry Publish (PUB — FUT-01)

- [x] **PUB-01**: The published package identities are renamed to fork-owned **`nono-sandbox`** family names — crates.io `nono-sandbox` / `nono-sandbox-proxy` / `nono-sandbox-cli`, PyPI `nono-sandbox`, npm `@oscarmackjr/nono-ts` — via `[package] name` (crates) + binding manifest changes only; the `[[bin]] name = "nono"`, `[lib] name = "nono"`, and the `OscarMackJr/nono` repo are unchanged, internal path-dep names/pins are reconciled, each new name's availability is confirmed on its registry, and the workspace + both binding builds are green. *(Plans 102-01/102-02/102-03/102-04 of 5 complete 2026-07-03: crates.io rename `[package] name` + `package=` reconciliation done, workspace `cargo build --workspace --all-targets` green, all 5 registry names live-confirmed available; Makefile + 4 permanent CI workflows reconciled to the renamed `-p nono-sandbox`/`-p nono-sandbox-cli` selectors, `make build`'s constituent commands + `cargo fmt --all -- --check` green; `../nono-py` Cargo.toml/pyproject.toml patched with `package=`/`[project] name = "nono-sandbox"`, `maturin build` green, `cargo tree` confirms resolution, DCO-signed commit `787e2dd` in the nono-py repo; `../nono-ts` Cargo.toml patched with `package=`, npm identity hand-rescoped to `@oscarmackjr/nono-ts` across package.json + all 4 platform subpackages + optionalDependencies (installed `napi rename` tool proved unusable — non-interactive-incapable and would have corrupted `nono-ts`'s own crate name), `napi build --platform --release` green, `cargo tree` confirms resolution, DCO-signed commit `c2f5aaa` in the nono-ts repo. Plan 102-05 (phase gate) closed 2026-07-03: fresh 5-way registry re-check (all 404, no same-window squat) + `cargo build --workspace --all-targets` + both `make build` constituent commands + `maturin build` + `napi build --platform --release` all green + all 4 SC1-SC4 confirmed against live state across all 3 repos. Phase 102 COMPLETE.)*
- [ ] **PUB-02**: `0.66.1` is published **LIVE** to all three registries under the fork-owned identities — crates.io in dependency order (`nono-sandbox` → `nono-sandbox-proxy` → `nono-sandbox-cli`) with index-visibility polling (not a fixed `sleep`), PyPI via maturin + `twine upload --skip-existing`, npm `@oscarmackjr/nono-ts` (all platform-specific packages present, avoiding the documented missing-platform-package failure) — each registry's own idempotency/resume semantics respected; `cargo install nono-sandbox-cli`, `pip install nono-sandbox`, and `npm i @oscarmackjr/nono-ts` all resolve post-publish.

### Clean-Host UAT — Azure VM (CHOST — FUT-03)

- [x] **CHOST-01**: A reproducible fresh-Win11 clean-host is stood up from fork-owned IaC (`az` CLI / Bicep under `scripts/azure/`) — a Gen2 + Trusted-Launch (vTPM) `windows-11` VM (SKU resolved live, not hardcoded), never-trusted-POC-cert, no VC++ runtime, RDP-reachable — with a documented create → use → teardown lifecycle (ephemeral, never persistent).
- [x] **CHOST-02**: Two new unattended `verify-dark.ps1` gates assert clean-host trust and plug into the existing gate-discovery harness (emitting `SKIP_HOST_UNAVAILABLE` when no clean host is present): a `trusted-signed-assertion` gate (Authenticode `Valid`, Issuer = `Microsoft ID Verified CS` root, reusing the SIGN-02 shared helper) and a **self-contained** `broker-spawn-on-clean-host` gate (install → `nono run --profile claude-code` spawns the broker with NO manual cert import → uninstall; ordered-safe under a `-All` sweep).
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
| SIGN-03 | Phase 101 | Failed/Deferred |
| REL-01 | Phase 104 | In Progress (Plans 01-02/3 done: D-04 flush fix + NoCheck-mode classification landed; publish-crates neutralized (Pitfall 104-A closed) + regression guard + local root pre-check added + REL-01/SC3 wording corrected; SC1-SC3 pending operator-in-loop Plan 03) |
| PUB-01 | Phase 102 | Complete (5/5 plans done, phase gate PASSED 2026-07-03) |
| PUB-02 | Phase 105 | Pending |
| CHOST-01 | Phase 103 | Complete |
| CHOST-02 | Phase 103 | Complete |
| CHOST-03 | Phase 106 | Pending |
| CLOSE-01 | Phase 107 | Pending |

---

# ══════════════════════════════════════════════════════════════════
# Requirements: nono v3.6 — UPST12: Upstream Sync v0.66.0 → v0.69.0
# ══════════════════════════════════════════════════════════════════

**Defined:** 2026-07-28
**Parallel to:** v3.5 (HELD on the external Azure Trusted-Signing root-propagation block; engineering proceeds on v3.6 meanwhile — operator decision 2026-07-28). Both milestones are simultaneously active; v3.5 owns phases 101–107, v3.6 owns phases 108–111.
**Core Value:** Keep the fork current with upstream `nolabs-ai/nono` without regressing the Windows security model or the ADR-86 policy-free-library boundary — a routine drain-then-sync (mirrors v3.1/v3.3/v3.4).

**Scope:** absorb the cross-platform delta from upstream `v0.66.0..v0.69.0` (v0.67.0, v0.67.1, v0.68.0, v0.69.0) — proxy/network features, profile/policy features (incl. `platform_overrides`), macOS Seatbelt carry, and resource-CLI alignment — then leapfrog all 6 workspace crates + both binding repos to `0.70.0` (prepare-only). **Explicitly EXCLUDES** the `tool-sandbox/` subsystem and its refinements: upstream introduced it in v0.65.0 (PR #1105) and the fork never absorbed it — it is a standing structural divergence handled by a dedicated later milestone (v3.7 Windows Tool-Sandbox Parity), not this routine sync. Scope source: quick task `260727-jkn` (`.planning/quick/260727-jkn-map-macos-0-69-parity-gap-phases-for-the/260727-jkn-PLAN.md`).

> **Architecture invariants:**
> - **tool-sandbox is OUT.** The 7 tool-sandbox refinement PRs in this window (#1280/#1322/#1325/#1384/#1394/#1413/#1417) ride on a subsystem the fork does not have; they are recorded DEFERRED→v3.7 in the ledger, not absorbed here.
> - **Never regress the Windows security model** (AppContainer / Low-IL / WFP / Job-Object) or the ADR-86 policy-free-library boundary. Verifier + code review + both cross-target clippy gates are mandatory before close.
> - **Cross-target clippy is MUST** for cfg-gated Unix edits — `cross` (linux-gnu) + `cargo-zigbuild` (apple-darwin), both GREEN locally, no PARTIAL→CI (retired in v3.3 Phase 96). `make ci` (clippy+fmt+tests), not clippy-only.
> - **`platform_overrides` is the intended vehicle for Windows divergence** — migrating the fork's `windows_low_il_broker`/`windows_interpreters` top-level flags into it retires flag-sprawl; keep back-compat aliases.
> - **WFP expresses port ranges natively** — the Windows remote-port-range emitter is simpler than macOS Seatbelt's per-port unroll; keep discrete-list back-compat.
> - **Binding struct-drift is caught only by building** — re-run `maturin build` (nono-py) + `napi build` (nono-ts) after any nono-proxy struct touch ([[project_v34_opened]] durable).
> - **Release scope = PREPARE ONLY** — leapfrog `0.70.0` (collision-free above upstream 0.69.0), no operator push. Repo PUBLIC; no `build_notes/`/`.gsd/` staged. DCO-signed.

## v3.6 Requirements

### UPST12 Divergence Audit (UPST12)
- [x] **UPST12-01**: An authoritative `108-DIVERGENCE-LEDGER.md` for upstream `v0.66.0..v0.69.0` exists — every substantive commit classified (adopt / adapt / skip / split) with a `windows-touch` flag and ADR-review verdict; re-export surfaces diff-inspected (not just `--name-only`, per the cluster-isolation-can-be-empirically-false lesson); and the 7 tool-sandbox refinement PRs (#1280/#1322/#1325/#1384/#1394/#1413/#1417) explicitly recorded DEFERRED→v3.7.

### Proxy / Network Absorb (NET)
- [ ] **NET-01**: `deny_domain` deny-list network filtering (#1374) is absorbed into the proxy filter + profile schema, composing correctly with the fork's existing `allow_domain` allowlist model without weakening default-deny.
- [ ] **NET-02**: SPIFFE/SPIRE workload-identity auth for upstream routes (#1272) is absorbed. **→ MOVED to Phase 113 (2026-07-29)**: `c831dade` measured 4354 ins / 545 del / 33 files (57% of the original Phase 109), crosses the ADR-86 boundary, rewrites fork-divergent `tls_intercept`/`reverse.rs`, and expands the dependency surface by ~633 lockfile lines — it gets its own ADR-gated phase.
- [ ] **NET-03**: The SigV4 encoded-URI generation fix (#1430) and the sibling-route cross-deny fix (#1437) are absorbed; HTTP/2 injection, `HTTP_PROXY` forward-proxy, and `no_proxy` bypass are verified non-regressed; `maturin` + `napi` binding builds are green.

### Profile / Policy Absorb (PROF)
- [x] **PROF-01**: `platform_overrides` per-OS profile patching (#1371) is absorbed and preserved through `extends` resolution (#1380); the fork's `windows_low_il_broker` and `windows_interpreters` top-level flags are migrated into `platform_overrides.windows` with back-compat aliases.
- [x] **PROF-02**: `$VAR` process-env token expansion (#1296) and `@git:*` dynamic token expansion (#1298) in profile filesystem paths are absorbed.
- [ ] **PROF-03**: The port-range profile schema (#1398) is absorbed with a WFP-native remote-port-range emitter on Windows and discrete-list back-compat.
- [ ] **PROF-04**: The `bun` (#1305) and `mise` (#1387) runtime presets are absorbed.

### macOS / Core Carry + Resource CLI (CORE)
- [ ] **CORE-01**: The macOS Seatbelt/core carry lands as-is for cross-target parity — `~/.cache` grant (#1378) and `MAX_CRYPTO_THREADS`=12 libdispatch tuning (#1424). **AMENDED 2026-07-30:** the `macos.rs` port-range emitter (#1398) is REMOVED from this requirement — Phase 110 absorbs commit `d5803b99` in full (schema + `capability.rs` + both Unix emitters + the WFP-native Windows emitter) so the port-range feature is coherent and testable in one phase. Phase 111 must not re-absorb it.
- [ ] **CORE-02**: The upstream resource-limit CLI surface (`--memory` / `--max-processes`, #1269/#1403) is reconciled with the fork's existing kernel-enforced Job Object implementation — flag names/semantics aligned, no new enforcement, no regression to `--cpu-percent`/`--timeout`.

### Security + Residual Sync (SEC / RES) — Phase 112
> **Added 2026-07-29** by operator approval of the Phase 108 ledger's D-18/D-19 coverage-gap amendment. Phase 108 measured **27 hand-verified CODE commits** in the `v0.66.0..v0.69.0` window mapping to none of v3.6's original 12 requirements. Work-list: `108-DIVERGENCE-LEDGER.md` §"Requirement Coverage Gap" + the `security-residual-and-misc` cluster.

- [ ] **SEC-01**: AWS SigV4 authentication for the MiTM proxy (#1195, `0ecc476b`) is absorbed.
- [ ] **SEC-02**: Declarative sandboxed OAuth capture (`9b692e07`), capture-boundary hardening (`3c59c62e`), and the stdin fixture test (`d033c631`) are absorbed.
- [ ] **SEC-03**: NVIDIA procfs mediation hardening (#1284, `a3243907`) is absorbed without regressing the fork's own GPU tests.
- [ ] **SEC-04**: The trust-policy `predicate` field distinguishing nono trust policies from foreign JSON (#1333, `f943fb5a`) is absorbed.
- [ ] **SEC-05**: The Linux execute-restriction `Refer` grant (#1397, `d84b4818`) is absorbed; cross-target clippy GREEN.
- [ ] **SEC-06**: Seccomp supervisor-ancestry for orphaned descendants (#1401, `ac5ccd70`) is absorbed.
- [ ] **SEC-07**: The standalone `nono proxy` command (#1261, `2663e990`) is absorbed.
- [ ] **SEC-08**: The `allow_vars` empty-list env-strip fix (#1204, `a5a441c2`) is absorbed.
- [ ] **SEC-09**: The credential-broker non-shim-entry guard relaxation (#1301, `f6f02751`) is absorbed. *(Found security-relevant during Phase 108's Cluster Summary rollup — not a D-18-named anchor.)*
- [ ] **RES-01**: The registry/update-check header residual (#1405/#1383/#1386/#1341/#1340) is individually reviewed and absorbed or explicitly skipped with reasoning.
- [ ] **RES-02**: The PTY-teardown (#1258) + test-infra residual is individually reviewed and absorbed or explicitly skipped with reasoning.

### Fork-Invariant Verify + Release (VERIFY / RLS)
- [ ] **VERIFY-01**: Both cross-target clippy gates (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) and `make ci` (clippy+fmt+tests) are GREEN locally, and a fork-invariant pass confirms the Windows security model + ADR-86 boundary are non-regressed.
- [ ] **RLS-14**: All 6 workspace crates + internal path-dep pins + both binding repos (`../nono-py`, `../nono-ts`) are leapfrogged to `0.70.0` (collision-free above upstream 0.69.0), Cargo.lock shows zero unexpected third-party drift, and the prepare-only release gate is GREEN — no operator push.

## v3.6 Out of Scope

| Feature | Reason |
|---------|--------|
| The `tool-sandbox/` subsystem (PR #1105) + its 7 refinement PRs | Standing structural divergence (fork never absorbed the v0.65.0 subsystem); handled by the dedicated v3.7 Windows Tool-Sandbox Parity milestone, not a routine sync. |
| A Windows `tool-sandbox/platform/windows.rs` driver | Belongs to v3.7 (needs the SCM_RIGHTS→handle-passing + peer-auth spike + an adopt-vs-formalize ADR). |
| macOS-only behavioral testing of the carried Seatbelt fixes | The fork ships macOS binaries but validates macOS via cross-target clippy + CI, not a live macOS host (established posture). |
| WFP daemon-path-only reachability hardening / filter-leak fix | Fork-internal hardening surfaced by the audit; tracked separately, not part of the upstream-parity sync. |
| Any operator push / live registry publish of `0.70.0` | Prepare-only (mirrors v3.1/v3.3/v3.4); live publish is a separate operator-gated step. |

## v3.6 Traceability

Phase numbering continues (v3.5 owns 101–107) → v3.6 owns **Phases 108–113**.

| Requirement | Phase | Status |
|-------------|-------|--------|
| UPST12-01 | Phase 108 | Complete |
| NET-01 | Phase 109 | Pending |
| NET-02 | Phase 113 | Pending |
| NET-03 | Phase 109 | Pending |
| PROF-01 | Phase 110 | Complete |
| PROF-02 | Phase 110 | Complete |
| PROF-03 | Phase 110 | Pending |
| PROF-04 | Phase 110 | Pending |
| CORE-01 | Phase 111 | Pending |
| CORE-02 | Phase 111 | Pending |
| VERIFY-01 | Phase 111 | Pending |
| RLS-14 | Phase 111 | Pending |
| SEC-01 | Phase 112 | Pending |
| SEC-02 | Phase 112 | Pending |
| SEC-03 | Phase 112 | Pending |
| SEC-04 | Phase 112 | Pending |
| SEC-05 | Phase 112 | Pending |
| SEC-06 | Phase 112 | Pending |
| SEC-07 | Phase 112 | Pending |
| SEC-08 | Phase 112 | Pending |
| SEC-09 | Phase 112 | Pending |
| RES-01 | Phase 112 | Pending |
| RES-02 | Phase 112 | Pending |
</content>
