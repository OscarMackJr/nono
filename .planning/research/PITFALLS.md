# Pitfalls Research

**Domain:** Signed policy overrides + external AWS cloud-trust integration in a capability-based OS sandbox
**Researched:** 2026-06-21
**Confidence:** HIGH — derived from project-specific sources (CLAUDE.md security footguns, ZT-Infra v2 CANONICAL_FORM spec, provisioner source, FAILURE_MODES.md, POC contract, SEED-005 breadcrumbs) rather than generic web research

---

## Critical Pitfalls

### Pitfall 1: Fail-OPEN on Any Error Path (the Cardinal Sin)

**What goes wrong:**
Any error condition in the verification pipeline — network timeout reaching KMS, JSON parse failure on the override token, `None`/`null` returned from a signature check, AWS credential expiry, panicked Rust thread — silently grants execution instead of denying it. The sandbox expands its `CapabilitySet` when it should have kept the baseline.

This is the single most dangerous pitfall in the entire milestone. It converts a security control into a liability: an attacker (or a network partition) can trigger the grant by engineering an error.

**Why it happens:**
Developers write `match result { Ok(verified) => grant(), Err(_) => grant_anyway() }` as a "don't block the developer" reflex. Or they use `unwrap_or_default()` on an `Option<Override>`, which returns an empty/permissive default when the parse fails (see CLAUDE.md Common Footgun #2: "silent fallbacks"). In Python/PyO3 this also manifests as a bare `except` clause that returns `True` (allow).

**How to avoid:**
Every error variant in the override verification path — network failure, timeout, parse error, missing field, unknown algorithm, expired cert, ledger unavailable — MUST map to `Err(NonoError::OverrideVerificationFailed(...))` and the caller MUST propagate `?` rather than pattern-matching to a fallback. No `unwrap_or`, no `unwrap_or_default`, no `unwrap_or_else(|_| allow())`. Apply `#[must_use]` to the verification `Result`. Write a test for every error variant that asserts deny (not grant) is the outcome. In the PyO3 boundary, ensure `PyCapsuleError` / `PyErr` propagates upward and is never silently absorbed by a broad `except` clause.

**Warning signs:**
- Verification function signature returns `bool` rather than `Result<bool, NonoError>` — eliminates the ability to distinguish "check passed" from "check errored".
- Code that does `if let Ok(ov) = verify_override(...) { expand() }` with no `else` branch.
- Tests that assert "error path doesn't crash" rather than "error path returns deny".
- Any place where a timeout is set to `None`/infinity for the AWS call.

**Phase to address:**
Phase 91 (Signed Override Format + Verification Core) — the verification function signature and error-propagation contract must be locked in as fail-closed from day one. Write the fail-open test vectors first; any implementation that passes them is wrong.

---

### Pitfall 2: Bypass of OS Confinement Layer (Override Replaces, Not Expands)

**What goes wrong:**
An approved override disables the OS sandbox entirely rather than expanding only the specific allow-list entry. Example: the code calls `Sandbox::apply()` with the baseline `CapabilitySet`, then on a valid override calls some kind of "unrestrict" or replaces the `CapabilitySet` wholesale with a permissive one. Since nono's sandbox cannot be un-applied once `restrict_self()` / `sandbox_init()` fires (CLAUDE.md Key Design Decision #1: "No escape hatch"), the temptation is to apply the sandbox AFTER verification — but if verification fails open or is skipped, the process runs unconfined.

A related failure: the override token is verified but the resulting `CapabilitySet` expansion is larger than the override scope (e.g., override grants write to `/tmp/project`, code grants write to all of `/tmp`).

**Why it happens:**
Developers conflate "policy decision layer" (ZT-Infra: allow/deny) with "enforcement layer" (nono: OS sandbox). The POC runbook is explicit that `allow` from zt-infra never bypasses nono confinement — nono runs underneath. But it is easy to invert this by applying nono conditionally or by treating a verified override as a bypass flag.

**How to avoid:**
Apply the OS sandbox unconditionally before any override processing. Overrides only add entries to the `CapabilitySet`; they never remove the baseline sandbox or change the enforcement mechanism. The override scope in the token (paths, network, access mode) MUST map 1:1 to discrete `FsCapability` / `NetworkCapability` additions, not to a profile replacement. Test that a valid override with scope `/tmp/project` does NOT grant write to `/tmp/project-evil` (path component comparison — see Pitfall 5).

**Warning signs:**
- Any code path where `Sandbox::apply()` is called inside an `if override_valid { ... }` branch.
- Code that constructs the full `CapabilitySet` from the override token's allow list rather than merging into the baseline.
- Override format that encodes a profile name (e.g., `"profile": "unrestricted"`) rather than explicit scoped grants.

**Phase to address:**
Phase 91 (Override Format) — require that the override schema encodes explicit scoped grants, not profile references. Phase 92 (Runtime Mutation) — enforce that the sandbox is applied before capability mutation; the mutation is an additive merge, never a replacement.

---

### Pitfall 3: Path Scope Escape via String Comparison

**What goes wrong:**
The override token grants write access to `/home/user/project`. The verification code checks whether the requested path is in scope using `path.starts_with("/home/user/project")` as a string comparison. An attacker (or a confused developer) requests access to `/home/user/project-evil` and the string comparison matches.

This is CLAUDE.md Common Footgun #1 and is listed explicitly in the Path Handling CRITICAL section: "String `starts_with()` on paths is a vulnerability."

**Why it happens:**
String `starts_with` is the obvious, idiomatic check. The path component boundary is non-obvious. In Python (the nono-py boundary), `pathlib.Path` provides `is_relative_to()` for correct component-aware containment, but developers often reach for `str.startswith()`. In Rust, `Path::starts_with()` IS correct (it compares components), but only if both paths have been canonicalized first. A non-canonicalized input path containing `..` segments can escape the prefix check.

**How to avoid:**
- In Rust: use `Path::starts_with()` (NOT `str.starts_with()`), and canonicalize both the token's scope path and the requested path before comparison. Use the existing "canonicalize at grant time" pattern in `capability.rs`.
- In Python/nono-py: use `pathlib.Path.is_relative_to()` (Python 3.9+), never `str.startswith()`. Canonicalize with `Path.resolve()` first.
- In the override token schema: store paths in canonicalized form; reject any token path that is not absolute.
- Write scope-escape test cases: `/tmp/project` does not cover `/tmp/project-evil`, `/tmp/project/../secret`, `/tmp/project//subdir` (double-slash).

**Warning signs:**
- Any occurrence of `str.startswith(scope_path)` or `.starts_with(scope_str)` (non-Path receiver) in the scope-checking code.
- Override paths stored without canonicalization in the token.
- Scope-check tests that only cover exact-match and subdirectory cases, with no adversarial escapes.

**Phase to address:**
Phase 91 (Override Format) — define the scope format as canonicalized absolute paths. Phase 92 (Runtime Mutation) — enforce component-aware containment in `apply_override()`.

---

### Pitfall 4: Self-Service Token Minting (Signer Identity Not Enforced)

**What goes wrong:**
The developer who hits the false positive is the same person who can produce a valid signature, because the signing authority is insufficiently constrained. Scenarios: the signing key is in the developer's local keychain; the override token only requires a valid KMS signature but does not verify which KMS key ID was used; the token schema has a `signer` field but no code checks that the signer is an authorized approver (engineering manager, security team); the local provisioner's allow list can be edited by the developer to grant themselves overrides.

**Why it happens:**
Developers focus on "is the signature valid?" (cryptographic check) and forget "is the signer authorized?" (policy check). KMS signature verification confirms that the KMS key signed the payload; it does not confirm that the key belongs to an authorized approver. The ZT-Infra provisioner's policy is loaded from a JSON file that could be editable locally.

**How to avoid:**
- The override token MUST include a `signer_key_id` field bound to a KMS key ARN. The verifier MUST check that the key ARN is in an allowlist of approved signing keys (stored in machine policy under `HKLM\SOFTWARE\Policies\nono` or equivalent, loaded at startup, not runtime-patchable by the sandboxed process).
- The approved signing key allowlist MUST be loaded from a policy channel the developer cannot modify without elevated privilege — not from the developer's home directory or a user-writable config file.
- Include a `signer_role` or `signer_identity` claim in the token that is checked against an expected approver identity (e.g., a service account ARN or a SPIFFE/SPIRE SVID).
- Audit test: developer-produced token (signed with a key not in the allowlist) must be rejected.

**Warning signs:**
- Verifier only checks that the signature is cryptographically valid, not that the signing key is authorized.
- The approved-signers list is stored in a user-writable location.
- No test for "valid signature from unauthorized key → reject".
- Override format does not include a `signer_key_id` / `signer_arn` field.

**Phase to address:**
Phase 91 (Override Format) — the token schema must include and mandate `signer_key_id`. Phase 92 (Verification) — implement the signer-allowlist check as a hard gate before capability expansion.

---

### Pitfall 5: Signature Stripping — Token Without Signature Accepted

**What goes wrong:**
The verifier parses the override token as JSON, checks signature fields if present, but does not reject a token that has no `kms_signature` field at all (or where `kms_signature.signature` is an empty string). An attacker strips the signature field and the "no signature = unsigned = unsigned is allowed" path executes.

Relatedly: the canonical form spec (CANONICAL_FORM.md R10) removes `current_hash` and `kms_signature` before hashing. If the verifier accidentally strips them before checking (rather than after), a stripped token hashes identically to a signed one.

**Why it happens:**
Pattern matching on JSON fields in a flexible language: `if token.get("kms_signature") { verify() }` — the `else` branch falls through to allow. In Rust this manifests as `if let Some(sig) = token.kms_signature { verify(sig) }` with no `else { return Err(...) }`.

**How to avoid:**
Require `kms_signature` as a mandatory field in the token schema (use `jsonschema` validation, which is already a workspace dependency). Deserialization must fail if the field is absent. The verifier must reject `{ "algorithm": "none", "key_id": "", "signature": "" }` (the "no KMS configured" shape from the local provisioner's `signHash` fallback). Enforce that `algorithm` must be exactly `"ECDSA_SHA_256"` — reject `"none"`, reject unknown strings.

**Warning signs:**
- Token schema where `kms_signature` is `Option<...>` rather than a required struct.
- Code that falls through to allow when signature verification is skipped.
- Tests that only test the "valid signature present" path, not the "signature absent" or "algorithm: none" paths.

**Phase to address:**
Phase 91 (Override Format + Schema) — make `kms_signature` mandatory in the JSON schema with `jsonschema` validation. Phase 92 (Verification) — add explicit rejection of the `algorithm: none` and empty-signature shapes.

---

### Pitfall 6: Algorithm Confusion and Missing Expiry/nbf Checks

**What goes wrong:**
**Algorithm confusion:** The token includes `"algorithm": "RS256"` or `"algorithm": "HS256"` but the verifier only checks the signature against its expected algorithm (`ECDSA_SHA_256`). If the verifier uses a library that auto-detects algorithm from the token header, an attacker can substitute a weaker algorithm. OR: the verifier hard-codes `ECDSA_SHA_256` but doesn't reject tokens claiming a different algorithm — those tokens pass the field check but the signature is verified against the wrong algorithm, producing a spurious pass.

**Missing expiry/nbf:** The override token has an `expires_at` field but the verifier doesn't check it, or checks it with clock skew too large (>5 minutes), or doesn't check `not_before`. An old override token works indefinitely.

**ECDSA low-S / malleability:** The ECDSA signature over NIST P-256 is malleable: for any valid `(r, s)` signature, `(r, n - s)` is also a valid signature. If the verifier accepts high-S signatures, a third party can produce an alternative valid signature for the same payload without the signing key, enabling audit bypass (two different signature bytes for the same token). The ZT-Infra provisioner already enforces low-S (`normalizeEcdsaDerLowS`) and the CANONICAL_FORM.md spec says "verifiers accepting signatures from arbitrary federation participants MUST enforce low-S."

**Why it happens:**
Algorithm confusion: developers trust the token's algorithm claim. Expiry: "we'll add expiry checking later." Low-S: developers are unaware ECDSA over P-256 is malleable without the low-S normalization.

**How to avoid:**
- Hard-code the expected algorithm (`ECDSA_SHA_256`) in the verifier and reject any token where `kms_signature.algorithm != "ECDSA_SHA_256"`. Do not read the algorithm from the token itself.
- Check `expires_at` (MUST be present) with a maximum of 2 minutes of clock skew tolerance. Reject tokens with `expires_at` in the past.
- Check `not_before` (nbf): reject tokens not yet valid.
- Enforce low-S on the DER-encoded signature before verifying: `s <= n/2` where `n` is the P-256 order. Reject high-S signatures.
- Test vector: expired token → reject; future nbf → reject; wrong algorithm claim → reject; high-S signature → reject.

**Warning signs:**
- Algorithm value read from the token struct rather than hard-coded in the verifier.
- No `expires_at` field in the override schema, or field present but not validated.
- No low-S enforcement in the Rust ECDSA verifier.
- Clock used for expiry check is `SystemTime::now()` without skew tolerance documented.

**Phase to address:**
Phase 91 (Override Format) — require `expires_at` and `not_before` as mandatory fields with ISO 8601 timestamps. Phase 92 (Verification) — enforce algorithm pinning, expiry, nbf, and low-S in the core verifier; include all four in the fail-closed test suite.

---

### Pitfall 7: Replay Attack — Token Reuse After Expiry or Revocation

**What goes wrong:**
A developer obtains a valid signed override for `/tmp/project` valid for 1 hour. After the hour expires, they (or a process with access to the token file) reuse the same token bytes. If the verifier only checks the signature and expiry timestamp but has no memory of previously-used tokens, the token is accepted indefinitely after its first use. Worse: the override is revoked mid-session (the manager who approved it changes their mind) but the process holds a valid non-expired token and the verifier has no revocation check.

**Why it happens:**
Stateless verification is simpler to implement. Developers assume expiry alone prevents replay. Revocation check requires a live network call, which conflicts with the "don't fail if AWS is slow" instinct (which itself is a fail-open instinct).

**How to avoid:**
- Include a `jti` (token ID / nonce) field in the override token. The verifier maintains a per-session set of consumed `jti` values. After first use, the `jti` is marked consumed and a second use is rejected.
- For longer-lived overrides: query the ZT-Infra `/actions` endpoint on each apply (not just at first use) with a dedicated "is this override still valid?" call. If the endpoint returns deny or the override is not in the ledger, block.
- Default override TTL should be short (15–60 minutes). The format should discourage multi-day tokens via a schema-level maximum.
- Revocation: the ZT-Infra ledger write is the revocation anchor. The verifier SHOULD check the ledger for the override token's `jti` before applying.

**Warning signs:**
- Override token format has no `jti`/nonce field.
- Verifier has no per-session consumed-token state.
- No test case for "valid token used twice → second use rejected".
- Override TTL defaulting to 24 hours or unbounded.

**Phase to address:**
Phase 91 (Override Format) — mandate `jti` and max TTL in the schema. Phase 92 (Verification) — implement consumed-token tracking. Phase 93 (Ledger Integration) — implement revocation check against ZT-Infra ledger.

---

### Pitfall 8: Canonicalization Mismatch Between Signer and Verifier

**What goes wrong:**
The ZT-Infra control plane produces the signed override record using the CAF v0.1 canonical form (sorted keys, no whitespace, UTF-8, no `0x` prefix on hashes, ASN.1 DER base64 signature). The nono Rust verifier deserializes the JSON with `serde_json` and re-serializes with a different key order or whitespace behavior, producing a different byte sequence, and therefore a different SHA-256 digest. Signature verification fails for all legitimately-signed tokens — or worse, the verifier falls back to a less-strict verification path that accepts the mismatch.

Related: the verifier receives a pretty-printed token (with whitespace) and hashes it directly without canonicalizing first. Since the signing side removed `current_hash` and `kms_signature` before hashing (CANONICAL_FORM R10), the verifier must also strip those fields before recomputing the digest.

**Why it happens:**
`serde_json` serializes object keys in insertion order by default; the CAF spec requires Unicode code-point sorted order. Developers test with a token produced by the same Rust code and never notice the mismatch. The cross-language mismatch (Node.js provisioner → Rust verifier) is only visible in integration testing.

**How to avoid:**
- Implement a dedicated `canonical_bytes()` function in Rust that: (1) strips `current_hash` and `kms_signature` fields before serializing, (2) serializes with sorted keys and no whitespace, (3) produces UTF-8 bytes. Use the CAF test vectors from `test-vectors/canonical-form/vectors.json` as the authoritative compliance suite.
- Test the Rust canonicalizer against those vectors before wiring to signature verification.
- The verifier MUST NOT hash the raw received bytes — it MUST re-derive the canonical form from the parsed struct.
- Reject tokens where received `current_hash` does not match the locally-computed canonical hash (hash-chain integrity check).

**Warning signs:**
- Verifier hashes the raw JSON bytes of the received token directly.
- No cross-language round-trip test of the canonicalizer against the provisioner's output.
- `serde_json::to_string()` used in the canonical path (produces non-deterministic key order).

**Phase to address:**
Phase 91 (Override Format) — define the canonical form for the nono override token (it need not be identical to CAF AuditRecord but must be documented). Phase 92 (Verification) — implement and test the Rust `canonical_bytes()` against ZT-Infra test vectors.

---

### Pitfall 9: KMS Public Key Trust and Rotation Blindness

**What goes wrong:**
The nono verifier fetches the KMS public key once at startup (or bundles it at build time) and uses it forever. When the KMS key is rotated — which is a routine operation — all future tokens are rejected because the old public key no longer matches the new signature. Meanwhile, any token signed with the old key (even after rotation) is still accepted if the old key is cached, opening a window where revoked-key tokens are valid.

Related: the verifier trusts any key it receives from the KMS `GetPublicKey` call without pinning the expected key ARN or alias. An attacker with AWS credential access can substitute a key they control.

**Why it happens:**
Fetching the public key on every verification is expensive and introduces AWS latency on the hot path. Developers cache it. They forget to implement key-ID checking or a key-rotation refresh path.

**How to avoid:**
- The override token MUST include `kms_signature.key_id` (already present in the ZT-Infra provisioner shape). The verifier MUST match this `key_id` against the allowlist of trusted key ARNs (stored in machine policy, not user config).
- Cache the KMS public key per `key_id` with a TTL (e.g., 5 minutes). On cache miss or TTL expiry, re-fetch via `GetPublicKey`.
- On key rotation: both the old and new key ARNs can appear in the allowlist during the transition window; tokens referencing the old key are rejected after the old ARN is removed from the allowlist.
- Pin the expected key ARN; never accept a `key_id` not in the allowlist regardless of whether the public key fetches successfully.

**Warning signs:**
- Public key fetched once and stored as a static `lazy_static!` / `OnceLock`.
- No `key_id` field in the override token schema.
- Verifier does not compare `token.kms_signature.key_id` against an approved-key allowlist before verifying.

**Phase to address:**
Phase 92 (Verification) — implement key-ID allowlist check and per-key-ID cache with TTL. Phase 93 (AWS Integration) — test key rotation path end-to-end.

---

### Pitfall 10: TOCTOU Between Verify and Apply

**What goes wrong:**
The verifier checks the override, returns `VerifiedOverride { scope, expires_at, ... }`, and then the caller applies the capability expansion in a separate step. In a multi-threaded nono-agentd scenario, another thread or a race on the supervisor pipe can inject a different scope between the verify and the apply steps. Alternatively: the token is verified, stored in a struct, and applied seconds later, by which time the token has expired — but the expiry was only checked at verify time, not at apply time.

**Why it happens:**
The separation of verify (async, may block on AWS) from apply (sync, must be fast) is architecturally sound, but creates a window if the verified result is mutable or if expiry is not re-checked.

**How to avoid:**
- The `VerifiedOverride` struct must be immutable (`pub struct VerifiedOverride { ... }` with no `&mut self` methods). Freeze capability scope at verify time.
- Check `expires_at >= now()` again at apply time, not just at verify time.
- Apply must happen atomically from the point of view of the supervisor: verify the override, then immediately expand the `CapabilitySet` within the same synchronous critical section, without yielding to the async runtime between the two steps.
- In nono-agentd: hold the supervisor pipe lock from the moment verification completes until capability expansion is committed.

**Warning signs:**
- `VerifiedOverride` struct with mutable fields or setter methods.
- `apply_override()` does not re-check `expires_at`.
- Async `.await` points between the end of `verify_override()` and the start of `CapabilitySet` mutation.
- Tests only single-threaded; no concurrent-apply tests.

**Phase to address:**
Phase 92 (Runtime Mutation) — design the verify→apply path as a single atomic function. Write a concurrent test that verifies a token, delays by TTL+1, and asserts the delayed apply is rejected.

---

### Pitfall 11: Audit Gap — Override Applied Without SecurityEventLayer Emission

**What goes wrong:**
A verified override expands the `CapabilitySet`. No security event is emitted into the v3.0/v3.1 `SecurityEventLayer` HMAC chain. The expansion is invisible to SIEM / Splunk / Sentinel. From the audit trail perspective, the process has more permissions than its profile but there is no record of why. This is silent privilege escalation — no override that doesn't emit a security event is acceptable (PROJECT.md §Tamper-evident audit linkage).

Relatedly: the override is emitted into the local HMAC chain but not written to the ZT-Infra ledger (missing the "zt-infra decides, nono records" coupling). The local chain can be tampered after the fact without the blockchain anchor.

**Why it happens:**
Audit emission is added as an afterthought. Developers test that the capability expansion works and forget that the security event path must also fire. The `SecurityEventLayer` lives on the daemon path (Phase 90 finding: it's in main.rs, not directly reachable from library code); wiring an override event through it requires the same `#[path]`-include or daemon-IPC pattern established in Phase 90.

**How to avoid:**
- Make audit emission a prerequisite of apply: the function signature must be `fn apply_override(verified: VerifiedOverride, auditor: &mut SecurityEventLayer) -> Result<()>`. If `auditor` is not wired, it's a compile error.
- Emit a `SecurityEvent` with a new EventID (next in the HMAC chain after EventIDs 10001-10005) containing: `jti`, `actor`, `scope` (paths + access modes), `signer_key_id`, `expires_at`, `timestamp`, HMAC chain hash.
- Also write the override decision to the ZT-Infra audit ledger via the `ActionAuditor.record()` shape so it appears in CloudWatch and the DAAL anchor.
- Write a test that asserts: after `apply_override()`, the audit layer has exactly one more event in its chain that matches the override's `jti`.

**Warning signs:**
- `apply_override()` has no `auditor` parameter.
- No EventID allocated for override events in the security event schema.
- Integration test for override applies but does not assert an audit event was emitted.
- Override ledger write to ZT-Infra is optional / best-effort.

**Phase to address:**
Phase 92 (Runtime Mutation) — `auditor` is a required parameter of `apply_override`. Phase 93 (Ledger Integration) — wire override decisions to ZT-Infra's `ActionAuditor.record()`.

---

### Pitfall 12: PyO3 Boundary Error Absorption

**What goes wrong:**
The override verification runs in Rust and returns a `Result<VerifiedOverride, NonoError>`. When called from Python via PyO3, a returned `Err` variant is converted to a Python exception. If the Python caller uses a bare `except Exception: pass` or a broad `except` that logs and continues, the Rust error is silently swallowed and execution proceeds as if the override was valid (or as if there was no override check). This is a special case of Pitfall 1 but specific to the language boundary.

Additionally: PyO3 conversion of complex types can panic if the Python interpreter state is not as expected (e.g., called from a thread without the GIL). A panic in a `#[pyfunction]` will unwind into Python with `SystemError` — but `SystemError` is a subclass of `Exception` and will be caught by `except Exception`.

**Why it happens:**
Python developers treat Rust extension errors as "unexpected" and use broad exception handling defensively. The nono-py binding is the enforcement surface (`confined_run` / `confine`) — if the Python layer fails to propagate the error, the Rust-side fail-close is bypassed.

**How to avoid:**
- Define a specific `NonoOverrideError(RuntimeError)` Python exception class in the nono-py binding. Verifier errors must raise this specific class, not the generic `RuntimeError`.
- The glue code in nono-py must `raise` on any `Err` from the Rust side — never convert to a return value of `None` or `False`.
- Document in the API that callers MUST NOT catch `NonoOverrideError` and continue execution; they must re-raise or fail.
- Test at the PyO3 boundary: inject a Rust-side error and assert that Python receives a `NonoOverrideError` exception (not `None`, not a boolean `False`).

**Warning signs:**
- `#[pyfunction]` that returns `PyResult<bool>` where `Ok(false)` means "override rejected" — this is ambiguous because `Err` is also a rejected state; error and deny look the same to the caller.
- No custom Python exception class in the nono-py binding for override failures.
- nono-py README shows usage with bare `except Exception` in the examples.
- No PyO3 boundary test for error propagation.

**Phase to address:**
Phase 92 (Verification) — define `NonoOverrideError` and enforce it in all `#[pyfunction]` wrappers. Phase 93 (nono-py Integration) — write the boundary error-propagation test suite.

---

### Pitfall 13: AWS Dependency Operational Failures (Latency, Outage, Credential Exposure)

**What goes wrong:**
**Latency:** KMS `GetPublicKey` and `Sign` calls add 50–200ms to every override verification. If override verification is on the hot path (called before every tool execution), this adds unacceptable latency. If the timeout is set too high, slow KMS responses can stall the agent.

**Outage:** AWS KMS or CloudWatch Logs is unavailable. The "fail closed vs. degrade gracefully" question arises. The correct answer per CLAUDE.md ("Fail Secure: On any error, deny access. Never silently degrade") is: KMS unavailable → no override can be verified → baseline capabilities only. But this blocks developer workflow if overrides are needed for legitimate work.

**Credential exposure:** The nono process needs AWS credentials to call KMS `GetPublicKey`. These credentials (IAM role, access key, or instance profile) must be accessible to the nono process. If the credentials are in environment variables visible to the sandboxed child process, a compromised agent can exfiltrate them and make its own KMS calls. If they are in a file readable by the child, same risk.

**Why it happens:**
AWS credential handling is an afterthought. Developers put the key in the environment (as in the zt-infra `.env` model) without considering that nono's supervised child inherits the parent's environment by default. Latency concerns cause developers to move verification into the background or skip it.

**How to avoid:**
- **Latency:** Cache the verified KMS public key per key-ID with a 5-minute TTL (see Pitfall 9). Do NOT call KMS on every verification — the public key is stable; only the signature check needs the public key, not a live KMS call. The one live AWS call (at cache-miss or TTL expiry) should have a 2-second timeout with fail-closed behavior.
- **Outage:** Define the policy clearly in requirements: KMS unreachable → no overrides accepted → baseline sandbox applies. This is the correct fail-closed behavior. Log the outage as a security event (not silently). Provide a scripted dark-gate test that simulates KMS unavailability and asserts baseline-only behavior.
- **Credential exposure:** Load AWS credentials in the supervisor (nono-cli / nono-agentd) ONLY. Strip the credentials from the child process's environment before spawning (the existing `env_clear` / env-filtering pattern in the exec strategy layer). The sandboxed child must never have `AWS_ACCESS_KEY_ID` or role credentials visible. Use the `keyring` crate (already a workspace dependency) or the machine policy channel to inject credentials into the supervisor process only.

**Warning signs:**
- AWS credentials passed via environment variables that are inherited by the sandboxed child.
- No timeout on the KMS `GetPublicKey` future.
- Override verification called on every tool invocation without caching.
- No test simulating KMS 503 or timeout that asserts deny.

**Phase to address:**
Phase 91 (Override Format + Trust Setup) — define credential handling policy (supervisor-only, never inherited by child). Phase 93 (AWS Integration) — implement timeout, caching, and credential isolation; write the KMS-outage dark gate.

---

### Pitfall 14: Repo-Context Binding Bypass

**What goes wrong:**
The override token is issued for repository context `github.com/org/projectA`. The developer copies the token file to a different working directory (`/home/user/projectB`) and the verifier does not check the repo-context claim. The override grants write access to projectB's directories because the scope paths were relative or loosely specified.

**Why it happens:**
Developers implement the cryptographic checks (signature, expiry, scope paths) but treat `repo_context` as informational metadata rather than an enforcement field. Path scope alone may not be sufficient if the override was intended for a specific project context and the developer has multiple projects in sibling directories.

**How to avoid:**
- Include a mandatory `repo_context` field in the override token (e.g., git remote URL or SHA of the repo's `.git/config`). At apply time, compute the current repo context from the working directory and compare against the token's claim. Reject if they don't match.
- The repo context MUST be verified using a structural comparison (git remote URL normalization: strip trailing `.git`, normalize `http://` vs `https://`), not a string equality check that can be tricked by case differences or trailing slashes.
- Write a test: token issued for `github.com/org/projectA` rejected when applied in `github.com/org/projectB` working directory.

**Warning signs:**
- No `repo_context` field in the override token schema.
- `repo_context` field present but not validated at apply time.
- Scope paths are relative (e.g., `./tmp`) rather than absolute and canonicalized.

**Phase to address:**
Phase 91 (Override Format) — require `repo_context` with a normalization spec. Phase 92 (Runtime Mutation) — enforce repo-context match at apply time.

---

### Pitfall 15: Wildcard and Glob Scope Expansion

**What goes wrong:**
The override token encodes scope as a glob pattern: `"/home/user/project/**"`. The verifier uses `glob::Pattern::matches()` to check if a requested path matches. Depending on the glob library's semantics, `**` may or may not match across directory separators, `?` may match `/`, and a carefully crafted scope pattern can match paths outside the intended directory (e.g., `"/home/user/project/../../../etc/**"`).

**Why it happens:**
Globs feel natural for expressing "write anywhere in this project." Glob libraries have inconsistent semantics around `**` and path separators. The ZT-Infra policy evaluator already uses a glob-like `endsWith("*")` matcher (see `policy.js` `matches()`). If nono adopts the same pattern without the CLAUDE.md path-component awareness, it inherits the escape risk.

**How to avoid:**
- Prohibit glob patterns in the override scope. Require explicit absolute canonical paths (directories or files). Expansion is "this exact directory and everything below it" — expressed as a canonicalized `PathBuf` prefix, not a glob string.
- If globs are required for flexibility, restrict to `/**` suffix only (no `?`, no `[...]`, no bare `*` in intermediate segments) and verify the prefix before the `/**` is a canonicalized absolute path with no `..` components.
- Apply CLAUDE.md's `Path::starts_with()` (component-aware) for all containment checks, never glob matching.

**Warning signs:**
- Override token schema uses `string` type for scope paths with no documented format constraint.
- Scope checking uses any form of glob, regex, or pattern matching instead of canonical prefix comparison.
- Test cases for scope validation do not include `..` traversal, double-slash, or glob injection attempts.

**Phase to address:**
Phase 91 (Override Format) — define scope as an array of absolute canonicalized path strings; document the containment semantics explicitly. Phase 92 (Verification) — enforce no-glob; use `Path::starts_with()` only.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Caching KMS public key forever (no TTL) | Eliminates AWS call latency after first use | Old key remains trusted after rotation; rotation window creates security gap | Never — implement 5-min TTL |
| Skipping ledger write on KMS outage | Prevents outage from blocking legitimate work | Override decisions not anchored; DAAL integrity broken | Never in production; acceptable in dark-gate test mode with explicit flag |
| Local provisioner for all dev testing | No AWS required for CI | Self-signed / no-KMS tokens exercise different code paths than production; `algorithm: none` passes verifier | Acceptable in unit tests if the `algorithm: none` path is explicitly rejected in production build via cfg flag |
| Single-use token without jti tracking | Simpler verifier state | Replay attacks trivially possible | Never |
| `unwrap_or_default()` on scope parse | Short code | Empty scope = deny nothing = full access | Never |
| Relative paths in override scope | Easier to write tokens manually | Symlink escape, working-directory ambiguity | Never in production tokens |

---

## Integration Gotchas

Common mistakes when connecting to external services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| AWS KMS signature verification | Using `MessageType=RAW` instead of `MessageType=DIGEST` — sends the full message to KMS and KMS double-hashes it, producing a wrong digest | Use `MessageType=DIGEST` with the raw 32-byte SHA-256 output, matching the ZT-Infra provisioner's `signHash()` implementation |
| ZT-Infra canonical form | Hashing the JSON bytes as received | Re-derive canonical bytes after parsing: sorted keys, no whitespace, strip `current_hash` and `kms_signature` before hashing (CANONICAL_FORM R10) |
| nono-py `confined_run` | Treating `Err` from override check as `None` / allowing execution | Override check must raise `NonoOverrideError`; caller must let it propagate |
| DAAL ledger write | Blocking the authorization path on ledger confirmation | DAAL write is async and must not block the nono apply path; local audit record is the system of record, ledger is eventual |
| CloudWatch Logs | Writing the override token's raw scope paths to CloudWatch without redaction | Log only `jti`, `actor`, `signer_key_id`, `expires_at`; hash or omit scope paths |
| sigstore-rs | Reusing the sigstore keyless-OIDC path for KMS override signing | KMS-signed overrides use a different trust root (AWS KMS key ARN) than sigstore's Rekor/Fulcio path; do not mix the two verification flows |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Live KMS call on every tool invocation | Agent latency spikes 100–300ms per tool call | Cache public key per key_id with 5-min TTL; live call only on miss or explicit refresh | From first tool call |
| Synchronous ledger write blocking apply | Override takes 2–10 seconds when DAAL submission is slow | Make DAAL write async (already the ZT-Infra architecture); only the local hash-chain write is synchronous | When blockchain congestion or provider latency increases |
| Schema validation on every token re-check | CPU spike if `jsonschema` validates large tokens on every capability check | Parse and validate once at ingest; store the `VerifiedOverride` struct | At high tool-call frequency |

---

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| AWS credentials inherited by sandboxed child | Child process exfiltrates IAM credentials, makes its own KMS calls | Strip `AWS_*` environment variables before spawning child; load credentials in supervisor only |
| Override token stored in user-writable directory | Attacker modifies token file between verify and apply (TOCTOU) | Store in a temp location owned by the supervisor; verify and apply in the same synchronous block |
| Accepting `algorithm: none` or unknown algorithms | Unsigned tokens accepted as verified | Hard-code expected algorithm; reject any deviation |
| Missing low-S enforcement | Signature malleability; two different byte sequences both valid for same payload | Enforce `s <= n/2` per CANONICAL_FORM spec; reject high-S |
| Wildcard scope `/**` matching `/../../../etc` | Path escape from intended scope | Validate that glob prefix is canonical and absolute before accepting |
| Override token logging with full scope in plaintext | Information disclosure of confidential directory layouts | Log only `jti`, `actor`, `signer_key_id`, `expires_at`; hash or omit scope paths |

---

## "Looks Done But Isn't" Checklist

Items that appear complete but are missing critical pieces.

- [ ] **Fail-closed on AWS outage:** verify that with KMS unreachable, the override is rejected (not granted) — test with a mock that returns 503
- [ ] **Fail-closed on parse error:** verify that a malformed token JSON returns `Err`, not `Ok(deny)` — both must be distinguishable from success
- [ ] **Signer-key allowlist check:** verify that a valid signature from an unauthorized KMS key ARN is rejected — not just that the signature is cryptographically valid
- [ ] **Algorithm pinning:** verify that tokens with `"algorithm": "none"` or `"algorithm": "RS256"` are rejected before signature verification runs
- [ ] **Expiry enforced at apply time:** verify that a token that was valid at verify time but expired before apply is rejected
- [ ] **Low-S enforcement:** verify that a high-S variant of a valid signature is rejected
- [ ] **Path component containment:** verify that `/tmp/project-evil` is not covered by a scope of `/tmp/project`
- [ ] **Repo-context binding:** verify that a token issued for projectA is rejected in projectB's working directory
- [ ] **Replay prevention:** verify that a consumed token (same `jti`) is rejected on second use
- [ ] **Audit emission:** verify that `apply_override()` emits exactly one `SecurityEventLayer` event per override application
- [ ] **PyO3 error propagation:** verify that a Rust-side `Err` raises `NonoOverrideError` in Python, not `None` or `False`
- [ ] **Child environment isolation:** verify that after `confined_run()` with an override, the child process cannot read `AWS_ACCESS_KEY_ID` from its environment

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Fail-open regression shipped | HIGH | Hotfix: add error→deny mapping; revoke any overrides granted during the window; audit SecurityEventLayer for anomalous expansions |
| OS confinement bypass discovered | HIGH | Emergency: disable override system entirely; revert to baseline profiles; incident response |
| Path escape via string comparison | HIGH | Patch comparison to use `Path::starts_with()`; review all existing issued tokens for scope overlap; rotate affected tokens |
| Replay attack on consumed token | MEDIUM | Invalidate all outstanding tokens; reissue with new `jti` values; patch consumed-token tracking |
| KMS key not in allowlist accepted | HIGH | Rotate the trusted-key allowlist; audit all overrides verified since the bad key was added |
| Audit gap (no SecurityEventLayer event) | MEDIUM | Reconstruct audit from ZT-Infra CloudWatch logs (secondary source); patch emission; re-verify retroactively |
| AWS credentials leaked to child | HIGH | Rotate IAM credentials immediately; review agent's network calls during the session; patch env stripping |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Fail-OPEN on any error | Phase 91 (format) + Phase 92 (verify) | Test every error variant asserts deny; `#[must_use]` on Result |
| OS confinement bypass | Phase 91 (format schema) + Phase 92 (runtime) | Test: override apply does not disable sandbox |
| Path scope escape via string comparison | Phase 91 (format) + Phase 92 (verify) | Adversarial test vectors: `/tmp/project-evil`, `../..` |
| Self-service token minting | Phase 91 (format) + Phase 92 (verify) | Test: unauthorized-signer token rejected |
| Signature stripping | Phase 91 (schema) + Phase 92 (verify) | Test: absent sig field → rejected; `algorithm: none` → rejected |
| Algorithm confusion + missing expiry/nbf/low-S | Phase 91 (format) + Phase 92 (verify) | Test vectors: wrong alg, expired, future nbf, high-S |
| Replay attack | Phase 91 (format, jti) + Phase 92 (verify, jti tracking) + Phase 93 (ledger) | Test: same jti twice → second rejected |
| Canonicalization mismatch | Phase 91 (format) + Phase 92 (verify) | Cross-lang vectors from ZT-Infra `test-vectors/` pass |
| KMS key trust / rotation blindness | Phase 92 (verify) + Phase 93 (AWS) | Test: key not in allowlist → rejected; rotation path |
| TOCTOU between verify and apply | Phase 92 (runtime mutation) | Concurrent test with delay between verify and apply |
| Audit gap | Phase 92 (runtime) + Phase 93 (ledger) | Test: apply emits SecurityEventLayer event |
| PyO3 error absorption | Phase 92 (verify) + Phase 93 (nono-py) | PyO3 boundary error propagation test |
| AWS credential exposure / outage | Phase 91 (trust setup) + Phase 93 (AWS integration) | Dark gate: child env has no `AWS_*`; KMS-503 → deny |
| Repo-context binding bypass | Phase 91 (format) + Phase 92 (verify) | Test: projectA token rejected in projectB workdir |
| Wildcard / glob scope expansion | Phase 91 (format) + Phase 92 (verify) | Test: no glob accepted; `..` in scope path → rejected |

---

## Sources

- `C:\Users\OMack\Nono\CLAUDE.md` — Security Considerations, Path Handling CRITICAL, Common Footguns, Key Design Decisions (project-specific, HIGH confidence)
- `C:\Users\OMack\Nono\.planning\seeds\SEED-005-zt-infra-policy-override-attestation.md` — scope, breadcrumbs, breadcrumb context (project-specific, HIGH confidence)
- `C:\Users\OMack\Nono\proj\POC-zt-infra-e5-local-provisioner.md` — fail-closed contract, E5 composition model, what an allow does and does not bypass (project-specific, HIGH confidence)
- `C:\Users\OMack\Nono\.planning\PROJECT.md` §v3.2 — milestone goal, key context/decisions (project-specific, HIGH confidence)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\CANONICAL_FORM.md` — CAF v0.1 canonical form spec (R1-R12), signature computation, low-S mandate, rationale (authoritative, HIGH confidence)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\audit.js` — KMS `MessageType=DIGEST`, `ECDSA_SHA_256`, `normalizeEcdsaDerLowS`, `algorithm: none` shape, hash-chain structure (authoritative, HIGH confidence)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\canonical.js` — `stableJson()` implementation, sorted keys, string validation, forbidden characters (authoritative, HIGH confidence)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\policy.js` — `matches()` with glob `endsWith("*")`, default-deny policy shape (authoritative, HIGH confidence)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\FAILURE_MODES.md` — operational failure mode table; policy engine unavailable = fail closed (authoritative, HIGH confidence)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\ENTERPRISE_READINESS.md` — DAAL async design, signer policy, failure behavior table (authoritative, HIGH confidence)
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\ARCHITECTURE.md` — composition model: zt-infra decides, nono enforces underneath; layer boundaries table (authoritative, HIGH confidence)

---
*Pitfalls research for: signed policy overrides + external AWS cloud-trust integration in a capability-based OS sandbox*
*Researched: 2026-06-21*
