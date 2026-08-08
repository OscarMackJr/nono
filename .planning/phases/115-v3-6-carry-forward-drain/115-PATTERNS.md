# Phase 115: v3.6 Carry-Forward Drain - Pattern Map

**Mapped:** 2026-08-08
**Files analyzed:** 13 (modify-only; zero brand-new source files — DRAIN-02's D-12 test is new *content* inside an existing file)
**Analogs found:** 13 / 13 (one item — the D-15 PyO3 struct-variant/nested-Vec wrapper — has no *exact* precedent; the closest partial precedent is documented explicitly below rather than left blank)

All line numbers below were re-grepped fresh this session against the tree at `157b2c6d` /
`nono-py` HEAD. Per CONTEXT.md's own warning, treat every number as a grep starting point, not
an address — replan by symbol if it has drifted further by execution time.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono-cli/src/profile/mod.rs` (`CustomCredentialDef`, D-01) | model (config struct) | transform (deserialize + merge) | same file — `merge_custom_credential_def`'s 12 other `.or(base)` arms | exact (in-file) |
| `crates/nono-cli/src/profile/mod.rs` (`merge_custom_credential_def`, D-01/D-02/D-05) | service (pure merge fn) | transform | same file — its own existing arms | exact |
| `crates/nono-cli/src/profile/mod.rs` (`HookConfig` guard, D-04) | utility (compile-time guard) | transform | same file — `merge_custom_credential_def`'s exhaustive struct literal; `nono-proxy/src/reverse.rs`'s exhaustive `match` (no wildcard arm) | role-match (no exact "guard" idiom exists yet; two structurally-equivalent precedents) |
| `crates/nono-cli/src/profile/mod.rs` (`validate_custom_credential` / `validate_oauth2_auth`, D-13/D-14) | middleware (validation) | request-response (fail at load) | same file — `aws_auth`/`credential_key` mutual-exclusion arm (`validate_custom_credential`) | exact |
| `crates/nono-cli/src/profile/mod.rs` (NEW-02 test extension, D-03) | test | CRUD (build+assert) | same file — `platform_overrides_custom_credential_collision_cannot_drop_capture_or_spiffe` | exact (extend, don't replace) |
| `crates/nono-cli/src/network_policy.rs` (D-01 production consumer) | service (config→proxy-config conversion) | transform | same file — sibling `RouteConfig` construction at the second conversion site (`:276-277`, already `InjectMode::Header` literal + `cred.inject_header.clone()`) | exact |
| `crates/nono-cli/data/nono-profile.schema.json` (D-01 schema shape) | config (JSON Schema) | transform | same file — `credential_format` block (`:669-675`, the `oneOf`-with-null shape with no `"default"` key) | exact |
| `crates/nono-proxy/src/audit.rs` (`EventContext`, `log_denied`, D-06) | service (audit emission) | event-driven | same file — `log_allowed`'s sibling signature/body | exact (sibling fn in same file) |
| `crates/nono-proxy/src/connect.rs` (D-06/D-07 site, `:83-90`) | controller (protocol handler) | request-response | `crates/nono-proxy/src/reverse.rs:360-378` (the categorised `HostDenied` template) | exact (cross-file, same crate) |
| `crates/nono-proxy/src/external.rs` (D-06/D-07 site `:133-140`, D-08 site `:197-204`) | controller | request-response | `reverse.rs:360-378` (HostDenied) for the first site; the file's own dead-context call shape for the second | exact |
| `crates/nono-proxy/src/reverse.rs` (D-06 mechanical sites; D-17 reorder) | controller | request-response | in-file — the 3 already-categorised `HostDenied` sites (`:365`, `:689`, `:982`) | exact |
| `crates/nono-proxy/src/server.rs` (D-06 mechanical sites) | controller | request-response | `reverse.rs`'s categorised sites (cross-file) | role-match |
| `crates/nono/src/undo/types.rs` (`NetworkAuditDenialCategory`, D-09/D-11) | model (audit vocabulary enum) | transform | in-file — the enum's own existing derives; `crates/nono/Cargo.toml`'s `thiserror` for the derive-only-dependency-in-core precedent (D-11) | exact (in-file) + precedent (cross-crate) |
| `../nono-py/src/proxy.rs` (D-10 encoder delete, D-15 `RouteConfig::new` expose fields, new `SpiffeAuthConfig`/`CaptureConfig`/`CaptureResponseField`/`EndpointPolicy*` `#[pyclass]` wrappers) | model + service (PyO3 binding) | transform | `InjectMode` (`proxy.rs:114-172`, C-like enum) for the encode/decode idiom; `CapabilitySource` + `FsCapability.source()`/`CapabilitySet.fs_capabilities()` in `../nono-py/src/lib.rs` (`:120-159`, `:373-379`) for the struct-variant-enum-as-wrapper-struct and Vec\<nested-pyclass\> getter idioms | role-match (composite — no single exact analog, see Pattern Assignments) |
| `../nono-py/src/undo.rs` (D-10 decoder delete, D-16 allowlist test) | model (PyO3 binding) | transform | in-file — the existing hand-written decoder's own match arms (delete, don't extend) | exact |
| `../nono-py/src/{proxy,undo}.rs` (D-12 first `#[cfg(test)] mod tests`) | test | CRUD | `crates/nono-cli/src/profile/mod.rs:4440-4442` (`#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { use super::*; ... }`) — the workspace's standard test-module preamble | role-match (cross-crate; same idiom, new crate) |
| `crates/nono-proxy/tests/spiffe_integration.rs` (D-17 test) | test (integration) | event-driven | in-file — `test_spiffe_jwt_fails_closed_on_missing_socket` (fail-closed, no-SPIRE-needed shape) and the live-SPIRE `SKIP[...]` convention (`:100-105`) | exact |

