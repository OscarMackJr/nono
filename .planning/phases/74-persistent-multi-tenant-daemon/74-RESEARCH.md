# Phase 74: Persistent Multi-Tenant Daemon - Research

**Researched:** 2026-06-14
**Domain:** Windows Win32 — persistent user-mode service, multi-tenant named-pipe IPC, server-side pipe impersonation, per-agent token/job lifetime, SCM-registered USER-privilege service binary
**Confidence:** HIGH (every finding grounded in current in-tree code; no new external dependencies; Win32 semantics verified against in-tree implementation plus authoritative MSDN references cited in milestone research)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Ship the daemon as a per-user Windows service (SCM-registered via `windows-service` crate, modeled on `nono-wfp-service.rs` shape) AND a foreground/on-demand fallback mode. Service runs as least-privilege USER in the user's session — NOT LocalSystem/SYSTEM. USER daemon is fully split from the elevated WFP service. Privilege model recorded as an ADR written BEFORE the service host is coded (SC4 — load-bearing ordering).
- **D-02:** The daemon's sole path to confinement is daemon-launches. Adopting an externally-spawned agent is OUT OF SCOPE for Phase 74.
- **D-03:** Agents die with the daemon (fail-secure). Daemon holds the per-agent job handle; `KILL_ON_JOB_CLOSE` means daemon exit terminates every confined agent. No orphaned agents survive a daemon restart.
- **D-04:** Profile-only; NO daemon→WFP coupling in Phase 74. Per-agent WFP egress scoping is Phase 75 / SUPP-02.
- **D-05:** Minimal operator surface — `nono daemon start|stop|status` lifecycle verbs; `nono agent launch --profile <engine> -- <cmd>` and `nono agent list`; reuse `nono classify <pid>` from Phase 73 for inspection.
- **D-06:** SC2 cross-tenant-denial negative test drives the capability pipe directly (in-process / integration test impersonating two tenants) — no operator CLI query verb required.

### Claude's Discretion

- The in-phase spike structure (how much to spike vs. build directly; harness shape) gated on fresh-token isolation + deterministic reap + cross-tenant denial.
- Whether the wire frame needs a new tenant id field or `session_id` suffices as the tenant key (SC5: extend ONLY if `session_id` proves insufficient).
- Exact owning-struct / `Drop` shape for per-agent resources, the per-tenant SDDL pipe-instance vs single-pipe-with-impersonation mechanism, Event Log IDs, and `nono daemon`/`nono agent` output formats. Keep fail-secure throughout.

### Deferred Ideas (OUT OF SCOPE)

- Adopting externally-spawned agents into the daemon (D-02; Phase 73 best-effort/demote-only only).
- Per-agent WFP egress scoping / daemon→WFP-service coordination (Phase 75 / SUPP-02; D-04).
- Post-hoc demote (Phase 75 / SUPP-01).
- Tenant-scoped `nono agent query` CLI verb (rejected D-05/D-06; isolation proven at protocol layer).
- Agents surviving a daemon restart (D-03 chose fail-secure kill-with-daemon).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DMON-01 | A persistent local daemon launches and confines multiple concurrent agents, each with a fresh token + job object, deterministically reaped on agent exit — running N agents over time returns to baseline handle/job count (no leak). | Answered in §§ Token/Job Fresh-vs-Reuse, Deterministic Reap/Drop shape, In-Phase Spike harness. Fresh-per-agent model confirmed sound; 100-agent test structure defined. |
| DMON-02 | The daemon's multi-tenant capability pipe isolates tenants — authenticates each client server-side (`ImpersonateNamedPipeClient` + per-tenant SID match) so one agent cannot read or use another agent's capabilities (a cross-tenant request is denied). | Answered in §§ ImpersonateNamedPipeClient Composition, Per-Tenant SDDL vs Single-Pipe-with-Impersonation, session_id-vs-tenant-field. Full Win32 call sequence documented. |
| DMON-03 | The daemon runs at least privilege (user, not LocalSystem) and is split from the elevated WFP-control service, so a confined agent that escapes cannot pivot to SYSTEM or to other tenants. (Backed by a privilege-model ADR.) | Answered in §§ Least-Privilege USER Service shape, Foreground Fallback, ADR content outline. |
</phase_requirements>

---

## Summary

Phase 74 generalizes three proven in-tree subsystems into a persistent, least-privilege, multi-tenant daemon. The research confirms this is overwhelmingly **composition work**, not green-field design. The two genuinely unspiked mechanisms (server-side `ImpersonateNamedPipeClient` composition with the existing Low-IL/AppContainer cap-pipe SDDL; token/job reuse-vs-fresh across many tenants) are both **resolvable with in-tree primitives** — no new Win32 APIs beyond what `windows-sys 0.59` already exposes, no new crates. The research de-risks both unknowns concretely below.

The strongest finding: the existing `build_capability_pipe_sddl` function already implements per-SID SDDL ACE injection, the `bind_low_integrity_with_session_and_package_sid` function already handles per-AppContainer-SID pipe DACL construction, and `PIPE_UNLIMITED_INSTANCES` already appears in `bind_aipc_pipe`. The daemon pipe accept loop is the generalization of this to N tenants — one named pipe, N instances, each with its own per-tenant SDDL that admits only that tenant's AppContainer package SID. Server-side `ImpersonateNamedPipeClient` is a defense-in-depth layer ON TOP of the per-tenant SDDL (which already prevents a wrong-SID client from connecting); it is straightforward to add because `RevertToSelf` already appears in the test suite (`socket_windows.rs` line ~2439) confirming the import path is understood.

The riskiest remaining unknown is **whether 100 concurrent fresh-token + fresh-job allocations over time return to baseline** — this is by definition untestable from reading the code and requires the in-phase spike. The spike should be run FIRST (Wave 0) before the full daemon binary is built.

**Primary recommendation:** Spike-first (Wave 0): write a standalone Windows test harness that mints 100 fresh token+job+pipe-instance triples, waits for each to exit, asserts handle count returns to baseline. Gate Wave 1 (daemon binary) on a green spike. Use per-tenant SDDL pipe instances (not single-pipe-with-impersonation) as the primary isolation mechanism; add `ImpersonateNamedPipeClient` as a defense-in-depth authorization layer on each accepted connection.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Agent launch (spawn + confine) | Daemon process (nono-agentd) | nono-cli broker arm (called from daemon) | Daemon is the owner/parent; it calls the existing Phase 71 broker arm exactly as `nono run` does |
| Tenant isolation / capability pipe | Daemon process (per-tenant pipe SDDL + server-side impersonation) | nono lib `socket_windows.rs` | Pipe DACL + impersonation live in the daemon's accept loop; the nono lib supplies the SDDL construction primitives |
| Token/job lifetime (reap) | Daemon process (owning struct + `Drop`) | nono lib (`CloseHandle` wrappers) | The daemon's in-memory tenant registry owns all per-agent handles; `Drop` closes them |
| AI_AGENT marker (Phase 73) | nono lib `agent.rs` `AgentRegistry` | Daemon (calls `insert` at spawn) | `AgentRegistry` already cross-platform; daemon calls `insert` at mint time, promoting the registry from per-run to persistent |
| Privilege model / service host | nono-agentd binary (USER service, split from WFP) | nono-cli MSI registration | Service runs at user privilege; WFP responsibility stays in the separate elevated service |
| CLI verbs (`nono daemon`, `nono agent`) | nono-cli | Daemon control pipe | Verbs route over the daemon's control pipe; all UX stays in nono-cli |
| Cross-target stub | nono-agentd binary `#[cfg(not(target_os = "windows"))]` | (none) | Same pattern as `nono-wfp-service.rs`; required by CLAUDE.md cross-target gate |

---

## Unspiked Mechanism 1: Token/Job Fresh-vs-Reuse Across Many Tenants

### Why this is the riskiest unknown

[CITED: `.planning/research/PITFALLS.md` §Pitfall 5] Token reuse collapses isolation in three independent ways:

1. **Restricting SID bleed**: A `WRITE_RESTRICTED` token carries the restricting SID as a permanent token field. Reusing a token from agent A for agent B means B's process passes the DACL second-pass check for A's relabeled workspace paths — B can write A's files.
2. **WFP scope blur**: WFP filters are keyed per AppContainer package SID (E4 identity). Reusing an AppContainer profile means A and B share one WFP filter scope — B's traffic passes A's egress rules.
3. **Job membership confusion**: A job object has KILL_ON_JOB_CLOSE semantics. A reused job handle closing early kills the wrong tenant; a reused job with different resource caps silently changes policy for the new tenant.

### Confirmed fresh-per-agent model

