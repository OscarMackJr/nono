---
phase: 51-no-pty-low-il-broker-token-routing-write-deny-preservation
reviewed: 2026-05-26T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/data/policy.json
  - crates/nono-shell-broker/src/main.rs
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 51: Code Review Report

**Reviewed:** 2026-05-26
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Phase 51 adds a `WindowsTokenArm::BrokerLaunchNoPty` arm that routes non-PTY
supervised launches (profile opt-in `windows_low_il_broker`) through
`nono-shell-broker.exe` with anonymous-pipe stdio, `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`
gating, Authenticode broker verification, and a `--no-pty`
`STARTF_USESTDHANDLES` binding inside the broker. The profile plumbing
(`policy.rs` → `Profile` → `ExecConfig.prefers_low_il_broker`) is clean, fail-safe
(`is_some_and` defaults to `false` → `WriteRestricted`), and well-tested for
deserialization and merge semantics. The Win32 FFI handle inheritance flip/unflip
discipline correctly mirrors the existing `BrokerLaunch` arm and unflips on both
success and error paths.

However, the new arm breaks a load-bearing invariant from the existing detached
stdio path: it binds the child's **stderr to a separate pipe whose read end the
supervisor never drains**. The supervisor relay (`start_logging`) reads only
`stdout_read`; the existing detached path merges stderr into stdout precisely to
avoid an unread stderr pipe. This is a deadlock/hang and output-loss BLOCKER for
exactly the heavy-runtime children (Claude Code) the profile targets. The
write-deny integration test masks it because its child emits only a few bytes of
stderr. Several fail-secure and robustness warnings accompany it.

## Critical Issues

### CR-01: Child stderr bound to an undrained pipe — deadlock / hang + stderr loss

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1679-1680` (and the broker binding at `crates/nono-shell-broker/src/main.rs:292-301`)

**Issue:**
The `BrokerLaunchNoPty` arm builds the inherited-handle set as three *separate*
pipe ends:

```rust
let pipes = DetachedStdioPipes::create()?;
let inherit_handles: [HANDLE; 3] =
    [pipes.stdin_read, pipes.stdout_write, pipes.stderr_write];
```

The broker then binds these positionally as the grandchild's stdio
(`main.rs:298-300`): `hStdInput = [0]`, `hStdOutput = [1]`, `hStdError = [2]` —
so the child's **stderr writes flow into `pipes.stderr_write`**, whose parent read
end is `pipes.stderr_read`.

But the supervisor relay drains only `stdout_read`. `start_logging`
(`supervisor.rs:618-658`) resolves a single `source_handle` from
`detached_stdio.stdout_read` (or the PTY) and never reads `stderr_read`. There is
no reader of `stderr_read` anywhere in the supervisor (confirmed: `stderr_read` is
referenced only in the struct definition, `Drop`, and tests). The existing
detached path deliberately *merges* stderr into stdout to avoid exactly this:

```rust
// launch.rs:1863-1866 (existing detached path)
startup_info.dwFlags = STARTF_USESTDHANDLES;
startup_info.hStdInput  = pipes.stdin_read;
startup_info.hStdOutput = pipes.stdout_write;
startup_info.hStdError  = pipes.stdout_write;   // <-- merged, NOT stderr_write
```

Consequences once a child writes more than the anonymous-pipe buffer (~4 KiB
default) to stderr:
1. The child **blocks** on the stderr write because nothing drains `stderr_read`
   → the grandchild hangs → the broker's `WaitForSingleObject(INFINITE)` hangs →
   the whole supervised session deadlocks.
2. Even below the buffer threshold, all stderr output is silently lost (never
   logged, never relayed to `nono attach` / `nono logs`).

The target of this profile (`claude-code`, the only profile setting
`windows_low_il_broker: true`) is a heavy Node/Electron runtime that emits
substantial stderr — so this path will hang in normal use.

The write-deny integration test does not catch this: its child is
`cmd.exe /c "echo x > <fixture>"`, which emits only a short "Access is denied."
line, far below the pipe buffer (the test comment at lines 613-617 even relies on
this: "far below the pipe buffer — no relay deadlock"). The test proves write-deny
but not the relay shape.

**Fix:**
Merge stderr into stdout for the broker handle set, matching the existing detached
path so the single supervisor reader drains both fds:

```rust
let pipes = DetachedStdioPipes::create()?;
// Merge stderr into stdout (mirrors the detached path at launch.rs:1866) so the
// single `stdout_read` reader in start_logging drains both child fd 1 and fd 2.
// stderr_write's read end (stderr_read) is never drained by the supervisor.
let inherit_handles: [HANDLE; 3] =
    [pipes.stdin_read, pipes.stdout_write, pipes.stdout_write];
