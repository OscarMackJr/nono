//! Phase 117 Plan 08 (CINT-02): the CLI-side attestation *decision* module —
//! `attest_and_decide()`, the single function every gate-insertion site
//! (Plans 09/10/11) calls with a suspended child's process handle and gets
//! back one [`AttestationDecision`] (proceed / proceed-downgraded / abort).
//!
//! # D-02: policy lives here, not in `crates/nono`
//!
//! `crates/nono/src/attestation.rs` (Plan 05) is a policy-free vocabulary
//! plus raw OS probes. This module owns the POLICY half: which layers are
//! required on which arm (read from Plan 01's [`layer_registry`]), what a
//! missing layer means (D-25's per-row [`layer_registry::ContractOutcome`]),
//! and how D-26's machine-policy/CLI-flag tighten-only union is applied.
//! `crates/nono` stays policy-free (ADR-86).
//!
//! # Open Question 1 (RESEARCH), resolved here
//!
//! `LayerId::WfpEgressFilters` uses
//! [`layer_registry::ProbeKind::ConfirmedByEnforcingComponentReport`]: a
//! report *from* the enforcing component (the elevated `nono-wfp-service`,
//! over its pre-spawn IPC protocol) — classified `Confirmed` when that
//! report already succeeded, `Unconfirmed` otherwise. This is a distinct
//! sub-case from an independent post-hoc kernel query
//! (`LiveTokenOrJobQuery`) — never silently promoted to the same meaning,
//! and never re-probed by this function itself (see
//! [`AttestationInput::wfp_preconfirmed`]).
//!
//! # Blocker-1 closure (registry declaration → dispatch → broker consumption)
//!
//! `LayerId::AppContainerProfile`'s `(EntryPath::Broker, None)` /
//! `(EntryPath::Daemon, None)` expectancy (Plan 01) is genuinely attested,
//! just never through THIS function — `attest_and_decide` is never called
//! with `EntryPath::Broker` (see [`AttestationInput::entry_path`]'s doc
//! comment) because `nono-shell-broker` is a separate binary that cannot
//! import this module. [`required_layers_for_broker`] (Task 2) is the one
//! code path through which a `(EntryPath::Broker, ...)` row's expectancy
//! reaches a decision: it derives the [`BROKER_REQUIRED_LAYERS_ENV_VAR`]
//! wire-contract value nono-cli sets when spawning the broker, and Plan 11's
//! own local probe-then-decide (inside `nono-shell-broker`, using the same
//! `crates/nono` primitives) reads it.
//!
//! # Deviations from the plan's literal text (documented per the executor's
//! deviation protocol; see `117-08-SUMMARY.md` for the full writeup)
//!
//! 1. **No live `read_machine_egress_policy()` call in this module.**
//!    [`AttestationInput::machine_required_layers`] takes the
//!    already-resolved `required_layers.required` slice from whichever
//!    single machine-policy read the caller's own entry path already
//!    performs (the daemon's Phase 83 D-04 "SOLE read" startup snapshot,
//!    `agent_daemon/mod.rs:353`; the direct-CLI path's own single startup
//!    read, `main.rs:189`). Calling `nono::machine_policy::read_machine_egress_policy()`
//!    fresh on every `attest_and_decide` invocation would violate the
//!    documented SOLE-read invariant (re-reading `HKLM` on every daemon
//!    tenant launch, not once at daemon startup) and would add a live
//!    Win32-registry read to the D-24 latency-sensitive per-tool-call hook
//!    path (D-23). This module still performs the D-26 tighten-only union
//!    and the fail-closed unrecognized-name rejection the
//!    `RequiredLayersPolicy` doc comment (`machine_policy.rs`) assigns to
//!    Plan 08 — it just sources the machine-policy half of that union from
//!    caller-supplied data instead of re-reading HKLM itself.
//! 2. **`EstablishedNotIndependentlyObservable` does not escalate a row's
//!    own `Abort` outcome unless the layer is explicitly D-26-tightened.**
//!    A literal reading of "a row whose status is not `Confirmed` and whose
//!    outcome is `Abort` → `Abort`" would abort **every** ordinary Windows
//!    launch that expects any `ProbeKind::ConfiguredOnly` row with
//!    `outcome: Abort` (`DaclSessionSidGrant`, `DaclPackageSidGrant`,
//!    `DaclAncestorTraverse`, `DaclAncestorReadAttrs`, `FirewallRulesEgress`,
//!    `BrokerAuthenticodeTrustGate`) — `ConfiguredOnly` rows classify to
//!    `EstablishedNotIndependentlyObservable` *unconditionally* (there is no
//!    live re-observation, by design), so under the literal reading they
//!    could never reach `Confirmed` and would always trip their own `Abort`
//!    default. That is not a per-row edge case; `DaclSessionSidGrant` alone
//!    is expected on every `WriteRestricted`-arm launch, nono-cli's default
//!    supervised path. Per `ProbeKind::ConfiguredOnly`'s own documented
//!    rationale (`layer_registry.rs`: "the apply-time `Result` already
//!    happened before this function runs — if construction had failed, the
//!    caller would have aborted before ever reaching this attestation
//!    call"), reaching this function at all with such a row means its real
//!    enforcement gate already succeeded fail-closed. This module therefore
//!    downgrades (never aborts) an `EstablishedNotIndependentlyObservable`
//!    row's own default `Abort` outcome, honoring D-18's "still downgrades
//!    the claim" — but a D-26-tightened requirement on that same layer still
//!    aborts (an explicit "must be Confirmed" requirement can never be
//!    satisfied by a layer that can only ever reach
//!    `EstablishedNotIndependentlyObservable`, so tightening it is
//!    fail-closed by design, not a no-op). See
//!    `decide_from_entries`/`ContractOutcome::Abort`'s match arm.
//!
//! # D-19 reminder
//!
//! This module never calls `OpenProcess` and never accepts a claim from the
//! confined process itself — every probe dispatched here takes the
//! caller-owned [`nono::attestation::ProcessHandle`] threaded through
//! [`AttestationInput::child_process`] (D-21's suspended-child handle).

use super::launch::WindowsTokenArm;
use super::layer_registry;
use nono::attestation::{
    probe_app_container_sid, probe_in_job, probe_restricted_sids, JobHandle,
    LayerAttestationStatus, ProcessHandle,
};
use nono::NonoError;
use std::collections::HashSet;

/// The wire contract nono-cli uses to tell `nono-shell-broker.exe` which
/// layers it must itself confirm active before resuming its own suspended
/// grandchild (Blocker-1's resolution, Task 2).
///
/// Value shape: a comma-separated list of `LayerId` `Debug`-format names
/// (e.g. `"AppContainerProfile"`), naming exactly the registry rows whose
/// `(EntryPath::Broker, ...)` expectancy is `expected: true` AND whose
/// `outcome` is `ContractOutcome::Abort` — see [`required_layers_for_broker`].
/// The broker (Plan 11) has no access to [`layer_registry`] itself; it reads
/// this env var and treats every named layer as must-be-`Confirmed`-or-
/// terminate. Layers not named in the list are the broker's own business to
/// leave unattested (D-02: the broker never invents its own policy, it only
/// enforces what nono-cli tells it).
pub(crate) const BROKER_REQUIRED_LAYERS_ENV_VAR: &str = "NONO_BROKER_REQUIRED_LAYERS";

