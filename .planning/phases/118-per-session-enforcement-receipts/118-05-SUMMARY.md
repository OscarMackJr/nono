---
phase: 118-per-session-enforcement-receipts
plan: 05
subsystem: windows-receipt-sink
tags: [windows, receipts, acl, mandatory-label, keyless-chain, dacl, security]

# Dependency graph
requires:
  - phase: 118-per-session-enforcement-receipts (Plan 01)
    provides: "EnforcementReceipt, LayerId, hash_receipt_event/hash_receipt_chain, RECEIPT_*_DOMAIN"
  - phase: 118-per-session-enforcement-receipts (Plan 02)
    provides: "deny_sid_on_path (D-08 DACL-deny primitive), require_receipts machine policy"
provides:
  - "crates/nono-cli/src/receipt_sink.rs: resolve_sink_dir, ensure_sink_guarded, ReceiptWriter"
  - "receipt_sink module reachable from both nono.exe (main.rs) and nono-agentd.exe (#[path] include)"
  - "[Rule 1 fix] nono::deny_sid_on_path flat re-export (was missing from 118-02)"
  - "[Rule 1 fix] corrected deny_sid_on_path doc: mandatory label does NOT cover a Medium-IL broker child"
affects: ["118-06 (broker sink)", "118-07 (nono.exe wiring)", "118-08 (nono-agentd.exe wiring)", "118-09 (nono receipt verify/show/list)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-writer JSONL sink mirroring AuditRecorder's OpenOptions::append shape, with a ReceiptRecord envelope (sequence/prev_head/leaf_hash/chain_head) mirroring AuditEventRecord"
    - "Mutex-guarded, keyless chain-advance-and-write held under ONE lock (WR-21 discipline mirrored from telemetry::advance_and_emit, without the key field, D-25)"
    - "D-08 both-not-either guard: DENY ACE (per-SID) + unconditional NO_READ_UP|NO_EXECUTE_UP mandatory label"

key-files:
  created:
    - crates/nono-cli/src/receipt_sink.rs
  modified:
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/bin/nono-agentd.rs
    - crates/nono/src/lib.rs
    - crates/nono/src/sandbox/windows.rs

key-decisions:
  - "Sink location: %PROGRAMDATA%\\nono\\receipts, chosen over %LOCALAPPDATA% because D-09's governance consumer is the operator/fleet admin (machine-wide retrieval), and live-verified labelable on this host via icacls /setintegritylevel probing the identical WRITE_OWNER requirement."
  - "Deny mask is comprehensive (FILE_GENERIC_READ|WRITE|EXECUTE|DELETE), not read-only, since no confined child has any legitimate reason to touch the shared sink directory at all."
  - "Per-session file naming (<session_id>.jsonl) is sufficient for this plan's scope (nono.exe + nono-agentd.exe only, which never share a session_id) — the broker's own sink (118-06) and any future cross-process concurrent-append safety question is explicitly named as out of this plan's scope, not silently assumed solved."
  - "[Rule 1 - bug] Corrected a factually wrong doc claim in crates/nono/src/sandbox/windows.rs::deny_sid_on_path: it previously claimed the mandatory label's NO_READ_UP closes the Medium-IL-broker-child read gap. A live probe disproved this — see Deviations."

requirements-completed: [RCPT-01, RCPT-02]

# Metrics
duration: ~2h30m
completed: 2026-08-16
---

# Phase 118 Plan 05: Operator-ACL'd Receipt Sink Summary

Built the dedicated, D-08-guarded (DENY ACE + unconditional `NO_READ_UP` mandatory label) receipt
sink `crates/nono-cli/src/receipt_sink.rs`, with a mutex-guarded, KEYLESS (D-25) per-writer chain
writer mirroring `AuditRecorder`'s append-only JSONL shape, wired reachable from both `nono.exe`
(`main.rs`) and `nono-agentd.exe` (`#[path]` include) — and, during design verification, empirically
disproved and corrected an existing (118-02) doc claim about what the mandatory label actually
protects against.

## Performance

- **Duration:** ~2h30m
- **Tasks:** 2 completed
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- `resolve_sink_dir()` — `%PROGRAMDATA%\nono\receipts`, with the sink-location choice justified by
  a **live labelability check on this development host** (not an assumption): created a real
  subdirectory under `C:\ProgramData` as the current non-elevated user and confirmed
  `icacls <dir> /setintegritylevel Low` succeeds (`Successfully processed 1 files`) — proving the
  creating user gets implicit `WRITE_OWNER`, which `try_set_mandatory_label`
  (`SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)`) needs.
- `ensure_sink_guarded(dir, session_sid, package_sid)` — applies D-08's both-not-either guard:
  `deny_sid_on_path` for each `Some` SID (comprehensive deny mask — read/write/execute/delete) and
  an UNCONDITIONAL `NO_READ_UP | NO_EXECUTE_UP` mandatory label (applied even when both SIDs are
  `None`, since the directory is shared across every launch). Fails closed on any I/O/ACL/label
  error — never silently proceeds unguarded (T-118-15).
- `ReceiptWriter` — `new(session_id, sink_dir)` validates `session_id` is filename-safe and
  creates/opens the per-session `<session_id>.jsonl` file; `write_receipt(&receipt)` locks a
  `Mutex<ReceiptChainState>` (NO `key` field — D-25) ONCE and holds it across
  serialize→`hash_receipt_event`→`hash_receipt_chain`→append-to-file, mirroring
  `telemetry::advance_and_emit`'s WR-21 discipline. Chain state only advances after the file write
  durably succeeds, so a failed write never desyncs in-memory state from disk.
- Wired `mod receipt_sink;` into `main.rs`'s module list and a fourth
  `#[path = "../receipt_sink.rs"]` include into `nono-agentd.rs`, matching the existing
  `agent_daemon`/`telemetry`/`telemetry_init` three-line pattern exactly. Both `nono.exe` and
  `nono-agentd.exe` compile with the module reachable (`cargo check --bin nono`/`--bin nono-agentd`
  both exit 0).

## Task Commits

1. **Task 1: Sink path resolution + DENY-ACE/NO_READ_UP guard + mutex-guarded keyless chain writer**
   — `717cf0b7` (feat)
2. **Task 2: Wire receipt_sink into both binaries' module trees** — `dcdc18ca` (feat)

**Plan metadata:** (this commit, pending)

## Files Created/Modified

- `crates/nono-cli/src/receipt_sink.rs` — new module: `resolve_sink_dir`, `ensure_sink_guarded`,
  `ReceiptChainState`, `ReceiptRecord`, `ReceiptWriter` (`new`/`session_id`/`file_path`/
  `write_receipt`), 9 tests.
- `crates/nono-cli/src/main.rs` — `mod receipt_sink;` (Windows-gated), inserted alphabetically
  between `query_ext` and `registry_client`.
- `crates/nono-cli/src/bin/nono-agentd.rs` — fourth `#[path = "../receipt_sink.rs"] mod
  receipt_sink;` include.
- `crates/nono/src/lib.rs` — added the missing `deny_sid_on_path` flat re-export (Rule 1 fix).
- `crates/nono/src/sandbox/windows.rs` — corrected `deny_sid_on_path`'s doc comment (Rule 1 fix,
  see Deviations).

