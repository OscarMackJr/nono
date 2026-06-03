//! THROWAWAY SPIKE (Phase 62) — answers ONE empirical question:
//!
//! **Does a WFP filter keyed on an AppContainer package SID actually BLOCK that
//! AppContainer process's outbound TCP connection?**
//!
//! It tests BOTH approaches:
//!   - TEST A — `FWPM_CONDITION_ALE_USER_ID` with a security descriptor
//!     `D:(A;;CC;;;<packageSid>)` (what nono currently builds; the access-check
//!     path). The package SID is a normal access-check participant on the lowbox
//!     token, so an SD ACE granting it `CC` *should* match — this spike confirms.
//!   - TEST B — `FWPM_CONDITION_ALE_PACKAGE_ID` with the raw package SID as an
//!     `FWP_SID` value (the purpose-built condition that matches the connection's
//!     AppContainer package SID directly via `FWP_MATCH_EQUAL`).
//!
//! For each test the spike installs an OUTBOUND BLOCK filter
//! (`FWPM_LAYER_ALE_AUTH_CONNECT_V4`, `FWP_ACTION_BLOCK`), spawns `curl.exe` as
//! an AppContainer (`SECURITY_CAPABILITIES { AppContainerSid, CapabilityCount: 0 }`)
//! from `C:\Windows\System32` (which grants ALL APPLICATION PACKAGES
//! read+execute+traverse, so the lowbox child starts cleanly and the cwd-access
//! problem is sidestepped), then inspects curl's output/exit code to decide
//! BLOCKED vs NOT BLOCKED, and removes the filter.
//!
//! This is an EXAMPLE (clearly throwaway, outside the shipped bins). It modifies
//! NO production code. It REQUIRES ELEVATION (WFP `FwpmFilterAdd0` needs admin).
//! Build-only verification is possible without admin; running needs an elevated
//! shell.
//!
//! Run (from an ELEVATED PowerShell/cmd):
//!   cargo run -p nono-cli --example spike_wfp_appcontainer
//! or the built exe:
//!   target\debug\examples\spike_wfp_appcontainer.exe
//!
//! It replicates the proven nono-wfp-service.rs sequence exactly, INCLUDING the
//! two fixes that otherwise make `FwpmFilterAdd0` fail RPC_X_BAD_STUB_DATA (1783):
//!   - 62-07: a NON-NULL `displayData.name` on the filter.
//!   - 62-08: the SD wrapped in an `FWP_BYTE_BLOB` (not a raw PSECURITY_DESCRIPTOR).

// Non-Windows stub so the crate still compiles everywhere (examples/ is
// auto-discovered by Cargo).
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("spike_wfp_appcontainer is Windows-only.");
}

#[cfg(target_os = "windows")]
fn main() -> std::process::ExitCode {
    windows_spike::run()
}

#[cfg(target_os = "windows")]
mod windows_spike {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::process::ExitCode;
    use std::ptr::{null, null_mut};

    use windows_sys::core::GUID;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, LocalFree, FWP_E_ALREADY_EXISTS, FWP_E_FILTER_NOT_FOUND,
        FWP_E_SUBLAYER_NOT_FOUND, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::{
        FwpmEngineClose0, FwpmEngineOpen0, FwpmFilterAdd0, FwpmFilterDeleteByKey0,
        FwpmSubLayerAdd0, FwpmSubLayerDeleteByKey0, FwpmTransactionAbort0, FwpmTransactionBegin0,
        FwpmTransactionCommit0, FWPM_ACTION0, FWPM_ACTION0_0, FWPM_CONDITION_ALE_PACKAGE_ID,
        FWPM_CONDITION_ALE_USER_ID, FWPM_DISPLAY_DATA0, FWPM_FILTER0, FWPM_FILTER0_0,
        FWPM_FILTER_CONDITION0, FWPM_LAYER_ALE_AUTH_CONNECT_V4, FWPM_SESSION0, FWPM_SUBLAYER0,
        FWP_ACTION_BLOCK, FWP_BYTE_BLOB, FWP_CONDITION_VALUE0, FWP_CONDITION_VALUE0_0,
        FWP_MATCH_EQUAL, FWP_SECURITY_DESCRIPTOR_TYPE, FWP_SID, FWP_UINT64, FWP_VALUE0,
        FWP_VALUE0_0,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        FreeSid, GetSecurityDescriptorLength, SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES, SID,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, CREATE_ALWAYS, FILE_ATTRIBUTE_TEMPORARY, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    use windows_sys::Win32::System::Rpc::RPC_C_AUTHN_WINNT;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
        InitializeProcThreadAttributeList, UpdateProcThreadAttribute, WaitForSingleObject,
        EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTF_USESTDHANDLES, STARTUPINFOEXW,
        STARTUPINFOW,
    };

