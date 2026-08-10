---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-10T00:00:00Z
iteration: 2
depth: deep
files_reviewed: 22
files_reviewed_list:
  - .github/workflows/ci.yml
  - Makefile
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/tests/env_vars.rs
  - crates/nono-cli/tests/layer_force_unavailable.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-shell-broker/src/main.rs
  - crates/nono/src/attestation.rs
  - crates/nono/src/machine_policy.rs
findings:
  critical: 1
  warning: 12
  info: 0
  total: 13
status: issues_found
---

# Phase 117: Code Review Report (iteration 2 — re-review after fix pass)

**Reviewed:** 2026-08-10
**Depth:** deep
**Files Reviewed:** 22 (the `936d1ddb..HEAD` fix delta plus the call sites it depends on)
**Status:** issues_found

## Summary

The fix pass is substantially real. Three of the four findings that broke the
phase's central claim are genuinely closed and I verified them independently
rather than by reading the fix report:

- **CR-02 is fully fixed.** `probe_in_job` now takes a mandatory `JobHandle`
  and rejects null/`INVALID_HANDLE_VALUE` fail-closed
  (`crates/nono/src/attestation.rs:280-287`), so the vacuous "in ANY job" form
  is unreachable by accident. Both mirrors pass a real job handle. The
  both-polarity test against a live suspended child is a genuine non-vacuity
  proof.
- **CR-03 is fully fixed.** The broker gate is hoisted out of
  `if is_app_container` (`nono-shell-broker/src/main.rs:857`), the legacy/PTY
  arm now spawns `CREATE_SUSPENDED` (`:785`), and I traced the spawn lifecycle:
  the gate sits strictly between spawn and the single `ResumeThread` (`:948`),
  every failure path — env-var absent, gate refusal, resume failure — calls
  `TerminateProcess` first, and there is no code path that resumes before the
  gate or that leaves a suspended child un-terminated. `unwrap_or_default()` is
  gone; unknown/empty required-layer names refuse resume; `MandatoryIntegrityLabel`
  is genuinely RID-checked; the AppContainer SID is compared to the expected
  per-run value. 34/34 broker tests pass locally.
- **CR-10 is fully fixed and I confirmed it executes.**
  `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` runs 5 tests
  in a **default** build (it previously compiled to zero), and the feature
  appears in no default-feature set and no release build path — only in the new
  `make` target, the new CI job, and `--all-features` clippy.
- **NEW-01 no longer deletes the tree.** I ran the touched suites; `git status`
  stayed clean.

