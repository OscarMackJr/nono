---
status: complete
slug: vm-driver-signing-runbook
completed: 2026-06-08
deliverable: .planning/phases/64-minifilter-spike-implementation-macos-p1-cherry-pick-wave/64-SC1-VM-RUNBOOK.md
---

# Quick Task Summary: Phase 64 Track A VM Runbook

One-liner: Wrote a copy-paste, junior-friendly cookbook for the Phase 64 Plan 64-04 Track A human
checkpoint (minifilter test-signing + load + deny harness on the Azure VM), with every Phase 63 UAT
gotcha folded in as an inline lesson.

## What was produced

`64-SC1-VM-RUNBOOK.md` — a 12-section runbook covering, in order: mental model → Phase 63 gotcha list
→ confirm/snapshot VM → connect (Bastion/run-command) → build the `.sys` (EWDK) → pick + commit a
non-colliding altitude → full test-sign pipeline (makecert→certmgr→inf2cat→signtool→bcdedit→pnputil)
→ confirm load (`fltmc`) → build+stage the Rust client → run the deny harness → capture evidence +
post-load snapshot → troubleshooting table + BSOD recovery → an evidence template matching the 64-04
resume signal → a reprovision-from-scratch appendix.

## Phase 63 UAT lessons baked in (the point of the task)

- RDP/3389 blocked by corporate egress → use Azure Bastion (443) or `az vm run-command`.
- `--security-type Standard` mandatory (Trusted Launch blocks `bcdedit /set testsigning on`).
- `Microsoft.Compute/UseStandardSecurityType` feature flag is a one-time subscription prereq.
- DSv5/DASv5 quota = 0 in eastus → fall back to `D4s_v4`.
- `az vm create` may fail once with `OSProvisioningTimedOut` (transient; delete + retry).
- EWDK ISO build env; `LaunchBuildEnv.cmd` hangs under `run-command` → build in a Bastion desktop.
- `.sys`/`.cat`/`.obj` are throwaway, never committed.
- EWDK `signtool` rejects WDK auto test-sign (deletes the `.sys`) → sign manually with `CN=NonoTestSign`.
- BSOD recovery via the `nono-fltmgr-snap-testsigning-ready` snapshot; DESIGN.md BSOD-triad causes.

## Accuracy

Resource names match the actual Phase 63 provisioning (`rg-nono-fltmgr-spike`, `nono-fltmgr-vm`,
`20.51.161.15`, `nono-fltmgr-snap-testsigning-ready`). The deny harness is copied verbatim from
64-RESEARCH.md §Scripted Deny Harness. The pipeline + altitude band match 64-04-PLAN / 64-RESEARCH /
DESIGN.md.

## Notes

- Documentation only — no code changed, no build/test run.
- This runbook is the operator aid for the still-open Phase 64 Plan 64-04 Track A checkpoint; it does
  not itself satisfy the checkpoint (the human must run it on the VM and paste real evidence).
