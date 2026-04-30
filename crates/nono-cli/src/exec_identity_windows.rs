//! Windows Authenticode exec-identity recording (REQ-AUD-03 acceptance #2/#3).
//!
//! Plan 22-05b Task 4 — fork-only addition per CONTEXT § Integration Points
//! line 248 (D-17 ALLOWED). FFI style mirrors Phase 21's
//! `crates/nono/src/sandbox/windows.rs::try_set_mandatory_label`:
//! `encode_wide` UTF-16 conversion, `unsafe { ... }` blocks paired with
//! `// SAFETY:` doc comments, RAII close guard for `WTD_STATEACTION_CLOSE`,
//! `GetLastError` -> typed `NonoError`.
//!
//! Sibling field on the audit envelope per RESEARCH Contradiction #2:
//! `AuthenticodeStatus` does NOT mutate upstream's `ExecutableIdentity`
//! struct shape; SHA-256 capture stays independent and always happens.
//!
//! On any FFI failure (helpers absent / runtime error / unsigned binary),
//! the caller falls back to the SHA-256-only audit path captured by
//! `exec_identity::compute`.
//!
//! ## REQ-AUDC-03 fail-closed contract (v2.3, Phase 28)
//!
//! Phase 28 enables the chain walker by adding the
//! `Win32_Security_Cryptography_Catalog` + `Win32_Security_Cryptography_Sip`
//! features to `windows-sys` 0.59. With those gates in place,
//! `WTHelperProvDataFromStateData` and `WTHelperGetProvSignerFromChain`
//! become reachable, and `parse_signer_subject` / `parse_thumbprint`
//! return live extraction results instead of the v2.2 Decision 4 sentinel.
//!
//! On `WinVerifyTrust = Valid` (HRESULT 0): both `signer_subject` and
//! `thumbprint` MUST be populated (REQ-AUDC-03 acceptance #2). Any
//! chain-walk failure (NULL prov-data, empty cert chain, NULL leaf
//! CERT_CONTEXT, `CertGetNameStringW` returning empty,
//! `CertGetCertificateContextProperty` returning false) causes
//! `query_authenticode_status` to return `Err(NonoError::SandboxInit(..))`
//! carrying the failure cause and the original `WinVerifyTrust` HRESULT —
//! NEVER a silent `<unknown>` fallback.
//!
//! `Unsigned` (`HRESULT == TRUST_E_NOSIGNATURE`) and `InvalidSignature`
//! (`HRESULT != 0 && != TRUST_E_NOSIGNATURE`) paths are unchanged — chain
//! walk is NOT attempted; the discriminant alone is recorded.
//!
//! Behavior change vs v2.2: callers previously seeing
//! `AuthenticodeStatus::Valid { signer_subject: "<unknown>", thumbprint: "" }`
//! now see either `AuthenticodeStatus::Valid { signer_subject: <RDN>,
//! thumbprint: <40-char-hex> }` (success) or `Err(NonoError::SandboxInit)`
//! (chain-walk failure on Valid signature). This is intentional per
//! REQ-AUDC-03 acceptance #2 (fail-closed audit-recording).

#![cfg(target_os = "windows")]

use nono::{NonoError, Result};
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Security::Cryptography::{
    CertGetCertificateContextProperty, CertGetNameStringW, CERT_CONTEXT, CERT_HASH_PROP_ID,
    CERT_NAME_RDN_TYPE,
};
use windows_sys::Win32::Security::WinTrust::{
    WinVerifyTrust, WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData,
    CRYPT_PROVIDER_CERT, CRYPT_PROVIDER_DATA, CRYPT_PROVIDER_SGNR,
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO,
    WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
};

