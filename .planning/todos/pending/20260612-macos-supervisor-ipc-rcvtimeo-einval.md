# TODO: macOS supervised+IPC runs fail at sandbox init — set_read_timeout EINVAL on AF_UNIX socketpair

**Captured:** 2026-06-12 (surfaced during Phase 68-01 macOS host UAT on Oscars-MacBook-Pro)
**Severity:** high — supervised runs that open the capability-expansion IPC socket (e.g. rollback/snapshots) fail to start on macOS
**Source:** `cargo test -p nono-cli` on real macOS host — `audit_attestation` 3 tests fail at "Applying sandbox..."
**Relation to Phase 68:** SEPARATE bug. Phase 68 (RESL enforcement) never touched the IPC layer. The RESL tests use only static `--read` grants (no IPC socket) so they bypass this — it is a confound for the RESL failures, not their cause. Filed standalone so Phase 68 debug stays scoped.

## Problem
On a real macOS host, supervised runs that create the supervisor IPC socket fail at sandbox init:

```
mode supervised (snapshots, supervisor)
Applying sandbox...
nono: Sandbox initialization failed: Failed to set socket read timeout: Invalid argument (os error 22)
```

Reproduced by `audit_attestation::{rollback_signed_session_verifies_from_audit_dir_bundle,
combo_rollback_audit_session_findable_by_audit_verify, combo_rollback_audit_session_findable_by_rollback_list}`.

## Root-cause hypothesis
`crates/nono/src/supervisor/socket.rs:194` calls `UnixStream::set_read_timeout(Some(dur))`, wired in
`crates/nono-cli/src/exec_strategy.rs:1378` with `crate::timeouts::supervisor_ipc_read_timeout()` (default
`Duration::from_secs(5)` — `crates/nono-cli/src/timeouts.rs:75`). A 5s value is non-zero/valid, yet macOS
returns `EINVAL (os error 22)` from the underlying `setsockopt(SO_RCVTIMEO)`. Leading hypothesis: **macOS
does not accept `SO_RCVTIMEO` on AF_UNIX (`UnixStream`) sockets the way Linux does** (the socket is created
via `SupervisorSocket::pair()` → `UnixStream::pair()`, or `bind`/`accept`). This was introduced/wired in the
Phase 59 supervisor-IPC-robustness work and was likely never exercised on a real macOS host. Confirm whether
this also reproduces at the Phase 59 base commit (regression vs. always-broken-on-macOS).

## Fix directions (investigate)
- Verify the exact failing syscall/path on macOS (which `set_read_timeout` call: the production
  `exec_strategy.rs:1378`, or a `from_stream` path). Add `NONO_LOG=debug` tracing around the call.
- If macOS rejects `SO_RCVTIMEO` on AF_UNIX: switch the slowloris-stall protection to a poll/`recv`-deadline
  mechanism (non-blocking + `poll(2)`/`kevent`) instead of `SO_RCVTIMEO`, or apply the timeout only where the
  platform supports it and fall back to a poll loop on macOS. Keep the Phase 59 slowloris guarantee.
- Add a real macOS integration test (gated like the RESL host tests) that runs a supervised rollback so this
  cannot silently regress again.

## Acceptance
`cargo test -p nono-cli --test audit_attestation` passes on a real macOS host, and a supervised
`nono run --rollback ...` reaches "Applying sandbox..." → child execution without the `set_read_timeout` EINVAL.
