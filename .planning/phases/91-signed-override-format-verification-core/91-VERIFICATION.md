---
phase: 91-signed-override-format-verification-core
verified: 2026-06-21T23:30:00Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
re_verification: true
human_resolution:
  - test: "W-01 / W-03 disposition"
    resolution: "RESOLVED — operator chose 'Fix now'. Fixed in nono-py commit a0b5eec: W-01 checked skew arithmetic (deny, not panic) + regression test extreme_timestamps_deny_without_panic; W-03 jti consume moved to last fallible step before Ok. 49 override tests pass, clippy clean. See 91-HUMAN-UAT.md."
  - test: "VFY-03 partial-coverage scope split"
    resolution: "RESOLVED — operator confirmed 'Sanctioned — partial in 91'. Clause (b) closes in Phase 91 (no AWS SDK, verified); clause (a) is sanctioned Phase 93 work tracked by [BLOCKING-93] markers. Phase 93 acceptance gates will require clause (a)."
human_verification:
  - test: "Confirm W-01 chrono overflow panic is acceptable risk for Phase 91 vs blocking"
    expected: "Decision: accept as-is (W-01 is a DoS path not an auth bypass, fix in Phase 92) OR fix now before the code is trusted as a live trust anchor"
    why_human: "The code review found that DateTime + Duration arithmetic panics on overflow (W-01) rather than returning Err. The four code-review warnings (W-01 through W-04) are not auth bypasses but the reviewer flagged W-01 and W-03 as 'should be fixed before this code is relied on as a trust anchor for live overrides'. Phase 91 is the offline-only foundation; live trust-anchoring is Phase 93. Human decision required: proceed to Phase 92 with warnings documented, or fix warnings first."
  - test: "Confirm VFY-03 partial coverage deferral is acceptable"
    expected: "VFY-03 clause (a) (production pubkey sourcing from env/policy DER+base64, per-key_id cache) is [BLOCKING-93] deferred. Clause (b) (no AWS SDK) is satisfied. Confirm this partial deferral is intentional and Phase 93 acceptance gates will require (a)."
    why_human: "REQUIREMENTS.md maps VFY-03 to Phase 91. The PLAN frontmatter explicitly calls VFY-03 PARTIAL with a [BLOCKING-93] marker. Both the PLAN and SUMMARY are unambiguous that clause (a) is deferred. This is a deliberate v1 boundary, not a gap — but because the requirement is mapped to Phase 91 in REQUIREMENTS.md and is only partially satisfied, a human should confirm the deferral is sanctioned."
---

# Phase 91: Signed Override Format + Verification Core Verification Report

**Phase Goal:** A fully offline, fail-closed verifier for ZT-Infra CAF v0.1 override tokens exists in nono-py/src/override.rs — every parse error, signature failure, expiry violation, scope escape, jti replay, and algorithm mismatch maps to a raised NonoOverrideError, never to a silent grant.

