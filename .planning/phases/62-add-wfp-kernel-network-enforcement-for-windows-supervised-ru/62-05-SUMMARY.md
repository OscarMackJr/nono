---
phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
plan: 05
subsystem: infra
tags: [msi, wfp, windows-service, scm, wix, gap-closure]

requires:
  - phase: 62-02
    provides: "dist/windows/nono-machine.wxs snapshot with Start=auto (generator was still demand)"

provides:
  - "scripts/build-windows-msi.ps1 $serviceComponentXml ServiceInstall Start=auto — generator now emits auto-start on every build"
  - "scripts/validate-windows-msi-contract.ps1 asserts Start=auto — regression guard active in CI"
  - "dist/windows/nono-machine.wxs regenerated: snapshot matches generator output"

affects: [62-UAT, release-pipeline, CI-msi-contract-gate]

tech-stack:
  added: []
  patterns:
    - "Generator-as-source-of-truth: the .wxs snapshot is always regenerated from the here-string template; changes to boot-start type must go in the generator, not the snapshot"
    - "Contract guard as regression fence: validate-windows-msi-contract.ps1 retargeted to the new expected value locks the contract against future drift"

key-files:
  created: []
  modified:
    - scripts/build-windows-msi.ps1
    - scripts/validate-windows-msi-contract.ps1
    - dist/windows/nono-machine.wxs

key-decisions:
  - "Change only ServiceInstall Start (demand→auto); ServiceControl Start=install is a separate element controlling install-time start, not boot-start, and must not change"
  - "Update the contract guard to -Expected auto so CI catches any future regression back to demand"
  - "Regenerate the .wxs snapshot with -EmitOnly so the tracked reference file matches the generator; Source= churn with dev-layout paths is the established convention (9e481141)"

patterns-established:
  - "Gap-closure commits: split into three atomic tasks — generator fix, guard retarget, snapshot regen — so bisect lands on the minimal relevant commit"

requirements-completed:
  - REQ-WFP-01

duration: 10min
completed: 2026-06-02
---

# Phase 62 Plan 05: F-62-01 Gap Closure — MSI Generator ServiceInstall Start=auto Summary

**MSI generator here-string and CI contract guard both updated to emit/assert ServiceInstall Start=auto, closing F-62-01 so every build path (CI release.yml + local POC) now registers nono-wfp-service as an auto-start SYSTEM service**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-06-02T~18:00Z
- **Completed:** 2026-06-02T~18:10Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Flipped `Start="demand"` to `Start="auto"` in the `$serviceComponentXml` here-string of `scripts/build-windows-msi.ps1` — this is the generator; every build now emits an auto-start SCM service entry
- Retargeted `Assert-Equal -Expected "demand"` to `-Expected "auto"` in `scripts/validate-windows-msi-contract.ps1`, converting the existing demand-assertion into a regression guard for the new auto value; message tightened to call out "auto/boot-start for out-of-box WFP enforcement"
- Regenerated `dist/windows/nono-machine.wxs` via `-EmitOnly` — snapshot now shows `Start="auto"` at the ServiceInstall node and `Start="install"` at the ServiceControl node; validator exited 0 with "Validated Windows MSI contract for machine and user scopes."

## Task Commits

1. **Task 1: Flip the generator ServiceInstall Start to auto** - `eccd199e` (fix)
2. **Task 2: Retarget the MSI contract guard to expect auto** - `9c0901d6` (fix)
3. **Task 3: Run the contract validator + regenerate the .wxs snapshot** - `35173b70` (chore)

## Files Created/Modified

- `scripts/build-windows-msi.ps1` — ServiceInstall Start="demand" → Start="auto" in $serviceComponentXml here-string (line 235); ServiceControl Start="install" / Stop="both" / Remove="uninstall" untouched
- `scripts/validate-windows-msi-contract.ps1` — Assert-Equal -Expected "demand" → -Expected "auto" at line 200; message updated; ServiceControl assertions untouched
- `dist/windows/nono-machine.wxs` — regenerated reference snapshot; ServiceInstall line 87 now Start="auto", ServiceControl line 94 still Start="install"

## Decisions Made

- **Generator is the source of truth:** Plan 62-02 set Start="auto" in the .wxs snapshot but the snapshot is overwritten on every build by the generator's here-string. The fix goes in the generator, not the snapshot. The snapshot is regenerated after to reflect the new state.
- **Two separate `Start=` attributes, one must change:** ServiceInstall `Start` controls SCM boot-start type (demand vs auto); ServiceControl `Start` controls install-time start (`Start="install"` = start the service at install time). These are independent; only the ServiceInstall attribute was changed.
- **Validator retargeted, not bypassed:** Rather than adding a new assertion, the existing demand-assertion was flipped to auto, making it the regression guard going forward.

## Deviations from Plan

None - plan executed exactly as written.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes. Only attributes in an MSI WiX template and a validator script were modified. T-62-10 and T-62-11 dispositions from the plan's threat model confirmed: ServiceControl Stop/Remove/Wait invariants untouched, and demand→auto does not expand attack surface.

## Issues Encountered

None. All three tasks executed cleanly on the first attempt. Validator exited 0 immediately after Tasks 1 and 2 were committed.

## Next Phase Readiness

- REQ-WFP-01 truth #1 ("the machine MSI registers nono-wfp-service with start=auto") is now structurally enforced by both the generator and the CI gate
- UAT scenarios SC1 (sc qc → AUTO_START), SC2 (boot-start survives reboot) are now achievable from a fresh MSI install
- The contract guard will fail CI if any future commit regresses the Start attribute back to demand

---
*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Completed: 2026-06-02*
