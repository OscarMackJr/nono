---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 07
subsystem: security
tags: [windows, fail-direction, layer-fault-injection, cint-03, appcontainer, broker, daemon]

# Dependency graph
requires:
  - phase: 117 (waves 1-2, Plan 04, Plan 06)
    provides: "layer_registry.rs (LayerId rows), NonoError::LayerAttestationFailed, the layer-fault-injection Cargo feature on nono-sandbox-cli (117-04), and the paired-arms hook idiom shipped for restricted token/mandatory label/DACL grants/Job Object (117-06)"
provides:
  - "layer-fault-injection Cargo feature on nono-shell-broker (new [features] block; the crate had none)"
  - "force_app_container_unavailable() seam in nono-shell-broker/src/main.rs, short-circuiting run()'s SECURITY_CAPABILITIES construction to LayerAttestationFailed"
  - "force_daemon_app_container_unavailable() seam in agent_daemon/launch.rs, short-circuiting spawn_appcontainer_process_suspended's SECURITY_CAPABILITIES construction to LayerAttestationFailed"
  - "2 feature-gated regression tests proving each hook fires before the real Win32 SECURITY_CAPABILITIES/CreateProcessW construction"
affects: [117-12 (per-layer forced-unavailable tests consume these seams end-to-end)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Paired-arms force-unavailable seam (same idiom as Plan 06): #[cfg(feature = \"layer-fault-injection\")] static AtomicBool + pub(crate) setter + private getter + short-circuit Err at the top of the real construction step — purely additive, no #[cfg(not(...))] stub needed"

key-files:
  created: []
  modified:
    - crates/nono-shell-broker/Cargo.toml
    - crates/nono-shell-broker/src/main.rs
    - crates/nono-cli/src/agent_daemon/launch.rs

key-decisions:
  - "The broker hook sits after create_app_container_profile/derive_app_container_sid (real Win32 calls) but before SECURITY_CAPABILITIES construction, exactly per the plan's cited insertion point (main.rs:425-459 pre-edit line numbers) — this means the regression test genuinely registers a real per-run AppContainer profile on this host rather than mocking around it; if a future CI environment rejects AppContainer profile registration, the test distinguishes that (Err(SandboxInit), loud eprintln skip per D-31) from an actual seam failure (any other Err/Ok, which panics)"
  - "The daemon hook sits at the very top of spawn_appcontainer_process_suspended, before package_sid_psid (a raw PSID) is ever touched — lets the regression test pass a null PSID safely, since the short-circuit returns before any dereference"
  - "force_app_container_unavailable (broker) needed a scoped #[allow(dead_code)] with a documented rationale: nono-shell-broker has no [lib] target and no blanket #![allow(dead_code)] (unlike exec_strategy_windows/mod.rs, which is why the Plan 06 sibling setters never needed one) — the setter's only caller is the #[cfg(test)]-gated regression test, which cargo build/clippy without --tests never compiles, so dead-code analysis cannot see that usage; this is a lint false positive on cfg-gated test-support code, not genuinely-unused code (a real regression test does call it under --features layer-fault-injection --tests / cargo test)"
  - "The daemon regression test could not be placed adjacent to the hook in the same style as restricted_token.rs's own-file mod tests, because spawn_appcontainer_process_suspended is a private fn in agent_daemon/launch.rs's windows_impl submodule, only reachable from nested descendants — placed the test as a nested mod inside windows_impl itself (not the file's outer #[cfg(test)] mod tests, which is a sibling and cannot see private items), discovered via cargo test --bin nono-agentd (agent_daemon is only compiled into the nono-agentd binary target, not nono, per the module's own module-independence doc)"

patterns-established:
  - "clippy::items_after_test_module (only visible under --all-targets, not the plan's literal -p ... -- -D warnings command) requires feature-gated #[cfg(test)] mod blocks to sit after all non-test items in their enclosing module, not interleaved — moved both new test modules to the tail of their respective modules"

requirements-completed: [CINT-03]

# Metrics
duration: 55min
completed: 2026-08-10
---

# Phase 117 Plan 07: AppContainer Force-Unavailable Hooks (Broker + Daemon) Summary

**Added two independent, compiled-out `layer-fault-injection`-gated force-unavailable seams for the two structurally-separate places nono actually constructs `SECURITY_CAPABILITIES`/AppContainer — the Medium-IL `nono-shell-broker` binary and the daemon's own `spawn_appcontainer_process_suspended` — since neither is reachable from a hook on nono-cli's `exec_strategy_windows/` side alone.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `nono-shell-broker/Cargo.toml` now has a `[features]` block (the crate had none) with `layer-fault-injection = []`, default-off, mirroring `nono-sandbox-cli`'s Plan 04 feature name.
- The broker's own `run()` refuses to build `SECURITY_CAPABILITIES` and returns `NonoError::LayerAttestationFailed { layer: "AppContainerProfile", .. }` before `CreateProcessW`'s `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` attribute is ever set, when the seam is armed.
- The daemon's independent, structurally-separate `spawn_appcontainer_process_suspended` (module doc: intentionally does not depend on `exec_strategy_windows/`) gets the equivalent hook with the same error shape, before its own `CreateProcessW`.
- Both hooks are compiled out entirely (no static, no setter symbol) in a default build; verified via `cargo build`/`cargo clippy -D warnings -D clippy::unwrap_used` on both feature sets for both crates, plus a full `cargo clippy --workspace --all-targets` pass and `cargo fmt --all -- --check`.
- Both regression tests pass for real (not via the environment-limited skip path) on this host: `run_fails_when_app_container_forced_unavailable` (broker, 24/24 tests) and `spawn_appcontainer_process_suspended_fails_when_forced_unavailable` (daemon, `nono-agentd` bin, 79/79 tests).

## Task Commits

Each task was committed atomically:

1. **Task 1: Add [features] block to nono-shell-broker** - `7bf44036` (chore)
2. **Task 2: AppContainer force-unavailable hooks — broker and daemon** - `a7df5c03` (feat)

**Plan metadata:** (this commit) `docs(117-07): complete AppContainer force-unavailable hooks plan`

## Files Created/Modified
- `crates/nono-shell-broker/Cargo.toml` - new `[features]` block, `layer-fault-injection = []`
- `crates/nono-shell-broker/src/main.rs` - `force_app_container_unavailable()` seam short-circuiting `run()`'s `SECURITY_CAPABILITIES` construction; `app_container_force_unavailable_tests` regression test module
- `crates/nono-cli/src/agent_daemon/launch.rs` - `force_daemon_app_container_unavailable()` seam short-circuiting `spawn_appcontainer_process_suspended`; `app_container_force_unavailable_tests` regression test module (nested inside `windows_impl`, since the hooked function is private and the file's outer `mod tests` is a sibling, not a descendant)

## Decisions Made
- Kept the exact D-29/D-30 paired-arms idiom from Plan 06 (`static AtomicBool` + `pub(crate)` setter + private getter + top-of-function `Err` short-circuit) for both hooks, per the plan's explicit instruction to mirror it.
- Placed both checks exactly where the plan's `<interfaces>` section cited them (broker: before `main.rs:425-459`'s `SECURITY_CAPABILITIES` construction; daemon: before `spawn_appcontainer_process_suspended`'s own construction) rather than earlier (e.g., before profile registration), even though this means the broker's regression test performs a real `CreateAppContainerProfile`/`derive_app_container_sid` round-trip before the seam fires.
- Added a scoped, documented `#[allow(dead_code)]` on the broker's setter function (see key-decisions above) rather than a blanket module-level allow — CLAUDE.md discourages lazy `#[allow(dead_code)]`, but this is a narrow, cited exception for a function a real regression test does call, gated the same way the codebase already accepts for `#[cfg(feature = ...)]` test-support code (`exec_strategy_windows/mod.rs`'s pre-existing `#![allow(dead_code)]` shields the Plan 06 siblings from the identical situation; the broker crate has no such blanket allow to lean on).
- Named both hooks' error layer `"AppContainerProfile"`, matching the plan's `<action>` text for the daemon hook and kept consistent for the broker hook (RESEARCH's single AppContainer registry row covers both arms per D-08's per-arm expectancy matrix — Plan 12 disambiguates by call site, not by layer name).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `clippy::items_after_test_module` under `--all-targets`**
- **Found during:** Task 2, post-implementation verification pass (`cargo clippy -p nono-sandbox-cli -p nono-shell-broker --all-targets --features ... -- -D warnings`)
- **Issue:** Both new `#[cfg(test)]` regression-test modules were placed immediately after the function they test but before other, later non-test items in the same module (`build_command_line`/`generate_tenant_id` in `agent_daemon/launch.rs`), which `clippy::items_after_test_module` rejects. The plan's own literal acceptance-criteria command (`-p ... -- -D warnings`, no `--all-targets`) does not compile test code at all so would not have caught this; found via the stricter `--all-targets` pass this executor ran as part of the repo's `make ci`-equivalent verify gate (CLAUDE.md § cross-target/verify discipline).
- **Fix:** Moved the daemon's test module to the tail of `windows_impl` (after `generate_tenant_id`, the module's last item). The broker's test module was already correctly positioned (last item before `mod broker`'s closing brace), so only `launch.rs` needed the move.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Commit:** `a7df5c03` (fixed before commit, not a separate follow-up commit)

