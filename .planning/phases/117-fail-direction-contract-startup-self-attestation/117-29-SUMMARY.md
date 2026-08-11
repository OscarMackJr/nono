---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 29
subsystem: infra
tags: [windows, mandatory-integrity-label, remediation, diagnostics, sacl, class-coverage]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "CR-01's INHERIT_ONLY_ACE rejection in low_integrity_label_ace (Phase 117-20); WR-04's remediation-line plumbing in render_error_for_operator (Phase 117-21)"
provides:
  - "render_error_for_operator branches remediation text by failed layer: icacls guidance for MandatoryIntegrityLabel, Windows Application event log pointer for every other layer"
  - "low_integrity_label_rid delegates to low_integrity_label_ace and rejects INHERIT_ONLY_ACE, closing CR-01's class in its second (and now only remaining) SACL reader"
affects: [117-gap-closure-round-4, windows-mandatory-label-readers, windows-cli-remediation-text]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single-reader delegation: is_low_integrity_compatible_dir's low_integrity_label_rid is now a thin wrapper over low_integrity_label_ace, eliminating a duplicate FFI SACL walk and guaranteeing both call sites of a security predicate share one implementation"
    - "Layer-name branch on structured remediation: NonoRemediation::ClearStaleLayerResidue { layer } is matched by exact layer string, not treated as a single opaque case"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/main.rs
    - crates/nono/src/sandbox/windows.rs

key-decisions:
  - "Kept one literal occurrence of the string \"nono setup --check-only\" in main.rs (a negative-assertion test guarding against regression) despite the plan's literal acceptance-criteria grep expecting 0 — the emitting code path itself (the match arms) has 0 occurrences; the remaining 1 is functionally required to test the string's absence from the WfpEgressFilters branch"
  - "Ported plant_mandatory_label_with_flags from nono-cli's labels_guard.rs test module into nono's windows.rs test module verbatim rather than exposing it pub(crate) cross-crate, per the plan's explicit guidance (the helper cannot be imported cross-crate as a private fn)"

patterns-established:
  - "Class-coverage enumeration before fixing: before touching low_integrity_label_rid, grepped every SYSTEM_MANDATORY_LABEL_ACE_TYPE reader in the codebase to confirm only two existed (low_integrity_label_ace and low_integrity_label_rid) and that no third site was missed"

requirements-completed: [CINT-02]

# Metrics
duration: ~35min
completed: 2026-08-11
---

# Phase 117 Plan 29: Gap-Closure Round 3 (WR-17, WR-18) Summary

**Layer-specific remediation text in render_error_for_operator (icacls for MandatoryIntegrityLabel, event log for every other layer) and low_integrity_label_rid delegating to the CR-01-hardened low_integrity_label_ace, closing both class-coverage gaps iteration 5 found.**

## Performance

- **Duration:** ~35 min (code changes ~15 min; cross-target clippy + full workspace test verification ~20 min)
- **Started:** 2026-08-11T11:32:00-04:00 (approx, immediately after 117-28)
- **Completed:** 2026-08-11T12:07:00-04:00 (approx)
- **Tasks:** 3 (2 code tasks + 1 verification-only task)
- **Files modified:** 2

