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

    // D-29/D-30 (Phase 117-07): a per-layer force-unavailable seam for the
    // broker's OWN AppContainer/SECURITY_CAPABILITIES construction (RESEARCH
    // finding 2 — the real AppContainer child is spawned INSIDE the broker's
    // own process; nono-cli's suspended-spawn window on the direct/daemon arm
    // never sees it, so a hook there alone cannot exercise this layer's real
    // fail-direction). Compiled out of the default build entirely via
    // `layer-fault-injection` — there is no runtime (env var or otherwise)
    // toggle; a default release binary does not contain either the static or
    // the setter symbol. Mirrors `restricted_token.rs`'s
    // `RESTRICTED_TOKEN_FORCE_UNAVAILABLE` idiom exactly (Plan 06).
    #[cfg(feature = "layer-fault-injection")]
    static APP_CONTAINER_FORCE_UNAVAILABLE: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    /// Set the broker's AppContainer test-force-unavailable flag for the
    /// CINT-03 per-layer forced-unavailable test harness. Only exists when
    /// built with `--features layer-fault-injection`.
    ///
    /// `#[allow(dead_code)]`: this crate has no `[lib]` target and no
    /// blanket `#![allow(dead_code)]` (unlike `exec_strategy_windows/mod.rs`,
    /// which shields the sibling setters this idiom mirrors — Plan 06). The
    /// setter's only caller is the `#[cfg(test)]`-gated regression test below
    /// it, which `cargo clippy`/`cargo build` without `--tests` never
    /// compiles, so dead-code analysis cannot see that usage — a lint false
    /// positive, not genuinely-unused code (CLAUDE.md's "write tests that use
    /// it" is satisfied by `run_fails_when_app_container_forced_unavailable`).
    #[cfg(feature = "layer-fault-injection")]
    #[allow(dead_code)]
    pub(crate) fn force_app_container_unavailable(unavailable: bool) {
        APP_CONTAINER_FORCE_UNAVAILABLE.store(unavailable, std::sync::atomic::Ordering::Relaxed);
    }

    #[cfg(feature = "layer-fault-injection")]
    fn app_container_force_unavailable() -> bool {
        APP_CONTAINER_FORCE_UNAVAILABLE.load(std::sync::atomic::Ordering::Relaxed)
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

    /// The layer names THIS binary is able to attest against its own
    /// suspended child. Phase 117 review CR-03.2: the previous gate matched
    /// only the literal `"AppContainerProfile"` and **silently ignored every
    /// other name**, so `MandatoryIntegrityLabel` — the other layer
    /// `required_layers_for_broker()` emits — went completely unattested.
    /// An unrecognized required layer is now a fail-closed refusal, matching
    /// `attest_and_decide`'s own unrecognized-name rejection rather than
    /// diverging from it (WR-12).
    const BROKER_ATTESTABLE_LAYERS: &[&str] = &["AppContainerProfile", "MandatoryIntegrityLabel"];

    /// `SECURITY_MANDATORY_LOW_RID`. A confined broker grandchild's token
    /// must sit at or below Low integrity; Medium (0x2000) or above means
    /// the mandatory label this row claims was never lowered.
    const SECURITY_MANDATORY_LOW_RID: u32 = 0x0000_1000;

    /// Plan 117-11 (D-21/D-23): pure decision logic for the broker's own
    /// resume gate — the third and final D-21 gate insertion site (RESEARCH
    /// finding 2: the real Low-IL/AppContainer child is spawned INSIDE this
    /// process, a site `nono-cli`'s own `attest_and_decide` structurally
    /// cannot see). Factored out of [`run`] so the policy is unit-testable
    /// against synthetic probe results without a live spawned child —
    /// mirrors `crates/nono-cli/src/exec_strategy_windows/attestation.rs`'s
    /// `decide_from_entries` testing pattern.
    ///
    /// `required_layers_raw` is the raw `NONO_BROKER_REQUIRED_LAYERS` wire
    /// contract value (comma-separated `LayerId` `Debug`-format names,
    /// `crates/nono-cli/src/exec_strategy_windows/attestation.rs`'s
    /// `BROKER_REQUIRED_LAYERS_ENV_VAR`). The probes are the real
    /// `nono::attestation` calls against the broker's own suspended child
    /// (D-19: the supervisor attests, the confined process never does).
    ///
    /// # Phase 117 review CR-03 — what changed and why
    ///
    /// 1. **Fail-closed on an empty contract.** The caller now propagates a
    ///    missing env var as an error; an empty/blank value reaching here is
    ///    a misconfiguration, not "require nothing". The previous
    ///    `unwrap_or_default()` + "empty list means require-nothing" pairing
    ///    turned an absent variable into a silent no-op gate (CLAUDE.md
    ///    footgun #2).
    /// 2. **Every required name is dispatched or refused.** Names outside
    ///    [`BROKER_ATTESTABLE_LAYERS`] refuse resume instead of being
    ///    ignored.
    /// 3. **`MandatoryIntegrityLabel` is genuinely attested** by re-reading
    ///    the child token's integrity RID and requiring it to be at or below
    ///    `SECURITY_MANDATORY_LOW_RID`. This is the token-level observation
    ///    CR-01 removed from the CLI core, relocated to the one gate that
    ///    observes the right process.
    /// 4. **The AppContainer SID is compared to the expected per-run value**
    ///    (WR-01), not merely tested for presence — a child in a *different*
    ///    AppContainer is not the one the WFP `ALE_USER_ID` filters are
    ///    scoped to.
    ///
    /// `app_container_probe` is `None` on a broker spawn shape that creates
    /// no AppContainer (the legacy/PTY arm). Requiring `AppContainerProfile`
    /// on such a shape is a contract violation and refuses resume rather
    /// than passing vacuously.
    fn broker_resume_gate(
        required_layers_raw: &str,
        app_container_probe: Option<NonoResult<Option<String>>>,
        expected_app_container_sid: Option<&str>,
        integrity_probe: NonoResult<u32>,
    ) -> std::result::Result<(), String> {
        let required_layers: Vec<&str> = required_layers_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if required_layers.is_empty() {
            return Err(
                "NONO_BROKER_REQUIRED_LAYERS named no layers — refusing to resume an \
                 unattested child (fail-closed)"
                    .to_string(),
            );
        }
        for name in &required_layers {
            if !BROKER_ATTESTABLE_LAYERS.contains(name) {
                return Err(format!(
                    "required layer {name:?} cannot be attested by nono-shell-broker — \
                     refusing to resume (fail-closed)"
                ));
            }
        }

        if required_layers.contains(&"AppContainerProfile") {
            match app_container_probe {
                None => {
                    return Err(
                        "AppContainerProfile is required but this broker spawn shape creates \
                         no AppContainer — refusing to resume (fail-closed)"
                            .to_string(),
                    )
                }
                Some(Ok(Some(sid))) => match expected_app_container_sid {
                    Some(expected) if sid.eq_ignore_ascii_case(expected) => {}
                    Some(expected) => {
                        return Err(format!(
                            "AppContainerProfile required: the child package SID {sid} is not \
                             the per-run SID {expected} this broker registered"
                        ))
                    }
                    None => {
                        return Err(
                            "AppContainerProfile required but the broker has no expected \
                             package SID to compare against — refusing to resume (fail-closed)"
                                .to_string(),
                        )
                    }
                },
                Some(Ok(None)) => {
                    return Err(
                        "AppContainerProfile required: probe succeeded but found no \
                         AppContainer SID on the child token"
                            .to_string(),
                    )
                }
                Some(Err(e)) => {
                    return Err(format!("AppContainerProfile required: probe failed: {e}"))
                }
            }
        }

        if required_layers.contains(&"MandatoryIntegrityLabel") {
            match integrity_probe {
                Ok(rid) if rid <= SECURITY_MANDATORY_LOW_RID => {}
                Ok(rid) => {
                    return Err(format!(
                        "MandatoryIntegrityLabel required: the child token integrity RID is \
                         0x{rid:X}, above SECURITY_MANDATORY_LOW_RID \
                         (0x{SECURITY_MANDATORY_LOW_RID:X}) — the child is not Low-IL"
                    ))
                }
                Err(e) => {
                    return Err(format!(
                        "MandatoryIntegrityLabel required: integrity-level probe failed: {e}"
                    ))
                }
            }
        }

        Ok(())
    }

    // ── Phase 118 Plan 06 (D-15/D-27): this broker's own enforcement-receipt
    // pipeline. `nono-shell-broker.exe` writes a SECOND, independently-
    // verifiable receipt for the real confined grandchild it spawns —
    // distinct from `nono.exe`'s own `EntryPath::DirectCli` receipt for
    // `nono-shell-broker.exe` itself (Plan 118-07, not yet wired), the two
    // correlated only by `session_id` (D-15's "no cross-process lock"
    // design; each segment verifies independently). This crate cannot
    // import `nono-cli`'s `receipt_sink.rs` (separate binary, no
    // `nono-cli` dependency), so the sink-guard + chain-write code here is
    // HAND-ROLLED from core (`crates/nono`) primitives only — small
    // duplication vs. `receipt_sink.rs`, expected and acceptable per D-15.

    /// D-27: the single place a `LayerId` named on the wire contract's
    /// `required` channel gets a real per-row status, reusing the SAME
    /// probe results [`broker_resume_gate`] already computed (no
    /// re-probing). A required name outside this binary's two attestable
    /// rows (`AppContainerProfile`/`MandatoryIntegrityLabel`) fails toward
    /// `Unconfirmed` — `broker_resume_gate`'s own unrecognized-name arm
    /// already refuses to resume in that case, but this is a PURE census
    /// function that may be called independently of that refusal (e.g. the
    /// pre-gate `NONO_BROKER_REQUIRED_LAYERS`-absent branch), so it never
    /// assumes `Confirmed`.
    fn broker_required_row_status(
        name: &str,
        app_container_probe: &Option<NonoResult<Option<String>>>,
        expected_app_container_sid: Option<&str>,
        integrity_probe: &NonoResult<u32>,
    ) -> nono::LayerAttestationStatus {
        match name {
            "AppContainerProfile" => match app_container_probe {
                Some(Ok(Some(sid))) => match expected_app_container_sid {
                    Some(expected) if sid.eq_ignore_ascii_case(expected) => {
                        nono::LayerAttestationStatus::Confirmed
                    }
                    _ => nono::LayerAttestationStatus::Unconfirmed,
                },
                _ => nono::LayerAttestationStatus::Unconfirmed,
            },
            "MandatoryIntegrityLabel" => match integrity_probe {
                Ok(rid) if *rid <= SECURITY_MANDATORY_LOW_RID => {
                    nono::LayerAttestationStatus::Confirmed
                }
                _ => nono::LayerAttestationStatus::Unconfirmed,
            },
            _ => nono::LayerAttestationStatus::Unconfirmed,
        }
    }

    /// D-27's runtime fail-safe (T-118-18): builds a real 13-row census from
    /// the widened wire contract. A row named in `not_applicable_raw`
    /// classifies `NotApplicable`; a row named in `required_layers_raw`
    /// classifies via [`broker_required_row_status`]; a row named in
    /// NEITHER channel — a version-skewed nono-cli/nono-shell-broker pair,
    /// or a `LayerId` this broker binary predates — fails toward
    /// `Unconfirmed`, never silently dropped from the 13-row output and
    /// never silently promoted to `Confirmed`/`NotApplicable`.
    ///
    /// Deviation from this plan's literal `<interfaces>` signature
    /// (documented per the executor's deviation protocol; see
    /// `118-06-SUMMARY.md`): `expected_app_container_sid` is an added
    /// parameter, not in the plan's stated 4-argument shape. Without it,
    /// `AppContainerProfile` would classify `Confirmed` merely because SOME
    /// AppContainer SID was present on the child token — never checking it
    /// is the SAME per-run SID `broker_resume_gate` itself requires — which
    /// would let a receipt claim a layer active more strongly than the
    /// gate's own security decision actually verified (the exact
    /// "claims success while structurally incapable of reporting failure"
    /// anti-pattern `118-CONTEXT.md`'s pause-handoff framing names).
    fn broker_census(
        required_layers_raw: &str,
        not_applicable_raw: &str,
        app_container_probe: &Option<NonoResult<Option<String>>>,
        expected_app_container_sid: Option<&str>,
        integrity_probe: &NonoResult<u32>,
    ) -> Vec<nono::LayerReceiptRow> {
        let required: Vec<&str> = required_layers_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let not_applicable: Vec<&str> = not_applicable_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        nono::LayerId::ALL
            .iter()
            .map(|&id| {
                let name = format!("{id:?}");
                let status = if not_applicable.contains(&name.as_str()) {
                    nono::LayerAttestationStatus::NotApplicable
                } else if required.contains(&name.as_str()) {
                    broker_required_row_status(
                        &name,
                        app_container_probe,
                        expected_app_container_sid,
                        integrity_probe,
                    )
                } else {
                    nono::LayerAttestationStatus::Unconfirmed
                };
                nono::LayerReceiptRow { id, status }
            })
            .collect()
    }

    /// `%PROGRAMDATA%\nono\receipts` — byte-identical resolution to
    /// `nono-cli`'s `receipt_sink.rs::resolve_sink_dir` (D-15: both writers
    /// land in the SAME directory, correlated by session id, never sharing
    /// code — separate crates, separate binaries).
    ///
    /// # Phase 118 review CR-04: this class of defect, fixed identically here
    ///
    /// `%PROGRAMDATA%` is a user-settable environment variable — trusting it
    /// raw would let a user relocate the guard, the writer, and the write
    /// together, so the D-04 degrade/abort machinery downstream of a write
    /// failure would never even run. This mirrors `nono-cli`'s
    /// `receipt_sink::resolve_sink_dir` fix byte-for-byte (see that
    /// function's doc for the full rationale): FAILS CLOSED (`Err`) unless
    /// the resolved base is both absolute AND not owned by the current
    /// (invoking) user — the real, machine-wide `%ProgramData%` root is
    /// provisioned by the OS installer and owned by a system principal, never
    /// by an ordinary interactive user. `nono::path_is_owned_by_current_user`
    /// is the same core primitive the CLI-side fix reuses; this crate already
    /// depends on `nono` core, so no new dependency is introduced.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `%PROGRAMDATA%` (or its `C:\ProgramData` fallback)
    /// resolves to a relative path, if its ownership cannot be determined, or
    /// if it is owned by the current user (a redirection signal).
    fn broker_receipt_sink_dir() -> NonoResult<PathBuf> {
        let base = PathBuf::from(
            std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string()),
        );
        if !base.is_absolute() {
            return Err(NonoError::Snapshot(format!(
                "broker receipt sink: %PROGRAMDATA% did not resolve to an absolute path ({}) — \
                 refusing to write receipts to a relative, redirectable location",
                base.display()
            )));
        }
        let owned_by_current_user = nono::path_is_owned_by_current_user(&base).map_err(|e| {
            NonoError::Snapshot(format!(
                "broker receipt sink: could not determine the owner of %PROGRAMDATA% ({}): {e} \
                 — refusing to trust an unverifiable machine-wide root",
                base.display()
            ))
        })?;
        if owned_by_current_user {
            return Err(NonoError::Snapshot(format!(
                "broker receipt sink: %PROGRAMDATA% ({}) is owned by the current user, not a \
                 system principal — this is not the real, machine-wide ProgramData root and \
                 would let a user-set environment variable silently relocate the receipt sink \
                 (defeating the admin-only RequireReceipts control); refusing to write receipts \
                 there",
                base.display()
            )));
        }
        Ok(base.join("nono").join("receipts"))
    }

    /// D-08's both-not-either guard (DENY ACE + unconditional `NO_READ_UP`
    /// mandatory label), hand-rolled from core primitives — mirrors
    /// `receipt_sink.rs::ensure_sink_guarded`'s mask values exactly.
    /// Idempotent: safe to reapply even if `nono.exe`'s own receipt write
    /// (Plan 118-07) already guarded this directory for this launch.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the directory cannot be created, if a DENY ACE
    /// cannot be applied for a `Some` SID, or if the mandatory label cannot
    /// be applied.
    fn ensure_broker_receipt_sink_guarded(
        dir: &std::path::Path,
        package_sid: Option<&str>,
    ) -> NonoResult<()> {
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        };
        use windows_sys::Win32::System::SystemServices::{
            SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP, SYSTEM_MANDATORY_LABEL_NO_READ_UP,
        };
        const DENY_MASK: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_EXECUTE | DELETE;
        const LABEL_MASK: u32 =
            SYSTEM_MANDATORY_LABEL_NO_READ_UP | SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP;

        std::fs::create_dir_all(dir).map_err(|e| {
            NonoError::Snapshot(format!(
                "broker receipt sink: failed to create sink directory {}: {e}",
                dir.display()
            ))
        })?;
        if let Some(sid) = package_sid {
            nono::deny_sid_on_path(dir, sid, DENY_MASK)?;
        }
        nono::try_set_mandatory_label(dir, LABEL_MASK)?;
        Ok(())
    }

    /// One JSONL line's envelope: the receipt plus the chain-linkage fields
    /// a consumer needs to recompute-and-compare (`nono receipt verify`,
    /// D-10). Field names deliberately match `receipt_sink.rs`'s
    /// `ReceiptRecord` byte-for-byte, so Plan 118-09's command family can
    /// read either writer's output with one code path.
    #[derive(serde::Serialize)]
    struct BrokerReceiptRecord<'a> {
        sequence: u64,
        prev_head: Option<nono::undo::ContentHash>,
        leaf_hash: nono::undo::ContentHash,
        chain_head: nono::undo::ContentHash,
        receipt: &'a nono::EnforcementReceipt,
    }

    /// Phase 118 review CR-02: a READ-side, OWNED mirror of
    /// [`BrokerReceiptRecord`]'s chain-linkage fields, used ONLY to resume an
    /// existing segment's chain state in [`BrokerReceiptWriter::new`].
    /// `BrokerReceiptRecord` itself cannot derive `Deserialize` — its
    /// `receipt` field is a borrowed `&'a nono::EnforcementReceipt`, and
    /// deserialization needs owned data. `serde(deny_unknown_fields)` is
    /// deliberately NOT set: this struct only needs `sequence` and
    /// `chain_head` off the tail line, and ignoring the (larger) `receipt`
    /// field it does not declare is exactly what an ignore-unknown-fields
    /// deserialize is for.
    #[derive(serde::Deserialize)]
    struct BrokerReceiptRecordTail {
        sequence: u64,
        chain_head: nono::undo::ContentHash,
    }

    /// Hand-rolled, single-writer, KEYLESS (D-25: no `key` field) chain
    /// state + JSONL append writer.
    ///
    /// Unlike `receipt_sink.rs`'s `ReceiptWriter`, this carries NO mutex:
    /// this binary's own receipt-write call site inside [`run`] is
    /// single-threaded by construction — one broker process handles exactly
    /// one launch, one suspended child, one gate decision, and every
    /// `write_receipt` call happens sequentially on `run`'s own call stack,
    /// never from a second concurrent caller — so the WR-21 "hold the mutex
    /// across the full build+advance+write sequence" discipline degenerates
    /// to plain sequential code with nothing to race.
    #[derive(Debug)]
    struct BrokerReceiptWriter {
        head: [u8; 32],
        sequence: u64,
        file_path: PathBuf,
    }

    impl BrokerReceiptWriter {
        /// Opens (creating if absent) `<sink_dir>/<session_id>.broker.jsonl`
        /// in append mode once, to fail fast if the sink is unwritable.
        ///
        /// # Phase 118 review CR-02: RESUME the existing chain, never assume genesis
        ///
        /// Mirrors `receipt_sink.rs`'s `ReceiptWriter::new` fix exactly (see
        /// that function's doc for the full rationale): reads the segment's
        /// LAST record, if any, before opening for append, and resumes from
        /// its `sequence + 1` / `chain_head` rather than unconditionally
        /// starting at `sequence: 0` — which every prior call site (this
        /// writer is reconstructed fresh per emission) made the SECOND record
        /// on any segment. Fails CLOSED if an existing tail cannot be parsed.
        ///
        /// # Errors
        ///
        /// Returns `Err` if an existing segment cannot be read for a reason
        /// other than "does not exist yet", if an existing segment's last
        /// record cannot be parsed, or if the sink file cannot be
        /// created/opened.
        fn new(session_id: &str, sink_dir: &std::path::Path) -> NonoResult<Self> {
            // D-05/CLAUDE.md path-security: never trust a wire-derived
            // string directly in a filename. `NONO_SESSION_ID` is
            // "audit-correlation only; accept empty" per the house
            // convention `nono::supervisor::aipc_sdk` documents for this
            // exact env var name — an unsafe/absent value here degrades to
            // a fixed, safe name rather than failing the whole receipt
            // write closed.
            let safe_id = if session_id.is_empty()
                || !session_id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                "unknown-session"
            } else {
                session_id
            };
            let file_path = sink_dir.join(format!("{safe_id}.broker.jsonl"));

            let (head, sequence) = match std::fs::read_to_string(&file_path) {
                Ok(existing) => match existing.lines().rev().find(|l| !l.trim().is_empty()) {
                    Some(last_line) => {
                        let last: BrokerReceiptRecordTail = serde_json::from_str(last_line)
                            .map_err(|e| {
                                NonoError::Snapshot(format!(
                                    "broker receipt sink: refusing to append to {} — its last \
                                     record could not be parsed (truncated or corrupt): {e}",
                                    file_path.display()
                                ))
                            })?;
                        (*last.chain_head.as_bytes(), last.sequence.saturating_add(1))
                    }
                    None => ([0u8; 32], 0),
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => ([0u8; 32], 0),
                Err(e) => {
                    return Err(NonoError::Snapshot(format!(
                        "broker receipt sink: failed to read existing sink file {} to resume its \
                         chain: {e}",
                        file_path.display()
                    )))
                }
            };

            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .map(|_file| ())
                .map_err(|e| {
                    NonoError::Snapshot(format!(
                        "broker receipt sink: failed to create/open sink file {}: {e}",
                        file_path.display()
                    ))
                })?;
            Ok(Self {
                head,
                sequence,
                file_path,
            })
        }

        /// Serialize `receipt`, advance the keyless chain, and append one
        /// JSONL record. Chain state only advances AFTER the file write
        /// durably succeeds, so a failed write never desyncs in-memory
        /// state from disk.
        ///
        /// # Errors
        ///
        /// Returns `Err` if `receipt` cannot be serialized, or if the sink
        /// file cannot be opened, written, or flushed.
        fn write_receipt(&mut self, receipt: &nono::EnforcementReceipt) -> NonoResult<()> {
            use std::io::Write as _;

            let event_bytes = serde_json::to_vec(receipt).map_err(|e| {
                NonoError::Snapshot(format!(
                    "broker receipt sink: failed to serialize enforcement receipt: {e}"
                ))
            })?;
            let leaf_hash = nono::hash_receipt_event(&event_bytes);
            let prev_head = if self.sequence == 0 {
                None
            } else {
                Some(nono::undo::ContentHash::from_bytes(self.head))
            };
            let chain_head = nono::hash_receipt_chain(prev_head.as_ref(), &leaf_hash);

            let record = BrokerReceiptRecord {
                sequence: self.sequence,
                prev_head,
                leaf_hash,
                chain_head,
                receipt,
            };
            let mut line = serde_json::to_vec(&record).map_err(|e| {
                NonoError::Snapshot(format!(
                    "broker receipt sink: failed to serialize receipt record: {e}"
                ))
            })?;
            line.push(b'\n');

            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
                .map_err(|e| {
                    NonoError::Snapshot(format!(
                        "broker receipt sink: failed to open sink file {} for append: {e}",
                        self.file_path.display()
                    ))
                })?;
            file.write_all(&line)
                .and_then(|()| file.flush())
                .map_err(|e| {
                    NonoError::Snapshot(format!(
                        "broker receipt sink: failed to append receipt record to {}: {e}",
                        self.file_path.display()
                    ))
                })?;

            self.head = *chain_head.as_bytes();
            self.sequence = self.sequence.saturating_add(1);
            Ok(())
        }
    }

    /// D-03/D-15: resolves+guards the shared sink directory, assembles the
    /// receipt from an already-built `census`, and writes it. Called at
    /// EVERY point [`run`] decides `Ran` or `Refused`, so a receipt exists
    /// before the child could ever execute an instruction (`Ran`) or
    /// definitely never will (`Refused`).
    ///
    /// D-04: an emitter failure degrades VISIBLY (a `tracing::warn!`)
    /// rather than aborting an already-decided session outcome — this
    /// function has no `Result` return for exactly that reason; the
    /// broker's actual resume/terminate decision is governed entirely by
    /// `broker_resume_gate`, never by whether the receipt write succeeded.
    fn record_broker_receipt(
        session_id: &str,
        pid: u32,
        outcome: nono::SessionOutcome,
        census: Vec<nono::LayerReceiptRow>,
        package_sid_for_guard: Option<&str>,
    ) {
        let receipt = nono::EnforcementReceipt {
            schema_version: 1,
            session_id: session_id.to_string(),
            pid,
            entry_path: nono::EntryPath::Broker,
            token_arm: None,
            outcome,
            layers: census,
        };
        // Phase 118 review CR-04: `broker_receipt_sink_dir` is now fallible
        // (a redirected/untrustworthy `%PROGRAMDATA%` is rejected). This
        // function already has no `Result` return (D-04: an emitter failure
        // degrades VISIBLY rather than aborting an already-decided session
        // outcome — see this function's own doc), so a resolution failure is
        // handled with the SAME visible-degrade posture as every other
        // failure branch below.
        let sink_dir = match broker_receipt_sink_dir() {
            Ok(dir) => dir,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "broker: failed to resolve a trustworthy receipt sink directory — degrading \
                     visibly (D-04), not aborting the already-decided session outcome"
                );
                return;
            }
        };
        if let Err(e) = ensure_broker_receipt_sink_guarded(&sink_dir, package_sid_for_guard) {
            tracing::warn!(
                error = %e,
                "broker: failed to guard the receipt sink directory — degrading visibly \
                 (D-04), not aborting the already-decided session outcome"
            );
            return;
        }
        match BrokerReceiptWriter::new(session_id, &sink_dir) {
            Ok(mut writer) => {
                if let Err(e) = writer.write_receipt(&receipt) {
                    tracing::warn!(
                        error = %e,
                        "broker: failed to write enforcement receipt — degrading visibly (D-04)"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "broker: failed to construct the enforcement-receipt writer — degrading \
                     visibly (D-04)"
                );
            }
        }
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

        // D-30 (Phase 117-07): the force-unavailable seam only exists when
        // built with `--features layer-fault-injection` — in a default build
        // this branch and the function it calls are not compiled in at all,
        // so there is no runtime toggle to read. Short-circuits BEFORE the
        // real SECURITY_CAPABILITIES/PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
        // construction so a forced-unavailable AppContainer never silently
        // spawns the child with a different (unconfined) attribute set — it
        // refuses to launch at all (T-117-14), matching the crate's existing
        // fail-closed doc comment (`main.rs:84-88`). Only meaningful on the
        // AppContainer path (`app_container_sid.is_some()`) — the legacy/PTY
        // path never constructs `SECURITY_CAPABILITIES` at all.
        #[cfg(feature = "layer-fault-injection")]
        if app_container_sid.is_some() && app_container_force_unavailable() {
            unsafe {
                // SAFETY: attr_list was initialized successfully above (the
                // InitializeProcThreadAttributeList + HANDLE_LIST update both
                // succeeded to reach this point); must be freed before bailing.
                DeleteProcThreadAttributeList(attr_list);
            }
            return Err(NonoError::LayerAttestationFailed {
                layer: "AppContainerProfile".into(),
                reason: "forced unavailable by test seam".into(),
            });
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
                    1, // bInheritHandles=TRUE (HANDLE_LIST gates)
                    // Phase 117 review CR-03.1: CREATE_SUSPENDED added so the
                    // legacy/PTY arm has the SAME pre-first-instruction window
                    // the AppContainer arm already had. Without it this arm had
                    // no resume point at all, which is why the D-21 gate ended up
                    // nested inside `if is_app_container` and never ran here.
                    // Still no new-console flag (D-01).
                    EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
                    std::ptr::null(), // lpEnvironment: inherit broker env
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

        // Phase 118 Plan 06 gap-closure (orchestrator follow-up, D-03/RCPT-01):
        // read the wire contract + correlation id EARLY — before the two
        // label-application fail-closed branches immediately below — so a
        // confined session that refuses HERE also gets a receipt. RCPT-01 is
        // "every confined session emits a receipt"; a refuse path that
        // emits nothing is exactly the gap this closes. Best-effort reads
        // (never fail-closed): these two branches' own security decision
        // (terminate + `return Err`) is completely unaffected by whether
        // this wire-contract data is present — it only shapes the CENSUS a
        // receipt carries, never whether the child is terminated.
        let required_layers_raw_for_early_census =
            std::env::var("NONO_BROKER_REQUIRED_LAYERS").unwrap_or_default();
        let not_applicable_raw =
            std::env::var("NONO_BROKER_NOT_APPLICABLE_LAYERS").unwrap_or_default();
        let session_id = std::env::var("NONO_SESSION_ID").unwrap_or_default();
        let pid = process_info.dwProcessId;

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
                // Phase 118 Plan 06 gap-closure: a confined session that
                // REFUSED here still gets a receipt (D-03/RCPT-01). Probe
                // read-only (D-19: real OS observation, never fabricated)
                // for the most truthful census this earliest branch can
                // produce — `probe_app_container_sid`/`probe_integrity_level`
                // open their OWN token handle internally, independent of
                // this branch's own failed `OpenProcessToken` call, so a
                // real (possibly also-failing) probe result is genuinely
                // available. D-13: an unknown row stays Unconfirmed/
                // NotApplicable, never fabricated Confirmed. Content-free
                // (D-14): only the bare `GetLastError` numeric code is
                // observed above, and it is never placed on the receipt —
                // the receipt-write path below takes no error-string input
                // at all. Receipt-write failure degrades visibly
                // (`record_broker_receipt`'s own D-04 contract) and never
                // masks this branch's own `TerminateProcess` + `return Err`.
                let app_container_probe = Some(nono::attestation::probe_app_container_sid(
                    child_process.raw(),
                ));
                let integrity_probe = nono::attestation::probe_integrity_level(child_process.raw());
                let expected_sid: Option<String> = match app_container_sid.as_ref() {
                    Some(owned) => nono::package_sid_to_string(owned).ok(),
                    None => None,
                };
                let census = broker_census(
                    &required_layers_raw_for_early_census,
                    &not_applicable_raw,
                    &app_container_probe,
                    expected_sid.as_deref(),
                    &integrity_probe,
                );
                record_broker_receipt(
                    &session_id,
                    pid,
                    nono::SessionOutcome::Refused,
                    census,
                    expected_sid.as_deref(),
                );
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
                // Phase 118 Plan 06 gap-closure: same reasoning as the
                // `OpenProcessToken` branch above — `OpenProcessToken`
                // itself succeeded here, so the probes below observe the
                // REAL (still-unlabeled, i.e. genuinely not-yet-Low-IL)
                // state of the child token — a truthful, not fabricated,
                // Unconfirmed outcome for `MandatoryIntegrityLabel`.
                let app_container_probe = Some(nono::attestation::probe_app_container_sid(
                    child_process.raw(),
                ));
                let integrity_probe = nono::attestation::probe_integrity_level(child_process.raw());
                let expected_sid: Option<String> = match app_container_sid.as_ref() {
                    Some(owned) => nono::package_sid_to_string(owned).ok(),
                    None => None,
                };
                let census = broker_census(
                    &required_layers_raw_for_early_census,
                    &not_applicable_raw,
                    &app_container_probe,
                    expected_sid.as_deref(),
                    &integrity_probe,
                );
                record_broker_receipt(
                    &session_id,
                    pid,
                    nono::SessionOutcome::Refused,
                    census,
                    expected_sid.as_deref(),
                );
                unsafe {
                    // SAFETY: see above — fail closed by terminating the child.
                    windows_sys::Win32::System::Threading::TerminateProcess(child_process.raw(), 1);
                }
                return Err(e);
            }
        }

        // Plan 117-11 (D-21/D-23, D-02): the broker's own attestation gate.
        // The real Low-IL/AppContainer child is spawned INSIDE this process —
        // a site `nono-cli`'s own `attest_and_decide` (Plan 08/10)
        // structurally cannot see (see that module's Blocker-1 doc comment,
        // `crates/nono-cli/src/exec_strategy_windows/attestation.rs`).
        //
        // Phase 117 review CR-03.1 (class coverage): this block used to sit
        // INSIDE `if is_app_container`, so the legacy/PTY broker arm — which
        // nono-cli nevertheless sends `NONO_BROKER_REQUIRED_LAYERS` on — was
        // never gated at all. It is now hoisted to cover BOTH spawn shapes,
        // and both are spawned `CREATE_SUSPENDED` so the gate always runs
        // before the child executes its first instruction.
        //
        // This reads the wire-contract env var `NONO_BROKER_REQUIRED_LAYERS`
        // — the SAME string literal as that module's
        // `BROKER_REQUIRED_LAYERS_ENV_VAR` constant; this crate cannot import
        // that constant directly (separate binary, no dependency on
        // `nono-sandbox-cli`), so the two are paired by this comment, by the
        // literal string matching, and by `nono-cli`'s
        // `broker_wire_contract_conformance` test (WR-12).
        //
        // CR-03.3: an absent/unreadable variable is fail-CLOSED. The previous
        // `unwrap_or_default()` turned it into `""`, which the gate read as
        // "require nothing" — CLAUDE.md footgun #2 applied to the gate's sole
        // input.
        //
        // D-19: the supervisor (the broker, here) attests; the confined
        // process never does — every probe reads the real child token from
        // outside via the shared `crates/nono` primitives.
        //
        // Phase 118 Plan 06 (D-15/D-27): the sibling `NotApplicable` wire
        // channel and the session-correlation id — `not_applicable_raw`,
        // `session_id`, `pid` — were already read above (before the
        // label-application fail-closed branches, gap-closure), but feed
        // this broker's own receipt CENSUS only — never its resume/
        // terminate DECISION, which stays governed exclusively by
        // `NONO_BROKER_REQUIRED_LAYERS`'s existing fail-closed read below
        // (D-27: an absent/unreadable value here degrades the RECEIPT —
        // rows named in neither channel fail toward `Unconfirmed` — it
        // never blocks resume). `NONO_SESSION_ID` is "audit-correlation
        // only; accept empty" per the house convention
        // `nono::supervisor::aipc_sdk` documents for this exact env var
        // name; D-15's correlation-by-session-id degrades gracefully (an
        // absent id still produces a valid, if uncorrelated, receipt)
        // rather than refusing the launch over a record-quality concern.
        let required_layers_raw = std::env::var("NONO_BROKER_REQUIRED_LAYERS").map_err(|_| {
            NonoError::SandboxInit(
                "NONO_BROKER_REQUIRED_LAYERS absent or unreadable — refusing to resume an \
                 unattested child (fail-closed)"
                    .into(),
            )
        });
        let (gate_result, census, expected_sid) = match required_layers_raw {
            Err(e) => {
                // Phase 118 Plan 06 (D-03): the gate never even ran — still
                // probe (read-only) for the most truthful census this
                // branch can produce, and RECORD `Refused` BEFORE
                // terminating, matching the other two producers' ordering
                // guarantee (a receipt survives even this earliest
                // fail-closed refusal).
                let app_container_probe = if is_app_container {
                    Some(nono::attestation::probe_app_container_sid(
                        child_process.raw(),
                    ))
                } else {
                    None
                };
                let integrity_probe = nono::attestation::probe_integrity_level(child_process.raw());
                let expected_sid: Option<String> = match app_container_sid.as_ref() {
                    Some(owned) => nono::package_sid_to_string(owned).ok(),
                    None => None,
                };
                let census = broker_census(
                    "",
                    &not_applicable_raw,
                    &app_container_probe,
                    expected_sid.as_deref(),
                    &integrity_probe,
                );
                record_broker_receipt(
                    &session_id,
                    pid,
                    nono::SessionOutcome::Refused,
                    census,
                    expected_sid.as_deref(),
                );
                unsafe {
                    // SAFETY: fail closed — terminate rather than resume an
                    // unattested child (D-22).
                    windows_sys::Win32::System::Threading::TerminateProcess(child_process.raw(), 1);
                }
                return Err(e);
            }
            Ok(raw) => {
                let app_container_probe = if is_app_container {
                    Some(nono::attestation::probe_app_container_sid(
                        child_process.raw(),
                    ))
                } else {
                    None
                };
                let expected_sid: Option<String> = match app_container_sid.as_ref() {
                    Some(owned) => match nono::package_sid_to_string(owned) {
                        Ok(s) => Some(s),
                        // A failure to render the SID we ourselves derived is
                        // an attestation failure, not a reason to skip the
                        // comparison: leaving it `None` makes the gate refuse.
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "broker: could not render the per-run package SID for comparison"
                            );
                            None
                        }
                    },
                    None => None,
                };
                let integrity_probe = nono::attestation::probe_integrity_level(child_process.raw());
                // Phase 118 Plan 06: build this broker's own receipt census
                // from the SAME probe results the gate below consumes — no
                // re-probing (this plan's interfaces requirement).
                let census = broker_census(
                    &raw,
                    &not_applicable_raw,
                    &app_container_probe,
                    expected_sid.as_deref(),
                    &integrity_probe,
                );
                let gate = broker_resume_gate(
                    &raw,
                    app_container_probe,
                    expected_sid.as_deref(),
                    integrity_probe,
                );
                (gate, census, expected_sid)
            }
        };
        if let Err(reason) = gate_result {
            record_broker_receipt(
                &session_id,
                pid,
                nono::SessionOutcome::Refused,
                census,
                expected_sid.as_deref(),
            );
            unsafe {
                // SAFETY: fail closed — terminate rather than resume an
                // unattested child (D-22).
                windows_sys::Win32::System::Threading::TerminateProcess(child_process.raw(), 1);
            }
            return Err(NonoError::SandboxInit(format!(
                "startup self-attestation failed for the broker suspended child: {reason}"
            )));
        }

        // Phase 118 Plan 06 (D-03): write the `Ran` receipt BEFORE
        // `ResumeThread` — the same ordering guarantee `nono.exe`/
        // `nono-agentd.exe` use, so a receipt exists before the child could
        // ever execute a single instruction. `census.clone()`: the
        // ResumeThread-failure branch immediately below needs the SAME
        // census again for its own (rare) corrective `Refused` record.
        record_broker_receipt(
            &session_id,
            pid,
            nono::SessionOutcome::Ran,
            census.clone(),
            expected_sid.as_deref(),
        );

        // CR-03.1: resume on BOTH arms — the legacy/PTY arm is now spawned
        // suspended too, so it needs the same resume.
        let resumed = unsafe {
            // SAFETY: child_thread.raw() is the valid main-thread handle of
            // the suspended child. ResumeThread returns the previous suspend
            // count, or u32::MAX (-1) on error.
            ResumeThread(child_thread.raw())
        };
        if resumed == u32::MAX {
            let err = unsafe { GetLastError() };
            // Phase 118 Plan 06: the gate decided `Ran` and that record is
            // already durable, but `ResumeThread` itself then failed — the
            // child never actually executes. Append a SECOND, correcting
            // `Refused` record to the SAME per-session chain (D-15's
            // append-only, per-writer segment: never a replaced record, a
            // truthful second one) — a governance consumer reading receipts
            // for this session_id should take the LAST record as
            // authoritative.
            record_broker_receipt(
                &session_id,
                pid,
                nono::SessionOutcome::Refused,
                census,
                expected_sid.as_deref(),
            );
            unsafe {
                // SAFETY: fail closed — terminate rather than leave suspended.
                windows_sys::Win32::System::Threading::TerminateProcess(child_process.raw(), 1);
            }
            return Err(NonoError::SandboxInit(format!(
                "ResumeThread on the broker child failed (GetLastError={err})"
            )));
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

    /// Plan 117-11 (D-21/D-23/D-02, T-117-05/T-117-02/T-117-21) + Phase 117
    /// review CR-03: pins the pure `broker_resume_gate` decision logic — the
    /// third and final D-21 gate insertion site — against synthetic probe
    /// results, no live spawned child required. Mirrors
    /// `decide_from_entries`'s testing pattern in
    /// `crates/nono-cli/src/exec_strategy_windows/attestation.rs`.
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod broker_resume_gate_tests {
        use super::*;
        use nono::NonoError;

        const EXPECTED_SID: &str = "S-1-15-2-1-2-3";

        fn low_il() -> NonoResult<u32> {
            Ok(SECURITY_MANDATORY_LOW_RID)
        }

        fn medium_il() -> NonoResult<u32> {
            Ok(0x0000_2000)
        }

        fn probe_err() -> NonoError {
            NonoError::LayerAttestationFailed {
                layer: "synthetic".into(),
                reason: "synthetic".into(),
            }
        }

        /// CR-03.3: an empty wire contract is a MISCONFIGURATION, not
        /// "require nothing". The pre-fix pairing of `unwrap_or_default()`
        /// with an "empty list permits resume" gate turned an absent
        /// environment variable into a no-op gate (CLAUDE.md footgun #2).
        #[test]
        fn empty_required_layers_refuses_resume() {
            let decision = broker_resume_gate("", None, None, low_il());
            assert!(
                decision.is_err(),
                "an empty required-layers list must refuse resume (fail-closed), got {decision:?}"
            );
            assert!(broker_resume_gate("  ,  ", None, None, low_il()).is_err());
        }

        /// CR-03.2: a required layer this binary cannot attest must refuse
        /// resume rather than being silently ignored. The pre-fix gate
        /// matched only the literal `"AppContainerProfile"` and dropped
        /// every other name on the floor.
        #[test]
        fn unattestable_required_layer_refuses_resume() {
            let decision = broker_resume_gate("RestrictedToken", None, None, low_il());
            assert!(
                decision.is_err(),
                "a required layer outside BROKER_ATTESTABLE_LAYERS must refuse resume, \
                 got {decision:?}"
            );
        }

        /// CR-03.2: `MandatoryIntegrityLabel` — the layer
        /// `required_layers_for_broker()` emits on BOTH broker arms and the
        /// pre-fix gate ignored entirely — is now genuinely attested against
        /// the child token's integrity RID, and CAN return the negative.
        #[test]
        fn mandatory_integrity_label_is_attested_and_can_fail() {
            assert!(
                broker_resume_gate("MandatoryIntegrityLabel", None, None, low_il()).is_ok(),
                "a Low-IL child token must satisfy MandatoryIntegrityLabel"
            );
            assert!(
                broker_resume_gate("MandatoryIntegrityLabel", None, None, medium_il()).is_err(),
                "a Medium-IL child token must NOT satisfy MandatoryIntegrityLabel — this is \
                 the negative the pre-CR-01/CR-03 code could never produce"
            );
            assert!(
                broker_resume_gate("MandatoryIntegrityLabel", None, None, Err(probe_err()))
                    .is_err(),
                "a failed integrity probe must refuse resume (fail-closed)"
            );
        }

        /// D-21/D-22: when the wire contract requires `AppContainerProfile`
        /// and the probe is unconfirmed — either an authoritative "no SID
        /// present" (`Ok(None)`) or the probe call itself failing (`Err`) —
        /// resume must be refused.
        #[test]
        fn tightened_unconfirmed_app_container_probe_refuses_resume() {
            assert!(broker_resume_gate(
                "AppContainerProfile",
                Some(Ok(None)),
                Some(EXPECTED_SID),
                low_il()
            )
            .is_err());
            assert!(broker_resume_gate(
                "AppContainerProfile",
                Some(Err(probe_err())),
                Some(EXPECTED_SID),
                low_il()
            )
            .is_err());
        }

        /// T-117-21: a probe that positively confirms THE EXPECTED
        /// AppContainer SID permits resume — the genuine attestation
        /// Blocker-1 closes, on the real process.
        #[test]
        fn tightened_confirmed_app_container_probe_permits_resume() {
            assert!(broker_resume_gate(
                "AppContainerProfile",
                Some(Ok(Some(EXPECTED_SID.to_string()))),
                Some(EXPECTED_SID),
                low_il()
            )
            .is_ok());
        }

        /// WR-01: a child in a DIFFERENT AppContainer — one the WFP
        /// `ALE_USER_ID` filters are not scoped to — must not attest
        /// identically to the correct one.
        #[test]
        fn app_container_sid_must_match_the_expected_per_run_value() {
            let decision = broker_resume_gate(
                "AppContainerProfile",
                Some(Ok(Some("S-1-15-2-9-9-9".to_string()))),
                Some(EXPECTED_SID),
                low_il(),
            );
            assert!(
                decision.is_err(),
                "a foreign AppContainer SID must refuse resume, got {decision:?}"
            );
            // No expected value to compare against is also fail-closed.
            assert!(broker_resume_gate(
                "AppContainerProfile",
                Some(Ok(Some(EXPECTED_SID.to_string()))),
                None,
                low_il()
            )
            .is_err());
        }

        /// CR-03.1: requiring `AppContainerProfile` on a broker spawn shape
        /// that creates no AppContainer (the legacy/PTY arm, where
        /// `app_container_probe` is `None`) is a contract violation and
        /// refuses resume rather than passing vacuously.
        #[test]
        fn app_container_required_on_a_non_app_container_shape_refuses_resume() {
            assert!(broker_resume_gate("AppContainerProfile", None, None, low_il()).is_err());
        }

        /// The env var value is comma-separated and whitespace-tolerant, so
        /// `required_layers_for_broker`'s `join(",")` output (no spaces) and
        /// a hand-edited multi-value list (with spaces) both parse
        /// correctly.
        #[test]
        fn required_layers_list_is_comma_split_and_trimmed() {
            assert!(
                broker_resume_gate(
                    "MandatoryIntegrityLabel, AppContainerProfile ",
                    Some(Ok(Some(EXPECTED_SID.to_string()))),
                    Some(EXPECTED_SID),
                    low_il()
                )
                .is_ok(),
                "a whitespace-padded multi-value list must parse"
            );
            assert!(
                broker_resume_gate(
                    "MandatoryIntegrityLabel, AppContainerProfile ",
                    Some(Ok(None)),
                    Some(EXPECTED_SID),
                    low_il()
                )
                .is_err(),
                "AppContainerProfile embedded in a whitespace-padded list must still be \
                 recognized"
            );
        }

        /// Phase 117-12 (D-24): measures the broker-arm gate's real
        /// wall-clock cost — the real `nono::attestation` Win32 calls against
        /// a real process handle (`GetCurrentProcess()`) plus
        /// `broker_resume_gate`'s own pure decision logic. Not a
        /// bound-asserting benchmark (see
        /// `exec_strategy_windows/attestation.rs`'s identical D-24 timing
        /// test for the same reasoning) — produces a real number for the
        /// SPEC's Latency budget table's Broker-arm row.
        #[test]
        fn broker_resume_gate_latency_with_real_probes() {
            use nono::attestation::{probe_app_container_sid, probe_integrity_level};
            use windows_sys::Win32::System::Threading::GetCurrentProcess;

            let real_process = unsafe { GetCurrentProcess() };
            let start = std::time::Instant::now();
            let probe_result = probe_app_container_sid(real_process);
            let integrity = probe_integrity_level(real_process);
            let _ = broker_resume_gate(
                "AppContainerProfile,MandatoryIntegrityLabel",
                Some(probe_result),
                Some(EXPECTED_SID),
                integrity,
            );
            let elapsed = start.elapsed();
            eprintln!(
                "D-24 measured broker-arm gate cost (probe_app_container_sid + \
                 probe_integrity_level + broker_resume_gate, real GetCurrentProcess() \
                 handle): {elapsed:?}"
            );
            assert!(
                elapsed < std::time::Duration::from_millis(250),
                "broker-arm gate took {elapsed:?}, exceeding the generous 250ms sanity bound"
            );
        }
    }

    /// Plan 117-11 Warning-10 fix: RAII guard that saves
    /// `NONO_BROKER_REQUIRED_LAYERS`'s prior value on construction and
    /// restores it (or removes the var if it was previously absent) in
    /// `Drop`, so test env-var mutation cannot leak across the shared
    /// `--test-threads=1` process even on a panicking assertion — per
    /// CLAUDE.md's "Environment variables in tests" save/restore rule
    /// (T-117-23).
    #[cfg(test)]
    struct EnvVarGuard {
        key: &'static str,
        prior: Option<String>,
    }

    // `clippy.toml`'s workspace-wide `disallowed-methods` entry for
    // `std::env::{set_var,remove_var}` points callers at
    // `crate::test_env::EnvVarGuard` (`crates/nono-cli/src/test_env.rs`) —
    // unusable here: `nono-cli` is a binary-only crate with no `[lib]`
    // target, so `nono-shell-broker` cannot depend on it (same crate-
    // boundary constraint the module-level doc comment on
    // `app_container_resume_gate` describes for `BROKER_REQUIRED_LAYERS_ENV_VAR`).
    // This `EnvVarGuard` IS this crate's own local equivalent safe wrapper
    // (mirrors `nono-cli`'s `#[allow(clippy::disallowed_methods)]` idiom on
    // its own guard's impl blocks) — save/restore-disciplined per CLAUDE.md's
    // "Environment variables in tests" rule (T-117-23, Warning-10 fix).
    #[cfg(test)]
    #[allow(clippy::disallowed_methods)]
    impl EnvVarGuard {
        /// Sets `key` to `value`, saving whatever was previously there
        /// (including "nothing") so `Drop` can restore it exactly.
        fn set(key: &'static str, value: &str) -> Self {
            let prior = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prior }
        }

        /// Removes `key`, saving whatever was previously there so `Drop`
        /// can restore it exactly (a no-op restore if it was already
        /// absent).
        fn unset(key: &'static str) -> Self {
            let prior = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, prior }
        }
    }

    #[cfg(test)]
    #[allow(clippy::disallowed_methods)]
    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // Restores on every exit path, including panic (Warning-10 fix).
            match &self.prior {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    /// Plan 117-11 Behavior 1/2: exercises the real
    /// `NONO_BROKER_REQUIRED_LAYERS` env-var read path (the exact call
    /// `run()` makes) via the Drop-restoring [`EnvVarGuard`], feeding the
    /// result into [`app_container_resume_gate`] alongside a synthetic
    /// probe result — no live spawned child required, but the env-var
    /// plumbing itself is real, not mocked.
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod env_wire_contract_tests {
        use super::*;

        const ENV_VAR: &str = "NONO_BROKER_REQUIRED_LAYERS";

        /// Behavior 1: with `NONO_BROKER_REQUIRED_LAYERS` set to
        /// `"AppContainerProfile"` and an unconfirmed probe, the gate
        /// refuses resume — the broker terminates the suspended child and
        /// returns `Err` before `ResumeThread` (verified at the `run()`
        /// call site: `broker_resume_gate`'s `Err` is the exact and only
        /// condition under which `run()` terminates the child instead of
        /// resuming it).
        #[test]
        fn required_layers_env_var_tightens_the_gate() {
            let _guard = EnvVarGuard::set(ENV_VAR, "AppContainerProfile");
            let raw = std::env::var(ENV_VAR).expect("guard just set it");
            assert_eq!(raw, "AppContainerProfile");
            assert!(
                broker_resume_gate(&raw, Some(Ok(None)), Some("S-1-15-2-1"), Ok(0x1000)).is_err(),
                "an unconfirmed probe must refuse resume when the env var requires \
                 AppContainerProfile"
            );
        }

        /// Phase 117 review CR-03.3: with `NONO_BROKER_REQUIRED_LAYERS`
        /// unset, `run()` refuses to resume at all — an absent wire contract
        /// is a misconfiguration, not permission to skip attestation. The
        /// pre-fix `unwrap_or_default()` turned the absent variable into
        /// `""`, which the gate read as "require nothing".
        #[test]
        fn absent_required_layers_env_var_is_fail_closed() {
            let _guard = EnvVarGuard::unset(ENV_VAR);
            let raw = std::env::var(ENV_VAR);
            assert!(raw.is_err(), "the guard must have removed the variable");
            // This is the exact shape `run()` uses: the read itself is the
            // fail-closed point, and the empty string a lenient read would
            // have produced is ALSO refused by the gate.
            assert!(
                broker_resume_gate("", None, None, Ok(0x1000)).is_err(),
                "an absent/empty required-layers env var must refuse resume (fail-closed)"
            );
        }
    }

    /// Phase 118 Plan 06 (D-15/D-27): the broker's own 13-row census,
    /// receipt-record shape, and D-14 sentinel round-trip.
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod broker_receipt_tests {
        use super::*;

        fn sample_receipt() -> nono::EnforcementReceipt {
            nono::EnforcementReceipt {
                schema_version: 1,
                session_id: "test-session".to_string(),
                pid: 4242,
                entry_path: nono::EntryPath::Broker,
                token_arm: None,
                outcome: nono::SessionOutcome::Ran,
                layers: nono::LayerId::ALL
                    .iter()
                    .map(|&id| nono::LayerReceiptRow {
                        id,
                        status: nono::LayerAttestationStatus::Confirmed,
                    })
                    .collect(),
            }
        }

        /// D-27: a `LayerId` named in NEITHER wire-contract channel — a
        /// version-skewed nono-cli/nono-shell-broker pair is the real-world
        /// shape this proves safe for — must classify `Unconfirmed`, never
        /// be silently dropped from the 13-row census. Uses a REAL
        /// `LayerId` (`RestrictedToken`) that is genuinely absent from both
        /// test strings.
        #[test]
        fn unrecognized_row_classifies_unconfirmed_never_dropped() {
            let census = broker_census(
                "AppContainerProfile",
                "MandatoryIntegrityLabel",
                &None,
                None,
                &Ok(0x1000),
            );
            assert_eq!(
                census.len(),
                13,
                "the census must always name all 13 rows, never a filtered subset"
            );
            let row = census
                .iter()
                .find(|r| r.id == nono::LayerId::RestrictedToken)
                .expect("RestrictedToken must be present in the census output");
            assert_eq!(
                row.status,
                nono::LayerAttestationStatus::Unconfirmed,
                "a row named in neither wire-contract channel must fail toward Unconfirmed, \
                 not be silently dropped or promoted to Confirmed/NotApplicable"
            );
        }

        #[test]
        fn not_applicable_channel_row_classifies_not_applicable() {
            let census = broker_census(
                "",
                "AppContainerProfile,MandatoryIntegrityLabel",
                &None,
                None,
                &Ok(0x1000),
            );
            let row = census
                .iter()
                .find(|r| r.id == nono::LayerId::AppContainerProfile)
                .expect("present");
            assert_eq!(row.status, nono::LayerAttestationStatus::NotApplicable);
        }

        #[test]
        fn required_app_container_profile_confirms_only_when_sid_matches_expected() {
            let matching = broker_census(
                "AppContainerProfile",
                "",
                &Some(Ok(Some("S-1-15-2-1".to_string()))),
                Some("S-1-15-2-1"),
                &Ok(0x1000),
            );
            let row = matching
                .iter()
                .find(|r| r.id == nono::LayerId::AppContainerProfile)
                .unwrap();
            assert_eq!(row.status, nono::LayerAttestationStatus::Confirmed);

            let mismatching = broker_census(
                "AppContainerProfile",
                "",
                &Some(Ok(Some("S-1-15-2-1".to_string()))),
                Some("S-1-15-2-999"),
                &Ok(0x1000),
            );
            let row = mismatching
                .iter()
                .find(|r| r.id == nono::LayerId::AppContainerProfile)
                .unwrap();
            assert_eq!(
                row.status,
                nono::LayerAttestationStatus::Unconfirmed,
                "an AppContainer SID present but not matching the per-run expected SID must \
                 not classify Confirmed"
            );
        }

        #[test]
        fn required_mandatory_integrity_label_confirms_at_or_below_low_rid() {
            let census = broker_census(
                "MandatoryIntegrityLabel",
                "",
                &None,
                None,
                &Ok(SECURITY_MANDATORY_LOW_RID),
            );
            let row = census
                .iter()
                .find(|r| r.id == nono::LayerId::MandatoryIntegrityLabel)
                .unwrap();
            assert_eq!(row.status, nono::LayerAttestationStatus::Confirmed);

            let census_medium =
                broker_census("MandatoryIntegrityLabel", "", &None, None, &Ok(0x2000));
            let row2 = census_medium
                .iter()
                .find(|r| r.id == nono::LayerId::MandatoryIntegrityLabel)
                .unwrap();
            assert_eq!(row2.status, nono::LayerAttestationStatus::Unconfirmed);
        }

        /// D-10/Plan 118-09: this crate's hand-rolled `BrokerReceiptRecord`
        /// field names must match `receipt_sink.rs`'s `ReceiptRecord` shape
        /// byte-for-byte, so the future `nono receipt` command family can
        /// read either writer's output with one code path.
        #[test]
        fn broker_receipt_record_field_names_match_receipt_sink_shape() {
            let receipt = sample_receipt();
            let leaf_hash = nono::hash_receipt_event(b"{}");
            let record = BrokerReceiptRecord {
                sequence: 0,
                prev_head: None,
                leaf_hash,
                chain_head: nono::hash_receipt_chain(None, &leaf_hash),
                receipt: &receipt,
            };
            let json = serde_json::to_value(&record).expect("record must serialize");
            let obj = json
                .as_object()
                .expect("record must serialize to a JSON object");
            let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                vec![
                    "chain_head",
                    "leaf_hash",
                    "prev_head",
                    "receipt",
                    "sequence"
                ],
                "field names must match receipt_sink.rs's ReceiptRecord shape byte-for-byte"
            );
        }

        #[test]
        fn broker_receipt_writer_chains_two_records_and_recomputes_from_disk() {
            let sink_dir = tempfile::tempdir().expect("tempdir");
            let mut writer = BrokerReceiptWriter::new("test-broker-session", sink_dir.path())
                .expect("writer must construct");

            let receipt_a = sample_receipt();
            let mut receipt_b = sample_receipt();
            receipt_b.outcome = nono::SessionOutcome::Refused;

            writer.write_receipt(&receipt_a).expect("write 1");
            writer.write_receipt(&receipt_b).expect("write 2");

            let contents = std::fs::read_to_string(&writer.file_path).expect("read sink file");
            let lines: Vec<&str> = contents.lines().collect();
            assert_eq!(lines.len(), 2, "expected exactly 2 JSONL records");

            let record_a: serde_json::Value =
                serde_json::from_str(lines[0]).expect("parse record 1");
            let record_b: serde_json::Value =
                serde_json::from_str(lines[1]).expect("parse record 2");
            assert_eq!(record_a["sequence"], 0);
            assert!(record_a["prev_head"].is_null());
            assert_eq!(record_b["sequence"], 1);
            assert_eq!(record_b["prev_head"], record_a["chain_head"]);

            // Recompute the chain independently from the on-disk leaf
            // hashes and confirm it matches what was stored — the
            // fail-closed recompute-and-compare shape `nono receipt verify`
            // will use later (D-10).
            let leaf_a: nono::undo::ContentHash =
                serde_json::from_value(record_a["leaf_hash"].clone()).expect("parse leaf hash");
            let recomputed_head_a = nono::hash_receipt_chain(None, &leaf_a);
            let stored_head_a: nono::undo::ContentHash =
                serde_json::from_value(record_a["chain_head"].clone()).expect("parse chain head");
            assert_eq!(recomputed_head_a, stored_head_a);
        }

        /// Phase 118 review CR-02: a SECOND, independently-constructed
        /// [`BrokerReceiptWriter`] over an EXISTING segment must resume the
        /// chain (`sequence: 1`, `prev_head` = the first writer's
        /// `chain_head`) rather than restarting at genesis — exactly the
        /// broker's own `Ran`-then-`ResumeThread`-fails corrective-record
        /// path exercises in production (`record_broker_receipt` constructs
        /// a fresh `BrokerReceiptWriter` on EVERY call). The test above
        /// reuses one writer for both writes and therefore cannot catch this.
        #[test]
        fn second_broker_writer_over_existing_segment_resumes_the_chain() {
            let sink_dir = tempfile::tempdir().expect("tempdir");
            let session_id = "test-broker-session-tworiters";

            let mut writer_1 = BrokerReceiptWriter::new(session_id, sink_dir.path())
                .expect("writer 1 must construct");
            writer_1.write_receipt(&sample_receipt()).expect("write 1");
            drop(writer_1);

            let mut writer_2 = BrokerReceiptWriter::new(session_id, sink_dir.path())
                .expect("writer 2 must construct");
            let mut receipt_b = sample_receipt();
            receipt_b.outcome = nono::SessionOutcome::Refused;
            writer_2.write_receipt(&receipt_b).expect("write 2");

            let contents = std::fs::read_to_string(&writer_2.file_path).expect("read sink file");
            let lines: Vec<&str> = contents.lines().collect();
            assert_eq!(lines.len(), 2, "expected exactly 2 JSONL records");

            let record_a: serde_json::Value =
                serde_json::from_str(lines[0]).expect("parse record 1");
            let record_b: serde_json::Value =
                serde_json::from_str(lines[1]).expect("parse record 2");
            assert_eq!(record_a["sequence"], 0);
            assert!(record_a["prev_head"].is_null());
            assert_eq!(
                record_b["sequence"], 1,
                "a second writer over an existing segment must resume sequence, not restart at 0"
            );
            assert_eq!(
                record_b["prev_head"], record_a["chain_head"],
                "a second writer's prev_head must chain from the first writer's chain_head"
            );
        }

        /// Phase 118 review CR-02: fail CLOSED when an existing segment's
        /// tail cannot be parsed, mirroring `receipt_sink.rs`'s identical
        /// fail-closed test.
        #[test]
        fn new_broker_writer_rejects_a_segment_with_an_unparseable_tail_record() {
            let sink_dir = tempfile::tempdir().expect("tempdir");
            let file_path = sink_dir.path().join("test-broker-corrupt.broker.jsonl");
            std::fs::write(&file_path, b"{ this is not valid JSON at all\n")
                .expect("seed corrupt file");

            let err = BrokerReceiptWriter::new("test-broker-corrupt", sink_dir.path())
                .expect_err("a segment with an unparseable tail must be rejected, not restarted");
            assert!(matches!(err, NonoError::Snapshot(_)));
        }

        #[test]
        fn ensure_broker_receipt_sink_guarded_applies_label_even_with_no_sid() {
            let dir = tempfile::tempdir().expect("tempdir");
            ensure_broker_receipt_sink_guarded(dir.path(), None)
                .expect("guard must apply the mandatory label unconditionally");
            assert!(
                nono::low_integrity_label_and_mask(dir.path()).is_some(),
                "the mandatory label must be applied even when no package SID was minted"
            );
        }

        /// D-14 half 2 (sentinel round-trip), THIRD producer, class
        /// coverage (RESEARCH.md's three-audit-questions discipline): the
        /// broker's closest analog to the CLI's `expected_session_sid` and
        /// the daemon's `expected_package_sid` sentinel-injection points is
        /// the PROBED (not merely expected/compared) AppContainer package
        /// SID string — arguably higher-risk since it originates from a
        /// live OS probe, not a caller-supplied comparison value.
        ///
        /// Negative-control demonstration (per this plan's required
        /// evidence, NOT committed — see `118-06-SUMMARY.md`): a temporary
        /// `raw_app_container_sid: String` field was added to
        /// `EnforcementReceipt`, populated verbatim from
        /// `app_container_probe`, and this test re-run to confirm it FAILS
        /// before the field was reverted.
        #[test]
        fn sentinel_seeded_app_container_sid_never_leaks_into_the_serialized_broker_receipt() {
            const SENTINEL: &str = "S-1-15-2-1-SENTINEL-BROKER-4f1a";
            let census = broker_census(
                "AppContainerProfile,MandatoryIntegrityLabel",
                "",
                &Some(Ok(Some(SENTINEL.to_string()))),
                Some(SENTINEL),
                &Ok(0x1000),
            );
            let receipt = nono::EnforcementReceipt {
                schema_version: 1,
                session_id: "sentinel-session".to_string(),
                pid: 4242,
                entry_path: nono::EntryPath::Broker,
                token_arm: None,
                outcome: nono::SessionOutcome::Ran,
                layers: census,
            };
            let json = serde_json::to_string(&receipt).expect("receipt must serialize");
            assert!(
                !json.contains("SENTINEL-BROKER-4f1a"),
                "the sentinel-seeded AppContainer SID must never appear in a serialized \
                 broker receipt: {json}"
            );
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

    /// CINT-03 forced-unavailable regression (Phase 117-07). Only compiled
    /// with `--features layer-fault-injection`; a default build contains
    /// neither `force_app_container_unavailable` nor this test.
    #[cfg(feature = "layer-fault-injection")]
    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod app_container_force_unavailable_tests {
        use super::*;
        use nono::NonoError;

        /// With the broker's AppContainer force-unavailable seam armed,
        /// `run()` refuses to construct `SECURITY_CAPABILITIES` and returns
        /// `NonoError::LayerAttestationFailed` before ever calling
        /// `CreateProcessW` with `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`.
        /// RESEARCH finding 2: this is the ONLY place the real AppContainer
        /// construction can be exercised — the actual lowbox child is
        /// spawned inside the broker's own process, never visible to a hook
        /// on nono-cli's side alone.
        ///
        /// `create_app_container_profile`/`derive_app_container_sid` run for
        /// real before our check (matching the plan's exact insertion point,
        /// mirroring `nono::sandbox::windows::create_app_container_profile_round_trips`'s
        /// environment-sensitivity discipline): if THIS specific environment
        /// rejects AppContainer profile registration, that is reported loudly
        /// (D-31) rather than silently skipped, and is distinguishable from an
        /// actual seam failure by error variant.
        #[test]
        fn run_fails_when_app_container_forced_unavailable() {
            let name = format!("nono.test.117-07.{}", std::process::id());
            let args = BrokerArgs {
                shell_path: PathBuf::from(r"C:\Windows\System32\cmd.exe"),
                shell_args: vec![],
                // Fake-but-nonzero/non-MAX handle values: UpdateProcThreadAttribute
                // does not dereference or validate handles when building the
                // attribute list (only CreateProcess does), and our
                // force-unavailable check fires before CreateProcessW is ever
                // reached.
                inherit_handles: vec![0x100 as HANDLE, 0x200 as HANDLE, 0x300 as HANDLE],
                cwd: PathBuf::from(r"C:\"),
                no_pty: true,
                app_container_name: Some(name),
            };

            force_app_container_unavailable(true);
            let result = run(args);
            // Always reset the flag, even on assertion failure, so this test
            // cannot leak state into other tests in the same binary.
            force_app_container_unavailable(false);

            match result {
                Err(NonoError::LayerAttestationFailed { layer, reason }) => {
                    assert_eq!(layer, "AppContainerProfile");
                    assert!(
                        reason.contains("forced unavailable"),
                        "reason must explain the forced-unavailable seam: {reason}"
                    );
                }
                Err(NonoError::SandboxInit(msg)) => {
                    eprintln!(
                        "skipping strict assertion for run_fails_when_app_container_forced_unavailable: \
                         this environment rejected AppContainer profile registration/derivation \
                         BEFORE the force-unavailable check could fire: {msg} \
                         (D-31: reported, not silently skipped; the live UAT run is the proof)"
                    );
                }
                Err(other) => panic!(
                    "expected NonoError::LayerAttestationFailed (or environment-limited \
                     SandboxInit before the seam) while forced unavailable, got Err({other})"
                ),
                Ok(_) => panic!(
                    "expected an Err while the AppContainer force-unavailable seam is armed, got Ok"
                ),
            }
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
