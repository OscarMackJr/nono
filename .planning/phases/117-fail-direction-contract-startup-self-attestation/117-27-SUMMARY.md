---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 27
subsystem: infra
tags: [windows, attestation, tracing, security-telemetry, cross-target-clippy]

# Dependency graph
requires:
  - phase: 117-21
    provides: apply_startup_attestation_gate's ProceedDowngraded arm, the D-28 operator-warn gate precedent, log_target_is_private()/set_log_target_is_private_for_test() test seam
provides:
  - all three D-28 emission sites in ProceedDowngraded gated on one shared layer_detail predicate (closes CR-03)
  - banner and withheld-detail warn point at the real Windows Application event log (event id 10011) instead of the nonexistent audit ledger (closes WR-15)
  - log_target_is_private() validated against this launch's own granted CapabilitySet paths, fail-secure on any unknown/error case (closes WR-16)
affects: [117-28, 117-REVIEW, proj/SPEC-windows-fail-direction-contract.md]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One D-28 gate decision computed once per ProceedDowngraded arm invocation (layer_detail), reused at every emission site instead of each site deciding independently"
    - "Discovery-based withholding assertions that scan every captured (name, value) pair for a substring, not a hardcoded field name — the specific class of blindness CR-03 exploited"
    - "Fail-secure granted-path validation: unknown/error at any step (no log-file arm, no path recorded, canonicalization failure) returns not-private"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/output.rs
    - crates/nono-cli/src/cli_bootstrap.rs

key-decisions:
  - "layer_detail is computed once immediately after downgraded_refs and reused verbatim at the two audit-emission-failure sites; the third (operator) site additionally wraps it in an operator_suffix that names the real destination (event id 10011) only when layer_detail is empty"
  - "The operator warn's structured downgraded_layers=%dedup_key field was folded into message-text interpolation (layer_detail) for consistency across all three sites — the pre-existing field-only withholding test was widened to a discovery-based text scan so this consolidation doesn't create a new blind spot"
  - "log_target_is_private()'s existing test seam (set_log_target_is_private_for_test) is preserved as a boolean override (LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE) so all Plan 21/27 launch.rs tests that only care about private/shared are unaffected; new granted-path test cases use dedicated seams that clear the override to exercise the real two-static computation"

requirements-completed: [CINT-02]

# Metrics
duration: 35min
completed: 2026-08-11
---

# Phase 117 Plan 27: Gate Every D-28 Emission Site, Name the Real Channel, Verify the Private-Log Predicate Summary

