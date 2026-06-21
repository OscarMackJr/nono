---
phase: 86-library-boundary-convergence
reviewed: 2026-06-19T00:00:00Z
depth: standard
files_reviewed: 53
files_reviewed_list:
  - bindings/c/src/diagnostic.rs
  - bindings/c/src/lib.rs
  - bindings/c/src/types.rs
  - crates/nono-cli/src/agent_daemon/control_loop.rs
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/audit_attestation.rs
  - crates/nono-cli/src/audit_commands.rs
  - crates/nono-cli/src/audit_integrity.rs
  - crates/nono-cli/src/audit_ledger.rs
  - crates/nono-cli/src/bin/nono-agentd.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/deprecation_warnings.rs
  - crates/nono-cli/src/diagnostic/formatter.rs
  - crates/nono-cli/src/diagnostic/mod.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/health.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile_save_runtime.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/rollback_runtime.rs
  - crates/nono-cli/src/sandbox_log.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/telemetry/event.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/src/telemetry/windows.rs
  - crates/nono-cli/src/trust_cmd.rs
  - crates/nono-cli/tests/daemon_handle_baseline.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/diagnostic.rs
  - crates/nono-proxy/src/lib.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono/src/audit.rs
  - crates/nono/src/diagnostic/codes.rs
  - crates/nono/src/diagnostic/detail.rs
  - crates/nono/src/diagnostic/mod.rs
  - crates/nono/src/diagnostic/observation.rs
  - crates/nono/src/diagnostic/records.rs
  - crates/nono/src/diagnostic/report.rs
  - crates/nono/src/error.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/machine_policy.rs
  - crates/nono/src/sandbox/windows.rs
  - crates/nono/src/trust/mod.rs
  - crates/nono/src/trust/signing.rs
findings:
  critical: 2
  warning: 3
  info: 2
  total: 7
status: issues_found
---

# Phase 86: Code Review Report

**Reviewed:** 2026-06-19
**Depth:** standard
**Files Reviewed:** 53
**Status:** issues_found

## Summary

Phase 86 cherry-picked 8 upstream commits to relocate the audit and structured-diagnostics stacks from `nono-cli` into the core `nono` crate, adding FFI exposure for diagnostics. The structural work is sound: thin wrappers in CLI correctly delegate to `nono::audit`, the Merkle inclusion proof and ledger chain math are well-formed, and FFI null-pointer handling is correct for the happy path. Two security-relevant defects were found: (1) the FFI thread-local diagnostic-code store is not updated when `set_last_error` is called directly (only when called via `map_error`), leaving stale diagnostic codes visible to C callers after certain error paths in the new `nono_session_diagnostic_report_to_json` and `nono_merge_diagnostic_report_json` functions; (2) `verify_audit_log` returns `records_verified: true` for logs with missing `event_json` fields when no stored metadata is supplied, misrepresenting partial logs as fully verified. Three warnings round out the findings.

## Structural Findings (fallow)

None provided.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: FFI `set_last_error` paths leave `LAST_DIAGNOSTIC_CODE` stale

**File:** `bindings/c/src/diagnostic.rs:97-102`, `bindings/c/src/diagnostic.rs:43-63`, `bindings/c/lib.rs:54-69`

**Issue:** `set_last_error(msg)` updates only `LAST_ERROR`; it does not touch `LAST_DIAGNOSTIC_CODE` or `LAST_REMEDIATION_JSON`. In the new FFI functions added by this phase, several error return paths call `set_last_error` directly (bypassing `map_error`) and therefore leave stale state in `LAST_DIAGNOSTIC_CODE`:

- `nono_merge_diagnostic_report_json` (lines 97 and 101) calls `set_last_error("session_json is null")` and `set_last_error("invalid UTF-8 in session_json")` without resetting the diagnostic code to `Other`.
- `nono_session_diagnostic_report_to_json` (lines 46, 52, 58) calls `set_last_error(&e)` for JSON-parse failures from `parse_json_array`. These are `String` errors routed through `set_last_error`, not `NonoError` errors routed through `map_error`.

After any of these paths, a C caller who then reads `nono_last_diagnostic_code()` will receive whatever code was set by the *previous* `map_error()` call on this thread — typically `Other` from an earlier different error, but potentially `SandboxDeniedPath` or `TrustVerificationFailed` from a completely unrelated prior operation. This misrepresents the failure category to consumers (e.g., nono-py, nono-ts) who key on the diagnostic code, not just the error string.

**Fix:** Add a `set_last_error_with_code(msg: &str, code: NonoDiagnosticCode)` helper (or reset the code inside `set_last_error`) so every error-reporting path leaves the thread-local in a consistent state. For the string-only error paths in diagnostic.rs, explicitly reset the code to `Other`:

