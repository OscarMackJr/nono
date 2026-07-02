---
phase: 98-upst11-divergence-audit
plan: 04
subsystem: audit
tags: [divergence-audit, carve-out, upstream-sync, adr]

requires:
  - phase: 98-upst11-divergence-audit
    provides: "Full per-commit DIVERGENCE-LEDGER (Plans 01-03) + ADR-98 (Plan 03)"

provides:
  - "Expanded carve-out re-touch check: six carve-outs with explicit git-log verdicts and guard tests"
  - "ADR-98 cross-reference wired throughout ledger (Headline, Cluster A, endpoint-policy carve-out)"
  - "Completeness verification: all five sweep assertions PASS"
  - "98-DIVERGENCE-LEDGER.md finalized — Phase 99 gate document"

affects: [phase-99-upst11-absorb, phase-100-release-reconcile]

tech-stack:
  added: []
  patterns:
    - "Carve-out re-touch check pattern: git log window -- path + explicit verdict + guard test + Phase N guidance"
    - "Completeness sweep pattern: SHA accounting + noise reconcile + TBD audit + verdict audit + floor cross-ref"

key-files:
  created: []
  modified:
    - .planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md

key-decisions:
  - "Phase 95 endpoint-policy carve-out cross-references ADR-98 (Accepted — full-sync-adopt) explicitly — highest re-touch exposure this window"
  - "linux.rs carve-out HIT is additive-only (no fork-expression conflict); guard tests proxy_no_v4_seccomp/proxy_v4_no_seccomp named for Phase 99 verify"
  - "Cluster A disposition updated from needs-decision to full-sync-adopt across all ledger sections (Headline, summary table, cluster body, ADR Review routing)"
  - "Completeness sweep: 14 substantive + 6 noise = 20 total; no bare TBD; six explicit verdicts; 0.66.1 leapfrog floor anchored"

patterns-established:
  - "ADR cross-reference pattern: cite standalone ADR file in Headline + cluster body + relevant carve-out subsection; state recommendation in one line; do not inline the analysis"
  - "Carve-out guard-test naming: every HIT names the specific test(s) that verify fork invariants after apply"

requirements-completed: [UPST11-01]

duration: 30min
completed: 2026-06-30
---

# Phase 98 Plan 04: Carve-out Re-touch Check + ADR-98 Cross-Reference Summary

**Six carve-out surfaces re-touch-checked with explicit git-log verdicts, guard tests, and Phase 99 guidance; ADR-98 cross-reference wired throughout the ledger; completeness sweep confirms all five assertions PASS.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-06-30
- **Completed:** 2026-06-30
- **Tasks:** 2 of 2
- **Files modified:** 1

## Accomplishments

### Task 1: Expanded carve-out re-touch check (six explicit verdicts)

The `## Carve-out Re-touch Check` section in `98-DIVERGENCE-LEDGER.md` already contained the six git-log results from Plans 01-03. Plan 04 expanded it to satisfy all D-07/D-08 requirements:

**CR-02** (`crates/nono/src/audit.rs`): clean — no re-touch in window.
Guard test: `verify_empty_log_with_no_stored_metadata_is_not_valid`. Phase 99: no conflict work needed.

**CR-01** (`bindings/c/src/` FFI entry points): clean — no re-touch in window.
Guard test: `diagnostic_code_is_cleared_between_calls`. Phase 99: no conflict work needed.

**Cluster F proxy fork-preserve surface** (5 paths): HIT (5 commits — Clusters A, C, E) — expected conflict — preserve fork expression. Guard tests named (Phase 89 proxy suite). Phase 99: Cluster F carve-out review on all three Cluster C commits; ADR-98 adoption reconciles `proxy_runtime.rs` overlap.

**Phase 95 endpoint-policy wiring** (`CompiledEndpointPolicy`/`endpoint_policy.evaluate()`): HIT (3 commits — cdeeb5b9, c808f000, 08ca19a8) — expected conflict — preserve fork expression. **ADR-98 cross-reference added** (this is the #1225-restructured surface, highest re-touch exposure per D-07). Guard tests: `denied_endpoint_returns_403_and_audit`, `allow_domain_endpoint_route_does_not_shadow_credential_route` (Phase 89). Phase 99: split extraction of cdeeb5b9; CompiledEndpointPolicy compat check required before applying 46bcfbb9 endpoint wiring.

**Phase 95 restored fork invariants** (`sandbox/linux.rs` AF_UNIX/seccomp/cgroup): HIT (1 commit — 5b8e94da, Cluster D) — additive only — no fork-expression conflict. Guard tests added: `proxy_no_v4_seccomp` / `proxy_v4_no_seccomp` (Phase 95/96). Phase 99: apply as will-sync with cross-target clippy gate; guard tests confirm Landlock/seccomp invariants intact.

**v3.2 override surface** (`PolicyOverrideApplied` / EventIDs 10006-10010): clean — no re-touch in window. Phase 99: no conflict work needed.

### Task 2: ADR-98 cross-reference + completeness sweep

**ADR-98 wired throughout ledger:**
- Headline: "settled" cluster A (ADR-98 Accepted — full-sync-adopt, 2026-06-30; one-line recommendation added to Fork-specific deliverables)
- Cluster Summary table: Cluster A updated from `needs-decision (ADR-98)` to `full-sync-adopt (ADR-98 Accepted)`
- Cluster A body: disposition updated from needs-decision to full-sync-adopt with one-line recommendation
- ADR Review downstream routing: Cluster A changed from `PENDING` to `Phase 99 full-sync-adopt`
- Endpoint-policy carve-out: explicit ADR-98 cross-reference with one-line recommendation

**Completeness verification (all five PASS):**
- (a) 14 SHAs each classified exactly once across 8 clusters — PASS
- (b) 14 substantive + 6 noise = 20 total; matches `git log --oneline 1d1c88c9..d817ed53 | wc -l` = 20 — PASS
- (c) No bare TBD; all 8 clusters have explicit risk-cell values — PASS
- (d) Six carve-out verdicts present; no silent entries — PASS
- (e) Cluster H carries 0.66.1 leapfrog floor cross-ref for Phase 100 — PASS

## Deviations from Plan

None — plan executed exactly as written. The carve-out section from Plans 01-03 was already structurally complete; Plan 04 added the missing pieces: ADR-98 cross-reference in the endpoint-policy subsection, explicit guard tests for two HIT carve-outs, settled status for ADR-98 throughout the ledger, and the completeness verification closing note.

## Self-Check

**Files exist:**
- `.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` — modified (Tasks 1 + 2)
- `.planning/phases/98-upst11-divergence-audit/98-04-SUMMARY.md` — this file

**Commits exist:**
- `a7633193` — docs(98-04): expanded carve-out re-touch check — six explicit verdicts
- `ae6468d0` — docs(98-04): wire ADR-98 cross-ref + completeness sweep

## Self-Check: PASSED
