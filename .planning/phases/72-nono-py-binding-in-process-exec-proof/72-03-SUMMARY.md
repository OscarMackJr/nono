---
phase: 72-nono-py-binding-in-process-exec-proof
plan: 03
subsystem: infra
tags: [docs, design-doc, engine-abstraction, contract, e1-e5, zt-infra, nono-py, nono-ts]

# Dependency graph
requires:
  - phase: 72-01
    provides: "Born-confined soundness PASS verdict on Win11 (proof that E1-E5 invariants are real)"
provides:
  - "proj/DESIGN-engine-abstraction.md: canonical E1-E5 engine-abstraction contract (nono repo, proj/)"
  - "docs/engine-abstraction.md: discoverable pointer in nono-py repo (44-broker-ffi-lockstep)"
  - "E5 -> zt-infra.org POST /actions forward-compat mapping documented (D-10)"
  - "Windows Shape A/B + Linux Landlock + macOS Seatbelt implementation notes"
affects:
  - 72-04-langchain-proof
  - 75-nono-ts-parity

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Design doc home: first-class DESIGN-*.md docs live in proj/ alongside DESIGN-library.md (D-09)"
    - "Link-file pattern: nono-py/docs/ contains pointer-only docs referencing canonical nono/proj/ sources"
    - "proj/ is gitignored for planning scratch; use git add -f for first-class tracked design docs"

key-files:
  created:
    - proj/DESIGN-engine-abstraction.md
    - .planning/phases/72-nono-py-binding-in-process-exec-proof/72-03-SUMMARY.md
  modified: []

key-decisions:
  - "proj/ gitignored for planning scratch — force-added first-class design doc via git add -f (same pattern as docs/cli/development/)"
  - "E5 zt-infra mapping documented as FUTURE phase only; no HTTP client/adapter built (D-10 honored)"
  - "Sandbox::apply not in Windows Shape B section — Windows section opens with advisory-only note at top, then Shape A/B subsections are broker-only"

patterns-established:
  - "E1-E5 invariants: coverage gate fail-secure, absolute paths only, user-owned workspace (R-B3), NONO_ALREADY_CONFINED guard for Shape B"
  - "zt-infra E5 slot: deny => skip exec fail-closed; nono enforces OS confinement underneath the control-plane decision"

requirements-completed: [ABI-02]

# Metrics
duration: ~20min
completed: 2026-06-14
---

# Phase 72 Plan 03: E1-E5 Engine-Abstraction Contract Doc Summary

**E1-E5 engine-abstraction contract authored as proj/DESIGN-engine-abstraction.md (canonical, nono repo) with zt-infra.org E5 forward-compat mapping, Windows Shape A/B + Linux/macOS platform notes, and a link file in nono-py/docs/.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-06-14
- **Completed:** 2026-06-14
- **Tasks:** 2
- **Files modified:** 2 (1 per repo)

## Accomplishments

- `proj/DESIGN-engine-abstraction.md` created in nono repo: full E1-E5 contract with per-point
  invariants, the zt-infra.org E5 forward-compat mapping (D-10), Windows Shape A/B implementation
  notes (NONO_ALREADY_CONFINED guard, ordering invariant, AppContainer gotcha, CLR env baseline),
  Linux Landlock and macOS Seatbelt notes, and contract versioning section (v1.0).
- `docs/engine-abstraction.md` created in nono-py repo: pointer document with E1-E5 summary
  table, reference to `DESIGN-engine-abstraction.md`, zt-infra mention, and nono-ts note.
- ABI-02 satisfied: the E1-E5 boundary is documented as a stable, versioned contract that
  nono-ts (Phase 75) and the future daemon (Phase 74) can reference.

## Task Commits

1. **Task 1: Create proj/DESIGN-engine-abstraction.md** — `40522453` (docs — nono repo)
2. **Task 2: Create docs/engine-abstraction.md in nono-py** — `736149f` (docs — nono-py repo)

**Plan metadata:** (this docs commit — nono repo)

## Files Created/Modified

**nono repo (`C:\Users\OMack\Nono`):**
- `proj/DESIGN-engine-abstraction.md` — NEW; canonical E1-E5 contract; 277 lines

**nono-py repo (`C:\Users\OMack\nono-py`, branch `44-broker-ffi-lockstep`):**
- `docs/engine-abstraction.md` — NEW; 31-line pointer to canonical doc

**nono repo planning:**
- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-03-SUMMARY.md` — This file

## Decisions Made

**proj/ gitignore deviation:** `proj/` is gitignored in `.gitignore` line 15 (planning scratch
convention). The plan and D-09 explicitly require `proj/DESIGN-engine-abstraction.md` as a
first-class tracked design doc alongside future `DESIGN-library.md`. Applied `git add -f` to
override the gitignore (same pattern as `docs/cli/development/` — `feedback_docs_cli_dev_gitignored`
memory). The `.gitignore` entry is not modified; the doc is simply force-added.

**E5 zt-infra mapping is FUTURE only:** D-10 says document the mapping but do NOT build any HTTP
client or adapter. The forward-compat section in the canonical doc states this explicitly: "The
integration itself is a FUTURE phase." No code was written.

**Sandbox::apply note placement:** The Windows section opens with a paragraph explaining that
`Sandbox::apply()` is preview/advisory-only on Windows (this is the invariant from D-01/D-02 that
implementers need to know). The Windows Shape B subsection itself contains no `Sandbox::apply`
reference — it describes only the born-confined re-exec via `nono.exe`. The plan's verification
check ("doc does NOT contain Sandbox::apply in the Windows Shape B section") passes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] proj/ directory gitignored**
- **Found during:** Task 1 (git add attempt)
- **Issue:** `.gitignore` line 15 ignores `proj/` for planning-scratch convention; `git add` exited 1
- **Fix:** Used `git add -f proj/DESIGN-engine-abstraction.md` to force-add the file; `.gitignore` not modified (the ignore entry is for scratch use; force-adding one first-class design doc is the correct resolution per the `docs/cli/development/` precedent)
- **Files modified:** none (git plumbing only)
- **Commit:** `40522453`

---

**Total deviations:** 1 auto-fixed (1 blocking — gitignore)
**Impact on plan:** Minimal; the correct fix (force-add) is well-established in this project.

## Known Stubs

None. The canonical doc is complete prose with no placeholder sections, TODO markers, or
hardcoded empty content. The E5 zt-infra section is intentionally documentation-only (not a stub
— the integration is a future phase by design).

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes. This plan creates
only Markdown documentation files. T-72-03-SC confirmed: no package installs.

## Self-Check

- `proj/DESIGN-engine-abstraction.md` exists: FOUND (committed `40522453`)
- `grep -c "E5" proj/DESIGN-engine-abstraction.md` = 17 (>= 3): PASS
- `grep -c "zt-infra" proj/DESIGN-engine-abstraction.md` = 13 (>= 2): PASS
- `grep -c "DESIGN-engine-abstraction" ../nono-py/docs/engine-abstraction.md` = 3 (>= 1): PASS
- Sandbox::apply absent from Windows Shape B section: PASS
- nono-py commit `736149f` in repo `44-broker-ffi-lockstep`: VERIFIED

## Self-Check: PASSED

## Next Phase Readiness

72-04 (LangChain proof + UAT) is unblocked. The contract doc is available for reference. Both
72-02 (binding) and 72-03 (contract doc) are complete — Wave 2 is fully done.

---
*Phase: 72-nono-py-binding-in-process-exec-proof*
*Completed: 2026-06-14*
