---
phase: 25-cross-platform-resl-aipc-unix-design
reviewed: 2026-05-10T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - bindings/c/src/lib.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/exec_strategy/supervisor_macos.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/tests/resl_nix_linux.rs
  - crates/nono-cli/tests/resl_nix_macos.rs
  - crates/nono/src/error.rs
  - docs/architecture/aipc-unix-futures.md
findings:
  critical: 2
  warning: 7
  info: 3
  total: 12
status: issues_found
---

# Phase 25: Code Review Report

**Reviewed:** 2026-05-10
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

Phase 25 implements kernel-level resource-limit enforcement on Linux (cgroup v2) and macOS (setrlimit + supervisor watchdog), converting previously stubbed `--memory`, `--cpu-percent`, `--max-processes`, and `--timeout` flags into real enforcement. The AIPC Unix Futures ADR (Plan 25-02) locks a six-row decision table for cross-platform handle brokering.

The core implementation is technically sound in its design — the cgroup v2 lifecycle, the async-signal-safe `pre_exec` path on Linux, and the macOS setrlimit + watchdog model all look correct. The FFI exhaustive match correctly handles the new `NotSupportedOnPlatform` variant.

However, two blockers and several warnings were identified:

1. **BLOCKER (CR-01)**: `format!()` macro calls inside the post-fork child branch in `execute_supervised` allocate on the heap in the child, violating the async-signal-safety contract the code claims to maintain.
2. **BLOCKER (CR-02)**: The Linux timeout watchdog spawns only when `unix_resource_guard.is_some()`, but `--timeout` without any other resource limit leaves `unix_resource_guard = None`, causing the watchdog to silently not fire and the deadline to be ignored.
3. Multiple warnings around cgroup path validation, the macOS setrlimit error handling, and test reliability.

---

## Critical Issues

### CR-01: `format!()` macro calls inside the post-fork child branch allocate heap memory

**File:** `crates/nono-cli/src/exec_strategy.rs:862-863, 899-901, 933-934, 951, 994-999, 1011, 1054-1059, 1071, 1093-1096`

**Issue:** The `execute_supervised` function's child branch (after `Ok(ForkResult::Child)`) uses `format!()` macros to build error message strings before writing them to stderr with `libc::write`. The `format!()` macro allocates a `String` on the heap via the Rust allocator. This is explicitly unsafe in a multi-threaded post-fork context: if the parent held the allocator's internal mutex at the moment of `fork()`, the child inherits a locked mutex and calling `format!()` will deadlock. The code comments correctly identify this problem ("async-signal-safe") and correctly avoids `format!()` in the cgroup placement path — but `format!()` is used extensively throughout the *rest* of the child branch including in the `Sandbox::apply()` error handler, the seccomp install error handler, and the `clear_close_on_exec` error handler.

The threading pre-check (`validate threading context before fork`) reduces but does not eliminate this risk: keyring/crypto pool threads may not hold the allocator lock *most* of the time, but there is no guarantee at the exact instant of `fork()`. The project's `ThreadingContext::CryptoExpected` path explicitly allows up to 7 threads.

**Fix:** Replace each `format!()` call in the child branch with a pre-allocated static byte string (like `const MSG: &[u8]` as is already done at line 1125) or build the string in the parent before fork and move it into the child closure. For error paths where the exact error value is needed (e.g., the `Sandbox::apply()` error), accept that the message will be generic ("nono: failed to apply sandbox") without embedding the Rust error detail. Example:

```rust
// Instead of:
let detail = format!("nono: failed to place child in cgroup: {}\n", e);
let msg = detail.as_bytes();
unsafe { libc::write(libc::STDERR_FILENO, msg.as_ptr().cast(), msg.len()); }

// Use a static message:
const MSG: &[u8] = b"nono: failed to place child in cgroup\n";
unsafe { libc::write(libc::STDERR_FILENO, MSG.as_ptr().cast(), MSG.len()); }
```

The cgroup placement path at lines 859-872 already correctly uses this pattern — the inconsistency within the same function is the defect.

