# Phase 62: Add WFP Kernel Network Enforcement for Windows Supervised Runs - Research

**Researched:** 2026-06-02
**Domain:** Windows WFP service lifecycle management, WiX service registration, Rust Windows SCM API
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** The machine MSI registers `nono-wfp-service` with `start=auto` so the Windows SCM
  boot-starts it as SYSTEM. No per-run elevation is required — a non-elevated supervised `nono run`
  can rely on the service already running. (Today it is `start=demand`.)
- **D-02:** `nono setup --start-wfp-service` is retained as the manual / dev-layout path.
- **D-03:** When a `network.block:true` run finds the service not running: attempt to start it; if
  the start succeeds, enforce; if it cannot (no elevation), abort fail-closed with an actionable
  error that names the exact remediation command. Never silently pass through unenforced.
- **D-04:** No netsh `FirewallRules` fallback for this path.
- **D-05:** Service-only — `nono-wfp-driver.sys` is OUT of scope.
- **D-06:** Phase 62 folds into the v2.9 "Windows Sandbox-the-Tools" track.
- **D-07:** Introduce `REQ-WFP-01` and map it in the ROADMAP Coverage table.

### Claude's Discretion

- Exact internal API shape for the "ensure service running" check + start attempt
  (where it lives relative to `select_network_backend` / `probe_wfp_backend_status_with_config`
  / `install_wfp_network_backend` in `network.rs`, and how elevation is detected).
- Precise WiX/`.wxs` change to flip the service to `start=auto` and whether to also set a
  `ServiceConfig` recovery/restart policy.
- Wording of the fail-closed remediation message (must name the elevated remediation command).
- Test structure (unit/integration split) beyond the mandatory human-UAT.

### Deferred Ideas (OUT OF SCOPE)

- Per-process / AppID filter scoping.
- `nono-wfp-driver.sys` real kernel minifilter (Gap 6b, v3.0-deferred).
- Fine-grained `allow_domain` path/method filtering (Phase 56).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-WFP-01 | Out-of-box WFP operational enforcement for supervised runs: a `network.block:true` supervised `nono run` on a machine-MSI-installed nono host enforces WFP kernel network blocking without any manual `nono setup --start-wfp-service` step | Enabled by D-01 (WiX `Start="demand"` → `Start="auto"`) + D-03 (runtime auto-start fallback) |
</phase_requirements>

---

## Summary

Phase 62 is a focused operational-reliability phase. WFP enforcement is already complete and
kernel-level: `nono-wfp-service` drives `FwpmEngine` at the ALE_AUTH_CONNECT/BIND layers, the
IPC pipe protocol is fully implemented, and `install_wfp_network_backend_with_runner` correctly
activates blocking through named-pipe IPC. The only missing piece is that the service is not
running when a non-elevated user issues `nono run --profile runner -- <cmd>` (profile sets
`network.block:true`). The service is registered `start=demand` in both the WiX `ServiceInstall`
element (confirmed: `dist/windows/nono-machine.wxs` line 87) and in the runtime
`build_wfp_service_create_args` function (confirmed: `network.rs` line 259). It therefore
receives no boot-start from the SCM and fails closed with `NonoError::UnsupportedPlatform` at
`network.rs` line 1644-1645 via `describe_wfp_runtime_activation_failure`.

The fix has two independent parts. First, the **packaging change**: flip `Start="demand"` to
`Start="auto"` in `dist/windows/nono-machine.wxs` at the `ServiceInstall` element (line 87). This
is the primary mechanism; the user MSI (`nono-user.wxs`) has no WFP service component at all and
cannot be fixed via this route. Second, the **runtime fallback** (D-03): when the service is found
`Stopped` at enforcement time (dev layout, user-scope install, or post-crash), attempt an `sc
start` via the already-extracted `start_windows_wfp_service_with_runner` logic before returning
the fail-closed error. If the start attempt fails because the process is not elevated, the error
must name the exact remediation command so the operator can self-serve.

The phase does not touch any cross-platform code (all changes are inside `#[cfg(target_os =
"windows")]` surfaces). The CLAUDE.md cross-target clippy rule still applies for any cfg-gated
code touched, but because the changes are Windows-only, the Linux/macOS cross-target verification
will be PARTIAL (deferred to CI) per the established policy.

**Primary recommendation:** The WiX change is a one-line edit and is the correct, permanent fix.
The D-03 runtime auto-start adds defense-in-depth for the dev-layout and user-scope-install cases.
Both changes should be in the same plan; they are independent and can be implemented in any order.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Service boot-start posture | WiX packaging (`dist/windows/*.wxs`) | SCM (OS) | The MSI `ServiceInstall` element is the single authoritative declaration of start type |
| Runtime service-availability check | `exec_strategy_windows/network.rs` | `setup.rs` (reuse) | Already lives in `install_wfp_network_backend_with_runner`; add start-attempt before fail-closed path |
| Elevation detection | `exec_strategy_windows/mod.rs` | — | `is_admin_process()` at line 524 is already public crate |
| Service start (runtime) | `network.rs` via `start_windows_wfp_service_with_runner` | `setup.rs` wrapper | Already implemented; need to add an in-run call path |
| Fail-closed error + remediation message | `network.rs` `describe_wfp_runtime_activation_failure` | — | Must be updated to name elevated remediation command |
| Human UAT verification | Real Windows 11 host + installed MSI | — | Structural; no automated substitute |