## Decisions Made

- **Sink location (Claude's discretion, D-06):** `%PROGRAMDATA%\nono\receipts` over
  `%LOCALAPPDATA%`, because D-09's governance consumer is the operator/fleet admin (machine-wide
  retrieval matches `provision_windows.rs`'s existing `%PROGRAMDATA%\nono\nono-poc-root.pem`
  precedent), and because labelability — flagged as host-dependent/undecided in
  `118-VALIDATION.md`'s Manual-Only table — was verified live on this host rather than assumed.
- **Deny mask is comprehensive**, not read-only: `FILE_GENERIC_READ | FILE_GENERIC_WRITE |
  FILE_EXECUTE | DELETE`. No confined child has a legitimate reason to touch the shared sink
  directory in any capacity.
- **File naming is `<session_id>.jsonl`**, sufficient for this plan's own scope (`nono.exe` +
  `nono-agentd.exe`, which per D-15 never share a session_id — the daemon is its own supervisor for
  its own sessions). The broker's two-receipts-per-session story (D-15) is explicitly Plan 118-06's
  own module and sink code, not this file's. A hypothetical future cross-process concurrent-append
  race (two different processes writing the SAME session_id file) is named in the module doc as a
  real, accepted limitation of this plan's scope rather than silently assumed non-existent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `deny_sid_on_path` was missing from `crates/nono`'s flat re-export namespace**

