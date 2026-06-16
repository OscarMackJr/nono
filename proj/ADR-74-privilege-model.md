# ADR-74: Privilege Model for nono-agentd (Persistent Multi-Tenant Daemon)

**Status:** Accepted
**Phase:** 74 — Persistent Multi-Tenant Daemon
**Date:** 2026-06-14
**Authors:** Phase 74 planning

---

## Context

The nono project already ships one Windows service: `nono-wfp-service`. That service runs as
**LocalSystem** (SYSTEM-level privilege) because it must call `FwpmFilterAdd0` and other WFP
kernel-mode filter APIs that only the elevated system account can exercise. Its service type is
`SERVICE_WIN32_OWN_PROCESS` in the machine-wide SCM.

Phase 74 introduces a second service: the **agent daemon** (`nono-agentd`). Its job is different:
launch and confine multiple concurrent AI agents over a single multi-tenant capability pipe,
track per-agent lifecycle, and serve capability requests to each tenant. It has no WFP
responsibility in Phase 74 (per D-04; per-agent WFP egress is Phase 75 / SUPP-02).

**Pitfall 4 (the problem this ADR exists to prevent):** If the daemon were to run as SYSTEM — by
copying `nono-wfp-service.rs` without changing the account type — a confined Low-IL agent that
escapes containment could pivot to the daemon process and obtain SYSTEM-level access. From there,
it could read or modify other tenants' capability grants, inject into the elevated WFP service,
and break the isolation boundary for the entire host. This would make the daemon a single point
of catastrophic failure rather than a least-privilege service.

This ADR records the privilege model for `nono-agentd` **before** any service host code is
written (SC4 ordering gate). The decisions here are locked for Phase 74; deviations require a
new ADR or revision.

---

## Decisions

### Decision 1 — USER privilege, not SYSTEM; `SERVICE_USER_OWN_PROCESS`

The agent daemon registers and runs as a **per-user Windows service** using SCM service type
`SERVICE_USER_OWN_PROCESS`. It runs under the interactive user's session logon token — the same
security context as any other process the user launches — and never elevates. This is the key
divergence from `nono-wfp-service`, which uses `SERVICE_WIN32_OWN_PROCESS` and runs as
LocalSystem.

A `SERVICE_USER_OWN_PROCESS` service in Windows 10/11 is installed in the user's SCM namespace
(`HKCU\SYSTEM\CurrentControlSet\Services`) and does not require administrative elevation to
install or start. The `nono daemon install` CLI verb (Phase 74) handles registration; no machine
MSI component registers this service (the machine MSI owns the elevated WFP service only).

Consequence of this choice: An escaped Low-IL agent that reaches the daemon process gains only
USER-level access (the interactive user's token), not SYSTEM-level access. It cannot inject into
`nono-wfp-service`, cannot read another user's session, and cannot modify WFP filter rules.

**USER is the ceiling; confinement keeps agents below it.**

### Decision 2 — Split from `nono-wfp-service`; no shared binary

The agent daemon binary (`nono-agentd.exe`) is fully split from the elevated WFP service
binary (`nono-wfp-service.exe`). In Phase 74 the daemon does NOT import or call any WFP Win32
APIs (`FwpmFilterAdd0`, `FwpmEngineOpen0`, `FwpmFilterDeleteById0`, or any other `Fwpm*`
function). There is no IPC channel between `nono-agentd` and `nono-wfp-service` in Phase 74.

Sharing a binary or adding a daemon-to-WFP IPC channel would be an architectural change
requiring a new ADR and cross-target impact analysis. This is not permitted in Phase 74.

The split is maintained structurally: `nono-agentd.rs` is a second `[[bin]]` target in
`crates/nono-cli/Cargo.toml`. Its Cargo dependencies do NOT include the `windows-service` SCM
account constants used by the machine-wide service type.

### Decision 3 — Foreground fallback without panic

When the daemon binary is invoked outside of an SCM context (e.g., `nono daemon start
--foreground`, or a direct developer invocation), `service_dispatcher` will fail with
`ERROR_FAILED_SERVICE_CONTROLLER_CONNECT`. The binary MUST fall through to foreground /
on-demand mode in this case, **without panicking**. This is the dev and testing path and is
expected behavior. It mirrors the non-fatal posture in `nono-wfp-service.rs`.

The foreground mode runs the same multi-tenant accept loop as the service mode, just without
SCM lifecycle management (no `set_service_status` calls). Closing the terminal kills the daemon
and, via `KILL_ON_JOB_CLOSE`, kills all running agents — consistent with D-03.

### Decision 4 — query-only pipe; no escape hatch

The capability pipe (`\\.\pipe\nono-agentd-cap-<rendezvous>`) accepts capability QUERY requests
only. The daemon serves the agent's pre-authorized `CapabilitySet` in response to each query.

The pipe MUST NOT accept requests that expand a running agent's capabilities after launch. Once
an `AgentTenant` is inserted into the daemon's tenant registry at spawn time with its
`CapabilitySet`, that set is immutable for the lifetime of the agent. A wire request from an
agent to ADD capabilities to its own running `CapabilitySet` is denied unconditionally.

This is the **no escape hatch** invariant: confinement cannot be weakened by the agent itself
after the agent is running. It mirrors the library's `Sandbox::apply()` irreversibility principle
on Linux and macOS. Enforcement is in the daemon's `serve_frames` path: any wire operation code
that does not correspond to a read-only capability query returns an error response and closes the
pipe instance.

### Decision 5 — `SeImpersonatePrivilege` and the impersonation gate

The daemon's capability pipe accept loop uses `ImpersonateNamedPipeClient` (after
`ConnectNamedPipe`) to perform server-side client identity verification. This call requires
`SeImpersonatePrivilege` in the server process's token.

