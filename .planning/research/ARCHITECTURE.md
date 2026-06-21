# Architecture Research: Signed Policy Overrides (ZT-Infra Attestation)

**Domain:** Runtime capability mutation via signed external policy decisions
**Researched:** 2026-06-21
**Confidence:** HIGH — all components are existing, readable source code; no speculative claims

---

## The Core Architectural Constraint: Library-Is-Policy-Free

The single most important architectural decision this milestone must honor is the one already
encoded in CLAUDE.md's Library vs CLI Boundary table:

> "The library applies ONLY what clients explicitly add to `CapabilitySet` — audit and diagnostics
> modules are observability primitives, not security policy."

This invariant has a direct consequence for signed-override placement:

- **`crates/nono` (core library):** MAY supply `CapabilitySet` builder mutations and a new
  `AuditEventPayload` variant. It must NOT embed HTTP clients, ZT-Infra URLs, signature
  verification policy, or token-format definitions. Adding "verify against ZT-Infra" to the
  core library would make the library opinionated about which external authority is trusted — a
  policy decision, not a primitive.
- **`crates/nono-cli`:** The existing group/deny resolver (`policy.rs`) is where all current
  policy lives. CLI-side is the natural home for an optional operator CLI verb (e.g. `nono
  override apply <token>`). But the CLI is not the enforcement surface for this milestone — the
  milestone's explicit decision is that enforcement lives in `nono-py`.
- **`nono-py` binding (enforcement surface):** This is where the signed-override verification
  and the HTTP round-trip to ZT-Infra `POST /actions` live. The milestone explicitly designates
  `nono-py` as the enforcement surface, consistent with the existing E5 pre-exec interception
  slot in `DESIGN-engine-abstraction.md` §"Forward-Compat: zt-infra.org Integration". The
  binding is already the integration point for `confined_run`/`confine`; adding override
  verification here keeps the Rust core policy-free and concentrates the external-authority
  dependency in the binding.

**Placement verdict:**

| Layer | Role in v3.2 |
|-------|-------------|
| `crates/nono` (lib) | New `AuditEventPayload::PolicyOverrideApplied` variant; `CapabilitySet` builder unchanged — still a pure primitive; `trust/` module's existing `verify_keyed_signature` reused for ECDSA token verification |
| `crates/nono-cli` | Optional new `nono override` verb for operator UX; does NOT add verification to the run path — that would conflate CLI policy with binding enforcement |
| `nono-py` | New `override.rs` module; mutated `confined_run`/`confine` signatures; owns the HTTP round-trip, token parsing, signature verification, expiry/scope checks, scoped `CapabilitySet` mutation, and audit event emission |
| ZT-Infra v2 (external) | `POST /actions` decision + hash-chained KMS-signed audit record; DAAL ledger anchor (optional, not a required gate) |

---

## System Overview