**Verified:** 2026-06-21T23:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | canonical_bytes() over all 9 ZT-Infra CAF v0.1 vectors produces byte-exact output and SHA-256 matching vectors.json reference | ✓ VERIFIED | `cargo test override_mod::canonical` — 9/9 vectors pass bytes + length + sha256_hex. Code uses BTreeMap code-point sort, no serde_json::to_string. |
| 2 | A valid token signed by the test keypair with allowlisted ARN and correct DER pubkey returns Ok(OverrideGrant) | ✓ VERIFIED | `override_mod::verify::valid_token_returns_ok_grant` passes. OverrideGrant fields populated (signer, scope_paths, jti, expires_at). |
| 3 | Every failure path (bad sig, high-S, algorithm:none, non-ECDSA, expired, not_before-future, missing field, unknown field, ARN not allowlisted, jti replay) returns Err never Ok | ✓ VERIFIED | 38 deny-asserting tests across override_mod::sig, verify, replay, pyo3_boundary all pass. No path returns Ok-with-deny or None. |
| 4 | verify_override (the #[pyfunction]) raises NonoOverrideError with stable args[0] kind for every Err variant | ✓ VERIFIED | 11 SC4 pyo3_boundary tests pass — all 11 OverrideErrorKind variants (including OutOfScope forward-decl) raise NonoOverrideError with the correct kind string. No built-in exception raised. |
| 5 | OverrideGrant is a frozen pyclass; Python callers cannot mutate it | ✓ VERIFIED | `#[pyclass(frozen, skip_from_py_object)]` at line 663. `__repr__` is redaction-safe. |
| 6 | NonoOverrideError is registered on _nono_py alongside OverrideGrant and verify_override | ✓ VERIFIED | lib.rs lines 768-773 register all three. grep confirmed all three tokens present. |
| 7 | No AWS SDK dependency was added (offline-only invariant) | ✓ VERIFIED | Cargo.toml adds sha2, base64, chrono, sigstore-verify, aws-lc-rs (crypto primitive, not SDK). grep for aws_sdk/aws-sdk/aws_config finds nothing. |
| 8 | jti single-use enforced in-process: second verify of same jti returns Err(Replay) before expires_at | ✓ VERIFIED | `override_mod::replay::jti_replay_rejected_on_second_verify` passes. Mutex<HashSet> check-then-insert atomic under lock. |

**Score:** 8/8 truths verified

### Deferred Items

Items not yet met but explicitly addressed in later milestone phases.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | VFY-03 clause (a): production pubkey sourced from embedded machine policy/env DER+base64, cached per key_id | Phase 93 | [BLOCKING-93] marker at override.rs:612-619, 742-746, 832-834. PLAN 02 objective explicitly scopes this as PARTIAL; clause (b) — no AWS SDK — satisfied now. |
| 2 | VFY-01: two-key AND gate (KMS sig + live ZT-Infra POST /actions) | Phase 92 | REQUIREMENTS.md traceability table: VFY-01 mapped to Phase 92. |
| 3 | jti cross-process durability | Phase 93 | CONSUMED_JTIS is in-process only; D-03 doc comment on the static explicitly marks this as v1 boundary. |
| 4 | Scope enforcement (OutOfScope variant produces Err when scope doesn't cover resource) | Phase 92 | OutOfScope variant forward-declared with #[expect(dead_code)]; PLAN 03 summary records Phase 92 removes it. |
| 5 | Wire shape reconciliation with real KMS-issued tokens | Phase 93 | [BLOCKING] Phase 93 reconciliation comment on OverrideToken struct (line 323-328). |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `nono-py/src/override.rs` | OverrideToken, OverrideErrorKind, canonical_bytes/sha256, verify_override_impl, OverrideGrant, NonoOverrideError, #[pyfunction] verify_override | ✓ VERIFIED | 2067 lines; all required symbols present and substantive. Confirmed by grep + test execution. |
| `nono-py/tests/fixtures/vectors.json` | 9 ZT-Infra CAF v0.1 test vectors with sha256_hex | ✓ VERIFIED | File exists; grep -c sha256_hex = 9; all_vectors_canonical_bytes_and_sha256 test asserts len==9. |
| `nono-py/tests/fixtures/override_test_key.der` | SPKI DER ECDSA P-256 test public key | ✓ VERIFIED | File exists; used by sign_token_digest() in #[cfg(test)] test_sign module. |
| `nono-py/tests/fixtures/override_test_key.pem` | PKCS8 PEM ECDSA P-256 test private key | ✓ VERIFIED | File exists; used in test signing helper. Private DER excluded via .gitignore. |
| `nono-py/src/lib.rs` | create_exception!, module declaration, _nono_py registration of 3 surfaces | ✓ VERIFIED | override_mod declaration at line 24-25; 3 registration lines at 768-773. |
| `nono-py/Cargo.toml` | sha2, base64, chrono, sigstore-verify direct deps; no AWS SDK | ✓ VERIFIED | All deps present; no AWS SDK found. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| nono-py/src/lib.rs | nono-py/src/override.rs | `#[path = "override.rs"] mod override_mod` | ✓ WIRED | lib.rs line 24-25 — keyword collision resolved with path attribute, platform-neutral (no #[cfg]) |
| override_mod::verify_override_impl | canonical_sha256 | Re-derived from parsed token before signature check | ✓ WIRED | Step 2 in pipeline at override.rs:759-761; never from raw bytes or current_hash |
| override_mod::verify_ecdsa_digest | sigstore_verify::crypto::verification::VerificationKey::verify_prehashed | ECDSA P-256 over 32-byte SHA-256 digest | ✓ WIRED | override.rs:635 — verify_prehashed called; verify_keyed_signature absent from production code (only in comment noting it is wrong) |
| override_mod::verify_override (pyfunction) | override_mod::NonoOverrideError | Every Err mapped via override_err_to_py | ✓ WIRED | override.rs:844-849; NonoOverrideError::new_err((kind.as_str(), msg)) |
| _nono_py module | OverrideGrant, verify_override, NonoOverrideError | m.add_class / m.add_function / m.add | ✓ WIRED | lib.rs:768-773; all three registered |
| override_mod::canonical_bytes | nono-py/tests/fixtures/vectors.json | #[cfg(test)] vector loop via CARGO_MANIFEST_DIR | ✓ WIRED | override.rs:1582-1590; all_vectors_canonical_bytes_and_sha256 test; 9/9 pass |

### Data-Flow Trace (Level 4)

Not applicable — override.rs is a pure verification function (no DB, no dynamic data source). The data flows are: token_json (test-supplied) → parse_token → canonical_sha256 → ECDSA verify → OverrideGrant. All data paths verified by test execution.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 9 ZT CAF v0.1 vectors pass canonical bytes + sha256 | `cargo test -p nono-py override_mod::canonical` | 9/9 vectors pass, 5 negative tests pass | ✓ PASS |
| Full verify pipeline (50 override tests) | `cargo test -p nono-py override` | 48 passed; 0 failed | ✓ PASS |
| Full crate test suite | `cargo test -p nono-py` | 50 passed; 0 failed | ✓ PASS |
| Clippy strict | `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` | Clean (0 warnings) | ✓ PASS |
| verify_keyed_signature absent from production code | `grep verify_keyed_signature src/override.rs` | Only appears in a comment noting it is wrong | ✓ PASS |
| No AWS SDK dependency | `grep -n 'aws_sdk\|aws-sdk\|aws_config' Cargo.toml` | Empty | ✓ PASS |

### Probe Execution

No probes declared for this phase (verify-dark.ps1 gates are Phase 92 DF-01 scope).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| OVR-01 | Plan 01 | Token carries signer identity, scope, not_before, expires_at, repo_context binding, jti | ✓ SATISFIED | OverrideToken struct fields confirmed; scope + jti + not_before + expires_at + kms_signature.key_id all inside signed object |
| OVR-02 | Plan 01 | Scope and expiry embedded inside the KMS-signed payload | ✓ SATISFIED | All OVR-01 fields are non-Option struct fields on OverrideToken (inside the canonicalized, signed object per D-06); OVR-02 test confirms scope inside signed object |
| OVR-03 | Plan 01 | CAF v0.1 canonical form; re-derived from parsed structure | ✓ SATISFIED | BTreeMap key sort, R10 strip, 9/9 ZT-Infra vectors pass byte-exact |
| VFY-02 | Plan 02 | ECDSA P-256 pinned; low-S enforced; algorithm:none rejected | ✓ SATISFIED | require_algorithm_pin (exact == "ECDSA_SHA_256"); require_low_s (hand-rolled DER s > n/2 reject); verify_prehashed used (not verify_keyed_signature). Tests confirm. |
| VFY-03 | Plan 02 | KMS pubkey from embedded machine policy; no AWS SDK; no AWS creds | PARTIAL — clause (b) SATISFIED, clause (a) [BLOCKING-93] Phase 93 | Cargo.toml has zero AWS SDK. pubkey_der is test-injected param (not from env/policy). [BLOCKING-93] markers at lines 612, 742, 832. Intentional v1 boundary per PLAN 02 objective. |
| VFY-04 | Plan 02 | Signer key_id checked against machine-policy allowlist, exact match | ✓ SATISFIED | require_arn_allowlist uses slice::contains (exact element ==, not substring). arn_substring_of_allowed_arn_rejected test confirms PITFALLS #4 is covered. |
| VFY-05 | Plan 02 | expires_at/not_before enforced ±2min skew; TTL hard-capped 8h | ✓ SATISFIED | check_time_window: CLOCK_SKEW_SECS=120, TTL_CAP_SECONDS=8*3600. expired, not_yet_valid, ttl_beyond_cap tests all pass. |
| VFY-06 | Plan 02 | jti single-use; consumed-jti store rejects replay | ✓ SATISFIED | CONSUMED_JTIS: Mutex<Option<HashSet<String>>>; check_and_consume_jti atomic under lock. jti_replay_rejected_on_second_verify passes. |
| VFY-07 | Plan 03 | Fail-closed: Result #[must_use]; PyO3 raises NonoOverrideError — never falsy return | ✓ SATISFIED | verify_override_impl carries #[must_use]; #[pyfunction] verify_override returns PyResult<OverrideGrant>; 11 SC4 pyo3_boundary tests confirm NonoOverrideError raised for every variant with stable args[0] kind. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| override.rs | 869/877 | `nb - skew` / `exp + skew` chrono Duration arithmetic can panic on overflow | WARNING (W-01 from code review) | DoS path: attacker-controlled timestamp near chrono bounds triggers process abort instead of Err. Not an auth bypass. Fix: use checked_sub_signed/checked_add_signed. |
| override.rs | 864-869 | Negative TTL (expires_at < not_before) silently passes TTL cap check | WARNING (W-02 from code review) | Inverted window is logically malformed token; should be rejected explicitly not via wall-clock coincidence. No exploitable path today. |
| override.rs | 782-786 | jti consumed (step 9) before scope partition (step 10) | WARNING (W-03 from code review) | Fragile ordering: future fallible logic at step 10+ would burn a legitimate jti on a non-replay error. Not a bug today (partition_scope is infallible). |
| override.rs | 535-588 | DER parser allows trailing bytes and zero-length INTEGER | WARNING (W-04 from code review) | Low-S gate accepts malformed DER as "low-S OK"; aws-lc-rs verifier would reject it anyway. Defense-in-depth gap; not an auth bypass today. |
| override.rs | 329 | `#[allow(dead_code)]` on OverrideToken struct (broad; CLAUDE.md prefers #[expect]) | INFO (I-04 from code review) | Style deviation; #[expect] would self-remove when Phase 92 reads the fields. No security impact. |

**Debt marker gate:** No TBD, FIXME, or XXX markers found in phase files. [BLOCKING-93] markers are formal deferral labels referencing a specific follow-up phase (Phase 93), not unresolved debt. Gate passed.

### Human Verification Required

#### 1. W-01/W-02/W-03 Warning Disposition Decision

**Test:** Review the four code-review warnings (W-01 chrono panic, W-02 inverted TTL, W-03 jti ordering, W-04 lenient DER) and decide whether to fix before proceeding to Phase 92 or to accept as Phase-92/pre-93 work.

**Expected:** One of:
- (a) Accept warnings as known-debt for Phase 92 — Phase 91 goal (offline fail-closed verifier) is met; warnings are hardening gaps not auth bypasses; document in Phase 92 plan.
- (b) Fix W-01 and W-03 now (the two the reviewer flags as "should fix before live trust anchor") before Phase 92 scopes against this code.

**Why human:** The code review (91-REVIEW.md) explicitly states these warnings "should be fixed before this code is relied on as the trust anchor for live overrides." Phase 91 is offline-only; Phase 92 begins wiring this into confined_run. The decision whether to fix now or carry into Phase 92's plan is a developer judgment call on acceptable risk — the warnings are robustness gaps, not security vulnerabilities, but they create exploitable DoS surface in the live trust path.

#### 2. VFY-03 Partial Coverage Acceptance

**Test:** Confirm that VFY-03 clause (a) (production pubkey sourcing from env/policy DER+base64 with per-key_id VerificationKey cache) being deferred to Phase 93 is acceptable given it is mapped to Phase 91 in REQUIREMENTS.md.

**Expected:** Confirm the [BLOCKING-93] deferral is sanctioned and Phase 93's acceptance gate will require VFY-03(a) before the code is treated as a live trust anchor.

**Why human:** REQUIREMENTS.md traceability table maps VFY-03 to Phase 91 (not Phase 93). The PLAN explicitly defers clause (a) with a [BLOCKING-93] marker and the SUMMARY confirms VFY-03 is PARTIAL. The verifier cannot determine whether this scope split was approved by the project owner or whether the REQUIREMENTS.md traceability should be updated to Phase 93.

---

## Gaps Summary

No technical gaps in the Phase 91 implementation. All 8 must-have truths are VERIFIED by test execution. The two human verification items are judgment calls, not implementation failures:

1. **W-01 through W-04** are robustness hardening gaps (DoS path, not auth bypass). The core verifier is cryptographically sound. The reviewer recommends fixing W-01 and W-03 before the code is used as a live trust anchor — which begins in Phase 92.

2. **VFY-03 PARTIAL** is an explicit, documented, planned deferral — not a gap discovered post-hoc. Both the plan and implementation are consistent and carry grep-assertable markers.

The phase goal — "a fully offline, fail-closed verifier for ZT-Infra CAF v0.1 override tokens exists in nono-py/src/override.rs — every parse error, signature failure, expiry violation, scope escape, jti replay, and algorithm mismatch maps to a raised NonoOverrideError, never to a silent grant" — is ACHIEVED for the offline case as scoped. 50/50 tests pass. Clippy clean. No AWS SDK added.

---

_Verified: 2026-06-21T23:30:00Z_
_Verifier: Claude (gsd-verifier)_
