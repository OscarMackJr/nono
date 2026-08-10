---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 08
subsystem: infra
tags: [windows, sandboxing, attestation, fail-direction, appcontainer, job-object, dacl, wfp, cint-02]

# Dependency graph
requires:
  - phase: 117-01
    provides: "layer_registry (LayerId, EntryPath, ArmExpectancy, ContractOutcome, ProbeKind, LayerRegistryEntry, all_entries())"
  - phase: 117-05
    provides: "crates/nono/src/attestation.rs (LayerAttestationStatus, ProcessHandle, probe_integrity_level, probe_in_job, probe_app_container_sid, probe_restricted_sids)"
  - phase: 117-02
    provides: "NonoError::LayerAttestationFailed, RequiredLayersPolicy/MachineEgressPolicy.required_layers"
provides:
  - "attest_and_decide() — the single CLI-side attestation decision function (proceed / proceed-downgraded / abort)"
  - "AttestationDecision, AttestationInput types"
  - "BROKER_REQUIRED_LAYERS_ENV_VAR + required_layers_for_broker() — the broker wire contract closing Blocker-1"
affects: [117-09, 117-10, 117-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-18 four-state classification via a pure ProbePositivity trait (bool/Option/Vec) decoupled from live OS probes, for deterministic unit testing"
    - "decide_from_entries() factored out of attest_and_decide() so D-25/D-26 outcome policy is testable against synthetic LayerRegistryEntry fixtures without a live ProcessHandle"

key-files:
  created:
    - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  modified:
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs

key-decisions:
  - "AttestationInput.machine_required_layers takes an already-resolved slice instead of this module calling read_machine_egress_policy() itself, preserving the Phase 83 D-04 SOLE-read invariant and avoiding a live HKLM read on the D-24 latency-sensitive per-tool-call hook path"
  - "A ConfiguredOnly row's EstablishedNotIndependentlyObservable status downgrades (never aborts) its own default Abort outcome unless the layer is explicitly D-26-tightened, because the literal plan text would abort every ordinary WriteRestricted-arm launch"
  - "Widened WindowsTokenArm from pub(super) to pub(crate) in launch.rs so AttestationInput (needed pub(crate) for Plan 10/11's cross-module gate sites) does not leak a less-visible type through a more-visible field"

patterns-established:
  - "Pattern: decision-policy functions that consume a live registry should be split into a thin wrapper (I/O, validation) plus a pure core (entries slice + input + params -> decision) so security-critical branching is unit-testable with synthetic fixtures"

requirements-completed: [CINT-02]

# Metrics
duration: 55min
completed: 2026-08-09
---

# Phase 117 Plan 08: CLI-Side Attestation Decision Module Summary

**`attest_and_decide()` — the single CLI-side startup self-attestation decision function (D-18 four-state classification, D-25/D-26 outcome policy, D-07 substitute fallback) plus the `NONO_BROKER_REQUIRED_LAYERS` wire contract closing Blocker-1 for `AppContainerProfile`.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-08-09T00:00:00Z (approx.)
- **Completed:** 2026-08-09
- **Tasks:** 2
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `attest_and_decide()` iterates `layer_registry::all_entries()`, classifies each expected row into D-18's four-state vocabulary (`Confirmed` / `EstablishedNotIndependentlyObservable` / `Unconfirmed` / `NotApplicable`), and returns a coarse `AttestationDecision` (`Proceed` / `ProceedDowngraded` / `Abort`) — the single call Plans 09/10/11 will use at every gate-insertion site.
- Resolved Open Question 1: `WfpEgressFilters` classifies from the caller-supplied `wfp_preconfirmed` flag alone (never re-probing), distinct from an independent post-hoc kernel query.
- Verified Blocker-1 end-to-end: `AppContainerProfile` is structurally never probed at `EntryPath::DirectCli` (no matching `ArmExpectancy` cell exists in the registry — Plan 01's fix), and `required_layers_for_broker()` is now the one code path through which a `(EntryPath::Broker, ...)` row's expectancy reaches a decision, closing the gap at the consumption site.
- Implemented D-26's tighten-only union (CLI flag ∪ machine-policy-required layers) with fail-closed rejection of unrecognized layer names — the validation responsibility `RequiredLayersPolicy`'s doc comment (Plan 02) assigns to this module.
- Implemented D-07's substitute-equivalent-mechanism fallback: an unconfirmed primary with an unconfirmed (or unnamed) alternate falls through to `Abort`; a confirmed alternate lets the session proceed without downgrading.
- 24 unit tests covering all 6 of Task 1's behavior bullets, plus additional coverage for the D-26 tightening escalation, the substitute fallback (both directions), the fail-closed unrecognized-name validation, and the broker wire-contract discovery invariant (all Broker-expected `Abort` rows and only those appear in the env var value).

## Task Commits

Each task was committed atomically:

1. **Task 1: `attest_and_decide()` — probe dispatch + four-state classification** - `09661db6` (feat)
2. **Task 2: Broker wire contract + module wiring** - `a55c2bdd` (feat)

_Note: Task 1's commit also includes the `pub(crate) mod attestation;` wiring in `mod.rs` and the `WindowsTokenArm` visibility widening in `launch.rs`, because Task 1's own acceptance criteria (`cargo build -p nono-cli succeeds`, `cargo test ... exec_strategy_windows::attestation`) require the module to be buildable/testable standalone, which structurally requires the module to be wired into the crate — see Deviations._

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/attestation.rs` - New module: `AttestationDecision`, `AttestationInput`, `attest_and_decide()`, `classify_row`/`classify_live_probe`/`decide_from_entries` (the D-18/D-25/D-26/D-07 decision pipeline), `BROKER_REQUIRED_LAYERS_ENV_VAR`, `required_layers_for_broker()`, and 24 tests (3 test modules: pure/synthetic-fixture tests, real-registry `registry_tests`, and `broker_wire_contract_tests`)
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - Added `pub(crate) mod attestation;`
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - Widened `WindowsTokenArm` from `pub(super)` to `pub(crate)` (visibility fix required for `AttestationInput` to compile as `pub(crate)`)

## Decisions Made

- **Test design:** Factored `decide_from_entries()` out of `attest_and_decide()` as a pure function over an `entries` slice, so the D-25/D-26/D-07 outcome-application policy is unit-testable against synthetic `LayerRegistryEntry` fixtures without depending on live Windows OS probe behavior (which is host-nondeterministic — e.g. `IsProcessInJob` against the test runner's own process — per `crates/nono/src/attestation.rs`'s own documented finding). Similarly, `classify_probe_outcome<T: ProbePositivity>` is a pure, platform-neutral classification of `Ok(positive)/Ok(negative)/Err` shapes, tested directly with synthetic `Result` values rather than real probe calls.
- **`MandatoryIntegrityLabel`'s live-probe classification:** `probe_integrity_level` returns `Ok(u32 RID)`, which has no natural "successful-but-negative" shape the way `bool`/`Option`/`Vec` probes do. Classified as `Ok(_) => Confirmed`, `Err(_) => Unconfirmed` — any successfully-read RID is treated as the positive confirmation that the token's own mandatory label was queryable and present; only the OS call itself failing is a negative result.
- **D-06/FailOpen tightening:** A `FailOpen`-outcome row (currently only `MinifilterAbsence`) is never added to `downgraded` when untightened (its absence never widens the claim, per D-06), but a D-26-tightened requirement on that layer still aborts — an explicit "must be Confirmed" requirement on a structurally-absent layer can never be satisfied, so tightening it is fail-closed by design rather than a silent no-op.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `EstablishedNotIndependentlyObservable` no longer unconditionally escalates a row's own `Abort` outcome**
- **Found during:** Task 1 (outcome-application logic)
- **Issue:** The plan's literal action text says "a row whose status is not `Confirmed` and whose `ContractOutcome` is `Abort` → return `AttestationDecision::Abort` immediately." Taken literally, this aborts **every** launch that expects any `ProbeKind::ConfiguredOnly` row with `outcome: Abort` (`DaclSessionSidGrant`, `DaclPackageSidGrant`, `DaclAncestorTraverse`, `DaclAncestorReadAttrs`, `FirewallRulesEgress`, `BrokerAuthenticodeTrustGate`), because `ConfiguredOnly` rows classify to `EstablishedNotIndependentlyObservable` *unconditionally* (there is no live re-observation, by design — RESEARCH §C / `layer_registry.rs`'s own doc comment) and can therefore never reach `Confirmed`. `DaclSessionSidGrant` alone is expected on every `WriteRestricted`-arm launch (nono-cli's default supervised path), so the literal reading would make Windows launches non-functional — a clear "requirement's prescribed fix is unimplementable" case (matching the documented project lesson `feedback_requirement_prescription_may_be_unexpressible`).
- **Fix:** `decide_from_entries()`'s `ContractOutcome::Abort` arm now escalates to `Abort` only when `status == Unconfirmed` (the probe genuinely failed or authoritatively found the state absent) OR the layer is explicitly D-26-tightened. An `EstablishedNotIndependentlyObservable` status on an untightened `Abort`-outcome row downgrades instead — honoring D-18's "still downgrades the claim" without breaking ordinary operation, since reaching this function at all with a `ConfiguredOnly` row means its real enforcement gate (the apply-time DACL grant / firewall rule / Authenticode compare) already succeeded fail-closed before `attest_and_decide` was ever invoked.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs`
- **Verification:** `established_not_independently_observable_downgrades_but_does_not_abort` (proves a `ConfiguredOnly`+`Abort` row downgrades, not aborts, when untightened) and `tightened_required_layer_aborts_even_when_default_outcome_is_fail_open`/`substitute_equivalent_mechanism_falls_through_to_abort_when_alternate_unconfirmed` (prove tightening still forces `Abort`). All 24 tests pass; full `nono-sandbox-cli` bin test suite (1565 passed, 11 pre-existing baseline failures unrelated to this file — see Issues Encountered) shows no regression.
- **Committed in:** `09661db6` (Task 1 commit)

**2. [Rule 1 - Bug] No live `read_machine_egress_policy()` call inside this module**
- **Found during:** Task 1 (D-26 union implementation)
- **Issue:** The plan's action text instructs parsing "machine-policy's `required_layers.required` (read via `nono::machine_policy::read_machine_egress_policy`)" directly inside `attest_and_decide`. This conflicts with the documented Phase 83 D-04 "SOLE read" invariant (`agent_daemon/mod.rs:353`: "the daemon startup path performs exactly ONE `read_machine_egress_policy()` call... GPO changes take effect on the NEXT daemon restart — the startup snapshot is held for the daemon's process lifetime") — the daemon is long-lived and handles many tenant launches over its lifetime (Phase 74's persistent multi-tenant `nono-agentd`), so calling `read_machine_egress_policy()` on every `attest_and_decide` invocation would re-read `HKLM` on every launch, not once at daemon startup, and would additionally add a live Win32-registry read to the D-24 latency-sensitive per-tool-call hook path (`claude_code_hook.rs`).
- **Fix:** Added `AttestationInput::machine_required_layers: &'a [String]`, taking the already-resolved `required_layers.required` slice from whichever single machine-policy read the caller's own entry path already performs (the daemon's D-04 startup snapshot; the direct-CLI path's own existing single startup read at `main.rs:189`). The D-26 tighten-only union and the fail-closed unrecognized-name validation (a responsibility `RequiredLayersPolicy`'s doc comment explicitly assigns to this Plan) are still performed inside `attest_and_decide`, just sourced from caller-supplied data.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs`
- **Verification:** `attest_and_decide_rejects_unrecognized_required_layer_name_from_machine_policy` / `_from_cli_flag` / `attest_and_decide_accepts_known_layer_names` exercise the union+validation path with caller-supplied slices.
- **Committed in:** `09661db6` (Task 1 commit)

**3. [Rule 3 - Blocking] Widened `WindowsTokenArm` from `pub(super)` to `pub(crate)`**
- **Found during:** Task 1 (initial `cargo build`)
- **Issue:** `AttestationInput.token_arm: Option<WindowsTokenArm>` triggered `private_interfaces` (a `pub(crate)` field naming a type visible only at `pub(in crate::exec_strategy)`), a hard failure under `-D warnings`. `AttestationInput` must be `pub(crate)` because Plan 10/11's `agent_daemon/launch.rs` gate-insertion site (outside the `exec_strategy_windows` module tree) needs to construct it.
- **Fix:** Widened `WindowsTokenArm`'s declaration in `launch.rs` from `pub(super)` to `pub(crate)` — a minimal, safe widening (the type remains fully crate-internal, never crosses the crate boundary) documented inline with the rationale.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/launch.rs`
- **Verification:** `cargo build -p nono-sandbox-cli --bin nono` and `cargo clippy ... -D warnings -D clippy::unwrap_used` both exit 0; full `exec_strategy::` test suite (213 tests) still passes with no regressions from the visibility change.
- **Committed in:** `09661db6` (Task 1 commit)

