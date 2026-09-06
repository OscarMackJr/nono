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
//! # D-08 SCOPE CORRECTION (Phase 118 Plan 10, Task 3): WRITE-integrity only
//!
//! **The guard below protects the INTEGRITY of the sink, not the
//! CONFIDENTIALITY of the receipts in it. A confined child CAN read every
//! receipt file in the sink. This is measured, not theorised** — see
//! `118-10-SUMMARY.md` for the full Task 3 record. Read this section before
//! the historical account that follows it; the account describes what was
//! believed while the guard was built, and parts of it are now falsified.
//!
//! Measured on a real host, 2026-09-05, on BOTH confined arms:
//!
//! - `nono run --read <sink> -- cmd /c type <receipt>.jsonl` on the
//!   `WriteRestricted` arm printed the receipt (child confirmed confined:
//!   `token_arm: WriteRestricted`, `RestrictedToken: Confirmed`).
//! - `nono run -p claude-code --allow <sink> -- cmd /c type <receipt>.jsonl`
//!   on the broker/AppContainer arm printed the receipt
//!   (`app_container=true`, `child_exit_code=0`).
//! - WRITE was denied on both arms even with the sink granted writable, and
//!   no file was created. The supervisor retained read+write throughout.
//!
//! Why reads get through, in one line: **both guards are scoped to the sink
//! DIRECTORY OBJECT and neither reaches the files inside it.**
//! [`nono::deny_sid_on_path`] applies its ACE with `NO_INHERITANCE`, and the
//! mandatory label carries no `(OI)(CI)`, so `icacls` on a receipt file shows
//! no DENY ACE and no `Mandatory Label` line at all — only allow-ACEs
//! inherited from `%PROGRAMDATA%`, including the invoking user, which is the
//! same user the confined child runs as. Creating a file, by contrast, is an
//! operation on the DIRECTORY, where the DENY ACE does apply — which is
//! exactly why write-denial holds and read-denial does not.
//!
//! **What D-08 therefore claims, and only this:** a confined child cannot
//! CREATE, overwrite, or delete entries in the sink, so it cannot forge or
//! destroy the enforcement record. That is the property RCPT-02's
//! tamper-evidence rests on, and it is intact. **What D-08 does NOT claim:**
//! that receipts are unreadable from inside containment. A confined child can
//! read every session's layer census, `session_id`, `pid`, `entry_path` and
//! `token_arm` — content-free by construction (D-05/D-14), so no user data is
//! exposed, but "which layers are inert on this host" is legible to it.
//! Narrowing the claim rather than widening the guard was the operator's
//! decision at the Task 3 checkpoint; widening it (an `(OI)(CI)` deny at this
//! call site, or per-file guards) remains available and unimplemented.
//!
//! # D-08 as built: BOTH a DENY ACE and a `NO_READ_UP` mandatory label
//!
//! *(Historical account — read the SCOPE CORRECTION above first. The
//! read-protection claims in this section and the next are the ones Task 3
//! falsified; they are kept because they record what the guard was designed
//! to do and why, which the correction above is otherwise unintelligible
//! without.)*
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
//! **⚠ The paragraph immediately below is the claim Task 3 falsified for the
//! shipped guard.** Its probe result is not disputed, but it does not
//! generalise to this sink: the probe's labeled object was the object being
//! read, whereas the sink's label sits on the DIRECTORY with no `(OI)(CI)`
//! and so never attaches to a receipt file. The probe's exact object and
//! inheritance shape were not recorded at the time, so the discrepancy
//! cannot be fully reconstructed — which is itself the lesson: a probe that
//! does not pin the object shape it measured cannot be relied on to describe
//! a differently-shaped production object.
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
//!
//! # Discretionary decisions (Phase 118 closeout)
//!
//! Plan 118-10 consolidates the record of every item `118-CONTEXT.md`'s
//! "Claude's Discretion" list left open, so a future reader does not have
//! to reconstruct them from SUMMARYs across Plans 118-01/05/06/09.
//!
//! **1. Sink location and layout:** `%PROGRAMDATA%\nono\receipts`
//! (machine-wide), one JSONL file per session (`<session_id>.jsonl` for
//! `nono.exe`/`nono-agentd.exe`, `<session_id>.broker.jsonl` for the
//! broker — 118-06). Chosen over `%LOCALAPPDATA%` because D-09's
//! governance consumer is the operator/fleet admin, who retrieves from a
//! machine-wide location, not a per-user one — see the "Sink location"
//! section above for the live labelability check that confirmed this
//! choice is actually viable on this host, not merely convenient.
//!
//! **2. Retention and rotation policy: NONE — unbounded growth, deferred.**
//! No file-size cap, no age-based pruning, and no `nono receipt cleanup`
//! command were built in this phase (contrast `AuditCommands::Cleanup`,
//! which the receipt command family — D-10 — deliberately did NOT mirror
//! for this reason). This is an explicit choice, not an oversight: every
//! session appends exactly one small JSONL line to its own per-session
//! file, so growth is bounded by session count, not by a single
//! unboundedly-growing log — the operational pressure a rotation policy
//! would normally exist to relieve is comparatively low. **Constraint on
//! any FUTURE rotation implementation, stated here so it cannot be missed**
//! (per `118-CONTEXT.md`'s explicit instruction): rotation must never
//! silently truncate a chain segment in a way that makes a retained
//! receipt unverifiable. Concretely, that means a future rotation design
//! may delete an ENTIRE per-session segment (verify already treats a
//! missing segment as a fail-closed error, not a pass — see
//! `receipt_commands.rs`), but must never truncate a segment mid-chain,
//! since [`ReceiptWriter::write_receipt`]'s `prev_head` linkage would then
//! make every record AFTER the truncation point unverifiable while still
//! appearing on disk as if it were.
//!
//! **3. The coarse Windows Event Log pointer (D-06) was NOT implemented —
//! this is deliberate, not an oversight.** No code in this codebase
//! registers a receipt-specific Event Log source or writes a "receipt N
//! emitted, chain head X" pointer event anywhere. D-06 named this pointer
//! as *at most* a coarse aid, explicitly never load-bearing for RCPT-02's
//! tamper-evidence claim — the dedicated sink built by this module is
//! what actually carries that claim. Justification for dropping it
//! entirely rather than building a best-effort version: it carries three
//! independent conditional dependencies (`SECURITY_LAYER` initialized,
//! telemetry enabled, `RegisterEventSourceW` succeeding) for a benefit
//! that, per D-07's live measurement on this host, is *itself* of
//! unresolved value — the `INTERACTIVE` SID's `0x3` (read+write) access
//! bit on the Application channel means the Event Log's readability
//! boundary from a confined child was never cleared as safe in the first
//! place (see D-07's full account above), so a pointer event would point a
//! governance consumer at a channel this phase never proved is
//! appropriately access-controlled. Building it anyway would have added
//! real complexity to close a gap that, per D-06's own reasoning, this
//! sink module already closes by a different, verified route. The
//! open question this drops (empirical proof of Event Log readability from
//! a real confined child) remains a tracked deferred item, not a lapsed
//! one — see `118-CONTEXT.md`'s `<deferred>` section.
//!
//! **4. `entry_path` and `token_arm` ARE first-class fields on
//! [`nono::EnforcementReceipt`]** (`crates/nono/src/receipt.rs`), closing
//! this discretion item explicitly: `entry_path: EntryPath` is mandatory
//! on every receipt (`DirectCli` / `Broker` / `Daemon`, the same identity
//! vocabulary D-15's cross-binary correlation story depends on), and
//! `token_arm: Option<TokenArm>` is `None` specifically for
//! `EntryPath::Broker`/`EntryPath::Daemon` (which bypass the token-arm
//! construction this field names) and `Some` otherwise. Both fields are
//! populated by the supervisor from its own observed control flow, never
//! from confined-process input (D-19).

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
/// (D-08's DACL half). The mask requests read, write/delete and execute,
/// since no confined child has any legitimate reason to touch the shared sink
/// directory. Mirrors `SESSION_SID_WRITE_MASK`'s shape
/// (`crates/nono/src/sandbox/windows.rs`, private to that module) but is a
/// DENY mask here, not a grant mask.
///
/// **⚠ What this mask REQUESTS is not what the guard DELIVERS (Phase 118 Plan
/// 10 Task 3).** This doc previously claimed the deny was "Comprehensive —
/// read (reconnaissance, T-118-14), write/delete (tampering, T-118-15)". The
/// read half was measured false: [`nono::deny_sid_on_path`] applies this mask
/// with `NO_INHERITANCE`, so the ACE lands on the sink DIRECTORY OBJECT only
/// and never reaches the receipt FILES inside it. A confined child reads
/// every receipt on both arms; `icacls` on a receipt file shows no DENY ACE
/// at all. **Only the write/delete half (T-118-15) is actually enforced** —
/// creating a file is an operation on the directory, where the ACE does
/// apply. T-118-14 (read/reconnaissance) is NOT closed by this mask. See the
/// module doc's "D-08 SCOPE CORRECTION" section.
const RECEIPT_SINK_DENY_MASK: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_EXECUTE | DELETE;

