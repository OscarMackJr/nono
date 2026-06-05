# Phase 57: Bitwarden Credential Source — Research

**Researched:** 2026-06-05
**Domain:** Rust keystore extension — external CLI subprocess hygiene, JSON parsing, `bw`/`bws` CLI contracts
**Confidence:** HIGH (in-repo patterns), MEDIUM (external CLI contracts — no live CLI available to verify exact output)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Both Bitwarden tools in one scheme, one phase: `bw://item/...` → `bw` CLI; `bw://secret/<uuid>` → `bws` CLI.
- **D-02:** Auth is environment-token only. `bw` backend → `BW_SESSION`; `bws` backend → `BWS_ACCESS_TOKEN`. No interactive unlock.
- **D-03:** URI grammar — typed first segment:
  ```
  bw://item/<id>/password
  bw://item/<id>/username
  bw://item/<id>/totp
  bw://item/<id>/notes
  bw://item/<id>/field/<name>
  bw://secret/<uuid>
  ```
- **D-04:** ID-based addressing only. `<id>` and `<uuid>` validated against strict injection-safe charset (hex/UUID + hyphen). Reject query strings/fragments and any char outside allowed set.
- **D-05:** Reserved field names matched exactly; custom fields go through explicit `field/<name>` prefix to avoid collision.
- **D-06:** `bws` secrets are single opaque values — field selectors do not apply. `bw://secret/<uuid>/anything` is a validation error.
- **D-07:** All fields supported for `bw` items: username, password, TOTP, notes, custom fields. `bw get item` JSON parsed once; field selected from parsed object.
- **D-08:** Fail-closed with clear actionable diagnostic. Token absent, CLI missing, vault locked, item/field not found → abort with named remediation.

### Claude's Discretion

- Exact module decomposition within `keystore.rs` (helper fn names, JSON parse location).
- JSON-parsing approach for `bw get item`.
- Whether `bw`/`bws` are invoked with `--nointeraction`/`--raw`-style flags.
- Test fixture strategy (mock CLI shim vs. trait-injected command runner).

### Deferred Ideas (OUT OF SCOPE)

- Non-interactive auto-unlock (passphrase/client-secret in env → `bw unlock`).
- Addressing items by name.
- Credential writing, session caching/daemonization, org/collection management.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-CRED-01 | `bw://` Bitwarden credential source alongside `keyring://`/`env://`/`file://`; secrets in `Zeroizing<String>` with in-place truncation; cross-platform; isolated to `crates/nono/src/keystore.rs` | External CLI contracts documented below; in-repo pattern mirroring the `op://` backend is the implementation path |
</phase_requirements>

---

## Summary

Phase 57 adds a `bw://` credential scheme to `keystore.rs` with two backends — the `bw` password-manager CLI (for vault items, addressed by item ID) and the `bws` Secrets Manager CLI (for machine-account secrets, addressed by UUID). The implementation slots into the existing is/validate/load + `load_secret_by_ref` dispatch ladder already used by `op://`, `keyring://`, `env://`, and `file://`.

The in-repo analog is fully understood. The research budget was spent on the external CLI contracts (JSON shapes, flags, exit behavior) that the planner cannot read from the codebase. Both CLIs are documented by Bitwarden, though neither publishes exhaustive exit-code tables — the classification approach must rely on stderr substring matching (mirroring `classify_op_error`) rather than on structured exit codes.

**Primary recommendation:** Mirror the `op://` pattern exactly: validate URI → run `Command::new("bw"/"bws")` with explicit args (no shell), capture stdout, parse JSON with `serde_json` (already a workspace dep), select field, wrap in `Zeroizing<String>`. For TOTP specifically, use `bw get totp <id>` (returns the live computed code, not the seed) rather than extracting `login.totp` from `bw get item` (which contains the TOTP seed/otpauth URI, not the current code).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `bw://` URI parsing and validation | Library (`keystore.rs`) | — | Isolated to keystore layer per REQ-CRED-01; no platform-specific code above keystore |
| `bw` CLI subprocess dispatch | Library (`keystore.rs`) | — | Matches `op://` pattern; runs before sandbox activation |
| `bws` CLI subprocess dispatch | Library (`keystore.rs`) | — | Same layer as `bw`; both resolve through same dispatch point |
| JSON field extraction (`bw get item`) | Library (`keystore.rs`) | — | `serde_json` parses stdout inline; stays within keystore module |
| `--credential` flag acceptance | CLI (`nono-cli`) | — | No change needed; existing flag already accepts arbitrary scheme strings and routes to `load_secrets` |

---

## Standard Stack

### Core (all workspace-resident — no new dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde_json` | `1.0.149` (workspace) | Parse `bw get item` JSON output | [VERIFIED: workspace Cargo.toml] already in `[dependencies]` of `crates/nono/Cargo.toml` |
| `zeroize` | `1.*` (workspace) | `Zeroizing<String>` for secret fields | [VERIFIED: workspace Cargo.toml] existing pattern in keystore.rs |
| `std::process::Command` | stdlib | Spawn `bw`/`bws` CLIs | [VERIFIED: keystore.rs line 24] existing pattern used for `op` and `security` |
| `tracing` | `0.1` (workspace) | Debug-level logging of secret loads | [VERIFIED: keystore.rs line 22] existing pattern |
| `thiserror` / `NonoError` | (workspace) | Fail-closed error variants | [VERIFIED: error.rs] `KeystoreAccess` and `SecretNotFound` are the correct variants |

**No new Cargo dependencies are introduced.** `serde_json` is already a direct dependency of `crates/nono`.

