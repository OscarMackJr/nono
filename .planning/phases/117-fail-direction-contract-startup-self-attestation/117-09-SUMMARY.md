---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 09
subsystem: security
tags: [telemetry, windows, attestation, hmac, tracing-subscriber, rustfmt, clippy]

# Dependency graph
requires:
  - phase: 117-01
    provides: "LayerId, EntryPath, ArmExpectancy, ContractOutcome, ProbeKind layer registry"
  - phase: 117-05
    provides: "LayerAttestationStatus four-state vocabulary, ProcessHandle, raw OS probes"
  - phase: 117-08
    provides: "attest_and_decide(), AttestationDecision, AttestationInput — the decision this plan's channels render"
provides:
  - "SecurityEventType::LayerAttestationDowngraded + EVENT_ID_LAYER_ATTESTATION_DOWNGRADED (10011)"
  - "SecurityEvent.downgraded_layers field (specific LayerId names, D-28 justified)"
  - "SecurityEventLayer::emit_attestation_event() — un-deduplicated, once-per-spawn HMAC-chained audit record"
  - "output::print_attestation_downgrade_banner() — coarse, non-silenceable, per-session-deduplicated console channel"
  - "output::attestation_downgrade_color() — companion color helper"
affects: [117-10, 117-11, 117-12, 118]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-27 three-channel downgrade surfacing: banner (coarse) + NonoDiagnosticCode (117-02, already wired) + HMAC-chained SecurityEvent (specific)"
    - "Per-session content-addressed dedup marker (DefaultHasher over sorted dedup_key) under sessions_dir()/<id>/attestation-downgrade/<hash>, mirroring ensure_audit_session_dir's create_dir_all + best-effort-never-fatal shape"
    - "cfg(target_os = \"windows\") gating for CLI-side render functions with no non-Windows caller, mirroring format_scope_status's existing Linux-only gating shape in the same file"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/telemetry/event.rs
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/telemetry/windows.rs
    - crates/nono-cli/src/output.rs

key-decisions:
  - "D-28 landed literally as a doc-comment-level constraint: SecurityEvent.downgraded_layers carries specific LayerId names (supervisor-only ETW/AppLog channel) while the banner is mechanically verified layer-name-free (grep -c \"LayerId\" output.rs == 0, including doc comments — required rewording the banner's own doc comment to avoid the literal substring)."
  - "print_attestation_downgrade_banner / attestation_downgrade_marker_path / attestation_downgrade_color gated #[cfg(target_os = \"windows\")] — not in the plan's literal action text, but required to pass both local cross-target clippy gates (unused-fn dead_code on linux-gnu/apple-darwin, since Plans 10/11's Windows-only callers haven't landed yet) and consistent with the plan's own interfaces note that format_scope_status (this file's closest precedent) is itself #[cfg(target_os = \"linux\")]-only."
  - "emit_attestation_event is never deduplicated (once per attestation decision / per spawn) — the HMAC-chained audit record intentionally keeps a complete forensics history, distinct from the banner's per-session dedup; documented explicitly in both the method's doc comment and this summary since it's easy to conflate the two dedup postures."

patterns-established:
  - "New SecurityEventType variants require updates at 4 sites: EVENT_ID_* const, enum variant, event_id_for match arm, severity_for match arm (all exhaustive, no wildcard) — plus every existing SecurityEvent { .. } struct-literal call site gains the new field."

requirements-completed: [CINT-02]

# Metrics
duration: ~55min
completed: 2026-08-09
---

# Phase 117 Plan 09: D-27 Downgrade Channels (Telemetry Event + Console Banner) Summary

**Lands the two D-27 channels Plans 10/11 will call from the real gate points: `SecurityEventLayer::emit_attestation_event()` (specific layer names, HMAC-chained, un-deduplicated) and `output::print_attestation_downgrade_banner()` (coarse, non-silenceable, per-session-deduplicated) — the third channel (typed `NonoDiagnosticCode`) was already wired in Plan 02.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 2
- **Files modified:** 4 (`telemetry/event.rs`, `telemetry/mod.rs`, `telemetry/windows.rs`, `output.rs`)

