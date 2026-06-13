//! SEED-004 Spike 003 — daemon-as-launcher
//!
//! Question: can ONE persistent launcher ("daemon") mediate arbitrary, *distinct*
//! agent engines — confining each identically — rather than only Claude Code?
//!
//! The confinement primitive (`nono run -- <exe>`) is already engine-neutral and
//! proven (broker arm; A6-3a landed a confined write). So this spike does NOT
//! re-implement token code — it delegates confinement to `nono run` and focuses on
//! the *engine-as-a-variable* claim: one process launches cmd.exe, powershell.exe,
//! and (if present) python.exe through the same confined path, and each shows the
//! same boundary — a write to the GRANTED workdir lands, a write OUTSIDE it is denied.
//!
//! True persistent-token reuse / multi-tenant IPC is spike 004; the abstraction
//! boundary / Python-binding path is spike 005. This spike proves the launcher shape
//! + engine neutrality.
//!
//! Prereqs (operator's real Win11 box, from the A6 testing):
//!   - dev-layout `nono.exe` (broker trust-gate exempt) — default C:\Users\OMack\Nono\target\debug\nono.exe
//!     (override with NONO_EXE).
//!   - a runner profile with `windows_low_il_broker:true` AND `network.block:false`
//!     (the UAT variant) — default `claude-code-tools-windows-runner` (override NONO_PROFILE).
//!   - run from a USER-OWNED dir (R-B3): an elevated console makes the granted subdir
//!     Administrators-owned and confined writes are denied. `takeown /F daemon_grant` if needed.

use std::path::PathBuf;
use std::process::Command;

fn which(exe: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let cand = dir.join(exe);
            if cand.is_file() {
                Some(cand)
            } else {
                None
            }
        })
    })
}

fn main() {
    let nono = std::env::var("NONO_EXE")
        .unwrap_or_else(|_| r"C:\Users\OMack\Nono\target\debug\nono.exe".to_string());
    let profile =
        std::env::var("NONO_PROFILE").unwrap_or_else(|_| "claude-code-tools-windows-runner".to_string());

    let cwd = std::env::current_dir().expect("cwd");
    let workdir = cwd.join("daemon_grant"); // the GRANTED (relabeled-Low) workdir
    std::fs::create_dir_all(&workdir).expect("create granted workdir");

    println!("[SPIKE-003] daemon-as-launcher — one persistent launcher, many engines, each confined");
    println!("[SPIKE-003] nono     = {nono}");
    println!("[SPIKE-003] profile  = {profile}");
    println!("[SPIKE-003] granted  = {}", workdir.display());
    println!("[SPIKE-003] outside  = {} (parent of grant; writes here must be DENIED)", cwd.display());
    println!("[SPIKE-003] NOTE: granted dir must be USER-OWNED (R-B3) or confined writes are denied.");
    println!();

    // Engine list: cmd + powershell always; python if on PATH (the convincing non-shell engine).
    let mut engines: Vec<(&str, String, Vec<String>)> = Vec::new();

    // cmd.exe: write granted (relative -> workdir), attempt outside (..\ -> parent).
    engines.push((
        "cmd",
        "cmd.exe".to_string(),
        vec![
            "/c".into(),
            "echo ok> granted_cmd.txt & echo no> ..\\outside_cmd.txt".into(),
        ],
    ));

    // powershell.exe (Windows PowerShell 5.1; runs in CLM under the broker arm — Set-Content is CLM-safe).
    engines.push((
        "powershell",
        "powershell.exe".to_string(),
        vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            "Set-Content granted_powershell.txt ok; Set-Content ..\\outside_powershell.txt no".into(),
        ],
    ));

    // python.exe — the real non-Claude/non-shell engine. Skip if not installed.
    match which("python.exe") {
        Some(py) => engines.push((
            "python",
            py.display().to_string(),
            vec![
                "-c".into(),
                "open('granted_python.txt','w').write('ok'); open(r'..\\outside_python.txt','w').write('no')"
                    .into(),
            ],
        )),
        None => println!("[SPIKE-003] python.exe not on PATH — skipping the python engine (install it for the strongest engine-variable proof)."),
    }

    let mut all_pass = true;
    let mut tested = 0;
    for (name, exe, inner) in &engines {
        let granted = workdir.join(format!("granted_{name}.txt"));
        let outside = cwd.join(format!("outside_{name}.txt"));
        let _ = std::fs::remove_file(&granted);
        let _ = std::fs::remove_file(&outside);

        // The persistent launcher mediates this engine via the proven confined primitive.
        let mut cmd = Command::new(&nono);
        cmd.arg("run")
            .arg("--profile")
            .arg(profile.as_str())
            .arg("--allow-cwd")
            .arg("--")
            .arg(exe)
            .args(inner)
            .current_dir(&workdir);
        let status = cmd.status();

        let granted_ok = granted.exists();
        let outside_blocked = !outside.exists();
        let pass = granted_ok && outside_blocked;
        all_pass &= pass;
        tested += 1;

        let code = status.as_ref().ok().and_then(|s| s.code());
        println!(
            "[SPIKE-003] engine={name:<11} nono_exit={code:?} granted_write={granted_ok} outside_write_blocked={outside_blocked}  => {}",
            if pass { "CONFINED ✓" } else { "CHECK ✗" }
        );
    }

    println!();
    println!("[SPIKE-003] ----------------------------------------------------------------");
    if tested == 0 {
        println!("[SPIKE-003] VERDICT: INCONCLUSIVE — no engines ran.");
    } else if all_pass {
        println!("[SPIKE-003] VERDICT: VALIDATED — one persistent launcher confined {tested} distinct engine(s) identically (granted write lands, outside write denied). 'Engine as a variable' holds; the launcher shape works. Next: 004 (persistent-token reuse + multi-tenant AI_AGENT marker/IPC), 005 (engine-agnostic abstraction via the nono-py binding).");
    } else {
        println!("[SPIKE-003] VERDICT: CHECK — at least one engine did not show the expected boundary. Common causes (NOT confinement bugs): granted dir not user-owned (R-B3 — takeown it), profile has network.block:true without the WFP service running (use the network.block:false variant), or non-dev-layout nono (broker trust gate). Re-check and re-run.");
    }
}
