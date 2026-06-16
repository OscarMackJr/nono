# Project Research Summary

**Project:** nono v2.12 "AI Agent Abstraction" (SEED-004)
**Domain:** Engine-agnostic AI-agent confinement on Windows -- productionized launch-and-confine + binding-exposed confined-run API + AI_AGENT process marking + persistent multi-tenant capability daemon
**Researched:** 2026-06-13
**Confidence:** HIGH (every primitive is grounded in current in-tree code; spike 003 VALIDATED supplies the launch primitive; the only genuinely unproven part -- multi-agent token/job reuse -- is isolated and flagged)

## Executive Summary

This is a **composition milestone, not a green-field one.** Three existing subsystems already implement nearly everything the four new components need: the launch-and-confine primitive (exec_strategy_windows/launch.rs -> broker arm -> Sandbox::apply, proven engine-neutral by spike 003 for cmd/powershell/python), the capability IPC (socket_windows.rs -- SDDL-scoped named pipes, PIPE_UNLIMITED_INSTANCES, per-SID DACL ACEs, framed JSON, re-accept), and the long-running user-mode service shape (nono-wfp-service.rs -- SCM dispatch, Event Log, control pipe, MSI registration, non-Windows stub). The work is **composition plus a thin amount of net-new Win32** (named job objects for the marker; multi-client accept-loop generalization), plus a binding API surface -- *not* new framework adoption. The single biggest "what NOT to do" is the kernel driver: out of scope by milestone definition AND by ADR-65 (No-go). User-mode only.

