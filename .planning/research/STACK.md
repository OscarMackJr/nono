# Stack Research

**Domain:** Engine-agnostic AI-agent confinement on Windows — persistent local security daemon (launcher + multi-tenant capability IPC) + process-marking + binding-exposed confined-run API (nono v2.12 "AI Agent Abstraction")
**Researched:** 2026-06-13
**Confidence:** HIGH (every NEW dependency already lives in-tree at a pinned version; only deltas verified against crates.io)

## Executive Framing

This is a *subsequent* milestone. The dominant finding is that **almost nothing new needs to be added to the stack**. The validated blueprint (spike 003) and the existing Windows backend already supply every primitive the three target features require:

- The persistent daemon's confining primitive is `nono run -- <exe>` — already engine-neutral (cmd/powershell/python proven identically).
- A long-running user-mode Windows service already exists in-tree (`nono-wfp-service`, built on `windows-service 0.7`), so the daemon is a *second instance of a pattern the repo already ships and signs in the MSI*, not a green-field service.
- The multi-tenant capability pipe is a generalization of `crates/nono/src/supervisor/socket_windows.rs`, which already implements SDDL-scoped named pipes, `PIPE_UNLIMITED_INSTANCES` (in `bind_aipc_pipe`), bounded reads, disconnect/re-accept, and per-SID DACL ACEs.
- Job-object lifecycle (`CreateJobObjectW`/`AssignProcessToJobObject`/`QueryInformationJobObject`/`TerminateJobObject`) is already used in `exec_strategy_windows/`.
- The bindings already exist (`../nono-py` PyO3 0.28, `../nono-ts` napi 2).

The work is **composition plus a thin amount of net-new Win32** (named job objects for the `AI_AGENT` marker; multi-client accept-loop generalization), not new framework adoption. The single most important "what NOT to add" is the kernel driver — explicitly out of scope and already covered by the WFP+AppContainer/Low-IL model (ADR-65 No-go).

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `windows-sys` | **0.59** (keep workspace pin — do NOT bump) | Raw Win32 FFI for named job objects, multi-client named pipes, SID derivation, token/SID query | Already the workspace-wide pin (`crates/nono/Cargo.toml` line 63). Every API the daemon/marker/IPC needs is present in 0.59: `Win32_System_JobObjects`, `Win32_System_Pipes`, `Win32_System_Services`, `Win32_Security`, `Win32_Security_Authorization`, `Win32_Security_Isolation`. 0.61.2 exists but a bump is gratuitous churn with cross-target-drift risk (memory `feedback_clippy_cross_target`) and buys nothing for this milestone. |
| `windows-service` | **0.7** (in-tree; latest 0.8.1) | Long-running Windows service host: SCM dispatch (`define_windows_service!`, `service_dispatcher`, `service_control_handler`), start/stop/control lifecycle | Already in-tree at 0.7 (`nono-cli/Cargo.toml` line 144) powering `nono-wfp-service`. The agent daemon is the SAME pattern (SCM-registered, MSI-installed, control-pipe-driven). 0.8.1 (latest, MSRV 1.71) adds polish but 0.7 is sufficient; align the daemon to whatever `nono-wfp-service` uses to avoid a split dependency. |
| `nono` (internal lib) | 0.62.2 (current) | The confining primitive (`Sandbox::apply`, `CapabilitySet`, `SupervisorSocket`, broker arms) | The daemon launches each engine *through* `nono`'s existing Low-IL broker arm (`windows_low_il_broker:true`) — this is the validated sound model, not a new mechanism. |
| `tokio` | 1.x (workspace; features `net`/`io-util`/`sync`/`time`/`macros`) | Async multi-client accept loop + per-tenant task supervision in the daemon | Already the async runtime for CLI/proxy AND already used inside `nono-wfp-service`'s control-pipe loop. A multi-tenant pipe server is naturally one accept-task-per-connection over `tokio::net::windows::named_pipe` (or the existing sync `PeekNamedPipe` poll generalized). |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio::net::windows::named_pipe` (`NamedPipeServer`/`ServerOptions`) | part of `tokio` 1.x | Async multi-client named-pipe server with `create()` re-arm idiom | Preferred for the NEW multi-tenant pipe IF the daemon's accept loop moves to async. The existing `socket_windows.rs` is sync (`PeekNamedPipe` deadline-poll); you can either generalize that to N clients with one thread/connection, or adopt tokio's named-pipe server. Recommend tokio for the daemon (many concurrent tenants) and keep the sync path for the per-`nono run` supervisor. |
| `getrandom` | 0.4 (workspace) | Per-tenant pipe-name nonce + per-agent token/marker nonce | Already used by `socket_windows.rs` (`unique_pair_name`, `create_nonce_hex`). Reuse verbatim for per-`AI_AGENT` rendezvous naming. |
| `sha2` | 0.11 (workspace) | Stable pipe-name derivation from rendezvous path / agent id | Already used in `pipe_name_from_rendezvous_path`. Reuse for deterministic multi-tenant pipe names. |
| `serde` / `serde_json` | workspace | Capability-request wire protocol over the multi-tenant pipe (already the `SupervisorMessage`/`SupervisorResponse` framing) | The per-agent capability protocol IS the existing length-prefixed JSON frame — extend `SupervisorMessage` with an agent/tenant id; do not invent a new wire format. |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 (`env-filter`) | Daemon structured logging + Windows Event Log surface | `nono-wfp-service` already does both (Event Log source + tracing); mirror it. |
| `pyo3` | **0.28** (keep; latest 0.28.3) | `nono-py` confined-run API surface | `../nono-py` already pins `pyo3 0.28`, `extension-module`, `tokio rt-multi-thread`. Add a `confined_run(...)` that delegates to the same daemon/launcher path; no new binding framework. Bump the internal `nono`/`nono-proxy` dep from the stale `0.57.0` pins to `0.62.x` as part of exposing the new API. |
| `napi` / `napi-derive` / `napi-build` | **2** (keep; 3.8.4 exists) | `nono-ts` confined-run API surface | `../nono-ts` pins `napi 2` (`napi9` feature). A napi-3 migration is a separate, larger effort — do NOT couple it to this milestone. Expose `confinedRun(...)` on the existing napi 2 surface; bump the internal `nono` dep from `0.33.0` to `0.62.x`. |

