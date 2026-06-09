//! `nono-fltmgr-client` — user-mode policy client for the nono minifilter spike.
//!
//! ALL code is `#[cfg(windows)]`. Compiles to an empty crate on Linux/macOS CI.
//! Phase 64 DRV-02 spike: connects to `\NonoPolicyPort`, receives `NonoIpcRequest`,
//! returns allow/deny decision.
//!
//! This crate intentionally has no dependency on the `nono` library crate — it is
//! a standalone spike binary used for VM-side round-trip testing (Plan 04 Step 5).

// On non-Windows targets the Windows-sys imports are cfg-gated out entirely.
// A stub `run_policy_client` is still exposed so the function is callable from
// build scripts or integration tests on any platform (Plan 64-03 Task 1, D-03).
#[cfg(not(windows))]
/// Non-Windows stub: the minifilter policy client is Windows-only.
///
/// # Errors
///
/// Always returns an error — there is no minifilter port to connect to off Windows.
pub fn run_policy_client(_deny_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    Err("nono-fltmgr-client is Windows-only".into())
}

#[cfg(windows)]
mod client {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::InstallableFileSystems::{
        FilterConnectCommunicationPort, FilterGetMessage, FilterReplyMessage,
        FILTER_MESSAGE_HEADER, FILTER_REPLY_HEADER,
    };

    /// IPC message buffer received from the minifilter driver via `FilterGetMessage`.
    ///
    /// `FILTER_MESSAGE_HEADER` MUST be the first field: `FilterGetMessage` writes the
    /// header prefix into the buffer before the payload. The C-side `NONO_IPC_REQUEST`
    /// does not include the header; the Rust side must include it in the receive buffer.
    ///
    /// Layout (payload only, excluding header):
    /// - `path_buffer`: 260 × u16 = 520 bytes
    /// - `process_id`:  4 bytes
    /// - `desired_access`: 4 bytes
    /// - `reserved`:    4 bytes
    ///
    /// Total payload: 532 bytes (must match C-side `_Static_assert(sizeof(NONO_IPC_REQUEST) == 532)`).
    #[repr(C, packed(1))]
    pub struct NonoIpcRequest {
        /// Message header written by `FilterGetMessage` as a prefix — must be first.
        pub header: FILTER_MESSAGE_HEADER,
        /// File path (null-terminated UTF-16, MAX_PATH WCHARs).
        pub path_buffer: [u16; 260],
        /// PID of the process that triggered the `IRP_MJ_CREATE`.
        pub process_id: u32,
        /// `DesiredAccess` from the `IRP_MJ_CREATE` parameters.
        pub desired_access: u32,
        /// Spike padding; reserved for future fields.
        pub reserved: u32,
    }

    // Compile-time layout assertion: the payload (excluding FILTER_MESSAGE_HEADER) must
    // be exactly 532 bytes, matching the C-side `_Static_assert(sizeof(NONO_IPC_REQUEST) == 532)`.
    // This assertion catches Rust/C ABI mismatches at compile time (T-64-SC-01).
    const _: () = assert!(
        std::mem::size_of::<NonoIpcRequest>()
            - std::mem::size_of::<FILTER_MESSAGE_HEADER>()
            == 532,
        "NonoIpcRequest payload size mismatch with C-side NONO_IPC_REQUEST"
    );

    /// Reply sent back to the minifilter driver via `FilterReplyMessage`.
    ///
    /// `decision = 0` → allow; `decision = 1` → deny (`STATUS_ACCESS_DENIED`).
    #[repr(C, packed(1))]
    pub struct NonoIpcReply {
        /// 0 = allow the file open; 1 = deny with STATUS_ACCESS_DENIED.
        pub decision: u32,
    }

    /// Internal reply buffer (includes the required `FILTER_REPLY_HEADER` prefix).
    #[repr(C)]
    struct ReplyBuf {
        header: FILTER_REPLY_HEADER,
        decision: u32,
    }

