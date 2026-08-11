---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 30
subsystem: testing
tags: [windows, mandatory-integrity-label, icacls, ownership, coverage-arithmetic, wr-20]

requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "Plan 117-20's WR-01 ownership-first reorder in AppliedLabelsGuard::snapshot_and_apply"
provides:
  - "A regression test that pins the non-owned + foreign-labeled combined condition as AppliedLabel::SkipNotOwned, distinguishing it from the incumbent C:\\Windows-based test which controls only one variable"
affects: [117-fail-direction-contract-startup-self-attestation]

tech-stack:
  added: []
  patterns:
    - "Own-then-label-then-give-away ownership construction (icacls /setowner, SYSTEM then Administrators fallback) for tests needing a real non-owned + labeled Windows path"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs

key-decisions:
  - "On a host lacking SeRestorePrivilege/SeTakeOwnershipPrivilege/SeBackupPrivilege, the new test panics loudly at the ownership-reassignment setup step rather than silently skipping or substituting an incidental path — matching the plan's own explicit D-31 guidance for this exact scenario."
  - "Tried Everyone and Authenticated Users as alternate /setowner targets (both are enabled token groups on this account) to see if a privilege-free construction path existed; both failed identically to SYSTEM/Administrators, confirming icacls' 'or a group you are a member of' MSDN exception requires the SE_GROUP_OWNER token attribute, not mere group membership — this account holds neither SeRestorePrivilege nor any group with that attribute enabled."

requirements-completed: [CINT-02]

duration: 45min
completed: 2026-08-11
---

# Phase 117 Plan 30: Pin the non-owned + foreign-label combined condition Summary

**New regression test constructs a real non-owned + foreign-mandatory-labeled Windows file (own → plant label → give ownership away via `icacls /setowner`) and pins `AppliedLabel::SkipNotOwned` over `SkipPreExistingLabel`; on this specific dev host the test fails loudly at setup because the session lacks `SeRestorePrivilege`, which is documented in full below rather than worked around.**

## Performance

- **Duration:** ~45 min
- **Completed:** 2026-08-11T16:21:13Z
- **Tasks:** 1/1
- **Files modified:** 1

## Accomplishments

- Added `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` to `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs`'s test module. The test:
  1. Creates a tempfile (owned by the current user by construction).
  2. Plants a genuine, non-inherit-only mandatory-label ACE via `plant_mandatory_label_with_flags` at a mask that deliberately mismatches the test's own `AccessMode::Read` rule (mirrors the existing NR3-01 precedent), while ownership is still held.
  3. Independently confirms the plant landed at the expected `(rid, mask)` and that `AceFlags` excludes `INHERIT_ONLY_ACE` (the CR-01 inert shape) — via `low_integrity_label_ace`, before giving ownership away.
  4. Reassigns ownership away via `icacls /setowner "NT AUTHORITY\SYSTEM"`, falling back to `"BUILTIN\Administrators"` if that fails; panics with a combined, actionable diagnostic if BOTH fail.
  5. Independently re-confirms the precondition via `path_is_owned_by_current_user(&file) == Ok(false)` before trusting the guard's output.
  6. Runs `AppliedLabelsGuard::snapshot_and_apply` and asserts `guard.entries[0]` is `AppliedLabel::SkipNotOwned` (never `SkipPreExistingLabel`), `coverage().skipped_pre_existing_label == 0`, `coverage().skipped_not_owned == 1`.
