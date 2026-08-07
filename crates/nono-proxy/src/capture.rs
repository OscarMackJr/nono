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

use crate::error::{ProxyError, Result};
use base64::Engine as _;
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
}
