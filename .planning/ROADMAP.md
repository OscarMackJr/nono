---
milestone: v2.13
milestone_name: Carry-Forward Closeout (Dark Factory)
status: shipped
created: 2026-06-17
last_updated: 2026-06-18
granularity: standard
---

# Roadmap — nono

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 01-12 (shipped 2026-03-31) — see [`milestones/v1.0-*`](milestones/)
- ✅ **v2.0 Windows Gap Closure** — Phases 13-18 — see [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md)
- ✅ **v2.1 Resource Limits / Extended IPC / Attach-Streaming** — see [`milestones/v2.1-ROADMAP.md`](milestones/v2.1-ROADMAP.md)
- ✅ **v2.2 Windows/macOS Parity Sweep** — see [`milestones/v2.2-ROADMAP.md`](milestones/v2.2-ROADMAP.md)
- ✅ **v2.3 Linux POC Unblock + Deferreds Closure** — see [`milestones/v2.3-ROADMAP.md`](milestones/v2.3-ROADMAP.md)
- ✅ **v2.4 Complete the Partial Ports + UPST4** — Phases 35, 36, 36.5, 39, 40 (shipped 2026-05-15) — see [`milestones/v2.4-ROADMAP.md`](milestones/v2.4-ROADMAP.md)
- ✅ **v2.5 Backlog Drain + UPST5** — Phases 37, 41, 42, 43 (shipped 2026-05-20) — see [`milestones/v2.5-ROADMAP.md`](milestones/v2.5-ROADMAP.md)
- ✅ **v2.6 UPST6 + v2.5 Drain** — Phases 44, 44.1, 45, 46, 47, 48, 49, 50 (shipped 2026-05-25) — see [`milestones/v2.6-ROADMAP.md`](milestones/v2.6-ROADMAP.md)
- ✅ **v2.7 Windows supervised-run hardening** — Phases 51, 52 (shipped 2026-05-26) — see [`milestones/v2.7-ROADMAP.md`](milestones/v2.7-ROADMAP.md)
- ✅ **v2.8 UPST7 + v2.7 Drain & Release** — Phases 53-59 (shipped 2026-06-06; tags `v2.8`+`v0.57.5`) — see [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md)
- ✅ **v2.9 Windows Sandbox-the-Tools — Confined Coding Loop** — Phases 60, 61, 62 (published as `v0.62.2` 2026-06-06) — see [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md)
- ✅ **v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity** — Phases 63-66 (shipped 2026-06-11; tag `v2.10`) — see [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md)
- ✅ **v2.11 Clean-Host Distribution Cleanup + UPST8** — Phases 67-70 (shipped 2026-06-13) — see [`milestones/v2.11-ROADMAP.md`](milestones/v2.11-ROADMAP.md)
- ✅ **v2.12 AI Agent Abstraction** — Phases 71-75 (shipped 2026-06-16; tag `v2.12`) — see [`milestones/v2.12-ROADMAP.md`](milestones/v2.12-ROADMAP.md)
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18; tag `v2.13`) — see [`milestones/v2.13-ROADMAP.md`](milestones/v2.13-ROADMAP.md)

## Phases

<details>
<summary>✅ v2.13 Carry-Forward Closeout (Dark Factory) (Phases 76-81) — SHIPPED 2026-06-18</summary>

- [x] Phase 76: Self-Verifying Harness Foundation (2/2, DARK-01) — 2026-06-17
- [x] Phase 77: Copilot CLI End-to-End Confinement (4/4, CPLT-01/02/03) — 2026-06-17
- [x] Phase 78: Cross-Process Classification (2/2, CLAS-01/02) — 2026-06-18
- [x] Phase 79: WFP Egress Isolation + nono-ts Ergonomics (2/2, WFP-01/TSRG-01) — 2026-06-18
- [x] Phase 80: Clean-Host Install UAT (2/2, INST-01) — 2026-06-18
- [x] Phase 81: Milestone Close Aggregator (1/1, DARK-02) — 2026-06-18