**4. [Rule 3 - Blocking] `mod.rs` wiring and command adaptation for the actual package/binary names**
- **Found during:** Task 1 (verification)
- **Issue:** Task 1's `<files>` list only names `attestation.rs`, but its own acceptance criteria (`cargo build -p nono-cli succeeds`, `cargo test -p nono-cli --lib exec_strategy_windows::attestation`) require the module to be part of the compiled crate — which needs the `pub(crate) mod attestation;` line Task 2 nominally owns. Additionally, the crate is actually named `nono-sandbox-cli` (binary `nono`) with **no `[lib]` target** (per project memory), so the plan's literal `-p nono-cli`/`--lib` commands do not exist as written.
- **Fix:** Added `pub(crate) mod attestation;` to `mod.rs` as part of Task 1's commit (structurally necessary for Task 1 to build/test standalone). Ran all verification via `cargo build -p nono-sandbox-cli --bin nono`, `cargo clippy -p nono-sandbox-cli --bin nono -- -D warnings -D clippy::unwrap_used` (with and without `--features layer-fault-injection`), and `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation:: -- --test-threads=1`.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/mod.rs`
- **Verification:** All commands above exit 0 / pass; see Issues Encountered for the full verification transcript summary.
- **Committed in:** `09661db6` (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 1 bug fixes, 2 Rule 3 blocking-issue fixes)
**Impact on plan:** Deviations 1 and 2 are both correctness-preserving fixes to an unimplementable-as-literally-written prescription (Deviation 1 would have broken ordinary Windows launches entirely; Deviation 2 would have violated a documented architecture invariant and added latency to a hot path). Deviations 3 and 4 are minimal, safe build-fixes with no security or scope impact. No scope creep — all four stay within Plan 08's stated deliverable (`attest_and_decide()`, `AttestationDecision`, the broker wire contract).

