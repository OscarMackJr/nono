//! Operator-ACL'd receipt sink (D-06/D-08) — the primary channel `nono.exe`
//! and `nono-agentd.exe` both write per-session [`nono::EnforcementReceipt`]
//! records through.
//!
//! # Shared plumbing, not a call site (Phase 118 Plan 05)
//!
//! This module builds the sink Plans 118-07 (`nono.exe` wiring) and 118-08
//! (`nono-agentd.exe` wiring) both call — see
//! `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs`'s
//! module header for why `nono-cli` has no `[lib]` target and why a symbol
//! reachable from only ONE binary's real call graph trips a `-D
//! warnings`-fatal `dead_code` lint on the other. Nothing in this file may be
//! added for only one binary's benefit; if such a need arises, it belongs in
//! that binary's own module instead.
//!
//! `nono-shell-broker.exe` does NOT use this module (Plan 118-06 gives it its
//! own sink code) — `nono-shell-broker` depends only on `nono` core, not
//! `nono-cli`, and cannot `#[path]`-include this file.
//!
//! # D-06: dedicated sink, primary — not the Windows Event Log
//!
//! The Windows Event Log's readability from a confined child was measured
//! (117/118 D-07) and NOT cleared either way. This sink sidesteps that
//! question entirely: it is a dedicated, directly-guarded directory, and it
//! is the PRIMARY channel a governance consumer retrieves receipts from — the
//! Event Log, if used at all, is at most a coarse pointer, never load-bearing
//! for RCPT-02's tamper-evidence claim (that pointer is not built by this
//! plan).
//!
//! # Sink location: `%PROGRAMDATA%\nono\receipts` (Claude's discretion, D-06)
//!
//! `%PROGRAMDATA%` (machine-wide) was chosen over `%LOCALAPPDATA%` (per-user)
//! because D-09's governance consumer is the operator/fleet admin — a
//! machine-wide location is what that consumer actually retrieves from,
//! matching `%PROGRAMDATA%\nono\nono-poc-root.pem`'s existing precedent
//! (`provision_windows.rs`).
//!
//! **Labelability was verified live on this development host** (per
//! `118-VALIDATION.md`'s Manual-Only table, which flagged
//! `%PROGRAMDATA%\nono\receipts` labelability as host-dependent and
//! undecided): creating a subdirectory under `C:\ProgramData` as the current
//! (non-elevated) user makes that user the NTFS owner of the new object
//! (standard Windows object-creation semantics — ownership is assigned to
//! the creating principal's token, not inherited from the parent), which
//! grants the implicit `WRITE_OWNER` [`nono::try_set_mandatory_label`]
//! needs. Verified with `icacls C:\ProgramData\<probe-dir> /setintegritylevel
//! Low`, which exercises the identical `SetNamedSecurityInfoW
//! (LABEL_SECURITY_INFORMATION)` privilege requirement and succeeded
//! (`Successfully processed 1 files`). This does **not** guarantee every
//! fleet host behaves identically (a host where `%ProgramData%\nono` is
//! pre-provisioned with a different owner, e.g. by an MSI running as
//! SYSTEM, could differ) — [`ensure_sink_guarded`] still fails closed
//! (returns `Err`, never silently proceeds unguarded) if labeling fails on
//! any given host, per D-04/T-118-15.
//!
//! # D-08: BOTH a DENY ACE and a `NO_READ_UP` mandatory label, unconditionally
//!
//! The supervisor and the confined child run as the **same user**, so an
//! ordinary allow-only DACL cannot structurally separate them. Neither
//! mechanism alone covers every arm (see
//! [`nono::deny_sid_on_path`]'s doc for the authoritative, empirically
//! corrected account of exactly what each mechanism blocks — Phase 118 Plan
//! 05 disproved that module's original claim about the mandatory label
//! covering a Medium-IL broker child; it does not). [`ensure_sink_guarded`]
//! therefore applies BOTH guards to the sink DIRECTORY, unconditionally,
//! regardless of which SIDs the CURRENT launch happens to have minted — the
//! directory is shared across every launch, so a session that minted no SID
//! must not leave it unguarded for the next session that does.
//!
//! **What is empirically proven** (live probe during this plan's design,
//! `CreateProcessAsUserW` with a real Low-IL primary token spawned via
//! [`nono::create_low_integrity_primary_token`], reading a file in a
//! directory labeled with this module's exact mask): a Low-IL subject's
//! read attempt fails (`cmd /c type` exits non-zero, "Access is denied"),
//! while the same test process at Medium IL reads the identical file
//! without restriction. The mandatory label protects against Low-IL-or-below
//! readers (the `WriteRestricted`/`LowIlPrimary`/`BrokerLaunchNoPty` arms'
//! eventual confined-child integrity level); it does **not** protect against
//! a Medium-IL reader — see [`nono::deny_sid_on_path`]'s doc for why that
//! residual boundary (a semi-trusted Medium-IL co-supervisor) is named and
//! accepted rather than silently assumed closed.
//!
//! # D-25: keyless chain, mutex discipline mirrors `advance_and_emit` (WR-21/WR-09)
//!
//! [`ReceiptWriter`]'s chain state ([`ReceiptChainState`]) carries no `key`
//! field — see `crates/nono/src/receipt_chain.rs`'s module doc for the exact
//! integrity claim this makes (tamper-evident, NOT "only the key holder
//! could have produced this"). [`ReceiptWriter::write_receipt`] locks the
//! chain-state mutex ONCE and holds it across the full build-event-bytes →
//! advance-chain → append-to-file sequence, mirroring
//! `crates/nono-cli/src/telemetry/mod.rs::advance_and_emit`'s WR-21
//! hardening (mutex held across the full sequence) and WR-09 (chain fields
//! private behind a single accessor).
//!
//! # Per-session file naming (D-15's "per-writer segment", this plan's scope)
//!
//! [`ReceiptWriter::new`] derives the sink file's name from `session_id`
//! alone (`<session_id>.jsonl`), which is exactly what this plan's
//! `nono.exe`/`nono-agentd.exe` scope needs: a `nono.exe` DirectCli session
//! and an `nono-agentd.exe` Daemon session are never the same session_id
//! (the daemon is its own supervisor for its own sessions — D-15 — and never
//! shares a session_id with a `nono.exe`-family launch). The broker's TWO-
//! receipts-per-session story (D-15) is Plan 118-06's own module and its own
//! sink code, not this file's concern; if a future plan ever calls this
//! module's [`ReceiptWriter::new`] twice with the SAME `session_id` from
//! TWO DIFFERENT PROCESSES concurrently, both processes append to the SAME
//! file with no cross-process locking beyond the OS's own
//! `OpenOptions::append` atomicity guarantee for a single `write` syscall —
//! that is a real, named limitation of this plan's scope, not a silently
//! assumed non-issue.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use nono::undo::ContentHash;
use nono::{
    deny_sid_on_path, hash_receipt_chain, hash_receipt_event, try_set_mandatory_label,
    EnforcementReceipt, NonoError, Result,
};
use serde::{Deserialize, Serialize};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
};
use windows_sys::Win32::System::SystemServices::{
    SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP, SYSTEM_MANDATORY_LABEL_NO_READ_UP,
};