/// D-27/T-117-21: layer-specific detail (which layer, why) belongs on the
/// operator's channel and the audit event, never on a channel the confined
/// process can read (D-28) — enforced by Plan 09's rendering, not this type.
/// This enum itself carries the full detail; callers are responsible for
/// routing it correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttestationDecision {
    /// Every expected row for this (entry_path, token_arm) either
    /// classified `Confirmed` or is a `ProbeKind::ConfiguredOnly` row the
    /// caller reported as applied (CR-09: that is the expected baseline for
    /// such a row, not a degradation). The session's confinement claim
    /// holds.
    Proceed,
    /// At least one expected row did not classify `Confirmed`, but its
    /// contracted outcome (or its status being
    /// `EstablishedNotIndependentlyObservable` on a non-tightened `Abort`
    /// row — see the module doc's Deviation 2) permits proceeding with a
    /// visibly downgraded claim (D-14/D-25).
    ProceedDowngraded {
        downgraded: Vec<layer_registry::LayerId>,
    },
    /// A required layer could not be confirmed and its contract (or a
    /// D-26-tightened requirement) demands abort. D-22: the caller
    /// `TerminateProcess`es the suspended child, unwinds guards in Drop
    /// order, and surfaces a typed `NonoError` naming `layer`.
    Abort {
        layer: layer_registry::LayerId,
        status: LayerAttestationStatus,
    },
}

/// Input to [`attest_and_decide`]. Constructed by the gate-insertion sites
/// Plans 09/10/11 add (`launch.rs` direct spawn, `agent_daemon/launch.rs`).
#[derive(Debug)]
pub(crate) struct AttestationInput<'a> {
    /// D-21: the real suspended child's process handle, opened by the
    /// caller. This module never opens its own handle (D-19).
    pub child_process: ProcessHandle,
    /// Phase 117 review CR-02: the supervisor's OWN containment Job Object
    /// handle (`ProcessContainment::job`), so `LayerId::JobObjectContainment`
    /// asks "is the child in THIS job" rather than the vacuous "is it in ANY
    /// job". A null handle is rejected fail-closed inside
    /// [`nono::attestation::probe_in_job`], never silently downgraded to the
    /// ANY-job query.
    pub containment_job: JobHandle,
    /// Which binary is spawning the process being attested.
    ///
    /// **This function is never called with `EntryPath::Broker`.** Rows
    /// expected only at that entry path (`AppContainerProfile`) are
    /// attested entirely inside `nono-shell-broker` (Plan 11), reached only
    /// through [`required_layers_for_broker`], not through this function —
    /// `nono-shell-broker` is a separate binary that cannot call into this
    /// crate's attestation module. Every real call this plan's own callers
    /// make (Plans 09/10) uses `EntryPath::DirectCli` or `EntryPath::Daemon`.
    pub entry_path: layer_registry::EntryPath,
    /// The `WindowsTokenArm` selected for this launch, or `None` for entry
    /// paths that bypass `select_windows_token_arm` entirely
    /// (`EntryPath::Daemon`; `EntryPath::Broker`, never passed here).
    pub token_arm: Option<WindowsTokenArm>,
    /// The already-computed WFP pre-spawn IPC readiness result (Open
    /// Question 1's resolution) — this function does not re-derive it, only
    /// reads it for `LayerId::WfpEgressFilters`'s
    /// `ProbeKind::ConfirmedByEnforcingComponentReport` classification.
    pub wfp_preconfirmed: bool,
    /// Phase 117 review CR-09: which layers the caller actually applied for
    /// THIS launch. `ProbeKind::ConfiguredOnly` rows used to classify
    /// `EstablishedNotIndependentlyObservable` unconditionally — a free pass
    /// that (a) made `AttestationDecision::Proceed` unreachable on every
    /// production path, so the D-27 downgrade banner fired on every session
    /// forever, and (b) reported layers as "established" on paths where no
    /// apply ever happened. Defaults are fail-secure: an unreported layer is
    /// `NotApplied`, which on an `expected: true` row classifies
    /// `Unconfirmed`.
    pub applied: layer_registry::AppliedLayers,
    /// Phase 117 review WR-01: the synthetic per-session restricting SID
    /// this launch built its `WRITE_RESTRICTED` token with
    /// (`config.session_sid`), so `LayerId::RestrictedToken` can check that
    /// THAT SID is among the child token's restricting SIDs rather than
    /// accepting any non-empty list. A token restricted by some other SID is
    /// not the token this session's confinement claim describes.
    ///
    /// `None` means "no session SID for this launch" — the row is only
    /// expected on the `WriteRestricted` arm, which by construction has one,
    /// so `None` there is a fail-closed `Unconfirmed`.
    pub expected_session_sid: Option<&'a str>,
    /// CLI-flag-required layer names (`LayerId` `Debug`-format strings).
    /// D-26: unions with `machine_required_layers`, tighten-only.
    pub required_layers_override: &'a [String],
    /// The machine policy's `required_layers.required` slice (D-26),
    /// already resolved by the caller from its own single
    /// `read_machine_egress_policy()` read — see the module doc's
    /// Deviation 1 for why this module does not read `HKLM` itself.
    pub machine_required_layers: &'a [String],
}

/// D-08 expectancy-cell match: does `input_arm` name the same
/// `WindowsTokenArm` variant `row_arm` names, comparing by `Debug`-format
/// string per `layer_registry`'s decoupling convention (no direct
/// `WindowsTokenArm` import into `layer_registry.rs`). `None == None` (both
/// bypass `select_windows_token_arm`); any other combination where one side
/// is `None` and the other `Some` is a non-match.
fn token_arm_matches(input_arm: Option<WindowsTokenArm>, row_arm: Option<&'static str>) -> bool {
    match (input_arm, row_arm) {
        (None, None) => true,
        (Some(arm), Some(name)) => format!("{arm:?}") == name,
        _ => false,
    }
}

/// D-18 classification helper: whether a probe's successful `Ok(value)`
/// represents a positive confirmation (`true` / `Some(..)` / non-empty
/// `Vec`) or an authoritative negative (`false` / `None` / empty `Vec`).
/// Pure — no OS calls — so the same rule is unit-testable independent of any
/// real `ProcessHandle`.
trait ProbePositivity {
    fn is_positive_confirmation(&self) -> bool;
}

impl ProbePositivity for bool {
    fn is_positive_confirmation(&self) -> bool {
        *self
    }
}

impl<T> ProbePositivity for Option<T> {
    fn is_positive_confirmation(&self) -> bool {
        self.is_some()
    }
}

impl<T> ProbePositivity for Vec<T> {
    fn is_positive_confirmation(&self) -> bool {
        !self.is_empty()
    }
}

/// D-18 classification of a single `LiveTokenOrJobQuery` probe result:
/// `Ok` + positive confirmation → `Confirmed`; `Ok` + authoritative negative
/// → `Unconfirmed`; `Err` (the OS call itself failed) → `Unconfirmed` — both
/// `Ok`-negative and `Err` land on the same output state (the distinction
/// matters for *why* the row is unconfirmed, not for the decision, per
/// D-18's four states not splitting further).
fn classify_probe_outcome<T: ProbePositivity>(result: nono::Result<T>) -> LayerAttestationStatus {
    match result {
        Ok(value) if value.is_positive_confirmation() => LayerAttestationStatus::Confirmed,
        Ok(_) => LayerAttestationStatus::Unconfirmed,
        Err(_) => LayerAttestationStatus::Unconfirmed,
    }
}

