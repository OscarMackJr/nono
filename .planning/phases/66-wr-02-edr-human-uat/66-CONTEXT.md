# Phase 66: WR-02 EDR HUMAN-UAT - Context

**Gathered:** 2026-06-11
**Status:** Ready for planning
**Source:** Synthesized from session decisions + STATE.md Phase-66 kickoff (in lieu of discuss-phase — the EDR runner, host, and exercising command were settled live this session)

<domain>
## Phase Boundary

Execute the long-deferred **WR-02 EDR-instrumented HUMAN-UAT** (deferred every milestone since v2.1) against a **real EDR runner**, on the **existing v2.9 binaries with NO new code**. Produce a `66-HUMAN-UAT.md` artifact recording ~10 pass/fail assertions in **two passes** (no-exclusion → with-exclusion), validating the two load-bearing Windows security boundaries under EDR observation, and **close or explicitly re-scope WR-02**.

This is a **HUMAN-UAT / validation phase**: no production code changes. The deliverables are planning artifacts (the UAT checklist + the recorded verdicts + the WR-02 close-out), not source edits.

**In scope:** running nono's supervised/broker path under EDR, observing EDR behavior, recording alert-vs-quarantine verdicts, closing WR-02.
**Out of scope (explicit):** building EDR integrations, EDR telemetry emission, EDR-evasion hardening (deferred as EDR-INTEG-01); running on a CI runner (requires a real EDR host).
</domain>

<decisions>
## Implementation Decisions

### EDR runner (LOCKED)
- **Sysmon + built-in Microsoft Defender Antivirus.** MDE (Defender for Endpoint) is NOT available (only the `NonoTestSign` kernel-driver test cert is on hand; no MDE tenant). The requirement explicitly allows "Sysmon as EDR-proxy **and/or** MDE," so Sysmon + Defender AV satisfies EDR-01.
  - **Defender AV** provides the real **alert + quarantine** side (validated: Normal mode, `RealTimeProtectionEnabled=True`, EICAR quarantined with `ActionSuccess=True`).
  - **Sysmon** provides the **telemetry/visibility** side (process-create with IntegrityLevel, ProcessAccess, CreateRemoteThread, ImageLoad). Running v15.20, schema 4.91, SwiftOnSecurity config.
- **Caveat to bake into the close-out:** Sysmon + Defender AV is a *representative AV + telemetry EDR-proxy*, NOT a full cloud-EDR. It validates the load-bearing OS boundaries and real quarantine, but not MDE-specific cloud behavioral detections. WR-02 closes as **"validated under a representative EDR-proxy"** (a legitimate close, not a re-scope). A later MDE re-run of the same matrix is an EDR-agnostic follow-up.

### Host (LOCKED)
- **Azure VM `nono-fltmgr-vm`** (rg `rg-nono-fltmgr-spike`, Win11 build 26200).
- **Install via the production-signed v0.62.2 *machine* MSI** (`nono-v0.62.2-x86_64-pc-windows-msvc-machine.msi`, Authenticode Valid). The **machine** MSI is REQUIRED: the broker self-trust gate (D-32-12, fail-secure) only spawns the broker from a signed Program-Files install — so EDR-02(b) is only exercisable from this MSI, never the dev-layout or test-cert build.
- EDR ≥24h bake is satisfied (Defender live for days).
- **Host hygiene before the baseline:** the VM still has `TESTSIGNING ON` from the minifilter spike. Either `bcdedit /set testsigning off` + reboot for a clean host, OR explicitly record the posture ("TESTSIGNING on, no kernel driver loaded") in the UAT baseline so driver/testsigning noise is not misattributed to nono. The test minifilter is already unloaded (post-latency-capture).

### The exercising command (LOCKED — grounds both EDR-02 boundaries)
- `nono run --profile claude-code -- <child>` on the signed machine MSI. The `claude-code` profile sets `windows_low_il_broker: true` (`crates/nono-cli/data/policy.json`), so the broker self-degrades via `nono::create_low_integrity_primary_token` and launches the child with `CreateProcessAsUserW(low_il_token, ...)`.
  - That token-create + `CreateProcessAsUserW` integrity-downgrade sequence **is the MITRE T1134.002** behavior EDR-02(b) measures.
  - The resulting **Low-IL child (`NO_WRITE_UP` mandatory label)** is the MIC boundary EDR-02(a) measures.