## Accomplishments
- WR-17 closed: `render_error_for_operator`'s remediation line now branches on the failed layer name — `MandatoryIntegrityLabel` gets the real `icacls <path> /setintegritylevel Medium` remedy; every other layer (`WfpEgressFilters`, `AppContainerProfile`, `JobObjectContainment`, forced-unavailable-seam) points at the Windows Application event log instead of the previously-hardcoded, always-wrong `nono setup --check-only` line.
- WR-18 closed: `low_integrity_label_rid` (the sole reader behind `is_low_integrity_compatible_dir` / `Sandbox::windows_supports_direct_writable_dir`) no longer runs its own standalone SACL walk. It now delegates to `low_integrity_label_ace` — the same CR-01-hardened reader `labels_guard.rs`'s residue-equivalence predicate uses — and rejects `INHERIT_ONLY_ACE` the same way. Enumerated every `SYSTEM_MANDATORY_LABEL_ACE_TYPE` reader in the codebase; confirmed exactly two existed and both now share one implementation.
- Both cross-target clippy gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`) ran clean with `-D warnings -D clippy::unwrap_used` against the full workspace.
- Full workspace test suite run: 1622 passed, 11 failed (all 11 match the documented pre-existing Windows-host baseline — audit_session ×1, config ×6, profile_cmd ×1, protected_paths ×3 — none are regressions from this plan).

## Task Commits

Each task was committed atomically:

1. **Task 1: Layer-specific remediation text in main.rs (WR-17)** - `0604e142` (fix)
2. **Task 2: Close CR-01's class in low_integrity_label_rid (WR-18)** - `97d9f0f5` (fix)
   - fmt-only follow-up (rustfmt reformatted the new test/helper code touched by Task 2) - `7f2f5496` (style)
3. **Task 3: Cross-target clippy verification** - no source changes; verification-only (see below)

**Plan metadata:** (this commit, following SUMMARY.md write)

_Note: Task 2's fmt commit is a direct consequence of Task 2's own edits to the same file — not a separate deviation, just `cargo fmt --all` catching two spots rustfmt wanted reformatted (an `assert_ne!` arg wrap and a call-site line-length wrap) that manual editing missed._

## Files Created/Modified
- `crates/nono-cli/src/main.rs` - `render_error_for_operator` now matches `layer.as_str()`: `"MandatoryIntegrityLabel"` → icacls remedy, `_` → Windows Application event log pointer. Updated the existing `render_error_for_operator_adds_remediation_for_layer_attestation_failed` test's assertion from `"nono setup --check-only"` to `"icacls"` + `"/setintegritylevel Medium"`. Added `render_error_for_operator_names_the_event_log_for_non_label_layers` asserting a `WfpEgressFilters` layer gets the event-log line and does NOT get the check-only command.
- `crates/nono/src/sandbox/windows.rs` - `low_integrity_label_rid` reduced from a ~90-line standalone `GetNamedSecurityInfoW`/`GetAce` SACL walk to a 4-line delegating wrapper over `low_integrity_label_ace`, filtering `INHERIT_ONLY_ACE` via the same bitwise predicate CR-01 already shipped in `labels_guard.rs`. Added `INHERIT_ONLY_ACE` to the `windows_sys::Win32::Security` import list. Ported `plant_mandatory_label_with_flags` (raw-SDDL mandatory-label planting helper) into this file's `#[cfg(test)] mod tests`, and added `is_low_integrity_compatible_dir_rejects_an_inherit_only_label`, which plants an inherit-only Low-RID label and asserts both `low_integrity_label_rid` and `is_low_integrity_compatible_dir` reject it.

## Decisions Made
- Kept the emitting code path (the `match layer.as_str()` arms) free of any `"nono setup --check-only"` string (0 occurrences, matching the plan's intent), but retained exactly one literal occurrence of that string in a negative-assertion test (`assert!(!lines[1].contains("nono setup --check-only"))`) — this is the mechanism that actually catches a regression back to the old hardcoded behavior. The plan's acceptance criterion for a literal `grep -c == 0` across the whole file is stricter than necessary here; documented as a deliberate, minor deviation rather than weakening the regression test.
- Removed the duplicate SACL-walking FFI implementation from `low_integrity_label_rid` entirely (not just its `INHERIT_ONLY_ACE` gap) — `low_integrity_label_and_mask` already established the "thin wrapper over `low_integrity_label_ace`" pattern immediately below in the same file, so `low_integrity_label_rid` now follows the same shape rather than duplicating logic a second way.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug/formatting] `cargo fmt --all` reformatted two spots in windows.rs's new test code**
- **Found during:** Task 2 verification (`cargo fmt --check`)
- **Issue:** Two lines in the newly-added `plant_mandatory_label_with_flags` helper and the new regression test did not match rustfmt's canonical wrapping (an `assert_ne!(ok, 0, ...)` argument list and a `plant_mandatory_label_with_flags(...)` call that exceeded the wrap threshold on one line but not when split by rustfmt's rules).
- **Fix:** Ran `cargo fmt --all`; verified `cargo fmt --check` then reported clean.
- **Files modified:** `crates/nono/src/sandbox/windows.rs`
- **Verification:** `cargo fmt --check` clean; `cargo test -p nono-sandbox --lib sandbox::windows` still 104/104 passing after the reformat.
- **Committed in:** `7f2f5496`

---

**Total deviations:** 1 auto-fixed (1 formatting)
**Impact on plan:** No scope creep — purely mechanical formatting on code this plan itself introduced.

## Class-Coverage Enumeration (Round 3 Discipline)

Per the round-3 discipline directive, both classes named in this plan were enumerated for sibling sites before fixing:

