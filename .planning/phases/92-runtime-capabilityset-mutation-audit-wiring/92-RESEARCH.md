# Phase 92: Runtime CapabilitySet Mutation + Audit Wiring — Research

**Researched:** 2026-06-22
**Domain:** Cross-repo override wiring — nono-py execution path + nono-cli HMAC audit chain
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (nono-py verifies + applies; nono-cli audits + gates the spawn):** Verifier stays in nono-py. Flow: `verify_override()` → `OverrideGrant` → sanitize `scope_paths` → append as `--allow` flags → pass trusted audit metadata via a new flag → nono-cli emits `PolicyOverrideApplied` into the SecurityEventLayer HMAC chain and aborts before spawning if the event cannot be committed.
- **D-02 (Mandatory flag + bilateral capability gate):** Override-granted `--allow` flags passed only alongside a new required flag `--override-audit <metadata>`. CLI side: fatal abort before spawn if paths present without committed `PolicyOverrideApplied`. nono-py side: refuses to launch unless target nono.exe advertises override support.
- **D-03 (Offline arm + composition seam only):** Phase 92 wires the offline verify into `confined_run`/`confine` and builds the explicit Phase 93 live-arm composition seam. VFY-01 recorded PARTIAL with `[BLOCKING-93]`.
- **D-04 (Windows `confined_run`/`confine` only):** `#[cfg(windows)]` only. Non-Windows `sandboxed_exec` seam documented only.
- **D-05 (Reuse nono-cli capability layer; sanitize grant paths in nono-py):** nono-py sanitizes grant paths (reject `..`, require absolute, canonicalize), then appends verbatim. nono-cli existing component-wise `Path::starts_with` enforces SC3 at the OS boundary. No new matching engine.
- **D-06 (Extend verified grant for audit fields; never re-parse the token):** Add read-only `zt_audit_hash` getter to `OverrideGrant` sourced from `token.current_hash`. All audit fields read from the already-verified grant, never by re-parsing the token.

### Claude's Discretion

- Exact name/format of the new audit-metadata flag (`--override-audit` working name) and whether metadata is JSON / base64 / repeated flags.
- The CLI capability-advertisement / min-version probe mechanism — must be fail-closed per D-02.
- Where `PolicyOverrideApplied` fields land on `AuditEventPayload` vs `SecurityEventLayer` emission shape, and how EventID 10006-10010 constants are defined in `telemetry/event.rs`.
- Redaction shape for override events — AUD-03.
- OVERRIDE-01 gate structure under `scripts/gates/` and test-token minting reusing Phase 91 committed local ECDSA P-256 test keypair.

### Deferred Ideas (OUT OF SCOPE)

- Live `POST /actions` two-key AND gate (VFY-01 clause b), revocation enforcement, AWS credential stripping, `nono override request/apply` CLI, DAAL anchoring — Phase 93.
- Non-Windows override wiring (`sandboxed_exec` / Landlock / Seatbelt parity) — future; seam documented only.
- Cross-process / persistent `jti` store — live ZT check is the durable enforcement point.
- `nono-ts` binding parity — FUT-03.
- Reconciling nono-side override-token wire shape with real KMS-issued tokens — Phase 91 D-06 `[BLOCKING]` for Phase 93.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MUT-01 | Verified override additively expands CapabilitySet for the specific repo context, scoped to only the granted paths/domains | Append Phase 91 `OverrideGrant.scope_paths` as `--allow` flags in `build_nono_run_args`; nono-cli existing capability layer handles the expansion |
| MUT-02 | Override is invocation-scoped — does not persist across `confined_run`/`confine` calls and does not mutate shared/global state | `OverrideGrant` passed per-call into `build_nono_run_args`; no global override store added |
| MUT-03 | Override cannot remove/weaken deny rules and cannot bypass the OS confinement layer | Additive `--allow` flags only; nono.exe still applies the full AppContainer+WFP stack; deny rules in policy.json are untouched |
| MUT-04 | Granted paths/domains matched by path-component/DNS-component comparison; never string `starts_with` | nono-cli's existing `capability.rs::path_covered` uses component-wise `Path::starts_with`; nono-py sanitizes before passing |
| MUT-05 | No-override code path is byte-for-byte identical to pre-v3.2 behavior, regression-proven | `confined_run`/`confine` default path (no `override_token`) constructs the same `Command` as today; proven by regression test |
| AUD-01 | Every override lifecycle event emits a security event into the SecurityEventLayer HMAC chain | Add `PolicyOverrideApplied` + `PolicyOverrideRejected` (etc.) events; wire in `execution_runtime.rs` before child spawn |
| AUD-02 | Override events embed ZT-Infra `audit.current_hash` for bi-directional link | `OverrideGrant.zt_audit_hash()` getter exposes `token.current_hash`; included in `PolicyOverrideApplied` fields |
| AUD-03 | EventID range 10006-10010 with redaction (no raw secrets; paths per existing redaction policy) | Define constants in `telemetry/event.rs`; scrub via existing `nono::scrub_value`; path hashed via existing `path_hash_for` |
| AUD-04 | Override that applies without a successfully-emitted audit event is treated as failure | Pre-spawn gate in `execute_sandboxed`/CLI path: if `--override-audit` present but event emission fails → `return Err(...)` before `exec_strategy::run` |
| VFY-01 | Two-key AND gate (PARTIAL — offline arm + composition seam only; live arm BLOCKING-93) | Phase 92 wires offline arm; composition seam documented for Phase 93 |
| DF-01 | `verify-dark.ps1 --gate OVERRIDE-01` gate exercises offline verify path + fail-closed cases | New `scripts/gates/override-01.ps1` following `egress-policy-deny.ps1` contract; mints tokens with Phase 91 test keypair |

</phase_requirements>

---

## Summary

Phase 92 wires the Phase 91 offline override verifier into the `confined_run`/`confine` execution path. The architecture is deliberately split across two repos: nono-py owns verification, path sanitization, and `--allow`-flag construction; nono-cli owns the SecurityEventLayer HMAC audit chain and the pre-spawn AUD-04 gate. The cross-boundary interface is a new CLI flag (`--override-audit`) that carries trusted audit metadata from the verified grant to nono-cli.

The dominant technical challenge is the SecurityEventLayer integration. The current layer intercepts only `tracing::warn!` events on `nono_security::*` targets (target-suffix dispatch in `on_event`). Adding override events requires extending `SecurityEventType` with 5 new variants (10006-10010: `PolicyOverridePresented`, `PolicyOverrideVerified`, `PolicyOverrideRejected`, `PolicyOverrideExpired`, `PolicyOverrideRevoked`), adding 5 new EventID constants to `event.rs`, and extending `on_event`'s target-suffix match arm + `severity_for`. The new variant also needs a dedicated visitor or structured fields approach since override events carry `zt_audit_hash`/`kms_key_id` rather than a path/host.

The AUD-04 bilateral handshake is the security-critical invariant: no override path ever reaches a sandboxed child without a committed audit event. This means the pre-spawn gate in nono-cli must check for the presence of `--override-audit` and block execution if the SecurityEventLayer emission fails. The canonical spawn point is `execute_sandboxed()` in `crates/nono-cli/src/execution_runtime.rs` (line 112), which is called from `run_sandbox()` in `command_runtime.rs` (line 183). The gate must sit between `start_proxy_runtime` and `exec_strategy::run` (i.e., after capabilities are fully resolved but before the child process is ever spawned).

