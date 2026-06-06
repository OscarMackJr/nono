# Windows Session Lifecycle Hook Executor

**Status:** Accepted
**Date:** 2026-06-05
**Phase:** 58 (session-lifecycle-hooks)
**Decision IDs:** D-05, D-06, D-07, D-08, D-09, D-10
**Related ADR:** [broker-trust-anchor.md](../../docs/architecture/broker-trust-anchor.md)
  (Phase 32 — establishes `LowIlPrimary` arm and its trust invariants)

## Context

Upstream commit `daa55c8` ("feat: session lifecycle hooks (#954)") adds a `session_hooks` profile
field that lets users configure scripts to run before and after the sandboxed child process. The
upstream design runs hooks with the host user's full Medium-IL token, outside the sandbox, with
fail-open error handling: if a hook fails (non-zero exit, timeout, validation error), upstream logs
a warning and continues session startup.

This fork diverges from upstream in two ways:

**1. Fail-policy (fork invariant, D-01/D-02):** The fork is fail-closed. Hook failure prevents
session start (before-hooks) or produces a non-zero supervisor exit (after-hooks). This matches the
fork's broader security posture: ambiguous states produce explicit errors, not degraded success.
This fork invariant is documented here as the canonical record per D-02.

**2. Windows execution model (net-new fork work, D-05..D-10):** Upstream has no Windows hook
executor — its `hook_runtime.rs` is gated `cfg(unix)`. The fork adds a complete Windows hook
executor in `hook_runtime_windows.rs`. The Windows implementation differs fundamentally from the
Unix side: instead of running hooks with the host user's full token, it confines hooks to Low
integrity level (IL) via the `LowIlPrimary` arm — the same mandatory-label enforcement that the
fork uses for the main sandboxed child.

The key architectural problem is that the Windows hook runs outside the AppContainer/Job Object
boundary of the sandboxed child, yet must not run with the full user token. A hook that writes
`PATH=<attacker-controlled>` to the env file would cause the Medium-IL parent to inject that
hijacked PATH into the child's environment (Low-IL-writer → Medium-IL-reader trust gap). The
decisions below close this gap structurally.

The choice of `LowIlPrimary` (NOT `WriteRestricted`) for hook execution is grounded in Phase 60
evidence: the .NET/PowerShell CLR fails to start under `WriteRestricted` tokens due to
`BaseNamedObjects` kernel object access failures (`STATUS_DLL_INIT_FAILED` / `0xC0000142`).
Since hooks are most naturally written as PowerShell scripts (`.ps1`), `WriteRestricted` would
make hooks non-functional for the most common use case. `LowIlPrimary` does not have this
restriction.

## Goals

This ADR records the following locked decisions for the Windows hook executor:

- **D-05:** Windows hooks execute via `LowIlPrimary` arm (`nono::create_low_integrity_primary_token()`),
  NOT `WriteRestricted`. The `LowIlPrimary` arm enforces Low-IL confinement via mandatory-label
  `NO_WRITE_UP`, which prevents write-up attacks via MIC pre-DACL kernel checks. `WriteRestricted`
  is explicitly forbidden for hook processes.

  **KNOWN LIMITATION (Research Open Question 1 — deferred follow-up):** As of Phase 58, hooks
  actually run at the **parent's (Medium-IL) integrity level**, not at Low-IL. The
  `nono::create_low_integrity_primary_token()` call is present but its token is **not plumbed
  into the spawn** because stable Rust's `std::process::Command` provides no custom-token API.
  Full `CreateProcessAsUserW` Low-IL plumbing requires raw FFI and is deferred. The Job Object
  provides process-tree containment (CPU/memory/handle-inheritance scope) but does NOT substitute
  for MIC Low-IL enforcement on filesystem/registry access. The `hook_runtime_windows.rs` module
  doc and the `run_hook_windows` inline comments accurately reflect this current state.

- **D-06:** Hook filesystem scope is minimal: session-dir write + cwd write + read on the script
  path only. The hook process cannot write to arbitrary paths outside its designated scope. This
  is the Windows analog of upstream's Unix `EnvFileGuard` RAII pattern, extended with mandatory-label
  enforcement.

