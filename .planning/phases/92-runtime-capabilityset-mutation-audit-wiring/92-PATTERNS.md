# Phase 92: Runtime CapabilitySet Mutation + Audit Wiring — Pattern Map

**Mapped:** 2026-06-22
**Files analyzed:** 8 new/modified files across 2 repos
**Analogs found:** 8 / 8

---

## File Classification

| New/Modified File | Repo | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|------|-----------|----------------|---------------|
| `crates/nono/src/audit.rs` | Nono | model | CRUD (enum extension) | Same file — existing `AuditEventPayload` variants | exact |
| `crates/nono-cli/src/telemetry/event.rs` | Nono | model + utility | event-driven | Same file — existing `SecurityEventType` + `event_id_for` | exact |
| `crates/nono-cli/src/telemetry/mod.rs` | Nono | service | event-driven HMAC | Same file — `on_event()` / `advance_chain()` pattern | exact |
| `crates/nono-cli/src/cli.rs` | Nono | config | request-response | Same file — existing `SandboxArgs` optional flag fields | exact |
| `crates/nono-cli/src/execution_runtime.rs` | Nono | controller | request-response | Same file — existing pre-spawn guards (`check_blocked_command`, proxy-only guard) | exact |
| `nono-py/src/override.rs` | nono-py | model | request-response | Same file — existing `#[getter]` methods on `OverrideGrant` pyclass | exact |
| `nono-py/src/windows_confined_run.rs` | nono-py | controller | request-response | Same file — `build_nono_run_args()` / `append_caps_allow_flags()` pattern | exact |
| `scripts/gates/override-01.ps1` | Nono | test / gate | request-response | `scripts/gates/telemetry-event-emit.ps1` | role-match |

---

## Pattern Assignments

### `crates/nono/src/audit.rs` — add `PolicyOverrideApplied` variant

**Analog:** Same file, existing `AuditEventPayload` variants (lines 67–111)

**Imports pattern** (lines 14–21):
```rust
use serde::{Deserialize, Serialize};
// (no new imports needed — serde already present)
```

**Core variant pattern** (lines 67–111 — copy serde tag/rename_all from the enum header, then follow the struct-variant style of `CapabilityDecision`/`Network`):
```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditEventPayload {
    // ... existing variants ...

    /// A signed policy override was verified and applied to this invocation.
    ///
    /// Emitted by nono-cli SecurityEventLayer before the sandboxed child spawns
    /// (AUD-01 / AUD-04). Fields use the same redaction contract as other variants:
    /// paths appear as salted hashes (AUD-03), not raw strings.
    PolicyOverrideApplied {
        /// JWT ID of the applied override token (single-use nonce).
        jti: String,
        /// Signing key identity (KMS key ARN) — redaction-safe (not raw DER).
        kms_key_id: String,
        /// ZT-Infra audit chain hash at time of override issuance (bi-directional link, AUD-02).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        zt_audit_hash: Option<String>,
        /// Paths granted by this override — path-hashed per existing redaction policy (AUD-03).
        /// Uses `path_hash_for(session_salt, path)` from `telemetry/event.rs`. Never raw paths.
        granted_path_hashes: Vec<String>,
        /// ISO-8601 expiry timestamp.
        expires_at: String,
    },
}
```

**Pattern notes:**
- `#[serde(default, skip_serializing_if = "Option::is_none")]` is copied exactly from `CapabilityDecision.reject_stage` (line 94–95) and `SessionStarted.redaction_policy` (line 75–76).
- This variant is a **data carrier only** — no policy logic (policy-free core invariant per CLAUDE.md).
- All exhaustive `match` sites on `AuditEventPayload` in the codebase must be updated. Use `Grep("AuditEventPayload", type: "rs")` at plan time to find them.

---

### `crates/nono-cli/src/telemetry/event.rs` — add EventIDs 10006–10010 + `SecurityEventType` variants

**Analog:** Same file, existing constants block (lines 30–41) and `SecurityEventType` enum (lines 52–64)

