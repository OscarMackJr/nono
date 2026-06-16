---
phase: 74-persistent-multi-tenant-daemon
plan: "06"
subsystem: doc
tags:
  - uat
  - human-gate
  - appcontainer
  - daemon
  - windows
  - phase-74

# Dependency graph
requires:
  - phase: 74-05
    provides: "nono daemon start|stop|status|install|uninstall + nono agent launch|list CLI verbs"
  - phase: 74-04
    provides: "accept_loop.rs + launch.rs — multi-tenant daemon implementation"
  - phase: 74-03
    provides: "nono-agentd binary skeleton, DaemonState, AgentTenant RAII"
  - phase: 74-02
    provides: "authenticate_pipe_client + ImpersonationGuard + SC5 wire-protocol guard test"
  - phase: 74-01
    provides: "proj/ADR-74-privilege-model.md + daemon_handle_baseline.rs spike harness (4/4 PASS)"
provides:
  - ".planning/phases/74-persistent-multi-tenant-daemon/74-HUMAN-UAT.md — step-by-step SC1-SC5 operator runbook with results table"
affects:
  - "74 phase close (blocked on human-verify checkpoint — 74-06 stays checkpoint/in-progress)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "UAT doc structure: SC4 first (privilege model must precede multi-tenant test), then SC1 (concurrent agents), SC2/SC3/SC5 (automated tier confirmation), regression, teardown"
    - "Known-stub documentation: control pipe (nono agent list) deferred to Phase 75; UAT notes the limitation and provides classify PID fallback"

key-files:
  created:
    - .planning/phases/74-persistent-multi-tenant-daemon/74-HUMAN-UAT.md
  modified: []

key-decisions:
  - "SC4 ordered first in UAT sequence — if daemon runs as SYSTEM, downstream SC1 concurrent-agent test exercises a high-privilege surface; must confirm USER_OWN_PROCESS before spawning any agent"
  - "nono agent list known stub — Phase 74 declares the control pipe name (nono-agentd-control) but Phase 75 wires the daemon-side listener; SC1 uses nono classify <pid> as fallback if list returns empty while agents are alive"
  - "SC3 run ALONE for cold-process characterization — sibling tests pre-pay AppX RPC warmup; when run alone, full canonical breakdown (+66 handles: EtwRegistration+26, Event+12, Semaphore+12, etc.) appears; both modes produce PASS (steady-state delta=0)"

patterns-established: []

requirements-completed:
  - DMON-01
  - DMON-02
  - DMON-03

# Metrics
duration: 25min
completed: "2026-06-15"
---

# Phase 74 Plan 06: SC1-SC5 UAT Protocol Summary

**UAT protocol document written (74-HUMAN-UAT.md) covering SC1-SC5 acceptance on real Win11 host — Phase 74 go/no-go gate awaiting operator execution.**

## Status

**AWAITING HUMAN UAT — NOT PASSED**

This plan is the final Wave 4 gate for Phase 74. The UAT document is written and committed.
The protocol has NOT been executed on a real Win11 host by the operator. Phase 74 remains
at checkpoint/in-progress status pending the human operator's "UAT PASS SC1-SC5" confirmation.

## Performance

- **Duration:** 25 min
- **Started:** 2026-06-15T02:15:00Z
- **Completed:** 2026-06-15T02:40:00Z
- **Tasks:** 1 (doc authoring — checkpoint-gate plan)
- **Files created:** 1

## Accomplishments

- Authored 640-line step-by-step UAT runbook for Phase 74's five acceptance criteria
- SC4 first sequencing documented (privilege model must precede concurrent-agent test)
- SC1 concurrent two-agent proof with distinct SID verification, per-step PowerShell commands,
  and explicit PASS/FAIL criteria
- SC2 cross-tenant denial confirmation via the Wave 0 spike harness integration test
- SC3 handle-baseline 100-cycle test with exact expected output and result-capture fields
- SC5 wire-protocol unit test reference
- Preconditions documented: R-B4 dev-layout gate, real-console requirement, R-B3
  user-owned workspace, pre-existing service registration check
- Teardown sequence and known-stub caveat for `nono agent list` (Phase 75 scope)
- Fill-in results table + common failure mode diagnostics
- Build prereq: `cargo build --release -p nono-cli` (produces BOTH nono.exe + nono-agentd.exe)

## Task Commits

