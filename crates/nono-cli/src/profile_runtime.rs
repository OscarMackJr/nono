use crate::cli::SandboxArgs;
use crate::{hooks, package, profile};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) struct PreparedProfile {
    pub(crate) loaded_profile: Option<profile::Profile>,
    pub(crate) capability_elevation: bool,
    #[cfg(target_os = "linux")]
    pub(crate) wsl2_proxy_policy: profile::Wsl2ProxyPolicy,
    #[cfg(target_os = "linux")]
    pub(crate) af_unix_mediation: profile::LinuxAfUnixMediation,
    pub(crate) workdir_access: Option<profile::WorkdirAccess>,
    pub(crate) rollback_exclude_patterns: Vec<String>,
    pub(crate) rollback_exclude_globs: Vec<String>,
    pub(crate) network_profile: Option<String>,
    pub(crate) allow_domain: Vec<crate::profile::AllowDomainEntry>,
    pub(crate) credentials: Vec<String>,
    pub(crate) custom_credentials: HashMap<String, profile::CustomCredentialDef>,
    pub(crate) upstream_proxy: Option<String>,
    pub(crate) upstream_bypass: Vec<String>,
    pub(crate) listen_ports: Vec<u16>,
    pub(crate) open_url_origins: Vec<String>,
    pub(crate) open_url_allow_localhost: bool,
    pub(crate) allow_launch_services: bool,
    pub(crate) bypass_protection_paths: Vec<PathBuf>,
    pub(crate) ignored_denial_paths: Vec<PathBuf>,
    pub(crate) suppressed_system_service_operations: Vec<String>,
    /// Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`):
    /// allow-list of environment variable names from `profile.environment.allow_vars`.
    /// `None` means inherit-all (default upstream behaviour); `Some([])`
    /// means strip all (fail-closed). Wired to the Unix execution path via
    /// `ExecConfig.allowed_env_vars`. Windows execution path uses the
    /// separate `exec_strategy_windows` module and does not consume this
    /// field; full Windows env-filter wiring tracked for a future plan
    /// (P34-DEFER-08a-1 if needed).
    pub(crate) allowed_env_vars: Option<Vec<String>>,
    /// Plan 34-08a Task 4 (D-20 replay of v0.52.0 `3657c935`): operator-
    /// controlled deny-list of environment variable names from
    /// `profile.environment.deny_vars`. `None` means no deny filter active.
    /// Wired to the Unix execution path via `ExecConfig.denied_env_vars`.
    pub(crate) denied_env_vars: Option<Vec<String>>,
    /// Expanded `environment.set_vars` entries (key, expanded-value). `None`
    /// when the profile has no `set_vars`. Values are expanded with
    /// [`profile::expand_vars`] at prepare time.
    pub(crate) set_vars: Option<Vec<(String, String)>>,
}