The recommended approach is **launch-and-confine as the center of gravity**, not generalizing the Claude PreToolUse hook. nono confines at the OS process boundary; if nono *parents* the engine process, then the engine in-process file writes, in-process Python exec(), and subprocess shells are ALL confined transitively. This is the killer argument over per-tool hooking (which costs a spawn per call, requires every engine to expose a rewrite-capable pre-hook, and cannot confine in-process ops). Every target engine except Windows-native Cursor (which is WSL-only -- an engine limitation, not nono's) fits launch-and-confine cleanly. The abstraction boundary is captured in a small, concrete contract (E1-E5) that every engine must satisfy.

The key risk is the **persistent multi-tenant daemon** (the former spike 004 -- the riskiest, unspiked component). Its load-bearing security properties are all things an ephemeral per-invocation supervisor never had to get right: cross-tenant pipe isolation, an unforgeable agent marker, a least-privilege daemon that is NOT merged into the elevated WFP service, and correct token/job handle lifetime in a process that lives for days. These pitfalls must become **phase success criteria with negative tests** (e.g. "tenant B is denied tenant A's grants," "100-agent launch/exit returns handle count to baseline"). The roadmap must build single-engine launch FIRST, then binding || marker, and only attempt the daemon once that foundation is a gated, working code path.

## Key Findings

### Recommended Stack

Almost nothing new needs to be added. Every NEW dependency already lives in-tree at a pinned version; the deltas are *features* on existing crates, the marker's named-job Win32 calls, and bumping stale internal nono pins in the bindings. Deliberate non-bumps: stay on windows-sys 0.59 (not 0.61.x -- gratuitous cross-target-drift churn), stay on napi 2 (not 3 -- a breaking migration that would balloon scope), and avoid any new wire protocol (the framed JSON SupervisorMessage is proven, bounded, replay-guarded). See [STACK.md](STACK.md).

**Core technologies:**
- windows-sys **0.59** (keep pin; add features only) -- Raw Win32 for named job objects, multi-client pipes, SID/token query. Every needed API (Win32_System_JobObjects, _Pipes, _Security_Isolation) is present; bumping buys nothing and adds drift risk.
- windows-service **0.7** (in-tree) -- SCM-hosted long-running service. The daemon is a *second instance* of the pattern nono-wfp-service already ships and signs in the MSI.
- nono lib **0.62.2** + broker arm (windows_low_il_broker:true) -- the validated confining primitive; the launcher uses it unchanged.
- tokio 1.x (net/io-util/sync) -- async multi-client accept loop for the daemon (sync per-thread is fine for a small known N; tokio for unbounded tenants).
- pyo3 **0.28** / napi **2** (keep) -- expose confined_run/confinedRun on the existing binding surfaces; bump only the internal nono pin (0.57.0 / 0.33.0 -> 0.62.x).

**Net-new Win32 (no new crate):** CreateJobObjectW with a *name*, OpenJobObjectW, IsProcessInJob/QueryInformationJobObject for the AI_AGENT marker.

### Expected Features

The abstraction boundary contract (E1-E5) is the spine of the milestone. E1-E4 are the **launch-and-confine** contract (already validated); **E5 (a pre-execution interception point) is the new contract** for engines nono cannot parent, and the part with real per-engine variance. See [FEATURES.md](FEATURES.md).

**The abstraction boundary contract (E1-E5):**
- **E1** -- Engine executable + interpreter path(s) (python.exe/node.exe); launch profile supplies --allow, fail-secure refuse if uncovered.
- **E2** -- An ownable launch command (argv + env); nono must be the parent, else fall back to E5/adopt.
- **E3** -- Intended writable workspace as an ABSOLUTE path (engines do NOT inherit launcher CWD -- PowerShell resolved a relative write to C:\).
- **E4** -- A network identity (AppContainer package SID, broker no-PTY arm) for per-agent WFP scoping.
- **E5** *(hook-camp only)* -- A pre-execution interception point, used ONLY when nono cannot be the parent (Claude PreToolUse built; Copilot JSON-RPC; Cursor permission gate).

**Must have (table stakes):**
- Generic launch-and-confine productionized -- the headline promise; de-spike the validated 003 path.
- Per-engine launch profiles (Aider + LangChain-Python, both python.exe) carrying E1+E3.
- Fail-secure exe/interpreter coverage gate per engine -- core nono invariant.
- Workspace-ownership/relabel handling + clear R-B3 diagnostic (admin-owned dir -> no WRITE_DAC -> confined write fails secure but opaque).
- Per-engine fit documentation (launch-and-confine vs hook vs Cursor-WSL-only).

**Should have (competitive):**
- nono-py engine binding -- proves "engine is a variable" in code on a real LangChain agent with NO Claude hook; directly addresses the in-process-exec() case. The formal abstraction proof.
- Persistent multi-tenant daemon -- the marquee v2.12 capability (run several confined agents at once, zero per-launch startup); riskiest, former spike 004.
- Uniform WFP per-agent network enforcement -- Docker-grade egress control without Docker, works for in-process network calls too.

**Defer (v2+):**
- Post-hoc demote control -- supplementary IR lever only, never the boundary (leaky/unsound as primary).
- Native Windows Cursor -- engine-blocked (CLI is Linux/macOS-only).
- Cross-platform parity of the abstraction (Landlock/Seatbelt) -- lower priority for a Windows milestone.

### Architecture Approach

Four new components map cleanly onto the three existing subsystems. The architecturally distinct path is the **in-process-exec() case (LangChain)**: there is no child boundary to wrap per tool call, so the binding must expose BOTH confined_run(exe, args, ...) (Shape A -- spawn a confined child, identical to spike 003) AND an in-process confine(caps) startup call (Shape B -- apply the sandbox to SELF before any risky work). Shape B is sound only if applied at process startup before any privileged handle is opened. See [ARCHITECTURE.md](ARCHITECTURE.md).

**Major components:**
1. **Engine-agnostic launch path** (MODIFIED) -- generalize the broker arm to parent any covered engine; add per-engine profiles + the AI_AGENT named-job/SID.
2. **AI_AGENT marker** (NEW Win32 + reuse) -- per-agent *named* job object (kill-group + descendant capture + enumeration) PLUS a per-agent identity SID (the authorization signal).
3. **nono agentd persistent multi-tenant daemon** (NEW binary) -- modeled byte-for-byte on nono-wfp-service.rs; launches/adopts many agents, owns one persistent multi-client pipe, tracks per-tenant state.
4. **Multi-tenant capability pipe** (MODIFIED socket_windows.rs) -- one persistent name, N concurrent tenants, each scoped to its own SID via per-tenant SDDL.
5. **nono-py / nono-ts confined_run + confine** (NEW API on existing bindings).

### Critical Pitfalls

The pitfalls below are NEW with a persistent multi-tenant daemon -- the per-invocation traps (post-hoc IL-drop demote-only, R-B3 user-owned workspace, exe-coverage gate, absolute grants, broker dev-layout/signing) carry forward unchanged. Each of these MUST become a phase success criterion with a negative test. See [PITFALLS.md](PITFALLS.md).

1. **Cross-tenant capability theft (load-bearing daemon security property)** -- One shared DACL admitting "any Low-IL same-session process" admits every tenant to every pipe instance. Avoid by authorizing against a kernel-vouched identity: server-side ImpersonateNamedPipeClient + GetTokenInformation (read session/package SID, job membership) + per-tenant SDDL pipe instances. Treat any agent_id in the wire frame as an untrusted routing hint only. Gate the phase on a "tenant B denied tenant A's grants" test.
2. **AI_AGENT marker forge/shed** -- A named job is openable by name (a rendezvous, not a secret); env-var/argv markers are trivially forgeable; an agent can break away from the job. The marker must be an **unforgeable token SID established by the daemon at spawn-time**, NOT the job name. Use the named job for kill-group/enumeration; use the token SID for authorization. Deny breakaway; ACL the job daemon-only. Adopted (not launched) agents are best-effort/demote-only.
3. **In-process-exec() engine cannot be confined post-hoc** -- There is structurally nothing to wrap for LangChain PythonREPLTool exec(). The daemon must be the PARENT of the interpreter (parent-or-self-confine); where it cannot, the binding answer is sandbox-self at startup. Never "confine LangChain" by finding a running python.exe and dropping its IL.
4. **Daemon attack surface / privilege** -- A persistent always-on launcher that runs as SYSTEM (like nono-wfp-service) gives an escaped agent a pivot to ALL tenants + the host. **Least-privilege split:** run the launcher at USER privilege; keep WFP filter manipulation in the SEPARATE elevated nono-wfp-service behind its narrow control pipe. Write the privilege model down as an ADR before coding the service host. The pipe is query-only -- never expands a running agent capabilities (no escape hatch).
5. **Token & job-object handle lifetime** -- A daemon living for days leaks handles, and reusing one token/job across agents collapses tenant isolation (B inherits A restricting SID + workspace relabel; WFP scope blurs). **One fresh confining token + one fresh job per agent**; tie every per-agent resource to a single owning struct with a Drop that closes all of them; reap exited agents deterministically (wait on job completion/process handle). Verify with a 100-agent launch/exit handle-count-returns-to-baseline test.
6. **Nested-job collisions / silent confinement loss** -- A daemon launching arbitrary engines routinely meets already-jobbed processes; AssignProcessToJobObject then fails or only nests, and the failure is often silent. Spawn suspended, assign to the AI_AGENT job BEFORE any code runs, fail-secure (terminate) on assign failure; no UI limits on the job.

## Implications for Roadmap

The dependency-ordered build sequence is the most important output of this research. **Single-engine launch productionization comes FIRST**; everything else sits on top of it. The quality gate ("single-launch before the multi-tenant daemon, and the in-process-exec case addressed") is structurally enforced by this ordering.

### Phase A: Engine-agnostic launch path (productionize spike 003)
**Rationale:** The whole milestone is "make nono run -- <engine> Just Work per engine." The daemon, binding, and marker all sit on top of the productionized launch path -- nothing can precede it.
**Delivers:** The validated 003 path promoted to a first-class (de-spiked) code path; per-engine profiles for Aider + LangChain-Python (both python.exe); fail-secure per-engine exe/interpreter coverage; workspace-ownership/relabel handling + R-B3 diagnostic; per-engine fit docs.
**Addresses:** Generic launch-and-confine; engine launch profiles; fail-secure coverage gate; R-B3 handling; per-engine fit docs (all table-stakes from FEATURES.md).
**Avoids:** Pitfall 6 (nested-job collisions -- spawn-suspended-then-assign, fail-secure on assign failure, no UI limits).
**Gate:** A non-Claude engine (Aider) confined end-to-end on a real Win11 host.

### Phase B: nono-py binding + in-process-exec() proof
**Rationale:** Depends only on Phase A launch semantics; independent of the daemon -- can run in PARALLEL with Phase C. Proves the abstraction in-library and directly addresses the in-process-exec case.
**Delivers:** Internal nono pin bump (0.57.0 -> 0.62.x); confined_run(exe, args, allow, profile) (Shape A); confine(caps) in-process self-confinement entrypoint (Shape B); a real Python/LangChain agent confined with NO Claude hook.
**Uses:** pyo3 0.28 (kept), nono 0.62.x (STACK.md).
**Implements:** The nono-py confined_run + confine component; the in-process-exec() data-flow path (ARCHITECTURE.md Shape A/B).
**Avoids:** Pitfall 3 (in-process engine confinement -- the binding MUST demonstrate sandbox-self at startup, not just external launch).
**Gate:** LangChain PythonREPLTool exec() write outside workspace denied; inside allowed.

### Phase C: AI_AGENT marker
**Rationale:** Depends only on Phase A; independent of Phase B (parallel-capable). The daemon prerequisite.
**Delivers:** Named job object (CreateJobObjectW(name) + OpenJobObjectW); per-agent identity SID (session SID + AppContainer package SID); IsProcessInJob/QueryInformationJobObject identification/enumeration.
**Uses:** windows-sys 0.59 Win32_System_JobObjects features (STACK.md).
**Implements:** The AI_AGENT marker component (ARCHITECTURE.md Pattern 1).
**Avoids:** Pitfall 2 (marker forge/shed -- marker is an unforgeable spawn-time token SID, NOT a named job; deny breakaway; ACL the job daemon-only).
**Gate:** Launch marks; an arbitrary PID is correctly classified as AI_AGENT or not; a non-daemon-spawned process cannot acquire the marker.

### Phase D: Persistent multi-tenant daemon (riskiest; former spike 004)
**Rationale:** Depends on A + C. The daemon is launch-and-confine (A) + marker (C) + multi-client pipe + the genuinely unspiked token/job reuse risk (isolated inside this phase). Cannot start until A is a gated, working code path -- this is the quality gate.
**Delivers:** nono agentd binary modeled on nono-wfp-service.rs (SCM, Event Log, control pipe, non-Windows stub, MSI reg, non-fatal start); multi-client capability pipe (PIPE_UNLIMITED_INSTANCES + per-tenant SDDL, tenant table keyed on session_id); per-agent capability/policy resolution; token/job reuse across agents; LaunchAgent/AdoptAgent verbs.
**Uses:** windows-service 0.7, tokio named-pipe server, the framed JSON SupervisorMessage (extend with tenant id ONLY if session_id proves insufficient).
**Implements:** The daemon + multi-tenant pipe components (ARCHITECTURE.md Patterns 2 & 3).
**Avoids:** Pitfall 1 (cross-tenant theft -- server-side ImpersonateNamedPipeClient + per-tenant SID); Pitfall 4 (privilege -- least-privilege USER-level launcher split from the elevated WFP service, ADR first); Pitfall 5 (lifetime -- fresh token+job per agent, deterministic reap).
**Gate:** Two concurrent confined agents, each served independently over one pipe, each scoped to its own SID; cross-tenant request rejected; 100-agent launch/exit returns handle count to baseline.

### Phase E: Supplementary controls + secondary engines (optional)
**Rationale:** Demote is supplementary (must follow a proven launch-time default); WFP per-agent and second-engine/second-binding profiles are low-cost adds once the shape is proven.
**Delivers:** Post-hoc demote control as a daemon "demote tenant" verb (framed explicitly as demote-only); uniform WFP per-agent egress via the existing elevated nono-wfp-service (E4 SID per tenant); Copilot CLI profile (second node engine); nono-ts confinedRun parity (bump internal nono pin 0.33.0 -> 0.62.x).
**Avoids:** the "detect-and-confine as primary model" anti-feature (demote stays an IR lever, never the boundary).

### Phase Ordering Rationale

- **A before everything** -- the daemon, binding, and marker all consume the productionized launch path; it is the foundation.
- **B || C** -- both depend only on A and are independent of each other, so they can run in parallel. B (binding) is the in-library abstraction proof + the in-process-exec answer; C (marker) is the daemon prerequisite.
- **D after A + C** -- the daemon = launch-and-confine + marker + multi-client pipe + the unspiked token/job-reuse risk. The quality gate is satisfied structurally: D cannot start until A is gated and C exists. The unproven part (D4, token/job reuse) is isolated and is where a spike-inside-the-phase belongs.
- **E last** -- demote must follow a proven launch-time default; the rest are low-cost adds.
- **Pitfall coverage by ordering** -- Pitfall 6 lands in A; Pitfall 3 in B; Pitfall 2 in C; Pitfalls 1, 4, 5 (the daemon load-bearing security properties) all land in D as success criteria.

### Research Flags

Phases likely needing deeper research during planning (/gsd:plan-phase --research-phase <N>):
- **Phase D (multi-tenant daemon):** The token/job *reuse across agents* is the explicitly UNSPIKED part of spike 003/004 -- the highest-risk unknown in the milestone. Server-side client authentication (ImpersonateNamedPipeClient is not yet present in socket_windows.rs), the least-privilege/privilege-model ADR, and nested-job adopt-mode semantics all warrant a spike-inside-the-phase.
- **Phase B (in-process self-confine, Shape B):** Sandbox-self via Sandbox::apply on the *current* process at startup is a usage pattern not yet exercised by the bindings; the soundness boundary (must precede any privileged handle open) needs validation.

Phases with standard patterns (skip research-phase):
- **Phase A:** spike 003 VALIDATED; the path is proven for cmd/powershell/python. De-spiking + profiles are well-understood; only the R-B3/coverage diagnostics are new work.
- **Phase C:** Named-job Win32 semantics are documented (Microsoft Learn) and the unnamed-job lifecycle already exists in exec_strategy_windows/; the delta is small (add a name + OpenJobObjectW/IsProcessInJob).
- **Phase E:** demote (spike 002), WFP (Phase 62), and node-engine profiles (Claude) are all proven shapes.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Every NEW dependency already lives in-tree at a pinned version; deltas verified against crates.io. Net-new Win32 is feature-additions on windows-sys 0.59, not a new crate. |
| Features | HIGH | Engine launch models verified against vendor docs + DeepWiki; confinement primitive proven by spike 003 (cmd/powershell/python confined identically). E5 per-engine variance is the one MEDIUM-ish edge (vendor hook surfaces churn). |
| Architecture | HIGH | Every integration point grounded in current in-tree code (socket_windows.rs, launch.rs, nono-wfp-service.rs); spike 003 supplies the launch primitive. |
| Pitfalls | HIGH | Grounded in the in-tree code, banked spike 001-003 findings, project memory, and verified Win32 job-object semantics. The daemon pitfalls are the load-bearing ones and are well-characterized. |

**Overall confidence:** HIGH

### Gaps to Address

- **Token/job reuse across many agents (Phase D, former spike 004):** The single genuinely unproven part of the milestone. Handle during planning by scoping a spike INSIDE Phase D (or a dedicated pre-D spike) gated on: fresh-token-per-agent isolation, deterministic reap (100-agent handle-baseline test), and cross-tenant denial.
- **Server-side client authentication in the pipe (Phase D):** ImpersonateNamedPipeClient is NOT currently in socket_windows.rs (the code verifies the *server* PID from the client side, the inverse). The accept-path impersonation + per-tenant SID match is net-new and is the load-bearing cross-tenant property -- validate early.
- **Daemon privilege model (Phase D):** Must be written down as an ADR (least-privilege USER-level launcher vs. the separate elevated WFP service) BEFORE coding the service host, to avoid inheriting the nono-wfp-service SYSTEM posture by default.
- **In-process sandbox-self soundness (Phase B):** Validate that Sandbox::apply on the current process is sound only when called before any privileged handle is opened; document that the agent must call confine() first.
- **AppContainer per-agent SID:** Must CreateAppContainerProfile, not derive-only (else CreateProcessW ERROR_FILE_NOT_FOUND); preserve SystemRoot/windir/SystemDrive env baseline (else CLR 0xFFFF0000). Banked, but re-assert per phase.

## Sources

### Primary (HIGH confidence)
- In-tree code: crates/nono/src/supervisor/socket_windows.rs (SDDL pipe, PIPE_UNLIMITED_INSTANCES, per-SID ACEs, framed JSON, re-accept, server-PID verify, NO server-side ImpersonateNamedPipeClient -- the multi-tenant gap); crates/nono-cli/src/exec_strategy_windows/launch.rs (job lifecycle, KILL_ON_JOB_CLOSE, suspended-spawn + terminate-on-assign-failure); crates/nono-cli/src/bin/nono-wfp-service.rs (windows-service 0.7 service skeleton); crates/nono/Cargo.toml (windows-sys 0.59); ../nono-py/Cargo.toml (pyo3 0.28); ../nono-ts/Cargo.toml (napi 2).
- Banked spike findings: .claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md (spike 003 VALIDATED; E1-E5 contract; R-B3/R-B4); windows-confinement-model.md (spikes 001 INVALIDATED / 002 PARTIAL -- post-hoc IL-drop leaky/demote-only).
- Project memory: windows_appcontainer_wfp_validated (AppContainer SID + CreateAppContainerProfile); windows_hook_interpreter_spawn_gotchas (env baseline); feedback_clippy_cross_target (cross-target gate); project_v210_opened (ADR-65 No-go on kernel driver).
- Milestone scope: .planning/PROJECT.md "Current Milestone: v2.12 AI Agent Abstraction".

### Secondary (MEDIUM confidence)
- GitHub Copilot CLI -- Tool Execution & Permissions (DeepWiki) -- all tool calls through one validation pipeline; shell + extensions as child processes.
- Aider Documentation / lint-test -- python process, in-place file edits, /run shell.
- Cursor CLI -- Installation -- ~/.local/bin, Linux/macOS only; WSL on Windows.
- LangChain Deep Agents / tools overview -- PythonREPLTool in-process exec(), subprocess ShellTool.
- Microsoft Learn: Job Objects and AssignProcessToJobObject -- named-job semantics, Windows 8+ nested-job rules.
- crates.io windows-service (0.8.1); docs.rs pyo3 (0.28.3); docs.rs napi (3.8.4, deliberately staying on 2); docs.rs windows-sys (0.61.2, deliberately staying on 0.59).

### Tertiary (LOW confidence)
- cursor-agent-cli-windows community patch -- evidence the official Cursor CLI is non-native on Windows.

---
*Research completed: 2026-06-13*
*Ready for roadmap: yes*