```rust
// In nono_merge_diagnostic_report_json and nono_session_diagnostic_report_to_json
// Replace bare: set_last_error("session_json is null");
// With:
set_last_error("session_json is null");
LAST_DIAGNOSTIC_CODE.with(|cell| *cell.borrow_mut() = Some(NonoDiagnosticCode::Other));
LAST_REMEDIATION_JSON.with(|cell| *cell.borrow_mut() = None);
```

Or preferably, unify all error-reporting through a single function that keeps all three thread-locals in sync.

---

### CR-02: `verify_audit_log` returns `records_verified: true` for logs with missing `event_json` when no stored metadata is provided

**File:** `crates/nono/src/audit.rs:1358-1406`

**Issue:** The guard against `missing_canonical_event_json` (line 1358) is conditioned on `stored.is_some()`:

```rust
if stored.is_some() && !leaf_hashes.is_empty() && missing_canonical_event_json {
    return Err(...);
}
```

When `stored` is `None` and some records are missing `event_json`, the function falls through and returns `records_verified: true` (line 1406). The `is_valid()` method on `AuditVerificationResult` includes `records_verified` in its conjunction, so a stripped audit log (with `event_json` fields removed) passes `is_valid()` as long as the caller omits the stored summary. This is reachable via the public `verify_audit_log` API and through the CLI's `audit verify` command when invoked without a session directory that contains `session.json`.

This is a correctness defect in the integrity guarantee: a log with absent `event_json` fields cannot prove that the written leaf hashes correspond to the canonical event JSON. The alpha scheme's security property ("no silent modification") requires that every record carry the canonical bytes used to derive its leaf hash, regardless of whether stored metadata is cross-checked.

**Fix:** Remove the `stored.is_some()` guard, or set `records_verified: false` when `missing_canonical_event_json` is true:

```rust
let records_verified = !missing_canonical_event_json;

if missing_canonical_event_json && !leaf_hashes.is_empty() {
    // Callers that don't pass stored metadata still get a result with
    // records_verified: false rather than a silent pass.
    return Ok(AuditVerificationResult {
        // ... other fields ...
        records_verified: false,
        // ...
    });
}
```

---

## Warnings

### WR-01: `BlockedCommand` variant maps to `SandboxDeniedPath` diagnostic code — semantically wrong

**File:** `crates/nono/src/error.rs:395-396`

**Issue:** The `diagnostic_code()` method maps `BlockedCommand { .. }` to `NonoDiagnosticCode::SandboxDeniedPath`. A blocked-command error (e.g., `rm -rf /` blocked by policy) is not a sandbox path-access denial — it is a policy configuration event. The `SandboxDeniedPath` code drives remediation suggestions in CLI and FFI consumers, potentially instructing users to add `--allow <path>` when the real corrective action is to fix the command or policy.

The same code is then surfaced to C FFI callers via `NonoDiagnosticCode::SandboxDeniedPath` (types.rs line 196) where the comment says "path that was sandbox-denied" — a misleading label for a command-blocked event.

**Fix:**

```rust
// In crates/nono/src/error.rs, diagnostic_code():
Self::SandboxInit(_) => NonoDiagnosticCode::SandboxDeniedPath,
Self::BlockedCommand { .. } => NonoDiagnosticCode::ConfigurationError,
```

---

### WR-02: `NonoDiagnosticCode` From impl has a silent wildcard arm that will swallow future upstream codes

**File:** `bindings/c/src/types.rs:237`

**Issue:** The `From<nono::NonoDiagnosticCode>` impl (lines 216-239) ends with `_ => Self::Other`. The library's `NonoDiagnosticCode` is marked `#[non_exhaustive]` (`crates/nono/src/diagnostic/codes.rs:20`), which means the wildcard is required by Rust. However, when upstream adds a new `NonoDiagnosticCode` variant (the explicit purpose of `#[non_exhaustive]`), the FFI layer will silently map it to `Other` and there is no compile-time signal that the FFI enum needs a new discriminant. C consumers pattern-matching on `NonoDiagnosticCode` will silently misclassify new errors.

This is not a bug in the current code — the wildcard is mandatory. The defect is the absence of any mechanism to detect the mismatch when the library grows. A new code in `nono::NonoDiagnosticCode` needs a new discriminant in `NonoDiagnosticCode` (C FFI enum) and a new match arm in the From impl; the current structure provides no early-warning mechanism.

**Fix:** Add a doc comment linking the two enums explicitly, and add a test that enumerates the full set of known variants against the From mapping so future additions surface as test failures:

```rust
/// SYNC REQUIRED: when `nono::NonoDiagnosticCode` gains a new variant,
/// add a new discriminant here AND a new arm in the From impl below.
/// See crates/nono/src/diagnostic/codes.rs.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonoDiagnosticCode { /* ... */ }
```

