---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 17
subsystem: security
tags: [windows, attestation, daemon, dead-code, telemetry, cross-binary]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "daemon_attest_and_decide / DaemonAttestationDecision (Plan 08/10/11), SecurityEventLayer::emit_attestation_event (Plan 10/11/16)"
provides:
  - "Two-state DaemonAttestationDecision (Proceed/Abort) with a discovery-based conformance test"
  - "exec_strategy_windows::attestation_downgrade_event — SecurityEventLayer::emit_attestation_event relocated to the one binary that actually calls it"
affects: [117-18 (verify-gate meta-test discovery), 117-19, CINT-02 requirement closure]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Discovery-based enum-shape conformance test: parses variant names straight out of the file's own source text via include_str!, indentation-filtered to exclude struct-variant field lines"
    - "Per-[[bin]]-target dead code: a shared #[path]-included file compiled into two independent binary crates can have a pub method with real production callers in only one of them — resolved by moving the method (as a second impl block) into a module tree that is structurally excluded from the other binary, not by #[allow(dead_code)]/#[expect(dead_code)]/cfg tricks (all verified not to work for this shape)"

key-files:
  created:
    - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs
  modified:
    - crates/nono-cli/src/agent_daemon/launch.rs
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs

key-decisions:
  - "Deletion path (not reachability path): DaemonAttestationDecision::ProceedDowngraded was removed rather than wired up, because every layer daemon_attest_and_decide models is fail-closed end-to-end in the daemon's own guards today (DaemonDaclGuard's three-pass apply has no partial-success return; wfp_filter_add's Err path terminates before the gate) — there is no partial-coverage state for a downgrade arm to represent"
  - "Discovery test filters by exact indentation (8 spaces = variant level), not just 'not a doc comment' — an earlier draft mis-parsed Abort's struct-variant field lines (`layer: &'static str,` at 12-space indent) as extra variants; caught by actually running the test against real code, not just the counterfactual"
  - "emit_attestation_event moved to exec_strategy_windows::attestation_downgrade_event (new file) rather than silenced with an attribute — see Deviations for the full investigation of why #[cfg(target_os=\"windows\")], a Cargo feature, and #[expect(dead_code)] were each verified NOT to solve this, before choosing the structural fix"
  - "SecurityEventLayerInner and its chain/session_id/config fields widened from private to pub(crate) (inner field likewise) — safe because nono-cli has no [lib] target, so pub(crate) never crosses an external API boundary; scoped to exactly the fields the relocated method needs (session_salt stays private)"

patterns-established:
  - "When a shared #[path]-included file's method has asymmetric real callers across sibling [[bin]] targets in the same package, move the method (not the whole file) into a module tree that only one binary includes — verified empirically that no attribute-level or Cargo-feature-level fix exists for this shape in a single-package, no-[lib] workspace member"

requirements-completed: [CINT-02]

# Metrics
duration: ~90min
completed: 2026-08-10
---

# Phase 117 Plan 17: Delete Dead DaemonAttestationDecision::ProceedDowngraded (NR3-05) Summary

**Shrank `DaemonAttestationDecision` to a genuine two-state enum (`Proceed`/`Abort`), removed the ~60-line dead audit-emission arm that could never fire, added a discovery-based conformance test, and — as a directly-caused, fully-resolved fallout fix — relocated `SecurityEventLayer::emit_attestation_event` out of the shared `telemetry/mod.rs` into a `nono`-binary-only module so it stopped being genuinely dead code inside `nono-agentd`'s independent compilation.**

## Performance