/// Authenticode status for an executable.
///
/// Sibling field on the audit envelope (RESEARCH Contradiction #2 — does
/// NOT mutate upstream's `ExecutableIdentity` struct shape; SHA-256 capture
/// stays independent and always happens).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticodeStatus {
    /// Signature valid; chain validated to a trusted root by `WinVerifyTrust`.
    ///
    /// Both `signer_subject` and `thumbprint` are guaranteed populated when
    /// this variant is constructed (REQ-AUDC-03 acceptance #2 fail-closed
    /// contract). If chain walking fails to extract either field on a
    /// `WinVerifyTrust=Valid` result, `query_authenticode_status` returns
    /// `Err(NonoError::SandboxInit(..))` carrying the chain-walk failure
    /// cause — it does NOT produce this variant with sentinel values.
    Valid {
        /// Signer subject (leaf-cert RDN, e.g.
        /// `"CN=Microsoft Windows, O=Microsoft Corporation, ..."`)
        /// extracted via `CertGetNameStringW(CERT_NAME_RDN_TYPE)` and
        /// sanitized to strip control characters via `sanitize_for_terminal`
        /// (defense-in-depth against attacker-controlled cert subjects
        /// containing terminal escape sequences — T-28-01 mitigation).
        signer_subject: String,
        /// SHA-1 thumbprint of the leaf signing cert as a 40-character
        /// UPPERCASE hex string, extracted via
        /// `CertGetCertificateContextProperty(CERT_HASH_PROP_ID)`.
        thumbprint: String,
    },
    /// File present but unsigned (`TRUST_E_NOSIGNATURE`).
    Unsigned,
    /// File signed but signature invalid / chain rejected. The `hresult`
    /// field carries the raw `WinVerifyTrust` return value for forensics.
    InvalidSignature { hresult: i32 },
    /// Signature query itself failed (e.g. file missing). Caller falls back
    /// to SHA-256-only audit envelope per AUD-03 acceptance #3.
    QueryFailed { reason: String },
}

/// `TRUST_E_NOSIGNATURE` — well-known WinTrust HRESULT for "file is not
/// signed". Surfaced verbatim in the audit ledger for forensic clarity.
const TRUST_E_NOSIGNATURE: u32 = 0x800B0100;

/// Record exec-identity Authenticode status for `path`.
///
/// Calls `WinVerifyTrust` with `WTD_REVOKE_NONE` (best-effort signature
/// query without CRL/OCSP latency per T-22-05b-02 mitigation; SHA-256
/// fallback ensures audit completes even on Authenticode failure). Always
/// pairs the `WTD_STATEACTION_VERIFY` call with a `WTD_STATEACTION_CLOSE`
/// call on Drop via `WinTrustCloseGuard` (T-22-05b-05 mitigation).
///
/// Returns `Ok(AuthenticodeStatus::QueryFailed { .. })` for path-conversion
/// failures rather than `Err(..)` so the caller's "fall through to SHA-256"
/// branch is exercised uniformly.
#[must_use = "ignoring the AuthenticodeStatus drops audit evidence"]
pub fn query_authenticode_status(path: &Path) -> Result<AuthenticodeStatus> {
    // UTF-16 path conversion (mirrors sandbox/windows.rs::try_set_mandatory_label).
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Heuristic: if `path` is empty post-conversion, the path conversion
    // produced nothing valid. Surface as QueryFailed so the caller falls
    // through to SHA-256.
    if wide.len() < 2 {
        return Ok(AuthenticodeStatus::QueryFailed {
            reason: format!("empty UTF-16 path conversion for {}", path.display()),
        });
    }

    let file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide.as_ptr(),
        hFile: std::ptr::null_mut(),
        pgKnownSubject: std::ptr::null_mut(),
    };

    let mut wtd = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        pPolicyCallbackData: std::ptr::null_mut(),
        pSIPClientData: std::ptr::null_mut(),
        dwUIChoice: WTD_UI_NONE,
        // Best-effort signature query without CRL/OCSP latency
        // (T-22-05b-02 mitigation; AUD-03 acceptance allows
        // "Signature failures do not prevent session start").
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &file_info as *const _ as *mut WINTRUST_FILE_INFO,
        },
        dwStateAction: WTD_STATEACTION_VERIFY,
        hWVTStateData: std::ptr::null_mut(),
        pwszURLReference: std::ptr::null_mut(),
        dwProvFlags: 0,
        dwUIContext: 0,
        pSignatureSettings: std::ptr::null_mut(),
    };

    // SAFETY: `WINTRUST_ACTION_GENERIC_VERIFY_V2` is a static GUID exported
    // by windows-sys; `&mut wtd` points to a valid stack-allocated
    // WINTRUST_DATA pre-populated above. `hWnd = NULL` is the documented
    // headless-verify shape. The first call requests verification; the
    // matching `WTD_STATEACTION_CLOSE` second call is guaranteed by the
    // RAII `WinTrustCloseGuard` constructed below (Drop fires even on
    // early return / panic). `wtd.hWVTStateData` is mutated by Windows
    // and read back in `parse_signer_subject` / `parse_thumbprint`.
    let verify_result: i32 = unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
            &mut wtd as *mut _ as *mut c_void,
        )
    };

    // RAII close guard MUST be constructed BEFORE we read `wtd.hWVTStateData`
    // so any early-return path (including panic propagation) still runs
    // the matching `WTD_STATEACTION_CLOSE` call. Mirrors Phase 21's
    // `_sd_guard` pattern (T-22-05b-05 mitigation).
    let _close_guard = WinTrustCloseGuard {
        wtd: &mut wtd as *mut WINTRUST_DATA,
    };

    let status = if verify_result == 0 {
        // Per REQ-AUDC-03 fail-closed contract: chain-walk failure on a
        // Valid signature returns Err(NonoError::SandboxInit) — NEVER a
        // silent <unknown> fallback. The `_close_guard` constructed above
        // dominates this branch, so its RAII Drop fires on the early-Err
        // path (T-22-05b-05 mitigation preserved; Drop runs the matching
        // WTD_STATEACTION_CLOSE call even on `?` propagation).
        let signer_subject = parse_signer_subject(&wtd)?;
        let thumbprint = parse_thumbprint(&wtd)?;
        AuthenticodeStatus::Valid {
            signer_subject,
            thumbprint,
        }
    } else if (verify_result as u32) == TRUST_E_NOSIGNATURE {
        AuthenticodeStatus::Unsigned
    } else {
        AuthenticodeStatus::InvalidSignature {
            hresult: verify_result,
        }
    };

    Ok(status)
}

