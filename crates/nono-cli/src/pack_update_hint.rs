//! Pack update hints for `nono run`.
//!
//! After the capabilities block, checks whether any pack-provided profile in
//! the active extends chain has a newer version available, and prints a one-line
//! hint if so. Results are cached per pack for 24 hours in the state directory
//! so the registry check never blocks startup. A background thread refreshes
//! stale entries for the next run.
//!
//! Respects the same opt-out as the CLI update check: `NONO_NO_UPDATE_CHECK=1`
//! or `[updates] check = false` in `~/.config/nono/config.toml`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;

const HINTS_STATE_FILE: &str = "pack-update-hints.json";
const CHECK_INTERVAL_SECS: i64 = 86400;

/// Per-pack cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackHintEntry {
    last_check: DateTime<Utc>,
    /// The installed version recorded at the time of the last check.
    installed_at_check: String,
    /// Latest registry version at check time. `None` if the check failed.
    latest: Option<String>,
}

/// Full cache state stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PackHintsState {
    /// Keyed by `"namespace/name"`.
    #[serde(default)]
    entries: HashMap<String, PackHintEntry>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Print update hints for every pack-provided profile in the active extends
/// chain, reading from a 24-hour local cache.
///
/// Silently no-ops on any error (network, I/O, parse). Stale cache entries
/// are refreshed in a background thread — startup latency is never blocked
/// on a registry lookup.
///
/// Phase 44 WR-05 P43 (REQ-REVIEW-FU-01 D-44-B2 option b): the pre-44
/// implementation took a synchronous-then-background branch on the
/// first-run cache-missing path. That branch could stall `nono run`
/// startup up to 5 minutes when the registry was unreachable, violating
/// CLAUDE.md § Performance "Zero startup latency". The synchronous path
/// has been removed entirely; first-run users see hints on the SECOND
/// `nono run` invocation (after the background refresh populates the
/// cache), which is preferable to a multi-minute first-run stall.
pub fn show_pack_update_hints(profile_name: &str, silent: bool) {
    if silent || is_opted_out() {
        return;
    }

    let packs = collect_profile_packs(profile_name);
    if packs.is_empty() {
        return;
    }

    let state = load_state();
    let now = Utc::now();

    let mut hints: Vec<(String, String, String)> = Vec::new(); // (pack_ref, installed, latest)
    let mut stale: Vec<(String, String)> = Vec::new(); // (pack_ref, installed)

    for (pack_ref, installed) in &packs {
        match state.entries.get(pack_ref) {
            Some(entry)
                if now.signed_duration_since(entry.last_check).num_seconds()
                    < CHECK_INTERVAL_SECS =>
            {
                // Cache is fresh — use it.
                if let Some(ref latest) = entry.latest {
                    if is_newer(installed, latest) {
                        hints.push((pack_ref.clone(), installed.clone(), latest.clone()));
                    }
                }
            }
            _ => {
                stale.push((pack_ref.clone(), installed.clone()));
            }
        }
    }

    if !stale.is_empty() {
        // Always background-refresh — first-run users see hints on the
        // 2nd `nono run` invocation rather than blocking startup on a
        // dead registry. Phase 44 WR-05 P43 (D-44-B2 option b);
        // CLAUDE.md § Performance "Zero startup latency".
        let shared = Arc::new(Mutex::new(state));
        refresh_in_background(stale, shared);
    }

    print_hints(&hints);
}

// ---------------------------------------------------------------------------
// Extends-chain pack collection
// ---------------------------------------------------------------------------

