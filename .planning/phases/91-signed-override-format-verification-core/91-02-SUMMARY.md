---
phase: 91-signed-override-format-verification-core
plan: "02"
subsystem: nono-py/override
tags: [ecdsa, verification, low-s, tdd, override-token, zt-infra, jti-replay, vfy-02, vfy-03, vfy-04, vfy-05]
dependency_graph:
  requires:
    - nono-py/src/override.rs (OverrideErrorKind, canonical_bytes, canonical_sha256, OverrideToken, parse_token) — from Plan 01
  provides:
    - nono-py/src/override.rs (verify_override, OverrideGrant, require_low_s, require_algorithm_pin, require_arn_allowlist, verify_ecdsa_digest, check_time_window, check_and_consume_jti, partition_scope, normalize_low_s in test_sign)
    - nono-py/tests/fixtures/override_test_key.pem (PKCS8 PEM test private key)
    - nono-py/tests/fixtures/override_test_key.der (SPKI DER test public key)
  affects:
    - nono-py/Cargo.toml (sigstore-verify = "0.8" + aws-lc-rs = "1" promoted to direct deps)
    - nono-py/.gitignore (override_test_private.der + python pdb excluded)
tech_stack:
  added:
    - sigstore-verify = "0.8" (VerificationKey::verify_prehashed for ECDSA P-256 KMS DIGEST mode)
    - aws-lc-rs = "1" (EcdsaKeyPair::from_pkcs8 for test signing; ECDSA_P256_SHA256_ASN1_SIGNING)
  patterns:
    - "verify_prehashed(Sha256Hash, sig) NOT verify_keyed_signature (D-05: DSSE/PAE wrong for KMS DIGEST)"
    - "explicit low-S: hand-rolled DER {r,s} parser + s > P256_ORDER_HALF reject (aws-lc-rs verify does NOT enforce)"
    - "test_sign::normalize_low_s: s' = P256_ORDER - s (big-endian subtraction) when aws-lc-rs signing produces high-S"
    - "partition_scope: starts_with('/') not std::path::is_absolute() (Windows compatibility)"
    - "CONSUMED_JTIS: Mutex<Option<HashSet<String>>> static (in-process v1 replay set, D-03)"
    - "OverrideGrant #[derive(PartialEq, Eq)] for assert_eq! in tests"
key_files:
  created:
    - nono-py/tests/fixtures/override_test_key.pem
    - nono-py/tests/fixtures/override_test_key.der
  modified:
    - nono-py/src/override.rs (1080+ insertions: full verify pipeline + tests)
    - nono-py/Cargo.toml (2 dep promotions)
    - nono-py/.gitignore (exclude private DER + pdb)
decisions:
  - "verify_prehashed NOT verify_keyed_signature: KMS DIGEST mode — sign operates over SHA256(canonical), verify uses the pre-computed digest; DSSE/PAE PAE() wrapper in verify_keyed_signature is wrong (D-05)"
  - "normalize_low_s in test_sign: aws-lc-rs ECDSA_P256_SHA256_ASN1_SIGNING does NOT guarantee low-S; test helper normalizes s' = P256_ORDER - s to match require_low_s gate in production verification"
  - "partition_scope uses starts_with('/') not is_absolute(): std::path::Path::is_absolute() returns false for /unix/path on Windows (drive-letter required); Unix-convention paths are the token wire format"
  - "VFY-03 PARTIAL: clause (b) satisfied (zero AWS SDK in Cargo.toml); clause (a) [BLOCKING-93] deferred to Phase 93 pubkey sourcing"
  - "OverrideGrant #[derive(PartialEq, Eq)]: required for assert_eq!(result, Err(...)) in tests; OverrideGrant is the Ok variant"
metrics:
  duration_minutes: 70
  completed_date: "2026-06-21T23:00:00Z"
  tasks_completed: 2
  files_created: 2
  files_modified: 3
---

# Phase 91 Plan 02: ECDSA Verify Pipeline + OverrideGrant + jti Replay Summary

