---
phase: 116-tool-sandbox-divergence-audit-disposition-adr
plan: 05
subsystem: docs/ADR
tags: [tool-sandbox, windows, adr, disposition, non-bias]
dependency-graph:
  requires: ["116-01", "116-02", "116-04"]
  provides: ["proj/ADR-116-tool-sandbox-disposition.md (through D-09 scoring, Status: Proposed)"]
  affects: ["116-06 (appends the verdict/Decision/triggers/Phase 120 sizing to the same file)"]
tech-stack:
  added: []
  patterns: ["symmetric two-pole scoring table (7 criteria x 2 poles, no weighting)", "considered-and-rejected intermediate shapes (D-06, ADR-113 OD-1 precedent)"]
key-files:
  created:
    - proj/ADR-116-tool-sandbox-disposition.md
  modified: []
decisions:
  - "Header uses '**Status: Proposed**' (bold-wrapping the whole phrase) rather than the house '**Status:** Proposed' split-bold style, because the plan's acceptance grep requires the literal substring 'Status: Proposed' -- the split-bold form (as used in ADR-111/113/114) does not satisfy that grep due to the '**' sitting between the colon and the value."
  - "command_policies docs-only claim re-verified live (2026-08-09) per D-16, not inherited from Phase 108 D-06: zero hits in crates/nono-cli/src/, six hits in crates/nono-cli/data/profile-authoring-guide.md."
metrics:
  duration: "~45 minutes"
  completed: 2026-08-09
---

# Phase 116 Plan 05: ADR-116 Header, Context, Two-Pole Framing, D-06, D-09 Scoring Summary

Wrote the first half of `proj/ADR-116-tool-sandbox-disposition.md` -- header (`Status: Proposed`),
Context citing the closed divergence ledger's finalized stats, symmetric two-pole framing (Pole A:
adopt + build `platform/windows.rs`; Pole B: formalize the fork-native PreToolUse-hook path), the
two considered-and-rejected intermediate shapes (D-06), and a 7-criteria x 2-pole D-09 scoring
table with zero weighting and zero comparative language -- deliberately stopping short of any
verdict, Decision section, or Phase 120 sizing, which is Plan 116-06's job.

## What Was Built

**Task 1 (commit `9531327a`):** ADR header block (`Status: Proposed`, with an HTML comment noting
Plan 116-06 flips it to `Accepted` per D-11), a `## Context` section citing
`116-DIVERGENCE-LEDGER.md`'s finalized `## Headline` figures verbatim (38 total dispositioned
commits: 7 pre-fence + 20 fenced-cross-referenced + 11 post-fence; `windows-touch: 0 of 38`; the
12-file/18,433-LOC subsystem inventory with `platform/linux.rs` + `platform/macos.rs` = 13,627 LOC
= 74% of the subsystem, and no `platform/windows.rs` upstream), followed by the literal neutral
framing sentence the plan requires verbatim. Two equal-depth `## Pole A`/`## Pole B` sections
follow, each one paragraph, citing the fork/upstream primitives each builds on with no comparative
sentence. `## D-06: Considered and Rejected Intermediate Shapes` records both named intermediates
(Adopt Unix-Only; Adopt the Policy Model Only) as considered-and-rejected, with the
`command_policies` docs-only claim re-verified live via `grep -rn "command_policies"
crates/nono-cli/src/ crates/nono-cli/data/` (recorded inline: zero hits in `src/`, six hits in
`crates/nono-cli/data/profile-authoring-guide.md`).

**Task 2 (commit `3f58a9b0`):** Appended `## D-09: Symmetric Scoring`, a 7-row table
(`Criterion | Pole A | Pole B`, no weighting/score/total column) covering, in the locked order:
per-command granularity, engine-agnosticism, enforcement depth, fail-direction under layer
failure, platform coverage, ADR-86 library-vs-CLI boundary impact, and ongoing divergence +
maintenance cost. The engine-agnosticism row applies the "primitive vs. entry point" distinction
verbatim, citing `claude_code_hook.rs` and `nono-tool-hook.ps1` by name. The enforcement-depth row
states the fork's kernel-enforced claim (AppContainer + WFP + Job Object, citing
`exec_strategy_windows/` symbols) as its strongest claim without softening, and states Pole A's
Windows enforcement as "unbuilt"/"unproven" without disqualifying language. A
`## Neutrality Self-Check` closes the plan, confirming "7 criteria x 2 poles = 14 populated cells"
and the absence of comparative/recommending language document-wide.

