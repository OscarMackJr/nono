# Phase 87: Security Sync - Pattern Map

**Mapped:** 2026-06-20
**Files analyzed:** 4 (all modifications to existing files)
**Analogs found:** 4 / 4 (all in-file analogs — every change extends existing patterns in the same file)

---

## File Classification

| Modified File | Role | Data Flow | Closest Analog (in-file) | Match Quality |
|---------------|------|-----------|--------------------------|---------------|
| `crates/nono/src/sandbox/linux.rs` | utility / security-primitive | event-driven (seccomp BPF) | `build_seccomp_proxy_filter` / `build_seccomp_af_unix_filter` (same file) | exact |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | middleware / supervisor | event-driven (USER_NOTIF dispatch) | `handle_network_notification` / `decide_network_notification` (same file) | exact |
| `crates/nono/src/capability.rs` | model / transform | transform (dedup, remap) | `deduplicate()` / `rewrite_procfs_self_reference` (same file) | exact |
| `crates/nono/src/audit.rs` | service / integrity | batch (log verification) | `verify_audit_log` — fix is inside this function | exact |

---

## Pattern Assignments

---

### `crates/nono/src/sandbox/linux.rs` — SEC-01: BPF filter extension + helpers

**What changes:** `build_seccomp_af_unix_filter` grows from 5 to 8 instructions; `build_seccomp_proxy_filter` grows from 19 to 23 instructions (sendto/sendmsg/sendmmsg JEQ arms inserted before the IO_URING arm). Three new `pub const SYS_SEND*` constants added. Two new helpers `read_msghdr_dest` and `read_mmsghdr_dests` added.

---

#### Imports pattern (lines 1–13)

```rust
use crate::capability::{AccessMode, CapabilitySet, IpcMode, NetworkMode, SignalMode};
use crate::error::{NonoError, Result};
use crate::sandbox::SupportInfo;
use landlock::{...};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::{debug, info, warn};
```

No new imports are needed for SEC-01. `read_msghdr_dest` uses `std::io::Read` inline (same pattern as `read_notif_sockaddr` which imports it locally with `use std::io::Read;` inside the function body).

---

#### `pub const SYS_*` pattern (lines 1119–1123)

Copy the exact pattern used for `SYS_CONNECT` / `SYS_BIND`:

```rust
// Syscall numbers for connect/bind (public for CLI supervisor handler)
#[cfg(target_os = "linux")]
pub const SYS_CONNECT: i32 = libc::SYS_connect as i32;
#[cfg(target_os = "linux")]
pub const SYS_BIND: i32 = libc::SYS_bind as i32;
```

Add immediately after `SYS_BIND` (line 1123):

```rust
// Syscall numbers for send-family (public for CLI supervisor handler)
#[cfg(target_os = "linux")]
pub const SYS_SENDTO: i32 = libc::SYS_sendto as i32;
#[cfg(target_os = "linux")]
pub const SYS_SENDMSG: i32 = libc::SYS_sendmsg as i32;
#[cfg(target_os = "linux")]
pub const SYS_SENDMMSG: i32 = libc::SYS_sendmmsg as i32;
```

Rules: same `#[cfg(target_os = "linux")]` guard, same `libc::SYS_* as i32` cast, same `pub const`, same block comment grouping pattern. These are exported for use in `supervisor_linux.rs` exactly as `SYS_CONNECT`/`SYS_BIND` are today.

---

#### BPF instruction construction pattern (lines 2172–2205, `build_seccomp_af_unix_filter`)

The fork's current 5-instruction filter:

```rust
/// Instruction layout:
/// ```text
///  0: ld  [nr]
///  1: jeq SYS_CONNECT jt=+2 (-> 4: notify)
///  2: jeq SYS_BIND    jt=+1 (-> 4: notify)
///  3: ret ALLOW
///  4: ret USER_NOTIF
/// ```
fn build_seccomp_af_unix_filter() -> Vec<SockFilterInsn> {
    vec![
        SockFilterInsn {
            code: BPF_LD | BPF_W | BPF_ABS,
            jt: 0,
            jf: 0,
            k: SECCOMP_DATA_NR_OFFSET,
        },
        SockFilterInsn {
            code: BPF_JMP | BPF_JEQ | BPF_K,
            jt: 2,
            jf: 0,
            k: SYS_CONNECT as u32,
        },
        SockFilterInsn {
            code: BPF_JMP | BPF_JEQ | BPF_K,
            jt: 1,
            jf: 0,
            k: SYS_BIND as u32,
        },
        SockFilterInsn {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_ALLOW,
        },
        SockFilterInsn {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_USER_NOTIF,
        },
    ]
}
```

**Must become** (8 instructions — add 3 JEQ arms before the ALLOW ret, update all jt offsets):

```rust
/// Instruction layout (8 instructions, jt = jump offset from next insn):
/// ```text
///  0: ld  [nr]
///  1: jeq SYS_CONNECT   jt=+5 (-> 7: notify)
///  2: jeq SYS_BIND      jt=+4 (-> 7: notify)
///  3: jeq SYS_SENDTO    jt=+3 (-> 7: notify)
///  4: jeq SYS_SENDMSG   jt=+2 (-> 7: notify)
///  5: jeq SYS_SENDMMSG  jt=+1 (-> 7: notify)
///  6: ret ALLOW
///  7: ret USER_NOTIF
/// ```
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

