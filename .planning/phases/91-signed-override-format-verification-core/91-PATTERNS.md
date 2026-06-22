# Phase 91: Signed Override Format + Verification Core - Pattern Map

**Mapped:** 2026-06-21
**Files analyzed:** 4 new + 2 modified (6)
**Analogs found:** 6 / 6 (all in-repo; one cross-repo seed reuse)

> Scope reminder: the primary deliverable is the NEW module `nono-py/src/override.rs`
> (`OverrideToken` serde model, `canonical_bytes()`, `verify_override()`, `OverrideGrant`
> `#[pyclass]`, `OverrideErrorKind`, `NonoOverrideError`). RESEARCH.md D-05 supersedes the
> `verify_keyed_signature` reference — the crypto analog below points at the **correct**
> `verify_prehashed` primitive, not the DSSE/PAE one. RESEARCH.md D-06 locks the serde model
> as a nono-side construct.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `nono-py/src/override.rs` (NEW) | service / verifier module | transform (parse → canonicalize → verify → grant) | `nono-py/src/policy.rs` (serde model + `#[pyclass]` getters) + `nono-py/src/sandboxed_exec.rs` (`#[pyclass(frozen)]` value type + `#[pyfunction]`) | role-match (composite) |
| `nono-py/src/override.rs` :: `OverrideGrant` `#[pyclass(frozen)]` | model (immutable value) | request-response (returned value) | `nono-py/src/sandboxed_exec.rs::ExecResult` (`#[pyclass(frozen)]` + `#[pyo3(get)]`) | exact |
| `nono-py/src/override.rs` :: `OverrideToken` serde struct | model (deserialize) | transform | `nono-py/src/policy.rs::Group`/`AllowOps` (`#[derive(Deserialize)]`, `#[serde(default)]`) | exact (role) — NOTE: override adds `#[serde(deny_unknown_fields)]` (policy.rs does NOT use it) |
| `nono-py/src/override.rs` :: ECDSA digest verify | utility (crypto call) | transform | sigstore-crypto `VerificationKey::verify_prehashed` (verification.rs:122) used per `bundle.rs:463-502` call shape | role-match (call-site shape from `verify_keyed_signature`, primitive swapped per D-05) |
| `nono-py/src/override.rs` :: `canonical_bytes()` | utility (canonicalization + hash) | transform | ZT seed `skeletons/rust/canonical.rs` (reuse verbatim, swap `Error`→`OverrideErrorKind`) + `crates/nono/src/trust/digest.rs` (sha2 + lowercase-hex idiom) | exact (cross-repo seed) |
| `nono-py/src/override.rs` :: low-S DER parse | utility | transform | `crates/nono/Cargo.toml` `der = "0.7"` (in nono, NOT yet nono-py direct) OR hand-roll per ZT `_require_low_s` | partial (no in-tree low-S analog) |
| `nono-py/src/lib.rs` (MODIFY) | provider / registration | request-response | `nono-py/src/lib.rs::_nono_py` `#[pymodule]` (existing `m.add_class`/`m.add_function`) + `to_py_err` mapper | exact |
| `nono-py/Cargo.toml` (MODIFY) | config | — | `nono-py/Cargo.toml` `[dependencies]` block (existing direct-dep shape) | exact |
| `nono-py/tests/test_override_canonical.py` (NEW) | test | transform | `nono-py/tests/conftest.py` (fixtures) + existing `tests/test_*.py` layout | role-match |
| `nono-py/tests/test_override_verify.py` (NEW) | test | request-response | same | role-match |
| `nono-py/tests/fixtures/` (NEW: keypair + vectors.json) | test fixture | file-I/O | `crates/nono/tests/fixtures/trust-root-frozen.json` + `crates/nono/src/trust/mod.rs::load_test_trusted_root` (test-only fixture loader) | exact (fixture-load idiom) |

## Pattern Assignments

### `nono-py/src/override.rs` :: `OverrideGrant` (model, immutable value type)

**Analog:** `nono-py/src/sandboxed_exec.rs::ExecResult` (lines 23-49)

This is the D-02 TOCTOU-safe value. Copy the `#[pyclass(frozen)]` + `#[pyo3(get)]` field
exposure + `__repr__` pattern exactly.