## Pattern Assignments

### `crates/nono-cli/src/profile/mod.rs` — `CustomCredentialDef` + `merge_custom_credential_def` (D-01, D-02, D-05)

**Analog:** same file, in-place — the struct's own current shape and the merge fn's own other 12 arms.

**Current field shape** (`profile/mod.rs:963-972`, symbol-verified at `:964-972`):
```rust
    /// Injection mode (default: "header")
    #[serde(default)]
    pub inject_mode: InjectMode,

    // --- Header mode fields ---

    /// Only used when inject_mode is "header".
    #[serde(default = "default_inject_header")]
    pub inject_header: String,
```
D-01 changes these to `Option<InjectMode>` / `Option<String>`, drops both `#[serde(default...)]`
attributes (the `Option` absence *is* the default now), matching every other optional field in
this same struct (e.g. `credential_key: Option<String>` immediately above, no serde default
attribute).

**Merge pattern to extend** (`profile/mod.rs:3560-3589`, current text):
```rust
fn merge_custom_credential_def(
    base: CustomCredentialDef,
    child: CustomCredentialDef,
) -> CustomCredentialDef {
    CustomCredentialDef {
        upstream: child.upstream,
        inject_mode: child.inject_mode,
        inject_header: child.inject_header,
        credential_key: child.credential_key.or(base.credential_key),
        auth: child.auth.or(base.auth),
        credential_format: child.credential_format.or(base.credential_format),
        path_pattern: child.path_pattern.or(base.path_pattern),
        path_replacement: child.path_replacement.or(base.path_replacement),
        query_param_name: child.query_param_name.or(base.query_param_name),
        env_var: child.env_var.or(base.env_var),
        tls_ca: child.tls_ca.or(base.tls_ca),
        aws_auth: child.aws_auth.or(base.aws_auth),
        spiffe: child.spiffe.or(base.spiffe),
        capture: child.capture.or(base.capture),
        // `endpoint_rules` is a Vec, so "omitted" is the empty vec. ...
        endpoint_rules: if child.endpoint_rules.is_empty() {
            base.endpoint_rules
        } else {
            child.endpoint_rules
        },
    }
}
```
D-01's edit: `inject_mode: child.inject_mode.or(base.inject_mode)` and
`inject_header: child.inject_header.or(base.inject_header)` — copy the exact `.or(base.X)` shape
already used by `credential_key`/`auth`/etc. two lines up. `upstream` stays bare per D-02 (do not
touch it; the discussion explicitly wants this asymmetry visible next to the new `.or()` arms).

**Doc comment to correct (D-05)** — currently at `profile/mod.rs:3556-3559`:
```rust
/// `upstream` is required (always present on the child) and `inject_mode` /
/// `inject_header` are `serde(default)`-populated presentation fields that
/// remove no security control, so those three take the child's value.
```
Rewrite in place (same doc-comment block, same location) to state the corrected rule: injection
placement is not presentation, `inject_mode`/`inject_header` now merge like every other optional
field, only `upstream` is deliberately child-only (with a one-line reason, mirroring D-02's own
wording).

**`.unwrap_or_default()` / `.unwrap_or_else()` precedent for consumers** — `InjectMode` derives
`Default` with `#[default] Header` (`crates/nono-proxy/src/config.rs:11-23`):
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectMode {
    #[default]
    Header,
    UrlPath,
    QueryParam,
    BasicAuth,
}
```
`default_inject_header()` already exists as a private fn in `profile/mod.rs` (`:1053-1055` area,
returns `"Authorization".to_string()`) — reuse it verbatim for `.unwrap_or_else(default_inject_header)`
at the one production consumer site (see `network_policy.rs` below). Do not invent a second copy.

---

### `crates/nono-cli/src/network_policy.rs` — D-01's one production consumer

**Analog:** the file's own second `RouteConfig` construction site.

**Current first site** (`network_policy.rs:228-229`, the one needing the `.unwrap_or_default()` edit):
```rust
                inject_mode: cred.inject_mode.clone(),
                inject_header: cred.inject_header.clone(),
```
**Sibling site already showing the target shape** (`network_policy.rs:276-277` — a *different*
conversion path that already hardcodes a concrete `InjectMode` and only clones the header):
```rust
                inject_mode: InjectMode::Header,
                inject_header: cred.inject_header.clone(),
```
Target edit at `:228-229`: `cred.inject_mode.clone().unwrap_or_default()` and
`cred.inject_header.clone().unwrap_or_else(default_inject_header)` — the destination type
(`nono_proxy::config::RouteConfig.inject_mode`/`.inject_header`) stays non-`Option`, confirmed
unaffected by D-01 (RESEARCH Q5).

---

### `crates/nono-cli/data/nono-profile.schema.json` — D-01 schema shape

**Analog:** `credential_format` block in the same `$defs.CustomCredentialDef.properties` object.

**Current `inject_mode`/`inject_header` shape** (`:659-668`):
```json
        "inject_mode": {
          "$ref": "#/$defs/InjectMode",
          "description": "Credential injection mode. Determines how the credential is inserted into outbound requests.",
          "default": "header"
        },
        "inject_header": {
          "type": "string",
          "description": "HTTP header name to inject the credential into. Only used when inject_mode is \"header\".",
          "default": "Authorization"
        },
