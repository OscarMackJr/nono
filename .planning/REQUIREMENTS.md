# Requirements: nono — v2.12 AI Agent Abstraction

**Defined:** 2026-06-13
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and that confinement must apply to *any* AI agent engine, not just Claude Code.

**Trigger:** SEED-004 (multi-engine pluggability). Spikes 001–003 proved the achievable model (`spike-findings-nono`): the TUI is OS-blocked, but **launch-and-confine is engine-neutral** — `nono run -- <exe>` already confines arbitrary executables and all their descendants; the Claude-specificity lives only in the PreToolUse hook. Spike 003 (VALIDATED) confined cmd/PowerShell/python identically through one launcher; spike 002 showed post-hoc IL-drop is demote-only. Research (`.planning/research/SUMMARY.md`) confirms v2.12 is a **composition** milestone over three existing subsystems (broker-arm launch, the SDDL capability pipe, the `nono-wfp-service` shape) plus one net-new marker, with the persistent multi-tenant daemon as the riskiest piece. User-mode only — no kernel driver (ADR-65 No-go).

## v1 Requirements (v2.12 Scope)

### Engine-Agnostic Launch (ENG)

- [x] **ENG-01**: A user can run a non-Claude agent engine (e.g. Aider) confined by nono end-to-end on a real Win11 host — files written inside the granted workspace land; writes outside it are denied (`NO_WRITE_UP`) — regardless of engine.
- [x] **ENG-02**: The launcher fails **secure with an actionable message** when an engine's executable/interpreter path is not covered by the launch policy, or when the granted workspace is not owned by the session user (R-B3) — never silent partial confinement.
- [x] **ENG-03**: A user can declare a **per-engine launch profile** (executable + interpreter path(s), absolute writable workspace, network identity) and launch any profiled engine through one engine-neutral path.

### Engine Abstraction & Bindings (ABI)

- [x] **ABI-01**: A Python/LangChain agent can be confined through the **`nono-py` binding with no Claude hook** — both by launching it confined (`confined_run`) and by self-confining at interpreter startup (`confine`) so its in-process `exec()` tools are bounded.
- [x] **ABI-02**: The **engine-abstraction contract** (what every engine must expose: executable/interpreter path, an ownable launch command, an absolute workspace grant, a network identity, and an optional pre-exec hook) is documented as a stable boundary other engines implement against.

### AI_AGENT Marker (MARK)

- [x] **MARK-01**: Each confined agent carries an **unforgeable `AI_AGENT` identity** bound to its daemon-minted token SID; a non-agent process cannot claim the identity and a confined agent cannot shed it. (Named job objects, if used, are for kill-group/enumeration/resource-caps only — never for authorization.)

### Multi-Tenant Daemon (DMON)

- [x] **DMON-01**: A persistent local daemon launches and confines **multiple concurrent agents**, each with a fresh token + job object, deterministically reaped on agent exit — running N agents over time returns to baseline handle/job count (no leak).
- [x] **DMON-02**: The daemon's multi-tenant capability pipe **isolates tenants** — it authenticates each client server-side (`ImpersonateNamedPipeClient` + per-tenant SID match) so one agent cannot read or use another agent's capabilities (a cross-tenant request is denied).
- [x] **DMON-03**: The daemon runs at **least privilege** (user, not LocalSystem) and is split from the elevated WFP-control service, so a confined agent that escapes cannot pivot to SYSTEM or to other tenants. (Backed by a privilege-model ADR.)

### Supplementary Controls & Reach (SUPP)

- [x] **SUPP-01**: An operator can **demote a running/misbehaving agent** on the fly (post-hoc token IL-drop) as a supplementary control, with the leak/soundness limits documented (explicitly not a standalone boundary).
- [x] **SUPP-02**: Outbound network egress is **scoped per confined agent** (WFP keyed to the agent's identity) so each agent's network policy is enforced independently.
- [x] **SUPP-03**: A second non-Claude engine ships as a profile (**GitHub Copilot CLI**) and the **`nono-ts` (Node) binding reaches parity** with `nono-py` (confined-run + self-confine) — proving the abstraction across ≥2 engines and ≥2 bindings.

## v2 Requirements (Deferred)

- **Signed-policy / decentralized attestation** (SEED-005 / review R-T1) — sign per-engine policy + map to a ledger trust root. X-Large; its own milestone.
- **Sound adoption of an already-running agent** nono did not launch — blocked by the post-hoc-IL-drop leak (spike 002); needs a different mechanism.
- **Cursor native-Windows confinement** — Cursor's agent CLI is Linux/macOS/WSL-only today (anti-feature for native Windows).

## Out of Scope (explicit exclusions)

- **Kernel driver / minifilter / `PsSetCreateProcessNotifyRoutine`** — ADR-65 No-go; user-mode only.
- **Enterprise fleet deployment (SEED-001), network-egress allowlist product (SEED-002), SIEM/EDR telemetry (SEED-003)** — a separate enterprise-hardening milestone.
- **Real publicly-trusted code signing** (Azure Trusted Signing) — cert-gated.

## Traceability

Every v1 requirement maps to exactly one phase. Coverage: **12/12 mapped, no orphans, no duplicates.**

| Requirement | Phase | Status |
|-------------|-------|--------|
| ENG-01 | Phase 71 — Engine-Agnostic Launch Productionization | Complete |
| ENG-02 | Phase 71 — Engine-Agnostic Launch Productionization | Complete |
| ENG-03 | Phase 71 — Engine-Agnostic Launch Productionization | Complete |
| ABI-01 | Phase 72 — nono-py Binding + In-Process-Exec Proof | Complete |
| ABI-02 | Phase 72 — nono-py Binding + In-Process-Exec Proof | Complete |
| MARK-01 | Phase 73 — AI_AGENT Marker | Complete |
| DMON-01 | Phase 74 — Persistent Multi-Tenant Daemon | Complete |
| DMON-02 | Phase 74 — Persistent Multi-Tenant Daemon | Complete |
| DMON-03 | Phase 74 — Persistent Multi-Tenant Daemon | Complete |
| SUPP-01 | Phase 75 — Supplementary Controls + Secondary Engines | Complete |
| SUPP-02 | Phase 75 — Supplementary Controls + Secondary Engines | Complete |
| SUPP-03 | Phase 75 — Supplementary Controls + Secondary Engines | Complete |

### Coverage by phase

| Phase | Requirements | Count |
|-------|--------------|-------|
| 71 — Engine-Agnostic Launch Productionization | ENG-01, ENG-02, ENG-03 | 3 |
| 72 — nono-py Binding + In-Process-Exec Proof | ABI-01, ABI-02 | 2 |
| 73 — AI_AGENT Marker | MARK-01 | 1 |
| 74 — Persistent Multi-Tenant Daemon | DMON-01, DMON-02, DMON-03 | 3 |
| 75 — Supplementary Controls + Secondary Engines | SUPP-01, SUPP-02, SUPP-03 | 3 |
| **Total** | | **12** |
