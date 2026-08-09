---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 02
subsystem: infra
tags: [windows, sandbox, security, error-handling, diagnostics, machine-policy, adr-86]

# Dependency graph
requires: []
provides:
  - "crates/nono/src/error.rs — NonoError::LayerAttestationFailed { layer, reason } variant + diagnostic_code() match arm (D-22)"
  - "crates/nono/src/diagnostic/codes.rs — NonoDiagnosticCode::LayerAttestationFailed variant (additive, non_exhaustive-safe)"
  - "crates/nono/src/machine_policy.rs — pub RequiredLayersPolicy { required: Vec<String> } + MachineEgressPolicy.required_layers field (D-26)"
  - "bindings/c/src/lib.rs — map_error() ErrSandboxInit arm for LayerAttestationFailed (FFI exhaustive-match fix)"
affects: [117-08-cli-decision-logic, 117-09-channels, 117-10-gate-insertion]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Own diagnostic_code() match arm (not folded into ConfigurationError) so an operator can distinguish an attestation failure from generic misconfiguration — precedent: TrustVerificationFailed's own arm"
    - "Platform-neutral NonoError variant, no cfg gate, matching LabelApplyFailed's unconditional-compile precedent (D-11)"
    - "pub (not pub(crate)) sub-struct nested in a pub struct, matching TelemetryConfig's cross-crate-consumer visibility precedent — avoids the private_interfaces clippy lint"
    - "Degrade-not-abort read lifecycle for a new machine-policy sub-section, matching TelemetryConfig's D-14 precedent rather than the egress fields' D-07 abort precedent"

key-files:
  created: []
  modified:
    - crates/nono/src/error.rs
    - crates/nono/src/diagnostic/codes.rs
    - crates/nono/src/machine_policy.rs
    - bindings/c/src/lib.rs

key-decisions:
  - "D-22 satisfied: LayerAttestationFailed is its own diagnostic_code() arm, not folded into ConfigurationError — the operator learns which layer failed, not that 'something' failed"
  - "D-26 satisfied: RequiredLayersPolicy is a pub struct (not pub(crate)) so Plan 08 can consume it cross-crate without tripping rustc's private_interfaces lint under -D warnings; excluded from is_unconfigured() per the same invariant as telemetry"
  - "RequiredLayersPolicy's read lifecycle is explicitly decided as degrade-not-abort (telemetry precedent, D-14), not abort-on-unreadable (egress precedent, D-07) — documented inline so a future reader does not have to re-derive the reasoning from the two competing precedents in this file"
  - "No registry sub-key reader for RequiredLayers is implemented in this plan — parse_policy() defaults required_layers to RequiredLayersPolicy::default() with an inline comment; the actual HKLM\\SOFTWARE\\Policies\\nono\\RequiredLayers reader is left for whichever downstream plan (Plan 08's consumer) needs it, since this plan's scope is the type + field only per its own acceptance criteria"

requirements-completed: [CINT-02]

# Metrics
duration: 15min
completed: 2026-08-10
---

# Phase 117 Plan 02: Library-Side Error + Diagnostic Code + Machine-Policy Surfaces for Self-Attestation Summary

**Two additive, policy-free `crates/nono` surfaces CINT-02's self-attestation needs before any gate-insertion work can compile against them: `NonoError::LayerAttestationFailed` + its own `NonoDiagnosticCode` (D-22), and `MachineEgressPolicy.required_layers: RequiredLayersPolicy` (D-26) with an explicit degrade-not-abort read-lifecycle decision.**

## Performance

- **Duration:** ~15 min (3 task commits, RED→GREEN TDD cycle for Task 1)
- **Tasks:** 2 completed
- **Files modified:** 4 (0 created, 4 modified — including 1 out-of-plan FFI fix)

## Accomplishments