What does not hold up is the *shape* of the CR-01/CR-09 fix. Both were closed
by replacing a probe that could only say yes with a caller-supplied
`AppliedLayers` report — but `PreparedWindowsLaunch::applied_layers()`
(`mod.rs:356-386`) derives that report from **guard construction**, not guard
**effect**, and hardcodes `mandatory_integrity_label: true` and
`interpreter_coverage_gate: true` as literals. `AppliedLabelsGuard::snapshot_and_apply`
returns `Ok` when it recorded `AppliedLabel::Skip` for **every** path
(`labels_guard.rs:135`, `:159`; its own test at `:540` asserts "Skip is not an
error"). So a direct-CLI launch that applied zero mandatory-label ACEs still
reports the layer as established, classifies it
`EstablishedNotIndependentlyObservable`, is *not* counted as a downgrade after
the CR-09 change, and reaches `Proceed`. That is the phase's stated goal —
"nono can no longer report enforcing while a confinement layer is silently
inert" — unmet on the primary path, with the failure mode relabelled rather
than removed.

The CR-09 fix also inverted rather than corrected the D-27 signal: because no
registry row carries `DegradeWithVisibleClaim` or `FailOpenDefect`, and
`FailOpen` rows are deliberately never pushed into `downgraded`
(`attestation.rs:474-483`), `AttestationDecision::ProceedDowngraded` is now
**unreachable on every production path**. The banner, the dedup marker, the
`LayerAttestationDowngraded` audit event and the WR-11 path validator are all
dead code in the shipped build. Iteration 1 said the signal fired always;
it now fires never.

Beyond that, four of the fix pass's new "deny" branches are only reachable from
synthetic test inputs the production callers cannot construct (NR-04, NR-05),
one new fail-secure claim is not actually fail-secure (NR-03), and the NEW-01
staging guard's "cannot be defeated" claim does not survive a `..` component
(NR-06).

CR-05 and CR-07 are legitimately `ACCEPTED_OPEN`: both record a named operator
decision in the SPEC (RF-03, RF-13), the registry/doc no longer make a false
claim, and CR-07 now emits a loud `RequiredLayersNotEnforced` warning. Neither
was closed by deleting a real capability — `DaclSessionSidGrant`'s deletion is
of a claim that was never backed by a call site, which I re-verified by grep.

## Iteration 1 Disposition

| ID | Verdict | Evidence |
|---|---|---|
| CR-01 `MandatoryIntegrityLabel` vacuous probe | **PARTIAL** | Wrong-object half fixed (row is `ConfiguredOnly`, `layer_registry.rs:847`) and a real RID check landed in the broker (`main.rs:159-175`). But on `DirectCli` the row's status comes from `applied_layers()`'s literal `mandatory_integrity_label: true` (`mod.rs:364`) — still cannot deny. See NR-01. |
| CR-02 `probe_in_job` null job | **RESOLVED** | `attestation.rs:274-304`: `job` mandatory, null/`INVALID_HANDLE_VALUE` rejected fail-closed; `launch.rs:2401` passes `containment.job`, `agent_daemon/launch.rs:874` passes `job_raw_owned`. Both-polarity live test at `attestation.rs:833`. |
| CR-03 broker gate nesting / coverage / fail-open | **RESOLVED** | Gate hoisted (`main.rs:857`), legacy arm `CREATE_SUSPENDED` (`:785`), single `ResumeThread` after the gate (`:948`), all error paths terminate. `BROKER_ATTESTABLE_LAYERS` refusal (`:112-119`), empty-list refusal (`:99-105`), env-absent refusal (`:886-892`). 34/34 broker tests pass. Human PTY verification still outstanding per the fix report. |
| CR-04 `WfpEgressFilters` aborts legit launches | **RESOLVED** | Tri-state `AppliedLayers::wfp_egress_filters`; `classify_row` returns `NotApplicable` for an unselected backend (`attestation.rs:377-380`). Availability defect gone. (Its *deny* direction is production-unreachable — NR-04.) |
| CR-05 `DaclSessionSidGrant` unbacked claim | **ACCEPTED_OPEN** | Expectancy emptied (`layer_registry.rs:646`), `call_sites: &[]`, seam relabelled to `DaclPackageSidGrant`, SPEC row + `MANUALLY_VERIFIED` entry + RF-03 operator decision. Re-verified by grep: no DACL grant of `config.session_sid` anywhere. |
| CR-06 daemon `DaclAncestorReadAttrs` | **RESOLVED** | Dropped from the daemon decision core and from `DACL_ANCESTOR_READ_ATTRS_EXPECTANCY` (`layer_registry.rs:701`, DirectCli-only); SPEC row 58 states the reason. |
| CR-07 `HKLM\...\RequiredLayers` no-op | **ACCEPTED_OPEN** | `warn_if_required_layers_configured` (`machine_policy.rs:676-707`) emits a named warning; type doc carries a NOT-YET-ENFORCED section; RF-13 records the three open decisions. Enforcement genuinely still absent, and now says so. |
| CR-08 four Windows block-net tests | **RESOLVED** | All four + the helper gated on `all(target_os="windows", feature="layer-fault-injection")`; stale `NONO_TEST_HARNESS` comments and `.env` calls removed; a runner exists (CR-10). Runner reliability is a separate new finding (NR-08). |
| CR-09 `Proceed` unreachable / permanent downgrade | **PARTIAL** | `Proceed` is reachable and proven at the real gate (`launch.rs:fully_applied_launch_passes_the_gate`). But the fix inverted the defect: `ProceedDowngraded` is now unreachable in production (NR-02), and the new deny direction is unreachable for every `DirectCli` `ConfiguredOnly` row (NR-01). |
| CR-10 fault-injection feature nothing enables | **RESOLVED** | Verified live: `layer_registry_meta_test` runs 5 tests in a default build; `layer_registry_selfcheck` 3/3. `make test-layer-fault-injection` + `windows-layer-fault-injection` CI job added. Feature absent from `default`, present only in `--all-features` clippy — no release-build leak. |
| WR-01 probes accept any value | **RESOLVED** | Broker (`main.rs:130-145`), daemon (`agent_daemon/launch.rs:1362-1366`), CLI `RestrictedToken` (`attestation.rs:295-308`) all compare against the expected value. Positive-direction coverage gap is NR-07. |
| WR-02 daemon drops `WfpEgressFilters` | **PARTIAL** | Parameterised and abortable in the unit test, but the only shipped call site passes `network_scoping_required` for **both** parameters (`agent_daemon/launch.rs:880-883`), making the predicate a constant `false`. See NR-05. |
| WR-03 `host_gated_rows_are_loud` cannot fail | **RESOLVED** | `manual_verification_section` slices at the heading; both loudness tests use it; `manual_verification_section_excludes_the_registry_table` is a real non-vacuity guard. All pass in a default build. (Minor ordering nit: NR-10.) |
| WR-04 meta-test accepts a function name; `MANUALLY_VERIFIED` unbounded | **NOT_RESOLVED** | Skipped by the fixer. The check is still `test_src.contains("fn force_unavailable_…")`, and the allow-list **grew** from 10 to 11 of 13 rows (`layer_registry_meta_test.rs:151-273`). |
| WR-05 duplicate DACL test | **RESOLVED** | One correctly-labelled `force_unavailable_dacl_package_sid_grant`; the shared seam now reports `DaclPackageSidGrant` (`dacl_guard.rs:148`) and its in-crate test asserts that name. |
| WR-06 stale call-site citations | **NOT_RESOLVED (worsened)** | Skipped. Actual positions today: `AppliedLabelsGuard` `mod.rs:479` (cited `:425`), `AppliedDaclGrantsGuard` `mod.rs:493` (cited `:436-440`), `AppliedAncestorTraverseGuard` `mod.rs:507` (cited `:449-455`), `AppliedAncestorReadAttributesGuard` `mod.rs:531` (cited `:473-482`) — every citation drifted a further ~54 lines, and the SPEC table repeats them. |
| WR-07 daemon `KILL_ON_JOB_CLOSE` abort path | **RESOLVED** | Explicit `TerminateProcess(process_handle_raw, 1)` before `cleanup_failed_agent` (`agent_daemon/launch.rs:962`). |
| WR-08 `DaemonAttestationDecision::Proceed` never constructed | **RESOLVED** | Returned at `agent_daemon/launch.rs:1401`; asserted by `real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`. |
| WR-09 expectancy under-models DACL rows | **RESOLVED** | `DACL_PACKAGE_SID_SCOPED_EXPECTANCY` covers all five `DirectCli` arms + `Daemon` (`layer_registry.rs:660-691`), matching `execution_runtime.rs:608`'s unconditional `package_sid: Some(..)` (re-verified). |
| WR-10 SPEC self-contradictions | **RESOLVED** | Structural-constraints paragraph reconciled with the `MinifilterAbsence` carve-out (SPEC:157-162); SC4-2/SC4-4 moved to RESOLVED with the historical Finding text preserved; `spec_matches_registry` + `host_gated_rows_are_loud` pass. No real capability was deleted to silence the finding. |
| WR-11 unvalidated `session_id` in a path | **RESOLVED** | `session_id_is_safe_path_component` is an ASCII allow-list (`output.rs:214-221`); `rejects_traversal_and_separator_shapes` drives 13 hostile shapes and asserts `None` from the marker helper. Verified passing. |
| WR-12 three cores, no conformance test | **PARTIAL** | Assertion (b) landed (`every_emitted_broker_required_layer_is_attestable_by_the_broker`); (c) is moot. Assertion (a) — the broker's `"NONO_BROKER_REQUIRED_LAYERS"` literal vs `BROKER_REQUIRED_LAYERS_ENV_VAR` — is still unasserted; a rename on either side still silently disconnects the wire contract (now producing a hard refusal rather than an unattested launch). |
| NEW-01 `Drop` deleting `crates/nono-cli/` | **PARTIAL** | The recursive delete is contained (`network.rs:197-209`, `Path::starts_with`, root itself refused) and I confirmed the tree stays clean across the suites. But the guard does not reject `..` components or canonicalize, so its stated "cannot be turned into an arbitrary recursive delete" property does not hold. See NR-06. |

## Critical Issues

### CR-14 (BLOCKER): `AppliedLayers` reports guard *construction*, not guard *effect* — the DirectCli attestation still cannot deny, and a launch with zero mandatory-label ACEs is reported as fully attested

**File:** `crates/nono-cli/src/exec_strategy_windows/mod.rs:356-386`,
`crates/nono-cli/src/exec_strategy_windows/labels_guard.rs:107-165`, `:200`,
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:394-414`, `:441-460`

**Issue:** CR-01 and CR-09 were both closed by moving `ProbeKind::ConfiguredOnly`
rows off an unconditional free pass and onto a caller-supplied
`layer_registry::AppliedLayers` report. The report is produced here:

```rust
// mod.rs:356
fn applied_layers(&self) -> layer_registry::AppliedLayers {
    layer_registry::AppliedLayers {
        mandatory_integrity_label: true,          // <-- literal
        dacl_package_sid_grant: self._applied_dacls.is_some(),
        dacl_ancestor_traverse: self._applied_ancestor_traverse.is_some(),
        dacl_ancestor_read_attrs: self._applied_ancestor_read_attrs.is_some(),
        ...
        interpreter_coverage_gate: true,          // <-- literal
    }
}
```

Every field is a function of *whether a guard object exists*, never of what the
guard achieved. Trace each one on a production `DirectCli` launch:

- `mandatory_integrity_label` is the constant `true`.
- `interpreter_coverage_gate` is the constant `true`.
- the three DACL fields are `config.package_sid.is_some()` — and
  `execution_runtime.rs:608` sets `package_sid: Some(windows_package_sid)`
  **unconditionally** (the same fact WR-09's fix relies on), so all three are
  always `true` whenever `prepare_live_windows_launch` returns `Ok`.
- the two network fields and the Authenticode field mirror the
  `_network_enforcement` discriminant (see NR-04).

So on every shipped `DirectCli` launch, **every** `ConfiguredOnly` row
classifies `Applied` → `EstablishedNotIndependentlyObservable`, which
`decide_from_entries` now treats as the expected baseline and `continue`s past
(`attestation.rs:449-459`). The `Unconfirmed` → `Abort` branch that CR-09's fix
exists to create is unreachable outside `#[cfg(test)]` fixtures; the two tests
that prove it (`unapplied_configured_only_row_aborts`,
`launch_missing_an_expected_configured_only_layer_is_refused`) construct an
`AppliedLayers` value that no production caller can produce.

The `mandatory_integrity_label: true` literal is worse than "merely
unreachable", because the guard it stands for is documented to succeed while
doing nothing:

```rust
// labels_guard.rs (snapshot_and_apply)
guard.entries.push(AppliedLabel::Skip);   // :135  pre-existing label
...
guard.entries.push(AppliedLabel::Skip);   // :159  non-user-owned system path
```

and its own test asserts `snapshot_and_apply must succeed (Skip is not an
error)` (`labels_guard.rs:540`). A workspace where every compiled
filesystem-policy path already carries a label, or is not user-owned, produces
a guard with an all-`Skip` entry list, `Ok(...)`, and therefore
`mandatory_integrity_label: true`. The gate then returns
`AttestationDecision::Proceed`, the banner does not print (NR-02), and the
session reports as fully attested — **while not a single NO_WRITE_UP ACE was
applied.** That is verbatim the failure mode the phase exists to eliminate, and
it is the same "the guard can only ever say yes" defect iteration 1 raised as
CR-01, relabelled from a live probe to a hardcoded application report.

**Fix:** make the report a fact about the applied state, not about object
existence, and let the guards own that fact:

```rust
// labels_guard.rs
impl AppliedLabelsGuard {
    /// True only when at least one mandatory-label ACE was actually written.
    pub(crate) fn applied_any(&self) -> bool {
        self.entries.iter().any(|e| !matches!(e, AppliedLabel::Skip))
    }
    /// True when every policy path is either labelled by us or was already
    /// labelled at or below the required level.
    pub(crate) fn covers_every_policy_path(&self) -> bool { /* ... */ }
}

// mod.rs
mandatory_integrity_label: self._applied_labels.covers_every_policy_path(),
interpreter_coverage_gate: self.coverage_gate_ran,   // set from the real call
dacl_package_sid_grant: self._applied_dacls
    .as_ref()
    .is_some_and(dacl_guard::AppliedDaclGrantsGuard::granted_every_writable_path),
```

Then add a gate-level test that drives a real launch whose label guard recorded
only `Skip` and asserts `Err(LayerAttestationFailed { layer:
"MandatoryIntegrityLabel", .. })`. Without such a test the row's deny direction
remains unproven against production inputs. If an all-`Skip` label set is
*deliberately* acceptable, the row must not be reported as established —
`DegradeWithVisibleClaim` is the honest outcome, and it would also make NR-02's
channel live.

## Warnings

### NR-02 (WARNING): `ProceedDowngraded` — and with it the entire D-27 banner/telemetry channel — is now unreachable on every production path

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:428-515`,
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:802-993`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1480-1525`,
`crates/nono-cli/src/output.rs:159-221`

**Issue:** `downgraded` is pushed to from exactly two arms:
`ContractOutcome::DegradeWithVisibleClaim` and
`ContractOutcome::FailOpenDefect` (`attestation.rs:461-473`). No row in
`REGISTRY_ENTRIES` uses either value — the registry's own doc comment says so
twice (`layer_registry.rs:322-325`, `:340-342`). The one `FailOpen` row
(`MinifilterAbsence`) is explicitly **not** added to `downgraded`
(`attestation.rs:474-483`), and its expectancy cell is `(DirectCli, None)`
which `token_arm_matches(Some(arm), None)` never matches anyway, so it
classifies `NotApplicable` before reaching the outcome match.

Therefore `downgraded.is_empty()` is always true and
`AttestationDecision::ProceedDowngraded` is never constructed in a shipped
build. Everything hanging off it is dead code on every production path:
`print_attestation_downgrade_banner`, the per-session dedup marker (and the
WR-11 validator that guards it), `emit_attestation_event`, and the
`LayerAttestationDowngraded` audit record. Iteration 1's CR-09 said the signal
fired on every launch; it now fires on none. Combined with CR-14, the only two
outcomes a shipped `DirectCli` launch can produce are `Proceed` and `Abort` —
there is no state in which nono tells an operator "this session is running with
a partially confirmed confinement claim", which is the D-27 deliverable.

**Fix:** either give at least one row an outcome that can produce a downgrade
(the honest home for a `ConfiguredOnly` row whose apply was a no-op — see
CR-14), or, if the design is genuinely "abort or fully attested", delete
`ProceedDowngraded`, the banner, the dedup marker and the telemetry event
rather than shipping an operator channel that structurally cannot fire. Add a
test that asserts the chosen answer:

```rust
#[test]
fn some_production_configuration_can_produce_a_downgrade() {
    let downgradable = layer_registry::all_entries().iter().any(|e| matches!(
        e.outcome,
        ContractOutcome::DegradeWithVisibleClaim | ContractOutcome::FailOpenDefect { .. }
    ));
    assert!(downgradable, "no row can ever downgrade — the D-27 channel is dead code");
}
```

---

### NR-03 (WARNING): `AppliedLayers`'s documented fail-secure default is false for its three tri-state fields

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:401-445`

**Issue:** The type's doc comment states plainly:

> Defaults to `NotApplied` for every field, NOT `Applied` — a caller that
> forgets to report a layer fails secure.

That is true for the five `bool` fields, but `firewall_rules_egress`,
`wfp_egress_filters` and `broker_authenticode_trust_gate` are `Option<bool>`,
whose `Default` is `None`, and `from_tristate(None)` maps to
`LayerApplication::NotApplicable` (`:440-445`) — which `classify_row` turns
into `LayerAttestationStatus::NotApplicable`, i.e. the row is dropped from the
decision entirely. A caller that forgets to report the resolved network backend
does not fail secure; it silently removes `WfpEgressFilters` and
`FirewallRulesEgress` from the session's attestation. This is the same
"unreported means fine" default the CR-09 fix was written to eliminate,
reintroduced for three of eight fields, with a doc comment asserting the
opposite.

**Fix:** make "not reported" distinguishable from "not part of this
composition" at the type level, so the compiler forces the caller to say which:

```rust
pub(crate) enum NetworkBackendReport {
    Unreported,                 // -> NotApplied (fail-secure)
    NotSelected,                // -> NotApplicable
    Selected { installed: bool },
}
```

and default the field to `Unreported`. At minimum, correct the doc comment so
it does not claim a property the type does not have.

---

### NR-04 (WARNING): the network rows can never classify `Unconfirmed` in production — CR-04's deny test uses an input `spawn_windows_child` cannot construct

**File:** `crates/nono-cli/src/exec_strategy_windows/mod.rs:370-377`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1538-1543`,
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:372-393`, `:816-850`

**Issue:** `applied_layers()` derives `wfp_egress_filters` from the
`_network_enforcement` discriminant, and `derive_wfp_preconfirmed`
(`launch.rs:1538`) derives `wfp_preconfirmed` from the **same** discriminant.
They are structurally in lockstep at the only production call site:

| `_network_enforcement` | `applied.wfp_egress_filters` | `wfp_preconfirmed` | classification |
|---|---|---|---|
| `Some(WfpServiceManaged)` | `Some(true)` | `true` | `Confirmed` |
| `Some(FirewallRules)` | `None` | `false` | `NotApplicable` |
| `None` | `None` | `false` | `NotApplicable` |

The `Unconfirmed` cell — `applied = Some(_)` **and** `wfp_preconfirmed = false`
— is unreachable. `network_row_for_an_unselected_backend_is_not_applicable`
(`attestation.rs:838-849`) reaches it only by hand-setting
`wfp_egress_filters: Some(false)` alongside `wfp_preconfirmed: false`, a pair
no caller produces. `FirewallRulesEgress` has the identical shape
(`Some(true)`/`None`, never `Some(false)`), so it too can only ever be
`Applied` or `NotApplicable`.

The availability half of CR-04 is genuinely fixed. The claim that the row "can
still abort when the backend WAS selected and did not confirm" is test-only.

**Fix:** report the two facts independently — thread the actual
`assert_wfp_activation_installed_filters` result (and the `netsh` rule-add
result) into `AppliedLayers` rather than re-deriving both from the same enum
discriminant, so "backend selected but enforcement not confirmed" becomes a
representable production state.

---

### NR-05 (WARNING): the daemon's new `WfpEgressFilters` abort predicate is a compile-time constant `false` at its only call site

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:874-884`, `:1385-1390`

**Issue:** WR-02's fix added

```rust
if network_scoping_required && !wfp_filters_installed { /* Abort */ }
```

but the single production call site passes the same local for both parameters:

```rust
daemon_attest_and_decide(
    process_handle_raw,
    job_raw_owned,
    &package_sid,
    true,                       // dacl_guard_applied — literal
    network_scoping_required,
    network_scoping_required,   // wfp_filters_installed
)
```

`x && !x` is always `false`, so the row can never abort in production. The same
applies to `dacl_guard_applied`, passed as a literal `true`. The justification
comments ("reached only if `wfp_filter_add` succeeded", "`dacl_guard` exists, so
apply ran") are correct about *why* the value is `true` today, but they make the
new parameters documentation rather than checks — the daemon mirror's deny
direction is, like CR-14's, unreachable outside the unit test.

**Fix:** capture the real outcomes rather than re-deriving them from the gate
condition:

```rust
let wfp_filters_installed = if network_scoping_required {
    match wfp_filter_add(&package_sid, &tenant_id, proxy_port) { Ok(()) => true, Err(e) => { /* ... */ false } }
} else { false };
```

so the two values can diverge, then pass `dacl_guard.is_some()` (or an
`applied_any()`-style accessor) instead of `true`.

---

### NR-06 (WARNING): the NEW-01 staging-root guard does not reject `..` or canonicalize, so its stated property does not hold

**File:** `crates/nono-cli/src/exec_strategy_windows/network.rs:174-209`, `:211-222`

**Issue:** The guard is correct about the thing CLAUDE.md footgun #1 warns
about — it uses `Path::starts_with` (component comparison), not string
`starts_with`. But `Path::starts_with` compares components **literally**,
without resolving `..`. A `staged_dir` of
`%TEMP%\nono-net-block\..\..\Windows` has components
`[C:\, Users, …, Temp, nono-net-block, .., .., Windows]`, which
`starts_with(%TEMP%\nono-net-block)` accepts — and `remove_dir_all` then
resolves the `..` and recurses outside the staging root. The equality check
`staged_dir == root` is likewise literal, so `%TEMP%\nono-net-block\.` passes it
and deletes the root. The doc comment's claim — "A `Drop` impl must never be
able to turn a mis-constructed guard into an arbitrary recursive delete" — is
therefore not achieved for exactly the class of mis-constructed value the
finding was about.

`cleanup_stale_network_enforcement_artifacts` (`:211-222`) additionally feeds
`read_dir` entry paths straight into the same function, so a directory junction
placed under the staging root by anything with write access to `%TEMP%` is
worth considering as well.

**Fix:** reject traversal components explicitly and prefer the canonicalized
form when it exists:

```rust
use std::path::Component;
if staged_dir.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
    tracing::warn!(...); return;
}
let root = network_enforcement_staging_root();
let (Ok(real_root), Ok(real_dir)) = (root.canonicalize(), staged_dir.canonicalize()) else { return; };
if real_dir == real_root || !real_dir.starts_with(&real_root) { tracing::warn!(...); return; }
```

Also assert the refusal in a test (`cleanup_network_enforcement_staging(Path::new("."))`
must leave `.` intact) — nothing currently pins the guard's behaviour.

---

### NR-07 (WARNING): `RestrictedToken`'s new SID-equality check has no positive-direction test, and a mismatch aborts every `WriteRestricted` launch

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:295-308`,
`crates/nono-cli/src/exec_strategy_windows/restricted_token.rs:21-32`, `:205-254`