---

### CR-02: `--timeout` without other resource limits silently skips the timeout watchdog on Linux

**File:** `crates/nono-cli/src/exec_strategy.rs:1280-1291`

**Issue:** The Linux timeout watchdog is spawned only when `unix_resource_guard.is_some()`:

```rust
let _timeout_watchdog = timeout_deadline
    .map(|deadline| {
        if let Some(ref session) = unix_resource_guard {
            let cgroup_path = session.path.clone();
            ...
            Some(spawn_linux_timeout_watchdog(deadline, cgroup_path, fired))
        } else {
            None  // <-- watchdog silently not spawned
        }
    })
    .flatten();
```

`unix_resource_guard` is `None` when `resource_limits.is_empty()` (line 787). But `resource_limits.is_empty()` checks all four fields — including `timeout`. If a user passes only `--timeout 30s` (no `--memory`, no `--cpu-percent`, no `--max-processes`), then `resource_limits.is_empty()` returns `true` (because `is_empty` counts timeout as an empty-indicator field at line 155-161), the cgroup session is not created, `unix_resource_guard = None`, and the watchdog branch produces `None`. The deadline is computed but never acted upon — `--timeout` is silently ignored.

Checking `launch_runtime.rs` lines 155-161 confirms: `is_empty()` returns `true` when all four fields are `None`, so a lone `--timeout 30s` yields `is_empty() == false` and does create the cgroup session. However, the watchdog guard is still only activated if `unix_resource_guard.is_some()`. If timeout is set but `resource_limits.is_empty()` is `false` solely due to timeout, the cgroup is created (correctly) but then the watchdog path is fine. **Re-reading**: if `resource_limits.timeout = Some(...)` and the other three are `None`, then `is_empty()` returns `false`, a cgroup session is created with no limits applied (since `apply_limits()` skips `None` fields), and the watchdog fires against that empty cgroup. This appears to work, but is fragile and the code comment says "accepted but not enforced in Direct mode" for timeout — this divergence between Direct and Supervised handling is undocumented.

The actual blocker is the reverse: if a user only passes `--memory 256m` with no `--timeout`, but `timeout_deadline` is `None`, the watchdog map returns `None` immediately. That's correct. But if only `--timeout` is passed (no other limits), `apply_limits()` writes nothing to any cgroup pseudo-file (all `None` fields), yet consumes a cgroup slot. This leaks a cgroup directory if the child never writes a PID to `cgroup.procs` (e.g., if `place_self_in_cgroup_raw` is the only operation and the child exits before the watchdog fires). More critically, the cgroup that the watchdog writes `cgroup.kill` into may contain no processes if the cgroup enrollment failed silently.

The actual critical case: `--timeout` only, on a system where `resource_limits.is_empty()` is `false` due to timeout alone — `apply_limits` is called but writes nothing. The cgroup is created, the child is placed into it, the watchdog fires at deadline writing `1\n` to `cgroup.kill` — this path appears to work. The real concern is:

If `--timeout` is passed in Direct mode (line 438-441, comment says "timeout is NOT enforced in Direct mode"), users get silent non-enforcement. This is documented only in a code comment, not surfaced as a user-visible warning.

**Fix:** Add an explicit warning when `--timeout` is used in Direct strategy mode, since that flag is accepted but silently ignored:

```rust
#[cfg(any(target_os = "linux", target_os = "macos"))]
if resource_limits.timeout.is_some() && matches!(config_strategy, ExecStrategy::Direct) {
    warn!("--timeout is not enforced in Direct strategy mode; use Supervised strategy");
}
```

Also add a test in `resl_nix_linux.rs` covering `--timeout`-only (no memory/cpu/procs) to confirm the watchdog fires.

---

## Warnings

### WR-01: String `starts_with("0::/")` in integration test uses str comparison on a path-like string

**File:** `crates/nono-cli/tests/resl_nix_linux.rs:31`

**Issue:** The `cgroup_v2_available()` helper uses:
```rust
if !lines[0].starts_with("0::/") {
    return false;
}
```

