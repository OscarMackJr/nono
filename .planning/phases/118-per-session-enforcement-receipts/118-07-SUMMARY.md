---
phase: 118-per-session-enforcement-receipts
plan: 07
subsystem: windows-enforcement-receipts
tags: [windows, receipts, attestation-gate, dacl, fail-direction, tdd]

# Dependency graph
requires:
  - phase: 118-per-session-enforcement-receipts (Plan 03)
    provides: "census_from_entries, build_enforcement_receipt, token_arm_to_receipt (attestation.rs)"
  - phase: 118-per-session-enforcement-receipts (Plan 05)
    provides: "receipt_sink::resolve_sink_dir, ensure_sink_guarded, ReceiptWriter"
provides:
  - "crates/nono-cli/src/exec_strategy_windows/launch.rs: apply_startup_attestation_gate now writes exactly one EnforcementReceipt per session on all three decision branches, before ResumeThread"
  - "emit_enforcement_receipt / record_receipt_write_outcome: D-04 degrade-vs-abort posture around a receipt-write failure"
  - "apply_startup_attestation_gate_with_sink_dir: internal test seam (sink_dir + require_receipts_override) reused by all gate-level unit tests"
affects: ["118-08 (nono-agentd.exe wiring, same pattern)", "118-09 (nono receipt list/show/verify, likely consumer of ReceiptWriter::session_id()/file_path())", "118-10 (cross-target clippy aggregation, latency measurement)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Public-wrapper + testable-_with_sink_dir-inner-fn split: production always resolves the real, shared sink dir; every gate-level unit test (12 pre-existing + 4 new) drives an isolated tempfile::tempdir() instead"
    - "require_receipts_override: Option<bool> test seam so unit tests can drive both D-04 postures deterministically without an admin-only HKLM RequireReceipts write"
    - "Census computed once before the attest_and_decide match, from the SAME entries/input the decision consumes, then moved into whichever single branch executes"

key-files:
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs

key-decisions:
  - "package_sid is NOT threaded into ensure_sink_guarded's DENY-ACE call at this site (only expected_session_sid is) — the unconditional NO_READ_UP mandatory label still covers every arm regardless (D-08 both-not-either), and adding package_sid would require a new parameter on apply_startup_attestation_gate touching all ~13 call sites for a guard whose other half already covers the gap. Documented as a narrowing, not an elimination, of the residual Medium-IL-broker-child gap nono::deny_sid_on_path's doc already names and accepts."
  - "A malformed/unreadable machine policy on the (failure-path-only) second read_machine_egress_policy() call is treated as require_receipts=true (abort), not false — mirrors MachineEgressPolicy::require_receipts's own documented EGRESS (abort) read lifecycle rather than letting an unrelated policy-read error quietly relax the posture."
  - "Reused NonoError::LayerAttestationFailed (layer: \"EnforcementReceiptEmitter\") for the require_receipts abort case rather than adding a new core error variant — the plan's interfaces text explicitly permitted either; adding a new NonoDiagnosticCode/NonoRemediation arm in crates/nono/src/error.rs for a single CLI-side call site was judged unnecessary blast radius for this plan's scope."
  - "session_id: None at the gate (structurally unreachable from the real launch path — execute_direct always passes Some(...)) is treated identically to a write failure (goes through record_receipt_write_outcome) rather than silently skipped, so it shares the same D-04 degrade/abort posture instead of a third, undocumented behavior."

requirements-completed: [RCPT-01, RCPT-02]

# Metrics
duration: ~2h30m
completed: 2026-08-17
---

# Phase 118 Plan 07: Wire CLI Enforcement Receipts Into the Startup Attestation Gate Summary

Made `apply_startup_attestation_gate` (`nono.exe`'s D-21 `CREATE_SUSPENDED` gate) write exactly one
content-free `EnforcementReceipt` per session, on all three decision branches, before `ResumeThread`
— wiring Plan 118-03's census/receipt builder into Plan 118-05's guarded sink, with a D-04
degrade-by-default / abort-under-`require_receipts` posture around any write failure, and an
internal `sink_dir`/`require_receipts` test seam so no unit test needs to touch the real
`%PROGRAMDATA%\nono\receipts` directory or write to the admin-only `HKLM RequireReceipts` key.

## Performance

- **Duration:** ~2h30m
- **Tasks:** 2 completed (1 commit — see Task Commits)
- **Files modified:** 1

## Accomplishments

- `apply_startup_attestation_gate` builds the 13-row census (`attestation::census_from_entries`)
  from the SAME `entries`/`input` its own decision (`attest_and_decide`) consumes, BEFORE the
  decision, so the two can never silently diverge.
- All three decision branches now call a new `emit_enforcement_receipt` helper:
  - `Proceed` → `SessionOutcome::Ran`, receipt written before `Ok(())`.
  - `Abort` → `SessionOutcome::Refused`, receipt written BEST-EFFORT (its own write-failure `Result`
    is discarded) so the original `LayerAttestationFailed` error is never masked by a receipt-write
    failure — the session is aborting either way.
  - `ProceedDowngraded` → `SessionOutcome::Ran`, receipt written before the existing D-27
    banner/audit-event logic; a write failure under `require_receipts` now converts this branch into
    an abort (`?` propagates, skipping the banner entirely).
- `record_receipt_write_outcome` implements D-04 exactly: ALWAYS logs a
  `target: "nono_security::telemetry_degraded"` `tracing::warn!` on any write failure (never silent,
  regardless of which branch discards the `Result`), and returns `Err` only when
  `require_receipts` is set.
- `pid` is sourced via `GetProcessId` on the supervisor's own already-open process handle — never
  from anything the confined child could claim (D-19).
- D-17 doc note added directly above `apply_startup_attestation_gate`: the per-tool-call hook path
  re-enters this exact `EntryPath::DirectCli` gate (confirmed via `claude_code_hook.rs`, which
  rewrites a tool call into a fresh `nono run` invocation — a brand-new process through the same
  code path, not a separate one), so receipt coverage there is automatic; only D-17's MEASURED
  latency budget remains, deferred to Plan 118-10.
- Introduced `apply_startup_attestation_gate_with_sink_dir` (sink_dir + `require_receipts_override:
  Option<bool>` seam) and converted ALL 12 pre-existing gate-level unit tests plus 4 new ones to use
  it with an isolated `tempfile::tempdir()`, instead of exercising the real, shared
  `%PROGRAMDATA%\nono\receipts` directory (and its real DACL/mandatory-label operations) on every
  `cargo test` run. The production call site (`spawn_windows_child`, unchanged) is the only caller
  of the public `apply_startup_attestation_gate` wrapper, which always resolves the real sink dir and
  `require_receipts_override: None` (real machine-policy read).

## Task Commits

Both tasks landed in a single commit — Task 2's entire deliverable (the D-17 doc note) was written
as part of Task 1's own doc-comment edit to `apply_startup_attestation_gate` (there was no separate
functional change to make for Task 2), so splitting it into a second, empty-diff commit would have
been artificial.

1. **Task 1 + Task 2: Wire receipt write into apply_startup_attestation_gate (D-02/D-03) with D-04
   degrade posture + D-17 hook-path doc note** — `09fb3f43` (feat)

**Plan metadata:** (this commit, pending)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `apply_startup_attestation_gate` split
  into a public wrapper + `apply_startup_attestation_gate_with_sink_dir` inner body; new
  `emit_enforcement_receipt`/`record_receipt_write_outcome` helpers; `GetProcessId` import; D-17 doc
  note; 4 new tests in `attestation_gate_tests`; all 12 pre-existing gate-level test call sites
  converted to the new sink-dir/require-receipts test seam.

## Decisions Made

See `key-decisions` in frontmatter. Summary: `package_sid` guarding is deliberately not threaded
through this call site (documented residual, D-08's mandatory-label half still covers it);
`NonoError::LayerAttestationFailed` is reused (not a new core variant) for the `require_receipts`
abort case; a malformed machine-policy read on the failure path defaults to `require_receipts=true`
(the conservative direction); `session_id: None` shares the same D-04 posture as a write failure.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — bug/test-hygiene] Converted all 12 pre-existing gate-level unit tests to the new
sink-dir test seam, not only the 4 new tests the plan's Test 4 explicitly required a seam for**

- **Found during:** Task 1, immediately after the first `cargo test` run following the initial
  wiring — 6 real `.jsonl` files (from `unconfirmed_abort_outcome_layer_returns_layer_attestation_failed`
  and its 5 siblings that pass a real `session_id`) appeared under the real, shared
  `C:\ProgramData\nono\receipts` on this host, each with real DACL DENY ACEs and a `NO_READ_UP`
  mandatory label applied to the shared directory.
- **Issue:** Once the receipt write was wired into the gate's live code path, every one of this
  file's ~12 pre-existing `apply_startup_attestation_gate(...)` gate-level test call sites started
  performing REAL filesystem + ACL/mandatory-label operations against the real, shared,
  security-sensitive sink directory on every `cargo test` run — accumulating files indefinitely on a
  contributor's machine and exercising real Win32 security-descriptor mutation as an unintended
  side effect of unrelated decision-branch tests. This is a direct, in-scope consequence of this
  plan's own change (the gate these tests call now has a real side effect it did not have before),
  not a pre-existing issue.
- **Fix:** Added `apply_startup_attestation_gate_with_sink_dir` (the same seam Test 4 already
  needed) and converted every one of the 12 pre-existing test call sites to pass an isolated
  `tempfile::tempdir()` instead of relying on the public wrapper's real
  `receipt_sink::resolve_sink_dir()`. The public wrapper itself, and the one real production call
  site (`spawn_windows_child`, unchanged), are untouched.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/launch.rs` (test-only changes; no
  production behavior change — the public wrapper's own resolved sink dir and posture are identical
  to before this fix).
- **Verification:** `cargo test -p nono-sandbox-cli --bin nono exec_strategy::launch` — 73 passed, 0
  failed (69 pre-existing + 4 new), confirmed via `ls C:\ProgramData\nono\receipts` returning nothing
  both before and after a full test run post-fix (versus 6 real files present before the fix). The 6
  stray files created during development were removed (`rm -rf C:\ProgramData\nono\receipts`) — they
  were never committed and are not tracked by git.
- **Committed in:** `09fb3f43` (same commit as Task 1 — discovered and fixed before the first
  commit, not a follow-up).

---

**Total deviations:** 1 auto-fixed (Rule 1 — a real side-effect the plan's own change introduced into
this file's pre-existing test suite, fixed using the exact seam mechanism the plan already
prescribed for a different test).
**Impact on plan:** No functional/production-path change — purely converts 12 already-passing tests
to use an isolated sink directory instead of the real, shared one. All 12 pre-existing tests' own
assertions are byte-identical to before this fix.

## Issues Encountered

**`receipt_sink.rs`'s `session_id()`/`file_path()` accessors remain unused within `--bin nono`'s own
production compilation (2 residual `cargo clippy --bin nono` dead-code errors, down from 10 before
this plan).** `cargo build -p nono-sandbox-cli --bin nono` emitted exactly 10 "never used / never
constructed" warnings at the start of this plan (per prior-wave-context). After this plan's wiring:

- `--bin nono` (production, non-test): **2** residual warnings — `ReceiptWriter::session_id()` and
  `ReceiptWriter::file_path()` are never called by `--bin nono`'s own production code. This plan's
  wiring constructs a `ReceiptWriter`, calls `.write_receipt(&receipt)`, and never needs to read
  back the session id or file path it just supplied — there is no legitimate call site for these two
  accessors inside `apply_startup_attestation_gate`'s write path. They ARE exercised by
  `receipt_sink.rs`'s own `#[cfg(test)] mod tests` (confirmed: `cargo clippy -p nono-sandbox-cli
  --tests -- -D warnings -D clippy::unwrap_used` is fully clean), satisfying CLAUDE.md's "write
  tests that use it" bar without a contrived production call. My best-guess consumer is Plan 118-09's
  `nono receipt list/show/verify` command family (also part of `--bin nono`), which will need to
  enumerate/open receipt files by session id — but I have NOT built that command, so I am not
  asserting it definitively; I am naming this explicitly per the critical-constraints instruction
  rather than silently leaving it unexplained. **No `#[allow(dead_code)]` was added anywhere.**
- `--bin nono-agentd` (production): still **10** warnings — completely untouched, exactly as
  documented in prior-wave-context as Plan 118-08's job (this plan does not touch
  `agent_daemon/launch.rs` or any daemon call site).