**Issue:** WR-01 changed the row from "any non-empty restricting-SID list
confirms" to "this launch's `config.session_sid` must appear among them,
compared with `eq_ignore_ascii_case`". Correct in direction, but the comparison
now depends on an untested string round-trip:
`generate_session_sid()` → `ConvertStringSidToSidW` → the token →
`ConvertSidToStringSidW` → the probe's `String`. The only new test
(`restricted_token_requires_this_launch_own_session_sid`) asserts the **negative**
directions (unrestricted token; no expected value). `create_restricted_token_with_sid_applies_write_restricted_flag`
asserts `GroupCount == 1` but never compares the rendered SID string.

`WriteRestricted` is `select_windows_token_arm`'s default non-PTY supervised arm
(no PTY, `prefers_low_il_broker == false`, session SID present), and the row's
outcome is `Abort`. If the round-trip ever differs — a formatting difference, a
future change to `generate_session_sid`'s field widths — every default
supervised Windows launch is refused. The fix report states this arm could not
be exercised on the fix host, so the positive direction is unvalidated in both
tests and manual runs.

**Fix:** add the paired positive assertion to the existing token test, which
already has the token in hand:

```rust
let sids = nono::attestation::probe_restricted_sids(unsafe { GetCurrentProcess() })?; // or via a spawned child
assert!(sids.iter().any(|s| s.eq_ignore_ascii_case(&sid)),
        "the restricting SID must render back to the exact string the launch was built with: {sids:?}");
```

