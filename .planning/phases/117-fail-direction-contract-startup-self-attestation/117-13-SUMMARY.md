---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 13
subsystem: windows-sandbox
tags: [windows, mandatory-integrity-label, seatbelt-analog, fail-direction-contract, remediation, clippy-cross-target]

# Dependency graph
requires:
  - phase: 117 (waves 1-5, 117-01..117-12)
    provides: layer registry, LayerAttestationFailed error variant, AppliedLabelsGuard + LabelCoverage from CR-14, NonoRemediation enum
provides:
  - "AppliedLabel::AlreadyAtRequiredLevel self-healing residue detection in AppliedLabelsGuard"
  - "NonoRemediation::ClearStaleLayerResidue { layer } variant, both exhaustive matches updated"
  - "NonoError::remediation() arm for LayerAttestationFailed"
affects: [117-14, 117-15, 117-16, 117-17, 117-18, 117-19]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Residue self-healing: compare a pre-existing OS security-descriptor state against THIS launch's own mode-derived expectation before treating it as a coverage gap"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
    - crates/nono/src/diagnostic/codes.rs
    - crates/nono-cli/src/query_ext.rs
    - crates/nono/src/error.rs

key-decisions:
  - "Self-healing residue is recognized by exact match (RID == SECURITY_MANDATORY_LOW_RID AND mask == label_mask_for_access_mode(rule.access)), not by any-Low-IL-label-present — narrows the trigger without widening what counts as covered"
  - "AlreadyAtRequiredLevel is never reverted by this guard's Drop (not applied this session; a concurrent session sharing the path may still depend on it), preserving the module's documented 'concurrent sessions: last session out restores' contract"
  - "Fixed a pre-existing test-data collision (Rule 1): the literal prior-mask 0x5 used in coverage_distinguishes_full_partial_and_zero_ace_launches's third-party-label fixture is numerically identical to AccessMode::Read's own wanted mask, so it would have silently become residue instead of a genuine mismatch under the new check — replaced with a computed label_mask_for_access_mode(AccessMode::Write)"
  - "NonoRemediation::ClearStaleLayerResidue is deliberately generic prose, not a CLI flag — no single flag clears an arbitrary layer's stale state and the exact remedy is layer-specific"

patterns-established:
  - "Residue-vs-coverage-gap disambiguation: compare prior OS state to the mask this launch's own policy would produce, not merely presence of a prior label"

requirements-completed: []  # CINT-02 spans 117-13..117-17; not marked complete until all wave-6 plans land (matches this repo's existing convention: TSBX-01/02 remain Pending in REQUIREMENTS.md despite Phase 116 being fully executed+verified)

# Metrics
duration: 25min
completed: 2026-08-10
---

# Phase 117 Plan 13: Self-Healing AppliedLabelsGuard Residue + LayerAttestationFailed Remediation Summary

