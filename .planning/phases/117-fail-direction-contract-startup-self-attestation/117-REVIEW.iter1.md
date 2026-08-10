---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-10T00:00:00Z
depth: deep
files_reviewed: 29
files_reviewed_list:
  - bindings/c/src/lib.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/telemetry/event.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/src/telemetry/windows.rs
  - crates/nono-cli/tests/layer_force_unavailable.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-shell-broker/src/main.rs
  - crates/nono/src/attestation.rs
  - crates/nono/src/diagnostic/codes.rs
  - crates/nono/src/error.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/machine_policy.rs
  - proj/SPEC-windows-fail-direction-contract.md
  - ./CLAUDE.md
findings:
  critical: 10
  warning: 12
  info: 0
  total: 22
status: issues_found
---

# Phase 117: Code Review Report

**Reviewed:** 2026-08-10
**Depth:** deep
**Files Reviewed:** 29
**Status:** issues_found

## Summary

Phase 117 ships a 13-row Windows fail-direction registry, three mirrored
attestation decision cores (`attest_and_decide`, `daemon_attest_and_decide`,
`app_container_resume_gate`), a compiled-out `layer-fault-injection` feature,
and a D-27 operator/telemetry downgrade channel.

The **fault-injection migration is clean**: every seam (`WINDOWS_WFP_TEST_FORCE_READY`,
`RESTRICTED_TOKEN_FORCE_UNAVAILABLE`, `MANDATORY_LABEL_FORCE_UNAVAILABLE`,
`DACL_GRANT_FORCE_UNAVAILABLE`, `JOB_OBJECT_FORCE_UNAVAILABLE`,
`APP_CONTAINER_FORCE_UNAVAILABLE`, the daemon variant) plus the Plan-12
env-var bridge in `command_runtime.rs` is behind `#[cfg(feature = "layer-fault-injection")]`
with no residual runtime reachability. The `NONO_TEST_HARNESS` runtime gate is
gone. The `LayerAttestationDowngraded` telemetry event carries only layer names
(no paths, args, or payload). Both are correct and verified.

Everything else does not hold up. The three decision cores diverge from each
other and from the registry; two of the four live probes are structurally
incapable of returning a negative result; the broker's gate is nested inside a
predicate that already excludes the failure case it exists to catch; two rows
are attested as "established" on paths where the layer is never applied; the
D-26 machine-policy tightening surface is never read from the registry; and the
entire CINT-03 forced-unavailable test suite plus the D-32 drift gate are behind
a feature no build, Makefile target, or CI workflow enables — so none of the
phase's own evidence executes.

Net effect: on a default Windows build, the attestation pass reaches `Proceed`
on **zero** code paths, aborts legitimate `BrokerLaunchNoPty` launches, and
confirms `MandatoryIntegrityLabel` for an entirely unconfined Medium-IL
`cmd.exe`. The phase's own test
(`confirmed_live_probes_with_configured_only_row_downgrades_without_aborting`,
`launch.rs:3372`) demonstrates the last point directly.

## Critical Issues

### CR-01: `MandatoryIntegrityLabel` probe is vacuous and measures the wrong object

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:262-270`,
`crates/nono/src/attestation.rs:122-229`, `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:212-214`

**Issue:** Two independent defects compound.

1. **Predicate width — the deny branch is unreachable.** The classifier is:
   ```rust
   LayerId::MandatoryIntegrityLabel => match probe_integrity_level(process) {
       Ok(_rid) => LayerAttestationStatus::Confirmed,
       Err(_) => LayerAttestationStatus::Unconfirmed,
   },
   ```
   `GetTokenInformation(TokenIntegrityLevel)` succeeds for **every** Windows
   token — every process has a mandatory label (Medium by default). The probe
   therefore returns `Ok(rid)` for a fully unconfined Medium-IL child and
   classifies it `Confirmed`. The only way to reach `Unconfirmed` is for the
   OS call itself to fail (invalid handle). The RID value is discarded
   (`_rid`) — nothing compares it against `SECURITY_MANDATORY_LOW_RID`. This
   guard can only ever say yes.

2. **Wrong object.** The registry defines this layer as "NO_WRITE_UP
   (± NO_READ_UP/NO_EXECUTE_UP) mandatory-label ACE on **every compiled
   filesystem-policy path**" (`layer_registry.rs:212-214`) — the thing
   `AppliedLabelsGuard::snapshot_and_apply` writes onto *files*. The probe
   inspects the *child token's* integrity level, a different kernel object
   entirely. Even a correct RID comparison would not attest what the row claims.

The phase's own test proves the impact: `confirmed_live_probes_with_configured_only_row_downgrades_without_aborting`
(`launch.rs:3372`) spawns a plain `cmd.exe /c exit 0` — no label applied to any
path, no Low-IL token — and the gate returns `Ok`, with `MandatoryIntegrityLabel`
classified `Confirmed`. On the `BrokerLaunch`/`BrokerLaunchNoPty` arms the probed
process is `nono-shell-broker.exe` itself, deliberately Medium-IL and unconfined
(`launch.rs:1634-1659`), and it too classifies `Confirmed`.

**Fix:** Either (a) change the row's probe to `ProbeKind::ConfiguredOnly` so it
honestly reports `EstablishedNotIndependentlyObservable` from the
`AppliedLabelsGuard` apply result, or (b) if a token-level check is wanted, make
it a *distinct* row and compare the RID:
```rust
LayerId::MandatoryIntegrityLabel => match probe_integrity_level(process) {
    // SECURITY_MANDATORY_LOW_RID == 0x1000
    Ok(rid) if rid <= 0x1000 => LayerAttestationStatus::Confirmed,
    Ok(_) => LayerAttestationStatus::Unconfirmed, // Medium/High = not lowered
    Err(_) => LayerAttestationStatus::Unconfirmed,
},
```
and exclude the row from the `BrokerLaunch`/`BrokerLaunchNoPty` arms, where the
observed process is the broker, not the confined grandchild (same Blocker-1
reasoning already applied to `AppContainerProfile`).

---

### CR-02: `JobObjectContainment` probe accepts membership in *any* job, not nono's containment job

**File:** `crates/nono/src/attestation.rs:248-269`,
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:260`

