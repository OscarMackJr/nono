---
phase: 108-upst12-divergence-audit
plan: 03
subsystem: infra
tags: [upstream-sync, divergence-audit, requirement-mapping, adr-108, proxy, profile, resource-limits]

requires:
  - phase: 108-01
    provides: "100-commit accounting, Cluster Summary skeleton (NET 12 / PROF 8 / CORE 4 / tool-sandbox-surface 20 / security-residual-and-misc 18)"
  - phase: 108-02
    provides: "ADR-108 (deny_domain #1374 disposition — Adapt, deny-layer-only, fail-closed via network_policy.rs)"
provides:
  - "NET cluster per-commit table (12 rows) with hand-verified NET-01/02/03 requirement mapping, windows-touch flags, and actual-diff re-export scans"
  - "PROF cluster per-commit table (10 rows: 8 full + 2 tool-sandbox pointer rows) with hand-verified PROF-01/02/03/04 requirement mapping"
  - "CORE cluster per-commit table (4 rows) with hand-verified CORE-01/02 requirement mapping"
  - "New cross-crate re-export finding (crates/nono/src/lib.rs pub mod resource) flagged for Phase 111 ADR-86 review"
affects: [109-net-absorb, 110-prof-absorb, 111-core-absorb-and-release]

tech-stack:
  added: []
  patterns: ["actual-diff (git show | grep) re-export scanning over --name-only", "hand-verification of requirement mapping via diff content, not commit-subject keyword matching"]

key-files:
  created: []
  modified:
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md

key-decisions:
  - "PROF per-commit table expanded from the plan's literal 5 named full-rows + 2 pointers (7 total) to 8 full-rows + 2 pointers (10 total), to cover all 8 commits Plan 108-01's own Cluster Summary already established as PROF-cluster members — omitting 3 already-confirmed commits (0374e454, 9ef5918169, f58c7c2242) from hand-verification would have silently violated the plan's own must-have truth."
  - "ae1c513e (#1371) and 719975cf (#1380) are windows-touch: yes on cited Os::Windows runtime-match-arm evidence, not a cfg(windows) compile gate — a materially different windows-relevance signal than every other row in this ledger, worth Phase 110 attention since it's exactly the field the fork's windows_low_il_broker/windows_interpreters migration will populate."
  - "e6d26871 (#1269 resource limiting) and 34c2c975 (#1403 --max-processes) mapped to CORE-02 with disposition adapt (not adopt) — CORE-02 explicitly requires reconciliation with the fork's existing Job Object mechanism, and upstream's diff carries zero Windows mentions at all, confirming Phase 111 authors the Windows side from scratch rather than adapting an upstream Windows path."
  - "e6d26871 adds a new cross-crate pub mod resource + pub use resource::ResourceLimits to crates/nono/src/lib.rs — the first CORE-cluster commit in this window to cross the library/CLI boundary; flagged as a Threat Flag for Phase 111 ADR-86 review rather than silently adopted."

patterns-established:
  - "Per-commit table discrepancy resolution: when a plan's literal task enumeration disagrees with an already-established Cluster Summary count, include the full established set and document the disagreement explicitly rather than truncating to the plan's literal list."

requirements-completed: [UPST12-01]

duration: 45min
completed: 2026-07-29
---

# Phase 108 Plan 03: NET/PROF/CORE Per-Commit Divergence Tables Summary

**Hand-verified per-commit requirement mapping (NET-01/02/03, PROF-01/02/03/04, CORE-01/02) and actual-diff re-export scans for 26 commits across the NET/PROF/CORE clusters, surfacing a new cross-crate `nono` core re-export and a zero-Windows-precedent gap in upstream's resource-limiting feature.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 2
- **Files modified:** 1 (`108-DIVERGENCE-LEDGER.md`, two Edit appends)

## Accomplishments

