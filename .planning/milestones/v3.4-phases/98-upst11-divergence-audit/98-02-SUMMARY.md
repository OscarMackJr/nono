---
phase: 98-upst11-divergence-audit
plan: 02
subsystem: audit
tags: [divergence-audit, upstream-sync, ledger, upst11, networkintent, actual-diff, adr-review]

requires:
  - phase: 98-upst11-divergence-audit
    plan: 01
    provides: cluster scaffold (A-H), window SHAs, drift JSON, carve-out re-touch check, 6-check empirical cross-check

provides:
  - 98-DIVERGENCE-LEDGER.md fully populated: per-cluster bodies (A-H), per-commit tables, re-export scans, D-14 cross-target clippy notes, ADR Review risk matrix, 12-check Empirical Cross-Check, finalized Headline with downstream routing
  - Uniform actual-diff (git show) for all 14 substantive commits (D-09 closed)
  - D-03 re-confirmation: every 260629-toe per-PR hypothesis confirmed at commit granularity
  - D-14 Phase 99 cross-target clippy notes: Cluster A (72bcfd66 + d457ecc3) and Cluster D (5b8e94da)
  - D-15 leapfrog floor recorded: 0.66.1 in Cluster H won't-sync cross-ref
  - Cluster E disposition updated: verify → will-sync (confirmed by D-03 actual-diff: mechanical nolabs-ai URL update)

affects: [98-upst11-divergence-audit-plan-03, 99-upst11-absorb]

tech-stack:
  added: []
  patterns:
    - "D-09 uniform actual-diff: git show on every substantive commit (no risk-tiering); eliminates risk of additive-looking commit hiding boundary/FFI/security change"
    - "D-03 re-confirmation: per-PR hypothesis → per-commit disposition; changes: E verify→will-sync"
    - "Phase 94 per-cluster body format: Commits/Disposition/Windows-touch/Inspection-depth/Rationale/Re-export/Per-commit-table"

key-files:
  created: []
  modified:
    - .planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md

key-decisions:
  - "#1225 NetworkIntent refactor confirmed CLI-side ONLY by actual-diff: all 11 touched files in crates/nono-cli/src/; capability.rs, manifest_convert.rs, sandbox/linux.rs untouched; conflict is fork CLI vs upstream CLI, not library-boundary issue"
  - "Cluster E disposition updated from verify to will-sync: c808f000 actual-diff confirms mechanical GitHub URL string updates (always-further → nolabs-ai); package namespace identifiers (always-further/claude etc.) correctly preserved; per-file review in Phase 99 for fork identity URLs"
  - "D-14 cross-target clippy notes recorded for Cluster A (72bcfd66 + d457ecc3 — both add cfg(linux/macos) blocks) and Cluster D (5b8e94da — linux.rs has cfg(target_os=linux) blocks)"
  - "Cluster C re-export check: cdeeb5b9 adds pub mod pool to nono-proxy/src/lib.rs (new connection-pooling module, intra-proxy, won't-apply path if tls_intercept/ hunks skipped)"
  - "ADR Review per-cluster risk matrix: Cluster A=H (ADR-98 blocking), B=L (won't-sync), C=H (large split), D=L (additive), E=L (mechanical), F=M (security dep), G=L (accuracy), H=L (won't-sync)"
  - "Outcome: continue — no escalation threshold reached; ADR-98 (Plan 03) is sole blocking gate before Phase 99 absorb"

patterns-established:
  - "12-check Empirical Cross-Check: Plan 01 mandatory carve-out checks (1-6) + Plan 02 mandatory #1225-surface checks (7-12); zero hits on capability.rs/manifest_convert.rs/network_policy.rs confirms CLI-side scope"

requirements-completed: [UPST11-01]

duration: 45min
completed: 2026-06-30
---

# Phase 98 Plan 02: UPST11 Divergence Audit — Cluster Classification Summary

**14 substantive commits uniformly inspected by actual-diff (git show); 8 clusters fully populated with per-commit tables, re-export scans, and ADR risk matrix; ADR-98 (Plan 03) confirmed as sole blocking gate before Phase 99 absorb — all other clusters have determinate Phase 99 dispositions**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-30
- **Completed:** 2026-06-30
- **Tasks:** 2 (both target same file; executed and committed together)
- **Files modified:** 1 (98-DIVERGENCE-LEDGER.md, +595/-30 lines)

## Accomplishments

- Ran `git show --stat` and `git show` on all 14 substantive commits (D-09 uniform actual-diff; no risk-tiering)
- Added per-cluster bodies for Clusters A through H following the Phase 94 template: Commits / Disposition / Windows-touch / Inspection depth / Rationale / cross-cluster re-export check / per-commit table
- Ran cross-cluster re-export scan (`git show <sha> | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'`) for all 14 commits
- D-03 re-confirmed each 260629-toe per-PR disposition at commit granularity; updated Cluster E from "verify" → "will-sync"
- D-14 Phase 99 cross-target clippy notes added for Cluster A (both commits add cfg(linux/macos) blocks) and Cluster D (linux.rs is cfg(linux)-gated)
- Extended Empirical Cross-Check from 6 to 12 checks; added mandatory #1225-surface files (capability.rs, manifest_convert.rs, network_policy.rs — all zero hits; confirms #1225 CLI-side)
- Added full ADR Review: 5-dimension overall table + per-cluster risk matrix (all 8 clusters scored across security/windows/maintenance/divergence/contributor + Overall); no TBD cells
- Finalized Headline: substantive count, per-cluster dispositions, windows-touch cluster callout, #1225 ADR-98 cross-ref, downstream routing list, Outcome paragraph

