# ADR-86: Library-Boundary Convergence — Audit Stack and Structured Diagnostics to Core Crate

**Status:** Accepted
**Phase:** 86 — Library-Boundary Convergence
**Date:** 2026-06-19
**Authors:** Phase 86 planning

---

## Context

Prior to Phase 86, the `nono` library was a **pure sandbox primitive**: all audit logic, all
diagnostic UX, and all security policy lived in `nono-cli`. The library applied only what clients
explicitly added to `CapabilitySet`. Diagnostic rendering (`DiagnosticFormatter`) lived in
`crates/nono/src/diagnostic.rs` as a single-file module; audit logic
(`audit_attestation.rs`, `audit_commands.rs`, `audit_integrity.rs`, `audit_session.rs`) lived
entirely in `nono-cli`.

Upstream `always-further/nono` v0.63.0–v0.64.0 (8 commits across DIVERGENCE-LEDGER Clusters A
and B) moved this logic:

- **Cluster A (4 commits):** `AuditRecorder`, ledger append/verify, merkle/inclusion-proof, and
  attestation sign/verify relocated from `nono-cli` into `crates/nono/src/audit.rs`. The 4
  CLI-side audit files became thin wrappers re-exporting from `nono::audit`.
- **Cluster B (4 commits):** `DiagnosticFormatter` (UX rendering) moved from the core library
  into `crates/nono-cli`; the single-file `crates/nono/src/diagnostic.rs` was replaced by a
  6-file module directory (`codes.rs`, `detail.rs`, `mod.rs`, `observation.rs`, `records.rs`,
  `report.rs`) containing structured facts, codes, and reports — NOT UX. `NonoError` gained
  `diagnostic_code()` and `remediation()` methods. Net-new: `bindings/c/src/diagnostic.rs`
  (3 `pub extern "C"` diagnostic functions) and `crates/nono-proxy/src/diagnostic.rs`
  (`ProxyDiagnostic`, `ProxyDiagnosticCode`, `ProxyDiagnosticSeverity`).

Phase 85 (DIVERGENCE-LEDGER audit, D-03) dispositioned both clusters as **will-sync /
adopt-upstream**, locking the decision to converge rather than fork-preserve. This ADR records
the resulting boundary change, the single deliberate fork carve-out, and guidance for future
upstream sync auditors.

The fork faced its highest merge risk: approximately 2200 LOC relocated, FFI surface added,
Windows diagnostic paths preserved under constraint SC#3 (no regression in Windows denial
output).

---

## Decisions

### Decision 1 — Adopt upstream's audit relocation (Cluster A)

`AuditRecorder`, ledger append/verify, merkle/inclusion-proof, and attestation sign/verify move
into `crates/nono/src/audit.rs`. The 4 CLI-side files (`audit_integrity.rs`,
`audit_ledger.rs`, `audit_attestation.rs`, `audit_session.rs`) are reduced to thin wrappers
with `pub(crate) use nono::audit::*` re-exports.

Unit tests for the relocated audit logic (recorder lifecycle, merkle/inclusion-proof,
ledger append/verify, attestation sign/verify) move into `crates/nono/src/audit.rs` alongside
the moved code, per D-04 (tests live with their unit-under-test). `nono-cli` retains only
thin-wrapper integration tests.

Fork-specific types absent from upstream's `audit.rs` (`RejectStage` enum, `reject_stage` on
`CapabilityDecision`, `chain_head_matches`/`merkle_root_matches` on `AuditVerificationResult`,
`is_valid()` on both result types) are preserved as extensions in `nono::audit` — they extend
upstream types without forking the types themselves.

**Rationale:** maximum convergence reduces reconciliation surface for every future upstream sync.
The audit stack is stable at the library level; CLI-side audit commands remain as thin UX wrappers.

### Decision 2 — Adopt upstream's structured-diagnostics relocation (Cluster B)

`DiagnosticFormatter` (UX, rendering, flag suggestions) moves OUT of the core library and into
`crates/nono-cli/src/diagnostic/formatter.rs`. The core library's single-file `diagnostic.rs`
is replaced by a 6-file module directory (`crates/nono/src/diagnostic/`) containing structured
diagnostic facts, codes (`NonoDiagnosticCode`), observations, records, and session reports —
all data-bearing, none UX.