[VERIFIED: in-tree code `crates/nono-cli/src/exec_strategy_windows/launch.rs`] The existing per-run launch path already mints fresh per-run:
- A per-run AppContainer profile via `CreateAppContainerProfile` (NOT `DeriveAppContainerSidFromAppContainerName` alone — the MUST-USE pattern per project memory `windows_appcontainer_wfp_validated`)
- A per-run job object via `CreateJobObjectW` (currently unnamed; Phase 73 named it; the daemon becomes the owner)
- A per-run confining token via the broker arm

The daemon orchestrates N of these fresh mints in sequence and in parallel. The model is: fresh-per-agent = the per-run broker call, unchanged, called N times. This is confirmed sound by spike 003 (VALIDATED) for cmd/powershell/python.

### What needs to be proven (the spike target)

The single genuinely unspiked question is: **do 100 successive fresh-token + fresh-job + fresh-pipe-instance allocations return to the Windows handle-count baseline after each agent exits?** This is a resource accounting question, not a design question. It requires a real Win11 host and live measurement.

**Known leak vectors to test:**
- Job handles not closed when the agent registry entry is removed
- Pipe instance handles not closed after `DisconnectNamedPipe` + re-arm (in the daemon accept loop)
- The broker process inheriting handles the daemon allocated (check `bInheritHandles=FALSE` for the rendezvous file grant / pipe ACL handles)
- `AppContainerProfile` registration accumulation: each `CreateAppContainerProfile` call creates a persistent entry in the registry under `HKCU\SOFTWARE\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppContainer\Storage\`. Confirm `DeleteAppContainerProfile` is called in `Drop` to avoid unbounded registry growth.

**Confidence:** [ASSUMED] that 100 agents return to baseline — this is the spike's job to prove, not research's. The design (fresh-per-agent, owned by one struct, `Drop` closes all) is sound; the question is whether every handle is actually being closed and no Win32 API leaves a reference alive.

### Cross-tenant isolation: what "fresh" buys

After minting fresh for each agent, isolation holds because:
- Each agent has a distinct AppContainer package SID (the daemon's tenant key)
- Each agent's pipe instance DACL admits only that package SID
- Each job has distinct `KILL_ON_JOB_CLOSE` — daemon closing one job handle kills only that tenant's processes
- No restricting SID is shared between tenants (each token is independently minted)

**Failure condition to test in the spike:** Launch agents A and B with overlapping workspace dirs; confirm B cannot write A's relabeled Low-writable path (the workspace relabeling uses a per-path mandatory label, which is not SID-keyed — this is fine because Low-IL cannot write Medium-IL-or-higher paths; the label makes the path writable ONLY from Low-IL, so both agents can write the path IF they are both Low-IL. This is expected: workspace dirs are per-agent and absolute — the profiles declare distinct absolute workspaces per D-01/ENG-03). The actual cross-tenant test is: can agent B open agent A's capability pipe instance and receive agent A's grants? Answer: No, because per-tenant SDDL prevents connection in the first place (see mechanism 2).

---

## Unspiked Mechanism 2: Server-Side `ImpersonateNamedPipeClient` Composition with the Existing Cap-Pipe SDDL DACL Handshake

### Current state of `socket_windows.rs`

[VERIFIED: in-tree code `crates/nono/src/supervisor/socket_windows.rs`] Today's implementation:
- Server side: `CreateNamedPipeW` → `ConnectNamedPipe` (blocking) → serve frames. No `ImpersonateNamedPipeClient` call. The server reads the client's PID via `GetNamedPipeClientProcessId` (called by `verify_connected_server_pid`, line ~1840) — but this is the CLIENT verifying the SERVER's PID, not the server authenticating the client.
- Client-side: DACL-based admission only (the per-tenant SDDL ACE admits the correct package SID; a wrong-SID process gets `ERROR_ACCESS_DENIED` from `CreateFileW` before the connection even completes).
- `ImpersonateNamedPipeClient` does NOT appear in any Rust source file in the codebase. It appears only in documentation/context files (confirmed by codebase grep).
- `RevertToSelf` DOES appear in `socket_windows.rs` tests (line ~2439) for `ImpersonateLoggedOnUser` test cleanup — confirming the import path and usage pattern are understood.

### Composition analysis: does `ImpersonateNamedPipeClient` work with the existing SDDL?

[ASSUMED — Win32 semantics from training knowledge, with HIGH confidence based on the design of the Win32 API]

**The DACL is the access gate; impersonation is the identity query.** These are independent mechanisms that compose cleanly:

1. The client calls `CreateFileW` with the pipe name. Windows runs the DACL check against the client's token. The per-tenant SDDL ACE `(A;;0x0012019F;;;S-1-15-2-<agent-pkg-sid>)` admits the correct AppContainer package SID — wrong-SID clients fail here before the connection completes. SDDL admission is still the first gate.

2. After `ConnectNamedPipe` returns (the client is now connected), the server calls `ImpersonateNamedPipeClient(pipe_handle)`. This causes the server thread to temporarily adopt the connected client's security context.

3. The server calls `GetTokenInformation(GetCurrentThread_token, TokenAppContainerSid, ...)` to extract the AppContainer package SID from the impersonated token, then calls `RevertToSelf()`.

4. The server compares the extracted SID against its tenant registry (`AgentRegistry::minted_sids`). If the SID is not in the registry, or does not match the tenant the wire-frame `session_id` claims, the server closes the pipe instance and refuses. [ASSUMED — registry membership check]

**Does `ImpersonateNamedPipeClient` require special privileges in a USER-privilege service?**

[ASSUMED — HIGH confidence] `ImpersonateNamedPipeClient` requires that the server process has `SeImpersonatePrivilege`. This privilege is granted automatically to accounts in the `SERVICE` group and to service processes running via SCM. A USER-privilege SCM-registered service DOES have `SeImpersonatePrivilege` by default (it is in the Windows "Service logon rights" grant). Confirm this holds for a per-user service (Interactive Logon account, not LocalSystem) — it should, as per-user SCM services receive the same privilege set.

**Win32 call sequence for the daemon's accept loop:**

```
// After ConnectNamedPipe returns successfully:

// Step 1: Impersonate the connected client
let ok = unsafe { ImpersonateNamedPipeClient(pipe_handle) };
if ok == 0 { /* fail-secure: close the instance */ }

// Step 2: Open the impersonated thread token
let mut token: HANDLE = null_mut();
unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, FALSE, &mut token) };

// Step 3: Read the AppContainer package SID from the token
// TokenAppContainerSid = 56 (Windows SDK constant)
let mut info_buf = vec![0u8; 512];
unsafe { GetTokenInformation(token, TokenAppContainerSid, ...) };

// Step 4: Convert the SID to string form and match against registry
// (using ConvertSidToStringSidW — already imported in socket_windows.rs line ~29)

// Step 5: ALWAYS revert (fail-secure: close instance on any error)
unsafe { RevertToSelf() };
unsafe { CloseHandle(token) };

