# Requirements: nono v3.2 — Signed Policy Overrides (ZT-Infra Attestation)

**Defined:** 2026-06-21
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms. A false-positive block must be resolvable by a *cryptographically-signed, ledgered, non-self-service exception* — never by disabling the sandbox.

**Scope source:** SEED-005. **Enforcement surface:** the `nono-py` binding. **Integration depth:** full ZT-Infra v2 AWS control plane (KMS-signed audit + DAAL ledger). **Milestone-marker only** (no crate publish).

> **Architecture invariant (from research):** verification is a **two-key AND gate** — a valid KMS signature *and* a live ZT-Infra `POST /actions` `allow`. An override **additively widens** the `CapabilitySet` for one invocation; it never removes a deny and never bypasses the OS confinement layer. The Rust core stays policy-free; all override logic lives in `nono-py`.

## v1 Requirements

### Override Token Format (OVR)

- [ ] **OVR-01**: A signed override token carries signer identity, scope (absolute filesystem paths + network domains), `not_before`, `expires_at`, a repo-context binding, and a unique `jti`.
- [ ] **OVR-02**: Scope and expiry are embedded **inside** the ZT-Infra KMS-signed payload (covered by the signature), not carried as unsigned wrapper metadata.
- [ ] **OVR-03**: The token uses the ZT-Infra CAF v0.1 canonical form; canonical bytes are **re-derived from the parsed structure** (sorted keys, signature/hash fields stripped), validated against ZT-Infra `test-vectors/canonical-form/vectors.json`.

### Verification (VFY)

- [ ] **VFY-01**: An override is accepted only when **both** the KMS signature verifies **and** a live ZT-Infra `POST /actions` lookup returns `allow` for the override `jti`/id (two-key AND gate; either alone is insufficient).
- [ ] **VFY-02**: Signature verification reuses the existing ECDSA P-256 primitive (`nono::trust::signing::verify_keyed_signature`); it pins `ECDSA_SHA_256`, enforces low-S, and explicitly rejects `algorithm:"none"` and any non-pinned algorithm.
- [ ] **VFY-03**: The KMS public key is sourced from embedded machine policy / env (DER+base64), cached per `key_id`; nono carries **no in-process AWS SDK** and **no AWS credentials**.
- [ ] **VFY-04**: The signer `key_id` is checked against a machine-policy allowlist of approved signing-key ARNs — cryptographic validity alone does not authorize.
- [ ] **VFY-05**: `expires_at`/`not_before` are enforced with a 2-minute clock-skew tolerance; maximum TTL is hard-capped in code (8h developer / 24h CI) and operator-configurable downward.
- [ ] **VFY-06**: Each token `jti` is single-use — a consumed-`jti` store rejects replay of an already-applied override.
- [ ] **VFY-07**: Verification is **fail-closed**: every failure (bad signature, parse error, ZT-Infra unavailable/timeout, expired, out-of-scope, replay, key not allowlisted) yields no expansion and runs nothing. The verify API returns `Result` (`#[must_use]`); the PyO3 boundary raises `NonoOverrideError` — never a falsy return.

### Runtime Ruleset Mutation (MUT)

- [ ] **MUT-01**: A verified override **additively** expands the `CapabilitySet` for the specific repo context, scoped to only the granted paths/domains.
- [ ] **MUT-02**: An override is **invocation-scoped** — it does not persist across `confined_run`/`confine` calls and does not mutate shared/global state.
- [ ] **MUT-03**: An override cannot remove or weaken an existing deny rule and cannot bypass the OS confinement layer (Landlock / Seatbelt / AppContainer + WFP remain enforced underneath).
- [ ] **MUT-04**: Granted paths/domains are matched by **path-component / DNS-component comparison** (never string `starts_with`); out-of-scope targets stay denied.
- [ ] **MUT-05**: The no-override code path is **byte-for-byte identical** to pre-v3.2 behavior (regression-proven).

### Audit Linkage (AUD)

- [ ] **AUD-01**: Every override lifecycle event (presented, verified, rejected, expired, revoked) emits a security event into the v3.0/v3.1 `SecurityEventLayer` HMAC chain.
- [ ] **AUD-02**: Override events embed the ZT-Infra `audit.current_hash`, creating a bi-directional tamper-evident link between the nono and ZT-Infra audit chains.
- [ ] **AUD-03**: Override events use a dedicated EventID range (10006–10010: PRESENTED / VERIFIED / REJECTED / EXPIRED / REVOKED) with redaction (no raw secrets; paths per the existing redaction policy).
- [ ] **AUD-04**: An override that applies without a successfully-emitted audit event is treated as a failure — no silent privilege escalation.

### Live ZT-Infra Integration + Revocation (ZTL)