/// Mandatory-label mask applied to the sink directory unconditionally
/// (D-08's label half): `NO_READ_UP` plus `NO_EXECUTE_UP` (matching
/// `label_mask_for_access_mode`'s `AccessMode::Write` shape exactly — the
/// supervisor itself must still be able to WRITE receipts, so `NO_WRITE_UP`
/// is deliberately absent).
///
/// **⚠ `NO_READ_UP` here is inert, not "the guard's actual job" (Phase 118
/// Plan 10 Task 3).** This doc previously called it exactly that. Two
/// independent measurements falsified it: (1) [`nono::try_set_mandatory_label`]
/// pins the object to a LOW integrity RID (hardcoded `LW`), and Windows denies
/// only subjects strictly BELOW the object's level — a Low-IL confined child
/// is EQUAL, not below, so the label cannot block it; (2) the label carries no
/// `(OI)(CI)`, so it never attaches to the receipt files regardless of any
/// subject's IL. Retained because it costs nothing and is correct for a
/// sub-Low subject, but it must not be described as providing read protection.
/// See the module doc's "D-08 SCOPE CORRECTION" section.
const RECEIPT_SINK_LABEL_MASK: u32 =
    SYSTEM_MANDATORY_LABEL_NO_READ_UP | SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP;

/// Resolve the receipt sink directory: `%PROGRAMDATA%\nono\receipts`.
///
/// Falls back to the conventional `C:\ProgramData` literal if `%PROGRAMDATA%`
/// is unset — the same fallback shape `provision_windows.rs::programdata_pem_path`
/// already uses for the sibling POC-cert path. See this module's doc for the
/// live labelability check that decided `%PROGRAMDATA%` over `%LOCALAPPDATA%`.
///
/// # CLAUDE.md Path Handling / Phase 118 review CR-04
///
/// `%PROGRAMDATA%` is a **user-settable environment variable** — CLAUDE.md's
/// Path Handling section is explicit that such values must never be trusted
/// raw ("Validate environment variables before use. Never assume `HOME`,
/// `TMPDIR`, etc. are trustworthy."). Trusting it raw here would let a user
/// relocate the guard, the writer, AND the write together (all three would
/// then succeed against the redirected path), so `record_receipt_write_outcome`
/// — the ONLY place that consults the admin-only
/// `HKLM\SOFTWARE\Policies\nono\RequireReceipts` control — would never even
/// run. This function therefore validates the resolved base before using it,
/// and FAILS CLOSED (`Err`, never a silent fallback to the untrusted value)
/// when validation does not hold:
///
/// - the resolved base must be an absolute path, and
/// - the resolved base must NOT be owned by the current (invoking) user.
///   The real, machine-wide `%ProgramData%` root is provisioned by the OS
///   installer and is owned by a system principal, never by an ordinary
///   interactive user — [`nono::path_is_owned_by_current_user`] is the same
///   ownership-equality primitive `try_set_mandatory_label`'s own
///   `WRITE_OWNER` check already relies on for this exact class of decision.
///   A directory the CURRENT user owns is, by definition, one that user (or
///   any process running as them) could have created themselves — exactly
///   the redirection this check exists to catch.
///
/// This is the documented, review-sanctioned alternative to resolving the
/// path via `SHGetKnownFolderPath(FOLDERID_ProgramData)`: it requires no new
/// FFI surface, reuses an existing core primitive, and still converts a
/// user-controlled input into a fail-closed decision rather than a silently
/// accepted redirection. Widening this to a true OS-sourced resolution
/// remains available as a future hardening step.
///
/// # Errors
///
/// Returns `Err` if `%PROGRAMDATA%` (or its `C:\ProgramData` fallback)
/// resolves to a relative path, if its ownership cannot be determined, or if
/// it is owned by the current user (a redirection signal).
pub fn resolve_sink_dir() -> Result<PathBuf> {
    Ok(resolve_sink_base()?.join("nono").join(RECEIPT_SINK_DIRNAME))
}