**Issue:** `probe_in_job` passes a **null** job handle to `IsProcessInJob`,
which asks "is this process in ANY job". The supervisor holds the real answer —
`containment.job` (`ProcessContainment`, `mod.rs:684-686`) — and does not use
it. On Windows 8+ nested-job hosts (shells, terminals, CI harnesses) a child
inherits its parent's job membership, so a process that was never assigned to
nono's containment job still returns `true` and classifies `Confirmed`.

This is not speculative: the executor documented the exact behaviour in a test
comment (`crates/nono/src/attestation.rs:674-688` and `launch.rs:3283-3290`,
"a REAL spawned-but-unassigned process is NOT a reliable way to force
`JobObjectContainment` unconfirmed, since it may silently inherit the test
runner's own job membership") and then shipped the vacuous probe as production.

**Fix:** Thread the containment job handle into the gate and probe against it:
```rust
// nono/src/attestation.rs
pub fn probe_in_job(process: ProcessHandle, job: ProcessHandle) -> Result<bool> {
    ... IsProcessInJob(process, job, &mut in_job) ...
}
```
`apply_startup_attestation_gate` already has `containment` in scope at its call
site (`launch.rs:2351`); pass `containment.job` through `AttestationInput`. Same
change for `daemon_attest_and_decide` (`agent_daemon/launch.rs:1325`), which has
`job_guard.0` available.

---

### CR-03: Broker resume gate is nested inside `if is_app_container`, and ignores every required layer except one

**File:** `crates/nono-shell-broker/src/main.rs:724`, `:755-782`, `:339-357`

**Issue:** Three defects in the phase's only broker-arm attestation.

1. **Class coverage — the gate never runs on the legacy/PTY broker arm.** The
   entire block, including `app_container_resume_gate`, sits inside
   `if is_app_container { ... }` (`main.rs:724`). The `BrokerLaunch` (PTY) arm
   spawns the broker **without** `--app-container-name`
   (`launch.rs:1837-1849` builds no such arg), so `app_container_sid` is `None`
   (`main.rs:418-422`), `is_app_container` is `false`, and `ResumeThread`
   (`main.rs:784` is skipped entirely — the legacy path falls straight through to
   `WaitForSingleObject` on an already-running child). nono-cli nevertheless sets
   `NONO_BROKER_REQUIRED_LAYERS` for **both** broker arms
   (`launch.rs:1592-1604`), so the wire contract demands
   `AppContainerProfile` be confirmed and the broker silently never checks it.
   This is the exact "guard nested inside a narrower condition that already
   excludes the failing case" defect.

2. **Coverage — one of two required layers is enforced.**
   `required_layers_for_broker()` emits `MandatoryIntegrityLabel,AppContainerProfile`
   (both are `(Broker, expected: true)` + `Abort`). `app_container_resume_gate`
   matches only the literal `"AppContainerProfile"` (`main.rs:350`) and
   **silently ignores every other name**, including `MandatoryIntegrityLabel`.
   Unlike `attest_and_decide`, which fail-closes on an unrecognized required-layer
   name (`attestation.rs:463-472`), the broker has no such rejection: an
   unknown/unimplemented required layer is a no-op. The SPEC's "Structural
   constraints" section claims these rows are "genuinely attested" through this
   path; one of them is not attested at all.

3. **Fail-open default on missing config.**
   `std::env::var("NONO_BROKER_REQUIRED_LAYERS").unwrap_or_default()`
   (`main.rs:770-771`) turns an absent/unreadable variable into `""`, which
   `app_container_resume_gate` treats as "require nothing" and returns `Ok(())`.
   This is CLAUDE.md footgun #2 (`unwrap_or_default()` on security config =
   no protection) applied to the gate's sole input.

