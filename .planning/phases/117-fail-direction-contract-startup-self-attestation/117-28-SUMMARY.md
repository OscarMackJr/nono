---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 28
subsystem: windows-sandbox-attestation
tags: [windows, dacl, ancestor-guard, layer-registry, daemon, attestation, discovery-test, gap-closure]

# Dependency graph
requires:
  - phase: 117 (plans 17, 22, 23)
    provides: "Two-state DaemonAttestationDecision (Plan 17); daemon-side ancestor-walk skip
      semantics and the first cross-mirror variant-set test (Plan 22); walked:bool on both
      CLI-side ancestor guards, making application() genuinely reachable in a coverage-gap
      state (Plan 23)"
provides:
  - "D-37's three-arm classification (walked, empty-applied, stopped-at-non-owned) on
    AppliedAncestorTraverseGuard and AppliedAncestorReadAttributesGuard::application()"
  - "A single reconciled doc-comment rule across mod.rs's applied_layers() and
    dacl_guard.rs's struct docs, both citing D-37 by name"
  - "D37_CLASSIFICATION_TABLE + ancestor_guard_classification_matches_the_d37_table: the
    CLI-mirror half of the cross-mirror classification proof, driven through real
    snapshot_and_apply scenarios"
  - "daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned: the daemon-mirror
    half, a concrete behavioral assertion (not a table) proving DaemonDaclGuard::apply still
    returns Ok for the identical physical condition"
  - "daemon_enum_segment(): a shared anchor-resolution helper both DAEMON-DECISION-ENUM
    discovery tests now route through (closes WR-14)"
  - "// (non-doc) comment exclusion in parse_enum_variant_names's candidate_indent"
affects: [117-29, any future daemon/CLI attestation-decision-shape plan]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Classification-rule proofs across independently-implemented mirrors (CLI vs daemon)
      state their type asymmetry explicitly rather than faking a shared table"
    - "Discovery tests that include_str! their own file must count their OWN literal
      self-reference in any occurrence-count precondition, not just the real declaration"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/agent_daemon/launch.rs

key-decisions:
  - "D-37 (LOCKED, 117-CONTEXT.md): a walk that ran, granted nothing, and stopped at the
    first non-owned ancestor is the D-04 contract-exempt outcome (NotApplicable), not a
    downgrade. 117-22's daemon-side reading wins over 117-23's CLI-side PartiallyApplied
    reading for the identical physical condition."
  - "The CLI/daemon type asymmetry is stated explicitly in both the CLI table's own doc
    comment and the daemon test's doc comment: the daemon has no LayerApplication-typed
    outcome, only Result<DaemonDaclGuard>, so a shared table cannot be asserted against it
    as written — a behavioral Ok/Err assertion substitutes."
  - "%PUBLIC% substituted for the plan's original %SystemRoot%\\Temp candidate in the daemon
    behavioral test: %SystemRoot%\\Temp empirically denies READ_CONTROL to this host's
    unprivileged test principal (verified independently via icacls), so the ownership
    *query* itself fails before the D-37 condition can be established. %PUBLIC% satisfies
    the same 'owned by SYSTEM, writable by the interactive user' shape without that
    restriction on this host."

requirements-completed: [CINT-02, CINT-03]

# Metrics
duration: ~55min
completed: 2026-08-11
---

# Phase 117 Plan 28: D-37 Ancestor-Guard Three-Arm Classification + Cross-Mirror Proof Summary

**Both ancestor DACL guards now distinguish the D-04 contract-exempt non-owned-ancestor stop
(NotApplicable) from a real coverage gap (NotApplied), proven identical to the daemon's real
non-downgrading behavior via a CLI-side classification table and a daemon-side behavioral
assertion, with WR-14's first-textual-occurrence anchor fragility closed on both daemon
discovery tests.**

## Performance

- **Duration:** ~55 min (continuation session; Task 1 was already committed by the
  interrupted prior executor)
