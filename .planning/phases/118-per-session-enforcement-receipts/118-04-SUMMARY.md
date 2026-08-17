---
phase: 118-per-session-enforcement-receipts
plan: 04
subsystem: windows-daemon-attestation-census
tags: [windows, enforcement-receipts, daemon, collect-all-then-decide, tdd, source-scan, fail-direction]
dependency-graph:
  requires:
    - "crates/nono::LayerId / EntryPath / TokenArm / LayerReceiptRow / SessionOutcome / EnforcementReceipt (118-01)"
    - "crates/nono-cli/src/exec_strategy_windows/attestation.rs::census_from_entries / build_enforcement_receipt pattern (118-03, mirrored not called)"
  provides:
    - "crates/nono-cli/src/agent_daemon/launch.rs::daemon_decision_from_booleans (pure precedence chain, restructure of daemon_attest_and_decide)"
    - "crates/nono-cli/src/agent_daemon/launch.rs::daemon_census_rows (pure, non-early-returning 5-row census pass)"
    - "crates/nono-cli/src/agent_daemon/launch.rs::DAEMON_UNMODELLED_LAYER_EXPECTANCY (8-row NotApplicable expectancy table)"
    - "crates/nono-cli/src/agent_daemon/launch.rs::build_daemon_receipt (5+8=13-row nono::EnforcementReceipt assembler, not yet wired)"
  affects: []
tech-stack:
  added: []
  patterns:
    - "collect-all-then-decide restructure: probe unconditionally into local bools FIRST, then apply the byte-identical precedence chain on the precomputed values (D-26)"
    - "decision (2-state) and census (4-state per-row) built as two separate PURE functions sharing one input shape, never merged into one return type"
    - "discovery-based cross-file source scanner with comment-stripping + balanced-brace extraction + const-alias-following, to keep a daemon-local hardcoded table honest against a registry it cannot import"
key-files:
  created: []
  modified:
    - crates/nono-cli/src/agent_daemon/launch.rs
decisions:
  - "daemon_decision_from_booleans/daemon_census_rows take 6 inputs (app_container_confirmed, job_confirmed, network_scoping_required, wfp_filters_installed, dacl_guard_applied, ancestor_traverse_applied), not the plan's suggested 5-arg shape (app_container_confirmed, job_confirmed, wfp_ok, dacl_ok, ancestor_ok). A single collapsed wfp_ok boolean cannot distinguish NotApplicable (network scoping not requested) from Unconfirmed (requested but not installed) for the WfpEgressFilters census row — losing that distinction would violate D-13 (all four states carried verbatim). Kept both underlying booleans."
  - "DaclPackageSidGrant/DaclAncestorTraverse census status uses EstablishedNotIndependentlyObservable (not Confirmed) when applied=true, matching the registry's own ProbeKind::ConfiguredOnly vocabulary for these two rows (verified against layer_registry.rs before writing daemon_census_rows) — Confirmed is reserved for the two LiveTokenOrJobQuery rows (AppContainerProfile/JobObjectContainment) that are genuinely re-probed."
  - "The cross-check discovery scanner follows const-to-const AND const-to-function-call aliases (not just direct literal references), because WfpEgressFilters's real registry row references an aliased const (WFP_EGRESS_FILTERS_EXPECTANCY = DACL_PACKAGE_SID_SCOPED_EXPECTANCY) — a resolver that stopped at the first non-literal const would silently miss it. Verified with a dedicated non-vacuous test (resolver_follows_a_const_to_const_alias_not_just_direct_literals)."
  - "Test 2's perturbation proof uses a fictional LayerId name (HypotheticalFutureLayer), not a real one, because all 13 real LayerIds are — by the property Test 1 asserts — already covered by either DAEMON_MODELLED_LAYER_IDS or DAEMON_UNMODELLED_LAYER_EXPECTANCY today; there is no real uncovered variant to point at. The scanner is pure text matching and does not care whether the identifier is a real Rust enum variant, so this still proves the guard's classification-check path can fail."
metrics:
  duration: "~3h"
  completed: 2026-08-16
---

# Phase 118 Plan 04: Windows Daemon Attestation Restructure (D-26) Summary

