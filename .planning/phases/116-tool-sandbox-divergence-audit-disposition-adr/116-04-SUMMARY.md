---
phase: 116-tool-sandbox-divergence-audit-disposition-adr
plan: 04
subsystem: git-archaeology / upstream-fork divergence audit
tags: [upstream-sync, tool-sandbox, divergence-ledger, D-19, carve-out-retouch, bucket-reconciliation, ledger-closure]

# Dependency graph
requires:
  - phase: 116-01
    provides: "116-DIVERGENCE-LEDGER.md with the Reproduction block, D-03 module-set
      re-derivation, and pre-fence/post-fence per-commit disposition tables"
  - phase: 116-03
    provides: "116-DIVERGENCE-LEDGER.md's Split-Commit Residue Accounting (161 rows) and
      Fenced-Window Residue Reconciliation (31 grep-verified paths) sections this plan's
      Bucket-Count Reconciliation and Carve-out Re-touch Check build on"
  - phase: 108-upst12-divergence-audit
    provides: "108-DIVERGENCE-LEDGER.md, the fenced-window ledger every carve-out HIT and
      every fenced-window commit is cross-referenced against rather than re-analyzed"
provides:
  - "116-DIVERGENCE-LEDGER.md gains a `## Carve-out Re-touch Check (D-19)` section: all 7 Phase
    108 originals plus 3 new carve-out surfaces named in ADR-108/111/113/114 checked against the
    full v0.64.1..v0.71.0 window, every result an explicit clean/HIT with SHA-level routing"
  - "a 12-commit `Uncovered-Window Finding` (carve-out-file touches outside both this ledger's
    38-commit surface and 108's fenced window) -- a proposal only, not applied to
    ROADMAP.md/REQUIREMENTS.md"
  - "`## Bucket-Count Reconciliation`, `## Security-Relevant Rollup`, and
    `## Completeness Verification` sections closing out D-16/D-17/D-18/D-01"
  - "a finalized `## Headline` restating the D-01 no-delta reconciliation, the disposition tally,
    the windows-touch tally, and pointers to 108-DIVERGENCE-LEDGER.md/116-FEASIBILITY-MATRIX.md/
    proj/ADR-116-tool-sandbox-disposition.md -- closing with 'closed except for named exceptions'"
