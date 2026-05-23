---
phase: "45"
plan: "45-02"
subsystem: aipc-wire-protocol
tags: [breaking-change, wire-format, security, compile-time-guarantee, ipc]
dependency_graph:
  requires: [45-01]
  provides: [SC#2-compile-time-guarantee, ApprovalDecision-Approved-variant]
  affects: [supervisor/types.rs, supervisor/aipc_sdk.rs, exec_strategy_windows/supervisor.rs, terminal_approval.rs]
tech_stack:
  added: []
  patterns: [rust-enum-variant-payload, compile-time-invariant, ipc-wire-format]
key_files:
  created:
    - .planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/45-02-CLIPPY-CROSS-TARGET.md
  modified:
    - crates/nono/src/supervisor/types.rs
    - crates/nono/src/supervisor/aipc_sdk.rs
    - crates/nono/src/supervisor/mod.rs
    - crates/nono/src/supervisor/socket.rs
    - crates/nono/src/supervisor/socket_windows.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
    - crates/nono-cli/src/terminal_approval.rs
    - CHANGELOG.md
    - docs/architecture/audit-bundle-target.md
decisions:
  - "D-45-C1: Single atomic feat commit for all cascade changes (partial migration = compile break by design)"
  - "D-45-C2: Wire-format break accepted — pre-v2.6 ledgers non-re-verifiable via typed path but still verifiable via raw-JSON audit verifier"
  - "D-45-C3: Renamed Granted → Approved in same atomic commit as wire-format change"
  - "SC#2 compile-time guarantee: (Approved, grant=None) is now structurally unrepresentable in Rust type system"
  - "Ok(None) broker path fail-secure: broker helpers returning Ok(None) treated as broker failure, not silent success"
metrics:
  duration: "~90 minutes (resumed from previous context)"
  completed: "2026-05-23"
  tasks_completed: 2
  files_changed: 11
---

# Phase 45 Plan 45-02: ApprovalDecision Wire-Format BREAKING Change Summary

**One-liner:** `ApprovalDecision::Granted → Approved(ResourceGrant)` — SC#2 compile-time guarantee that (Approved, grant=None) is structurally unrepresentable in the AIPC wire protocol.

## What Was Built

Plan 45-02 implements a BREAKING wire-format change to the AIPC (AI Process Communication) supervisor IPC protocol:

1. **`ApprovalDecision::Granted` → `Approved(ResourceGrant)`** — the resource grant payload is now inlined into the variant, not carried as a separate optional field.
2. **`SupervisorResponse::Decision.grant` field removed** — `grant: Option<ResourceGrant>` is gone; the grant is structurally inside `Approved(grant)`.
3. **`is_granted()` renamed `is_approved()`** — in the `impl ApprovalDecision` block.
4. **SC#2 `ok_or_else` branch removed from `aipc_sdk.rs`** — the defense-in-depth "supervisor granted but returned no ResourceGrant" check is no longer needed; the type system enforces it.
5. **All consumers cascade-updated** in one atomic commit (D-45-C1).

## Commits

| Hash | Message |
|------|---------|
| `4a60f675` | `feat(45-02): ApprovalDecision::Granted → Approved(ResourceGrant) — SC#2 compile-time guarantee` |
| `7a6038fc` | `docs(45-02): cross-target clippy verification artifact (PARTIAL)` |

## Task Execution

### Task 1: Inventory (completed in prior session)
- Confirmed nono-py has `Granted` in `python/nono_py/audit.py:433` — deferred per D-44-D1 lockstep situation (documented in CONTEXT.md § Deferred Ideas).
- nono-ts: no references.

### Task 2: Cascade edits + atomic commit
Cascade order: `types.rs` → `aipc_sdk.rs` → `mod.rs` → `socket.rs` → `socket_windows.rs` → `exec_strategy.rs` → `supervisor_linux.rs` → `exec_strategy_windows/supervisor.rs` → `terminal_approval.rs`.

Key fix applied automatically (deviation Rule 1): `exec_strategy_windows/supervisor.rs` broker dispatcher received `Ok(g)` where `g: Option<ResourceGrant>`. After type change, `Ok(None)` is treated fail-secure as a broker error (not a silent empty success), yielding `Denied { reason: "broker returned Ok(None) — no ResourceGrant (internal error)" }`. This preserves the security invariant while handling the edge case the type system exposed.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo check --workspace --all-features` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | PASS |
| `cargo test --workspace --all-features` | PASS (694+ unit tests, 0 failures) |
| AUD-05: `recorded_ledger_redacts_session_token` | PASS (verbatim) |
| Cross-target Linux clippy | PARTIAL (C cross-linker absent — deferred to CI) |
| Cross-target macOS clippy | PARTIAL (C cross-linker absent — deferred to CI) |

## Pitfall Compliance

| Pitfall | Status |
|---------|--------|
| Pitfall 3: `audit_entry_with_redacted_token` at supervisor.rs:1303-1318 preserved verbatim | VERIFIED — not edited |
| Pitfall 4: `audit_integrity.rs:83-93` docstring about `reject_stage` not edited | VERIFIED — not touched |
| AUD-05 test `recorded_ledger_redacts_session_token` at supervisor.rs:5037 preserved verbatim | VERIFIED — PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `Ok(None)` broker path required fail-secure handling**
- **Found during:** Task 2, exec_strategy_windows/supervisor.rs dispatcher
- **Issue:** After changing broker functions to return `Result<Option<ResourceGrant>>` (returning `Ok(Some(g))` on success), the match `Ok(g) => (nono::ApprovalDecision::Approved(g), true)` where `g: Option<ResourceGrant>` caused a type error. A new `Ok(None)` branch needed to be handled.
- **Fix:** Added explicit normalization: `Ok(Some(g)) => Ok(g)`, `Ok(None) => Err(NonoError::SandboxInit("broker returned Ok(None) — no ResourceGrant (internal error)"))`, `Err(e) => Err(e)`. The `Ok(None)` case treats an unexpected empty result as a broker error, triggering the G-04 flip to `Denied`.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`
- **Commit:** `4a60f675`

**2. [Rule 1 - Bug] Additional `grant: None` and `grant.is_none()` sites in test assertions**
- **Found during:** Task 2, exec_strategy_windows/supervisor.rs test module
- **Issue:** 10 additional test sites contained `grant, ..` pattern matches and `grant.is_none()` assertions referencing the now-removed `grant` field of `SupervisorResponse::Decision`. These were the G-04 broker-failure tests, containment-Job guard tests, privileged-port tests, and bind-role tests.
- **Fix:** All 10 sites updated to match `decision, ..` and assert `matches!(decision, nono::ApprovalDecision::Denied { .. })` instead. Comments updated to reflect the new compile-time invariant.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`
- **Commit:** `4a60f675`

**3. [Rule 1 - Comment] Stale doc comment updated**
- **Found during:** Task 2, exec_strategy_windows/supervisor.rs capability_handler_tests module doc
- **Issue:** Module-level `//!` comment mentioned `is_granted()` and "returned `Granted`" — the old API names.
- **Fix:** Updated to `is_approved()` and "returned `Approved`".
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`
- **Commit:** `4a60f675`

## Known Stubs

None. All approval backends return a well-formed `Approved(ResourceGrant)`. The `TerminalApproval` uses `sideband_file_descriptor` as a typed placeholder — this is intentional and documented in the code comment. The actual grant is replaced by the dispatcher in `exec_strategy.rs` before being sent to the child.

## Threat Flags

None. This change narrows the attack surface: the illegal `(Approved, grant=None)` shape can no longer be constructed, reducing the number of possible states the AIPC protocol can be in.

## Cross-Target Clippy

See `45-02-CLIPPY-CROSS-TARGET.md` for the full PARTIAL disposition. Windows-host clippy PASS; Linux/macOS deferred to live CI.

## Self-Check: PASSED

Files created/modified:
- `crates/nono/src/supervisor/types.rs` — FOUND (Approved variant, no grant field, is_approved method)
- `crates/nono/src/supervisor/aipc_sdk.rs` — FOUND (ok_or_else branch removed, match on Approved(grant))
- `crates/nono-cli/src/terminal_approval.rs` — FOUND (Approved(sideband_file_descriptor))
- `CHANGELOG.md` — FOUND (BREAKING entry with wire shape before/after)
- `docs/architecture/audit-bundle-target.md` — FOUND (Amendment 45-A)
- `45-02-CLIPPY-CROSS-TARGET.md` — FOUND (contains literal string PARTIAL)

Commits:
- `4a60f675` — FOUND in git log
- `7a6038fc` — FOUND in git log
