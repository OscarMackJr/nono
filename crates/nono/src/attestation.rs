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

use crate::{NonoError, Result};
use serde::{Deserialize, Serialize};

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

/// Handle to a specific Job Object, passed to [`probe_in_job`] so the probe
/// asks "is this process in **THIS** job" rather than the vacuous "is this
/// process in ANY job" (Phase 117 review CR-02). Same platform-neutrality
/// rationale as [`ProcessHandle`].
#[cfg(target_os = "windows")]
pub type JobHandle = windows_sys::Win32::Foundation::HANDLE;

/// Non-Windows stub (D-11).
#[cfg(not(target_os = "windows"))]
pub type JobHandle = ();

/// Result of attesting one confinement layer against a real spawned child
/// (D-18).
///
/// Four states, not three or five — a reader must be able to tell "we proved
/// it", "the apply succeeded and we cannot independently re-check it", "we
/// checked and it is not there (or the check itself failed)", and "no such
/// layer exists here" apart, without out-of-band knowledge.
///
/// `Serialize`/`Deserialize` (Rule 3 auto-fix, Phase 118 Plan 01): this type
/// is a field of `crate::receipt::LayerReceiptRow`, which must round-trip
/// through the receipt's on-disk JSON representation (D-12/RCPT-02). No
/// behavior change — this remains the same policy-free status vocabulary
/// `LayerAttestationStatus` always was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Queries the target process's mandatory integrity level via
/// `GetTokenInformation(TokenIntegrityLevel)` and returns the RID (e.g.
/// `SECURITY_MANDATORY_LOW_RID`) — the same call shape already used by this
/// crate's supervisor for its own token (`exec_strategy_windows/mod.rs`'s
/// `probe_integrity_level_support`), generalized here to query an arbitrary
/// external process handle (D-17, D-19).
///
/// # Errors
///
/// Returns [`NonoError::LayerAttestationFailed`] if opening the token or
/// either `GetTokenInformation` call fails. Never returns a default RID on
/// failure — fail-closed (T-117-12).
///
/// The function is `safe` because `process` is only ever passed to
/// `OpenProcessToken`, whose own unsafe contract is documented at its call
/// site inside this function; the caller-owned handle is never dereferenced
/// as a Rust reference or pointer arithmetic target.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn probe_integrity_level(process: ProcessHandle) -> Result<u32> {
    #[cfg(target_os = "windows")]
    {
        use crate::sandbox::windows::OwnedHandle;
        use std::mem::size_of;
        use std::ptr::null_mut;
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::Security::{
            GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
            TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::OpenProcessToken;

        let mut h_token_raw = null_mut();
        let ok = unsafe {
            // SAFETY: `process` is a caller-owned, already-open process
            // handle (D-19: this module never calls `OpenProcess` itself);
            // `h_token_raw` is a valid out-pointer. Requests TOKEN_QUERY
            // only.
            OpenProcessToken(process, TOKEN_QUERY, &mut h_token_raw)
        };
        if ok == 0 {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "IntegrityLevel".to_string(),
                reason: format!("OpenProcessToken failed (GetLastError={gle})"),
            });
        }
        let h_token = OwnedHandle(h_token_raw);

        // Two-call GetTokenInformation pattern: first probe with a null
        // buffer to discover the required size.
        let mut needed: u32 = 0;
        unsafe {
            // SAFETY: null buffer + 0 length is the documented size-probe
            // call; the required size is written into `needed`.
            GetTokenInformation(
                h_token.raw(),
                TokenIntegrityLevel,
                null_mut(),
                0,
                &mut needed,
            );
        }
        if (needed as usize) < size_of::<TOKEN_MANDATORY_LABEL>() {
            return Err(NonoError::LayerAttestationFailed {
                layer: "IntegrityLevel".to_string(),
                reason: format!(
                    "GetTokenInformation(TokenIntegrityLevel) size probe returned an \
                     undersized buffer ({needed} bytes)"
                ),
            });
        }

        let mut buf = vec![0u8; needed as usize];
        let ok = unsafe {
            // SAFETY: `buf` is sized by the probe call above; `h_token` is a
            // valid open token handle owned by this function.
            GetTokenInformation(
                h_token.raw(),
                TokenIntegrityLevel,
                buf.as_mut_ptr().cast::<std::ffi::c_void>(),
                needed,
                &mut needed,
            )
        };
        if ok == 0 {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "IntegrityLevel".to_string(),
                reason: format!(
                    "GetTokenInformation(TokenIntegrityLevel) failed (GetLastError={gle})"
                ),
            });
        }

        // SAFETY: `buf` was filled by the successful GetTokenInformation
        // call above with a TOKEN_MANDATORY_LABEL prefix; layout is
        // documented in the Win32 SDK.
        let label = unsafe { &*(buf.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()) };
        // SAFETY: `label.Label.Sid` is a valid SID pointer for `buf`'s
        // lifetime; `GetSidSubAuthorityCount` returns a pointer to a u8
        // within that SID structure.
        let sub_authority_count = unsafe { *GetSidSubAuthorityCount(label.Label.Sid) };
        if sub_authority_count == 0 {
            return Err(NonoError::LayerAttestationFailed {
                layer: "IntegrityLevel".to_string(),
                reason: "integrity-label SID has zero sub-authorities".to_string(),
            });
        }
        // SAFETY: same SID pointer is still valid; `(count - 1)` is
        // in-range given the check above.
        let rid = unsafe { *GetSidSubAuthority(label.Label.Sid, (sub_authority_count - 1) as u32) };
        Ok(rid)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // `process` is unit (`()`) off Windows (D-11); this binding exists
        // solely to mark the parameter as used, since it is a genuine,
        // referenced argument on the Windows branch above.
        #[allow(clippy::let_unit_value)]
        let _ = process;
        Err(NonoError::UnsupportedPlatform(
            "attestation probes are Windows-only".to_string(),
        ))
    }
}