/// D-17: dispatch a live `crates/nono` probe keyed by `LayerId`, exhaustive
/// over every `LayerId` variant (no wildcard arm — T-117-06 mirror) so a new
/// variant forces this dispatch to be updated explicitly.
///
/// Only `RestrictedToken`, `MandatoryIntegrityLabel`, `AppContainerProfile`,
/// and `JobObjectContainment` have a live probe today (RESEARCH §C); every
/// other `LayerId` is grouped into the defensive fallback arm below, which
/// is unreachable in practice — `classify_row` only calls this function when
/// `entry.probe == ProbeKind::LiveTokenOrJobQuery`, and none of the other
/// nine IDs' registry rows declare that probe kind (they use `ConfiguredOnly`,
/// `ConfirmedByEnforcingComponentReport`, or `NotApplicable` directly,
/// without ever reaching this function). The fallback exists solely so a
/// future registry/dispatch drift fails secure (`Unconfirmed`) instead of
/// panicking or silently mis-classifying.
fn classify_live_probe(
    id: layer_registry::LayerId,
    input: &AttestationInput,
) -> LayerAttestationStatus {
    use layer_registry::LayerId;

    let process = input.child_process;
    match id {
        // WR-01: presence of *some* restricting SID is not the claim. The
        // claim is that THIS launch's synthetic per-session SID is the
        // restricting SID, which is what makes the WRITE_RESTRICTED
        // double-check mean anything for this session.
        LayerId::RestrictedToken => {
            match (probe_restricted_sids(process), input.expected_session_sid) {
                (Ok(sids), Some(expected)) => {
                    if sids.iter().any(|sid| sid.eq_ignore_ascii_case(expected)) {
                        LayerAttestationStatus::Confirmed
                    } else {
                        LayerAttestationStatus::Unconfirmed
                    }
                }
                // No expected value to compare against, or the probe failed:
                // fail closed.
                (Ok(_), None) | (Err(_), _) => LayerAttestationStatus::Unconfirmed,
            }
        }
        // CR-02: probed against the supervisor's OWN containment job, not
        // the vacuous "in ANY job" query a null handle would ask.
        LayerId::JobObjectContainment => {
            classify_probe_outcome(probe_in_job(process, input.containment_job))
        }
        LayerId::AppContainerProfile => classify_probe_outcome(probe_app_container_sid(process)),
        // Phase 117 review CR-01: `MandatoryIntegrityLabel` no longer
        // dispatches here. It used to call `probe_integrity_level` on the
        // child token — the wrong kernel object for a row that describes an
        // ACE on filesystem paths, and a call that succeeds for every
        // Windows token, so its deny branch was unreachable. The row is now
        // `ProbeKind::ConfiguredOnly` (see `layer_registry.rs`) and never
        // reaches this dispatch; it is grouped into the defensive
        // fail-secure fallback below alongside the other non-live-probed
        // IDs, so a future registry/dispatch drift lands on `Unconfirmed`.
        LayerId::MandatoryIntegrityLabel
        | LayerId::DaclSessionSidGrant
        | LayerId::DaclPackageSidGrant
        | LayerId::DaclAncestorTraverse
        | LayerId::DaclAncestorReadAttrs
        | LayerId::WfpEgressFilters
        | LayerId::FirewallRulesEgress
        | LayerId::MinifilterAbsence
        | LayerId::BrokerAuthenticodeTrustGate
        | LayerId::InterpreterCoverageGate => {
            // Defensive fallback (D-19 fail-secure): no live probe is
            // defined for these IDs today; reached only on a
            // registry/dispatch drift.
            LayerAttestationStatus::Unconfirmed
        }
    }
}

/// D-18 classification of one registry row against `input`'s
/// (entry_path, token_arm) configuration. `NotApplicable` when no matching
/// `ArmExpectancy` cell is found OR the matching cell has `expected: false`
/// — both mean "not this function's concern" (a registry row can omit a
/// cell rather than spell out every `expected: false` combination).
///
/// **Blocker-1 verification:** `AppContainerProfile`'s expectancy has no
/// `EntryPath::DirectCli` cell at all (Plan 01), so for any `DirectCli`
/// input this always falls through to the `NotApplicable` early return
/// below without ever reaching the `match entry.probe` dispatch — no probe
/// function is called.
fn classify_row(
    entry: &layer_registry::LayerRegistryEntry,
    input: &AttestationInput,
) -> LayerAttestationStatus {
    let expected = entry
        .expectancy
        .iter()
        .find(|arm| {
            arm.entry_path == input.entry_path && token_arm_matches(input.token_arm, arm.token_arm)
        })
        .map(|arm| arm.expected)
        .unwrap_or(false);

    if !expected {
        return LayerAttestationStatus::NotApplicable;
    }

    match entry.probe {
        layer_registry::ProbeKind::LiveTokenOrJobQuery => classify_live_probe(entry.id, input),
        layer_registry::ProbeKind::ConfirmedByEnforcingComponentReport => {
            // CR-04: a row whose enforcing component was not part of this
            // launch's composition at all (the WFP backend was not the one
            // selected) is NOT applicable — it is not degraded, and it must
            // not abort the launch.
            match input.applied.status(entry.id) {
                layer_registry::LayerApplication::NotApplicable => {
                    LayerAttestationStatus::NotApplicable
                }
                // Open Question 1's resolution: a report FROM the enforcing
                // component (already fail-closed pre-spawn), never re-probed
                // here — this function never calls any of the four
                // `crates/nono` probe functions for this row.
                _ => {
                    if input.wfp_preconfirmed {
                        LayerAttestationStatus::Confirmed
                    } else {
                        LayerAttestationStatus::Unconfirmed
                    }
                }
            }
        }
        layer_registry::ProbeKind::ConfiguredOnly => {
            // CR-09: "the apply-time Result already happened before this
            // function runs" is only true for a layer the caller actually
            // applied. The caller now says so per launch instead of this
            // function assuming it.
            match input.applied.status(entry.id) {
                // The apply ran and succeeded fail-closed. There is no
                // independent post-hoc query for this layer, so the honest
                // label is "established, not independently observable" —
                // never `Confirmed`.
                layer_registry::LayerApplication::Applied => {
                    LayerAttestationStatus::EstablishedNotIndependentlyObservable
                }
                // Expected on this arm, but the supervisor did not apply it.
                // This is the negative the whole contract exists to catch.
                layer_registry::LayerApplication::NotApplied => LayerAttestationStatus::Unconfirmed,
                layer_registry::LayerApplication::NotApplicable => {
                    LayerAttestationStatus::NotApplicable
                }
            }
        }
        layer_registry::ProbeKind::NotApplicable => LayerAttestationStatus::NotApplicable,
    }
}