**Frozen value-type pattern** (`sandboxed_exec.rs:29-49`):
```rust
#[pyclass(frozen)]
pub struct ExecResult {
    #[pyo3(get)]
    pub stdout: Vec<u8>,
    #[pyo3(get)]
    pub stderr: Vec<u8>,
    #[pyo3(get)]
    pub exit_code: i32,
}

#[pymethods]
impl ExecResult {
    fn __repr__(&self) -> String {
        format!("ExecResult(exit_code={}, ...)", self.exit_code, ...)
    }
}
```
> For `OverrideGrant`: `#[pyclass(frozen)]` carrying `signer`, `scope_paths`, `scope_domains`,
> `not_before`, `expires_at`, `jti`, `repo_context` — each `#[pyo3(get)]` (or `#[getter]` if a
> derived/clone is needed, per `lib.rs::FsCapability` getters at lib.rs:179-225). `frozen` =
> immutable from Python (closes PITFALLS #6). NOTE: `lib.rs::SandboxState` (skip_from_py_object,
> NOT frozen) is the wrong analog here — use the `frozen` ExecResult shape.

---

### `nono-py/src/override.rs` :: `OverrideToken` (model, serde deserialize)

**Analog:** `nono-py/src/policy.rs::Group` / `AllowOps` (lines 15-79) — serde model living in a
nono-py module, deserialized via `serde_json::from_str` (policy.rs:224).

**serde struct pattern** (`policy.rs:20-46`):
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub allow: Option<AllowOps>,
    ...
}
```

**Parse-with-error-mapping pattern** (`policy.rs:223-227`):
```rust
pub fn load_policy(json: &str) -> NonoResult<Policy> {
    let policy = serde_json::from_str(json)
        .map_err(|e| NonoError::ConfigParse(format!("Failed to parse policy.json: {}", e)))?;
    Ok(Policy { inner: policy })
}
```
> **DEVIATION (Claude's Discretion + D-06):** `OverrideToken` MUST add
> `#[serde(deny_unknown_fields)]` at the struct level — `policy.rs` does NOT use it (it is
> forward-compat-lenient). The override token is fail-secure: unknown field → `Err{kind:Parse}`.
> Map `serde_json::from_str` error → `OverrideErrorKind::Parse`; a serde "missing field" error →
> `OverrideErrorKind::MissingField` (string-inspect the serde error or model required fields as
> non-`Option`). The nested signed object mirrors the CAF AuditRecord field set (see
> `vectors.json` TV-01 `input`): `actor, action, resource, decision, reason, timestamp,
> previous_hash, current_hash, kms_signature{algorithm,key_id,signature}` — OVR-01 fields ride
> *inside* the canonicalized object per D-06.

---

### `nono-py/src/override.rs` :: ECDSA P-256 digest verify (utility, crypto)

**Analog (call-site shape):** `crates/nono/src/trust/bundle.rs::verify_keyed_signature`
(lines 463-502) — **BUT** D-05 swaps the primitive. Copy the `from_spki` + `SignatureBytes`
ceremony from this site; replace `vk.verify(&pae_bytes, ...)` with `vk.verify_prehashed(&digest, ...)`.

**Primitive (correct, per D-05):** sigstore-crypto 0.8.0 `verification.rs:122`:
```rust
// C:\Users\OMack\.cargo\registry\...\sigstore-crypto-0.8.0\src\verification.rs:118-143
pub fn verify_prehashed(&self, digest: &Sha256Hash, signature: &SignatureBytes) -> Result<()> {
    // SigningScheme::EcdsaP256Sha256 →
    //   UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, &self.bytes)
    //       .verify_digest(&aws_digest, signature.as_bytes())   // aws-lc-rs, DIGEST mode
}
```

