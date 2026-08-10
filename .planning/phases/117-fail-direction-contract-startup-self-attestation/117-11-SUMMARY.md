---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 11
subsystem: security
tags: [windows, appcontainer, attestation, broker, fail-direction, cint-02]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "layer registry (Plan 01), LayerAttestationStatus + raw OS probes (Plan 05), nono-cli's CLI-side attest_and_decide + BROKER_REQUIRED_LAYERS_ENV_VAR wire contract (Plan 08), nono-cli setting that env var when spawning the broker (Plan 10), the layer-fault-injection feature seam (Plan 07)"
provides:
  - "the third and final D-21 gate insertion site: a probe-then-decide check inside nono-shell-broker's own suspended-spawn window, immediately before its own ResumeThread call"
  - "the genuine AppContainerProfile attestation on the broker arm (Blocker-1 closure) — the only site in the phase that observes the REAL confined child, since it is spawned inside the broker's own process"
  - "app_container_resume_gate: a pure, unit-testable decision function mirroring exec_strategy_windows/attestation.rs's decide_from_entries pattern"
affects: [117-12, 118-per-session-receipts]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure decision function factored out of an unsafe FFI-heavy run() body for unit-testability without a live spawned process (mirrors decide_from_entries in exec_strategy_windows/attestation.rs)"
    - "Crate-local EnvVarGuard (with #[allow(clippy::disallowed_methods)]) as the sanctioned safe wrapper around std::env::set_var/remove_var when a crate cannot depend on nono-cli's shared crate::test_env::EnvVarGuard (binary-only crate, no [lib] target)"

key-files:
  created: []
  modified:
    - crates/nono-shell-broker/src/main.rs

key-decisions:
  - "Reused the existing TerminateProcess-then-Err idiom (already used 3 times in the same is_app_container block) as a 4th instance rather than inventing new control flow, per the plan's threat register (T-117-05)."
  - "Kept NonoError::SandboxInit (not LayerAttestationFailed) for this failure, matching the file's 3 existing failure branches in the same function — a deliberate consistency choice per the plan's explicit instruction, not an oversight."
  - "Factored the gate's decision logic into a standalone pure function (app_container_resume_gate) instead of testing only through a full live run() spawn, since forcing a REAL spawned-and-suspended AppContainer child's probe to disagree with its own successful construction is not expressible without faking the OS — see Deviations."
  - "Added #[allow(clippy::disallowed_methods)] on the crate-local EnvVarGuard's impl blocks, mirroring nono-cli's own test_env.rs idiom, since nono-shell-broker cannot depend on nono-cli (binary-only crate, no [lib] target) to reuse its shared guard."

requirements-completed: [CINT-02]

# Metrics
duration: 30min
completed: 2026-08-10
---

# Phase 117 Plan 11: Broker-Side Resume Gate Summary

**Added a probe-then-decide AppContainer attestation gate immediately before `nono-shell-broker`'s own `ResumeThread` call, reading nono-cli's `NONO_BROKER_REQUIRED_LAYERS` wire contract and probing the real suspended child's token via `nono::attestation::probe_app_container_sid`.**

This closes the third and final D-21 gate insertion site named in the phase's `<threat_model>`: the broker's own suspended-spawn window for the real Low-IL/AppContainer child — the site RESEARCH finding 2 identifies as structurally invisible to `nono-cli`'s own `attest_and_decide` (Plan 08/10), since the broker spawns this child inside its own process after `nono-cli`'s own spawn call has already returned. Per Blocker-1, this is also the ONLY place in the phase that genuinely attests `AppContainerProfile` on the broker arms.

## Performance

- **Duration:** ~30 min
- **Completed:** 2026-08-10
- **Tasks:** 1/1 completed
- **Files modified:** 1

## Accomplishments

