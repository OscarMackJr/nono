---
slug: lowil-cwd-write-denied
status: fix_applied_pending_live_verify
trigger: "Phase 60 UAT F-60-UAT-04 — the Low-IL confined child cannot write to its granted CWD on Windows 11; the sandbox-the-tools confined-write mechanism (Phase 60 SC 1) fails at the OS level."
created: 2026-06-01
updated: 2026-06-01
---

# Debug: Low-IL confined child cannot write to its granted CWD (Windows)

## Symptoms

**Expected:** A `nono run --profile claude-code-tools-windows-runner --allow-cwd -- <cmd>` Low-IL
child should be able to WRITE files inside its granted CWD (`C:\Users\OMack\nono-poc`), per the
`r+w` capability shown in nono's own banner. This is the core of Phase 60 SC 1 ("file edits work,
confined").

**Actual:** Every confined write to the granted CWD is denied at the OS level, regardless of shell:
- **A) `cmd.exe /c "echo hi> test_cmd.txt"`** (no .NET): jail applies, banner shows
  `r+w \\?\C:\Users\OMack\nono-poc`, then the write → **`Access is denied.`**
- **B) `pwsh.exe ...`**: never launches — `Windows filesystem policy does not cover the executable
  path required for launch: C:\Program Files\WindowsApps\Microsoft.PowerShell_7.6.1.0_x64__8wekyb3d8bbwe\pwsh.exe`
  (PS7 is a Store/MSIX app under WindowsApps; secondary issue).
- **C) `powershell.exe -NoProfile -NonInteractive -Command "Set-Content test_ps5.txt hi"`**:
  `Starting the CLR failed with HRESULT 80070005` (E_ACCESSDENIED class — .NET Framework CLR can't
  start at Low IL, presumably because it needs to write to Medium-IL `%TEMP%`/fusion).

**Error messages / key log evidence:** every `nono run` emits repeated:
```
WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may
have no observable enforcement effect depending on pre-existing label)
   path=C:\Users\OMack\.cargo  prior_rid="0x1000"  prior_mask="0x5"
   ... (also .config\git\ignore, .gitconfig, .local\bin, .rustup)
WARN label guard: path not owned by current user; skipping mandatory label apply (system paths are
   Medium-IL by default and already readable by Low-IL subjects)  path=C:\Windows  access=Read
Access is denied.
```
NOTE: NO `label guard` line is emitted for `nono-poc` itself — so the CWD relabel apparently was
attempted/not-skipped, yet the Low-IL child still cannot write to it.

**Timeline:** Never worked in live testing. Phase 60 shipped `human_needed` (code statically
verified, never run on a real host). This is the first live Win11 UAT — discovered 2026-06-01.

**Reproduction (operator runs from a real PowerShell console; NOT the Claude Code MSYS Bash tool,
which has no real Win32 console and cannot launch Low-IL `nono run`):**
```powershell
Set-Location "$env:USERPROFILE\nono-poc"
$N = "C:\Users\OMack\Nono\target\x86_64-pc-windows-msvc\release\nono.exe"   # 0.57.5, has Phase 60 code
& $N run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi> test_cmd.txt"
```
Dev-layout binary (skips the unsigned-broker trust gate). Runner profile installed at
`%APPDATA%\nono\profiles\claude-code-tools-windows-runner.json` (UAT variant with
`network.block:false`; original saved as `...json.netblock.bak`).

## Leading Hypothesis (CONFIRMED via static analysis — see Resolution)

The runner-profile CWD grant does NOT produce an *effective* writable path for the Low-IL child.
ROOT CAUSE turned out to be the WRITE_RESTRICTED restricting-SID DACL gap, not the mandatory label
(the label is applied correctly). A (CWD write denied) and C (CLR `%TEMP%` write denied) are the
same root failure surfaced two ways: the restricting SID is absent from the DACL of every path the
child tries to write.

## Relevant Code

- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` —
  `create_restricted_token_with_sid` builds the Low-IL child token with `CreateRestrictedToken(...,
  WRITE_RESTRICTED, ..., 1, &sid_restrict, ...)`. The restricting SID is the synthetic per-session
  `S-1-5-117-*` from `generate_session_sid()`. THIS is the missing-DACL-grant half of the bug.
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` — `AppliedLabelsGuard::
  snapshot_and_apply(&fs_policy)` applies ONLY the mandatory-label SACL (`try_set_mandatory_label`).
  It takes no session SID and never touches the DACL. This is the SOLE per-path ACL apply site on
  the CLI path (mod.rs:312). The fix must extend this (or add a sibling guard) to add a DACL
  allow-ACE for the restricting SID on every writable rule.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs:312` — call site; `fs_policy` rules carry
  `access` (Read/Write/ReadWrite). `config.session_sid` is the SID string used for the token.
- `crates/nono/src/sandbox/windows.rs:647` `label_mask_for_access_mode` — CORRECT
  (`ReadWrite`→`NO_EXECUTE_UP` only; does NOT set NO_WRITE_UP). Label is not the problem.
- `crates/nono-cli/src/claude_code_hook.rs`, `packages/claude-code/
  claude-code-tools-windows-runner.profile.json` — hook + runner profile (unchanged by this dx).

## What is already KNOWN GOOD (do not re-investigate)

- The Claude Code PreToolUse hook engages; deny+additionalContext → Bash retry (A1) is reliable.
- Capability scoping is correct (banner shows exactly `r+w nono-poc`, nothing broader).
- Self-disable guard (F-60-UAT-01) and CLAUDE_CONFIG_DIR isolation work.
- The `network.block`/WFP layer (F-60-UAT-03) is orthogonal and disabled for this repro.
- The mandatory-label apply on `nono-poc` SUCCEEDS (no `label guard` WARN). The label is NOT the
  failing axis.

## Current Focus

- status: ROOT CAUSE CONFIRMED (static analysis + live env-instrument A/B on 2026-06-01). Ready to
  implement the DACL-grant fix; operator verifies the full hook→confined-write path after.
- root cause: the Low-IL child token is built with `WRITE_RESTRICTED` + a synthetic restricting SID
  (`S-1-5-117-*`), but nono NEVER adds that restricting SID to the DACL of granted writable paths.
  Under `WRITE_RESTRICTED`, every WRITE access check runs TWICE — once vs the user's normal SIDs
  (passes; user owns `nono-poc`) and once vs the restricting-SID list (contains ONLY the synthetic
  session SID, which appears in NO ACE on `nono-poc`'s DACL). The second pass denies → `Access is
  denied`. The mandatory-label lowering (correct: `ReadWrite`→`NO_EXECUTE_UP`) is necessary but NOT
  sufficient; the missing piece is a DACL grant for the restricting SID on each writable path.
- why no `label guard` WARN for nono-poc: the label apply (SACL) SUCCEEDED. The failure is on the
  DACL/restricting-SID axis, an entirely separate ACL mechanism never wired to the writable grants.
- next_action: operator runs the confirmation experiment below; then apply the DACL-grant fix.

## Operator Confirmation Experiment (run from a real PowerShell console)

Goal: prove the restricting-SID DACL gap, not the mandatory label, is what denies the write.

APPROACH (chosen 2026-06-01): one-off env-gated instrument build. `generate_session_sid()` mints a
fresh random SID per run, so a plain pre-grant can't match it. Added a temporary, env-gated override
(`NONO_DEBUG_FIXED_SID`) to `restricted_token.rs::generate_session_sid` (no effect when unset; REVERT
before commit) and rebuilt `target/x86_64-pc-windows-msvc/release/nono.exe`. This pins the restricting
SID so it can be pre-granted on the CWD. Two tests isolate exactly one variable (the restricting SID):

```powershell
Set-Location "$env:USERPROFILE\nono-poc"
$N = "C:\Users\OMack\Nono\target\x86_64-pc-windows-msvc\release\nono.exe"