**EventID constants pattern** (lines 31–41 — copy spacing and doc-comment format exactly):
```rust
// ── EventID constants (locked by ROADMAP SC-1) ────────────────────────────────

/// EventID for a file-system path-deny event.
pub const EVENT_ID_PATH_DENY: u32 = 10001;
/// EventID for a network-egress-deny event.
pub const EVENT_ID_NETWORK_DENY: u32 = 10002;
/// EventID for a mandatory-integrity label violation.
pub const EVENT_ID_LABEL_VIOLATION: u32 = 10003;
/// EventID for a hook fail-closed event.
pub const EVENT_ID_HOOK_FAIL_CLOSED: u32 = 10004;
/// EventID for a telemetry-degraded self-describing event (D-14).
pub const EVENT_ID_TELEMETRY_DEGRADED: u32 = 10005;
// ADD after line 41:
/// EventID for a policy-override token presented (lifecycle start).
pub const EVENT_ID_POLICY_OVERRIDE_PRESENTED: u32 = 10006;
/// EventID for a policy-override token verified and applied.
pub const EVENT_ID_POLICY_OVERRIDE_VERIFIED: u32 = 10007;
/// EventID for a policy-override token rejected (any `OverrideErrorKind`).
pub const EVENT_ID_POLICY_OVERRIDE_REJECTED: u32 = 10008;
/// EventID for a policy-override token rejected because it was expired.
pub const EVENT_ID_POLICY_OVERRIDE_EXPIRED: u32 = 10009;
/// EventID for a policy-override token rejected due to replay (jti already consumed).
pub const EVENT_ID_POLICY_OVERRIDE_REVOKED: u32 = 10010;
```

**`SecurityEventType` variant pattern** (lines 52–64 — copy derive and serde attrs, then add variants):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    PathDeny,
    NetworkDeny,
    LabelViolation,
    HookFailClosed,
    TelemetryDegraded,
    // ADD — Phase 92 override lifecycle events (EventIDs 10006-10010):
    /// Signed override token presented for verification (EventID 10006).
    PolicyOverridePresented,
    /// Signed override token verified and applied (EventID 10007).
    PolicyOverrideVerified,
    /// Signed override token rejected (EventID 10008 — maps from OverrideErrorKind).
    PolicyOverrideRejected,
    /// Signed override token rejected as expired (EventID 10009).
    PolicyOverrideExpired,
    /// Signed override token rejected as already-consumed/revoked (EventID 10010).
    PolicyOverrideRevoked,
}
```

**`event_id_for` match arm pattern** (lines 71–79 — exhaustive match, add all 5 new arms):
```rust
#[must_use]
pub fn event_id_for(t: &SecurityEventType) -> u32 {
    match t {
        SecurityEventType::PathDeny             => EVENT_ID_PATH_DENY,
        SecurityEventType::NetworkDeny          => EVENT_ID_NETWORK_DENY,
        SecurityEventType::LabelViolation       => EVENT_ID_LABEL_VIOLATION,
        SecurityEventType::HookFailClosed       => EVENT_ID_HOOK_FAIL_CLOSED,
        SecurityEventType::TelemetryDegraded    => EVENT_ID_TELEMETRY_DEGRADED,
        // Phase 92 additions:
        SecurityEventType::PolicyOverridePresented => EVENT_ID_POLICY_OVERRIDE_PRESENTED,
        SecurityEventType::PolicyOverrideVerified  => EVENT_ID_POLICY_OVERRIDE_VERIFIED,
        SecurityEventType::PolicyOverrideRejected  => EVENT_ID_POLICY_OVERRIDE_REJECTED,
        SecurityEventType::PolicyOverrideExpired   => EVENT_ID_POLICY_OVERRIDE_EXPIRED,
        SecurityEventType::PolicyOverrideRevoked   => EVENT_ID_POLICY_OVERRIDE_REVOKED,
    }
}
```

**`severity_for` match arm pattern** (lines 154–162 in `mod.rs` — copy existing pipe-grouped pattern):
```rust
fn severity_for(t: &SecurityEventType) -> nono::TelemetrySeverity {
    use nono::TelemetrySeverity;
    match t {
        SecurityEventType::PathDeny
        | SecurityEventType::NetworkDeny
        | SecurityEventType::LabelViolation
        | SecurityEventType::HookFailClosed
        | SecurityEventType::TelemetryDegraded => TelemetrySeverity::Warning,
        // Phase 92: override events are Warning-level (authorization events, not denial-only).
        SecurityEventType::PolicyOverridePresented
        | SecurityEventType::PolicyOverrideVerified
        | SecurityEventType::PolicyOverrideRejected
        | SecurityEventType::PolicyOverrideExpired
        | SecurityEventType::PolicyOverrideRevoked => TelemetrySeverity::Warning,
    }
}
```

**Critical:** `event.rs` is `cfg`-unconditional — adding variants without updating all exhaustive match sites in `mod.rs` causes compile errors on ALL platforms. Cross-target clippy is MANDATORY per CLAUDE.md after any change here.

---

### `crates/nono-cli/src/telemetry/mod.rs` — add `emit_override_event()` method

**Analog:** Same file, `on_event()` method (lines 260–350) and `advance_chain()` (lines 113–140)

**`emit_override_event` method pattern** — add to `impl SecurityEventLayer` block (lines 193–257), after the `new()` constructor:
```rust
impl SecurityEventLayer {
    // ... existing new() and chain_sequence() methods ...

