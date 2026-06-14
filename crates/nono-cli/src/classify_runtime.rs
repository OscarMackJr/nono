//! `nono classify <PID>` — operator-facing AI_AGENT marker check.
//!
//! # Scope (Phase 73 D-04): STRUCTURAL, NON-AUTHORITATIVE
//!
//! A standalone `nono classify` invocation runs in a **separate process** from
//! any launcher. Its [`nono::AgentRegistry`] is created fresh and empty, so the
//! registry membership check (the sound authorization predicate) returns
//! [`nono::AgentClassification::NotAnAgent`] for **every** PID. The verb therefore
//! NEVER emits an authoritative `AI_AGENT` verdict cross-process.
//!
//! Instead it performs a **structural pre-filter**:
//! - Does the PID's token carry an AppContainer package SID?
//! - Is the process in a job object?
//!
//! A "structural match" means both hold. This is explicitly non-authoritative —
//! a self-created AppContainer in a self-created job passes the filter but is not
//! in any launcher's registry. Every output path prints a disclaimer.
//!
//! Registry-backed cross-process authoritative classification is Phase 74
//! (the persistent daemon shares its `AgentRegistry` across clients).

use crate::cli::ClassifyArgs;
use nono::{NonoError, Result};
use std::sync::{Arc, Mutex};

/// Non-authoritative disclaimer printed on the structural-match path (long form
/// pointing at the Phase 74 daemon as the authoritative source).
const NOTE_LONG: &str = "  (NOTE: This check is structural only -- not an authorization decision.\n         Registry-backed authoritative classification requires Phase 74 daemon.)";

/// Non-authoritative disclaimer printed on every "not an agent" path.
const NOTE_SHORT: &str =
    "  (NOTE: This check is structural only -- not an authorization decision.)";

/// JSON-embedded form of the disclaimer (so the `--json` path is labelled too).
const NOTE_JSON: &str = "This check is structural only -- not an authorization decision. Registry-backed authoritative classification requires Phase 74 daemon.";

/// The structural outcome of classifying a PID.
enum Outcome {
    /// Has an AppContainer SID AND is in a job object (the strongest standalone
    /// verdict — still non-authoritative).
    StructuralMatch(String),
    /// Has an AppContainer SID but is NOT in a job object.
    AppContainerNoJob,
    /// Token carries no AppContainer SID (a plain Medium-IL process).
    NoAppContainer,
    /// The PID could not be opened (nonexistent / access denied / non-Windows).
    NotFound,
}

/// Entry point for `nono classify <PID>`.
///
/// Fail-secure: any error reading the target process resolves to a
/// "not an agent" verdict and `Ok(())`; the only `Err` paths are genuine
/// internal failures (mutex poison, JSON serialization).
pub(crate) fn run_classify(args: ClassifyArgs, registry: Arc<Mutex<nono::AgentRegistry>>) -> Result<()> {
    let pid = args.pid;

    // Structural pre-filter input: the AppContainer package SID (if any).
    let sid_result = nono::agent::read_process_appcontainer_sid(pid);

    // Honor the registry as the sound predicate even though, standalone, it is
    // empty (so this returns NotAnAgent for every PID). We deliberately do NOT
    // surface this as an authoritative AI_AGENT verdict — the displayed result is
    // the STRUCTURAL filter below. Locking still exercises the predicate path the
    // Phase 74 daemon will rely on, and propagates a poisoned mutex as an error.
    let _registry_verdict = registry
        .lock()
        .map_err(|_| NonoError::SandboxInit("AgentRegistry mutex poisoned".into()))?
        .classify(pid);

    let outcome = match sid_result {
        // Nonexistent PID, access denied, or non-Windows stub → fail-secure.
        Err(_) => Outcome::NotFound,
        // Not an AppContainer process.
        Ok(None) => Outcome::NoAppContainer,
        // Has an AppContainer SID: the structural verdict turns on job membership.
        Ok(Some(sid)) => {
            if process_in_job(pid) {
                Outcome::StructuralMatch(sid)
            } else {
                Outcome::AppContainerNoJob
            }
        }
    };

    if args.json {
        print_json(pid, &outcome)?;
    } else {
        print_human(pid, &outcome);
    }
    Ok(())
}

/// Human-readable output. The disclaimer NOTE is printed on every branch.
fn print_human(pid: u32, outcome: &Outcome) {
    match outcome {
        Outcome::StructuralMatch(sid) => {
            println!("PID {pid}: structural match (non-authoritative)");
            println!("  AppContainer: yes");
            println!("  In job: yes");
            println!("  Package SID: {sid}");
            println!("{NOTE_LONG}");
        }
        Outcome::AppContainerNoJob => {
            println!("PID {pid}: not an agent");
            println!("  AppContainer: yes");
            println!("  In job: no");
            println!("{NOTE_SHORT}");
        }
        Outcome::NoAppContainer => {
            println!("PID {pid}: not an agent");
            println!("  AppContainer: no");
            println!("{NOTE_SHORT}");
        }
        Outcome::NotFound => {
            println!("PID {pid}: not an agent (process not found or access denied)");
            println!("{NOTE_SHORT}");
        }
    }
}

/// JSON output. `authoritative` is always `false`; a `note` field carries the
/// disclaimer so the machine-readable path is labelled too.
fn print_json(pid: u32, outcome: &Outcome) -> Result<()> {
    let (verdict, package_sid, in_job) = match outcome {
        Outcome::StructuralMatch(sid) => ("structural_match", Some(sid.clone()), true),
        Outcome::AppContainerNoJob => ("not_an_agent", None, false),
        Outcome::NoAppContainer => ("not_an_agent", None, false),
        Outcome::NotFound => ("not_an_agent", None, false),
    };
    let value = serde_json::json!({
        "pid": pid,
        "verdict": verdict,
        "authoritative": false,
        "package_sid": package_sid,
        "in_job": in_job,
        "note": NOTE_JSON,
    });
    let rendered = serde_json::to_string_pretty(&value)
        .map_err(|e| NonoError::ConfigParse(format!("JSON serialization failed: {e}")))?;
    println!("{rendered}");
    Ok(())
}

/// Structural pre-filter: is the PID in ANY job object?
///
/// Opens the process with `PROCESS_QUERY_LIMITED_INFORMATION` and calls
/// `IsProcessInJob` with a null job handle (tests membership in any job).
/// Fail-secure: any failure returns `false`.
#[cfg(target_os = "windows")]
fn process_in_job(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::IsProcessInJob;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    // SAFETY: a valid u32 PID with the minimal QUERY_LIMITED access right; on
    // failure the handle is null and we fail-secure below.
    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if h.is_null() {
        return false;
    }
    let mut in_job: i32 = 0;
    // SAFETY: `h` is a valid open process handle; a null job handle asks whether
    // the process is in any job; `in_job` is a valid out-pointer.
    let ok = unsafe { IsProcessInJob(h, std::ptr::null_mut(), &mut in_job) };
    // SAFETY: `h` was returned by OpenProcess and is closed exactly once here.
    unsafe {
        let _ = CloseHandle(h);
    }
    ok != 0 && in_job != 0
}

/// Non-Windows stub: there is no AppContainer/job concept; never a structural match.
#[cfg(not(target_os = "windows"))]
fn process_in_job(_pid: u32) -> bool {
    false
}
