---
milestone: v3.6
milestone_name: "UPST12: Upstream Sync v0.66.0 -> v0.69.0"
status: active
parallel_milestone: v3.5
parallel_milestone_name: Trusted Signing Go-Live + First Distributed Release
parallel_milestone_status: held-on-external-azure-block
updated: 2026-07-28
---

# Roadmap: nono

## Milestones

- 🔄 **v3.6 UPST12 Upstream Sync (v0.66.0→v0.69.0)** — Phases 108-114 (active 2026-07-28, parallel to v3.5)
- 🔄 **v3.5 Trusted Signing Go-Live + First Distributed Release** — Phases 101-107 (active 2026-07-02, HELD on external Azure block)
- ✅ **v3.4 UPST11 Upstream Sync to v0.66.0 + Release-Reconcile** — Phases 98-100 (shipped 2026-07-02) — [archive](milestones/v3.4-ROADMAP.md)
- ✅ **v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release** — Phases 94-97 (shipped 2026-06-26) — [archive](milestones/v3.3-ROADMAP.md)
- ✅ **v3.2 Signed Policy Overrides (ZT-Infra Attestation)** — Phases 91-93 (shipped 2026-06-23) — [archive](milestones/v3.2-ROADMAP.md)
- ✅ **v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain** — Phases 85-90 (shipped 2026-06-21) — [archive](milestones/v3.1-ROADMAP.md)
- ✅ **v3.0 Enterprise Hardening I — Deploy · Control · Compliance** — Phases 82-84 (shipped 2026-06-19) — [archive](milestones/v3.0-ROADMAP.md)
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18) — [archive](milestones/v2.13-ROADMAP.md)

> Earlier milestones (v2.5–v2.12) are archived under `.planning/milestones/`.

## Phases

<details open>
<summary>🔄 v3.6 UPST12 Upstream Sync v0.66.0→v0.69.0 (Phases 108-114) — ACTIVE (parallel to v3.5)</summary>