```
  Developer workflow (out-of-band — happens before the confined_run call)
  ────────────────────────────────────────────────────────────────────────
  Developer hits a nono block
       |
       v
  Operator/EM uses ZT-Infra v2 control plane (AWS Tailscale/SSM or local provisioner)
       |  POST /actions  {"actor":"dev","action":"nono.fs.override",
       |                  "resource":"git://github.com/org/repo","scope":{...},"expiry":"..."}
       |
       v
  ZT-Infra ActionAuditor.record():
      previous_hash <- last chain entry (or ZERO_HASH)
      current_hash = SHA-256(stableJson({actor,action,resource,decision,reason,timestamp,previous_hash}))
      kms_signature = KMS.SignCommand(ECDSA_SHA_256, current_hash)  // normalizeEcdsaDerLowS applied
      append to audit-chain.jsonl + CloudWatch + optional DAAL anchor (Base Sepolia)
       |
       v
  Operator delivers signed override token to developer (email / secrets manager / file):
  {
    "actor": "dev@example.com",
    "action": "nono.fs.override",
    "decision": "allow",
    "scope": {"paths":["/var/cache/pip"],"mode":"READ_WRITE"},
    "expiry": "2026-06-22T00:00:00Z",
    "repo_context": "git://github.com/org/repo@abc123",
    "audit": {
      "previous_hash": "0000...000",
      "current_hash": "a1b2...",
      "kms_signature": {"algorithm":"ECDSA_SHA_256","key_id":"arn:...","signature":"<b64>"}
    }
  }

  Runtime execution path (in-band — inside confined_run / confine)
  ─────────────────────────────────────────────────────────────────

  nono_py.confined_run(exe, args, allow, profile, override_token=token_json)
       |
       +--[1] No token? --> proceed as today (no mutation, no new behavior)
       |
       +--[2] Token present? --> verify_override(token_json, kms_pubkey_config)
       |       a. Parse token JSON, validate required fields present
       |       b. Check expiry: token.expiry > utcnow()
       |          FAIL-CLOSED: expired -> raise PyRuntimeError, nothing runs
       |       c. Check repo_context binding (git URL + commit hash match)
       |          FAIL-CLOSED: mismatch -> raise PyRuntimeError, nothing runs
       |       d. Recompute current_hash = SHA-256(stableJson(unsigned_fields))
       |          Verify ECDSA_SHA_256(kms_signature.signature, current_hash, pinned_kms_pubkey)
       |          via nono::trust::signing::verify_keyed_signature
       |          FAIL-CLOSED: invalid sig -> raise PyRuntimeError, nothing runs
       |       e. (Optional) Live ZT-Infra check: POST /actions to confirm not revoked
       |          FAIL-CLOSED: deny or timeout -> raise PyRuntimeError, nothing runs
       |       f. Return OverrideGrant{paths, mode, expiry, zt_audit_hash}
       |
       +--[3] Scoped CapabilitySet mutation (Rust side, before spawn):
       |       base_caps = build from (allow, profile) as today
       |       for path in grant.paths:
       |           append --allow <path> to nono.exe run invocation
       |       (network expansion via allow_domain deferred -- nono-proxy is the right surface)
       |       OS confinement still applies to the expanded set -- no bypass
       |
       +--[4] Emit override event into nono audit HMAC chain (before spawn, fail-soft):
       |       AuditEventPayload::PolicyOverrideApplied {
       |           actor,
       |           scope (paths, mode, expiry),
       |           zt_audit_hash: token.audit.current_hash  // <-- bi-directional link
       |           kms_key_id
       |       }
       |       SecurityEventLayer -> EventID 10006 (NONO_POLICY_OVERRIDE_APPLIED)
       |       -> Windows Event Log (Application channel) + ETW -> Splunk/Sentinel
       |
       +--[5] Execute under OS confinement:
               nono.exe run --profile <profile> --allow <paths>... --allow <grant.paths>... -- <exe> <args>
               Low-IL + AppContainer + Job + WFP apply regardless of grant
               An allow NEVER bypasses the OS layer -- nono is the sandbox underneath

  Forensic cross-reference (bi-directional):
  ────────────────────────────────────────────
  nono chain -> ZT-Infra:  zt_audit_hash in PolicyOverrideApplied points to the ZT-Infra
                            audit-chain.jsonl / CloudWatch entry that authorized the override
  ZT-Infra -> nono chain:  correlation_id in the POST /actions request can carry the nono
                            session's HMAC chain head (optional; the server.js correlation_id
                            field is already supported)
```

---

## Component Boundaries