**Key rules from the existing pattern:**
- Each JEQ instruction comment states `-> target_idx (jt = target-current-1)`. This in-code arithmetic is mandatory for auditability (see `build_seccomp_proxy_filter` lines 2001–2019 for the full comment table pattern).
- `jf: 0` on all JEQs (fall-through on no-match).
- `jt: 0, jf: 0` on all RET instructions (unused fields).
- `BPF_RET | BPF_K` for returns, `BPF_JMP | BPF_JEQ | BPF_K` for conditional jumps, `BPF_LD | BPF_W | BPF_ABS` for loads.

---

#### `build_seccomp_proxy_filter` extension (lines 1964–2155)

The proxy filter must grow from 19 to 23 instructions. The three `SYS_SENDTO/SENDMSG/SENDMMSG` JEQ arms are inserted **before** the existing IO_URING arm (currently instruction 5 in the 19-insn filter, will become instruction 8 in the 23-insn filter). Every subsequent `jt` offset that targets an instruction after index 4 must be recounted.

The existing instruction-table comment block (lines 2001–2019) must be updated to match the new layout. Follow the exact same comment format: one line per instruction, `//  N: opcode operand  -> target (if applicable)`. The existing proxy filter shows the complete model; copy its comment style verbatim.

After insertion the proxy filter send-syscall arms target `USER_NOTIF` (the same target as `SYS_CONNECT`, currently instruction 16, which shifts to 20 after adding 4 instructions before it — recount all `jt` values carefully).

---

#### `read_notif_sockaddr` as the analog for `read_msghdr_dest` (lines 2298–2358)

`read_notif_sockaddr` is the established pattern for reading child memory via `/proc/PID/mem`. Copy its exact idiom for `read_msghdr_dest`:

```rust
// Pattern from read_notif_sockaddr (lines 2329–2352):
let mem_path = format!("/proc/{}/mem", pid);
let mut file = std::fs::File::open(&mem_path)
    .map_err(|e| NonoError::SandboxInit(format!("Failed to open {}: {}", mem_path, e)))?;

std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(addr_ptr))
    .map_err(|e| NonoError::SandboxInit(format!("Failed to seek in {}: {}", mem_path, e)))?;
```

`read_msghdr_dest` uses `file.read_exact(&mut buf)` instead of `file.read(&mut buf[..read_len])` because the msghdr layout is fixed and we need all 12 bytes. The error message format follows the same template: `"Failed to {verb} {thing} from {mem_path}: {e}"`.

The `#[must_use]` attribute applies (CLAUDE.md: "#[must_use] on functions returning critical Results"):

```rust
/// Read `msg_name` pointer and `msg_namelen` from a child's `struct msghdr`
/// via `/proc/PID/mem`.
///
/// Returns `None` if `msg_name` is NULL (connected socket, no per-message
/// destination). Returns the raw pointer value and length otherwise.
///
/// # Errors
///
/// Returns an error if `/proc/{pid}/mem` cannot be read at `msghdr_ptr`.
#[must_use = "caller must check None (fast-path allow) vs Some (validate sockaddr)"]
pub fn read_msghdr_dest(pid: u32, msghdr_ptr: u64) -> Result<Option<(u64, u64)>> {
    use std::io::Read;
    const MSGHDR_MIN_READ: usize = 12; // msg_name (8 bytes) + msg_namelen (4 bytes)
    let mem_path = format!("/proc/{}/mem", pid);
    let mut file = std::fs::File::open(&mem_path)
        .map_err(|e| NonoError::SandboxInit(format!("Failed to open {}: {}", mem_path, e)))?;
    std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(msghdr_ptr))
        .map_err(|e| NonoError::SandboxInit(format!("Failed to seek in {}: {}", mem_path, e)))?;
    let mut buf = [0u8; MSGHDR_MIN_READ];
    file.read_exact(&mut buf).map_err(|e| {
        NonoError::SandboxInit(format!("Failed to read msghdr from {}: {}", mem_path, e))
    })?;
    let msg_name = u64::from_ne_bytes(buf[0..8].try_into().map_err(|_| {
        NonoError::SandboxInit("msghdr buf slice error".to_string())
    })?);
    let msg_namelen =
        u32::from_ne_bytes(buf[8..12].try_into().map_err(|_| {
            NonoError::SandboxInit("msghdr namelen slice error".to_string())
        })?) as u64;
    if msg_name == 0 {
        Ok(None)
    } else {
        Ok(Some((msg_name, msg_namelen)))
    }
}
```

