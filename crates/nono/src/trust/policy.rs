//! Trust policy loading, merging, and evaluation
//!
//! Provides functions for parsing trust policy JSON, merging multiple policies
//! from different levels (embedded, user, project), and evaluating files
//! against the merged policy.
//!
//! # Policy Composition
//!
//! Multiple `trust-policy.json` files are merged with additive-only semantics:
//! - Include patterns: union (all patterns from all layers)
//! - Explicit file paths: union
//! - Blocklist digests: union (all blocked digests from all layers)
//! - Blocked publishers: union
//! - Enforcement: strictest wins (deny > warn > audit)
//! - Publishers: union across the layers marked as trust anchors only
//!
//! Every field except `publishers` is monotonically *narrowing*. Contributing
//! an include pattern or an explicit file path can only cause more files to be
//! verified; contributing a blocklist entry can only cause more digests or
//! signers to be rejected; and enforcement is strictest-wins. No layer can
//! weaken another by contributing to any of them.
//!
//! `publishers` is the sole exception, because it is the **trust anchor set**.
//! Adding an entry to it *widens* the policy: a new publisher is a new signer
//! whose signatures will be accepted, and [`Publisher`]`::public_key` lets that
//! entry carry its own verification key inline rather than resolving one from
//! the system keystore. A layer must therefore be declared a trust anchor
//! ([`PolicyLayer::trust_anchor`]) before its publishers are merged;
//! [`PolicyLayer::narrowing_only`] layers can narrow the effective policy but
//! never contribute signers to it.
//!
//! [`merge_policies`] treats **every** input as a trust anchor, so it is only
//! safe for levels the caller already trusts (embedded + user). A caller that
//! merges an untrusted level — a project-level `trust-policy.json` shipped by
//! the repository being run against — must use [`merge_policy_layers`] and
//! mark that level [`PolicyLayer::narrowing_only`].
//!
//! Which level is which is a security-policy judgement and is decided by the
//! caller (`nono-cli`), not here: this module supplies only the mechanism.
//! See `proj/ADR-86-library-boundary-convergence.md`.

use crate::error::{NonoError, Result};

use super::types::{
    BlockedPublisher, Blocklist, BlocklistEntry, Enforcement, Publisher, SignerIdentity,
    TrustPolicy, VerificationOutcome, VerificationResult, TRUST_POLICY_PREDICATE,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Parse a trust policy from a JSON string.
///
/// # Errors
///
/// Returns `NonoError::TrustPolicy` if the JSON is malformed or missing
/// required fields.
pub fn load_policy_from_str(json: &str) -> Result<TrustPolicy> {
    let policy: TrustPolicy = serde_json::from_str(json)
        .map_err(|e| NonoError::TrustPolicy(format!("failed to parse trust policy: {e}")))?;
    policy.validate_version()?;
    Ok(policy)
}

/// Parse a trust policy from a JSON file.
///
/// # Errors
///
/// Returns `NonoError::Io` if the file cannot be read, or
/// `NonoError::TrustPolicy` if the JSON is invalid.
pub fn load_policy_from_file<P: AsRef<Path>>(path: P) -> Result<TrustPolicy> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(NonoError::Io)?;
    load_policy_from_str(&content)
}

/// One policy level participating in a merge, together with whether it is
/// allowed to contribute trust anchors.
///
/// This is the mechanism half of policy composition. The library takes no view
/// on which level is which — the caller declares, per layer, whether that
/// layer's `publishers` may enter the effective trust anchor set. See the
/// module documentation for why `publishers` is the only widening field.
#[derive(Debug, Clone, Copy)]
pub struct PolicyLayer<'a> {
    policy: &'a TrustPolicy,
    contributes_publishers: bool,
}

impl<'a> PolicyLayer<'a> {
    /// A layer that defines trust anchors: its `publishers` are merged into
    /// the effective policy alongside everything else.
    #[must_use]
    pub fn trust_anchor(policy: &'a TrustPolicy) -> Self {
        Self {
            policy,
            contributes_publishers: true,
        }
    }

    /// A layer that may only *narrow* the effective policy.
    ///
    /// Its include patterns, explicit files, blocklist entries and enforcement
    /// level are merged as usual; its `publishers` are ignored entirely, so it
    /// cannot nominate a signer (nor smuggle one in via an inline
    /// [`Publisher::public_key`]).
    #[must_use]
    pub fn narrowing_only(policy: &'a TrustPolicy) -> Self {
        Self {
            policy,
            contributes_publishers: false,
        }
    }

    /// The publishers this layer declared that a merge will discard.
    ///
    /// Empty for a [`trust_anchor`](Self::trust_anchor) layer. Callers use
    /// this to tell the operator that a policy file's `publishers` block had
    /// no effect, rather than dropping it silently.
    #[must_use]
    pub fn dropped_publishers(&self) -> &[Publisher] {
        if self.contributes_publishers {
            &[]
        } else {
            &self.policy.publishers
        }
    }
}

