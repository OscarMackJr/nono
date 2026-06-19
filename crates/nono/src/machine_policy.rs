//! Machine-level egress policy read from the Windows registry.
//!
//! This module provides the platform-neutral [`MachineEgressPolicy`] type and
//! the [`read_machine_egress_policy`] function that reads from
//! `HKLM\SOFTWARE\Policies\nono`.
//!
//! # Fail-Secure Taxonomy (D-07)
//!
//! | Registry state | Return value |
//! |----------------|-------------|
//! | Key **absent** (`ERROR_FILE_NOT_FOUND`) | `Ok(None)` — fall through to per-user config |
//! | Key **present but unreadable** (e.g. `ERROR_ACCESS_DENIED`) | `Err(NonoError::PolicyLoadFailed)` |
//! | Key **present but malformed** (wrong REG_* type, bad UTF-16) | `Err(NonoError::PolicyLoadFailed)` |
//!
//! Once the HKLM key exists, **any** read or parse failure aborts.  It is
//! never permissible to fall through to per-user configuration when the key
//! is present but unreadable — that would be a fail-open vulnerability.
//!
//! # Platform Notes
//!
//! Only the *reader* is `#[cfg(target_os = "windows")]`.  The
//! [`MachineEgressPolicy`] type is platform-neutral so the workspace
//! cross-compiles on Linux and macOS (Pitfall 5, EGRESS-03).  The non-Windows
//! stub returns `Ok(None)`.
//!
//! # 64-bit Registry View (D-09)
//!
//! The Windows reader always opens the key with `KEY_WOW64_64KEY` so that a
//! 32-bit Intune MDM extension writing to `WOW6432Node` cannot make the key
//! appear absent.

use serde::{Deserialize, Serialize};

use crate::Result;

/// Platform-neutral representation of the machine-level egress policy.
///
/// Populated by [`read_machine_egress_policy`] from `HKLM\SOFTWARE\Policies\nono`.
///
/// The type intentionally contains only `Vec<String>` fields so that it
/// compiles on every platform and can be serialized/deserialized for testing
/// and IPC without pulling in any Windows-only types.
///
/// Preset token→FQDN expansion happens in the CLI layer (Plan 03), which
/// has access to the embedded `policy.json`.  This type carries the raw preset
/// *tokens* only.
///
/// # Phase 84 Extension Note
///
/// This struct is designed to be extended with a `telemetry` section for
/// Phase 84 without re-architecting the single startup read (D-06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MachineEgressPolicy {
    /// Wildcard FQDN suffixes the admin allows egress to, e.g. `*.anthropic.com`.
    ///
    /// Populated from `AllowedSuffixes\` sub-key values (N × REG_SZ, ADMX `<list>` shape).
    #[serde(default)]
    pub allowed_suffixes: Vec<String>,

    /// Exact FQDN hosts the admin allows egress to, e.g. `api.github.com`.
    ///
    /// Populated from `AllowedHosts\` sub-key values (N × REG_SZ, ADMX `<list>` shape).
    #[serde(default)]
    pub allowed_hosts: Vec<String>,

    /// Group preset tokens, e.g. `"anthropic"`, `"openai"`.
    ///
    /// The CLI layer (Plan 03) expands these tokens to FQDNs using the embedded
    /// `policy.json` group map.  This type carries only the raw tokens so the
    /// library stays policy-free (CLAUDE.md § Library vs CLI Boundary).
    #[serde(default)]
    pub preset_tokens: Vec<String>,
}

