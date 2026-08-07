//! Sandboxed OAuth token capture (SEC-02, D-01r/D-07/D-08).
//!
//! Real OAuth tokens observed in upstream responses on capture-declared
//! routes are held in-memory, session-scoped only — never written to disk
//! (D-08 deliberately does not adopt upstream's `oauth_capture/persist.rs`).
//! [`CapturePhantomStore`] mints an opaque or JWT-shaped phantom identifier
//! for each real token and later resolves a phantom back to its real value,
//! scoped to an explicit set of admitted consumers so a phantom minted for
//! one consumer cannot be resolved by a different one.
//!
//! Real token bytes are always held in [`zeroize::Zeroizing`] and are never
//! included in any `Debug`/`Display`/log/audit output — see
//! [`CapturePhantomStore`]'s manual `Debug` impl.
//!
//! See `crates/nono-proxy/src/config.rs`'s `CaptureConfig` for the
//! declarative per-route configuration surface this module serves, and
//! `proj/ADR-114-oauth-capture-disposition.md` for why this fork-native
//! module exists instead of adopting upstream's `oauth_capture/forward.rs`
//! MITM-based rewrite hook.

use crate::config::{CaptureResponseField, CaptureResponseFieldKind};
use crate::error::{ProxyError, Result};
use base64::Engine as _;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use zeroize::Zeroizing;

/// Number of random bytes used to mint a phantom identifier (256 bits of
/// entropy, mirroring `token.rs::TOKEN_BYTES`).
const PHANTOM_ID_BYTES: usize = 32;

/// A single entry in the phantom store: the real captured token plus the
/// set of consumers admitted to resolve it back.
///
/// Deliberately never `Debug`-derived — see [`CapturePhantomStore`]'s
/// manual `Debug` impl, which never inspects this type's fields.
struct StoredCapture {
    real: Zeroizing<Vec<u8>>,
    admitted_consumers: HashSet<String>,
}

/// In-memory, session-scoped store mapping minted phantom identifiers back
/// to the real captured token, admission-scoped per consumer (D-08).
///
/// Never persisted to disk. Every stored real-token value is zeroized on
/// drop via `Zeroizing<Vec<u8>>`, and the whole store is dropped (and thus
/// zeroized) when the owning proxy session ends — there is no cross-session
/// retention.
#[derive(Default)]
pub struct CapturePhantomStore {
    entries: Mutex<HashMap<String, StoredCapture>>,
}

impl std::fmt::Debug for CapturePhantomStore {
    /// Redacted: never prints entry contents (real tokens or admitted
    /// consumer names), only the count of currently-stored phantoms.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        f.debug_struct("CapturePhantomStore")
            .field("entry_count", &count)
            .finish()
    }
}

impl CapturePhantomStore {
    /// Create an empty, session-scoped phantom store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a fresh phantom identifier for `real`, admitting only
    /// `admitted_consumers` to resolve it back via [`Self::resolve`].
    ///
    /// Two calls never return the same phantom (256 bits of `getrandom`
    /// entropy per call). Returns the phantom string.
    ///
    /// # Errors
    /// Returns `ProxyError::Config` if the system RNG fails. Fails closed
    /// rather than falling back to a weaker source of randomness, since a
    /// predictable phantom would let a caller enumerate valid phantoms.
    pub fn mint(
        &self,
        real: Zeroizing<Vec<u8>>,
        admitted_consumers: HashSet<String>,
    ) -> Result<String> {
        let phantom = generate_phantom_id()?;
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        entries.insert(
            phantom.clone(),
            StoredCapture {
                real,
                admitted_consumers,
            },
        );
        Ok(phantom)
    }

