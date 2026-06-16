# Phase 73: AI_AGENT Marker - Context

**Gathered:** 2026-06-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Give every **launcher-spawned** confined agent an **unforgeable `AI_AGENT` identity** anchored to a per-run token SID — the authorization signal the Phase 74 multi-tenant daemon will key on. A non-agent process cannot claim the identity; a confined agent cannot shed it. Named job objects are kill-group / descendant-capture / enumeration / resource-caps only — **never** authorization. Requirement: **MARK-01**. This is the **daemon prerequisite**; depends only on Phase 71's productionized launch path, independent of Phase 72 (parallel-capable).

**In scope:**
- Anchor the marker to the per-run **AppContainer package SID** already minted on every `BrokerLaunchNoPty` agent (D-01).
- An in-memory **AgentRegistry** (minting authority's private set) as the **sound authorization predicate** (D-02), wired into the live launch path so real launches register at spawn (D-04).
- **Job hardening**: explicit owner/daemon-only ACL on the job + lock the breakaway-denied / can't-shed invariants via negative tests (D-03).
- **Classification mechanism** in the `nono` crate (read a PID's package SID + `IsProcessInJob`) + a best-effort, explicitly non-authoritative `nono classify <pid>` CLI verb (D-04).
- Document **adopted (not-launched) agents** as best-effort / demote-only (SC5).

**Out of scope (own phases):**
- The persistent multi-tenant daemon, multi-client capability pipe, cross-tenant denial, token/job reuse-vs-fresh (Phase 74).
- Per-agent WFP egress scoping, post-hoc demote, Copilot/`nono-ts` (Phase 75).
- Making the registry persistent / cross-process / multi-tenant — Phase 73's registry is per-run, in-memory, single-launch (Phase 74 makes it daemon state).

</domain>

<decisions>
## Implementation Decisions

### Marker anchor & token placement (SC1)
- **D-01:** The `AI_AGENT` marker is anchored to the **per-run AppContainer package SID** already present on every `BrokerLaunchNoPty` agent's token (derived from the random `nono.session.<uuid v4>` name → 122-bit unguessable; queryable via `GetTokenInformation(TokenAppContainerSid)`; already the WFP **E4** network identity). **One per-run SID serves confinement-network-identity AND authorization — no new token-crafting.** The marker exists the moment the agent spawns.
  - **Rejected — dedicated added marker SID:** at user privilege the only mechanism to stamp a synthetic SID onto a token is `CreateRestrictedToken`, which makes it a *restricting* SID with access-check effects (the exact thing that broke the CLR — F-60-UAT-05); a pure group SID needs `SeCreateTokenPrivilege` (TCB-only, violates DMON-03 least-privilege). The package SID sidesteps this entirely.
  - **Rejected — reuse `S-1-5-117-*` restricting SID:** that SID is carried **only on the `WriteRestricted` arm**; the engine-agnostic/daemon path is `BrokerLaunchNoPty` (Low-IL primary token via `create_low_integrity_primary_token()`, which adds **no** SID). It is not present on the target agents.
  - **CONSTRAINT (load-bearing):** the daemon path **MUST always route through the Broker arm** (`BrokerLaunchNoPty`, which already `ok_or_else`-refuses to spawn without an AppContainer — launch.rs:1867). `LowIlPrimary` (carries neither SID) and `WriteRestricted` (carries `S-1-5-117-*`, not the package SID) fall **outside** this anchor and are not the marked path.

### Unforgeability / authorization predicate (SC2)
- **D-02:** Authorization = **the token's AppContainer package SID is a member of the minting authority's PRIVATE registry of SIDs it actually minted.** The 122-bit random name makes the SID unguessable; the private registry means a self-created AppContainer (even one named `nono.session.<guess>`) is **rejected** unless it is the exact minted value.
  - The `nono.session.*` namespace and job membership are **cheap enumeration pre-filters ONLY — never the authorization check.** (Package SID is a *pure function* of the name via `DeriveAppContainerSidFromAppContainerName`, so the namespace/pattern alone is forgeable; only the random suffix + private registry is sound.)
  - **Rejected — namespace pattern match** (forgeable; fails SC2) and **token-handle pinning** (stronger, but more live state than 73 needs pre-daemon; revisit in 74 if SID-value collision ever becomes a concern).

### Job hardening (SC3, SC4)
- **D-03:** Phase 73:
  1. Add an **explicit security descriptor** to `CreateJobObjectW` granting only the launcher/owner principal (becomes daemon-only in 74) and **denying the agent's package SID / Low-IL** any job access. (Currently null security attributes → default DACL — launch.rs:199.)
  2. **Negative tests** asserting `JOB_OBJECT_LIMIT_BREAKAWAY_OK` is **never** set (already true today — only `KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION`) and the confined child **cannot open/modify** its own job (MIC already blocks Low→Medium; assert it).
  3. Classification reads job membership via `IsProcessInJob` / `QueryInformationJobObject` for **enumeration only**, never for authz.
  - The existing job **name** (`Local\nono-session-{id}`) is kept; opening the job by name confers **no** identity (authz is the token SID — SC2).

### Classification surface & wiring (SC4)
- **D-04:** The `nono` crate gets: **marker extraction** (open a PID → `OpenProcessToken` → read the AppContainer package SID; `IsProcessInJob`) and an **in-memory `AgentRegistry`** (insert-on-mint; `classify(pid)` = sound registry check + namespace/job pre-filter). **Wire mint→registry into the live launch path** so real launches register the package SID at spawn-time (genuinely satisfies SC1 "a launched agent is marked").
  - **SC4 proof = in-process integration test:** a real confined agent → `AI_AGENT`; an unrelated process → not; a self-made AppContainer spoof → not (rejected, absent from registry).
  - Ship a **best-effort `nono classify <pid>` CLI verb**, **documented as NON-authoritative** (it can only do the structural pre-filter cross-process; it is the operator/demo surface and the SC5 adopted-agent path — not the security boundary).
  - **Rejected — lib+tests only (no live wiring)**: would prove SC1 only in a harness, not the shipping path. **Rejected — CLI-first persisted on-disk registry**: throwaway infra the 74 in-memory daemon replaces + a second source of truth.

### Claude's Discretion
- **SC5 — adopted/not-launched agents:** documented as **best-effort / demote-only** (the marker is sound only for launcher-spawned agents). The exact wording + where it lives (binding docs / DESIGN doc) is Claude's discretion; the best-effort `nono classify` verb is the concrete surface for the weaker structural classification.
- `AgentRegistry` internal shape (map type, key = package SID bytes/string), error-path wording, and the precise `nono classify` output format are Claude's discretion — keep fail-secure (unknown PID → "not an agent", never a false positive).
- Exact SDDL/security-descriptor construction for the job ACL (which `Deny` ACEs for the package SID + Low-IL label) is Claude's discretion within D-03's intent.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spike findings (the validated model this marker sits on)
- `.claude/skills/spike-findings-nono/SKILL.md` — spike-findings index (auto-loaded for token-labeling/daemon work).
- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` — SEED-004 / spike-003 (VALIDATED): daemon-as-launcher; the multi-tenant `AI_AGENT` marker is called out as **not yet spiked** (was spike 004); the supervisor cap pipe is the generalization starting point.
- `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` — spawn-time is the sound mode; post-hoc IL-drop is demote-only/leaky (the basis for SC5's adopted = best-effort/demote-only).

### Milestone / requirements
- `.planning/ROADMAP.md` §"Phase 73: AI_AGENT Marker" — SC1–SC5; the "named job = kill-group/enumeration only, never authorization" invariant; §"Phase 74" for the daemon consumer this prerequisite feeds.
- `.planning/REQUIREMENTS.md` — **MARK-01** + traceability (DMON-01..03 downstream).

### Carried-forward phase context
- `.planning/phases/71-engine-agnostic-launch-productionization/71-CONTEXT.md` — the productionized Broker-arm launch path this marker hooks into; locked "composition over existing subsystems / no new framework" constraint; AppContainer/`CreateAppContainerProfile` + CLR env-baseline re-assertions.
- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-CONTEXT.md` — parallel binding work (E1–E5 contract; E4 = network identity = the package SID this marker reuses).
- `proj/DESIGN-engine-abstraction.md` — E1–E5 abstraction boundary (E4 network identity = AppContainer package SID).

### Existing code (implementation targets)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — job creation (`CreateJobObjectW`, ~line 199; null SA today → add ACL per D-03); `BrokerLaunchNoPty` AppContainer-required spawn (`app_container_name` `ok_or_else`, ~line 1867); `select_windows_token_arm` (~line 1190).
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — `generate_session_sid()` (`S-1-5-117-*`, NOT the chosen anchor), `generate_app_container_name()` (`nono.session.<uuid>` → the chosen anchor's source), `create_restricted_token_with_sid()` (WriteRestricted arm only).
- `crates/nono/src/sandbox/windows.rs` — `create_low_integrity_primary_token()` (Broker arm token; adds no SID — ~line 534); package-SID derivation helpers; the natural home for the new marker-extraction + `AgentRegistry` library primitives (library-vs-CLI boundary: mechanism in `nono`, the verb/UX in `nono-cli`).
- `crates/nono/src/supervisor/socket_windows.rs` — cap-pipe + `ImpersonateNamedPipeClient` neighborhood the Phase 74 daemon will extend to consume the registry.

### Cross-target discipline
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for any cfg-gated Unix code touched (the new library primitives must keep a non-Windows stub).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Per-run AppContainer package SID** — already minted (`generate_app_container_name` → `CreateAppContainerProfile` → `SECURITY_CAPABILITIES.AppContainerSid`) on every `BrokerLaunchNoPty` agent and already the WFP E4 identity. The marker reuses it wholesale (D-01) — no new SID, no new token crafting.
- **Named, breakaway-denied job** — `CreateJobObjectW` with `Local\nono-session-{id}`, `KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION`, `BREAKAWAY_OK` never set. SC3's breakaway guarantee is **already true**; 73 adds the explicit ACL + negative tests (D-03).
- **`select_windows_token_arm`** — already routes the engine-agnostic path to `BrokerLaunchNoPty` (AppContainer-required), the only marked arm.

### Established Patterns
- **Library-vs-CLI boundary** — the marker *mechanism* (SID extraction, `AgentRegistry`, `classify`) belongs in the `nono` crate (policy-free primitive); the `nono classify` verb + UX belongs in `nono-cli`. (CLAUDE.md "Library is policy-free".)
- **Fail-secure default** — unknown/unmatched PID classifies as "not an agent" (never a false positive); coverage/authz checks deny on any error.
- **cfg-gated Unix stubs** — every new Windows primitive needs a non-Windows stub and cross-target clippy (CLAUDE.md MUST).

### Integration Points
- `BrokerLaunchNoPty` spawn path → on successful mint, insert the package SID into the `AgentRegistry` (the new wiring in D-04).
- `AgentRegistry::classify(pid)` → consumed in-process by the SC4 test now; consumed by the Phase 74 daemon over the cap pipe later.
- New job security descriptor threads into the existing `CreateJobObjectW` call (replace null SA).

</code_context>

<specifics>
## Specific Ideas

- The **package SID is a pure function of the AppContainer name** (`DeriveAppContainerSidFromAppContainerName`) — this is *why* the namespace alone is forgeable and the registry is mandatory for SC2. Researcher: confirm reading a foreign PID's AppContainer SID via `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `OpenProcessToken(TOKEN_QUERY)` → `GetTokenInformation(TokenAppContainerSid)`.
- SC4 acceptance is an **in-process integration test** (no daemon), exercising all three outcomes: real confined agent = AI_AGENT, unrelated process = not, self-made-AppContainer spoof = not.
- Re-assert at implementation time (carried from 71): AppContainer per-agent SID needs `CreateAppContainerProfile` (derive-only → `CreateProcessW ERROR_FILE_NOT_FOUND`); preserve `SystemRoot`/`windir`/`SystemDrive` env baseline (else CLR `0xFFFF0000`). Broker arm needs dev-layout or signed `nono.exe` (R-B4 trust gate) to run on a real host.

</specifics>

<deferred>
## Deferred Ideas

- **Persistent / multi-tenant / cross-process registry** — Phase 73's registry is per-run, in-memory, single-launch. Persistence + multi-tenant isolation + serving over the cap pipe is Phase 74 (DMON-01/02).
- **Token-handle pinning** (identity tied to the live primary-token handle, not just the SID value) — stronger anti-impersonation; revisit in 74 only if SID-value collision becomes a real concern.
- **A first-class `nono agent` / daemon verb namespace** — the daemon (74) may introduce verbs; 73 adds only the minimal best-effort `nono classify <pid>`.
- **Marking `WriteRestricted` / `LowIlPrimary` arms** — out of scope; the daemon path is Broker-arm-only (D-01 constraint). Revisit only if a non-Broker marked path is ever needed.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — keyword false-positive (MSI/distribution prereq), unrelated to the in-process marker. Not folded.
- `20260611-poc-cert-broker-clean-host.md` — keyword false-positive (POC cert broker on clean host / distribution), unrelated to MARK-01. Not folded.

</deferred>

---

*Phase: 73-ai-agent-marker*
*Context gathered: 2026-06-14*