affects: [proj/ADR-116-tool-sandbox-disposition.md]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Carve-out re-touch check run against the FULL ledger window (not the 38-commit
      tool-sandbox-scoped surface) surfaces HITs from unrelated commits that also touch a
      carve-out file -- every HIT is routed to whichever existing document already classifies
      it (108-DIVERGENCE-LEDGER.md per-cluster tables, this ledger's own tables, or a specific
      phase-numbered ADR), with a residual set named honestly as a new finding when no existing
      document covers it (D-19's 'silence is never evidence' extended to 'unrouted HITs are
      never silently absorbed into an existing citation either')"

key-files:
  created: []
  modified:
    - .planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md

key-decisions:
  - "12 commits touching a carve-out surface fall outside BOTH this ledger's 38-commit
    tool-sandbox surface AND 108-DIVERGENCE-LEDGER.md's fenced window (v0.66.0..v0.69.0) --
    6 pre-fence (46bcfbb9, cdeeb5b9, 72bcfd66, 08ca19a8, 5b8e94da, 9ce74e92) and 6 post-fence
    (c5c7f56e, e9c17607, ebdbe4c9, b3b048bf, 062344a9, 46db3ffb). Named as an
    'Uncovered-Window Finding' -- neither prior audit's scope ever covered 'every commit
    touching a named carve-out file across the full v0.64.1..v0.71.0 range', so this is a
    genuine gap in existing audit coverage, not a defect in this ledger. Proposed as a future
    UPST13/FUT-08 audit item, not dispositioned or applied to ROADMAP.md/REQUIREMENTS.md by
    this phase (D-20 audit-only)."
  - "ADR-108 names no carve-out surface -- its Decision is Adapt (Option B), not a decline/
    permanent-boundary, so it is explicitly recorded as contributing zero new carve-out rows
    rather than silently omitted from the accumulated-surface sweep."
  - "ADR-111/113/114 each name a genuine new carve-out surface: ADR-111's rejected core-module
    absorb (crates/nono/src/resource/, upstream-only) plus the fork-native platform
    resource-enforcement files (supervisor_linux.rs/supervisor_macos.rs); ADR-113's OD-1 /
    ADR-114's OD section both point at the same TLS-interception boundary
    (crates/nono-proxy/src/forward.rs, tls_intercept/, upstream-only) -- consolidated into one
    carve-out row since both ADRs name an identical, not merely overlapping, surface."
  - "Bucket-Count Reconciliation re-ran git show --name-only --format='' <sha> | wc -l fresh
    this session for all 7 split commits rather than trusting Plan 116-03's own prior counts --
    every count reproduced exactly (31/91/12/8/8/9/2), confirming Sigma_bucketed=161 =
    Sigma_touched=161 by independent re-measurement, not inheritance (D-16)."
  - "Security-Relevant Rollup flags 13 of 18 pre-fence/post-fence commits (4/7 pre-fence, 8/11
    post-fence) via subject-line + diff-read scan for auth/credential/token/trust/permission/
    deny themes -- chosen as a standalone rollup substitute for a dedicated per-row column
    (Claude's Discretion, per 116-CONTEXT.md), since retrofitting a column across
    already-committed Plan 116-01 tables was avoided in favor of a queryable standalone section."

patterns-established:
  - "Finalized-Headline pattern: the pre-existing Headline (Plan 116-01) is edited in place to
    add a D-01 restatement, disposition/windows-touch tallies, sibling-document pointers, and a
    'closed except for named exceptions' closing phrase -- rather than duplicated as a second
    Headline-like section elsewhere in the file, so a Headline-only reader gets the complete
    picture without needing the rest of the document."

requirements-completed: [TSBX-01]

# Metrics
duration: ~70min
completed: 2026-08-09
---

# Phase 116 Plan 04: Divergence Ledger Closure — Carve-out Re-touch + Bucket Reconciliation Summary

**Closed `116-DIVERGENCE-LEDGER.md`: ran the D-19 carve-out re-touch check against every
accumulated fork-invariant surface (Phase 108's 7 originals plus 3 new surfaces named in
ADR-108/111/113/114) across the full `v0.64.1..v0.71.0` window, surfacing a 12-commit
Uncovered-Window Finding as a byproduct; reconciled the D-16 bucket-count arithmetic
(Sigma_bucketed=161=Sigma_touched, shown per commit, not asserted); added a Security-Relevant
Rollup and Completeness Verification; and finalized the Headline with the D-01 no-delta
restatement, disposition/windows-touch tallies, and sibling-document pointers, closing
"except for named exceptions."**

## Performance

- **Duration:** ~70 min
- **Completed:** 2026-08-09
- **Tasks:** 2 (Task 1: carve-out re-touch check; Task 2: bucket-count reconciliation +
  security-relevant rollup + completeness verification + finalized Headline)
- **Files modified:** 1 (`116-DIVERGENCE-LEDGER.md`, append + in-place Headline edit — no
  existing content from Plans 116-01/116-03 removed or renumbered)

## Accomplishments

- Read `proj/ADR-108-deny-domain-posture.md`, `proj/ADR-111-resource-limits-boundary.md`,
  `proj/ADR-113-spiffe-disposition.md`, and `proj/ADR-114-oauth-capture-disposition.md` in full
  and extracted each ADR's own named carve-out / permanent-boundary / rejected-absorb surface:
  ADR-108 names none (its Decision is Adapt, recorded explicitly as a non-finding); ADR-111
  names the rejected core-module absorb (`crates/nono/src/resource/`) plus fork-native
  platform-specific resource-enforcement files; ADR-113/114 both name the same
  TLS-interception boundary (`forward.rs`/`tls_intercept/`, upstream-only), consolidated into
  one carve-out row.
- Ran the carve-out re-touch check (`git log --no-merges --oneline $RANGE -- <exact-path>`) for
  all 10 accumulated surfaces against the full `v0.64.1..v0.71.0` window — every result is an
  explicit clean/HIT, never silence: 1 clean (`exec_strategy_windows/`, RESEARCH.md §2i's
  stronger "upstream has no Windows-specific exec-strategy directory at all" phrasing), 9 HIT
  results with every SHA named and routed to whichever existing document already classifies it.
- Discovered, as a direct byproduct of running the check against the full window rather than
  the 38-commit tool-sandbox surface, that 12 commits touch a carve-out file but fall outside
  both this ledger's own surface and `108-DIVERGENCE-LEDGER.md`'s fenced window (which only
  covers `v0.66.0..v0.69.0`) — named as an honest "Uncovered-Window Finding" rather than
  force-fit into an existing citation, and proposed (not applied) as a future UPST13/FUT-08
  audit item.
- Re-ran `git show --name-only --format='' <sha> | wc -l` fresh for all 7 pre-fence/post-fence
  split commits and confirmed the touched-path counts (31/91/12/8/8/9/2) reproduce Plan
  116-03's own figures exactly; built the `## Bucket-Count Reconciliation` section showing
  `Sigma_bucketed = 161`, `Sigma_touched = 161`, `161 == 161` with per-commit arithmetic shown.
- Built the `## Security-Relevant Rollup` (Claude's Discretion substitute for a dedicated
  column): scanned all 18 pre-fence/post-fence commit subjects (reading diffs/bodies for two
  ambiguous cases, `691e0f4f` and `7011bc85`) and flagged 13 as security-relevant
  (auth/credential/token/trust/permission/deny themes), with reasons cited per row.
- Built `## Completeness Verification`: confirmed `7 + 20 + 11 = 38` with actual numbers, and
  swept all 18 primary-row SHAs across the Pre-Fence/Post-Fence tables for uniqueness — 18
  unique, zero duplicates (pre-fence and post-fence date ranges do not overlap).
- Finalized the `## Headline` in place (edited Plan 116-01's original section, nothing
  duplicated elsewhere): added the explicit "measured 38, hypothesis 38, no delta" restatement,
  a disposition tally (20 adopt/pure, 0 adapt, 0 skip, 18 split = 38), a windows-touch tally (0
  of 38, cross-referencing `108-DIVERGENCE-LEDGER.md`'s own Plan 108-05 sweep for the fenced
  20), relative-path pointers to `108-DIVERGENCE-LEDGER.md`, `116-FEASIBILITY-MATRIX.md`, and
  `proj/ADR-116-tool-sandbox-disposition.md`, and closed with the literal phrase "closed except
  for named exceptions."

## Task Commits

Both tasks build the same file with a direct data dependency (Task 2's Completeness
Verification and finalized Headline reference Task 1's carve-out findings and the ledger's
existing tables), authored end-to-end in one continuous pass and committed together, matching
the exact deviation pattern Plans 116-01 and 116-03 both already documented for the same reason:

1. **Task 1 + Task 2: Append Carve-out Re-touch Check, Bucket-Count Reconciliation,
   Security-Relevant Rollup, Completeness Verification, and finalize the Headline in
   `116-DIVERGENCE-LEDGER.md`** - `9d1091ab` (docs)

**Plan metadata:** this SUMMARY.md's own commit (below)

_Note: unlike typical per-task commits, this plan's two tasks incrementally close one shared
ledger file with a direct data dependency; splitting into two partial commits would have
required reconstructing an artificial intermediate state with no independent review value. This
mirrors Plan 116-01's and Plan 116-03's own documented deviations. All acceptance-criteria
automated-verify greps for both tasks pass against the single resulting commit (verified below)._

## Files Created/Modified

- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md`
  - Modified: finalized the `## Headline` section in place (D-01 no-delta restatement,
    disposition/windows-touch tallies, sibling-document pointers, closing phrase).
  - Appended (all of Plans 116-01/116-03's existing sections preserved intact, nothing
    renumbered or removed): `## Carve-out Re-touch Check (D-19)` (2 sub-tables covering 10
    carve-out surfaces + Uncovered-Window Finding, 12 commits), `## Bucket-Count Reconciliation
    (D-16)`, `## Security-Relevant Rollup (D-17/D-18 discretion note)`, `## Completeness
    Verification (D-01)`, and an updated closing `Ledger status` line.