    /// Emit a PolicyOverride lifecycle event directly into the HMAC chain.
    ///
    /// Bypasses the tracing-intercept path (`on_event`) because override events
    /// carry `zt_audit_hash`/`kms_key_id` fields, not `path`/`host`. The direct
    /// method is accessible from `execute_sandboxed` where the layer instance
    /// is in scope.
    ///
    /// Returns `Ok(chain_head_hex)` when the event was emitted and the chain advanced.
    /// Returns `Err(&'static str)` if the mutex is poisoned or the event cannot be
    /// committed.
    ///
    /// # AUD-04 contract
    ///
    /// Callers MUST treat `Err` as FATAL and abort before spawning the sandboxed child.
    /// A poisoned mutex means telemetry is in an unrecoverable state — the override
    /// path must not proceed without a committed audit record.
    #[must_use]
    pub fn emit_override_event(
        &self,
        event_type: &SecurityEventType,
        jti: &str,
        kms_key_id: &str,
        zt_audit_hash: Option<&str>,
    ) -> Result<String, &'static str> {
        let mut inner = self.inner.lock().map_err(|_| "mutex poisoned")?;

        // Telemetry-disabled is NOT an AUD-04 failure — it is a policy choice.
        // Chain still advances (for sequence correctness) but no ETW/AppLog emit.
        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Build canonical event bytes for chain advancement (same format as on_event).
        let pre_chain_bytes = format!(
            "{event_type:?}|{jti}|{kms_key_id}|{zt}|{session_id}|{ts}",
            zt = zt_audit_hash.unwrap_or(""),
            session_id = inner.session_id,
            ts = timestamp_unix_ms,
        );

        advance_chain(&mut inner.chain, pre_chain_bytes.as_bytes());
        let chain_head = chain_head_hex(&inner.chain.head);

        if inner.config.enabled {
            let security_event = SecurityEvent {
                event_type: event_type.clone(),
                agent_pid: std::process::id(),
                path_hash: None,     // override events carry no path — use kms_key_id instead
                path_category: None,
                host: None,
                session_id: inner.session_id.clone(),
                chain_head: chain_head.clone(),
                timestamp_unix_ms,
            };
            windows::emit_security_event(&security_event);
        }

        Ok(chain_head)
    }
}
```

**`on_event` target-suffix pattern** (lines 278–285 — extend if override events should also be reachable via tracing macro path; keep as-is if direct-emit only):
```rust
// Current pattern — add new arms only if Option A (tracing interception) is chosen:
let event_type = match event.metadata().target() {
    t if t.ends_with("path_deny")           => SecurityEventType::PathDeny,
    t if t.ends_with("network_deny")        => SecurityEventType::NetworkDeny,
    t if t.ends_with("label_violation")     => SecurityEventType::LabelViolation,
    t if t.ends_with("hook_fail_closed")    => SecurityEventType::HookFailClosed,
    t if t.ends_with("telemetry_degraded")  => SecurityEventType::TelemetryDegraded,
    _ => return, // Unknown sub-target — skip.
};
```
The research (RESEARCH.md §Architecture Patterns) recommends **Option B (direct-emit method)** — do NOT extend `on_event` for override events unless the planner explicitly chooses Option A.

**Test pattern** for `emit_override_event` (copy from `mod.rs` test block lines 480–510):
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn emit_override_event_advances_chain_and_returns_chain_head() {
        let layer = SecurityEventLayer::new(
            nono::TelemetryConfig::default(),
            "test-override-session".to_string(),
        );
        assert_eq!(layer.chain_sequence(), 0);
        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideVerified,
            "test-jti-123",
            "arn:aws:kms:us-east-1:123456789012:key/test",
            Some("abc123deadbeef"),
        );
        assert!(result.is_ok(), "emit_override_event must succeed on fresh layer");
        // AUD-01: chain must have advanced exactly once.
        assert_eq!(layer.chain_sequence(), 1, "chain must advance by exactly 1 after emit");
    }

    #[test]
    fn emit_override_event_err_on_poisoned_mutex() {
        // AUD-04: poisoned mutex must return Err, never silently succeed.
        use std::sync::Arc;
        let layer = Arc::new(SecurityEventLayer::new(
            nono::TelemetryConfig::default(),
            "test-poison".to_string(),
        ));
        // Poison the mutex by panicking while holding the lock.
        let layer_clone = Arc::clone(&layer);
        let _ = std::panic::catch_unwind(move || {
            let _guard = layer_clone.inner.lock().unwrap();
            panic!("intentionally poison the mutex");
        });
        let result = layer.emit_override_event(
            &SecurityEventType::PolicyOverrideRejected,
            "jti",
            "kms_key_id",
            None,
        );
        assert!(result.is_err(), "poisoned mutex must return Err (AUD-04 fail-closed)");
    }
}
```

