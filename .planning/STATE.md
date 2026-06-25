---
gsd_state_version: 1.0
milestone: v3.3
milestone_name: UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release
status: executing
stopped_at: Phase 94 context gathered
last_updated: "2026-06-25T23:45:15.993Z"
last_activity: 2026-06-25 -- Phase 94 planning complete
progress:
  total_phases: 1
  completed_phases: 0
  total_plans: 2
  completed_plans: 0
  percent: 0
---

# Project State: nono — v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release

## Project Reference

See: `.planning/PROJECT.md` (v3.3 milestone active 2026-06-25; v3.2 Phases 91-93 complete + archived; tag `v3.2` local). Phase numbering continues from Phase 93 (Phases 94-97 — NOT reset). Roadmap: `.planning/ROADMAP.md`. Requirements: `.planning/REQUIREMENTS.md`.

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms. The fork stays current with upstream without regressing its Windows security model — and is, for the first time, genuinely releasable: a gated, signed, multi-registry pipeline prepared GREEN for a one-step operator push.

**Current Focus:** Phase 94 — UPST10 Divergence Audit (not yet started; run `/gsd:plan-phase 94`).

## Current Position

```
Phase 94 of 97 | Plan: — | Status: Not started
[                                        ] 0%
```

Phase: 94 — UPST10 Divergence Audit
Plan: —
Status: Ready to execute
Last activity: 2026-06-25 -- Phase 94 planning complete

## Performance Metrics

**Velocity:** (v3.3 — reset; populated as phases complete)

- Total plans completed: 0
- Average duration: —
- Total execution time: —

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|

*Updated after each plan completion*

## Accumulated Context

### Key Decisions (v3.3 roadmap)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| 4 phases (94-97), not 2-3 | all | Three distinct concerns (audit, absorb, cross-target, release) each have a clean delivery boundary and different risk profiles; collapsing absorb+release creates a dependency inversion (version bump must come after sync). |
| UPST10-04 (remote relocation) folded into Phase 94 | 94 | The `nolabs-ai/nono` rename is audit-setup work — done at audit-open when fetching commits; a separate phase would be artificial. |
| Version leapfrog (RLS-05) in Phase 97, after Phase 95 sync | 97 | Bump once, post-sync, to a clean ≥ 0.65.0; bumping mid-sync creates a rebasing treadmill and dirty Cargo.lock during cherry-picks. |
| Cross-target (Phase 96) sequenced after Phase 95 sync | 96 | XTGT clippy gates should run against the synced + post-sync tree, not a pre-sync snapshot that will change. |
| Release scope = PREPARE ONLY | 97 | Preserves LOCAL-ONLY posture; repo PUBLIC pending Microsoft minifilter-altitude approval; actual push/publish is operator-gated manual step outside this milestone. |

### Pending Todos

None yet.

### Blockers/Concerns

- **Repo stays PUBLIC**: verify no `build_notes/` or `.gsd/` files staged before any `git push` (minifilter-altitude approval pending). All tags remain LOCAL ONLY; push is operator-gated.
- **Upstream relocated**: canonical upstream is now `nolabs-ai/nono` (was `always-further/nono`); Phase 94 updates the remote and PROJECT.md.
- **Cross-target clippy**: XTGT-03 (apple-darwin) explicitly allows a documented hard-blocker outcome if osxcross/SDK is infeasible from Windows. Phase 96 resolves the outcome either way.
- **Cross-repo release**: nono-py at `../nono-py`, nono-ts at `../nono-ts`. Phase 97 version bump must touch both sibling repos.
- **PARTIAL→CI carry-forwards**: SEC-01/SEC-02 (v3.1), ZTL-04 AWS_* strip (v3.2) — still PARTIAL→CI; Phase 96 may resolve if linux-gnu toolchain clears them.
- **All commits DCO-signed**: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` required on every commit including cherry-picks (use `-x` + manual DCO trailer).

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260624-p1c | Cargo Audit: bump quinn-proto past RUSTSEC-2026-0185 (remote memory exhaustion) | 2026-06-24 | 78b50f04 | [260624-p1c-cargo-audit-bump-quinn-proto-past-rustse](./quick/260624-p1c-cargo-audit-bump-quinn-proto-past-rustse/) |
| 260624-q98 | Remove orphan audit_ledger.rs + dead state_paths helpers (never compiled) | 2026-06-24 | e350df23 | [260624-q98-remove-orphan-audit-ledger-rs-and-its-de](./quick/260624-q98-remove-orphan-audit-ledger-rs-and-its-de/) |
| 260624-q9j | Fix red Docs Checks: force-add already-in-nav windows-win-1706-option-1-workstream.mdx | 2026-06-24 | 3475b470 | [260624-q9j-exclude-docs-cli-development-from-docs-c](./quick/260624-q9j-exclude-docs-cli-development-from-docs-c/) |
| 260625-crs | Phase 83 deferred code-review findings: WR-02/03/04/05 + IN-01/IN-03 (interpreter PATH-hijack, GetWindowsDirectoryW, canonical expander, validate(), gate probe, SID regex) | 2026-06-25 | 4af1e8f9 | [260625-crs-address-phase-83-code-review-deferred-fi](./quick/260625-crs-address-phase-83-code-review-deferred-fi/) |

## Deferred Items

Items acknowledged and deferred at **v3.2 close (2026-06-23)** — `gsd-sdk query audit-open` reported 47 open artifacts, user acknowledged-all. All historical or host-gated; none blockers:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Historical | 36 open quick-tasks (Mar–Apr 2026 dates, all `missing`/cleaned-up) | Acknowledged | v3.2 close |
| Historical | 6 seeds SEED-001…006 (all consumed or dormant; SEED-005 = v3.2 scope, delivered) | Acknowledged | v3.2 close |
| Historical | 4 empty/"None" todo parse artifacts | Acknowledged | v3.2 close |
| Host-gated | OVERRIDE-02 (DF-02) live allow/revoke proof — needs ZT-Infra provisioner + openssl + elevated session; SKIP_HOST_UNAVAILABLE by design | Open (host-gated) | v3.2 close |
| PARTIAL→CI | Cross-target clippy (linux-gnu + apple-darwin) for ZTL-04 `AWS_*` strip | Open (CI-decisive; may resolve in Phase 96) | v3.2 close |

Prior carry-forwards from v3.1 close (2026-06-21): SEC-01/SEC-02 AF_UNIX+procfs guards (PARTIAL→CI), DRAIN-01/02/03 live host-gated UAT, 2 env-sensitive Phase-74 DACL-guard tests.

## Session Continuity

Last session: 2026-06-25T23:25:53.326Z
Stopped at: Phase 94 context gathered
Resume file: .planning/phases/94-upst10-divergence-audit/94-CONTEXT.md

## Operator Next Steps

- Run `/gsd:plan-phase 94` to plan the UPST10 divergence audit