/// The validation half of [`resolve_sink_dir`], split out so it is
/// unit-testable against ARBITRARY candidate base paths (a real
/// `%ProgramData%`-shaped path, and a user-owned tempdir standing in for a
/// hostile redirection) without mutating the real `PROGRAMDATA` process
/// environment variable — CLAUDE.md's env-var test discipline requires
/// saving/restoring any such mutation, and Rust's parallel test runner makes
/// that fragile for a value this many other tests could transitively read.
/// See [`resolve_sink_dir`]'s doc for the exact validation this performs and
/// why.
fn validate_and_join_sink_base(base: PathBuf) -> Result<PathBuf> {
    if !base.is_absolute() {
        return Err(NonoError::Snapshot(format!(
            "receipt_sink: %PROGRAMDATA% did not resolve to an absolute path ({}) — refusing to \
             write receipts to a relative, redirectable location",
            base.display()
        )));
    }
    let owned_by_current_user = nono::path_is_owned_by_current_user(&base).map_err(|e| {
        NonoError::Snapshot(format!(
            "receipt_sink: could not determine the owner of %PROGRAMDATA% ({}): {e} — refusing \
             to trust an unverifiable machine-wide root",
            base.display()
        ))
    })?;
    if owned_by_current_user {
        return Err(NonoError::Snapshot(format!(
            "receipt_sink: %PROGRAMDATA% ({}) is owned by the current user, not a system \
             principal — this is not the real, machine-wide ProgramData root and would let a \
             user-set environment variable silently relocate the receipt sink (defeating the \
             admin-only RequireReceipts control); refusing to write receipts there",
            base.display()
        )));
    }
    Ok(base.join("nono").join(RECEIPT_SINK_DIRNAME))
}

