---
phase: 112-security-residual-sync
reviewed: 2026-08-05T00:00:00Z
depth: standard
files_reviewed: 33
files_reviewed_list:
  - crates/nono/src/lib.rs
  - crates/nono/src/sandbox/linux.rs
  - crates/nono/src/sandbox/mod.rs
  - crates/nono/src/supervisor/socket.rs
  - crates/nono/src/trust/mod.rs
  - crates/nono/src/trust/policy.rs
  - crates/nono/src/trust/types.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/proxy_command.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/pty_proxy.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/timeouts.rs
  - crates/nono-cli/src/trust_cmd.rs
  - crates/nono-cli/src/trust_scan.rs
  - crates/nono-cli/tests/proxy_command_run.rs
  - crates/nono-cli/tests/socket_access_run.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/connect.rs
  - crates/nono-proxy/src/external.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/server.rs
  - docs/cli/features/trust.mdx
  - tests/integration/test_trust_cli.sh
findings:
  critical: 4
  warning: 12
  info: 5
  total: 21
status: issues_found
---

# Phase 112: Code Review Report

**Reviewed:** 2026-08-05
**Depth:** standard
**Files Reviewed:** 33 (diff base `39ebe2fb`)
**Status:** issues_found

## Summary

Phase 112 absorbs five upstream threads: a standalone `nono proxy` command with new
`require_auth`/`strict_connect_auth` proxy knobs, a new core-library
`apply_seccomp`/`SeccompOpts` surface plus NVIDIA procfs hardening, `PR_SET_CHILD_SUBREAPER`
+ orphan reaping in the Linux supervisor, a PTY teardown CPR drain, and a trust-policy
`predicate` discriminator replacing the max-version merge.

The CONNECT-auth work is genuinely good: the `strict_connect_auth` gate is correctly
`state.config.require_auth && state.config.strict_connect_auth` on both CONNECT call sites,
and making seccomp-notify setup fatal instead of warn-and-continue closes a real
fail-open. Token validation remains constant-time over 256 bits of entropy.

However, four of the five threads introduce a **silent security downgrade**, and in each
case the downgrade is the *default* path rather than an edge case:

1. `nono proxy --profile X` discards the profile's entire deny layer, `no_proxy` list, and
   `network.block` → an operator's deny-scoped or fully-blocked profile becomes an
   allow-all forwarding proxy (CR-01).
2. Every `trust-policy.json` written by any prior nono release lacks the new `predicate`
   field, and the loader's response is to fall back to an *empty* policy — and to stop
   consulting the user-level policy entirely. An untrusted repo can neutralize the
   operator's global trust policy by shipping a predicate-less `trust-policy.json` (CR-02).
3. The new orphan reaper now catches the primary child's exit on the common path, which
   bypasses `drain_pending_network_notifications()` and drops the final network denial
   records from diagnostics and audit (CR-03).
4. `SeccompOpts::external_tcp()` ships an unused, externally-callable escape hatch in the
   *core library* that ignores `CapabilitySet`'s network mode entirely and returns the same
   `SeccompNetFallback::None` that means "Landlock is enforcing the network" (CR-04).

All four collide with CLAUDE.md's non-negotiables: "Fail Secure — on any error, deny
access. Never silently degrade", "Configuration load failures must be fatal", and "The
library applies ONLY what's in `CapabilitySet`".

## Critical Issues

### CR-01: `nono proxy` silently discards the profile's deny layer, `no_proxy`, and `network.block`

**File:** `crates/nono-cli/src/proxy_command.rs:125-147` (fields at `:129`, `:130`, `:144`)

**Issue:** `build_launch_options()` reads `--profile` via `profile::load_profile(name)` and
merges `net.allow_domain`, `net.credentials`, `net.custom_credentials`,
`net.upstream_proxy`, `net.upstream_bypass`, `net.allow_http2` — but hardcodes three
security-relevant fields to empty/false:

```rust
deny_domain: Vec::new(),
no_proxy: Vec::new(),
...
strict_filter: false,
```

`crates/nono-cli/src/profile/mod.rs:1734,1745,1710` show `NetworkConfig` carries
`deny_domain`, `no_proxy`, and `block`. The `nono run` equivalent
(`proxy_runtime::resolve_effective_proxy_settings` + `prepare_proxy_launch_options`) honours
all three: `deny_domain` flows to `ProxyConfig.denied_hosts` via
`network_policy::expand_proxy_deny`, and `profile_network_block` sets `strict_filter: true`.