fn install_profile_hooks(profile_name: Option<&str>, profile: &profile::Profile, silent: bool) {
    if profile.hooks.hooks.is_empty() {
        return;
    }

    match hooks::install_profile_hooks(profile_name, &profile.hooks.hooks) {
        Ok(results) => {
            for (target, result) in results {
                match result {
                    hooks::HookInstallResult::Installed => {
                        if !silent {
                            eprintln!("  Installing {} hook to ~/.claude/hooks/", target);
                        }
                    }
                    hooks::HookInstallResult::Updated => {
                        if !silent {
                            eprintln!("  Updating {} hook (new version available)", target);
                        }
                    }
                    hooks::HookInstallResult::AlreadyInstalled
                    | hooks::HookInstallResult::Skipped => {}
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to install profile hooks: {}", e);
            if !silent {
                eprintln!("  Warning: Failed to install hooks: {}", e);
            }
        }
    }
}

/// Verify that all packs declared in the profile are installed and intact.
///
/// For each pack:
/// 1. Check the pack directory exists
/// 2. Verify artifact SHA-256 digests against the lockfile
/// 3. Re-verify Sigstore bundles from the stored `.nono-trust.bundle` file
///    and check signer identity against the lockfile pin.
fn verify_profile_packs(packs: &[String]) -> crate::Result<()> {
    if packs.is_empty() {
        return Ok(());
    }

    let lockfile = package::read_lockfile()?;

    for pack_ref in packs {
        let parts: Vec<&str> = pack_ref.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(nono::NonoError::PackageInstall(format!(
                "invalid pack reference '{}': expected <namespace>/<name>",
                pack_ref
            )));
        }
        let (namespace, name) = (parts[0], parts[1]);

        let install_dir = package::package_install_dir(namespace, name)?;
        if !install_dir.exists() {
            tracing::warn!(
                "Pack '{}' declared by profile but not installed. \
                 Install it with: nono pull {}",
                pack_ref,
                pack_ref
            );
            continue;
        }

        // D-20 replay of db073750 (C4): require a lockfile entry — missing entry
        // means the package was installed without proper metadata tracking.
        // Fail hard so users know to reinstall rather than silently skipping.
        let locked_pkg = lockfile.packages.get(pack_ref).ok_or_else(|| {
            nono::NonoError::PackageVerification {
                package: pack_ref.clone(),
                reason: format!(
                    "pack '{}' has no lockfile entry - reinstall with: nono pull {} --force",
                    pack_ref, pack_ref
                ),
            }
        })?;

        for (artifact_name, locked_artifact) in &locked_pkg.artifacts {
            let artifact_path = install_dir.join(artifact_name);
            if !artifact_path.exists() {
                return Err(nono::NonoError::PackageInstall(format!(
                    "pack '{}' is missing artifact '{}'. Reinstall with: nono pull {} --force",
                    pack_ref, artifact_name, pack_ref
                )));
            }

            let bytes = std::fs::read(&artifact_path).map_err(|e| {
                nono::NonoError::PackageInstall(format!(
                    "failed to read artifact '{}' in pack '{}': {}",
                    artifact_name, pack_ref, e
                ))
            })?;
            let digest = Sha256::digest(&bytes);
            let hash = digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            if hash != locked_artifact.sha256 {
                return Err(nono::NonoError::PackageInstall(format!(
                    "pack '{}' artifact '{}' has been tampered with.\n\
                     Expected: {}\n\
                     Found:    {}\n\
                     Reinstall with: nono pull {} --force",
                    pack_ref, artifact_name, locked_artifact.sha256, hash, pack_ref
                )));
            }
        }

        // D-20 replay of db073750 (C4): require a .nono-trust.bundle — if the
        // bundle is missing the pack cannot be re-verified and the install is
        // considered corrupted. Fail hard so users know to reinstall.
        let bundle_path = install_dir.join(".nono-trust.bundle");
        if !bundle_path.exists() {
            return Err(nono::NonoError::PackageVerification {
                package: pack_ref.clone(),
                reason: format!(
                    "pack '{}' is missing .nono-trust.bundle - reinstall with: nono pull {} --force",
                    pack_ref, pack_ref
                ),
            });
        }

        let pinned_signer = locked_pkg
            .provenance
            .as_ref()
            .map(|p| p.signer_identity.as_str())
            .ok_or_else(|| nono::NonoError::PackageVerification {
                package: pack_ref.clone(),
                reason: format!(
                    "pack '{}' has no signer identity in the lockfile - reinstall with: nono pull {} --force",
                    pack_ref, pack_ref
                ),
            })?;
        verify_stored_bundles(&install_dir, &bundle_path, pack_ref, Some(pinned_signer))?;
    }

    Ok(())
}

fn canonical_signer(uri: &str) -> &str {
    uri.rsplit_once('@').map_or(uri, |(prefix, _)| prefix)
}

/// Re-verify each artifact's Sigstore bundle from the stored trust bundle file.
fn verify_stored_bundles(
    install_dir: &Path,
    bundle_path: &Path,
    pack_ref: &str,
    pinned_signer: Option<&str>,
) -> crate::Result<()> {
    let bundle_content = std::fs::read_to_string(bundle_path).map_err(|e| {
        nono::NonoError::PackageInstall(format!(
            "failed to read trust bundle for pack '{}': {}",
            pack_ref, e
        ))
    })?;

    let entries: Vec<serde_json::Value> = serde_json::from_str(&bundle_content).map_err(|e| {
        nono::NonoError::PackageInstall(format!(
            "failed to parse trust bundle for pack '{}': {}",
            pack_ref, e
        ))
    })?;

    let trusted_root = nono::trust::load_production_trusted_root()?;
    let policy = nono::trust::VerificationPolicy::default();

    for entry in &entries {
        let artifact_name = entry
            .get("artifact")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                nono::NonoError::PackageInstall(format!(
                    "trust bundle entry missing 'artifact' field in pack '{}'",
                    pack_ref
                ))
            })?;

        // Resolve the installed path from the bundle entry, falling back to
        // the artifact filename for backwards compatibility with bundles
        // produced before the installed_path field was added (D-32-15).
        let installed_path = entry
            .get("installed_path")
            .and_then(|v| v.as_str())
            .unwrap_or(artifact_name);

        let bundle_value = entry.get("bundle").ok_or_else(|| {
            nono::NonoError::PackageInstall(format!(
                "trust bundle entry missing 'bundle' field for '{}' in pack '{}'",
                artifact_name, pack_ref
            ))
        })?;

        // Require the digest field so we can verify the stored artifact
        // matches the bundle's recorded digest (stricter than verify_bundle_subject_name).
        let expected_digest = entry
            .get("digest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                nono::NonoError::PackageInstall(format!(
                    "trust bundle entry missing 'digest' field for '{}' in pack '{}'",
                    artifact_name, pack_ref
                ))
            })?;

        // Validate installed_path before using it to locate the artifact on
        // disk (defense-in-depth against path traversal in attacker-crafted bundles).
        let safe_installed_path =
            validate_bundle_relative_path(installed_path, artifact_name, pack_ref)?;
        let artifact_path = install_dir.join(safe_installed_path);
        if !artifact_path.exists() {
            continue;
        }

        let artifact_bytes = std::fs::read(&artifact_path).map_err(|e| {
            nono::NonoError::PackageInstall(format!(
                "failed to read '{}' for bundle verification in pack '{}': {}",
                artifact_name, pack_ref, e
            ))
        })?;

        let bundle_json = serde_json::to_string(bundle_value).map_err(|e| {
            nono::NonoError::PackageInstall(format!(
                "failed to serialize bundle for '{}' in pack '{}': {}",
                artifact_name, pack_ref, e
            ))
        })?;

        let bundle = nono::trust::load_bundle_from_str(
            &bundle_json,
            Path::new(&format!("{}.bundle", artifact_name)),
        )?;

        // Verify that the bundle's subjects contain the expected artifact name
        // and sha256 digest (more precise than verify_bundle_subject_name which
        // only checks the name, not the digest value).
        let subjects = nono::trust::extract_all_subjects(
            &bundle,
            Path::new(&format!("{}.bundle", artifact_name)),
        )?;
        if !subjects
            .iter()
            .any(|(name, digest)| name == artifact_name && digest == expected_digest)
        {
            return Err(nono::NonoError::PackageInstall(format!(
                "trust bundle for '{}' in pack '{}' does not contain the expected subject digest",
                artifact_name, pack_ref
            )));
        }
        nono::trust::verify_bundle(
            &artifact_bytes,
            &bundle,
            &trusted_root,
            &policy,
            Path::new(artifact_name),
        )
        .map_err(|e| {
            nono::NonoError::PackageInstall(format!(
                "Sigstore verification failed for '{}' in pack '{}': {}\n\
                 Reinstall with: nono pull {} --force",
                artifact_name, pack_ref, e, pack_ref
            ))
        })?;

        // Check the verified signer identity against the lockfile pin.
        // All artifacts in a pack share the same signer, so we check on each
        // entry and fail fast on any mismatch.
        if let Some(pinned) = pinned_signer {
            let identity = nono::trust::extract_signer_identity(&bundle, Path::new(artifact_name))?;
            let verified_uri = match &identity {
                nono::trust::SignerIdentity::Keyless {
                    repository,
                    workflow,
                    git_ref,
                    ..
                } => format!("https://github.com/{repository}/{workflow}@{git_ref}"),
                nono::trust::SignerIdentity::Keyed { key_id } => {
                    format!("keyed:{key_id}")
                }
            };
            // Strip @<git_ref> for canonical comparison — we pin repo+workflow,
            // not the specific tag that triggered each release.
            if canonical_signer(verified_uri.as_str()) != canonical_signer(pinned) {
                return Err(nono::NonoError::PackageVerification {
                    package: pack_ref.to_string(),
                    reason: format!(
                        "signer identity mismatch for '{}': bundle was signed by '{}' \
                         but lockfile pins '{}'. Reinstall with: nono pull {} --force",
                        artifact_name, verified_uri, pinned, pack_ref
                    ),
                });
            }
        }
    }

    Ok(())
}