## Verification

All plan-mandated greps were re-run against the final file after both tasks:

```
Status: Proposed        -> 1
^## Pole (count)         -> 2
^## D-06 (count)          -> 1
^## D-09 (count)          -> 1
^## Decision (count)      -> 0
primitive vs (count)      -> 2
comparative-language scan -> 0
D-09 table rows            -> 7
```

One live defect was caught and fixed mid-execution: the house-style header format
`**Status:** Proposed` (used by ADR-111/113/114) does **not** satisfy the plan's literal
`grep -c "Status: Proposed"` acceptance check, because the bold markers sit between the colon and
the value (`Status:** Proposed`, not `Status: Proposed`). Verified this is true of the existing
ADRs too (`grep -c "Status: Accepted" proj/ADR-113-spiffe-disposition.md` also returns 0). Switched
to `**Status: Proposed**` (bold-wrapping the whole phrase) to satisfy the explicit, checkable
acceptance criterion while staying visually equivalent. A second defect: the Neutrality Self-Check
paragraph originally quoted the negative-grep's own regex alternatives verbatim
(`` `is better`, `preferable`, ... ``), which self-matched the plan's own comparative-language scan.
Rewrote that sentence to describe the scan's target abstractly instead of spelling out the trigger
strings, so the self-check no longer trips its own check.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Header format did not satisfy the plan's literal `Status: Proposed` grep**
- **Found during:** Task 1, immediately after first draft
- **Issue:** House-style `**Status:** Proposed` places `**` between the colon and the value,
  so the substring `Status: Proposed` never appears literally.
- **Fix:** Changed to `**Status: Proposed**`.
- **Files modified:** `proj/ADR-116-tool-sandbox-disposition.md`
- **Commit:** `9531327a`

**2. [Rule 1 - Bug] Neutrality Self-Check self-matched its own negative-grep scan**
- **Found during:** Task 2, post-write verification
- **Issue:** The self-check sentence quoted the plan's comparative-language regex patterns
  verbatim as literal text, causing `grep -icE "is better|preferable|..."` to return 1 (a
  false-positive match against the self-check's own description of the scan, not an actual
  comparative claim).
- **Fix:** Rewrote the sentence to describe the scan target abstractly instead of quoting the
  regex alternation.
- **Files modified:** `proj/ADR-116-tool-sandbox-disposition.md`
- **Commit:** `3f58a9b0`

**3. [Rule 3 - Blocking] `proj/` is gitignored; ADR files are force-tracked**
- **Found during:** Task 1, first commit attempt
- **Issue:** `git status --short` showed nothing after creating the file, because `.gitignore`
  excludes `proj/` wholesale; existing ADRs (ADR-111, ADR-113, ADR-114, etc.) are tracked only via
  `git add -f`, matching the known `docs/cli/development/` pattern from user memory
  (`feedback_docs_cli_dev_gitignored.md`).
- **Fix:** Used `git add -f proj/ADR-116-tool-sandbox-disposition.md` for both commits.
- **Files modified:** none (git plumbing only)
- **Commit:** `9531327a`, `3f58a9b0`

No other deviations. Plan executed as written otherwise.

## Known Stubs

None. The document is intentionally incomplete per its own scope (no verdict, Decision, triggers,
Consequences, or Phase 120 sizing) -- this is the plan's stated boundary with Plan 116-06, not a
stub.

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes were
introduced -- this plan is documentation-only (D-20: zero `.rs`/`Cargo.toml` edits).

## STATE.md / ROADMAP.md / REQUIREMENTS.md

**Not touched, per this project's critical rule.** The orchestrator owns those writes for this
project (nono has two parallel milestones open; SDK state/roadmap writers have previously
destroyed hand-maintained tracking blocks). Requirement `TSBX-02` is not marked complete by this
plan -- ADR-116 remains `Status: Proposed` until Plan 116-06 lands the verdict.

## Self-Check

```
$ [ -f "proj/ADR-116-tool-sandbox-disposition.md" ] && echo FOUND || echo MISSING
FOUND
$ git log --oneline --all | grep -q "9531327a" && echo FOUND || echo MISSING
FOUND
$ git log --oneline --all | grep -q "3f58a9b0" && echo FOUND || echo MISSING
FOUND
```

## Self-Check: PASSED
