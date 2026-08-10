---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 16
subsystem: security
tags: [windows, attestation, tracing, telemetry, firewall, wfp, gate-tests]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "apply_startup_attestation_gate, layer_registry AppliedLayers, NetworkEnforcementGuard fixtures (117-13..15)"
provides:
  - "Unconditional tracing::warn! on ProceedDowngraded's success path carrying a structured downgraded_layers field"
  - "FirewallRulesEgress gate-level force-unavailable automated test (both directions)"
affects: [117-18 (verify-gate meta-test discovery), 117-VERIFICATION.md NR3-04/SC3 findings]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Custom capturing tracing::Subscriber (Arc-wrapped, Dispatch::from) for asserting structured field presence in unit tests without a global subscriber"
    - "Gate-level force-unavailable test pattern (spawn real suspended child + real Job Object + fabricated-but-real guard value, drive apply_startup_attestation_gate, assert Err(LayerAttestationFailed))"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs

key-decisions:
  - "Distinguished the new structured downgraded_layers field from the pre-existing warns' text-embedded 'downgraded_layers=...' by asserting on FIELD NAME equality, not message substring — this makes the counterfactual (commenting out the new warn) genuinely fail rather than false-passing on the old warns' text"
  - "Used Arc<CapturingSubscriber> + tracing::dispatcher::with_default (not tracing::subscriber::with_default, which would move/drop the subscriber) so captured events remain inspectable after the closure returns"
  - "FirewallRulesEgress test sets wfp_egress_filters = None (NotApplicable) since the FirewallRules backend, not WFP, was selected — mirrors the WFP analog test's inverse composition"

patterns-established:
  - "Gate-level per-row force-unavailable tests should assert BOTH the abort direction and a non-vacuous companion where the same fixture reaches Ok(()), proving the abort case is reachable and not tautological"

requirements-completed: [CINT-02, CINT-03]

# Metrics
duration: ~5min
completed: 2026-08-10
---

# Phase 117 Plan 16: D-27 Downgrade Banner Truthfulness + FirewallRulesEgress Gate Test Summary

**Made the D-27 downgrade banner's "see diagnostic output for details" claim true on the success path via an unconditional `tracing::warn!`, and added the missing `FirewallRulesEgress` gate-level force-unavailable test using pre-existing fixtures.**

## Performance

- **Duration:** ~5 min (commit timestamps 18:31:35 → 18:32:53, plus verification runs)
- **Tasks:** 2 completed
- **Files modified:** 1 (`crates/nono-cli/src/exec_strategy_windows/launch.rs`)

## Accomplishments
- NR3-04 closed: `apply_startup_attestation_gate`'s `ProceedDowngraded` arm now emits an unconditional `tracing::warn!` (mirroring `agent_daemon/launch.rs:965-972`) carrying `downgraded_layers`/`downgraded_count` as structured fields, positioned after the audit-emission match and before `print_attestation_downgrade_banner`. Previously the banner's claimed channel was only populated on FAILURE to emit the audit record (the `None`/`Err` arms) — on the common success path, nothing wrote to it.
- SC3 closed: `FirewallRulesEgress` now has a gate-level automated test (`firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate`) mirroring the pre-existing WFP analog, driving the real registry + gate with `firewall_rules_guard_with_rule_count(1)` (asserting `Err(LayerAttestationFailed { layer: "FirewallRulesEgress", reason: "Unconfirmed" })`) and a `rule_count(2)` companion (asserting `Ok(())`), using zero new production seams.

## Task Commits

Each task was committed atomically:

1. **Task 1: Unconditional tracing::warn! on the ProceedDowngraded success path (NR3-04)** - `15a9f287` (fix)
2. **Task 2: FirewallRulesEgress gate-level forced-unavailable test (SC3)** - `681633d6` (test)

**Plan metadata:** (this commit)

## Files Created/Modified
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - Task 1: added unconditional `tracing::warn!` in `ProceedDowngraded` arm + `CapturingSubscriber`/`FieldCapture` test harness + `proceed_downgraded_success_path_logs_downgraded_layers_field_unconditionally` test. Task 2: added `firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate` test.

