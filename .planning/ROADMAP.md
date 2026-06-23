---
milestone: v3.2
milestone_name: Signed Policy Overrides (ZT-Infra Attestation)
status: in_progress
updated: 2026-06-22
---

# Roadmap: nono

## Milestones

- 🚧 **v3.2 Signed Policy Overrides (ZT-Infra Attestation)** — Phases 91-93 (in progress)
- ✅ **v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain** — Phases 85-90 (shipped 2026-06-21) — [archive](milestones/v3.1-ROADMAP.md)
- ✅ **v3.0 Enterprise Hardening I — Deploy · Control · Compliance** — Phases 82-84 (shipped 2026-06-19) — [archive](milestones/v3.0-ROADMAP.md)
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18) — [archive](milestones/v2.13-ROADMAP.md)

> Earlier milestones (v2.5–v2.12) are archived under `.planning/milestones/`.

## Phases

<details>
<summary>✅ v3.1 UPST9 Upstream Sync + v3.0 Drain (Phases 85-90) — SHIPPED 2026-06-21</summary>

Drain-then-sync upstream milestone: absorbed `always-further/nono` `v0.62.0..v0.64.0` (90 commits / 140 files) converging toward upstream's layout (audit stack + structured diagnostics relocated into the core `nono` crate) without regressing the Windows security model, then drained v3.0's host-gated UAT debt. Milestone-marker only — no crate publish; a future release leapfrogs the crate version to ≥ `0.65.0`. Full detail: [milestones/v3.1-ROADMAP.md](milestones/v3.1-ROADMAP.md).

- [x] Phase 85: UPST9 Divergence Audit (1/1 plans) — completed 2026-06-19
- [x] Phase 86: Library-Boundary Convergence (3/3 plans) — completed 2026-06-20
- [x] Phase 87: Security Sync (3/3 plans) — completed 2026-06-20
- [x] Phase 88: Feature + Dependency Cherry-Pick Wave (6/6 plans) — completed 2026-06-20
- [x] Phase 89: Proxy Hardening Sync (4/4 plans) — completed 2026-06-21
- [x] Phase 90: v3.0 Host-Gated UAT Drain (2/2 plans) — completed 2026-06-21

</details>

### 🚧 v3.2 Signed Policy Overrides (ZT-Infra Attestation) — Active

**Milestone Goal:** Replace the "just disable the sandbox" temptation with cryptographically-signed, ledger-logged policy exceptions. A developer who hits a false-positive block obtains an authorized, scoped, expiring signed override that the `nono-py` binding verifies against the ZT-Infra v2 AWS control plane (KMS-signed audit + DAAL ledger) and applies as a temporary, auditable, revocable expansion of the runtime ruleset — non-self-service. Milestone-marker only (no crate publish). Enforcement surface: `nono-py` binding. Rust core stays policy-free.

- [x] **Phase 91: Signed Override Format + Verification Core** - Define the ZT-Infra CAF v0.1 token schema and build the fully offline, fail-closed ECDSA verifier (completed 2026-06-21)
- [x] **Phase 92: Runtime CapabilitySet Mutation + Audit Wiring** - Wire the verifier into `confined_run`/`confine`, fuse additive mutation with mandatory SecurityEventLayer audit emission in one atomic phase (completed 2026-06-22)
- [x] **Phase 93: Live ZT-Infra Integration + Revocation + Request Flow** - Add the live `POST /actions` AND gate, KMS pubkey pin + key-ARN allowlist, AWS credential stripping, DAAL anchoring, CLI affordances, and Dark Factory scripted gates (completed 2026-06-23)

## Phase Details