- **Found during:** Task 1, while writing `receipt_sink.rs`'s `use nono::{deny_sid_on_path, ...}`
  import.
- **Issue:** Plan 118-02 added `pub fn deny_sid_on_path` to `crates/nono/src/sandbox/windows.rs`
  but did not add it to `crates/nono/src/lib.rs`'s `#[cfg(target_os = "windows")] pub use
  sandbox::windows::{...}` flat re-export block — every other public windows.rs primitive
  (`grant_sid_*_on_path`, `revoke_sid_on_path`, `try_set_mandatory_label`, etc.) is re-exported
  there, but `deny_sid_on_path` was reachable only via the fully-qualified
  `nono::sandbox::windows::deny_sid_on_path` path, breaking the established flat-namespace
  convention this crate follows everywhere else.
- **Fix:** Added `deny_sid_on_path` to the flat re-export list (alphabetically), no other change.
- **Files modified:** `crates/nono/src/lib.rs`
- **Verification:** `cargo check -p nono-sandbox` and `cargo check -p nono-sandbox-cli --bin nono`
  both exit 0 with `use nono::deny_sid_on_path` (not the fully-qualified path) in
  `receipt_sink.rs`.
- **Committed in:** `717cf0b7` (Task 1 commit)

**2. [Rule 1 - Bug] `deny_sid_on_path`'s doc comment made an empirically false claim about
`NO_READ_UP` mandatory-label coverage**

- **Found during:** Task 1, while designing `ensure_sink_guarded`'s guard-mask choice — the
  interfaces section instructed mirroring `label_mask_for_access_mode(AccessMode::Write)`'s mask,
  and the existing `deny_sid_on_path` doc comment (118-02) claimed: "A Medium-IL broker child
  (`nono-shell-broker.exe`, Phase 31) is not covered by ANY per-session SID... The mandatory
  label's `NO_READ_UP` is what closes it instead." This directly contradicted `118-CONTEXT.md`'s
  own D-08 text ("the mandatory label misses a Medium-IL broker child, and the DENY ACE misses an
  arm where no session SID was minted") — the two documents asserted opposite claims about which
  mechanism covers the broker arm.
- **Issue:** `try_set_mandatory_label` hardcodes the object's own mandatory-label RID to
  `SECURITY_MANDATORY_LOW_RID` (the SDDL `LW` alias) — it can never label an object Medium or
  higher. Whether `NO_READ_UP` on a Low-RID object blocks a Medium-IL reader is a genuine, testable
  OS-behavior question this codebase's existing tests never actually exercised (they only assert
  the ACE's mask value, never a real cross-integrity-level access attempt).
- **Investigation (live probe, then reverted before committing — never part of any commit):** added
  a temporary `#[test]` to `crates/nono/src/sandbox/windows.rs` that: (1) labeled a real tempdir
  with `NO_READ_UP | NO_EXECUTE_UP` via `try_set_mandatory_label`; (2) spawned a REAL child process
  via `CreateProcessAsUserW` using a genuine Low-IL primary token from
  `nono::create_low_integrity_primary_token()`, attempting `cmd /c type <file>` against a file in
  that directory; (3) compared against the SAME test process (Medium IL) reading the identical file
  directly. Result: the Low-IL child's read attempt failed (exit code 1, "Access is denied",
  redirect-captured output empty); the Medium-IL test process read the file successfully
  (`Ok("secret-content")`). This empirically confirms: `NO_READ_UP` on a Low-RID-labeled object
  blocks Low-IL-or-below readers only — it structurally cannot block a Medium-IL reader, since a
  Medium-IL subject is never "below" a Low-RID object. Reverted the probe test via `git checkout --
  crates/nono/src/sandbox/windows.rs` (destructive_git_prohibition's sanctioned single-file
  discard) before making any real change — the probe itself was never committed.