- Added `NonoDiagnosticCode::LayerAttestationFailed` to the `#[non_exhaustive]` enum (additive, no downstream match breaks)
- Added `NonoError::LayerAttestationFailed { layer: String, reason: String }`, modeled on `LabelApplyFailed`'s shape but with plain `String` fields (no `PathBuf`/`hresult` — this error covers OS-probe failures, not path/ACE mutation failures)
- Mapped it to its **own** `diagnostic_code()` match arm — deliberately not folded into the existing `ConfigurationError` arm, per D-22's requirement that the operator learn which layer failed
- Verified via TDD: a RED test asserting the mapping failed to compile (variant didn't exist), then GREEN implementation made it pass, confirmed by `cargo test -p nono-sandbox --lib layer_attestation_failed_maps_to_own_diagnostic_code`
- Added `RequiredLayersPolicy { required: Vec<String> }` as a `pub` struct (matching `TelemetryConfig`'s cross-crate visibility, avoiding the `private_interfaces` lint) and wired `MachineEgressPolicy.required_layers: RequiredLayersPolicy` with `#[serde(default)]`
- Documented, inline, the D-26 abort-vs-degrade tension and its resolution: `required_layers` follows `TelemetryConfig`'s degrade-not-abort lifecycle (D-14), not the egress fields' abort-on-unreadable one (D-07) — a malformed registry value falls back to empty rather than bricking every confined launch fleet-wide
- Added the `MUST NOT be counted in [Self::is_unconfigured]` invariant comment on `required_layers`, matching `telemetry`'s existing comment verbatim in intent, and a new test (`is_unconfigured_ignores_required_layers_field`) proving it
- Updated every exhaustive `MachineEgressPolicy { ... }` struct literal that didn't already use `..Default::default()` (the production `parse_policy()` constructor plus 3 test literals) so the new field didn't silently go unexercised in serde round-trip / `is_unconfigured` coverage

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: failing test for `LayerAttestationFailed` diagnostic code** — `441ab307` (test)
2. **Task 1 GREEN: `NonoError::LayerAttestationFailed` + `NonoDiagnosticCode::LayerAttestationFailed`** — `bf2a3ba7` (feat)
3. **Task 2: `RequiredLayersPolicy` machine-policy field (D-26) + FFI exhaustive-match fix** — `5ef4f298` (feat)

## Files Created/Modified

- `crates/nono/src/error.rs` — new `LayerAttestationFailed { layer, reason }` variant + its own `diagnostic_code()` arm + a new test in `diagnostic_tests`
- `crates/nono/src/diagnostic/codes.rs` — new `LayerAttestationFailed` variant on `NonoDiagnosticCode` (additive)
- `crates/nono/src/machine_policy.rs` — new `RequiredLayersPolicy` struct + `MachineEgressPolicy.required_layers` field + updated 4 exhaustive struct literals + 1 new test
- `bindings/c/src/lib.rs` — new `ErrSandboxInit` match arm for `LayerAttestationFailed` in `map_error()` (deviation, see below)

## Decisions Made

- **D-22 mapping choice:** `LayerAttestationFailed` gets its own `diagnostic_code()` arm rather than joining the `ConfigurationError` bucket that `LabelApplyFailed`/`DaclApplyFailed`/`PolicyLoadFailed`/telemetry variants share — the plan's explicit instruction, and the only choice consistent with D-22's "the operator learns which layer failed" requirement.
- **D-26 visibility (Blocker-2 precedent):** `RequiredLayersPolicy` is `pub`, matching `TelemetryConfig` exactly, because `MachineEgressPolicy` is a `pub` struct and a `pub` field of a `pub(crate)` type trips rustc's `private_interfaces` lint under `-D warnings`. Confirmed by `cargo clippy -p nono-sandbox -- -D warnings -D clippy::unwrap_used` passing clean.
- **D-26 read lifecycle:** Explicitly decided and documented as degrade-not-abort (telemetry precedent), not abort-on-unreadable (egress precedent) — stated inline on `RequiredLayersPolicy`'s doc comment per the plan's own instruction to "state this reasoning inline so a future reader does not have to re-derive it."
- **No registry reader implemented here:** `parse_policy()` sets `required_layers` to `RequiredLayersPolicy::default()` with an inline comment noting no `RequiredLayers` sub-key reader exists yet. This plan's acceptance criteria are structural only (struct/field/invariant-comment greps, build/clippy/test pass) — actual HKLM sub-key parsing is left for the downstream plan that consumes it.
- **FFI error-code mapping:** `LayerAttestationFailed` maps to `NonoErrorCode::ErrSandboxInit`, the same code `LabelApplyFailed`/`DaclApplyFailed`/`BrokerNotFound` already use — structurally the same class (a fail-closed sandbox-setup/attestation failure), consistent with those variants' documented rationale in the same match.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Added `bindings/c/src/lib.rs` match arm for the new `NonoError` variant**
- **Found during:** Task 2, running `cargo check --workspace --all-targets` per the repo's CLAUDE.md-documented hazard ("a new enum variant can break FFI exhaustive matches")
- **Issue:** `bindings/c/src/lib.rs`'s `map_error()` function pattern-matches every `NonoError` variant exhaustively (no wildcard, by design — the doc comment states this is deliberate so new variants get flagged). Adding `LayerAttestationFailed` in Task 1 broke this build with `E0004: non-exhaustive patterns`.
- **Fix:** Added `nono::NonoError::LayerAttestationFailed { .. } => NonoErrorCode::ErrSandboxInit,` alongside the existing `LabelApplyFailed`/`DaclApplyFailed`/`BrokerNotFound` arms it structurally matches, with an inline comment explaining the choice. No new `NonoErrorCode` variant was added (ABI-stable, matching this file's own established convention for prior additions like `UnsupportedKernelFeature`).
- **Files modified:** `bindings/c/src/lib.rs`
- **Verification:** `cargo check --workspace --all-targets` exits 0; `cargo clippy -p nono-ffi -- -D warnings -D clippy::unwrap_used` exits 0.
- **Committed in:** `5ef4f298` (Task 2 commit — bundled since it's a direct compile-blocking consequence of Task 1's variant addition, only surfaced once Task 2's workspace-wide check ran)

---

**Total deviations:** 1 auto-fixed (1 Rule 3 blocking-issue fix)
**Impact on plan:** Zero scope creep — the fix is the exact one-line-per-variant maintenance the FFI file's own doc comment anticipates, and it was explicitly flagged as a risk in this plan's `<repo_specific_hard_constraints>`.

## Issues Encountered

None beyond the deviation above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 08 (CLI-side attestation decision logic) can now construct/match `NonoError::LayerAttestationFailed` and consume `MachineEgressPolicy.required_layers: RequiredLayersPolicy` cross-crate without any `private_interfaces` friction.
- Plan 08 owns: parsing `RequiredLayersPolicy.required` strings against its own `LayerId::ALL` (fail-closed-rejecting unrecognized names, per D-02's CLI-owns-registry boundary), the actual `RequiredLayers` HKLM sub-key reader (this plan deliberately left it as `RequiredLayersPolicy::default()`), and the union-with-CLI-flag "can only tighten" enforcement (D-26).
- `cargo build -p nono-sandbox`, `cargo clippy -p nono-sandbox -- -D warnings -D clippy::unwrap_used`, `cargo clippy -p nono-ffi -- -D warnings -D clippy::unwrap_used`, `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo test -p nono-sandbox --lib` (828/828) all pass.
- No blockers for downstream plans.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*
