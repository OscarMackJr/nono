# Phase 91: Signed Override Format + Verification Core - Research

**Researched:** 2026-06-21
**Domain:** Security-critical crypto verifier (ECDSA P-256 over CAF v0.1 canonical JSON) in a Rust/PyO3 binding (`nono-py`), with a cross-language canonicalization conformance guarantee and a live external spec dependency (ZT-Infra v2).
**Confidence:** HIGH — every load-bearing claim verified by reading the actual source in both repos (nono in-tree + ZT-Infra v2) and the sigstore-crypto crate source on disk.

## Summary

Phase 91 builds a fully offline, fail-closed verifier for ZT-Infra CAF v0.1 signed override tokens, living in a NEW module `nono-py/src/override.rs`. The work decomposes into five concrete, independently-testable pieces: (1) a CAF v0.1 `canonical_bytes()` re-derivation validated **verbatim** against ZT-Infra's shipped `test-vectors/canonical-form/vectors.json` (9 vectors, exact bytes + length + SHA-256); (2) a strict `OverrideToken` serde parse model; (3) the offline `verify_override()` pipeline (canonical → digest → ECDSA P-256 verify with explicit low-S + algorithm pin + key-ARN allowlist + expiry/not_before/skew + jti replay); (4) an immutable `OverrideGrant` value type (`#[pyclass]`) that closes the TOCTOU gap; and (5) `NonoOverrideError` (a custom PyO3 exception with a stable `kind` enum) raised for every `Err`.

**The single most important finding** is that the in-tree primitive named in the ROADMAP/CONTEXT — `nono::trust::verify_keyed_signature` (`bundle.rs:463`) — is **NOT** the right primitive. It verifies a **Sigstore DSSE/in-toto bundle** by computing PAE over an in-toto payload, not a raw ECDSA signature over a SHA-256 digest. ZT-Infra/AWS KMS signs the **raw 32-byte SHA-256 digest** of the canonical bytes (`MessageType=DIGEST`). The correct primitive already exists one layer down: `sigstore_verify::crypto::verification::VerificationKey::verify_prehashed(&Sha256Hash, &SignatureBytes)` (sigstore-crypto 0.8.0, on disk), which calls `aws_lc_rs ... verify_digest` over a P-256 SHA-256 ASN.1 signature — exactly the KMS DIGEST semantics. The planner must NOT call `verify_keyed_signature`; instead either (a) add a thin `nono::trust` re-export `verify_ecdsa_p256_digest(pubkey_der, digest, sig_der)` or (b) call `VerificationKey::verify_prehashed` directly from `override.rs` (sigstore-verify is a transitive dep of nono-py via the `nono` crate).

**The second most important finding** is that **low-S is NOT enforced by aws-lc-rs's ASN.1 ECDSA verify** — both ZT-Infra reference implementations (`audit.js::normalizeEcdsaDerLowS` on the signer side, `signatures.py::_require_low_s` on the verifier side) enforce it as an **explicit separate DER-parse check**. The Rust verifier MUST replicate `_require_low_s` (parse the DER `SEQUENCE { r, s }`, reject if `s > n/2`), or it fails VFY-02 and PITFALLS #2 (malleability).

**Primary recommendation:** Build canonical-form FIRST and prove it against the 9 shipped vectors before wiring any signature path (matches Success Criterion ordering). Reuse the shipped ZT-Infra Rust skeleton (`test-vectors/canonical-form/skeletons/rust/canonical.rs`) almost verbatim — it is a complete, spec-conformant, test-passing implementation. Then layer the verify pipeline on top, with explicit low-S, algorithm pin, ARN allowlist, expiry, and an in-process jti set, every branch returning `Err(NonoOverrideError{kind})`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01 (Hybrid fixtures):** Two fixture sources by purpose.
  - **Canonical form (SC1 / OVR-03):** consume ZT-Infra's `test-vectors/canonical-form/vectors.json` **verbatim** to prove `canonical_bytes()` produces matching SHA-256 digests — the cross-language guarantee OVR-03 requires; do not regenerate locally.
  - **Signature path (SC2 / VFY-02):** mint signed-token fixtures with a **committed local ECDSA P-256 test keypair** whose DER public key is injected **only in tests** (never a production trust root). Lets "valid token → `Ok`" be tested without coupling CI to AWS/KMS and without ever accepting `algorithm:"none"`.
  - Keep the test pubkey injection path test-only — production pubkey sourcing is embedded machine-policy/env DER+base64 (carried-forward lock), not the test key.
- **D-02 (VerifiedOffline value type):** the offline verify returns an **immutable verified-grant value** (e.g. `VerifiedOffline` / `OverrideGrant`) carrying the already-parsed, already-checked scope/expiry/identity. Phase 93's live `POST /actions` AND-gate is a **separate step that consumes this value** — the token is never re-parsed or re-read between offline and live checks (closes TOCTOU verify→apply, PITFALLS #8). AND-gate composition stays explicit: offline-pass is necessary but not sufficient.
- **D-03 (Ephemeral in-process consumed-`jti` set):** Phase 91 ships an in-process set that rejects a second `verify_override()` of the same `jti` within the process (satisfies SC3 + VFY-06 for v1). **Cross-process / persistent replay protection is deferred** — the live ZT-Infra check (Phase 93) is the durable single-use enforcement point across processes. This boundary is intentional and must be stated in the phase plan so it is not mistaken for a gap.
- **D-04 (One error + machine-readable kind enum):** a single `NonoOverrideError` carries a stable `kind` reason code — `BadSignature`, `Expired`, `NotYetValid`, `OutOfScope`, `Replay`, `AlgorithmMismatch`, `KeyNotAllowlisted`, `Parse`, `MissingField` (extend as verification surfaces) — plus a **redaction-safe** message (no raw secrets; paths per existing redaction policy). One PyO3 exception type, raised for **every** `Err` variant (SC4). The `kind` enum is the contract Phase 92 maps 1:1 to the REJECTED audit event (EventID 10008) without string-parsing messages.