### Phase 91: Signed Override Format + Verification Core
**Goal**: A fully offline, fail-closed verifier for ZT-Infra CAF v0.1 override tokens exists in `nono-py/src/override.rs` — every parse error, signature failure, expiry violation, scope escape, jti replay, and algorithm mismatch maps to a raised `NonoOverrideError`, never to a silent grant
**Depends on**: Phase 90 (SecurityEventLayer EventID 10001-10005 schema shipped; `nono::trust::signing::verify_keyed_signature` ECDSA P-256 primitive available)
**Requirements**: OVR-01, OVR-02, OVR-03, VFY-02, VFY-03, VFY-04, VFY-05, VFY-06, VFY-07
**Success Criteria** (what must be TRUE):
  1. `canonical_bytes()` applied to ZT-Infra test-vector inputs produces SHA-256 digests that match the reference output in `test-vectors/canonical-form/vectors.json` — the cross-language canonicalization is verified before signature verification is wired
  2. `verify_override()` with a valid token, correct KMS pubkey DER, and an allowlisted key ARN returns `Ok(OverrideGrant)`; every failure mode — bad signature, expired, `not_before` in future, missing required field, `algorithm:"none"`, `algorithm` other than `"ECDSA_SHA_256"`, high-S signature, key ARN not in allowlist — returns `Err` (not `Ok` with a deny flag, not `None`)
  3. A consumed `jti` is rejected on a second `verify_override()` call in the same process; the same token cannot be replayed even before its `expires_at`
  4. `NonoOverrideError` (not a generic `RuntimeError` or `None`) is raised at the PyO3 boundary for every `Err` variant from the Rust side
  5. The `#[must_use]` attribute on the verification `Result` triggers a compile warning if the caller ignores the return value
**Plans**: 3 plans (3 waves)
- [x] 91-01-PLAN.md — Canonical-form foundation: `override.rs` scaffold (keyword-collision resolved), strict `OverrideToken` serde model, `canonical_bytes()`/`canonical_sha256()` proven against the 9 ZT vectors (SC1; OVR-01/02/03)
- [x] 91-02-PLAN.md — Offline `verify_override()` pipeline: algorithm pin, ARN allowlist, explicit low-S, `verify_prehashed` over the digest, expiry/skew/TTL cap, in-process jti replay → immutable `OverrideGrant` (SC2/SC3; VFY-02..06)
- [x] 91-03-PLAN.md — PyO3 boundary: frozen `OverrideGrant` pyclass, first-in-repo `NonoOverrideError` custom exception, module registration, `#[must_use]` check (SC4/SC5; VFY-07)

### Phase 92: Runtime CapabilitySet Mutation + Audit Wiring
**Goal**: A verified override additively expands the `CapabilitySet` for exactly one `confined_run`/`confine` invocation and every such expansion emits an `AuditEventPayload::PolicyOverrideApplied` event into the `SecurityEventLayer` HMAC chain before the child spawns — an override that cannot emit its audit record is blocked, not silently applied
**Depends on**: Phase 91
**Requirements**: MUT-01, MUT-02, MUT-03, MUT-04, MUT-05, AUD-01, AUD-02, AUD-03, AUD-04, VFY-01, DF-01
**Success Criteria** (what must be TRUE):
  1. `confined_run(override_token=<valid>)` appends exactly the grant paths as `--allow` flags to the `nono.exe run` invocation; the OS confinement layer (AppContainer + WFP / Landlock / Seatbelt) still applies to the expanded set — the sandbox is never bypassed or conditionally applied
  2. `confined_run(override_token=None)` produces byte-for-byte identical `nono.exe run` invocations to pre-v3.2 behavior, proven by a regression test
  3. A grant for `/tmp/project` does not cover `/tmp/project-evil` or a path containing `..`; path-component comparison (`Path::starts_with` / `Path.is_relative_to`) is used, never string `starts_with`
  4. After a verified override applies, the HMAC chain contains exactly one new `AuditEventPayload::PolicyOverrideApplied` entry with the correct `zt_audit_hash` and `kms_key_id` fields, and the chain hash has advanced — the bi-directional tamper-evident link to the ZT-Infra audit chain is present
  5. `verify-dark.ps1 --gate OVERRIDE-01` emits a machine-readable `PASS` verdict against the offline verify path and the full set of fail-closed cases (bad sig, expired, out-of-scope, replay, `algorithm:"none"`)
**Plans**: 4 plans (3 waves)
Plans:
**Wave 1**
- [x] 92-01-PLAN.md — Core data types: `PolicyOverrideApplied` in `audit.rs`; EventIDs 10006–10010 + `SecurityEventType` variants in `telemetry/event.rs` (Wave 1; Nono repo)
- [x] 92-02-PLAN.md — nono-py wiring: `zt_audit_hash` field+getter on `OverrideGrant`; `append_override_args`/`sanitize_override_path`; extended `confined_run`/`confine` signatures (Wave 1; nono-py repo)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 92-03-PLAN.md — nono-cli bilateral gate: `--override-audit` flag + `OverrideAuditMeta`; `SECURITY_LAYER` OnceLock; `emit_override_event` method; AUD-04 pre-spawn gate in `execute_sandboxed` (Wave 2; Nono repo)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 92-04-PLAN.md — Verification: `scripts/gates/override-01.ps1` DF-01 gate + pytest `test_override_wiring.py` (Wave 3; both repos)