**Primary recommendation:** Add `PolicyOverrideApplied` to `AuditEventPayload` in `crates/nono/src/audit.rs`, add 5 new `SecurityEventType` variants to `event.rs` with a new `emit_override_event()` direct-emit helper (bypassing the tracing-interceptor path since override events carry non-path/non-host audit fields), add `--override-audit <base64-json>` to `SandboxArgs` in `cli.rs`, wire nono-py to pass audit metadata through `build_nono_run_args`, and add a pre-spawn gate in `execute_sandboxed`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Override token verification | nono-py | — | Phase 91 verifier already there; crypto lives only in nono-py; policy-free core invariant |
| `--allow` flag construction from grant | nono-py (`build_nono_run_args`) | — | Existing flag-building function; all other `--allow` sources are in nono-py |
| Grant path sanitization (reject `..`, absolute check, canonicalize) | nono-py | — | Must happen before flags reach CLI; CLI trusts its own `--allow` inputs |
| Audit metadata transport (nono-py → nono-cli) | Process argument channel (`--override-audit`) | — | No IPC channel exists; CLI args are the established nono-py→nono.exe boundary |
| HMAC chain audit emission (`PolicyOverrideApplied`) | nono-cli (`SecurityEventLayer`) | — | SecurityEventLayer lives only in nono-cli; relocating to core violates policy-free boundary |
| AUD-04 pre-spawn gate | nono-cli (`execution_runtime.rs::execute_sandboxed`) | — | Must be in the process that spawns the child; nono-py cannot gate nono-cli's fork |
| EventID 10006-10010 constants | nono-cli (`telemetry/event.rs`) | — | All existing EventID constants are there; symmetry and co-location |
| `AuditEventPayload::PolicyOverrideApplied` variant | nono core (`audit.rs`) | — | All `AuditEventPayload` variants live in core; AUD-01 requires it in the HMAC chain |
| Capability-advertisement / min-version probe | nono-cli (`cli.rs` env handshake) | nono-py | nono.exe must expose the signal; nono-py reads it |
| DF-01 gate script | scripts/gates (`override-01.ps1`) | — | Gate auto-discovery is file-based; new gate file is the only addition needed |

---

## Canonical Reference Verification

All CONTEXT.md canonical refs were verified against live code. Corrections follow.

### nono-py/src/override.rs

| Symbol | CONTEXT.md Citation | Actual Line | Status |
|--------|--------------------|-----------:|--------|
| `verify_override()` #[pyfunction] | :838 | 840 | CONFIRMED (2-line drift; functionally correct) |
| `OverrideGrant` #[pyclass] | :663 | 663 | EXACT |
| `OverrideToken` struct | :332 | 332 | EXACT |
| `current_hash` field on `OverrideToken` | :366 | 366 | EXACT |
| `kms_signature.key_id` | :312 | 312 | EXACT |
| `OverrideErrorKind` enum | :56 | 56 | EXACT |
| `partition_scope()` | :918 | 918 | EXACT |

**Key finding — `zt_audit_hash` getter does NOT yet exist.** `OverrideGrant` currently exposes `signer`, `scope_paths`, `scope_domains`, `not_before`, `expires_at`, `jti`, `repo_context`. The D-06 `zt_audit_hash` getter is Phase 92 work. The data is available at `token.current_hash` (Option<String>) which flows into `OverrideGrant` construction (line 797). [VERIFIED: direct read of nono-py/src/override.rs]

**Key finding — `OutOfScope` variant is forward-declared.** Line 73: `#[expect(dead_code, reason = "forward-declared for Phase 92 scope enforcement")]`. Phase 92 must remove this `#[expect]` when it adds the first non-test construction site. [VERIFIED: direct read]

### nono-py/src/windows_confined_run.rs

| Symbol | CONTEXT.md Citation | Actual Line | Status |
|--------|--------------------|-----------:|--------|
| `confined_run()` | :176 | 176 | EXACT |
| `confine()` | :255 | 255 | EXACT |
| `build_nono_run_args()` | :104 | 104 | EXACT |
| `append_caps_allow_flags()` | :133 | 133 | EXACT |

**Key finding — override hook location.** In `confined_run`, the override `--allow` flags and `--override-audit` flag must be appended inside `build_nono_run_args` (called at line 197) or after that call and before `cmd.arg("--").arg(&exe)` (line 203). In `confine`, the same hook sits between `build_nono_run_args` (line 288) + `append_caps_allow_flags` (line 297) and `cmd.arg("--").arg(&current_exe)` (line 303). [VERIFIED: direct read]

**Key finding — signatures need an `override_token: Option<&OverrideGrant>` parameter.** Both functions currently have no override parameter. Phase 92 must extend both signatures. `confined_run` is a `#[pyfunction]` with `#[pyo3(signature = (...))]` — the Python-visible signature changes. [VERIFIED: direct read]

### nono-py/src/lib.rs

| Symbol | CONTEXT.md Citation | Actual Line | Status |
|--------|--------------------|-----------:|--------|
| Module registration block | :733-769 | 722-775 | CONFIRMED (range shifted by ~11 lines) |
| `#[cfg(windows)]` confined_run/confine registrations | :761-764 | 762-764 | CONFIRMED |

### nono core — crates/nono/src/audit.rs

| Symbol | CONTEXT.md Citation | Actual Line | Status |
|--------|--------------------|-----------:|--------|
| `AuditEventPayload` enum | :67 | 67 | EXACT |
| `PolicyOverrideApplied` variant | (new, to add here) | NOT YET PRESENT | CONFIRMED ABSENT — must be added |

**Existing variants for context:** `SessionStarted`, `SessionEnded`, `CapabilityDecision`, `UrlOpen`, `Network`. The new `PolicyOverrideApplied` variant follows the same `#[serde(tag = "type", rename_all = "snake_case")]` pattern. [VERIFIED: direct read of audit.rs:67-111]

### nono-cli — crates/nono-cli/src/telemetry/event.rs

| Symbol | CONTEXT.md Citation | Actual Line | Status |
|--------|--------------------|-----------:|--------|
| EventID constants 10001-10005 | :30-41 | 31-41 | CONFIRMED (1-line drift; match exact) |
| `SecurityEventType` enum | — | 52-64 | VERIFIED |
| EventIDs 10006-10010 | (new, to add) | NOT YET PRESENT | CONFIRMED ABSENT — must be added |

**Key finding — current SecurityEventType has 5 variants** (PathDeny, NetworkDeny, LabelViolation, HookFailClosed, TelemetryDegraded) mapped to EventIDs 10001-10005 via `event_id_for()`. The function uses an exhaustive `match` — adding new variants will require extending the match arm (and `severity_for` in mod.rs, and `on_event` target-suffix dispatch). [VERIFIED: direct read of event.rs]

### nono-cli — crates/nono-cli/src/telemetry/mod.rs

| Symbol | CONTEXT.md Citation | Actual Line | Status |
|--------|--------------------|-----------:|--------|
| `SecurityEventLayer` struct | :189 | 189 | EXACT |
| `chain_sequence()` test accessor | :210 | 211 | CONFIRMED (1-line drift) |
| `on_event()` method | :259 | 260 | CONFIRMED (1-line drift) |
| `advance_chain()` | :113 | 113 | EXACT |