## Task Commits

Both tasks target the same ledger file and were executed in a single write pass (same pattern as Plan 01):

1. **Task 1 + Task 2: Per-cluster bodies, re-export scans, ADR Review, Empirical Cross-Check** - `84e2f860` (docs)

**Plan metadata:** (this commit, created below)

## Files Created/Modified

- `.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` — Full cluster bodies (A-H), per-commit tables, re-export scans, D-14 notes, finalized Headline, ADR Review (5-dim overall + per-cluster matrix), Empirical Cross-Check (12 checks); total ~930 lines

## Decisions Made

- Tasks 1 and 2 executed together (same target file; one atomic commit)
- #1225 scope confirmed CLI-side ONLY by actual-diff: `crates/nono/src/capability.rs` has zero hits in the window; all 11 touched files are in `crates/nono-cli/src/`
- Cluster E disposition: "verify" → "will-sync" (D-03 re-confirmation; actual-diff of c808f000 shows mechanical GitHub URL string replacements; package namespace identifiers `always-further/claude` etc. explicitly preserved by upstream author; per-file review in Phase 99 for OscarMackJr/nono fork identity)
- Cluster A disposition unchanged: "needs-decision (ADR-98)" — both 72bcfd66 and d457ecc3 gate on ADR-98 since d457ecc3 references NetworkIntent type
- `pub mod pool` in cdeeb5b9 is intra-proxy (nono-proxy lib.rs); no cross-library violation; won't-apply if tls_intercept/ hunks skipped (pool.rs IS in shared surface, separately applicable)

## Deviations from Plan

None — plan executed as specified. The two tasks were combined into a single write pass (same pattern as Plan 01; both tasks target the same file and the content was produced in one atomic operation). All acceptance criteria for both tasks verified via automated checks before commit.

## Issues Encountered

None. All 14 `git show` invocations succeeded. Re-export scans clean (no cross-library violations). All automated verification checks passed on first run.

## User Setup Required

None — this is a docs-only audit plan. No external service configuration required.

## Next Phase Readiness

- **Plan 03 (NetworkIntent ADR-98):** Ready to execute. Inputs confirmed: actual-diff of 72bcfd66 = CLI-side only; 11 files in nono-cli/src/; `capability.rs`/`manifest_convert.rs`/`sandbox/linux.rs` untouched; windows-touch=yes on 7 files; d457ecc3 pairs with #1225; fork touchpoints of NetworkMode::ProxyOnly confirmed (capability.rs ~1045/1065/843/1386/2711+, manifest_convert.rs:47, sandbox/linux.rs:386/620)
- **Plan 04 (carve-out re-touch):** All 6 surface checks completed in Plan 01; Plan 04 does deeper inspect on hits (5 Cluster F commits, 1 linux.rs commit)
- **Phase 99 (absorb):** Gated on ADR-98 for Cluster A; all other clusters (C, D, E, F, G) have determinate split/will-sync dispositions with clear Phase 99 guidance recorded in cluster bodies

**Blockers/Concerns carrying into Plan 03:**
- Cluster A (NetworkIntent) ADR-98: must settle adopt vs fork-diverge before Phase 99 begins; windows-touch=yes means cross-target clippy is mandatory on adoption
- Cluster C (#983 HTTP/2): large split; pool.rs + shared surfaces extractable; tls_intercept/ won't-apply — this is well-understood but labor-intensive in Phase 99
- Cluster F (5 proxy carve-out hits): all confirmed in carve-out check; Phase 89 guard tests must pass; CompiledEndpointPolicy compat check required for 46bcfbb9

## Self-Check: PASSED

```
Task commit:        84e2f860 → FOUND (git rev-parse --short HEAD)
won't-sync:         grep -q "won't-sync" → PASS
0.66.1 floor:       grep -Eq 'floor[^0-9]*0\.66\.1' → PASS
re-export:          grep -qi 're-export' → PASS
windows-touch:      grep -qi 'windows-touch' → PASS
## ADR Review:      grep -q '## ADR Review' → PASS
## Empirical:       grep -q '## Empirical Cross-Check' → PASS
## Headline:        grep -q '## Headline' → PASS
capability.rs:      grep -q 'capability.rs' → PASS
No TBD cells:       grep -nE '\| *TBD *\|' → PASS (no matches)
No deletions:       git diff --diff-filter=D HEAD~1 HEAD → CLEAN
```

---
*Phase: 98-upst11-divergence-audit*
*Completed: 2026-06-30*