```

(The broker positionally binds `hStdError = inherit_handles[2]`, so passing
`stdout_write` as the third element merges the streams at the child. `stderr_write`
/ `stderr_read` then remain unused but are still closed by `close_child_ends` /
`Drop`.) Alternatively, spawn a second relay thread that drains `stderr_read` — but
the merge is the smaller change and is the invariant the rest of the supervised
pipe path already assumes. Add a regression test whose child emits >8 KiB of
stderr and assert the session completes without hanging.

## Warnings

### WR-01: Broker `--no-pty` silently falls open to the console when fewer than 3 handles arrive

**File:** `crates/nono-shell-broker/src/main.rs:292-301`

**Issue:**
The std-handle binding is guarded by `if args.no_pty && args.inherit_handles.len() >= 3`.
If `--no-pty` is passed but fewer than three `--inherit-handle` values arrive
(truncated/malformed argv, future caller change, a single capability handle), the
branch is skipped with **no else and no error** — `STARTF_USESTDHANDLES` is never
set, and the Low-IL child inherits the broker's console handles instead of the
intended pipes. That is a fail-open degradation: the supervisor relay gets nothing
and the child's stdio escapes to whatever console the broker holds. CLAUDE.md
mandates "Fail secure on any unsupported shape — never silently degrade."

**Fix:**
When `no_pty` is set, treat `< 3` handles as fatal rather than falling through:

```rust
if args.no_pty {
    if args.inherit_handles.len() < 3 {
        return Err(NonoError::SandboxInit(format!(
            "--no-pty requires exactly 3 inherit-handles (stdin,stdout,stderr); got {}",
            args.inherit_handles.len()
        )));
    }
    startup_info_ex.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup_info_ex.StartupInfo.hStdInput  = args.inherit_handles[0];
    startup_info_ex.StartupInfo.hStdOutput = args.inherit_handles[1];
    startup_info_ex.StartupInfo.hStdError  = args.inherit_handles[2];
}
```

### WR-02: No bounds check that `inherit_handles` contains exactly the 3 stdio handles

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1677-1680` and `crates/nono-shell-broker/src/main.rs:298-300`

**Issue:**
The contract between the producer (nono-cli) and consumer (broker) is purely
positional and unvalidated. The broker assumes `inherit_handles[0..3]` are
stdin/stdout/stderr, but `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` and the
`--inherit-handle` accumulator accept any number of handles in any order. If a
future change prepends another inheritable handle (e.g. a capability/job handle)
to the list, the broker would bind the wrong handle as the child's stdin — a
correctness and potential confused-deputy issue, since the child would receive an
arbitrary supervisor handle as fd 0/1/2. The `>= 3` guard does not pin which three
handles are the stdio set.

**Fix:**
Make the stdio handles explicit rather than positional within a variadic list —
e.g. add `--stdio-handles <in> <out> <err>` distinct from generic
`--inherit-handle`, or assert in the broker that the no-PTY stdio handles are the
first three AND that `inherit_handles.len() == 3` on the no-PTY path. At minimum,
document the positional contract as a hard invariant with a debug assertion in the
production arm.