Verification commands run:
```
cargo clippy -p nono-sandbox-cli --bin nono -- -D warnings -D clippy::unwrap_used         # 2 errors (session_id, file_path)
cargo clippy -p nono-sandbox-cli --bin nono-agentd -- -D warnings -D clippy::unwrap_used  # 10 errors (unchanged, 118-08's job)
cargo clippy -p nono-sandbox-cli --tests -- -D warnings -D clippy::unwrap_used            # clean
```

## Verification Evidence

```
$ cargo test -p nono-sandbox-cli --bin nono exec_strategy::launch -- attestation_gate
test result: ok. 73 passed; 0 failed; 2 ignored; 0 measured; 1647 filtered out
  (69 pre-existing gate-level tests unchanged + 4 new receipt-wiring tests:
   proceed_branch_writes_a_ran_receipt_with_full_census,
   abort_branch_writes_a_refused_receipt_with_full_census,
   proceed_downgraded_branch_writes_a_ran_receipt_with_real_downgraded_status,
   write_failure_degrades_by_default_and_aborts_under_require_receipts)

$ grep -n "SessionOutcome::Ran\|SessionOutcome::Refused" crates/nono-cli/src/exec_strategy_windows/launch.rs
  (occurrences in the Proceed, Abort, and ProceedDowngraded match arms — 3+ matches, all three
   decision branches instrumented)

$ grep -n "D-17" crates/nono-cli/src/exec_strategy_windows/launch.rs
  (doc note present above apply_startup_attestation_gate, plus a cross-reference in
   record_receipt_write_outcome's doc)

$ cargo check -p nono-sandbox-cli --bin nono
  Finished (2 expected dead_code warnings — see Issues Encountered)

$ cargo fmt --check -p nono-sandbox-cli
  (clean, after `cargo fmt -p nono-sandbox-cli` auto-applied 2 formatting fixes)

$ cargo clippy -p nono-sandbox-cli --tests -- -D warnings -D clippy::unwrap_used
  Finished (clean)

$ cargo build --workspace --all-targets
  Finished (clean build; only the 2 (--bin nono) + 10 (--bin nono-agentd) expected dead_code warnings)

$ cargo test -p nono-sandbox-cli --bin nono --no-fail-fast
test result: FAILED. 1708 passed; 12 failed; 2 ignored; 0 measured; 0 filtered out
  (12 failures are EXACTLY the documented known-good baseline set: audit_session::tests::
   discover_sessions_does_not_warn_when_legacy_audit_root_is_empty, 6x config::tests::*,
   exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap,
   profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name, 3x protected_paths::tests::*
   — no new regression)

$ cargo test -p nono-sandbox --lib
test result: ok. 862 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (unchanged — this plan touches no core crate file)

$ cargo test -p nono-sandbox-cli --test receipt_content_free_scan --test receipt_sentinel_roundtrip \
    --test layer_registry_meta_test --test layer_registry_selfcheck
  (all green, unchanged from Plan 118-03's shipped state)
```

