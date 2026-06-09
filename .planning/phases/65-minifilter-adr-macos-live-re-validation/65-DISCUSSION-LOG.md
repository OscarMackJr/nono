# Phase 65: Minifilter ADR + macOS Live Re-validation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-09
**Phase:** 65-minifilter-adr-macos-live-re-validation
**Areas discussed:** Latency data source, Go/no-go verdict, ADR location + structure, macOS UAT sequencing

---

## Latency data source

| Option | Description | Selected |
|--------|-------------|----------|
| Instrument a real number | Re-spin VM, add KeQueryPerformanceCounter timing, run N iterations, report percentiles | ✓ |
| Budget + qualitative | Document the ~500ms timeout budget + qualitative sub-second observation; flag "not instrumented" gap | |
| Lightweight re-run | Coarse single representative number if VM trivially recreatable | |

**User's choice:** Instrument a real number.
**Notes:** Phase 65 therefore carries a Track A VM/driver component (rebuild → re-sign → reload), not just doc-writing. The ~500ms is the fail-open timeout DESIGN, not a measurement.

### Latency follow-up — measurement scope

| Option | Description | Selected |
|--------|-------------|----------|
| Kernel worker-thread span | IPC-only: before FltSendMessage → after FilterReplyMessage | |
| Full pre-op to completion | Enqueue → IRP completion with STATUS_ACCESS_DENIED | |
| Both, layered | Capture both spans to attribute cost | ✓ |

**User's choice:** Both, layered. Report median + p99 over ~100 denied creates.

### Latency follow-up — VM status

| Option | Description | Selected |
|--------|-------------|----------|
| Still alive / I'll confirm | VM up or restartable; plan assumes existing VM with recreate-if-needed | ✓ |
| Torn down — recreate | Full recreate from runbook required | |
| Unknown — plan for recreate | Idempotent provisioning | |

**User's choice:** Still alive / operator will confirm (planner should still make provisioning idempotent).

---

## Go/no-go verdict

| Option | Description | Selected |
|--------|-------------|----------|
| CONDITIONAL GO (gated) | Proceed but gated on cert/altitude/gap/maintenance preconditions | |
| NO-GO (sufficient already) | WFP+AppContainer suffices; shelve as validated-but-deferred | |
| GO (clear value) | Build the production driver; spike de-risked it | |
| Let evidence decide | ADR weighs latency + burden vs WFP gap; user reviews recommendation before final | ✓ |

**User's choice:** Let evidence decide.
**Notes:** Verdict is NOT pre-committed. The written analysis drives a recommended direction, but the final recommendation is a HUMAN-review gate — operator reviews before it's locked.

---

## ADR location + structure

### ADR home

| Option | Description | Selected |
|--------|-------------|----------|
| Follow repo convention | `.planning/architecture/`, single file, note SC-path deviation | ✓ |
| Honor SC literally | Create `.planning/adr/` as written | |
| Convention + symlink/pointer | Real ADR in architecture/ + pointer stub at SC path | |

**User's choice:** Follow repo convention. Note the `.planning/adr/` → `.planning/architecture/` deviation in verification so the close gate doesn't flag a wrong-path miss.

### ADR shape

| Option | Description | Selected |
|--------|-------------|----------|
| Single comprehensive ADR | One doc, all six topics, references DESIGN.md | |
| ADR + appendix | Concise decision ADR + linked latency-data appendix/evidence file | ✓ |

**User's choice:** ADR + appendix. Raw measurement tables live in a separate appendix; the ADR references (not duplicates) DESIGN.md and SC1 evidence.

---

## macOS UAT sequencing

### macOS host

| Option | Description | Selected |
|--------|-------------|----------|
| Host available now | In-phase live HUMAN-UAT step with SC2 assertions run live | |
| Code-ready now, UAT-gated | Automatable part green now; live sandbox_init() assertions as a close-blocking HUMAN-UAT checklist | ✓ |
| CI-as-primary | Lean on CI; live run best-effort (risks unmet SC2) | |

**User's choice:** Code-ready now, UAT-gated. No macOS host confirmed at discuss time; mirrors prior Windows HUMAN-UAT phase structure.

### CI gate enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Cross-target clippy + CI leg, evidenced | x86_64-apple-darwin clippy + cherry-pick scan + green CI run SHA captured as literal gate evidence | ✓ |
| CI leg only | Rely on release.yml macOS leg; skip local cross-target pre-check | |

**User's choice:** Cross-target clippy + CI leg, evidenced. HARD gate per the v2.9 cross-target-drift incident — no tag until macOS CI green.

---

## Claude's Discretion

- Exact ADR section ordering/headings, appendix filename, precise instrumentation code (counter placement, log format) — provided the layered spans + median/p99 rigor are honored.
- VM recreate mechanics if the VM turns out to be gone — follow `64-SC1-VM-RUNBOOK.md`.

## Deferred Ideas

- Production EV/WHQL-signed driver (DRV-PROD-01) — gated on this ADR's verdict.
- EDR/ETW telemetry emission (EDR-INTEG-01).
- Non-macOS UPST8 cherry-picks (UPST8-NONMAC-01).
- `NonoIpcRequest` ABI-insurance fields.
- Production fail-direction (fail-open vs fail-closed) — note in ADR, don't decide.