**(a) Mandatory-label ACE readers** — searched `crates/nono/src/sandbox/windows.rs` and `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` for every `SYSTEM_MANDATORY_LABEL_ACE_TYPE` filter check. Found exactly two: `low_integrity_label_ace` (already CR-01-hardened) and `low_integrity_label_rid` (this plan's fix target). No third site exists anywhere in the workspace (confirmed via `grep -rn "SYSTEM_MANDATORY_LABEL_ACE_TYPE\|LABEL_SECURITY_INFORMATION" --include="*.rs" crates/ bindings/` — the only other hit is a comment, not a reader). Post-fix, both readers delegate to one implementation.

**(b) `render_error_for_operator` remediation branches** — the function has exactly one remediation-emitting branch (`ClearStaleLayerResidue { layer }`); no sibling branch for a different `NonoRemediation` variant exists that also needed layer-awareness. The fix is complete within that single branch's `match layer.as_str()`.

No third site was found for either class; nothing was deferred.

## Perturbation Proofs

**Task 1 (WR-17):** Temporarily changed the match arm from `"MandatoryIntegrityLabel" =>` to `"__PERTURBATION_DISABLED__" =>` (forcing the icacls-remedy branch to never fire, falling through to the event-log `_` arm for every layer including `MandatoryIntegrityLabel`), then ran:
```
cargo test -p nono-sandbox-cli --bin nono render_error_for_operator
```
Result: `render_error_for_operator_adds_remediation_for_layer_attestation_failed` FAILED with `assertion failed: lines[1].contains("icacls")` (2 passed, 1 failed). Reverted; all 3 tests pass clean.

**Task 2 (WR-18):** Temporarily dropped the `.filter(|(_, _, flags)| (u32::from(*flags) & INHERIT_ONLY_ACE) == 0)` call from `low_integrity_label_rid`'s delegating body, then ran:
```
cargo test -p nono-sandbox --lib sandbox::windows::tests::is_low_integrity_compatible_dir_rejects_an_inherit_only_label
```
Result: FAILED — `assertion \`left == right\` failed: an INHERIT_ONLY_ACE mandatory-label ACE, even at the Low RID, must never be reported as an effective label by low_integrity_label_rid (WR-18)`, `left: Some(4096), right: None`. Reverted; full `sandbox::windows` suite (104 tests) passes clean.

## Verification Evidence

- `cargo build --workspace --all-targets` — clean (2m55s).
- `cargo fmt --check` — clean (after the fmt fix above).
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean, zero warnings.
- `cargo test -p nono-sandbox-cli --bin nono render_error_for_operator` — 3/3 passed.
- `cargo test -p nono-sandbox --lib sandbox::windows` — 104/104 passed.
- `cargo test --workspace` — 1622 passed, 11 failed, 2 ignored. All 11 failures match the documented pre-existing Windows-host baseline exactly by name and count: `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty` (1), `config::tests::{nono_home_dir_falls_through_when_unset, nono_home_dir_rejects_non_absolute_override, nono_home_dir_returns_override_when_set, test_validated_home_falls_back_to_userprofile, test_validated_home_ignores_non_absolute_home_when_userprofile_exists, user_state_dir_uses_localappdata_on_windows}` (6), `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` (1), `protected_paths::tests::{blocks_child_directory_capability, blocks_parent_directory_capability, requested_path_blocks_nonexistent_child_under_protected_root}` (3). None are new regressions; both new tests added by this plan are in the passing set.

## Cross-Target Clippy Verification (Task 3, D-35/D-11)

Both `main.rs` and `windows.rs` are in scope per `.planning/templates/cross-target-verify-checklist.md`: `main.rs` contains `#[cfg(target_os = "linux")]` and `#[cfg(target_os = "macos")]` blocks elsewhere in the file and compiles on every platform, so Task 1's edit is exercised by both gates directly. (Note: `crates/nono/src/sandbox/windows.rs` itself is gated entirely behind `pub mod windows;`'s own `#[cfg(target_os = "windows")]` in `sandbox/mod.rs` — the file compiles to nothing on Linux/macOS targets, so Task 2's edit is not independently exercised by these two gates; it is still covered by the native Windows build/clippy/test run above. The plan's stated rationale that "the file itself also carries non-Windows code paths" does not hold for `windows.rs`'s own content — corrected here for the record — but does not change the outcome since Task 1's file is what makes the gates apply to this plan.)

- **linux-gnu** (`cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`): Docker engine confirmed up (`Server Version: 29.6.2`). Ran to completion clean — `Checking nono-sandbox-cli v0.70.0 ... Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 7m 56s`. Zero errors, zero warnings.
- **apple-darwin** (`SDKROOT= cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`, `SDKROOT` unset): Ran to completion clean — `Checking nono-sandbox-cli v0.70.0 ... Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 48.19s`. Zero errors, zero warnings.

Both gates pass; no PARTIAL→CI fallback needed.

## Issues Encountered
None beyond the fmt deviation documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Both WR-17 and WR-18 class-coverage gaps from iteration 5 are closed and verified with perturbation proofs.
- Per the round-3 discipline enumeration, no sibling site was left broken for either class — future rounds do not need to revisit `render_error_for_operator`'s layer branching or `windows.rs`'s mandatory-label ACE readers unless new layers or new readers are introduced.
- No blockers for gap-closure round 4 (if scheduled) or for continuing Phase 117 work.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*
