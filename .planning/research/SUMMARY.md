# Project Research Summary

**Project:** nono v3.2 — Signed Policy Overrides (ZT-Infra Attestation)
**Domain:** Security-critical capability sandbox + external decentralized-attestation control plane
**Researched:** 2026-06-21
**Confidence:** HIGH

## Executive Summary

nono v3.2 adds a **non-self-service signed override system**: authorized approvers issue time-limited, cryptographically-signed tokens that expand the OS sandbox's `CapabilitySet` for a single invocation. The architecture is deliberate — nono does **not** call KMS directly, does **not** sign tokens, and does **not** make policy decisions. nono verifies that ZT-Infra v2 *already authorized* the exception and that the KMS-signed audit record is authentic. An `allow` from ZT-Infra **widens** what the OS sandbox permits; it never removes a deny. Landlock / Seatbelt / AppContainer + WFP remain structurally enforced underneath every override.

The recommended implementation places all new override logic in **`nono-py`** (the chosen enforcement surface), with two thin additions to the core library: a new `AuditEventPayload::PolicyOverrideApplied` variant and **reuse of the existing `crates/nono/src/trust/signing.rs::verify_keyed_signature` ECDSA P-256 primitive** (which already uses `aws-lc-rs` under the hood). No new trust-root logic, no KMS SDK in-process, and no HTTP client in Rust core. Python `urllib.request` handles the `POST /actions` round-trip (POC-proven, zero new dependencies). This split keeps the Rust core **policy-free** (CLAUDE.md library boundary + ADR-86) while concentrating the external-authority dependency in the binding layer.

The single most dangerous risk is **fail-open**: any error in the verification pipeline that silently grants instead of denies converts a security control into a liability. Every error variant must map to `Err(NonoError::OverrideVerificationFailed)` with `#[must_use]` enforced, and the PyO3 boundary must raise `NonoOverrideError` — never return `None` / `False`.

## Key Findings

### Recommended Stack

No net-new Rust *crates* are required for the crypto path — it is already covered by in-tree deps. The only Cargo.toml work is **promoting** transitive/CLI-only deps to direct deps where the new module needs them. The HTTP round-trip lives in Python stdlib. See `STACK.md`.

**Core technologies:**
- **`nono::trust::signing::verify_keyed_signature`** (ECDSA P-256, `aws-lc-rs`-backed): verify the ZT-Infra KMS signature (`ECDSA_SHA_256`, low-S) — reuse, do **not** add a KMS SDK or reuse the sigstore keyless chain.
- **`serde_json` + `sha2` (workspace)**: re-derive the CAF v0.1 canonical bytes (sorted keys, no whitespace, strip `current_hash`+`kms_signature`, SHA-256, lowercase hex) — ~60–80 LOC, no new crate.
- **Python `urllib.request` (stdlib)**: the `POST /actions` live decision call from `nono-py` — POC-proven, zero new deps, trivially mockable.
- **Promote to direct deps in `crates/nono/Cargo.toml`**: `aws-lc-rs = "1"`, `base64 = "0.22"`, `chrono = { version = "0.4", features = ["serde"] }` (verify pins against `Cargo.lock` at plan time).

### Expected Features

From `FEATURES.md`. The enforcement model is a **two-key AND gate**: an override must pass *both* a cryptographic signature check *and* a live ZT-Infra `POST /actions` lookup for the override ID — either alone is insufficient (the live lookup is what makes it non-self-service and revocable).

**Must have (table stakes):**
- Signed exception token: signer identity, scope (absolute paths / network), `expires_at`, `not_before`, repo-context binding, `jti`.
- Fail-closed AND-gate verification (signature + live ZT-Infra decision); expired / out-of-scope / unavailable → nothing runs.
- Additive, invocation-scoped `CapabilitySet` expansion (cannot remove deny rules, cannot persist, cannot bypass the OS layer).
- Audit linkage: every override event (presented/verified/rejected/expired/revoked) enters the `SecurityEventLayer` HMAC chain, embedding ZT-Infra `audit.current_hash` for a bi-directional tamper-evident link.
- Revocation via ZT-Infra deny-list (add override ID to `provisioner/policies/actions.json` deny array; detected on next live check) — no new revocation infra.

**Should have (competitive):**
- `nono override request` / `nono override apply` CLI affordances reading the `DiagnosticFormatter` denial path.
- Single-use (`jti`) replay enforcement — **escalated to v1** (see open questions; replay risk is severe).

**Defer (v1.x / Phase 94):**
- DAAL async on-chain anchoring (non-blocking sidecar; authorization never waits on ledger finality).
- Expiry-aware session watchdog.

**Explicit anti-features (must be excluded):** self-service minting, open-ended/wildcard scope, offline/cached verification, deny-rule removal, OS-layer bypass.

### Architecture Approach