---

### `crates/nono-cli/src/cli.rs` — add `--override-audit` to `SandboxArgs`

**Analog:** Same file, `SandboxArgs` struct, existing optional/hidden flags (lines 1960–1997)

**Optional hidden flag pattern** (copy `dangerous_force_wfp_ready` at lines 1964–1968 for the `hide = true` pattern; copy `dry_run` at lines 1995–1997 for simple `Option<String>`):
```rust
// Inside SandboxArgs, add before the closing brace (after `dry_run` at line 1997):

/// Signed override audit metadata (base64url-encoded JSON).
///
/// Present only when nono-py has verified a signed policy override and is
/// passing trusted audit metadata for the SecurityEventLayer HMAC chain.
/// nono-cli treats `--allow` paths supplied in this session as override-granted
/// if and only if this flag is also present (AUD-04 bilateral gate, D-02).
///
/// INTERNAL — not intended for direct user invocation.
/// Value: base64url-no-pad of `{"zt_audit_hash":…,"kms_key_id":…,"jti":…,
///        "granted_paths":[…],"expires_at":…}`.
#[arg(long, value_name = "META", hide = true, help_heading = "OPTIONS")]
pub override_audit: Option<String>,
```

**`OverrideAuditMeta` struct pattern** (add near `SandboxArgs` in `cli.rs`, or in a new `override_audit.rs` module — planner's choice; mimic the `serde` derive pattern on `SandboxArgs` and nearby structs):
```rust
/// Trusted override audit metadata passed via `--override-audit`.
///
/// Deserialized from base64url-no-pad-encoded JSON. All fields are read from
/// the already-verified `OverrideGrant` in nono-py (D-06: never re-parsed
/// from the raw token; closes the TOCTOU verify→apply gap).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideAuditMeta {
    /// ZT-Infra audit chain hash (bi-directional link, AUD-02). `None` if the token
    /// predates ZT-Infra audit chain integration (CAF v0.1 allows `Option<String>`).
    #[serde(default)]
    pub zt_audit_hash: Option<String>,
    /// KMS key ARN used to sign the override token (from `OverrideGrant.signer`).
    pub kms_key_id: String,
    /// JWT ID — single-use nonce (from `OverrideGrant.jti`).
    pub jti: String,
    /// Paths granted by this override, already sanitized in nono-py (D-05).
    pub granted_paths: Vec<String>,
    /// RFC3339 expiry timestamp (from `OverrideGrant.expires_at`).
    pub expires_at: String,
}
```

---

### `crates/nono-cli/src/execution_runtime.rs` — AUD-04 pre-spawn gate

**Analog:** Same file, existing pre-spawn guards (lines 131–240):
- `check_blocked_command` guard (lines 131–139): returns `Err` before any spawn.
- proxy-only guard (lines 234–240): returns `Err` before `start_proxy_runtime`.

**AUD-04 gate pattern** — insert after `start_proxy_runtime` (line 242) and before `apply_pre_fork_sandbox` (line 261):

DECODE-ONCE RULE (enforced by Plan 92-03): The base64-JSON decode happens in `launch_runtime.rs`
during argument preparation (Task 1 of Plan 92-03). By the time `execute_sandboxed` is called,
`flags.override_audit` is already `Option<OverrideAuditMeta>` (the decoded struct), NOT
`Option<String>`. The gate reads the already-decoded struct directly — no base64/JSON decode
inside `execute_sandboxed`. Executors following the old inline-decode pattern produce the wrong
field type and fail Task 1's acceptance criterion.

```rust
// AUD-04: If override audit metadata was passed (--override-audit flag), the
// PolicyOverrideApplied event MUST be committed to the SecurityEventLayer HMAC
// chain BEFORE the sandboxed child spawns. An override that cannot emit its
// audit record is blocked — never silently applied (D-02 bilateral gate).
//
// flags.override_audit is Option<OverrideAuditMeta> — already decoded in
// launch_runtime.rs (Plan 92-03 Task 1). No base64/JSON decode here.
//
// The gate is placed after proxy start (audit metadata may include proxy-related
// context) and BEFORE apply_pre_fork_sandbox to ensure the child never runs
// against override paths without a committed audit record.
if let Some(ref meta) = flags.override_audit {
    // SECURITY_LAYER is a OnceLock<Arc<SecurityEventLayer>> set by init_tracing
    // (Plan 92-03 Task 2 / RESEARCH.md OQ-1 resolution).
    let emit_result = crate::telemetry::SECURITY_LAYER
        .get()
        .ok_or_else(|| NonoError::SandboxInit(
            "override audit emission failed — SecurityEventLayer not initialized (AUD-04)".to_string()
        ))?
        .emit_override_event(
            &crate::telemetry::SecurityEventType::PolicyOverrideVerified,
            &meta.jti,
            &meta.kms_key_id,
            meta.zt_audit_hash.as_deref(),
        );
    match emit_result {
        Ok(_chain_head) => {
            // Chain advanced — spawn may proceed.
        }
        Err(e) => {
            return Err(NonoError::SandboxInit(format!(
                "override audit emission failed — aborting before spawn (AUD-04): {e}"
            )));
        }
    }
}
```

**Existing `return Err(NonoError::SandboxInit(...))` pattern** (line 236–239 — copy exactly for the AUD-04 error):
```rust
return Err(NonoError::SandboxInit(
    "Cannot use proxy-only mode without a network profile or credential configuration."
        .to_string(),
));
```

---

### `nono-py/src/override.rs` — add `zt_audit_hash` getter + remove `#[expect(dead_code)]`