| Component | Responsibility | Communicates With | Status |
|-----------|---------------|-------------------|--------|
| `nono-py/src/override.rs` (NEW) | Token parse, expiry/scope/sig verify, optional live ZT check, `OverrideGrant` type | ZT-Infra `POST /actions` (HTTP blocking); `crates/nono::trust::signing::verify_keyed_signature` | New file |
| `nono-py/src/windows_confined_run.rs` (MODIFIED) | Accept `override_token: Option<String>`; call `verify_override`; add grant paths via `append_caps_allow_flags`; emit audit event before spawn | `override.rs` (same crate); `nono.exe run` subprocess | Modified |
| `nono-py/src/sandboxed_exec.rs` (MODIFIED) | Same override wiring for Unix `sandboxed_exec` | `override.rs` | Modified |
| `nono-py/src/lib.rs` (MODIFIED) | Register new `override.rs` module and expose `OverrideGrant` as `#[pyclass]` | PyO3 module registry | Modified |
| `crates/nono/src/audit.rs` (MODIFIED) | New `AuditEventPayload::PolicyOverrideApplied` variant with `zt_audit_hash` field | `SecurityEventLayer` in nono-cli via `tracing` | Modified |
| nono-cli `SecurityEventLayer` (MODIFIED) | Emit EventID 10006 for `PolicyOverrideApplied`; follow existing EventID 10001-10005 pattern | Windows Event Log + ETW | Modified |
| `crates/nono/src/trust/signing.rs` (REUSED AS-IS) | `verify_keyed_signature` for raw ECDSA P-256 verification | Called from `override.rs` | Existing, unchanged |
| ZT-Infra `POST /actions` (EXTERNAL) | Policy decision + hash-chained KMS-signed audit record | Called by `override.rs` over HTTP | External, unchanged |

---

## Data Flow

### Override Token Format

The token is a JSON envelope delivered out-of-band. It is the ZT-Infra `POST /actions`
response augmented with scope and expiry fields added by the operator or a thin helper
script. It is NOT a Sigstore bundle (those are for file attestation). The `audit.current_hash`
field is the bi-directional linkage key.

The `unsigned_fields` used for hash computation match the `stableJson` fields in ZT-Infra's
`ActionAuditor.record()` implementation exactly:
`{actor, action, resource, decision, reason, timestamp, previous_hash}`.
The `scope`, `expiry`, and `repo_context` fields are additional metadata NOT in the original
unsigned payload — they must be validated against the signed `action`/`resource` fields, not
treated as independently authenticated.

This creates one design choice to resolve in the planning phase: either (a) the operator
embeds scope/expiry directly into the ZT-Infra `resource` or `action` field (making them part
of the signed payload) or (b) scope/expiry are out-of-band metadata validated separately.
Option (a) is more tamper-evident and should be the recommendation to the planner.

### Signature Verification Path

```
nono-py/src/override.rs::verify_override(token_json, kms_pubkey_config)
  |
  +-> serde_json::from_str(token_json) -> OverrideToken
  +-> check token.expiry > Utc::now()      [fail-closed]
  +-> check token.repo_context             [fail-closed]
  +-> recompute current_hash = SHA-256(stableJson(unsigned_fields))
  +-> nono::trust::signing::verify_keyed_signature(
          message: current_hash_bytes,
          signature: base64_decode(token.audit.kms_signature.signature),
          public_key: kms_pubkey_config.der_bytes
      )
      [fail-closed: any Err -> raise PyRuntimeError]
  +-> (optional) HTTP POST /actions to ZT-Infra for live revocation check
      [fail-closed: deny or timeout -> raise PyRuntimeError]
  +-> Ok(OverrideGrant { paths, mode, expiry, zt_audit_hash })
```

The `verify_keyed_signature` function in `crates/nono/src/trust/signing.rs` already handles
ECDSA P-256 verification. The KMS output uses the same P-256 curve and DER encoding. The
`normalizeEcdsaDerLowS` normalization applied by ZT-Infra's `audit.js` ensures low-S form,
which is the form `verify_keyed_signature` expects. This reuse is verified by reading the
actual source of both components.

To expose `verify_keyed_signature` to `override.rs` (same Rust workspace via path dep):
`nono-py` already depends on `nono` (it uses `nono::CapabilitySet`, `nono::Sandbox`, etc.).
The call from `override.rs` to `nono::trust::signing::verify_keyed_signature` is a plain
Rust function call — no PyO3 boundary needed for the internal path.

