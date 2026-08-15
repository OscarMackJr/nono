---
slug: ci-windows-117-failures
status: fix_applied
fix_applied: true
awaiting: >
  CI confirmation. Local green is NECESSARY BUT NOT SUFFICIENT — clusters A and C
  target conditions this NON-elevated dev host structurally cannot reproduce, so
  the fix stays UNPROVEN until the `windows-layer-fault-injection` job runs on
  `milestone/v2.13-carryforward-closeout`.
goal: find_root_cause_only
created: 2026-08-15
updated: 2026-08-15
trigger: >
  Triage 19 Phase 117 test failures that appear only on the GitHub Actions
  windows-latest CI runner, and decide for each whether it is a CI-environment
  artifact, a genuine product defect, or a test defect.
---

# Debug: 19 Phase 117 test failures on the windows-latest CI runner

## Symptoms

**Expected:** `cargo test -p nono-sandbox-cli --features layer-fault-injection -- --test-threads=1`
passes on CI, as it substantially does on the dev host (documented Windows-host baseline:
1688 passed / 12 failed, where the 12 are 6 `config::tests` env races, 3 `protected_paths::tests`,
`profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`,
`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`,
and the host-blocked WR-20 pin).

**Actual:** On CI the same command reports `test result: FAILED. 1684 passed; 22 failed; 2 ignored`.
Only 3 of the 22 (`protected_paths::tests::*`) overlap the documented baseline. The other 19 are new
and cluster in Phase 117's own machinery. Notably the dev-host baseline failures that are NOT
`protected_paths` all PASSED on CI — the 6 `config::tests` env races passed under `--test-threads=1`,
and the WR-20 pin `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` PASSED
(the elevated runner can construct its precondition).

**Errors captured so far:**

- `inherit_only_residue_is_not_treated_as_already_covered` —
  "an INHERIT_ONLY_ACE mandatory-label ACE, even with a matching mask, must never be treated as
  already-covered residue (CR-01)". This is CR-01's own regression test, a BLOCKER fix from review
  iteration 4.
- `labels_guard_mixed_owned_and_non_owned_reports_applied_not_partially_applied` —
  `assertion left == right failed: the owned path must be labelled:
  LabelCoverage { policy_paths: 2, applied: 0, skipped_pre_exis...`, left: 0, right: 1.
  ZERO labels applied where one was required.

**Timeline:** First observed 2026-08-15 in CI run 31888431118 — which is the FIRST CI run ever
executed against this branch's Phase 115/116/117 work. The branch was 687 commits ahead of origin
and origin's tip (2026-07-29) predates the entire phase. So "when did this start" is unanswerable
from CI history: there is no prior green CI run to regress from. These tests have only ever been
observed on the dev host until now.

**Reproduction:** `gh run view --job 95021022031 --log` (run 31888431118, job
`windows-layer-fault-injection`, branch `milestone/v2.13-carryforward-closeout`).
A fresh run can be triggered with `gh workflow run "CI" --ref milestone/v2.13-carryforward-closeout`.
NOT reproducible on this dev host — these tests pass here.

## The 22 failures

| Group | Count | Tests |
|---|---|---|
| `exec_strategy::labels_guard::tests` | 9 | coverage_distinguishes_full_partial_and_zero_ace_launches · cr_06_remediation_command_actually_clears_the_condition · guard_apply_then_drop_reverts_label_for_fresh_file · guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label · inherit_only_residue_is_not_treated_as_already_covered · labels_guard_mixed_owned_and_non_owned_reports_applied_not_partially_applied · mismatched_prior_mask_still_records_a_coverage_gap · residue_is_not_reverted_on_drop · stale_residue_from_an_abnormal_exit_does_not_self_lock_out_the_next_launch |
| `exec_strategy::dacl_guard::tests` | 2 | ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop · writable_rule_applies_sid_ace_and_reverts_on_drop |
| `exec_strategy` other | 2 | launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file · rb3_gate_tests::workspace_owned_by_current_user_passes_write_owner_check |
| `hook_runtime_windows::tests` | 3 | test_cr02_timeout_hook_exits_cleanly · test_execute_before_hook_powershell_does_not_clr_fail · test_validate_rejects_world_writable_parent |
| `policy::tests` | 3 | test_resolve_read_group · test_validate_deny_overlaps_detects_conflict · test_validate_deny_overlaps_no_false_positive |
| `protected_paths::tests` (pre-existing baseline) | 3 | blocks_child_directory_capability · blocks_parent_directory_capability · requested_path_blocks_nonexistent_child_under_protected_root |

## Leading hypothesis — UNPROVEN, must be tested not assumed

GitHub's `windows-latest` runner executes as `runneradmin`, a member of the local Administrators
group with an elevated token. These tests were authored against this dev host's NON-elevated
`TWGGLOBAL\OMack` domain account. Ownership, `WRITE_OWNER`, mandatory-label and DACL semantics all
differ between those two principals, so tests asserting on ownership/labelling outcomes may be
pinning host-specific behaviour rather than product behaviour.

**The direction cuts both ways and is suspicious.** `applied: 0` means the guard applied FEWER
labels running as an admin, which is not the naive expectation and needs a real mechanism, not a
hand-wave. A satisfying answer explains WHY elevation reduces applied coverage.

**Do not assume this explains all 19.** `policy::tests` and `hook_runtime_windows::tests` look
unrelated to ownership semantics and plausibly have separate causes. Prove any cluster before
claiming it.

## Why this matters

Several of these are Phase 117's own security regression pins:
- `inherit_only_residue_is_not_treated_as_already_covered` pins **CR-01**, a BLOCKER — an
  `INHERIT_ONLY_ACE` (structurally inert) being counted as effective mandatory-label coverage.
- `residue_is_not_reverted_on_drop` pins **WR-02**.

If they fail for environment reasons, those pins are weaker than believed on the only Windows
runner that executes them (`windows-layer-fault-injection` is the sole Windows Rust-unit-test job;
the main `test` matrix is `[ubuntu-latest, macos-latest]`). If they fail for product reasons, there
is a real defect in the mandatory-label / DACL path. Both outcomes are consequential, and the
distinction matters more than any fix.

