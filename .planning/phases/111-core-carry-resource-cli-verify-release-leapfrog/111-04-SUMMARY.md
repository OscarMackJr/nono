---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
plan: 04
subsystem: testing
tags: [cross-target-clippy, cargo-test, maturin, napi, fork-invariant-verification, adr-86, d-01]

requires:
  - phase: 111-01
    provides: "CORE-01 upstream carries (~/.cache policy grant, MAX_CRYPTO_THREADS 7->12)"
  - phase: 111-02
    provides: "CORE-02 help-text correction for --memory/--timeout/--max-processes"
  - phase: 111-03
    provides: "ADR-111 + 108-DIVERGENCE-LEDGER standing-divergence addendum"
  - phase: 110-08
    provides: "the prior fork-invariant certification this plan explicitly supersedes per D-09"
provides:
  - "Combined 108-111 fork-invariant verification record (111-04-VERIFICATION-NOTES.md)"
  - "Both mandatory cross-target clippy gates confirmed GREEN over the combined surface"
  - "24-name failure baseline diff, honestly reproduced as a strict subset (no new regressions)"
  - "Both sibling bindings (nono-py, nono-ts) confirmed rebuilding clean at 0.66.1"
affects: [111-05, 111-06]

tech-stack:
  added: []
  patterns: ["baseline-diff verification (never re-litigate, only diff)", "explicit run_in_background:true for long-running test suites (harness auto-promotion-on-timeout silently kills the job)"]

key-files:
  created:
    - .planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-04-VERIFICATION-NOTES.md
  modified: []

key-decisions:
  - "First cargo test --workspace --no-fail-fast attempt was silently killed by this harness's timeout-auto-promotion-to-background behavior (no process/log growth for ~39min mid-env_vars.rs); redone explicitly with run_in_background:true, which completed end-to-end in ~50 additional minutes."
  - "111-04-VERIFICATION-NOTES.md explicitly supersedes 110-08-VERIFICATION-NOTES.md per D-09, since that record predates 4 defect-fix commits (7c7a189c/ea26b5b2/6d7ef719/4aec1944) and Phase 111's own CORE-01/CORE-02 changes."

patterns-established:
  - "Long-running (>10min) cargo test suites on this host MUST be started with run_in_background:true explicitly, not left to the harness's foreground-timeout auto-promotion, which has been observed to silently kill the process without writing an exit marker."

requirements-completed: [VERIFY-01]

duration: 100min
completed: 2026-08-04
---

# Phase 111 Plan 04: Combined 108-111 Fork-Invariant Verification Summary

**Both mandatory cross-target clippy gates re-confirmed GREEN over the combined 108-111 surface, D-01 and the ADR-86 Windows carve-out both confirmed structurally unregressed, the cargo test --workspace --no-fail-fast failure set (17 names) confirmed as a strict subset of the documented 24-name baseline, and both sibling bindings (nono-py, nono-ts) confirmed rebuilding clean — superseding the now-stale 110-08 certification per D-09.**

## Performance

- **Duration:** ~100 min (including one ~40-min stalled first attempt at the full workspace test sweep that had to be diagnosed and redone)
- **Started:** 2026-08-04T23:37:00Z (approx, first Task 1 read)
- **Completed:** 2026-08-05T01:14:29Z
- **Tasks:** 3
- **Files modified:** 1 (created)

## Accomplishments

- Both mandatory local cross-target clippy gates (`cross clippy` linux-gnu, `cargo-zigbuild clippy` apple-darwin) re-ran GREEN with zero findings over the tree with Wave 1 (111-01/02/03) merged, satisfying D-09/D-10's "combined 108-111 surface" requirement rather than merely re-verifying Phase 111's own small diff.
- D-01 (no `pub mod resource;` in `crates/nono/src/lib.rs`) confirmed still holding after CORE-01/CORE-02's edits — resource limits remain CLI-side per ADR-111's ADAPT-not-adopt disposition.
- ADR-86's Windows denial-rendering carve-out (`crates/nono-cli/src/exec_strategy_windows/`) confirmed structurally unregressed via an empty `git diff` since the pre-Wave-1 base SHA.
- `cargo test --workspace --no-fail-fast`'s failing-test-name set (17 names) is a confirmed strict subset of the documented 24-name baseline from `110-08-VERIFICATION-NOTES.md` — zero new failure names, diffed name-by-name, not merely re-litigated.
- Both sibling binding repos (`../nono-py` via `maturin build`, `../nono-ts` via `npx napi build --platform --release`) rebuild clean with no fix required at the current pre-bump `0.66.1` version.

