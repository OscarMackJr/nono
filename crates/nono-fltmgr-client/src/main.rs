//! CLI entry point for the `nono-fltmgr-client` spike binary.
//!
//! Usage (Windows only):
//! ```text
//! nono_fltmgr_client.exe <deny_path>
//! ```
//!
//! Connects to the `\NonoPolicyPort` minifilter communication port, then runs
//! the allow/deny policy loop: for each `IRP_MJ_CREATE` event received from the
//! driver, denies access if the file path matches `<deny_path>` (case-insensitive),
//! allows all others. Runs until Ctrl-C or the port disconnects.
//!
//! This binary is the runnable artifact used in the DRV-02 VM round-trip proof
//! (Plan 04 Step 5). A library-only crate cannot be used as an `.exe` in
//! `nono_fltmgr_client.exe C:\...\secret.txt`.

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <deny_path>\n\
             Example: {} C:\\nono-deny-test\\secret.txt\n\
             \n\
             Connects to \\NonoPolicyPort and denies opens of <deny_path>.\n\
             Ensure the nono minifilter driver is loaded before running.",
            args[0], args[0]
        );
        std::process::exit(1);
    }

    let deny_path = &args[1];

    eprintln!(
        "nono-fltmgr-client: connecting to \\NonoPolicyPort (deny target: {deny_path})"
    );

    match nono_fltmgr_client::run_policy_client(deny_path) {
        Ok(()) => {
            eprintln!("nono-fltmgr-client: port disconnected, exiting.");
        }
        Err(e) => {
            eprintln!("nono-fltmgr-client: fatal error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "nono_fltmgr_client is Windows-only. \
         This build target should not ship it. \
         Phase 64 D-03: cross-compile parity stub."
    );
    std::process::exit(1);
}