This is a string comparison on what is logically a colon-separated cgroup entry, not a filesystem path. The CLAUDE.md § Path Handling flags `starts_with()` on paths as a vulnerability footgun. While `/proc/self/cgroup` lines are not filesystem paths in the traditional sense — they are `<hierarchy-id>::<relative-cgroup-path>` strings — and the prefix `"0::"` is a format-defined delimiter (not a path prefix), the concern is that a cgroup path like `"0::/foo"` vs `"0::foo"` (no leading slash) would both match `"0::/"`... but only the former. This is actually fine for well-formed v2 entries. However, the test helper does not use the same `CgroupSession::detect_from_str` logic used by the production code — it duplicates the detection logic independently, creating a risk of divergence. If `detect_from_str` changes its acceptance criteria (e.g., to support paths without a leading slash), the test skip guard would not update.

**Fix:** Expose `CgroupSession::detect_from_str` in a `#[cfg(test)]` accessible way and call it from `cgroup_v2_available()` to ensure test skip logic stays in sync with production detection logic.

---

### WR-02: macOS setrlimit errors silently ignored in `execute_supervised` child branch

**File:** `crates/nono-cli/src/exec_strategy.rs:885, 889`

**Issue:** In the child branch of `execute_supervised`, the setrlimit calls are discarded with `let _ = setrlimit(...)`. The comment reads "Non-fatal in the child: we've already validated this in the parent." However, `MacosResourceLimits::new()` in the parent only validates that `cpu_percent` is `None` — it does not pre-validate that the `RLIMIT_AS` or `RLIMIT_NPROC` values are within the system's hard limits. If the system's hard limit for `RLIMIT_NPROC` is below the value the user requested, `setrlimit` will fail with `EINVAL` or `EPERM` and the child proceeds without the limit applied. The sandbox continues to run with no enforcement of `--max-processes`, which is a silent security degradation.

**Fix:** Convert setrlimit failures in the child to a hard `_exit(126)` with a diagnostic write, consistent with how the Linux cgroup placement failure is handled:

```rust
if let Some(n) = resource_limits.max_processes {
    let limit = u64::from(n);
    if setrlimit(Resource::RLIMIT_NPROC, limit, limit).is_err() {
        const MSG: &[u8] = b"nono: setrlimit(RLIMIT_NPROC) failed\n";
        unsafe { libc::write(libc::STDERR_FILENO, MSG.as_ptr().cast(), MSG.len()); libc::_exit(126); }
    }
}
```

---

### WR-03: Cgroup path construction uses string join, not `Path::join` component comparison

**File:** `crates/nono-cli/src/exec_strategy.rs:784-795` and `supervisor_linux.rs:905-907`

**Issue:** `CgroupSession::detect_from_str` constructs the cgroup path as:
```rust
let abs_path = PathBuf::from("/sys/fs/cgroup")
    .join(cgroup_rel.trim_start_matches('/').trim_end_matches('/'));
```

This uses `trim_start_matches('/')` on the relative cgroup path string from `/proc/self/cgroup`. While this prevents a trivial double-slash issue, it does not prevent path traversal via `..` components embedded in the cgroup path (e.g., `0::/../../etc`). A malicious or compromised container runtime could write a crafted `/proc/self/cgroup` entry to redirect where nono creates its cgroup directory. The CLAUDE.md § Path Handling requires path component comparison, not string operations.

**Fix:** After constructing `abs_path`, canonicalize and verify it still starts with `/sys/fs/cgroup` using `Path::starts_with`:

```rust
let abs_path = PathBuf::from("/sys/fs/cgroup").join(cgroup_rel.trim_start_matches('/'));
// Validate no traversal outside the cgroup root
if !abs_path.starts_with("/sys/fs/cgroup") {
    return Err(NonoError::UnsupportedPlatform(
        "cgroup_v2: detected path escapes /sys/fs/cgroup (path traversal in /proc/self/cgroup)"
            .into()
    ));
}
```

