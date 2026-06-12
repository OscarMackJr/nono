# Phase 68: macOS Resource-Limit Enforcement Fix - Pattern Map

**Mapped:** 2026-06-12
**Files analyzed:** 4 modified files (no new files)
**Analogs found:** 4 / 4 (all in-file analogs — this phase modifies existing files only)

---

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` | service (resource-limit applier) | request-response (pre-exec hook + watchdog) | In-file: existing `RLIMIT_AS` block in `install_pre_exec` (lines 119-125) | exact — same method, same async-signal-safe `setrlimit` pattern |
| `crates/nono-cli/src/exec_strategy.rs` — child arm (~lines 940-986) | service (supervised fork child arm) | request-response (post-fork pre-exec) | In-file: existing `MSG_RLIMIT_AS_FAIL` block (lines 952-965) | exact — same CR-01 sentinel region, same `const MSG_*: &[u8]` + `libc::write` + `libc::_exit` pattern |
| `crates/nono-cli/src/exec_strategy.rs` — parent arm (~lines 1407-1430) | service (watchdog spawn + `getpgid` logic) | event-driven (deadline watchdog) | In-file: existing WR-04 `match getpgid(Some(child))` block (lines 1409-1429) | exact — same watchdog spawn site, same `match getpgid` skip-on-err pattern |
| `crates/nono-cli/tests/resl_nix_macos.rs` | test (integration, env-gated) | request-response (binary invocation) | In-file: existing `macos_timeout_kills_at_deadline` + `run_bounded` harness (lines 27-55, 124-191) | exact — bonus D-09 `--memory` assertion copies same `run_bounded` + `host_enforcement_validated()` gate structure |

---

## Pattern Assignments

### `supervisor_macos.rs` — `install_pre_exec` closure (Direct path fix)

**Analog:** In-file `RLIMIT_AS` block at lines 119-125 in `install_pre_exec`.

**Existing RLIMIT_AS pattern to replicate for RLIMIT_NPROC** (lines 106-144):
```rust
pub(crate) fn install_pre_exec(&self, cmd: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    let memory_bytes = self.memory_bytes;
    let max_processes = self.max_processes;

    // SAFETY: pre_exec runs in the forked child, post-fork pre-exec.
    // setrlimit is async-signal-safe (POSIX). No heap allocation or locks
    // are taken inside the closure. All captured values are Copy.
    unsafe {
        cmd.pre_exec(move || -> std::io::Result<()> {
            #[cfg(target_os = "macos")]
            {
                use nix::sys::resource::{setrlimit, Resource};
                if let Some(bytes) = memory_bytes {
                    let limit: nix::libc::rlim_t = bytes;
                    setrlimit(Resource::RLIMIT_AS, limit, limit)
                        .map_err(std::io::Error::from)?;   // <-- error propagation via pre_exec Err return
                }
                if let Some(_n) = max_processes {
                    // CURRENT: tracing::warn! (the bug to fix — silent no-op)
                    tracing::warn!(
                        "--max-processes is not enforced on macOS \
                         (RLIMIT_NPROC unavailable in nix v0.31's macOS subset)"
                    );
                }
            }
            // ...
            Ok(())
        });
    }
}
```

**What to change:** Replace the `tracing::warn!` block for `max_processes` with the raw `libc::setrlimit(libc::RLIMIT_NPROC, …)` call (Pattern 1 below). The `RLIMIT_AS` block immediately above is the direct structural template — copy its shape: `if let Some(n) = max_processes { … }`, use `libc::setrlimit` + `std::io::Error::last_os_error()` on failure (since `pre_exec` returns `std::io::Result<()>`, the error is returned via `?` not via `_exit(126)` — `pre_exec` returning `Err` already aborts spawn).

**Struct field change required:** `MacosResourceLimits` must gain a `baseline_uid_count: u64` field. Pattern to follow for the field addition (lines 39-46):
```rust
#[derive(Debug)]
pub(crate) struct MacosResourceLimits {
    memory_bytes: Option<u64>,
    max_processes: Option<u32>,
    // NEW: baseline_uid_count: u64   — precomputed in parent before fork;
    // captured by Copy into the pre_exec closure.
}
```
The existing `memory_bytes` and `max_processes` fields are `Copy` types — `baseline_uid_count: u64` follows the same `Copy`-capture discipline. The SAFETY doc on `install_pre_exec` (lines 88-100) already documents this requirement: "All captured values are Copy."

**`MacosResourceLimits::new` signature change:** Must accept `baseline_uid_count: u64` as a parameter (or compute it internally via `uid_process_count()`). The existing `new` pattern (lines 62-72):
```rust
pub(crate) fn new(limits: &ResourceLimits) -> Result<Self> {
    if limits.cpu_percent.is_some() {
        return Err(NonoError::NotSupportedOnPlatform {
            feature: "cpu_percent_macos".into(),
        });
    }
    Ok(Self {
        memory_bytes: limits.memory_bytes,
        max_processes: limits.max_processes,
    })
}
```
The fail-closed pattern is already established here (`return Err(...)` — not `unwrap`, not silent). The parent-side `uid_process_count()` failure must use the same `return Err(NonoError::SandboxInit(...))` convention (see Shared Patterns below).

---

### `exec_strategy.rs` — Child arm RLIMIT_NPROC block (~lines 967-986)

**Analog:** In-file `MSG_RLIMIT_AS_FAIL` block at lines 952-965 (the RLIMIT_AS fail-closed handler immediately above the RLIMIT_NPROC no-op).

**Existing RLIMIT_AS block to replicate for RLIMIT_NPROC** (lines 940-965):
```rust
// On macOS, apply setrlimit BEFORE the sandbox is applied.
// setrlimit is async-signal-safe per POSIX. No allocation.
#[cfg(target_os = "macos")]
if macos_resource_limits.is_some() {
    use nix::sys::resource::{setrlimit, Resource};
    if let Some(bytes) = resource_limits.memory_bytes {
        let limit: nix::libc::rlim_t = bytes;
        // WR-02: fail closed — if setrlimit fails the sandbox MUST NOT
        // continue without the requested --memory enforcement.
        if setrlimit(Resource::RLIMIT_AS, limit, limit).is_err() {
            const MSG_RLIMIT_AS_FAIL: &[u8] =
                b"nono: setrlimit(RLIMIT_AS) failed in pre-exec child; aborting\n";
            // SAFETY: write and _exit are async-signal-safe; we are in
            // the post-fork child branch where heap allocation is unsafe.
            unsafe {
                libc::write(
                    libc::STDERR_FILENO,
                    MSG_RLIMIT_AS_FAIL.as_ptr().cast::<libc::c_void>(),
                    MSG_RLIMIT_AS_FAIL.len(),
                );
                libc::_exit(126);
            }
        }
    }
    if let Some(_n) = resource_limits.max_processes {
        // CURRENT (the bug): MSG_RLIMIT_NPROC_UNAVAILABLE write + continue
        // REPLACE with: libc::setrlimit(libc::RLIMIT_NPROC, …) fail-closed
        const MSG_RLIMIT_NPROC_UNAVAILABLE: &[u8] =
            b"nono: --max-processes is unavailable on macOS \
              (RLIMIT_NPROC absent from nix v0.31's macOS subset); continuing without enforcement\n";
        unsafe {
            libc::write(
                libc::STDERR_FILENO,
                MSG_RLIMIT_NPROC_UNAVAILABLE.as_ptr().cast::<libc::c_void>(),
                MSG_RLIMIT_NPROC_UNAVAILABLE.len(),
            );
        }
    }
}
```

**What to change:** Replace the `MSG_RLIMIT_NPROC_UNAVAILABLE` block with a real `libc::setrlimit(libc::RLIMIT_NPROC, …)` call following the `MSG_RLIMIT_AS_FAIL` pattern exactly. The const name must become `MSG_RLIMIT_NPROC_FAIL` (required by `resl_nix_async_signal_safety.rs` line 253 — the test names this const in its assertion that at least 11 `const MSG_*: &[u8]` declarations exist).

**CR-01 constraint:** All code between `// CR-01-CHILD-ARM-START` (line 879) and `// CR-01-CHILD-ARM-END` (line 1251) must contain zero `format!(` invocations. The `const MSG_RLIMIT_NPROC_FAIL: &[u8] = b"...";` pattern is the only permitted error message form. The existing `MSG_RLIMIT_AS_FAIL` block demonstrates exactly how to do this.

