---
slug: wfp-driver-gate-placeholder
status: resolved
trigger: "Windows WFP out-of-box enforcement fails closed at the kernel-driver gate, contradicting Phase 62 D-05 (service-only). Surfaced during 62-04 HUMAN-UAT."
created: 2026-06-02
updated: 2026-06-02
phase: 62
requirements:
  - REQ-WFP-01
---

# Debug: WFP driver-gate / placeholder enforcement contradicts Phase 62 D-05

## Symptoms

**Expected behavior:** A non-elevated supervised `nono run --block-net` on a machine-MSI-installed host (nono-wfp-service registered `start=auto` + RUNNING) enforces WFP kernel network blocking on the confined child out of the box — the child runs but its outbound TCP is blocked (REQ-WFP-01 SC1).

**Actual behavior:** The run prints `net outbound blocked`, then FAILS CLOSED before the child ever execs:
```
nono: Platform not supported: Windows WFP runtime activation is required for blocked Windows network access but the WFP driver `nono-wfp-driver` is not registered. Run `nono setup --install-wfp-driver` first (preferred backend: windows-filtering-platform, active backend: windows-filtering-platform). This request remains fail-closed until WFP activation is implemented.
```
Fail-secure is CORRECT (no unenforced pass-through), but there is NO operational enforcement, so SC1 cannot pass.

**Error messages:** see above; plus in-source placeholder admissions (network.rs): L804 "runtime activation is still not implemented in this build", L1204 "placeholder driver still does not provide network enforcement yet", L1375 "placeholder service host still does not provide network enforcement yet", L915 service responds status=="not-implemented".

**Timeline:** Surfaced 2026-06-02 during Phase 62 (v2.9) 62-04 HUMAN-UAT, on the freshly built+signed machine MSI (`nono-v0.57.5-wfp62.1`). Phase 62 plans 62-01/02/03/05 already shipped (start=auto in MSI generator+wxs, IU pipe SDDL, D-03 service auto-start hook).

**Reproduction:** Machine MSI installed; non-elevated PowerShell:
`nono run --block-net --allow-cwd -- powershell -Command "try { (Invoke-WebRequest https://api.ipify.org -TimeoutSec 5).Content } catch { 'BLOCKED: ' + $_.Exception.Message }"`

## The contradiction to root-cause

- **Phase 62 D-05 / 62-CONTEXT.md L54-56 / 62-RESEARCH.md L55 assert:** "WFP enforcement is already complete/wired; the user-mode `nono-wfp-service` IS the kernel enforcement; the kernel driver `nono-wfp-driver.sys` is OUT of scope (v3.0-deferred)."
- **BUT** the client path `crates/nono-cli/src/exec_strategy_windows/network.rs` REQUIRES the kernel driver `nono-wfp-driver` registered FIRST (error ~L453, L463/L1647) and labels both driver + service as non-enforcing placeholders.
- **YET** the service binary `crates/nono-cli/src/bin/nono-wfp-service.rs` contains real FWPM calls: `FwpmEngineOpen0` (L1178), `FwpmSubLayerAdd0` (L1211), `FwpmFilterAdd0` (L1394), plus filter enum/delete/cleanup.

## Key questions