1. **Task 1: Write 74-HUMAN-UAT.md** - `e1dcb9e0` (docs)

## Files Created/Modified

- `.planning/phases/74-persistent-multi-tenant-daemon/74-HUMAN-UAT.md` — Full SC1-SC5 operator
  runbook: preconditions, build prereq, SC4→SC1→SC2→SC3→SC5→regression→teardown sequence,
  go/no-go checklist, results capture table, common failure modes, security assertions,
  resume-signal instructions

## Decisions Made

1. **SC4 first** — Privilege model is the load-bearing gate for the entire daemon. If the
   service registers as WIN32_OWN_PROCESS (SYSTEM-level), the concurrent-agent test is
   exercising a high-privilege surface. SC4 must PASS before SC1 proceeds.

2. **Known-stub caveat for `nono agent list`** — Phase 74 wires the CLI verb and the control
   pipe CLIENT (`nono-agentd-control`) but the daemon-side LISTENER for the control pipe is
   Phase 75 scope (per 74-05-SUMMARY §Known Stubs). The UAT notes this and provides
   `nono classify <pid>` as an alternative for confirming agent confinement class. The SC1
   test relies on the `nono agent list` response, so if the control listener is not yet
   accepting, SC1 may show 0 tenants even while agents are alive. This is a known Phase 75
   gap, not a Phase 74 regression. If observed, record it as an expected limitation and
   use `classify` for PID-level confirmation.

3. **SC3 run-alone guidance** — When run with sibling tests in parallel (default `cargo test`),
   the AppX RPC warmup is pre-paid and the one-time delta is small (~3-4 handles). When run
   alone (cold process), the full +66 canonical breakdown appears. Both are PASS; the UAT doc
   explains both modes to prevent operator confusion.

## Deviations from Plan

None — plan executed exactly as written. This is a doc-only plan (74-HUMAN-UAT.md creation)
with a terminal checkpoint gate.

## Known Stubs

**Phase 74 control pipe daemon-side listener (INTENTIONAL — Phase 75 scope):**

The `nono agent launch` and `nono agent list` CLI verbs (74-05) connect to
`\\.\pipe\nono-agentd-control`. Phase 74 declares and implements the CLIENT side of this pipe.
The DAEMON-SIDE LISTENER for the control protocol is Phase 75 work. As a result, SC1's
`nono agent list` step may return "No agents running" or a pipe-not-found diagnostic even when
agents ARE running — because the daemon is not yet serving the control pipe.

Workaround for SC1 verification during UAT:
- Use `nono classify <agent-pid>` (Phase 73, reused) to confirm each agent PID is classified
  as `AI_AGENT`
- Or use `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test daemon_concurrent_agents` (Wave 0
  spike test) to confirm the daemon's internal tenant table is populated with 2 distinct SIDs

This stub is intentional and documented in 74-05-SUMMARY.md §Known Stubs.

## Issues Encountered

None during doc authoring. The UAT protocol synthesizes facts from the Wave 0-3 SUMMARY docs,
the Wave 0 spike run output (74-01-SUMMARY.md §Spike Results), the agent_cli.rs source (exact
CLI verb syntax), and the ADR-74-privilege-model.md (SC4 framing).

## Phase 74 Carry-Forward to Phase 75

Phase 74 leaves the following items open for Phase 75 (per ADR-74 and 74-CONTEXT.md):

| Item | Phase | Note |
|------|-------|------|
| Control pipe daemon-side listener (nono-agentd-control) | 75 | `nono agent launch|list` stub currently wired to client only |
| Per-agent WFP egress scoping (SUPP-02) | 75 | ADR-74 D-04: no WFP in Phase 74 daemon binary |
| MSI packaging for nono-agentd.exe | 75 | Per-user service install via `nono daemon install`; no machine MSI change in Phase 74 |
| A1 final confirmation in SERVICE context | 75 | Wave 0 spike confirmed `SeImpersonatePrivilege` from test process; UAT SC4 step confirms USER_OWN_PROCESS, which predicts A1 remains true, but running the daemon AS a service and exercising ImpersonateNamedPipeClient from within the service token is the definitive A1 confirmation |

## Self-Check: PASSED

Files verified:
- `.planning/phases/74-persistent-multi-tenant-daemon/74-HUMAN-UAT.md` — FOUND (e1dcb9e0)

Commit verified:
- `e1dcb9e0` — FOUND (`git log --oneline`)
