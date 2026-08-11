---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 22
subsystem: security-attestation
tags: [windows, dacl, attestation, doc-correctness, discovery-tests, rust]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "Plan 17's DaemonAttestationDecision two-state enum and its NR3-05 doc comment; Plan 09/10's CLI-side three-state AttestationDecision"
provides:
  - "Corrected DaemonAttestationDecision doc comment stating the true under-granting-not-under-confining rationale"
  - "Reusable parse_enum_variant_names(src, enum_marker) helper for discovery-based enum-shape tests"
  - "every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence: a cross-mirror subset test pinning the daemon/CLI AttestationDecision relationship"
  - "DAEMON-DECISION-ENUM anchor marker + occurrence-count-hardened two-state discovery test"
  - "daemon_attest_and_decide_result_matches_exhaustively: an exhaustive-match behavioral test with no wildcard arm"
affects: [117-review-gap-closure, future-DaemonAttestationDecision-changes, future-AttestationDecision-changes]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Anchor-comment + occurrence-count assertion for self-referencing include_str! discovery tests (avoids first-match blindness when a search literal necessarily appears twice in its own source)"
    - "Exhaustive match with no wildcard arm as a compile-time pin against future enum variants, placed in the test module itself (not just production call sites)"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/agent_daemon/launch.rs

key-decisions:
  - "Recovered the interrupted prior executor's diff after auditing every doc-comment claim against DaemonDaclGuard::apply's actual three passes (lines 147-260) and against the CLI's AppliedDaclGrantsGuard::SkipWritableNotOwned (dacl_guard.rs) — all claims verified accurate, no corrections needed"
  - "Split the single recovered diff into two atomic per-task commits matching the plan's task boundaries, by first downgrading the file to a 'Task 1 only' intermediate state (no anchor comment, non-anchor two-state test, no exhaustive test), committing, then restoring the full target content for the Task 2 commit"
  - "Proved all three new/hardened test mechanisms can actually fail by deliberately perturbing the source three separate times (duplicating the anchor literal, adding an undocumented CLI variant, adding an undocumented daemon variant) and confirming each trips, then reverting before committing"

requirements-completed: [CINT-02]

# Metrics
duration: ~45min
completed: 2026-08-11
---

# Phase 117 Plan 22: Correct DaemonAttestationDecision Doc + Harden Discovery Tests Summary

**Corrected the DaemonAttestationDecision doc comment's false "no partial-success return" premise to the true under-granting-vs-under-confining rationale, added a discovery-based cross-mirror subset test against the CLI's three-state AttestationDecision, hardened the existing two-state test against its own self-referencing literal via an anchor-comment + occurrence-count assertion, and added an exhaustive-match behavioral test with no wildcard arm.**

## Performance

