---
phase: 66-wr-02-edr-human-uat
plan: 01
subsystem: testing
tags: [edr, sysmon, defender, wr-02, t1134.002, mic, appcontainer, low-il, human-uat]

requires:
  - phase: 62-wfp-kernel-network-enforcement
    provides: the v2.9/v0.62.2 Windows broker + Low-IL/AppContainer supervised path validated here
provides:
  - 66-HUMAN-UAT.md with 10 EDR assertions executed live + WR-02 CLOSED verdict
  - "EDR-02(a) finding: AppContainer-confined child invisible to Sysmon telemetry"
  - "2 release-packaging findings (VC++ prereq, untrusted POC cert) as follow-up todos"
affects: [wr-02, release-signing, msi-packaging, edr-visibility]

tech-stack:
  added: []
  patterns: ["clean-host EDR HUMAN-UAT: signed-MSI install → broker Low-IL spawn → Sysmon/Defender observation, two passes"]

key-files:
  created:
    - .planning/phases/66-wr-02-edr-human-uat/66-HUMAN-UAT.md
    - .planning/todos/pending/20260611-msi-vcredist-prereq.md
    - .planning/todos/pending/20260611-poc-cert-broker-clean-host.md
  modified:
    - .planning/REQUIREMENTS.md

key-decisions:
  - "WR-02 CLOSED — validated under a representative EDR-proxy (Sysmon + Defender AV); MDE re-run optional, non-blocking"
  - "Release-packaging gaps (VC++ prereq, POC cert) are deployment-robustness findings, NOT WR-02 failures — nono behaves correctly"

patterns-established:
  - "EDR-proxy UAT close: boundaries validated under Sysmon + Defender AV; cloud-EDR (MDE) re-run is an EDR-agnostic follow-up"

requirements-completed: [EDR-01, EDR-02]

duration: ~1 session (host-setup heavy)
completed: 2026-06-11
---

# Phase 66 (Plan 01): WR-02 EDR HUMAN-UAT Summary

**WR-02 CLOSED — nono's OS-enforced Low-IL/AppContainer containment validated live under a representative EDR-proxy (Sysmon + Defender AV): boundary survives AV exclusions, T1134.002 token-downgrade runs without tripping Defender, and the confined child is (notably) invisible to Sysmon telemetry.**

## Accomplishments
- Authored `66-HUMAN-UAT.md` (10 assertions, two passes, baseline stamp, WR-02 decision table) and **executed it live** on `nono-fltmgr-vm` against the signed v0.62.2 machine MSI.
- **EDR-02(a):** Low-IL/AppContainer child confirmed (`whoami /groups` → `Mandatory Label\Low Mandatory Level` `S-1-16-4096`); **survives Defender exclusions** (A-P2-09 — AV scoping ⊥ kernel MIC). Finding: the confined child is **not captured by Sysmon** (Event 1/7), while 54 unconfined `cmd.exe` creates were — containment ⇄ EDR-observability trade-off.
- **EDR-02(b):** the broker chain (`nono.exe`→broker, both High IL → Low-IL child) is the T1134.002 integrity-drop; **Defender raised no alert and no quarantine** on it (integrity-downgrade ≠ the escalation T1134.002 targets).
- Closed EDR-01 + EDR-02 in REQUIREMENTS.md; logged 2 follow-up todos.

## Task Commits
1. **Task 1: Author 66-HUMAN-UAT.md** — `b1f27734` (docs)
2. **Task 2: Operator UAT run + WR-02 close-out** — this commit (verdicts + decision + SUMMARY + requirement closure + todos)

## Findings (the substance of the UAT)
1. **EDR-02(a) — confined child invisible to Sysmon.** Strong/recordable; relies on the boundary (not EDR) to monitor sandboxed work. Scoped to Sysmon; MDE may differ.
2. **VC++ runtime prerequisite missing** → MSI `1603` on a clean host (WFP service couldn't load → SCM 7009 → rollback). → todo `20260611-msi-vcredist-prereq`.
3. **Untrusted POC signing cert (headline)** → broker self-trust gate refuses on a clean host (CERT_E_UNTRUSTEDROOT) → supervised path non-functional out-of-box. → todo `20260611-poc-cert-broker-clean-host`.

## Deviations from Plan
The plan assumed the signed MSI would install and run cleanly. Reality (clean-host UAT surfaced exactly what it should): install required a **VC++ runtime** install; the broker required **manually trusting the POC cert**; the AppContainer child required **`takeown` of the cwd** (admin-created dir was Administrators-owned → nono fail-secure-skipped the package-SID DACL grant). All recorded as host deviations + findings — none are nono behaving incorrectly; they're deployment-robustness gaps + correct fail-secure behavior.

## Issues Encountered
Resolved live: 0xC0000135 (VC++), 1603 (WFP service start, same VC++ root cause), CERT_E_UNTRUSTEDROOT (broker gate), "current directory is invalid" (AppContainer cwd ownership), `.nono` cwd-overlap guard. Each became a recorded finding or a known-guard note.

## Next Phase Readiness
- **WR-02 CLOSED.** EDR-01/EDR-02 satisfied. v2.10 Phase 66 complete.
- 2 follow-up todos filed (MSI VC++ prereq; POC-cert broker-on-clean-host) — release-robustness, separate from WR-02.
- Optional future: re-run the same EDR-agnostic matrix under MDE if tenant access becomes available.

---
*Phase: 66-wr-02-edr-human-uat*
*Completed: 2026-06-11*