/// Queries whether the target process is a member of the **specific** Job
/// Object named by `job`, via `IsProcessInJob` — promotes the call from
/// test-only usage (`exec_strategy_windows/launch.rs`'s
/// `broker_dispatch_tests`) to production, generalized to accept any
/// external process handle (D-17, D-19).
///
/// # Why `job` is mandatory (Phase 117 review CR-02)
///
/// The original signature passed a **null** job handle, which asks Windows
/// "is this process in ANY job". On Windows 8+ nested-job hosts (shells,
/// terminals, CI harnesses) a child inherits its parent's job membership, so
/// a process that was never assigned to the supervisor's containment job
/// still answered `true`. The probe could therefore only ever say yes, and
/// its `Confirmed` classification carried no information. Requiring the
/// caller to name the job it actually created makes the negative answer
/// reachable: a process the supervisor failed to assign is NOT in that job
/// and the probe returns `Ok(false)`.
///
/// A null/invalid `job` is rejected as an error rather than silently
/// restoring the "ANY job" behaviour — fail-closed, so the vacuous form is
/// not reachable by accident.
///
/// # Errors
///
/// Returns [`NonoError::LayerAttestationFailed`] if `job` is null, or if the
/// `IsProcessInJob` call itself fails — fail-closed.
///
/// The function is `safe` because `process`/`job` are only ever passed to
/// `IsProcessInJob`, whose own unsafe contract is documented at its call
/// site inside this function; the caller-owned handles are never
/// dereferenced as a Rust reference or pointer arithmetic target.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn probe_in_job(process: ProcessHandle, job: JobHandle) -> Result<bool> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::{GetLastError, BOOL, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::JobObjects::IsProcessInJob;

        if job.is_null() || job == INVALID_HANDLE_VALUE {
            return Err(NonoError::LayerAttestationFailed {
                layer: "JobObjectContainment".to_string(),
                reason: "no containment job handle supplied — refusing to fall back to the \
                         vacuous \"is this process in ANY job\" query (fail-closed)"
                    .to_string(),
            });
        }

        let mut in_job: BOOL = 0;
        let ok = unsafe {
            // SAFETY: `process` and `job` are caller-owned, already-open
            // handles; `in_job` is a valid out-pointer. Naming a concrete
            // job asks "is this process in THIS job", per the documented
            // `IsProcessInJob` contract.
            IsProcessInJob(process, job, &mut in_job)
        };
        if ok == 0 {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "JobObjectContainment".to_string(),
                reason: format!("IsProcessInJob failed (GetLastError={gle})"),
            });
        }
        Ok(in_job != 0)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // `process`/`job` are unit (`()`) off Windows (D-11); these bindings
        // exist solely to mark the parameters as used, since they are
        // genuine, referenced arguments on the Windows branch above.
        #[allow(clippy::let_unit_value)]
        let _ = process;
        #[allow(clippy::let_unit_value)]
        let _ = job;
        Err(NonoError::UnsupportedPlatform(
            "attestation probes are Windows-only".to_string(),
        ))
    }
}