## Task Commits

Each task was committed atomically:

1. **Task 1: Both cross-target clippy gates + D-01/ADR-86 grep-diff assertions** - `d7928c07` (docs)
2. **Task 2: make ci substitution + 24-name baseline diff + resource-flag regression guard** - `f027805e` (docs)
3. **Task 3: Rebuild both sibling bindings + write the verification-notes record** - `ea3990ad` (docs)

_This plan is verification-only — all 3 tasks are `docs` commits appending to a single evidence file; no source code was modified._

## Files Created/Modified

- `.planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-04-VERIFICATION-NOTES.md` - The combined-108-111 fork-invariant verification record, explicitly superseding `110-08-VERIFICATION-NOTES.md` per D-09

## Decisions Made

- **First full-suite test run silently killed by harness auto-promotion, redone explicitly.** The first `cargo test --workspace --no-fail-fast` invocation was started as a foreground command; when it exceeded this harness's 600s wall-clock cap it was auto-promoted to a background job. That auto-promoted job then produced zero further log output for ~39 minutes while stalled inside `env_vars.rs`, with no `cargo`/`nono`/`rustc` process observable via `tasklist`/`ps` — indicating the harness silently killed it rather than letting it continue. This is a harness/environment behavior, not a code defect, and is documented verbatim in `111-04-VERIFICATION-NOTES.md`. The run was redone with `run_in_background: true` set explicitly from the start, which completed end-to-end (`env_vars.rs` alone took 2702s/45min this run, vs 986s/16.4min in `110-08` — consistent with that binary's already-documented host-state-contention flakiness, not a new regression).
- **111-04-VERIFICATION-NOTES.md explicitly supersedes 110-08-VERIFICATION-NOTES.md per D-09.** That record predates four defect-fix commits landed 2026-08-04 (`7c7a189c` `has_port_rules()` omission, `ea26b5b2` `cmd_show`'s missing `Network:` section, `6d7ef719` version-skew fail-open, `4aec1944` broken filter-sweep enumeration) plus Phase 111's own CORE-01/CORE-02 changes. `has_port_rules()` shipped broken straight through the 110-08 certification — exactly the scenario D-09 exists to prevent.

## Deviations from Plan

None - plan executed exactly as written. The harness-auto-promotion stall (documented above) required a workaround (explicit `run_in_background: true`) but did not require any deviation from the plan's tasks, acceptance criteria, or scope — it is a process/tooling note, not a code change or a Rule 1-4 deviation.

## Issues Encountered

- **Harness auto-promoted background job silently died.** See "Decisions Made" above. Resolved by restarting the same command with `run_in_background: true` set explicitly rather than relying on foreground-timeout auto-promotion.
- **`env_vars.rs` took 45 minutes this run** (vs 16.4 min in 110-08) with only 3/10 of its previously-documented failure names reproducing (the other 7 passed this run). Both observations are consistent with 110-08's own characterization of this specific test binary as flaky due to host-state contention (stale mandatory-label ACEs on real host paths from prior sessions), not a new regression — all 3 names that did fail are within the documented 24-name baseline, and no name outside that baseline appeared.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

VERIFY-01 is satisfied. Both mandatory cross-target clippy gates, the `make ci` substitution set, the baseline-honest `cargo test --workspace --no-fail-fast` diff, and both sibling binding rebuilds are all GREEN (or, for the pre-existing test failures, honestly confirmed as a non-exceeding subset of the documented baseline) over the combined 108-111 surface. Phase 111 Wave 2 is complete. Ready for Wave 3 (`111-05`, the in-repo `0.70.0` version leapfrog) and Wave 4 (`111-06`, sibling-repo bump + prepare-only release gate).

---
*Phase: 111-core-carry-resource-cli-verify-release-leapfrog*
*Completed: 2026-08-04*

## Self-Check: PASSED

- FOUND: `.planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-04-VERIFICATION-NOTES.md`
- FOUND: `.planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-04-SUMMARY.md`
- FOUND: commit `d7928c07`
- FOUND: commit `f027805e`
- FOUND: commit `ea3990ad`