Restructured `nono-agentd`'s `daemon_attest_and_decide` from a 5-of-13-layer early-return chain into
a collect-all-then-decide pass, with a proven-unchanged decision outcome, then gave the daemon a
real 13-row census by pairing the restructured 5 modelled rows with an 8-row, drift-guarded,
`NotApplicable` expectancy table for the layers `nono-agentd` structurally cannot reach.

## Decision-Equivalence Proof (D-26, required evidence)

This is the highest-risk plan in Phase 118: a fail-direction change to shipped security code. The
table below maps each of the 4 original early-return branches (plus the terminal non-aborting
Proceed path) to the scenario, expected outcome, and test that proves the restructured
`daemon_decision_from_booleans` reaches the IDENTICAL decision the pre-restructure code reached.

| # | Original early-return branch (pre-restructure line) | Input shape | Pre-restructure outcome | Post-restructure outcome (measured) | Proof |
|---|---|---|---|---|---|
| 1 | `if !app_container_confirmed { return Abort{"AppContainerProfile"} }` (~1435) | `app_container_confirmed=false`, all else true | `Abort{AppContainerProfile}` | `Abort{AppContainerProfile}` — IDENTICAL | `daemon_decision_from_booleans_matches_pre_restructure_precedence_for_every_scenario`, scenario "AppContainerProfile fails alone"; also `null_handle_aborts_on_app_container_profile` (unchanged pre-existing test, still passes unmodified) |
| 2 | `if !job_confirmed { return Abort{"JobObjectContainment"} }` (~1446) | `app_container_confirmed=true`, `job_confirmed=false`, all else true | `Abort{JobObjectContainment}` | `Abort{JobObjectContainment}` — IDENTICAL | same test, scenario "JobObjectContainment fails alone"; also pre-existing `real_appcontainer_process_outside_the_named_job_aborts_on_job_containment`, still passes unmodified |
| 3 | `if network_scoping_required && !wfp_filters_installed { return Abort{"WfpEgressFilters"} }` (~1455) | `app_container_confirmed=true`, `job_confirmed=true`, `network_scoping_required=true`, `wfp_filters_installed=false` | `Abort{WfpEgressFilters}` | `Abort{WfpEgressFilters}` — IDENTICAL | same test, scenario "WfpEgressFilters fails alone"; also pre-existing `real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`'s `unscoped_decision` assertion, still passes unmodified |
| 4 | `if !dacl_guard_applied { return Abort{"DaclPackageSidGrant"} }` (~1470) | `app_container_confirmed=true`, `job_confirmed=true`, `network_scoping_required=false`, `dacl_guard_applied=false` | `Abort{DaclPackageSidGrant}` | `Abort{DaclPackageSidGrant}` — IDENTICAL | same test, scenario "DaclPackageSidGrant fails alone"; also pre-existing `real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`'s `unapplied_dacl_decision` assertion and `ancestor_traverse_and_package_sid_grant_classify_independently`, both still pass unmodified |
| 5 (non-abort) | `if !ancestor_traverse_applied { warn!(); }` then falls through to `Proceed` (~1489) | `ancestor_traverse_applied=false`, all else true | `Proceed` (WR-06: never aborts) | `Proceed` — IDENTICAL | same test, scenario "ancestor_traverse_applied=false never aborts, even combined with an unrelated abort"; also pre-existing `ancestor_traverse_and_package_sid_grant_classify_independently`, still passes unmodified |
| 6 (terminal) | Falls through all 4 checks → `Proceed` (~1499) | all inputs true | `Proceed` | `Proceed` — IDENTICAL | same test, scenario "all pass"; also pre-existing `real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`'s primary `decision` assertion, still passes unmodified |
| 7 (multi-fail precedence, NEW coverage) | Pre-restructure: only branch 1 could ever be observed — `probe_in_job` was never called if `probe_app_container_sid` had already failed, so the old code could not have produced ANY other outcome for this input | `app_container_confirmed=false`, `job_confirmed=false` (both fail), all else true | `Abort{AppContainerProfile}` (the ONLY outcome the old code could produce, since `JobObjectContainment` was unreachable) | `Abort{AppContainerProfile}` — IDENTICAL, and the precedence race is now won by construction (both booleans exist before the decision runs), not by "the losing check never ran" | same test, scenario "two fail simultaneously: AppContainerProfile AND JobObjectContainment" |