# TEST 1 (no icacls): restricting SID = the user's OWN SID, already on nono-poc's DACL with write.
$mySid = ([System.Security.Principal.WindowsIdentity]::GetCurrent()).User.Value
$env:NONO_DEBUG_FIXED_SID = $mySid
& $N run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi> test_t1.txt"
Get-Content .\test_t1.txt          # expect: hi  (was 'Access is denied' with the synthetic SID)

# TEST 2 (faithful fix proxy): synthetic SID, granted on the CWD DACL == exactly what the fix does.
$env:NONO_DEBUG_FIXED_SID = "S-1-5-117-1-2-3-4"
& $N run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo no> test_t2_before.txt"  # expect: still denied
icacls . /grant "*S-1-5-117-1-2-3-4:(OI)(CI)M"
& $N run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi> test_t2_after.txt"
Get-Content .\test_t2_after.txt    # expect: hi

# CLEANUP
icacls . /remove:g "*S-1-5-117-1-2-3-4"; Remove-Item Env:NONO_DEBUG_FIXED_SID
```

Interpretation:
- TEST 1 write LANDS → the gate is "restricting SID present+writable on the target DACL" (confirmed).
- TEST 2 `before` denied + `after` LANDS → confirms the precise fix: add the synthetic session SID
  to each granted writable path's DACL, inheritable (OI)(CI). Strongest proof.

REVERT NOTE: after the experiment, `git checkout -- crates/nono-cli/src/exec_strategy_windows/restricted_token.rs`
to remove the instrument; the real fix is implemented separately (below).

If the operator prefers, skip straight to the fix (below) — the mechanism is textbook
WRITE_RESTRICTED double-access-check semantics and the code gap is unambiguous.

## Fix Applied (2026-06-01) — pending operator live-verify

Implemented the DACL-grant fix. Commits (DCO-signed, on `main`):
- `b5324b3c` feat(windows): add session-SID DACL grant/revoke primitives
  (`crates/nono/src/sandbox/windows.rs` + `lib.rs` export + `NonoError::DaclApplyFailed` in
  `error.rs` + FFI arm in `bindings/c/src/lib.rs`):
  - `pub fn grant_sid_write_on_path(path, sid, inheritable)` — merges an allow-ACE granting `sid`
    `FILE_GENERIC_WRITE | DELETE` (NOT FullControl) onto the existing DACL via GetNamedSecurityInfoW
    → SetEntriesInAclW → SetNamedSecurityInfoW (unprotected → inherited ACEs preserved); `(OI)(CI)`
    when inheritable.
  - `pub fn revoke_sid_on_path(path, sid)` — REVOKE_ACCESS for the trustee (safe: the synthetic SID
    is unique to nono, pre-exists in no ACE).
- `aff506eb` feat(windows): wire session-SID DACL guard into supervised launch
  (`crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` new + `mod.rs` wiring):
  - `AppliedDaclGrantsGuard::snapshot_and_apply(policy, session_sid)` — writable rules only
    (read-only → Skip), ownership-gated, fail-closed, LIFO best-effort revoke on Drop. Held in
    `PreparedWindowsLaunch._applied_dacls` (built only when `session_sid` is Some). Operative on the
    WriteRestricted arm; inert (but reverted) on other arms.

Verification done on this host (Windows dev box):
- `cargo build --release --target x86_64-pc-windows-msvc -p nono-cli` → clean.
- Unit tests 5/5 PASS (verified directly): nono lib `dacl_grant_tests::{grant_then_revoke_sid_round_trips_on_tempdir,
  grant_single_file_and_invalid_sid_fails_closed}`; nono-cli bin `exec_strategy::dacl_guard::tests::
  {writable_rule_applies_sid_ace_and_reverts_on_drop, read_only_rule_is_skipped_no_dacl_change,
  mid_loop_grant_failure_reverts_already_applied}`.
- `cargo clippy -p nono --lib` and `-p nono-cli --bin nono` with `-D warnings -D clippy::unwrap_used`
  → clean (EXIT 0).
- DEFERRED (cross-target rule): Linux/macOS clippy can't run on this Windows host; all new code is
  `#[cfg(target_os = "windows")]` → defer to CI.