// Step 6: Compare extracted SID against daemon's tenant registry
// If NOT in registry → close pipe instance (deny)
// If in registry → continue serving (the wire-frame session_id is a hint, the
//                  kernel-vouched SID is the authorization signal)
```

**`TokenAppContainerSid` enum variant:** The `windows-sys 0.59` crate exposes `TOKEN_INFORMATION_CLASS` variants. `TokenAppContainerSid` has value `56` in the Windows SDK. Verify the variant name is `TokenAppContainerSid` in `windows-sys 0.59` (it may require a `windows_sys::Win32::Security` import). [ASSUMED — HIGH confidence from training; verify in the crate docs before coding]

**Import requirements (windows-sys 0.59, no version bump needed):**

```rust
use windows_sys::Win32::Security::{
    ImpersonateNamedPipeClient,   // new import
    OpenThreadToken,               // new import
    RevertToSelf,                  // already in test scope; add to production scope
    TokenAppContainerSid,          // new import — verify exact variant name in 0.59
    TOKEN_QUERY,                   // already imported
};
use windows_sys::Win32::System::Threading::GetCurrentThread; // new import
```

None of these require enabling new `windows-sys` features — all are in `Win32_Security` and `Win32_System_Threading` which are already enabled.

### Recommended approach: per-tenant SDDL as primary gate, impersonation as defense-in-depth

The two mechanisms compose as follows:

| Mechanism | Role | What it prevents |
|-----------|------|-----------------|
| Per-tenant SDDL pipe instance (existing machinery) | **Primary gate** — prevents connection from wrong-SID client | A wrong-SID process cannot open the pipe at all (OS-enforced) |
| `ImpersonateNamedPipeClient` + registry check | **Defense-in-depth** — server-side identity confirmation after connection | A process that somehow passed DACL (e.g., a SID spoofing scenario) cannot impersonate a different tenant; also enables `GetNamedPipeClientProcessId` + `IsProcessInJob` double-check |

**Recommendation:** Implement BOTH. The SDDL is already proven (it's the existing production path for the single-tenant supervisor). Add `ImpersonateNamedPipeClient` + registry check on every accepted connection as the second layer.

**Single-pipe-with-impersonation only (alternative):** Would require NOT using per-tenant SDDL, relying purely on `ImpersonateNamedPipeClient` to distinguish tenants. This is weaker because it allows any Low-IL same-session process to connect (the DACL would need to admit all Low-IL processes). Rejected in favor of layered approach.

---

## `session_id` vs. Net-New Tenant Field

### Research determination

[VERIFIED: in-tree code `crates/nono/src/supervisor/types.rs` lines 458-479]

The existing `CapabilityRequest` struct has:
```rust
pub session_id: String,
```

This field is already present in all `SupervisorMessage::Request(CapabilityRequest)` messages. The field's existing semantic is a per-run identifier supplied at launch time.

**For the multi-tenant daemon:** each agent launch mints one `session_id` (a per-tenant synthetic SID string, or a UUID derived from the AppContainer package SID). The daemon's tenant registry maps `session_id → TenantState { package_sid, job_handle, caps, ... }`.

**Verdict: `session_id` SUFFICES as the tenant key.** No net-new wire field is needed because:
1. `session_id` is already in every `CapabilityRequest`
2. The daemon uses `session_id` as a *routing hint* to look up the tenant entry
3. Authorization is by the kernel-vouched AppContainer package SID from `ImpersonateNamedPipeClient`, not by `session_id` (so `session_id` spoofing by a malicious agent doesn't enable privilege escalation — the impersonation catches it)
4. The replay-guard already uses `session_id` to detect duplicate request IDs within a session

**Wire protocol change: NONE required.** SC5 directive ("extend ONLY if `session_id` proves insufficient") is satisfied: it is sufficient.

---

## Deterministic Reap: Owning-Struct / `Drop` Shape + 100-Agent Test Structure

### Per-agent owning struct

```rust
// In: crates/nono-cli/src/bin/nono-agentd.rs  (Windows-only)
// or in the nono lib's daemon module

#[cfg(target_os = "windows")]
struct AgentTenant {
    /// The tenant's unique key (also the session_id in wire messages).
    tenant_id: String,
    /// The AppContainer package SID minted at spawn time (the authorization key).
    package_sid: String,
    /// Job handle with KILL_ON_JOB_CLOSE — closing this kills all agent processes.
    job_handle: OwnedHandle,
    /// The primary process handle for wait-on-exit.
    process_handle: OwnedHandle,
    /// The AppContainer profile name (for DeleteAppContainerProfile in Drop).
    profile_name: String,
    /// The CapabilitySet / profile snapshot decided at launch time.
    caps: CapabilitySet,
}

#[cfg(target_os = "windows")]
impl Drop for AgentTenant {
    fn drop(&mut self) {
        // job_handle and process_handle are OwnedHandle — closed on drop.
        // Also delete the AppContainer profile to avoid registry accumulation.
        let _ = delete_app_container_profile(&self.profile_name);
        // Pipe instances for this tenant are separately reaped by the accept loop.
    }
}
```

[ASSUMED — the exact field names are discretionary; `OwnedHandle` is already used in `socket_windows.rs` line 17 — VERIFIED in-tree]

**Registry structure:**

```rust
#[cfg(target_os = "windows")]
struct DaemonState {
    tenants: Arc<Mutex<HashMap<String, AgentTenant>>>,  // tenant_id → state
    agent_registry: Arc<Mutex<AgentRegistry>>,          // Phase 73 registry; daemon makes it persistent
}
```

`AgentRegistry` (Phase 73) is wrapped in `Arc<Mutex<_>>` as stated in its doc comment (line ~71 of `agent.rs`). The daemon `insert`s at spawn time and `remove`s on reap. The `AgentRegistry` and `DaemonState.tenants` maps must be updated atomically to avoid a TOCTOU window where a dead agent's SID is still in the registry.

### Reap-on-exit: waiting on agent processes

The daemon needs to detect agent exit and trigger cleanup. Two standard Win32 approaches:

1. **Job completion port** (`CreateIoCompletionPort` + `SetInformationJobObject(JobObjectAssociateCompletionPortInformation)`): The daemon creates a completion port and associates every job with it. When any process in a job exits, the OS posts `JOB_OBJECT_MSG_EXIT_PROCESS` to the port. A dedicated daemon task (`tokio::spawn` + `IOCP` polling) receives exit notifications and calls `tenants.remove(tenant_id)`, which triggers `AgentTenant::Drop`.

2. **Per-agent `WaitForSingleObject` on the process handle** in a dedicated async task: Spawn a `tokio::task` per agent that calls `WaitForSingleObject(process_handle, INFINITE)` (or the async equivalent via `tokio::task::spawn_blocking`). When the process exits, the task removes the tenant entry.

**Recommendation:** Job completion port for the daemon. It is a single-notification mechanism (one IOCP watcher task, not one task per agent), scales to many concurrent agents, and the IOCP machinery is already used in the WFP service's async loop. [ASSUMED — LOW: the WFP service uses tokio named-pipe I/O, not IOCP directly; verify tokio's named-pipe server uses IOCP internally on Windows, which it does per tokio docs]

Simpler fallback if IOCP is complex to wire: `spawn_blocking` per agent, `WaitForSingleObject`. Adequate for the expected scale (< 10 concurrent agents in practice).

### 100-agent handle-count-returns-to-baseline test structure

**Purpose:** Prove DMON-01 (no handle leak over N agent lifetimes). This is an integration test, not a unit test, because it requires real Win32 handle allocation.

**Structure:**

```rust
#[cfg(target_os = "windows")]
#[cfg(test)]
mod handle_baseline_tests {
    // Required: run on Windows with the broker arm available (dev-layout or signed nono.exe).
    // Can be gated behind an env var (NONO_DAEMON_INTEGRATION_TESTS=1) like existing
    // integration tests to avoid slowing the standard test suite.

