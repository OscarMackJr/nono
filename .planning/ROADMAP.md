---
milestone: v2.6
milestone_name: TBD
status: planning
created: 2026-05-20
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
- 📋 **v2.6 TBD** — scope to be defined via `/gsd-new-milestone v2.6`

## Phases

<details>
<summary>✅ v2.5 Backlog Drain + UPST5 (Phases 37, 41, 42, 43) — SHIPPED 2026-05-20</summary>

- [x] Phase 37: Linux RESL backends + PKGS auto-pull (6/6 plans) — completed 2026-05-20
- [x] Phase 41: CI cleanup + v24 broker code-review closure (11/10 plans) — completed 2026-05-16
- [x] Phase 42: UPST5 audit (1/1 plan) — completed 2026-05-17
- [x] Phase 43: UPST5 sync execution (7/7 plans) — completed 2026-05-19

Full details: [`milestones/v2.5-ROADMAP.md`](milestones/v2.5-ROADMAP.md)
Requirements: [`milestones/v2.5-REQUIREMENTS.md`](milestones/v2.5-REQUIREMENTS.md)
Audit: [`milestones/v2.5-MILESTONE-AUDIT.md`](milestones/v2.5-MILESTONE-AUDIT.md)

</details>

### 📋 v2.6 (Planning)

Scope locked via `/gsd-new-milestone v2.6`. Known forward-cadence items inherited from v2.5 close:

- **UPST6 audit** — see `## Future Cycles` below (cadence trigger met by `v0.55.0` tag fetched 2026-05-17).
- **Phase 38 (REQ-AAHX-HOST-01)** — native re-validation on Linux/macOS, re-deferred from v2.5 (depends on native Linux host availability for UAT).
- **Phase 35 + 36 human-verify backlog** — 11 UAT items + 7 verification items host-blocked at v2.4 close; carry from v2.4/v2.5 chain.
- **Cluster 2 Edition 2024 source migration** — 39 `#[unsafe(no_mangle)]` rewrites in `bindings/c/src/`; deferred from Phase 43 Plan 43-01b DEC-3 per DIVERGENCE-LEDGER split-disposition (commit `79715aa5`).
- **REVIEW.md polish (16 warnings total)** — Phase 37 (10 warnings, including WR-09 OIDC issuer-pin wiring) + Phase 43 (6 warnings, including WR-05 pack-update sync startup-latency CLAUDE.md hit).
- **Phase 41 follow-up todos (5)** — Class D Linux deny-overlap regression, Class E Windows env_vars parallel flakes (2), v24 broker CR-01/02 cross-binding lockstep.

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 37. Linux RESL + PKGS auto-pull | v2.5 | 6/6 | Complete | 2026-05-20 |
| 41. CI cleanup + broker CR | v2.5 | 11/10 | Complete | 2026-05-16 |
| 42. UPST5 audit | v2.5 | 1/1 | Complete | 2026-05-17 |
| 43. UPST5 sync execution | v2.5 | 7/7 | Complete | 2026-05-19 |

(Prior milestones rolled up under `milestones/v*-ROADMAP.md`.)

## Future Cycles

Entries queued for v2.6 per the Phase 33 ADR `### Future audit cadence` rule — "per upstream release, lazily-evaluated". They activate when v2.6 scope locks. **UPST6 cadence trigger met:** `v0.55.0` tag fetched 2026-05-17 during Phase 42 audit-open's `git fetch upstream --tags`.

### Phase TBD-NN: UPST6 — Upstream v0.54.0…+ sync audit

**Goal:** Mirror Phase 33 / Phase 39 / Phase 42 audit shape. Inventory of upstream divergence from v0.54.0 forward (commits accumulated post-Phase 42 audit cutoff `6b00932f`, including the now-shipped v0.55.0 tag). Per-cluster disposition + parity-strategy review against Phase 33 ADR; absorbs the 2 known post-v0.54.0 commits (`fc965ccc chore(deps): bump tokio`, `089cf6a0 chore(deps): bump cosign-installer`) plus any subsequent additions from v0.55.0+.

**Depends on:** Phase 43 (UPST5 execution baseline lands fork at v0.54.0).

**Requirements:** TBD when v2.6 scope locks.

**Plans:** 0 / TBD — to be populated during `/gsd-plan-phase TBD-NN`.

**Estimated effort:** ~1 week (mirrors Phase 39 + Phase 42 sizing).

**Reference:** `.planning/phases/33-windows-parity-upstream-0-52-divergence/` (audit-shape root template), `.planning/phases/39-upst4-audit/` (windows-touch column zero-fire example via git archive), `.planning/phases/42-upst5-audit/` (Phase 42 worked example with windows-touch:yes fires + per-cell L/M/H ADR verdict table + empirical cross-check subsection), `docs/architecture/upstream-parity-strategy.md` § Future audit cadence (Phase 33 ADR cadence rule).