Concrete consequences for `nono proxy --profile X`:

- **Deny-layer bypass.** A profile declaring `allow_domain: ["*.corp.com"]` +
  `deny_domain: ["secrets.corp.com"]` produces `denied_hosts: []`. `secrets.corp.com`
  matches the `*.corp.com` allowlist and is forwarded. ADR-108's deny layer is simply
  absent on this path.
- **ADR-108 (b) strict-selection can never fire.** `network_policy::build_proxy_config`
  computes `strict_filter = !denied_hosts.is_empty() && allowed_hosts.is_empty()`; with
  `denied_hosts` forced empty this is always `false`.
- **`network.block` ignored.** A profile with `network: { "block": true }` becomes an
  allow-all proxy instead of a strict-filtered one.
- **Bare `nono proxy` is an open forwarding proxy.** With no profile and no
  `--allow-domain`, `allowed_hosts` is empty and `strict_filter` is `false`, which
  `ProxyConfig.allowed_hosts` documents as "allow all hosts (except deny list)".

This is reachable ambiently: `ProxyArgs::profile` carries `env = "NONO_PROFILE"`
(`cli.rs:2378`), so an exported `NONO_PROFILE` is picked up with its deny list stripped.

**Fix:** thread all three through, and add a `--deny-domain` flag mirroring `nono run`:

```rust
// in build_launch_options, alongside the allow_domain merge:
let mut deny_domain: Vec<String> = args.deny_proxy.clone();
let mut no_proxy: Vec<String> = Vec::new();
let mut profile_network_block = false;
if let Some(ref name) = args.profile {
    let loaded = profile::load_profile(name)?;
    let net = loaded.network;
    deny_domain.extend(net.deny_domain.clone());
    no_proxy.extend(net.no_proxy.clone());
    profile_network_block = net.block;
    // ... existing merges
}
// D-04 parity: reject a deny-only effective config here too.
Ok(ProxyLaunchOptions {
    deny_domain,
    no_proxy,
    strict_filter: profile_network_block,
    ..
})
```

Until the flags exist, at minimum **fail closed**: reject `--profile` whose
`network.deny_domain`/`no_proxy`/`block` are non-default rather than silently dropping them.

---

### CR-02: Trust-policy `predicate` migration is fail-open — a predicate-less policy neutralizes the operator's user-level trust policy

**File:** `crates/nono-cli/src/trust_cmd.rs:1591-1593`, `:1599-1602`;
`crates/nono-cli/src/trust_scan.rs:44-79`, `:144`; `crates/nono/src/trust/types.rs:110-116`

**Issue:** `load_nono_policy()` returns `Ok(None)` (warning to stderr, no error) when a
`trust-policy.json` lacks the new `predicate` field. Every callsite treats `None` as
"there is no policy", and the fallbacks are permissive:

```rust
// trust_cmd.rs:1591-1593 — project policy present but predicate-less
let project_policy = match crate::trust_scan::load_nono_policy(&cwd_policy)? {
    Some(policy) => policy,
    None => return Ok(trust::TrustPolicy::default()),   // <-- empty includes; RETURNS EARLY
};
```

Three separate downgrades:

1. **User policy is never consulted.** The `None` arm above `return`s *before* the
   user-level `~/.config/nono/trust-policy.json` is loaded. So an untrusted repository that
   ships a `trust-policy.json` without `"predicate"` (or with any other predicate string)
   makes nono discard the operator's global publishers, blocklist, and `Deny` enforcement
   scope. `TrustPolicy::default()` is `enforcement: Deny` but `includes: []` and
   `files: []`, so *zero files are ever selected for verification* — nominally "deny mode",
   effectively no trust enforcement at all. This is attacker-controllable input deciding
   whether the operator's policy applies.

2. **Every pre-existing policy in the field trips this.** `run_init` previously wrote
   `"version": 1` with no `predicate` (see the diff at `trust_cmd.rs:138-141` and the
   `docs/cli/features/trust.mdx` examples that this phase edited). There is no migration
   step, no auto-upgrade, and `validate_version()` was relaxed
   (`types.rs:110-116`) so the *library* still accepts predicate-less policies — only the
   CLI loader rejects them. So the upgrade path for every existing deployment is "trust
   silently stops covering anything, with one yellow stderr line".

