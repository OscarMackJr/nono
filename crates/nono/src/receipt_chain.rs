//! Keyless, domain-separated hash chain primitive for per-session
//! enforcement receipts (RCPT-02, D-11 amended by D-25).
//!
//! # D-25: keyless — the integrity claim, precisely
//!
//! This chain is **keyless SHA-256**, mirroring `crate::audit`'s alpha
//! chain construction (`hash_event`/`hash_chain`), NOT the CLI-side
//! telemetry module's `Hmac<Sha256>` construction
//! (`crates/nono-cli/src/telemetry/mod.rs`). The integrity claim this
//! produces is: **"nobody edited this without leaving a hash mismatch."**
//! It is deliberately NOT the stronger claim an HMAC chain would make —
//! **"only the key holder could have produced this"** — because the
//! telemetry chain's key is generated fresh per-process from `OsRng` and
//! zeroized on `Drop` (`ChainState.key`), never persisted or recoverable.
//! Mirroring that literally, as D-11 originally prescribed, would produce
//! receipts that nobody — including the operator — could verify once the
//! emitting process exits, contradicting D-09's "governance consumer = the
//! HMAC key holder" premise and RCPT-02's actual use case.
//!
//! **No plan or CLI text may claim the stronger property.** A reviewer
//! auditing this file should confirm no struct here ever grows a `key`
//! field (see the reviewer note below) — that would silently upgrade the
//! claim this module makes without updating the documentation that
//! disclaims it.
//!
//! # D-11 (amended, not overturned): a THIRD, independent chain domain
//!
//! Receipts get their own chain domain, using the identical construction
//! and discipline as the core audit chain — but a domain constant distinct
//! from BOTH `crate::audit::CHAIN_DOMAIN_ALPHA`/`EVENT_DOMAIN_ALPHA` AND
//! the CLI telemetry module's `TELEMETRY_CHAIN_DOMAIN`/
//! `TELEMETRY_EVENT_DOMAIN`. The decisive reason (D-11, unchanged by
//! D-25): an HMAC or hash chain commits each entry to `prev_head`. If
//! receipts and telemetry events interleaved on one chain but landed in
//! two different sinks (D-06 requires a dedicated receipt sink, distinct
//! from the telemetry/audit sinks), verifying the receipt sink *alone*
//! would be structurally impossible — a consumer would need the telemetry
//! events just to recompute the intermediate heads, and would see a chain
//! full of holes. A dedicated domain (and, by construction, a dedicated
//! chain state per writer) sidesteps this entirely: each receipt-chain
//! segment is independently verifiable from the receipt sink alone.
//!
//! # What this module deliberately does NOT define
//!
//! No chain-state struct, no mutex, no `key` field. The advance-under-mutex
//! discipline (mutex held across the full build+advance+emit sequence,
//! chain fields private behind a single accessor — the hardening
//! `crates/nono-cli/src/telemetry/mod.rs::advance_and_emit` already
//! carries) is binary-specific, per-writer state, and is built in later
//! plans (118-05 for `nono.exe`/`nono-agentd.exe`, 118-06 for
//! `nono-shell-broker.exe`) using the two functions below as the
//! primitive. This file is the primitive only.

use crate::undo::types::ContentHash;
use sha2::{Digest, Sha256};

/// Domain separator for receipt-chain event leaf hashes. Distinct from both
/// `crate::audit::EVENT_DOMAIN_ALPHA` (core audit) and the CLI telemetry
/// module's `TELEMETRY_EVENT_DOMAIN` — see this module's doc for why a
/// third, independent domain is required (D-11).
pub const RECEIPT_EVENT_DOMAIN: &[u8] = b"nono.receipt.event.alpha\n";

/// Domain separator for receipt-chain rolling-chain-head hashes. Distinct
/// from both `crate::audit::CHAIN_DOMAIN_ALPHA` and the CLI telemetry
/// module's `TELEMETRY_CHAIN_DOMAIN` — see this module's doc for why a
/// third, independent domain is required (D-11).
pub const RECEIPT_CHAIN_DOMAIN: &[u8] = b"nono.receipt.chain.alpha\n";

/// Hash canonical receipt-event bytes into a receipt-chain leaf.
///
/// Keyless SHA-256 (D-25), identical shape to `crate::audit::hash_event`,
/// with the receipt-only [`RECEIPT_EVENT_DOMAIN`] domain separator instead
/// of the audit or telemetry domains.
#[must_use]
pub fn hash_receipt_event(event_bytes: &[u8]) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(RECEIPT_EVENT_DOMAIN);
    hasher.update(event_bytes);
    ContentHash::from_bytes(hasher.finalize().into())
}

