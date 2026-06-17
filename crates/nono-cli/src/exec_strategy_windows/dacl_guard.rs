//! Applied-DACL-grants RAII guard (WriteRestricted token-arm fix).
//!
//! # Why this exists
//!
//! On the Windows `WriteRestricted` token arm,
//! `create_restricted_token_with_sid` builds the Low-IL child token with
//! `CreateRestrictedToken(.., WRITE_RESTRICTED, .., 1, &sid_restrict)` where
//! `sid_restrict` is the synthetic per-session SID from `generate_session_sid`
//! (`S-1-5-117-...`). Under `WRITE_RESTRICTED`, every WRITE access check runs
//! TWICE — once against the token's normal SIDs (passes) and once against the
//! restricting-SID list (= only the synthetic SID) — and BOTH must grant.
//! Because the synthetic SID is on NO granted path's DACL, the second check
//! always denies, so every confined write fails with `Access is denied`.
//! (The mandatory integrity label is NOT the gate here: the WriteRestricted
//! child stays Medium-IL and writing into the Low-labeled CWD is writing-DOWN,
//! which is permitted.)
//!
//! nono already adds the session SID to the capability PIPE's DACL but never to
//! FILESYSTEM grants — this guard closes exactly that gap.
//!
//! # What it does
//!
//! For every WRITABLE filesystem rule (`AccessMode::Write` / `ReadWrite` only —
//! read-only rules need no DACL edit because `WRITE_RESTRICTED` double-checks
//! WRITE-class access only), it adds an allow-ACE granting the synthetic
//! session SID write-class rights (`FILE_GENERIC_WRITE | DELETE`, NOT
//! FullControl), inheritable `(OI)(CI)` for directory rules so files the child
//! CREATES inherit the grant. On Drop it revokes exactly the ACEs it added.
//!
//! # Scope of effect
//!
//! The grant is OPERATIVE only on the `WriteRestricted` token arm (where the
//! synthetic SID is a restricting SID on the child token). On every other arm
//! (`BrokerLaunch*`, `LowIlPrimary`, `Null`, detached) the SID is on no child
//! token, so the added ACE is inert. It is always reverted on Drop. Granting a
//! synthetic per-session SID write on an already-user-owned, already-grant-
//! scoped path does not broaden the trust boundary.
//!
//! Modeled on `labels_guard::AppliedLabelsGuard`: same crate, same module,
//! same snapshot/apply/revert/Drop shape, same fail-closed discipline.

use std::path::{Path, PathBuf};

use nono::{
    grant_sid_read_attributes_on_path, grant_sid_traverse_on_path, grant_sid_write_on_path,
    path_is_owned_by_current_user, revoke_sid_on_path, AccessMode, Result, WindowsFilesystemPolicy,
};

/// Per-rule state recorded at snapshot time.
#[derive(Debug)]
enum AppliedDaclGrant {
    /// Rule was skipped — no DACL edit performed, nothing to revert. Either:
    /// 1. The rule is read-only (`WRITE_RESTRICTED` only double-checks
    ///    WRITE-class access, so read-only grants need no SID ACE), OR
    /// 2. The path is not owned by the current user. Editing the DACL needs
    ///    `WRITE_DAC`, which path owners hold implicitly; for non-owned paths
    ///    (system paths granted read via policy groups) we skip rather than
    ///    explode. Non-owned writable paths are exotic and also skipped with a
    ///    warning.
    Skip,
    /// We added the session-SID write-class allow-ACE to this path's DACL. On
    /// Drop, revoke it.
    Applied { path: PathBuf },
}

/// RAII guard that revokes the session-SID DACL grants when dropped.
///
/// Constructed via [`AppliedDaclGrantsGuard::snapshot_and_apply`]. The guard
/// owns the apply side-effect; Drop runs revert. The `session_sid` is stored so
/// Drop can call `revoke_sid_on_path` for each `Applied` entry.
#[derive(Debug)]
pub(crate) struct AppliedDaclGrantsGuard {
    entries: Vec<AppliedDaclGrant>,
    session_sid: String,
}