/// The VALIDATED machine-wide `%ProgramData%` base — i.e. exactly what
/// [`resolve_sink_dir`] resolves and checks, but WITHOUT the
/// `nono\receipts` suffix.
///
/// # Phase 118 debug `broker-receipt-not-written`
///
/// `nono-shell-broker.exe` writes the second of D-15's two per-session
/// receipts and must land in the SAME directory this crate's writer uses. It
/// used to re-derive that directory from its own `%PROGRAMDATA%` — but
/// `exec_strategy_windows::launch` gives the broker a clone of the CONFINED
/// CHILD's sanitized environment, in which `PROGRAMDATA` has been rewritten to
/// the sandbox-local redirect `<runtime_root>\programdata`, so every broker
/// receipt was written to a workdir-local path instead of the machine-wide
/// sink. The redirect cannot be undone on the broker's environment because the
/// broker forwards that environment verbatim to the confined child
/// (`lpEnvironment = NULL`).
///
/// This function is therefore the single source of truth: `nono.exe` resolves
/// and validates the base ONCE and hands it to the broker on argv
/// (`--receipt-sink-base`), which makes D-15's same-directory invariant
/// structural rather than coincidental. The broker re-runs the same
/// absolute/ownership validation on receipt as defense in depth.
///
/// # Errors
///
/// Same as [`resolve_sink_dir`]: `Err` if `%PROGRAMDATA%` (or its
/// `C:\ProgramData` fallback) is relative, if its ownership cannot be
/// determined, or if it is owned by the current user (a redirection signal).
pub fn resolve_sink_base() -> Result<PathBuf> {
    let base = PathBuf::from(
        std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string()),
    );
    // Validate, then hand back the BASE. The `nono\receipts` join belongs to
    // each writer: this crate's own [`resolve_sink_dir`] performs it, and the
    // broker performs its own (D-15: same directory, never shared code).
    let validated = validate_and_join_sink_base(base.clone())?;
    debug_assert!(validated.starts_with(&base));
    Ok(base)
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