/// D-25/D-13/D-14/D-06/D-07/D-26 outcome application, factored out of
/// [`attest_and_decide`] so the policy logic is unit-testable against
/// synthetic registry rows without a live `ProcessHandle` or the
/// Windows-populated registry.
fn decide_from_entries(
    entries: &[layer_registry::LayerRegistryEntry],
    input: &AttestationInput,
    required_names: &HashSet<String>,
) -> AttestationDecision {
    let mut downgraded: Vec<layer_registry::LayerId> = Vec::new();

    for entry in entries {
        let status = classify_row(entry, input);
        if status == LayerAttestationStatus::NotApplicable
            || status == LayerAttestationStatus::Confirmed
        {
            continue;
        }

        let layer_name = format!("{:?}", entry.id);
        let tightened = required_names.contains(&layer_name);

        match &entry.outcome {
            layer_registry::ContractOutcome::Abort => {
                if tightened || status == LayerAttestationStatus::Unconfirmed {
                    return AttestationDecision::Abort {
                        layer: entry.id,
                        status,
                    };
                }
                // status == EstablishedNotIndependentlyObservable, not
                // tightened. CR-09: this is now the EXPECTED baseline for a
                // layer the caller reported as applied — the apply ran and
                // succeeded fail-closed, and no independent re-observation
                // exists by design. It is therefore NOT a downgrade. Before
                // this fix every such row was pushed into `downgraded`,
                // which made `Proceed` unreachable and fired the
                // non-silenceable D-27 banner on every Windows session
                // forever — destroying exactly the signal it exists to
                // carry. A row the caller did NOT apply now classifies
                // `Unconfirmed` and aborts on the branch above.
            }
            layer_registry::ContractOutcome::DegradeWithVisibleClaim
            | layer_registry::ContractOutcome::FailOpenDefect { .. } => {
                if tightened {
                    return AttestationDecision::Abort {
                        layer: entry.id,
                        status,
                    };
                }
                // D-14: a deferred FailOpenDefect fix still downgrades the
                // runtime claim — never silently folded into Confirmed just
                // because the fix is deferred.
                downgraded.push(entry.id);
            }
            layer_registry::ContractOutcome::FailOpen { .. } => {
                if tightened {
                    return AttestationDecision::Abort {
                        layer: entry.id,
                        status,
                    };
                }
                // D-06: hardening-only; absence never widens the claim —
                // not added to `downgraded`.
            }
            layer_registry::ContractOutcome::SubstituteEquivalentMechanism { alternate } => {
                // D-07: the equivalence claim is confirmed per session, not
                // asserted once. Look up the alternate's own registry row
                // (within this same `entries` slice, so tests can supply
                // both rows together) and re-classify it.
                let alt_confirmed = entries
                    .iter()
                    .find(|e| e.name == *alternate)
                    .map(|alt_entry| {
                        classify_row(alt_entry, input) == LayerAttestationStatus::Confirmed
                    })
                    .unwrap_or(false);
                if alt_confirmed && !tightened {
                    continue;
                }
                // D-07: an unconfirmed substitute (or a D-26-tightened
                // requirement on the primary) falls through to abort — the
                // conservative fail-secure default when neither the primary
                // mechanism nor its named alternate is confirmed.
                return AttestationDecision::Abort {
                    layer: entry.id,
                    status,
                };
            }
        }
    }

    if downgraded.is_empty() {
        AttestationDecision::Proceed
    } else {
        AttestationDecision::ProceedDowngraded { downgraded }
    }
}

/// The single CLI-side attestation decision function (CINT-02). See the
/// module doc for D-02/Open Question 1/Blocker-1 and the two documented
/// deviations from the plan's literal text.
///
/// # Errors
///
/// Returns `Err(NonoError::LayerAttestationFailed)` when
/// `required_layers_override` or `machine_required_layers` names a string
/// that does not match any `LayerId` `Debug`-format name — fail-closed
/// rejection of an unrecognized required-layer name, per the validation
/// responsibility `RequiredLayersPolicy`'s doc comment
/// (`crates/nono/src/machine_policy.rs`) assigns to this module. Never
/// returns `Err` for any other reason — an unconfirmed or missing layer is
/// represented as `Ok(AttestationDecision::Abort { .. })`, not an `Err`, so
/// callers can uniformly match on the decision without a second error path.
pub(crate) fn attest_and_decide(input: AttestationInput) -> nono::Result<AttestationDecision> {
    let known_names: HashSet<String> = layer_registry::ALL
        .iter()
        .map(|id| format!("{id:?}"))
        .collect();

    let mut required_names: HashSet<String> = HashSet::new();
    for name in input
        .required_layers_override
        .iter()
        .chain(input.machine_required_layers.iter())
    {
        if !known_names.contains(name) {
            return Err(NonoError::LayerAttestationFailed {
                layer: name.clone(),
                reason: "unrecognized required-layer name in machine policy or CLI flag \
                         (fail-closed; must match a LayerId Debug-format name)"
                    .to_string(),
            });
        }
        required_names.insert(name.clone());
    }

    Ok(decide_from_entries(
        layer_registry::all_entries(),
        &input,
        &required_names,
    ))
}

