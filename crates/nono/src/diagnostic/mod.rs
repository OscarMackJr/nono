//! Structured sandbox diagnostics for library and binding clients.
//!
//! Denial records, stable codes, remediations, and session reports. Footer
//! text and CLI flag formatting live in `nono-cli`.

mod codes;
mod detail;
mod observation;
mod records;
mod report;

pub use codes::{NonoDiagnostic, NonoDiagnosticCode, NonoDiagnosticSeverity, NonoRemediation};
pub use detail::{NonoDiagnosticDetail, StderrObservationKind};
pub use observation::{
    diagnostic_application_failure, diagnostic_likely_sandbox_path, diagnostic_missing_path,
    diagnostic_network_blocked, diagnostic_protected_file_write, follow_up_diagnostics,
    SessionObservationInput,
};
pub use records::{
    seatbelt_operation_to_access, DenialReason, DenialRecord, IpcDenialRecord, SandboxViolation,
};
pub use report::{dedupe_denials, filesystem_denials_from_violations, SessionDiagnosticReport};
