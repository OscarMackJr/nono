# Architecture Research

**Domain:** Engine-agnostic AI-agent confinement on Windows — persistent multi-tenant daemon + `AI_AGENT` process marking + engine-agnostic launch path + binding-exposed confined-run API (nono v2.12 "AI Agent Abstraction")
**Researched:** 2026-06-13
**Confidence:** HIGH (every integration point grounded in current in-tree code; spike 003 VALIDATED supplies the launch primitive)

## Executive Framing

This milestone is **composition over invention**. Three existing subsystems already implement nearly everything the four new components need:

1. **The launch-and-confine primitive** (`exec_strategy_windows/launch.rs` → `select_windows_token_arm` → `BrokerLaunch`/`BrokerLaunchNoPty` → `nono-shell-broker`) already confines arbitrary executables (cmd/powershell/python proven identically, spike 003). The engine is already a variable — the Claude-specificity lives only in the PreToolUse hook.
2. **The capability IPC** (`crates/nono/src/supervisor/socket_windows.rs`) already implements SDDL-scoped named pipes with `PIPE_UNLIMITED_INSTANCES`, per-session/per-package SID DACL ACEs, framed JSON `SupervisorMessage`, bounded reads, and disconnect/re-accept.
3. **The long-running user-mode service shape** (`crates/nono-cli/src/bin/nono-wfp-service.rs`) already implements SCM dispatch, Event Log, a control pipe, MSI registration, and a non-Windows stub `main` — the exact daemon skeleton.