/// Task 2 / Blocker-1 resolution: filters `entries` to the rows expected at
/// `(EntryPath::Broker, expected: true)` with `outcome: ContractOutcome::Abort`,
/// joining their `Debug`-format `LayerId` names with commas — the exact
/// [`BROKER_REQUIRED_LAYERS_ENV_VAR`] wire-contract value. This is the ONLY
/// code path through which a `(EntryPath::Broker, ...)` row's expectancy
/// reaches a decision; [`attest_and_decide`] never dispatches on
/// `EntryPath::Broker` itself (see [`AttestationInput::entry_path`]'s doc
/// comment), so this helper is what makes those rows genuinely attested
/// rather than merely unclaimed. Plan 10 calls this to build the env var
/// value it sets when spawning `nono-shell-broker.exe`; Plan 11 (inside
/// `nono-shell-broker`, a separate binary with no access to
/// [`layer_registry`]) reads the resulting env var and treats every named
/// layer as must-be-`Confirmed`-or-terminate.
///
/// A row expected at `EntryPath::Broker` with a non-`Abort` outcome is
/// deliberately excluded here (Item-1, `layer_registry.rs`'s
/// `broker_expected_rows_are_abort_only` test enforces that no such row
/// exists in the registry today) — that combination would otherwise go
/// completely unattested on the broker arm.
/// # Phase 117 review CR-03.1 — arm scoping
///
/// `spawn_arm` is the `WindowsTokenArm` nono-cli selected for THIS launch
/// (`BrokerLaunch` for the PTY shape, `BrokerLaunchNoPty` for the
/// AppContainer shape). A `(EntryPath::Broker, token_arm: None)` cell means
/// "applies to every broker spawn shape"; a `(EntryPath::Broker,
/// Some(arm))` cell applies only to that shape. Without this, the wire
/// contract demanded `AppContainerProfile` on the PTY shape, where the
/// broker creates no AppContainer at all — an unsatisfiable requirement the
/// broker used to paper over by ignoring it.
pub(crate) fn required_layers_for_broker(
    entries: &[layer_registry::LayerRegistryEntry],
    spawn_arm: WindowsTokenArm,
) -> String {
    entries
        .iter()
        .filter(|entry| {
            entry.outcome == layer_registry::ContractOutcome::Abort
                && entry.expectancy.iter().any(|arm| {
                    arm.entry_path == layer_registry::EntryPath::Broker
                        && arm.expected
                        && match arm.token_arm {
                            None => true,
                            Some(name) => format!("{spawn_arm:?}") == name,
                        }
                })
        })
        .map(|entry| format!("{:?}", entry.id))
        .collect::<Vec<_>>()
        .join(",")
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
fn dummy_process() -> ProcessHandle {
    std::ptr::null_mut()
}

/// CR-02: the synthetic-registry policy tests never exercise a live probe
/// (their rows are `ConfiguredOnly`/`ConfirmedByEnforcingComponentReport`),
/// so a null job handle is correct here — and if a future edit accidentally
/// routes one of them through `probe_in_job`, the null handle now fails
/// CLOSED (`Unconfirmed`) rather than silently confirming.
#[cfg(test)]
fn dummy_job() -> JobHandle {
    std::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;
    use layer_registry::{
        ArmExpectancy, ContractOutcome, EntryPath, LayerId, LayerRegistryEntry, ProbeKind,
    };

    // ---- classify_probe_outcome (pure, platform-neutral D-18 classification) ----

    #[test]
    fn positive_bool_confirms() {
        let result: nono::Result<bool> = Ok(true);
        assert_eq!(
            classify_probe_outcome(result),
            LayerAttestationStatus::Confirmed
        );
    }

    #[test]
    fn negative_bool_is_unconfirmed() {
        let result: nono::Result<bool> = Ok(false);
        assert_eq!(
            classify_probe_outcome(result),
            LayerAttestationStatus::Unconfirmed
        );
    }

    #[test]
    fn err_is_unconfirmed() {
        let result: nono::Result<bool> = Err(NonoError::LayerAttestationFailed {
            layer: "Test".to_string(),
            reason: "synthetic".to_string(),
        });
        assert_eq!(
            classify_probe_outcome(result),
            LayerAttestationStatus::Unconfirmed
        );
    }

    #[test]
    fn positive_option_confirms() {
        let result: nono::Result<Option<String>> = Ok(Some("S-1-15-2-1".to_string()));
        assert_eq!(
            classify_probe_outcome(result),
            LayerAttestationStatus::Confirmed
        );
    }

    #[test]
    fn negative_option_is_unconfirmed() {
        let result: nono::Result<Option<String>> = Ok(None);
        assert_eq!(
            classify_probe_outcome(result),
            LayerAttestationStatus::Unconfirmed
        );
    }

    #[test]
    fn positive_vec_confirms() {
        let result: nono::Result<Vec<String>> = Ok(vec!["S-1-5-12".to_string()]);
        assert_eq!(
            classify_probe_outcome(result),
            LayerAttestationStatus::Confirmed
        );
    }

    #[test]
    fn empty_vec_is_unconfirmed() {
        let result: nono::Result<Vec<String>> = Ok(Vec::new());
        assert_eq!(
            classify_probe_outcome(result),
            LayerAttestationStatus::Unconfirmed
        );
    }

    // ---- token_arm_matches ----

    #[test]
    fn token_arm_matches_none_none() {
        assert!(token_arm_matches(None, None));
    }

    #[test]
    fn token_arm_matches_debug_format() {
        assert!(token_arm_matches(
            Some(WindowsTokenArm::WriteRestricted),
            Some("WriteRestricted")
        ));
    }

    #[test]
    fn token_arm_mismatched_option_shape_is_false() {
        assert!(!token_arm_matches(Some(WindowsTokenArm::Null), None));
        assert!(!token_arm_matches(None, Some("Null")));
    }

    // ---- decide_from_entries policy application (synthetic registry rows) ----

    const NULL_ARM_EXPECTANCY: [ArmExpectancy; 1] = [ArmExpectancy {
        entry_path: EntryPath::DirectCli,
        token_arm: Some("Null"),
        expected: true,
    }];

    fn null_arm_input(applied: layer_registry::AppliedLayers) -> AttestationInput<'static> {
        AttestationInput {
            child_process: dummy_process(),
            containment_job: dummy_job(),
            entry_path: EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::Null),
            wfp_preconfirmed: false,
            applied,
            expected_session_sid: None,
            required_layers_override: &[],
            machine_required_layers: &[],
        }
    }

    /// Phase 117 review CR-09, half 1: a `ConfiguredOnly` row the caller
    /// REPORTED AS APPLIED is the expected baseline, so the decision is
    /// `Proceed` — not a permanent downgrade.
    ///
    /// Before this fix every `ConfiguredOnly` row was pushed into
    /// `downgraded` unconditionally, so `Proceed` was unreachable on every
    /// production path and the non-silenceable D-27 banner printed on every
    /// Windows session forever.
    #[test]
    fn applied_configured_only_row_proceeds_without_a_downgrade() {
        const ENTRIES: [LayerRegistryEntry; 1] = [LayerRegistryEntry {
            id: LayerId::MandatoryIntegrityLabel,
            name: "synthetic-configured-only-abort",
            call_sites: &[],
            expectancy: &NULL_ARM_EXPECTANCY,
            outcome: ContractOutcome::Abort,
            probe: ProbeKind::ConfiguredOnly,
        }];
        let applied = layer_registry::AppliedLayers {
            mandatory_integrity_label: true,
            ..Default::default()
        };
        let input = null_arm_input(applied);
        assert_eq!(
            decide_from_entries(&ENTRIES, &input, &HashSet::new()),
            AttestationDecision::Proceed
        );
    }

    /// Phase 117 review CR-09, half 2 — THE non-vacuity proof: the SAME row,
    /// on the SAME arm, with the caller reporting it as NOT applied, aborts.
    ///
    /// The pre-fix classifier returned
    /// `EstablishedNotIndependentlyObservable` for this row unconditionally,
    /// so this deny was structurally unreachable no matter what the
    /// supervisor had (or had not) done.
    #[test]
    fn unapplied_configured_only_row_aborts() {
        const ENTRIES: [LayerRegistryEntry; 1] = [LayerRegistryEntry {
            id: LayerId::MandatoryIntegrityLabel,
            name: "synthetic-configured-only-abort",
            call_sites: &[],
            expectancy: &NULL_ARM_EXPECTANCY,
            outcome: ContractOutcome::Abort,
            probe: ProbeKind::ConfiguredOnly,
        }];
        // mandatory_integrity_label: false (the fail-secure default).
        let input = null_arm_input(layer_registry::AppliedLayers::default());
        assert_eq!(
            decide_from_entries(&ENTRIES, &input, &HashSet::new()),
            AttestationDecision::Abort {
                layer: LayerId::MandatoryIntegrityLabel,
                status: LayerAttestationStatus::Unconfirmed,
            }
        );
    }

    /// Phase 117 review CR-04: a network row whose backend was NOT the one
    /// this launch selected is `NotApplicable` — not applicable, not
    /// degraded, and never a reason to refuse the launch.
    ///
    /// Before this fix `WfpEgressFilters` was `expected: true`
    /// unconditionally at `(DirectCli, BrokerLaunchNoPty)` and derived its
    /// status solely from `wfp_preconfirmed`, so every launch that chose the
    /// `FirewallRules` backend (or requested no network restriction) was
    /// refused with `LayerAttestationFailed { layer: "WfpEgressFilters" }`.
    #[test]
    fn network_row_for_an_unselected_backend_is_not_applicable() {
        const ENTRIES: [LayerRegistryEntry; 1] = [LayerRegistryEntry {
            id: LayerId::WfpEgressFilters,
            name: "synthetic-wfp",
            call_sites: &[],
            expectancy: &NULL_ARM_EXPECTANCY,
            outcome: ContractOutcome::Abort,
            probe: ProbeKind::ConfirmedByEnforcingComponentReport,
        }];
        // wfp_egress_filters: None — a different backend was selected.
        let not_selected = null_arm_input(layer_registry::AppliedLayers::default());
        assert_eq!(
            classify_row(&ENTRIES[0], &not_selected),
            LayerAttestationStatus::NotApplicable
        );
        assert_eq!(
            decide_from_entries(&ENTRIES, &not_selected, &HashSet::new()),
            AttestationDecision::Proceed
        );

        // ...but when WFP *was* the selected backend and its pre-spawn IPC
        // report did NOT confirm, the row still aborts.
        let mut selected = null_arm_input(layer_registry::AppliedLayers {
            wfp_egress_filters: Some(false),
            ..Default::default()
        });
        selected.wfp_preconfirmed = false;
        assert_eq!(
            decide_from_entries(&ENTRIES, &selected, &HashSet::new()),
            AttestationDecision::Abort {
                layer: LayerId::WfpEgressFilters,
                status: LayerAttestationStatus::Unconfirmed,
            }
        );
    }

    #[test]
    fn tightened_required_layer_aborts_even_when_default_outcome_is_fail_open() {
        const ENTRIES: [LayerRegistryEntry; 1] = [LayerRegistryEntry {
            id: LayerId::MandatoryIntegrityLabel,
            name: "synthetic-fail-open",
            call_sites: &[],
            expectancy: &NULL_ARM_EXPECTANCY,
            outcome: ContractOutcome::FailOpen {
                justification: "test fixture",
            },
            probe: ProbeKind::ConfiguredOnly,
        }];
        let input = null_arm_input(layer_registry::AppliedLayers {
            mandatory_integrity_label: true,
            ..Default::default()
        });
        let mut required = HashSet::new();
        required.insert(format!("{:?}", LayerId::MandatoryIntegrityLabel));
        assert_eq!(
            decide_from_entries(&ENTRIES, &input, &required),
            AttestationDecision::Abort {
                layer: LayerId::MandatoryIntegrityLabel,
                status: LayerAttestationStatus::EstablishedNotIndependentlyObservable,
            }
        );
    }

    #[test]
    fn untightened_fail_open_row_never_downgrades() {
        const ENTRIES: [LayerRegistryEntry; 1] = [LayerRegistryEntry {
            id: LayerId::MandatoryIntegrityLabel,
            name: "synthetic-fail-open",
            call_sites: &[],
            expectancy: &NULL_ARM_EXPECTANCY,
            outcome: ContractOutcome::FailOpen {
                justification: "test fixture",
            },
            probe: ProbeKind::ConfiguredOnly,
        }];
        // D-06: hardening-only, and NOT applied — its absence must still
        // never widen (or downgrade) the claim.
        let input = null_arm_input(layer_registry::AppliedLayers::default());
        assert_eq!(
            decide_from_entries(&ENTRIES, &input, &HashSet::new()),
            AttestationDecision::Proceed
        );
    }

    #[test]
    fn substitute_equivalent_mechanism_falls_through_to_abort_when_alternate_unconfirmed() {
        const ENTRIES: [LayerRegistryEntry; 2] = [
            LayerRegistryEntry {
                id: LayerId::MandatoryIntegrityLabel,
                name: "primary",
                call_sites: &[],
                expectancy: &NULL_ARM_EXPECTANCY,
                outcome: ContractOutcome::SubstituteEquivalentMechanism {
                    alternate: "alternate-mechanism",
                },
                probe: ProbeKind::ConfiguredOnly,
            },
            LayerRegistryEntry {
                id: LayerId::DaclPackageSidGrant,
                name: "alternate-mechanism",
                call_sites: &[],
                expectancy: &NULL_ARM_EXPECTANCY,
                outcome: ContractOutcome::Abort,
                probe: ProbeKind::ConfiguredOnly,
            },
        ];
        // The primary was applied (so it reaches the substitute logic with
        // `Established`), the alternate was not (so it cannot be Confirmed).
        let input = null_arm_input(layer_registry::AppliedLayers {
            mandatory_integrity_label: true,
            ..Default::default()
        });
        assert_eq!(
            decide_from_entries(&ENTRIES, &input, &HashSet::new()),
            AttestationDecision::Abort {
                layer: LayerId::MandatoryIntegrityLabel,
                status: LayerAttestationStatus::EstablishedNotIndependentlyObservable,
            }
        );
    }

    #[test]
    fn substitute_equivalent_mechanism_proceeds_when_alternate_confirmed() {
        // The alternate uses `ConfirmedByEnforcingComponentReport` (driven
        // by `wfp_preconfirmed` once its backend is the selected one) rather
        // than a live-probed LayerId, so the "alternate confirmed" branch is
        // exercised deterministically without depending on real OS probe
        // behavior against `dummy_process()`'s null handle.
        const ENTRIES: [LayerRegistryEntry; 2] = [
            LayerRegistryEntry {
                id: LayerId::MandatoryIntegrityLabel,
                name: "primary",
                call_sites: &[],
                expectancy: &NULL_ARM_EXPECTANCY,
                outcome: ContractOutcome::SubstituteEquivalentMechanism {
                    alternate: "alternate-mechanism",
                },
                probe: ProbeKind::ConfiguredOnly,
            },
            LayerRegistryEntry {
                id: LayerId::WfpEgressFilters,
                name: "alternate-mechanism",
                call_sites: &[],
                expectancy: &NULL_ARM_EXPECTANCY,
                outcome: ContractOutcome::Abort,
                probe: ProbeKind::ConfirmedByEnforcingComponentReport,
            },
        ];
        let mut input = null_arm_input(layer_registry::AppliedLayers {
            mandatory_integrity_label: true,
            wfp_egress_filters: Some(true),
            ..Default::default()
        });
        input.wfp_preconfirmed = true;
        assert_eq!(
            decide_from_entries(&ENTRIES, &input, &HashSet::new()),
            AttestationDecision::Proceed
        );
    }

    // ---- attest_and_decide: fail-closed unrecognized-name validation ----

    #[test]
    fn attest_and_decide_rejects_unrecognized_required_layer_name_from_cli_flag() {
        let overrides = vec!["NotARealLayer".to_string()];
        let input = AttestationInput {
            child_process: dummy_process(),
            containment_job: dummy_job(),
            entry_path: layer_registry::EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::Null),
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers::default(),
            expected_session_sid: None,
            required_layers_override: &overrides,
            machine_required_layers: &[],
        };
        let result = attest_and_decide(input);
        match result {
            Err(NonoError::LayerAttestationFailed { layer, .. }) => {
                assert_eq!(layer, "NotARealLayer");
            }
            other => panic!("expected Err(LayerAttestationFailed), got {other:?}"),
        }
    }

    #[test]
    fn attest_and_decide_rejects_unrecognized_required_layer_name_from_machine_policy() {
        let machine_required = vec!["AlsoNotReal".to_string()];
        let input = AttestationInput {
            child_process: dummy_process(),
            containment_job: dummy_job(),
            entry_path: layer_registry::EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::Null),
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers::default(),
            expected_session_sid: None,
            required_layers_override: &[],
            machine_required_layers: &machine_required,
        };
        let result = attest_and_decide(input);
        assert!(matches!(
            result,
            Err(NonoError::LayerAttestationFailed { .. })
        ));
    }

    #[test]
    fn attest_and_decide_accepts_known_layer_names() {
        let overrides = vec![format!("{:?}", LayerId::RestrictedToken)];
        let input = AttestationInput {
            child_process: dummy_process(),
            containment_job: dummy_job(),
            entry_path: layer_registry::EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::Null),
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers::default(),
            expected_session_sid: None,
            required_layers_override: &overrides,
            machine_required_layers: &[],
        };
        // A known name must not be rejected as unrecognized — the launch
        // may still resolve to Abort (RestrictedToken is tightened and, on
        // a null process handle, unconfirmed), but that is a decision, not
        // a validation Err.
        let result = attest_and_decide(input);
        assert!(result.is_ok());
    }
}