---

### NR-08 (WARNING): the new `windows-layer-fault-injection` CI job runs the whole crate's suite unverified and is likely red on first run

**File:** `.github/workflows/ci.yml:308-346`, `Makefile:70-83`

**Issue:** The job runs the *entire* `-p nono-sandbox-cli` test set with the
feature enabled, not a scoped selection. Three concrete risks, none of which was
exercised before the job was added:

1. The fix report itself records "the known 11 pre-existing `nono-sandbox-cli`
   Windows failures were not re-swept end-to-end". Any of those that reproduce
   on `windows-latest` make this job red for reasons unrelated to the phase.
2. The sibling `windows-security` job sets `NONO_CI_HAS_WFP: true` for
   `windows-latest`; the new job sets no env at all. The four block-net tests
   CR-08 moved into this job depend on WFP behaviour on the runner.
3. Two in-crate tests spawn a pre-built `nono-shell-broker.exe` from
   `target/{release,x86_64-pc-windows-msvc/release}` and `panic!` when it is
   absent (`launch.rs:3776`, `:4588`). The job never builds it, and the other
   Windows jobs provision artefacts via `scripts/windows-test-harness.ps1`,
   which this job does not use.

A gate that is red from day one gets marked non-required or removed, which
restores exactly the CR-10 state it was added to fix.

