# Phase 57: bitwarden-credential-source — Pattern Map

**Mapped:** 2026-06-05
**Files analyzed:** 1 (all new code units live in `crates/nono/src/keystore.rs`)
**Analogs found:** 8 / 8 (all within keystore.rs — the op:// block is the direct analog for every new code unit)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono/src/keystore.rs` | utility / secret-loader | request-response (subprocess + JSON) | `crates/nono/src/keystore.rs` — the `op://` block | exact |
| `crates/nono/src/error.rs` | error types | N/A | `crates/nono/src/error.rs` — `KeystoreAccess` / `SecretNotFound` | exact |

All new code units below live inside `crates/nono/src/keystore.rs`.

---

## Pattern Assignments

### 1. Constants (`BW_URI_PREFIX`, `BW_ITEM_SEGMENT`, `BW_SECRET_SEGMENT`)

**Analog:** `OP_URI_PREFIX` (line 45), `KEYRING_URI_PREFIX` (line 54)

**Imports / constants pattern** (lines 44–55):
```rust
/// The `op://` URI scheme prefix, indicating 1Password CLI backend.
const OP_URI_PREFIX: &str = "op://";

/// The `keyring://` URI scheme prefix, indicating a custom-service keyring lookup.
const KEYRING_URI_PREFIX: &str = "keyring://";
```

**Copy as:**
```rust
/// The `bw://` URI scheme prefix, indicating a Bitwarden backend (bw CLI or bws CLI).
const BW_URI_PREFIX: &str = "bw://";

/// First path segment selecting the `bw` password-manager CLI backend.
const BW_ITEM_SEGMENT: &str = "item";

/// First path segment selecting the `bws` Secrets Manager CLI backend.
const BW_SECRET_SEGMENT: &str = "secret";
```

Place immediately after the `OP_URI_PREFIX` block (after line 45). No structural changes to the imports block (lines 20–26) — all needed types are already imported (`Command`, `Stdio`, `Zeroizing`, `NonoError`, `Result`). Add `use serde_json;` only if the crate is not already in scope; `serde_json` is already a direct dep of `crates/nono` per the workspace manifest.

---

### 2. `is_bw_uri` (URI scheme detector)

**Analog:** `is_op_uri` (lines 275–278), `is_keyring_uri` (lines 366–369)

**Analog excerpt** (lines 275–278):
```rust
/// Returns true if the credential reference is a 1Password `op://` URI.
#[must_use]
pub fn is_op_uri(credential_ref: &str) -> bool {
    credential_ref.starts_with(OP_URI_PREFIX)
}
```

**Copy as:**
```rust
/// Returns true if the credential reference is a Bitwarden `bw://` URI.
#[must_use]
pub fn is_bw_uri(credential_ref: &str) -> bool {
    credential_ref.starts_with(BW_URI_PREFIX)
}
```

---

### 3. `validate_bw_uri` (structural + injection-safe charset validation)

**Analog:** `validate_op_uri` (lines 230–272) — closest structural match. Also consult `validate_apple_password_uri` (lines 300–338) for the two-segment exact-count pattern used by `secret/<uuid>`.

**Analog excerpt — full `validate_op_uri`** (lines 230–272):
```rust
pub fn validate_op_uri(uri: &str) -> Result<()> {
    let path = uri.strip_prefix(OP_URI_PREFIX).ok_or_else(|| {
        NonoError::ConfigParse(format!(
            "credential reference '{}' does not start with '{}'",
            uri, OP_URI_PREFIX
        ))
    })?;

    // Reject shell metacharacters to prevent injection
    if let Some(bad) = path.chars().find(|c| FORBIDDEN_URI_CHARS.contains(c)) {
        return Err(NonoError::ConfigParse(format!(
            "1Password URI contains forbidden character {:?}: {}",
            bad, uri
        )));
    }

    // Reject query strings and fragments
    if path.contains('?') || path.contains('#') {
        return Err(NonoError::ConfigParse(format!(
            "1Password URI must not contain query strings or fragments: {}",
            uri
        )));
    }

    // Split into segments: vault/item/field (minimum 3)
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 3 {
        return Err(NonoError::ConfigParse(format!(
            "1Password URI must have at least vault/item/field segments: {}",
            uri
        )));
    }

    // No empty segments
    if segments.iter().any(|s| s.is_empty()) {
        return Err(NonoError::ConfigParse(format!(
            "1Password URI has empty path segment: {}",
            uri
        )));
    }

    Ok(())
}
```

**Key differences for `validate_bw_uri`:**
- After `FORBIDDEN_URI_CHARS` and `?`/`#` rejection, dispatch on `segments[0]` (`item` vs `secret`) rather than a minimum-count check.
- `item/` form: 3 segments for reserved selectors (`item/<id>/password`), 4 segments for custom fields (`item/<id>/field/<name>`). Each `<id>` must pass `is_valid_bw_id`.
- `secret/` form: exactly 2 segments (`secret/<uuid>`). Any additional segments are a validation error per D-06.
- Add a private `is_valid_bw_id(s: &str) -> bool` helper that accepts `!s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')`.

---

### 4. `redact_bw_uri` (safe log rendering)

**Analog:** `redact_op_uri` (lines 1244–1254), `redact_keyring_uri` (lines 532–551)

**Analog excerpt — `redact_op_uri`** (lines 1244–1254):
```rust
pub fn redact_op_uri(uri: &str) -> String {
    if let Some(path) = uri.strip_prefix(OP_URI_PREFIX) {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() >= 3 {
            return format!("op://{}/{}/<redacted>", parts[0], parts[1]);
        }
    }
    "op://***".to_string()
}
```

**Copy as:**
```rust
/// Redact the selector/UUID segment of a `bw://` URI for safe logging.
///
/// `bw://item/<id>/<selector>` → `bw://item/<id>/<redacted>`
/// `bw://secret/<uuid>`        → `bw://secret/<redacted>`
pub fn redact_bw_uri(uri: &str) -> String {
    if let Some(path) = uri.strip_prefix(BW_URI_PREFIX) {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        match parts.as_slice() {
            [seg, id, _] if *seg == BW_ITEM_SEGMENT => {
                return format!("bw://item/{}/<redacted>", id);
            }
            [seg, _] if *seg == BW_SECRET_SEGMENT => {
                return "bw://secret/<redacted>".to_string();
            }
            _ => {}
        }
    }
    "bw://***".to_string()
}
```

Note: `splitn(3, '/')` on `item/<id>/field/name` will produce `["item", "<id>", "field/name"]` — the third element absorbs the rest. For `item` URIs the field is always in position 2, which is what gets redacted. This mirrors `redact_op_uri` exactly.

---

### 5. `load_from_op` — subprocess spawn, timeout, stdout-to-Zeroizing pattern

This is the most critical analog. Every subprocess-backed loader (`load_from_bw`, `load_from_bws`, and the second `bw get totp` call) follows this pattern exactly.

**Analog excerpt — `load_from_op`** (lines 977–1028):
```rust
fn load_from_op(uri: &str) -> Result<Zeroizing<String>> {
    validate_op_uri(uri)?;

    tracing::debug!("Loading secret from 1Password: {}", redact_op_uri(uri));

    let mut child = Command::new("op")
        .args(["read", "--", uri])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                NonoError::KeystoreAccess(
                    "1Password CLI ('op') not found. \
                     Install it from https://developer.1password.com/docs/cli/"
                        .to_string(),
                )
            } else {
                NonoError::KeystoreAccess(format!("Could not start the 1Password CLI: {}", e))
            }
        })?;

    let output = wait_with_timeout(
        &mut child,
        SECRET_MANAGER_TIMEOUT,
        "1Password CLI",
        "Is 1Password waiting for authentication?",
    )
    .inspect_err(|_e| {
        let _ = child.kill();
        let _ = child.wait();
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(classify_op_error(&stderr, uri));
    }

    // Convert stdout to string, trim trailing newline, wrap in Zeroizing.
    let raw = String::from_utf8(output.stdout).map_err(|_| {
        NonoError::KeystoreAccess(format!(
            "1Password returned non-UTF-8 data for '{}'",
            redact_op_uri(uri)
        ))
    })?;

    let trimmed = raw.trim_end_matches(['\n', '\r']).to_string();
    Ok(Zeroizing::new(trimmed))
}
```

**Apply this pattern to:**
- `load_from_bw(uri)` — binary `"bw"`, args `["get", "item", "--nointeraction", "--", item_id]`, pre-flight check `BW_SESSION` non-empty, parse stdout as JSON then call `extract_bw_field`.
- `load_from_bws(uri)` — binary `"bws"`, args `["secret", "get", "--", secret_uuid]`, pre-flight check `BWS_ACCESS_TOKEN` non-empty, extract `json["value"].as_str()`.
- `load_totp_via_bw_get_totp(item_id, uri)` — binary `"bw"`, args `["get", "totp", "--nointeraction", "--", item_id]`, `BW_SESSION` pre-flight, returns trimmed stdout directly (no JSON parse needed).

**Critical deviations from `load_from_op`:**
1. Pre-flight token check before spawn (not in `load_from_op` because `op` handles auth internally). Mirror `load_from_env`'s empty-check pattern (lines 727–730) for the `BW_SESSION` / `BWS_ACCESS_TOKEN` validation.
2. For `load_from_bw`, stdout is parsed as JSON rather than returned raw — call `serde_json::from_slice(&output.stdout)` then extract the requested field.
3. Do NOT pass `BW_SESSION` as a CLI arg (`--session`). Let `bw` read it from the environment implicitly (pitfall 2 in RESEARCH.md).

**`load_from_env` empty-check pattern** (lines 727–730) for pre-flight token validation:
```rust
match std::env::var(var_name) {
    Ok(value) if value.is_empty() => Err(NonoError::SecretNotFound(format!(
        "environment variable '{}' is set but empty",
        var_name
    ))),
```

---

### 6. `classify_op_error` — stderr-to-NonoError classification

**Analog excerpt — `classify_op_error`** (lines 1185–1214):
```rust
fn classify_op_error(stderr: &str, uri: &str) -> NonoError {
    let redacted = redact_op_uri(uri);
    let stderr_trimmed = stderr.trim();

    if stderr.contains("not signed in")
        || stderr.contains("sign in")
        || stderr.contains("authentication required")
        || stderr.contains("session expired")
    {
        NonoError::KeystoreAccess(format!(
            "1Password authentication required for '{}'. \
             Run 'op signin' or set OP_SERVICE_ACCOUNT_TOKEN. \
             Detail: {}",
            redacted, stderr_trimmed
        ))
    } else if stderr.contains("not found")
        || stderr.contains("could not find")
        || stderr.contains("isn't an item")
    {
        NonoError::SecretNotFound(format!(
            "1Password item not found: '{}'. Detail: {}",
            redacted, stderr_trimmed
        ))
    } else {
        NonoError::KeystoreAccess(format!(
            "1Password CLI failed for '{}': {}",
            redacted, stderr_trimmed
        ))
    }
}
```

**Copy as `classify_bw_error`:**
- Auth keywords: `"Vault is locked"`, `"not logged in"`, `"Session key is invalid"`, `"Invalid master password"` → `KeystoreAccess` with "Run `bw unlock --raw` and export BW_SESSION" remediation.
- Not-found keywords: `"not found"`, `"No items"`, `"invalid UUID"` → `SecretNotFound`.
- Fallthrough: generic `KeystoreAccess`.

**Copy as `classify_bws_error`:**
- Auth keywords: `"Missing access token"`, `"access token"`, `"Unauthorized"`, `"authentication"` → `KeystoreAccess` with "Set BWS_ACCESS_TOKEN" remediation.
- Not-found keywords: `"not found"`, `"404"` → `SecretNotFound`.
- Fallthrough: generic `KeystoreAccess`.

Both functions take `(stderr: &str, uri: &str)` and return `NonoError` — identical signature to `classify_op_error`.

---

### 7. `load_secret_by_ref` dispatch ladder — inserting the `bw://` branch

**Analog:** The existing dispatch ladder (lines 204–218):
```rust
pub fn load_secret_by_ref(service: &str, credential_ref: &str) -> Result<Zeroizing<String>> {
    if credential_ref.starts_with(FILE_URI_PREFIX) {
        load_from_file(credential_ref)
    } else if credential_ref.starts_with(ENV_URI_PREFIX) {
        load_from_env(credential_ref)
    } else if credential_ref.starts_with(OP_URI_PREFIX) {
        load_from_op(credential_ref)
    } else if is_apple_password_uri(credential_ref) {
        load_from_apple_password(credential_ref)
    } else if is_keyring_uri(credential_ref) {
        load_from_keyring_uri(credential_ref)
    } else {
        load_single_secret(service, credential_ref)
    }
}
```

**Insert before the `is_keyring_uri` branch:**
```rust
    } else if is_bw_uri(credential_ref) {
        load_from_bw_dispatch(credential_ref)
```

Where `load_from_bw_dispatch` fans out by first segment:
```rust
fn load_from_bw_dispatch(uri: &str) -> Result<Zeroizing<String>> {
    validate_bw_uri(uri)?;
    let path = uri.strip_prefix(BW_URI_PREFIX).ok_or_else(|| {
        NonoError::ConfigParse(format!("invalid bw:// URI: {}", uri))
    })?;
    match path.split('/').next() {
        Some(BW_ITEM_SEGMENT)   => load_from_bw(uri),
        Some(BW_SECRET_SEGMENT) => load_from_bws(uri),
        other => Err(NonoError::ConfigParse(format!(
            "Unknown bw:// backend segment '{:?}': expected 'item' or 'secret'", other
        ))),
    }
}
```

Note: `validate_bw_uri` is called at the top of `load_from_bw_dispatch` (not again inside `load_from_bw`/`load_from_bws`) to match the `load_from_op` → `validate_op_uri` once-at-top pattern.

---

### 8. `build_mappings_from_list` — `bw://` requires `=VAR_NAME`

**Analog:** The `op://` branch (lines 1433–1460):
```rust
        } else if entry.starts_with(OP_URI_PREFIX) {
            if let Some(eq_pos) = entry.rfind('=') {
                let uri = &entry[..eq_pos];
                let var_name = &entry[eq_pos + 1..];

                if var_name.is_empty() {
                    return Err(NonoError::ConfigParse(format!(
                        "1Password credential '{}' has '=' but no variable name. \
                         Use format: op://vault/item/field=MY_VAR",
                        redact_op_uri(uri)
                    )));
                }

                validate_op_uri(uri)?;
                validate_destination_env_var(var_name)?;

                mappings.insert(uri.to_string(), var_name.to_string());
            } else {
                return Err(NonoError::ConfigParse(format!(
                    "1Password credential requires an explicit variable name. \
                     Use format: op://vault/item/field=MY_VAR (got '{}')",
                    redact_op_uri(entry)
                )));
            }
```

**Copy as `bw://` branch** — insert immediately after the `op://` branch, before `is_apple_password_uri`:
```rust
        } else if entry.starts_with(BW_URI_PREFIX) {
            if let Some(eq_pos) = entry.rfind('=') {
                let uri = &entry[..eq_pos];
                let var_name = &entry[eq_pos + 1..];
                if var_name.is_empty() {
                    return Err(NonoError::ConfigParse(format!(
                        "Bitwarden credential '{}' has '=' but no variable name. \
                         Use format: bw://item/<id>/password=MY_VAR",
                        redact_bw_uri(uri)
                    )));
                }
                validate_bw_uri(uri)?;
                validate_destination_env_var(var_name)?;
                mappings.insert(uri.to_string(), var_name.to_string());
            } else {
                return Err(NonoError::ConfigParse(format!(
                    "Bitwarden credential requires an explicit variable name. \
                     Use format: bw://item/<id>/password=MY_VAR (got '{}')",
                    redact_bw_uri(entry)
                )));
            }
```

Also add `bw://` validation to `build_mappings_from_pairs` (lines 1505–1511), mirroring the `op://` branch there:
```rust
        } else if is_bw_uri(credential_ref) {
            validate_bw_uri(credential_ref)?;
```

---

### 9. `#[cfg(test)]` module — test structure and attribute pattern

**Analog:** The existing test module (lines 1559–2910):
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::disallowed_methods)] // Tests use unique env var names (NONO_TEST_*), no contention.
mod tests {
    use super::*;
```

All `bw://` tests belong in this same `mod tests` block. No new module is needed.

**Test pattern — positive validation** (mirrors `test_validate_op_uri_valid_3_segments`, lines 1761–1763):
```rust
    #[test]
    fn test_validate_bw_uri_valid_item_password() {
        assert!(validate_bw_uri("bw://item/abc123def456/password").is_ok());
    }
```

**Test pattern — forbidden char** (mirrors `test_validate_op_uri_forbidden_semicolon`, lines 1850–1858):
```rust
    #[test]
    fn test_validate_bw_uri_forbidden_char() {
        let err = validate_bw_uri("bw://item/abc;rm-rf/password").expect_err("should be rejected");
        assert!(err.to_string().contains("forbidden character"), "got: {}", err);
    }
```

**Test pattern — dispatch error path via env var** (mirrors `test_load_secret_by_ref_dispatches_op`, lines 2104–2116):
```rust
    #[test]
    fn test_load_from_bw_no_session() {
        // BW_SESSION absent → KeystoreAccess with actionable message
        // Use a unique env-var name prefix NONO_TEST_* to avoid test interference.
        unsafe { std::env::remove_var("BW_SESSION") };
        let result = load_from_bw("bw://item/abc123def456/password");
        assert!(result.is_err());
        let err = result.expect_err("should fail").to_string();
        assert!(err.contains("BW_SESSION"), "got: {}", err);
    }
```

**Test pattern — missing CLI binary** (mirrors `test_load_secret_by_ref_dispatches_op` checking for "op" in error):
- Invoke `load_from_bw` / `load_from_bws` with valid URI and env token set, but with a PATH that has no `bw`/`bws` binary. Verify the error contains `"'bw'"` or `"'bws'"` and `"not found"`.

**Test pattern — `build_mappings` rejection** (mirrors `test_build_mappings_op_uri_without_var_rejected`, lines 1632–1641):
```rust
    #[test]
    fn test_build_mappings_bw_uri_without_var_rejected() {
        let err = build_mappings_from_list("bw://item/abc123/password")
            .expect_err("should reject bare bw:// URI");
        assert!(err.to_string().contains("explicit variable name"), "got: {}", err);
    }
```

**Test pattern — `extract_bw_field` using static JSON fixture** (no analog in existing tests — uses `serde_json::json!` macro):
```rust
    #[test]
    fn test_extract_bw_field_password() {
        let json = serde_json::json!({
            "login": { "username": "alice", "password": "hunter2" },
            "notes": null,
            "fields": []
        });
        let result = json_str_field(&json, &["login", "password"], "bw://item/abc/<redacted>");
        assert_eq!(result.unwrap().as_str(), "hunter2");
    }
```

---

## Shared Patterns

### Zeroizing secret return
**Source:** `crates/nono/src/keystore.rs` — every `load_from_*` function
**Apply to:** `load_from_bw`, `load_from_bws`, `load_totp_via_bw_get_totp`

All functions that return a resolved secret must return `Result<Zeroizing<String>>`. The raw `String` is wrapped immediately after extraction:
```rust
Ok(Zeroizing::new(trimmed))        // for raw-stdout backends
Zeroizing::new(s.to_string())      // for JSON field extraction
```

### `wait_with_timeout` reuse
**Source:** `crates/nono/src/keystore.rs` lines 1283–1333
**Apply to:** All three subprocess calls (`bw get item`, `bws secret get`, `bw get totp`)

Signature: `wait_with_timeout(&mut child, SECRET_MANAGER_TIMEOUT, "backend name", "hint string") -> Result<Output>`

Chain `.inspect_err` to kill the child process on timeout, exactly as in `load_from_op` (line 1006–1009).

### `FORBIDDEN_URI_CHARS` injection defense
**Source:** `crates/nono/src/keystore.rs` line 130–132
**Apply to:** `validate_bw_uri` — apply to the full path string after stripping the `bw://` prefix, identical to `validate_op_uri` lines 239–244

```rust
const FORBIDDEN_URI_CHARS: &[char] = &[
    ';', '|', '&', '$', '`', '(', ')', '{', '}', '<', '>', '!', '\\', '"', '\'', '\n', '\r', '\0',
];
```

### Error variants
**Source:** `crates/nono/src/error.rs` lines 108–117

```rust
// Keystore errors
#[error("Failed to access system keystore: {0}")]
KeystoreAccess(String),

