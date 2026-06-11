---
phase: 66
slug: wr-02-edr-human-uat
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-11
---

# Phase 66 — Validation Strategy

> Per-phase validation contract. **This is a HUMAN-UAT phase with NO new production code** —
> validation is operator-run on a real EDR host, not an automated test suite. There is no code
> to sample-test; the UAT assertions themselves ARE the validation. Nyquist "Wave 0 test
> infrastructure" is N/A (nothing to build); the manual-verification map below is the contract.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | none — HUMAN-UAT against existing v2.9 binaries (no new code; no automated tests authored) |
| **Config file** | none |
| **Quick run command** | N/A — manual operator assertions (`66-HUMAN-UAT.md`) on a real Windows EDR host |
| **Full suite command** | N/A — single operator pass-set (two passes: no-exclusion → with-exclusion) |
| **Estimated runtime** | operator-paced (host already baked ≥24 h) |

---

## Sampling Rate

- **Not applicable.** This phase produces no code commits to sample-test. The single validation
  event is the operator executing `66-HUMAN-UAT.md` on `nono-fltmgr-vm` and recording verdicts.
- **Before WR-02 close:** every assertion in `66-HUMAN-UAT.md` must record EDR product + version +
  policy mode, and distinguish *alert* from *quarantine* (SC3).

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 66-01-* | 01 | 1 | EDR-01 / EDR-02 | T-66-MIC / T-66-T1134 | Low-IL child confirmed (`IntegrityLevel=Low`); broker T1134.002 sequence recorded as alert/quarantine; exclusions don't weaken the MIC boundary | manual (HUMAN-UAT) | N/A — operator runs `66-HUMAN-UAT.md` assertions | ✅ (artifact authored by plan) | ⬜ pending operator run |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*None — Existing v2.9 binaries cover all phase requirements; no test infrastructure to install (no new code).*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| nono supervised/broker run under EDR raises (or does not raise) Defender alert / quarantine | EDR-01 | Requires a real EDR-instrumented Windows host (impossible in CI) | `66-HUMAN-UAT.md` Pass 1 (no-exclusion) + Pass 2 (with-exclusion); record `Get-MpThreatDetection` `ThreatStatusID` delta per scenario |
| EDR DLL-injection vs the Low-IL child; child `IntegrityLevel=Low` confirmed at the MIC boundary | EDR-02(a) | OS MIC enforcement only observable on a live host | `66-HUMAN-UAT.md` — Sysmon Event 1 IL + Event 7 ImageLoad of the child + a Medium-IL injection probe |
| Broker `CreateProcessAsUserW` + `create_low_integrity_primary_token` (T1134.002) alert/quarantine verdict | EDR-02(b) | Behavioral EDR response only observable live | `66-HUMAN-UAT.md` — Sysmon Event 1 parent-child IL mismatch + Defender threat delta |
| AV exclusions suppress alerts WITHOUT weakening the Low-IL MIC boundary | EDR-01/02 | Security-invariant check on a live host | `66-HUMAN-UAT.md` Pass 2 re-asserts the MIC boundary with exclusions applied |

---

## Validation Sign-Off

- [x] All "tasks" are manual HUMAN-UAT assertions with explicit operator instructions (no automated verify possible — real EDR host)
- [x] Sampling continuity: N/A (no code commits to sample)
- [x] Wave 0 covers all MISSING references: N/A (no new code)
- [x] No watch-mode flags: N/A
- [x] Feedback latency: operator-paced (host pre-baked)
- [x] `nyquist_compliant: true` — manual-only validation is the correct and only strategy for a no-code EDR HUMAN-UAT

**Approval:** pending (set at operator UAT completion)