**Fix:** scope the job to what CR-10 actually needs and provision its
preconditions, e.g.

```yaml
- name: Pre-build the broker (two in-crate tests require it)
  run: cargo build -p nono-shell-broker --release
- name: Run layer-fault-injection suites
  env:
    NONO_CI_HAS_WFP: true
  run: |
    cargo test -p nono-sandbox-cli --features layer-fault-injection --test layer_force_unavailable --test layer_registry_meta_test -- --test-threads=1
    cargo test -p nono-sandbox-cli --features layer-fault-injection --bin nono exec_strategy -- --test-threads=1
    cargo test -p nono-shell-broker --features layer-fault-injection -- --test-threads=1
```

and widen it once the job is green.

---

### NR-09 (WARNING): the broker's required-layers contract travels on an environment variable the confined grandchild inherits

**File:** `crates/nono-shell-broker/src/main.rs:754`, `:786`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1614-1628`

**Issue:** Both broker spawn shapes pass `lpEnvironment: std::ptr::null()`
("inherit broker env"), and the broker's own environment carries
`NONO_BROKER_REQUIRED_LAYERS`. The confined AppContainer/Low-IL grandchild can
therefore read the exact list of confinement layers the supervisor requires.
D-28's stated property is that layer-specific detail stays off channels the
confined process can read (it is why the D-27 banner carries only a count), and
`telemetry/event.rs` carries a manual-verification item asserting the same
property for the Application Event Log. The wire contract is a straightforward
counterexample, and CR-03's fix made this variable the gate's sole input, so it
is now load-bearing rather than incidental.

**Fix:** strip the variable from the grandchild's environment before spawning —
build an explicit environment block for the grandchild rather than inheriting,
or `std::env::remove_var(BROKER_REQUIRED_LAYERS_ENV_VAR)` in the broker
immediately after the read at `main.rs:886` and before the two `CreateProcess*`
calls. Add a test that the spawned grandchild's environment does not contain the
name.

---

### NR-10 (WARNING): `manual_verification_section` can select the wrong heading

**File:** `crates/nono-cli/tests/layer_registry_meta_test.rs:335-346`

**Issue:** The line-start iterator is built as
`match_indices('\n').map(|(i,_)| i+1).chain(std::iter::once(0))` — index `0` is
appended **after** every other line start, so the iterator is not in ascending
order. If the SPEC's very first line were ever a `## ` heading containing
"manual" or "host-gated", `find` would return a later heading instead, silently
narrowing (or widening) the searched section. Harmless against today's document,
but the helper is the load-bearing part of WR-03's fix.