/// Validate that `installed_path` from a `.nono-trust.bundle` entry is safe
/// to use as a path relative to the package install directory.
///
/// Rejects empty strings, absolute paths, and any path component that is not
/// `Component::Normal` (i.e., `..`, `.`, root `/`, Windows drive prefixes).
/// This is defense-in-depth against attacker-crafted bundle files with unsafe
/// `installed_path` values (T-48-08-01; CLAUDE.md § Path Handling).
///
/// Uses path-component comparison per CLAUDE.md § Common Footguns #1 (never
/// string `starts_with()`).
fn validate_bundle_relative_path<'a>(
    installed_path: &'a str,
    artifact_name: &str,
    pack_ref: &str,
) -> crate::Result<&'a Path> {
    let path = Path::new(installed_path);
    if installed_path.is_empty() || path.is_absolute() {
        return Err(nono::NonoError::PackageInstall(format!(
            "trust bundle entry for '{}' in pack '{}' has unsafe installed_path '{}'",
            artifact_name, pack_ref, installed_path
        )));
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err(nono::NonoError::PackageInstall(format!(
                    "trust bundle entry for '{}' in pack '{}' has unsafe installed_path '{}'",
                    artifact_name, pack_ref, installed_path
                )));
            }
        }
    }
    Ok(path)
}

