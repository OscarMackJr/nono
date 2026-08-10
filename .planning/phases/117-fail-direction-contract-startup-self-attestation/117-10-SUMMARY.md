---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 10
subsystem: infra
tags: [windows, sandboxing, attestation, job-object, appcontainer, telemetry, security]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "layer_registry (Plan 01), NonoError::LayerAttestationFailed (Plan 02), attest_and_decide()/AttestationDecision (Plan 08), SecurityEvent::LayerAttestationDowngraded + print_attestation_downgrade_banner (Plan 09)"
provides:
  - "D-21 attestation gate wired into launch.rs's spawn_windows_child (direct nono run, hook path, broker arms up to broker.exe's own spawn)"
  - "D-21 attestation gate wired into agent_daemon/launch.rs's launch_agent as an independent 'step 6.7' mirror (daemon nono agent launch)"
  - "NONO_BROKER_REQUIRED_LAYERS env var set on broker spawn (Blocker-1 wire contract)"
  - "SC4-2 struct-field drop-order comment corrected on PreparedWindowsLaunch"
  - "session_id + sorted dedup_key threaded into print_attestation_downgrade_banner at both banner call sites (Item-2 fix)"
affects: [118-per-session-receipts, 119-security-model-boundary]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "apply_startup_attestation_gate(): factor the post-spawn D-21 decision handling out of spawn_windows_child into a small function, unit-testable against a real (never-resumed) suspended child without invoking the full spawn cascade"
    - "AUD-04 non-fatal contract: a failed audit-event emission never blocks an already-downgraded launch; only Abort-outcome and attest_and_decide's own validation Err block the launch"
    - "Independent per-binary mirror: a policy decision function structurally unreachable across a #[path]-included, no-[lib]-crate binary boundary is reimplemented locally rather than shared, with an explicit doc-comment cross-reference and discrepancy note"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/agent_daemon/launch.rs
    - crates/nono-cli/src/telemetry/mod.rs

key-decisions:
  - "agent_daemon/launch.rs's D-21 gate is a genuinely independent reimplementation (daemon_attest_and_decide), not a call into exec_strategy_windows::attestation::attest_and_decide — nono-agentd is a separate binary crate (src/bin/nono-agentd.rs) that never #[path]-includes exec_strategy_windows, so that module is structurally unreachable from agent_daemon code"
  - "The daemon's D-27 human-visible channel uses tracing::warn! (Windows Event Log), not exec_strategy_windows::output::print_attestation_downgrade_banner — that module is equally unreachable from nono-agentd, and a SERVICE_USER_OWN_PROCESS service has no attached console for an stderr banner anyway"
  - "A failed D-27 audit-event emission (SecurityEventLayer.get() returning None, or emit_attestation_event returning Err) is non-fatal to the launch decision at both gate sites, per emit_attestation_event's own AUD-04 doc-comment contract — this differs from execution_runtime.rs's PolicyOverrideVerified emission, which does abort on Err, because that call site guards a request with nothing to lose by refusing while these gates guard an already-downgraded launch that was already proceeding"
  - "D-26's tighten-only union (CLI flag / machine-policy required-layers) is not wired at either gate site in this plan — both pass empty slices; every row's own per-row default outcome (overwhelmingly Abort) still governs the decision, so this is a deferred feature, not a fail-open gap"
  - "The daemon's independent mirror does not model WfpEgressFilters at all: by the time step 6.7 runs, the WFP concern is already fully resolved by existing fail-closed code earlier in the same function (either scoping was not required, or it was required and already succeeded) — re-modeling it would check a condition the call site has already made structurally impossible to fail"

requirements-completed: [CINT-02]

# Metrics
duration: 95min
completed: 2026-08-10
---

# Phase 117 Plan 10: D-21 Attestation Gate Wiring (Direct CLI + Daemon) Summary

**Wired the startup self-attestation gate into both nono-cli-crate spawn sites — `launch.rs`'s `spawn_windows_child` and `agent_daemon/launch.rs`'s `launch_agent` — so a suspended Windows child is probed and, on an unconfirmable required layer, terminated and never resumed, closing two of the three D-23 confined-child paths this crate directly controls.**

## Performance

