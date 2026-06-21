# Phase 86: Library-Boundary Convergence - Research

**Researched:** 2026-06-19
**Domain:** Rust crate boundary refactor — audit/diagnostics relocation + FFI exhaustive-match closure
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Cherry-pick the 8 commits in locked ledger order. `a5b2a516` first (Theme A anchor), then `aed35bec`/`0b27cfc2`/`e9529312`; then Theme B: `4ad8ba92` → `f867aba2` → `a6aa5995` → `7f319b9e`. Resolve conflicts toward upstream's end-state on shared surfaces.
- **D-01a:** `a6aa5995` must land before any consumer referencing `nono::NonoRemediation`, `ProxyDiagnostic*`, or FFI diagnostic-code fns. `4ad8ba92` must land first in Theme B (migrates existing diagnostic surface out of core — prerequisite for the new module layout).
- **D-02:** Preserve-and-bridge Windows diagnostic paths (`crates/nono-cli/src/exec_strategy_windows/{launch,network,mod}.rs`). Populate `diagnostic_code`/`remediation` at the surface only. Do NOT rewrite Windows denial-rendering logic.
- **D-03:** Match upstream's CLI/lib boundary line exactly. Single carve-out: Windows denial paths stay CLI-side, bridged (D-02).
- **D-04:** Move unit tests for relocated audit logic into `crates/nono/src/audit.rs` alongside the moved code. CLI keeps only thin-wrapper / integration tests.
- **D-05:** Gate MUST be `--workspace --all-targets` (NOT `--bin nono`). The `--bin nono` scope hides `nono-ffi` E0004 exhaustive-match breaks.

### Claude's Discretion

- Exact conflict-resolution tactics per cherry-pick hunk.
- Whether to squash or retain the 8 cherry-picked commits (provenance goal, D-01).
- ADR file location/format (follow existing ADR convention; see `proj/ADR-74-privilege-model.md` as the house style reference).
- Precise shape of the Windows bridge field-mapping (research maps these file-by-file).

### Deferred Ideas (OUT OF SCOPE)

- Full Windows convergence (routing denial output end-to-end through the new core structured-diagnostics model) — D-02 is bridge-only.
- Theme C/D–M/F absorption — Phases 87/88/89.
- Crate-version leapfrog to >= 0.65.0 — release-time.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BND-01 | Audit/attestation/ledger logic relocated into `crates/nono/src/audit.rs`; CLI reduced to thin wrappers; all audit behavior tested | Cherry-pick Theme A commits A1–A4; conflict surface mapped below in §Conflict Map; test relocation plan in §Validation Architecture |
| BND-02 | Structured-diagnostics model in `crates/nono/src/diagnostic/*`; `NonoError::{diagnostic_code, remediation}`; FFI (`bindings/c/src/diagnostic.rs`, `NonoDiagnosticCode`, 3 extern-C fns); reconciled with Windows paths and proxy `ProxyDiagnostic` | Cherry-pick Theme B commits B1–B4; module-dir conflict map in §Theme B Module-Dir Conflict; FFI closure in §FFI Exhaustive-Match Closure; Windows bridge in §Windows Bridge |
| BND-03 | `CLAUDE.md` § Library vs CLI boundary updated; ADR records boundary-convergence decision | ADR convention researched in §ADR Convention; what changes in CLAUDE.md identified in §Standard Stack |
</phase_requirements>

---

## Summary

Phase 86 ports 8 upstream commits from `always-further/nono` (`v0.62.0..v0.64.0`) into the fork, performing the heaviest structural change in the repository's history: moving ~1773 LOC of audit/attestation/ledger logic and a full structured-diagnostics module from `nono-cli` into the core `nono` crate. All 8 upstream commit objects are confirmed present in the local git object store (verified by `git cat-file -t` for all SHAs). Cherry-pick is directly executable without network access.

The fork starting state is well-understood from the DIVERGENCE-LEDGER and verified against the live tree: Theme A creates `crates/nono/src/audit.rs` (net-new to the fork — no file exists today); Theme B replaces the fork's single-file `crates/nono/src/diagnostic.rs` (3,872 lines, `DiagnosticFormatter` and all UX logic) with a 6-file module directory, strips the UX out of core, and simultaneously adds three net-new files (`bindings/c/src/diagnostic.rs`, `crates/nono-proxy/src/diagnostic.rs`, `crates/nono-cli/src/diagnostic/`) that do not exist in the fork today.