3. **A predicate-less user policy is dropped while the project policy survives**
   (`trust_cmd.rs:1596-1602`): the `if let Some(user_policy)` block simply falls through,
   returning `project_policy` alone. Because `merge_policies` takes the *strictest*
   enforcement, dropping the user policy can only weaken the result.
   `trust_scan::load_scan_policy:144`'s `(None, None) => Ok(TrustPolicy::default())` arm has
   the same shape.

CLAUDE.md: *"Configuration load failures must be fatal. If security lists fail to load,
abort"* and *"Silent fallbacks: `unwrap_or_default()` on security config returns empty
permissions = no protection."* Note the inconsistency: `run_sign_policy`
(`trust_cmd.rs:752-758`) and the explicit-`--policy` path (`:1578-1583`) correctly hard-error
on a missing predicate. Only the auto-discovery path fails open.

**Fix:** make the discriminator miss fatal on the enforcement path, and migrate legacy
policies rather than discarding them:

```rust
// trust_scan.rs — distinguish "foreign file" from "legacy nono policy"
match raw.get("predicate").and_then(|v| v.as_str()) {
    Some(p) if p == nono::trust::TRUST_POLICY_PREDICATE => {}
    Some(other) => return Err(NonoError::TrustPolicy(format!(
        "{}: unrecognised trust policy predicate '{}'", path.display(), sanitize(other)))),
    None => {
        // Legacy nono policy (pre-predicate): accept + warn, do not skip.
        // Only a file that also fails TrustPolicy deserialization is "foreign".
        return match trust::load_policy_from_str(&content) {
            Ok(p) => { warn_legacy(path); Ok(Some(p)) }
            Err(_) => Ok(None),   // genuinely foreign JSON
        };
    }
}
```

```rust
// trust_cmd.rs:1591 — never early-return a default policy when a project file exists
let project_policy = crate::trust_scan::load_nono_policy(&cwd_policy)?
    .ok_or_else(|| NonoError::TrustPolicy(format!(
        "{} exists but is not a nono trust policy; refusing to run with trust \
         enforcement silently disabled (remove the file or add the predicate)",
        cwd_policy.display())))?;
```

Also tighten `TrustPolicy::validate_version()` to require `predicate.is_some()` so the
library and CLI agree on what a valid policy is.

---

### CR-03: `reap_reparented_orphans` early-return skips the final network-notification drain, silently losing denial records

**File:** `crates/nono-cli/src/exec_strategy.rs:3452-3456` (new) vs `:3484-3493` (existing)

**Issue:** The new reaper is called at the top of every supervisor-loop iteration, *before*
`waitpid(child, WNOHANG)`:

```rust
if let Some(status) = reap_reparented_orphans(child) {
    return Ok((status, denials, ipc_denials));       // <-- no drain
}

match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
    ...
    Ok(status) => {
        drain_pending_network_notifications(         // <-- the drain the new path skips
            proxy_notify_raw_fd, config, &mut rate_limiter,
            &mut denials, &mut ipc_denials,
        );
        return Ok((status, denials, ipc_denials));
    }
```

`reap_reparented_orphans` calls `waitpid(-1, WNOHANG)`, which reaps **any** terminated
direct child — including the tracked primary. Because it runs first, the primary child's
exit is now essentially always observed here, not at `:3484`. That makes the
`Ok(status)` arm at `:3484` dead code on the normal exit path, and
`drain_pending_network_notifications` never runs.

That function is what collects network seccomp-notify events still queued at child exit
into `denials` / `ipc_denials`. Those vectors feed the denial diagnostic footer and the
audit/session denial record. So this phase silently drops end-of-run network denial
telemetry on every Linux supervised run with `proxy_notify_raw_fd` active — exactly the
data a security tool must not lose. The ECHILD arm (`:3495-3505`) has the same drain and is
likewise now unreachable.

**Fix:** drain before returning from the reaper path (and factor the shared exit path):

```rust
if let Some(status) = reap_reparented_orphans(child) {
    drain_pending_network_notifications(
        proxy_notify_raw_fd,
        config,
        &mut rate_limiter,
        &mut denials,
        &mut ipc_denials,
    );
    return Ok((status, denials, ipc_denials));
}
```

---

### CR-04: `SeccompOpts::external_tcp()` is an unused, fail-open network escape hatch in the core library

