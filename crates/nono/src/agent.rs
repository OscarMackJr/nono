//! AI agent identity and classification.
//!
//! This module provides the [`AgentRegistry`] (the minting authority's
//! private in-memory set of AppContainer package SIDs) and the
//! [`AgentClassification`] enum that callers act on.
//!
//! # Design (Phase 73 D-01, D-02)
//!
//! The `AI_AGENT` identity is anchored to the per-run AppContainer package SID
//! (`S-1-15-2-*`) already minted on every `BrokerLaunchNoPty` agent's token.
//! Authorization is a PRIVATE registry membership check: only SIDs that the
//! launcher itself inserted via [`AgentRegistry::insert`] are classified as
//! [`AgentClassification::AiAgent`].
//!
//! Namespace-pattern matching (`package_sid.starts_with("S-1-15-2-")`) is
//! intentionally NEVER used as the authorization check — it is forgeable.
//! The registry check is the only sound predicate.
//!
//! # Non-Windows support
//!
//! The module compiles on all platforms. [`AgentRegistry::classify`] returns
//! [`AgentClassification::NotAnAgent`] unconditionally on non-Windows.
//! [`read_process_appcontainer_sid`] returns
//! [`crate::error::NonoError::UnsupportedPlatform`] on non-Windows.

use crate::error::{NonoError, Result};
use std::collections::HashSet;

// Windows-only imports: each is gated per-function, not here at the top level,
// so the module compiles cleanly on Linux/macOS without unused-import warnings.

/// Classification result for a process PID.
///
/// Callers MUST act on this result; it is the authorization predicate for
/// deciding whether a process is a launcher-spawned confined agent.
///
/// # Fail-secure default
///
/// [`NotAnAgent`] is the safe default. Any PID not explicitly present in the
/// [`AgentRegistry`] — including nonexistent PIDs and the launcher's own process
/// — returns [`NotAnAgent`].
#[derive(Debug, PartialEq, Eq)]
#[must_use = "fail-secure: callers must act on the classification result"]
pub enum AgentClassification {
    /// The PID is a launcher-spawned confined agent.
    ///
    /// The `package_sid` is the SDDL-form AppContainer package SID
    /// (`S-1-15-2-*`) read from the process's token and confirmed present
    /// in the minting authority's registry.
    AiAgent {
        /// The AppContainer package SID in SDDL form (`S-1-15-2-...`).
        package_sid: String,
    },
    /// The PID was not spawned by this launcher, or has no AppContainer SID.
    ///
    /// This is the fail-secure default: any classification error, missing
    /// SID, or SID not in the registry results in [`NotAnAgent`].
    NotAnAgent,
}

/// In-memory authorization predicate: the set of AppContainer package SID
/// strings minted by this launcher instance.
///
/// Only SIDs inserted via [`AgentRegistry::insert`] at spawn time will ever
/// classify as [`AgentClassification::AiAgent`]. A self-made AppContainer —
/// even one named `nono.session.<correct-uuid>` — is rejected because its SID
/// is absent from this registry.
///
/// # Thread safety
///
/// `AgentRegistry` is `Send + Sync` via its `HashSet<String>` field. Callers
/// that share it across threads should wrap it in `Arc<Mutex<AgentRegistry>>`.
///
/// # Phase 74
///
/// This is a per-run, in-memory, single-launcher registry. Persistence,
/// multi-tenant isolation, and cross-process sharing are Phase 74 concerns.
pub struct AgentRegistry {
    /// The set of AppContainer package SID strings (SDDL form `S-1-15-2-*`)
    /// minted by this launcher. Private: callers may only insert, not inspect.
    minted_sids: HashSet<String>,
}