Drain-then-sync upstream milestone (mirrors v3.1/v3.3/v3.4), running **in parallel** with the operator-blocked v3.5. Absorb the cross-platform delta from `nolabs-ai/nono` `v0.66.0..v0.69.0` (v0.67.0/.1, v0.68.0, v0.69.0) — proxy/network (`deny_domain`, SPIFFE/SPIRE, SigV4 + sibling-route fixes), profile/policy (`platform_overrides` + migrate the fork's `windows_*` flags into it, `$VAR`/`@git` tokens, port-range schema with a WFP-native emitter, bun/mise presets), macOS Seatbelt carry, and resource-CLI alignment onto the existing Job Object impl — WITHOUT regressing the Windows security model or the ADR-86 boundary, then leapfrog all 6 crates + both binding repos to **`0.70.0`** (prepare-only). **Explicitly EXCLUDES** the `tool-sandbox/` subsystem (PR #1105, introduced v0.65.0, never absorbed — a standing structural divergence deferred to the dedicated **v3.7 Windows Tool-Sandbox Parity** milestone). Scope source: quick `260727-jkn`.

- [x] **Phase 108: UPST12 Divergence Audit** — 5/5 plans
- [x] **Phase 109: Proxy/Network Absorb** — 5/5 plans
- [x] **Phase 110: Profile/Policy Absorb + platform_overrides** — 8/8 plans, all 4 requirements complete; 110-06's live-kernel checkpoint (PROF-03e) resolved 2026-08-04
- [x] **Phase 111: Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog** — 6/6 plans
- [ ] **Phase 112: Security + Residual Sync** — 0/8 plans
- [ ] **Phase 113: SPIFFE/SPIRE Workload Identity** — 0/? plans
- [ ] **Phase 114: OAuth Capture Absorb (SEC-02)** — 0/? plans — carved out of Phase 112 by operator decision 2026-08-05 (ROADMAP Amendment)

</details>

<details open>
<summary>🔄 v3.5 Trusted Signing Go-Live + First Distributed Release (Phases 101-107) — ACTIVE (HELD on external Azure block)</summary>

Full go-live EXECUTE milestone (operator-in-loop): harden the CI Authenticode verify-gate and resolve the Azure Trusted Signing `UnknownError`, rename the fork's published package identities to fork-owned `nono-sandbox` names, stand up ephemeral Azure Win11 VM IaC + new clean-host gates, cut the first publicly-trusted-signed `0.66.1` release, publish it live to crates.io/PyPI/npm, drain both host-gated clean-host UAT todos on the real VM, then retire the POC signing path. Hard dependency spine: verify-gate hardening (101) gates the release cut (104); the rename (102) and the Azure IaC+gates (103) are parallelizable with 101/104; live publish (105) needs both the rename and a real release; clean-host UAT (106) needs the real release (not the live publish); close-out (107) is strictly gated on clean-host UAT PASS, never merely on a green release.

- [ ] **Phase 101: Verify-Gate Hardening + Azure Profile Confirmation** — 4/4 plans (SIGN-03 FAILED/Deferred, RED — live-dispatched and diagnosed 2026-07-03, not fully satisfied)
- [x] **Phase 102: Fork-Owned Package Rename** — 5/5 plans
- [x] **Phase 103: Azure Clean-Host VM IaC + New Verify-Dark Gates** — 3/3 plans
- [ ] **Phase 104: Smoke Green + Cut the Trusted-Signed Release** — 1/3 plans
- [ ] **Phase 105: Live Multi-Registry Publish** — 0/5 plans
- [ ] **Phase 106: Azure VM Clean-Host UAT** — 0/? plans
- [ ] **Phase 107: Close-Out** — 0/? plans

</details>

<details>
<summary>✅ v3.4 UPST11 Upstream Sync to v0.66.0 + Release-Reconcile (Phases 98-100) — SHIPPED 2026-07-02</summary>

Drain-then-sync upstream milestone: audited and absorbed the `nolabs-ai/nono` `v0.65.1..v0.66.0` window (19 PRs → 14 substantive commits / 8 clusters) without regressing the Windows security model or the ADR-86 policy-free-library boundary; settled the high-conflict #1225 `NetworkIntent` refactor via **ADR-98 full-sync-adopt** (CLI-side only). Reconciled the prepare-only release pipeline (ADR-100 ADAPT of upstream CI #1245/#1251), closed the carried-forward nono-py `RouteConfig` PyPI blocker, and leapfrogged all 6 workspace crates + both binding repos to **`0.66.1`** — one collision-free step above upstream's own `0.66.0`, operator-push-ready. Both cross-target clippy gates GREEN locally (no PARTIAL→CI). Release scope = PREPARE ONLY. Full detail: [milestones/v3.4-ROADMAP.md](milestones/v3.4-ROADMAP.md).

- [x] Phase 98: UPST11 Divergence Audit (4/4 plans) — completed 2026-06-30
- [x] Phase 99: Upstream Absorb + Fork-Invariant Verify (7/7 plans) — completed 2026-06-30
- [x] Phase 100: Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker (5/5 plans) — completed 2026-07-02

</details>

<details>
<summary>✅ v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release (Phases 94-97) — SHIPPED 2026-06-26</summary>

Drain-then-sync upstream milestone: audited and absorbed the relocated `nolabs-ai/nono` `v0.64.0..v0.65.1` window (8 commits / 4 clusters) without regressing the Windows security model — verifier + code review + a new local clippy gate caught and closed four fork-invariant regressions the cherry-pick introduced. Stood up a local cross-target clippy toolchain (Docker `cross` for linux-gnu, zig `cargo-zigbuild` for apple-darwin), both GREEN locally, retiring the chronic PARTIAL→CI default. Leapfrogged the tree to **0.66.0** (first SemVer > upstream 0.65.1) and made the workspace genuinely releasable for the first time: a gated build+sign MSI pipeline, a 3-registry dry-run orchestrator, an auto-discovered release-readiness gate (PASS), and an operator one-step-push runbook. Release scope = PREPARE ONLY (the actual push/publish is an operator-gated manual step outside the milestone). Full detail: [milestones/v3.3-ROADMAP.md](milestones/v3.3-ROADMAP.md).

- [x] Phase 94: UPST10 Divergence Audit (2/2 plans) — completed 2026-06-26
- [x] Phase 95: Upstream Absorb + Fork-Invariant Verify (7/7 plans) — completed 2026-06-26
- [x] Phase 96: Cross-Target Toolchain (3/3 plans) — completed 2026-06-26
- [x] Phase 97: Release Engineering — Leapfrog + Pipeline + Runbook (4/4 plans) — completed 2026-06-26

</details>

<details>
<summary>✅ v3.2 Signed Policy Overrides (ZT-Infra Attestation) (Phases 91-93) — SHIPPED 2026-06-23</summary>

Replaced the "just disable the sandbox" temptation with cryptographically-signed, ledger-logged policy exceptions: a developer who hits a false-positive block obtains an authorized, scoped, expiring signed override that the `nono-py` binding verifies against the ZT-Infra v2 control plane and applies as a temporary, auditable, revocable expansion — non-self-service. Delivered the **two-key AND gate** (KMS signature verifies offline AND a live `POST /actions` returns `allow`), live-check-as-revocation-point, `AWS_*` credential stripping, async DAAL anchoring, and `nono override request`/`apply` CLI affordances. Closed both carry-forward blockers: VFY-01 clause b (live arm) and VFY-03a (production HKLM trust sourcing). Rust core stayed policy-free (only `AuditEventPayload::PolicyOverrideApplied` + EventIDs 10006-10010); all override logic in `nono-py`. Milestone-marker only — no crate publish (future release leapfrogs ≥ `0.65.0`). Full detail: [milestones/v3.2-ROADMAP.md](milestones/v3.2-ROADMAP.md).

- [x] Phase 91: Signed Override Format + Verification Core (3/3 plans) — completed 2026-06-22
- [x] Phase 92: Runtime CapabilitySet Mutation + Audit Wiring (4/4 plans) — completed 2026-06-22
- [x] Phase 93: Live ZT-Infra Integration + Revocation + Request Flow (6/6 plans) — completed 2026-06-23

</details>

<details>
<summary>✅ v3.1 UPST9 Upstream Sync + v3.0 Drain (Phases 85-90) — SHIPPED 2026-06-21</summary>

Drain-then-sync upstream milestone: absorbed `always-further/nono` `v0.62.0..v0.64.0` (90 commits / 140 files) converging toward upstream's layout (audit stack + structured diagnostics relocated into the core `nono` crate) without regressing the Windows security model, then drained v3.0's host-gated UAT debt. Milestone-marker only — no crate publish; a future release leapfrogs the crate version to ≥ `0.65.0`. Full detail: [milestones/v3.1-ROADMAP.md](milestones/v3.1-ROADMAP.md).

- [x] Phase 85: UPST9 Divergence Audit (1/1 plans) — completed 2026-06-19
- [x] Phase 86: Library-Boundary Convergence (3/3 plans) — completed 2026-06-20
- [x] Phase 87: Security Sync (3/3 plans) — completed 2026-06-20
- [x] Phase 88: Feature + Dependency Cherry-Pick Wave (6/6 plans) — completed 2026-06-20
- [x] Phase 89: Proxy Hardening Sync (4/4 plans) — completed 2026-06-21
- [x] Phase 90: v3.0 Host-Gated UAT Drain (2/2 plans) — completed 2026-06-21

</details>

## Phase Details

### Phase 101: Verify-Gate Hardening + Azure Profile Confirmation
**Goal**: The Trusted Signing verify path is provably fixed — root cause disambiguated and documented, the fail-closed verify hardened without ever loosening it — and the smoke workflow runs GREEN on GitHub's clean `windows-latest` runner, proving the signing chain is live before any release is cut.
**Depends on**: Nothing (first phase of v3.5)
**Requirements**: SIGN-01, SIGN-02, SIGN-03
**Success Criteria** (what must be TRUE):
  1. **[Operator-in-loop]** `az trustedsigning certificate-profile show` confirms `profileType == PublicTrust` (or the operator creates a `PublicTrust` profile and corrects `TRUSTED_SIGNING_PROFILE` + the FIC subject `repo:OscarMackJr/nono:environment:Development`); the finding (profile type + issuer chain) is documented, not guessed.
  2. A shared `scripts/verify-authenticode.ps1` helper exists and is dot-sourced by both fail-closed verify sites in `release.yml` (~line 259, ~281-321) and `trusted-signing-smoke.yml` (~line 62-73), adding a `signtool verify /pa /v` deep-check fallback and classifying chain-build failure distinctly from a genuine untrusted root.
  3. The fail-closed `Status -ne 'Valid'` contract is provably unweakened — diff review confirms the condition itself is unchanged; all new logic is additive corroboration/diagnosis, not a loosened pass condition.
  4. The **"Trusted Signing Smoke Test"** workflow runs GREEN on `windows-latest` (Gate 1) — a throwaway exe signs and Authenticode-verifies `Valid` with an issuer chaining to a public `Microsoft ID Verified CS EOC/AOC CA NN` root (not `PublicTrustTest`, not the POC root).
**Plans**:
- [x] 101-01-PLAN.md — Build scripts/verify-authenticode.ps1 (Assert-TrustedSignature: D-01 AND-gate, D-02 retry, D-03 modes, D-04 diagnostics) + test harness (TDD)
- [x] 101-02-PLAN.md — Wire the shared helper into release.yml (2 sites) + trusted-signing-smoke.yml (1 site), preserving the fail-closed condition and .sys carve-out
- [x] 101-03-PLAN.md — Operator checkpoint: confirm/fix the Azure Trusted Signing certificate profile type (SIGN-01), document the finding — confirmed PublicTrust (profile `NonoCertProfile`), no fix needed; UnknownError root cause open (not profile-type); GitHub Trusted Signing config originally reported absent, later CORRECTED 2026-07-02 (config confirmed to already exist and work — see `101-SIGN01-FINDING.md`)
- [x] 101-04-PLAN.md — Dispatch the Trusted Signing Smoke Test on windows-latest, assert GREEN with correct issuer chain (SIGN-03) — **live-dispatched 2026-07-03** (operator authorized push + dispatch): Run `28636000664` failed on a real missing-`actions/checkout` wiring defect (fixed `83eefe11`); authoritative Run `28636133664` ran the hardened verify path and FAILED (`Status:UnknownError`, issuer `Microsoft Enterprise ID Verified Policy AOC CA 02`) — fail-closed gate correctly refused to pass; disproves the research issuer-naming heuristic. Verdict rewritten BLOCKED → **FAILED (RED, diagnosed)** in `101-SIGN03-SMOKE-VERDICT.md`; SIGN-03 hands off to Phase 104 for root-cause fix

### Phase 102: Fork-Owned Package Rename
**Goal**: The published package identities are renamed to fork-owned `nono-sandbox` family names across all three registries, with the `nono` binary/lib/repo names left unchanged, so a later live publish (Phase 105) has an unblocked, owned target.
**Depends on**: Nothing (parallelizable with Phase 101 — independent of signing)
**Requirements**: PUB-01
**Success Criteria** (what must be TRUE):
  1. crates.io `[package] name` is renamed on the 3-crate publish set: `nono` → `nono-sandbox`, `nono-proxy` → `nono-sandbox-proxy`, `nono-cli` → `nono-sandbox-cli`; internal path-dependency names/pins are reconciled across the workspace; `[[bin]] name = "nono"` and `[lib] name = "nono"` are unchanged (`use nono::` internal imports untouched).
  2. The `nono-py` binding's PyPI project name is renamed to `nono-sandbox`, and the `nono-ts` binding's npm package name is renamed to the scoped `@oscarmackjr/nono-ts` — in both sibling binding repos.
  3. Each new registry identity's availability is confirmed live against the actual registry (crates.io `nono-sandbox`/`nono-sandbox-proxy`/`nono-sandbox-cli`, PyPI `nono-sandbox`, npm `@oscarmackjr/nono-ts`) before committing to the rename.
  4. The workspace build (`make build`) and both binding builds (`maturin build`, napi build) are green under the new names.
**Plans**: 5 plans
- [x] 102-01-PLAN.md — Live registry-availability gate + rename the 3-crate publish set (nono/nono-proxy/nono-cli) + patch remaining in-workspace dependents + regenerate Cargo.lock
- [x] 102-02-PLAN.md — Reconcile Makefile + permanent CI workflow package selectors (release.yml publish steps explicitly deferred to Phase 105)
- [x] 102-03-PLAN.md — nono-py sibling rename (Cargo.toml package= fix + pyproject.toml PyPI name + maturin build + DCO commit)
- [x] 102-04-PLAN.md — nono-ts sibling rename (Cargo.toml package= fix + napi rename --package-name + napi build + DCO commit)
- [x] 102-05-PLAN.md — Phase gate: fresh registry re-check + make ci + consolidated SC1-SC4 verification across all 3 repos

### Phase 103: Azure Clean-Host VM IaC + New Verify-Dark Gates
**Goal**: A reproducible, fork-owned Azure Win11 clean-host VM can be stood up and torn down on demand, and two new self-contained `verify-dark.ps1` gates exist to assert trusted-signed status and out-of-box broker spawn on it — both authored and provably `SKIP_HOST_UNAVAILABLE` on the dev host, ready to run for-real once a VM and a real release exist.
**Depends on**: Phase 101 (the `trusted-signed-assertion` gate reuses the `verify-authenticode.ps1` shared helper)
**Requirements**: CHOST-01, CHOST-02
**Success Criteria** (what must be TRUE):
  1. `scripts/azure/clean-vm/` contains a Bicep module (`main.bicep`) provisioning a Gen2 + Trusted-Launch (`--enable-secure-boot true --enable-vtpm true`) `MicrosoftWindowsDesktop:windows-11` VM with the SKU resolved live via `az vm image list-skus` (never hardcoded), an NSG scoped to the operator's IP, and deploy/teardown scripts implementing a documented ephemeral create → use → teardown lifecycle (never persistent).
  2. `scripts/gates/trusted-signed-assertion.ps1` exists, reuses the Phase 101 `verify-authenticode.ps1` shared helper to assert Authenticode `Valid` + issuer chaining to the `Microsoft ID Verified CS` root on staged artifacts, and plugs into the existing `verify-dark.ps1` gate-discovery harness with zero harness code changes.
  3. `scripts/gates/broker-spawn-on-clean-host.ps1` exists, is **self-contained** (install → `nono run --profile claude-code` spawns the broker with no manual cert import → uninstall) — not dependent on `-All`'s alphabetical execution order.
  4. Both new gates run on the dev host and correctly return `SKIP_HOST_UNAVAILABLE` (same discipline as the existing `clean-host-install.ps1` precondition), proving they are wired into the harness before any VM exists.
**Plans**: 3 plans
- [x] 103-01-PLAN.md — Bicep IaC (main.bicep + deploy.ps1/teardown.ps1), Gen2+Trusted-Launch, live SKU/IP resolution, ephemeral RG_Nono_CleanHost lifecycle
- [x] 103-02-PLAN.md — Two new verify-dark gates (trusted-signed-assertion.ps1 reusing verify-authenticode.ps1; broker-spawn-on-clean-host.ps1 self-contained), both SKIP-safe on dev host
- [x] 103-03-PLAN.md — Phase gate: bicep lint + both single-gate SKIP checks + full -All sweep + harness-diff regression, consolidated SC1-SC4 verification

### Phase 104: Smoke Green + Cut the Trusted-Signed Release
**Goal**: The first publicly-trusted-signed `0.66.1` release exists on GitHub, with every signed artifact passing the hardened fail-closed verify and showing Verified publisher — the actual go-live moment.
**Depends on**: Phase 101 (SIGN-03 smoke green is a hard precondition — "do not cut a release until smoke is green")
**Requirements**: REL-01
**Success Criteria** (what must be TRUE):
  1. **[Operator-in-loop]** Immediately before the tag push, the operator re-runs the hardened "Trusted Signing Smoke Test" workflow and confirms it is GREEN (the FIC-subject/profile canary re-check, per Pitfall 9's AADSTS700213 recurrence risk).
  2. **[Operator-in-loop]** The operator pushes tag `v0.66.1`; the `Release` workflow runs to green — all top-level `.exe` (`nono.exe`, `nono-shell-broker.exe`, `nono-wfp-service.exe`) and both MSIs pass the hardened fail-closed verify from Phase 101.
  3. The published `OscarMackJr/nono` GitHub Release artifacts show **Verified publisher** — `Get-AuthenticodeSignature.Status -eq 'Valid'` plus a non-test signer (rejecting `CN=nono Test Signing`/`PublicTrustTest`), with the issuer captured informationally only, never gated on an issuer-substring match (per `101-SIGN03-SMOKE-VERDICT.md` Finding B, which disproved the prior issuer-naming heuristic).
**Plans**: 3 plans
- [x] 104-01-PLAN.md — verify-authenticode.ps1 hardening: D-04 diagnostic-flush fix (Write-Error -ErrorAction Continue) + NoCheck-mode untrusted-root classification (Merge-NoCheckOverride), plus 2 new regression-guard test cases
- [x] 104-02-PLAN.md — Neutralize release.yml's stale publish-crates job (if: false, Pitfall 104-A) + local publish-selector regression guard + local root-availability pre-check (certutil) + correct disproven issuer-substring wording in REQUIREMENTS.md/ROADMAP.md — publish-crates job-level `if: false` (regression-proven), certutil pre-check live-run (ABSENT, 554 certs checked, matching research), REL-01/SC3 wording corrected to Status=Valid + non-test-signer condition
- [ ] 104-03-PLAN.md — Operator checkpoint: poll-until-green smoke re-run (SC1) → tag push v0.66.1 → confirm Release workflow green + Verified publisher via the Finding-B-corrected gate (SC2 + SC3)

### Phase 105: Live Multi-Registry Publish
**Goal**: `0.66.1` is live and installable from all three registries under the fork-owned `nono-sandbox` identities.
**Depends on**: Phase 102 (renamed, owned identities) + Phase 104 (a real trusted-signed release exists to publish)
**Requirements**: PUB-02
**Success Criteria** (what must be TRUE):
  1. **[Operator-in-loop]** crates.io publish runs in strict dependency order — `nono-sandbox` → `nono-sandbox-proxy` → `nono-sandbox-cli` — using index-visibility polling (not a fixed `sleep`) to confirm each crate is indexed before the next depends on it.
  2. **[Operator-in-loop]** PyPI publish (`maturin` build + `twine upload --skip-existing`) succeeds for `nono-sandbox`, and the actual published wheel/platform coverage is verified post-publish (not just exit code).
  3. **[Operator-in-loop]** npm publish succeeds for `@oscarmackjr/nono-ts` with all required platform-specific native packages present (explicitly avoiding the documented upstream missing-platform-package failure).
  4. `cargo install nono-sandbox-cli`, `pip install nono-sandbox`, and `npm i @oscarmackjr/nono-ts` all resolve successfully post-publish.
**Plans**: 5 plans
- [ ] 105-01-PLAN.md — crates.io machinery: sparse-index poll helper + workflow_dispatch-only publish-crates.yml (retires the neutralized in-line job)
- [ ] 105-02-PLAN.md — PyPI machinery: twine install + confirm-gated maturin build + twine upload + post-publish coverage verification
- [ ] 105-03-PLAN.md — npm machinery: close the win32-x64-msvc platform-package gap + confirm-gated, platform-first-then-main publish script
- [ ] 105-04-PLAN.md — Verification tooling: pre-checkpoint registry-availability/scope re-check + SC4 isolated post-publish resolve wrapper
- [ ] 105-05-PLAN.md — Operator checkpoint (Wave 2): pre-publish gate, live publish (crates.io/PyPI/npm), post-publish resolve verification

### Phase 106: Azure VM Clean-Host UAT
**Goal**: Both long-standing host-gated distribution todos are drained to a genuine PASS on a real, never-contaminated Azure Win11 VM running the real trusted-signed `0.66.1` release.
**Depends on**: Phase 103 (VM IaC + gates exist) + Phase 104 (a real trusted-signed release exists to install) — does NOT require Phase 105's live registry publish; the GitHub Release artifact is sufficient.
**Requirements**: CHOST-03
**Success Criteria** (what must be TRUE):
  1. **[Operator-in-loop]** The operator deploys the Phase 103 IaC, RDPs into the fresh VM (never touched by the POC cert or corporate trust store), and stages the Phase 104 published Release artifacts.
  2. **[Operator-in-loop]** The `broker-spawn-on-clean-host` gate PASSes — `nono run --profile claude-code` spawns the broker with zero manual cert-trust step.
  3. **[Operator-in-loop]** The reused `clean-host-install` gate PASSes — the machine MSI installs on fresh Win11 with no VC++ redist preinstalled (no `1603`/rollback, `nono.exe` launches, no `0xC0000135`).
  4. Both `poc-cert-broker-clean-host` and `msi-vcredist-prereq` todos are moved `pending/` → `resolved/` with the passing verdict JSON referenced.
**Plans**: 5 plans
- [ ] 105-01-PLAN.md — crates.io machinery: sparse-index poll helper + workflow_dispatch-only publish-crates.yml (retires the neutralized in-line job)
- [ ] 105-02-PLAN.md — PyPI machinery: twine install + confirm-gated maturin build + twine upload + post-publish coverage verification
- [ ] 105-03-PLAN.md — npm machinery: close the win32-x64-msvc platform-package gap + confirm-gated, platform-first-then-main publish script
- [ ] 105-04-PLAN.md — Verification tooling: pre-checkpoint registry-availability/scope re-check + SC4 isolated post-publish resolve wrapper
- [ ] 105-05-PLAN.md — Operator checkpoint (Wave 2): pre-publish gate, live publish (crates.io/PyPI/npm), post-publish resolve verification

### Phase 107: Close-Out
**Goal**: The POC signing path is retired, the disposable smoke workflow is removed, and stale documentation + STATE are corrected — but strictly only after clean-host UAT has proven the new path works end-to-end, preserving a fallback signing path until then.
**Depends on**: Phase 106 (CHOST-03 must PASS first — never merely a green release; this is the highest-severity sequencing risk in the milestone)
**Requirements**: CLOSE-01
**Success Criteria** (what must be TRUE):
  1. **[Operator-in-loop]** The close-out step structurally requires a link to a passed Phase 106 verdict JSON before the POC secrets (`WINDOWS_SIGNING_CERT` / `WINDOWS_SIGNING_CERT_PASSWORD`) are deleted from GitHub repo settings.
  2. `trusted-signing-smoke.yml` is removed (`git rm`) now that the real release path is proven green end-to-end.
  3. `docs/cli/development/windows-signing-guide.mdx` is corrected to point at the go-live cookbook instead of the retired PFX flow (gitignored-but-tracked — needs `git add -f`).
  4. `RELEASE-RUNBOOK.md` and `STATE.md`/`PROJECT.md` are updated to reflect FUT-01/FUT-02/FUT-03 + DIST-SIGN-01 cleared.
**Plans**: 5 plans
- [ ] 105-01-PLAN.md — crates.io machinery: sparse-index poll helper + workflow_dispatch-only publish-crates.yml (retires the neutralized in-line job)
- [ ] 105-02-PLAN.md — PyPI machinery: twine install + confirm-gated maturin build + twine upload + post-publish coverage verification
- [ ] 105-03-PLAN.md — npm machinery: close the win32-x64-msvc platform-package gap + confirm-gated, platform-first-then-main publish script
- [ ] 105-04-PLAN.md — Verification tooling: pre-checkpoint registry-availability/scope re-check + SC4 isolated post-publish resolve wrapper
- [ ] 105-05-PLAN.md — Operator checkpoint (Wave 2): pre-publish gate, live publish (crates.io/PyPI/npm), post-publish resolve verification

### Phase 108: UPST12 Divergence Audit
**Goal**: An authoritative per-commit divergence ledger for upstream `v0.66.0..v0.69.0` exists, so the absorb phases (109–111) have a classified, ADR-reviewed work-list — with the tool-sandbox subsystem's refinements provably fenced off to v3.7.
**Depends on**: Nothing (first phase of v3.6; parallel to v3.5)
**Requirements**: UPST12-01
**Success Criteria** (what must be TRUE):
  1. `108-DIVERGENCE-LEDGER.md` classifies every substantive commit in `v0.66.0..v0.69.0` (adopt / adapt / skip / split) with a `windows-touch` flag and an ADR-review verdict per cluster.
  2. Re-export/public-surface diffs are inspected (not just `git diff --name-only`), per the "cluster isolation can be empirically false" lesson.
  3. The 7 tool-sandbox refinement PRs (#1280/#1322/#1325/#1384/#1394/#1413/#1417) are explicitly recorded **DEFERRED→v3.7** with the reason (base subsystem absent).
  4. **[CONTEXT.md D-19]** As originally written ("the ledger maps each will-sync cluster onto Phase 109/110/111"), this SC is unsatisfiable — live measurement found ~28 commits mapping to none of v3.6's 12 requirements. The corrected SC: the ledger surfaces this gap with an exact count, names the unmapped clusters, and proposes a **Phase 112 (Security + Residual Sync)** roadmap amendment — gated on explicit operator approval before Phase 109 planning begins.
**Plans**: 5 plans
- [x] 108-01-PLAN.md — Ledger reproduction, full 100-commit CODE/DEPS/CI/DOCS accounting, Cluster Summary taxonomy skeleton
- [x] 108-02-PLAN.md — ADR-108: deny_domain (#1374) posture — settles ADAPT per D-12
- [x] 108-03-PLAN.md — NET/PROF/CORE per-commit tables, hand-verified requirement mapping + re-export scans
- [x] 108-04-PLAN.md — tool-sandbox 20-commit surface (pure/split residue accounting) + DEPS/CI/DOCS clusters
- [x] 108-05-PLAN.md — Carve-out re-touch check, D-18/D-19 requirement-coverage gap + Phase 112 proposal, ledger completeness sweep

### Phase 109: Proxy/Network Absorb
**Goal**: The v0.67–v0.69 proxy/network features are absorbed into the fork's proxy without regressing its fork-divergent TLS-interception + allowlist model, with the bindings rebuilt.
**Depends on**: Phase 108 (ledger dispositions)
**Requirements**: NET-01, NET-03
**Success Criteria** (what must be TRUE):
  1. `deny_domain` (#1374) is wired into the proxy filter + profile schema and composes with `allow_domain` without weakening default-deny.
  2. *(moved to Phase 113 — see below.)* ~~SPIFFE/SPIRE workload-identity auth for upstream routes (#1272)~~
  3. The SigV4 encoded-URI fix (#1430) and sibling-route cross-deny fix (#1437) are confirmed non-applicable — both target upstream subsystems (`aws/sign.rs`, `tls_intercept/handle.rs`) absent from the fork's architecture, per `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`; `no_proxy` bypass and `HTTP_PROXY` forward-proxy are verified non-regressed.
  4. `maturin build` (nono-py) and `napi build` (nono-ts) are green after the nono-proxy struct changes.
**Plans**: 5 plans
- [x] 109-01-PLAN.md — deny_domain (NET-01): library deny-suffix mechanism, CLI plumbing, D-04/D-05 fail-closed guard at both entry points
- [x] 109-02-PLAN.md — no_proxy (NET-03) proxy-crate mechanism: D-06 validators + D-07 localhost/127.0.0.1 regression proof
- [x] 109-03-PLAN.md — no_proxy (NET-03) CLI-crate wiring: profile schema + validate_profile_no_proxy + launch-time/group-expanded conflict validators
- [x] 109-04-PLAN.md — HTTP_PROXY forward-proxy (#1335, NET-03): classify_request_target/handle_forward_http absolute-form dispatch
- [x] 109-05-PLAN.md — Verification: N/A finding for #1430/#1437 (target subsystems absent from fork), cross-target clippy confirmation, binding rebuild (D-09/SC4)

### Phase 110: Profile/Policy Absorb + platform_overrides
**Goal**: The fork gains upstream's per-OS profile-patch model and the v0.67–v0.68 profile/policy features, and retires its top-level `windows_*` flag sprawl into `platform_overrides.windows`.
**Depends on**: Phase 108 (ledger dispositions)
**Requirements**: PROF-01, PROF-02, PROF-03, PROF-04
**Success Criteria** (what must be TRUE):
  1. `platform_overrides` (#1371) is absorbed, preserved through `extends` resolution (#1380), and the fork's `windows_low_il_broker`/`windows_interpreters` flags are migrated into `platform_overrides.windows` with back-compat aliases (existing profiles still load).
  2. `$VAR` (#1296) and `@git:*` (#1298) token expansion works in profile filesystem paths.
  3. The port-range profile schema (#1398) is absorbed with a **WFP-native** remote-port-range emitter on Windows and discrete-`Vec<u16>` back-compat.
  4. The `bun` (#1305) and `mise` (#1387) runtime presets are present and resolvable.

**Plans**: 8 plans
- [x] 110-01-PLAN.md — platform_overrides field + extends preservation + windows_low_il_broker/windows_interpreters back-compat proof (PROF-01)
- [x] 110-02-PLAN.md — $VAR process-env expansion + ported @git:* dynamic tokens (PROF-02)
- [x] 110-03-PLAN.md — CapabilitySet port-range mechanism + macOS/Linux Unix emitters (PROF-03 library+Unix)
- [x] 110-04-PLAN.md — NetworkConfig open_port_range/listen_port_range + profile_runtime.rs validation (PROF-03 schema)
- [x] 110-05-PLAN.md — capability_ext.rs profile-pathway wiring + manifest-pathway conversion (PROF-03 wiring)
- [x] 110-06-PLAN.md — Windows WFP-native remote-port-range emitter, fork-original (PROF-03 Windows) — **COMPLETE 2026-08-04**: Tasks 1-2 (`ff10f52f`/`ca8b3f25`/`8fe69f43`) plus Task 3's `checkpoint:human-verify` (PROF-03e) resolved live on an Administrator-elevated session — 8 filters from a verified-zero baseline, 4 `FWP_MATCH_RANGE` conditions at `49200..49210`, torn down to 0 on exit. Verdict + limitations: `110-06-PROF-03e-VERDICT.md`; procedure: `110-06-CHECKPOINT-RUNBOOK.md`. Running the checkpoint surfaced 4 defects, all fixed (`7c7a189c`, `ea26b5b2`, `6d7ef719`, `4aec1944`), 3 proven live. Behavioural connect probe NOT RUN (`0xC0000142`) — closure accepted on the filter-table proof.
- [x] 110-07-PLAN.md — bun/mise runtime presets + resolvability tests (PROF-04)
- [x] 110-08-PLAN.md — Phase gate: cross-target clippy + make ci + binding rebuild + fork-invariant verify — both cross-target gates GREEN live, both sibling bindings green, SC3/ADR-86 confirmed, all 4 Wave-0 gaps closed; see `110-08-SUMMARY.md`. **Phase gate itself is GREEN, but the phase is still not closed — 110-06 Task 3's checkpoint remains pending.**

### Phase 111: Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog
**Goal**: The macOS/core carry and resource-CLI alignment land, the whole sync is proven non-regressing under both cross-target clippy gates, and the tree leapfrogs to a prepare-only `0.70.0`.
**Depends on**: Phase 109 + Phase 110 (all absorb work landed)
**Requirements**: CORE-01, CORE-02, VERIFY-01, RLS-14
**Success Criteria** (what must be TRUE):
  1. macOS/core carry lands as-is for cross-target parity — `~/.cache` (#1378) and `MAX_CRYPTO_THREADS`=12 (#1424). **(#1398's `macos.rs` port-range emitter REMOVED from this criterion 2026-07-30 — Phase 110 absorbs commit `d5803b99` whole, including both Unix emitters, so the port-range feature lands coherently in one place. Do not re-absorb it here.)**
  2. The `--memory`/`--max-processes` CLI surface (#1269/#1403) is aligned onto the fork's existing Job Object impl with no new enforcement and no regression to `--cpu-percent`/`--timeout`.
  3. Both cross-target clippy gates (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) and `make ci` are GREEN locally; a fork-invariant pass confirms the Windows security model + ADR-86 boundary are unregressed.
  4. All 6 workspace crates + path-dep pins + both binding repos leapfrog to `0.70.0` (collision-free above upstream 0.69.0), Cargo.lock shows zero unexpected drift, and the prepare-only release gate is GREEN (no operator push).

**Plans**: 6 plans
- [x] 111-01-PLAN.md — CORE-01: macOS `~/.cache` policy grant (#1378) + `MAX_CRYPTO_THREADS` 7→12 (#1424), each with a new Wave-0 by-value test
- [x] 111-02-PLAN.md — CORE-02: correct the stale Unix resource-limit help text in `cli.rs` + `docs/cli/usage/flags.mdx` (D-04/D-05/D-06)
- [x] 111-03-PLAN.md — D-01/D-02/D-03: `proj/ADR-111-resource-limits-boundary.md` + standing-divergence addendum in `108-DIVERGENCE-LEDGER.md`
- [x] 111-04-PLAN.md — VERIFY-01: both cross-target clippy gates + `make ci` substitution + 24-name baseline diff + D-09 fork-invariant assertions + binding rebuild, over the combined 108-111 surface
- [x] 111-05-PLAN.md — RLS-14 (in-repo half): bump the 6 workspace crates + path-dep pins to `0.70.0`, regenerate Cargo.lock, correct `release-readiness.ps1`/`release-dry-run.ps1`'s hardcoded version strings
- [x] 111-06-PLAN.md — RLS-14 (sibling half): bump `../nono-py` + `../nono-ts` to `0.70.0`, rebuild both, confirm the prepare-only `release-dry-run.ps1` gate GREEN

### Phase 112: Security + Residual Sync
**Goal**: The security-relevant and residual commits from the `v0.66.0..v0.69.0` window that no other v3.6 phase covers are absorbed under a fork-invariant review kept separate from any release-cut phase — mirrors the v3.1 Phase 87 precedent.
**Depends on**: Phase 108 (the ledger's `security-residual-and-misc` cluster + Requirement Coverage Gap section are the work-list)
**Requirements**: SEC-01, SEC-03, SEC-04, SEC-05, SEC-06, SEC-07, SEC-08, SEC-09, RES-01, RES-02 *(SEC-02 carved out to Phase 114 — see Amendment below)*
**Origin**: Added 2026-07-29 by operator approval of the Phase 108 ledger's D-18/D-19 coverage-gap amendment. Phase 108 measured 27 hand-verified CODE commits mapping to none of v3.6's original 12 requirements; ROADMAP SC4 for Phase 108 was unsatisfiable as written (D-19) and this phase is its resolution.
**Amendment (2026-08-05, operator decision during `/gsd:plan-phase 112`)**: **SEC-02 (OAuth capture — `9b692e07` / `3c59c62e` / `d033c631`) is carved out of Phase 112 into its own Phase 114.** Phase 112's research pass established that ~9 of `9b692e07`'s 26 files depend on subsystems absent from the fork (`oauth_capture/` core, `tls_intercept/*`, and `forward.rs` from unabsorbed ancestor `149abde0`). `forward.rs` carries the response-rewrite hook that keeps real OAuth tokens out of the sandboxed client; a reduced-scope absorb without an equivalent enforcement point risks shipping a half-feature that leaks tokens. This exceeds the discretion `112-CONTEXT.md` D-04 granted the planner (which named SEC-02a as "own plan, own wave" and explicitly rejected a follow-on split), so it was escalated for operator adjudication and approved — mirroring the NET-02/SPIFFE → Phase 113 split, which was likewise an explicit operator decision rather than planner discretion. SC1 below is amended accordingly; the carve-out is recorded, not silently dropped.
**Success Criteria** (what must be TRUE):
  1. Eight of the nine security-relevant requirements (SEC-01, SEC-03..SEC-09) are absorbed with a fork-invariant review distinct from any release-cut phase, mirroring the v3.1 Phase 87 separation of security review from feature absorb. **SEC-02 is explicitly carved out to Phase 114 by the Amendment above** — Phase 112 must still record SEC-02's reality-check evidence and its deferred disposition in the D-05 ledger addendum, never silently drop it.
  2. The registry/update-check header residual (RES-01) and the PTY-teardown/test-infra residual (RES-02) are each individually reviewed and either absorbed or explicitly skipped with recorded reasoning — never silently dropped.
  3. `373a67ae` (#1369, `crossbeam-epoch` 0.9.18→0.9.20) is prioritized ahead of routine DEPS-cluster absorb — it is the direct fix for the live RUSTSEC-2026-0204 advisory the fork's `Cargo.lock` currently carries. *(If already closed by an out-of-band quick task, record that and confirm `cargo audit` is clean rather than re-absorbing.)*
  4. Both cross-target clippy gates (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) and `make ci` are GREEN locally after the absorb, consistent with VERIFY-01's framing in Phase 111.
**Plans**: 8 plans
- [ ] 112-01-PLAN.md — Wave 1 reality-check finalization (D-02) + SEC-01 won't-sync finding + SEC-02a/b/c go/no-go decision + RES-01 skip docs + D-07 confirmation
- [ ] 112-02-PLAN.md — SEC-03: NVIDIA procfs mediation hardening + Sandbox::apply_seccomp/apply_seccomp_with_abi Linux API refactor
- [ ] 112-03-PLAN.md — SEC-04: trust-policy predicate discriminator + SEC-08: ADR-112 preserving the fork's fail-closed allow_vars default
- [ ] 112-04-PLAN.md — RES-02: PTY late-CPR-reply teardown drain (adopt) + socket.rs /tmp switch (skip) + test-infra tightening (adopt)
- [ ] 112-05-PLAN.md — SEC-05: Landlock Refer grant in the execute-restriction layer
- [ ] 112-06-PLAN.md — SEC-06: seccomp-notify supervisor-ancestry orphan reaping (adapted)
- [ ] 112-07-PLAN.md — SEC-07: standalone `nono proxy` command, adapted to the fork's ProxyLaunchOptions API
- [ ] 112-08-PLAN.md — D-05 ledger addendum (all 18 dispositions) + SEC-09 carry-forward note + combined-surface verification + REQUIREMENTS/ROADMAP reconciliation


### Phase 113: SPIFFE/SPIRE Workload Identity
**Goal**: Upstream's SPIFFE/SPIRE workload-identity auth for upstream routes (#1272) is absorbed under its own ADR-gated review, without regressing the fork's divergent TLS-interception model or the ADR-86 policy-free-library boundary.
**Depends on**: Phase 109 (shares `nono-proxy` files — `server.rs`, `route.rs`, `credential.rs`, `oauth2.rs`, `tls_intercept/*`; 109 lands first and 113 rebases onto it)
**Requirements**: NET-02
**Origin**: Split out of Phase 109 on 2026-07-29 by operator decision during `/gsd:discuss-phase 109`. Measurement: `c831dade` is **4354 insertions / 545 deletions / 33 files** — 57% of the original Phase 109 by volume — and is a refactor of fork-divergent code, not an addition.
**Success Criteria** (what must be TRUE):
  1. A standalone `proj/ADR-113-spiffe-disposition.md` settles adopt-vs-adapt-vs-defer, weighing: the ADR-86 core-library crossing, the 545-deletion rewrite of the fork's divergent `tls_intercept`/`reverse.rs`, and the ~633-line `Cargo.lock` dependency-surface expansion on a security tool.
  2. SPIFFE/SPIRE workload-identity auth for upstream routes (#1272) is absorbed per the ADR's disposition and is configurable via profile.
  3. The ADR-86 boundary is confirmed non-regressed: the core-library additions (`crates/nono/src/undo/types.rs`, `crates/nono/src/audit.rs`) are shown to be audit/telemetry data types carrying no policy or enforcement logic — the reading recorded in `109-CONTEXT.md` — or the absorb is adapted to make that true.
  4. Cross-target clippy is GREEN (`c831dade` touches `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, a cfg-gated Unix surface — the gate is mandatory, no PARTIAL→CI), and `maturin build` + `napi build` are green after the `nono-proxy` struct changes.

### Phase 114: OAuth Capture Absorb (SEC-02)
**Goal**: Upstream's OAuth-capture surface (`9b692e07` / `3c59c62e` / `d033c631`) is absorbed or formally declined under its own disposition review, with the guarantee that real OAuth tokens never reach the sandboxed client preserved as a hard precondition — no reduced-scope half-feature ships.
**Depends on**: Phase 112 (SEC-07 creates `crates/nono-cli/src/proxy_command.rs`, which `9b692e07` edits — 112 lands first), and Phase 113 (shares the `nono-proxy` `oauth2.rs` / `credential.rs` / `tls_intercept` surface)
**Requirements**: SEC-02
**Origin**: Carved out of Phase 112 on 2026-08-05 by operator decision during `/gsd:plan-phase 112`, after the plan-checker escalated it as exceeding the discretion `112-CONTEXT.md` D-04 granted the planner. Measurement: `9b692e07` is **26 files / +4,425 insertions**; Phase 112's research pass found ~9 of those files depend on subsystems absent from the fork (`oauth_capture/` core, `tls_intercept/*`, `forward.rs` from unabsorbed ancestor `149abde0`). Mirrors the NET-02/SPIFFE → Phase 113 split.
**Success Criteria** (what must be TRUE):
  1. A disposition decision for all three SEC-02 SHAs (`9b692e07`, `3c59c62e`, `d033c631`) is recorded with cited evidence — adopt, adapt-down, or formally decline — and the `108-DIVERGENCE-LEDGER.md` addendum row written by Phase 112 is updated to the final disposition rather than left at "deferred".
  2. The token-confinement property is proven, not assumed: either the `forward.rs` response-rewrite hook (or an equivalent fork-side enforcement point) is in place, or the absorb is declined with the reasoning recorded. **A reduced-scope absorb that drops the rewrite hook without an equivalent enforcement point is forbidden** — that is the specific failure this phase exists to prevent.
  3. If absorbed, the ADR-86 / ADR-111 boundary is confirmed non-regressed — no policy or enforcement logic lands in the core `nono` crate.
  4. Both cross-target clippy gates (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) are GREEN locally, plus `cargo fmt --all --check` and the workspace test suite diffed against the documented inherited failing baseline.

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 85. UPST9 Divergence Audit | v3.1 | 1/1 | Complete | 2026-06-19 |
| 86. Library-Boundary Convergence | v3.1 | 3/3 | Complete | 2026-06-20 |
| 87. Security Sync | v3.1 | 3/3 | Complete | 2026-06-20 |
| 88. Feature + Dependency Cherry-Pick Wave | v3.1 | 6/6 | Complete | 2026-06-20 |
| 89. Proxy Hardening Sync | v3.1 | 4/4 | Complete | 2026-06-21 |
| 90. v3.0 Host-Gated UAT Drain | v3.1 | 2/2 | Complete | 2026-06-21 |
| 91. Signed Override Format + Verification Core | v3.2 | 3/3 | Complete | 2026-06-22 |
| 92. Runtime CapabilitySet Mutation + Audit Wiring | v3.2 | 4/4 | Complete | 2026-06-22 |
| 93. Live ZT-Infra Integration + Revocation + Request Flow | v3.2 | 6/6 | Complete | 2026-06-23 |
| 94. UPST10 Divergence Audit | v3.3 | 2/2 | Complete | 2026-06-26 |
| 95. Upstream Absorb + Fork-Invariant Verify | v3.3 | 7/7 | Complete | 2026-06-26 |
| 96. Cross-Target Toolchain | v3.3 | 3/3 | Complete | 2026-06-26 |
| 97. Release Engineering — Leapfrog + Pipeline + Runbook | v3.3 | 4/4 | Complete | 2026-06-26 |
| 98. UPST11 Divergence Audit | v3.4 | 4/4 | Complete    | 2026-06-30 |
| 99. Upstream Absorb + Fork-Invariant Verify | v3.4 | 7/7 | Complete    | 2026-06-30 |
| 100. Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker | v3.4 | 5/5 | Complete   | 2026-07-02 |
| 101. Verify-Gate Hardening + Azure Profile Confirmation | v3.5 | 4/4 | Failed (SIGN-03 RED, diagnosed, deferred to Phase 104) | 2026-07-03 |
| 102. Fork-Owned Package Rename | v3.5 | 5/5 | Complete   | 2026-07-03 |
| 103. Azure Clean-Host VM IaC + New Verify-Dark Gates | v3.5 | 3/3 | Complete   | 2026-07-03 |
| 104. Smoke Green + Cut the Trusted-Signed Release | v3.5 | 1/3 | In Progress|  |
| 105. Live Multi-Registry Publish | v3.5 | 0/5 | Not started | - |
| 106. Azure VM Clean-Host UAT | v3.5 | 0/? | Not started | - |
| 107. Close-Out | v3.5 | 0/? | Not started | - |
| 108. UPST12 Divergence Audit | v3.6 | 5/5 | Complete | 2026-07-29 |
| 109. Proxy/Network Absorb | v3.6 | 5/5 | Complete | 2026-07-29 |
| 110. Profile/Policy Absorb + platform_overrides | v3.6 | 8/8 | Complete | 2026-08-04 |
| 111. Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog | v3.6 | 6/6 | Complete   | 2026-08-05 |
