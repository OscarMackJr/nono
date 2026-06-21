# Phase 88: Feature + Dependency Cherry-Pick Wave - Pattern Map

**Mapped:** 2026-06-20
**Files analyzed:** 22 new/modified files across 5 crates
**Analogs found:** 22 / 22 (all files have strong analogs; one file is net-new with a direct analog)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/state_paths.rs` [NEW] | utility | request-response | `crates/nono-cli/src/config/mod.rs` (state-path helpers, NONO_TEST_HOME pattern) | role-match |
| `crates/nono-cli/src/config/mod.rs` | utility | request-response | same file (delegation rewrite of existing helpers) | exact |
| `crates/nono-cli/src/audit_session.rs` | service | CRUD | `crates/nono-cli/src/rollback_session.rs` (state-root delegation pattern) | exact |
| `crates/nono-cli/src/rollback_session.rs` | service | CRUD | `crates/nono-cli/src/audit_session.rs` (symmetric pattern) | exact |
| `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | utility | transform | same file (existing validate_env_var_patterns); add validate_set_vars | exact |
| `crates/nono-cli/src/exec_strategy.rs` | service | request-response | same file (ExecConfig struct additions; PTY nix:: additions) | exact |
| `crates/nono-cli/src/hook_runtime.rs` [UNIX ONLY] | service | event-driven | same file (build_hook_command env injection) | exact |
| `crates/nono-cli/src/hook_runtime_windows.rs` | service | event-driven | `crates/nono-cli/src/hook_runtime.rs` (D-14 carve-out: Windows retains env_clear) | exact |
| `crates/nono-proxy/src/config.rs` | model | CRUD | same file (RouteConfig additive field; AwsAuthConfig new struct) | exact |
| `crates/nono-proxy/src/credential.rs` | service | CRUD | same file (CredentialStore additive fields; aws_routes HashMap) | exact |
| `crates/nono-proxy/src/route.rs` | service | request-response | same file (requires_managed_credential, auth_mechanism_for_route additions) | exact |
| `crates/nono-proxy/src/server.rs` | service | request-response | same file (test-only aws_auth: None additions + fork 501 stub) | exact |
| `crates/nono-cli/src/network_policy.rs` | utility | transform | `crates/nono-cli/src/capability_ext.rs` (policy-resolution pattern) | role-match |
| `crates/nono-cli/src/profile/mod.rs` | service | CRUD | same file (validate_custom_credential; profile-load-time validation) | exact |
| `crates/nono/src/keystore.rs` | utility | request-response | same file (env-var timeout pattern) | exact |
| `crates/nono-cli/src/update_check.rs` | utility | request-response | same file (env-var lookup additions) | exact |
| `crates/nono-cli/src/cli.rs` | config | request-response | same file (BoolishValueParser; truthy env flags) | exact |
| `crates/nono-cli/data/policy.json` | config | CRUD | same file (profile namespace + alias additions) | exact |
| `crates/nono-cli/src/profile/builtin.rs` | config | CRUD | same file (test coverage for alias names) | exact |
| `bindings/c/src/diagnostic.rs` | utility | request-response | same file + `bindings/c/src/lib.rs` nono_clear_error pattern | exact |
| `bindings/c/src/lib.rs` | utility | request-response | same file (clear_last_call_state helper; thread-local set/get) | exact |
| `crates/nono/Cargo.toml` + `Cargo.lock` | config | — | same files (typify spec edit pattern) | exact |

---

## Pattern Assignments

### `crates/nono-cli/src/state_paths.rs` [NEW] (utility, request-response)

**Analog:** `crates/nono-cli/src/config/mod.rs` (primary) + `crates/nono-cli/src/provision_windows.rs` (Windows arm)

This is the upstream module being cherry-picked. The fork must add a Windows-specific arm that mirrors the existing `scratch_dir()` pattern in `provision_windows.rs`.