**`setpgid(0,0)` insertion point:** Must be placed in the child arm BEFORE the `macos_resource_limits.is_some()` block (i.e., before line 941) and before any `setup_child_pty` call. The earliest possible position in the CR-01 child arm is after the `#[cfg(target_os = "macos")]` cap-clone block (~line 894-901) and before the resource-limits block (~line 941). Use `libc::setpgid(0, 0)` directly (async-signal-safe). Emit `const MSG_SETPGID_FAIL: &[u8]` to stderr on failure but continue (the WR-04 skip logic in the parent provides the safety net — do not `_exit(126)` on `setpgid` failure per RESEARCH.md Open Question 1 recommendation).

---

### `exec_strategy.rs` — Parent arm watchdog spawn site (~lines 1407-1430)

**Analog:** The existing WR-04 `match getpgid(Some(child))` block at lines 1409-1429 — this is already correct and is the pattern to preserve.

**Existing WR-04 pattern to preserve** (lines 1407-1430):
```rust
#[cfg(target_os = "macos")]
let _timeout_watchdog = timeout_deadline.and_then(|deadline| {
    use nix::unistd::getpgid;
    // WR-04: Do NOT fall back to child PID on getpgid failure.
    match getpgid(Some(child)) {
        Ok(child_pgrp) => Some(supervisor_macos::spawn_macos_timeout_watchdog(
            deadline, child_pgrp,
        )),
        Err(e) => {
            warn!(
                "getpgid({}) failed ({}); skipping timeout watchdog — \
                     no PID fallback to avoid wrong-pgrp kill under PID reuse",
                child.as_raw(),
                e
            );
            None
        }
    }
});
```

