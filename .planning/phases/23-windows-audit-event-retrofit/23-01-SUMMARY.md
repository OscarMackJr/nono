---
phase: 23-windows-audit-event-retrofit
plan: 01
subsystem: audit
tags: [windows, aipc, audit-integrity, ndjson, supervisor, reject-stage, security]

# Dependency graph
requires:
  - phase: 22-upst2-upstream-v038-v040-parity-sync
    provides: AuditRecorder lifecycle (Plan 22-05a), record_capability_decision API stub (Phase 22-05a deferral hook), AppliedLabelsGuard Drop ordering (Plan 22-05b)
  - phase: 18-aipc-01
    provides: 5 AIPC HandleKinds + dispatcher 5-site funnel + WR-01 verdict matrix (G-05) + G-04 broker-failure flip + audit_entry_with_redacted_token
provides:
  - "RejectStage enum (BeforePrompt | AfterPrompt) on AuditEventPayload::CapabilityDecision"
  - "Windows AIPC dispatcher emits one ledger capability_decision event per request at all 5 push sites"
  - "WR-01 verdict matrix mechanically enforced in NDJSON ledger via per-event reject_stage discriminator"
  - "nono audit show <id> surfaces Capability Decisions counter (text) + capability_decisions array (JSON)"
  - "Token-redaction regression guard on the persistent ledger (T-23-01 mitigation)"
  - "Multi-kind layer-1 E2E test locking AUD-05 #1 acceptance"
affects: [v2.2-milestone-close, future-Windows-OpenUrl-broker-phase, future-cross-platform-RejectStage-extension]

# Tech tracking
tech-stack:
  added: [no new dependencies — uses existing tempfile, serde, serde_json, tracing]
  patterns:
    - "Arc<Mutex<AuditRecorder>> shared-ownership across capability-pipe thread boundary"
    - "warn-and-continue ledger emission (recorder errors MUST NOT abort dispatcher per Discretion #3 / T-23-03)"
    - "kebab-case serde rename on RejectStage matching existing snake_case parent enum convention"
    - "co-located reject_stage tracking at G-04 flip site (no downstream string-prefix re-parse)"
    - "best-effort ledger reader (Ok(vec![]) on missing/parse-error) for UX surfaces — load-bearing failure surface stays the integrity-summary path"

key-files:
  created: []
  modified:
    - "crates/nono-cli/src/audit_integrity.rs"
    - "crates/nono-cli/src/exec_strategy_windows/supervisor.rs"
    - "crates/nono-cli/src/exec_strategy_windows/mod.rs"
    - "crates/nono-cli/src/audit_commands.rs"
    - "crates/nono-cli/src/supervised_runtime.rs"
    - "crates/nono-cli/src/exec_strategy.rs"
    - "crates/nono-cli/src/rollback_runtime.rs"

key-decisions:
  - "D-01 honored: thread Option<&Arc<Mutex<AuditRecorder>>> into handle_windows_supervisor_message; emit at all 5 existing audit_log.push sites (single-site discipline preserved per G-04)"
  - "D-02 honored: RejectStage enum + reject_stage field land ONLY on audit_integrity::AuditEventPayload — cross-platform nono::supervisor::AuditEntry untouched (D-19 invariant preserved)"
  - "D-03 honored: SupervisorMessage::OpenUrl arm UNTOUCHED — Windows OpenUrl broker phase will land emission when delegated-browser flow exists"
  - "D-04 honored: 2-layer test coverage with Step 7 fallback (layer 1 dispatcher unit tests + layer 1 multi-kind E2E in capability_handler_tests; layer 2 integration test deferred per plan's authorized fallback because handle_windows_supervisor_message is pub(super) and not exposed via library boundary)"
  - "Discretion #1 (planner): kebab-case serde tag; matches NetworkAuditDecision precedent"
  - "Discretion #2 (planner): per-site lock-and-drop on the recorder mutex (matches supervised_runtime.rs:362 idiom; minimizes contention with AppliedLabelsGuard Drop)"
  - "Discretion #3 (planner): tracing::warn! on recorder error, never propagate via ? — wire response goes out regardless (T-23-03 mitigation)"
  - "Discretion #4 (planner): test fixtures duplicate Arc<Mutex<AuditRecorder>> construction inline rather than share a helper — keeps each wr01_* test self-contained and readable"
  - "Single 2-arg API shape on record_capability_decision (entry, reject_stage) — no _with_stage variant, no shortcut; dispatcher always knows the stage at the call site"
  - "Mutex<AuditRecorder> promoted to Arc<Mutex<AuditRecorder>> at supervised_runtime.rs:235 to enable cross-thread sharing into the capability-pipe-server thread closure (cross-platform plumbing change is type-only — D-21 invariance preserved)"