```
**Sibling `Option<T>` shape to copy** (`credential_format`, `:669-675`):
```json
        "credential_format": {
          "oneOf": [
            { "type": "string" },
            { "type": "null" }
          ],
          "description": "Template with {} for the secret (header inject mode). If set, used as-is. If omitted: Authorization header (case-insensitive) defaults to Bearer {}; other headers default to {} (secret only)."
        },
```
Rewrite `inject_mode` as `"oneOf": [{"$ref": "#/$defs/InjectMode"}, {"type": "null"}]` (drop
`"default"`), and `inject_header` as `"oneOf": [{"type": "string"}, {"type": "null"}]` (drop
`"default"`), matching this exact no-`default`-key `oneOf`-null shape. Neither field is in the
`"required": ["upstream"]` array (`:618`) — no requiredness change needed.

---

### `crates/nono-cli/src/profile/mod.rs` — `HookConfig` compile-time guard (D-04)

**No exact "exhaustive destructure as a standalone guard" idiom exists yet in this repo.**
Two structurally-equivalent precedents exist; combine them:

**Precedent 1 — exhaustive struct literal already load-bearing in the very function this guard
protects** (`merge_custom_credential_def`, `profile/mod.rs:3564-3589`, shown in full above): every
field of `CustomCredentialDef` is named individually with no `..` rest pattern, so adding a field
to the struct without updating this literal is already an `E0063` compile error today. The
identical idiom applied to `HookConfig` (`profile/mod.rs:2005-2012`, 3 plain `String` fields) is:
```rust
let HookConfig { event: _, matcher: _, script: _ } = <some HookConfig value>;
```
placed as a dead-store colocated with the D-03 regression test, or as a zero-cost module-level
guard:
```rust
const _: fn(HookConfig) = |HookConfig { event: _, matcher: _, script: _ }| {};
```

**Precedent 2 — exhaustive `match` with no wildcard arm, same codebase, same "force the compiler"
intent** (`crates/nono-proxy/src/reverse.rs:153-155`):
```rust
    // IMPORTANT: match is exhaustive (no wildcard arm) so the compiler forces
    // handling of every current and future EndpointPolicyOutcome variant.
    match route.endpoint_policy.evaluate(&method, &upstream_path) {
        EndpointPolicyOutcome::Allow { .. } => { ... }
        EndpointPolicyOutcome::Deny { reason, rule_label } => { ... }
```
Use this comment style (`// IMPORTANT: ... exhaustive ... so the compiler forces handling of
every current and future ... field/variant.`) verbatim on whichever guard shape is chosen — it is
this codebase's established way of documenting a compile-time-enforced invariant.

**Do not field-merge `hooks.hooks`** — current whole-value-replace merge stays as-is
(`profile/mod.rs:3819-3825`):
```rust
        hooks: HooksConfig {
            hooks: {
                let mut merged = base.hooks.hooks;
                merged.extend(child.hooks.hooks);
                merged
            },
        },
```

---

### `crates/nono-cli/src/profile/mod.rs` — validation rejections (D-13, D-14)

**Analog:** `validate_custom_credential`'s existing mutual-exclusion arm, to extend for D-13; `validate_oauth2_auth`'s early-return shape, to extend for D-14.

**D-13 template — the mutual-exclusion rejection idiom** (`profile/mod.rs:1146-1154`):
```rust
fn validate_custom_credential(name: &str, cred: &CustomCredentialDef) -> Result<()> {
    // Mutual exclusion: aws_auth is incompatible with credential_key and auth.
    if cred.aws_auth.is_some() && (cred.credential_key.is_some() || cred.auth.is_some()) {
        return Err(NonoError::ProfileParse(format!(
            "custom credential '{}' has 'aws_auth' set together with 'credential_key' or 'auth'; \
             aws_auth is mutually exclusive with both — remove the other auth field",
            name
        )));
    }
```
Copy this exact `NonoError::ProfileParse(format!(...))` shape for D-13's unconditional
`aws_auth.is_some()` rejection (naming SigV4 as unimplemented in the message, per CONTEXT.md's
"Claude's Discretion" wording latitude). The 501 guard this supersedes is at
`crates/nono-proxy/src/reverse.rs:323-331`:
```rust
    // AWS SigV4 signing is not yet implemented. Return 501 so the caller
    // knows the route exists but is not functional. This branch will be
    // replaced with real SigV4 signing in a follow-up. (D-15 fork adaptation: ...)
    if aws_route.is_some() {
        send_error(stream, 501, "Not Implemented").await?;
        return Ok(());
    }
```
— reference this comment's wording ("not yet implemented") for message-style consistency.

**D-14 insertion point — `validate_oauth2_auth`** (`profile/mod.rs:1302-1333`, current text):
```rust
fn validate_oauth2_auth(name: &str, auth: &OAuth2Config) -> Result<()> {
    validate_upstream_url(&auth.token_url, &format!("{}/auth.token_url", name))?;

    if let Some(ref assertion) = auth.client_assertion {
        match assertion {
            nono_proxy::config::ClientAssertionConfig::SpiffeJwt { workload_api_socket, audience, .. } => {
                if workload_api_socket.is_empty() { return Err(NonoError::ProfileParse(...)); }
                if audience.is_empty() { return Err(NonoError::ProfileParse(...)); }
            }
        }
        return Ok(());
    }

    if auth.client_id.is_empty() {
        return Err(NonoError::ProfileParse(format!(
            "auth.client_id for custom credential '{}' cannot be empty",
            name
        )));
    }
    ...
```
Insert the D-14 rejection immediately after the `client_assertion.is_some()` early-return
(`:1332`, `return Ok(());`) and before the `client_id.is_empty()` check (`:1335`) — gate on
`auth.client_assertion.is_none()`, same `NonoError::ProfileParse(format!("... for custom
credential '{}' ...", name))` shape as every other error in this function.

---

### `crates/nono-cli/src/profile/mod.rs` — NEW-02 exhaustive regression test (D-03)

**Analog:** the existing test itself — extend, do not replace.

**Structure to extend** (`profile/mod.rs:10169-10258`, full text read):
```rust
    #[test]
    fn platform_overrides_custom_credential_collision_cannot_drop_capture_or_spiffe() {
        use nono_proxy::config::{CaptureConfig, CaptureResponseField, CaptureResponseFieldKind};

        fn cred(upstream: &str, capture: Option<CaptureConfig>) -> CustomCredentialDef {
            CustomCredentialDef {
                upstream: upstream.to_string(),
                credential_key: None,
                auth: None,
                inject_mode: InjectMode::default(),
                inject_header: default_inject_header(),
                credential_format: None,
                path_pattern: None,
                path_replacement: None,
                query_param_name: None,
                env_var: None,
                endpoint_rules: Vec::new(),
                tls_ca: None,
                aws_auth: None,
                spiffe: None,
                capture,
            }
        }

        let base_def = cred("https://token.example.com", Some(CaptureConfig { ... }));
        let override_def = cred("https://token.windows.example.com", None);
        // ... build Profile, override_for_current_platform, finalize_profile ...
        assert!(resolved.capture.is_some(), "NEW-02: ...");
        assert_eq!(resolved.capture.as_ref().map(|c| c.response_fields.len()).unwrap_or(0), 1, "...");
        assert_eq!(resolved.upstream, "https://token.windows.example.com", "...");
    }
```
D-03's rewrite: replace the local `cred(upstream, capture)` helper with one that sets **a distinct
non-default value on every field** (matching the "give the base a value for every field, the child
only redefines `upstream`" instruction), then assert every field except `upstream` equals the
base's value, and `upstream` equals the child's — including the new `.or(base)` arms for
`inject_mode`/`inject_header` from D-01, and the previously-unasserted `spiffe` field ACC-04
named. Keep the module's `#[test]` attribute, the `assert!`/`assert_eq!` with explanatory string
literal shape (the codebase's convention — every assertion in this test carries a "why" message),
and the `finalize_profile(profile).expect("finalize must succeed")` idiom (the test module carries
`#[allow(clippy::unwrap_used)]` at `mod tests {` scope, `profile/mod.rs:4440-4442`, so `.expect()`
here is compliant, not a violation of the crate's normal `.expect()` ban). Also fix the doc comment
immediately above (`:10160-10168`) while there — CONTEXT.md's D-03 flags it as inaccurate.

---

### `crates/nono-proxy/src/audit.rs` — `EventContext` / `log_denied` signature (D-06)

**Analog:** `log_allowed`, the sibling function in the same file, which never reads
`denial_category` — proof the field can move out of the shared struct without touching that path.

**Current struct** (`audit.rs:40-50`):
```rust
#[derive(Debug, Clone, Default)]
pub struct EventContext<'a> {
    pub route_id: Option<&'a str>,
    pub auth_mechanism: Option<NetworkAuditAuthMechanism>,
    pub auth_outcome: Option<NetworkAuditAuthOutcome>,
    pub managed_credential_active: Option<bool>,
    pub injection_mode: Option<NetworkAuditInjectionMode>,
    pub denial_category: Option<NetworkAuditDenialCategory>,
    pub spiffe_context: Option<SpiffeAuditContext>,
    pub capture_context: Option<CaptureAuditContext>,
}
```
D-06's shape: remove `denial_category` from the struct; add it as `log_denied`'s new required
3rd positional parameter (see RESEARCH Q6's exact recommended signature — reproduced here as the
locked target, not a re-derivation):
```rust
pub fn log_denied(
    audit_log: Option<&SharedAuditLog>,
    mode: ProxyMode,
    category: NetworkAuditDenialCategory,   // NEW — required, not in ctx
    ctx: &EventContext<'_>,
    host: &str,
    port: u16,
    reason: &str,
) { /* push_event(..., denial_category: Some(category), ...) */ }
```
`log_allowed` (unaffected — confirm by reading `audit.rs:138-178`, it hardcodes
`denial_category: None,` in its own event-push and never reads `ctx.denial_category`) needs no
signature change.

---

### `crates/nono-proxy/src/{connect,external,reverse}.rs` — denial-site fixes (D-06, D-07, D-08)

**The verbatim template — `reverse.rs:365-376`** (already-correct shape, copy this pattern to the
three broken sites and to every other of the 28 `log_denied` call sites' 3rd-arg insertion):
```rust
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::Reverse,
            &audit::EventContext {
                route_id: Some(&service),
                denial_category: Some(nono::undo::NetworkAuditDenialCategory::HostDenied),
                ..Default::default()
            },
            &service,
            0,
            &reason,
        );
```
(post-D-06, becomes `audit::log_denied(ctx.audit_log, audit::ProxyMode::Reverse,
nono::undo::NetworkAuditDenialCategory::HostDenied, &audit::EventContext { route_id: Some(&service),
..Default::default() }, &service, 0, &reason);` — category moves to the new positional arg,
`denial_category` is deleted from the `EventContext` literal.)

**Broken site 1 — `connect.rs:83-90`** (current, to become `HostDenied` per D-07):
```rust
    let check = filter.check_host(&host, port).await?;
    if !check.result.is_allowed() {
        let reason = check.result.reason();
        audit::log_denied(
            audit_log,
            audit::ProxyMode::Connect,
            &audit::EventContext::default(),
            &host,
            port,
            &reason,
        );
```
**Broken site 2 — `external.rs:133-140`** (current, to become `HostDenied` per D-07): identical
shape, `audit::ProxyMode::External` instead of `Connect`.

**Broken site 3 — `external.rs:197-204`** (current, to become `ExternalProxyRejected` per D-08):
```rust
    if status != 200 {
        audit::log_denied(
            ctx.audit_log,
            audit::ProxyMode::External,
            &audit::EventContext::default(),
            &host,
            port,
            &format!("external proxy rejected with status {}", status),
        );
```

**A 4th site already correctly categorised, useful as a second template for the "build via
literal, not `Default`" idiom when `EventContext` needs other fields too** (`connect.rs:56-73`,
the `ProxyAuthorization`/strict-mode-reject path):
```rust
            audit::log_denied(
                audit_log,
                audit::ProxyMode::Connect,
                &audit::EventContext {
                    auth_mechanism: Some(nono::undo::NetworkAuditAuthMechanism::ProxyAuthorization),
                    auth_outcome: Some(nono::undo::NetworkAuditAuthOutcome::Failed),
                    denial_category: Some(
                        nono::undo::NetworkAuditDenialCategory::AuthenticationFailed,
                    ),
                    ..Default::default()
                },
```

**`connect.rs:238-248` — another already-categorised site (`UpstreamConnectFailed`), the
`..audit::EventContext::default()` (not full-`Default::default()` literal head) variant, for
completeness when auditing all 28 sites for D-06's mechanical edit:**
```rust
    audit::log_denied(
        audit_log,
        audit::ProxyMode::Connect,
        &audit::EventContext {
            denial_category: Some(nono::undo::NetworkAuditDenialCategory::UpstreamConnectFailed),
            ..audit::EventContext::default()
        },
        host,
        port,
        reason,
    );
```
`crates/nono-proxy/src/server.rs`'s 6 call sites were not individually re-read this session
(RESEARCH Q6 confirms the count) — apply the same 3rd-positional-arg mechanical edit; none of the
6 are among the 3 broken (default-context) sites, so no category assignment decision is needed
there, only the signature-call mechanical update.

---

### `crates/nono-proxy/src/reverse.rs` — D-17 SPIFFE mint-ordering reorder

**Current order** (`reverse.rs:531-635`, symbol-verified this session): the `managed_auth.acquire()`
call (`:635`) is reached before the filter host-check that appears at `:684` in the same handler
function (`handle_spiffe_route`-style flow — confirm exact fn name by grep at execution time, not
by this session's line numbers alone). The already-correct sibling ordering (host-check before any
credential work) is the same `reverse.rs:360-378`/`:977-995` `HostDenied` template shown above —
those two sites check-then-deny *before* any credential acquisition happens on their paths. D-17's
fix is purely a reorder: move the `check_host`/`HostDenied` block (currently after `acquire()`) to
run *before* `managed_auth.acquire().await` is called, mirroring those two sibling call sites'
existing check-first structure. No new denial-emission code is needed — the categorised
`HostDenied` block your reorder relocates is functionally identical to the templates already shown.

---

### `crates/nono/src/undo/types.rs` — `NetworkAuditDenialCategory` (D-09, D-11)

**Current enum** (`:242-262` partial, confirmed derives at `:242-244`):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAuditDenialCategory {
    AuthenticationFailed,
    EndpointPolicy,
    ManagedCredentialUnavailable,
    HostDenied,
    InterceptHandshakeFailed,     // <-- D-09: delete this variant and its doc comment
    UpstreamConnectFailed,
    ConnectBypassesL7,
    ExternalProxyRejected,
    SpiffeUnsupportedPath,
    CaptureUnsupportedPath,
    CaptureBufferOrRewriteFailed,
}
```
D-09: remove the `InterceptHandshakeFailed` line only — no other variant is touched, no
`#[serde(other)]` escape hatch is needed (RESEARCH Q1: zero persisted-ledger risk, verified).

**D-11 dependency precedent (`thiserror` in the core crate)** — cite directly, do not
re-investigate: `crates/nono/Cargo.toml:35`, `thiserror.workspace = true`, already a direct
dependency of this exact crate, already the codebase's own justification in CLAUDE.md ("Use
`NonoError` for all errors... propagation via `?`"). If `strum` is chosen: add
`#[derive(strum_macros::EnumIter)]` alongside the existing derives on this same enum. If the
zero-new-dependency fallback is chosen instead, no precedent for `pub const ALL: &[Self]` exists
anywhere in this repo (grepped, zero hits) — the fallback would be genuinely new to this codebase,
not a copy of an existing idiom; write it as a private `const ALL: &[NetworkAuditDenialCategory] =
&[...]` plus an exhaustive `match` guard function analogous to the `HookConfig` guard pattern
above (Precedent 2, `reverse.rs:153-155`'s "exhaustive, no wildcard arm" comment style).

---

### `../nono-py/src/proxy.rs` — D-10 encoder delete + D-15 `RouteConfig::new` expose + new PyO3 wrapper types

**D-10 — delete this hand-written encoder match, replace with the enum's own serde**
(`proxy.rs:76-105`, full current text):
```rust
    dict.set_item(
        "denial_category",
        event.denial_category.as_ref().map(|c| match c {
            nono::undo::NetworkAuditDenialCategory::AuthenticationFailed => "authentication_failed",
            nono::undo::NetworkAuditDenialCategory::EndpointPolicy => "endpoint_policy",
            nono::undo::NetworkAuditDenialCategory::ManagedCredentialUnavailable => {
                "managed_credential_unavailable"
            }
            nono::undo::NetworkAuditDenialCategory::HostDenied => "host_denied",
            nono::undo::NetworkAuditDenialCategory::InterceptHandshakeFailed => {
                "intercept_handshake_failed"
            }
            nono::undo::NetworkAuditDenialCategory::UpstreamConnectFailed => {
                "upstream_connect_failed"
            }
            nono::undo::NetworkAuditDenialCategory::ConnectBypassesL7 => "connect_bypasses_l7",
            nono::undo::NetworkAuditDenialCategory::ExternalProxyRejected => {
                "external_proxy_rejected"
            }
            nono::undo::NetworkAuditDenialCategory::SpiffeUnsupportedPath => {
                "spiffe_unsupported_path"
            }
            nono::undo::NetworkAuditDenialCategory::CaptureUnsupportedPath => {
                "capture_unsupported_path"
            }
            nono::undo::NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed => {
                "capture_buffer_or_rewrite_failed"
            }
        }),
    )?;
```
Delete the whole `match`; replace with a call through the enum's own
`#[serde(rename_all = "snake_case")]` derive, e.g.
`event.denial_category.as_ref().map(|c| serde_json::to_value(c)).transpose()?` (or
`serde_json::to_string(c)` trimmed of quotes) — RESEARCH Q8 empirically confirmed byte-identity
for all 11 strings, including the "awkward" `ConnectBypassesL7` → `connect_bypasses_l7`, so no
manual string massaging is needed.

**D-15 — `RouteConfig::new`, current hardcoded-`None` fields to change**
(`proxy.rs:186-241`, full current text already read in full above). The 5 fields currently always
`None` inside the struct literal:
```rust
                oauth2: None,
                aws_auth: None,
                spiffe: None,
                capture: None,
                ...
                endpoint_policy: None,
```
D-15 keeps `oauth2: None` and `aws_auth: None` **on purpose** (comment why, citing D-13/D-14's
rejection and Q2's unwired trace), and changes `spiffe`, `capture`, `endpoint_policy` to real
constructor parameters, following the existing optional-parameter idiom already used in this same
`#[pyo3(signature = (...))]` block (e.g. `credential_key = None`, `tls_ca = None`) — add
`spiffe: Option<SpiffeAuthConfig> = None`, `capture: Option<CaptureConfig> = None`,
`endpoint_policy: Option<EndpointPolicy> = None` to the signature and body, each converted via
`.map(Into::into)` mirroring `inject_mode.into()` (`:222`) and `external_proxy.map(|e| e.inner)`
(`ProxyConfig::new`, `:410`).

**New PyO3 wrapper types — composite precedent (no single exact analog exists; RESEARCH's Open
Risk #3 confirmed and closed here with the two closest real precedents):**

1. **Simple enum wrapper (only precedent for *any* enum wrapper in this crate)** —
   `InjectMode` (`proxy.rs:113-172`, full text already read above): `#[pyclass(frozen, eq, hash,
   from_py_object)]` C-like enum + `#[pymethods] __repr__/__str__` + `impl From<X> for RustX` +
   `impl From<RustX> for X`. Directly reusable for any *new* C-like enum this phase might add
   (e.g. `CaptureResponseFieldKind::{Opaque, Jwt}`), but `SpiffeAuthConfig` and
   `EndpointPolicyDecision` are not C-like — see next precedent.

2. **Struct-variant Rust enum exposed as a Python wrapper *struct* with `#[staticmethod]`
   constructors per variant — the actual shape-match for `SpiffeAuthConfig`** (single variant,
   `Jwt { workload_api_socket, audience, inject_header, credential_format, svid_hint }`).
   Closest real precedent, `CapabilitySource` in `../nono-py/src/lib.rs:120-159`:
   ```rust
   #[pyclass(frozen, skip_from_py_object)]
   #[derive(Clone)]
   pub struct CapabilitySource {
       inner: RustCapabilitySource,
   }

   #[pymethods]
   impl CapabilitySource {
       #[staticmethod]
       fn user() -> Self { Self { inner: RustCapabilitySource::User } }

       #[staticmethod]
       fn group(name: String) -> Self { Self { inner: RustCapabilitySource::Group(name) } }

       #[staticmethod]
       fn system() -> Self { Self { inner: RustCapabilitySource::System } }
       ...
   }
   ```
   Since `SpiffeAuthConfig` (`nono-proxy/src/config.rs:656-673`) currently has exactly one
   variant (`Jwt { .. }`), the equivalent shape is a `#[pyclass] struct SpiffeAuthConfig { inner:
   RustSpiffeAuthConfig }` with a single `#[new]` constructor building
   `RustSpiffeAuthConfig::Jwt { workload_api_socket, audience, inject_header, credential_format,
   svid_hint }` directly (no need for the multi-`#[staticmethod]` dispatch `CapabilitySource`
   uses, since there is only one arm) — mirror `RouteConfig`'s own `#[new]` + `#[pyo3(signature =
   (...))]` idiom (`proxy.rs:187-216`) for the constructor shape, and `CapabilitySource`'s
   `inner: Rust...` field + getter idiom for read access.

3. **`Vec<nested-pyclass>` getter — the shape-match for `CaptureConfig.response_fields:
   Vec<CaptureResponseField>`.** Closest real precedent,
   `CapabilitySet::fs_capabilities()` in `../nono-py/src/lib.rs:369-379`:
   ```rust
   /// Get a list of all filesystem capabilities.
   fn fs_capabilities(&self) -> Vec<FsCapability> {
       self.inner
           .fs_capabilities()
           .iter()
           .map(|cap| FsCapability { inner: cap.clone() })
           .collect()
   }
   ```
   Mirror this for `CaptureConfig`'s `response_fields()` getter (each `RustCaptureResponseField`
   wrapped as a nested `#[pyclass] CaptureResponseField { inner: ... }`, itself following the
   `FsCapability`/`CapabilitySource`-style plain wrapper-struct-with-getters idiom for its own
   two fields, `path: String` and `kind: CaptureResponseFieldKind` — the latter a small C-like
   enum, use the `InjectMode` idiom (precedent 1) for it). `RouteConfig`'s existing
   `endpoint_rules: Vec<(String, String)>` constructor param (`proxy.rs:214`,
   `.map(|(method, path)| RustEndpointRule { method, path }).collect()`) is the precedent for
   accepting a `Vec` on the *input* (constructor) side, if `CaptureConfig`'s own `#[new]` needs a
   `Vec<CaptureResponseField>` parameter rather than only exposing it as a getter.

4. **`EndpointPolicy` wrapper (struct with nested `Vec<struct>` + nested enum-with-default)** —
   compose precedents 2 and 3: `EndpointPolicyConfig { default: EndpointPolicyDefault, deny:
   Vec<EndpointPolicyRule>, approve: Vec<EndpointPolicyRule>, allow: Vec<EndpointPolicyRule> }`
   (`config.rs:807-816`) wraps via the `CapabilitySource`/`FsCapability` plain-wrapper-with-`inner`
   idiom at every level; `EndpointPolicyDecision` (`Deny | Approve | Allow`, `#[default] Deny`,
   `config.rs:762-770`) uses the `InjectMode` C-like-enum idiom (note the `#[default]` attribute —
   copy `InjectMode`'s own `#[derive(..., Default, ...)] #[default] Header` shape, not a bare
   enum). Confirm `endpoint_policy` is exposed as `Option<EndpointPolicy>` in `RouteConfig::new`
   per D-15/Q4 — it is live-enforced, not a stub.

---

### `../nono-py/src/undo.rs` — D-10 decoder delete + D-16 allowlist test

**D-10 — delete this hand-written decoder match, replace with the enum's own serde**
(`undo.rs:587-623`, full current text already read above, 9 arms + `other => Err(...)`):
```rust
                    denial_category: dict
                        .get_item("denial_category")?
                        .and_then(|v| v.extract::<String>().ok())
                        .map(|s| match s.as_str() {
                            "authentication_failed" => { Ok(...AuthenticationFailed) }
                            "endpoint_policy" => { Ok(...EndpointPolicy) }
                            ... 9 arms total ...
                            other => Err(PyValueError::new_err(format!(
                                "invalid denial_category: {}",
                                other
                            ))),
                        })
                        .transpose()?,
```
Delete the whole `match`; replace with `serde_json::from_value::<NetworkAuditDenialCategory>(...)`
(or `from_str` on a quoted string) mapped to `PyValueError::new_err` on failure — the
`PyValueError::new_err(format!("invalid denial_category: {}", ...))` message shape is worth
preserving for error-message continuity even though the matching mechanism changes.

**D-16 — allowlist test.** No existing "assert no field is unconditionally `None`" test exists in
this crate (first Rust test in the crate, D-12). Build it directly against the exhaustive
`RustRouteConfig` struct literal already present in `RouteConfig::new` (`proxy.rs:217-240`) as the
thing under test: construct a `RouteConfig` through the Python-facing constructor with every
parameter supplied, then assert each `inner` field on the resulting `RustRouteConfig` is
`Some(...)` except a named allowlist (`oauth2`, `aws_auth` — the two D-15 intentionally excludes).
No comment-only precedent should be trusted per D-16's own rejected alternative — this must be a
real runtime assertion test, colocated with the D-12 test module (same `#[cfg(test)] mod tests`
block is fine, or a sibling one in `proxy.rs` since `RouteConfig` lives there).

---

### `../nono-py` — D-12's first Rust `#[cfg(test)] mod tests` block

**Analog (cross-crate, same workspace, same idiom):** `crates/nono-cli/src/profile/mod.rs:4440-4446`:
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_env::EnvVarGuard;
    use std::sync::MutexGuard;
    use tempfile::tempdir;
    ...
```
`nono-py`'s `Cargo.toml` already carries a `[dev-dependencies]` block enabling
`pyo3 = { features = ["extension-module", "auto-initialize"] }` specifically so `cargo test` can
start the Python interpreter without an explicit `Python::initialize()` call in every test
(`Cargo.toml:35-40`, comment already explains this) — this is the crate's own pre-existing
groundwork for exactly this first-test addition; no further interpreter bootstrap plumbing is
needed. Place the `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { use super::*; ... }`
block at the bottom of `proxy.rs` (D-10's round-trip + D-16's allowlist both touch types defined
there) following the same preamble shape as the `nono-cli` analog above, minus the
`nono-cli`-specific `EnvVarGuard`/`tempdir` imports (not needed here).

---

### `crates/nono-proxy/tests/spiffe_integration.rs` — D-17 regression test

**Analog:** the file's own existing fail-closed / live-SPIRE test pair.

**Fail-closed shape (no SPIRE needed), full current text of the smallest example**
(`spiffe_integration.rs:79-96`):
```rust
#[tokio::test]
async fn test_spiffe_jwt_fails_closed_on_missing_socket() {
    let route = make_jwt_route(
        "/tmp/nono-test-nonexistent-spire-socket.sock",
        "http://127.0.0.1:1",
    );
    let result = server::start(ProxyConfig {
        routes: vec![route],
        ..ProxyConfig::default()
    })
    .await;
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.contains("SPIFFE") || err.contains("spiffe") || err.contains("socket"),
        "unexpected error: {}",
        err
    );
}
```
**Live-SPIRE `SKIP[...]` convention, to reuse only if the planner picks a live-agent test**
(`spiffe_integration.rs:100-105`):
```rust
#[tokio::test]
async fn test_spiffe_jwt_live_fetch() {
    let Some(socket) = live_socket() else {
        eprintln!("SKIP[{}]: SPIRE_AGENT_SOCKET not set", module_path!());
        return;
    };
```
**`make_jwt_route` helper to reuse for the D-17 test's route fixture** (`:34-60`, already
constructs a `RouteConfig` with `spiffe: Some(SpiffeAuthConfig::Jwt { ... })`). Per RESEARCH's
Open Risk #1, the structural (host-check-runs-before-acquire, provable without a live SPIRE
agent) option is the cheapest and matches this phase's own "unrepresentable over tested-for"
framing — if chosen, the test can point `workload_api_socket` at a nonexistent path (same idiom
as `test_spiffe_jwt_fails_closed_on_missing_socket` above) against a route whose `upstream` is
deliberately on a denied host, and assert the specific 403 `HostDenied` error text appears
**without** the SPIFFE-socket error text appearing — proving the deny fired before any connection
attempt to the (nonexistent) socket was made.

## Shared Patterns

### `NonoError::ProfileParse` validation-rejection wording
**Source:** `crates/nono-cli/src/profile/mod.rs:1146-1194` (all of `validate_custom_credential`'s
existing arms)
**Apply to:** D-13 (`aws_auth` unconditional rejection) and D-14 (plain OAuth2 `client_credentials`
rejection) — both go in this same file, both must use
`NonoError::ProfileParse(format!("custom credential '{}' has ... ; ...", name))` /
`NonoError::ProfileParse(format!("auth.<field> for custom credential '{}' ...", name))` matching
whichever function they land in, with the credential name always the first `format!` interpolation.

### `.or(base)` optional-field merge
**Source:** `crates/nono-cli/src/profile/mod.rs:3564-3589` (`merge_custom_credential_def`)
**Apply to:** D-01's two new fields — the file already has 12 live examples of this exact shape in
one function; do not invent a different merge idiom for the two new ones.

### `EventContext { field: Some(...), ..Default::default() }` audit-context construction
**Source:** `crates/nono-proxy/src/reverse.rs:365-376`, `:689-696`, `:982-989` (all three identical)
**Apply to:** every one of the 28 `log_denied` call sites touched by D-06/D-07/D-08 across
`connect.rs`, `external.rs`, `reverse.rs`, `server.rs` — same struct-literal shape, same
`..Default::default()` tail (post-D-06: `denial_category` moves out of this literal into the new
positional argument, everything else about the shape is unchanged).

### PyO3 `inner: RustX` wrapper-struct idiom
**Source:** `../nono-py/src/lib.rs` (`CapabilitySource`, `FsCapability`, `CapabilitySet` — every
non-C-like-enum wrapper in the crate uses this) and `../nono-py/src/proxy.rs` (`RouteConfig`,
`ExternalProxyConfig`, `ProxyConfig`)
**Apply to:** every new D-15 wrapper type (`SpiffeAuthConfig`, `CaptureConfig`,
`CaptureResponseField`, `EndpointPolicy` and its nested `EndpointPolicyDefault`/`EndpointPolicyRule`)
— one `#[pyclass] struct X { inner: RustX }`, `#[new]` constructor building `RustX` directly,
`#[getter]` methods reading through `self.inner`, no exceptions.

### `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { use super::*; ... }`
**Source:** `crates/nono-cli/src/profile/mod.rs:4440-4442` (workspace-standard preamble)
**Apply to:** D-12's first Rust test module in `../nono-py`, and D-16's allowlist test if placed
in the same module.

## No Analog Found

None outright — every touched file/symbol has at least a role-match analog. The one item flagged
by RESEARCH as having no *exact* match (D-15's PyO3 wrapper types for a struct-variant enum and a
struct with nested `Vec<struct>`) is resolved above via a composite of two real precedents
(`CapabilitySource` for struct-variant-as-wrapper, `CapabilitySet::fs_capabilities()` for
`Vec<nested-pyclass>` getters) rather than left without guidance — flagged here so the planner
does not mistake "composite precedent" for "found in one file."

## Metadata

**Analog search scope:** `crates/nono-cli/src/{profile/mod.rs,network_policy.rs}`,
`crates/nono-cli/data/nono-profile.schema.json`, `crates/nono-proxy/src/{audit,connect,external,
reverse,server,config,credential}.rs`, `crates/nono-proxy/tests/spiffe_integration.rs`,
`crates/nono/src/undo/types.rs`, `crates/nono/Cargo.toml`, `../nono-py/src/{lib,proxy,undo}.rs`,
`../nono-py/Cargo.toml`.
**Files scanned:** 17 read/grepped directly this session (all symbol-verified against the live
tree, not inherited from CONTEXT.md/RESEARCH.md line numbers without re-check).
**Pattern extraction date:** 2026-08-08
