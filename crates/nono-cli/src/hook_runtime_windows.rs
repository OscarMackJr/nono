// STUB — placeholder bodies only. Full Windows implementation is in Plan 03 Task 2.
// Do NOT run this code; the stub will never be called in Plan 02 since the Windows
// execution_runtime.rs dispatch is also a stub until Plan 03 completes.
//
// This file exists so that `#[cfg(windows)] mod hook_runtime_windows;` in main.rs
// compiles cleanly on the Windows dev host (where cfg(windows) is true). Plan 03
// Task 2 replaces these placeholder bodies with the full Windows implementation.
//
#![cfg(windows)]

use crate::profile;
use nono::Result;
use std::path::Path;

/// Execute a before-hook and return exported environment variables.
///
/// STUB — placeholder only. Full implementation in Plan 03 Task 2.
pub(crate) fn execute_before_hook(
    _hook: &profile::SessionHook,
    _session_id: &str,
    _cwd: &Path,
) -> Result<Vec<(String, String)>> {
    Ok(Vec::new())
}

/// Execute an after-hook for cleanup.
///
/// STUB — placeholder only. Full implementation in Plan 03 Task 2.
pub(crate) fn execute_after_hook(
    _hook: &profile::SessionHook,
    _session_id: &str,
    _cwd: &Path,
    _child_exit_code: i32,
) -> Result<()> {
    Ok(())
}
