---
milestone: none
milestone_name: (between milestones — v2.8 + v2.9 shipped)
status: planning_next
created: 2026-05-28
last_updated: 2026-06-06
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

## Future Cycles

### UPST8 — Upstream v0.59.0… sync audit (placeholder)

**Goal**: Audit upstream `v0.59.0..<next-tag>` divergence per the Phase 33 ADR `continue` cadence rule. Inherits the audit-shape template from Phase 33 + 39 + 42 + 47 + 54 verbatim. The first deferred-from-UPST7 targets are **v0.60.0 (`9a05a4ff`), v0.61.0, and v0.61.1** (the 2026-06-04 UPST7 re-fetch surfaced all three past the locked `v0.57.0..v0.59.0` range; the deferred set is `v0.60.0..v0.61.1`, NOT v0.60.0 alone — and NOT the unrelated Feb-2026 v0.6.x tag line). Title may flip from `sync audit` to `sync execution` if the next cycle's commit set is small enough to skip a dedicated audit (auditor's call at UPST8 plan-phase).
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

Both v2.8 and v2.9 are shipped and archived. No active milestone. Start the next cycle with `/gsd:new-milestone` (likely UPST8, or a Phase 58 D-05 Low-IL hook-confinement follow-up). Fresh `REQUIREMENTS.md` is created by `/gsd:new-milestone`.

## References

- `.planning/PROJECT.md` — project context + current state.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.9).
- `.planning/milestones/v2.8-ROADMAP.md` / `v2.8-REQUIREMENTS.md` / `v2.8-MILESTONE-AUDIT.md` — archived v2.8.
- `.planning/milestones/v2.9-ROADMAP.md` / `v2.9-REQUIREMENTS.md` — archived v2.9.
