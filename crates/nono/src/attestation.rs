//! Startup self-attestation primitives (CINT-02).
//!
//! This module is a policy-free status vocabulary plus raw OS probes over an
//! external process handle. It carries no decision about which layers are
//! required, what a missing layer should do, or how to render a downgraded
//! claim — that policy lives entirely in `crates/nono-cli` (D-02, ADR-86;
//! wired up by Plan 08). Everything here is mechanical observation.
//!
//! # D-04: shared vocabulary
//!
//! [`LayerAttestationStatus`] is the same type Phase 118's per-session
//! receipts consume when attesting "which layers were confirmed active for
//! this process". One type, not two lists that can diverge.
//!
//! # D-11: platform-neutral types, `cfg(windows)` population
//!
//! [`LayerAttestationStatus`] and [`ProcessHandle`] compile on every target so
//! shared consumers (this crate's own tests, Phase 118's receipt type) never
//! need a `cfg(windows)` gate of their own. Only the probe function bodies —
//! the actual OS calls — are `cfg(target_os = "windows")`-populated; off
//! Windows every probe returns [`NonoError::UnsupportedPlatform`].
//!
//! # D-19: the supervisor attests, the confined process never does
//!
//! Every probe function in this module takes an already-open
//! [`ProcessHandle`] for a *target* process and performs an OS query against
//! it from outside. No function in this module accepts a claim, struct, or
//! string supplied BY the process being probed — the confined child is never
//! a source of any statement about its own containment (T-117-02). This
//! module also never itself calls `OpenProcess`: every probe takes a handle
//! the caller already has open (e.g. `launch.rs`'s D-21 gate point, which
//! holds the suspended child's handle from `CREATE_SUSPENDED`), keeping the
//! probe surface minimal and testable against handles the caller controls
//! (including the calling process's own handle, in this module's tests).

/// Platform-neutral process handle passed to every probe function.
///
/// D-11 (Warning-6 fix, checker pass 2): `crates/nono/Cargo.toml` only
/// declares `windows-sys` under `[target.'cfg(target_os = "windows")'.dependencies]`
/// — off Windows the crate is not even a dependency, so a non-cfg-gated
/// function signature naming `windows_sys::Win32::Foundation::HANDLE` directly
/// fails to compile on Linux/macOS. This alias is the fix: on Windows it IS
/// `HANDLE` (zero-cost); off Windows it is the unit type, so every probe
/// function signature in this module compiles everywhere while never naming
/// raw `HANDLE`.
#[cfg(target_os = "windows")]
pub type ProcessHandle = windows_sys::Win32::Foundation::HANDLE;

/// Non-Windows stub: no analogous OS handle type exists off Windows (D-11).
#[cfg(not(target_os = "windows"))]
pub type ProcessHandle = ();

/// Result of attesting one confinement layer against a real spawned child
/// (D-18).
///
/// Four states, not three or five — a reader must be able to tell "we proved
/// it", "the apply succeeded and we cannot independently re-check it", "we
/// checked and it is not there (or the check itself failed)", and "no such
/// layer exists here" apart, without out-of-band knowledge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerAttestationStatus {
    /// An independent OS query against the target process succeeded and
    /// matched the expected state — e.g. `IsProcessInJob` returned `true`
    /// for a layer whose contract requires containment.
    ///
    /// **Named sub-case, not a fifth variant:** a layer may also be
    /// `Confirmed` when the enforcing component itself reported success over
    /// a trusted IPC channel *before* spawn (e.g. the WFP service's
    /// filter-add acknowledgement). This sub-case is named explicitly —
    /// "confirmed-by-report-from-the-enforcing-component" — and is distinct
    /// from an independent post-hoc kernel observation like
    /// `IsProcessInJob`. A future reader must not conflate the WFP row's
    /// classification with a direct-query row's classification just because
    /// both happen to read `Confirmed`.
    Confirmed,
    /// The apply-time call succeeded (already fail-closed) but no
    /// independent post-hoc query exists to re-confirm it — e.g. DACL
    /// grants, where the apply `Result` itself IS the confirmation and there
    /// is no separate kernel object worth re-querying.
    ///
    /// Distinct from [`Self::Confirmed`]: the *apply* succeeded, but no
    /// probe re-observed it, because none exists for this layer. Distinct
    /// from [`Self::Unconfirmed`]: no probe was even attempted.
    EstablishedNotIndependentlyObservable,
    /// A probe was attempted and either the OS call itself failed, or it
    /// succeeded and authoritatively reported the expected state as ABSENT —
    /// e.g. a successful `TokenAppContainerSid` query that returns "no
    /// AppContainer SID present".
    ///
    /// This module deliberately does not distinguish "could not check" from
    /// "checked and it's absent" with a separate variant — no D-NN decision
    /// requires a fifth state — but a caller (Plan 08) must not treat a
    /// successful-but-negative probe result as meaning anything other than
    /// `Unconfirmed`.
    Unconfirmed,
    /// No such layer exists on this platform/arm — e.g. the minifilter row:
    /// ADR-65 stands, no minifilter, so per-file read policy is explicitly
    /// not claimed and that row reads `NotApplicable`, never `Unconfirmed`.
    NotApplicable,
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_attestation_status_derives_and_matches_exhaustively() {
        let statuses = [
            LayerAttestationStatus::Confirmed,
            LayerAttestationStatus::EstablishedNotIndependentlyObservable,
            LayerAttestationStatus::Unconfirmed,
            LayerAttestationStatus::NotApplicable,
        ];
        for status in statuses {
            let copy = status; // Copy
            assert_eq!(status, copy); // PartialEq/Eq
            let _debug = format!("{status:?}"); // Debug

            // Exhaustive match, no wildcard arm — compiler-enforced coverage
            // of every current and future variant.
            let _label: &str = match status {
                LayerAttestationStatus::Confirmed => "confirmed",
                LayerAttestationStatus::EstablishedNotIndependentlyObservable => {
                    "established-not-independently-observable"
                }
                LayerAttestationStatus::Unconfirmed => "unconfirmed",
                LayerAttestationStatus::NotApplicable => "not-applicable",
            };
        }
    }

    #[test]
    fn process_handle_compiles_as_function_parameter_on_every_target() {
        fn accepts_process_handle(_h: ProcessHandle) {}

        #[cfg(target_os = "windows")]
        let handle: ProcessHandle = std::ptr::null_mut();
        #[cfg(not(target_os = "windows"))]
        let handle: ProcessHandle = ();

        accepts_process_handle(handle);
    }
}
