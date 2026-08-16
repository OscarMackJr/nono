//! Per-session enforcement receipt vocabulary (RCPT-01/RCPT-02/RCPT-03).
//!
//! This module is a policy-free record shape: it defines what a receipt
//! *looks like*, never what a receipt *requires*, when it must be emitted,
//! or how a violation should be rendered — that policy lives entirely in
//! `crates/nono-cli` (D-12, ADR-86). Everything here is inert data.
//!
//! # D-19: the supervisor attests, the confined process never does
//!
//! No field on this type may ever be populated from a claim made by the
//! process being confined — the supervisor attests, the confined process
//! never does. Every [`EnforcementReceipt`] is built by the binary that
//! itself performed the `CREATE_SUSPENDED` gate (`nono.exe`,
//! `nono-agentd.exe`, or `nono-shell-broker.exe`), from data the supervisor
//! itself observed. This is 117's D-19 invariant, unchanged, carried into
//! the record this phase produces from that same attestation pass.
//!
//! # D-20: this type describes the launch, not the session's course
//!
//! No mid-session field exists on [`EnforcementReceipt`] or will be added.
//! The claim is startup-only (117's D-20, carried forward): the receipt is
//! written once, at the D-21 attestation gate, before `ResumeThread`
//! (D-03), and it never claims anything about what happens to the child
//! after that point. Continuous or periodic re-attestation is explicitly
//! out of scope for this phase (see `118-CONTEXT.md`'s `<deferred>`
//! section).
//!
//! # D-18: Windows-only
//!
//! This type compiles on every target (no `#[cfg]` gate on the declarations
//! in this file, mirroring [`crate::attestation::LayerAttestationStatus`]'s
//! own platform-neutral-declaration precedent, 117-D-11) but is only ever
//! *populated* on Windows — on non-Windows hosts nothing in this crate or
//! its consumers constructs an [`EnforcementReceipt`]. Linux Landlock and
//! macOS Seatbelt have no layer registry to attest; their non-coverage is a
//! named boundary already handed to Phase 119 (117 D-12), and receipts
//! inherit that same boundary rather than inventing a second one.
//!
//! # ADR-86 boundary argument (D-12)
//!
//! `proj/ADR-86-library-boundary-convergence.md` records the fork's standing
//! precedent for moving code core-ward: **Cluster A** ("audit logic
//! relocated core-ward" — `AuditRecorder`, ledger append/verify,
//! merkle/inclusion-proof, and attestation sign/verify moved into
//! `crates/nono/src/audit.rs`) versus **Cluster B** ("diagnostic UX stayed
//! CLI-side" — `DiagnosticFormatter` and all rendering logic stayed in
//! `crates/nono-cli`, with only structured, policy-free facts/codes moving
//! into core). D-12 applies that same split here, explicitly:
//!
//! - [`LayerId`] and [`crate::attestation::LayerAttestationStatus`] are
//!   **Cluster-A-shaped**: identity/status vocabulary, policy-free, and
//!   core-eligible for the same reason the audit ledger's append/verify
//!   machinery was — every producer binary needs the identical type to
//!   build a receipt against, and only an exhaustive enum makes "every
//!   layer is covered" a compile-time property (D-01's census-completeness
//!   requirement).
//! - `ArmExpectancy`, `EntryPath`, `ProbeKind`, `REGISTRY_ENTRIES`, and every
//!   enforcing call site are **Cluster-B-shaped**: policy (per-arm
//!   expectancy, contracted outcome, the decision of what a missing layer
//!   *should do*) and stay exactly where they are today, in
//!   `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`. This
//!   module never imports from that file and never will — the dependency
//!   arrow points one way, from the policy layer down to this vocabulary,
//!   never back.
//!
//! This module doc IS the D-12 deliverable: a written argument, not an
//! assumption.

use crate::attestation::LayerAttestationStatus;
use serde::{Deserialize, Serialize};

