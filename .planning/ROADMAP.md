---
milestone: v3.7
milestone_name: Composite Integrity + Tool-Sandbox Disposition
status: planning
parallel_milestone: v3.5
parallel_milestone_name: Trusted Signing Go-Live + First Distributed Release
parallel_milestone_status: paused-at-phase-104-on-operator-tag-push
shipped_milestone: v3.6
shipped_milestone_name: "UPST12: Upstream Sync v0.66.0 -> v0.69.0"
shipped_date: 2026-08-08
updated: 2026-08-09
---

# Roadmap: nono

## Milestones

- 🔄 **v3.7 Composite Integrity + Tool-Sandbox Disposition** — Phases 115-120 (active 2026-08-08)
- ✅ **v3.6 UPST12 Upstream Sync (v0.66.0→v0.69.0)** — Phases 108-114 (shipped 2026-08-08) — [archive](milestones/v3.6-ROADMAP.md)
- 🔄 **v3.5 Trusted Signing Go-Live + First Distributed Release** — Phases 101-107 (open 2026-07-02, PAUSED at Phase 104 on the operator tag-push decision)
- ✅ **v3.4 UPST11 Upstream Sync to v0.66.0 + Release-Reconcile** — Phases 98-100 (shipped 2026-07-02) — [archive](milestones/v3.4-ROADMAP.md)
- ✅ **v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release** — Phases 94-97 (shipped 2026-06-26) — [archive](milestones/v3.3-ROADMAP.md)
- ✅ **v3.2 Signed Policy Overrides (ZT-Infra Attestation)** — Phases 91-93 (shipped 2026-06-23) — [archive](milestones/v3.2-ROADMAP.md)
- ✅ **v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain** — Phases 85-90 (shipped 2026-06-21) — [archive](milestones/v3.1-ROADMAP.md)
- ✅ **v3.0 Enterprise Hardening I — Deploy · Control · Compliance** — Phases 82-84 (shipped 2026-06-19) — [archive](milestones/v3.0-ROADMAP.md)
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18) — [archive](milestones/v2.13-ROADMAP.md)

> Earlier milestones (v2.5–v2.12) are archived under `.planning/milestones/`.

## Phases

<details open>
<summary>🔄 v3.7 Composite Integrity + Tool-Sandbox Disposition (Phases 115-120) — ACTIVE</summary>

Make the fork's deny-by-*composition* Windows model **prove** it is enforcing, settle the standing tool-sandbox divergence by recorded decision, and drain v3.6's six carry-forward findings. No new confinement layers — higher trustworthiness in what already exists. **ADR-65 stands**, so the production minifilter (G-WIN-3) and driver lifecycle (G-WIN-4) are out, and per-file read policy is explicitly not claimed. **Milestone-marker only** — tree stays at `0.70.0`, no publish.

Sequencing rationale: **115** drains the v3.6 findings first because DRAIN-03 cleans the denial/audit spine that the receipt work in 118 builds on, and because the drain is independent of everything else. **116** runs the tool-sandbox ledger + ADR *early* so its verdict — which may or may not imply substantial engineering — is known before the milestone's remaining capacity is committed; its execution (120) is deliberately last and sized by that verdict. **117 → 118 → 119** run in dependency order: the fail-direction contract enumerates every layer, the receipts attest that enumeration per session, and the boundary statement can only be truthful once the contract says what each layer actually does.

