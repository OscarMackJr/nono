//! `nono-shell-broker` — Phase 31 D-05 broker binary.
//!
//! Medium-IL intermediary spawned by `nono.exe` for the `nono shell` command on
//! Windows. The broker:
//!
//! 1. Inherits a console attachment from `nono.exe` at Medium IL (KernelBase
//!    skips CSRSS attach for already-inherited consoles — RESEARCH A1, validated
//!    by the 2026-05-08 PoC at `.planning/quick/260508-m99-.../`).
//! 2. Constructs a Low-IL primary token via `nono::create_low_integrity_primary_token`
//!    (D-06: single source of truth shared with `nono-cli`).
//! 3. Spawns the actual sandboxed shell child via `CreateProcessAsUserW` with
//!    `dwCreationFlags = EXTENDED_STARTUPINFO_PRESENT` only (D-01: NO new
//!    console flag, NO pseudoconsole proc-thread attribute — child inherits
//!    broker's console without re-triggering CSRSS attach at Low IL).
//! 4. Restricts inherited handles via `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` to
//!    only those passed by `nono.exe` via `--inherit-handle <hex>` (D-02:
//!    capability-pipe and other supervisor handles are NEVER inheritable past
//!    `nono.exe`).
//! 5. Waits for the child via `WaitForSingleObject(INFINITE)` and propagates
//!    the exit code via `std::process::exit(child_exit_code as i32)` (D-03).
//!
//! No JSON parsing surface; argv is the only IPC channel from `nono.exe` (D-08).

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "nono-shell-broker is a Windows-only binary; \
         this build target should not ship it. \
         Phase 31 D-05: cross-compile parity stub."
    );
    std::process::exit(1);
}

#[cfg(windows)]
mod broker {
    use std::ffi::{OsStr, OsString};
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;

    use nono::{NonoError, OwnedHandle, Result as NonoResult};
    use windows_sys::Win32::Foundation::{GetLastError, HANDLE};
    use windows_sys::Win32::Security::SECURITY_CAPABILITIES;
    use windows_sys::Win32::Security::{TOKEN_ADJUST_DEFAULT, TOKEN_QUERY};
    use windows_sys::Win32::System::Console::AllocConsole;
    use windows_sys::Win32::System::Threading::{
        CreateProcessAsUserW, CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
        InitializeProcThreadAttributeList, OpenProcessToken, ResumeThread,
        UpdateProcThreadAttribute, WaitForSingleObject, CREATE_SUSPENDED,
        EXTENDED_STARTUPINFO_PRESENT, INFINITE, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
        PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
        STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW,
    };

    /// D-08: argv-only IPC. CapabilitySet/Profile NOT passed (RESEARCH §3a —
    /// labels applied supervisor-side BEFORE the broker is spawned).
    #[derive(Debug)]
    pub struct BrokerArgs {
        pub shell_path: PathBuf,
        pub shell_args: Vec<String>,
        pub inherit_handles: Vec<HANDLE>,
        pub cwd: PathBuf,
        /// Phase 51 Plan 02: when `true`, the child is spawned with
        /// `STARTF_USESTDHANDLES` binding the three `inherit_handles` as
        /// `hStdInput`/`hStdOutput`/`hStdError` instead of inheriting the
        /// broker's console. Set by the `--no-pty` flag passed from
        /// `nono-cli`'s `BrokerLaunchNoPty` arm. When `false` (default), the
        /// existing PTY/console path is byte-behaviorally unchanged (D-05:
        /// `AllocConsole` console-presence probe is independent of std-handle
        /// wiring and untouched).
        pub no_pty: bool,
        /// Plan 62-12 (F-62-UAT-05 redesign, debug `wfp-write-restricted-0142`):
        /// the per-run AppContainer moniker `nono.session.<uuid>`. The broker
        /// derives the package SID (`S-1-15-2-*`) from this name via
        /// `nono::derive_app_container_sid` and spawns the confined child as a
        /// per-run AppContainer (lowbox) carrying
        /// `SECURITY_CAPABILITIES { AppContainerSid, CapabilityCount: 0 }`.
        /// This starts cleanly (private AppContainerNamedObjects namespace),
        /// eliminating the 0xC0000142 STATUS_DLL_INIT_FAILED that the falsified
        /// 62-10 WRITE_RESTRICTED token caused, AND the same package SID scopes
        /// the WFP ALE_USER_ID filter (single source: nono-cli derives the same
        /// SID from the same name). Present on the `--no-pty` (BrokerLaunchNoPty)
        /// path only; absent on the PTY/legacy path (PTY waives per-session WFP).
        /// FAIL-CLOSED: when `no_pty` is true, `app_container_name` MUST be Some;
        /// parse_args rejects `--no-pty` without `--app-container-name` as a
        /// hard error (spawning a non-AppContainer child = the WFP filter
        /// matches nothing = silent non-enforcement, the worst outcome).
        pub app_container_name: Option<String>,
    }

