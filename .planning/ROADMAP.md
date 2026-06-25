---
milestone: v3.3
milestone_name: UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release
status: active
updated: 2026-06-25
---

# Roadmap: nono

## Milestones

- 🔄 **v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release** — Phases 94-97 (active 2026-06-25)
- ✅ **v3.2 Signed Policy Overrides (ZT-Infra Attestation)** — Phases 91-93 (shipped 2026-06-23) — [archive](milestones/v3.2-ROADMAP.md)
- ✅ **v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain** — Phases 85-90 (shipped 2026-06-21) — [archive](milestones/v3.1-ROADMAP.md)
- ✅ **v3.0 Enterprise Hardening I — Deploy · Control · Compliance** — Phases 82-84 (shipped 2026-06-19) — [archive](milestones/v3.0-ROADMAP.md)
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18) — [archive](milestones/v2.13-ROADMAP.md)

> Earlier milestones (v2.5–v2.12) are archived under `.planning/milestones/`.

## Phases

<details>
<summary>🔄 v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release (Phases 94-97) — ACTIVE</summary>

Drain-then-sync upstream milestone: audit and absorb the `nolabs-ai/nono` `v0.64.0..v0.65.1` window (v0.64.1, v0.65.0, v0.65.1) without regressing the Windows security model; stand up a local cross C toolchain to retire the PARTIAL→CI debt; then make the workspace genuinely releasable — crate leapfrog ≥ `0.65.0`, a gated build+sign+dry-run pipeline, and a one-step operator push runbook. Release scope = PREPARE ONLY (the actual push/publish is operator-gated manual step outside the milestone).

- [ ] **Phase 94: UPST10 Divergence Audit** — 0/TBD plans
- [ ] **Phase 95: Upstream Absorb + Fork-Invariant Verify** — 0/TBD plans
- [ ] **Phase 96: Cross-Target Toolchain** — 0/TBD plans
- [ ] **Phase 97: Release Engineering — Leapfrog + Pipeline + Runbook** — 0/TBD plans

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

### Phase 94: UPST10 Divergence Audit
**Goal**: The fork has a complete, actionable DIVERGENCE-LEDGER for the `nolabs-ai/nono` `v0.64.0..v0.65.1` window and the upstream remote points at the new canonical source.
**Depends on**: Nothing (first phase of v3.3)
**Requirements**: UPST10-01, UPST10-04
**Success Criteria** (what must be TRUE):
  1. A `DIVERGENCE-LEDGER` document exists for `v0.64.0..v0.65.1` (covering v0.64.1, v0.65.0, v0.65.1) with every commit classified into will-sync / fork-preserve / won't-sync / split clusters, a `windows-touch` flag per commit, and a per-cell ADR-review verdict.
  2. The git `upstream` remote and PROJECT.md `## Upstream Parity Process` both reference `nolabs-ai/nono` (not the former `always-further/nono`), with a Future Cycles stub noting the next sync trigger past v0.65.1.
  3. Each cluster's disposition is justified by one of the three criteria: security impact, Windows-backend touch, or library-boundary relevance — no cluster is left with a bare `TBD` verdict.
**Plans**: TBD

### Phase 95: Upstream Absorb + Fork-Invariant Verify
**Goal**: All will-sync clusters from the Phase 94 ledger are absorbed into the fork and the Windows security model is provably unregressed.
**Depends on**: Phase 94
**Requirements**: UPST10-02, UPST10-03
**Success Criteria** (what must be TRUE):
  1. Every commit in will-sync clusters is present in the fork (cherry-picked with `-x` trailer or manual-replayed), DCO-signed (`Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`), with no `will-sync` row left open in the DIVERGENCE-LEDGER.
  2. `make build` and `make test` pass on the dev host (Windows) on the post-sync tree with no new test failures introduced by the cherry-picks.
  3. Fork-divergent invariants — the AppContainer/WFP/broker Windows backend, the ADR-86 audit/diagnostics library-boundary carve-out, and the `exec_strategy_windows/` denial-rendering fork — are each explicitly verified post-sync (checklist entry per invariant, none marked regressed).
  4. Any security-relevant will-sync commit (network filtering, seccomp, path-handling) has a dedicated verification note confirming the fork's Windows equivalents remain intact.
**Plans**: TBD

### Phase 96: Cross-Target Toolchain
**Goal**: The dev host can run `linux-gnu` clippy locally, retiring the automatic PARTIAL→CI default for that gate; the `apple-darwin` gate outcome (pass or documented hard-blocker) is explicitly resolved.
**Depends on**: Phase 95
**Requirements**: XTGT-01, XTGT-02, XTGT-03, XTGT-04
**Success Criteria** (what must be TRUE):
  1. A local cross C toolchain (cross/Docker or equivalent) is installed and documented with setup steps + the exact invocation command, sufficient for another developer to reproduce on the same Windows host.
  2. `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` runs to completion locally and exits 0; any drift found in cfg-gated Unix code is fixed before this criterion is satisfied.
  3. The apple-darwin gate is either: (a) passing locally with the same invocation pattern, or (b) accompanied by a written hard-blocker record (osxcross/SDK infeasibility from Windows) that explicitly commits apple-darwin to PARTIAL→CI with rationale — either outcome closes XTGT-03.
  4. CLAUDE.md and `.planning/templates/cross-target-verify-checklist.md` are updated to reflect which gates are now locally runnable, retiring the PARTIAL→CI *default* for those gates.
**Plans**: TBD

### Phase 97: Release Engineering — Leapfrog + Pipeline + Runbook
**Goal**: The workspace is one operator push away from a fully published release — all versions bumped, all artifacts built and signed, all publish paths dry-run GREEN, with a documented runbook for the final step.
**Depends on**: Phase 95 (sync complete before bumping); Phase 96 (cross-target verification covers the release tree)
**Requirements**: RLS-05, RLS-06, RLS-07, RLS-08, RLS-09
**Success Criteria** (what must be TRUE):
  1. All 5 workspace crates (`nono`, `nono-cli`, `nono-proxy`, `nono-shell-broker`, `nono-ffi`) and both binding manifests (`nono-py/Cargo.toml`, `nono-ts/package.json`) carry the leapfrogged version ≥ `0.65.0`; internal path-dep `version` pins are consistent across every `Cargo.toml`; `Cargo.lock` is regenerated and `make build` is green.
  2. The release pipeline produces signed Windows machine + user MSIs (payload signed before WiX harvest, with an admin-extract verify gate), `nono-py` wheels, and `nono-ts` native packages — all reproducible from a single tag.
  3. `cargo publish --dry-run` completes without errors for each workspace crate in dependency order; `twine check` (or `maturin build` validation) and `npm publish --dry-run` also pass — no live registry push occurs.
  4. `release.yml` runs (or is locally validated to produce) a GitHub Release carrying signed MSI + binary assets with no `0s startup_failure` and all required build legs green.
  5. A documented operator runbook exists, a green release-readiness gate confirms the workspace is publish-ready, and the runbook embeds the PUBLIC-repo pre-push checklist (no `build_notes/`/`.gsd/` staged; crate leapfrog ≥ `0.65.0` confirmed; operator push = sole remaining action).
**Plans**: TBD
**UI hint**: no

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
| 94. UPST10 Divergence Audit | v3.3 | 0/TBD | Not started | - |
| 95. Upstream Absorb + Fork-Invariant Verify | v3.3 | 0/TBD | Not started | - |
| 96. Cross-Target Toolchain | v3.3 | 0/TBD | Not started | - |
| 97. Release Engineering — Leapfrog + Pipeline + Runbook | v3.3 | 0/TBD | Not started | - |
