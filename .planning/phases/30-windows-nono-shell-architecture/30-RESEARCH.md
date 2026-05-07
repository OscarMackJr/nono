# Phase 30: Windows nono shell Interactive Enforcement Architecture - Research

**Researched:** 2026-05-07
**Domain:** Windows token shapes (Mandatory Integrity Control), ConPTY pseudoconsole, named-pipe DACL+SACL, supervisor IPC
**Confidence:** HIGH (token mechanics, MIC semantics, capability-pipe SDDL); MEDIUM (Wave 1 lurking failures); LOW (Wave 2 outcomes — by definition exploratory)

## Summary

Phase 30 lifts `create_low_integrity_primary_token()` from dead code to live code on the supervised+PTY (`nono shell`) path. The Wave 1 edit is surgical: a new sixth arm in the `spawn_windows_child` token cascade at `crates/nono-cli/src/exec_strategy_windows/launch.rs:1140-1160`, taken when `pty.is_some()`, that calls the existing `create_low_integrity_primary_token()` instead of `create_restricted_token_with_sid()`. The capability-pipe SDDL infrastructure is already correct for Low-IL primary token clients (mandatory-label SACL `(ML;;NW;;;LW)` admits Low-IL subjects; the per-session-SID and logon-SID DACL ACEs added in commit `938887f` are still present and harmless when the child token does not carry the per-session restricting SID — DACL evaluation falls back to the OW (Owner Rights) ACE, which matches because the supervisor-created pipe and the Low-IL child share the same user SID).

OS-level write-deny works end-to-end for Wave 1 by construction: `try_set_mandatory_label` puts NO_WRITE_UP labels on grant-set paths at Low IL; paths outside the grant set are unlabeled (Windows treats unlabeled as Medium-IL default); the Low-IL child token's mandatory policy strips GENERIC_WRITE on any access where TokenDominates=FALSE, which is true for Low → Medium. This is a kernel-level access check that runs BEFORE the DACL — `MS-DTYP MandatoryIntegrityCheck` is unambiguous on this point.