### Phase 93: Live ZT-Infra Integration + Revocation + Request Flow
**Goal**: The complete two-key AND gate is operational — a signed token is accepted only when both the KMS signature verifies offline AND a live ZT-Infra `POST /actions` lookup returns `allow`; revoked tokens are rejected on the next live check; AWS credentials never reach the sandboxed child environment; a developer can request and apply overrides via CLI
**Depends on**: Phase 92
**Requirements**: ZTL-01, ZTL-02, ZTL-03, ZTL-04, ZTL-05, CLI-01, CLI-02, DF-02
**Success Criteria** (what must be TRUE):
  1. With `NONO_ZT_ACTIONS_URL` set, `verify_override()` makes a `POST /actions` call; a `deny` response or a timeout exceeding 2 seconds blocks the invocation with `NonoOverrideError` — the live gate is fail-closed, not fail-open
  2. An override id added to the ZT-Infra deny-list is rejected on the next `verify_override()` call without any new revocation infrastructure in nono (the live check is the revocation enforcement point)
  3. After `confined_run()` with a verified override, the child process environment contains no `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, or other `AWS_*` variables — credential isolation is verified by a test inspecting the child's environment
  4. `nono override request` surfaces the denial context (paths, domains, repo) from `DiagnosticFormatter`; `nono override apply <token-path>` runs the full fail-closed verification before any expansion
  5. `verify-dark.ps1 --gate OVERRIDE-02` (live paths) emits `SKIP_HOST_UNAVAILABLE` when AWS/ZT-Infra is absent, consistent with the Dark Factory mandate; DAAL anchoring is recorded asynchronously and does not block the spawn path
**Plans**: 6 plans (3 waves)
Plans:
**Wave 1** *(parallel — disjoint file ownership)*
- [x] 93-01-PLAN.md — nono-py trust foundation: HKLM `Override\` reader (winreg, D-05/D-06), LiveRevoked/LiveUnavailable kinds (D-02), per-key_id VerificationKey cache closing VFY-03a `[BLOCKING-93]` (nono-py repo)
- [x] 93-02-PLAN.md — nono-cli AWS_* env strip (ZTL-04) + `override audit-emit` subcommand for the live-reject HMAC emission (OQ-1 a; 10008/10010) (Nono repo)

**Wave 2** *(blocked on Wave 1)*
- [x] 93-03-PLAN.md — `nono override request` CLI: denial bundle from DiagnosticFormatter (CLI-01); adds Request to the override group (Nono repo)
- [x] 93-04-PLAN.md — Python live arm `_live.py`: fail-closed POST /actions AND-gate (ZTL-01/02/03/05), Python pre-step closing VFY-01 clause b (OQ-2) (nono-py repo)

**Wave 3** *(blocked on Wave 2)*
- [x] 93-05-PLAN.md — `nono-override-apply` console-script: one-shot offline+live verify-then-run (CLI-02, OQ-5) (nono-py repo)
- [x] 93-06-PLAN.md — DF-02 gate `scripts/gates/override-02.ps1`: live allow+revoke proof, SKIP_HOST_UNAVAILABLE when provisioner absent (Nono repo)

## Progress

**Execution Order:** 91 → 92 → 93

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 85. UPST9 Divergence Audit | v3.1 | 1/1 | Complete | 2026-06-19 |
| 86. Library-Boundary Convergence | v3.1 | 3/3 | Complete | 2026-06-20 |
| 87. Security Sync | v3.1 | 3/3 | Complete | 2026-06-20 |
| 88. Feature + Dependency Cherry-Pick Wave | v3.1 | 6/6 | Complete | 2026-06-20 |
| 89. Proxy Hardening Sync | v3.1 | 4/4 | Complete | 2026-06-21 |
| 90. v3.0 Host-Gated UAT Drain | v3.1 | 2/2 | Complete | 2026-06-21 |
| 91. Signed Override Format + Verification Core | v3.2 | 3/3 | Complete    | 2026-06-22 |
| 92. Runtime CapabilitySet Mutation + Audit Wiring | v3.2 | 4/4 | Complete    | 2026-06-22 |
| 93. Live ZT-Infra Integration + Revocation + Request Flow | v3.2 | 6/6 | Complete   | 2026-06-23 |