- NOTE: the implementing agent ran `cargo fmt --all`, reformatting 24 unrelated pre-existing-fmt-drift
  files; that noise was reverted (`git checkout -- crates/ bindings/`) and the binary rebuilt from the
  clean committed source. Only the 6 fix files are committed.

PENDING (operator, real PowerShell console — cannot run on the orchestrator MSYS host): live-verify
the confined write now LANDS with the DEFAULT synthetic SID (no env override, no icacls):
```powershell
Set-Location "$env:USERPROFILE\nono-poc"
$N = "C:\Users\OMack\Nono\target\x86_64-pc-windows-msvc\release\nono.exe"
& $N run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo dacl-fix> test_fix.txt"
Get-Content .\test_fix.txt    # expect: dacl-fix   (previously 'Access is denied')
```
Then re-run the Phase 60 UAT 1 end-to-end through the Claude Code hook. On confirmation → status
`resolved`; then revisit secondary gaps B (pwsh under WindowsApps) and C (.NET CLR %TEMP%).

## Proposed Fix (security-critical Windows ACL code — respect cross-target clippy + fail-secure)

Add a DACL allow-ACE for the session restricting SID to every **writable** filesystem rule
(`AccessMode::Write` and `AccessMode::ReadWrite`), as a new RAII guard sibling to
`AppliedLabelsGuard`, applied at `mod.rs:312` with `config.session_sid` threaded in:

- For each rule with write access: `SetNamedSecurityInfoW(SE_FILE_OBJECT,
  DACL_SECURITY_INFORMATION)` adding an inheritable (OI|CI) allow-ACE granting the restricting SID
  the write-class rights (FILE_GENERIC_WRITE / FILE_ADD_FILE / FILE_ADD_SUBDIRECTORY / DELETE as
  the grant mode warrants). Read-only rules need NO DACL change (WRITE_RESTRICTED only double-checks
  WRITE access; reads already pass on the user SIDs).
- RAII Drop removes exactly the ACE it added (mirror `AppliedLabelsGuard` revert discipline; track
  Applied vs Skip; LIFO revert; best-effort, never panic). Fail-CLOSED on apply error.
- Ownership/WRITE_DAC pre-check: editing the DACL needs WRITE_DAC. Owner has implicit WRITE_DAC
  (memory: Owner grants WRITE_DAC+READ_CONTROL but NOT WRITE_OWNER), and writable grants are
  user-owned paths, so this should succeed where label-apply already succeeds. Still pre-check and
  fail-closed.
- New-file inheritance: granting (OI)(CI) on the directory makes new child files inherit the ACE,
  so files the child CREATES are writable too (addresses the `%TEMP%`/CLR case C if `%TEMP%` is in
  the grant set — verify whether the runner profile grants the CLR's required temp/fusion dirs;
  if not, that is a SEPARATE grant-set gap, not this ACL fix).
- Security review: the restricting SID is per-session and synthetic — granting it write on an
  already-user-owned, already-Low-IL path does NOT broaden the trust boundary (the child already
  has the user SID; the restricting pass is the ONLY thing currently over-denying). Confirm the ACE
  is removed on Drop so no residue persists across sessions. Cross-target clippy is N/A for revert
  of Windows-only cfg code but the workspace clippy/fmt MUST stay green for `--bin nono`.

