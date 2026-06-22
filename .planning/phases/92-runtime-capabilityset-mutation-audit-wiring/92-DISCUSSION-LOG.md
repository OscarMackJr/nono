# Phase 92: Runtime CapabilitySet Mutation + Audit Wiring - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-21
**Phase:** 92-runtime-capabilityset-mutation-audit-wiring
**Areas discussed:** Mutation + audit locus, VFY-01 treatment, Platform scope, Scope match + audit fields, AUD-04 fail-closed handshake

---

## Mutation + audit locus

| Option | Description | Selected |
|--------|-------------|----------|
| nono-py verifies, CLI audits (Option B) | nono-py runs Phase 91 verify, appends grant paths as `--allow`, passes trusted audit metadata to nono.exe; nono-cli emits PolicyOverrideApplied + aborts before spawn if it can't emit. Keeps verifier in nono-py; CLI trusts launcher metadata (same model as `--allow`). | ✓ |
| CLI verifies end-to-end (Option A) | Pass `--override-token` to nono.exe; nono-cli verifies+applies+audits+confines in one process. Requires duplicating Phase 91 crypto or relocating the verifier into core nono. | |

**User's choice:** nono-py verifies, CLI audits (Option B)
**Notes:** Captured as D-01. Driven by the finding that `SecurityEventLayer` lives only in nono-cli with no production path from nono-py, and Phase 91's deliberate placement of the verifier in nono-py.

---

## VFY-01 treatment

| Option | Description | Selected |
|--------|-------------|----------|
| PARTIAL seam | Wire the offline arm + build the composition point where Phase 93's live arm slots in; mark VFY-01 PARTIAL with `[BLOCKING-93]`. Mirrors Phase 91 VFY-03. | ✓ |
| Pull live arm forward | Implement the live POST /actions AND-gate now. Contradicts ROADMAP order + Phase 91 D-02. | |

**User's choice:** PARTIAL seam
**Notes:** Captured as D-03.

---

## Platform scope

| Option | Description | Selected |
|--------|-------------|----------|
| Windows-only v1 | Wire overrides into Windows `confined_run`/`confine` only; document a seam for non-Windows `sandboxed_exec`. | ✓ |
| Windows + non-Windows | Also wire `sandboxed_exec` / Landlock+Seatbelt now. Doubles surface; non-Windows branch untestable on the dev host. | |

**User's choice:** Windows-only v1
**Notes:** Captured as D-04.

---

## Scope match + audit fields

| Option | Description | Selected |
|--------|-------------|----------|
| Reuse layer + extend grant | Sanitize grant paths in nono-py + append verbatim; rely on nono-cli's existing component-wise `Path::starts_with`; add a read-only `zt_audit_hash` getter to `OverrideGrant` (from `token.current_hash`); `kms_key_id` already exposed as `signer`. | ✓ |
| New matching in nono-py | Build dedicated path-component matching in nono-py + separate parsed-token accessor. Duplicates the nono-cli layer; risks divergence. | |

**User's choice:** Reuse layer + extend grant
**Notes:** Captured as D-05 (matching) and D-06 (audit fields). Audit fields read from the verified grant, never by re-parsing the token (honors Phase 91 D-02 TOCTOU closure).

---

## AUD-04 fail-closed handshake

| Option | Description | Selected |
|--------|-------------|----------|
| Mandatory flag + capability gate | nono-py passes override `--allow` only with a required `--override-audit` flag; nono-cli aborts before spawn if it can't emit; nono-py refuses to launch a nono.exe that doesn't advertise override support. Fail-closed both sides. | ✓ |
| CLI-side gate only | Only nono-cli enforces; no nono-py version probe. Leaves a silent-escalation window with an old CLI. | |
| You decide (fail-closed default) | Leave mechanism to research/planner with a locked fail-closed-end-to-end constraint. | |

**User's choice:** Mandatory flag + capability gate
**Notes:** Captured as D-02. Bilateral fail-closed handshake — neither side alone is sufficient to close AUD-04 across the process/repo boundary.

---

## Claude's Discretion

- Exact name/format of the audit-metadata flag (`--override-audit` working name).
- CLI capability-advertisement / min-version probe mechanism (must be fail-closed).
- Field placement of `PolicyOverrideApplied` on `AuditEventPayload` vs the `SecurityEventLayer` emission shape; EventID 10006–10010 constant definitions.
- Redaction shape for override events (AUD-03).
- OVERRIDE-01 gate token minting (reuse Phase 91 test keypair) and gate file structure.

## Deferred Ideas

- Live POST /actions AND-gate, revocation, AWS cred stripping, `nono override request/apply` CLI, DAAL anchoring — Phase 93.
- Non-Windows override wiring (`sandboxed_exec` / Landlock / Seatbelt) — future; seam documented only.
- Cross-process / persistent `jti` store — Phase 91 D-03 deferral; live ZT check is the durable point.
- `nono-ts` binding parity — FUT-03, future milestone.
- Reconciling the nono-side token wire shape with real KMS tokens — Phase 91 D-06 `[BLOCKING]` for Phase 93.