/// Queries the target process's token for an AppContainer package SID via
/// `TokenAppContainerSid` — genuinely new production FFI surface (this exact
/// `TOKEN_INFORMATION_CLASS` value has zero production hits elsewhere in the
/// crate), generalized to accept any external process handle (D-17, D-19).
///
/// Returns `Ok(None)` if the token carries no AppContainer SID — a
/// successful probe with a negative result, never conflated with a probe
/// failure. Returns `Ok(Some(sid_string))` (SDDL form, `S-1-15-2-*`) if
/// present.
///
/// # Errors
///
/// Returns [`NonoError::LayerAttestationFailed`] only when an OS call itself
/// fails (opening the token, or `ConvertSidToStringSidW`) — never when the
/// probe succeeds and simply finds no AppContainer SID.
///
/// The function is `safe` because `process` is only ever passed to
/// `OpenProcessToken`, whose own unsafe contract is documented at its call
/// site inside this function; the caller-owned handle is never dereferenced
/// as a Rust reference or pointer arithmetic target.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn probe_app_container_sid(process: ProcessHandle) -> Result<Option<String>> {
    #[cfg(target_os = "windows")]
    {
        use crate::sandbox::windows::OwnedHandle;
        use std::mem::size_of;
        use std::ptr::null_mut;
        use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
        use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenAppContainerSid, TOKEN_APPCONTAINER_INFORMATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::OpenProcessToken;

        let mut h_token_raw = null_mut();
        let ok = unsafe {
            // SAFETY: `process` is caller-owned and already open (D-19);
            // this module never calls `OpenProcess` itself.
            OpenProcessToken(process, TOKEN_QUERY, &mut h_token_raw)
        };
        if ok == 0 {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "AppContainerProfile".to_string(),
                reason: format!("OpenProcessToken failed (GetLastError={gle})"),
            });
        }
        let h_token = OwnedHandle(h_token_raw);

        let mut needed: u32 = 0;
        unsafe {
            // SAFETY: null-buffer size probe; `needed` receives the
            // required size. For non-AppContainer tokens `needed` stays 0.
            GetTokenInformation(
                h_token.raw(),
                TokenAppContainerSid,
                null_mut(),
                0,
                &mut needed,
            );
        }
        // needed == 0, or an undersized non-zero value (some Windows builds
        // return a short size for a non-AppContainer token), means the token
        // carries no AppContainer SID — a successful probe with a negative
        // result, NOT a probe failure.
        if needed == 0 || (needed as usize) < size_of::<TOKEN_APPCONTAINER_INFORMATION>() {
            return Ok(None);
        }

        let mut buf = vec![0u8; needed as usize];
        let ok = unsafe {
            // SAFETY: `buf` is sized by the probe call above; `h_token` is a
            // valid open token handle owned by this function.
            GetTokenInformation(
                h_token.raw(),
                TokenAppContainerSid,
                buf.as_mut_ptr().cast::<std::ffi::c_void>(),
                needed,
                &mut needed,
            )
        };
        if ok == 0 {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "AppContainerProfile".to_string(),
                reason: format!(
                    "GetTokenInformation(TokenAppContainerSid) failed (GetLastError={gle})"
                ),
            });
        }

        // SAFETY: `buf` is at least size_of::<TOKEN_APPCONTAINER_INFORMATION>()
        // bytes, filled by the successful GetTokenInformation call above.
        let info = unsafe { &*(buf.as_ptr().cast::<TOKEN_APPCONTAINER_INFORMATION>()) };
        // Some Windows builds return the struct with a null SID pointer
        // instead of needed==0 — treat that as "no AppContainer SID" too.
        if info.TokenAppContainer.is_null() {
            return Ok(None);
        }

        // Convert the PSID to SDDL string form while `buf` (and therefore
        // the PSID it owns) is still alive.
        let mut str_ptr: windows_sys::core::PWSTR = null_mut();
        let ok = unsafe {
            // SAFETY: `info.TokenAppContainer` is a valid PSID owned by
            // `buf` (kept alive in this scope); `str_ptr` is a valid
            // out-pointer.
            ConvertSidToStringSidW(info.TokenAppContainer, &mut str_ptr)
        };
        if ok == 0 || str_ptr.is_null() {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "AppContainerProfile".to_string(),
                reason: format!("ConvertSidToStringSidW failed (GetLastError={gle})"),
            });
        }
        let sid_str = unsafe {
            // SAFETY: `str_ptr` points to a nul-terminated UTF-16 string
            // allocated by ConvertSidToStringSidW; scan for the terminator
            // to size it before copying.
            let mut len = 0usize;
            while *str_ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(str_ptr, len);
            String::from_utf16_lossy(slice)
        };
        unsafe {
            // SAFETY: `str_ptr` was allocated by ConvertSidToStringSidW and
            // is freed exactly once here via LocalFree as documented.
            let _ = LocalFree(str_ptr.cast::<std::ffi::c_void>());
        }
        // `buf` drops here — the PSID inside it is no longer referenced.
        Ok(Some(sid_str))
    }

    #[cfg(not(target_os = "windows"))]
    {
        // `process` is unit (`()`) off Windows (D-11); this binding exists
        // solely to mark the parameter as used, since it is a genuine,
        // referenced argument on the Windows branch above.
        #[allow(clippy::let_unit_value)]
        let _ = process;
        Err(NonoError::UnsupportedPlatform(
            "attestation probes are Windows-only".to_string(),
        ))
    }
}

