# Phase 62: Add WFP kernel network enforcement for Windows supervised runs - Context

**Gathered:** 2026-06-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Make `network.block: true` on a **supervised / broker (Low-IL) `nono run`** reliably
enforce WFP kernel network filtering **out of the box**, without a manual
`nono setup --start-wfp-service` step — directly closing Phase 60's F-60-UAT-03
carry-forward.

**The problem being solved:** WFP network enforcement is *already wired and already
kernel-level* for supervised runs (Direct and Supervised/broker paths both activate WFP
through the same named-pipe IPC, threading `session_sid`; WFP filters sit at the kernel
`ALE_AUTH_CONNECT/BIND` layers). The pain is **operational**: `nono-wfp-service` is
registered `start=demand` and is not started by `nono run`, so a non-elevated supervised
run with `network.block:true` **fails closed** with `UnsupportedPlatform` ("WFP service
registered but not running"). That made network blocking impossible to UAT in Phase 60.

**This phase delivers:** the service is present and running by the time enforcement is
needed (boot-started by the SCM as SYSTEM), the run never silently passes through
unenforced, and the enforced-block behavior is human-verifiable on a real Windows host.

**Explicitly NOT this phase** (see Deferred): no new kernel minifilter driver, no
per-process/AppID filter scoping, no fine-grained `allow_domain` path/method work
(that is Phase 56 / proxy-level).

</domain>

<decisions>
## Implementation Decisions

### Service start posture
- **D-01:** The machine MSI registers `nono-wfp-service` with **`start=auto`** so the
  Windows SCM boot-starts it as SYSTEM. No per-run elevation is required — a non-elevated
  supervised `nono run` can rely on the service already running. (Today it is registered
  `start=demand`; see `network.rs` `build_wfp_service_create_args`, ~line 260.)
- **D-02:** `nono setup --start-wfp-service` is **retained** as the manual / dev-layout
  path (dev binaries are not MSI-installed, so they will not get the boot-start posture).

### Behavior when the service is unavailable at enforcement time
- **D-03:** When a `network.block:true` run finds the service not running (stopped, dev
  layout, or otherwise): **attempt to start it; if the start succeeds, enforce. If it
  cannot (e.g. no elevation), abort fail-closed** with an actionable error that names the
  exact remediation command. **Never silently pass through unenforced** (fail-secure is
  non-negotiable — CLAUDE.md "Fail Secure").
- **D-04:** No netsh `FirewallRules` fallback for this path — the phase intent is WFP
  enforcement; a silent backend swap would diverge behavior. (The `FirewallRulesNetworkBackend`
  remains available for its existing explicit selection, but is not an automatic fallback here.)

### Kernel driver scope
- **D-05:** **Service-only — `nono-wfp-driver.sys` is OUT of scope.** The user-mode
  `nono-wfp-service` already drives kernel WFP via `FwpmEngine` at the ALE_AUTH layers,
  which *is* kernel enforcement. A real kernel minifilter ("Gap 6b") stays v3.0-deferred
  (requires a signed kernel driver). This phase adds no signing/distribution burden.

### Milestone / track / requirement
- **D-06:** Phase 62 folds into the **v2.9 "Windows Sandbox-the-Tools" track** (alongside
  Phases 60/61), since it closes Phase 60's F-60-UAT-03 carry-forward.
- **D-07:** Introduce **`REQ-WFP-01`** (out-of-box WFP operational enforcement for
  supervised runs) and map it in the ROADMAP Coverage table. Replace the phase's
  `Requirements: TBD` and the `[To be planned]` goal with a concrete goal + success criteria.

### Claude's Discretion
- Exact internal API shape for the "ensure service running" check + start attempt
  (where it lives relative to `select_network_backend` / `probe_wfp_backend_status_with_config`
  in `network.rs`, and how elevation is detected).
- Precise WiX/`.wxs` change to flip the service to `start=auto` and whether to also set a
  `ServiceConfig` recovery/restart policy.
- Wording of the fail-closed remediation message (must name the elevated remediation command).
- Test structure (unit/integration split) beyond the mandatory human-UAT.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The gap this phase closes
- `.planning/phases/60-sandbox-the-tools-confined-coding-loop-v2-9/60-HUMAN-UAT.md` §F-60-UAT-03 (~lines 136-157) — origin of the operational gap: runner profile sets `network.block:true`, WFP service registered-but-stopped, run fails closed; network blocking deferred out of Phase 60 UAT.

### WFP enforcement & service code (the surfaces to change)
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — backend selection (`select_network_backend`, ~1445-1479), WFP activation/probe (`probe_wfp_backend_status_with_config`, `install_wfp_network_backend`, ~1544-1675), the fail-closed `UnsupportedPlatform` error (~1644-1645), and service lifecycle command builders (`build_wfp_service_create_args` ~253-265, stop/delete ~1758-1759).
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — the `nono-wfp-service` Windows service (SCM registration, named pipe `\\.\pipe\nono-wfp-control`, startup orphan-filter sweep).
- `crates/nono-cli/src/windows_wfp_contract.rs` — `WfpRuntimeActivationRequest` / `WfpRuntimeActivationResponse` IPC contract (protocol v1).
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `prepare_network_enforcement` wiring for Direct vs Supervised (~337, 518 service-install export).
- `crates/nono-cli/src/setup.rs` — `install_windows_wfp_service` / `--start-wfp-service` entry points (~95-96, 192-204).

### MSI / packaging (the start=auto change)
- `dist/windows/*.wxs` — WiX source that registers the service via `ServiceInstall`/`ServiceControl`; this is where `start=demand → start=auto` lands. (NOTE per project state: the dev `.wxs` may still point at a stale `target\release` path — confirm current source before editing.)
- `crates/nono-cli/build.rs` — build-time MSI/service artifact handling (if relevant).

### Tests & related prior work
- `crates/nono-cli/tests/wfp_port_integration.rs` — existing WFP policy-compilation + real-TCP-through-filter tests (admin + running-service gated, `#[ignore]`).
- `.planning/ROADMAP.md` Phase 53 §REQ-DRN-01 + debug `wfp-service-stop-uninstall` — the elevated WFP service stop / `msiexec /x` uninstall leaves-nothing work (start=auto must not regress clean uninstall).
- `.planning/PROJECT.md` "Out of scope" — Gap 6b (kernel minifilter) v3.0 deferral; confirms D-05.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `nono-wfp-service` + `WfpRuntimeActivationRequest` IPC: fully functional; this phase changes *when/how the service is running*, not the activation protocol.
- `build_wfp_service_create_args` (network.rs ~253-265): the single place the `start=demand` posture is set — flip point for D-01 (or the equivalent WiX `ServiceInstall` for the MSI-installed case).
- `nono setup --start-wfp-service` / `install_windows_wfp_service` (setup.rs): existing elevated start path — reuse for the D-03 auto-start attempt and the D-02 manual path.

### Established Patterns
- **Fail-secure, never silent-degrade** (CLAUDE.md): the existing `UnsupportedPlatform` hard-fail is the correct baseline; D-03 only *adds* an auto-start attempt + better remediation, it must not weaken the no-pass-through guarantee.
- **Elevation gating**: `sc start` / service install require admin; setup.rs already checks elevation — supervised runs typically are NOT elevated, which is exactly why D-01 (boot-start) is the primary mechanism.
- **Clean uninstall invariant** (Phase 53 / REQ-DRN-01): `sc stop` + `msiexec /x` must leave nothing behind — a `start=auto` service must still uninstall cleanly.

### Integration Points
- WiX `.wxs` service registration ↔ runtime `select_network_backend`/probe in `network.rs`.
- Supervised/broker path (`config.session_sid.is_some()`) → `prepare_network_enforcement` → WFP service IPC (already wired; verify the auto-start hook sits before the probe).

</code_context>

<specifics>
## Specific Ideas

- Human-UAT acceptance (the proof this phase works): on a real elevated Windows 11 host, after installing the machine MSI, a supervised `nono run` on the runner profile with `network.block:true` **denies the confined child's outbound network** while any explicitly allowed ports still pass — with **no manual `nono setup --start-wfp-service`** beforehand. This is the inverse of the Phase 60 workaround (which had to set `network.block:false`).
- The phase title says "kernel" — note in planning that WFP-via-service is *already* kernel-enforced; the deliverable is operational reliability, not a new kernel layer.

</specifics>

<deferred>
## Deferred Ideas

- **Per-process / AppID filter scoping** — scoping WFP filters to the confined child executable (not just `session_sid`). A real net-new capability; its own phase if pursued.
- **`nono-wfp-driver.sys` real kernel minifilter (Gap 6b)** — v3.0-deferred per PROJECT.md; requires a signed kernel driver.
- **Fine-grained `allow_domain` path/method filtering** — that is Phase 56 (REQ-NET-01), proxy/application layer, distinct from WFP kernel blocking.

### Reviewed Todos (not folded)
- `2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md` ("Runner profile must deny ~/.claude regardless of --allow-cwd", score 0.6) — **not folded.** It is a *filesystem* deny-policy concern for the runner profile, orthogonal to WFP network enforcement. Belongs with runner-profile/tool-mediation work, not this network phase.

</deferred>

---

*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Context gathered: 2026-06-02*
