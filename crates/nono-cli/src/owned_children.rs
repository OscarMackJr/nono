//! Process-global registry of child PIDs owned by an in-process
//! `std::process::Child` handle (Unix only).
//!
//! # Why this exists (Phase 112, WR-01)
//!
//! The Linux supervisor sets `PR_SET_CHILD_SUBREAPER` so that descendants of
//! the sandboxed child which re-parent away (daemonizing helpers, double-forked
//! tools) land on nono rather than on init, and it drains them with
//! `reap_reparented_orphans`. That reaper used `waitpid(-1, WNOHANG)`, which is
//! **process-global**: it reaps *any* terminated direct child, including one
//! whose exit status another part of this same process is going to collect
//! through a `std::process::Child`.
//!
//! When that happens the status is consumed by the reaper, libstd's later
//! `Child::wait()` / `Child::wait_with_output()` fails with `ECHILD`, and the
//! caller sees a spurious failure for a subprocess that actually succeeded. The
//! freed pid can also be recycled while the stale handle still refers to it.
//!
//! # The invariant
//!
//! Any code that spawns a subprocess **and intends to wait for it itself** must
//! spawn it through [`spawn_owned`]. The pid is then registered for as long as
//! the returned [`OwnedChild`] is alive, and the orphan reaper will not consume
//! its status.
//!
//! Fire-and-forget spawns that deliberately abandon the handle (for example the
//! detached pack-update-hint helper) must **not** use this module — those are
//! exactly the processes the reaper exists to collect.
//!
//! # Atomicity
//!
//! [`spawn_owned`] holds the registry mutex across `Command::spawn()` and the
//! insert, and the reaper holds the same mutex for its whole (non-blocking)
//! reap pass. There is therefore no window in which a child exists but is not
//! yet registered while the reaper is running.

use std::collections::HashSet;
use std::io;
use std::process::{Child, Command, Output};
use std::sync::{Mutex, MutexGuard, OnceLock};

fn registry() -> &'static Mutex<HashSet<i32>> {
    static REGISTRY: OnceLock<Mutex<HashSet<i32>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Lock the registry, tolerating poisoning.
///
/// A panic while the lock is held cannot corrupt the invariant this set
/// protects — it is a plain set of pids — and failing closed here would mean
/// refusing to reap (or refusing to spawn) for the rest of the process
/// lifetime. Recovering the inner value is the conservative choice.
pub(crate) fn lock_owned_pids() -> MutexGuard<'static, HashSet<i32>> {
    match registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Spawn `cmd` and register its pid as owned for the lifetime of the returned
/// handle.
///
/// The registry mutex is held across the spawn so the pid cannot be observed by
/// the orphan reaper before it is registered.
pub(crate) fn spawn_owned(cmd: &mut Command) -> io::Result<OwnedChild> {
    let mut owned = lock_owned_pids();
    let child = cmd.spawn()?;
    // `Child::id()` is the OS pid and always fits in i32 on Unix
    // (`PID_MAX_LIMIT` is 2^22), but convert fallibly rather than casting:
    // a lossy cast here would register the wrong pid and silently drop the
    // protection this type exists to provide.
    let pid = i32::try_from(child.id()).map_err(|_| {
        io::Error::other(format!(
            "spawned child pid {} does not fit in i32",
            child.id()
        ))
    })?;
    owned.insert(pid);
    drop(owned);
    Ok(OwnedChild {
        child: Some(child),
        pid,
    })
}

/// A `std::process::Child` whose pid is registered as owned by this process.
///
/// Dropping the handle deregisters the pid, so the orphan reaper is free to
/// collect it again.
#[derive(Debug)]
pub(crate) struct OwnedChild {
    child: Option<Child>,
    pid: i32,
}

impl OwnedChild {
    /// The child's pid.
    pub(crate) fn id(&self) -> i32 {
        self.pid
    }

    /// Wait for the child and collect its output, then deregister the pid.
    ///
    /// Deregistration happens in `Drop`, i.e. after the wait has returned.
    pub(crate) fn wait_with_output(mut self) -> io::Result<Output> {
        match self.child.take() {
            Some(child) => child.wait_with_output(),
            None => Err(io::Error::other("owned child handle already consumed")),
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        lock_owned_pids().remove(&self.pid);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn spawned_pid_is_registered_until_the_handle_is_dropped() {
        let mut cmd = Command::new("true");
        let Ok(owned) = spawn_owned(&mut cmd) else {
            // `true` is not guaranteed present on every host; skip rather than
            // fail on a missing coreutils binary.
            return;
        };
        let pid = owned.id();
        assert!(
            lock_owned_pids().contains(&pid),
            "pid {pid} must be registered while the OwnedChild is alive"
        );
        let _ = owned.wait_with_output();
        assert!(
            !lock_owned_pids().contains(&pid),
            "pid {pid} must be deregistered once the OwnedChild is dropped"
        );
    }

    #[test]
    fn dropping_without_waiting_still_deregisters() {
        let mut cmd = Command::new("true");
        let Ok(owned) = spawn_owned(&mut cmd) else {
            return;
        };
        let pid = owned.id();
        drop(owned);
        assert!(
            !lock_owned_pids().contains(&pid),
            "pid {pid} must be deregistered on plain drop, not only after a wait"
        );
    }

    #[test]
    fn a_failed_spawn_registers_nothing() {
        let before = lock_owned_pids().len();
        let mut cmd = Command::new("/nonexistent/nono-owned-children-test-binary");
        assert!(spawn_owned(&mut cmd).is_err());
        assert_eq!(
            lock_owned_pids().len(),
            before,
            "a failed spawn must not leave a registration behind"
        );
    }
}