- **Fix:** Corrected `deny_sid_on_path`'s doc comment (`crates/nono/src/sandbox/windows.rs`) to
  state the mandatory label does NOT close the Medium-IL-broker-child read gap, cite the live
  probe's method and result, and name closing that specific gap as an accepted, out-of-scope
  residual boundary (the broker is a semi-trusted, Authenticode-gated co-supervisor per D-09's
  same-trust-domain framing, not adversarial input this plan is scoped to defend against) — rather
  than silently leaving the incorrect claim in place for a future reader to trust.
  `receipt_sink.rs`'s own module doc cites this corrected account, not the original claim.
- **Files modified:** `crates/nono/src/sandbox/windows.rs` (doc-only; no behavior change — the
  actual mask/RID logic in `try_set_mandatory_label`/`deny_sid_on_path` was already correct, only
  the doc's claim about what it achieves was wrong).
- **Verification:** `cargo check -p nono-sandbox` and `cargo test -p nono-sandbox --lib` both clean
  (862 passed, 0 failed, unchanged from Plan 118-02's recorded baseline — doc-only change, no test
  behavior affected).
- **Committed in:** `717cf0b7` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bug/doc-correctness fixes discovered while
building this plan's own security-critical guard).
**Impact on plan:** Both fixes were necessary before this plan's own guard design could be trusted
— receipt_sink.rs's module doc makes an accurate, empirically-verified claim about what D-08's
mandatory-label half actually protects against, rather than inheriting an unverified, contradictory
claim from the prior plan. No scope creep — both fixes are surgical (one missing re-export line,
one doc-comment correction) with no behavior change to any function's actual logic.

## Issues Encountered

**Task 2's expected `dead_code` warnings materialize under `-D warnings` as documented in the
plan.** `cargo clippy -p nono-sandbox-cli --bin nono --bin nono-agentd -- -D warnings -D
clippy::unwrap_used` hard-fails on 10 `dead_code` diagnostics (the sink module's public functions
and types are not yet called by either binary — that's Plans 118-07/118-08's job). This is EXACTLY
what Task 2's `<done>` criterion anticipated and explicitly forbade suppressing via
`#[allow(dead_code)]`. Verified `cargo clippy -p nono-sandbox-cli --tests -- -D warnings -D
clippy::unwrap_used` (which additionally compiles the `#[cfg(test)]` module, exercising every
symbol) is fully clean — confirming the warnings are the anticipated "not yet called by production
code" state, not a real defect. One residual: `ReceiptWriter::session_id()` was initially unused
even under `--tests` (no test called it) — per CLAUDE.md's "write tests that use it" guidance (not
`#[allow(dead_code)]`), added one assertion (`writer.session_id()`) to an existing test rather than
leaving the accessor untested.

## Verification Evidence