fn expand_bypass_protection_path(path: &Path, workdir: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let expanded = profile::expand_vars(&path_str, workdir).unwrap_or_else(|_| path.to_path_buf());
    if expanded.exists() {
        expanded.canonicalize().unwrap_or(expanded)
    } else {
        expanded
    }
}

fn collect_bypass_protection_paths(
    loaded_profile: Option<&profile::Profile>,
    cli_bypass_protection: &[PathBuf],
    workdir: &Path,
) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = loaded_profile
        .map(|profile| {
            profile
                .policy
                .bypass_protection
                .iter()
                .filter_map(|template| {
                    profile::expand_vars(template, workdir)
                        .ok()
                        .map(|expanded| {
                            if expanded.exists() {
                                expanded.canonicalize().unwrap_or(expanded)
                            } else {
                                expanded
                            }
                        })
                })
                .collect()
        })
        .unwrap_or_default();

    for path in cli_bypass_protection {
        let canonical = expand_bypass_protection_path(path, workdir);
        if !paths.contains(&canonical) {
            paths.push(canonical);
        }
    }

    paths
}

/// Plan 35-02 (REQ-PORT-CLOSURE-06 / P34-DEFER-09-1): cherry-pick of
/// upstream `bdf183e9` (v0.44.0) — pre-create `~/.config/nono/profiles/`
/// BEFORE the caller (sandbox_prepare.rs:298 → Sandbox::apply →
/// landlock::restrict_self) locks the filesystem ruleset. Landlock is
/// strictly allow-list and requires the parent directory of any
/// granted child path to exist at ruleset-apply time, even when the
/// child path is explicitly granted write. Without this pre-create,
/// first-run `nono run` on a clean install (with `~/.config/nono/`
/// missing) produces a confusing `No such file or directory` error
/// pointing at the profiles path.
///
/// macOS and Windows are compile-time no-ops (Seatbelt and Windows
/// Job-Object sandbox have no equivalent restriction).
#[cfg(target_os = "linux")]
fn pre_create_landlock_profiles_dir() -> crate::Result<()> {
    let dir = crate::config::user_profiles_dir()?;
    std::fs::create_dir_all(&dir).map_err(nono::NonoError::Io)?;
    Ok(())
}

/// Pre-create the profile-drafts directory if missing. Cross-platform
/// (unlike the Linux-only `pre_create_landlock_profiles_dir`). Best-effort:
/// non-fatal if the create fails (the actual create happens in setup_profiles).
/// Phase 36.5 D-36.5-B1.
fn pre_create_drafts_dir() {
    if let Ok(drafts_dir) = crate::package::profile_drafts_dir() {
        let _ = std::fs::create_dir_all(&drafts_dir);
    }
}

/// Expand the values of `environment.set_vars` using the same variable
/// substitution as profile paths (`$HOME`, `~`, `$WORKDIR`, `$TMPDIR`,
/// `$XDG_*`, `$NONO_PACKAGES`). Keys are preserved verbatim. Returns `None`
/// when the profile has no `set_vars`. Expansion errors are fatal so a
/// misconfigured value never silently reaches the child.
fn expand_profile_set_vars(
    loaded_profile: Option<&profile::Profile>,
    workdir: &Path,
) -> crate::Result<Option<Vec<(String, String)>>> {
    let Some(env_config) = loaded_profile.and_then(|profile| profile.environment.as_ref()) else {
        return Ok(None);
    };
    if env_config.set_vars.is_empty() {
        return Ok(None);
    }

    // Sort keys for deterministic ordering (HashMap iteration order is random).
    let mut keys: Vec<&String> = env_config.set_vars.keys().collect();
    keys.sort();

    let mut expanded = Vec::with_capacity(keys.len());
    for key in keys {
        let Some(value) = env_config.set_vars.get(key) else {
            continue;
        };
        let expanded_value = profile::expand_vars(value, workdir)?
            .to_string_lossy()
            .into_owned();
        expanded.push((key.clone(), expanded_value));
    }
    Ok(Some(expanded))
}