10-step offline verification pipeline over CAF v0.1 canonical SHA-256 digest: algorithm pin -> ARN allowlist -> base64 decode -> explicit low-S -> ECDSA P-256 verify_prehashed -> expiry/skew/TTL-cap -> in-process jti replay. Returns immutable OverrideGrant on success; every failure path returns Err(OverrideErrorKind). All 39 nono-py tests green.

## Tasks Completed

| Task | Name | Commit (nono-py) | Files |
|------|------|--------|-------|
| 1+2 | ECDSA verify pipeline + OverrideGrant + jti replay | 61d9811 | nono-py/src/override.rs, Cargo.toml, tests/fixtures/override_test_key.{pem,der}, .gitignore |

Note: Tasks 1 and 2 were committed together because the low-S normalization fix (a Task 1 deviation) and the `partition_scope` Windows fix (a Task 2 deviation) were both discovered during green-phase iteration and interleaved with the test modules. The TDD contract is met — the RED phase tests drove both implementation passes.

## Outcome

- SC2 MET: `cargo test -p nono-py override_mod::sig` passes all 14 tests (algorithm pin, ARN allowlist, base64, low-S, verify_prehashed).
- SC3 MET: `cargo test -p nono-py override_mod::verify` passes all 9 tests + `override_mod::replay` passes 2 tests.
- SC4: `[BLOCKING-93] VFY-03 clause (a)` comment present and grep-assertable in override.rs.
- `#[must_use = "verify_override Result must be checked -- dropping it silently may grant access"]` on `verify_override`.
- `verify_prehashed` is the cryptographic primitive (6 occurrences in override.rs).
- `verify_keyed_signature` does NOT appear as a call (only in a comment noting it is wrong: "NOT verify_keyed_signature").
- All 39 nono-py tests pass: 39/0 passed/failed.
- `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` clean.
- VFY-03 PARTIAL: clause (b) satisfied (Cargo.toml has zero AWS SDK); clause (a) pubkey source is `[BLOCKING-93]` deferred to Phase 93.

## Requirement Coverage

| Requirement | Status |
|-------------|--------|
| VFY-02 (ECDSA P-256, explicit low-S) | DONE — require_low_s + verify_prehashed |
| VFY-03 (no in-process AWS creds) | PARTIAL — clause (b) done; clause (a) [BLOCKING-93] Phase 93 |
| VFY-04 (ARN allowlist exact match) | DONE — require_arn_allowlist using slice::contains (exact element equality) |
| VFY-05 (expiry + skew + TTL cap) | DONE — check_time_window (+-120s skew, 8h TTL cap) |
| VFY-06 (jti single-use) | DONE — check_and_consume_jti (in-process Mutex<HashSet>) |
| VFY-07 (OverrideGrant immutable) | DONE — OverrideGrant struct (no interior mutability; Clone only) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] aws-lc-rs produces high-S signatures — require_low_s correctly rejects them**
- **Found during:** Task 1 green phase — `valid_signature_verifies_ok` failing with `Err(BadSignature)`
- **Issue:** `ECDSA_P256_SHA256_ASN1_SIGNING` signing does NOT guarantee low-S output (RFC 6979 ensures deterministic nonce, not low-S). Our `require_low_s` correctly rejects high-S sigs, but the test signing helper did not normalize.
- **Fix:** Added `normalize_low_s(sig_der: &[u8]) -> Result<Vec<u8>>` in `test_sign` module: parse DER to get (r, s); if `s > P256_ORDER_HALF`, compute `s' = P256_ORDER - s` via big-endian subtraction (`subtract_be_32`); re-encode DER via `encode_ecdsa_der`. Both `(r, s)` and `(r, s')` are valid ECDSA signatures; the verifier (`ECDSA_P256_SHA256_ASN1` used in `verify_prehashed`) accepts both.
- **Files modified:** `nono-py/src/override.rs` (`test_sign` module: `normalize_low_s`, `subtract_be_32`, `encode_ecdsa_der`; `P256_ORDER` const)
- **Commit:** 61d9811

