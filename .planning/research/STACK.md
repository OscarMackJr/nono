# Stack Research

**Domain:** Cryptographically-signed policy-override tokens + AWS ZT-Infra v2 control plane integration for a Rust/PyO3 capability sandbox (nono v3.2)
**Researched:** 2026-06-21
**Confidence:** HIGH — verified against in-tree Cargo.toml, nono-py Cargo.toml, ZT-Infra v2 source (provisioner/src/audit.js + canonical.js), CANONICAL_FORM.md spec, and crates.io current releases

---

## Executive Framing

The v3.2 milestone needs three new capabilities on top of the existing workspace:

1. **Signed policy-exception token** — define, parse, and verify a structured token (signer identity, scope, expiry, repo-context binding) using a compatible signature scheme. The KMS signature algorithm is `ECDSA_SHA_256` over P-256 (secp256r1). The existing `sigstore-verify` / `sigstore-sign` + `aws-lc-rs` are already present; the question is whether they already expose raw P-256 ECDSA verification, or whether a lightweight shim crate is needed.

2. **AWS ZT-Infra v2 HTTP client from nono-py** — `POST /actions` round-trip, including reading the `audit.kms_signature` field from the JSON response. The enforcement surface is the `nono-py` PyO3 binding (Python process), not the Rust CLI. The simplest correct split is: do the HTTP call in Python (standard library or a thin library), then pass the verified decision and relevant hash fields into a new Rust function via PyO3.

3. **KMS signature verification** — verify the `ECDSA_SHA_256` DER-encoded, base64-stored signature from the ZT-Infra audit record against the AWS KMS public key. The canonical form is fully specified in `CANONICAL_FORM.md` (CAF v0.1): SHA-256 over sorted-key, whitespace-free UTF-8 JSON with `current_hash` and `kms_signature` stripped before hashing.

---

## Reuse vs New — Decision Table

| Capability | Status | Crate / Library | Action |
|---|---|---|---|
| ECDSA P-256 signature verification | **REUSE** | `aws-lc-rs` 1.x (already in `nono-cli`) | Expose raw `UnparsedPublicKey::verify` path behind a new `nono` library function |
| SHA-256 hashing | **REUSE** | `sha2` workspace dep | Already at `sha2 = "0.11"` — no change |
| JSON serialization (canonical form) | **REUSE** | `serde_json` workspace dep (`preserve_order = false` for sorted keys is the default) | Implement `canonical_json()` helper following CAF v0.1 R1-R10 — no new crate |
| Base64 decode (DER signature from audit record) | **REUSE** | `aws-lc-rs` (includes `base64` utilities) or `base64` crate 0.22 | `base64` 0.22 already transitive via sigstore graph — promote to direct dep in `nono` if needed, or use `aws-lc-rs::encoding` |
| Sigstore keyless bundle verification (existing use case) | **REUSE** | `sigstore-verify` 0.8.0 | Unchanged; KMS override path is a separate code path |
| HTTP client (nono-py → ZT-Infra `POST /actions`) | **NEW (Python-side)** | Python `urllib.request` (stdlib) | No new Rust dep; the POC already demonstrates this works and is the cheapest path for a host-gated integration |
| Policy-exception token type (serde struct) | **NEW (Rust-side, in `nono` lib)** | `serde` + `serde_json` (already present) | New `PolicyOverride` struct and `OverrideVerifier` in `crates/nono/src/override.rs` |
| Expiry / timestamp parsing | **NEW (Rust-side)** | `chrono` 0.4 (already in `nono-cli`) or RFC 3339 via `serde_json` string + `std::time` | Promote `chrono` to `nono` lib or use `std::time::SystemTime` comparison with ISO-8601 string parsing |
| PyO3 bridge for override verification result | **NEW (nono-py-side)** | `pyo3` 0.28 (already in `nono-py/Cargo.toml`) | New `#[pyfunction] fn verify_override(...)` in `nono-py/src/override.rs` |
| SecurityEventLayer audit emission | **REUSE** | Existing `SecurityEventLayer` HMAC chain (v3.0/v3.1) | Call existing emission path from the override-verify result |

---

## Recommended Stack

### Core Technologies — Rust Side (`crates/nono`)