- A child that makes the Low-IL state observable: `cmd /c whoami /groups` (shows `Mandatory Label\Low Mandatory Level`).

### Two-pass structure (LOCKED, EDR-01)
- **Pass 1 — no exclusions:** characterize false-positive exposure (does the EDR alert/quarantine nono's normal operation?).
- **Pass 2 — with exclusions:** add Defender exclusions for nono (`Add-MpPreference -ExclusionPath/-ExclusionProcess`) and confirm suppression is sufficient. **Key security assertion:** AV exclusions must NOT weaken the OS MIC enforcement — the Low-IL boundary must still hold with exclusions applied (exclusions are AV-scoping, not a security downgrade).
- Every assertion records: **EDR product + version + policy mode**, and distinguishes **"did not alert" from "did not quarantine"** (per SC3).

### Claude's Discretion (research will inform)
- The exact **observation methodology** for each boundary: how Defender/Sysmon actually surface the T1134.002 token-manipulation sequence (which event IDs / threat names), and how to observe whether the EDR's monitoring DLL is injected into the Low-IL child (Sysmon Event 7 ImageLoad / a controlled Medium-IL injection probe).
- The precise count/wording of the ~10 assertions, and the publisher-trust-state recording (whether the v0.62.2 Authenticode cert is publicly-trusted or a POC/org cert affects reputation-driven alerts and must be recorded per-assertion so they're not misattributed to the T1134.002 behavior).
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning.**

### Requirements + roadmap
- `.planning/REQUIREMENTS.md` — EDR-01, EDR-02 (the two requirement IDs); the two-pass + alert-vs-quarantine + MITRE T1134.002 wording.
- `.planning/ROADMAP.md` § Phase 66 — the three Success Criteria (HUMAN-UAT artifact, the two boundaries, WR-02 close/re-scope).

### The code paths the UAT exercises (read-only — no edits this phase)
- `crates/nono-cli/data/policy.json` — `claude-code` profile sets `windows_low_il_broker: true`.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `CreateProcessAsUserW(low_il_token, ...)` + `create_low_integrity_primary_token` (the T1134.002 sequence + Low-IL spawn).
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — token construction.

### Prior art / precedent
- `.planning/phases/65-minifilter-adr-macos-live-re-validation/65-HUMAN-UAT.md` — the house HUMAN-UAT artifact shape (assertion blocks, pass/blocked checkboxes, host-context stamp, sign-off, resume-signal).
- `.planning/STATE.md` § "Phase 66 EDR UAT — KICKED OFF" — the locked EDR/host decisions + the exercising command.

</canonical_refs>

<specifics>
## Specific Ideas

- Host already validated ready: signed v0.62.2 machine MSI (Authenticode Valid), Defender AV 4.18.26050.15 Normal mode (EICAR quarantine proven `ActionSuccess=True`), Sysmon v15.20 schema 4.91 (SwiftOnSecurity config, events flowing).
- Sysmon event IDs to lean on: **1** (process create + IntegrityLevel = MIC boundary visibility), **8** (CreateRemoteThread = injection), **10** (ProcessAccess = token/handle access ≈ T1134.002 proxy), **7** (ImageLoad = whether the EDR DLL loaded into the Low-IL child).
- Defender introspection: `Get-MpThreat`, `Get-MpThreatDetection` (ActionSuccess), `Get-MpComputerStatus`; exclusions via `Add-MpPreference -ExclusionPath/-ExclusionProcess`, reverted with `Remove-MpPreference`.
- Output artifact path: `.planning/phases/66-wr-02-edr-human-uat/66-HUMAN-UAT.md`, close-blocking, with a resume-signal (operator pastes verdicts + host/EDR-version stamp).
</specifics>

<deferred>
## Deferred Ideas

- **MDE (Defender for Endpoint) run** — re-run the same EDR-agnostic matrix under MDE if/when tenant access is available. Not blocking WR-02 close.
- **EDR telemetry emission / EDR-evasion-resistance hardening** — out of scope (EDR-INTEG-01); v2.10 validates *under* EDR, it does not build EDR integrations.
- **CI-runner EDR UAT** — impossible (requires a real EDR host); re-affirmed since v2.1.
</deferred>

---

*Phase: 66-wr-02-edr-human-uat*
*Context gathered: 2026-06-11 (synthesized from session decisions in lieu of discuss-phase)*