impl AppliedDaclGrantsGuard {
    /// For every rule in `policy.rules`:
    /// 1. If the rule is read-only (`!access.contains(Write)`), record `Skip`.
    /// 2. Otherwise (writable) gate on `nono::path_is_owned_by_current_user`:
    ///    - `Ok(false)`: record `Skip` + warn (editing the DACL needs
    ///      `WRITE_DAC`, held implicitly only by the owner).
    ///    - `Err(_)`: `revert_all` already-applied entries and propagate
    ///      (fail-closed — ownership-check errors are NEVER swallowed).
    ///    - `Ok(true)`: call `nono::grant_sid_write_on_path` with
    ///      `inheritable = !rule.is_file`, record `Applied { path }`.
    /// 3. If `grant_sid_write_on_path` fails mid-loop, `revert_all`
    ///    already-applied entries and return the original Err.
    ///
    /// Fail-closed: returns `Err` on any apply failure OR ownership-check
    /// failure; no partial-success state is returned.
    pub(crate) fn snapshot_and_apply(
        policy: &WindowsFilesystemPolicy,
        session_sid: &str,
    ) -> Result<Self> {
        let mut guard = Self {
            entries: Vec::new(),
            session_sid: session_sid.to_string(),
        };

        for rule in &policy.rules {
            // Read-only rules: WRITE_RESTRICTED only double-checks WRITE-class
            // access, so no SID ACE is needed. Skip (no revert).
            if !rule.access.contains(AccessMode::Write) {
                guard.entries.push(AppliedDaclGrant::Skip);
                continue;
            }

            // Writable rule: editing the DACL requires WRITE_DAC, which the
            // path owner holds implicitly. Non-owned paths cannot be edited;
            // skip them (system paths granted write are not expected, but
            // fail-open is forbidden — we skip + warn, never silently widen).
            match path_is_owned_by_current_user(&rule.path) {
                Ok(false) => {
                    tracing::warn!(
                        path = %rule.path.display(),
                        access = ?rule.access,
                        "dacl guard: writable path not owned by current user; skipping session-SID \
                         DACL grant (cannot edit DACL without WRITE_DAC — confined writes here will \
                         be denied on the WriteRestricted arm)"
                    );
                    guard.entries.push(AppliedDaclGrant::Skip);
                    continue;
                }
                Err(err) => {
                    tracing::warn!(
                        path = %rule.path.display(),
                        error = %err,
                        "dacl guard: ownership check failed; reverting entries already applied"
                    );
                    guard.revert_all();
                    return Err(err);
                }
                Ok(true) => {
                    // Current user owns the path — proceed to grant.
                }
            }

            // Directory rules get inheritable (OI)(CI) so files the child
            // CREATES inherit the grant; single-file rules do not.
            let inheritable = !rule.is_file;
            if let Err(err) = grant_sid_write_on_path(&rule.path, session_sid, inheritable) {
                tracing::warn!(
                    path = %rule.path.display(),
                    inheritable,
                    "dacl guard: grant failed; reverting entries already applied"
                );
                guard.revert_all();
                return Err(err);
            }
            guard.entries.push(AppliedDaclGrant::Applied {
                path: rule.path.clone(),
            });
        }

        Ok(guard)
    }

    /// Best-effort revert of every applied entry, LIFO. Drop-safe: errors are
    /// logged, never panic. Mirrors `labels_guard::AppliedLabelsGuard`.
    fn revert_all(&mut self) {
        while let Some(entry) = self.entries.pop() {
            match entry {
                AppliedDaclGrant::Skip => {
                    // No-op: we never applied, so there is nothing to revert.
                }
                AppliedDaclGrant::Applied { path } => {
                    Self::best_effort_revert(&path, &self.session_sid);
                }
            }
        }
    }

    fn best_effort_revert(path: &Path, session_sid: &str) {
        if let Err(err) = revoke_sid_on_path(path, session_sid) {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "dacl guard: revoke failed; the session SID may remain on this path's DACL"
            );
        }
    }
}

impl Drop for AppliedDaclGrantsGuard {
    fn drop(&mut self) {
        self.revert_all();
    }
}

/// RAII guard that grants the per-run PACKAGE SID `FILE_TRAVERSE` on the
/// USER-OWNED ancestor directories of the confined cwd, and revokes them on Drop.
///
/// # Why this exists (Plan 62-13, debug `wfp-write-restricted-0142`)
///
/// On the Phase-62 AppContainer arm the confined child runs as the per-run
/// package SID (`S-1-15-2-*`) — a DIFFERENT principal than the user, with NO
/// inherent access to the user-profile directory chain. The cwd LEAF is already
/// granted read+write+traverse by [`AppliedDaclGrantsGuard`] (the 0x1301BF
/// writable mask). But to SET its current directory to a profile-deep cwd (e.g.
/// `%USERPROFILE%\.claude`) the child's token must also TRAVERSE every ANCESTOR
/// (`C:\Users\<user>`, ...).
///
/// This guard walks the cwd ancestors from the immediate parent upward and grants
/// the package SID traverse-only on each USER-OWNED ancestor. It STOPS at the
/// first non-owned ancestor (`C:\Users`, `C:\` — owned by SYSTEM/TrustedInstaller,
/// no `WRITE_DAC`, cannot be edited). Reaching the cwd through those non-owned
/// ancestors depends on the lowbox retaining bypass-traverse
/// (`SeChangeNotifyPrivilege`), which the follow-up live UAT confirms.
///
/// # Best-effort + fail-closed
///
/// - Non-owned ancestor (`Ok(false)`): STOP the walk (cannot edit; higher
///   ancestors are also non-owned). Best-effort — not an error.
/// - Ownership-check error (`Err`): fail-closed — revert what was applied and
///   propagate (ownership-check errors are NEVER swallowed).
/// - Grant error on an owned ancestor: fail-closed — revert + propagate.
///
/// Modeled on [`AppliedDaclGrantsGuard`]: same snapshot/apply/revert/Drop shape,
/// same `path_is_owned_by_current_user` gate, same fail-closed discipline.
#[derive(Debug)]
pub(crate) struct AppliedAncestorTraverseGuard {
    /// The owned ancestor directories we granted traverse on (revoke on Drop).
    applied: Vec<PathBuf>,
    /// The package SID stored so Drop can `revoke_sid_on_path` each entry.
    package_sid: String,
}