/// Prepare the profile-derived configuration for sandbox execution.
///
/// Phase 37 D-12: takes a [`profile::ResolveContext`] so callers in the
/// `nono run` / `nono wrap` handlers can honor `--no-auto-pull`. Sites that
/// don't care about auto-pull suppression supply
/// `&profile::ResolveContext::default()` (or go through the
/// [`crate::sandbox_prepare::prepare_sandbox`] legacy wrapper, which does so
/// automatically).
pub(crate) fn prepare_profile_with_context(
    args: &SandboxArgs,
    silent: bool,
    workdir: &Path,
    resolve_ctx: &profile::ResolveContext,
) -> crate::Result<PreparedProfile> {
    #[cfg(target_os = "linux")]
    pre_create_landlock_profiles_dir()?;

    // Phase 36.5: pre-create profile-drafts dir cross-platform (D-36.5-B1).
    pre_create_drafts_dir();

    let loaded_profile = if let Some(ref profile_name) = args.profile {
        let profile = profile::load_profile_with_context(profile_name, resolve_ctx)?;
        // Phase 36.5: enforce package status (e.g., refuse load if installed
        // pack is yanked). Mitigates T-36.5-06 (registry-spoof — RegistryClient
        // routes through resolve_registry_url allowlist) and provides the
        // upstream's ActionRequired second-callsite for yanked packs.
        // Advisory-only by default; strict via NONO_REQUIRE_PACK_STATUS=1.
        crate::package_status::enforce_for_active_profile(Some(profile_name.as_str()), silent)?;
        // If the profile was addressed by pack ref (e.g. --profile always-further/hermes),
        // ensure that pack is verified even if the profile JSON doesn't list it in `packs`.
        // Pack refs are injected into profile.packs at load time for every
        // pack-store resolution — both direct registry refs and name/alias
        // paths — so no post-hoc lookup is needed here.
        let mut packs_to_verify = profile.packs.clone();

        // For direct registry refs the pack key may not yet be in packs if
        // load_registry_profile found the pack installed but the profile JSON
        // predates the injection convention. Guard with a fallback.
        if profile::is_registry_ref(profile_name) {
            let key = profile_name
                .as_str()
                .split_once('@')
                .map_or(profile_name.as_str(), |(p, _)| p)
                .to_string();
            if !packs_to_verify.contains(&key) {
                packs_to_verify.push(key);
            }
        }
        verify_profile_packs(&packs_to_verify)?;
        if !packs_to_verify.is_empty() && !silent {
            eprintln!("  Verified {} pack(s)", packs_to_verify.len());
        }
        install_profile_hooks(Some(profile_name.as_str()), &profile, silent);
        Some(profile)
    } else {
        None
    };

    Ok(PreparedProfile {
        capability_elevation: loaded_profile
            .as_ref()
            .and_then(|profile| profile.security.capability_elevation)
            .unwrap_or(false),
        #[cfg(target_os = "linux")]
        wsl2_proxy_policy: loaded_profile
            .as_ref()
            .and_then(|profile| profile.security.wsl2_proxy_policy)
            .unwrap_or_default(),
        #[cfg(target_os = "linux")]
        af_unix_mediation: loaded_profile
            .as_ref()
            .and_then(|profile| profile.linux.af_unix_mediation)
            .unwrap_or_default(),
        workdir_access: loaded_profile
            .as_ref()
            .map(|profile| profile.workdir.access.clone()),
        rollback_exclude_patterns: loaded_profile
            .as_ref()
            .map(|profile| profile.rollback.exclude_patterns.clone())
            .unwrap_or_default(),
        rollback_exclude_globs: loaded_profile
            .as_ref()
            .map(|profile| profile.rollback.exclude_globs.clone())
            .unwrap_or_default(),
        network_profile: loaded_profile.as_ref().and_then(|profile| {
            profile
                .network
                .resolved_network_profile()
                .map(|value| value.to_string())
        }),
        allow_domain: loaded_profile
            .as_ref()
            .map(|profile| profile.network.allow_domain.clone())
            .unwrap_or_default(),
        credentials: loaded_profile
            .as_ref()
            .and_then(|profile| profile.network.credentials.clone())
            .unwrap_or_default(),
        custom_credentials: loaded_profile
            .as_ref()
            .map(|profile| profile.network.custom_credentials.clone())
            .unwrap_or_default(),
        upstream_proxy: loaded_profile
            .as_ref()
            .and_then(|profile| profile.network.upstream_proxy.clone()),
        upstream_bypass: loaded_profile
            .as_ref()
            .map(|profile| profile.network.upstream_bypass.clone())
            .unwrap_or_default(),
        listen_ports: loaded_profile
            .as_ref()
            .map(|profile| profile.network.listen_port.clone())
            .unwrap_or_default(),
        open_url_origins: loaded_profile
            .as_ref()
            .and_then(|profile| profile.open_urls.as_ref())
            .map(|open_urls| open_urls.allow_origins.clone())
            .unwrap_or_default(),
        open_url_allow_localhost: loaded_profile
            .as_ref()
            .and_then(|profile| profile.open_urls.as_ref())
            .map(|open_urls| open_urls.allow_localhost)
            .unwrap_or(false),
        allow_launch_services: loaded_profile
            .as_ref()
            .and_then(|profile| profile.allow_launch_services)
            .unwrap_or(false),
        bypass_protection_paths: collect_bypass_protection_paths(
            loaded_profile.as_ref(),
            &args.bypass_protection,
            workdir,
        ),
        // cc21229f (C3) adaptation: upstream `collect_ignored_denial_paths` accepts
        // a CLI `suppress_save_prompt` slice (not in fork's SandboxArgs yet).
        // Replicate the profile-only path inline. `profile_save_runtime` is
        // cfg-gated to non-Windows, so use `PathBuf::from` on Windows
        // (no sandbox enforcement difference — this field only affects UX).
        ignored_denial_paths: {
            #[cfg(not(target_os = "windows"))]
            let paths = loaded_profile
                .as_ref()
                .map(|profile| {
                    profile
                        .filesystem
                        .suppress_save_prompt
                        .iter()
                        .map(|raw| crate::profile_save_runtime::canonicalize_suppress_path(raw))
                        .collect()
                })
                .unwrap_or_default();
            #[cfg(target_os = "windows")]
            let paths: Vec<PathBuf> = Vec::new();
            paths
        },
        suppressed_system_service_operations: loaded_profile
            .as_ref()
            .map(|profile| profile.diagnostics.suppress_system_services.clone())
            .unwrap_or_default(),
        // Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`):
        // surface `profile.environment.allow_vars` as a runtime allow-list.
        // Plan 34-08a Task 5 (D-20 replay of v0.52.0 `780965d7`): preserve
        // fail-closed semantics for empty allow_vars. An empty `allow_vars`
        // list returns `Some([])` (strip all inherited vars) rather than
        // `None` (no filtering). Profiles that set env_credentials but omit
        // allow_vars would otherwise silently inherit every parent env var.
        //
        // Phase 41 Plan 09 (REQ-CI-01 SC#4 gap closure + WR-06 close-out):
        // Delegates to the canonical
        // `crate::exec_strategy::validate_env_var_patterns` re-export
        // (declared at `exec_strategy.rs:50`). The previously local copy
        // was retained on the (incorrect) assumption that calling into
        // `exec_strategy::*` would reach across the
        // `exec_strategy_windows/` module boundary — but
        // `exec_strategy/env_sanitization.rs` lives in `exec_strategy/`
        // (platform-agnostic), not `exec_strategy_windows/` (Windows-only),
        // so D-34-E1 is not implicated. CI run 25972316892 surfaced the
        // local copy as the canonical fn's only-on-Windows live caller,
        // making the canonical fn dead-code on Linux/macOS. Folding the
        // duplicate into a delegate call (a) clears the dead-code lint and
        // (b) closes WR-06 (drift risk from byte-identical local copy).
        allowed_env_vars: loaded_profile.as_ref().and_then(|profile| {
            profile.environment.as_ref().map(|env_config| {
                if let Some(err) = crate::exec_strategy::validate_env_var_patterns(
                    &env_config.allow_vars,
                    "allow_vars",
                ) {
                    eprintln!("Warning: {}", err);
                }
                env_config.allow_vars.clone()
            })
        }),
        denied_env_vars: loaded_profile.as_ref().and_then(|profile| {
            profile.environment.as_ref().and_then(|env_config| {
                if env_config.deny_vars.is_empty() {
                    return None;
                }
                if let Some(err) = crate::exec_strategy::validate_env_var_patterns(
                    &env_config.deny_vars,
                    "deny_vars",
                ) {
                    eprintln!("Warning: {}", err);
                }
                Some(env_config.deny_vars.clone())
            })
        }),
        set_vars: expand_profile_set_vars(loaded_profile.as_ref(), workdir)?,
        loaded_profile,
    })
}