| Technology | Version | Purpose | Why Recommended |
|---|---|---|---|
| `aws-lc-rs` | `1` (already in `nono-cli`) | P-256 ECDSA signature verification for KMS-signed audit records | AWS KMS produces `ECDSA_SHA_256` (P-256/secp256r1) with low-S DER encoding. `aws-lc-rs` is the canonical Rust wrapper for AWS-LC (BoringSSL fork), used by rustls and the sigstore ecosystem. It exposes `agreement::UnparsedPublicKey` and `signature::UnparsedPublicKey::verify` for raw ECDSA P-256. Already a direct dep in `nono-cli`; must be promoted to `nono` lib dep to live at the right library boundary. |
| `sha2` | `0.11` (workspace) | SHA-256 for CAF v0.1 canonical hash computation | Already present, no change. CAF v0.1 `canonical_hash = lower_hex(sha256(canonical_bytes(record)))`. |
| `serde` / `serde_json` | workspace | `PolicyOverride` token deserialize + CAF canonical JSON serialization | Already present. Canonical form requires sorted keys with no whitespace — `serde_json::to_string` produces RFC 8259 JSON that can be sorted with a custom serializer or a `BTreeMap` intermediary to satisfy CAF R3. |
| `base64` | `0.22` | Decode KMS DER signature from base64 string in audit record | Already transitively present via sigstore; promote to direct dep in `nono`. Version 0.22 uses the Engine API (non-deprecated). |
| `chrono` | `0.4` (already in `nono-cli`) | RFC 3339 expiry timestamp parsing and comparison for `PolicyOverride.expires_at` | Already in `nono-cli`; needs to be promoted to `nono` lib if expiry logic lives there, OR expiry check stays CLI-side with only token parsing in the lib. Prefer promoting to `nono` lib — the fail-closed expiry gate is a security invariant that belongs in the verifier. |
| `zeroize` | `1` (workspace) | Zero sensitive token fields (signing key material) on drop | Already present. |

### Core Technologies — Python Side (`nono-py`)

| Technology | Version | Purpose | Why Recommended |
|---|---|---|---|
| Python `urllib.request` | stdlib (Python 3.10+) | `POST /actions` HTTP call to ZT-Infra control plane | No new dependency. The POC runbook already demonstrates this (30 lines; timeout via `urlopen(..., timeout=5)`). The ZT-Infra endpoint speaks plain HTTP/HTTPS JSON. Adding `httpx` or `requests` would be unnecessary weight for a single endpoint call. |
| `pyo3` | `0.28` (already in `nono-py/Cargo.toml`) | Bridge for new `verify_override` Rust function callable from Python | No change needed. |

### Supporting Libraries — Rust Side

| Library | Version | Purpose | When to Use |
|---|---|---|---|
| `sigstore-verify` | `0.8.0` (already in `nono`) | Sigstore keyless bundle verification (existing use: instruction-file attestation) | NOT on the KMS override path — keep as-is for the existing sigstore use case. Do not reuse for KMS verification; they are different trust roots. |
| `x509-cert` | `0.2` (already in `nono`) | DER cert parsing (used in existing trust module) | May be needed to import the AWS KMS public key from its DER/PEM export for verification bootstrapping. Already present. |
| `der` | `0.7` (already in `nono`) | DER encoding/decoding | Needed if parsing the DER-encoded ECDSA signature directly (before `aws-lc-rs` ingestion). Already present. |
| `hmac` | `0.13` (already in `nono-cli`) | HMAC-SHA256 for SecurityEventLayer chain | Unchanged; override events enter the existing chain. |

### Development Tools

| Tool | Purpose | Notes |
|---|---|---|
| `cargo clippy --workspace --target x86_64-unknown-linux-gnu` | Cross-target lint for new cfg-gated code | Mandatory per CLAUDE.md §Coding Standards — any new `#[cfg(windows)]` code in `nono` or `nono-py` must be verified cross-target |
| `maturin develop --release` | Build nono-py binding with new Rust functions | Required after adding `verify_override` to `nono-py/src/` |
| Python `unittest` / `pytest` | Test `POST /actions` adapter and token parsing in Python | Keep in `nono-py/tests/`; gate with `pytest.mark.integration` for host-gated AWS path |

---

## Installation (New Additions Only)

All Rust-side additions are version bumps or promotions of existing transitive deps — no new `cargo add` calls for net-new crates.

```toml
# In crates/nono/Cargo.toml — promote these from transitive to direct:
aws-lc-rs = "1"          # was direct in nono-cli only; needed in nono lib for verify_override
base64 = "0.22"          # was transitive via sigstore; needed for DER sig decode
chrono = { version = "0.4", features = ["serde"] }   # was in nono-cli only
```

