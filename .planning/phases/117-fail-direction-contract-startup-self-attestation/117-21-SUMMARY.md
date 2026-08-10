---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 21
subsystem: infra
tags: [tracing, d-28, startup-self-attestation, windows, cross-target-clippy, cli-error-handling]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "apply_startup_attestation_gate ProceedDowngraded arm (117-09/10/11), NonoRemediation::ClearStaleLayerResidue (117-13)"
provides:
  - "cli_bootstrap::log_target_is_private() — Windows-only query tracking whether the active tracing subscriber writes to a private (file-log) channel vs. a shared stderr channel the confined child can read"
  - "D-28-compliant gating of the downgrade warn's specific LayerId names on that query"
  - "main.rs render_error_for_operator() rendering NonoError::remediation()'s ClearStaleLayerResidue guidance for the operator"
affects: [117-fail-direction-contract-startup-self-attestation, windows-composite-integrity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Windows-only AtomicBool query gated behind #[cfg(target_os = \"windows\")] when its only consumer is itself Windows-only (mirrors output::print_attestation_downgrade_banner's CINT-02 gating)"
    - "Process-global #[cfg(test)] Mutex to serialize tests that mutate a shared global (mirrors test_env::ENV_LOCK)"
    - "Factor CLI error-printing into a small pure fn (render_error_for_operator) so branch coverage is unit-testable without invoking run_cli"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/cli_bootstrap.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/output.rs
    - crates/nono-cli/src/main.rs

key-decisions:
  - "log_target_is_private() and its static/test-seam are cfg-gated to target_os = \"windows\" (not left cross-platform) — the only consumer is Windows-only exec_strategy_windows::launch, so leaving it un-gated is dead code under -D warnings on Linux/macOS, discovered by the linux-gnu cross clippy gate."
  - "The gate decision compares log_target_is_private() at the point the downgrade warn fires (not cached earlier), so a future call site cannot forget to re-check it."
  - "render_error_for_operator() renders ONLY the ClearStaleLayerResidue remediation variant per WR-04's scope; other remediation() variants are deliberately left unrendered by this plan, matching the plan's acceptance criteria."

patterns-established:
  - "D-28 channel-privacy gating: query the actual selected tracing arm at the warn call site rather than assuming a mechanism-name-agnostic comment is still true."

requirements-completed: [CINT-02]

# Metrics
duration: 34min
completed: 2026-08-10
---

# Phase 117 Plan 21: Gate Downgrade-Warn Layer Detail on D-28 Channel Privacy + Wire WR-04 Remediation Summary

**Fixed a real D-28 information-disclosure gap (CR-02): the downgrade warn's specific `LayerId` names now only reach `tracing::warn!` when the active log target is a private file, not the default/fallback stderr arm the confined child can read; also wired `NonoError::remediation()`'s existing `ClearStaleLayerResidue` guidance into `main.rs`'s printed output (WR-04).**

## Performance

- **Duration:** 34 min
- **Started:** 2026-08-10T20:52:00Z (approx, first Read call)
- **Completed:** 2026-08-10T21:14:55Z
- **Tasks:** 3 (Task 3 verification-only)
- **Files modified:** 4

## Accomplishments
- Added `cli_bootstrap::log_target_is_private()`, a Windows-only query defaulting `false` (conservative) and flipping `true` only when `init_tracing_with_security`'s file-log-succeeded arm is selected.
- Gated `apply_startup_attestation_gate`'s `ProceedDowngraded` warn: specific `LayerId` names (`downgraded_layers` field) now only appear when the private channel is confirmed active; the shared-console arms get a `downgraded_count`-only message pointing at the audit ledger.
- Corrected the stale comment in `launch.rs` that falsely claimed `tracing::warn!` never reaches the confined child's stderr.
- Updated the D-27 banner text in `output.rs` to cite "the audit ledger" instead of "diagnostic output".
- Wired `NonoError::remediation()`'s `ClearStaleLayerResidue` variant into `main.rs`'s error path via a new, independently unit-tested `render_error_for_operator()` helper — an operator hitting a real `LayerAttestationFailed` now sees an actionable `nono setup --check-only` pointer, not just the bare `Display` line.
- Ran both mandated cross-target clippy gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`) per D-35/D-11, both clean after a structural cfg fix (see Deviations).

## Task Commits

1. **Task 1: Add a private-log-channel query to cli_bootstrap.rs** - `5fa52bc7` (feat)
2. **Task 2: Gate the downgrade warn's layer-name detail on the private channel; correct the banner text; wire WR-04's remediation** - `751ab7af` (fix)
3. **Cross-target structural fix discovered by Task 3 verification** - `e082745d` (fix) — see Deviations

**Plan metadata:** committed alongside this SUMMARY (see final commit below)

## Files Created/Modified
- `crates/nono-cli/src/cli_bootstrap.rs` - Adds `TRACING_LOG_TARGET_IS_PRIVATE` (Windows-only), `log_target_is_private()`, and test seams (`set_log_target_is_private_for_test`, a process-global test lock)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - Gates the downgrade warn's `downgraded_layers` field on `log_target_is_private()`; corrects the stale D-28 comment; renames and splits the NR3-04 test into private-channel and shared-channel variants
- `crates/nono-cli/src/output.rs` - Corrects the D-27 banner's channel-pointer text
- `crates/nono-cli/src/main.rs` - Adds `render_error_for_operator()` and wires it into `main()`'s generic error branch; adds two unit tests

## Decisions Made
- Kept `log_target_is_private()` Windows-only (`#[cfg(target_os = "windows")]`) rather than making it cross-platform-safe-but-unused, since its only real consumer (`exec_strategy_windows::launch`) is itself Windows-only. This mirrors the existing `output::print_attestation_downgrade_banner` Windows-only gating pattern and keeps the codebase's cross-target clippy gate meaningful (no `#[allow(dead_code)]`).
- `render_error_for_operator()` prints the `Display` line unconditionally, then appends a second line ONLY for `ClearStaleLayerResidue` — other `NonoRemediation` variants are intentionally left unrendered by this plan (WR-04's stated scope), avoiding unplanned UX scope creep.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test flakiness from unsynchronized global static across parallel test threads**
- **Found during:** Task 2 (running the two new downgrade-warn tests together)
- **Issue:** `TRACING_LOG_TARGET_IS_PRIVATE` is a shared, process-global `AtomicBool`. Rust runs unit tests in parallel within the same process; the private-channel and shared-channel tests set opposite values on this static concurrently, causing one test to observe the other's write mid-assertion (reproduced: `proceed_downgraded_success_path_logs_downgraded_layers_field_on_private_log_channel` failed intermittently with `downgraded_count`-only output even though it had set `private=true`).
- **Fix:** Added a `#[cfg(test)]` process-global `Mutex<()>` (`LOG_TARGET_IS_PRIVATE_TEST_LOCK`) plus a `lock_log_target_is_private_test()` accessor (poison-recovering, mirroring `test_env::lock_env()`), acquired for the full duration of both tests.
- **Files modified:** crates/nono-cli/src/cli_bootstrap.rs, crates/nono-cli/src/exec_strategy_windows/launch.rs
- **Verification:** `cargo test -p nono-sandbox-cli --bin nono -- proceed_downgraded` passes reliably (multiple repeated runs, both orderings).
- **Committed in:** `751ab7af` (Task 2 commit)

**2. [Rule 1 - Bug] Cross-target dead-code error surfaced by the mandated linux-gnu clippy gate**
- **Found during:** Task 3 (cross-target clippy verification, per plan's explicit instruction to run both gates)
- **Issue:** `log_target_is_private()` and its backing static were originally left cross-platform (not cfg-gated), but their only consumer, `exec_strategy_windows::launch::apply_startup_attestation_gate`, is itself compiled Windows-only. `cross clippy --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` failed with `error: function 'log_target_is_private' is never used` (`-D dead-code` implied by `-D warnings`).
- **Fix:** cfg-gated the static, the query fn, both test seams, and the single `.store(true, ...)` call site inside the shared `init_tracing_with_security` function to `#[cfg(target_os = "windows")]` (test seams: `#[cfg(all(test, target_os = "windows"))]`) — mirroring the existing `output::print_attestation_downgrade_banner` Windows-only gating for the same CINT-02 reason. No `#[allow(dead_code)]` used (forbidden by the checklist's Anti-pattern 2).
- **Files modified:** crates/nono-cli/src/cli_bootstrap.rs
- **Verification:** Re-ran `cargo build`/`clippy`/`fmt --check` on the Windows host (clean), then both cross-target gates (see below) — both clean.
- **Committed in:** `e082745d`

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs — one test-flakiness fix, one cross-target structural fix)
**Impact on plan:** Both fixes were necessary for correctness (test reliability) and for satisfying the plan's own Task 3 acceptance criteria (both cross-target clippy gates clean). No scope creep — no behavior change to the D-28/WR-04 fixes themselves.

## Issues Encountered
None beyond the two auto-fixed deviations above.

## Cross-Target Clippy Verification (Task 3)

Per CLAUDE.md D-35/D-11 and `.planning/templates/cross-target-verify-checklist.md`, both gates were run locally (not deferred to CI):

- **linux-gnu:** `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **clean** (0 errors, 0 warnings) after the cfg fix above. Docker engine confirmed up (`Server Version: 29.6.2`) before running; pinned image `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5`.
- **apple-darwin:** `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (direct-binary form, `SDKROOT` unset) — **clean** (0 errors, 0 warnings).
- **Windows host (all-targets, standard gate):** `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean. `cargo fmt --check` — clean.

No PARTIAL→CI fallback needed — both gates ran to completion with a genuine result.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CR-02 (D-28 violation) and WR-04 (invisible remediation) are both closed at the code level; the plan's `must_haves` truths (private-channel gating, shared-channel withholding, operator-actionable remediation) are all covered by passing, order-independent unit tests.
- No blockers for subsequent 117-series plans. This plan's changes are additive/gating-only and do not alter `apply_startup_attestation_gate`'s decision logic (`Proceed`/`Abort`/`ProceedDowngraded`), only what is logged and printed.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*
