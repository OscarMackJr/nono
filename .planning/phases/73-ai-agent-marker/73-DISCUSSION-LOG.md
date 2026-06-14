# Phase 73: AI_AGENT Marker - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 73-ai-agent-marker
**Areas discussed:** Marker SID & placement, Unforgeability model, Named job hardening, Classify surface & wiring

---

## Marker SID & placement

Pre-discussion scout finding: the engine-agnostic/daemon path uses `BrokerLaunchNoPty`, whose token comes from `create_low_integrity_primary_token()` (adds NO SID). The `S-1-5-117-*` session SID is carried only on the `WriteRestricted` arm. But `BrokerLaunchNoPty` *refuses to spawn without an AppContainer* (launch.rs:1867), so every such agent carries a per-run AppContainer package SID.

| Option | Description | Selected |
|--------|-------------|----------|
| AppContainer package SID | Reuse the per-run package SID already on every Broker agent (random `nono.session.<uuid>`, unguessable, queryable, already WFP E4). One SID for confinement-identity + authz; no new token crafting. | ✓ |
| Dedicated added marker SID | Mint a separate uniform marker SID across all arms. Blocked: user-privilege can only add *restricting* SIDs (`CreateRestrictedToken`, broke CLR) or needs `SeCreateTokenPrivilege` (TCB-only, violates DMON-03). | |
| Per-arm: whichever SID that arm has | Daemon registry maps any minted SID → agent regardless of arm. Flexible but multi-slot; `LowIlPrimary` unmarkable. | |

**User's choice:** AppContainer package SID (Recommended)
**Notes:** Implies the daemon path must always route through the Broker arm (load-bearing constraint captured in CONTEXT D-01).

---

## Unforgeability model

Wrinkle surfaced: the package SID is a pure function of the AppContainer name (`DeriveAppContainerSidFromAppContainerName`), so the `nono.session.*` namespace alone is forgeable. Unforgeability must rest on the random suffix + a private registry.

| Option | Description | Selected |
|--------|-------------|----------|
| Registry of minted SIDs | Authz = token's package SID ∈ minting authority's private set of actually-minted SIDs. Random name = unguessable; namespace is pre-filter only. | ✓ |
| Namespace pattern match | Authz = SID derives from `nono.session.*`. Simpler but forgeable; fails SC2. | |
| Registry + token-handle pinning | Registry membership + retain the live token/process handle. Stronger but more live state than 73 needs pre-daemon. | |

**User's choice:** Registry of minted SIDs (Recommended)
**Notes:** Namespace/job membership are enumeration pre-filters only, never the authz check.

---

## Named job hardening

Current state (launch.rs:190-244): job is already named `Local\nono-session-{id}`, breakaway already denied (`BREAKAWAY_OK` never set), but created with null security attributes (default DACL). The Low-IL/AppContainer child is already MIC-blocked from its Medium-IL job.

| Option | Description | Selected |
|--------|-------------|----------|
| Lock invariants + explicit ACL now | Add explicit owner/daemon-only SD + deny agent SID/Low-IL; negative tests (breakaway never set, child can't open job); `IsProcessInJob` for enumeration only. | ✓ |
| Lock invariants only; defer ACL to daemon | Test existing structural guarantees; defer explicit ACL to Phase 74 where peers exist. Smaller 73; SC3 ACL bullet slips. | |
| Full ACL + rename to per-run random | As option 1 plus drop/randomize the job name. Strongest SC2 but changes supervisor job-discovery-by-name assumptions. | |

**User's choice:** Lock invariants + explicit ACL now (Recommended)
**Notes:** Keep the existing `Local\nono-session-{id}` name; opening by name confers no identity.

---

## Classify surface & wiring

A stateless `nono classify <pid>` can't hold the private registry across processes, so by itself it could only do the forgeable pre-filter. The sound check needs an in-memory registry held by the minting process — so the fork is how much wires into the live launch path before the daemon exists.

| Option | Description | Selected |
|--------|-------------|----------|
| Lib mechanism + registry, wired now | `nono` crate: marker extraction + in-memory `AgentRegistry` (insert-on-mint, sound `classify`). Wire mint→registry into the live launch path (satisfies SC1). SC4 proven by in-process integration test. Best-effort non-authoritative `nono classify <pid>` verb. | ✓ |
| Lib primitives + tests only; daemon wires it | Mechanism + API + proof-by-test, no live wiring, no verb. Smallest; SC1 proven only in a harness. | |
| CLI-first with persisted registry | `nono classify` over an on-disk ACL'd registry. More infra the 74 daemon replaces; second source of truth. | |

**User's choice:** Lib mechanism + registry, wired now (Recommended)
**Notes:** Registry is per-run/in-memory in 73; persistence + multi-tenant is Phase 74.

## Claude's Discretion

- SC5 adopted-agent wording/location (binding docs vs DESIGN doc); the best-effort `nono classify` verb is its concrete surface.
- `AgentRegistry` internal shape, error wording, `nono classify` output format (fail-secure: unknown → not an agent).
- Exact SDDL/security-descriptor for the job ACL within D-03's intent.

## Deferred Ideas

- Persistent/multi-tenant/cross-process registry → Phase 74.
- Token-handle pinning → revisit in 74 if SID-value collision matters.
- First-class `nono agent`/daemon verb namespace → Phase 74.
- Marking `WriteRestricted`/`LowIlPrimary` arms → out of scope (Broker-arm-only).
- Reviewed-not-folded todos: `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md` (keyword false-positives, distribution-related).