### Alternative Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `serde_json` parse of `bw get item` stdout | `bw get password <id>` shortcut | Shortcut only covers password; D-07 requires all fields including TOTP, notes, custom fields. One JSON parse covers all. |
| `bw get item` for TOTP | Extracting `login.totp` from the item JSON | `login.totp` contains the TOTP **seed/otpauth URI**, not the current 6-digit code. `bw get totp <id>` returns the computed time-based token. For `selector=totp`, use the dedicated subcommand. |

---

## Package Legitimacy Audit

No new packages introduced. This phase uses only workspace-resident dependencies.

| Package | Registry | Notes | Disposition |
|---------|----------|-------|-------------|
| `serde_json` | crates.io | Already in workspace; established Rust JSON library | Already approved — no action |

*slopcheck not needed — no new package additions.*

---

## Architecture Patterns

### System Architecture Diagram

```
Profile / --credential flag
        │  bw://item/<id>/<selector>
        │  bw://secret/<uuid>
        ▼
load_secret_by_ref()          (dispatch ladder, keystore.rs ~line 204)
        │
        ├── starts_with("bw://") ──▶ is_bw_uri() ──▶ validate_bw_uri()
        │                                                  │
        │                         ┌────────────────────────┴───────────────────────┐
        │                         │  item/ backend                  secret/ backend │
        │                         ▼                                 ▼               │
        │                  load_from_bw()                  load_from_bws()         │
        │                    │                                   │                  │
        │              BW_SESSION env                     BWS_ACCESS_TOKEN env      │
        │              Command::new("bw")                 Command::new("bws")       │
        │              args: ["get","item","--","<id>",   args: ["secret","get",    │
        │                     "--session","<token>",             "--","<uuid>"]      │
        │                     "--nointeraction"]                                     │
        │                    │                                   │                  │
        │              wait_with_timeout()                wait_with_timeout()       │
        │              parse JSON stdout                  parse JSON stdout          │
        │              extract selector field             extract .value field       │
        │              Zeroizing::new(value)              Zeroizing::new(value)     │
        └─────────────────────────────────────────────────────────────────────────-┘
                                   │
                             Zeroizing<String>
```

### Recommended Project Structure

No new files. All changes are isolated to `crates/nono/src/keystore.rs`.

Within `keystore.rs`, follow the existing per-scheme block structure:

```
// Constants (lines ~44-133)
const BW_URI_PREFIX: &str = "bw://";
const BW_ITEM_SEGMENT: &str = "item";
const BW_SECRET_SEGMENT: &str = "secret";

// Validation (after validate_keyring_uri / before load_from_env)
pub fn validate_bw_uri(uri: &str) -> Result<()>   // structural + charset validation
pub fn is_bw_uri(uri: &str) -> bool
pub fn redact_bw_uri(uri: &str) -> String          // bw://item/<id>/<redacted> pattern

// Loading (after load_from_keyring_uri)
fn load_from_bw(uri: &str) -> Result<Zeroizing<String>>      // item backend
fn load_from_bws(uri: &str) -> Result<Zeroizing<String>>     // secret backend

// Error classification (after classify_op_error)
fn classify_bw_error(stderr: &str, uri: &str) -> NonoError
fn classify_bws_error(stderr: &str, uri: &str) -> NonoError

// Dispatch (load_secret_by_ref, insert before is_keyring_uri branch)
} else if is_bw_uri(credential_ref) {
    load_from_bw_or_bws(credential_ref)    // fan-out by first segment
```

### Pattern 1: URI Validation (mirrors validate_op_uri)

**What:** Strip `bw://` prefix, reject FORBIDDEN_URI_CHARS and `?`/`#`, split on `/`, validate segment count and content.
**When to use:** Called at the top of `load_from_bw`/`load_from_bws` before any subprocess spawn.

```rust
// Source: keystore.rs validate_op_uri pattern (lines 230-272)
pub fn validate_bw_uri(uri: &str) -> Result<()> {
    let path = uri.strip_prefix(BW_URI_PREFIX).ok_or_else(|| {
        NonoError::ConfigParse(format!(
            "credential reference '{}' does not start with '{}'",
            uri, BW_URI_PREFIX
        ))
    })?;

    // Reject shell metacharacters (reuse FORBIDDEN_URI_CHARS)
    if let Some(bad) = path.chars().find(|c| FORBIDDEN_URI_CHARS.contains(c)) {
        return Err(NonoError::ConfigParse(format!(
            "Bitwarden URI contains forbidden character {:?}: {}", bad, uri
        )));
    }

    // Reject query strings and fragments
    if path.contains('?') || path.contains('#') {
        return Err(NonoError::ConfigParse(format!(
            "Bitwarden URI must not contain query strings or fragments: {}", uri
        )));
    }

    let segments: Vec<&str> = path.split('/').collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err(NonoError::ConfigParse(format!(
            "Bitwarden URI has empty path segment: {}", uri
        )));
    }

    match segments.first().map(|s| *s) {
        Some(BW_ITEM_SEGMENT) => validate_bw_item_uri_segments(&segments, uri),
        Some(BW_SECRET_SEGMENT) => validate_bws_uri_segments(&segments, uri),
        _ => Err(NonoError::ConfigParse(format!(
            "Bitwarden URI must start with 'item/' or 'secret/': {}", uri
        ))),
    }
}

// ID/UUID charset: hex digits + hyphens only (rejects all injection chars)
fn is_valid_bw_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}
```

### Pattern 2: `bw` CLI invocation (item backend)

**What:** Spawn `bw get item <id> --session <token> --nointeraction`, capture stdout as JSON, parse with `serde_json`, select field.
**When to use:** `bw://item/<id>/<selector>` URIs.