/// The subdirectory `%PROGRAMDATA%` hosts the receipt sink under.
const RECEIPT_SINK_DIRNAME: &str = "receipts";

/// Deny-ACE access mask applied to the sink directory for each named SID
/// (D-08's DACL half). Comprehensive — read (reconnaissance, T-118-14),
/// write/delete (tampering, T-118-15), and execute — since no confined
/// child, on any arm, has any legitimate reason to touch the shared sink
/// directory at all. Mirrors `SESSION_SID_WRITE_MASK`'s shape
/// (`crates/nono/src/sandbox/windows.rs`, private to that module) but is a
/// DENY mask here, not a grant mask.
const RECEIPT_SINK_DENY_MASK: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_EXECUTE | DELETE;

/// Mandatory-label mask applied to the sink directory unconditionally
/// (D-08's label half): `NO_READ_UP` (the guard's actual job — see module
/// doc) plus `NO_EXECUTE_UP` (matching `label_mask_for_access_mode`'s
/// `AccessMode::Write` shape exactly — the supervisor itself must still be
/// able to WRITE receipts, so `NO_WRITE_UP` is deliberately absent).
const RECEIPT_SINK_LABEL_MASK: u32 =
    SYSTEM_MANDATORY_LABEL_NO_READ_UP | SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP;