**Key finding — override events cannot use the current `on_event` tracing-intercept path as-is.** The current path dispatches on `nono_security::*` target suffixes: `path_deny`, `network_deny`, `label_violation`, `hook_fail_closed`, `telemetry_degraded`. Override events carry `zt_audit_hash` and `kms_key_id` fields — neither `path` nor `host`. Two design options:

1. **Option A (extend on_event path):** Add new target suffixes (`nono_security::policy_override_applied`, etc.), add new fields to `SecurityEventVisitor`, extend the event-type match arm in `on_event`. Emitter calls `tracing::warn!(target: "nono_security::policy_override_applied", zt_audit_hash = %hash, ...)`.
2. **Option B (direct-emit helper):** Add a new `emit_override_event()` method on `SecurityEventLayer` that calls `advance_chain` + `windows::emit_security_event` directly, bypassing tracing interception. The nono-cli pre-spawn gate calls this method directly.

**Recommendation: Option B.** The `on_event` tracing-intercept path was designed for denial events fired from deep in the sandbox supervisor (where `SecurityEventLayer` is not in scope but tracing macros are). The override audit event is fired from `execute_sandboxed` where the `SecurityEventLayer` instance is accessible. A direct method is simpler, testable with `chain_sequence()`, and avoids extending the visitor with override-specific fields. [ASSUMED — architectural judgment, not a locked upstream pattern]

### nono-cli — crates/nono/src/capability.rs

| Symbol | CONTEXT.md Citation | Actual Line | Status |
|--------|--------------------|-----------:|--------|
| Component-wise `Path::starts_with` matching | :~1239-1244 | 1762-1766 | CORRECTED — lines cited are the `set_network_blocked`/`set_network_mode_mut` methods; the actual component-wise matching is in `path_covered()` at line 1762 and `path_covered_with_access()` at line 1774 |

**Corrected ref:** `path_covered(&self, path: &Path) -> bool` at line 1762, using `path.starts_with(&cap.resolved)` where `cap.resolved` is a `PathBuf` (so this is `Path::starts_with`, NOT string `starts_with`). Also `path_covered_with_access` at line 1774. [VERIFIED: direct read of capability.rs]

### ZT-Infra v2 — current_hash semantics

`provisioner/src/audit.js` line 240: `current_hash = sha256(stableJson(unsigned))`. The `unsigned` object is the pre-signature record (actor, action, resource, decision, reason, timestamp, previous_hash). The `current_hash` is the SHA-256 hex of the canonical (stableJson) unsigned record — it is the hash of the content BEFORE the KMS signature is appended. `zt_audit_hash` in the nono audit record links to this value, creating a bi-directional link. [VERIFIED: direct read of audit.js:236-251]

---

## Standard Stack

### Core (no new crates — additive only)

| Library | Location | Purpose | Why Standard |
|---------|----------|---------|--------------|
| `nono-py/src/override.rs` | nono-py | Phase 91 verifier — source of `OverrideGrant` | Already shipped; Phase 92 reads from it |
| `nono-py/src/windows_confined_run.rs` | nono-py | `confined_run`/`confine` + `build_nono_run_args` | Established execution entry point |
| `crates/nono/src/audit.rs::AuditEventPayload` | nono core | Append-only audit payload enum | All audit events use this |
| `crates/nono-cli/src/telemetry/` | nono-cli | `SecurityEventLayer` HMAC chain | Existing v3.0/v3.1 chain |
| `crates/nono-cli/src/execution_runtime.rs::execute_sandboxed` | nono-cli | Pre-spawn gate location | Only spawn point for `nono run` |
| `crates/nono-cli/src/cli.rs::SandboxArgs` | nono-cli | `--override-audit` flag target | All `nono run` flags live here |

### Supporting

| Library | Location | Purpose | When to Use |
|---------|----------|---------|-------------|
| `nono::scrub_value` | nono core | Redaction for free-text fields | All override event fields that are not zt_audit_hash or jti |
| `telemetry::event::path_hash_for` | nono-cli | Salted path hashing | For `scope_paths` in AUD-03 path redaction |
| `serde_json` | workspace dep | JSON metadata for `--override-audit` payload | Serialize/deserialize the audit metadata struct |
| `base64` | already in nono-py Cargo.toml | Encode `--override-audit` for single-arg CLI transport | Keep the flag to one argument |

**Package Legitimacy Audit:** No new external packages. All code reuses existing workspace dependencies. [VERIFIED: no new Cargo.toml entries required in nono or nono-cli]

---

## Package Legitimacy Audit

No new packages are installed in this phase. All implementation reuses existing workspace dependencies. Audit table is not applicable.

---

## Architecture Patterns

### System Architecture Diagram

```text
NONO-PY (Python binding)
  confined_run(override_token=Some(grant))
       │
       ▼
  verify_override() [Phase 91, already done]
       │ OverrideGrant
       ▼
  sanitize_grant_paths()              ← rejects non-absolute, canonicalizes, rejects ..
       │ Vec<String> (safe paths)
       ▼
  build_nono_run_args()               ← appends --allow <path> per grant path
       │ also appends --override-audit <base64-json>
       ▼
  nono.exe run --profile X            ← existing flag channel
              --allow <p1>            ← existing grant path (from grant)
              --allow <p2>            ← (repeated for each scope_path)
              --override-audit <meta> ← NEW: carries zt_audit_hash, kms_key_id, jti, paths[]
              -- <exe> <args>

NONO-CLI (nono.exe run)
  SandboxArgs (parse --override-audit) ← NEW clap field
       │ OverrideAuditMeta { zt_audit_hash, kms_key_id, jti, granted_paths }
       ▼
  prepare_run_launch_plan()
       │ LaunchPlan (flags.override_audit = Some(meta))
       ▼
  execute_sandboxed()
       │
       ├── [existing] start_proxy_runtime + caps resolution
       │
       ├── [NEW — AUD-04 gate, BEFORE ANY SPAWN]
       │     emit_override_event(&layer, &meta)  → SecurityEventLayer
       │             │
       │             ├── advance_chain(override event bytes)
       │             │       ↕ chain_head advances
       │             └── emit to ETW + App Event Log (EventID 10006)
       │     if emit fails → return Err (abort before spawn)
       │
       └── [existing] exec_strategy::run / execute_supervised_runtime
                        ↓
                  sandboxed child process (OS confinement applied)

REGRESSION PATH (no override_token):
  confined_run(override_token=None)
       │
       ▼
  build_nono_run_args() [identical to pre-v3.2]
       │ no --override-audit flag
       ▼
  nono.exe run [identical CLI args to pre-v3.2]
       │
       ▼
  execute_sandboxed() — override_audit is None → gate skipped
  [byte-for-byte identical behavior — MUT-05]
```

### Recommended Project Structure Changes

```
nono-py/src/
├── override.rs          # MODIFY: add zt_audit_hash getter, sanitize_grant_paths, remove #[expect(dead_code)] on OutOfScope
└── windows_confined_run.rs  # MODIFY: extend confined_run/confine signatures, wire override path

crates/nono/src/
└── audit.rs             # MODIFY: add PolicyOverrideApplied variant to AuditEventPayload

crates/nono-cli/src/
├── cli.rs               # MODIFY: add --override-audit to SandboxArgs + ShellArgs; add NONO_OVERRIDE_SUPPORT env constant
├── telemetry/
│   ├── event.rs         # MODIFY: add EVENT_ID_* constants 10006-10010, SecurityEventType variants, event_id_for arms, severity_for arms
│   └── mod.rs           # MODIFY: add emit_override_event() direct-emit method on SecurityEventLayer
├── sandbox_prepare.rs   # LIKELY MODIFY: parse/validate --override-audit metadata into OverrideAuditMeta
├── launch_runtime.rs    # MODIFY: add override_audit: Option<OverrideAuditMeta> to LaunchPlan / ExecutionFlags
└── execution_runtime.rs # MODIFY: pre-spawn AUD-04 gate in execute_sandboxed()

scripts/gates/
└── override-01.ps1      # NEW: OVERRIDE-01 gate (Test-Precondition + Invoke-Gate contract)
```