## Decisions Made
- The new test asserts on the presence of a *structured* `downgraded_layers` field (via a custom capturing `tracing::Subscriber`), not a substring match on rendered text — this is deliberately stricter than a naive grep-style check, because the pre-existing conditional warns already contain the literal text `downgraded_layers=...` inside their `message` field. A substring assertion would have false-passed even with the fix reverted. Verified this is not vacuous: with the new `tracing::warn!` line temporarily removed, the test failed and the captured-events debug output showed only the old warn's message-embedded text, confirming field-name equality is the correct predicate.
- `tracing::dispatcher::with_default(&Dispatch::from(Arc::clone(&subscriber)), || { ... })` was used instead of the higher-level `tracing::subscriber::with_default(subscriber, || {...})`, because the latter takes ownership of the subscriber for the closure's lifetime and drops it afterward — this test needs to inspect captured events after the gate call returns, so an `Arc` handle is kept outside the closure.
- Task 2's test sets `applied.wfp_egress_filters = None` — the `FirewallRules` backend was selected for this synthetic launch, not WFP, so that row is correctly `NotApplicable` and must never be a reason to abort. This mirrors the WFP analog test's inverse (which leaves `firewall_rules_egress` untouched at its `fully_applied_layers()` default of `None`).

## Deviations from Plan

None - plan executed exactly as written. Both tasks matched their `<action>`/`<behavior>` specs; no Rule 1-4 triggers encountered.

## Issues Encountered

None. The counterfactual verification (commenting out the new `tracing::warn!`, confirming the new test fails, then restoring it) was performed manually as required by Task 1's acceptance criteria, using a temporary in-place `Edit`/revert (not a git stash — per this project's `git stash` prohibition) so no working-tree state was left uncommitted at any point. Observed failure text on the counterfactual:

```
panicked at crates\nono-cli\src\exec_strategy_windows\launch.rs:3728:9:
the ProceedDowngraded success path must log a STRUCTURED `downgraded_layers` field unconditionally
(not only inside the audit-emission-failure arms), so the D-27 banner's "see diagnostic output for
details" claim is truthful; captured events: [[("message", "attestation downgrade audit emission
unavailable (AUD-04: SecurityEventLayer not initialized) — proceeding per AUD-04's non-fatal
contract; downgraded_layers=MandatoryIntegrityLabel")]]
```

This confirms the pre-existing conditional warn's `downgraded_layers=...` text lives entirely inside the `message` field (not a separate structured field), so the test is a genuine, non-vacuous check of the fix.

## Verification Performed

- `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 launch::attestation_gate_tests::` — 12 passed, 0 failed (both new tests plus all 10 pre-existing tests in the module, confirming no regression).
- `cargo build --workspace --all-targets` — clean.
- `cargo clippy -p nono-sandbox-cli --all-targets --features layer-fault-injection -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo clippy --workspace --all-targets --features layer-fault-injection -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo fmt --all -- --check` — clean (one rustfmt pass applied before commit to two lines rustfmt preferred differently wrapped).
- `grep -n "downgraded_layers = %dedup_key" crates/nono-cli/src/exec_strategy_windows/launch.rs` confirms the new call is textually after the closing `}` of the `match crate::telemetry::SECURITY_LAYER.get()` block.
- `git diff --stat` for Task 2's commit shows only test-module additions (108 insertions, 0 deletions, 0 non-test files) — no production code changed.

**Cross-target clippy:** DOES NOT APPLY. `launch.rs` is under `crates/nono-cli/src/exec_strategy_windows/`, which is exclusively `#[cfg(windows)]`/`#[cfg(all(test, target_os = "windows"))]` surface (verified: `grep -n '#\[cfg(target_os = "linux"\|"macos"' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns no matches), and is not under `bindings/c/src/` or the Unix `exec_strategy/` directory. Per CLAUDE.md's cross-target-verify-checklist, ran the Windows-host `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (with `--features layer-fault-injection` for this plan's test-gated code) plus `cargo fmt --all -- --check` as the applicable minimum, both clean.

## Known Stubs

None.

## Threat Flags

None — this plan closes a repudiation gap (T-117-16-01) in an existing diagnostic channel; it does not introduce new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.

## Next Phase Readiness
- NR3-04 and SC3 are both closed; ready for 117-17/117-18 (verify-gate meta-test discovery, which is expected to register `firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate` as this row's automated coverage per the plan's stated downstream link).
- No blockers.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*