### WR-03: `is_dev_build_layout` substring match is an Authenticode-verification bypass surface

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1971-1977` (consumed by the new arm at 1662-1672)

**Issue:**
The new `BrokerLaunchNoPty` arm gates broker Authenticode verification behind
`!is_dev_build_layout(&nono_exe)`. That helper decides via a raw substring scan of
the exe path:

```rust
s.contains(r"\target\debug\") || s.contains(r"\target\release\")
 || s.contains("/target/debug/") || s.contains("/target/release/")
```

Any installation whose path happens to contain one of these substrings (e.g.
`C:\Users\dev\target\release\nono\nono.exe`, or a deliberately crafted directory)
causes the production fail-closed broker-signature check to be **skipped**, letting
an unsigned/attacker-substituted `nono-shell-broker.exe` run at the caller's
identity and self-degrade to Low IL. This is pre-existing (the PTY `BrokerLaunch`
arm uses the same gate), so it is not a Phase 51 regression — but Phase 51 newly
relies on it for a second, profile-default-enabled spawn path, widening the blast
radius. CLAUDE.md path rules forbid string operations for security decisions.

**Fix:**
Replace the substring heuristic with a positive, robust dev-build signal:
canonicalize the path and compare path components against a `target/{debug,release}`
ancestor *and* require `cfg!(debug_assertions)` or an explicit, audited dev marker;
or gate the skip on a build-time feature flag rather than the runtime install path.
Track as hardening even if deferred — the new arm should not silently inherit a
string-based trust gate.

### WR-04: Write-deny integration test asserts an exact broker exit code that couples to cmd.exe's ERRORLEVEL

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:3256-3262` (`assert_eq!(exit_code, 1, ...)`)

**Issue:**
The "non-vacuousness gate" asserts `exit_code == 1`, relying on `cmd.exe` setting
`ERRORLEVEL 1` when `echo x > <fixture>` cannot open the redirect target. This is
brittle: `cmd.exe`'s redirect-failure exit code is not contractually 1 across all
Windows builds/locales, and the broker propagates the child code verbatim
(`run` returns `exit_code as i32`). If a future Windows build returns a different
code for the denied redirect, the test fails as a false negative even though
write-deny still holds (the `after == b"sentinel"` assertion below it is the real
proof). Tightly pinning the intermediary's exit code makes the test fragile to
factors unrelated to the security property under test.

**Fix:**
Treat the exit code as the *vacuousness* discriminator more loosely: assert
`exit_code != 0` (child ran and the write was denied) and `exit_code != 2` (broker
did spawn the child), rather than `== 1`. Keep the `after == b"sentinel"` content
assertion as the authoritative write-deny verdict. Document that exit 2 is the
broker-internal-error sentinel (`main.rs:807`).

## Info

### IN-01: Duplicated broker-bootstrap block between `BrokerLaunch` and `BrokerLaunchNoPty`

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1631-1672` vs `1315-1349`

**Issue:**
The broker-path resolution (`current_exe` → parent → `nono-shell-broker.exe` →
`exists()` → `BrokerNotFound`) and the `is_dev_build_layout` / `verify_broker_authenticode`
block are copied near-verbatim into the new arm. Duplication invites drift — a
future hardening of one bootstrap (e.g. the WR-03 fix) could miss the other.

**Fix:**
Extract a `resolve_and_verify_broker() -> Result<PathBuf>` helper used by both arms.

### IN-02: `attr_size` from the probe `InitializeProcThreadAttributeList` is not checked for zero

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:2208-2214`

**Issue:**
The size-probe call's return value is intentionally ignored (documented Win32
idiom), but `attr_size` is then used to allocate `vec![0u8; attr_size]` with no
guard that it is non-zero. If the probe failed in an unexpected way and left
`attr_size == 0`, the subsequent real `InitializeProcThreadAttributeList` with a
zero-length buffer would fail and be caught by the `ok == 0` check — so this is not
exploitable, just slightly less defensive than ideal. Mirrors the existing
`BrokerLaunch` block, so consistent with prior code.

**Fix:**
Optional: after the probe, `debug_assert!(attr_size > 0)` or convert a zero size
into an explicit `SandboxInit` error before allocating.

### IN-03: Comment claims MIC write-deny "no DACL needed" without the arm itself enforcing IL

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1640-1644` and `1273-1274`

**Issue:**
Several comments in the new arm assert that NO_WRITE_UP write-deny "is preserved on
the Low-IL grandchild by the OS MIC pre-DACL kernel check." That enforcement
depends entirely on the broker actually constructing a Low-IL primary token
(`nono::create_low_integrity_primary_token` inside `broker::run`, `main.rs:206`).
The nono-cli arm passes a null token and trusts the broker to self-degrade; nothing
in this arm verifies the grandchild's integrity level. The security property is
correct in the current code path, but the comments overstate local guarantees —
the invariant is cross-process and rests on the (Authenticode-verified, modulo
WR-03) broker binary.

**Fix:**
Tighten the comment to state the dependency explicitly ("write-deny holds *iff* the
verified broker constructs a Low-IL token") so future readers do not assume this
arm enforces IL locally.

---

## Structural Findings (fallow)

No structural findings block was provided with this review.

---

_Reviewed: 2026-05-26_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