**No input that previously refused can now proceed.** Every branch above that previously returned
`Abort` still returns `Abort` on the same named layer for the same input; the only behavioral change
is that `daemon_census_rows` (a SEPARATE, non-decision-affecting function) can now also report a real
status for the layers past the first abort point — it never widens what `daemon_decision_from_booleans`
allows. Widening COVERAGE (probing more layers) is exactly what D-26 requires; the decision's own
fail direction (which layer aborts, and that Proceed requires all four to hold) is provably unchanged.

**Perturbation proof that the equivalence test itself can fail (not just pass by construction):**
swapped the precedence order of the `AppContainerProfile`/`JobObjectContainment` checks inside
`daemon_decision_from_booleans` (job check first). Re-ran the test suite: the equivalence test failed
with `left: "JobObjectContainment", right: "AppContainerProfile"` on the multi-fail scenario, AND the
pre-existing `null_handle_aborts_on_app_container_profile` test also failed with the same mismatch —
confirming the guard is anchored to real precedence, not vacuous. Reverted immediately; full 26-test
suite (pre-Task-2 count) passed again unchanged afterward.

## What Was Built

**Task 1 (`30a3b2ba`)** — `feat(118-04): restructure daemon_attest_and_decide to collect-all-then-decide (D-26)`

- `daemon_attest_and_decide` now calls `probe_app_container_sid` and `probe_in_job` **both,
  unconditionally**, before delegating to a new pure function `daemon_decision_from_booleans` for the
  decision. Its own signature and the `launch_agent` call site are both unchanged (the 7-argument
  shape `daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition` asserts on is
  untouched).
- `daemon_decision_from_booleans(app_container_confirmed, job_confirmed, network_scoping_required,
  wfp_filters_installed, dacl_guard_applied, ancestor_traverse_applied) -> DaemonAttestationDecision`
  — the exact original abort-precedence chain, byte-identical in decision terms, now taking
  precomputed booleans instead of making probe calls inline.
- `daemon_census_rows(..same 6 inputs..) -> Vec<nono::LayerReceiptRow>` — a NEW, non-early-returning
  pass producing a real `LayerAttestationStatus` for all 5 modelled rows (`AppContainerProfile`,
  `JobObjectContainment`, `WfpEgressFilters`, `DaclPackageSidGrant`, `DaclAncestorTraverse`)
  regardless of which one the decision aborts on first. Status mapping verified per-row against
  `layer_registry.rs`'s own `ProbeKind` for each: `LiveTokenOrJobQuery` rows map to
  `Confirmed`/`Unconfirmed`; the `ConfirmedByEnforcingComponentReport` row (`WfpEgressFilters`) maps
  to `NotApplicable`/`Confirmed`/`Unconfirmed` depending on both `network_scoping_required` AND
  `wfp_filters_installed`; the two `ConfiguredOnly` rows (`DaclPackageSidGrant`,
  `DaclAncestorTraverse`) map to `EstablishedNotIndependentlyObservable`/`Unconfirmed`.
- `DAEMON_MODELLED_LAYER_IDS` — the 5 `LayerId`s named above, used only by Task 2's cross-check.
- 2 new tests in `windows_impl::attestation_gate_tests`: the equivalence-proof test (7 scenarios, see
  table above) and a census-completeness test proving `JobObjectContainment`/`WfpEgressFilters`/
  `DaclPackageSidGrant`/`DaclAncestorTraverse` all report real, non-vacuous status even when
  `AppContainerProfile` fails first.

**Task 2 (`c22ef174`)** — `test(118-04): daemon-local 8-row expectancy table + cross-check drift guard + receipt assembly + sentinel round-trip`