patterns-established:
  - "RejectStage discriminator pattern: optional + skip_serializing_if + #[serde(default)] for backward-compat field addition to a serde-tagged enum variant"
  - "emit_to_ledger closure helper: warn-and-continue at all error branches, never block wire response"
  - "Per-event ledger surface helper (read_capability_decisions_from_ledger): BufReader+lines + serde_json::Value end-to-end, best-effort degrade on missing/malformed"
  - "Phase 23 deferred-items.md pattern: pre-existing main-branch issues (clippy in nono::manifest, fmt in audit_attestation) tracked in phase-local file rather than scope-creeping into the phase commits"

requirements-completed: [AUD-05]

# Metrics
duration: 75min
completed: 2026-04-29
---

# Phase 23 Plan 01: Windows Audit-Event Retrofit Summary

**Wires the existing AuditRecorder into the Windows AIPC capability-decision dispatcher so that every Windows supervisor decision (Approved / Denied) for the 5 AIPC HandleKinds (Event, Mutex, Pipe, Socket, JobObject) plus the legacy File path is appended to the persistent `audit-events.ndjson` ledger and surfaced through `nono audit show <id>`, with WR-01 reject-stage asymmetry recorded explicitly per event via a new `reject_stage: Option<RejectStage>` field on `AuditEventPayload::CapabilityDecision`. Closes REQ-AUD-05.**

## Performance

- **Duration:** ~75 min
- **Started:** 2026-04-29 (session start)
- **Completed:** 2026-04-29
- **Tasks:** 3 (all `type="auto" tdd="true"`, all autonomous)
- **Files modified:** 7 (5 load-bearing + 3 cross-platform parameter-threading; one of those is in both lists — audit_integrity.rs is touched by all 3 tasks)

## Accomplishments