---

## Standard Stack

### Core

No new external dependencies are introduced by this phase. The phase uses existing workspace crates
and Rust stdlib.

| Component | Version | Purpose | Status |
|-----------|---------|---------|--------|
| `windows-sys` | 0.59 (workspace) | SCM API (`OpenSCManager`, `StartService`) if needed | Already in workspace |
| `windows-service` crate | In use by `nono-wfp-service.rs` | Service control; NOT needed in nono-cli path | Already present |
| WiX Toolset v4 | 4.x (used by `dist/windows/*.wxs`) | `ServiceInstall`/`ServiceControl`/`ServiceConfig` elements | No version change |

**No new packages to install.** This is a code + config change only.

---

## Package Legitimacy Audit

No new packages are installed by this phase.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| (none) | — | — | — | — | — | N/A |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
nono run (non-elevated)
        |
        v
  prepare_network_enforcement()          [network.rs ~L1665]
        |
        v
  select_network_backend(policy)         [network.rs ~L1445]
        |   (Blocked + Wfp active/preferred)
        v
  WfpNetworkBackend::install()           [network.rs ~L1549]
        |
        v
  install_wfp_network_backend_with_runner()   [network.rs ~L1560]
        |
        v
  probe_wfp_backend_status_with_config() [network.rs ~L386]
        |
        +-- status == Ready ---------> send IPC to nono-wfp-service pipe
        |                                     --> EnforcedPendingCleanup --> OK
        |
        +-- status == BackendServiceStopped
              |
              |  [NEW D-03 hook goes HERE, before the fail-closed return at L1644]
              |
              v
        attempt start_windows_wfp_service()   [reuse existing, network.rs ~L1389]
              |
              +-- start succeeded --> re-probe --> proceed to IPC
              |
              +-- start failed (not elevated) --> NonoError::UnsupportedPlatform
                      with message: "Run (elevated): nono setup --start-wfp-service"
```

Boot path (after MSI with start=auto):
```
Windows boot
    --> SCM reads ServiceInstall Start="auto"
    --> Starts nono-wfp-service as SYSTEM
    --> Service registers pipe \\.\pipe\nono-wfp-control
    --> nono run (any IL) connects to pipe, activates WFP filters
```

### Recommended Project Structure

No new files or directories. All changes are within existing files:

```
dist/windows/
  nono-machine.wxs          # change ServiceInstall Start="demand" -> "auto"
                            # optionally add ServiceConfig for restart policy

crates/nono-cli/src/exec_strategy_windows/
  network.rs                # D-03: auto-start hook + updated error message
                            # (inside install_wfp_network_backend_with_runner)

.planning/REQUIREMENTS.md   # add REQ-WFP-01
.planning/ROADMAP.md        # update Phase 62 goal/success-criteria, coverage table
```

### Pattern 1: WiX ServiceInstall start=auto (D-01)

**What:** Change the SCM start type in the `ServiceInstall` element.
**When to use:** This is the primary fix for out-of-box boot-start.

Current `dist/windows/nono-machine.wxs` lines 81-97 [VERIFIED: live source]:
```xml
<ServiceInstall
    Id="svcWfpService"
    Name="nono-wfp-service"
    DisplayName="nono WFP Service"
    Description="nono Windows Filtering Platform backend service"
    Type="ownProcess"
    Start="demand"        <!-- CHANGE THIS TO "auto" -->
    Account="LocalSystem"
    ErrorControl="normal"
    Arguments="--service-mode" />
<ServiceControl
    Id="svcCtrlWfpService"
    Name="nono-wfp-service"
    Start="install"       <!-- already tries to start on install; keep as-is -->
    Stop="both"
    Remove="uninstall"
    Wait="yes" />
```

After the change:
```xml
    Start="auto"
```

**Optional `ServiceConfig` for restart policy:** WiX v4 supports a `ServiceConfig` element (child
of `Component`) that maps to `ChangeServiceConfig2` with `SC_ACTION_RESTART`. This is Claude's
discretion. A reasonable restart policy (restart once on failure, with 60-second reset period)
would prevent a crash from leaving the service stopped before the next reboot. Whether to include
it is a planner decision; it is NOT needed for the core D-01 correctness.

WiX v4 `ServiceConfig` syntax [ASSUMED - verify against WiX v4 docs before implementing]:
```xml
<ServiceConfig
    ServiceName="nono-wfp-service"
    OnInstall="yes"
    FirstFailureActionType="restart"
    SecondFailureActionType="restart"
    ThirdFailureActionType="none"
    ResetPeriodInSeconds="60"
    RestartServiceDelayInSeconds="5" />