    #[test]
    fn n_agents_over_time_returns_to_baseline_handle_count() {
        // 1. Record baseline handle count for the test process.
        //    Use GetProcessHandleCount(GetCurrentProcess(), &mut count).
        let baseline = get_handle_count();

        // 2. Run 100 agent launch/wait/reap cycles sequentially.
        //    Each cycle: mint token+job, spawn a trivial agent (cmd.exe /C exit 0),
        //    wait for exit, call AgentTenant::drop (via remove from HashMap).
        for _ in 0..100 {
            let tenant = spawn_minimal_agent();  // mints AppContainer, job, process
            wait_for_agent_exit(&tenant);
            drop(tenant);  // triggers AgentTenant::Drop → CloseHandle × 2 + DeleteAppContainerProfile
        }

        // 3. Assert handle count returns within a small epsilon of baseline.
        //    Allow +5 for any test harness overhead; a real leak accumulates 200+.
        let post = get_handle_count();
        assert!(
            post <= baseline + 5,
            "handle count did not return to baseline: before={baseline} after={post}"
        );
    }
}
```

The test's `spawn_minimal_agent` calls the same code path as the production daemon — not a mock. If the production path leaks, the test catches it. Run this test on a real Win11 host (not in the standard CI suite due to the broker-arm requirement — host-gated like SC1).

---

## Least-Privilege USER Service Shape + Foreground Fallback

### SCM account for a per-user service (D-01, DMON-03)

[ASSUMED — HIGH confidence from Windows service documentation in training knowledge]

A per-user service runs under the **Interactive Logon account** (the logged-in user's account, not `LocalSystem`). In SCM registration terms, this means the `lpServiceStartName` parameter to `CreateServiceW` is either the user's UPN (`DOMAIN\user`) or `NT SERVICE\nono-agentd` (a virtual service account — preferred for least-privilege). The machine MSI cannot register a per-user service with a hardcoded user account (it doesn't know which user will install it). Therefore:

**Options for per-user service registration:**
1. **Per-user SCM service** (`HKCU` services): Registered and started via SCM in the user's session. Supported since Windows 10 1703. Registered with `CreateService(... SERVICE_USER_OWN_PROCESS, ...)`.
2. **Foreground process (on-demand mode)**: The user runs `nono daemon start` which launches `nono-agentd --service-mode=foreground` as a regular process in the user's session. No SCM registration needed for dev/testing.

**Recommended model for Phase 74:**
- **SCM per-user service** as the primary: registered by `nono daemon install` (a CLI sub-verb, or via the per-user MSI). The service binary is `nono-agentd.exe`; it calls `service_dispatcher` on the `--service-mode` path (same as `nono-wfp-service.rs`). Per-user services in Windows 10/11 do NOT require elevation — they are installed in the user's `HKCU\SYSTEM\CurrentControlSet\Services` and run at the user's IL. No desktop/window-station access needed for a headless daemon.
- **Foreground fallback**: `nono daemon start --foreground` (or simply running `nono-agentd` without `--service-mode`) runs as a regular foreground process. This is the dev/testing path (matches the `nono-wfp-service.rs` foreground-vs-service pattern).

### Key divergences from `nono-wfp-service.rs`

[VERIFIED: in-tree code `crates/nono-cli/src/bin/nono-wfp-service.rs`]

| Aspect | `nono-wfp-service` | `nono-agentd` (daemon) |
|--------|--------------------|------------------------|
| Privilege | LocalSystem (SYSTEM) — needed for WFP FwpmFilterAdd0 | USER (interactive logon) — no WFP manipulation |
| Service type | `SERVICE_WIN32_OWN_PROCESS` (machine-wide SCM) | `SERVICE_USER_OWN_PROCESS` (per-user SCM) |
| MSI registration | Machine MSI (`nono-machine.wxs`) | Per-user MSI (or `nono daemon install` verb) |
| Window station | Irrelevant (SYSTEM service) | Irrelevant (headless; no GUI) |
| WFP dependency | Drives WFP filters (the whole purpose) | Does NOT talk to WFP in Phase 74 (D-04) |
| Startup account | LocalSystem | Current user's account |
| Non-Windows stub | `eprintln! + exit(1)` | Same pattern — required by CLAUDE.md |

The `windows-service 0.7` crate's `define_windows_service!` / `service_dispatcher` / `service_control_handler` macros work for both machine-wide and per-user services. The account type is a SCM registration-time parameter, not a code change.

### Per-user service: does `ImpersonateNamedPipeClient` work?

[ASSUMED — HIGH] `SeImpersonatePrivilege` is required. For a per-user SCM service running as the interactive user, the service token is the user's interactive logon token (same as a regular process). Interactive users have `SeImpersonatePrivilege` by default on Windows 10/11. This is confirmed by the fact that any interactive-user-launched process can impersonate pipe clients (it's a normal user-mode operation). The per-user service inherits the same privilege.

### MSI registration

The per-user daemon can be registered as a SCM per-user service via the user-scoped MSI (`nono-user.wxs`) with a `ServiceInstall` element using:
- `Account="[LOGONUSER]"` or the per-user SCM account parameter
- `Type="userService"` (WiX per-user service element)

Or via a CLI verb `nono daemon install` / `nono daemon uninstall` that calls the SCM API directly (simpler for Phase 74, avoids MSI complexity for the new binary). The MSI path can be deferred to Phase 75 packaging.

The v2.11 non-fatal-service-start posture must be inherited: if `service_dispatcher` fails (e.g. not running in SCM context), the binary falls through to foreground mode without panicking.

---

## In-Phase Spike Recommendation

### What to spike vs. what to build directly

**SPIKE (Wave 0, before any daemon binary code):**
Everything in DMON-01 that depends on handle accounting under real Win32. The spike is a self-contained Rust integration test (or standalone binary) that exercises the full mint→spawn→wait→reap→Drop cycle N=100 times. It does NOT need the full daemon binary — it only needs the `AgentTenant` struct and its `Drop` impl.

**BUILD DIRECTLY (Wave 1+, after spike green):**
- The daemon service binary shell (modeled on `nono-wfp-service.rs`)
- The multi-tenant accept loop with per-tenant SDDL pipe instances
- `ImpersonateNamedPipeClient` + registry auth layer
- `nono daemon` / `nono agent` CLI verbs
- ADR document (authored in Wave 0 before any service code is written)

### Spike harness shape

The spike is a Windows integration test in `crates/nono-cli/tests/daemon_handle_baseline.rs` (or in the new `crates/nono-agentd/` crate if the daemon gets its own crate). It must:

1. **Prove: fresh token + job per agent** — spawn 100 agents with distinct AppContainer profiles; assert each has a distinct package SID (check via `read_process_appcontainer_sid`).

2. **Prove: deterministic reap** — after each agent exits and `AgentTenant` is dropped, assert the handle count returned to within epsilon of the pre-agent baseline. Use `GetProcessHandleCount(GetCurrentProcess(), ...)`.

3. **Prove: cross-tenant denial** — after establishing tenant A's pipe instance, attempt to connect as an impersonated tenant B (using the existing `ImpersonateLoggedOnUser` pattern from `socket_windows.rs` test line ~2349) and assert the connection is denied with `ERROR_ACCESS_DENIED` (DACL gate) — tenant B's AppContainer SID is not in tenant A's pipe SDDL. This is an in-process test (same pattern as `capability_pipe_admits_restricted_token_child_with_session_sid`).

**Gate:** All three assertions pass on a real Win11 host (dev-layout or signed nono.exe, per R-B4 broker trust gate). Wave 1 is blocked until all three pass.

**What happens if the handle count does NOT return to baseline:**
Look for: job handle leaked from the tenant registry (missing `CloseHandle` in `Drop`), pipe instance handle not disconnected/closed after reap, AppContainer profile not deleted (`DeleteAppContainerProfile` omitted in `Drop`), process handle not closed (missing `CloseHandle` on `process_handle` in `Drop`).

---

## Standard Stack

### Core (no new crates required)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows-service` | 0.7 (in-tree) | SCM dispatch, service_dispatcher, define_windows_service!, control handler | Already ships in `nono-wfp-service`; daemon is a second instance of the same pattern |
| `windows-sys` | 0.59 (workspace pin — DO NOT BUMP) | `ImpersonateNamedPipeClient`, `OpenThreadToken`, `GetCurrentThread`, `TokenAppContainerSid`, `RevertToSelf`, `GetProcessHandleCount`, `DeleteAppContainerProfile` | All present in 0.59; no version change needed |
| `tokio` | 1.x (workspace) | Async multi-client accept loop; `spawn_blocking` for `WaitForSingleObject` per-agent | Already the async runtime; `nono-wfp-service` uses tokio |
| `nono` lib | 0.62.x (current) | `AgentRegistry`, `read_process_appcontainer_sid`, SDDL pipe construction, broker arm launch | Unchanged; daemon calls it |
| `serde` / `serde_json` | workspace | Framed JSON `SupervisorMessage` / `SupervisorResponse` wire protocol | Existing; no new protocol |
| `tracing` / `tracing-subscriber` | workspace | Daemon structured logging + Event Log surface | Same as `nono-wfp-service` |
| `getrandom` | 0.4 (workspace) | Per-tenant pipe-name nonce, per-agent `tenant_id` nonce | Already used in `socket_windows.rs` |

### Net-New Win32 API surface (no new crates)

| Win32 API | windows-sys Module | Purpose | Status in codebase |
|-----------|-------------------|---------|-------------------|
| `ImpersonateNamedPipeClient` | `Win32_Security` | Server-side: adopt connected client's security context | NEW — not yet imported in production code |
| `OpenThreadToken` | `Win32_Security` | After impersonation: open the current thread's impersonated token | NEW import |
| `GetCurrentThread` | `Win32_System_Threading` | Required by `OpenThreadToken` | NEW import |
| `TokenAppContainerSid` (value 56) | `Win32_Security` | Extract AppContainer package SID from the impersonated token | NEW — verify exact variant name in 0.59 |
| `RevertToSelf` | `Win32_Security` | Revert impersonation after SID extraction | Already imported in test scope; add to production scope |
| `GetProcessHandleCount` | `Win32_System_Threading` | 100-agent baseline test: count open handles | NEW import for test harness |
| `DeleteAppContainerProfile` | `Win32_Security_Isolation` | Drop cleanup: remove AppContainer profile registration | Verify availability in 0.59 (feature `Win32_Security_Isolation` already enabled) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Per-tenant SDDL + ImpersonateNamedPipeClient (layered) | Single-pipe, ImpersonateNamedPipeClient only | Weaker: single-pipe with open DACL admits ALL Low-IL same-session processes; impersonation is the only gate. Rejected: per-tenant SDDL already exists and is proven. |
| Job completion port (single IOCP watcher) | Per-agent `WaitForSingleObject` in `spawn_blocking` | IOCP is lower per-agent overhead; `spawn_blocking` is simpler. Either works for Phase 74 scale. |
| `SERVICE_USER_OWN_PROCESS` per-user SCM | LocalSystem machine service | LocalSystem is the pitfall (Pitfall 4, P4): escaping agent pivots to SYSTEM. Per-user SERVICE_USER_OWN_PROCESS is the correct least-privilege shape. |