/// Queries the target process's token for its restricting SIDs via
/// `TokenRestrictedSids` — promotes the call from test-only usage
/// (`exec_strategy_windows/restricted_token.rs`'s test module) to production,
/// generalized to accept any external process handle (D-17, D-19).
///
/// Returns the SDDL string form of every restricting SID. An unrestricted
/// token legitimately reports zero restricting SIDs — an empty `Vec`, not an
/// error.
///
/// # Errors
///
/// Returns [`NonoError::LayerAttestationFailed`] if opening the token, the
/// `GetTokenInformation` fill call, or any per-SID `ConvertSidToStringSidW`
/// call fails.
///
/// The function is `safe` because `process` is only ever passed to
/// `OpenProcessToken`, whose own unsafe contract is documented at its call
/// site inside this function; the caller-owned handle is never dereferenced
/// as a Rust reference or pointer arithmetic target.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn probe_restricted_sids(process: ProcessHandle) -> Result<Vec<String>> {
    #[cfg(target_os = "windows")]
    {
        use crate::sandbox::windows::OwnedHandle;
        use std::ptr::null_mut;
        use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
        use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenRestrictedSids, TOKEN_GROUPS, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::OpenProcessToken;

        let mut h_token_raw = null_mut();
        let ok = unsafe {
            // SAFETY: `process` is caller-owned and already open (D-19).
            OpenProcessToken(process, TOKEN_QUERY, &mut h_token_raw)
        };
        if ok == 0 {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "RestrictedSids".to_string(),
                reason: format!("OpenProcessToken failed (GetLastError={gle})"),
            });
        }
        let h_token = OwnedHandle(h_token_raw);

        let mut needed: u32 = 0;
        unsafe {
            // SAFETY: null-buffer size probe.
            GetTokenInformation(
                h_token.raw(),
                TokenRestrictedSids,
                null_mut(),
                0,
                &mut needed,
            );
        }
        if (needed as usize) < std::mem::size_of::<TOKEN_GROUPS>() {
            // An unrestricted token legitimately reports zero restricting
            // SIDs (GroupCount == 0) — an empty Vec, not an error.
            return Ok(Vec::new());
        }

        let mut buf = vec![0u8; needed as usize];
        let ok = unsafe {
            // SAFETY: `buf` is sized by the probe call above; `h_token` is a
            // valid open token handle owned by this function.
            GetTokenInformation(
                h_token.raw(),
                TokenRestrictedSids,
                buf.as_mut_ptr().cast::<std::ffi::c_void>(),
                needed,
                &mut needed,
            )
        };
        if ok == 0 {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::LayerAttestationFailed {
                layer: "RestrictedSids".to_string(),
                reason: format!(
                    "GetTokenInformation(TokenRestrictedSids) failed (GetLastError={gle})"
                ),
            });
        }

        // SAFETY: `buf` is at least size_of::<TOKEN_GROUPS>() bytes, filled
        // by the successful GetTokenInformation call above.
        let token_groups = unsafe { &*(buf.as_ptr().cast::<TOKEN_GROUPS>()) };
        let group_count = token_groups.GroupCount as usize;
        let groups_ptr = token_groups.Groups.as_ptr();

        let mut result = Vec::with_capacity(group_count);
        for i in 0..group_count {
            // SAFETY: `groups_ptr` points into `buf`, which the OS
            // populated with exactly `GroupCount` `SID_AND_ATTRIBUTES`
            // entries; `i` is in-range.
            let entry = unsafe { &*groups_ptr.add(i) };
            let mut str_ptr: windows_sys::core::PWSTR = null_mut();
            let ok = unsafe {
                // SAFETY: `entry.Sid` is a valid PSID owned by `buf` (alive
                // for this scope); `str_ptr` is a valid out-pointer.
                ConvertSidToStringSidW(entry.Sid, &mut str_ptr)
            };
            if ok == 0 || str_ptr.is_null() {
                let gle = unsafe { GetLastError() };
                return Err(NonoError::LayerAttestationFailed {
                    layer: "RestrictedSids".to_string(),
                    reason: format!(
                        "ConvertSidToStringSidW failed for restricting SID #{i} \
                         (GetLastError={gle})"
                    ),
                });
            }
            let sid_str = unsafe {
                // SAFETY: `str_ptr` is nul-terminated UTF-16 allocated by
                // ConvertSidToStringSidW; scan for the terminator to size it.
                let mut len = 0usize;
                while *str_ptr.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(str_ptr, len);
                String::from_utf16_lossy(slice)
            };
            unsafe {
                // SAFETY: freed exactly once here via LocalFree.
                let _ = LocalFree(str_ptr.cast::<std::ffi::c_void>());
            }
            result.push(sid_str);
        }
        Ok(result)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // `process` is unit (`()`) off Windows (D-11); this binding exists
        // solely to mark the parameter as used, since it is a genuine,
        // referenced argument on the Windows branch above.
        #[allow(clippy::let_unit_value)]
        let _ = process;
        Err(NonoError::UnsupportedPlatform(
            "attestation probes are Windows-only".to_string(),
        ))
    }
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

