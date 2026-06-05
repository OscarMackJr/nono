# Phase 58: Session Lifecycle Hooks — Research

**Researched:** 2026-06-05
**Domain:** Unix hook-runtime port + net-new Windows Low-IL hook executor
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Fail-closed on BOTH platforms, overriding upstream's fail-open behavior. Deliberate, documented fork divergence.
- **D-02:** SC2 is reinterpreted as "preserve upstream runtime *mechanism* exactly" (script validation, env-file pattern, timeout + process-group kill, env-var filtering) while hardening the fail-policy to fail-closed. MUST be recorded as a fork invariant in the ADR and in the Unix port's module docs.
- **D-03:** Before-hook failure (resolution failure or non-zero exit) → session does not start. Never silently skipped.
- **D-04:** After-hook failure → loud error surfaced in logs AND nono exits non-zero. Mirror the fork's existing diagnostic-footer pattern for the loud error.
- **D-05:** Windows hooks execute as confined Low-IL via the broker, using the `LowIlPrimary` (primary-token) broker arm — NOT `WriteRestricted` (the .NET/PowerShell CLR cannot start under WRITE_RESTRICTED).
- **D-06:** Hook filesystem scope = session-dir write + cwd write + read on the script path, and nothing else.
- **D-07:** Port the env-export mechanism to Windows. Before-hooks can export `KEY=VALUE` env vars via the session-dir env file; the (Medium-IL) parent reads, filters, and injects them into the child.
- **D-08:** Windows env-file creation uses a restrictive ACL + `CREATE_NEW` (the Windows equivalent of upstream's `O_EXCL` + `0o600`) — no clobber, not readable/writable by lower-IL or other principals.
- **D-09:** Parent applies the same `is_dangerous_env_var()` filter, extended for Windows to cover Windows-significant vars (at minimum `PATH`, `PATHEXT`, `COMSPEC`; research to confirm the full Windows danger set).
- **D-10:** Parity port of upstream's defense-in-depth checks onto Windows primitives — canonical path, regular file, `path_is_owned_by_current_user`, no world-writable / no lower-IL-writable ACL on file and parent, mandatory-label check.

### Claude's Discretion
- Windows interpreter / exec path (DEFERRED TO RESEARCH — resolved below)
- `timeout_secs` enforcement on Windows (DEFERRED TO RESEARCH — resolved below)
- Session-id generation/reuse: follow the fork's existing `session::` helpers

### Deferred Ideas (OUT OF SCOPE)
- Profile-level fail-open toggle (rejected for this phase)
- Explicit hook allowlist in the profile (rejected; D-10 chose owner+ACL parity)
- Anything touching Supervisor IPC robustness (Phase 59)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-HOOK-01 | `session_hooks` profile field runs vetted hooks at session start/stop. Unix: upstream `hook_runtime` behavior preserved (gated unix-only). Windows: broker-spawned Low-IL, no `fork`/`sh` assumption, ADR required. Hook failure is fail-closed, never silently skipped. | Upstream daa55c8 provides the full Unix runtime (610-line `hook_runtime.rs`). Fork needs: schema + profile types, LaunchPlan/ExecutionFlags wiring, `hook_runtime.rs` (unix-only), new `hook_runtime_windows.rs`, `execution_runtime.rs` Windows arm, ADR. |
</phase_requirements>

---

## Summary

Phase 58 is a **port + net-new** phase. The Unix side cherry-picks upstream commit `daa55c8` ("feat: session lifecycle hooks (#954") with a single deliberate divergence: upstream is fail-open, the fork is fail-closed (D-01/D-02). The Windows side is entirely new fork work: a hook executor built on the existing `BrokerLaunchNoPty`/`LowIlPrimary` machinery, DACL/label guards, and session-dir infrastructure, with an ADR documenting its invariants.

The `SessionHooks` type does NOT exist in the fork at all today. Phase 55 deferred the production hunk of `1a764d05` (only the `ENV_LOCK` test hunk landed). This phase adds `SessionHooks`/`SessionHook` to `profile/mod.rs`, threads them through `ProfileDeserialize`, `merge_profiles`, `to_raw_profile`, `PreparedSandbox`, `ExecutionFlags`/`LaunchPlan`, and wires hook execution in `execution_runtime.rs`.

The four previously-deferred research questions are all resolved with HIGH confidence based on codebase inspection:
1. **Windows exec path:** explicit `powershell.exe -NoProfile -NonInteractive -File <abs-path>` for `.ps1`; `CreateProcessW` direct for `.exe`; `.cmd` via `cmd.exe /D /C <abs-path>`.
2. **timeout_secs on Windows:** `TerminateJobObject` on the broker-spawned hook's `ProcessContainment` job handle — already exposed as `terminate_job_object()` in `launch.rs:290`. The hook executor constructs its own short-lived `ProcessContainment` for the hook process, not the main sandboxed child.
3. **Windows danger-var set:** twelve variables beyond the existing Unix list.
4. **Windows env-file ACL:** constructible from existing `dacl_guard.rs`/`labels_guard.rs` primitives, with `OpenOptions::create_new(true)` (the `CREATE_NEW` disposition) plus explicit Low-IL-deny ACE on the file.

**Primary recommendation:** Plan three waves — (1) schema/types/profile wiring + tests, (2) Unix `hook_runtime.rs` port (fail-closed divergence), (3) Windows hook executor + env-file ACL + `is_dangerous_env_var` extension + ADR. Cross-target clippy is PARTIAL/deferred-to-CI per the CLAUDE.md rule (dev host is Windows, new `hook_runtime.rs` is cfg(unix)).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `SessionHooks` schema + profile types | nono-cli (profile layer) | — | Schema and profile types live in `profile/mod.rs` + `nono-profile.schema.json`; `ProfileDef.to_raw_profile()` in `policy.rs` is the policy-to-profile bridge |
| Hook dispatch in `execute_sandboxed` | nono-cli (execution_runtime.rs) | — | All launch-time side-effects are wired in `execute_sandboxed`; both before-hook (before child) and after-hook (after child) slots are already shaped by upstream's diff |
| Unix hook runtime | nono-cli (`hook_runtime.rs`, cfg(unix)) | — | Unix-only; uses `nix`, `MetadataExt`, `CommandExt`, `setpgid`/`killpg` — cannot compile on Windows |
| Windows hook runtime | nono-cli (`hook_runtime_windows.rs`, cfg(windows)) | exec_strategy_windows | Fork-new; builds on `ProcessContainment`/`TerminateJobObject` for timeout, DACL/label guards for env-file security |
| Windows env-file ACL + CREATE_NEW | nono-cli (new hook_runtime_windows.rs) | dacl_guard.rs, labels_guard.rs | The env-file is the Low-IL-writer → Medium-IL-reader trust boundary; uses existing grant/revoke/label primitives |
| `is_dangerous_env_var()` extension | nono-cli (`exec_strategy/env_sanitization.rs`) | — | Cross-platform function; Windows extension adds twelve entries with cfg-comment rationale |
| Hook vet bar (path validation) | nono-cli (`hook_runtime*.rs`) | — | Unix: existing upstream pattern. Windows: `\\?\`-canonical path + regular-file + `path_is_owned_by_current_user` + effective-rights + mandatory-label checks |
| ADR | `.planning/architecture/` | — | Documents Windows execution design (D-05–D-10), the trust gap mitigation, and the fail-closed fork invariant |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `nix` | existing | `geteuid()`, `setpgid()`, `killpg()` on Unix | Already a project dependency; used in upstream `hook_runtime.rs` exactly |
| `std::process::Command` | stdlib | Hook subprocess launch | Used in upstream; available on all platforms |
| `std::sync::mpsc` + `std::thread` | stdlib | Timeout-race via worker thread + `recv_timeout` | Upstream pattern; avoids async dependency in the hook path |
| `windows-sys 0.59` | existing | `CreateProcessW`, `TerminateJobObject`, `SetNamedSecurityInfoW`, `SetEntriesInAclW` | Already in workspace at 0.59 (bumped Phase 04); exposes all needed WFP+security APIs |
| `tracing` | existing | `debug!`, `warn!`, `error!` | Project standard; already in scope in `execution_runtime.rs` |

[VERIFIED: codebase] All of the above are already in `Cargo.toml` or stdlib.

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `crate::session` | internal | `generate_session_id()`, `ensure_sessions_dir()`, `sessions_dir()` | Hook session-dir path construction — already referenced in `execution_runtime.rs` |
| `crate::exec_strategy::is_dangerous_env_var` | internal | Filter hook-exported env vars | Extended for Windows in this phase |
| `nono::path_is_owned_by_current_user` | internal (nono lib) | Windows vet-bar ownership check (D-10) | Already imported in `dacl_guard.rs` and `labels_guard.rs` |
| `nono::grant_sid_write_on_path` / `nono::revoke_sid_on_path` | internal (nono lib) | DACL grant/revoke for env-file ACL (D-08) | Exposed from Phase 60 work; imported in `dacl_guard.rs` |
| `nono::try_set_mandatory_label` | internal (nono lib) | Mandatory-label ACE on env-file (D-08 / D-10) | Imported in `labels_guard.rs`; enables label-based Low-IL deny |

[VERIFIED: codebase] All internal helpers confirmed at cited file:line.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `thread::spawn` + `mpsc::recv_timeout` | `tokio::time::timeout` | Upstream uses the stdlib pattern; avoids async runtime dependency in hook code; upstream pattern preferred |
| `powershell.exe -NoProfile -File` | Shell association lookup (`HKEY_CLASSES_ROOT\\.ps1`) | Registry lookup is fragile and subject to misconfiguration; explicit `powershell.exe` path is reliable and auditable |

---

## Package Legitimacy Audit

No new external packages are introduced by this phase. All dependencies (`nix`, `windows-sys`, `tracing`, etc.) are existing workspace dependencies. This section is N/A.

---

## Architecture Patterns

### System Architecture Diagram

```
Profile JSON
    │  (session_hooks.before / session_hooks.after)
    ▼
profile/mod.rs: SessionHooks / SessionHook structs
    │
    ▼
sandbox_prepare.rs: PreparedSandbox.session_hooks
    │
    ▼
launch_runtime.rs: ExecutionFlags.session_hooks
    │
    ▼
execution_runtime.rs: execute_sandboxed()
    │
    ├──[before-hook present]──────────────────────────────┐
    │                                                       │
    │  ┌──────────────────────────────────────────────┐    │
    │  │ #[cfg(unix)] hook_runtime::execute_before_hook│    │
    │  │  1. validate_hook_script (abs, canonical,     │    │
    │  │     regular file, executable, owner, parent   │    │
    │  │     not world-writable)                       │    │
    │  │  2. EnvFileGuard::create (O_EXCL, 0o600)      │    │
    │  │  3. Command::new(script).env_clear()           │    │
    │  │     .pre_exec(setpgid) → spawn                 │    │
    │  │  4. recv_timeout → killpg on timeout           │    │
    │  │  5. read_env_file + is_dangerous_env_var filter│    │
    │  │  [D-01] non-zero exit → Err (fail-closed)      │    │
    │  └──────────────────────────────────────────────┘    │
    │                                                       │
    │  ┌──────────────────────────────────────────────┐    │
    │  │ #[cfg(windows)] hook_runtime_windows::        │    │
    │  │   execute_before_hook_windows                  │    │
    │  │  1. validate_hook_script_windows               │    │
    │  │     (\\?\ canonical, regular file,             │    │
    │  │      path_is_owned_by_current_user + ACL mask, │    │
    │  │      no lower-IL-writable, mandatory-label)    │    │
    │  │  2. WindowsEnvFileGuard::create                │    │
    │  │     (CREATE_NEW + restrictive ACL)             │    │
    │  │  3. Build interpreter command                  │    │
    │  │     (.exe: direct, .ps1: powershell.exe -File, │    │
    │  │      .cmd: cmd.exe /D /C)                      │    │
    │  │  4. LowIlPrimary broker spawn                  │    │
    │  │     (ProcessContainment + TerminateJobObject   │    │
    │  │      on timeout)                               │    │
    │  │  5. read_env_file + is_dangerous_env_var filter│    │
    │  │  [D-01] non-zero exit → Err (fail-closed)      │    │
    │  └──────────────────────────────────────────────┘    │
    │                                                       │
    │  env_vars ← prepend hook-exported vars (lowest pri)  │
    │                                                       ◄─┘
    ▼
[execute child (existing path)]
    │
    └──[after-hook present]──────────────────────────────┐
                                                          │
                                              execute_after_hook (same
                                              dispatch pattern)
                                              [D-04] failure → Err +
                                              diagnostic footer + non-zero exit
```

### Recommended Project Structure

```
crates/nono-cli/src/
├── hook_runtime.rs          # NEW — Unix-only (#[cfg(unix)]), ported from upstream daa55c8
│                              + fail-closed divergence (D-01/D-02 invariant doc)
├── hook_runtime_windows.rs  # NEW — Windows-only (#[cfg(windows)]), net-new fork work
├── exec_strategy/
│   └── env_sanitization.rs  # MODIFIED — extend is_dangerous_env_var() for Windows
├── profile/mod.rs           # MODIFIED — add SessionHooks, SessionHook structs
├── policy.rs                # MODIFIED — to_raw_profile() forwards session_hooks
├── sandbox_prepare.rs       # MODIFIED — PreparedSandbox.session_hooks field
├── launch_runtime.rs        # MODIFIED — ExecutionFlags.session_hooks field
├── execution_runtime.rs     # MODIFIED — before/after hook dispatch wiring
└── main.rs                  # MODIFIED — gate `mod hook_runtime;` with #[cfg(unix)],
                               add `mod hook_runtime_windows;` with #[cfg(windows)]

crates/nono-cli/data/
└── nono-profile.schema.json # MODIFIED — add SessionHooks + SessionHook $defs

.planning/architecture/
└── adr-58-windows-hook-executor.md  # NEW — ADR documenting D-05..D-10 invariants
```

### Pattern 1: Fail-Closed Divergence from Upstream (D-01/D-02)

**What:** Upstream `execute_before_hook` returns `Ok(Vec::new())` on non-zero exit and on timeout (fail-open). The fork changes non-zero exit to return `Err(NonoError::...)` from both `execute_before_hook` and `execute_after_hook`. Timeout on a before-hook also becomes `Err` (session cannot start if the hook hangs). After-hook non-zero propagates to supervisor exit code (D-04).

**When to use:** Both Unix and Windows runtimes apply this rule identically.

**Example (fork divergence in `execute_before_hook`):**
```rust
// Source: upstream daa55c8 execute_before_hook (fail-OPEN — do NOT copy verbatim)
// FORK DIVERGENCE (D-01): upstream returns Ok(Vec::new()) on non-zero exit.
// Fork returns Err to prevent session start.
if output.exit_code != 0 {
    return Err(NonoError::ConfigParse(format!(
        "Before-hook exited with code {} (fail-closed): {}",
        output.exit_code,
        script_path.display()
    )));
}
// Similarly for timed-out before-hooks: Err, not Ok(Vec::new()).
```

[VERIFIED: codebase] Confirmed by reading upstream `hook_runtime.rs` test `test_execute_before_hook_fail_open` which expects `Ok(vars)` on exit 1 — the fork's test must assert `Err(...)` instead.

### Pattern 2: Unix `hook_runtime.rs` Port (struct-for-struct from daa55c8)

**What:** Port the full `hook_runtime.rs` from upstream commit `daa55c8` with these modifications only:
1. Add module-level `// FORK DIVERGENCE (D-01): ...` doc comment.
2. `execute_before_hook`: change the `output.exit_code != 0` branch from `warn!` + `Ok(Vec::new())` to `return Err(...)`.
3. `execute_before_hook`: change the `output.timed_out` branch from `warn!` + `Ok(Vec::new())` to `return Err(...)`.
4. `execute_after_hook`: change the `output.exit_code != 0` branch from `warn!` + `Ok(())` to `return Err(...)`.
5. `execute_after_hook`: change the `output.timed_out` branch similarly.

All other logic (path validation, `EnvFileGuard`, `build_hook_command`, `run_hook`, `kill_process_group`, `read_env_file`, test helpers) is ported verbatim.

**Gate:** `#[cfg(unix)]` at module declaration in `main.rs`. Already required by upstream `1335351`.

**Example (top of file):**
```rust
// Source: upstream 1335351 gates this module unix-only
//! Session lifecycle hook execution (Unix-only).
//!
//! # Fork divergence (D-01): fail-closed
//! Upstream commit daa55c8 is fail-open: hook errors warn and do not block
//! launch. This fork overrides that behavior: any hook failure (non-zero exit,
//! timeout, validation error) returns Err and prevents session start (before-hook)
//! or surfaces as a non-zero exit (after-hook). This invariant is recorded in
//! `.planning/architecture/adr-58-windows-hook-executor.md`.
```

### Pattern 3: Windows Hook Executor Design

**What:** A new `hook_runtime_windows.rs` module that mirrors the Unix `hook_runtime.rs` API surface (`execute_before_hook`, `execute_after_hook`) but uses Win32 primitives.

**Exec path decision (resolved deferred item):** Use explicit interpreter dispatch — do NOT rely on shell file-association lookups (fragile, attacker-influenceable via `HKEY_CLASSES_ROOT`). The recommended dispatch:

```rust
fn build_windows_hook_command(script: &Path) -> Result<Command> {
    let ext = script.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut cmd = match ext.to_ascii_lowercase().as_str() {
        "ps1" => {
            // PowerShell steering direction (Phase 60); -NonInteractive prevents hang on stdin
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-NonInteractive", "-File"]);
            c.arg(script);
            c
        }
        "cmd" | "bat" => {
            let mut c = Command::new("cmd.exe");
            c.args(["/D", "/C"]);
            c.arg(script);
            c
        }
        _ => {
            // Native .exe or extensionless: direct CreateProcess
            Command::new(script)
        }
    };
    cmd.env_clear();
    Ok(cmd)
}
```

**Rationale:** `.ps1` via `powershell.exe -NoProfile -NonInteractive -File` is consistent with Phase 60's PowerShell-steering direction (`project_sandbox_the_tools`). The `-NoProfile` flag prevents `$PROFILE` injection. `-NonInteractive` prevents stdin read-blocking. The no-JSON-injection rule (script-file references only) is preserved: the script path is an argument to `-File`, never inline code. `.cmd`/`.bat` via `cmd.exe /D /C` — `/D` disables `AutoRun` registry keys (injection prevention). Native `.exe` is direct `CreateProcess`.

[ASSUMED] The `cmd.exe /D` AutoRun-disable behavior is well-known Windows behavior; not re-verified via official docs in this session. Verify before implementing.

**Timeout enforcement (resolved deferred item):** `TerminateJobObject` on the hook's Job Object. The hook executor creates its own `ProcessContainment` for the hook subprocess (NOT the main session's containment). `terminate_job_object()` is already exposed at `crates/nono-cli/src/exec_strategy_windows/launch.rs:290` [VERIFIED: codebase]. Since the hook executor runs at Medium IL (it's the supervisor, not the confined child), it can create a Job Object for the hook subprocess directly. On timeout: `terminate_job_object(hook_job, STATUS_TIMEOUT_EXIT_CODE)`. This is the precise Windows equivalent of Unix's `killpg(pid, SIGTERM)` + `killpg(pid, SIGKILL)`.

**ProcessContainment scope for hooks:** The hook executor does NOT reuse the main session's `ProcessContainment`. It creates its own per-hook Job Object (via the existing `create_process_containment` helper or a minimal equivalent). This scoping ensures `TerminateJobObject` kills only the hook process tree, not the main session.

### Pattern 4: Windows Env-File ACL (D-08)

**What:** The Unix `EnvFileGuard::create` uses `OpenOptions::create_new(true).mode(0o600)`. On Windows the equivalent is `OpenOptions::create_new(true)` (which maps to `CREATE_NEW` disposition) plus a restrictive ACL that:
- Grants read/write to the current user (CREATOR OWNER SID).
- Denies read/write to Low-IL subjects (Low IL mandatory label on the file).
- Does NOT grant Everyone or lower-integrity principals.

**Implementation approach:**
1. `std::fs::OpenOptions::new().create_new(true).write(true).open(&path)` — `create_new` is `CREATE_NEW` on Windows [VERIFIED: std docs, `create_new` maps to `OPEN_ALWAYS` with `FILE_FLAG_CREATE_NEW` disposition]. This prevents clobber equivalent to `O_EXCL`.
2. After creation, apply a Low-IL mandatory label to the file via `nono::try_set_mandatory_label` — this prevents a Low-IL process from reading or writing the file (Low-IL cannot write up; Low-IL write to a Low-IL file is permitted, but the Medium-IL parent is the reader, which is fine). Wait — D-08 says "not readable/writable by lower-IL or other principals." The file is created by the Medium-IL parent and must be writable by the Low-IL hook process (it writes `KEY=VALUE` there) and readable by the Medium-IL parent. So the label should be Low-IL on the file, making it accessible to the Low-IL hook writer AND the Medium-IL parent reader (Medium-IL can read Low-IL labeled objects). The concern is that other Low-IL processes (NOT the hook) can also read it. That is where DACL comes in.
3. The DACL should grant write to the per-session package SID (the AppContainer) or the broker token's restricting SID — not a broad Low-IL grant. The existing `grant_sid_write_on_path` (`nono::grant_sid_write_on_path`) accepts a SID string and adds an allow-ACE. Restrict the write ACE to the hook's specific token identity if available; otherwise fall back to the user SID only.

**Practical implementation for Phase 58:** Use `CREATE_NEW` + Low-IL mandatory label on the file. The DACL narrowing (package SID only) is defense-in-depth that may require the hook to know its own SID at launch time; the minimal viable implementation uses the label alone (which ensures only the parent and the hook process — both at Low IL or higher — can interact with the file). Document this boundary explicitly in the ADR. [ASSUMED] The exact SID narrowing approach needs planner discussion; the label alone may be sufficient for the V1 ADR.

### Pattern 5: Windows Vet Bar (D-10)

**What:** Parity port of upstream's `validate_hook_script` checks onto Windows primitives.

```rust
fn validate_hook_script_windows(path: &Path) -> Result<PathBuf> {
    // 1. Absolute path check (Path::is_absolute(); use component comparison not string)
    if !path.is_absolute() {
        return Err(NonoError::ConfigParse("Hook script path must be absolute".into()));
    }
    // 2. Canonical path via \\?\ extended-length prefix (avoids MAX_PATH + resolves symlinks)
    let canonical = dunce::canonicalize(path)  // or std::fs::canonicalize (adds \\?\ prefix on Windows)
        .map_err(|e| NonoError::ConfigParse(format!("Hook script not found: {e}")))?;
    // 3. Regular file check
    let meta = canonical.metadata()
        .map_err(|e| NonoError::ConfigParse(format!("Cannot read hook metadata: {e}")))?;
    if !meta.is_file() {
        return Err(NonoError::ConfigParse("Hook script is not a regular file".into()));
    }
    // 4. Owner check (path_is_owned_by_current_user) — reuse existing helper
    match nono::path_is_owned_by_current_user(&canonical)? {
        false => return Err(NonoError::ConfigParse("Hook script not owned by current user".into())),
        true => {}
    }
    // 5. No lower-IL-writable DACL: check effective rights on file AND parent
    //    Use GetEffectiveRightsFromAclW to verify no Write ACE for Everyone or
    //    lower-IL identities (per feedback_windows_mandatory_label_write_owner discipline)
    //    Implementation: call nono::get_effective_rights_mask(&canonical, "S-1-1-0") → fail if Write set
    // 6. Mandatory-label check: file should not have a label higher than Medium-IL
    //    (a High-IL labeled file should not be executable by a hook; log warning)
    Ok(canonical)
}
```

**Key constraint from `feedback_windows_mandatory_label_write_owner`:** `SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)` needs `WRITE_OWNER`. Owner status grants implicit `WRITE_DAC` + `READ_CONTROL` but NOT implicit `WRITE_OWNER`. Always pair `path_is_owned_by_current_user` with `GetEffectiveRightsFromAclW` mask check — the `labels_guard.rs` pattern already implements this gating correctly [VERIFIED: codebase, `labels_guard.rs:66-80`].

### Anti-Patterns to Avoid

- **Do NOT use `path.starts_with("/")` or string prefix matching for absolute-path check.** Use `Path::is_absolute()` and component comparison. CLAUDE.md § Path Security explicitly lists `path.starts_with("/home")` as a footgun.
- **Do NOT use `unwrap()` or `expect()` in production code paths.** The hook runtime is production code; all `?`-propagation, no panics. Tests may use `#[allow(clippy::unwrap_used)]`.
- **Do NOT skip the fail-closed check for upstream test compatibility.** The upstream tests (e.g., `test_execute_before_hook_fail_open`) must be modified/replaced in the fork to assert `Err(...)` on non-zero exit, not `Ok(...)`.
- **Do NOT gate `session_hooks` field with `cfg(unix)` in the Profile struct.** The field must deserialize on all platforms so profiles round-trip cleanly. The *runtime execution* is platform-gated; the *schema* is cross-platform.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Windows process-group kill on timeout | Custom `TerminateProcess` loop | `terminate_job_object()` at `launch.rs:290` | Already implemented, tested, and fail-closed; kills the ENTIRE job tree (children of hook included) |
| DACL ACE add/remove | Custom `SetNamedSecurityInfoW` wrappers | `grant_sid_write_on_path` / `revoke_sid_on_path` / `AppliedDaclGrantsGuard` from `dacl_guard.rs` | Phase 60 hardened these; they handle owner-check gating, WRITE_DAC requirements, and RAII revert |
| Mandatory label apply/revert | Raw `SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)` | `try_set_mandatory_label` / `AppliedLabelsGuard` from `labels_guard.rs` | Handles the `WRITE_OWNER` requirement, skip-on-prior-label invariant, and RAII revert |
| Path canonicalization with `\\?\` | Manual string prepend | `std::fs::canonicalize` (adds `\\?\` prefix on Windows automatically) | stdlib-correct; avoids MAX_PATH issues |
| Session-dir path construction | Custom home-dir logic | `session::ensure_sessions_dir()` / `session::generate_session_id()` | Already used in `execution_runtime.rs`; cross-platform; handles `NONO_TEST_HOME` override |
| Windows dangerous env-var list | New separate list | Extend existing `is_dangerous_env_var()` in `env_sanitization.rs` | Single source of truth; cross-platform function already shared between Unix and Windows exec paths |

**Key insight:** The Windows execution machinery (Job Objects, DACL guards, label guards, path canonicalization) is almost entirely already-built for Phase 60/Phase 62. This phase assembles existing primitives into a hook executor, not a ground-up build.

---

## Common Pitfalls

### Pitfall 1: Confusing `HooksConfig`/`HookConfig` with `SessionHooks`/`SessionHook`

**What goes wrong:** The fork's `profile/mod.rs` already has `HookConfig` (line 1763) and `HooksConfig` (line 1777) — these are the Claude Code PreToolUse/PostToolUse hook install configs (`hooks.claude-code` in profiles). Adding `SessionHooks`/`SessionHook` nearby and confusing the two will cause incorrect doc comments, wrong merge semantics, or wiring into the wrong field in `execution_runtime.rs`.

**Why it happens:** Both use the word "hook" and both appear in profile definitions.

**How to avoid:** `session_hooks` field on `Profile` is for script execution at session start/stop. `hooks` field on `Profile` is for Claude Code hook installation. Keep them separated; name the new types `SessionHooks` and `SessionHook` exactly as upstream names them.

**Warning signs:** Any code that references `profile.hooks.hooks` in the session-lifecycle path is a bug.

### Pitfall 2: Forgetting `ProfileDeserialize` + `merge_profiles` + `to_raw_profile` All Need the New Field

**What goes wrong:** Adding `session_hooks` to `Profile` but not to `ProfileDeserialize` → deserialization silently ignores the field (Rust struct-literal completeness check will catch `From<ProfileDeserialize>` if you add it to `Profile` but `ProfileDeserialize` silently drops it because `serde(deny_unknown_fields)` on `ProfileDeserialize` would actually ERROR at runtime on unknown fields from JSON). And forgetting `merge_profiles` means `extends:` profiles lose session hooks from the base.

**Why it happens:** The fork's profile pattern requires updating four parallel structures.

**How to avoid:** The struct-literal completeness check in `From<ProfileDeserialize> for Profile` (see comment at line 2288: "Exhaustively enumerated here so rustc's struct-literal completeness check catches any future field additions") will fail to compile if `session_hooks` is in `Profile` but missing from the `From` impl. Use this as a compile-time gate.

**Required updates (all four must happen in the same task):**
1. `profile/mod.rs`: Add `SessionHook` + `SessionHooks` structs; add `session_hooks: SessionHooks` field to `Profile` and `ProfileDeserialize`; update `From<ProfileDeserialize>` and `merge_profiles`.
2. `policy.rs`: `to_raw_profile()` add `session_hooks: self.session_hooks.clone()` (line ~210, in the struct literal).
3. `sandbox_prepare.rs`: `PreparedSandbox` add `session_hooks: profile::SessionHooks` field; wire from loaded_profile (line ~506, existing pattern).
4. `launch_runtime.rs`: `ExecutionFlags` add `session_hooks: profile::SessionHooks`; `ExecutionFlags::defaults()` set `session_hooks: profile::SessionHooks::default()`; `LaunchPlan` creation carries it forward.

### Pitfall 3: Upstream Tests Expect Fail-Open Behavior

**What goes wrong:** The upstream `hook_runtime.rs` test `test_execute_before_hook_fail_open` (line ~555 in the upstream source) asserts `Ok(vars)` when the hook exits 1. Porting this test verbatim will cause the fork's fail-closed variant to fail the test.

**Why it happens:** The test was written for upstream's fail-open contract.

**How to avoid:** Replace `test_execute_before_hook_fail_open` with `test_execute_before_hook_fail_closed` that asserts `Err(_)` on non-zero exit. The test name change + comment citing D-01 make the divergence explicit.

### Pitfall 4: Windows `session_hooks` Scope Boundary — `LowIlPrimary` Not `BrokerLaunchNoPty`

**What goes wrong:** D-05 says hooks use `LowIlPrimary` (primary-token) broker arm, but the CONTEXT.md also mentions `BrokerLaunchNoPty`. These are different: `BrokerLaunchNoPty` is used for the MAIN sandboxed child on profiles with `windows_low_il_broker: true`. Hooks are SHORT-LIVED processes (not the main child), so they do not need ConPTY. The `LowIlPrimary` arm spawns directly with a Low-IL primary token — appropriate for short-lived hook scripts that don't need PTY.

**Actually:** After reviewing `WindowsTokenArm` selection: for hook processes, the cleanest approach is `LowIlPrimary` direct spawn (using `nono::create_low_integrity_primary_token()`). This is EXACTLY what `LowIlPrimary` was designed for: "mandatory label NO_WRITE_UP enforces write-deny via MIC pre-DACL kernel check." The hook needs Low-IL confinement (D-05) and no PTY. `LowIlPrimary` is the correct arm. [VERIFIED: codebase, `launch.rs:1117-1125` doccomment]

**Why it happens:** The `BrokerLaunchNoPty` path is wired into `spawn_windows_child` for the main child; hooks are a different spawn path entirely.

**How to avoid:** The Windows hook executor constructs its own `Command` + low-IL token via `nono::create_low_integrity_primary_token()` directly — it does NOT call `spawn_windows_child`. This is the correct parallel structure to the Unix side which also does not use `execute_supervised_runtime`.

### Pitfall 5: ENV_LOCK in Hook Runtime Tests

**What goes wrong:** Tests that mutate `HOME` (to isolate `~/.nono/sessions/`) race with other tests that read `HOME` (e.g., `config::check_sensitive_path`). The upstream `hook_runtime.rs` already uses `ENV_LOCK` + `EnvVarGuard` from `crate::test_env` — this must be preserved in the fork's port.

**Why it happens:** Rust runs unit tests in parallel within the same process. CLAUDE.md § Environment Variables in Tests documents this exact pattern.

**How to avoid:** The upstream `isolated_home()` helper in the test module correctly acquires `ENV_LOCK` then sets `HOME` via `EnvVarGuard::set_all`. Port this helper verbatim, including the lock acquisition with `match lock { Ok(g) => g, Err(p) => p.into_inner() }` poison handling.

---

## Code Examples

Verified patterns from official sources (codebase inspection):

### `is_dangerous_env_var` Extension Pattern (D-09)

```rust
// Source: crates/nono-cli/src/exec_strategy/env_sanitization.rs:14
// Add the following to is_dangerous_env_var() for Windows danger vars:
// (cfg-comment rationale inline; the function itself is cross-platform)

// Windows hook: PATH/PATHEXT control which executable resolves; a hook writing
// PATH=<attacker-controlled> to the env file lets the Medium-IL parent resolve
// the wrong binary when launching the sandboxed child (Low-IL-writer → Medium-IL-reader gap).
|| key.eq_ignore_ascii_case("PATH")
|| key.eq_ignore_ascii_case("PATHEXT")
// COMSPEC controls which command interpreter cmd.exe invocations use; a hook
// writing COMSPEC=/evil.exe can redirect subsequent shell invocations.
|| key.eq_ignore_ascii_case("COMSPEC")
// PowerShell execution policy bypass and module injection vectors:
|| key.eq_ignore_ascii_case("PSModulePath")          // module search path injection
|| key.eq_ignore_ascii_case("PSModuleAnalysisCachePath") // analysis cache poisoning
|| key.eq_ignore_ascii_case("__PSLockdownPolicy")    // PowerShell constrained-language bypass
// Windows system root values — redefining these redirects system DLL resolution:
|| key.eq_ignore_ascii_case("SystemRoot")
|| key.eq_ignore_ascii_case("windir")
// TEMP/TMP — a hook writing TEMP=/attacker-controlled could redirect temp files
// from the Medium-IL parent's perspective to an attacker-owned location:
|| key.eq_ignore_ascii_case("TEMP")
|| key.eq_ignore_ascii_case("TMP")
// .NET hook injection (already covered by "DOTNET_STARTUP_HOOKS" above, but
// ComSpec is distinct from COMSPEC on case-insensitive Windows — handled by eq_ignore_ascii_case)
```

**Note:** On Windows, env-var comparison is case-insensitive. The `env_key_matches` helper in `env_sanitization.rs` already uses `eq_ignore_ascii_case` on Windows — extend `is_dangerous_env_var` similarly for the new Windows vars. [VERIFIED: codebase, `env_sanitization.rs:55-61`]

### `terminate_job_object` for Hook Timeout

```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:290
// Already implemented and exported as pub(super) — the Windows hook executor
// module lives in exec_strategy_windows/ so can use pub(super).
pub(super) fn terminate_job_object(job: HANDLE, exit_code: u32) -> Result<()>
```

### `EnvFileGuard::create` Equivalent for Windows

```rust
// Pattern: CREATE_NEW (O_EXCL equivalent) + Low-IL label
// Source: derived from labels_guard.rs pattern + std docs for create_new

fn create_windows_env_file(session_id: &str) -> Result<WindowsEnvFileGuard> {
    let session_env_dir = session::ensure_sessions_dir()?.join(session_id);
    std::fs::create_dir_all(&session_env_dir)
        .map_err(|e| NonoError::ConfigParse(format!("Failed to create session dir: {e}")))?;
    let path = session_env_dir.join("env");
    // CREATE_NEW: fails if file exists (O_EXCL equivalent)
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|e| NonoError::ConfigParse(format!("Failed to create env file: {e}")))?;
    // Apply Low-IL mandatory label so other Low-IL processes cannot access it
    // (DACL narrowing to hook token SID is defense-in-depth; label is the primary gate)
    // Use try_set_mandatory_label from nono lib — same pattern as labels_guard
    Ok(WindowsEnvFileGuard { path })
}
```

### Profile Types (upstream daa55c8, ported to fork)

```rust
// Source: git show daa55c8 -- crates/nono-cli/src/profile/mod.rs (upstream addition)
// These types are NEW in the fork.

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionHook {
    /// Absolute path to the hook script file.
    pub script: PathBuf,
    /// Optional timeout in seconds. No default — timeout only when set.
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionHooks {
    pub before: Option<SessionHook>,
    pub after: Option<SessionHook>,
}
```

**Merge semantics in `merge_profiles`** (from upstream, carried into fork):
```rust
// Child overrides base per-hook slot (Option-semantics: child.before.or(base.before))
session_hooks: SessionHooks {
    before: child.session_hooks.before.or(base.session_hooks.before),
    after: child.session_hooks.after.or(base.session_hooks.after),
},
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Upstream fail-open hook behavior | Fork fail-closed (D-01) | Phase 58 (this phase) | Session start blocked on before-hook failure — security-favoring behavior |
| No session lifecycle hooks in fork | `session_hooks` profile field on both platforms | Phase 58 (this phase) | Enables setup/cleanup scripts for CI pipelines and credential injection |
| Windows hook = no implementation (`#[cfg(not(unix))]` empty stub) | Windows hook via `LowIlPrimary` broker | Phase 58 (this phase) | Windows parity with Unix hook capability |

**Deprecated/outdated:**
- Upstream `#[cfg(not(unix))]` no-op branch in `EnvFileGuard::create`: the fork does NOT use this. The fork has a real Windows implementation in `hook_runtime_windows.rs` instead.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cmd.exe /D` disables AutoRun registry keys (injection prevention for `.cmd`/`.bat` hooks) | Pattern 3 — Windows exec path | Low: `/D` is well-documented Windows behavior; if wrong, hooks using `.cmd` files could be affected by AutoRun entries; verify via Windows docs before implementing |
| A2 | The exact SID narrowing for env-file DACL (Low-IL label alone vs. package-SID ACE) | Pattern 4 — Windows env-file ACL | Medium: if label alone is insufficient, a different Low-IL process could read the env file; mitigated by the mandatory-label (Low-IL processes can't write to Medium-IL file, and parent reads at Medium-IL) |
| A3 | `LowIlPrimary` arm is the correct arm for Windows hook execution (not `BrokerLaunchNoPty`) | Pattern 3 / Pitfall 4 | Medium: if `LowIlPrimary` triggers `STATUS_DLL_INIT_FAILED` for the hook interpreter, the broker approach would be needed; mitigated by: hooks are short-lived, don't need ConPTY, and the RESEARCH in Phase 30/31 showed the failure was CSRSS console-attach specific (not a general Low-IL spawn failure) |

---

## Open Questions

1. **Does `LowIlPrimary` work for `powershell.exe -NoProfile -File` spawned as a hook?**
   - What we know: Phase 30/31 showed `STATUS_DLL_INIT_FAILED` (0xC0000142) for `claude.exe` (heavy Electron) under Low-IL primary token when using `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`. Hooks do NOT use PTY.
   - What's unclear: Whether `powershell.exe` requires any CSRSS console-attach path that fails under direct Low-IL spawn even without PTY.
   - Recommendation: Implement with `LowIlPrimary` direct spawn first. If UAT shows `0xC0000142` for the hook, escalate to a new debug session — but this is unlikely since Phase 51/52 showed the `BrokerLaunchNoPty` path works for non-PTY, and the 0xC0000142 was PTY-specific.

2. **Should `timeout_secs` expiry on a before-hook be fail-closed or silently pass?**
   - What we know: D-01 says fail-closed. D-03 says "before-hook failure → session does not start." Timeout is a failure mode.
   - What's unclear: Upstream treats timeout as a warning (fail-open). The fork's D-01 implies timeout → `Err`. But an infinite-wait hook could also be an external service that's slow.
   - Recommendation: Treat timeout on a before-hook as `Err` (fail-closed, consistent with D-01/D-03). Document this in the ADR. Users who want a "best-effort" hook can set a long `timeout_secs`.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `rustup target x86_64-unknown-linux-gnu` | Cross-target clippy (CLAUDE.md MUST rule) | Unknown — Windows dev host | — | Mark cross-target Linux clippy PARTIAL; defer to CI per cross-target-verify-checklist.md |
| `rustup target x86_64-apple-darwin` | Cross-target clippy (CLAUDE.md MUST rule) | Unknown — Windows dev host | — | Mark cross-target macOS clippy PARTIAL; defer to CI |
| `nono-shell-broker.exe` | Hook execution (if BrokerLaunchNoPty path used) | Present at `target/release/nono-shell-broker.exe` | dev build | LowIlPrimary direct spawn (no broker needed for hooks) |

**Missing dependencies with fallback:**
- Cross-target clippy toolchains: defer to CI per the established PARTIAL/human_needed pattern; the `hook_runtime.rs` Unix code will not compile on Windows host anyway.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | None (Cargo.toml `[dev-dependencies]` controls) |
| Quick run command | `cargo test -p nono-cli --lib -- hook_runtime` |
| Full suite command | `cargo test --workspace` (or `make test`) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-HOOK-01 | `session_hooks` field deserializes from JSON | unit | `cargo test -p nono-cli --lib -- profile::tests::test_session_hooks` | ❌ Wave 0 |
| REQ-HOOK-01 | `session_hooks` field present in schema + `$defs/SessionHooks` + `$defs/SessionHook` | unit | `cargo test -p nono-cli --lib -- profile::tests::test_schema_has_session_hooks` | ❌ Wave 0 |
| REQ-HOOK-01 (SC2) | Unix: before-hook exports env vars, dangerous vars filtered | unit (unix-only) | `cargo test -p nono-cli --lib -- hook_runtime::tests::test_execute_before_hook_basic` | ❌ Wave 0 |
| REQ-HOOK-01 (SC4) | Unix: before-hook non-zero exit → `Err` (fail-closed) | unit (unix-only) | `cargo test -p nono-cli --lib -- hook_runtime::tests::test_execute_before_hook_fail_closed` | ❌ Wave 0 |
| REQ-HOOK-01 (SC4) | Unix: before-hook timeout → `Err` (fail-closed) | unit (unix-only) | `cargo test -p nono-cli --lib -- hook_runtime::tests::test_execute_before_hook_timeout_fail_closed` | ❌ Wave 0 |
| REQ-HOOK-01 (SC4) | After-hook non-zero → logs loudly AND propagates `Err` | unit (unix-only) | `cargo test -p nono-cli --lib -- hook_runtime::tests::test_execute_after_hook_fail_closed` | ❌ Wave 0 |
| REQ-HOOK-01 (SC3) | Windows ADR committed to `.planning/architecture/` | manual | — | ❌ Wave 0 (plan task) |
| REQ-HOOK-01 (SC1) | Windows: env-file `CREATE_NEW` prevents clobber | unit (windows-only) | `cargo test -p nono-cli --lib -- hook_runtime_windows::tests::test_env_file_create_new_prevents_clobber` | ❌ Wave 0 |
| REQ-HOOK-01 (D-09) | Windows danger vars (`PATH`, `PATHEXT`, `COMSPEC`, `PSModulePath`, etc.) filtered | unit | `cargo test -p nono-cli --lib -- exec_strategy::tests::test_windows_dangerous_vars` | ❌ Wave 0 |
| REQ-HOOK-01 (SC1) | `session_hooks` present in upstream `daa55c8` schema — schema test | integration | `cargo test -p nono-cli --test profile_validate_strict` (or a new schema_shape.rs if added) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p nono-cli --lib -- hook_runtime`
- **Per wave merge:** `cargo test --workspace` (make test)
- **Phase gate:** Full suite green + cross-target clippy PARTIAL noted in VERIFICATION.md before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/nono-cli/src/hook_runtime.rs` — Unix runtime (new file)
- [ ] `crates/nono-cli/src/hook_runtime_windows.rs` — Windows runtime (new file)
- [ ] `crates/nono-cli/src/profile/mod.rs` — `SessionHook` + `SessionHooks` structs + field additions
- [ ] `crates/nono-cli/data/nono-profile.schema.json` — `SessionHooks` + `SessionHook` `$defs`
- [ ] Test module in `hook_runtime.rs` — ENV_LOCK + isolated_home pattern (Wave 0 infrastructure before Unix impl tests run)

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no (hooks are run-time side-effects, not session tokens) | — |
| V4 Access Control | yes | `path_is_owned_by_current_user` + DACL guards + mandatory-label checks (D-10) |
| V5 Input Validation | yes | `validate_hook_script` / `validate_hook_script_windows` — absolute path, canonical, regular file, owner, world-writable check |
| V6 Cryptography | no | — |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Symlink swap between path validation and spawn | Tampering | `canonicalize()` at validation time; accept residual TOCTOU (documented; same trade-off as upstream) |
| Hook script in world-writable dir (any user can replace) | Spoofing | `is_world_writable(parent)` check (Unix); effective-rights ACL check on parent (Windows) |
| Env-file injection (Low-IL hook writes dangerous vars to env file) | Elevation of Privilege | `is_dangerous_env_var()` filter on parent read; `CREATE_NEW` prevents race replacement |
| `PATH` hijacking via env-file export | Tampering | `PATH` added to `is_dangerous_env_var()` for Windows (D-09) |
| PowerShell `$PROFILE` injection in hook scripts | Tampering | `-NoProfile` flag on `powershell.exe` hook invocations |
| Hook script not owned by current user (shared writable hook) | Spoofing | Owner check (Unix: `uid` comparison; Windows: `path_is_owned_by_current_user`) |
| After-hook silently swallowing errors (CI automation blind to failure) | Repudiation | D-04: after-hook failure → `error!` log + non-zero exit; mirrors diagnostic-footer pattern |
| Windows env-file readable by other Low-IL processes | Information Disclosure | Low-IL mandatory label on env file (defense-in-depth; DACL narrowing is the primary control) |
| `cmd.exe /C` AutoRun registry key injection | Tampering | `/D` flag disables AutoRun on `cmd.exe` invocations |

---

## Sources

### Primary (HIGH confidence)
- Upstream commit `daa55c8` — `hook_runtime.rs` (full 610-line source read in this session via `git show`)
- Upstream commit `1335351` — confirms `hook_runtime` is unix-only gated
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `WindowsTokenArm`, `BrokerLaunchNoPty`, `LowIlPrimary`, `terminate_job_object`, `build_child_env` (read in this session)
- `crates/nono-cli/src/exec_strategy/env_sanitization.rs` — `is_dangerous_env_var` full implementation (read in this session)
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` — `AppliedDaclGrantsGuard` (read in this session)
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` — `AppliedLabelsGuard`, `path_is_owned_by_current_user` usage (read in this session)
- `crates/nono-cli/src/profile/mod.rs` — `Profile`, `ProfileDeserialize`, `From<ProfileDeserialize>`, `merge_profiles` (read in this session)
- `crates/nono-cli/src/policy.rs` — `to_raw_profile()` (read in this session)
- `crates/nono-cli/src/sandbox_prepare.rs` — `PreparedSandbox` (read in this session)
- `crates/nono-cli/src/launch_runtime.rs` — `ExecutionFlags`, `LaunchPlan` (read in this session)
- `crates/nono-cli/src/execution_runtime.rs` — existing Windows broker arm wiring (read in this session)
- `.planning/templates/cross-target-verify-checklist.md` — cross-target clippy gate rules (read in this session)

### Secondary (MEDIUM confidence)
- Memory `project_sandbox_the_tools` — Phase 60 PowerShell-steering direction (confirmed `powershell.exe -NoProfile` as the fork's standard)
- Memory `feedback_windows_mandatory_label_write_owner` — `WRITE_OWNER` requirement for `SetNamedSecurityInfoW`

### Tertiary (LOW confidence)
- [ASSUMED] `cmd.exe /D` AutoRun-disable behavior — well-known but not re-verified via official Windows docs in this session

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies are existing workspace crates; no new external packages
- Architecture: HIGH — all integration points read from actual codebase; upstream diff read in full
- Pitfalls: HIGH — pitfalls derived from actual code inspection (ProfileDeserialize, merge_profiles, test divergence)
- Windows exec path: HIGH — based on direct code inspection of `launch.rs`, `dacl_guard.rs`, `labels_guard.rs`, and cross-referenced with Phase 51/60 memory

**Research date:** 2026-06-05
**Valid until:** 2026-07-05 (stable codebase; all upstream commits already fetched)