## Deliverable

A PER-TEST disposition — `ci-environment-artifact` | `product-defect` | `test-defect` — each with
evidence. Not a blanket verdict. Group only where a single root cause genuinely explains a cluster,
and prove the cluster rather than assuming it. Where an answer requires an elevated Windows session,
say so explicitly rather than guessing.

## Constraints

- **Diagnosis only.** Do NOT change production code or tests without checking in first.
- Do NOT `git push`.
- Do NOT touch `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md`.
  A parallel milestone (v3.5) is open.
- Any commit needs an explicit trailer `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`
  (`git commit -s` stamps the wrong name on this host).

## Host capabilities and limits

- This dev host is a NON-elevated `TWGGLOBAL\OMack` domain account with no
  `SeTakeOwnershipPrivilege` / `SeRestorePrivilege` / `SeBackupPrivilege` (confirm via `whoami /priv`).
  It CANNOT reproduce the elevated condition.
- `gh` CLI is available and authenticated — CI logs are readable, and
  `gh workflow run "CI" --ref milestone/v2.13-carryforward-closeout` can trigger a fresh run.
- The full CI log for the failing job is retrievable via `gh run view --job 95021022031 --log`
  (~3565 lines). Read the actual assertion messages for all 19 rather than reasoning from names.

## Current Focus

- status: root cause found for all 19; three independent causes, all proven
- hypothesis: >
  THREE separate root causes, each proven by direct observation:
  (A) 15 tests — the `windows-latest` runner executes ELEVATED, and objects it creates are
      owned by `BUILTIN\Administrators`, not by the token's user SID (`runneradmin`).
      `nono::path_is_owned_by_current_user` (crates/nono/src/sandbox/windows.rs:1256-1440)
      compares the NTFS owner SID to `TokenUser` via `EqualSid` — so every freshly-created
      test fixture reports NOT-owned, and every ownership-gated code path short-circuits.
      This is why elevation produces FEWER applied labels: it is a fail-safe SKIP, not a grant.
  (B) 3 `policy::tests` — hardcoded Unix path `/tmp`. On Windows this resolves to `C:\tmp`,
      which happens to exist on the dev host (a stale scratch dir, 45 files, oldest May 11)
      and does not exist on CI. Nothing to do with elevation.
  (C) 1 broker test — the test invokes the real broker with `--no-pty` but WITHOUT
      `--app-container-name`, an argument combination the broker has rejected fail-closed
      since commit cb341165 (2026-06-02). It only passes on the dev host because the test's
      two-candidate artifact lookup prefers a STALE broker binary dated 2026-06-01.
- test: >
  (A) Within-CI-run controlled comparison: in `dacl_guard::tests`, the 5 tests that call
      `take_ownership_for_current_user()` all PASSED; the 2 that do not both FAILED.
      In `labels_guard::tests`, all 9 tests that require `owned == true` FAILED and all 5 that
      do not require ownership PASSED. Zero exceptions in either module.
  (B) `C:\tmp` confirmed present on the dev host; `PathNotFound("/tmp")` on CI.
  (C) Ran BOTH local broker binaries with the test's exact argument shape.
- expecting: >
  (A) A 2-vs-5 split inside one module with one controlled difference. OBSERVED.
  (B) A stray `C:\tmp`. OBSERVED (45 files, dates May–Aug).
  (C) Stale broker parses; fresh broker exits 2 with the app-container message. OBSERVED.
- next_action: >
  CHECKPOINT — remediation requires test changes, which the operator must authorise.
  No further investigation needed.
- reasoning_checkpoint:
    hypothesis: >
      Three independent causes as above. The dominant one (A) is that the elevated CI token's
      created-object owner is the Administrators group, not the token user, so
      `path_is_owned_by_current_user` returns Ok(false) for every test fixture.
    confirming_evidence:
      - "crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:763-769 documents this exact
         mechanism verbatim in the codebase's own test-helper doc comment, written by a prior
         Phase 117 author: 'In an ELEVATED session, freshly-created tempdirs are owned by
         BUILTIN\\Administrators (not the user), which would make the ownership check return
         false, stop the walk immediately, and leave applied empty'."
      - "Controlled within-run comparison: dacl_guard 5 helper-callers PASS / 2 non-callers FAIL;
         labels_guard 9 ownership-requiring FAIL / 5 ownership-agnostic PASS. No exceptions."
      - "CI log: rb3_gate_tests::workspace_owned_by_current_user_passes_write_owner_check FAILED
         on a tempdir the test itself had just created — direct proof owner != TokenUser."
      - "CI log: the WR-20 pin non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap
         PASSED on CI while it is host-BLOCKED here for lack of SeRestorePrivilege — proving the
         CI token is elevated (holds SeRestorePrivilege)."
      - "Dev host: `dir /q` on a freshly created file shows owner TWGGLOBAL\\OMack == token user;
         `whoami /priv` confirms no SeTakeOwnership/SeRestore/SeBackup. The mirror image."
    falsification_test: >
      (A) would be refuted if any labels_guard/dacl_guard test that requires `owned == true`
      had PASSED on CI, or if any ownership-normalising test had FAILED. Neither occurred.
      It would also be refuted if `nono` lib test
      `path_is_owned_by_current_user_returns_true_for_tempfile` passed on a windows-latest
      runner — but no CI job runs `-p nono` tests on Windows (ci.yml:158 is the
      ubuntu/macos-only `test` matrix), so this discriminator is unavailable.
    fix_rationale: >
      No product fix is indicated. The product behaviour under (A) is correct and fail-safe:
      declining to lower a mandatory label on a path nono does not own is a REFUSAL TO GRANT,
      and the R-B3 gate (exec_strategy_windows/mod.rs, path_has_write_owner) hard-fails an
      elevated live launch rather than degrading. The remediation is test-side and already
      invented in this repo: call take_ownership_for_current_user (dacl_guard.rs:770) in the
      ownership-dependent tests, exactly as the 5 passing dacl_guard tests do.
      (B) and (C) are genuine test defects requiring test edits.
    blind_spots: >
      - Not directly observed that the CI created-object owner is specifically
        BUILTIN\\Administrators (S-1-5-32-544); what IS observed is `owner != TokenUser`.
        The specific principal is corroborated by the repo's own dacl_guard.rs:765-767 comment
        and by public reports of the same behaviour on GitHub Windows runners, but no CI log
        line prints the SID. This refinement does not change any disposition.
      - The 15 (A)-cluster tests were not re-run on an elevated Windows session by me; the host
        cannot produce one. The within-run 5-vs-2 / 5-vs-9 splits substitute for that.
      - The broker's stderr in (C) goes to a test-created pipe that is never drained or printed,
        so the broker's actual error string is not in the CI log. It was reproduced locally
        instead, which is stronger.

