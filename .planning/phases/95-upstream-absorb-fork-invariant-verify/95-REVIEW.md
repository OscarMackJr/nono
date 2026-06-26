---
phase: 95-upstream-absorb-fork-invariant-verify
reviewed: 2026-06-26T05:11:02Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/tests/socket_access_run.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/route.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono/src/audit.rs
  - crates/nono/src/keystore.rs
  - crates/nono/src/sandbox/linux.rs
  - crates/nono/src/sandbox/mod.rs
  - crates/nono/src/scrub.rs
  - crates/nono/src/supervisor/socket.rs
findings:
  critical: 1
  warning: 6
  info: 2
  total: 9
status: issues_found
---

# Phase 95: Code Review Report

**Reviewed:** 2026-06-26T05:11:02Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

This phase absorbs upstream commits via cherry-pick (Cluster A: AF_UNIX seccomp-notify
deadlock fix; Cluster B: tool-sandbox shared-surface additions). Several fork invariants
explicitly checked for in the review brief are **intact**: the CR-02 audit invariant
(`records_verified: event_count > 0`, audit.rs:1570) is preserved, the `cmd://` keystore
scheme fails closed in `load_secret_by_ref` (keystore.rs:306), `restrict_execute` fails
secure on partial/non-enforcement (linux.rs:844), and config-load failures for the new
endpoint policy propagate fatally (route.rs:91).

However, the cherry-pick **clobbered two documented fork invariants** in
`crates/nono/src/sandbox/linux.rs`: the WR-04 arch-portable `msghdr` field-offset
derivation was reverted to a hardcoded LP64 layout, and the entire `--allow-gpu` Linux
enforcement (`collect_linux_gpu_paths` / `is_nvidia_compute_device` and the `caps.gpu()`
allowlist branch) was deleted. This is precisely the class of fork-invariant loss this
phase is meant to detect.

The most serious issue is the new endpoint-policy machinery in `nono-proxy`: it is fully
deserializable from operator config (`RouteConfig.endpoint_policy`), compiled at startup,
and stored on every `LoadedRoute` — but `CompiledEndpointPolicy::evaluate()` is **never
called on the request path**, which still consults the legacy `endpoint_rules`. An operator
who configures `endpoint_policy.deny` rules gets silent non-enforcement (fail-open).

A secondary theme: a large amount of Cluster B "shared-surface" code (audit recorders,
env-scrub helpers, the entire endpoint-policy outcome surface) was absorbed but is **not
wired into any caller**, contradicting the project's no-dead-code convention and shipping
untested public API.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Endpoint policy is configurable but never enforced (fail-open)

**File:** `crates/nono-proxy/src/config.rs:419` (`evaluate`), `crates/nono-proxy/src/route.rs:91` (compile-and-store), `crates/nono-proxy/src/reverse.rs:96` (request path)
**Issue:**
The new `EndpointPolicyConfig` is a live, operator-facing config surface:
`RouteConfig.endpoint_policy` is `#[serde(default)]`-deserializable and parses with
`deny_unknown_fields`. At startup `RouteStore::load` compiles it into
`LoadedRoute.endpoint_policy` (route.rs:91) and the policy supports `deny`, `approve`, and
`allow` rules with a fail-secure default (`EndpointPolicyDecision::Deny`).

But the actual request authorization in `reverse.rs:96` still calls
`route.endpoint_rules.is_allowed(...)` — the *legacy* path. `CompiledEndpointPolicy::evaluate()`
and `allows_all_without_l7()` have **zero callers** anywhere in the crate (verified via grep;
only `compile` is referenced, at route.rs:91 and a single test). No test exercises the
deny/approve/allow evaluation logic.

Consequence: an operator who writes an `endpoint_policy` with `deny` rules — reasonably
expecting requests matching those rules to be blocked — gets those rules silently ignored.
Because the legacy `endpoint_rules` is empty in that configuration, it defaults to allow-all.
A control the operator believes is enforced is not. In a security-critical proxy that
mediates outbound agent traffic, a configured-but-unenforced deny rule is a fail-open gap.