**File:** `crates/nono/src/sandbox/linux.rs:1136-1206`, `:724-732`, `:752-755`;
`crates/nono/src/sandbox/mod.rs:683-733`; `crates/nono/src/lib.rs:102-106`

**Issue:** `Sandbox::apply_seccomp(caps, SeccompOpts::external_tcp())` applies filesystem
and process sandboxing but sets `handle_tcp = false`, which:

- **Silently ignores `CapabilitySet`'s network mode.** With `NetworkMode::BlockAll` or
  `ProxyOnly`, `needs_network_handling` is forced `false` (`:752-755`), no
  `handle_access(AccessNet)` is installed, no `NetPort` rules are added
  (`landlock_network_active` stays `false`), and no seccomp fallback is installed. The
  caller's declared network policy is discarded with only an `info!` log. This directly
  contradicts CLAUDE.md's *"The library applies ONLY what's in `CapabilitySet`"* — here it
  applies *less* than what is in it.
- **Skips the `open_port 0` fail-closed validation** (`:724-732` now gated on `handle_tcp`),
  so a configuration that is a hard error on Linux is accepted.
- **Returns `Ok(SeccompNetFallback::None)`** — the same discriminant that, on every other
  code path, means "Landlock is handling the network, no fallback needed". A caller
  branching on the return value (as the CLI does for `SeccompNetFallback::ProxyOnly`) cannot
  distinguish "network enforced by Landlock" from "network not enforced at all".
- **`apply_external()` (`:1211-1215`) is a pure no-op** returning `Ok(())`, yet its doc
  comment on `sandbox/mod.rs:726-728` states *"Returns an error if the marker cannot be
  recorded"*. Nothing is recorded anywhere — the doc describes behaviour that does not exist.

Additionally, `grep` across the workspace shows **zero callers** of `apply_seccomp`,
`apply_seccomp_with_abi`, `apply_external`, or `SeccompOpts::external_tcp()` outside the
declaration sites and one unit test that only asserts `handles_tcp()`. This is untested,
unused, externally-reachable API (re-exported from `nono::lib.rs:102-106`) in a published
security crate — also a violation of CLAUDE.md's "avoid dead code / if code is unused,
either remove it or write tests that use it".

**Fix:** either remove the surface until it has a real consumer, or make the degraded state
impossible to mistake for enforcement:

```rust
pub enum SeccompNetFallback {
    None,
    BlockAll,
    ProxyOnly { .. },
    /// TCP enforcement was explicitly delegated to infrastructure; nono
    /// installed NO network policy. Callers MUST treat this as unenforced.
    ExternalTcp,
}

fn apply_with_abi_inner(...) -> Result<SeccompNetFallback> {
    if !handle_tcp {
        // Fail secure: refuse to silently drop a restrictive declared mode.
        if !matches!(caps.network_mode(), NetworkMode::AllowAll) {
            return Err(NonoError::SandboxInit(
                "SeccompOpts::external_tcp() cannot be combined with a restrictive \
                 NetworkMode; the declared policy would be silently dropped".into()));
        }
    }
    ...
}
```

And either delete `apply_external()` or have it record something observable
(e.g. into `SandboxState`) so the doc comment becomes true.

## Warnings

### WR-01: `waitpid(-1)` can steal the exit status of unrelated `std::process::Child` handles

**File:** `crates/nono-cli/src/exec_strategy.rs:3359-3378`

**Issue:** `reap_reparented_orphans` uses `waitpid(Some(Pid::from_raw(-1)), WNOHANG)`, which
is process-global. Any `std::process::Child` alive in the same process whose status has not
yet been collected can be reaped out from under libstd, so the eventual `Child::wait()` /
`wait_with_output()` fails with `ECHILD` — and, worse, the freed PID can be recycled and
waited on by mistake. A concrete instance already exists in-tree:
`hook_runtime::run_hook` (`hook_runtime.rs:397-430`) spawns a worker thread that holds
`child.wait_with_output()`; on hook **timeout** `run_hook` returns while that thread is
still blocked, so the orphaned waiter outlives the hook and overlaps the supervised run.

**Fix:** track pids the supervisor is allowed to reap, or centralize reaping behind a
single owner. Minimal mitigation — only reap pids the supervisor knows about:

```rust
fn reap_reparented_orphans(child: Pid, tracked: &HashSet<Pid>) -> Option<WaitStatus> {
    // ... skip (and do not consume) statuses for pids in `tracked`
}
```
If that is impractical, replace `wait_with_output()`-in-a-thread in `run_hook` with a
supervisor-owned reap queue so there is exactly one `waitpid` caller per process.

---

### WR-02: `discard_late_terminal_input` consumes and discards a byte of real user type-ahead

**File:** `crates/nono-cli/src/pty_proxy.rs:1562-1613` (loop at `:1596-1612`)

**Issue:** The drain loop `read()`s one byte, *then* feeds it to `CprReplyParse::step`. When
`step` returns `false` the byte has already been consumed from the terminal and is dropped
on the floor. With no CPR reply pending, the first byte the user typed ahead is always
lost. The phase's own test encodes the loss as expected behaviour:

```rust
// pty_proxy.rs test: discard_late_terminal_input_preserves_type_ahead_with_no_reply
nix::unistd::write(&master, b"xRy")...;
assert_eq!(&buf[..n], b"Ry", "non-reply type-ahead must be preserved");
//                    ^^^^ the 'x' the user typed is gone
```

This contradicts the function's own doc — *"queued type-ahead (which fails the grammar) is
left for the shell rather than scanned for a stray `R`"* — which is only true from the
second byte onward.

**Fix:** peek before consuming. Either `recv(fd, .., MSG_PEEK)` and only `read()` the byte
once the grammar accepts it, or bail out entirely unless the first readable byte is `ESC`:

```rust
let mut byte = [0u8; 1];
let n = unsafe { libc::recv(fd, byte.as_mut_ptr().cast(), 1, libc::MSG_PEEK) };
if n != 1 || !parse.clone().step(byte[0]) { break; }        // leave it for the shell
let _ = unsafe { libc::read(fd, byte.as_mut_ptr().cast(), 1) };  // now consume
parse.step(byte[0]);
```

---

### WR-03: `discard_late_terminal_input` aborts on `EINTR` in `poll()` — the common case at teardown

**File:** `crates/nono-cli/src/pty_proxy.rs:1598-1603`

**Issue:** The `read()` arm explicitly retries `ErrorKind::Interrupted`, but the `poll()`
arm does not:

```rust
let ready = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
if ready <= 0 || (pfd.revents & libc::POLLIN) == 0 {
    break;   // EINTR is indistinguishable from timeout/error here
}
```

This runs on the *final teardown path*, immediately after the child exits — precisely when
`SIGCHLD` (and `SIGWINCH` on a resize) is most likely to interrupt the 30 ms poll. When that
happens the drain aborts early and the CPR reply is handed to the shell, i.e. the exact bug
this code was added to fix, intermittently.

**Fix:**

```rust
if ready < 0 {
    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
        continue;   // deadline still bounds the loop
    }
    break;
}
if ready == 0 || (pfd.revents & libc::POLLIN) == 0 { break; }
```

---

### WR-04: The `--no-auth ⇒ loopback` fail-secure guard lives only in the CLI, not in the proxy crate

**File:** `crates/nono-cli/src/proxy_command.rs:36-42`; `crates/nono-proxy/src/server.rs`
(`start`), `crates/nono-proxy/src/config.rs:98-125`

**Issue:** `ProxyConfig { require_auth: false, bind_addr: <non-loopback> }` is rejected only
by the guard in `run_proxy`. `nono_proxy::server::start()` accepts it unconditionally. Both
new fields are `Deserialize` (`require_auth` via `default_require_auth`,
`strict_connect_auth` via `#[serde(default)]` = `false`), so any current or future
`ProxyConfig` deserialization — or any external embedder of the published `nono-proxy`
crate — can create an unauthenticated proxy reachable from the network, with credential
injection routes attached. Defense in depth requires the invariant at the enforcement
boundary, not only at the CLI.

**Fix:** re-assert it in `server::start`:

```rust
if !config.require_auth && !config.bind_addr.is_loopback() {
    return Err(ProxyError::Config(
        "require_auth=false requires a loopback bind_addr; refusing to start an \
         open proxy reachable from other hosts".into()));
}
```

---

### WR-05: Non-loopback `--listen` exposes the session token and injected credentials over plaintext HTTP

**File:** `crates/nono-cli/src/cli.rs:2334-2341` (`listen`), `proxy_command.rs:196-216`

