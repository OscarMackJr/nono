//! poc-broker: Broker-process pattern PoC.
//!
//! Two modes:
//!   (default)  A1 PoC — validates RESEARCH.md Assumption A1: a Low-IL child that
//!              INHERITS the broker's already-attached console skips the CSRSS ALPC
//!              connect (KernelBase ConClntInitialize) and survives DllMain.
//!
//!   --conpty   B' PoC (debug nono-shell-claude-hang) — a Medium-IL parent creates a
//!              ConPTY and spawns Low-IL PowerShell ATTACHED to it via
//!              PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, then relays stdio. This tests the
//!              two open unknowns for Option B':
//!                (1) does attaching a pseudoconsole to a Low-IL spawn re-trip the
//!                    Phase-30 0xC0000142 STATUS_DLL_INIT_FAILED? (direct-child gate)
//!                (2) once PowerShell is up on the ConPTY, do GRANDCHILDREN
//!                    (type `cmd /c echo HI`, `claude`) get a usable TTY, or do they
//!                    still hang on cross-IL conhost registration?
//!
//! Build: cd poc-broker && cargo build --release --target x86_64-pc-windows-msvc
//! Run (A1):     .\target\release\poc-broker.exe
//! Run (B' test): .\target\release\poc-broker.exe --conpty
//!                then, at the sandboxed PowerShell prompt, type:
//!                  cmd /c echo HI        (grandchild output should appear, prompt returns)
//!                  claude                (should the interactive TUI render?)
//!                  exit                  (ends the PoC; exit code is interpreted)

#[cfg(windows)]
use std::{
    ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt, os::windows::io::FromRawHandle,
};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::Security::{
    CreateWellKnownSid, DuplicateTokenEx, SecurityAnonymous, SetTokenInformation,
    TokenIntegrityLevel, TokenPrimary, WinLowLabelSid, SECURITY_ATTRIBUTES,
    SECURITY_IMPERSONATION_LEVEL, SECURITY_MAX_SID_SIZE, TOKEN_ADJUST_DEFAULT,
    TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    AllocConsole, ClosePseudoConsole, CreatePseudoConsole, GetConsoleMode, GetStdHandle,
    SetConsoleMode, COORD, DISABLE_NEWLINE_AUTO_RETURN, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT,
    ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    HPCON, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
#[cfg(windows)]
use windows_sys::Win32::System::Pipes::CreatePipe;
#[cfg(windows)]
use windows_sys::Win32::System::SystemServices::SE_GROUP_INTEGRITY;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    CreateProcessAsUserW, DeleteProcThreadAttributeList, GetCurrentProcess, GetExitCodeProcess,
    InitializeProcThreadAttributeList, OpenProcessToken, UpdateProcThreadAttribute,
    WaitForSingleObject, EXTENDED_STARTUPINFO_PRESENT, INFINITE, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTUPINFOEXW, STARTUPINFOW,
};

#[cfg(not(windows))]
fn main() {
    eprintln!("[POC] Windows-only binary. Build with: cargo build --release --target x86_64-pc-windows-msvc");
    eprintln!("[POC] Skeleton compiled OK — Win32 wiring activates only on Windows targets.");
}

#[cfg(windows)]
fn main() {
    if std::env::args().any(|a| a == "--conpty") {
        run_conpty_poc();
    } else {
        run_inherited_console_poc();
    }
}