/// Resolve the receipt sink directory: `%PROGRAMDATA%\nono\receipts`.
///
/// Falls back to the conventional `C:\ProgramData` literal if `%PROGRAMDATA%`
/// is unset — the same fallback shape `provision_windows.rs::programdata_pem_path`
/// already uses for the sibling POC-cert path. See this module's doc for the
/// live labelability check that decided `%PROGRAMDATA%` over `%LOCALAPPDATA%`.
#[must_use]
pub fn resolve_sink_dir() -> PathBuf {
    let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    PathBuf::from(base).join("nono").join(RECEIPT_SINK_DIRNAME)
}

/// Create `dir` if absent, then apply D-08's BOTH-not-either guard:
///
/// - A DENY ACE (session SID and/or package SID, whichever are `Some`) via
///   [`nono::deny_sid_on_path`], scoped to [`RECEIPT_SINK_DENY_MASK`].
/// - A `NO_READ_UP | NO_EXECUTE_UP` mandatory-label ACE via
///   [`nono::try_set_mandatory_label`], applied UNCONDITIONALLY (it does not
///   need a SID, and it covers arms where no SID was minted — the directory
///   is shared across every launch, per this module's doc).
///
/// Fails closed: any I/O or ACL/label failure returns `Err` and never
/// silently leaves the directory created-but-unguarded for a caller to
/// mistake as safe (T-118-15). The caller (Plans 118-07/08) is responsible
/// for D-04's "degrade visibly" behavior around this `Err` — this function
/// only reports failure faithfully.
///
/// # Errors
///
/// Returns `Err` if the directory cannot be created, if a DENY ACE cannot be
/// applied for a `Some` SID, or if the mandatory label cannot be applied.
pub fn ensure_sink_guarded(
    dir: &Path,
    session_sid: Option<&str>,
    package_sid: Option<&str>,
) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| {
        NonoError::Snapshot(format!(
            "receipt_sink: failed to create sink directory {}: {e}",
            dir.display()
        ))
    })?;

    if let Some(sid) = session_sid {
        deny_sid_on_path(dir, sid, RECEIPT_SINK_DENY_MASK)?;
    }
    if let Some(sid) = package_sid {
        deny_sid_on_path(dir, sid, RECEIPT_SINK_DENY_MASK)?;
    }

    try_set_mandatory_label(dir, RECEIPT_SINK_LABEL_MASK)?;

    Ok(())
}

/// Mutable per-writer chain state (D-25: KEYLESS — no `key` field, unlike
/// the CLI telemetry module's HMAC `ChainState`). See this module's doc and
/// `crates/nono/src/receipt_chain.rs`'s module doc for the exact integrity
/// claim this makes.
#[derive(Debug)]
struct ReceiptChainState {
    /// Current chain head (genesis = all-zero, mirroring
    /// `crate::audit::hash_chain`'s genesis behavior — `hash_receipt_chain`
    /// only ever sees this value via `Some` once `sequence > 0`).
    head: [u8; 32],
    /// Monotonically increasing per-writer event sequence number.
    sequence: u64,
}

/// One JSONL line's envelope: the receipt plus the chain-linkage fields a
/// consumer needs to recompute-and-compare (`nono receipt verify`, D-10).
/// Mirrors `crate::audit::AuditEventRecord`'s shape
/// (`crates/nono/src/audit.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReceiptRecord {
    /// Zero-based, monotonically increasing per-writer sequence number.
    sequence: u64,
    /// The chain head immediately BEFORE this record (`None` at genesis,
    /// `sequence == 0`).
    prev_head: Option<ContentHash>,
    /// `hash_receipt_event` of this record's canonical receipt JSON bytes.
    leaf_hash: ContentHash,
    /// The chain head immediately AFTER this record —
    /// `hash_receipt_chain(prev_head, leaf_hash)`.
    chain_head: ContentHash,
    /// The receipt itself.
    receipt: EnforcementReceipt,
}

/// A mutex-guarded, per-writer, KEYLESS chain-advance-and-append receipt
/// sink writer. One instance per session per writing process (D-15's
/// "per-writer segment", scoped to this plan's `nono.exe`/`nono-agentd.exe`
/// callers — see module doc).
#[derive(Debug)]
pub struct ReceiptWriter {
    inner: Mutex<ReceiptChainState>,
    session_id: String,
    file_path: PathBuf,
}

