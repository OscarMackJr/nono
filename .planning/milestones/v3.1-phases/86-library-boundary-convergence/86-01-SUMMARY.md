---
phase: 86-library-boundary-convergence
plan: "01"
subsystem: audit
tags: [audit, attestation, ledger, merkle, nono-core, library-boundary, cherry-pick, upstream-sync]

requires:
  - phase: 85-upst9-divergence-audit
    provides: DIVERGENCE-LEDGER mapping 4 audit commits to cherry-pick

provides:
  - "crates/nono/src/audit.rs — full audit/ledger/merkle/attestation logic in core library"
  - "crates/nono-cli audit_integrity.rs, audit_ledger.rs, audit_attestation.rs as thin wrappers"
  - "BND-01 satisfied: audit stack lives in nono core matching upstream"

affects:
  - 86-02-PLAN
  - 86-03-PLAN
  - nono-ffi (audit.rs is now part of the core library surface)

tech-stack:
  added: []
  patterns:
    - "Fork-extend upstream structs: add fork-only fields/methods to upstream types without forking the type itself (AuditVerificationResult.chain_head_matches, AuditAttestationVerificationResult.is_valid())"
    - "Thin-wrapper CLI modules: nono-cli audit_*.rs re-export from nono::audit via pub(crate) use"
    - "write_audit_attestation converged with upstream: rollback_runtime now uses SessionMetadata-based attestation API (upstream 6ecade2e pattern)"

key-files:
  created:
    - "crates/nono/src/audit.rs (4071 lines, all audit/ledger/merkle/attestation logic + tests)"
    - "crates/nono-cli/src/audit_ledger.rs (pre-state file introduced as preparatory commit, then shrunk to thin wrapper)"
  modified:
    - "crates/nono/src/lib.rs (added pub mod audit)"
    - "crates/nono-cli/src/audit_integrity.rs (thin wrapper, re-exports from nono::audit)"
    - "crates/nono-cli/src/audit_attestation.rs (thin wrapper, write_audit_attestation + verify)"
    - "crates/nono/src/trust/signing.rs (added sign_statement_bundle, public_key_id_hex)"
    - "crates/nono/src/trust/mod.rs (re-export new trust functions)"
    - "crates/nono-cli/src/trust_cmd.rs (TRUST_SERVICE made pub(crate))"
    - "crates/nono-cli/src/rollback_runtime.rs (migrated from sign_session_attestation to write_audit_attestation)"
    - "crates/nono-cli/src/supervised_runtime.rs (prepare_audit_signer call fix: Some(key_ref))"
    - "crates/nono-cli/src/audit_commands.rs (verify_audit_attestation updated to 3-arg upstream API)"
    - "crates/nono-cli/src/exec_strategy_windows/supervisor.rs (record_capability_decision_with_reject_stage)"

key-decisions:
  - "Fork-specific types preserved in nono::audit: RejectStage enum, reject_stage on CapabilityDecision, chain_head_matches/merkle_root_matches on AuditVerificationResult, is_valid() on both result types"
  - "sign_session_attestation removed: rollback_runtime converged to upstream write_audit_attestation pattern using full SessionMetadata, eliminating synthetic-subject attestation path"
  - "audit_ledger.rs pre-state strategy: upstream's aed35bec cherry-pick assumes audit_ledger.rs exists (rename/split ancestor); manually bootstrapped from upstream parent state before applying the shrink-to-wrapper commit"
  - "Rust 2021 let-chain incompatibility: all if-let-and patterns converted to nested if-let blocks (3 occurrences in audit.rs)"
  - "cargo fmt workspace drift committed as separate style commit to keep cherry-pick provenance tracers clean"

metrics:
  duration: "~4h (across two context windows)"
  completed: "2026-06-19"
  tasks-completed: 4
  files-changed: 16
---

# Phase 86 Plan 01: Library-Boundary Convergence (BND-01) Summary

Cherry-picked 4 upstream audit commits (`a5b2a516` → `aed35bec` → `0b27cfc2` → `e9529312`) in ledger order onto `milestone/v2.13-carryforward-closeout`, moving the full audit/ledger/merkle/attestation stack from `nono-cli` into `crates/nono/src/audit.rs` with the 4 CLI-side files reduced to thin `pub(crate) use nono::audit::*` wrappers.

## Commits Produced

| Hash | Type | Description |
|------|------|-------------|
| `21e0088c` | refactor | move audit integrity logic to nono crate (a5b2a516 cherry-pick) |
| `44a5d43f` | chore | introduce audit_ledger.rs pre-state for aed35bec cherry-pick |
| `4b7cfd90` | refactor | move audit ledger logic to library crate (aed35bec cherry-pick) |
| `7faabe9e` | refactor | move attestation logic to core library (0b27cfc2 cherry-pick + fork compat) |
| `6a0bd80a` | fix | address ledger review and clippy (e9529312 cherry-pick) |
| `52f073f6` | style | apply cargo fmt to pre-existing workspace drift |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Rust 2021 let-chain incompatibility in audit.rs**
- **Found during:** Task 1 (a5b2a516 cherry-pick), gate `cargo clippy`
- **Issue:** `if let Some(x) = y && condition` syntax is Rust 2024-only; workspace uses edition 2021
- **Fix:** 3 occurrences converted to nested `if let Some(x) = y { if condition { } }` in audit.rs
- **Files modified:** `crates/nono/src/audit.rs`

**2. [Rule 1 - Bug] `index % 2 == 0` → `index.is_multiple_of(2)` (clippy::manual_is_multiple_of)**
- **Found during:** Task 1 gate
- **Fix:** 2 occurrences in audit.rs
- **Files modified:** `crates/nono/src/audit.rs`