### CapabilitySet Mutation

No new Rust primitive is needed. The existing `append_caps_allow_flags` function in
`windows_confined_run.rs` already converts a list of paths into `--allow` flags on the
`nono.exe run` command. The override grant paths are appended to the same command:

```rust
// Existing code path (unchanged):
build_nono_run_args(&mut cmd, profile.as_deref(), allow.as_deref(), cwd.as_deref());
// New: override grant paths appended via the same mechanism
if let Some(ref grant) = override_grant {
    for path in &grant.paths {
        cmd.arg("--allow").arg(path);
    }
}
```

The `CapabilitySet` builder in `crates/nono` is not mutated at the Rust library level —
the mutation happens entirely at the `nono.exe run` invocation level, staying inside
`nono-py` where policy belongs.

### Audit Chain + SecurityEventLayer Bi-Directional Linkage

The new `AuditEventPayload::PolicyOverrideApplied` variant follows the exact same pattern
as the existing fork extension `RejectStage` (which also adds fields not present in upstream):

```rust
// In crates/nono/src/audit.rs, added to the existing AuditEventPayload enum:
/// A signed policy override was verified and applied to the runtime CapabilitySet.
PolicyOverrideApplied {
    /// Actor identity from the override token.
    actor: String,
    /// Canonical granted paths.
    paths: Vec<String>,
    /// Access mode granted ("read", "write", or "read+write").
    mode: String,
    /// ISO-8601 expiry from the token.
    expiry: String,
    /// The ZT-Infra audit chain hash that authorized this override.
    /// Cross-reference: look up in ZT-Infra CloudWatch / audit-chain.jsonl
    /// to find the original authorization record.
    zt_audit_hash: String,
    /// AWS KMS key ID used for signing (for key rotation auditing).
    kms_key_id: String,
},
```

This event is appended to the nono HMAC chain via `AuditRecorder` before the confined
process is spawned. The `chain_hash` field of the resulting `AuditEventRecord` commits to
all prior events in the session, making the override tamper-evident within the session log.

The SecurityEventLayer in nono-cli handles the new variant identically to existing ones:
emit to `NONO_POLICY_OVERRIDE_APPLIED` (EventID 10006) with named EventData fields matching
the struct. This follows the v3.0 pattern for EventIDs 10001-10005.

---

## PyO3 Python/Rust Split

| Concern | Language | Location | Rationale |
|---------|----------|----------|-----------|
| HTTP round-trip to ZT-Infra | Rust (`ureq` or `reqwest` blocking) | `nono-py/src/override.rs` | Keep in Rust; avoids Python HTTP dep; runs under `py.detach()` GIL release |
| Token JSON parsing | Rust (`serde_json`) | `nono-py/src/override.rs` | Type safety; avoid Python-side field confusion |
| Expiry check (UTC) | Rust (`chrono`) | `nono-py/src/override.rs` | Avoid Python/Rust clock drift; `chrono` already in workspace |
| Repo context binding check | Rust | `nono-py/src/override.rs` | String-exact comparison of canonical git URL |
| ECDSA sig verification | Rust (`nono::trust::signing`) | `nono-py/src/override.rs` → direct crate call | Crypto stays in audited Rust; no PyO3 boundary for the internal call |
| `OverrideGrant` type | Rust `#[pyclass]` | `nono-py/src/override.rs` | Python-consumable typed result |
| `verify_override` | Rust `#[pyfunction]` | `nono-py/src/override.rs` | Optionally exposed to Python for testing; always called from Rust in the hot path |
| CapabilitySet mutation | Rust (inside `confined_run`) | `windows_confined_run.rs` / `sandboxed_exec.rs` | `append_caps_allow_flags` + manual `--allow` args; no new API |
| Audit event emission | Rust | `crates/nono/src/audit.rs` + nono-cli `SecurityEventLayer` | Follows existing AuditRecorder pattern |
| `confined_run` / `confine` signature | Rust `#[pyfunction]` | `windows_confined_run.rs` | Add `override_token: Option<String>` parameter |