impl AppliedAncestorTraverseGuard {
    /// Grant the package SID `FILE_TRAVERSE` on every USER-OWNED ancestor of
    /// `current_dir`, from the immediate parent upward, stopping at the first
    /// non-owned ancestor.
    ///
    /// Fail-closed: returns `Err` (after reverting already-applied grants) on any
    /// ownership-check error or grant failure on an owned ancestor.
    pub(crate) fn snapshot_and_apply(current_dir: &Path, package_sid: &str) -> Result<Self> {
        let mut guard = Self {
            applied: Vec::new(),
            package_sid: package_sid.to_string(),
        };

        // Walk ancestors from the immediate parent upward. `Path::ancestors`
        // yields `current_dir` first, then each parent up to the root, so skip
        // the leaf (index 0 — already granted read+write+traverse by the writable
        // DACL guard).
        for ancestor in current_dir.ancestors().skip(1) {
            match path_is_owned_by_current_user(ancestor) {
                Ok(true) => {
                    if let Err(err) = grant_sid_traverse_on_path(ancestor, package_sid) {
                        tracing::warn!(
                            ancestor = %ancestor.display(),
                            "ancestor-traverse guard: grant failed; reverting entries already applied"
                        );
                        guard.revert_all();
                        return Err(err);
                    }
                    guard.applied.push(ancestor.to_path_buf());
                }
                Ok(false) => {
                    // First non-owned ancestor (e.g. C:\Users, C:\). Cannot edit
                    // its DACL (no WRITE_DAC); every ancestor ABOVE it is also
                    // non-owned. STOP — reaching the cwd through these depends on
                    // the lowbox's bypass-traverse (confirmed by the live UAT).
                    tracing::debug!(
                        ancestor = %ancestor.display(),
                        "ancestor-traverse guard: ancestor not owned by current user; stopping the \
                         walk (cannot grant traverse without WRITE_DAC — relies on lowbox \
                         bypass-traverse from here up)"
                    );
                    break;
                }
                Err(err) => {
                    tracing::warn!(
                        ancestor = %ancestor.display(),
                        error = %err,
                        "ancestor-traverse guard: ownership check failed; reverting entries already applied"
                    );
                    guard.revert_all();
                    return Err(err);
                }
            }
        }

        Ok(guard)
    }

    /// Best-effort revert of every applied grant, LIFO. Drop-safe: errors are
    /// logged, never panic.
    fn revert_all(&mut self) {
        while let Some(path) = self.applied.pop() {
            if let Err(err) = revoke_sid_on_path(&path, &self.package_sid) {
                tracing::warn!(
                    ancestor = %path.display(),
                    error = %err,
                    "ancestor-traverse guard: revoke failed; the package SID may remain on this \
                     ancestor's DACL"
                );
            }
        }
    }
}

impl Drop for AppliedAncestorTraverseGuard {
    fn drop(&mut self) {
        self.revert_all();
    }
}

/// RAII guard that grants the per-run PACKAGE SID `FILE_READ_ATTRIBUTES` (RA)
/// on the USER-OWNED ancestor directories of the **confined target's resolution
/// chain**, and revokes them on Drop.
///
/// # Why this exists (Plan 77-01, CPLT-01)
///
/// On the AppContainer arm the confined child runs as the per-run package SID
/// (`S-1-15-2-*`) — a DIFFERENT principal than the user. The Node-ESM runtime
/// (`realpathSync`/`lstat`) walks EVERY ancestor of a module path to resolve
/// canonical paths at startup. Each ancestor directory requires at least
/// `FILE_READ_ATTRIBUTES` on the queried directory object. Because the package
/// SID has no inherent access to the user-profile directory chain, every
/// ancestor `lstat` fails with `STATUS_ACCESS_DENIED`.
///
/// This guard walks the ancestors of the confined **target binary** (the walk
/// target fed by `config.resolved_program` — NOT the cwd) from the immediate
/// parent upward and grants the package SID `FILE_READ_ATTRIBUTES`-only on
/// each USER-OWNED ancestor. It STOPS at the first non-owned ancestor (`C:\Users`,
/// `C:\` — owned by SYSTEM/TrustedInstaller, no `WRITE_DAC`). Reaching the
/// resolution chain through those system ancestors requires the one-time-admin
/// `nono setup --grant-ancestors` command (CPLT-02), which grants the
/// well-known `ALL APPLICATION PACKAGES` SID (`S-1-15-2-1`) on `C:\` and
/// `C:\Users` non-destructively.
///
/// # Relationship to AppliedAncestorTraverseGuard
///
/// - **`AppliedAncestorTraverseGuard`** walks the **cwd** ancestors and grants
///   `FILE_TRAVERSE | FILE_LIST_DIRECTORY` (0x21) so the AppContainer child can
///   SET its current directory to a profile-deep cwd.
/// - **`AppliedAncestorReadAttributesGuard`** (this guard) walks the **target
///   binary** ancestors and grants `FILE_READ_ATTRIBUTES` (0x80) so Node-ESM
///   `realpathSync`/`lstat` succeeds on the module resolution chain.
///
/// The two guards are kept separate because:
/// - The walk targets differ (cwd vs target binary path).
/// - The required right differs (TRAVERSE vs READ_ATTRIBUTES).
/// - The error surfaces differ — merge would require complex per-ancestor
///   multi-right tracking with no robustness benefit.
///
/// # Best-effort + fail-closed
///
/// - Non-owned ancestor (`Ok(false)`): STOP the walk (the D-04 split — proves
///   the runtime guard structurally cannot touch `C:\Users`/`C:\`). Not an error.
/// - Ownership-check error (`Err`): fail-closed — revert what was applied and
///   propagate (ownership errors are NEVER swallowed).
/// - Grant error on an owned ancestor: fail-closed — revert + propagate.
///
/// Always uses `NO_INHERITANCE` (the RA grant is scoped to the specific
/// directory object; it must NOT propagate to descendants).
#[derive(Debug)]
pub(crate) struct AppliedAncestorReadAttributesGuard {
    /// The owned ancestor directories we granted FILE_READ_ATTRIBUTES on
    /// (revoke on Drop).
    applied: Vec<PathBuf>,
    /// The package SID stored so Drop can `revoke_sid_on_path` each entry.
    package_sid: String,
}