#[cfg(all(test, target_os = "windows"))]
mod windows_probe_tests {
    use super::*;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    /// RAII wrapper over a real, unnamed Job Object created for one test.
    struct TestJob(HANDLE);

    impl TestJob {
        fn create() -> Self {
            use windows_sys::Win32::System::JobObjects::CreateJobObjectW;
            // SAFETY: null security attributes + null name creates an
            // unnamed job object owned by this process.
            let h = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            assert!(!h.is_null(), "CreateJobObjectW failed in test setup");
            Self(h)
        }

        fn raw(&self) -> HANDLE {
            self.0
        }
    }

    impl Drop for TestJob {
        fn drop(&mut self) {
            // SAFETY: `self.0` is a live handle this struct owns.
            unsafe { CloseHandle(self.0) };
        }
    }

    /// RAII wrapper over a real `CREATE_SUSPENDED` child that is terminated
    /// (never resumed) on drop — the same shape the D-21 gate observes.
    struct SuspendedChild {
        process: HANDLE,
        thread: HANDLE,
    }

    impl SuspendedChild {
        fn spawn() -> Self {
            use windows_sys::Win32::System::Threading::{
                CreateProcessW, CREATE_SUSPENDED, PROCESS_INFORMATION, STARTUPINFOW,
            };

            let mut command_line: Vec<u16> = "cmd.exe /c exit 0".encode_utf16().collect();
            command_line.push(0);
            // SAFETY: zeroed POD structs are the documented initial state.
            let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
            si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
            let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
            let ok = unsafe {
                // SAFETY: `command_line` is a mutable, NUL-terminated UTF-16
                // buffer; every other pointer argument is null (defaults).
                CreateProcessW(
                    std::ptr::null(),
                    command_line.as_mut_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    CREATE_SUSPENDED,
                    std::ptr::null(),
                    std::ptr::null(),
                    &si,
                    &mut pi,
                )
            };
            assert!(
                ok != 0,
                "CreateProcessW(CREATE_SUSPENDED) failed in test setup"
            );
            Self {
                process: pi.hProcess,
                thread: pi.hThread,
            }
        }