```rust
#[test]
fn non_exhaustive_from_covers_all_known_variants() {
    use nono::NonoDiagnosticCode as Core;
    // If this test fails to compile after an upstream sync, add the new
    // variant to the NonoDiagnosticCode C enum and the From impl.
    let known = [
        Core::SandboxDeniedPath, Core::SandboxDeniedNetwork,
        Core::SandboxDeniedUnixSocket, Core::CommandNotFound,
        Core::CommandFailedLikelySandbox, Core::CommandFailedApplication,
        Core::CredentialNotFound, Core::CredentialUnavailable,
        Core::UnsupportedPlatformFeature, Core::RollbackBudgetExceeded,
        Core::CwdAccessRequired, Core::ConfigurationError,
        Core::TrustVerificationFailed, Core::IoError,
        Core::Cancelled, Core::Other,
    ];
    for variant in &known {
        let ffi = NonoDiagnosticCode::from(*variant);
        assert_ne!(ffi, NonoDiagnosticCode::Other,
            "unexpected Other mapping for {variant:?}");
    }
}
```

---

### WR-03: `path_bytes` uses `to_string_lossy` on Windows, causing session-digest cross-platform inconsistency

**File:** `crates/nono/src/audit.rs:690-693`

**Issue:** The `path_bytes` function used in `compute_session_digest` serializes paths differently per platform:

```rust
#[cfg(unix)]
fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    path.as_os_str().as_bytes().to_vec()   // raw OS bytes
}

#[cfg(not(unix))]
fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()  // lossy UTF-8
}
```

`to_string_lossy()` replaces non-UTF-8 byte sequences with U+FFFD (`\xEF\xBF\xBD`). On Windows, paths containing non-Unicode WTF-16 surrogates that round-trip through `Path` will be replaced silently; two distinct paths could produce the same `path_bytes` output. More critically, a session digest computed on Windows and then re-verified on Linux (or vice versa, e.g., in a cross-platform audit toolchain) will differ for any path that is not valid Unicode, because the Unix branch preserves raw bytes while the Windows branch normalizes. The ledger verification result `session_digest_matches` will incorrectly return `false` for cross-platform sessions.

The impact is bounded in the current deployment (sessions start and end on the same host), but the `verify_session_in_ledger_reader` function can receive metadata from any source, and the non-determinism is a latent correctness defect in the audit integrity guarantee.

**Fix:** Normalize path representation to a platform-independent form in both branches (e.g., always use forward-slash-separated UTF-8 strings) or document the platform constraint explicitly with a test that asserts the encoding is deterministic per platform. Minimally, the Windows branch should use `path.display().to_string().into_bytes()` plus a comment explaining the deliberate encoding choice.

---

## Info

### IN-01: Duplicate doc-comment for `public_key_id_hex`

**File:** `crates/nono/src/trust/signing.rs:183-199`

**Issue:** Lines 183-188 and 191-195 both define identical doc-comments for `public_key_id_hex` (the second is for `key_id_hex`). The second doc-comment block (line 191) appears to have been copied from the first and not updated to describe `key_id_hex` accurately — the second function takes a `KeyPair`, not raw DER bytes.

**Fix:** Update the doc comment at line 191 to describe `key_id_hex` accurately:

```rust
/// Compute the key ID hex string for a `KeyPair` by exporting its public key
/// and hashing the SPKI DER bytes with SHA-256.
```

---

### IN-02: `verify_audit_log` calls `records_verified: true` without testing the no-events-no-stored case

**File:** `crates/nono/src/audit.rs:1406`, test coverage

**Issue:** There is no test that calls `verify_audit_log` on an empty log with `stored = None` and asserts the returned `AuditVerificationResult` fields. The `is_valid()` method vacuously returns `true` for an empty log (all match-fields default to `true`), which may or may not be the intended semantics for "no events, no metadata, nothing to verify". The test at `audit_integrity.rs:203` covers the `missing_event_json` rejection case only when records exist, not the empty-file case.

**Fix:** Add a test:

```rust
#[test]
fn verify_empty_audit_log_with_no_stored_metadata_is_not_is_valid() {
    let dir = tempfile::tempdir().unwrap();
    // Create an empty events file
    std::fs::write(dir.path().join(AUDIT_EVENTS_FILENAME), b"").unwrap();
    let result = verify_audit_log(dir.path(), None).unwrap();
    assert_eq!(result.event_count, 0);
    // No events = nothing proven; is_valid() should be false or at minimum
    // documented as vacuously true with a note in the code.
}
```

Alternatively, document in `is_valid()`'s doc-comment that zero-event logs vacuously return `true`.

---

_Reviewed: 2026-06-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