## Evidence

- timestamp: 2026-08-15
  checked: "Full CI log for job 95021022031, verbatim panic text for all 22 failures"
  found: >
    Ownership language dominates. 15 of 19 non-baseline failures carry one of:
    `SkipNotOwned`, `SkipWritableNotOwned`, `skipped_not_owned: N`, `applied = []`,
    `Hook script not owned by current user`, `current user must be WRITE_OWNER of their own
    tempdir`. The remaining 4 split cleanly into `PathNotFound("/tmp")` x2 +
    `assertion failed: caps.has_fs()` x1, and `got exit_code=2` x1.
  implication: "Three failure MODES, not one. Confirms the brief's warning against a blanket verdict."

- timestamp: 2026-08-15
  checked: "crates/nono/src/sandbox/windows.rs:1256-1440 (path_is_owned_by_current_user)"
  found: >
    Reads OWNER_SECURITY_INFORMATION via GetNamedSecurityInfoW (line 1278), then compares it to
    `GetTokenInformation(TokenUser)` (line 1381) with `EqualSid` (line 1415).
    `path_has_write_owner` (line 1227) is a thin delegate to it.
  implication: >
    The predicate is owner-SID == token USER SID. It is NOT a group-membership or an effective-
    access check. Any principal whose created objects are owned by a GROUP fails it universally.

- timestamp: 2026-08-15
  checked: "crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs:763-791"
  found: >
    The repo already documents the mechanism verbatim: "In an ELEVATED session, freshly-created
    tempdirs are owned by BUILTIN\\Administrators (not the user), which would make the ownership
    check return false, stop the walk immediately, and leave `applied` empty — a session-elevation
    artifact, not a logic failure. Taking ownership keeps these ownership-dependent tests green
    whether or not the suite runs elevated." — and provides
    `fn take_ownership_for_current_user(path)` as the normalisation.
  implication: >
    This is the missing mechanism the brief demanded, already written down by a prior Phase 117
    author, and it explains the counterintuitive direction: elevation makes the owner a GROUP,
    which the user-SID equality test rejects, so the guard SKIPS. Elevation reduces `applied`.

- timestamp: 2026-08-15
  checked: "Which dacl_guard tests call take_ownership_for_current_user, cross-referenced to CI outcomes"
  found: >
    Call sites at dacl_guard.rs:1266, 1364, 1512, 1574, 1625-1626 →
    ancestor_guard_classification_matches_the_d37_table,
    ancestor_read_attributes_grants_owned_ancestors_and_reverts_on_drop,
    ancestor_read_attributes_snapshot_and_apply_fails_when_forced_unavailable,
    ancestor_read_attributes_dedups_shared_ancestor_across_targets,
    ancestor_read_attributes_multi_target_covers_each_chain_and_stops_at_root.
    ALL FIVE PASSED on CI. The two dacl_guard tests that do NOT call it —
    writable_rule_applies_sid_ace_and_reverts_on_drop (dacl_guard.rs:916) and
    ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop (dacl_guard.rs:1054) —
    BOTH FAILED. `ancestor_read_attributes_grants_owned_ancestors_and_reverts_on_drop` is the
    near-twin of the failing `ancestor_traverse_...`, differing essentially by the helper call.
  implication: >
    A controlled comparison inside a single module in a single CI run, with one deliberate
    difference. This PROVES the ownership mechanism; it is no longer a plausible story.

- timestamp: 2026-08-15
  checked: "All 14 labels_guard::tests outcomes on CI"
  found: >
    FAILED (9, all require owned==true): coverage_distinguishes_full_partial_and_zero_ace_launches,
    cr_06_remediation_command_actually_clears_the_condition,
    guard_apply_then_drop_reverts_label_for_fresh_file,
    guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label,
    inherit_only_residue_is_not_treated_as_already_covered,
    labels_guard_mixed_owned_and_non_owned_reports_applied_not_partially_applied,
    mismatched_prior_mask_still_records_a_coverage_gap, residue_is_not_reverted_on_drop,
    stale_residue_from_an_abnormal_exit_does_not_self_lock_out_the_next_launch.
    PASSED (5, none require owned==true): audit_flush_before_drop,
    guard_reverts_all_entries_if_mid_loop_apply_fails, guard_skips_path_not_owned_by_current_user,
    non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap (WR-20),
    snapshot_and_apply_fails_when_forced_unavailable.
  implication: >
    Perfect partition by the ownership predicate, zero exceptions. The two PASSING tests that
    assert the NEGATIVE (`SkipNotOwned` / non-owned+foreign-label) pass trivially on CI because
    the environment satisfies their precondition for free.

- timestamp: 2026-08-15
  checked: "labels_guard.rs:1242-1288 (WR-20 test setup) vs CI outcome"
  found: >
    WR-20 reassigns ownership away via `icacls /setowner`, and panics loudly (D-31) when the
    host lacks SeRestorePrivilege. It is host-BLOCKED on the dev host for exactly that reason —
    and it PASSED on CI.
  implication: "Independent proof the CI token holds SeRestorePrivilege, i.e. is elevated."

- timestamp: 2026-08-15
  checked: "Dev-host principal and created-object owner"
  found: >
    `whoami /priv` on this host lists only SeShutdown/SeChangeNotify/SeUndock/
    SeIncreaseWorkingSet/SeTimeZone — no SeTakeOwnership, SeRestore, or SeBackup.
    `dir /q` on a file just created in %TEMP% shows owner `TWGGLOBAL\OMack`, which equals the
    token user. Hence path_is_owned_by_current_user == true here for every fixture.
  implication: "The dev host is the exact mirror image of CI. Both observations are consistent."

