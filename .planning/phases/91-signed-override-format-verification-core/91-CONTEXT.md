# Phase 91: Signed Override Format + Verification Core - Context

**Gathered:** 2026-06-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the **fully offline, fail-closed verifier** for ZT-Infra CAF v0.1 override tokens, living in **`nono-py/src/override.rs`** (a new module). Deliverables: the `OverrideToken` parse model, a `canonical_bytes()` re-derivation validated against ZT-Infra's cross-language test vectors, the offline `verify_override` path (signature + expiry/`not_before` + scope + `jti` replay + algorithm/key-allowlist checks), an immutable verified-grant value type, and `NonoOverrideError` raised at the PyO3 boundary for every failure. Promote `aws-lc-rs` / `base64` / `chrono` to direct deps where the module needs them.

**In scope:** CAF v0.1 token format (OVR-01/02/03); offline verification (VFY-02 algorithm/low-S pinning, VFY-03 embedded KMS pubkey, VFY-04 key-ARN allowlist, VFY-05 expiry/skew/TTL cap, VFY-06 single-use `jti`, VFY-07 fail-closed `#[must_use]` Result → `NonoOverrideError`).

**Explicitly NOT in this phase (later phases own these):**
- **Live `POST /actions` AND-gate** (VFY-01) → Phase 93. Phase 91 is offline-only by design; the API is shaped so the live arm slots in (see D-02).
- **`CapabilitySet` mutation + audit emission** (MUT-*, AUD-*) → Phase 92.
- **AWS cred stripping, revocation, CLI `request`, DAAL** → Phase 93.

</domain>

<decisions>
## Implementation Decisions

### Test-fixture strategy
- **D-01 (Hybrid fixtures):** Two fixture sources, by purpose.
  - **Canonical form (SC1 / OVR-03):** consume ZT-Infra's `test-vectors/canonical-form/vectors.json` **verbatim** to prove `canonical_bytes()` produces matching SHA-256 digests — this is the cross-language guarantee OVR-03 requires; do not regenerate it locally.
  - **Signature path (SC2 / VFY-02):** mint signed-token fixtures with a **committed local ECDSA P-256 test keypair** whose DER public key is injected **only in tests** (never a production trust root). This lets "valid token → `Ok`" be tested without coupling CI to AWS/KMS and without ever accepting `algorithm:"none"`.
- Keep the test pubkey injection path test-only — production pubkey sourcing is embedded machine-policy/env DER+base64 (carried-forward lock), not the test key.

### verify_override API shape
- **D-02 (VerifiedOffline value type):** the offline verify returns an **immutable verified-grant value** (e.g. `VerifiedOffline` / `OverrideGrant`) carrying the already-parsed, already-checked scope/expiry/identity. Phase 93's live `POST /actions` AND-gate is a **separate step that consumes this value** — the token is never re-parsed or re-read between the offline and live checks (closes the TOCTOU verify→apply gap, PITFALLS #8). The AND-gate composition stays explicit: offline-pass is necessary but not sufficient; Phase 93 adds the live arm on top of this value.

### jti replay store
- **D-03 (Ephemeral in-process consumed-`jti` set):** Phase 91 ships an in-process set that rejects a second `verify_override()` of the same `jti` within the process (satisfies SC3 + VFY-06 for v1). **Cross-process / persistent replay protection is deferred** — the live ZT-Infra check (Phase 93) is the durable single-use enforcement point across processes, so a persistent local store is not required for v1. This boundary is intentional and must be stated in the phase plan so it is not mistaken for a gap.

### Error model
- **D-04 (One error + machine-readable kind enum):** a single `NonoOverrideError` carries a stable `kind` reason code — `BadSignature`, `Expired`, `NotYetValid`, `OutOfScope`, `Replay`, `AlgorithmMismatch`, `KeyNotAllowlisted`, `Parse`, `MissingField` (extend as verification surfaces) — plus a **redaction-safe** message (no raw secrets; paths per existing redaction policy). One PyO3 exception type, raised for **every** `Err` variant (SC4). The `kind` enum is the contract Phase 92 maps 1:1 to the REJECTED audit event (EventID 10008) without string-parsing messages.

