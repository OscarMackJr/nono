---
phase: 114-oauth-capture-absorb-sec-02
plan: 01
subsystem: audit
tags: [oauth-capture, audit, sec-02, adr-86, network-audit]

# Dependency graph
requires: []
provides:
  - "CaptureAuditContext pure-data struct in crates/nono/src/undo/types.rs (route_id, rewritten_fields, phantom_ids)"
  - "NetworkAuditDenialCategory::CaptureUnsupportedPath variant for the Plan 114-07 D-06 fail-closed guard"
  - "NetworkAuditEvent.capture_context field, threaded through EventContext and all 4 nono-proxy push_event call sites"
affects: [114-05, 114-06, 114-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-data audit-context struct mirroring SpiffeAuditContext (Phase 113 precedent) — identifiers only, zero if/match logic, ADR-86-safe"
    - "Denial-category variant added ahead of need so a later guard plan needs no second enum edit (mirrors SpiffeUnsupportedPath)"

key-files:
  created: []
  modified:
    - crates/nono/src/undo/types.rs
    - crates/nono-proxy/src/audit.rs
    - crates/nono/src/audit.rs
    - crates/nono-cli/src/audit_integrity.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/proxy_command.rs

key-decisions:
  - "CaptureAuditContext has exactly 3 fields (route_id: String, rewritten_fields: Vec<String>, phantom_ids: Vec<String>) — no field capable of holding token bytes, satisfying D-08 structurally"
  - "EventContext.capture_context imported and referenced by short name (CaptureAuditContext), not the plan's literal fully-qualified nono::undo::CaptureAuditContext — matches the file's existing spiffe_context import/usage pattern; the one plan acceptance-criteria grep for the fully-qualified string does not match as written (see Deviations)"
  - "Rule 3 auto-fix: capture_context: None also added to 4 test/production NetworkAuditEvent literals outside this plan's files_modified list (crates/nono/src/audit.rs x2, crates/nono-cli/src/audit_integrity.rs, crates/nono-cli/src/exec_strategy/supervisor_linux.rs, crates/nono-cli/src/proxy_command.rs) — required for cargo build --workspace --all-targets to pass; the plan's own per-task verify commands (cargo build -p nono-sandbox lib-only, cargo test -p nono-sandbox-proxy audit::) did not surface these because they don't compile the nono-sandbox test target or the nono-sandbox-cli crate"

patterns-established:
  - "Pattern already established by Phase 113 (SpiffeAuditContext); this plan is a straight application, not a new pattern"

requirements-completed: [SEC-02]

# Metrics
duration: ~25min
completed: 2026-08-06
---

# Phase 114 Plan 01: OAuth Capture Audit Vocabulary Summary

**Added `CaptureAuditContext` (pure-data, 3 identifier fields, no token-capable field) and `NetworkAuditDenialCategory::CaptureUnsupportedPath` to the core `nono` crate, then threaded `capture_context` through `nono-proxy`'s `EventContext` and all `NetworkAuditEvent` construction sites workspace-wide.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 2 planned + 1 Rule-3 blocking-issue fix
- **Files modified:** 6

## Accomplishments
- `CaptureAuditContext` struct exists in `crates/nono/src/undo/types.rs`, mirroring `SpiffeAuditContext`'s shape exactly (`#[derive(Debug, Clone, Serialize, Deserialize)]`, zero `impl` logic) — structurally satisfies D-08 (cannot carry a raw token) and D-09 (ADR-86 boundary preserved).
- `NetworkAuditDenialCategory::CaptureUnsupportedPath` variant added ahead of need for Plan 114-07's D-06 cross-path fail-closed guard, doc-commented with the same shape as `SpiffeUnsupportedPath`.
- `NetworkAuditEvent.capture_context: Option<CaptureAuditContext>` field added immediately after `spiffe_context`, `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- `nono-proxy`'s `EventContext<'a>` gained `capture_context: Option<CaptureAuditContext>`; all 4 `push_event` construction sites (`log_allowed`, `log_denied`, `log_l7_request`, `log_reverse_proxy`) now thread it through.
- `cargo build --workspace --all-targets` exits 0; `cargo fmt --all -- --check` clean.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add CaptureAuditContext + CaptureUnsupportedPath to the core nono crate** - `3345468c` (feat)
2. **Task 2: Wire capture_context through EventContext and all 4 audit emitters** - `273db64f` (feat)
3. **Rule 3 blocking-issue fix: add capture_context to remaining NetworkAuditEvent literals** - `3fe78d29` (fix)

## Files Created/Modified
- `crates/nono/src/undo/types.rs` - `CaptureAuditContext` struct, `CaptureUnsupportedPath` variant, `NetworkAuditEvent.capture_context` field
- `crates/nono-proxy/src/audit.rs` - `EventContext.capture_context` field + `capture_context` line in all 4 `push_event` call sites
- `crates/nono/src/audit.rs` - `capture_context: None,` added to 2 `#[cfg(test)]` `NetworkAuditEvent` literals (Rule 3)
- `crates/nono-cli/src/audit_integrity.rs` - `capture_context: None,` added to 1 test `NetworkAuditEvent` literal (Rule 3)
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` - `capture_context: None,` added to `record_network_audit_denial`'s production `NetworkAuditEvent` literal (Rule 3, Linux AF_UNIX audit denial path)
- `crates/nono-cli/src/proxy_command.rs` - `capture_context: None,` added to 1 test `NetworkAuditEvent` literal (Rule 3)

## Decisions Made
- **Short-name import over fully-qualified path:** `crates/nono-proxy/src/audit.rs` already imports `SpiffeAuditContext` by name and uses it unqualified (`Option<SpiffeAuditContext>`). `CaptureAuditContext` was added the same way (`use nono::undo::{CaptureAuditContext, ...}`, then `Option<CaptureAuditContext>`), matching the file's own established convention rather than the plan's `<interfaces>` block literal text `Option<nono::undo::CaptureAuditContext>`. This is a live-source-wins call per the plan's own ground-truth caveat — the file's real pattern is more authoritative than the plan's inline example. Functionally identical; only affects one of the plan's acceptance-criteria grep strings (see Deviations).
- **CaptureAuditContext field types:** `route_id: String`, `rewritten_fields: Vec<String>`, `phantom_ids: Vec<String>` exactly as specified in the plan's `<action>` block — no deviation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `capture_context: None,` to 4 NetworkAuditEvent literals outside the plan's `files_modified` list**
- **Found during:** Post-Task-2 workspace verification (`cargo build --workspace --all-targets`, run per this plan's overall `<verification>` requirement, not either task's narrower per-task verify command)
- **Issue:** `NetworkAuditEvent` gained a new non-`Default`-derived field (the struct itself has no `#[derive(Default)]`). Every existing struct-literal construction site across the workspace needed the new field or the build fails with E0063. Task 1's own verify command (`cargo build -p nono-sandbox`, lib target only) and Task 2's (`cargo build -p nono-sandbox-proxy` + `cargo test -p nono-sandbox-proxy audit::`) never compiled `nono-sandbox`'s own test target or the `nono-sandbox-cli` crate, so they didn't surface the 4 additional sites: `crates/nono/src/audit.rs` (2x, in `#[cfg(test)]`), `crates/nono-cli/src/audit_integrity.rs` (1x, test), `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (1x, production — Linux AF_UNIX network-deny audit path), `crates/nono-cli/src/proxy_command.rs` (1x, test).
- **Fix:** Added `capture_context: None,` immediately after each site's existing `spiffe_context: None,` line — no capture context is available at any of these 4 call sites (none involve a capture-declared route).
- **Files modified:** `crates/nono/src/audit.rs`, `crates/nono-cli/src/audit_integrity.rs`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, `crates/nono-cli/src/proxy_command.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0; `cargo fmt --all -- --check` clean.
- **Committed in:** `3fe78d29`

**2. [Discrepancy, not a code deviation] One plan acceptance-criteria grep does not match**
- **Found during:** Task 2 acceptance-criteria verification
- **Issue:** The plan's stated acceptance criterion `grep -c "pub capture_context: Option<nono::undo::CaptureAuditContext>" crates/nono-proxy/src/audit.rs` returns 0, because the field was written as `pub capture_context: Option<CaptureAuditContext>` (short name, imported) to match the file's existing `spiffe_context: Option<SpiffeAuditContext>` convention.
- **Resolution:** No code change — the short-name form is functionally identical and structurally consistent with the file. Recorded here per the "live source wins, record the discrepancy" instruction; not treated as a defect.
- **Committed in:** `273db64f`

---

**Total deviations:** 2 (1 Rule-3 auto-fix, 1 acceptance-criteria-text discrepancy with no functional impact)
**Impact on plan:** The Rule-3 fix was necessary for the plan's own overall `<verification>` requirement (`cargo build --workspace` exits 0) to hold — no scope creep, purely additive `None` defaults at pre-existing call sites. The acceptance-criteria discrepancy has zero functional impact.

## Issues Encountered
None beyond the deviations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
`CaptureAuditContext`, `NetworkAuditDenialCategory::CaptureUnsupportedPath`, and `NetworkAuditEvent.capture_context` / `EventContext.capture_context` are all in place for Plans 114-05/114-06 (buffer-and-rewrite helper) and 114-07 (D-06 cross-path guard) to consume without a second edit to `types.rs` or `audit.rs`'s `EventContext`/`push_event` shape. No blockers.

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-06*