- **Duration:** ~90 min (most of it: an exhaustive, evidence-based investigation of why the deletion's `dead_code` fallout could NOT be solved with an attribute, before committing to the structural fix — see Deviations)
- **Tasks:** 1 planned task, completed; 1 additional in-scope fallout fix (Rule 1/3)
- **Files modified:** 3; 1 new file created

## Accomplishments

- NR3-05 closed: `DaemonAttestationDecision` is a genuine two-state enum. `daemon_attest_and_decide` never constructed `ProceedDowngraded` (dead code that looked like a supported state — full audit-event emission, dedup key, Event Log warning, all fully implemented, zero reachability). Deleted the variant, its doc, and the ~60-line match arm in `launch_agent`. Corrected the module doc (previously claimed a three-state mirror of the CLI-side `AttestationDecision`) to state the daemon's design is deliberately narrower and WHY.
- Added `daemon_attestation_decision_is_deliberately_two_state`, a discovery-based test that parses `DaemonAttestationDecision`'s variant names straight out of `launch.rs`'s own source text (via `include_str!`, following the file's pre-existing `include_str!`-based wiring-check idiom) and asserts the list is exactly `["Proceed", "Abort"]`. Verified genuinely non-vacuous: temporarily re-added a `DummyCounterfactual` variant (and a matching wildcard arm, since Rust's own exhaustiveness check ALSO catches an unmatched variant — an even stronger signal than the test) — the test failed naming the exact mismatch (`got ["Proceed", "Abort", "DummyCounterfactual"]`), then both were reverted.
- All 4 pre-existing tests exercising `daemon_attest_and_decide` directly (including `real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`, which spawns a real AppContainer-confined suspended child) pass unchanged — confirmed a pure regression-free narrowing, not a behavior change.

## Task Commits

1. **Task 1: Delete dead ProceedDowngraded variant/arm + correct module doc + add discovery test, plus the directly-caused emit_attestation_event relocation fallout fix** — `f5f9587f` (fix)

Both the plan's single task and its in-scope fallout fix landed in one commit, since the fallout fix was strictly necessary for Task 1's own acceptance criteria (clean `-D warnings` clippy on the `nono-agentd` binary) to hold — splitting them would have left an intermediate commit with a known-broken build.

## Files Created/Modified

- `crates/nono-cli/src/agent_daemon/launch.rs` — deleted `ProceedDowngraded` variant + its ~60-line match arm in `launch_agent`; corrected the enum's doc comment and the step-6.7 module-level doc; added `daemon_attestation_decision_is_deliberately_two_state`; fixed a stale doc comment on `real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable` that still described a "downgraded claim" outcome the code no longer produces.
- `crates/nono-cli/src/telemetry/mod.rs` — removed `SecurityEventLayer::emit_attestation_event` and its 3 tests (relocated, not deleted — see below); widened `SecurityEventLayerInner` (struct + `chain`/`session_id`/`config` fields) and `SecurityEventLayer::inner` from private to `pub(crate)`, scoped to exactly what the relocated method needs.
- `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` (new) — `impl SecurityEventLayer { pub fn emit_attestation_event }` (unchanged behavior/doc, just relocated) plus its 3 tests, in a module tree `nono-agentd.rs` never `#[path]`-includes.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — added `mod attestation_downgrade_event;` declaration.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1/3 — directly-caused blocking regression] `emit_attestation_event` became genuinely dead code inside `nono-agentd`'s own compilation after deleting its only daemon-side call site**