The four new components map cleanly onto these three: the **daemon** generalizes the service shape + the IPC accept-loop to many tenants; the **`AI_AGENT` marker** adds a *named* job object (the one piece of genuinely net-new Win32) plus a per-agent SID; the **engine-agnostic launch path** generalizes the broker arm + adds per-engine launch profiles; the **binding API** (`nono-py`) exposes the same launch path with one twist — the in-process-`exec()` case (LangChain) where confinement must be applied **from inside** the agent process at startup, since there is no child boundary to wrap.

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│  CALLER SURFACE                                                            │
│  ┌────────────┐  ┌──────────────┐  ┌───────────────────────────────────┐  │
│  │ nono run   │  │ nono-py /    │  │ nono agentd (NEW daemon)          │  │
│  │ (single    │  │ nono-ts      │  │  client: CLI subcommand / binding │  │
│  │  launch)   │  │ confined_run │  │  talks to daemon control pipe     │  │
│  └─────┬──────┘  └──────┬───────┘  └───────────────┬───────────────────┘  │
├────────┼─────────────────┼─────────────────────────┼──────────────────────┤
│        │      ENGINE-AGNOSTIC LAUNCH PATH (MODIFIED) │                      │
│        │   ┌──────────────────────────────────────────────────────────┐   │
│        └──▶│ exec_strategy_windows/launch.rs                           │◀──┘
│           │   select_windows_token_arm → BrokerLaunch[NoPty]           │   │
│           │   + per-engine launch profile (E1 exe/interp, E3 workspace)│   │
│           │   + AI_AGENT named-job + per-agent SID (NEW)               │   │
│           └───────────────┬──────────────────────────────────────────┘   │
├───────────────────────────┼────────────────────────────────────────────────┤
│   CONFINEMENT PRIMITIVE    │  (UNCHANGED — nono lib + broker)               │
│   ┌────────────────────────▼─────────┐   ┌─────────────────────────────┐   │
│   │ nono-shell-broker (Medium IL)    │   │ nono lib: Sandbox::apply    │   │
│   │  builds Low-IL primary token /   │──▶│  CapabilitySet, NO_WRITE_UP │   │
│   │  AppContainer; CreateProcess child│   │  WFP package/AppID scope    │   │
│   └────────────────┬─────────────────┘   └─────────────────────────────┘   │
│                    │ (confined engine process: cmd/python/node/aider…)      │
├────────────────────┼────────────────────────────────────────────────────────┤
│  MULTI-TENANT IPC   │  (MODIFIED: socket_windows.rs generalized N tenants)  │
│   ┌─────────────────▼────────────────────────────────────────────────────┐ │
│   │ Capability pipe: \\.\pipe\nono-agentd-<rendezvous>                    │ │
│   │   PIPE_UNLIMITED_INSTANCES · per-tenant SDDL (session+package SID)    │ │
│   │   framed JSON SupervisorMessage (keyed on session_id) · replay gate    │ │
│   └──────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────────┤
│  ENFORCEMENT (UNCHANGED): Low-IL mandatory label · WFP (nono-wfp-service)     │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | New / Modified / Unchanged | Built From |
|-----------|----------------|----------------------------|------------|
| **`nono agentd` (persistent multi-tenant daemon)** | Long-running user-mode host that launches/adopts multiple agents, owns one persistent multi-client capability pipe, tracks per-tenant state (job handle, SID, profile), serves capability requests | **NEW binary** | `nono-wfp-service.rs` service skeleton (SCM dispatch, Event Log, control pipe, non-Windows stub, MSI reg) |
| **Engine launch profiles** | Carry per-engine E1 (exe+interpreter coverage) and E3 (absolute writable workspace) facts so `nono run -- <engine>` Just Works | **NEW data** (`policy.json` / profile entries) | existing `claude-code` profile shape (`target_binary`, `windows_low_il_broker`) |
| **`AI_AGENT` marker** | Per-agent **named** job object (`nono-ai-agent-<id>`) + per-agent identity SID; kill-group + descendant capture + enumerable identity | **NEW Win32** (named-job create/open) + reuse | `exec_strategy_windows/` job lifecycle (unnamed today); add `OpenJobObjectW`, `IsProcessInJob` |
| **Per-agent capability/policy resolution** | Map a pipe request's tenant correlator → that agent's CapabilitySet/profile → approve/deny | **NEW logic** in daemon | existing policy resolver (`policy.rs`) + `CapabilityRequest` |
| **Engine-agnostic launch path** | Generalize the broker arm to parent any covered engine; honor exe-coverage gate + absolute grants + user-owned workspace | **MODIFIED** | `select_windows_token_arm` / `BrokerLaunch[NoPty]` (already engine-neutral) |
| **Multi-tenant capability pipe** | One persistent pipe name, N concurrent tenants, each scoped to its own SID via SDDL | **MODIFIED** | `socket_windows.rs` (`bind_*_with_session_and_package_sid`, `PIPE_UNLIMITED_INSTANCES`, re-accept) |
| **`nono-py` / `nono-ts` `confined_run`** | Binding-exposed confined launch; **plus** in-process self-confinement entrypoint for the no-child-boundary case | **NEW API** on existing bindings | `../nono-py` (PyO3 0.28), `../nono-ts` (napi 2); bump stale internal `nono` pin to 0.62 |
| **nono lib / broker / WFP service** | Confinement primitive, token construction, kernel network enforcement | **UNCHANGED** | as-is |

## The Abstraction Boundary Contract

Every engine must expose the following for nono to mediate it. E1–E4 are the **launch-and-confine** contract (validated by spike 003); E5 is the fallback for engines nono cannot parent.

| # | Exposed thing | How nono consumes it | Owner |
|---|---------------|----------------------|-------|
| **E1** | Engine **executable + interpreter path(s)** | Launch profile supplies `--allow <exe-dir>` for `python.exe`/`node.exe`/engine binary; fail-secure refuse if uncovered | Launch profile |
| **E2** | An **ownable launch command** (argv + env) | Daemon/CLI is the parent; if a third party owns the spawn, fall back to E5 or adopt | Launch path |
| **E3** | Intended **writable workspace as an ABSOLUTE path** | Granted + relabeled Low-writable; engines do NOT inherit launcher CWD | Launch profile + caller |
| **E4** | A **network identity** (AppContainer package SID, broker no-PTY arm) | Per-agent WFP scoping; daemon assigns one SID per tenant | Marker / launch path |
| **E5** | *(hook-camp only)* A **pre-execution interception point** | Only when nono cannot be the parent (IDE-embedded engine, already-running process). Claude PreToolUse (built); Copilot JSON-RPC hooks; Cursor permission gate | Engine vendor |