- **Duration:** ~45 min (continuation of an interrupted prior session; this session's work was auditing the recovered diff, verifying every claim against production code, splitting it into atomic task commits, and proving each new test mechanism can actually fail)
- **Completed:** 2026-08-11T01:23:02Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- WR-06 closed: `DaemonAttestationDecision`'s doc comment now states the true reason `ProceedDowngraded` stays removed — passes 1 and 3 of `DaemonDaclGuard::apply` DO have real skip/break arms `granted_write_access()` cannot express, but their fail direction is always under-granting (less filesystem access than the policy intended), never under-confining (the sandbox boundary is never wider than intended), unlike the CLI's `SkipWritableNotOwned` which gates a promised WRITE grant
- WR-06's requested cross-mirror test added: `every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence` parses both enums' variant names from their own source text via `include_str!` and asserts the daemon side stays a documented subset of the CLI side (with `ProceedDowngraded` named as the one intentional divergence)
- WR-11 closed: `daemon_attestation_decision_is_deliberately_two_state` no longer parses the first occurrence of a literal that necessarily appears twice in its own source — it now anchors on a `DAEMON-DECISION-ENUM` marker comment and asserts the marker occurs exactly twice before trusting which segment holds the real enum
- WR-11's second half closed: `daemon_attest_and_decide_result_matches_exhaustively` drives the real function with a deterministic null-process-handle fixture and matches the result with an exhaustive `match` (no `_` arm), so a future third variant fails this test file's own compilation

## Task Commits

Each task was committed atomically:

1. **Task 1: Correct the doc comment's false premise; add the cross-mirror subset test** - `7c23886e` (docs)
2. **Task 2: Harden the two-state discovery test against its own self-reference (WR-11); add an exhaustive-match behavioral test** - `2d84196f` (test)

**Plan metadata:** (this SUMMARY.md commit)

## Files Created/Modified
- `crates/nono-cli/src/agent_daemon/launch.rs` - Corrected `DaemonAttestationDecision` doc comment, added `parse_enum_variant_names` helper, added `every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence`, added `DAEMON-DECISION-ENUM` anchor + hardened `daemon_attestation_decision_is_deliberately_two_state`, added `daemon_attest_and_decide_result_matches_exhaustively`

## Decisions Made
- Recovered the interrupted prior executor's uncommitted diff (~229/49) after a full claim-by-claim audit against `DaemonDaclGuard::apply`'s actual three passes (read lines 100-260) and the CLI's `AppliedDaclGrantsGuard::SkipWritableNotOwned` (`dacl_guard.rs` lines 207-241): pass 1's `Ok(false)` skip arm genuinely warns-and-records-nothing on a non-owned read-only-rule path; pass 2 is genuinely fail-closed end-to-end (`Ok(false)` and `Err` both `revert_all` + return `Err`); pass 3 genuinely `break`s at the first non-owned ancestor; `SkipWritableNotOwned` genuinely gates a WRITE-class rule, not a read-only one. All doc-comment assertions checked out — no corrections were needed, only verification.
- Split the single recovered diff into two atomic per-task commits by reconstructing an intermediate "Task 1 only" file state (doc fix + helper + cross-mirror test, two-state test refactored to use the helper but not yet anchor-hardened, no exhaustive test), running the full test/clippy/fmt gate on that intermediate state, committing it, then restoring the complete target content (anchor comment + anchor-hardened two-state test + exhaustive test) for the Task 2 commit. This matches the plan's task-boundary intent (Task 2's `<action>` explicitly assigns the anchor-comment insertion) more precisely than committing the whole recovered diff in one lump.
- Proved each new/hardened test mechanism can actually fail, per this plan's `<verification_note>`, by three separate temporary perturbations (each reverted before the corresponding commit): (1) duplicated the `DAEMON-DECISION-ENUM` literal a third time — the occurrence-count assertion failed with `left: 3, right: 2` as expected; (2) added an undocumented `PerturbationTestVariant` to the CLI's `AttestationDecision` — the cross-mirror test failed naming the exact unaccounted-for variant; (3) added an undocumented `PerturbationTestVariant` to `DaemonAttestationDecision` — compilation failed with `E0004: non-exhaustive patterns` at the production `launch_agent` match site (and would equally break the new exhaustive-match test's own `match`).

## Deviations from Plan

None - the recovered diff's implementation matched every `<action>`/`<behavior>` item in both tasks; my work was auditing, splitting into atomic commits, and proving non-vacuity via perturbation, not authoring new logic.

## Issues Encountered
- `cargo fmt --check` flagged a stray blank line left over from reconstructing the Task-1-only intermediate file state (an artifact of manually removing the Task 2 test block). Fixed with `cargo fmt --package nono-sandbox-cli` before the Task 1 commit; re-verified clean.
- The plan's stated verification command `cargo test -p nono-sandbox-cli --lib agent_daemon::launch` does not work as written — `nono-sandbox-cli` has no library target (it is a two-binary crate: `nono` and `nono-agentd`, no `src/lib.rs`). The `agent_daemon` module is `#[path]`-included only into `nono-agentd.rs`, gated `#[cfg(target_os = "windows")]`. Used `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch` instead, which exercises the identical test set. Not treated as a plan defect worth a deviation entry — it's a copy-paste-stale flag in the plan's own verify block, and the equivalent working command was self-evident from the crate layout.

## Cross-Target Verification

`crates/nono-cli/src/agent_daemon/launch.rs` is compiled only under `#[cfg(target_os = "windows")]` (the `mod agent_daemon;` declaration itself in `nono-agentd.rs` is gated on `target_os = "windows"`). It contains no `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks, and is not under `exec_strategy/` or `bindings/c/src/`. Per CLAUDE.md's cross-target-verify scope, the linux-gnu `cross clippy` gate does not apply to this change.

Ran on the Windows host:
- `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch` — 21/21 passed (both before and after each commit)
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean
- `cargo fmt --check` — clean

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- WR-06 and WR-11 are both fully closed for this plan's scope; no follow-on work identified.
- The `parse_enum_variant_names` helper is now available for any future discovery-based enum-shape test in this file.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-22-SUMMARY.md`
- FOUND commit: `7c23886e` (Task 1)
- FOUND commit: `2d84196f` (Task 2)
- FOUND commit: `b1438a52` (this SUMMARY.md)
- Working tree clean for `crates/nono-cli/src/agent_daemon/launch.rs`