**Primary recommendation:** Wave 1 is a 1-arm cascade extension + 1 cookbook paragraph + 1 PROJECT.md correction. The lurking risks are (a) ConPTY's known integrity-mismatch failure mode (Microsoft Q&A documents this for SYSTEM-creating-pseudoconsole / lower-IL-child; nono's case is supervisor-creating-pseudoconsole / Low-IL-child where the supervisor is also Medium-IL — a different mismatch but the same class), and (b) `create_low_integrity_primary_token` was authored for a different call site (legacy Direct path) and has never been exercised under ConPTY before. Wave 1 must include a smoke-build-and-field-test as task #1, NOT defer it to a later acceptance gate.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Low-IL primary token construction | nono-cli (`launch.rs`) | nono lib | Already implemented; cli-private |
| Mandatory-label NO_WRITE_UP enforcement | OS kernel (MIC SeAccessCheck) | nono lib (`sandbox/windows.rs`) | Library applies labels; kernel enforces |
| ConPTY pseudoconsole allocation | nono-cli (`pty_proxy`) | OS kernel (`\Device\ConDrv`) | Already wired; unchanged in Wave 1 |
| Supervisor IPC capability pipe | nono lib (`socket_windows.rs`) | nono-cli (`exec_strategy_windows::supervisor`) | Already DACL-correct for Low-IL subjects |
| AppliedLabelsGuard RAII apply+revert | nono-cli (`exec_strategy_windows::labels_guard.rs`) | nono lib (`try_set_mandatory_label`) | Existing guard remains correct under new token |
| Kernel network identity | nono-wfp-service WFP filter | nono-cli supervisor | Falls back to AppID matching when session_sid threading is altered |
| Cookbook security-envelope documentation | docs (`windows-poc-handoff.mdx`) | — | Wave 1 final task |

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Wave 1 fix is **Low-IL primary token via `create_low_integrity_primary_token()`** — no WRITE_RESTRICTED, no session-SID. Rationale: WRITE_RESTRICTED+ConPTY = `STATUS_DLL_INIT_FAILED (0xC0000142)` (parallel to Phase 15's WRITE_RESTRICTED+DETACHED_PROCESS finding); Low-IL primary token avoids the brittleness while preserving mandatory-label write-deny because Low-IL subjects vs default Medium-IL files trigger NO_WRITE_UP.
- **D-02:** Null token (Option A — caller's identity Medium-IL) is **rejected**. Long-lived interactive shells warrant write protection at minimum.
- **D-03:** Anonymous-pipe stdio (Option 2) is **rejected** because losing TUI rendering is worse than the security cost.
- **D-04:** Wave 2 (ProcMon-driven Win32 investigation) is **conditional**, not unconditional. Spawn it only if Wave 1 field-test fails. Timebox: 3-5 working days.
- **D-05:** `nono shell` on Windows MUST host Claude Code's full TUI. Acceptance #2 fails the phase if TUI rendering is degraded.
- **D-06:** OS-level write-deny is REQUIRED, not optional. The Claude Code PreToolUse hook is **defense-in-depth**, not the primary boundary. Acceptance #3 fails the phase if writes succeed at OS level.
- **D-07:** v2.3 milestone delivery is NOT blocked on this phase. Time pressure not binding.
- **D-08:** Hook-firing investigation OUT OF SCOPE (separate debug `claude-code-hook-not-firing`).
- **D-09:** AppliedLabelsGuard label leak OUT OF SCOPE (separate debug `nono-labels-guard-leak`). 9 leaked Low-IL labels expected to surface as warnings during Wave 1 field test — not failure indicators.
- **D-10:** PROJECT.md SHELL-01 entry correction is in scope for Wave 1 task #1, regardless of technical outcome.

### Claude's Discretion

- Wave structure: Wave 1 = Option 3 field-test; Wave 2 = ProcMon (conditional). Plan-phase determines task breakdown.
- Whether Wave 1 implementation reuses the exact reverted Option D edit OR refines it. Helper-vs-inline gate, naming, comment shape — planner discretion.

### Deferred Ideas (OUT OF SCOPE)

- AppContainer-based isolation for `nono shell` — v3.0 candidate.
- AppContainer profile for the Claude Code child specifically — v3.0.
- Kernel mini-filter driver for FS deny enforcement — Phase 6b territory; long-deferred to v3.0.
- `nono shell --integrity <Untrusted|Low|Medium>` user-controlled IL — v2.4+ ergonomic improvement.
- `nono shell` on Linux/macOS — separate work if/when needed.
- `claude-code-hook-not-firing` debug session — separate.
- `nono-labels-guard-leak` debug session — separate.

## Phase Requirements

No formal REQ-IDs. D-01..D-10 from CONTEXT.md are the tracking unit. Acceptance criteria #1-#6 from CONTEXT.md drive validation; the decision-coverage gate enforces D-01..D-10 through plans.

## Phase Boundary Recap

Land OS-enforced filesystem write protection AND interactive TUI rendering for `nono shell --profile <name>` on Windows 10/11. Either ship a working path under ConPTY+Low-IL primary token, OR document evidence that no user-mode token shape can deliver both (deferred to v3.0 kernel mini-filter driver work).

## Wave 1 Implementation Details

### Token Cascade Edit Shape

Today's HEAD cascade in `spawn_windows_child` (launch.rs:1139-1160) is a 4-arm if/else if:

```
if is_windows_detached_launch:        null token        (Phase 15 detached)
else if config.session_sid.is_some(): WRITE_RESTRICTED  (current supervised path — TRIGGERS 0xC0000142 on PTY)
else if should_use_low_integrity_windows_launch(caps): Low-IL primary   (legacy Direct, dead code on supervised path)
else:                                  null fallback
```

Wave 1 inserts a NEW arm BEFORE the `config.session_sid.is_some()` arm:

```
if is_windows_detached_launch:        null token        (unchanged)
else if pty.is_some():                Low-IL primary    (NEW — Wave 1)
else if config.session_sid.is_some(): WRITE_RESTRICTED  (unchanged for non-PTY supervised, e.g. nono run)
else if should_use_low_integrity_windows_launch(caps): Low-IL primary (unchanged)
else:                                  null fallback    (unchanged)
```

Critical implementation details lifted from the reverted Option D in the debug session:

1. **RAII holder pattern (launch.rs:1131-1160):** Each holder must be bound to a *named local* (`let _restricted_holder = Some(holder)`) so its `Drop` does not run before `CreateProcessAsUserW` reads the raw handle. The 260417-wla quick task (commit history) fixed a UAF here. Wave 1 adds a third holder local for the Low-IL primary token; do NOT collapse into a temporary.
2. **`OwnedHandle` double-close avoidance (lines 1161-1162):** The comment "do NOT re-wrap h_token in a fresh OwnedHandle" remains binding. The new `_low_integrity_holder: Option<OwnedHandle>` already exists at line 1132 — Wave 1 just sets it from a new code path.
3. **Branch ordering matters:** `pty.is_some()` MUST precede `config.session_sid.is_some()` because `config.session_sid` is unconditionally `Some(generate_session_sid())` for all Windows supervised launches (`execution_runtime.rs:334`). The new arm is reached *because* it short-circuits before the WRITE_RESTRICTED branch.

[VERIFIED: launch.rs:1131-1160; execution_runtime.rs:334; 260417-wla quick task SUMMARY]

### Security Envelope Under Wave 1

| Property | HEAD (WRITE_RESTRICTED + session-SID) | Wave 1 (Low-IL primary, no session-SID) |
|----------|--------------------------------------|----------------------------------------|
| Mandatory-label write-deny on grant-set paths | Yes (subject Medium-IL, label NO_WRITE_UP, but TokenDominates=TRUE so write is permitted *— wait, that's wrong*; HEAD's actual semantics: WRITE_RESTRICTED double-gates writes against the session SID which is absent from object DACLs, so writes fail by DACL not by label) | Yes (subject Low-IL, label NO_WRITE_UP, TokenDominates=FALSE → kernel strips GENERIC_WRITE pre-DACL) |
| Mandatory-label read-deny on Write-only grant paths (mask=0x6 NO_READ_UP\|NO_EXECUTE_UP) | No effect (Medium-IL subject) | Yes (Low-IL subject, label NO_READ_UP fires) |
| Read-deny on paths outside grant set | No (WRITE_RESTRICTED only blocks writes; reads pass with normal user SID) | No (Low-IL subjects can read Medium-IL by default — same outcome) |
| Per-session SID kernel network identity (WFP `FWPM_CONDITION_ALE_USER_ID`) | Yes | **No — falls back to AppID-based filter** (Phase 15 detached-path waiver applies same shape; documented in `nono-wfp-service.rs:1244-1255`) |
| Job Object containment | Yes | Yes (unchanged) |
| Capability pipe access for sandboxed child | Yes (DACL has session-SID + logon-SID ACEs) | Yes (DACL's OW ACE matches; user SID equality between supervisor and child) |

[VERIFIED: MS-DTYP MandatoryIntegrityCheck pseudocode; sandbox/windows.rs:484-494; restricted_token.rs:82-93; nono-wfp-service.rs:1224-1255]

**Net security-envelope shift:** Per-session WFP SID differentiation is lost (fall-back to AppID; same waiver Phase 15 took for the detached path). Mandatory-label enforcement gains the read-deny dimension on Write-only grants (a strengthening relative to HEAD's WRITE_RESTRICTED-only shape). Long-lived interactive shells now have OS-level write-deny via MIC instead of via WRITE_RESTRICTED — a *qualitatively different mechanism* but acceptance-equivalent for D-06's "OS-level write-deny on paths outside grant set."

## Capability-Pipe SDDL Deep-Dive (Question 1 — most important)

**Question:** Does the capability-pipe SDDL (post-`938887f`) admit Low-IL primary-token clients?

**Answer: YES**, with high confidence. The pipe DACL already admits Low-IL clients via the OW (Owner Rights) ACE.

### Evidence chain

The capability pipe is created by `crates/nono/src/supervisor/socket_windows.rs::build_capability_pipe_sddl` with the SDDL [VERIFIED: socket_windows.rs:1196-1211]:

```
D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019F;;;<session_sid>)(A;;0x0012019F;;;<logon_sid>)S:(ML;;NW;;;LW)
```

Decomposed:
- **DACL:** `P` (protected), no inherited ACEs.
  - `(A;;GA;;;SY)` — System has Generic All. Doesn't apply (child is unprivileged user).
  - `(A;;GA;;;BA)` — Built-in Administrators have Generic All. Doesn't apply (typical POC user is non-admin).
  - `(A;;GA;;;OW)` — **Owner Rights have Generic All.** This is the load-bearing ACE.
  - `(A;;0x0012019F;;;<session_sid>)` — `FILE_GENERIC_READ | FILE_GENERIC_WRITE | SYNCHRONIZE` for the per-session restricting SID. **Wave 1 Low-IL token does NOT carry this SID (no `WRITE_RESTRICTED` second-pass check).** Harmless — DACL just doesn't match on this ACE for Wave 1 clients; check moves on.
  - `(A;;0x0012019F;;;<logon_sid>)` — Same mask for logon SID `S-1-5-5-X-Y`. **Wave 1 Low-IL token DOES carry this** (logon SID is preserved across `DuplicateTokenEx`; `SetTokenInformation(TokenIntegrityLevel)` does not remove group SIDs). Backup match path; pipe access succeeds even if OW path somehow fails.
- **SACL:** `(ML;;NW;;;LW)` — Low Integrity mandatory label, NO_WRITE_UP. Object IL = Low. Subject IL = Low. **TokenDominates(Low, Low) = TRUE.** MIC pre-check passes; access proceeds to DACL.

### Why OW matches under Low-IL primary token

`OW` (Owner Rights, SID `S-1-3-4`) is a virtual SID that resolves at access-check time to the *current owner of the object*. The capability pipe is created by the `nono.exe` supervisor process; the pipe's owner is the supervisor's user SID (the unprivileged user). The Low-IL primary token is constructed by:
1. `OpenProcessToken(GetCurrentProcess(), ...)` — opens the supervisor's token.
2. `DuplicateTokenEx(...)` — duplicates it.
3. `SetTokenInformation(token, TokenIntegrityLevel, &low_label, ...)` — replaces the integrity SID only.

[VERIFIED: launch.rs:1024-1112]

The token user SID is unchanged. The Low-IL child has the same TokenUser SID as the supervisor. When the child opens the pipe, the access check resolves OW to the pipe's owner SID, compares it to TokenUser, and **matches**. Generic All is granted by DACL.

### Why this is different from the pre-`938887f` failure mode

The `supervisor-pipe-access-denied.md` debug session (April 2026) hit `ERROR_ACCESS_DENIED` because of `WRITE_RESTRICTED`'s **second-pass** access check against restricting SIDs — a check that runs ONLY when the token has `TokenRestrictedSids` populated. The Low-IL primary token has NO restricting SIDs (no `WRITE_RESTRICTED`, no `CreateRestrictedToken` at all). There is no second pass to fail. Single-pass DACL evaluation matches OW and grants access.

[VERIFIED: socket_windows.rs:1196-1211; restricted_token.rs:72-93; supervisor-pipe-access-denied.md cycle 3 analysis]

### Lurking edge case (Wave 1 must verify in field test)

If the supervisor process's *own* identity has any non-default token shape that would cause OW resolution to differ on the supervisor side vs. the child side, the OW match could fail. **Verification:** Wave 1 task should include a `tracing::debug!` log line in the capability-pipe server thread (or read from existing logs) confirming "child connected to pipe" before declaring success. The existing `start_capability_pipe_server` already logs connection events — the field-test task just needs to read them.

[ASSUMED] The supervisor's token is the user's normal interactive-logon token; OW resolution is symmetric. Verified via field test, NOT via static analysis alone.

## Mandatory-Label Write-Deny Verification (Question 2)

**Question:** Does `try_set_mandatory_label` correctly fire NO_WRITE_UP for a Low-IL subject vs default Medium-IL files?

**Answer: YES**, with HIGH confidence. The MS-DTYP MandatoryIntegrityCheck algorithm is unambiguous.

### The kernel access-check arithmetic

For a Low-IL subject attempting to write to a Medium-IL (or unlabeled, Medium-default) object:

1. **TokenDominates(Low=0x1000, Medium=0x2000) = FALSE** [VERIFIED: MS-DTYP](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/ae69a089-473d-4c23-bf3d-7a12a9d11123)
2. **Phase 1 — Token policy:** `TOKEN_MANDATORY_POLICY_NO_WRITE_UP` is the default. With TokenDominates=FALSE, GENERIC_WRITE is **not** added to AllowedAccess. GENERIC_READ + GENERIC_EXECUTE are added.
3. **Phase 2 — ACE mask:** Object's SACL has `SYSTEM_MANDATORY_LABEL_NO_WRITE_UP` (default for objects without explicit label, AND explicitly set by `try_set_mandatory_label` for Read-mode grants). With TokenDominates=FALSE, GENERIC_WRITE is removed (was never added anyway).
4. **MIC check fails for write; DACL never gets a chance to grant write.**

### Two specific cases nono creates

**Case A — Path NOT in grant set (e.g., `~\Desktop`):**
- Path is unlabeled. Windows treats unlabeled as Medium-IL with default NO_WRITE_UP.
- Low-IL child writes → MIC denies. DACL never reached.
- **`Out-File ~/Desktop/test.txt` → "Access is denied."** [CITED: MS Learn — MIC §"Default integrity level"](https://learn.microsoft.com/en-us/windows/win32/secauthz/mandatory-integrity-control)

**Case B — Path IN grant set with mode=Read:**
- `try_set_mandatory_label(path, NO_WRITE_UP | NO_EXECUTE_UP)` applies an explicit Low-IL SACL label.
- Low-IL child reads → TokenDominates=TRUE (Low=Low). MIC permits. DACL grants read via user SID.
- Low-IL child writes → TokenDominates=TRUE. MIC permits write. **But:** the file's NTFS owner is the user; Windows defaults grant the user write access; the `try_set_mandatory_label` call itself does not strip user write rights. **HMMMM — this is a potential gap.** The mandatory label NO_WRITE_UP only triggers when TokenDominates=FALSE. If subject is Low and object is Low, NO_WRITE_UP doesn't fire. Therefore, *for paths inside the grant set with mode=Read*, the Low-IL subject CAN actually write at the kernel level.

**Wait — this matters for Acceptance #3 framing.** Acceptance #3 says "writes outside the grant set fail." For paths INSIDE the grant set with Read mode, Wave 1's enforcement is:
- Filesystem capability set blocks at the application layer (capability check).
- Mandatory label does NOT block (subject IL == object IL).

The cookbook's security-envelope paragraph must be honest about this: *Read-only grants are a CapabilitySet contract, not a kernel-MIC enforcement.* The kernel-MIC enforcement only fires for paths outside the grant set (no label = Medium default → Low subject blocked). This is a refinement, not a regression — HEAD's WRITE_RESTRICTED has the same property (the restricting SID is absent from grant-set paths' DACLs too).

[VERIFIED: MS-DTYP algorithm; sandbox/windows.rs:484-494 mask construction]

**Acceptance #3 verification recipe (drives Wave 1 field test):**

```powershell
# Inside sandboxed shell:
Out-File C:\Users\<u>\Desktop\nono-test.txt "should fail"
# Expected: "Access to the path 'C:\Users\<u>\Desktop\nono-test.txt' is denied."
# OR PowerShell exception with HResult 0x80070005 (ERROR_ACCESS_DENIED).
```

The path `~/Desktop` is NOT in the claude-code profile grant set; therefore unlabeled; therefore Medium-IL default; therefore Low → Medium write triggers MIC denial.

### Caveat: the existing leaked Low-IL labels

CONTEXT.md D-09 notes 9 user-home paths already carry leaked Low-IL labels with `prior_rid="0x1000"`. Per `AppliedLabelsGuard::snapshot_and_apply` D-02 semantics ([VERIFIED: labels_guard.rs:83-100]), the guard SKIPS apply+revert when ANY pre-existing mandatory-label ACE is detected. So:
- For these 9 leaked-label paths: `(ML;;<leaked-mask>;;;LW)` is already on disk. If `<leaked-mask>` includes NO_WRITE_UP, Low-IL child still gets write-deny (same outcome as fresh apply). If leaked-mask is something else, behavior may diverge from fresh-apply.
- Field test will see "label guard: skipping apply + revert" warnings. **These are EXPECTED and NOT failure indicators.** D-09 explicitly declares this out of scope.

## ProcMon Trace Plan for Wave 2 (Question 3)

**Trigger:** Wave 1 field test fails (either 0xC0000142 launch failure OR mandatory-label write-deny doesn't fire).

**Goal:** Surface a sixth option from the actual Win32 mechanism (`\Device\ConDrv` ALPC, `\BaseNamedObjects` access, conhost handshake), not a token-shape iteration.

**Timebox:** 3-5 working days per D-04.

### Tooling

[VERIFIED: [Microsoft Sysinternals — Process Monitor](https://learn.microsoft.com/en-us/sysinternals/downloads/procmon)]

ProcMon controls:
- `CTRL+E` — toggle data collection on/off
- `CTRL+X` — clear collected data
- `CTRL+S` — save trace as `.pml`

Companion: Process Explorer (Sysinternals) shows HPCON values in process handle listings.

### Filter recipe for ConPTY + restricted-token failures

[VERIFIED: web research — multiple ConPTY ProcMon traces in MS Q&A and devblogs]

Apply these filters BEFORE running the failing nono command:

```
Process Name is conhost.exe                        (capture pseudoconsole host)
Process Name is powershell.exe                     (the shell child)
Process Name is cmd.exe                            (alternate shell child)
Path contains \Device\ConDrv                       (console driver — Server, Signal, etc.)
Path contains \BaseNamedObjects                    (named sections / sync objects)
Path contains \Sessions\                           (per-session subdirectories)
Operation is Process Create                        (CreateProcessAsUserW chain)
Result is not SUCCESS                              (surface ACCESS_DENIED, INVALID_HANDLE)
```

### Events to look for

1. **Failed `\Device\ConDrv` operations** — ALPC port DACL denials when Low-IL primary token attempts to call ConPTY's input/output ports. Conhost.exe's command line contains `--server <handle>`, `--signal <handle>`, `--width <N>`, `--height <N>`. If conhost.exe spawns but fails to handshake, the parent supervisor process will see the failure cascade.
2. **`\BaseNamedObjects\` per-session access denials** — section objects, mutexes, events that the CLR or shell loader requires. Each per-Windows-session subdir under `\Sessions\<n>\BaseNamedObjects\` may have a DACL that excludes Low-IL writes.
3. **`Process Create` chain failures** — `CreateProcessAsUserW` returns success, but the child immediately exits 0xC0000142. The relevant trace rows are the *ImageLoad* events between the create and the process-exit. Look for the *first* ImageLoad with Result != SUCCESS — that's the DLL whose DllMain failed, equivalent identification to Phase 15's CLR-DllMain framing.

### What "surfaced a 6th option" looks like

A sixth option = a fix that does NOT mutate the token shape. Examples:
- "Add a DACL ACE to `\BaseNamedObjects\<X>` granting Low-IL ReadAccess." — a per-Windows-system setup step (requires elevation; documents an admin-install requirement for `nono shell` Low-IL).
- "ConPTY's signal pipe DACL needs Low-IL admit; create the pseudoconsole BEFORE dropping integrity." — a sequencing fix in `pty_proxy::open_pty` or `spawn_windows_child`.
- "Pre-load CLR/System32 DLLs into the supervisor and inject by hand into the child." — a broker-process pattern matching Microsoft Q&A's recommended workaround.

### What surfacing nothing looks like

If 3-5 days of ProcMon work yields no surfaced fix, Phase 30 ships the failure-mode outcome:
- PROJECT.md SHELL-01 → "deferred to v3.0"
- Cookbook revert (see Cookbook Rollback section below)
- v3.0 kernel mini-filter driver work scope — already deferred per CONTEXT.md.

## Acceptance #3 Test Pattern (Question 4)

**Question:** What test pattern verifies acceptance #3 (OS-level write-deny inside the live shell)?

### Existing test infrastructure

[VERIFIED: glob `crates/nono-cli/tests/*.rs`]

- `crates/nono-cli/tests/env_vars.rs:487` — `windows_run_executes_basic_command()` is the closest existing pattern. Spawns `nono.exe run -- cmd /c echo hello`, asserts on combined stdout+stderr.
- `nono_bin()` helper produces a `Command` for the workspace's nono binary.
- `combined_output(&output)` joins stdout + stderr for assertion.
- Test is gated by `#[cfg(target_os = "windows")]`.

There is **no existing test fixture** that drives `nono shell` end-to-end with stdin scripting. The interactive-shell tests in the codebase do dry-run plumbing only, not live shell I/O. This is an existing gap; Wave 1 task can introduce a fixture OR rely on manual harness.

### Recommended test pattern

**Option A — Manual harness (recommended for Wave 1 first-shot):**

```powershell
# scripts/test-windows-shell-write-deny.ps1
# Run on Windows test box after fresh `nono.exe` build from Wave 1.
$ErrorActionPreference = 'Continue'
Write-Host "==> Build:"
cargo build -p nono-cli --release --target x86_64-pc-windows-msvc

$nono = ".\target\x86_64-pc-windows-msvc\release\nono.exe"

# Inject a one-shot script into PowerShell that attempts a write outside the grant set.
$test = "Out-File C:\Users\$env:USERNAME\Desktop\nono-acceptance3.txt 'should fail'; if (Test-Path C:\Users\$env:USERNAME\Desktop\nono-acceptance3.txt) { exit 1 } else { exit 42 }"

Write-Host "==> Acceptance #3:"
& $nono shell --profile claude-code --allow-cwd --shell powershell.exe -- -NoLogo -Command $test

if ($LASTEXITCODE -eq 42) { Write-Host "PASS"; exit 0 }
elseif ($LASTEXITCODE -eq 1) { Write-Host "FAIL — write succeeded inside sandbox"; exit 1 }
else { Write-Host "INDETERMINATE — exit $LASTEXITCODE"; exit 2 }
```

This is intentionally non-Cargo-test-harness. The acceptance criteria explicitly require a *live* shell run; reproducing that under Cargo test is a separate engineering task and would gate Wave 1 on a test infrastructure deliverable.

**Option B — Cargo integration test (Wave 1 stretch goal):**

A test that spawns `nono.exe shell --shell cmd.exe -- /c "echo test > %USERPROFILE%\Desktop\nono-test.txt && exit 0 || exit 1"` and asserts the supervisor exits non-zero with the child's "Access is denied" surfaced in stderr. Caveat: cmd.exe under ConPTY may not propagate the access-denied error through `&&` cleanly; PowerShell-script harness is more reliable.

### Reads-still-work test (acceptance #4)

```powershell
# Inside the same sandboxed shell:
$claudeJson = "$env:USERPROFILE\.claude\claude.json"
if (Test-Path $claudeJson) {
    Get-Content $claudeJson -TotalCount 1
    # Expect: first line of JSON; exit 0
}
```

This tests that Read grants still work with Low-IL primary token. Behavior matches both HEAD (WRITE_RESTRICTED reads pass) and Wave 1 (Low-IL labels grant read at TokenDominates=TRUE). Should pass cleanly.

## Cookbook Rollback Path (Question 5)

**Question:** What's the rollback path for the cookbook if Wave 2 also fails?

### What today's commit `0c69bd4b` added

[VERIFIED: git show 0c69bd4b stat; cookbook lines 11-17, 155-160, 233-256]

The 2026-05-07 cookbook update at `docs/cli/development/windows-poc-handoff.mdx`:
1. Top-of-doc `<Note>` block recommending `nono shell --profile claude-code` as the TUI-agent path on Windows (lines 11-17).
2. Step 4 instruction: "Use `nono shell`, not `nono run`" with PowerShell example (lines 155-160).
3. Step 5 "Interactive verification (manual)" block describing the shell+claude flow (lines 217-231).
4. Step 6 user-handoff table row: "Always use `nono shell` on Windows" (line 250).
5. "Known limitation: `nono run` cannot host TUI agents" section (lines 233-239).

### Wave 1 success path (cookbook update)

If Wave 1 Wave 2 closes successfully, the cookbook gets an honest security-envelope paragraph. Suggested location: between step 4 and step 5, OR appended to the `<Note>` block. Sample text shape (planner has full discretion; this is a starting point):

```markdown
**Security envelope under `nono shell` on Windows (Phase 30):**
The sandboxed shell child runs under a Low Integrity primary token. Filesystem
write enforcement comes from per-path mandatory integrity labels (NO_WRITE_UP
mask) — kernel-level access checks deny writes to paths outside the grant set
before DACL evaluation. Read access to granted paths uses the same mandatory-
label mechanism (NO_READ_UP for Write-only grants; pass-through for ReadWrite
grants). Per-session WFP differentiation via the synthetic restricting SID is
NOT used on this path; outbound network filtering falls back to AppID-based
filtering (same waiver Phase 15 documented for the `nono run --detached` path).
The Claude Code PreToolUse hook is defense-in-depth on top of the OS-level
write-deny.
```

### Wave 2 failure path (cookbook revert)

If Wave 1 fails AND Wave 2 ProcMon investigation surfaces no sixth option, the cookbook reverts. Two options:

**Option Rev-A (clean revert):** `git revert 0c69bd4b` produces a single revert commit. Output cookbook is byte-identical to pre-`0c69bd4b` state. Pro: zero ambiguity. Con: loses the now-known-true text about "`nono run` cannot host TUI agents" (which was correct content even though the recommendation it set up was wrong).

**Option Rev-B (text replacement):** Keep the "Known limitation: `nono run` cannot host TUI agents" section (lines 233-239 — that's still factually correct). Strip the `<Note>` recommendation, the Step 4 instruction, the Step 5 interactive-verification block, and the Step 6 row. Add a new section: "`nono shell` on Windows is deferred to v3.0" with a brief explanation referencing this phase's evidence.

**Recommendation:** Option Rev-B. The "known limitation" content has independent value; the `nono shell` recommendation is the part that needs to come out. Wave 2 failure-path task should be specific text-replacement, not a `git revert`.

### Bookkeeping symmetry

PROJECT.md SHELL-01 entry tracking:
- Wave 1 first task (always): flip `✔` to `⚠ needs-rework` (or equivalent "in flight" marker).
- Wave 1 success task (last): flip `⚠` to `✔ validated v2.X Phase 30` with a reference to Phase 30 acceptance evidence.
- Wave 2 failure task (last): flip `⚠` to `✘ deferred to v3.0` with a reference to this phase's failure-mode evidence.

## Test Coverage for `create_low_integrity_primary_token` (Question 6)

**Question:** What's the test for `create_low_integrity_primary_token` itself?

### Current state

[VERIFIED: grep across crates and planning docs]

- The function is defined at `launch.rs:1024-1112`.
- It is **dead code on the supervised path** today (per Phase 15 documentation and CONTEXT.md). Live callers exist on the LEGACY Direct path via `should_use_low_integrity_windows_launch(caps)` — but in practice that path is rarely if ever taken because `execution_runtime.rs:334` always populates `config.session_sid` for supervised launches, and the supervised path is the only one taken for non-detached commands.
- Looking at the launch.rs cascade more carefully: the `should_use_low_integrity_windows_launch(caps)` branch is reached only when `is_windows_detached_launch == false` AND `config.session_sid.is_none()`. Since `session_sid` is unconditionally `Some(...)` in `execution_runtime.rs:334`, this branch is **structurally unreachable** at runtime today.
- **Existing tests for `create_low_integrity_primary_token`:** None directly. The function has unit-test discoverable patterns in the 260417-wla quick task plan ("test that the OwnedHandle is bound to a named local") but no `#[cfg(test)]` block exercises the function.

### Wave 1 implications

Because Wave 1 is the FIRST live-runtime exercise of `create_low_integrity_primary_token` outside test fixtures, Wave 1 must include:

1. **Unit test** asserting the function returns a non-null handle and the duplicated token has integrity SID `S-1-16-4096` (Low). Pattern lifted from `restricted_token.rs:148-198`'s test for `create_restricted_token_with_sid_applies_write_restricted_flag`. Use `GetTokenInformation(TokenIntegrityLevel)` to verify the IL.
2. **Cascade gate test** asserting that when `pty.is_some()`, the cascade arm taken is the Low-IL primary one (NOT the WRITE_RESTRICTED one). Pattern: extract the gate decision into a pure function or helper testable without spawning a process. Mirror the `detached_token_gate_tests` pattern at `launch.rs:1408-1437` — 4 tests asserting the truth table.
3. **Field smoke** as part of acceptance criteria — see Acceptance #3 Test Pattern above.

### Why this matters

If `create_low_integrity_primary_token` has a latent bug (e.g., the `SetTokenInformation(TokenIntegrityLevel)` succeeds but the integrity label doesn't actually apply to the duplicated token in some corner case), Wave 1 would silently produce a Medium-IL child and Acceptance #3 would fail without obvious diagnosis. The unit test surface that case immediately on the build host (no Windows test box trip required).

[VERIFIED: launch.rs grep; restricted_token.rs:123-258 test patterns]

## AppliedLabelsGuard Interaction (Question 7)

**Question:** What's the AppliedLabelsGuard expected behavior under Low-IL primary token?

**Answer: Unchanged — guard logic is token-shape-agnostic.**

[VERIFIED: labels_guard.rs:83-154]

The guard's responsibility is supervisor-side: it labels paths in the policy set with mandatory-label ACEs at Low IL (with mode-derived mask), and reverts them at Drop. The guard runs in the supervisor process BEFORE child spawn. The child token shape doesn't enter the guard's decision tree.

What WILL surface during Wave 1 field test:

1. **9 "skipping apply + revert" warnings** for the leaked-label user-home paths. Per CONTEXT.md D-09 these are out of scope. Expected; not failure indicators.
2. **1 "path not owned by current user" warning** for `C:\Windows`. Per Phase 21 design (labels_guard.rs:114-124). Expected; not a failure indicator.
3. **0 "apply failed; reverting" errors** on a clean run. If any appear, that's a real failure — propagate via `NonoError::LabelApplyFailed`.

### Edge case worth flagging

If Wave 1's field test runs first on a host where prior nono runs have leaked Low-IL labels with masks that DIFFER from what Wave 1's `try_set_mandatory_label` would produce (e.g., a leaked mask of `0x4` NO_EXECUTE_UP only on a path that should now be mode=Read with mask `0x5` NO_WRITE_UP|NO_EXECUTE_UP), the `Skip` semantics mean the path operates under the OLD leaked mask. Acceptance #3 might pass or fail differently depending on the leaked mask.

**Wave 1 mitigation:** Field test should run on a **fresh** test-box state where leaked labels don't exist, OR explicitly clear them at test start via a helper script. CONTEXT.md notes 9 specific paths; a small PowerShell prelude can clear them:

```powershell
$paths = @(
    "$env:USERPROFILE\.cache\claude",
    "$env:USERPROFILE\.cargo",
    "$env:USERPROFILE\.claude",
    "$env:USERPROFILE\.config\git\ignore",
    "$env:USERPROFILE\.gitconfig",
    "$env:USERPROFILE\.local\bin",
    "$env:USERPROFILE\.rustup",
    "$env:USERPROFILE\AppData\Roaming\nono\profiles",
    "$env:USERPROFILE\Nono"
)
foreach ($p in $paths) { icacls $p /setintegritylevel "(NX)Medium" 2>$null }
# Or use the dedicated label-clear API; icacls' /setintegritylevel can also CLEAR via /remove:s syntax.
```

[VERIFIED: labels_guard.rs:83-100; CONTEXT.md D-09]

## Other `create_low_integrity_primary_token` Callsites (Question 8)

**Question:** Has any other code path used `create_low_integrity_primary_token` already?

**Answer: One structurally-unreachable callsite. Wave 1 is effectively the first live use.**

[VERIFIED: grep across `crates/`]

- `crates/nono-cli/src/exec_strategy_windows/launch.rs:1024` — the definition.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs:1151` — the only callsite, in the legacy Direct cascade arm `should_use_low_integrity_windows_launch(config.caps)`. As analyzed in Question 6, that arm is structurally unreachable because `config.session_sid` is always `Some` for Windows.
- Planning artifacts reference the function (Phase 06, 14-01, 15) but those are docs / archived plans, not live code.

**Implication:** Wave 1 effectively introduces the first live runtime use of `create_low_integrity_primary_token` end-to-end. Treat this as a new feature, not as an extension of an established pattern. Field test is non-optional.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Windows 10/11 build 17763+ | ConPTY (HPCON) | Test box: ✓ (Win 11 26200) | 10.0.26200 | — |
| Sysinternals ProcMon (Wave 2 only) | ProcMon trace plan | ✓ (free download) | n/a | — |
| Sysinternals Process Explorer (Wave 2 only) | HPCON handle inspection | ✓ (free download) | n/a | — |
| Rust toolchain 1.77+ | Workspace builds | ✓ | per `rust-toolchain.toml` | — |
| `--target x86_64-pc-windows-msvc` | Wave 1 build | ✓ | n/a | — |
| WFP service (`nono-wfp-service`) | Network kernel enforcement | Test box: ✗ (per-user MSI) | n/a | Wave 1 doesn't depend on this; AppID fallback works without WFP service |
| `claude` CLI | Acceptance #2 TUI verification | Per CONTEXT.md user has it | n/a | — |
| `powershell.exe` 5.1 | Default `nono shell` shell | ✓ (Windows-native) | 5.1 | `cmd.exe` |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** WFP service. Network filtering is already documented as off on this host; Wave 1 doesn't introduce or remove that limitation.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Cargo test runner (Rust built-in) + `proptest` for property-based tests + manual PowerShell harness for live-shell flows |
| Config file | `Cargo.toml` workspace; per-crate `[dev-dependencies]`; no central pytest-style config |
| Quick run command | `cargo test -p nono-cli --target x86_64-pc-windows-msvc --bin nono detached_token_gate_tests` (≈3s on a hot build) |
| Full suite command | `cargo test --workspace --target x86_64-pc-windows-msvc --all-features` (≈3-6 min on a hot build); plus `scripts/windows-test-harness.ps1 -Suite all` |
| Live-shell harness | Manual: `scripts/test-windows-shell-write-deny.ps1` (Wave 1 task creates this) |

### Phase Requirements → Test Map

Maps decisions D-01..D-10 + acceptance #1-#6 (CONTEXT.md tracking units) to validation:

| Tracking Unit | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| D-01 / Acc #1 | `nono shell --profile claude-code --allow-cwd` launches without 0xC0000142 | manual + smoke | `.\target\<target>\release\nono.exe shell --profile claude-code --allow-cwd` (exit code 0 OR ≥ user-typed `exit`) | ❌ Wave 1 manual harness |
| D-01 | Low-IL primary token has integrity SID 0x1000 | unit | `cargo test -p nono-cli low_integrity_primary_token_sets_low_il --target x86_64-pc-windows-msvc` | ❌ Wave 1 |
| D-01 | Token cascade selects Low-IL primary when `pty.is_some()` AND `!is_windows_detached_launch` | unit | `cargo test -p nono-cli pty_token_gate_tests --target x86_64-pc-windows-msvc` | ❌ Wave 1 (mirror `detached_token_gate_tests`) |
| D-05 / Acc #2 | Claude Code TUI renders inside sandboxed shell (alternate screen, cursor positioning, raw-mode input) | manual | Visual: launch `nono shell --profile claude-code --allow-cwd`, run `claude`, observe TUI; type `/quit` then `exit` | ❌ Wave 1 manual; documented in field-smoke runbook |
| D-06 / Acc #3 | `Out-File ~/Desktop/test.txt` from inside shell fails "Access is denied" | live shell | `scripts/test-windows-shell-write-deny.ps1` (exit 42 = pass) | ❌ Wave 1 |
| Acc #4 | `Get-Content ~/.claude/claude.json` from inside shell succeeds | live shell | Inline in same harness script | ❌ Wave 1 |
| D-10 / Acc #5 | PROJECT.md SHELL-01 reflects current reality | grep | `grep -E 'SHELL-01.*v2\.0 Phase 08' .planning/PROJECT.md` should return EMPTY after Wave 1 first task | n/a (manual file edit verification) |
| Acc #6 | Cookbook documents security envelope honestly | grep | `grep -E 'Low.Integrity.primary.token' docs/cli/development/windows-poc-handoff.mdx` should return non-empty after Wave 1 last task | n/a |
| D-09 (negative) | Wave 1 does NOT block on AppliedLabelsGuard label leak | manual smoke | "skipping apply + revert" warnings present and acknowledged in field-test log | n/a |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli --target x86_64-pc-windows-msvc <unit-tests-for-modified-arms>` (≤30s).
- **Per wave merge:** Full `scripts/windows-test-harness.ps1 -Suite smoke` (≈2-5 min) + `scripts/test-windows-shell-write-deny.ps1` on test box.
- **Phase gate:** All acceptance criteria verified manually + automated unit tests green + cookbook + PROJECT.md updates committed.

### Wave 0 Gaps

- [ ] **`scripts/test-windows-shell-write-deny.ps1`** — does not exist; Wave 1 creates it. Drives Acceptance #3 + #4 verification.
- [ ] **Unit tests for `create_low_integrity_primary_token`** — no existing test exercises the function. Wave 1 adds a `#[cfg(all(test, target_os = "windows"))]` test asserting non-null handle and integrity SID 0x1000.
- [ ] **Unit tests for the `pty.is_some()` cascade gate** — Wave 1 mirrors `detached_token_gate_tests` (launch.rs:1408-1437) for the new arm. ≈4 tests covering the truth table: (`is_detached=false, has_pty=true` → Low-IL primary), (`is_detached=true, has_pty=true` → null), (`is_detached=false, has_pty=false, has_session_sid=true` → WRITE_RESTRICTED), (`is_detached=false, has_pty=false, has_session_sid=false` → null fallback).
- [ ] **Field-smoke runbook** — text checklist (PROCMON-style filter + log-line markers + expected outputs) for Acceptance #1 + #2. Embed in commit body OR add to `.planning/phases/30-windows-nono-shell-architecture/30-FIELD-SMOKE.md`. Planner discretion.

## Code Examples

Verified patterns from existing nono code:

### Token cascade arm (existing pattern at launch.rs:1140-1160 — Wave 1 extends with new arm)

```rust
// Source: launch.rs:1131-1160 (HEAD)
let _restricted_holder: Option<restricted_token::RestrictedToken>;
let _low_integrity_holder: Option<OwnedHandle>;
let is_windows_detached_launch = is_windows_detached_launch();
let h_token: HANDLE = if is_windows_detached_launch {
    _restricted_holder = None;
    _low_integrity_holder = None;
    std::ptr::null_mut()
} else if let Some(ref sid) = config.session_sid {        // Wave 1 INSERTS new arm BEFORE this line
    let holder = restricted_token::create_restricted_token_with_sid(sid)?;
    let raw = holder.h_token;
    _restricted_holder = Some(holder);
    _low_integrity_holder = None;
    raw
} else if should_use_low_integrity_windows_launch(config.caps) {
    let holder = create_low_integrity_primary_token()?;
    let raw = holder.0;
    _low_integrity_holder = Some(holder);
    _restricted_holder = None;
    raw
} else {
    _restricted_holder = None;
    _low_integrity_holder = None;
    std::ptr::null_mut()
};
```

### Wave 1 new arm shape

```rust
// New arm — inserted between is_windows_detached_launch and config.session_sid arms.
// Pattern: bind holder to a named local; set h_token from raw handle; null out other holders.
} else if pty.is_some() {
    // Phase 30 D-01: ConPTY path uses Low-IL primary token (no WRITE_RESTRICTED,
    // no session-SID). WRITE_RESTRICTED + ConPTY triggers STATUS_DLL_INIT_FAILED
    // (0xC0000142) — same class of bug Phase 15 hit on the detached path with
    // DETACHED_PROCESS. Mandatory-label NO_WRITE_UP enforces write-deny because
    // Low-IL subjects do not dominate Medium-IL files (MIC pre-DACL kernel check).
    // Per-session WFP differentiation via FWPM_CONDITION_ALE_USER_ID is waived
    // on this path (falls back to AppID-based filtering, same as Phase 15
    // detached-path waiver). See .planning/phases/30-windows-nono-shell-architecture/30-CONTEXT.md.
    let holder = create_low_integrity_primary_token()?;
    let raw = holder.0;
    _low_integrity_holder = Some(holder);
    _restricted_holder = None;
    raw
}
```

### Existing test pattern — `detached_token_gate_tests` (Wave 1 mirrors for `pty_token_gate_tests`)

```rust
// Source: launch.rs:1408-1437
#[cfg(test)]
mod detached_token_gate_tests {
    use super::is_windows_detached_launch;
    use crate::test_env::{lock_env, EnvVarGuard};

    #[test]
    fn returns_false_when_env_unset() {
        let _lock = lock_env();
        let g = EnvVarGuard::set_all(&[("NONO_DETACHED_LAUNCH", "1")]);
        g.remove("NONO_DETACHED_LAUNCH");
        assert!(!is_windows_detached_launch());
    }
    // ... 2 more tests covering the truth table.
}
```

### Existing pattern — Low-IL token integrity verification (Wave 1 reuses for new unit test)

```rust
// Source: restricted_token.rs:148-198 (the WRITE_RESTRICTED test, but pattern is reusable)
let token = create_low_integrity_primary_token().expect("must succeed");
assert!(!token.0.is_null(), "low-integrity primary token handle is non-null");

// Query TokenIntegrityLevel — must be Low (0x1000).
let mut needed: u32 = 0;
unsafe {
    GetTokenInformation(token.0, TokenIntegrityLevel, std::ptr::null_mut(), 0, &mut needed);
}
let mut buf = vec![0u8; needed as usize];
let ok = unsafe {
    GetTokenInformation(token.0, TokenIntegrityLevel, buf.as_mut_ptr() as *mut _, needed, &mut needed)
};
assert!(ok != 0);
let label = unsafe { &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL) };
let sub_authority_count = unsafe { *GetSidSubAuthorityCount(label.Label.Sid) };
let last_sub_authority = unsafe {
    *GetSidSubAuthority(label.Label.Sid, (sub_authority_count - 1) as u32)
};
assert_eq!(last_sub_authority, SECURITY_MANDATORY_LOW_RID,
    "duplicated token must be at Low integrity (0x1000)");
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Token integrity drop | Custom token-creation FFI | Existing `create_low_integrity_primary_token()` | Already implemented; tested in 260417-wla quick task; correct UAF discipline |
| ConPTY allocation | New pseudoconsole code | Existing `pty_proxy::open_pty()` | Already wired; ConPTY semantics unchanged in Wave 1 |
| Mandatory label apply | Custom SDDL parsing | Existing `try_set_mandatory_label` (sandbox/windows.rs:514) | Already wraps `ConvertStringSecurityDescriptorToSecurityDescriptorW`; correct error handling per CLAUDE.md |
| Capability pipe DACL | New SDDL string | Existing `build_capability_pipe_sddl(session_sid)` | Already correct for Low-IL clients (per Question 1 analysis above) |
| Test env-var save/restore | Manual `std::env::set_var` calls | Existing `EnvVarGuard` + `lock_env()` (test_env.rs) | CLAUDE.md mandates env-var save/restore pattern |
| Live-shell I/O scripting | Cargo integration test | Manual PowerShell harness (Wave 1 first delivery) | Cargo can't drive live shell I/O cleanly; harness is honest |

## Common Pitfalls

### Pitfall 1: OwnedHandle UAF when h_token comes from a temporary

**What goes wrong:** Binding `let h_token = create_low_integrity_primary_token()?.0;` returns a raw HANDLE from a temporary `OwnedHandle` that drops (closing the handle) before `CreateProcessAsUserW` reads it. Result: `ERROR_INVALID_HANDLE` (6).
**Why it happens:** `OwnedHandle::Drop` calls `CloseHandle`. Rust temporaries die at end-of-statement.
**How to avoid:** Bind to a NAMED local (`let _holder = ...; let raw = _holder.0;`) so the holder lives until end-of-function. The existing cascade does this; Wave 1 must do the same for the new arm.
**Warning signs:** `CreateProcess*` returns 0 with GetLastError = 6 (ERROR_INVALID_HANDLE).

[VERIFIED: 260417-wla quick task SUMMARY; comment block at launch.rs:1126-1130]

### Pitfall 2: ConPTY + integrity-level mismatch (Microsoft-documented)

**What goes wrong:** Microsoft Q&A documents that when a SYSTEM-level process creates a pseudoconsole and launches a lower-IL child, the child gets ACCESS_DENIED on `WriteConsoleInput()`. Source: integrity-level mismatch between pseudoconsole and child.
**Why it happens:** `CreatePseudoConsole` doesn't take a security descriptor; the pseudoconsole is created at the caller's IL.
**How to avoid:** Recommended workaround is a "broker process" pattern — the broker creates the pseudoconsole at the *child's* IL.
**Why this MIGHT bite Wave 1:** nono's supervisor is Medium-IL. The pseudoconsole is created in the supervisor (`pty_proxy::open_pty()`). The Wave 1 child is Low-IL. **This is exactly the integrity-mismatch scenario Microsoft Q&A flags.**
**Warning signs:** Child launches without 0xC0000142, BUT user typing into the shell produces no input (silent input drop) OR shell echo is broken OR cmd.exe prints prompt but never accepts a command.
**Mitigation in Wave 1:** Field test must specifically test interactive typing inside the shell, not just shell launch + immediate exit.

[VERIFIED: [Microsoft Q&A — CreatePseudoConsole with reduced integrity level](https://learn.microsoft.com/en-us/answers/questions/1040676/createpseudoconsole-with-reduced-integrity-level)]

### Pitfall 3: Mandatory-label NO_WRITE_UP ineffective when subject and object IL match

**What goes wrong:** A Low-IL child writes to a Low-IL labeled file (path inside the grant set) and the write **succeeds**, even though the SACL has NO_WRITE_UP.
**Why it happens:** MIC's NO_WRITE_UP check fires only when TokenDominates=FALSE. Low → Low has TokenDominates=TRUE; the write is permitted.
**How to avoid:** Don't expect mandatory-label enforcement on grant-set paths. CapabilitySet contract is what bounds writes inside the grant set; mandatory label only enforces OUTSIDE the grant set.
**Warning signs:** Wave 1 cookbook security-envelope text overclaims if it implies mandatory-label denies all unauthorized writes. Be specific: "writes to paths OUTSIDE the grant set are denied."

[VERIFIED: MS-DTYP MandatoryIntegrityCheck pseudocode]

### Pitfall 4: Capability-pipe rendezvous file under labeled directory

**What goes wrong:** `execution_runtime.rs:211` puts the rendezvous file under `std::env::temp_dir()`. If `%TEMP%` is somehow under a labeled directory (it shouldn't be, but Windows weirdness), the Low-IL child might fail to read the rendezvous file even though the named-pipe DACL admits it.
**Why it happens:** The rendezvous file is a regular file with default Medium-IL labels; Low-IL child can read it (per MIC default — NO_READ_UP is NOT default). But if the directory above it has NO_READ_UP set, traversal can fail.
**How to avoid:** Verify `%TEMP%` is not under any nono-labeled path. On the test box, `%TEMP%` = `C:\Users\<u>\AppData\Local\Temp`, which IS under `C:\Users\<u>` — not labeled by claude-code profile, so safe. But the leaked label set per D-09 includes `~/AppData/Roaming/nono/profiles` (sibling of Local/Temp; safe), not `~/AppData/Local/Temp` itself.
**Warning signs:** "Failed to connect to Windows supervisor pipe" with `ERROR_FILE_NOT_FOUND` (rendezvous file unreadable) instead of `ERROR_ACCESS_DENIED` (DACL mismatch).
**Mitigation in Wave 1:** Likely non-issue; document for vigilance only.

[VERIFIED: execution_runtime.rs:211; supervisor-pipe-access-denied.md cycle 1 H1 elimination]

### Pitfall 5: Releasing h_token before `CreateProcessAsUserW` returns

**What goes wrong:** Wrapping `h_token` in a fresh `OwnedHandle` outside the holder pattern double-closes the handle.
**Why it happens:** The cascade's existing comment (launch.rs:1161-1162) flags this. Wave 1 must preserve the discipline.
**How to avoid:** The holder bindings already exist (`_restricted_holder`, `_low_integrity_holder`). Wave 1 just sets `_low_integrity_holder = Some(holder)` from the new arm and reads `holder.0` once.

[VERIFIED: comment at launch.rs:1161-1162]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| WRITE_RESTRICTED + session-SID restricting set + ConPTY (HEAD) | Low-IL primary token + ConPTY (Wave 1) | Phase 30 (this) | Fixes 0xC0000142 launch failure; gains read-deny on Write-only grants; loses per-session WFP SID differentiation |
| `should_allocate_pty()` returns true unconditionally on Windows | Returns true ONLY when `interactive_pty` (`nono shell`) is set, not for detached | Phase 15 (commit `802c958`) | Detached path is null-token; supervised PTY path is what Wave 1 changes |
| `nono shell` claimed validated v2.0 Phase 08 | "needs rework" → either "validated v2.X Phase 30" or "deferred to v3.0" | Wave 1 first task + last task | PROJECT.md SHELL-01 entry corrected |

**Deprecated/outdated:**

- The Phase 08 (v2.0) claim that `nono shell` interactive ConPTY was validated on Windows. The smoke gate at the time did not include `--profile claude-code` end-to-end with full WRITE_RESTRICTED + ConPTY. Bookkeeping correction is Wave 1 task #1.

## Sources

### Primary (HIGH confidence)

- `crates/nono-cli/src/exec_strategy_windows/launch.rs:1024-1349` — token construction, cascade, CreateProcess flow
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs:34-258` — WRITE_RESTRICTED semantics + tests
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` — AppliedLabelsGuard apply/revert logic
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs:148-417` — capability-pipe server lifecycle
- `crates/nono/src/supervisor/socket_windows.rs:1054-1211` — `current_logon_sid`, `build_capability_pipe_sddl`
- `crates/nono/src/sandbox/windows.rs:46-117, 470-650` — `apply()`, `try_set_mandatory_label`, `path_is_owned_by_current_user`
- `crates/nono-cli/src/supervised_runtime.rs:95-417` — `should_allocate_pty`, `execute_supervised_runtime`
- `crates/nono-cli/src/bin/nono-wfp-service.rs:1224-1437` — WFP filter installation; `FWPM_CONDITION_ALE_USER_ID` vs `FWPM_CONDITION_ALE_APP_ID` selection
- `.planning/debug/nono-shell-status-dll-init-failed.md` — full investigation trail (paused-pending-architecture-review)
- `.planning/debug/resolved/windows-supervised-exec-cascade.md` — Phase 15 precedent + smoke-gate evidence
- `.planning/debug/resolved/supervisor-pipe-access-denied.md` — capability-pipe SDDL cycle-3 SDDL evolution
- `.planning/phases/30-windows-nono-shell-architecture/30-CONTEXT.md` — locked decisions D-01..D-10
- [Microsoft Learn — Mandatory Integrity Control](https://learn.microsoft.com/en-us/windows/win32/secauthz/mandatory-integrity-control) — MIC overview, default labels
- [MS-DTYP MandatoryIntegrityCheck Algorithm Pseudocode](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/ae69a089-473d-4c23-bf3d-7a12a9d11123) — exact access-check algorithm
- [Microsoft Learn — Pseudoconsoles overview](https://learn.microsoft.com/en-us/windows/console/pseudoconsoles) — ConPTY architecture
- [Microsoft Learn — Creating a Pseudoconsole session](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session) — startup sequence
- [Microsoft Sysinternals — Process Monitor](https://learn.microsoft.com/en-us/sysinternals/downloads/procmon) — Wave 2 ProcMon tool

### Secondary (MEDIUM confidence)

- [Microsoft Q&A — CreatePseudoConsole with reduced integrity level](https://learn.microsoft.com/en-us/answers/questions/1040676/createpseudoconsole-with-reduced-integrity-level) — confirms the integrity-mismatch failure mode (Pitfall 2). MEDIUM because the Q&A's scenario (SYSTEM creating pseudoconsole) is not byte-identical to nono's scenario (Medium-IL supervisor creating pseudoconsole, Low-IL child). The class of failure is the same but the specific manifestation may differ.
- [DevBlog — Introducing the Windows Pseudo Console (ConPTY)](https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/) — historical context for ConPTY's design tradeoffs.

### Tertiary (LOW confidence)

- [Wikipedia — Mandatory Integrity Control](https://en.wikipedia.org/wiki/Mandatory_Integrity_Control) — overview reference; confirmed against MS Learn primary source.
- [Steve Riley — Mandatory integrity control in Windows Vista](https://learn.microsoft.com/en-us/archive/blogs/steriley/mandatory-integrity-control-in-windows-vista) — historical Vista-era explainer; semantics unchanged on Windows 10/11.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The supervisor's token is the user's normal interactive-logon token; OW resolution is symmetric (same user SID on both supervisor and Low-IL child). | Capability-Pipe SDDL Deep-Dive | If supervisor runs with a different user SID (unusual but possible in non-default contexts), child's OW match would fail and pipe access would be denied. Wave 1 field test surfaces this immediately as "Failed to connect to Windows supervisor pipe." |
| A2 | `SetTokenInformation(TokenIntegrityLevel)` applied to a duplicated token reliably produces an effective Low-IL token at child-process level (no Windows quirk where the integrity SID is silently ignored). | Question 6 — Test Coverage | If the call succeeds but the integrity level is silently un-applied, child runs at Medium-IL and Acceptance #3 fails despite passing builds and unit tests. Wave 1 mitigation: query `TokenIntegrityLevel` in unit test (already proposed). |
| A3 | Windows treats files without an explicit mandatory-label SACL as Medium-IL with default NO_WRITE_UP, applying MIC pre-DACL check uniformly. | Question 2 — Mandatory-Label Write-Deny | If specific filesystems (ReFS, network shares) skip MIC enforcement, writes outside the grant set could succeed silently. NTFS is in scope; ReFS/network are out of scope per existing nono limitations. |
| A4 | The Microsoft Q&A integrity-mismatch failure mode (Pitfall 2) does not manifest for Medium-IL supervisor + Low-IL child specifically; only for SYSTEM-IL + lower-IL. | Pitfall 2 | If it DOES manifest (silent input drop, broken echo), Wave 1 launches cleanly but TUI is unusable — Acceptance #2 fails. Wave 2 ProcMon then surfaces the mechanism. |
| A5 | `pty_proxy::open_pty()` does not embed any token-shape assumption that breaks under Low-IL primary token (the call is in the supervisor, before child spawn). | Architecture Map | If the pseudoconsole has hidden affinity to a token that isn't the child's, behavior is unspecified. Existing tests confirm pseudoconsole works under WRITE_RESTRICTED today; Low-IL is a different shape but no specific reason to expect divergence. |
| A6 | The 9 leaked Low-IL labels on the test box are a stable known-issue and don't interact with Wave 1 enforcement in surprising ways. | Question 7 — AppliedLabelsGuard | If a leaked label has a mask that contradicts what Wave 1's mode-derived mask would set, the path enforces under the stale mask, possibly making Acceptance #3 false-pass or false-fail on those specific paths. Mitigation: clear leaked labels at field-test start. |

**Acknowledgement to planner / discuss-phase:** Six [ASSUMED] claims is more than ideal. A1, A2, A4 are the ones most worth user confirmation before Wave 1 commits to the field test. A3, A5, A6 are operational safeguards already mitigated by test recipes; user confirmation is nice-to-have.

## Open Questions

1. **Should `config.session_sid` be passed to the supervisor's capability-pipe server even when the child Low-IL token does not carry it?**
   - What we know: `execution_runtime.rs:334` always populates `session_sid`. The capability-pipe SDDL adds the per-session-SID DACL ACE based on this. Under Wave 1, the Low-IL child does NOT carry the per-session SID, so this ACE never matches.
   - What's unclear: Is leaving the dead ACE in the SDDL a security-irrelevant no-op, or does it open an attack surface (e.g., another process forging the per-session SID into its token to access the pipe)?
   - Recommendation: Wave 1 KEEPS the existing SDDL plumbing unchanged. The per-session SID is generated fresh per-session via `Uuid::new_v4()` (`restricted_token.rs:21-32`); forging it requires guessing 128 bits of randomness. Treat the leftover ACE as a benign no-op that simplifies the cascade. Document in cookbook security-envelope text.

2. **Wave 1 commit body — does the `Security-Waiver:` trailer block from Phase 15 apply here verbatim, or do we need a new wording?**
   - What we know: Phase 15's commit `802c9584` had a `Security-Waiver:` trailer documenting null-token + AppID-WFP fallback for the detached path.
   - What's unclear: Wave 1's waiver is narrower (per-session WFP SID is lost; mandatory-label-based write-deny REPLACES WRITE_RESTRICTED-based write-deny — qualitatively similar enforcement, different mechanism). Does a fresh trailer make sense, or does the existing waiver shape generalize?
   - Recommendation: Planner discretion. Either way, the trailer must explicitly state: (1) per-session WFP SID waived; (2) AppID-WFP filter is the network-identity fallback; (3) mandatory-label NO_WRITE_UP is the write-deny mechanism for the supervised+PTY path; (4) waiver is scoped to the supervised+PTY path (Job Object + capabilities + Low-IL labels remain primary).

3. **Should Wave 1 also test cmd.exe under Low-IL primary token + ConPTY, or only PowerShell 5.1?**
   - What we know: The original debug session showed `cmd.exe` ALSO failed 0xC0000142 under WRITE_RESTRICTED + ConPTY (debug log 2026-05-07T20:00Z). The CLR-DllMain framing was incorrect; the trigger is broader than CLR.
   - What's unclear: Whether cmd.exe under Low-IL + ConPTY launches cleanly (different DLL load chain than PowerShell 5.1).
   - Recommendation: Wave 1 field test should run BOTH `nono shell --profile claude-code --allow-cwd` (default = PowerShell 5.1) AND `nono shell --profile claude-code --allow-cwd --shell C:\Windows\System32\cmd.exe`. If cmd.exe also fails, both fail. If only PowerShell fails, the issue is shell-binary-specific and a different sixth option emerges (e.g., default cmd.exe on Windows). Same field-test infrastructure either way.

4. **What's the "field test passed" gate criterion? Does TUI rendering have to run a specific Claude Code workflow, or does shell prompt + claude `--version` count?**
   - What we know: D-05 / Acc #2 requires "claude runs inside the sandboxed shell with full TUI rendering (alternate screen buffer, cursor positioning, raw-mode input)."
   - What's unclear: Does the user need to demonstrate a multi-turn Claude conversation, or is a clean TUI launch (alternate-screen alloc + initial render) sufficient?
   - Recommendation: Plan-phase clarifies. Suggested baseline: launch `claude` inside `nono shell`, observe the alternate-screen TUI (logo + chat input box), type one message, observe response render, type `/quit`, type `exit` from the shell. If any of those four user interactions fail or render incorrectly, fail Acceptance #2.

## Risks & Landmines (CONTEXT.md flagged + new ones surfaced by research)

### CONTEXT.md flagged

- **Capability-pipe SDDL admits Low-IL primary token clients?** RESOLVED — YES, via OW match. Documented in Question 1 above. **Field-test verifies in the wild.**
- **9 leaked Low-IL labels** — out of scope per D-09; expected warnings; not blockers.
- **PreToolUse hook didn't fire** — out of scope per D-08; tracked separately.

### New, surfaced by research

- **Microsoft-documented ConPTY + integrity-mismatch failure mode (Pitfall 2)** — class of failure that nono's scenario fits. Wave 1 field test must specifically test interactive typing, not just shell launch+exit. If this bites, Wave 2 ProcMon investigation should specifically trace `\Device\ConDrv\Server` and `\Device\ConDrv\Connect` ALPC port DACLs.
- **Mandatory-label NO_WRITE_UP only enforces OUTSIDE the grant set, not inside** (Pitfall 3). Cookbook security-envelope text must be precise.
- **`create_low_integrity_primary_token` is structurally first-live-use under Wave 1** (Question 6, Question 8). Treat as new feature; field test is non-optional; unit tests for the function are mandatory.
- **OwnedHandle UAF discipline** (Pitfall 1) — already known but worth re-flagging because Wave 1 introduces a NEW holder location in the cascade.

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — Code under change is fully read, all callsites identified.
- Architecture (token cascade + capability pipe): HIGH — every relevant SDDL component decoded; MIC algorithm sourced from Microsoft Open Specification.
- Pitfalls: MEDIUM-HIGH — 5 distinct landmines identified with verified sources; Pitfall 2 (ConPTY+IL mismatch) is documented for a slightly different scenario, hence MEDIUM in our case.
- Wave 2 plan: LOW by definition — exploratory; success metric is "surface a sixth option," which can't be predicted statically.

**Research date:** 2026-05-07
**Valid until:** 2026-06-07 (30 days for stable Win32 mechanisms; Microsoft docs unlikely to shift). Re-validate if Wave 1 starts after that date.