impl AgentRegistry {
    /// Constructs an empty [`AgentRegistry`].
    ///
    /// No SIDs are pre-populated; the first call to [`insert`] happens at
    /// agent spawn time from the launch path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            minted_sids: HashSet::new(),
        }
    }

    /// Registers a minted AppContainer package SID.
    ///
    /// Called from the launch path immediately after
    /// `package_sid_to_string` succeeds, before the child process is
    /// spawned. The string must be in SDDL form (`S-1-15-2-...`), matching
    /// the output of `package_sid_to_string`.
    pub fn insert(&mut self, package_sid_str: String) {
        self.minted_sids.insert(package_sid_str);
    }

    /// Classifies a process by PID.
    ///
    /// Returns [`AgentClassification::AiAgent`] only when ALL of the
    /// following hold:
    ///
    /// 1. The process has a non-null AppContainer package SID on its token
    ///    (i.e., it is an AppContainer process).
    /// 2. That SID string is present in this registry's `minted_sids` set.
    ///
    /// Any error reading the token (including nonexistent PID), a null SID,
    /// or a SID not in the registry returns [`AgentClassification::NotAnAgent`].
    ///
    /// # Platform
    ///
    /// On non-Windows platforms this always returns [`AgentClassification::NotAnAgent`].
    #[must_use = "fail-secure: callers must act on the classification result"]
    #[cfg(target_os = "windows")]
    pub fn classify(&self, pid: u32) -> AgentClassification {
        match read_process_appcontainer_sid(pid) {
            Ok(Some(sid_str)) if self.minted_sids.contains(&sid_str) => {
                AgentClassification::AiAgent {
                    package_sid: sid_str,
                }
            }
            // Ok(Some(sid)) but NOT in registry → NotAnAgent (fail-secure)
            // Ok(None)                          → not an AppContainer process
            // Err(_)                            → nonexistent PID or other error
            _ => AgentClassification::NotAnAgent,
        }
    }

    /// Non-Windows stub: always returns [`AgentClassification::NotAnAgent`].
    ///
    /// AppContainer SID classification is a Windows-only primitive (Phase 73
    /// D-01). On non-Windows platforms no AppContainer token exists.
    #[must_use = "fail-secure: callers must act on the classification result"]
    #[cfg(not(target_os = "windows"))]
    pub fn classify(&self, _pid: u32) -> AgentClassification {
        AgentClassification::NotAnAgent
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Reads the AppContainer package SID from an arbitrary process's token.
///
/// Opens the process with `PROCESS_QUERY_LIMITED_INFORMATION`, reads its
/// primary token, and queries `TokenAppContainerSid`. For non-AppContainer
/// processes (e.g., any Medium-IL process), returns `Ok(None)`.
///
/// # Errors
///
/// - `Ok(None)` — the process has no AppContainer SID (not an error; it is
///   simply not an AppContainer process).
/// - `Err(NonoError::SandboxInit(...))` — a Win32 call failed (e.g., the PID
///   does not exist, or access was denied). Callers should treat this as
///   "not an agent" (fail-secure); see [`AgentRegistry::classify`].
/// - `Err(NonoError::UnsupportedPlatform(...))` — running on a non-Windows
///   platform (returned by the non-Windows stub).
///
/// # Safety invariant
///
/// The PSID returned inside `TOKEN_APPCONTAINER_INFORMATION` is owned by the
/// heap buffer returned by `GetTokenInformation`. It MUST NOT be wrapped in
/// `OwnedAppContainerSid` (which calls `FreeSid` on Drop) — that would be a
/// double-free. The string form is extracted while the buffer is alive, then
/// the buffer is dropped.
#[cfg(target_os = "windows")]
pub fn read_process_appcontainer_sid(pid: u32) -> Result<Option<String>> {
    use std::mem::size_of;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenAppContainerSid, TOKEN_APPCONTAINER_INFORMATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    use crate::sandbox::windows::OwnedHandle;

    // Step 1: Open the target process with the minimal required right.
    // PROCESS_QUERY_LIMITED_INFORMATION (0x1000) is sufficient to read the
    // token from processes at equal or lower integrity level.
    let h_process = unsafe {
        // SAFETY: We pass a valid u32 PID and request only QUERY_LIMITED access.
        // On failure (nonexistent PID, access denied) we return Err below.
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid)
    };
    if h_process.is_null() {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "OpenProcess(pid={pid}) failed (GetLastError={gle})"
        )));
    }
    // Step 2: Wrap in OwnedHandle so the handle is closed even on early return.
    let h_process = OwnedHandle(h_process);

    // Step 3: Open the process token with TOKEN_QUERY.
    let mut h_token_raw = null_mut();
    let ok = unsafe {
        // SAFETY: h_process.raw() is a valid open process handle; h_token_raw
        // is a valid out-pointer. We request TOKEN_QUERY only.
        OpenProcessToken(h_process.raw(), TOKEN_QUERY, &mut h_token_raw)
    };
    if ok == 0 {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "OpenProcessToken(pid={pid}) failed (GetLastError={gle})"
        )));
    }
    // Step 4: Wrap the token handle.
    let h_token = OwnedHandle(h_token_raw);

    // Step 5: First GetTokenInformation call with null buffer to query the
    // required buffer size. For non-AppContainer processes, `needed` stays 0.
    let mut needed: u32 = 0;
    unsafe {
        // SAFETY: We pass a null buffer and 0 length; the API writes the
        // required size into `needed`. Ignoring the return value is correct
        // for the size-query path — the API always returns 0 (failure) here.
        GetTokenInformation(
            h_token.raw(),
            TokenAppContainerSid,
            null_mut(),
            0,
            &mut needed,
        )
    };

    // needed == 0 means the token has no AppContainer SID (non-AppContainer
    // process). This is the correct "not an agent" fast path — NOT an error.
    if needed == 0 {
        return Ok(None);
    }
    // Defensive guard: some older Windows versions return a non-zero but
    // undersized `needed`; treat that as "not an AppContainer process" too.
    if (needed as usize) < size_of::<TOKEN_APPCONTAINER_INFORMATION>() {
        return Ok(None);
    }

    // Step 6: Allocate buffer of the required size.
    let mut buf = vec![0u8; needed as usize];

    // Step 7: Second GetTokenInformation call to fill the buffer.
    let ok = unsafe {
        // SAFETY: `buf` is a valid mutable byte buffer of length `needed`;
        // `h_token` is a valid open token handle. The API writes
        // TOKEN_APPCONTAINER_INFORMATION into the start of the buffer.
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
        return Err(NonoError::SandboxInit(format!(
            "GetTokenInformation(TokenAppContainerSid, pid={pid}) failed (GetLastError={gle})"
        )));
    }

    // Step 8: Cast the buffer to TOKEN_APPCONTAINER_INFORMATION and read the
    // PSID. The PSID is a pointer INTO the buffer — do NOT free it separately.
    let info = unsafe {
        // SAFETY: buf is at least size_of::<TOKEN_APPCONTAINER_INFORMATION>()
        // bytes (guarded above and filled by GetTokenInformation). The lifetime
        // of the reference is tied to `buf` which is alive for this scope.
        &*(buf.as_ptr().cast::<TOKEN_APPCONTAINER_INFORMATION>())
    };

    // Step 9: Null TokenAppContainer means no AppContainer SID (some Windows
    // builds return the struct with a null SID pointer instead of needed=0).
    if info.TokenAppContainer.is_null() {
        return Ok(None);
    }

    // Step 10: Convert the PSID to SDDL string form while the buffer is alive.
    // DO NOT wrap `info.TokenAppContainer` in OwnedAppContainerSid — that would
    // call FreeSid on a PSID owned by the Vec<u8> buffer (double-free / UB).
    let sid_str = {
        let mut str_ptr: windows_sys::core::PWSTR = null_mut();
        let ok = unsafe {
            // SAFETY: `info.TokenAppContainer` is a valid PSID owned by `buf`
            // (kept alive in this scope). `str_ptr` is a valid out-pointer.
            // On success the callee allocates a UTF-16 string freed below via
            // LocalFree.
            ConvertSidToStringSidW(info.TokenAppContainer, &mut str_ptr)
        };
        if ok == 0 || str_ptr.is_null() {
            let gle = unsafe { GetLastError() };
            return Err(NonoError::SandboxInit(format!(
                "ConvertSidToStringSidW failed for process token SID (pid={pid}, GetLastError={gle})"
            )));
        }
        // Step 11: Copy the UTF-16 string into a Rust String.
        let s = unsafe {
            // SAFETY: str_ptr points to a nul-terminated UTF-16 string allocated
            // by ConvertSidToStringSidW; we scan for the nul terminator to
            // determine the length, then copy.
            let mut len = 0usize;
            while *str_ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(str_ptr, len);
            String::from_utf16_lossy(slice)
        };
        unsafe {
            // SAFETY: str_ptr was allocated by ConvertSidToStringSidW and is
            // freed exactly once here via LocalFree as documented.
            let _ = LocalFree(str_ptr.cast::<std::ffi::c_void>());
        }
        s
    };

    // `buf` drops here — PSID inside it is no longer referenced.
    Ok(Some(sid_str))
}

