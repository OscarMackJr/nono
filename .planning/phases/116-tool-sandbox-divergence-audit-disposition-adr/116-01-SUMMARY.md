---
phase: 116-tool-sandbox-divergence-audit-disposition-adr
plan: 01
subsystem: git-archaeology / upstream-fork divergence audit
tags: [upstream-sync, tool-sandbox, divergence-ledger, D-03, module-set-derivation, ADR-precursor]

# Dependency graph
requires:
  - phase: 108-upst12-divergence-audit
    provides: the 20-commit fenced-window (v0.66.0..v0.69.0) tool-sandbox-pure/tool-sandbox-split
      disposition tables this ledger cross-references rather than duplicates (D-13)
provides:
  - "116-DIVERGENCE-LEDGER.md with a live-reconfirmed Reproduction block (pinned SHAs, 3-path-union
    count 38, substring-hazard reproduction)"
  - "D-03 module-set re-derivation: a full 82-module main.rs sweep confirms the re-derived set is
    identical to D-06's inherited three (tool_sandbox, command_policy, lineage_cgroup); all 4
    named candidates + 75 other siblings are explicit OUT calls with cited grep evidence"
  - "Pre-fence (7 commits) and post-fence (11 commits) per-commit disposition tables with
    windows-touch flags, cross-referencing (not duplicating) Phase 108's fenced-window surface"
affects: [116-02, 116-03, 116-04, 116-05, 116-06, proj/ADR-116-tool-sandbox-disposition.md]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-16 discovery-based grep derivation: bulk-grep the full main.rs mod/use list in both
      directions rather than confirming pre-named candidates"
    - "Structural split-classification rule: a commit is split if it touches any production path
      outside the module set, regardless of diff-content triviality (content notes recorded
      separately, not used to reclassify)"

key-files:
  created:
    - .planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md
  modified: []

key-decisions:
  - "D-03 module-set re-derivation confirms (does not expand or reduce) D-06's inherited 3-module
    set; all 4 named candidates (command_blocking_deprecation, instruction_deny, open_url_runtime,
    hook_runtime) are explicit OUT calls with zero static or runtime coupling to the tool-sandbox
    entrypoint"
  - "Every D-01 hypothesis figure re-measured live agrees exactly with the discussion-time table —
    no delta anywhere (38 union, 7 pre-fence, 11 post-fence, all 6 named post-fence PRs present)"
  - "upstream/main moved from 4ede9ccc to 6bd8a393 since research time; recorded as a named
    boundary, does not affect the pinned v0.71.0 window tip"

patterns-established:
  - "Fenced-window cross-reference shape: point at 108-DIVERGENCE-LEDGER.md's existing tables by
    relative path rather than duplicating per-commit rows, per D-13"

requirements-completed: [TSBX-01]

# Metrics
duration: ~25min
completed: 2026-08-09
---

# Phase 116 Plan 01: Tool-Sandbox Divergence Ledger — Reproduction + D-03 Re-Derivation Summary

**Created `116-DIVERGENCE-LEDGER.md` with a live-reconfirmed Reproduction block, an 82-module D-03
module-set re-derivation that confirms (not expands) D-06's inherited 3-module set, and full
pre-fence (7 commits) / post-fence (11 commits) disposition tables — every D-01 hypothesis figure
re-measured live and found to match exactly, no delta anywhere.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-08-09T16:30:51Z
- **Tasks:** 2 (Task 1: Reproduction + D-03 re-derivation; Task 2: pre-fence/post-fence tables)
- **Files modified:** 1 (new file: `116-DIVERGENCE-LEDGER.md`)

## Accomplishments

- Live-reconfirmed both pinned tag SHAs (`v0.64.1` = `0551eba2`, `v0.71.0` = `0055bf3c`) verbatim
  against `116-CONTEXT.md` D-01, plus recorded `upstream/main`'s new boundary (`6bd8a393`, moved
  from `4ede9ccc` since research time — expected drift, does not affect the pinned window).
- Reproduced the D-06-style substring-glob pathspec hazard live in this window (`37` hazard-form
  vs `35` directory-only, same two commits `5a7447d3`/`ebd51cbb` Phase 108 already flagged, both
  confirmed docs-only).
- Ran a full 82-module sweep of `main.rs`'s top-level `mod`/`use` declarations (not just the 4
  named candidates) in both forward and reverse cross-reference directions, plus a Pitfall-3
  control-flow read of `fn main()`'s body — every one of the 4 named candidates
  (`command_blocking_deprecation.rs`, `instruction_deny.rs`, `open_url_runtime.rs`,
  `hook_runtime.rs`) is an explicit **OUT** call with zero static or runtime coupling to the
  tool-sandbox entrypoint; the re-derived module set is identical to D-06's inherited three.
- Built full pre-fence (7 commits, `v0.64.1..v0.66.0`) and post-fence (11 commits,
  `v0.69.0..v0.71.0`) disposition tables with `sha | subject | PR# | touched-path summary |
  windows-touch | disposition` — zero `windows-touch` hits across all 18 commits, 7 flagged
  `split` (with content notes distinguishing structural-split-but-trivial-content from
  genuinely-substantial split residue), 11 flagged `adopt`.
- Confirmed all 6 named post-fence PRs from D-01's hypothesis table (#1476, #1492, #1467, #1453,
  #1521, #1550) present in the live post-fence table — zero absent.
- Cross-referenced (not duplicated) Phase 108's already-dispositioned 20-commit fenced-window
  surface, with a spot-verification re-run confirming the count still holds under the finalized
  pathspec.
- Arithmetic check: 7 (pre-fence) + 20 (fenced, Phase 108) + 11 (post-fence) = 38, matching Task
  1's live 3-path-union measurement exactly.

