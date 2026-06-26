---
phase: 95-upstream-absorb-fork-invariant-verify
plan: "04"
subsystem: infra
tags: [upstream-sync, fork-invariant, verification, divergence-ledger, security, windows, adr-86, adr-87]

# Dependency graph
requires:
  - phase: 95-01
    provides: "Cluster A cherry-pick (ae77d198): AF_UNIX mediation deadlock fixed"
  - phase: 95-02
    provides: "Cluster B shared-surface absorb (91d526e6): SandboxRuntimeAuditEvent, CompiledEndpointPolicy, etc."
  - phase: 95-03
    provides: "Cluster C structural no-op (62dbf013): Phase 89 fail-secure preserved"
provides:
  - "95-VERIFICATION.md: fork-invariant checklist (3 invariants PASS, SC4 AF_UNIX security note, PARTIAL->96 record)"
  - "94-DIVERGENCE-LEDGER.md: Clusters A/B/C marked absorbed; no open will-sync rows"
  - "Phase 95 completion commit e19f7b59 with DCO sign-off"
affects:
  - Phase 96 (cross-target clippy verification — ae77d198, 61689ef8, 91d526e6 carry PARTIAL->96)
  - Phase 97 (leapfrog floor >= 0.65.0; Cluster D still won't-sync)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fork-invariant verification: each invariant verified by actual git diff commands against phase-base SHA, not HEAD~N"
    - "PARTIAL->96 record links specific commits (with file-level detail) to Phase 96 cross-target clippy gates"

key-files:
  created:
    - ".planning/phases/95-upstream-absorb-fork-invariant-verify/95-VERIFICATION.md"
  modified:
    - ".planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md"

key-decisions:
  - "Use explicit baseline SHA (449138a9) for git diff instead of HEAD~N — docs commits are interspersed between absorb commits making HEAD~N unreliable"
  - "cargo test -p nono-cli --lib fails (no lib target in nono-cli); used cargo test -p nono-cli instead for proxy guard test"
  - "make ci equivalent run as 3 separate commands (clippy + fmt-check + cargo test); 1 pre-existing D-04 failure in try_set_mandatory_label not a regression"

requirements-completed:
  - UPST10-02
  - UPST10-03

# Metrics
duration: 30min
completed: 2026-06-26
---

# Phase 95 Plan 04: Fork-Invariant Verification + DIVERGENCE-LEDGER Closeout Summary

**3 fork-invariant checklist entries all PASS (exec_strategy_windows/ byte-unchanged, ADR-86 audit boundary preserved, ADR-87 CR-02 guard test green); 94-DIVERGENCE-LEDGER.md Clusters A/B/C marked absorbed; PARTIAL->96 deferral record produced for Phase 96 cross-target clippy**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-06-26T05:00:00Z
- **Completed:** 2026-06-26T05:30:00Z
- **Tasks:** 2
- **Files modified:** 2 (95-VERIFICATION.md created, 94-DIVERGENCE-LEDGER.md updated)

## Accomplishments

- Ran all 8 invariant verification commands against actual git history (not from memory) and recorded actual outputs in 95-VERIFICATION.md
- All 3 invariants PASS: exec_strategy_windows/ byte-unchanged, ADR-86 boundary carve-out intact, ADR-86 D-03 denial-rendering fork intact
- CR-02 guard test `verify_empty_log_with_no_stored_metadata_is_not_valid` PASS (1/1)
- Cluster F proxy guard test `proxy_activates_with_custom_credentials_only` PASS (1/1)
- SC4 security note: Cluster A AF_UNIX deadlock fix — Windows equivalents confirmed intact via empty diff
- PARTIAL->96 record: 3 absorb commits (ae77d198, 61689ef8, 91d526e6) carrying cfg-gated Unix code listed with file-level detail for Phase 96
- 94-DIVERGENCE-LEDGER.md: Cluster Summary table updated with phase-95-status column; Downstream routing block updated with ABSORBED status and commit SHAs
- Final make ci equivalent: clippy GREEN, fmt-check GREEN, test 1 pre-existing D-04 failure (not new), cargo audit 0 errors

## Task Commits

1. **Task 1+2 (combined): Fork-invariant verification + DIVERGENCE-LEDGER closeout** - `e19f7b59` (docs, DCO signed)

**Plan metadata:** (included in e19f7b59)

## Files Created/Modified

- `.planning/phases/95-upstream-absorb-fork-invariant-verify/95-VERIFICATION.md` - Fork-invariant checklist (3 invariants PASS), SC4 security note, PARTIAL->96 record, SC1-SC5 completion gate results
- `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` - Cluster Summary table: added phase-95-status column with absorbed annotations for A/B/C; Downstream routing block: updated ABSORBED status with commit SHAs and DCO refs

## Decisions Made

- **Baseline SHA over HEAD~N**: The plan specified `git diff HEAD~3` but the phase has 7 commits above baseline (4 absorb + 3 docs). Used explicit baseline SHA `449138a9` for all diff commands to avoid ambiguity from interspersed docs commits.
- **nono-cli test invocation**: The plan specified `cargo test -p nono-cli --lib` for the proxy guard test, but nono-cli has no lib target (documented pre-existing issue). Used `cargo test -p nono-cli -- proxy_activates_with_custom_credentials_only` which correctly finds the test in main.rs. Result: PASS.
- **make ci as 3 commands**: `make` is not in the Bash tool's PATH on Windows. Ran clippy + fmt-check + cargo test + cargo audit individually. All GREEN (1 pre-existing D-04 failure is not a regression).

## Deviations from Plan

None — plan executed as written. The two command adjustments (baseline SHA vs HEAD~N, lib vs binary test runner) are documentation-only adaptations for correctness, not scope changes.

## Phase 95 SC1–SC5 Gate Results

| Gate | Check | Result |
|------|-------|--------|
| SC1 | All will-sync commits present (9ce74e92 via ae77d198; 11fd10e0 in 91d526e6 body; 9b37dc52 in 62dbf013 body) | PASS |
| SC2 | No new test failures vs D-04 baseline (5 pre-existing; 0 new) | PASS |
| SC3 | Fork-invariant checklist 3/3 PASS | PASS |
| SC4 | AF_UNIX security note present; exec_strategy_windows/ diff empty | PASS |
| SC5 | No open will-sync rows in ledger (A/B/C all marked absorbed) | PASS |

## PARTIAL Deferrals

**Cross-target clippy DEFERRED to Phase 96 per D-03:**

| Commit | Files | Phase 96 gate |
|--------|-------|---------------|
| ae77d198 (Cluster A: 9ce74e92) | sandbox/linux.rs, supervisor_linux.rs, exec_strategy.rs | cargo clippy --workspace --target x86_64-unknown-linux-gnu AND --target x86_64-apple-darwin |
| 61689ef8 (Cluster A post-fix) | Same files (let-chain rewrites, block structure fix) | Same |
| 91d526e6 (Cluster B) | sandbox/mod.rs (restrict_execute, cfg(linux) blocks) | Same |

Carry-forward PARTIAL->CI from prior phases (unresolved by Phase 95): SEC-01/SEC-02 AF_UNIX guards (v3.1/Phase 87), ZTL-04 AWS_* strip (v3.2/Phase 92-93).

## Phase 95 Completion Commit

**SHA:** e19f7b59
**Signed-off-by:** Oscar Mack Jr <oscar.mack.jr@gmail.com> (via git commit -s)
**Phase 95 full commit sequence:**
- 449138a9 chore: D-04 baseline capture (phase-base)
- ae77d198 fix: Cluster A cherry-pick (9ce74e92)
- 61689ef8 fix: Cluster A post-fix
- 91d526e6 feat: Cluster B absorb
- 62dbf013 refactor: Cluster C structural no-op
- e19f7b59 docs: fork-invariant verification + DIVERGENCE-LEDGER closeout (THIS COMMIT)
(plus interspersed docs commits: 251c1481, 438fc8f9, a6576138)

## Requirements Updated

- UPST10-02: PARTIAL (absorbed Clusters A/B/C with DCO; cross-target clippy PARTIAL->96)
- UPST10-03: PASS (fork-invariants verified via actual commands; PARTIAL for cross-target-touching commits -> Phase 96)

## Issues Encountered

None.

## Next Phase Readiness

Phase 96 (cross-target clippy) is ready to proceed against this HEAD:
- Verify ae77d198, 61689ef8, 91d526e6 against x86_64-unknown-linux-gnu and x86_64-apple-darwin
- Resolve SEC-01/SEC-02 (v3.1) and ZTL-04 (v3.2) PARTIAL->CI carry-forwards
- Phase 97 (leapfrog floor >= 0.65.0 + release prepare) follows after Phase 96

## Threat Flags

None — this plan modifies only planning documents. No new production code, no new trust boundaries, no new network endpoints, no schema changes.

## Self-Check: PASSED

- `e19f7b59` exists in git log: CONFIRMED (`git log --oneline -1` shows e19f7b59)
- `.planning/phases/95-upstream-absorb-fork-invariant-verify/95-VERIFICATION.md`: FOUND (committed in e19f7b59)
- `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` contains "absorbed": CONFIRMED (Cluster A/B/C rows)
- Invariants all PASS: CONFIRMED (actual command outputs in 95-VERIFICATION.md)
- DCO sign-off `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`: CONFIRMED (git commit -s flag)
- cargo test -p nono --lib verify_empty_log_with_no_stored_metadata_is_not_valid: PASS (1/1)
- cargo test -p nono-cli proxy_activates_with_custom_credentials_only: PASS (1/1)

---
*Phase: 95-upstream-absorb-fork-invariant-verify*
*Completed: 2026-06-26*
