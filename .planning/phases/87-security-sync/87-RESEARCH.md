# Phase 87: Security Sync - Research

**Researched:** 2026-06-20
**Domain:** Linux seccomp BPF filter hardening, capability dedup guard, audit integrity
**Confidence:** HIGH — all three items traced to actual upstream diffs and fork source code

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Hybrid gate: no unix-socket grant → static `SECCOMP_RET_ERRNO(EPERM)` baked into filter; grant exists → route to existing USER_NOTIF supervisor for per-call `sockaddr_un` destination validation.
- **D-02:** No-grant deny is fail-secure silent EPERM. Opportunistic structured-diagnostic remediation via `NonoError::remediation` is nice-to-have, not a blocker.
- **D-03:** The `linux.rs:844-847` TODO is the anchor for SEC-01; implementer must resolve/update it.
- **D-04:** `git cherry-pick -x` per commit (one atomic commit per SHA: `e2086877` then `6b3eb013`), each with DCO `Signed-off-by` trailer.
- **D-05:** Expect conflicts in security hunks; port upstream semantics onto fork structures, do NOT accept upstream wholesale.
- **D-06:** Port upstream test matrix adapted to fork's test module, plus at least one fork-specific test for grant-present `sockaddr_un` destination validation.
- **D-07:** Linux-execution leg is PARTIAL→CI (seccomp tests cannot run on Windows dev-host).
- **D-08:** Cross-target clippy mandatory on cfg-gated Unix edits; host has rustup std but no cross C-compiler → expected PARTIAL→CI.
- **D-09:** Confirm-then-port for SEC-02 — write regression test first.
- **D-10:** If test fails, port guard adapted to fork's divergent keying + deferred `original_updates`/`access_upgrades` logic.
- **D-11:** If test passes, keep test as regression guard and document why no code change was needed.
- **D-12:** Harden CR-02 here; record as deliberate fork-divergence in divergence ledger / ADR note.
- **D-13:** Defer CR-01 to Phase 88.

### Claude's Discretion

- Exact seccomp rule construction (BPF arg-matching for `AF_UNIX` family detection, abstract-namespace handling, `sendmmsg` iovec walking).
- Whether the connect-grant tracking needs supervisor-side state vs. a static filter decision.

### Deferred Ideas (OUT OF SCOPE)