Note: use `try_into()` for the byte slice conversions (no `.unwrap()` — enforced by `clippy::unwrap_used`).

---

#### `read_mmsghdr_dests` — overflow-safe vector walking

Use `checked_mul` / `checked_add` (CLAUDE.md arithmetic rule). Cap at `MAX_MMSGHDRS = 1024`. Use `std::mem::size_of::<libc::mmsghdr>()` for stride (no hardcoded 28). Return `Vec<Option<(u64, u64)>>`.

```rust
/// Maximum `mmsghdr` entries processed per `sendmmsg` notification.
const MAX_MMSGHDRS: u32 = 1024;

/// Read destination addresses from a child's `mmsghdr` vector via `/proc/PID/mem`.
///
/// Each entry in the returned `Vec` corresponds to one `mmsghdr` entry.
/// `None` means the entry has a NULL `msg_name` (connected socket fast path).
///
/// # Errors
///
/// Returns an error if `/proc/{pid}/mem` cannot be read.
#[must_use]
pub fn read_mmsghdr_dests(pid: u32, msgvec_ptr: u64, vlen: u32) -> Result<Vec<Option<(u64, u64)>>> {
    let stride = std::mem::size_of::<libc::mmsghdr>() as u64;
    let count = vlen.min(MAX_MMSGHDRS) as u64;
    let mut results = Vec::with_capacity(count as usize);
    for idx in 0..count {
        let offset = idx
            .checked_mul(stride)
            .and_then(|o| o.checked_add(msgvec_ptr))
            .ok_or_else(|| {
                NonoError::SandboxInit(format!(
                    "mmsghdr offset overflow at index {} stride {}",
                    idx, stride
                ))
            })?;
        results.push(read_msghdr_dest(pid, offset)?);
    }
    Ok(results)
}
```

---

#### BPF filter test pattern (lines 3519–3539)

The existing test `test_build_seccomp_af_unix_filter_notifies_connect_bind_only` is the model. After the port it must be updated to assert 8 instructions and check the new syscall constants at the correct indices:

```rust
// EXISTING test (lines 3530–3538) — must be updated:
#[test]
fn test_build_seccomp_af_unix_filter_notifies_connect_bind_only() {
    let filter = build_seccomp_af_unix_filter();
    assert_eq!(filter.len(), 5);                         // → 8 after port
    assert_eq!(filter[0].code, BPF_LD | BPF_W | BPF_ABS);
    assert_eq!(filter[0].k, SECCOMP_DATA_NR_OFFSET);
    assert_eq!(filter[1].k, SYS_CONNECT as u32);
    assert_eq!(filter[2].k, SYS_BIND as u32);
    assert_eq!(filter[3].k, SECCOMP_RET_ALLOW);
    assert_eq!(filter[4].k, SECCOMP_RET_USER_NOTIF);
}

// After port (8 instructions):
#[test]
fn test_build_seccomp_af_unix_filter_notifies_connect_bind_send() {
    let filter = build_seccomp_af_unix_filter();
    assert_eq!(filter.len(), 8);
    assert_eq!(filter[0].code, BPF_LD | BPF_W | BPF_ABS);
    assert_eq!(filter[0].k, SECCOMP_DATA_NR_OFFSET);
    assert_eq!(filter[1].k, SYS_CONNECT as u32);
    assert_eq!(filter[2].k, SYS_BIND as u32);
    assert_eq!(filter[3].k, SYS_SENDTO as u32);
    assert_eq!(filter[4].k, SYS_SENDMSG as u32);
    assert_eq!(filter[5].k, SYS_SENDMMSG as u32);
    assert_eq!(filter[6].k, SECCOMP_RET_ALLOW);
    assert_eq!(filter[7].k, SECCOMP_RET_USER_NOTIF);
    // Verify jt offsets for correctness
    assert_eq!(filter[1].jt, 5); // -> insn 7
    assert_eq!(filter[2].jt, 4); // -> insn 7
    assert_eq!(filter[3].jt, 3); // -> insn 7
    assert_eq!(filter[4].jt, 2); // -> insn 7
    assert_eq!(filter[5].jt, 1); // -> insn 7
}
```

Similarly update the proxy filter count assertion (line 3521: `assert_eq!(filter.len(), 19)` → `assert_eq!(filter.len(), 23)`).

---

### `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — SEC-01: send-syscall dispatch

**What changes:** `unix_socket_op_for_syscall` gains a `Send` arm; `decide_network_notification` gains `SYS_SENDTO | SYS_SENDMSG | SYS_SENDMMSG` in the match; `handle_network_notification` gains a multi-sockaddr loop for the send family; `ipc_denial_details` and `record_af_unix_ipc_denial` gain `UnixSocketOp::Send` handling.

---

#### Imports to extend (line 14 in supervisor_linux.rs)

Current:
```rust
use nono::{try_canonicalize, AccessMode, NonoRemediation, UnixSocketCapability, UnixSocketOp};
```