```rust
// Source: keystore.rs load_from_op pattern (lines 977-1028)
fn load_from_bw(uri: &str) -> Result<Zeroizing<String>> {
    validate_bw_uri(uri)?;
    let (item_id, selector) = parse_bw_item_uri(uri)?;  // extracts validated id + selector

    let session = std::env::var("BW_SESSION").map_err(|_| {
        NonoError::KeystoreAccess(
            "bw:// requires BW_SESSION; run `bw unlock --raw` and export it".to_string()
        )
    })?;
    if session.is_empty() {
        return Err(NonoError::KeystoreAccess(
            "BW_SESSION is set but empty; run `bw unlock --raw` and export it".to_string()
        ));
    }

    let mut child = Command::new("bw")
        .args(["get", "item", "--", item_id, "--session", &session, "--nointeraction"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                NonoError::KeystoreAccess(
                    "Bitwarden CLI ('bw') not found. \
                     Install from https://bitwarden.com/help/cli/".to_string()
                )
            } else {
                NonoError::KeystoreAccess(format!("Could not start Bitwarden CLI: {}", e))
            }
        })?;

    let output = wait_with_timeout(&mut child, SECRET_MANAGER_TIMEOUT,
        "Bitwarden CLI", "Is the vault locked? Check BW_SESSION.")
        .inspect_err(|_| { let _ = child.kill(); let _ = child.wait(); })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(classify_bw_error(&stderr, uri));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| NonoError::KeystoreAccess(format!(
            "Bitwarden CLI returned invalid JSON for '{}': {}", redact_bw_uri(uri), e
        )))?;

    extract_bw_field(&json, &selector, uri)
}
```

### Pattern 3: `bws` CLI invocation (secret backend)

**What:** Spawn `bws secret get -- <uuid>`, using `BWS_ACCESS_TOKEN` from environment (bws reads it automatically), capture stdout as JSON, extract `.value`.
**When to use:** `bw://secret/<uuid>` URIs.

```rust
fn load_from_bws(uri: &str) -> Result<Zeroizing<String>> {
    validate_bw_uri(uri)?;
    let secret_uuid = parse_bws_uri(uri)?;

    // bws reads BWS_ACCESS_TOKEN from the environment automatically.
    // We validate it's present first for a clean actionable error.
    if std::env::var("BWS_ACCESS_TOKEN").map_or(true, |v| v.is_empty()) {
        return Err(NonoError::KeystoreAccess(
            "bw://secret/ requires BWS_ACCESS_TOKEN; set it to your \
             Bitwarden Secrets Manager service-account access token".to_string()
        ));
    }

    let mut child = Command::new("bws")
        .args(["secret", "get", "--", secret_uuid])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                NonoError::KeystoreAccess(
                    "Bitwarden Secrets Manager CLI ('bws') not found. \
                     Install from https://bitwarden.com/help/secrets-manager-cli/".to_string()
                )
            } else {
                NonoError::KeystoreAccess(format!("Could not start bws CLI: {}", e))
            }
        })?;

    // ... wait_with_timeout, success check, JSON parse, extract .value
}
```

### Pattern 4: Field extraction from `bw get item` JSON

