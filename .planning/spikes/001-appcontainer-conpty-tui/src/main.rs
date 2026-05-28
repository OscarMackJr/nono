//! Spike 001: appcontainer-conpty-tui
//!
//! KILLER question: can a process confined in a Windows **AppContainer** attach to a ConPTY and act
//! as a console client (render output / a TUI) WITHOUT re-tripping the Phase-30 0xC0000142
//! STATUS_DLL_INIT_FAILED that kills a raw Low-IL child? (AppContainer tokens are themselves Low
//! integrity + an AppContainer SID + capability SIDs — so this may hit the SAME console-subsystem
//! wall. That is exactly what this spike tests.)
//!
//! Apples-to-apples with the dead Option B' PoC (poc-broker --conpty), but the child is spawned in an
//! AppContainer (PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES) instead of with a raw Low-IL token.
//!
//! Build: cargo build --release --target x86_64-pc-windows-msvc
//! Run:   .\target\release\appcontainer-conpty-tui.exe
//!        At the cmd prompt that (hopefully) appears: `echo HI`  (console signal),
//!        then optionally `powershell -NoLogo`, `claude`, and `exit`.
//!
//! Verdict reading:
//!   child exit 0xC0000142  => AppContainer ALSO hits the console wall → idea DEAD (stop; 002-004 moot).
//!   `echo HI` prints + prompt returns => AppContainer console client WORKS → proceed to 002-004.

#[cfg(not(windows))]
fn main() {
    eprintln!("[SPIKE-001] Windows-only. Build with: cargo build --release --target x86_64-pc-windows-msvc");
}

#[cfg(windows)]
use std::{ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt, os::windows::io::FromRawHandle};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
#[cfg(windows)]
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
#[cfg(windows)]
use windows_sys::Win32::Security::{FreeSid, SECURITY_CAPABILITIES};
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
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
    InitializeProcThreadAttributeList, UpdateProcThreadAttribute, WaitForSingleObject,
    EXTENDED_STARTUPINFO_PRESENT, INFINITE, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    STARTUPINFOEXW, STARTUPINFOW,
};

#[cfg(windows)]
type Psid = *mut core::ffi::c_void;

