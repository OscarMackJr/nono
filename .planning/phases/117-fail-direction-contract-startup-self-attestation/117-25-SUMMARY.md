---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 25
subsystem: infra
tags: [ci-yml, telemetry, hmac-chain, security-event-layer, windows]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "Plan 117-17's SecurityEventLayerInner pub(crate) widening + emit_attestation_event relocation (the state this plan reverts/narrows), and Plan 117-18's windows-layer-fault-injection job (the state this plan trims)"
provides:
  - "windows-layer-fault-injection CI job with the dead NONO_CI_HAS_WFP env block removed (WR-05 closed)"
  - "SecurityEventLayer::advance_and_snapshot — the sole cross-module HMAC-chain-advancement accessor"
  - "SecurityEventLayerInner's chain/session_id/config fields private again (WR-09 closed)"
affects: [117-review, telemetry, ci]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Snapshot-accessor pattern for cross-file access to a module-private Mutex-guarded struct: lock once inside the owning module, return an owned (String, String, bool) tuple instead of exposing the fields."

key-files:
  created: []
  modified:
    - .github/workflows/ci.yml
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs

key-decisions:
  - "Task 1 scoped the env-block removal to the windows-layer-fault-injection job only, not the Windows Security job that carries the identical-looking NONO_CI_HAS_WFP block — the latter is a genuine, load-bearing consumer (windows-test-harness.ps1 line 189 gates the WFP-filtered test subset on it), so removing it there would silently skip real CI coverage. The plan's acceptance-criteria grep implied a file-wide zero-match outcome; followed the task's own scoped <action> text and the WR-05 finding's actual claim (this job's block, specifically) instead, since the literal acceptance-criteria reading would have caused a real regression."
  - "advance_and_snapshot has zero reachable callers inside nono-agentd's independent #[path]-copy of telemetry/mod.rs (its only caller, attestation_downgrade_event.rs, is Windows-exec-strategy-only and never #[path]-included there) — same multi-binary compilation artifact the codebase already carries #[allow(dead_code)] + justification comment for on emit_override_event. Applied the identical, already-established pattern rather than reintroducing pub(crate) fields or duplicating the accessor."

patterns-established: []

requirements-completed: [CINT-02, CINT-03]

# Metrics
duration: 34min
completed: 2026-08-11
---

# Phase 117 Plan 25: Close WR-05 (dead CI env block) + WR-09 (HMAC chain field exposure) Summary

**Removed a CI job's inert `NONO_CI_HAS_WFP` env block and reverted `SecurityEventLayerInner`'s `chain`/`session_id`/`config` fields to private behind a new `advance_and_snapshot` accessor, closing two iteration-4 code-review findings without changing any externally-observable behavior.**

## Performance

- **Duration:** 34 min
- **Started:** 2026-08-11T01:49:26Z
- **Completed:** 2026-08-11T02:23:00Z (approx)
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- WR-05 closed: the `windows-layer-fault-injection` job's dead `NONO_CI_HAS_WFP` env block (zero consumers — that job never calls `windows-test-harness.ps1`) is removed; `.github/workflows/ci.yml` still parses as valid YAML.
- WR-09 closed: `SecurityEventLayerInner`'s `chain`/`session_id`/`config` fields are private again; the only way any module outside `telemetry::mod` can advance the tamper-evident HMAC chain or read session/config state is `SecurityEventLayer::advance_and_snapshot`, which always routes the mutation through `advance_chain`.
- `emit_attestation_event` rewritten to call `advance_and_snapshot` instead of locking `inner` and touching its fields directly; all 3 pre-existing tests pass unchanged with byte-identical assertions.

## Task Commits

1. **Task 1: Remove the dead NONO_CI_HAS_WFP env block from ci.yml** - `05a823cf` (fix)
2. **Task 2: Narrow SecurityEventLayerInner back to private fields behind a chain-advancement accessor** - `34a57579` (fix)

**Plan metadata:** (this commit, to follow)

