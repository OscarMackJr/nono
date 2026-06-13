//! SEED-004 Spike 002 — post-hoc-token-confine (KILLER)
//!
//! Question: can a "daemon" confine an agent process it did NOT spawn, by lowering
//! that already-running process's primary-token integrity from outside?
//!
//! Method (uses a NON-cooperating standard binary, `cmd.exe`, driven via stdin):
//!   1. Spawn `cmd.exe` with a piped stdin (Medium IL, like any agent we didn't launch).
//!   2. Drive a PRE-confinement write to a Medium-IL %TEMP% file  → baseline (should succeed).
//!   3. Open the running cmd's primary token (OpenProcessToken, TOKEN_ADJUST_DEFAULT|TOKEN_QUERY)
//!      and lower its integrity to Low via SetTokenInformation(TokenIntegrityLevel)
//!      — mirroring crates/nono/src/sandbox/windows.rs::apply_low_il_label, but targeting
//!      ANOTHER process. This is the "confine after the fact" attempt.
//!   4. Drive a POST-confinement write to a Medium-IL %TEMP% file.
//!   5. Verdict from observable facts:
//!        - SetTokenInformation fails           → INVALIDATED (cannot even lower a running token)
//!        - lowered, POST write DENIED            → PARTIAL (post-hoc IL lowering DID take effect for
//!                                                  NEW opens; but note the leaked-handle + no
//!                                                  restricting-SID + no grant-relabel caveats)
//!        - lowered, POST write SUCCEEDS          → INVALIDATED (lowering had no enforcement effect)
//!
//! Run on real Win11 from a NORMAL (non-elevated) PowerShell console as the same user.

