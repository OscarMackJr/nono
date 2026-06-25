---
phase: 92-runtime-capabilityset-mutation-audit-wiring
plan: 01
subsystem: audit-model
tags: [audit, telemetry, siem, policy-override, aud-01, aud-03]
dependency_graph:
  requires: []
  provides:
    - AuditEventPayload::PolicyOverrideApplied variant in crates/nono/src/audit.rs
    - SecurityEventType PolicyOverride* variants in crates/nono-cli/src/telemetry/event.rs
    - EVENT_ID_POLICY_OVERRIDE_* constants 10006-10010 in crates/nono-cli/src/telemetry/event.rs
    - Exhaustive severity_for coverage for all 10 SecurityEventType variants
  affects:
    - crates/nono/src/audit.rs (AuditEventPayload enum extended)
    - crates/nono-cli/src/telemetry/event.rs (SecurityEventType, event_id_for extended)
    - crates/nono-cli/src/telemetry/mod.rs (severity_for extended)
tech_stack:
  added: []
  patterns:
    - serde tag/rename_all=snake_case enum variant pattern (existing)
    - skip_serializing_if=Option::is_none on optional fields (existing)
    - Exhaustive match extension for SecurityEventType (existing)
key_files:
  created: []
  modified:
    - crates/nono/src/audit.rs
    - crates/nono-cli/src/telemetry/event.rs
    - crates/nono-cli/src/telemetry/mod.rs
decisions:
  - "PolicyOverrideApplied is a pure data carrier — no policy logic in the core crate (D-01 / policy-free core invariant)"
  - "All 5 new SecurityEventType variants are Warning-level (authorization events, not denial-only)"
  - "cross-target clippy PARTIAL->CI: cross-toolchain absent on Windows host (CLAUDE.md documented pattern)"
metrics:
  duration: "~20 minutes"
  completed: "2026-06-22"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 3
requirements:
  - AUD-01
  - AUD-03
---

# Phase 92 Plan 01: Data Model Additions Summary

**One-liner:** Added `AuditEventPayload::PolicyOverrideApplied` variant (core crate, data carrier only) and 5 new `SecurityEventType` variants with EventID constants 10006-10010 (nono-cli telemetry), providing the foundation for all downstream Phase 92 plans.

## What Was Built

### Task 1: PolicyOverrideApplied variant in AuditEventPayload (crates/nono/src/audit.rs)

Added a new `PolicyOverrideApplied` variant to the `AuditEventPayload` enum after the existing `Network` variant. The variant is a **pure data carrier** — no policy logic in the core crate (CLAUDE.md policy-free core invariant, D-01 from 92-CONTEXT.md).

Fields:
- `jti: String` — JWT ID (single-use nonce)
- `kms_key_id: String` — KMS key ARN (redaction-safe)
- `zt_audit_hash: Option<String>` — with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- `granted_path_hashes: Vec<String>` — path-hashed per AUD-03 redaction policy
- `expires_at: String` — ISO-8601 expiry timestamp

Three unit tests added (all passing):
- `policy_override_applied_serializes_with_type_tag` — verifies `"type":"policy_override_applied"` in JSON output
- `policy_override_applied_zt_audit_hash_absent_when_none` — verifies `skip_serializing_if` behavior
- `policy_override_applied_round_trips` — serialize/deserialize round-trip equality

### Task 2: EventIDs 10006-10010 and SecurityEventType variants (telemetry/event.rs + mod.rs)

**event.rs changes:**
- 5 new EventID constants after `EVENT_ID_TELEMETRY_DEGRADED`:
  - `EVENT_ID_POLICY_OVERRIDE_PRESENTED = 10006`
  - `EVENT_ID_POLICY_OVERRIDE_VERIFIED = 10007`
  - `EVENT_ID_POLICY_OVERRIDE_REJECTED = 10008`
  - `EVENT_ID_POLICY_OVERRIDE_EXPIRED = 10009`
  - `EVENT_ID_POLICY_OVERRIDE_REVOKED = 10010`
- 5 new `SecurityEventType` variants: `PolicyOverridePresented`, `PolicyOverrideVerified`, `PolicyOverrideRejected`, `PolicyOverrideExpired`, `PolicyOverrideRevoked`
- `event_id_for` exhaustive match extended with 5 new arms
- Module doc table updated to show all 10 EventIDs

**mod.rs changes:**
- `severity_for` extended with a new pipe-grouped arm covering all 5 new variants → `TelemetrySeverity::Warning`

Three new tests added (all passing):
- `override_event_ids_are_10006_through_10010` (in event.rs)
- `override_event_type_serde_roundtrip` (in event.rs)
- `severity_for_override_lifecycle_events_is_warning` (in mod.rs)

## Test Results

```
cargo test -p nono --lib audit::
  15 passed; 0 failed (includes 3 new policy_override_applied_* tests)

cargo test --bin nono
  telemetry::event::tests::override_event_ids_are_10006_through_10010 ... ok
  telemetry::event::tests::override_event_type_serde_roundtrip ... ok
  telemetry::tests::severity_for_override_lifecycle_events_is_warning ... ok
  (4 pre-existing baseline failures: profile_cmd init + 3 protected_paths — documented in nono_cli_windows_baseline_test_failures.md)

cargo build --workspace
  Finished (all 5 crates + bindings green)
```

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| Task 1 | `6ecc67b7` | feat(92-01): add PolicyOverrideApplied variant to AuditEventPayload |
| Task 2 | `fa15bf91` | feat(92-01): add EventIDs 10006-10010 and SecurityEventType PolicyOverride variants |

## Deviations from Plan

None — plan executed exactly as written. Both tasks follow the exact field names, serde attributes, doc-comment style, and match-arm patterns specified in 92-PATTERNS.md.

## Cross-Target Clippy Status

**PARTIAL → CI** (as documented in the plan's acceptance criteria)

Attempted `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — cross-toolchain (`x86_64-linux-gnu-gcc`) is not installed on this Windows 11 host. Same result for `--target x86_64-apple-darwin`.

The changes are fully cfg-unconditional (`event.rs` has no `#[cfg]` guards; `audit.rs` has the new variant at the enum level). The exhaustive match extensions cover all 10 `SecurityEventType` variants in both `event_id_for` and `severity_for`. Native Windows `cargo build --workspace` exits 0. Linux/macOS verification deferred to live CI per CLAUDE.md rule.

## Known Stubs

None — this plan adds data model types only. No UI rendering, no data sources, no stubs.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond those in the plan's `<threat_model>`.

- T-92-01 (Tampering / AuditEventPayload): Mitigated — variant is data carrier only; serde tag enforces type identity at deserialization.
- T-92-02 (Information Disclosure / granted_path_hashes): Mitigated by design — field stores hashes, never raw paths; type enforces the contract at the model level.
- T-92-03 (Elevation of Privilege / match exhaustiveness): Mitigated on Windows host (`cargo build --workspace` green); PARTIAL for Linux/macOS (deferred to CI).

## Self-Check: PASSED

| Item | Status |
|------|--------|
| crates/nono/src/audit.rs exists | FOUND |
| crates/nono-cli/src/telemetry/event.rs exists | FOUND |
| crates/nono-cli/src/telemetry/mod.rs exists | FOUND |
| 92-01-SUMMARY.md exists | FOUND |
| Commit 6ecc67b7 (Task 1) exists | FOUND |
| Commit fa15bf91 (Task 2) exists | FOUND |