Add the new sandbox constants to the use sites inside functions — same pattern as existing:

```rust
// Inside unix_socket_op_for_syscall (line 747):
use nono::sandbox::{SYS_BIND, SYS_CONNECT};
// Add:
use nono::sandbox::{SYS_BIND, SYS_CONNECT, SYS_SENDMSG, SYS_SENDMMSG, SYS_SENDTO};
```

Add `read_msghdr_dest, read_mmsghdr_dests` to the use block inside `handle_network_notification` (line 820):

```rust
// Existing (lines 819–822):
use nono::sandbox::{
    continue_notif, deny_notif, notif_id_valid, read_notif_sockaddr, recv_notif,
    respond_notif_errno,
};
// After port:
use nono::sandbox::{
    continue_notif, deny_notif, notif_id_valid, read_msghdr_dest, read_mmsghdr_dests,
    read_notif_sockaddr, recv_notif, respond_notif_errno,
};
```

---

#### `UnixSocketOp` enum extension — exhaustiveness

`UnixSocketOp` in `capability.rs` gains a `Send` variant. Every `match op` in `supervisor_linux.rs` must be exhausted. Known match sites (confirmed by grep):

1. `decide_af_unix_pathname` (line 706): `match op { Connect => ..., Bind => ... }` — add `Send` arm
2. `ipc_denial_details` (line 939): `match op { Connect => ..., Bind => ... }` — add `Send` arm
3. `record_af_unix_ipc_denial` (line 897): `match op { Connect => ..., Bind => ... }` — add `Send` arm
4. `unix_socket_op_for_syscall` (lines 749–752): add `SYS_SENDTO | SYS_SENDMSG | SYS_SENDMMSG => Some(UnixSocketOp::Send)`
5. `unix_socket_allowlist_allows` (line 756): if it has a `match op` — add `Send` arm

The `Display` impl in `capability.rs` for `UnixSocketOp` must also gain a `Send` arm:

```rust
// Existing pattern (lines 226–232):
impl std::fmt::Display for UnixSocketOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnixSocketOp::Connect => write!(f, "connect"),
            UnixSocketOp::Bind => write!(f, "bind"),
        }
    }
}
// After port (add Send arm):
            UnixSocketOp::Send => write!(f, "send"),
```

---

#### `unix_socket_op_for_syscall` pattern (lines 746–754)

The existing function:

```rust
fn unix_socket_op_for_syscall(syscall: i32) -> Option<UnixSocketOp> {
    use nono::sandbox::{SYS_BIND, SYS_CONNECT};

    match syscall {
        SYS_CONNECT => Some(UnixSocketOp::Connect),
        SYS_BIND => Some(UnixSocketOp::Bind),
        _ => None,
    }
}
```

After port:

```rust
fn unix_socket_op_for_syscall(syscall: i32) -> Option<UnixSocketOp> {
    use nono::sandbox::{SYS_BIND, SYS_CONNECT, SYS_SENDMSG, SYS_SENDMMSG, SYS_SENDTO};

    match syscall {
        SYS_CONNECT => Some(UnixSocketOp::Connect),
        SYS_BIND => Some(UnixSocketOp::Bind),
        SYS_SENDTO | SYS_SENDMSG | SYS_SENDMMSG => Some(UnixSocketOp::Send),
        _ => None,
    }
}
```

---

#### `handle_network_notification` multi-sockaddr loop pattern (lines 812–868)

The current single-sockaddr flow is:

```rust
// Read sockaddr from child's memory: args[1] = sockaddr*, args[2] = addrlen
let sockaddr = match read_notif_sockaddr(notif.pid, notif.data.args[1], notif.data.args[2]) {
    Ok(info) => info,
    Err(e) => { debug!(...); let _ = deny_notif(notify_fd, notif.id); return Ok(()); }
};
// TOCTOU check
if !notif_id_valid(notify_fd, notif.id)? { ... return Ok(()); }
match decide_network_notification(..., &sockaddr, config) {
    Allow => { continue_notif(...) }
    Deny  => { record_af_unix_ipc_denial(...); respond_notif_errno(..., EACCES)?; ... }
}
```

After port, the dispatch branches on `notif.data.nr` to build a `Vec<SockaddrInfo>`:

```rust
use nono::sandbox::{SYS_SENDMSG, SYS_SENDMMSG, SYS_SENDTO};

let sockaddrs: Vec<nono::sandbox::SockaddrInfo> = match notif.data.nr {
    SYS_SENDTO => {
        // args[4] = dest_addr*, args[5] = addrlen (0 means connected socket)
        if notif.data.args[4] == 0 {
            // Connected socket — no per-call destination; fast-path allow.
            if let Err(e) = continue_notif(notify_fd, notif.id) {
                debug!("continue_notif failed for sendto (connected socket): {}", e);
                return deny_notif(notify_fd, notif.id);
            }
            return Ok(());
        }
        match read_notif_sockaddr(notif.pid, notif.data.args[4], notif.data.args[5]) {
            Ok(info) => vec![info],
            Err(e) => {
                debug!("Failed to read sendto sockaddr: {}", e);
                let _ = deny_notif(notify_fd, notif.id);
                return Ok(());
            }
        }
    }
    SYS_SENDMSG => {
        match read_msghdr_dest(notif.pid, notif.data.args[1]) {
            Ok(None) => {
                // NULL msg_name = connected socket, fast-path allow.
                if let Err(e) = continue_notif(notify_fd, notif.id) {
                    debug!("continue_notif failed for sendmsg (connected socket): {}", e);
                    return deny_notif(notify_fd, notif.id);
                }
                return Ok(());
            }
            Ok(Some((addr_ptr, addrlen))) => {
                match read_notif_sockaddr(notif.pid, addr_ptr, addrlen) {
                    Ok(info) => vec![info],
                    Err(e) => {
                        debug!("Failed to read sendmsg msg_name sockaddr: {}", e);
                        let _ = deny_notif(notify_fd, notif.id);
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                debug!("Failed to read msghdr from sendmsg: {}", e);
                let _ = deny_notif(notify_fd, notif.id);
                return Ok(());
            }
        }
    }
    SYS_SENDMMSG => {
        match read_mmsghdr_dests(notif.pid, notif.data.args[1], notif.data.args[2] as u32) {
            Ok(entries) => {
                let addrs: Vec<_> = entries
                    .into_iter()
                    .flatten()
                    .filter_map(|(addr_ptr, addrlen)| {
                        read_notif_sockaddr(notif.pid, addr_ptr, addrlen).ok()
                    })
                    .collect();
                if addrs.is_empty() {
                    // All entries had NULL msg_name = connected sockets.
                    if let Err(e) = continue_notif(notify_fd, notif.id) {
                        debug!("continue_notif failed for sendmmsg (all connected): {}", e);
                        return deny_notif(notify_fd, notif.id);
                    }
                    return Ok(());
                }
                addrs
            }
            Err(e) => {
                debug!("Failed to read mmsghdr vector: {}", e);
                let _ = deny_notif(notify_fd, notif.id);
                return Ok(());
            }
        }
    }
    // connect / bind — existing single-sockaddr path:
    _ => {
        match read_notif_sockaddr(notif.pid, notif.data.args[1], notif.data.args[2]) {
            Ok(info) => vec![info],
            Err(e) => {
                debug!("Failed to read sockaddr from seccomp notification: {}", e);
                let _ = deny_notif(notify_fd, notif.id);
                return Ok(());
            }
        }
    }
};
```

Then the TOCTOU check and the decision loop:

```rust
if !notif_id_valid(notify_fd, notif.id)? { ... return Ok(()); }

// For the multi-sockaddr case: deny on ANY denial (fail-secure).
for sockaddr in &sockaddrs {
    match decide_network_notification(notif.pid, notif.data.nr, sockaddr, config) {
        NetworkDecision::Allow => {} // continue to next entry
        NetworkDecision::Deny => {
            record_af_unix_ipc_denial(sockaddr, notif.pid, notif.data.nr, denials, ipc_denials);
            respond_notif_errno(notify_fd, notif.id, libc::EACCES)?;
            if let Err(err) = record_network_audit_denial(config, sockaddr, notif.data.nr) {
                warn!("Failed to record network denial audit event: {}", err);
            }
            return Ok(());
        }
    }
}
// All sockaddrs allowed:
if let Err(e) = continue_notif(notify_fd, notif.id) {
    debug!("continue_notif failed for network notification: {}", e);
    return deny_notif(notify_fd, notif.id);
}
```

**Styling rules from existing code:**
- `debug!` on all deny paths inside helper failures (same as lines 837, 844).
- `let _ = deny_notif(...)` to suppress the result when we're already in an error path (same as line 829).
- `deny_notif(notify_fd, notif.id)` propagated as `return deny_notif(...)` from the fallback (same as lines 855–856).
- `warn!` on audit record failures, not `error!` (same as line 861).

---

#### `decide_network_notification` extension (lines 574–656)

Add `SYS_SENDTO | SYS_SENDMSG | SYS_SENDMMSG` to the `SYS_CONNECT` arm in the final `match syscall`:

```rust
// Existing (lines 619–635):
match syscall {
    SYS_CONNECT => {
        if sockaddr.is_loopback && sockaddr.port == config.proxy_port { ... Allow }
        else { ... Deny }
    }
    SYS_BIND => { ... }
    other => { warn!("Unexpected syscall {} ...", other); NetworkDecision::Deny }
}

// After port:
match syscall {
    SYS_CONNECT | SYS_SENDTO | SYS_SENDMSG | SYS_SENDMMSG => {
        // Send-family: same loopback+proxy_port check as connect for TCP;
        // AF_UNIX pathname routing is already handled above by decide_af_unix_pathname.
        if sockaddr.is_loopback && sockaddr.port == config.proxy_port { ... Allow }
        else { ... Deny }
    }
    SYS_BIND => { ... }
    other => { warn!("Unexpected syscall {} ...", other); NetworkDecision::Deny }
}
```