**Issue:** With auth enabled, `--listen 0.0.0.0` is permitted. The proxy speaks plain HTTP:
the session token traverses the network in a `Proxy-Authorization` header, and
credential-injection routes mint real upstream secrets for whoever presents it. There is no
TLS, no per-source-IP restriction, and no failed-auth rate limiting or lockout. The command
also prints `NONO_PROXY_TOKEN=<token>` to stderr, which lands in CI logs by default.

The help text is also misleading: `"Address the proxy listens on (loopback only unless
--no-auth is omitted)"` reads as if loopback is always required.

**Fix:** require an explicit opt-in for non-loopback binds and reword the help:

```rust
/// Address the proxy listens on. Non-loopback addresses expose the
/// session token and any injected credentials over plaintext HTTP and
/// require --allow-remote.
```
Add `--allow-remote`, and gate credential-injection routes off entirely (or warn loudly)
when `bind_addr` is not loopback.

---

### WR-06: `--allow-gpu` now makes seccomp user-notification mandatory and fatal on every host

**File:** `crates/nono-cli/src/sandbox_prepare.rs:833-835`;
`crates/nono-cli/src/exec_strategy.rs:1331-1400`;
`crates/nono-cli/src/execution_runtime.rs:476-486`;
`crates/nono-cli/src/command_runtime.rs:371-380`;
`crates/nono/src/sandbox/linux.rs:616-625`

**Issue:** `proc_comm_notify: args.allow_gpu` is set unconditionally for `--allow-gpu`,
regardless of whether NVIDIA hardware is present. Combined with the (otherwise correct)
change making seccomp-notify setup fatal, `--allow-gpu` now hard-fails the child with
`_exit(126)` on any host where `install_seccomp_notify()` fails — WSL2 (explicit error),
containers without `CAP_SYS_ADMIN` in the user namespace (Docker/Kubernetes default), and
kernels without `SECCOMP_FILTER_FLAG_NEW_LISTENER`. Meanwhile the Landlock grant for
`/proc/self/task` was downgraded from `ReadWrite` to `Read`, so the previously working
non-supervisor path is gone. `nono wrap --allow-gpu` is now a hard error outright
(`command_runtime.rs:371-380`).

The in-code justification — *"safe even when no NVIDIA hardware is actually present (the
path simply never matches)"* — addresses the mediation scope but not the availability cost:
enabling the flag also makes the whole seccomp-notify handshake required.

**Fix:** gate `proc_comm_notify` on actual NVIDIA presence rather than on the flag, reusing
the `nvidia_present` signal that `collect_linux_gpu_paths()` already computes:

```rust
#[cfg(target_os = "linux")]
proc_comm_notify: args.allow_gpu && nono::sandbox::nvidia_devices_present(),
```
And add a release-note entry: `nono wrap --allow-gpu` is no longer supported on Linux.

---

### WR-07: New `unsafe` blocks lack the mandated `// SAFETY:` comments

**File:** `crates/nono-cli/src/exec_strategy.rs:1084`;
`crates/nono/src/sandbox/linux.rs:5364`

**Issue:** CLAUDE.md: *"Unsafe Code: Restrict to FFI; must be wrapped in safe APIs with
`// SAFETY:` docs."* Two new unsafe blocks have none:

```rust
// exec_strategy.rs:1084 — no SAFETY comment
let ret = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };
```
```rust
// linux.rs:5364 — no SAFETY comment; also fork() from a (multi-threaded) test harness
let pid = unsafe { libc::fork() };
```
The neighbouring code in the same functions does carry `// SAFETY:` comments, so this is
inconsistent as well as non-conforming.

**Fix:** add the justifications:

```rust
// SAFETY: prctl with PR_SET_CHILD_SUBREAPER takes only scalar arguments and
// mutates no memory; the return value is checked below.
let ret = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };
```

---

### WR-08: `restrict_execute()`'s `Refer`-on-`/` grant is only safe because of an unenforced ordering precondition

**File:** `crates/nono/src/sandbox/linux.rs:1219-1290`

