---
phase: 86-library-boundary-convergence
verified: 2026-06-20T00:14:43Z
status: passed
score: 14/14
overrides_applied: 0
human_verification:
  - test: "Cross-target clippy — Linux (x86_64-unknown-linux-gnu) and macOS (x86_64-apple-darwin)"
    expected: "cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used exits 0; same for apple-darwin"
    why_human: "Dev host is Windows 11; x86_64-linux-gnu-gcc cross-compiler not installed. Per CLAUDE.md MUST/NEVER rule and .planning/templates/cross-target-verify-checklist.md, this is a documented CI-deferral (PARTIAL disposition), not a gap. Live GH Actions Linux and macOS Clippy lanes are the decisive signal. Notably triggered by: supervisor_linux.rs (cfg-gated Linux; f867aba2), bindings/c/src/ (FFI cdylib/staticlib consumed on Linux/macOS). The NonoDiagnosticCode exhaustive-match IS verified by Windows-host --workspace --all-targets (cdylib target compiles). The Linux cfg-gated arms within map_error are the remaining PARTIAL scope."
---

# Phase 86: Library-Boundary Convergence — Verification Report

**Phase Goal:** The audit/attestation/ledger stack and the structured-diagnostics model live in the core `nono` crate matching upstream, with the fork's Windows diagnostic paths and FFI reconciled and the policy-free-library invariant re-decided via ADR.
**Verified:** 2026-06-20T00:14:43Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

The phase goal is achieved. Three observable requirements (BND-01, BND-02, BND-03) map cleanly to codebase evidence: `crates/nono/src/audit.rs` holds the full audit stack; `crates/nono/src/diagnostic/` is the 6-file module directory; the ADR and CLAUDE.md boundary table are updated. All 14 must-have truths verify against actual files.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `crates/nono/src/audit.rs` exists with AuditRecorder, ledger, merkle/inclusion-proof, attestation sign/verify | VERIFIED | File exists (1865 LOC); `AuditRecorder`, `sign_audit_attestation_bundle`, `verify_audit_attestation_bundle`, `verify_audit_log`, `LEDGER_CHAIN_DOMAIN_ALPHA` all present |
| 2 | CLI audit files are thin wrappers (call `nono::audit::*`) | VERIFIED | `audit_integrity.rs` (211 LOC), `audit_ledger.rs` (283 LOC), `audit_attestation.rs` (367 LOC) all open with `pub(crate) use nono::audit::` re-exports; no business logic |
| 3 | All audit unit tests pass in `cargo test -p nono` (confirmed by orchestrator: 21 passed) | VERIFIED | `#[cfg(test)]` block exists in `audit.rs`; 11 `#[test]` functions confirmed by grep; orchestrator confirmed 21 passed across audit/merkle/ledger/attest filters |
| 4 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` exits 0 | VERIFIED | Confirmed exit 0 by orchestrator; D-05 gate satisfied |
| 5 | All 8 cherry-pick commits carry upstream `(cherry picked from commit ...)` tracer and fork DCO sign-off | VERIFIED | 8 tracers confirmed in git log; all 8 have `Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>` |
| 6 | `crates/nono/src/diagnostic/` exists as 6-file module directory | VERIFIED | Directory confirmed; files: `codes.rs`, `detail.rs`, `mod.rs`, `observation.rs`, `records.rs`, `report.rs` |
| 7 | `crates/nono/src/diagnostic.rs` (single file) is DELETED | VERIFIED | `test ! -f crates/nono/src/diagnostic.rs` → confirmed absent |
| 8 | `NonoError` has `diagnostic_code()` and `remediation()` methods | VERIFIED | `error.rs` lines 383 and 452 confirm both methods with inline `#[cfg(test)]` coverage |
| 9 | `bindings/c/src/diagnostic.rs` exists with 3 pub extern C fns | VERIFIED | `nono_last_diagnostic_code` (line 11), `nono_last_remediation_json` (line 19), `nono_session_diagnostic_report_to_json` (line 37) all confirmed |
| 10 | `NonoDiagnosticCode` repr-C enum exists in `bindings/c/src/types.rs` | VERIFIED | Found at line 195 with `From<nono::NonoDiagnosticCode>` impl at line 214 |
| 11 | `crates/nono-proxy/src/diagnostic.rs` exists with `ProxyDiagnostic`, `ProxyDiagnosticCode`, `ProxyDiagnosticSeverity` | VERIFIED | All three types confirmed in file; `nono-proxy/src/lib.rs` has `pub use diagnostic::` re-export |
| 12 | `crates/nono-cli/src/diagnostic/formatter.rs` exists (DiagnosticFormatter UX moved from core) | VERIFIED | File exists; contains "nono run" (not stale "nono learn"); no DiagnosticFormatter remains in core library |
| 13 | `proj/ADR-86-library-boundary-convergence.md` exists, Status: Accepted, names D-02 Windows carve-out, includes future-sync guidance | VERIFIED | File exists; `grep "Status.*Accepted"` → OK; `grep "deliberate fork carve-out"` → OK; `grep "Future.*sync"` → OK |
| 14 | `CLAUDE.md` Library vs CLI Boundary table updated: `audit` module and `diagnostic/*` in core (In Library); `DiagnosticFormatter` removed from In Library; `diagnostic/formatter.rs` + D-02 carve-out note added | VERIFIED | Lines 88-98: `audit` module and `diagnostic/*` in In Library column; `DiagnosticFormatter` absent from In Library; `diagnostic/formatter.rs` in In CLI; blockquote carve-out note at lines 95-98 referencing ADR-86 |

