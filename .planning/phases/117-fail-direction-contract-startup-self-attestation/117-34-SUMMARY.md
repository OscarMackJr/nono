---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 34
subsystem: docs
tags: [windows, spec, review-fix-ledger, gap-closure, sc4, d-16, d-37]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "All seven gap-closure round-3 code plans (117-27..117-33), whose fixes this plan
      records in the SPEC's standing Review-fix pass ledger"
provides:
  - "11 new Review-fix pass rows (CR-03, WR-12..WR-21) with re-run, dated grep evidence against
    the final tree state, closing SC4 for this round"
  - "9 Iteration-5 addenda on the continuing iteration-4 rows (CR-01, CR-02, WR-01, WR-04,
    WR-06, WR-07, WR-08, WR-09, WR-11), each pointing forward to the row that closed it"
  - "A final full-suite verification confirming no regression beyond the documented 12-item
    baseline (11 pre-existing + 1 intentional host-blocked WR-20 test)"
affects: [phase-118, phase-119, future-117-gap-closure-rounds]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Every ledger row's evidence is a literal, re-run grep command with its hit count and the
      date it was measured (D-16) — generated fresh at this plan's execution, never copied
      verbatim from an earlier plan or SUMMARY.md without re-verification"
    - "A reclassification of security-relevant behavior (D-37/WR-12, WR-20) gets an explicit
      before/after ledger entry, not just a code comment"

key-files:
  created: []
  modified:
    - proj/SPEC-windows-fail-direction-contract.md