        fn process(&self) -> HANDLE {
            self.process
        }
    }

    impl Drop for SuspendedChild {
        fn drop(&mut self) {
            use windows_sys::Win32::System::Threading::TerminateProcess;
            // SAFETY: both handles are live and owned by this struct; the
            // child is still suspended and has executed no user code.
            unsafe {
                TerminateProcess(self.process, 1);
                CloseHandle(self.thread);
                CloseHandle(self.process);
            }
        }
    }

    #[test]
    fn integrity_level_probe_on_invalid_handle_returns_layer_attestation_failed() {
        let result = probe_integrity_level(std::ptr::null_mut());
        match result {
            Err(NonoError::LayerAttestationFailed { .. }) => {}
            other => panic!(
                "expected Err(LayerAttestationFailed) for a null process handle, got {other:?}"
            ),
        }
    }

    #[test]
    fn in_job_probe_on_invalid_handle_returns_layer_attestation_failed() {
        // A real (non-null) job handle is supplied so the failure under test
        // is the invalid PROCESS handle, not the null-job fail-closed guard.
        let job = TestJob::create();
        let result = probe_in_job(std::ptr::null_mut(), job.raw());
        match result {
            Err(NonoError::LayerAttestationFailed { .. }) => {}
            other => panic!(
                "expected Err(LayerAttestationFailed) for a null process handle, got {other:?}"
            ),
        }
    }

    /// CR-02 fail-closed guard: a null job handle must be rejected outright
    /// rather than silently restoring the vacuous "is this process in ANY
    /// job" query.
    #[test]
    fn in_job_probe_with_null_job_handle_is_rejected_fail_closed() {
        // SAFETY: GetCurrentProcess returns an always-valid pseudo-handle.
        let current = unsafe { GetCurrentProcess() };
        match probe_in_job(current, std::ptr::null_mut()) {
            Err(NonoError::LayerAttestationFailed { reason, .. }) => {
                assert!(
                    reason.contains("no containment job handle"),
                    "expected the fail-closed null-job reason, got {reason:?}"
                );
            }
            other => {
                panic!("expected Err(LayerAttestationFailed) for a null job handle, got {other:?}")
            }
        }
    }