**Issue:** The new layer does `handle_access(AccessFs::Refer)` + `add_rule(PathBeneath::new("/",
Refer))`, and the doc asserts *"Bare `Refer` alone can't widen access."* That is true only
because Landlock layers intersect **and** the first layer (`apply`/`apply_with_abi`) also
handles `Refer` and grants it narrowly. `restrict_execute` is `pub` and its doc only says
"Call this after `apply()`" — nothing enforces it. Called standalone (or on an ABI where
the earlier layer's `Refer` was dropped under `CompatLevel::BestEffort`), this layer imposes
**no** rename/link restriction anywhere on the filesystem while appearing to be a
restriction layer.

**Fix:** make the precondition explicit and checkable — e.g. take the `DetectedAbi`/prior
`RulesetStatus` as a parameter, or document it as a hard `# Panics`/`# Errors`
precondition and add a debug assertion that a prior nono layer is active
(`SandboxState`/`landlock_scope_policy` already tracks enough to do this).

---

### WR-09: `ctrl_c()` future is recreated every `select!` iteration and can miss a SIGINT

**File:** `crates/nono-cli/src/proxy_command.rs:168-180`

**Issue:**

```rust
loop {
    tokio::select! {
        signal = tokio::signal::ctrl_c() => { ... break; }
        _ = audit_drain.tick() => { let _ = handle.drain_audit_events(); }
    }
}
```
`tokio::signal::ctrl_c()` constructs a fresh listener each iteration. A SIGINT delivered
after the previous future is dropped and before the new one registers is not observed by
the new listener, so the first Ctrl-C can be swallowed — and because tokio has replaced the
default SIGINT disposition, the process does not die either. The user must press Ctrl-C
again.

**Fix:** bind the stream once, outside the loop:

```rust
let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
loop {
    tokio::select! {
        _ = sigint.recv() => break,
        _ = audit_drain.tick() => { let _ = handle.drain_audit_events(); }
    }
}
```
(or hoist `let ctrl_c = tokio::signal::ctrl_c(); tokio::pin!(ctrl_c);` before the loop.)

---

### WR-10: Breaking public-API changes to the core `nono` crate with an inert deprecation shim

**File:** `crates/nono/src/trust/types.rs:11-19`, `:37-49`; `crates/nono/src/trust/mod.rs:65-67`

**Issue:** Three source-breaking changes ship together:

- `TrustPolicy.version` changed from `u32` to `Option<u32>`, and a new `predicate:
  Option<String>` field was added — every external struct-literal construction breaks.
- `TRUST_POLICY_VERSION` was removed from the `nono::trust` re-export list
  (`mod.rs:65-67`) in the same commit that marked it `#[deprecated(since = "0.66.0", ...
  "will be removed in v1.0.0")]`. Because `types` is a private module, the constant is now
  unreachable from outside the crate — so the deprecation shim can never be observed by any
  downstream consumer. It was effectively removed immediately, not deprecated.

Per MEMORY, `nono-py` and `nono-ts` are separate repos that have historically broken on
exactly this kind of core-crate struct drift.

**Fix:** re-export the deprecated constant for one release so the deprecation is real, and
note the `TrustPolicy` field changes in the changelog:

```rust
#[allow(deprecated)]
pub use types::TRUST_POLICY_VERSION;
```
Also verify `nono-py`/`nono-ts` still build against the new `TrustPolicy` shape.

---

### WR-11: Parser differential between the `predicate` peek and the full parse

**File:** `crates/nono-cli/src/trust_scan.rs:56-79`

**Issue:** The peek is `raw.get("predicate").and_then(|v| v.as_str())`. A `predicate` that
is present but not a JSON string — `{"predicate": 1, ...}` or
`{"predicate": {"$ref": "..."}}` — yields `None` from `as_str()` and is therefore treated
identically to "no predicate at all": a silent skip into the fail-open default of CR-02,
with a misleading message ("has no predicate field"). A full `TrustPolicy` deserialization
would have rejected the type mismatch. The two parsers disagree about what "the predicate
field" means.

Second, smaller differential: the `serde_json::from_str::<Value>` error is reported as
`"failed to parse {path}"` even for a plainly foreign, well-formed-JSON-but-not-nono file,
which is the case the loader was introduced to handle gracefully.

**Fix:** distinguish the three cases explicitly:

```rust
match raw.get("predicate") {
    None => { /* absent: legacy or foreign — see CR-02 fix */ }
    Some(serde_json::Value::String(p)) if p == TRUST_POLICY_PREDICATE => {}
    Some(serde_json::Value::String(other)) => return Err(unrecognised(other)),
    Some(_) => return Err(NonoError::TrustPolicy(format!(
        "{}: 'predicate' must be a string", path.display()))),
}
```

---

### WR-12: The standalone proxy discards its entire network audit buffer

**File:** `crates/nono-cli/src/proxy_command.rs:162-179`

**Issue:**

```rust
_ = audit_drain.tick() => {
    let _ = handle.drain_audit_events();
}
```
`nono proxy` is, by construction, the *only* enforcement boundary in its mode — there is no
OS sandbox behind it. Yet every network audit event it produces (allow/deny decisions,
authentication failures, credential-route usage) is drained and immediately dropped on the
floor to keep the ring buffer from filling. So the one mode with no kernel enforcement also
has zero auditability. The module doc frames this as intentional, but "nothing else
consumes it" is a gap to close, not a justification.

**Fix:** at minimum emit the drained events through `tracing` at `info!`, and offer
`--audit-log <PATH>` to persist them:

```rust
_ = audit_drain.tick() => {
    for ev in handle.drain_audit_events() {
        tracing::info!(target: "nono_security", event = ?ev, "proxy network audit");
    }
}
```

## Info

### IN-01: `--no-auth` on loopback is still an open proxy for every local user

**File:** `crates/nono-cli/src/proxy_command.rs:36-42`, `:207-215`

**Issue:** The guard limits `--no-auth` to loopback, but loopback is reachable by every
local user and every container sharing the network namespace. In `--no-auth` mode any of
them can drive the credential-injection routes and obtain real upstream secrets. The banner
warns, but the warning does not mention the local multi-user exposure.

**Fix:** extend the warning text — *"every local process, including other users on this
host, can use this proxy and any credentials it injects"* — and consider requiring
`SO_PEERCRED`/uid matching when `require_auth` is false.

### IN-02: `NONO_PROXY_MAX_CONNECTIONS=0` silently disables the connection ceiling

**File:** `crates/nono-cli/src/cli.rs:2350-2359`

**Issue:** `max_connections` is env-settable and `0` means unlimited per
`ProxyConfig.max_connections`'s doc. An ambient env var can therefore remove the DoS
ceiling without appearing on the command line.

**Fix:** reject `0` from the env path, or require an explicit `--max-connections 0` to opt
into unlimited.

### IN-03: `--listen` help text is self-contradictory

**File:** `crates/nono-cli/src/cli.rs:2334-2341`

**Issue:** `"Address the proxy listens on (loopback only unless --no-auth is omitted)"` —
the double negative inverts on a quick read. The actual rule is "non-loopback is allowed
only when `--no-auth` is *not* used".

**Fix:** `"Address the proxy listens on. --no-auth requires a loopback address."`

### IN-04: `test_restrict_execute_does_not_break_rename_into_new_subdir` uses a predictable temp path and raw `fork()`

**File:** `crates/nono/src/sandbox/linux.rs:5345-5400`

**Issue:** The test builds `std::env::temp_dir().join(format!("nono-restrict-execute-rename-test-{}", std::process::id()))`
— a predictable name in a world-writable directory, then `create_dir_all` + `write` into
it (pre-existing-symlink hazard on a shared host), and calls `libc::fork()` from the
multi-threaded cargo test harness before allocating in the child. It also asserts hard on
`apply()`/`restrict_execute()` success (exit codes 2/3), which will fail rather than skip in
CI sandboxes where Landlock reports an ABI but `restrict_self` is blocked.

**Fix:** use `tempfile::tempdir()` for the root, and treat exit codes 2/3 as "skip" rather
than "fail" so the test degrades gracefully on constrained runners.

### IN-05: `socket_access_run.rs` allowlist assertion was materially weakened

**File:** `crates/nono-cli/tests/socket_access_run.rs:176-192`

**Issue:** The assertion changed from "stderr must contain no unix-socket denial at all" to
`!stderr.contains(&socket_arg)`. `socket_arg` is a full path that can appear in stderr for
reasons unrelated to a denial (e.g. echoed in a diagnostic hint), so the test can now pass
on a genuine regression *and* fail spuriously. The rationale in the comment is sound, but
the marker is imprecise.

**Fix:** assert against the formatter's actual denial block — e.g. that no line matching
`^\s*{socket_arg}$` appears *within* the `[nono] IPC denial:` block — rather than a bare
substring over all of stderr.

---

_Reviewed: 2026-08-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