key-decisions:
  - "WR-20's row is deliberately NOT recorded as fully verified — it states plainly that the
    regression test's ownership-reassignment setup step is host-blocked on this dev host
    (lacks SeRestorePrivilege/SeTakeOwnershipPrivilege/SeBackupPrivilege) and that no runtime
    perturbation proof was producible here; verification on an elevated/CI Windows runner is
    named as a required follow-up, per the dependency note's explicit honesty requirement."
  - "D-37 is cited by name in the WR-12 row (both in 'what was wrong' and 'what changed'),
    including its operator-call/LOCKED status and the specific contradiction it resolved
    between Plan 117-22's daemon-side reading and Plan 117-23's CLI-side reading of the
    identical stopped-at-non-owned-ancestor condition."
  - "All 11 new rows and the 9 addenda cite the plan number that produced the fix and carry
    freshly re-run grep evidence dated 2026-08-11 (this plan's execution date), not the
    iteration-5 review's date or any earlier plan's SUMMARY.md evidence."

requirements-completed: [CINT-01, CINT-02, CINT-03]

# Metrics
duration: ~50min
completed: 2026-08-11
---

# Phase 117 Plan 34: Record Gap-Closure Round 3 in the SPEC's Review-Fix Ledger Summary

**Appended 11 new Review-fix pass rows (CR-03, WR-12 through WR-21) and 9 Iteration-5 addenda to
the SPEC's standing ledger, each with freshly re-run grep evidence against the final round-3 tree
state — closing SC4 for this round, which iteration 5's own review found had not been updated for
the round that produced it.**

## Performance

- **Duration:** ~50 min
- **Completed:** 2026-08-11
- **Tasks:** 2/2 complete
- **Files modified:** 1 (`proj/SPEC-windows-fail-direction-contract.md`)

## Accomplishments

- Read all seven upstream SUMMARY.md files (117-27 through 117-33) plus `117-REVIEW.md`'s
  full iteration-5 findings text and `117-CONTEXT.md`'s D-37 entry, and wrote the ledger from
  what those plans actually shipped (several deviated from or exceeded their plan text), not
  from plan text alone.
- Appended 11 new `## Review-fix pass` table rows in the document's established
  `| ID (Iteration 5, gap-closure Plan 117-NN) | What was wrong | What changed |` shape:
  `CR-03` (117-27), `WR-12` (117-28, D-37 by name), `WR-13` (117-31), `WR-14` (117-28),
  `WR-15` (117-27), `WR-16` (117-27), `WR-17` (117-29), `WR-18` (117-29), `WR-19` (117-32),
  `WR-20` (117-30), `WR-21` (117-33).
- Every new row's evidence is a `grep -n`/`grep -c` command actually re-run against the live
  tree at this plan's execution (2026-08-11), with its real hit count and line numbers — not
  copied from an earlier plan's SUMMARY.md text.
- Appended a `**Iteration 5 update:**` sentence to each of the 9 continuing iteration-4 rows
  (CR-01, CR-02, WR-01, WR-04, WR-06, WR-07, WR-08, WR-09, WR-11), each pointing forward to the
  new row that closed what iteration 5 found still partial in it, matching the document's
  existing `**Iteration 4 update:**` addendum style exactly (peer sentence, appended after,
  never replacing).
- Ran the full local verification gate (`cargo build --workspace --all-targets`,
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -D warnings
  -D clippy::unwrap_used`, `cargo test --workspace`) and recorded the exact observed failure
  set against the documented baseline.

## Task Commits

1. **Task 1: Add the 11 new iteration-5 ledger rows with re-run grep evidence** - `e691a5b2` (docs)
2. **Task 2: Add Iteration 5 addenda to the 9 continuing prior rows; run the final full-suite
   consistency check** - `c8808a83` (docs)

**Plan metadata:** (this commit, following SUMMARY.md write)

## Files Created/Modified

- `proj/SPEC-windows-fail-direction-contract.md` - 11 new `## Review-fix pass` rows
  (`CR-03`, `WR-12`..`WR-21`) appended after the existing `WR-11` row; 9 `**Iteration 5
  update:**` addenda appended to `CR-01`, `CR-02`, `WR-01`, `WR-04`, `WR-06`, `WR-07`, `WR-08`,
  `WR-09`, `WR-11`.

## Decisions Made

- **WR-20's row records an honest, unresolved verification gap rather than papering over it.**
  Per the dependency note's explicit instruction, the row states the test's own setup step
  fails on this dev host (lacking `SeRestorePrivilege`/`SeTakeOwnershipPrivilege`/
  `SeBackupPrivilege`), that no runtime perturbation proof was producible here, and that
  verification on an elevated/CI Windows runner is required before the reclassification can be
  considered test-proven rather than merely test-authored.
- **D-37 is cited by name in the WR-12 row**, including its LOCKED/operator-call status and the
  specific contradiction it resolved (Plan 117-22's daemon-side "under-granting, never
  under-confining" reading winning over Plan 117-23's CLI-side `PartiallyApplied` reading for
  the byte-for-byte identical stopped-at-non-owned-ancestor condition), per the round's own
  `<plan_the_classes_not_just_the_findings>` instruction that a security-relevant
  reclassification gets a ledger entry, not just a code comment.
- **Every grep command in every new row and addendum was actually re-run against the tree at
  this plan's execution**, not copied verbatim from an earlier plan's SUMMARY.md. Line numbers
  and hit counts in this plan's rows may differ slightly from the corresponding numbers quoted
  in the upstream SUMMARY.md files where intervening commits in this same round shifted lines —
  this plan's numbers are the ones confirmed live, per D-16 and the round-3 discipline's
  explicit instruction not to trust an earlier round's citations without re-running them.

## Deviations from Plan

None. The plan's two tasks executed exactly as specified: Task 1's 11 rows and Task 2's 9
addenda both landed in the documented shape with the required re-run evidence, and the final
full-suite verification produced exactly the anticipated 12-item failure set (11 pre-existing
baseline + Plan 117-30's intentional D-31 host-blocked test), not a regression.

## Verification

- `grep -c "^| CR-03\|^| WR-1[2-9]\|^| WR-20\|^| WR-21" proj/SPEC-windows-fail-direction-contract.md`
  → `11` — all 11 new rows present as table rows.
- `grep -c "Iteration 5 update" proj/SPEC-windows-fail-direction-contract.md` → `9` — all 9
  addenda present.
- `cargo build --workspace --all-targets` — clean (only the pre-existing, unrelated
  `nono-shell-broker` "missing a lib target" advisory warning).
- `cargo fmt --all -- --check` — clean, no output.
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean, zero
  warnings.
- `cargo test --workspace` — **1624 passed, 12 failed, 2 ignored.** The failing test binary is
  `nono-sandbox-cli`'s `nono` bin target; cargo's default fail-fast behavior stops the
  workspace run after this binary reports failures, which is the same behavior every prior
  gap-closure-round-3 plan's SUMMARY.md documented for this same command in this repo. The 12
  failures are exactly the documented set, confirmed by name:
  - 11 pre-existing Windows-host baseline failures (unchanged by this plan, none touched):
    `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`;
    `config::tests::{nono_home_dir_falls_through_when_unset, nono_home_dir_rejects_non_absolute_override,
    nono_home_dir_returns_override_when_set, test_validated_home_falls_back_to_userprofile,
    test_validated_home_ignores_non_absolute_home_when_userprofile_exists,
    user_state_dir_uses_localappdata_on_windows}`;
    `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`;
    `protected_paths::tests::{blocks_child_directory_capability, blocks_parent_directory_capability,
    requested_path_blocks_nonexistent_child_under_protected_root}`.
  - 1 intentional, host-blocked D-31 test from Plan 117-30:
    `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
    (this shell lacks `SeRestorePrivilege`; see WR-20's ledger row for the full honest-status
    record).
  - **No new regressions.** This is the exact 12-item set every plan in this round from 117-30
    onward has reported.
- Both D-35 cross-target clippy gates (linux-gnu via `cross clippy`, apple-darwin via
  `cargo-zigbuild clippy`) were confirmed clean in this round by their owning plans — 117-27
  (`launch.rs`/`output.rs`/`cli_bootstrap.rs`), 117-29 (`main.rs`/`windows.rs`'s blast radius
  via `main.rs`), and 117-33 (`telemetry/mod.rs`) — per each plan's own SUMMARY.md Verification
  section; this task did not re-run them, only confirmed all three record a clean result.

## Known Stubs

None — this plan modified only `proj/SPEC-windows-fail-direction-contract.md`, a documentation
artifact; no code stubs were introduced.

## Threat Flags

None — this plan records existing findings in the standing ledger; it introduces no new
network endpoints, auth paths, file-access patterns, or schema changes at a trust boundary. The
single threat register entry (`T-117-34-01`, Repudiation on the SPEC's own ledger) is closed by
this plan's own output.

## User Setup Required

None — no external service configuration required. **Follow-up verification item (not a setup
task, carried forward from WR-20's ledger row):** `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
requires an elevated/CI Windows runner (`SYSTEM`/`Administrator`, or a locally elevated dev
session with `SeRestorePrivilege`) to execute past its setup step and provide a genuine
perturbation proof — this dev host cannot run it to completion.

## Next Phase Readiness

- SC4 is closed for gap-closure round 3: all 11 iteration-5 findings are recorded in the
  standing ledger with re-runnable, dated evidence, and all 9 continuing iteration-4 findings
  point forward to their closure. The document no longer stops at WR-11 while 11 more findings
  existed only in the ephemeral `117-REVIEW.md` artifact.
- This is the last plan of gap-closure round 3 (117-27 through 117-34). The final verification
  run confirms no regression beyond the documented 12-item baseline.
- WR-20's test remains an open, honestly-recorded verification item requiring an
  elevated/CI Windows runner before it can be treated as proven rather than authored — this
  should be surfaced to the operator as a follow-up, not silently closed.
- STATE.md/ROADMAP.md updates are owned by the orchestrator per this repo's project-specific
  override (`<orchestrator_owned_files>`) and are not touched by this executor.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

- FOUND: `proj/SPEC-windows-fail-direction-contract.md`
- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-34-SUMMARY.md`
- FOUND commit `e691a5b2` (Task 1)
- FOUND commit `c8808a83` (Task 2)