And the `use` block at line 580 adds the new constants:

```rust
use nono::sandbox::{UnixSocketKind, SYS_BIND, SYS_CONNECT, SYS_SENDMSG, SYS_SENDMMSG, SYS_SENDTO};
```

---

#### Test pattern for new send dispatch (lines 1316–1463)

The existing `mod network_decision` submodule with its `make_config`, `unix_pathname`, `unix_abstract`, `unix_unnamed`, `inet_loopback`, `inet_external` helpers is the complete pattern. New tests for `SYS_SENDTO`/`SYS_SENDMSG`/`SYS_SENDMMSG` are added into this same `mod network_decision` block:

```rust
// Follow this exact structure:
#[cfg(target_os = "linux")]
#[test]
fn af_unix_pathname_sendto_is_allowed_by_grant() {
    let backend = DenyAllBackend;
    let dir = tempfile::tempdir().expect("tempdir");
    let path = socket_path(&dir, "test.sock");
    // create the socket file so canonicalize succeeds:
    UnixListener::bind(&path).expect("bind");
    let allowlist = vec![
        UnixSocketCapability::new_file(&path, UnixSocketMode::ConnectBind).expect("socket grant"),
    ];
    let config = make_config(&backend, 0, Vec::new(), &allowlist);
    assert_eq!(
        decide_network_notification(test_pid(), SYS_SENDTO, &unix_pathname(&path), &config),
        NetworkDecision::Allow,
        "pathname AF_UNIX sendto must be allowed when a send-capable grant covers it"
    );
}

#[test]
fn af_unix_abstract_sendto_is_denied() {
    let backend = DenyAllBackend;
    let config = make_config(&backend, 0, Vec::new(), &[]);
    assert_eq!(
        decide_network_notification(test_pid(), SYS_SENDTO, &unix_abstract(), &config),
        NetworkDecision::Deny,
    );
}
```

The fork-specific test for the grant-present `sockaddr_un` destination validation (D-06 requirement) must be placed here. This is the one test upstream has no equivalent for.

---

### `crates/nono/src/capability.rs` — SEC-02: procfs-remap dedup guard

**What changes:** `UnixSocketOp` gains `Send` variant; `is_procfs_remap_original` private helper added near `rewrite_procfs_self_reference` (line 1804); both `original_updates` push sites guarded.

---

#### `UnixSocketOp` enum (lines 218–233)

The existing enum with two variants is the exact model:

```rust
// Existing:
pub enum UnixSocketOp {
    Connect,
    Bind,
}

// After port (add Send):
pub enum UnixSocketOp {
    /// About to call `connect(2)`.
    Connect,
    /// About to call `bind(2)`.
    Bind,
    /// About to call `sendto(2)`, `sendmsg(2)`, or `sendmmsg(2)`.
    Send,
}
```

The `Display` impl directly below (lines 226–232) must add a `Send => write!(f, "send")` arm. Doc comment on `Send` should follow the style of `Connect` and `Bind` (same one-sentence format).

---

#### `is_procfs_remap_original` helper (after line 1859)

Place immediately after `rewrite_procfs_self_reference` closes (line 1859), before the `#[cfg(test)]` block (line 1861). This mirrors upstream exactly:

```rust
/// Returns `true` if `path` is one of the `/dev/stdin`, `/dev/stdout`,
/// `/dev/stderr`, or `/dev/fd` aliases that `rewrite_procfs_self_reference`
/// maps to procfs entries.
///
/// Used by `deduplicate()` to prevent these aliases from being inherited as
/// the `original` field of a surviving direct entry, which would cause
/// `remap_procfs_self_references` to rewrite the resolved path incorrectly.
#[cfg(target_os = "linux")]
fn is_procfs_remap_original(path: &Path) -> bool {
    rewrite_procfs_self_reference(path, 0, None).is_some()
}
```

Rules: `#[cfg(target_os = "linux")]` only (not Windows-gated). `fn` not `pub fn` (private helper). Must NOT be `#[allow(dead_code)]` — the guard in `deduplicate()` and the regression test both use it immediately.

---

#### `deduplicate()` guard insertion (lines 1581–1606)

The existing two push sites that need the guard:

**Site 1** — `keep_new = true` branch (lines 1586–1589):

```rust
// Existing:
#[cfg(target_os = "linux")]
if cap.original == cap.resolved && existing.original != existing.resolved {
    original_updates.push((i, existing.original.clone()));
}

// After port (add the guard as the third condition):
#[cfg(target_os = "linux")]
if cap.original == cap.resolved
    && existing.original != existing.resolved
    && !is_procfs_remap_original(&existing.original)
{
    original_updates.push((i, existing.original.clone()));
}
```

