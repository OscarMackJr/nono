---
phase: 90-v3-0-host-gated-uat-drain
plan: 02
subsystem: closeout / verify-dark gates
tags: [drain-01, drain-02, drain-03, host-gated-uat, verify-dark, telemetry-event-emit]
dependency_graph:
  requires: []
  provides: [DRAIN-01, DRAIN-02, DRAIN-03]
  affects:
    - .planning/phases/90-v3-0-host-gated-uat-drain/90-HUMAN-UAT.md
tech_stack:
  added: []
  patterns:
    - "verify-dark.ps1 -File scripted-gate closeout (never -Command bare path)"
    - "Per-gate verdict + operator-gated residual record mirroring 88-HUMAN-UAT.md"
key_files:
  created:
    - .planning/phases/90-v3-0-host-gated-uat-drain/90-HUMAN-UAT.md
  modified: []
decisions:
  - "telemetry-event-emit FAIL recorded as host-gated residual, NOT a gate defect — gate ran SC-1 correctly; FAIL is environmental (pre-telemetry PATH binary + unobservable AppContainer denial + proxy filtering unimplemented for direct Windows supervised runs)"
  - "No gate-script code changed (D-04); gate-improvement noted as future debug finding only"
  - "egress-policy-deny SKIP (not PASS as RESEARCH guessed) — proxy-filter supervision is daemon-path only on Windows"
metrics:
  duration: "~1 hour (incl. live telemetry-emit investigation)"
  completed: "2026-06-20"
  tasks: 2
  files: 1
requirements: [DRAIN-01, DRAIN-02, DRAIN-03]
---

# Phase 90 Plan 02: v3.0 Host-Gated UAT Drain Summary

**One-liner:** Ran the 5 existing `verify-dark.ps1` closeout gates on this dev host via the
`-File` form, captured per-gate verdicts, and recorded them plus the operator-gated residual live
steps for DRAIN-01/02/03 in a new `90-HUMAN-UAT.md` — no gate-script code changed (D-04).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 0 | Human-action checkpoint: run the 5 gates on this dev host | (verdicts persisted by verify-dark.ps1 → `.nono-runtime/verdicts/*.json`) | — |
| 1 | Author 90-HUMAN-UAT.md with per-gate verdicts + host-gated residuals (D-05) | `d1dacba9` | 90-HUMAN-UAT.md |

## Gate Verdicts (D-06 mapping)

| Gate | Req | Verdict | Exit | Disposition |
|------|-----|---------|------|-------------|
| clean-host-install | DRAIN-01 | SKIP_HOST_UNAVAILABLE | 3 | Expected (nono already installed; needs fresh Win11 VM) |
| deploy-silent-install | DRAIN-01 | SKIP_HOST_UNAVAILABLE | 3 | Expected (MSI not staged) |
| wfp-egress-isolation | DRAIN-02 | SKIP_HOST_UNAVAILABLE | 3 | Expected (daemon not running; needs admin + WFP service) |
| egress-policy-deny | DRAIN-02 | SKIP_HOST_UNAVAILABLE | 3 | Acceptable (daemon not running; proxy-filter is daemon-path only) |
| telemetry-event-emit | DRAIN-03 | FAIL | 2 | Host-gated residual — root-caused, not a gate defect |

Verdicts persisted by `verify-dark.ps1` (WR-04) under `.nono-runtime/verdicts/<gate>.json`; the doc
references them and does not re-implement persistence.

## Verification

- All 5 gates invoked via `pwsh -File scripts/verify-dark.ps1 -Gate <name>` (never `-Command`).
- `grep -c '^### ' 90-HUMAN-UAT.md` = **8** (≥5 required; 5 gate blocks + 3 residual/analysis subsections).
- Each gate block records `expected:` / `why_human:` / `result:` with the verdict + exit code + JSON ref.
- Each of DRAIN-01/02/03 has an explicit operator-gated residual-live-step note in `## Gaps`.
- `files_modified` is the doc only — no gate-script files touched (D-04 honored).

## Must-Haves Verified

| Truth | Status |
|-------|--------|
| Each of the 5 gates run via verify-dark.ps1 -File on this dev host and its verdict captured | PASS |
| 90-HUMAN-UAT.md records a per-gate verdict (PASS/FAIL/SKIP_HOST_UNAVAILABLE) for all 5 gates | PASS |
| Each requirement's residual live step explicitly recorded as operator-gated host-gated tech-debt | PASS |
| Verdict JSON persistence stays owned by verify-dark.ps1 (doc references, does not re-implement) | PASS |
| D-04: no gate-script code changed — broken/limited gate escalated as debug finding, not patched | PASS |
| D-06: 5 gates map to requirements exactly | PASS |

## Deviations from Plan

**1. telemetry-event-emit returned FAIL, not SKIP — adjudicated as host-gated residual.**
The plan anticipated SKIPs for most gates and flagged that "only a verdict FAIL on a gate that should
run here is a real issue." The telemetry gate FAILed (exit 2) because its `Invoke-TriggerDenial`
seeds via a file-read deny that is unobservable on the Windows AppContainer backend, and the PATH
`nono` is the pre-telemetry v0.57.5 build. Per the user's direction, a live attempt was made to
generate a real event with the v0.62.2 dev build: path-deny (kernel-side, unobserved) and network-deny
(`--allow-domain` proxy filtering reports "not available yet" for Windows supervised runs) both could
not emit. The only telemetry-emitting denial path on Windows is the daemon + WFP route (the DRAIN-02
residual). Conclusion: the FAIL is environmental and collapses to the DRAIN-03 operator-gated residual;
the gate itself is correct (not patched — D-04). Full root-cause recorded in `90-HUMAN-UAT.md § Gaps`.

**2. egress-policy-deny SKIP'd rather than PASS.** RESEARCH guessed it might PASS in-process; it gates
on the daemon control pipe, and proxy-filter-driven supervision is daemon-path only on Windows
(independently confirmed during the telemetry investigation). Recorded as an acceptable SKIP.

## Known Stubs

None. This plan is a closeout/record plan — no production code. The residual live steps are explicit,
operator-gated host-gated tech-debt (fresh Win11 VM / live daemon + WFP / clean v3.0 MSI), recorded for
`/gsd:progress` and `/gsd:audit-uat` follow-up.

## Gate-Improvement Finding (future debug, not actioned here)

`scripts/gates/telemetry-event-emit.ps1` assumes a file-read deny seeds EventID 10001, which is false
on the AppContainer backend, and its precondition does not detect a non-telemetry build or an
unobservable-denial host. Noted in `90-HUMAN-UAT.md` as a future hardening; not patched (D-04).

## Threat Flags

No new threat surface. This plan ran existing read-only gate scripts and authored a record doc. Verdict
provenance is preserved via verify-dark.ps1's persist-before-emit (WR-04 / T-90-06); `-File`-only
invocation preserved exit-code distinctions (T-90-07).

## Self-Check: PASSED

**Files exist:**
- `.planning/phases/90-v3-0-host-gated-uat-drain/90-HUMAN-UAT.md`: FOUND

**Commits exist:**
- `d1dacba9`: FOUND (test(90-02): record v3.0 host-gated UAT drain verdicts)

**Gate-block count:** 8 `### ` blocks (≥5 required)
**Verdicts captured:** 5/5 (4 SKIP_HOST_UNAVAILABLE + 1 FAIL)
**Residuals recorded:** DRAIN-01, DRAIN-02, DRAIN-03 each have an operator-gated residual note