### Net-New Win32 surface (no new crate — additions to existing `windows-sys 0.59` feature set)

| Win32 API | Module (already enabled) | Purpose | Notes |
|-----------|--------------------------|---------|-------|
| `CreateJobObjectW(attrs, name)` with a **named** job | `Win32_System_JobObjects` | The `AI_AGENT` marker: one named job per tenant (e.g. session-scoped `nono-ai-agent-<id>`) | Already imported (unnamed) in `exec_strategy_windows/mod.rs`. The new bit is passing a name + reopening via `OpenJobObjectW`. Named jobs ARE the recommended user-mode "process marker" — durable, kernel-tracked, enumerable, and they double as the kill-group + resource cap. |
| `AssignProcessToJobObject` | `Win32_System_JobObjects` | Bind each launched engine (and its descendants) into its `AI_AGENT` job | Already used. `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` already wired — gives clean teardown of an escaped/misbehaving agent. |
| `IsProcessInJob` / `QueryInformationJobObject` | `Win32_System_JobObjects` | Identify whether an arbitrary PID is a marked `AI_AGENT`; enumerate a tenant's processes | `IsProcessInJob` already imported (test-gated). This is the user-mode answer to "mark/identify AI_AGENT processes" without `PsSetCreateProcessNotifyRoutine`. |
| `OpenJobObjectW` | `Win32_System_JobObjects` | Daemon re-opens a named job to adopt/inspect/terminate a tenant across calls | NEW import; present in 0.59. |
| `CreateNamedPipeW(... PIPE_UNLIMITED_INSTANCES ...)` | `Win32_System_Pipes` | Multi-client capability pipe (one persistent name, N concurrent tenants) | Already used in `bind_aipc_pipe` (the single-instance control pipe uses `1`; the multi-tenant daemon pipe uses `PIPE_UNLIMITED_INSTANCES`). |
| `ConvertSidToStringSidW` / `ConvertStringSecurityDescriptorToSecurityDescriptorW` | `Win32_Security_Authorization` | Per-tenant SDDL DACL ACE so each agent's Low-IL/AppContainer SID reaches only its own pipe instance | Already used; the per-session-SID + per-package-SID ACE machinery in `socket_windows.rs` is exactly the multi-tenant scoping primitive. |
| `DeriveAppContainerSidFromAppContainerName` / `CreateAppContainerProfile` | `Win32_Security_Isolation` (already enabled) | Per-agent AppContainer identity for WFP network scoping | Already the proven network-scoping identity (memory `windows_appcontainer_wfp_validated`). Derive-only is NOT enough — must register the profile. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cbindgen` 0.29 | C header regen for FFI | Unchanged; only relevant if the daemon control surface is also exposed via C FFI (not required). |
| `maturin` | Build/publish `nono-py` wheels | Existing nono-py toolchain; pyo3 0.28.3 compatible. |
| `@napi-rs/cli` | Build `nono-ts` native addon | Existing napi 2 toolchain. |
| Cross-target clippy (`x86_64-unknown-linux-gnu` + `x86_64-apple-darwin`) | Catch cfg-gated drift | MANDATORY per CLAUDE.md. The daemon is Windows-only (`#[cfg(target_os = "windows")]`) — provide a non-Windows stub `main` exactly like `nono-wfp-service.rs` does so the workspace `cargo check` stays green on Unix. |