- **Duration:** ~95 min (including three full environmental-corruption recovery cycles — see Issues Encountered)
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- `apply_startup_attestation_gate()` (new, `launch.rs`) inserted between `apply_resource_limits` and `resume_contained_process` in `spawn_windows_child` — gates direct `nono run`, the per-tool-call hook path (which re-enters this same code with no site of its own), and the `BrokerLaunch`/`BrokerLaunchNoPty` arms up to the point where they hand off to the broker's own suspended child.
- `daemon_attest_and_decide()` (new, `agent_daemon/launch.rs`) inserted as "step 6.7" immediately before `ResumeThread` in `launch_agent` — gates daemon `nono agent launch`. A genuinely independent implementation (not a shared call — see Deviations) mirroring the same Proceed / ProceedDowngraded / Abort decision shape using the shared `nono::attestation` probes.
- `NONO_BROKER_REQUIRED_LAYERS` env var set on the broker's environment block (only for `BrokerLaunch`/`BrokerLaunchNoPty`) from `attestation::required_layers_for_broker(layer_registry::all_entries())` — the Blocker-1 wire contract that lets the broker (Plan 11) know which layers it must itself confirm before resuming the real AppContainer-confined grandchild.
- SC4-2 fixed: `PreparedWindowsLaunch`'s field-comment claim of "reverse-of-declaration" struct-field drop order was factually wrong (Rust struct fields drop in forward declaration order; only local variables drop last-declared-first) — corrected in both the struct's own comments and the unrelated `execute_supervised` local-variable comment that legitimately uses the (correct, for locals) reverse-order claim, reworded to avoid ambiguity with the fixed struct comment.
- `spawn_windows_child` gained a `network_enforcement: Option<&NetworkEnforcementGuard>` parameter (Warning-5 fix); both `mod.rs` call sites now pass `prepared._network_enforcement.as_ref()`. `derive_wfp_preconfirmed()` factors the mapping into a small, unit-tested pure function.
- `_session_id` renamed to `session_id` (now genuinely used) and threaded into `print_attestation_downgrade_banner` at both gate sites with a sorted, comma-joined `LayerId` dedup key (Item-2 fix) — Plan 09's per-session dedup marker logic is now actually exercised in production, not merely built.
- `SecurityEventLayer::emit_attestation_event`'s stale `#[allow(dead_code)]` and "no call site yet" doc comment removed now that two real call sites exist in two separate binary crates.

## Task Commits

Each task was committed atomically:

1. **Task 1: D-21 gate in launch.rs's direct spawn + broker env-var wiring + SC4-2 comment fix** - `8820e3f4` (feat)
2. **Task 2: D-21 gate in agent_daemon/launch.rs** - `a11c09c8` (feat)

_No separate TDD RED/GREEN commits — both tasks are `type="auto" tdd="true"` but tests and implementation landed together per task, consistent with this plan's own scale (each task is a single cohesive gate-wiring change, not a standalone feature with an independently-meaningful failing-test state)._

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - `apply_startup_attestation_gate()`, `derive_wfp_preconfirmed()`, D-21 gate call site, broker env-var wiring, `network_enforcement`/`session_id` signature changes, 8 new unit tests
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - SC4-2 comment fix (both the struct comment and the unrelated local-variable comment), both `spawn_windows_child` call sites updated
- `crates/nono-cli/src/agent_daemon/launch.rs` - `DaemonAttestationDecision`, `daemon_attest_and_decide()`, step 6.7 gate insertion, `network_scoping_required` capture, 2 new unit tests
- `crates/nono-cli/src/telemetry/mod.rs` - removed stale `#[allow(dead_code)]`/doc comment on `emit_attestation_event` now that real call sites exist

## Decisions Made

