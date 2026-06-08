---
id: SEED-002
status: dormant
planted: 2026-06-08
planted_during: v2.10 (Phase 63)
trigger_when: milestone scope includes outbound network egress control, data-exfil prevention, domain allowlisting, or WFP enhancement
scope: medium
priority: P1
---

# SEED-002: Network Egress Filtering — Corporate Domain Allowlist (Anti-Exfiltration)

## Why This Matters

An agent can leak corporate data or source code just as easily by POSTing a base64-encoded string to an unauthorized third-party endpoint as by writing to a local folder. Enterprises need a **deny-by-default outbound boundary** that allowlists only authorized corporate domains and trusted AI-provider APIs (e.g. `*.anthropic.com`, `*.openai.com`, internal model gateways).

This is the **P1 ("Control")** priority of the enterprise horizon.

## When to Surface

**Trigger:** when a milestone targets outbound network control, exfiltration prevention, enterprise domain allowlisting, or extending the WFP component.

This seed will surface during `/gsd:new-milestone` when the milestone scope matches.

## Scope Estimate

**Medium — substantially an ENHANCEMENT, not greenfield.** ⚠️ nono **already has** kernel WFP network enforcement (Phase 62, 13/13, Windows supervised) and fine-grained `allow_domain` method+path filtering (Phase 56, REQ-NET-01). When this surfaces, **scope it as gap-closure**, not a from-scratch build. Net-new work likely:
- Enterprise-policy-managed allowlists (corporate domains + AI-provider wildcards) sourced from machine policy rather than per-user profile.
- Default-deny posture verification at fleet scale.
- Reconcile the proxy-based filtering (`nono-proxy`) with the kernel WFP path (`nono-wfp-service`) into one coherent enterprise egress story.

## Breadcrumbs

- `crates/nono-proxy/` — domain filtering + credential injection (`server.rs`, `filter.rs`, `credential.rs`).
- `nono-wfp-service` / Phase 62 — kernel WFP `FwpmFilterAdd0` enforcement (`windows_wfp_enforcement_is_service_only`).
- Phase 56 — fine-grained `allow_domain` (per-endpoint method+path); `windows_appcontainer_wfp_validated`.

## Notes

Captured 2026-06-08 (CISO/CTO horizon). Heavy overlap with shipped WFP work — verify current capability before planning. Sibling seeds: [[SEED-001]], [[SEED-003]], [[SEED-004]], [[SEED-005]].