**Analog:** Same file, existing `#[getter]` methods on `OverrideGrant` (lines 692–714)

**Existing getter pattern** (lines 693–714 — copy `scope_paths` getter exactly):
```rust
#[pymethods]
impl OverrideGrant {
    /// Absolute filesystem paths covered by this override.
    #[getter]
    fn scope_paths(&self) -> Vec<String> {
        self.scope_paths.clone()
    }

    /// Network domain names covered by this override (non-path scope entries).
    #[getter]
    fn scope_domains(&self) -> Vec<String> {
        self.scope_domains.clone()
    }
}
```

**New `zt_audit_hash` getter pattern** — add to `OverrideGrant` struct (line 685, add field) AND `#[pymethods]` block:
```rust
// In struct OverrideGrant { ... } — add field after repo_context:
/// ZT-Infra audit chain hash from `token.current_hash` (D-06 / AUD-02).
/// `None` when the override token predates ZT-Infra audit chain integration.
pub zt_audit_hash: Option<String>,

// In #[pymethods] impl OverrideGrant — add getter:
/// ZT-Infra audit chain hash at time of override issuance (AUD-02 bi-directional link).
/// `None` when not present in the token (`current_hash` field absent or null in CAF v0.1).
#[getter]
fn zt_audit_hash(&self) -> Option<String> {
    self.zt_audit_hash.clone()
}
```

**`OverrideGrant` construction site** (line 791–799 — add `zt_audit_hash` to the `Ok(OverrideGrant { ... })` call):
```rust
Ok(OverrideGrant {
    signer: token.kms_signature.key_id.clone(),
    scope_paths,
    scope_domains,
    not_before: token.not_before.clone(),
    expires_at: token.expires_at.clone(),
    jti: token.jti.clone(),
    repo_context: token.repo_context.clone(),
    zt_audit_hash: token.current_hash.clone(),  // ADD: Phase 92 D-06
})
```

**`#[expect(dead_code)]` removal** (line 73 — remove this attribute when Phase 92 adds the first `OutOfScope` construction site):
```rust
// BEFORE (Phase 91, line 73):
#[expect(dead_code, reason = "forward-declared for Phase 92 scope enforcement; remove when Phase 92 constructs it")]
OutOfScope,

// AFTER (Phase 92):
OutOfScope,
```

---

### `nono-py/src/windows_confined_run.rs` — override hook in `confined_run` / `confine`

**Analog:** Same file, `confined_run` (lines 174–215) and `confine` (lines 253–325)

**`confined_run` signature extension pattern** (lines 174–183 — add `override_token` parameter after `timeout_secs`):
```rust
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
    override_token: Option<PyRef<'_, OverrideGrant>>,  // Phase 92: D-01
) -> PyResult<ExecResult> {
```