**3. [Rule 2 - Missing] Fork-specific types absent from upstream audit.rs**
- **Found during:** Tasks 1-2
- **Issue:** Upstream `audit.rs` missing `RejectStage` enum, `reject_stage` on `CapabilityDecision`, `chain_head_matches`/`merkle_root_matches` on `AuditVerificationResult`, `is_valid()` methods on both result types
- **Fix:** Added all fork-specific extensions to `nono::audit` as part of the relocated module
- **Files modified:** `crates/nono/src/audit.rs`

**4. [Rule 1 - Bug] audit_ledger.rs pre-state missing**
- **Found during:** Task 2 (aed35bec cherry-pick)
- **Issue:** Upstream's `aed35bec` moves content from `audit_ledger.rs` to core library, but the fork never had `audit_ledger.rs` (it was created by `a5b2a516` in upstream's tree which takes a different rename path in the fork context)
- **Fix:** Bootstrapped `audit_ledger.rs` from `git show aed35bec^:crates/nono-cli/src/audit_ledger.rs` (559-line parent state) as a preparatory commit, then let the `aed35bec` cherry-pick shrink it to the thin wrapper
- **Commits:** `44a5d43f` (pre-state), `4b7cfd90` (shrink)

**5. [Rule 2 - Missing] trust::sign_statement_bundle and trust::public_key_id_hex absent**
- **Found during:** Task 3 (0b27cfc2 cherry-pick gate)
- **Issue:** Upstream's `audit_attestation.rs` thin wrapper calls `nono::trust::sign_statement_bundle` and `nono::trust::public_key_id_hex` which don't exist in the fork's trust module
- **Fix:** Added both functions to `crates/nono/src/trust/signing.rs` and updated the `trust/mod.rs` re-export list
- **Files modified:** `crates/nono/src/trust/signing.rs`, `crates/nono/src/trust/mod.rs`

**6. [Rule 1 - Bug] rollback_runtime.rs used old sign_session_attestation API (dead code)**
- **Found during:** Task 3 gate — `write_audit_attestation` was dead; `sign_session_attestation` removed by plan but still needed
- **Issue:** After thin-wrapping `audit_attestation.rs`, `rollback_runtime.rs` still imported the removed `sign_session_attestation` function. Adding it back as a compat shim left `write_audit_attestation` unused (dead code, clippy error).
- **Fix:** Converged `rollback_runtime.rs` to upstream's approach — build `SessionMetadata` first with `audit_attestation: None`, then call `write_audit_attestation` after (matching upstream `6ecade2e` pattern). Removed `sign_session_attestation` entirely.
- **Files modified:** `crates/nono-cli/src/rollback_runtime.rs`, `crates/nono-cli/src/audit_attestation.rs`

**7. [Rule 1 - Bug] prepare_audit_signer called with wrong argument type**
- **Found during:** Task 3 gate
- **Issue:** `supervised_runtime.rs` called `prepare_audit_signer(key_ref)` (passing `&str`) but upstream signature is `prepare_audit_signer(Option<&str>)`
- **Fix:** Changed call to `prepare_audit_signer(Some(key_ref))`
- **Files modified:** `crates/nono-cli/src/supervised_runtime.rs`

**8. [Rule 1 - Bug] verify_audit_attestation old 5-arg signature in audit_commands.rs**
- **Found during:** Task 3 gate
- **Issue:** `audit_commands.rs` called old fork API with 5 separate args; upstream's thin wrapper has 3-arg API `(session_dir, metadata, public_key_file)`
- **Fix:** Updated call site to new 3-arg API; added `.is_valid()` result check
- **Files modified:** `crates/nono-cli/src/audit_commands.rs`

**9. [Rule 1 - Bug] CapabilityDecision test struct missing fork fields (CapabilityRequest)**
- **Found during:** Gate checks
- **Issue:** Upstream's `audit.rs` test created `CapabilityRequest` without fork-added fields (`session_token`, `kind`, `target`, `access_mask`)
- **Fix:** Added all fields with defaults; added `#[allow(deprecated)]` for the deprecated `path` field
- **Files modified:** `crates/nono/src/audit.rs`, `crates/nono-cli/src/audit_integrity.rs`

**10. [Rule 1 - Bug] SessionMetadata test struct missing fork rollback_status field**
- **Found during:** Gate checks
- **Issue:** Upstream test struct literal missing fork-added `rollback_status: RollbackStatus` field
- **Fix:** Added `rollback_status: Default::default()` in all test struct literals
- **Files modified:** `crates/nono/src/audit.rs`, `crates/nono-cli/src/audit_attestation.rs`

## Known Stubs

None — all audit business logic wired to `nono::audit`; CLI wrappers delegate correctly.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. The audit module moves from CLI to core library without changing the audit schema or trust model.

## Test Results

- `cargo test -p nono -- audit`: 11 passed, 0 failed
- `cargo test -p nono-cli` (audit-related): 24+ audit/attestation/ledger tests green
- Pre-existing baseline failures (env-specific, documented in MEMORY): 1 in nono (try_set_mandatory_label), 4 in nono-cli (profile_cmd + 3 protected_paths)
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`: PASS
- `cargo fmt --all -- --check`: PASS

## Self-Check: PASSED

- [x] `crates/nono/src/audit.rs` — exists (4000+ lines with full business logic + tests)
- [x] `crates/nono-cli/src/audit_integrity.rs` — thin wrapper (`pub(crate) use nono::audit::*`)
- [x] `crates/nono-cli/src/audit_ledger.rs` — thin wrapper
- [x] `crates/nono-cli/src/audit_attestation.rs` — thin wrapper
- [x] `crates/nono/src/lib.rs` — contains `pub mod audit`
- [x] All 6 commits present in `git log --oneline -8`
- [x] Clippy gate: PASS
- [x] Fmt check: PASS