#[cfg(not(windows))]
fn main() {
    eprintln!("[SPIKE-002] Windows-only spike.");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    use std::ffi::c_void;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::Security::{
        CreateWellKnownSid, GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation,
        SetTokenInformation, TokenIntegrityLevel, WinLowLabelSid, SECURITY_MAX_SID_SIZE,
        TOKEN_ADJUST_DEFAULT, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::SystemServices::SE_GROUP_INTEGRITY;
    use windows_sys::Win32::System::Threading::OpenProcessToken;

    // --- best-effort query of a token's integrity RID (0x2000=Medium, 0x1000=Low) ---
    unsafe fn query_il_rid(token: HANDLE) -> Option<u32> {
        let mut len = 0u32;
        GetTokenInformation(token, TokenIntegrityLevel, std::ptr::null_mut(), 0, &mut len);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        if GetTokenInformation(
            token,
            TokenIntegrityLevel,
            buf.as_mut_ptr() as *mut c_void,
            len,
            &mut len,
        ) == 0
        {
            return None;
        }
        let label = buf.as_ptr() as *const TOKEN_MANDATORY_LABEL;
        let sid = (*label).Label.Sid;
        if sid.is_null() {
            return None;
        }
        let count_ptr = GetSidSubAuthorityCount(sid);
        if count_ptr.is_null() {
            return None;
        }
        let count = *count_ptr;
        if count == 0 {
            return None;
        }
        let rid_ptr = GetSidSubAuthority(sid, (count - 1) as u32);
        if rid_ptr.is_null() {
            return None;
        }
        Some(*rid_ptr)
    }

    // --- lower a target token to Low IL (mirrors apply_low_il_label, different target) ---
    unsafe fn lower_token_to_low(token: HANDLE) -> Result<(), u32> {
        let mut sid_buffer = [0u8; SECURITY_MAX_SID_SIZE as usize];
        let mut sid_size = sid_buffer.len() as u32;
        if CreateWellKnownSid(
            WinLowLabelSid,
            std::ptr::null_mut(),
            sid_buffer.as_mut_ptr() as *mut c_void,
            &mut sid_size,
        ) == 0
        {
            return Err(GetLastError());
        }
        let label_size = std::mem::size_of::<TOKEN_MANDATORY_LABEL>() + sid_size as usize;
        let mut label_buffer = vec![0u8; label_size];
        let label_ptr = label_buffer.as_mut_ptr() as *mut TOKEN_MANDATORY_LABEL;
        let sid_ptr = label_buffer
            .as_mut_ptr()
            .add(std::mem::size_of::<TOKEN_MANDATORY_LABEL>()) as *mut c_void;
        std::ptr::copy_nonoverlapping(sid_buffer.as_ptr(), sid_ptr as *mut u8, sid_size as usize);
        (*label_ptr).Label.Sid = sid_ptr;
        (*label_ptr).Label.Attributes = SE_GROUP_INTEGRITY as u32;
        if SetTokenInformation(
            token,
            TokenIntegrityLevel,
            label_ptr as *const c_void,
            label_size as u32,
        ) == 0
        {
            return Err(GetLastError());
        }
        Ok(())
    }

    fn rid_name(rid: Option<u32>) -> String {
        match rid {
            Some(0x4000) => "System (0x4000)".into(),
            Some(0x3000) => "High (0x3000)".into(),
            Some(0x2000) => "Medium (0x2000)".into(),
            Some(0x1000) => "Low (0x1000)".into(),
            Some(other) => format!("0x{other:04X}"),
            None => "unknown".into(),
        }
    }

    let temp = std::env::var("TEMP").unwrap_or_else(|_| "C:\\Windows\\Temp".to_string());
    let pre = format!("{temp}\\spike002_pre.txt");
    let post = format!("{temp}\\spike002_post.txt");
    let _ = std::fs::remove_file(&pre);
    let _ = std::fs::remove_file(&post);

    println!("[SPIKE-002] post-hoc-token-confine — can we confine a process we did NOT spawn?");
    println!("[SPIKE-002] TEMP target dir (Medium-IL): {temp}");

    // 1. Spawn a non-cooperating cmd.exe with piped stdin.
    let mut child = Command::new("cmd.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let hproc = child.as_raw_handle() as HANDLE;
    println!("[SPIKE-002] spawned cmd.exe pid={}", child.id());

    // 2. PRE-confinement write (Medium IL → Medium %TEMP%: should succeed).
    {
        let stdin = child.stdin.as_mut().expect("child stdin piped");
        writeln!(stdin, "echo pre> \"{pre}\"")?;
        stdin.flush()?;
    }
    std::thread::sleep(Duration::from_millis(700));
    println!("[SPIKE-002] pre-confinement write issued (exists={})", std::path::Path::new(&pre).exists());

    // 3. Open the running cmd's token and attempt to lower its IL from outside.
    let mut htok: HANDLE = std::ptr::null_mut();
    let opened = unsafe {
        OpenProcessToken(hproc, TOKEN_ADJUST_DEFAULT | TOKEN_QUERY, &mut htok)
    };
    if opened == 0 {
        let gle = unsafe { GetLastError() };
        println!("[SPIKE-002] OpenProcessToken FAILED GLE={gle}");
        println!("[SPIKE-002] VERDICT: INVALIDATED — cannot open the running agent's token to confine it.");
        let _ = child.kill();
        return Ok(());
    }
    let before = unsafe { query_il_rid(htok) };
    println!("[SPIKE-002] target IL before = {}", rid_name(before));

    let lower_result = unsafe { lower_token_to_low(htok) };
    match &lower_result {
        Ok(()) => println!("[SPIKE-002] SetTokenInformation(Low) returned SUCCESS"),
        Err(gle) => println!("[SPIKE-002] SetTokenInformation(Low) FAILED GLE={gle}"),
    }
    let after = unsafe { query_il_rid(htok) };
    println!("[SPIKE-002] target IL after  = {}", rid_name(after));
    unsafe {
        CloseHandle(htok);
    }

    // 4. POST-confinement write (if lowering took effect: Low → Medium %TEMP% should be DENIED).
    {
        let stdin = child.stdin.as_mut().expect("child stdin piped");
        writeln!(stdin, "echo post> \"{post}\"")?;
        writeln!(stdin, "exit")?;
        stdin.flush()?;
    }
    let _ = child.wait()?;

    let pre_ok = std::path::Path::new(&pre).exists();
    let post_ok = std::path::Path::new(&post).exists();
    println!("[SPIKE-002] result: pre_exists={pre_ok} post_exists={post_ok}");

    // 5. Verdict.
    println!("[SPIKE-002] ----------------------------------------------------------------");
    match lower_result {
        Err(gle) => {
            println!("[SPIKE-002] VERDICT: INVALIDATED — could not lower a running process's primary-token IL (GLE={gle}). Post-hoc confinement of the primary token is not available; the daemon must confine at SPAWN time (pivot to daemon-as-launcher, spike 003).");
        }
        Ok(()) => {
            if pre_ok && !post_ok {
                println!("[SPIKE-002] VERDICT: PARTIAL — post-hoc IL lowering took effect for NEW opens (post write to Medium-IL %TEMP% was DENIED). BUT this is not a sound confinement boundary: (a) handles the agent opened BEFORE lowering keep their access (leak), (b) no restricting SID / no CWD grant relabel / network not covered. A daemon-as-launcher (spike 003) avoids the leak by confining before any agent code runs.");
            } else if pre_ok && post_ok {
                println!("[SPIKE-002] VERDICT: INVALIDATED — lowering the running token had NO enforcement effect (post write to Medium-IL %TEMP% still SUCCEEDED). Cannot confine a running agent post-hoc; pivot to daemon-as-launcher (spike 003).");
            } else {
                println!("[SPIKE-002] VERDICT: INCONCLUSIVE — pre_exists={pre_ok} post_exists={post_ok}. Re-run; ensure %TEMP% is Medium-IL and the user owns it. (If pre did not land, cmd stdin timing was off.)");
            }
        }
    }
    Ok(())
}