### Pattern 1: extend SandboxArgs for `--override-audit`

**What:** Add a new optional CLI flag to `SandboxArgs` in `cli.rs`.
**When to use:** Whenever nono-py passes override audit metadata to nono-cli.
**Precedent:** All existing optional security flags follow this pattern.

```rust
// crates/nono-cli/src/cli.rs — inside SandboxArgs
/// Signed override audit metadata (base64-encoded JSON).
/// Present only when nono-py has verified an override and passed
/// trusted audit metadata for the SecurityEventLayer HMAC chain.
/// nono-cli treats --allow paths as override-granted if and only if
/// this flag is also present (AUD-04 bilateral gate).
/// INTERNAL — not intended for direct user invocation.
#[arg(long, value_name = "META", help_heading = "OVERRIDE")]
#[serde(skip)]
pub override_audit: Option<String>,
```

**The metadata struct** (recommend inline base64-JSON):
```rust
// crates/nono-cli/src/cli.rs or sandbox_prepare.rs
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OverrideAuditMeta {
    pub zt_audit_hash: Option<String>,  // from OverrideGrant.zt_audit_hash()
    pub kms_key_id: String,             // from OverrideGrant.signer
    pub jti: String,                    // from OverrideGrant.jti
    pub granted_paths: Vec<String>,     // from OverrideGrant.scope_paths (already sanitized)
    pub expires_at: String,             // from OverrideGrant.expires_at
}
```

### Pattern 2: `emit_override_event()` direct-emit method on SecurityEventLayer

**What:** A method that accepts `OverrideAuditMeta` + `SecurityEventType` (one of the 10006-10010 variants) and advances the chain + emits without going through the tracing interception path.
**When to use:** Called from `execute_sandboxed` before spawn for AUD-01/AUD-04.

```rust
// crates/nono-cli/src/telemetry/mod.rs
impl SecurityEventLayer {
    /// Emit an override lifecycle event directly into the HMAC chain.
    ///
    /// Bypasses the tracing-intercept path (on_event) because override events
    /// carry zt_audit_hash/kms_key_id fields, not path/host.
    ///
    /// Returns Ok(()) if the event was emitted and the chain advanced.
    /// Returns Err if the internal mutex is poisoned or telemetry is disabled.
    /// AUD-04: callers MUST treat Err as fatal (abort before spawn).
    #[must_use]
    pub fn emit_override_event(
        &self,
        event_type: SecurityEventType,  // one of PolicyOverride* variants
        meta: &OverrideAuditMeta,
    ) -> Result<String, &'static str> {
        // Lock, advance_chain, emit, return chain_head or Err
    }
}
```

The event bytes fed to `advance_chain` include: `event_type | jti | kms_key_id | zt_audit_hash | session_id | ts`.

### Pattern 3: `PolicyOverrideApplied` in AuditEventPayload

**What:** New variant in the existing enum in `crates/nono/src/audit.rs`.
**When to use:** Emitted when a verified override is about to be applied.

```rust
// crates/nono/src/audit.rs — inside AuditEventPayload
/// A signed policy override was verified and applied to this invocation.
PolicyOverrideApplied {
    /// JWT ID of the applied override token (single-use nonce).
    jti: String,
    /// Signing key identity (KMS key ARN) — redaction-safe (not raw DER).
    kms_key_id: String,
    /// ZT-Infra audit chain hash at time of override issuance (bi-directional link).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    zt_audit_hash: Option<String>,
    /// Paths granted by this override (path-hashed per existing redaction policy, not raw).
    granted_path_hashes: Vec<String>,
    /// ISO-8601 expiry timestamp.
    expires_at: String,
},
```

Note: `granted_path_hashes` uses the existing `path_hash_for(session_salt, path)` pattern for AUD-03 redaction — raw paths never appear in the audit record. [ASSUMED — field names pending planner decision, but structure is architecturally correct]

### Pattern 4: Capability-advertisement / min-version probe (D-02, Claude's Discretion)

**Recommendation: environment-variable handshake (Option C), not `--version` gate.**

Three options were evaluated:

| Option | Mechanism | Fail-closed guarantee | Overhead |
|--------|-----------|----------------------|----------|
| A. `--version` gate | `nono --version` → parse version string; require ≥ 3.2.0 | No — version string comparison is fragile and may pass a fork that never implemented the feature | Subprocess call |
| B. `--capabilities` query | `nono --capabilities` → parse JSON | Yes — but requires adding a new top-level CLI subcommand | Subprocess call |
| C. `NONO_OVERRIDE_SUPPORT=1` env sentinel | nono.exe sets this env var in child when `--override-audit` is parsed successfully | Yes if nono.exe sets it; but nono-py must check it before passing the flag | Requires nono-py to check env before each invocation |
| D. Env probe: nono-py runs `nono --override-audit ""` and checks exit code | nono-cli: unknown flag → clap error non-zero exit | Yes — clap treats unknown flags as errors; a too-old nono.exe without `--override-audit` will exit non-zero | One subprocess call per invocation |

**Recommendation: Option D (fail-closed probe at first invocation, cached).** nono-py runs `nono run --override-audit ""` before the real invocation and checks the exit code. An old nono.exe (without `--override-audit`) exits with clap's "unexpected argument" error (non-zero). A new nono.exe exits with "requires a value" or similar (also non-zero if `""` is an empty required-value). To make this fully reliable, nono-cli should treat `--override-audit` presence as a signal that override is active and validate that the value is well-formed JSON — an empty value is an error.