/// A1 PoC (original): Low-IL child inherits the broker's console; no pseudoconsole.
#[cfg(windows)]
fn run_inherited_console_poc() {
    // 1. AllocConsole — attach to a console at Medium IL. Returns 0 if one already
    //    exists (non-fatal; inherited console satisfies the mechanism equally).
    let alloc_rc = unsafe {
        // SAFETY: AllocConsole takes no arguments and is safe to call unconditionally.
        AllocConsole()
    };
    println!(
        "[POC] AllocConsole rc={alloc_rc} (0=inherited parent console, non-zero=new console)"
    );

    // 2-6. Construct the Low Mandatory Level primary token.
    let h_new_token = unsafe { build_low_il_token() };

    // 7. Build UTF-16 command line. No CREATE_NEW_CONSOLE, no DETACHED_PROCESS — the
    //    child inherits the broker's already-attached console. This is the critical flag
    //    combination that tests Assumption A1.
    let mut cmd_wide: Vec<u16> = OsStr::new("powershell.exe -NoLogo")
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: STARTUPINFOW and PROCESS_INFORMATION are #[repr(C)] POD structs;
    // all-zero is a valid representation. Required init pattern per Win32 docs.
    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = size_of::<STARTUPINFOW>() as u32;
    // SAFETY: PROCESS_INFORMATION zero-init is the documented Win32 idiom.
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    println!("[POC] Mechanism: AllocConsole + DuplicateTokenEx(SecurityAnonymous,TokenPrimary) + SetTokenInformation(Low) + CreateProcessAsUserW(dwCreationFlags=0)");

    // 8. Spawn PowerShell at Low IL, inheriting the broker's console.
    let ok = unsafe {
        // SAFETY: h_new_token is a valid primary token; cmd_wide is null-terminated UTF-16;
        // si and pi are correctly sized zero-initialised structs.
        CreateProcessAsUserW(
            h_new_token,
            std::ptr::null(),      // lpApplicationName: derive from cmd line
            cmd_wide.as_mut_ptr(), // lpCommandLine: mutable per Win32 ABI
            std::ptr::null(),      // lpProcessAttributes
            std::ptr::null(),      // lpThreadAttributes
            0,                     // bInheritHandles = FALSE
            0,                     // dwCreationFlags = 0 (no CREATE_NEW_CONSOLE)
            std::ptr::null(),      // lpEnvironment: inherit parent
            std::ptr::null(),      // lpCurrentDirectory: inherit parent
            &si,
            &mut pi,
        )
    };
    fatal_if_zero(ok, "CreateProcessAsUserW");

    println!("[POC] Child PID: {}", pi.dwProcessId);
    println!("[POC] Waiting for child...");

    let exit_code = unsafe { wait_and_exit_code(pi.hProcess) };
    println!("[POC] Child exit code: {exit_code:#010x} ({exit_code})");
    match exit_code {
        0 => println!("[POC] PASS — broker pattern viable; child survived KernelBase DllMain at Low-IL"),
        3_221_225_794 => println!(
            "[POC] FAIL variant A — CSRSS still denies Low-IL child even with inherited console; \
            broker pattern NOT viable without further mechanism"
        ),
        other => println!(
            "[POC] FAIL variant B — unexpected exit code {other:#010x}; \
            capture ProcMon trace and analyze"
        ),
    }

    unsafe {
        // SAFETY: handles valid and owned by this process; each closed exactly once.
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        CloseHandle(h_new_token);
    }
}