/// RAII close-guard for the second `WinVerifyTrust` call with
/// `WTD_STATEACTION_CLOSE`. Mirrors Phase 21's `_sd_guard` pattern in
/// `sandbox/windows.rs`. ALWAYS runs the close call to release the
/// state allocated by the first verify call (T-22-05b-05 mitigation:
/// state-leak via mis-ordered close).
struct WinTrustCloseGuard {
    wtd: *mut WINTRUST_DATA,
}

impl Drop for WinTrustCloseGuard {
    fn drop(&mut self) {
        // SAFETY: `self.wtd` points to the same stack-allocated WINTRUST_DATA
        // referenced by the matching VERIFY call above. Setting
        // `dwStateAction = WTD_STATEACTION_CLOSE` and re-invoking
        // WinVerifyTrust with the same hWVTStateData is the documented
        // close-pair pattern. Errors from the close call are best-effort
        // (we are in Drop and cannot propagate); they do not affect audit
        // correctness because the state being leaked is verify-side only.
        unsafe {
            (*self.wtd).dwStateAction = WTD_STATEACTION_CLOSE;
            let _ = WinVerifyTrust(
                std::ptr::null_mut(),
                &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
                self.wtd as *mut c_void,
            );
        }
    }
}

/// Walk the WinVerifyTrust state data to the leaf signing certificate and
/// extract the RDN-formatted subject string via `CertGetNameStringW`.
///
/// Per REQ-AUDC-03 fail-closed contract: returns `Err(NonoError::SandboxInit)`
/// if any step in the chain fails. The caller MUST propagate via `?` —
/// `query_authenticode_status` is responsible for ensuring the
/// `WinTrustCloseGuard` is alive on the failure path (RAII Drop fires
/// even on early-Err return).
fn parse_signer_subject(wtd: &WINTRUST_DATA) -> Result<String> {
    let leaf_cert = leaf_cert_from(wtd)?;

    // First call: query the required UTF-16 buffer length (returns
    // wide-char count INCLUDING the null terminator).
    // SAFETY: `leaf_cert` is a non-NULL CERT_CONTEXT pointer obtained from
    // `leaf_cert_from` (which returns Err on NULL). Passing NULL/0 for
    // pvTypePara/pszNameString/cchNameString returns the required size.
    // CERT_NAME_RDN_TYPE produces an X.500 RDN string (e.g.
    // "CN=Microsoft Corporation, O=...").
    let cch_required = unsafe {
        CertGetNameStringW(
            leaf_cert,
            CERT_NAME_RDN_TYPE,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    if cch_required <= 1 {
        // Returns 1 on failure (just the null terminator); 0 should not
        // occur per Microsoft docs but defensively treat as fail-closed.
        return Err(authenticode_chain_walk_error(format!(
            "CertGetNameStringW(RDN_TYPE) sizing call returned {cch_required} (no subject available)"
        )));
    }

    // Second call: actually read the wide string.
    // SAFETY: buffer is sized to `cch_required` u16 elements per the
    // first-call result; `CertGetNameStringW` writes UP TO `cch_required`
    // wide chars including the null terminator.
    let mut buf: Vec<u16> = vec![0u16; cch_required as usize];
    let written = unsafe {
        CertGetNameStringW(
            leaf_cert,
            CERT_NAME_RDN_TYPE,
            0,
            std::ptr::null_mut(),
            buf.as_mut_ptr(),
            cch_required,
        )
    };
    if written <= 1 {
        return Err(authenticode_chain_walk_error(
            "CertGetNameStringW(RDN_TYPE) read call returned empty subject".to_string(),
        ));
    }

    // Strip the trailing null and decode UTF-16. Use saturating_sub for
    // CLAUDE.md § Coding Standards "Arithmetic" compliance.
    let truncated_len = written.saturating_sub(1) as usize;
    let raw = String::from_utf16_lossy(&buf[..truncated_len]);
    Ok(sanitize_for_terminal(&raw))
}

/// Walk the WinVerifyTrust state data to the leaf signing certificate and
/// extract the SHA-1 thumbprint via
/// `CertGetCertificateContextProperty(CERT_HASH_PROP_ID)`. Renders the
/// 20-byte hash as a 40-character UPPERCASE hex string.
///
/// Per REQ-AUDC-03 fail-closed contract: returns `Err(NonoError::SandboxInit)`
/// on any chain-walk failure. The caller MUST propagate via `?`.
fn parse_thumbprint(wtd: &WINTRUST_DATA) -> Result<String> {
    let leaf_cert = leaf_cert_from(wtd)?;

    // First call: query required byte length of the SHA-1 hash (always 20
    // for CERT_HASH_PROP_ID, but Microsoft pattern is to ask twice).
    let mut cb_required: u32 = 0;
    // SAFETY: `leaf_cert` is non-NULL per the helper's contract; NULL
    // pvData + zero pcbData populates `cb_required` with the needed byte
    // count. `CertGetCertificateContextProperty` returns BOOL (0 = fail).
    let ok = unsafe {
        CertGetCertificateContextProperty(
            leaf_cert,
            CERT_HASH_PROP_ID,
            std::ptr::null_mut(),
            &mut cb_required,
        )
    };
    if ok == 0 || cb_required == 0 || cb_required > 64 {
        // SHA-1 is 20 bytes; refuse implausible sizes (defense-in-depth
        // against malformed cert state — T-28-04 acceptance bound).
        return Err(authenticode_chain_walk_error(format!(
            "CertGetCertificateContextProperty(CERT_HASH_PROP_ID) sizing call failed (ok={ok}, cb_required={cb_required})"
        )));
    }

    // Second call: read the bytes.
    let mut buf: Vec<u8> = vec![0u8; cb_required as usize];
    // SAFETY: `buf` is sized per the first-call result; `cb_required` is
    // updated to the actual bytes-written count by Windows.
    let ok = unsafe {
        CertGetCertificateContextProperty(
            leaf_cert,
            CERT_HASH_PROP_ID,
            buf.as_mut_ptr() as *mut c_void,
            &mut cb_required,
        )
    };
    if ok == 0 {
        return Err(authenticode_chain_walk_error(
            "CertGetCertificateContextProperty(CERT_HASH_PROP_ID) read call failed".to_string(),
        ));
    }

    // Render as 40-char UPPERCASE hex (per must-haves.truths regex anchor
    // ^[0-9A-F]{40}$).
    let hex: String = buf
        .iter()
        .take(cb_required as usize)
        .map(|b| format!("{:02X}", b))
        .collect();
    Ok(hex)
}

/// Walk `WTHelperProvDataFromStateData → WTHelperGetProvSignerFromChain`
/// down to the leaf `CERT_CONTEXT` pointer. Shared between
/// `parse_signer_subject` and `parse_thumbprint` to avoid duplicating the
/// null-check ladder.
///
/// The returned pointer is owned by the WinTrust state data (which is in
/// turn owned by the caller's `WinTrustCloseGuard` RAII binding). The
/// caller MUST NOT free it. Lifetime is bounded by the close-guard.
fn leaf_cert_from(wtd: &WINTRUST_DATA) -> Result<*const CERT_CONTEXT> {
    // SAFETY: `wtd.hWVTStateData` was populated by the matching
    // `WinVerifyTrust(... WTD_STATEACTION_VERIFY ...)` call in
    // `query_authenticode_status` and is owned by the live
    // `WinTrustCloseGuard`. `WTHelperProvDataFromStateData` accepts a
    // state-data handle and returns either a non-NULL
    // `*mut CRYPT_PROVIDER_DATA` whose lifetime is tied to the state data
    // (do NOT free), or NULL on failure.
    let prov_data: *mut CRYPT_PROVIDER_DATA =
        unsafe { WTHelperProvDataFromStateData(wtd.hWVTStateData) };
    if prov_data.is_null() {
        return Err(authenticode_chain_walk_error(
            "WTHelperProvDataFromStateData returned NULL".to_string(),
        ));
    }

    // SAFETY: `prov_data` is non-NULL per the check above. The 0/0 indices
    // request the primary signer (idxSigner=0) and the leaf cert chain
    // (fCounterSigner=FALSE / idxCounterSigner=0). Returns NULL if the
    // signer index is out of range (treat as fail-closed).
    let signer: *mut CRYPT_PROVIDER_SGNR =
        unsafe { WTHelperGetProvSignerFromChain(prov_data, 0, 0, 0) };
    if signer.is_null() {
        return Err(authenticode_chain_walk_error(
            "WTHelperGetProvSignerFromChain returned NULL — no primary signer".to_string(),
        ));
    }

    // SAFETY: `signer` is non-NULL per the check above. The `pasCertChain`
    // field is a non-owning pointer to an array of `csCertChain`
    // CRYPT_PROVIDER_CERT entries. The leaf cert is the LAST entry
    // (index `csCertChain - 1`) per the Microsoft Authenticode chain
    // ordering convention (root at index 0, leaf at the end).
    let (cert_chain, chain_len): (*mut CRYPT_PROVIDER_CERT, u32) =
        unsafe { ((*signer).pasCertChain, (*signer).csCertChain) };
    if cert_chain.is_null() || chain_len == 0 {
        return Err(authenticode_chain_walk_error(format!(
            "Authenticode signer carries empty cert chain (chain_len={chain_len})"
        )));
    }

    // SAFETY: leaf is at `chain_len - 1`. We checked chain_len > 0 above
    // (so the saturating_sub never underflows even though it cannot here).
    // `pCert` is a `*const CERT_CONTEXT` (PCCERT_CONTEXT) owned by the
    // WinTrust state data.
    let leaf_cert: *const CERT_CONTEXT = unsafe {
        let leaf_idx = chain_len.saturating_sub(1) as usize;
        let leaf_entry: *mut CRYPT_PROVIDER_CERT = cert_chain.add(leaf_idx);
        (*leaf_entry).pCert
    };
    if leaf_cert.is_null() {
        return Err(authenticode_chain_walk_error(
            "Authenticode leaf CERT_CONTEXT is NULL".to_string(),
        ));
    }

    Ok(leaf_cert)
}

/// Build a fail-closed `NonoError` carrying the chain-walk failure cause.
///
/// REQ-AUDC-03 acceptance #2: chain-walk failure on a `WinVerifyTrust=Valid`
/// signature is an audit-integrity failure (we cannot record the binary's
/// identity). Phase 28 routes this through `NonoError::SandboxInit`
/// because the existing `NonoError` taxonomy in `crates/nono/src/error.rs`
/// does not have an `AuditIntegrity` variant; `SandboxInit` is the
/// established Phase 21 + Phase 22 sink for Windows-FFI-adjacent failures
/// (see `capability_ext.rs`, `execution_runtime.rs`, `exec_strategy.rs`
/// for prior usage). The "authenticode chain-walk failed" prefix makes
/// the cause unambiguous in logs.
fn authenticode_chain_walk_error(hint: String) -> NonoError {
    NonoError::SandboxInit(format!(
        "authenticode chain-walk failed (REQ-AUDC-03 fail-closed): {hint}"
    ))
}

/// Strip control characters and ANSI escape sequences from a chain-extracted
/// subject string before recording it in the audit ledger.
///
/// Defense-in-depth (T-28-01 mitigation): a malicious cert subject containing
/// terminal escape sequences must not be able to reflow the operator's TTY
/// when `nono audit show <id>` renders the audit ledger. Mirrors the
/// `sanitize_for_terminal` helper in `audit_commands.rs` /
/// `terminal_approval.rs`. Inlined here (rather than re-exported) because
/// those functions are private to their respective modules and Phase 28's
/// scope is intentionally tight.
fn sanitize_for_terminal(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC: consume CSI/OSC/DCS/APC/PM/SOS sequences without emitting.
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    // CSI: consume until final byte 0x40-0x7E.
                    chars.next();
                    for seq_c in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&seq_c) {
                            break;
                        }
                    }
                } else if matches!(next, ']' | 'P' | '_' | '^' | 'X') {
                    // OSC/DCS/APC/PM/SOS: consume until ST (ESC \) or BEL.
                    chars.next();
                    let mut prev = '\0';
                    for seq_c in chars.by_ref() {
                        if seq_c == '\x07' || (prev == '\x1b' && seq_c == '\\') {
                            break;
                        }
                        prev = seq_c;
                    }
                } else {
                    // Lone ESC followed by non-CSI: drop ESC, keep next.
                    chars.next();
                }
            }
        } else if c.is_control() && c != '\t' {
            // Drop other control chars (newlines, CR, BS, etc. should not
            // appear in a cert RDN subject; if they do, attacker-controlled).
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_temp_file_returns_unsigned_or_invalid() {
        // A short tempfile that LOOKS like a PE start but has no signature.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unsigned.exe");
        std::fs::write(&path, b"MZ\x90\x00\x03\x00\x00\x00").unwrap();
        let status = query_authenticode_status(&path).unwrap();
        // Either Unsigned (most likely) or InvalidSignature is acceptable —
        // both signal "fall back to SHA-256". The unit test refuses to
        // require Valid for a tempfile.
        assert!(
            matches!(
                status,
                AuthenticodeStatus::Unsigned | AuthenticodeStatus::InvalidSignature { .. }
            ),
            "expected Unsigned or InvalidSignature, got: {status:?}"
        );
    }

    #[test]
    fn missing_path_returns_invalid_or_query_failed() {
        let path = Path::new(r"C:\nonexistent\path\that\should\not\exist.exe");
        let result = query_authenticode_status(path);
        match result {
            Ok(AuthenticodeStatus::QueryFailed { .. })
            | Ok(AuthenticodeStatus::InvalidSignature { .. })
            | Ok(AuthenticodeStatus::Unsigned)
            | Err(_) => (),
            other => panic!(
                "expected QueryFailed/InvalidSignature/Unsigned/Err for missing path, got: {other:?}"
            ),
        }
    }
}