**Contract invariants (banked, non-negotiable):**
- The exe/interpreter-coverage gate is fail-secure: an uncovered binary is refused, never silently degraded.
- All grants are absolute paths — never assume CWD inheritance (PowerShell resolved a relative write to `C:\`).
- The workspace must be **user-owned** — an elevated/admin-owned dir defeats the DACL/label grant (no `WRITE_DAC`, R-B3).
- Confinement ≥ the per-invocation `nono run` model: `NO_WRITE_UP` + deny-network-unless-granted.

## Data Flow

### Single-launch flow (table-stakes; productionize spike 003)

```
nono run --profile aider --allow <python-dir> -- python -m aider <abs-workspace>
    ↓
[exe-coverage gate]  cover python.exe + aider script? ── no ──▶ fail-secure refuse
    ↓ yes
select_windows_token_arm(is_detached=F, has_pty, has_session_sid=T,
                         caps_demand_low_il, prefers_low_il_broker=T)
    ↓  →  BrokerLaunch (PTY) | BrokerLaunchNoPty (no PTY)
nono-shell-broker (Medium IL) → Low-IL primary token / AppContainer
    ↓  CreateProcess(engine.exe)  [+ relabel workspace Low-writable]
confined engine process  ── in-process file write / exec() / subprocess shell ──▶
    all OS-enforced (Low-IL label + WFP), inherited by descendants
```

### Multi-tenant daemon flow (agent launch → mark → confine → capability requests)

```
client → nono agentd control pipe:  LaunchAgent{ profile, exe, args, workspace }
    ↓
daemon allocates tenant_id (synthetic session SID); 
        CreateJobObjectW("nono-ai-agent-<tenant_id>")                    ← MARK
        derive per-agent SID (session SID + AppContainer pkg SID)
    ↓
daemon → engine-agnostic launch path (same broker arm as single-launch)
    ↓   AssignProcessToJobObject(job, child)                            ← MARK
confined engine process spawned, scoped to tenant's session+package SID
    ↓
engine SDK / hook → CONNECT to \\.\pipe\nono-agentd-<rendezvous>        ← multi-client
    ↓   (SDDL admits only this tenant's SID to its pipe instance)
SupervisorMessage::Request(CapabilityRequest{ ..., session_id=tenant })  [framed JSON]
    ↓
daemon: session_id → per-agent CapabilitySet/profile → policy resolve → decide
    ↓
SupervisorResponse::Decision{ request_id, ApprovalDecision::Approved(ResourceGrant) }
    ↓   (handle brokered via DuplicateHandle into the tenant process, as today)
... pipe stays open; daemon serves the next tenant's request concurrently ...
[demote]  misbehaving tenant → post-hoc IL-drop (supplementary) or TerminateJobObject
```

### In-process-exec() case (LangChain — no child-process boundary)

This is the architecturally distinct path and the reason the binding exists. LangChain's `PythonREPLTool` runs `exec()` **inside** the already-running python process; `ShellTool` spawns a subprocess. There is no child the daemon can wrap *per tool call*.

Two viable shapes, both sound:

```
SHAPE A — external launch (preferred default, identical to spike 003):
  nono parents the python entrypoint.  The whole interpreter is sandboxed.
  → in-process exec() is confined because the PROCESS is confined.
  → subprocess ShellTool children inherit the Low-IL token + job.
  No new mechanism; the binding is just a convenience wrapper over nono run.

SHAPE B — self-confinement from inside (nono-py, the abstraction proof):
  The agent process calls nono.confine(caps) at startup, BEFORE risky work.
  → applies the sandbox to SELF (Sandbox::apply on the current process).
  → all later in-process exec()/file I/O is OS-enforced.
  Caveat (banked from spike 002): self-applied confinement is sound ONLY if
  applied before any privileged handle is opened — i.e. at process startup,
  not mid-run. Mid-run self-demote is the leaky post-hoc IL-drop (demote-only).
```

**Roadmap consequence:** the binding must expose **both** `confined_run(exe, args, …)` (Shape A, spawns a confined child) **and** an in-process `confine(caps)` startup call (Shape B). Shape B is what proves "engine is a variable" for the in-process case — LangChain has no child boundary, so external launch (A) confines it transitively, and self-confine (B) confines it intrinsically. Document that B must be the first thing the agent does.

## Architectural Patterns

### Pattern 1: Named job object as the `AI_AGENT` marker (not a token SID alone)

**What:** One named kernel job per tenant (`CreateJobObjectW(attrs, "nono-ai-agent-<id>")`), reopened across daemon calls via `OpenJobObjectW`, queried via `IsProcessInJob`/`QueryInformationJobObject`.
**When to use:** Whenever the daemon launches or adopts an agent.
**Trade-offs:** Job objects are kernel-tracked, capture descendants, double as the kill-group (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, already wired) and resource-cap, and survive across calls. A SID alone gives identity but not containment or teardown. **Use a per-agent SID *in addition*** (for WFP + per-tenant pipe DACL scoping), not instead.

```rust
// NEW: name the job (today it is unnamed in exec_strategy_windows/)
let job = unsafe { CreateJobObjectW(std::ptr::null(), wide("nono-ai-agent-<id>").as_ptr()) };
// reopen later from the daemon to inspect/terminate a tenant:
let job = unsafe { OpenJobObjectW(JOB_OBJECT_ALL_ACCESS, FALSE, wide(name).as_ptr()) };
let mut in_job: BOOL = 0;
unsafe { IsProcessInJob(proc_handle, job, &mut in_job) }; // "is this PID a marked AI_AGENT?"
```

### Pattern 2: Multi-client capability pipe via per-tenant SDDL scoping

**What:** One persistent pipe name created with `PIPE_UNLIMITED_INSTANCES`; each new client connection is a fresh instance whose SDDL admits **only that tenant's** session+package SID.
**When to use:** The daemon's capability channel.
**Trade-offs:** Reuses the proven `socket_windows.rs` machinery — `CAPABILITY_PIPE_SDDL` base + the per-session/per-package SID ACE appends, the object-specific `CAPABILITY_PIPE_RESTRICTING_SID_MASK = 0x0012019F` mask (NOT a `G*` generic mnemonic — see the second-DACL-pass analysis in that file), framed JSON, 64 KiB cap, bounded read timeout, replay/token gate. The single change is moving from `1` instance + one session SID to N instances + a tenant→SID map.

```text
// MODIFIED: CapabilityRequest already carries session_id; reuse it AS the tenant
// correlator (one synthetic session SID per agent). The daemon keys its tenant
// table on session_id — NO enum/wire change is forced. Add a dedicated tenant_id
// field ONLY if session_id proves insufficient.
```

### Pattern 3: Daemon-as-second-service-instance (reuse the WFP-service skeleton)

**What:** `nono agentd` is a second `windows-service 0.7` host modeled byte-for-byte on `nono-wfp-service.rs`: `define_windows_service!`, `service_dispatcher`, `service_control_handler`, Event Log source, control pipe, `--service-mode`, and the `#[cfg(not(target_os="windows"))]` stub `main`.
**When to use:** Building the daemon binary.
**Trade-offs:** Maximum reuse of audited, MSI-installed, signed service plumbing. The daemon does **not** need elevation for launch/confine (the Low-IL broker runs as the user); it only coordinates with the *separate* elevated `nono-wfp-service` for network enforcement over its existing `\\.\pipe\nono-wfp-control`. Keep the v2.11 non-fatal-service-start posture so a daemon hiccup doesn't roll back the MSI.

## Anti-Patterns

### Anti-Pattern 1: A new wire protocol or new IPC transport for the daemon
**What people do:** Invent protobuf/bincode or a second pipe design for "the multi-tenant case."
**Why it's wrong:** The framed JSON `SupervisorMessage`/`SupervisorResponse` is already bounded (64 KiB), replay-guarded, timeout-bounded, re-accept-capable, and serde-derived. A new format is unjustified security risk in the critical path.
**Do this instead:** Reuse the existing frame; key the daemon's tenant table on the existing `session_id` (one synthetic session SID per agent). Only add a `tenant_id` field if `session_id` proves insufficient.

### Anti-Pattern 2: Post-hoc "detect-and-confine any running AI_AGENT" as the primary model
**What people do:** Enumerate processes, drop their IL after the fact.
**Why it's wrong:** Leaky/unsound — handles/sections/threads opened before the drop survive at higher IL (banked spike 002).
**Do this instead:** Launch-time confinement (broker arm) is the boundary. Post-hoc IL-drop ships ONLY as a supplementary "demote a misbehaving/escaped agent" incident-response lever, never as the wall.

### Anti-Pattern 3: Generalizing the Claude PreToolUse hook to wrap every tool call
**What people do:** Spawn a fresh `nono run` per tool call for all engines (the Claude path's shape).
**Why it's wrong:** Per-call spawn cost × N; requires every engine to expose a rewrite-capable pre-hook (most don't); does **not** confine in-process ops (LangChain `exec()`, Aider in-process writes). It was a workaround for not owning the Claude spawn.
**Do this instead:** Own the *engine* spawn once (launch-and-confine). Keep per-tool hooking (E5) only for engines nono cannot parent.

### Anti-Pattern 4: Bumping `windows-sys`/`napi`/`pyo3` "to be current" inside this milestone
**What people do:** Couple a `windows-sys 0.59→0.61` or `napi 2→3` bump to feature work.
**Why it's wrong:** Cross-target-drift hazard (two cfg-gated compile errors already reached release tags — `feedback_clippy_cross_target`); napi 3 is a breaking surface. No needed API is missing from the current pins.
**Do this instead:** Stay on pinned versions; add *features* to `windows-sys 0.59`, not versions. Bump only the **internal `nono` pin** in the bindings (0.57.0/0.33.0 → 0.62.x) to expose the new API.

## Integration Points

### Internal Boundaries

| Boundary | Communication | New/Modified | Notes |
|----------|---------------|--------------|-------|
| `nono agentd` ↔ launch path | direct fn call (in-process, same binary or `nono-cli` lib) | NEW caller of MODIFIED path | daemon invokes the same `BrokerLaunch[NoPty]` arm `nono run` uses |
| launch path ↔ `nono-shell-broker` | broker command line + handle list (Phase 31 contract) | UNCHANGED | engine-neutral already; just supply per-engine exe/args |
| launch path ↔ `nono` lib | `Sandbox::apply(CapabilitySet)` | UNCHANGED | confinement primitive |
| `nono agentd` ↔ marker | named job create/open/query | NEW Win32 in `exec_strategy_windows/` | `CreateJobObjectW`(named), `OpenJobObjectW`, `IsProcessInJob` |
| confined engine ↔ daemon | multi-client named pipe, framed JSON | MODIFIED `socket_windows.rs` | `PIPE_UNLIMITED_INSTANCES` + per-tenant SDDL (both already exist) |
| `nono agentd` ↔ `nono-wfp-service` | existing WFP control pipe (`\\.\pipe\nono-wfp-control`) | UNCHANGED | daemon requests per-agent egress via the existing elevated service |
| `nono-py`/`nono-ts` ↔ launch path | `confined_run` FFI → same launch path; `confine()` → `Sandbox::apply(self)` | NEW API | bump internal `nono` pin to 0.62.x |
| daemon binary ↔ MSI / SCM | `windows-service 0.7` dispatch + MSI registration | NEW (mirrors WFP service) | reuse v2.11 non-fatal-start posture |

### Cross-Target Discipline (MANDATORY — CLAUDE.md)

Daemon + marker code is `#[cfg(target_os = "windows")]`. Provide a non-Windows stub `main` exactly like `nono-wfp-service.rs` so workspace `cargo check` stays green on Unix, and run cross-target clippy (`x86_64-unknown-linux-gnu` + `x86_64-apple-darwin`). Two cfg-gated compile errors already reached release tags this fork — this is a hard gate.

## Dependency-Ordered Build Sequence

The quality gate requires single-launch before the multi-tenant daemon, and the in-process-exec case addressed. This order respects all feature dependencies from FEATURES.md.

```
PHASE A — Engine-agnostic launch path (productionize spike 003)        [P1, foundation]
  A1. Promote the validated 003 path to a first-class code path (de-spike).
  A2. Per-engine launch profiles: aider + langchain-python (both python.exe)
      carrying E1 (exe+interpreter coverage) + E3 (absolute workspace).
  A3. Fail-secure exe/interpreter coverage gate per engine (exists; assert per profile).
  A4. Workspace-ownership/relabel handling + clear R-B3 diagnostic.
  A5. Per-engine fit docs (launch-and-confine vs hook vs Cursor-WSL-only).
  ── Gate: a non-Claude engine (aider) confined end-to-end on a real Win11 host.

PHASE B — nono-py binding + in-process-exec proof                      [P2, parallel-capable with C]
  B1. Bump internal nono pin 0.57.0 → 0.62.x in ../nono-py.
  B2. confined_run(exe, args, allow, profile) — Shape A (spawn confined child).
  B3. confine(caps) in-process self-confinement entrypoint — Shape B (LangChain exec()).
  B4. Prove: confine a real Python/LangChain agent with NO Claude hook.
  ── Gate: LangChain PythonREPLTool exec() write outside workspace denied; inside allowed.
  (Depends on A's launch semantics; independent of the daemon.)

PHASE C — AI_AGENT marker                                              [P2, prerequisite for D]
  C1. Named job object: CreateJobObjectW(name) + OpenJobObjectW in exec_strategy_windows/.
  C2. Per-agent identity SID (session SID + AppContainer package SID) allocation.
  C3. IsProcessInJob / QueryInformationJobObject: identify/enumerate a marked agent.
  ── Gate: launch marks; an arbitrary PID is correctly classified as AI_AGENT or not.

PHASE D — Persistent multi-tenant daemon (riskiest; former spike 004) [P2/P3, depends A+C]
  D1. nono agentd binary modeled on nono-wfp-service.rs (SCM, Event Log, control pipe,
      non-Windows stub, MSI reg, non-fatal start).
  D2. Multi-client capability pipe: generalize socket_windows.rs to PIPE_UNLIMITED_INSTANCES
      + per-tenant SDDL; key tenant table on session_id (no wire change).
  D3. Per-agent capability/policy resolution: session_id → CapabilitySet/profile → decide.
  D4. Token/job REUSE across many agents (the unproven part — spike inside this phase).
  D5. LaunchAgent / AdoptAgent control verbs over the daemon control pipe.
  ── Gate: two concurrent confined agents, each served independently over one pipe,
           each scoped to its own SID; cross-tenant request rejected.

PHASE E — Supplementary controls + secondary engines                  [P2/P3, optional]
  E1. Post-hoc demote control (spike 002 IL-drop) wired as a daemon "demote tenant" verb.
      Framed explicitly as demote-only, never the boundary.
  E2. Uniform WFP per-agent egress via the existing nono-wfp-service (E4 SID per tenant).
  E3. Copilot CLI profile (second node engine; low marginal cost after A).
  E4. nono-ts confinedRun parity (bump internal nono pin 0.33.0 → 0.62.x).
```

**Ordering rationale:**
- **A before everything** — the whole milestone is "make `nono run -- <engine>` Just Work per engine"; the daemon, binding, and marker all sit on top of the productionized launch path.
- **B and C are independent of each other** and both depend only on A — they can run in parallel. B (binding) proves the abstraction in-library and directly addresses the in-process-exec case; C (marker) is the daemon's prerequisite.
- **D after A + C** — the daemon is launch-and-confine (A) + a marker (C) + a multi-client pipe + token/job reuse. The quality gate ("single-launch before multi-tenant daemon") is satisfied: D cannot start until A is a gated, working code path. D4 (token/job reuse across agents) is the genuinely unspiked risk and is isolated inside D.
- **E last** — demote is supplementary (must follow a proven launch-time default); WFP per-agent and the second-engine/second-binding profiles are low-cost adds once the shape is proven.

## Scaling Considerations

| Scale | Architecture |
|-------|--------------|
| 1 agent | `nono run` single-launch (Phase A). No daemon needed. |
| 2–handful concurrent | Daemon with sync per-tenant threads generalizing `socket_windows.rs` (`PeekNamedPipe` deadline-poll), or tokio per-connection tasks. Reuses proven code. |
| Many concurrent | `tokio::net::windows::named_pipe` `ServerOptions` + `PIPE_UNLIMITED_INSTANCES`, one task per connection (mirrors `nono-wfp-service`'s tokio loop). Per-connection tasks scale without thread blowup. |

**First bottleneck:** capability-pipe accept-loop concurrency — addressed by the tokio-per-connection model when tenant count is unbounded; the sync-per-thread model is fine for a known small N and maximizes reuse of audited code.

## Sources

- In-tree (HIGH, authoritative current code):
  - `crates/nono/src/supervisor/socket_windows.rs` — SDDL-scoped pipe, `PIPE_UNLIMITED_INSTANCES`, `CAPABILITY_PIPE_RESTRICTING_SID_MASK 0x0012019F`, framed JSON, re-accept, per-session/package SID ACEs.
  - `crates/nono/src/supervisor/types.rs` — `SupervisorMessage`/`SupervisorResponse`/`CapabilityRequest`(`session_id`)/`ResourceGrant`(`ApprovalDecision::Approved` inline).
  - `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `select_windows_token_arm`, `WindowsTokenArm::BrokerLaunch`/`BrokerLaunchNoPty`, job-object lifecycle.
  - `crates/nono-cli/src/bin/nono-wfp-service.rs` — `windows-service 0.7` SCM dispatch, Event Log, control pipe (`\\.\pipe\nono-wfp-control`), non-Windows stub `main`, MSI registration.
- Spike blueprint (HIGH): `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` (spike 003 VALIDATED; E1–E5 contract; R-B3 ownership trap).
- Sibling research (HIGH): `.planning/research/STACK.md`, `.planning/research/FEATURES.md` (per-engine fit table; in-process-exec analysis; abstraction-boundary contract).
- Milestone scope (HIGH): `.planning/PROJECT.md` "Current Milestone: v2.12 AI Agent Abstraction".
- Project memory (HIGH): `windows_appcontainer_wfp_validated` (AppContainer SID + `CreateAppContainerProfile`), `windows_hook_interpreter_spawn_gotchas` (env baseline), `feedback_clippy_cross_target` (cross-target gate), `project_v210_opened` (ADR-65 No-go on kernel driver).

---
*Architecture research for: engine-agnostic AI-agent confinement on Windows (nono v2.12)*
*Researched: 2026-06-13*
