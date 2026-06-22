---
phase: 91
slug: signed-override-format-verification-core
status: verified
threats_open: 0
asvs_level: 2
created: 2026-06-22
---

# Phase 91 — Security

> Per-phase security contract for the offline, fail-closed ZT-Infra CAF v0.1 override-token
> verifier. Threat register authored at plan time (3 plans / 22 threats), verified against the
> live implementation in the separate `nono-py` repo (`C:\Users\OMack\nono-py`,
> branch `44-broker-ffi-lockstep`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| untrusted token JSON → `override.rs` parser | Attacker-controlled JSON (key order, whitespace, extra fields, malformed leaves) enters the canonicalizer | Untrusted bytes |
| received bytes → canonical digest | The signed digest MUST derive from the parsed struct, never the raw received bytes | Re-derived SHA-256 |
| crypto-valid signature → authorization | A valid signature from a non-allowlisted key MUST NOT authorize | Identity / key ARN |
| offline verify → (Phase 93) live AND-gate | The verified `OverrideGrant` value (not the raw token) crosses into the live check — TOCTOU boundary | Immutable grant |
| Rust verifier `Result` → Python caller | Every `Err` must surface as a raised exception, never a falsy/`None` return treated as "allow" | PyResult / exception |
| `OverrideGrant` → Python | Grant must be immutable from Python so a caller cannot widen scope after verification | Frozen pyclass |
| exception message → logs | Raised message must not leak raw signature/key bytes | Redaction-safe string |

---

## Threat Register

All 22 plan-time threats verified against the implementation (`register_authored_at_plan_time: true` →
verify-mitigations mode). Evidence is file:line in `nono-py/src/override.rs` (or `lib.rs`) and/or test name.

| Threat ID | Category | Component | Disposition | Mitigation (verified evidence) | Status |
|-----------|----------|-----------|-------------|--------------------------------|--------|
| T-91-01-CANON | Tampering | `canonical_bytes` re-derivation | mitigate | Re-derived from parsed `Value` (BTreeMap code-point sort + strip `current_hash`/`kms_signature`); never `serde_json::to_string`/`sha256(raw)`. 9/9 ZT vectors byte-exact (`all_vectors_canonical_bytes_and_sha256`). | closed |
| T-91-01-UNKNOWN | Tampering | `OverrideToken` serde model | mitigate | `#[serde(deny_unknown_fields)]` (override.rs:307,331). Test `unknown_field_rejected_parse` → `Err{Parse}`. | closed |
| T-91-01-MISSING | Spoofing | required signed fields | mitigate | Required fields non-`Option`; `key_id` required non-empty. Tests `missing_jti_*`/`missing_expires_at_* → MissingField`, `absent_key_id_rejected → Parse`. | closed |
| T-91-01-FLOAT | Tampering | canonicalize leaf validation | mitigate | Rejects float/null/bool leaves, out-of-range ints, ctrl/U+2028/U+2029/supra-BMP chars → `Err{Parse}`. Tests `float_leaf_rejected`, `control_char_in_string_rejected`, `integer_out_of_range_rejected`, `supra_bmp_char_rejected`, `line_separator_rejected`. | closed |
| T-91-01-WIRESHAPE | Tampering | nono-side token wire shape (D-06) | **accept (v1)** | Wire shape signed by LOCAL TEST KEYPAIR; real-KMS reconciliation `[BLOCKING]`-marked for Phase 93 (override.rs:323). Deliberate, tracked v1 boundary. See Accepted Risks. | closed |
| T-91-01-SC | Tampering (supply chain) | cargo dep promotions | mitigate | Added deps are promotions of crates already in the dependency graph — parent `nono` crate already pins `sigstore-verify = "0.8.0"`; `aws-lc-rs`/`sha2`/`base64`/`chrono` are established, ubiquitous crates. Zero typosquat/hallucination surface; build clean. | closed |
| T-91-02-FAILOPEN | Elevation of Privilege | `verify_override` error paths | mitigate | `#[must_use]` on `verify_override_impl` (override.rs:747); every step propagates `Err` via `?`; no `unwrap`/`expect`/`panic` in production code (verifier-confirmed). Deny-asserting test per failure variant. | closed |
| T-91-02-ALGNONE | Spoofing/Tampering | algorithm pin | mitigate | `require_algorithm_pin` exact `== "ECDSA_SHA_256"` BEFORE any signature work (override.rs:460-465,765). Tests `algorithm_none_*`/`non_ecdsa_* → AlgorithmMismatch`. | closed |
| T-91-02-HIGHS | Tampering | low-S enforcement | mitigate | Explicit `require_low_s` parses DER `{r,s}`, rejects `s > P256_ORDER_HALF` → `BadSignature` (override.rs:525-531); aws-lc-rs does not enforce it. | closed |
| T-91-02-WRONGPRIM | Tampering | crypto primitive | mitigate | `VerificationKey::verify_prehashed` over the 32-byte digest (override.rs:635); `verify_keyed_signature` absent from code (only a "NOT …" comment). | closed |
| T-91-02-ARN | Spoofing | key-ARN allowlist | mitigate | `allowed_arns.contains(&key_id)` — exact `==`, no substring/`starts_with` (override.rs:476). Test `arn_not_in_allowlist_* → KeyNotAllowlisted`. | closed |
| T-91-02-EXPIRY | Tampering | expiry/skew/TTL | mitigate | RFC3339 + `Utc::now()` ±120s skew; `expires_at-not_before ≤ TTL_CAP`. **W-01 hardened (commit `a0b5eec`)**: skew math now `checked_add_signed`/`checked_sub_signed` → `Err` (no panic on overflow); regression test `extreme_timestamps_deny_without_panic`. | closed |
| T-91-02-REPLAY | Replay | in-process jti set | mitigate | `Mutex<HashSet>` check-then-insert under one lock (override.rs:885-893). **W-03 hardened (`a0b5eec`)**: consume moved to LAST fallible step before `Ok`. Test `jti_replay_rejected_on_second_verify`. | closed |
| T-91-02-PUBKEYSRC | Spoofing | VFY-03 pubkey source | **mitigate (partial)** | Clause (b) satisfied now: no in-process AWS SDK / no AWS creds (Cargo.toml has zero `aws-sdk`/`aws-config`/`rusoto`). Clause (a) env/policy sourcing `[BLOCKING-93]`-marked (override.rs:612,742,835), operator-sanctioned for Phase 93. See Accepted Risks. | closed |
| T-91-02-TOCTOU | Tampering | `OverrideGrant` value | mitigate | `verify_override` returns an immutable grant carrying already-checked fields; Phase 93 consumes the value, never re-parses (override.rs:643-648). | closed |
| T-91-02-DIGEST | Tampering | digest source | mitigate | Signature checked against `canonical_sha256(parsed, strip)` (override.rs:759-762), never `sha256(raw)`/`current_hash`. | closed |
| T-91-02-SC | Tampering (supply chain) | der/base64 dep | mitigate | Low-S DER parse is hand-rolled (zero new dep, override.rs:535-588); `base64`/`sigstore-verify` are promotions of graph-resident crates. No net-new registry package. | closed |
| T-91-03-FALSY | Elevation of Privilege | PyO3 boundary return | mitigate | `#[pyfunction]` returns `PyResult`; every `Err` → raised `NonoOverrideError` via `override_err_to_py` (override.rs:837-850); no `None`/`False`/`Ok`-with-deny. 11 `pyo3_boundary` tests. | closed |
| T-91-03-WRONGEXC | Spoofing | exception type | mitigate | Single dedicated `NonoOverrideError` (`create_exception!`), not a built-in; `args[0]` = stable kind string for the Phase 92 EventID-10008 map (D-04). | closed |
| T-91-03-MUTGRANT | Tampering | grant immutability | mitigate | `#[pyclass(frozen)]` (override.rs:663) — Python cannot mutate the grant to widen scope. | closed |
| T-91-03-LEAK | Information Disclosure | exception message | mitigate | `override_err_to_py` message uses `kind.as_str()` only (no raw sig/key bytes); `__repr__` shows jti+expiry, not key ARN material (override.rs:658-662). | closed |
| T-91-03-MUSTUSE | Elevation of Privilege | ignored Result | mitigate | `#[must_use]` on `verify_override_impl` (override.rs:747) + SC5 negative-control documented; PyO3 `PyResult` forces Python-side checking. | closed |

*Status: open · closed* — *Disposition: mitigate · accept · transfer*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-91-1 | T-91-01-WIRESHAPE | The override wire shape is a nono-side construct signed by a local test keypair this phase. Reconciliation with real KMS-issued token shape is `[BLOCKING-93]`-tracked; Phase 93 acceptance gates require it. Deliberate offline-only v1 boundary, not a silent gap. | Operator (oscar.mack.jr) | 2026-06-22 |
| AR-91-2 | T-91-02-PUBKEYSRC | VFY-03 clause (a) — production pubkey sourcing from env/policy DER+base64 + per-`key_id` `VerificationKey` cache + live ARN allowlist wiring — is sanctioned PARTIAL, deferred to Phase 93 (`[BLOCKING-93]`). Clause (b) (no AWS SDK / no creds) is satisfied and verified now. | Operator (oscar.mack.jr) | 2026-06-22 |
| AR-91-3 | (code-review W-02) | Inverted-TTL (`expires_at < not_before`) can pass the TTL-cap check within a narrow skew window. Non-blocking, not an auth bypass (time-window checks still gate normal cases). Carry to Phase 92/93 hardening. | Operator (oscar.mack.jr) | 2026-06-22 |
| AR-91-4 | (code-review W-04) | The hand-rolled low-S DER parser accepts trailing bytes / zero-length INTEGERs. Not exploitable today (aws-lc-rs verify is strict on the full signature); the malleability gate should be made strict on its own. Carry to Phase 92/93 hardening. | Operator (oscar.mack.jr) | 2026-06-22 |

*Accepted risks do not resurface in future audit runs. AR-91-1/-2 are sanctioned Phase-93 scope; AR-91-3/-4 are non-blocking hardening items.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-22 | 22 | 22 | 0 | gsd-secure-phase (inline audit; gsd-security-auditor spawn blocked by API 529) |

**Audit notes:** Core verification is cryptographically sound — none of the highest-risk surfaces
(signature bypass, low-S malleability, algorithm downgrade, ARN substring bypass, unsigned-field
injection, jti replay TOCTOU, grant mutation, fail-open) is exploitable. W-01/W-03 (the two
fail-secure warnings the code review recommended fixing before live trust-anchoring) were fixed in
nono-py commit `a0b5eec` during phase close. 49 override tests pass; clippy
`-D warnings -D clippy::unwrap_used` clean. No AWS SDK / no credentials in the offline verifier
(VFY-03 clause (b) verified).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-22