**Fix:** `std::iter::once(0).chain(spec.match_indices('\n').map(|(i, _)| i + 1))`.

---

### WR-04-R (WARNING, carry-forward): `every_registry_row_has_a_test` still accepts a function name, and `MANUALLY_VERIFIED` grew

**File:** `crates/nono-cli/tests/layer_registry_meta_test.rs:151-273`, `:293-324`

**Issue:** Unchanged from iteration 1 (the fixer skipped it), but the blast
radius grew rather than shrank: the allow-list went from 10 to 11 of 13 rows
with the addition of `DaclSessionSidGrant`, so 85% of the registry is now
"covered" by prose. The remaining check is still
`test_src.contains("fn force_unavailable_<snake>")` — an empty stub body
satisfies it. Now that the gate actually executes (CR-10), strengthening it is
cheap and the payoff is real.

**Fix:** as iteration 1 — additionally assert the row's env-var bridge name
appears in `command_runtime.rs`, and add a hard cap so the allow-list cannot
keep growing silently:

```rust
assert!(MANUALLY_VERIFIED.len() <= 11,
        "MANUALLY_VERIFIED must not grow without an explicit decision — {} entries",
        MANUALLY_VERIFIED.len());
```

---

### WR-06-R (WARNING, carry-forward): every registry call-site citation drifted a further ~54 lines, and the SPEC repeats the stale numbers

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:814-818`,
`:885-890`, `:898`, `:906`, `proj/SPEC-windows-fail-direction-contract.md:53`,
`:56-58`, `crates/nono-cli/tests/layer_registry_selfcheck.rs:134-138`

**Issue:** The fix pass added ~45 lines to `mod.rs` above the guard block.
Measured against the current tree:

| Cited | Actual |
|---|---|
| `mod.rs:425` (`AppliedLabelsGuard::snapshot_and_apply`) | `mod.rs:479` |
| `mod.rs:436-440` (`AppliedDaclGrantsGuard`) | `mod.rs:490-494` |
| `mod.rs:449-455` (`AppliedAncestorTraverseGuard`) | `mod.rs:503-509` |
| `mod.rs:473-482` (`AppliedAncestorReadAttributesGuard`) | `mod.rs:527-536` |

`registry_call_sites_exist` only asserts the *file* exists, and its
`// TODO(117-12)` acknowledging line drift is still open. The SPEC table and the
`MANUALLY_VERIFIED` reasons now repeat the same stale numbers, so the drift is
in three places for a document that declares itself "the source of truth".

