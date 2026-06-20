---
phase: 87
plan: "01"
subsystem: linux-seccomp
tags: [security, seccomp, af-unix, sendto, sendmsg, sendmmsg, bpf, cherry-pick]
dependency_graph:
  requires: []
  provides: [SEC-01-AF-UNIX-DATAGRAM-BYPASS-CLOSED]
  affects: [linux.rs, supervisor_linux.rs, capability.rs, exec_strategy.rs, sandbox/mod.rs]
tech_stack:
  added: []
  patterns:
    - "BPF jt offset formula: jt = target_insn_index - current_insn_index - 1"
    - "No-grant static-EPERM filter (D-01 fork-specific hybrid gate)"
    - "Multi-sockaddr Vec dispatch with TOCTOU-once-before-loop"
    - "Connected-socket NULL fast-paths for sendto/sendmsg/sendmmsg"
key_files:
  created: []
  modified:
    - crates/nono/src/capability.rs
    - crates/nono/src/sandbox/linux.rs
    - crates/nono/src/sandbox/mod.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
decisions:
  - "D-01: No-grant hybrid gate — install static EPERM BPF filter when !proxy_fallback && !is_pathname()"
  - "D-04: Atomic cherry-pick commit with (cherry picked from commit e2086877) attribution + DCO"
  - "D-05: UnixSocketOp::Send covered by Connect grant — no new grant type needed"
  - "D-06: Fork-specific tests (af_unix_pathname_sendto_is_allowed_by_grant, abstract_sendto_denied, af_unix_only_mode_allows_non_af_unix_sendto)"
  - "D-07: TOCTOU check (notif_id_valid) called once before per-sockaddr loop, not inside"
  - "D-08: All-NULL sendmmsg fast-path → continue_notif (connected-socket datagram)"
  - "Cross-target clippy PARTIAL: Windows host lacks x86_64-linux-gnu-gcc; deferred to GH Actions CI"
metrics:
  duration: "~90 minutes (continuation session)"
  completed: "2026-06-19"
  tasks_completed: 4
  files_modified: 5
  commits: 1
---

# Phase 87 Plan 01: SEC-01 AF_UNIX Datagram Bypass Fix Summary

One-liner: Cherry-pick e2086877 onto fork — extend seccomp BPF to trap sendto/sendmsg/sendmmsg, closing SOCK_DGRAM AF_UNIX bypass, plus D-01 no-grant static-EPERM filter.

## What Was Built

Port of upstream commit e2086877 ("fix(linux): trap sendto/sendmsg to prevent AF_UNIX datagram bypass") onto the fork's divergent Linux enforcement code, with fork-specific extensions.

### Root Cause Closed

SOCK_DGRAM AF_UNIX sockets holding an existing fd could call `sendto`/`sendmsg`/`sendmmsg` to reach AF_UNIX peers. The seccomp BPF filter only trapped `connect`/`bind` — the send-family fell through to `SECCOMP_RET_ALLOW`, bypassing the supervisor's mediation entirely.

### Changes by File

**`crates/nono/src/capability.rs`**
- Added `UnixSocketOp::Send` variant for sendto/sendmsg/sendmmsg with a destination address
- Display: `Send => "send"`
- `unix_socket_allowed`: `Connect | Send => true` (any connect grant covers send)
- Tests: `test_unix_socket_op_display`, `test_capability_set_unix_socket_send_covered_by_connect_grant`

**`crates/nono/src/sandbox/linux.rs`**
- `build_seccomp_proxy_filter`: 19 → 23 instructions; 3 new JEQ arms (SENDTO/SENDMSG/SENDMMSG at insns 5,6,7) + 1 USER_NOTIF ret for send-family (insn 21); all jt offsets recounted
- `build_seccomp_af_unix_filter`: 5 → 8 instructions; SENDTO/SENDMSG/SENDMMSG JEQ arms (insns 3,4,5); ALLOW shifted to insn 6, USER_NOTIF to insn 7; CONNECT jt 2→5, BIND jt 1→4
- `build_seccomp_af_unix_nogrant_filter` (fork-specific D-01): 6-instruction BPF; ld syscall nr + 3 JEQ (SENDTO jt=3, SENDMSG jt=2, SENDMMSG jt=1) + ALLOW fallthrough + EPERM return; used when no AF_UNIX grant is present
- `install_seccomp_af_unix_nogrant_filter`: pub fn, same pattern as `install_seccomp_block_network` (PR_SET_NO_NEW_PRIVS + SYS_seccomp)
- `read_msghdr_dest`: reads first 12 bytes of `struct msghdr` from `/proc/PID/mem`; returns `None` if `msg_name == 0` (connected socket)
- `read_mmsghdr_dests`: reads `mmsghdr` array capped at `MAX_MMSGHDRS=1024`; `size_of::<libc::mmsghdr>()` stride; `checked_mul`/`checked_add` overflow safety
- SYS_SENDTO/SENDMSG/SENDMMSG constants exported
- Tests updated: proxy filter 19→23, af_unix filter 5→8, new `test_build_seccomp_af_unix_nogrant_filter_denies_send_family`