Python side — no new packages:
```bash
# Python stdlib only for the HTTP adapter (urllib.request + json)
# pytest already in nono-py dev dependencies
```

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|---|---|---|
| `aws-lc-rs` for P-256 ECDSA verify | `ring` | `ring` is FIPS-incompatible and blocked by the existing aws-lc-rs choice in nono-cli; importing both would create two crypto backends. `aws-lc-rs` is a drop-in with the same `ring`-compatible API surface. |
| `aws-lc-rs` for P-256 ECDSA verify | `p256` crate (RustCrypto) | `p256` is fine cryptographically but adds a second ECDSA implementation when `aws-lc-rs` already ships one. Duplication with no benefit. |
| `sigstore-verify` for KMS override | reusing existing sigstore bundle path | The Sigstore trust chain (Fulcio CA + Rekor transparency log) is incompatible with the ZT-Infra KMS trust model. KMS uses a static asymmetric key whose public key is fetched once from AWS KMS; Sigstore uses OIDC-issued short-lived certs and CT logs. These are fundamentally different trust anchors; do not conflate them. |
| Python `urllib.request` for HTTP | `httpx` or `requests` in nono-py | nono-py is a compiled PyO3 extension — adding a Python runtime dependency makes distribution fragile (pip install required) and the existing POC works fine with stdlib. |
| Python `urllib.request` for HTTP | Rust `hyper`/`reqwest` in nono-py Rust layer | The Rust-side has no tokio runtime in the nono-py `confined_run` synchronous call path. Spinning up an async runtime for a single HTTP call inside a PyO3 `#[pyfunction]` is overly complex. Python handles the HTTP round-trip; Rust handles signature verification. |
| `chrono` for timestamp | `time` crate | `time` is already in the workspace (transitive via sigstore); either works. `chrono` is already a direct dep in `nono-cli` and its `DateTime::parse_from_rfc3339` + `Utc::now()` comparison is the idiomatic approach for the override expiry pattern. |
| BTreeMap-based canonical JSON | `olpc-cjson` or `json-canon` crates | Adding a new crate for something implementable in ~40 lines against the normative CAF v0.1 spec (CANONICAL_FORM.md). The spec is fully documented; a bespoke `canonical_json()` function is auditable and zero-dependency. |

---

## What NOT to Add

| Avoid | Why | Use Instead |
|---|---|---|
| `reqwest` in nono-py Cargo.toml | Pulls in a full async HTTP stack (tokio, h2, rustls) inside a sync PyO3 extension; version conflicts with existing tokio features in nono-py are likely | Python `urllib.request` stdlib in the Python adapter layer |
| A new JWT library (`jsonwebtoken` / `jwt-simple`) | The `PolicyOverride` token format is not JWT — it is a custom JSON struct signed with KMS ECDSA. Adding JWT parsing would import the wrong token model and a needless dep | `serde_json` deserialization of a defined `PolicyOverride` struct + manual ECDSA verify via `aws-lc-rs` |
| `openssl` crate | Would introduce a second TLS/crypto backend conflicting with `aws-lc-rs`. Forbidden by the existing security posture | `aws-lc-rs` already provides equivalent P-256 ECDSA |
| DAAL/blockchain integration in this milestone | The DAAL layer is async, EVM-specific, and requires external wallet infrastructure. The primary authorization path doesn't wait for ledger confirmation (per ZT-Infra architecture). DAAL anchoring is a ZT-Infra server-side concern, not nono-side | If DAAL receipt verification is ever needed client-side, treat it as a later phase with a scoped spike |
| `ethers` or `viem` in nono-py | EVM transaction sending is not nono's responsibility; nono reads `audit.daal` as an opaque field for logging into SecurityEventLayer | Read `daal` field as `Option<serde_json::Value>` and emit into event chain without interpretation |
| Promoting `sigstore-sign` to nono-py | nono-py does not sign overrides — it verifies them. Sigstore signing is a nono-cli concern (instruction-file attestation, existing use) | Verification-only path via `aws-lc-rs` |

---

## Architecture Split: Python vs Rust

The enforcement surface is `nono-py`'s `confined_run` / `confine`. The split must be clean:

```
Python layer (nono-py Python wrapper or caller):
  1. POST /actions to ZT-Infra (urllib.request, JSON body)
  2. Parse JSON response → {decision, audit.current_hash, audit.kms_signature}
  3. Call Rust: verify_override(token_json: str, kms_public_key_der: bytes) -> PyResult<OverrideDecision>
  4. On OverrideDecision::Allow → call confined_run() with expanded CapabilitySet
  5. On OverrideDecision::Deny / verification failure → fail-closed (do not spawn)

Rust layer (nono lib + nono-py bridge):
  1. PolicyOverride struct: serde-deserialize from token_json
     Fields: actor, action, resource, scope (paths/network), expires_at (RFC 3339), repo_context (git remote URL hash), kms_key_id
  2. OverrideVerifier::verify(token, public_key_der):
     a. Strip current_hash + kms_signature per CAF v0.1 R10
     b. Compute canonical_json() → SHA-256 → 32-byte digest
     c. Base64-decode kms_signature.signature → DER bytes
     d. aws-lc-rs UnparsedPublicKey::verify(EC_PUBLIC_KEY, P256, digest, sig)
     e. Check expires_at > Utc::now() (fail-closed if expired or parse fails)
     f. Check repo_context matches caller's repo (supplied by nono-py)
     g. Return Ok(OverrideDecision::Allow { scope }) or Err(NonoError::OverrideVerificationFailed)
  3. #[pyfunction] verify_override(...) → PyResult<PyOverrideDecision> in nono-py bridge
  4. Emit SecurityEventLayer event (EventID 10006 or next available) on both Allow and Deny
```