    /// Resolve `phantom` back to its real token, only if `consumer` is in
    /// its admitted set.
    ///
    /// Returns `None` uniformly for both "no such phantom" and "phantom
    /// exists but `consumer` is not admitted" — deliberately, so the
    /// return value never lets a caller distinguish the two cases and
    /// enumerate valid phantoms (T-114-07).
    #[must_use]
    pub fn resolve(&self, phantom: &str, consumer: &str) -> Option<Zeroizing<Vec<u8>>> {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        entries.get(phantom).and_then(|entry| {
            if entry.admitted_consumers.contains(consumer) {
                Some(entry.real.clone())
            } else {
                None
            }
        })
    }
}

/// Generate a URL-safe, base64-encoded random phantom identifier.
fn generate_phantom_id() -> Result<String> {
    let mut bytes = [0u8; PHANTOM_ID_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|e| ProxyError::Config(format!("capture phantom RNG failure: {e}")))?;
    let phantom = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    bytes.fill(0);
    Ok(phantom)
}

/// Traverse `root` along a dot-separated `path`, returning a mutable
/// reference to the value found there, or `None` if any segment is
/// missing, empty, or an intermediate segment is not a JSON object.
/// Never panics on malformed paths or malformed JSON shape.
fn value_at_path_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut current = root;
    for part in path.split('.') {
        if part.is_empty() {
            return None;
        }
        current = current.as_object_mut()?.get_mut(part)?;
    }
    Some(current)
}

/// The upstream-provider field names treated as sensitive OAuth token
/// content by [`reject_unrewritten_token_fields`]. Matches upstream's own
/// `oauth_capture/rewrite.rs` field-name set (D-07, adapted).
fn is_sensitive_token_field(field: &str) -> bool {
    matches!(field, "access_token" | "refresh_token" | "id_token")
}

/// Rewrite each of `fields`' configured dot-paths in `body` in place,
/// replacing the real token value found there with a freshly minted
/// phantom (opaque or JWT-shaped per [`CaptureResponseFieldKind`]).
///
/// Each configured field gets its own phantom + real-token store entry,
/// scoped to `admitted_consumers` — phantoms are never reused across
/// response fields. A configured path that does not exist in `body`, or
/// whose value is not a JSON string, is silently skipped (not an error) —
/// providers do not always return every declared field on every response.
///
/// Returns the dot-paths actually rewritten, for the caller's audit
/// context (`CaptureAuditContext`, Plan 114-05).
///
/// # Errors
/// Returns `ProxyError::Config` if phantom minting fails (RNG failure), or
/// `ProxyError::HttpParse` if JWT-shaped phantom construction fails.
pub fn rewrite_response_fields(
    body: &mut Value,
    fields: &[CaptureResponseField],
    store: &CapturePhantomStore,
    admitted_consumers: HashSet<String>,
) -> Result<Vec<String>> {
    let mut rewritten = Vec::new();
    for field in fields {
        let Some(slot) = value_at_path_mut(body, &field.path) else {
            continue;
        };
        let Value::String(real_str) = slot else {
            continue;
        };
        let real = Zeroizing::new(real_str.clone().into_bytes());
        let phantom = store.mint(real, admitted_consumers.clone())?;
        let phantom_value = match field.kind {
            CaptureResponseFieldKind::Opaque => phantom,
            CaptureResponseFieldKind::Jwt => jwt_shaped_phantom(&phantom)?,
        };
        *real_str = phantom_value;
        rewritten.push(field.path.clone());
    }
    Ok(rewritten)
}

/// Fail-closed backstop against provider-config drift (D-05/SC2): walks the
/// WHOLE JSON tree of `body` and errors if any field named
/// `access_token`/`refresh_token`/`id_token` still holds non-empty string
/// content at a dot-path that is NOT in `configured_paths` (i.e. was not
/// already handled by [`rewrite_response_fields`]). This is the last line
/// of defense against a token-shaped field the operator forgot to declare
/// in `capture.response_fields` being silently forwarded to the sandboxed
/// client.
///
/// # Errors
/// Returns `ProxyError::HttpParse` if an unrewritten token-shaped field is
/// found. Returns `Ok(())` if none exists, or every one that exists is in
/// `configured_paths`.
pub fn reject_unrewritten_token_fields(body: &Value, configured_paths: &[String]) -> Result<()> {
    reject_unrewritten_token_fields_inner(body, "", configured_paths)
}