/// RAII guard that revokes a per-launch synthetic SID's sink-directory DENY
/// ACE when the confined session that minted it ends.
///
/// # Why this exists — Phase 118 review CR-05
///
/// [`ensure_sink_guarded`] adds a DENY ACE for a caller-supplied SID to the
/// SHARED, PERMANENT sink directory on every receipt emission. Every SID it
/// is ever called with is unique per launch — `generate_session_sid()`'s
/// UUID-derived session SID on the CLI producer
/// (`exec_strategy_windows::launch::spawn_windows_child`), and the daemon's
/// per-run AppContainer package SID, derived from a `getrandom`-seeded tenant
/// id (`agent_daemon::launch::launch_agent`) — so with nothing ever calling
/// [`nono::revoke_sid_on_path`] for this directory, the sink's DACL grows one
/// ACE per launch, forever, until the Win32 `AclSize` (`u16`) ceiling is hit.
/// `revoke_sid_on_path` already exists and is tested
/// (`deny_then_revoke_sid_round_trips_on_tempdir`) to remove both allow and
/// deny ACEs for a trustee; nothing called it for this directory. This guard
/// is that missing caller, scoped to exactly the SID this session applied.
///
/// # Own-only, by construction
///
/// Both source SIDs this guard is ever constructed with (the CLI's session
/// SID, the daemon's package SID) are minted fresh per launch from a
/// UUID/`getrandom` source — no two launches ever share a SID. Revoking "this
/// session's own SID" can therefore never touch an ACE a DIFFERENT,
/// concurrent session depends on: there is no SID-collision case to guard
/// against here, unlike `labels_guard::AppliedLabelsGuard`'s adopt-vs-apply
/// distinction (which exists because that guard's mandatory label targets
/// caller-supplied, potentially SHARED workspace paths). This guard never
/// walks the sink's DACL looking for SIDs to clean up — it only ever revokes
/// the one SID string it was constructed with.
///
/// # Session lifetime, not call lifetime
///
/// [`ensure_sink_guarded`] is called once per receipt EMISSION, and a single
/// confined session can emit more than one receipt with the SAME SID (a
/// `Ran` receipt followed by a corrective `Refused` record on a failed
/// `ResumeThread`, for example — re-applying the identical DENY ACE is an
/// idempotent no-op, not a duplicate). This guard must therefore be
/// constructed exactly ONCE per session, as early as the SID is known —
/// before the first possible emission — and kept alive for the session's
/// full duration by its caller: moved into the long-lived session owner
/// (`exec_strategy_windows::supervisor::WindowsSupervisedChild` on the CLI
/// producer, `agent_daemon::reap::AgentTenant` on the daemon producer) once
/// the session is confirmed to actually run, or left as a local variable
/// that reverts automatically via `Drop` on any of that session's
/// early-return failure paths (Rust drops locals at every `return`,
/// including from inside a `match` arm or an `if let Err` block) — the
/// confined child never runs (or was already terminated) on every such path,
/// so revoking immediately is correct.
///
/// # Fail-secure, unconditional revoke
///
/// [`nono::revoke_sid_on_path`] on a SID that was never actually applied
/// (e.g. because the sink directory could not be resolved, created, or
/// labeled at all, so the DENY ACE this guard exists to clean up was never
/// written) is a documented no-op: it walks the DACL, finds no matching ACE,
/// and writes the unchanged DACL back successfully. This guard's `Drop` is
/// therefore safe to run unconditionally, without first proving the ACE was
/// written.
///
/// A revoke failure never panics and never fails the session: the session's
/// outcome is already decided by the time it ends, so `Drop` only warns
/// (D-04's "degrade visibly, never fatally" posture) — a stale DENY ACE is
/// fail-secure on its own ([`nono::revoke_sid_on_path`]'s own doc: "it can
/// only keep denying, never grant something it shouldn't").
#[derive(Debug)]
pub struct SinkSidGuard {
    dir: PathBuf,
    sid: String,
}

impl SinkSidGuard {
    /// Construct a guard that will revoke `sid`'s DENY ACE from `dir` when
    /// dropped. Does not itself apply anything — [`ensure_sink_guarded`]
    /// remains the sole apply site; this is purely the missing revoke half
    /// CR-05 found absent.
    pub fn new(dir: PathBuf, sid: String) -> Self {
        Self { dir, sid }
    }
}

