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
    grant_sid_write_on_path, path_is_owned_by_current_user, revoke_sid_on_path, AccessMode, Result,
    WindowsFilesystemPolicy,
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
}
