---
milestone: v2.10
milestone_name: Kernel-Driver Spike + EDR UAT + macOS Upstream Parity
status: shipped
created: 2026-05-28
last_updated: 2026-06-11
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

## Phases

<details>
<summary>✅ v2.8 UPST7 + v2.7 Drain & Release (Phases 53-59) — SHIPPED 2026-06-06</summary>

- [x] Phase 53: Release & Drain (3/4) — completed 2026-05-29 (shipped v0.57.5)
- [x] Phase 54: UPST7 Audit (1/1) — completed 2026-06-04
- [x] Phase 55: UPST7 Cherry-pick Wave (7/7) — completed 2026-06-05
- [x] Phase 56: Fine-grained Network Filtering (4/4) — completed 2026-06-05
- [x] Phase 57: Bitwarden Credential Source (1/1) — completed 2026-06-05
- [x] Phase 58: Session Lifecycle Hooks (3/3) — completed 2026-06-06
- [x] Phase 59: Supervisor IPC Robustness (3/3) — completed 2026-06-06

Audit: `tech_debt`, 10/10 reqs satisfied, 0 blockers. Full detail: [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md).

</details>

<details>
<summary>✅ v2.9 Windows Sandbox-the-Tools — Confined Coding Loop (Phases 60-62) — PUBLISHED as v0.62.2 2026-06-06</summary>

- [x] Phase 60: Confined Coding Loop (3/3) — completed 2026-05-29
- [x] Phase 61: Ship/Release v2.9 (4/4) — completed 2026-06-06 (published v0.62.2)
- [x] Phase 62: WFP kernel network enforcement — Windows supervised (13/13) — completed 2026-06-03

Separate initiative from UPST7 (builds on merged PR #4). The v0.62.0/v0.62.1 release attempts failed on two latent cfg-gated cross-target compile errors (E0716 + edition-2024 let-chain), fixed in `4de294e8`+`7bb7c7e3` → v0.62.2 published. Full detail: [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md).

</details>

<details>
<summary>✅ v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity (Phases 63-66) — SHIPPED 2026-06-11</summary>

- [x] Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit (3/3) — completed 2026-06-08
- [x] Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave (5/5) — completed 2026-06-09
- [x] Phase 65: Minifilter ADR + macOS Live Re-validation (4/4) — completed 2026-06-11 (D-11c CI green; gate-65-A Seatbelt PASS; go/no-go ADR **Accepted** — No-go/Conditional-go)
- [x] Phase 66: WR-02 EDR HUMAN-UAT (1/1) — completed 2026-06-11 (**WR-02 CLOSED** under Sysmon+Defender EDR-proxy)

9/9 reqs satisfied (DRV-01..04, EDR-01..02, MACOS-01..03). DRV-PROD-01 (production driver) deferred to v2.11/v3.0 per ADR-65. Full detail: [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md).

</details>

## Phase Details

<details>
<summary>✅ v2.8 UPST7 + v2.7 Drain & Release — Phase Details (archived)</summary>

See [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md) for full phase detail blocks.

</details>

<details>
<summary>✅ v2.9 Windows Sandbox-the-Tools — Phase Details (archived)</summary>

See [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md) for full phase detail blocks.

</details>

<details>
<summary>✅ v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity — Phase Details (archived)</summary>

See [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md) for full phase detail blocks (Phases 63-66).

</details>

## Progress

All v2.10 phases (63-66) complete — see the collapsed milestone summary above. Per-phase detail archived in [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md).

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 63. Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit | 3/3 | Complete | 2026-06-08 |
| 64. Minifilter Spike Implementation + macOS P1 Cherry-pick Wave | 5/5 | Complete | 2026-06-09 |
| 65. Minifilter ADR + macOS Live Re-validation | 4/4 | Complete | 2026-06-11 |
| 66. WR-02 EDR HUMAN-UAT | 1/1 | Complete | 2026-06-11 |

## Future Cycles

### UPST8 — Upstream v0.59.0… sync audit (placeholder)

**Goal**: Audit upstream `v0.59.0..<next-tag>` divergence per the Phase 33 ADR `continue` cadence rule. Inherits the audit-shape template from Phase 33 + 39 + 42 + 47 + 54 verbatim. The first deferred-from-UPST7 targets are **v0.60.0 (`9a05a4ff`), v0.61.0, and v0.61.1** (the 2026-06-04 UPST7 re-fetch surfaced all three past the locked `v0.57.0..v0.59.0` range; the deferred set is `v0.60.0..v0.61.1`, NOT v0.60.0 alone — and NOT the unrelated Feb-2026 v0.6.x tag line). Title may flip from `sync audit` to `sync execution` if the next cycle's commit set is small enough to skip a dedicated audit (auditor's call at UPST8 plan-phase). Note: v2.10 absorbs the macOS-relevant slice of v0.60.0..v0.61.2 (Phases 63-65); the non-macOS UPST8 clusters remain deferred here.
**Depends on**: Phase 55 (UPST7 cherry-pick wave closed; cadence rule preserves linear ordering)
**Plans**: 0 / TBD
**Reference**: `docs/architecture/upstream-parity-strategy.md` § Future audit cadence

UPST8 fires when the maintainer decides the accumulated cherry-pick labor (v0.60.0..v0.61.1 deferred at Phase 54; will grow before UPST8 fires) warrants absorbing.

### Carried v2-deferred requirements (from v2.8)

These were defined but deferred during v2.8 (not yet milestone-scoped):

- **REQ-WSRH-AUDIT-01** — profile-wide audit of which heavy-runtime binaries hit the `WriteRestricted` gate.
- **REQ-RLS-ATTEST-01** — evaluate `actions/attest-build-provenance` vs the existing sigstore/TUF + Authenticode pipeline.
- **REQ-UPST-RESID-01** — residual v0.44–v0.57 macОS-learn-diagnostics refactors (`b5f0a3ab`, `bbdf7b85`, `wiring.rs`).
- **REQ-DENY-PREFLIGHT-01** — Linux-host-gated `validate_deny_overlaps` preflight investigation (security equivalence already proven).
- **REQ-UNDO-TOCTOU-01** — full fd-relative TOCTOU hardening of `validate_restore_target` (standalone security phase, ~2-3 wk).

## Next

**v2.10 is shipped (tag `v2.10`, 2026-06-11).** No active phase. Start the next milestone with `/gsd:new-milestone` — the strongest candidates are the 3 v2.11 carry-forward todos (untrusted-POC-cert broker on clean host, MSI VC++ prereq, macOS resl enforcement defect) plus the **UPST8** upstream sync and **DRV-PROD-01** (gated on ADR-65 — currently No-go/Conditional-go) below.

## References

- `.planning/PROJECT.md` — project context + current state.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.10).
- `.planning/milestones/v2.10-REQUIREMENTS.md` — archived v2.10 requirements (DRV-01..04, EDR-01..02, MACOS-01..03).
- `.planning/research/SUMMARY.md` — HIGH-confidence research; build-order recommendations.
- `.planning/research/ARCHITECTURE.md` — integration points per theme.
- `.planning/research/PITFALLS.md` — pitfall→phase ownership; BSOD guards; cross-target drift guards.
- `.planning/milestones/v2.8-ROADMAP.md` / `v2.8-REQUIREMENTS.md` / `v2.8-MILESTONE-AUDIT.md` — archived v2.8.
- `.planning/milestones/v2.9-ROADMAP.md` / `v2.9-REQUIREMENTS.md` — archived v2.9.