/// Merge multiple trust policies into a single effective policy, treating
/// **every** policy as a trust anchor.
///
/// Policies are merged in order (first = highest priority for
/// first-occurrence-wins deduplication).
///
/// # Security
///
/// Every input is treated as authoritative for `publishers`, so this function
/// must only be given levels the caller already trusts (embedded + user). To
/// merge an untrusted level — a project-level `trust-policy.json` supplied by
/// the repository being run against — use [`merge_policy_layers`] and mark
/// that level [`PolicyLayer::narrowing_only`].
///
/// # Errors
///
/// Returns `NonoError::TrustPolicy` if no policies are provided.
pub fn merge_policies(policies: &[TrustPolicy]) -> Result<TrustPolicy> {
    let layers: Vec<PolicyLayer<'_>> = policies.iter().map(PolicyLayer::trust_anchor).collect();
    merge_policy_layers(&layers)
}

/// Merge policy layers into a single effective policy, honouring each layer's
/// trust-anchor declaration.
///
/// Layers are merged in order (first = highest priority for
/// first-occurrence-wins deduplication). All merging is additive-only:
/// - Blocklist entries, blocked publishers, include patterns and explicit
///   files are unioned (deduplicated by identity)
/// - Enforcement uses the strictest level across all layers
/// - Publishers are unioned across [`PolicyLayer::trust_anchor`] layers only;
///   a [`PolicyLayer::narrowing_only`] layer's publishers are discarded
///
/// # Errors
///
/// Returns `NonoError::TrustPolicy` if no layers are provided.
pub fn merge_policy_layers(layers: &[PolicyLayer<'_>]) -> Result<TrustPolicy> {
    if layers.is_empty() {
        return Err(NonoError::TrustPolicy(
            "no trust policies to merge".to_string(),
        ));
    }

    for layer in layers {
        layer.policy.validate_version()?;
    }

    let mut merged_patterns: Vec<String> = Vec::new();
    let mut seen_patterns: HashSet<String> = HashSet::new();

    let mut merged_files: Vec<String> = Vec::new();
    let mut seen_files: HashSet<String> = HashSet::new();

    let mut merged_publishers: Vec<Publisher> = Vec::new();
    let mut seen_publisher_names: HashSet<String> = HashSet::new();

    let mut merged_digest_entries: Vec<BlocklistEntry> = Vec::new();
    let mut seen_digests: HashSet<String> = HashSet::new();

    let mut merged_blocked_publishers: Vec<BlockedPublisher> = Vec::new();
    let mut seen_blocked_identities: HashSet<String> = HashSet::new();

    let mut strictest_enforcement = Enforcement::Audit;

    for layer in layers {
        let policy = layer.policy;

        // Merge include patterns (deduplicate by pattern string)
        for pattern in &policy.includes {
            if seen_patterns.insert(pattern.clone()) {
                merged_patterns.push(pattern.clone());
            }
        }

        // Merge explicit file paths (deduplicate by path string)
        for file in &policy.files {
            if seen_files.insert(file.clone()) {
                merged_files.push(file.clone());
            }
        }

        // Merge publishers (deduplicate by name, first-occurrence wins).
        // Callers pass layers in precedence order (user-level first), so user
        // publishers take priority over any other layer's.
        //
        // Publishers are the trust anchor set and are the one field where
        // merging *widens* the policy, so a layer that was not declared a
        // trust anchor contributes none of them. Dedup-by-name protected an
        // already-known publisher name but let a narrowing-only layer add a
        // brand-new signer outright — including one carrying its own inline
        // `public_key`, which `verify_keyed_crypto` prefers over the system
        // keystore.
        if layer.contributes_publishers {
            for publisher in &policy.publishers {
                if !seen_publisher_names.insert(publisher.name.clone()) {
                    tracing::debug!(
                        "trust policy merge: publisher '{}' appears in multiple policies, using the highest-precedence definition for verification",
                        publisher.name
                    );
                } else {
                    merged_publishers.push(publisher.clone());
                }
            }
        } else if !policy.publishers.is_empty() {
            tracing::debug!(
                "trust policy merge: discarding {} publisher(s) from a narrowing-only policy layer",
                policy.publishers.len()
            );
        }

        // Merge blocklist digests (deduplicate by sha256)
        for entry in &policy.blocklist.digests {
            if seen_digests.insert(entry.sha256.clone()) {
                merged_digest_entries.push(entry.clone());
            }
        }

        // Merge blocked publishers (deduplicate by identity)
        for blocked in &policy.blocklist.publishers {
            if seen_blocked_identities.insert(blocked.identity.clone()) {
                merged_blocked_publishers.push(blocked.clone());
            }
        }

        // Enforcement: strictest wins
        strictest_enforcement = strictest_enforcement.strictest(policy.enforcement);
    }

    Ok(TrustPolicy {
        predicate: Some(TRUST_POLICY_PREDICATE.to_string()),
        version: None,
        includes: merged_patterns,
        files: merged_files,
        publishers: merged_publishers,
        blocklist: Blocklist {
            digests: merged_digest_entries,
            publishers: merged_blocked_publishers,
        },
        enforcement: strictest_enforcement,
    })
}

/// Evaluate a file against a trust policy.
///
/// Runs the full verification pipeline:
/// 1. Blocklist check (fast reject by digest)
/// 2. If no signer identity provided, file is unsigned
/// 3. Publisher matching against the trust policy
///
/// Returns a [`VerificationResult`] with the outcome and file metadata.
///
/// This function does NOT perform cryptographic verification of bundles.
/// Bundle verification is handled by higher-level code that extracts the
/// [`SignerIdentity`] before calling this function.
pub fn evaluate_file(
    policy: &TrustPolicy,
    path: &Path,
    digest: &str,
    signer: Option<&SignerIdentity>,
) -> VerificationResult {
    // Step 1: Blocklist check (always runs, regardless of enforcement mode)
    if let Some(entry) = policy.check_blocklist(digest) {
        return VerificationResult {
            path: path.to_path_buf(),
            digest: digest.to_string(),
            outcome: VerificationOutcome::Blocked {
                reason: entry.description.clone(),
            },
        };
    }

    // Step 2: Check if file is signed
    let identity = match signer {
        Some(id) => id,
        None => {
            return VerificationResult {
                path: path.to_path_buf(),
                digest: digest.to_string(),
                outcome: VerificationOutcome::Unsigned,
            };
        }
    };

    // Step 3: Check blocked publishers
    if is_publisher_blocked(policy, identity) {
        return VerificationResult {
            path: path.to_path_buf(),
            digest: digest.to_string(),
            outcome: VerificationOutcome::UntrustedPublisher {
                identity: identity.clone(),
            },
        };
    }

    // Step 4: Publisher matching
    let matches = policy.matching_publishers(identity);
    if matches.is_empty() {
        return VerificationResult {
            path: path.to_path_buf(),
            digest: digest.to_string(),
            outcome: VerificationOutcome::UntrustedPublisher {
                identity: identity.clone(),
            },
        };
    }

    VerificationResult {
        path: path.to_path_buf(),
        digest: digest.to_string(),
        outcome: VerificationOutcome::Verified {
            publisher: matches[0].name.clone(),
        },
    }
}

/// Check if a signer identity is on the blocked publishers list.
fn is_publisher_blocked(policy: &TrustPolicy, identity: &SignerIdentity) -> bool {
    policy
        .blocklist
        .publishers
        .iter()
        .any(|blocked| match identity {
            SignerIdentity::Keyed { key_id } => blocked.identity == *key_id,
            SignerIdentity::Keyless {
                issuer, repository, ..
            } => {
                if blocked.identity != *issuer {
                    return false;
                }
                // If the blocklist entry specifies a repository, match it.
                // If no repository specified, the entire issuer is blocked.
                match &blocked.repository {
                    Some(blocked_repo) => blocked_repo == repository,
                    None => true,
                }
            }
        })
}

/// Well-known directory names that never contain instruction files and are
/// typically very large. Sorted for binary search.
const SKIP_DIRS: &[&str] = &[
    ".cache",
    ".git",
    ".gradle",
    ".hg",
    ".mypy_cache",
    ".next",
    ".nuxt",
    ".pytest_cache",
    ".ruff_cache",
    ".svn",
    ".terraform",
    ".tox",
    ".venv",
    "__pycache__",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "venv",
];

/// Scan a directory for files matching instruction patterns.
///
/// Returns the list of paths that match any pattern in the trust policy.
/// Hidden directories are scanned unless they are explicitly listed in the
/// built-in heavy-directory skip set or provided via `extra_skip_dirs`.
///
/// # Errors
///
/// Returns `NonoError::TrustPolicy` if patterns cannot be compiled, or
/// `NonoError::Io` if directory traversal fails.
pub fn find_included_files<P: AsRef<Path>>(policy: &TrustPolicy, root: P) -> Result<Vec<PathBuf>> {
    find_included_files_with_skip_dirs(policy, root, &[])
}

/// Scan a directory for files matching instruction patterns, skipping extra
/// directory names in addition to the built-in heavy-directory list.
pub fn find_included_files_with_skip_dirs<P: AsRef<Path>>(
    policy: &TrustPolicy,
    root: P,
    extra_skip_dirs: &[String],
) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();
    let matcher = policy.include_matcher()?;
    let extra_skip_dirs: std::collections::HashSet<&str> =
        extra_skip_dirs.iter().map(String::as_str).collect();
    let mut results = Vec::new();
    let mut visited = std::collections::HashSet::new();

    if policy.includes.is_empty() {
        return Ok(results);
    }

    find_files_recursive(
        root,
        root,
        &matcher,
        &extra_skip_dirs,
        &mut results,
        &mut visited,
        0,
    )?;

    results.sort();
    Ok(results)
}