**`confine` signature extension pattern** (lines 253–258 — add `override_token` parameter):
```rust
#[pyfunction]
#[pyo3(signature = (profile=None, allow=None, caps=None, override_token=None))]
pub fn confine(
    profile: Option<String>,
    allow: Option<Vec<String>>,
    caps: Option<&CapabilitySet>,
    override_token: Option<PyRef<'_, OverrideGrant>>,  // Phase 92: D-01
) -> PyResult<()> {
```

**Override hook insertion pattern** — in `confined_run`, after `build_nono_run_args` call (line 197) and before `cmd.arg("--").arg(&exe)` (line 203):
```rust
// Phase 92: append override allow flags + audit metadata (D-01 / MUT-01 / AUD-04)
if let Some(ref grant) = override_token {
    append_override_args(&mut cmd, grant)?;
}
```

In `confine`, after `append_caps_allow_flags` (line 297) and before `cmd.arg("--")` (line 303):
```rust
// Phase 92: append override allow flags + audit metadata (D-01 / MUT-01 / AUD-04)
if let Some(ref grant) = override_token {
    append_override_args(&mut cmd, grant)?;
}
```

**New `append_override_args` helper pattern** — follows `append_caps_allow_flags` (lines 133–139); add as a new `fn` in the same helper section:
```rust
/// Sanitize and append `--allow` flags for override-granted paths, then append
/// the `--override-audit <base64-meta>` flag carrying trusted audit metadata.
///
/// # Security
/// - Rejects path components containing `..` (MUT-04 / D-05).
/// - Requires absolute paths (D-05).
/// - Reads all audit fields from the already-verified `OverrideGrant` (D-06:
///   never re-parses the raw token; closes the TOCTOU verify→apply gap).
fn append_override_args(cmd: &mut Command, grant: &OverrideGrant) -> PyResult<()> {
    // D-05: sanitize grant paths before they become --allow flags.
    for raw_path in &grant.scope_paths {
        let path = sanitize_override_path(raw_path)?;
        cmd.arg("--allow").arg(path.as_os_str());
    }

    // D-06: read audit fields from the verified grant — never re-parse the token.
    let meta = serde_json::json!({
        "zt_audit_hash": grant.zt_audit_hash(),
        "kms_key_id": &grant.signer,
        "jti": &grant.jti,
        "granted_paths": &grant.scope_paths,
        "expires_at": &grant.expires_at,
    });
    use base64::Engine as _;
    let meta_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(meta.to_string().as_bytes());
    cmd.arg("--override-audit").arg(meta_b64);

    Ok(())
}

/// Validate an override scope path: must be absolute and must not contain `..`.
///
/// Uses `Path::components()` iteration — never string `starts_with` (CLAUDE.md
/// §Path Handling footgun; MUT-04).
fn sanitize_override_path(raw: &str) -> PyResult<std::path::PathBuf> {
    let p = std::path::Path::new(raw);
    if !p.is_absolute() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "override scope path must be absolute: {raw}"
        )));
    }
    for component in p.components() {
        if component == std::path::Component::ParentDir {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "override scope path must not contain '..': {raw}"
            )));
        }
    }
    Ok(p.to_path_buf())
}
```

---

### `scripts/gates/override-01.ps1` — new OVERRIDE-01 gate

**Analog:** `scripts/gates/telemetry-event-emit.ps1` (telemetry-event-emit is the closest Python-free analog; `egress-policy-deny.ps1` is the structural twin for the gate contract)

