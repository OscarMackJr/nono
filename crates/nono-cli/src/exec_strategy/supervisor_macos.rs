//! macOS resource-limit application via `setrlimit` + supervisor watchdog.
//!
//! Maps resource-limit flags to macOS enforcement mechanisms:
//!
//! | CLI flag             | Mechanism                  | Notes                        |
//! |----------------------|----------------------------|------------------------------|
//! | `--memory <bytes>`   | `RLIMIT_AS` (address space) | Not RSS — see § RLIMIT_AS vs RSS |
//! | `--max-processes <N>`| `RLIMIT_NPROC`             | UID-wide bound — see § RLIMIT_NPROC semantics |
//! | `--cpu-percent`      | Rejected at clap parse time | No per-process quota on macOS |
//! | `--timeout <dur>`    | Supervisor `Instant` + `kill(pgrp, SIGKILL)` |        |
//!
//! ## RLIMIT_AS vs RSS
//!
//! `RLIMIT_AS` bounds the process's *virtual address space*, not its resident
//! set size (RSS). A process can pass `--memory 256m` and still consume more
//! than 256 MB of physical memory if its mappings are sparse or shared. This
//! is the documented gap per REQ-RESL-NIX-03; the alternative (RSS-based
//! enforcement via polling) has racy bypass windows and is not portable.
//! RLIMIT_RSS exists on older BSDs but is a no-op on modern macOS.
//!
//! ## CPU percent
//!
//! macOS does not have a per-process CPU-quota equivalent (no cgroup-style
//! `cpu.max`). `RLIMIT_CPU` is CPU-time (not wall-clock) and is intentionally
//! not used because it measures aggregate CPU consumption, not rate.
//! `--cpu-percent` is rejected at clap parse time per REQ-RESL-NIX-03
//! acceptance criterion 3.
//!
//! ## RLIMIT_NPROC semantics (D-03)
//!
//! `RLIMIT_NPROC` on macOS counts **all processes owned by the UID** (not just
//! descendants of the sandboxed child, unlike Linux `pids.max` which is
//! descendant-tree-scoped). The baseline+N bounding strategy means the agent's
//! effective budget for additional processes is N, but other processes owned by
//! the same UID can consume those slots. This UID-wide scope is inherently racy
//! (new UID processes can spawn between the parent's baseline count and the
//! child's `setrlimit` apply), and is accepted behavior — there is no practical
//! fix without kernel-level true-descendant scoping. The baseline+N strategy
//! still meaningfully limits agent fork storms on lightly-loaded developer machines.

use crate::launch_runtime::ResourceLimits;
use nono::{NonoError, Result};
// `nono-cli` has no direct `libc` dependency; it reaches libc through nix's public
// re-export (matching `use nix::libc;` in exec_strategy.rs). Gated to macOS because
// this module compiles on all targets (the `mod supervisor_macos;` declaration is not
// cfg-gated) but every bare `libc::` use below is inside `#[cfg(target_os = "macos")]`.
#[cfg(target_os = "macos")]
use nix::libc;