---

## Package Legitimacy Audit

No new external crates are introduced in this phase. All dependencies already live in the workspace at pinned versions. The `windows-service 0.7` crate is already in `crates/nono-cli/Cargo.toml` powering `nono-wfp-service`. No slopcheck run needed — zero new packages.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
CLIENT (nono-cli `nono agent launch`)
        |
        | control pipe \\.\pipe\nono-agentd-control
        v
+---------------------------------------+
|  nono-agentd (USER privilege)          |  <- new binary, mirrors nono-wfp-service.rs shape
|                                       |
|  [1] LaunchAgent(profile, exe, args)   |
|       |                               |
|       | calls Phase 71 broker arm      |
|       v                               |
|  nono-shell-broker (Medium IL)         |
|       |                               |
|       | CreateProcessW (Low-IL token   |
|       |  + AppContainer pkg SID)      |
|       v                               |
|  Confined Agent Process               |  <- fresh job + fresh token + fresh pkg SID
|       |                               |
|  [2] AgentTenant { job_handle,        |
|       process_handle, pkg_sid, caps } |
|  AgentRegistry.insert(pkg_sid)         |
|                                       |
|  CAPABILITY PIPE ACCEPT LOOP           |
|  \\.\pipe\nono-agentd-cap-<rendezvous> |
|  PIPE_UNLIMITED_INSTANCES             |
|                                       |
|  Per connection:                      |
|    ConnectNamedPipe (blocking)         |
|    ImpersonateNamedPipeClient          |  <- NEW (DMON-02)
|    GetTokenInformation(TokenAppContainerSid)
|    RevertToSelf                       |
|    SID in registry?                   |
|    YES: serve capability request      |
|    NO:  close instance (deny)         |
|                                       |
|  [3] REAP: wait on process_handle     |
|    -> remove from tenants HashMap     |
|    -> AgentTenant::Drop               |
|       -> CloseHandle(job_handle)      |  <- KILL_ON_JOB_CLOSE kills agent
|       -> CloseHandle(process_handle)  |
|       -> DeleteAppContainerProfile    |
|       -> AgentRegistry.remove(sid)    |
+---------------------------------------+
        |
        | existing control pipe (unchanged)
        v
nono-wfp-service (SYSTEM) — Phase 75 only (D-04)
```

### Recommended Project Structure

```
crates/nono-cli/src/bin/
├── nono-wfp-service.rs    # existing (SYSTEM; unchanged)
└── nono-agentd.rs         # NEW daemon (USER; mirrors nono-wfp-service.rs shape)

crates/nono-cli/src/
├── agent_daemon/
│   ├── mod.rs             # daemon state, tenant registry
│   ├── accept_loop.rs     # multi-client pipe accept loop + ImpersonateNamedPipeClient
│   ├── launch.rs          # daemon-side launch orchestration (calls Phase 71 broker arm)
│   └── reap.rs            # AgentTenant Drop + IOCP / WaitForSingleObject reaper
└── agent_cli.rs           # `nono daemon` + `nono agent` verb implementations

proj/ADR-74-privilege-model.md   # WRITTEN FIRST (SC4 ordering gate)
```

The daemon mechanism (pipe server, impersonation, per-tenant auth, reap) belongs in `nono-cli` (not the `nono` lib) per the library-vs-CLI boundary in CLAUDE.md: policy and orchestration stay in the CLI; the lib supplies primitives (`AgentRegistry`, SDDL construction, broker arm). The `AgentRegistry` already lives in the lib (Phase 73); the daemon promotes it from per-run to persistent state by keeping it in `DaemonState` across agent lifetimes.

### Pattern 1: Multi-tenant accept loop with per-tenant SDDL

```rust
// Source: generalizes crates/nono/src/supervisor/socket_windows.rs bind_impl pattern
// Each accepted connection gets a fresh pipe instance with the tenant's pkg SID in the SDDL.

async fn accept_loop(daemon_state: Arc<DaemonState>) {
    loop {
        // Create a new named pipe instance (re-arm) with PIPE_UNLIMITED_INSTANCES.
        // The instance SDDL is built WITHOUT a tenant SID (we don't know the tenant yet).
        // After accepting the connection, we impersonate to learn the tenant SID,
        // then authorize from the registry.
        //
        // Alternative: pre-create one instance per known tenant at launch time,
        // each with a per-tenant SDDL. Simpler but requires pre-creating before
        // the agent process connects (connection ordering).
        let pipe = create_capability_pipe_instance(&base_sddl)?;
        let _ = tokio::task::spawn_blocking(|| {
            ConnectNamedPipe(pipe_handle, null_mut())
        }).await;

        // Spawn a per-connection task.
        let state = Arc::clone(&daemon_state);
        tokio::spawn(async move {
            handle_one_connection(pipe, state).await;
        });
    }
}

