---
milestone: v2.12
milestone_name: AI Agent Abstraction
status: active
created: 2026-06-13
last_updated: 2026-06-14
granularity: standard
---

# Roadmap — nono

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 01-12 (shipped 2026-03-31) — see [`milestones/v1.0-*`](milestones/)
- ✅ **v2.0 Windows Gap Closure** — Phases 13-18 — see [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md)
- ✅ **v2.1 Resource Limits / Extended IPC / Attach-Streaming** — see [`milestones/v2.1-ROADMAP.md`](milestones/v2.1-ROADMAP.md)
- ✅ **v2.2 Windows/macOS Parity Sweep** — see [`milestones/v2.2-ROADMAP.md`](milestones/v2.2-ROADMAP.md)
- ✅ **v2.3 Linux POC Unblock + Deferreds Closure** — see [`milestones/v2.3-ROADMAP.md`](milestones/v2.3-ROADMAP.md)
- ✅ **v2.4 Complete the Partial Ports + UPST4** — Phases 35, 36, 36.5, 39, 40 (shipped 2026-05-15) — see [`milestones/v2.4-ROADMAP.md`](milestones/v2.4-ROADMAP.md)
- ✅ **v2.5 Backlog Drain + UPST5** — Phases 37, 41, 42, 43 (shipped 2026-05-20) — see [`milestones/v2.5-ROADMAP.md`](milestones/v2.5-ROADMAP.md)
- ✅ **v2.6 UPST6 + v2.5 Drain** — Phases 44, 44.1, 45, 46, 47, 48, 49, 50 (shipped 2026-05-25) — see [`milestones/v2.6-ROADMAP.md`](milestones/v2.6-ROADMAP.md)
- ✅ **v2.7 Windows supervised-run hardening** — Phases 51, 52 (shipped 2026-05-26) — see [`milestones/v2.7-ROADMAP.md`](milestones/v2.7-ROADMAP.md)
- ✅ **v2.8 UPST7 + v2.7 Drain & Release** — Phases 53-59 (shipped 2026-06-06; tags `v2.8`+`v0.57.5`) — see [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md)
- ✅ **v2.9 Windows Sandbox-the-Tools — Confined Coding Loop** — Phases 60, 61, 62 (published as `v0.62.2` 2026-06-06) — see [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md)
- ✅ **v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity** — Phases 63-66 (shipped 2026-06-11; tag `v2.10`) — see [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md)
- ✅ **v2.11 Clean-Host Distribution Cleanup + UPST8** — Phases 67-70 (shipped 2026-06-13) — see [`milestones/v2.11-ROADMAP.md`](milestones/v2.11-ROADMAP.md)
- 🚧 **v2.12 AI Agent Abstraction** — Phases 71-75 (active; started 2026-06-13)

## Phases

- [x] **Phase 71: Engine-Agnostic Launch Productionization** — A user can confine *any* non-Claude AI agent engine end-to-end via per-engine launch profiles through one engine-neutral `nono run` path, with fail-secure exe/interpreter coverage + R-B3 workspace-ownership diagnostics.
- [x] **Phase 72: nono-py Binding + In-Process-Exec Proof** — A Python/LangChain agent is confined through the `nono-py` binding with NO Claude hook — both `confined_run` (spawn-confined) and `confine` (self-confine at startup) — and the engine-abstraction contract is documented as a stable boundary.
- [x] **Phase 73: AI_AGENT Marker** — Each confined agent carries an unforgeable `AI_AGENT` identity bound to its daemon-minted token SID (named job objects, if used, are kill-group/enumeration only — never authorization).
- [x] **Phase 74: Persistent Multi-Tenant Daemon** — A least-privilege persistent daemon launches/confines multiple concurrent agents over one tenant-isolated capability pipe (server-side `ImpersonateNamedPipeClient` + per-tenant SID), fresh token+job per agent, deterministically reaped, split from the elevated WFP service. *(riskiest — former spike 004)*
- [ ] **Phase 75: Supplementary Controls + Secondary Engines** — An operator can demote a running agent (post-hoc IL-drop, demote-only); outbound egress is WFP-scoped per agent; and the abstraction is proven across ≥2 engines (Copilot CLI) and ≥2 bindings (`nono-ts` parity with `nono-py`).