**2. [Rule 1 - Bug] partition_scope used std::path::is_absolute() — returns false for /unix/path on Windows**
- **Found during:** Task 2 green phase — `valid_token_returns_ok_grant` returning `Err(Parse)` after all other steps succeeded
- **Issue:** `std::path::Path::new("/tmp/test").is_absolute()` returns `false` on Windows (requires `C:\` or `\\server\share` prefix). Override tokens use Unix-convention absolute paths (starting with `/`). The Windows dev host caused false rejects.
- **Fix:** Changed `partition_scope` to use `entry.starts_with('/')` check instead. Added doc comment explaining the decision.
- **Files modified:** `nono-py/src/override.rs` (partition_scope function + doc comment)
- **Commit:** 61d9811

**3. [Rule 3 - Blocking] OverrideGrant missing PartialEq for assert_eq! in tests**
- **Found during:** Task 2 test compilation
- **Issue:** `assert_eq!(result, Err(OverrideErrorKind::BadSignature))` where `result: Result<OverrideGrant, _>` requires `OverrideGrant: PartialEq`.
- **Fix:** Added `#[derive(PartialEq, Eq)]` to `OverrideGrant`.
- **Commit:** 61d9811

**4. [Rule 2 - Security] .gitignore: exclude override_test_private.der + python pdb files**
- **Found during:** Task 1 fixture creation — openssl genpkey also produced a DER private key file
- **Issue:** `tests/fixtures/override_test_private.der` appeared untracked; leaving it untracked risks accidental commit in the future.
- **Fix:** Added both patterns to `.gitignore`.
- **Commit:** 61d9811

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced. `verify_override` is a pure in-memory verification function. The `CONSUMED_JTIS` static adds a process-lifetime in-memory replay set — this is the deliberate v1 in-process boundary per D-03.

T-91-02-FAILOPEN: Every `Err` path in `verify_override` denies. No path returns `Ok` with a deny flag, `None`, or `false`. `#[must_use]` prevents silent drop.
T-91-02-ALGNONE: `require_algorithm_pin` rejects `"none"`, empty string, and any value other than `"ECDSA_SHA_256"` with `Err(AlgorithmMismatch)`. Tests confirm.
T-91-02-ARN: `require_arn_allowlist` uses `allowed_arns.contains(&key_id)` — slice element equality, NOT substring match. `arn_substring_of_allowed_arn_rejected` test confirms.
T-91-02-LOWS: `require_low_s` hand-parses DER to extract `s`, compares to `P256_ORDER_HALF` byte-by-byte. `high_s_signature_rejected` test confirms.
T-91-02-TOCTOU: `OverrideGrant` is an immutable struct (no `Cell`, no `RefCell`, no `Mutex` fields).
T-91-02-VFY03: `[BLOCKING-93] VFY-03 clause (a)` comment embedded in `verify_ecdsa_digest` doc — grep-assertable.

## Known Stubs

- **VFY-03 clause (a):** `pubkey_der` param is test-injected. Production pubkey sourcing from embedded machine-policy or `env://` DER+base64 with per-`key_id` VerificationKey caching is `[BLOCKING-93]` deferred to Phase 93.
- **jti cross-process:** `CONSUMED_JTIS` is in-process only. Cross-process + persistent single-use enforcement deferred to Phase 93 (D-03 deliberate v1 boundary).
- **Scope enforcement:** `partition_scope` validates format but does NOT enforce scope against sandbox policy. Full enforcement is Phase 92.
- **Operator TTL config:** `TTL_CAP_SECONDS = 8h` is hard-coded. 24h CI cap deferred to Phase 93.

## Self-Check: PASSED

Commits in nono-py git log (branch 44-broker-ffi-lockstep):
- `14aafc1` feat(91-01): scaffold override.rs module...
- `a6128fb` feat(91-01): implement canonical_bytes/sha256...
- `61d9811` feat(91-02): implement ECDSA verify pipeline + OverrideGrant + jti replay

Test result: `39 passed; 0 failed` (cargo test -p nono-py, 2026-06-21)
Clippy: clean (cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used)