## Decisions Made

See `key-decisions` in frontmatter for the 5 load-bearing findings (the 12-commit
Uncovered-Window Finding, ADR-108's no-carve-out non-finding, the ADR-111/113/114 carve-out
extractions, the fresh bucket-count re-verification, and the Security-Relevant Rollup scope
choice).

## Deviations from Plan

None affecting correctness or scope. One process note (matching Plans 116-01's and 116-03's own
precedent):

**Single commit covering both Task 1 and Task 2.** Documented above under Task Commits — both
tasks incrementally close the same ledger file with a direct data dependency (Task 2's
Completeness Verification and Headline finalization directly reference Task 1's carve-out
findings), so they were authored and committed as one unit. All acceptance-criteria automated
verify greps for both tasks pass against the single resulting file (verified below).

**One CLAUDE.md-driven deviation (Rule 2 — auto-add missing critical functionality), not a
scope change:** the plan's acceptance criteria required every HIT row to name every SHA and
cross-reference it "to where it is already classified" — for the 12 Uncovered-Window commits,
no existing document classifies them at all. Rather than force-fit a misleading citation, this
was surfaced as an explicit new finding (per D-19's own "silence is never evidence" spirit
extended to "an unrouted HIT must not be silently mis-cited either"), consistent with the
project's fail-secure/explicit-over-implicit conventions.