async fn handle_one_connection(pipe: PipeHandle, state: Arc<DaemonState>) {
    // 1. Impersonate → extract pkg SID → revert
    let client_sid = authenticate_client(&pipe)?;  // ImpersonateNamedPipeClient sequence

    // 2. Look up tenant by SID (registry membership check)
    let tenant = state.find_tenant_by_sid(&client_sid)?;

    // 3. Serve capability requests (session_id from wire frame is a routing hint,
    //    client_sid is the authorization signal)
    serve_frames(&pipe, &tenant, &state).await;
}
```

### Anti-Patterns to Avoid

- **Sharing one pipe SDDL across all tenants**: The baseline `CAPABILITY_PIPE_SDDL` admits SY/BA/OW — NOT the AppContainer Low-IL process. A shared Low-IL-admitting SDDL would admit ALL same-session Low-IL processes to ALL instances (Pitfall 1). Always use per-tenant SDDL or impersonation (both are implemented here).
- **Authorizing by wire-frame `session_id`**: `session_id` is a self-reported routing hint. Authorize by the kernel-vouched SID from `ImpersonateNamedPipeClient`. A compromised agent could send any `session_id` string.
- **Reusing a confining token or job across agents**: Collapses tenant isolation (Pitfall 5). Fresh-per-agent, always.
- **`KILL_ON_JOB_CLOSE` omitted**: Without it, a daemon crash leaves orphaned unmanaged processes running at Low-IL. The existing launch path already sets this flag — the daemon must not remove it.
- **`DeleteAppContainerProfile` omitted in Drop**: AppContainer profile registry entries accumulate in `HKCU\SOFTWARE\Classes\Local Settings\...\AppContainer\Storage\`. Over 1000+ agent launches the registry bloats. Clean up in `Drop`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-client pipe server | Custom polling loop | `tokio::net::windows::named_pipe` `ServerOptions` | Already proven in `nono-wfp-service`'s tokio loop; handles IOCP internally |
| Win32 SDDL string builder | Custom string concat | `build_capability_pipe_sddl` in `socket_windows.rs` | Already handles SDDL injection defense, length validation, per-SID ACE injection |
| AppContainer profile lifecycle | Custom registry manipulation | `create_process_containment` / `AppContainerProfile` / `OwnedAppContainerSid` in `sandbox/windows.rs` | Already handles `CreateAppContainerProfile` + `DeriveAppContainerSidFromAppContainerName`; lib-exported |
| Service skeleton | Custom SCM plumbing | `define_windows_service!` / `service_dispatcher` from `windows-service 0.7` | Already proven in `nono-wfp-service.rs`; copy the pattern byte-for-byte |
| Tenant wire protocol | Custom binary protocol | Existing framed JSON `SupervisorMessage` | Already bounded (64 KiB), replay-guarded, timeout-bounded, serde-derived |
| Token/SID extraction | Custom `GetTokenInformation` logic | `read_process_appcontainer_sid` from `nono::agent` | Already handles allocation + `ConvertSidToStringSidW`; returns the validated string form |

**Key insight:** The daemon is a composition of five already-proven in-tree subsystems. Every "new" thing the daemon does is either a call into existing code or a thin adaptation of an existing pattern. The only truly net-new Win32 calls are `ImpersonateNamedPipeClient`, `OpenThreadToken`, and `GetCurrentThread` — and all three are single-call wrappers with well-documented semantics.

---

## Runtime State Inventory

This phase is NOT a rename/refactor/migration. Runtime state inventory SKIPPED (greenfield daemon binary; no renaming of existing symbols).

---

## Common Pitfalls

### Pitfall 1: DACL-only pipe with shared Low-IL SDDL admitting all tenants (Cross-tenant capability theft)

**What goes wrong:** The accept loop reuses one SDDL constant (`CAPABILITY_PIPE_SDDL`) for all pipe instances. Any Low-IL same-session process can connect to any instance and receive any tenant's capability grants.
**Why it happens:** The single-tenant supervisor only needed one DACL (for the one child). N-tenant generalization drops the one-child assumption.
**How to avoid:** Per-tenant SDDL (embed the tenant's AppContainer package SID in each instance's SDDL ACE) PLUS `ImpersonateNamedPipeClient` authorization layer. Never reuse the base SDDL for the multi-tenant daemon.
**Warning signs:** No `ImpersonateNamedPipeClient` call in the accept loop; one `CAPABILITY_PIPE_SDDL` constant for all tenants.

### Pitfall 2: Token or job reuse across agents (Tenant isolation collapse)

**What goes wrong:** The daemon reuses an AppContainer profile from a previous agent to avoid the `CreateAppContainerProfile` cost. Agent B inherits A's restricting SID / workspace relabel / WFP scope.
**Why it happens:** The `CreateAppContainerProfile` call is expensive (writes to the registry) and tempting to cache.
**How to avoid:** Fresh profile = fresh SID = fresh isolation. The cost is acceptable (profile creation is a one-time-per-agent initialization, not per-request). Include `DeleteAppContainerProfile` in `AgentTenant::Drop` to clean up.
**Warning signs:** A "profile pool" or "reuse if same profile name" optimization.

### Pitfall 3: `ImpersonateNamedPipeClient` without `RevertToSelf` (Thread identity leak)

**What goes wrong:** The server thread retains the impersonated client identity after the SID check. Subsequent operations (DACL edits, process spawns) run with the client's identity instead of the daemon's identity — potentially failing or granting unintended access.
**Why it happens:** Error paths that `return Err(...)` before the `RevertToSelf` call.
**How to avoid:** Use a RAII guard that calls `RevertToSelf` on drop, analogous to the `AppliedDaclGrantsGuard` pattern from Phase 60. Never `return Err` from a scope containing an active impersonation without reverting first.
**Warning signs:** `ImpersonateNamedPipeClient` succeeds but `RevertToSelf` is only called on the happy path.

### Pitfall 4: `DeleteAppContainerProfile` omitted in `AgentTenant::Drop` (Registry accumulation)

**What goes wrong:** Over 1000+ agent launches, `HKCU\SOFTWARE\Classes\Local Settings\...\AppContainer\Storage\` accumulates stale profile entries. This is a registry bloat / information-leak issue (old agent profile names visible).
**Why it happens:** The single-run launch path doesn't need to clean up profiles (the per-run supervisor dies and profiles age out). The daemon's long lifetime means profiles never age out.
**How to avoid:** Call `DeleteAppContainerProfile(profile_name)` in `AgentTenant::Drop`. This is already called in the `OwnedAppContainerSid` cleanup path in the nono lib (verify and leverage this).

### Pitfall 5: ADR written AFTER the service host is coded (SC4 ordering violation)

**What goes wrong:** The privilege model is decided implicitly during coding (e.g., by copying `nono-wfp-service.rs` and forgetting to change the SCM account type), resulting in an undocumented SYSTEM-privilege daemon.
**Why it happens:** The `nono-wfp-service.rs` skeleton uses SYSTEM; it's the path of least resistance to copy without reading the account-type parameter.
**How to avoid:** The ADR (`proj/ADR-74-privilege-model.md`) is the FIRST deliverable in Wave 0, before any service binary code is written. SC4 makes this ordering explicit. The plan MUST gate Wave 1 on ADR completion.

### Pitfall 6: Broker arm not available in the daemon process context (R-B4 trust gate)

**What goes wrong:** The daemon binary is not `nono.exe` — it is `nono-agentd.exe`. The broker arm's trust gate (`R-B4: broker requires signed nono.exe or dev-layout`) checks the nono.exe binary specifically. If the daemon is in a different binary, does the broker still trust it?
**Why it happens:** The broker launch arm validates the calling binary's signature / path. The daemon's `nono-agentd.exe` may not be in the expected path / signed cert.
**How to avoid:** Investigate whether the broker trust gate checks the LAUNCHER binary (nono-agentd, which would need to be trusted) or the BROKER binary (nono-shell-broker, which is already trusted). The simplest resolution: have the daemon call into the same nono-cli launch code path (which is already in the trusted nono.exe binary). Option: daemon calls `nono.exe agent launch --internal-daemon-invoke` as a subprocess. Option: daemon link the launch logic as a library call from within nono-agentd.exe (same binary). Recommended: daemon is a SECOND BIN in `nono-cli` (`crates/nono-cli/src/bin/nono-agentd.rs`) which links the same launch logic — no separate binary trust problem.
**Warning signs:** `CreateProcessW ERROR_FILE_NOT_FOUND` on the broker spawn from the daemon (the classic broker trust gate failure symptom).

---

## Code Examples

### Example 1: `ImpersonateNamedPipeClient` + token SID extraction

```rust
// Source: derived from socket_windows.rs test pattern (RevertToSelf at line ~2439)
// and Win32 documentation for ImpersonateNamedPipeClient + GetTokenInformation.