## Installation

```toml
# crates/nono-cli/Cargo.toml — the daemon is a new bin (or new crate) reusing the existing service dep.
# No new dependency line if it lives in nono-cli alongside nono-wfp-service.
windows-service = "0.7"   # already present; optionally → "0.8.1"

# crates/nono/Cargo.toml — windows-sys feature additions (same 0.59 crate, add features only if missing):
#   Win32_System_JobObjects, Win32_System_Pipes, Win32_Security_Isolation are ALREADY enabled.
#   No version change.

# ../nono-py/Cargo.toml — bump stale internal pins to expose confined_run:
nono = "0.62"        # was "0.57.0"
nono-proxy = "0.62"  # was "0.57.0"
pyo3 = { version = "0.28", features = ["extension-module"] }   # unchanged

# ../nono-ts/Cargo.toml — bump stale internal pin:
nono = "0.62"        # was "0.33.0"
napi = { version = "2", default-features = false, features = ["napi9"] }   # unchanged
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `windows-sys` 0.59 (keep) | `windows-sys` 0.61.2 (latest) | Only if a future milestone needs an API absent from 0.59. None of the daemon/marker/IPC APIs are. Bumping now adds cross-target-drift risk for zero functional gain. |
| `windows-service` 0.7 (in-tree) | `windows-service` 0.8.1; Microsoft's newer `windows-services` 0.26.1 | 0.8.1 if you want the latest of the same crate (low-risk minor). `windows-services` (MS, different crate) only if starting green-field — not worth diverging from the crate `nono-wfp-service` already ships. |
| Named **job object** as the `AI_AGENT` marker | Token group SID / synthetic restricting SID; a sentinel env var; a named mutex | Job objects win: kernel-tracked, descendant-capturing, enumerable (`IsProcessInJob`), double as kill-group + resource caps, and survive across daemon calls via `OpenJobObjectW`. A SID/marker alone gives identity but not containment or teardown. Use a SID *in addition* (for WFP/pipe DACL scoping), not instead. |
| `tokio` async multi-client pipe server | Generalize the existing sync `PeekNamedPipe` deadline-poll to N threads | Sync-per-thread is fine for a handful of tenants and reuses proven code; tokio scales better for many concurrent agents and matches `nono-wfp-service`. Choose sync if you want minimal new code; tokio if tenant count is unbounded. |
| Extend `SupervisorMessage` JSON frame with a tenant id | New protobuf/bincode protocol | The length-prefixed JSON frame is proven, bounded (64 KiB), replay-guarded, and already serde-derived. A new wire format is unjustified risk. |
| Reuse `nono run` broker arm as the launcher | Post-hoc token IL-drop (spike 002) | Post-hoc demote is **demote-only/unsound** (leaky) — ship it ONLY as a supplementary "demote a misbehaving agent" control, never as the confinement boundary. Launch-time confinement is the sound model. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Any kernel driver / minifilter / `PsSetCreateProcessNotifyRoutine`** | Out of scope by milestone definition AND by ADR-65 (No-go/Conditional-go): WFP+AppContainer/Low-IL already gives kernel-enforced isolation; a production driver is high cert/maintenance cost for incremental gain. User-mode only. | Named job objects (`IsProcessInJob`) for marking/identification; broker-arm launch-time confinement; WFP service for network. |
| **Post-hoc token IL-drop as the primary confinement model** | Spike 002: feasible but leaky/unsound (demote-only). Handles/objects opened before the drop survive. | Launch-time confinement via the broker arm (spike 003, VALIDATED). Layer IL-drop only as a "demote escaped agent" supplementary control. |
| **`windows-sys` version bump (0.59 → 0.61.x) "to be current"** | Gratuitous churn; cross-target-drift hazard (two cfg-gated compile errors already reached release tags — memory `feedback_clippy_cross_target`). No needed API is missing from 0.59. | Stay on the workspace 0.59 pin; add *features*, not versions. |
| **napi 2 → napi 3 migration coupled to this milestone** | napi 3.x is a breaking surface (3.8.4 latest); migrating is its own effort that would balloon scope and risk the binding build. | Expose `confinedRun` on the existing napi 2 surface; defer the napi-3 migration to a dedicated binding-maintenance task. |
| **A brand-new wire protocol for capability IPC** | The existing framed JSON (`SupervisorMessage`) is proven, bounded, replay-guarded, timeout-bounded, and re-accept-capable. | Add a tenant/agent-id field to the existing message types. |
| **Assuming the engine inherits the launcher CWD** | PowerShell did not (relative write resolved to `C:\`, correctly denied) — a recurring contract. | Express ALL grants as absolute paths (banked contract). |
| **Launching the agent workspace from an elevated context** | Elevated-created dirs are `BUILTIN\Administrators`-owned → no `WRITE_DAC` for the label/DACL grant → confined writes fail-secure (R-B3). | Workspace MUST be user-owned (`takeown /F` or create non-elevated). |
| **Deriving an AppContainer SID without `CreateAppContainerProfile`** | Derive-only SID → `CreateProcessW` `ERROR_FILE_NOT_FOUND` (memory `windows_appcontainer_wfp_validated`); `env_clear()` strips `SystemRoot` → CLR `0xFFFF0000` (memory `windows_hook_interpreter_spawn_gotchas`). | Register the profile via `CreateAppContainerProfile`; preserve `SystemRoot`/`windir`/`SystemDrive` baseline env. |

## Stack Patterns by Variant

**If the daemon serves a small, known number of concurrent agents:**
- Generalize the existing sync `socket_windows.rs` accept loop to one thread per tenant (reuse `PeekNamedPipe` deadline-poll + disconnect/re-accept verbatim).
- Because it maximizes reuse of proven, audited code and avoids a tokio accept-loop in the security-critical path.

**If the daemon must scale to many concurrent agents:**
- Use `tokio::net::windows::named_pipe` `ServerOptions` with `PIPE_UNLIMITED_INSTANCES` + one task per connection, mirroring `nono-wfp-service`'s tokio control loop.
- Because per-connection tasks scale without thread-count blowup and reuse the existing async runtime.

**If exposing confined-run through Python (LangChain agent, spike 005 proof):**
- `nono-py` `confined_run(exe, args, allow=[...], profile=...)` delegating to the same broker-arm launch path; prove the abstraction by confining a real Python/LangChain agent with NO Claude hook.
- Because the binding must demonstrate "engine is a variable" — python was the strongest spike-003 proof.

**If marking/identifying an already-running agent (adopt, not launch):**
- Re-open the tenant's named job via `OpenJobObjectW` + `IsProcessInJob`; apply post-hoc IL-drop ONLY as a demote control (never as the boundary).
- Because adoption-after-spawn cannot retroactively achieve launch-time soundness — flag it as best-effort demote.

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `windows-sys 0.59` | `windows-service 0.7`/`0.8.1` | `windows-service` brings its own `windows-sys`; both resolve fine. Confirmed by current in-tree `nono-wfp-service` build. |
| `pyo3 0.28.3` | Rust 1.95 (workspace MSRV), `tokio 1` | nono-py already builds on this combo (edition 2024, rust 1.95). |
| `napi 2 (napi9)` | Rust 1.77+ (nono-ts MSRV), Node 10+ | Existing nono-ts pin; do not bump to napi 3 in this milestone. |
| `nono 0.62.x` | nono-py / nono-ts internal pins (currently stale 0.57.0 / 0.33.0) | Bumping the internal pin is REQUIRED to expose the new confined-run API; pin to `0.62`. |
| `windows-service 0.7` vs `0.8.1` | Both MSRV ≤ 1.95 | 0.8.1 MSRV is 1.71; safe. Align daemon to whichever `nono-wfp-service` uses to keep one version. |

## Integration Points with Existing Backend

- **Launcher primitive:** daemon → existing broker arm (`windows_low_il_broker:true`) → `Sandbox::apply` on a `CapabilitySet`. No new confinement code; honor exe/interpreter coverage gate + absolute grants + user-owned workspace.
- **Multi-tenant IPC:** generalize `crates/nono/src/supervisor/socket_windows.rs` (`bind_low_integrity_with_session_and_package_sid`, `bind_aipc_pipe` with `PIPE_UNLIMITED_INSTANCES`, `disconnect_and_reconnect`, framed JSON). Add a tenant/agent id to `SupervisorMessage`.
- **Service host:** model the daemon binary on `crates/nono-cli/src/bin/nono-wfp-service.rs` (SCM dispatch, Event Log, control pipe, non-Windows stub `main`, MSI registration). MSI service-start was made non-fatal in v2.11 — reuse that posture.
- **Marker:** add named-job-object create/open to `exec_strategy_windows/` (job lifecycle already lives there).
- **Bindings:** add `confined_run`/`confinedRun` to `../nono-py` and `../nono-ts`; bump internal `nono` pins to 0.62.x.
- **Cross-target discipline:** daemon + marker code is `cfg(target_os = "windows")`; provide Unix stubs and run cross-target clippy (CLAUDE.md MUST).

## Sources

- In-tree: `crates/nono/Cargo.toml` (windows-sys 0.59 + feature set), `crates/nono-cli/Cargo.toml` (windows-service 0.7), `crates/nono/src/supervisor/socket_windows.rs` (named-pipe IPC, SDDL, PIPE_UNLIMITED_INSTANCES, re-accept), `crates/nono-cli/src/bin/nono-wfp-service.rs` (existing user-mode service pattern), `crates/nono-cli/src/exec_strategy_windows/mod.rs` (job-object lifecycle), `../nono-py/Cargo.toml` (pyo3 0.28), `../nono-ts/Cargo.toml` (napi 2) — HIGH confidence (authoritative, current code).
- Spike blueprint: `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` (spike 003 VALIDATED; contracts) — HIGH.
- Project memory: `windows_appcontainer_wfp_validated`, `windows_hook_interpreter_spawn_gotchas`, `feedback_clippy_cross_target`, `project_v210_opened` (ADR-65) — HIGH.
- [crates.io windows-service](https://crates.io/crates/windows-service) — latest 0.8.1, MSRV 1.71 — MEDIUM (web).
- [docs.rs pyo3](https://docs.rs/crate/pyo3/latest) — latest 0.28.3 (0.28.0/0.28.1 yanked) — MEDIUM (web).
- [docs.rs napi](https://docs.rs/crate/napi/latest) — latest 3.8.4 (we deliberately stay on 2) — MEDIUM (web).
- [docs.rs windows-sys](https://docs.rs/crate/windows-sys/latest) — latest 0.61.2 (we deliberately stay on 0.59) — MEDIUM (web).
- [Microsoft Learn: Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects) — named-job marker semantics — MEDIUM.

---
*Stack research for: engine-agnostic AI-agent confinement on Windows (nono v2.12)*
*Researched: 2026-06-13*