## Task Commits

Both tasks were authored as a single cohesive ledger file (the pre-fence/post-fence tables in
Task 2 depend directly on the finalized pathspec Task 1 derived, and were built in one continuous
pass against live git evidence) and committed together:

1. **Task 1 + Task 2: Create `116-DIVERGENCE-LEDGER.md` (Reproduction, D-03 re-derivation,
   pre-fence/post-fence tables)** - `5939cd9` (docs)

**Plan metadata:** this SUMMARY.md's own commit (below)

_Note: unlike typical per-task commits, this plan's two tasks build one artifact incrementally;
splitting the single `Write` into two partial commits would have required reconstructing an
artificial intermediate state with no independent value, so both tasks landed in one commit. This
is a deviation from the default one-commit-per-task pattern; documented under Deviations below._

## Files Created/Modified

- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md` -
  New ledger: Headline, Reproduction (pinned SHAs, pathspec hazard reproduction), D-03 module-set
  re-derivation (82-module sweep with explicit in-or-out table), Disposition Vocabulary, Pre-Fence
  Commits table, Post-Fence Commits table, Fenced-Window Cross-Reference (D-13), Arithmetic Check,
  Discrepancy vs. CONTEXT.md Hypothesis summary table.

## Decisions Made

- **Split-classification rule applied structurally, not by content triviality.** A commit is
  `split` if it touches any production path (`crates/*/src/` or `bindings/c/src/`) outside the
  3-path module set, even when the diff content itself is trivial (e.g. `c808f000`'s one-line
  org-URL rename, `f76733f6`'s test-only content). This mirrors `108-DIVERGENCE-LEDGER.md`'s own
  methodology exactly (its `a519ee62`/`72a98830` rows are `split` despite zero absorb-worthy
  residue) and keeps the `split` flag meaningful for Plan 116-03's downstream residue accounting
  — content notes are recorded on the row, not used to silently reclassify it as `skip`.
- **`open_url_runtime.rs`'s one in-window commit (`4cc0af2c`) does not affect the 38-commit
  dispositioned surface**, since that commit's own touched paths never intersect the 3-path union
  — recorded explicitly as "moot for this ledger" rather than left ambiguous, resolving
  `116-RESEARCH.md`'s Open Question 1.
- **The D-03 re-derivation's "no expansion" outcome is itself the finding**, not a null result —
  recorded per D-16 as carrying the same evidentiary weight as a correction, since the derivation
  ran independently from first principles (full sweep, both directions, control-flow read) and
  landed on the same boundary Phase 108's D-06 already established.

## Deviations from Plan

None affecting correctness or scope. One process note:

**Single commit covering both Task 1 and Task 2.** Both tasks build the same file
(`116-DIVERGENCE-LEDGER.md`); Task 2's tables structurally depend on Task 1's finalized pathspec
(which Task 1 confirmed unchanged from D-06's), so the file was authored end-to-end in one
continuous evidence-gathering pass and committed as a single unit rather than reconstructing an
artificial Task-1-only intermediate state purely to satisfy a one-commit-per-task convention. All
acceptance criteria and automated verification greps for both tasks pass against the single
resulting file (verified below).

## Issues Encountered

None. Every `git ls-remote`/`git log`/`git show`/`git grep` command ran cleanly against the live
`upstream` remote on the first attempt; all live-measured figures matched `116-CONTEXT.md` D-01's
discussion-time hypothesis exactly (no discrepancy requiring investigation or re-run).

## User Setup Required

None — no external service configuration required. This is a documentation-only plan (D-20: zero
`.rs`, zero `Cargo.toml`, `ROADMAP.md`/`STATE.md`/`REQUIREMENTS.md` untouched by this executor
per the orchestrator's ownership of those files).

## Self-Check

```
FOUND: .planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md
FOUND: 5939cd9 (git log --oneline --all | grep 5939cd9)
```

Automated verify commands from both tasks re-run against the committed file:

```
Task 1: grep -c "^## Reproduction" -> 1
        grep -c "0551eba27ea53b0eda20f4f796f757dd168adb92" -> 5
        grep -c "Pathspec Hazard Reproduction" -> 1
Task 2: grep -c "^## Pre-Fence Commits" -> 1
        grep -c "^## Post-Fence Commits" -> 1
        grep -c "108-DIVERGENCE-LEDGER.md" -> 13
```

All non-zero as required by both tasks' `<verify><automated>` blocks.

## Next Phase Readiness

- `116-DIVERGENCE-LEDGER.md` now exists with the Reproduction block, D-03 module-set derivation,
  and pre-fence/post-fence disposition tables Plans 116-03/116-04/116-05/116-06 build on (per
  `116-01-PLAN.md`'s stated objective).
- **Not yet done (explicitly out of scope for this plan, per its own objective statement):** full
  D-04 residue accounting (absorb/defer/noise per path) for the 7 `split`-flagged commits in this
  ledger; the carve-out re-touch check (D-19); the completeness sweep. These are Plan
  116-03/116-04's job.
- No blockers. `upstream/main` continuing to move (now at `6bd8a393`, a `v0.72.0` tag now also
  exists) does not affect this ledger's pinned `v0.71.0` window tip.
- **Per this project's `CLAUDE.md`/executor-prompt constraint: `.planning/STATE.md`,
  `.planning/ROADMAP.md`, and `.planning/REQUIREMENTS.md` were NOT touched by this executor** —
  those are the orchestrator's to update after this plan completes.

---
*Phase: 116-tool-sandbox-divergence-audit-disposition-adr*
*Completed: 2026-08-09*