**Fix:**
```rust
// Hoist out of `if is_app_container` so BOTH broker spawn shapes are gated.
let required_layers_raw = std::env::var("NONO_BROKER_REQUIRED_LAYERS")
    .map_err(|_| NonoError::SandboxInit(
        "NONO_BROKER_REQUIRED_LAYERS absent — refusing to resume an unattested \
         child (fail-closed)".into()))?;
// Reject names this binary cannot attest, rather than ignoring them.
const ATTESTABLE: &[&str] = &["AppContainerProfile", "MandatoryIntegrityLabel"];
for name in required_layers_raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
    if !ATTESTABLE.contains(&name) {
        return Err(/* fail-closed: required layer this binary cannot confirm */);
    }
}
```
and add a `MandatoryIntegrityLabel` probe (the broker already applies the label at
`main.rs:747` and can re-read the child token's RID).

---

### CR-04: `WfpEgressFilters` aborts every `BrokerLaunchNoPty` launch that is not WFP-backed

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:513-533`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1476-1481`,
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:320-330`

**Issue:** `WFP_EGRESS_FILTERS_EXPECTANCY` marks the row `expected: true`
**unconditionally** at `(DirectCli, BrokerLaunchNoPty)`. Its status is derived
solely from `wfp_preconfirmed`, which `derive_wfp_preconfirmed`
(`launch.rs:1476-1481`) sets `true` only for
`NetworkEnforcementGuard::WfpServiceManaged`. Two legitimate, non-degraded
configurations produce `false`:

- **`FirewallRules` backend.** `select_network_backend` returns
  `FirewallRulesNetworkBackend` for `(Blocked, FirewallRules)`
  (`network.rs:1514-1517`) — a live, reachable dispatch arm the registry's own
  evidence ledger documents as a "structurally distinct alternative, not a
  fallback chain". `wfp_preconfirmed` is then `false`.
- **No network restriction.** `(AllowAll, None)` returns `Ok(None)`
  (`network.rs:1511-1513`) → no guard → `false`.

In both cases `classify_row` yields `Unconfirmed`, and `decide_from_entries`'s
`Abort` arm fires unconditionally on `Unconfirmed`
(`attestation.rs:366-372`) — the launch is refused with
`LayerAttestationFailed { layer: "WfpEgressFilters" }`. `BrokerLaunchNoPty` is
the primary supervised Windows arm (the `claude-code` / sandbox-the-tools path),
so this breaks legitimate runs.

Note that the executor recognised this exact hazard on the daemon path and wrote
it up in `daemon_attest_and_decide`'s doc comment
(`agent_daemon/launch.rs:1296-1313`: "Calling `attest_and_decide` literally
against that row would abort every daemon-launched agent whose profile does not
request network scoping") — then worked around it only for the daemon, leaving
the direct-CLI path exposed to the very defect it names.

**Fix:** Make the row's expectancy conditional on the resolved network policy
rather than the arm. Minimally, thread the selected backend into
`AttestationInput` and treat "no WFP enforcement requested" as
`NotApplicable`, and `FirewallRules` as the registry's own
`SubstituteEquivalentMechanism { alternate: "firewall-rules-egress" }` (a
vocabulary value the registry declares but no row uses):
```rust
pub enum WfpExpectation { NotRequested, SubstitutedByFirewallRules, RequiredAndConfirmed(bool) }
```

---

### CR-05: `DaclSessionSidGrant` has no enforcing call site — the guard is passed the *package* SID

**File:** `crates/nono-cli/src/exec_strategy_windows/mod.rs:449-453`,
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:219-221`, `:659-666`,
`:502-506`

**Issue:** The registry declares:

> `DaclSessionSidGrant` — "DACL grant of the synthetic per-session SID on
> writable filesystem grants — the `WriteRestricted`-arm double-check companion
> to `RestrictedToken`", expected at `(DirectCli, WriteRestricted)`, call sites
> `dacl_guard.rs:92`, `mod.rs:436-440`.

The cited call site is:
```rust
let applied_dacls = config
    .package_sid                       // <-- PACKAGE SID, not session_sid
    .as_deref()
    .map(|sid| dacl_guard::AppliedDaclGrantsGuard::snapshot_and_apply(&fs_policy, sid))
    .transpose()?;
```
This is the **only** production call to `AppliedDaclGrantsGuard::snapshot_and_apply`
in the tree (verified by grep; all other hits are tests and comments). The
synthetic per-session restricting SID (`config.session_sid`) is used *only* for
`create_restricted_token_with_sid` (`launch.rs:1620`) and never granted on any
DACL.

Consequence: on every `WriteRestricted`-arm launch, `attest_and_decide`
classifies `DaclSessionSidGrant` as `EstablishedNotIndependentlyObservable` —
whose documented meaning is "the apply-time `Result` already happened and
succeeded fail-closed" (`layer_registry.rs:359-364`) — for a grant that was
never applied. That is a false claim of an active confinement layer emitted by
the module whose entire purpose is truthful layer accounting.

**Fix:** Either delete the row (the layer does not exist as described) or make it
real by also granting `config.session_sid` when the `WriteRestricted` arm is
selected. Do not leave a row that reports "established" for an unapplied grant.

---

### CR-06: Daemon attests `DaclAncestorReadAttrs` as established although the daemon never grants read-attributes

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:1335-1341`, `:110-251`,
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:688-695`

**Issue:** `daemon_attest_and_decide` returns, unconditionally:
```rust
DaemonAttestationDecision::ProceedDowngraded {
    downgraded: vec!["DaclPackageSidGrant", "DaclAncestorTraverse", "DaclAncestorReadAttrs"],
}
```
justified by "`DaemonDaclGuard::apply` already succeeded fail-closed at step 6.6"
(`launch.rs:1288-1293`). But `DaemonDaclGuard::apply` (`launch.rs:133-251`)
performs exactly three operations: `grant_sid_traverse_on_path` on read-only
rules, `grant_sid_write_on_path` on the workspace, and
`grant_sid_traverse_on_path` on workspace ancestors. It **never** calls
`grant_sid_read_attributes_on_path` — the `FILE_READ_ATTRIBUTES` grant that
defines `DaclAncestorReadAttrs`. The registry's own call sites for that row
(`dacl_guard.rs:401`, `mod.rs:473-482`) are CLI-side files the daemon binary
does not even link.

The daemon therefore reports a layer as `EstablishedNotIndependentlyObservable`
("apply succeeded, cannot re-observe") when no apply ever occurred on that path.
Same defect class as CR-05, on the daemon mirror.

**Fix:** Drop `DaclAncestorReadAttrs` from the daemon's downgraded set and from
`PACKAGE_SID_SCOPED_EXPECTANCY`'s `(Daemon, None)` cell, or implement the
read-attributes grant in `DaemonDaclGuard::apply`. Then derive the daemon's
downgraded list from the guard's *actual* applied state rather than a hardcoded
`vec![]`.

---

### CR-07: `HKLM\...\RequiredLayers` is never read — the D-26 machine-policy control is a documented no-op

**File:** `crates/nono/src/machine_policy.rs:658-662`, `:169-184`, `:228-241`,
`crates/nono-cli/src/exec_strategy_windows/launch.rs:1454-1455`

**Issue:** `MachineEgressPolicy::required_layers` is introduced with a public
doc comment promising a real control:

> "An admin can require specific composed Windows confinement layers be
> confirmed active by startup self-attestation, fleet-wide; a local CLI flag can
> only add to (never remove from) this set."

The Windows registry reader never reads the sub-key:
```rust
// machine_policy.rs:658-662
let required_layers = RequiredLayersPolicy::default();   // always empty
```
and the only gate call site passes empty slices for both halves of the union:
```rust
// launch.rs:1454-1455
required_layers_override: &[],
machine_required_layers: &[],
```
No `--required-layers` CLI flag exists either (the `cli.rs` diff adds only the
feature gate on `dangerous_force_wfp_ready`). The tighten-only union in
`attest_and_decide` — including its fail-closed unrecognized-name rejection, the
one place where a `FailOpen` row like `MinifilterAbsence` can be escalated to
`Abort` — is unreachable in production.

An administrator who sets the documented registry policy gets no enforcement and
no warning. That is a silently-ignored security control on a fleet-management
surface. Nothing in the shipped code or the SPEC tells the operator the control
is inert.

**Fix:** Implement the reader (mirroring `parse_telemetry_config`'s
degrade-not-abort shape as the doc comment prescribes) and thread
`state.machine_egress_policy.required_layers.required` into
`apply_startup_attestation_gate`. If the read must be deferred, mark the field
`#[doc(hidden)]` / rewrite the doc comment to state plainly that it is not yet
enforced, and add a startup warning when the key is present but unread.

---

### CR-08: Removing `--dangerous-force-wfp-ready` from default builds breaks two Windows tests and makes two others vacuous

**File:** `crates/nono-cli/src/cli.rs:2214-2219`,
`crates/nono-cli/tests/env_vars.rs:854`, `:913`, `:1006`, `:3058`

**Issue:** Plan 04 correctly put the flag behind
`#[cfg(feature = "layer-fault-injection")]`, but four pre-existing Windows
integration tests in `env_vars.rs` still pass `--dangerous-force-wfp-ready` and
are gated only on `#[cfg(target_os = "windows")]` — not on the feature. On a
default `cargo test -p nono-cli` (i.e. `make test` / CI) clap now rejects the
unknown argument and `nono.exe` exits non-zero before doing any work:

- `windows_run_block_net_cleans_up_promoted_wfp_filters_after_exit`
  (`env_vars.rs:913`) asserts `blocked_text.contains("connect failed") ||
  contains("exit code 42")` → **fails**.
- `windows_run_block_net_blocks_probe_connection_through_cmd_host`
  (`env_vars.rs:1006`) asserts the same strings → **fails**.
- `windows_run_block_net_blocks_probe_connection` (`env_vars.rs:854`) asserts
  only `!output.status.success()` → now **passes for the wrong reason** (a clap
  parse error, not a blocked connection). The listener-accept assertion also
  passes trivially because no child ever ran.
- The supervised-rollback blocked-net test (`env_vars.rs:3058`) asserts
  `!success` plus three `!text.contains(...)` negatives → also **passes
  vacuously**.

Two of the project's four Windows block-net enforcement tests are now green
without exercising network enforcement at all. This is the same "test that
cannot fail" failure mode this phase exists to eliminate.

**Fix:** Gate those four tests on the feature
(`#![cfg(all(target_os = "windows", feature = "layer-fault-injection"))]` around
them, or move them into a feature-gated file) **and** add the feature to
whatever CI job is expected to run them — otherwise the block-net coverage is
silently dropped rather than fixed. Also drop the now-false `NONO_TEST_HARNESS`
comments at `env_vars.rs:846-848`, `:906-908`, `:997-999`, `:3051-3053`.

---

### CR-09: `AttestationDecision::Proceed` is unreachable on every production path — the downgrade signal fires on every launch

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:331-338`,
`:365-378`, `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:542`,
`crates/nono-cli/src/agent_daemon/launch.rs:1335-1341`

**Issue:** `ProbeKind::ConfiguredOnly` rows *always* classify
`EstablishedNotIndependentlyObservable` (by design,
`attestation.rs:331-338`), and Deviation 2 routes those into `downgraded`
rather than `Abort`. `FirewallRulesEgress` is `ConfiguredOnly` and
`expected: true` on **all five** `DirectCli` arms
(`FIREWALL_RULES_EGRESS_EXPECTANCY = ALL_DIRECT_CLI_ARMS_EXPECTANCY`,
`layer_registry.rs:542`). Therefore:

- Direct `nono run`, every arm: `downgraded` is non-empty on every successful
  launch (on the default `WriteRestricted` arm it is
  `[DaclSessionSidGrant, FirewallRulesEgress]`). `Proceed` is unreachable.
- Daemon: `daemon_attest_and_decide` has no `Proceed` return path at all — the
  variant is constructed nowhere (silenced only by the module-wide
  `#[allow(dead_code)]` at `agent_daemon/launch.rs:64`).
- Broker: no downgrade concept exists.

Consequences:
1. The **non-silenceable** D-27 banner ("N confinement layer(s) could not be
   fully confirmed at startup") prints on *every* Windows session, forever. An
   operator cannot distinguish a real downgrade from the permanent baseline —
   the signal is destroyed exactly as the SPEC's own dedup rationale warns
   ("would train an operator to ignore the signal entirely").
2. `emit_attestation_event` is explicitly **not** deduplicated
   (`telemetry/mod.rs:416-420`), so the per-tool-call hook path appends a
   `LayerAttestationDowngraded` record to the HMAC audit chain and writes an
   Application Event Log warning on *every tool invocation*.
3. A session can never be reported as fully attested, which is the state the
   phase was built to be able to claim.
4. `FirewallRulesEgress` is reported as downgraded even when the policy selected
   the WFP backend and no `netsh` rule was ever installed — the row is not
   applicable, not degraded (see also CR-04).

**Fix:** Make `ConfiguredOnly` rows' expectancy conditional on the layer
actually having been applied for this launch (thread the guard results in), and
only surface `EstablishedNotIndependentlyObservable` rows in the operator banner
when the set differs from the expected baseline. A permanently-on warning is
functionally identical to no warning.

---

### CR-10: The entire CINT-03 test suite and the D-32 drift gate are behind a feature no build enables

**File:** `crates/nono-cli/tests/layer_force_unavailable.rs:1`,
`crates/nono-cli/tests/layer_registry_meta_test.rs:1`,
`crates/nono-cli/Cargo.toml:45-48`, `Makefile`, `.github/workflows/*`

**Issue:** Both new test files begin with
`#![cfg(all(target_os = "windows", feature = "layer-fault-injection"))]`, and
every in-crate forced-unavailable regression test (`restricted_token.rs:320`,
`labels_guard.rs:416`, `dacl_guard.rs:238`/`:277`/`:318`, `launch.rs:4685`,
broker `main.rs:1602`, daemon `launch.rs:315`) is `#[cfg(feature = "layer-fault-injection")]`.
Grep across `.github/` and `Makefile` returns **zero** references to
`layer-fault-injection`. No workflow, no `make` target, and no default
`cargo test` invocation enables it.

Therefore none of the following ever run in any gate:
- all 3 automated forced-unavailable integration tests,
- all 8 in-crate seam regression tests,
- `every_registry_row_has_a_test` (the D-32 discovery gate),
- `host_gated_rows_are_loud` / `security_assumptions_are_loud`.

This directly falsifies the SPEC's claims: "mechanically enforced loud by
`layer_registry_meta_test.rs`'s `host_gated_rows_are_loud` test (every entry
below must appear, by name, in this section, **or that test fails the build**)"
(SPEC:189-192) and "all three fail the build if a row is added to
`MANUALLY_VERIFIED`..." (SPEC:226-229). Adding a 14th `LayerId` today fails
nothing. Note the meta-test is pure source-text scanning and does not need the
feature at all — the gate is gratuitous.

**Fix:** Remove `feature = "layer-fault-injection"` from
`layer_registry_meta_test.rs`'s crate-level `cfg` (it reads files, it does not
touch seams), and add a CI job / `make` target that runs
`cargo test -p nono-sandbox-cli --features layer-fault-injection` on Windows.
Without an executing gate, "a contract entry with no such test is not satisfied"
is unenforced.