`NonoError` gains `diagnostic_code()` and `remediation()` methods (implemented for all variants,
including 9 fork-specific variants not present in upstream's `NonoError`). Three new
`pub extern "C"` functions (`nono_last_diagnostic_code`, `nono_last_remediation_json`,
`nono_session_diagnostic_report_to_json`) expose the diagnostic surface to FFI clients.
`NonoDiagnosticCode` is added as a `#[repr(C)]` enum in `bindings/c/src/types.rs`.

`crates/nono-proxy/src/diagnostic.rs` is added net-new, providing `ProxyDiagnostic`,
`ProxyDiagnosticCode`, and `ProxyDiagnosticSeverity` for the proxy layer.

**Rationale:** the library becomes leaner (no UX code); FFI clients (`nono-py`, `nono-ts`) gain
structured diagnostic access without depending on CLI rendering logic; upstream's library boundary
is adopted verbatim, minimizing per-sync friction on the diagnostic surface.

### Decision 3 — Windows denial paths: preserve-and-bridge, NOT converge (D-02)

`crates/nono-cli/src/exec_strategy_windows/{launch.rs,network.rs,mod.rs}` are UNCHANGED — zero
LOC rewritten. Windows denial paths gain `diagnostic_code()` and `remediation()` "for free" as
methods on existing `NonoError` variants; no new bridge code was required beyond the method
implementations in Decision 2.

Full routing of Windows denial output end-to-end through the new core structured-diagnostics
model (having Windows denial render via `DiagnosticFormatter` in `nono-cli/src/diagnostic/`) is
deferred. The regression risk is high (SC#3: no regression in Windows denial output), and the
preserve-and-bridge approach satisfies the Phase 86 acceptance criteria without rewriting proven
Windows paths.

**This is the single deliberate fork carve-out from upstream's boundary line.** It is intentional
design, not drift. See Future Sync Guidance in Consequences.

### Decision 4 — Verification gate must be `--workspace --all-targets` (D-05)

The build/clippy verification gate for Phase 86 (and all subsequent phases touching the FFI
surface) MUST be:

```
cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
```

The `nono-ffi` crate (`bindings/c/`) is built as `cdylib`/`staticlib` — it is NOT reachable
via `--bin nono` scope. `NonoDiagnosticCode`'s `#[repr(C)]` enum adds exhaustive `match` arms;
a `--bin nono`-scoped gate hides `nono-ffi` E0004 exhaustive-match errors (durable v3.0 lesson:
Phase 84 Plan 04 executor faked complete and blamed the gate; caught by code review). The 3 new
`pub extern "C"` diagnostic functions and `NonoDiagnosticCode` must build clean with all arms
covered under `--all-targets`.

---

## Consequences

### Boundary invariant change

The library is NO LONGER a pure policy-free sandbox primitive with no audit or diagnostic logic.

**New invariant:** The `nono` library owns:
- Audit integrity/ledger/attestation/merkle (observability primitives, not security policy)
- Structured diagnostic codes, facts, and session reports (data layer, not UX)

The `nono-cli` binary owns:
- All diagnostic rendering, flag suggestions, and interactive UX (`diagnostic/formatter.rs`)
- Audit command UX, argument parsing, file-path resolution, output rendering
- Windows denial rendering (`exec_strategy_windows/`) — deliberate fork carve-out

**The policy-free-sandbox security principle is preserved:** the library still applies ONLY what
clients explicitly add to `CapabilitySet`. Audit and diagnostic modules are observability
primitives — they observe and report on sandbox operations; they do not define security policy,
grant permissions, or apply sandbox restrictions.

### FFI surface expansion

FFI clients (`nono-py`, `nono-ts`) gain structured diagnostic access via 3 new `extern "C"`
functions. `NonoDiagnosticCode` is a stable ABI type (`#[repr(C)]`) that future FFI consumers
can pattern-match.

### Upstream sync friction reduction

Future upstream syncs: the library/CLI boundary is now aligned with upstream's boundary on the
audit and diagnostics surfaces, reducing per-sync reconciliation friction on these ~2200 LOC.
The Windows carve-out (Decision 3) is the only surface that will continue to accumulate
reconciliation debt.

### Cross-target verification partial

`crates/nono-cli/src/exec_strategy/supervisor_linux.rs` contains `#[cfg(target_os = "linux")]`-
gated code modified during Cluster B cherry-picks. Cross-target clippy verification cannot be
performed from the Windows dev host. Status: PARTIAL, deferred to live CI per
`.planning/templates/cross-target-verify-checklist.md`. All other modified files are
platform-agnostic and verified clean via `cargo clippy --workspace --all-targets --all-features`.

### Future sync guidance

> **The Windows denial path (`exec_strategy_windows/`) is a deliberate fork carve-out — it is
> NOT drift.** Future upstream sync audits must recognize Decision 3 / D-02 as intentional.
> The next upstream sync (Phase 89 proxy hardening or later) should re-evaluate whether full
> Windows convergence (routing Windows denial output end-to-end through the new core
> structured-diagnostics model) is feasible and desirable. The bridge pattern (gain
> `diagnostic_code()`/`remediation()` methods via `NonoError` impl without rewriting the Windows
> denial rendering paths) is stable until that re-evaluation occurs.

---

## Alternatives Considered

| Alternative | Reason rejected |
|-------------|----------------|
| Fork-preserve: keep audit + diagnostics in `nono-cli` | Creates growing divergence from upstream; every future upstream sync accumulates more conflict debt on these surfaces (~2200 LOC). Rejected per Phase 85 D-03 (will-sync locked). |
| Split: move only audit OR only diagnostics | Half-convergence still leaves heavy conflict debt; the two clusters are architecturally coupled (diagnostics use audit records; both touch `NonoError`). Rejected for the same reconciliation-friction reason. |
| Full Windows convergence (route Windows denial through new structured model in Phase 86) | High regression risk (SC#3 requires no regression in Windows denial output); the preserve-and-bridge approach is sufficient for Phase 86 acceptance. Deferred — see Decision 3 and Future Sync Guidance. |
| `--bin nono` verification gate | Hides `nono-ffi` E0004 exhaustive-match errors. Rejected unconditionally — durable v3.0 lesson (Phase 84 close). `--workspace --all-targets` is mandatory. |

---

## References

- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` — Clusters A + B commit inventory, cherry-pick ordering, conflict surface map
- `.planning/phases/86-library-boundary-convergence/86-CONTEXT.md` — D-01 through D-05 locked decisions
- `.planning/phases/86-library-boundary-convergence/86-01-SUMMARY.md` — BND-01 execution record (audit relocation commits + deviations)
- `.planning/phases/86-library-boundary-convergence/86-02-SUMMARY.md` — BND-02 execution record (diagnostics relocation commits + deviations)
- `.planning/templates/cross-target-verify-checklist.md` — cross-target verification protocol
- `CLAUDE.md` § Library vs CLI Boundary — operationalized boundary table (updated by this phase)
- `proj/DESIGN-library.md` — library architecture and workspace layout
