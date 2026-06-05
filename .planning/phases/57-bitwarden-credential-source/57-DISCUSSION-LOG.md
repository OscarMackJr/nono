# Phase 57: Bitwarden Credential Source - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-05
**Phase:** 57-bitwarden-credential-source
**Areas discussed:** CLI & auth model, URI grammar, field types, locked/missing behavior, concrete grammar, scope/delivery order

---

## CLI & Auth Model

| Option | Description | Selected |
|--------|-------------|----------|
| `bw` CLI (session token) | Password-manager CLI; items by name/id + field; BW_SESSION unlock; op:// analog | |
| `bws` Secrets Manager (access token) | Machine/unattended; secrets by UUID; BWS_ACCESS_TOKEN | |
| Both / abstract over them | Support both behind bw://; max flexibility, larger surface | ✓ |

**User's choice:** Both / abstract over them.
**Notes:** Accepted the larger surface knowingly; the two backends are distinguished by URI shape (`item/` vs `secret/`).

---

## URI Grammar (addressing)

| Option | Description | Selected |
|--------|-------------|----------|
| Item + field (by name) | Human-readable; non-unique; harder to validate | |
| Item + field (by id) | UUID/hex charset; unique + injection-safe; opaque to author | ✓ |
| Let me specify | Custom grammar | |

**User's choice:** Item + field (by id).
**Notes:** ID-based for both backends — strongest validation story.

---

## Field Types

| Option | Description | Selected |
|--------|-------------|----------|
| Password only | Minimal surface | |
| Username + password | Both login fields | ✓ |
| Custom fields | Named custom fields (where API keys often live) | ✓ |
| TOTP / notes | One-time codes + secure-note bodies | ✓ |

**User's choice:** Username+password, Custom fields, TOTP/notes (effectively all field types; password implied).
**Notes:** Applies to the `bw` item backend only — `bws` secrets are single opaque values.

---

## Locked / Missing Behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Fail-closed, clear diagnostic | Abort with remediation message; never proceed without secret | ✓ |
| Fail-closed + auto-unlock attempt | Try non-interactive unlock from env material first | |

**User's choice:** Fail-closed, clear diagnostic.
**Notes:** Auto-unlock explicitly rejected — keeps passphrase out of env, tighter threat surface.

---

## Concrete Grammar (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| Typed first segment | `bw://item/<id>/<field>` (+ `field/<name>`) and `bw://secret/<uuid>` | ✓ |
| Backend sub-scheme | `bw+item://` / `bw+secret://` | |
| Let me specify | Custom selector | |

**User's choice:** Typed first segment.
**Notes:** Reserved field names + explicit `field/<name>` for custom fields; `secret/` form takes no field selector.

---

## Scope / Delivery Order (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| Both in one phase | bw + bws together; one cohesive change/review | ✓ |
| bw first, bws as a follow-on plan | Land bw item backend first, bws as final plan | |

**User's choice:** Both in one phase.
**Notes:** bw:// surface ships complete in Phase 57.

---

## Claude's Discretion

- Module decomposition within `keystore.rs`; JSON-parsing approach for `bw get item`.
- Exact non-interactive CLI flags per tool; precise diagnostic wording.
- Test fixture strategy (mock CLI shim vs trait-injected command runner).

## Deferred Ideas

- Non-interactive auto-unlock (passphrase/client-secret in env) — rejected for this phase; possible opt-in future enhancement.
- Addressing items by name (vs id) — rejected for validation safety; possible future validated name-lookup mode.
- Credential writing, session caching/daemonization, org/collection management — separate capabilities, out of scope.
