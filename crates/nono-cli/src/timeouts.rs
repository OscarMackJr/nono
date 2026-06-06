//! Named timeout and polling-interval constants used across the CLI.
//!
//! User-facing timeouts can be overridden via environment variables.
//! Internal poll intervals use fixed defaults.

use std::time::Duration;
use tracing::warn;

// exec_strategy

/// Quiet period to drain final PTY output after child exit before parent
/// diagnostics/prompts take over the terminal.
#[cfg(unix)]
pub const POST_EXIT_PTY_DRAIN_TIMEOUT: Duration = Duration::from_millis(100);

/// Poll interval for the non-blocking `waitpid` loop.
#[cfg(unix)]
pub const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(200);

// pty_proxy

/// Read timeout on the attach socket when reading the request-kind byte.
#[cfg(unix)]
pub const ATTACH_SOCKET_READ_TIMEOUT: Duration = Duration::from_millis(500);

/// Delay before forwarding stdin in the attach warm-up loop, giving the
/// supervisor time to replay buffered screen content.
#[cfg(unix)]
pub const ATTACH_STDIN_DELAY: Duration = Duration::from_millis(250);

/// Sleep before retrying a session connection that failed with `SessionGone`.
#[cfg(unix)]
pub const ATTACH_RETRY_DELAY: Duration = Duration::from_millis(150);

// startup_runtime

/// Maximum time to wait for a detached session to create its session file
/// and attach socket.
pub const DETACH_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Poll interval while waiting for a detached session to become attachable.
pub const SESSION_READY_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Poll interval while waiting for a detached launch process to exit after
/// SIGTERM.
#[cfg(unix)]
pub const TERMINATE_POLL_INTERVAL: Duration = Duration::from_millis(25);

// session_commands

/// Poll interval while waiting for a session to exit after SIGTERM.
#[cfg(unix)]
pub const STOP_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Poll interval when tailing a log file and the reader reaches EOF.
#[cfg(unix)]
pub const LOG_TAIL_POLL_INTERVAL: Duration = Duration::from_millis(250);

// learn

/// Delay after spawning `fs_usage` to let it attach the kernel trace
/// facility before the child command starts (macOS).
#[cfg(target_os = "macos")]
pub const FS_USAGE_SETTLE_TIME: Duration = Duration::from_secs(2);

/// Grace period after SIGTERM before escalating to SIGKILL (learn mode).
#[cfg(target_os = "macos")]
pub const SIGTERM_GRACE_PERIOD: Duration = Duration::from_secs(3);

// supervisor IPC

/// Bounded read timeout for the supervisor IPC listener.
///
/// Matches upstream d1851c9 (5 s). Defends against a slow or silent child
/// holding a partial frame and thereby blocking the supervisor indefinitely.
///
/// The value is intentionally not `#[cfg(unix)]`: Windows reads the same
/// constant via the `NONO_SUPERVISOR_IPC_READ_TIMEOUT` env override, and
/// `env_duration_secs` itself is non-cfg'd.
// Wired in 59-02 (Unix) and 59-03 (Windows); the const is defined here so
// both plans can reference it without creating a dependency between the two
// Wave-2 plans.
#[allow(dead_code)]
pub const SUPERVISOR_IPC_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Read `NONO_SUPERVISOR_IPC_READ_TIMEOUT` (seconds), clamped to `MAX_TIMEOUT`.
///
/// Returns [`SUPERVISOR_IPC_READ_TIMEOUT`] when the variable is absent or
/// contains an unparseable value.
// Wired in 59-02 (Unix) and 59-03 (Windows).
#[allow(dead_code)]
pub fn supervisor_ipc_read_timeout() -> Duration {
    env_duration_secs(
        "NONO_SUPERVISOR_IPC_READ_TIMEOUT",
        SUPERVISOR_IPC_READ_TIMEOUT,
    )
}

// Configurable user-facing timeouts

/// Read `NONO_DETACH_STARTUP_TIMEOUT` (seconds). Returns the default when
/// the variable is absent or unparseable.
pub fn detach_startup_timeout() -> Duration {
    env_duration_secs("NONO_DETACH_STARTUP_TIMEOUT", DETACH_STARTUP_TIMEOUT)
}

/// Read `NONO_PTY_DRAIN_TIMEOUT` (milliseconds). Returns the default when
/// the variable is absent or unparseable.
#[cfg(unix)]
pub fn pty_drain_timeout() -> Duration {
    env_duration_millis("NONO_PTY_DRAIN_TIMEOUT", POST_EXIT_PTY_DRAIN_TIMEOUT)
}