- CR-01 (FFI stale `LAST_DIAGNOSTIC_CODE`) → Phase 88.
- No other scope creep raised.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SEC-01 | Close Linux AF_UNIX datagram bypass — trap `sendto`/`sendmsg`/`sendmmsg` in seccomp filter, gate on connect grant | Full upstream diff recovered; fork divergence mapped; port plan documented below |
| SEC-02 | Guard `deduplicate()` against inheriting procfs-remap originals (upstream #1064) | Upstream diff recovered; fork dedup machinery read; divergence analysis complete |

</phase_requirements>

---

## Summary

Phase 87 ports two upstream security commits (`e2086877` v0.64.0 and `6b3eb013` v0.63.0) onto the fork's Linux enforcement code, and adds the CR-02 audit-integrity hardening deferred from Phase 86.

**SEC-01 (`e2086877`):** The upstream fix expands both BPF filters (`build_seccomp_proxy_filter` and `build_seccomp_af_unix_filter`) to trap `sendto`/`sendmsg`/`sendmmsg` in addition to `connect`/`bind`. It adds three new `pub const SYS_SEND*` constants in `linux.rs`, a `pub fn read_msghdr_dest` helper and `pub fn read_mmsghdr_dests` helper for extracting destination addresses from `struct msghdr` and `struct mmsghdr` via `/proc/PID/mem`, a new `UnixSocketOp::Send` variant, and extends `handle_network_notification` in `supervisor_linux.rs` to dispatch on the new syscall numbers and fan out over the multi-message vector for `sendmmsg`. None of these are present in the fork today — confirmed by grep. The fork's `build_seccomp_af_unix_filter` exists but only traps `connect`/`bind` (5-instruction filter); upstream extended it to 8 instructions covering `sendto`/`sendmsg`/`sendmmsg`. The fork's `build_seccomp_proxy_filter` is currently 19 instructions; upstream expanded it to 23 (adding three send-family jmps before the IO_URING check).

**SEC-02 (`6b3eb013`):** The upstream fix adds a private `is_procfs_remap_original(path)` helper and guards both `original_updates` push sites in `deduplicate()` so that `/dev/stdin`, `/dev/stdout`, `/dev/stderr`, and `/dev/fd` are never inherited as the `original` field of a surviving direct entry. The fork's `deduplicate()` has its own divergent structure (platform-specific keying, deferred `original_updates`, `access_upgrades`), but the two guard sites are syntactically similar to upstream's — the patch is small (7 lines added, 2 changed) and localizes cleanly. Critically, `rewrite_procfs_self_reference` (the function `is_procfs_remap_original` delegates to) already exists in the fork at line 1804 and handles exactly `/dev/stdin|stdout|stderr|fd`. The upstream approach of delegating to the rewriter keeps the two in sync automatically. The regression test (D-09) must be run first to determine whether the fork's divergent keying already avoids the bug.

**CR-02:** `verify_audit_log` in `crates/nono/src/audit.rs` hardcodes `records_verified: true` at line 1406 regardless of whether `stored` is `None` (i.e., regardless of whether any cross-check was performed). When `stored` is `None`, the function still returns `records_verified: true`, which is semantically misleading: it signals "records verified" when in fact no stored metadata was supplied to verify against. The fix is minimal: set `records_verified` to `false` when `stored.is_none() && event_count == 0`, or more precisely, set it to reflect whether the loop completed without errors AND at least one integrity claim can be made. This must be recorded as a deliberate fork-divergence from upstream's identical code at `e9529312`.

**Primary recommendation:** Cherry-pick `e2086877` first (SEC-01 is larger and touches more files); resolve conflicts by porting the upstream semantics onto the fork's existing structures rather than accepting upstream wholesale; cherry-pick `6b3eb013` second (SEC-02 is smaller and conflict-free if SEC-01 landed correctly). Apply CR-02 as a separate fork-hardening commit with ADR/ledger note. All three produce `#[cfg(target_os = "linux")]`-gated code; cross-target clippy is mandatory but PARTIAL→CI on this Windows dev-host.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| BPF filter construction (SEC-01 datagram trap) | Library (`crates/nono/src/sandbox/linux.rs`) | — | Filter bytecode is a kernel-level primitive; it belongs in the library's sandbox module, not in CLI policy code |
| `read_msghdr_dest` / `read_mmsghdr_dests` helpers | Library (`linux.rs`) | — | They read from `/proc/PID/mem` using the same pattern as `read_notif_sockaddr`; keep in the same module |
| `SYS_SENDTO/SENDMSG/SENDMMSG` constants | Library (`linux.rs`) as `pub const` | — | Exported to CLI supervisor handler (same pattern as `SYS_CONNECT`/`SYS_BIND`) |
| `UnixSocketOp::Send` variant | Library (`capability.rs`) | — | `UnixSocketOp` lives in `capability.rs`; extending it stays there |
| `handle_network_notification` dispatch (sendto/sendmsg/sendmmsg) | CLI (`supervisor_linux.rs`) | — | Per-connection policy decision is CLI-side; library supplies the helpers |
| `decide_network_notification` `Send` arm | CLI (`supervisor_linux.rs`) | — | Same function; add `SYS_SENDTO|SYS_SENDMSG|SYS_SENDMMSG` to the `SYS_CONNECT` arm |
| `deduplicate()` procfs-remap guard (SEC-02) | Library (`capability.rs`) | — | Guard lives in the same function where `original_updates` is pushed |
| `is_procfs_remap_original` helper | Library (`capability.rs`) | — | Private helper; sits next to `rewrite_procfs_self_reference` at line 1804 |
| `verify_audit_log` CR-02 fix | Library (`audit.rs`) | — | Fix the `records_verified: true` hardcode; divergence ledger note records it |

---

## Standard Stack

No new external dependencies. All changes are pure Rust on existing crates.

| Component | File | Current State | After Phase 87 |
|-----------|------|---------------|----------------|
| `linux.rs` | `crates/nono/src/sandbox/linux.rs` | `build_seccomp_af_unix_filter` (5 insn), `build_seccomp_proxy_filter` (19 insn); no `SYS_SEND*` consts | 8-insn af_unix filter; 23-insn proxy filter; `SYS_SENDTO/SENDMSG/SENDMMSG` pub consts; `read_msghdr_dest`; `read_mmsghdr_dests` |
| `capability.rs` | `crates/nono/src/capability.rs` | `UnixSocketOp::{Connect, Bind}`; `deduplicate()` without procfs-remap guard | `UnixSocketOp::Send` variant; `is_procfs_remap_original` helper; guard at both `original_updates` push sites |
| `supervisor_linux.rs` | `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | `handle_network_notification` dispatches on `SYS_CONNECT`/`SYS_BIND` only; `unix_socket_op_for_syscall` handles two variants | Extended dispatch for `SYS_SENDTO/SENDMSG/SENDMMSG`; `sockaddrs: Vec<SockaddrInfo>` loop; `read_msghdr_dest`/`read_mmsghdr_dests` imported |
| `audit.rs` | `crates/nono/src/audit.rs` | `records_verified: true` hardcoded at line 1406 | `records_verified` set correctly based on whether any record-level checks ran; divergence from upstream documented |

**Package Legitimacy Audit:** No external packages added. Skip.

---

## Architecture Patterns

### System Architecture Diagram

```
Sandboxed child process
        |
        | syscall: sendto / sendmsg / sendmmsg
        v
[seccomp BPF filter (linux.rs)]
        |
        |-- no unix-socket grant in CapabilitySet
        |        --> SECCOMP_RET_ERRNO(EPERM)  [static, fail-secure, no notify]
        |
        |-- unix-socket grant exists OR ProxyOnly mode active
                 --> SECCOMP_RET_USER_NOTIF
                        |
                        v
        [Supervisor: handle_network_notification (supervisor_linux.rs)]
                |
                |-- sendto: read sockaddr from args[4]/args[5]
                |-- sendmsg: read_msghdr_dest -> extract msg_name/msg_namelen
                |-- sendmmsg: read_mmsghdr_dests -> Vec<Option<(addr,len)>>
                |-- for each sockaddr: decide_network_notification()
                        |
                        |-- AF_UNIX pathname: check against unix_socket_allowed(Send)
                        |-- AF_UNIX abstract/unnamed: deny
                        |-- AF_INET/6: check loopback + proxy_port
                        v
                [Allow: continue_notif] or [Deny: respond_notif_errno(EACCES)]
```

### Recommended Project Structure

No new directories. All changes are in-place edits to existing files:

```
crates/nono/src/
├── sandbox/linux.rs      # BPF filter expansion + SYS_SEND* consts + msghdr helpers
├── capability.rs         # UnixSocketOp::Send + is_procfs_remap_original + dedup guard
└── audit.rs              # CR-02: records_verified fix

crates/nono-cli/src/exec_strategy/
└── supervisor_linux.rs   # handle_network_notification dispatch + send arm
```

---

## SEC-01: Detailed Port Analysis

### The bypass (before fix)

Before the fix, a sandboxed process could do:

```c
int fd = socket(AF_UNIX, SOCK_DGRAM, 0);
// connect() is trapped → checked against allowlist → denied
// But sendto() is NOT trapped in either filter:
sendto(fd, data, len, 0, (struct sockaddr *)&dest_addr, sizeof(dest_addr));
// This bypasses the AF_UNIX pathname allowlist entirely
```

The BPF filter only matched `SYS_CONNECT` and `SYS_BIND`; `SYS_SENDTO`/`SYS_SENDMSG`/`SYS_SENDMMSG` fell through to `SECCOMP_RET_ALLOW`.

### Fork state: what already exists vs. what is missing

**Already exists in fork** (confirmed by code read):
- `build_seccomp_af_unix_filter()` — 5-instruction filter trapping only `connect`/`bind`
- `build_seccomp_proxy_filter()` — 19-instruction filter trapping only `connect`/`bind`
- `read_notif_sockaddr()` — reads sockaddr from `/proc/PID/mem` (reused)
- `classify_af_unix()`, `UnixSocketKind`, `SockaddrInfo` — already present
- `unix_socket_allowed(path, op)` on `CapabilitySet` — already present
- `handle_network_notification()` in `supervisor_linux.rs` — already dispatches on `SYS_CONNECT`/`SYS_BIND`
- `decide_network_notification()` — already handles `SYS_CONNECT` + AF_UNIX pathname routing

**Missing from fork** (confirmed by grep returning no matches):
- `SYS_SENDTO`, `SYS_SENDMSG`, `SYS_SENDMMSG` pub constants
- `UnixSocketOp::Send` variant
- `read_msghdr_dest()` helper
- `read_mmsghdr_dests()` helper
- Extended BPF filters (af_unix: 8 insn; proxy: 23 insn)
- sendto/sendmsg/sendmmsg arms in `handle_network_notification`
- Send arm in `unix_socket_op_for_syscall` and `unix_socket_allowlist_allows`

### The hybrid gate (D-01)

The fork's `build_seccomp_af_unix_filter` is the "grant exists" path — it is installed when the `CapabilitySet` has unix-socket grants, routing `connect`/`bind` to `USER_NOTIF` for per-call validation. After the port, this filter also routes `sendto`/`sendmsg`/`sendmmsg` to `USER_NOTIF`.

The no-grant path (static `SECCOMP_RET_ERRNO(EPERM)` per D-01): this is distinct from the `build_seccomp_block_network_filter` (which is the `BlockAll` network-mode filter, installed when no TCP is allowed at all). For SEC-01, the no-grant AF_UNIX case must deny datagram sends even when TCP/Landlock is in play. Review the install path in `apply_with_abi` after porting to ensure the AF_UNIX datagram trap is installed whenever there are unix-socket grants, independent of network mode.

### BPF mechanics: why seccomp can trap on syscall NR alone

Classic BPF (`SECCOMP_SET_MODE_FILTER`) can load and compare syscall numbers from the `seccomp_data` struct. The struct layout is:

```
offset 0:  nr       (i32) — syscall number  [SECCOMP_DATA_NR_OFFSET = 0]
offset 4:  arch     (u32)
offset 8:  ip       (u64)
offset 16: args[0]  (u64)  [SECCOMP_DATA_ARG0_OFFSET = 16]
...
```

The BPF filter loads at offset 0 (`BPF_LD | BPF_W | BPF_ABS, k=0`) to get the syscall number, then uses `BPF_JMP | BPF_JEQ | BPF_K` to branch. The BPF instruction field at syscall-dispatch time is `k` = the syscall NR as a 32-bit constant. BPF CANNOT dereference a pointer (args hold virtual addresses that live in the child's address space). This is why:

- For `sendto`: BPF routes all `SYS_SENDTO` to `USER_NOTIF`; the supervisor then checks `args[4]` (dest_addr pointer). If `args[4] == 0` (connected socket, no per-call destination) → `continue_notif` immediately. If `args[4] != 0` → read sockaddr from `/proc/PID/mem` using `read_notif_sockaddr`.
- For `sendmsg`: BPF routes all `SYS_SENDMSG` to `USER_NOTIF` unconditionally (BPF cannot dereference `msghdr*` from args[1]). The supervisor reads `args[1]` (msghdr pointer), uses `read_msghdr_dest` to extract `msg_name` and `msg_namelen`. If `msg_name == NULL` → `continue_notif`. If not NULL → read sockaddr.
- For `sendmmsg`: BPF routes all `SYS_SENDMMSG` to `USER_NOTIF`. The supervisor reads each entry in the `mmsghdr` array (each starts with a `msghdr`). Up to `MAX_MMSGHDRS = 1024` entries; entries without a destination are skipped. If ALL entries have NULL destinations → `continue_notif`.

**Abstract-namespace AF_UNIX handling:** Abstract sockets (`sun_path[0] == 0`) are detected by `classify_af_unix()` → `UnixSocketKind::Abstract` → denied by `decide_network_notification`. This applies to sendto/sendmsg/sendmmsg exactly as to connect (they all route through `decide_network_notification` after extracting the sockaddr).

### `read_msghdr_dest` implementation (to port)

`struct msghdr` on x86_64 Linux:
```
offset 0: msg_name*     (void*, 8 bytes)
offset 8: msg_namelen   (socklen_t, 4 bytes)
```
Read 12 bytes from `/proc/PID/mem` at `msghdr_ptr`. Parse `msg_name` as `u64::from_ne_bytes` and `msg_namelen` as `u32::from_ne_bytes` cast to `u64`. Use `read_exact` (not manual length check). Return `Ok(None)` if `msg_name == 0`, else `Ok(Some((msg_name, msg_namelen as u64)))`.

### `read_mmsghdr_dests` implementation (to port)

`struct mmsghdr` contains `struct msghdr` at offset 0 (the msg_hdr field), followed by `msg_len: u32`. On x86_64 Linux, `sizeof(mmsghdr) = 28`. The stride is `std::mem::size_of::<libc::mmsghdr>()`. For each index `0..vlen`, compute `offset = idx * stride`, check arithmetic won't overflow (use `checked_mul`/`checked_add`), call `read_msghdr_dest(pid, msgvec_ptr + offset)`. Cap `vlen` at 1024 entries. Return `Vec<Option<(u64, u64)>>`.

---

## SEC-02: Detailed Port Analysis

### The bug (before fix)

In `--detached` mode, nono sets stdin/stdout to `/dev/null`. `system_read_linux_core` adds two `FsCapability` entries with the same `resolved` path (`/dev/null`): one with `original = /dev/null` (direct entry), one with `original = /dev/stdin` (symlink entry, canonicalized to `/dev/null`).

`deduplicate()` (Linux path, keyed on `resolved`) sees both entries share the key `(/dev/null, true)`. The existing entry is kept; the discarded entry is the `/dev/stdin` one. The `original_updates` branch fires: `existing.original == existing.resolved` (both are `/dev/null`) and `cap.original != cap.resolved` (`/dev/stdin != /dev/null`), so it pushes `(existing_idx, /dev/stdin)`. After the deferred update, the surviving entry now has `original = /dev/stdin`.

`remap_procfs_self_references` later calls `rewrite_procfs_self_reference` on each entry's `original`. For `original = /dev/stdin` it returns `Some(/proc/{pid}/fd/0)` and writes that into `resolved`. The Landlock rule is now attached to `/proc/{pid}/fd/0` (the PTY slave), not `/dev/null`. The sandboxed process is denied `/dev/null` with EACCES.

### Fork divergence analysis

The fork's `deduplicate()` has the same structure as upstream's `original_updates` logic (it was ported earlier). The bug reproduces in the fork. Evidence:

1. `rewrite_procfs_self_reference` handles `/dev/stdin`, `/dev/stdout`, `/dev/stderr`, `/dev/fd` (confirmed at lines 1815-1822).
2. Both `original_updates` push sites in the fork's `deduplicate()` have the exact same conditions as pre-fix upstream (lines 1587-1589 and 1598-1600 — no `is_procfs_remap_original` guard).
3. `is_procfs_remap_original` does NOT exist in the fork (confirmed by reading the source).

Therefore, D-09 outcome: the regression test (per upstream's `remap_preserves_dev_null_when_deduped_with_dev_stdin`) will FAIL on the fork → D-10 applies.

### Guard to port

Upstream adds `is_procfs_remap_original(path: &Path) -> bool` (Linux-cfg-gated) defined as:

```rust
#[cfg(target_os = "linux")]
fn is_procfs_remap_original(path: &Path) -> bool {
    rewrite_procfs_self_reference(path, 0, None).is_some()
}
```

Then guards both `original_updates` push sites:

**Site 1** (`keep_new` branch, after `seen.insert(key, i)`):
```rust
#[cfg(target_os = "linux")]
if cap.original == cap.resolved
    && existing.original != existing.resolved
    && !is_procfs_remap_original(&existing.original)  // NEW guard
{
    original_updates.push((i, existing.original.clone()));
}
```

**Site 2** (`!keep_new` branch, after `to_remove.push(i)`):
```rust
#[cfg(target_os = "linux")]
if existing.original == existing.resolved
    && cap.original != cap.resolved
    && !is_procfs_remap_original(&cap.original)  // NEW guard
{
    original_updates.push((existing_idx, cap.original.clone()));
}
```

This ports cleanly onto the fork's code without structural change. The fork's `access_upgrades` and deferred-update logic is orthogonal to the `original_updates` guard.

---

## CR-02: Detailed Analysis

### The bypass

`verify_audit_log(session_dir, stored: Option<&AuditIntegritySummary>)` always returns `records_verified: true` (line 1406 in the fork's `audit.rs`). This is the upstream-identical behavior (confirmed at upstream `e9529312` lines ~875/915).

The field name `records_verified` is interpreted by callers (`audit_commands.rs`) as "the record-level verification loop completed successfully." Since the function returns early via `Err` on any record-level failure (bad chain hash, bad leaf hash, bad sequence, etc.), it is ALWAYS true that when the function returns `Ok(...)`, no record-level error was found. So `records_verified: true` is not technically wrong for the records-processed case.

However, when `stored.is_none()` AND the log is empty, the function returns:
- `event_count: 0`
- `records_verified: true` (no records were verified — 0 iterations)
- `event_count_matches: true` (vacuously, `unwrap_or(true)`)
- `chain_head_matches: true` (vacuously)
- `merkle_root_matches: true` (vacuously)
- `is_valid()` returns `true`

This makes `is_valid()` return `true` for an empty log with no stored metadata — i.e., "nothing was verified and everything passed," which is a misleading audit integrity report.

### Fix

Set `records_verified` to reflect whether the record loop actually ran:

```rust
records_verified: event_count > 0,
```

Or, if keeping `records_verified` as a "no record-level error" flag (its original semantic), clarify the `is_valid()` or `AuditVerificationResult` semantics and add a test documenting the vacuous-true behaviour explicitly (per 86-REVIEW.md IN-02). The minimum acceptable fix for security purposes: if `stored.is_none()`, callers must know no stored claims were checked. The fork can diverge from upstream here deliberately.

**Fork-divergence record:** This fix introduces the first intentional divergence from upstream's audit code since Phase 86's convergence. Must be recorded in:
1. The divergence ledger (`.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md`) as a post-Phase-87 addendum — or in a new Phase 87 DIVERGENCE section if the ledger format supports it.
2. An ADR note (can be a short entry in `proj/ADR-86-library-boundary-convergence.md` or a new `proj/ADR-87-cr02-audit-bypass.md`).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Sockaddr reading from child memory | Custom procfs reader | Existing `read_notif_sockaddr` in `linux.rs` | Already handles AF_UNIX/INET/INET6 parsing, length bounds, byte order |
| msghdr field extraction | Manual offset arithmetic | Port `read_msghdr_dest` exactly as upstream | Upstream uses `read_exact` + `u32::from_ne_bytes` → correct on all endiannesses |
| mmsghdr vector walking | Unsafe pointer math | Port `read_mmsghdr_dests` with `checked_mul`/`checked_add` | Overflow checks required; upstream already handles them |
| BPF instruction table comments | Skip the in-code layout diagrams | Include full BPF comment table as upstream does | Instruction-index comments are the only way to audit BPF correctness without running on Linux |
| procfs-remap alias detection | Hardcoded list of `/dev/stdin` etc. | Delegate to `rewrite_procfs_self_reference(path, 0, None).is_some()` | Keeps guard and rewriter in sync automatically |
| Audit integrity bypass test | Manual log construction | Use `tempfile::tempdir()` + `std::fs::write` | Pattern already established in existing `audit_integrity.rs` tests |

---

## Common Pitfalls

### Pitfall 1: Forgetting the `sendmmsg` NULL-destination case
**What goes wrong:** `sendmmsg` with all-NULL `msg_name` entries should be allowed immediately (connected-socket fast path). If the supervisor returns `Deny` for this case, regular process-to-process connected datagram sends break.
**Why it happens:** Conflating "no per-message destination" (NULL `msg_name`) with "datagram bypass."
**How to avoid:** In `handle_network_notification` for `SYS_SENDMMSG`, if `sockaddrs` is empty after processing all entries (all had NULL `msg_name`), call `continue_notif` and return.

### Pitfall 2: BPF instruction count mismatch after adding sendto/sendmsg/sendmmsg
**What goes wrong:** The in-code jump offset tables use raw integer offsets from `next_insn`. Adding 3 instructions before existing ones shifts every subsequent `jt` value. Any un-updated `jt` silently jumps to the wrong instruction.
**Why it happens:** BPF is a flat array; adding instructions in the middle invalidates all forward-jump offsets.
**How to avoid:** Follow upstream's exact recount. `build_seccomp_af_unix_filter`: 5→8 instructions; `build_seccomp_proxy_filter`: 19→23 instructions. Update the comment table AND the `jt` values together. The existing unit tests (`test_build_seccomp_proxy_filter_with_bind`, `test_build_seccomp_af_unix_filter_notifies_*`) check instruction counts and specific instruction indices — port these updated assertions.

### Pitfall 3: `records_verified: true` fix breaks is_valid() callers that expect vacuous-true
**What goes wrong:** Callers that call `verify_audit_log(dir, None)` and then check `result.is_valid()` to decide if "nothing broke" will now get `false` for empty logs, potentially surfacing errors where none occurred.
**Why it happens:** The vacuous-true semantic was implicit contract of `records_verified`.
**How to avoid:** When fixing CR-02, add a test that documents the intended semantics: an empty log with `stored: None` should have `is_valid()` returning `false` (nothing verified = not valid). Update `is_valid()` doc-comment to explain vacuous-true vs. meaningful-true.

### Pitfall 4: Cherry-pick conflicts in `supervisor_linux.rs` if fork diverged from upstream's handle_network_notification
**What goes wrong:** `e2086877` replaces the single `let sockaddr = ...` with `let sockaddrs: Vec<SockaddrInfo> = match notif.data.nr { ... }` and the single `match decide_network_notification(...)` with a `for sockaddr in &sockaddrs` loop. If the fork's copy has diverged (e.g., different denial record logic), the hunk won't apply cleanly.
**Why it happens:** Phase 86 cherry-picks touched `supervisor_linux.rs` (structured diagnostics).
**How to avoid:** Inspect the diff of the fork's `handle_network_notification` against upstream's pre-`e2086877` version before applying the cherry-pick. Accept the multi-sockaddr loop from upstream; merge fork-specific context (denial records, IPC records, audit records) into the new loop body.

### Pitfall 5: Cross-target clippy on `UnixSocketOp::Send` match exhaustiveness
**What goes wrong:** Any existing `match op { Connect => ..., Bind => ... }` becomes non-exhaustive when `Send` is added. The Windows host won't catch these because they're inside `#[cfg(target_os = "linux")]` or `#[cfg(unix)]` blocks.
**Why it happens:** `UnixSocketOp` is defined in `capability.rs` without cfg-gating; match arms that handle it inside `#[cfg(unix)]` blocks are invisible to the Windows clippy run.
**How to avoid:** After adding `UnixSocketOp::Send`, grep for every `match op` and `match unix_socket_op` in the codebase. Known sites: `unix_socket_allowlist_allows`, `decide_af_unix_pathname`, `ipc_denial_details`, `record_af_unix_ipc_denial` in `supervisor_linux.rs`. Port upstream's handling (`Connect | Send` arm) to each. The cross-target clippy (PARTIAL→CI) is the decisive gate.

---

## Code Examples

### BPF filter extension pattern (af_unix filter, 8-instruction post-fix)

```rust
// Source: upstream e2086877 -- build_seccomp_af_unix_filter
// Instruction layout:
//  0: ld  [nr]
//  1: jeq SYS_CONNECT   jt=+5 (-> 7: notify)
//  2: jeq SYS_BIND      jt=+4 (-> 7: notify)
//  3: jeq SYS_SENDTO    jt=+3 (-> 7: notify)
//  4: jeq SYS_SENDMSG   jt=+2 (-> 7: notify)
//  5: jeq SYS_SENDMMSG  jt=+1 (-> 7: notify)
//  6: ret ALLOW
//  7: ret USER_NOTIF
fn build_seccomp_af_unix_filter() -> Vec<SockFilterInsn> {
    vec![
        // 0: ld [nr]
        SockFilterInsn { code: BPF_LD | BPF_W | BPF_ABS, jt: 0, jf: 0, k: SECCOMP_DATA_NR_OFFSET },
        // 1: jeq SYS_CONNECT -> 7 (jt = 7-1-1 = 5)
        SockFilterInsn { code: BPF_JMP | BPF_JEQ | BPF_K, jt: 5, jf: 0, k: SYS_CONNECT as u32 },
        // 2: jeq SYS_BIND -> 7 (jt = 7-2-1 = 4)
        SockFilterInsn { code: BPF_JMP | BPF_JEQ | BPF_K, jt: 4, jf: 0, k: SYS_BIND as u32 },
        // 3: jeq SYS_SENDTO -> 7 (jt = 7-3-1 = 3)
        SockFilterInsn { code: BPF_JMP | BPF_JEQ | BPF_K, jt: 3, jf: 0, k: SYS_SENDTO as u32 },
        // 4: jeq SYS_SENDMSG -> 7 (jt = 7-4-1 = 2)
        SockFilterInsn { code: BPF_JMP | BPF_JEQ | BPF_K, jt: 2, jf: 0, k: SYS_SENDMSG as u32 },
        // 5: jeq SYS_SENDMMSG -> 7 (jt = 7-5-1 = 1)
        SockFilterInsn { code: BPF_JMP | BPF_JEQ | BPF_K, jt: 1, jf: 0, k: SYS_SENDMMSG as u32 },
        // 6: ret ALLOW
        SockFilterInsn { code: BPF_RET | BPF_K, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW },
        // 7: ret USER_NOTIF
        SockFilterInsn { code: BPF_RET | BPF_K, jt: 0, jf: 0, k: SECCOMP_RET_USER_NOTIF },
    ]
}
```

### `read_msghdr_dest` helper (to add to linux.rs)

```rust
// Source: upstream e2086877 -- read_msghdr_dest
// Reads msg_name pointer and msg_namelen from child's struct msghdr via /proc/PID/mem.
// Returns None if msg_name is NULL (connected socket, no per-message destination).
pub fn read_msghdr_dest(pid: u32, msghdr_ptr: u64) -> Result<Option<(u64, u64)>> {
    use std::io::Read;
    const MSGHDR_MIN_READ: usize = 12; // msg_name (8) + msg_namelen (4)

    let mem_path = format!("/proc/{}/mem", pid);
    let mut file = std::fs::File::open(&mem_path)
        .map_err(|e| NonoError::SandboxInit(format!("Failed to open {}: {}", mem_path, e)))?;
    std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(msghdr_ptr))
        .map_err(|e| NonoError::SandboxInit(format!("Failed to seek in {}: {}", mem_path, e)))?;
    let mut buf = [0u8; MSGHDR_MIN_READ];
    file.read_exact(&mut buf).map_err(|e| {
        NonoError::SandboxInit(format!("Failed to read msghdr from {}: {}", mem_path, e))
    })?;
    let msg_name = u64::from_ne_bytes([buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7]]);
    let msg_namelen = u32::from_ne_bytes([buf[8], buf[9], buf[10], buf[11]]) as u64;
    if msg_name == 0 {
        Ok(None)
    } else {
        Ok(Some((msg_name, msg_namelen)))
    }
}
```

### `is_procfs_remap_original` helper (to add to capability.rs)

```rust
// Source: upstream 6b3eb013 -- is_procfs_remap_original
// Private; placed next to rewrite_procfs_self_reference at line ~1804.
#[cfg(target_os = "linux")]
fn is_procfs_remap_original(path: &Path) -> bool {
    rewrite_procfs_self_reference(path, 0, None).is_some()
}
```

### SEC-02 regression test (to add)

```rust
// Source: upstream 6b3eb013 -- remap_preserves_dev_null_when_deduped_with_dev_stdin
#[cfg(target_os = "linux")]
#[test]
fn remap_preserves_dev_null_when_deduped_with_dev_stdin() {
    let dev_null = PathBuf::from("/dev/null");
    let mut caps = CapabilitySet::new();
    caps.add_fs(FsCapability {
        original: dev_null.clone(),
        resolved: dev_null.clone(),
        access: AccessMode::Read,
        is_file: true,
        source: CapabilitySource::Group("system_read_linux_core".to_string()),
    });
    caps.add_fs(FsCapability {
        original: PathBuf::from("/dev/stdin"),
        resolved: dev_null.clone(),
        access: AccessMode::Read,
        is_file: true,
        source: CapabilitySource::Group("system_read_linux_core".to_string()),
    });
    caps.deduplicate();
    assert_eq!(caps.fs_capabilities().len(), 1);
    assert_eq!(
        caps.fs_capabilities()[0].original,
        dev_null,
        "deduplicate must not rename /dev/null original to /dev/stdin"
    );
    caps.remap_procfs_self_references(4242, None);
    assert_eq!(
        caps.fs_capabilities()[0].resolved,
        dev_null,
        "resolved must remain /dev/null after remap"
    );
}
```

### CR-02 fix pattern (in audit.rs)

```rust
// Source: fork-hardening, deliberate divergence from upstream e9529312
// records_verified: reflects whether the record loop found no errors
// (always true when function returns Ok — but explicitly false when event_count == 0
// and stored is None so callers cannot confuse "nothing verified" with "all verified").
Ok(AuditVerificationResult {
    // ... other fields unchanged ...
    records_verified: event_count > 0,
    // ...
})
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| seccomp filters trap only `connect`/`bind` for AF_UNIX mediation | Filters trap `sendto`/`sendmsg`/`sendmmsg` too | v0.64.0 upstream (`e2086877`) | Closes SOCK_DGRAM datagram bypass |
| `deduplicate()` inherits any symlink original, including `/dev/stdin` | Guard prevents procfs-remap aliases from propagating as originals | v0.63.0 upstream (`6b3eb013`) | Fixes `--detached` mode `/dev/null` denial |
| `records_verified: true` hardcoded | `records_verified` reflects actual loop execution | Fork hardening (CR-02, Phase 87) | Makes `is_valid()` semantically honest for empty-log case |

---

## Project Constraints (from CLAUDE.md)

| Directive | Applies to Phase 87 |
|-----------|---------------------|
| No `.unwrap()` or `.expect()` — enforced by `clippy::unwrap_used` | Yes: `read_msghdr_dest`, `read_mmsghdr_dests` must use `?` throughout |
| `// SAFETY:` docs on all `unsafe` blocks | Yes: `read_msghdr_dest` uses safe Rust only (no unsafe needed) |
| Cross-target clippy MUST/NEVER rule | Yes: all modified files are `#[cfg(target_os = "linux")]`-gated; PARTIAL→CI expected |
| DCO `Signed-off-by` on every commit | Yes: cherry-pick `-x` + DCO required per D-04 |
| `#[must_use]` on critical Results | Yes: `read_msghdr_dest`, `read_mmsghdr_dests` return `Result`; apply `#[must_use]` |
| Fail secure on any unsupported shape | Yes: any unexpected syscall in `handle_network_notification` → `deny_notif` |
| Path security — use `Path::starts_with`, not string operations | N/A for this phase (sockaddr paths compared via `UnixSocketCapability::covers`) |
| `#[allow(dead_code)]` forbidden | Yes: `UnixSocketOp::Send` must be used in tests immediately; don't leave unused |
| Tests must save/restore env vars if modified | N/A — SEC-01 tests use `tempfile::tempdir`; SEC-02 test doesn't touch env vars |
| Commits: DCO sign-off | Yes — every commit must have `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Fork's `deduplicate()` will reproduce the SEC-02 bug (D-09 outcome = test will fail) | SEC-02 Analysis | Low risk: code read confirms both guard conditions absent. Test will show this conclusively before any code change |
| A2 | `struct msghdr` layout (msg_name at offset 0, msg_namelen at offset 8) is stable on x86_64 Linux | SEC-01 `read_msghdr_dest` | Known stable ABI; documented in `man 2 sendmsg`. Very low risk |
| A3 | `sizeof(struct mmsghdr) == 28` on x86_64 Linux | SEC-01 `read_mmsghdr_dests` | Computed via `std::mem::size_of::<libc::mmsghdr>()` at runtime — no hardcoding needed, assumption is moot |

---

## Open Questions (RESOLVED)

1. **Where exactly does the no-grant static-deny (D-01 no-unix-socket-grant path) get installed?** (RESOLVED)

   **Resolution:** `build_seccomp_block_network_filter` (BlockAll) does NOT cover this case. It traps only `SYS_SOCKET`, `SYS_SOCKETPAIR`, and `SYS_IO_URING_SETUP` — not `sendto`/`sendmsg`/`sendmmsg`. In `exec_strategy.rs` (lines ~1226-1248), `install_seccomp_af_unix_filter()` is called only when `config.af_unix_mediation.is_pathname()` is true (grant-present path). In the no-grant case, `install_network_notify` evaluates to false and no filter is installed for send-family syscalls — they fall through to `SECCOMP_RET_ALLOW`. The bypass is confirmed open.

   **Fix (implemented in Plan 87-01 Task 0):** Add `build_seccomp_af_unix_nogrant_filter()` — a new 6-instruction filter that bakes a static `SECCOMP_RET_ERRNO(EPERM)` for `SYS_SENDTO`/`SYS_SENDMSG`/`SYS_SENDMMSG` (ld + 3 JEQ + ALLOW + EPERM). Install it via `install_seccomp_af_unix_nogrant_filter()` (returns `Result<()>`, no notify fd) from `exec_strategy.rs` when mediation is active but no pathname grants exist. This is the D-01 no-grant static-EPERM path: fail-secure, deterministic, no USER_NOTIF involvement.

   **Acceptance criterion:** Code inspection of `exec_strategy.rs` confirms the install condition; the install function is a peer of `install_seccomp_block_network()`. Runtime proof requires Linux.

2. **Does the fork have `LinuxNetworkNotifyMode::AfUnixOnly` that needs updating?** (RESOLVED)

   **Resolution:** `LinuxNetworkNotifyMode::AfUnixOnly` exists in the fork at `supervisor_linux.rs` line 610. The existing guard in `decide_network_notification` (lines ~608-617) checks for `AfUnixOnly` AFTER the AF_UNIX pathname routing block — meaning once `unix_socket_op_for_syscall` returns `Some(UnixSocketOp::Send)` for the new `SYS_SENDTO|SENDMSG|SENDMMSG` syscalls, those calls automatically flow through the existing `AfUnixOnly` guard logic without additional code changes. Task 3 Step 7 in Plan 87-01 confirms this by reading the guard and adding a commit message note. No additional dispatch arms required.

---

## Environment Availability

Step 2.6: SKIPPED for external tools (all changes are pure Rust code edits, no external process dependencies). Note for verification:

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo clippy --target x86_64-unknown-linux-gnu` | Cross-target clippy (D-08) | PARTIAL — rustup std available but C cross-compiler absent | — | PARTIAL→CI per cross-target-verify-checklist.md |
| Linux kernel with seccomp | SEC-01 runtime test execution (D-07) | Not available on Windows dev-host | — | PARTIAL→CI (GH Actions Linux lane) |

---

## Sources

### Primary (HIGH confidence)
- `e2086877` diff — recovered via `git show e2086877` against the git history; complete diff read for `linux.rs`, `capability.rs`, `supervisor_linux.rs`
- `6b3eb013` diff — recovered via `git show 6b3eb013`; complete diff read for `capability.rs`
- Fork source read: `crates/nono/src/sandbox/linux.rs` (lines 820-2450), `crates/nono/src/capability.rs` (lines 1480-1680, 1804-1860), `crates/nono/src/audit.rs` (lines 1247-1410), `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (lines 120-360)
- `87-CONTEXT.md` — locked decisions D-01..D-13
- `86-VERIFICATION.md` and `86-REVIEW.md` — CR-02 upstream-inherited finding (lines 109, 255-278)
- `85-DIVERGENCE-LEDGER.md` §Cluster C — Cluster C disposition and per-commit table (lines 225-254)

### Secondary (MEDIUM confidence)
- Linux `man 2 sendmsg` / `man 2 sendmmsg` — struct msghdr layout (offset 0: msg_name, offset 8: msg_namelen) [ASSUMED from training; layout is stable ABI]
- `cross-target-verify-checklist.md` — PARTIAL→CI procedure

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; all changes are in already-present files
- SEC-01 architecture: HIGH — full upstream diff recovered from git history; fork state confirmed by grep
- SEC-02 architecture: HIGH — full upstream diff recovered; fork vulnerability confirmed by code read
- CR-02 architecture: HIGH — fork code read directly, bypass confirmed at line 1406
- Pitfalls: HIGH — derived from direct BPF instruction analysis and seccomp semantics
- PARTIAL→CI boundary: HIGH — established by cross-target-verify-checklist.md

**Research date:** 2026-06-20
**Valid until:** Upstream-stable (these are security fixes; no staleness risk within 90 days)