Interactive users and their services on Windows 10/11 have `SeImpersonatePrivilege` by default
as part of the "Service" logon rights grant. A `SERVICE_USER_OWN_PROCESS` daemon running under
the interactive user's token retains this privilege. **Assumption A1** (per 74-RESEARCH.md §Open
Questions): that this privilege is present in the per-user service token must be empirically
confirmed in the Wave 0 spike (`daemon_handle_baseline.rs`). The cross-tenant denial test
(`daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance`) exercises the
`ImpersonateNamedPipeClient` path; if it passes, the privilege is confirmed present.

If A1 is falsified (privilege absent in the per-user service token), the fallback mechanism is:
SDDL-only admission (`build_capability_pipe_sddl` with the per-tenant AppContainer package SID)
plus `GetNamedPipeClientProcessId` + `IsProcessInJob` double-check. The SDDL gate remains the
primary isolation mechanism regardless.

`ImpersonateNamedPipeClient` is always followed by `RevertToSelf` on BOTH the success and error
paths. A RAII guard (`ImpersonationGuard`) enforces this in the production accept loop (see
RESEARCH.md Pitfall 3: thread identity leak).

---

## Consequences

1. **Binary artifact:** `nono-agentd.exe` is a second `[[bin]]` target in `crates/nono-cli/Cargo.toml`.
   It is NOT listed alongside `nono-wfp-service.exe` in the machine-scoped `nono-machine.wxs` MSI.
   Installation is via `nono daemon install` (Phase 74 CLI verb) or a future per-user MSI (Phase 75).

2. **Attack surface reduction:** An escaped agent reaching the daemon process gains USER-level
   access, not SYSTEM. It cannot pivot to other user sessions, to WFP filter rules, or to the
   machine-wide SCM namespace.

3. **MSI scope:** The machine MSI (`nono-machine.wxs`) remains the sole installer for
   `nono-wfp-service`. No machine MSI change is required in Phase 74.

4. **Non-Windows stub required:** Per CLAUDE.md cross-target gate, `nono-agentd.rs` MUST include a
   `#[cfg(not(target_os = "windows"))]` stub that prints a diagnostic and exits non-zero (same
   pattern as `nono-wfp-service.rs` lines 18-22). This stub keeps the workspace `cargo check`
   passing on Linux and macOS.

5. **Wave 1 gate:** No service host code (`nono-agentd.rs`, `agent_daemon/mod.rs`,
   `agent_daemon/accept_loop.rs`, `agent_daemon/launch.rs`, `agent_daemon/reap.rs`) is written
   until this ADR exists in git AND the Wave 0 spike (`daemon_handle_baseline.rs`) passes on a
   real Win11 host (human checkpoint "approved + spike green").

---

## Alternatives Considered

| Alternative | Reason rejected |
|-------------|----------------|
| Run daemon as LocalSystem (copy `nono-wfp-service.rs` without changing account type) | Pitfall 4: escaped agent pivots to SYSTEM; breaks tenant isolation; gives access to WFP, other sessions, and machine-wide SCM. |
| Run daemon as `NT SERVICE\nono-agentd` (virtual service account, machine-scoped) | Still a system-account with `SeAssignPrimaryTokenPrivilege` and cross-session visibility; does not run in the user's interactive session; harder to install without elevation. |
| Single binary merging daemon + WFP service | Two privileges in one process. A WFP-crash takes down the daemon and all agents. Split is the right boundary. |
| Skip `ImpersonateNamedPipeClient`, use SDDL only | Weaker: SDDL is the primary gate, impersonation is defense-in-depth. Removing either layer reduces security. |

---

## References

- `crates/nono-cli/src/bin/nono-wfp-service.rs` — the proven service skeleton this daemon adapts
- `.planning/phases/74-persistent-multi-tenant-daemon/74-CONTEXT.md` §D-01, D-03, D-04 — locked decisions
- `.planning/phases/74-persistent-multi-tenant-daemon/74-RESEARCH.md` §Least-Privilege USER Service Shape — key divergences table; SCM account analysis
- `.planning/research/PITFALLS.md` §Pitfall 4 — daemon privilege attack surface
- `proj/DESIGN-supervisor.md` — process model, execution strategies, supervisor IPC
- `crates/nono-cli/tests/daemon_handle_baseline.rs` — Wave 0 spike (confirms A1/A2/A6 empirically)