All override logic lives in `nono-py` (the enforcement surface + the external-authority dependency), keeping the Rust core policy-free. The core library gains only an observability primitive (`AuditEventPayload::PolicyOverrideApplied`) and reuses its existing ECDSA primitive. Control flow: `confined_run` hits a would-be denial → caller supplies a signed token → `nono-py` verifies signature (Rust call) + live `POST /actions` (Python) → on pass, additively expands the `CapabilitySet` for that one invocation → executes under unchanged OS confinement → emits the override security event. See `ARCHITECTURE.md`.

**Major components:**
1. `nono-py/src/override.rs` — `OverrideToken`, `OverrideGrant` (`#[pyclass]`), `verify_override`, `canonical_bytes()`, `NonoOverrideError`.
2. `nono-py` Python layer — `urllib.request` `POST /actions` round-trip + path-component scope checks (`Path.is_relative_to`, never `startswith`).
3. `crates/nono/src/audit.rs` — new `AuditEventPayload::PolicyOverrideApplied { id, signer, expires_at, zt_audit_hash }` variant (fork-extension pattern, like `RejectStage`).
4. `SecurityEventLayer` (nono-cli) — new override EventIDs (10006–10010), HMAC-chain advancement.
5. Live integration — KMS public-key pin (per-`key_id` cache + TTL), 2s timeout fail-closed, key-ARN allowlist, `verify-dark.ps1 --gate OVERRIDE-01`.

### Critical Pitfalls

Top items from `PITFALLS.md` (15 total):

1. **Fail-open (cardinal sin)** — every error variant (KMS timeout, JSON parse fail, PyO3 unwind, `unwrap_or_default()` on scope) must map to *deny*. Verify returns `Result<VerifiedOverride, NonoError>` `#[must_use]`, not `bool`/`Option`; PyO3 raises, never returns falsy.
2. **Algorithm confusion / low-S malleability** — ZT-Infra `audit.js` has an `algorithm: "none"` local-fallback shape the Rust verifier must explicitly reject; pin `ECDSA_SHA_256`; enforce low-S; validate against `test-vectors/canonical-form/vectors.json` first.
3. **Canonicalization mismatch** — re-derive CAF bytes from the parsed struct (`stableJson()` semantics: sorted keys), never hash raw received bytes; `serde_json::to_string()` key order is non-deterministic.
4. **Self-service trap** — local provisioner policy is a dev-editable JSON file; check `kms_signature.key_id` against a **machine-policy allowlist of approved signing key ARNs**, not just signature validity.
5. **Path-scope escape** — `str.startswith("/tmp/project")` matches `/tmp/project-evil` (same as the CLAUDE.md footgun); use `Path.is_relative_to()` (Python) / `Path::starts_with()` on canonicalized paths (Rust).
6. **Audit gap = silent privilege escalation** — an override that applies without emitting a security event defeats the purpose; audit emission is a mandatory gate, not optional.
7. **AWS credential leakage** — strip `AWS_*` from the child env (extend the existing `env_clear`/env-filter in the exec strategy); never let the sandboxed child inherit KMS creds.
8. **TOCTOU verify→apply** — capture an immutable `VerifiedOverride` and apply from it; don't re-read the token between verify and apply.

## Cross-Dimension Tensions Reconciled

**Tension 1 — KMS verification path.** STACK.md (raw ECDSA P-256 via `aws-lc-rs`) and ARCHITECTURE.md (reuse `trust/signing.rs::verify_keyed_signature`) are the **same conclusion** — `verify_keyed_signature` already uses `aws-lc-rs`. **Single recommended path: call `nono::trust::signing::verify_keyed_signature` from `nono-py/src/override.rs`. Do NOT use the `sigstore-verify` keyless/OIDC path — different trust root entirely.**

**Tension 2 — HTTP client placement.** STACK.md favors Python `urllib.request`; ARCHITECTURE.md favors a Rust `ureq` dep in nono-py. **Recommended default: Python `urllib.request`** (POC-proven, zero new deps, easy to mock). Re-evaluate `ureq` only if mTLS or TLS-stack fragility appears — make this an explicit checkpoint at the live-integration phase, not a silently deferred decision.

## Implications for Roadmap

Suggested 4-phase structure (phase numbers continue from Phase 90):

### Phase 91: Signed Override Format + Verification Core
**Rationale:** the token format is the dependency of every downstream phase; changing it post-wiring breaks the pipeline. Self-contained — no AWS, no spawn.
**Delivers:** `nono-py/src/override.rs` (`OverrideToken`, `OverrideGrant`, `verify_override`, `canonical_bytes()` tested against ZT-Infra `test-vectors/canonical-form/vectors.json`), `NonoOverrideError`, `nono override apply` CLI. Promote `aws-lc-rs`/`base64`/`chrono` to direct deps.
**Avoids:** algorithm confusion, canonicalization mismatch, fail-open (error-as-deny established here).