    // userenv.dll AppContainer profile APIs. A derived package SID is NOT enough
    // for CreateProcess(SECURITY_CAPABILITIES) — the AppContainer PROFILE (registry
    // + AppContainerNamedObjects namespace) must be REGISTERED first, else
    // CreateProcessW fails ERROR_FILE_NOT_FOUND. Declared as a local extern shim so
    // this throwaway spike needs no production Cargo.toml feature change.
    #[link(name = "userenv")]
    extern "system" {
        fn CreateAppContainerProfile(
            pszAppContainerName: *const u16,
            pszDisplayName: *const u16,
            pszDescription: *const u16,
            pCapabilities: *const core::ffi::c_void,
            dwCapabilityCount: u32,
            ppSidAppContainerSid: *mut *mut core::ffi::c_void,
        ) -> i32;
        fn DeleteAppContainerProfile(pszAppContainerName: *const u16) -> i32;
    }

    fn to_wide_nul(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    /// HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS).
    const HR_ALREADY_EXISTS: i32 = 0x8007_0050u32 as i32;

    /// Register the spike AppContainer profile so the package SID names a real
    /// lowbox the kernel can launch into. Tolerates ERROR_ALREADY_EXISTS (a prior
    /// run left it registered; the SID is deterministic from the name).
    fn register_app_container_profile() -> Result<(), String> {
        let name = to_wide_nul(APP_CONTAINER_NAME);
        let display = to_wide_nul("nono spike");
        let desc = to_wide_nul("nono WFP AppContainer spike (throwaway)");
        let mut sid: *mut core::ffi::c_void = null_mut();
        // SAFETY: name/display/desc are nul-terminated UTF-16; no capabilities;
        // sid is a valid out-pointer freed below.
        let hr = unsafe {
            CreateAppContainerProfile(name.as_ptr(), display.as_ptr(), desc.as_ptr(), null(), 0, &mut sid)
        };
        if hr == 0 {
            if !sid.is_null() {
                // SAFETY: sid was allocated by CreateAppContainerProfile; free per its contract.
                unsafe { FreeSid(sid) };
            }
            Ok(())
        } else if hr == HR_ALREADY_EXISTS {
            Ok(())
        } else {
            Err(format!(
                "CreateAppContainerProfile failed (HRESULT=0x{:08X})",
                hr as u32
            ))
        }
    }

    /// Best-effort: delete the spike AppContainer profile registered above.
    fn unregister_app_container_profile() {
        let name = to_wide_nul(APP_CONTAINER_NAME);
        // SAFETY: nul-terminated UTF-16 name; best-effort cleanup, result ignored.
        unsafe {
            let _ = DeleteAppContainerProfile(name.as_ptr());
        }
    }

    /// Fixed per-spike AppContainer moniker. Deterministic → stable package SID.
    const APP_CONTAINER_NAME: &str = "nono.spike.wfptest";

    /// A fresh, spike-only sublayer GUID (distinct from the production
    /// NONO_SUBLAYER_GUID so the spike never collides with shipped filters).
    const SPIKE_SUBLAYER_GUID: GUID = GUID::from_u128(0x5f9e7c10_3b2a_4d8e_9a16_c0ffee5b1a5e);

    /// Deterministic filter keys for the two tests (stable so cleanup can
    /// delete-by-key even across runs).
    const FILTER_KEY_USER_ID: GUID = GUID::from_u128(0x5f9e7c11_3b2a_4d8e_9a16_c0ffee5e1d00);
    const FILTER_KEY_PACKAGE_ID: GUID = GUID::from_u128(0x5f9e7c12_3b2a_4d8e_9a16_c0ffee9c0de0);

    fn to_utf16_null(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain([0]).collect()
    }

    fn zeroed<T>() -> T {
        // SAFETY: every struct constructed this way is plain-old-data
        // (WFP/STARTUPINFO POD) and is fully initialized field-by-field before
        // being handed to a Win32 API.
        unsafe { std::mem::zeroed() }
    }

    fn fmt_err(status: u32, ctx: &str) -> String {
        format!("{ctx} (win32 status {status}, 0x{status:08x})")
    }

    // -- RAII guards (mirror nono-wfp-service.rs) ---------------------------

    struct WfpEngine(HANDLE);
    impl Drop for WfpEngine {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: handle returned by FwpmEngineOpen0; closed exactly once.
                unsafe {
                    let _ = FwpmEngineClose0(self.0);
                }
            }
        }
    }

    struct WfpSecurityDescriptor(*mut core::ffi::c_void);
    impl WfpSecurityDescriptor {
        fn as_ptr(&self) -> *mut core::ffi::c_void {
            self.0
        }
    }
    impl Drop for WfpSecurityDescriptor {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW;
                // released via LocalFree exactly once.
                unsafe {
                    LocalFree(self.0 as _);
                }
            }
        }
    }

    struct OwnedFileHandle(HANDLE);
    impl Drop for OwnedFileHandle {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                // SAFETY: file handle opened by CreateFileW; closed exactly once.
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }
    }

    fn open_engine() -> Result<WfpEngine, String> {
        let mut session: FWPM_SESSION0 = zeroed();
        // Persistent (flags = 0): objects are shared across engine handles, the
        // same reason nono-wfp-service.rs uses a persistent session.
        session.flags = 0;
        let mut handle: HANDLE = null_mut();
        // SAFETY: all pointers null or valid POD; handle wrapped immediately.
        let status =
            unsafe { FwpmEngineOpen0(null(), RPC_C_AUTHN_WINNT, null(), &session, &mut handle) };
        if status != 0 {
            return Err(fmt_err(status, "FwpmEngineOpen0 failed"));
        }
        if handle.is_null() {
            return Err("FwpmEngineOpen0 returned a null handle".to_string());
        }
        Ok(WfpEngine(handle))
    }

    fn create_sublayer(engine: &WfpEngine) -> Result<(), String> {
        let mut sub: FWPM_SUBLAYER0 = zeroed();
        sub.subLayerKey = SPIKE_SUBLAYER_GUID;
        let name = to_utf16_null(OsStr::new("nono WFP AppContainer Spike Sublayer"));
        sub.displayData = FWPM_DISPLAY_DATA0 {
            name: name.as_ptr() as *mut _,
            description: null_mut(),
        };
        sub.weight = 0x1000;
        // SAFETY: engine valid; sub points to initialized POD that outlives the call.
        let status = unsafe { FwpmSubLayerAdd0(engine.0, &sub, null_mut()) };
        if status != 0 && status != FWP_E_ALREADY_EXISTS as u32 {
            return Err(fmt_err(status, "FwpmSubLayerAdd0 failed"));
        }
        Ok(())
    }

    fn delete_sublayer(engine: &WfpEngine) {
        // SAFETY: engine valid; key points to an initialized GUID.
        let status = unsafe { FwpmSubLayerDeleteByKey0(engine.0, &SPIKE_SUBLAYER_GUID) };
        if status != 0 && status != FWP_E_SUBLAYER_NOT_FOUND as u32 {
            eprintln!("[cleanup] FwpmSubLayerDeleteByKey0: {}", fmt_err(status, ""));
        }
    }

    /// Build the SD `D:(A;;CC;;;<sid>)` (62-08: wrapped later in an FWP_BYTE_BLOB).
    fn sid_to_security_descriptor(sid_str: &str) -> Result<WfpSecurityDescriptor, String> {
        let sddl = format!("D:(A;;CC;;;{sid_str})");
        let sddl_wide = to_utf16_null(OsStr::new(&sddl));
        let mut sd: *mut core::ffi::c_void = null_mut();
        // SAFETY: sddl_wide is a valid nul-terminated UTF-16 buffer; sd receives
        // the LocalAlloc'd self-relative SD.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl_wide.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(fmt_err(
                unsafe { GetLastError() },
                &format!("ConvertStringSecurityDescriptorToSecurityDescriptorW('{sddl}')"),
            ));
        }
        Ok(WfpSecurityDescriptor(sd))
    }

    /// Which condition the BLOCK filter keys on.
    enum Condition {
        /// ALE_USER_ID == SD D:(A;;CC;;;<packageSid>) wrapped in FWP_BYTE_BLOB.
        UserIdSd(*mut core::ffi::c_void),
        /// ALE_PACKAGE_ID == the package SID as an FWP_SID (raw SID bytes).
        PackageIdSid(*mut SID),
    }

    /// Install ONE outbound BLOCK filter at FWPM_LAYER_ALE_AUTH_CONNECT_V4 with
    /// the given condition. Replicates nono-wfp-service.rs add_policy_filter,
    /// INCLUDING the 62-07 non-null displayData.name and the 62-08 SD→FWP_BYTE_BLOB.
    fn add_block_filter(
        engine: &WfpEngine,
        key: GUID,
        condition: Condition,
    ) -> Result<(), String> {
        // sd_blob / sid_value must outlive FwpmFilterAdd0 (declared before
        // `conditions`, which borrow them).
        let mut sd_blob = FWP_BYTE_BLOB {
            size: 0,
            data: null_mut(),
        };

        let mut conditions: Vec<FWPM_FILTER_CONDITION0> = Vec::with_capacity(1);
        match condition {
            Condition::UserIdSd(sd) => {
                // SAFETY: sd is a valid self-relative SD; GetSecurityDescriptorLength
                // reads its header to size the FWP_BYTE_BLOB. sd outlives this call.
                sd_blob.size = unsafe { GetSecurityDescriptorLength(sd) };
                sd_blob.data = sd as *mut u8;
                conditions.push(FWPM_FILTER_CONDITION0 {
                    fieldKey: FWPM_CONDITION_ALE_USER_ID,
                    matchType: FWP_MATCH_EQUAL,
                    conditionValue: FWP_CONDITION_VALUE0 {
                        r#type: FWP_SECURITY_DESCRIPTOR_TYPE,
                        Anonymous: FWP_CONDITION_VALUE0_0 { sd: &mut sd_blob },
                    },
                });
            }
            Condition::PackageIdSid(sid) => {
                conditions.push(FWPM_FILTER_CONDITION0 {
                    fieldKey: FWPM_CONDITION_ALE_PACKAGE_ID,
                    matchType: FWP_MATCH_EQUAL,
                    conditionValue: FWP_CONDITION_VALUE0 {
                        r#type: FWP_SID,
                        // The raw package SID bytes (the OwnedAppContainerSid keeps
                        // them alive for the duration of this call).
                        Anonymous: FWP_CONDITION_VALUE0_0 { sid },
                    },
                });
            }
        }

        let action = FWPM_ACTION0 {
            r#type: FWP_ACTION_BLOCK,
            Anonymous: FWPM_ACTION0_0 {
                filterType: zero_guid(),
            },
        };

        let mut weight_value: u64 = 0; // BLOCK weight (mirrors nono-wfp-service SID-path block=0).

        // 62-07: displayData.name MUST be non-null or FwpmFilterAdd0 → 1783.
        let name = to_utf16_null(OsStr::new("nono WFP AppContainer Spike Block Filter"));
        let mut filter: FWPM_FILTER0 = zeroed();
        filter.filterKey = key;
        filter.displayData = FWPM_DISPLAY_DATA0 {
            name: name.as_ptr() as *mut _,
            description: null_mut(),
        };
        filter.layerKey = FWPM_LAYER_ALE_AUTH_CONNECT_V4;
        filter.subLayerKey = SPIKE_SUBLAYER_GUID;
        filter.weight = FWP_VALUE0 {
            r#type: FWP_UINT64,
            Anonymous: FWP_VALUE0_0 {
                uint64: &mut weight_value,
            },
        };
        filter.numFilterConditions = conditions.len() as u32;
        filter.filterCondition = conditions.as_mut_ptr();
        filter.action = action;
        filter.Anonymous = FWPM_FILTER0_0 { rawContext: 0 };

        let mut filter_id = 0u64;
        // SAFETY: engine valid; filter + its nested pointers (sd_blob/sid/name/
        // conditions) all outlive this call.
        let status = unsafe { FwpmFilterAdd0(engine.0, &filter, null_mut(), &mut filter_id) };
        if status != 0 {
            return Err(fmt_err(status, "FwpmFilterAdd0 failed"));
        }
        Ok(())
    }

    fn delete_filter(engine: &WfpEngine, key: GUID) {
        // SAFETY: engine valid; key points to an initialized GUID.
        let status = unsafe { FwpmFilterDeleteByKey0(engine.0, &key) };
        if status != 0 && status != FWP_E_FILTER_NOT_FOUND as u32 {
            eprintln!("[cleanup] FwpmFilterDeleteByKey0: {}", fmt_err(status, ""));
        }
    }

    fn zero_guid() -> GUID {
        GUID::from_u128(0)
    }

    /// Run a transactional install of one BLOCK filter, then probe with an
    /// AppContainer curl, then remove the filter. Returns the probe verdict.
    fn run_test(
        engine: &WfpEngine,
        label: &str,
        filter_key: GUID,
        condition: Condition,
        package_sid_psid: *mut core::ffi::c_void,
    ) -> Probe {
        // Install.
        if let Err(e) = transactional(engine, |eng| add_block_filter(eng, filter_key, condition)) {
            return Probe::error(format!("filter install failed: {e}"));
        }

        // Probe: spawn curl as the AppContainer and capture output/exit.
        let probe = spawn_appcontainer_curl(package_sid_psid);

        // Remove (best-effort; logged on error).
        let _ = transactional(engine, |eng| {
            delete_filter(eng, filter_key);
            Ok(())
        });

        let _ = label;
        probe
    }

    /// Helper: wrap a closure in a WFP transaction (begin/commit, abort on err).
    fn transactional<F>(engine: &WfpEngine, f: F) -> Result<(), String>
    where
        F: FnOnce(&WfpEngine) -> Result<(), String>,
    {
        // SAFETY: engine valid.
        let status = unsafe { FwpmTransactionBegin0(engine.0, 0) };
        if status != 0 {
            return Err(fmt_err(status, "FwpmTransactionBegin0 failed"));
        }
        match f(engine) {
            Ok(()) => {
                // SAFETY: engine valid; active transaction.
                let status = unsafe { FwpmTransactionCommit0(engine.0) };
                if status != 0 {
                    return Err(fmt_err(status, "FwpmTransactionCommit0 failed"));
                }
                Ok(())
            }
            Err(e) => {
                // SAFETY: engine valid; aborting the uncommitted transaction.
                unsafe {
                    let _ = FwpmTransactionAbort0(engine.0);
                }
                Err(e)
            }
        }
    }

    // -- Probe (spawn curl as an AppContainer) ------------------------------

    struct Probe {
        exit_code: Option<u32>,
        output: String,
        /// None = could not even spawn / no decision; Some(true) = BLOCKED.
        blocked: Option<bool>,
        note: Option<String>,
    }

    impl Probe {
        fn error(msg: String) -> Self {
            Probe {
                exit_code: None,
                output: String::new(),
                blocked: None,
                note: Some(msg),
            }
        }

        fn verdict_str(&self) -> &'static str {
            match self.blocked {
                Some(true) => "BLOCKED",
                Some(false) => "NOT BLOCKED",
                None => "INDETERMINATE",
            }
        }
    }

    /// Open a temp file that the (inheritable) child can write its stdout/stderr to.
    fn open_inheritable_temp(path: &std::path::Path) -> Result<OwnedFileHandle, String> {
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: 1, // inheritable so the child receives it via STARTF_USESTDHANDLES
        };
        let wide = to_utf16_null(path.as_os_str());
        // SAFETY: wide is a valid nul-terminated path; sa is a valid SECURITY_ATTRIBUTES.
        let h = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                &sa,
                CREATE_ALWAYS,
                FILE_ATTRIBUTE_TEMPORARY,
                null_mut(),
            )
        };
        if h == INVALID_HANDLE_VALUE || h.is_null() {
            return Err(fmt_err(
                unsafe { GetLastError() },
                &format!("CreateFileW({}) failed", path.display()),
            ));
        }
        Ok(OwnedFileHandle(h))
    }

    /// Spawn `curl.exe -sS -m 5 https://api.ipify.org` as an AppContainer
    /// (SECURITY_CAPABILITIES{packageSid, 0}) from C:\Windows\System32, capturing
    /// stdout+stderr to a temp file, and decide BLOCKED vs NOT BLOCKED.
    fn spawn_appcontainer_curl(package_sid_psid: *mut core::ffi::c_void) -> Probe {
        // Temp file for combined stdout+stderr.
        let mut out_path = std::env::temp_dir();
        out_path.push(format!("nono-spike-curl-{}.txt", std::process::id()));

        let out_file = match open_inheritable_temp(&out_path) {
            Ok(f) => f,
            Err(e) => return Probe::error(e),
        };

        // Attribute list with 1 slot: SECURITY_CAPABILITIES.
        let mut attr_size: usize = 0;
        // SAFETY: probe call with null list returns the required size (documented idiom).
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut attr_size);
        }
        let mut attr_buf = vec![0u8; attr_size];
        let attr_list: LPPROC_THREAD_ATTRIBUTE_LIST =
            attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
        // SAFETY: attr_list points to attr_buf sized by the probe above for 1 slot.
        let ok = unsafe { InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size) };
        if ok == 0 {
            return Probe::error(fmt_err(
                unsafe { GetLastError() },
                "InitializeProcThreadAttributeList failed",
            ));
        }

        // Empty capability set: the most restrictive lowbox (D1 step 2).
        let caps = SECURITY_CAPABILITIES {
            AppContainerSid: package_sid_psid,
            Capabilities: null_mut(),
            CapabilityCount: 0,
            Reserved: 0,
        };
        // SAFETY: attr_list initialized for 1 slot; caps + its AppContainerSid
        // (owned by the caller's OwnedAppContainerSid) outlive the spawn below.
        let ok = unsafe {
            UpdateProcThreadAttribute(
                attr_list,
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                &caps as *const SECURITY_CAPABILITIES as *mut _,
                std::mem::size_of::<SECURITY_CAPABILITIES>(),
                null_mut(),
                null_mut(),
            )
        };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            // SAFETY: attr_list was initialized above.
            unsafe {
                DeleteProcThreadAttributeList(attr_list);
            }
            return Probe::error(fmt_err(
                err,
                "UpdateProcThreadAttribute(SECURITY_CAPABILITIES) failed",
            ));
        }

        let mut si: STARTUPINFOEXW = zeroed();
        si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = attr_list;
        si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        // Send both stdout and stderr to the same inheritable temp-file handle.
        si.StartupInfo.hStdOutput = out_file.0;
        si.StartupInfo.hStdError = out_file.0;
        si.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;

        // Mutable command line (CreateProcessW may write into it).
        let mut cmdline: Vec<u16> = to_utf16_null(OsStr::new(
            "curl.exe -sS -m 5 https://api.ipify.org",
        ));
        let cwd = to_utf16_null(OsStr::new("C:\\Windows\\System32"));

        let mut pi: PROCESS_INFORMATION = zeroed();
        let lp_si = &si.StartupInfo as *const STARTUPINFOW;

        // SAFETY: cmdline/cwd are nul-terminated UTF-16; si carries
        // EXTENDED_STARTUPINFO_PRESENT + the SECURITY_CAPABILITIES attribute.
        // bInheritHandles=1 so the inheritable temp-file handle reaches the child.
        let created = unsafe {
            CreateProcessW(
                null(),
                cmdline.as_mut_ptr(),
                null(),
                null(),
                1, // bInheritHandles
                EXTENDED_STARTUPINFO_PRESENT,
                null(),
                cwd.as_ptr(),
                lp_si,
                &mut pi,
            )
        };

        // SAFETY: attr_list initialized above; no longer needed after spawn.
        unsafe {
            DeleteProcThreadAttributeList(attr_list);
        }

        if created == 0 {
            let err = unsafe { GetLastError() };
            return Probe::error(format!(
                "CreateProcessW (AppContainer) failed (GetLastError={err})"
            ));
        }

        // Wait up to ~8s.
        // SAFETY: pi.hProcess is a valid process handle from CreateProcessW.
        unsafe {
            WaitForSingleObject(pi.hProcess, 8000);
        }
        let mut exit: u32 = 0;
        // SAFETY: pi.hProcess valid; &mut exit is writable.
        let got = unsafe { GetExitCodeProcess(pi.hProcess, &mut exit) };
        let exit_code = if got != 0 { Some(exit) } else { None };

        // Close process/thread handles.
        // SAFETY: both handles valid from CreateProcessW; closed exactly once.
        unsafe {
            let _ = CloseHandle(pi.hThread);
            let _ = CloseHandle(pi.hProcess);
        }

        // Drop the file handle so the read below sees a flushed, unlocked file.
        drop(out_file);

        let output = std::fs::read_to_string(&out_path).unwrap_or_default();
        let _ = std::fs::remove_file(&out_path);

        // Decision: NOT BLOCKED iff curl printed an IPv4 address (and exited 0).
        let has_ipv4 = output.split_whitespace().any(looks_like_ipv4);
        let blocked = if has_ipv4 && exit_code == Some(0) {
            Some(false)
        } else {
            // Non-zero exit / timeout / no IP → treat as blocked. (curl exit 28 =
            // timeout, 7 = connection refused, etc. — all consistent with a WFP block.)
            Some(true)
        };

        let note = if exit_code.is_none() {
            Some("child still running / exit code unavailable after 8s".to_string())
        } else {
            None
        };

        Probe {
            exit_code,
            output: output.trim().to_string(),
            blocked,
            note,
        }
    }

    fn looks_like_ipv4(token: &str) -> bool {
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 4
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u8>().is_ok())
    }

    pub fn run() -> ExitCode {
        println!("=== nono WFP AppContainer SPIKE (Phase 62) ===");
        println!("AppContainer name : {APP_CONTAINER_NAME}");

        // 0) REGISTER the AppContainer profile (the fix under test): without a
        //    registered profile, CreateProcess(SECURITY_CAPABILITIES) fails
        //    ERROR_FILE_NOT_FOUND even with a fully-accessible cwd + exe.
        match register_app_container_profile() {
            Ok(()) => println!("AppContainer profile registered (or already existed)."),
            Err(e) => {
                eprintln!("FATAL: {e}");
                eprintln!("(CreateAppContainerProfile needs an ELEVATED process. Re-run from an admin shell.)");
                return ExitCode::from(2);
            }
        }

        // 1) Derive the per-spike package SID (PSID + string form).
        let package_sid = match nono::derive_app_container_sid(APP_CONTAINER_NAME) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("FATAL: derive_app_container_sid failed: {e}");
                return ExitCode::from(2);
            }
        };
        let package_sid_str = match nono::package_sid_to_string(&package_sid) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("FATAL: package_sid_to_string failed: {e}");
                return ExitCode::from(2);
            }
        };
        println!("Package SID       : {package_sid_str}");
        println!();

        // 2) Open the engine + create the spike sublayer.
        let engine = match open_engine() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("FATAL: {e}");
                eprintln!("(WFP requires an ELEVATED process. Re-run from an admin shell.)");
                return ExitCode::from(2);
            }
        };
        if let Err(e) = create_sublayer(&engine) {
            eprintln!("FATAL: {e}");
            return ExitCode::from(2);
        }

        // 3) TEST A — ALE_USER_ID == SD D:(A;;CC;;;<packageSid>).
        let probe_a = match sid_to_security_descriptor(&package_sid_str) {
            Ok(sd) => {
                // sd must outlive run_test (its FWP_BYTE_BLOB points into it).
                let p = run_test(
                    &engine,
                    "ALE_USER_ID",
                    FILTER_KEY_USER_ID,
                    Condition::UserIdSd(sd.as_ptr()),
                    package_sid.as_psid(),
                );
                drop(sd);
                p
            }
            Err(e) => Probe::error(format!("SD build failed: {e}")),
        };

        // 4) TEST B — ALE_PACKAGE_ID == package SID (FWP_SID).
        let probe_b = run_test(
            &engine,
            "ALE_PACKAGE_ID",
            FILTER_KEY_PACKAGE_ID,
            Condition::PackageIdSid(package_sid.as_psid() as *mut SID),
            package_sid.as_psid(),
        );

        // 5) Tear down the sublayer (best-effort).
        let _ = transactional(&engine, |eng| {
            delete_filter(eng, FILTER_KEY_USER_ID);
            delete_filter(eng, FILTER_KEY_PACKAGE_ID);
            Ok(())
        });
        delete_sublayer(&engine);

        // 6) Final verdict.
        println!();
        println!("SPIKE RESULT (package SID {package_sid_str}):");
        print_line("ALE_USER_ID", &probe_a);
        print_line("ALE_PACKAGE_ID", &probe_b);
        println!();
        interpret(&probe_a, &probe_b);

        // 7) Best-effort: unregister the spike AppContainer profile.
        unregister_app_container_profile();

        ExitCode::SUCCESS
    }

    fn print_line(name: &str, p: &Probe) {
        let exit = p
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "n/a".to_string());
        let out = if p.output.is_empty() {
            "<none>".to_string()
        } else {
            // single-line, truncated
            let one = p.output.replace('\n', " ");
            if one.len() > 60 {
                format!("{}…", &one[..60])
            } else {
                one
            }
        };
        print!(
            "  {name:<14} : {:<14} (curl exit={exit}, output: {out})",
            p.verdict_str()
        );
        if let Some(note) = &p.note {
            print!("  [{note}]");
        }
        println!();
    }

    fn interpret(a: &Probe, b: &Probe) {
        println!("Interpretation:");
        match (a.blocked, b.blocked) {
            (Some(true), Some(true)) => println!(
                "  BOTH conditions block an AppContainer connection. nono's existing \
                 ALE_USER_ID(packageSid) path WORKS; ALE_PACKAGE_ID is a valid cleaner alternative."
            ),
            (Some(true), _) => println!(
                "  ALE_USER_ID(packageSid) BLOCKS the AppContainer connection — nono's current \
                 marshaling is sufficient; the F-62-UAT-05 redesign's first increment is viable."
            ),
            (Some(false), Some(true)) => println!(
                "  ALE_USER_ID does NOT match the AppContainer connection, but ALE_PACKAGE_ID DOES. \
                 nono must switch the per-run filter to the FWP_SID ALE_PACKAGE_ID condition (D1 step 4)."
            ),
            (Some(false), Some(false)) => println!(
                "  NEITHER condition blocks the AppContainer connection. The package-SID WFP-match \
                 premise is FALSE — the AppContainer approach does not yield a WFP-enforceable block; \
                 reconsider the design."
            ),
            _ => println!(
                "  INDETERMINATE — at least one probe could not produce a verdict (see notes/errors above). \
                 Re-run elevated; confirm curl.exe is on PATH in C:\\Windows\\System32."
            ),
        }
    }
}
