---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 01
subsystem: infra
tags: [windows, sandbox, security, appcontainer, wfp, dacl, job-object, restricted-token, layer-registry]

# Dependency graph
requires: []
provides:
  - "crates/nono-cli/src/exec_strategy_windows/layer_registry.rs — the code-resident Windows fail-direction layer registry (LayerId, EntryPath, ArmExpectancy, ContractOutcome, ProbeKind, LayerRegistryEntry) with 13 populated rows"
  - "LayerId::ALL + assert_all_layer_ids_covered compile-time drift guard"
  - "all_entries() cfg(windows)/cfg(not(windows)) accessor"
  - "broker_expected_rows_are_abort_only discovery-based invariant test"
affects: [117-03-spec, 117-05-shared-probes, 117-08-cli-decision-logic, 117-09-channels, 117-10-gate-insertion, 117-11-broker-attestation, 117-12-tests, 118-receipts]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Compile-time exhaustive drift guard (ALL const + no-wildcard match), copied verbatim from crates/nono/src/undo/types.rs's NetworkAuditDenialCategory idiom"
    - "Platform-neutral types + cfg(target_os = \"windows\") population split, copied from crates/nono/src/machine_policy.rs's read_machine_egress_policy pattern"
    - "Per-(entry_path, token-arm) expectancy matrix instead of a flat expected/not-expected flag"

key-files:
  created:
    - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  modified:
    - crates/nono-cli/src/exec_strategy_windows/mod.rs

key-decisions:
  - "SC4-3 resolved: FirewallRulesNetworkBackend is reachable in production (network.rs:1500-1533 dispatch) and already fails closed — it is its own registry row (LayerId::FirewallRulesEgress), not a WFP sub-case"
  - "AppContainerProfile expectancy corrected to (EntryPath::Broker, None) and (EntryPath::Daemon, None) only, never (EntryPath::DirectCli, ...) — closes the cross-process attestation gap where nono-cli's own gate would have observed nono-shell-broker.exe instead of the real confined grandchild"
  - "Every row's ContractOutcome is spelled out explicitly (13/13); MinifilterAbsence is the sole FailOpen row (ADR-65), every other row is Abort"
  - "Deviation (Rule 1): scoped the broker_expected_rows_are_abort_only invariant to rows with probe != ProbeKind::NotApplicable — MinifilterAbsence is expected at every (entry_path, None) cell including Broker with a deliberate FailOpen outcome, and the unscoped invariant produced a false positive since a structurally-absent layer has nothing to attest on any arm"

requirements-completed: [CINT-01]

# Metrics
duration: 24min
completed: 2026-08-09
---

# Phase 117 Plan 01: Windows Fail-Direction Layer Registry Summary

**Code-resident 13-row Windows fail-direction layer registry (`LayerId`/`EntryPath`/`ArmExpectancy`/`ContractOutcome`/`ProbeKind`/`LayerRegistryEntry`) with a compile-time exhaustive drift guard and a discovery-based broker-abort-only invariant test — the single source of truth every downstream Phase 117 deliverable (SPEC, self-attestation, per-layer tests) reads from.**

## Performance