## Warnings

### WR-01: AppContainer/restricted-SID probes accept *any* value, not the expected one

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:259`, `:261`,
`crates/nono-shell-broker/src/main.rs:351`,
`crates/nono-cli/src/agent_daemon/launch.rs:1317`

**Issue:** `probe_app_container_sid` returns the SID string, and all three
decision cores discard it — `Ok(Some(_)) => confirmed`. A child in a *different*
AppContainer (one the WFP `ALE_USER_ID` filters are not scoped to) attests
identically to the correct one. Same for `probe_restricted_sids`: any non-empty
restricting-SID list confirms, without checking that `config.session_sid` is
among them.

**Fix:** Compare against the expected value the caller already holds:
```rust
match probe_app_container_sid(process) {
    Ok(Some(sid)) if sid.eq_ignore_ascii_case(expected_package_sid) => Confirmed,
    Ok(_) => Unconfirmed,
    Err(_) => Unconfirmed,
}
```

---

### WR-02: Daemon mirror silently drops `WfpEgressFilters`; the local captured for it is never used

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:709-714`, `:1294-1313`

**Issue:** `network_scoping_required` is introduced with the comment "captured
into a local **so step 6.7's attestation gate can reuse this exact result**"
(`launch.rs:709-713`), but step 6.7 never reads it — `daemon_attest_and_decide`
takes only a `HANDLE`. The registry marks `WfpEgressFilters` as
`(Daemon, None)` / `Abort`; the daemon mirror does not model it at all. The
executor documented the divergence in a doc comment and deferred it. This is a
three-way fail-direction divergence between the registry, the CLI core, and the
daemon core.

