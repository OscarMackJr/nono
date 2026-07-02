---
phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker
plan: 02
subsystem: infra
tags: [github-actions, ci-cd, release-pipeline, cargo-publish, cross-compile]

# Dependency graph
requires:
  - phase: 100-01
    provides: "workspace leapfrogged to crate version 0.66.1 (release.yml/ci.yml unaffected by that plan)"
provides:
  - "Idempotent publish-crates job in release.yml (cargo search guard on all 3 publishable crates)"
  - "Operator-invocable cross-compile pre-flight job in ci.yml (workflow_dispatch-gated)"
  - "proj/ADR-100-ci-pipeline-reconcile.md documenting the adopt-vs-adapt disposition for #1245/#1251"
affects: [100-03, 100-04, 100-05, release-runbook]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cargo search idempotency guard for retry-safe crates.io publish steps"
    - "workflow_dispatch as the fork's operator-invoked pre-release CI trigger (distinct from release.yml's own tag-input workflow_dispatch)"

key-files:
  created:
    - proj/ADR-100-ci-pipeline-reconcile.md
  modified:
    - .github/workflows/release.yml
    - .github/workflows/ci.yml

key-decisions:
  - "D-05: #1245/#1251 are ADAPTED, not adopted wholesale — publish-crates idempotency is a clean 1:1 port; cross-compile job body adopted verbatim except its if: trigger, rewritten from the dead upstream-only 'chore: release v...' PR-title convention to github.event_name == 'workflow_dispatch'"
  - "84b5e7ce's (#1251) pre-corrected quoted-string if: form applied inline from the start, never reproducing then re-fixing the double-braced ${{ }} bug"
  - "D-06 confirmed: release-readiness verify-dark gate and signed-MSI sign-before-harvest build order are untouched by either hunk — both edits are confined to publish-crates (downstream of signing) and ci.yml (separate pre-release pipeline)"

requirements-completed: [RLS-11]

# Metrics
duration: 12min
completed: 2026-07-02
---

# Phase 100 Plan 02: CI Pipeline Reconcile Summary

**Reconciled upstream #1245 (idempotent publish-crates + cross-compile check) and #1251 (its if: bugfix) into the fork's release/CI pipeline via ADAPT, not verbatim ADOPT — the cross-compile job's trigger was rewritten from a dead upstream-only PR-title convention to an operator-invocable `workflow_dispatch`.**

## Performance

- **Duration:** ~12 min (task work; excludes file-read/context-load overhead)
- **Started:** 2026-07-02T10:23:17-04:00 (first task commit)
- **Completed:** 2026-07-02T10:25:50-04:00 (last task commit)
- **Tasks:** 3 completed
- **Files modified:** 2 (release.yml, ci.yml); 1 file created (ADR-100)

## Accomplishments
- `release.yml`'s `publish-crates` job is now retry-safe: each of the 3 `cargo publish`
  calls (nono, nono-proxy, nono-cli) is guarded by a `cargo search` idempotency check
  using the fork's existing two-line `VERSION` stripping convention; re-running the job
  after a partial failure no longer errors on an already-published crate.
- `ci.yml` gained an operator-invocable `cross-compile:` job (4-target matrix:
  x86_64-unknown-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin,
  aarch64-unknown-linux-gnu, plus a Linux-only libdbus-link portability check) gated on
  `workflow_dispatch`, a trigger this fork's event model actually produces — replacing
  upstream's dead `chore: release v...` PR-title convention.
- `proj/ADR-100-ci-pipeline-reconcile.md` records the adopt-vs-adapt call for both
  upstream commits, mirroring ADR-98's Status/Context/Decision/Consequences shape.

## Task Commits

Each task was committed atomically:

1. **Task 1: Adapt #1245's idempotent publish-crates hunk into release.yml** - `7b78a859` (feat)
2. **Task 2: Adapt #1245+#1251's cross-compile job into ci.yml with a rewritten, fork-fireable trigger** - `b5582429` (feat)
3. **Task 3: Write proj/ADR-100-ci-pipeline-reconcile.md** - `5a722140` (docs)

## Files Created/Modified
- `.github/workflows/release.yml` - Each of the 3 `publish-crates` steps (nono, nono-proxy, nono-cli) now derives `VERSION` via the fork's two-line strip convention and wraps `cargo publish` in a `cargo search`-based idempotency guard; existing `sleep 30` waits and the job's `needs`/`if:` gate unchanged
- `.github/workflows/ci.yml` - Added `workflow_dispatch:` to the top `on:` block; inserted a `cross-compile:` job between `audit:` and `docs-checks:`, replaying upstream `ebd94275`'s job body verbatim except the `if:` trigger (rewritten to `github.event_name == 'workflow_dispatch'`, using #1251's pre-corrected quoted-string form)
- `proj/ADR-100-ci-pipeline-reconcile.md` - New ADR documenting the adopt-vs-adapt disposition for both upstream commits (`proj/` is gitignored but individually force-tracked for ADR files, matching the existing `proj/ADR-98-*.md` pattern)

## Decisions Made
- Both upstream hunks were ADAPTED rather than adopted verbatim, per plan D-05: the `publish-crates` idempotency check required no structural change (clean 1:1 port, just the fork's existing two-line VERSION-strip form instead of upstream's one-liner); the `cross-compile` job's trigger was the only structural change needed — it now fires on `workflow_dispatch` (a condition this fork's `changes` job resolves to `run_code_jobs=true` automatically, per the `*` event-name fallback at `ci.yml:53-57`/`:39`) instead of a PR-title convention this fork's manual tag-push/`workflow_dispatch` release model will never produce.
- `#1251`'s quoted-string `if:` form was applied inline from the start rather than reproducing the double-braced `${{ }}` bug and then "fixing" it in a follow-up commit — there is no value in replaying a since-fixed historical defect.
- The upstream job's `actions/checkout@9c091bb...` (v7.0.0) pin was carried over verbatim as directed by the plan's "replay verbatim except the if:" instruction, even though the rest of `ci.yml` pins `actions/checkout@de0fac2e...` (v6). This is a minor pin inconsistency, noted here rather than silently fixed, since the plan explicitly scoped the verbatim-replay boundary to only the `if:` condition.

## Deviations from Plan

None - plan executed exactly as written. All three tasks' verification commands and acceptance criteria passed on first attempt (`grep -c 'cargo search'` == 3, `sleep 30` count unmoved at 2, `workflow_dispatch:`/`cross-compile:` present with zero `if: ${{` occurrences inside the new job, ADR grep-count == 5 with both SHAs cited).

## Issues Encountered
- `proj/` is gitignored (`.gitignore:16`); `git add proj/ADR-100-ci-pipeline-reconcile.md` was rejected until force-added with `git add -f`, consistent with the existing pattern for other individually-tracked ADR files under `proj/` (same mechanism noted in project memory for `docs/cli/development/`).

## User Setup Required

None - no external service configuration required. This plan edits CI YAML only; no workflow was triggered, pushed, or dispatched (PREPARE ONLY scope, per platform notes).

## Next Phase Readiness
- `release.yml`'s `publish-crates` job and `ci.yml`'s new `cross-compile` job are ready for the eventual operator-gated release push (Phase 100's remaining plans / RELEASE-RUNBOOK.md).
- No blockers. The nono-py `RouteConfig` PyPI blocker (RLS-12) remains open for a later plan in this phase.
- Both YAML files were validated with `python -c "import yaml; yaml.safe_load(...)"` (parse-clean) as an additional automated sanity check beyond the plan's `grep`-based verification.

---
*Phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker*
*Completed: 2026-07-02*