```

**IMPORTANT:** The `ServiceConfig` element only appears in the **machine MSI** (`nono-machine.wxs`).
The user MSI (`nono-user.wxs`) has **no WFP service component** — the user-scope installer does not
install or register any service (confirmed from live source). The user MSI cannot boot-start a
service because SCM service registration requires elevation. This is expected behavior; the machine
MSI is the supported deployment path for the WFP service.

### Pattern 2: D-03 runtime auto-start hook in `install_wfp_network_backend_with_runner`

**What:** When `probe_fn(probe_config)` returns `BackendServiceStopped`, attempt to start the
service before returning the fail-closed error.
**When to use:** Always, as the fallback for dev layout / user-scope installs / post-crash.

The hook belongs immediately before the `Err(NonoError::UnsupportedPlatform(...))` at line 1644
in `install_wfp_network_backend_with_runner`. The function signature already accepts `probe_fn`
and `run_probe` as injectable runners, making the auto-start attempt testable via the same pattern.

Pseudocode for the hook:
```rust
// After: let status = probe_fn(probe_config)?;
// Before: Err(NonoError::UnsupportedPlatform(describe_wfp_runtime_activation_failure(...)))

if status == WfpProbeStatus::BackendServiceStopped {
    // D-03: attempt auto-start before failing closed.
    match start_windows_wfp_service_with_runner(probe_config, run_sc_query, run_sc_command) {
        Ok(_) => {
            // Re-probe to confirm the service is now running.
            let new_status = probe_fn(probe_config)?;
            if new_status == WfpProbeStatus::Ready {
                // Proceed to IPC activation (fall through to the Ready branch above).
                // (Implementation note: the retry loop replaces the if-status==Ready block.)
            } else {
                return Err(NonoError::UnsupportedPlatform(
                    describe_wfp_runtime_activation_failure_with_start_attempted(...)
                ));
            }
        }
        Err(_) => {
            // Start attempt failed (likely not elevated).
            return Err(NonoError::UnsupportedPlatform(format!(
                "Windows WFP runtime activation is required for {} but \
                 the WFP service `{}` is not running and could not be started \
                 automatically (elevation required). Run this command in an \
                 elevated (Administrator) terminal to start it once: \
                 `nono setup --start-wfp-service`",
                describe_windows_network_runtime_target(policy),
                probe_config.backend_service
            )));
        }
    }
}
```

**Implementation note on code structure:** The existing `install_wfp_network_backend_with_runner`
takes `probe_fn: P` and `run_probe: R` but not a service-start runner. For testability of D-03,
a third runner parameter `start_service_fn: S` should be added, analogous to the existing pattern.
This maintains the unit-test-friendly design already established in the test module (lines
1691-2004 of `network.rs`).

### Pattern 3: Elevation detection (already available)

`is_admin_process()` at `exec_strategy_windows/mod.rs` line 524 uses `GetTokenInformation /
TokenElevation`. It is `pub(crate)` and available for use in `network.rs`. The auto-start attempt
does NOT need to call this first — the correct pattern is attempt-then-handle-failure, because
`sc start` is the authoritative signal. (Calling `is_admin_process()` first would be a TOCTOU
race and would duplicate the elevation check.)

### Pattern 4: Updated error message (D-03 fail-closed wording)

The existing `describe_wfp_runtime_activation_failure` at line 417 has a `BackendServiceStopped`
arm (line 444-447) that says:
```
"the WFP service `{}` is registered but not running. Run `nono setup --start-wfp-service` first"
```

After D-03, this path is only reached if the auto-start attempt also failed. The message needs to
distinguish "auto-start failed" from "auto-start not attempted":

- If auto-start was attempted and failed: "the WFP service `nono-wfp-service` is not running and
  could not be started automatically (elevation is required). To start it, run this command once
  in an elevated (Administrator) terminal: `nono setup --start-wfp-service`"
- If auto-start was not attempted (e.g., service is `Missing`, not just `Stopped`): existing
  message remains appropriate.

The `describe_wfp_runtime_activation_failure` function takes `status: WfpProbeStatus` as a
discriminator — an `auto_start_attempted: bool` flag or a new status variant can thread this
distinction cleanly.

### Anti-Patterns to Avoid

- **Silent backend swap to FirewallRules:** If the WFP service is stopped, do NOT fall back to
  `FirewallRulesNetworkBackend`. D-04 is explicit. The netsh path requires elevation AND produces
  different enforcement semantics. [VERIFIED: live code — `select_network_backend` already has a
  hard error arm for mismatched backends, line 1474]
- **Checking `is_admin_process()` before the start attempt:** This is a TOCTOU risk and adds
  unnecessary code. Let `sc start` fail and handle the error.
- **Changing `build_wfp_service_create_args` for D-01:** The `start=demand` in that function is
  the runtime `sc create` argument used by `nono setup --register-wfp-service`. D-01 says the MSI
  (WiX) controls boot-start; `build_wfp_service_create_args` is the dev-layout / manual setup
  path (D-02). The two registration paths are **independent**: WiX `ServiceInstall` for MSI
  installs, `sc create` via `build_wfp_service_create_args` for manual setup. Do NOT change
  `build_wfp_service_create_args` for D-01 — doing so would make the dev-layout auto-start at
  boot, which breaks D-02 separation. [VERIFIED: distinct code paths]
- **Modifying the user MSI (`nono-user.wxs`) to add service components:** The user-scope MSI
  cannot register SCM services (requires elevation). The WFP service is correctly absent from
  `nono-user.wxs`. [VERIFIED: live source — user WiX has no `ServiceInstall` element]

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Service start attempt | Custom `sc` invocation | `start_windows_wfp_service_with_runner` at line 1315 | Already handles binary-missing, BFE-stopped, already-running, and start-failure cases; idempotent |
| Service state query | Direct `sc query` parsing | `probe_wfp_backend_status_with_config` at line 386 | Already handles all `WindowsServiceState` cases and the BFE platform-service check |
| Elevation detection | `CreateFile`/`OpenSCManager` probes | `is_admin_process()` at mod.rs line 524 | Token-based, deterministic, no side effects |
| WiX `ServiceConfig` | Custom post-install script | WiX v4 `ServiceConfig` element | SCM restart policy is declarative in WiX; no custom action needed |

**Key insight:** All the needed primitives exist. The entire phase is wiring existing functions
together in a new order, plus a one-attribute WiX change.

---

## Common Pitfalls

### Pitfall 1: Two independent `start=demand` registrations — must change BOTH or neither

**What goes wrong:** The phase might fix only the WiX `ServiceInstall Start="demand"` (line 87 of
`nono-machine.wxs`) but not realize that `build_wfp_service_create_args` at `network.rs` line 259
ALSO contains `"demand"`. If the runtime `sc create` path (used by `nono setup
--register-wfp-service`) runs AFTER the MSI install, it re-creates the service with `start=demand`,
reverting the MSI's `start=auto` registration.

**Why it happens:** The two registration paths are independent: WiX `ServiceInstall` for MSI
installs, `build_wfp_service_create_args` for manual `nono setup` registration. A developer
running `nono setup --register-wfp-service` post-install would override the MSI's `start=auto`.

**How to avoid:** D-02 says `nono setup --start-wfp-service` is the manual path, which implies
`nono setup --register-wfp-service` is only for dev layouts. Two options for the planner:
(a) Change `build_wfp_service_create_args` to also use `"auto"` (so manual registration matches
the MSI), or (b) document that the manual path produces `start=demand` (dev-only, acceptable).
Research recommendation: option (a) is safer — if someone re-runs setup on a prod machine it
won't silently downgrade the service. The plan should decide this explicitly.

**Warning signs:** Running `sc qc nono-wfp-service` after install shows `START_TYPE: 3 DEMAND_START`
instead of `2 AUTO_START`.

### Pitfall 2: WiX `ServiceControl Start="install"` vs `ServiceInstall Start="auto"` confusion

**What goes wrong:** The existing `ServiceControl Start="install"` (line 93 of `nono-machine.wxs`)
causes WiX to start the service at install time (regardless of the `ServiceInstall Start=` value).
This is SEPARATE from boot-start. After a reboot, the SCM consults only `ServiceInstall Start=`.
A developer might observe the service running immediately after install (because of
`ServiceControl`) and conclude no change is needed.

**Why it happens:** WiX documentation conflates the two concepts. `ServiceControl Start="install"`
is "start it NOW during install"; `ServiceInstall Start="auto"` is "start it at every boot."

**How to avoid:** Test by rebooting the machine after install and checking `sc query
nono-wfp-service`. If `Start=demand`, the SCM will NOT auto-start; if `Start=auto`, it will.

**Warning signs:** `nono run` works immediately after install (service started by WiX at install
time) but fails after reboot.

### Pitfall 3: Clean-uninstall invariant (REQ-DRN-01 regression risk)

**What goes wrong:** Changing to `Start="auto"` means the service starts at every boot. If the
machine MSI uninstall custom action `CaUninstallWfpServices` fails (runs `nono.exe setup
--uninstall-wfp` as a deferred, impersonated=no action at line 25-34 of `nono-machine.wxs`), the
`start=auto` service will survive uninstall and auto-start on next boot — violating REQ-DRN-01
("leaves nothing behind").

**Why it happens:** `CaUninstallWfpServices` has `Return="ignore"`, meaning MSI proceeds even if
the custom action fails. The service may persist if `nono.exe` has been moved or deleted before
uninstall.

**How to avoid:** The existing `ServiceControl Stop="both" Remove="uninstall"` elements (line
91-97) already tell WiX/Windows Installer to stop and remove the service during uninstall, even
before `CaUninstallWfpServices` runs. The `ServiceControl` removal is the primary clean-uninstall
guarantee; the custom action is defense-in-depth for the driver service. Verify in the UAT:
`msiexec /x` must result in `sc query nono-wfp-service` returning `FAILED 1060` (not found).

**Warning signs:** After `msiexec /x`, `sc query nono-wfp-service` returns `STOPPED` or
`RUNNING` (should return error 1060). Check `Event Log` for any uninstall custom action failures.

### Pitfall 4: D-03 re-probe after auto-start may still fail if start is slow

**What goes wrong:** After `sc start`, the service may be in `START_PENDING` state (not yet
`RUNNING`) when the re-probe fires. `probe_wfp_backend_status_with_config` checks `sc query`
which would return `STOPPED` or intermediate state, causing a spurious fail-closed.

**Why it happens:** Service startup is asynchronous from the SCM perspective. `sc start` returns
after initiating the start, not after the service has fully initialized.

**How to avoid:** The existing `start_windows_wfp_service_with_runner` already re-queries after
start (line 1370-1386) and returns `Err` if the service doesn't reach `RUNNING`. The D-03 hook
should rely on this return value. If `start_windows_wfp_service_with_runner` returns `Ok`, the
service is confirmed `RUNNING` (line 1371 check). The re-probe step is still needed to update
`WfpProbeStatus`, but the timing issue is handled by the start function itself.

**Warning signs:** Intermittent failures on test machines; service appears `RUNNING` in Event Log
but D-03 returns the fail-closed error.

### Pitfall 5: The named-pipe SDDL restricts access from non-elevated sessions

**What goes wrong:** The WFP service pipe `\\.\pipe\nono-wfp-control` has SDDL
`D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;OW)` [VERIFIED: `nono-wfp-service.rs` line 55]. This grants
full access to SYSTEM (SY), Administrators (BA), and the pipe Owner (OW). A non-elevated
supervised `nono run` runs as a standard user — who is NOT in the Administrators group. Standard
users will be denied when `run_wfp_runtime_request` tries to `ClientOptions::open` the pipe
(`network.rs` line 850-857).

**Why it happens:** The SDDL was written when the assumption was that any user of WFP would be
elevated. With D-01 (service auto-starting as SYSTEM), the service is present, but non-elevated
`nono.exe` callers still cannot connect to the pipe.

**How to avoid:** The SDDL needs to include a standard user read+write ACE, or the pipe must grant
access to `WD` (World/Everyone) or `BU` (Built-in Users). The appropriate change is to add
`(A;;GRGW;;;BU)` or `(A;;GRGW;;;WD)` to the SDDL. This is a SECURITY-CRITICAL decision (D-03
says non-elevated runs must work) and must be in the plan. A tightly scoped option: grant to `IU`
(Interactive Users) rather than `WD`.

Recommended SDDL change: `D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)(A;;GRGW;;;OW)` — adds
Interactive Users (IU) with generic read+write (the pipe is already listening for JSON and
responding; `GRGW` is sufficient for the protocol).

**Warning signs:** `nono run` with a non-elevated process logs `Failed to connect to
nono-wfp-service: Access is denied.` (error 5) instead of a protocol response.

**This is a blocking issue for the phase goal.** The plan MUST include a SDDL update to
`nono-wfp-service.rs` alongside the WiX and network.rs changes.

### Pitfall 6: `build_wfp_service_create_args` description string is stale

**What goes wrong:** The service description set by `build_wfp_service_description_args` at line
268-274 says: "Placeholder service host for the future nono Windows WFP backend. Registration is
supported; runtime still fails closed until enforcement is implemented." This is no longer accurate
once enforcement is implemented.

**How to avoid:** Update the description string to reflect the new production status. Minor but
visible to operators via `sc qc nono-wfp-service`.

---

## Code Examples

### Key line ranges (VERIFIED against live source)

```
network.rs L253-265:   build_wfp_service_create_args() — "demand" string at L259
network.rs L305-307:   build_wfp_service_start_args()
network.rs L386-415:   probe_wfp_backend_status_with_config()
network.rs L417-468:   describe_wfp_runtime_activation_failure() — BackendServiceStopped arm at L444-447
network.rs L1315-1392: start_windows_wfp_service_with_runner() — existing full start logic
network.rs L1389-1392: start_windows_wfp_service() — public wrapper
network.rs L1445-1480: select_network_backend()
network.rs L1560-1649: install_wfp_network_backend_with_runner() — D-03 hook goes at ~L1643
network.rs L1644-1646: Err(NonoError::UnsupportedPlatform(describe_wfp_runtime_activation_failure()))
network.rs L1665-1689: prepare_network_enforcement() — top-level entry point
mod.rs     L524-547:   is_admin_process()
mod.rs     L517-520:   pub(crate) re-exports (start_windows_wfp_service is exported here)