## Files Created/Modified
- `.github/workflows/ci.yml` - removed the `env: NONO_CI_HAS_WFP: true` block from the `windows-layer-fault-injection` job only; the `Windows Security` job's identical-looking block (a genuine consumer) is untouched
- `crates/nono-cli/src/telemetry/mod.rs` - `SecurityEventLayerInner.chain`/`.session_id`/`.config` reverted to private; added `SecurityEventLayer::advance_and_snapshot` (`#[allow(dead_code)]` + justification, matching `emit_override_event`'s existing precedent for the same multi-binary-compile-unit issue)
- `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` - `emit_attestation_event` rewritten to call `advance_and_snapshot`; module doc comment and a stale `inner.config.enabled` doc reference updated to match

## Decisions Made
- **Task 1 scope:** followed the task's `<action>` text (scoped to the `windows-layer-fault-injection` job) over a literal reading of the acceptance-criteria grep (which implied removing every `NONO_CI_HAS_WFP` occurrence file-wide). The `Windows Security` job's identical block is a real, load-bearing consumer — `scripts/windows-test-harness.ps1` line 189 checks `$env:NONO_CI_HAS_WFP -eq 'true'` to decide whether to run the WFP-filtered subset of its `security` suite. Removing it there would have silently disabled that CI coverage, contradicting the plan's own stated "no runtime behavior change" scope for this task.
- **Task 2 dead-code handling:** `advance_and_snapshot`'s only caller (`attestation_downgrade_event.rs`) is gated under `exec_strategy_windows/`, which is Windows-`nono`-binary-only and never `#[path]`-included by `nono-agentd.rs`. This reproduces the exact multi-binary dead_code problem the plan's own `<interfaces>` block documents for `emit_attestation_event`'s original relocation. Rather than reintroducing `pub(crate)` fields (which would undo WR-09) or duplicating the accessor per-binary, applied `#[allow(dead_code)]` with a justification comment — the identical pattern the codebase already carries on `emit_override_event` a few lines above for the same reason.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Task 1 acceptance criteria would have caused a real CI regression if followed literally**
- **Found during:** Task 1
- **Issue:** The plan's acceptance criteria states `grep -n "NONO_CI_HAS_WFP" .github/workflows/ci.yml` should return zero matches, but the file has two occurrences of that env block — one in `windows-layer-fault-injection` (genuinely dead, the WR-05 target) and one in `windows-security` (genuinely live — gates WFP test execution in `windows-test-harness.ps1`). Removing both would silently disable the WFP-filtered subset of the Windows Security CI suite.
- **Fix:** Removed only the `windows-layer-fault-injection` job's block, per the task's own `<action>` text and the WR-05 finding's actual scope. Left the `Windows Security` job's block untouched.
- **Files modified:** `.github/workflows/ci.yml`
- **Verification:** `grep -n "NONO_CI_HAS_WFP" .github/workflows/ci.yml` now shows exactly one remaining occurrence (the live one in the Windows Security job); `python -c "import yaml; yaml.safe_load(...)"` confirms valid YAML.
- **Committed in:** `05a823cf`

**2. [Rule 3 - Blocking] advance_and_snapshot triggered a -D warnings dead_code build failure on the nono-agentd binary**
- **Found during:** Task 2
- **Issue:** `cargo clippy -p nono-sandbox-cli --bins -- -D warnings` failed with `error: method 'advance_and_snapshot' is never used` for the `nono-agentd` binary target — its independent `#[path]`-copy of `telemetry/mod.rs` never reaches `attestation_downgrade_event.rs` (Windows-`exec_strategy_windows`-only, `nono`-binary-only), so the new method has zero reachable callers in that compilation unit.
- **Fix:** Added `#[allow(dead_code)]` with a justification comment mirroring the existing precedent already on `emit_override_event` (same multi-binary compilation artifact, documented in that method's own comment).
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`
- **Verification:** `cargo clippy -p nono-sandbox-cli --bins -- -D warnings -D clippy::unwrap_used` clean; `cargo build -p nono-sandbox-cli --bin nono-agentd` succeeds; `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` clean.
- **Committed in:** `34a57579`

---

**Total deviations:** 2 auto-fixed (1 bug in plan acceptance criteria, 1 blocking build issue)
**Impact on plan:** Both deviations were necessary to avoid a real regression (CI coverage loss) and a build break, respectively. No scope creep — both fixes stayed within the two files/one job the plan targeted.

## Issues Encountered
None beyond the two deviations documented above.

## Cross-Target Verification

`crates/nono-cli/src/telemetry/mod.rs` is compiled unconditionally (not `#[cfg(windows)]` gated) into both the `nono` and `nono-agentd` binaries and is included cross-platform in `main.rs` (`pub(crate) mod telemetry;`, no cfg gate) — per this plan's `<cross_target_note>`, all three gates were run:

| Gate | Command | Result |
|------|---------|--------|
| Windows host | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | Clean |
| Windows host | `cargo fmt --check` | Clean |
| linux-gnu (cross/Docker) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | Clean |
| apple-darwin (cargo-zigbuild) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (SDKROOT unset) | Clean |
| Targeted test | `cargo test -p nono-sandbox-cli --bin nono attestation_downgrade_event` | 3/3 pass |
| Daemon build | `cargo build -p nono-sandbox-cli --bin nono-agentd` | Succeeds |
| YAML parse | `python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` | Succeeds |

`crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` is gated `#[cfg(target_os = "windows")]` at the module level (`exec_strategy_windows/mod.rs`), so it never compiles on the Unix cross targets — the linux-gnu/apple-darwin gates above exercise only the `telemetry/mod.rs` change from this plan.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- WR-05 and WR-09 both closed. No known follow-ups from this plan.
- The `Windows Security` job's genuine `NONO_CI_HAS_WFP` usage remains as-is and was explicitly out of scope for this plan (see Decisions Made).

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

All created/modified files verified present on disk; all task commit hashes (`05a823cf`, `34a57579`) and the summary commit (`ca34b9e5`) verified present in `git log`.
