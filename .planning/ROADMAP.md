---
milestone: v3.4
milestone_name: UPST11 Upstream Sync to v0.66.0 + Release-Reconcile
status: active
updated: 2026-06-30
---

# Roadmap: nono

## Milestones

- 🔄 **v3.4 UPST11 Upstream Sync to v0.66.0 + Release-Reconcile** — Phases 98-100 (active 2026-06-30)
- ✅ **v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release** — Phases 94-97 (shipped 2026-06-26) — [archive](milestones/v3.3-ROADMAP.md)
- ✅ **v3.2 Signed Policy Overrides (ZT-Infra Attestation)** — Phases 91-93 (shipped 2026-06-23) — [archive](milestones/v3.2-ROADMAP.md)
- ✅ **v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain** — Phases 85-90 (shipped 2026-06-21) — [archive](milestones/v3.1-ROADMAP.md)
- ✅ **v3.0 Enterprise Hardening I — Deploy · Control · Compliance** — Phases 82-84 (shipped 2026-06-19) — [archive](milestones/v3.0-ROADMAP.md)
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18) — [archive](milestones/v2.13-ROADMAP.md)

> Earlier milestones (v2.5–v2.12) are archived under `.planning/milestones/`.

## Phases

<details open>
<summary>🔄 v3.4 UPST11 Upstream Sync to v0.66.0 + Release-Reconcile (Phases 98-100) — ACTIVE</summary>

Drain-then-sync upstream milestone: audit and absorb the `nolabs-ai/nono` `v0.65.1..v0.66.0` window (19 PRs) without regressing the Windows security model or the ADR-86 policy-free-library boundary; settle the high-conflict #1225 `NetworkIntent` refactor via an ADR; then reconcile the prepare-only release pipeline, close the nono-py `RouteConfig` PyPI blocker, and leapfrog all workspace crates to `0.66.1` so an operator push is one step away. Cross-target clippy must be GREEN locally on both Unix gates (no PARTIAL→CI). Release scope = PREPARE ONLY.

- [x] **Phase 98: UPST11 Divergence Audit** — 4/4 plans — completed 2026-06-30
- [x] **Phase 99: Upstream Absorb + Fork-Invariant Verify** — 7/7 plans — completed 2026-06-30
- [ ] **Phase 100: Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker** — 0/TBD plans

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