impl MachineEgressPolicy {
    /// Returns the raw allowlist entries (suffixes + exact hosts) as a flat `Vec<String>`.
    ///
    /// Preset tokens are **not** expanded here; call the CLI-layer expansion to
    /// obtain the full FQDN set (Plan 03).  The returned list is suitable for
    /// passing directly to [`nono::HostFilter`] as the base entries once the
    /// CLI layer has appended the expanded preset FQDNs.
    ///
    /// # Suffix normalization (CR-01)
    ///
    /// The ADMX template instructs admins to enter `AllowedSuffixes` with a
    /// leading dot and no `*` (e.g. `.anthropic.com`).  [`crate::HostFilter`]
    /// only buckets entries that start with `*` as wildcard suffixes — a bare
    /// `.anthropic.com` would otherwise be treated as an *exact* host and match
    /// nothing.  To honor the ADMX-documented format and keep the EGRESS-03
    /// contract intact, `allowed_suffixes` entries are normalized here to the
    /// `*.`-prefixed wildcard form `HostFilter` understands:
    ///
    /// - `*.x.com`  → kept as-is
    /// - `.x.com`   → `*.x.com`
    /// - bare `x.com` → `*.x.com` (treated as a suffix per ADMX intent for this list)
    ///
    /// `allowed_hosts` (exact) entries are left untouched.  Suffixes precede
    /// hosts in the returned list.
    #[must_use]
    pub fn raw_allowlist(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(
            self.allowed_suffixes
                .len()
                .saturating_add(self.allowed_hosts.len()),
        );
        out.extend(self.allowed_suffixes.iter().map(|s| Self::normalize_suffix(s)));
        out.extend(self.allowed_hosts.iter().cloned());
        out
    }

    /// Normalize a single `AllowedSuffixes` entry to the `*.`-prefixed wildcard
    /// form that [`crate::HostFilter`] buckets as a suffix (CR-01).
    ///
    /// - `*.x.com`  → `*.x.com` (unchanged)
    /// - `.x.com`   → `*.x.com`
    /// - bare `x.com` → `*.x.com`
    fn normalize_suffix(s: &str) -> String {
        if s.starts_with("*.") {
            s.to_string()
        } else if let Some(rest) = s.strip_prefix('.') {
            format!("*.{rest}")
        } else {
            format!("*.{s}")
        }
    }
}

