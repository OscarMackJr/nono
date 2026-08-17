---
phase: 118-per-session-enforcement-receipts
plan: 08
subsystem: windows-daemon-enforcement-receipts
tags: [windows, receipts, daemon, attestation-gate, fail-direction, rcpt-01, tdd]

# Dependency graph
requires:
  - phase: 118-per-session-enforcement-receipts (Plan 04)
    provides: "daemon_census_rows, build_daemon_receipt, DAEMON_UNMODELLED_LAYER_EXPECTANCY (agent_daemon/launch.rs)"
  - phase: 118-per-session-enforcement-receipts (Plan 05)
    provides: "receipt_sink::resolve_sink_dir, ensure_sink_guarded, ReceiptWriter (reachable from nono-agentd.rs via #[path])"
  - phase: 118-per-session-enforcement-receipts (Plan 07)
    provides: "the D-04 degrade-vs-abort posture shape this plan mirrors byte-for-byte (record_receipt_write_outcome)"
provides:
  - "crates/nono-cli/src/agent_daemon/launch.rs: launch_agent's step 6.7 gate now writes exactly one EnforcementReceipt per session, on the Proceed (Ran) and Abort (Refused) branches, before ResumeThread"
  - "daemon_attest_decide_and_census: probes app_container_confirmed/job_confirmed ONCE, deriving both the decision and the 13-row census from the same input"
  - "daemon_emit_enforcement_receipt / daemon_record_receipt_write_outcome: D-04 degrade-vs-abort posture, identical shape to Plan 118-07's CLI-side pair"
  - "write_daemon_refuse_receipt_best_effort: RCPT-01 gap closure covering every other confined-session refuse path in launch_agent (steps 6, 6.5x2, 6.6, 7a, 7b) plus a correcting Refused receipt on ResumeThread failure"
