---
status: partial
phase: 90-v3-0-host-gated-uat-drain
source: [90-RESEARCH.md, v3.0-MILESTONE-AUDIT.md]
started: 2026-06-20T22:28:02Z
updated: 2026-06-21T02:40:00Z
---

## Current Test

[closeout complete — all 5 scripted gates run via `verify-dark.ps1 -File` on this dev host;
verdicts captured below. The residual live steps for DRAIN-01/02/03 are operator-gated
host-gated tech-debt (fresh Win11 VM / live daemon + WFP / clean v3.0 MSI install) and are
NOT runnable on this dev host.]

Invocation contract (MEMORY durable + RESEARCH Pitfall 5): `pwsh -File scripts/verify-dark.ps1 -Gate <name>`.
NEVER `-Command "<bare path>"` (swallows exit N → 1). Exit-code map: PASS=0, FAIL=2,
SKIP_HOST_UNAVAILABLE=3, harness-internal=4. verify-dark.ps1 persists
`.nono-runtime/verdicts/<gate>.json` (WR-04) before the stdout line; this doc references those
persisted verdicts, it does NOT re-implement persistence.

## Tests

### 1. clean-host-install (DRAIN-01)
expected: A fresh Win11 host has NO pre-existing `nono.exe` under `C:\Program Files\nono`, so a
clean machine-scope MSI install can be validated from a known-clean baseline.
why_human: Needs a fresh/snapshot-restored Win11 VM — this dev host already has nono installed,
so the gate cannot assert a clean baseline. Host-gated.
result: SKIP_HOST_UNAVAILABLE (exit 3). Reason: "nono.exe detected under C:\Program Files\nono —
host is not clean; snapshot/restore and retry on a fresh Win11 VM". See
`.nono-runtime/verdicts/clean-host-install.json`. EXPECTED drained-to-tech-debt outcome.

### 2. deploy-silent-install (DRAIN-01)
expected: The machine MSI installs silently (`/qn`) with the HKLM policy spine staged, proving
the unattended enterprise-deploy path.
why_human: Needs the built MSI staged at `dist\windows\nono-machine.msi` on a deploy host; not
present on this dev host. Host-gated.
result: SKIP_HOST_UNAVAILABLE (exit 3). Reason: "MSI not found at
C:\Users\OMack\Nono\scripts\dist\windows\nono-machine.msi — stage dist\windows\nono-machine.msi
on this VM before running the gate". See `.nono-runtime/verdicts/deploy-silent-install.json`.
EXPECTED drained-to-tech-debt outcome.

### 3. wfp-egress-isolation (DRAIN-02)
expected: A daemon-launched confined agent with a WFP egress filter cannot reach a blocked
target; the kernel-level WFP filter state is structurally provable.
why_human: Needs admin + the `nono-wfp-service` running + a non-elevated `nono-agentd` daemon (and
ideally a 2nd host). The daemon control pipe is absent on this host. Host-gated.
result: SKIP_HOST_UNAVAILABLE (exit 3). Reason: "nono-agentd is not running (pipe
\\.\pipe\nono-agentd-control absent) — start the daemon in the user (non-elevated) context with
`nono daemon start` then re-run". See `.nono-runtime/verdicts/wfp-egress-isolation.json`.
EXPECTED drained-to-tech-debt outcome.

### 4. egress-policy-deny (DRAIN-02)
expected: A daemon-launched confined agent is denied egress to a non-allowlisted host at the
proxy/policy layer (in-process), proving domain-level egress filtering.
why_human: Requires the `nono-agentd` daemon running (control pipe present). RESEARCH guessed this
*might* PASS in-process on the dev host, but it gates on the daemon pipe. Independently confirmed
during this closeout: proxy-filter-driven supervision is daemon-path only — a direct
`nono run --allow-domain ...` on Windows reports "Windows supervised execution does not implement
proxy-filter-driven supervision yet. Supported Windows supervised features currently: none", so
there is no non-daemon route to exercise this here. Host-gated.
result: SKIP_HOST_UNAVAILABLE (exit 3). Reason: "nono-agentd is not running (pipe
\\.\pipe\nono-agentd-control absent) — start the daemon in the non-elevated user context with
`nono daemon start`, then re-run". See `.nono-runtime/verdicts/egress-policy-deny.json`. Acceptable
drained-to-tech-debt outcome (SKIP, not FAIL).

### 5. telemetry-event-emit (DRAIN-03)
expected: A confined nono denial writes a security event to the Windows Application Event Log under
source `nono` at EventID 10001–10005 with the correct named JSON fields (SC-1), no raw path strings
(SC-3), and the ETW provider `nono` is detectable via logman (SC-5).
why_human: Requires a telemetry-capable (v3.0) nono build AND an *observable* denial AND the
Application-Log `nono` source registered (Phase-82 v3.0 MSI). On this dev host none of those hold —
see the root-cause analysis in `## Gaps`. The live emit + SIEM ingestion + admin opt-out/min_severity
HKLM→emit path is host-gated.
result: FAIL (exit 2). Reason: "SC-1 FAILED: no nono security events found in Application log
(EventID 10001-10005) in the last 5 minutes — run a confined nono command to generate a denial event,
then re-run the gate" (`triggered: true`, `eventCount: 0`). See
`.nono-runtime/verdicts/telemetry-event-emit.json`. The gate ran its real SC-1 assertion correctly
(it is NOT broken — D-04 does NOT apply); the FAIL is environmental, fully root-caused in `## Gaps`,
and collapses to the DRAIN-03 operator-gated residual.