- `RejectStage` enum (`BeforePrompt` | `AfterPrompt`) added to `crates/nono-cli/src/audit_integrity.rs` with `kebab-case` serde tag and `#[serde(default, skip_serializing_if = "Option::is_none")]` on the new `reject_stage: Option<RejectStage>` field of `AuditEventPayload::CapabilityDecision`. Phase-22-shaped NDJSON files deserialize cleanly with `reject_stage = None`; new entries with `reject_stage = None` omit the field on the wire (cross-platform NDJSON byte-identical).
- `Mutex<AuditRecorder>` promoted to `Arc<Mutex<AuditRecorder>>` end-to-end (supervised_runtime.rs → exec_strategy.rs → rollback_runtime.rs → exec_strategy_windows/mod.rs → WindowsSupervisorRuntime field) so the recorder can cross the Windows capability-pipe thread boundary. Cross-platform layers see only the additional `Arc` wrapper — D-21 invariance preserved.
- `handle_windows_supervisor_message` accepts an 11th parameter `audit_recorder: Option<&Arc<Mutex<AuditRecorder>>>`. A local `emit_to_ledger` closure (warn-and-continue on lock-poison or write-error) emits one `capability_decision` ledger entry at each of the 5 existing `audit_log.push` sites with the WR-01-locked `reject_stage` discriminator.
- All 38 existing test call sites of the dispatcher updated mechanically to pass `None` as the 11th argument; behavior unchanged.
- `record_capability_decision` is now LIVE (no `#[allow(dead_code)]`) and takes the explicit 2-arg shape `(entry, reject_stage)` — single API surface, no `_with_stage` variant.
- `cmd_show` (text) now emits `Capability Decisions: N (M before-prompt, K after-prompt rejections)` when the ledger has capability_decision records. JSON output gains a `capability_decisions` array (or `null` when absent). Both surfaces use a new `read_capability_decisions_from_ledger` helper that mirrors the `verify_audit_log` BufReader+lines pattern and degrades gracefully on missing/malformed ledger.
- 5 wr01_* tests extended in-place with on-disk reject_stage assertions matching the WR-01 verdict matrix (Event/Mutex/JobObject → "before-prompt"; Pipe ReadWrite + Socket privileged-port G-04 flips → "after-prompt"). The Socket privileged-port test additionally asserts the persisted event JSON contains both `"broker failed:"` and `"privileged port"` substrings (Phase 23 AC#2).
- New tests in `capability_handler_tests`:
  - `recorder_emits_one_capability_decision_per_dispatched_request` — happy-path emission proof.
  - `recorder_does_not_abort_dispatcher_on_lock_poison` — T-23-03 lock-poison regression guard.
  - `recorder_emission_is_optional_when_none` — `None` preserves byte-identical pre-Phase-23 behavior.
  - `recorded_ledger_redacts_session_token` — T-23-01 sanitization regression on persistent ledger.
  - `audit_integrity_records_5_handle_kinds_in_ledger` — AUD-05 #1 acceptance: 5 dispatches → 5 ledger entries covering all 5 HandleKinds.
- 4 new tests in `audit_integrity::tests` (Task 1) + 2 new tests in `audit_commands::tests` (Task 3) cover the serde + ledger-reader surfaces.

## Task Commits

Each task was committed atomically with DCO sign-off:

1. **Task 1: Add RejectStage enum + reject_stage field to AuditEventPayload** — `427e1283` (feat)
2. **Task 2: Wire AuditRecorder into Windows AIPC dispatcher at 5 push sites with WR-01-matching reject_stage** — `a9307802` (feat)
3. **Task 3: Surface capability_decisions in audit show; lock WR-01 reject_stage in ledger via wr01_* tests** — `263795a9` (feat)

## Files Created/Modified

- `crates/nono-cli/src/audit_integrity.rs` — RejectStage enum + reject_stage field on AuditEventPayload::CapabilityDecision; AUDIT_EVENTS_FILENAME promoted to pub(crate); record_capability_decision now live + 2-arg; 4 new TDD tests.
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — handle_windows_supervisor_message gains audit_recorder parameter + emit_to_ledger closure helper + 5-site emission with WR-01-matching stages; G-04 flip site tracks reject_stage co-located. WindowsSupervisorRuntime gains audit_recorder field; start_capability_pipe_server clones the Arc into the spawned thread closure. 5 wr01_* tests extended with ledger assertions; 5 new tests added.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — execute_supervised parameter type updated to `Arc<Mutex<...>>`; pass recorder into WindowsSupervisorRuntime::initialize.
- `crates/nono-cli/src/audit_commands.rs` — read_capability_decisions_from_ledger helper + cmd_show counter line + capability_decisions JSON field; 2 new TDD tests.
- `crates/nono-cli/src/supervised_runtime.rs` — Mutex::new → Arc::new(Mutex::new(...)) at the recorder construction site (D-01 type plumbing). Cross-platform; no behavior change.
- `crates/nono-cli/src/exec_strategy.rs` — audit_recorder parameter type updated to `Arc<Mutex<...>>`. Cross-platform; no behavior change.
- `crates/nono-cli/src/rollback_runtime.rs` — RollbackExitContext.audit_recorder type updated to `Arc<Mutex<...>>`. Cross-platform; no behavior change.

## Decisions Made

All 4 plan decisions (D-01..D-04) honored verbatim:

- **D-01 (recorder threading):** ledger emission piggybacks on the existing 5-site funnel inside `handle_windows_supervisor_message`. Per-kind helpers (`handle_event_request`, etc.) untouched — single-site discipline preserved per G-04.
- **D-02 (RejectStage encoding):** `RejectStage` enum + `reject_stage` field land ONLY on `AuditEventPayload::CapabilityDecision` in `audit_integrity.rs`. Cross-platform `nono::supervisor::AuditEntry` is byte-identical to HEAD~3 (D-19 invariant).
- **D-03 (OpenUrl scope):** `SupervisorMessage::OpenUrl` arm at supervisor.rs untouched — no Denied "not implemented" event emitted. Windows OpenUrl audit emission deferred until Windows grows a delegated-browser broker.
- **D-04 (test approach):** 2-layer test coverage. Layer 1 = dispatcher unit tests in `capability_handler_tests` (5 wr01_* extended + 5 new tests). Layer 2 = `audit_integrity_records_5_handle_kinds_in_ledger` multi-kind E2E in the same mod (per plan's authorized fallback — `handle_windows_supervisor_message` is `pub(super)` so the integration-test file at `tests/aipc_handle_brokering_integration.rs` cannot reach it without a CLAUDE.md "library is policy-free" violation).

Discretion items honored: kebab-case serde tag (#1); per-site lock-and-drop on the recorder mutex (#2); `tracing::warn!` and continue on recorder error (#3); duplicate Arc construction inline in each test rather than share a helper (#4).

## Deviations from Plan

### Layer-2 integration test deferral (authorized by plan Step 7)

**Found during:** Task 3 Step 7
**Issue:** The plan's Step 7 layer-2 directive asked for an extension to `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` that exercises `handle_windows_supervisor_message` with an AuditRecorder + ledger parsing. The file calls the lower-level `nono::supervisor::socket::broker_*_to_process` functions directly — it does NOT (and cannot) call `handle_windows_supervisor_message`, which is `pub(super)` inside `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`. Exposing it via the public library surface would violate CLAUDE.md "library is policy-free" (the dispatcher carries CLI-specific approval-backend + recorder logic).
**Fix:** Used the plan's explicit Step 7 fallback: "If the existing test scaffolding doesn't expose `handle_windows_supervisor_message` from a dependency, fall back to extending one of the existing dispatcher unit tests in `capability_handler_tests` mod with the multi-kind scenario — the spirit is the same: 5 dispatch calls produce 5 ledger entries with all 5 HandleKinds." Added `audit_integrity_records_5_handle_kinds_in_ledger` to `capability_handler_tests` mod which dispatches one request per HandleKind through a shared recorder and asserts the ledger contains exactly 5 capability_decision entries with the union of HandleKinds covering {Event, Mutex, Pipe, Socket, JobObject}.
**Files modified:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (added the layer-1 multi-kind test). `tests/aipc_handle_brokering_integration.rs` is unchanged this phase.
**Verification:** `cargo test capability_handler_tests::audit_integrity_records_5_handle_kinds_in_ledger` passes. AUD-05 #1 acceptance is met by the layer-1 test.
**Committed in:** `263795a9` (Task 3)
**Plan frontmatter note:** the plan's `files_modified` list includes `aipc_handle_brokering_integration.rs`. This file is unchanged in the final commit chain because the plan-authorized fallback redirected the layer-2 work into `supervisor.rs`. No scope creep.

---

**Total deviations:** 1 (authorized by plan Step 7 fallback clause)
**Impact on plan:** Spirit of D-04 layer-2 coverage preserved via layer-1 multi-kind test in `capability_handler_tests`. AUD-05 #1 acceptance fully met. `aipc_handle_brokering_integration.rs` was listed in plan frontmatter `files_modified` as a fallback target; this commit chain did not need to modify it.

## Issues Encountered

### Pre-existing main-branch issues (out of scope)

Tracked in `.planning/phases/23-windows-audit-event-retrofit/deferred-items.md`:

1. **Pre-existing clippy errors in `crates/nono/src/manifest.rs` lines 95 + 103** (`clippy::collapsible_match`). Verified by stashing Phase 23 changes and re-running `cargo clippy --package nono --lib -- -D warnings` — same errors. Last touched commit on `manifest.rs` predates Phase 22.
2. **Pre-existing rustfmt drift in `crates/nono-cli/src/audit_attestation.rs`** (lines 281, 421, 444 region). Verified by running `cargo fmt --all -- --check` against a clean tree — same drift. Reverted my fmt application to that file to keep the Phase 23 commit chain minimal.

Both items belong to a future cleanup quick task. Phase 23 leaves them as-is per CLAUDE.md "fix only issues caused by current task" + the plan's `<scope_guardrails>` Phase 23 must not regress, but is not responsible for fixing pre-existing issues.

### Auto-applied formatting

Cargo fmt run during Task 2 produced stylistic changes to the Phase 23 source files (e.g., `Option<...>` line wrap at column 100, multi-line argument lists in `println!` invocations). These are accepted as part of the Phase 23 commits since they affect Phase 23 code only.

## Test Counts

- `audit_integrity::tests::*` — 8 passed (4 pre-existing + 4 new from Task 1)
- `capability_handler_tests::*` — 41 passed (28 pre-existing + 5 wr01_* extended in-place + 3 recorder TDD (Task 2) + 1 redaction regression + 1 multi-kind E2E + 3 unrelated kept under same mod)
- `audit_commands::tests::*` — 5 passed (3 pre-existing + 2 new from Task 3)
- `aipc_handle_brokering_integration::*` — 5 passed (unchanged from pre-Phase-23)
- `audit_flush_before_drop` (labels_guard.rs Phase 22-05b invariant) — passed (UNCHANGED, regression-guard satisfied)

Total new test assertions added: ~14 (4 audit_integrity + 5 capability_handler_tests + 2 audit_commands + 5 wr01_* ledger extensions in-place).

## Invariance Gates (final sweep across HEAD~3..HEAD)

```
=== D-19 Invariant (cross-phase) ===
git diff --stat HEAD~3 HEAD -- crates/nono/src/ \
  crates/nono-cli/src/terminal_approval.rs \
  crates/nono-cli/src/profile/ \
  crates/nono-cli/data/
→ EMPTY OUTPUT ✓

=== D-19 strict (RejectStage stays nono-cli-local) ===
grep -rn "reject_stage\|RejectStage" crates/nono/src/
→ NO MATCHES ✓

=== D-21 (cross-platform exec/runtime files) ===
grep -c "RejectStage" \
  crates/nono-cli/src/exec_strategy.rs \
  crates/nono-cli/src/supervised_runtime.rs \
  crates/nono-cli/src/rollback_runtime.rs
→ All :0 ✓

=== D-03 (OpenUrl arm untouched) ===
git diff HEAD~3 HEAD -- crates/nono-cli/src/exec_strategy_windows/supervisor.rs \
  | grep -A20 "SupervisorMessage::OpenUrl" \
  | grep -E "emit_to_ledger|record_capability_decision"
→ NO MATCHES ✓

=== 5 emit_to_ledger sites in dispatcher ===
grep -nE "^\s+emit_to_ledger\(" \
  crates/nono-cli/src/exec_strategy_windows/supervisor.rs | wc -l
→ 5 ✓

=== record_capability_decision is live ===
grep -B1 "fn record_capability_decision" \
  crates/nono-cli/src/audit_integrity.rs | grep -c "#\[allow(dead_code)\]"
→ 0 ✓
```

All 14 plan `<success_criteria>` items hold.

## Out of Scope (Explicit Deferrals)

Per CONTEXT.md § Deferred Ideas, these items are explicitly NOT shipped in this phase and are NOT in REQ-AUD-05's acceptance:

- **Windows OpenUrl audit emission** — deferred until Windows actually has a delegated-browser broker (no existing surface to emit from; D-03). REQUIREMENTS.md REQ-AUD-05 is left as-is so the requirement remains on file for the future Windows OpenUrl phase.
- **Cross-platform RejectStage on `nono::supervisor::AuditEntry`** — kept Windows-AIPC-local to preserve D-19 invariant. If macOS/Linux ever grow analogous staged rejection paths, revisit.
- **`nono audit show <id> --stage before-prompt` filter** — UX add for triage; not required by AUD-05. Add to v2.3 backlog if asked.
- **Stage-aware ledger compaction / retention policy** — out of scope (AUD-04 already shipped via Phase 22-05b).
- **`AuditEventPayload` schema versioning policy** — `#[serde(default)]` + `skip_serializing_if` is the canonical backward-compat pattern by construction; no schema bump needed.

## User Setup Required

None — no external service configuration required. The new ledger emission path is automatically active for any Windows session run with `--audit-integrity`.

## Next Phase Readiness

**v2.2 milestone is now ready for `/gsd-complete-milestone v2.2`.**

Phase 23 was the only outstanding phase in the v2.2 milestone (per ROADMAP.md). With AUD-05 closed:
- v2.2 scope: 3/3 phases complete (Phase 22 ✓, Phase 23 ✓, Phase 24 ✓).
- Plans: 9/9 complete (8 prior + this 1 plan in Phase 23).

No blockers. No deferred work that blocks the milestone close.

## TDD Gate Compliance

The plan's task `tdd="true"` markers were honored at the per-task level:
- Task 1 added 4 failing tests (RED), then made the field changes (GREEN). Single commit `427e1283` covers both — the field change is small enough that the RED-then-GREEN cycle was internal to one TDD pass.
- Task 2 added 3 failing tests (RED) for the new dispatcher parameter behavior, then implemented (GREEN). Single commit `a9307802`.
- Task 3 extended 5 wr01_* tests + added 2 new dispatcher tests + 2 new audit_commands tests (RED), then implemented helper + counter line + JSON field (GREEN). Single commit `263795a9`.

Each commit's tests are visible via `git show <hash> -- <test_file>` and prove the GREEN was reached. No separate test/feat/refactor split was requested by the plan's `<done>` clauses (which specify a single commit message per task).

## Self-Check: PASSED

- SUMMARY.md exists at expected path ✓
- deferred-items.md exists ✓
- 3 task commits exist in git log: 427e1283, a9307802, 263795a9 ✓
- All Phase 23 verification gates passed (D-19, D-21, D-03, 5 emit sites, record_capability_decision live) ✓
- All test counts match expected: audit_integrity 8 passed, capability_handler_tests 41 passed, audit_commands 5 passed, aipc_handle_brokering_integration 5 passed, audit_flush_before_drop 1 passed ✓

---
*Phase: 23-windows-audit-event-retrofit*
*Plan: 01*
*Completed: 2026-04-29*