- `DAEMON_UNMODELLED_LAYER_EXPECTANCY: [(LayerId, LayerAttestationStatus); 8]` — the 8 `LayerId`s
  `daemon_census_rows` does not model (`RestrictedToken`, `MandatoryIntegrityLabel`,
  `DaclSessionSidGrant`, `DaclAncestorReadAttrs`, `FirewallRulesEgress`, `MinifilterAbsence`,
  `BrokerAuthenticodeTrustGate`, `InterpreterCoverageGate`), classified per each row's REAL
  `(EntryPath::Daemon, ..)` expectancy cell in `layer_registry.rs`'s `REGISTRY_ENTRIES`, read
  directly before writing this table (not assumed uniform). Row-by-row citation:
  - `RestrictedToken` → `RESTRICTED_TOKEN_EXPECTANCY` (1 entry, `DirectCli`/`WriteRestricted` only) — no `(Daemon, ..)` cell → `NotApplicable`.
  - `MandatoryIntegrityLabel` → `MANDATORY_INTEGRITY_LABEL_EXPECTANCY` (6 entries: 5 `DirectCli` + 1 `Broker`) — no `(Daemon, ..)` cell → `NotApplicable`.
  - `DaclSessionSidGrant` → `DACL_SESSION_SID_GRANT_EXPECTANCY` = `[]` (empty, CR-05: layer does not exist in the shipped tree) → `NotApplicable`.
  - `DaclAncestorReadAttrs` → `DACL_ANCESTOR_READ_ATTRS_EXPECTANCY` = `ALL_DIRECT_CLI_ARMS_EXPECTANCY` (alias, all `DirectCli`; CR-06's own comment: "the daemon never grants FILE_READ_ATTRIBUTES ... the `(Daemon, None)` cell is therefore dropped") → `NotApplicable`.
  - `FirewallRulesEgress` → `FIREWALL_RULES_EGRESS_EXPECTANCY` = `ALL_DIRECT_CLI_ARMS_EXPECTANCY` (alias, all `DirectCli`) → `NotApplicable`.
  - `MinifilterAbsence` → `MINIFILTER_ABSENCE_EXPECTANCY` (3 entries, INCLUDING `(Daemon, None, expected: true)`) — the only one of the 8 with a real `expected: true` Daemon cell, but its `probe: ProbeKind::NotApplicable` forces `NotApplicable` regardless (D-21/ADR-65: structural absence, nothing to probe) → `NotApplicable`, named explicitly rather than falling out of a generic default.
  - `BrokerAuthenticodeTrustGate` → `BROKER_AUTHENTICODE_TRUST_GATE_EXPECTANCY` (2 entries, `DirectCli`/`BrokerLaunch{,NoPty}` only) — no `(Daemon, ..)` cell → `NotApplicable`.
  - `InterpreterCoverageGate` → `ALL_DIRECT_CLI_ARMS_EXPECTANCY` (alias via function, all `DirectCli`) — no `(Daemon, ..)` cell → `NotApplicable`.
- `build_daemon_receipt(census, session_id, pid, outcome) -> nono::EnforcementReceipt` — combines
  the 5-row modelled census with the 8-row unmodelled table into a full 13-row `Vec<LayerReceiptRow>`.
  `entry_path: nono::EntryPath::Daemon`, `token_arm: None` (post-118-01 enum typing; see "Plan-Text
  Supersession" below). **Not wired into the live `launch_agent` gate by this plan** — a later plan's
  job; this plan proves the assembly path is correct and content-free only.
- `daemon_expectancy_cross_check` module — a discovery-based source scanner reading
  `layer_registry.rs`'s CURRENT text fresh on every run (`include_str!`, the legitimate cross-file
  use per this file's existing `daemon_decision_enum_variants` precedent). Strips `//`/`///` line
  comments (verified: this file has zero `/* */` block comments), then extracts every
  `LayerRegistryEntry`'s `(id, expectancy const name)` pair via balanced-brace scanning, resolves the
  expectancy const's literal `ArmExpectancy` body by following BOTH const-to-const aliases (e.g.
  `WFP_EGRESS_FILTERS_EXPECTANCY` → `DACL_PACKAGE_SID_SCOPED_EXPECTANCY`) and const-to-function-call
  indirection (e.g. `ALL_DIRECT_CLI_ARMS_EXPECTANCY` → `all_direct_cli_arms_expectancy()`), and checks
  each resolved body for an `(entry_path: EntryPath::Daemon, expected: true)` cell.
  - `every_daemon_expected_registry_row_has_a_matching_daemon_local_classification` — Test 1: every
    discovered `LayerId` must be covered by `DAEMON_MODELLED_LAYER_IDS` or
    `DAEMON_UNMODELLED_LAYER_EXPECTANCY`. Includes an explicit non-vacuity guard: the discovered set
    must be non-empty (a scanner bug that silently found nothing would otherwise pass trivially).
  - `synthetic_unrecognised_daemon_cell_is_caught_by_the_discovery_scan` — Test 2 (perturbation
    proof): a synthetic, in-test-only registry-shaped string naming a fictional `LayerId`
    (`HypotheticalFutureLayer` — all 13 real variants are, by design, already covered, so no real
    variant can demonstrate a gap) proves the discovery pipeline correctly flags an uncovered cell.
  - `resolver_follows_a_const_to_const_alias_not_just_direct_literals` — dedicated non-vacuity test
    for the alias-following mechanism itself: confirms `WfpEgressFilters` (whose registry row
    references an ALIASED const, not a direct literal) is correctly discovered.
  - `minifilter_absence_classifies_not_applicable_never_unconfirmed` — D-21 pinned test: asserts
    `DAEMON_UNMODELLED_LAYER_EXPECTANCY`'s `MinifilterAbsence` entry is `NotApplicable`.
  - `unconfirmed_on_proceed_is_representable_in_a_daemon_receipt` — Test 3 (D-16 triage): an
    ancestor-traverse-absent session reaches `Proceed` (WR-06) while its `DaclAncestorTraverse`
    census row still reads `Unconfirmed`; the resulting 13-row receipt serializes without panicking.
  - `sentinel_seeded_expected_package_sid_never_leaks_into_the_serialized_daemon_receipt` — Test 4
    (D-14 half 2, daemon producer): runs the FULL pipeline (`daemon_attest_and_decide` →
    `daemon_census_rows` → `build_daemon_receipt`) with `expected_package_sid` seeded to
    `"C:\Users\SENTINEL-DAEMON-9c2e\secret-project"`, serializes the resulting receipt, and asserts
    neither substring appears in the output.

### Negative-control demonstration (Test 4, per this plan's required evidence — not committed)

Temporarily added `pub raw_expected_package_sid: String` to `crates/nono/src/receipt.rs`'s
`EnforcementReceipt`, threaded a `raw_expected_package_sid_negative_control: String` parameter through
`build_daemon_receipt`, updated the 3 struct literals in `receipt.rs`'s own test module and the 1
struct literal in the CLI's `build_enforcement_receipt` (118-03's producer, also affected by the type
change) to populate the new field with `String::new()`, then threaded the REAL sentinel value through
the sentinel test's call site. Re-ran `sentinel_seeded_expected_package_sid_never_leaks_into_the_serialized_daemon_receipt`
— it **FAILED** as expected, with the full panic transcript showing
`"raw_expected_package_sid":"C:\\Users\\SENTINEL-DAEMON-9c2e\\secret-project"` verbatim inside the
serialized JSON (captured live during execution). All changes (the field, the parameter, both call
sites' `String::new()`/`sentinel.to_string()` args, and the two touched producer files) were then
reverted in full — confirmed via `git checkout -- crates/nono/src/receipt.rs
crates/nono-cli/src/exec_strategy_windows/attestation.rs` (both files fully clean, zero diff) and
`git diff --stat` showing only `crates/nono-cli/src/agent_daemon/launch.rs` modified afterward. The
full `attestation::` test suite (Plan 118-03's 47 tests) and `receipt::` test suite (5 tests) were
re-run and pass unchanged post-revert.

## Verification

- `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch` — **32 passed**, 0 failed
  (22 pre-existing + 10 new: 2 from Task 1, 6 from Task 2's `daemon_expectancy_cross_check` module,
  and 2 additional non-vacuity tests — `resolver_follows_a_const_to_const_alias_not_just_direct_literals`,
  `minifilter_absence_classifies_not_applicable_never_unconfirmed`). Non-zero passed count confirmed,
  no filtered-to-zero silent pass.
- `cargo test -p nono-sandbox --lib receipt::` — **5 passed**, 0 failed (unchanged from 118-01/118-03;
  confirms `receipt.rs` is byte-identical to its pre-negative-control state after the revert).
- `cargo check -p nono-sandbox-cli --all-targets` — exit 0.
- `cargo clippy -p nono-sandbox-cli --all-targets -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo fmt --all -- --check` — clean (workspace-wide).
- `git status --short` — only `crates/nono-cli/src/agent_daemon/launch.rs` modified across both
  tasks; `crates/nono/src/receipt.rs` and `crates/nono-cli/src/exec_strategy_windows/attestation.rs`
  confirmed unchanged after the negative-control revert.
- **Diligence pass beyond the plan's own gate:** `cargo test -p nono-sandbox-cli --all-targets
  --no-fail-fast` (background run). The `--bin nono` unit-test result matched the documented
  12-failure baseline EXACTLY (`1695 passed; 12 failed`, same 12 names) — zero regressions in the
  primary unit-test binary. Three additional, previously-uncatalogued failures surfaced in unrelated
  integration-test binaries (`audit_verify_reports_signed_attestation_with_pinned_public_key`,
  `rollback_signed_session_verifies_from_audit_dir_bundle`,
  `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified`) — none touch
  `agent_daemon/`, `layer_registry.rs`, `attestation.rs`, or `receipt.rs`; logged to
  `deferred-items.md` per the SCOPE BOUNDARY rule rather than investigated (out of scope for this
  plan's own file). The `windows_run_*` live-run integration suite did not finish within a
  reasonable diligence window on this host — documented elsewhere as an expected ~25-minute stall
  (spawns real `nono.exe` child processes repeatedly), not a hang introduced by this plan.

## TDD Gate Compliance

Both tasks were tagged `tdd="true"`. As in Plan 118-03, the executor committed new production code
(`daemon_decision_from_booleans`/`daemon_census_rows` in Task 1; `DAEMON_UNMODELLED_LAYER_EXPECTANCY`/
`build_daemon_receipt`/the cross-check scanner in Task 2) together with their tests in one commit per
task, rather than separate RED (failing test against a stub)/GREEN (implementation) commits. No
`test(...)` commit precedes a `feat(...)` commit for Task 1's restructure.

Mitigating context, matching 118-03's precedent: every assertion in this plan carries its own
perturbation proof performed LIVE during execution (not merely asserted) — the precedence-order swap
for the equivalence test, the `HypotheticalFutureLayer` synthetic-cell proof, the const-alias-following
non-vacuity check, and the sentinel negative control all failed as expected before being
fixed/reverted. Additionally, one genuine bug was caught and fixed mid-development by exactly this
discipline: the cross-check scanner's initial bracket-position logic searched for `[` starting from
the marker text itself (which already contains a `[LayerRegistryEntry; 13]` type annotation with its
own brackets), causing it to extract the WRONG array body and report zero discovered cells — caught
immediately when the "must be non-empty" guard in Test 1 fired on the real registry, fixed by
anchoring the search on `=` first. Flagging the literal RED/GREEN gate miss per the TDD compliance
instruction rather than silently claiming full compliance.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Cross-check scanner's bracket-position bug (self-caught before commit)**
- **Found during:** Task 2, first test run of `every_daemon_expected_registry_row_has_a_matching_daemon_local_classification`.
- **Issue:** `registry_entry_expectancy_names`'s array-body extraction searched for the array's
  opening `[` starting from the `"const REGISTRY_ENTRIES: [LayerRegistryEntry; 13] = ["` marker
  itself — but that marker text contains an EARLIER `[` (the `[LayerRegistryEntry; 13]` type
  annotation), so the extraction grabbed the wrong span and every downstream check found zero
  entries. All three cross-check tests failed with empty discovered sets.
- **Fix:** Anchor the search on `"const REGISTRY_ENTRIES:"` then find `=` after that, then find `[`
  after the `=` — skipping the type annotation's own brackets entirely.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Commit:** `c22ef174`

**2. [Rule 1 - Bug] Test 2's perturbation proof used a real LayerId that was already classified**
- **Found during:** Task 2, second test run — `synthetic_unrecognised_daemon_cell_is_caught_by_the_discovery_scan`
  failed its OWN self-check ("must NOT already be classified") because `RestrictedToken` (the
  originally-chosen synthetic name) is one of the 8 entries in `DAEMON_UNMODELLED_LAYER_EXPECTANCY`.
- **Fix:** Switched the synthetic to a fictional `LayerId` name (`HypotheticalFutureLayer`) that
  cannot appear in either classification table by construction, since it is not a real Rust enum
  variant — documented in the test's own doc comment why a real variant cannot be used for this
  perturbation (all 13 are, by design, already fully covered).
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Commit:** `c22ef174`

### Orchestrator-Directed Deviations

None — this plan executed within the scope described by its own `<tasks>` block; the prior-wave
context (`LayerId`/`EntryPath` core promotion) was already complete from Plan 118-03.

### Plan-Text Supersession (not a deviation from intent — the plan's own interfaces text predates a later correction)

**3. `daemon_decision_from_booleans`/`daemon_census_rows` take 6 inputs, not the plan's suggested 5**
- The plan's `<action>` text for Task 1 suggested a 5-argument census function shape
  (`app_container_confirmed, job_confirmed, wfp_ok, dacl_ok, ancestor_ok`). A single collapsed
  `wfp_ok` boolean cannot distinguish `NotApplicable` (network scoping not requested for this launch)
  from `Unconfirmed` (requested but the filters were not installed) — collapsing these would violate
  D-13 ("all four states are a FLOOR, not a cap") for the `WfpEgressFilters` census row specifically.
  Both underlying booleans (`network_scoping_required`, `wfp_filters_installed`) — already parameters
  on the untouched `daemon_attest_and_decide` signature — are threaded through instead.
- **entry_path/token_arm are the core enums, not `&'static str`**, matching 118-03's own identical
  supersession note (118-01's post-plan correction retyped `EnforcementReceipt.entry_path`/
  `.token_arm` before this plan executed).

## Cross-Target Clippy Inventory (for Plan 118-10)

`crates/nono-cli/src/agent_daemon/launch.rs` carries `#[cfg(target_os = "windows")]` gates (the
whole `mod windows_impl` block, line 63) — this plan's changes (both tasks) are entirely inside that
gated module and its nested `#[cfg(test)]` submodules. Per CLAUDE.md's cross-target clippy MUST, this
file qualifies for both local gates. Neither gate was run by this plan — deferred to Plan 118-10 per
the phase's own verification strategy (`118-VALIDATION.md`: "Before `/gsd:verify-work`: ... both
cross-target clippy gates").

## Self-Check: PASSED

```
FOUND: crates/nono-cli/src/agent_daemon/launch.rs
FOUND commit: 30a3b2ba
FOUND commit: c22ef174
```

## Threat Flags

None. This plan's changes are exactly the surface its own `<threat_model>` names (T-118-11/T-118-12/
T-118-13/T-118-14) — the `daemon_attest_and_decide` restructure (mitigated by the equivalence proof
above), the daemon-local expectancy table's drift risk against `layer_registry.rs` (mitigated by the
discovery-based cross-check with a perturbation proof), the `Unconfirmed`-on-`Proceed`
representability requirement (accepted-and-documented per D-16, proven representable), and the
`expected_package_sid` leak surface (mitigated by the sentinel round-trip with a documented negative
control). No new network endpoint, auth path, file-access pattern, or schema change at a trust
boundary was introduced. `build_daemon_receipt` is not wired into any live code path in this plan.

## Known Stubs

None. `daemon_decision_from_booleans`, `daemon_census_rows`, `DAEMON_UNMODELLED_LAYER_EXPECTANCY`,
and `build_daemon_receipt` are complete, functioning implementations — not wired into
`launch_agent`'s live gate (explicitly out of scope for this plan, assigned to a later plan), but
that is a documented forward-reference to a later plan's deliverable, not a stub left unfinished in
this plan's own scope.