## Phase Details

### Phase 71: Engine-Agnostic Launch Productionization
**Goal**: nono can parent-and-confine any covered AI agent engine (starting with Aider + a LangChain-Python profile) through one engine-neutral launch path — the validated spike-003 path promoted to a first-class, de-spiked code path that every later phase consumes. This is the foundation; the daemon, binding, and marker all sit on top of it.
**Depends on**: Nothing (first phase of v2.12; builds on the existing `exec_strategy_windows/launch.rs` broker arm + `windows_low_il_broker:true`)
**Requirements**: ENG-01, ENG-02, ENG-03
**Success Criteria** (what must be TRUE):
  1. A user runs a non-Claude engine (Aider) confined end-to-end on a real Win11 host through the engine-neutral path: files written inside the granted absolute workspace land; writes outside it are denied (`NO_WRITE_UP`) — and the engine's in-process and subprocess operations are confined transitively (nono is the parent, not a per-tool hook).
  2. A user can declare a per-engine launch profile (executable + interpreter path(s) like `python.exe`, an ABSOLUTE writable workspace, a network identity) and launch any profiled engine through that single path — engines do NOT inherit the launcher CWD (relative writes must resolve against the declared absolute workspace, per the PowerShell→`C:\` trap).
  3. The launcher fails SECURE with an actionable message when an engine's executable/interpreter path is not covered by the launch policy — never silent partial confinement (core nono coverage-gate invariant).
  4. The launcher fails SECURE with a clear R-B3 diagnostic when the granted workspace is not owned by the session user (admin-owned dir → no `WRITE_DAC` → confined write would fail opaquely) — the diagnostic names the ownership problem, not a generic deny.
  5. Per-engine fit is documented (launch-and-confine vs hook vs Cursor-WSL-only), and the job-assignment path is hardened against nested-job collisions: spawn suspended, assign to the agent job BEFORE any code runs, fail-secure (terminate) on assign failure, no UI limits on the job.
**Plans**: 5 plans
- [x] 71-01-PLAN.md — windows_interpreters profile field + aider/langchain-python engine profiles (ENG-03)
- [x] 71-02-PLAN.md — SC5 named foreign-job (GLE-5) diagnostic + fail-secure assign negative test (P6)
- [x] 71-03-PLAN.md — library fail-secure primitives: interpreter coverage gate + path_has_write_owner helper (ENG-02)
- [x] 71-04-PLAN.md — CLI integration: --workspace flag, child-CWD/grant, interpreter resolution, R-B3 pre-launch gate (ENG-01, ENG-02)
- [x] 71-05-PLAN.md — 71-HUMAN-UAT.md + SC1 real-Win11 Aider end-to-end gate (ENG-01)

### Phase 72: nono-py Binding + In-Process-Exec Proof
**Goal**: The engine abstraction is proven in code — a real Python/LangChain agent is confined through the `nono-py` binding with NO Claude hook, exercising both the external-spawn shape and the in-process-self-confine shape — and the abstraction-boundary contract (E1-E5) is written down as a stable boundary other engines implement against. Depends only on Phase 71 launch semantics; independent of the daemon (parallel-capable with Phase 73).
**Depends on**: Phase 71 (consumes the productionized launch semantics)
**Requirements**: ABI-01, ABI-02
**Success Criteria** (what must be TRUE):
  1. A Python/LangChain agent is confined via `confined_run(exe, args, allow, profile)` (Shape A — spawn a confined child, identical to spike 003) with NO Claude hook in the loop, and writes outside the granted workspace are denied.
  2. The same agent can self-confine via `confine(profile, allow)` (Shape B — BORN CONFINED at its own entrypoint via a broker re-exec before any privileged handle is opened): a LangChain `PythonREPLTool` `exec()` write outside the granted workspace is DENIED (Low-IL / Job / AppContainer enforced), while a write inside is ALLOWED. [Reworded 2026-06-14 per D-05: Windows `Sandbox::apply` is preview-only; Shape B is realized as a born-confined self-re-exec via nono.exe broker.]
  3. Shape B's soundness boundary is enforced and documented: `confine()` invokes nono.exe as the FIRST operation at process startup (before any privileged handle is opened — ORDERING IS THE INVARIANT), producing a Low-IL born-confined child; the binding docs state the agent must call `confine()` before any other operation. [Reworded 2026-06-14 per D-05: Windows-equivalent of the soundness invariant.]
  4. The internal `nono` pin in `nono-py` is bumped from `0.57.0` to `0.62.x` (pyo3 0.28 kept; no napi/pyo3 major migration) and the binding builds + tests green.
  5. The engine-abstraction contract (E1: executable/interpreter path; E2: an ownable launch command; E3: an absolute workspace grant; E4: a network identity; E5: an optional pre-exec interception point) is documented as a stable boundary that other engines implement against.
**Plans**: 4 plans
- [x] 72-01-PLAN.md — Shape B soundness spike (real Win11 host) + ROADMAP SC2/SC3 reword (ABI-01)
- [x] 72-02-PLAN.md — nono pin bump + windows_confined_run.rs (confined_run + confine) + lib.rs registration + __init__.py exports (ABI-01)
- [x] 72-03-PLAN.md — proj/DESIGN-engine-abstraction.md E1-E5 contract + zt-infra E5 mapping + ../nono-py/docs link (ABI-02)
- [x] 72-04-PLAN.md — examples/15_langchain_confined.py + tests/test_confined_run.py + Win11 UAT gate (ABI-01)

### Phase 73: AI_AGENT Marker
**Goal**: Every confined agent carries an unforgeable `AI_AGENT` identity that a non-agent process cannot claim and a confined agent cannot shed — the authorization signal the multi-tenant daemon will key on. Depends only on Phase 71; independent of Phase 72 (parallel-capable). The daemon prerequisite.
**Depends on**: Phase 71 (the marker is established at the productionized launch's spawn-time)
**Requirements**: MARK-01
**Success Criteria** (what must be TRUE):
  1. A launched agent is marked with an unforgeable identity bound to its spawn-time token SID (minted by the launcher/daemon), and this SID — NOT a job name, env var, or argv — is what authorization keys on.
  2. A non-agent process (one the launcher did not spawn) cannot acquire the `AI_AGENT` identity: it cannot mint or impersonate the token SID, and opening the named job by name does not confer the identity.
  3. A confined agent cannot shed the marker — job breakaway is denied (`JOB_OBJECT_LIMIT_BREAKAWAY_OK` NOT set / breakaway refused) and the job object is ACL'd daemon-only.
  4. Given an arbitrary PID, the system correctly classifies it as `AI_AGENT` or not (`IsProcessInJob` / `QueryInformationJobObject` for enumeration + the token-SID check for authorization); the named job is used for kill-group / descendant capture / resource caps only.
  5. Adopted (not-launched) agents are explicitly handled as best-effort / demote-only (the marker is sound only for launcher-spawned agents — adoption is documented as a weaker guarantee).
**Plans**: 3 plans
- [x] 73-01-PLAN.md — nono crate: AgentRegistry, AgentClassification, read_process_appcontainer_sid + non-Windows stubs + lib.rs re-export (MARK-01 SC2/SC4 unit paths)
- [x] 73-02-PLAN.md — Job object SDDL hardening: create_process_containment signature refactor + explicit DACL + negative tests job_never_has_breakaway_ok + job_security_descriptor_denies_low_il (MARK-01 SC3)
- [x] 73-03-PLAN.md — CLI verb nono classify + mint→registry wiring in execution_runtime.rs + SC4 in-process integration tests + SC5 adopted-agent doc (MARK-01 SC1/SC4/SC5)

### Phase 74: Persistent Multi-Tenant Daemon
**Goal**: A persistent, least-privilege local daemon launches and confines multiple concurrent agents over one tenant-isolated capability pipe, with correct per-agent token/job lifetime in a process that lives for days — the marquee (and riskiest) v2.12 capability. This is launch-and-confine (Phase 71) + marker (Phase 73) + a multi-client pipe + the genuinely unspiked token/job-reuse risk, isolated inside this phase. It MUST NOT begin until Phase 71 is a gated, working single-launch code path and Phase 73 exists — that ordering IS the quality gate.
**Depends on**: Phase 71 (working single-launch path — hard gate) AND Phase 73 (marker)
**Requirements**: DMON-01, DMON-02, DMON-03
**Success Criteria** (what must be TRUE):
  1. The daemon launches and confines multiple concurrent agents, each with a FRESH confining token + a FRESH job object (no reuse across tenants — reuse would collapse isolation: B inheriting A's restricting SID / workspace relabel / WFP scope); two concurrent confined agents are each served independently over one persistent pipe, each scoped to its own SID.
  2. The capability pipe ISOLATES tenants: the daemon authenticates each client server-side (`ImpersonateNamedPipeClient` + `GetTokenInformation` + per-tenant SID match / per-tenant SDDL pipe instances), treats any `agent_id` in the wire frame as an untrusted routing hint only, and a cross-tenant request (tenant B asking for tenant A's grants) is DENIED — proven with a negative test.
  3. Agents are deterministically reaped on exit (wait on job completion / process handle; every per-agent resource tied to one owning struct with a `Drop` that closes all handles): running N agents over time returns to baseline handle/job count — proven with a 100-agent launch/exit handle-count-returns-to-baseline test (no leak).
  4. The daemon runs at LEAST privilege (USER, not LocalSystem) and is SPLIT from the elevated `nono-wfp-service`, so an escaped agent cannot pivot to SYSTEM or to other tenants; the pipe is query-only and never expands a running agent's capabilities (no escape hatch). The privilege model is recorded as an ADR written BEFORE the service host is coded.
  5. The daemon is modeled on the proven `nono-wfp-service.rs` shape (SCM dispatch, Event Log, control pipe, non-Windows stub, MSI registration, non-fatal start) and reuses the framed JSON `SupervisorMessage` wire protocol (extended with a tenant id ONLY if `session_id` proves insufficient — no net-new wire protocol).
**Research flag**: Plan this phase with `--research-phase 74`. Two mechanisms are unspiked / net-new: (a) token/job REUSE-vs-fresh across many tenants (the explicitly unspiked part of spike 003/004 — the milestone's highest-risk unknown; scope a spike INSIDE the phase gated on fresh-token isolation + deterministic reap + cross-tenant denial); (b) whether server-side `ImpersonateNamedPipeClient` (NOT currently in `socket_windows.rs` — it verifies the *server* PID from the client side today) composes with the existing Low-IL/AppContainer cap-pipe SDDL DACL handshake. Also re-assert: AppContainer per-agent SID needs `CreateAppContainerProfile` (not derive-only, else `CreateProcessW` `ERROR_FILE_NOT_FOUND`); preserve `SystemRoot`/`windir`/`SystemDrive` env baseline (else CLR `0xFFFF0000`).
**Plans**: 8 plans (74-07 + 74-08 gap-closure added during execution)
Plans:
**Wave 1**
- [x] 74-01-PLAN.md — ADR (privilege model) + Wave 0 spike harness: fresh-token isolation + handle baseline + cross-tenant denial (DMON-01/02/03)
- [x] 74-02-PLAN.md — nono lib primitives: AgentRegistry::remove + authenticate_pipe_client + ImpersonationGuard (DMON-01/02)
- [x] 74-03-PLAN.md — nono-agentd binary skeleton: second [[bin]] + non-Windows stub + SCM dispatch + DaemonState/AgentTenant RAII (DMON-01/03)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 74-04-PLAN.md — Daemon accept loop (per-tenant SDDL + impersonation auth) + launch orchestration (fresh token+job + reap task) (DMON-01/02/03)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 74-05-PLAN.md — CLI verbs: nono daemon start|stop|status|install|uninstall + nono agent launch|list (DMON-01/03)

**Wave 5 (gap closure)** *(SC1 end-to-end control plane + operator UX polish)*
- [x] 74-07-PLAN.md — daemon control-pipe server (launch/list → launch_agent + tenant table) + dev-layout daemon start/status (DMON-01/02/03)
- [x] 74-08-PLAN.md — operator UX polish: daemon-start clean detach + bare-exe SearchPathW resolution + engine-profile in list (DMON-01/03)

**Wave 4** *(blocked on Wave 5 — UAT validates the now-wired SC1 end-to-end)*
- [x] 74-06-PLAN.md — 74-HUMAN-UAT.md + Win11 UAT gate: SC1-SC5 go/no-go (DMON-01/02/03) — UAT PASS 2026-06-15

### Phase 75: Supplementary Controls + Secondary Engines
**Goal**: Round out the milestone with the supplementary (never-the-boundary) controls and the second-engine/second-binding parity that proves the abstraction generalizes — all low-cost adds once the launch-time default (Phases 71-74) is proven. Demote must FOLLOW a proven launch-time default; it is an incident-response lever, not a confinement model.
**Depends on**: Phase 74 (demote is a daemon verb; WFP-per-agent keys on the daemon's per-tenant identity) — and Phase 72 (nono-ts parity mirrors the nono-py binding shape)
**Requirements**: SUPP-01, SUPP-02, SUPP-03
**Success Criteria** (what must be TRUE):
  1. An operator can demote a running/misbehaving agent on the fly (post-hoc token IL-drop) as a daemon "demote tenant" verb, with the leak/soundness limits documented (spike-002 finding: this is demote-only, explicitly NOT a standalone confinement boundary — never the "detect-and-confine-as-primary" anti-feature).
  2. Outbound network egress is scoped PER confined agent — WFP filtering keyed to each agent's identity (E4 SID per tenant) via the existing elevated `nono-wfp-service` — so each agent's network policy is enforced independently (one agent's allowed domain does not leak to another).
  3. GitHub Copilot CLI ships as a second non-Claude engine profile (a second `node.exe` engine), confined through the same engine-neutral launch path proven in Phase 71.
  4. The `nono-ts` (Node) binding reaches parity with `nono-py`: both `confinedRun` (spawn-confined) and `confine` (self-confine) exist, with the internal `nono` pin bumped `0.33.0` → `0.62.x` (napi 2 kept — no napi 3 migration).
  5. The abstraction is demonstrably proven across ≥2 engines (Aider + Copilot CLI) and ≥2 bindings (`nono-py` + `nono-ts`) — closing the "engine is a variable" claim in code.
**Plans**: 5 plans
Plans:
**Wave 1**
- [x] 75-01-PLAN.md — SUPP-02: per-agent WFP filter add at launch / remove at reap + D-05 fail-secure gate (SUPP-02)
- [x] 75-03-PLAN.md — SUPP-03a: copilot-cli engine profile in policy.json (native PE, no windows_interpreters) (SUPP-03)
- [x] 75-04-PLAN.md — SUPP-03b: nono-ts confinedRun/confine parity + nono pin bump 0.33.0 → 0.62 + cross-target clippy (SUPP-03)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 75-02-PLAN.md — SUPP-01: ControlRequest::Demote + handle_demote (IL-drop + WFP-cut) + agent_demote CLI verb (SUPP-01)

**Wave 3** *(blocked on Wave 2 completion)*
- [ ] 75-05-PLAN.md — Live Win11 UAT: SC1-SC5 gates + A1/A2/A4 assumption confirmations (SUPP-01, SUPP-02, SUPP-03)

<details>
<summary>✅ v2.11 Clean-Host Distribution Cleanup + UPST8 (Phases 67-70) — SHIPPED 2026-06-13</summary>

- [ ] Phase 67: Clean-Host Windows Install (DIST-01/02, TRUST-01/02) — host-gated UAT pending (clean Win11 host)
- [x] Phase 68: macOS Resource-Limit Enforcement Fix (2/2) — completed 2026-06-12
- [x] Phase 69: UPST8 Audit (1/1) — completed 2026-06-13
- [x] Phase 70: UPST8 Cherry-pick Sync (3/3) — completed 2026-06-13

Phases 68/69/70 complete; Phase 67 carries forward host-gated. Full detail: [`milestones/v2.11-ROADMAP.md`](milestones/v2.11-ROADMAP.md).

</details>

<details>
<summary>✅ v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity (Phases 63-66) — SHIPPED 2026-06-11</summary>

- [x] Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit (3/3) — completed 2026-06-08
- [x] Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave (5/5) — completed 2026-06-09
- [x] Phase 65: Minifilter ADR + macOS Live Re-validation (4/4) — completed 2026-06-11
- [x] Phase 66: WR-02 EDR HUMAN-UAT (1/1) — completed 2026-06-11

9/9 reqs satisfied. Full detail: [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md).

</details>

<details>
<summary>✅ v2.8 / v2.9 (Phases 53-62) — SHIPPED 2026-06-06</summary>

v2.8 UPST7 + v2.7 Drain & Release (Phases 53-59, tags `v2.8`+`v0.57.5`); v2.9 Windows Sandbox-the-Tools (Phases 60-62, published `v0.62.2`). Full detail: [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md) / [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md).

</details>

## Progress

v2.12 active (Phases 71-75). Build order is dependency-driven: **71 (foundation) FIRST**, then **72 ∥ 73** (parallel — both depend only on 71), then **74** (the riskiest daemon; hard-gated behind a working single-launch path 71 + marker 73), then **75** (supplementary controls + secondary engines). The daemon (74) MUST NOT precede a solid single-launch path.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 71. Engine-Agnostic Launch Productionization | 5/5 | Complete   | 2026-06-14 |
| 72. nono-py Binding + In-Process-Exec Proof | 4/4 | Complete    | 2026-06-14 |
| 73. AI_AGENT Marker | 3/3 | Complete   | 2026-06-14 |
| 74. Persistent Multi-Tenant Daemon | 8/8 | Complete   | 2026-06-15 |
| 75. Supplementary Controls + Secondary Engines | 4/5 | In Progress|  |

## Dependency Graph

```
                  ┌──────────────────────────────┐
                  │ 71 Engine-Agnostic Launch      │  (foundation — everything sits on top)
                  │    (ENG-01/02/03)              │
                  └───────────┬──────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
  ┌────────────────────────┐    ┌────────────────────────┐
  │ 72 nono-py Binding      │    │ 73 AI_AGENT Marker      │   (72 ∥ 73 — parallel)
  │    (ABI-01/02)          │    │    (MARK-01)            │
  └───────────┬────────────┘    └───────────┬────────────┘
              │                              │
              │            ┌─────────────────┘
              │            ▼
              │   ┌────────────────────────────────┐
              │   │ 74 Multi-Tenant Daemon          │  (RISKIEST — gated behind 71 working + 73)
              │   │    (DMON-01/02/03)              │  research-flag 74
              │   └───────────┬────────────────────┘
              │               │
              └───────┬───────┘
                      ▼
          ┌────────────────────────────────┐
          │ 75 Supplementary + 2nd Engines  │  (demote, per-agent WFP, Copilot, nono-ts)
          │    (SUPP-01/02/03)              │
          └────────────────────────────────┘
```

## Pitfall → Phase Ownership

Each load-bearing pitfall is owned by exactly one phase as a success-criterion-with-negative-test (per research PITFALLS.md):

| Pitfall | Owner phase | How it's baked in |
|---------|-------------|-------------------|
| Nested-job collisions / silent confinement loss (P6) | 71 | SC5 — spawn suspended, assign before any code runs, fail-secure on assign failure, no UI limits |
| R-B3 user-owned workspace / exe-coverage fail-secure (carry-forward) | 71 | SC3 + SC4 — fail-secure coverage gate + R-B3 ownership diagnostic |
| In-process-exec() cannot be confined post-hoc (P3) | 72 | SC2 + SC3 — `confine()` born-confined broker re-exec at process startup before any privileged handle (Windows-equivalent; ordering is the invariant) |
| AI_AGENT marker forge/shed (P2) | 73 | SC1-SC4 — unforgeable token SID (not a named job), deny breakaway, daemon-only job ACL |
| Cross-tenant capability theft (P1, load-bearing) | 74 | SC2 — server-side `ImpersonateNamedPipeClient` + per-tenant SID; cross-tenant-denial negative test |
| Daemon attack surface / privilege (P4) | 74 | SC4 — least-privilege USER daemon split from elevated WFP service; privilege-model ADR first |
| Token & job-object handle lifetime (P5) | 74 | SC1 + SC3 — fresh token+job per agent, deterministic reap, 100-agent baseline test |
| "Detect-and-confine as primary model" anti-feature | 75 | SC1 — demote stays an IR lever, never the boundary |

## Next

**v2.12 is active.** Build in dependency order:
- `/gsd:plan-phase 71` — engine-agnostic launch productionization (the foundation; standard patterns, spike-003 VALIDATED — skip research-phase). Needs a real Win11 host for the Aider end-to-end gate.
- `/gsd:plan-phase 72` and `/gsd:plan-phase 73` — parallel-safe once 71 lands (binding proof ∥ marker; both standard-pattern, skip research-phase).
- `/gsd:plan-phase 74 --research-phase 74` — the riskiest daemon; hard-gated behind a working 71 + 73. Research the token/job reuse-vs-fresh mechanism and the `ImpersonateNamedPipeClient`-vs-cap-pipe-DACL composition BEFORE coding.
- `/gsd:plan-phase 75` — supplementary controls + Copilot profile + nono-ts parity (proven shapes; skip research-phase).

**Constraints honored this milestone:** user-mode only (no kernel driver — ADR-65 No-go); isolation ≥ the per-invocation `nono run` model (`NO_WRITE_UP`, deny network unless granted); composition over existing subsystems (broker-arm launch, `socket_windows.rs` capability pipe, `nono-wfp-service` shape) — no new framework adoption, no new wire protocol, windows-sys stays 0.59 / pyo3 0.28 / napi 2.

**Repo MUST stay PUBLIC** until Microsoft approves the minifilter altitude (verify no `build_notes/`/`.gsd/` staged before any push).

## References

- `.planning/PROJECT.md` — project context + current state (v2.12 milestone scope).
- `.planning/REQUIREMENTS.md` — v2.12 requirements (ENG-01..03, ABI-01..02, MARK-01, DMON-01..03, SUPP-01..03) + traceability.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.11).
- `.planning/research/SUMMARY.md` — HIGH-confidence research; the A-E build-order recommendation this roadmap implements (Phases 71-75).
- `.planning/research/PITFALLS.md` — pitfall→phase ownership; the load-bearing daemon security properties.
- `.planning/research/ARCHITECTURE.md` — Shape A/B in-process-exec data flow; the 4 new components over 3 existing subsystems.
- `.planning/research/STACK.md` — deliberate non-bumps (windows-sys 0.59, napi 2); net-new Win32 (named job objects).
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for any cfg-gated Unix code touched.
- `.planning/milestones/v2.11-ROADMAP.md` — archived v2.11 (Phases 67-70).
- `proj/DESIGN-engine-abstraction.md` — E1-E5 engine-abstraction contract (authored in Phase 72, plan 72-03).