/// Identity of every layer the Windows backend composes into the deny-by-
/// composition confinement model, plus the deliberate structural-absence
/// row (`MinifilterAbsence`, ADR-65/D-33).
///
/// Promoted verbatim from `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`
/// (D-12) — same 13 variant names, same declaration order as the CLI-side
/// registry's own `LayerId` and `ALL` const. The CLI's `layer_registry.rs`
/// remains the single source of truth for *policy* (which arms expect which
/// layers); this enum is only the *identity* vocabulary, moved here so it
/// is reachable from every producer binary (`nono.exe`, `nono-agentd.exe`,
/// `nono-shell-broker.exe`) without requiring `nono-cli` as a dependency —
/// see the ADR-86 boundary argument above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerId {
    /// WRITE_RESTRICTED token + per-session synthetic restricting SID.
    RestrictedToken,
    /// NO_WRITE_UP (± NO_READ_UP/NO_EXECUTE_UP) mandatory-label ACE on
    /// every compiled filesystem-policy path.
    MandatoryIntegrityLabel,
    /// Per-run AppContainer profile + derived package SID applied to the
    /// confined child's token at spawn time.
    AppContainerProfile,
    /// DACL grant of the synthetic per-session SID on writable filesystem
    /// grants — the `WriteRestricted`-arm double-check companion to
    /// `RestrictedToken`.
    DaclSessionSidGrant,
    /// DACL grant of the per-run AppContainer package SID on writable
    /// filesystem grants.
    DaclPackageSidGrant,
    /// DACL grant of `FILE_TRAVERSE` to the package SID on user-owned
    /// ancestors of the confined cwd.
    DaclAncestorTraverse,
    /// DACL grant of `FILE_READ_ATTRIBUTES` to the package SID on
    /// user-owned ancestors of the resolved binary / workspace chains.
    DaclAncestorReadAttrs,
    /// Session/package-SID-scoped WFP `ALE_USER_ID` allow-filter egress
    /// enforcement, installed via the elevated `nono-wfp-service`.
    WfpEgressFilters,
    /// Program-path-scoped `netsh advfirewall` block-rule egress
    /// enforcement — the legacy/fallback network backend, distinct from
    /// WFP.
    FirewallRulesEgress,
    /// The deliberate structural-absence row: no production minifilter
    /// exists in this tree (ADR-65/D-21). Per-file read policy within one
    /// directory is explicitly not claimed.
    MinifilterAbsence,
    /// Job Object assignment (kill-group + `--timeout` enforcement) on the
    /// suspended child, with a deny-DACL on the job object itself.
    JobObjectContainment,
    /// Authenticode signature comparison between `nono.exe` and the
    /// sibling `nono-shell-broker.exe` before spawning the broker.
    BrokerAuthenticodeTrustGate,
    /// Pre-spawn static coverage check: does the compiled filesystem
    /// policy already cover the program, cwd, and every resolved
    /// interpreter path the wrapper will spawn.
    InterpreterCoverageGate,
}

impl LayerId {
    /// Every [`LayerId`] variant, in declaration order — mirrors
    /// `layer_registry.rs::ALL`'s membership and order byte-for-byte
    /// (D-12's promotion requirement). Platform-neutral (no `#[cfg]`) so
    /// drift tests and every receipt producer can iterate it on any host.
    pub const ALL: &'static [LayerId] = &[
        LayerId::RestrictedToken,
        LayerId::MandatoryIntegrityLabel,
        LayerId::AppContainerProfile,
        LayerId::DaclSessionSidGrant,
        LayerId::DaclPackageSidGrant,
        LayerId::DaclAncestorTraverse,
        LayerId::DaclAncestorReadAttrs,
        LayerId::WfpEgressFilters,
        LayerId::FirewallRulesEgress,
        LayerId::MinifilterAbsence,
        LayerId::JobObjectContainment,
        LayerId::BrokerAuthenticodeTrustGate,
        LayerId::InterpreterCoverageGate,
    ];
}

/// One row of a receipt's per-layer census: which [`LayerId`] this row
/// names, and the [`LayerAttestationStatus`] the supervisor observed for it
/// at the D-03 write point.
///
/// D-01: a receipt's `layers` field carries one [`LayerReceiptRow`] per
/// [`LayerId::ALL`] entry, on every confined session — a full census, never
/// a filtered subset and never downgrade-only. `status` carries
/// [`LayerAttestationStatus`] verbatim (D-13: all four states are a FLOOR,
/// not a cap) rather than collapsing it to a coarser vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerReceiptRow {
    /// Which layer this row reports on.
    pub id: LayerId,
    /// The observed state of that layer at receipt-write time.
    pub status: LayerAttestationStatus,
}