**Fix:** Pass `network_scoping_required` into `daemon_attest_and_decide` and
classify the row `NotApplicable` when scoping was not required, `Confirmed` when
`wfp_filter_add` succeeded. Delete the misleading comment otherwise.

---

### WR-03: `host_gated_rows_are_loud` cannot fail — it searches the whole SPEC, not the manual section

**File:** `crates/nono-cli/tests/layer_registry_meta_test.rs:309-345`

**Issue:** The test's stated contract is "the row's name must appear in the
SPEC's **manual-verification section**". It computes `has_manual_heading` and
then never uses it to scope anything; the actual assertion is
`spec.contains(name)` over the entire document (`:334`). Every one of the 13
`LayerId` names already appears in the SPEC's registry table at lines 52-64, so
this assertion is true for any row that exists at all. The test cannot detect a
row that was added to `MANUALLY_VERIFIED` without a manual-verification entry —
the precise failure it advertises.

**Fix:** Slice the document at the manual-verification heading and search only
the tail:
```rust
let idx = spec.find("## Manual verification").expect("manual section");
let manual_section = &spec[idx..];
assert!(manual_section.contains(name), ...);
```

---

### WR-04: `every_registry_row_has_a_test` accepts a function *name* in source text, and `MANUALLY_VERIFIED` is an unbounded escape hatch