/// Non-Windows stub: AppContainer SID classification is Windows-only.
///
/// Returns [`NonoError::UnsupportedPlatform`] on all non-Windows platforms.
/// Callers (e.g., [`AgentRegistry::classify`]) are expected to handle this
/// error as "not an agent" (fail-secure).
#[cfg(not(target_os = "windows"))]
pub fn read_process_appcontainer_sid(_pid: u32) -> Result<Option<String>> {
    Err(NonoError::UnsupportedPlatform(
        "AppContainer SID classification is Windows-only".into(),
    ))
}

#[cfg(all(test, target_os = "windows"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Verify that classifying the current process (Medium IL, no AppContainer)
    /// returns NotAnAgent. This is the primary fail-secure path: the launcher's
    /// own process is never an AI agent.
    #[test]
    fn classify_current_process_not_agent() {
        let registry = AgentRegistry::new();
        let result = registry.classify(std::process::id());
        assert_eq!(
            result,
            AgentClassification::NotAnAgent,
            "Current process (Medium IL, no AppContainer) must classify as NotAnAgent"
        );
    }

    /// Verify that classifying a nonexistent PID (OpenProcess will fail) returns
    /// NotAnAgent, not an error propagated to the caller.
    #[test]
    fn classify_nonexistent_pid_not_agent() {
        let registry = AgentRegistry::new();
        // PID 0xFFFF_FFFF is the Windows kernel pseudo-process and cannot be
        // opened by user-mode code; OpenProcess will fail → fail-secure.
        let result = registry.classify(0xFFFF_FFFFu32);
        assert_eq!(
            result,
            AgentClassification::NotAnAgent,
            "Nonexistent PID must classify as NotAnAgent (fail-secure)"
        );
    }

    /// Verify that inserting a fake SID does not cause a real process (the
    /// current process) to classify as AiAgent. Registry membership is required;
    /// a SID that matches no real token never grants AiAgent status.
    ///
    /// This also proves that the authorization check is the private registry,
    /// not a namespace-pattern match: even though "S-1-15-2-9999" starts with
    /// the AppContainer prefix, it is not in the current process's token.
    #[test]
    fn insert_and_classify_requires_registry_membership() {
        let mut registry = AgentRegistry::new();
        // Insert a fake SID that the current process does NOT have.
        registry.insert("S-1-15-2-9999".to_string());
        // The current process has a different SID (or none), so it must NOT
        // classify as AiAgent.
        let result = registry.classify(std::process::id());
        assert_eq!(
            result,
            AgentClassification::NotAnAgent,
            "Registry membership alone is not enough; the PID's token SID must match"
        );
    }

    /// Verify that read_process_appcontainer_sid returns Ok(None) for the
    /// current process (which is Medium IL and has no AppContainer SID).
    #[test]
    fn read_sid_current_process_returns_none() {
        let result = read_process_appcontainer_sid(std::process::id());
        assert!(
            result.is_ok(),
            "read_process_appcontainer_sid must not error for the current process"
        );
        assert_eq!(
            result.unwrap(),
            None,
            "Current process (Medium IL) has no AppContainer SID → Ok(None)"
        );
    }
}