**`crates/nono/src/sandbox/mod.rs`**
- Re-exports: `install_seccomp_af_unix_nogrant_filter`, `read_msghdr_dest`, `read_mmsghdr_dests`, `SYS_SENDMMSG`, `SYS_SENDMSG`, `SYS_SENDTO`

**`crates/nono-cli/src/exec_strategy.rs`**
- D-01 no-grant filter install: `if !config.seccomp_proxy_fallback && !config.af_unix_mediation.is_pathname()` → `install_seccomp_af_unix_nogrant_filter()`
- Error path uses `libc::write` (async-signal-safe, no `eprintln!`)

**`crates/nono-cli/src/exec_strategy/supervisor_linux.rs`**
- `decide_network_notification`: matches `SYS_SENDTO | SYS_SENDMSG | SYS_SENDMMSG`
- `unix_socket_op_for_syscall`: `SYS_SENDTO | SYS_SENDMSG | SYS_SENDMMSG => Some(UnixSocketOp::Send)`
- `decide_af_unix_pathname`: `Connect | Send` arm for canonicalization
- `unix_socket_allowlist_allows`: `Connect | Send => true`
- `handle_network_notification`: multi-sockaddr `Vec<SockaddrInfo>` dispatch; TOCTOU check once before loop; NULL fast-paths (sendto args[4]==0, sendmsg msg_name==NULL, sendmmsg all-None); deny-on-ANY policy
- `record_af_unix_ipc_denial`: `Connect | Send => AccessMode::Read`
- `ipc_denial_details`: `Connect | Send => resolved.canonicalize()`
- Tests added: `af_unix_pathname_sendto_is_allowed_by_grant`, `af_unix_abstract_sendto_is_denied`, `af_unix_only_mode_allows_non_af_unix_sendto`

## Commit

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 0-3 (atomic) | SEC-01 AF_UNIX datagram bypass — trap sendto/sendmsg/sendmmsg | `6cf2645c` | 5 modified |

All tasks (0, 1, 2, 3) staged and committed atomically per D-04 cherry-pick discipline.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written. The 87-PATTERNS.md file referenced in the plan did not
exist (it was listed in gitStatus as untracked but was missing on disk). Used 87-RESEARCH.md
code examples section instead, which contained all required implementation patterns inline.
This was a documentation lookup deviation only, not a code deviation.

## Cross-Target Clippy Status

**PARTIAL → CI**

Windows dev host lacks `x86_64-linux-gnu-gcc` (C cross-compiler required by `aws-lc-sys`).
`cargo check --workspace` completed clean (native Windows target).
`cargo clippy --workspace --target x86_64-unknown-linux-gnu` failed with:
`error: failed to find tool "x86_64-linux-gnu-gcc": program not found`

Per `.planning/templates/cross-target-verify-checklist.md`:
> Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
> (x86_64-unknown-linux-gnu). The live GH Actions Linux Clippy lane on the head SHA is the
> decisive signal. REQ marked PARTIAL pending CI confirmation.

SEC-01 verification requires GH Actions Linux + macOS clippy lanes to confirm clean on commit `6cf2645c`.

## Known Stubs

None — all send-family dispatch paths are fully wired.

## Threat Flags

None beyond the plan's threat model. This plan closes the existing AF_UNIX SOCK_DGRAM bypass
surface; it does not introduce new network endpoints, auth paths, or trust boundary crossings.

## Self-Check: PASSED

- [x] `6cf2645c` exists in git log
- [x] All 5 modified files present in worktree
- [x] `(cherry picked from commit e2086877)` in commit message
- [x] `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` in commit message
- [x] `cargo check --workspace` clean (native Windows target)
- [x] Cross-target clippy: PARTIAL — deferred to CI per checklist
- [x] No unexpected file deletions in commit
- [x] No untracked files after commit
