---
gsd_state_version: 1.0
milestone: v3.2
milestone_name: Signed Policy Overrides (ZT-Infra Attestation)
status: Awaiting next milestone
stopped_at: v3.2 milestone completed and archived
last_updated: "2026-06-23T10:34:06.470Z"
last_activity: 2026-06-23 — Milestone v3.2 completed and archived
progress:
  total_phases: 3
  completed_phases: 3
  total_plans: 13
  completed_plans: 13
  percent: 100
---

# Project State: nono — v3.2 Signed Policy Overrides (ZT-Infra Attestation)

## Project Reference

See: `.planning/PROJECT.md` (v3.2 milestone active 2026-06-21; v3.1 Phases 85-90 complete + archived; tag `v3.1` local). Phase numbering continues from Phase 90 (Phases 91-93 — NOT reset). Scope source: `.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md`. Roadmap: `.planning/ROADMAP.md`.

**Core Value:** A false-positive nono block must be resolvable by a cryptographically-signed, ledgered, non-self-service exception — never by disabling the sandbox.

**Current Focus:** Planning next milestone — v3.2 SHIPPED + ARCHIVED 2026-06-23 (all 3 phases 91-93 complete; VFY-01 b + VFY-03 a [BLOCKING-93] both closed). Run `/gsd-new-milestone`.

## Current Position

Phase: Milestone v3.2 complete
Plan: —
Status: Awaiting next milestone
Last activity: 2026-06-24 — Completed quick task 260624-p1c: bump quinn-proto past RUSTSEC-2026-0185

## Performance Metrics

**Velocity:** (v3.2 — reset; populated as phases complete)

- Total plans completed: 13
- Average duration: —
- Total execution time: —

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|

*Updated after each plan completion*

## Accumulated Context

### Decisions (v3.2 roadmap)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| 3 phases (not 4) — jti in Phase 91+92, DAAL in Phase 93 | all | jti single-use escalated to v1 (VFY-06); DAAL async/non-blocking (ZTL-05) fits the live-integration phase; all 28 reqs cover in 91/92/93 with no orphans. Research's optional Phase 94 hardening is not needed because jti is already v1 scope. |
| Mutation + audit fused into Phase 92 (never split) | 92 | An override that applies without emitting a SecurityEventLayer event is silent privilege escalation (AUD-04). Splitting creates a shippable-but-silent window. Both land in Phase 92 atomically. |
| DF-01 offline gate attaches to Phase 92, DF-02 live gate to Phase 93 | 92/93 | Dark Factory gates verify the phase whose behavior they gate: offline verify path is exercisable at Phase 92 close; live AWS/KMS path is exercisable only at Phase 93 close. |
| All override logic in nono-py; core gets only AuditEventPayload variant | all | CLAUDE.md library boundary (policy-free core). VFY-01 two-key AND gate, ZTL-01 live decision, CLI-01/02 UX — all nono-py or nono-cli. Core: `PolicyOverrideApplied` variant + reuse of `trust/signing::verify_keyed_signature`. |
| Python urllib.request for POST /actions (not ureq in Rust) | 93 | Research SUMMARY Tension-2 resolved: stdlib, zero new deps, easily mockable. Re-evaluate ureq only if mTLS fragility appears — explicit checkpoint at Phase 93 plan time. |

### Pending Todos

None.

### Blockers/Concerns

- **Repo stays PUBLIC**: verify no `build_notes/` or `.gsd/` files staged before any `git push` (minifilter-altitude approval pending). All v3.2 commits + the `v3.2` tag are LOCAL ONLY; push is operator-gated.
- **Milestone-marker only**: no crate publish; a future release must leapfrog the crate version to ≥ `0.65.0`.
- **Cross-target clippy (PARTIAL→CI)**: the ZTL-04 `AWS_*` strip in `crates/nono-cli/src/exec_strategy/env_sanitization.rs` is verified native-Windows only; linux-gnu + apple-darwin clippy deferred to CI (host lacks cross C compiler), per CLAUDE.md MUST/NEVER. (Resolved-at-close: the milestone was overwhelmingly additive core/nono-py work; native `cargo build`/`clippy` green on both crates.)

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260624-p1c | Cargo Audit: bump quinn-proto past RUSTSEC-2026-0185 (remote memory exhaustion) | 2026-06-24 | 78b50f04 | [260624-p1c-cargo-audit-bump-quinn-proto-past-rustse](./quick/260624-p1c-cargo-audit-bump-quinn-proto-past-rustse/) |

## Deferred Items

Items acknowledged and deferred at **v3.2 close (2026-06-23)** — `gsd-sdk query audit-open` reported 47 open artifacts, user acknowledged-all (Proceed without audit). All historical or host-gated; none blockers:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Historical | 36 open quick-tasks (Mar–Apr 2026 dates, all `missing`/cleaned-up) | Acknowledged | v3.2 close |
| Historical | 6 seeds SEED-001…006 (all consumed or dormant; SEED-005 = v3.2 scope, delivered) | Acknowledged | v3.2 close |
| Historical | 4 empty/"None" todo parse artifacts | Acknowledged | v3.2 close |
| Host-gated | OVERRIDE-02 (DF-02) live allow/revoke proof — needs ZT-Infra provisioner + openssl + elevated session; SKIP_HOST_UNAVAILABLE by design | Open (host-gated) | v3.2 close |
| PARTIAL→CI | Cross-target clippy (linux-gnu + apple-darwin) for ZTL-04 `AWS_*` strip — host lacks cross C compiler | Open (CI-decisive) | v3.2 close |

Prior carry-forwards from v3.1 close (2026-06-21, see `.planning/milestones/v3.1-MILESTONE-AUDIT.md`) remain deferred: SEC-01/SEC-02 AF_UNIX+procfs guards (PARTIAL→CI), DRAIN-01/02/03 live host-gated UAT, 2 env-sensitive Phase-74 DACL-guard tests.

## Session Continuity

Last session: 2026-06-23 — v3.2 milestone completed and archived
Stopped at: v3.2 shipped; awaiting next milestone
Resume file: — (start next milestone via /gsd-new-milestone)

## Operator Next Steps

- Start the next milestone with /gsd-new-milestone