Secondary, separate fixes (track but do not conflate with this root cause):
- B) `pwsh.exe` under `C:\Program Files\WindowsApps\...` (MSIX) not covered by the launch
  executable policy. Either route confined writes through `powershell.exe` (Windows PowerShell 5.1,
  not MSIX) or extend the launch-exe coverage. Note C shows 5.1's CLR ALSO can't start at Low IL
  without temp/fusion write — so the durable answer for confined edits is likely `cmd.exe`-class
  or a nono-provided writer, not a .NET shell.
- C) .NET CLR needs Medium-IL `%TEMP%`/fusion write. After the DACL fix, retest whether granting
  the CLR's temp dirs (with the same restricting-SID DACL grant) lets the CLR start. If the runner
  profile does not grant those dirs, that is a grant-set gap.

## Evidence

- timestamp: 2026-06-01 — Repro A/B/C captured live on Win11 build (operator PowerShell), 0.57.5
  msvc binary. A: `Access is denied`; B: exe-path-not-covered; C: CLR 80070005. label-guard WARNs
  present for group paths; none for nono-poc.
- timestamp: 2026-06-01 — STATIC ROOT CAUSE (orchestrator, code read). `restricted_token.rs:94-107`
  builds the child token via `CreateRestrictedToken(..., WRITE_RESTRICTED, 0, null, 0, null, 1,
  &sid_restrict, ...)` where `sid_restrict.Sid` is the synthetic `S-1-5-117-*` session SID. The
  file's OWN doc comment (lines 72-93) states WRITE_RESTRICTED makes every WRITE check run against
  BOTH the user SIDs AND the restricting SID. `labels_guard.rs` (sole per-path ACL apply site,
  mod.rs:312) applies ONLY the mandatory-label SACL and takes NO session SID — grep across
  `exec_strategy_windows/` for `AddAccessAllowedAce|SetEntriesInAcl|EXPLICIT_ACCESS|
  DACL_SECURITY_INFORMATION` returns ZERO matches. Therefore the restricting SID is in NO writable
  path's DACL → the WRITE_RESTRICTED second access-check pass denies every write to the granted CWD.
  `label_mask_for_access_mode` (windows.rs:647) maps ReadWrite→NO_EXECUTE_UP only (no NO_WRITE_UP),
  so the mandatory label is NOT the gate — consistent with the absence of a `label guard` WARN for
  nono-poc.

- timestamp: 2026-06-01 — ROOT CAUSE CONFIRMED via env-instrument A/B. Both runs took the
  `WriteRestricted` token arm (logged: `has_pty=false, prefers_low_il_broker=false`). PLAIN run:
  restricting SID = random synthetic `S-1-5-117-4056659201-...` (NOT on CWD DACL) → `Access is
  denied`. TEST 1: restricting SID overridden to the user's own SID `S-1-5-21-...-2727` (already on
  `nono-poc`'s DACL with full control) → `cmd.exe` write LANDED (`Get-Content test_t1.txt` = `hi`).
  Only the restricting SID changed between the two runs. This is decisive: under WRITE_RESTRICTED
  the write requires the restricting SID to grant write on the target DACL; nono's synthetic
  per-session SID is never added to granted filesystem paths' DACLs. The mandatory label is not the
  gate (WriteRestricted child stays Medium-IL; writing to a Low-labeled CWD is writing-down, allowed).

## Eliminated

- hypothesis: "confined write fails only because .NET CLR can't start at Low IL" — REFUTED by
  repro A: `cmd.exe` (no CLR) write to the granted CWD is ALSO `Access is denied`. The failure is
  the Low-IL write to the CWD itself, not .NET.
- hypothesis (the prior Leading Hypothesis): "CWD grant yields no effective Low-IL mandatory label /
  NO_WRITE_UP blocks the write" — REFUTED by static analysis: the mandatory label IS applied
  (no skip WARN for nono-poc) and the ReadWrite mask is `NO_EXECUTE_UP` only (never NO_WRITE_UP).
  The denial is on the DACL/restricting-SID axis, not the integrity-label axis.
