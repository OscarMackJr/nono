---
phase: 102-fork-owned-package-rename
plan: 03
subsystem: infra
tags: [cargo, pyproject, maturin, package-rename, sibling-repo, PUB-01]

# Dependency graph
requires:
  - phase: 102-fork-owned-package-rename plan 01
    provides: renamed [package] name on the 3-crate publish set (nono/nono-proxy/nono-cli -> nono-sandbox/nono-sandbox-proxy/nono-sandbox-cli) in this workspace
provides:
  - "../nono-py/Cargo.toml nono/nono-proxy path-dependencies carry package = \"nono-sandbox\" / package = \"nono-sandbox-proxy\" (dependency table keys unchanged)"
  - "../nono-py/pyproject.toml [project] name renamed to nono-sandbox (PyPI distribution identity only)"
  - "Green maturin build in ../nono-py under the new names; cargo tree confirms the resolved dependency graph points at the renamed core crates via the correct relative path"
affects: [102-05-phase-gate, 105-live-publish]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Sibling-repo package= propagation: a repo outside this Cargo workspace that path-deps into it by relative path must have its OWN Cargo.toml dependency entries patched with package= keys after an upstream rename -- this repo's own cargo build --workspace never surfaces that breakage (per 102-RESEARCH.md Pitfall 2)"
    - "Gitignored Cargo.lock in a maturin-built PyPI binding still needs a force-add (git add -f) when a rename/repin must be captured in history for reproducibility of the resolved dependency graph"

key-files:
  created: []
  modified:
    - ../nono-py/Cargo.toml
    - ../nono-py/pyproject.toml
    - ../nono-py/Cargo.lock

key-decisions:
  - "Combined Task 1 (manifest edits) and Task 2 (build-green proof + commit) into a single DCO-signed nono-py commit, per the plan's own Task 2 acceptance criteria which require Cargo.toml + pyproject.toml + Cargo.lock all present in the SAME commit -- no intermediate Task-1-only commit was made in the nono-py repo."
  - "Rule 3 auto-fix: force-added the gitignored Cargo.lock (`git add -f`) since the plan's files_modified/acceptance_criteria explicitly require it committed to lock resolution against the renamed core crates; this is a normal Rust-library gitignore pattern that the plan deliberately overrides for this rename commit."

patterns-established: []

requirements-completed: [PUB-01]

# Metrics
duration: 6min
completed: 2026-07-03
---

# Phase 102 Plan 03: nono-py Sibling-Repo Package Rename Summary

**Patched `../nono-py`'s own Cargo.toml path-dependencies with `package = "nono-sandbox"` / `package = "nono-sandbox-proxy"` and renamed `pyproject.toml`'s `[project] name` to `nono-sandbox`, then proved `maturin build` green and committed all three files (Cargo.toml, pyproject.toml, regenerated Cargo.lock) as one DCO-signed commit in the nono-py repo.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-03T13:57:xxZ (approx, session start)
- **Completed:** 2026-07-03T14:03:43Z
- **Tasks:** 2
- **Files modified:** 3 (all in `../nono-py`: Cargo.toml, pyproject.toml, Cargo.lock)

## Accomplishments
- `../nono-py/Cargo.toml`: added `package = "nono-sandbox"` to the `nono` path-dependency and `package = "nono-sandbox-proxy"` to the `nono-proxy` path-dependency; dependency table keys (`nono`, `nono-proxy`) left unchanged; `../nono-py`'s own `[package] name = "nono-py"` untouched (not in the 3-crate publish set)
- `../nono-py/pyproject.toml`: `[project] name` renamed from `nono-py` to `nono-sandbox`; `[tool.maturin] module-name = "nono_py._nono_py"` and the `python/nono_py/` import surface left untouched
- `maturin build` run twice from `../nono-py`: first run compiled `nono-sandbox v0.66.1` and `nono-sandbox-proxy v0.66.1` from the renamed core-crate paths and produced `nono_sandbox-0.66.1-cp312-cp312-win_amd64.whl`; second run confirmed exit code 0 with an up-to-date incremental build
- `cargo tree -p nono-py | grep nono-sandbox` in `../nono-py` returned 3 matches (`nono-sandbox`, `nono-sandbox-proxy`, and the transitive `nono-sandbox` under `nono-sandbox-proxy`), confirming the resolved dependency graph points at the renamed crates via the expected `../Nono/crates/...` relative path, not a stray registry hit (T-102-02 mitigated)
- All three files (Cargo.toml, pyproject.toml, force-added Cargo.lock) committed together in `../nono-py` with a DCO sign-off, as a commit SEPARATE from this repo's own Plan 102-01/102-02 commit history