/// Validates that `session_id` is safe to embed directly in a filename:
/// ASCII alphanumeric, `-`, or `_` only, and non-empty. `session_id` is
/// D-05's opaque, never-path-derived identifier — this is defense in depth
/// (CLAUDE.md's path-security rule), not a claim that a malicious
/// `session_id` is expected in practice.
///
/// `pub(crate)` (not private): Plan 118-09's `receipt_commands.rs` reuses
/// this SAME check for the read-side `nono receipt show`/`verify`
/// `session_id` CLI argument, rather than re-deriving an independent
/// filename-safety rule that could drift from this one — a CLI-provided
/// `session_id` is exactly as untrusted as a supervisor-generated one from
/// this module's own path-security standpoint.
pub(crate) fn validate_session_id_for_filename(session_id: &str) -> Result<()> {
    if session_id.is_empty()
        || !session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(NonoError::Snapshot(format!(
            "receipt_sink: session_id is not safe for use as a filename: {session_id:?}"
        )));
    }
    Ok(())
}

/// Compute (without any I/O) the sink file path for `session_id` under
/// `sink_dir`, validating `session_id` is filename-safe first. Pure and
/// side-effect-free — unlike [`ReceiptWriter::new`], never creates the file.
///
/// `pub(crate)`: shared by [`ReceiptWriter::new`] (the write side, below)
/// and `receipt_commands.rs` (Plan 118-09's read side, D-10), so both agree
/// on the exact `<session_id>.jsonl` naming convention from ONE place
/// instead of two independently-maintained copies that could drift.
///
/// # Errors
///
/// Returns `Err` if `session_id` is not filename-safe.
pub(crate) fn session_file_path(session_id: &str, sink_dir: &Path) -> Result<PathBuf> {
    validate_session_id_for_filename(session_id)?;
    Ok(sink_dir.join(format!("{session_id}.jsonl")))
}

impl ReceiptWriter {
    /// Build a writer for `session_id`, whose records land in
    /// `<sink_dir>/<session_id>.jsonl`. Opens (creating if absent) the JSONL
    /// file in append mode once, to fail fast if the sink is unwritable —
    /// the handle itself is not retained; [`Self::write_receipt`] reopens by
    /// [`Self::file_path`] under the chain-state mutex on every call (this
    /// struct holds no live `File` handle, only the resolved path).
    ///
    /// # Errors
    ///
    /// Returns `Err` if `session_id` is not filename-safe, or if the sink
    /// file cannot be created/opened.
    pub fn new(session_id: String, sink_dir: &Path) -> Result<Self> {
        let file_path = session_file_path(&session_id, sink_dir)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map(|_file| ())
            .map_err(|e| {
                NonoError::Snapshot(format!(
                    "receipt_sink: failed to create/open sink file {}: {e}",
                    file_path.display()
                ))
            })?;

        Ok(Self {
            inner: Mutex::new(ReceiptChainState {
                head: [0u8; 32],
                sequence: 0,
            }),
            session_id,
            file_path,
        })
    }

    /// This writer's session id (the value passed to [`Self::new`]).
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// This writer's sink file path.
    #[must_use]
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Serialize `receipt`, advance the keyless chain, and append one JSONL
    /// record — all under a SINGLE mutex acquisition held across the full
    /// build+advance+write sequence (WR-21 discipline, D-25).
    ///
    /// On any serialization, chain, or I/O failure, returns `Err` and never
    /// panics; the chain state is only mutated AFTER the file write
    /// succeeds, so a failed write never advances `sequence`/`head` out of
    /// sync with what is actually durable on disk.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the chain-state mutex is poisoned, if `receipt`
    /// cannot be serialized, or if the sink file cannot be opened, written,
    /// or flushed.
    pub fn write_receipt(&self, receipt: &EnforcementReceipt) -> Result<()> {
        let mut state = self.inner.lock().map_err(|_| {
            NonoError::Snapshot("receipt_sink: chain-state mutex poisoned".to_string())
        })?;

        let event_bytes = serde_json::to_vec(receipt).map_err(|e| {
            NonoError::Snapshot(format!(
                "receipt_sink: failed to serialize enforcement receipt: {e}"
            ))
        })?;
        let leaf_hash = hash_receipt_event(&event_bytes);
        let prev_head = if state.sequence == 0 {
            None
        } else {
            Some(ContentHash::from_bytes(state.head))
        };
        let chain_head = hash_receipt_chain(prev_head.as_ref(), &leaf_hash);

        let record = ReceiptRecord {
            sequence: state.sequence,
            prev_head,
            leaf_hash,
            chain_head,
            receipt: receipt.clone(),
        };
        let mut line = serde_json::to_vec(&record).map_err(|e| {
            NonoError::Snapshot(format!(
                "receipt_sink: failed to serialize receipt record: {e}"
            ))
        })?;
        line.push(b'\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .map_err(|e| {
                NonoError::Snapshot(format!(
                    "receipt_sink: failed to open sink file {} for append: {e}",
                    self.file_path.display()
                ))
            })?;
        file.write_all(&line)
            .and_then(|()| file.flush())
            .map_err(|e| {
                NonoError::Snapshot(format!(
                    "receipt_sink: failed to append receipt record to {}: {e}",
                    self.file_path.display()
                ))
            })?;