**AppliedLabelsGuard now recognizes its own abnormal-exit label residue (or an identical concurrent session's grant) as already-covered instead of hard-locking the operator out of their own workspace, and NonoError::remediation() no longer falls through to None for a startup self-attestation failure.**

## Performance

- **Duration:** ~25 min (first commit 17:35 UTC+1, last commit 17:50 UTC+1, plus investigation/verification time)
- **Started:** 2026-08-10
- **Completed:** 2026-08-10
- **Tasks:** 3/3
- **Files modified:** 4

## Accomplishments
- Closed NR3-01, the one open BLOCKER from `117-REVIEW.md` iteration 3: a workspace's stale mandatory-label residue from an abnormal exit (Ctrl-C/kill/crash) no longer self-locks the next launch of the same policy
- Confirmed the negative direction still holds: a genuine zero-coverage launch (no matching residue) still hard-aborts (`guard_skips_path_not_owned_by_current_user`, unchanged, still passing) and a mask-mismatched prior label still records a real coverage gap (new `mismatched_prior_mask_still_records_a_coverage_gap` test)
- `NonoError::remediation()` returns a typed, layer-naming `NonoRemediation::ClearStaleLayerResidue { layer }` for `LayerAttestationFailed` instead of silently returning `None`
- Both mandatory cross-target clippy gates (linux-gnu via `cross`, apple-darwin via `cargo-zigbuild`) re-ran GREEN across all three tasks with `-D warnings -D clippy::unwrap_used`

## Task Commits

Each task was committed atomically:

1. **Task 1: Self-healing residue detection in AppliedLabelsGuard (NR3-01 core fix)** - `da256a15` (fix)
2. **Task 2: NonoRemediation::ClearStaleLayerResidue variant (both exhaustive matches)** - `e77f64d4` (feat)
3. **Task 3: Wire NonoError::remediation() for LayerAttestationFailed** - `068d6f94` (fix)

_No TDD test-first commit split was made — Task 1 and Task 3 are `tdd="true"` but each task's test(s) and implementation landed together in a single commit per task, matching this task's `<action>` which specifies adding the implementation and its tests as one unit; both directions (residue recovers; genuine zero-coverage still aborts) are covered within Task 1's single commit._

## Files Created/Modified
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` - Added `AppliedLabel::AlreadyAtRequiredLevel` variant, self-healing branch in `snapshot_and_apply`'s `prior.is_some()` arm, `coverage()`/`revert_all()` arms for the new variant, 2 new regression tests (residue self-heals; mask mismatch still degrades), and fixed a test-data collision in the pre-existing `coverage_distinguishes_full_partial_and_zero_ace_launches` test
- `crates/nono/src/diagnostic/codes.rs` - Added `NonoRemediation::ClearStaleLayerResidue { layer: String }` and its arm in `suggested_flag_for_remediation`'s exhaustive match
- `crates/nono-cli/src/query_ext.rs` - Added the matching arm to the `cfg(not(target_os = "windows"))`-gated `suggested_flag_for_remediation` copy
- `crates/nono/src/error.rs` - Added `remediation()` arm for `LayerAttestationFailed`, threading the real `layer` field through; added a regression test pinning the exact `Some(...)` value

## Decisions Made
- Self-healing match is exact (RID + mask), never "any prior Low-IL label" — narrows the CR-14 trigger without widening what CR-14 correctly hardened
- `AlreadyAtRequiredLevel` entries are never reverted (not ours to tear down; a concurrent session may still depend on the ACE)
- `ClearStaleLayerResidue` stays generic prose rather than a CLI flag, since the real remedy is layer-specific
- CINT-02 in `REQUIREMENTS.md` left `Pending` (not marked complete) — it spans plans `117-13`..`117-17`; matches this repo's existing pattern where multi-plan requirements are flipped to `Complete` only at full closure, not per-plan (e.g. TSBX-01/02 remain `Pending` despite Phase 116 being fully executed and verified)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed a plan-authoring/pre-existing-test numeric collision that the new self-healing check would have silently exploited**
- **Found during:** Task 1 (running the new tests against the existing test suite)
- **Issue:** Two places in the codebase used the literal mask `0x5` as a stand-in for "some other party's label": (a) the plan's own `<interfaces>` section described `0x5` as "Write-class mask ... a different bit pattern" from `AccessMode::Read`'s wanted mask, but `label_mask_for_access_mode(AccessMode::Read)` is *exactly* `NO_WRITE_UP | NO_EXECUTE_UP = 0x5` — the plan's own worked example was wrong; (b) the pre-existing `coverage_distinguishes_full_partial_and_zero_ace_launches` test pre-labeled its "third-party label" fixture with the same literal `0x5` against a `file_rule` (`AccessMode::Read`) — under the new self-healing check this collision would have silently reclassified that fixture as residue (`AlreadyAtRequiredLevel`) instead of a genuine mismatch, flipping `coverage.applied` from 1 to 2 and breaking the test's `PartiallyApplied` assertion (confirmed live: first test run failed with `left: 2, right: 1`)
- **Fix:** In the new `mismatched_prior_mask_still_records_a_coverage_gap` test, replaced the literal `0x5` with a computed `label_mask_for_access_mode(AccessMode::Write)` plus an `assert_ne!` precondition proving it differs from Read's wanted mask. In the pre-existing `coverage_distinguishes_full_partial_and_zero_ace_launches` test, replaced its prelabel literal `0x5` with the same computed `Write` mask and an `assert_ne!` precondition, preserving the test's original intent (a genuine third-party/mismatched label producing `PartiallyApplied`) without relying on a numeric coincidence
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs`
- **Verification:** `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 labels_guard::` — 9/9 passing after the fix (was 7/9 before)
- **Committed in:** `da256a15` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 - bug in test data / plan worked-example)
**Impact on plan:** Necessary for correctness of the new negative-direction test and to prevent the fix from silently changing an existing test's meaning. No scope creep — same file, same task, discovered while running the plan's own prescribed `<verify>` command.

## Issues Encountered
None beyond the deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness

Wave 6 continues with `117-14`, `117-15`, `117-16`, `117-17` (all `depends_on: []`, mutually independent of each other and of `117-13`). Wave 7's `117-18` depends on `117-14`/`117-15`/`117-16`. Wave 8's `117-19` depends on all six wave-6/7 plans including this one. No blockers for the remaining gap-closure plans from this plan's work — `117-13` touched only `labels_guard.rs`, `error.rs`, `codes.rs`, and `query_ext.rs`, none of which are in the `files_modified` list of `117-14` through `117-17` (per the plan set's own wave-6 parallel-and-disjoint-files framing).

Both mandatory cross-target clippy gates (`cross` linux-gnu, `cargo-zigbuild` apple-darwin) were run locally for this plan and are GREEN as of this commit — no PARTIAL→CI fallback was needed. `cargo build --workspace --all-targets`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --features layer-fault-injection -- -D warnings -D clippy::unwrap_used` all clean on this host.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs`
- FOUND: `crates/nono/src/diagnostic/codes.rs`
- FOUND: `crates/nono-cli/src/query_ext.rs`
- FOUND: `crates/nono/src/error.rs`
- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-13-SUMMARY.md`
- FOUND: commit `da256a15` (Task 1)
- FOUND: commit `e77f64d4` (Task 2)
- FOUND: commit `068d6f94` (Task 3)
- FOUND: this SUMMARY.md itself is committed in a `docs(117-13)` commit following the three task commits above (self-referential hash omitted deliberately — amending this file to record its own post-amend hash would not terminate)