**Site 2** — `keep_new = false` branch (lines 1597–1600):

```rust
// Existing:
#[cfg(target_os = "linux")]
if existing.original == existing.resolved && cap.original != cap.resolved {
    original_updates.push((existing_idx, cap.original.clone()));
}

// After port (add the guard as the third condition):
#[cfg(target_os = "linux")]
if existing.original == existing.resolved
    && cap.original != cap.resolved
    && !is_procfs_remap_original(&cap.original)
{
    original_updates.push((existing_idx, cap.original.clone()));
}
```

No structural change beyond the `&& !is_procfs_remap_original(...)` guard. The `#[cfg(target_os = "linux")]` wrapping is kept as-is. The `access_upgrades` logic is orthogonal and unchanged.

---

#### SEC-02 regression test placement (after line 2459 in `procfs_remap_tests` module or the dedup test module)

The regression test goes into the `mod procfs_remap_tests` block (line 1862) because it tests the interaction of `deduplicate()` with `remap_procfs_self_references`. The existing tests in that module (`remap_procfs_self_rewrites_proc_self_capability`, `remap_procfs_self_rewrites_dev_fd_aliases`) are the structural model:

```rust
// Existing test structure to copy (lines 1865–1888):
#[test]
fn remap_procfs_self_rewrites_proc_self_capability() {
    let mut caps = CapabilitySet::new();
    caps.add_fs(FsCapability {
        original: PathBuf::from("/proc/self"),
        resolved: PathBuf::from("/proc/111/self-was-parent"),
        access: AccessMode::Read,
        is_file: false,
        source: CapabilitySource::Group("system_read_linux".to_string()),
    });
    caps.remap_procfs_self_references(4242, None);
    assert_eq!(caps.fs_capabilities()[0].original, ...);
}

// SEC-02 regression test — same module, same structure:
#[cfg(target_os = "linux")]
#[test]
fn remap_preserves_dev_null_when_deduped_with_dev_stdin() {
    use crate::capability::{CapabilitySource, FsCapability};
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
        "resolved must remain /dev/null after remap; /dev/stdin alias must not be inherited"
    );
}
```

---

### `crates/nono/src/audit.rs` — CR-02: `records_verified` semantic fix

**What changes:** Line 1406 `records_verified: true` becomes `records_verified: event_count > 0`.

---

#### `verify_audit_log` return struct pattern (lines 1396–1409)

The existing return block is the complete context:

```rust
// Existing (lines 1396–1409):
Ok(AuditVerificationResult {
    hash_algorithm: AUDIT_HASH_ALGORITHM.to_string(),
    merkle_scheme: MERKLE_SCHEME_ALPHA.to_string(),
    event_count,
    computed_chain_head,
    computed_merkle_root,
    stored_event_count,
    stored_chain_head,
    stored_merkle_root,
    event_count_matches,
    records_verified: true,       // <-- the bug: line 1406
    chain_head_matches,
    merkle_root_matches,
})

// After fix:
    records_verified: event_count > 0,  // deliberate fork-divergence from upstream e9529312
```

The comment on the fix line is mandatory (same pattern as `chain_head_matches`/`merkle_root_matches` which carry the comment `// Fork-extension: ...` at line 1388). Use:

```rust
// Fork-hardening (CR-02): set records_verified to false when no records were
// processed. Upstream hardcodes true (e9529312). Deliberate divergence;
// see proj/ADR-87-cr02-audit-bypass.md and 85-DIVERGENCE-LEDGER.md.
records_verified: event_count > 0,
```

---

#### `AuditVerificationResult` doc comment update (lines 152–153)

```rust
// Existing:
/// True when all record-level checks passed.
pub records_verified: bool,

// After fix:
/// True when at least one record was processed and all record-level checks
/// passed. False for an empty log, which means no integrity claim was made.
/// (Fork divergence from upstream — see ADR-87-cr02-audit-bypass.md.)
pub records_verified: bool,
```

---

#### `is_valid()` doc comment update (lines 163–171)

Add a note that `is_valid()` returns false for an empty log with no stored metadata:

```rust
/// Returns true when all integrity checks passed: record-level verification,
/// event count, chain head, and Merkle root all match.
///
/// Returns false for an empty log with `stored: None` (no verification was
/// performed, so `records_verified` is false). This is intentional:
/// callers must not mistake "nothing checked" for "everything passed."
#[must_use]
pub fn is_valid(&self) -> bool { ... }
```

---

#### CR-02 test (in `mod tests` at line 1412)

The existing `recorder_produces_integrity_summary` test (line 1423) uses `tempfile::tempdir()` + `AuditRecorder` — same pattern for the CR-02 test:

```rust
#[test]
fn verify_empty_log_with_no_stored_metadata_is_not_valid() {
    let dir = tempfile::tempdir().unwrap();
    // Write an empty audit events file (no records):
    let events_path = dir.path().join(AUDIT_EVENTS_FILENAME);
    std::fs::write(&events_path, "").unwrap();
    let result = verify_audit_log(dir.path(), None).unwrap();
    assert_eq!(result.event_count, 0);
    assert!(!result.records_verified, "empty log must have records_verified = false");
    assert!(
        !result.is_valid(),
        "is_valid() must return false when no records were verified"
    );
}
```

This test documents the semantic contract and will serve as a regression guard against future reversion to `records_verified: true`.

---

## Shared Patterns

### Error Handling
**Source:** All files — `NonoError::SandboxInit(format!(...))` pattern
**Apply to:** `read_msghdr_dest`, `read_mmsghdr_dests`, any new function in `linux.rs`

```rust
// Pattern (from read_notif_sockaddr, lines 2330–2334):
.map_err(|e| NonoError::SandboxInit(format!("Failed to open {}: {}", mem_path, e)))?;
```

Always use `NonoError::SandboxInit` for seccomp/sandbox primitive failures in `linux.rs`. Use `NonoError::Snapshot` for audit failures in `audit.rs` (see lines 1254–1257).

### No-unwrap / Result propagation
**Source:** CLAUDE.md §Coding Standards; enforced by `clippy::unwrap_used`
**Apply to:** All new code in all four files

Every `.unwrap()` in new code will fail CI. Use `?`, `.map_err(...)`, `ok_or_else(...)`, or `match`. The only exception is inside `#[cfg(test)]` / `#[allow(clippy::unwrap_used)]` blocks (see `mod tests` in `audit.rs` line 1413).

### `#[must_use]` on `Result`-returning public functions
**Source:** CLAUDE.md §Coding Standards; examples at lines 1144, 1164 of `linux.rs`
**Apply to:** `read_msghdr_dest`, `read_mmsghdr_dests`

### `#[cfg(target_os = "linux")]` gating
**Source:** Lines 1112–1123, 1523–1524, 1586–1600 of `capability.rs`; entire `linux.rs`
**Apply to:** `is_procfs_remap_original`, both guard sites, `SYS_SEND*` constants, all new BPF code

All new code that is Linux-specific must be wrapped with `#[cfg(target_os = "linux")]`. The cross-target clippy run (PARTIAL→CI) is the decisive gate for exhaustiveness.

### Seccomp deny-on-error (fail-secure)
**Source:** `handle_network_notification` lines 836–840 and 828–830

```rust
// Pattern: on any parse/read failure → deny immediately, return Ok(())
// Never leave the child blocked and never propagate unless deny also fails.
Err(e) => {
    debug!("Failed to read ...: {}", e);
    let _ = deny_notif(notify_fd, notif.id);
    return Ok(());
}
```

This is the fail-secure contract. Every new arm in `handle_network_notification` for send-family syscalls must follow this exact pattern on error.

### TOCTOU check before responding
**Source:** Lines 843–847 of `supervisor_linux.rs`

```rust
if !notif_id_valid(notify_fd, notif.id)? {
    debug!("Network seccomp notification expired (TOCTOU check)");
    return Ok(());
}
```

This check must remain between sockaddr extraction and the `decide_network_notification` call. It must NOT be moved inside the per-sockaddr loop for the multi-entry sendmmsg case — perform it once before the loop.

### In-code BPF instruction-index comment table
**Source:** `build_seccomp_proxy_filter` lines 2001–2019

The comment block preceding the `vec![...]` body that enumerates every instruction index and its jump target is the mandatory documentation form. Both `build_seccomp_af_unix_filter` and `build_seccomp_proxy_filter` must have their comment tables updated to reflect the new 8-/23-instruction counts and all recounted `jt` offsets.

### DCO sign-off and cherry-pick `-x`
**Source:** CLAUDE.md §Coding Standards; Phase 86 execution pattern

Every commit: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`. Cherry-picks: `git cherry-pick -x <SHA>` (the `-x` line is mandatory for upstream provenance). CR-02 is a separate fork-hardening commit (not a cherry-pick).

---

## No Analog Found

All four files have strong in-file analogs. No new file types or patterns without precedent.

| Item | Reason |
|------|---------|
| `read_mmsghdr_dests` multi-entry vector walking | No existing multi-entry vector walk in the codebase, but the pattern is a straightforward loop using `read_msghdr_dest` (itself modeled on `read_notif_sockaddr`). RESEARCH.md provides the complete implementation specification. |
| `ADR-87-cr02-audit-bypass.md` (new ADR file) | Prose ADR — no code analog needed. Follow the structure of `proj/ADR-86-library-boundary-convergence.md` for the header + decision + rationale sections. |

---

## Metadata

**Analog search scope:** `crates/nono/src/sandbox/linux.rs`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, `crates/nono/src/capability.rs`, `crates/nono/src/audit.rs`
**Files scanned:** 4 source files + CONTEXT.md + RESEARCH.md
**Pattern extraction date:** 2026-06-20