- **D-07:** The env-export mechanism is ported to Windows. Before-hooks can write `KEY=VALUE` entries
  to the session-dir env file (`NONO_ENV_FILE`). The Medium-IL parent reads this file after the hook
  exits and injects the filtered pairs into the child's environment. The mechanism is structurally
  identical to upstream's Unix design.

- **D-08:** Windows env-file creation uses `CREATE_NEW` disposition (`OpenOptions::create_new(true)`,
  the Windows equivalent of Unix `O_EXCL`) plus a Low-IL mandatory label (mask `0x5` =
  `NO_WRITE_UP | NO_EXECUTE_UP`). `CREATE_NEW` prevents a pre-created env file from being injected
  before the hook runs. The mandatory label restricts access to the Low-IL hook process (writer) and
  the Medium-IL parent (reader).

- **D-09:** The parent applies `is_dangerous_env_var()` to all env-file entries before injecting
  them into the child's environment. The function is extended with 10 Windows-specific env vars
  that represent the Low-IL-writer → Medium-IL-reader injection vector: `PATH`, `PATHEXT`,
  `COMSPEC`, `PSModulePath`, `PSModuleAnalysisCachePath`, `__PSLockdownPolicy`, `SystemRoot`,
  `windir`, `TEMP`, and `TMP`. All comparisons use `eq_ignore_ascii_case` (Windows env vars are
  case-insensitive).