- Extended the broker's existing fail-closed label-apply gate (`main.rs:610-661` pre-change) with a fourth `TerminateProcess`-then-`Err` check, immediately before `ResumeThread`, reading `NONO_BROKER_REQUIRED_LAYERS` and probing the child's real token.
- D-02 enforced: an absent or unrelated `NONO_BROKER_REQUIRED_LAYERS` value never aborts — the broker has no policy authority to require a layer nono-cli did not explicitly ask it to require.
- D-19 enforced: the probe is dispatched against the broker's own suspended child's process handle from outside; the confined process is never a source of any claim about its own containment.
- Decision logic factored into a pure `app_container_resume_gate(required_layers_raw, probe_result) -> Result<(), String>` function, unit-tested against 4 synthetic scenarios (untightened+unconfirmed permits, tightened+unconfirmed refuses via both `Ok(None)` and `Err`, tightened+confirmed permits, comma-split/whitespace-tolerant parsing).
- Added a crate-local, Drop-restoring `EnvVarGuard` and two tests exercising the real `NONO_BROKER_REQUIRED_LAYERS` env-var read path (set-then-tighten; unset-then-not-tighten), satisfying CLAUDE.md's env-var save/restore rule (Warning-10 fix) and the plan's `impl Drop for` acceptance check.
- Full workspace `cargo check --workspace --all-targets`, `cargo fmt --all -- --check`, and `cargo test -p nono-shell-broker --features layer-fault-injection -- --test-threads=1` (30/30 passing, including all 5 pre-existing test modules) all verified clean, with and without `layer-fault-injection`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Broker-side probe-then-decide gate before ResumeThread** - `47017085` (feat)

_No separate plan-metadata commit — SUMMARY.md and this doc commit are combined per this project's `sequential_execution` protocol (no STATE.md/ROADMAP.md writes; orchestrator owns those)._

## Files Created/Modified

- `crates/nono-shell-broker/src/main.rs` - Added `app_container_resume_gate` (pure decision function), the gate call site inside `run()`'s `if is_app_container` block immediately before `ResumeThread`, a crate-local `EnvVarGuard` RAII helper, and 3 new test modules (`app_container_resume_gate_tests`, `env_wire_contract_tests`) covering the D-02/D-21/D-22 behaviors.

## Decisions Made

- **Error type consistency over introducing a second shape:** used `NonoError::SandboxInit` for the new failure, matching this file's 3 existing failure branches in the same `if is_app_container` block, per the plan's explicit instruction to keep this a deliberate consistency choice rather than switching to `LayerAttestationFailed` (whose constructor is available but would introduce a second error shape for the same logical event inside one function).
- **Pure-function extraction for testability:** rather than inline the probe-and-decide logic directly at the call site, factored it into `app_container_resume_gate` — mirrors the CLI-side `attest_and_decide`/`decide_from_entries` split in `crates/nono-cli/src/exec_strategy_windows/attestation.rs`, and is what makes the Behavior tests possible without a live spawned child (see Deviations).
- **Crate-local `EnvVarGuard` with an explicit `#[allow(clippy::disallowed_methods)]`:** the workspace `clippy.toml` disallows bare `std::env::set_var`/`remove_var` and points callers at `crate::test_env::EnvVarGuard` in `nono-cli` — unusable here since `nono-cli` is a binary-only crate with no `[lib]` target that `nono-shell-broker` could depend on. Mirrored `nono-cli`'s own `#[allow(clippy::disallowed_methods)] // This IS the safe wrapper` idiom on this crate's local equivalent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `#[allow(clippy::disallowed_methods)]` to the new `EnvVarGuard`**
- **Found during:** Task 1 verification (`cargo clippy --all-targets`)
- **Issue:** The workspace `clippy.toml` disallows bare `std::env::set_var`/`remove_var` calls anywhere in the tree (message: "use `crate::test_env::EnvVarGuard` instead"). My new crate-local `EnvVarGuard`'s `set`/`unset`/`Drop` implementations tripped this lint, since `nono-shell-broker` cannot import `nono-cli`'s shared guard (binary-only crate, no `[lib]` target — the same crate-boundary constraint documented throughout this phase for `BROKER_REQUIRED_LAYERS_ENV_VAR`).
- **Fix:** Added `#[allow(clippy::disallowed_methods)]` on the `EnvVarGuard` `impl` blocks, with a doc comment explaining the crate-boundary reason and citing `nono-cli`'s own identical idiom in `crates/nono-cli/src/test_env.rs` as precedent.
- **Files modified:** `crates/nono-shell-broker/src/main.rs`
- **Verification:** `cargo clippy -p nono-shell-broker --all-targets [--features layer-fault-injection] -- -D warnings -D clippy::unwrap_used` exits 0, both with and without the feature.
- **Committed in:** `47017085` (Task 1 commit)