impl Drop for SinkSidGuard {
    fn drop(&mut self) {
        if let Err(e) = nono::revoke_sid_on_path(&self.dir, &self.sid) {
            tracing::warn!(
                dir = %self.dir.display(),
                sid = %self.sid,
                error = %e,
                "receipt_sink: failed to revoke this session's sink DENY ACE on session end; it \
                 may remain on the sink directory's DACL (Phase 118 review CR-05)"
            );
        }
    }
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
/// No `session_id` field: it is consumed by [`ReceiptWriter::new`] solely to
/// derive `file_path` (via [`session_file_path`]), and `file_path` already
/// encodes it — storing it as well gave every build a field nothing read.
/// A `session_id()`/`file_path()` accessor pair existed here through Plans
/// 118-05..118-09 on the hypothesis (named in `118-07-SUMMARY.md`) that the
/// read side would need to recover them from a writer instance. Plan 118-09
/// built that read side and took [`session_file_path`] instead — a pure
/// function both sides share — because `receipt_commands.rs` is handed a
/// `session_id` string by the CLI and never holds a `ReceiptWriter` at all.
/// The accessors were removed at phase close-out rather than carried as
/// permanently-dead production surface.
#[derive(Debug)]
pub struct ReceiptWriter {
    inner: Mutex<ReceiptChainState>,
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
    /// the handle itself is not retained; [`Self::write_receipt`] reopens the
    /// resolved `file_path` under the chain-state mutex on every call (this
    /// struct holds no live `File` handle, only the resolved path).
    ///
    /// # Phase 118 review CR-02: RESUME the existing chain, never assume genesis
    ///
    /// Previously this always initialised `ReceiptChainState { head: [0; 32],
    /// sequence: 0 }` unconditionally — correct for a brand-new segment, but
    /// WRONG for any segment a prior `ReceiptWriter` instance already wrote
    /// to (every real emission site constructs a fresh writer per call, so
    /// this was every second-or-later record on any segment). The SECOND
    /// record on an existing segment would then re-emit `sequence: 0,
    /// prev_head: null`, which `receipt_commands.rs::verify_records` reports
    /// as `"a record was reordered or deleted"` — an honest, system-generated
    /// record indistinguishable from tampering.
    ///
    /// This now reads the segment's LAST record (if any) before opening for
    /// append, and resumes from its `sequence + 1` / `chain_head` — the exact
    /// state [`Self::write_receipt`] would have left in memory had the SAME
    /// writer instance produced every prior record. Fails CLOSED
    /// (`Err`) if an existing segment's tail line cannot be parsed: silently
    /// restarting at genesis on a corrupt tail would re-introduce the exact
    /// ambiguity this fix exists to remove (a fresh `sequence: 0` record
    /// appended after unparseable bytes is just as tamper-shaped as the
    /// original bug). D-25's keyless discipline is preserved — this reads
    /// only `sequence` and `chain_head`, never a `key` field (none exists).
    ///
    /// # Errors
    ///
    /// Returns `Err` if `session_id` is not filename-safe, if an existing
    /// segment cannot be read for a reason other than "does not exist yet",
    /// if an existing segment's last record cannot be parsed, or if the sink
    /// file cannot be created/opened.
    pub fn new(session_id: String, sink_dir: &Path) -> Result<Self> {
        let file_path = session_file_path(&session_id, sink_dir)?;

        let (head, sequence) = match std::fs::read_to_string(&file_path) {
            Ok(existing) => match existing.lines().rev().find(|l| !l.trim().is_empty()) {
                Some(last_line) => {
                    let last: ReceiptRecord = serde_json::from_str(last_line).map_err(|e| {
                        NonoError::Snapshot(format!(
                            "receipt_sink: refusing to append to {} — its last record could not \
                             be parsed (truncated or corrupt): {e}",
                            file_path.display()
                        ))
                    })?;
                    (*last.chain_head.as_bytes(), last.sequence.saturating_add(1))
                }
                // File exists but is empty (or all-blank lines): genesis.
                None => ([0u8; 32], 0),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => ([0u8; 32], 0),
            Err(e) => {
                return Err(NonoError::Snapshot(format!(
                    "receipt_sink: failed to read existing sink file {} to resume its chain: {e}",
                    file_path.display()
                )))
            }
        };

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
            inner: Mutex::new(ReceiptChainState { head, sequence }),
            file_path,
        })
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

    // A SECOND, DISTINCT synthetic SID standing in for a different session's
    // (or a concurrent process's) own DENY ACE on the shared sink directory —
    // used by `SinkSidGuard`'s own-only regression test below (CR-05 rule 1).
    const TEST_OTHER_SESSION_SID: &str = "S-1-5-117-11-22-33-44";

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
        let dir = resolve_sink_dir()
            .expect("the real, machine-wide %PROGRAMDATA% root on this host must validate");
        assert!(
            dir.ends_with(Path::new("nono").join("receipts")),
            "sink dir must end in nono\\receipts, got {}",
            dir.display()
        );
    }

    /// Phase 118 review CR-04: a user-owned directory standing in for a
    /// hostile `%PROGRAMDATA%` redirection (`set
    /// PROGRAMDATA=C:\Users\me\fake`) must be REJECTED, not silently
    /// accepted. `tempdir()` creates a directory owned by the current test
    /// process's user — exactly the ownership shape a redirected
    /// `%PROGRAMDATA%` would have, and exactly what
    /// `nono::path_is_owned_by_current_user` is checking for.
    #[test]
    fn validate_and_join_sink_base_rejects_a_user_owned_redirection() {
        let dir = tempdir().expect("tempdir");
        let err = validate_and_join_sink_base(dir.path().to_path_buf())
            .expect_err("a user-owned base must be rejected as a redirection signal");
        assert!(matches!(err, NonoError::Snapshot(_)));
    }

    /// Perturbation proof for the test above: a RELATIVE base must also be
    /// rejected outright (never silently joined onto the current directory),
    /// independent of the ownership check.
    #[test]
    fn validate_and_join_sink_base_rejects_a_relative_path() {
        let err = validate_and_join_sink_base(PathBuf::from("relative\\path"))
            .expect_err("a relative base must be rejected");
        assert!(matches!(err, NonoError::Snapshot(_)));
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

    /// CR-05: a session's own DENY ACE is present while its `SinkSidGuard` is
    /// alive, and GONE once the guard drops — proving `SinkSidGuard` actually
    /// performs the missing revoke `ensure_sink_guarded` never did on its
    /// own.
    #[test]
    fn sink_sid_guard_revokes_its_own_deny_ace_on_drop() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path();

        ensure_sink_guarded(path, Some(TEST_SESSION_SID), None).expect("guard must apply");
        assert!(
            dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID),
            "sink dir DACL must contain the session's DENY ACE while the session is live"
        );