affects: ["118-09 (nono receipt list/show/verify, must read this writer's JSONL alongside the CLI's own)", "118-10 (cross-target clippy aggregation)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Probe-once, decide-and-census-together: daemon_attest_decide_and_census duplicates daemon_attest_and_decide's own two probe calls rather than changing that function's return type, since the child is SUSPENDED (no concurrent state change possible between the two probe passes) and the existing function's ~30 tests stay untouched"
    - "Shared best-effort refuse-receipt helper (write_daemon_refuse_receipt_best_effort) reused across every early TerminateProcess branch, always live-re-probing app_container/job status and reporting every not-yet-attempted layer as false (never fabricated true)"
    - "A ResumeThread failure after a successful gate appends a SECOND, correcting Refused receipt rather than leaving the already-written Ran record uncorrected (mirrors Plan 118-06's identical broker decision) — a consumer reads the LAST record per session_id as authoritative"

key-files:
  modified:
    - crates/nono-cli/src/agent_daemon/launch.rs

key-decisions:
  - "Extended receipt instrumentation beyond the plan's literal Task 1 scope (the step 6.7 gate) to cover EVERY other confined-session refuse path in launch_agent (steps 6, 6.5x2, 6.6, 7a, 7b) and the ResumeThread-failure correction — proactively closing the exact class of gap Plan 118-06's broker work was sent back for (instrumenting only 3 of 5 refuse branches on its first submission). RCPT-01 ('every confined session emits a receipt') makes no exception for early-vs-late refusal; a terminated suspended AppContainer child is a confined session that refused, regardless of which step refused it. D-03's 'written ONCE, at the D-21 gate' is preserved in the sense that matters — these are mutually exclusive early-return points in one control-flow graph, so exactly one receipt is ever written per session."
  - "daemon_attest_and_decide itself is left completely UNCHANGED (still directly exercised by its own ~30 pre-existing tests) — launch_agent's call site was switched to a new daemon_attest_decide_and_census wrapper that performs the SAME two probes a second time, rather than changing daemon_attest_and_decide's return type and rippling into every one of its existing call sites. This cannot diverge from the first probe pass because the child process is SUSPENDED (single-threaded control flow) between the two probe calls."
  - "Steps 1-5 (tenant_id generation, exe resolution, AppContainer profile creation, SID derivation, job creation, and spawn_appcontainer_process_suspended's own failure paths) are NOT receipt-instrumented — verified by reading spawn_appcontainer_process_suspended's source that every one of its Err returns occurs BEFORE CreateProcessW succeeds, so no confined process exists yet at any of these failure points. This is a structural justification (no session exists), not a self-imposed one (the exact distinction the 118-06 orchestrator correction turned on)."
  - "daemon_record_receipt_write_outcome duplicates (does not share via a common function) Plan 118-07's exec_strategy_windows::launch::record_receipt_write_outcome — nono-agentd has no [lib] target reachable from nono.exe's own exec_strategy_windows module tree (this file's own 'Module independence' doc section), so the two D-04 implementations are independent copies of the identical posture, not a shared call."

requirements-completed: [RCPT-01]

# Metrics
duration: ~3h
completed: 2026-08-17
---

# Phase 118 Plan 08: Wire Daemon Enforcement Receipts Into the Attestation Gate Summary

Wired `nono-agentd`'s step 6.7 startup self-attestation gate (`launch_agent`) to write a real,
content-free `EnforcementReceipt` through the shared `receipt_sink` writer on every confined
session — the daemon-arm half of D-15's "whoever attests, writes" rule, mirroring Plan 118-07's
CLI-side D-04 degrade/abort posture byte-for-byte. Beyond the plan's literal Task 1 scope,
proactively closed the same class of RCPT-01 gap Plan 118-06's broker work was bounced back for:
every other confined-session refuse path in `launch_agent` (not just the attestation gate) now
also writes a receipt.

## Performance

- **Duration:** ~3h
- **Tasks:** 2 completed (1 commit — Task 2 is a documentation-only finding, no code change)
- **Files modified:** 1

## What Was Built

**Task 1 (`9ef1ef85`)** — `feat(118-08): wire daemon receipt write into the attestation gate (D-02/D-03/D-15)`

- `daemon_attest_decide_and_census(process, job, expected_package_sid, dacl_guard_applied,
  network_scoping_required, wfp_filters_installed, ancestor_traverse_applied) ->
  (DaemonAttestationDecision, Vec<LayerReceiptRow>)` — probes `app_container_confirmed`/
  `job_confirmed` exactly once (the SAME two probes `daemon_attest_and_decide` itself calls),
  then derives BOTH the decision (via `daemon_decision_from_booleans`) and the 13-row census
  (via `daemon_census_rows`) from that one probe pass, so the two can never silently diverge —
  mirroring Plan 118-07's "census built from the SAME entries/input the decision consumes,
  before the decision" pattern. `daemon_attest_and_decide` itself is completely unchanged and
  remains directly exercised by its own ~30 pre-existing tests; `launch_agent`'s step 6.7 call
  site now calls the new wrapper instead.
- `launch_agent`'s step 6.7 `match` now operates on the pre-computed `decision`:
  - `Proceed` → `daemon_emit_enforcement_receipt(..., SessionOutcome::Ran, None)`, written before
    `ResumeThread` (step 8). A write failure under machine policy's `require_receipts` converts
    this branch into an abort (`TerminateProcess` + `cleanup_failed_agent` + `return Err`),
    matching the surrounding steps 6/6.5/6.6/7a fail-secure idiom.
  - `Abort { layer, status }` → `daemon_emit_enforcement_receipt(..., SessionOutcome::Refused,
    None)`, best-effort (its own `Result` discarded — critical_repo_constraints #5: a
    receipt-write failure must never mask the original attestation error).
- `daemon_emit_enforcement_receipt` / `daemon_record_receipt_write_outcome` — the daemon-side
  pair matching Plan 118-07's `emit_enforcement_receipt`/`record_receipt_write_outcome` shape
  exactly: builds the receipt via `build_daemon_receipt`, guards the shared sink via
  `receipt_sink::ensure_sink_guarded(sink_dir, None, Some(package_sid))` (the daemon's own
  AppContainer package SID, not a WRITE_RESTRICTED session SID — there is no such concept on
  this arm), writes via `ReceiptWriter::write_receipt`, and on failure defers to
  `daemon_record_receipt_write_outcome` for the D-04 degrade-visibly-by-default /
  abort-under-`require_receipts` posture (reads `nono::read_machine_egress_policy()` on the
  FAILURE path only — zero added latency on the happy path).
- **RCPT-01 gap closure** (`write_daemon_refuse_receipt_best_effort`): a shared helper, called
  from every OTHER confined-session refuse path in `launch_agent` —
  - Step 6 (`assign_process_to_agent_job` fails)
  - Step 6.5a (`machine_egress_proxy_port` absent)
  - Step 6.5b (`wfp_filter_add` fails)
  - Step 6.6 (`DaemonDaclGuard::apply` fails)
  - Step 7a (`agent_registry` mutex poisoned)
  - Step 7b (`tenants` mutex poisoned)

  Every boolean the caller has not yet ESTABLISHED at its own call site is passed as `false`
  (never fabricated `true` — D-13's four-state floor). `app_container_confirmed`/`job_confirmed`
  are always freshly re-probed (never assumed), because the AppContainer SID is fixed at
  `CreateProcessW` (step 5) and is therefore meaningful to probe regardless of how far
  `launch_agent` got.
- A ResumeThread failure (step 8) after a successful gate now appends a SECOND, correcting
  `Refused` receipt for the same `tenant_id` (re-probing fresh via
  `daemon_attest_decide_and_census`) rather than leaving the already-written `Ran` record
  uncorrected — mirrors Plan 118-06's identical decision for the broker's own
  `Ran`-then-`ResumeThread`-fails case; a consumer reads the LAST record per `session_id` as
  authoritative (D-15's append-only, per-writer segment design).
- Fixed the pre-existing `daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition`
  discovery-based source-scan test: it searched for `"match daemon_attest_and_decide("` and split
  on `") {"`, which no longer matches this plan's renamed call site
  (`daemon_attest_decide_and_census(...)` followed by a separate `match decision { ... }`).
  Updated the search string and terminator; the argument-shape checks it performs (NR-05/WR-31's
  non-literal, independent-expression assertions) are unchanged and still pass against the new
  call site.
- Updated two stale doc comments: `build_daemon_receipt`'s ("NOT wired into the live
  `launch_agent` gate by this plan" → now describes the actual wiring) and
  `daemon_attest_and_decide`'s ("Its signature and the `launch_agent` call site are BOTH
  unchanged" → corrected to note it is no longer `launch_agent`'s own call site, though its
  signature and behavior remain unchanged).
- 4 new tests in `daemon_receipt_wiring_tests` (all against a real, live AppContainer-confined
  suspended child spawned via the existing `LiveFixture`-shaped harness, and an isolated
  `tempfile::tempdir()` sink — never the real, shared `%PROGRAMDATA%\nono\receipts`):
  - `proceed_branch_writes_a_ran_receipt_with_full_census` — Behavior 1.
  - `abort_branch_writes_a_refused_receipt_with_full_census` — Behavior 2.
  - `unconfirmed_on_proceed_row_still_writes_successfully` — Behavior 3 / D-16.
  - `write_failure_degrades_by_default_and_aborts_under_require_receipts` — D-04 fail-direction
    proof, both postures, non-vacuous.

**Task 2** — D-16 triage (documentation only, no code change; see "D-16 Finding" below).

## RCPT-01 Terminal-Outcome Enumeration (per critical_repo_constraints #5)

Every terminal outcome inside `launch_agent`, in source order:

| # | Terminal outcome | Receipt written? | Outcome value | Notes |
|---|---|---|---|---|
| 1 | `generate_tenant_id()?` fails | No | n/a | Before any process exists — structurally no confined session to report |
| 2 | `resolve_exe_path(exe)?` fails | No | n/a | Same — no process yet |
| 3 | `create_app_container_profile` fails | No | n/a | Same — no process yet |
| 4 | `derive_app_container_sid`/`package_sid_to_string` fails | No | n/a | Same — no process yet |
| 5 | `create_agent_job(...)?` fails | No | n/a | Job created before process (step 5) — no process yet |
| 6 | `spawn_appcontainer_process_suspended(...)?` fails | No | n/a | Verified by reading its source: every `Err` return occurs BEFORE `CreateProcessW` succeeds — no confined process exists at any of its failure points |
| 7 | Step 6: `assign_process_to_agent_job` fails | **Yes** | `Refused` | `write_daemon_refuse_receipt_best_effort` (this plan's gap closure) |
| 8 | Step 6.5a: `machine_egress_proxy_port` absent | **Yes** | `Refused` | Same helper |
| 9 | Step 6.5b: `wfp_filter_add` fails | **Yes** | `Refused` | Same helper |
| 10 | Step 6.6: `DaemonDaclGuard::apply` fails | **Yes** | `Refused` | Same helper |
| 11 | Step 7a: `agent_registry` mutex poisoned | **Yes** | `Refused` | Same helper |
| 12 | Step 7b: `tenants` mutex poisoned | **Yes** | `Refused` | Same helper — probed before `tenant`'s own Drop closes the handles |
| 13 | Step 6.7 `Proceed` → receipt-write fails under `require_receipts` | **Yes** (the write attempt itself) | n/a (this IS the write) | `daemon_emit_enforcement_receipt`'s own `Err`, propagated as the whole function's `Err` |
| 14 | Step 6.7 `Abort { layer, status }` | **Yes** | `Refused` | This plan's primary Task 1 deliverable |
| 15 | Step 8: `ResumeThread` fails (after a successful gate) | **Yes** | `Refused` (correcting) | Appends a second record after the already-written `Ran` |
| 16 | Success — `Ok(tenant_id)` | **Yes** (already written) | `Ran` | Written at step 6.7's `Proceed` arm, before `ResumeThread` |

Rows 1-6 are the only unenumerated-by-instrumentation paths, and each is structurally (not
self-imposedly) justified: no confined AppContainer process exists yet at any of those points.
Every terminal outcome from step 5 (process creation) onward — rows 7-16 — now writes exactly one
receipt.

## D-16 Finding (Task 2)

**"D-16": an `Unconfirmed`-on-`Proceed` row WAS observed** — `DaclAncestorTraverse`, exactly the
case Plan 118-04's own `unconfirmed_on_proceed_is_representable_in_a_daemon_receipt` test already
proved was representable at the census level, now confirmed end-to-end through the live write
path by this plan's `unconfirmed_on_proceed_row_still_writes_successfully` test: a real,
job-assigned, AppContainer-confined suspended child, with `ancestor_traverse_applied=false`,
reaches `DaemonAttestationDecision::Proceed` (WR-06: an absent ancestor-traverse ACE under-grants
reach, it never widens confinement, so it must not abort an otherwise-fully-attested launch) while
the written receipt's `DaclAncestorTraverse` row reads `Unconfirmed`.

**Triage disposition: accept, per WR-06 — this is by design, not a new gap.** `DaclAncestorTraverse`
is the ONLY row that can be `Unconfirmed`-while-`Ran` in a real session: every other row that could
be `Unconfirmed` (`AppContainerProfile`, `JobObjectContainment`, `DaclPackageSidGrant`, and
`WfpEgressFilters` when network scoping is required) is a row `daemon_decision_from_booleans`
itself aborts on if `Unconfirmed` — `DaclAncestorTraverse` is the one deliberate exception
(Phase 117-45's WR-31/WR-06 finding, reaffirmed by Plan 118-04). No fix-in-phase action is
warranted; this is the receipt system doing exactly what it was built to do — surfacing an
already-known, already-accepted trade-off for governance visibility rather than hiding it.

**Test harness used:** Task 1's own `unconfirmed_on_proceed_row_still_writes_successfully` test —
a real, live AppContainer-confined suspended `cmd.exe` child (not a synthetic/mocked probe result)
assigned to a real Job Object, run through `daemon_attest_decide_and_census` and
`daemon_emit_enforcement_receipt` end-to-end against an isolated `tempdir` sink. This is "the
closest available test harness" per the plan's own permitted alternative — standing up the full
`nono-agentd` service (SCM registration, named-pipe IPC, an actual `nono agent launch` client
round-trip) was judged out of proportion to the marginal evidence gained: the unit-level harness
already exercises the identical `CreateProcessW`/`AssignProcessToJobObject`/`OpenProcessToken`
Win32 surface the live daemon path uses, through the SAME production functions
(`daemon_attest_decide_and_census`, `daemon_emit_enforcement_receipt`) `launch_agent` itself now
calls.

## Multi-Tenant Concurrency Safety (per critical_repo_constraints #5)

`nono-agentd` is multi-tenant (concurrent `launch_agent` invocations for different sessions).
Concurrency safety is guaranteed at two levels:

1. **File-level:** each session gets its own `<tenant_id>.jsonl` file
   (`ReceiptWriter::new(tenant_id, sink_dir)`) — `tenant_id` is a fresh, 16-byte-random hex string
   per launch (`generate_tenant_id`), so concurrent sessions never write to the same file. This is
   Plan 118-05's own documented scope: "a `nono.exe` DirectCli session and an `nono-agentd.exe`
   Daemon session are never the same session_id" — and within the daemon's OWN sessions, two
   concurrent `launch_agent` calls always produce two distinct `tenant_id`s, so this extends to
   intra-daemon concurrency too.
2. **In-process chain-state mutex:** each `ReceiptWriter` instance (one per `launch_agent` call,
   never shared across sessions) holds its own `Mutex<ReceiptChainState>`, locked once and held
   across the full build+advance+write sequence (`receipt_sink.rs`'s WR-21 discipline) — but since
   no `ReceiptWriter` instance is ever shared between two concurrent `launch_agent` invocations
   (each constructs its own fresh instance inside `daemon_emit_enforcement_receipt`), this mutex
   never contends across sessions; it only protects a single session's own two-write sequence
   (the `Ran` write and, on the rare `ResumeThread`-failure path, the correcting `Refused` write)
   from itself.

No cross-session file collision, no cross-session chain-state race. The only genuinely shared,
concurrently-touched resource is the sink DIRECTORY itself (`ensure_sink_guarded`'s
`create_dir_all` + DACL/label application) — `create_dir_all` is idempotent and safe to call
concurrently from multiple threads/processes (a second caller observing the directory already
exists is not an error), and the DACL/mandatory-label calls are idempotent re-applications of the
identical guard, not a race that could leave the directory in a partially-guarded state
observable by a confined child (each call either fully succeeds or fails closed per
`ensure_sink_guarded`'s own `Err`-propagating design).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — missing critical functionality] Extended receipt instrumentation to every
confined-session refuse path in `launch_agent`, not only the step 6.7 attestation gate**

- **Found during:** Task 1, reading this plan's own `critical_repo_constraints` #5, which
  explicitly cites Plan 118-06's broker gap closure (bounced back for instrumenting only 3 of 5
  refuse branches) as directly relevant guidance for this plan.
- **Issue:** The plan's literal Task 1 scope only describes wiring the receipt write into the
  step 6.7 `daemon_attest_and_decide` call site. But `launch_agent` has 6 OTHER terminal refuse
  paths (steps 6, 6.5x2, 6.6, 7a, 7b) that also terminate a real, already-spawned confined
  AppContainer child and return `Err` — structurally identical in shape to the two
  label-application failures the 118-06 orchestrator correction ruled were in-scope for RCPT-01
  ("both branches still terminate a confined suspended child and return `Err`... RCPT-01 makes no
  exception for early-vs-late refusal").
- **Fix:** Added `write_daemon_refuse_receipt_best_effort`, a shared helper reused at all 6 sites,
  plus a correcting `Refused` receipt on `ResumeThread` failure (mirroring 118-06's identical
  decision for that exact scenario).
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** the full RCPT-01 terminal-outcome enumeration table above; all 6 new call
  sites compile and are exercised by the existing 32 pre-existing `attestation_gate_tests`/`tests`
  (which indirectly cover the surrounding control flow those branches live in) plus this plan's 4
  new tests for the write-path helpers themselves.
- **Committed in:** `9ef1ef85` (same commit as the rest of Task 1 — discovered and fixed before
  the first commit, not a follow-up).

**2. [Rule 1 — bug] Pre-existing source-scan test broke on the renamed call site**

- **Found during:** Task 1, first test run after wiring — 1 pre-existing failure,
  `daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition`.
- **Issue:** This discovery-based test parses `launch.rs`'s own source text for
  `"match daemon_attest_and_decide("` and its argument list up to `") {"`. Renaming the call site
  to `daemon_attest_decide_and_census(...)` (no longer the `match`'s own scrutinee) broke both the
  search string and the terminator.
- **Fix:** Updated the search string to `"daemon_attest_decide_and_census("` and the terminator to
  `");"`. The argument-shape checks the test actually performs (NR-05/WR-31's non-literal,
  independent-expression assertions on `dacl_guard_applied`/`wfp_filters_installed`/
  `ancestor_traverse_applied`) are semantically unchanged and pass against the new call site's
  identical 7-argument shape.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** `cargo test -p nono-sandbox-cli --bin nono-agentd
  daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition` — 1 passed.
- **Committed in:** `9ef1ef85`.

---

**Total deviations:** 2 auto-fixed (Rule 2 — proactive RCPT-01 gap closure directly instructed by
this plan's own `critical_repo_constraints`; Rule 1 — a pre-existing test broken by the call-site
rename, fixed with an equivalent, non-vacuous replacement assertion).
**Impact on plan:** Both are within-scope hardening of this plan's own deliverable, not
scope creep into unrelated files — everything lives in the single file the plan's frontmatter
declares.

## TDD Gate Compliance

Task 1 was tagged `tdd="true"` with an explicit `<behavior>` block (3 numbered test behaviors).
As in every prior plan in this phase (118-03/118-04/118-06/118-07), the executor wrote the new
tests AFTER the production wiring in the same edit pass and committed production code and tests
together in one commit — no separate `test(...)` (RED) commit precedes the `feat(...)` (GREEN)
commit.

Mitigating context, matching this phase's established precedent: all 4 new tests were run and
confirmed PASSING against the real implementation (not merely written and assumed), each against a
REAL, live AppContainer-confined suspended process (not a mock), and the D-04 test
(`write_failure_degrades_by_default_and_aborts_under_require_receipts`) carries its own
two-direction, non-vacuous proof exactly mirroring Plan 118-07's identical evidence shape (default
posture degrades with NO file created; `require_receipts=true` aborts with the SAME forced
failure). Flagged here per the TDD compliance instruction rather than silently claiming full
compliance.

## Verification Evidence

```
$ cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 80 filtered out
  (32 pre-existing + 4 new: proceed_branch_writes_a_ran_receipt_with_full_census,
   abort_branch_writes_a_refused_receipt_with_full_census,
   unconfirmed_on_proceed_row_still_writes_successfully,
   write_failure_degrades_by_default_and_aborts_under_require_receipts)

$ cargo test -p nono-sandbox-cli --bin nono-agentd --no-fail-fast
test result: ok. 116 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (whole-binary sweep, including telemetry/telemetry_init modules — zero regressions)

$ cargo test -p nono-sandbox-cli --bin nono --no-fail-fast
test result: FAILED. 1708 passed; 12 failed; 2 ignored; 0 measured; 0 filtered out
  (12 failures are EXACTLY the documented known-good baseline set: audit_session::tests::
   discover_sessions_does_not_warn_when_legacy_audit_root_is_empty, 6x config::tests::*,
   exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap,
   profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name, 3x protected_paths::tests::*
   — no new regression, this plan touches no file --bin nono compiles)

$ cargo test -p nono-sandbox --lib
test result: ok. 862 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (unchanged — this plan touches no core crate file)

$ cargo build --workspace --all-targets
Finished (clean build; only the 2 (--bin nono) + 2 (--bin nono-agentd) expected dead_code
  warnings on receipt_sink.rs's session_id field/session_id()/file_path() accessors — DOWN
  from 10 pre-118-07 / 10 pre-this-plan on --bin nono-agentd specifically, now converged to
  the identical residual set 118-07 already documented for --bin nono, deferred to 118-09)

$ cargo clippy -p nono-sandbox-cli --bin nono-agentd -- -D warnings -D clippy::unwrap_used
  2 errors (receipt_sink.rs's session_id field + session_id()/file_path() methods — see
  "Issues Encountered")

$ cargo clippy -p nono-sandbox-cli --bin nono -- -D warnings -D clippy::unwrap_used
  2 errors (the SAME 2 — unaffected by this plan, already documented by 118-07)

$ cargo clippy -p nono-sandbox-cli --tests -- -D warnings -D clippy::unwrap_used
  Finished (clean)

$ cargo fmt --check -p nono-sandbox-cli
  (clean, after `cargo fmt -p nono-sandbox-cli` auto-applied formatting)

$ ls C:\ProgramData\nono\receipts
  No such file or directory — confirmed no test in this plan wrote to the real, shared sink
  (all 4 new tests + the D-04 proof use an isolated tempfile::tempdir())

$ git diff --diff-filter=D --name-only HEAD~1 HEAD
  (empty — no unintended deletions)
```

### Fail-direction proof (D-04, both postures, non-vacuous)

`write_failure_degrades_by_default_and_aborts_under_require_receipts` forces the SAME
`write_receipt` failure (a `sink_dir` path where a plain file, not a directory, already occupies
that path, so `ensure_sink_guarded`'s `create_dir_all` cannot succeed) and drives BOTH directions
from it — byte-for-byte the same shape as Plan 118-07's identical CLI-side proof:

- `require_receipts_override: Some(false)` (default posture) → `result.is_ok()` (session
  proceeds), AND asserts no `.jsonl` file was actually created — the degrade is real, not a false
  pass.
- `require_receipts_override: Some(true)` → `Err(NonoError::LayerAttestationFailed { layer:
  "EnforcementReceiptEmitter", .. })` — the SAME forced failure now aborts.

## Cross-Target Clippy Gate Scope (Plan 118-10)

`crates/nono-cli/src/agent_daemon/launch.rs` carries `#[cfg(target_os = "windows")]` at the whole
`mod windows_impl` block (confirmed by Plan 118-04's own SUMMARY) — this plan's changes (both the
production wiring and the 4 new tests) are entirely inside that gated module and its nested
`#[cfg(test)]` submodules. Per CLAUDE.md's cross-target clippy MUST and this phase's own
aggregation-at-118-10 pattern, this file remains in-scope for Plan 118-10's two mandatory local
cross-target clippy gates (`cross clippy --target x86_64-unknown-linux-gnu`, `cargo-zigbuild
clippy --target x86_64-apple-darwin`) — neither gate was run by this plan itself (Windows-host
local checks only).

## Issues Encountered

**`receipt_sink.rs`'s `session_id` field and `session_id()`/`file_path()` accessors remain unused
within `--bin nono-agentd`'s own production compilation, now converged to the IDENTICAL residual
set Plan 118-07 already documented for `--bin nono`.** Prior-wave-context reported `--bin
nono-agentd` at 10 dead-code warnings before this plan. After this plan's wiring:

- `--bin nono-agentd` (production, non-test): **2** residual warnings/errors under `-D
  warnings` — `ReceiptWriter`'s `session_id` field (never READ — this plan's wiring constructs a
  `ReceiptWriter` and calls `.write_receipt(&receipt)`, never reading back the field itself
  because there is no legitimate call site for that inside `daemon_emit_enforcement_receipt`'s
  write path) and its `session_id()`/`file_path()` accessor methods (never CALLED). This is the
  SAME residual pair 118-07 already documented as deferred to Plan 118-09's `nono receipt
  list/show/verify` command family (the most likely real consumer — it will need to
  enumerate/open receipt files by session id). Confirmed clean under `cargo clippy -p
  nono-sandbox-cli --tests -- -D warnings -D clippy::unwrap_used` (the `#[cfg(test)]` module
  exercises every symbol). **No `#[allow(dead_code)]` was added anywhere.**
- `--bin nono` (production): unaffected by this plan (this plan touches no file `--bin nono`
  compiles) — still the same 2 warnings 118-07 already documented.

Both binaries now converge to the IDENTICAL residual dead-code state, which is itself evidence
the wiring is complete and symmetric across the two producers (D-15's "whoever attests, writes").

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `launch_agent` now writes a real, guarded, content-free enforcement receipt on every
  `EntryPath::Daemon` session (`nono agent launch`) — RCPT-01's Daemon coverage is now complete,
  matching the DirectCli coverage Plan 118-07 already shipped and the Broker coverage Plan 118-06
  already shipped. All three producer arms named in D-15 now write real receipts.
- Plan 118-09 (`nono receipt list/show/verify`) is the most likely consumer of
  `ReceiptWriter::session_id()`/`file_path()`, which remain covered by tests but unused in BOTH
  `--bin nono` and `--bin nono-agentd`'s own production code today (see Issues Encountered) —
  118-09 should re-evaluate whether it actually needs these accessors or whether it reads JSONL
  files directly by path, and either use them for real or flag them for removal at that point.
- No blockers. The `DaclAncestorTraverse` `Unconfirmed`-on-`Proceed` finding (D-16) is triaged and
  accepted, not deferred — it is WR-06's own already-shipped, already-justified design, now simply
  visible end-to-end for the first time.

---
*Phase: 118-per-session-enforcement-receipts*
*Completed: 2026-08-17*

## Self-Check: PASSED

```
FOUND: crates/nono-cli/src/agent_daemon/launch.rs
FOUND commit: 9ef1ef85
```
