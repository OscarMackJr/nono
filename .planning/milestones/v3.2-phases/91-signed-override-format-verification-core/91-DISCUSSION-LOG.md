# Phase 91: Signed Override Format + Verification Core - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-21
**Phase:** 91-signed-override-format-verification-core
**Areas discussed:** Signed test-fixture strategy, verify_override API shape, jti store scope, NonoOverrideError taxonomy

---

## Signed test-fixture strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid | ZT-Infra `test-vectors/canonical-form/vectors.json` verbatim for `canonical_bytes()` (SC1) + local ECDSA P-256 test keypair (test-only DER pubkey) for signed-token fixtures (SC2). No AWS coupling, never accepts `algorithm:none`. | ✓ |
| ZT assets only | Reuse ZT-Infra zt-verify/provisioner fixtures for everything; couples nono tests to external repo + live/AWS-minted sigs. | |
| Local test key only | Generate all fixtures locally; loses the cross-language canonical-form guarantee OVR-03 requires. | |

**User's choice:** Hybrid (recommended)
**Notes:** Canonical form must be proven against ZT-Infra's cross-language vectors; signature path tested with a test-only key kept out of any production trust root.

---

## verify_override API shape

| Option | Description | Selected |
|--------|-------------|----------|
| VerifiedOffline value | Offline verify returns an immutable verified-grant value; Phase 93's live AND-gate consumes it without re-parsing the token (TOCTOU-safe, PITFALLS #8). | ✓ |
| Pluggable live-check seam | Single `verify_override()` with the live check injected later via callback/param; less explicit AND-gate composition. | |
| Two distinct public fns | `verify_signature_offline()` now; Phase 93 adds composing `verify_override()`; more surface area. | |

**User's choice:** VerifiedOffline value (recommended)
**Notes:** Keeps offline-pass necessary-but-not-sufficient; live arm layers on top in Phase 93.

---

## jti store scope

| Option | Description | Selected |
|--------|-------------|----------|
| Ephemeral in-process | In-process consumed-`jti` set satisfies SC3/VFY-06 for v1; cross-process replay deferred to the live ZT-Infra check (Phase 93). | ✓ |
| Persistent store now | File/SQLite consumed-`jti` store for cross-process protection; adds storage/locking + host-state to a pure verifier phase. | |
| Trait now, impl later | `ConsumedJtiStore` trait + in-process impl now; persistent impl later. | |

**User's choice:** Ephemeral in-process (recommended)
**Notes:** Boundary is intentional and must be stated in the plan so it is not read as a gap; live check is the durable cross-process single-use point.

---

## NonoOverrideError taxonomy

| Option | Description | Selected |
|--------|-------------|----------|
| One error + kind enum | Single `NonoOverrideError` with a stable machine-readable `kind` (BadSignature/Expired/NotYetValid/OutOfScope/Replay/AlgorithmMismatch/KeyNotAllowlisted/Parse/MissingField) + redaction-safe message; maps 1:1 to Phase 92 EventID 10008. | ✓ |
| Distinct exception classes | A PyO3 subclass per failure mode; more Pythonic but more boilerplate + wider redaction surface. | |
| Single opaque error | Message only, no machine-readable kind; forces Phase 92 to string-parse for audit reasons. | |

**User's choice:** One error + kind enum (recommended)
**Notes:** `kind` enum is the contract Phase 92 maps to the REJECTED audit event without parsing messages.

---

## Claude's Discretion

- Canonical re-derivation mechanics (re-derive from parsed struct; sorted keys; strip `current_hash`+`kms_signature`; SHA-256 lowercase hex) — fail-secure defaults per ZT-Infra `CANONICAL_FORM.md`.
- serde parse strictness (`deny_unknown_fields` unless a concrete ZT-Infra forward-compat need surfaces at plan time).
- Module/type layout in `nono-py/src/override.rs`, `#[pyclass]` exposure, test-fixture placement, exact `#[must_use]` placement.

## Deferred Ideas

- Cross-process / persistent `jti` store (live check is the durable point).
- Live `POST /actions` AND-gate, KMS pubkey distribution, revocation, AWS cred stripping, `nono override request`, DAAL — Phase 93.
- `CapabilitySet` mutation + audit emission (EventIDs 10006–10010) — Phase 92.
- `nono-ts` parity — FUT-03, future milestone.