- **Duration:** 24 min (23:39 → 00:03 local, three task commits)
- **Tasks:** 3 completed
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- Resolved RESEARCH's two open questions with direct, re-runnable evidence: SC4-3 (`FirewallRulesEgress` is reachable in production, distinct from WFP, fails closed) and the D-10 in/out call for every remaining candidate layer (Job Object containment IN, env sanitization OUT, interpreter coverage gate IN as pre-flight, broker Authenticode trust gate IN scoped to broker arms, proxy egress OUT)
- Landed `LayerId` (13 variants), `EntryPath` (the axis-2 entry-path/process-topology finding from RESEARCH §B), `ArmExpectancy` (the D-08 per-(entry_path, token-arm) matrix cell), `ContractOutcome` (the four-value D-05 vocabulary plus the distinct D-06 `FailOpenDefect`), and `ProbeKind` (D-17/D-18, including the `ConfirmedByEnforcingComponentReport` resolution of Open Question 1 for WFP)
- Populated all 13 rows with real `"file:line"` call-site citations verified against the live tree (not re-derived from stale line numbers), an explicit spelled-out `ContractOutcome` per row, and a per-arm expectancy matrix
- Applied the Blocker-1 fix: `AppContainerProfile` is `expected: true` ONLY at `(EntryPath::Broker, None)` and `(EntryPath::Daemon, None)` — `nono-cli`'s own `DirectCli` gate never claims to attest it, closing the exact honesty-gap failure mode CINT-02 exists to prevent from reappearing inside its own fix
- Added `LayerId::ALL` + `assert_all_layer_ids_covered` (exhaustive, no wildcard arm — copied from `crates/nono/src/undo/types.rs`'s `NetworkAuditDenialCategory` idiom) so a future `LayerId` variant added without a corresponding `ALL` entry and match arm fails to compile
- Added the `all_entries()` `cfg(windows)`/`cfg(not(windows))` split (mirrors `machine_policy.rs`'s `read_machine_egress_policy` pattern) so the types stay usable on every host per D-11
- Added `broker_expected_rows_are_abort_only`, a discovery-based unit test (iterates `all_entries()`, never hardcodes `LayerId::AppContainerProfile` or any other variant name) enforcing that every `EntryPath::Broker`-expected row has `ContractOutcome::Abort` — closing the Item-1 green-by-absence gap mechanically, not just in a comment
- Wired `pub(crate) mod layer_registry;` into `mod.rs`

## Task Commits

Each task was committed atomically:

1. **Task 1: Resolve open D-10 derivation questions and record the evidence** - `9ec456b8` (docs)
2. **Task 2: Define LayerId, the expectancy matrix, and every row's explicit outcome** - `3734d182` (feat)
3. **Task 3: Exhaustive drift guard, broker-abort-only invariant test, platform-neutral/cfg(windows) split, and module wiring** - `d63b68f8` (feat)

_Note: Task 2 was verified compiling + clippy-clean by temporarily wiring the module into `mod.rs`, running `cargo check`/`cargo clippy` on the `nono` bin target, then reverting the wiring before committing — Task 3 landed the permanent wire-up._

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` - New file: the registry types, the D-16-shaped evidence module doc comment, 13 populated rows, the exhaustive drift guard, the `all_entries()` cfg split, and the invariant test module
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - Added `pub(crate) mod layer_registry;` (single-line addition, existing `mod` declarations otherwise untouched)

## Decisions Made

- **SC4-3 resolved (Task 1):** `FirewallRulesNetworkBackend` is reachable in production (`network.rs:1500-1533` dispatch match arm on `active_backend`), materially distinct from `WfpEgressFilters` (program-path-scoped `netsh` block rules vs. session/package-SID-scoped WFP allow filters), and already fails closed with partial-rule rollback on the inbound-rule failure path. It becomes its own registry row, `LayerId::FirewallRulesEgress`.
- **D-10 candidate calls (Task 1):** Job Object containment IN as a full row (load-bearing kill-group/`--timeout` guarantee, already fail-closed via terminate+`Err`); env sanitization OUT (data hygiene, not a kernel confinement guarantee a compromised child can defeat); interpreter coverage gate IN but as a pre-flight row (`probe: ProbeKind::NotApplicable`) since it runs before spawn with nothing to re-attest; broker Authenticode trust gate IN scoped to the `BrokerLaunch`/`BrokerLaunchNoPty` arms only; proxy egress OUT (cross-platform, Phase 119 BOUND-01 territory).
- **Blocker-1 (Task 2):** `AppContainerProfile`'s expectancy corrected to `(EntryPath::Broker, None)`/`(EntryPath::Daemon, None)` only. `nono-cli`'s own `spawn_windows_child` gate on the `DirectCli` entry path observes `nono-shell-broker.exe` itself (Medium-IL, unconfined) — never the real AppContainer-confined grandchild the broker spawns in its own separate `CREATE_SUSPENDED` window (`nono-shell-broker/src/main.rs:537-661`).
- **Item-1 (Task 2/3):** every row's `ContractOutcome` is spelled out explicitly (verified: `grep -c "outcome: ContractOutcome::"` = 13, matching the 13-row `LayerId` enum). `MinifilterAbsence` is the sole `FailOpen` row (ADR-65, per-file read policy not claimed); every other row is `Abort`, matching the enforcing call site's already-fail-closed behavior confirmed by direct read (not inferred from doc comments).
- **Expectancy modeling choice (Task 2):** rows present on both the `DirectCli`/`Daemon` entry paths (`WfpEgressFilters`, `DaclPackageSidGrant`/`DaclAncestorTraverse`/`DaclAncestorReadAttrs`, `JobObjectContainment`) carry dual `call_sites` citations in one flat list rather than a structural `(entry_path, call_site)` pair type, per RESEARCH Open Question 3's resolution.
- **WFP program-path-scoped simplification (Task 2, recorded per D-10):** `WfpEgressFilters`'s expectancy is scoped to `(DirectCli, BrokerLaunchNoPty)` and `(Daemon, None)` — the package-SID-scoped cases RESEARCH explicitly evidenced. The program-path-scoped WFP variant (selected without a package SID via the `AllowAll`+port-rules branch, `network.rs:1501-1503`) can in principle apply to other arms too; this is recorded as a known simplification in the row's doc comment rather than modeled as a 14th row, deferred to Plan 08's dispatch nuance.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Scoped `broker_expected_rows_are_abort_only` to attestable rows (`probe != ProbeKind::NotApplicable`)**
- **Found during:** Task 3, running the invariant test live after wiring the module in
- **Issue:** The unscoped invariant (as literally specified: "every `EntryPath::Broker`-expected row must have `ContractOutcome::Abort`") failed on `MinifilterAbsence`. Per Task 2's own instructions, `MinifilterAbsence`'s expectancy applies to every `(entry_path, None)` cell uniformly, including `EntryPath::Broker`, with the deliberate ADR-65-justified `FailOpen` outcome. Applying the invariant unconditionally produced a false positive: `MinifilterAbsence` is not a layer that goes silently unattested on the broker arm — it is a row documenting a layer that does not exist and therefore has nothing to attest on ANY arm.
- **Fix:** Added a `continue` guard skipping rows whose `probe == ProbeKind::NotApplicable` before applying the broker-abort-only check. This is a discovery-based scope (keyed on a field every row already carries, not a hardcoded `LayerId` name), consistent with D-32's "prefer tests that discover their targets" rule — a future row is exempted only if it also documents `ProbeKind::NotApplicable`, itself a deliberate, reviewable choice recorded on that row.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`
- **Verification:** `cargo test -p nono-sandbox-cli --bin nono layer_registry -- --test-threads=1` — 4/4 tests pass, including `broker_expected_rows_are_abort_only` and a companion sanity test (`at_least_one_row_is_broker_expected`) confirming the invariant still has real rows to exercise (`AppContainerProfile`, `MandatoryIntegrityLabel`).
- **Committed in:** `d63b68f8` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug fix)
**Impact on plan:** The fix preserves the invariant's actual security purpose (prevent an attestable layer from going silently unattested on the broker arm) while correctly exempting the one row that was never attestable to begin with. No scope creep — the fix is scoped entirely to the test added in the same task.

## Issues Encountered

None beyond the deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` is the ready-to-cite data source for Plan 03 (SPEC authoring/drift-check), Plan 05 (shared probes), Plans 06/07 (fault-injection hooks), Plan 08 (CLI decision logic / `required_layers_for_broker()`), Plan 09 (channels), Plans 10/11 (gate insertion, including the broker's own local probe-then-decide), and Plan 12 (per-layer forced-unavailable tests).
- The module doc comment's evidence ledger (SC4-3, the cross-target cfg table, the D-10 in/out calls, Blocker-1, Item-1) is written to be directly citable by Plan 03's SPEC authoring without a separate scratch document, per the plan's own verification criterion.
- `cargo build -p nono-sandbox-cli --bin nono`, `cargo clippy -p nono-sandbox-cli --bin nono -- -D warnings -D clippy::unwrap_used`, `cargo fmt --all -- --check`, and `cargo test -p nono-sandbox-cli --bin nono layer_registry -- --test-threads=1` all pass. `cargo check --workspace --all-targets` passes.
- No blockers for downstream plans. The one recorded simplification (WFP program-path-scoped expectancy on non-`BrokerLaunchNoPty`/non-`Daemon` arms) is explicitly flagged in the row's doc comment as deferred nuance for Plan 08, not a silent gap.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-09*

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`
- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-01-SUMMARY.md`
- FOUND commit: `9ec456b8` (Task 1)
- FOUND commit: `3734d182` (Task 2)
- FOUND commit: `d63b68f8` (Task 3)
- FOUND commit: `0562440d` (SUMMARY.md)