**What changes here:** This block is NOT changed. The fix is upstream: `setpgid(0,0)` in the child arm ensures that `getpgid(Some(child))` in this block returns the child's own dedicated pgrp (child_pid == child_pgrp after `setpgid(0,0)`). The `kill(-child_pgrp, SIGKILL)` in `spawn_macos_timeout_watchdog` then correctly targets only the agent tree.

**`apply_resource_limits_unix` dispatch (~lines 90-112):** The parent call to `MacosResourceLimits::new` must pass the precomputed `baseline_uid_count`. The `uid_process_count()` helper (new function in `supervisor_macos.rs`) must be called in the parent before `apply_resource_limits_unix`. Pattern: same fail-closed `return Err(...)` the existing function uses for `cpu_percent`.

---

### `crates/nono-cli/tests/resl_nix_macos.rs` — D-09 bonus `--memory` assertion

**Analog:** In-file `macos_timeout_kills_at_deadline` test (lines 124-191) — same structure: `host_enforcement_validated()` gate, `run_bounded` harness, assert `!output.status.success()`, elapsed-window assertion.

**Gate pattern to copy** (lines 125-133):
```rust
fn macos_timeout_kills_at_deadline() {
    if !host_enforcement_validated() {
        eprintln!(
            "SKIP macos_timeout_kills_at_deadline: ..."
        );
        return;
    }
    // ...
}
```

**`run_bounded` harness** (lines 27-55) — reuse as-is. No changes to the harness.

**Existing test invocation pattern** (lines 266-286):
```rust
let output = run_bounded(
    &[
        "run",
        "--max-processes",
        "5",
        "--read=/bin",
        "--read=/usr",
        "--read=/private",
        "--",
        "bash",
        "-c",
        "for i in $(seq 1 20); do sleep 5 & done; wait",
    ],
    Duration::from_secs(20),
);
```
The new D-09 `--memory` test follows the same `run_bounded` call shape with `--memory 32m` and a memory-heavy child (e.g., `python3 -c "x=[0]*10**9"`), bounded at 10s.

---

## Shared Patterns

### Async-Signal-Safe Child Arm Error Reporting
**Source:** `crates/nono-cli/src/exec_strategy.rs` lines 952-965 (`MSG_RLIMIT_AS_FAIL` block)
**Apply to:** All new error-reporting points inside the CR-01 child arm (RLIMIT_NPROC fail, `setpgid` fail)

```rust
const MSG_RLIMIT_AS_FAIL: &[u8] =
    b"nono: setrlimit(RLIMIT_AS) failed in pre-exec child; aborting\n";
// SAFETY: write and _exit are async-signal-safe; we are in
// the post-fork child branch where heap allocation is unsafe.
unsafe {
    libc::write(
        libc::STDERR_FILENO,
        MSG_RLIMIT_AS_FAIL.as_ptr().cast::<libc::c_void>(),
        MSG_RLIMIT_AS_FAIL.len(),
    );
    libc::_exit(126);
}
```

Rules:
- Const name must match `const MSG_*: &[u8]` (the count assertion at `resl_nix_async_signal_safety.rs:243` requires ≥ 11 such declarations; `MSG_RLIMIT_NPROC_FAIL` is specifically named at line 253).
- Use `libc::write` + `libc::_exit(126)` for fatal errors (matches `MSG_RLIMIT_AS_FAIL`).
- Use `libc::write` without `_exit` for non-fatal warnings (matches `MSG_RLIMIT_NPROC_UNAVAILABLE` current pattern — replicate for `setpgid` failure).
- No `format!()`, no `String`, no `Vec`, no `tracing::warn!` inside the CR-01 child arm.

### Parent-Side Fail-Closed Error Pattern
**Source:** `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` lines 62-67 (`MacosResourceLimits::new`)
**Apply to:** `uid_process_count()` helper function; parent-side baseline computation failure

