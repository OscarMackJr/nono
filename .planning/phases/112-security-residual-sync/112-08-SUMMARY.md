---
phase: 112-security-residual-sync
plan: 08
subsystem: infra
tags: [upstream-sync, divergence-ledger, close-out, requirements-reconciliation, roadmap, verification]

# Dependency graph
requires:
  - phase: 112-security-residual-sync
    provides: "All 7 prior plans' landed dispositions (112-01 through 112-07 SUMMARYs) — the executed reality this plan records, superseding 112-DISPOSITION-TABLE.md's pre-execution plan text in 3 places"
provides:
  - "Phase 112 Security + Residual Sync Addendum in 108-DIVERGENCE-LEDGER.md: all 18 SHA dispositions recorded as shipped, including 3 corrections to the pre-execution disposition table"
  - "SEC-09 Carry-Forward Note filed directly in the ledger's tool-sandbox-surface cluster section, obligating a future v3.7 planner to consciously re-decide f6f02751's relaxed guard"
  - "REQUIREMENTS.md reconciled: 10 Phase-112-owned requirements marked complete with cited evidence; SEC-02 explicitly carved out to Phase 114, not silently left Pending"
  - "ROADMAP.md Phase 112 checklist/plan/Progress-table entries flipped to complete; Amendment paragraph and Phase 114 section confirmed present, unedited"
  - "Combined Wave 2+3 fork-invariant verification: cargo fmt clean, full workspace test sweep's 17-name failing set confirmed identical to 111-04's own 17-name run, a strict subset of the 27-name baseline"
affects: [114-oauth-capture-absorb, future-v3.7-tool-sandbox-absorb, future-upst-sync-planning-methodology]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Ledger-addendum-records-shipped-reality, not plan-text: the addendum's per-SHA table cites each Wave 2/3 plan's actual landed outcome (112-0N-SUMMARY.md), explicitly diffing against 112-DISPOSITION-TABLE.md's pre-execution call where they diverged"
    - "Sequential single-branch execution (no worktree isolation) means each plan's cross-target clippy gate run already re-verifies the cumulative state of all prior plans in the same branch history — a documentation-only close-out plan can cite the last code-touching plan's GREEN gate result plus a zero-crates-diff check, rather than re-running multi-hour Docker/zig gates for no code change"

key-files:
  created: []
  modified:
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "Ledger addendum records 3 divergences between 112-DISPOSITION-TABLE.md's pre-execution call and what actually shipped: SEC-03 (a3243907) planned 'adopt, HIGH confidence' but shipped ADAPT (depends on the unabsorbed fa21a004/8a4237f2 #1283 SeccompPolicy refactor, a standing Phase-111 residual); RES-02/4cc0af2c52 planned 'adapt with caution, or skip' but shipped SKIP (target socket_test_dir() helper confirmed absent from the fork entirely, not merely risky); RES-02/9840a16f35 planned 'adopt, HIGH confidence' (implying near-verbatim) but shipped adopt-with-adaptation (fork's diagnostic formatter never emits upstream's literal denial-marker string)"
  - "Recorded a methodological lesson in the addendum: the disposition table's confidence column was derived from file-presence evidence (ls/git show --stat), not symbol-level verification (grep for the diff's referenced types/functions/fields) — and was wrong in 2 of the 3 dispositions re-checked at symbol level during execution. Future UPST-sync Wave-1 passes should require symbol-level verification before assigning HIGH confidence."
  - "SEC-07's newly-discovered pre-existing fork security defect (nono-proxy had no --no-auth concept; CONNECT-tunnel auth was unconditionally lenient, a session-token-boundary bypass on a standalone non-loopback proxy) is recorded in the addendum's 'New information generated during Phase 112 execution' section as a fork-internal finding, distinct from any upstream-sync disposition"
  - "SEC-02a/b/c recorded in the ledger addendum as 'deferred -> Phase 114' — not an adopt/adapt/decline verdict — per the ROADMAP Amendment (2026-08-05); the addendum and REQUIREMENTS.md both cite 112-OAUTH-CAPTURE-DISPOSITION.md's evidence trail so Phase 114 inherits the reality-check rather than re-deriving it"
  - "REQUIREMENTS.md: flipped SEC-01/SEC-08/SEC-09/RES-01/RES-02 to [x] (SEC-03..07 were already [x], flipped by their own single-contributing plans earlier in this phase per the standard per-plan requirements.mark-complete step) — bringing all 10 Phase-112-owned requirements to complete. SEC-02 deliberately left unchecked with an explicit '-> CARVED OUT to Phase 114' note, mirroring the NET-02 -> Phase 113 precedent, rather than silently left Pending"
  - "roadmap.update-plan-progress (SDK verb) returned updated:true but did not add a Progress-table row for Phase 112 — added the row by hand to keep the table consistent with Phases 108-111's existing rows"
  - "Did not re-run the multi-hour cross/cargo-zigbuild clippy Docker gates in this plan: git diff across all 3 of this plan's commits touches zero files under crates/ (docs-only), and git log confirms zero crates/ commits landed between 112-07's last gate-verified commit (ad8132e7, both gates GREEN with --all-targets, cargo build --workspace --all-targets clean) and this plan's HEAD — so the current crates/ state is byte-identical to what 112-07 already verified GREEN. Ran the parts of Task 2 that are meaningful on a zero-crates-diff plan instead: cargo fmt --all -- --check (clean) and the full cargo test --workspace --no-fail-fast baseline diff (17-name failing set, byte-identical to 111-04's own 17-name run, confirmed strict subset of the 27-name baseline)."