/// The terminal, two-value outcome of a confined session's launch (D-02).
///
/// Deliberately two unit variants, not a `Refused { layer: LayerId }`
/// variant carrying detail — which layer refused (if any) is discoverable
/// from the receipt's `layers` census rows, and duplicating that detail
/// into this enum would create a second, potentially-inconsistent source
/// of truth for the same fact (T-118-04, accepted-and-documented in this
/// plan's threat register).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionOutcome {
    /// The session's confined child was resumed (`ResumeThread`) — the
    /// attestation gate did not abort the launch. A `Ran` outcome does not
    /// imply every layer was `Confirmed`; the per-row census may still show
    /// `Unconfirmed` or `EstablishedNotIndependentlyObservable` rows (D-16).
    Ran,
    /// The attestation gate aborted the launch before `ResumeThread` — the
    /// suspended child was terminated instead. This is still the
    /// highest-value record there is (D-02): it names the state of all
    /// thirteen layers, including the one that caused the refusal, via the
    /// accompanying `layers` census.
    Refused,
}

/// A single, policy-free enforcement receipt — the full census of every
/// registry row's observed state at the moment one confined session's
/// launch was decided, plus the terminal outcome of that decision.
///
/// Every future producer (`nono.exe`, `nono-agentd.exe`,
/// `nono-shell-broker.exe`) builds against this one type (D-12) — there is
/// no second, binary-specific receipt shape. Fields are deliberately
/// content-free (D-05/D-14): no path, no argument, no payload content, and
/// no field type capable of smuggling one in by accident. The one named
/// exception is `session_id`, an opaque per-session identifier that is
/// never path-derived, not even salted (D-05) — see
/// `crates/nono-cli/tests/receipt_content_free_scan.rs` for the
/// mechanically-enforced allowlist this shape must satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcementReceipt {
    /// On-disk schema version, for forward-compatible parsing by
    /// `nono receipt verify` and any future consumer.
    pub schema_version: u16,
    /// Opaque per-session identifier (D-05) — generated per session, never
    /// derived from a path, argument, or any other content-shaped value.
    /// The one field permitted to be an owned `String` rather than
    /// `&'static str`, since it is not statically known at compile time.
    pub session_id: String,
    /// The confined child's process id (D-05) — a bare integer, not a
    /// content-shaped value.
    pub pid: u32,
    /// `Debug`-format name of the CLI-side `EntryPath` variant
    /// (`DirectCli` / `Broker` / `Daemon`) that produced this receipt.
    /// Decoupled per the `ArmExpectancy` precedent
    /// (`layer_registry.rs::ArmExpectancy.token_arm`): this module never
    /// imports the CLI's `EntryPath` type directly, so it stays policy-free
    /// per D-12. The caller (the supervisor binary) produces this string
    /// via its own `Debug` formatting — never from process-controlled
    /// input (T-118-03, accepted-and-documented).
    pub entry_path: &'static str,
    /// `Debug`-format name of the CLI-side `WindowsTokenArm` variant, if
    /// one applies to this entry path (`None` for `EntryPath::Broker` and
    /// `EntryPath::Daemon`, which bypass `select_windows_token_arm`
    /// entirely). Same decoupling rationale as `entry_path`.
    pub token_arm: Option<&'static str>,
    /// The terminal ran/refused outcome of this session's launch (D-02).
    pub outcome: SessionOutcome,
    /// The full per-layer census — one [`LayerReceiptRow`] per
    /// [`LayerId::ALL`] entry, on every confined session (D-01). Never a
    /// filtered subset.
    pub layers: Vec<LayerReceiptRow>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_id_all_has_thirteen_entries_in_declaration_order() {
        assert_eq!(LayerId::ALL.len(), 13);
        assert_eq!(LayerId::ALL[0], LayerId::RestrictedToken);
        assert_eq!(LayerId::ALL[12], LayerId::InterpreterCoverageGate);
    }

    #[test]
    fn enforcement_receipt_round_trips_through_json() {
        let receipt = EnforcementReceipt {
            schema_version: 1,
            session_id: "20260816-000000-1".to_string(),
            pid: 4242,
            entry_path: "DirectCli",
            token_arm: Some("WriteRestricted"),
            outcome: SessionOutcome::Ran,
            layers: LayerId::ALL
                .iter()
                .map(|id| LayerReceiptRow {
                    id: *id,
                    status: LayerAttestationStatus::Confirmed,
                })
                .collect(),
        };
        let json = serde_json::to_string(&receipt).expect("receipt must serialize");
        let round_tripped: EnforcementReceipt =
            serde_json::from_str(&json).expect("receipt must deserialize");
        assert_eq!(receipt, round_tripped);
        assert_eq!(round_tripped.layers.len(), 13);
    }

    #[test]
    fn session_outcome_refused_round_trips() {
        let json = serde_json::to_string(&SessionOutcome::Refused).expect("must serialize");
        let round_tripped: SessionOutcome = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(round_tripped, SessionOutcome::Refused);
    }
}
