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
//! | Key **present but unconfigured** (only the MSI `InstalledByMsi` sentinel; no egress entries) | `Ok(None)` — not enforcing → fall through to per-user config (CR-02) |
//! | Key **present WITH ≥1 egress entry** (suffix/host/preset) | `Ok(Some(policy))` — enforcement active |
//! | Key **present but unreadable** (e.g. `ERROR_ACCESS_DENIED`) | `Err(NonoError::PolicyLoadFailed)` |
//! | Key **present but malformed** (wrong REG_* type, bad UTF-16) | `Err(NonoError::PolicyLoadFailed)` |
//!
//! Once the HKLM key exists, **any** read or parse failure aborts.  It is
//! never permissible to fall through to per-user configuration when the key
//! is present but unreadable — that would be a fail-open vulnerability.
//!
//! Enforcement is gated on configured *content*, not mere key presence
//! (CR-02): the machine MSI always creates the key with an `InstalledByMsi`
//! sentinel value, so a bare install with no GPO configured would otherwise
//! flip every confined agent to strict deny-all egress.  A present-but-empty
//! policy (no `AllowedSuffixes`/`AllowedHosts`/`PresetTokens` entries) is
//! treated as "present but unconfigured = not enforcing" → `Ok(None)`.  A
//! malformed value is still a hard abort, never a fall-through.
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

// ── Telemetry configuration types (Phase 84 D-12/D-13/D-14) ──────────────────

/// Minimum severity level for security-event telemetry emission.
///
/// Controls which security events are forwarded to the configured channel.
/// Default is `Warning` (D-13): informational events are suppressed by default
/// to reduce noise in a fleet deployment.
///
/// Severity ordering is `Debug < Info < Warning < Error`, derived from variant
/// declaration order via `PartialOrd`/`Ord`. The telemetry layer compares an
/// event's severity against `min_severity` with this ordering (WR-02 / TELEM-04
/// level filtering): an event emits only when `event_severity >= min_severity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum TelemetrySeverity {
    /// Emit all security events (PathDeny, NetworkDeny, LabelViolation, …).
    Debug,
    /// Emit informational-level and above events.
    Info,
    /// Emit warning-level and above events (D-13 default).
    #[default]
    Warning,
    /// Emit only error-level events.
    Error,
}

fn default_telemetry_enabled() -> bool {
    true
}

fn default_telemetry_channel() -> String {
    "Application".to_string()
}

fn default_telemetry_min_severity() -> TelemetrySeverity {
    TelemetrySeverity::Warning
}