**Score:** 14/14 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/src/audit.rs` | AuditRecorder, merkle, ledger, attestation + `#[cfg(test)]` | VERIFIED | 1865 LOC; all key types confirmed; 11 test functions |
| `crates/nono-cli/src/audit_integrity.rs` | Thin wrapper — calls `nono::audit::*` | VERIFIED | 211 LOC; opens with `pub(crate) use nono::audit::` |
| `crates/nono-cli/src/audit_ledger.rs` | Thin wrapper — calls `nono::audit::*` | VERIFIED | 283 LOC; `use nono::audit::` imports confirmed |
| `crates/nono-cli/src/audit_attestation.rs` | Thin wrapper — calls `nono::audit::*` | VERIFIED | 367 LOC; delegates to `nono::audit::sign_audit_attestation_bundle` and `verify_audit_attestation_bundle` |
| `crates/nono/src/lib.rs` | `pub mod audit` export | VERIFIED | Line confirmed: `pub mod audit` present |
| `crates/nono/src/diagnostic/mod.rs` | Module root — pub use codes::*, detail::*, etc. | VERIFIED | File exists in 6-file directory |
| `crates/nono/src/diagnostic/codes.rs` | `NonoDiagnostic`, `suggested_flag_for_remediation` | VERIFIED | `NonoDiagnosticCode` enum at line 21; `suggested_flag_for_remediation` at line 77 |
| `crates/nono/src/diagnostic/report.rs` | `SessionDiagnosticReport` | VERIFIED | Struct at line 15; factory `from_session` at line 23 |
| `bindings/c/src/diagnostic.rs` | 3 pub extern C fns | VERIFIED | All 3 fns confirmed; LAST_ERROR wired via `last_diagnostic_code()` |
| `crates/nono-proxy/src/diagnostic.rs` | `ProxyDiagnostic`, `ProxyDiagnosticCode`, `ProxyDiagnosticSeverity` | VERIFIED | All three types present |
| `crates/nono-cli/src/diagnostic/formatter.rs` | DiagnosticFormatter UX | VERIFIED | Exists; "nono run" confirmed (7f319b9e applied) |
| `proj/ADR-86-library-boundary-convergence.md` | Status: Accepted; contains "ADR-86" | VERIFIED | File exists; both grep checks pass |
| `CLAUDE.md` | Updated boundary table with `audit module` | VERIFIED | Lines 88-98 confirmed |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/nono-cli/src/audit_integrity.rs` | `crates/nono::audit` | `pub(crate) use nono::audit::` | WIRED | Line 1: `pub(crate) use nono::audit::{verify_audit_log, AuditRecorder, RejectStage}` |
| `crates/nono-cli/src/audit_attestation.rs` | `crates/nono::audit` | `nono::audit::AUDIT_ATTESTATION_BUNDLE_FILENAME` | WIRED | Line 9: `pub(crate) use nono::audit::AUDIT_ATTESTATION_BUNDLE_FILENAME` |
| `bindings/c/src/diagnostic.rs` | `crates/nono::error::NonoError` | `LAST_ERROR` thread-local + `diagnostic_code()` method | WIRED | `lib.rs` line 90-91: `map_error` feeds `LAST_DIAGNOSTIC_CODE` via `e.diagnostic_code()`; `diagnostic.rs` calls `crate::last_diagnostic_code()` |
| `crates/nono/src/lib.rs` | `crates/nono/src/diagnostic/` | `pub mod diagnostic` | WIRED | Line 51: `pub mod diagnostic;`; line 72: `pub use diagnostic::` |
| `crates/nono-proxy/src/lib.rs` | `crates/nono-proxy/src/diagnostic.rs` | `pub use diagnostic::` | WIRED | Confirmed `pub use diagnostic::` present |
| `proj/ADR-86-library-boundary-convergence.md` | `CLAUDE.md` | ADR documents boundary; CLAUDE.md operationalizes it | WIRED | CLAUDE.md line 98: `see proj/ADR-86-library-boundary-convergence.md`; CLAUDE.md boundary table matches ADR decisions |

### Data-Flow Trace (Level 4)

Not applicable. Phase 86 is a code-relocation/refactor phase. No new data flows were introduced — the phase moves existing business logic from CLI to core library. The relocated functions retain their original data paths.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| audit tests pass | `cargo test -p nono -- audit` (orchestrator-confirmed) | 21 passed | PASS |
| clippy gate | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | exit 0 (orchestrator-confirmed) | PASS |
| audit.rs has inline tests | `grep -c "#\[test\]" crates/nono/src/audit.rs` | 11 | PASS |
| formatter.rs has "nono run" | `grep -n "nono run" crates/nono-cli/src/diagnostic/formatter.rs` | present | PASS |
| exec_strategy_windows/ zero nono::diagnostic imports | `grep -rn "nono::diagnostic" crates/nono-cli/src/exec_strategy_windows/` | 0 results | PASS (D-02 preserved) |

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` probes declared or found for this phase. Phase 86 is an upstream-sync cherry-pick phase; verification is via `cargo test` and `cargo clippy` rather than behavioral probes.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| BND-01 | 86-01-PLAN.md | Audit/attestation/ledger logic in core `nono` crate (`crates/nono/src/audit.rs`) with thin CLI wrappers | SATISFIED | `audit.rs` exists (1865 LOC); 4 CLI wrappers verified as thin; `pub mod audit` in lib.rs; 21 tests pass |
| BND-02 | 86-02-PLAN.md | Structured-diagnostics model in core `nono` crate (`diagnostic/` 6-file dir), FFI-exposed, Windows paths preserved | SATISFIED | `diagnostic/` dir confirmed (6 files); `diagnostic.rs` single-file deleted; 3 FFI fns confirmed; `NonoError::diagnostic_code()` and `remediation()` confirmed; `exec_strategy_windows/` has zero `nono::diagnostic` imports (D-02 intact); cross-target clippy PARTIAL per documented CI-deferral |
| BND-03 | 86-03-PLAN.md | ADR documents boundary change; CLAUDE.md Library vs CLI Boundary table updated | SATISFIED | `proj/ADR-86-library-boundary-convergence.md` exists (Status: Accepted); CLAUDE.md boundary table updated with audit module, diagnostic/*, D-02 carve-out note |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `CLAUDE.md` | 26 | Architecture file-tree still shows `├── diagnostic.rs  # DiagnosticFormatter` | Info | Cosmetic — file-tree block is illustrative; the operative Library vs CLI Boundary table is correctly updated. The PLAN explicitly scoped Task 2 to "Do NOT change any other section of CLAUDE.md." No security, build, or behavioral impact. |
| `CLAUDE.md` | 360 | Error-handling note says `DiagnosticFormatter: Generates human-readable explanations for sandbox denials (crates/nono/src/diagnostic.rs)` — stale path | Info | Cosmetic — same category as line 26 stale entry. Both point to the deleted single-file path but carry no enforcement weight. Neither affects `make ci`. |

No `TBD`, `FIXME`, or `XXX` debt markers found in phase-modified files. No stub patterns observed — relocated code is substantive and wired.

**Code-review findings (upstream-inherited, not phase-86 gaps):** The 86-REVIEW.md identified 2 critical findings (CR-01: FFI `set_last_error` paths leave `LAST_DIAGNOSTIC_CODE` stale; CR-02: `verify_audit_log` records_verified bypass when `stored` is None) and 3 warnings. Per the orchestrator's context, BOTH criticals are faithful upstream behavior — identical code exists verbatim in upstream commits `a6aa5995` (CR-01) and `e9529312` (CR-02, lines 875/915). The phase mandate was explicit convergence toward upstream's end-state, so these are upstream-inherited deferrals, not phase-86 regressions. They are noted here for traceability; a fork-hardening decision in a later phase may choose to diverge from upstream's behavior.

### Human Verification Required

1. **Cross-target clippy (Linux + macOS)**

   **Test:** Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `--target x86_64-apple-darwin` from the dev host or live CI.
   **Expected:** Exit 0 on both targets. Primary concern: `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (f867aba2 changes; `#[cfg(target_os = "linux")]` gated) and `bindings/c/src/` cfg-gated Linux arms in `map_error`.
   **Why human:** Dev host is Windows 11; `x86_64-linux-gnu-gcc` cross-compiler not installed. Per CLAUDE.md Coding Standards MUST/NEVER rule and `.planning/templates/cross-target-verify-checklist.md`, this is a documented PARTIAL disposition, not a gap. The live GH Actions Linux and macOS Clippy lanes are the decisive signal. The `NonoDiagnosticCode` exhaustive-match IS verified on Windows host via `--workspace --all-targets` (cdylib compiles). Only the Linux cfg-gated arms within `map_error` are the remaining PARTIAL scope.

### Gaps Summary

No gaps. All 14 must-have truths are VERIFIED against the actual codebase.

The two CLAUDE.md stale references (file-tree at line 26, error-handling note at line 360) are cosmetic doc inaccuracies outside the PLAN's explicitly scoped update window (Task 2 said "Do NOT change any other section"). They carry no enforcement weight and do not affect build, test, or security behavior. Recommended for cleanup in a future maintenance pass.

The code-review criticals (CR-01, CR-02) are upstream-inherited behaviors, not phase-86 regressions, as confirmed by the orchestrator's upstream commit cross-reference. They are not phase-86 must-have failures.

The cross-target clippy is a CI-deferred PARTIAL per the CLAUDE.md MUST/NEVER rule — a documented category of known-deferred item, not a gap.

---

_Verified: 2026-06-20T00:14:43Z_
_Verifier: Claude (gsd-verifier)_