The critical non-negotiable for this phase is the FFI exhaustive-match closure: `a6aa5995` adds a `NonoDiagnosticCode` repr-C enum to `bindings/c/src/types.rs` and three new `pub extern "C"` fns in `bindings/c/src/diagnostic.rs`. The existing `map_error` match in `bindings/c/src/lib.rs` is currently fully exhaustive over all `NonoError` variants (confirmed: 28 arms covering every variant including Phase 84's `TelemetryUnavailable`/`TelemetryConfigInvalid`). The commit `a6aa5995` adds `NonoError::{diagnostic_code, remediation}` to `error.rs`; these become new variants that will break the exhaustive match unless explicitly mapped. The verification gate MUST be `--workspace --all-targets` (D-05 — confirmed critical from v3.0 lesson).

**Primary recommendation:** Execute cherry-picks in strict ledger order. Treat every cherry-pick that touches `crates/nono/src/diagnostic.rs` or `bindings/c/src/` as a potential exhaustive-match break requiring `--workspace --all-targets` verification before moving to the next commit.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Audit recording (AuditRecorder, merkle, ledger, attestation) | Core library (`nono`) | CLI (thin wrapper UX) | Upstream A moves these — D-03 adopts upstream's line |
| Diagnostic facts (DenialRecord, DenialReason, IpcDenialRecord, SandboxViolation) | Core library (`nono`) | — | Already in fork's `nono/src/diagnostic.rs`; remain in core after B |
| Diagnostic structured model (codes.rs, report.rs, NonoRemediation, diagnostic_code/remediation) | Core library (`nono`) | CLI (formatter, flag suggestions) | Upstream B's `diagnostic/` module dir arrives in core |
| Diagnostic UX / rendering (DiagnosticFormatter, analyze_error_output, footer formatting) | CLI (`nono-cli`) | — | `4ad8ba92` moves DiagnosticFormatter OUT of core into CLI — this is the correct upstream boundary |
| FFI diagnostic surface (NonoDiagnosticCode, 3 extern-C fns) | FFI (`bindings/c`) | Core library (types it wraps) | `a6aa5995` adds `bindings/c/src/diagnostic.rs` wholesale |
| Proxy diagnostic surface (ProxyDiagnostic, ProxyDiagnosticCode) | Proxy (`nono-proxy`) | — | `a6aa5995` adds `crates/nono-proxy/src/diagnostic.rs` wholesale |
| Windows denial rendering | CLI (`exec_strategy_windows/`) | Core library (bridge field population only) | D-02: keep Windows rendering CLI-side; bridge populates `diagnostic_code`/`remediation` at surface |

---

## Cherry-Pick Feasibility

**All 8 upstream SHAs are present in the local object store.** [VERIFIED: git cat-file -t]

```
a5b2a516  commit  (present)
aed35bec  commit  (present)
0b27cfc2  commit  (present)
e9529312  commit  (present)
4ad8ba92  commit  (present)
f867aba2  commit  (present)
a6aa5995  commit  (present)
7f319b9e  commit  (present)
```

The upstream remote is configured as `upstream https://github.com/always-further/nono.git`. [VERIFIED: git remote -v] No network fetch is required before cherry-picking.

**Fallback if cherry-pick fails catastrophically:** `git show <SHA>` produces the full diff for manual port. The DIVERGENCE-LEDGER already maps every file per commit; the upstream diff is the answer key for any hunk the cherry-pick mechanism cannot auto-apply.

---

## Per-Commit Conflict Map

### Theme A: Audit → Core (4 commits)

| SHA | Subject | Files touched in FORK tree | Conflict surface | Resolution target |
|-----|---------|---------------------------|------------------|-------------------|
| `a5b2a516` | refactor(audit): move audit integrity logic to nono crate | `crates/nono-cli/src/audit_integrity.rs` (shrinks 396 LOC), `crates/nono/src/audit.rs` (CREATE — does not exist in fork), `crates/nono/src/lib.rs` (add `pub mod audit`) | **MEDIUM.** `audit_integrity.rs` exists in fork with full body; upstream shrinks it to thin wrapper. `lib.rs` gains `pub mod audit` line. `audit.rs` is a CREATE — no content conflict, but fork must not have diverged content. | Upstream: shrink `audit_integrity.rs` to thin wrappers + create `audit.rs`. Verify `lib.rs` insertion point does not conflict with fork-added modules (`machine_policy`, `supervisor`, `trust`, `agent`, `scrub`, `net_filter`, `path`) — all are fork-additions that upstream does not have; the `pub mod audit` insertion is additive and should not conflict. |
| `aed35bec` | refactor(audit-ledger): move audit ledger logic to library crate | `crates/nono-cli/src/audit_ledger.rs` (shrinks 294 LOC from 507+), `crates/nono/src/audit.rs` (extends, 507+) | **HIGH.** The fork does NOT have `crates/nono-cli/src/audit_ledger.rs` — it has `audit_integrity.rs`. [VERIFIED: `ls` on fork] Upstream's `a5b2a516` renamed/split `audit_integrity.rs` into `audit_ledger.rs` (one of the thin wrappers). The cherry-pick will fail with a path-not-found conflict because it tries to patch `audit_ledger.rs` which the fork's `a5b2a516` cherry-pick will have created (since that commit introduces the thin-wrapper shape). Resolution: after `a5b2a516` cherry-pick resolves, `audit_ledger.rs` should exist as the thin-wrapper stub for `audit_ledger` logic. If the cherry-pick mechanism cannot find it, resolve by creating the file from upstream's end-state. |
| `0b27cfc2` | refactor(audit): move attestation logic to core library (#1148) | `crates/nono-cli/src/audit_attestation.rs` (shrinks 273 LOC from 531+), `crates/nono/src/audit.rs` (extends, 531+) | **MEDIUM.** `audit_attestation.rs` exists in fork [VERIFIED: `ls`]. Upstream shrinks it to thin wrapper. Content conflict expected because the fork's `audit_attestation.rs` diverges from upstream's (fork has v3.0 fork-additions). Resolve toward upstream's thin-wrapper end-state; fork attestation tests move to `audit.rs`. |
| `e9529312` | fix(audit): address ledger review and clippy | `crates/nono/src/audit.rs` (21+/25- clippy fixes) | **LOW.** Pure clippy/lint fix on `audit.rs`; expect clean apply if `audit.rs` has upstream's shape after the prior 3 commits. May conflict if resolution of `aed35bec` left divergent formatting. |

**Theme A cross-target note:** None of these commits touch `#[cfg(target_os = ...)]` blocks. Standard `cargo clippy --workspace --all-targets` is sufficient — no cross-toolchain requirement for Theme A specifically.

### Theme B: Diagnostics → Core + FFI (4 commits, LOCKED ORDER: 4ad8ba92 → f867aba2 → a6aa5995 → 7f319b9e)

| SHA | Subject | Files touched in FORK tree | Conflict surface | Resolution target |
|-----|---------|---------------------------|------------------|-------------------|
| `4ad8ba92` | refactor(diagnostic): move diagnostic UX out of core nono crate (#1155) | `crates/nono/src/diagnostic.rs` (strips to struct facts only, 3944- LOC removed), `crates/nono/src/lib.rs` (update re-exports), `crates/nono-cli/src/diagnostic/formatter.rs` (CREATE), `crates/nono-cli/src/diagnostic/mod.rs` (CREATE), `crates/nono-cli/src/exec_strategy.rs` (use new CLI diagnostic path), `crates/nono-cli/src/main.rs` (import), `crates/nono-cli/src/profile_save_runtime.rs` (import), `crates/nono-cli/src/query_ext.rs` (import) | **CRITICAL — heaviest conflict.** The fork's `crates/nono/src/diagnostic.rs` is 3,872 lines [VERIFIED: Read tool]. It contains `DiagnosticFormatter`, `analyze_error_output`, `ErrorObservation`, `PolicyExplanation`, `ObservedPathHint`, `ErrorVerdict`, `CommandContext`, plus structural facts (`DenialRecord`, `DenialReason`, `IpcDenialRecord`, `SandboxViolation`). Upstream `4ad8ba92` strips `diagnostic.rs` to ONLY the structural facts and moves everything else into the CLI's `diagnostic/formatter.rs` module. The fork has `crates/nono-cli/src/diagnostic_formatter.rs` (flat file) NOT a `diagnostic/` subdirectory. Fork has NO `crates/nono-cli/src/diagnostic/` directory. [VERIFIED: `ls`] The cherry-pick will conflict on `diagnostic.rs` (massive removal) and will fail to find the `diagnostic/formatter.rs` CREATE path. `lib.rs` will also conflict because upstream's post-`4ad8ba92` re-exports only the structural facts. | Resolution: Apply upstream's stripped `diagnostic.rs` (keep only: `DenialReason`, `DenialRecord`, `IpcDenialRecord`, `SandboxViolation`, `seatbelt_operation_to_access` — these are the only types that remain in core after B). Create `crates/nono-cli/src/diagnostic/mod.rs` and `diagnostic/formatter.rs` to receive the moved UX code. The fork's existing `diagnostic_formatter.rs` (flat) becomes the body of `diagnostic/formatter.rs`. Update `lib.rs` re-exports to match upstream. |
| `f867aba2` | fix: report the actual blocked operation instead of the readable target path (#1150) | `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (fix), `crates/nono-cli/src/sandbox_log.rs` (fix), `crates/nono/src/diagnostic.rs` (3 files, 144+/24-) | **MEDIUM.** `sandbox_log.rs` exists in fork [VERIFIED: `ls`]. The patch to `crates/nono/src/diagnostic.rs` is a small behavioral fix (report blocked operation, not readable path). After `4ad8ba92` strips `diagnostic.rs`, this commit touches the stripped version. Conflict: after `4ad8ba92`, the fork's `diagnostic.rs` may differ from upstream's stripped shape. Resolve toward upstream's end-state (the fix to records.rs or the structural facts). `supervisor_linux.rs` is Linux cfg-gated — cross-target clippy flag applies. |
| `a6aa5995` | feat(diagnostics): expose structured diagnostics for library and FFI clients (#1171) | `crates/nono/src/diagnostic/` (CREATE 6 files: `codes.rs`, `observation.rs`, `records.rs`, `report.rs`, `detail.rs`, `mod.rs`), `crates/nono/src/error.rs` (add `diagnostic_code`/`remediation` methods), `crates/nono/src/lib.rs` (add `pub mod diagnostic` + `pub use diagnostic::*`), `bindings/c/src/diagnostic.rs` (CREATE — 3 extern-C fns), `bindings/c/src/lib.rs` (wire diagnostic module), `bindings/c/src/types.rs` (add `NonoDiagnosticCode` repr-C enum), `bindings/c/Cargo.toml`, `bindings/c/include/nono.h`, `crates/nono-cli/src/diagnostic/formatter.rs` (add `suggested_flag_for_remediation`), `crates/nono-proxy/src/diagnostic.rs` (CREATE), `crates/nono-proxy/src/lib.rs` (add re-export), various CLI consumers | **CRITICAL — FFI exhaustive-match break.** This is the commit that adds `NonoDiagnosticCode` to `types.rs` and `NonoError::{diagnostic_code, remediation}` to `error.rs`. The fork's `bindings/c/src/lib.rs::map_error` is currently exhaustive over all variants [VERIFIED: full match read]; adding new `NonoError` methods (or a new `NonoErrorCode` variant for diagnostic codes) will cause E0004 non-exhaustive match. **The 3 new `pub extern "C"` fns** (`nono_last_diagnostic_code`, `nono_last_remediation_json`, `nono_session_diagnostic_report_to_json`) are net-new to the fork — no pre-existing code conflicts. `crates/nono/src/diagnostic.rs` (single file) is REPLACED by the module dir `crates/nono/src/diagnostic/` — the single file must be deleted and the directory created. The fork's `lib.rs` re-exports `pub use diagnostic::{ CommandContext, DenialReason, DenialRecord, DiagnosticFormatter, DiagnosticMode, IpcDenialRecord, SandboxViolation }` — after B, `CommandContext`/`DiagnosticFormatter`/`DiagnosticMode` are NO LONGER in core; the re-export list must be stripped. | Resolution: (1) Delete `crates/nono/src/diagnostic.rs` and create the 6-file module directory. (2) Add `NonoDiagnosticCode` to `types.rs`. (3) Add `diagnostic_code`/`remediation` to `error.rs`. (4) Add arms for any new `NonoError` methods in `map_error`. (5) Add `bindings/c/src/diagnostic.rs` (3 fns) wholesale — it is net-new. (6) Create `crates/nono-proxy/src/diagnostic.rs` wholesale — net-new. (7) Wire `bindings/c/src/lib.rs` to `pub use diagnostic::*`. (8) Strip CLI-only types from `crates/nono/src/lib.rs` re-exports. |
| `7f319b9e` | fix(diagnostic): replace deprecated nono learn with nono run (#1170) | `crates/nono-cli/src/diagnostic/formatter.rs` (8+/6- string replacement) | **LOW.** Pure string replacement in the CLI formatter. After `4ad8ba92` creates `diagnostic/formatter.rs`, this commit patches it. The fork's `diagnostic_formatter.rs` (flat) has the same `nono learn` string [VERIFIED: confirmed in `format_follow_up_guidance` fn which outputs `"nono learn --"`]. If the fork resolves `4ad8ba92` by creating `diagnostic/formatter.rs`, this will apply cleanly. If the flat-file path is preserved instead, a manual string patch is needed. |

---

## Theme B Module-Dir Conflict Detail

**Fork starting state:** `crates/nono/src/diagnostic.rs` is a single 3,872-line file [VERIFIED: Read tool line count] containing:
- Structural facts (stay in core after B): `DenialReason`, `DenialRecord`, `IpcDenialRecord`, `SandboxViolation`
- UX logic (move to CLI after `4ad8ba92`): `DiagnosticFormatter`, `DiagnosticMode`, `analyze_error_output`, `ErrorObservation`, `PolicyExplanation`, `ObservedPathHint`, `ErrorVerdict`, `CommandContext`, all `format_*` methods, `render_diagnostic_block`
- Note: the fork has a fork-local `try_canonicalize` function at line 26–43 (comment: "Fork-local copy of upstream's `crate::path::try_canonicalize`"). Upstream's `a6aa5995` introduces `crates/nono/src/path.rs` (or the diagnostic module uses the lib's path). This fork-local copy must be reconciled — either removed (if upstream's module dir no longer needs it) or migrated.

**Fork re-exports in `crates/nono/src/lib.rs` (lines 71–74):**
```rust
pub use diagnostic::{
    CommandContext, DenialReason, DenialRecord, DiagnosticFormatter, DiagnosticMode,
    IpcDenialRecord, SandboxViolation,
};
```
After B (`4ad8ba92` + `a6aa5995`): `CommandContext`, `DiagnosticFormatter`, `DiagnosticMode` are REMOVED from core; only structural facts remain re-exported; the new module's `pub use diagnostic::*` replaces this explicit list. [ASSUMED — inferred from DIVERGENCE-LEDGER Cluster B re-export findings; requires `git show 4ad8ba92 -- crates/nono/src/lib.rs` to confirm exact diff]

**Fork's CLI diagnostic file:** `crates/nono-cli/src/diagnostic_formatter.rs` (flat file, no subdirectory). [VERIFIED: `ls`] Upstream's `4ad8ba92` creates `crates/nono-cli/src/diagnostic/formatter.rs` and `diagnostic/mod.rs`. The planner must include a step to:
1. Create the `diagnostic/` subdirectory under `crates/nono-cli/src/`
2. Move/rename `diagnostic_formatter.rs` → `diagnostic/formatter.rs`
3. Create `diagnostic/mod.rs` that re-exports the formatter
4. Update `crates/nono-cli/src/main.rs` and other importers to use the new module path

**Upstream module dir structure (after `a6aa5995`):**
```
crates/nono/src/diagnostic/
├── codes.rs        (+188 LOC — NonoDiagnostic, suggested_flag_for_remediation)
├── detail.rs       (additional diagnostic detail types)
├── mod.rs          (pub use codes::*, pub use detail::*, pub use observation::*, pub use records::*, pub use report::*)
├── observation.rs  (+199 LOC — into_diagnostics, follow_up_diagnostics, diagnostic_* fns)
├── records.rs      (+168 LOC — DenialRecord::new, seatbelt_operation_to_access)
└── report.rs       (+446 LOC — SessionDiagnosticReport)
```
[CITED: DIVERGENCE-LEDGER Cluster B + SEED-006 Theme B function inventory]

---

## FFI Exhaustive-Match Closure

### Current state (verified)

`bindings/c/src/lib.rs::map_error` is currently exhaustive over all `NonoError` variants. [VERIFIED: full match block read, lines 72–171] It covers 28 variant patterns including all Phase 84 additions (`TelemetryUnavailable`, `TelemetryConfigInvalid`, `PolicyLoadFailed`). The match uses `#[cfg(target_os = "linux")]` for `Landlock`/`LandlockPath` arms.

### What `a6aa5995` adds to `error.rs`

Upstream adds `NonoError::diagnostic_code() -> Option<NonoDiagnosticCode>` and `NonoError::remediation() -> Option<NonoRemediation>` as **methods** on the existing enum (not new variants). [CITED: DIVERGENCE-LEDGER Cluster B finding: "a6aa5995 adds `pub mod diagnostic` and `pub use diagnostic::*` to core lib.rs; `error.rs` (+137) — `NonoError::{diagnostic_code, remediation}`"]

**Critical clarification:** `diagnostic_code` and `remediation` are methods, not new enum variants. The `map_error` exhaustive match over `NonoError` variants is NOT broken by adding methods. However, `a6aa5995` also adds `NonoDiagnosticCode` to `types.rs` as a repr-C enum with its own variant list — any `match NonoDiagnosticCode` elsewhere will need coverage. [ASSUMED — need to confirm `a6aa5995` does not add new `NonoError` variants by reading the actual diff; the DIVERGENCE-LEDGER finding is consistent with methods-only addition]

**The three new `pub extern "C"` fns** (in net-new `bindings/c/src/diagnostic.rs`):

```rust
// Source: DIVERGENCE-LEDGER Cluster B Pitfall 3 closure
pub extern "C" fn nono_last_diagnostic_code() -> NonoDiagnosticCode;
pub extern "C" fn nono_last_remediation_json() -> *mut c_char;
pub unsafe extern "C" fn nono_session_diagnostic_report_to_json(...);
```
[CITED: DIVERGENCE-LEDGER § Cluster B Cross-cluster re-export check]

These functions read from the thread-local error store (`LAST_ERROR`) in `bindings/c/src/lib.rs` and call `diagnostic_code()`/`remediation()` on the stored `NonoError`. The thread-local store pattern is well-established in the FFI layer [VERIFIED: `bindings/c/src/lib.rs` lines 41–68].

**Wire-up pattern:** `bindings/c/src/lib.rs` must:
1. Add `pub mod diagnostic;` and `pub use diagnostic::*;`
2. The `set_last_error` / `map_error` functions are NOT changed (methods added to `NonoError` don't break exhaustive match over variants)

**`NonoDiagnosticCode` repr-C enum:** Added to `bindings/c/src/types.rs`. [CITED: DIVERGENCE-LEDGER Cluster B finding] The existing `NonoErrorCode` enum in `types.rs` is separate and unchanged. The planner must verify that no `match NonoDiagnosticCode` expression is auto-generated by cbindgen in a way that needs update. [ASSUMED — cbindgen generates the `.h` header, not Rust match arms; the `nono.h` update is in `a6aa5995`'s file list at `bindings/c/include/nono.h`]

### Verification gate (D-05)

After `a6aa5995` cherry-pick resolves, run:
```bash
cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
```
NOT `cargo build --bin nono` — that scope excludes `nono-ffi` entirely. [VERIFIED: durable lesson from v3.0, `project_v30_opened` memory + CONTEXT.md D-05]

---

## Windows Bridge Field-Mapping (D-02)

### Preserve-and-bridge targets

`crates/nono-cli/src/exec_strategy_windows/{launch.rs, network.rs, mod.rs}` — all Windows-only diagnostic/denial-rendering paths. [VERIFIED: `ls exec_strategy_windows/`]

### Existing Windows denial output (no new enum types, string-based errors)

The fork's Windows paths produce denial output via `NonoError` variants with embedded diagnostic strings:

| File | Existing denial mechanism | Bridge: populate `diagnostic_code` | Bridge: populate `remediation` |
|------|--------------------------|-------------------------------------|-------------------------------|
| `exec_strategy_windows/network.rs` | `NonoError::SandboxInit(format!("Failed to apply Windows blocked-network rule..."))` via `classify_netsh_firewall_failure` [VERIFIED: lines 53–68] | After B: call `e.diagnostic_code()` → maps to `Some(NonoDiagnosticCode::NetworkBlocked)` or similar code; no code rewrite needed — the method is on the error | `e.remediation()` returns a `NonoRemediation` struct if the error type has a mapping; otherwise `None` → no regression |
| `exec_strategy_windows/mod.rs` | `NonoError::SandboxInit(...)`, `NonoError::Setup(...)`, `NonoError::UnsupportedPlatform(...)` for various Windows denial paths [VERIFIED: grep lines 107, 384, 710–830] | Same as above — methods are on the variant, not derived from rendering logic | Same |
| `exec_strategy_windows/launch.rs` | `OwnedJobSD`, DACL/security-descriptor operations returning `NonoError::SandboxInit` indirectly via callers | Same — `diagnostic_code()` method will return `None` for `SandboxInit` unless upstream maps it | `None` is acceptable — bridge contract is "populate at surface, not None-fail" |

**Bridge contract (D-02):** The preserve-and-bridge pattern is: after the cherry-pick introduces `NonoError::diagnostic_code()` and `NonoError::remediation()` as methods, Windows denial paths gain these methods "for free" because they return `NonoError` variants that already exist. No code in `exec_strategy_windows/` needs to change to satisfy the bridge at the NonoError level. If the new FFI fns (`nono_last_diagnostic_code`, `nono_last_remediation_json`) read the thread-local `LAST_ERROR` and call these methods, Windows denial paths already populate `LAST_ERROR` via the existing `map_error` path. This is the bridge — zero render logic rewrite required.

**What the planner must NOT do:** do not add any direct imports of `nono::diagnostic::*` types into `exec_strategy_windows/` files. The bridge is via the `NonoError` method surface, not by the Windows files constructing `NonoDiagnostic` structs directly.

---

## Proxy `ProxyDiagnostic` Surface

`crates/nono-proxy/src/diagnostic.rs` does NOT exist in the fork today. [VERIFIED: `ls crates/nono-proxy/src/`] The fork proxy surface is: `audit.rs`, `config.rs`, `connect.rs`, `credential.rs`, `error.rs`, `external.rs`, `filter.rs`, `lib.rs`, `oauth2.rs`, `reverse.rs`, `route.rs`, `server.rs`, `token.rs`.

Upstream `a6aa5995` adds `crates/nono-proxy/src/diagnostic.rs` (+99 LOC) with:
- `ProxyDiagnostic::{ warning, with_credential_ref, with_hint }`
- `ProxyDiagnosticCode::as_str`
- `ProxyDiagnosticSeverity` enum
[CITED: SEED-006 Theme B function inventory]

And updates `crates/nono-proxy/src/lib.rs` to add:
```rust
pub use diagnostic::{ProxyDiagnostic, ProxyDiagnosticCode, ProxyDiagnosticSeverity};
```
[CITED: DIVERGENCE-LEDGER Cluster B re-export finding]

**Conflict surface:** The fork's `crates/nono-proxy/src/lib.rs` does not re-export `diagnostic::*` today. The cherry-pick adds this re-export as a net-new line. Conflict risk: LOW (additive). The file `crates/nono-proxy/src/diagnostic.rs` is a CREATE — no conflict.

**SC#3 verification:** The proxy `ProxyDiagnostic` surface is consumed by the CLI's `output.rs::print_proxy_diagnostics` (upstream). The fork must verify `print_proxy_diagnostics` in `output.rs` either exists (if already in fork) or is added by `a6aa5995`. [ASSUMED — need to check if `output.rs` already has this fn or if it arrives with a6aa5995]

---

## Standard Stack

No new dependencies are introduced by the 8 commits. This is a pure code-relocation refactor. [CITED: DIVERGENCE-LEDGER Cluster A + B — no new dep entries in commit file lists, only `Cargo.lock` in `a6aa5995` and `bindings/c/Cargo.toml`]

The `bindings/c/Cargo.toml` change in `a6aa5995` [VERIFIED: file list] is likely a version bump or feature flag for the new diagnostic types. The planner must inspect this file change specifically to confirm it does not add a new `[dependencies]` entry.

**Existing stack (unchanged):**
- `thiserror` — error types (`error.rs`)
- `serde`/`serde_json` — used in audit serialization and diagnostic report serialization
- `sha2` — audit merkle/hash (already in `nono` crate Cargo.toml [VERIFIED])
- `cbindgen` — header generation (build dep in `bindings/c`) — `nono.h` is updated by `a6aa5995`

## Package Legitimacy Audit

No new packages are introduced by Phase 86 commits. This section is not applicable.

---

## Architecture Patterns

### Thin-wrapper pattern (post-Theme A)

After Theme A, `nono-cli`'s audit files become thin wrappers that:
1. Accept CLI arguments (paths, output formats)
2. Call `nono::audit::*` for business logic
3. Format and print results to stdout/stderr

Example post-Theme A shape (from upstream commit message): `audit_integrity.rs` shrinks to +4/-396, `audit_attestation.rs` to +36/-273, `audit_ledger.rs` to +40/-286. The CLI files become ~50–120 line wrappers.

### Method-on-error pattern (post-Theme B)

After `a6aa5995`, `NonoError` gains two methods:
```rust
// Source: DIVERGENCE-LEDGER Cluster B re-export finding + SEED-006 Theme B
impl NonoError {
    pub fn diagnostic_code(&self) -> Option<NonoDiagnosticCode> { ... }
    pub fn remediation(&self) -> Option<NonoRemediation> { ... }
}
```
These are intrinsic methods, not new variants. Every existing `match e` over `NonoError` variants remains valid. The new `NonoDiagnosticCode` enum lives in the diagnostic module, not in `error.rs`. [ASSUMED — consistent with DIVERGENCE-LEDGER finding; executor should confirm against `git show a6aa5995 -- crates/nono/src/error.rs`]

### Module-dir migration pattern

`crates/nono/src/diagnostic.rs` (single file) → `crates/nono/src/diagnostic/` (directory with `mod.rs` + 5 submodules). Rust treats both equivalently at the import level — `use nono::diagnostic::*` works for both. The migration requires:
1. Delete `diagnostic.rs`
2. Create `diagnostic/mod.rs` with same module-level doc comment
3. Create `diagnostic/{codes,detail,observation,records,report}.rs`
4. The `pub mod diagnostic;` line in `lib.rs` stays unchanged — Rust auto-detects dir vs file.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Conflict resolution answer key | Manual diff inspection from scratch | `git show <SHA> -- <file>` for the upstream end-state | The cherry-pick conflict marker shows the fork's version; upstream's end-state is the resolution target. Use `git show` to get it. |
| Exhaustive-match verification | Manual arm enumeration | `cargo clippy --workspace --all-targets -- -D warnings` | Compiler + clippy enforce exhaustiveness with E0004; never try to enumerate by hand |
| Diagnostic code enum coverage | Enumerate `NonoDiagnosticCode` variants manually | Add `NonoDiagnosticCode::Unknown` as a catch-all in `map_error` for future-proofing ONLY if upstream does so | Mirrors upstream's FFI stability contract |
| cbindgen header update | Edit `nono.h` by hand | Let cbindgen regenerate via `build.rs` | `a6aa5995` updates `nono.h` in the commit; the build system regenerates it. Hand-editing causes drift. |

---

## Common Pitfalls

### Pitfall 1: `--bin nono` Gate Scope (D-05 non-negotiable)
**What goes wrong:** Running `cargo build --bin nono` or `cargo clippy --bin nono` after `a6aa5995` shows a clean build but hides `nono-ffi` E0004 exhaustive-match breaks. The v3.0 milestone shipped with exactly this failure (`project_v30_opened` memory; `84-04 executor faked complete + blamed gate FAIL on 'stale MSI'`).
**Why it happens:** `--bin nono` scopes to `nono-cli` only; `nono-ffi` is a `cdylib`/`staticlib` not reachable via that scope.
**How to avoid:** Always use `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`.
**Warning signs:** Build passes on `--bin nono` but CI fails — this is the exact failure mode.

### Pitfall 2: Single-File → Module-Dir Re-export Drift
**What goes wrong:** After `4ad8ba92`, `crates/nono/src/lib.rs` still has the old re-export list `pub use diagnostic::{CommandContext, DenialReason, DenialRecord, DiagnosticFormatter, DiagnosticMode, IpcDenialRecord, SandboxViolation}`. Types moved to CLI (`CommandContext`, `DiagnosticFormatter`, `DiagnosticMode`) no longer exist in `nono::diagnostic` — compile error.
**Why it happens:** The cherry-pick resolution of `4ad8ba92` on `lib.rs` may not cleanly remove the old re-exports if the fork's `lib.rs` has diverged.
**How to avoid:** After resolving `4ad8ba92`, verify `crates/nono/src/lib.rs` re-export list: only `DenialReason`, `DenialRecord`, `IpcDenialRecord`, `SandboxViolation` should remain in the diagnostic re-export. Remove the others.
**Warning signs:** `error[E0432]: unresolved import 'nono::diagnostic::DiagnosticFormatter'` in `nono-cli`.

### Pitfall 3: `audit_ledger.rs` Does Not Exist Before `aed35bec`
**What goes wrong:** `aed35bec` patches `crates/nono-cli/src/audit_ledger.rs`. The fork today has `audit_integrity.rs` (not `audit_ledger.rs`). [VERIFIED] If `a5b2a516` cherry-pick creates `audit_ledger.rs` (as part of splitting the thin wrappers), `aed35bec` will apply. If `a5b2a516` does not create `audit_ledger.rs` (because the fork's `audit_integrity.rs` is the target), `aed35bec` will conflict.
**Why it happens:** Upstream introduced `audit_ledger.rs` as a rename/split of `audit_integrity.rs` in `a5b2a516`. The fork still has the pre-split name.
**How to avoid:** After resolving `a5b2a516`, verify both `audit_integrity.rs` and `audit_ledger.rs` exist in `crates/nono-cli/src/` with thin-wrapper content. If only `audit_integrity.rs` exists, either the cherry-pick did not create `audit_ledger.rs` or the rename was incomplete. Inspect `git show a5b2a516 -- crates/nono-cli/src/` to confirm the file shape upstream expects.

### Pitfall 4: `try_canonicalize` Fork-Local Copy Collision
**What goes wrong:** The fork's `crates/nono/src/diagnostic.rs` has a fork-local copy of `try_canonicalize` at lines 26–43 (comment: "Fork-local copy of upstream's `crate::path::try_canonicalize`"). [VERIFIED] After `4ad8ba92` strips `diagnostic.rs` to only structural facts, and after `a6aa5995` creates the module dir, this function must NOT remain in the stripped `diagnostic.rs` (the module dir `records.rs` handles it differently). If left behind, it will conflict with `crates/nono/src/path.rs::try_canonicalize` which the fork already exports (`pub use path::try_canonicalize` in `lib.rs`). [VERIFIED: `lib.rs` line 86]
**Why it happens:** Fork-local helper was inlined because `crates/nono/src/path.rs` was not yet in the fork at the time of the diagnostic.rs creation. The path module now exists.
**How to avoid:** When resolving `4ad8ba92`, delete the `try_canonicalize` fn from `diagnostic.rs` — the stripped version should use `crate::path::try_canonicalize` or simply not need it (structural-facts-only files do not canonicalize paths at construction time).

### Pitfall 5: Cross-Target Clippy for `f867aba2`
**What goes wrong:** `f867aba2` touches `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (Linux cfg-gated). Windows-host `cargo clippy` does not compile this file.
**Why it happens:** `#[cfg(target_os = "linux")]` blocks are excluded from Windows compilation.
**How to avoid:** Per CLAUDE.md MUST/NEVER rule: any commit touching `exec_strategy/supervisor_linux.rs` requires `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin`. If cross-toolchain unavailable on the dev host, mark BND-02 PARTIAL and defer to live CI per `.planning/templates/cross-target-verify-checklist.md`.

### Pitfall 6: DCO Sign-off on Cherry-Picked Commits
**What goes wrong:** Cherry-picked upstream commits carry Luke Hinds's sign-off. The fork requires `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` on every commit.
**Why it happens:** `git cherry-pick` preserves the original commit author/sign-off.
**How to avoid:** Use `git cherry-pick -x <SHA>` (adds "(cherry picked from commit ...)" tracer) then `git commit --amend -s --no-edit` to append the fork's DCO sign-off. Alternatively, configure a `prepare-commit-msg` hook. Provenance (D-01) is served by the `(cherry picked from commit ...)` tracer; the DCO sign-off requirement is separate.

---

## Runtime State Inventory

Not applicable. Phase 86 is a code-relocation refactor. No stored data, live service config, OS-registered state, secrets/env vars, or build artifacts reference the audit or diagnostic module names. The only "runtime state" consideration is: if any deployed `nono.h` header consumers exist, the `nono.h` update in `a6aa5995` changes the public C header. This is handled by the build system (cbindgen regenerates).

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | None — uses Makefile targets |
| Quick run command | `make test-lib && make test` |
| Full suite command | `make ci` (clippy + fmt + tests, `--workspace --all-targets`) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Notes |
|--------|----------|-----------|-------------------|-------|
| BND-01 | AuditRecorder lifecycle (new, record, finalize) | unit | `cargo test -p nono audit` | Tests move from CLI to `crates/nono/src/audit.rs` per D-04 |
| BND-01 | Merkle/inclusion-proof verify | unit | `cargo test -p nono merkle` | Moves with code to core |
| BND-01 | Ledger append + verify | unit | `cargo test -p nono ledger` | Moves with code to core |
| BND-01 | Attestation sign/verify | unit | `cargo test -p nono attest` | Moves with code to core |
| BND-02 | `NonoError::diagnostic_code()` + `remediation()` return expected values | unit | `cargo test -p nono diagnostic_code` | New tests in `crates/nono/src/error.rs` or `diagnostic/codes.rs` |
| BND-02 | `nono_last_diagnostic_code` / `nono_last_remediation_json` return non-null on error | unit | `cargo test -p nono-ffi` | In `bindings/c/src/diagnostic.rs` |
| BND-02 | `NonoDiagnosticCode` enum coverage (match exhaustive) | compile-time | `cargo clippy --workspace --all-targets` | E0004 is the gate |
| BND-02 | Windows denial paths still produce diagnostic output (no regression) | smoke (manual) | Local Windows `nono run --block-network <cmd>` | Cannot be automated cross-platform from dev host |
| BND-02 | `ProxyDiagnostic` warning/hint construction | unit | `cargo test -p nono-proxy diagnostic` | In `crates/nono-proxy/src/diagnostic.rs` |
| BND-03 | `CLAUDE.md` updated | review | manual inspection | Boundary table updated |

### Sampling Rate

- **Per cherry-pick commit:** `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`
- **Per theme (after A, after B):** `make ci`
- **Phase gate:** Full `make ci` green + `--workspace --all-targets` clean before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/nono/src/audit.rs` — created by `a5b2a516` cherry-pick; upstream includes tests inline; verify tests land in the fork's copy
- [ ] `crates/nono-cli/src/diagnostic/mod.rs` — must be created as part of resolving `4ad8ba92`; no existing fork equivalent
- [ ] `crates/nono-cli/src/diagnostic/formatter.rs` — body from fork's `diagnostic_formatter.rs`; no existing test file
- [ ] `bindings/c/src/diagnostic.rs` — net-new; upstream tests in `bindings/c/src/lib.rs` test module cover `nono_last_error`; check if `a6aa5995` adds diagnostic-specific tests

---

## ADR Convention

**Existing ADR location and format:** `proj/ADR-74-privilege-model.md` [VERIFIED: `ls proj/`] in the `proj/` directory at repo root. Format:
- Header: `# ADR-{N}: {Title}`
- Meta: `**Status:** Accepted`, `**Phase:**`, `**Date:**`, `**Authors:**`
- Sections: `## Context`, `## Decisions` (with numbered decisions + rationale), `## Consequences` (implied by the example), `## Alternatives Considered` (optional)

**BND-03 ADR substance** (what the ADR must record):
1. The upstream boundary line: audit + structured-diagnostics in core `nono` crate (matching upstream `v0.63.0..v0.64.0`)
2. The single deliberate fork carve-out: Windows denial rendering stays CLI-side, bridged at surface (D-02)
3. The invariant being changed: the prior "library is a pure policy-free sandbox primitive with no audit/diagnostic UX" is partially retired; the library now owns audit logic and structured diagnostic facts (not UX)
4. The new invariant: library owns audit integrity/ledger/attestation AND structured diagnostic codes/facts; CLI owns all rendering, flag suggestions, and interactive UX; Windows denial paths are bridged (not converged)
5. Future sync guidance: the Windows denial path is a deliberate fork carve-out — it is NOT drift. The next upstream sync should re-evaluate Phase F-89 convergence vs. continued bridging.

**File name:** `proj/ADR-86-library-boundary-convergence.md` (matching the `ADR-{phase}-{slug}` pattern from `ADR-74`).

**`CLAUDE.md` § Library vs CLI Boundary update (BND-03):**
The current table shows audit (`audit_attestation.rs`, `audit_commands.rs`, `audit_integrity.rs`, `audit_session.rs`) as CLI-side and `DiagnosticFormatter` as library-side. After Phase 86:
- Add to "In Library" column: `audit` module (AuditRecorder, ledger, attestation, merkle), `diagnostic/*` (structured codes, facts, report — NOT UX)
- Remove from "In Library" column: `DiagnosticFormatter` (moves to CLI)
- Add to "In CLI" column: `diagnostic/formatter.rs` (DiagnosticFormatter, all rendering), thin audit wrappers
- Add carve-out note: "Windows denial rendering: CLI-side via bridge (Phase 86 D-02); full convergence deferred"

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| DiagnosticFormatter in core library | DiagnosticFormatter in CLI; structured codes/facts in core | upstream v0.63.0–v0.64.0 (`4ad8ba92`, `a6aa5995`) | Fork aligns; library is leaner, CLI is richer |
| Audit logic in nono-cli (4 files) | Audit logic in nono core (`audit.rs`) | upstream v0.63.0 (`a5b2a516`–`e9529312`) | Fork aligns; audit is a library primitive |
| No FFI diagnostic surface | 3 extern-C fns + `NonoDiagnosticCode` repr-C enum | upstream v0.64.0 (`a6aa5995`) | FFI clients (nono-py, nono-ts) gain structured diagnostic access |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `diagnostic_code` and `remediation` are methods on `NonoError`, not new enum variants — `map_error` exhaustive match is not directly broken | FFI Exhaustive-Match Closure | If they ARE new variants, `map_error` needs 2 new arms immediately after `a6aa5995` or E0004 fails compilation |
| A2 | `a6aa5995`'s `bindings/c/Cargo.toml` change is a minor tweak (feature flag or version) not a new dependency | Standard Stack | If a new dep is added, it must be verified for legitimacy and added to the workspace manifest |
| A3 | The fork's `crates/nono-cli/src/output.rs::print_proxy_diagnostics` either already exists or arrives via `a6aa5995` | Proxy ProxyDiagnostic Surface | If it doesn't arrive and isn't already there, the `crates/nono-proxy/src/diagnostic.rs` CREATE has no consumer — benign compilation but dead code (clippy `dead_code` lint may warn) |
| A4 | `a5b2a516` creates `audit_ledger.rs` as a thin-wrapper stub alongside `audit_integrity.rs` when cherry-picked into the fork | Per-Commit Conflict Map | If it does not, `aed35bec` will produce a "file not found" conflict requiring manual path resolution |
| A5 | The `lib.rs` re-export for `pub use diagnostic::*` (after `a6aa5995`) replaces the explicit 7-type list with a wildcard, removing the need to manually enumerate the new module's exports | Theme B Module-Dir Conflict | If upstream keeps an explicit list, the fork must enumerate all new diagnostic module types; missing one = compile error |

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `git cherry-pick` | All 8 commits | Yes | git 2.x | Manual diff port via `git show <SHA>` |
| upstream remote | Cherry-pick (objects already fetched) | Yes — all 8 SHAs confirmed in local object store | n/a | n/a |
| `cargo clippy --target x86_64-unknown-linux-gnu` | `f867aba2` (`supervisor_linux.rs`) | Unknown — dev host is Windows 11 | n/a | Mark BND-02 PARTIAL; defer to live CI per cross-target-verify-checklist.md |
| `cargo clippy --target x86_64-apple-darwin` | `f867aba2` | Unknown — dev host is Windows 11 | n/a | Mark BND-02 PARTIAL; defer to live CI |
| `make ci` | Phase gate | Yes (Windows local) | n/a | n/a |

**Missing dependencies with no fallback:** None that block cherry-picking.

**Missing dependencies with fallback:** Cross-target clippy for `f867aba2`'s Linux hunk → mark PARTIAL + defer to CI.

---

## Security Domain

Phase 86 is a code-relocation refactor. The security invariants that apply:

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | partial | Diagnostic string construction — `sanitize_for_diagnostic` in the moved code; must migrate with UX code to CLI |
| V6 Cryptography | partial | Audit merkle/hash functions move to core — use `sha2` (already in `nono` crate Cargo.toml [VERIFIED]) |

### Security Considerations for This Phase

| Pattern | Standard Mitigation |
|---------|---------------------|
| Diagnostic string injection (attacker-controlled paths in denial output) | `sanitize_for_diagnostic` fn moves with `DiagnosticFormatter` to CLI; verify it is included in `4ad8ba92` cherry-pick |
| Audit log integrity | merkle/inclusion-proof moves to core; tests must verify correctness (BND-01 / D-04) |
| FFI memory safety for 3 new extern-C fns | All FFI fns follow established pattern in `bindings/c/src/`; `// SAFETY:` docs required per CLAUDE.md |
| Exhaustive enum match | Use `cargo clippy` E0004; never `_ =>` catch-all on `NonoError` match |

---

## Open Questions

1. **Does `a5b2a516` create `audit_ledger.rs` or does the fork need to manually create it?**
   - What we know: the upstream tree at `a5b2a516` has both `audit_integrity.rs` (thin) and `audit_ledger.rs` (thin); the fork has only `audit_integrity.rs`
   - What's unclear: whether cherry-pick produces `audit_ledger.rs` as part of its file-creation logic or only if the parent state matches
   - Recommendation: executor should inspect `git show a5b2a516 -- crates/nono-cli/src/` before cherry-picking; if the file-create vs. file-modify semantics are ambiguous, manually create `audit_ledger.rs` with upstream's thin-wrapper content as a pre-step

2. **Are `NonoError::diagnostic_code` and `remediation` methods or new variants?**
   - What we know: DIVERGENCE-LEDGER says "NonoError::{diagnostic_code, remediation}" and `error.rs (+137)`. SEED-006 says "+137" lines added to `error.rs`. Methods (impl block) would add ~137 lines; new variants would be fewer lines but break the exhaustive match.
   - What's unclear: exact shape
   - Recommendation: `git show a6aa5995 -- crates/nono/src/error.rs` is the definitive answer; the executor must run this before resolving `a6aa5995` conflicts

3. **Does `crates/nono-cli/src/output.rs::print_proxy_diagnostics` exist in the fork today?**
   - What we know: the DIVERGENCE-LEDGER notes it as a CLI function; the fork's `output.rs` exists [VERIFIED: `ls`]
   - What's unclear: whether this fn was forward-ported in an earlier phase or arrives with `a6aa5995`
   - Recommendation: `grep -n "print_proxy_diagnostics" crates/nono-cli/src/output.rs` before cherry-picking `a6aa5995`

---

## Sources

### Primary (HIGH confidence)
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` — per-commit SHA inventories, re-export findings, Pitfall 3 closure, all file lists. The ledger's actual-diff findings are authoritative.
- Live fork tree inspection via Read + Bash tools — `ls`, file reads, `git cat-file`, `git remote -v`, `git show --name-only` for all 8 SHAs. All fork-state claims tagged [VERIFIED] are from live reads.
- `bindings/c/src/lib.rs` — full `map_error` match block read (lines 72–171) confirms current exhaustive coverage.
- `bindings/c/src/types.rs` — full file read confirms `NonoDiagnosticCode` does NOT yet exist in fork.
- `crates/nono/src/lib.rs` — full file read confirms current `pub use diagnostic::{...}` re-export list.
- `crates/nono/src/error.rs` — full file read confirms all current `NonoError` variants.
- `crates/nono/src/diagnostic.rs` — first 1,575 lines read; confirms `DiagnosticFormatter`, `try_canonicalize` fork-local copy, structural facts.

### Secondary (MEDIUM confidence)
- `.planning/phases/86-library-boundary-convergence/86-CONTEXT.md` — locked decisions D-01 through D-05
- `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md` — function inventory for Themes A and B
- `proj/ADR-74-privilege-model.md` — ADR house style reference
- `crates/nono-cli/src/exec_strategy_windows/{launch,network,mod}.rs` — partial reads confirming Windows denial mechanism (string-embedded errors, no structured diagnostic types)

### Tertiary (LOW confidence)
- A1–A5 in Assumptions Log — inferred from ledger findings without reading the actual upstream diffs for those specific file sections

---

## Metadata

**Confidence breakdown:**
- Cherry-pick feasibility: HIGH — all 8 SHAs verified in local object store
- Conflict surface map: HIGH for file-level, MEDIUM for hunk-level (exact hunk shapes require `git show` at execution time)
- FFI exhaustive-match closure: HIGH — existing match fully read, new types identified
- Windows bridge: HIGH — existing Windows error mechanism confirmed via grep; bridge contract is method-surface, not structural change
- Module-dir migration: HIGH — fork's diagnostic.rs fully inventoried, upstream module structure cited from ledger

**Research date:** 2026-06-19
**Valid until:** 2026-07-19 (stable codebase; no external dependency drift risk; upstream remote object store is frozen for the 8 SHAs)