## Accomplishments
- `SecurityEventType::LayerAttestationDowngraded` (EventID 10011) + `SecurityEvent.downgraded_layers: Option<String>` field, carrying comma-separated `LayerId` `Debug`-names on the supervisor-only ETW/AppLog channel (D-28), with the one unverified child-readability assumption named as a concrete Plan 12 Task 2 follow-up (`wevtutil gl Application` SDDL inspection) directly in the field's doc comment.
- `SecurityEventLayer::emit_attestation_event(&[&str]) -> Result<String, &'static str>`, modeled on `emit_override_event`'s direct-call shape: advances the HMAC chain unconditionally, only emits to ETW/AppLog when `config.enabled` (D-14 degrade-not-abort), `#[must_use = "AUD-04: ..."]`.
- `output::print_attestation_downgrade_banner(downgraded_count: usize, session_id: Option<&str>, dedup_key: &str)` — no `silent` parameter (Warning-8 fix: a downgraded claim is exempt from `--silent`), per-session content-addressed dedup via `std::hash::DefaultHasher` over an opaque caller-supplied `dedup_key`, marker lives under `sessions_dir()/<session_id>/attestation-downgrade/<hash>` (mirrors `ensure_audit_session_dir`'s best-effort, never-fatal I/O discipline). Mechanically verified layer-name-free.
- Companion `output::attestation_downgrade_color(downgraded_count, &theme::Theme) -> Rgb`, mirroring `scope_status_color`'s shape.
- Both local cross-target clippy gates (linux-gnu via `cross`, apple-darwin via `cargo-zigbuild`) run clean against the full workspace after the changes.

## Task Commits

1. **Task 1: New telemetry event type + downgraded_layers field + emission method** - `34b711a6` (feat)
2. **Task 2: Coarse, arm-independent, non-silenceable, per-session-deduplicated downgrade banner** - `5af3cec3` (feat)

**Plan metadata:** (this commit, pending) `docs(117-09): complete plan`

## Files Created/Modified
- `crates/nono-cli/src/telemetry/event.rs` — `EVENT_ID_LAYER_ATTESTATION_DOWNGRADED`, `SecurityEventType::LayerAttestationDowngraded`, `SecurityEvent.downgraded_layers`, 3 new tests.
- `crates/nono-cli/src/telemetry/mod.rs` — `SecurityEventLayer::emit_attestation_event()`, exhaustive `severity_for` arm, `downgraded_layers: None` added to the two pre-existing `SecurityEvent { .. }` construction sites, 3 new tests, one `cargo fmt` line-wrap of the new method's signature.
- `crates/nono-cli/src/telemetry/windows.rs` — `downgraded_layers: None` added to a test's `SecurityEvent { .. }` construction (Rule 1: compile-blocking consequence of the struct-field addition).
- `crates/nono-cli/src/output.rs` — `print_attestation_downgrade_banner`, `attestation_downgrade_marker_path` (private), `attestation_downgrade_color`, all `#[cfg(target_os = "windows")]`; `use crate::state_paths;` import (also windows-gated).

## Decisions Made
- **D-28 enforcement is mechanical, not just prose.** The plan's own acceptance criterion (`grep -c "LayerId" output.rs` returns 0) caught a doc-comment self-reference during implementation — my first draft explained the design by naming `LayerId` in a doc comment, which is itself a violation of the "no layer identifier anywhere in this function's output" intent even though doc comments aren't runtime output. Reworded to describe the caller's type without naming it literally. This is worth flagging because it shows the acceptance criterion is stricter than "no `LayerId` in the printed string" — it's "no `LayerId` substring in the file at all," which is a more conservative, easier-to-verify bar.
- **`emit_attestation_event` is intentionally never deduplicated.** Unlike the banner (per-session dedup), every attestation decision gets its own HMAC-chained audit record — this is the forensics channel, and Plans 10/11 must call it once per spawn regardless of whether the banner printed or was suppressed as a repeat.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[cfg(target_os = "windows")]` gating on Task 2's three new items, not specified in the plan's literal action text**
- **Found during:** Task 2 verification (cross-target clippy gate, per CLAUDE.md's cross-target clippy MUST for files containing `#[cfg(target_os = "linux")]` blocks — `output.rs` qualifies)
- **Issue:** `cross clippy --target x86_64-unknown-linux-gnu` failed with `error: function 'print_attestation_downgrade_banner' is never used` (and the same for the marker-path helper and the color helper) — these functions have no caller until Plans 10/11 land their Windows-only gate points, so on non-Windows cross-target builds (where the file-level `#![cfg_attr(target_os = "windows", allow(dead_code))]` doesn't apply) they trip `-D warnings`.
- **Fix:** Gated all three new items `#[cfg(target_os = "windows")]`. This is consistent with the plan's own `<interfaces>` note that the closest shape precedent, `format_scope_status`, is itself `#[cfg(target_os = "linux")]`-only — i.e., the plan already implied a platform-gated Windows-side analog rather than a cross-platform function, it just didn't spell out the exact attribute. Also gated the new `use crate::state_paths;` import the same way (otherwise unused-import on non-Windows).
- **Files modified:** `crates/nono-cli/src/output.rs`
- **Verification:** Both `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (SDKROOT unset) exit clean; Windows-host `cargo build`/`clippy` for both `nono` and `nono-agentd` bins also clean.
- **Committed in:** `5af3cec3` (Task 2 commit)

**2. [Rule 1 - Bug] `SecurityEvent { .. }` struct-literal call sites needed `downgraded_layers: None` added**
- **Found during:** Task 1 (adding the new field to `SecurityEvent`)
- **Issue:** Adding a non-`Option`-defaulted required field to a struct without `..Default::default()` breaks every existing construction site at compile time.
- **Fix:** Added `downgraded_layers: None` to the two production construction sites in `telemetry/mod.rs` (`emit_override_event`, `on_event`) and one test construction site in `telemetry/windows.rs`.
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`, `crates/nono-cli/src/telemetry/windows.rs`
- **Verification:** `cargo build -p nono-sandbox-cli --bin nono` succeeds; existing telemetry test suite (40 pre-existing tests) still passes unchanged.
- **Committed in:** `34b711a6` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking cross-target gate closure, 1 compile-blocking struct-field consequence)
**Impact on plan:** Both auto-fixes were required for the plan's own verification gate (cross-target clippy MUST, and a plain `cargo build`) to pass. No scope creep — no new call sites were added, no Plans 10/11 work was pulled forward.

## Issues Encountered
- The plan's acceptance criterion for Task 2's function signature (`grep -c "fn print_attestation_downgrade_banner(downgraded_count: usize, session_id: Option<&str>, dedup_key: &str)"` equals 1) is written as a single logical string that word-wraps across two lines in the plan markdown. After `cargo fmt`, the actual 3-parameter signature is formatted one-parameter-per-line (a 111-character single-line signature exceeds rustfmt's 100-column default and this repo has no `rustfmt.toml` override), so the literal single-line grep pattern does not match. Verified the equivalent semantic intent instead: `awk` confirms the exact 3 parameters (`downgraded_count: usize`, `session_id: Option<&str>`, `dedup_key: &str`) with no `silent` parameter anywhere in the signature block. `grep -c "DefaultHasher"` (≥1) and `grep -c "LayerId"` (== 0) acceptance criteria both pass exactly as specified.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Both D-27 channels Plans 10/11 need now exist with concrete, compiling signatures: `SecurityEventLayer::emit_attestation_event(&[&str])` and `output::print_attestation_downgrade_banner(usize, Option<&str>, &str)` (plus `attestation_downgrade_color`).
- D-28's child-readability call is explicit and documented in `SecurityEvent.downgraded_layers`'s doc comment, naming Plan 12 Task 2's `wevtutil gl Application` verification as the concrete follow-up (not a bare mention).
- The banner cannot be silenced (Warning-8 closed) and dedupes per session, not per spawn (Item-2 closed), with the first occurrence always printing.
- Plans 10/11 gate points (`launch.rs` direct spawn, `agent_daemon/launch.rs`, and the broker arm via `required_layers_for_broker`) can now call both channels directly — neither function currently has a caller (by design; interface-first ordering), so both are `#[cfg(target_os = "windows")]`-gated and will start compiling into the live call graph once Plans 10/11 land in this same wave.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-09*

## Self-Check: PASSED

All 4 modified source files and the SUMMARY.md itself confirmed present on disk;
all 3 commit hashes (`34b711a6`, `5af3cec3`, `9403a03a`) confirmed present in `git log --oneline --all`.