#[cfg(windows)]
const APPCONTAINER_NAME: &str = "nono.spike.appcontainer.conpty.tui";
// HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS)
#[cfg(windows)]
const E_ALREADY_EXISTS: i32 = 0x8007_00B7u32 as i32;
#[cfg(windows)]
const STATUS_DLL_INIT_FAILED: u32 = 0xC000_0142;

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn main() {
    println!("[SPIKE-001] AppContainer + ConPTY: spawn cmd.exe confined in an AppContainer, attached");
    println!("[SPIKE-001] to a ConPTY, and relay stdio. Tests whether an AppContainer process can be a");
    println!("[SPIKE-001] console client without the Phase-30 0xC0000142 that kills a raw Low-IL child.");
    println!("[SPIKE-001] At the prompt: `echo HI` (console signal), then optionally `powershell -NoLogo`,");
    println!("[SPIKE-001] `claude`, and `exit`.\n");

    unsafe {
        // SAFETY: AllocConsole takes no args; non-fatal if already attached.
        AllocConsole();
    }

    // 1. Create (or derive) the AppContainer profile SID.
    let name_w = wide(APPCONTAINER_NAME);
    let mut ac_sid: Psid = std::ptr::null_mut();
    let hr = unsafe {
        // SAFETY: name_w is null-terminated UTF-16; ac_sid is a valid out-pointer. Empty caps (count 0).
        CreateAppContainerProfile(
            name_w.as_ptr(),
            name_w.as_ptr(),
            name_w.as_ptr(),
            std::ptr::null(),
            0,
            &mut ac_sid,
        )
    };
    if hr != 0 {
        if hr == E_ALREADY_EXISTS {
            let d = unsafe { DeriveAppContainerSidFromAppContainerName(name_w.as_ptr(), &mut ac_sid) };
            if d != 0 {
                eprintln!("[SPIKE-001] FATAL: DeriveAppContainerSidFromAppContainerName failed 0x{d:X}");
                std::process::exit(1);
            }
        } else {
            eprintln!("[SPIKE-001] FATAL: CreateAppContainerProfile failed 0x{hr:X}");
            std::process::exit(1);
        }
    }
    println!("[SPIKE-001] AppContainer profile ready (name={APPCONTAINER_NAME}, 0 capabilities).");

    // 2. ConPTY pipes + pseudoconsole (created in THIS Medium-IL parent, as in the B' PoC).
    let mut in_read: HANDLE = INVALID_HANDLE_VALUE;
    let mut in_write: HANDLE = INVALID_HANDLE_VALUE;
    let mut out_read: HANDLE = INVALID_HANDLE_VALUE;
    let mut out_write: HANDLE = INVALID_HANDLE_VALUE;
    unsafe {
        // SAFETY: out-pointers valid; default security; no inheritance needed (pseudoconsole owns refs).
        if CreatePipe(&mut in_read, &mut in_write, std::ptr::null(), 0) == 0 {
            fatal("CreatePipe(input)");
        }
        if CreatePipe(&mut out_read, &mut out_write, std::ptr::null(), 0) == 0 {
            fatal("CreatePipe(output)");
        }
    }
    let mut hpcon: HPCON = 0;
    let size = COORD { X: 120, Y: 30 };
    let hr = unsafe {
        // SAFETY: in_read/out_write are valid pipe handles; hpcon is a valid out-pointer.
        CreatePseudoConsole(size, in_read, out_write, 0, &mut hpcon)
    };
    unsafe {
        // SAFETY: pseudoconsole keeps its own references; release our child-end copies.
        CloseHandle(in_read);
        CloseHandle(out_write);
    }
    if hr != 0 {
        eprintln!("[SPIKE-001] FATAL: CreatePseudoConsole failed HRESULT 0x{hr:X}");
        std::process::exit(1);
    }

    // 3. Attribute list (2 slots): SECURITY_CAPABILITIES (AppContainer) + PSEUDOCONSOLE.
    let mut sec_caps = SECURITY_CAPABILITIES {
        AppContainerSid: ac_sid,
        Capabilities: std::ptr::null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    let hpcon_value = hpcon;
    let mut attr_size: usize = 0;
    unsafe {
        // SAFETY: probe call with null list returns the required size for 2 attributes.
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 2, 0, &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size];
    let attr_list: LPPROC_THREAD_ATTRIBUTE_LIST =
        attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
    unsafe {
        // SAFETY: attr_list sized by the probe; sec_caps/hpcon_value outlive CreateProcess below.
        if InitializeProcThreadAttributeList(attr_list, 2, 0, &mut attr_size) == 0 {
            fatal("InitializeProcThreadAttributeList");
        }
        if UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            std::ptr::addr_of_mut!(sec_caps) as *mut _,
            size_of::<SECURITY_CAPABILITIES>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) == 0
        {
            fatal("UpdateProcThreadAttribute(SECURITY_CAPABILITIES)");
        }
        if UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            std::ptr::addr_of!(hpcon_value) as *mut _,
            size_of::<HPCON>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) == 0
        {
            fatal("UpdateProcThreadAttribute(PSEUDOCONSOLE)");
        }
    }

    // 4. Spawn cmd.exe in the AppContainer, attached to the ConPTY. bInheritHandles=FALSE.
    let mut cmd_wide = wide("cmd.exe");
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.lpAttributeList = attr_list;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let created = unsafe {
        // SAFETY: cmd_wide null-terminated; EXTENDED_STARTUPINFO_PRESENT matches STARTUPINFOEXW;
        // the SECURITY_CAPABILITIES attribute places the child in the AppContainer.
        CreateProcessW(
            std::ptr::null(),
            cmd_wide.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT,
            std::ptr::null(),
            std::ptr::null(),
            &si.StartupInfo as *const STARTUPINFOW,
            &mut pi,
        )
    };
    unsafe {
        // SAFETY: attr_list initialized above; release after CreateProcess.
        DeleteProcThreadAttributeList(attr_list);
    }
    if created == 0 {
        let err = unsafe { GetLastError() };
        eprintln!(
            "[SPIKE-001] FAIL — CreateProcessW (AppContainer + PSEUDOCONSOLE) rejected at creation \
             (GetLastError={err}). The AppContainer+ConPTY spawn shape is refused before the child runs."
        );
        cleanup(ac_sid, &name_w, hpcon);
        std::process::exit(1);
    }
    println!(
        "[SPIKE-001] Spawned cmd.exe in the AppContainer on the ConPTY, PID {}.\n",
        pi.dwProcessId
    );

    // 5. VT pass-through + stdio relay.
    enable_vt_passthrough();
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

    // 6. Wait + interpret.
    let exit_code = unsafe { wait_and_exit_code(pi.hProcess) };
    println!("\n[SPIKE-001] child (cmd.exe) exit code: {exit_code:#010x} ({exit_code})");
    match exit_code {
        STATUS_DLL_INIT_FAILED => println!(
            "[SPIKE-001] FAIL — 0xC0000142: an AppContainer process ALSO re-trips the console-subsystem \
             loader crash. AppContainer does not escape the Low-IL console wall → idea DEAD; spikes 002-004 \
             are moot."
        ),
        _ => println!(
            "[SPIKE-001] cmd.exe survived (no 0xC0000142). Verdict depends on what you saw ABOVE: if \
             `echo HI` printed and the prompt returned, an AppContainer CAN be a ConPTY console client → \
             proceed to spikes 002-004 (isolation parity). If it hung or showed nothing, capture details."
        ),
    }

    unsafe {
        // SAFETY: process handles valid and owned; out_read/in_write owned by the relay File wrappers.
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }
    cleanup(ac_sid, &name_w, hpcon);
    std::process::exit(exit_code as i32);
}

#[cfg(windows)]
fn cleanup(ac_sid: Psid, name_w: &[u16], hpcon: HPCON) {
    unsafe {
        // SAFETY: hpcon valid; ac_sid from CreateAppContainerProfile/Derive is freed with FreeSid;
        // DeleteAppContainerProfile removes the profile we created (best-effort).
        ClosePseudoConsole(hpcon);
        if !ac_sid.is_null() {
            FreeSid(ac_sid);
        }
        let _ = DeleteAppContainerProfile(name_w.as_ptr());
    }
}

#[cfg(windows)]
fn enable_vt_passthrough() {
    unsafe {
        // SAFETY: std handles are pseudo-handles; mode get/set are best-effort (failures ignored).
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

#[cfg(windows)]
unsafe fn wait_and_exit_code(process: HANDLE) -> u32 {
    if WaitForSingleObject(process, INFINITE) != 0 {
        eprintln!("[SPIKE-001] FATAL: WaitForSingleObject failed");
        std::process::exit(1);
    }
    let mut code: u32 = 0;
    if GetExitCodeProcess(process, &mut code) == 0 {
        eprintln!("[SPIKE-001] FATAL: GetExitCodeProcess failed");
        std::process::exit(1);
    }
    code
}

#[cfg(windows)]
fn fatal(ctx: &str) {
    let err = unsafe { GetLastError() };
    eprintln!("[SPIKE-001] FATAL: {ctx} failed (GetLastError={err})");
    std::process::exit(1);
}