**Closed CR-03 (2 of 3 `ProceedDowngraded` warn sites leaked `LayerId` names into message text ungated), WR-15 (banner/warn pointed at a nonexistent "audit ledger" instead of the real Windows Application event log, event id 10011), and WR-16 (`log_target_is_private()` treated "a file was opened" as proof of privacy without checking it against the launch's own granted filesystem policy).**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-08-11
- **Tasks:** 3 (2 code tasks + 1 verification-only task)
- **Files modified:** 3 (`launch.rs`, `output.rs`, `cli_bootstrap.rs`)

## Accomplishments

- All three D-28 emission sites in `apply_startup_attestation_gate`'s `ProceedDowngraded` arm now share one `layer_detail` gate computed once from `log_target_is_private()`, instead of only the third site (the Plan 21 operator warn) deciding independently while the other two interpolated `downgraded_layers={dedup_key}` into message text unconditionally.
- Widened the existing field-name-only withholding test into a discovery-based scan of every captured event's every `(name, value)` pair, and added a new sibling test driving the previously-untested `SECURITY_LAYER.get() == None` arm — the exact site CR-03 named as ungated.
- The banner (`output.rs`) and the withheld-detail operator warn now name the real destination — the Windows Application event log, source `nono`, event id 10011 — instead of the nonexistent "audit ledger".
- `log_target_is_private()` now validates the resolved `--log-file` path against this launch's own granted `CapabilitySet` paths (component-wise `Path::starts_with`, canonicalized, fail-secure on any unknown/error case), rather than treating "a file was successfully opened" as sufficient proof of privacy.
- `spawn_windows_child` records the launch's granted filesystem paths once via a new `set_granted_read_paths()`, consumed by the new predicate.
- Both local cross-target clippy gates (linux-gnu via `cross`, apple-darwin via `cargo-zigbuild`) run clean; the linux-gnu run caught a real unused-import error the Windows-host build could not see, fixed in a follow-up commit.

## Task Commits

1. **Task 1: Gate all three D-28 emission sites on one shared predicate; point the destination text at the real channel** - `70787446` (fix)
2. **Task 2: Make log_target_is_private() validate against the launch's own granted filesystem policy (WR-16)** - `0f52c948` (fix)
   - **Follow-up fix (Task 3 discovery):** `97ab295e` (fix) — cfg-gate the `PathBuf` import to Windows-only; see Deviations.
3. **Task 3: Cross-target clippy verification** - no source commit (verification-only task; see Deviations for the fix it triggered)

**Plan metadata:** (this commit, `docs(117-27): complete gate-every-D-28-site plan`)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — one `layer_detail` computation reused at all three `ProceedDowngraded` emission sites; `operator_suffix` names the real event-log destination; `spawn_windows_child` records granted paths; widened/added withholding tests plus a shared `assert_no_downgraded_layer_name_leaked` helper.
- `crates/nono-cli/src/output.rs` — `print_attestation_downgrade_banner`'s eprintln text now names the Windows Application event log (event id 10011) instead of "the audit ledger".
- `crates/nono-cli/src/cli_bootstrap.rs` — `TRACING_LOG_TARGET_PATH` + `GRANTED_READ_PATHS` statics, granted-path-aware `log_target_is_private()`, `set_granted_read_paths()`, rewritten test seams (`LOG_TARGET_IS_PRIVATE_TEST_OVERRIDE`, `set_log_target_path_for_test`, `set_granted_read_paths_for_test`, `clear_log_target_is_private_test_state`), 4 new unit tests (3 required cases + 1 perturbation-proof test), and the Windows-only `PathBuf` import cfg-gate.

## Decisions Made

- Folded the operator warn's previously-structured `downgraded_layers = %dedup_key` field into the same message-text interpolation pattern the other two sites use (`layer_detail`), rather than keeping one site structured and two text-only. This is the direct fix for CR-03's root cause (inconsistent predicate width across sites of the same class) — see round-3 discipline. The pre-existing test that depended on the structured field was widened to a discovery-based text scan so the consolidation doesn't reintroduce a blind spot.
- `log_target_is_private()`'s test-seam contract is preserved exactly for existing callers (`set_log_target_is_private_for_test` still forces the boolean return value directly) while three new seams (`set_log_target_path_for_test`, `set_granted_read_paths_for_test`, `clear_log_target_is_private_test_state`) drive the real two-static granted-path computation for the new WR-16 test cases.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Cross-target clippy caught an unused-import error not visible on the Windows host**
- **Found during:** Task 3 (cross-target clippy verification)
- **Issue:** `use std::path::{Path, PathBuf};` in `cli_bootstrap.rs` — `PathBuf` is only referenced by the Windows-only `log_target_is_private()` machinery Task 2 added; on `x86_64-unknown-linux-gnu` this import is unused, which `-D warnings` turns into a hard `cargo clippy` error (`error: unused import: PathBuf`). Invisible from `cargo build`/`cargo clippy` on this Windows dev host, which only compiles the `#[cfg(target_os = "windows")]` branch.
- **Fix:** Split the import: `use std::path::Path;` stays unconditional (used by `SharedFileMakeWriter::new` on every platform); `#[cfg(target_os = "windows")] use std::path::PathBuf;` gates the Windows-only half.
- **Files modified:** `crates/nono-cli/src/cli_bootstrap.rs`
- **Verification:** `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` both re-ran clean afterward; native Windows build + `cli_bootstrap`/`exec_strategy_windows::launch` test suites re-confirmed green.
- **Committed in:** `97ab295e`

---

**Total deviations:** 1 auto-fixed (1 blocking, Rule 3)
**Impact on plan:** Necessary structural fix surfaced by the cross-target gate itself — exactly the class of drift D-35/D-11 exist to catch on a Windows-only dev host. No scope creep; no `#[allow]` silencing used.

## Perturbation Proofs

**Task 1 (CR-03 discovery tests):** Temporarily reverted the `SECURITY_LAYER.get() == None` arm's message to unconditionally include `downgraded_layers={dedup_key}` (bypassing `layer_detail`), leaving Task 2's changes in place. Re-ran:

```
cargo test -p nono-sandbox-cli --bin nono exec_strategy_windows::launch -- proceed_downgraded
```

Result: 2 of 3 tests FAILED, both correctly naming the leak:

```
test exec_strategy::launch::attestation_gate_tests::proceed_downgraded_security_layer_absent_arm_withholds_layer_names_on_the_shared_console_channel ... FAILED
test exec_strategy::launch::attestation_gate_tests::proceed_downgraded_success_path_withholds_layer_names_on_the_shared_console_channel ... FAILED
...
panicked ... "on the shared-console (non-private) channel, no captured event's rendered output
(field or message text) may contain the downgraded LayerId \"MandatoryIntegrityLabel\" (D-28) —
captured events: [[("message", "attestation downgrade audit emission unavailable (AUD-04:
SecurityEventLayer not initialized) — proceeding per AUD-04's non-fatal contract; downgraded_layers=
MandatoryIntegrityLabel")], ...]"
```

Reverted the perturbation; re-ran the same command — all 3 tests passed again (`test result: ok. 3 passed; 0 failed`).

**Task 2 (WR-16 granted-path test):** `perturbed_granted_path_no_longer_covering_log_path_flips_result_to_private` is itself the perturbation proof required by the plan's acceptance criteria — it takes the exact fixture from `log_path_inside_a_granted_directory_is_not_private` and substitutes a granted path that does NOT cover the log file, asserting the result flips from `false` to `true`. Ran alongside the other three new tests:

```
cargo test -p nono-sandbox-cli --bin nono cli_bootstrap
...
test cli_bootstrap::tests::log_target_is_private_tests::log_path_inside_a_granted_directory_is_not_private ... ok
test cli_bootstrap::tests::log_target_is_private_tests::perturbed_granted_path_no_longer_covering_log_path_flips_result_to_private ... ok
test cli_bootstrap::tests::log_target_is_private_tests::log_path_outside_every_granted_directory_is_private ... ok
test cli_bootstrap::tests::log_target_is_private_tests::no_log_path_recorded_is_not_private ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1626 filtered out
```

Both the positive and the perturbed-negative case pass, demonstrating the assertion is not vacuously true.

## Issues Encountered

None beyond the Task 3 cross-target discovery documented above.

## Verification

- `cargo build --workspace --all-targets` — clean.
- `cargo test -p nono-sandbox-cli --bin nono exec_strategy_windows::launch -- proceed_downgraded` — 3/3 passed. (Note: the plan's literal verification command uses `--lib`, but package `nono-sandbox-cli` has no library target — only `[[bin]] name = "nono"` and `name = "nono-agentd"` — so `--lib` errors with "no library targets found". `--bin nono` is the equivalent working invocation and exercises the identical test code; this is a pre-existing crate-shape fact, not something this plan changed.)
- `cargo test -p nono-sandbox-cli --bin nono cli_bootstrap` — 5/5 passed (4 new + 1 pre-existing).
- `cargo test -p nono-sandbox-cli --bin nono` (full suite) — 1618 passed, 11 failed, 2 ignored. The 11 failures are in `audit_session.rs`, `config/mod.rs`, `profile_cmd.rs`, and `protected_paths.rs` — none touched by this plan — and match the documented pre-existing Windows-host baseline (memory `nono_cli_windows_baseline_test_failures`: 11 known-baseline failures, not regressions).
- `cargo clippy -p nono-sandbox-cli --bin nono -- -D warnings -D clippy::unwrap_used` (native Windows) — clean.
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — clean (after the Task 3 discovery fix above).
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) — clean.
- `grep -c "audit ledger" crates/nono-cli/src/output.rs crates/nono-cli/src/exec_strategy_windows/launch.rs` — 0 in both files.
- `grep -n "event id 10011" crates/nono-cli/src/output.rs` — present.
- `grep -c "layer_detail" crates/nono-cli/src/exec_strategy_windows/launch.rs` — 9 (declared once, referenced at all three sites plus doc comments).
- `grep -n "fn set_granted_read_paths" crates/nono-cli/src/cli_bootstrap.rs` — present, `pub(crate)`, non-test.
- `grep -n "config.caps.fs_capabilities()" crates/nono-cli/src/exec_strategy_windows/launch.rs` — present inside `spawn_windows_child` (line 1646), alongside the pre-existing usage at line 882.
- `grep -n "\.starts_with(" crates/nono-cli/src/cli_bootstrap.rs` — the new comparison (`canonical_log_path.starts_with(&canonical_granted)`) uses `Path`/`PathBuf` values, never a bare string receiver; the only other hit is the pre-existing legacy-flag string check, unrelated to path security.

## Known Stubs

None.

## Threat Flags

None — this plan closes existing information-disclosure findings (CR-03, WR-15, WR-16) rather than introducing new security-relevant surface. No new network endpoints, auth paths, file-access patterns, or schema changes at trust boundaries.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- CR-03, WR-15, and WR-16 are closed with re-runnable evidence (perturbation proofs above).
- The `ProceedDowngraded` arm's D-28 discipline is now internally consistent: one gate, three sites, all covered by discovery-based tests.
- `log_target_is_private()`'s contract now genuinely matches its name; any future caller relying on it for a privacy decision inherits the granted-path check automatically.
- No blockers for the remaining 117 gap-closure round-3 plans.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

All modified files confirmed present on disk (`launch.rs`, `output.rs`, `cli_bootstrap.rs`,
this SUMMARY.md). All four commit hashes (`70787446`, `0f52c948`, `97ab295e`, `a89a5ead`)
confirmed present in `git log --oneline --all`.