    #[test]
    fn app_container_sid_probe_on_invalid_handle_returns_layer_attestation_failed() {
        let result = probe_app_container_sid(std::ptr::null_mut());
        match result {
            Err(NonoError::LayerAttestationFailed { .. }) => {}
            other => panic!(
                "expected Err(LayerAttestationFailed) for a null process handle, got {other:?}"
            ),
        }
    }

    #[test]
    fn restricted_sids_probe_on_invalid_handle_returns_layer_attestation_failed() {
        let result = probe_restricted_sids(std::ptr::null_mut());
        match result {
            Err(NonoError::LayerAttestationFailed { .. }) => {}
            other => panic!(
                "expected Err(LayerAttestationFailed) for a null process handle, got {other:?}"
            ),
        }
    }

    /// CR-02 non-vacuity proof, BOTH polarities, on a real spawned child.
    ///
    /// The pre-fix probe passed a null job handle ("is this process in ANY
    /// job") and could not return a negative on this host: the executor's own
    /// comment recorded that a spawned-but-unassigned process silently
    /// inherits the test runner's job membership and still answers `true`.
    /// Naming a concrete job makes the negative reachable — this test asserts
    /// the SAME child answers `true` for the job it was assigned to and
    /// `false` for a different, live job it was never assigned to. If the
    /// null-job form were restored, the `false` assertion below would fail.
    #[test]
    fn in_job_probe_distinguishes_the_assigned_job_from_another_job() {
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;

        let assigned_job = TestJob::create();
        let other_job = TestJob::create();
        let child = SuspendedChild::spawn();

        let ok = unsafe {
            // SAFETY: both handles are live and owned by this test.
            AssignProcessToJobObject(assigned_job.raw(), child.process())
        };
        assert!(ok != 0, "AssignProcessToJobObject failed in test setup");

        assert_eq!(
            probe_in_job(child.process(), assigned_job.raw()).ok(),
            Some(true),
            "the child WAS assigned to `assigned_job`, so the probe must confirm it"
        );
        assert_eq!(
            probe_in_job(child.process(), other_job.raw()).ok(),
            Some(false),
            "the child was NEVER assigned to `other_job` — the probe MUST be able to return \
             this negative, which the pre-CR-02 null-job form structurally could not"
        );
    }

    #[test]
    fn app_container_sid_probe_on_current_process_returns_ok_none() {
        // SAFETY: GetCurrentProcess returns a pseudo-handle that is always
        // valid; no cleanup is required (it is not a real handle to close).
        let current = unsafe { GetCurrentProcess() };
        let result = probe_app_container_sid(current);
        assert_eq!(
            result.ok(),
            Some(None),
            "the test runner's own token carries no AppContainer SID"
        );
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod non_windows_stub_tests {
    use super::*;

    #[test]
    fn integrity_level_probe_off_windows_returns_unsupported_platform() {
        assert!(matches!(
            probe_integrity_level(()),
            Err(NonoError::UnsupportedPlatform(_))
        ));
    }

    #[test]
    fn in_job_probe_off_windows_returns_unsupported_platform() {
        assert!(matches!(
            probe_in_job((), ()),
            Err(NonoError::UnsupportedPlatform(_))
        ));
    }

    #[test]
    fn app_container_sid_probe_off_windows_returns_unsupported_platform() {
        assert!(matches!(
            probe_app_container_sid(()),
            Err(NonoError::UnsupportedPlatform(_))
        ));
    }

    #[test]
    fn restricted_sids_probe_off_windows_returns_unsupported_platform() {
        assert!(matches!(
            probe_restricted_sids(()),
            Err(NonoError::UnsupportedPlatform(_))
        ));
    }
}