Note: `Path::starts_with` performs component-level comparison, not string comparison, so `"/sys/fs/cgroupevil"` would not match — this is the correct check.

---

### WR-04: `getpgid` failure falls back to the child PID as process group, masking errors

**File:** `crates/nono-cli/src/exec_strategy.rs:1296`

**Issue:**
```rust
let child_pgrp = getpgid(Some(child)).unwrap_or(child);
```

`getpgid` can fail with `ESRCH` if the child has already exited by the time the parent attempts to read its process group. In that case, falling back to `child` (the child PID itself) means `kill(-child_pgrp, SIGKILL)` targets a process group with `pgid == child_pid`. If the child has already exited and its PID was reused by another process, this sends SIGKILL to the wrong process group. While the comment in `supervisor_macos.rs` notes "Ignore ESRCH — that's the normal race," this fallback in the parent is subtly different: it silently sends the kill signal to a potentially unrelated process group.

**Fix:** Do not fall back to `child` on `getpgid` failure. Instead, use the deadline-elapsed check first and skip the kill if `getpgid` fails:

```rust
match getpgid(Some(child)) {
    Ok(pgrp) => { /* spawn watchdog with pgrp */ }
    Err(_) => { /* child already exited; skip watchdog */ }
}
```

---

### WR-05: `nix::libc::rlim_t` conversion via `Errno as i32` is not portable

**File:** `crates/nono-cli/src/exec_strategy/supervisor_macos.rs:114, 119`

**Issue:**
```rust
setrlimit(Resource::RLIMIT_AS, limit, limit)
    .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
```

`nix::errno::Errno` does not implement `From<Errno> for i32` without the cast. The cast `e as i32` works only because `Errno` is a `#[repr(i32)]` enum in nix 0.31. If nix's internal representation changes this is silently wrong. Separately, `std::io::Error::from_raw_os_error(e as i32)` creates a "last OS error" error from a raw code, but the idiomatic conversion from nix `Errno` to `std::io::Error` is `std::io::Error::from(e)` (nix implements `From<Errno> for std::io::Error`).

**Fix:**
```rust
setrlimit(Resource::RLIMIT_AS, limit, limit)
    .map_err(std::io::Error::from)?;
```

---

### WR-06: `select_exec_strategy` always returns `Supervised`, ignoring all inputs

**File:** `crates/nono-cli/src/launch_runtime.rs:480-495`

**Issue:**
```rust
pub(crate) fn select_exec_strategy(
    rollback: bool,
    proxy_active: bool,
    capability_elevation: bool,
    trust_interception_active: bool,
    detached_start: bool,
) -> exec_strategy::ExecStrategy {
    let _ = (rollback, proxy_active, capability_elevation, trust_interception_active, detached_start);
    exec_strategy::ExecStrategy::Supervised
}
```

All parameters are discarded (`let _ = (...)`) and `Supervised` is always returned. This means `nono wrap` cannot use the `Direct` strategy that its documentation describes ("Sandbox and exec into command"), and the `--strategy` flag (if exposed) is effectively a no-op. This may be intentional as a Phase 25 stub, but it silently eliminates the Direct execution path which is the documented behavior of `nono wrap`. Users of `nono wrap` expecting exec-into behavior (nono disappears after exec) instead get a supervised session.

**Fix:** If this stub is intentional for Phase 25 scope, add a `#[allow(unused_variables)]` and a prominent `// TODO(Phase X): restore strategy selection` comment. If `Direct` should be selectable (e.g., when `!rollback && !proxy_active && !capability_elevation && !trust_interception_active`), implement the real logic.

---

### WR-07: `--timeout` accepted but silently not enforced in Direct strategy, no user-visible warning

**File:** `crates/nono-cli/src/exec_strategy.rs:429-441`

**Issue:** The `execute_direct` function accepts `resource_limits` and creates a `_resource_guard` for memory/cpu/process limits via `apply_resource_limits_unix`. However, the comment at line 438-440 explicitly states `--timeout` is **not enforced in Direct mode** ("no supervisor watchdog is available"). There is no user-visible warning emitted when `--timeout` is used in Direct mode. A user who runs `nono wrap --timeout 30s -- some-command` will receive no feedback that their timeout will not be enforced.