See `key-decisions` in frontmatter above.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `agent_daemon/launch.rs`'s D-21 gate cannot literally call `exec_strategy_windows::attestation::attest_and_decide`**
- **Found during:** Task 2, before writing any code — re-reading the plan's own cited module-independence invariant (`agent_daemon/launch.rs:26-30`) against `nono-agentd.rs`'s actual `#[path]`-include list.
- **Issue:** `nono-agentd` is compiled as a separate binary crate (`crates/nono-cli` has no `[lib]` target — each `[[bin]]` is its own compilation unit). `src/bin/nono-agentd.rs` `#[path]`-includes only `agent_daemon/mod.rs`, `telemetry/mod.rs`, and `agent_daemon/telemetry_init.rs` — it never declares `exec_strategy_windows`. `attest_and_decide`, `AttestationInput`, and `layer_registry` are therefore not reachable symbols from `agent_daemon/launch.rs`, regardless of `pub(crate)` visibility (a different crate, not just a different module).
- **Fix:** Wrote `daemon_attest_and_decide()` as a genuinely independent reimplementation using only the shared, policy-free `nono::attestation` probe functions (`probe_app_container_sid`, `probe_in_job`) — the one dependency both binaries actually share via the `nono` library crate. It mirrors `attest_and_decide`'s decision shape (Proceed/ProceedDowngraded/Abort) for the subset of `layer_registry.rs` rows genuinely expected at `(EntryPath::Daemon, None)`: `AppContainerProfile` and `JobObjectContainment` (both independently re-probed), plus `DaclPackageSidGrant`/`DaclAncestorTraverse`/`DaclAncestorReadAttrs` (always downgraded once reached, per Plan 08's own documented Deviation 2 for `ConfiguredOnly`-probed rows). `WfpEgressFilters` is deliberately not modeled (see decision above and the function's own doc comment for the full discrepancy note against the shared registry's unconditional Daemon-row expectancy).
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** `cargo test -p nono-sandbox-cli --bin nono-agentd --features layer-fault-injection` — 87/87 pass, including 2 new tests (`null_handle_aborts_on_app_container_profile`, `real_appcontainer_job_process_proceeds_downgraded` — the latter spawns a real AppContainer profile + job-assigned suspended process to deterministically exercise both live probes).
- **Committed in:** `a11c09c8`