GIL handling: the existing `py.detach(|| do_spawn_and_wait(...))` block in `confined_run`
must wrap both the `verify_override` call and the spawn+wait. The structure:

```rust
py.detach(|| {
    let grant = override_token.as_deref()
        .map(|tok| verify_override(tok, &kms_pubkey_config))
        .transpose()?;
    do_spawn_and_wait(build_cmd(nono_path, allow, profile, grant.as_ref()), timeout_secs)
})
```

---

## Fail-Closed Points

| Failure | Behavior | Where enforced |
|---------|----------|---------------|
| Token missing required field | `Err -> PyRuntimeError` before spawn | `override.rs` |
| Token expired | `Err -> PyRuntimeError` before spawn | `override.rs` |
| Repo context mismatch | `Err -> PyRuntimeError` before spawn | `override.rs` |
| Signature verification failure | `Err -> PyRuntimeError` before spawn | `override.rs` via `nono::trust` |
| ZT-Infra live check returns deny | `Err -> PyRuntimeError` before spawn | `override.rs` |
| ZT-Infra HTTP call times out | `Err -> PyRuntimeError` before spawn — never degrade | `override.rs` |
| KMS pubkey not configured but token present | `Err -> PyRuntimeError` before spawn | `override.rs` config validation |
| Audit event append failure | Warn, DO NOT block execution (observability, not a security gate) | `crates/nono/src/audit.rs` |
| SecurityEventLayer emit failure | Warn, DO NOT block execution | nono-cli |

The final two entries follow the existing behavior of `AuditRecorder`: audit is observability.
The OS confinement (Low-IL + AppContainer + Job) is applied by `nono.exe run` regardless.

---

## New vs. Modified Components

### New Components

| Component | File | Purpose |
|-----------|------|---------|
| Override verifier | `nono-py/src/override.rs` | Parse token, verify ECDSA sig via `nono::trust`, check expiry/scope/repo-context, optional live ZT check, produce `OverrideGrant #[pyclass]` |
| KMS pubkey config | env var `NONO_ZT_KMS_PUBKEY` (DER base64) or config file | Pin the KMS public key used to verify tokens; fail-closed if absent when a token is presented |
| `PolicyOverrideApplied` audit variant | `crates/nono/src/audit.rs` | New `AuditEventPayload` variant; carries `zt_audit_hash` for bi-directional linkage; follows fork-extension pattern |
| EventID 10006 | nono-cli `SecurityEventLayer` | New EventID constant + match arm for `PolicyOverrideApplied`; mirrors existing 10001-10005 pattern |

### Modified Components

| Component | File | Change |
|-----------|------|--------|
| `confined_run` | `nono-py/src/windows_confined_run.rs` | Add `override_token: Option<String>` parameter; call `verify_override`; add grant paths via `--allow` flags; emit audit event inside `py.detach` block |
| `confine` | `nono-py/src/windows_confined_run.rs` | Same override wiring |
| `sandboxed_exec` | `nono-py/src/sandboxed_exec.rs` | Same override wiring for Unix path |
| Module registration | `nono-py/src/lib.rs` | `mod override; m.add_class::<override::OverrideGrant>()?;` |
| `AuditEventPayload` | `crates/nono/src/audit.rs` | New variant (non-breaking: tagged enum, `#[serde(tag="type")]`, existing variants unchanged) |
| SecurityEventLayer match | nono-cli `main.rs` | New arm for `AuditEventPayload::PolicyOverrideApplied` → EventID 10006 |

### Unchanged Components

| Component | Reason |
|-----------|--------|
| `crates/nono/src/capability.rs` | `CapabilitySet` builder already sufficient; no new API needed |
| `crates/nono-cli/src/policy.rs` | Group/deny resolver unchanged; overrides are additive, not a policy group |
| `crates/nono/src/trust/signing.rs` | Reused as-is via direct crate call from `override.rs` |
| `crates/nono/src/sandbox/` | OS confinement layer completely unchanged |
| ZT-Infra v2 codebase | External dependency; nono consumes existing `POST /actions` API unchanged |