## Task Commits

Task execution in `../nono-py` (a separate git repository from this one):

1. **Task 1 + Task 2 combined: Patch Cargo.toml/pyproject.toml identity + prove maturin build green + DCO-signed commit** - `787e2dd` (fix), committed in `../nono-py` on its `44-broker-ffi-lockstep` branch

**This repo's plan-metadata commit:** (this SUMMARY.md + STATE.md + ROADMAP.md, committed separately below)

## Files Created/Modified
- `../nono-py/Cargo.toml` - `nono` dependency: `+ package = "nono-sandbox"`; `nono-proxy` dependency: `+ package = "nono-sandbox-proxy"`; keys unchanged
- `../nono-py/pyproject.toml` - `[project] name`: `nono-py` -> `nono-sandbox`
- `../nono-py/Cargo.lock` - regenerated via `maturin build`; force-added (gitignored by default in nono-py, per plan's explicit requirement)

## Decisions Made
- Deferred the nono-py commit until after `maturin build` succeeded, so Task 1's manifest edits and Task 2's build-proof landed in a single commit containing all three required files, matching the plan's Task 2 acceptance criteria (`git -C ../nono-py show --stat -1 includes Cargo.toml, pyproject.toml, and Cargo.lock`) rather than the generic per-task commit protocol's default of one commit per task.
- Force-added the gitignored `Cargo.lock` rather than treating the gitignore rule as a blocker, since the plan's own `files_modified` frontmatter and acceptance criteria explicitly name this file for the commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Force-added gitignored `../nono-py/Cargo.lock`**
- **Found during:** Task 2, staging step
- **Issue:** `../nono-py/.gitignore` excludes `Cargo.lock` (standard practice for a Rust library/binding crate). A plain `git add Cargo.lock` would silently no-op, leaving the regenerated lockfile untracked and violating the plan's explicit acceptance criterion that the commit include Cargo.lock.
- **Fix:** Used `git add -f Cargo.lock` to force-stage the regenerated lockfile alongside the normally-staged Cargo.toml/pyproject.toml, then committed all three together.
- **Files modified:** `../nono-py/Cargo.lock` (staging mechanism only; file content is the expected `maturin build` regeneration output)
- **Verification:** `git show --stat -1` in `../nono-py` confirms all three files (Cargo.toml, pyproject.toml, Cargo.lock) are present in the commit
- **Commit:** `787e2dd` (in `../nono-py`)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary to satisfy the plan's own explicit acceptance criterion that the DCO-signed commit contain all three named files. No scope creep — the fix is a staging-mechanism workaround for a pre-existing gitignore rule, not a change to any file's content or the rename's substance.

## Issues Encountered
None beyond the deviation documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `../nono-py`'s half of PUB-01 SC2 (PyPI project name renamed) and PUB-01 SC4 (maturin build green) is complete.
- Plan 102-04 (nono-ts rename) is independent and can proceed; Plan 102-05 (phase gate) should re-verify `maturin build` fresh in `../nono-py` alongside the nono-ts equivalent, per 102-RESEARCH.md's "three separate, sequential verification steps" guidance (Pitfall 2).
- No blockers. Note for Plan 102-05/105 authors: `../nono-py`'s current git branch is `44-broker-ffi-lockstep` (pre-existing, not created by this plan) — the rename commit landed on that branch, consistent with the prior commit (`84e8f18`) already present there.

---
*Phase: 102-fork-owned-package-rename*
*Completed: 2026-07-03*

## Self-Check: PASSED

- FOUND: `.planning/phases/102-fork-owned-package-rename/102-03-SUMMARY.md`
- FOUND: commit `787e2dd` (in `../nono-py`)