    /// Manual argv loop. No `clap` — RESEARCH §4a: broker attack surface MUST
    /// be minimal. Parse errors fail fast; no positional args, every arg is
    /// flag-prefixed.
    pub fn parse_args(raw: &[OsString]) -> NonoResult<BrokerArgs> {
        let mut shell_path: Option<PathBuf> = None;
        let mut shell_args: Vec<String> = Vec::new();
        let mut inherit_handles: Vec<HANDLE> = Vec::new();
        let mut cwd: Option<PathBuf> = None;
        let mut no_pty: bool = false;
        let mut app_container_name: Option<String> = None;

        // Skip argv[0] (the broker binary path).
        let mut iter = raw.iter().skip(1);
        while let Some(flag) = iter.next() {
            let flag_str = flag.to_string_lossy();
            match flag_str.as_ref() {
                "--shell" => {
                    let v = iter
                        .next()
                        .ok_or_else(|| NonoError::SandboxInit("--shell requires a value".into()))?;
                    shell_path = Some(PathBuf::from(v));
                }
                "--shell-arg" => {
                    let v = iter.next().ok_or_else(|| {
                        NonoError::SandboxInit("--shell-arg requires a value".into())
                    })?;
                    shell_args.push(v.to_string_lossy().into_owned());
                }
                "--inherit-handle" => {
                    let v = iter.next().ok_or_else(|| {
                        NonoError::SandboxInit("--inherit-handle requires a hex value".into())
                    })?;
                    let hex_str = v.to_string_lossy();
                    let stripped = hex_str.trim_start_matches("0x").trim_start_matches("0X");
                    let raw_value = usize::from_str_radix(stripped, 16).map_err(|e| {
                        NonoError::SandboxInit(format!(
                            "--inherit-handle parse error for '{hex_str}': {e}"
                        ))
                    })?;
                    // Phase 41 D-11 (CR-02): reject null (0) and INVALID_HANDLE_VALUE
                    // (usize::MAX on the pointer width — (HANDLE)-1 on 64-bit Windows).
                    // Passing null HANDLE to PROC_THREAD_ATTRIBUTE_HANDLE_LIST is undefined
                    // Win32 behavior; pseudo-handle confusion at (HANDLE)0 could resolve
                    // to the calling process's pseudo-handle in some Win32 paths.
                    if raw_value == 0 || raw_value == usize::MAX {
                        return Err(NonoError::SandboxInit(format!(
                            "--inherit-handle value '{hex_str}' is null or INVALID_HANDLE_VALUE; reject"
                        )));
                    }
                    inherit_handles.push(raw_value as HANDLE);
                }
                "--cwd" => {
                    let v = iter
                        .next()
                        .ok_or_else(|| NonoError::SandboxInit("--cwd requires a value".into()))?;
                    cwd = Some(PathBuf::from(v));
                }
                "--no-pty" => {
                    // Phase 51 Plan 02: boolean flag; takes no value. When present,
                    // run() will engage STARTF_USESTDHANDLES to bind inherit_handles
                    // as the child's stdio instead of inheriting the broker's console.
                    no_pty = true;
                }
                "--app-container-name" => {
                    // Plan 62-12: the per-run AppContainer moniker. The broker
                    // derives the package SID from it and spawns the child as a
                    // per-run AppContainer (lowbox), which starts cleanly and is
                    // WFP-matchable via the same package SID.
                    let v = iter.next().ok_or_else(|| {
                        NonoError::SandboxInit("--app-container-name requires a value".into())
                    })?;
                    app_container_name = Some(v.to_string_lossy().into_owned());
                }
                other => {
                    return Err(NonoError::SandboxInit(format!(
                        "unknown broker arg: '{other}'"
                    )));
                }
            }
        }

        let shell_path =
            shell_path.ok_or_else(|| NonoError::SandboxInit("missing required --shell".into()))?;
        let cwd = cwd.ok_or_else(|| NonoError::SandboxInit("missing required --cwd".into()))?;
        // Phase 41 D-12 (CR-03): reject empty --inherit-handle list. The broker
        // requires at least one inheritable handle so the child has a valid
        // PROC_THREAD_ATTRIBUTE_HANDLE_LIST to bind against. Supersedes Plan 31-02
        // SUMMARY's "empty list = most-restrictive" claim — the broker now makes
        // this state correct-by-construction-rejected, not correct-by-runtime-error.
        if inherit_handles.is_empty() {
            return Err(NonoError::SandboxInit(
                "--inherit-handle list is empty; broker requires at least one inheritable handle"
                    .into(),
            ));
        }

        // Plan 62-12: validate the AppContainer name by deriving the package SID
        // at parse time (fail-closed). A malformed/unusable name is caught here
        // before any spawn attempt. The derived SID is dropped immediately — we
        // only need the validity verdict; run() re-derives it for the spawn.
        if let Some(ref name) = app_container_name {
            // nono::derive_app_container_sid fails closed on an empty/invalid
            // moniker (non-S_OK HRESULT / null PSID) and frees the SID on drop.
            let _validated = nono::derive_app_container_sid(name)?;
        }

        // FAIL-CLOSED: the --no-pty path enables per-session WFP enforcement;
        // the broker child MUST be a per-run AppContainer whose package SID the
        // WFP filter keys on. Spawning a non-AppContainer child means the WFP
        // filter installs but matches nothing — silent non-enforcement, the
        // worst outcome (plan 62-12 / debug D4c).
        if no_pty && app_container_name.is_none() {
            return Err(NonoError::SandboxInit(
                "--no-pty requires --app-container-name (WFP per-session enforcement); \
                 refusing to spawn a non-AppContainer (unmatched WFP) child"
                    .into(),
            ));
        }

        Ok(BrokerArgs {
            shell_path,
            shell_args,
            inherit_handles,
            cwd,
            no_pty,
            app_container_name,
        })
    }

    /// Build a Win32 command line: `"<shell_path>" arg1 arg2 ...`.
    /// Quoting policy: shell_path always quoted; args quoted if they contain
    /// whitespace or `"`. This matches the PoC's implicit shape (PoC used a
    /// single literal string `"powershell.exe -NoLogo"`).
    pub fn build_command_line(args: &BrokerArgs) -> Vec<u16> {
        let mut cmd = String::new();
        cmd.push('"');
        cmd.push_str(&args.shell_path.to_string_lossy());
        cmd.push('"');
        for a in &args.shell_args {
            cmd.push(' ');
            if a.contains(' ') || a.contains('"') {
                cmd.push('"');
                // Escape embedded quotes by doubling them (PowerShell convention).
                cmd.push_str(&a.replace('"', "\"\""));
                cmd.push('"');
            } else {
                cmd.push_str(a);
            }
        }
        OsStr::new(&cmd).encode_wide().chain(Some(0)).collect()
    }

    fn to_u16_null_terminated(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(Some(0)).collect()
    }

    /// Order-preserving dedup of the inheritable-handle list for the
    /// `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`.
    ///
    /// **Why this exists (bug `broker-nopty-createproc-gle87`, 2026-05-27):**
    /// the no-PTY path's CR-01 (commit `f79a5a1a`) stderr→stdout merge makes
    /// `nono-cli` pass three `--inherit-handle` values in which `hStdOutput`
    /// and `hStdError` are the SAME handle value (`stdout_write`). A
    /// `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` that contains a DUPLICATE handle is
    /// rejected by the kernel at process-creation time, so
    /// `CreateProcessAsUserW` returns `ERROR_INVALID_PARAMETER` (87) — exactly
    /// the observed failure (token constructed, then 87). The HANDLE_LIST must
    /// gate each unique inheritable handle EXACTLY ONCE; the std-handle BIND
    /// (`hStdInput`/`hStdOutput`/`hStdError`) is free to alias the same handle
    /// in two slots, and is NOT changed by this dedup. `nono-cli`'s own
    /// HANDLE_LIST already dedupes (`gated_handles`); this restores the same
    /// invariant on the broker side.
    ///
    /// Insertion order is preserved so the (already CR-02-validated) handle
    /// values keep their argv order for any future ordering-sensitive logic.
    fn dedup_handles_preserve_order(handles: &[HANDLE]) -> Vec<HANDLE> {
        let mut seen: Vec<HANDLE> = Vec::with_capacity(handles.len());
        for &h in handles {
            if !seen.contains(&h) {
                seen.push(h);
            }
        }
        seen
    }