---

## Suggested Build Order

Dependency order: verify infrastructure before integration, integration before emission,
emission before verification closure.

**Wave 1 — Token verifier (no AWS, no spawn)**

Build `nono-py/src/override.rs`:
- `OverrideToken` (serde), `OverrideGrant` (`#[pyclass]`), `verify_override` function
- Expiry check, repo context check, ECDSA sig verify via `nono::trust::signing`
- Unit tests using `generate_signing_key` + `sign_bytes` from `trust/signing.rs` (no AWS, no KMS)
- Test fail-closed paths: expired, bad sig, wrong repo, missing fields
- Test that `override_token = None` produces identical behavior to today

Wave 1 is fully self-contained on any dev host. No AWS, no local provisioner needed.

**Wave 2 — CapabilitySet mutation wiring**

Wire `override_token: Option<String>` into `confined_run`, `confine`, and `sandboxed_exec`:
- Call `verify_override` when token present; fail-closed on any error
- Add grant paths as `--allow` flags via existing `append_caps_allow_flags` or inline `cmd.arg`
- Integration test: a token granting `/tmp/override-dir` produces a `nono.exe run` invocation
  that includes `--allow /tmp/override-dir` and the path is accessible inside the confined process
- Verify no-token path is byte-for-byte identical to pre-v3.2 (regression gate)

**Wave 3 — Audit + SecurityEventLayer wiring**

Add `AuditEventPayload::PolicyOverrideApplied` and EventID 10006:
- Unit test: new variant serializes to expected JSON; chain hash advances correctly
- Unit test: existing variants' serialization is unchanged (regression)
- `SecurityEventLayer` arm emits EventID 10006 with correct EventData field names
- Cross-target clippy: `audit.rs` is cfg-unconditional; Windows and Linux clippy both cover it.
  The SecurityEventLayer match is in nono-cli which may have `cfg(windows)` guards on the ETW
  emitter — verify no `unreachable_patterns` warning on Linux CI

**Wave 4 — Live ZT-Infra integration (host-gated)**

Add the optional live-check `POST /actions` call inside `verify_override` controlled by
`NONO_ZT_ACTIONS_URL` (unset = offline-only; set = live check enabled):
- Local provisioner (`cd provisioner && npm install && npm start`) fully testable on dev host
- AWS KMS path is host-gated; scripted `verify-dark.ps1` gate per Dark Factory mandate
- DAAL ledger linkage (`token.audit.daal`) recorded in `PolicyOverrideApplied` as an
  informational field; not a required verification gate in v3.2

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Verify inside `crates/nono` (core library)

Adding a `CapabilitySet::apply_override(token, kms_pubkey)` method or similar to the library
makes it opinionated about the external authority format and key pinning policy. Every downstream
client embedding the library would be forced into the ZT-Infra model. The CLAUDE.md boundary
table is explicit on this. Keep `crates/nono` to the `AuditEventPayload` variant (observability
primitive) and the existing `trust/signing` ECDSA primitive. Everything ZT-Infra-specific goes
in `nono-py`.

### Anti-Pattern 2: Using `decision: "allow"` as the sole gate

The `decision` field is unauthenticated JSON text. An attacker can craft `{"decision":"allow"}`
with any fields. The signature over `current_hash` — which commits to the `decision` field via
`stableJson` — is the only authenticating gate. Verification order in `verify_override` must be:
parse → verify signature → then inspect semantic fields (scope, expiry, decision).

### Anti-Pattern 3: Expanding beyond the signed scope

The override grants exactly the paths listed in `scope.paths` at `scope.mode`. Any expansion
(adding parent directories, upgrading READ to READ_WRITE) violates the signed grant. Apply
`grant.paths` exactly; fail-closed if the scoped paths are insufficient for the task. The
developer should request a broader token from the operator.

