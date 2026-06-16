# Phase 74: Persistent Multi-Tenant Daemon - Context

**Gathered:** 2026-06-14
**Status:** Ready for planning

<domain>
## Phase Boundary

A persistent, **least-privilege USER** local daemon that **launches and confines multiple concurrent AI agents** over one tenant-isolated capability pipe, with correct per-agent token/job lifetime in a process that lives for days. This is launch-and-confine (Phase 71) + the AI_AGENT marker (Phase 73) + a multi-client pipe + the genuinely **unspiked** token/job reuse-vs-fresh risk, isolated inside this phase. The marquee and **riskiest** v2.12 capability. Requirements: **DMON-01, DMON-02, DMON-03**.

**Hard gate:** MUST NOT begin until Phase 71 is a gated, working single-launch code path AND Phase 73 (marker) exists — that ordering IS the quality gate.

**In scope:**
- A persistent daemon (least-priv USER, **split** from the elevated `nono-wfp-service`) that launches/confines multiple concurrent agents, each with a **fresh** confining token + **fresh** job object (no reuse across tenants).
- One **multi-client capability pipe** that authenticates each client **server-side** (`ImpersonateNamedPipeClient` + per-tenant SID) and **denies cross-tenant** requests; `agent_id` in the wire frame is an untrusted routing hint only. Pipe is **query-only** — never expands a running agent's capabilities (no escape hatch).
- **Deterministic reap**: every per-agent resource owned by one struct with a `Drop` that closes all handles; N-agents-over-time returns to baseline handle/job count.
- **Privilege-model ADR written BEFORE the service host is coded** (SC4).
- An **in-phase spike** gated on fresh-token isolation + deterministic reap + cross-tenant denial (the explicitly unspiked token/job-reuse risk — research-flag 74).

**Out of scope (own phases):**
- Post-hoc **demote** (spike-002, demote-only), **per-agent WFP egress** scoping, **Copilot CLI** engine, **`nono-ts`** parity — all Phase 75.
- **Adopting externally-spawned agents** into the daemon — stays Phase 73's best-effort/demote-only note; not a 74 path (D-02).
- A net-new wire protocol — reuse framed-JSON `SupervisorMessage`, extend with a tenant id ONLY if `session_id` proves insufficient (research-determined).

</domain>

<decisions>
## Implementation Decisions

### Daemon process model (DMON-03, SC4, SC5)
- **D-01:** Ship the daemon as a **per-user Windows service** (SCM-registered via the `windows-service` crate, modeled on the proven `nono-wfp-service.rs` shape — `service_dispatcher` / `define_windows_service!`, Event Log, control pipe, non-Windows stub, MSI registration, **non-fatal start**) **AND** a **foreground/on-demand fallback** mode for dev/testing and hosts where the service isn't installed. The service runs as the **least-privilege USER in the user's session — NOT LocalSystem/SYSTEM** (the key divergence from `nono-wfp-service`, which is elevated). The USER daemon stays fully **split** from the elevated WFP service so an escaped agent cannot pivot to SYSTEM or to other tenants.
  - The privilege model (USER not LocalSystem; split-from-WFP-service; query-only pipe) is recorded as an **ADR written BEFORE the service host is coded** (SC4 — load-bearing ordering).

### Launch vs adopt entry path (DMON-01, SC1)
- **D-02:** The daemon's **sole** path to confinement is **daemon-launches**: a client asks the daemon to spawn the engine, and the daemon owns the **fresh token + fresh job from birth**, so confinement and the AI_AGENT marker are guaranteed at spawn. **Adopting an externally-spawned agent is OUT of scope for Phase 74** — it remains Phase 73's documented best-effort/demote-only structural classification. (Soundest path; no weaker-guarantee adopt surface to build or test in the riskiest phase.)