fn should_skip_dir(name: &str, extra_skip_dirs: &std::collections::HashSet<&str>) -> bool {
    SKIP_DIRS.binary_search(&name).is_ok() || extra_skip_dirs.contains(name)
}

fn find_files_recursive(
    root: &Path,
    dir: &Path,
    matcher: &super::types::IncludePatterns,
    extra_skip_dirs: &std::collections::HashSet<&str>,
    results: &mut Vec<PathBuf>,
    visited: &mut std::collections::HashSet<(u64, u64)>,
    depth: u32,
) -> Result<()> {
    const MAX_DEPTH: u32 = 16;
    if depth > MAX_DEPTH {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir).map_err(NonoError::Io)?;

    for entry in entries {
        let entry = entry.map_err(NonoError::Io)?;
        let path = entry.path();
        let meta = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if meta.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if should_skip_dir(&name_str, extra_skip_dirs) {
                continue;
            }

            #[cfg(unix)]
            let file_id = Some({
                use std::os::unix::fs::MetadataExt;
                (meta.dev(), meta.ino())
            });
            #[cfg(not(unix))]
            let file_id: Option<(u64, u64)> = None;

            if let Some(file_id) = file_id {
                if !visited.insert(file_id) {
                    continue;
                }
            }

            find_files_recursive(
                root,
                &path,
                matcher,
                extra_skip_dirs,
                results,
                visited,
                depth + 1,
            )?;
        } else if meta.is_file() {
            if path.to_string_lossy().ends_with(".bundle") {
                continue;
            }

            if let Ok(relative) = path.strip_prefix(root) {
                if matcher.is_match(relative) {
                    results.push(path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_policy(
        enforcement: Enforcement,
        publishers: Vec<Publisher>,
        blocklist_digests: Vec<BlocklistEntry>,
    ) -> TrustPolicy {
        TrustPolicy {
            predicate: Some(TRUST_POLICY_PREDICATE.to_string()),
            version: None,
            includes: vec!["SKILLS*".to_string(), "CLAUDE*".to_string()],
            files: vec![],
            publishers,
            blocklist: Blocklist {
                digests: blocklist_digests,
                publishers: vec![],
            },
            enforcement,
        }
    }

    fn keyed_publisher(name: &str, key_id: &str) -> Publisher {
        Publisher {
            name: name.to_string(),
            issuer: None,
            repository: None,
            workflow: None,
            ref_pattern: None,
            key_id: Some(key_id.to_string()),
            public_key: None,
        }
    }

    fn keyless_publisher(name: &str, issuer: &str, repo: &str) -> Publisher {
        Publisher {
            name: name.to_string(),
            issuer: Some(issuer.to_string()),
            repository: Some(repo.to_string()),
            workflow: Some("*".to_string()),
            ref_pattern: Some("*".to_string()),
            key_id: None,
            public_key: None,
        }
    }

    // -----------------------------------------------------------------------
    // load_policy_from_str
    // -----------------------------------------------------------------------

    #[test]
    fn load_valid_policy() {
        let json = r#"{
            "version": 1,
            "includes": ["SKILLS*"],
            "publishers": [],
            "blocklist": { "digests": [] },
            "enforcement": "deny"
        }"#;
        let policy = load_policy_from_str(json).unwrap();
        assert_eq!(policy.version, Some(1));
        assert_eq!(policy.enforcement, Enforcement::Deny);
        assert_eq!(policy.includes.len(), 1);
    }

    #[test]
    fn load_policy_with_publishers() {
        let json = r#"{
            "version": 1,
            "includes": ["SKILLS*"],
            "publishers": [
                {
                    "name": "local",
                    "key_id": "nono-keystore:default"
                },
                {
                    "name": "ci",
                    "issuer": "https://token.actions.githubusercontent.com",
                    "repository": "org/repo",
                    "workflow": "*",
                    "ref_pattern": "refs/tags/v*"
                }
            ],
            "blocklist": { "digests": [] },
            "enforcement": "warn"
        }"#;
        let policy = load_policy_from_str(json).unwrap();
        assert_eq!(policy.publishers.len(), 2);
        assert!(policy.publishers[0].is_keyed());
        assert!(policy.publishers[1].is_keyless());
        assert_eq!(policy.enforcement, Enforcement::Warn);
    }

    #[test]
    fn load_policy_invalid_json() {
        let result = load_policy_from_str("not json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("failed to parse trust policy"));
    }

    #[test]
    fn load_policy_missing_field() {
        let json = r#"{ "version": 1 }"#;
        let result = load_policy_from_str(json);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // load_policy_from_file
    // -----------------------------------------------------------------------

    #[test]
    fn load_policy_from_file_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trust-policy.json");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            write!(
                f,
                r#"{{
                    "version": 1,
                    "includes": ["AGENT.MD"],
                    "publishers": [],
                    "blocklist": {{ "digests": [] }},
                    "enforcement": "audit"
                }}"#
            )
            .unwrap();
        }
        let policy = load_policy_from_file(&path).unwrap();
        assert_eq!(policy.enforcement, Enforcement::Audit);
    }

    #[test]
    fn load_policy_from_file_not_found() {
        let result = load_policy_from_file("/nonexistent/trust-policy.json");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // merge_policies
    // -----------------------------------------------------------------------

    #[test]
    fn merge_empty_errors() {
        let result = merge_policies(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn merge_single_policy_unchanged() {
        let policy = make_policy(
            Enforcement::Warn,
            vec![keyed_publisher("dev", "key1")],
            vec![],
        );
        let merged = merge_policies(std::slice::from_ref(&policy)).unwrap();
        assert_eq!(merged.enforcement, Enforcement::Warn);
        assert_eq!(merged.publishers.len(), 1);
    }

    #[test]
    fn merge_unions_publishers() {
        let p1 = make_policy(
            Enforcement::Audit,
            vec![keyed_publisher("dev", "key1")],
            vec![],
        );
        let p2 = make_policy(
            Enforcement::Audit,
            vec![keyless_publisher("ci", "https://issuer", "org/repo")],
            vec![],
        );
        let merged = merge_policies(&[p1, p2]).unwrap();
        assert_eq!(merged.publishers.len(), 2);
    }

    #[test]
    fn merge_deduplicates_publishers_by_name() {
        let p1 = make_policy(
            Enforcement::Audit,
            vec![keyed_publisher("dev", "key1")],
            vec![],
        );
        let p2 = make_policy(
            Enforcement::Audit,
            vec![keyed_publisher("dev", "key2")], // same name, different key
            vec![],
        );
        let merged = merge_policies(&[p1, p2]).unwrap();
        assert_eq!(merged.publishers.len(), 1);
        // First occurrence wins
        assert_eq!(merged.publishers[0].key_id.as_deref(), Some("key1"));
    }

    #[test]
    fn merge_unions_blocklist_digests() {
        let entry1 = BlocklistEntry {
            sha256: "aaaa".to_string(),
            description: "bad1".to_string(),
            added: "2026-01-01".to_string(),
        };
        let entry2 = BlocklistEntry {
            sha256: "bbbb".to_string(),
            description: "bad2".to_string(),
            added: "2026-02-01".to_string(),
        };
        let p1 = make_policy(Enforcement::Audit, vec![], vec![entry1]);
        let p2 = make_policy(Enforcement::Audit, vec![], vec![entry2]);
        let merged = merge_policies(&[p1, p2]).unwrap();
        assert_eq!(merged.blocklist.digests.len(), 2);
    }

    #[test]
    fn merge_deduplicates_blocklist_by_digest() {
        let entry = BlocklistEntry {
            sha256: "aaaa".to_string(),
            description: "bad".to_string(),
            added: "2026-01-01".to_string(),
        };
        let p1 = make_policy(Enforcement::Audit, vec![], vec![entry.clone()]);
        let p2 = make_policy(Enforcement::Audit, vec![], vec![entry]);
        let merged = merge_policies(&[p1, p2]).unwrap();
        assert_eq!(merged.blocklist.digests.len(), 1);
    }

    #[test]
    fn merge_unions_includes() {
        let mut p1 = make_policy(Enforcement::Audit, vec![], vec![]);
        p1.includes = vec!["SKILLS*".to_string()];
        let mut p2 = make_policy(Enforcement::Audit, vec![], vec![]);
        p2.includes = vec!["AGENT.MD".to_string()];
        let merged = merge_policies(&[p1, p2]).unwrap();
        assert_eq!(merged.includes.len(), 2);
    }

    #[test]
    fn merge_deduplicates_patterns() {
        let p1 = make_policy(Enforcement::Audit, vec![], vec![]);
        let p2 = make_policy(Enforcement::Audit, vec![], vec![]);
        // Both have "SKILLS*" and "CLAUDE*"
        let merged = merge_policies(&[p1, p2]).unwrap();
        assert_eq!(merged.includes.len(), 2);
    }

    #[test]
    fn merge_strictest_enforcement_wins() {
        let p1 = make_policy(Enforcement::Audit, vec![], vec![]);
        let p2 = make_policy(Enforcement::Warn, vec![], vec![]);
        let p3 = make_policy(Enforcement::Deny, vec![], vec![]);
        let merged = merge_policies(&[p1, p2, p3]).unwrap();
        assert_eq!(merged.enforcement, Enforcement::Deny);
    }

    #[test]
    fn merge_project_cannot_weaken() {
        // User sets deny, project tries audit — deny wins
        let user = make_policy(Enforcement::Deny, vec![], vec![]);
        let project = make_policy(Enforcement::Audit, vec![], vec![]);
        let merged = merge_policies(&[user, project]).unwrap();
        assert_eq!(merged.enforcement, Enforcement::Deny);
    }

    // -----------------------------------------------------------------------
    // CR-05 regression: a narrowing-only layer cannot contribute trust anchors
    // -----------------------------------------------------------------------

    /// CR-05: a project-level policy could previously *add* a publisher to the
    /// effective trust anchor set. Dedup-by-name protected an already-known
    /// name but accepted a brand-new one outright, so a repository could
    /// nominate itself as a trusted signer under the operator's policy.
    #[test]
    fn narrowing_only_layer_cannot_add_a_publisher() {
        let user = make_policy(
            Enforcement::Deny,
            vec![keyed_publisher("operator", "operator-key")],
            vec![],
        );
        let project = make_policy(
            Enforcement::Deny,
            vec![keyed_publisher("attacker", "attacker-key")],
            vec![],
        );

        let merged = merge_policy_layers(&[
            PolicyLayer::trust_anchor(&user),
            PolicyLayer::narrowing_only(&project),
        ])
        .unwrap();

        assert_eq!(merged.publishers.len(), 1);
        assert_eq!(merged.publishers[0].name, "operator");
        assert!(
            !merged.publishers.iter().any(|p| p.name == "attacker"),
            "a narrowing-only layer must not contribute a trust anchor: {:?}",
            merged.publishers
        );
    }

    /// CR-05: the inline `public_key` escalation specifically. A project
    /// publisher carrying its own base64 SPKI is the strongest form of the
    /// bug, because `trust_scan::verify_keyed_crypto` prefers an inline key
    /// over the system keystore. It must never reach the merged policy.
    #[test]
    fn narrowing_only_layer_cannot_smuggle_an_inline_public_key() {
        let user = make_policy(Enforcement::Deny, vec![], vec![]);
        let mut attacker = keyed_publisher("attacker", "evil");
        attacker.public_key = Some("QUFBQUFBQUFBQUFB".to_string());
        let project = make_policy(Enforcement::Deny, vec![attacker], vec![]);

        let merged = merge_policy_layers(&[
            PolicyLayer::trust_anchor(&user),
            PolicyLayer::narrowing_only(&project),
        ])
        .unwrap();

        assert!(
            merged.publishers.is_empty(),
            "no publisher may survive from a narrowing-only layer: {:?}",
            merged.publishers
        );
    }

    /// CR-05: a narrowing-only layer must still be able to *narrow* — that is
    /// the whole point of merging it. Guards against over-correcting into
    /// "ignore project policies entirely".
    #[test]
    fn narrowing_only_layer_still_narrows() {
        let user = make_policy(Enforcement::Audit, vec![], vec![]);
        let mut project = make_policy(Enforcement::Deny, vec![], vec![]);
        project.includes = vec!["PROJECT.md".to_string()];
        project.files = vec!["docs/AGENT.md".to_string()];
        project.blocklist.digests.push(BlocklistEntry {
            sha256: "cafe".to_string(),
            description: "known bad".to_string(),
            added: "2026-01-01".to_string(),
        });

        let merged = merge_policy_layers(&[
            PolicyLayer::trust_anchor(&user),
            PolicyLayer::narrowing_only(&project),
        ])
        .unwrap();

        assert_eq!(merged.enforcement, Enforcement::Deny);
        assert!(merged.includes.contains(&"PROJECT.md".to_string()));
        assert!(merged.files.contains(&"docs/AGENT.md".to_string()));
        assert_eq!(merged.blocklist.digests.len(), 1);
    }

    /// The dropped set is reported so callers can warn instead of silently
    /// discarding an operator-visible block of the policy file.
    #[test]
    fn dropped_publishers_reports_the_discarded_set() {
        let policy = make_policy(
            Enforcement::Deny,
            vec![keyed_publisher("attacker", "evil")],
            vec![],
        );

        assert_eq!(
            PolicyLayer::narrowing_only(&policy)
                .dropped_publishers()
                .len(),
            1
        );
        assert!(PolicyLayer::trust_anchor(&policy)
            .dropped_publishers()
            .is_empty());
    }

    /// `merge_policies` keeps its documented all-trust-anchor behaviour, so
    /// embedded+user composition is unchanged.
    #[test]
    fn merge_policies_still_treats_every_input_as_a_trust_anchor() {
        let embedded = make_policy(
            Enforcement::Audit,
            vec![keyed_publisher("embedded", "k1")],
            vec![],
        );
        let user = make_policy(
            Enforcement::Audit,
            vec![keyed_publisher("user", "k2")],
            vec![],
        );
        let merged = merge_policies(&[embedded, user]).unwrap();
        assert_eq!(merged.publishers.len(), 2);
    }

    #[test]
    fn merge_ignores_legacy_version_field() {
        // version is deprecated — merge succeeds regardless of its value.
        let p1 = make_policy(Enforcement::Audit, vec![], vec![]);
        let mut p2 = make_policy(Enforcement::Audit, vec![], vec![]);
        p2.version = Some(99);
        assert!(merge_policies(&[p1, p2]).is_ok());
    }

    // -----------------------------------------------------------------------
    // evaluate_file
    // -----------------------------------------------------------------------

    #[test]
    fn evaluate_blocked_file() {
        let entry = BlocklistEntry {
            sha256: "deadbeef".to_string(),
            description: "known malicious".to_string(),
            added: "2026-01-01".to_string(),
        };
        let policy = make_policy(Enforcement::Deny, vec![], vec![entry]);
        let result = evaluate_file(
            &policy,
            Path::new("SKILLS.md"),
            "deadbeef",
            Some(&SignerIdentity::Keyed {
                key_id: "key".to_string(),
            }),
        );
        assert!(matches!(
            result.outcome,
            VerificationOutcome::Blocked { .. }
        ));
    }

    #[test]
    fn evaluate_unsigned_file() {
        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let result = evaluate_file(&policy, Path::new("SKILLS.md"), "abcd1234", None);
        assert!(matches!(result.outcome, VerificationOutcome::Unsigned));
    }

    #[test]
    fn evaluate_trusted_keyed() {
        let policy = make_policy(
            Enforcement::Deny,
            vec![keyed_publisher("dev", "my-key")],
            vec![],
        );
        let identity = SignerIdentity::Keyed {
            key_id: "my-key".to_string(),
        };
        let result = evaluate_file(&policy, Path::new("SKILLS.md"), "abcd", Some(&identity));
        assert!(result.outcome.is_verified());
        if let VerificationOutcome::Verified { publisher } = &result.outcome {
            assert_eq!(publisher, "dev");
        }
    }

    #[test]
    fn evaluate_trusted_keyless() {
        let policy = make_policy(
            Enforcement::Deny,
            vec![keyless_publisher("ci", "https://issuer", "org/repo")],
            vec![],
        );
        let identity = SignerIdentity::Keyless {
            issuer: "https://issuer".to_string(),
            repository: "org/repo".to_string(),
            workflow: ".github/workflows/sign.yml".to_string(),
            git_ref: "refs/tags/v1.0.0".to_string(),
        };
        let result = evaluate_file(&policy, Path::new("CLAUDE.md"), "abcd", Some(&identity));
        assert!(result.outcome.is_verified());
    }

    #[test]
    fn evaluate_untrusted_publisher() {
        let policy = make_policy(
            Enforcement::Deny,
            vec![keyed_publisher("dev", "my-key")],
            vec![],
        );
        let identity = SignerIdentity::Keyed {
            key_id: "unknown-key".to_string(),
        };
        let result = evaluate_file(&policy, Path::new("SKILLS.md"), "abcd", Some(&identity));
        assert!(matches!(
            result.outcome,
            VerificationOutcome::UntrustedPublisher { .. }
        ));
    }

    #[test]
    fn evaluate_blocked_publisher() {
        let mut policy = make_policy(
            Enforcement::Deny,
            vec![keyless_publisher("ci", "https://evil.issuer", "evil/repo")],
            vec![],
        );
        policy.blocklist.publishers.push(BlockedPublisher {
            identity: "https://evil.issuer".to_string(),
            repository: None,
            reason: "compromised".to_string(),
            added: "2026-01-01".to_string(),
        });
        let identity = SignerIdentity::Keyless {
            issuer: "https://evil.issuer".to_string(),
            repository: "evil/repo".to_string(),
            workflow: "*".to_string(),
            git_ref: "*".to_string(),
        };
        let result = evaluate_file(&policy, Path::new("SKILLS.md"), "abcd", Some(&identity));
        assert!(matches!(
            result.outcome,
            VerificationOutcome::UntrustedPublisher { .. }
        ));
    }

    #[test]
    fn evaluate_blocked_publisher_by_repository() {
        let mut policy = make_policy(
            Enforcement::Deny,
            vec![
                keyless_publisher(
                    "ci",
                    "https://token.actions.githubusercontent.com",
                    "good/repo",
                ),
                keyless_publisher(
                    "ci2",
                    "https://token.actions.githubusercontent.com",
                    "evil/repo",
                ),
            ],
            vec![],
        );
        policy.blocklist.publishers.push(BlockedPublisher {
            identity: "https://token.actions.githubusercontent.com".to_string(),
            repository: Some("evil/repo".to_string()),
            reason: "compromised repo".to_string(),
            added: "2026-01-01".to_string(),
        });

        // evil/repo should be blocked
        let evil_identity = SignerIdentity::Keyless {
            issuer: "https://token.actions.githubusercontent.com".to_string(),
            repository: "evil/repo".to_string(),
            workflow: "*".to_string(),
            git_ref: "*".to_string(),
        };
        let result = evaluate_file(
            &policy,
            Path::new("SKILLS.md"),
            "abcd",
            Some(&evil_identity),
        );
        assert!(matches!(
            result.outcome,
            VerificationOutcome::UntrustedPublisher { .. }
        ));

        // good/repo should NOT be blocked
        let good_identity = SignerIdentity::Keyless {
            issuer: "https://token.actions.githubusercontent.com".to_string(),
            repository: "good/repo".to_string(),
            workflow: "*".to_string(),
            git_ref: "*".to_string(),
        };
        let result = evaluate_file(
            &policy,
            Path::new("SKILLS.md"),
            "abcd",
            Some(&good_identity),
        );
        assert!(matches!(
            result.outcome,
            VerificationOutcome::Verified { .. }
        ));
    }

    #[test]
    fn evaluate_blocklist_checked_before_signer() {
        // Even with a valid signer, blocklist entry should win
        let entry = BlocklistEntry {
            sha256: "baddigest".to_string(),
            description: "malicious".to_string(),
            added: "2026-01-01".to_string(),
        };
        let policy = make_policy(
            Enforcement::Deny,
            vec![keyed_publisher("dev", "my-key")],
            vec![entry],
        );
        let identity = SignerIdentity::Keyed {
            key_id: "my-key".to_string(),
        };
        let result = evaluate_file(
            &policy,
            Path::new("SKILLS.md"),
            "baddigest",
            Some(&identity),
        );
        assert!(matches!(
            result.outcome,
            VerificationOutcome::Blocked { .. }
        ));
    }

    #[test]
    fn evaluate_result_contains_path_and_digest() {
        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let result = evaluate_file(&policy, Path::new("AGENT.MD"), "digest123", None);
        assert_eq!(result.path, Path::new("AGENT.MD"));
        assert_eq!(result.digest, "digest123");
    }

    // -----------------------------------------------------------------------
    // find_included_files
    // -----------------------------------------------------------------------

    #[test]
    fn find_included_files_in_directory() {
        let dir = tempfile::tempdir().unwrap();
        // Create matching files
        std::fs::write(dir.path().join("SKILLS.md"), "content").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "content").unwrap();
        // Create non-matching files
        std::fs::write(dir.path().join("README.md"), "content").unwrap();
        std::fs::write(dir.path().join("main.rs"), "content").unwrap();

        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let files = find_included_files(&policy, dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn find_included_files_in_claude_subdir() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude").join("commands");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("deploy.md"), "content").unwrap();

        let mut policy = make_policy(Enforcement::Deny, vec![], vec![]);
        policy.includes.push(".claude/**/*.md".to_string());

        let files = find_included_files(&policy, dir.path()).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn find_included_files_skips_git_dir() {
        let dir = tempfile::tempdir().unwrap();
        let hidden = dir.path().join(".git");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("SKILLS.md"), "content").unwrap();

        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let files = find_included_files(&policy, dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn find_included_files_in_non_special_hidden_dir() {
        let dir = tempfile::tempdir().unwrap();
        let hidden = dir.path().join(".hidden").join("commands");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("deploy.md"), "content").unwrap();

        let mut policy = make_policy(Enforcement::Deny, vec![], vec![]);
        policy.includes.push(".hidden/**/*.md".to_string());

        let files = find_included_files(&policy, dir.path()).unwrap();
        assert_eq!(files, vec![hidden.join("deploy.md")]);
    }

    #[test]
    fn find_included_files_skips_well_known_heavy_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let node_modules = dir.path().join("node_modules");
        std::fs::create_dir_all(&node_modules).unwrap();
        std::fs::write(node_modules.join("SKILLS.md"), "content").unwrap();
        std::fs::write(dir.path().join("SKILLS.md"), "content").unwrap();

        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let files = find_included_files(&policy, dir.path()).unwrap();

        assert_eq!(files, vec![dir.path().join("SKILLS.md")]);
    }

    #[test]
    fn find_included_files_respects_extra_skip_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let generated = dir.path().join("generated");
        std::fs::create_dir_all(&generated).unwrap();
        std::fs::write(generated.join("SKILLS.md"), "content").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "content").unwrap();

        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let files =
            find_included_files_with_skip_dirs(&policy, dir.path(), &[String::from("generated")])
                .unwrap();

        assert_eq!(files, vec![dir.path().join("CLAUDE.md")]);
    }

    #[test]
    fn find_included_files_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let files = find_included_files(&policy, dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn find_included_files_follows_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real_skills.md");
        std::fs::write(&target, "content").unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join("SKILLS.md")).unwrap();

        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let files = find_included_files(&policy, dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("SKILLS.md"));
    }

    #[test]
    fn find_included_files_skips_bundle_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SKILLS.md"), "content").unwrap();
        std::fs::write(dir.path().join("SKILLS.md.bundle"), "{}").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "content").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md.bundle"), "{}").unwrap();

        let policy = make_policy(Enforcement::Deny, vec![], vec![]);
        let files = find_included_files(&policy, dir.path()).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .all(|path| !path.to_string_lossy().ends_with(".bundle")));
    }
}