**What:** Parse the JSON object from `bw get item` stdout and select the requested field without string splicing.
**Key insight:** TOTP selector must use `bw get totp <id>` (separate call) — `login.totp` in `bw get item` contains the **TOTP seed** (otpauth:// URI), not the current time-based code.

```rust
fn extract_bw_field(
    json: &serde_json::Value,
    selector: &BwSelector,
    uri: &str,
) -> Result<Zeroizing<String>> {
    let redacted = redact_bw_uri(uri);
    match selector {
        BwSelector::Password => json_str_field(json, &["login", "password"], &redacted),
        BwSelector::Username => json_str_field(json, &["login", "username"], &redacted),
        BwSelector::Notes    => json_str_field(json, &["notes"], &redacted),
        BwSelector::Totp     => {
            // login.totp contains the otpauth:// seed, NOT the computed code.
            // The computed code must come from `bw get totp` (a separate subprocess call).
            // See: D-07; this branch spawns the dedicated subcommand.
            load_totp_via_bw_get_totp(item_id, uri)
        }
        BwSelector::CustomField(name) => {
            let fields = json["fields"].as_array().ok_or_else(|| {
                NonoError::SecretNotFound(format!(
                    "Bitwarden item '{}' has no custom fields", redacted
                ))
            })?;
            fields.iter()
                .find(|f| f["name"].as_str() == Some(name))
                .and_then(|f| f["value"].as_str())
                .map(|v| Zeroizing::new(v.to_string()))
                .ok_or_else(|| NonoError::SecretNotFound(format!(
                    "Custom field '{}' not found in Bitwarden item '{}'", name, redacted
                )))
        }
    }
}
```

### Anti-Patterns to Avoid

- **Using `bw get password <id>` for scripting:** Only returns password; D-07 requires all fields from one call. Use `bw get item <id>` and parse JSON.
- **Extracting `login.totp` from `bw get item` JSON for TOTP selector:** That field contains the TOTP seed/otpauth URI. The live code requires `bw get totp <id>`. [ASSUMED — confirmed by community sources; no official Bitwarden doc explicitly documents this distinction, but multiple scripting blogs and the drumm.sh series confirm it]
- **Shell-spawning the CLIs:** Always use `Command::new` (not `sh -c`), passing item ID as a separate `--` argument. Mirroring `load_from_op`.
- **Passing `BW_SESSION` on the command line:** The session token must NOT appear in process argument lists (visible in `ps`). Pass it to `bw` via `--session` flag only when the token is already obtained from the env (still visible in `/proc/<pid>/cmdline`). The safer pattern is to let `bw` read `BW_SESSION` from the environment implicitly — omit `--session` and pass only the item ID. This avoids placing the token in the argv. [ASSUMED — the `--session` flag exists but env-only is safer for audit/ps visibility]
- **Asserting a specific exit code other than 0/non-0:** The `bw` CLI does not have documented unique exit codes per error type (feature request bitwarden/cli#29 was never resolved before the old CLI repo was archived). Rely on stderr substring matching for classification.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON parsing of `bw get item` output | Custom string parser / regex | `serde_json` (workspace dep) | Shell-special chars in item values; embedded quotes; nested objects require proper JSON |
| Timeout for CLI subprocesses | `std::thread::sleep` loop | `wait_with_timeout()` already in keystore.rs | Function already exists at line 1283; handles polling loop + timeout |
| Shell metacharacter injection | Ad-hoc character filtering | Reuse `FORBIDDEN_URI_CHARS` + `is_valid_bw_id()` | Defense-in-depth; mirrors `validate_op_uri` |
| Redacted logging of bw:// URIs | Logging raw URI | `redact_bw_uri()` (new, mirrors `redact_op_uri`) | ID segment is not a secret but field/value path should be masked |
| Subprocess hygiene | `sh -c` invocation | `Command::new("bw").args([...])` | No shell = no argument injection via special chars even if validation misses something |

**Key insight:** `wait_with_timeout` is already implemented and reusable — zero new timeout infrastructure needed.

---

## `bw` CLI Contract

### Command: `bw get item <id>`

**Full invocation for scripting:**
```
bw get item --nointeraction -- <id>
```
`BW_SESSION` should be present in the environment; the `--session` flag is an alternative but places the token in argv.

**JSON output shape** (type=1 login item): [CITED: bitwarden.com/help/condition-bitwarden-import/, community examples]
```json
{
  "object": "item",
  "id": "<uuid>",
  "organizationId": "<uuid or null>",
  "folderId": "<uuid or null>",
  "type": 1,
  "reprompt": 0,
  "name": "string",
  "notes": "string or null",
  "favorite": false,
  "login": {
    "uris": [{"match": null, "uri": "string"}],
    "username": "string",
    "password": "string",
    "totp": "otpauth://totp/...?secret=...   ← seed, NOT current code",
    "passwordRevisionDate": "ISO-8601 or null"
  },
  "fields": [
    {
      "name": "field-name",
      "value": "field-value",
      "type": 0
    }
  ],
  "collectionIds": [],
  "revisionDate": "ISO-8601"
}
```

**fields[].type integer meanings** [CITED: bitwarden.com/help/custom-fields/]:
- `0` = Text (visible freeform)
- `1` = Hidden (masked freeform — still returned as plaintext in JSON)
- `2` = Boolean (value is "true" or "false")
- `3` = Linked (links to username or password field)

**All field types return a string `value`** — no special handling needed by type.

### Command: `bw get totp <id>`

Returns the **current computed TOTP code** (a 6–8 digit string) to stdout, not the otpauth URI. [ASSUMED — confirmed by community scripting resources; drumm.sh blog explicitly states "the JSON data returned by bw list doesn't contain the current TOTP token — use `bw get totp <ID>`"]

```
bw get totp --nointeraction -- <id>
```

Output: raw digits, one line, trailing newline. Wrap same as `op read` output.

### Error behavior

**Vault locked / session missing:** [MEDIUM confidence — community/issue sources]
- stdout: `{"success":false,"message":"Vault is locked."}` or plain `"Session key is invalid."`  
- stderr: may contain `"Vault is locked."` or `"You are not logged in."`
- exit: 1

**Item not found:**
- stderr contains: item-not-found language (exact text varies)
- exit: 1

**Classification approach:** Mirror `classify_op_error` — check for locked/auth keywords and not-found keywords in stderr; fall through to generic.

```rust
fn classify_bw_error(stderr: &str, uri: &str) -> NonoError {
    let redacted = redact_bw_uri(uri);
    let s = stderr.trim();
    if stderr.contains("Vault is locked") || stderr.contains("not logged in")
        || stderr.contains("Session key is invalid") || stderr.contains("Invalid master password")
    {
        NonoError::KeystoreAccess(format!(
            "Bitwarden vault is locked for '{}'. \
             Run `bw unlock --raw` and export BW_SESSION. Detail: {}",
            redacted, s
        ))
    } else if stderr.contains("not found") || stderr.contains("No items")
        || stderr.contains("invalid UUID")
    {
        NonoError::SecretNotFound(format!(
            "Bitwarden item not found: '{}'. Detail: {}", redacted, s
        ))
    } else {
        NonoError::KeystoreAccess(format!(
            "Bitwarden CLI failed for '{}': {}", redacted, s
        ))
    }
}
```

---

## `bws` CLI Contract

### Command: `bws secret get <uuid>`

**Full invocation:**
```
bws secret get -- <uuid>
```
`BWS_ACCESS_TOKEN` is read from the environment automatically by `bws` (env-var is the canonical auth mechanism). [CITED: bitwarden.com/help/secrets-manager-cli/]

**JSON output shape** (default `--output json`): [CITED: bitwarden.com/help/secrets-manager-cli/]
```json
{
  "object": "secret",
  "id": "<uuid>",
  "organizationId": "<uuid>",
  "projectId": "<uuid or null>",
  "key": "SECRET_NAME",
  "value": "the-actual-secret-value",
  "note": "string",
  "creationDate": "ISO-8601",
  "revisionDate": "ISO-8601"
}
```

**Extraction:** `json["value"].as_str()` — a single `String`, no field selector.

**Error behavior** [MEDIUM confidence — deepwiki.com/bitwarden/sdk-sm docs]:
- General error exit code: `1`
- Missing access token: stderr/stdout contains `"Missing access token"` or similar
- Invalid token: auth error message
- Secret not found: stderr message referencing the UUID

```rust
fn classify_bws_error(stderr: &str, uri: &str) -> NonoError {
    let redacted = redact_bw_uri(uri);
    let s = stderr.trim();
    if stderr.contains("Missing access token") || stderr.contains("access token")
        || stderr.contains("Unauthorized") || stderr.contains("authentication")
    {
        NonoError::KeystoreAccess(format!(
            "bws authentication failed for '{}'. \
             Set BWS_ACCESS_TOKEN to your service-account access token. Detail: {}",
            redacted, s
        ))
    } else if stderr.contains("not found") || stderr.contains("404") {
        NonoError::SecretNotFound(format!(
            "Bitwarden secret not found: '{}'. Detail: {}", redacted, s
        ))
    } else {
        NonoError::KeystoreAccess(format!(
            "Bitwarden Secrets Manager CLI failed for '{}': {}", redacted, s
        ))
    }
}
```

---

## URI Validation Design

### ID/UUID Charset

Both `<id>` (bw item id) and `<uuid>` (bws secret UUID) must pass an injection-safe charset check. UUIDs are lowercase hex + hyphens. Item IDs may also be non-UUID opaque strings (short alphanumeric IDs) — broaden slightly.

```rust
/// Accept only characters safe for passing as CLI arguments:
/// lowercase hex, uppercase hex, digits, and hyphens.
/// This covers all UUID forms and Bitwarden's compact item IDs.
fn is_valid_bw_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64   // reasonable upper bound; UUIDs are 36 chars
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}
```

This is stricter than `FORBIDDEN_URI_CHARS` rejection (which allows any char except the listed metacharacters). For IDs we use an allowlist — mirrors D-04.

### Custom Field Name Charset

For `bw://item/<id>/field/<name>`, the `<name>` is validated to exclude shell metacharacters. Use `FORBIDDEN_URI_CHARS` check (already defined). No additional length limit needed beyond the URI max (reuse `KEYRING_URI_MAX_LEN` = 1024 for the whole `bw://` URI).

### `bw://secret/<uuid>` shape

```
bw://secret/<uuid>   ← exactly 2 segments after "bw://"
bw://secret/<uuid>/anything  ← D-06: validation error ("field selectors not supported for bws secrets")
```

### Segment count rules

| URI form | Segments after `bw://` | Valid count |
|----------|------------------------|-------------|
| `bw://item/<id>/password` | 3 | exactly 3 |
| `bw://item/<id>/field/<name>` | 4 | exactly 4 |
| `bw://secret/<uuid>` | 2 | exactly 2 |
| `bw://item/<id>` | 2 | invalid — missing selector |
| `bw://secret/<uuid>/anything` | 3 | invalid — D-06 |

---

## Common Pitfalls

### Pitfall 1: TOTP — seed vs. computed code confusion
**What goes wrong:** Implementor extracts `json["login"]["totp"]` from `bw get item` output and returns it as the TOTP value. This contains the TOTP seed (e.g. `otpauth://totp/...?secret=JBSWY3DPEHPK3PXP`), not the current 6-digit code.
**Why it happens:** The field name `totp` implies the computed value, but bw stores the *configuration* there, not the live token.
**How to avoid:** For `selector=totp`, issue a **second** subprocess call: `bw get totp --nointeraction -- <id>`. The stdout is the raw numeric code. [ASSUMED — community consensus; no official doc explicitly documents both behaviors in one place]
**Warning signs:** Returned value starts with `otpauth://` or is longer than 8 digits.

### Pitfall 2: `bw` session token in argv
**What goes wrong:** Passing `--session <BW_SESSION>` places the session token in process argv, visible to other processes via `ps` or `/proc/<pid>/cmdline`.
**Why it happens:** The `--session` flag exists and works; it's the obvious approach.
**How to avoid:** Let `bw` read `BW_SESSION` from the environment implicitly (don't pass `--session`). The token is still in the child process environment (also somewhat visible) but argv exposure is reduced.
**Warning signs:** `ps aux | grep bw` shows session token in argument list.

### Pitfall 3: `bws` does not use `BW_SESSION`
**What goes wrong:** Implementor checks for `BW_SESSION` for both backends; `bws` silently fails auth.
**Why it happens:** The env var name difference (`BW_SESSION` vs `BWS_ACCESS_TOKEN`) is easy to confuse.
**How to avoid:** Pre-flight check is backend-specific: `BW_SESSION` for `item/` paths, `BWS_ACCESS_TOKEN` for `secret/` paths.
**Warning signs:** `bws` exits non-zero with "Missing access token" in stderr.

### Pitfall 4: Non-zeroized intermediate Vec<u8>
**What goes wrong:** `output.stdout` (a `Vec<u8>`) holds the full JSON (including secrets) and is not zeroized before being dropped.
**Why it happens:** `std::process::Output` has no zeroize support.
**How to avoid:** Document this as a known limitation (same class as `op://` — see keystore.rs line ~200 comment). Do not regress beyond what `op://` already does: parse quickly, wrap in `Zeroizing<String>`, drop the stdout buffer.
**Warning signs:** No test for this — it's an acknowledged limitation, not a code bug.

### Pitfall 5: Empty `BW_SESSION` silently accepted
**What goes wrong:** Operator exports `BW_SESSION=""` (common mistake after failed unlock); `bw` is invoked but returns "Vault is locked."
**Why it happens:** `std::env::var` succeeds for empty strings.
**How to avoid:** Validate non-empty before spawning subprocess (mirror `load_from_env`'s empty-check pattern).

### Pitfall 6: `bw://` in `build_mappings_from_list` without `=VAR_NAME`
**What goes wrong:** An operator puts `bw://item/abc123/password` in `--env-credential` without a `=MY_VAR` suffix; uppercasing the raw URI produces a garbage env var name.
**Why it happens:** The `build_mappings_from_list` function already handles this correctly for `op://` — the same guard must be added for `bw://`.
**How to avoid:** Add a `bw://`-prefix branch in `build_mappings_from_list` that requires explicit `=VAR_NAME`, mirroring the `op://` branch (lines ~1433-1460). Return a clear error: `"Bitwarden credential requires an explicit variable name. Use format: bw://item/<id>/password=MY_VAR"`.

---

## Code Examples

### URI parsing + dispatch (complete flow sketch)

```rust
// Source: keystore.rs load_secret_by_ref (line 204) — insert before is_keyring_uri branch
} else if is_bw_uri(credential_ref) {
    load_from_bw_dispatch(credential_ref)
```

```rust
// Source: mirrors load_from_op pattern (lines 977-1028)
fn load_from_bw_dispatch(uri: &str) -> Result<Zeroizing<String>> {
    validate_bw_uri(uri)?;
    // First segment after "bw://" determines backend
    let path = uri.strip_prefix(BW_URI_PREFIX)
        .ok_or_else(|| NonoError::ConfigParse(format!("invalid bw:// URI: {}", uri)))?;
    let first_segment = path.split('/').next().unwrap_or("");  // safe: validated non-empty
    match first_segment {
        BW_ITEM_SEGMENT   => load_from_bw(uri),
        BW_SECRET_SEGMENT => load_from_bws(uri),
        other => Err(NonoError::ConfigParse(format!(
            "Unknown bw:// backend segment '{}': expected 'item' or 'secret'", other
        ))),
    }
}
```

Note: the `unwrap_or("")` above is fine only because `validate_bw_uri` already rejected empty segments. An alternative without any `unwrap_or` can use `.next().ok_or_else(...)`.

### serde_json field extraction helper

```rust
fn json_str_field(
    json: &serde_json::Value,
    path: &[&str],
    redacted_uri: &str,
) -> Result<Zeroizing<String>> {
    let mut node = json;
    for key in path {
        node = node.get(key).ok_or_else(|| {
            NonoError::SecretNotFound(format!(
                "Bitwarden item '{}' missing field '{}'", redacted_uri, key
            ))
        })?;
    }
    node.as_str()
        .filter(|s| !s.is_empty())
        .map(|s| Zeroizing::new(s.to_string()))
        .ok_or_else(|| NonoError::SecretNotFound(format!(
            "Bitwarden field is empty or not a string for '{}'", redacted_uri
        )))
}
```

### Redaction pattern (mirrors redact_op_uri)

```rust
// bw://item/<id>/<selector>  →  bw://item/<id>/<redacted>
// bw://secret/<uuid>         →  bw://secret/<redacted>
pub fn redact_bw_uri(uri: &str) -> String {
    if let Some(path) = uri.strip_prefix(BW_URI_PREFIX) {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        match parts.as_slice() {
            [BW_ITEM_SEGMENT, id, _] => {
                return format!("bw://item/{}/<redacted>", id);
            }
            [BW_SECRET_SEGMENT, _] => {
                return format!("bw://secret/<redacted>");
            }
            _ => {}
        }
    }
    "bw://***".to_string()
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `bw get password <id>` (single-field shortcut) | `bw get item <id>` + JSON parse | One call covers all field types; D-07 requires this |
| `bw get totp` for seed | `bw get totp` returns live code | Confirmed: this is the correct command for computed TOTP |
| `bws` 0.x CLI (older releases used different commands) | `bws secret get <uuid>` (current) | Current `bws` API is stable per 2023+ docs |

**No deprecated patterns to warn about** within the scope of this phase.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `bw get totp <id>` returns the **computed 6-digit TOTP code**, not the seed | TOTP pitfall, Code Examples | If wrong: `bw://item/<id>/totp` returns an otpauth seed instead of current code; operator would need to use `bw://item/<id>/field/<name>` to get the seed if that's what they want. The current design would still be safe — just returns different data |
| A2 | Passing `--session` places session token in argv (visible in ps) | Pitfall 2 | If wrong: no security regression — env-only approach is still equally secure or better |
| A3 | `bws` reads `BWS_ACCESS_TOKEN` from environment automatically without `--access-token` flag | bws CLI Contract | If wrong: bws would return "missing access token" even with env set; fix: add `--access-token` flag to bws invocation |
| A4 | `bw` CLI exit code is always non-0 on any error (locked/not-found/missing-session) | Error behavior | If wrong: success check `output.status.success()` would not catch all errors; would need to also parse stdout for `{"success":false,...}` JSON |
| A5 | `bw` custom fields of all types (0=text, 1=hidden, 2=boolean, 3=linked) return a string `.value` in the item JSON | Field extraction | If wrong: boolean fields return JSON boolean rather than string "true"/"false"; fix: coerce with `value.to_string()` |
| A6 | TOTP dispatch via a second `bw get totp` subprocess is acceptable latency for the `totp` selector | Architecture | If wrong: could instead return `login.totp` seed and document it — operators using TOTP integrations would not get a usable token |

---

## Open Questions

1. **Should `--session` be passed explicitly or omitted?**
   - What we know: `--session` flag exists; `BW_SESSION` env var also works natively.
   - What's unclear: Whether omitting `--session` and relying on the env var is sufficient across all `bw` versions.
   - Recommendation: Omit `--session`; pre-validate `BW_SESSION` is non-empty; rely on `bw`'s native env var support. This avoids session token in argv.

2. **`bw get totp` vs `login.totp` for TOTP selector**
   - What we know: Community consensus is that `bw get totp` returns the computed code. The CONTEXT.md D-07 says "field specific subcommands" are avoided, but TOTP is the one exception because the item JSON field holds the seed, not the code.
   - What's unclear: Whether the `bw get item` output ever includes a pre-computed TOTP code (it doesn't per current understanding).
   - Recommendation: Use `bw get totp <id>` for the `totp` selector. Document this explicitly as the one selector that requires a second subprocess call.

3. **Does `bw get item` require `--nointeraction` or is it implied when `BW_SESSION` is set?**
   - What we know: `--nointeraction` is a documented global flag meaning "Do not prompt for interactive user input."
   - What's unclear: Whether omitting it could cause interactive prompts on some vault states.
   - Recommendation: Always pass `--nointeraction`; it is defensive, costs nothing.

---

## Environment Availability

The `bw` and `bws` CLIs are operator-installed tools. This phase does NOT require them at build time or test time.

| Dependency | Required By | Available at build | Fallback |
|------------|------------|-------------------|----------|
| `bw` CLI | `bw://item/...` backend at runtime | Not required | Fail-closed error if missing at runtime |
| `bws` CLI | `bw://secret/...` backend at runtime | Not required | Fail-closed error if missing at runtime |
| `serde_json` | JSON parsing in `load_from_bw` | Yes — workspace dep | N/A (always present) |

**Missing dependencies with no fallback (runtime):** `bw` and `bws` are optional; absence is a fail-closed `KeystoreAccess` error (same as `op` CLI missing). This is correct behavior — not a build blocker.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner |
| Config file | none (workspace `[lints]` only) |
| Quick run command | `cargo test -p nono keystore` |
| Full suite command | `cargo test -p nono` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-CRED-01 | `validate_bw_uri` accepts valid URIs | unit | `cargo test -p nono keystore::tests::test_validate_bw_uri` | ❌ Wave 0 |
| REQ-CRED-01 | `validate_bw_uri` rejects forbidden chars | unit | `cargo test -p nono keystore::tests::test_validate_bw_uri_forbidden_char` | ❌ Wave 0 |
| REQ-CRED-01 | `validate_bw_uri` rejects query strings | unit | `cargo test -p nono keystore::tests::test_validate_bw_uri_no_query` | ❌ Wave 0 |
| REQ-CRED-01 | `validate_bw_uri` rejects `secret/<uuid>/field` (D-06) | unit | `cargo test -p nono keystore::tests::test_validate_bw_uri_secret_no_field` | ❌ Wave 0 |
| REQ-CRED-01 | `is_bw_uri` positive/negative | unit | `cargo test -p nono keystore::tests::test_is_bw_uri` | ❌ Wave 0 |
| REQ-CRED-01 | `redact_bw_uri` item + secret forms | unit | `cargo test -p nono keystore::tests::test_redact_bw_uri` | ❌ Wave 0 |
| REQ-CRED-01 | `load_secret_by_ref` dispatches `bw://` | unit | `cargo test -p nono keystore::tests::test_load_secret_by_ref_dispatches_bw` | ❌ Wave 0 |
| REQ-CRED-01 | `load_from_bw` returns `KeystoreAccess` when `BW_SESSION` missing | unit | `cargo test -p nono keystore::tests::test_load_from_bw_no_session` | ❌ Wave 0 |
| REQ-CRED-01 | `load_from_bws` returns `KeystoreAccess` when `BWS_ACCESS_TOKEN` missing | unit | `cargo test -p nono keystore::tests::test_load_from_bws_no_token` | ❌ Wave 0 |
| REQ-CRED-01 | `load_from_bw` returns `KeystoreAccess` when `bw` binary not found | unit | `cargo test -p nono keystore::tests::test_load_from_bw_cli_not_found` | ❌ Wave 0 |
| REQ-CRED-01 | `extract_bw_field` extracts password/username/notes from JSON | unit | `cargo test -p nono keystore::tests::test_extract_bw_field_*` | ❌ Wave 0 |
| REQ-CRED-01 | `extract_bw_field` extracts custom field by name | unit | `cargo test -p nono keystore::tests::test_extract_bw_field_custom` | ❌ Wave 0 |
| REQ-CRED-01 | `build_mappings_from_list` rejects bare `bw://` URI | unit | `cargo test -p nono keystore::tests::test_build_mappings_bw_uri_without_var` | ❌ Wave 0 |
| REQ-CRED-01 | Secret values cleared (Zeroizing) — structural | unit (compile-time) | `cargo test -p nono` (type system guarantee) | ✅ always |
| REQ-CRED-01 | Live `bw get item` fetch | manual | None — requires live vault | manual only |

### Sampling Rate

- **Per task commit:** `cargo test -p nono keystore`
- **Per wave merge:** `cargo test -p nono`
- **Phase gate:** Full `make test` green before `/gsd:verify-work`

### Wave 0 Gaps

All test functions listed above are ❌ new. The planner must schedule a Wave 0 task that adds:
- Unit tests for all `validate_bw_uri`, `is_bw_uri`, `redact_bw_uri` functions
- Unit tests for `load_from_bw` / `load_from_bws` error paths (missing session/token, missing CLI binary)
- Unit tests for `extract_bw_field` using static JSON fixtures
- Unit test for `build_mappings_from_list` `bw://` rejection

These all run without any live Bitwarden account. Test fixtures use static JSON strings matching the documented schema.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Auth is pre-performed by operator; nono consumes tokens |
| V3 Session Management | No | Session token lifecycle is external to nono |
| V4 Access Control | No | Access is capability-gated at the subprocess level |
| V5 Input Validation | **Yes** | `validate_bw_uri` — `FORBIDDEN_URI_CHARS` + `is_valid_bw_id` allowlist |
| V6 Cryptography | No | Zeroize for secrets; no crypto operations in nono |
| V7 Error Handling | **Yes** | Fail-closed; `classify_bw_error` / `classify_bws_error` |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| URI injection via item ID | Tampering | `is_valid_bw_id` allowlist (alphanumeric + hyphen only); `FORBIDDEN_URI_CHARS` already rejects metacharacters |
| Session token exposure in argv | Information Disclosure | Rely on env var rather than `--session` flag; pre-validate non-empty |
| Shell injection via custom field name | Tampering | `FORBIDDEN_URI_CHARS` check on `<name>` segment; `Command::new` (no shell) |
| Secret logged at debug level | Information Disclosure | `redact_bw_uri` in all tracing calls; never log field value |
| `bws` token leaked to child env | Information Disclosure | Subprocess inherits parent env; documented known limitation (same class as `op://`) |
| Empty `BW_SESSION` bypasses pre-flight, vault locked error | Spoofing | Explicit empty-string check before spawn (mirrors `load_from_env` pattern) |

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase 57 |
|-----------|-------------------|
| No `.unwrap()` / `.expect()` anywhere | All JSON field accesses use `ok_or_else`; `strip_prefix` results are `?`-propagated |
| `clippy::unwrap_used = "deny"` (workspace manifest) | Enforced at compile time; use `ok_or_else`, `map_err`, `?` throughout |
| `Zeroizing<String>` for all secret material | All return values from `load_from_bw` / `load_from_bws` are `Zeroizing<String>` |
| Fail secure — never silently degrade | Missing token / locked vault / missing CLI → `Err(NonoError::KeystoreAccess(...))`, never `Ok(empty)` |
| `must_use` on critical Results | Tag `validate_bw_uri`, `is_bw_uri`, `load_from_bw_dispatch` with `#[must_use]` |
| No `#[allow(dead_code)]` | All new public fns must have tests or be used by `load_secret_by_ref` |
| Path component comparison | N/A — no filesystem paths in this phase |
| DCO sign-off on all commits | All commits require `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` |
| Cross-target clippy for cfg-gated Unix code | `keystore.rs` has no cfg-gated Unix blocks; the new `bw://` code must remain platform-agnostic. If any cfg block is added, cross-target clippy MUST run |
| GSD workflow — no direct edits outside GSD | Only via `/gsd:execute-phase 57` |

---

## Sources

### Primary (HIGH confidence)
- `crates/nono/src/keystore.rs` — read in session; is/validate/load pattern for `op://` is the definitive implementation analog
- `crates/nono/src/error.rs` — read in session; `KeystoreAccess` and `SecretNotFound` are the correct NonoError variants
- `crates/nono/Cargo.toml` + `Cargo.toml` — read in session; `serde_json = "1.0.149"` confirmed as direct dep of `crates/nono`
- `.planning/phases/57-bitwarden-credential-source/57-CONTEXT.md` — locked decisions D-01..D-08

### Secondary (MEDIUM confidence)
- [Bitwarden Secrets Manager CLI docs](https://bitwarden.com/help/secrets-manager-cli/) — `bws secret get` command, JSON output shape with `.value` field, `BWS_ACCESS_TOKEN` env var, `--output json` default confirmed
- [Bitwarden Password Manager CLI docs](https://bitwarden.com/help/cli/) — `bw get item`, `--nointeraction`, `--session` flags documented; JSON schema not exhaustively shown but field names confirmed
- [Bitwarden import format docs](https://bitwarden.com/help/condition-bitwarden-import/) — complete login item JSON schema including `login.{username,password,totp}`, `fields[].{name,value,type}`, `notes`
- [Bitwarden custom fields docs](https://bitwarden.com/help/custom-fields/) — field type integers 0=text, 1=hidden, 2=boolean, 3=linked confirmed
- [bitwarden/sdk-sm CLI commands (deepwiki)](https://deepwiki.com/bitwarden/sdk-sm/5.1-cli-commands-and-usage) — `bws` exit code 1 for general errors confirmed

### Tertiary (LOW confidence / ASSUMED)
- [bitwarden/cli issue #29](https://github.com/bitwarden/cli/issues/29) — `bw` uses exit code 1 for all errors (feature request for unique codes was never resolved; old CLI repo archived)
- [bitwarden/cli issue #188](https://github.com/bitwarden/cli/issues/188) — stdout/stderr behavior for locked vault (`"Vault is locked."` / `"You are not logged in."` on stderr with exit 1)
- [drumm.sh blog](https://www.drumm.sh/blog/2021/09/17/more-bw-cli/) — `bw get totp` vs `login.totp` distinction; community scripting practice
- Community consensus on `--session` argv exposure risk (multiple scripting discussions)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — workspace deps confirmed; no new packages
- Architecture: HIGH — op:// pattern is fully understood; bw:// slots in directly
- bw CLI JSON contract: MEDIUM — schema confirmed via import docs and community examples; exit behavior from issue trackers
- bws CLI contract: MEDIUM — official docs show command + JSON shape; error messages from deepwiki/community
- TOTP seed vs. computed code: LOW/ASSUMED — community consensus; no single authoritative Bitwarden doc page explicitly documents both behaviors side by side

**Research date:** 2026-06-05
**Valid until:** 2026-07-05 (CLI schemas are stable; `bws` may have minor API changes in new releases)