/// Behavior tests exercised against the real, Windows-populated registry
/// (`layer_registry::all_entries()`). Kept in a separate module (mirroring
/// `layer_registry.rs`'s own `#[cfg(all(test, target_os = "windows"))] mod
/// tests` convention) even though this whole file only compiles on Windows
/// today (D-11: the module tree is included only via
/// `#[cfg(target_os = "windows")] #[path = "exec_strategy_windows/mod.rs"]`
/// in `main.rs`), so these tests keep working unchanged if that inclusion
/// ever changes.
#[cfg(all(test, target_os = "windows"))]
mod registry_tests {
    use super::*;
    use layer_registry::{EntryPath, LayerId};

    /// Behavior 1 (117-08 Task 1): a row not expected for the current
    /// (entry_path, token_arm) classifies `NotApplicable` without probing.
    /// `RestrictedToken` is expected ONLY at `(DirectCli, WriteRestricted)`.
    #[test]
    fn not_expected_for_arm_classifies_not_applicable() {
        let entries = layer_registry::all_entries();
        let restricted_token = entries
            .iter()
            .find(|e| e.id == LayerId::RestrictedToken)
            .expect("RestrictedToken row must exist");
        let input = AttestationInput {
            child_process: dummy_process(),
            containment_job: dummy_job(),
            entry_path: EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::Null),
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers::default(),
            expected_session_sid: None,
            required_layers_override: &[],
            machine_required_layers: &[],
        };
        assert_eq!(
            classify_row(restricted_token, &input),
            LayerAttestationStatus::NotApplicable
        );
    }

    /// Behavior 2 (Blocker-1 verification): `AppContainerProfile` is always
    /// `NotApplicable` at `EntryPath::DirectCli`, for every `WindowsTokenArm`.
    #[test]
    fn app_container_profile_is_never_expected_at_direct_cli() {
        let entries = layer_registry::all_entries();
        let app_container = entries
            .iter()
            .find(|e| e.id == LayerId::AppContainerProfile)
            .expect("AppContainerProfile row must exist");
        for arm in [
            WindowsTokenArm::Null,
            WindowsTokenArm::WriteRestricted,
            WindowsTokenArm::LowIlPrimary,
            WindowsTokenArm::BrokerLaunch,
            WindowsTokenArm::BrokerLaunchNoPty,
        ] {
            let input = AttestationInput {
                child_process: dummy_process(),
                containment_job: dummy_job(),
                entry_path: EntryPath::DirectCli,
                token_arm: Some(arm),
                wfp_preconfirmed: false,
                applied: layer_registry::AppliedLayers::default(),
                expected_session_sid: None,
                required_layers_override: &[],
                machine_required_layers: &[],
            };
            assert_eq!(
                classify_row(app_container, &input),
                LayerAttestationStatus::NotApplicable,
                "AppContainerProfile must be NotApplicable at (DirectCli, {arm:?})"
            );
        }
    }

    /// Phase 117 review WR-01 non-vacuity: `RestrictedToken` must confirm
    /// only when THIS launch's synthetic per-session SID is among the
    /// child token's restricting SIDs.
    ///
    /// Before WR-01 the classifier accepted ANY non-empty restricting-SID
    /// list, so a token restricted by an unrelated SID — not the one the
    /// session's WRITE_RESTRICTED double-check is built on — attested
    /// identically to the correct one.
    #[test]
    fn restricted_token_requires_this_launch_own_session_sid() {
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        let entries = layer_registry::all_entries();
        let row = entries
            .iter()
            .find(|e| e.id == LayerId::RestrictedToken)
            .expect("RestrictedToken row must exist");
        // SAFETY: GetCurrentProcess returns an always-valid pseudo-handle.
        let unrestricted: ProcessHandle = unsafe { GetCurrentProcess() };
        let mut input = AttestationInput {
            child_process: unrestricted,
            containment_job: dummy_job(),
            entry_path: EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::WriteRestricted),
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers::default(),
            expected_session_sid: Some("S-1-5-117-1-2-3-4"),
            required_layers_override: &[],
            machine_required_layers: &[],
        };
        assert_eq!(
            classify_row(row, &input),
            LayerAttestationStatus::Unconfirmed,
            "the test runner's own unrestricted token does not carry this launch's session              SID and must not confirm RestrictedToken"
        );
        // No expected value at all is also fail-closed.
        input.expected_session_sid = None;
        assert_eq!(
            classify_row(row, &input),
            LayerAttestationStatus::Unconfirmed
        );
    }

    /// Phase 117 review CR-01 non-vacuity guard: `MandatoryIntegrityLabel`
    /// must not be able to reach `Confirmed` from a live token query against
    /// an ordinary, unconfined Medium-IL process.
    ///
    /// Before CR-01 this row was `ProbeKind::LiveTokenOrJobQuery` dispatching
    /// to `probe_integrity_level`, which succeeds for EVERY Windows token
    /// (every process has a mandatory label, Medium by default) and whose
    /// classifier discarded the RID — so the test runner's own fully
    /// unconfined process classified `Confirmed`. Asserting the row is NOT
    /// `Confirmed` here fails immediately if that dispatch is ever restored.
    #[test]
    fn mandatory_integrity_label_cannot_confirm_from_an_unconfined_medium_il_token() {
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        let entries = layer_registry::all_entries();
        let row = entries
            .iter()
            .find(|e| e.id == LayerId::MandatoryIntegrityLabel)
            .expect("MandatoryIntegrityLabel row must exist");
        // SAFETY: GetCurrentProcess returns an always-valid pseudo-handle for
        // this fully unconfined, Medium-IL test-runner process.
        let unconfined: ProcessHandle = unsafe { GetCurrentProcess() };
        let input = AttestationInput {
            child_process: unconfined,
            containment_job: dummy_job(),
            entry_path: EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::Null),
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers::default(),
            expected_session_sid: None,
            required_layers_override: &[],
            machine_required_layers: &[],
        };
        assert_ne!(
            classify_row(row, &input),
            LayerAttestationStatus::Confirmed,
            "an unconfined Medium-IL process must never confirm MandatoryIntegrityLabel —              the pre-CR-01 token-integrity-level probe did exactly that"
        );
    }

    /// Behavior 4: `WfpEgressFilters` (`ConfirmedByEnforcingComponentReport`)
    /// classifies from `wfp_preconfirmed` alone, never re-probing.
    #[test]
    fn wfp_egress_filters_classifies_from_preconfirmed_flag() {
        let entries = layer_registry::all_entries();
        let wfp = entries
            .iter()
            .find(|e| e.id == LayerId::WfpEgressFilters)
            .expect("WfpEgressFilters row must exist");
        // WfpEgressFilters is expected at (DirectCli, BrokerLaunchNoPty) and
        // (Daemon, None) per PACKAGE_SID_SCOPED_EXPECTANCY.
        let confirmed_input = AttestationInput {
            child_process: dummy_process(),
            containment_job: dummy_job(),
            entry_path: EntryPath::Daemon,
            token_arm: None,
            wfp_preconfirmed: true,
            // CR-04: the WFP backend must be the one this launch selected,
            // else the row is NotApplicable rather than Confirmed.
            applied: layer_registry::AppliedLayers {
                wfp_egress_filters: Some(true),
                ..Default::default()
            },
            expected_session_sid: None,
            required_layers_override: &[],
            machine_required_layers: &[],
        };
        assert_eq!(
            classify_row(wfp, &confirmed_input),
            LayerAttestationStatus::Confirmed
        );

        let unconfirmed_input = AttestationInput {
            child_process: dummy_process(),
            containment_job: dummy_job(),
            entry_path: EntryPath::Daemon,
            token_arm: None,
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers {
                wfp_egress_filters: Some(false),
                ..Default::default()
            },
            expected_session_sid: None,
            required_layers_override: &[],
            machine_required_layers: &[],
        };
        assert_eq!(
            classify_row(wfp, &unconfirmed_input),
            LayerAttestationStatus::Unconfirmed
        );
    }

    /// Behavior 5: `MinifilterAbsence` (`ProbeKind::NotApplicable`) is
    /// always `NotApplicable`, regardless of arm.
    #[test]
    fn minifilter_absence_always_not_applicable() {
        let entries = layer_registry::all_entries();
        let minifilter = entries
            .iter()
            .find(|e| e.id == LayerId::MinifilterAbsence)
            .expect("MinifilterAbsence row must exist");
        for (entry_path, token_arm) in [
            (EntryPath::DirectCli, Some(WindowsTokenArm::Null)),
            (EntryPath::Broker, None),
            (EntryPath::Daemon, None),
        ] {
            let input = AttestationInput {
                child_process: dummy_process(),
                containment_job: dummy_job(),
                entry_path,
                token_arm,
                wfp_preconfirmed: false,
                applied: layer_registry::AppliedLayers::default(),
                expected_session_sid: None,
                required_layers_override: &[],
                machine_required_layers: &[],
            };
            assert_eq!(
                classify_row(minifilter, &input),
                LayerAttestationStatus::NotApplicable
            );
        }
    }
}