**2. [Interpretation adjustment, not a Rule 1-4 deviation] Behavior tests adapted from a live-spawn description to a pure-function + real-env-var-read design**
- **Found during:** Task 1 test design
- **Issue:** The plan's `<behavior>` text describes the two Behavior tests in terms of a live spawned-and-suspended child ("the broker terminates the suspended child... a subsequent IsProcessInJob-style liveness check on the (now-terminated) child handle") whose AppContainer probe is forced to fail "via Plan 07's force-unavailable hook". In the actual code, Plan 07's existing `APP_CONTAINER_FORCE_UNAVAILABLE` seam fires during `SECURITY_CAPABILITIES`/attribute-list construction, strictly *before* `CreateProcessW` is ever called — arming it prevents the child from ever being spawned at all, so it cannot be used to make a *successfully spawned* suspended child's *post-spawn* probe (this plan's new gate, which runs after spawn and after the label is applied) read as unconfirmed. Forcing a real, successfully-constructed AppContainer child's live `TokenAppContainerSid` query to disagree with its own construction is not expressible without faking the OS call itself.
- **Fix:** Extracted the gate's policy into `app_container_resume_gate` (pure function over a caller-supplied probe `Result`), tested directly against synthetic `Ok(Some(_))`/`Ok(None)`/`Err(_)` probe outcomes — this exercises the exact decision logic `run()` calls, without needing to fake OS behavior. Separately, `env_wire_contract_tests` exercises the *real* `std::env::var("NONO_BROKER_REQUIRED_LAYERS")` read (via the Drop-restoring `EnvVarGuard`, satisfying the plan's literal Warning-10/`impl Drop for` requirement) feeding into the same pure function. This is the same pattern the CLI-side `attest_and_decide`/`decide_from_entries` split already uses for its own equivalent policy tests, none of which use a live `ProcessHandle` either.
- **Files modified:** `crates/nono-shell-broker/src/main.rs`
- **Verification:** `cargo test -p nono-shell-broker --features layer-fault-injection -- --test-threads=1` — 30/30 passing, including 4 new `app_container_resume_gate_tests` and 2 new `env_wire_contract_tests`, plus the pre-existing `run_fails_when_app_container_forced_unavailable` (Plan 07's own live-spawn test) unaffected.
- **Committed in:** `47017085` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking-clippy), 1 documented interpretation adjustment (test-design, not a Rule 1-4 code fix).
**Impact on plan:** No scope creep. The clippy fix was required to pass the plan's own acceptance criteria. The test-design adjustment preserves the exact security-relevant behavior under test (D-02/D-21/D-22) while working within the actual capabilities of the existing `layer-fault-injection` seam.

## Issues Encountered

None beyond the deviations documented above.

## User Setup Required

None - no external service configuration required.

## Threat Flags

None - this plan's only new surface (the broker's own probe-then-decide gate) is exactly the mitigation the phase's `<threat_model>` assigned to this plan's files (T-117-05, T-117-02, T-117-21, T-117-23), all `mitigate`-dispositioned and addressed as documented above.

## Known Stubs

None. The gate is fully wired: real env-var read, real `nono::attestation::probe_app_container_sid` call against the real suspended child, real `TerminateProcess`-then-typed-`Err` on refusal.

## Next Phase Readiness

- All three independent spawn implementations named in D-23 (`launch.rs` direct CLI path — Plan 09, `agent_daemon/launch.rs` daemon path — Plan 10, `nono-shell-broker/main.rs` broker path — this plan) are now gated, closing RESEARCH finding 1's enumeration.
- Blocker-1 is fully closed: `AppContainerProfile` is genuinely attested on a broker arm, on the real spawned process, for the first time in the phase.
- `proj/SPEC-windows-fail-direction-contract.md`'s existing `nono-shell-broker/src/main.rs` line citations (e.g. `:615-644`, `:322-336`, `:536-556`) have shifted due to this plan's ~260 added lines; this is expected and out of this plan's scope (`files_modified` is `main.rs` only) — the SPEC's own Broker-arm latency row is already explicitly marked `TBD — measured in Plan 117-12`, so Plan 12 is the intended place to refresh these citations alongside the latency measurement. `layer_registry_selfcheck.rs`'s `registry_call_sites_exist` test only validates file existence, not line-range accuracy, so this does not fail CI in the interim.
- Ready for Plan 12 (latency budget measurement, D-24) and for the phase's overall SC2 (every confined-child path gated) sign-off.

## Self-Check: PASSED

- FOUND: `crates/nono-shell-broker/src/main.rs`
- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-11-SUMMARY.md`
- FOUND: commit `47017085` (Task 1: feat)
- FOUND: commit `24479817` (docs: SUMMARY)
- CONFIRMED: `.planning/STATE.md` and `.planning/ROADMAP.md` untouched by this plan's commits

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*