**Imports pattern** (`config/mod.rs` lines 1-25):
```rust
use nono::{NonoError, Result};
use std::path::{Path, PathBuf};
```

**NONO_TEST_HOME override pattern** (`config/mod.rs` lines 130-153):
```rust
pub fn nono_home_dir() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("NONO_TEST_HOME") {
        let path = PathBuf::from(&value);
        if !path.is_absolute() {
            return Err(NonoError::EnvVarValidation {
                var: "NONO_TEST_HOME".to_string(),
                reason: format!("must be an absolute path, got: {}", value),
            });
        }
        warn_once_test_home(&path);
        return Ok(path);
    }
    dirs::home_dir().ok_or(NonoError::HomeNotFound)
}

fn warn_once_test_home(path: &Path) {
    static WARNED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if WARNED.get().is_none() {
        tracing::warn!("NONO_TEST_HOME override active: {}", path.display());
        let _ = WARNED.set(());
    }
}
```

**Windows LOCALAPPDATA arm pattern** (`provision_windows.rs` lines 273-284 — copy this structure for D-02):
```rust
fn scratch_dir() -> Result<PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
        NonoError::Setup(
            "provision_windows: %LOCALAPPDATA% is not set (cannot resolve scratch dir)".to_string(),
        )
    })?;
    if local_app_data.trim().is_empty() {
        return Err(NonoError::Setup(
            "provision_windows: %LOCALAPPDATA% is empty".to_string(),
        ));
    }
    Ok(PathBuf::from(local_app_data).join("nono").join("workspace"))
}
```

**D-02 Windows arm for `state_paths::user_state_dir()` — copy this structure exactly:**
```rust
pub fn user_state_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
            NonoError::Setup(
                "state_paths: %LOCALAPPDATA% is not set".to_string(),
            )
        })?;
        if local_app_data.trim().is_empty() {
            return Err(NonoError::Setup(
                "state_paths: %LOCALAPPDATA% is empty".to_string(),
            ));
        }
        return Ok(PathBuf::from(local_app_data).join("nono"));
    }
    #[cfg(not(target_os = "windows"))]
    Ok(resolve_xdg_state_base()?.join("nono"))
}
```

**Test env-var guard pattern** (`config/mod.rs` tests, lines 398-410):
```rust
#[test]
fn nono_home_dir_returns_override_when_set() {
    let _guard = test_env_lock().lock().expect("env lock");
    #[cfg(target_os = "windows")]
    let abs = r"C:\nono-test-home-override-1";
    #[cfg(not(target_os = "windows"))]
    let abs = "/tmp/nono-test-home-override-1";
    let _env = EnvVarGuard::set_all(&[("NONO_TEST_HOME", abs)]);

    let home = nono_home_dir().expect("override should be honored");
    assert_eq!(home, PathBuf::from(abs));
}
```

---

### `crates/nono-cli/src/config/mod.rs` (utility, request-response)

**Role:** D-01 delegation rewrite — replace the inline `dirs::state_dir()` chain with a call to `state_paths::user_state_dir()`.

**Current `user_state_dir()` implementation to rewrite** (lines 174-191):
```rust
// BEFORE (current fork implementation):
pub fn user_state_dir() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("NONO_TEST_HOME") {
        let path = PathBuf::from(&value);
        if path.is_absolute() {
            return Some(path.join(".nono"));
        }
    }
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|p| p.join("nono"))
}

// AFTER (D-01 delegation):
pub fn user_state_dir() -> Option<PathBuf> {
    crate::state_paths::user_state_dir().ok()
}
```

**`legacy_windows_state_dir()` analog** (`config/mod.rs` lines 203-207) — stays; it is the legacy-path function for protected-path checks, NOT the current-state function:
```rust
#[cfg(target_os = "windows")]
pub fn legacy_windows_state_dir() -> Result<PathBuf> {
    let home = validated_home()?;
    Ok(Path::new(&home).join(".nono"))
}
```