    /// 8-step sequence. Mechanism MUST stay byte-equivalent to the validated
    /// PoC at `.planning/quick/260508-m99-.../poc-broker/src/main.rs:36-186`,
    /// with token construction unified through `nono::create_low_integrity_primary_token`
    /// per D-06 and HANDLE_LIST discipline added per D-02.
    pub fn run(args: BrokerArgs) -> NonoResult<i32> {
        // Step 1: AllocConsole — non-fatal if parent already attached.
        // rc=0 means console inherited (expected when spawned by nono.exe);
        // rc != 0 means new console (when broker invoked standalone for testing).
        let alloc_rc = unsafe {
            // SAFETY: AllocConsole takes no arguments; safe to call unconditionally.
            AllocConsole()
        };
        tracing::info!(alloc_console_rc = alloc_rc, "broker: console attach probe");

        // Steps 2-5: token / AppContainer setup.
        //
        // Plan 62-12 (F-62-UAT-05 redesign, debug `wfp-write-restricted-0142`):
        //   - When `app_container_name` is present (--no-pty path), the child is
        //     spawned as a per-run AppContainer (lowbox) via `CreateProcessW` +
        //     `SECURITY_CAPABILITIES`. The broker derives the package SID here
        //     (FAIL-CLOSED: `?` propagates any FFI failure; we NEVER fall back to
        //     a plain token or spawn without the AppContainer). The Low-IL label
        //     is applied to the SUSPENDED child's primary token after spawn.
        //   - Otherwise (PTY / legacy path) the broker self-degrades to a plain
        //     Low-IL primary token and spawns via `CreateProcessAsUserW` (D-06).
        //
        // The lowbox is per-run-unique and starts cleanly (private
        // AppContainerNamedObjects namespace), eliminating the 0xC0000142
        // STATUS_DLL_INIT_FAILED that the falsified 62-10 WRITE_RESTRICTED token
        // caused. The SAME package SID scopes the WFP ALE_USER_ID filter
        // (single source: nono-cli derives it from the same name).
        // Plan 62-13 (the SPAWN fix — debug `wfp-write-restricted-0142` decisive
        // spike): a DERIVE-ONLY package SID is insufficient. `CreateProcessW` with
        // `SECURITY_CAPABILITIES` fails `ERROR_FILE_NOT_FOUND` unless the
        // AppContainer PROFILE is REGISTERED first (it creates the registry entry
        // + the `\Sessions\<n>\AppContainerNamedObjects\<pkgSid>` namespace the
        // lowbox launches into). REGISTER the per-run profile BEFORE deriving the
        // SID / building SECURITY_CAPABILITIES, and HOLD the guard until the child
        // exits (the guard drops at the END of `run`, after WaitForSingleObject,
        // so DeleteAppContainerProfile runs on child exit — RAII).
        //
        // FAIL-CLOSED: `?` propagates any CreateAppContainerProfile failure (other
        // than ALREADY_EXISTS, which the lib tolerates) — we NEVER spawn an
        // unregistered/unmatched child (silent non-enforcement). The SAME name
        // (single source) yields the SAME package SID on BOTH the broker spawn and
        // the WFP ALE_USER_ID filter.
        let _app_container_profile: Option<nono::AppContainerProfile> =
            match args.app_container_name.as_deref() {
                Some(name) => {
                    let profile = nono::create_app_container_profile(name)?;
                    tracing::info!(
                        app_container_name = %name,
                        "broker: AppContainer profile registered"
                    );
                    Some(profile)
                }
                None => None,
            };
        let app_container_sid: Option<nono::OwnedAppContainerSid> =
            match args.app_container_name.as_deref() {
                Some(name) => Some(nono::derive_app_container_sid(name)?),
                None => None,
            };
        // For the legacy/PTY path only: build the plain Low-IL primary token.
        // For the AppContainer path the child token is produced by the lowbox at
        // spawn time, so no primary token is built here.
        let low_il_token: Option<OwnedHandle> = if app_container_sid.is_some() {
            None
        } else {
            Some(nono::create_low_integrity_primary_token()?)
        };
        tracing::info!(
            app_container = app_container_sid.is_some(),
            "broker: token/AppContainer setup complete"
        );

        // Step 6: Build the proc-thread attribute list.
        // Slot count: 1 (HANDLE_LIST) for the legacy path, 2 (HANDLE_LIST +
        // SECURITY_CAPABILITIES) when spawning an AppContainer child.
        let attr_count: u32 = if app_container_sid.is_some() { 2 } else { 1 };
        let mut attr_size: usize = 0;
        unsafe {
            // SAFETY: First call with null list queries required size; documented Win32 idiom.
            // Documented to return ERROR_INSUFFICIENT_BUFFER and write the required size.
            InitializeProcThreadAttributeList(std::ptr::null_mut(), attr_count, 0, &mut attr_size);
        }
        let mut attr_buf = vec![0u8; attr_size];
        let attr_list: LPPROC_THREAD_ATTRIBUTE_LIST =
            attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
        let ok = unsafe {
            // SAFETY: attr_list points to attr_buf, sized by the probe call above for `attr_count`.
            InitializeProcThreadAttributeList(attr_list, attr_count, 0, &mut attr_size)
        };
        if ok == 0 {
            let err = unsafe {
                // SAFETY: GetLastError takes no arguments; always safe to call.
                GetLastError()
            };
            return Err(NonoError::SandboxInit(format!(
                "InitializeProcThreadAttributeList failed (GetLastError={err})"
            )));
        }

        // D-02: HANDLE_LIST = the UNIQUE inheritable handles passed via --inherit-handle.
        // Phase 41 D-12 (CR-03): the empty-list case is rejected by parse_args()
        // before reaching here, so inherit_handles is guaranteed non-empty.
        //
        // Bug broker-nopty-createproc-gle87 (2026-05-27): the no-PTY stderr→stdout
        // merge means hStdOutput and hStdError arrive as the SAME handle value, so
        // args.inherit_handles can contain a duplicate. A PROC_THREAD_ATTRIBUTE_HANDLE_LIST
        // with a duplicate handle is rejected by the kernel at CreateProcessAsUserW time
        // with ERROR_INVALID_PARAMETER (87). Dedup (order-preserving) so the HANDLE_LIST
        // gates each unique handle exactly once; the std-handle BIND below is unaffected
        // and may still alias the same handle across hStdOutput/hStdError.
        let handles_array: Vec<HANDLE> = dedup_handles_preserve_order(&args.inherit_handles);
        let handles_byte_size = std::mem::size_of_val(handles_array.as_slice());
        let ok = unsafe {
            // SAFETY: attr_list initialized above; handles_array lives for the duration of the call.
            UpdateProcThreadAttribute(
                attr_list,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles_array.as_ptr() as *mut _,
                handles_byte_size,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            let err = unsafe {
                // SAFETY: GetLastError takes no arguments; always safe to call.
                GetLastError()
            };
            unsafe {
                // SAFETY: attr_list was initialized successfully above.
                DeleteProcThreadAttributeList(attr_list);
            }
            return Err(NonoError::SandboxInit(format!(
                "UpdateProcThreadAttribute(HANDLE_LIST) failed (GetLastError={err})"
            )));
        }

        // Plan 62-12: when spawning a per-run AppContainer, add the second
        // proc-thread attribute PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
        // carrying SECURITY_CAPABILITIES{ AppContainerSid, CapabilityCount: 0 }.
        // The EMPTY capability set is the security-correct default (the child
        // can reach nothing extra). `security_caps` and `app_container_sid`
        // MUST outlive the CreateProcessW call below — both are owned by `run`'s
        // stack and dropped only after the spawn returns.
        let security_caps: Option<SECURITY_CAPABILITIES> =
            app_container_sid.as_ref().map(|sid| SECURITY_CAPABILITIES {
                AppContainerSid: sid.as_psid(),
                Capabilities: std::ptr::null_mut(),
                CapabilityCount: 0,
                Reserved: 0,
            });
        if let Some(ref caps) = security_caps {
            let ok = unsafe {
                // SAFETY: attr_list was initialized with slot count 2 above (the
                // AppContainer branch sets attr_count=2). `caps` is a valid
                // SECURITY_CAPABILITIES whose AppContainerSid is kept live by
                // `app_container_sid` for the duration of the spawn. The pointer
                // and size describe a single SECURITY_CAPABILITIES value.
                UpdateProcThreadAttribute(
                    attr_list,
                    0,
                    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                    caps as *const SECURITY_CAPABILITIES as *mut _,
                    size_of::<SECURITY_CAPABILITIES>(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                let err = unsafe {
                    // SAFETY: GetLastError takes no arguments; always safe to call.
                    GetLastError()
                };
                unsafe {
                    // SAFETY: attr_list was initialized successfully above.
                    DeleteProcThreadAttributeList(attr_list);
                }
                return Err(NonoError::SandboxInit(format!(
                    "UpdateProcThreadAttribute(SECURITY_CAPABILITIES) failed (GetLastError={err})"
                )));
            }
        }

        // Step 7: CreateProcessAsUserW with dwCreationFlags = EXTENDED_STARTUPINFO_PRESENT only.
        // D-01: no new-console flag, no pseudoconsole proc-thread attribute — child inherits
        // the broker's already-attached console; KernelBase skips CSRSS attach at Low IL because
        // a console handle is already inherited (RESEARCH A1, PoC-validated 2026-05-08).
        let mut command_line = build_command_line(&args);
        let cwd_wide = to_u16_null_terminated(args.cwd.as_os_str());

        let mut startup_info_ex: STARTUPINFOEXW = unsafe {
            // SAFETY: STARTUPINFOEXW is #[repr(C)] POD; zero-init is documented Win32 idiom.
            std::mem::zeroed()
        };
        startup_info_ex.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup_info_ex.lpAttributeList = attr_list;

        // Phase 51 Plan 02 (T-51B-01 mitigation): bind the three passed pipe handles
        // as the child's stdio when the no-PTY path is active. Without this,
        // `CreateProcessAsUserW` would wire the child's stdio to the broker's console
        // and the supervisor relay would never receive the child's output (Pitfall 7
        // in RESEARCH.md). The guard `args.inherit_handles.len() >= 3` ensures we do
        // not partially bind (which would leave one stdio slot pointing at the console).
        //
        // Security note: BROKER-CR-02 (null/INVALID_HANDLE_VALUE rejection) and
        // BROKER-CR-03 (empty-list rejection) both execute in parse_args() before this
        // branch is reached — they are not bypassed by `--no-pty` (T-51B-03 accepted).
        // STARTF_USESTDHANDLES only changes which fd the child writes stdout to;
        // mandatory-label NO_WRITE_UP enforcement is at token/kernel level, entirely
        // independent of stdio handle binding (T-51B-02 accepted).
        //
        // WR-01 (Phase 51 code review): fail CLOSED if --no-pty is requested
        // without the full set of three stdio handles. Silently skipping the
        // bind would leave the child's stdio pointing at the broker's inherited
        // console — a silent degrade that violates CLAUDE.md "never silently
        // degrade / fail secure". The production nono-cli path always passes
        // exactly three; this guard rejects any malformed invocation.
        //
        // Bug broker-nopty-createproc-gle87: hStdOutput and hStdError MAY be the
        // same handle value (the supervisor merges child stderr into stdout). That
        // aliasing is intentional and correct HERE — only the HANDLE_LIST above is
        // deduped; the bind keeps all three slots.
        if args.no_pty && args.inherit_handles.len() < 3 {
            return Err(NonoError::SandboxInit(format!(
                "--no-pty requires three inherited stdio handles (stdin, stdout, stderr); got {}. \
                 Refusing to bind child stdio to the broker console (fail-closed).",
                args.inherit_handles.len()
            )));
        }
        if args.no_pty && args.inherit_handles.len() >= 3 {
            // SAFETY: hStd* fields accept raw HANDLE values passed from the trusted
            // nono-cli caller via --inherit-handle. BROKER-CR-02 has already validated
            // that each handle is non-null and non-INVALID_HANDLE_VALUE; the HANDLE
            // values themselves are opaque integers — no dereference occurs here.
            startup_info_ex.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
            startup_info_ex.StartupInfo.hStdInput = args.inherit_handles[0];
            startup_info_ex.StartupInfo.hStdOutput = args.inherit_handles[1];
            startup_info_ex.StartupInfo.hStdError = args.inherit_handles[2];
        }

        let mut process_info: PROCESS_INFORMATION = unsafe {
            // SAFETY: PROCESS_INFORMATION zero-init is documented Win32 idiom.
            std::mem::zeroed()
        };

        let lp_startup_info = &startup_info_ex.StartupInfo as *const STARTUPINFOW;

        // Plan 62-12: two spawn shapes.
        //   - AppContainer (no-PTY): CreateProcessW (the broker's Medium-IL token
        //     is the base; SECURITY_CAPABILITIES derives the lowbox child token).
        //     Spawned CREATE_SUSPENDED so we can label the child token Low-IL
        //     before any user code runs, then resume (defence-in-depth /
        //     NO_WRITE_UP parity, D1 step 3). bInheritHandles=1 — the HANDLE_LIST
        //     gates the inherited set.
        //   - Legacy/PTY: CreateProcessAsUserW with the plain Low-IL primary token.
        let (created, is_app_container) = if let Some(_caps) = security_caps.as_ref() {
            let creation_flags = EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED;
            let rc = unsafe {
                // SAFETY: command_line/cwd_wide are null-terminated UTF-16. The
                // startup struct carries EXTENDED_STARTUPINFO_PRESENT and the
                // 2-slot attribute list (HANDLE_LIST + SECURITY_CAPABILITIES).
                // bInheritHandles=1 is required for the HANDLE_LIST attribute.
                CreateProcessW(
                    std::ptr::null(),
                    command_line.as_mut_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1, // bInheritHandles=TRUE (HANDLE_LIST gates)
                    creation_flags,
                    std::ptr::null(), // inherit broker env
                    cwd_wide.as_ptr(),
                    lp_startup_info,
                    &mut process_info,
                )
            };
            (rc, true)
        } else {
            // Legacy path requires the plain Low-IL primary token.
            let token = low_il_token.as_ref().ok_or_else(|| {
                NonoError::SandboxInit(
                    "internal: non-AppContainer spawn reached without a Low-IL token".into(),
                )
            })?;
            let rc = unsafe {
                // SAFETY: token.raw() is a valid primary token (RAII-owned).
                // command_line/cwd_wide are null-terminated UTF-16; the startup
                // struct carries EXTENDED_STARTUPINFO_PRESENT + the HANDLE_LIST.
                CreateProcessAsUserW(
                    token.raw(),
                    std::ptr::null(),
                    command_line.as_mut_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1,                            // bInheritHandles=TRUE (HANDLE_LIST gates)
                    EXTENDED_STARTUPINFO_PRESENT, // dwCreationFlags (D-01: no new-console flag)
                    std::ptr::null(),             // lpEnvironment: inherit broker env
                    cwd_wide.as_ptr(),
                    lp_startup_info,
                    &mut process_info,
                )
            };
            (rc, false)
        };

        unsafe {
            // SAFETY: attr_list was initialized above and is no longer needed
            // after the spawn call consumed it.
            DeleteProcThreadAttributeList(attr_list);
        }

        if created == 0 {
            let err = unsafe {
                // SAFETY: GetLastError takes no arguments; always safe to call.
                GetLastError()
            };
            return Err(NonoError::SandboxInit(format!(
                "{} failed (GetLastError={err})",
                if is_app_container {
                    "CreateProcessW (AppContainer)"
                } else {
                    "CreateProcessAsUserW"
                }
            )));
        }

        // Wrap child handles in OwnedHandle for RAII cleanup.
        let child_process = OwnedHandle(process_info.hProcess);
        let child_thread = OwnedHandle(process_info.hThread);

        // Plan 62-12 (D1 step 3 / D5 #4): for the AppContainer child, apply the
        // Low-IL mandatory label to the (suspended) child's primary token for
        // explicit NO_WRITE_UP parity, then resume the main thread. FAIL-CLOSED:
        // any failure terminates the child and propagates Err — we never run an
        // unlabeled child.
        if is_app_container {
            let mut child_token: HANDLE = std::ptr::null_mut();
            let opened = unsafe {
                // SAFETY: child_process.raw() is a valid, suspended process
                // handle; &mut child_token is a valid out-pointer.
                OpenProcessToken(
                    child_process.raw(),
                    TOKEN_ADJUST_DEFAULT | TOKEN_QUERY,
                    &mut child_token,
                )
            };
            if opened == 0 {
                let err = unsafe { GetLastError() };
                unsafe {
                    // SAFETY: terminate the suspended child before bailing so we
                    // never leave an unlabeled (un-resumed) process behind.
                    windows_sys::Win32::System::Threading::TerminateProcess(child_process.raw(), 1);
                }
                return Err(NonoError::SandboxInit(format!(
                    "OpenProcessToken on AppContainer child failed (GetLastError={err})"
                )));
            }
            let child_token = OwnedHandle(child_token);
            if let Err(e) = nono::apply_low_il_label_to_token(child_token.raw()) {
                unsafe {
                    // SAFETY: see above — fail closed by terminating the child.
                    windows_sys::Win32::System::Threading::TerminateProcess(child_process.raw(), 1);
                }
                return Err(e);
            }
            let resumed = unsafe {
                // SAFETY: child_thread.raw() is the valid main-thread handle of
                // the suspended child. ResumeThread returns the previous suspend
                // count, or u32::MAX (-1) on error.
                ResumeThread(child_thread.raw())
            };
            if resumed == u32::MAX {
                let err = unsafe { GetLastError() };
                unsafe {
                    // SAFETY: fail closed — terminate rather than leave suspended.
                    windows_sys::Win32::System::Threading::TerminateProcess(child_process.raw(), 1);
                }
                return Err(NonoError::SandboxInit(format!(
                    "ResumeThread on AppContainer child failed (GetLastError={err})"
                )));
            }
        }
        let _child_thread = child_thread;
        tracing::info!(
            child_pid = process_info.dwProcessId,
            app_container = is_app_container,
            "broker: spawned child"
        );

        // Step 8: Wait + propagate exit code (D-03).
        let wait_rc = unsafe {
            // SAFETY: child_process.raw() is a valid process handle from CreateProcessAsUserW.
            WaitForSingleObject(child_process.raw(), INFINITE)
        };
        if wait_rc != 0 {
            let err = unsafe {
                // SAFETY: GetLastError takes no arguments; always safe to call.
                GetLastError()
            };
            return Err(NonoError::SandboxInit(format!(
                "WaitForSingleObject failed (rc={wait_rc}, GetLastError={err})"
            )));
        }

        let mut exit_code: u32 = 0;
        let ok = unsafe {
            // SAFETY: child_process.raw() is still valid; exit_code is a valid out-pointer.
            GetExitCodeProcess(child_process.raw(), &mut exit_code)
        };
        if ok == 0 {
            let err = unsafe {
                // SAFETY: GetLastError takes no arguments; always safe to call.
                GetLastError()
            };
            return Err(NonoError::SandboxInit(format!(
                "GetExitCodeProcess failed (GetLastError={err})"
            )));
        }

        tracing::info!(child_exit_code = exit_code, "broker: child exited");
        // OwnedHandle Drop closes child_process, child_thread, and low_il_token automatically.
        Ok(exit_code as i32)
    }

    /// Phase 31 Plan 31-02 Task 2 — Nyquist gap-fill: pin the broker argv
    /// parser's behavior at the unit-test layer. Plan 31-05's field-test
    /// validates the end-to-end shape; these tests pin the contract so future
    /// regressions surface at unit-test time, not field-test time.
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod parse_args_tests {
        use super::*;
        use nono::NonoError;

        fn os(s: &str) -> OsString {
            OsString::from(s)
        }

        /// Helper: argv0 ("broker.exe") followed by the actual flags. The parser
        /// skips argv[0], so the first OsString must always be a placeholder.
        fn argv(rest: &[&str]) -> Vec<OsString> {
            let mut v = vec![os("broker.exe")];
            v.extend(rest.iter().map(|s| os(s)));
            v
        }

        /// D-08: `--shell` is required; absence is fatal with a structured
        /// `SandboxInit` error mentioning the missing flag. Guards against
        /// regressions that would let the broker spawn an arbitrary or
        /// defaulted shell when nono.exe forgets to pass `--shell`.
        #[test]
        fn parse_args_missing_shell_returns_error() {
            let raw = argv(&["--cwd", r"C:\foo"]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error when --shell is omitted");
            };
            assert!(
                msg.contains("missing required --shell"),
                "error message must explicitly call out missing --shell; got: {msg}"
            );
        }

        /// D-08: `--cwd` is required; absence is fatal. Guards against
        /// regressions that would let the broker default the cwd (e.g. to
        /// the broker's own working dir, which is the supervisor's cwd —
        /// a capability leak).
        #[test]
        fn parse_args_missing_cwd_returns_error() {
            let raw = argv(&["--shell", r"C:\Windows\System32\notepad.exe"]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error when --cwd is omitted");
            };
            assert!(
                msg.contains("missing required --cwd"),
                "error message must explicitly call out missing --cwd; got: {msg}"
            );
        }

        /// T-31-20 mitigation: unknown flags MUST hard-fail. The broker is a
        /// minimal-attack-surface binary; silently accepting unknown flags
        /// would let a future bug in nono.exe pass attacker-controlled data
        /// through.
        #[test]
        fn parse_args_unknown_flag_returns_error() {
            let raw = argv(&[
                "--unknown-flag",
                "value",
                "--shell",
                r"C:\foo.exe",
                "--cwd",
                r"C:\",
            ]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error on unknown flag");
            };
            assert!(
                msg.contains("unknown broker arg"),
                "error message must call out 'unknown broker arg'; got: {msg}"
            );
        }

        /// D-08: `--inherit-handle` values are hex-encoded HANDLE values.
        /// Non-hex inputs MUST fail-fast — silently coercing them to 0 would
        /// either break inheritance or worse, accidentally reference a
        /// real handle in the broker's table.
        #[test]
        fn parse_args_invalid_hex_inherit_handle_returns_error() {
            let raw = argv(&[
                "--inherit-handle",
                "xyz",
                "--shell",
                r"C:\foo.exe",
                "--cwd",
                r"C:\",
            ]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error on non-hex --inherit-handle");
            };
            assert!(
                msg.contains("--inherit-handle parse error"),
                "error message must mention --inherit-handle parse error; got: {msg}"
            );
        }

        /// D-08: `--shell-arg` is repeatable and order-preserving. Argv order
        /// determines argv order in the spawned shell — re-ordering would
        /// silently change the meaning of the spawn (e.g. moving `-Command`
        /// past its payload).
        ///
        /// Note: includes one `--inherit-handle` value to satisfy the Phase 41
        /// D-12 (CR-03) requirement that the list be non-empty.
        #[test]
        fn parse_args_shell_arg_preserves_order() {
            let raw = argv(&[
                "--shell",
                "foo.exe",
                "--shell-arg",
                "-A",
                "--shell-arg",
                "-B",
                "--shell-arg",
                "--foo",
                "--inherit-handle",
                "0xa",
                "--cwd",
                r"C:\",
            ]);
            let parsed = parse_args(&raw).expect("parse must succeed");
            assert_eq!(
                parsed.shell_args,
                vec!["-A".to_string(), "-B".to_string(), "--foo".to_string()],
                "shell_args order must match argv order; reordering would silently \
                 change the spawned command's meaning"
            );
        }

        /// D-08: `--inherit-handle` accepts both `0x` and `0X` prefixes (and
        /// strips them before hex parsing). Both are accumulated in argv
        /// order. Guards against the prefix-matching bug where only one case
        /// was stripped → the other case would parse as a different value
        /// (or fail entirely).
        #[test]
        fn parse_args_multiple_inherit_handles_accumulate() {
            let raw = argv(&[
                "--inherit-handle",
                "0xa",
                "--inherit-handle",
                "0X10",
                "--shell",
                "foo",
                "--cwd",
                r"C:\",
            ]);
            let parsed = parse_args(&raw).expect("parse must succeed");
            assert_eq!(
                parsed.inherit_handles.len(),
                2,
                "both --inherit-handle flags must accumulate"
            );
            assert_eq!(
                parsed.inherit_handles[0] as usize, 0xa,
                "first handle must parse from lowercase 0x prefix"
            );
            assert_eq!(
                parsed.inherit_handles[1] as usize, 0x10,
                "second handle must parse from uppercase 0X prefix"
            );
        }

        /// Phase 41 D-12 (CR-03): an empty inherit-handle list is REJECTED at the
        /// broker argv parser. Supersedes Plan 31-02 SUMMARY's "most-restrictive"
        /// claim — the broker now requires at least one inheritable handle, making
        /// the empty-list shape correct-by-construction-rejected.
        #[test]
        fn parse_args_empty_inherit_handle_list_returns_error() {
            let raw = argv(&["--shell", "foo", "--cwd", r"C:\"]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error on empty --inherit-handle list");
            };
            assert!(
                msg.contains("empty"),
                "error message must indicate empty-list rejection, got: {msg}"
            );
        }

        /// Phase 41 D-11 (CR-02): a null or INVALID_HANDLE_VALUE handle is REJECTED
        /// at the broker argv parser. Pseudo-handle confusion at `(HANDLE)0` and
        /// the `(HANDLE)-1` sentinel are blocked before any UpdateProcThreadAttribute
        /// call. Locks the CR-02 fix against regression.
        #[test]
        fn parse_args_null_inherit_handle_returns_error() {
            let raw = argv(&["--shell", "foo", "--cwd", r"C:\", "--inherit-handle", "0x0"]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error on --inherit-handle 0x0");
            };
            assert!(
                msg.contains("null") || msg.contains("INVALID_HANDLE_VALUE"),
                "error message must indicate null-handle rejection, got: {msg}"
            );
        }

        /// Phase 41 D-11 (CR-02): the INVALID_HANDLE_VALUE sentinel (0xFFFFFFFFFFFFFFFF on
        /// 64-bit Windows) is also REJECTED. Defense-in-depth alongside the null check.
        #[test]
        fn parse_args_invalid_handle_value_inherit_handle_returns_error() {
            let raw = argv(&[
                "--shell",
                "foo",
                "--cwd",
                r"C:\",
                "--inherit-handle",
                "0xFFFFFFFFFFFFFFFF",
            ]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error on --inherit-handle 0xFFFFFFFFFFFFFFFF");
            };
            assert!(
                msg.contains("null") || msg.contains("INVALID_HANDLE_VALUE"),
                "error message must indicate INVALID_HANDLE_VALUE rejection, got: {msg}"
            );
        }

        /// Defensive parse: a flag at the end of argv with no following value
        /// MUST fail — silently treating it as an empty string would let a
        /// truncated argv slip through (e.g., from a corrupted IPC channel).
        #[test]
        fn parse_args_dangling_flag_value_returns_error() {
            // `--shell` is the last token; no value follows.
            let raw = argv(&["--cwd", r"C:\", "--shell"]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!("expected SandboxInit error when --shell has no value");
            };
            assert!(
                msg.contains("--shell requires a value"),
                "dangling --shell must report 'requires a value'; got: {msg}"
            );
        }

        /// Phase 51 Plan 02 / Plan 62-12: `--no-pty` flag is recognized and sets
        /// `no_pty=true`. When passed alongside three `--inherit-handle` values and
        /// a valid `--app-container-name` (required by the Plan 62-12 fail-closed
        /// gate), the flags must parse without error and the resulting
        /// `BrokerArgs.no_pty` must be `true`. Guards against regressions where
        /// `--no-pty` falls through to the `unknown broker arg` arm, causing the
        /// broker to hard-fail when nono-cli passes the flag via `BrokerLaunchNoPty`.
        #[test]
        fn parse_args_no_pty_flag_accepted() {
            let raw = argv(&[
                "--shell",
                r"C:\foo.exe",
                "--inherit-handle",
                "0x0000000000000100",
                "--inherit-handle",
                "0x0000000000000200",
                "--inherit-handle",
                "0x0000000000000300",
                "--no-pty",
                "--app-container-name",
                "nono.session.deadbeefcafebabe0123456789abcdef",
                "--cwd",
                r"C:\",
            ]);
            let parsed = parse_args(&raw)
                .expect("--no-pty with valid --app-container-name must parse without error");
            assert!(
                parsed.no_pty,
                "BrokerArgs.no_pty must be true when --no-pty is present"
            );
            assert_eq!(
                parsed.inherit_handles.len(),
                3,
                "all three --inherit-handle values must accumulate (needed for STARTF_USESTDHANDLES)"
            );
        }

        /// Phase 51 Plan 02: when `--no-pty` is absent, `BrokerArgs.no_pty`
        /// defaults to `false`. Guards against a regression where the field is
        /// accidentally initialized to `true`, which would silently engage
        /// STARTF_USESTDHANDLES on every spawn regardless of whether nono-cli
        /// requested the no-PTY path.
        #[test]
        fn parse_args_no_pty_absent_defaults_false() {
            let raw = argv(&[
                "--shell",
                r"C:\foo.exe",
                "--inherit-handle",
                "0x0000000000000100",
                "--cwd",
                r"C:\",
            ]);
            let parsed = parse_args(&raw).expect("parse without --no-pty must succeed");
            assert!(
                !parsed.no_pty,
                "BrokerArgs.no_pty must be false when --no-pty is absent"
            );
        }

        // -------------------------------------------------------------------
        // Plan 62-12: --app-container-name tests
        // -------------------------------------------------------------------

        /// Plan 62-12: `--app-container-name` is parsed into
        /// `BrokerArgs.app_container_name`. Pins the flag is recognised (not
        /// falling through to `unknown broker arg`) and that the value survives
        /// into the struct.
        #[test]
        fn parse_args_app_container_name_parsed() {
            let name = "nono.session.deadbeefcafebabe0123456789abcdef";
            let raw = argv(&[
                "--shell",
                r"C:\foo.exe",
                "--inherit-handle",
                "0x0000000000000100",
                "--inherit-handle",
                "0x0000000000000200",
                "--inherit-handle",
                "0x0000000000000300",
                "--no-pty",
                "--app-container-name",
                name,
                "--cwd",
                r"C:\",
            ]);
            let parsed =
                parse_args(&raw).expect("valid --app-container-name must parse without error");
            assert_eq!(
                parsed.app_container_name.as_deref(),
                Some(name),
                "BrokerArgs.app_container_name must equal the provided moniker"
            );
        }

        /// Plan 62-12 FAIL-CLOSED: `--no-pty` without `--app-container-name` MUST
        /// return Err. Spawning a non-AppContainer child means the WFP filter
        /// matches nothing (silent non-enforcement — the worst outcome, D4c).
        #[test]
        fn parse_args_no_pty_without_app_container_name_returns_error() {
            let raw = argv(&[
                "--shell",
                r"C:\foo.exe",
                "--inherit-handle",
                "0x0000000000000100",
                "--inherit-handle",
                "0x0000000000000200",
                "--inherit-handle",
                "0x0000000000000300",
                "--no-pty",
                "--cwd",
                r"C:\",
            ]);
            let Err(NonoError::SandboxInit(msg)) = parse_args(&raw) else {
                panic!(
                    "parse_args must return Err(SandboxInit) when --no-pty is set \
                     without --app-container-name (fail-closed WFP enforcement)"
                );
            };
            assert!(
                msg.contains("--no-pty") && msg.contains("--app-container-name"),
                "error message must name both --no-pty and --app-container-name; got: {msg}"
            );
        }

        /// Plan 62-12 FAIL-CLOSED: an empty `--app-container-name` value MUST
        /// return Err at parse time (derive_app_container_sid rejects empty
        /// monikers before any spawn).
        #[test]
        fn parse_args_empty_app_container_name_returns_error() {
            let raw = argv(&[
                "--shell",
                r"C:\foo.exe",
                "--inherit-handle",
                "0x0000000000000100",
                "--inherit-handle",
                "0x0000000000000200",
                "--inherit-handle",
                "0x0000000000000300",
                "--no-pty",
                "--app-container-name",
                "",
                "--cwd",
                r"C:\",
            ]);
            let Err(NonoError::SandboxInit(_)) = parse_args(&raw) else {
                panic!(
                    "parse_args must return Err(SandboxInit) for an empty --app-container-name value"
                );
            };
        }
    }

    /// Bug `broker-nopty-createproc-gle87` (2026-05-27) regression guard: the
    /// no-PTY stderr→stdout merge makes `nono-cli` pass a `--inherit-handle`
    /// list with a DUPLICATE handle (hStdOutput == hStdError). The broker's
    /// `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` must gate each UNIQUE handle exactly
    /// once or `CreateProcessAsUserW` returns ERROR_INVALID_PARAMETER (87).
    /// These tests pin `dedup_handles_preserve_order` so the fix cannot silently
    /// regress back to the raw `args.inherit_handles.clone()` that caused 87.
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod dedup_handles_tests {
        use super::*;

        fn h(v: usize) -> HANDLE {
            v as HANDLE
        }

        /// The production no-PTY shape: [stdin_read, stdout_write, stdout_write]
        /// (positions 1 and 2 aliased by the stderr→stdout merge). Dedup MUST
        /// collapse to the two UNIQUE handles in first-seen order, so the
        /// HANDLE_LIST passed to UpdateProcThreadAttribute has no duplicate.
        #[test]
        fn dedup_collapses_merged_stdout_stderr_duplicate() {
            let stdin_read = h(0x100);
            let stdout_write = h(0x200);
            let raw = vec![stdin_read, stdout_write, stdout_write];
            let deduped = dedup_handles_preserve_order(&raw);
            assert_eq!(
                deduped,
                vec![stdin_read, stdout_write],
                "merged-stdio HANDLE_LIST must dedup to the unique set in first-seen order \
                 (else CreateProcessAsUserW returns ERROR_INVALID_PARAMETER 87)"
            );
        }

        /// A list with no duplicates is returned unchanged (order preserved).
        /// Guards against the dedup accidentally reordering or dropping unique
        /// handles on the PTY path (which passes distinct handles).
        #[test]
        fn dedup_preserves_unique_list_unchanged() {
            let raw = vec![h(0xa), h(0xb), h(0xc)];
            let deduped = dedup_handles_preserve_order(&raw);
            assert_eq!(
                deduped,
                vec![h(0xa), h(0xb), h(0xc)],
                "a list with no duplicates must pass through unchanged in order"
            );
        }

        /// A single handle round-trips. Smallest valid HANDLE_LIST.
        #[test]
        fn dedup_single_handle_unchanged() {
            let raw = vec![h(0x42)];
            let deduped = dedup_handles_preserve_order(&raw);
            assert_eq!(deduped, vec![h(0x42)]);
        }
    }

    /// Phase 31 Plan 31-02 Task 2 — Nyquist gap-fill: pin the broker
    /// command-line builder's quoting behavior. The Win32 CommandLine grammar
    /// is fragile; quoting bugs here would silently mis-tokenize the spawned
    /// shell's argv on the other side of `CreateProcessAsUserW`.
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod build_command_line_tests {
        use super::*;
        use std::path::PathBuf;

        fn args(shell_path: &str, shell_args: Vec<String>) -> BrokerArgs {
            BrokerArgs {
                shell_path: PathBuf::from(shell_path),
                shell_args,
                inherit_handles: vec![],
                cwd: PathBuf::from(r"C:\"),
                no_pty: false,
                app_container_name: None,
            }
        }

        /// Decode the trailing-null UTF-16 buffer back to a `String` for
        /// human-readable assertions. Drops the trailing 0 terminator.
        fn decode(wide: &[u16]) -> String {
            assert!(
                !wide.is_empty(),
                "command line must have at least the null terminator"
            );
            String::from_utf16_lossy(&wide[..wide.len() - 1])
        }

        /// D-08 contract: shell_path is ALWAYS quoted, even if it contains no
        /// whitespace, so the path-with-spaces case (e.g. `C:\Program Files\...`)
        /// can never be silently mis-tokenized.
        #[test]
        fn build_command_line_quotes_shell_path() {
            let a = args(r"C:\Windows\System32\powershell.exe", vec![]);
            let wide = build_command_line(&a);
            let s = decode(&wide);
            assert_eq!(
                s, "\"C:\\Windows\\System32\\powershell.exe\"",
                "shell_path must always be enclosed in literal double-quotes"
            );
        }

        /// Simple args (no whitespace, no quotes) round-trip without quoting.
        /// Order matches argv order.
        #[test]
        fn build_command_line_appends_simple_args() {
            let a = args(
                r"C:\foo.exe",
                vec!["-NoLogo".to_string(), "-NoProfile".to_string()],
            );
            let wide = build_command_line(&a);
            let s = decode(&wide);
            assert_eq!(
                s, "\"C:\\foo.exe\" -NoLogo -NoProfile",
                "simple args must be appended unquoted in argv order"
            );
        }

        /// Args containing whitespace MUST be enclosed in double-quotes so the
        /// child's CRT command-line parser tokenizes them as a single argv
        /// entry. Without this, "hello world" would arrive as two separate args.
        #[test]
        fn build_command_line_quotes_args_with_whitespace() {
            let a = args(r"C:\foo.exe", vec!["hello world".to_string()]);
            let wide = build_command_line(&a);
            let s = decode(&wide);
            assert!(
                s.contains("\"hello world\""),
                "whitespace-bearing args must be quoted; got: {s}"
            );
        }

        /// Embedded literal quotes in args must be doubled (PowerShell
        /// convention). Failure here would either truncate the arg at the
        /// embedded quote or leave the command line unbalanced.
        #[test]
        fn build_command_line_doubles_embedded_quotes() {
            let a = args(r"C:\foo.exe", vec!["a\"b".to_string()]);
            let wide = build_command_line(&a);
            let s = decode(&wide);
            assert!(
                s.contains("\"a\"\"b\""),
                "embedded quotes must be doubled (PowerShell convention); got: {s}"
            );
        }

        /// Win32 CommandLine MUST be null-terminated UTF-16. Without the
        /// trailing null, `CreateProcessAsUserW` reads past the buffer end.
        #[test]
        fn build_command_line_terminates_with_null() {
            let a = args(r"C:\foo.exe", vec!["a".to_string()]);
            let wide = build_command_line(&a);
            assert_eq!(
                wide.last(),
                Some(&0),
                "command line buffer must be null-terminated UTF-16"
            );
        }
    }
}

#[cfg(windows)]
fn main() {
    // Tracing → broker's stderr; nono.exe's WindowsSupervisorRuntime captures
    // broker stderr per existing log routing (Claude's Discretion: stderr-only,
    // no separate file).
    //
    // EnvFilter resolution: explicit `match` (not `unwrap_or_else`) — CLAUDE.md
    // § Unwrap Policy. RUST_LOG override → use it; otherwise default to "info".
    let env_filter = match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => tracing_subscriber::EnvFilter::new("info"),
    };
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(env_filter)
        .init();

    let raw: Vec<std::ffi::OsString> = std::env::args_os().collect();
    match broker::parse_args(&raw).and_then(broker::run) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            tracing::error!(error = %e, "broker: fatal error");
            eprintln!("nono-shell-broker: {e}");
            std::process::exit(2);
        }
    }
}