/// Task 2 broker wire-contract tests, against the real registry.
#[cfg(all(test, target_os = "windows"))]
mod broker_wire_contract_tests {
    use super::*;

    /// Discovery-based (D-32): recomputes the expected set from the registry
    /// rather than naming rows, for BOTH broker spawn shapes.
    #[test]
    fn required_layers_for_broker_includes_only_broker_expected_abort_rows_for_the_arm() {
        let entries = layer_registry::all_entries();
        for spawn_arm in [
            WindowsTokenArm::BrokerLaunch,
            WindowsTokenArm::BrokerLaunchNoPty,
        ] {
            let value = required_layers_for_broker(entries, spawn_arm);
            let got: HashSet<String> = value
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();

            let expected: HashSet<String> = entries
                .iter()
                .filter(|entry| {
                    entry.outcome == layer_registry::ContractOutcome::Abort
                        && entry.expectancy.iter().any(|arm| {
                            arm.entry_path == layer_registry::EntryPath::Broker
                                && arm.expected
                                && match arm.token_arm {
                                    None => true,
                                    Some(name) => format!("{spawn_arm:?}") == name,
                                }
                        })
                })
                .map(|entry| format!("{:?}", entry.id))
                .collect();

            assert_eq!(got, expected, "mismatch for {spawn_arm:?}");
            assert!(
                !expected.is_empty(),
                "expected at least one Broker-expected Abort row for {spawn_arm:?}"
            );
        }
    }