### Claude's Discretion
- **Canonical re-derivation mechanics:** re-derive CAF bytes from the **parsed struct** (sorted keys, strip `current_hash` + `kms_signature`, no whitespace, SHA-256, lowercase hex) per ZT-Infra `CANONICAL_FORM.md` / `canonical.js` — never hash raw received bytes (PITFALLS #3). Mirror ZT-Infra's `zt-verify` Python reference verifier where it clarifies intent.
- **serde parse strictness:** default to fail-secure (`deny_unknown_fields` unless a concrete ZT-Infra forward-compat need is found at plan time — flag it if so).
- **Module/type layout** inside `nono-py/src/override.rs`, `#[pyclass]` exposure of the grant type, and where the test pubkey/test keypair fixtures live in the test tree.
- **Exact `#[must_use]` placement** on the verification `Result` (SC5).

### Deferred Ideas (OUT OF SCOPE)
- **Cross-process / persistent `jti` store** — deferred; live ZT-Infra check is the durable single-use point (D-03).
- **Live `POST /actions` AND-gate, KMS pubkey distribution procedure, revocation, AWS cred stripping, `nono override request`, DAAL anchoring** — Phase 93.
- **`CapabilitySet` mutation + audit emission (EventIDs 10006–10010)** — Phase 92.
- **`nono-ts` binding parity** — FUT-03, future milestone.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OVR-01 | A signed override token carries signer identity, scope (absolute fs paths + network domains), `not_before`, `expires_at`, repo-context binding, unique `jti`. | **GAP FLAGGED:** ZT-Infra's signed payload is a CAF v0.1 AuditRecord (`actor/action/resource/decision/reason/timestamp/previous_hash`), NOT an override token with these fields. The override-token field set is a **nono-side construct** Phase 91 must define + decide how OVR fields map into the signed CAF payload. See Open Question OQ-1. The serde model + `#[pyclass]` grant carries these fields. |
| OVR-02 | Scope and expiry embedded **inside** the KMS-signed payload (covered by signature), not unsigned wrapper metadata. | Canonical re-derivation strips only `current_hash` + `kms_signature` (R10); every other field is covered by the signature. Any OVR field placed in the canonicalized object IS signature-covered. The verifier MUST reject tokens where scope/expiry live outside the signed object. |
| OVR-03 | CAF v0.1 canonical form; canonical bytes **re-derived from parsed structure** (sorted keys, signature/hash fields stripped), validated against `test-vectors/canonical-form/vectors.json`. | FULLY RESOLVED. Spec = `CANONICAL_FORM.md` (R1–R12). Reference impls read. 9 verbatim vectors. Shipped Rust skeleton is spec-conformant. |
| VFY-02 | Signature verification reuses existing ECDSA P-256 primitive; pins `ECDSA_SHA_256`, enforces low-S, rejects `algorithm:"none"` and any non-pinned algorithm. | `verify_keyed_signature` is WRONG (DSSE/PAE). Use `VerificationKey::verify_prehashed` (sigstore-crypto 0.8.0). **Low-S NOT auto-enforced** — must add explicit `_require_low_s` DER check. `algorithm:"none"` shape pinned from `audit.js:181`. |
| VFY-03 | KMS public key sourced from embedded machine policy / env (DER+base64), cached per `key_id`; no in-process AWS SDK, no AWS creds. | Phase 91 = offline only. Pubkey passed in as DER bytes param (test-injected per D-01). DER parse: `VerificationKey::from_spki(&DerPublicKey, EcdsaP256Sha256)`. |
| VFY-04 | Signer `key_id` checked against machine-policy allowlist of approved signing-key ARNs — crypto validity alone does not authorize. | `key_id` lives in `kms_signature.key_id` (ARN form: `arn:aws:kms:us-east-2:111122223333:key/demo`). Allowlist param to `verify_override`; exact-match (not substring). PITFALLS #4. |
| VFY-05 | `expires_at`/`not_before` enforced with 2-min skew; max TTL hard-capped (8h dev / 24h CI), operator-configurable downward. | `chrono` RFC3339 parse + `Utc::now()` compare. Skew ±120s. TTL cap = `expires_at - not_before <= cap`. Timestamps are CAF-validated `\d{4}-...Z` millisecond ISO-8601 (see canonical.py TIMESTAMP_RE). |
| VFY-06 | Each token `jti` single-use — consumed-`jti` store rejects replay. | D-03 in-process `HashSet` / `Mutex<HashSet>`. Insert-on-success; reject second verify of same jti even before expiry (SC3). |
| VFY-07 | Fail-closed: every failure yields no expansion, runs nothing. Verify returns `Result` (`#[must_use]`); PyO3 raises `NonoOverrideError`, never falsy. | `#[must_use]` on `verify_override` return. Every `Err` → `NonoOverrideError`. No `unwrap_or`, no `Option`-with-deny. PITFALLS #1. SC4/SC5. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CAF canonical-bytes re-derivation | nono-py Rust (`override.rs`) | — | Pure function; security-critical; type-safe in Rust; reuses ZT skeleton. |
| ECDSA P-256 digest verify | nono crate `trust` (thin reuse) / sigstore-crypto | nono-py `override.rs` | Crypto primitive lives at the lib boundary; nono-py calls it. Do NOT add a 2nd crypto backend. |
| Low-S enforcement | nono-py Rust (`override.rs`) | — | aws-lc-rs ASN.1 verify does not enforce; explicit DER check required. |
| Token parse / field model | nono-py Rust (`override.rs`) | — | serde struct; `deny_unknown_fields`. |
| Expiry / skew / TTL cap | nono-py Rust (`override.rs`) | — | Security invariant belongs in the verifier (fail-closed). |
| Key-ARN allowlist | nono-py Rust (`override.rs`) | — | Authorization (not just crypto) check. |
| jti replay set | nono-py Rust (`override.rs`) | — | In-process state (D-03). |
| `OverrideGrant` value + PyO3 exception | nono-py Rust (`override.rs` + `lib.rs` registration) | — | The PyO3 boundary surface Phase 92/93 consume. |
| Live `POST /actions` AND-gate | (Phase 93, Python `urllib`) | — | OUT OF SCOPE this phase. |
| `CapabilitySet` mutation + audit | (Phase 92) | — | OUT OF SCOPE this phase. |

**Note on the policy-free-core invariant (CLAUDE.md):** All override *policy* (allowlist, scope, expiry, jti) lives in **nono-py**, not the `nono` core crate. The only thing that may touch the `nono` core is an optional thin ECDSA-digest verify re-export (a crypto primitive, not policy) — and even that can be avoided by calling sigstore-crypto directly from `override.rs`. Confirm the chosen path keeps the core policy-free.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `sigstore-crypto` (via `sigstore-verify`) | 0.8.0 | `VerificationKey::verify_prehashed` → ECDSA P-256 over the 32-byte digest (KMS DIGEST mode). `from_spki` parses DER SPKI pubkey. `SignatureBytes::from_base64`. | `[VERIFIED: on-disk crate source]` Already a transitive dep of nono-py (via `nono` crate). Backed by `aws-lc-rs`. Exactly matches KMS `MessageType=DIGEST`. |
| `sha2` | 0.10/0.11 (both in lock) | SHA-256 of canonical bytes → 32-byte digest. | `[VERIFIED: nono Cargo.toml + nono-py Cargo.lock]` Workspace dep; ZT Rust skeleton uses sha2 0.10. |
| `serde` / `serde_json` | 1 | `OverrideToken` deserialize; `serde_json::Value` walk for canonical re-derivation. | `[VERIFIED: nono-py Cargo.toml]` Already direct deps in nono-py. |
| `pyo3` | 0.28 | `#[pyclass] OverrideGrant`, `create_exception!` for `NonoOverrideError`, `#[pyfunction]`. | `[VERIFIED: nono-py Cargo.toml]` Already direct. |

### Supporting (promote to direct deps in `nono-py/Cargo.toml`)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chrono` | 0.4.45 (in lock) | RFC3339 `expires_at`/`not_before` parse + `Utc::now()` compare + skew/TTL math. | `[VERIFIED: nono-py Cargo.lock]` Transitive today; promote to direct with `features=["serde"]` (or `["clock"]` for `Utc::now`). |
| `base64` | 0.22.1 (in lock) | Decode `kms_signature.signature` (base64 DER) and the env/policy DER pubkey. | `[VERIFIED: nono-py Cargo.lock]` Transitive today; promote to direct. Use the 0.22 Engine API. NOTE: `SignatureBytes::from_base64` already exists in sigstore-types — may avoid a direct `base64` dep for the signature field; still likely needed for the pubkey DER. |
| `aws-lc-rs` | 1.17.0 (in lock) | Only if calling aws-lc-rs directly for low-S/verify rather than via sigstore-crypto. | `[VERIFIED: nono-py Cargo.lock]` Transitive. Prefer going through sigstore-crypto's `VerificationKey` to avoid a parallel crypto path; promote to direct ONLY if you need raw `signature::UnparsedPublicKey`. |
| `der` | 0.7 | Parse the ECDSA DER `SEQUENCE { INTEGER r, INTEGER s }` for the explicit low-S check. | `[VERIFIED: nono Cargo.toml]` Already in `nono`; NOT yet in nono-py direct deps. Alternative: hand-roll the ~30-line DER integer parse like `audit.js`/`_require_low_s` (no new dep). Decide at plan time. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `VerificationKey::verify_prehashed` | `nono::trust::verify_keyed_signature` | **REJECTED** — it is a Sigstore DSSE/PAE verifier, not raw ECDSA-over-digest. Would never verify a KMS signature. |
| sigstore-crypto `VerificationKey` | direct `aws_lc_rs::signature::UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1)` + `verify_digest` | Both work (sigstore-crypto wraps exactly this). Going through `VerificationKey` reuses an in-tree-blessed surface and avoids promoting aws-lc-rs to a direct nono-py dep. |
| `der` crate for low-S | hand-rolled DER `r,s` parse | Hand-roll mirrors the audited `audit.js`/`_require_low_s` exactly, zero new dep, ~30 LOC; `der` crate is cleaner but a new direct dep. Either acceptable. |
| custom canonical walk | ZT-Infra shipped Rust skeleton (`skeletons/rust/canonical.rs`) | **USE THE SKELETON** — it is complete, spec-conformant, and passes all vectors. Adapt error type to `NonoOverrideError`. |

**Installation (no new crates — all already in nono-py's lock; promotions only):**
```toml
# nono-py/Cargo.toml [dependencies] — promote transitive → direct as the module needs:
chrono  = { version = "0.4", features = ["serde", "clock"] }
base64  = "0.22"
# der  = "0.7"   # only if not hand-rolling the low-S DER parse
# sha2 = "0.10"  # if not already reachable as a direct dep; verify at plan time
```

**Version verification (done this session):** all four candidate deps confirmed present in `C:\Users\OMack\nono-py\Cargo.lock` at the versions above. No `cargo add` of a net-new crate is required. `[VERIFIED: nono-py Cargo.lock 2026-06-21]`

## Package Legitimacy Audit

> Every package this phase uses is **already resolved in `nono-py/Cargo.lock`** as a transitive or direct dependency of the existing, audited build. No new package is introduced from an external/untrusted source; this phase only **promotes** already-present transitive deps to direct status. slopcheck/registry-hallucination risk is therefore **not applicable** — there is no new name to slop-check.

| Package | Registry | In nono-py lock | Source | Disposition |
|---------|----------|-----------------|--------|-------------|
| `sigstore-crypto` | crates.io | 0.8.0 (transitive via nono→sigstore-verify) | sigstore-rust (read on disk) | Approved (reuse) |
| `sha2` | crates.io | 0.10.9 + 0.11.0 | RustCrypto | Approved (workspace) |
| `serde` / `serde_json` | crates.io | 1.x | serde-rs | Approved (direct already) |
| `pyo3` | crates.io | 0.28 | PyO3 | Approved (direct already) |
| `chrono` | crates.io | 0.4.45 | chronotope/chrono | Approved — promote to direct |
| `base64` | crates.io | 0.22.1 | marshallpierce/rust-base64 | Approved — promote to direct |
| `aws-lc-rs` | crates.io | 1.17.0 | aws/aws-lc-rs | Approved — direct only if needed |
| `der` | crates.io | 0.7 (in nono) | RustCrypto/formats | Approved — direct only if used |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────────────────────┐
 signed override    │  nono-py/src/override.rs  ::  verify_override()  [#[must_use]]│
 token JSON (str) ──▶│                                                              │
 + pubkey DER bytes │  ① serde parse → OverrideToken  (deny_unknown_fields)         │
 + key-ARN allowlist│        │  parse error / missing field → Err{kind:Parse|MissingField}
 + repo context     │        ▼                                                      │
                    │  ② canonical_bytes(token)   (re-derive from STRUCT, not raw)  │
                    │        strip current_hash + kms_signature (R10), sort keys,   │
                    │        no whitespace, reject floats/null/bool/ctrl/supra-BMP  │
                    │        │                                                      │
                    │        ▼  SHA-256 → 32-byte digest                            │
                    │  ③ algorithm pin: kms_signature.algorithm == "ECDSA_SHA_256"  │
                    │        else → Err{kind:AlgorithmMismatch}   ("none" rejected) │
                    │  ④ key-ARN allowlist: kms_signature.key_id ∈ allowlist        │
                    │        else → Err{kind:KeyNotAllowlisted}                     │
                    │  ⑤ base64-decode signature → DER bytes                        │
                    │  ⑥ low-S check: parse DER {r,s}; s > n/2 → Err{kind:BadSignature}
                    │  ⑦ VerificationKey::from_spki(pubkey_der, EcdsaP256Sha256)    │
                    │        .verify_prehashed(digest, sig) else → Err{BadSignature}│
                    │  ⑧ time: not_before-120s ≤ now ≤ expires_at+120s;             │
                    │        expires_at-not_before ≤ TTL cap                        │
                    │        → Err{kind:NotYetValid|Expired}                        │
                    │  ⑨ jti replay: consumed_set.contains(jti) → Err{kind:Replay}  │
                    │        else insert(jti)                                       │
                    │        │                                                      │
                    │        ▼                                                      │
                    │  Ok(OverrideGrant{ signer, scope_paths, scope_domains,        │
                    │        not_before, expires_at, jti, repo_context })  [pyclass,│
                    │        frozen/immutable — the D-02 TOCTOU-safe value]         │
                    └────────────────────────────┬────────────────────────────────┘
                                                 │  consumed by (later phases):
                                                 ├─▶ Phase 92: additive CapabilitySet mutation + audit (EventID 10006/10008)
                                                 └─▶ Phase 93: live POST /actions AND-gate (necessary-but-not-sufficient)

   Every Err → to_py_err → raise NonoOverrideError(kind=..., message=<redacted>)   (SC4)
   No path returns Ok-with-deny-flag, None, or False.                              (PITFALLS #1)
```

### Recommended Project Structure
```
nono-py/src/
├── override.rs        # NEW: OverrideToken, canonical_bytes, verify_override, OverrideGrant, OverrideErrorKind
├── lib.rs             # MODIFY: create_exception! NonoOverrideError; m.add(...); m.add_class::<OverrideGrant>(); m.add_function(verify_override)
nono-py/tests/
├── test_override_canonical.py   # NEW: consume ../../ZeroTrust2/.../vectors.json verbatim (or a copied snapshot)
├── test_override_verify.py      # NEW: valid-token Ok + every Err variant; jti replay; algorithm:"none" reject
├── fixtures/
│   ├── override_test_key.pem/.der   # NEW: committed ECDSA P-256 TEST keypair (test-only; D-01)
│   └── vectors.json                  # OPTIONAL: snapshot copy of ZT vectors (see OQ-2)
```
**Test-vector sourcing decision (OQ-2):** ZT-Infra's `tests/test_canonical_vectors.py` reads `vectors.json` via a relative path two parents up — assumes a co-located monorepo. nono-py is a separate repo. Plan must decide: (a) commit a **snapshot copy** of `vectors.json` into `nono-py/tests/fixtures/` (deterministic, repo-isolated, but can drift if ZT bumps CAF), or (b) read from `$ZT_INFRA_REPO/test-vectors/...` via env var (always fresh, but CI-host-gated). **Recommend (a) snapshot copy** with a comment recording the source commit + a `verify-dark` gate (Phase 92 DF-01) that can re-diff against the live ZT repo when present. The 9 vectors are tiny (~250 lines).

### Pattern 1: Re-derive-from-struct canonicalization (NOT hash-the-bytes)
**What:** Parse JSON → object → strip `current_hash`+`kms_signature` → emit CAF canonical bytes → SHA-256. NEVER `sha256(received_raw_bytes)`.
**When to use:** Always, for OVR-03 / PITFALLS #3. Received bytes have arbitrary key order/whitespace; only the canonical re-derivation matches the signer's digest.
**Example (adapted from ZT-Infra shipped Rust skeleton — spec-conformant, passes all 9 vectors):**
```rust
// Source: C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\test-vectors\canonical-form\skeletons\rust\canonical.rs
// (read 2026-06-21; reproduce nearly verbatim, swap Error→OverrideErrorKind)
use serde_json::Value; use sha2::{Digest, Sha256}; use std::collections::BTreeMap;

pub fn canonicalize(value: &Value, strip: &[&str]) -> Result<Vec<u8>, OverrideErrorKind> {
    // strip top-level fields, then write_value (objects use BTreeMap = R3 code-point sort)
    // write_string: escape only " and \ ; emit / literally (R7.3); reject ctrl/U+2028/U+2029/supra-BMP (R7.4)
    // reject Null/Bool leaf; reject f64 numbers; integers must be in [0, 2^53-1]
}
pub fn canonical_hash(value: &Value, strip: &[&str]) -> Result<String, OverrideErrorKind> {
    let bytes = canonicalize(value, strip)?;
    Ok(hex::encode(Sha256::digest(&bytes)))   // lowercase hex, 64 chars, no 0x
}
// For the override: strip = &["current_hash", "kms_signature"]   (R10)
```

### Pattern 2: ECDSA P-256 verify over the prehashed 32-byte digest (KMS DIGEST mode)
**What:** Verify the base64-DER signature against the 32-byte SHA-256 digest, NOT against the message.
**Example:**
```rust
// Source: sigstore-crypto 0.8.0 src/verification.rs (on disk) + bundle.rs:468-497 usage pattern
use sigstore_verify::crypto::signing::SigningScheme;
use sigstore_verify::crypto::verification::VerificationKey;
use sigstore_verify::types::{SignatureBytes, DerPublicKey};
use sigstore_verify::types::Sha256Hash; // typed 32-byte digest

let pub_key = DerPublicKey::from(pubkey_der.to_vec());
let vk = VerificationKey::from_spki(&pub_key, SigningScheme::EcdsaP256Sha256)
    .map_err(|_| OverrideErrorKind::BadSignature)?;            // bad/non-P256 pubkey
let sig = SignatureBytes::from_base64(&sig_b64)
    .map_err(|_| OverrideErrorKind::BadSignature)?;
let digest = Sha256Hash::from_bytes(<[u8;32]>::try_from(&sha256_bytes[..])?);
vk.verify_prehashed(&digest, &sig)                              // aws-lc-rs verify_digest
    .map_err(|_| OverrideErrorKind::BadSignature)?;
```
> `verify_prehashed` (sigstore-crypto 0.8.0) calls `aws_lc_rs::signature::UnparsedPublicKey::verify_digest` with `ECDSA_P256_SHA256_ASN1`. `verify` (non-prehashed) hashes the message internally and would be WRONG for KMS DIGEST mode.

### Pattern 3: Explicit low-S enforcement (aws-lc-rs does NOT do this)
**What:** Parse the DER `SEQUENCE { INTEGER r, INTEGER s }`; reject if `s > P256_ORDER/2`.
**Example:**
```rust
// Source: zt-verify/src/zt_verify/signatures.py::_require_low_s + audit.js::normalizeEcdsaDerLowS
const P256_ORDER_HALF: /* big int */ = 0x7fffffff800000007fffffffffffffffde737d56d38bcf4279dce5617e3192a8;
fn require_low_s(sig_der: &[u8]) -> Result<(), OverrideErrorKind> {
    let (_r, s) = decode_ecdsa_der(sig_der).map_err(|_| OverrideErrorKind::BadSignature)?;
    if s > P256_ORDER_HALF { return Err(OverrideErrorKind::BadSignature); }
    Ok(())
}
```
> Run this BEFORE or alongside `verify_prehashed`. The Python reference returns `MALFORMED` for high-S; map to `BadSignature` (or a distinct kind if Phase 92 needs it).

### Pattern 4: Custom PyO3 exception with a machine-readable kind
**What:** `create_exception!` macro defines `NonoOverrideError`; the `kind` is exposed as an attribute or message prefix Phase 92 maps to EventID 10008.
**Example:**
```rust
// Source: PyO3 0.28 create_exception! pattern (existing nono-py uses built-in exceptions via to_py_err)
use pyo3::create_exception;
create_exception!(_nono_py, NonoOverrideError, pyo3::exceptions::PyException);
// in #[pymodule]: m.add("NonoOverrideError", py.get_type::<NonoOverrideError>())?;
fn override_err_to_py(kind: OverrideErrorKind, msg: String) -> PyErr {
    // kind carried in a structured form Phase 92 reads without string-parsing.
    // Option A: NonoOverrideError with .kind attr (needs a #[pyclass] error or args tuple).
    // Option B (simplest): NonoOverrideError::new_err((kind_str, redacted_msg)) — args[0] = stable kind.
    NonoOverrideError::new_err((kind.as_str().to_string(), msg))
}
```
> NOTE: existing nono-py error mapping (`lib.rs::to_py_err`) maps `NonoError` variants to **built-in** exceptions; it does NOT use `create_exception!`. Phase 91 introduces the first custom exception — confirm the registration line in `#[pymodule] _nono_py` and that PyO3 0.28's `create_exception!` + `m.add` signature is current (verify against PyO3 0.28 docs at plan time; the macro is stable across 0.2x).

### Anti-Patterns to Avoid
- **Calling `verify_keyed_signature`** — it is the Sigstore DSSE path; will never verify a KMS digest signature.
- **`vk.verify(message, sig)` instead of `vk.verify_prehashed(digest, sig)`** — double-hashes; signature never matches.
- **Trusting `record.current_hash` as the digest** — recompute from canonical bytes (the Python reference explicitly refuses to trust `current_hash`; see `signatures.py` docstring).
- **`serde_json::to_string()` for canonical bytes** — key order is insertion-order/non-deterministic; violates R3. Must walk with sorted keys.
- **String `starts_with` for scope** (PITFALLS #5 / CLAUDE.md footgun #1) — scope checks are mostly Phase 92, but if any scope normalization happens here, use path-component comparison on canonicalized absolute paths.
- **`unwrap_or_default()` on a parsed `Option<scope>`** (PITFALLS #1) — empty default = permissive = fail-open.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CAF v0.1 canonical serialization | A bespoke JSON canonicalizer from scratch | ZT-Infra shipped Rust skeleton `skeletons/rust/canonical.rs` (adapt error type) | It is complete, spec-conformant, and passes all 9 vectors; rewriting risks a byte-mismatch that silently breaks signature verification. |
| ECDSA P-256 digest verification | A new ECDSA impl, `p256`/`ring`/`openssl` | `sigstore-crypto` `VerificationKey::verify_prehashed` (already in tree, aws-lc-rs-backed) | A 2nd crypto backend conflicts with the existing aws-lc-rs posture; the right primitive already exists. |
| DER signature decode for low-S | A full ASN.1 library you wire from scratch | `der` 0.7 (already in nono) OR the ~30-line integer parse mirrored from `audit.js`/`_require_low_s` | The reference impl is small and audited; either reuse path is fine. |
| RFC3339 timestamp parse | Manual string slicing | `chrono::DateTime::parse_from_rfc3339` | Skew/TTL math + tz correctness; chrono already in lock. |

**Key insight:** This phase's correctness is defined by **byte-identical agreement with an external reference implementation**. Hand-rolling any of the canonical/crypto pieces reintroduces exactly the cross-language interop bugs CAF v0.1 was written to eliminate. Reuse the shipped skeleton + the in-tree crypto primitive.

## Common Pitfalls

### Pitfall 1: Fail-OPEN on any error path (the cardinal sin)
**What goes wrong:** Any error (parse fail, missing field, bad pubkey, expired) silently grants instead of denies.
**Why it happens:** `match { Ok=>grant, Err(_)=>grant }` reflex; `unwrap_or_default()`; bare PyO3 `except` returning truthy.
**How to avoid:** Every `Err` → `NonoOverrideError`; `#[must_use]` on the result; no `unwrap_or*`; write a deny-asserting test for EVERY variant FIRST.
**Warning signs:** verify returns `bool`/`Option`; `if let Ok(..)` with no `else`; tests that assert "doesn't crash" rather than "returns deny".

### Pitfall 2: Algorithm confusion + low-S malleability
**What goes wrong:** Accepting `algorithm:"none"` (the local-fallback shape) or a high-S signature.
**Why it happens:** aws-lc-rs ASN.1 verify accepts high-S; `audit.js` emits `{algorithm:"none",key_id:"",signature:""}` when no KMS key is configured.
**How to avoid:** Pin `algorithm == "ECDSA_SHA_256"` BEFORE verify (reject `"none"` and empty signature explicitly); add explicit low-S DER check.
**Warning signs:** no algorithm equality check; no `_require_low_s` equivalent; tests don't include a high-S and an `algorithm:"none"` vector.

### Pitfall 3: Canonicalization mismatch (hash-the-raw-bytes)
**What goes wrong:** `sha256(received_json_bytes)` instead of `sha256(canonical_bytes(parsed))` → digest never matches the signer's.
**How to avoid:** Re-derive from the parsed struct, strip `current_hash`+`kms_signature`, sorted keys, no whitespace. Validate against the 9 vectors before wiring signatures (SC1 ordering).
**Warning signs:** `serde_json::to_string` feeding the hasher; vectors test absent or run after the signature test.

### Pitfall 4: Self-service / key-ARN not enforced
**What goes wrong:** Valid signature from an unauthorized key is accepted (crypto-valid ≠ authorized).
**How to avoid:** Exact-match `kms_signature.key_id` against the ARN allowlist param; reject otherwise. Never substring-match an ARN.
**Warning signs:** only "is the signature valid?" is checked; allowlist absent; `key_id.contains(...)`.

### Pitfall 5: Path-scope escape via string comparison
**What goes wrong:** `/tmp/project` scope matches `/tmp/project-evil`.
**How to avoid:** Path-component comparison on canonicalized absolute paths (mostly Phase 92, but if scope is normalized/validated here, do it right). Reject non-absolute scope paths in the token.
**Warning signs:** `str.starts_with(scope)`; non-canonicalized scope stored in the grant.

### Pitfall 6: TOCTOU verify→apply
**What goes wrong:** Token re-parsed/re-read between offline verify and live (Phase 93) check; a swapped token slips through.
**How to avoid (D-02):** Return an immutable `OverrideGrant` carrying the already-checked fields; downstream consumes the value, never the raw token. Make `OverrideGrant` `#[pyclass(frozen)]`.

### Pitfall 7: edition / MSRV mismatch between nono-py and nono
**What goes wrong:** nono-py is `edition = "2024"`, `rust-version = "1.95"`; the `nono` workspace is edition 2021 / Rust 1.82. Adding a `nono`-crate re-export must compile under BOTH toolchains.
**How to avoid:** If you add a `verify_ecdsa_p256_digest` re-export in `nono::trust`, keep it edition-2021-clean (no 2024-only syntax). Prefer calling sigstore-crypto directly from `override.rs` to avoid touching the core crate at all.
**Warning signs:** `make ci` (nono workspace, 1.82) fails after a core edit that compiled fine in nono-py.

## Runtime State Inventory

> Greenfield-in-a-new-module phase (no rename/refactor/migration of existing stored state). The only "state" introduced is the **in-process** jti consumed-set (D-03), which is ephemeral by design and intentionally NOT persisted. No databases, OS registrations, secrets, or build artifacts carry a renamed string.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — the jti set is in-process only (D-03); no DB/datastore touched. | None |
| Live service config | None this phase (live `POST /actions` is Phase 93). | None |
| OS-registered state | None. | None |
| Secrets/env vars | Production pubkey sourcing is `DER+base64` from machine policy/env — but Phase 91 only *accepts a pubkey param*; no env var is read this phase (test injects the key, D-01). The env-var name is a Phase 93 concern. | None this phase (flag the name for Phase 93) |
| Build artifacts | `nono-py` must be rebuilt via `maturin develop --release` after adding the module + new `#[pyclass]`/exception (stale `_nono_py` extension otherwise won't expose `NonoOverrideError`). | `maturin develop --release` in the test workflow |

## Code Examples

### Verbatim test-vector conformance (Python side, the SC1 proof)
```python
# Source: adapted from ZT-Infra zt-verify/tests/test_canonical_vectors.py (read 2026-06-21)
import hashlib, json
from pathlib import Path
import _nono_py   # the built extension exposing override canonical helpers (or test via a Rust #[cfg(test)])

VECTORS = json.loads((Path(__file__).parent / "fixtures" / "vectors.json").read_text())

def test_canonical_bytes_match_zt_vectors():
    for v in VECTORS:
        # canonical_bytes() must reproduce v["canonical_bytes_utf8"] exactly, and its
        # sha256 must equal v["sha256_hex"], after stripping v["strip_fields"].
        digest = _nono_py.override_canonical_sha256(json.dumps(v["input"]), v["strip_fields"])
        assert digest == v["sha256_hex"], v["name"]
```
> The 9 vectors include: TV-01 minimal, TV-02 BMP-unicode (no `\uXXXX` escapes), TV-03 escape table (`"`,`\`,literal `/`), TV-04 chain linkage, TV-05/08/09 registry entries (strip `signature`), TV-06 key-order invariance, TV-07 v0.2 key_id-stripped. **Override tokens use the AuditRecord strip set `["current_hash","kms_signature"]`** (TV-01/02/03/04/07 shape), not the registry `["signature"]` set — but the canonicalizer must handle both correctly, so test all 9.

### The `algorithm:"none"` shape that MUST be rejected
```js
// Source: ZT-Infra provisioner/src/audit.js:181 (read 2026-06-21) — local-fallback when no KMS key configured
return { algorithm: "none", key_id: "", signature: "" };
```
> Rust verifier must reject this at step ③ (algorithm != "ECDSA_SHA_256") AND step ⑤ (empty signature). The Python reference's `KmsSignature` validator also rejects empty signature and non-`ECDSA_SHA_256` algorithm (`canonical.py:70-95`).

### The signed-payload field set (AuditRecord — the ACTUAL signed object)
```
actor, action, resource (default ""), decision, reason, timestamp (ISO-8601 millis Z),
previous_hash (64 lc hex), current_hash (stripped), kms_signature{algorithm,key_id?,signature} (stripped)
```
> Source: `zt-verify/src/zt_verify/canonical.py` (`UnsignedAuditRecord` + `AuditRecord`), `vectors.json`, `server.js` POST /actions response. **`key_id` is `Optional` in the Python model** — but VFY-04 requires it for the ARN allowlist, so the override verifier should require a non-null `key_id` (fail-closed) unless OQ-1 resolves otherwise.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ROADMAP/research said primitive is `trust/signing.rs::verify_keyed_signature` | Actual def is `bundle.rs:463`, and it is the WRONG primitive (DSSE/PAE, not raw-digest ECDSA) | Discovered this session | Planner must use `VerificationKey::verify_prehashed`, not `verify_keyed_signature`. |
| "low-S enforced by the verify primitive" (assumption) | aws-lc-rs ASN.1 verify does NOT enforce low-S; both ZT reference impls add an explicit check | Confirmed from ZT source | Phase 91 MUST add an explicit `_require_low_s` DER parse. |
| serde forward-compat unknown fields | CAF v0.1 + ZT models are `extra="forbid"` / `deny_unknown_fields` (`canonical.py` ConfigDict, R10 "no additional fields") | CAF v0.1 spec is strict | `deny_unknown_fields` is the spec-correct, fail-secure default (resolves the OQ on serde strictness). |

**Deprecated/outdated:**
- The CONTEXT/ROADMAP reference to `verify_keyed_signature` as the signature primitive — superseded (see above). It remains correct that the ECDSA P-256 / aws-lc-rs *stack* is reused; just via a different function.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The override token's signed payload is CAF-AuditRecord-shaped (strip `current_hash`+`kms_signature`), and OVR-01 override fields (scope/not_before/expires_at/jti/repo_context) must be carried *inside* that signed object by a nono-defined convention — because ZT-Infra defines NO override-token schema (only AuditRecord + registry entry). | Phase Requirements / OQ-1 | If ZT-Infra actually has (or will have) a distinct override-token canonical shape, the strip-set and field model are wrong and signatures won't verify. **MUST confirm with the ZT-Infra operator / SEED-005 before locking the serde model.** |
| A2 | PyO3 0.28 `create_exception!` + `m.add(name, py.get_type::<E>())` is the current registration idiom (the macro has been stable across 0.2x). | Pattern 4 | Low — verify against PyO3 0.28 changelog at plan time; if changed, adjust the one registration line. |
| A3 | Committing a snapshot of `vectors.json` into nono-py is acceptable (vs. reading the live ZT repo). | Project Structure / OQ-2 | Low — snapshot can drift if ZT bumps CAF; mitigated by a Phase-92 verify-dark re-diff gate. |
| A4 | The production pubkey env-var name and the operator-config TTL-downward mechanism are Phase 93 concerns; Phase 91 only accepts a pubkey param + a hard-coded cap. | VFY-03/VFY-05 | Low — consistent with phase boundary; flag if the planner wants the env read here. |
| A5 | `der` 0.7 (in nono) is reachable from nono-py, OR a hand-rolled DER `r,s` parse is acceptable for low-S. | Stack / Pattern 3 | Low — both verified viable; pick at plan time. |

## Open Questions (RESOLVED)

1. **OQ-1 (BLOCKING the serde model): What is the exact override-token wire shape?**
   - **RESOLVED: D-05/D-06 (operator-confirmed, supersede the open question).**
   - What we know: ZT-Infra's `POST /actions` returns a signed CAF **AuditRecord** (`actor/action/resource/decision/reason/timestamp/previous_hash` + `kms_signature`). There is **no** override-token schema anywhere in ZT-Infra (grepped `provisioner/`, `policies/`, `docs/` for `jti`/`not_before`/`expires_at`/`scope`/`override` — only AuditRecord + registry entries exist; `override` appears only in `package.json`).
   - What's unclear: How OVR-01's fields (scope, not_before, expires_at, jti, repo_context, signer identity) are carried. Most likely they are encoded into the AuditRecord's `resource`/`action`/`reason` (e.g., `resource` = a JSON or path-list, `action` = `nono.override.grant`), OR the override is a nono-defined envelope `{ audit_record: <signed CAF record>, ...nono fields }` where only the inner record is signature-covered (which would violate OVR-02 for the nono fields).
   - Recommendation: **Confirm with the ZT-Infra operator / SEED-005 author before locking the serde struct.** Fail-secure interim: define the token so EVERY OVR-01 field lives inside the canonicalized (signature-covered) object (satisfies OVR-02), and reject any field outside it. Flag this as the one decision that, if wrong, breaks signature verification end-to-end.

2. **OQ-2 (non-blocking): vectors.json sourcing — snapshot copy vs. live-repo read?**
   - **RESOLVED: D-01 snapshot vectors.json into nono-py test tree.**
   - Recommendation: snapshot copy into `nono-py/tests/fixtures/vectors.json` with a source-commit comment; add a Phase-92 verify-dark gate to re-diff. (See Project Structure.)

3. **OQ-3 (non-blocking): one custom exception with `kind` as an attribute vs. as `args[0]`?**
   - **RESOLVED: D-04 / plan 03 (kind via `create_exception!` args[0]).**
   - D-04 wants a machine-readable `kind`. Simplest PyO3-idiomatic form: `NonoOverrideError.new_err((kind_str, message))` so `e.args[0]` is the stable kind. A `.kind` attribute requires a `#[pyclass]` exception subclass (heavier). Recommend `args[0]` unless Phase 92 prefers an attribute.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (nono-py) | building `override.rs` | ✓ (assumed dev host) | edition 2024 / 1.95 (nono-py) | — |
| `maturin` | building the `_nono_py` extension | ✓ (POC/prior phases used it) | — | — |
| ZT-Infra v2 repo | `vectors.json` + spec verbatim | ✓ | `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2` | snapshot vectors into nono-py |
| sigstore-crypto / aws-lc-rs / chrono / base64 | all in nono-py Cargo.lock | ✓ | 0.8.0 / 1.17.0 / 0.4.45 / 0.22.1 | — (no net-new crate) |
| AWS / KMS | NOT needed (offline phase) | n/a | — | committed test keypair (D-01) |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** ZT-Infra repo (snapshot vectors if absent in CI).

## Validation Architecture

> nyquist_validation = true (`.planning/config.json`). Strata below map each Success Criterion to a concrete, automated validation point.

### Test Framework
| Property | Value |
|----------|-------|
| Framework (Python) | `pytest` (already in nono-py dev deps; `nono-py/tests/` + `conftest.py` present) |
| Framework (Rust unit) | Rust built-in `#[cfg(test)]` (canonical-form vectors testable in-crate, mirroring ZT skeleton) |
| Config file | `nono-py/pyproject.toml` (pytest) ; no custom rustfmt/clippy override beyond nono workspace |
| Quick run command | `cd nono-py && maturin develop --release && python -m pytest tests/test_override_canonical.py tests/test_override_verify.py -x` |
| Full suite command | `cd nono-py && maturin develop --release && python -m pytest -x` ; plus `cargo test` in nono-py for in-crate canonical tests |

### Phase Requirements → Test Map
| Req / SC | Behavior | Test Type | Automated Command | File Exists? |
|----------|----------|-----------|-------------------|-------------|
| OVR-03 / SC1 | `canonical_bytes` matches all 9 ZT vectors (bytes+len+sha256) | unit/conformance | `pytest tests/test_override_canonical.py -x` (or `cargo test canonical_vectors`) | ❌ Wave 0 |
| VFY-02 / SC2 | valid token + correct pubkey + allowlisted ARN → `Ok(OverrideGrant)` | integration | `pytest tests/test_override_verify.py::test_valid_token_ok -x` | ❌ Wave 0 |
| VFY-02 | `algorithm:"none"` rejected | unit | `::test_algorithm_none_rejected` | ❌ Wave 0 |
| VFY-02 | non-`ECDSA_SHA_256` algorithm rejected | unit | `::test_algorithm_mismatch_rejected` | ❌ Wave 0 |
| VFY-02 | high-S signature rejected | unit | `::test_high_s_rejected` | ❌ Wave 0 |
| VFY-02 | bad signature rejected | unit | `::test_bad_signature_rejected` | ❌ Wave 0 |
| VFY-04 | key ARN not in allowlist rejected | unit | `::test_key_not_allowlisted` | ❌ Wave 0 |
| VFY-05 | expired / not_before-in-future rejected; skew honored; TTL cap | unit | `::test_expired`, `::test_not_yet_valid`, `::test_ttl_cap` | ❌ Wave 0 |
| VFY-06 / SC3 | second verify of same jti rejected in-process (before expiry) | unit | `::test_jti_replay_rejected` | ❌ Wave 0 |
| VFY-07 / SC4 | every Err raises `NonoOverrideError` (not RuntimeError/None) | unit | `::test_raises_nono_override_error` (assert exception type per variant) | ❌ Wave 0 |
| OVR-01 | missing required field rejected (kind=MissingField); unknown field rejected (deny_unknown_fields) | unit | `::test_missing_field`, `::test_unknown_field_rejected` | ❌ Wave 0 |
| SC5 | `#[must_use]` warns if caller ignores the verify Result | compile-check | `cargo build` shows `unused_must_use` warning when a test fixture ignores it (or a `trybuild`/grep assertion) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `pytest tests/test_override_canonical.py tests/test_override_verify.py -x` (+ `cargo test` for in-crate canonical).
- **Per wave merge:** full `pytest -x` + `cargo test` in nono-py; `make ci` in the `nono` workspace IF a core re-export was added (edition-1.82 gate).
- **Phase gate:** full suite green before `/gsd:verify-work`; cross-target clippy if any `#[cfg(...)]` code is added (CLAUDE.md — unlikely here, override.rs is platform-neutral).

### Wave 0 Gaps
- [ ] `nono-py/tests/fixtures/vectors.json` — snapshot of ZT vectors (source-commit recorded) — covers OVR-03/SC1
- [ ] `nono-py/tests/fixtures/override_test_key.{pem,der}` — committed test ECDSA P-256 keypair (D-01)
- [ ] A signed-fixture minting helper (mint a valid override token with the test key; also mint high-S + `algorithm:"none"` + bad-sig + expired variants) — Python or a `#[cfg(test)]` Rust helper
- [ ] `tests/test_override_canonical.py` and `tests/test_override_verify.py`
- [ ] Decide SC5 verification mechanism: a `trybuild`-style compile-fail test, or a documented `cargo build` warning assertion (lighter)
- [ ] `maturin develop --release` step wired into the test workflow (stale-extension guard)

## Security Domain

> `security_enforcement` not set to false in config → ENABLED. This entire phase IS a security control.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication (here: signer authorization) | yes | Key-ARN allowlist (VFY-04) — crypto validity ≠ authorization. |
| V3 Session Management | no | Override is invocation-scoped, no sessions. |
| V4 Access Control | yes (boundary) | Scope is enforced in Phase 92 (additive caps); Phase 91 validates/normalizes scope fields. |
| V5 Input Validation | yes | `deny_unknown_fields`, CAF R7.4 string rejection, absolute-path scope, ISO-8601 timestamp, base64-DER signature. |
| V6 Cryptography | yes | Reuse aws-lc-rs via sigstore-crypto; explicit low-S; algorithm pin; never hand-roll ECDSA; reject `algorithm:"none"`. |
| V7 Error Handling / Logging | yes | Fail-closed; `#[must_use]`; redaction-safe messages (no raw secrets/keys); single exception type. |

### Known Threat Patterns for {ECDSA-over-canonical-JSON verifier}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Fail-open on error | Elevation of Privilege | Every `Err` → deny → `NonoOverrideError`; `#[must_use]`; deny-asserting test per variant (PITFALLS #1). |
| `algorithm:"none"` downgrade | Spoofing/Tampering | Pin `ECDSA_SHA_256`; reject empty signature; reject `"none"` (PITFALLS #2). |
| Signature malleability (high-S) | Tampering | Explicit low-S DER check (`_require_low_s`); aws-lc-rs does NOT enforce. |
| Canonicalization mismatch | Tampering | Re-derive from struct, strip R10 fields, sorted keys; vectors conformance (PITFALLS #3). |
| Unauthorized-but-valid signature | Spoofing | Exact-match key-ARN allowlist (PITFALLS #4). |
| Replay of a consumed token | Replay | In-process jti set (D-03); durable cross-process check is Phase 93. |
| TOCTOU verify→apply | Tampering | Immutable `OverrideGrant` value (D-02). |
| Scope-prefix escape | EoP | Path-component comparison on canonicalized absolute paths (PITFALLS #5; mainly Phase 92). |

## Sources

### Primary (HIGH confidence — read on disk this session)
- ZT-Infra v2: `docs/CANONICAL_FORM.md` (CAF v0.1 R1–R12, hash + signature computation, low-S SHOULD/MUST).
- ZT-Infra v2: `test-vectors/canonical-form/vectors.json` (9 vectors: name/input/strip_fields/canonical_bytes_utf8/canonical_bytes_length/sha256_hex) + `skeletons/rust/canonical.rs` (complete spec-conformant Rust impl + 9 positive + 6 negative tests).
- ZT-Infra v2: `provisioner/src/canonical.js` (`stableJson`, `stripTopLevelFields`), `provisioner/src/audit.js` (`algorithm:"none"` shape line 181, `normalizeEcdsaDerLowS`, `MessageType:"DIGEST"`, `SigningAlgorithm:"ECDSA_SHA_256"`), `provisioner/src/server.js` (POST /actions response = signed AuditRecord), `provisioner/src/policy.js` (allow/deny eval).
- ZT-Infra v2: `zt-verify/src/zt_verify/canonical.py` (`AuditRecord`/`KmsSignature` strict models, `stable_json`, `current_hash`), `signatures.py` (`_require_low_s`, `verify_record_signature_result` recompute-don't-trust, `ECDSA(Prehashed(SHA256))`), `verifier.py` (verification order), `tests/test_canonical_vectors.py` (verbatim-vector consumption pattern).
- nono in-tree: `crates/nono/src/trust/bundle.rs:430-502` (`verify_keyed_signature` = DSSE/PAE — the WRONG primitive), `crates/nono/src/trust/mod.rs` (re-exports), `crates/nono/src/trust/signing.rs:152-153` (re-exports `SigningScheme`,`DerPublicKey`,`SignatureBytes`), `crates/nono/Cargo.toml` (sha2/serde_json/sigstore-verify 0.8.0/der/x509-cert; no base64/chrono/aws-lc-rs direct).
- sigstore-crypto 0.8.0 (cargo registry on disk): `src/verification.rs` (`VerificationKey::from_spki`, `::verify`, `::verify_prehashed` → aws-lc-rs `verify_digest`), `src/checkpoint.rs` (`verify_ecdsa_p256` uses `ECDSA_P256_SHA256_ASN1`), `src/hash.rs`. sigstore-verify 0.8.0 `src/lib.rs` (`pub use sigstore_crypto as crypto`).
- nono-py: `Cargo.toml` (edition 2024, rust 1.95, pyo3 0.28, serde/serde_json direct), `Cargo.lock` (sigstore-crypto 0.8.0, aws-lc-rs 1.17.0, chrono 0.4.45, base64 0.22.1, sha2 0.10/0.11 all present), `src/lib.rs` (`to_py_err` built-in-exception mapping, `#[pymodule] _nono_py` registration), `src/sandboxed_exec.rs` (`#[pyclass(frozen)]` ExecResult pattern), `tests/` layout + `conftest.py`.
- nono: `proj/POC-zt-infra-e5-local-provisioner.md` (E5 composition; `POST /actions` returns `{decision,reason,audit:{...,kms_signature}}`; Python `urllib` is Phase-93 concern).
- nono `.planning`: `CONTEXT.md`, `REQUIREMENTS.md`, `research/SUMMARY.md`, `research/STACK.md`, `research/PITFALLS.md`, `CLAUDE.md`.

### Secondary (MEDIUM)
- AWS aws-lc-rs / aws-lc docs (general ECDSA/aws-lc-rs background) — used only to corroborate that ASN.1 ECDSA verify does not enforce low-S; the authoritative ground truth is the two ZT reference impls that add the check explicitly. https://github.com/aws/aws-lc-rs , https://aws.amazon.com/blogs/opensource/introducing-aws-libcrypto-for-rust-an-open-source-cryptographic-library-for-rust/

## Metadata

**Confidence breakdown:**
- Canonical form (OVR-03/SC1): HIGH — spec + 3 reference impls + 9 verbatim vectors + a shipped Rust impl read directly.
- Crypto primitive (VFY-02/03): HIGH — `verify_keyed_signature` shown to be wrong; `verify_prehashed` shown to be right, read in the on-disk crate source.
- Low-S: HIGH — both ZT reference impls enforce it explicitly; aws-lc-rs does not.
- Token field model (OVR-01/02): MEDIUM — ZT-Infra defines no override schema; the override-token shape is a nono construct needing operator confirmation (OQ-1, A1).
- Error model / PyO3 (VFY-07/SC4/SC5): HIGH for the pattern; MEDIUM on the exact PyO3 0.28 `create_exception!` registration line (A2 — verify at plan time).
- Deps: HIGH — all present in nono-py Cargo.lock; promotions only, no net-new crate.

**Research date:** 2026-06-21
**Valid until:** ~2026-07-21 (stable: in-tree + pinned crate versions). Re-verify OQ-1 with the ZT-Infra operator before locking the serde model regardless of date.

## RESEARCH COMPLETE
