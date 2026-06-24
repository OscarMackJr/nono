---
status: issues_found
phase: 91-signed-override-format-verification-core
depth: standard
reviewed_files: 4
critical: 0
warning: 4
info: 5
---

# Phase 91 Code Review — Signed Override Format + Verification Core

**Reviewed:** 2026-06-21
**Depth:** standard (adversarial; security-critical crypto module)
**Files reviewed:**
- `C:\Users\OMack\nono-py\src\override.rs` (production lines 1-924; test modules 930-2066 spot-checked)
- `C:\Users\OMack\nono-py\src\lib.rs`
- `C:\Users\OMack\nono-py\Cargo.toml`
- `C:\Users\OMack\nono-py\tests\fixtures\vectors.json`

**Status:** issues_found (0 Critical, 4 Warning, 5 Info)

## Summary of the security-critical pipeline (verified sound)

The core verification pipeline in `verify_override_impl` (lines 748-797) is well-constructed and the
highest-risk properties hold up under adversarial tracing:

- **ECDSA verify-over-digest is correct.** `verify_ecdsa_digest` (624-637) uses
  `VerificationKey::verify_prehashed` over a 32-byte SHA-256 of the canonical bytes; the test signer
  (1061-1113) signs `canonical_bytes` (aws-lc-rs hashes internally), so signer/verifier agree on the
  digested message. No raw-byte signature path; the digest is re-derived, never trusted from
  `current_hash` (which is stripped).
- **Low-S enforcement works and cannot be bypassed.** `require_low_s` (525-531) parses the real `s`
  from DER and rejects `s > n/2` (`Greater` only, so `s == n/2` is correctly allowed). It runs as an
  additional gate before `verify_prehashed`, and both paths decode the same base64, so a high-S
  attacker cannot hide `s`.
- **Algorithm pinning is exact** (`== "ECDSA_SHA_256"`, 460-466); `none`/empty/`RS256` all rejected
  with `AlgorithmMismatch`, and the pin runs before any signature work (step 3 < step 7).
- **ARN allowlist is exact-match** via `slice::contains` (472-481) — no `starts_with`/substring;
  tests confirm the substring case is rejected (1202-1209).
- **`deny_unknown_fields` protects the signed set.** Step 1 strict-parses the struct (rejecting
  unknown fields) before step 2 re-derives the canonical hash, so the hash always covers exactly the
  validated field set — no unsigned-field injection (T-91-01-UNKNOWN).
- **jti replay is atomic.** `check_and_consume_jti` (885-893) holds the `Mutex` guard across
  `contains`-then-`insert`; no TOCTOU. jti consumption happens *after* crypto verification, so an
  attacker cannot burn a victim's jti without a valid signature.
- **`OverrideGrant` is `#[pyclass(frozen)]`** with cloned getters — Python cannot widen scope
  post-verification (closes verify→apply TOCTOU).
- **Fail-closed.** Every production error path returns `Err(OverrideErrorKind::*)`; no `unwrap`/
  `expect`/`panic` in non-test code; `#[must_use]` on the verifier; the PyO3 wrapper maps every
  `Err` to `NonoOverrideError` with stable `args[0]` kind code (never a built-in / `None`).
- **No AWS SDK added** — `Cargo.toml` adds only `sha2`, `base64`, `chrono`, `sigstore-verify`,
  `aws-lc-rs` (a crypto primitive, not the AWS SDK). VFY-03 clause (b) satisfied; offline-only holds.

No defect rises to Critical. The findings below are robustness/quality gaps and forward-deferred
items.

## Findings

### WARNING