**Gate header + contract comment pattern** (copy from `telemetry-event-emit.ps1` lines 1–54 verbatim, adapting gate name, SC assertions, and skip-condition rationale):
```powershell
# scripts/gates/override-01.ps1
#
# Phase 92 Plan XX - override-01 gate (DF-01: offline verify path + fail-closed cases)
#
# CONTRACT (cloned from scripts/gates/telemetry-event-emit.ps1— structural twin):
# this gate exports exactly two functions, dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object — it MUST NOT call exit and MUST NOT call
# Persist-Verdict. Only the runner owns exit-code mapping (PASS=0 / FAIL=2 /
# SKIP_HOST_UNAVAILABLE=3 / harness-internal=4) and the persist-before-emit (WR-04).
#
#   Test-Precondition -> $null (preconditions met, run Invoke-Gate)
#                      | "reason string" (SKIP_HOST_UNAVAILABLE - exit 3, Invoke-Gate never runs)
#   Invoke-Gate       -> [ordered]@{ gate; verdict; reason; detail; timestamp }
#                        verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
#                        a `throw` here = harness-internal error (exit 4), never a silent PASS
#
# WHAT THIS PROVES (satisfies DF-01 / MUT-01 / MUT-05):
#   SC1 (valid token → --override-audit + --allow flags appended):
#     Mints a valid token using the Phase 91 committed test keypair
#     (nono-py/tests/fixtures/override_test_key.pem + override_test_private.der).
#     Calls confined_run(..., override_token=grant) in a dry-run / arg-capture mode.
#     Asserts --override-audit appears in nono.exe args and scope_paths appear as --allow.
#   SC2 (fail-closed cases raise NonoOverrideError):
#     Calls verify_override with: bad sig, expired token, out-of-scope, replay, alg:none.
#     Asserts each raises NonoOverrideError (not a built-in exception, not None).
#   SC3 (no-token path produces byte-identical args):
#     Calls confined_run(..., override_token=None) and asserts --override-audit is absent.
#     Asserts the arg list is identical to the pre-Phase-92 baseline (MUT-05 regression).
#
# WHY SKIP (not FAIL) when prerequisites are absent:
#   Requires Python + nono module importable (nono-py maturin-built).
#   Requires nono.exe on PATH (build nono before running this gate).
#   Admin NOT required — offline verify and arg-capture use no elevated APIs.
#   On a dev host without Python/nono-py, the gate SKIPs cleanly.
#
# INVOCATION RULE (MEMORY durable):
#   pwsh -File scripts\verify-dark.ps1 --gate override-01
#   NEVER: pwsh -Command "<bare path>" (swallows exit N -> 1)
```

**`Assert-True` helper pattern** (copy from `telemetry-event-emit.ps1` lines 91–101 — identical in every gate):
```powershell
function Assert-True {
    param(
        [Parameter(Mandatory = $true)][bool]$Condition,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if (-not $Condition) { throw $Message }
}
```

**`Test-Precondition` pattern** (copy from `telemetry-event-emit.ps1` lines 137–169 — check nono on PATH + Python importable, return `$null` on success or reason string on SKIP):
```powershell
function Test-Precondition {
    # 1. nono on PATH (harness-internal if absent — checked in Invoke-Gate via Assert-True;
    #    but SKIP if absent because override-01 gate itself cannot run without nono.exe).
    if (-not (Get-Command nono -ErrorAction SilentlyContinue)) {
        return 'nono is not on PATH — build and install nono before running this gate'
    }
    # 2. Python + nono module importable (maturin develop required).
    $check = & python -c "import nono; print('ok')" 2>&1
    if ($LASTEXITCODE -ne 0 -or $check -ne 'ok') {
        return "nono Python module not importable ($check) — run 'maturin develop' in nono-py then re-run"
    }
    return $null
}
```

**`Invoke-Gate` verdict object pattern** (copy from `telemetry-event-emit.ps1` lines 171–399 — NEVER calls `exit`, NEVER calls `Persist-Verdict`, returns exactly one `[ordered]@{gate; verdict; reason; detail; timestamp}`):
```powershell
function Invoke-Gate {
    $ErrorActionPreference = 'Continue'

    $fixturesPath = Join-Path $PSScriptRoot '..\..\..\..\nono-py\tests\fixtures'
    # Resolve relative to script location — verify-dark.ps1 runs from Nono root.
    # Actual path: C:\Users\OMack\nono-py\tests\fixtures
    $fixturesPath = 'C:\Users\OMack\nono-py\tests\fixtures'

    $stamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')

    # SC1 + SC2 + SC3: run inline Python that calls verify_override + confined_run
    # in arg-capture mode and reports results.
    $script = @"
import nono, sys, json, base64
from pathlib import Path

fixtures = Path(r'$fixturesPath')
pubkey_der  = (fixtures / 'override_test_key.der').read_bytes()
privkey_der = (fixtures / 'override_test_private.der').read_bytes()

# ... SC1: mint valid token, verify, check confined_run args ...
# ... SC2: bad-sig / expired / replay / alg-none all raise NonoOverrideError ...
# ... SC3: no-token path produces same args as baseline ...
print(json.dumps({'sc1': True, 'sc2': True, 'sc3': True}))
"@

    $raw = & python -c $script 2>&1
    # ... parse $raw, check sc1/sc2/sc3 bools, build verdict object ...

    return [ordered]@{
        gate      = 'override-01'
        verdict   = 'PASS'   # or 'FAIL'
        reason    = 'SC1/SC2/SC3 verified: override wiring, fail-closed cases, regression path.'
        detail    = [ordered]@{ sc1 = $true; sc2 = $true; sc3 = $true }
        timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
    }
}
```