**Error type for validation** (`config/mod.rs` lines 44-51):
```rust
return Err(NonoError::EnvVarValidation {
    var: source_var.to_string(),
    reason: format!("must be an absolute path, got: {}", home),
});
```

---

### `crates/nono-cli/src/audit_session.rs` and `crates/nono-cli/src/rollback_session.rs` (service, CRUD)

**Role:** State-root callsite migration — FEAT-02 scope. Each must stop calling `nono_home_dir()?.join(".nono").join(...)` directly and instead delegate to `state_paths::audit_root()` / `state_paths::rollback_root()`.

**State root delegation pattern to adopt** (from RESEARCH.md table — current pattern to replace):
```rust
// BEFORE in audit_session.rs line 36:
let home = crate::config::nono_home_dir()?;
Ok(home.join(".nono").join("audit"))

// AFTER (delegate to state_paths):
state_paths::audit_root()
```

**Fail-secure error propagation** (use `?`, never `.unwrap_or_default()`):
```rust
// Pattern from credential.rs lines 175-176 (skip-with-warn vs abort):
// For migration errors: abort, don't swallow. See D-03.
maybe_migrate_legacy_audit_ledger()?;  // propagate as fatal
```

---

### `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (utility, transform)

**Role:** FEAT-01 — add `validate_set_vars()` and `is_valid_env_var_name()` to the existing module. The file already contains `validate_env_var_patterns()`, `is_env_var_allowed()`, and `is_env_var_denied()` — the new functions follow the same `Option<String>` error-return convention.

**Existing module pattern** (lines 151-167, `validate_env_var_patterns`):
```rust
pub(crate) fn validate_env_var_patterns(patterns: &[String], field_name: &str) -> Option<String> {
    for pattern in patterns {
        if pattern.contains('*') && !pattern.ends_with('*') {
            return Some(format!(
                "Invalid {} pattern '{}': '*' is only valid as a trailing suffix",
                field_name, pattern
            ));
        }
        if pattern.starts_with('*') && pattern.len() > 1 {
            return Some(format!(
                "Invalid {} pattern '{}': use a bare '*' to match all variables, or a specific prefix like 'AWS_*'",
                field_name, pattern
            ));
        }
    }
    None
}
```

**New `validate_set_vars()` — copy same `Option<String>` return convention** (from upstream `d48aeb7b`):
```rust
pub(crate) fn validate_set_vars(
    set_vars: &std::collections::HashMap<String, String>,
) -> Option<String> {
    for key in set_vars.keys() {
        if key == "PATH" {
            return Some("Invalid set_vars key 'PATH': use allow_vars/deny_vars to control PATH inheritance".to_string());
        }
        if key.starts_with("NONO_") {
            return Some(format!("Invalid set_vars key '{}': NONO_* prefix is reserved", key));
        }
        if !is_valid_env_var_name(key) {
            return Some(format!("Invalid set_vars key '{}': must match [A-Za-z_][A-Za-z0-9_]*", key));
        }
    }
    None
}
```

**Note on `ExecConfig::set_vars` field:** The upstream `d48aeb7b` adds `set_vars: Vec<(String, String)>` to `ExecConfig` (`exec_strategy.rs` lines 288-340). All `ExecConfig { ... }` construction sites must add `set_vars: Vec::new()` (or the profile-resolved value). The existing `env_vars: Vec<(&'a str, &'a str)>` field (line 299) is the template — the new field uses owned `String` instead of references.

---

### `crates/nono-cli/src/hook_runtime.rs` (service, event-driven) [UNIX ONLY]

**Role:** FEAT-05 (add `PACK_DIR` env injection) + Cluster M / `e54cf9cb` (remove `env_clear()`). Apply I-before-M.