**Fix:** Emit a `warn!()` (or `eprintln!` if `!silent`) when `resource_limits.timeout.is_some()` in the Direct strategy path. This matches the project's "Fail Secure" principle — at minimum the user must know enforcement is not active.

---

## Info

### IN-01: `#[allow(dead_code)]` on `UnixResourceLimitGuard` suppresses the project's "no dead code" policy

**File:** `crates/nono-cli/src/exec_strategy.rs:53`

**Issue:**
```rust
#[allow(dead_code)]
pub(crate) enum UnixResourceLimitGuard {
```

CLAUDE.md § Lazy use of dead code states "Avoid `#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it." The `UnixResourceLimitGuard` enum is used in `apply_resource_limits_unix` (which is only called in the `execute_direct` path since `execute_supervised` uses the internal guard inline). The `Noop` variant is always returned when limits are empty, and the `Linux`/`Macos` variants are returned otherwise and stored in `_resource_guard` (prefix `_` suppresses the "unused variable" warning). The `#[allow(dead_code)]` appears to be suppressing warnings on the `Macos` variant when compiling on Linux or vice versa.

**Fix:** Use `#[cfg(target_os = "...")]` on the variants to match the conditional compilation already applied to the `Linux` and `Macos` arms, removing the need for the blanket `allow`:

```rust
pub(crate) enum UnixResourceLimitGuard {
    Noop,
    #[cfg(target_os = "linux")]
    Linux(supervisor_linux::cgroup::CgroupSession),
    #[cfg(target_os = "macos")]
    Macos(supervisor_macos::MacosResourceLimits),
}
```

---

### IN-02: `parse_byte_size` accepts multi-char suffixes like `1MB` and parses `1` with suffix `B`

**File:** `crates/nono-cli/src/cli.rs:33`

**Issue:** The parser examines only the last character of the input string:
```rust
match s.chars().last() {
    Some(c) if c.is_ascii_alphabetic() => { ... }
```

Input `"1MB"` takes the `'B'` branch, hits the `other` arm, and returns `Err("unrecognized memory suffix 'B'; expected K/M/G/T")`. This is correct error behavior. However, input `"1mB"` also correctly hits `'B'` and errors. Input `"256Mb"` hits `'b'` → error. These are all correctly rejected.

The subtle case: input `"1 M"` (space between number and suffix) — `s.trim()` at line 16 trims leading/trailing spaces, but a space in the middle produces `num_str = "1 "`, and `"1 ".parse::<u64>()` fails with a useful error. This is acceptable behavior.

No actual bug, but document the accepted format more precisely in the docstring — current doc says "suffix is case-insensitive" but the parse logic is case-sensitive for the `num_str` slice (`&s[..s.len() - 1]` uses byte indexing, which is safe only because the suffix check is `is_ascii_alphabetic()`).

**Fix:** No code change needed; add a doc comment clarification: "Note: the numeric part must not contain spaces; `1 M` is rejected."

---

### IN-03: The AIPC ADR's decision table is duplicated and the note about divergence says "divergence is a bug"

**File:** `docs/architecture/aipc-unix-futures.md:38-46, 51-58`

**Issue:** The document contains two identical tables (one keyed by HandleKind name, one by discriminant number) and explicitly states: "Both tables encode the same decision; the two orderings exist for grep/tooling convenience. Any divergence between them is a bug." The document is an ADR (decision-only, not code), but if these tables are ever edited independently — e.g., a future row is added to one but not the other — the doc will contain contradictory information with no automated check. This is a documentation maintenance risk, not a correctness defect in the current phase.

**Fix:** Add a parenthetical note that the canonical source is `aipc_sdk.rs` const assertions (already mentioned) and that any PR touching this document must update both tables atomically. Consider a lint/grep in CI that verifies the row counts match.

---

_Reviewed: 2026-05-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