**File:** `crates/nono-cli/tests/layer_registry_meta_test.rs:271-302`, `:141-251`

**Issue:** The check is `test_src.contains("fn force_unavailable_<snake>")` —
an empty stub body satisfies it. Ten of thirteen rows (77%) are on the
`MANUALLY_VERIFIED` allow-list, whose only requirement is a non-empty reason
string plus a name appearing anywhere in the SPEC (see WR-03). The combination
means a new row can be "covered" by adding one tuple with prose.

**Fix:** At minimum assert the test function exists *and* is not on the manual
list *and* that the row's env-var bridge name appears in `command_runtime.rs`;
better, run the tests and assert their outcome rather than grepping for their
names.

---

### WR-05: `force_unavailable_dacl_package_sid_grant` is a byte-identical duplicate that never forces its own row unavailable

**File:** `crates/nono-cli/tests/layer_force_unavailable.rs:197-201` vs `:175-179`

**Issue:** Both tests set `NONO_FORCE_UNAVAILABLE_DACL_GRANT` and assert the same
`"DaclSessionSidGrant"` string. The package-SID test therefore proves nothing
about `DaclPackageSidGrant`; the registry row is counted as automated coverage
by `every_registry_row_has_a_test` purely because a function with the right name
exists. Combined with CR-05 (the guard only ever grants the *package* SID), the
two tests are exercising the same code path with the wrong label on it.