## Issues Encountered
- `cargo build`/plain `cargo clippy -p nono-sandbox-cli --features layer-fault-injection` (no `--tests`/`--all-targets`) reported **zero** dead-code warnings for Plan 06's structurally-identical unused-outside-test setter functions (`force_restricted_token_unavailable` et al.), while the broker's analogous new setter WAS flagged under the identical command shape. Root-caused via a `compile_error!` probe (confirming the feature really was active) and a `pub`-vs-`pub(crate)` visibility experiment (no difference) before finding the real cause: `exec_strategy_windows/mod.rs:13` carries a pre-existing `#![allow(dead_code)]` that blankets the whole module tree (and `agent_daemon/launch.rs`'s `windows_impl` mod has its own `#[allow(dead_code)]` at line 64) — `nono-shell-broker` has neither shield. No plan or production-code change needed beyond the scoped `#[allow(dead_code)]` documented above; this was purely an investigation into why the same idiom behaves differently across two crates, not a bug in either.
- `agent_daemon` is only compiled into the `nono-agentd` binary target (not `nono`) per the module's own "module independence" doc comment — the daemon regression test only appears under `cargo test -p nono-sandbox-cli --bin nono-agentd`, not `--bin nono`. Confirmed by listing tests with `-- --list` after an initial `0 tests` result on the wrong bin target.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Both independent AppContainer construction sites (broker, daemon) now have a compiled-out force-unavailable hook, matching this plan's stated success criterion.
- Combined with Plan 06's four CLI-side layers, every registry-row construction site named in the plan family's scope now has a seam Plan 12's per-layer CINT-03 tests can drive end-to-end through the real launch path (broker via a real spawned Medium-IL process, daemon via `nono-agentd`'s own launch path).
- `crates/nono-cli/tests/layer_registry_selfcheck.rs`'s `registry_call_sites_exist`/`spec_matches_registry` tests re-ran clean (file-presence checks only, unaffected by inserting hook code ahead of the hooked functions — same caveat Plan 06's SUMMARY already flagged for future tightening, not reopened here).

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*

## Self-Check: PASSED

All 3 modified source files and the SUMMARY.md itself confirmed present on disk; commits `7bf44036` and `a7df5c03` confirmed in `git log --oneline --all`.
