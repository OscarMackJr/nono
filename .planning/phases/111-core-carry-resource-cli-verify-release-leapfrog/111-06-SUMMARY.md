---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
plan: 06
subsystem: release-engineering
tags: [cargo, maturin, napi, pypi, npm, crates.io, semver-leapfrog, dry-run]

# Dependency graph
requires:
  - phase: 111-05
    provides: "All 6 in-repo workspace crates + path-dep pins leapfrogged to 0.70.0, Cargo.lock zero third-party drift, release-readiness.ps1 corrected to assert 0.70.0/0.69.0"
provides:
  - "../nono-py bumped to 0.70.0 (Cargo.toml, pyproject.toml, Cargo.lock), maturin build clean"
  - "../nono-ts bumped to 0.70.0 (Cargo.toml incl. corrected nono path-dep version requirement, package.json incl. all 5 optionalDependencies pins, 4 npm/*/package.json platform packages), napi build clean"
  - "scripts/release-dry-run.ps1 PRE_PUBLISH_REGISTRY_BLOCKED detection fixed to recognize post-rename cargo error phrasing"
  - "RLS-14 fully satisfied and marked Complete"
affects: [112-security-residual, release-engineering]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Sibling binding repos each get their own DCO-signed commit in their own git history (D-08)"]

key-files:
  created: []
  modified:
    - "../nono-py/Cargo.toml"
    - "../nono-py/pyproject.toml"
    - "../nono-py/Cargo.lock"
    - "../nono-ts/Cargo.toml"
    - "../nono-ts/package.json"
    - "../nono-ts/npm/darwin-arm64/package.json"
    - "../nono-ts/npm/darwin-x64/package.json"
    - "../nono-ts/npm/linux-arm64-gnu/package.json"
    - "../nono-ts/npm/linux-x64-gnu/package.json"
    - "../nono-ts/Cargo.lock"
    - "scripts/release-dry-run.ps1"
    - ".planning/REQUIREMENTS.md"

key-decisions:
  - "Widened release-dry-run.ps1's PRE_PUBLISH_REGISTRY_BLOCKED regex to also match cargo's 'no matching package named' phrasing, not just 'failed to select a version for the requirement'"
  - "Included ../nono-ts's Cargo.lock in the Task 2 commit even though the plan's <files> tag didn't name it, for the same regenerated-lockfile-must-be-committed reason Task 1 explicitly required for ../nono-py"
  - "requirements.mark-complete RLS-14 applied by hand-editing REQUIREMENTS.md rather than invoking the SDK verb, given the documented SDK auto-flip caution from 111-05"

patterns-established: []

requirements-completed: [RLS-14]

# Metrics
duration: 30min
completed: 2026-08-04
---

# Phase 111 Plan 06: Sibling-Repo 0.70.0 Leapfrog + Prepare-Only Release Gate Summary