**Fix:** Either (a) wire `endpoint_policy.evaluate()` into the request path in `reverse.rs`
so it takes precedence over / replaces `endpoint_rules.is_allowed`, mapping `Deny` →
403/blocked, `Approve` → approval flow, `Allow` → forward; or (b) if enforcement is
deliberately deferred to a later phase, do NOT expose `endpoint_policy` in the public
`RouteConfig` schema yet — reject configs that set it with an explicit
`ProxyError::Config("endpoint_policy not yet supported")` at load time so operators cannot
silently rely on an unenforced control. Fail closed on an unimplemented security knob.
```rust
// reverse.rs, replacing the endpoint_rules.is_allowed check:
match route.endpoint_policy.evaluate(&method, &upstream_path) {
    EndpointPolicyOutcome::Allow { .. } => { /* proceed */ }
    EndpointPolicyOutcome::Deny { reason, rule_label } => {
        // emit audit + return 403
        return Ok(forbidden_response(rule_label, reason));
    }
    EndpointPolicyOutcome::Approve { .. } => { /* route to approval backend */ }
}
```

## Warnings

### WR-01: `format!()` heap allocation in post-fork/pre-exec child reverts CR-01 fork-safety invariant

**File:** `crates/nono-cli/src/exec_strategy.rs:1411`, `crates/nono-cli/src/exec_strategy.rs:1450`
**Issue:**
Both new error paths in the proxy-notify-fd handling run in the post-`fork`/pre-`execve`
child. The surrounding code (lines 1304-1331, unchanged) deliberately uses `const … &[u8]`
static byte strings with the explicit comment *"CR-01: static byte string in post-fork
child."* — because the child is single-threaded after fork and the global allocator lock may
have been held by another thread at fork time, so any heap allocation can deadlock. The new
code instead builds the message with `format!(...)` (heap allocation) before the `write` +
`_exit(126)`:
```rust
let detail = format!(
    "nono: failed to write proxy seccomp notify fd number: {}\n",
    std::io::Error::last_os_error()
);  // <-- heap alloc in post-fork child
```
This regresses the established CR-01 invariant. It is on error paths only, so it will rarely
trigger, but a child that hits it can hang instead of reporting the failure and exiting.
**Fix:** Use a static `&[u8]` message exactly as the adjacent code does, dropping the errno
interpolation (or write the message then the raw errno via a stack-only itoa). Example:
```rust
const MSG_PROXY_WRITE: &[u8] = b"nono: failed to write proxy seccomp notify fd number\n";
unsafe {
    libc::write(libc::STDERR_FILENO, MSG_PROXY_WRITE.as_ptr().cast(), MSG_PROXY_WRITE.len());
    libc::_exit(126);
}
```

### WR-02: WR-04 arch-portable `msghdr` offset derivation reverted to hardcoded LP64 layout

**File:** `crates/nono/src/sandbox/linux.rs:2427` (`read_msghdr_dest`)
**Issue:**
Pre-diff (baseline `ed6cdde1`) this function derived `msg_name` / `msg_namelen` offsets at
compile time via `core::mem::offset_of!(libc::msghdr, …)` and `size_of`, with compile-time
overlap assertions, explicitly documented as *"the WR-04 fix: the original code hard-coded
`MSGHDR_MIN_READ = 12` (valid only on LP64)."* The cherry-pick reverted this to the hardcoded
form: `const MSGHDR_MIN_READ: usize = 12;` and `buf[0..8]` → `msg_name`, `buf[8..12]` →
`msg_namelen`. This is the AF_UNIX `sendmsg`/`sendmmsg` destination-mediation read path — a
security-critical primitive. On the fork's shipped 64-bit targets (x86_64, aarch64) the
offsets are coincidentally correct, so runtime behavior is unaffected today, but the
explicit fork-hardening invariant was silently dropped by the absorb, and a future 32-bit /
LP32 target would misread the destination sockaddr (wrong mediation decision). Two
`#[must_use]` attributes on `read_msghdr_dest` and `read_mmsghdr_dests` were also removed,
violating the CLAUDE.md "`#[must_use]` on functions returning critical Results" rule, and the
WR-01 TOCTOU documentation comment at the `msg_name == 0` branch was deleted.
**Fix:** Restore the `offset_of!`/`size_of`-derived constants and the compile-time
assertions, re-apply `#[must_use = "..."]` to both `read_msghdr_dest` and
`read_mmsghdr_dests`, and restore the TOCTOU comment. This is a fork-invariant the phase is
chartered to preserve; record it in the DIVERGENCE-LEDGER if the revert was intentional.