- Verified `cargo build --workspace --all-targets`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` are all clean.
- Verified the other 10 tests in `labels_guard.rs`'s suite pass with zero regressions.
- Investigated whether a privilege-free `/setowner` target exists on this host (tried `Everyone` and `Authenticated Users`, both enabled token groups this account belongs to) — both failed identically, confirming the combined condition genuinely cannot be constructed here without elevation.

## Host-Specific Outcome (REQUIRED READING — this is the load-bearing section)

**This host cannot execute the new test's assertion under test.** The dev host's shell runs as a non-elevated, non-admin domain account (`TWGGLOBAL\OMack`). Verified via the real Windows `whoami.exe` (not the MSYS shim, which silently intercepts bare `whoami` calls in this environment and must be invoked by full `C:\Windows\System32\whoami.exe` path to get real output):

```
$ powershell -NoProfile -Command "& 'C:\Windows\System32\whoami.exe' /priv"
PRIVILEGES INFORMATION
----------------------
Privilege Name                Description                          State
============================= ==================================== ========
SeShutdownPrivilege           Shut down the system                 Disabled
SeChangeNotifyPrivilege       Bypass traverse checking             Enabled
SeUndockPrivilege             Remove computer from docking station Disabled
SeIncreaseWorkingSetPrivilege Increase a process working set       Disabled
SeTimeZonePrivilege           Change the time zone                 Disabled
```

`SeRestorePrivilege`, `SeTakeOwnershipPrivilege`, and `SeBackupPrivilege` are **absent entirely** from the token (not merely disabled). `BUILTIN\Administrators` is present in `whoami /groups` output but marked `Group used for deny only` (UAC-filtered) and `IsInRole(Administrator)` returns `False`. Per MSDN, `SetNamedSecurityInfoW`/`icacls /setowner` to an arbitrary SID requires either `SeRestorePrivilege` or that the target SID be the caller or a group the caller belongs to **with the `SE_GROUP_OWNER` token attribute** (not mere enabled membership) — I confirmed this distinction empirically by trying `Everyone` and `Authenticated Users` (both enabled, non-owner-flagged groups this account belongs to); both failed with the identical error to `SYSTEM`/`Administrators`:

```
f.txt: This security ID may not be assigned as the owner of this object.
```

Consequently, the new test's ownership-reassignment setup step (Step 2, both the `SYSTEM` attempt and the `Administrators` fallback) fails on this host, and the test **panics loudly** rather than silently passing or silently skipping — this is the exact behavior the plan's `<action>` and the round-3 discipline directed for this scenario ("a LOUD test-setup failure, per D-31, not a silent skip, if the host cannot perform this reassignment"):

```
thread 'exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap' panicked at crates\nono-cli\src\exec_strategy_windows\labels_guard.rs:1215:13:
test setup: could not reassign ownership of C:\Users\OMack\AppData\Local\Temp\.tmpyzyDeg\foreign-labeled.txt away from the current user — icacls /setowner "NT AUTHORITY\SYSTEM" failed (C:\Users\OMack\AppData\Local\Temp\.tmpyzyDeg\foreign-labeled.txt: This security ID may not be assigned as the owner of this object.), and the "BUILTIN\Administrators" fallback also failed (C:\Users\OMack\AppData\Local\Temp\.tmpyzyDeg\foreign-labeled.txt: This security ID may not be assigned as the owner of this object.). This host's session almost certainly lacks SeRestorePrivilege (non-elevated / non-admin token) and cannot construct the non-owned + foreign-labeled combined condition this test pins. This is a documented, loud test-setup failure (D-31), not a silent skip — see 117-30-SUMMARY.md for the host-behavior record.
```

Full-suite run for `-p nono-sandbox-cli --bin nono` (1622 passed, 12 failed, 2 ignored) confirms this is **exactly the documented 11-item baseline plus this one new host-blocked test** — no other regression:

```
failures:
    audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty      [baseline]
    config::tests::nono_home_dir_falls_through_when_unset                                       [baseline]
    config::tests::nono_home_dir_rejects_non_absolute_override                                  [baseline]
    config::tests::nono_home_dir_returns_override_when_set                                       [baseline]
    config::tests::test_validated_home_falls_back_to_userprofile                                 [baseline]
    config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists          [baseline]
    config::tests::user_state_dir_uses_localappdata_on_windows                                   [baseline]
    exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap   [NEW — this host's privilege gap]
    profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name                           [baseline]
    protected_paths::tests::blocks_child_directory_capability                                     [baseline]
    protected_paths::tests::blocks_parent_directory_capability                                     [baseline]
    protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root            [baseline]

test result: FAILED. 1622 passed; 12 failed; 2 ignored; 0 measured; 0 filtered out
```

The isolated run for the file's own module (`exec_strategy::labels_guard`, skipping the new test) is fully green — no regression among the ~10 existing tests:

```
$ cargo test -p nono-sandbox-cli --bin nono exec_strategy::labels_guard -- --skip non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap
running 10 tests
test exec_strategy::labels_guard::tests::guard_skips_path_not_owned_by_current_user ... ok
test exec_strategy::labels_guard::tests::residue_is_not_reverted_on_drop ... ok
test exec_strategy::labels_guard::tests::guard_apply_then_drop_reverts_label_for_fresh_file ... ok
test exec_strategy::labels_guard::tests::inherit_only_residue_is_not_treated_as_already_covered ... ok
test exec_strategy::labels_guard::tests::audit_flush_before_drop ... ok
test exec_strategy::labels_guard::tests::mismatched_prior_mask_still_records_a_coverage_gap ... ok
test exec_strategy::labels_guard::tests::guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label ... ok
test exec_strategy::labels_guard::tests::guard_reverts_all_entries_if_mid_loop_apply_fails ... ok
test exec_strategy::labels_guard::tests::coverage_distinguishes_full_partial_and_zero_ace_launches ... ok
test exec_strategy::labels_guard::tests::stale_residue_from_an_abnormal_exit_does_not_self_lock_out_the_next_launch ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 1626 filtered out
```

**Note on module path:** the plan's acceptance criteria specify `cargo test -p nono-sandbox-cli --lib exec_strategy_windows::labels_guard`. This crate has no `--lib` target (it is bin-only, `nono`/`nono-agentd`); the correct invocation is `cargo test -p nono-sandbox-cli --bin nono exec_strategy::labels_guard` — the module path is `exec_strategy::labels_guard`, not `exec_strategy_windows::labels_guard`, because `main.rs` remaps the module via `#[cfg(target_os = "windows")] #[path = "exec_strategy_windows/mod.rs"] mod exec_strategy;`. This is a pre-existing crate-layout fact, not something this plan changed; documented here since it affects how any reviewer reruns the plan's stated verification command.

## Perturbation Proof — status: NOT PRODUCIBLE ON THIS HOST, with reasoning

The round-3 discipline requires actual command output showing the new test fails if the ownership gate is moved back after the label check, and states "a test you cannot show failing is not evidence." I could not produce this literally on this host, and I want to be explicit and honest about why rather than fabricate or approximate it:

Both the "reorder production code" and "flip this test's expected variant" perturbation options require the test to first reach the post-setup assertion. On this host, **setup itself (Step 2, ownership reassignment) fails identically regardless of what the production code or the assertion say** — the test panics at line 1215, before `AppliedLabelsGuard::snapshot_and_apply` is ever called. Perturbing the code downstream of a step that never executes produces no observable difference; running either perturbation would show the *same* panic message, at the *same* line, for the *same* reason (`SeRestorePrivilege` absent), which is not evidence of anything about the ownership-vs-label ordering — it is only evidence of the host's privilege ceiling, which I already have and reported above.

What I verified as the next-best substitute (explicitly **not** equivalent to a runtime perturbation proof, and I am not presenting it as one):

1. **Structural trace of the shipped code** (`crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:258-268`): `path_is_owned_by_current_user`'s `Ok(false)` arm unconditionally pushes `AppliedLabel::SkipNotOwned` and executes `continue` — the `continue` statement makes it impossible, as a matter of Rust control flow, for `low_integrity_label_ace` (called at line 284, after this arm) to ever run for that iteration of the loop. There is no code path connecting a `SkipNotOwned` push to a later `SkipPreExistingLabel` push within the same loop iteration. This is a compile-time-checkable structural fact, not a runtime observation, and I am flagging the gap between this and the round-3 discipline's actual bar rather than quietly substituting one for the other.
2. **The test's own field-level assertion design directly encodes the discriminator**: `matches!(guard.entries[0], AppliedLabel::SkipNotOwned)` would fail (not panic — a normal assertion failure) if a future edit re-inserted a label check ahead of the ownership check and that check produced `SkipPreExistingLabel` for this exact combined-condition file, *provided the test can reach that line* on the executing host. On a suitably privileged host (CI runner as `SYSTEM`/`Administrator`, or an elevated local dev session), this assertion is exactly the guard the round-3 discipline is asking for, and the earlier `assert_ne!(foreign_mask, label_mask_for_access_mode(AccessMode::Read))` precondition specifically ensures the mismatched-mask path would land on `SkipPreExistingLabel` (never `AlreadyAtRequiredLevel`) if the ownership gate were bypassed — keeping the discriminator unambiguous once the test can run at all.

**Recommendation for the phase record:** this test requires `SeRestorePrivilege` (or `SeTakeOwnershipPrivilege` combined with being able to first take, then relinquish, ownership — not attempted here since taking ownership FOR yourself doesn't help construct "non-owned by current user") to execute meaningfully. It will run and produce a real perturbation-provable result on a Windows CI runner executing as `SYSTEM`/`Administrator`, or a locally elevated dev session. This should be tracked as a follow-up verification item rather than treated as resolved by this plan's local run.

## Task Commits

1. **Task 1: Test the non-owned + foreign-label combined condition** - `34157104` (test)

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` - Added `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`, a regression test constructing a real Windows file that is both non-owned by the current user and carries a genuine foreign mandatory-label ACE.

## Decisions Made

- Followed the plan's explicit anticipation of a privilege-gated host: the test fails loudly (panic with an actionable, combined diagnostic naming both the `SYSTEM` and `Administrators` attempts) rather than silently skipping, per D-31.
- Added a `BUILTIN\Administrators` fallback attempt (per the plan's `<behavior>` spec: "whichever `icacls /setowner` succeeds against on this host") even though it does not help on this specific host, since it will help on a locally-elevated-but-not-SYSTEM host.
- Did not add an `#[ignore]` attribute or any environment-probing early return — the plan's own action text explicitly directs an `assert!`/panic-style failure, and an `#[ignore]` would be a form of the "silent skip" the round-3 discipline forbids (ignored tests do not exercise or report anything about the condition under test; a panic does, and states exactly why).

## Deviations from Plan

None - plan executed exactly as written. The plan's `<action>` section explicitly anticipated and pre-authorized the loud-failure host outcome documented above ("this is a LOUD test-setup failure, per D-31, not a silent skip, if the host cannot perform this reassignment"); no rule-based auto-fix or architectural change was needed — this is exactly the documented, expected behavior for a non-privileged host, not a defect.

## Issues Encountered

- **`whoami` shim collision:** bare `whoami` inside a `powershell -NoProfile -Command` invocation from this Bash tool resolved to the MSYS `/usr/bin/whoami` (which doesn't understand `/priv`/`/groups`) rather than the real Windows binary, because MSYS's PATH entry precedes `System32` inside the spawned PowerShell's inherited environment. Resolved by invoking `& 'C:\Windows\System32\whoami.exe'` with the explicit full path.
- **Module path mismatch in the plan's stated verify command:** `--lib` does not exist for `nono-sandbox-cli` (bin-only crate); the correct invocation is `--bin nono`, and the module path is `exec_strategy::labels_guard` (not `exec_strategy_windows::labels_guard`) due to the `#[path = "exec_strategy_windows/mod.rs"] mod exec_strategy;` remap in `main.rs`. Documented above; not a code change, just a verification-command correction.
- **Host cannot construct the test's precondition (see "Host-Specific Outcome" above)** — the core issue this plan's execution surfaced. Not treated as a Rule 1/2/3 auto-fix candidate because there is no code defect to fix; it is a real OS privilege ceiling on this specific dev host, exactly the scenario the plan's `<action>` text pre-authorized a loud-failure response for.

## User Setup Required

None - no external service configuration required. (Running this specific new test to a passing/perturbation-provable state requires either a Windows CI runner executing as `SYSTEM`/`Administrator`, or a locally elevated dev session with `SeRestorePrivilege`; this is an execution-environment note, not a setup task.)

## Next Phase Readiness

- WR-20's test scaffolding is in place and correct by inspection and structural trace; it is ready to provide real, host-verified perturbation evidence the next time it runs on a sufficiently privileged Windows session (CI or elevated dev).
- No blockers for closing out this gap-closure round from a code-correctness standpoint — the guard's WR-01 ordering itself is untouched and was already correct on the merits per the plan's `<interfaces>` section; this plan only adds test coverage.
- Recommend a follow-up note (or operator awareness) that this dev host's default session cannot execute ownership-reassignment-based regression tests at all — any FUTURE gap-closure round that needs the same technique will hit the identical ceiling here.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` (modified, contains `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`)
- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-30-SUMMARY.md`
- FOUND commit `34157104` (test: pin non-owned+foreign-labeled combined condition)
- FOUND commit `bb59c4f3` (docs: record WR-20 gap-closure test and host-privilege limitation)
