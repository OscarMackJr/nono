//! CLI-owned diagnostic rendering and stderr heuristics.
//!
//! The core `nono::diagnostic` module owns the structured denial records; this
//! module owns all user-facing diagnostic UX: the `nono diagnostic` footer,
//! CLI flag suggestions, policy explanations, and best-effort parsing of a
//! command's own error output.
//!
//! The formatter sub-module is excluded from Windows builds because the
//! execution paths that consume it (`exec_strategy`, `profile_save_runtime`)
//! are Unix-only in this fork.

#[cfg(not(target_os = "windows"))]
mod formatter;

#[cfg(not(target_os = "windows"))]
pub use formatter::{
    CommandContext, DiagnosticFormatter, DiagnosticMode, ErrorObservation, PolicyExplanation,
    analyze_error_output,
};