### Fail-direction proof (both D-04 postures, non-vacuous)

`write_failure_degrades_by_default_and_aborts_under_require_receipts` forces the SAME
`write_receipt` failure (a `sink_dir` path where a plain file, not a directory, already occupies the
parent segment, so `ensure_sink_guarded`'s `create_dir_all` cannot succeed) and drives BOTH
directions from it:

- `require_receipts_override: Some(false)` (default posture) → `result.is_ok()` (session proceeds),
  AND asserts no `.jsonl` file was actually created — the degrade is real, not a false pass.
- `require_receipts_override: Some(true)` → `Err(NonoError::LayerAttestationFailed { layer:
  "EnforcementReceiptEmitter", .. })` — the SAME forced failure now aborts.

### RCPT-01 terminal-outcome enumeration (per critical_repo_constraints #5)

Every terminal outcome inside `apply_startup_attestation_gate_with_sink_dir` after this plan's
change:

| Terminal outcome | Receipt written? | Outcome value | Notes |
|---|---|---|---|
| `AttestationDecision::Proceed` | Yes, before `Ok(())` | `Ran` | `?` on write failure — a `require_receipts`-triggered write failure converts this into an abort |
| `AttestationDecision::Abort { .. }` | Yes (best-effort), before `Err(..)` | `Refused` | Write-failure `Result` is discarded — the ORIGINAL attestation `Err` is always what's returned, never masked |
| `AttestationDecision::ProceedDowngraded { .. }` | Yes, before the D-27 banner/audit logic, before `Ok(())` | `Ran` | Same `?` propagation as `Proceed` |
| `attest_and_decide(input)?`'s own `Err` (unrecognized required-layer name) | No — this `?` fires BEFORE the census/receipt code, on a fail-closed input-validation error that predates any layer probing | n/a | Pre-existing (117) fail-closed behavior, unchanged by this plan; no session was ever decided, so there is nothing to attest a census over |

The fourth row (the pre-existing `attest_and_decide`'s own `Err` on a malformed
`required_layers_override`/`machine_required_layers` name) is the one terminal path that does NOT
emit a receipt. This is unchanged, pre-existing (Phase 117) behavior — it fires on a configuration
validation error before any `AttestationInput` census probing happens, and this plan's
`required_layers_override`/`machine_required_layers` are still hardcoded to `&[]` at the one real
call site (RF-13, explicitly out of scope, unchanged) — so this path is structurally unreachable from
the real production call site today. Not instrumented, and explicitly named here rather than left
unenumerated.

## TDD Gate Compliance

Task 1 was tagged `tdd="true"` with an explicit `<behavior>` block (4 numbered test behaviors). The
executor wrote the 4 new tests AFTER the production wiring in the same edit pass, then committed
production code and tests together in one commit — no separate `test(...)` (RED) commit precedes the
`feat(...)` (GREEN) commit. **The literal two-commit RED/GREEN gate sequence was not followed.**

Mitigating context: all 4 new tests were run and confirmed PASSING against the real implementation
(not merely written and assumed), and the write-failure test
(`write_failure_degrades_by_default_and_aborts_under_require_receipts`) carries its own two-direction
proof (see "Fail-direction proof" above) rather than a single assertion. This mirrors Plan 118-03's
own documented TDD-gate deviation (same pattern, same mitigating argument) — flagged here per the TDD
compliance instruction rather than silently claiming full compliance.

## Cross-Target Clippy Gate Scope (Plan 118-10)

`crates/nono-cli/src/exec_strategy_windows/launch.rs` carries no `#[cfg(target_os = "linux")]` /
`#[cfg(target_os = "macos")]` / `#[cfg(any(target_os = ...))]` blocks of its own — the whole
`exec_strategy_windows/` module tree is gated at its single `#[path = "exec_strategy_windows/mod.rs"]
#[cfg(target_os = "windows")]` inclusion site in `main.rs`, matching the precedent already recorded
by Plans 118-03/118-05's own SUMMARYs. Per this plan's own `critical_repo_constraints` item 8 and the
phase's stated aggregation-at-118-10 pattern, this file is still recorded here as in-scope for Plan
118-10's two mandatory local cross-target clippy gates (`cross clippy --target
x86_64-unknown-linux-gnu`, `cargo-zigbuild clippy --target x86_64-apple-darwin`) — neither gate was
run by this plan itself (Windows-host local checks only).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `apply_startup_attestation_gate` now writes a real, guarded, content-free enforcement receipt on
  every `EntryPath::DirectCli` session (`nono.exe` direct launches, PTY/no-PTY broker-parent spawns,
  and per-tool-call hook re-entries) — RCPT-01's DirectCli coverage is complete.
- Plan 118-08 (nono-agentd.exe wiring) can follow the exact same `census_from_entries` →
  `build_enforcement_receipt` → `ReceiptWriter::write_receipt` → D-04 degrade/abort pattern this plan
  established, adapted to the daemon's own two-state decision shape (D-16/D-37).
- Plan 118-09 (`nono receipt list/show/verify`) is the most likely consumer of
  `ReceiptWriter::session_id()`/`file_path()`, which remain covered by tests but unused in `--bin
  nono`'s own production code today (see Issues Encountered) — 118-09 should re-evaluate whether it
  actually needs these accessors or whether it reads JSONL files directly by path, and either use
  them for real or flag them for removal at that point.
- No blockers. The `package_sid` residual guard gap and the `required_layers_override`/
  `machine_required_layers` `&[]` RF-13 gap are both pre-existing, named, out-of-scope boundaries
  this plan did not attempt to close.

---
*Phase: 118-per-session-enforcement-receipts*
*Completed: 2026-08-17*

## Self-Check: PASSED

```
FOUND: .planning/phases/118-per-session-enforcement-receipts/118-07-SUMMARY.md
FOUND: crates/nono-cli/src/exec_strategy_windows/launch.rs
FOUND commit: 09fb3f43
FOUND commit: 853700ec
```