/// B' PoC (`--conpty`): Medium-IL parent creates a ConPTY and spawns Low-IL PowerShell
/// attached via PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, relaying stdio to this console.
#[cfg(windows)]
fn run_conpty_poc() {
    println!("[POC-CONPTY] Option B' shape: a Medium-IL parent creates a ConPTY and spawns");
    println!("[POC-CONPTY] Low-IL powershell.exe ATTACHED to it (PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE).");
    println!("[POC-CONPTY] Tests: (1) does the Low-IL child re-trip 0xC0000142 at spawn/DllMain?");
    println!("[POC-CONPTY]        (2) at the prompt, type `cmd /c echo HI` then `claude` — do");
    println!("[POC-CONPTY]            grandchildren produce output / render a TUI, or still hang?");
    println!("[POC-CONPTY] Type `exit` to end the PoC.\n");

    unsafe {
        // SAFETY: AllocConsole takes no args; non-fatal if a console is already attached.
        AllocConsole();
    }

    // 1. Create the two ConPTY pipes (input + output). bInheritHandle=0: the pseudoconsole
    //    takes its own references; the child attaches via the pseudoconsole, not inheritance.
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 0,
    };
    let mut in_read: HANDLE = INVALID_HANDLE_VALUE;
    let mut in_write: HANDLE = INVALID_HANDLE_VALUE;
    let mut out_read: HANDLE = INVALID_HANDLE_VALUE;
    let mut out_write: HANDLE = INVALID_HANDLE_VALUE;
    unsafe {
        // SAFETY: out-pointers are valid stack locals; `sa` lives for the call.
        fatal_if_zero(CreatePipe(&mut in_read, &mut in_write, &sa, 0), "CreatePipe(input)");
        fatal_if_zero(
            CreatePipe(&mut out_read, &mut out_write, &sa, 0),
            "CreatePipe(output)",
        );
    }

    // 2. Create the pseudoconsole from the CHILD ends; keep the PARENT ends for the relay.
    let mut hpcon: HPCON = 0;
    let size = COORD { X: 120, Y: 30 };
    let hr = unsafe {
        // SAFETY: in_read/out_write are valid pipe handles; hpcon is a valid out-pointer.
        CreatePseudoConsole(size, in_read, out_write, 0, &mut hpcon)
    };
    unsafe {
        // SAFETY: the pseudoconsole keeps its own references; we close our copies of the
        // child ends now (documented CreatePseudoConsole contract).
        CloseHandle(in_read);
        CloseHandle(out_write);
    }
    if hr != 0 {
        eprintln!("[POC-CONPTY] FATAL: CreatePseudoConsole failed HRESULT 0x{hr:X}");
        std::process::exit(1);
    }

    // 3. Low Mandatory Level primary token.
    let h_token = unsafe { build_low_il_token() };

    // 4. Attribute list (1 slot): PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE = hpcon. Mirrors
    //    the windows-sys idiom in nono-cli launch.rs (addr_of! + size_of::<HPCON>()).
    let mut attr_size: usize = 0;
    unsafe {
        // SAFETY: probe call with null list returns the required size (Win32 idiom).
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size];
    let attr_list: LPPROC_THREAD_ATTRIBUTE_LIST =
        attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
    unsafe {
        // SAFETY: attr_list points into attr_buf sized by the probe above for one slot.
        fatal_if_zero(
            InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size),
            "InitializeProcThreadAttributeList",
        );
    }
    let hpcon_value = hpcon;
    unsafe {
        // SAFETY: attr_list initialized above; hpcon_value outlives the CreateProcess call.
        fatal_if_zero(
            UpdateProcThreadAttribute(
                attr_list,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                std::ptr::addr_of!(hpcon_value) as *mut _,
                size_of::<HPCON>(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ),
            "UpdateProcThreadAttribute(PSEUDOCONSOLE)",
        );
    }

    // 5. Spawn Low-IL powershell.exe attached to the pseudoconsole. No STARTF_USESTDHANDLES,
    //    no new-console flag — stdio flows through the ConPTY. bInheritHandles=FALSE.
    let mut cmd_wide: Vec<u16> = OsStr::new("powershell.exe -NoLogo")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut si: STARTUPINFOEXW = unsafe {
        // SAFETY: STARTUPINFOEXW is #[repr(C)] POD; zero-init is documented.
        std::mem::zeroed()
    };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.lpAttributeList = attr_list;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let created = unsafe {
        // SAFETY: h_token is a valid Low-IL primary token; cmd_wide is null-terminated;
        // si uses EXTENDED_STARTUPINFO_PRESENT matching the STARTUPINFOEXW layout.
        CreateProcessAsUserW(
            h_token,
            std::ptr::null(),
            cmd_wide.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0, // bInheritHandles = FALSE (pseudoconsole carries stdio)
            EXTENDED_STARTUPINFO_PRESENT,
            std::ptr::null(),
            std::ptr::null(),
            &si.StartupInfo as *const STARTUPINFOW,
            &mut pi,
        )
    };
    unsafe {
        // SAFETY: attr_list initialized above; safe to release after CreateProcess.
        DeleteProcThreadAttributeList(attr_list);
    }
    if created == 0 {
        let err = unsafe { GetLastError() };
        eprintln!(
            "[POC-CONPTY] FAIL — CreateProcessAsUserW (Low-IL + PSEUDOCONSOLE) rejected at \
             creation (GetLastError={err}). The pseudoconsole-on-Low-IL-spawn shape is refused \
             before the child even starts → Option B' as-shaped is NOT viable."
        );
        std::process::exit(1);
    }
    println!(
        "[POC-CONPTY] Spawned Low-IL PowerShell on the ConPTY, PID {}.\n",
        pi.dwProcessId
    );

    // 6. Put THIS console into VT pass-through + raw input so the ConPTY's output renders
    //    and keystrokes forward unbuffered (best-effort; ignore failures).
    enable_vt_passthrough();

    // 7. Relay: ConPTY output -> our stdout, our stdin -> ConPTY input. The File wrappers
    //    OWN the parent-end handles and close them on drop / process exit.
    let mut out_file = unsafe { std::fs::File::from_raw_handle(out_read as *mut _) };
    let mut in_file = unsafe { std::fs::File::from_raw_handle(in_write as *mut _) };
    std::thread::spawn(move || {
        let mut stdout = std::io::stdout();
        let _ = std::io::copy(&mut out_file, &mut stdout);
    });
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut lock = stdin.lock();
        let _ = std::io::copy(&mut lock, &mut in_file);
    });

    // 8. Wait for PowerShell to exit, then interpret.
    let exit_code = unsafe { wait_and_exit_code(pi.hProcess) };
    println!("\n[POC-CONPTY] PowerShell exit code: {exit_code:#010x} ({exit_code})");
    match exit_code {
        3_221_225_794 => println!(
            "[POC-CONPTY] FAIL — 0xC0000142 STATUS_DLL_INIT_FAILED: attaching a pseudoconsole to \
             the Low-IL child re-trips the Phase-30 CSRSS denial. Option B' as-shaped is NOT viable."
        ),
        _ => println!(
            "[POC-CONPTY] PowerShell itself survived (no 0xC0000142). Verdict on Option B' depends \
             on what you observed ABOVE: did `cmd /c echo HI` print + return, and did `claude` render? \
             If grandchildren produced output, B' is viable; if they hung, the ConPTY's Medium-IL \
             conhost still blocks Low-IL grandchildren and B' needs more (conhost-IL) work."
        ),
    }

    unsafe {
        // SAFETY: hpcon valid; process handles valid and owned. out_read/in_write are owned
        // by the relay File wrappers (closed on process exit) — NOT closed here (no double free).
        ClosePseudoConsole(hpcon);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        CloseHandle(h_token);
    }
    // The relay threads (blocked on stdin) are terminated by process exit.
    std::process::exit(exit_code as i32);
}