**Fix:** Either give the DACL family per-row flags so each row's own apply
function can be independently forced, or move `DaclPackageSidGrant` onto
`MANUALLY_VERIFIED` with an honest reason instead of a duplicate test.

---

### WR-06: Registry/SPEC call-site citations are stale, and the self-check only verifies file existence

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:630-634`,
`:662`, `:683`, `:691`, `crates/nono-cli/tests/layer_registry_selfcheck.rs:134-138`

**Issue:** This phase's own edits to `mod.rs` shifted the cited lines. Actual
positions after the change: `AppliedLabelsGuard::snapshot_and_apply` is at
`mod.rs:438` (cited `mod.rs:425`), `AppliedDaclGrantsGuard` at `mod.rs:449-453`
(cited `:436-440`), `AppliedAncestorTraverseGuard` at `mod.rs:462-468` (cited
`:449-455`), `AppliedAncestorReadAttributesGuard` at `mod.rs:486-495` (cited
`:473-482`). `registry_call_sites_exist` asserts only that the *file* exists and
carries a `// TODO(117-12)` acknowledging line drift is unclosed. For a document
declared "the source of truth", drifted citations at the moment of authoring is
a poor start.

**Fix:** Cite stable symbol names rather than line numbers
(`call_sites: &["mod.rs::prepare_live_windows_launch/AppliedLabelsGuard"]`) and
have the self-check assert the file contains that symbol.

---