- NET cluster: 12/12 commits hand-verified against `NET-01`/`NET-02`/`NET-03`; `3b207eeb`
  (#1374, `deny_domain`) disposition cross-references ADR-108 instead of re-deriving the
  strict/deny-only posture analysis inline.
- PROF cluster: 8/8 Cluster-Summary-confirmed commits hand-verified (expanded from the plan's
  literal 5, see Deviations) plus 2 tool-sandbox-entangled pointer rows for `d5803b99`/`d4927f95`.
- CORE cluster: 4/4 commits hand-verified against `CORE-01`/`CORE-02`, with a finding that
  upstream's resource-limiting work has zero Windows precedent to adapt from.
- Discovered and flagged a new cross-crate `pub mod resource;` / `pub use resource::
  ResourceLimits;` re-export landing in `crates/nono/src/lib.rs` via `e6d26871` (#1269) — the
  first CORE-cluster commit to cross the library/CLI boundary rather than stay CLI-internal.
- Confirmed `f58c7c2242` (#1320, CLI `--extends` flag) does not conflict with the fork's
  pre-existing JSON-schema `extends` field (fork commit `91c3b1a0`, predates this window) — a
  different surface (CLI flag vs. profile-JSON field) that composes cleanly.
- All 26 SHAs across NET/PROF/CORE re-verified resolvable via `git cat-file -e <sha>^{commit}`
  before use.

## Task Commits

1. **Task 1: NET cluster per-commit table** - `fee56801` (docs)
2. **Task 2: PROF + CORE cluster per-commit tables** - `f6374106` (docs)

**Plan metadata:** this commit (docs: complete plan, includes SUMMARY.md)

## Files Created/Modified

- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` - Appended NET (12
  rows), PROF (10 rows: 8 full + 2 pointer), CORE (4 rows) per-commit tables, plus a Threat
  Flags section for the new cross-crate `nono` core re-export.

## Decisions Made

1. **PROF table row-count discrepancy resolved by inclusion, not truncation.** The plan's Task 2
   action text names exactly 5 PROF commits for full-row treatment plus 2 pointer rows (7 total,
   matching the acceptance criteria's literal "exactly 7 rows"). Plan 108-01's own Cluster
   Summary, however, already confirmed **8** PROF-cluster members. Rather than silently drop the
   3 unnamed commits (`0374e454`, `9ef5918169`, `f58c7c2242`) from hand-verification — which
   would contradict this plan's own must-have truth that *every* cluster commit gets a
   hand-verified requirement mapping — all 8 are included as full rows, with the discrepancy
   documented inline in the ledger. This mirrors the phase's established discipline (108-01
   caught and documented two prior hypothesis-vs-measurement disagreements the same way).
2. **`ae1c513e`/`719975cf` windows-touch determined via `Os::Windows` runtime-match evidence, not
   `cfg(windows)`.** Both commits add/thread a `windows: Option<Box<PlatformOverride>>` field and
   an `Os::Windows` match arm with zero compile-time cfg gates. Marked `windows-touch: yes` with
   the cited grep evidence per the plan's explicit requirement, distinct from every other row in
   this ledger (all of which are `windows-touch: no` via `cfg(target_os = "windows")` grep).
3. **CORE-02 rows (`e6d26871`, `34c2c975`) dispositioned `adapt`, not `adopt`.** CORE-02's
   wording explicitly requires reconciliation with the fork's existing Job Object mechanism.
   Grepping both diffs for any Windows mention returned zero hits — upstream's implementation is
   Linux-cgroup-only with no Windows counterpart at all, confirming Phase 111 must author the
   Windows reconciliation from scratch rather than adapt an upstream Windows path.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] PROF per-commit table expanded from 7 to 10 rows to cover all established cluster members**
- **Found during:** Task 2 (PROF + CORE cluster per-commit tables)
- **Issue:** The plan's Task 2 action text and acceptance criteria explicitly enumerate only 5
  PROF commits for full-row treatment (`ae1c513e`, `719975cf`, `2cbaa9a0`, `b620ed8e`,
  `f016b2d5`) plus 2 pointer rows, with acceptance criteria requiring "exactly 7 rows total."
  Plan 108-01's Cluster Summary, however, already established **8** confirmed PROF-cluster
  commits (the 5 named plus `0374e454`, `9ef5918169`, `f58c7c2242`). Following the plan literally
  would have left 3 already-confirmed cluster members with no hand-verified requirement mapping
  in the per-commit table — directly contradicting this plan's own must-have truth ("every
  NET/PROF/CORE cluster commit has a hand-verified requirement mapping").
- **Fix:** Included all 8 Cluster-Summary-confirmed PROF commits as full rows (all mapped: 5 to
  named PROF-0X requirements, 3 to `none` with rationale), plus the 2 required pointer rows —
  10 rows total. The discrepancy and its resolution are documented inline in the ledger's PROF
  section under "Row-count reconciliation note," not silently resolved.
- **Files modified:** `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`
- **Verification:** All 8 full-row SHAs cross-checked against Plan 108-01's PROF cluster commit
  list (lines 396-404 of the pre-existing ledger); all 10 rows have non-blank requirement-mapping
  and disposition cells.
- **Committed in:** `f6374106` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug: table would have silently under-covered an
already-established cluster).
**Impact on plan:** Necessary for correctness — the acceptance criteria's literal "7 rows" was
in tension with the plan's own must-have truth requiring complete cluster coverage; resolving in
favor of completeness (and documenting why) avoids Phase 110 silently missing 3 commits' review.
No scope creep — the 3 additional rows use the exact same columns/methodology as the 5 the plan
named.

## Issues Encountered

None beyond the PROF row-count discrepancy documented above (handled as an auto-fixed deviation,
not a blocker).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 109 (NET absorb) has a complete, hand-verified 12-row NET table with the `3b207eeb`
  ADR-108 cross-reference in place — ready to consume directly.
- Phase 110 (PROF absorb) has a complete 10-row PROF table (8 full + 2 pointer), including the
  `Os::Windows`-runtime-match evidence for `ae1c513e`/`719975cf` that PROF-01's
  `windows_low_il_broker`/`windows_interpreters` migration will need.
- Phase 111 (CORE absorb) has a complete 4-row CORE table and two open items to carry forward:
  (a) the new `crates/nono/src/lib.rs` cross-crate `resource` module re-export needs an ADR-86
  policy-free-library confirmation before absorption; (b) CORE-02's Windows Job Object
  reconciliation has zero upstream precedent to adapt from and must be authored fresh.
- No blockers for Plan 108-04 (tool-sandbox-surface split) or Plan 108-05 (roadmap amendment
  proposal for the security-residual-and-misc cluster).

---
*Phase: 108-upst12-divergence-audit*
*Completed: 2026-07-29*