#### W-01 — `DateTime + Duration` skew arithmetic can panic instead of failing closed
**File:** `src\override.rs:872, 877` (`check_time_window`)
**Issue:** `now < nb - skew` and `now > exp + skew` use chrono's `Add`/`Sub<Duration>` for
`DateTime<Utc>`. In chrono 0.4.45 these operators **panic** on overflow (the non-panicking forms are
`checked_sub_signed`/`checked_add_signed`). `not_before` / `expires_at` are fully attacker-controlled
RFC3339 strings. The TTL-cap check (864-869) only bounds `exp - nb ≤ 8h`; it does not bound the
absolute magnitude of either timestamp, so a token whose `not_before`/`expires_at` parse near
chrono's representable bound can still overflow when `±120s` skew is applied, and a panic across the
PyO3 boundary is a process-level abort — not the clean fail-closed `Err` the module otherwise
guarantees (violates PITFALLS #1 fail-secure, CLAUDE.md "libraries should almost never panic").
**Why it matters:** A crafted token could turn a "deny" into an interpreter crash / DoS rather than a
catchable `NonoOverrideError`. Security-critical code must deny, not abort.
**Fix:** Use the checked variants and map overflow to an error:
```rust
let lower = nb.checked_sub_signed(skew).ok_or(OverrideErrorKind::Parse)?;
if now < lower { return Err(OverrideErrorKind::NotYetValid); }
let upper = exp.checked_add_signed(skew).ok_or(OverrideErrorKind::Parse)?;
if now > upper { return Err(OverrideErrorKind::Expired); }
```
(Also consider clamping accepted years at parse time.)

#### W-02 — Negative / inverted TTL (`expires_at < not_before`) is not explicitly rejected
**File:** `src\override.rs:864-869` (`check_time_window`)
**Issue:** `ttl = exp.signed_duration_since(nb).num_seconds()` is compared only with
`ttl > TTL_CAP_SECONDS`. When `expires_at < not_before`, `ttl` is negative, silently passes the cap
check, and the token's validity is left entirely to the two skew comparisons. A token with
`expires_at` earlier than `not_before` is logically malformed (it can never be both not-before-valid
and not-expired in a sane window) and should be rejected on its face rather than relying on
wall-clock coincidence.
**Why it matters:** Defense-in-depth / explicit-over-implicit (CLAUDE.md). An inverted window is a
sign of a malformed or adversarial token and should fail closed deterministically, independent of
`now`.
**Fix:** Add `if ttl < 0 { return Err(OverrideErrorKind::Parse); }` (or `Expired`) before the cap
check.

#### W-03 — jti is consumed before the final fallible step; ordering couples replay-burn to later failures
**File:** `src\override.rs:782-786` (`verify_override_impl`)
**Issue:** Step 9 (`check_and_consume_jti`) runs before step 10 (`partition_scope`). Today
`partition_scope` is infallible so no jti is wrongly burned, but the ordering is fragile: any future
fallible logic added at step 10+ (e.g. Phase 92 scope validation, the reserved `OutOfScope` path)
would consume the single-use jti and *then* return `Err`, permanently bricking a legitimate token on
a transient/format error. Single-use consumption should be the **last** action before returning
`Ok`.
**Why it matters:** A single-use nonce burned on a non-replay failure is a correctness/availability
bug and an easy regression to introduce in Phase 92.
**Fix:** Move `check_and_consume_jti` to immediately precede the `Ok(OverrideGrant { .. })`
construction (after all other validation), or split into `check_jti` (early) + `consume_jti`
(last). Document the invariant: "jti is consumed only on a fully successful verify."

#### W-04 — Lenient hand-rolled DER parser ignores trailing bytes and zero-length INTEGERs
**File:** `src\override.rs:535-588` (`parse_ecdsa_der_rs` / `parse_der_integer`)
**Issue:** The parser used for the low-S gate is intentionally minimal but is more permissive than a
strict ASN.1 decoder: (a) `if header_len + seq_len > der.len()` allows trailing bytes after the
SEQUENCE; (b) after parsing `s` it never asserts the SEQUENCE content was fully consumed; (c)
`parse_der_integer` accepts `len == 0` (empty INTEGER → empty value → treated as numerically `Less`
than n/2, silently passing low-S). None of these is exploitable for a high-S bypass today (the real
aws-lc-rs verifier rejects non-canonical/short DER, and low-S is an additional gate), so this is a
robustness defect rather than a vulnerability. But a "low-S OK" verdict on structurally invalid DER
is a latent footgun if the verify/low-S ordering ever changes or this parser is reused.
**Why it matters:** A signature-malleability gate should not silently accept malformed DER as
"low-S"; defense-in-depth wants the gate strict on its own.
**Fix:** Reject `len == 0` in `parse_der_integer`; require `rest` to be empty after parsing `s` in
`parse_ecdsa_der_rs`; reject trailing bytes (`header_len + seq_len != der.len()`). Add a malformed-S
test (currently only `garbage_der` and `high-S` are covered).

### INFO

#### I-01 — `repo_context: null` (explicit) causes `Parse` failure rather than being treated as absent
**File:** `src\override.rs:205-208, 360-361`
**Issue:** The struct treats absent `repo_context` as `None` via `#[serde(default)]`, but the
canonical re-parse to `Value` keeps an explicit `"repo_context": null`, which `write_value` rejects
(`Value::Null => Err(Parse)`). So `omitted` and `null` behave differently. This is fail-closed and
documented in `make_signed_test_token`, but is a surprising asymmetry for token producers.
**Fix:** Either document the "omit, never null" rule at the public `parse_token`/`verify_override`
surface, or strip top-level `null` optional fields before canonicalization to match the
"absent == null" intuition.

#### I-02 — Duplicate JSON keys are silently last-wins in both parse and canonicalization
**File:** `src\override.rs:382-391, 759`
**Issue:** `deny_unknown_fields` does not reject duplicate occurrences of a *known* key; serde_json
resolves duplicates last-wins in both the struct parse (step 1) and the `Value` parse (step 2). The
two are consistent so there is no hash/validation divergence, but a strict CAF producer might expect
duplicate keys to be rejected outright.
**Fix:** Optional: pre-scan for duplicate object keys and reject with `Parse` for strict CAF
conformance. Low priority — no security divergence exists.

#### I-03 — `__repr__` exposes the full `jti` and `expires_at`
**File:** `src\override.rs:705-713`
**Issue:** The repr is deliberately redaction-safe for key material (good), but prints the full `jti`
nonce and expiry. The jti is a single-use nonce, not a long-lived secret, so this is acceptable; just
confirm downstream logging policy is comfortable surfacing it.
**Fix:** None required; note for the Phase 92 logging review.

#### I-04 — `OverrideToken` field carries `#[allow(dead_code)]`; most fields unused in this phase
**File:** `src\override.rs:329` (`#[allow(dead_code)]` on `OverrideToken`)
**Issue:** CLAUDE.md discourages lazy `#[allow(dead_code)]`. Several `OverrideToken` fields (`actor`,
`action`, `decision`, `reason`, `timestamp`, `previous_hash`, `current_hash`) are parsed/canonicalized
but not otherwise read in Phase 91. This is justified (they are signature-covered and consumed by
Phase 92), but the broad crate-style allow is worth narrowing.
**Fix:** Prefer `#[expect(dead_code, reason = "...Phase 92...")]` (as already used for
`OutOfScope`) so the allow self-removes when the fields gain a reader, keeping parity with the
codebase's stated convention.

#### I-05 — Forward-deferred [BLOCKING-93] items are correctly scoped, not defects of this phase
**File:** `src\override.rs:323-328, 612-619, 742-746`
**Issue (note only):** Production pubkey sourcing (env/policy DER + per-`key_id` `VerificationKey`
cache) is `[BLOCKING-93]`-deferred; `pubkey_der` is a test-injected parameter here. The wire-shape
reconciliation with real KMS tokens is also deferred. Per the review brief these are intentional v1
boundaries and are *not* counted as defects of Phase 91. Flagged here for traceability so Phase 93
does not lose them. The `OutOfScope` variant (73-74) is similarly a documented Phase 92
forward-declaration.
**Fix:** None for Phase 91. Carry into Phase 93/92 acceptance gates.

## Summary

The Phase 91 override-verification core is a solid, fail-closed implementation: ECDSA-over-digest is
correct, explicit low-S rejection is sound and unbypassable, algorithm pinning and the ARN allowlist
are exact-match, `deny_unknown_fields` binds the canonical hash to exactly the validated field set,
jti replay is atomic and gated behind crypto verification, and `OverrideGrant` is frozen to close the
verify→apply TOCTOU. No AWS SDK was added and the offline-only invariant holds. There are **zero
Critical findings.** The four Warnings are robustness/fail-closed hardening: a chrono skew-arithmetic
panic that should be a catchable `Err` (W-01), an unrejected inverted/negative TTL window (W-02), a
jti-consume ordering that is safe today but fragile for Phase 92 (W-03), and a lenient DER parser in
the low-S gate that should be made strict on its own (W-04). None of these is currently exploitable
for an auth bypass, but W-01 (panic-as-deny) and W-02/W-03 should be fixed before this code is relied
on as the trust anchor for live overrides.