    /// Phase 117 review CR-03.1: the PTY broker shape creates no
    /// AppContainer, so the wire contract must NOT demand one there. The
    /// pre-fix code sent `AppContainerProfile` on both shapes, an
    /// unsatisfiable requirement on the PTY arm.
    #[test]
    fn pty_broker_arm_is_not_asked_to_attest_app_container_profile() {
        let entries = layer_registry::all_entries();
        let pty = required_layers_for_broker(entries, WindowsTokenArm::BrokerLaunch);
        assert!(
            !pty.split(',').any(|s| s == "AppContainerProfile"),
            "the PTY broker arm has no AppContainer; requiring it is unsatisfiable: {pty:?}"
        );
        assert!(
            pty.split(',').any(|s| s == "MandatoryIntegrityLabel"),
            "the PTY broker arm still applies a Low-IL mandatory label and must attest it: \
             {pty:?}"
        );
        let no_pty = required_layers_for_broker(entries, WindowsTokenArm::BrokerLaunchNoPty);
        assert!(
            no_pty.split(',').any(|s| s == "AppContainerProfile"),
            "the no-PTY broker arm DOES create an AppContainer and must attest it: {no_pty:?}"
        );
    }

    /// WR-12: every name the wire contract can emit must be one the broker
    /// binary is actually able to attest. `nono-shell-broker`'s
    /// `BROKER_ATTESTABLE_LAYERS` is duplicated here by necessity (separate
    /// binary, no shared lib target) — this test is what keeps the two in
    /// step, and it fails the build if a new Broker-expected Abort row is
    /// added without teaching the broker to probe it.
    #[test]
    fn every_emitted_broker_required_layer_is_attestable_by_the_broker() {
        // Mirror of `nono-shell-broker/src/main.rs`'s BROKER_ATTESTABLE_LAYERS.
        const BROKER_ATTESTABLE_LAYERS: &[&str] =
            &["AppContainerProfile", "MandatoryIntegrityLabel"];
        let entries = layer_registry::all_entries();
        for spawn_arm in [
            WindowsTokenArm::BrokerLaunch,
            WindowsTokenArm::BrokerLaunchNoPty,
        ] {
            for name in required_layers_for_broker(entries, spawn_arm)
                .split(',')
                .filter(|s| !s.is_empty())
            {
                assert!(
                    BROKER_ATTESTABLE_LAYERS.contains(&name),
                    "{name} is emitted on the {spawn_arm:?} wire contract but \
                     nono-shell-broker cannot attest it — the broker fails closed on \
                     unrecognized names, so this would refuse every launch on that arm"
                );
            }
        }
    }

    #[test]
    fn required_layers_for_broker_never_includes_non_broker_only_rows() {
        let entries = layer_registry::all_entries();
        let value = required_layers_for_broker(entries, WindowsTokenArm::BrokerLaunchNoPty);
        for entry in entries {
            let is_broker_expected = entry.expectancy.iter().any(|arm| {
                matches!(
                    arm,
                    layer_registry::ArmExpectancy {
                        entry_path: layer_registry::EntryPath::Broker,
                        expected: true,
                        ..
                    }
                )
            });
            if !is_broker_expected {
                let name = format!("{:?}", entry.id);
                assert!(
                    !value.split(',').any(|s| s == name),
                    "{name} is not Broker-expected but appeared in required_layers_for_broker output"
                );
            }
        }
    }
}

/// Phase 117-12 (D-24): measures `attest_and_decide()`'s real wall-clock
/// cost for a representative launch, replacing the SPEC's `TBD` latency
/// placeholder for the direct `nono run` (`EntryPath::DirectCli`) gate
/// point. Uses a REAL process handle (`GetCurrentProcess()`, a pseudo-handle
/// always valid within this process) rather than `dummy_process()`'s null
/// handle — `dummy_process()` short-circuits every live probe on an
/// invalid-handle `Err` without ever reaching the real
/// `GetTokenInformation`/`IsProcessInJob` Win32 calls, which would measure
/// error-path cost, not the real probe cost D-24 asks for.
#[cfg(all(test, target_os = "windows"))]
mod latency_measurement {
    use super::*;
    use layer_registry::EntryPath;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    /// D-24: "state the measured number rather than asserting 'negligible'."
    /// This test does not assert a specific micro-timing bound (real Win32
    /// call latency varies host-to-host and is not itself a security
    /// property) — it exists to PRODUCE a real number for the SPEC's
    /// Latency budget table via its own eprintln output, captured by this
    /// plan's execution and hand-transcribed into
    /// `proj/SPEC-windows-fail-direction-contract.md`. It still asserts a
    /// generous upper bound (`< 250ms`) as a basic sanity/regression guard —
    /// the project's stated constraint is "zero startup latency," so a
    /// measurement in the hundreds-of-milliseconds range would itself be a
    /// finding worth a follow-up, not a passing baseline to silently accept.
    #[test]
    fn attest_and_decide_direct_cli_write_restricted_write_restricted_latency() {
        // SAFETY: GetCurrentProcess takes no arguments and always returns a
        // valid pseudo-handle for the calling process; safe to call
        // unconditionally.
        let real_process: ProcessHandle = unsafe { GetCurrentProcess() };
        // CR-02: a REAL job handle, so the measurement covers the real
        // `IsProcessInJob(process, job)` call rather than the null-job
        // fail-closed early return.
        let real_job: JobHandle = unsafe {
            // SAFETY: null security attributes + null name creates an
            // unnamed job object owned by this process.
            windows_sys::Win32::System::JobObjects::CreateJobObjectW(
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        assert!(!real_job.is_null(), "CreateJobObjectW failed in test setup");
        let input = AttestationInput {
            child_process: real_process,
            containment_job: real_job,
            entry_path: EntryPath::DirectCli,
            token_arm: Some(WindowsTokenArm::WriteRestricted),
            wfp_preconfirmed: false,
            applied: layer_registry::AppliedLayers::default(),
            expected_session_sid: None,
            required_layers_override: &[],
            machine_required_layers: &[],
        };
        let start = std::time::Instant::now();
        let _ = attest_and_decide(input);
        let elapsed = start.elapsed();
        unsafe {
            // SAFETY: `real_job` is the live handle created above and is not
            // referenced after this point.
            windows_sys::Win32::Foundation::CloseHandle(real_job);
        }
        eprintln!(
            "D-24 measured attest_and_decide cost (EntryPath::DirectCli, \
             WindowsTokenArm::WriteRestricted, real GetCurrentProcess() handle): {elapsed:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(250),
            "attest_and_decide took {elapsed:?}, exceeding the generous 250ms sanity bound — \
             the project's D-24 constraint is zero startup latency, so this is worth \
             investigating rather than silently accepting"
        );
    }
}