- **Found during:** Task 1's own acceptance-criteria verification (`cargo clippy -p nono-sandbox-cli --bin nono-agentd -- -D warnings -D clippy::unwrap_used`), immediately after deleting the `ProceedDowngraded` match arm.
- **Root cause:** `nono-cli` has no `[lib]` target — only `[[bin]]` targets (`nono` via `src/main.rs`, `nono-agentd` via `src/bin/nono-agentd.rs`). `telemetry/mod.rs` is `#[path]`-included wholesale into BOTH binaries as two structurally independent compilations of the same source text. `emit_attestation_event` had exactly two real production callers before this plan: `exec_strategy_windows/launch.rs`'s `apply_startup_attestation_gate` (`nono`-only) and `agent_daemon/launch.rs`'s step 6.7 gate (`nono-agentd`-only, the arm this plan deletes). After deletion, `nono-agentd`'s own `#[path]`-copy of `telemetry/mod.rs` had zero non-test callers for that one method — a `-D warnings`-fatal `dead_code` error specific to that one binary's build (the `nono` binary's copy was unaffected, since its real caller still exists).
- **Investigation before choosing a fix (each verified empirically, not assumed):**
  - `#[cfg(target_os = "windows")]` (the codebase's own existing precedent for a similar "genuinely dead on some targets" case, at this exact function) does NOT help — both binaries target Windows; there is no target_os distinction between them.
  - A Cargo feature flag does NOT help — features are resolved per-package for a given build invocation, not per-`[[bin]]`-target; both binaries are typically built together and would receive the same feature set regardless.
  - `#[expect(dead_code)]` (the compiler's own suggested alternative to `#[allow(dead_code)]`) was tried and empirically verified NOT to work: applying it made `nono-agentd`'s build clean, but broke `nono`'s build instead (`error: this lint expectation is unfulfilled` — because the SAME attributed item is genuinely NOT dead in that binary's copy). Confirmed via two direct `cargo clippy --bin nono-agentd` / `cargo clippy --bin nono` runs with the attribute applied.
  - `#[allow(dead_code)]` was not attempted — CLAUDE.md explicitly forbids it as a lazy fix ("If code is unused, either remove it or write tests that use it"), and the existing precedent comment on this exact function already states this reasoning.
- **Fix:** Moved `emit_attestation_event` (unchanged doc/behavior) out of `telemetry/mod.rs`'s shared `impl SecurityEventLayer` block into a new file, `exec_strategy_windows/attestation_downgrade_event.rs`, declared as `mod attestation_downgrade_event;` inside `exec_strategy_windows/mod.rs` — a module tree confirmed (via `nono-agentd.rs`'s own `#[path]`-include list: only `agent_daemon/mod.rs`, `telemetry/mod.rs`, `agent_daemon/telemetry_init.rs`) to be structurally excluded from the daemon binary. This required widening `SecurityEventLayerInner`/`SecurityEventLayer::inner` and the three fields the method reads (`chain`, `session_id`, `config`) from private to `pub(crate)` — safe because `nono-cli` has no `[lib]` target, so `pub(crate)` never crosses an external API boundary; it only affects organization within this one package's own binaries. `session_salt` (unused by the relocated method) stayed private. The call site in `exec_strategy_windows/launch.rs` (`security_layer.emit_attestation_event(&downgraded_refs)`) needed no edit — the method signature and doc are unchanged, only its physical location and one struct's field visibility.
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`, `crates/nono-cli/src/exec_strategy_windows/mod.rs`, `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` (new).
- **Commit:** `f5f9587f`
- **Test-count consequence (documented, not hidden):** the daemon binary's own `#[cfg(test)]` copy of these 3 tests moved WITH the method to the `nono` binary's tree. `cargo test -p nono-sandbox-cli --bin nono-agentd` now reports **87 passed** (was 90 immediately after Task 1's own addition: 89 pre-existing + 1 new discovery test) — a **-3** delta, not a **+1**, relative to the plan's literal acceptance-criteria wording ("89+ tests, none newly failing"). The 3 tests were NOT lost: they now run under `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- emit_attestation_event` (verified: 3 passed, 0 failed), which is the binary where they now belong, since that's the only binary that still compiles the method they test.

## Non-Vacuity Proof (per Task 1's acceptance criteria)

Temporarily re-added `DummyCounterfactual` to the enum (plus a matching wildcard-style arm, since Rust's exhaustiveness checker independently caught the unmatched variant at compile time — an even stronger signal than the runtime test). Ran the new test:

```
thread '...daemon_attestation_decision_is_deliberately_two_state' panicked at ...:
assertion `left == right` failed: DaemonAttestationDecision must stay a genuine two-state
enum (Proceed / Abort). ... got ["Proceed", "Abort", "DummyCounterfactual"]
  left: ["Proceed", "Abort", "DummyCounterfactual"]
 right: ["Proceed", "Abort"]
```

Confirmed the exact mismatch was named, then reverted both edits. An earlier draft of the test's parsing logic (before this proof) also caught a real bug in itself: filtering only by "not a doc comment" mis-parsed `Abort { layer: &'static str, status: ... }`'s FIELD lines as extra variants (`got ["Proceed", "Abort", "layer: &'static str", "status: ..."]`) — fixed by filtering on exact indentation (8 spaces = variant level; struct-variant fields sit at 12 spaces) before extracting the name.

## Issues Encountered

None beyond the investigated-and-resolved deviation above. No auth gates. No checkpoints (plan is fully autonomous, single task).

## Verification Performed

- `cargo test -p nono-sandbox-cli --bin nono-agentd -- --test-threads=1` — **87 passed, 0 failed** (see test-count note above).
- `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 emit_attestation_event` — **3 passed, 0 failed** (the relocated tests, now in `exec_strategy::attestation_downgrade_event::tests`).
- `cargo clippy -p nono-sandbox-cli --bin nono-agentd -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo clippy -p nono-sandbox-cli --bin nono -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo clippy --workspace --all-targets --features layer-fault-injection -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo build --workspace --all-targets` — clean.
- `cargo fmt --all -- --check` — clean.
- `grep -n "ProceedDowngraded" crates/nono-cli/src/agent_daemon/launch.rs` — zero matches outside historical/explanatory doc comments (confirmed: the enum variant, the match arm, and the hardcoded-literal-history doc-comment reference to the OLD hardcoded value are the only remaining textual occurrences, none of them live code).
- `grep -n "mirrors its decision shape" crates/nono-cli/src/agent_daemon/launch.rs` — shows the corrected two-state text.
- `git diff --diff-filter=D --name-only HEAD~1 HEAD` — no unexpected file deletions.

**Cross-target clippy:**
- `agent_daemon/launch.rs`: DOES NOT APPLY (unchanged from the plan's own stated verification — confirmed via `grep -n '#\[cfg(target_os = "linux"\|"macos"'` returning no matches; the whole file is only `#[path]`-included under `target_os = "windows"`).
- `exec_strategy_windows/mod.rs` and the new `attestation_downgrade_event.rs`: DOES NOT APPLY for the same structural reason — the entire `exec_strategy_windows` tree is `#[path]`-included only under `#[cfg(target_os = "windows")]` in `main.rs` (verified by reading the surrounding `main.rs` lines).
- `telemetry/mod.rs`: this file IS cross-platform (compiles on Linux/macOS too — Windows-specific emitters are cfg-gated inside it), so it is not automatically exempt the way the other three files are. However, `grep -n 'cfg(target_os = "linux"\|cfg(target_os = "macos"\|cfg(any(target_os = "linux"'` against this file returns **zero matches** — the CLAUDE.md trigger condition ("files containing `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks") is not met. The edit itself (removing one method + widening one struct's field visibility to `pub(crate)`) is platform-agnostic Rust with no OS-conditional branching, so there is no Unix-specific surface for a cross-target build to catch. The local `cross`/`cargo-zigbuild` gates were NOT run for this reason; documented here per the checklist's own decision tree rather than silently skipped.

## Known Stubs

None.

## Threat Flags

None. This plan removes dead code that misrepresented the daemon's decision-state coverage (a repudiation-class gap already dispositioned `mitigate` in the plan's own threat model) and relocates an existing audit-emission method without changing its behavior, callers, or the security event schema it emits.

## Next Phase Readiness

- NR3-05 is fully resolved: no dead decision-state variant remains; the daemon's actual (stricter, two-state) posture is documented and mechanically protected against silent drift by a discovery-based test.
- CINT-02 is closed at the requirements level (wave 6, spanning 117-13 through this plan) — all four prior plans' SUMMARY.md files were verified present before making this call (`117-13-SUMMARY.md`, `117-14-SUMMARY.md`, `117-15-SUMMARY.md`, `117-16-SUMMARY.md`, all confirmed present in the phase directory). `.planning/REQUIREMENTS.md` updated by hand (checkbox + traceability table row for CINT-02 → Complete); `git diff` on that file reviewed before commit and shows only the two intended CINT-02 line changes.
- 117-18 (verify-gate meta-test discovery) should be aware: the daemon binary's raw test count dropped from what a naive "89+ then growing" expectation might assume, because of the legitimate cross-binary test relocation documented above — a meta-test that hardcodes an expected COUNT (rather than discovering tests by name/pattern) against `nono-agentd`'s test suite would need updating for this.
- No blockers.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs`
- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-17-SUMMARY.md`
- FOUND commit `f5f9587f` (fix)
- FOUND commit `feae9ede` (docs)