**Why this split?** The HTTP call is unavoidably host-gated (requires live AWS endpoint) — keeping it in Python makes it easy to mock with `unittest.mock.patch`. The cryptographic verification is Rust-side where type safety and the existing `aws-lc-rs` dep live. The PyO3 boundary is the narrowest possible interface: token JSON string in, decision + scope out.

---

## KMS Public Key Distribution

This is an **operational gap that must be resolved in requirements**, not a stack question:

- The nono-py verifier needs the AWS KMS P-256 public key in DER format to verify signatures offline (without a round-trip to KMS `GetPublicKey`).
- The public key is stable as long as the KMS key is not rotated.
- Two viable approaches: (a) embed the public key in a config file loaded by nono-py / nono-cli at startup; (b) fetch it at startup via `aws-kms GetPublicKey` (requires AWS credentials in the nono process — heavy dep). Approach (a) is recommended for MVP: the public key is not secret (it is the *public* half), and embedding it avoids an AWS SDK dep in nono.
- If approach (a), the key distribution/rotation mechanism is a policy question out of scope for the stack, but the roadmap must flag it.

---

## CAF v0.1 Canonical Form Implementation Note

The ZT-Infra audit record canonical form is fully specified in `CANONICAL_FORM.md`. Implementing it in Rust requires:

1. A `BTreeMap`-based JSON value walk that sorts object keys (satisfying R3) and produces no whitespace (R2).
2. String validation per R7.4: reject control chars U+0000–U+001F, U+2028, U+2029, surrogates, and chars above U+FFFF.
3. Strip `current_hash` and `kms_signature` from the top level before hashing (R10).
4. SHA-256 the canonical UTF-8 bytes; output as lowercase hex (no `0x` prefix).

This is ~60-80 lines of Rust and does not require a new crate. Implementing it against the spec (not against the JS reference implementation) is the correct approach.

---

## Version Compatibility

| Package A | Compatible With | Notes |
|---|---|---|
| `aws-lc-rs 1` | `sha2 0.11` (workspace) | No conflict; separate crates with no shared types at the verify boundary |
| `aws-lc-rs 1` | `sigstore-verify 0.8.0` | sigstore-verify already uses aws-lc-rs transitively; no second copy |
| `base64 0.22` | `aws-lc-rs 1` | base64 0.22 already in lock file via sigstore graph; promote is safe |
| `chrono 0.4` | `serde 1` (workspace) | chrono's `serde` feature required for `DateTime<Utc>` deserialization |
| `pyo3 0.28` | Python 3.10+ | Already the nono-py constraint; no change |

---

## Sources

- `C:\Users\OMack\Nono\crates\nono\Cargo.toml` — confirmed `sigstore-verify 0.8.0`, `aws-lc-rs 1` (via `nono-cli/Cargo.toml`), `sha2 0.11`, `base64` transitive, `x509-cert 0.2`, `der 0.7` (HIGH confidence — in-tree)
- `C:\Users\OMack\Nono\crates\nono-cli\Cargo.toml` — confirmed `aws-lc-rs = "1"` direct dep, `chrono 0.4`, `hmac 0.13`, `sigstore-sign 0.8.0`, `sigstore-trust-root 0.8.0` (HIGH confidence — in-tree)
- `C:\Users\OMack\nono-py\Cargo.toml` — confirmed `pyo3 = "0.28"`, `serde_json = "1"` (HIGH confidence — in-tree)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\audit.js` — confirmed `ECDSA_SHA_256`, `SigningAlgorithm`, `MessageType: "DIGEST"`, DER encoding, base64 output, `normalizeEcdsaDerLowS` (HIGH confidence — source)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\CANONICAL_FORM.md` — confirmed CAF v0.1 spec: P-256 / SHA-256, sorted keys, strip `current_hash`+`kms_signature`, low-S enforcement (HIGH confidence — normative spec)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\canonical.js` — confirms `stableJson()` key-sort implementation matches CAF spec (HIGH confidence — source)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\package.json` — confirmed `@aws-sdk/client-kms 3.1046.0`, `ethers 6.16.0` (ZT-Infra server-side, NOT nono-side) (HIGH confidence — source)
- `C:\Users\OMack\Nono\proj\POC-zt-infra-e5-local-provisioner.md` — confirmed Python `urllib.request` is the proven POC HTTP path; E5 composition contract (HIGH confidence — in-tree)

---
*Stack research for: nono v3.2 Signed Policy Overrides (ZT-Infra Attestation)*
*Researched: 2026-06-21*