1. Why does the client (network.rs) gate on the kernel driver being registered before exercising the service? BUG contradicting D-05, or genuine requirement?
2. Is the service's `FwpmFilterAdd0` enforcement path actually reachable + functional end-to-end? Trace: client `install_wfp_network_backend_with_runner` → WFP IPC activation request → service handler → does it call `FwpmFilterAdd0`, or return `not-implemented` first?
3. Genuinely wired (small fix: correct the driver gate so the service's real filter path runs) or genuinely a stub (large gap: `FwpmFilterAdd0` exists but is never invoked / behind a not-implemented branch)?
4. Minimal change to achieve REQ-WFP-01 out-of-box operational enforcement, consistent with D-05 (no real kernel minifilter)?

### Answers

1. **BUG contradicting D-05.** `build_wfp_probe_status` (network.rs L344-383) makes the kernel driver an unconditional precondition of `Ready`. D-05 (62-01-PLAN L20, 62-CONTEXT L53-57) keeps the kernel driver out of scope. The gate is wrong.
2. **Reachable + functional, NOT a stub.** Client `install_wfp_network_backend_with_runner` sends the IPC only when `status == Ready` (L1625). Service handler (L679-687) routes `activate_blocked_mode` → `activate_policy_mode` (L1497) → `install_wfp_policy_filters` (L1423) → real `FwpmFilterAdd0` (L1394), returning `enforced-pending-cleanup` (L758). No `not-implemented` exists in the service activation flow; it is a client-side parse arm (L915) the service never emits.
3. **Genuinely wired — SMALL FIX.** The only reason the service path doesn't run is the client driver gate stranding it. Remove the kernel-driver requirement from the `Ready` decision and the real filter path runs.
4. **Minimal change:** in `build_wfp_probe_status`, drop the kernel-driver binary-exists + service-running requirement from the path to `Ready`, so BFE RUNNING + nono-wfp-service RUNNING is sufficient. Retire stale placeholder strings. Keep all other fail-secure prerequisite checks.

## Current Focus

- hypothesis: ROOT-CAUSED — the client-side `build_wfp_probe_status` gate requires the kernel driver (`nono-wfp-driver`) be RUNNING before returning `Ready`, and `Ready` is the SOLE precondition for sending the activation IPC. Since D-05 deliberately keeps the kernel driver out of scope, the driver is never registered, so the run fails closed before the (fully-wired, real) service `FwpmFilterAdd0` path is ever reached. SMALL FIX.
- next_action: (diagnosis complete — see Resolution)

## Evidence

- timestamp: 2026-06-02 — Live Win11 repro produced the driver-not-registered fail-closed error (above); confined child never ran.
- timestamp: 2026-06-02 — Static grep: service binary has FwpmEngineOpen0/FwpmSubLayerAdd0/FwpmFilterAdd0; network.rs has placeholder/not-implemented strings + driver-registration gate.
- timestamp: 2026-06-02 — network.rs `build_wfp_probe_status` (L344-383): after platform-service + backend-service checks pass, it requires `backend_driver_binary_exists` (else `BackendDriverBinaryMissing`) AND `backend_driver` service == Running (else `BackendDriverMissing`/`BackendDriverStopped`) before returning `Ready`. The kernel driver is an UNCONDITIONAL precondition of `Ready`.
- timestamp: 2026-06-02 — network.rs `install_wfp_network_backend_with_runner` (L1560-1686): the activation IPC (`run_probe` → service → FwpmFilterAdd0) is sent ONLY inside `if status == WfpProbeStatus::Ready` (L1625). Any non-Ready status (incl. `BackendDriverMissing`) falls through to L1681 `UnsupportedPlatform(describe_wfp_runtime_activation_failure(...))` — exactly the observed error — WITHOUT sending the IPC. So the driver gate strands the service path.
- timestamp: 2026-06-02 — nono-wfp-service.rs handler (L679-687): `activate_blocked_mode`/`activate_proxy_mode`/`activate_allow_all_mode` → `activate_policy_mode` (L1497) → (Windows cfg) `install_wfp_policy_filters` (L1423) → real `FwpmFilterAdd0` (L1394). On success returns `enforced-pending-cleanup` (L758-770); unit test asserts this (L1708). The service emits NO `not-implemented` status anywhere in the activation flow — the only `NotImplemented` (L509) is an unrelated service-control event-handler default. The `status=="not-implemented"` string lives ONLY on the client parse side (network.rs L915) as a response the client *can* handle but the service never sends.
- timestamp: 2026-06-02 — Phase 62 planning intent confirms the gate is a bug, not a requirement: 62-01-PLAN L20 "No kernel minifilter (nono-wfp-driver.sys) is added — enforcement stays user-mode-service/FwpmEngine-driven kernel WFP only (D-05, service-only)"; 62-CONTEXT L53-57 "D-05: Service-only — nono-wfp-driver.sys is OUT of scope. The user-mode nono-wfp-service already drives kernel WFP via FwpmEngine at the ALE_AUTH layers, which IS kernel enforcement"; 62-CONTEXT L14-17 "WFP network enforcement is already wired and already kernel-level ... through the same named-pipe IPC."

## Eliminated

- LARGE GAP (service is a stub) — ELIMINATED. `FwpmFilterAdd0` is genuinely invoked end-to-end by the activation handler; it is NOT behind a `not-implemented` branch. The service returns `enforced-pending-cleanup` on success (test-covered). The only stub artifacts are (a) stale placeholder *strings* in network.rs describing the driver/service, and (b) the client's `not-implemented` parse arm, which the service never triggers.

## Resolution

- root_cause: The client-side WFP readiness gate (`build_wfp_probe_status` in `crates/nono-cli/src/exec_strategy_windows/network.rs` L371-381) treats the kernel driver `nono-wfp-driver` (binary present + service RUNNING) as a hard precondition for `WfpProbeStatus::Ready`. Because `Ready` is the only state that triggers the activation IPC in `install_wfp_network_backend_with_runner` (L1625), and because Phase 62 D-05 deliberately leaves the kernel driver unregistered (service-only model), every WFP `nono run` fails closed at the driver gate (L1681 → `BackendDriverMissing`) BEFORE the fully-wired user-mode service ever runs its real `FwpmFilterAdd0` enforcement path. The driver gate directly contradicts D-05. Fail-secure behavior is correct, but the precondition is wrong, so REQ-WFP-01 SC1 can never pass.
- classification: SMALL FIX (bug, not a large gap). The service's real `FwpmFilterAdd0` path is fully implemented and reachable the moment the gate is corrected.
- fix: not applied (goal = find_root_cause_only). Recommended disposition: gap-closure plan against `build_wfp_probe_status` — remove the kernel-driver registration/running requirement from the `Ready` decision so that platform-service (BFE) RUNNING + backend-service (nono-wfp-service) RUNNING is sufficient to reach `Ready` and send the activation IPC. Keep all fail-secure semantics for the BFE and backend-service checks (those ARE prerequisites). Also retire the stale placeholder strings (network.rs L301/L804/L1204/L1212/L1375/L1382) and re-evaluate whether the `BackendDriverBinaryMissing`/`BackendDriverMissing`/`BackendDriverStopped` states should be dropped from the `Ready` path entirely under D-05. Re-run the 62-04 HUMAN-UAT repro to confirm SC1.
