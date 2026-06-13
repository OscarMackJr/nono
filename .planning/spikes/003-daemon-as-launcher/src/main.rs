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
//! v2 (after the first operator run): use ABSOLUTE write paths (engines like
//! powershell don't inherit the launcher's CWD as $PWD, so relative paths went to
//! C:\ and were — correctly — denied), and pass `--allow <exe-dir>` so nono's launch
//! policy COVERS each engine's executable path (python lives under %LOCALAPPDATA%,
//! which the runner profile doesn't cover → nono fail-secure refused to launch it).
//! That coverage requirement is itself a SEED-004 finding (see README).
//!
//! Prereqs (operator's real Win11 box): dev-layout `nono.exe` (NONO_EXE), a runner
//! profile with windows_low_il_broker:true + network.block:false (NONO_PROFILE), and a
//! USER-OWNED run dir (R-B3 — elevated console makes daemon_grant Administrators-owned).

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

struct Engine {
    name: &'static str,
    exe: String,
    /// Directory to `--allow` so nono's launch policy covers the exe (None = already
    /// covered by default system groups, e.g. System32).
    allow_dir: Option<PathBuf>,
}

fn main() {
    let nono = std::env::var("NONO_EXE")
        .unwrap_or_else(|_| r"C:\Users\OMack\Nono\target\debug\nono.exe".to_string());
    let profile = std::env::var("NONO_PROFILE")
        .unwrap_or_else(|_| "claude-code-tools-windows-runner".to_string());

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

    let mut engines: Vec<Engine> = vec![
        Engine { name: "cmd", exe: "cmd.exe".to_string(), allow_dir: None },
        Engine { name: "powershell", exe: "powershell.exe".to_string(), allow_dir: None },
    ];
    match which("python.exe") {
        Some(py) => {
            let dir = py.parent().map(|p| p.to_path_buf());
            engines.push(Engine { name: "python", exe: py.display().to_string(), allow_dir: dir });
        }
        None => println!("[SPIKE-003] python.exe not on PATH — skipping python (install it for the strongest engine-variable proof)."),
    }

    let mut all_pass = true;
    let mut tested = 0;
    for e in &engines {
        let granted = workdir.join(format!("granted_{}.txt", e.name));
        let outside = cwd.join(format!("outside_{}.txt", e.name));
        let _ = std::fs::remove_file(&granted);
        let _ = std::fs::remove_file(&outside);
        let g = granted.display().to_string();
        let o = outside.display().to_string();

        // ABSOLUTE write paths — do not rely on each engine inheriting the launcher CWD.
        let inner: Vec<String> = match e.name {
            "cmd" => vec!["/c".into(), format!("echo ok> \"{g}\" & echo no> \"{o}\"")],
            "powershell" => vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                format!("Set-Content -LiteralPath '{g}' ok; Set-Content -LiteralPath '{o}' no"),
            ],
            "python" => vec![
                "-c".into(),
                format!("open(r'{g}','w').write('ok'); open(r'{o}','w').write('no')"),
            ],
            _ => vec![],
        };

        let mut cmd = Command::new(&nono);
        cmd.arg("run").arg("--profile").arg(profile.as_str()).arg("--allow-cwd");
        if let Some(d) = &e.allow_dir {
            cmd.arg("--allow").arg(d); // cover the engine's executable path for launch
        }
        cmd.arg("--").arg(&e.exe).args(&inner).current_dir(&workdir);
        let status = cmd.status();

        let granted_ok = granted.exists();
        let outside_blocked = !outside.exists();
        let pass = granted_ok && outside_blocked;
        all_pass &= pass;
        tested += 1;

        let code = status.as_ref().ok().and_then(|s| s.code());
        println!(
            "[SPIKE-003] engine={:<11} nono_exit={code:?} granted_write={granted_ok} outside_write_blocked={outside_blocked}  => {}",
            e.name,
            if pass { "CONFINED ✓" } else { "CHECK ✗" }
        );
    }

    println!();
    println!("[SPIKE-003] ----------------------------------------------------------------");
    if tested == 0 {
        println!("[SPIKE-003] VERDICT: INCONCLUSIVE — no engines ran.");
    } else if all_pass {
        println!("[SPIKE-003] VERDICT: VALIDATED — one persistent launcher confined {tested} distinct engine(s) identically (granted write lands, outside write denied). 'Engine as a variable' holds. Finding: each engine's executable path must be COVERED by the launch policy (python needed --allow of its install dir). Next: 004 (persistent token/job reuse + multi-tenant AI_AGENT marker/IPC), 005 (engine-agnostic abstraction via nono-py).");
    } else {
        println!("[SPIKE-003] VERDICT: CHECK — an engine missed the expected boundary. If granted_write=false with an 'Access denied' to a path OUTSIDE daemon_grant (e.g. C:\\...), the engine didn't inherit the grant CWD — already fixed by absolute paths here. If nono refused to launch ('policy does not cover the executable path'), --allow the engine's exe dir (done for python). Otherwise: grant dir not user-owned (R-B3) or network.block profile without WFP. Paste the lines.");
    }
}