#[cfg(test)]
mod tests {
    use crate::profile::{EnvironmentConfig, Profile};

    /// RAII guard that saves and restores an environment variable.
    ///
    /// Required per CLAUDE.md § "Environment variables in tests": tests that
    /// modify `HOME`, `XDG_CONFIG_HOME`, or other env vars MUST save and
    /// restore the original value, because Rust runs unit tests in parallel
    /// within the same process.
    #[cfg(target_os = "linux")]
    struct EnvGuard {
        key: String,
        prior: Option<std::ffi::OsString>,
    }

    #[allow(clippy::disallowed_methods)]
    // EnvGuard IS the Drop-restore primitive — same role as
    // crate::test_env::EnvVarGuard. Lint applies to consumers.
    #[cfg(target_os = "linux")]
    impl EnvGuard {
        fn set(key: &str, value: &std::path::Path) -> Self {
            let prior = std::env::var_os(key);
            // SAFETY per CLAUDE.md: set_var is sound in single-threaded test
            // setup; the Drop impl unwinds the change before parallel tests
            // resume. The modified window is as short as possible.
            std::env::set_var(key, value);
            Self {
                key: key.to_string(),
                prior,
            }
        }
    }

    #[allow(clippy::disallowed_methods)] // Restoring env vars is the other half of the safe wrapper.
    #[cfg(target_os = "linux")]
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.prior.take() {
                Some(val) => std::env::set_var(&self.key, val),
                None => std::env::remove_var(&self.key),
            }
        }
    }

    /// Plan 35-02 (REQ-PORT-CLOSURE-06): regression test locking the
    /// idempotent + first-run-creates-dir invariant for the Landlock
    /// pre-create hunk. Runs in CI Linux lane (D-35-D3); compile-time
    /// no-op on Windows/macOS.
    ///
    /// Verifies:
    /// 1. Calling `pre_create_landlock_profiles_dir()` creates
    ///    `<XDG_CONFIG_HOME>/nono/profiles/` on a clean fixture.
    /// 2. A second call succeeds without error (idempotency via
    ///    `std::fs::create_dir_all`).
    #[cfg(target_os = "linux")]
    #[test]
    fn test_pre_create_landlock_profiles_dir_idempotent() {
        // WR-03 fix (REVIEW.md): acquire the process-wide env lock BEFORE
        // mutating XDG_CONFIG_HOME so parallel tests don't read the tempdir
        // value during the modified window. EnvGuard::Drop restores on the
        // way out, but other tests reading XDG_CONFIG_HOME during this
        // test's runtime would still see the tempdir value without this
        // lock. Matches the convention in policy.rs / profile_save_runtime
        // tests per CLAUDE.md § Environment variables in tests.
        let _env_lock = crate::test_env::lock_env();
        let tmp = tempfile::TempDir::new().expect("create tempdir");
        let _xdg_guard = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());

        // First call — creates <tmp>/nono/profiles/
        super::pre_create_landlock_profiles_dir()
            .expect("first pre-create call must succeed on clean fixture");
        let expected = tmp.path().join("nono").join("profiles");
        assert!(
            expected.is_dir(),
            "Expected profiles dir at {} after first pre-create call",
            expected.display(),
        );

        // Second call — idempotent (create_dir_all succeeds on existing dir)
        super::pre_create_landlock_profiles_dir()
            .expect("second pre-create call must succeed (idempotent on existing dir)");
        assert!(
            expected.is_dir(),
            "Profiles dir should still exist after second call",
        );
    }

    /// Plan 34-08a Task 5 regression test (v0.52.0 `780965d7`):
    /// an `EnvironmentConfig` with empty `allow_vars` MUST surface as
    /// `Some(vec![])` (strip-all / fail-closed) rather than `None`
    /// (no filter / inherit-all). This is the security invariant that
    /// `3657c935` regressed and `780965d7` restored.
    ///
    /// Direct-tests the closure shape used in `prepare_profile`:
    /// `profile.environment.as_ref().map(|cfg| cfg.allow_vars.clone())`.
    #[test]
    fn empty_allow_vars_fails_closed() {
        let profile = Profile {
            environment: Some(EnvironmentConfig {
                allow_vars: vec![],
                deny_vars: vec![],
                set_vars: Default::default(),
            }),
            ..Default::default()
        };
        let allowed: Option<Vec<String>> = profile
            .environment
            .as_ref()
            .map(|env_config| env_config.allow_vars.clone());
        // Must be Some(vec![]) -- strip all -- NOT None.
        assert_eq!(allowed, Some(Vec::<String>::new()));
        assert!(
            allowed.as_ref().is_some_and(|v| v.is_empty()),
            "empty allow_vars must surface as Some(vec![]), not None"
        );
    }

    /// Companion: no `environment` block at all -> None (inherit-all).
    /// Distinguishes "absent" (None) from "explicit empty" (Some([])).
    #[test]
    fn absent_environment_block_returns_none() {
        let profile = Profile {
            environment: None,
            ..Default::default()
        };
        let allowed: Option<Vec<String>> = profile
            .environment
            .as_ref()
            .map(|env_config| env_config.allow_vars.clone());
        assert_eq!(allowed, None);
    }

    // -------------------------------------------------------------------------
    // D-20 replay of db073750 (C4): verify_profile_packs strict lockfile + bundle checks
    // -------------------------------------------------------------------------

    fn with_config_env<F, R>(f: F) -> R
    where
        F: FnOnce(&std::path::Path) -> R,
    {
        let _guard = crate::test_env::lock_env();
        let tmp = tempfile::TempDir::new().expect("failed to create tempdir");
        let config_dir = tmp
            .path()
            .canonicalize()
            .expect("failed to canonicalize tempdir");
        let config_str = config_dir
            .to_str()
            .expect("tempdir path is not valid UTF-8");
        // On Windows, APPDATA takes priority over XDG_CONFIG_HOME in
        // `resolve_user_config_dir`. Set both so the test works cross-platform.
        let _env = crate::test_env::EnvVarGuard::set_all(&[
            ("XDG_CONFIG_HOME", config_str),
            ("APPDATA", config_str),
        ]);
        f(&config_dir)
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn expand_profile_set_vars_expands_home() {
        use super::expand_profile_set_vars;
        use std::path::Path;

        let _guard = match crate::test_env::ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let _env = crate::test_env::EnvVarGuard::set_all(&[("HOME", "/home/tester")]);

        let mut profile = Profile::default();
        let mut set_vars = std::collections::HashMap::new();
        set_vars.insert("RUST_LOG".to_string(), "debug".to_string());
        set_vars.insert("CFG".to_string(), "$HOME/.config".to_string());
        profile.environment = Some(EnvironmentConfig {
            allow_vars: vec![],
            deny_vars: vec![],
            set_vars,
        });

        let workdir = Path::new("/tmp/work");
        let expanded = expand_profile_set_vars(Some(&profile), workdir)
            .expect("expansion should succeed")
            .expect("set_vars should be present");

        // Keys are sorted for determinism: CFG before RUST_LOG.
        assert_eq!(
            expanded,
            vec![
                ("CFG".to_string(), "/home/tester/.config".to_string()),
                ("RUST_LOG".to_string(), "debug".to_string()),
            ]
        );
    }

    #[test]
    fn expand_profile_set_vars_none_when_absent() {
        use super::expand_profile_set_vars;
        use std::path::Path;

        let profile = Profile::default();
        let result = expand_profile_set_vars(Some(&profile), Path::new("/tmp/work"))
            .expect("expansion should succeed");
        assert!(result.is_none());
    }

    fn create_pack_dir(
        config_dir: &std::path::Path,
        namespace: &str,
        name: &str,
    ) -> std::path::PathBuf {
        let install_dir = config_dir
            .join("nono")
            .join("packages")
            .join(namespace)
            .join(name);
        std::fs::create_dir_all(&install_dir).expect("failed to create pack dir");
        install_dir
    }

    fn write_lockfile_with_artifact(pack_ref: &str, artifact_name: &str, artifact_bytes: &[u8]) {
        use sha2::Digest as _;
        let digest = sha2::Sha256::digest(artifact_bytes);
        let sha256 = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();

        let mut artifacts = std::collections::BTreeMap::new();
        artifacts.insert(
            artifact_name.to_string(),
            crate::package::LockedArtifact {
                sha256,
                artifact_type: crate::package::ArtifactType::Profile,
                installed_path: None,
            },
        );

        let mut packages = std::collections::BTreeMap::new();
        packages.insert(
            pack_ref.to_string(),
            crate::package::LockedPackage {
                version: "1.0.0".to_string(),
                installed_at: "2026-01-01T00:00:00Z".to_string(),
                pinned: false,
                provenance: Some(crate::package::PackageProvenance {
                    signer_identity:
                        "https://github.com/acme/repo/.github/workflows/release.yml@refs/tags/v1.0.0"
                            .to_string(),
                    repository: "acme/repo".to_string(),
                    workflow: ".github/workflows/release.yml".to_string(),
                    git_ref: "refs/tags/v1.0.0".to_string(),
                    rekor_log_index: 1,
                    signed_at: "2026-01-01T00:00:00Z".to_string(),
                }),
                artifacts,
            },
        );

        let lockfile = crate::package::Lockfile {
            lockfile_version: crate::package::LOCKFILE_VERSION,
            registry: "https://registry.example.test".to_string(),
            packages,
        };
        crate::package::write_lockfile(&lockfile).expect("failed to write lockfile");
    }

    /// D-20 replay of db073750 (C4): a pack directory that exists but has no
    /// lockfile entry must now be a hard PackageVerification error rather than
    /// a soft "not installed" continue.
    #[test]
    fn verify_profile_packs_requires_lockfile_entry_for_installed_pack() {
        let result = with_config_env(|config_dir| {
            create_pack_dir(config_dir, "acme", "widget");
            super::verify_profile_packs(&["acme/widget".to_string()])
        });

        let err = result.expect_err("installed pack without lockfile entry must fail verification");
        assert!(
            err.to_string().contains("no lockfile entry"),
            "unexpected error: {err}"
        );
    }

    /// D-20 replay of db073750 (C4): a pack with a lockfile entry but no
    /// .nono-trust.bundle must be a hard PackageVerification error.
    #[test]
    fn verify_profile_packs_requires_trust_bundle_for_locked_pack() {
        let result = with_config_env(|config_dir| {
            let install_dir = create_pack_dir(config_dir, "acme", "widget");
            let artifact_bytes = br#"{"meta":{"name":"widget"}}"#;
            std::fs::write(install_dir.join("package.json"), artifact_bytes)
                .expect("failed to write package artifact");
            write_lockfile_with_artifact("acme/widget", "package.json", artifact_bytes);

            super::verify_profile_packs(&["acme/widget".to_string()])
        });

        let err = result.expect_err("locked pack without trust bundle must fail verification");
        assert!(
            err.to_string().contains("missing .nono-trust.bundle"),
            "unexpected error: {err}"
        );
    }
}