**Ceremony to copy** (adapted from `bundle.rs:482-501`, primitive swapped):
```rust
use sigstore_verify::crypto::signing::SigningScheme;
use sigstore_verify::crypto::verification::VerificationKey;
use sigstore_verify::types::{SignatureBytes, DerPublicKey, Sha256Hash};

let pub_key = DerPublicKey::from(pubkey_der.to_vec());
let vk = VerificationKey::from_spki(&pub_key, SigningScheme::EcdsaP256Sha256)
    .map_err(|_| OverrideErrorKind::BadSignature)?;
let sig = SignatureBytes::from_base64(&sig_b64)
    .map_err(|_| OverrideErrorKind::BadSignature)?;
let digest = Sha256Hash::from_bytes(<[u8;32]>::try_from(&sha256_bytes[..])?);
vk.verify_prehashed(&digest, &sig)              // NOT vk.verify(...) — that double-hashes
    .map_err(|_| OverrideErrorKind::BadSignature)?;
```
> **These types are re-exported from `nono`** so `override.rs` can import them through the `nono`
> crate dependency (no new dep): `crates/nono/src/trust/mod.rs:39-47` re-exports `DerPublicKey`,
> `Sha256Hash`; `signing.rs:152-153` re-exports `SigningScheme`, `SignatureBytes`. Alternatively
> import from `sigstore_verify::crypto` directly (transitive). ANTI-PATTERN (RESEARCH PITFALLS):
> do NOT call `nono::trust::verify_keyed_signature` — it computes PAE over an in-toto payload and
> calls `vk.verify` (bundle.rs:497), never verifies a KMS raw-digest signature.

---

### `nono-py/src/override.rs` :: `canonical_bytes()` + SHA-256 (utility)

**Analog (primary):** ZT-Infra seed `test-vectors/canonical-form/skeletons/rust/canonical.rs`
(reuse near-verbatim per "Don't Hand-Roll"). It is spec-conformant and passes all 9 vectors.

**Seed shape** (`canonical.rs:27-31, 33-34`):
```rust
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;        // R3 code-point key sort
use std::fmt::Write;
pub const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
// Error enum (FloatNotAllowed/NullNotAllowed/BoolNotAllowed/IntegerOutOfRange/
//   ControlCharInString/LineSeparatorInString/SurrogateInString/SupraBmpInString/UnsupportedType)
```
> Swap the seed's `thiserror`-derived `Error` for `OverrideErrorKind` (canonicalize errors map to
> `OverrideErrorKind::Parse`). Strip set for override tokens = `&["current_hash","kms_signature"]`
> (R10). NEVER `serde_json::to_string` the received bytes (PITFALLS #3 / R3) — walk the parsed
> `Value` with `BTreeMap` sorting.

**Analog (lowercase-hex digest idiom, in-tree):** `crates/nono/src/trust/digest.rs::bytes_digest`
+ `hex_encode` (lines 34-48):
```rust
#[must_use]
pub fn bytes_digest(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    hex_encode(&hash)        // lowercase {b:02x}, no 0x prefix
}
```
> Matches the seed's `hex::encode(Sha256::digest(&bytes))`. The `#[must_use]` on `bytes_digest`
> is the in-tree precedent for SC5 `#[must_use]` placement (apply it to `verify_override`'s
> `Result`). The `sha2` workspace dep is already reachable (`crates/nono/Cargo.toml:27`
> `sha2.workspace = true`); confirm it's a direct nono-py dep or import the `nono::trust::digest`
> helpers.

---

### `nono-py/src/override.rs` :: low-S DER check (utility) — NO in-tree analog

**No close analog.** The nono crate has `der = "0.7"` and `x509-cert = "0.2"` as deps
(`crates/nono/Cargo.toml:50-51`) but uses them for cert parsing, not ECDSA `r,s` low-S. Two
research-blessed paths (decide at plan time):
- (a) promote `der = "0.7"` to a nono-py direct dep and parse `SEQUENCE { INTEGER r, INTEGER s }`;
- (b) hand-roll the ~30-line integer parse mirroring ZT `audit.js::normalizeEcdsaDerLowS` /
  `signatures.py::_require_low_s` (reject `s > n/2`), zero new dep.
> See "No Analog Found" table. aws-lc-rs ASN.1 verify does NOT enforce low-S (RESEARCH D-05) —
> this check is mandatory for VFY-02.

---

### `nono-py/src/lib.rs` (MODIFY — provider/registration)

**Analog:** the existing `#[pymodule] _nono_py` (lib.rs:720-766) + `to_py_err` (lib.rs:29-45).

**Module-registration pattern** (`lib.rs:722-762`):
```rust
m.add_class::<CapabilitySet>()?;
...
m.add_function(wrap_pyfunction!(apply, m)?)?;
#[cfg(not(windows))]
m.add_function(wrap_pyfunction!(sandboxed_exec::sandboxed_exec, m)?)?;
```
> Add (platform-neutral — `override.rs` has no `#[cfg]`): `mod override;` near lib.rs:17-23;
> `m.add_class::<override_mod::OverrideGrant>()?;`
> `m.add_function(wrap_pyfunction!(override_mod::verify_override, m)?)?;`
> and the custom-exception registration (below). NOTE `override` is a reserved-ish module name —
> use a non-keyword module path (e.g. `mod override_grant;` or `#[path = "override.rs"] mod
> override_mod;`) since `override` is a reserved Rust keyword; the FILE is `override.rs` but the
> `mod` identifier must be escaped (`r#override`) or renamed. **Flag at plan time** — this is a
> real compile blocker not present in any existing module name.

**Custom exception — NO in-tree analog (first custom exception in nono-py).**
`to_py_err` (lib.rs:29-45) maps `NonoError` to **built-in** exceptions (`PyRuntimeError`,
`PyValueError`, ...) via `new_err(e.to_string())`. There is **zero** use of `create_exception!`
anywhere in the repo (confirmed by grep — only RESEARCH.md references it). Pattern to introduce
(PyO3 0.28, per RESEARCH Pattern 4 — verify the macro/`m.add` signature against PyO3 0.28 at impl):
```rust
use pyo3::create_exception;
create_exception!(_nono_py, NonoOverrideError, pyo3::exceptions::PyException);
// in #[pymodule]: m.add("NonoOverrideError", m.py().get_type::<NonoOverrideError>())?;
fn override_err_to_py(kind: OverrideErrorKind, msg: String) -> PyErr {
    NonoOverrideError::new_err((kind.as_str().to_string(), msg))  // args[0] = stable kind (D-04)
}
```
> Mirror `to_py_err`'s map-on-the-boundary placement, but build a NEW `override_err_to_py` (don't
> extend `to_py_err`, which is `NonoError`-typed). `kind` as `args[0]` is the OQ-3 recommendation;
> D-04 requires the stable `kind` be machine-readable for Phase 92's EventID 10008 map.