### Claude's Discretion
Researcher/planner decide these with the fail-secure defaults below; no user input needed:
- **Canonical re-derivation mechanics:** re-derive CAF bytes from the **parsed struct** (sorted keys, strip `current_hash` + `kms_signature`, no whitespace, SHA-256, lowercase hex) per ZT-Infra `CANONICAL_FORM.md` / `canonical.js` — never hash raw received bytes (PITFALLS #3). Mirror ZT-Infra's `zt-verify` Python reference verifier where it clarifies intent.
- **serde parse strictness:** default to fail-secure (`deny_unknown_fields` unless a concrete ZT-Infra forward-compat need is found at plan time — flag it if so).
- **Module/type layout** inside `nono-py/src/override.rs`, `#[pyclass]` exposure of the grant type, and where the test pubkey/test keypair fixtures live in the test tree.
- **Exact `#[must_use]` placement** on the verification `Result` (SC5).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` § "Phase 91" — goal, 5 success criteria, requirement set (OVR-01/02/03, VFY-02..07).
- `.planning/REQUIREMENTS.md` — full OVR/VFY requirement text + the architecture invariant (two-key AND gate; additive-only; Rust core policy-free) + Out-of-Scope table.
- `.planning/research/SUMMARY.md` — HIGH-confidence synthesis; the milestone-level gap resolutions (KMS pubkey embed, scope/expiry inside payload, 8h/24h TTL, `jti` v1, EventIDs 10006–10010).

### Research detail (read for Phase 91 planning)
- `.planning/research/STACK.md` — crate decisions: reuse `verify_keyed_signature`; promote `aws-lc-rs`/`base64`/`chrono`; no KMS SDK; canonical bytes via `serde_json` + `sha2`.
- `.planning/research/ARCHITECTURE.md` — component breakdown (`override.rs` surface).
- `.planning/research/PITFALLS.md` — 15 pitfalls; Phase-91-critical: #1 fail-open, #2 algorithm-confusion/low-S, #3 canonicalization mismatch, #4 self-service/key-ARN allowlist, #5 path-scope escape, #8 TOCTOU verify→apply.

### nono in-tree (reuse, don't reinvent)
- `crates/nono/src/trust/bundle.rs:463` — **`verify_keyed_signature`** (ECDSA P-256, `aws-lc-rs`-backed) — the signature primitive to call. Re-exported via `crates/nono/src/trust/mod.rs`. (NOTE: research said `trust/signing.rs::verify_keyed_signature`; the actual definition is in `bundle.rs`.)
- `nono-py/src/lib.rs`, `nono-py/src/windows_confined_run.rs`, `nono-py/src/sandboxed_exec.rs` — existing `#[pyclass]`/PyO3 patterns and the `confined_run`/`confine` entry points Phase 92 will wire into.
- `CLAUDE.md` § "Library vs CLI Boundary" + § Security Considerations (path-handling footguns) — the policy-free-core invariant and the `starts_with` path footgun.

### External dependency — ZT-Infra v2 (`C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2`)
- `docs/CANONICAL_FORM.md` — CAF v0.1 canonical-form spec (sorted keys, stripped fields).
- `test-vectors/canonical-form/vectors.json` — **the SC1 reference vectors** `canonical_bytes()` is validated against.
- `provisioner/src/canonical.js` — `stableJson()` reference implementation.
- `zt-verify/` (Python reference verifier; `tests/test_canonical_vectors.py`) — reference CAF verification logic to mirror for intent.
- `provisioner/src/audit.js` — source of the `algorithm:"none"` local-fallback shape the Rust verifier must explicitly reject.
- `proj/POC-zt-infra-e5-local-provisioner.md` — the E5 composition POC runbook (where the nono↔ZT glue lives today).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `nono::trust::verify_keyed_signature` (`bundle.rs:463`) — ECDSA P-256 verification over `aws-lc-rs`; the signature check. Do **not** use the `sigstore-verify` keyless/OIDC path (different trust root).
- `serde_json` + `sha2` (workspace deps) — canonical-bytes re-derivation, ~60–80 LOC, no new crate.
- Existing `#[pyclass]` / PyO3 error-raising patterns in `nono-py/src/*.rs`.

### Established Patterns
- **Fail-secure error handling** (CLAUDE.md): `Result` + `?`; no `.unwrap()`/`.expect()` (clippy `unwrap_used` enforced); `#[must_use]` on critical Results; libraries don't panic on expected error conditions.
- **Path security** (CLAUDE.md): path-component comparison only — never string `starts_with`. Applies to scope checks here and in Phase 92.
- **Fork-extension audit pattern** (`AuditEventPayload`) — relevant context for Phase 92, not built here.

### Integration Points
- `override.rs` is consumed by `confined_run`/`confine` in Phase 92 (not wired this phase).
- The `VerifiedOffline` value (D-02) is the seam Phase 93's live AND-gate consumes.
- The `NonoOverrideError.kind` enum (D-04) is the contract Phase 92 maps to EventID 10008 (REJECTED).

</code_context>

<specifics>
## Specific Ideas

- "Reuse, don't reinvent" — the user (fork co-author) wants the existing `verify_keyed_signature` ECDSA primitive used, not a fresh crypto path, and the ZT-Infra cross-language canonical vectors honored verbatim.
- Test-only test keypair must never become a production trust anchor — keep the injection path test-gated.

</specifics>

<deferred>
## Deferred Ideas

- **Cross-process / persistent `jti` store** — deferred; live ZT-Infra check is the durable single-use point (D-03). Revisit only if a v1.x need surfaces.
- **Live `POST /actions` AND-gate, KMS pubkey distribution procedure, revocation, AWS cred stripping, `nono override request`, DAAL anchoring** — Phase 93.
- **`CapabilitySet` mutation + audit emission (EventIDs 10006–10010)** — Phase 92.
- **`nono-ts` binding parity** — FUT-03, future milestone.

</deferred>

---

*Phase: 91-signed-override-format-verification-core*
*Context gathered: 2026-06-21*