```rust
pub(crate) fn new(limits: &ResourceLimits) -> Result<Self> {
    if limits.cpu_percent.is_some() {
        return Err(NonoError::NotSupportedOnPlatform {
            feature: "cpu_percent_macos".into(),
        });
    }
    // ...
}
```

For `uid_process_count()` failure, use `NonoError::SandboxInit(format!("sysctl(KERN_PROC_UID) failed: {}", std::io::Error::last_os_error()))` — the `SandboxInit` variant is the established pattern for "sandbox setup failed before child was spawned" (confirmed in `crates/nono/src/error.rs` line 54).

### WR-04 `getpgid` Skip Pattern
**Source:** `crates/nono-cli/src/exec_strategy.rs` lines 1409-1429
**Apply to:** Preserve exactly. The `wr_04_no_pid_fallback_on_getpgid_failure` test at `resl_nix_async_signal_safety.rs:308-323` asserts:
1. `src` does NOT contain `"unwrap_or(child)"`.
2. `src` DOES contain `"match getpgid("`.

Do not change the watchdog spawn site structure — only `setpgid(0,0)` in the child arm makes the existing `match getpgid` pattern reliable.

### `Copy`-Capture Discipline for `pre_exec` Closures
**Source:** `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` lines 107-113 (capture setup before `unsafe { cmd.pre_exec(...) }`)
**Apply to:** `install_pre_exec` closure when `baseline_uid_count: u64` field is added

```rust
let memory_bytes = self.memory_bytes;
let max_processes = self.max_processes;
// NEW: let baseline_uid_count = self.baseline_uid_count;
// All three are Copy types (Option<u64>, Option<u32>, u64).
// No references, no Arc, no allocation inside the closure.
```

### `NonoError` Variants for This Phase
**Source:** `crates/nono/src/error.rs`

| Failure site | Variant to use |
|---|---|
| `uid_process_count()` sysctl failure (parent) | `NonoError::SandboxInit(String)` |
| `cpu_percent` defense-in-depth (existing) | `NonoError::NotSupportedOnPlatform { feature }` |
| `setrlimit` failure in child arm | `libc::_exit(126)` with `const MSG_*: &[u8]` (cannot use `NonoError` in child arm — no allocation) |
| `setpgid` failure in child arm | `libc::write(stderr)` + continue (non-fatal; WR-04 provides safety net) |

---

## No Analog Found

None. All files modified in this phase have strong in-file analogs for every change point.

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/exec_strategy/`, `crates/nono-cli/tests/`
**Files scanned:** 5 (supervisor_macos.rs, exec_strategy.rs, supervisor_linux.rs, resl_nix_macos.rs, resl_nix_async_signal_safety.rs)
**Pattern extraction date:** 2026-06-12

---

## Critical Constraints Summary (for planner)

1. **CR-01 sentinel:** All child-arm code lives between `// CR-01-CHILD-ARM-START` (exec_strategy.rs line 879) and `// CR-01-CHILD-ARM-END` (line 1251). Zero `format!(` calls anywhere in that region — tested by `cr_01_no_format_macro_in_post_fork_child_branch`.

2. **`MSG_RLIMIT_NPROC_FAIL` const required:** `resl_nix_async_signal_safety.rs` line 243 expects ≥ 11 `const MSG_*: &[u8]` in `exec_strategy.rs`; line 253 names `MSG_RLIMIT_NPROC_FAIL` in its assertion message. The const MUST be added.

3. **WR-02 no silent discards:** `wr_02_no_silent_setrlimit_discards` (line 327) asserts zero `let _ = setrlimit(...)` patterns in `exec_strategy.rs`. All `setrlimit` calls must check the return value.

4. **WR-04 preserved:** `match getpgid(Some(child))` pattern at lines 1409-1429 must not be altered structurally.

5. **`baseline_uid_count` must be used in both paths:** CLAUDE.md prohibits `#[allow(dead_code)]`. The new `baseline_uid_count` field must be used in both the `install_pre_exec` closure (Direct path) and in the exec_strategy.rs child arm (Supervised path) — otherwise clippy fails.

6. **Cross-target clippy deferred to CI:** Windows dev host cannot run `--target x86_64-unknown-linux-gnu` or `--target x86_64-apple-darwin` clippy. Per CLAUDE.md and `.planning/templates/cross-target-verify-checklist.md`, mark cross-target REQs PARTIAL/deferred-to-CI.

7. **`setpgid(0,0)` placement:** Must be in the CR-01 child arm before the `macos_resource_limits.is_some()` block. On the PTY path, `setup_child_pty` calls `setsid()` which supersedes `setpgid` — both orderings are safe. On the non-PTY path, `setpgid(0,0)` is the only group-isolation mechanism and must be present.