**Simpler alternative (strongly recommended for v1):** nono-py reads `nono --version` and checks for `v3.2.*` or later in the version string. This is the lowest-complexity option and acceptable for the closed-universe of nono-py+nono.exe always shipped together. The real fail-closed property is the AUD-04 gate inside nono-cli, not the probe. [ASSUMED — final choice is Claude's Discretion per CONTEXT.md D-02]

### Pattern 5: `scripts/gates/override-01.ps1` structure

**Analog:** `scripts/gates/egress-policy-deny.ps1` — the closest structural twin. Copy the Test-Precondition/Invoke-Gate contract verbatim.

**What OVERRIDE-01 proves (DF-01):**
- SC1: `confined_run(override_token=valid)` → nono.exe receives `--override-audit` + override `--allow` flags → audit chain advances (chain_sequence > 0 after run).
- SC2: fail-closed cases all raise `NonoOverrideError` from Python: bad sig, expired, out-of-scope, replay, algorithm:none.
- SC3: `confined_run(override_token=None)` produces identical `nono.exe run` args to pre-v3.2 (no `--override-audit` flag).

**Precondition:** `nono.exe` on PATH, Python + nono-py importable. Admin NOT required. This gate is non-host-gated by design (offline verify, no WFP/daemon).

```powershell
# scripts/gates/override-01.ps1
# CONTRACT: same as egress-policy-deny.ps1
# Test-Precondition -> $null | "reason string"
# Invoke-Gate      -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                     NEVER calls exit. NEVER calls Persist-Verdict.

function Test-Precondition {
    # 1. nono on PATH
    if (-not (Get-Command nono -ErrorAction SilentlyContinue)) {
        return 'nono is not on PATH — build/install nono before running this gate'
    }
    # 2. Python + nono module importable
    $check = & python -c "import nono; print('ok')" 2>&1
    if ($check -ne 'ok') {
        return "nono Python module not importable: $check"
    }
    return $null
}
```

**Test token minting:** Reuse `tests/fixtures/override_test_key.pem` (committed ECDSA P-256 keypair). The gate calls `verify_override(token_json, pubkey_der, allowed_arns)` via Python inline script. [VERIFIED: nono-py/tests/fixtures/ contains override_test_key.der, override_test_key.pem, override_test_private.der, vectors.json]

### Anti-Patterns to Avoid

- **Emitting `PolicyOverrideApplied` into the HMAC chain AFTER spawn:** The child must never run against override paths without a committed audit record. The gate is strictly pre-spawn.
- **Re-parsing the token in nono-cli:** D-06 explicitly prohibits this. All audit fields come from the already-verified `OverrideGrant` (passed via `--override-audit`). nono-cli trusts the metadata flag with the same trust level as `--allow` flags.
- **Using string `starts_with()` for path scope matching in nono-py sanitization:** Must use `Path::starts_with()` for any component-wise check. The sanitization phase at the nono-py boundary rejects `..` and requires absolute — it does not need to match against granted paths (that's nono-cli's existing capability layer job).
- **Adding override paths without `--override-audit` flag:** The bilateral gate requires both sides. A nono.exe that silently accepts extra `--allow` flags without `--override-audit` is a violation of AUD-04. The gate in `execute_sandboxed` must check: if any override paths present AND `override_audit` is None, treat as FATAL.
- **Allowing `zt_audit_hash` to be `None` to silently succeed:** If `OverrideGrant.zt_audit_hash()` returns `None` (token was created before ZT-Infra assigned a `current_hash`), the `PolicyOverrideApplied` event MUST still emit with `zt_audit_hash: null`. This is not a failure condition — `current_hash` is `Option<String>` by design in CAF v0.1.
- **Cross-target clippy regression:** `PolicyOverrideApplied` is added to `audit.rs` (cfg-unconditional) and the new `emit_override_event` is in `telemetry/mod.rs`. The `SecurityEventType` match arms in `severity_for` and `event_id_for` must be exhaustive on both Linux and macOS. Cross-target clippy is MANDATORY per CLAUDE.md.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HMAC chain advancement | Custom HMAC | `advance_chain()` in `telemetry/mod.rs:113` | Existing implementation with zeroize, domain separators, D-14 degrade |
| Path component matching | String `starts_with` | `Path::starts_with(&resolved)` in `capability.rs:1762` | Footgun documented in CLAUDE.md; existing implementation correct |
| Path redaction in override events | Raw path in event fields | `path_hash_for(session_salt, path)` from `telemetry/event.rs` | Existing salted-SHA256 approach; consistent with SC-3 |
| Free-text scrubbing | Custom regex | `nono::scrub_value(&str)` | Existing tested scrubber |
| Event JSON serialization | Custom format string | `serde_json::to_string` + existing `#[serde(rename_all = "PascalCase")]` | Consistent with `SecurityEvent` field naming |
| Base64 encoding of audit metadata | Custom encoding | `base64` crate already in `nono-py/Cargo.toml` | Already a direct dep since Phase 91 |

---

## Common Pitfalls

### Pitfall 1: SecurityEventLayer is behind a Mutex — emit can fail silently
**What goes wrong:** `emit_override_event` locks the Mutex; if poisoned (a previous panic in a test or production thread), it returns the genesis value / Err. If the caller treats this as success, AUD-04 is violated.
**Why it happens:** `Mutex::lock()` returns `Err` on poisoning. The existing code on `on_event` returns silently (fail-silent D-03). But the pre-spawn gate needs fail-closed.
**How to avoid:** `emit_override_event` must return a `Result` (not `void`). `execute_sandboxed` checks the result and returns `Err(NonoError::SandboxInit(...))` before proceeding.
**Warning signs:** A test that poison-tests the mutex returning `Ok` from `emit_override_event`.

### Pitfall 2: Cross-target clippy — new SecurityEventType variants in exhaustive match arms
**What goes wrong:** `event_id_for` and `severity_for` in `event.rs`/`mod.rs` use exhaustive `match` on `SecurityEventType`. Adding 5 new variants without updating these matches produces a compile error on all platforms — but if the developer only tests on Windows, they might miss that the Linux clippy check would also fail (it will, because `event.rs` is cfg-unconditional).
**Why it happens:** Windows-only compilation doesn't exercise Linux codepaths.
**How to avoid:** CLAUDE.md MUST rule — run `cargo clippy --workspace --target x86_64-unknown-linux-gnu` after any `event.rs` change. The cross-target check is not optional.
**Warning signs:** CI failing on Linux after a Windows-green local pass.

### Pitfall 3: AUD-04 window between override-path resolution and spawn
**What goes wrong:** Override audit metadata is parsed during `prepare_run_launch_plan`, but the emit happens in `execute_sandboxed` (later in the call chain). If any path between those two points can return early (e.g., dry-run, blocked-command check), the gate might be skipped.
**Why it happens:** `execute_sandboxed` already has multiple early-return paths (`check_blocked_command`, proxy-only mode guard, etc.) before the spawn point.
**How to avoid:** The `emit_override_event` + AUD-04 check must be placed AFTER all other early returns but BEFORE the first call that could reach `exec_strategy::run`. In `execute_sandboxed`, this is after `start_proxy_runtime` and before `apply_pre_fork_sandbox` (line 261).
**Warning signs:** A test that passes `--override-audit` to a dry-run invocation and finds the chain did not advance.

### Pitfall 4: `--override-audit` accepted without override paths (one-sided gate violation)
**What goes wrong:** A caller passes `--override-audit` without any additional `--allow` flags. The gate emits a `PolicyOverrideApplied` event but no extra capability was added.
**Why it happens:** The bilateral gate checks separately: (a) override paths present without `--override-audit` → fatal; but (b) `--override-audit` present without override paths is currently undefined.
**How to avoid:** Treat (b) as a warning, not a fatal error — it is non-harmful (no new capabilities granted). The fatal direction is (a) only.

### Pitfall 5: nono-py `confine()` uses `std::process::exit(exit_code)` — override metadata must be passed BEFORE the exit call
**What goes wrong:** `confine()` calls `std::process::exit(exit_code)` after spawning the nono.exe child (line 323). If override metadata is resolved AFTER the child returns (which is impossible since the parent calls exit), the metadata is never emitted.
**Why it happens:** `confine()` is a re-exec pattern — the parent exits with the child's code. There is no "after spawn" callback.
**How to avoid:** Not an issue — the audit emission happens in nono-cli (inside the spawned nono.exe process), not in the nono-py parent. The parent only needs to pass `--override-audit` in the command arguments before spawning.

### Pitfall 6: `OverrideGrant.zt_audit_hash()` adding a getter on a `#[pyclass(frozen)]`
**What goes wrong:** `#[pyclass(frozen)]` prevents mutation but does NOT prevent adding new `#[getter]` methods. However, if the field `current_hash` is not stored in `OverrideGrant` (it is currently absent — see verified ref above), the getter cannot be added without also modifying the struct construction at line 791.
**Why it happens:** Phase 91 `OverrideGrant` was built without `zt_audit_hash` because Phase 92 owns it.
**How to avoid:** D-06 requires adding BOTH the stored field (e.g., `zt_audit_hash: Option<String>`) to `OverrideGrant` AND a `#[getter] fn zt_audit_hash(&self) -> Option<String>` in `#[pymethods]`. Update `verify_override_impl` construction (line 791) to populate `zt_audit_hash: token.current_hash.clone()`.

---

## Code Examples

### Example 1: Extending confined_run to accept an override token

```rust
// nono-py/src/windows_confined_run.rs
#[pyfunction]
#[pyo3(signature = (exe, args, allow=None, profile=None, cwd=None,
                     timeout_secs=None, override_token=None))]
pub fn confined_run(
    py: Python<'_>,
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,
    profile: Option<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
    override_token: Option<PyRef<'_, OverrideGrant>>,  // Phase 92 addition
) -> PyResult<ExecResult> {
    // existing validation ...

    let nono_path = find_nono_exe()?;
    let mut cmd = Command::new(&nono_path);
    cmd.arg("run");
    build_nono_run_args(&mut cmd, profile.as_deref(), allow.as_deref(), cwd.as_deref());

    // Phase 92: append override allow flags + audit metadata
    if let Some(ref grant) = override_token {
        append_override_args(&mut cmd, grant)?;
    }

    cmd.arg("--").arg(&exe).args(&args);
    // ... rest of function
}
```

### Example 2: append_override_args helper

```rust
// nono-py/src/windows_confined_run.rs
fn append_override_args(cmd: &mut Command, grant: &OverrideGrant) -> PyResult<()> {
    // Sanitize and append scope_paths as --allow flags (D-05 / MUT-04)
    for raw_path in &grant.scope_paths {
        let path = sanitize_override_path(raw_path)?;  // rejects .., requires absolute
        cmd.arg("--allow").arg(path.as_str());
    }

    // Build and append --override-audit metadata (D-06: read from grant, never re-parse)
    let meta = serde_json::json!({
        "zt_audit_hash": grant.zt_audit_hash(),   // Option<String>
        "kms_key_id": &grant.signer,
        "jti": &grant.jti,
        "granted_paths": &grant.scope_paths,
        "expires_at": &grant.expires_at,
    });
    let meta_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(meta.to_string().as_bytes());
    cmd.arg("--override-audit").arg(meta_b64);

    Ok(())
}

fn sanitize_override_path(raw: &str) -> PyResult<std::path::PathBuf> {
    let p = std::path::Path::new(raw);
    // Reject non-absolute (Unix-style / or Windows C:\)
    if !raw.starts_with('/') && !p.is_absolute() {
        return Err(PyValueError::new_err(
            format!("override scope path must be absolute: {raw}")
        ));
    }
    // Reject path traversal
    for component in p.components() {
        if component == std::path::Component::ParentDir {
            return Err(PyValueError::new_err(
                format!("override scope path must not contain ..: {raw}")
            ));
        }
    }
    Ok(p.to_path_buf())
}
```

### Example 3: emit_override_event in SecurityEventLayer

```rust
// crates/nono-cli/src/telemetry/mod.rs
impl SecurityEventLayer {
    /// Emit a PolicyOverride lifecycle event directly into the HMAC chain.
    ///
    /// Returns Ok(chain_head_hex) on success.
    /// Returns Err(&str) if the mutex is poisoned or telemetry is disabled.
    /// AUD-04: callers MUST treat Err as FATAL — abort before spawn.
    #[must_use]
    pub fn emit_override_event(
        &self,
        event_type: &SecurityEventType,  // one of PolicyOverride* variants
        jti: &str,
        kms_key_id: &str,
        zt_audit_hash: Option<&str>,
    ) -> Result<String, &'static str> {
        let mut inner = self.inner.lock().map_err(|_| "mutex poisoned")?;
        if !inner.config.enabled {
            // Telemetry disabled → still advance chain so the sequence is correct,
            // but do NOT emit to the event log. Return Ok so the gate passes.
            // (Telemetry-disabled is not an AUD-04 failure — it is a policy choice.)
        }
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let event_bytes = format!(
            "{event_type:?}|{jti}|{kms_key_id}|{zt}|{sid}|{ts}",
            zt = zt_audit_hash.unwrap_or(""),
            sid = inner.session_id,
        );
        advance_chain(&mut inner.chain, event_bytes.as_bytes());
        let chain_head = chain_head_hex(&inner.chain.head);
        // emit to Windows Application log + ETW
        let se = SecurityEvent { event_type: event_type.clone(), ... };
        windows::emit_security_event(&se);
        Ok(chain_head)
    }
}
```

### Example 4: AUD-04 pre-spawn gate in execute_sandboxed

```rust
// crates/nono-cli/src/execution_runtime.rs — inside execute_sandboxed()
// AFTER start_proxy_runtime, BEFORE apply_pre_fork_sandbox

// AUD-04 bilateral gate: if override paths were granted, audit MUST be committed
// before the child spawns. An override that cannot emit its audit record is blocked.
if let Some(ref meta) = flags.override_audit {
    // Retrieve the SecurityEventLayer from the tracing subscriber registry.
    // (The layer was registered at init_tracing() time.)
    // NOTE: this requires a way to reach the layer — see open question OQ-1 below.
    let result = emit_override_audit(meta, &flags.session.session_id);
    match result {
        Ok(_) => { /* chain advanced; spawn may proceed */ }
        Err(e) => {
            return Err(NonoError::SandboxInit(format!(
                "override audit emission failed — aborting before spawn (AUD-04): {e}"
            )));
        }
    }
}
```

### Example 5: `scripts/gates/override-01.ps1` skeleton

```powershell
# scripts/gates/override-01.ps1
# OVERRIDE-01 gate: offline verify path + fail-closed cases (DF-01).
# CONTRACT: dot-sourced by verify-dark.ps1; exports Test-Precondition + Invoke-Gate.
# NEVER calls exit. NEVER calls Persist-Verdict.

function Test-Precondition {
    if (-not (Get-Command nono -ErrorAction SilentlyContinue)) {
        return 'nono not on PATH'
    }
    $check = & python -c "import nono; print('ok')" 2>&1
    if ($check -ne 'ok') { return "nono Python module not importable: $check" }
    return $null
}

function Invoke-Gate {
    $ErrorActionPreference = 'Continue'

    # SC1: valid token → confined_run receives --override-audit + --allow flags
    # SC2: fail-closed cases all raise NonoOverrideError
    # SC3: no-token path produces identical args to pre-v3.2

    $script = @'
import nono, sys, base64, json
from pathlib import Path

fixtures = Path(r"C:\Users\OMack\nono-py\tests\fixtures")
pubkey_der = (fixtures / "override_test_key.der").read_bytes()
# ... mint a valid token using test private key ...
# ... call verify_override() → OverrideGrant ...
# ... call confined_run(..., override_token=grant) in dry-run mode ...
# ... assert --override-audit present in nono.exe args ...
# ... assert fail-closed cases raise NonoOverrideError ...
print("PASS")
'@

    $result = & python -c $script 2>&1
    if ($result -eq 'PASS') {
        return [ordered]@{
            gate      = 'override-01'
            verdict   = 'PASS'
            reason    = 'SC1/SC2/SC3 verified: override wiring, fail-closed cases, regression path.'
            detail    = [ordered]@{}
            timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
        }
    }
    return [ordered]@{
        gate      = 'override-01'
        verdict   = 'FAIL'
        reason    = "SC check failed: $result"
        detail    = [ordered]@{ output = "$result" }
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No override path in `confined_run` | `override_token: Option<OverrideGrant>` parameter | Phase 92 | Additive; no behavior change for existing callers |
| EventIDs 10001-10005 only | EventIDs 10001-10010 (adds 10006-10010 for override lifecycle) | Phase 92 | SIEM parsers must be updated to recognize new EventIDs |
| `SecurityEventLayer` only intercepts denial events | `SecurityEventLayer` also records override authorization events | Phase 92 | Audit coverage expanded to authorization surface |
| `AuditEventPayload` has 4 variants | +1 variant `PolicyOverrideApplied` | Phase 92 | All exhaustive match sites in core must be updated |

**Deprecated/outdated:**
- `#[expect(dead_code)]` on `OverrideErrorKind::OutOfScope`: must be removed in Phase 92 when the first non-test construction site is added.

---

## Open Questions

1. **How to reach the `SecurityEventLayer` instance from `execute_sandboxed`.**
   - What we know: `SecurityEventLayer` is registered as a `tracing_subscriber::Layer` at `init_tracing()` time. The `on_event` dispatch works because tracing globally routes all events through registered layers. But `emit_override_event` requires a direct method call on the layer instance.
   - What's unclear: Whether there is a registry lookup (e.g., `tracing_subscriber::registry::LookupSpan`) that returns the layer instance, or whether the layer instance must be stored somewhere accessible (e.g., a global `Arc<SecurityEventLayer>` or `OnceLock`).
   - Recommendation: Store the `SecurityEventLayer` in a `OnceLock<Arc<SecurityEventLayer>>` (or similar) set at `init_tracing()` time. This is the same pattern used for the telemetry config lookup. Alternatively, add a dedicated `nono_security::policy_override_applied` tracing target and extend `on_event` (Option A from the architecture section). The planner must choose; both are viable. [ASSUMED — exact mechanism depends on init_tracing() architecture]

2. **`--override-audit` base64 payload size on the Windows CLI arg limit.**
   - What we know: Windows command line limit is ~32767 characters. The metadata struct (`zt_audit_hash` 64 chars, `kms_key_id` ~90 chars ARN, `jti` ~36 chars UUID, paths up to ~10 entries × ~200 chars each) is well under 2000 characters after base64 encoding.
   - What's unclear: Nothing — this is not a real risk at the projected payload sizes.
   - Recommendation: No action needed; document the theoretical limit in comments. [VERIFIED: current metadata fits comfortably; not a constraint]

3. **Where the `OverrideAuditMeta` struct lives** (nono-cli side).
   - What we know: It is deserialized from the `--override-audit` base64-JSON flag value in `sandbox_prepare.rs` or `cli.rs`.
   - Recommendation: Define in `cli.rs` near `SandboxArgs` for co-location with the flag definition. Or in a new `override_audit.rs` module. Planner's choice.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | cargo build | ✓ | 1.82 (MSRV) | — |
| nono-py Python module | OVERRIDE-01 gate | ✓ | from Phase 91 | — |
| Phase 91 test keypair | OVERRIDE-01 token minting | ✓ | `tests/fixtures/override_test_key.pem` | — |
| `scripts/gates/` directory | override-01.ps1 | ✓ | 7 existing gate files | — |
| ZT-Infra local provisioner | NOT needed in Phase 92 | N/A | Phase 93 requirement | — |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework — nono-py | pytest (existing `nono-py/tests/test_*.py`) |
| Framework — nono-cli | Rust built-in test runner (`cargo test`) |
| Config file | `nono-py/tests/conftest.py` (fixtures shared) |
| Quick run (nono-py) | `cd nono-py && maturin develop && pytest tests/ -x -q` |
| Quick run (nono) | `cd Nono && cargo test -p nono -p nono-cli` |
| Full suite | `cd Nono && make test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Notes |
|--------|----------|-----------|-------------------|-------|
| MUT-01 | `confined_run(override_token=grant)` appends grant paths as `--allow` flags | unit (Python) | `pytest tests/test_confined_run.py::test_override_appends_allow_flags -x` | New test file or extend test_confined_run.py |
| MUT-02 | Two `confined_run` calls with different tokens do not share state | unit (Python) | `pytest tests/test_confined_run.py::test_override_invocation_scoped -x` | Verify no shared-state leakage |
| MUT-03 | Override does not remove deny rules; OS confinement still applied | integration (dark-factory gate) | `verify-dark.ps1 --gate OVERRIDE-01` | Structural: nono.exe still applies AppContainer |
| MUT-04 | `/tmp/project` does not cover `/tmp/project-evil` | unit (Python sanitize + Rust capability) | `pytest tests/test_confined_run.py::test_override_path_scope -x` | Path-component check at both boundaries |
| MUT-05 | No-override path produces byte-identical args | unit (Python) | `pytest tests/test_confined_run.py::test_override_regression_no_token -x` | Capture `Command` args as string list and diff |
| AUD-01 | HMAC chain advances after override emit | unit (Rust `#[cfg(test)]`) | `cargo test -p nono-cli telemetry::tests::override_event_advances_chain` | Uses `chain_sequence()` test accessor |
| AUD-02 | `PolicyOverrideApplied` carries correct `zt_audit_hash` | unit (Rust) | `cargo test -p nono-cli telemetry::tests::override_event_zt_audit_hash` | Verify field in emitted event struct |
| AUD-03 | Override events use EventIDs 10006-10010; paths are hashed not raw | unit (Rust) | `cargo test -p nono-cli telemetry::event::tests::override_event_ids` | Verify `event_id_for` mapping |
| AUD-04 | Abort before spawn if audit emission fails | unit (Rust) | `cargo test -p nono-cli execution_runtime::tests::override_gate_abort_on_emit_fail` | Poison mutex → execute_sandboxed returns Err |
| VFY-01 | PARTIAL — offline arm wired; seam documented for Phase 93 | — | — | No test for live arm; seam is a TODO comment |
| DF-01 | `verify-dark.ps1 --gate OVERRIDE-01` emits PASS | integration (dark-factory) | `pwsh -File scripts/verify-dark.ps1 --gate OVERRIDE-01` | Must exit 0; SC1/SC2/SC3 all PASS |

### Sampling Rate

- **Per task commit:** `cargo test -p nono -p nono-cli` + `pytest tests/test_confined_run.py -x -q`
- **Per wave merge:** `make test` (full workspace)
- **Phase gate:** Full suite green + `verify-dark.ps1 --gate OVERRIDE-01` PASS before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `nono-py/tests/test_confined_run.py` — extend or new file covering MUT-01..05 override path (MUT-05 regression test especially critical)
- [ ] `crates/nono-cli/src/telemetry/` — inline `#[cfg(test)]` for `emit_override_event` (AUD-01/AUD-04 using `chain_sequence()`)
- [ ] `scripts/gates/override-01.ps1` — new gate file for DF-01
- [ ] `crates/nono-cli/src/execution_runtime.rs` — inline `#[cfg(test)]` for AUD-04 pre-spawn gate abort behavior

*(No new test framework install needed — all frameworks already present)*

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | yes | Invocation-scoped override (MUT-02); `jti` in-process replay store (Phase 91 D-03) |
| V4 Access Control | yes | AUD-04 fail-closed gate; additive-only MUT-03; bilateral capability gate D-02 |
| V5 Input Validation | yes | `sanitize_override_path` rejects `..` and non-absolute; `--override-audit` base64-JSON deserialization with `deny_unknown_fields` |
| V6 Cryptography | no | Phase 91 owns all crypto; Phase 92 only reads from verified grant |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Override paths present without audit record (silent escalation) | Elevation of Privilege | AUD-04 bilateral gate: abort before spawn if `--override-audit` absent |
| Forged `--override-audit` flag (attacker-controlled metadata) | Tampering | nono-cli treats metadata as log-only (no re-verification); security gate is AUD-04 + the OS confinement layer still enforces the granted `--allow` paths — over-granting metadata is harmless; under-granting paths is harmless |
| Grant path traversal (`..` in scope) | Elevation of Privilege | `sanitize_override_path` rejects `..` components at nono-py boundary; nono-cli path_covered uses component-wise `Path::starts_with` |
| `/tmp/project-evil` path-prefix escape | Elevation of Privilege | `Path::starts_with` (not string) at `capability.rs:1762`; SC3 test coverage |
| Cross-target clippy regression (new match arms) | Integrity | CLAUDE.md MUST rule: cross-target clippy required after any `event.rs`/`mod.rs` change |
| Audit chain poisoned mutex silently passing | Tampering | `emit_override_event` returns `Result`; caller must treat `Err` as fatal (AUD-04) |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `SecurityEventLayer` should use a direct `emit_override_event()` method (Option B) rather than extending the `on_event` tracing-intercept path | Architecture Patterns | If tracing-intercept is the required approach, the planner must extend `SecurityEventVisitor` with override-specific fields and add new target-suffix dispatch. Low risk — both approaches are functionally equivalent. |
| A2 | `OnceLock<Arc<SecurityEventLayer>>` (or equivalent) is the correct mechanism for reaching the layer from `execute_sandboxed` | Open Questions | If `init_tracing()` does not expose the layer instance, a refactor of `init_tracing()` return type is needed. Medium risk — worth checking `init_tracing()` signature at plan time. |
| A3 | base64-JSON encoding of `--override-audit` payload is the correct transport format | Standard Stack / Pattern 1 | If the planner prefers repeated flags (`--override-audit-jti`, `--override-audit-kms-key-id`, etc.), the metadata struct shape changes but the logic is identical. Low risk. |
| A4 | `nono --version` string-match is the simplest acceptable min-version probe for D-02 | Pattern 4 | If nono-py and nono.exe are not always deployed together, version-string matching is fragile. In practice, they are co-deployed per the milestone-marker-only release model. Low risk for v1. |
| A5 | `granted_path_hashes` in `PolicyOverrideApplied` (path-hashed, not raw) satisfies AUD-03 | Code Examples / Pattern 3 | If the audit specification requires raw paths (e.g., for SIEM correlation), this design fails AUD-03 intent. The REQUIREMENTS.md says "paths per the existing redaction policy" which is path-hashed. Low risk. |

---

## Sources

### Primary (HIGH confidence)

- `nono-py/src/override.rs` — direct read, all line numbers verified [VERIFIED: direct code read]
- `nono-py/src/windows_confined_run.rs` — direct read, all line numbers verified [VERIFIED: direct code read]
- `crates/nono/src/audit.rs` — direct read, AuditEventPayload at line 67 confirmed [VERIFIED: direct code read]
- `crates/nono-cli/src/telemetry/event.rs` — direct read, EventIDs 10001-10005 confirmed; 10006-10010 absent [VERIFIED: direct code read]
- `crates/nono-cli/src/telemetry/mod.rs` — direct read, SecurityEventLayer at line 189; advance_chain at 113; on_event at 260 [VERIFIED: direct code read]
- `crates/nono/src/capability.rs` — direct read, path_covered at 1762; CONTEXT.md line cite ~1239-1244 was incorrect (corrected above) [VERIFIED: direct code read]
- `crates/nono-cli/src/execution_runtime.rs` — direct read, execute_sandboxed at line 112; spawn happens via exec_strategy after line 261 [VERIFIED: direct code read]
- `scripts/verify-dark.ps1` — direct read, Test-Precondition/Invoke-Gate contract confirmed [VERIFIED: direct code read]
- `scripts/gates/egress-policy-deny.ps1` — direct read, gate structural template [VERIFIED: direct code read]
- `ZeroTrust2/ZERO_TRUST_V2/provisioner/src/audit.js` — direct read, `current_hash = sha256(stableJson(unsigned))` at line 240 [VERIFIED: direct code read]
- `nono-py/tests/fixtures/` — direct list, confirmed `override_test_key.der`, `override_test_key.pem`, `override_test_private.der`, `vectors.json` exist [VERIFIED: bash ls]

### Secondary (MEDIUM confidence)

- CONTEXT.md architectural decisions D-01..D-06 — used as primary constraint source
- REQUIREMENTS.md MUT/AUD/VFY/DF requirement text — confirmed Phase 92 scope
- Phase 91 CONTEXT.md + PATTERNS.md — confirmed Phase 91 deliverables and crypto primitives

### Tertiary (LOW confidence)

- A1-A5 in Assumptions Log — architectural recommendations based on codebase patterns, not externally documented requirements

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — all symbols verified against live code
- Architecture: HIGH — based on verified code read + locked decisions from CONTEXT.md
- Pitfalls: HIGH — all except A1-A5 are derived from verified code patterns or CLAUDE.md explicit rules
- Open Questions: MEDIUM — OQ-1 (layer access mechanism) requires checking `init_tracing()` at plan time

**Research date:** 2026-06-22
**Valid until:** 2026-07-22 (stable, no upstream sync in Phase 92 scope; codebase is milestone-marker-only)

---

## Project Constraints (from CLAUDE.md)

Directives applicable to Phase 92 planning and implementation:

| Constraint | Applies To | Directive |
|------------|-----------|-----------|
| Policy-free core | `crates/nono/src/audit.rs` addition | `PolicyOverrideApplied` variant is a data carrier only — no policy logic in the core crate |
| No `.unwrap()`/`.expect()` | All new code | Enforced by `clippy::unwrap_used`; `emit_override_event` must use `match` not `?unwrap` |
| `#[must_use]` on critical Results | `emit_override_event` return value | Must be marked `#[must_use]`; `execute_sandboxed` must check the result |
| Cross-target clippy MUST | `event.rs`, `mod.rs`, `audit.rs` | Run `cargo clippy --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin` after any change to `SecurityEventType` match arms |
| Path security — never string `starts_with` | `sanitize_override_path` in nono-py | Use `Path::components()` iteration, not string prefix check |
| Arithmetic: checked/saturating for security-critical | `advance_chain` uses `saturating_add` (already) | New code inherits existing pattern |
| DCO sign-off on all commits | Git commits | `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` |
| GSD workflow enforcement | File changes | Use `/gsd:execute-phase 92` — no direct edits outside GSD workflow |
| No `#[allow(dead_code)]` | `OutOfScope` variant in nono-py | Remove `#[expect(dead_code)]` from `OverrideErrorKind::OutOfScope` when Phase 92 adds the first construction site |
| Separate test env vars (save/restore) | Any test touching env vars | `override_token` tests that modify env vars must save/restore (parallel test runner) |