        // Only advance in-memory state AFTER the write durably succeeded —
        // the MutexGuard (`state`) stays alive across the entire build,
        // hash, and write sequence above (WR-21 discipline).
        state.head = *chain_head.as_bytes();
        state.sequence = state.sequence.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nono::{EntryPath, LayerAttestationStatus, LayerId, LayerReceiptRow, SessionOutcome};
    use std::os::windows::ffi::OsStrExt;
    use tempfile::tempdir;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSidToSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        EqualSid, GetAce, ACCESS_DENIED_ACE, ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        PSID,
    };
    use windows_sys::Win32::System::SystemServices::ACCESS_DENIED_ACE_TYPE;

    // A unique synthetic SID for tests, shaped like `generate_session_sid`'s
    // `S-1-5-117-...` output (mirrors `dacl_guard.rs`'s `TEST_SESSION_SID`
    // convention). Pre-exists in NO real ACE.
    const TEST_SESSION_SID: &str = "S-1-5-117-55-66-77-88";

    fn sample_receipt(session_id: &str, outcome: SessionOutcome) -> EnforcementReceipt {
        EnforcementReceipt {
            schema_version: 1,
            session_id: session_id.to_string(),
            pid: 4242,
            entry_path: EntryPath::DirectCli,
            token_arm: None,
            outcome,
            layers: LayerId::ALL
                .iter()
                .map(|id| LayerReceiptRow {
                    id: *id,
                    status: LayerAttestationStatus::Confirmed,
                })
                .collect(),
        }
    }

    /// Returns `true` iff `path`'s DACL contains a DENY-type ACE for `sid`.
    /// Explicitly checks `AceType == ACCESS_DENIED_ACE_TYPE` before reading
    /// the embedded SID — a bare `SidStart` read without this check would
    /// also match an ALLOW ACE at the same struct offset (the exact bug
    /// Phase 118 Plan 02 found and fixed in the core crate's own test
    /// helper).
    fn dacl_contains_deny_ace_for_sid(path: &Path, sid: &str) -> bool {
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
                // SAFETY: `ace` was populated by GetAce above.
                let header = unsafe { &*(ace as *const windows_sys::Win32::Security::ACE_HEADER) };
                if u32::from(header.AceType) != ACCESS_DENIED_ACE_TYPE {
                    continue;
                }
                // SAFETY: AceType checked above; bytes are an ACCESS_DENIED_ACE.
                let ace_sid = unsafe {
                    (&(*(ace as *const ACCESS_DENIED_ACE)).SidStart) as *const u32 as PSID
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

    #[test]
    fn resolve_sink_dir_is_under_programdata_nono_receipts() {
        let dir = resolve_sink_dir();
        assert!(
            dir.ends_with(Path::new("nono").join("receipts")),
            "sink dir must end in nono\\receipts, got {}",
            dir.display()
        );
    }

    #[test]
    fn ensure_sink_guarded_applies_both_deny_ace_and_no_read_up_label() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path();

        ensure_sink_guarded(path, Some(TEST_SESSION_SID), None).expect("guard must apply");

        assert!(
            dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID),
            "sink dir DACL must contain a DENY ACE for the session SID"
        );

        let (_, mask) = nono::low_integrity_label_and_mask(path)
            .expect("sink dir must carry a mandatory label");
        assert_eq!(
            mask & SYSTEM_MANDATORY_LABEL_NO_READ_UP,
            SYSTEM_MANDATORY_LABEL_NO_READ_UP,
            "sink dir mandatory label must carry NO_READ_UP"
        );
    }

    /// Perturbation proof: WITHOUT calling `ensure_sink_guarded`, a fresh
    /// tempdir carries neither guard — proving the assertions above are
    /// actually exercising the guard, not a tautology.
    #[test]
    fn unguarded_dir_carries_neither_guard() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path();
        assert!(!dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID));
        assert!(nono::low_integrity_label_and_mask(path).is_none());
    }

    #[test]
    fn ensure_sink_guarded_applies_label_even_with_no_sids() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path();
        ensure_sink_guarded(path, None, None).expect("guard must apply label unconditionally");
        assert!(
            nono::low_integrity_label_and_mask(path).is_some(),
            "the mandatory label must be applied even when no SIDs were minted for this launch"
        );
    }

    #[test]
    fn write_receipt_chains_two_records_and_recomputes_from_disk() {
        let sink_dir = tempdir().expect("tempdir");
        let writer = ReceiptWriter::new("test-session-writeloop".to_string(), sink_dir.path())
            .expect("writer must construct");
        assert_eq!(writer.session_id(), "test-session-writeloop");

        let receipt_a = sample_receipt("test-session-writeloop", SessionOutcome::Ran);
        let receipt_b = sample_receipt("test-session-writeloop", SessionOutcome::Refused);
        writer.write_receipt(&receipt_a).expect("write 1");
        writer.write_receipt(&receipt_b).expect("write 2");

        let contents = std::fs::read_to_string(writer.file_path()).expect("read sink file");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "expected exactly 2 JSONL records");

        let record_a: ReceiptRecord = serde_json::from_str(lines[0]).expect("parse record 1");
        let record_b: ReceiptRecord = serde_json::from_str(lines[1]).expect("parse record 2");

        assert_eq!(record_a.sequence, 0);
        assert!(record_a.prev_head.is_none());
        assert_eq!(record_b.sequence, 1);
        assert_eq!(record_b.prev_head, Some(record_a.chain_head));

        // Recompute the chain independently from the on-disk leaf hashes and
        // confirm it matches what was stored — the fail-closed
        // recompute-and-compare shape `nono receipt verify` will use later
        // (D-10).
        let recomputed_head_a = hash_receipt_chain(None, &record_a.leaf_hash);
        assert_eq!(recomputed_head_a, record_a.chain_head);
        let recomputed_head_b = hash_receipt_chain(Some(&record_a.chain_head), &record_b.leaf_hash);
        assert_eq!(recomputed_head_b, record_b.chain_head);

        assert_eq!(record_a.receipt, receipt_a);
        assert_eq!(record_b.receipt, receipt_b);
    }

    /// Perturbation proof for the recompute-and-compare check above: an
    /// edited on-disk leaf hash must NOT recompute to the stored chain head.
    #[test]
    fn tampered_leaf_hash_fails_recompute_and_compare() {
        let sink_dir = tempdir().expect("tempdir");
        let writer = ReceiptWriter::new("test-session-tamper".to_string(), sink_dir.path())
            .expect("writer must construct");
        writer
            .write_receipt(&sample_receipt("test-session-tamper", SessionOutcome::Ran))
            .expect("write");

        let contents = std::fs::read_to_string(writer.file_path()).expect("read sink file");
        let record: ReceiptRecord =
            serde_json::from_str(contents.lines().next().expect("one line")).expect("parse record");

        // Tamper: hash a DIFFERENT event, simulating an edited receipt.
        let tampered_leaf = hash_receipt_event(b"tampered-event-bytes");
        assert_ne!(tampered_leaf, record.leaf_hash);
        let recomputed_from_tampered =
            hash_receipt_chain(record.prev_head.as_ref(), &tampered_leaf);
        assert_ne!(
            recomputed_from_tampered, record.chain_head,
            "recompute-and-compare must detect a tampered leaf hash"
        );
    }

    #[test]
    fn write_receipt_rejects_unsafe_session_id_for_filename() {
        let sink_dir = tempdir().expect("tempdir");
        let err = ReceiptWriter::new("../escape".to_string(), sink_dir.path())
            .expect_err("path-traversal-shaped session_id must be rejected");
        assert!(matches!(err, NonoError::Snapshot(_)));

        let err2 = ReceiptWriter::new(String::new(), sink_dir.path())
            .expect_err("empty session_id must be rejected");
        assert!(matches!(err2, NonoError::Snapshot(_)));
    }
}