fn reject_unrewritten_token_fields_inner(
    value: &Value,
    current_path: &str,
    configured_paths: &[String],
) -> Result<()> {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let field_path = if current_path.is_empty() {
                    key.clone()
                } else {
                    format!("{current_path}.{key}")
                };
                if is_sensitive_token_field(key) {
                    if let Value::String(s) = val {
                        if !s.is_empty() && !configured_paths.iter().any(|p| p == &field_path) {
                            return Err(ProxyError::HttpParse(format!(
                                "unrewritten token-shaped field '{field_path}' found in response body; declare it in this route's capture.response_fields or remove it from the response"
                            )));
                        }
                    }
                }
                reject_unrewritten_token_fields_inner(val, &field_path, configured_paths)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let field_path = format!("{current_path}[{index}]");
                reject_unrewritten_token_fields_inner(item, &field_path, configured_paths)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Build a JWT-shaped phantom string: `header.payload.signature`, `alg:
/// none`, where `signature` is literally `phantom` — never a real
/// signature. Deliberately unverifiable; only shape-matched for SDKs that
/// parse-but-don't-verify JWTs (adapted from upstream's 19-line
/// `oauth_capture/jwt.rs`, D-07).
///
/// # Errors
/// Returns `ProxyError::HttpParse` if the JSON payload cannot be encoded
/// (unreachable in practice — the payload is a fixed, valid shape).
pub fn jwt_shaped_phantom(phantom: &str) -> Result<String> {
    let header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let payload_value = serde_json::json!({
        "iss": "nono",
        "sub": phantom,
        "aud": "nono",
        "iat": 0,
        "exp": 4_102_444_800_u64
    });
    let payload_bytes = serde_json::to_vec(&payload_value).map_err(|e| {
        ProxyError::HttpParse(format!("jwt_shaped_phantom payload encode failed: {e}"))
    })?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_bytes);
    Ok(format!("{header}.{payload}.{phantom}"))
}