/// Read `NONO_PTY_ATTACH_TIMEOUT` (milliseconds). Returns the default when
/// the variable is absent or unparseable.
#[cfg(unix)]
pub fn pty_attach_timeout_ms() -> i32 {
    env_duration_millis(
        "NONO_PTY_ATTACH_TIMEOUT",
        Duration::from_millis(PTY_ATTACH_TIMEOUT_MS as u64),
    )
    .as_millis()
    .min(i32::MAX as u128) as i32
}

/// Default for `wait_for_attach_ready` poll timeout.
#[cfg(unix)]
pub const PTY_ATTACH_TIMEOUT_MS: i32 = 1000;

/// Upper bound for any user-supplied timeout. Prevents `Instant + Duration`
/// overflow from user-controlled values (u64::MAX seconds would panic).
const MAX_TIMEOUT: Duration = Duration::from_secs(3600);

fn env_duration_secs(var: &str, default: Duration) -> Duration {
    match std::env::var(var) {
        Ok(val) => match val.parse::<u64>() {
            Ok(secs) => {
                let d = Duration::from_secs(secs);
                if d > MAX_TIMEOUT {
                    warn!(
                        "{var}={val} exceeds maximum ({} s), clamping",
                        MAX_TIMEOUT.as_secs()
                    );
                    MAX_TIMEOUT
                } else {
                    d
                }
            }
            Err(_) => {
                warn!("{var}={val:?} is not a valid number of seconds, using default");
                default
            }
        },
        Err(_) => default,
    }
}

#[cfg(unix)]
fn env_duration_millis(var: &str, default: Duration) -> Duration {
    match std::env::var(var) {
        Ok(val) => match val.parse::<u64>() {
            Ok(ms) => {
                let d = Duration::from_millis(ms);
                if d > MAX_TIMEOUT {
                    warn!(
                        "{var}={val} exceeds maximum ({} s), clamping",
                        MAX_TIMEOUT.as_secs()
                    );
                    MAX_TIMEOUT
                } else {
                    d
                }
            }
            Err(_) => {
                warn!("{var}={val:?} is not a valid number of milliseconds, using default");
                default
            }
        },
        Err(_) => default,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_env::EnvVarGuard;

    const TIMEOUT_VAR: &str = "NONO_SUPERVISOR_IPC_READ_TIMEOUT";

    /// Default value is 5 s when `NONO_SUPERVISOR_IPC_READ_TIMEOUT` is absent.
    #[test]
    fn supervisor_ipc_read_timeout_default() {
        // Acquire the process-global env-var lock, then use EnvVarGuard to
        // capture the current value (if any) and temporarily set a sentinel
        // value we immediately remove so the env var is absent during the test.
        let _env_lock = crate::test_env::lock_env();
        // set_all captures the original and sets the sentinel; remove() unsets
        // it for the actual test; Drop restores the original.
        let _guard = EnvVarGuard::set_all(&[(TIMEOUT_VAR, "__sentinel__")]);
        _guard.remove(TIMEOUT_VAR);

        let got = supervisor_ipc_read_timeout();
        assert_eq!(got, Duration::from_secs(5));
    }

    /// `NONO_SUPERVISOR_IPC_READ_TIMEOUT=1` yields a 1-second duration.
    #[test]
    fn supervisor_ipc_read_timeout_env_override() {
        let _env_lock = crate::test_env::lock_env();
        let _guard = EnvVarGuard::set_all(&[(TIMEOUT_VAR, "1")]);

        let got = supervisor_ipc_read_timeout();
        assert_eq!(got, Duration::from_secs(1));
    }

    /// A value greater than `MAX_TIMEOUT` (3600 s) is clamped to `MAX_TIMEOUT`.
    #[test]
    fn supervisor_ipc_read_timeout_clamp() {
        let _env_lock = crate::test_env::lock_env();
        let _guard = EnvVarGuard::set_all(&[(TIMEOUT_VAR, "99999")]);

        let got = supervisor_ipc_read_timeout();
        assert_eq!(got, MAX_TIMEOUT);
    }

    /// An unparseable env value (e.g. "abc") falls back to the 5 s default.
    #[test]
    fn supervisor_ipc_read_timeout_invalid_fallback() {
        let _env_lock = crate::test_env::lock_env();
        let _guard = EnvVarGuard::set_all(&[(TIMEOUT_VAR, "abc")]);

        let got = supervisor_ipc_read_timeout();
        assert_eq!(got, SUPERVISOR_IPC_READ_TIMEOUT);
    }
}