patterns-established:
  - "Phase-close ledger addendum shape: per-SHA table with 'table's planned disposition' vs 'disposition as shipped' columns, a dedicated 'Divergences' section explaining each mismatch with cited evidence, a 'New information' section for fork-internal findings surfaced during execution but not upstream-sync items, and an arithmetic-closed disposition-class tally — reusable by any future multi-plan UPST-sync phase's close-out gate"

requirements-completed: [SEC-01, SEC-08, SEC-09, RES-01, RES-02]  # SEC-03..07 already marked complete by their own single-contributing plans (112-02/03/05/06/07) earlier in this phase; this plan's contribution is the remaining 5 plus the SEC-02 carve-out reconciliation (not a requirement completion — SEC-02 is explicitly NOT marked complete).

# Metrics
duration: ~2h20min
completed: 2026-08-05
---

# Phase 112 Plan 08: Close-Out — Ledger Addendum, SEC-09 Carry-Forward, Requirements/Roadmap Reconciliation Summary

**Appended the D-05 standing-divergence addendum recording all 18 security-residual-and-misc SHA dispositions as they actually shipped (correcting 3 places where execution diverged from the pre-execution disposition table), filed the SEC-09 v3.7 carry-forward obligation directly in the ledger's tool-sandbox-surface work-list, reconciled REQUIREMENTS.md/ROADMAP.md (10 requirements resolved, SEC-02 explicitly carved out to Phase 114), and confirmed the combined Wave 2+3 surface is still green via a zero-crates-diff evidence chain plus a fresh cargo fmt + full workspace test sweep.**

## Performance