impl AppliedAncestorReadAttributesGuard {
    /// Grant the package SID `FILE_READ_ATTRIBUTES` on every USER-OWNED ancestor
    /// of `walk_target`, from the immediate parent upward, stopping at the first
    /// non-owned ancestor. Thin wrapper over [`Self::snapshot_and_apply_targets`]
    /// for the single-walk-target callers.
    ///
    /// Fail-closed: returns `Err` (after reverting already-applied grants) on any
    /// ownership-check error or grant failure on an owned ancestor.
    pub(crate) fn snapshot_and_apply(walk_target: &Path, package_sid: &str) -> Result<Self> {
        Self::snapshot_and_apply_targets(&[walk_target], package_sid)
    }

    /// Grant the package SID `FILE_READ_ATTRIBUTES` on every USER-OWNED ancestor
    /// of EACH `walk_target`, from each target's immediate parent upward,
    /// stopping at the first non-owned ancestor PER CHAIN. Ancestors shared
    /// across targets (e.g. `C:\Users\<user>`) are granted EXACTLY ONCE and
    /// recorded once, so Drop reverts each exactly once (no double-grant, no
    /// double-revoke).
    ///
    /// Two walk targets are needed in the AppContainer arm (CPLT-01 / 77-04):
    /// 1. `config.resolved_program` — the confined target binary's resolution
    ///    chain (covers engines that resolve modules next to their executable).
    /// 2. `config.current_dir` — the `--workspace` (child CWD) chain. Node
    ///    engines such as the WinGet `@github/copilot` CLI **self-extract their
    ///    package under the workspace** (`<ws>\.nono-runtime\…\AC\…`), so
    ///    `realpathSync`/`lstat` walks the WORKSPACE's ancestor chain and needs
    ///    RA up to (but not including) the first non-owned ancestor — i.e. on
    ///    `C:\Users\<user>`, which the binary chain may not reach when a
    ///    non-owned dir (e.g. a WinGet-installed package dir) sits mid-chain.
    ///
    /// The per-chain stop preserves the D-04 structural split: the runtime guard
    /// provably never touches `C:\Users` or `C:\` (those are CPLT-02 admin-grant
    /// territory). Fail-closed: returns `Err` (after reverting already-applied
    /// grants) on any ownership-check error or grant failure on an owned ancestor.
    pub(crate) fn snapshot_and_apply_targets(
        walk_targets: &[&Path],
        package_sid: &str,
    ) -> Result<Self> {
        let mut guard = Self {
            applied: Vec::new(),
            package_sid: package_sid.to_string(),
        };

        for walk_target in walk_targets {
            // Walk ancestors from the immediate parent upward. `Path::ancestors`
            // yields `walk_target` first (the leaf itself), so skip index 0. The
            // RA grant is on ancestor DIRECTORIES, not on the leaf.
            for ancestor in walk_target.ancestors().skip(1) {
                // Dedup across walk targets: a shared ancestor already granted by
                // a prior target must not be re-granted/re-recorded. Skip the
                // grant but keep walking up — higher ancestors are shared too and
                // were already processed (granted or stopped) by the prior walk.
                if guard.applied.iter().any(|p| p.as_path() == ancestor) {
                    continue;
                }
                match path_is_owned_by_current_user(ancestor) {
                    Ok(true) => {
                        if let Err(err) = grant_sid_read_attributes_on_path(ancestor, package_sid) {
                            tracing::warn!(
                                ancestor = %ancestor.display(),
                                "ancestor-RA guard: grant failed; reverting entries already applied"
                            );
                            guard.revert_all();
                            return Err(err);
                        }
                        guard.applied.push(ancestor.to_path_buf());
                    }
                    Ok(false) => {
                        // First non-owned ancestor on THIS chain (e.g. C:\Users,
                        // C:\). Cannot edit its DACL (no WRITE_DAC); every ancestor
                        // ABOVE it is also non-owned. STOP this chain — the
                        // one-time-admin CPLT-02 grant covers these system
                        // ancestors. D-04 structural split: the runtime guard
                        // provably never touches C:\Users or C:\.
                        tracing::debug!(
                            ancestor = %ancestor.display(),
                            "ancestor-RA guard: ancestor not owned by current user; stopping this \
                             chain (CPLT-02 admin grant covers system ancestors from here up)"
                        );
                        break;
                    }
                    Err(err) => {
                        tracing::warn!(
                            ancestor = %ancestor.display(),
                            error = %err,
                            "ancestor-RA guard: ownership check failed; reverting entries already applied"
                        );
                        guard.revert_all();
                        return Err(err);
                    }
                }
            }
        }

        Ok(guard)
    }