**Bumped `../nono-py` and `../nono-ts` to 0.70.0 (including nono-ts's live semver path-dep pin), rebuilt both bindings clean, then found and fixed a real regression in `release-dry-run.ps1`'s pre-publish-block detection before confirming the gate GREEN — closing out RLS-14 and Phase 111's release-leapfrog objective.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-08-04T22:15:00-04:00 (approx.)
- **Completed:** 2026-08-04T22:44:29-04:00
- **Tasks:** 3 completed
- **Files modified:** 9 sibling-repo files + 1 in-repo script + REQUIREMENTS.md

## Accomplishments
- `../nono-py` reports `0.70.0` in both `Cargo.toml` and `pyproject.toml`; `maturin build` exits 0 with no struct-drift fix required; `Cargo.lock` regenerated with drift scoped to only the two path-dep packages.
- `../nono-ts` reports `0.70.0` everywhere, including the load-bearing `nono` path-dependency version requirement corrected from `"0.66"` to `"0.70"` (a live semver constraint that would otherwise fail to resolve against this repo's now-`0.70.0` `nono-sandbox` package); all 5 `optionalDependencies` platform pins and the 4 existing `npm/*/package.json` files bumped; `npx napi build --platform --release` exits 0 with no fix required.
- Discovered and fixed a real, previously-undetected regression in `scripts/release-dry-run.ps1`: the Phase 102/103 fork-owned rename to `nono-sandbox` means crates.io has never indexed that package name at all, so cargo now reports `no matching package named` instead of the `failed to select a version for the requirement` phrasing the script's regex was written against (pre-rename, when the crate was still named `nono` and upstream had already published *that* name). The regex had silently never been exercised against real post-rename output since the July 3 rename — this plan is the first live run of the full script since then. Widened the match to accept both phrasings.
- `pwsh -File scripts/release-dry-run.ps1` now exits 0 with the exact documented verdict: `PASS: Hard failures: 0. Blocked: 2 (pre-publish). Skipped: 1 (toolchain absent).`
- `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` returns `PASS` with `version_family: "0.70.0 confirmed for all 6 version-family crates"` and `leapfrog: "0.70.0 > 0.69.0 (leapfrog confirmed)"`.
- RLS-14 marked Complete — both the in-repo half (111-05) and this sibling-repo half are now satisfied together.
- D-07 upheld throughout: no tag created or pushed by this plan (the two pre-existing local `v0.7.0`/`v0.70.0` tags predate this session by months and are unrelated), no live `cargo publish`/`twine upload`/`npm publish` executed anywhere — every publish-adjacent command used `--dry-run` or was itself a dry-run script.

## Task Commits

Each task was committed atomically, each sibling repo in its own git history:

1. **Task 1: Bump `../nono-py` to 0.70.0, rebuild, commit** - `c24b20b` (chore, in `../nono-py` repo)
2. **Task 2: Bump `../nono-ts` to 0.70.0 (incl. live path-dep requirement), rebuild, commit** - `6406096` (chore, in `../nono-ts` repo)
3. **Task 3: Run the full prepare-only release gate and confirm GREEN** - `be3d0ecc` (fix, in this repo — the `release-dry-run.ps1` regex fix Task 3 required to reach GREEN)

**Plan metadata:** commit pending (docs: complete plan) — this SUMMARY + REQUIREMENTS.md + STATE.md + ROADMAP.md update

## Files Created/Modified
- `../nono-py/Cargo.toml` - `version = "0.66.1"` -> `"0.70.0"`
- `../nono-py/pyproject.toml` - `version = "0.66.1"` -> `"0.70.0"` (`name = "nono-sandbox"` unchanged)
- `../nono-py/Cargo.lock` - regenerated via `maturin build`, drift scoped to `nono-sandbox`/`nono-sandbox-proxy` path-dep versions only
- `../nono-ts/Cargo.toml` - `version = "0.66.1"` -> `"0.70.0"`; `nono` path-dep `version = "0.66"` -> `"0.70"` (load-bearing)
- `../nono-ts/package.json` - top-level `version` -> `"0.70.0"`; all 5 `optionalDependencies` entries -> `"0.70.0"`
- `../nono-ts/npm/darwin-arm64/package.json` - `version` -> `"0.70.0"`
- `../nono-ts/npm/darwin-x64/package.json` - `version` -> `"0.70.0"`
- `../nono-ts/npm/linux-arm64-gnu/package.json` - `version` -> `"0.70.0"`
- `../nono-ts/npm/linux-x64-gnu/package.json` - `version` -> `"0.70.0"`
- `../nono-ts/Cargo.lock` - regenerated via `napi build`, drift scoped to the version bump
- `scripts/release-dry-run.ps1` - widened `PRE_PUBLISH_REGISTRY_BLOCKED` detection to match both cargo error phrasings
- `.planning/REQUIREMENTS.md` - RLS-14 checkbox and traceability row flipped to Complete

## Decisions Made
- Widened `release-dry-run.ps1`'s regex rather than special-casing the fork-owned rename elsewhere, because the underlying condition (base crate not yet on the live registry) is identical regardless of which cargo error phrasing reports it — the fix preserves the script's own documented intent instead of routing around it.
- Included `../nono-ts/Cargo.lock` in the Task 2 commit despite the plan's `<files>` tag not naming it: `npx napi build --platform --release` regenerates it as a side effect (same as `maturin build` does for `../nono-py`, which the plan explicitly required committing), and leaving it uncommitted would leave the sibling repo's working tree dirty after this plan claims completion.
- Applied `requirements.mark-complete` for RLS-14 by hand-editing `.planning/REQUIREMENTS.md` rather than invoking the `gsd-sdk` verb, consistent with 111-05's documented caution that the SDK verb only reads a single plan's frontmatter and can misrepresent split-requirement completion state; here it was safe since this genuinely is the completing half, but the hand-edit avoids any risk of the verb re-touching adjacent lines unexpectedly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `release-dry-run.ps1`'s pre-publish-block detection regex**
- **Found during:** Task 3 (running the full prepare-only release gate)
- **Issue:** `pwsh -File scripts/release-dry-run.ps1` exited 1 (not the plan's expected GREEN exit 0). `cargo publish --dry-run -p nono-sandbox-proxy`/`-p nono-sandbox-cli` failed with cargo's `no matching package named `nono-sandbox` found` error, which the script's `PRE_PUBLISH_REGISTRY_BLOCKED` regex (`'failed to select a version for the requirement'`) did not match — so the script classified an expected pre-publish state as a hard `FAIL` and the whole gate reported `FAIL: 2 dry-run check(s) failed.` This is a real, previously-undetected regression: the regex was written before the Phase 102/103 fork-owned rename to `nono-sandbox`, when the crate was still named `nono` and upstream had already published *that* name (so cargo's error was "version not found", not "package not found"). No full run of this script against real registry state had happened since the rename (confirmed via `git log` — the last full run, `34adcb12`/100-04, predates the rename by a day; 111-05 ran the sibling `release-readiness.ps1` gate, a different script).
- **Fix:** Widened the regex to `-match 'failed to select a version for the requirement' -or -match 'no matching package named'`, and updated the accompanying docstring/comments to describe both phrasings and why they both represent the same pre-publish condition.
- **Files modified:** `scripts/release-dry-run.ps1`
- **Verification:** Re-ran `pwsh -File scripts/release-dry-run.ps1`; exit 0, verdict line `PASS: Hard failures: 0. Blocked: 2 (pre-publish). Skipped: 1 (toolchain absent).` — matches the plan's `acceptance_criteria` verbatim.
- **Committed in:** `be3d0ecc` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug fix)
**Impact on plan:** The fix was necessary to reach the plan's own stated acceptance criteria (GREEN exit 0 with the documented verdict line) — without it, Task 3 could not have been completed as specified. No scope creep; the fix is scoped to the exact detection logic the plan's Task 3 depends on.

## Issues Encountered
None beyond the auto-fixed regex issue documented above.

## User Setup Required
None - no external service configuration required. No live registry credentials were referenced or required at any point (D-07 prepare-only posture).

## Next Phase Readiness
- RLS-14 is now fully satisfied: all 6 in-repo crates (111-05) + both sibling repos (this plan) report `0.70.0`; `Cargo.lock` shows zero unexpected third-party drift in any of the three repos; the prepare-only release gate (`release-dry-run.ps1`) and the release-readiness gate (`verify-dark.ps1 -Gate release-readiness`) are both GREEN; no operator push occurred anywhere.
- This closes the release-leapfrog half of Phase 111. CORE-01, CORE-02, VERIFY-01, and RLS-14 are all Complete — Phase 111's 4 requirements are fully satisfied.
- Phase 112 (Security + Residual, carries the live `crossbeam-epoch` RUSTSEC fix context) and Phase 113 (SPIFFE) remain unbuilt on the ROADMAP.
- The actual tag push + live registry publish of `0.70.0` remains a separate, explicit, operator-gated decision — not part of this milestone's scope (mirrors v3.1/v3.3/v3.4's prepare-only posture).

---
*Phase: 111-core-carry-resource-cli-verify-release-leapfrog*
*Completed: 2026-08-04*