## Issues Encountered

One correction made mid-task, recorded rather than silently fixed: the initial draft of the
`exec_strategy_windows/` clean-result cell used an em-dash ("clean — no re-touch in window")
instead of the plan's required literal ASCII double-hyphen phrase ("clean -- no re-touch in
window"), which caused the Task 1 automated verify grep (`grep -c "clean -- no re-touch in
window"`) to return 0 instead of 1. Caught by re-running the plan's own literal verify command
before committing (not by a later review pass) and fixed in place. No other issues — every
`git log`/`git show`/`ls` command in both tasks ran cleanly against the live working tree on
the first attempt.

## User Setup Required

None — no external service configuration required. This is a documentation-only plan (D-20:
zero `.rs`, zero `Cargo.toml`, `ROADMAP.md`/`REQUIREMENTS.md`/`116-FEASIBILITY-MATRIX.md`
untouched by this executor, confirmed via `git diff --stat` showing only
`116-DIVERGENCE-LEDGER.md` modified). Per this project's `CLAUDE.md`/executor-prompt
constraint, `.planning/STATE.md` and `.planning/ROADMAP.md` were NOT touched by this executor —
those are the orchestrator's to update after this plan completes.

## Self-Check

```
FOUND: .planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md
FOUND: 9d1091ab (git log --oneline --all | grep 9d1091ab)
```

Automated verify commands from both tasks re-run against the committed file:

```
Task 1: grep -c "^## Carve-out Re-touch Check" -> 1
        grep -c "clean -- no re-touch in window" -> 1
Task 2: grep -c "^## Bucket-Count Reconciliation" -> 1
        grep -c "^## Completeness Verification" -> 1
        grep -c "closed except for named exceptions" -> 2
```

Additional acceptance-criteria checks re-run:

```
Substring-glob check scoped to the Carve-out Re-touch Check section only -> 0 matches
  (the 2 whole-file matches are Plan 116-01's own pre-existing Pitfall-1 hazard-demonstration
  text, `'*tool-sandbox*'`, correctly retained as documented anti-pattern evidence, not part
  of this plan's carve-out table)
Headline section (lines 13-87) contains "no delta" -> 1
Headline section contains "108-DIVERGENCE-LEDGER.md" -> 5
Headline section contains "116-FEASIBILITY-MATRIX.md" -> 1
Headline section contains "ADR-116-tool-sandbox-disposition.md" -> 1
Headline section contains "closed except for named exceptions" -> 1
Security-Relevant Rollup contains "Claude's Discretion" -> 1
git diff --stat -- '*.rs' '*.toml' .planning/ROADMAP.md .planning/REQUIREMENTS.md
  .planning/STATE.md 116-FEASIBILITY-MATRIX.md -> empty (D-20 confirmed)
```

All non-zero/empty as required by both tasks' `<verify><automated>` blocks and this plan's own
success criteria.

## Next Phase Readiness

- `116-DIVERGENCE-LEDGER.md` is now fully closed per this plan's own success criteria: every
  accumulated carve-out surface has an explicit clean/HIT result against the full window; the
  bucket-count arithmetic is shown and reconciles; the Security-Relevant Rollup substitutes for
  the (not-taken) dedicated column with its discretion reasoning on the record; the
  Completeness Verification proves zero silent commit loss; and the finalized Headline restates
  the D-01 reconciliation, states the total counts, and points to
  `108-DIVERGENCE-LEDGER.md`, `116-FEASIBILITY-MATRIX.md`, and
  `proj/ADR-116-tool-sandbox-disposition.md`.
- Two named exceptions remain on the record, both proposals requiring operator approval before
  becoming a new phase or FUT item: the Uncovered-Window Finding (this plan, 12 commits) and
  the Post-Fence Residue Finding (Plan 116-03, 3 module-scoped refinement items). Neither
  blocks this ledger's own closure per D-19/D-04's "named exception, not silent gap" shape.
- `proj/ADR-116-tool-sandbox-disposition.md` does not yet exist — this ledger's evidence
  (D-01 through D-04 findings, the carve-out re-touch results, and the disposition/
  windows-touch tallies) is what the ADR (Plans 116-05/116-06, if the phase's remaining plans
  build it) will cite and build on. This plan does not create the ADR.
- No blockers. `upstream` was not re-fetched during this plan's execution (all `git log`/`git
  show` commands ran against the already-fetched pinned-SHA range from Plan 116-01, which
  remains valid and unchanged).

---
*Phase: 116-tool-sandbox-divergence-audit-disposition-adr*
*Completed: 2026-08-09*