    /// Best-effort revert of every applied grant, LIFO. Drop-safe: errors are
    /// logged, never panic. Mirrors `AppliedAncestorTraverseGuard::revert_all`.
    fn revert_all(&mut self) {
        while let Some(path) = self.applied.pop() {
            if let Err(err) = revoke_sid_on_path(&path, &self.package_sid) {
                tracing::warn!(
                    ancestor = %path.display(),
                    error = %err,
                    "ancestor-RA guard: revoke failed; the package SID may remain on this \
                     ancestor's DACL"
                );
            }
        }
    }
}

impl Drop for AppliedAncestorReadAttributesGuard {
    fn drop(&mut self) {
        self.revert_all();
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nono::{CapabilitySource, NonoError, WindowsFilesystemRule};
    use tempfile::tempdir;

    // A unique synthetic SID for tests, shaped like generate_session_sid's
    // `S-1-5-117-...` output. Pre-exists in NO real ACE, so REVOKE removes
    // only what the guard added.
    const TEST_SESSION_SID: &str = "S-1-5-117-5-6-7-8";

    /// Make `path` owned by the CURRENT user so the ancestor-RA / ancestor-traverse
    /// ownership gate (`path_is_owned_by_current_user`) returns `Ok(true)` for it
    /// deterministically. In an ELEVATED session, freshly-created tempdirs are owned
    /// by `BUILTIN\Administrators` (not the user), which would make the ownership
    /// check return `false`, stop the walk immediately, and leave `applied` empty —
    /// a session-elevation artifact, not a logic failure. Taking ownership keeps
    /// these ownership-dependent tests green whether or not the suite runs elevated.
    fn take_ownership_for_current_user(path: &Path) {
        // `whoami` prints `domain\user`, which icacls /setowner accepts.
        let who = std::process::Command::new("whoami")
            .output()
            .expect("run whoami");
        let user = String::from_utf8_lossy(&who.stdout).trim().to_string();
        assert!(!user.is_empty(), "whoami returned an empty user");
        let out = std::process::Command::new("icacls")
            .arg(path)
            .arg("/setowner")
            .arg(&user)
            .arg("/Q")
            .output()
            .expect("run icacls /setowner");
        assert!(
            out.status.success(),
            "icacls /setowner {} -> {} failed: {}",
            path.display(),
            user,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Returns true iff `path`'s DACL contains an ACE for `sid`.
    fn dacl_contains_sid(path: &Path, sid: &str) -> bool {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{
            ConvertStringSidToSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
        };
        use windows_sys::Win32::Security::{
            EqualSid, GetAce, ACCESS_ALLOWED_ACE, ACL, DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, PSID,
        };

        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_sid: Vec<u16> = sid.encode_utf16().chain(std::iter::once(0)).collect();

        let mut want_sid: PSID = std::ptr::null_mut();
        // SAFETY: valid nul-terminated UTF-16 SID string + valid out-pointer.
        let ok = unsafe { ConvertStringSidToSidW(wide_sid.as_ptr(), &mut want_sid) };
        assert!(ok != 0 && !want_sid.is_null(), "parse test SID");

        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: valid path buffer + valid out-pointers; SD freed below.
        let status = unsafe {
            GetNamedSecurityInfoW(
                wide_path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut sd,
            )
        };
        assert_eq!(status, 0, "GetNamedSecurityInfoW(DACL) must succeed");

        let mut found = false;
        if !dacl.is_null() {
            // SAFETY: `dacl` points into the SD we own until LocalFree below.
            let ace_count = unsafe { (*dacl).AceCount };
            for index in 0..ace_count {
                let mut ace = std::ptr::null_mut();
                // SAFETY: `dacl` is valid; `ace` is a valid out-pointer.
                let got = unsafe { GetAce(dacl, u32::from(index), &mut ace) };
                if got == 0 || ace.is_null() {
                    continue;
                }
                // SAFETY: allow/deny ACEs share the SidStart layout; we read
                // the embedded SID at that offset.
                let ace_sid = unsafe {
                    (&(*(ace as *const ACCESS_ALLOWED_ACE)).SidStart) as *const u32 as PSID
                };
                // SAFETY: both SIDs are valid for the duration of the call.
                if unsafe { EqualSid(ace_sid, want_sid) } != 0 {
                    found = true;
                    break;
                }
            }
        }

        // SAFETY: both allocations came from Win32 and must be LocalFree'd.
        unsafe {
            if !want_sid.is_null() {
                let _ = LocalFree(want_sid as _);
            }
            if !sd.is_null() {
                let _ = LocalFree(sd as _);
            }
        }
        found
    }

    fn writable_dir_rule(path: PathBuf) -> WindowsFilesystemPolicy {
        WindowsFilesystemPolicy {
            rules: vec![WindowsFilesystemRule {
                path,
                access: AccessMode::ReadWrite,
                is_file: false,
                source: CapabilitySource::User,
            }],
            unsupported: vec![],
        }
    }

    #[test]
    fn writable_rule_applies_sid_ace_and_reverts_on_drop() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        let policy = writable_dir_rule(path.clone());

        assert!(
            !dacl_contains_sid(&path, TEST_SESSION_SID),
            "test precondition: synthetic SID must not pre-exist on the DACL"
        );

        {
            let guard = AppliedDaclGrantsGuard::snapshot_and_apply(&policy, TEST_SESSION_SID)
                .expect("apply");
            assert_eq!(guard.entries.len(), 1);
            assert!(
                matches!(guard.entries[0], AppliedDaclGrant::Applied { .. }),
                "writable owned rule must record Applied; got {:?}",
                guard.entries[0]
            );
            assert!(
                dacl_contains_sid(&path, TEST_SESSION_SID),
                "during guard lifetime the SID's ACE must be present on the DACL"
            );
        } // guard drops here → revert

        assert!(
            !dacl_contains_sid(&path, TEST_SESSION_SID),
            "after guard drop the SID's ACE must be revoked"
        );
    }

    #[test]
    fn read_only_rule_is_skipped_no_dacl_change() {
        let dir = tempdir().expect("tempdir");
        let file = dir.path().join("note.txt");
        std::fs::write(&file, "x").expect("write file");
        let policy = WindowsFilesystemPolicy {
            rules: vec![WindowsFilesystemRule {
                path: file.clone(),
                access: AccessMode::Read,
                is_file: true,
                source: CapabilitySource::User,
            }],
            unsupported: vec![],
        };

        let guard =
            AppliedDaclGrantsGuard::snapshot_and_apply(&policy, TEST_SESSION_SID).expect("apply");
        assert_eq!(guard.entries.len(), 1, "one rule → one guard entry");
        assert!(
            matches!(guard.entries[0], AppliedDaclGrant::Skip),
            "read-only rule must record Skip; got {:?}",
            guard.entries[0]
        );
        assert!(
            !dacl_contains_sid(&file, TEST_SESSION_SID),
            "read-only rule must not add the SID to the DACL"
        );
        drop(guard);
    }

    #[test]
    fn mid_loop_grant_failure_reverts_already_applied() {
        // Two writable rules: the first (a real owned tempdir) applies fine,
        // the second points at a non-existent path so grant_sid_write_on_path
        // fails (GetNamedSecurityInfoW cannot read a DACL for a missing path).
        // The guard must revert the first before returning Err.
        let dir = tempdir().expect("tempdir");
        let ok_dir = dir.path().to_path_buf();
        let bad_path = dir.path().join("does-not-exist");

        let policy = WindowsFilesystemPolicy {
            rules: vec![
                WindowsFilesystemRule {
                    path: ok_dir.clone(),
                    access: AccessMode::ReadWrite,
                    is_file: false,
                    source: CapabilitySource::User,
                },
                WindowsFilesystemRule {
                    path: bad_path,
                    access: AccessMode::Write,
                    is_file: true,
                    source: CapabilitySource::User,
                },
            ],
            unsupported: vec![],
        };

        let result = AppliedDaclGrantsGuard::snapshot_and_apply(&policy, TEST_SESSION_SID);
        // Fail-closed: the second rule's non-existent path aborts the apply.
        // The ownership pre-check (`path_is_owned_by_current_user`) runs first
        // and surfaces `LabelApplyFailed` for a missing path; if a path somehow
        // passed ownership but failed the grant, `DaclApplyFailed` would
        // surface instead. Either way the apply must NOT succeed and the
        // already-applied first rule must be reverted.
        assert!(
            matches!(
                result,
                Err(NonoError::DaclApplyFailed { .. }) | Err(NonoError::LabelApplyFailed { .. })
            ),
            "mid-loop failure on a non-existent path must fail closed; got {result:?}"
        );

        // The first (ok) rule must have been reverted by the in-function
        // rollback — its DACL must no longer carry the synthetic SID.
        assert!(
            !dacl_contains_sid(&ok_dir, TEST_SESSION_SID),
            "first rule's SID ACE must be reverted after the mid-loop failure"
        );
    }

    // A package-SID-shaped (S-1-15-2-*) test SID for the ancestor-traverse guard;
    // on no real ACE, so revoke removes only what the guard added.
    const TEST_PACKAGE_SID: &str = "S-1-15-2-10-20-30-40-50-60-70";

    /// Plan 62-13 Task 3: the ancestor-traverse guard grants the package SID
    /// traverse on the USER-OWNED ancestors of a profile-deep cwd, and reverts
    /// every grant on Drop. A tempdir lives under `%TEMP%` (user-owned), so its
    /// immediate parent chain up to the user profile is owned by the current user.
    #[test]
    fn ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop() {
        let dir = tempdir().expect("tempdir");
        // A nested cwd so there is at least one owned ancestor below the tempdir
        // root: <temp>/<rand>/leaf — the parent <temp>/<rand> is user-owned.
        let leaf = dir.path().join("leaf");
        std::fs::create_dir(&leaf).expect("create leaf");
        let parent = dir.path().to_path_buf();

        assert!(
            !dacl_contains_sid(&parent, TEST_PACKAGE_SID),
            "test precondition: package SID must not pre-exist on the parent DACL"
        );

        {
            let guard = AppliedAncestorTraverseGuard::snapshot_and_apply(&leaf, TEST_PACKAGE_SID)
                .expect("apply ancestor traverse");
            // The tempdir parent is user-owned, so it must have received a grant.
            assert!(
                guard.applied.iter().any(|p| p == &parent),
                "the user-owned tempdir parent must be granted traverse; applied = {:?}",
                guard.applied
            );
            assert!(
                dacl_contains_sid(&parent, TEST_PACKAGE_SID),
                "during the guard lifetime the package SID's traverse ACE must be on the parent DACL"
            );
        } // guard drops → revert all

        assert!(
            !dacl_contains_sid(&parent, TEST_PACKAGE_SID),
            "after guard drop, the package SID's ancestor ACE must be revoked"
        );
    }

    /// The walk STOPS at the first non-owned ancestor (e.g. `C:\Users`, `C:\`):
    /// those are never granted, and reaching them relies on lowbox bypass-traverse.
    #[test]
    fn ancestor_traverse_stops_at_non_owned_ancestor() {
        let dir = tempdir().expect("tempdir");
        let leaf = dir.path().join("leaf");
        std::fs::create_dir(&leaf).expect("create leaf");

        let guard = AppliedAncestorTraverseGuard::snapshot_and_apply(&leaf, TEST_PACKAGE_SID)
            .expect("apply ancestor traverse");

        // C:\ (or whatever the drive root is) is owned by SYSTEM/TrustedInstaller,
        // never the current user, so it must NOT appear in the applied set.
        let root = leaf.ancestors().last().expect("a root ancestor exists");
        assert!(
            !guard.applied.iter().any(|p| p.as_path() == root),
            "the drive root ({}) must never be granted (non-owned)",
            root.display()
        );
        drop(guard);
    }

    // ── AppliedAncestorReadAttributesGuard tests (CPLT-01) ──────────────────
    //
    // A distinct SID to avoid cross-test DACL state leak with the traverse
    // guard tests above.
    const TEST_RA_PACKAGE_SID: &str = "S-1-15-2-200-300-400-500-600-700-800";

    /// CPLT-01: the ancestor-RA guard grants FILE_READ_ATTRIBUTES on the
    /// USER-OWNED ancestors of the walk target (the confined binary's resolution
    /// chain), and reverts every grant on Drop. Mirrors
    /// `ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop`.
    #[test]
    fn ancestor_read_attributes_grants_owned_ancestors_and_reverts_on_drop() {
        let dir = tempdir().expect("tempdir");
        // A nested target so there is at least one owned ancestor: <temp>/<rand>/leaf
        let leaf = dir.path().join("leaf");
        std::fs::create_dir(&leaf).expect("create leaf");
        let parent = dir.path().to_path_buf();
        // Ensure the parent is owned by the current user (elevation-robust).
        take_ownership_for_current_user(&parent);

        assert!(
            !dacl_contains_sid(&parent, TEST_RA_PACKAGE_SID),
            "test precondition: package SID must not pre-exist on the parent DACL"
        );

        {
            let guard = AppliedAncestorReadAttributesGuard::snapshot_and_apply(
                &leaf,
                TEST_RA_PACKAGE_SID,
            )
            .expect("apply ancestor read-attributes");
            // The tempdir parent is user-owned, so it must have received an RA grant.
            assert!(
                guard.applied.iter().any(|p| p == &parent),
                "the user-owned tempdir parent must be granted RA; applied = {:?}",
                guard.applied
            );
            assert!(
                dacl_contains_sid(&parent, TEST_RA_PACKAGE_SID),
                "during the guard lifetime the package SID's RA ACE must be on the parent DACL"
            );
        } // guard drops → revert all

        assert!(
            !dacl_contains_sid(&parent, TEST_RA_PACKAGE_SID),
            "after guard drop, the package SID's ancestor RA ACE must be revoked"
        );
    }

    /// CPLT-01 / D-04 structural split: the walk STOPS at the first non-owned
    /// ancestor (e.g. `C:\Users`, `C:\`). The drive root must never appear in
    /// `applied`, proving the runtime guard cannot grant system ancestors.
    #[test]
    fn ancestor_read_attributes_stops_at_non_owned_ancestor() {
        let dir = tempdir().expect("tempdir");
        let leaf = dir.path().join("leaf");
        std::fs::create_dir(&leaf).expect("create leaf");

        let guard = AppliedAncestorReadAttributesGuard::snapshot_and_apply(
            &leaf,
            TEST_RA_PACKAGE_SID,
        )
        .expect("apply ancestor read-attributes");

        // The drive root (C:\ or equivalent) is owned by SYSTEM/TrustedInstaller,
        // never the current user — it must NOT appear in the applied set (D-04).
        let root = leaf.ancestors().last().expect("a root ancestor exists");
        assert!(
            !guard.applied.iter().any(|p| p.as_path() == root),
            "the drive root ({}) must never be granted (non-owned); D-04 structural split",
            root.display()
        );
        drop(guard);
    }

    /// CPLT-01 / 77-04 gap closure: walking MULTIPLE targets that share a
    /// user-owned ancestor grants that ancestor EXACTLY ONCE (recorded once in
    /// `applied`) and reverts it exactly once on Drop — no double-grant, no
    /// double-revoke. Two sibling leaves under one owned tempdir parent.
    #[test]
    fn ancestor_read_attributes_dedups_shared_ancestor_across_targets() {
        let dir = tempdir().expect("tempdir");
        let parent = dir.path().to_path_buf();
        let leaf_a = dir.path().join("a");
        let leaf_b = dir.path().join("b");
        std::fs::create_dir(&leaf_a).expect("create a");
        std::fs::create_dir(&leaf_b).expect("create b");
        // Ensure the shared parent is owned by the current user (elevation-robust).
        take_ownership_for_current_user(&parent);

        assert!(
            !dacl_contains_sid(&parent, TEST_RA_PACKAGE_SID),
            "test precondition: package SID must not pre-exist on the shared parent DACL"
        );

        {
            let guard = AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets(
                &[&leaf_a, &leaf_b],
                TEST_RA_PACKAGE_SID,
            )
            .expect("apply multi-target ancestor read-attributes");

            // The shared owned parent must appear EXACTLY ONCE in `applied`.
            let count = guard
                .applied
                .iter()
                .filter(|p| p.as_path() == parent)
                .count();
            assert_eq!(
                count, 1,
                "shared ancestor must be granted exactly once across targets; applied = {:?}",
                guard.applied
            );
            assert!(
                dacl_contains_sid(&parent, TEST_RA_PACKAGE_SID),
                "during the guard lifetime the package SID's RA ACE must be on the shared parent DACL"
            );
        } // guard drops → revert all (single revoke for the shared ancestor)

        assert!(
            !dacl_contains_sid(&parent, TEST_RA_PACKAGE_SID),
            "after guard drop, the shared ancestor's RA ACE must be revoked (no residual)"
        );
    }

    /// CPLT-01 / 77-04 gap closure: multi-target walk grants the user-owned
    /// ancestors of EACH distinct target chain, and the D-04 stop is preserved
    /// across both (the drive root is never granted).
    #[test]
    fn ancestor_read_attributes_multi_target_covers_each_chain_and_stops_at_root() {
        let dir_a = tempdir().expect("tempdir a");
        let dir_b = tempdir().expect("tempdir b");
        let leaf_a = dir_a.path().join("leaf");
        let leaf_b = dir_b.path().join("leaf");
        std::fs::create_dir(&leaf_a).expect("create leaf a");
        std::fs::create_dir(&leaf_b).expect("create leaf b");
        let parent_a = dir_a.path().to_path_buf();
        let parent_b = dir_b.path().to_path_buf();
        // Ensure both distinct parents are owned by the current user (elevation-robust).
        take_ownership_for_current_user(&parent_a);
        take_ownership_for_current_user(&parent_b);

        let guard = AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets(
            &[&leaf_a, &leaf_b],
            TEST_RA_PACKAGE_SID,
        )
        .expect("apply multi-target ancestor read-attributes");

        // Each target's distinct user-owned immediate parent must be granted.
        assert!(
            guard.applied.iter().any(|p| p == &parent_a),
            "target A's owned parent must be granted; applied = {:?}",
            guard.applied
        );
        assert!(
            guard.applied.iter().any(|p| p == &parent_b),
            "target B's owned parent must be granted; applied = {:?}",
            guard.applied
        );
        // D-04 stop preserved across both chains: the drive root is never granted.
        let root = leaf_a.ancestors().last().expect("a root ancestor exists");
        assert!(
            !guard.applied.iter().any(|p| p.as_path() == root),
            "the drive root ({}) must never be granted across any chain (D-04)",
            root.display()
        );
        drop(guard);
    }
}