**Fix:** as iteration 1 — cite stable symbols
(`"mod.rs::prepare_live_windows_launch/AppliedLabelsGuard"`) and have
`registry_call_sites_exist` assert the file contains the symbol. If the format
change is deferred again, at minimum re-run the numbers so the citations are
correct on the day they ship.

---

### WR-12-R (WARNING, carry-forward): nothing asserts the broker's env-var literal equals `BROKER_REQUIRED_LAYERS_ENV_VAR`

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:117`,
`crates/nono-shell-broker/src/main.rs:886`

**Issue:** The two sides are still paired only by a comment and by the string
matching. The failure direction improved (a rename now produces a hard refusal
rather than an unattested launch, because the broker fail-closes on an absent
variable), so this is no longer a fail-open. It is now an availability
landmine: renaming the constant on the nono-cli side silently breaks every
broker-arm launch with a message that names the *variable*, not the rename.

**Fix:** a source-text conformance test is enough and needs no shared crate —
`layer_registry_selfcheck.rs` already reads sibling sources:

```rust
#[test]
fn broker_env_var_literal_matches_the_cli_constant() {
    let broker_src = std::fs::read_to_string(
        workspace_root().join("crates/nono-shell-broker/src/main.rs")).unwrap();
    let cli_src = std::fs::read_to_string(
        manifest_dir().join("src/exec_strategy_windows/attestation.rs")).unwrap();
    let name = cli_src.split("BROKER_REQUIRED_LAYERS_ENV_VAR: &str = \"")
        .nth(1).and_then(|s| s.split('"').next()).expect("constant");
    assert!(broker_src.contains(&format!("std::env::var(\"{name}\")")),
            "nono-shell-broker does not read {name} — the wire contract is disconnected");
}
```

---

_Reviewed: 2026-08-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep — iteration 2 (re-review)_