### Phase 98: UPST11 Divergence Audit
**Goal**: The fork has a complete, commit-level DIVERGENCE-LEDGER for the `nolabs-ai/nono` `v0.65.1..v0.66.0` window and the #1225 `NetworkIntent`-vs-`ProxyOnly` disposition is settled by an ADR.
**Depends on**: Nothing (first phase of v3.4)
**Requirements**: UPST11-01
**Success Criteria** (what must be TRUE):
  1. A DIVERGENCE-LEDGER document exists for `v0.65.1..v0.66.0` (the 19 PRs referenced by upstream release-cut #1293) classifying every commit into will-sync / fork-preserve / won't-sync / split clusters, with a `windows-touch` flag per commit and a per-cell ADR-review verdict (continue/escalate) — refining the `260629-toe` quick-task ledger's preliminary per-PR dispositions to per-commit resolution.
  2. The #1225 `NetworkIntent`-vs-`ProxyOnly` adopt-vs-fork-divergence call is settled in an ADR — either full-sync-adopt (precedent: v3.1 Phase 86 boundary-convergence) or a written fork-divergence carve-out with rationale for which fork invariants (policy-free-library boundary ADR-86, Windows WFP/AppContainer backends) would be affected.
  3. Each cluster's disposition is justified by the established criteria (security impact, Windows-backend touch, or library-boundary relevance); no cluster carries a bare TBD verdict.
  4. The audit is the sole deliverable of this phase: no cherry-picks are initiated here; the ledger and ADR gate Phase 99.
**Plans**: 4 plans
- [x] 98-01-PLAN.md — Window fetch, Reproduction block + noise reconciliation + cluster scaffold
- [x] 98-02-PLAN.md — Per-commit cluster classification, re-export scan + ADR risk matrix
- [x] 98-03-PLAN.md — #1225 NetworkIntent-vs-ProxyOnly disposition ADR (proj/ADR-98)
- [x] 98-04-PLAN.md — Expanded six-carve-out re-touch check + ADR-98 cross-reference

### Phase 99: Upstream Absorb + Fork-Invariant Verify
**Goal**: All will-sync clusters from the Phase 98 ledger are absorbed into the fork in dependency order and the Windows security model, policy-free-library boundary, and cross-target clippy gates are provably unregressed.
**Depends on**: Phase 98
**Requirements**: UPST11-02, UPST11-03, UPST11-04
**Success Criteria** (what must be TRUE):
  1. Every commit in will-sync clusters is absorbed: Cluster A full-sync-adopt (ADR-98): 72bcfd66 (#1225 NetworkIntent) + d457ecc3 (#1263 contradictory-flag guard); Cluster C split: cdeeb5b9 (#983 HTTP/2 pool), 46bcfbb9 (#1127 endpoint wiring), 08ca19a8 (#1243 wildcard route); Cluster D: 5b8e94da (#1207 9P warning); each commit DCO-signed (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`) with upstream SHA trailer. tool-sandbox (#1268/#1271/#1253/#1249) is won't-sync (Cluster B — fork lacks tool-sandbox/ dir; carry-forward).
  2. Clusters E (c808f000 #1235 org-ref migration), F (2e64798d #1229 sigstore-trust-root 0.8.0→0.9.0), and G (a4d68189 #1246 proxy docs/X-Nono-Token fix) absorbed. criterion #1232, CI-yaml #1251, docs #1247 are out-of-drift-filter noise — reconcile in Phase 100. Cargo.lock regenerated; make build GREEN.
  3. Local cross-target clippy is GREEN on both Unix gates (`cross clippy` x86_64-unknown-linux-gnu + direct-binary `cargo-zigbuild clippy` x86_64-apple-darwin, `-D warnings -D clippy::unwrap_used`, no PARTIAL→CI); `make ci` (clippy + fmt + tests) is clean on the dev host.
  4. Fork-divergent invariants are explicitly verified post-sync — the AppContainer/WFP/broker Windows backends, the ADR-86 policy-free-library boundary, and the `exec_strategy_windows/` denial-rendering carve-out each have a checklist entry (none marked regressed); a code-review + verifier pass confirm no Windows-backend or boundary regression.
**Plans**: 7 plans
- [x] 99-01-PLAN.md — D-02 SC/REQUIREMENTS reconciliation (stale text correction)
- [x] 99-02-PLAN.md — Cluster A: 72bcfd66 NetworkIntent replay + D-08 deviation tests
- [x] 99-03-PLAN.md — Cluster A: d457ecc3 validate_block_net_conflicts + Phase 89 proxy guard verification
- [x] 99-04-PLAN.md — Clusters D+E: 5b8e94da 9P warning + c808f000 org-ref migration
- [x] 99-05-PLAN.md — Clusters F+G: 2e64798d sigstore-trust-root bump + a4d68189 proxy docs fix
- [x] 99-06-PLAN.md — Cluster C split: cdeeb5b9 pool (APPLY) + 46bcfbb9 endpoint wiring + 08ca21a8 wildcard fix
- [x] 99-07-PLAN.md — Fork-invariant verify gate: cross-target clippy + make ci + D-10 carve-out checklist

### Phase 100: Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker
**Goal**: The workspace is at crate version `0.66.1` (minimal collision-free bump above upstream `0.66.0`), the prepare-only release pipeline is reconciled and gate-GREEN, the nono-py PyPI blocker is closed, and a one-step operator push is the only remaining action.
**Depends on**: Phase 99 (sync complete before bumping; the post-sync tree is what gets released)
**Requirements**: RLS-10, RLS-11, RLS-12, RLS-13
**Success Criteria** (what must be TRUE):
  1. All 5 workspace crates (`nono`, `nono-cli`, `nono-proxy`, `nono-shell-broker`, `nono-ffi`) plus the `nono-py`/`nono-ts` binding manifests carry version `0.66.1`; internal path-dep `version` pins are consistent across every `Cargo.toml`; `Cargo.lock` is regenerated and `make build` passes clean.
  2. The carried-forward nono-py `RouteConfig` PyPI blocker is closed: the missing `endpoint_policy` field is added at `src/policy.rs:743` and `src/proxy.rs:206` so the `nono-py` wheel builds and `twine check` / maturin validation passes for publish.
  3. Upstream CI changes are reconciled against the fork's prepare-only pipeline — #1245 (idempotent `publish-crates` + cross-compile check on release PRs) and #1251 (compile-step mapping fix) adopted or adapted without breaking the existing `release-readiness` verify-dark gate or the signed-MSI build order.
  4. `scripts/release-dry-run.ps1` and the `release-readiness` verify-dark gate both re-run GREEN at `0.66.1`; `RELEASE-RUNBOOK.md` is updated for the `0.66.1` tag embedding the PUBLIC-repo pre-push checklist (no `build_notes/`/`.gsd/` staged; crate version `0.66.1` > upstream `0.66.0` confirmed); the actual tag push + registry publish remain operator-gated.
**Plans**: TBD

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
| 99. Upstream Absorb + Fork-Invariant Verify | v3.4 | 7/7 | Complete   | 2026-06-30 |
| 100. Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker | v3.4 | 0/TBD | Not started | - |