- **D-10:** `validate_hook_script_windows` performs full security validation before every hook
  execution: absolute path check (`Path::is_absolute()`, NOT string `starts_with`), canonical
  path resolution (`std::fs::canonicalize` — adds `\\?\` prefix on Windows), regular file check,
  owner check (`nono::path_is_owned_by_current_user`), effective-rights ACL check on both the
  file AND its parent directory (DACL enumeration with `Everyone` SID `S-1-1-0` via
  `GetNamedSecurityInfoW` + `GetAce` + `EqualSid`), and mandatory-label consistency check.
  This check is UNCONDITIONAL — there is no fallback path or profile-level bypass.

## Non-goals

- **Upstream fail-open behavior is NOT adopted.** Upstream warns and continues on hook failure.
  This fork returns `Err` and aborts (D-01). There is no profile-level toggle for fail-open
  behavior (considered and rejected in the Alternatives section below).

- **No host-trusted hook execution (target state).** The design goal (D-05) is that hooks are
  NOT run with the user's full Medium-IL token (upstream's design). The intended runtime is
  Low-IL via `LowIlPrimary`. However, as of Phase 58, full Low-IL spawn is deferred (see D-05
  KNOWN LIMITATION above) — hooks currently run at Medium-IL inside a Job Object. The motivation
  for the eventual Low-IL enforcement remains: least-privilege confinement so a compromised or
  malicious hook has reduced ability to affect the rest of the system.

- **No inline script execution.** Hook scripts are always referenced by path (argument to
  `-File`, argument to `/C`), never as inline code. This preserves upstream's no-JSON-injection
  rule: a profile that sets `script: "Write-Host hello"` (inline code) will fail validation
  because the path does not exist on disk.

- **No shell-association lookup.** The interpreter for `.ps1` and `.cmd`/`.bat` files is
  determined by the file extension only, using explicit dispatcher logic. Windows registry
  lookups (`HKEY_CLASSES_ROOT\.ps1` shell association) are not used — they are attacker-
  influenceable via registry manipulation.

## Decision Table

| Decision | Options Considered | Choice | Rationale |
|----------|--------------------|--------|-----------|
| **Token arm for hook spawn (D-05)** | `LowIlPrimary` vs `WriteRestricted` vs `BrokerLaunchNoPty` vs full user token | `LowIlPrimary` (target; **deferred** — see KNOWN LIMITATION) | `WriteRestricted` causes `STATUS_DLL_INIT_FAILED` (0xC0000142) for `powershell.exe` due to `BaseNamedObjects` kernel object access failure — proven in Phase 60. `BrokerLaunchNoPty` is for the main sandboxed child (long-lived, broker-mediated); hooks are short-lived and do not need PTY or broker mediation. Full user token provides no confinement. `LowIlPrimary` is the correct target arm for short-lived, non-PTY Low-IL processes, but plumbing it via `CreateProcessAsUserW` raw FFI is deferred as Research Open Question 1. **As of Phase 58, hooks spawn with the parent's token (Medium-IL) inside a Job Object.** |
| **Env-file integrity model (D-08)** | Low-IL mandatory label alone vs DACL-narrowing to hook token SID vs both | Low-IL mandatory label (primary gate); DACL narrowing deferred as V2 | The mandatory label (mask `0x5`) is the primary access control gate: it prevents Medium-IL+ processes other than the parent from being confused about the file's security relevance, and it restricts write-up attacks. DACL narrowing to the hook's specific token SID would require the hook to know its own SID at env-file creation time — which is before the hook spawn. Deferred as defense-in-depth improvement for a future phase. The label alone is sufficient for the V1 trust model. |
| **Fail policy divergence (D-01)** | Fail-closed (Err) vs fail-open (warn+Ok) vs profile-configurable | Fail-closed (unconditional) | The fork's security model is fail-closed throughout. Hooks are security-relevant side-effects (they can inject env vars into the child). A failing hook that is silently skipped creates a false assumption about the child's execution context. The profile-level toggle option was explicitly rejected: it creates a footgun where operators disable fail-closed for convenience and then forget to re-enable it. |
| **World-writable ACL check approach (D-10)** | `GetEffectiveRightsFromAclW` vs DACL enumeration with `EqualSid` | DACL enumeration (`GetNamedSecurityInfoW` + `GetAce` + `EqualSid`) | `GetEffectiveRightsFromAclW` was tried and dropped in the fork (see `260522-wn0` debug session): it walks group memberships from the full token but `SetNamedSecurityInfoW(LABEL_*)` runs under the UAC-filtered token, causing false positives for local admins. DACL enumeration with `EqualSid` directly checks whether the `Everyone` SID (`S-1-1-0`) has a write-class ACE, which is the exact threat we are mitigating (T-58-03-03). |

## Trust Boundary

The primary trust boundary in the Windows hook executor is the **Low-IL-writer → Medium-IL-reader
env-file gap**:

1. The hook process runs at **Medium-IL** (parent's token — D-05 Low-IL spawn is deferred;
   see D-05 KNOWN LIMITATION above). It writes `KEY=VALUE` entries to the session-dir env
   file via the `NONO_ENV_FILE` path it receives from the Medium-IL parent.

2. The Medium-IL parent reads this file after the hook exits and injects the pairs into the child's
   environment. At this point, the parent has elevated the hook's output from Low-IL context to
   Medium-IL context.

3. If a malicious or compromised hook writes dangerous env vars to the env file, the parent could
   inadvertently grant the attacker execution-context control over the child.

**Mitigations in layered defense:**

- **CREATE_NEW (D-08):** The parent creates the env file with `CREATE_NEW` before spawning the hook.
  A pre-created env file (e.g., a symlink pointing to attacker-controlled content) causes the create
  to fail, preventing the hook from being spawned with a pre-poisoned env file.

- **Low-IL mandatory label (D-08):** Mask `0x5` (`NO_WRITE_UP | NO_EXECUTE_UP`) on the env file.
  The Low-IL hook can write to it (write-down is permitted under MIC). The Medium-IL parent can
  read it. Other Medium-IL or Higher-IL processes that do NOT hold `READ_CONTROL` on the file's
  SACL cannot access it in attacker-controlled ways. This reduces the attack surface for other
  Low-IL processes reading the env file.

- **is_dangerous_env_var() filter (D-09):** The parent filters ALL env-file entries through
  `is_dangerous_env_var()` before injection. The 10 Windows-specific danger vars are:

  | Var | Attack vector |
  |-----|---------------|
  | `PATH` | Executable resolution hijacking — a hook writing `PATH=<attacker-controlled>` causes the parent to resolve the wrong binary when launching the child |
  | `PATHEXT` | Extension association hijacking — controls which file extensions are treated as executables |
  | `COMSPEC` | cmd interpreter redirect — `cmd.exe`-based invocations use this to find the interpreter |
  | `PSModulePath` | PowerShell module injection — all `Import-Module` calls search this path |
  | `PSModuleAnalysisCachePath` | PS analysis cache poisoning — PowerShell's module analysis cache |
  | `__PSLockdownPolicy` | PS constrained-language bypass — setting this can disable PowerShell lockdown mode |
  | `SystemRoot` | System DLL resolution redirect — `%SystemRoot%\system32` is the Windows system directory |
  | `windir` | System directory redirect — alias for `SystemRoot` in many contexts |
  | `TEMP` | Temp file redirect — from the parent's perspective, redirecting TEMP affects where the parent writes temporary files |
  | `TMP` | Same as TEMP |

- **Validate script before every execution (D-10):** All 7 validation checks run unconditionally
  before every hook spawn. The effective-rights ACL check on BOTH the script file AND its parent
  directory prevents scripts placed in world-writable directories even if the file itself has tight
  permissions.

- **RAII env-file cleanup:** `WindowsEnvFileGuard::drop()` zero-fills the file contents and then
  removes it. This limits the window during which the env-file's contents are readable by other
  processes, even if a crash or panic delays cleanup.

## Invariants

The following invariants MUST be preserved in all future modifications to `hook_runtime_windows.rs`:

1. **Mandatory-label enforcement:** The env file MUST carry a Low-IL mandatory label (mask `0x5`)
   created via `nono::try_set_mandatory_label`. Removing the label apply reduces the trust boundary
   to DACL-only (which is absent in V1).

2. **No host-trusted hook execution (target state):** The intended design (D-05) is that hooks
   run via the `LowIlPrimary` arm. `WriteRestricted` is FORBIDDEN (CLR startup failure).
   `BrokerLaunchNoPty` is inappropriate for short-lived hook processes. As of Phase 58, the
   full Low-IL spawn via `CreateProcessAsUserW` is deferred (Research Open Question 1); hooks
   run at Medium-IL inside a Job Object. See the D-05 KNOWN LIMITATION note in Goals above.

3. **Session-dir + cwd-only scope (D-06):** The hook process's filesystem capability grant MUST
   be restricted to session-dir write + cwd write + read on the script path. No broader grants
   are permitted. This is the Windows analog of Unix's `EnvFileGuard` RAII isolation.

4. **Script-file references only:** Hook commands MUST reference the script by path (as an argument
   to `-File`, `/C`, or as the executable name). Inline script execution is NEVER permitted. This
   preserves the no-JSON-injection rule from upstream.

5. **Fail-closed divergence (D-01):** Any hook failure (non-zero exit, timeout, validation error)
   MUST return `Err`. There is no profile-level toggle. There is no warn-and-continue path.

6. **LowIlPrimary arm only (D-05):** `WriteRestricted` is EXPLICITLY FORBIDDEN for hook execution.
   Any future refactor that routes hooks through `WriteRestricted` will cause PowerShell hooks
   to fail with `0xC0000142`.

7. **Unconditional ACL check (D-10):** `validate_hook_script_windows` MUST call
   `check_no_world_writable_acl` on BOTH the canonical script path AND its parent directory.
   There is NO fallback path, NO profile-level bypass, NO conditional skip. D-10 is a locked,
   unconditional security requirement. If the ACL query fails, the function MUST return `Err`
   (fail-closed per D-01), not silently proceed.

## Fork Divergence Record (D-02 Requirement)

This ADR is the canonical record of the fail-closed divergence from upstream commit `daa55c8`, as
required by decision D-02 in `.planning/phases/58-session-lifecycle-hooks/58-CONTEXT.md`.

Upstream `daa55c8` is fail-open: `execute_before_hook` returns `Ok(Vec::new())` on non-zero exit
and on timeout, and `execute_after_hook` returns `Ok(())` on non-zero exit and timeout. Both
functions emit `warn!()` log lines. This behavior was validated by the upstream test
`test_execute_before_hook_fail_open` (which asserts `Ok(vars)` on exit 1).

The fork overrides this behavior:
- `execute_before_hook` returns `Err(NonoError::ConfigParse(...))` on non-zero exit or timeout.
  The error message cites "fail-closed" so operators can distinguish fork behavior from a bug.
- `execute_after_hook` returns `Err(NonoError::ConfigParse(...))` on non-zero exit or timeout.
- The upstream test `test_execute_before_hook_fail_open` is replaced by
  `test_execute_before_hook_fail_closed` in `hook_runtime.rs`, which asserts `Err(_)`.

**The upstream runtime MECHANISM is preserved exactly (D-02 SC2):** script validation, env-file
pattern, timeout + process kill, env-var filtering. Only the fail-policy is hardened.

**This record is permanent and must not be removed.** If a future upstream version changes the
fail policy (e.g., introduces a configurable error mode), this ADR must be updated to reflect
the new upstream behavior and the fork's corresponding decision.

## Alternatives Considered

### WriteRestricted token arm (rejected for D-05)

**Proposal:** Run hooks using the `WriteRestricted` token arm (same as the non-PTY supervised
path for the main sandboxed child on profiles without `windows_low_il_broker`).

**Rejection reason:** The .NET/PowerShell CLR (`clr.dll`) fails to initialize under `WriteRestricted`
tokens. The CLR startup sequence accesses `BaseNamedObjects` kernel objects that require
non-restricted SIDs in the token's restricting-SID list. Since hooks are most naturally written
as PowerShell scripts (`.ps1`), `WriteRestricted` would render them non-functional for the
most common use case. This failure was proven in Phase 60 (debugging `lowil-cwd-write-denied`)
and is the primary motivation for D-05.

### BrokerLaunchNoPty for hook execution (rejected for D-05)

**Proposal:** Route hooks through `nono-shell-broker.exe` using the `BrokerLaunchNoPty` arm
(Phase 51 D-06 design for the non-PTY supervised main-child path).

**Rejection reason:** Hooks are short-lived, non-PTY processes that do not need broker mediation.
`BrokerLaunchNoPty` is designed for the main sandboxed child (persistent session, needs the
broker trust anchor, no ConPTY). Routing hooks through the broker introduces broker binary
dependency at hook runtime, Authenticode verification overhead on every hook spawn, and
unnecessary complexity. `LowIlPrimary` direct spawn is appropriate for short-lived hook scripts.

### Profile-level fail-open toggle (rejected for D-01)

**Proposal:** Add a `session_hooks.fail_open: bool` field to the profile schema, allowing users
to opt into upstream-compatible fail-open behavior.

**Rejection reason:** Fail-open is categorically insecure for hooks that can inject env vars into
the sandboxed child. A compromised hook that exits 1 while writing malicious env vars to the env
file would succeed in injecting those vars under fail-open mode. The toggle creates a footgun
where operators disable fail-closed for convenience and then forget to re-enable it. The fork's
security model is fail-closed throughout. Deferred as a "v2 idea" without specific timeline.

### Skipping effective-rights ACL check when no existing helper available (rejected for D-10)

**Proposal:** If `GetEffectiveRightsFromAclW` or a similar helper is not available in the
codebase, accept the limitation and note it in the SUMMARY without implementing the check.

**Rejection reason:** D-10 is a locked, unconditional requirement. The effective-rights ACL check
on BOTH the file and its parent directory is a core security guarantee of the Windows hook
executor. If no existing helper provides this capability, it must be implemented inline. The
implementation in `check_no_world_writable_acl` uses `GetNamedSecurityInfoW` + `GetAce` + `EqualSid`
to enumerate DACL entries and check for `Everyone` (`S-1-1-0`) write-class ACEs — a direct and
reliable approach that avoids the `GetEffectiveRightsFromAclW` false-positive issue documented in
the `260522-wn0` debug session.

### `GetEffectiveRightsFromAclW` for world-writable check (rejected, replaced by DACL enumeration)

**Proposal:** Use `GetEffectiveRightsFromAclW` with the `Everyone` SID as the trustee to compute
effective access rights.

**Rejection reason:** This approach was tried and dropped in a prior debug session (`260522-wn0`):
`GetEffectiveRightsFromAclW` walks group memberships from the full impersonation token, but the
`SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)` call that follows runs under the UAC-filtered
token. This mismatch causes false positives for local admins — the pre-flight check concludes the
user has `WRITE_OWNER` but the apply fails with `ERROR_ACCESS_DENIED`. DACL enumeration with
`EqualSid` is a more direct and reliable check for the specific threat (Everyone write-class ACE).

---

**File path note:** This ADR lives at `.planning/architecture/adr-58-windows-hook-executor.md`
per D-46-A2 precedent (`.planning/architecture/` is the v2.6+ ADR location; `docs/architecture/`
holds Phase 32 and earlier ADRs). The `broker-trust-anchor.md` related ADR in `docs/architecture/`
is linked above for cross-reference.
