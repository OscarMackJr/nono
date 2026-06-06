---
phase: 58-session-lifecycle-hooks
reviewed: 2026-06-06T02:28:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - crates/nono-cli/src/hook_runtime.rs
  - crates/nono-cli/src/hook_runtime_windows.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/exec_strategy/env_sanitization.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/src/proxy_runtime.rs
findings:
  critical: 3
  warning: 4
  info: 2
  total: 9
status: issues_found
---

# Phase 58: Code Review Report

**Reviewed:** 2026-06-06T02:28:00Z
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

Phase 58 ports upstream session-lifecycle hooks (before/after) with a fail-closed divergence
(any hook failure aborts the session). The wiring, types, schema, and profile threading are
structurally sound. The fail-closed invariant is consistently implemented and the dangerous-env-var
filter (D-09) is correctly applied.

Three critical findings were identified. Two are security issues in the Windows hook executor:
the Low-IL token created by `run_hook_windows` is never used to spawn the hook process (the hook
runs at parent's Medium-IL), and the Job Object assignment failure is silently downgraded from a
fail-closed error to a warning (violating D-01). The third critical issue is a logic error in
`execution_runtime.rs`: the after-hook is never called when the `Direct` execution strategy is
used, because the Direct branch calls `std::process::exit` without running the after-hook.

Four warnings cover: a TOCTOU window between `validate_hook_script` and spawn on both platforms;
the Unix `run_hook` leaking the child's stderr buffer into the background thread indefinitely on
timeout; the `EnvFileGuard` zeroing via `metadata.len()` which reads the file size at `Drop` time
(not at creation time, so a concurrent writer could grow the file and leave a tail of real content
unzeroed); and the `is_dangerous_env_var` function using case-sensitive comparison for `LD_*` and
`DYLD_*` prefixes on Unix, which means a hook could export `Ld_PRELOAD` (mixed-case) and have it
pass the filter before being injected into the sandboxed child.

---

## Critical Issues

### CR-01: Windows hook process runs at parent Medium-IL, not Low-IL as documented (D-05 not enforced)

**File:** `crates/nono-cli/src/hook_runtime_windows.rs:345-360`

**Issue:** `run_hook_windows` calls `nono::create_low_integrity_primary_token()` and stores the
token in `_low_il_token`, but the token is never actually used to spawn the hook process.
`cmd.spawn()` on line 361 spawns using the parent process token (Medium-IL). The comment
acknowledges this explicitly ("std::process::Command does not support custom token directly on
stable Rust ... The _low_il_token is held in scope to demonstrate the D-05 intent").

This is a security regression from the documented design. Module-level doc comment claims "Hooks
run as Low-IL primary token processes (D-05)" and "Hooks run as Low-IL confined processes outside
the sandboxed child." Both are false. The hook script runs with the full Medium-IL supervisor
token. If the hook script is compromised (e.g., via a supply-chain attack on the script path),
it has the full access rights of the nono supervisor process — including any credentials loaded
from the keystore and stored in memory, and the ability to modify files that the supervisor can
modify.

The Job Object confinement does NOT substitute for the Low-IL token: Job Objects constrain
CPU/memory/handle inheritance but do NOT restrict filesystem or registry access. A hook running
Medium-IL inside a Job Object has the same DACL access as the parent.

The `_low_il_token` binding also introduces a second bug: the `_low_il_token` variable is of type
`Option<OwnedHandle>`. `OwnedHandle::drop` presumably closes the handle. This handle is closed
before the hook process exits (at function return), but since the handle was never used to spawn
anything, this is harmless. However, creating and immediately discarding a Low-IL token on every
hook call is a misleading and wasteful no-op.

**Fix:** Either implement the Low-IL spawn via `CommandExt::raw_attribute` + `CreateProcessAsUserW`
before this code ships with its current security claims, OR revise the doc comment and module-level
claims to accurately state "hooks run at parent's integrity level (Medium-IL) confined to a Job
Object" and remove the `_low_il_token` creation (which is pure theater). Given this is
security-critical, the recommended fix is to remove the misleading claim:

```rust
// BEFORE (misleading — token is created but never used):
let _low_il_token: Option<OwnedHandle> = match nono::create_low_integrity_primary_token() {
    Ok(token) => Some(token),
    Err(e) => { unsafe { CloseHandle(job_handle) }; return Err(e); }
};

// AFTER option A — remove the theater and accurately document:
// NOTE (D-05 deferred): Hook currently runs at parent's integrity level (Medium-IL)
// confined to a Job Object. Low-IL spawn via CreateProcessAsUserW is deferred to a
// follow-up phase. See ADR-58. This does NOT match the doc comment claims below —
// update doc comments to reflect actual behavior.

// AFTER option B — implement correctly:
let low_il_token = nono::create_low_integrity_primary_token().map_err(|e| {
    unsafe { CloseHandle(job_handle) };
    e
})?;
// ... use low_il_token in a CreateProcessAsUserW call instead of cmd.spawn()
```

---

### CR-02: Job Object assignment failure silently degrades timeout enforcement (fail-closed violation)

**File:** `crates/nono-cli/src/hook_runtime_windows.rs:376-383`

**Issue:** When `assign_process_to_job` fails, the code logs a `warn!` and continues:

```rust
if let Err(e) = assign_result {
    warn!(
        "Failed to assign hook process {} to job object: {e}; timeout may not terminate hook tree",
        pid
    );
}
```

This is a direct violation of the fail-closed D-01 invariant. When Job Object assignment fails,
the timeout mechanism (`TerminateJobObject`) becomes ineffective. A hook that hangs will not be
killed at `timeout_secs`, so the before-hook timeout that is supposed to prevent an infinite
pre-session hang becomes a soft advisory.

The `assign_process_to_job` failure modes include:
- The hook process already exited (benign — TOCTOU between spawn and assign)
- Access denied due to permission model (indicates the process is running at an unexpected IL)
- Invalid parameter (indicates PID was invalid, which should not happen for a freshly spawned child)

The first case is benign and is the scenario the comment tries to justify. But the comment's
framing ("process has already exited") does not justify silently degrading timeout enforcement for
all other failure modes. A hook that immediately exits should be observed by the worker thread's
`wait_with_output` and return before the timeout fires anyway, so the degraded case is the one
where the hook is STILL running and assignment fails — exactly when the timeout matters.

Additionally, the race window between `cmd.spawn()` (line 361) and `assign_process_to_job` (line
376) means a fast-forking hook subprocess can escape the Job Object entirely. The correct pattern
is to create the Job Object with `JOB_OBJECT_ASSIGN_PROCESS` and set
`JOBOBJECT_EXTENDED_LIMIT_INFORMATION.BasicLimitInformation.LimitFlags |=
JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` before spawning, or use a handle-inheritance-based approach.

**Fix:** At minimum, treat assignment failure as an error when the hook is still expected to be
running (i.e., the worker thread has not yet signaled), or fail-closed on any assignment error:

```rust
if let Err(e) = assign_result {
    // Fail closed: if we cannot contain the hook in the job object,
    // the timeout cannot be enforced. Abort rather than run uncontrolled.
    unsafe { CloseHandle(job_handle) };
    return Err(NonoError::CommandExecution(std::io::Error::other(format!(
        "Failed to assign hook process to job object (fail-closed, D-01): {e}"
    ))));
}
```

Alternatively, create the Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and the
kill-on-close semantics BEFORE spawning, then assign before the job handle is ever checked, to
eliminate the TOCTOU window.

---

### CR-03: After-hook never executed in `Direct` strategy (after-hook silently skipped)

**File:** `crates/nono-cli/src/execution_runtime.rs:529-559`

**Issue:** The `match strategy` block in `execute_sandboxed` handles `ExecStrategy::Direct` and
`ExecStrategy::Supervised`. The after-hook dispatch is wired only inside the `Supervised` arm
(lines 587-603). The `Direct` arm calls `std::process::exit(exit_code)` on Windows (line 546) or
performs an `exec` replacement on Unix (line 550-558), with no after-hook call in either path.

On Unix, the Direct strategy replaces the nono process via `exec`, so there is no opportunity to
run the after-hook — this is a documented trade-off of the Direct strategy. However, on Windows,
the Direct arm collects an `exit_code` and calls `std::process::exit` (it does NOT exec), which
means there IS a point at which the after-hook could be called, but is not.

More importantly: the session_hooks infrastructure does not document or enforce that `after` hooks
are ignored in Direct mode. A user who sets an `after` hook in their profile will silently get no
after-hook execution when `nono run` happens to select the Direct strategy (e.g. via `--direct`
flag). This is a logic correctness defect: the feature advertises a guarantee ("hook executed after
the sandboxed process exits") that is not upheld in all code paths.

**Fix:** For Windows Direct strategy, call the after-hook before `std::process::exit`:

```rust
exec_strategy::ExecStrategy::Direct => {
    #[cfg(target_os = "windows")]
    {
        let exit_code = exec_strategy::execute_direct(...)?;
        cleanup_capability_state_file(&cap_file_path);

        // After-hook: must run before process exit (Direct has no exec replacement on Windows).
        if let Some((after, session_id)) =
            flags.session_hooks.after.as_ref().zip(hook_session_id.as_deref())
        {
            hook_runtime_windows::execute_after_hook(after, session_id, &current_dir, exit_code)?;
        }

        drop(config);
        for (_, value) in &mut hook_env_vars_owned { value.zeroize(); }
        drop(loaded_secrets);
        std::process::exit(exit_code);
    }
    ...
}
```

For Unix Direct (exec), the constraint is real (no after-hook possible after exec), but this
should be documented in the `session_hooks` schema and profile docs, and optionally warned at
profile load time if an after-hook is configured alongside a Direct strategy profile.

---

## Warnings

### WR-01: TOCTOU race between `validate_hook_script` and spawn (both platforms)

**File:** `crates/nono-cli/src/hook_runtime.rs:236-296`, `crates/nono-cli/src/hook_runtime_windows.rs:474-556`

**Issue:** Both validators call `path.canonicalize()` to get a canonical path, check ownership and
ACLs on that canonical path, and return the canonical path. The `build_hook_command` (Unix) and
`build_windows_hook_command` (Windows) then spawn the canonical path as the process executable.

There is a TOCTOU window between the last security check in `validate_hook_script` and the
`cmd.spawn()` call: an attacker who can observe the process and has write access to the parent
directory (or can perform a symlink swap) could replace the script file between validation and
execution. On Unix this is partially mitigated by the world-writable parent check, but the check
itself is racy (metadata read, then spawn). On Windows, the ACL check on the parent is similarly
racy.

This is an inherent limitation of POSIX/Win32 hook architectures without an `O_PATH`/
`CreateFileW` fd-based execution model. The current approach is consistent with upstream daa55c8
and with common sandboxing practice, but operators should be aware.

**Fix (hardening, not full elimination):** On Linux, open the script with `O_PATH | O_NOFOLLOW`
to get a file descriptor, verify the fd's metadata (ownership/mode) via `fstat`, then use
`/proc/self/fd/<n>` as the executable path for `Command::new`. This eliminates the window between
validation and spawn at the cost of a `/proc` dependency. Document the residual TOCTOU risk in
the `validate_hook_script` doc comment, as the current comment implies the checks are complete.

---

### WR-02: Unix `run_hook` leaks background thread holding `Child` stdout/stderr handles on timeout

**File:** `crates/nono-cli/src/hook_runtime.rs:400-427`

**Issue:** On timeout, `kill_process_group(pid)` is called and `run_hook` returns
`Ok(HookOutput { timed_out: true })`. The background thread (line 401-403) is still running and
still owns `child` — which owns the piped `stdout` and `stderr` handles. If the child process
does not die quickly (e.g., SIGTERM is caught/ignored and SIGKILL is pending in the 100ms
`thread::sleep`), the background thread is blocked in `wait_with_output()` until `child` exits
or the process crashes. The caller discards the thread handle (line 401: no join, no abort
mechanism).

On the caller side, `run_hook` returns and `execute_before_hook` returns `Err(timed_out)`. The
session is aborted, but the background thread continues running, holding a reference to the child
process stdout/stderr pipe read-ends. In a long-running nono supervisor session this is a thread
and fd leak per timed-out hook.

The Windows implementation (hook_runtime_windows.rs:386-394) has the same pattern.

**Fix:** After calling `kill_process_group`/`TerminateJobObject`, the caller should signal the
background thread to abandon its wait. The simplest approach is to use a `tokio` task with
`tokio::time::timeout` (which the codebase already uses in other places) rather than a raw thread.
Alternatively, the background thread should be given a channel to receive a "kill" signal, or the
`Child` should be moved into an `Arc<Mutex<...>>` so the main thread can call `child.kill()` to
unblock the `wait_with_output()` from the background thread.

Short-term mitigation: drop all piped stdout/stderr on the hook command before spawn on the
timeout path by using `Stdio::null()` for both in the timed-out case, so the background thread
unblocks when the process exits after SIGKILL (it will still block for up to `kill_process_group`'s
100ms sleep, then unblock).

---

### WR-03: `EnvFileGuard::Drop` reads `metadata.len()` at drop time, not creation time

**File:** `crates/nono-cli/src/hook_runtime.rs:359-371`, `crates/nono-cli/src/hook_runtime_windows.rs:761-775`

**Issue:** The `Drop` implementation on both `EnvFileGuard` (Unix) and `WindowsEnvFileGuard`
(Windows) zeros the file contents using `metadata.len()` to determine how many bytes to write:

```rust
let zeros = vec![0u8; metadata.len() as usize];
let _ = file.write_all(&zeros);
```

The `metadata()` call is made at `Drop` time. If the hook process appended data after the initial
file creation (which is the intended use — the hook writes its env vars into the file), then
`metadata.len()` correctly reflects the full file size. However, there is a small race: if
the hook process is killed mid-write and the file is partially written, `metadata.len()` reflects
only the bytes written so far. The zeroing will cover exactly those bytes, which is the correct
behavior.

The actual bug is subtler: `metadata.len() as usize` can truncate on 32-bit targets where
`u64 as usize` silently truncates to `u32::MAX`. If the file is larger than 4 GiB (not realistic
for an env file, but enforcing correct arithmetic is required by CLAUDE.md's arithmetic rules for
security-critical code), the zeroing silently writes only 4 GiB of zeros and leaves the tail
unzeroed.

A more real concern: the `metadata()` is called on the file handle (`file.metadata()`), not on
the path. On Windows, if the file has been externally opened and locked between the `open()` and
`metadata()` calls, `metadata()` may return an error and the zeroing block is skipped entirely
(the `let Ok(metadata)` pattern silently skips on Err).

**Fix:** Use `checked_cast` / `try_into()` for the length conversion, and log (but continue) if
zeroing fails:

```rust
if let Ok(meta) = file.metadata() {
    let len = meta.len();
    if let Ok(len_usize) = usize::try_from(len) {
        let zeros = vec![0u8; len_usize];
        let _ = file.write_all(&zeros);
        let _ = file.sync_all();
    }
    // else: file too large to zero (> usize::MAX bytes) — skip zeroing, still delete
}
let _ = std::fs::remove_file(&self.path);
```

---

### WR-04: `is_dangerous_env_var` uses case-sensitive prefix check for `LD_*`/`DYLD_*` on Unix

**File:** `crates/nono-cli/src/exec_strategy/env_sanitization.rs:18-20`

**Issue:** The dangerous-var filter uses `key.starts_with("LD_")` and `key.starts_with("DYLD_")`
for linker injection checks. On Linux and macOS, environment variable names are conventionally
uppercase, but the kernel and dynamic linker DO perform case-sensitive lookups. However, on
macOS, `DYLD_INSERT_LIBRARIES` is matched case-sensitively, while `dyld_insert_libraries` would
not be blocked.

More critically: the filter is used to filter vars exported by a hook script into the sandboxed
child's environment. A hook script (which is expected to be trusted, per the ownership checks)
that outputs `Ld_PRELOAD=/evil.so` would pass the filter and reach the child. While the hook
script itself is validated for ownership, a compromised hook (or a hook that reads from an
untrusted source, like a CI environment variable) could inject mixed-case variants.

The Windows D-09 vars correctly use `eq_ignore_ascii_case`, but the linker/shell injection vars
use plain `starts_with`, which is case-sensitive. This inconsistency is architecturally fragile.

**Fix:** Apply `eq_ignore_ascii_case` for the known named vars and use a normalized check for
prefixes. For prefix checks, normalize to uppercase first:

```rust
pub(crate) fn is_dangerous_env_var(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    // Linker injection
    upper.starts_with("LD_")
        || upper.starts_with("DYLD_")
    // ... rest of the function using upper for prefix checks
    // Named vars can use eq_ignore_ascii_case
        || key.eq_ignore_ascii_case("BASH_ENV")
        || ...
}
```

Note: This changes behavior on Linux for `LD_*` — the glibc dynamic linker IS case-sensitive and
would not honor `ld_preload`, so the practical injection risk is low. However, keeping the filter
consistent with case-insensitive checking costs nothing and future-proofs against interpreter
implementations that do case-folding.

---

## Info

### IN-01: Session ID sourced from env var without validation on untrusted input

**File:** `crates/nono-cli/src/execution_runtime.rs:264-270`

**Issue:** `hook_session_id` is derived from `DETACHED_SESSION_ID_ENV`:

```rust
std::env::var(DETACHED_SESSION_ID_ENV)
    .ok()
    .filter(|id| !id.is_empty())
    .unwrap_or_else(session::generate_session_id)
```

The session ID is used as a directory component in `session::ensure_sessions_dir()` via
`EnvFileGuard::create(session_id)` which calls `sessions_dir.join(session_id)`. If
`NONO_DETACHED_SESSION_ID` is set to a value containing `..` or path separators (e.g.,
`../../etc`), the session directory could be created outside the expected `~/.nono/sessions/`
prefix — a directory traversal via a deliberately crafted env var.

In typical usage this env var is set by nono itself (detached launch path), so the attack surface
requires either a compromised parent process or an operator who manually sets it. However,
CLAUDE.md's path security section mandates validating environment variables before use, and
`NONO_DETACHED_SESSION_ID` is read from the environment without any component validation.

**Fix:** Validate the session ID before use: reject any value containing `/`, `\`, `..`, or
non-alphanumeric-non-hyphen-non-underscore characters:

```rust
fn is_safe_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
```

---

### IN-02: `unwrap_or` default on extension extraction silently accepts extensionless scripts as native executables

**File:** `crates/nono-cli/src/hook_runtime_windows.rs:251-254`

**Issue:**

```rust
let ext = interpreter_path
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or("");
```

An empty extension falls through to the `_` arm, which executes the path via `Command::new`
(direct spawn). This is documented as "Native .exe or extensionless: direct CreateProcess." The
behavior is intentional, but extensionless scripts on Windows (which Windows associates with no
interpreter by default) will silently fail to run and return a spawn error rather than a clear
"unrecognized extension" diagnostic. A user who creates `pre-hook` (no `.ps1` extension) will get
a cryptic OS error rather than a helpful message about unsupported extensions.

This is an info/usability issue, not a security issue, because `validate_hook_script_windows`
already validates the file is a regular file and is user-owned.

**Fix:** Add a diagnostic warning when an extensionless/unrecognized extension is used:

```rust
_ => {
    if !ext.is_empty() && !matches!(ext.to_ascii_lowercase().as_str(), "exe") {
        warn!("Unrecognized hook script extension '.{}'; attempting direct spawn. \
               Supported extensions: .ps1, .cmd, .bat, .exe", ext);
    }
    Command::new(&interpreter_path)
}
```

---

_Reviewed: 2026-06-06T02:28:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
