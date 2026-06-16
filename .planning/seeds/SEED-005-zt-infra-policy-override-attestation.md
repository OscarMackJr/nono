---
id: SEED-005
status: dormant
planted: 2026-06-08
planted_during: v2.10 (Phase 63)
trigger_when: milestone scope includes signed policy overrides, decentralized attestation, immutable audit ledger, or ZT-Infra integration
scope: x-large
priority: P3
---

# SEED-005: Decentralized Attestation via ZT-Infra — Signed Policy Overrides

## Why This Matters

Developers will hit false positives where nono blocks a legitimate write or network call. In a local-only setup the temptation is to disable the sandbox or bypass the profile — defeating the whole control. Instead, an exception should be a **cryptographically signed policy override** (signed by, e.g., an engineering manager) logged to an **immutable audit ledger**. If the signature verifies, nono temporarily mutates its runtime ruleset for that specific repository context — auditable, revocable, non-self-service.

## When to Surface

**Trigger:** when a milestone targets signed/attested policy overrides, decentralized exception management, immutable audit ledgers, or integration with the ZT-Infra v2 ledger project.

This seed will surface during `/gsd:new-milestone` when the milestone scope matches.

## Scope Estimate

**X-Large with an EXTERNAL DEPENDENCY (ZT-Infra v2 ledger) — likely its own later milestone or a cross-project effort.** Work:
- Signed policy-exception format (who signs, what scope, expiry, repo-context binding).
- Signature verification + runtime ruleset mutation gated on a valid signature (nono already has sigstore-based attestation primitives — reuse, don't reinvent).
- Ledger write/read integration with the external ZT-Infra v2 project (the dependency that makes this a later milestone).
- Tamper-evident link to [[SEED-003]] (the audit-logging pipeline) — overrides are themselves security events.

## Breadcrumbs

- Existing attestation stack: `sigstore-rs` (`sigstore-verify`, `sigstore-sign`) — already a dependency; the signature-verification primitive for signed overrides.
- `crates/nono-cli/src/policy.rs` — the group/deny policy resolver whose ruleset would be temporarily mutated on a verified override.
- `crates/nono/src/capability.rs` — `CapabilitySet` (the runtime ruleset that an attested override would expand for a repo context).
- External: **ZT-Infra v2 decentralized ledger project** (not in this repo — the integration dependency). Live codebase: `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2` (Apache-2.0 AWS control plane; `POST /actions` allow/deny + hash-chained KMS-signed audit + optional DAAL ledger). Cheapest local exercise = the **local provisioner**, no AWS: `cd provisioner && npm install && npm start` → `POST http://127.0.0.1:3000/actions` (see its `README.md` "Local Provisioner"). A nono-side POC runbook for the E5 composition (zt-infra decides → `confined_run` enforces) is at [`proj/POC-zt-infra-e5-local-provisioner.md`](../../proj/POC-zt-infra-e5-local-provisioner.md). NOTE (2026-06-15): this seed is NOT on the Phase 73 or 75 path — both are pure nono work; this remains its own future milestone.

## Notes

Captured 2026-06-08 (CISO/CTO horizon). Most speculative of the five due to the external ledger dependency; sequence AFTER [[SEED-003]] (audit pipeline) and likely as a standalone milestone. Sibling seeds: [[SEED-001]], [[SEED-002]], [[SEED-003]], [[SEED-004]].