### Phase 92: Runtime CapabilitySet Mutation + Audit Wiring
**Rationale:** wire the proven verifier into `confined_run`/`confine`/`sandboxed_exec` **and** add audit emission in one atomic phase — an override without an audit event is silent privilege escalation.
**Delivers:** modified `windows_confined_run.rs` + `sandboxed_exec.rs`; `AuditEventPayload::PolicyOverrideApplied` + `CapabilitySource::Override`; EventID 10006 arm in `SecurityEventLayer`; regression test proving the no-token path is byte-for-byte identical to pre-v3.2.
**Avoids:** audit gap, OS-layer-bypass, additive-only violation.

### Phase 93: Live ZT-Infra Integration + Revocation + Request Flow
**Rationale:** the live `POST /actions` check is the non-self-service gate; host-gated, so it follows the proven offline path.
**Delivers:** `NONO_ZT_ACTIONS_URL` support, 2s timeout fail-closed, KMS pubkey pin + key-ARN allowlist + per-`key_id` cache, revocation path, `nono override request` CLI, `verify-dark.ps1 --gate OVERRIDE-01`. Confirm Tension-2 HTTP decision at phase start.
**Avoids:** self-service trap, fail-open on outage, AWS credential leakage.

### Phase 94: v1.x Hardening (Post-Validation)
**Rationale:** correct but not required for a meaningful milestone; add after end-to-end validation.
**Delivers:** `jti` consumed-token tracking, expiry-aware session watchdog, DAAL async non-blocking anchoring (ZT-Infra sidecar model).

### Phase Ordering Rationale
- Token format first (everything depends on it); verification core before any wiring.
- Mutation + audit fused (separating them risks a shippable-but-silent escalation window).
- Live/AWS path isolated last as the expected host-gated Dark-Factory tech-debt wave.
- `jti`/DAAL hardening deferred only after the end-to-end path is validated.

### Research Flags
Phases likely needing deeper research during planning:
- **Phase 91:** test-first CAF canonical-form against ZT-Infra vectors; scope/expiry placement (inside vs outside signed payload) needs a ZT-Infra-operator answer.
- **Phase 93:** HTTP-client confirmation; KMS public-key distribution procedure.

Phases with standard patterns (skip research-phase):
- **Phase 92:** existing `append_caps_allow_flags`, `py.detach()`, `AuditEventPayload` fork-extension patterns.
- **Phase 94:** DAAL sidecar follows the documented ZT-Infra async model.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified against in-tree Cargo.toml / Cargo.lock + ZT-Infra `audit.js` + CANONICAL_FORM.md |
| Features | HIGH | Grounded in nono source + ZT-Infra provisioner + ENTERPRISE_READINESS/FAILURE_MODES |
| Architecture | HIGH | CLAUDE.md boundary + `trust/signing.rs`/`audit.rs`/`windows_confined_run.rs` read directly |
| Pitfalls | HIGH | Project-specific (CLAUDE.md footguns + ZT-Infra source); no generic web inference |

**Overall confidence:** HIGH

### Gaps to Address (resolve in requirements, before Phase 91 planning)

- **KMS public-key distribution (embed vs fetch):** recommend embed as DER/base64 in `NONO_ZT_KMS_PUBKEY` / machine policy; cache per `key_id` (5-min TTL); avoids an in-process AWS SDK.
- **Scope/expiry inside vs outside the signed payload:** recommend **inside** ZT-Infra's signed `action`/`resource` fields (more tamper-evident) — needs ZT-Infra operator confirmation.
- **Max TTL / clock-skew:** recommend 8h developer / 24h CI (operator-configurable, hard-capped in code); 2-min clock-skew tolerance.
- **Single-use `jti` in v1 vs v1.x:** PITFALLS says v1, FEATURES says v1.x → **escalate to v1**; accept v1.x only with explicit cost justification.
- **EventID allocation:** reserve 10006–10010 (OVERRIDE_PRESENTED / VERIFIED / REJECTED / EXPIRED / REVOKED).

## Sources

### Primary (HIGH confidence)
- nono in-tree: `crates/nono/Cargo.toml`, `crates/nono/src/trust/signing.rs`, `crates/nono/src/audit.rs`, `crates/nono/src/capability.rs`, `crates/nono-cli/src/policy.rs`, `nono-py/src/windows_confined_run.rs`, `CLAUDE.md`.
- ZT-Infra v2 (`C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2`): `provisioner/src/{server,audit,policy,canonical}.js`, `CANONICAL_FORM.md`, `test-vectors/canonical-form/vectors.json`, `docs/ARCHITECTURE.md`, `ENTERPRISE_READINESS.md`, `FAILURE_MODES.md`.
- `proj/POC-zt-infra-e5-local-provisioner.md`, `.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md`.

### Secondary (MEDIUM confidence)
- v3.0/v3.1 SecurityEventLayer EventID schema (10001–10005) from milestone records.

---
*Research completed: 2026-06-21*
*Ready for roadmap: yes*
