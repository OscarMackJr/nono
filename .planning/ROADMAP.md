---
milestone: v3.3
milestone_name: UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release
status: shipped
updated: 2026-06-26
---

# Roadmap: nono

## Milestones

- ✅ **v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release** — Phases 94-97 (shipped 2026-06-26) — [archive](milestones/v3.3-ROADMAP.md)
- ✅ **v3.2 Signed Policy Overrides (ZT-Infra Attestation)** — Phases 91-93 (shipped 2026-06-23) — [archive](milestones/v3.2-ROADMAP.md)
- ✅ **v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain** — Phases 85-90 (shipped 2026-06-21) — [archive](milestones/v3.1-ROADMAP.md)
- ✅ **v3.0 Enterprise Hardening I — Deploy · Control · Compliance** — Phases 82-84 (shipped 2026-06-19) — [archive](milestones/v3.0-ROADMAP.md)
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18) — [archive](milestones/v2.13-ROADMAP.md)

> Earlier milestones (v2.5–v2.12) are archived under `.planning/milestones/`.

## Phases

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