## Summary

total: 5
passed: 0
issues: 1
pending: 0
skipped: 4
blocked: 0

## Gaps

### Operator-gated residual live steps (host-gated tech-debt)

Per the v3.0-Drain intent, each host-gated UAT item collapses to its unattended scripted gate with
the residual live step explicitly operator-gated. None of these are runnable on this dev host.

- **DRAIN-01 residual** — live silent machine-MSI install on a fresh/snapshot-restored Win11 VM,
  validating the HKLM policy spine post-install. (Covers clean-host-install + deploy-silent-install.)
  Operator-gated host-gated tech-debt.
- **DRAIN-02 residual** — live dual-layer egress block (in-process proxy policy + kernel-level WFP
  filter) against a confined daemon-launched agent reaching a blocked target, ideally from a 2nd
  host. Requires admin + `nono-wfp-service` + a running non-elevated `nono-agentd`.
  (Covers wfp-egress-isolation + egress-policy-deny.) Operator-gated host-gated tech-debt.
- **DRAIN-03 residual** — live security-event emit to the Application Log + ETW with downstream SIEM
  ingestion, plus the admin opt-out / `min_severity` HKLM→emit policy honoring. Requires a
  telemetry-capable v3.0 install and an observable denial (see root-cause below).
  Operator-gated host-gated tech-debt.

### telemetry-event-emit FAIL — root-cause analysis (closeout investigation)

A live attempt was made during this closeout to convert the DRAIN-03 FAIL into a real verdict. It is
architecturally blocked on this dev host by compounding realities (the gate itself is correct — this
is the residual, not a gate defect):

1. **PATH binary is pre-telemetry.** `nono` on PATH is `C:\Program Files\nono\nono.exe` **v0.57.5**
   (a v2.8-era build, before Phase 84 wired the Event-Log layer). The gate's `Invoke-TriggerDenial`
   runs the PATH binary, which emits no Application-Log security events. The current telemetry-capable
   build is the dev `target\release\nono.exe` **v0.62.2**.
2. **path-deny is unobservable on the AppContainer backend.** Running the v0.62.2 build's
   path-deny trigger (`run --profile claude-code -- cmd /c type C:\Windows\System32\config\SAM`) from a
   supported execution dir (`%TEMP%`) DID apply the sandbox and the child got "Access is denied" — but
   the denial is kernel-side (AppContainer) and is NOT observed as a `DenialRecord`, so
   `exec_strategy.rs:1995`'s `nono_security::path_deny` emit loop never fires. Path-deny telemetry only
   fires when the IL/mandatory-label backend surfaces `STATUS_ACCESS_DENIED` as a DenialRecord
   (`exec_strategy.rs:1987`), which did not occur here.
3. **network-deny is not available for direct Windows supervised runs.** Running v0.62.2 with proxy
   filtering (`--allow-domain api.anthropic.com -- ... curl https://example.com`) reported "Windows
   supervised execution does not implement proxy-filter-driven supervision yet. Supported Windows
   supervised features currently: none", so the proxy never filters and no `nono_security::network_deny`
   (`audit.rs:203`) is emitted on the direct-run path.
4. **Only the daemon + WFP path emits on Windows.** The lone telemetry-emitting denial path on Windows
   is the daemon-launched agent + WFP egress denial — which is exactly the operator-gated DRAIN-02
   residual (daemon not running). Phase 90-01 (DRAIN-04) just wired the daemon-side `SecurityEventLayer`
   so this path WILL emit once the live daemon+WFP scenario is exercised.

Therefore DRAIN-03's live emit collapses to the operator-gated residual above. No direct-run route to a
real PASS exists on this host.

### Gate-improvement finding (future debug, not patched here — D-04)

`scripts/gates/telemetry-event-emit.ps1`'s `Invoke-TriggerDenial` assumes a file-read deny
(`type SAM`) produces EventID 10001. On the Windows AppContainer backend that denial is kernel-side and
unobserved, so the trigger can never seed an event there. The gate's `Test-Precondition` checks only
"nono on PATH or a recent event exists" — it does not verify the build is telemetry-capable (v3.0+) or
that a denial is observable, so it returns FAIL rather than SKIP when those host conditions are absent.
A future hardening could (a) detect a non-telemetry build / unobservable-denial host and SKIP, and/or
(b) seed via an observable network_deny through the daemon path. Recorded as a debug finding per D-04;
NO gate-script code is changed in this phase (files_modified is this doc only).