**Current `build_hook_command()` pattern** (lines 189-223):
```rust
fn build_hook_command(
    script: &Path,
    session_id: &str,
    workdir: &Path,
    kind: &HookKind<'_>,
) -> Command {
    let mut cmd = Command::new(script);
    cmd.env_clear();              // <-- e54cf9cb removes this line (Unix only)
    cmd.env("NONO_SESSION_ID", session_id);
    cmd.env("NONO_WORKDIR", workdir);
    cmd.env("NONO_HOOK_TYPE", kind.type_env());
    // ...
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            let _ = nix::unistd::setpgid(...);
            Ok(())
        });
    }
```

**PACK_DIR injection from `7d274cf7`** — insert AFTER the existing env injections and BEFORE `env_clear()` removal makes context:
```rust
// After NONO_WORKDIR, add:
if let Some(pack_dir) = pack_dir {
    cmd.env("PACK_DIR", pack_dir);
}
```

**D-14 invariant:** `hook_runtime_windows.rs` line 301 (`cmd.env_clear()`) and the SystemRoot/windir/SystemDrive restore block (lines 307-326) MUST NOT be modified. The `e54cf9cb` cherry-pick applies ONLY to `hook_runtime.rs` (Unix path).

---

### `crates/nono-proxy/src/config.rs` (model, CRUD)

**Role:** FEAT-03 — add `AwsAuthConfig` struct and `aws_auth: Option<AwsAuthConfig>` field to `RouteConfig`.

**Existing additive-field pattern in `RouteConfig`** (lines 88-171) — new field follows the same `#[serde(default)]` pattern as all optional fields:
```rust
// Existing pattern (oauth2 field, lines 169-171):
/// Optional OAuth2 client_credentials configuration.
/// Mutually exclusive with `credential_key`.
#[serde(default)]
pub oauth2: Option<OAuth2Config>,

// New field to add (same pattern):
/// Optional AWS auth configuration.
/// Mutually exclusive with `credential_key` and `oauth2`.
#[serde(default)]
pub aws_auth: Option<AwsAuthConfig>,
```

**New struct — follow `OAuth2Config` shape**:
```rust
/// AWS SigV4 authentication configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AwsAuthConfig {
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
}
```

**Test-site impact:** Every `RouteConfig { ... }` literal in tests under `crates/nono-proxy/src/` needs `aws_auth: None` added (compiler enforces exhaustive struct init).

---

### `crates/nono-proxy/src/credential.rs` (service, CRUD)

**Role:** FEAT-03 — add `aws_routes: HashMap<String, ()>` to `CredentialStore`; add `get_aws()` accessor; update `load()`, `empty()`, `is_empty()`, `len()`, `loaded_prefixes()`.

**Existing `CredentialStore` struct + methods pattern** (lines 123-261):
```rust
pub struct CredentialStore {
    credentials: HashMap<String, LoadedCredential>,
    // Add: aws_routes: HashMap<String, ()>,
}

impl CredentialStore {
    pub fn load(routes: &[RouteConfig]) -> Result<Self> { ... }
    pub fn empty() -> Self { Self { credentials: HashMap::new() } }
    pub fn get(&self, prefix: &str) -> Option<&LoadedCredential> { ... }
    pub fn is_empty(&self) -> bool { self.credentials.is_empty() }
    pub fn len(&self) -> usize { self.credentials.len() }
    pub fn loaded_prefixes(&self) -> std::collections::HashSet<String> { ... }
}
```

**New `get_aws()` accessor pattern** — mirror `get()`:
```rust
#[must_use]
pub fn get_aws(&self, prefix: &str) -> Option<&()> {
    self.aws_routes.get(prefix)
}
```