---

## Shared Patterns

### Pattern: Fail-Closed `return Err(NonoError::SandboxInit(...))`

**Source:** `crates/nono-cli/src/execution_runtime.rs` lines 234–240
**Apply to:** AUD-04 gate in `execute_sandboxed`, any new fail-closed abort before spawn
```rust
return Err(NonoError::SandboxInit(
    "override audit emission failed — aborting before spawn (AUD-04): <detail>".to_string(),
));
```

### Pattern: `#[must_use]` on security-critical `Result`s

**Source:** `crates/nono-cli/src/telemetry/event.rs` line 70 (`event_id_for`); `crates/nono-cli/src/telemetry/mod.rs` line 231 (`new`)
**Apply to:** `emit_override_event()` return value; `sanitize_override_path()` return value
```rust
#[must_use]
pub fn emit_override_event(...) -> Result<String, &'static str> { ... }
```

### Pattern: `#[serde(default, skip_serializing_if = "Option::is_none")]` on optional fields

**Source:** `crates/nono/src/audit.rs` lines 94–95 (`reject_stage`) and 75–76 (`redaction_policy`)
**Apply to:** `zt_audit_hash` in `PolicyOverrideApplied`; `zt_audit_hash` in `OverrideAuditMeta`
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub zt_audit_hash: Option<String>,
```

### Pattern: `advance_chain` + `chain_head_hex` for HMAC advancement

**Source:** `crates/nono-cli/src/telemetry/mod.rs` lines 113–146
**Apply to:** `emit_override_event()` body — call `advance_chain(&mut inner.chain, bytes)` then `chain_head_hex(&inner.chain.head)`
```rust
advance_chain(&mut inner.chain, pre_chain_bytes.as_bytes());
let chain_head = chain_head_hex(&inner.chain.head);
```

### Pattern: `path_hash_for(salt, path)` for path redaction

**Source:** `crates/nono-cli/src/telemetry/event.rs` lines 188–199
**Apply to:** `granted_path_hashes` field in `PolicyOverrideApplied` — hash each path with the session salt, never emit raw path strings
```rust
let path_hash = path_hash_for(&inner.session_salt, std::path::Path::new(raw_path));
```

### Pattern: `Path::components()` iteration for path validation (never string `starts_with`)

**Source:** `crates/nono-cli/src/telemetry/event.rs` lines 125–165 (`classify_path`); `crates/nono/src/capability.rs` line 1762 (`path_covered`)
**Apply to:** `sanitize_override_path()` in `nono-py/src/windows_confined_run.rs`
```rust
for component in p.components() {
    if component == std::path::Component::ParentDir { ... }
}
```

### Pattern: Gate verdict object shape

**Source:** `scripts/gates/egress-policy-deny.ps1` lines 244–299; `scripts/gates/telemetry-event-emit.ps1` lines 212–224
**Apply to:** `scripts/gates/override-01.ps1` — every return from `Invoke-Gate`
```powershell
[ordered]@{
    gate      = 'override-01'
    verdict   = 'PASS'  # or 'FAIL' or 'SKIP_HOST_UNAVAILABLE'
    reason    = 'human-readable description'
    detail    = [ordered]@{ assertion = 'SC1+SC2+SC3'; ... }
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}
```

---

## No Analog Found

All 8 files have close analogs. No files require fallback to RESEARCH.md patterns only.

---

## Metadata

**Analog search scope:** `crates/nono/src/`, `crates/nono-cli/src/telemetry/`, `crates/nono-cli/src/` (cli.rs, execution_runtime.rs), `nono-py/src/`, `scripts/gates/`
**Files scanned:** 8 primary analog files + 2 gate structural twins
**Pattern extraction date:** 2026-06-22

### Critical Cross-Cutting Rules (from CLAUDE.md)

1. **Cross-target clippy MANDATORY** after any change to `event.rs` (new `SecurityEventType` variants) or `mod.rs` (new match arms in `severity_for`). Must pass both `--target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`.
2. **No `.unwrap()`/`.expect()`** in `emit_override_event`. Use `map_err` + `?` or explicit `match`.
3. **`#[must_use]`** on `emit_override_event` return — callers that ignore `Err` violate AUD-04.
4. **Never string `starts_with` for paths** in `sanitize_override_path` — use `Path::components()`.
5. **D-14 degrade-not-abort** for telemetry-disabled case: chain still advances, no emit, `Ok` returned.
6. **DCO sign-off** on all commits: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