/// Hash one receipt-chain rolling-chain link.
///
/// Keyless SHA-256 (D-25), identical shape to `crate::audit::hash_chain`,
/// with the receipt-only [`RECEIPT_CHAIN_DOMAIN`] domain separator instead
/// of the audit or telemetry domains. The genesis case (`previous == None`)
/// hashes a zeroed 32-byte block, matching `crate::audit::hash_chain`'s
/// genesis behavior exactly.
#[must_use]
pub fn hash_receipt_chain(previous: Option<&ContentHash>, leaf_hash: &ContentHash) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(RECEIPT_CHAIN_DOMAIN);
    if let Some(prev) = previous {
        hasher.update(prev.as_bytes());
    } else {
        hasher.update([0u8; 32]);
    }
    hasher.update(leaf_hash.as_bytes());
    ContentHash::from_bytes(hasher.finalize().into())
}

// REVIEWER NOTE (D-25 is not mechanically enforceable by a compile error):
// this file must never gain a symmetric-secret field on any struct, nor a
// keyed-hash construction (`Hmac<...>`) anywhere in its hash functions.
// Nothing in Rust's type system stops a future edit from adding one —
// `cargo check` would still pass with such a field bolted onto a
// hypothetical struct here. The perturbation proof for this specific claim
// is manual, by design: a reviewer (or `/gsd:code-review`, D-22) must
// re-read this module's doc against its actual hash functions on every
// touch, the same way `crates/nono-cli/src/telemetry/mod.rs`'s
// `ChainState` secret-material field comment documents the OPPOSITE choice
// for its own chain. If a symmetric-secret field is ever proposed for this
// file, that is a D-25 policy reversal requiring a fresh operator decision,
// not a routine edit.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{CHAIN_DOMAIN_ALPHA, EVENT_DOMAIN_ALPHA};

    fn leaf(bytes: &[u8]) -> ContentHash {
        hash_receipt_event(bytes)
    }

    #[test]
    fn hash_receipt_chain_is_deterministic() {
        let leaf_a = leaf(b"{\"session_id\":\"abc\"}");
        let head_1 = hash_receipt_chain(None, &leaf_a);
        let head_2 = hash_receipt_chain(None, &leaf_a);
        assert_eq!(head_1.as_bytes(), head_2.as_bytes());
    }

    #[test]
    fn hash_receipt_chain_changes_when_leaf_hash_changes_by_one_byte() {
        let leaf_a = leaf(b"{\"session_id\":\"abc\"}");
        let leaf_b = leaf(b"{\"session_id\":\"abd\"}"); // one byte differs
        let head_a = hash_receipt_chain(None, &leaf_a);
        let head_b = hash_receipt_chain(None, &leaf_b);
        assert_ne!(
            head_a.as_bytes(),
            head_b.as_bytes(),
            "changing one byte of the leaf hash must change the chain head (avalanche/tamper-detection)"
        );
    }

    #[test]
    fn hash_receipt_chain_advances_across_links() {
        let leaf_a = leaf(b"event-1");
        let leaf_b = leaf(b"event-2");
        let head_1 = hash_receipt_chain(None, &leaf_a);
        let head_2 = hash_receipt_chain(Some(&head_1), &leaf_b);
        assert_ne!(head_1.as_bytes(), head_2.as_bytes());
        // Recomputing from the same inputs reproduces the same head
        // (fail-closed recompute-and-compare, the shape `nono receipt
        // verify` will consume later in this phase).
        let head_2_again = hash_receipt_chain(Some(&head_1), &leaf_b);
        assert_eq!(head_2.as_bytes(), head_2_again.as_bytes());
    }

    #[test]
    fn receipt_domains_are_distinct_from_core_audit_domains() {
        assert_ne!(RECEIPT_CHAIN_DOMAIN, CHAIN_DOMAIN_ALPHA);
        assert_ne!(RECEIPT_EVENT_DOMAIN, EVENT_DOMAIN_ALPHA);
    }

    #[test]
    fn receipt_domains_are_distinct_from_each_other() {
        assert_ne!(RECEIPT_CHAIN_DOMAIN, RECEIPT_EVENT_DOMAIN);
    }
}