// ── Windows reader ────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod windows_reader {
    use super::{MachineEgressPolicy, Result};
    use crate::NonoError;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    use winreg::{RegKey, RegValue};

    /// Win32 `ERROR_FILE_NOT_FOUND` — key or sub-key **absent** (D-07 fall-through).
    const ERROR_FILE_NOT_FOUND: i32 = 2;

    /// Convert a `RegValue` to a `String`, failing with a reason string if the
    /// value is not `REG_SZ` or contains invalid UTF-16.
    fn reg_value_to_string(val: &RegValue, context: &str) -> std::result::Result<String, String> {
        use winreg::enums::RegType;
        use winreg::types::FromRegValue;
        if val.vtype != RegType::REG_SZ {
            return Err(format!(
                "{context}: expected REG_SZ, got {:?} (malformed — D-07 abort)",
                val.vtype
            ));
        }
        String::from_reg_value(val)
            .map_err(|e| format!("{context}: REG_SZ to String failed (bad UTF-16?): {e}"))
    }

    /// Enumerate an ADMX `<list>` sub-key as N × REG_SZ values.
    ///
    /// Returns `Ok(Vec::new())` if the sub-key is absent (parent key present is
    /// what gates enforcement).  Returns `Err(reason)` for any other error or
    /// if any value has the wrong REG type (D-07 malformed → abort).
    pub(super) fn read_list_subkey(
        parent: &RegKey,
        name: &str,
    ) -> std::result::Result<Vec<String>, String> {
        let sub = match parent.open_subkey_with_flags(name, KEY_READ | KEY_WOW64_64KEY) {
            Ok(k) => k,
            Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => {
                // Sub-key absent is fine; the parent key existing is what gates enforcement.
                return Ok(Vec::new());
            }
            Err(e) => return Err(format!("open sub-key `{name}`: {e}")),
        };

        let mut out = Vec::new();
        for item in sub.enum_values() {
            let (_vname, val) = item.map_err(|e| format!("enum_values `{name}`: {e}"))?;
            let s = reg_value_to_string(&val, name)?;
            out.push(s);
        }
        Ok(out)
    }

    /// Read the preset-token sub-key (`PresetTokens\`) as N × REG_SZ.
    ///
    /// Same absent-is-ok / wrong-type-is-abort semantics.
    pub(super) fn read_preset_subkey(
        parent: &RegKey,
        name: &str,
    ) -> std::result::Result<Vec<String>, String> {
        read_list_subkey(parent, name)
    }

    /// Inner parser: read all sub-keys from an already-opened policy `RegKey`.
    pub(super) fn parse_policy(key: &RegKey) -> std::result::Result<MachineEgressPolicy, String> {
        let allowed_suffixes = read_list_subkey(key, "AllowedSuffixes")?;
        let allowed_hosts = read_list_subkey(key, "AllowedHosts")?;
        let preset_tokens = read_preset_subkey(key, "PresetTokens")?;
        Ok(MachineEgressPolicy {
            allowed_suffixes,
            allowed_hosts,
            preset_tokens,
        })
    }

    /// Read `HKLM\SOFTWARE\Policies\nono` and deserialize into
    /// [`MachineEgressPolicy`].
    ///
    /// # Fail-Secure Taxonomy (D-07)
    ///
    /// - Key **absent** (`ERROR_FILE_NOT_FOUND=2`) → `Ok(None)`.
    /// - Key **present but unreadable** (any other OS error) →
    ///   `Err(NonoError::PolicyLoadFailed)`.
    /// - Key **present but malformed** (wrong REG_* type, bad UTF-16) →
    ///   `Err(NonoError::PolicyLoadFailed)`.
    ///
    /// Never use `unwrap_or` / `unwrap_or_default` / `.ok()` on the read path —
    /// every non-absent error propagates as `PolicyLoadFailed` (Pitfall 3).
    pub fn read_machine_egress_policy_impl() -> Result<Option<MachineEgressPolicy>> {
        const POLICY_PATH: &str = r"SOFTWARE\Policies\nono";

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        // D-09: KEY_WOW64_64KEY forces the 64-bit view regardless of process bitness.
        let key = match hklm.open_subkey_with_flags(POLICY_PATH, KEY_READ | KEY_WOW64_64KEY) {
            Ok(k) => k,
            Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => {
                // Key absent → fall through to per-user config (D-07).
                return Ok(None);
            }
            Err(e) => {
                // Key present but unreadable (ACCESS_DENIED=5, etc.) → abort (D-07).
                return Err(NonoError::PolicyLoadFailed {
                    reason: format!("machine policy key present but unreadable: {e}"),
                });
            }
        };

        // Any malformed shape (wrong REG_* type, bad UTF-16) → abort (D-07).
        let policy = parse_policy(&key).map_err(|reason| NonoError::PolicyLoadFailed { reason })?;
        Ok(Some(policy))
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Read the machine-level egress policy from `HKLM\SOFTWARE\Policies\nono`.
///
/// # Return values
///
/// | Condition | Return |
/// |-----------|--------|
/// | Key absent | `Ok(None)` — caller falls through to per-user config |
/// | Key present, readable, valid | `Ok(Some(policy))` |
/// | Key present but unreadable or malformed | `Err(NonoError::PolicyLoadFailed)` |
///
/// # Fail-secure contract
///
/// Once the HKLM key exists, **any** error returns `Err(PolicyLoadFailed)` and
/// the caller MUST NOT fall through to per-user configuration (D-07).
/// Use the `?` operator at the call site — never `.ok()` or `unwrap_or`.
///
/// # Non-Windows
///
/// Returns `Ok(None)` unconditionally; no registry access is attempted.
#[cfg(target_os = "windows")]
pub fn read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>> {
    windows_reader::read_machine_egress_policy_impl()
}

/// Non-Windows stub: returns `Ok(None)` (no HKLM registry on Linux/macOS).
#[cfg(not(target_os = "windows"))]
pub fn read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>> {
    Ok(None)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{NonoError, Result};

    // ── Platform-neutral serde / type tests ──────────────────────────────────

    #[test]
    fn empty_policy_deserializes_and_raw_allowlist_is_empty() {
        let policy = MachineEgressPolicy::default();
        assert!(policy.raw_allowlist().is_empty());
    }

    #[test]
    fn policy_serde_round_trip() {
        let original = MachineEgressPolicy {
            allowed_suffixes: vec!["*.anthropic.com".to_string(), "*.openai.com".to_string()],
            allowed_hosts: vec!["api.github.com".to_string()],
            preset_tokens: vec!["anthropic".to_string()],
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: MachineEgressPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn raw_allowlist_concatenates_suffixes_then_hosts() {
        let policy = MachineEgressPolicy {
            // Already in `*.`-prefixed wildcard form — must pass through unchanged.
            allowed_suffixes: vec!["*.anthropic.com".to_string()],
            allowed_hosts: vec!["api.github.com".to_string()],
            preset_tokens: vec![],
        };
        let list = policy.raw_allowlist();
        assert_eq!(list, vec!["*.anthropic.com", "api.github.com"]);
    }

    /// CR-01: `AllowedSuffixes` entries are normalized to the `*.`-prefixed
    /// wildcard form that `HostFilter` buckets as a suffix, regardless of the
    /// ADMX-documented leading-dot or bare-domain input shape.
    #[test]
    fn raw_allowlist_normalizes_suffix_shapes() {
        let policy = MachineEgressPolicy {
            allowed_suffixes: vec![
                "*.anthropic.com".to_string(), // wildcard → unchanged
                ".openai.com".to_string(),     // leading-dot → *.openai.com
                "github.com".to_string(),      // bare → *.github.com
            ],
            allowed_hosts: vec!["api.exact.com".to_string()],
            preset_tokens: vec![],
        };
        let list = policy.raw_allowlist();
        assert_eq!(
            list,
            vec![
                "*.anthropic.com",
                "*.openai.com",
                "*.github.com",
                "api.exact.com",
            ]
        );
    }

    /// CR-01 contract lock: a leading-dot `.anthropic.com` suffix (the
    /// ADMX-documented `AllowedSuffixes` shape) must flow through
    /// `raw_allowlist()` → `HostFilter::new_strict` and match subdomains
    /// component-safely, mirroring `net_filter::sc4_dns_component_matrix`.
    #[test]
    fn cr01_leading_dot_suffix_matches_via_hostfilter() {
        use crate::net_filter::HostFilter;
        use std::net::{IpAddr, Ipv4Addr};

        let policy = MachineEgressPolicy {
            // Leading-dot form exactly as the ADMX instructs admins to enter.
            allowed_suffixes: vec![".anthropic.com".to_string()],
            allowed_hosts: vec![],
            preset_tokens: vec![],
        };
        let allowlist = policy.raw_allowlist();
        assert_eq!(
            allowlist,
            vec!["*.anthropic.com"],
            "leading-dot suffix must normalize to *.anthropic.com"
        );

        let filter = HostFilter::new_strict(&allowlist);
        let ip = vec![IpAddr::V4(Ipv4Addr::new(104, 18, 7, 96))];

        // Subdomain — must be allowed.
        assert!(
            filter.check_host("api.anthropic.com", &ip).is_allowed(),
            "api.anthropic.com must be allowed by a .anthropic.com suffix"
        );
        // Bare domain — wildcard must not match parent.
        assert!(
            !filter.check_host("anthropic.com", &ip).is_allowed(),
            "anthropic.com (bare) must NOT be allowed"
        );
        // No leading-dot boundary — must be rejected.
        assert!(
            !filter.check_host("evilanthropic.com", &ip).is_allowed(),
            "evilanthropic.com must NOT be allowed (no boundary)"
        );
        // Suffix-injection — must be rejected.
        assert!(
            !filter.check_host("anthropic.com.evil.com", &ip).is_allowed(),
            "anthropic.com.evil.com must NOT be allowed (suffix injection)"
        );
    }

    #[test]
    fn non_windows_stub_returns_ok_none() {
        // On the dev host (Windows) this calls the real reader which will return
        // Ok(None) if the HKLM key is absent — that's the same contract the stub
        // satisfies on Linux/macOS.  The test asserts the stub contract (no Err).
        let result = read_machine_egress_policy();
        // Either Ok(None) (key absent / non-Windows) or Ok(Some(_)) (key present on CI);
        // Err is a fail.
        assert!(result.is_ok(), "read_machine_egress_policy must not return Err on a host without the policy key: {result:?}");
    }

    #[test]
    fn policy_load_failed_display_contains_reason() {
        let err = NonoError::PolicyLoadFailed {
            reason: "test reason string".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("test reason string"),
            "Display must contain the reason; got: {msg}"
        );
    }

    #[test]
    fn policy_load_failed_is_pattern_matchable() {
        let err = NonoError::PolicyLoadFailed {
            reason: "x".to_string(),
        };
        assert!(
            matches!(err, NonoError::PolicyLoadFailed { .. }),
            "PolicyLoadFailed must be pattern-matchable"
        );
    }

    #[test]
    fn policy_load_failed_propagates_via_result_alias() {
        fn producer() -> Result<()> {
            Err(NonoError::PolicyLoadFailed {
                reason: "propagation test".to_string(),
            })
        }
        let err = producer().expect_err("must error");
        assert!(matches!(err, NonoError::PolicyLoadFailed { .. }));
    }

    // ── Windows-only integration tests ────────────────────────────────────────

    /// Seed a temp key under HKCU (writable without elevation) and verify
    /// the list-subkey enumerator returns the seeded REG_SZ values.
    ///
    /// This exercises the `read_list_subkey` logic without needing HKLM admin.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_list_subkey_reads_reg_sz_values() {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_WOW64_64KEY};
        use winreg::RegKey;

        // Create a temp test key under HKCU.
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (test_key, _disp) = hkcu
            .create_subkey_with_flags(
                r"SOFTWARE\nono-test\machine_policy_test\AllowedSuffixes",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();

        // Write two REG_SZ values (the way ADMX <list> materializes them).
        test_key.set_value("1", &"*.anthropic.com".to_string()).unwrap();
        test_key.set_value("2", &"*.openai.com".to_string()).unwrap();
        drop(test_key);

        // Open the parent and call read_list_subkey.
        let parent = hkcu
            .open_subkey_with_flags(
                r"SOFTWARE\nono-test\machine_policy_test",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();

        let values = super::windows_reader::read_list_subkey(&parent, "AllowedSuffixes").unwrap();
        assert!(
            values.contains(&"*.anthropic.com".to_string()),
            "Missing *.anthropic.com; got: {values:?}"
        );
        assert!(
            values.contains(&"*.openai.com".to_string()),
            "Missing *.openai.com; got: {values:?}"
        );

        // Cleanup.
        hkcu.delete_subkey_all(r"SOFTWARE\nono-test\machine_policy_test")
            .unwrap();
        let _ = hkcu.delete_subkey(r"SOFTWARE\nono-test");
    }

    /// A value with a non-REG_SZ type in a list sub-key is MALFORMED → Err (D-07).
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_wrong_reg_type_returns_policy_load_failed() {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_WOW64_64KEY};
        use winreg::{RegKey, RegValue};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (test_key, _disp) = hkcu
            .create_subkey_with_flags(
                r"SOFTWARE\nono-test\wrong_type_test\AllowedSuffixes",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();

        // Write a DWORD value — wrong type for a list entry.
        let raw_dword = RegValue {
            bytes: std::borrow::Cow::Owned(vec![1u8, 0, 0, 0]),
            vtype: winreg::enums::RegType::REG_DWORD,
        };
        test_key.set_raw_value("1", &raw_dword).unwrap();
        drop(test_key);

        let parent = hkcu
            .open_subkey_with_flags(
                r"SOFTWARE\nono-test\wrong_type_test",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();

        let result = super::windows_reader::read_list_subkey(&parent, "AllowedSuffixes");
        assert!(
            result.is_err(),
            "Wrong REG type must return Err (malformed); got: {result:?}"
        );
        let reason = result.unwrap_err();
        assert!(
            reason.contains("malformed") || reason.contains("REG_SZ"),
            "Error reason must mention malformed or REG_SZ; got: {reason}"
        );

        // Cleanup.
        hkcu.delete_subkey_all(r"SOFTWARE\nono-test\wrong_type_test")
            .unwrap();
        let _ = hkcu.delete_subkey(r"SOFTWARE\nono-test");
    }
}