    /// Connect to `\NonoPolicyPort` and run the allow/deny policy loop.
    ///
    /// Blocks until the port disconnects or an irrecoverable error occurs.
    /// For each incoming `NonoIpcRequest`, checks whether the normalized file path
    /// matches `deny_path` (case-insensitive ASCII comparison). Matching files receive
    /// a deny reply; all others receive allow.
    ///
    /// # Errors
    ///
    /// Returns an error string if `FilterConnectCommunicationPort` fails or if a
    /// fatal `FilterGetMessage` error occurs.
    pub fn run_policy_client(deny_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let port_name: Vec<u16> = OsStr::new("\\NonoPolicyPort")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut port: HANDLE = INVALID_HANDLE_VALUE;

        // SAFETY: FilterConnectCommunicationPort requires a null-terminated wide string
        // port name. `port_name` is a Vec<u16> terminated with the `0` appended above.
        // The context pointer and size are 0/null (no connection context for this spike).
        // `lpSecurityAttributes` is null (use default security for the connection).
        // `hport` is a valid out-parameter pointing to our `port` local variable.
        let hr = unsafe {
            FilterConnectCommunicationPort(
                port_name.as_ptr(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                &mut port,
            )
        };

        if hr != 0 || port == INVALID_HANDLE_VALUE {
            return Err(format!(
                "FilterConnectCommunicationPort failed (HRESULT=0x{hr:08X}). \
                 Ensure the nono minifilter driver is loaded and \\NonoPolicyPort is open."
            )
            .into());
        }

        loop {
            // Zero-initialise the receive buffer before each `FilterGetMessage` call.
            // SAFETY: NonoIpcRequest is a repr(C, packed(1)) struct of integer types;
            // all-zero bits is a valid representation for every field.
            let mut buf: NonoIpcRequest = unsafe { std::mem::zeroed() };
            let buf_size = std::mem::size_of::<NonoIpcRequest>() as u32;

            // SAFETY: `buf.header` is the first field of `NonoIpcRequest` (repr(C, packed(1))).
            // The pointer is valid and writable for `buf_size` bytes. The overlapped argument
            // is null (synchronous call). FilterGetMessage writes FILTER_MESSAGE_HEADER at the
            // buffer start, then the payload bytes immediately after.
            let hr = unsafe {
                FilterGetMessage(
                    port,
                    std::ptr::addr_of_mut!(buf.header),
                    buf_size,
                    std::ptr::null_mut(),
                )
            };

            if hr != 0 {
                // Port disconnected or unrecoverable error — exit the loop cleanly.
                // SAFETY: closing a valid handle obtained from FilterConnectCommunicationPort.
                unsafe { CloseHandle(port) };
                break;
            }

            // Decode the null-terminated UTF-16 path from the fixed-size buffer.
            // We copy path_buffer into a local aligned array before decoding because
            // `buf` is `repr(C, packed(1))` and taking a reference to a packed field
            // would be undefined behavior (UB) in Rust (misaligned reference).
            //
            // SAFETY: `buf.path_buffer` is a `[u16; 260]` field inside a packed struct.
            // We use `ptr::read_unaligned` to copy it to an aligned local array.
            let path_local: [u16; 260] = unsafe {
                std::ptr::read_unaligned(std::ptr::addr_of!(buf.path_buffer))
            };
            let path = String::from_utf16_lossy(&path_local);
            let path = path.trim_end_matches('\0');

            // Policy: deny if the intercepted path matches the configured deny target.
            // The kernel reports the OPENED name in device form (e.g.
            // \Device\HarddiskVolumeN\nono-deny-test\secret.txt), which does NOT
            // exact-match a "C:\..." deny target. Compare on the path tail (everything
            // from the first backslash — drive/volume independent) so the device form
            // and the drive form match. Case-insensitive (Windows paths are).
            let deny_tail = match deny_path.find('\\') {
                Some(idx) => &deny_path[idx..],
                None => deny_path,
            };
            let is_deny = !deny_tail.is_empty()
                && path
                    .to_ascii_lowercase()
                    .ends_with(&deny_tail.to_ascii_lowercase());
            let decision: u32 = if is_deny { 1 } else { 0 };

            // Spike diagnostics: log each intercepted create + decision to stderr so the
            // round-trip is visible and the SC1 evidence can capture the DENY line.
            eprintln!("[{}] {}", if is_deny { "DENY " } else { "allow" }, path);

            // Echo the MessageId so the kernel can correlate the reply to the
            // pending FltSendMessage call (single-connection spike — trivially correct).
            // SAFETY: `buf.header.MessageId` is inside a packed struct; we use
            // `ptr::read_unaligned` to avoid a misaligned reference.
            let message_id: u64 = unsafe {
                std::ptr::read_unaligned(std::ptr::addr_of!(buf.header.MessageId))
            };
            let mut reply = ReplyBuf {
                header: FILTER_REPLY_HEADER {
                    Status: 0,
                    MessageId: message_id,
                },
                decision,
            };

            // SAFETY: `reply.header` is the first field of `ReplyBuf` (repr(C)).
            // The pointer is valid for `size_of::<ReplyBuf>()` bytes. The port handle
            // is still valid (we only close it after a FilterGetMessage failure above).
            unsafe {
                FilterReplyMessage(
                    port,
                    std::ptr::addr_of_mut!(reply.header),
                    std::mem::size_of::<ReplyBuf>() as u32,
                )
            };
        }

        Ok(())
    }
}

#[cfg(windows)]
pub use client::{NonoIpcReply, NonoIpcRequest, run_policy_client};