/// Telemetry sub-section of [`MachineEgressPolicy`] (D-12).
///
/// Read from `HKLM\SOFTWARE\Policies\nono\Telemetry\` during the **same
/// single registry read** Phase 83 performs — no second registry read (D-12).
///
/// # Default-ON semantics (D-13)
///
/// When the HKLM policy key is absent, all three fields default to the
/// most-useful value: **enabled → Application log**.  A clean-host MSI install
/// with no GPO configured must emit telemetry by default; a default-OFF policy
/// would make the SC-1/SC-5 gate fail on every fresh install.
///
/// # Degrade-not-abort (D-14)
///
/// Malformed telemetry registry values fall back to this struct's `Default`
/// and surface a `TelemetryConfigInvalid` warning to stderr.  They do NOT
/// return `Err` — contrast with `PolicyLoadFailed` for egress (D-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether security-event telemetry is enabled.
    ///
    /// Defaults to `true` (D-13 — security telemetry on by default; admins opt out).
    #[serde(default = "default_telemetry_enabled")]
    pub enabled: bool,

    /// Windows Event Log channel to write to.
    ///
    /// Default is `"Application"` (D-01.2 — the Phase-82-registered Application
    /// source; no `wevtutil im` required).
    #[serde(default = "default_telemetry_channel")]
    pub channel: String,

    /// Minimum severity level to emit.
    ///
    /// Default is `Warning` (D-13).
    #[serde(default = "default_telemetry_min_severity")]
    pub min_severity: TelemetrySeverity,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_telemetry_enabled(),
            channel: default_telemetry_channel(),
            min_severity: default_telemetry_min_severity(),
        }
    }
}

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

    /// Telemetry sub-section (Phase 84 D-12).
    ///
    /// Populated from `HKLM\SOFTWARE\Policies\nono\Telemetry\` during the same
    /// single registry read that populates the egress fields.  Defaults to
    /// [`TelemetryConfig::default()`] (enabled → Application log, Warning level)
    /// when absent or malformed — see D-13 and D-14.
    ///
    /// **INVARIANT:** This field MUST NOT be counted in [`Self::is_unconfigured`].
    /// A telemetry-only HKLM write must not flip the daemon to strict deny-all
    /// egress (84-PATTERNS.md invariant 3 / CR-02).
    #[serde(default)]
    pub telemetry: TelemetryConfig,
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

    /// Whether this policy carries no configured egress entries (CR-02).
    ///
    /// Returns `true` when `allowed_suffixes`, `allowed_hosts`, and
    /// `preset_tokens` are all empty — i.e. the `HKLM` key exists but contains
    /// only the MSI `InstalledByMsi` sentinel.  The Windows reader maps this to
    /// `Ok(None)` so a bare machine-MSI install (no GPO) does not flip the
    /// daemon to strict deny-all egress.
    ///
    /// **The `telemetry` sub-section is deliberately NOT counted here** (Phase 84
    /// D-12 / 84-PATTERNS.md invariant 3).  A policy with only telemetry fields
    /// set must still return `true` so the egress-enforcement gate is not
    /// accidentally tripped by a telemetry-only GPO write.
    #[must_use]
    pub fn is_unconfigured(&self) -> bool {
        self.allowed_suffixes.is_empty()
            && self.allowed_hosts.is_empty()
            && self.preset_tokens.is_empty()
        // NOTE: telemetry config MUST NOT be counted here — see Phase 84 D-12 and
        // 84-PATTERNS.md invariant 3.
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
    use super::{MachineEgressPolicy, Result, TelemetryConfig, TelemetrySeverity};
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

    /// Read an optional `REG_DWORD` value from a key.
    ///
    /// Returns `None` if the value is absent (sub-key or value not found).
    /// Returns `Err(reason)` if the value is present but has the wrong type.
    fn read_optional_dword(key: &RegKey, name: &str) -> std::result::Result<Option<u32>, String> {
        match key.get_value::<u32, _>(name) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => Ok(None),
            Err(e) => Err(format!("read DWORD `{name}`: {e}")),
        }
    }

    /// Read an optional `REG_SZ` value from a key.
    ///
    /// Returns `None` if the value is absent.
    /// Returns `Err(reason)` if present but wrong type or invalid UTF-16.
    fn read_optional_sz(key: &RegKey, name: &str) -> std::result::Result<Option<String>, String> {
        match key.get_value::<String, _>(name) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => Ok(None),
            Err(e) => Err(format!("read REG_SZ `{name}`: {e}")),
        }
    }

    /// Parse the `Telemetry\` sub-section from an already-opened policy key.
    ///
    /// Per D-14: malformed telemetry values degrade to `TelemetryConfig::default()` and
    /// emit a warning to stderr — they do NOT propagate `Err` to the egress-abort path.
    pub(super) fn parse_telemetry_config(key: &RegKey) -> TelemetryConfig {
        // Open Telemetry\ sub-key; absent = fine (use defaults, D-13).
        let telem_key = match key.open_subkey_with_flags("Telemetry", KEY_READ | KEY_WOW64_64KEY) {
            Ok(k) => k,
            Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => {
                return TelemetryConfig::default();
            }
            Err(e) => {
                // Present but unreadable — degrade, do not abort (D-14).
                eprintln!(
                    "[nono] TelemetryConfigInvalid: cannot open Telemetry sub-key: {e}; \
                     falling back to defaults"
                );
                return TelemetryConfig::default();
            }
        };

        let enabled = match read_optional_dword(&telem_key, "TelemetryEnabled") {
            Ok(Some(v)) => v != 0,
            Ok(None) => true, // absent → default ON (D-13)
            Err(e) => {
                eprintln!(
                    "[nono] TelemetryConfigInvalid: TelemetryEnabled malformed ({e}); \
                     defaulting to enabled=true"
                );
                true
            }
        };

        let channel = match read_optional_sz(&telem_key, "TelemetryChannel") {
            Ok(Some(v)) if !v.is_empty() => v,
            Ok(_) => "Application".to_string(), // absent or empty → default
            Err(e) => {
                eprintln!(
                    "[nono] TelemetryConfigInvalid: TelemetryChannel malformed ({e}); \
                     defaulting to Application"
                );
                "Application".to_string()
            }
        };

        let min_severity = match read_optional_sz(&telem_key, "TelemetryMinSeverity") {
            Ok(Some(s)) => match s.to_lowercase().as_str() {
                "debug" => TelemetrySeverity::Debug,
                "info" => TelemetrySeverity::Info,
                "warning" | "warn" => TelemetrySeverity::Warning,
                "error" => TelemetrySeverity::Error,
                other => {
                    eprintln!(
                        "[nono] TelemetryConfigInvalid: unknown TelemetryMinSeverity \
                         value {:?}; defaulting to Warning",
                        other
                    );
                    TelemetrySeverity::Warning
                }
            },
            Ok(None) => TelemetrySeverity::Warning, // absent → default
            Err(e) => {
                eprintln!(
                    "[nono] TelemetryConfigInvalid: TelemetryMinSeverity malformed ({e}); \
                     defaulting to Warning"
                );
                TelemetrySeverity::Warning
            }
        };

        TelemetryConfig {
            enabled,
            channel,
            min_severity,
        }
    }

    /// Inner parser: read all sub-keys from an already-opened policy `RegKey`.
    pub(super) fn parse_policy(key: &RegKey) -> std::result::Result<MachineEgressPolicy, String> {
        let allowed_suffixes = read_list_subkey(key, "AllowedSuffixes")?;
        let allowed_hosts = read_list_subkey(key, "AllowedHosts")?;
        let preset_tokens = read_preset_subkey(key, "PresetTokens")?;
        // D-12: read telemetry sub-section in the same registry open.
        // D-14: malformed telemetry degrades; only egress errors abort (D-07).
        let telemetry = parse_telemetry_config(key);
        Ok(MachineEgressPolicy {
            allowed_suffixes,
            allowed_hosts,
            preset_tokens,
            telemetry,
        })
    }

    /// Read `HKLM\SOFTWARE\Policies\nono` and deserialize into
    /// [`MachineEgressPolicy`].
    ///
    /// # Fail-Secure Taxonomy (D-07, CR-02)
    ///
    /// - Key **absent** (`ERROR_FILE_NOT_FOUND=2`) → `Ok(None)`.
    /// - Key **present but unconfigured** (only the MSI `InstalledByMsi`
    ///   sentinel; no egress entries) → `Ok(None)` (CR-02).
    /// - Key **present WITH ≥1 egress entry** → `Ok(Some(policy))`.
    /// - Key **present but unreadable** (any other OS error) →
    ///   `Err(NonoError::PolicyLoadFailed)`.
    /// - Key **present but malformed** (wrong REG_* type, bad UTF-16) →
    ///   `Err(NonoError::PolicyLoadFailed)`.
    ///
    /// Never use `unwrap_or` / `unwrap_or_default` / `.ok()` on the read path —
    /// every non-absent error propagates as `PolicyLoadFailed` (Pitfall 3).
    ///
    /// Enforcement is gated on configured content, not mere key presence: the
    /// machine MSI unconditionally creates the key with an `InstalledByMsi`
    /// sentinel value, so a present-but-empty policy must NOT activate strict
    /// deny-all (CR-02).  A malformed value still aborts — only a cleanly-read
    /// but empty policy falls through.
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

        // CR-02: gate enforcement on configured content, not mere key presence.
        // The MSI always creates the key with only an `InstalledByMsi` sentinel
        // value (no egress sub-keys).  A present-but-unconfigured policy is
        // "not enforcing" → fall through to per-user config.  This is reached
        // only after a clean parse, so a malformed value has already aborted.
        if policy.is_unconfigured() {
            return Ok(None);
        }

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
/// | Key present but unconfigured (sentinel only, no egress entries) | `Ok(None)` — not enforcing → per-user fall-through (CR-02) |
/// | Key present WITH ≥1 egress entry, readable, valid | `Ok(Some(policy))` |
/// | Key present but unreadable or malformed | `Err(NonoError::PolicyLoadFailed)` |
///
/// # Fail-secure contract
///
/// Once the HKLM key exists, **any** read or parse error returns
/// `Err(PolicyLoadFailed)` and the caller MUST NOT fall through to per-user
/// configuration (D-07).  A *cleanly-read but empty* policy (only the MSI
/// sentinel value, no configured egress entries) is the one exception: it is
/// "present but unconfigured = not enforcing" → `Ok(None)` (CR-02).  A
/// malformed value still aborts.  Use the `?` operator at the call site —
/// never `.ok()` or `unwrap_or`.
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

    // ── Phase 84 TelemetryConfig / TelemetrySeverity tests ───────────────────

    /// D-13: TelemetryConfig::default() must have enabled=true, channel="Application",
    /// min_severity=Warning.
    #[test]
    fn telemetry_config_default_has_expected_values() {
        let cfg = TelemetryConfig::default();
        assert!(cfg.enabled, "D-13: telemetry must default to enabled");
        assert_eq!(
            cfg.channel, "Application",
            "D-13: default channel must be Application"
        );
        assert_eq!(
            cfg.min_severity,
            TelemetrySeverity::Warning,
            "D-13: default min_severity must be Warning"
        );
    }

    /// WR-02 / TELEM-04: TelemetrySeverity must order Debug < Info < Warning < Error
    /// so the telemetry layer can compare an event's severity against min_severity.
    #[test]
    fn telemetry_severity_orders_debug_to_error() {
        assert!(TelemetrySeverity::Debug < TelemetrySeverity::Info);
        assert!(TelemetrySeverity::Info < TelemetrySeverity::Warning);
        assert!(TelemetrySeverity::Warning < TelemetrySeverity::Error);
        // Default (Warning) must be >= itself and emit at the default threshold.
        assert!(TelemetrySeverity::Warning >= TelemetrySeverity::default());
    }

    /// Invariant 3 (84-PATTERNS.md / CR-02): MachineEgressPolicy with ONLY telemetry
    /// fields populated must still return is_unconfigured()=true.
    #[test]
    fn is_unconfigured_ignores_telemetry_field() {
        // A policy that has ONLY the telemetry section set (all egress lists empty).
        let policy = MachineEgressPolicy {
            allowed_suffixes: vec![],
            allowed_hosts: vec![],
            preset_tokens: vec![],
            telemetry: TelemetryConfig {
                enabled: true,
                channel: "Application".to_string(),
                min_severity: TelemetrySeverity::Debug,
            },
        };
        assert!(
            policy.is_unconfigured(),
            "is_unconfigured must ignore telemetry (invariant 3 / CR-02); policy: {policy:?}"
        );
    }

    /// D-14: TelemetryConfigInvalid variant displays the reason string.
    #[test]
    fn telemetry_config_invalid_display_contains_reason() {
        let err = NonoError::TelemetryConfigInvalid {
            reason: "bad REG_SZ for TelemetryChannel".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("bad REG_SZ for TelemetryChannel"),
            "TelemetryConfigInvalid display must contain reason; got: {msg}"
        );
        assert!(
            msg.contains("invalid") || msg.contains("Telemetry config"),
            "Display should mention telemetry config invalid; got: {msg}"
        );
    }

    /// D-03: TelemetryUnavailable variant displays the reason string.
    #[test]
    fn telemetry_unavailable_display_contains_reason() {
        let err = NonoError::TelemetryUnavailable {
            reason: "RegisterEventSourceW returned null".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("RegisterEventSourceW returned null"),
            "TelemetryUnavailable display must contain reason; got: {msg}"
        );
    }

    /// Phase 84: serde round-trip for the full MachineEgressPolicy including telemetry.
    #[test]
    fn policy_serde_round_trip_with_telemetry() {
        let original = MachineEgressPolicy {
            allowed_suffixes: vec!["*.anthropic.com".to_string()],
            allowed_hosts: vec!["api.github.com".to_string()],
            preset_tokens: vec!["anthropic".to_string()],
            telemetry: TelemetryConfig {
                enabled: false,
                channel: "Security".to_string(),
                min_severity: TelemetrySeverity::Error,
            },
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: MachineEgressPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored, "serde round-trip must preserve all fields");
    }

    /// Phase 84: deserializing a policy JSON with no telemetry section must
    /// produce TelemetryConfig::default() (via #[serde(default)]).
    #[test]
    fn policy_serde_without_telemetry_uses_defaults() {
        let json = r#"{"allowed_suffixes":[],"allowed_hosts":[],"preset_tokens":[]}"#;
        let policy: MachineEgressPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(
            policy.telemetry,
            TelemetryConfig::default(),
            "missing telemetry section must deserialize to default"
        );
    }

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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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

    // ── CR-02: enforcement gated on configured content, not key presence ──────

    #[test]
    fn is_unconfigured_true_for_empty_policy() {
        assert!(MachineEgressPolicy::default().is_unconfigured());
    }

    #[test]
    fn is_unconfigured_false_with_any_entry() {
        let suffix_only = MachineEgressPolicy {
            allowed_suffixes: vec![".anthropic.com".to_string()],
            ..Default::default()
        };
        assert!(!suffix_only.is_unconfigured());

        let host_only = MachineEgressPolicy {
            allowed_hosts: vec!["api.github.com".to_string()],
            ..Default::default()
        };
        assert!(!host_only.is_unconfigured());

        let preset_only = MachineEgressPolicy {
            preset_tokens: vec!["anthropic".to_string()],
            ..Default::default()
        };
        assert!(!preset_only.is_unconfigured());
    }

    /// CR-02: a sentinel-only key (created by the MSI, no `AllowedSuffixes`/
    /// `AllowedHosts`/`PresetTokens` sub-keys) parses to an *unconfigured*
    /// policy, which the reader maps to `Ok(None)` → per-user fall-through.
    /// Exercised against an HKCU-seeded key (no HKLM admin needed); this
    /// mirrors the `is_unconfigured()` gate the Windows reader applies after
    /// a clean `parse_policy`.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_sentinel_only_key_is_unconfigured() {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_WOW64_64KEY};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        // Create the policy key with ONLY the MSI sentinel value — no sub-keys.
        let (key, _disp) = hkcu
            .create_subkey_with_flags(
                r"SOFTWARE\nono-test\sentinel_only_test",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();
        key.set_value("InstalledByMsi", &"1".to_string()).unwrap();
        drop(key);

        let policy_key = hkcu
            .open_subkey_with_flags(
                r"SOFTWARE\nono-test\sentinel_only_test",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();

        let policy = super::windows_reader::parse_policy(&policy_key).unwrap();
        assert!(
            policy.is_unconfigured(),
            "sentinel-only key must parse to an unconfigured policy (→ Ok(None)); got: {policy:?}"
        );

        // Cleanup.
        hkcu.delete_subkey_all(r"SOFTWARE\nono-test\sentinel_only_test")
            .unwrap();
        let _ = hkcu.delete_subkey(r"SOFTWARE\nono-test");
    }

    /// CR-02 control: a key WITH one `AllowedSuffixes` value parses to a
    /// *configured* policy (→ `Ok(Some(...))` = enforcement active).
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_configured_key_is_not_unconfigured() {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_WOW64_64KEY};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (sub, _disp) = hkcu
            .create_subkey_with_flags(
                r"SOFTWARE\nono-test\configured_test\AllowedSuffixes",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();
        sub.set_value("1", &".anthropic.com".to_string()).unwrap();
        drop(sub);

        let policy_key = hkcu
            .open_subkey_with_flags(
                r"SOFTWARE\nono-test\configured_test",
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
            )
            .unwrap();

        let policy = super::windows_reader::parse_policy(&policy_key).unwrap();
        assert!(
            !policy.is_unconfigured(),
            "key with an AllowedSuffixes value must be configured (→ Ok(Some)); got: {policy:?}"
        );
        assert_eq!(policy.allowed_suffixes, vec![".anthropic.com"]);

        // Cleanup.
        hkcu.delete_subkey_all(r"SOFTWARE\nono-test\configured_test")
            .unwrap();
        let _ = hkcu.delete_subkey(r"SOFTWARE\nono-test");
    }
}
