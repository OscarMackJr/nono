---
type: quick
slug: vm-driver-signing-runbook
created: 2026-06-08
task: Write a junior-friendly VM runbook for the Phase 64 Track A minifilter test-signing + deny harness, baking in all Phase 63 UAT lessons.
---

# Quick Task: Phase 64 Track A VM Runbook (junior dev/ops)

## Objective

Produce a step-by-step, copy-paste cookbook that a junior dev/ops person can follow to complete
the Phase 64 Plan 64-04 **Track A** human checkpoint on the Azure test VM, without prior context.
It must fold in every gotcha discovered during Phase 63 UAT so the junior does not re-hit them.

## Deliverable

`.planning/phases/64-minifilter-spike-implementation-macos-p1-cherry-pick-wave/64-SC1-VM-RUNBOOK.md`

## Source material (read-only)

- `.planning/phases/63-.../63-SC1-vm-state.md` — Phase 63 UAT lessons (RDP blocked, quota, Trusted Launch, EWDK)
- `.planning/phases/64-.../64-RESEARCH.md` — deny harness, pitfalls A–F, BSOD triad
- `drivers/nono-fltmgr/DESIGN.md` — BSOD-avoidance contract
- `.planning/phases/63-.../63-altitude-request.md` — altitude band / official request

## Acceptance

- Runbook covers: confirm/provision VM, snapshot safety, connect (Bastion/run-command), build .sys,
  pick altitude, update+commit INF, full test-sign pipeline, load+confirm, build+copy Rust client,
  run deny harness, capture evidence, BSOD recovery.
- Every Phase 63 UAT gotcha is surfaced as an inline "⚠ Phase 63 lesson" callout.
- Exact resource names match what was actually provisioned (rg-nono-fltmgr-spike, nono-fltmgr-vm,
  20.51.161.15, nono-fltmgr-snap-testsigning-ready).
- Evidence-capture template matches the 64-04 resume signal.