```
$ cargo test -p nono-sandbox-cli --bin nono receipt_sink -- --nocapture
running 7 tests
test receipt_sink::tests::resolve_sink_dir_is_under_programdata_nono_receipts ... ok
test receipt_sink::tests::unguarded_dir_carries_neither_guard ... ok
test receipt_sink::tests::write_receipt_rejects_unsafe_session_id_for_filename ... ok
test receipt_sink::tests::ensure_sink_guarded_applies_both_deny_ace_and_no_read_up_label ... ok
test receipt_sink::tests::ensure_sink_guarded_applies_label_even_with_no_sids ... ok
test receipt_sink::tests::tampered_leaf_hash_fails_recompute_and_compare ... ok
test receipt_sink::tests::write_receipt_chains_two_records_and_recomputes_from_disk ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1709 filtered out

$ cargo check --bin nono -p nono-sandbox-cli && cargo check --bin nono-agentd -p nono-sandbox-cli
(both exit 0; 10 expected temporary dead_code warnings each, per Task 2's <done> criteria)

$ cargo clippy -p nono-sandbox-cli --tests -- -D warnings -D clippy::unwrap_used
Finished (clean)

$ cargo clippy -p nono-sandbox -- -D warnings -D clippy::unwrap_used
Finished (clean)

$ cargo fmt --check -p nono-sandbox-cli -p nono-sandbox
(clean, after `cargo fmt` auto-applied 6 formatting fixes)

$ cargo build --workspace --all-targets
Finished (clean build, only the anticipated dead_code warnings)

$ cargo test -p nono-sandbox-cli --bin nono --no-fail-fast
test result: FAILED. 1702 passed; 12 failed; 2 ignored; 0 measured; 0 filtered out
  (12 failures are EXACTLY the documented known-good baseline set: 6x config::tests::*,
  3x protected_paths::tests::*, profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name,
  audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty,
  exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap
  — no new regression)

$ cargo test -p nono-sandbox --lib
test result: ok. 862 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (unchanged from Plan 118-02's recorded baseline — doc-only core change)

$ grep -c "key:" crates/nono-cli/src/receipt_sink.rs
0

$ grep -c "receipt_sink" crates/nono-cli/src/bin/nono-agentd.rs
3

$ grep -c "mod receipt_sink" crates/nono-cli/src/main.rs
1
```

## Cross-Target Clippy Gate Scope (Plan 118-10)

`crates/nono-cli/src/receipt_sink.rs` is entirely Windows-only content (gated at the `mod`
declaration site in both `main.rs` and `nono-agentd.rs`, matching the `windows_wfp_contract`/
`provision_windows` precedent — the file itself carries no internal `#[cfg]`, since it is simply
never compiled on non-Windows targets). `crates/nono/src/sandbox/windows.rs` (doc-only edit) is
already `#[cfg(target_os = "windows")]`-scoped per its own module gating and was already flagged
in-scope for Plan 118-10's cross-target gates by Plan 118-02's SUMMARY. `crates/nono/src/lib.rs`'s
one-line re-export addition sits inside an existing `#[cfg(target_os = "windows")]` block. No new
Unix `cfg` branches were introduced by this plan; Plan 118-10 should still run both mandatory local
gates (`cross clippy --target x86_64-unknown-linux-gnu`, `cargo-zigbuild clippy --target
x86_64-apple-darwin`) per the MUST/NEVER rule — this plan did not run them itself (Windows-host
local pre-checks only, per the phase's stated aggregation-at-118-10 pattern).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `receipt_sink.rs` is complete, tested, and reachable from both `nono.exe` and `nono-agentd.exe`'s
  real module trees — ready for Plans 118-07/118-08 to add the actual call sites at the D-21
  attestation gate (`apply_startup_attestation_gate`/daemon launch path).
- `nono::deny_sid_on_path`'s doc now accurately documents its coverage boundary — Plans 118-06
  (broker sink) and any future work touching the Medium-IL-broker read-protection question should
  read that corrected doc rather than re-deriving the same investigation.
- No blockers. The named residual scope boundary (Medium-IL broker child read protection; the
  cross-process concurrent-append-to-the-same-file limitation) are both documented in
  `receipt_sink.rs`'s and `deny_sid_on_path`'s doc comments for the plans that will need to reason
  about them (118-06/118-07/118-08).

---
*Phase: 118-per-session-enforcement-receipts*
*Completed: 2026-08-16*