/// Walk the extends chain from `profile_name` and return
/// `(pack_ref, installed_version)` for each pack-provided profile encountered.
///
/// User and builtin profiles in the chain are walked but not collected — only
/// entries that map to an installed pack are returned.
fn collect_profile_packs(profile_name: &str) -> Vec<(String, String)> {
    let pack_map: HashMap<String, String> = crate::profile::list_pack_store_profiles()
        .into_iter()
        .collect();

    let lockfile = match crate::package::read_lockfile() {
        Ok(lf) => lf,
        Err(_) => return Vec::new(),
    };

    let mut result: Vec<(String, String)> = Vec::new();
    let mut seen_packs: HashSet<String> = HashSet::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue = vec![profile_name.to_string()];

    while let Some(name) = queue.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if let Some(pack_ref) = pack_map.get(&name) {
            if seen_packs.insert(pack_ref.clone()) {
                if let Some(locked) = lockfile.packages.get(pack_ref) {
                    result.push((pack_ref.clone(), locked.version.clone()));
                }
            }
        }
        // Walk extends for all profiles, pack-provided or not, so a user
        // profile that extends a pack profile is handled correctly.
        if let Some(bases) = crate::profile::load_profile_extends(&name) {
            queue.extend(bases);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Background refresh
// ---------------------------------------------------------------------------

fn refresh_in_background(stale: Vec<(String, String)>, state: Arc<Mutex<PackHintsState>>) {
    let registry_url = crate::registry_client::resolve_registry_url(None);
    // Phase 44 IN-02 P43 (D-44-B5 accept-as-documented): the JoinHandle
    // returned by `thread::spawn` is intentionally detached. If `nono`
    // exits before the HTTP request and `save_state` complete, the
    // network request is killed mid-flight and the cache may not be
    // updated on this run. Worst case: more-aggressive registry checking
    // on the next run — acceptable per CONTEXT.md D-44-B5. A graceful
    // shutdown signal path would require routing through the supervisor
    // lifecycle, which is out of scope for the WR-05 fix.
    let _ = thread::spawn(move || {
        let client = crate::registry_client::RegistryClient::new(registry_url);
        let mut changed = false;

        for (pack_ref, installed) in stale {
            let pkg_ref = match crate::package::parse_package_ref(&pack_ref) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let latest = client
                .fetch_package_status(&pkg_ref, Some(&installed))
                .ok()
                .and_then(|s| s.latest_version);

            if let Ok(mut guard) = state.lock() {
                guard.entries.insert(
                    pack_ref,
                    PackHintEntry {
                        last_check: Utc::now(),
                        installed_at_check: installed,
                        latest,
                    },
                );
                changed = true;
            }
        }

        if changed {
            if let Ok(guard) = state.lock() {
                save_state(&guard);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn print_hints(hints: &[(String, String, String)]) {
    if hints.is_empty() {
        return;
    }
    let t = crate::theme::current();
    for (pack_ref, installed, latest) in hints {
        eprintln!(
            "  {} {}  {} {} {}",
            crate::theme::fg("update available", t.yellow),
            crate::theme::fg(pack_ref, t.text),
            crate::theme::fg(&format!("{installed} →"), t.subtext),
            crate::theme::fg(latest, t.green),
            crate::theme::fg(" run: nono update", t.subtext),
        );
    }
    eprintln!();
}

// ---------------------------------------------------------------------------
// Cache I/O
// ---------------------------------------------------------------------------

fn state_file_path() -> Option<std::path::PathBuf> {
    crate::package::nono_config_dir()
        .ok()
        .map(|d| d.join(HINTS_STATE_FILE))
}

fn load_state() -> PackHintsState {
    let path = match state_file_path() {
        Some(p) => p,
        None => return PackHintsState::default(),
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(state: &PackHintsState) {
    let path = match state_file_path() {
        Some(p) => p,
        None => return,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        // Phase 44 IN-01 P43 (REQ-REVIEW-FU-01 D-44-B5): atomic
        // tmp+rename write, mirroring the canonical pattern in
        // `crates/nono-cli/src/package.rs::write_lockfile`. Prevents
        // partial-write corruption if the process is killed between
        // the file truncate and the JSON serialization completion.
        let tmp_path = path.with_extension("json.tmp");
        if std::fs::write(&tmp_path, json).is_ok() {
            let _ = std::fs::rename(&tmp_path, &path);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_opted_out() -> bool {
    if std::env::var("NONO_NO_UPDATE_CHECK").is_ok() {
        return true;
    }
    match crate::config::user::load_user_config() {
        Ok(Some(config)) => !config.updates.check,
        _ => false,
    }
}

/// Determine whether `latest` is strictly newer than `installed`.
///
/// Phase 44 WR-03 P43 (REQ-REVIEW-FU-01 D-44-A4): the pre-44 parser
/// `s.splitn(4, '.')` left the third capture as the raw `"<patch>[-pre]"`
/// substring, so `is_newer("1.2.3-beta", "1.2.3")` returned `false` on
/// `(Some, Some)` but `is_newer("1.2.3", "1.2.3-beta")` returned `true`
/// — depending on which side parsed successfully, the function emitted
/// false-positive "update available" hints when the installed version
/// was a pre-release of the same major.minor.patch as `latest`. The
/// retrofit strips `-pre` / `+build-metadata` before splitting on `.`,
/// so the parser matches `Cargo.toml`'s semver semantics. When EITHER
/// side fails to parse, suppress the hint (pre-release suppression
/// trumps a possibly-misleading legacy-installed signal).
fn is_newer(installed: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Option<(u64, u64, u64)> {
        let s = s.strip_prefix('v').unwrap_or(s);
        // Strip semver pre-release / build-metadata before splitting on '.'
        // so "1.2.3-beta" → "1.2.3" → (1, 2, 3) and "1.2.3+build5" → (1, 2, 3).
        let core = s.split(['-', '+']).next().unwrap_or(s);
        let mut parts = core.splitn(3, '.');
        let major: u64 = parts.next()?.parse().ok()?;
        let minor: u64 = parts.next()?.parse().ok()?;
        let patch: u64 = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    };
    match (parse(installed), parse(latest)) {
        (Some(i), Some(l)) => l > i,
        // If EITHER side is unparseable, suppress the hint rather than
        // false-positiving on pre-release installs.
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Phase 44 WR-03 P43 regression: pre-release installs must NOT
    /// trigger an update hint to the same release version. Pre-44
    /// `is_newer("1.2.3-beta", "1.2.3")` returned `true` because the
    /// `splitn(4, '.')` parser left `"3-beta"` as the third capture
    /// and `"3-beta".parse::<u64>()` failed; the `None`-fallback
    /// branch then misclassified the comparison.
    #[test]
    fn is_newer_suppresses_hint_on_prerelease_installed() {
        assert!(
            !is_newer("1.2.3-beta", "1.2.3"),
            "pre-release installed must not trigger an update hint to the same release version"
        );
        assert!(
            !is_newer("2.0.0-rc1", "1.9.0"),
            "pre-release of a higher major must not trigger an update hint to a lower major"
        );
        assert!(
            !is_newer("1.2.3+build5", "1.2.3"),
            "build-metadata-installed must not trigger an update hint to the same release version"
        );
    }

    /// Phase 44 WR-03 P43: happy-path upgrade detection must still
    /// fire when the parser succeeds on both sides.
    #[test]
    fn is_newer_returns_true_on_genuine_upgrade() {
        assert!(is_newer("1.2.3", "1.2.4"));
        assert!(is_newer("v1.2.3", "v1.3.0"));
        assert!(is_newer("1.2.3", "2.0.0"));
    }

    /// Phase 44 WR-03 P43: monotone — downgrades and equal versions
    /// never trigger an update hint.
    #[test]
    fn is_newer_returns_false_on_downgrade_or_equal() {
        assert!(!is_newer("1.2.4", "1.2.3"));
        assert!(!is_newer("1.2.3", "1.2.3"));
        assert!(!is_newer("v2.0.0", "v1.9.9"));
    }
}