**2. [Rule 1 - Bug] The plan's literal D-27 banner call for the daemon path is unreachable**
- **Found during:** Task 2, same investigation as Deviation 1.
- **Issue:** The plan's `<action>` text says to call `output::print_attestation_downgrade_banner(downgraded.len(), Some(&tenant_id), &dedup_key)` from `agent_daemon/launch.rs`. `output.rs` is not `#[path]`-included into `nono-agentd.rs` either (same module-independence boundary as Deviation 1) — it is structurally unreachable. Additionally, `nono-agentd` runs as a `SERVICE_USER_OWN_PROCESS` Windows service with no attached console, so an `eprintln!`-based banner would be inert even if it compiled.
- **Fix:** Used `tracing::warn!` (the Windows Event Log, via the `telemetry` module this file's own existing degraded/fail-secure log lines already route through) as the equivalent D-27 human-visible channel for the daemon path. The event carries the same coarse `downgraded_count` (D-28: no per-layer detail on this channel) that the CLI-side banner would have shown.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** Compiles and passes clippy; the `tracing::warn!` call sites are structurally identical to this file's own pre-existing degraded-condition logging idiom (e.g., the DACL-guard/WFP-gate failure paths above it).
- **Committed in:** `a11c09c8`

**3. [Rule 1 - Bug] `emit_attestation_event`'s own AUD-04 contract requires non-fatal handling, not the abort-on-Err idiom the plan's text implies**
- **Found during:** Task 1, while reading `emit_attestation_event`'s doc comment (`telemetry/mod.rs`) before wiring the call site — the plan's `<action>` text says to treat the emission's `Err` as itself AUD-04-fatal, "mirroring how `emit_override_event`'s callers already treat its `Err`" (which does abort).
- **Issue:** `emit_attestation_event`'s own doc comment explicitly states the opposite: "Callers MUST treat `Err` as non-fatal for the *launch* decision — an audit-record failure must never itself block a downgraded session from proceeding, since blocking here would trade an honesty gap for an availability regression. Callers MUST still surface the downgrade to the operator through the banner even if this call fails." Following the plan's literal instruction would have contradicted this already-shipped (Plan 09) contract and, worse, would have left a suspended child needing `terminate_suspended_process` on a path the plan's own text didn't call that on (an inconsistency that itself risked a leaked suspended process).
- **Fix:** Both gate sites now log (`tracing::warn!`) an emission failure or missing `SecurityEventLayer` and continue to print the banner / emit the Event Log warning and return `Ok(())` — the launch proceeds with the downgraded claim exactly as it would have if the audit record had committed successfully.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/launch.rs`, `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** `confirmed_live_probes_with_configured_only_row_downgrades_without_aborting` (launch.rs) runs in a test environment where `SECURITY_LAYER` is uninitialized and still asserts `Ok(())`, exercising the non-fatal path for real.
- **Committed in:** `8820e3f4`, `a11c09c8`

---

**Total deviations:** 3 auto-fixed (all Rule 1 — bugs in the plan's literal prescription discovered against the actual crate-boundary/module-independence structure and an already-shipped doc-comment contract, not against my own implementation).
**Impact on plan:** All three preserve or strengthen the plan's actual security intent (D-21/D-22/D-27/AUD-04) while fitting the real compilation-unit boundaries this codebase has always had. No scope creep — no files outside the plan's declared `files_modified` list were touched.

## Issues Encountered

**Repeated, severe environmental corruption of the working tree.** During this session, the entire `crates/nono-cli/` directory's working-tree contents (198 files) were externally deleted from disk five separate times, each time silently and without any action of my own (confirmed via `git status --short` showing 198 unstaged deletions spanning every file in the crate, including files I had never touched, e.g. `README.md`, `data/policy.json`, `examples/`). This is a real Windows worktree/checkout, not a Claude Code worktree — multiple `.claude/worktrees/agent-*` sessions were concurrently active against the same `.git` directory per `git worktree list`, though the mechanism by which their activity (if related) reached this main checkout's working-tree files was not identified. Each occurrence was recovered via a scoped `git checkout -- crates/nono-cli` (restoring from the index/HEAD, never a blanket `git checkout -- .`), which is safe and lossless for anything already committed but discards anything not yet committed. The first two occurrences struck before any commit existed and cost two full redo cycles of both tasks' edits; after that, both tasks were committed immediately upon reaching a clean, verified state, and the third/fourth/fifth occurrences (which struck mid-verification, after both commits already existed) were recovered instantly via the same `git checkout` with zero rework. **Flag for the user/orchestrator:** this appears to be an active hazard on this shared main checkout independent of this plan's work; the mitigation used here (commit early, restore via scoped checkout, never touch anything outside the affected directory) is a workaround, not a fix, and the root cause was not determined from within this session.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Two of the three D-23-relevant paths this crate directly controls (direct `nono run` / hook path, and daemon `nono agent launch`) are now gated. The broker arm's own suspended spawn of the real confined grandchild (`nono-shell-broker/src/main.rs`) is Plan 11's responsibility, consuming the `NONO_BROKER_REQUIRED_LAYERS` env var this plan sets.
- D-26's tighten-only union (CLI flag / machine-policy required-layers) is not wired at either gate site — a genuine gap if a future plan or the phase's own success criteria expect it; currently both gates pass empty `required_layers_override`/`machine_required_layers` slices, so every row's own per-row default (overwhelmingly `Abort`) still governs, but no admin/operator can additionally *tighten* a row from either of these two call sites yet.
- Flagged discrepancy for a follow-up (not fixed here, per this plan's own scope — see `daemon_attest_and_decide`'s doc comment): `layer_registry.rs`'s `WfpEgressFilters` row expectancy is unconditional for `(EntryPath::Daemon, None)`, with no "only when this profile needs scoping" distinction. If a future call site ever invokes `exec_strategy_windows::attestation::attest_and_decide` against that row literally for the daemon path, it would abort every daemon-launched agent whose profile does not request network scoping — worth a registry review before Phase 118/119 build further on this row.

## Self-Check: PASSED

- FOUND: crates/nono-cli/src/exec_strategy_windows/launch.rs
- FOUND: crates/nono-cli/src/exec_strategy_windows/mod.rs
- FOUND: crates/nono-cli/src/agent_daemon/launch.rs
- FOUND: crates/nono-cli/src/telemetry/mod.rs
- FOUND: commit 8820e3f4 (git log --oneline --all)
- FOUND: commit a11c09c8 (git log --oneline --all)

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*