- [ ] **ZTL-01**: The live decision endpoint is configurable (`NONO_ZT_ACTIONS_URL`) and integrates with the deployed ZT-Infra v2 AWS control plane (KMS-signed audit + DAAL ledger).
- [ ] **ZTL-02**: The live `POST /actions` call has a bounded timeout (2s default) and fails closed on timeout / error / `deny`.
- [ ] **ZTL-03**: Revocation is honored — an override id added to the ZT-Infra deny-list is rejected on the next live check; nono adds **no new revocation infrastructure**.
- [ ] **ZTL-04**: `AWS_*` credentials are stripped from the sandboxed child environment (extending the existing exec-strategy env filter).
- [ ] **ZTL-05**: Override authorizations are anchored to the ZT-Infra DAAL ledger **asynchronously and non-blocking** — authorization never waits on ledger finality.

### CLI / Developer UX (CLI)

- [ ] **CLI-01**: A developer who hits a false-positive denial can request an exception via `nono override request`, surfacing the denial context (paths/domains, repo) from the `DiagnosticFormatter`.
- [ ] **CLI-02**: A developer can apply a received signed token via `nono override apply` (and/or by supplying it to `confined_run`), which runs the full fail-closed verification before any expansion.

### Verification / Dark Factory (DF)

- [ ] **DF-01**: An unattended `verify-dark.ps1 --gate OVERRIDE-01` gate exercises the offline verify path + the fail-closed cases (bad sig, expired, out-of-scope, replay, `algorithm:"none"`) against the ZT-Infra local provisioner / test vectors, emitting a machine-readable verdict.
- [ ] **DF-02**: Live AWS/KMS + DAAL-anchoring paths are exercised by scripted gates that emit `SKIP_HOST_UNAVAILABLE` when AWS/host is absent (acknowledged host-gated tech-debt, consistent with prior milestones).

## v2 / Future Requirements

Tracked, not in this roadmap.

### Approval & Revocation (FUT)

- **FUT-01**: M-of-N multi-signer / threshold approval for high-blast-radius overrides.
- **FUT-02**: Real-time revocation push (webhook/stream) instead of pull-on-next-check.
- **FUT-03**: `nono-ts` binding parity for signed overrides (v3.2 ships the `nono-py` surface only).

## Out of Scope

Explicit exclusions (anti-features from research + structural impossibilities). Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Self-service override minting (developer signs their own) | Destroys the non-self-service model — the whole control. Signing authority is external (approver + KMS). |
| Open-ended / wildcard scope | Scope-creep escape; scope must be explicit absolute paths / DNS-component domains. |
| Offline / cached verification | Defeats live revocation (VFY-01/ZTL-03); the live check is mandatory. |
| Override that removes/weakens a deny rule | Overrides are additive-only (MUT-01/MUT-03). |
| Override that bypasses the OS confinement layer | An `allow` widens the allow-list *under* the OS layer; it never bypasses Landlock/Seatbelt/AppContainer+WFP. |
| Mid-session expiry revocation of already-granted OS capabilities | OS caps are fixed at spawn (immutable post-spawn); the invocation lifetime **is** the grant scope. Expiry is enforced at apply-time (VFY-05). |
| In-process AWS SDK / KMS signing inside nono | nono verifies, never signs; no KMS credentials in the nono process (VFY-03). |
| Crate publish / `v*.*.*` release | Milestone-marker only; a future release leapfrogs the crate version to ≥ `0.65.0`. |

## Traceability

Populated during roadmap creation (phase numbers continue from Phase 90 → Phase 91+).

| Requirement | Phase | Status |
|-------------|-------|--------|
| OVR-01 | — | Pending |
| OVR-02 | — | Pending |
| OVR-03 | — | Pending |
| VFY-01 | — | Pending |
| VFY-02 | — | Pending |
| VFY-03 | — | Pending |
| VFY-04 | — | Pending |
| VFY-05 | — | Pending |
| VFY-06 | — | Pending |
| VFY-07 | — | Pending |
| MUT-01 | — | Pending |
| MUT-02 | — | Pending |
| MUT-03 | — | Pending |
| MUT-04 | — | Pending |
| MUT-05 | — | Pending |
| AUD-01 | — | Pending |
| AUD-02 | — | Pending |
| AUD-03 | — | Pending |
| AUD-04 | — | Pending |
| ZTL-01 | — | Pending |
| ZTL-02 | — | Pending |
| ZTL-03 | — | Pending |
| ZTL-04 | — | Pending |
| ZTL-05 | — | Pending |
| CLI-01 | — | Pending |
| CLI-02 | — | Pending |
| DF-01 | — | Pending |
| DF-02 | — | Pending |

**Coverage:**
- v1 requirements: 28 total
- Mapped to phases: 0 (roadmap pending)
- Unmapped: 28 ⚠️ (filled by roadmapper)

---
*Requirements defined: 2026-06-21*
*Last updated: 2026-06-21 after initial definition*
