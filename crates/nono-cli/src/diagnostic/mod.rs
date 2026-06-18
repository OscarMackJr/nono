//! CLI diagnostic footer and stderr parsing.
//!
//! Structured denial records live in `nono::diagnostic`. This module renders
//! them and applies CLI-specific policy labels and flag formatting.
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