### WR-07: Daemon abort path relies on `KILL_ON_JOB_CLOSE`, which cannot work when the abort reason is "not in a job"

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:938-948`, `:1325-1331`

**Issue:** On `Abort`, the daemon closes the thread handle and calls
`cleanup_failed_agent`, whose termination mechanism is "removes tenant → Drops
`AgentTenant` → closes `job_handle` → `KILL_ON_JOB_CLOSE` terminates the still-
suspended process". When the abort reason is `JobObjectContainment` unconfirmed,
the process is by hypothesis *not* in that job, so closing the job kills nothing.
The process handle is closed by the same Drop, leaving an orphaned suspended
process with no owner. Fail-secure (it never resumes) but leaks a process, and
the surrounding steps 6/6.5/6.6 all use an explicit `TerminateProcess` instead.

**Fix:** Call `TerminateProcess(process_handle_raw, 1)` explicitly before
`cleanup_failed_agent`, matching the adjacent gates' idiom.

---

### WR-08: `DaemonAttestationDecision::Proceed` is never constructed

**File:** `crates/nono-cli/src/agent_daemon/launch.rs:1242-1246`, `:64`

**Issue:** The variant is matched (`:873`) but never returned by
`daemon_attest_and_decide`. It is only invisible to `dead_code` because of the
module-wide `#[allow(dead_code)]` at `:64` — which CLAUDE.md explicitly
discourages ("Avoid `#[allow(dead_code)]`. If code is unused, either remove it
or write tests that use it"). Dead code in a security decision enum reads as
"this state is achievable" to a future maintainer.

**Fix:** Fix CR-09 so `Proceed` becomes reachable, or delete the variant.

---

### WR-09: Expectancy matrix under-models the DACL rows, so applied layers are classified `NotApplicable`

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:513-524`,
`crates/nono-cli/src/execution_runtime.rs:608`

**Issue:** `execution_runtime.rs:608` sets `package_sid: Some(windows_package_sid)`
**unconditionally** for every Windows launch, so `prepare_live_windows_launch`
constructs the package-SID DACL guard, the ancestor-traverse guard, and the
ancestor-read-attrs guard on *every* `DirectCli` arm. The registry marks
`DaclPackageSidGrant`/`DaclAncestorTraverse`/`DaclAncestorReadAttrs` as expected
only at `(DirectCli, BrokerLaunchNoPty)`. On the `Null`/`WriteRestricted`/
`LowIlPrimary`/`BrokerLaunch` arms these applied layers therefore classify
`NotApplicable` and are excluded from the attestation entirely — the session's
"attested" claim silently omits three real layers.

**Fix:** Widen the expectancy to all five `DirectCli` arms (matching the
unconditional `package_sid`), or make the guards conditional on the arm so code
and registry agree.

---

### WR-10: SPEC contradicts itself and the code on `MinifilterAbsence`, and still lists two closed items as deferred

**File:** `proj/SPEC-windows-fail-direction-contract.md:61`, `:151-155`, `:130`, `:132`

**Issue:** Several internal inconsistencies in the document the phase declares
the human-readable contract:

1. Line 61 gives `MinifilterAbsence` outcome `FailOpen` (matching
   `layer_registry.rs:733`), but lines 151-155 assert "every
   `EntryPath::Broker`-expected row in the table above
   (`MandatoryIntegrityLabel`, `AppContainerProfile`, `MinifilterAbsence`)
   reads `Abort`". Directly contradictory; the test that supposedly enforces the
   invariant (`broker_expected_rows_are_abort_only`) skips the row via a
   `probe == NotApplicable` carve-out, so neither statement is checked.
2. SC4-2 (line 130) reads "**Deferred to Plan 117-10**", but 117-10 landed the
   fix in this same phase (`mod.rs:300-311` says "SC4-2 fix (Phase 117 Plan 10)").
3. SC4-4 (line 132) describes the `NONO_TEST_HARNESS` runtime gate as current
   behaviour and "**Deferred to Plan 117-04**", but 117-04 landed in this phase
   and the runtime gate no longer exists.

**Fix:** Reconcile the structural-constraints paragraph with the table, and move
SC4-2/SC4-4 to a "resolved in this phase" state.

---

### WR-11: `attestation_downgrade_marker_path` joins an unvalidated `session_id` into a filesystem path

**File:** `crates/nono-cli/src/output.rs:159-176`

**Issue:** `dir.join(session_id).join("attestation-downgrade").join(key_hex)`
followed by `create_dir_all(parent)` + `write` (`output.rs:98-114`). `session_id`
is threaded from the caller as an opaque `&str`; on Windows a value containing
`..` or a drive-absolute prefix escapes the sessions root and creates
directories/files elsewhere. Today's callers pass an internally-generated id, so
this is defence-in-depth rather than an active vulnerability, but it violates
CLAUDE.md's "validate and canonicalize all paths" rule at a write boundary.

**Fix:** Reject or hash the session id before joining:
```rust
if session_id.is_empty()
   || session_id.contains(['/', '\\', ':'])
   || session_id.contains("..") { return None; }
```

---

### WR-12: Three duplicated decision cores with no shared conformance test

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:347-434`,
`crates/nono-cli/src/agent_daemon/launch.rs:1315-1342`,
`crates/nono-shell-broker/src/main.rs:339-357`

**Issue:** The crate-boundary duplication is justified, but nothing keeps the
three in step: the CLI core fail-closes on unrecognized required-layer names, the
broker ignores them (CR-03); the CLI core aborts on unconfirmed
`WfpEgressFilters`, the daemon does not model it (WR-02); the daemon's downgraded
set is a hardcoded literal that no test compares against the registry. The
`BROKER_REQUIRED_LAYERS_ENV_VAR` constant is paired with the broker's literal
`"NONO_BROKER_REQUIRED_LAYERS"` "only by this comment and by the literal string
matching" (broker `main.rs:761-766`) — a rename on either side silently
disconnects the wire contract, and the broker's fail-open default (CR-03.3)
means the failure is silent in the permissive direction.

**Fix:** Add a cross-binary conformance test that asserts (a) the broker's
literal equals `BROKER_REQUIRED_LAYERS_ENV_VAR`, (b) every name
`required_layers_for_broker()` emits is in the broker's attestable set, and
(c) the daemon's hardcoded downgraded list equals the registry's
`(Daemon, None)`-expected `ConfiguredOnly` rows. All three are pure data
comparisons and need no live spawn.

---

_Reviewed: 2026-08-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