        {
            let _guard = SinkSidGuard::new(path.to_path_buf(), TEST_SESSION_SID.to_string());
            assert!(
                dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID),
                "the ACE must still be present for the guard's entire lifetime"
            );
        }
        // `_guard` has dropped here.

        assert!(
            !dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID),
            "CR-05: the session's own DENY ACE must be revoked once its SinkSidGuard drops \
             (this is the regression this fix closes — without it, the ACE accumulates forever)"
        );
    }

    /// CR-05 rule 1 (own-only): revoking THIS session's SID must never touch
    /// a DIFFERENT SID's pre-existing DENY ACE on the same shared directory —
    /// simulating a concurrent session (or residue from a prior one) whose
    /// guard has not yet run. Without this test, an implementation that
    /// (incorrectly) walked the whole DACL revoking every deny ACE it found
    /// would pass `sink_sid_guard_revokes_its_own_deny_ace_on_drop` above
    /// while silently un-guarding every OTHER session sharing the directory.
    #[test]
    fn sink_sid_guard_drop_never_touches_a_different_sids_deny_ace() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path();

        // Simulate a different session's own, still-live DENY ACE.
        ensure_sink_guarded(path, Some(TEST_OTHER_SESSION_SID), None)
            .expect("other session's guard must apply");
        // This session's own DENY ACE, on the same directory.
        ensure_sink_guarded(path, Some(TEST_SESSION_SID), None)
            .expect("this session's guard must apply");

        assert!(dacl_contains_deny_ace_for_sid(path, TEST_OTHER_SESSION_SID));
        assert!(dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID));

        drop(SinkSidGuard::new(
            path.to_path_buf(),
            TEST_SESSION_SID.to_string(),
        ));

        assert!(
            !dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID),
            "this session's own ACE must be gone after its guard drops"
        );
        assert!(
            dacl_contains_deny_ace_for_sid(path, TEST_OTHER_SESSION_SID),
            "own-only (CR-05 rule 1): a DIFFERENT session's DENY ACE must SURVIVE this guard's \
             drop — this guard must revoke ONLY the SID it was constructed with"
        );
    }

    /// Perturbation proof for the own-only test above: without it, a
    /// same-shaped defect (revoking by walking the DACL instead of by exact
    /// SID) would still pass `sink_sid_guard_revokes_its_own_deny_ace_on_drop`
    /// alone. This test independently confirms `SinkSidGuard::new` is a pure
    /// constructor with NO apply side effect of its own — constructing (and
    /// immediately dropping) a guard for a SID that was NEVER granted an ACE
    /// must be a safe no-op, never an error and never a panic (D-04: a
    /// revoke-time failure only warns).
    #[test]
    fn sink_sid_guard_drop_is_a_safe_no_op_when_nothing_was_ever_applied() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path();
        assert!(!dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID));

        drop(SinkSidGuard::new(
            path.to_path_buf(),
            TEST_SESSION_SID.to_string(),
        ));

        assert!(!dacl_contains_deny_ace_for_sid(path, TEST_SESSION_SID));
    }

    #[test]
    fn write_receipt_chains_two_records_and_recomputes_from_disk() {
        let sink_dir = tempdir().expect("tempdir");
        let writer = ReceiptWriter::new("test-session-writeloop".to_string(), sink_dir.path())
            .expect("writer must construct");
        // Asserts BEHAVIOR (the writer derived its sink file from the
        // session id it was given, pinning the `<session_id>.jsonl`
        // convention `receipt_commands.rs`'s read side resolves
        // independently via `session_file_path`) rather than merely that a
        // getter returns its constructor argument, which is what the removed
        // `session_id()` accessor's assertion did. Perturbation-proved:
        // changing the production naming convention to `.PERTURBED` fails
        // this assertion with left/right shown.
        assert_eq!(
            writer.file_path.file_name().and_then(|n| n.to_str()),
            Some("test-session-writeloop.jsonl"),
        );

        let receipt_a = sample_receipt("test-session-writeloop", SessionOutcome::Ran);
        let receipt_b = sample_receipt("test-session-writeloop", SessionOutcome::Refused);
        writer.write_receipt(&receipt_a).expect("write 1");
        writer.write_receipt(&receipt_b).expect("write 2");

        let contents = std::fs::read_to_string(&writer.file_path).expect("read sink file");
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

    /// Phase 118 review CR-02: a SECOND, independently-constructed
    /// [`ReceiptWriter`] over an EXISTING segment must resume the chain
    /// (`sequence: 1`, `prev_head` = the first writer's `chain_head`) rather
    /// than restarting at genesis — the exact bug this fix closes. This is
    /// the perturbation the pre-fix test
    /// (`write_receipt_chains_two_records_and_recomputes_from_disk`) could
    /// not catch, since it reuses ONE writer for both writes.
    ///
    /// `receipt_commands.rs::verify_records` is the read side's own
    /// recompute-and-compare check, but it is a module-private function in a
    /// different module (`receipt_commands.rs`, not `pub(crate)`) — not
    /// reachable from this module without either widening its visibility or
    /// duplicating its logic, both of which are out of this fix's scope. This
    /// test instead mirrors `write_receipt_chains_two_records_and_recomputes_from_disk`'s
    /// own recompute-and-compare pattern, which exercises the identical
    /// chain-linkage invariant `verify_records` checks.
    #[test]
    fn second_writer_over_existing_segment_resumes_the_chain() {
        let sink_dir = tempdir().expect("tempdir");
        let session_id = "test-session-tworiters";

        // Writer 1: writes the segment's first (genesis) record, then is
        // dropped — simulating one process's `ReceiptWriter::new` call.
        let writer_1 =
            ReceiptWriter::new(session_id.to_string(), sink_dir.path()).expect("writer 1");
        let receipt_a = sample_receipt(session_id, SessionOutcome::Ran);
        writer_1.write_receipt(&receipt_a).expect("write 1");
        drop(writer_1);

        // Writer 2: a SEPARATE `ReceiptWriter::new` call over the SAME
        // existing segment — e.g. a corrective `Refused` record from a later
        // call site, or (per CR-03's now-fixed correlation) a broker/daemon
        // writer landing on the same session_id. Pre-fix, this would
        // initialise `sequence: 0` again; post-fix, it must resume.
        let writer_2 =
            ReceiptWriter::new(session_id.to_string(), sink_dir.path()).expect("writer 2");
        let receipt_b = sample_receipt(session_id, SessionOutcome::Refused);
        writer_2.write_receipt(&receipt_b).expect("write 2");

        let contents = std::fs::read_to_string(&writer_2.file_path).expect("read sink file");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "expected exactly 2 JSONL records");

        let record_a: ReceiptRecord = serde_json::from_str(lines[0]).expect("parse record 1");
        let record_b: ReceiptRecord = serde_json::from_str(lines[1]).expect("parse record 2");

        assert_eq!(record_a.sequence, 0);
        assert!(record_a.prev_head.is_none());
        // The crux of this fix: writer 2 must NOT re-emit `sequence: 0,
        // prev_head: None` — it must resume from writer 1's tail.
        assert_eq!(
            record_b.sequence, 1,
            "a second writer over an existing segment must resume sequence, not restart at 0"
        );
        assert_eq!(
            record_b.prev_head,
            Some(record_a.chain_head),
            "a second writer's prev_head must chain from the first writer's chain_head"
        );

        // Recompute-and-compare over the COMBINED file, exactly as `nono
        // receipt verify` does — this is what `verify_records` would accept
        // and what the pre-fix bug made it reject as "reordered or deleted".
        let recomputed_head_a = hash_receipt_chain(None, &record_a.leaf_hash);
        assert_eq!(recomputed_head_a, record_a.chain_head);
        let recomputed_head_b = hash_receipt_chain(Some(&record_a.chain_head), &record_b.leaf_hash);
        assert_eq!(recomputed_head_b, record_b.chain_head);
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

        let contents = std::fs::read_to_string(&writer.file_path).expect("read sink file");
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

    /// Phase 118 review CR-02: fail CLOSED when an existing segment's tail
    /// cannot be parsed, rather than silently restarting at genesis — the
    /// review's own explicit requirement ("silently starting a new genesis on
    /// a corrupt tail would re-introduce the same ambiguity").
    #[test]
    fn new_writer_rejects_a_segment_with_an_unparseable_tail_record() {
        let sink_dir = tempdir().expect("tempdir");
        let file_path = sink_dir.path().join("test-session-corrupt.jsonl");
        std::fs::write(&file_path, b"{ this is not valid JSON at all\n")
            .expect("seed corrupt file");

        let err = ReceiptWriter::new("test-session-corrupt".to_string(), sink_dir.path())
            .expect_err("a segment with an unparseable tail must be rejected, not restarted");
        assert!(matches!(err, NonoError::Snapshot(_)));
    }
}