- timestamp: 2026-08-15
  checked: "crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:292-316 (gate ordering)"
  found: >
    The ownership gate at line 292 runs BEFORE the prior-label inspection at line 318
    (this ordering is WR-01's deliberate reorder). On Ok(false) it pushes
    `AppliedLabel::SkipNotOwned` and `continue`s, never reaching the residue / INHERIT_ONLY_ACE
    logic at lines 335-339 or the mask-mismatch branch.
  implication: >
    On CI, CR-01's INHERIT_ONLY_ACE check (line 338) and WR-02's no-revert-residue rule
    (lines 422-424) are never reached. The product logic is PRESENT and unchanged — only the
    pins are dark on that runner.

- timestamp: 2026-08-15
  checked: "crates/nono-cli/src/hook_runtime_windows.rs:528-560 (validate_hook_script_windows)"
  found: >
    Step 4 at line 556 is `if !path_is_owned_by_current_user(&canonical)?` — the identical
    predicate. It runs after the absolute/canonical/is-file checks and BEFORE the D-10
    world-writable-parent check. CI outcomes match exactly: the two tests that stop before
    step 4 (rejects_non_file, rejects_relative) PASSED; the three that reach it FAILED.
  implication: >
    The hook cluster MERGES into cluster (A) with a file:line proof, not by resemblance.
    Consequence: on an elevated host the D-10 world-writable check is masked and untested.

- timestamp: 2026-08-15
  checked: "crates/nono-cli/src/policy.rs:1563, 1801, 2341, 2384 and the local filesystem"
  found: >
    All three policy tests depend on the literal Unix path `/tmp`.
    `sample_policy_json()` line 1563 sets `\"allow\": { \"read\": [\"/tmp\"] }`;
    test_resolve_read_group line 1800 carries the comment `// /tmp should exist on all platforms`
    — which is false on Windows. On Windows `/tmp` is a root-relative path resolving to
    `C:\tmp` on the current drive. `C:\tmp` EXISTS on this dev host: 45 files, oldest
    `34-04b-close-gates.txt` dated 2026-05-11, i.e. an accumulated scratch directory from
    prior sessions (it is even listed among this session's additional working directories).
    On CI it does not exist → `FsCapability::new_dir` → `PathNotFound("/tmp")`.
  implication: >
    Cluster (B) is entirely independent of elevation, exactly as the brief suspected. These
    tests have been passing on the dev host only by the accident of a stray scratch directory.

- timestamp: 2026-08-15
  checked: "crates/nono-cli/src/exec_strategy_windows/launch.rs:5923-5936 vs crates/nono-shell-broker/src/main.rs:234-245"
  found: >
    The test builds broker_args = --shell, --shell-arg /c, --shell-arg \"echo x > FIXTURE\",
    --no-pty, 3x --inherit-handle, --cwd. There is NO --app-container-name.
    Broker `parse_args` at main.rs:239 rejects that combination fail-closed:
    \"--no-pty requires --app-container-name (WFP per-session enforcement); refusing to spawn a
    non-AppContainer (unmatched WFP) child\", and main.rs:1975 maps the Err to
    `std::process::exit(2)` — precisely the exit code observed.
  implication: "Deterministic, not environmental. The test's argument set is stale."

- timestamp: 2026-08-15
  checked: "Local differential execution of both on-disk broker binaries with the test's arg shape"
  found: >
    `target/x86_64-pc-windows-msvc/release/nono-shell-broker.exe` (dated 2026-06-01 23:26)
      → proceeds past parse and reaches CreateProcessAsUserW (GetLastError=87, from my dummy
        handle values) — i.e. NO app-container gate.
    `target/release/nono-shell-broker.exe` (dated 2026-08-04 12:28)
      → exit 2 with \"--no-pty requires --app-container-name\".
    The gate was introduced by commit cb341165 (2026-06-02 21:42, feat(62-12)), i.e. ONE DAY
    after the stale artifact was built. The test's candidate lookup (launch.rs:5855-5858)
    prefers the `x86_64-pc-windows-msvc` path, so the dev host has been testing the stale binary.
    CI has no `x86_64-pc-windows-msvc` artifact and freshly builds the default-target one
    (ci.yml:386-387, added by Plan 117-18 NR-08) — so CI is the first environment to run this
    test against a current broker.
  implication: >
    The REQ-WSRH-03 / D-07 real-spawn NO_WRITE_UP proof has not executed against a current
    broker since 2026-06-02. The test's own non-vacuity gate (launch.rs:6035, exit != 0 && != 2)
    is what caught it — that gate did its job.

- timestamp: 2026-08-15
  checked: "crates/nono-cli/src/exec_strategy_windows/launch.rs:2384-2397 (production BrokerLaunchNoPty arm)"
  found: >
    The PRODUCTION no-pty arm resolves `config.app_container_name` with `ok_or_else` (fail-closed,
    line 2389) and pushes `--app-container-name` (lines 2396-2397).
  implication: >
    No product defect. The broken argument set exists ONLY in the hand-rolled test harness at
    launch.rs:5923-5936. Cluster (C) is squarely a test-defect.

- timestamp: 2026-08-15
  checked: "CI job conclusion vs test result"
  found: >
    Job 95021022031 reported `conclusion: success` while its log contains
    `test result: FAILED. 1684 passed; 22 failed`. The run's log group (log line ~829) shows BOTH
    cargo commands inside a SINGLE pwsh `run:` block, so the step status was the exit code of the
    last native command. The working tree's ci.yml:389-408 already splits them into two steps and
    documents this exact run in its comment.
  implication: >
    The remediation is already in the tree but post-dates run 31888431118. Until it lands on a
    branch that CI runs, these 22 failures are non-blocking — which is why they went unnoticed.

- timestamp: 2026-08-15
  checked: >
    Targeted local run on this NON-elevated dev host:
    `cargo test -p nono-sandbox-cli --features layer-fault-injection -- --test-threads=1
     protected_paths::tests exec_strategy::dacl_guard::tests::writable_rule_applies_sid_ace_and_reverts_on_drop`
  found: >
    `writable_rule_applies_sid_ace_and_reverts_on_drop` ... **ok** locally (it FAILED on CI with
    `SkipWritableNotOwned`) — the cluster-(A) local/CI differential, executed rather than assumed.
    The three `protected_paths::tests` failed locally with byte-identical panics to CI:
    `blocked: ()` at protected_paths.rs:309:68, :289:78 and :342:10 respectively.
    Total test count matches CI exactly (7 run + 1701 filtered = 1708 = CI's 1684+22+2).
  implication: >
    (i) The protected_paths trio is confirmed pre-existing baseline by MESSAGE and LINE, not just
    by name — correctly excluded from the 19. (ii) Cluster (A) is confirmed environment-dependent
    by direct execution on both principals.

## Eliminated

- hypothesis: "A single root cause explains all 19 failures."
  evidence: >
    Three disjoint failure modes with three separately proven mechanisms: ownership predicate
    (15), missing `C:\tmp` (3), stale broker argument contract (1). The `/tmp` tests reference no
    ownership API and the broker test fails before any ownership code runs.
  timestamp: 2026-08-15

- hypothesis: "The elevated runner grants MORE access, so tests should over-apply, not under-apply."
  evidence: >
    The predicate is SID EQUALITY against `TokenUser` (windows.rs:1415), not an effective-access
    or privilege check. Elevation changes the OWNER of created objects to a group SID, which the
    equality test rejects. Privilege is irrelevant to the predicate. Confirmed by the repo's own
    dacl_guard.rs:765-767 note and by the 5-pass/2-fail helper split.
  timestamp: 2026-08-15

- hypothesis: "policy::tests fail for the same elevation reason as the labels/dacl guards."
  evidence: >
    Verbatim failures are `PathNotFound(\"/tmp\")` and `assertion failed: caps.has_fs()`.
    Neither test touches any ownership API. `C:\tmp` confirmed present locally (45 files) and
    absent on CI. Purely a hardcoded-Unix-path defect.
  timestamp: 2026-08-15

- hypothesis: "The broker write-deny test fails because the elevated runner alters NO_WRITE_UP / MIC."
  evidence: >
    The broker exits 2 at ARGUMENT PARSE time (main.rs:239 → 1975), before any token or MIC work.
    Reproduced deterministically on this NON-elevated dev host with the fresh broker binary.
  timestamp: 2026-08-15

- hypothesis: "CR-01 or WR-02 has regressed in the product."
  evidence: >
    Both fixes are present and unmodified: the INHERIT_ONLY_ACE rejection at labels_guard.rs:336-339
    and the AlreadyAtRequiredLevel no-revert set at labels_guard.rs:422-424. Both pins PASS on the
    non-elevated dev host. On CI they short-circuit at the ownership gate (line 292) before
    reaching the pinned logic, so they neither pass nor fail on the merits — they are dark.
  timestamp: 2026-08-15

## Resolution

root_cause: >
  Three independent causes.
  (A) 15/19 — `nono::path_is_owned_by_current_user` (crates/nono/src/sandbox/windows.rs:1256,
      compare at :1415) tests owner-SID == token USER SID. The `windows-latest` runner is
      elevated, and objects it creates are owned by the Administrators group rather than by
      `runneradmin`, so every freshly-created test fixture reports NOT-owned and every
      ownership-gated path short-circuits (labels_guard.rs:292, dacl_guard.rs:224/406,
      hook_runtime_windows.rs:556, mod.rs path_has_write_owner). Elevation therefore REDUCES
      `applied` — the guard is refusing to grant, which is the fail-safe direction.
  (B) 3/19 — `policy::tests` hardcode the Unix path `/tmp`, which resolves to `C:\tmp` on
      Windows. `C:\tmp` exists on the dev host as a stale scratch directory and does not exist
      on CI.
  (C) 1/19 — the write-deny broker test invokes the broker with `--no-pty` and no
      `--app-container-name`; the broker has rejected that fail-closed since 2026-06-02. The
      test passes on the dev host only because its artifact lookup prefers a stale broker
      binary built 2026-06-01.

fix: >
  Applied 2026-08-15 as quick task 260815-gfd, test-code only — no production
  line in `crates/nono/src/` or `crates/nono-cli/src/` outside a `#[cfg(test)]`
  region was changed.

  (A) 15 tests — the `take_ownership_for_current_user` helper was PROMOTED out of
      `dacl_guard::tests` (where it was private, which is why only that module
      could use it) into a new crate-level test-only module
      `crates/nono-cli/src/test_ownership_windows.rs`, declared at main.rs:158-166
      under `#[cfg(test)] #[cfg(target_os = "windows")]`. All 15 ownership-gated
      tests now route through that ONE implementation, preserving the 5-pass/2-fail
      controlled comparison this investigation relied on. A second helper,
      `take_ownership_of_script_and_parent`, covers the hook cluster, whose
      validation gates on the script's owner and separately on the PARENT
      directory's DACL. The two tests that pin the NON-owned branch
      (`guard_skips_path_not_owned_by_current_user`, and the WR-20 pin
      `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`)
      deliberately do NOT call it — they construct the opposite precondition.

  (B) 3 `policy::tests` — the shared `sample_policy_json()` fixture keeps its
      `/tmp` literal (correct and portable for its 15 PARSE-only callers); a new
      `sample_policy_json_reading(dir)` repoints the read grants at a real
      directory for `test_resolve_read_group`, the only caller that asserts on a
      RESOLVED capability. The two `validate_deny_overlaps` tests now allow a
      `tempfile::tempdir()` and build their deny child from `cap.resolved` (the
      CANONICALIZED grant), which also fixes a latent macOS `/tmp` →
      `/private/tmp` mismatch. The false comment at policy.rs:1800 is gone.

  (C) 1 broker test — `--app-container-name` added at launch.rs:5930, minted with
      `generate_app_container_name()` and mirroring the production arm at
      launch.rs:2389-2397. That exposed a SECOND stale-harness defect (see below).
      The stdout/stderr pipes are now drained on background threads so a
      broker-internal failure surfaces its message instead of vanishing.

  (C2) NEWLY FOUND, not in the original diagnosis. With the arg-parse gate passed,
      the broker got further and then refused at a LATER fail-closed gate:
      "NONO_BROKER_REQUIRED_LAYERS absent or unreadable — refusing to resume an
      unattested child" (nono-shell-broker/src/main.rs:886, fail-closed since
      Phase 117 review CR-03.3). The harness passed a null `lpEnvironment`, so the
      broker inherited the test process's environment, which does not carry the
      wire-contract variable. Fixed by mirroring production (launch.rs:1786-1798):
      an explicit `CREATE_UNICODE_ENVIRONMENT` block built from
      `std::env::vars()` plus the pair derived from
      `required_layers_for_broker(all_entries(), BrokerLaunchNoPty)` — the same
      registry query production uses, so the value cannot drift from a hard-coded
      list. The test now PASSES against a broker built at current HEAD: the broker
      registers the AppContainer, spawns a real child, and the child exits 1
      (denied) with the fixture unmodified.

  Hygiene: the stale `target/x86_64-pc-windows-msvc/release/nono-shell-broker.exe`
  (2026-06-01) was deleted and `target/release/nono-shell-broker.exe` rebuilt at
  current HEAD.

verification: >
  On this NON-elevated dev host, `cargo test -p nono-sandbox-cli --features
  layer-fault-injection -- --test-threads=1`:
  **1694 passed / 12 failed / 2 ignored** (1708 total, matching the pre-fix total
  exactly). The 12 failures are the documented dev-host baseline, name for name:
  6 `config::tests` env races, 3 `protected_paths::tests`,
  `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`,
  `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`,
  and the host-blocked WR-20 pin. ZERO regressions; all 15 cluster-A tests, all 3
  cluster-B tests and the cluster-C test pass locally.

  `cargo clippy -p nono-sandbox-cli --all-targets -- -D warnings
  -D clippy::unwrap_used` clean, with and without `--features
  layer-fault-injection`. `cargo fmt --all -- --check` exit 0.

  Non-vacuity proof for cluster B: repointing `sample_policy_json_reading` at a
  NONEXISTENT directory makes `test_resolve_read_group` fail with the CI failure
  mode, confirming the test genuinely depends on path existence rather than
  passing for an unrelated reason. Reverted after the check.

  NOT run: the `tests/*.rs` integration binaries. Cargo stopped after the `--bin
  nono` unit binary reported its baseline failures; all 19 tests in scope live in
  that binary.

  **Local green is necessary but NOT sufficient.** Cluster A targets a condition
  (owner == BUILTIN\\Administrators) that this non-elevated host cannot construct
  — the same limitation that makes the WR-20 pin fail here. Locally, taking
  ownership of a path already owned is a no-op, so a green local run confirms only
  that the change is harmless here, never that it is effective there. The
  elevated-runner claim remains UNPROVEN until `windows-layer-fault-injection`
  runs. Note also that job 95021022031 reported `conclusion: success` despite 22
  failures; the ci.yml step split (commit de77b1fd) must be in effect for the next
  run to be readable as a verdict.

files_changed:
  - crates/nono-cli/src/test_ownership_windows.rs (new, test-only module)
  - crates/nono-cli/src/main.rs (test-only `mod` declaration)
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs (tests only)
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs (tests only)
  - crates/nono-cli/src/exec_strategy_windows/mod.rs (rb3_gate_tests only)
  - crates/nono-cli/src/exec_strategy_windows/launch.rs (write_deny_… tests only)
  - crates/nono-cli/src/hook_runtime_windows.rs (tests only)
  - crates/nono-cli/src/policy.rs (tests only)

## Disposition table (19 non-baseline failures)

| # | test | observed failure mode (verbatim-derived) | disposition | evidence | provisional? |
|---|---|---|---|---|---|
| 1 | `exec_strategy::dacl_guard::tests::ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop` | "the user-owned tempdir parent must be granted traverse; applied = []" | ci-environment-artifact | dacl_guard.rs:1071 assert; gate at dacl_guard.rs:406; twin test at :1364 calls `take_ownership_for_current_user` and PASSED | no |
| 2 | `exec_strategy::dacl_guard::tests::writable_rule_applies_sid_ace_and_reverts_on_drop` | "writable owned rule must record Applied; got SkipWritableNotOwned" | ci-environment-artifact | dacl_guard.rs:930 assert; gate at dacl_guard.rs:224→279; test body at :916-919 omits the helper | no |
| 3 | `exec_strategy::labels_guard::tests::coverage_distinguishes_full_partial_and_zero_ace_launches` | `LabelCoverage { policy_paths: 2, applied: 0, skipped_not_owned: 2 }`, left 0 right 2 | ci-environment-artifact | labels_guard.rs:685; fixture comment at :673 states the unmet assumption "fresh and owned" | no |
| 4 | `exec_strategy::labels_guard::tests::cr_06_remediation_command_actually_clears_the_condition` | "precondition: the foreign label must register as a coverage gap: … applied: 0, skipped_not_owned: 1" | ci-environment-artifact | labels_guard.rs:1552; gate at :292 | no |
| 5 | `exec_strategy::labels_guard::tests::guard_apply_then_drop_reverts_label_for_fresh_file` | "label must be present" (`.expect` on `low_integrity_label_and_mask`) | ci-environment-artifact | labels_guard.rs:648; no label applied because :292 skipped | no |
| 6 | `exec_strategy::labels_guard::tests::guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label` | "expected SkipPreExistingLabel, got SkipNotOwned" | ci-environment-artifact | labels_guard.rs:797; ownership gate :292 precedes prior-label inspection :318 | no |
| 7 | `exec_strategy::labels_guard::tests::inherit_only_residue_is_not_treated_as_already_covered` (**CR-01 pin**) | "an INHERIT_ONLY_ACE … must never be treated as already-covered residue (CR-01): got SkipNotOwned" | ci-environment-artifact — **pin DARK on CI** | labels_guard.rs:964; pinned product logic at :336-339 present and unchanged, never reached | no |
| 8 | `exec_strategy::labels_guard::tests::labels_guard_mixed_owned_and_non_owned_reports_applied_not_partially_applied` | "the owned path must be labelled: … applied: 0, skipped_not_owned: 2", left 0 right 1 | ci-environment-artifact | labels_guard.rs:1491; the "owned" half of the fixture is not owned on CI | no |
| 9 | `exec_strategy::labels_guard::tests::mismatched_prior_mask_still_records_a_coverage_gap` | "a mask mismatch must still record SkipPreExistingLabel …, got SkipNotOwned" | ci-environment-artifact | labels_guard.rs:1040; gate :292 precedes :318 | no |
| 10 | `exec_strategy::labels_guard::tests::residue_is_not_reverted_on_drop` (**WR-02 pin**) | "expected AlreadyAtRequiredLevel residue, got SkipNotOwned" | ci-environment-artifact — **pin DARK on CI** | labels_guard.rs:992; pinned product logic at :422-424 present and unchanged, never reached | no |
| 11 | `exec_strategy::labels_guard::tests::stale_residue_from_an_abnormal_exit_does_not_self_lock_out_the_next_launch` | "residue label must be present" | ci-environment-artifact | labels_guard.rs:833; setup label never applied because :292 skipped | no |
| 12 | `exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file` | "write-deny non-vacuous gate FAILED … got exit_code=2" | **test-defect** | test args launch.rs:5923-5936 omit `--app-container-name`; broker rejects at main.rs:239, exits 2 at main.rs:1975; reproduced locally on both binaries; production arm launch.rs:2389-2397 is correct | no |
| 13 | `exec_strategy::rb3_gate_tests::workspace_owned_by_current_user_passes_write_owner_check` | "current user must be WRITE_OWNER of their own tempdir (R-B3 gate PASS)" | ci-environment-artifact | mod.rs:1471; `path_has_write_owner` delegates to the same predicate, windows.rs:1227-1232 | no |
| 14 | `hook_runtime_windows::tests::test_cr02_timeout_hook_exits_cleanly` | "Configuration parse error: Hook script not owned by current user: \\\\?\\C:\\Users\\runneradmin\\…\\quick_hook.ps1" | ci-environment-artifact | hook_runtime_windows.rs:1208 assert; gate at hook_runtime_windows.rs:556 | no |
| 15 | `hook_runtime_windows::tests::test_execute_before_hook_powershell_does_not_clr_fail` | "…returned a FUNCTIONAL failure…: Hook script not owned by current user: …test_hook.ps1" | ci-environment-artifact | hook_runtime_windows.rs:1134 assert; gate at :556. NOT the `-65536` or `code 1` signature the message warns about | no |
| 16 | `hook_runtime_windows::tests::test_validate_rejects_world_writable_parent` | "Error must mention world-writable or Everyone: … Hook script not owned by current user: …hook.ps1" | ci-environment-artifact | hook_runtime_windows.rs:985 assert; owner gate :556 fires before the D-10 world-writable check, masking it | no |
| 17 | `policy::tests::test_resolve_read_group` | `assertion failed: caps.has_fs()` | **test-defect** | policy.rs:1801 assert (comment at :1800 claims "/tmp should exist on all platforms"); fixture policy.rs:1563 grants read on `/tmp`; `C:\tmp` absent on CI | no |
| 18 | `policy::tests::test_validate_deny_overlaps_detects_conflict` | `/tmp must exist: PathNotFound("/tmp")` | **test-defect** | policy.rs:2341 `FsCapability::new_dir(Path::new("/tmp"))` | no |
| 19 | `policy::tests::test_validate_deny_overlaps_no_false_positive` | `/tmp must exist: PathNotFound("/tmp")` | **test-defect** | policy.rs:2384 same construction | no |

Baseline (excluded from the 19, per brief): `protected_paths::tests::blocks_child_directory_capability`,
`::blocks_parent_directory_capability`, `::requested_path_blocks_nonexistent_child_under_protected_root`
— all three panic with `blocked: ()` at protected_paths.rs:309/289/342. Names match the documented
dev-host baseline exactly; set aside.

## Security-pin impact

- **CR-01** (`inherit_only_residue_is_not_treated_as_already_covered`) and **WR-02**
  (`residue_is_not_reverted_on_drop`) are **NOT weakened in the product.** The fixes are present
  and unchanged at labels_guard.rs:336-339 and labels_guard.rs:422-424, and both pins execute and
  pass on the non-elevated dev host.
- They ARE **dark on CI**: on `windows-latest` both short-circuit at the ownership gate
  (labels_guard.rs:292) before reaching the logic they pin. Since
  `windows-layer-fault-injection` is the only Windows Rust-unit-test job (ci.yml:158's `test`
  matrix is ubuntu+macos), CI currently provides ZERO automated regression coverage for CR-01 and
  WR-02. Compounding this, the job reported `conclusion: success` despite 22 failures.
- The **D-10 world-writable-parent** hook check is likewise masked on CI (owner gate fires first).
- **REQ-WSRH-03 / D-07** real-spawn NO_WRITE_UP proof has not run against a current broker since
  2026-06-02 anywhere, including the dev host.

## Proposed remediation (NOT applied — needs operator authorisation)

1. **Cluster A (15):** call `take_ownership_for_current_user` (dacl_guard.rs:770) on the fixture
   paths in the 11 labels_guard/dacl_guard tests and the rb3 gate test; for the 3 hook tests, do
   the same on the script + parent. This is the pattern already proven by the 5 dacl_guard tests
   that pass on CI. Promote the helper to a shared test util rather than duplicating it.
2. **Cluster B (3):** replace `/tmp` with `std::env::temp_dir()` (or a `tempfile::tempdir()`),
   and delete the false comment at policy.rs:1800.
3. **Cluster C (1):** add `--app-container-name` to the broker args at launch.rs:5930, matching
   the production arm at launch.rs:2396-2397. Separately consider draining the broker's stderr
   pipe in that test so a broker-internal failure surfaces its message.
4. **Hygiene:** the stale `target/x86_64-pc-windows-msvc/release/nono-shell-broker.exe`
   (2026-06-01) on this dev host should be deleted or rebuilt — it silently pinned an obsolete
   broker for 2.5 months.

*(All four applied 2026-08-15 as quick task 260815-gfd. See `fix:` and `verification:` above.)*

## Post-fix: does each cluster-A test now REACH ITS SUBJECT on an elevated runner?

Turning 15 tests green is the smaller half of the result. The larger half is that they
can now reach the logic they name on the only Windows job that runs them. This section is
a per-test judgement of **genuinely exercises its subject** vs **merely stops failing** —
the input to scoping the separate planned follow-up. It is a code-reading judgement, not a
measurement: nothing here was observed on an elevated session.

| # | test | after taking ownership | judgement |
|---|---|---|---|
| 1 | `labels_guard::guard_apply_then_drop_reverts_label_for_fresh_file` | gate passes → real `SetNamedSecurityInfoW(LABEL_)` apply + real Drop revert | **exercises** |
| 2 | `labels_guard::coverage_distinguishes_full_partial_and_zero_ace_launches` | all four sub-cases (Applied / PartiallyApplied / NotApplied / NotApplicable) driven by real ACEs | **exercises** — the non-vacuity proof for `LayerCoverage::application` |
| 3 | `labels_guard::guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label` | reaches the prior-label inspection at :318, so WR-03's `SkipPreExistingLabel` path is real | **exercises** |
| 4 | `labels_guard::stale_residue_from_an_abnormal_exit_does_not_self_lock_out_the_next_launch` | first apply writes a real ACE; second launch adopts it | **exercises** (NR3-01) |
| 5 | `labels_guard::inherit_only_residue_is_not_treated_as_already_covered` | reaches the `INHERIT_ONLY_ACE` rejection at :336-339 | **exercises — CR-01 BLOCKER pin recovered.** The single most valuable line in this table |
| 6 | `labels_guard::residue_is_not_reverted_on_drop` | reaches `AlreadyAtRequiredLevel` at :348 and the no-revert set at :422-424 | **exercises — WR-02 pin recovered** |
| 7 | `labels_guard::mismatched_prior_mask_still_records_a_coverage_gap` | reaches the wanted-mask comparison | **exercises** |
| 8 | `labels_guard::labels_guard_mixed_owned_and_non_owned_reports_applied_not_partially_applied` | owned half normalised; `C:\Windows` stays TrustedInstaller-owned so the exempt half is still exempt even elevated | **exercises** (WR-28's mixed denominator) |
| 9 | `labels_guard::cr_06_remediation_command_actually_clears_the_condition` | full broken → remedy → success loop runs | **exercises** |
| 10 | `dacl_guard::writable_rule_applies_sid_ace_and_reverts_on_drop` | real SID ACE written and revoked; `Applied` variant genuinely produced | **exercises** |
| 11 | `dacl_guard::ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop` | immediate parent granted; assertion is on that parent only | **exercises**, with a caveat: only the immediate parent is normalised, so the walk's DEPTH may differ between an elevated runner and this host. That is a scope difference, not vacuity — but a test that pinned walk depth would need more |
| 12 | `exec_strategy::rb3_gate_tests::workspace_owned_by_current_user_passes_write_owner_check` | asserts `path_has_write_owner == true` on a dir whose ownership the test itself just assigned | **WEAKEST OF THE 15 — flag for the follow-up.** Not circular (icacls `/setowner` vs `GetNamedSecurityInfoW`+`EqualSid` are different implementations), but the ORIGINAL intent — "a workspace a user naturally creates passes the R-B3 gate" — is precisely what is FALSE on an elevated runner, and normalising the fixture hides that. See the note below |
| 13 | `hook_runtime_windows::test_validate_rejects_world_writable_parent` | owner gate no longer fires first, so the D-10 world-writable check is reached | **exercises — second most valuable recovery.** D-10 is currently 100% dark on CI. Residual caveat: the test still early-`return`s if `grant_sid_write_on_path` fails, which an elevated runner should not hit |
| 14 | `hook_runtime_windows::test_execute_before_hook_powershell_does_not_clr_fail` | validation passes, so PowerShell is really spawned and `HOOK_OK=yes` really asserted | **exercises**. Residual caveat: the narrowly-allowed spawn-failure skip remains |
| 15 | `hook_runtime_windows::test_cr02_timeout_hook_exits_cleanly` | benign-exit-with-timeout path really runs | **exercises** |

**Score: 14 of 15 genuinely exercise their subject; 1 (#12) is near-tautological by
construction.**

### The #12 note, which is product-relevant

On an elevated runner a workspace the operator creates is owned by
`BUILTIN\Administrators`, so `path_has_write_owner` returns `Ok(false)` and the R-B3 gate
HARD-FAILS the launch. That is intended behaviour — the gate's own error text names this
exact condition ("created from an elevated console (owned by BUILTIN\\Administrators)")
and D-08 forbids auto-takeown. But note the consequence: after this fix, the CI job has a
test for the R-B3 PASS branch and **no test for the elevated-workspace FAIL branch**, which
is the branch an elevated operator actually meets. `system_dir_lacks_write_owner_for_
standard_user` does not close that gap — it is explicitly tolerant of either outcome. A
deliberate FAIL-branch pin is a candidate for the follow-up.

### Two tests deliberately left un-normalised

`guard_skips_path_not_owned_by_current_user` and the WR-20 pin
`non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` must never call the
helper: their subject IS the non-owned branch. WR-20 additionally remains host-blocked here
(it needs `SeRestorePrivilege` to give ownership away) and passes on CI for free — the
mirror image of the other 15. Preserving that asymmetry is what keeps the module's
pass/fail split readable as a control.

### Also newly dark, and NOT closed by this fix

`REQ-WSRH-03 / D-07`'s real-spawn NO_WRITE_UP proof now runs against a current broker for
the first time since 2026-06-02. Note the denial mechanism CHANGED in the process: the
child is now a per-run AppContainer with a Low-IL primary token (the production shape),
where before it was a Low-IL non-AppContainer child. The test asserts the write is denied,
not WHICH of the two mechanisms denied it. If the intent is specifically to pin MIC
`NO_WRITE_UP`, that discrimination is not currently made.
