//! Runtime for `nono override request` — denial context bundle for the approver pipeline (CLI-01).
//!
//! When a developer hits a false-positive sandbox block, they need a non-self-service path to
//! request an exception.  This module gathers the denial context (scope paths/domains,
//! `repo_context`, denial reason) and emits:
//!
//! 1. A structured JSON **request bundle** with shape
//!    `{ "scope": { "paths": [...], "domains": [...] }, "repo_context": "<repo>",
//!       "reason": "<denial reason>", "nonce": "<fresh-random-hex>" }`
//! 2. A human-readable summary to stdout.
//!
//! The bundle is intended for the out-of-nono approver/KMS-signing pipeline.  Per D-07 /
//! D-08 this command performs **no crypto and no live check** — it is strictly a context
//! gather and bundle emit step.  The `apply` verb (full offline+live verify) lives in
//! nono-py, not nono.exe.
//!
//! # Threat model (T-93-03-01 / T-93-03-02 / T-93-03-03)
//!
//! - Fresh per-invocation `nonce` (16 random bytes, hex-encoded) addresses replay (T-93-03-01).
//! - No verification or grant happens here; `apply` in nono-py runs the full fail-closed
//!   verify before any capability expansion (T-93-03-02 accepted).
//! - Bundle carries only scope paths/domains/repo/reason/nonce — no credentials or key
//!   material (T-93-03-03 mitigated via existing redaction policy).

use crate::cli::OverrideRequestArgs;
use nono::{NonoError, Result};
use rand::RngExt;
use serde_json::{json, Value};

/// Emit a structured JSON override-request bundle to stdout plus a human-readable summary.
///
/// # Parameters
///
/// - `args` — the parsed `OverrideRequestArgs` carrying scope paths, domains, `repo_context`,
///   and denial reason.
///
/// # Returns
///
/// `Ok(())` on success.  The bundle JSON and summary are written to stdout.
/// `Err(NonoError)` if JSON serialization fails.
///
/// # Bundle shape
///
/// ```json
/// {
///   "scope": { "paths": ["<path>", ...], "domains": ["<domain>", ...] },
///   "repo_context": "<repo or empty>",
///   "reason": "<denial reason>",
///   "nonce": "<32-char hex>"
/// }
/// ```
pub(crate) fn run_override_request(args: OverrideRequestArgs) -> Result<()> {
    let nonce = fresh_nonce();
    let bundle = build_bundle(&args.scope_paths, &args.scope_domains, &args.repo_context, &args.reason, &nonce);

    let bundle_json = serde_json::to_string_pretty(&bundle).map_err(|e| {
        NonoError::SandboxInit(format!(
            "override request: failed to serialize bundle: {e}"
        ))
    })?;

    // Human-readable summary to stdout (before the JSON block for visual clarity).
    println!("=== nono override request bundle ===");
    println!("Repo context : {}", args.repo_context.as_deref().unwrap_or("<not set>"));
    println!("Denial reason: {}", args.reason);
    if !args.scope_paths.is_empty() {
        println!("Scope paths  : {}", args.scope_paths.join(", "));
    }
    if !args.scope_domains.is_empty() {
        println!("Scope domains: {}", args.scope_domains.join(", "));
    }
    println!("Nonce        : {nonce}");
    println!();
    println!("Submit the JSON bundle below to your override approver:");
    println!("{bundle_json}");

    Ok(())
}

/// Build the request bundle as a JSON `Value`.
///
/// Extracted from `run_override_request` so tests can call it without I/O.
fn build_bundle(
    paths: &[String],
    domains: &[String],
    repo_context: &Option<String>,
    reason: &str,
    nonce: &str,
) -> Value {
    json!({
        "scope": {
            "paths": paths,
            "domains": domains,
        },
        "repo_context": repo_context.as_deref().unwrap_or(""),
        "reason": reason,
        "nonce": nonce,
    })
}

/// Generate a fresh 16-byte random nonce, hex-encoded to a 32-character lowercase string.
///
/// Two calls to `fresh_nonce()` are statistically guaranteed to differ (128 bits of entropy).
fn fresh_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The bundle JSON must contain all four required top-level keys.
    #[test]
    fn bundle_json_has_required_keys() {
        let bundle = build_bundle(
            &["/tmp/work".to_string()],
            &["api.example.com".to_string()],
            &Some("github.com/example/repo".to_string()),
            "sensitive_path",
            "deadbeefdeadbeefdeadbeefdeadbeef",
        );

        // Must be valid JSON (serde_json::Value already guarantees this — assert round-trip).
        let serialized = serde_json::to_string(&bundle).unwrap();
        let parsed: Value = serde_json::from_str(&serialized).unwrap();

        // All four top-level keys must be present.
        assert!(parsed.get("scope").is_some(), "bundle must have 'scope'");
        assert!(parsed.get("repo_context").is_some(), "bundle must have 'repo_context'");
        assert!(parsed.get("reason").is_some(), "bundle must have 'reason'");
        assert!(parsed.get("nonce").is_some(), "bundle must have 'nonce'");

        // Scope must have 'paths' and 'domains' sub-keys.
        let scope = parsed.get("scope").unwrap();
        assert!(scope.get("paths").is_some(), "scope must have 'paths'");
        assert!(scope.get("domains").is_some(), "scope must have 'domains'");
    }

    /// Two calls to `fresh_nonce()` must produce different values (distinct per invocation).
    #[test]
    fn nonce_is_distinct_across_invocations() {
        let n1 = fresh_nonce();
        let n2 = fresh_nonce();
        assert_ne!(n1, n2, "nonces must differ across invocations (T-93-03-01)");
    }

    /// Nonce is 32 hex characters (16 bytes × 2 hex digits).
    #[test]
    fn nonce_is_32_hex_chars() {
        let n = fresh_nonce();
        assert_eq!(n.len(), 32, "nonce must be exactly 32 hex chars");
        assert!(
            n.chars().all(|c| c.is_ascii_hexdigit()),
            "nonce must be lowercase hex: {n}"
        );
    }

    /// Bundle carries scope paths and domains from args.
    #[test]
    fn bundle_scope_contains_provided_paths_and_domains() {
        let paths = vec!["/home/user/.aws".to_string(), "/tmp/work".to_string()];
        let domains = vec!["internal.example.com".to_string()];
        let bundle = build_bundle(
            &paths,
            &domains,
            &Some("github.com/org/proj".to_string()),
            "path_not_granted",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );

        let scope = bundle.get("scope").unwrap();
        let bundle_paths: Vec<&str> = scope
            .get("paths")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(bundle_paths, vec!["/home/user/.aws", "/tmp/work"]);

        let bundle_domains: Vec<&str> = scope
            .get("domains")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(bundle_domains, vec!["internal.example.com"]);
    }

    /// When repo_context is None the bundle emits an empty string (not null / absent).
    #[test]
    fn bundle_repo_context_absent_emits_empty_string() {
        let bundle = build_bundle(&[], &[], &None, "path_not_granted", "abcd1234");
        let repo = bundle.get("repo_context").unwrap().as_str().unwrap();
        assert_eq!(repo, "", "absent repo_context must be empty string in bundle");
    }
}