### WR-03: `--allow-gpu` Linux enforcement deleted — silent feature regression

**File:** `crates/nono/src/sandbox/linux.rs` (removed `collect_linux_gpu_paths`, `is_nvidia_compute_device`, and the `if caps.gpu() { … }` allowlist branch around former line 915)
**Issue:**
The cherry-pick removed the entire Linux GPU-path enforcement: the `collect_linux_gpu_paths`
and `is_nvidia_compute_device` helpers (and their tests) are gone, and the `caps.gpu()`
branch that translated the grant into Landlock allowlist rules for `/dev/dri/renderD*`,
`/dev/nvidia*`, `/dev/dxg`, `/sys/class/drm`, etc. is gone. The capability itself still
exists (`capability.rs:1451 gpu()`), the `--allow-gpu` CLI flag still exists
(`cli.rs`, `capability_ext.rs`), and **macOS still honors it** (`sandbox/macos.rs`). The net
effect: on Linux, `nono run --allow-gpu …` now silently grants no GPU device paths, so GPU
access for sandboxed agents is broken on Linux while the flag still appears accepted.
This is not a security *weakening* (the sandbox is strictly more restrictive), but it is a
silent breakage of a documented fork feature and a divergence from macOS behavior, and
exactly the kind of fork-invariant loss this phase verifies against.
**Fix:** Re-apply the fork's GPU-path collection and the `caps.gpu()` Landlock branch on
Linux, or — if GPU support is being deliberately dropped — also remove/error the
`--allow-gpu` flag and `caps.gpu()` so the capability cannot be requested silently, and
record the removal in the DIVERGENCE-LEDGER.

### WR-04: WSL2 child/parent asymmetry can deadlock the supervisor on the notify-fd handshake

**File:** `crates/nono-cli/src/exec_strategy.rs:1364` (child WSL2 skip) vs `crates/nono-cli/src/exec_strategy.rs:1694` (parent recv predicate)
**Issue:**
The child installs the notify filter and writes the fd number only when
`install_network_notify && !is_wsl2()` (the WSL2 branch at 1364 writes a warning and skips
the send). The parent unconditionally calls `recv_raw_fd_number()` whenever
`config.seccomp_proxy_fallback || config.af_unix_mediation.is_pathname()` (1694), with **no**
`is_wsl2()` guard. `recv_raw_fd_number` does a blocking `read_exact` of 4 bytes
(`supervisor/socket.rs:185`). On WSL2 the child never writes those bytes, so the parent
blocks until the child reaches `execve` and the socket closes (EOF) — at which point it is
handled, but the new ack handshake means the child also blocks on its own
`read(fd, ack, 1)` only when it took the send path, so the ordering is fragile. This WSL2
asymmetry pre-dates this diff (the old parent also lacked the guard), but this change
rewrote both predicates and the handshake, so it should be made symmetric now rather than
relying on EOF semantics.
**Fix:** Guard the parent recv with the same WSL2 check the child uses:
```rust
let proxy_notify_fd = if (config.seccomp_proxy_fallback
    || config.af_unix_mediation.is_pathname())
    && !nono::sandbox::is_wsl2()
{ /* recv_raw_fd_number + pidfd_getfd */ } else { None };
```