- **Tasks:** 3/3 complete (Task 1 pre-existing at session start; Tasks 2-3 completed and
  Task 1's perturbation proof produced this session)
- **Files modified:** 3 (dacl_guard.rs, mod.rs [Task 1 only, no further changes this
  session], agent_daemon/launch.rs)

## Accomplishments

- **Task 1** (pre-existing commit `25abcf92`): `stopped_at_non_owned: bool` added to both
  `AppliedAncestorTraverseGuard` and `AppliedAncestorReadAttributesGuard`; `application()`
  rewritten to the D-37 three-arm match; `mod.rs`'s self-contradicting comment reconciled
  with `dacl_guard.rs`'s struct doc, both citing D-37.
- **Task 2** (this session, commit `9ae2dbd1`): CLI-mirror `D37_CLASSIFICATION_TABLE` +
  `ancestor_guard_classification_matches_the_d37_table`, driving
  `AppliedAncestorTraverseGuard::snapshot_and_apply` through all three reachable scenarios
  (drive root, System32 leaf, tempdir leaf) and asserting each result against the table.
  Daemon-mirror `daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned`
  primary behavioral test, plus the retained (and self-reference-bug-fixed) secondary prose
  guard `daemon_ancestor_skip_rationale_agrees_with_cli_notapplicable_classification`.
- **Task 3** (this session, commit `9ae2dbd1`): `daemon_enum_segment()` anchor helper
  factored out; both `DAEMON-DECISION-ENUM`-anchored discovery tests
  (`daemon_attestation_decision_is_deliberately_two_state` and
  `every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence`) now resolve
  the real enum declaration through it; `parse_enum_variant_names` now also excludes bare
  `//` comment lines.
- Fixed a self-reference bug in the uncommitted secondary prose test left by the interrupted
  prior executor: its `include_str!`-based occurrence-count precondition expected exactly 1
  occurrence of its scoping heading, but the test's own literal reference to that heading
  (embedded via `include_str!("launch.rs")`) makes the real count 2 — the exact class of
  self-reference bug WR-11 already flagged elsewhere in this file. Bumped the expected count
  to 2 with an explicit doc comment explaining why.
- `cargo fmt` fix for two match-scrutinee line-wraps in `dacl_guard.rs`, pre-existing from
  the already-committed Task 1 commit `25abcf92` (this plan's verification gate runs `cargo
  fmt --check`, which the prior commit did not pass).

## Task Commits

1. **Task 1: Implement D-37's three-arm classification; reconcile doc comments** -
   `25abcf92` (fix) — already committed by the interrupted prior executor; perturbation
   proof produced this session (see below).
2. **Task 2 + Task 3: Classification-rule cross-mirror check + WR-14 anchor hardening** -
   `9ae2dbd1` (test) — both tasks landed in one commit since the prior executor's
   uncommitted work-in-progress already combined Task 2's two new test functions with the
   working tree state; Task 3's `daemon_enum_segment()` extraction and its callers'
   rewiring were added on top in this session before committing.

**Plan metadata:** (this commit, following SUMMARY)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` - `stopped_at_non_owned` field +
  three-arm `application()` on both ancestor guards (Task 1); `D37_CLASSIFICATION_TABLE` +
  `ancestor_guard_classification_matches_the_d37_table` (Task 2); `cargo fmt` fix for two
  match-scrutinee wraps.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - reconciled `applied_layers()` doc
  comment citing D-37 (Task 1 only; no further changes this session).
- `crates/nono-cli/src/agent_daemon/launch.rs` - `daemon_enum_segment()` anchor helper
  (Task 3); both `DAEMON-DECISION-ENUM` discovery tests rewired to use it (Task 3); `//`
  comment exclusion in `parse_enum_variant_names` (Task 3);
  `daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned` primary behavioral
  test (Task 2); `daemon_ancestor_skip_rationale_agrees_with_cli_notapplicable_classification`
  secondary prose guard, self-reference bug fixed (Task 2).

## Decisions Made

- D-37 implemented exactly as locked: 117-22's daemon-side "under-granting, never
  under-confining" reading governs; the CLI's `PartiallyApplied` collapse from Plan 117-23
  is corrected to `NotApplicable` for the specific stopped-at-non-owned-ancestor state,
  while the third arm (`NotApplied`, not reachable via today's `snapshot_and_apply`) is kept
  live via a direct struct-literal-constructed test, per the plan's exact acceptance
  criteria.
- Type asymmetry stated explicitly per the plan's requirement: `DaemonDaclGuard::apply`
  returns `nono::Result<Self>` with no `LayerApplication`-typed outcome, so the daemon side
  cannot be driven through the same table as the CLI side. The primary daemon test's doc
  comment and the CLI table's own doc comment both state this in words, not just in code
  structure.
- `%PUBLIC%` substituted for `%SystemRoot%\Temp` (the plan's originally-cited candidate) in
  the daemon behavioral test, because `%SystemRoot%\Temp` empirically denies `READ_CONTROL`
  to this host's unprivileged test principal — the ownership query itself fails before the
  D-37 condition can be established. This substitution was made by the interrupted prior
  executor and is documented directly in the test's own doc comment (`agent_daemon/launch.rs`
  lines ~2618-2637); verified in this session by re-running the test and confirming the
  precondition assertion (`path_is_owned_by_current_user` on `%PUBLIC%` returns `Ok(false)`)
  holds on this host.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Self-reference bug in the uncommitted secondary prose guard's
occurrence-count precondition**
- **Found during:** Task 2 verification (running the full `agent_daemon::launch` test
  module before committing)
- **Issue:** `daemon_ancestor_skip_rationale_agrees_with_cli_notapplicable_classification`
  (left uncommitted by the interrupted prior executor) asserted its scoping heading
  (`"Phase 117 D-21 (CINT-02) / step 6.7's decision shape"`) occurs exactly once in
  `include_str!("launch.rs")`. Since `include_str!` embeds the entire file — including this
  test's own source, which necessarily re-states the heading literal it searches for — the
  real count is 2, not 1. The test failed unconditionally: `left: 2, right: 1`.
- **Fix:** Bumped the expected count to 2 with an explicit doc comment (the "Self-reference"
  section) explaining that `.split(heading).nth(1)` still correctly extracts the segment
  strictly between the two occurrences (the real doc comment's body), since the real heading
  textually precedes the test and the test's own literal sits below it.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch`
  — the fixed test passes; re-broke it via perturbation (see below) to confirm it still
  discriminates.
- **Committed in:** `9ae2dbd1`

**2. [Rule 3 - Blocking] `cargo fmt --check` failure in the already-committed Task 1 code**
- **Found during:** Pre-commit verification for this session's work (`cargo fmt --check`
  run per this plan's `<verification>` gate)
- **Issue:** Both `AppliedAncestorTraverseGuard::application()` and
  `AppliedAncestorReadAttributesGuard::application()`'s match-scrutinee tuples exceeded
  rustfmt's line-width preference, requiring a multi-line wrap `rustfmt` insists on — a
  pre-existing formatting debt from commit `25abcf92` (Task 1), not introduced this session.
- **Fix:** Ran `cargo fmt` (workspace-wide, touched only these two match statements in
  `dacl_guard.rs`).
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs`
- **Verification:** `cargo fmt --check` exits 0 after the fix.
- **Committed in:** `9ae2dbd1`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking/CI-gate)
**Impact on plan:** Both fixes necessary for the plan's own verification gate to pass
honestly. No scope creep — both are directly inside this plan's `files_modified` list.

## Perturbation Proofs

Per the round-3 discipline requirement, every discovery/classification test in this plan
was proven to actually discriminate by breaking the condition it guards, confirming the
test fails, then reverting verbatim. All perturbations below were performed, verified, and
reverted in this session; `git diff --stat` was checked after each revert to confirm no
residue.

### Task 1 (D-37 three-arm classification)

**Perturbation:** Inverted the return values of the `(true, true, true)` and
`(true, true, false)` match arms in both `AppliedAncestorTraverseGuard::application()` and
`AppliedAncestorReadAttributesGuard::application()` (swapping `NotApplicable` ↔
`NotApplied`).

**Command:** `cargo test -p nono-sandbox-cli --bin nono dacl_guard`

**Result (broken):** 5 of 16 tests failed —
`ancestor_traverse_application_reports_not_applicable_when_stopped_at_non_owned`,
`ancestor_traverse_application_reports_not_applied_for_the_unreachable_third_state`,
`ancestor_read_attrs_application_reports_not_applicable_when_stopped_at_non_owned`,
`ancestor_read_attrs_application_reports_not_applied_for_the_unreachable_third_state`, and
`ancestor_guard_classification_matches_the_d37_table` (Task 2's new table test also caught
it). Example failure: `D-37: a walk that ran, granted nothing, and stopped at the first
non-owned ancestor is the D-04 contract-exempt outcome — NotApplicable, not a downgrade:
left: NotApplied, right: NotApplicable`.

**Reverted:** `sed`-restored the original arm order; `cargo test -p nono-sandbox-cli --bin
nono dacl_guard` → 16/16 pass; `git diff --stat` confirmed no residual change beyond the
intended additions.

### Task 2, proof (a): CLI table row perturbation

**Perturbation:** Changed `D37_CLASSIFICATION_TABLE`'s `immediate-parent-non-owned` row
expectation from `NotApplicable` to `PartiallyApplied` (reintroducing WR-12's collapse).

**Command:** `cargo test -p nono-sandbox-cli --bin nono dacl_guard --
ancestor_guard_classification_matches_the_d37_table`

**Result (broken):** FAILED — `D-37 classification-rule table mismatch for scenario
'immediate-parent-non-owned (System32 leaf, stopped-at-non-owned)': expected
PartiallyApplied, got NotApplicable`.

**Reverted:** row restored to `NotApplicable`; test passes; `git diff --stat` confirmed
clean.

### Task 2, proof (b): daemon precondition perturbation

**Perturbation:** Changed the daemon behavioral test's `parent` source from
`std::env::var_os("PUBLIC")` to `std::env::var_os("TEMP")` (a directory owned by the current
user on this host), breaking the D-37 precondition on purpose.

**Command:** `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch --
daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned`

**Result (broken):** FAILED loudly at the precondition assertion, naming the exact
environment assumption that broke: `environment assumption broken: %PUBLIC%
(C:\Users\OMack\AppData\Local\Temp) is owned by the current user on this host, so this test
cannot exercise the D-37 non-owned-immediate-ancestor condition. This test must fail loudly
here rather than silently skip past an unexercised scenario.` — proving the precondition
check does not silently pass an unexercised scenario.

**Reverted:** `parent` source restored to `"PUBLIC"`; test passes; `git diff --stat`
confirmed clean.

### Task 2, proof (c): secondary prose-guard rationale-sentence deletion

**Perturbation:** Changed `under-granting` to `underGranting` (breaking the exact phrase
match) in `DaemonAttestationDecision`'s doc comment at `launch.rs:1259`.

**Command:** `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch --
daemon_ancestor_skip_rationale_agrees_with_cli_notapplicable_classification`

**Result (broken):** FAILED — `DaemonAttestationDecision's doc comment must still state the
under-granting/never-under-confining rationale for the ancestor-traverse and read-only-rule
skip arms — if this assertion fails, the doc comment was edited to remove or reword the
rationale D-37 (117-CONTEXT.md, WR-12) relies on...`.

**Reverted:** phrase restored to `under-granting`; test passes; `git diff --stat` confirmed
clean.

### Task 2, proof (d): primary behavioral test's own discriminating power

**Perturbation:** Replaced `DaemonDaclGuard::apply`'s pass-3 `Ok(false) => { ...; break; }`
arm body with a simulated downgrade: `guard.revert_all(); return
Err(NonoError::SandboxInit(format!("daemon dacl: ancestor not owned (simulated
downgrade): {}", ancestor.display())));`.

**Command:** `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch --
daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned`

**Result (broken):** FAILED — `D-37: DaemonDaclGuard::apply must still return Ok when the
workspace's immediate ancestor is non-owned (pass 3's contract-exempt skip, not a downgrade)
— got Err(Sandbox initialization failed: daemon dacl: ancestor not owned (simulated
downgrade): C:\Users\Public). This is the exact class of regression D-37 exists to
prevent...`. (Two unrelated pre-existing tests in the same module also failed as collateral
from this arm being shared code — `daemon_dacl_guard_applies_and_reverts_write_grant` and
`daemon_dacl_guard_reap_revokes_traverse_paths` — expected, since they also exercise pass 3
on a tempdir whose parent happens to be non-owned on this host.)

**Reverted:** arm body restored verbatim to the original `tracing::debug!` + `break`;
`cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch` → 23/23 pass;
`git diff --stat` confirmed clean.

### Task 3, proof (a): duplicated anchor

**Perturbation:** Inserted a duplicate `DAEMON-DECISION-ENUM` literal (inside a comment)
immediately after `mod tests {`, ahead of the real declaration.

**Command:** `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch --
daemon_attestation_decision_is_deliberately_two_state
every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence`

**Result (broken):** BOTH tests FAILED, each naming the mismatch: `expected exactly 3
occurrences of the gap-closure anchor marking the real enum declaration; found 4 — the
anchor was duplicated or removed, and neither discovery test can trust which occurrence is
the real declaration`.

**Reverted:** duplicate literal removed; both tests pass; `git diff --stat` confirmed clean.

### Task 3, proof (b): bare `//` comment inside the enum body

**Perturbation:** Inserted a `// scratch` comment line at variant indent immediately above
`Proceed` inside `DaemonAttestationDecision`'s body.

**Command:** `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch --
daemon_attestation_decision_is_deliberately_two_state
every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence`

**Result (unbroken, as intended):** BOTH tests still passed — 23/23 in the full module run
— confirming the comment line is correctly excluded from the parsed variant list by the new
`|| trimmed.starts_with("//")` filter.

**Reverted:** comment line removed; `git diff --stat` confirmed clean.

## Issues Encountered

- The plan's literal verification commands (`cargo test -p nono-sandbox-cli --lib
  exec_strategy_windows::dacl_guard` and `--lib agent_daemon::launch`) do not work as
  written: `nono-sandbox-cli` has no `[lib]` target (only `[[bin]] nono` and `[[bin]]
  nono-agentd`), and `exec_strategy_windows/mod.rs` is included into the `nono` binary under
  the module alias `exec_strategy` (via `#[path = "exec_strategy_windows/mod.rs"] mod
  exec_strategy;` in `main.rs`), while `agent_daemon/mod.rs` is included into the
  `nono-agentd` binary. The equivalent working commands used throughout this session were
  `cargo test -p nono-sandbox-cli --bin nono exec_strategy::dacl_guard` and `cargo test -p
  nono-sandbox-cli --bin nono-agentd agent_daemon::launch`. All specified test names and
  pass/fail behavior match the plan's intent exactly; only the package-target flag differs
  from the plan's literal text.
- `cargo test --workspace` (and `cargo test -p nono-sandbox-cli --bins`) silently excludes
  the `nono-agentd` binary's own test suite from its run — no error, no mention of the
  target, and no exit-code signal. Root cause not determined (no `test = false` or
  `required-features` gating found in `Cargo.toml`); this is a Windows-host cargo-invocation
  quirk, not a code regression. Worked around by invoking `--bin nono-agentd` explicitly.
  Flagging for future plans that iterate on `agent_daemon/`.

## Verification

- `cargo build --workspace --all-targets` — clean.
- `cargo fmt --check` — clean (after the fmt fix documented above).
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo test -p nono-sandbox-cli --bin nono exec_strategy::dacl_guard` — 16/16 pass,
  including `ancestor_guard_classification_matches_the_d37_table`.
- `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch` — 23/23 pass,
  including `daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned`,
  `daemon_ancestor_skip_rationale_agrees_with_cli_notapplicable_classification`,
  `daemon_attestation_decision_is_deliberately_two_state`, and
  `every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence`.
- `cargo test --workspace` — 1621 passed, 11 failed (baseline; see below), 2 ignored. The 11
  failures are the documented pre-existing Windows baseline
  (`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`;
  `config::tests::nono_home_dir_falls_through_when_unset`,
  `config::tests::nono_home_dir_rejects_non_absolute_override`,
  `config::tests::nono_home_dir_returns_override_when_set`,
  `config::tests::test_validated_home_falls_back_to_userprofile`,
  `config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists`,
  `config::tests::user_state_dir_uses_localappdata_on_windows`;
  `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`;
  `protected_paths::tests::blocks_child_directory_capability`,
  `protected_paths::tests::blocks_parent_directory_capability`,
  `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`).
  This matches the recorded figure of 11 pre-existing baseline failures (not the earlier
  4-failure record) — none are in this plan's `files_modified` scope and none were touched
  this session.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None — no hardcoded empty/placeholder values were introduced.

## Threat Flags

None — this plan's changes are entirely within the threat surface already registered in its
own `<threat_model>` (T-117-28-01, T-117-28-02); no new network endpoints, auth paths, file
access patterns, or schema changes at a trust boundary were introduced.

## Next Phase Readiness

- D-37 is fully implemented and cross-mirror-proven per its exact locked wording. WR-12 and
  WR-14 are both closed.
- No known blockers for subsequent Phase 117 gap-closure plans. The `--workspace` /
  `--bins` test-discovery quirk documented above (nono-agentd's tests silently excluded) is
  worth carrying forward as institutional knowledge for any future plan touching
  `agent_daemon/`.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*