## Issues Encountered

- **Package/binary naming mismatch with the plan's literal verification commands:** the plan's `<verification>` and acceptance criteria reference `cargo build -p nono-cli` / `cargo test -p nono-cli --lib ...`, but the crate is `nono-sandbox-cli` with no `[lib]` target (binary `nono`). Adapted all commands per project memory's documented convention (`-p nono-sandbox-cli --bin nono`). No functional impact.
- **Full-crate test baseline:** `cargo test -p nono-sandbox-cli --bin nono` (unscoped) shows 11 pre-existing failures (`audit_session`, `config::tests` (5), `profile_cmd`, `protected_paths` (3)) unrelated to this plan's files — this matches the documented baseline exactly (project memory: "nono-cli/nono Windows baseline test failures... real figure is 11, not the 4 first recorded"). Not chased as regressions; verified identical failure set across two independent full runs (parallel and `--test-threads=1`).
- **Cross-target clippy applicability:** Per CLAUDE.md's cross-target verification rule and the already-established, checker-reviewed precedent in `layer_registry.rs`'s own module doc (D-11 consequence table, Plan 01), files strictly under `exec_strategy_windows/` that introduce no literal `target_os = "linux"`/`"macos"` cfg branches do not trigger the cross-target clippy MUST on their own — the entire `exec_strategy_windows/` module tree is excluded from non-Windows builds via `main.rs`'s `#[cfg(target_os = "windows")] #[path = ...] mod exec_strategy;` swap. Confirmed via `grep -n 'target_os = "linux"\|target_os = "macos"\|cfg(unix)'` against the new file: zero matches. Ran the standard native (Windows-host) `cargo build`/`clippy`/`fmt --check` gates, both with and without `--features layer-fault-injection`, per the plan's own `<verification>` block; did not run the Docker `cross`/`cargo-zigbuild` cross-target gates for this plan, consistent with the D-11 precedent already reviewed and accepted in this phase.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `attest_and_decide()`, `AttestationDecision`, `AttestationInput`, `BROKER_REQUIRED_LAYERS_ENV_VAR`, and `required_layers_for_broker()` are all `pub(crate)` and ready for Plan 09 (same wave — D-27 output/telemetry channels) and Wave 4 Plans 10/11 (wiring the decision function into the three real spawn sites: `launch.rs` direct spawn, `agent_daemon/launch.rs`, and `nono-shell-broker`'s own local probe-then-decide).
- Plan 10/11 will need to: (a) compute `wfp_preconfirmed` from the existing pre-spawn WFP IPC check and pass it in; (b) resolve `machine_required_layers` from their own single `read_machine_egress_policy()` call (per Deviation 2, this module deliberately does not read it); (c) call `required_layers_for_broker(layer_registry::all_entries())` when constructing the broker's command line/environment, setting `BROKER_REQUIRED_LAYERS_ENV_VAR`.
- No blockers. All verification (build, clippy with/without `layer-fault-injection`, `cargo fmt --all -- --check`, full test suite) is green; the two known deviations from the plan's literal text are documented above with rationale and test coverage.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-09*

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/exec_strategy_windows/attestation.rs`
- FOUND: commit `09661db6` (Task 1)
- FOUND: commit `a55c2bdd` (Task 2)
- `pub(crate) mod attestation;` present in `mod.rs` (count: 1)