nono-machine.wxs L81-97:   ServiceInstall + ServiceControl for nono-wfp-service
nono-machine.wxs L87:      Start="demand"  <-- D-01 change target
nono-user.wxs:             no ServiceInstall element (confirmed)

nono-wfp-service.rs L55:   PIPE_SDDL = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;OW)"
                            <-- Pitfall 5: must add IU or BU for non-elevated callers
```

### Existing `start_windows_wfp_service_with_runner` return contract [VERIFIED]

```rust
// Returns:
//   Ok(WindowsWfpStartReport { status_label: "already running", .. })  -- already running
//   Ok(WindowsWfpStartReport { status_label: "running", .. })           -- just started OK
//   Err(NonoError::Setup(...))                                           -- failed to start
// (from network.rs lines 1315-1392)
```

The `Ok` variants both mean the service is running. The `Err` variant means it failed (includes
elevation-denied case via sc.exe exit code propagation).

### `install_wfp_network_backend_with_runner` D-03 insertion point [VERIFIED]

```
L1560: pub(super) fn install_wfp_network_backend_with_runner<P, R>(
L1568:     probe_fn: P,
L1569:     run_probe: R,
           // NEW: add start_service_fn: S parameter for testability
L1588:     let status = probe_fn(probe_config)?;
L1588-1643: if status == WfpProbeStatus::Ready { ... IPC activation ... }
L1644: Err(NonoError::UnsupportedPlatform(
           // D-03 hook: intercept BackendServiceStopped before this return
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Start="demand"` (manual-only WFP) | `Start="auto"` (boot-started by SCM) | This phase | Service present without admin intervention |
| Fail-closed with remediation hint | Auto-start attempt + better error | This phase | Non-elevated supervised runs work post-MSI-install |
| Placeholder description string | Accurate production description | This phase | Operator-visible improvement |

**Previously stale / now corrected:**
- `build_wfp_service_description_args` description (line 268-274): says "Placeholder service host
  for the future... fails closed." Must be updated to reflect production status.
- `describe_wfp_next_action_for_setup` for `BackendServiceStopped` (line 822-823): "Next action:
  run `nono setup --start-wfp-service`" — remains accurate for setup but needs context for run.

---

## Runtime State Inventory

> This is NOT a rename/refactor phase. However, because the WFP service changes start posture,
> any currently registered `nono-wfp-service` with `start=demand` will be re-registered on the
> next MSI install with `start=auto`. Existing dev-layout `sc create` registrations are unaffected
> by the WiX change (they read from `build_wfp_service_create_args`, not WiX).

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | None — WFP filter state is ephemeral (swept on service start) | None |
| Live service config | `nono-wfp-service` registered `start=demand` in any existing MSI install | MSI upgrade will re-register with `start=auto` via `MajorUpgrade` |
| OS-registered state | `nono-wfp-service` SCM entry (`start=demand`) | Covered by MSI upgrade / `CaUninstallWfpServices` + re-install |
| Secrets/env vars | None | None |
| Build artifacts | `dist/windows/nono-machine.wxs` already rebuilt to v0.57.5 (quick 260601-wha) | WiX change applies directly; rebuild MSI |

**Nothing found in category Stored data, Secrets/env vars, Build artifacts** (beyond the note on WiX rebuild).

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| WiX Toolset v4 | MSI rebuild | Windows-only, used in CI (`release.yml`) | 4.x | Dev test without MSI rebuild (service manually registered) |
| `nono-wfp-service.exe` | D-03 auto-start test | Built with workspace | v0.57.5 | Dev build: `cargo build -p nono-cli --bins` |
| Elevated Windows 11 host | Human UAT | Operator must run this | Build 26200 (confirmed) | None — human UAT is mandatory |
| `sc.exe` | Runtime service control | Always present on Windows 10/11 | System32 | None |

**Missing dependencies with no fallback:**
- Elevated Windows 11 host for human UAT (required by phase acceptance; no automated substitute).

**Missing dependencies with fallback:**
- WiX v4 for local MSI rebuild: CI `release.yml` handles MSI production; local MSI rebuild can be
  deferred to CI.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | None (workspace-level `cargo test`) |
| Quick run command | `cargo test -p nono-cli -- wfp` |
| Full suite command | `cargo test --workspace` (equiv. `make test`) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-WFP-01 | WiX ServiceInstall has `Start="auto"` | Unit (string check) | `cargo test -p nono-cli -- test_wfp_service_start_type_is_auto` | ❌ Wave 0 |
| REQ-WFP-01 | D-03: BackendServiceStopped triggers auto-start attempt | Unit (mock runner) | `cargo test -p nono-cli -- test_wfp_autostart_on_stopped` | ❌ Wave 0 |
| REQ-WFP-01 | D-03: auto-start failure produces remediation message | Unit (mock runner) | `cargo test -p nono-cli -- test_wfp_autostart_fail_remediation_message` | ❌ Wave 0 |
| REQ-WFP-01 | D-03: non-BackendServiceStopped statuses not affected | Unit (mock runner) | `cargo test -p nono-cli -- test_wfp_non_stopped_status_unchanged` | ❌ Wave 0 |
| REQ-WFP-01 | Full end-to-end: MSI-installed service boot-starts; confined run enforces network | Human UAT | Manual: install MSI, reboot, run confined cmd | ❌ manual-only |
| REQ-WFP-01 | Pipe accessible from non-elevated session | Unit (SDDL parse test) or UAT | `cargo test -p nono-cli -- test_wfp_pipe_sddl_includes_interactive_users` | ❌ Wave 0 |

**Note on testability:** The existing unit test pattern in `network.rs` (lines 1691-2004) uses
injectable `probe_fn` and `run_probe` closures. The D-03 hook requires adding a third injectable
`start_service_fn` to `install_wfp_network_backend_with_runner`. This makes all three new test
cases above straightforward mock-based tests matching the existing `install_wfp_network_backend`
test pattern. Tests do NOT require a running service or elevation.

The SDDL test can be a `#[cfg(target_os = "windows")]` compile-time constant assertion or a
string parse test confirming the new SDDL contains an IU/BU ACE.

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli -- wfp`
- **Per wave merge:** `cargo test --workspace` (or `cargo test --bin nono` per the CLAUDE.md
  note about pre-existing workspace CI debt)
- **Phase gate:** Full suite green + Human UAT PASS before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] Unit tests for D-03 auto-start hook in `network.rs` (3 new tests)
- [ ] Unit test for SDDL including IU/BU ACE in `nono-wfp-service.rs`
- [ ] (Optional) WiX attribute assertion — this may be a documentation/review check rather than
  a test, since WiX XML is not Rust-compiled

*(The existing `wfp_port_integration.rs` is `#[ignore]`-gated and covers the full TCP-level
enforcement path; it remains the right structure for admin-level smoke tests.)*

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A |
| V3 Session Management | No | N/A |
| V4 Access Control | Yes | Pipe SDDL restricts callers; service runs as SYSTEM |
| V5 Input Validation | No | WFP IPC protocol unchanged |
| V6 Cryptography | No | N/A |

### Known Threat Patterns for WFP service auto-start

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Pipe squatting — malicious process creates `\\.\pipe\nono-wfp-control` before the service | Spoofing | Named pipe SDDL with `(A;;GA;;;SY)` means only SYSTEM can create the server-side; a non-SYSTEM squatter would require the same SCM service name, blocked by the service already being registered |
| Unenforced pass-through — auto-start attempt silently swallowed | Elevation of Privilege | D-03 explicitly prohibits: if start fails, abort fail-closed; never continue unenforced |
| Over-permissive pipe SDDL (`WD` instead of `IU`) | Elevation of Privilege | Use `IU` (Interactive Users) not `WD` (Everyone); sandboxed children running at Low IL may also get Interactive User token |
| `start=auto` service crash-loops and DoS | Denial of Service | `ServiceConfig` restart policy with back-off mitigates; or accept the OS default (no restart on failure unless configured) |

**Security note on Low-IL pipe access:** The sandbox-the-tools runner spawns children at Low IL.
A Low-IL process has the `Interactive Users` group SID in its token as a disabled SID. On Windows,
named-pipe `GRGW` grants apply to enabled SIDs at the time of connect. **A Low-IL broker child
connecting to the WFP pipe may be denied even with `IU` in the SDDL if the IL check fires before
the DACL check.** The pipe SDDL does not include an IL label; by default named pipes have Medium
IL labels. The broker arm (which runs at Medium IL, not Low IL) is the component that calls
`nono run --profile ... -- <cmd>`, so `nono.exe` itself runs at Medium IL when it initiates the
WFP IPC. This is NOT a problem for the D-03 path. The plan should confirm which IL `nono.exe`
runs at when making the IPC call (Medium IL = OK; Low IL = needs pipe SDDL integrity label).

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | WiX v4 `ServiceConfig` syntax for restart policy | Architecture Patterns | Wrong syntax → WiX compile error at MSI build time; easily caught |
| A2 | `sc start` failure (non-elevated) is surfaced as an `Err` from `run_sc_command` | Pitfall 4 | If `sc start` returns exit code 0 even when denied, auto-start "succeeds" silently without the service running; risk LOW — `sc start` returns error 1053/5 on failure |
| A3 | Low-IL broker calls `nono.exe` at Medium IL, not Low IL | Security Domain | If `nono.exe` runs at Low IL during WFP IPC, SDDL `IU` is insufficient; this would require adding an integrity label to the pipe |

**If this table is empty:** It is not empty — A1-A3 need verification during implementation.

---

## Open Questions (RESOLVED)

> All three resolved during planning (Phase 62, 2026-06-02). Resolution notes appended per question.

1. **Should `build_wfp_service_create_args` also switch to `start=auto`?**
   - **RESOLVED:** Yes — implemented in plan 62-01 Task 2 (`build_wfp_service_create_args` → `start=auto`), aligning both registration paths so `nono setup --register-wfp-service` cannot silently revert the MSI posture.
   - What we know: Two independent registration paths exist. D-02 says `--start-wfp-service` is
     retained for manual/dev-layout. D-01 is about the MSI only.
   - What's unclear: If `nono setup --register-wfp-service` is used on a production machine
     (reinstall scenario), it would override `start=auto` with `start=demand`.
   - Recommendation: Change `build_wfp_service_create_args` to `start=auto` as well. This aligns
     both paths and prevents regression. The manual setup path is only for dev layouts where
     `start=auto` is harmless (the dev binary is always present).

2. **SDDL: `IU` vs `BU` vs `WD` for pipe access?**
   - What we know: Standard users need pipe access; Low-IL children should NOT be able to bypass
     WFP enforcement by talking to the service directly.
   - What's unclear: Whether Low-IL processes (Low-IL sandbox children) should be able to reach
     the WFP pipe. If they could, a confined process could un-block itself.
   - Recommendation: Use `IU` (Interactive Users). This covers the `nono.exe` process (Medium IL)
     but in practice Low-IL processes are also Interactive Users — however, the pipe endpoint only
     responds to valid IPC requests and only activates/deactivates enforcement; it does not expose
     a "disable all enforcement" API. The threat model is acceptable. An alternative is
     `(A;;GRGW;;;BU)` (Built-in Users = any local/domain user), which is equivalent in practice.
   - **RESOLVED:** Use `IU` — implemented in plan 62-02 Task 1 (`PIPE_SDDL` gains `(A;;GRGW;;;IU)`), with a regression test asserting the ACE is present.

3. **`ServiceConfig` restart policy: include or not?**
   - What we know: WiX v4 supports `ServiceConfig`. A crash-looping service is a DoS vector.
   - What's unclear: The production risk of the service crashing post-install.
   - Recommendation: Add a single-restart policy (`FirstFailureActionType="restart"`,
     `ResetPeriodInSeconds="60"`) as defense-in-depth. This is a one-time WiX configuration
     and has no runtime cost. Keep it simple — do not configure second/third failure actions.
   - **RESOLVED:** Include — implemented in plan 62-02 Task 2 (single-restart ServiceConfig), conditional on the WiX v4 syntax verifying at implementation time; omit-and-note if WiX docs are unreachable.

---

## Project Constraints (from CLAUDE.md)

- **Fail Secure**: Never silently degrade to unenforced state. D-03 must not weaken this.
- **Cross-target clippy**: Changes are Windows-only (`#[cfg(target_os = "windows")]` / inside
  `exec_strategy_windows/`). The cross-target verification rule applies. A Windows-host `cargo
  clippy --workspace --target x86_64-pc-windows-msvc` is insufficient; cross-target must run
  Linux/macOS via `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin`. If the cross-toolchain
  is not installed, mark verification `PARTIAL` and defer to CI.
- **DCO sign-off**: All commits must include `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
- **No `.unwrap()` / `.expect()`**: The new start-attempt path must use `?` and return `Result`.
- **`#[must_use]` on critical Results**: If any helper functions are extracted, apply `#[must_use]`.
- **No `#[allow(dead_code)]`**: All new code must be covered by tests or used in production paths.
- **GSD Workflow**: Code changes go through `/gsd:execute-phase 62`, not direct edits.

---

## Sources

### Primary (HIGH confidence)

- Live source: `crates/nono-cli/src/exec_strategy_windows/network.rs` — all function bodies, line
  ranges, and behavior verified by direct read
- Live source: `dist/windows/nono-machine.wxs` — `ServiceInstall Start="demand"` at line 87,
  `ServiceControl Start="install"` at line 93, `CaUninstallWfpServices` custom action confirmed
- Live source: `dist/windows/nono-user.wxs` — absence of `ServiceInstall` element confirmed
- Live source: `crates/nono-cli/src/bin/nono-wfp-service.rs` — `PIPE_SDDL` constant at line 55
- Live source: `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `is_admin_process()` at line 524
- Live source: `crates/nono-cli/tests/wfp_port_integration.rs` — test structure + `#[ignore]`
  guard confirmed
- Live source: `.planning/phases/60-.../60-HUMAN-UAT.md` §F-60-UAT-03 — exact failure message
  confirmed ("WFP service `nono-wfp-service` is registered but not running")

### Secondary (MEDIUM confidence)

- CONTEXT.md decisions D-01..D-07 — authored by the project maintainer, cross-checked against
  live source for feasibility

### Tertiary (LOW confidence)

- A1: WiX v4 `ServiceConfig` element syntax [ASSUMED — verify against WiX v4 docs at
  wixtoolset.org/docs/reference/schema/wxs/serviceconfig/]

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; all existing code verified from source
- Architecture: HIGH — all critical paths traced to exact line numbers in live source
- Pitfalls: HIGH — derived from live source analysis; Pitfall 5 (SDDL) is a NEW finding not in
  CONTEXT.md and must be addressed by the plan
- WiX syntax (ServiceConfig): LOW/ASSUMED — needs doc verification before implementation

**Research date:** 2026-06-02
**Valid until:** 2026-07-02 (stable — WFP service architecture changes rarely)