---

### `nono-py/Cargo.toml` (MODIFY — config)

**Analog:** the existing `[dependencies]` block (Cargo.toml:15-22).

**Current direct-dep shape:**
```toml
[dependencies]
nono = { path = "../Nono/crates/nono" }
pyo3 = { version = "0.28", features = ["extension-module"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```
> Promote (RESEARCH STACK — all already transitive in `nono-py/Cargo.lock`, promotions only):
> ```toml
> chrono = { version = "0.4", features = ["clock"] }   # +["serde"] only if deserializing timestamps via serde
> base64 = "0.22"
> # sha2 = "0.10"   # if not reachable as a direct dep (verify; nono re-exports digest helpers)
> # der  = "0.7"    # only if NOT hand-rolling low-S
> ```
> Match the `nono` workspace pin style. No net-new crate. Edition/MSRV note (PITFALLS #7): nono-py
> is edition 2024 / rust 1.95; if you add a `nono::trust` re-export, keep it edition-2021-clean —
> prefer calling through the existing re-exports (no core edit needed).

---

## Shared Patterns

### Fail-secure error handling (`#[must_use]` + every-Err-denies)
**Source:** `crates/nono/src/trust/digest.rs:35` (`#[must_use]` on a critical Result-ish fn) +
CLAUDE.md § Coding Standards (no `.unwrap()`/`.expect()`, clippy `unwrap_used` enforced).
**Apply to:** `verify_override`'s `Result` (SC5), every internal helper. No `unwrap_or*`, no
`Option`-with-deny (PITFALLS #1). Every `Err` → `override_err_to_py` → `NonoOverrideError`.

### Map-error-at-the-PyO3-boundary
**Source:** `nono-py/src/lib.rs::to_py_err` (lib.rs:29-45) + per-call `.map_err(to_py_err)`
(e.g. lib.rs:272, 484, 514).
**Apply to:** `verify_override` returns `PyResult<OverrideGrant>`; internal core returns
`Result<OverrideGrant, OverrideErrorKind>`; the boundary fn maps `OverrideErrorKind` →
`NonoOverrideError` exactly as `to_py_err` maps `NonoError` → built-ins.

### Path-component comparison, never string `starts_with`
**Source:** CLAUDE.md § Security Considerations footgun #1; `nono-py/src/lib.rs::path_covered`
(lib.rs:362-364) delegates to `Path::path_covered`.
**Apply to:** any scope-path normalization/validation done here (mostly Phase 92, but reject
non-absolute scope paths in the token; if normalized, use `Path` component compare). PITFALLS #5.

### Test-only fixture loader (committed fixture under `tests/fixtures/`, gated)
**Source:** `crates/nono/src/trust/mod.rs:74-81` `#[cfg(test)] load_test_trusted_root()` loads
`tests/fixtures/trust-root-frozen.json` via `CARGO_MANIFEST_DIR`; `crates/nono/tests/fixtures/`.
**Apply to:** the committed ECDSA P-256 test keypair (D-01) + the `vectors.json` snapshot must
live under `nono-py/tests/fixtures/`, loaded only in tests (Python `Path(__file__).parent /
"fixtures"` per RESEARCH Code Example; or a `#[cfg(test)]` Rust loader mirroring this gate). The
test pubkey injection path MUST stay test-gated — never a production trust anchor (D-01).

## No Analog Found

| File / Concern | Role | Data Flow | Reason |
|----------------|------|-----------|--------|
| `override.rs` :: low-S DER `{r,s}` parse + `s > n/2` reject | utility | transform | No ECDSA-malleability/low-S check exists in-tree. `der`/`x509-cert` in nono are for cert parsing. Use `der = "0.7"` (promote) OR hand-roll per ZT `_require_low_s`. |
| `override.rs` :: `NonoOverrideError` custom PyO3 exception | provider | request-response | Zero `create_exception!` usage anywhere in the repo (grep-confirmed). nono-py raises only built-in exceptions via `to_py_err`. This is the first custom exception — follow PyO3 0.28 `create_exception!` (RESEARCH Pattern 4); verify the macro + `m.add` line at impl (A2). |
| `override.rs` :: in-process `jti` consumed-set | store (ephemeral) | event-driven | No in-process replay/nonce store exists in nono-py. Build a `Mutex<HashSet<String>>` (D-03); intentionally NOT persisted (cross-process is Phase 93). |
| `override.rs` :: expiry/skew/TTL-cap (`chrono`) | utility | transform | No RFC3339 time-window verifier in-tree. Use `chrono::DateTime::parse_from_rfc3339` + `Utc::now()`; ±120s skew; `expires_at - not_before <= cap` (8h/24h). |

> For all four: the planner should use RESEARCH.md patterns (Pattern 3, Pattern 4, D-03,
> VFY-05) directly — they are spec-derived from the ZT reference impls, not from in-tree analogs.

## Metadata

**Analog search scope:** `nono-py/src/` (lib.rs, policy.rs, sandboxed_exec.rs,
windows_confined_run.rs, proxy.rs, undo.rs), `nono-py/tests/`, `nono-py/Cargo.toml`;
`crates/nono/src/trust/` (bundle.rs, mod.rs, digest.rs, Cargo.toml); sigstore-crypto-0.8.0 on
disk; ZT-Infra v2 `test-vectors/canonical-form/`.
**Files scanned:** ~14 (read) + grep across both repos.
**Key durable findings for the planner:**
- D-05 confirmed in-tree: `verify_keyed_signature` (bundle.rs:497) calls `vk.verify(&pae_bytes,..)`
  over PAE — WRONG. `verify_prehashed` (sigstore-crypto verification.rs:122) is the right call;
  `Sha256Hash`/`DerPublicKey`/`SignatureBytes`/`SigningScheme` all re-exported via `nono::trust`.
- `create_exception!` has NO precedent in the repo — this phase introduces the first one.
- `#[serde(deny_unknown_fields)]` is NOT used in policy.rs — override.rs deviates (fail-secure).
- `mod override;` is a Rust-keyword collision — the `mod` identifier needs escaping/renaming
  even though the FILE is `override.rs` (planner must flag; not present in any existing module).
- `#[pyclass(frozen)]` ExecResult is the exact `OverrideGrant` immutability analog (D-02).
**Pattern extraction date:** 2026-06-21