### Daemon-death → agent fate (lifecycle, SC3)
- **D-03:** **Agents die with the daemon (fail-secure).** The daemon **holds the per-agent job handle**; `KILL_ON_JOB_CLOSE` means daemon exit (stop/crash/logoff) **terminates every confined agent**. No orphaned, unmanaged-but-still-confined agents survive a daemon restart. Accepted cost: an operator loses all running agents on a daemon restart/crash and must re-launch. (Simplest + safest lifetime story; reinforces the deterministic-reap invariant — job-handle lifetime is bound to the owning daemon struct's `Drop`.)
  - Consequence for D-01's foreground fallback: closing the foreground daemon terminal also kills its agents — consistent and expected.

### Network scope relationship (DMON-03 boundary)
- **D-04:** **Profile-only; NO daemon→WFP coupling in Phase 74.** A Phase 74 launched agent gets whatever network policy its **engine profile** declares (e.g. `network.block:true/false`); the USER daemon does **NOT** talk to the elevated `nono-wfp-service` (clean DMON-03 split). **Per-agent WFP egress keying is Phase 75 (SUPP-02)** and is NOT pulled forward. SC1's "confined" for this phase = filesystem `NO_WRITE_UP` + the profile's existing network posture.

### Operator CLI verb surface (SC5 / UX)
- **D-05:** Add a **minimal** operator surface — Phase 71 deferred the `nono agent`/`nono launch` namespace to "the daemon may introduce verbs later"; 74 is later:
  - **Daemon lifecycle verbs**: `nono daemon start|stop|status`.
  - **Agent launch/list verbs**: `nono agent launch --profile <engine> -- <cmd>` (route a launch through the daemon) and `nono agent list` (enumerate live tenants).
  - **Reuse `nono classify <pid>`** (Phase 73) for inspection — do NOT add a new inspection verb where the existing one suffices.
  - **NO tenant-scoped CLI query verb** this phase (rejected — keeps surface minimal; isolation is proven at the protocol layer per D-06).

### Isolation proof (DMON-02, SC2)
- **D-06:** The **SC2 cross-tenant-denial negative test drives the capability pipe directly** (in-process / integration test impersonating two tenants) — no operator CLI query verb required. The query path exists at the protocol layer regardless; the test exercises tenant B being **denied** tenant A's grants programmatically. Keeps the verb surface minimal (D-05) while proving isolation at the boundary that matters.

### Claude's Discretion
- The **in-phase spike** structure (how much to spike vs. build directly; harness shape) gated on fresh-token isolation + deterministic reap + cross-tenant denial — planning/research's call within the research-flag scope.
- Whether the wire frame needs a **new tenant id field** or `session_id` suffices as the tenant key — **research-determined** (SC5: extend ONLY if `session_id` proves insufficient).
- Exact owning-struct / `Drop` shape for per-agent resources, the per-tenant SDDL pipe-instance vs single-pipe-with-impersonation mechanism, Event Log IDs, and `nono daemon`/`nono agent` output formats — Claude's discretion within the decisions above. Keep fail-secure throughout (any auth/coverage error → deny).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Research (HIGH-confidence; the unspiked mechanisms this phase de-risks)
- `.planning/research/SUMMARY.md` — milestone research synthesis (Shape A/B data flow; load-bearing security properties; deliberate non-bumps).
- `.planning/research/PITFALLS.md` — P1 cross-tenant capability theft (load-bearing, owned by 74/SC2), P4 daemon attack surface/privilege (74/SC4), P5 token & job-object handle lifetime (74/SC1+SC3).
- `.planning/research/ARCHITECTURE.md` — daemon architecture; the supervisor cap-pipe generalization.
- `.planning/research/STACK.md` — `windows-service` crate, `windows-sys` 0.59 pin (deliberate non-bump).

### Spike findings (the validated model this daemon generalizes)
- `.claude/skills/spike-findings-nono/SKILL.md` — spike-findings index (auto-loaded for daemon/multi-engine/token-labeling work).
- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` — SEED-004 / spike-003 (VALIDATED): daemon-as-launcher is the sound primary model; **"Not yet spiked"** explicitly names persistent token/job *reuse* across many agents + the multi-tenant marker + one persistent multi-client capability pipe (was spike 004) — exactly this phase. `socket_windows.rs` is the generalization starting point.
- `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` — spawn-time is the sound mode; post-hoc IL-drop is demote-only/leaky (basis for keeping adopt out of scope, D-02).
- `.planning/spikes/003-daemon-as-launcher/` — original spike source (incl. the untracked `daemon_grant/` working dir).

### Milestone / requirements
- `.planning/ROADMAP.md` §"Phase 74: Persistent Multi-Tenant Daemon" — SC1–SC5; §"Pitfall → Phase Ownership" (P1/P4/P5 owned here); the **research flag** (token/job reuse-vs-fresh + `ImpersonateNamedPipeClient`-vs-cap-pipe-DACL composition).
- `.planning/REQUIREMENTS.md` — **DMON-01, DMON-02, DMON-03** + traceability.
- `.planning/PROJECT.md` — v2.12 milestone scope; "user-mode only, no kernel driver"; isolation ≥ per-invocation `nono run`.

### Carried-forward phase context
- `.planning/phases/73-ai-agent-marker/73-CONTEXT.md` — the unforgeable AI_AGENT token-SID marker + in-memory `AgentRegistry` this daemon makes persistent/multi-tenant; the package-SID authorization predicate (D-01/D-02 there); the named-job = enumeration-only-never-authz invariant; job ACL (D-03 there) the daemon becomes owner of.
- `.planning/phases/71-engine-agnostic-launch-productionization/71-CONTEXT.md` — the productionized Broker-arm single-launch path (hard gate); engine profiles in `policy.json`; absolute-grant + exe/interpreter coverage contracts; R-B3 user-owned workspace; SC5 nested-job-collision hardening; AppContainer/CLR re-assertions.
- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-CONTEXT.md` — E1–E5 contract; E4 network identity = AppContainer package SID.
- `proj/DESIGN-engine-abstraction.md` — E1–E5 abstraction boundary.
- `proj/DESIGN-supervisor.md` — process model, execution strategies, supervisor IPC (the framed `SupervisorMessage` this phase reuses).

### Existing code (implementation targets)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — the **proven service shape** to model the daemon on (`service_dispatcher`, `define_windows_service!`, Event Log via `RegisterEventSourceW`/`ReportEventW`, `--service-mode` arg, non-Windows stub, non-fatal start). NOTE: it runs as SYSTEM/elevated; the daemon must run as least-priv USER (D-01).
- `crates/nono/src/supervisor/socket_windows.rs` — cap-pipe (`CreateNamedPipeW`, `PIPE_UNLIMITED_INSTANCES`), `CAPABILITY_PIPE_SDDL`, `bind_low_integrity_with_session_and_package_sid`, framing. **`ImpersonateNamedPipeClient` is NOT present today** (the server verifies the *server* PID from the client side via `GetNamedPipeClientProcessId`/broker helpers) — this phase ADDS server-side client impersonation (research: confirm it composes with the Low-IL/AppContainer cap-pipe SDDL DACL handshake).
- `crates/nono/src/supervisor/types.rs` — `SupervisorMessage` enum + `session_id` (the candidate tenant key; ~line 464/472).
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — the fresh-token + fresh-job per-agent spawn the daemon orchestrates; `CreateJobObjectW` + the Phase 73 job ACL (daemon becomes the owning principal).
- `crates/nono/src/sandbox/windows.rs` — `AgentRegistry` + marker primitives (Phase 73) the daemon makes its multi-tenant state.

### Cross-target discipline
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for any cfg-gated Unix code touched (the daemon needs a non-Windows stub, like `nono-wfp-service`).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`nono-wfp-service.rs`** — a complete, shipped Windows-service skeleton (SCM dispatch, Event Log, control pipe, non-Windows stub, MSI registration, non-fatal start). The daemon copies this shape; the delta is **USER not SYSTEM** + the multi-tenant pipe + per-agent launch/reap.
- **`socket_windows.rs` cap pipe** — `PIPE_UNLIMITED_INSTANCES` already supports a multi-client pipe; SDDL construction + Low-IL/package-SID binds already exist. Add server-side `ImpersonateNamedPipeClient` + per-tenant SID match on top.
- **Phase 73 `AgentRegistry` + marker** — the per-run in-memory registry becomes the daemon's persistent multi-tenant state; the package-SID authorization predicate is the per-tenant key.
- **Phase 71 Broker-arm single-launch path** — the daemon orchestrates N of these; fresh token + fresh job per agent already mints a per-run AppContainer package SID (the E4 identity / tenant key).

### Established Patterns
- **Library-vs-CLI boundary** — daemon *mechanism* (pipe server, impersonation, per-tenant auth, reap) in the `nono` crate / service binary; the `nono daemon`/`nono agent` verbs + UX in `nono-cli`.
- **Fail-secure default** — any auth/coverage/impersonation error → deny; cross-tenant request → deny; `agent_id` wire field is an untrusted routing hint, never trusted for authz.
- **Deterministic reap via `Drop`** — every per-agent handle (token, job, pipe instance) owned by one struct whose `Drop` closes all of them; baseline-handle-count test enforces no leak.
- **cfg-gated Unix stub** — the daemon binary needs a non-Windows stub (mirror `nono-wfp-service.rs`) + cross-target clippy.

### Integration Points
- Client (`nono agent launch`) → daemon control/capability pipe → daemon spawns via the Phase 71 launch path → registers the package SID in the (now-daemon) `AgentRegistry`.
- Daemon pipe server → `ImpersonateNamedPipeClient` + `GetTokenInformation` → per-tenant SID match against the registry → serve or deny.
- Per-agent owning struct holds token + job + pipe-instance handles; `Drop` reaps on agent exit (wait on job/process handle).
- MSI registration adds the daemon as a per-user service alongside (but split from) `nono-wfp-service`.

</code_context>

<specifics>
## Specific Ideas

- SC1/SC3 acceptance are **real Win11 host** gates: 2 concurrent confined agents each served independently over one pipe (each scoped to its own SID); a **100-agent launch/exit handle-count-returns-to-baseline** test (no leak); a **cross-tenant-denial negative test** (tenant B denied tenant A's grants).
- The Broker arm only works from a real host (or dev-layout/signed `nono.exe` per the R-B4 trust gate).
- Re-assert at implementation time (carried from 71/73): AppContainer per-agent SID needs `CreateAppContainerProfile` (derive-only → `CreateProcessW ERROR_FILE_NOT_FOUND`); preserve `SystemRoot`/`windir`/`SystemDrive` env baseline (else CLR `0xFFFF0000`).
- **Write the privilege-model ADR FIRST** — SC4 makes the ordering load-bearing (ADR before the service host is coded).

</specifics>

<deferred>
## Deferred Ideas

- **Adopting externally-spawned agents** into the daemon — out of scope (D-02); stays Phase 73's best-effort/demote-only structural classification. Revisit only if a sound adopt path is ever needed.
- **Per-agent WFP egress scoping** (daemon→WFP-service coordination) — Phase 75 / SUPP-02 (D-04 keeps the DMON-03 split clean in 74).
- **Post-hoc demote** (spike-002 IL-drop, demote-only) — Phase 75 / SUPP-01.
- **Tenant-scoped `nono agent query` CLI verb** — rejected this phase (D-05/D-06); isolation proven at the protocol layer instead. Revisit if an operator-facing capability-inspection surface is later wanted.
- **Agents surviving a daemon restart** (decouple job lifetime + re-attach) — rejected (D-03 chose fail-secure kill-with-daemon). Revisit only if days-long-session UX demands it and an orphan-management design is justified.

### Reviewed Todos (not folded)
None — no pending todos matched Phase 74 scope.

</deferred>

---

*Phase: 74-persistent-multi-tenant-daemon*
*Context gathered: 2026-06-14*