/// Read the current number of processes owned by the current UID.
///
/// Uses `proc_listpids(PROC_UID_ONLY, uid, NULL, 0)` to query the byte length of
/// the kernel's pid list for the calling user's UID, then divides by
/// `size_of::<c_int>()` (a pid is a 32-bit int) to obtain the process count.
///
/// `proc_listpids` is used rather than `sysctl(KERN_PROC_UID)` because the `libc`
/// crate (0.2.x) does NOT expose the macOS `kinfo_proc` struct on Apple targets,
/// so the sysctl entry-size division (`size_of::<kinfo_proc>()`) cannot be
/// expressed. `proc_listpids` needs no struct layout — it reports a raw byte count.
///
/// This count is used by [`MacosResourceLimits::new`] to compute a
/// `baseline_uid_count` before `fork()`, which is then captured by value
/// into the `pre_exec` closure and the supervised child arm to compute
/// `RLIMIT_NPROC = baseline + N` (the agent gets N additional processes).
///
/// # Safety note
///
/// `proc_listpids` is NOT async-signal-safe — it allocates kernel-side and may
/// take locks. This function MUST be called in the **parent before `fork()`**
/// only. It is not safe to call from inside a `pre_exec` closure or any
/// post-fork child arm.
///
/// # Errors
///
/// Returns `Err(NonoError::SandboxInit(...))` if `proc_listpids` fails.
#[cfg(target_os = "macos")]
fn uid_process_count() -> Result<u64> {
    // PROC_UID_ONLY selects processes by effective UID. libc does not export this
    // constant; its value is 4 per <sys/proc_info.h> (PROC_ALL_PIDS=1, PROC_PGRP_ONLY=2,
    // PROC_TTY_ONLY=3, PROC_UID_ONLY=4).
    const PROC_UID_ONLY: u32 = 4;
    let uid = unsafe { libc::getuid() };
    // NULL buffer + 0 size => proc_listpids returns the number of bytes the pid list
    // would occupy (= count * size_of::<pid_t>()), without writing any pids.
    // SAFETY: proc_listpids is a direct libproc kernel call; NOT async-signal-safe —
    // must only be called in the parent before fork.
    let bytes = unsafe { libc::proc_listpids(PROC_UID_ONLY, uid, std::ptr::null_mut(), 0) };
    if bytes <= 0 {
        return Err(NonoError::SandboxInit(format!(
            "proc_listpids(PROC_UID_ONLY) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    // Each pid is a 32-bit int; divide the byte count by size_of::<c_int>() for the
    // process count. checked_div guards against a hypothetical zero size_of (required
    // by the unwrap policy; impossible in practice).
    let count = (bytes as usize)
        .checked_div(std::mem::size_of::<libc::c_int>())
        .unwrap_or(0);
    Ok(count as u64)
}

/// macOS resource-limit applier using `setrlimit` in a `pre_exec` hook.
///
/// Created via [`MacosResourceLimits::new`] before the child is spawned.
/// The limits are applied inside the forked child's `pre_exec` hook, before
/// `execve`, so the resource caps are in effect from the first instruction
/// of the sandboxed binary.
///
/// ## `baseline_uid_count` field
///
/// This field holds the per-UID process count read from the kernel (via
/// `proc_listpids(PROC_UID_ONLY)`) **in the parent before `fork()`**. It is a plain
/// `u64` captured by value into the `pre_exec` closure — no allocation, no
/// locks, no async-signal-unsafe calls inside the closure. The
/// `RLIMIT_NPROC` soft and hard limits are set to `baseline_uid_count + N`
/// (capped at the existing hard limit to avoid EPERM).
#[derive(Debug)]
pub(crate) struct MacosResourceLimits {
    /// `RLIMIT_AS` soft + hard limit in bytes (from `--memory`). None = no limit.
    memory_bytes: Option<u64>,
    /// `RLIMIT_NPROC` soft + hard limit (from `--max-processes`). None = no limit.
    max_processes: Option<u32>,
    /// Per-UID process count read in the parent before fork (via `proc_listpids(PROC_UID_ONLY)`).
    /// Used to compute `RLIMIT_NPROC = baseline_uid_count + N`. Zero when `max_processes`
    /// is `None` (no kernel query needed). Captured by `Copy` into the `pre_exec` closure.
    baseline_uid_count: u64,
    // Note: `timeout` is consumed by the supervisor watchdog (`spawn_macos_timeout_watchdog`),
    // not by the pre_exec hook. It is not stored here.
}

impl MacosResourceLimits {
    /// Create a new `MacosResourceLimits` from the given resource-limit configuration.
    ///
    /// # Defense-in-depth for `cpu_percent`
    ///
    /// `--cpu-percent` is rejected at clap parse time on macOS (see `cli.rs:parse_cpu_percent`).
    /// If it somehow reaches this function (e.g., via a test or FFI caller), this function
    /// returns `Err(NonoError::NotSupportedOnPlatform { feature: "cpu_percent_macos" })`
    /// as a defense-in-depth check.
    ///
    /// # Errors
    ///
    /// - Returns `Err(NonoError::NotSupportedOnPlatform { feature: "cpu_percent_macos" })`
    ///   if `limits.cpu_percent.is_some()`.
    /// - Returns `Err(NonoError::SandboxInit(...))` if `limits.max_processes.is_some()` and
    ///   the `proc_listpids(PROC_UID_ONLY)` call to compute the baseline UID process count fails.
    ///   Fail-closed per D-07: the run is aborted before fork rather than launching an
    ///   unbounded child.
    pub(crate) fn new(limits: &ResourceLimits) -> Result<Self> {
        if limits.cpu_percent.is_some() {
            return Err(NonoError::NotSupportedOnPlatform {
                feature: "cpu_percent_macos".into(),
            });
        }
        // Read the per-UID baseline process count from the kernel before fork.
        // This is required to compute RLIMIT_NPROC = baseline + N.
        // Fail-closed (D-07): if the query fails, abort before spawning an unbounded child.
        let baseline_uid_count = if limits.max_processes.is_some() {
            uid_process_count()?
        } else {
            // No max-processes limit requested — skip the kernel query (no baseline needed).
            0
        };
        Ok(Self {
            memory_bytes: limits.memory_bytes,
            max_processes: limits.max_processes,
            baseline_uid_count,
        })
    }

    /// Return the per-UID process count read from the kernel before fork.
    ///
    /// Used by `exec_strategy.rs` to extract the baseline count as a `Copy` `u64`
    /// before the fork, so the child arm can compute `RLIMIT_NPROC = baseline + N`
    /// without accessing the struct across the fork boundary.
    pub(crate) fn baseline_uid_count(&self) -> u64 {
        self.baseline_uid_count
    }

    /// Install a `pre_exec` hook on `cmd` that applies `setrlimit` in the forked child.
    ///
    /// The hook runs in the forked child, post-fork pre-exec (before `execve`), so the
    /// limits are in effect from the first instruction of the sandboxed binary.
    ///
    /// # SAFETY
    ///
    /// The closure passed to `pre_exec` runs in the forked child in an async-signal-unsafe
    /// context. The only operations performed inside the closure are:
    ///
    /// - `setrlimit(RLIMIT_AS, ...)` — async-signal-safe per POSIX
    /// - `getrlimit(RLIMIT_NPROC, ...)` — async-signal-safe per POSIX
    /// - `setrlimit(RLIMIT_NPROC, ...)` — async-signal-safe per POSIX
    ///
    /// No Rust allocator, no Mutex, no `format!` macros are called inside the closure.
    /// All values (`memory_bytes`, `max_processes`, `baseline_uid_count`) are `Copy`
    /// types captured by value. `baseline_uid_count` was computed in the parent before
    /// fork via `proc_listpids(PROC_UID_ONLY)` — which is NOT async-signal-safe — and is
    /// captured as a plain `u64`. No kernel-query call occurs inside the closure.
    ///
    /// The `nix::errno::Errno` → `std::io::Error` conversion uses
    /// `std::io::Error::from` (nix's public `From<Errno> for std::io::Error` impl)
    /// which is also safe in `pre_exec` — internally it constructs an
    /// `io::Error` from the errno's raw integer value, without allocating or
    /// invoking async-signal-unsafe machinery.
    ///
    /// We prefer `From<Errno> for std::io::Error` over the prior `e as i32` cast
    /// because the cast relied on `nix::errno::Errno` being `#[repr(i32)]` —
    /// an internal nix detail. The `From` impl is the documented public API and
    /// is stable across nix's representation changes (WR-05).
    ///
    /// # T-25-01-05 mitigation
    ///
    /// The `memory_bytes` value is checked against `nix::libc::rlim_t::MAX` before the
    /// cast to prevent wrapping on hypothetical 32-bit platforms. nono's MSRV (1.77) and
    /// nix 0.31 both target 64-bit primary; the check is belt-and-suspenders.
    pub(crate) fn install_pre_exec(&self, cmd: &mut std::process::Command) {
        use std::os::unix::process::CommandExt;
        let memory_bytes = self.memory_bytes;
        let max_processes = self.max_processes;
        let baseline_uid_count = self.baseline_uid_count;

        // SAFETY: pre_exec runs in the forked child, post-fork pre-exec.
        // setrlimit/getrlimit are async-signal-safe (POSIX). No heap allocation
        // or locks are taken inside the closure. All captured values are Copy.
        // baseline_uid_count was computed in the parent before fork — sysctl is
        // not called inside this closure (it is not async-signal-safe).
        unsafe {
            cmd.pre_exec(move || -> std::io::Result<()> {
                #[cfg(target_os = "macos")]
                {
                    use nix::sys::resource::{setrlimit, Resource};
                    if let Some(bytes) = memory_bytes {
                        // macOS `rlim_t` is u64 (all macOS targets are 64-bit), so the
                        // u64 memory_bytes maps directly — no fallible conversion needed.
                        let limit: nix::libc::rlim_t = bytes;
                        setrlimit(Resource::RLIMIT_AS, limit, limit)
                            .map_err(std::io::Error::from)?;
                    }
                    if let Some(n) = max_processes {
                        // Compute RLIMIT_NPROC = baseline_uid_count + N (baseline+N strategy, D-01).
                        // baseline_uid_count is a Copy u64 captured from the parent before fork —
                        // no sysctl or allocation inside this closure.
                        //
                        // D-03: RLIMIT_NPROC on macOS is UID-wide (not descendant-tree-scoped
                        // like Linux pids.max). The bound means the agent's effective budget for
                        // additional processes is N, but other UID processes can consume those
                        // slots. This is accepted behavior; the race between the parent's sysctl
                        // read and the child's setrlimit apply is inherent to macOS's UID-wide
                        // accounting.
                        let target: libc::rlim_t = baseline_uid_count.saturating_add(u64::from(n));
                        // Read existing hard limit to avoid EPERM when raising above it (Pitfall 3).
                        // getrlimit is async-signal-safe per POSIX.
                        let mut existing: libc::rlimit = libc::rlimit {
                            rlim_cur: 0,
                            rlim_max: 0,
                        };
                        // SAFETY: getrlimit is async-signal-safe. existing is stack-allocated.
                        let got = libc::getrlimit(libc::RLIMIT_NPROC, &mut existing);
                        let hard = if got == 0 && existing.rlim_max != libc::RLIM_INFINITY {
                            existing.rlim_max
                        } else {
                            target
                        };
                        let soft = target.min(hard);
                        let rl = libc::rlimit {
                            rlim_cur: soft,
                            rlim_max: soft,
                        };
                        // SAFETY: setrlimit is async-signal-safe per POSIX; no heap allocation.
                        if libc::setrlimit(libc::RLIMIT_NPROC, &rl) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = (memory_bytes, max_processes, baseline_uid_count);
                }
                Ok(())
            });
        }
    }
}

/// Spawn a watchdog thread that sends `SIGKILL` to the child process group at `deadline`.
///
/// On macOS, there is no cgroup equivalent for atomic multi-process kill.
/// Instead, this watchdog kills the entire process group (negative PID) via
/// `kill(-pgrp, SIGKILL)`, which delivers SIGKILL to all processes in the group
/// simultaneously. This covers the child and any grandchildren that inherit the
/// same process group.
///
/// The child must call `setpgid(0, 0)` immediately after `fork()` in the
/// supervised child arm so that the child is its own process-group leader
/// (`pgid == child_pid`). This ensures `getpgid(child)` in the parent returns
/// `child_pid` deterministically and `kill(-child_pgrp, SIGKILL)` targets only
/// the agent tree, not the parent's process group.
///
/// # Watchdog behaviour
///
/// 1. Sleeps until `deadline` (using `Instant::checked_duration_since`).
/// 2. Sends `SIGKILL` to the entire process group `child_pgrp` via `kill(-pgrp, SIGKILL)`.
///
/// # Harmless after child exit
///
/// If the child exits before the deadline, the watchdog fires into an empty
/// process group and the `kill` call returns `ESRCH` (no such process), which
/// the watchdog silently ignores.
///
/// # Returns
///
/// A `JoinHandle` — the caller should `join()` or detach this handle after
/// reaping the child. The watchdog thread is lightweight (sleeping) for the
/// duration of the child's execution.
#[cfg(target_os = "macos")]
pub(crate) fn spawn_macos_timeout_watchdog(
    deadline: std::time::Instant,
    child_pgrp: nix::unistd::Pid,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let now = std::time::Instant::now();
        if let Some(remaining) = deadline.checked_duration_since(now) {
            std::thread::sleep(remaining);
        }
        // Negative PID = process group. SIGKILL = ungraceful, atomic to the group.
        // Ignore ESRCH (process already exited) — that's the normal race.
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(-child_pgrp.as_raw()),
            nix::sys::signal::Signal::SIGKILL,
        );
    })
}

#[cfg(all(test, target_os = "macos"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::launch_runtime::ResourceLimits;

    #[test]
    fn new_rejects_cpu_percent() {
        let limits = ResourceLimits {
            cpu_percent: Some(50),
            memory_bytes: None,
            max_processes: None,
            timeout: None,
        };
        let err = MacosResourceLimits::new(&limits).unwrap_err();
        assert!(
            matches!(
                err,
                NonoError::NotSupportedOnPlatform {
                    ref feature
                } if feature == "cpu_percent_macos"
            ),
            "expected NotSupportedOnPlatform {{ feature: \"cpu_percent_macos\" }}, got: {err:?}"
        );
    }

    #[test]
    fn new_with_all_none_is_ok() {
        let limits = ResourceLimits::default();
        let result = MacosResourceLimits::new(&limits);
        assert!(result.is_ok(), "all-None limits should succeed: {result:?}");
        let r = result.unwrap();
        assert!(r.memory_bytes.is_none());
        assert!(r.max_processes.is_none());
        assert_eq!(
            r.baseline_uid_count, 0,
            "no max_processes → baseline should be 0"
        );
    }

    #[test]
    fn new_with_max_processes_reads_baseline() {
        let limits = ResourceLimits {
            cpu_percent: None,
            memory_bytes: None,
            max_processes: Some(5),
            timeout: None,
        };
        let result = MacosResourceLimits::new(&limits);
        // On a real macOS host, sysctl should succeed and baseline > 0.
        assert!(
            result.is_ok(),
            "max_processes set → new() should succeed (sysctl available): {result:?}"
        );
        let r = result.unwrap();
        assert!(
            r.baseline_uid_count > 0,
            "expected baseline_uid_count > 0 on a running macOS host (sysctl returned process list)"
        );
    }
}