- **Duration:** ~2h20min (dominated by the ~65min full `cargo test --workspace --no-fail-fast` sweep run in the background while Task 3's edits were authored in parallel)
- **Started:** 2026-08-05 (session start)
- **Completed:** 2026-08-05
- **Tasks:** 3/3 completed
- **Files modified:** 3 (all documentation/planning artifacts; zero files under `crates/`)

## Accomplishments

- Appended `## Phase 112 Security + Residual Sync Addendum` to `108-DIVERGENCE-LEDGER.md`, immediately after the existing Phase 111 Standing Divergence Addendum, with an 18-row "table's planned disposition" vs. "disposition as shipped" comparison built from every one of the 7 prior plans' SUMMARYs (not re-copied from `112-DISPOSITION-TABLE.md`'s pre-execution text). Arithmetic closes at 10 named anchors + 1 SEC-09 + 4 RES-01 + 3 RES-02 = 18, matching D-08. The "Ledger closed" declaration (~line 1675) is textually unmodified — confirmed by `git diff --stat` showing zero deletions across both ledger edits (156 insertions, 0 deletions).
- Filed the "SEC-09 Carry-Forward Note (Phase 112, D-01)" subsection directly after the tool-sandbox-surface cluster commits (20) table, stating the obligation explicitly: a future v3.7 planner absorbing the base `tool_sandbox_runtime`/`command_policies` subsystem must consciously decide whether to bring `f6f02751`'s relaxed non-shim-entry guard along with it, rather than silently inheriting upstream's relaxed posture.
- Recorded 3 genuine divergences between `112-DISPOSITION-TABLE.md`'s pre-execution call and what shipped (SEC-03 adopt->adapt, RES-02/`4cc0af2c52` adapt-or-skip->skip, RES-02/`9840a16f35` adopt->adopt-adapted), plus the methodological lesson that the table's confidence column was file-presence-derived and wrong in 2 of 3 symbol-level re-checks — and a fork-internal security finding from SEC-07 (the `nono-proxy` `--no-auth`/CONNECT-auth-bypass gap SEC-07 closed) that is new information, not an upstream-sync item.
- Reconciled `REQUIREMENTS.md`: flipped `SEC-01`, `SEC-08`, `SEC-09`, `RES-01`, `RES-02` to `[x]` with RESOLVED notes citing their evidence trail (`SEC-03..07` were already `[x]`, flipped by their own single-contributing plans' standard `requirements.mark-complete` step earlier this phase). `SEC-02` deliberately left `[ ]` with an explicit "-> CARVED OUT to Phase 114 (2026-08-05, ROADMAP Amendment)" note, mirroring the `NET-02` -> Phase 113 precedent. Traceability table updated: all 10 Phase-112-owned rows show a non-Pending status; `SEC-02`'s row now points at Phase 114.
- Confirmed (verification-only, not rewritten) that `ROADMAP.md`'s Phase 112 Amendment paragraph (2026-08-05) and the `### Phase 114: OAuth Capture Absorb (SEC-02)` section are both present and consistent with this plan's own addendum and `112-OAUTH-CAPTURE-DISPOSITION.md` — no discrepancy found, so neither was rewritten. Flipped `ROADMAP.md`'s Phase 112 top-level checklist box, the `112-08-PLAN.md` line item, and added the missing Phase 112 row to the Progress table (`roadmap.update-plan-progress` returned `updated:true` but did not itself add the row — added by hand).
- Ran the combined-surface verification Task 2 asks for, scoped to what a zero-crates-diff close-out plan can meaningfully re-verify: confirmed via `git log ad8132e7..HEAD` that zero `crates/` commits landed since `112-07`'s own both-gates-GREEN, `cargo build --workspace --all-targets`-clean confirmation; ran `cargo fmt --all -- --check` fresh (clean, exit 0); ran the full `cargo test --workspace --no-fail-fast` sweep fresh (exit 101, expected) and diffed its 17-name failing set against the inherited 27-name baseline — byte-for-byte identical to `111-04-VERIFICATION-NOTES.md`'s own 17-name run, a confirmed strict subset with zero new names.

## Task Commits

Each task was committed atomically:

1. **Task 1: D-05 ledger addendum + D-01 SEC-09 carry-forward note** - `e32f0192` (docs)
2. **Task 2: Combined-surface fork-invariant verification pass** - no commit (pure verification task, zero files to modify — matches the `112-05` Task 2 precedent of "no code changes, no commit needed")
3. **Task 3: Reconcile REQUIREMENTS.md and ROADMAP.md** - `a3d69d48` (docs)

_No plan-metadata-only commit was created separately — this SUMMARY's own commit serves as the final metadata commit per the workflow's `final_commit` step._

## Files Created/Modified

- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` - SEC-09 Carry-Forward Note (inserted after the tool-sandbox-surface cluster table) + Phase 112 Security + Residual Sync Addendum (appended after the Phase 111 addendum); append-only, "Ledger closed" line unmodified
- `.planning/REQUIREMENTS.md` - 5 checkbox flips (`SEC-01`, `SEC-08`, `SEC-09`, `RES-01`, `RES-02`) with RESOLVED notes; `SEC-02` line updated with explicit carve-out note; Traceability table updated for all 11 Phase-112/114-related rows
- `.planning/ROADMAP.md` - Phase 112 checklist box + `112-08-PLAN.md` line flipped to complete; Progress table row for Phase 112 added

## Decisions Made

See `key-decisions` in frontmatter for the full list. Summary: this plan's central job was recording executed reality over pre-execution plan text — three genuine disposition divergences (SEC-03, RES-02/`4cc0af2c52`, RES-02/`9840a16f35`) are now the ledger's authoritative record, `112-DISPOSITION-TABLE.md` is explicitly superseded for those three rows, and a methodological lesson (file-presence confidence is not symbol-level confidence) is recorded for future UPST-sync planning. SEC-02 was deliberately kept at "carved out," never treated as this plan's decision to make.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - blocking-issue-adjacent, scope correction] Task 3's acceptance criterion "`grep -c "^\- \[x\]"` increases by exactly 10" does not match the file's actual pre-plan state**
- **Found during:** Task 3's `read_first` step, before any edit.
- **Issue:** The plan's acceptance criterion assumes all 10 Phase-112-owned requirements (`SEC-01`, `SEC-03..SEC-09`, `RES-01`, `RES-02`) were still unchecked at the start of this plan. Live inspection found `SEC-03`, `SEC-04`, `SEC-05`, `SEC-06`, `SEC-07` were already `[x]` — each flipped by its own single-contributing absorb plan (`112-02`/`03`/`05`/`06`/`07`) during the standard per-plan `requirements.mark-complete` step earlier this phase, which is correct behavior for a requirement resolved entirely within one plan (unlike a split multi-plan requirement, which this project's own memory notes explicitly warn against prematurely flipping). This plan's actual delta is 5 new flips (`SEC-01`, `SEC-08`, `SEC-09`, `RES-01`, `RES-02`), bringing the Phase-112-owned total to the full 10 the plan describes — the *end state* the acceptance criterion is checking for is correct; only its stated *delta* (10, vs. the actual 5) was stale.
- **Fix:** Verified the end state directly (all 10 Phase-112-owned requirement IDs, plus `SEC-02` explicitly excluded, in the Traceability table and checkbox list) rather than mechanically chasing a delta count that would have required either double-checking already-checked boxes (a no-op) or misreporting a false discrepancy in this SUMMARY.
- **Files modified:** none beyond the planned `REQUIREMENTS.md` edit.
- **Verification:** `grep -n "^| SEC-\|^| RES-" .planning/REQUIREMENTS.md` confirms all 10 Phase-112-owned rows show non-Pending status and `SEC-02` points at Phase 114.
- **Committed in:** `a3d69d48`

**2. [Rule 3 - blocking-issue, tool gap] `roadmap.update-plan-progress` did not add a Progress-table row for Phase 112**
- **Found during:** Task 3, after invoking the SDK verb per the workflow's `state_updates` guidance.
- **Issue:** `gsd-sdk query roadmap.update-plan-progress "112"` returned `{"updated": true, "phase": "112", "plan_count": 8, "summary_count": 7, "status": "In Progress", "complete": false}`, but `git diff --stat -- .planning/ROADMAP.md` afterward showed only my own prior manual checkbox edits — no new Progress-table row was inserted. `grep -n "^| 11" .planning/ROADMAP.md` confirmed Phases 108-111 all have rows but 112 did not.
- **Fix:** Added the Phase 112 Progress-table row by hand, matching the existing row format (`| 112. Security + Residual Sync | v3.6 | 8/8 | Complete (SEC-02 carved out to Phase 114) | 2026-08-05 |`).
- **Files modified:** `.planning/ROADMAP.md`
- **Verification:** `grep -n "^| 11" .planning/ROADMAP.md` now shows both 111 and 112 rows.
- **Committed in:** `a3d69d48`

---

**Total deviations:** 2 (both Rule 3-class: a stale plan-text delta and an SDK-verb gap, neither a security or correctness issue — both scope-verification corrections)
**Impact on plan:** No change to WHAT was delivered — every `must_haves` truth in the plan frontmatter is satisfied. Both deviations are documentation-accuracy corrections discovered before any edit was made incorrectly.

## Issues Encountered

None blocking. The `cargo test --workspace --no-fail-fast` sweep took ~65 minutes wall-clock (dominated by `env_vars.rs`'s live-process Windows-run tests, matching `111-04`'s own timing characterization) — run in the background via `run_in_background: true` while Task 3's REQUIREMENTS.md/ROADMAP.md edits were authored in parallel, so it did not block overall plan progress.

## User Setup Required

None - no external service configuration required. This is a documentation-only plan (zero files under `crates/` touched, no package installs, no code changes).

## Verification

**Mandatory cross-target clippy gate applicability (per this plan's `<mandatory_project_gate>`):**
`git diff --name-only` across all of this plan's commits (`e32f0192`, `a3d69d48`) touches zero files under `crates/`. Per the gate instruction, the cross-target clippy gates are therefore **not required** for this plan's own changes. Independently, `git log ad8132e7..HEAD -- crates/` (where `ad8132e7` is `112-07`'s own last commit, which confirmed both cross-target clippy gates GREEN with `--all-targets` plus `cargo build --workspace --all-targets` clean, after all of Wave 2 and Wave 3's code had already landed sequentially on this single branch with no worktree merges) returns empty — confirming the current HEAD's `crates/` state is byte-identical to what `112-07` already verified GREEN. No fresh multi-hour Docker/zig gate re-run was performed; the evidence chain above satisfies Task 2's "one final time over the FULL combined Wave 2+3 surface" intent without re-running gates against unchanged code.

**`cargo fmt --all -- --check`:** exit 0, clean, run fresh this session.

**`cargo test --workspace --no-fail-fast`:** exit 101 (expected — matches `111-04`'s own documented finding that this command does not exit 0 on this host). Full failing-test-name set (17 names, `2960 passed; 17 failed` aggregate across 53 test binaries):

```
audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty
audit_verify_reports_signed_attestation_with_pinned_public_key
config::tests::nono_home_dir_falls_through_when_unset
config::tests::nono_home_dir_rejects_non_absolute_override
config::tests::nono_home_dir_returns_override_when_set
config::tests::test_validated_home_falls_back_to_userprofile
config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists
config::tests::user_state_dir_uses_localappdata_on_windows
cr_01_no_format_macro_in_post_fork_child_branch
profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name
protected_paths::tests::blocks_child_directory_capability
protected_paths::tests::blocks_parent_directory_capability
protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root
rollback_signed_session_verifies_from_audit_dir_bundle
windows_run_allow_all_network_probe_connects
windows_run_blocks_live_block_net_without_enforcement
windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified
```

This 17-name set is **byte-for-byte identical** to `111-04-VERIFICATION-NOTES.md`'s own 17-name run (same names, same count) — a confirmed strict subset of the documented 27-name baseline (24-name `111-04` baseline + the 3-name `profile_cmd.rs` shared-fixture-flake addendum), with **zero new failure names**. The 3 addendum names (`test_init_creates_valid_profile`, `test_init_rejects_existing_file_without_force`, `test_schema_output_to_file`) did not appear this run either — consistent with `111-04`'s own documented run-to-run instability characterization for that shared-fixture flake, not a regression.

## Next Phase Readiness

- Phase 112 is closed: all 8 plans complete, `SEC-01`/`SEC-03..SEC-09`/`RES-01`/`RES-02` (10 requirements) resolved with cited evidence, `SEC-02` explicitly carved out to Phase 114 with a full evidence trail (`112-OAUTH-CAPTURE-DISPOSITION.md`, this plan's ledger addendum, `ROADMAP.md`'s Amendment + Phase 114 section).
- Phase 114 (OAuth Capture Absorb) can proceed directly from `112-OAUTH-CAPTURE-DISPOSITION.md` and this plan's ledger addendum row — no re-derivation needed.
- A future v3.7 (Windows Tool-Sandbox Parity) planner will find the SEC-09 carry-forward obligation directly in `108-DIVERGENCE-LEDGER.md`'s tool-sandbox-surface cluster section, not buried in a phase-specific addendum only.
- A future absorb plan revisiting `fa21a004`/`8a4237f2` (#1283, the 21-file `LinuxSandboxPolicy`/`SeccompPolicy` CLI-policy-selection refactor) will find it flagged in both this plan's addendum and `112-02-SUMMARY.md` as a standing, unabsorbed gap that SEC-03's adapt depends on but did not close.
- No blockers for milestone close-out.

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*

## Self-Check: PASSED

Verified files exist:
- FOUND: `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` (contains "Phase 112 Security + Residual Sync Addendum" and "SEC-09 Carry-Forward Note")
- FOUND: `.planning/REQUIREMENTS.md` (10 Phase-112-owned rows non-Pending, SEC-02 -> Phase 114)
- FOUND: `.planning/ROADMAP.md` (Phase 112 checklist/plan-line/Progress-row all flipped to complete)

Verified commits exist:
- FOUND: `e32f0192` (Task 1)
- FOUND: `a3d69d48` (Task 3)

Working tree clean confirmed via `git status --short` immediately before this SUMMARY was written (only untracked `112-08-SUMMARY.md` itself, which this commit adds).