10/10 requirements satisfied; milestone audit `tech_debt` (no requirement gaps, 0 wiring defects;
host-execution deferrals only). Dark Factory mandate met: every host-gated item collapses to a
single unattended `scripts/verify-dark.ps1` gate; the no-flag aggregator (`_aggregate.json`) is the
machine-readable close signal. Full detail: [`milestones/v2.13-ROADMAP.md`](milestones/v2.13-ROADMAP.md).

</details>

<details>
<summary>✅ v2.12 AI Agent Abstraction (Phases 71-75) — SHIPPED 2026-06-16</summary>

- [x] Phase 71: Engine-Agnostic Launch Productionization (5/5, ENG-01/02/03) — 2026-06-14
- [x] Phase 72: nono-py Binding + In-Process-Exec Proof (4/4, ABI-01/02) — 2026-06-14
- [x] Phase 73: AI_AGENT Marker (3/3, MARK-01) — verified 2026-06-16
- [x] Phase 74: Persistent Multi-Tenant Daemon (8/8, DMON-01/02/03) — 2026-06-15 *(marquee)*
- [x] Phase 75: Supplementary Controls + Secondary Engines (8/8, SUPP-01/02/03) — 2026-06-16

12/12 requirements satisfied; milestone audit PASSED. Engine-neutral confinement + persistent
multi-tenant daemon + unforgeable AI_AGENT marker + per-agent WFP/demote + nono-ts parity.
SC3 re-scope: Copilot confine-only (Node-ESM/AppContainer limit); claude-code is Engine-2.
Full detail: [`milestones/v2.12-ROADMAP.md`](milestones/v2.12-ROADMAP.md).

</details>

<details>
<summary>✅ v2.11 Clean-Host Distribution Cleanup + UPST8 (Phases 67-70) — SHIPPED 2026-06-13</summary>

- [ ] Phase 67: Clean-Host Windows Install (DIST-01/02, TRUST-01/02) — host-gated UAT closed in v2.13 Phase 80
- [x] Phase 68: macOS Resource-Limit Enforcement Fix (2/2) — completed 2026-06-12
- [x] Phase 69: UPST8 Audit (1/1) — completed 2026-06-13
- [x] Phase 70: UPST8 Cherry-pick Sync (3/3) — completed 2026-06-13

Phases 68/69/70 complete; Phase 67 carried to v2.13 Phase 80. Full detail: [`milestones/v2.11-ROADMAP.md`](milestones/v2.11-ROADMAP.md).

</details>

<details>
<summary>✅ v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity (Phases 63-66) — SHIPPED 2026-06-11</summary>

- [x] Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit (3/3) — completed 2026-06-08
- [x] Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave (5/5) — completed 2026-06-09
- [x] Phase 65: Minifilter ADR + macOS Live Re-validation (4/4) — completed 2026-06-11
- [x] Phase 66: WR-02 EDR HUMAN-UAT (1/1) — completed 2026-06-11

9/9 reqs satisfied. Full detail: [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md).

</details>

<details>
<summary>✅ v2.8 / v2.9 (Phases 53-62) — SHIPPED 2026-06-06</summary>

v2.8 UPST7 + v2.7 Drain & Release (Phases 53-59, tags `v2.8`+`v0.57.5`); v2.9 Windows Sandbox-the-Tools (Phases 60-62, published `v0.62.2`). Full detail: [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md) / [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md).

</details>

## References

- `.planning/PROJECT.md` — project context + current milestone scope.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.13).
- `.planning/milestones/v2.13-ROADMAP.md` — archived v2.13 roadmap (Phases 76-81) with full phase details + success criteria.
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for cfg-gated Unix code.
- `proj/DESIGN-engine-abstraction.md` — E1-E5 engine-abstraction contract (Phase 72).