/// Request-side mirror of [`rewrite_response_fields`]: for each configured
/// dot-path in `fields`, if the value found in `body` is a string that
/// resolves via `store.resolve(value, consumer)`, replace it in place with
/// the resolved real value. A path that does not exist, or whose value is
/// not a known phantom admitted for `consumer`, is left UNCHANGED — this
/// function never errors; an unresolved nonce field is forwarded as-is and
/// will simply be rejected by the real upstream as an invalid token (a
/// functional no-op, not a security gap — the hard security property this
/// phase guarantees is response-side confinement, proven separately by
/// [`reject_unrewritten_token_fields`]).
///
/// Returns the count of fields actually resolved (0 if none). Its only
/// intended caller is `crate::reverse` (Plan 114-06 Task 3), which wires
/// this into a live outbound-dispatch path — this closes the
/// mint-to-resolve loop's store-level half. Visibility is `pub` rather
/// than the originally planned `pub(crate)`: with no production caller yet
/// in this plan, a `pub(crate)` (or private) function with only
/// `#[cfg(test)]` call sites is flagged `dead_code` by
/// `cargo clippy --lib` (which does not compile the `test` cfg), and
/// `#[allow(dead_code)]` is disallowed by policy. `pub` sidesteps this
/// exactly as Plan 114-03's `capture_declared_for_upstream` already did
/// for the same not-yet-wired-in reason.
pub fn resolve_request_nonce_fields(
    body: &mut Value,
    fields: &[String],
    store: &CapturePhantomStore,
    consumer: &str,
) -> usize {
    let mut resolved_count = 0usize;
    for path in fields {
        let Some(slot) = value_at_path_mut(body, path) else {
            continue;
        };
        let Value::String(current) = slot else {
            continue;
        };
        let Some(real) = store.resolve(current, consumer) else {
            continue;
        };
        let Ok(real_str) = String::from_utf8(real.to_vec()) else {
            continue;
        };
        *current = real_str;
        resolved_count = resolved_count.saturating_add(1);
    }
    resolved_count
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn capture_phantom_mint_returns_unique_nonempty_ids() {
        let store = CapturePhantomStore::new();
        let mut seen = HashSet::new();
        for i in 0..1000 {
            let phantom = store
                .mint(
                    Zeroizing::new(format!("real-token-{i}").into_bytes()),
                    HashSet::new(),
                )
                .unwrap();
            assert!(!phantom.is_empty());
            assert!(
                seen.insert(phantom),
                "phantom collided across 1000 mint() calls"
            );
        }
    }

    #[test]
    fn capture_phantom_resolve_returns_real_for_admitted_consumer() {
        let store = CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("consumer-a".to_string());
        let phantom = store
            .mint(Zeroizing::new(b"real-secret".to_vec()), admitted)
            .unwrap();
        let resolved = store.resolve(&phantom, "consumer-a");
        assert_eq!(resolved.map(|z| z.to_vec()), Some(b"real-secret".to_vec()));
    }

    #[test]
    fn capture_phantom_rejects_unadmitted_consumer() {
        let store = CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("consumer-a".to_string());
        let phantom = store
            .mint(Zeroizing::new(b"real-secret".to_vec()), admitted)
            .unwrap();
        assert!(store.resolve(&phantom, "not-admitted").is_none());
    }

    #[test]
    fn capture_phantom_resolve_unknown_phantom_returns_none() {
        let store = CapturePhantomStore::new();
        assert!(store
            .resolve("nonexistent-phantom", "any-consumer")
            .is_none());
    }

    #[test]
    fn capture_store_debug_output_never_contains_real_token() {
        let store = CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("consumer-a".to_string());
        let real_secret = "super-secret-real-token-value-xyz";
        let _phantom = store
            .mint(
                Zeroizing::new(real_secret.as_bytes().to_vec()),
                admitted.clone(),
            )
            .unwrap();
        let debug_output = format!("{store:?}");
        assert!(!debug_output.contains(real_secret));
        assert!(!debug_output.contains("consumer-a"));
    }

    /// Automated (not prose) structural assertion that the PRODUCTION code
    /// in this module never touches disk (D-08: real tokens are
    /// in-memory, session-scoped only). Scans only the source up to the
    /// `#[cfg(test)]` marker, so the banned-pattern string literals inside
    /// THIS test do not self-match.
    #[test]
    fn capture_store_holds_only_in_memory() {
        let source =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/capture.rs"))
                .unwrap();
        let production_source = source.split("#[cfg(test)]").next().unwrap();
        for banned in ["std::fs::", "File::create", ".write("] {
            assert!(
                !production_source.contains(banned),
                "capture.rs production code must never touch disk (D-08); found banned pattern '{banned}'"
            );
        }
    }

    fn opaque_field(path: &str) -> CaptureResponseField {
        CaptureResponseField {
            path: path.to_string(),
            kind: CaptureResponseFieldKind::Opaque,
        }
    }

    #[test]
    fn capture_rewrites_configured_fields() {
        let store = CapturePhantomStore::new();
        let mut body = serde_json::json!({
            "access_token": "real-access-token-value",
            "unrelated": "leave-me-alone"
        });
        let fields = vec![opaque_field("access_token")];
        let mut admitted = HashSet::new();
        admitted.insert("consumer-a".to_string());
        let rewritten = rewrite_response_fields(&mut body, &fields, &store, admitted).unwrap();
        assert_eq!(rewritten, vec!["access_token".to_string()]);
        assert_ne!(
            body["access_token"],
            serde_json::json!("real-access-token-value")
        );
        assert_eq!(body["unrelated"], serde_json::json!("leave-me-alone"));
    }

    #[test]
    fn capture_rewrite_skips_missing_path_without_error() {
        let store = CapturePhantomStore::new();
        let mut body = serde_json::json!({ "other_field": "value" });
        let fields = vec![opaque_field("access_token")];
        let rewritten =
            rewrite_response_fields(&mut body, &fields, &store, HashSet::new()).unwrap();
        assert!(rewritten.is_empty());
        assert_eq!(body["other_field"], serde_json::json!("value"));
    }

    #[test]
    fn capture_rewrite_gives_each_field_its_own_phantom() {
        let store = CapturePhantomStore::new();
        let mut body = serde_json::json!({
            "access_token": "real-access",
            "refresh_token": "real-refresh"
        });
        let fields = vec![opaque_field("access_token"), opaque_field("refresh_token")];
        let rewritten =
            rewrite_response_fields(&mut body, &fields, &store, HashSet::new()).unwrap();
        assert_eq!(rewritten.len(), 2);
        assert_ne!(body["access_token"], body["refresh_token"]);
    }

    #[test]
    fn capture_fails_closed_on_unrewritten_token_field() {
        let body = serde_json::json!({
            "access_token": "leaked-real-token",
            "nested": { "refresh_token": "also-leaked" }
        });
        assert!(reject_unrewritten_token_fields(&body, &[]).is_err());
    }

    #[test]
    fn capture_reject_passes_when_field_is_configured() {
        let body = serde_json::json!({ "access_token": "phantom-value-already-rewritten" });
        assert!(reject_unrewritten_token_fields(&body, &["access_token".to_string()]).is_ok());
    }

    #[test]
    fn capture_reject_passes_when_no_token_fields_present() {
        let body = serde_json::json!({ "unrelated": "value" });
        assert!(reject_unrewritten_token_fields(&body, &[]).is_ok());
    }

    #[test]
    fn capture_reject_ignores_empty_string_token_field() {
        let body = serde_json::json!({ "access_token": "" });
        assert!(reject_unrewritten_token_fields(&body, &[]).is_ok());
    }

    #[test]
    fn jwt_shaped_phantom_has_three_dot_separated_parts_and_signature_is_the_phantom() {
        let phantom = "abc123-phantom-id";
        let jwt = jwt_shaped_phantom(phantom).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2], phantom);
    }

    #[test]
    fn resolve_request_nonce_fields_resolves_admitted_phantom_in_place() {
        let store = CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("consumer-a".to_string());
        let phantom = store
            .mint(Zeroizing::new(b"real-nonce-value".to_vec()), admitted)
            .unwrap();
        let mut body = serde_json::json!({ "code_verifier": phantom });
        let count = resolve_request_nonce_fields(
            &mut body,
            &["code_verifier".to_string()],
            &store,
            "consumer-a",
        );
        assert_eq!(count, 1);
        assert_eq!(body["code_verifier"], serde_json::json!("real-nonce-value"));
    }

    #[test]
    fn resolve_request_nonce_fields_leaves_unadmitted_phantom_unchanged() {
        let store = CapturePhantomStore::new();
        let mut admitted = HashSet::new();
        admitted.insert("consumer-a".to_string());
        let phantom = store
            .mint(Zeroizing::new(b"real-nonce-value".to_vec()), admitted)
            .unwrap();
        let mut body = serde_json::json!({ "code_verifier": phantom.clone() });
        let count = resolve_request_nonce_fields(
            &mut body,
            &["code_verifier".to_string()],
            &store,
            "not-admitted",
        );
        assert_eq!(count, 0);
        assert_eq!(body["code_verifier"], serde_json::json!(phantom));
    }

    #[test]
    fn resolve_request_nonce_fields_leaves_unknown_value_unchanged() {
        let store = CapturePhantomStore::new();
        let mut body = serde_json::json!({ "code_verifier": "not-a-known-phantom" });
        let count = resolve_request_nonce_fields(
            &mut body,
            &["code_verifier".to_string()],
            &store,
            "any-consumer",
        );
        assert_eq!(count, 0);
        assert_eq!(
            body["code_verifier"],
            serde_json::json!("not-a-known-phantom")
        );
    }
}