### Anti-Pattern 4: Silent fallback on token verification failure

`confined_run(..., override_token=bad_token)` must raise `PyRuntimeError` and spawn nothing.
Never silently fall back to the base profile: the caller does not know whether the override
was applied, making audit logs unreliable.

### Anti-Pattern 5: Token carries its own verification key

If the token's JSON includes the public key used to verify it, an attacker provides both the
data and the verification key. The KMS public key must be pinned in the deployment configuration
(`NONO_ZT_KMS_PUBKEY` env var or operator-controlled config), not inside the token.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| ZT-Infra `POST /actions` (local or AWS) | Blocking HTTPS POST from `override.rs`; timeout enforced; response parsed for `decision` + `audit` | URL via `NONO_ZT_ACTIONS_URL`; unset = offline token-only mode |
| AWS KMS | NOT called directly by nono; KMS is called by ZT-Infra; nono only verifies the KMS-produced signature using a pinned public key | Avoids AWS SDK as a nono-py dependency |
| ZT-Infra DAAL ledger | `token.audit.daal` recorded in the nono audit event as informational; not verified by nono in v3.2 | Full DAAL reconciliation is a future enhancement |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `nono-py/override.rs` -> `crates/nono/trust/signing.rs` | Direct Rust crate call (`nono-py` already depends on `nono`); no PyO3 boundary | `verify_keyed_signature(message, sig, pubkey)` -> `Result<(), NonoError>` |
| `windows_confined_run.rs` -> `override.rs` | Direct Rust call (same crate); `verify_override` returns `Result<OverrideGrant, PyErr>` | Both in `nono-py`; no subprocess or IPC |
| `crates/nono/audit.rs` -> nono-cli `SecurityEventLayer` | Existing `tracing` event mechanism; new EventID 10006 arm in the `AuditEventPayload` match | Follows exact same pattern as EventIDs 10001-10005 from v3.0 |
| `confined_run` (nono-py) -> `nono.exe run` (nono-cli) | Unchanged subprocess spawn; override just adds more `--allow` flags | No new IPC; existing `build_nono_run_args` + `--allow` pattern reused |

---

## Sources

- `CLAUDE.md` §"Library vs CLI Boundary" (confirmed current; post-ADR-86)
- `proj/DESIGN-engine-abstraction.md` §"E5" + §"Forward-Compat: zt-infra.org Integration"
- `proj/POC-zt-infra-e5-local-provisioner.md` — E5 composition proof and `POST /actions` data shape
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\audit.js` — `ActionAuditor.record()`; KMS signing; `normalizeEcdsaDerLowS`; hash-chain structure
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\server.js` — `POST /actions` request/response shape; `correlation_id` field
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\provisioner\src\policy.js` — policy evaluation; default-deny shape
- `C:\Users\OMack\ZeroTrust2\ZERO_TRUST_V2\docs\ARCHITECTURE.md` — layer boundaries; ZT-Infra/nono composition model
- `crates/nono/src/audit.rs` — `AuditEventPayload` enum; `AuditEventRecord` chain; fork extension pattern (`RejectStage`)
- `crates/nono/src/trust/mod.rs` + `trust/signing.rs` — `verify_keyed_signature`, `verify_bundle_keyed`
- `nono-py/src/windows_confined_run.rs` — `confined_run`, `confine`, `append_caps_allow_flags`, `build_nono_run_args`, `py.detach` GIL pattern
- `nono-py/src/lib.rs` — PyO3 module registration pattern; `mod` declarations
- `.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md` — scope, breadcrumbs, design intent
- `.planning/PROJECT.md` §"Current Milestone: v3.2" — enforcement surface decision, fail-closed requirement, AWS-depth requirement

---
*Architecture research for: Signed Policy Overrides (ZT-Infra Attestation) — v3.2*
*Researched: 2026-06-21*
