---
gsd_state_version: 1.0
milestone: v3.2
milestone_name: Signed Policy Overrides (ZT-Infra Attestation)
status: ready_to_plan
stopped_at: Phase 92 executed + verified (PASS-WITH-PARTIALS)
last_updated: "2026-06-22T00:00:00.000Z"
last_activity: 2026-06-22 -- Phase 92 executed (4/4 plans) + verified; next is plan Phase 93
progress:
  total_phases: 3
  completed_phases: 2
  total_plans: 7
  completed_plans: 7
  percent: 67
---

# Project State: nono — v3.2 Signed Policy Overrides (ZT-Infra Attestation)

## Project Reference

See: `.planning/PROJECT.md` (v3.2 milestone active 2026-06-21; v3.1 Phases 85-90 complete + archived; tag `v3.1` local). Phase numbering continues from Phase 90 (Phases 91-93 — NOT reset). Scope source: `.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md`. Roadmap: `.planning/ROADMAP.md`.

**Core Value:** A false-positive nono block must be resolvable by a cryptographically-signed, ledgered, non-self-service exception — never by disabling the sandbox.

**Current Focus:** Phase 93 — live-zt-infra-integration (next: plan-phase). Phases 91 + 92 complete.

## Current Position

Phase: 93
Plan: Not started
Status: Ready to plan
Last activity: 2026-06-22

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:** (v3.2 — reset; populated as phases complete)

- Total plans completed: 7
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

- **Cross-target clippy**: Phase 92 adds `AuditEventPayload::PolicyOverrideApplied` to `crates/nono/src/audit.rs` (cfg-unconditional) + a new arm in the `SecurityEventLayer` match in nono-cli (may have `cfg(windows)` guards). Both Windows AND Linux clippy must pass (CLAUDE.md MUST/NEVER rule). Note this milestone is overwhelmingly nono-py + additive core-crate work — cross-target is lower risk than v3.1's Unix security patches, but the `SecurityEventLayer` match arm must not produce `unreachable_patterns` on Linux CI.
- **Repo stays PUBLIC**: verify no `build_notes/` or `.gsd/` files staged before any `git push` (minifilter-altitude approval pending).
- **Milestone-marker only**: no crate publish; a future release must leapfrog the crate version to ≥ `0.65.0`.
- **ZT-Infra local provisioner required at Phase 93**: `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2` — `npm install && npm start` in the provisioner directory. Confirm availability before Phase 93 planning.

## Deferred Items

Items acknowledged and deferred at v3.1 close (2026-06-21) — see `.planning/milestones/v3.1-MILESTONE-AUDIT.md`:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| PARTIAL→CI | SEC-01 AF_UNIX seccomp trap (Phase 87) — Unix-cfg-gated, GH Actions decisive | Open (CI-decisive) | v3.1 close |
| PARTIAL→CI | SEC-02 procfs-remap dedup guard (Phase 87) — Unix-cfg-gated | Open (CI-decisive) | v3.1 close |
| PARTIAL→CI | Cross-target clippy (linux-gnu + apple-darwin) — host lacks cross C compiler | Open (CI-decisive) | v3.1 close |
| Host-gated | DRAIN-01 live clean-VM silent MSI install — SKIP_HOST_UNAVAILABLE by design | Open (host-gated) | v3.1 close |
| Host-gated | DRAIN-02 live dual-layer WFP egress block — SKIP by design | Open (host-gated) | v3.1 close |
| Host-gated | DRAIN-03 live SIEM telemetry emit + admin opt-out — gate FAIL is environmental | Open (host-gated) | v3.1 close |
| Out-of-scope | 2 env-sensitive DACL-guard tests (Phase 74 code) fail at real-ACL on this host | Open (env-specific) | v3.1 close |
| Historical | 48 open artifacts (historical quick-tasks + dormant seeds + todos) | Acknowledged | v3.1 close |

## Session Continuity

Last session: 2026-06-22T01:52:58.315Z
Stopped at: Phase 92 executed (4/4 plans) + verified PASS-WITH-PARTIALS; next is /gsd-plan-phase 93
Resume file: .planning/phases/92-runtime-capabilityset-mutation-audit-wiring/92-VERIFICATION.md