### WR-05: Cluster B audit/scrub surface absorbed with no callers (untested dead public API)

**File:** `crates/nono/src/audit.rs:584` (`record_sandbox_runtime_event`), `crates/nono/src/audit.rs:590` (`record_command_policy_event`), `crates/nono/src/scrub.rs:236` (`scrub_env_name`), `crates/nono/src/scrub.rs:251` (`scrub_env_value`)
**Issue:**
The newly absorbed `record_sandbox_runtime_event` / `record_command_policy_event` recorders
(and the whole `SandboxRuntimeAuditEvent` / `CommandPolicyAuditEvent` / `CommandPolicyStdio*`
type family) and the `scrub_env_name` / `scrub_env_value` helpers have **no callers**
anywhere in the workspace outside their own definitions (verified via `git grep`). The audit
recorders even carry `#[cfg_attr(not(target_os = "linux"), allow(dead_code))]`, acknowledging
they are dead off-Linux — but they are dead on Linux too. CLAUDE.md: *"Avoid
`#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it."*
Because these are `pub`, clippy will not flag them, so the dead surface ships silently and
untested. This is staged infrastructure for a feature (tool-sandbox command mediation) that
is not wired up in this fork.
**Fix:** Either wire these recorders/helpers into the actual sandbox-runtime and
tool-sandbox command paths, or defer absorbing them until the consuming code lands. If they
must be carried now, add unit tests that exercise serialization + the redaction logic, and
note the staged-surface decision in the DIVERGENCE-LEDGER so reviewers don't read it as
accidental dead code.

### WR-06: New `cmd://` keystore validators are unused and untested public API

**File:** `crates/nono/src/keystore.rs:862` (`is_cmd_uri`), `crates/nono/src/keystore.rs:870` (`validate_cmd_uri`)
**Issue:**
`is_cmd_uri` and `validate_cmd_uri` are new `pub fn`s with no callers and no tests anywhere
in the workspace (verified via `git grep`). The fail-closed branch in `load_secret_by_ref`
(keystore.rs:306) does its own inline `starts_with(CMD_URI_PREFIX)` check rather than calling
`is_cmd_uri`, so the helpers are pure dead surface. `validate_cmd_uri`'s `[A-Za-z0-9_]+`
allowlist is reasonable, but with no caller nothing actually validates a `cmd://` name before
use, and with no test the validator's behavior is unverified.
**Fix:** Call `validate_cmd_uri` wherever a `cmd://` reference is parsed/accepted (the
supervisor credential-capture path the doc-comment references), and add unit tests for the
empty-name and invalid-character rejection cases. If the consuming path is not in scope for
this phase, defer the helpers until it lands.

## Info

### IN-01: Doubled space in `restrict_execute` error strings

**File:** `crates/nono/src/sandbox/linux.rs:794` and following error strings in `restrict_execute`
**Issue:** Every error message in `restrict_execute` reads `"Tool Sandbox  execute
restriction…"` with two spaces between "Sandbox" and "execute". Cosmetic, but user-facing.
**Fix:** Collapse to a single space.

### IN-02: `read()` ack loop in supervisor child ignores partial/zero-byte reads

**File:** `crates/nono-cli/src/exec_strategy.rs:1432`
**Issue:** The child's ack loop breaks on any non-EINTR return, including `n == 0` (EOF,
parent died) and `n < 0` for non-EINTR errors. On EOF/error it proceeds to exec rather than
treating a lost ack as fatal. The comment claims the loop ensures the parent acquired the fd
before `O_CLOEXEC` closes it, but a zero/negative read defeats that guarantee silently. Low
practical risk (parent death is already terminal), but the loop does not deliver the
invariant its comment asserts.
**Fix:** Distinguish `n == 1` (acked) from `n <= 0` (parent gone / error) and, on the
latter, write a static error message and `_exit(126)` rather than proceeding to exec.

---

_Reviewed: 2026-06-26T05:11:02Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