/// Build a Low Mandatory Level primary token by duplicating the current process token
/// and lowering its integrity label. Mirrors poc steps 2-6 / nono's
/// `create_low_integrity_primary_token`.
///
/// # Safety
/// Calls Win32 token APIs; the returned HANDLE is a primary token the caller must close.
#[cfg(windows)]
unsafe fn build_low_il_token() -> HANDLE {
    let mut h_token: HANDLE = std::ptr::null_mut();
    fatal_if_zero(
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
            &mut h_token,
        ),
        "OpenProcessToken",
    );

    let mut h_new_token: HANDLE = std::ptr::null_mut();
    fatal_if_zero(
        DuplicateTokenEx(
            h_token,
            TOKEN_ASSIGN_PRIMARY | TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ADJUST_DEFAULT,
            std::ptr::null(),
            SecurityAnonymous as SECURITY_IMPERSONATION_LEVEL,
            TokenPrimary,
            &mut h_new_token,
        ),
        "DuplicateTokenEx",
    );

    let mut sid_buf = [0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut sid_size = sid_buf.len() as u32;
    fatal_if_zero(
        CreateWellKnownSid(
            WinLowLabelSid,
            std::ptr::null_mut(),
            sid_buf.as_mut_ptr() as *mut _,
            &mut sid_size,
        ),
        "CreateWellKnownSid(WinLowLabelSid)",
    );

    let label_size = size_of::<TOKEN_MANDATORY_LABEL>() + sid_size as usize;
    let mut label_buf = vec![0u8; label_size];
    let label_ptr = label_buf.as_mut_ptr() as *mut TOKEN_MANDATORY_LABEL;
    let sid_ptr = label_buf.as_mut_ptr().add(size_of::<TOKEN_MANDATORY_LABEL>()) as *mut _;
    std::ptr::copy_nonoverlapping(sid_buf.as_ptr(), sid_ptr as *mut u8, sid_size as usize);
    (*label_ptr).Label.Sid = sid_ptr;
    (*label_ptr).Label.Attributes = SE_GROUP_INTEGRITY as u32;
    fatal_if_zero(
        SetTokenInformation(
            h_new_token,
            TokenIntegrityLevel,
            label_ptr as *mut _,
            label_size as u32,
        ),
        "SetTokenInformation(TokenIntegrityLevel, Low)",
    );

    CloseHandle(h_token);
    h_new_token
}

/// Best-effort: enable VT output processing on stdout and raw VT input on stdin so the
/// relayed ConPTY stream renders and keystrokes forward unbuffered.
#[cfg(windows)]
fn enable_vt_passthrough() {
    unsafe {
        // SAFETY: GetStdHandle returns a pseudo-handle; GetConsoleMode/SetConsoleMode take it
        // plus valid mode pointers/values. All failures are ignored (best-effort).
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode: u32 = 0;
        if GetConsoleMode(stdout, &mut mode) != 0 {
            let _ = SetConsoleMode(
                stdout,
                mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN,
            );
        }
        let stdin = GetStdHandle(STD_INPUT_HANDLE);
        let mut mode: u32 = 0;
        if GetConsoleMode(stdin, &mut mode) != 0 {
            let raw = (mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT;
            let _ = SetConsoleMode(stdin, raw);
        }
    }
}

/// Wait for `process` to exit and return its exit code.
///
/// # Safety
/// `process` must be a valid process handle.
#[cfg(windows)]
unsafe fn wait_and_exit_code(process: HANDLE) -> u32 {
    let wait_rc = WaitForSingleObject(process, INFINITE);
    if wait_rc != 0 {
        eprintln!("[POC] FATAL: WaitForSingleObject failed (rc={wait_rc})");
        std::process::exit(1);
    }
    let mut exit_code: u32 = 0;
    fatal_if_zero(
        GetExitCodeProcess(process, &mut exit_code),
        "GetExitCodeProcess",
    );
    exit_code
}

/// Exit the process with a descriptive FATAL message if `result` is zero.
/// Acceptable in PoC code whose sole purpose is pass/fail detection.
#[cfg(windows)]
fn fatal_if_zero(result: i32, ctx: &str) {
    if result == 0 {
        // SAFETY: GetLastError() takes no arguments and is always safe to call.
        let err = unsafe { GetLastError() };
        eprintln!("[POC] FATAL: {} failed (GetLastError={})", ctx, err);
        std::process::exit(1);
    }
}