#[cfg(target_os = "windows")]
fn authenticate_pipe_client(pipe_handle: HANDLE) -> Result<String> {
    use windows_sys::Win32::Security::{
        GetTokenInformation, ImpersonateNamedPipeClient, OpenThreadToken,
        RevertToSelf, TokenAppContainerSid, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThread;

    // SAFETY: pipe_handle is a valid connected pipe handle returned by ConnectNamedPipe.
    let ok = unsafe { ImpersonateNamedPipeClient(pipe_handle) };
    if ok == 0 {
        return Err(NonoError::SandboxInit(format!(
            "ImpersonateNamedPipeClient failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    // Open the impersonated thread token. Must call RevertToSelf before returning
    // (use a RAII guard in production code).
    let mut token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 0 /* bOpenAsSelf=FALSE */, &mut token)
    };
    if ok == 0 {
        unsafe { RevertToSelf() };
        return Err(NonoError::SandboxInit(format!(
            "OpenThreadToken failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    // Extract the AppContainer SID from the impersonated token.
    // Delegate to the existing read_process_appcontainer_sid logic if possible,
    // or replicate its GetTokenInformation(TokenAppContainerSid) + ConvertSidToStringSidW call.
    let pkg_sid_result = extract_appcontainer_sid_from_token(token);

    // ALWAYS revert before returning — even on error.
    unsafe { RevertToSelf() };
    unsafe { windows_sys::Win32::Foundation::CloseHandle(token) };

    pkg_sid_result
}
```

### Example 2: Per-tenant pipe SDDL instance (from existing machinery)

```rust
// Source: generalizes build_capability_pipe_sddl in socket_windows.rs.
// For the daemon's multi-tenant accept loop: each accepted connection
// gets a pipe instance whose SDDL embeds the tenant's AppContainer package SID.

fn create_tenant_capability_pipe_instance(tenant_pkg_sid: &str) -> Result<HANDLE> {
    // build_capability_pipe_sddl already handles None session_sid + Some package_sid:
    //   "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019F;;;{pkg_sid})S:(ML;;NW;;;LW)"
    // This SDDL admits ONLY the specific AppContainer package SID at Low-IL.
    let sddl = build_capability_pipe_sddl(None, Some(tenant_pkg_sid))?;
    // ... CreateNamedPipeW with PIPE_UNLIMITED_INSTANCES and the tenant SDDL
}
```

### Example 3: `AgentTenant` Drop (resource reap)

```rust
#[cfg(target_os = "windows")]
impl Drop for AgentTenant {
    fn drop(&mut self) {
        // OwnedHandle fields are closed by OwnedHandle's own Drop.
        // job_handle: KILL_ON_JOB_CLOSE kills all processes in the job.
        // process_handle: closed cleanly.

        // Delete the AppContainer profile to avoid HKCU registry accumulation.
        // Best-effort: log on failure but do not panic (daemon must stay up).
        if let Err(e) = delete_app_container_profile(&self.profile_name) {
            tracing::warn!(
                tenant_id = %self.tenant_id,
                error = %e,
                "Failed to delete AppContainer profile on agent reap — \
                 HKCU registry entry may persist"
            );
        }
        // AgentRegistry removal is the caller's responsibility (DaemonState removes
        // the tenant from the HashMap, which triggers this Drop).
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single-tenant supervisor (one child, one pipe instance) | Multi-tenant daemon (N children, N pipe instances, per-tenant SDDL) | Phase 74 | Requires server-side impersonation auth; per-tenant SDDL ACEs |
| `AgentRegistry` as per-run in-memory state | `AgentRegistry` as persistent daemon state across many agent lifetimes | Phase 74 | Must be wrapped in `Arc<Mutex<_>>` for concurrent access |
| Job object unnamed (Phase 73 changed to named) | Named job, daemon as owner (Phase 74) | Phase 74 | Daemon holds the job handle; KILL_ON_JOB_CLOSE binds agent lifetime to daemon |
| `ImpersonateNamedPipeClient` absent from production code | Added to daemon's accept path as defense-in-depth | Phase 74 | Server-side identity verification (DMON-02) |

**Deprecated/outdated:**
- Single shared `CAPABILITY_PIPE_SDDL` constant for multi-tenant use: deprecated for the daemon; the single-tenant supervisor still uses it correctly.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | None — workspace-default |
| Quick run command | `cargo test -p nono-cli --lib` (unit tests, no broker needed) |
| Full suite command | `cargo test -p nono-cli` (includes integration tests; some gated behind `NONO_DAEMON_INTEGRATION_TESTS=1` and require Win11 host) |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DMON-01 / SC1 | 2 concurrent confined agents, each served independently | integration (Win11 host) | `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test -p nono-cli daemon_concurrent_agents` | ❌ Wave 0 |
| DMON-01 / SC3 | 100-agent launch/exit returns handle count to baseline | integration (Win11 host) | `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test -p nono-cli daemon_handle_baseline` | ❌ Wave 0 (spike) |
| DMON-02 / SC2 | Cross-tenant-denial negative test | integration (in-process impersonation) | `cargo test -p nono-cli daemon_cross_tenant_denial` | ❌ Wave 0 |
| DMON-03 / SC4 | Privilege model ADR exists and daemon NOT running as SYSTEM | manual + doc check | Review `proj/ADR-74-privilege-model.md`; `sc qc nono-agentd` (manual) | ❌ Wave 0 |
| DMON-01 / SC5 | Wire protocol reuses `session_id`; no new field in `SupervisorMessage` | unit | `cargo test -p nono supervisor_message_no_tenant_id_field` | ❌ Wave 1 |
| DMON-02 | `ImpersonateNamedPipeClient` call present in accept loop; `RevertToSelf` on all paths | unit (mock pipe) | `cargo test -p nono-cli authenticate_pipe_client_reverts_on_error` | ❌ Wave 1 |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli --lib` (unit tests only; < 30s)
- **Per wave merge:** `cargo test -p nono-cli` (includes integration test suite; broker-arm tests are host-gated)
- **Phase gate:** All SC1–SC5 green, including Win11 host UAT, before `/gsd:verify-work`

### Wave 0 Gaps (must exist before Wave 1 implementation starts)

- [ ] `proj/ADR-74-privilege-model.md` — privilege model ADR (MUST be first; SC4 ordering gate)
- [ ] Spike harness: `crates/nono-cli/tests/daemon_handle_baseline.rs` — 100-agent reap test + fresh-token proof + cross-tenant denial test (in-process impersonation variant)
- [ ] `tests/daemon_cross_tenant_denial.rs` — or folded into the spike harness file

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | YES | Server-side `ImpersonateNamedPipeClient` + `AgentRegistry` membership check — kernel-vouched identity |
| V3 Session Management | YES | `session_id` as routing hint (not trust); per-agent fresh token (isolation); `KILL_ON_JOB_CLOSE` (session lifetime) |
| V4 Access Control | YES | Per-tenant SDDL DACL as primary gate; impersonation + registry as secondary; query-only pipe (no capability expansion) |
| V5 Input Validation | YES | Wire frames bounded at 64 KiB; `session_id` is a routing hint validated against registry, not trusted for authz; SDDL SID injection defense already in `validate_package_sid_for_sddl` |
| V6 Cryptography | NO | No new crypto; per-agent SID is a Windows kernel primitive |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Agent B reads Agent A's capability grants via shared pipe | Elevation of privilege | Per-tenant SDDL + `ImpersonateNamedPipeClient` + registry check (DMON-02) |
| Escaped agent pivots to daemon (which is SYSTEM) | Elevation of privilege | Daemon runs at USER privilege, split from WFP service (DMON-03, ADR) |
| Wire-frame `agent_id` / `session_id` spoofing | Spoofing | Authorize by kernel-vouched SID (not wire-frame field) |
| Token reuse blurs tenant isolation | Elevation of privilege | Fresh-per-agent token + job (DMON-01) |
| `ImpersonateNamedPipeClient` without `RevertToSelf` leaks daemon identity | Elevation of privilege | RAII guard for impersonation revert |
| AppContainer profile forge (guessing another agent's profile name) | Spoofing | Profile name alone is not the auth signal; SID in `AgentRegistry` is |
| Denial of service via malformed large capability request | Denial of service | 64 KiB hard cap already in `MAX_MESSAGE_SIZE` constant (line ~113 of `socket_windows.rs`) |

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Windows 10/11 (Job Objects, AppContainer, SCM per-user services) | DMON-01/02/03 | ✓ (CI Win11 host) | 26200.8390 | — |
| `windows-service 0.7` crate | Daemon service host | ✓ (in-tree) | 0.7 | — |
| Signed `nono.exe` or dev-layout | R-B4 broker trust gate (daemon launch path) | ✓ (dev-layout on dev host) | dev | Production: signed MSI |
| `nono-wfp-service` running | NOT required in Phase 74 (D-04; no WFP coupling) | N/A | N/A | — |
| Real Win11 host | SC1/SC3 UAT (concurrent agents; handle baseline test) | ✓ (operator's machine) | Win11 | — |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** none

---

## Open Questions (RESOLVED)

1. **`SeImpersonatePrivilege` for per-user SCM service** — **RESOLVED (A1)**
   - What we know: Interactive users have `SeImpersonatePrivilege` by default on Win10/11. SCM service processes also receive it.
   - What was unclear: Whether a per-user `SERVICE_USER_OWN_PROCESS` service registered under the interactive user's account retains `SeImpersonatePrivilege`.
   - **Accepted position:** ASSUMED PRESENT. Interactive users and the service tokens derived from their logon sessions carry `SeImpersonatePrivilege` on Win10/11 by default (documented Windows behavior). If the Wave 0 spike (74-01) proves the privilege absent in the per-user service token, the daemon falls back to SDDL-primary + `GetNamedPipeClientProcessId` + `IsProcessInJob` auth (no impersonation). The per-tenant SDDL remains the primary gate in both branches; impersonation is defense-in-depth on top.

2. **`TokenAppContainerSid` exact variant name in `windows-sys 0.59`** — **RESOLVED (A2)**
   - What we know: The Win32 `TOKEN_INFORMATION_CLASS` enum has `TokenAppContainerSid = 56`.
   - What was unclear: Whether `windows-sys 0.59` exposes this as `TokenAppContainerSid` by name or only as a numeric value.
   - **Accepted position:** Use the named constant `TokenAppContainerSid` if present in `windows_sys::Win32::Security`; otherwise use the numeric literal `56u32` with a cast and a comment citing this decision. Build-time failure (compile error) is the safe outcome if neither resolves — caught at build, never a runtime regression. The executor must check and record the actual constant form used in 74-02-SUMMARY.md.

3. **Broker trust gate for `nono-agentd.exe`** — **RESOLVED (A6)**
   - What we know: The broker trust gate (`R-B4`) validates the calling binary. Today the caller is `nono.exe`.
   - What was unclear: Whether the gate validates the CALLING binary (`nono-agentd.exe`) or the BROKER binary (`nono-shell-broker.exe`).
   - **Accepted position:** Implement `nono-agentd` as a second `[[bin]]` target inside `crates/nono-cli`; the gate validates the BROKER binary path (not the caller). The executor MUST confirm this by code-reading the broker trust-gate logic in `exec_strategy_windows/launch.rs` during 74-01 and recording the finding in 74-01-SUMMARY.md. If the caller path IS checked, the fix is to ensure `nono-agentd.exe` is built from the same signed nono-cli crate so it passes the path whitelist.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ImpersonateNamedPipeClient` requires `SeImpersonatePrivilege` and per-user SCM services have it | Unspiked Mechanism 2 | If the privilege is absent in a per-user service token, impersonation fails; fallback is SDDL-only + `GetNamedPipeClientProcessId` + `IsProcessInJob` |
| A2 | `TokenAppContainerSid` is the correct variant name in `windows-sys 0.59` | Code Examples + net-new Win32 table | If the name differs, the build fails with a compile error (safe — caught at build time) |
| A3 | 100 fresh-token + fresh-job allocations over time return to the Win32 handle-count baseline | In-Phase Spike | If handles leak, DMON-01 is unachievable without identifying and fixing the leak source. The spike's job is to find this out. |
| A4 | Per-user SCM `SERVICE_USER_OWN_PROCESS` services work correctly on Win11 26200 for a headless process | Least-Privilege USER Service shape | If per-user services are not available (older Win10 build), fall back to foreground mode as the primary and SCM as an enhancement |
| A5 | `DeleteAppContainerProfile` is present in `windows-sys 0.59` under `Win32_Security_Isolation` | Don't Hand-Roll table | If absent, accumulation is mitigated by calling `DeleteAppContainerProfile` via `windows::core::PWSTR` from a `windows` crate import; or the accumulation is accepted as a known limitation and cleaned at daemon startup |
| A6 | The broker trust gate checks the BROKER binary path, not the caller's binary path | Pitfall 6 | If the gate checks the caller, `nono-agentd.exe` must be signed/in the expected path or the launch fails; mitigation is implementing the daemon as a bin target within nono-cli |

---

## Sources

### Primary (HIGH confidence)

- In-tree code (authoritative, current):
  - `crates/nono/src/supervisor/socket_windows.rs` — SDDL construction, `bind_low_integrity_with_session_and_package_sid`, `PIPE_UNLIMITED_INSTANCES`, `build_capability_pipe_sddl`, `CAPABILITY_PIPE_RESTRICTING_SID_MASK 0x0012019F`, 64 KiB frame cap, `RevertToSelf` test at line ~2439
  - `crates/nono/src/agent.rs` — `AgentRegistry`, `AgentClassification`, `read_process_appcontainer_sid`, `Arc<Mutex<AgentRegistry>>` thread-safety doc
  - `crates/nono-cli/src/bin/nono-wfp-service.rs` — service skeleton shape (`define_windows_service!`, `service_dispatcher`, Event Log, control pipe, `--service-mode`, non-Windows stub)
  - `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `CreateJobObjectW`, `AssignProcessToJobObject`, `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, suspended-spawn + terminate-on-assign-failure
  - `crates/nono/src/supervisor/types.rs` — `SupervisorMessage`, `CapabilityRequest`, `session_id` field (confirmed present; lines ~458–479)
  - `crates/nono/Cargo.toml` — `windows-sys 0.59` workspace pin confirmed
  - `crates/nono-cli/Cargo.toml` — `windows-service 0.7` confirmed in-tree
- Milestone research (authoritative documents from prior research sessions, 2026-06-13):
  - `.planning/research/PITFALLS.md` — P1/P4/P5 root causes, warning signs, recovery costs
  - `.planning/research/ARCHITECTURE.md` — daemon architecture, tenant table, per-tenant SDDL pattern
  - `.planning/research/STACK.md` — deliberate non-bumps, net-new Win32 table
  - `.planning/research/SUMMARY.md` — composition rationale, confidence assessment
- Project memory (banked gotchas):
  - `windows_appcontainer_wfp_validated` — `CreateAppContainerProfile` MUST (not derive-only)
  - `windows_hook_interpreter_spawn_gotchas` — preserve `SystemRoot`/`windir`/`SystemDrive` baseline
  - `windows_appcontainer_cap_pipe_reachability` — package-SID READ grant before blocking `ConnectNamedPipe`
  - `feedback_clippy_cross_target` — cross-target gate is mandatory

### Secondary (MEDIUM confidence)

- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` — spike 003 VALIDATED; "Not yet spiked" explicit callout of token/job reuse across many agents + one persistent multi-client capability pipe
- `.planning/phases/73-ai-agent-marker/73-CONTEXT.md` — Phase 73 `AgentRegistry` design, job ACL (D-03 there), package-SID authorization predicate, naming invariant

### Tertiary (LOW confidence — verify before coding)

- Win32 documentation (training knowledge): `ImpersonateNamedPipeClient`, `OpenThreadToken`, `TokenAppContainerSid` value, `SERVICE_USER_OWN_PROCESS`, `SeImpersonatePrivilege` — flagged as [ASSUMED] where used; verify against official Microsoft Learn pages before coding

---

## Metadata

**Confidence breakdown:**

| Area | Level | Reason |
|------|-------|--------|
| Standard stack | HIGH | No new crates; every dependency in-tree and pinned |
| Architecture | HIGH | Grounded in current in-tree code; four functions in `socket_windows.rs` already implement the SDDL and DACL primitives needed |
| `ImpersonateNamedPipeClient` composition | MEDIUM | Win32 semantics from training + in-tree `RevertToSelf` test context; A1/A2 assumptions need spike verification |
| Deterministic reap / handle accounting | MEDIUM | Design is sound; whether 100 agents return to baseline is an empirical question the spike answers |
| Pitfalls | HIGH | Grounded in P1/P4/P5 from `PITFALLS.md` which are themselves grounded in the in-tree code |

**Research date:** 2026-06-14
**Valid until:** 2026-07-14 (30 days for stable Win32 semantics; the in-tree code basis is stable longer)

---

## RESEARCH COMPLETE

**Phase:** 74 - Persistent Multi-Tenant Daemon
**Confidence:** HIGH (architecture and stack); MEDIUM (two specific Win32 runtime behaviors require spike verification)

### Key Findings

- **Token/job fresh-vs-reuse:** Fresh-per-agent is confirmed as the correct model (reuse collapses isolation in three independent ways: restricting SID bleed, WFP scope blur, job membership confusion). The spike must prove 100 successive allocations return to the handle-count baseline. No design uncertainty — only empirical verification needed.

- **`ImpersonateNamedPipeClient` composition:** Composes cleanly with the existing per-tenant SDDL mechanism. The SDDL DACL is the primary gate (prevents connection from wrong-SID client); `ImpersonateNamedPipeClient` + `AgentRegistry` check is a defense-in-depth layer after connection. Win32 call sequence is documented concretely. Two assumptions (A1: `SeImpersonatePrivilege` in per-user service; A2: `TokenAppContainerSid` variant name) need verification during spike.

- **`session_id` suffices as tenant key:** No net-new wire protocol field required. `session_id` is already in every `CapabilityRequest`; it serves as a routing hint; the kernel-vouched AppContainer SID from impersonation is the authorization signal. SC5 directive satisfied.

- **Deterministic reap:** `AgentTenant` owning-struct + `Drop` pattern is the right shape. `OwnedHandle` is already in the codebase. Key addition: `DeleteAppContainerProfile` in `Drop` to prevent HKCU registry accumulation. Reaper: either job completion port (IOCP, lower per-agent overhead) or `spawn_blocking` + `WaitForSingleObject` per agent (simpler, adequate for Phase 74 scale).

- **Least-privilege USER service:** `SERVICE_USER_OWN_PROCESS` per-user SCM service with `windows-service 0.7` (already in-tree). Key divergences from `nono-wfp-service.rs`: account type (USER not LocalSystem), service type (per-user not machine-wide), no WFP calls (D-04). Non-Windows stub required (CLAUDE.md cross-target gate).

- **In-phase spike:** Wave 0. Three-clause spike: (1) fresh-token isolation (distinct pkg SIDs), (2) deterministic reap (handle baseline at N=100), (3) cross-tenant denial (in-process impersonation test). Gate Wave 1 on all three passing on real Win11.

### File Created

`.planning/phases/74-persistent-multi-tenant-daemon/74-RESEARCH.md`

### Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard Stack | HIGH | All dependencies in-tree and pinned; no new crates |
| Architecture | HIGH | Four existing in-tree functions implement all primitives; composition is clear |
| `ImpersonateNamedPipeClient` semantics | MEDIUM | A1 (SeImpersonatePrivilege) and A2 (TokenAppContainerSid variant) need spike-time verification |
| Handle accounting (reap) | MEDIUM | Design is proven correct; empirical baseline test is the spike's job |
| Pitfalls | HIGH | P1/P4/P5 from milestone research are grounded in in-tree code |

### Open Questions (RESOLVED)

All three open questions from this section are resolved with accepted positions. See `## Open Questions (RESOLVED)` above for the full decision rationale.

1. **A1 — `SeImpersonatePrivilege`:** ASSUMED PRESENT; fallback to SDDL-primary + `GetNamedPipeClientProcessId` + `IsProcessInJob` if the spike (74-01) proves absent.
2. **A2 — `TokenAppContainerSid` variant name:** Use the named constant if present in windows-sys 0.59; else use `56u32`. Build failure is the safe outcome. Executor records the actual form in 74-02-SUMMARY.md.
3. **A6 — Broker trust gate:** Implement `nono-agentd` as a second `[[bin]]` in `crates/nono-cli`; gate validates BROKER binary (not caller). Executor confirms by code-reading `launch.rs` during 74-01.

### Ready for Planning

Research complete. Planner can now create PLAN.md files. The spike harness (Wave 0) must be planned as the FIRST wave gating all subsequent daemon implementation work.