#[error("Secret not found in keystore: {0}")]
SecretNotFound(String),

// Configuration errors
#[error("Configuration parse error: {0}")]
ConfigParse(String),
```

**Usage rules:**
- `KeystoreAccess` — auth failure (missing/empty token, locked vault, missing CLI binary, subprocess timeout).
- `SecretNotFound` — item/secret/field not found in vault.
- `ConfigParse` — URI validation failure (bad structure, forbidden chars, wrong segment count).

### `#[must_use]` attribute
**Source:** `is_op_uri` line 275, `load_secret_by_ref` line 164
**Apply to:** `is_bw_uri`, `validate_bw_uri`, `redact_bw_uri`, `load_from_bw_dispatch`

```rust
#[must_use]
pub fn is_bw_uri(credential_ref: &str) -> bool { ... }

#[must_use = "loaded secret should be used or explicitly dropped"]
fn load_from_bw_dispatch(uri: &str) -> Result<Zeroizing<String>> { ... }
```

### `tracing::debug!` with redacted URI
**Source:** `load_from_op` line 980, `load_from_keyring_uri` line 1109
**Apply to:** `load_from_bw`, `load_from_bws`

```rust
tracing::debug!("Loading secret from Bitwarden: {}", redact_bw_uri(uri));
```

Never log the secret value or the raw token. Always use `redact_bw_uri(uri)` in debug lines, not `uri`.

---

## No Analog Found

No new files lack analogs. The entire `bw://` implementation slots directly into the existing per-scheme pattern.

The one code unit with no direct in-repo analog is the `serde_json` field extraction helper (`json_str_field` / `extract_bw_field`). The pattern for this comes from RESEARCH.md Pattern 4 (lines 329–360) and is straightforward JSON traversal via `serde_json::Value::get` + `as_str`. The planner should use the RESEARCH.md code sketch verbatim for this unit.

| Code Unit | Reason |
|---|---|
| `json_str_field` / `extract_bw_field` | No existing JSON-parse-from-subprocess pattern in keystore.rs (all other backends return raw strings). Use RESEARCH.md Pattern 4 sketch. |
| `is_valid_bw_id` | No ID allowlist validator exists — closest is `FORBIDDEN_URI_CHARS` (denylist). Implement as `!s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')` per RESEARCH.md URI Validation Design section. |

---

## Metadata

**Analog search scope:** `crates/nono/src/keystore.rs` (lines 1–2911), `crates/nono/src/error.rs` (lines 1–512)
**Files scanned:** 2 (both read in full)
**Pattern extraction date:** 2026-06-05