**AWS 501 stub — fork-specific (D-15):** The upstream 501 short-circuit lives in `tls_intercept/handle.rs` (won't-apply). After absorbing the FEAT-03 shared hunks, add a 501 response in the non-TLS server path when `aws_route.is_some()`. Pattern: check `crate::route::injection_mode_for_route()` return, emit `hyper::StatusCode::NOT_IMPLEMENTED` with body `"AWS auth not supported on this proxy path"`. This is the one net-new fork divergence in FEAT-03.

---

### `crates/nono-proxy/src/route.rs` (service, request-response)

**Role:** FEAT-03 — add `aws_auth.is_some()` to `requires_managed_credential`; update `missing_managed_credential()` signature; add AWS arms to `auth_mechanism_for_route()` and `injection_mode_for_route()`.

**Existing `requires_managed_credential` pattern** (existing function that references `credential_key` and `oauth2`):
The new `aws_auth` arm follows the existing `oauth2` arm convention: `route.aws_auth.is_some()` treated symmetrically.

---

### `crates/nono-cli/src/profile/mod.rs` (service, CRUD)

**Role:** FEAT-01 (set_vars profile parse + validate_set_vars call), FEAT-03 (aws_auth mutual-exclusion validation in `validate_custom_credential()`), FEAT-05 ($PACK_DIR source_pack), FEAT-06 (profile namespace forms).

**Existing `validate_custom_credential()` pattern** — the mutual-exclusion check lives at profile-load-time. The existing `oauth2` mutual-exclusion (from Phase 22) is the direct template:
```rust
// Existing oauth2 mutual-exclusion pattern (find in profile/mod.rs):
// If credential_key is Some AND oauth2 is Some → error
// New: if aws_auth is Some AND (credential_key is Some OR oauth2 is Some) → error
```

**Profile namespace alias registration** (D-07/D-08): The `get_policy_profile()` and `list_policy_profiles()` calls in `builtin.rs` (lines 9-16) delegate to `crate::policy` — the alias lookup must be wired at the `policy.rs` level so that `get_policy_profile("always-further/claude")` returns the same profile object as `get_policy_profile("claude-code")`. Pattern: the policy resolver checks both the canonical key and the alias map before returning `None`.

---

### `crates/nono/src/keystore.rs` (utility, request-response)

**Role:** FEAT-04 — add `NONO_KEYRING_TIMEOUT_SECS` env-var timeout (default 120s, 0 = no timeout).

**Existing env-var parsing pattern in the codebase** (`config/mod.rs` lines 42-53):
```rust
pub fn validated_home() -> Result<String> {
    let (home, source_var) = resolve_home_env()?;
    if !Path::new(&home).is_absolute() {
        return Err(NonoError::EnvVarValidation {
            var: source_var.to_string(),
            reason: format!("must be an absolute path, got: {}", home),
        });
    }
    Ok(home)
}
```

**Timeout env-var pattern to add** (from upstream `c6b13345`):
```rust
fn keyring_timeout_secs() -> Option<u64> {
    let raw = std::env::var("NONO_KEYRING_TIMEOUT_SECS").ok()?;
    match raw.parse::<u64>() {
        Ok(0) => None,           // 0 = no timeout (wait forever)
        Ok(n) => Some(n),
        Err(_) => {
            tracing::warn!(
                "NONO_KEYRING_TIMEOUT_SECS='{}' is not a valid u64; defaulting to 120s",
                raw
            );
            Some(120)
        }
    }
}
```

**Note:** Invalid values fall back to 120s with `tracing::warn!` (not an error — matches the RESEARCH.md specification).

---

### `crates/nono-cli/src/update_check.rs` (utility, request-response)

**Role:** FEAT-06a — add `detect_ci_provider() -> Option<&'static str>` (pure env-var lookup, no cfg gates).

**Existing env-var lookup pattern in the file** — mirrors the existing CI-detection-adjacent code. New function is additive, pure `std::env::var` calls, no platform cfg needed.

---

### `crates/nono-cli/src/cli.rs` (config, request-response)

**Role:** FEAT-06c — add `BoolishValueParser` to selected flags; wire `NONO_TRUST_OVERRIDE` env source.

**Existing clap flag pattern** (the file already has `--trust-proxy-ca`, `--trust-override`, `--capability-elevation` flags — add `BoolishValueParser` to each). Pattern: look at how existing env-var-backed flags are wired (likely using `.env()` on the clap `Arg`).

---

### `crates/nono-cli/data/policy.json` (config, CRUD)

**Role:** D-07/D-08 — add namespace forms as aliases for built-in profiles (`claude-code` → `always-further/claude`), namespace fork-only profiles (`swival` → `swival/default`). The internal key stays as the bare name; the namespace form is a registered alias.

**Existing profile entry shape** (policy.json `§profiles` starting at line 605): Each profile entry has `"name"`, `"extends"`, and domain-specific fields. The alias mechanism may be a new top-level `"profile_aliases"` object, or inline per-profile. The planner should look at how the policy resolver in `policy.rs` currently handles profile lookup and extend it consistently.

---

### `crates/nono-cli/src/profile/builtin.rs` (config, CRUD)

**Role:** D-07/D-08 — update test coverage to assert both bare names and namespace forms resolve. The existing `get_builtin()` tests (lines 24-31, 71-77, 163-190) are the direct template:

**Existing test pattern** (lines 24-31):
```rust
#[test]
fn test_get_builtin_claude_code() {
    let profile = get_builtin("claude-code").expect("Profile not found");
    assert_eq!(profile.meta.name, "claude-code");
    // ...
}
```

**New alias test pattern** (add alongside existing tests):
```rust
#[test]
fn test_get_builtin_claude_code_by_namespace_alias() {
    let profile = get_builtin("always-further/claude").expect("Alias not found");
    assert_eq!(profile.meta.name, "claude-code");  // canonical internal name unchanged
}
```

**`test_list_builtin()` pattern** (lines 366-380) — currently asserts specific profile names; must be updated to assert aliases appear if `list_builtin()` includes them, or remain unchanged if aliases are resolution-only.

---

### `bindings/c/src/lib.rs` (utility, request-response)

**Role:** CR-01 — add `clear_last_call_state()` helper that resets all three thread-locals atomically.

**Existing thread-local store** (lines 46-51):
```rust
thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
    static LAST_DIAGNOSTIC_CODE: RefCell<Option<types::NonoDiagnosticCode>> =
        const { RefCell::new(None) };
    static LAST_REMEDIATION_JSON: RefCell<Option<String>> = const { RefCell::new(None) };
}
```

**Existing `nono_clear_error()` pattern** (lines 263-273 — this clears all three; `clear_last_call_state()` is the internal version of the same operation):
```rust
#[unsafe(no_mangle)]
pub extern "C" fn nono_clear_error() {
    LAST_ERROR.with(|cell| { *cell.borrow_mut() = None; });
    LAST_DIAGNOSTIC_CODE.with(|cell| { *cell.borrow_mut() = None; });
    LAST_REMEDIATION_JSON.with(|cell| { *cell.borrow_mut() = None; });
}
```

**New `clear_last_call_state()` helper** (add at the same level as `set_last_error`, `last_diagnostic_code`, `last_remediation_json`):
```rust
pub(crate) fn clear_last_call_state() {
    LAST_ERROR.with(|c| *c.borrow_mut() = None);
    LAST_DIAGNOSTIC_CODE.with(|c| *c.borrow_mut() = None);
    LAST_REMEDIATION_JSON.with(|c| *c.borrow_mut() = None);
}
```

**Existing `map_error()` pattern** (lines 87-193) — `map_error` already sets all three correctly; the bug is that `set_last_error`-only call sites set `LAST_ERROR` without resetting `LAST_DIAGNOSTIC_CODE`. The fix is call `clear_last_call_state()` at entry of each `pub unsafe extern "C"` fn, before any other operation.

---

### `bindings/c/src/diagnostic.rs` (utility, request-response)

**Role:** CR-01 — add `clear_last_call_state()` call at entry of `nono_session_diagnostic_report_to_json()` and `nono_merge_diagnostic_report_json()`.

**Current entry of `nono_session_diagnostic_report_to_json()`** (lines 37-48):
```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nono_session_diagnostic_report_to_json(
    exit_code: i32,
    denials_json: *const c_char,
    ipc_denials_json: *const c_char,
    violations_json: *const c_char,
) -> *mut c_char {
    let denials = match parse_denials(denials_json) {
```

**After CR-01 fix:**
```rust
pub unsafe extern "C" fn nono_session_diagnostic_report_to_json(...) -> *mut c_char {
    crate::clear_last_call_state();  // CR-01: reset stale diagnostic state from prior call
    let denials = match parse_denials(denials_json) {
```

**Current entry of `nono_merge_diagnostic_report_json()`** (lines 92-99 — the two `set_last_error` paths that are not preceded by a reset):
```rust
pub unsafe extern "C" fn nono_merge_diagnostic_report_json(...) -> *mut c_char {
    if session_json.is_null() {
        set_last_error("session_json is null");  // BUG: LAST_DIAGNOSTIC_CODE stale
        return std::ptr::null_mut();
    }
```

**After CR-01 fix:**
```rust
pub unsafe extern "C" fn nono_merge_diagnostic_report_json(...) -> *mut c_char {
    crate::clear_last_call_state();  // CR-01: reset stale diagnostic state from prior call
    if session_json.is_null() {
        set_last_error("session_json is null");
        return std::ptr::null_mut();
    }
```

**CR-01 regression test pattern** (add to `diagnostic.rs` tests section, lines 146+):
```rust
#[test]
fn diagnostic_code_is_cleared_between_calls() {
    // Arrange: populate LAST_DIAGNOSTIC_CODE via map_error path
    // (use nono_session_diagnostic_report_to_json with invalid JSON to trigger map_error)
    // ...

    // Act: call the set_last_error-only path
    let json_ptr = unsafe {
        nono_merge_diagnostic_report_json(std::ptr::null(), std::ptr::null())
    };
    assert!(json_ptr.is_null());

    // Assert: diagnostic code is Other (reset at entry), NOT stale code from prior call
    assert_eq!(nono_last_diagnostic_code(), NonoDiagnosticCode::Other);
}
```

---

### `crates/nono/Cargo.toml` + `Cargo.lock` (config)

**Role:** DEPS-02 — typify spec edit + 8 lockfile-only bumps.

**Only Cargo.toml edit** (`crates/nono/Cargo.toml` line 71):
```toml
# BEFORE:
typify = { version = "0.6", ... }
# AFTER:
typify = { version = "0.7", ... }
```

**Lockfile bumps via `cargo update`** — no Cargo.toml changes needed for the other 8 packages. Run:
```
cargo update -p typify
cargo update -p cbindgen
cargo update -p hyper
cargo update -p zeroize
cargo update -p time
cargo update -p chrono
cargo update -p ignore
cargo update -p which
cargo update -p x509-parser
```

**D-06 path-dep pin gate** — after the DEPS commit, verify no accidental crate-version drift:
```
grep "version" Cargo.toml crates/*/Cargo.toml bindings/c/Cargo.toml | grep -E 'nono = \{|nono-proxy = \{'
```

---

## Shared Patterns

### NONO_TEST_HOME env-override (cross-cutting for all state-path tests)
**Source:** `crates/nono-cli/src/config/mod.rs` lines 130-153 + test lines 398-410
**Apply to:** All tests in `state_paths.rs` that need to override the state root
```rust
// Pattern: acquire ENV_LOCK, use EnvVarGuard::set_all, let _guard drop at end of test
let _guard = test_env_lock().lock().expect("env lock");
let _env = EnvVarGuard::set_all(&[("NONO_TEST_HOME", abs_path)]);
// ... assertions ...
// _env drops here, restoring original env
```

### `NonoError::Setup` for environment validation failures
**Source:** `crates/nono-cli/src/provision_windows.rs` lines 274-281
**Apply to:** `state_paths::user_state_dir()` Windows arm (D-02), `keyring_timeout_secs()` fallback
```rust
return Err(NonoError::Setup(
    "descriptive message about what env var is missing/empty".to_string(),
));
```

### `#[must_use]` on accessor functions
**Source:** `bindings/c/src/lib.rs` lines 72-80, `credential.rs` lines 235-256
**Apply to:** `clear_last_call_state()` return (it's `()`, so `#[must_use]` not applicable), `get_aws()` accessor
```rust
#[must_use]
pub fn get_aws(&self, prefix: &str) -> Option<&()> { ... }
```

### `tracing::warn!` for non-fatal fallback behavior
**Source:** `credential.rs` lines 168-174
**Apply to:** `keyring_timeout_secs()` invalid-value fallback, `proxy/credential.rs` load-failure warn for AWS routes
```rust
warn!(
    "Credential '{}' not available for route '{}': {}. \
     Managed-credential requests on this route will be denied until the credential is available.",
    key, normalized_prefix, msg
);
```

### Cherry-pick commit message pattern (D-12)
**Source:** Phase 86/87 history (8 cherry-picks with `-x` + DCO)
**Apply to:** All 14 upstream SHA cherry-picks in this wave
```
feat(scope): <upstream commit subject>

cherry-picked from commit <SHA>

Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
```

### Deliberate fork-divergence commit (CR-01 / D-11)
**Source:** Phase 87 CR-02 addendum pattern in `85-DIVERGENCE-LEDGER.md` lines ~807-828
**Apply to:** CR-01 commit only (NOT a cherry-pick; no `-x` upstream SHA)
```
fix(ffi): clear stale diagnostic state on every FFI entry (CR-01)

Deliberate fork-divergence: clear LAST_DIAGNOSTIC_CODE and LAST_REMEDIATION_JSON
at the entry of nono_session_diagnostic_report_to_json() and
nono_merge_diagnostic_report_json() to prevent stale diagnostic codes from a
prior call on the same thread.

Upstream-equivalent fix at a6aa9995 (already absorbed via Phase 86 boundary
convergence; this is the fork-specific extension to the clear-on-entry pattern).
Recorded as fork-divergence in 85-DIVERGENCE-LEDGER.md Phase 88 addendum.

Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
```

### PARTIAL→CI deferral note pattern
**Source:** `.planning/templates/cross-target-verify-checklist.md`
**Apply to:** commits touching `exec_strategy.rs`, `pty_proxy.rs`, `hook_runtime.rs`, `audit_session.rs`, `state_paths.rs` (Windows arm cfg gates)
```
# In verification document, per commit:
- [ ] PARTIAL: Windows-host cargo clippy cannot verify #[cfg(unix)] / nix:: usage
      → Deferred to GH Actions Linux/macOS CI leg per CLAUDE.md MUST/NEVER rule
```

---

## No Analog Found

All 22 files/areas have strong analogs within the codebase. No greenfield surfaces require falling back to RESEARCH.md patterns only. However, two fork-specific adaptations have no upstream analog and must be authored from scratch:

| Adaptation | Role | Reason | Use Pattern From |
|------------|------|---------|------------------|
| 501 stub in `proxy/server.rs` non-TLS path when `aws_route.is_some()` | service | Upstream's 501 is in `tls_intercept/handle.rs` (won't-apply, D-15) | Existing error-response pattern in `server.rs`; emit `hyper::StatusCode::NOT_IMPLEMENTED` |
| `state_paths::user_state_dir()` Windows arm (D-02) | utility | Upstream's implementation lacks the `%LOCALAPPDATA%` arm (platform-agnostic) | `provision_windows.rs::scratch_dir()` lines 273-284 |

---

## Metadata

**Analog search scope:** `crates/nono/src/`, `crates/nono-cli/src/`, `crates/nono-proxy/src/`, `bindings/c/src/`
**Files scanned:** 14 files read directly; 8 files confirmed by Grep/Glob
**Pattern extraction date:** 2026-06-20