- [x] **Phase 115: v3.6 Carry-Forward Drain** — 6/6 plans (2026-08-09)
- [x] **Phase 116: Tool-Sandbox Divergence Audit + Disposition ADR** — 6/6 plans (2026-08-09)
- [x] **Phase 117: Fail-Direction Contract + Startup Self-Attestation** — 34/34 plans executed. Gap-closure round 3 EXECUTED 2026-08-11 (waves 12-14), closing CR-03 (BLOCKER) and WR-12..WR-21; D-37 implemented as the three-arm ancestor classification. Full suite 1624 passed / 12 failed = the 11 documented Windows-host baseline + **one intentional host-blocked test** (117-30's WR-20 pin needs an elevated/CI runner — `SeTakeOwnershipPrivilege` absent here, so WR-20 is authored-but-unverified). Code review ran to round 9; re-verified 2026-08-15 (117-VERIFICATION.md, status: human_needed): SC1/SC2 VERIFIED, SC3 ACCEPTED via operator override (still factually 10/13), SC4 VERIFIED with disclosed limits — its recording gap closed the same day by quick task 260815-b0s. One human item open: the WR-20 CI run.
- [x] **Phase 118: Per-Session Enforcement Receipts** — 10/10 plans (2026-09-06; 1 item open — RCPT-02 text divergence, see Progress)
- [ ] **Phase 119: Security-Model Boundary Statement + State-of-the-Art Decision Log** — 0/? plans
- [ ] **Phase 120: Tool-Sandbox Verdict Execution** — 0/? plans

</details>

<details>
<summary>✅ v3.6 UPST12 Upstream Sync v0.66.0→v0.69.0 (Phases 108-114) — SHIPPED 2026-08-08</summary>

Drain-then-sync upstream milestone (mirrors v3.1/v3.3/v3.4), running **in parallel** with the operator-blocked v3.5. Absorb the cross-platform delta from `nolabs-ai/nono` `v0.66.0..v0.69.0` (v0.67.0/.1, v0.68.0, v0.69.0) — proxy/network (`deny_domain`, SPIFFE/SPIRE, SigV4 + sibling-route fixes), profile/policy (`platform_overrides` + migrate the fork's `windows_*` flags into it, `$VAR`/`@git` tokens, port-range schema with a WFP-native emitter, bun/mise presets), macOS Seatbelt carry, and resource-CLI alignment onto the existing Job Object impl — WITHOUT regressing the Windows security model or the ADR-86 boundary, then leapfrog all 6 crates + both binding repos to **`0.70.0`** (prepare-only). **Explicitly EXCLUDES** the `tool-sandbox/` subsystem (PR #1105, introduced v0.65.0, never absorbed — a standing structural divergence deferred to the dedicated **v3.7 Windows Tool-Sandbox Parity** milestone). Scope source: quick `260727-jkn`.

- [x] **Phase 108: UPST12 Divergence Audit** — 5/5 plans
- [x] **Phase 109: Proxy/Network Absorb** — 5/5 plans
- [x] **Phase 110: Profile/Policy Absorb + platform_overrides** — 8/8 plans, all 4 requirements complete; 110-06's live-kernel checkpoint (PROF-03e) resolved 2026-08-04
- [x] **Phase 111: Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog** — 6/6 plans
- [x] **Phase 112: Security + Residual Sync** — 8/8 plans (SEC-02 explicitly carved out to Phase 114)
- [x] **Phase 113: SPIFFE/SPIRE Workload Identity** — 8/8 plans
- [x] **Phase 114: OAuth Capture Absorb (SEC-02)** — 11/11 plans — carved out of Phase 112 by operator decision 2026-08-05 (ROADMAP Amendment); planned 2026-08-06 (11 plans, 6 waves); COMPLETE 2026-08-07 (11/11 plans; code-reviewed — 3 Criticals found and fixed, 9 Warning + 6 Info open by operator scoping)

</details>

<details>
<summary>🔄 v3.5 Trusted Signing Go-Live + First Distributed Release (Phases 101-107) — OPEN, PAUSED at Phase 104 on the operator tag-push decision</summary>

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

### Phases 108–114 (v3.6 — SHIPPED 2026-08-08)

Full goals, success criteria, and per-plan detail archived to
[`milestones/v3.6-ROADMAP.md`](milestones/v3.6-ROADMAP.md).
Requirements: [`milestones/v3.6-REQUIREMENTS.md`](milestones/v3.6-REQUIREMENTS.md).
Audit: [`milestones/v3.6-MILESTONE-AUDIT.md`](milestones/v3.6-MILESTONE-AUDIT.md).

---

### Phase 115: v3.6 Carry-Forward Drain
**Goal**: The six findings the v3.6 audit carried forward are closed at the class level, not the symptom level — so the denial/audit spine the receipt work builds on is clean before it is built on, and neither hand-maintained-list drift (DRAIN-02/03) nor removal-by-silence (DRAIN-01) can silently recur.
**Depends on**: Nothing — independent of both other workstreams
**Requirements**: DRAIN-01, DRAIN-02, DRAIN-03, DRAIN-04, DRAIN-05, DRAIN-06
**Success Criteria** (what must be TRUE):
  1. A `platform_overrides.<os>` block that redefines a credential to change `upstream` and omits `inject_mode`/`inject_header` leaves a `url_path` or `query_param` route injecting where the base profile said — proven by a test that fails if the merge reverts to taking the child value unconditionally. The NEW-02 regression test also asserts its `spiffe` arm (ACC-04).
  2. Every denial category `nono-py`'s encoder can emit decodes without raising — enforced by a test driven from the encoder's own output rather than a second hand-written list, so a future variant cannot reintroduce the gap.
  3. No production `log_denied` call site passes a default `EventContext`; `connect.rs` — `deny_domain`'s HTTPS enforcement point — carries a real denial category, and no denial variant remains with zero production constructors.
  4. A profile declaring both `aws_auth` and `capture` on one route is rejected when the profile is validated, not accepted and then 501'd at request time.
  5. A Python embedder can set `capture` and `spiffe` on a `RouteConfig`, and a `deny_domain`-blocked SPIFFE route is denied before any JWT-SVID is minted.
**Plans**: 6 plans
- [x] 115-01-PLAN.md — DRAIN-01: CustomCredentialDef inject_mode/inject_header -> Option<T>, exhaustive .or(base) merge, HookConfig compile-time guard
- [x] 115-02-PLAN.md — DRAIN-02/DRAIN-03: log_denied required-category signature, 28 call-site migration + HostDenied/ExternalProxyRejected wiring, InterceptHandshakeFailed removal + self-enumerating ALL const
- [x] 115-03-PLAN.md — DRAIN-04: reject aws_auth and plain OAuth2 client_credentials at profile-validation time
- [x] 115-04-PLAN.md — DRAIN-06: hoist deny_domain host-check above managed_auth.acquire() in the SPIFFE dispatch path
- [x] 115-05-PLAN.md — DRAIN-02: nono-py denial-category codec — delete hand-written matches, self-enumerating round-trip test, maturin build
- [x] 115-06-PLAN.md — DRAIN-05: nono-py RouteConfig exposes spiffe/capture/endpoint_policy, D-16 allowlist test, maturin build

### Phase 116: Tool-Sandbox Divergence Audit + Disposition ADR
**Goal**: The standing structural divergence — upstream's `tool-sandbox/` subsystem, never absorbed since v0.65.0 — stops being an unexamined gap and becomes a recorded decision, with the fork's own hook + Low-IL broker path weighed as a real alternative rather than assumed inferior. Run early so the verdict is known before the milestone's remaining capacity is committed.
**Depends on**: Nothing — the ADR needs targeted symbol-level knowledge of the fork's hook path, not the full fail-direction contract
**Requirements**: TSBX-01, TSBX-02
**Success Criteria** (what must be TRUE):
  1. A per-commit ledger covers PR #1105 and all 7 fenced refinement PRs (#1280/#1322/#1325/#1384/#1394/#1413/#1417) with `windows-touch` flags and per-cluster dispositions, in the Phase 108/98/94/85 shape.
  2. Every confidence rating in the ledger cites a grep for the actual type, function or field it depends on — a reviewer can re-run the evidence, and no rating rests on "the target files exist."
  3. The ADR states an unambiguous verdict (adopt / formalize fork-native) and records what would have to change for the losing option to win, so a future absorb has a decision to reconcile against rather than silence.
  4. The ADR evaluates PR #4's PreToolUse-hook + Low-IL-primary-token-broker path on its merits — including the durable finding that .NET/PowerShell CLR cannot start under `WRITE_RESTRICTED` — rather than treating it as a stopgap.
  5. Phase 120's scope is sized from the verdict and written down, so the milestone's tail is no longer open-ended.
**Plans**: 6 plans
- [x] 116-01-PLAN.md — Reproduction block + D-03 module-set re-derivation + pre-fence/post-fence per-commit disposition tables
- [x] 116-02-PLAN.md — Feasibility matrix appendix: fork Windows-confinement primitive inventory + upstream capability rating (D-07/D-08)
- [x] 116-03-PLAN.md — Split-commit residue accounting (D-04) + grep-verified fenced-window reconciliation + post-fence residue finding
- [x] 116-04-PLAN.md — Carve-out re-touch check (D-19) + bucket-count reconciliation + completeness verification + Headline finalization
- [x] 116-05-PLAN.md — ADR Part 1: Context + two-pole framing (D-05) + rejected intermediate shapes (D-06) + D-09 symmetric scoring table
- [x] 116-06-PLAN.md — ADR Part 2: Decision/verdict + D-07/D-08 summary + D-12 triggers/re-test point + D-14/D-15 Phase 120 sizing proposal

**Verdict**: **Formalize fork-native (Pole B)** — `proj/ADR-116-tool-sandbox-disposition.md`, Status: Accepted.
**Operator-gated findings** (proposed in the ledger, deliberately NOT applied here): Uncovered-Window Finding — 12 commits touch a named fork-invariant carve-out surface but are dispositioned by no audit to date (proposed home: UPST13/FUT-08); Post-Fence Residue Finding — disposed "no new successor phase needed".

### Phase 117: Fail-Direction Contract + Startup Self-Attestation
**Goal**: The composite's fail-direction stops being decided per-layer-in-isolation and becomes one system-level answer — and nono can no longer report "enforcing" while a layer is silently inert, which is the failure mode this codebase has already hit once.
**Depends on**: Phase 115 (clean audit/denial spine)
**Requirements**: CINT-01, CINT-02, CINT-03
**Success Criteria** (what must be TRUE):
  1. One document names every layer the Windows backend composes — restricted token, mandatory integrity label, AppContainer profile + package SID, DACL grants, WFP egress filters, and the minifilter's absence — and states each one's behaviour when it cannot be established, citing the enforcing call site.
  2. Starting a confined session with a layer forced unavailable produces either an abort or a visibly downgraded claim; there is no path on which nono presents a confinement guarantee it did not confirm.
  3. Every entry in the contract has a test that forces that layer unavailable and asserts the contracted outcome — a contract row without a test is not counted as satisfied.
  4. Where the contract and the code disagree, the code is changed or the contract is corrected in the same phase, with the discrepancy recorded rather than quietly reconciled.
**Plans**: 34 plans in 14 waves (all 34 executed: waves 1-8 in the first two rounds, waves 9-11 on 2026-08-11, waves 12-14 on 2026-08-11; second gap-closure round added 2026-08-10 from the re-verification pass in `117-VERIFICATION.md` — status gaps_found, 0/4 truths fully verified — closing CR-01 BLOCKER, CR-02 BLOCKER-adjacent/D-28, WR-01..WR-11, and the remaining 9/13 SC1 citation gap; see `117-REVIEW.md` iteration 4)

**Wave 1** — the registry is the source of truth everything else derives from (D-01)
- [x] 117-01-PLAN.md — Layer registry core: LayerId enum, per-row expectancy matrix, D-10 derivation + Open Question 2/5 resolution
- [x] 117-02-PLAN.md — Library surfaces: NonoDiagnosticCode::LayerAttestationFailed + machine_policy required_layers field

**Wave 2** *(blocked on Wave 1)*
- [x] 117-03-PLAN.md — proj/SPEC-windows-fail-direction-contract.md + registry self-check/drift-check tests
- [x] 117-04-PLAN.md — layer-fault-injection Cargo feature + WFP toggle migration off NONO_TEST_HARNESS (SC4-4)
- [x] 117-05-PLAN.md — Shared crates/nono attestation primitive: LayerAttestationStatus (4-state) + raw OS probes

**Wave 3** *(blocked on Wave 2)*
- [x] 117-06-PLAN.md — Per-layer fault-injection hooks: restricted token, mandatory label, DACL grants, Job Object
- [x] 117-07-PLAN.md — AppContainer fault-injection hooks: nono-shell-broker (new [features] block) + agent_daemon
- [x] 117-08-PLAN.md — CLI-side attestation decision module: attest_and_decide() + broker wire contract + Open Question 1 resolution
- [x] 117-09-PLAN.md — D-27 channels: coarse downgrade banner + HMAC-chained telemetry event

**Wave 4** *(blocked on Wave 3 — the three independent suspended-spawn gate sites)*
- [x] 117-10-PLAN.md — D-21 gate insertion: nono-cli direct spawn + daemon path + SC4-2 drop-order comment fix
- [x] 117-11-PLAN.md — D-21 gate insertion: nono-shell-broker's own suspended AppContainer child

**Wave 5** *(blocked on Wave 4)*
- [x] 117-12-PLAN.md — Forced-unavailable tests + D-32 meta-test + D-31 loud-gap list + cross-target clippy + D-24 latency + SPEC close-out

**Gap closure (2026-08-10, from `117-VERIFICATION.md`, status: gaps_found, 0/4 truths fully verified)** — see `117-REVIEW.md` iteration 3 (1 Blocker + 9 Warning) and `117-REVIEW-FIX.md` for the findings these plans close.

**Wave 6** *(blocked on Wave 5 — parallel, disjoint files)*
- [x] 117-13-PLAN.md — NR3-01 BLOCKER: self-healing mandatory-label residue detection + NonoError::remediation() arm for LayerAttestationFailed
- [x] 117-14-PLAN.md — NR3-02: real coverage accessor for DaclAncestorTraverse/DaclAncestorReadAttrs (was a constant Applied, structurally could not deny)
- [x] 117-15-PLAN.md — NR3-03 + NR3-08: close the DaclSessionSidGrant fail-open landmine (false test citation) + convert 4 rows' call-site citations to symbol form
- [x] 117-16-PLAN.md — NR3-04: unconditional downgrade-banner diagnostic log line + new FirewallRulesEgress forced-unavailable test (SC3)
- [x] 117-17-PLAN.md — NR3-05: delete the daemon's dead ProceedDowngraded decision state; document abort-or-proceed by design

**Wave 7** *(blocked on Wave 6 — reads dacl_guard.rs/launch.rs/layer_registry.rs)*
- [x] 117-18-PLAN.md — SC3: broaden layer_registry_meta_test.rs discovery (ALSO_AUTOMATED, 8 rows) + CI fix (NR-08, build nono-shell-broker + NONO_CI_HAS_WFP)

**Wave 8** *(blocked on Wave 7 — documents the final state of every gap-closure plan)*
- [x] 117-19-PLAN.md — SC4: port NR-04/NR-05/NR-06 + NR3-* rows into the SPEC's Review-fix pass ledger, fix stale RF-14, update registry-table citations, reduce manual-verification section to 3 operator-accepted rows

**Gap closure round 2 (2026-08-10, from `117-VERIFICATION.md` re-verification pass, status: gaps_found, 0/4 truths fully verified)** — the first gap-closure round (waves 6-8) closed NR3-01..NR3-08 but its own fixes reopened CR-14's class as CR-01 and introduced a D-28 violation as CR-02; see `117-REVIEW.md` iteration 4 (2 Critical + 11 Warning) for the findings these plans close.

**Wave 9** *(blocked on Wave 8 — parallel, disjoint files)*
- [x] 117-20-PLAN.md — CR-01 BLOCKER: ACE-flags-aware residue predicate (rejects INHERIT_ONLY_ACE), ownership-gate reorder (WR-01), corrected revert-semantics doc comment (WR-02), restored/added test coverage (WR-03)
- [x] 117-21-PLAN.md — CR-02 BLOCKER-adjacent/D-28: gate the downgrade warn's layer-name detail behind a private-log-channel check; wire NonoError::remediation() into main.rs's error path (WR-04)
- [x] 117-22-PLAN.md — WR-06: correct DaemonAttestationDecision's false "no partial-success return" premise + cross-mirror subset test; WR-11: anchor-marker-hardened two-state discovery test + exhaustive-match behavioral test
- [x] 117-24-PLAN.md — SC1/CINT-01: convert the remaining 9/13 registry rows to verified symbol-form citations; WR-08: definition-site-aware content check; WR-10: contains_fn_exact's documented `!` boundary + definition-line-prefix requirement
- [x] 117-25-PLAN.md — WR-05: remove dead NONO_CI_HAS_WFP CI config; WR-09: revert SecurityEventLayerInner's fields to private behind an advance_and_snapshot accessor

**Wave 10** *(blocked on Wave 9 — 117-23 shares launch.rs with 117-21)*
- [x] 117-23-PLAN.md — WR-07: 3-state application() (walked/PartiallyApplied/Applied) for AppliedAncestorTraverseGuard and AppliedAncestorReadAttributesGuard, closing the expected:true-row-silently-drops predicate-width gap; gate-level regression test

**Wave 11** *(blocked on Wave 10 — documents the final state of every gap-closure-round-2 plan, alone per SC4)*
- [x] 117-26-PLAN.md — SC4: record CR-01, CR-02, WR-01..WR-11 in the SPEC's Review-fix pass ledger with re-runnable evidence; sync the registry-table citations to the post-117-24 code; correct the now-stale NR3-01/NR3-04/NR3-05 rows

**Gap closure round 3 (2026-08-11, from `117-REVIEW.md` iteration 5)** — round 2 closed 7 of iteration 4's 13 findings outright but left 6 partial, and its own fixes produced the next blocker for the third consecutive round: CR-02's D-28 gate landed on 1 of 3 leak sites (CR-03), and plans 117-22 and 117-23 shipped **contradictory rules for the same physical condition** (WR-12). The recurring mechanism is one class: a guard fixed at one call site, paired with a test that NAMES its target instead of DISCOVERING it. Round 3 therefore plans by CLASS, not by site — every plan that closes a one-site finding must first enumerate every site of that class, and every test must discover its targets and carry a perturbation proof that it can actually fail. **D-37 was LOCKED by the operator before planning** to settle WR-12: an ancestor walk that stops at the first non-owned ancestor is contract-exempt, not a downgrade. Plan-checker: 4 warnings -> 1 blocker -> **VERIFICATION PASSED** at revision iteration 2.

**Wave 12** *(blocked on Wave 11 — 6 plans, disjoint files)*
- [x] 117-27-PLAN.md — CR-03 BLOCKER: gate ALL 3 `LayerId`-name-to-console sites (field AND message text) behind one shared D-28 gate, and widen the withholding test to scan text; WR-15 (banner names a channel the event never reaches); WR-16 (`log_target_is_private()` does not validate the log path against the child's own granted policy)
- [x] 117-28-PLAN.md — WR-12/**D-37**: distinguish "walked, stopped at non-owned ancestor" (contract-exempt) from "walked, granted nothing, not exempt" (real gap) on both CLI ancestor guards, keep the daemon two-state, and extend the cross-mirror test from variant NAME SETS to the CLASSIFICATION RULE; WR-14 (anchor hardening applied to 1 of 2 tests)
- [x] 117-29-PLAN.md — WR-18: CR-01's class-coverage gap — `low_integrity_label_rid` still ignores `AceFlags`; WR-17: remediation names a command that cannot diagnose the cause it names
- [x] 117-30-PLAN.md — WR-20: the WR-01 reordering silently loosened the coverage claim for non-owned paths carrying a third-party mandatory label, with no test and no ledger entry
- [x] 117-31-PLAN.md — WR-13: `content_defines_symbol` lacks the trailing word-boundary check its sibling got in the same plan, so the SPEC's "a renamed enforcing function fails the build" claim is false
- [x] 117-33-PLAN.md — WR-21: `emit_attestation_event` emits outside the chain mutex while `emit_override_event` emits inside it; `SecurityEventLayer::inner` visibility comment no longer explains itself

**Wave 13** *(blocked on Wave 12 — 117-32 consumes the matcher 117-31 produces)*
- [x] 117-32-PLAN.md — WR-19: stale `launch.rs:2190`/`:2194` citation in the SPEC's Manual verification table, outside `spec_matches_registry`'s coverage

**Wave 14** *(blocked on Waves 12-13 — records the whole round, alone per SC4)*
- [x] 117-34-PLAN.md — SC4: record CR-03 + WR-12..WR-21 in the SPEC's Review-fix pass ledger (11 new rows) and add iteration-5 addenda to the 9 continuing prior rows (CR-01, CR-02, WR-01, WR-04, WR-06, WR-07, WR-08, WR-09, WR-11)

**Cross-cutting constraints** (phase-wide invariants cited across multiple plans):
- **D-19** — the supervisor attests; the confined process is never the source of a claim about its own containment.
- **D-28** — layer-specific downgrade detail stays off any channel the confined process can read; the banner is coarse on every arm.
- **D-30** — the fault-injection seam is compiled out of release builds, not merely refused at runtime.
- **D-31 / D-32** — a row that cannot be tested on an ordinary host is a loud named gap; the meta-test **discovers** registry rows rather than naming them.
- **D-35** — cross-target clippy (local `cross` linux-gnu + `cargo-zigbuild` apple-darwin) is a MUST, no PARTIAL→CI.
- **Broker-arm invariant** (added checker pass 3) — every row expected at `EntryPath::Broker` must carry `ContractOutcome::Abort`, enforced by a discovery-based test; otherwise a future row would go unattested on that arm.

### Phase 118: Per-Session Enforcement Receipts
**Goal**: The conjunction "restricted token AND low-integrity label AND AppContainer profile AND WFP coverage" stops being asserted at launch and becomes attested per session — turning "we configured enforcement" into "we can show enforcement held."
**Depends on**: Phase 117 (the contract's layer enumeration is what a receipt attests), Phase 115 (audit event shape)
**Requirements**: RCPT-01, RCPT-02, RCPT-03
**Success Criteria** (what must be TRUE):
  1. Every confined session emits a receipt naming which layers were confirmed active for that process.
  2. A receipt contains no paths, no arguments, and no payload content — verified by a test that fails if a field capable of carrying process content is added, in the shape of the existing self-enforcing source scans.
  3. A receipt's integrity is verifiable on the same terms as the core audit chain — a **keyless SHA-256 hash chain** (D-25) — so an edited or reordered receipt is detectable after the writing process has exited. *(Amended 2026-09-06. Original text read "on the same terms as the existing HMAC-chained audit events". That was inaccurate in both directions: what shipped is keyless per D-25, because `SecurityEventLayer`'s HMAC key is per-process, OsRng-generated and zeroized on `Drop`, so an HMAC-chained receipt would be unverifiable the moment the process exited — and `crates/nono/src/audit.rs`, the actual pre-existing audit chain, is itself keyless. The criterion's INTENT — a downstream consumer can detect an edit — is unchanged; only the named mechanism is corrected. Claim boundary: tamper-EVIDENCE and ordering, never authorship; D-25 forbids any "only the key holder could have produced this" claim. Accepted limitation: a clean tail-record deletion is undetectable by a keyless chain with no external anchor.)*
  4. A reader can distinguish "confirmed active" from "not expected in this configuration" from "expected but unconfirmed" without out-of-band knowledge — an unattested layer never renders as attested.

**Plans**: 10 plans in 6 waves

**Wave 1** *(no dependencies — core primitives, both binaries' foundation)*
- [ ] 118-01-PLAN.md — Core receipt vocabulary (EnforcementReceipt + promoted LayerId) + keyless receipt chain primitive (D-11/D-25) + content-free type-allowlist scan
- [ ] 118-02-PLAN.md — Core security primitives: deny-ACE Win32 wrapper (D-08) + require_receipts machine policy field (D-04)

**Wave 2** *(blocked on Wave 1 completion)*
- [ ] 118-03-PLAN.md — nono.exe full 13-row census (Finding 1a) + receipt assembly + sentinel round-trip + census-completeness meta-test
- [ ] 118-04-PLAN.md — nono-agentd daemon_attest_and_decide collect-all-then-decide restructure (D-26) with equivalence proof + 8-row expectancy table
- [ ] 118-05-PLAN.md — Receipt sink infrastructure: DENY-ACE + NO_READ_UP guard, keyless mutex-guarded chain writer, shared by nono.exe and nono-agentd.exe

**Wave 3** *(blocked on Wave 2 completion — needs 118-03's census shape and 118-05's sink envelope)*
- [ ] 118-06-PLAN.md — Broker wire-contract widening (D-27) + nono-shell-broker's own 13-row census, receipt assembly, and sink/chain write

**Wave 4** *(blocked on Wave 3 completion)*
- [ ] 118-07-PLAN.md — Wire receipt write into nono.exe's DirectCli attestation gate (D-02/D-03/D-04)
- [ ] 118-08-PLAN.md — Wire receipt write into nono-agentd's Daemon attestation gate + D-16 triage

**Wave 5** *(blocked on Wave 4 completion)*
- [ ] 118-09-PLAN.md — nono receipt list|show|verify command family (D-10) + four-state rendering discovery test (RCPT-03)

**Wave 6** *(blocked on Wave 5 completion — **not autonomous**, carries a blocking operator checkpoint)*
- [ ] 118-10-PLAN.md — Phase gate: cross-target clippy (D-23), D-17 latency measurement, confined-child sink-guard checkpoint (D-08), discretion-decision recording

**Cross-cutting constraints** *(appear in 2+ plans' `must_haves`; violating any one invalidates plans beyond the one being executed)*:
- **D-25 (amends D-11)** — the receipt chain is a **keyless** domain-separated SHA-256 construction, mirroring `crates/nono/src/audit.rs::hash_chain`, **not** the telemetry `Hmac<Sha256>`. The integrity claim is "nobody edited this without leaving a hash mismatch" — no code, doc, help text, or CLI output may imply the stronger key-holder claim.
- **D-19** — the supervisor attests; the confined process never does. No receipt field may be populated from a claim made by the process being confined.
- **D-05 + D-14** — strict identity (opaque session id + pid only) is what makes the content-free claim mechanically provable. Enforced by a type-allowlist source scan **and** a per-producer sentinel round-trip; both carry perturbation proofs.
- **D-12** — `LayerId` + `LayerAttestationStatus` are policy-free vocabulary in core; per-arm expectancy, contracted outcome, and enforcing call sites stay in `layer_registry.rs`. The ADR-86 boundary argument is a written deliverable, not an assumption.
- **D-23** — both cross-target clippy gates (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) are required; no PARTIAL→CI fallback.
- **Test-selector discipline** — `-p nono` / `-p nono-cli` do not resolve (real names: `nono-sandbox`, `nono-sandbox-cli`), `nono-cli` has **no lib target**, and the Windows module is bound as `exec_strategy`, not `exec_strategy_windows`. A wrong selector exits 0 having run **zero** tests. Every verify command must assert a non-zero test count.

### Phase 119: Security-Model Boundary Statement + State-of-the-Art Decision Log
**Goal**: What nono governs and what it does not is written down before anyone downstream can over-claim it — and the Windows isolation techniques the fork did *not* adopt become a set of recorded decisions rather than a set of omissions.
**Depends on**: Phase 117 (the boundary statement can only be truthful once the contract says what each layer actually does)
**Requirements**: BOUND-01, BOUND-02, BOUND-03
**Carried defect (flagged 2026-09-06 from the Phase 118 close-out — NOT a BOUND requirement)**: `nono shell`'s interactive rendering is broken on the dev host. The child spawns (`broker: spawned child`, `app_container=false`, `BrokerLaunch` arm) and the broker blocks in `WaitForSingleObject`, but keystrokes never echo and no `child exited` line appears. Distinct from the historical `0xC0000142` TUI block — the process starts fine, only the ConPTY plumbing fails to render. Evidence: `.planning/debug/resolved/broker-receipt-not-written.md` §Residual. **Fit caveat for whoever plans this phase:** this is a functional defect, not a boundary statement — either fix it here or convert it into a recorded limit under BOUND-01, but do not let it lapse by sitting in a docs-only phase.
**Success Criteria** (what must be TRUE):
  1. The security model states that nono governs destination, credential and containment but not payload — naming explicitly that a prompt exfiltrating a secret to an **allowlisted** host is invisible to host-level filtering, so the limit is discoverable without reading the proxy source.
  2. The filesystem guarantee is stated in terms of the shipped mechanism, with per-file read policy inside one directory (`src/` yes, `.env` no) explicitly named as not enforced and ADR-65 cited as the standing reason.
  3. Each of the six §4 techniques — Windows Sandbox/HCS, PPL for protecting nono's own supervisor, WFP ALE layers beyond connect-time, ETW escape-attempt detection, AppContainer capability grants, WDAC exec gating — carries a written ruled-in or ruled-out decision with reasoning.
  4. The PPL item specifically records the supervisor's own protection posture, since a contained process that can tamper with its supervisor is a containment escape.

### Phase 120: Tool-Sandbox Verdict Execution
**Goal**: Phase 116's verdict is carried out, so the tool-sandbox divergence is closed by decision — either absorbed, or formalized as a permanent named boundary a future absorb can reconcile against.
**Depends on**: Phase 116 (scope is defined by its verdict — this phase is deliberately provisional until then)
**Requirements**: TSBX-03
**Success Criteria** (what must be TRUE):
  1. The verdict is executed as written — if *adopt*, the subsystem is absorbed with a `platform/windows.rs` driver and the 7 refinement PRs dispositioned; if *formalize*, the fork-native path is named as a permanent scope boundary in the ADR-111/ADR-113 shape.
  2. A future reader encountering upstream's `tool-sandbox/` finds a recorded decision explaining its absence or its adapted form — not silence.
  3. The divergence ledger's tool-sandbox rows are all closed, with no row left in an open or deferred state without a named successor.
  4. If the verdict cannot be fully executed inside v3.7, the remainder is recorded as FUT-09 with its scope sized from the verdict — never left as an implicit carry-forward.

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
| 112. Security + Residual Sync | v3.6 | 8/8 | Complete (SEC-02 carved out to Phase 114) | 2026-08-05 |
| 113. SPIFFE/SPIRE Workload Identity | v3.6 | 8/8 | Complete | 2026-08-06 |
| 114. OAuth Capture Absorb (SEC-02) | v3.6 | 11/11 | Complete | 2026-08-07 |
| 115. v3.6 Carry-Forward Drain | v3.7 | 6/6 | Complete | 2026-08-09 |
| 116. Tool-Sandbox Divergence Audit + Disposition ADR | v3.7 | 6/6 | Complete (verdict: formalize fork-native) | 2026-08-09 |
| 117. Fail-Direction Contract + Startup Self-Attestation | v3.7 | 34/34 | Complete (SC3 via operator override; 1 human item open) | 2026-08-15 |
| 118. Per-Session Enforcement Receipts | v3.7 | 10/10 | Complete with 2 items OPEN — Task 3's human-verify gate FAILED and narrowed D-08 to write-integrity only (receipts are NOT read-confidential); code review found 6 Criticals, all fixed + re-verified; verifier 4/4 SC, status `human_needed`. CR-05 closed on all 3 producers 2026-09-06 (a583566e, live-verified on the AppContainer arm). OPEN: RCPT-02 requirement text says "HMAC-chained" but a KEYLESS SHA-256 chain shipped (D-25) — needs amendment or an accepted-divergence record | 2026-09-06 |
| 119. Security-Model Boundary Statement + State-of-the-Art Decision Log | v3.7 | 0/? | Not started | - |
| 120. Tool-Sandbox Verdict Execution | v3.7 | 0/? | Not started | - |
